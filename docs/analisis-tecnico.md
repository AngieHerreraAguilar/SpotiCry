# Análisis técnico de la solución

## Arquitectura

SpotiCry tiene dos procesos independientes:

```
 ┌──────────────────────┐     WebSocket JSON      ┌────────────────────────┐
 │  Cliente (React)     │ ◄── ws://.../ws ─────►  │  Servidor (Rust)       │
 │  - Vite + Zustand    │                         │  - Tokio + Axum        │
 │  - <audio> + seek    │     HTTP Range          │  - CLI de texto        │
 │  - 3 stores          │ ◄── /stream/:id ──────  │  - WebSocket handler   │
 │                      │                         │  - HTTP streaming      │
 └──────────────────────┘                         │  - Funcional puro      │
                                                  │    en `playlists/`     │
                                                  └─────┬──────────────────┘
                                                        ▼
                                          ┌─────────────────────────────┐
                                          │  library.json               │
                                          │  playlists.json             │
                                          │  /library/*.mp3             │
                                          └─────────────────────────────┘
```

Cada cliente que se conecta al `/ws` recibe un snapshot inicial de biblioteca y playlists, y a partir de ahí permanece suscrito a un `tokio::sync::broadcast` que reenvía cada mutación a todos los pares conectados. El audio viaja por un canal HTTP separado para no mezclar control y datos en el mismo socket.

## Stack

- **Backend**: Rust 2021 + Tokio (runtime async) + Axum 0.7 (web framework con soporte nativo de WebSocket) + `tower-http::ServeFile` (HTTP Range automático) + `serde`/`serde_json` (serialización) + `im` (estructuras inmutables persistentes) + `id3` (lectura de tags MP3) + `rspotify` + `reqwest` (cliente Spotify).
- **Frontend**: React 19 + Vite 8 + Zustand 5 (estado global) + React Router 7. Sin librerías de UI: el design system se construye con CSS variables, módulos CSS y un único stylesheet con breakpoint a 768 px.
- **Persistencia**: dos archivos JSON en el directorio del servidor (`library.json`, `playlists.json`) con una task de Tokio que los rescribe con debounce de 500 ms tras cualquier mutación.
- **Protocolo**: WebSocket JSON para comandos y eventos (ver [`protocolo.md`](protocolo.md)) + HTTP Range para audio.

## Decisiones clave y justificación

### WebSocket en vez de TCP "crudo"

El enunciado original sugería sockets TCP, pero los navegadores **no pueden** abrir sockets TCP arbitrarios desde JavaScript: la única primitiva disponible es WebSocket, que es TCP con un handshake HTTP de upgrade. Se conserva el espíritu del enunciado (conexión persistente bidireccional, mensajes empujados desde el servidor) y se gana interoperabilidad con la web. La distinción "control vs datos" del enunciado se preserva: WebSocket transporta JSON de control y HTTP Range transporta los bytes de audio.

### Playlists globales compartidas (sin auth)

Maximiza el material a documentar sobre concurrencia: cada mutación de playlist por un cliente debe propagarse a todos los demás. Sin login no hay distracciones de OAuth ni tokens y el problema del "race read-modify-write" queda en el centro.

### Módulo funcional para playlists

Patrón **functional core / imperative shell**:

- El módulo [`playlists/`](../server/src/playlists) **no importa** `tokio`, `std::sync` ni nada relacionado con concurrencia. Sus funciones (`create`, `add_song`, `remove_song`, `reorder`, `filter_songs`, `sort_by`, `total_duration`) son puras: reciben `&State` y devuelven un `State` nuevo, sin `let mut`.
- El wiring vive en [`ws.rs`](../server/src/ws.rs): se clona el snapshot dentro de un `read()`, se aplica la función pura sin lock, y al final se hace `*write().await = next` con un único swap.
- Las queries usan `map`/`filter`/`fold` y closures (requisito explícito del curso): `total_duration` está implementado con `.iter().filter_map(...).map(...).fold(0, |acc, n| acc + n)`.

### Tres criterios de búsqueda técnicamente distintos

| Criterio | Algoritmo | Complejidad |
|---|---|---|
| Título | substring `to_lowercase().contains` sobre cada canción | O(n·m) — scan lineal del catálogo |
| Género | índice secundario `HashMap<Genre, Vec<SongId>>` | O(1) lookup + O(k) materialización |
| Año | filtro numérico `s.year >= from && s.year <= to` | O(n) — comparación de rango |

Tres algoritmos clásicos con costes distintos, cada uno motivado por la naturaleza del campo: texto libre, valor categórico cerrado, intervalo numérico.

### Persistencia con debounce

Volcar a JSON tras cada mutación sería caro y fragmentaría la escritura del archivo. El servidor expone un `mpsc::Sender<()>` en `AppState`; cada handler que muta envía `try_send(())` (no-bloqueante). Una task `persistence::run_debouncer` espera 500 ms tras el primer evento, drena los que hayan llegado en la ventana, y entonces sí escribe. Esto coalesce ráfagas multi-pestaña en un solo flush.

## Administración en memoria

El backend usa **dos estrategias distintas** para dos tipos de estado, intencionalmente. La elección no es accidental: cada estructura tiene un perfil de uso muy diferente.

### Library — `Arc<RwLock<HashMap<SongId, Song>>>`

La biblioteca tiene **muchas lecturas y pocas escrituras**: cualquier `search` o `library.list` es un read; los `add`/`remove` solo ocurren desde la CLI o (en menor medida) ante la finalización de una operación con Spotify. Para este patrón, `RwLock` es óptimo: múltiples lectores concurrentes sin contención, escritor exclusivo cuando hace falta.

Además mantiene un **índice secundario** `by_genre: HashMap<String, Vec<SongId>>` que se actualiza junto al hashmap principal. Esto convierte `search_by_genre` en O(1) sin tener que iterar todas las canciones — un compromiso clásico: pagar O(1) extra en escritura para acelerar lecturas a O(1).

Ver [`library.rs`](../server/src/library.rs:5-10) para la struct y [`library.rs:142-144`](../server/src/library.rs#L142-L144) para el getter por id.

### Playlists — `Arc<RwLock<im::HashMap<PlaylistId, Playlist>>>`

Las playlists tienen un perfil opuesto: **muchas escrituras concurrentes** desde varias pestañas a la vez (la prueba multi-pestaña), y cada `Playlist` contiene a su vez una `im::Vector<SongId>` (también inmutable persistente).

`im::HashMap` (de la crate `im`) implementa una estructura **inmutable persistente con structural sharing**: cuando se "muta", devuelve una versión nueva que comparte la mayoría de sus nodos con la anterior, pagando solo O(log n) por la copia. Esto habilita el patrón de [`ws.rs`](../server/src/ws.rs):

```rust
let snapshot = state.playlists.read().await.clone();   // O(1) — solo Arc bumps
let next = ops::add_song(&snapshot, pid, sid);         // pura, sin lock
*state.playlists.write().await = next;                 // swap único
```

La ventaja es que **el cómputo ocurre fuera del lock**: durante `ops::add_song`, otros lectores pueden seguir leyendo la versión anterior sin bloquear. Solo el swap final pasa por el lock, y es trivial.

`State` también mantiene `next_id: PlaylistId` para asignación monotónica — ver [`playlists/state.rs`](../server/src/playlists/state.rs).

### Playback — `Arc<Mutex<HashSet<SongId>>>`

El estado de reproducción no necesita lecturas concurrentes (es consultado puntualmente por `library::remove_song`) y las escrituras son cortas (`insert`/`remove`). Un `Mutex` simple basta: ver [`playback.rs`](../server/src/playback.rs).

## Manejo de archivos de música

### Carga de tags

Cuando la CLI hace `add <ruta>`, [`library::add_song_from_file`](../server/src/library.rs#L64-L85) usa la crate `id3` para leer los frames ID3v2 del MP3 (`title`, `artist`, `album`, `genre`, `year`). Los frames ausentes caen a `"Unknown"` para no fallar la carga. Si el usuario pasa además un `track-id` de Spotify (`add <ruta> <track-id>`), [`spotify::fetch_track`](../server/src/spotify.rs) sobreescribe la metadata con la oficial y agrega `cover_url` y `spotify_preview_url` (este último funciona como fallback de audio si el archivo local desaparece).

### Streaming HTTP con Range

El endpoint [`GET /stream/:id`](../server/src/http_stream.rs) sirve los bytes del MP3:

1. Lee la canción del `Library`. Si tiene `file_path`, marca `playback::mark_playing(id)` y delega en `tower_http::services::ServeFile`. `ServeFile` honra automáticamente el header `Range: bytes=X-Y` que envía el navegador y responde `206 Partial Content` con el slice pedido.
2. Si solo tiene `spotify_preview_url`, el servidor hace de proxy: `reqwest::get(url)` y reenvía el `bytes_stream` con `Body::from_stream`.

El navegador HTML5 abre la conexión inicial pidiendo `bytes=0-`, recibe los primeros bytes, y a medida que el `<audio>` los consume va emitiendo nuevas Range requests. **Cuando el usuario hace seek (`audio.currentTime = t`), el navegador descarta su buffer si el offset cae fuera, dispara una nueva Range request al servidor y la reproducción continúa desde la nueva posición**. Toda esta mecánica del cliente es estándar — no hay código nuestro implementando seek ni buffer; lo único que escribimos en el frontend es `seekTo: (s) => audio.currentTime = s`.

### Marcado de "playing" / "stopped"

`mark_playing` se dispara en dos lugares: el handler `Play` del WebSocket (intención del usuario) y el handler `/stream/:id` (cuando los bytes empiezan a fluir). `mark_stopped`, en cambio, **solo** se dispara desde:

- El handler `Stop` del WebSocket (el cliente avisa al pausar o cambiar de canción), o
- El cleanup al cerrar la conexión WebSocket (si el usuario cierra la pestaña sin avisar, [`ws.rs`](../server/src/ws.rs) mantiene un `HashSet<SongId>` por conexión y libera todo lo que ese cliente tenía playing).

Esto evita un bug que tuvimos en una versión previa: marcar `stopped` justo después de devolver la respuesta HTTP rompía la regla "no borrar canción reproduciéndose" porque el cliente seguía consumiendo bytes mientras el servidor ya creía que el playback había terminado. Ver [`http_stream.rs`](../server/src/http_stream.rs) y la sección "Conflictos" más abajo.

## Manejo de conflictos y excepciones

### "No se puede eliminar una canción que está sonando"

Es la regla principal del enunciado. Se valida exclusivamente en backend: [`library::remove_song`](../server/src/library.rs#L88-L108) consulta `playback::is_playing(id)` y retorna el error `CANNOT_DELETE_PLAYING` si está sonando — tanto si el delete viene de la CLI como (teóricamente) del WS. Hay tres tests unitarios que protegen el invariante.

### Cliente cierra pestaña durante reproducción

Riesgo: si el cliente no envía `stop` antes de cerrar, `playback::is_playing` quedaría devolviendo `true` para siempre, bloqueando el delete. La mitigación: cada conexión WebSocket mantiene un `HashSet<SongId>` con las canciones que ese cliente puso en "playing"; cuando el `tokio::select!` cae al rama `else => break`, el `for id in owned_playing.drain()` libera todo y emite `PlaybackStopped` por broadcast. Así, cerrar la pestaña tiene el mismo efecto que enviar `stop` explícitamente.

### MP3 corrupto o ruta inválida

`Tag::read_from_path` propaga `id3::Error`; la CLI lo captura en `handle_add` e imprime `✗ error agregando '...': <e>` sin abortar el servidor. Si el archivo desaparece después de haberse registrado, `ServeFile` responde `404` cuando llega la siguiente Range request — el cliente ve un `<audio>` que falla y muestra el error.

### Spotify rate-limited / 401

[`spotify::fetch_track`](../server/src/spotify.rs) maneja dos rutas: refresh de token (caché TTL 1 h) y errores HTTP. Si falta `SPOTIFY_CLIENT_ID/SECRET`, el server arranca igual con `spotify: None` y la CLI rechaza `add-spotify` con un mensaje claro. Un `429`/`401` de la API se imprime como warning y se permite reintentar.

### Mensajes WebSocket malformados

`serde_json::from_str::<ClientMsg>` falla con un `Err` capturado en [`ws.rs`](../server/src/ws.rs#L65-L73), que devuelve un `ServerEvent::Error { code: BadRequest, msg: "invalid message" }` al cliente sin cerrar la conexión.

### `Play` con `song_id` inexistente

Antes era posible marcar como "playing" un id que no estaba en la library. Ahora el handler valida primero con `library.get(song_id)` y emite `ServerEvent::Error { code: NotFound }` si no existe, sin tocar el estado de playback.
