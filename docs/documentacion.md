# SpotiCry — Documentación del Proyecto

**Instituto Tecnológico de Costa Rica · Campus Tecnológico Local San Carlos**

**Escuela de Ingeniería en Computación · Lenguajes de Programación**

**Profesor:** Oscar Víquez

**Integrantes:** Angie Herrera Aguilar (2020035640) · Kevin Rivera Gonzalez (2024157337)

**Semestre I · Año 2026**

---

## 1. Introducción

SpotiCry es una aplicación web de reproducción y gestión de música compuesta por un backend en **Rust** y un frontend desarrollado con **React**. El sistema permite administrar una biblioteca de canciones, reproducir audio mediante streaming HTTP y mantener comunicación en tiempo real con los clientes a través de WebSockets.

El backend está diseñado bajo principios de concurrencia segura utilizando `Arc` y `RwLock`, junto con un sistema de eventos basado en `broadcast`, lo que permite manejar múltiples conexiones simultáneamente de forma eficiente. Por su parte, el frontend ofrece una interfaz interactiva que consume estos servicios para visualizar la biblioteca, ejecutar búsquedas, controlar la reproducción y gestionar playlists en tiempo real.

SpotiCry integra múltiples fuentes de audio, incluyendo archivos locales y previews obtenidos desde Spotify, logrando así una experiencia híbrida entre reproducción local y servicios en línea. La arquitectura modular del sistema separa claramente responsabilidades en componentes como biblioteca, playlists, reproducción, streaming y protocolo de comunicación, facilitando su escalabilidad y mantenimiento.

El sistema persiste su estado en archivos JSON locales con escritura *debounced* (`library.json`, `playlists.json`, y un manifest `library/tracks.json` con las preferencias del scan automático). Sigue además el patrón **functional-core / imperative-shell**, donde el módulo de playlists está implementado como funciones puras sobre estructuras inmutables persistentes (`im::HashMap`, `im::Vector`) y solo el wiring concurrente vive en el shell imperativo.

---

## 2. Descripción del problema y objetivos

### 2.1 Contexto

SpotiCry es un proyecto académico desarrollado para la asignatura de Lenguajes de Programación. El curso enfatiza el contraste entre dos paradigmas dentro de un mismo sistema: **imperativo** (estado mutable controlado por locks) y **funcional puro** (estructuras inmutables, funciones sin efectos, transformaciones con `map`/`filter`/`fold` y closures).

A nivel arquitectónico el enunciado pide además: (1) concurrencia real con sockets, (2) transmisión de bytes de audio entre procesos, (3) persistencia en disco y (4) documentación que compare la operación del sistema "con vs sin sincronización".

### 2.2 Problema a resolver

Construir una aplicación tipo Spotify con dos componentes independientes que se comunican por red:

- Un **servidor en Rust** que mantiene la biblioteca de canciones, las playlists compartidas, el estado de reproducción y sirve los archivos de audio por streaming.
- Un **cliente web** (React) que consume ese servidor para buscar, reproducir con seek libre, crear playlists y ver el catálogo en tiempo real, incluyendo cambios hechos por otros usuarios.

El sistema debe demostrar de forma explícita el patrón **functional core, imperative shell**: el módulo de playlists es funcional puro (sin `let mut`, sin locks, con `im::HashMap`/`im::Vector`), mientras que el resto del backend (library, playback, persistencia) es imperativo y se sincroniza con `Arc<RwLock<…>>` y `tokio::sync::broadcast`.

### 2.3 Objetivos

**General:** diseñar e implementar una aplicación cliente–servidor que reproduzca audio de forma concurrente, mostrando explícitamente el contraste entre programación imperativa y funcional dentro del mismo proyecto.

**Específicos del backend:**
- Modelar `Song`, `Playlist` y el estado global compartido (`AppState`).
- Implementar tres criterios de búsqueda técnicamente distintos (título, género, año).
- Cumplir la regla "no se puede eliminar una canción que está sonando" en backend.
- Implementar el módulo `playlists/` en estilo funcional puro.
- Servir audio por HTTP Range (`206 Partial Content`).
- Persistir biblioteca y playlists en JSON.
- Integrar la API de Spotify (Client Credentials Flow) para enriquecer metadata.

**Específicos del frontend:**
- Cliente WebSocket con reconexión y reenvío de comandos.
- Reproductor `<audio>` con seek libre apoyado en HTTP Range.
- Vistas para biblioteca, búsqueda y playlists en tiempo real.
- Responsive con breakpoint a 768 px.

**Concurrencia y robustez:**
- Sincronización fan-out vía `broadcast::channel` a todos los clientes.
- Demostrar con prueba multi-pestaña que ninguna mutación se pierde.

### 2.4 Restricciones técnicas explícitas

- El módulo `playlists/` no puede usar `let mut`, `tokio` ni primitivas de sincronización: solo estructuras inmutables y combinadores funcionales (`map`, `filter`, `fold`).
- La regla "no borrar canción reproduciéndose" se valida en el backend; el frontend nunca es la única línea de defensa.
- El protocolo WebSocket quedó congelado al final del Día 1 y cualquier cambio posterior requería acuerdo del equipo.

---

## 3. Funcionalidades implementadas

Para cada bloque se describe **qué hace** seguido de **por qué se hizo así**.

### 3.1 Biblioteca y búsqueda

**Qué hace.** Mantiene una colección de canciones (`Song`) cargadas desde MP3s locales y/o desde Spotify (metadata + portada + preview de 30 s cuando esté disponible). Permite agregar, eliminar, listar y buscar por tres criterios.

> **Nota sobre el preview de Spotify.** Desde 2024 Spotify dejó de exponer `preview_url` en la mayoría de las respuestas de la Web API para aplicaciones con Client Credentials Flow. En la práctica esto significa que para la mayor parte de los tracks el campo viene en `null`. Por eso el flujo recomendado es `add <ruta-mp3> <spotify-id>` (MP3 local + metadata + portada) en lugar de `add-spotify <id>` solo, que solo funciona en los pocos tracks que aún devuelven preview.

- **Auto-scan** al iniciar el servidor: detecta MP3s nuevos en la carpeta `library/` y los configura via CLI con un menú simple (`[s]` enlace de Spotify, `[m]` metadata manual, `[t]` tags ID3, `[x]` saltar). Las preferencias se guardan en `library/tracks.json` para que arranques siguientes sean automáticos sin intervención del usuario.
- **Tres algoritmos de búsqueda técnicamente distintos:**
  - Título → substring case-insensitive (scan lineal, complejidad O(n·m)).
  - Género → índice secundario `HashMap<Genre, Vec<SongId>>` (lookup O(1)).
  - Año → filtro numérico de rango inclusivo (O(n)).

**Por qué se hizo así.**
- El auto-scan + manifest reemplaza un comando de CLI complejo (`add <ruta-larga> <id-base62>`) por un flow donde el usuario solo arrastra MP3s a una carpeta y responde un menú. Reduce errores de tipeo y hace la app usable sin saber comandos de memoria.
- Los tres algoritmos de búsqueda son requisito explícito del curso ("tres criterios técnicamente distintos"). Cada uno está motivado por la naturaleza del campo: texto libre (substring), valor categórico cerrado (índice), intervalo numérico (filtro). El índice secundario por género paga O(1) extra en escritura para acelerar lecturas a O(1), un trade-off clásico que vale la pena cuando hay muchas más búsquedas que altas.

### 3.2 Reproducción

**Qué hace.**
- Streaming HTTP con Range requests; el `<audio>` del navegador hace seek nativo emitiendo nuevas peticiones automáticas.
- Controles del reproductor: play, pausa, reanudación, seek arrastrable, saltos ±10 s.
- Sincroniza pausa/play/end con el server vía WebSocket: cada cambio del lado del cliente avisa al server para actualizar el estado de reproducción.
- Cleanup automático al cerrar la pestaña: si un cliente se desconecta sin avisar, el server libera las canciones que tenía marcadas como "playing".
- Validación del invariante "no se puede eliminar una canción en reproducción" (`CANNOT_DELETE_PLAYING`), protegida por tres tests unitarios.

**Por qué se hizo así.**
- HTTP Range es lo más natural para `<audio>`: el navegador ya implementa buffer + seek + recargas — no se escribe JS para ninguna de esas mecánicas, solo `audio.currentTime = t`. Si se hubiera usado WebSocket para audio, habría que reimplementar todo eso a mano.
- La separación "control va por WS, audio va por HTTP" mantiene los dos canales con responsabilidades distintas: el WS lleva mensajes pequeños y frecuentes, el HTTP transporta bytes pesados y reusables. Mezclarlos saturaría el WS y haría el seek imposible.
- El cleanup por conexión es defensa contra el caso real "el usuario cierra la pestaña sin pausar". Sin esto, el set de "en reproducción" se llenaría de falsos positivos eternos y la regla del enunciado se rompería.

### 3.3 Playlists — módulo funcional puro

**Qué hace.**
- Crear, eliminar, agregar/quitar canciones, reordenar, filtrar por criterio (título/género/año) dentro de la playlist, ordenar por título/año/duración.
- Validación de nombres duplicados (case-insensitive con trim) y nombres vacíos: en backend (`Option<>`) y con feedback inmediato en el modal del frontend.
- Estado inmutable persistente con `im::HashMap<PlaylistId, Playlist>` e `im::Vector<SongId>`.
- Patrón **functional core / imperative shell**: el módulo `playlists/` no importa `tokio` ni `std::sync` ni usa `let mut`. Sus funciones son puras: reciben `&State` y devuelven un `State` nuevo.

**Por qué se hizo así.**
- La regla del curso obliga a que un módulo del proyecto sea funcional puro. Las playlists son el candidato natural porque tienen muchas operaciones combinables (filter/sort/map/fold) y se prestan a estructuras inmutables.
- `im::HashMap` da **structural sharing O(log n)**: clonar un snapshot es barato y no copia toda la estructura. Eso habilita el patrón clave: leer un snapshot, computar fuera del lock, hacer un swap atómico al final. El cómputo no bloquea a otros lectores.
- La validación de duplicados se hace en backend (línea de defensa real, multi-cliente) y en frontend (feedback inmediato al usuario, evita el round-trip al server). Si solo estuviera en el frontend, dos pestañas podrían crear el mismo nombre simultáneamente.

### 3.4 WebSocket y broadcast

**Qué hace.**
- Conexión cliente-servidor en `ws://<host>:8080/ws` con reconexión automática (backoff exponencial).
- Snapshot inicial automático al conectar: `library.snapshot` + `playlist.snapshot` sin que el cliente los pida.
- Broadcast en tiempo real de mutaciones de biblioteca/playlists, eventos de reproducción y resultados de búsqueda — usando `tokio::sync::broadcast::channel`.
- Cleanup por conexión: cada cliente mantiene un `HashSet<SongId>` de canciones que tenía sonando y se libera al desconectar.

**Por qué se hizo así.**
- WebSocket es la única primitiva persistente bidireccional disponible para un navegador (no se pueden abrir TCP arbitrarios desde JS). Conserva el espíritu del enunciado original ("conexión persistente con sockets") y suma compatibilidad con la web.
- El snapshot inicial automático evita una ronda extra (cliente conecta → cliente pide → server responde). Garantiza que el UI tenga datos al renderizar la primera pantalla, sin estados intermedios "cargando" innecesarios.
- `broadcast::channel` resuelve el fan-out sin que el publicador conozca a los suscriptores. Cualquier handler que mute estado solo hace `tx.send(event)` y todos los WS conectados reciben el evento. Es exactamente el patrón pub/sub que el enunciado pide para "concurrencia compartida".

### 3.5 Streaming HTTP

**Qué hace.**
- `GET /stream/:id` con soporte de Range requests (streaming parcial).
- Reproducción desde archivo local (`tower_http::services::ServeFile`) o proxy de preview de Spotify (`reqwest`).
- `Cache-Control: no-cache` en cada respuesta para evitar que el navegador sirva audio cacheado obsoleto tras un reset.
- Cache-buster `?t=<timestamp>` en el frontend: cada `play()` produce una URL única.

**Por qué se hizo así.**
- `tower_http::ServeFile` ya implementa `Range`/`206 Partial Content` correctamente — reusarlo en lugar de reimplementarlo evita bugs de off-by-one y respuestas mal formadas.
- El proxy de preview Spotify mantiene la abstracción "una sola URL por canción" (`/stream/:id`) — el cliente no necesita saber si el audio viene de disco local o de Spotify. Esto desacopla el frontend del origen del archivo.
- El doble fix de caché (`no-cache` + cache-buster) fue necesario porque Chrome tiene un **caché de medios separado** del HTTP normal, que no se purga con `Cmd+Shift+R`. Tras varios resets de `library.json`, el caché de medios podía servir audio antiguo asociado a un id que ahora apuntaba a otro archivo. Con las dos defensas combinadas el problema desaparece.

### 3.6 Persistencia

**Qué hace.**
- `library.json` y `playlists.json` se leen al arrancar y se vuelcan automáticamente con **debounce de 500 ms** tras cualquier mutación (CLI o WS).
- `library/tracks.json` guarda las preferencias del scan automático.
- Filtrado al cargar de entradas con `file_path` que ya no existen — auto-limpieza tras renombrar/mover MP3s.

**Por qué se hizo así.**
- Volcar a JSON tras cada mutación sería caro y fragmentaría la escritura. El servidor expone un `mpsc::Sender<()>` en `AppState`; cada handler que muta envía `try_send(())` no bloqueante. La task `persistence::run_debouncer` espera 500 ms tras el primer evento, drena los que hayan llegado en la ventana, y entonces sí escribe. Esto **coalesce ráfagas multi-pestaña en un solo flush**.
- Separar el manifest (`tracks.json`) del estado real (`library.json`) permite que las dos cosas evolucionen independientemente: el manifest decide cómo cargar futuros MP3s; el estado guarda lo que ya está cargado.
- El filtrado de `file_path` inexistentes evita arrastrar canciones huérfanas indefinidamente: si el usuario mueve un MP3 fuera de la carpeta, la siguiente sesión limpia automáticamente.

### 3.7 CLI del servidor

**Qué hace.**
- Comandos `add <ruta>`, `add <ruta> <spotify-id>`, `add-spotify <id>`, `remove <id>`, `list`, `playlists`, `help`, `quit`/`exit`.
- Setup interactivo al arrancar para configurar MP3s nuevos detectados por el scan.
- Persiste estado al `quit`/EOF (Ctrl-D) además del debouncer automático.

**Por qué se hizo así.**
- El enunciado exige una "interfaz de texto" del servidor — es la administración mínima del catálogo. Mantener la CLI como camino de respaldo (incluso con auto-scan) cubre los casos donde el setup interactivo no aplica.
- Combinar el `add` con `add <ruta> <id>` (mp3 local + metadata de Spotify) da lo mejor de los dos mundos: audio completo del disco y metadata + portada bonita de Spotify.

### 3.8 Frontend

**Qué hace.**
- Tres páginas: Inicio (biblioteca), Buscar (3 tabs), Playlist (vista detalle).
- Tres stores Zustand: `library`, `playlists`, `player`.
- Botón `+` estilo Spotify para agregar canciones a playlists, con check verde si ya están.
- Mosaico 2×2 de portadas reales como hero de cada playlist.
- Reproductor fijo al fondo del viewport (no se necesita scroll para llegar a él).
- Responsive con breakpoint a 768 px (sidebar → bottom nav).

**Por qué se hizo así.**
- Zustand sobre Redux/Context API: mucho menos boilerplate, un solo `create()` por store, y suscripción automática con selectores. Para tres stores pequeños es la opción más liviana.
- Sin librería de UI: el design system se construye con CSS variables y un único stylesheet. Esto da control total y un bundle más chico.
- El layout fijo al viewport (`100dvh` + `min-height: 0` en el grid item de scroll) usa una técnica conocida de CSS Grid que asegura que el reproductor esté siempre visible y la lista scrollee internamente. `100dvh` (dynamic viewport height) respeta la barra de URL dinámica de iOS Safari.

---

## 4. Funcionalidades NO incluidas

### 4.1 Spotify
- Sin autenticación de usuario (solo Client Credentials Flow para metadata pública).
- No se importan playlists desde Spotify; las playlists viven solo en SpotiCry.
- La búsqueda solo opera sobre la biblioteca local, no sobre el catálogo global de Spotify.
- **El preview de 30 s no está garantizado.** Spotify cambió su política en 2024 y dejó de exponer `preview_url` en la mayoría de tracks vía Client Credentials Flow. Las canciones sin MP3 local quedan registradas con metadata + portada pero pueden no ser reproducibles. El modo recomendado es siempre combinar el MP3 local con el track ID de Spotify (`add <ruta-mp3> <spotify-id>`).

### 4.2 Persistencia avanzada
- No hay base de datos relacional; el estado vive en archivos JSON locales.
- No hay versionado, snapshots históricos ni undo.
- No hay servicio de backups automáticos.

### 4.3 Reproducción
- No hay slider de volumen integrado (se delega al sistema operativo). Sí hay pausa, reanudación, seek libre y saltos ±10 s.
- No hay catálogo continuo tipo Spotify: el audio principal son MP3 locales, con preview de 30 s como fallback.
- No hay reproducción automática "siguiente canción" ni cola.
- No hay segmentación HLS/DASH ni adaptación de bitrate.

### 4.4 Multiusuario
- No hay manejo de usuarios, autenticación ni sesiones.
- Estado global compartido — todas las playlists son visibles para cualquier cliente conectado. Decisión deliberada para maximizar el material a documentar sobre concurrencia.

### 4.5 Despliegue
- No hay deploy automatizado en producción. La entrega corre en localhost; un screencast acompaña la demo si es necesario.

---

## 5. Análisis técnico

### 5.1 Arquitectura

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
                                          │  library/tracks.json        │
                                          │  library/*.mp3              │
                                          └─────────────────────────────┘
```

Cada cliente que se conecta al `/ws` recibe un snapshot inicial de biblioteca y playlists, y queda suscrito a un `tokio::sync::broadcast` que reenvía cada mutación a todos los pares conectados. El audio viaja por un canal HTTP separado para no mezclar control y datos en el mismo socket.

### 5.2 Stack y por qué cada pieza

| Pieza | Por qué |
|---|---|
| Rust 2021 | Compilador estricto evita usos de variables movidas, datos compartidos sin sincronización, errores no manejados. Una vez que compila, suele funcionar. |
| Tokio (runtime async) | Permite expresar I/O concurrente con código que se lee secuencial; soporta miles de conexiones WS sin un thread por cliente. |
| Axum 0.7 | Web framework con soporte nativo de WebSocket y handlers fáciles de testear; integra `tower-http`. |
| `tower-http::ServeFile` | Implementa Range/`206 Partial Content` correctamente — reusarlo evita bugs de off-by-one. |
| `im` (HashMap, Vector) | Estructuras inmutables persistentes con structural sharing O(log n); habilita el patrón "compute fuera del lock + swap atómico". |
| `id3` | Lectura de tags ID3v2 de MP3. |
| `rspotify` | Cliente oficial de la API de Spotify; soporta Client Credentials Flow sin OAuth de usuario. |
| `reqwest` | Cliente HTTP para hacer proxy de previews Spotify. |
| `serde` / `serde_json` | Serialización tipada para el protocolo y la persistencia. |
| `dotenvy` | Carga `server/.env` automáticamente para no tener que prefijar variables. |
| React 19 + Vite 8 | Build rápido, HMR instantáneo, ecosistema maduro. |
| Zustand 5 | Estado global con mucho menos boilerplate que Redux/Context para 3 stores pequeños. |
| React Router 7 | Routing declarativo para 3 páginas. |

### 5.3 Decisiones clave y justificación

**WebSocket en vez de TCP "crudo".** El enunciado original sugería sockets TCP, pero los navegadores no pueden abrir sockets TCP arbitrarios desde JS: la única primitiva disponible es WebSocket, que es TCP con un handshake HTTP de upgrade. Se conserva el espíritu del enunciado (conexión persistente bidireccional, mensajes empujados desde el server) y se gana interoperabilidad con la web.

**Playlists globales compartidas (sin auth).** Maximiza el material a documentar sobre concurrencia: cada mutación de playlist por un cliente debe propagarse a todos los demás. Sin login no hay distracciones de OAuth y el problema del "race read-modify-write" queda en el centro.

**Módulo funcional para playlists.** El módulo `playlists/` no importa `tokio`, `std::sync` ni nada de concurrencia. Sus funciones (`create`, `add_song`, `remove_song`, `reorder`, `filter_songs`, `sort_by`, `total_duration`) son puras: reciben `&State`, devuelven `State` nuevo, sin `let mut`. El wiring vive en `ws.rs` con el patrón "leer snapshot → aplicar fn pura → swap":

```rust
let snapshot = state.playlists.read().await.clone();   // O(1) gracias a im
let next = ops::add_song(&snapshot, pid, sid);         // pura, sin lock
*state.playlists.write().await = next;                 // swap único
```

**Tres criterios de búsqueda con algoritmos distintos.**

| Criterio | Algoritmo | Complejidad |
|---|---|---|
| Título | substring `to_lowercase().contains` sobre cada canción | O(n·m) — scan lineal |
| Género | índice secundario `HashMap<Genre, Vec<SongId>>` | O(1) lookup + O(k) materialización |
| Año | filtro `s.year >= from && s.year <= to` | O(n) — comparación de rango |

**Persistencia con debounce.** Un `mpsc::Sender<()>` en `AppState` recibe un `try_send(())` no bloqueante tras cada mutación. Una task `persistence::run_debouncer` espera 500 ms, drena el canal, y vuelca `library.json` y `playlists.json`. Coalesce ráfagas multi-pestaña en un solo flush.

### 5.4 Administración en memoria

El backend usa **dos estrategias intencionalmente distintas** para dos tipos de estado:

**Library — `Arc<RwLock<HashMap<SongId, Song>>>`.**
Muchas lecturas (cada `search`, cada `library.list`) y pocas escrituras (CLI). `RwLock` deja a múltiples lectores entrar a la vez sin contención y solo serializa al escritor. Mantiene un índice secundario `by_genre: HashMap<String, Vec<SongId>>` que permite `search_by_genre` en O(1) — un trade-off clásico: pagar O(1) extra en escritura para acelerar lecturas.

**Playlists — `Arc<RwLock<im::HashMap<PlaylistId, Playlist>>>`.**
Muchas escrituras concurrentes (multi-pestaña). `im::HashMap` tiene structural sharing: clonar un snapshot es O(1). Esto habilita "compute fuera del lock + swap" y elimina la contención del cómputo.

**Playback — `Arc<Mutex<HashSet<SongId>>>`.**
No requiere lecturas concurrentes (consultas puntuales) y las escrituras son cortas. Un `Mutex<HashSet>` simple basta.

### 5.5 Manejo de archivos de música

**Carga.** `Library::add_song_from_file(Path)` usa `id3::Tag::read_from_path` para leer los frames ID3v2. Frames ausentes caen a `"Unknown"`. Si el usuario pasa además un track-id de Spotify, `spotify::fetch_track` sobreescribe la metadata con la oficial y agrega `cover_url` y `spotify_preview_url`.

**Streaming.** El endpoint `GET /stream/:id`:
1. Si la canción tiene `file_path` → marca `playback::mark_playing(id)` y delega en `ServeFile`. `ServeFile` honra el header `Range: bytes=X-Y` y responde `206 Partial Content`.
2. Si solo tiene `spotify_preview_url` → proxy con `reqwest::get(url)` reenviando el `bytes_stream`.
3. En ambos casos se inyecta `Cache-Control: no-cache` para evitar el caché de medios de Chrome.

**Seek.** Cuando el usuario hace `audio.currentTime = t`, el navegador descarta el buffer si el offset cae fuera, dispara una nueva Range request, y la reproducción continúa desde la nueva posición. No se escribe código JS para esto; el `<audio>` lo hace nativo.

### 5.6 Manejo de conflictos y excepciones

| Caso | Cómo se maneja |
|---|---|
| Eliminar canción reproduciéndose | `library::remove_song` consulta `playback::is_playing(id)` y devuelve `CANNOT_DELETE_PLAYING`. Tres tests unitarios protegen el invariante. |
| Cliente cierra pestaña sin avisar | Cada conexión WS mantiene `HashSet<SongId>` con lo que tiene "playing"; al hacer `break` del `tokio::select!`, libera todo y emite `PlaybackStopped` por broadcast. |
| MP3 corrupto o ruta inválida | `Tag::read_from_path` propaga `id3::Error`; la CLI lo captura e imprime mensaje sin abortar el server. Si el archivo desaparece después, `ServeFile` responde 404. |
| Spotify rate-limited / 401 | Refresh de token cacheado (TTL 1 h). Si falta `SPOTIFY_CLIENT_ID/SECRET`, el server arranca con `spotify: None` y la CLI rechaza `add-spotify` con mensaje claro. |
| WebSocket malformado | `serde_json::from_str` falla → handler envía `ServerEvent::Error { code: BadRequest }` sin cerrar la conexión. |
| `Play` con `song_id` inexistente | Validación previa con `library.get(song_id)`; emite `Error { code: NotFound }` sin tocar playback. |
| Caché viejo del navegador | `Cache-Control: no-cache` + cache-buster `?t=<timestamp>` en cada `play()`. |
| Nombres duplicados de playlist | `ops::create` devuelve `Option`; el handler emite `Error { code: BadRequest }` con mensaje. Frontend además valida en vivo y deshabilita el botón "Guardar". |

---

## 6. Protocolo de mensajes

### 6.1 Transport
- Comandos (bidireccional): WebSocket JSON en `ws://<host>:8080/ws`.
- Audio (servidor → cliente): HTTP `GET` con `Range: bytes=X-Y` en `http://<host>:8080/stream/:id`.

### 6.2 Cliente → Servidor

| `op` | Parámetros | Descripción |
|---|---|---|
| `search` | `by`, `value` | Busca por título / género / year_range |
| `library.list` | — | Solicita la biblioteca completa |
| `playlist.list` | — | Solicita todas las playlists |
| `playlist.create` | `name` | Crea playlist global |
| `playlist.delete` | `playlist_id` | Borra playlist |
| `playlist.add` | `playlist_id`, `song_id` | Agrega canción a playlist |
| `playlist.remove` | `playlist_id`, `song_id` | Quita canción de playlist |
| `playlist.filter` | `playlist_id`, `by`, `value` | Filtra dentro de playlist |
| `playlist.sort` | `playlist_id`, `by` | Ordena por title/year/duration |
| `play` | `song_id` | Marca como "en reproducción" |
| `stop` | `song_id` | Marca como parada |

### 6.3 Servidor → Cliente

| `ev` | Payload | Descripción |
|---|---|---|
| `library.snapshot` | `songs: Song[]` | Estado completo de la biblioteca |
| `playlist.snapshot` | `playlists: Playlist[]` | Estado completo de playlists |
| `search.result` | `songs: Song[]` | Resultado de `search` |
| `now_playing` | `song_id`, `stream_url` | Alguien inició reproducción |
| `playback.stopped` | `song_id` | Alguien detuvo reproducción |
| `error` | `code`, `msg` | Error del servidor |

**Códigos de error:** `CANNOT_DELETE_PLAYING`, `NOT_FOUND`, `BAD_REQUEST`, `INTERNAL`.

### 6.4 Convenciones críticas

1. **Snapshot inicial automático.** Al abrir el WS, el servidor envía `library.snapshot` + `playlist.snapshot` sin que el cliente los pida.
2. **Broadcast en mutaciones de playlist.** Cualquier `playlist.create/delete/add/remove/sort` emite `playlist.snapshot` a TODOS los clientes conectados. Esto valida el requisito de "concurrencia compartida".
3. **Semántica de `stop`.** El cliente envía `stop` cuando cambia de canción, pausa o cierra la app. Sin esta convención el set "en reproducción" se llena de falsos positivos.

### 6.5 Ejemplos

```json
→ { "op": "search", "by": "year_range", "value": [1990, 2000] }
← { "ev": "search.result", "songs": [...] }

→ { "op": "playlist.create", "name": "Road Trip" }
← { "ev": "playlist.snapshot", "playlists": [...] }

→ { "op": "play", "song_id": 42 }
← { "ev": "now_playing", "song_id": 42, "stream_url": "/stream/42" }
```

---

## 7. Concurrencia: con vs sin sincronización

### 7.1 El problema sin sincronización

Si las playlists vivieran como un `HashMap` plano sin lock y cada cliente aplicara `playlist.add` directamente:

```
T0  Estado: playlist.songs = [x, y]
T1  Cliente A: leer  → [x, y]
T2  Cliente B: leer  → [x, y]
T3  Cliente A: write → [x, y, z]   ✓
T4  Cliente B: write → [x, y, w]   ✗  (z se pierde)
```

Es un *read-modify-write race condition*: dos hilos leen la misma versión, calculan en paralelo, y el último write borra al anterior. La cantidad de "z perdidos" crece con el número de pestañas.

### 7.2 La solución aplicada — patrón functional core / imperative shell

```rust
let snapshot = state.playlists.read().await.clone();   // O(1) — solo Arc bumps
let next = ops::add_song(&snapshot, pid, sid);         // pura, sin lock
*state.playlists.write().await = next;                 // swap único
```

Tres garantías:
1. **Ningún add se pierde.** Las write locks se serializan; aplicaciones secuenciales sobre el `next` consolidado.
2. **Sin lock retenido durante el cómputo.** `ops::add_song` corre mientras otros leen.
3. **Atomicidad observable.** Nunca hay un estado público "intermedio".

### 7.3 Procedimiento experimental

1. Levantar el servidor: `cd server && cargo run`.
2. Cargar 10–20 canciones por CLI (mezcla `add <ruta>` + `add-spotify <id>`).
3. Levantar el frontend: `cd client && npm run dev`.
4. Abrir 3 pestañas en `http://localhost:5173`.
5. Crear desde una pestaña una playlist vacía "concurrency-test".
6. Esperar 1 s a que el snapshot llegue a las 3 pestañas.
7. En cada pestaña, hacer `addSong(playlist_id, song_X_distinta)` en ráfaga (~10 adds por pestaña en menos de 2 s).
8. Cerrar la ráfaga y esperar 1 s.
9. Comparar:
   - Total esperado: 30 canciones en la playlist.
   - Total observado en cada pestaña tras el último broadcast.
   - Total observado en `playlists.json` tras el debounce.

Pruebas adicionales:
- **Remove durante reproducción.** Reproducir → `remove <id>` desde CLI → debe rechazar `CANNOT_DELETE_PLAYING` → pausar → repetir → debe pasar.
- **Cleanup al cerrar pestaña.** Reproducir → cerrar pestaña sin pausar → `remove <id>` desde CLI → debe pasar (cleanup automático).

### 7.4 Resultado esperado por construcción
- **Adds no se pierden**: total final = `n_pestañas × adds_por_pestaña`.
- **Convergencia total-order**: las 3 pestañas convergen al mismo conteo.
- **`playlists.json` coherente**: el flush captura exactamente el estado final, sin estados intermedios.

### 7.5 Reflexión a producción a gran escala

| Dimensión | Hoy | Producción |
|---|---|---|
| Almacenamiento | JSON local | Postgres (library con `tsvector` para fulltext) + S3 (audio) |
| Concurrencia | `RwLock` + `im` | CRDTs (ORSet, RGA) o event sourcing con orden total |
| Auth | Sin auth | OIDC, JWT firmado en cada conexión WS |
| Streaming | HTTP Range | HLS / DASH + DRM + adaptación de bitrate |
| Observabilidad | `tracing` con stdout | OpenTelemetry, dashboards Grafana, alertas |
| Despliegue | localhost | Kubernetes con autoscaler por conexiones WS, CDN para frontend |

El patrón **functional-core / imperative-shell** no se reemplaza, se generaliza: la parte funcional pura sigue siendo el motor; lo que cambia es cómo se persiste el estado, cómo se replica entre instancias y cómo se autoriza cada operación.

---

## 8. Resultados y métricas

### 8.1 Funcionalidades implementadas (MVP)

Todos los items del MVP del enunciado quedaron operativos al cierre del proyecto. Backend Rust con Tokio + Axum, tres criterios de búsqueda, CLI completa, persistencia JSON con debounce, módulo funcional puro, regla "no borrar playing" con tests, streaming HTTP Range, integración Spotify Client Credentials. Frontend con tres stores Zustand, buscador con tabs, reproductor con seek libre, CRUD de playlists, sincronización en vivo, responsive a 768 px.

### 8.2 Métricas (entorno de desarrollo, macOS Apple Silicon, build debug)

- **Latencia del WebSocket inicial**: < 50 ms (snapshot tras `101 Switching Protocols`).
- **Latencia de seek**: < 100 ms en localhost (`audio.currentTime = t` → nueva Range request → primer byte).
- **Tamaño de chunks Range**: 64 KiB por defecto del navegador.
- **Memoria del proceso del servidor**: < 30 MB en reposo con decenas de canciones cargadas.
- **Build incremental**: `cargo build` ~3 s tras el primer build limpio.
- **Tests**: `cargo test` ejecuta 5 tests en < 0.1 s (los 3 del invariante + smoke de CLI + remove inexistente).

### 8.3 Manejo de archivos de música — flujo end-to-end

1. **Auto-scan** o **CLI `add`** → `Library::add_song_from_file` lee tags ID3 con la crate `id3` y deja `file_path` apuntando al archivo local. La canción recibe un `id` autoincremental empezando en 1.
2. **`library.json`** → tras cualquier mutación, el debouncer (`mpsc::Sender<()>`) acumula 500 ms y vuelca el snapshot completo. Al siguiente arranque, `main.rs` llama a `load_library` + `load_playlists` y construye el estado inicial.
3. **Cliente conecta** → `ws_handler` envía `LibrarySnapshot` y `PlaylistSnapshot` automáticamente.
4. **Reproducción** → click en una canción → `sendCmd("play", { song_id })` → server marca `Playback::mark_playing` → emite `NowPlaying { song_id, stream_url: "/stream/{id}" }`.
5. **Streaming** → `<audio>` hace `GET /stream/:id` con `Range: bytes=0-` → handler responde `206 Partial Content` con el slice. Seeks generan nuevas Range requests automáticas.
6. **Stop** → cambio de canción / pausa / cierre de pestaña → `sendCmd("stop", { song_id })` → `Playback::mark_stopped` → libera para `remove`.

Caso especial: si la canción se agregó con `add-spotify` y no hay archivo local, `file_path` queda en `None` y el endpoint hace de proxy hacia `spotify_preview_url` (preview de 30 s). En la práctica este fallback solo funciona para los tracks donde Spotify aún expone `preview_url` — desde 2024 la mayoría devuelve `null` por cambios en la política de Client Credentials Flow.

```spoticry/
 ├── docs/
 │    ├── documentacion.md
 │    └── images/
 │         ├── devtools-network-concurrency.png
 │         ├── home.png
 │         ├── search-1.png
 │         ├── search-2.png
 │         ├── playlist-detail.png
 │         ├── player.png
 │         ├── mobile-768.png
 │         ├── cargo-test.png
 │         └── remove-while-playing.png
```

### 8.4 Capturas de pantalla

  En esta sección se presentan las capturas del sistema en funcionamiento, incluyendo la interfaz de usuario, pruebas de concurrencia y herramientas de desarrollo utilizadas para validar el comportamiento de la aplicación.

**8.4.1 DevTools – Requests concurrentes**

  Se utilizó la herramienta de desarrollo del navegador Google Chrome (DevTools), específicamente la pestaña Network, para observar múltiples peticiones HTTP ejecutándose en paralelo durante pruebas de concurrencia.

**8.4.2 Vista Home**

  Pantalla principal de la aplicación donde se accede a las funcionalidades generales del sistema

**8.4.3 Búsqueda de canciones**

  Se realizaron búsquedas utilizando distintos criterios para validar el correcto filtrado de resultados.

**8.4.4 Detalle de playlist**

  Vista del contenido de una playlist con sus canciones organizadas.

**8.4.6 Vista responsive**

  Validación del diseño adaptable en pantallas móviles o reducidas

**8.4.7 Terminal y pruebas del sistema**

  Evidencia de ejecución de pruebas automáticas (cargo test) y validación de comportamiento del sistema bajo condiciones específicas.
  Tests exitosos
  Intento de eliminación durante reproducción

---

## 9. Conclusiones y recomendaciones

### 9.1 Sobre el contraste imperativo vs funcional dentro del mismo sistema

El proyecto demuestra que ambos paradigmas pueden convivir sin que uno "contamine" al otro. La biblioteca usa `HashMap` mutable con `let mut` y `for` loops; las playlists usan `im::HashMap`/`im::Vector` y exponen funciones puras. El pegamento entre ambos mundos vive en `ws.rs`, donde el patrón **functional core, imperative shell** aparece literal en código: lock para clonar el snapshot, computación pura fuera del lock, swap único al final. El módulo funcional queda limpio de detalles de concurrencia y se puede testear sin runtime async.

### 9.2 Sobre concurrencia con Tokio

`async/await` permite expresar I/O concurrente con código que se lee secuencial. `Arc<RwLock<…>>` cubre el caso "muchas lecturas, pocas escrituras" (la biblioteca). `tokio::sync::broadcast::channel` resuelve el fan-out: cada WS se suscribe y recibe los `ServerEvent` que cualquier tarea publique, sin que el publicador conozca a los suscriptores. Una observación práctica: **nunca sostener un guard de `RwLock` a través de un `await`** — convención que se siguió extrayendo datos en un bloque `{ … }` y soltando el guard antes de cualquier I/O.

### 9.3 Sobre protocolos de red

Se eligió WebSocket JSON en vez de TCP "crudo" porque los navegadores no pueden abrir sockets TCP arbitrarios. WebSocket es esencialmente TCP con handshake HTTP, así que conserva la persistencia y el orden de los mensajes pero suma compatibilidad con la web. Para audio se eligió HTTP Range: el `<audio>` del navegador hace seek nativo emitiendo nuevas peticiones automáticamente; no se escribe JS para buffer ni seek.

### 9.4 Sobre Rust como lenguaje del backend

El compilador atrapa una clase entera de errores que en lenguajes dinámicos se descubren en producción: usos de variables movidas, datos compartidos sin sincronización, paths de error sin manejar. La curva de aprendizaje es real (ownership, borrowing, lifetimes), pero una vez que el código compila, suele funcionar. El ecosistema de crates fue clave: `tokio` + `axum` para el server, `serde` para JSON, `im` para inmutables, `tower-http`'s `ServeFile`, `id3` y `rspotify`.

### 9.5 Dificultades encontradas

- **Curva de Rust** (ownership y lifetimes) — especialmente al combinar `async` + locks.
- **Bug crítico de persistencia** descubierto a media fase: `save_library`/`save_playlists` existían pero nadie las invocaba; los cambios se perdían al reiniciar. Se corrigió añadiendo un debouncer real conectado al `mpsc::Sender<()>` desde cada handler.
- **`mark_stopped` prematuro** en `http_stream.rs`: se ejecutaba justo después de devolver el `Response`, no cuando el stream terminaba realmente. Rompía la regla "no borrar canción reproduciéndose". Se arregló moviendo `mark_stopped` exclusivamente al evento WS `stop` y al cleanup por desconexión.
- **Caché de medios de Chrome**: tras varios resets de `library.json`, Chrome servía audio antiguo asociado a un `id` que ahora apuntaba a otro archivo. `Cmd+Shift+R` no lo limpiaba. Se arregló con `Cache-Control: no-cache` + cache-buster `?t=<timestamp>`.
- **IDs en UI vs IDs reales**: la columna `01..05` en `SongList.jsx` mostraba `idx + 1` (posición), no el `id` real. Eso confundía cuando se hacía `remove <n>` en la CLI. Se cambió para mostrar `s.id` con padding 2 dígitos; ahora UI ↔ CLI son consistentes.
- **`PlaylistFilter` sin handler**: el match arm en `ws.rs` faltaba y la variante caía silenciosamente en `_ => {}`. Se descubrió revisando warnings de `cargo check` ("function `filter_songs` is never used").
- **Spotify dejó de devolver `preview_url`**: a mitad del proyecto descubrimos que la API de Spotify en 2024 dejó de incluir `preview_url` en la mayoría de los tracks accedidos vía Client Credentials Flow. Esto invalidó parcialmente el modo `add-spotify <id>` (que dependía solo del preview) y nos llevó a priorizar el flujo combinado `add <ruta-mp3> <spotify-id>`: MP3 local para el audio, Spotify solo para metadata y portada.

### 9.6 Recomendaciones / qué cambiaríamos en producción

- **Auth real**: OAuth con Spotify para identificar usuarios y asociar playlists privadas. Hoy todas son globales.
- **Base de datos**: Postgres para la biblioteca y Redis para playback. Reescribir `library.json` completo no escala.
- **CDN para audio**: HLS/DASH con segmentación, calidad adaptativa y caché en el borde.
- **Multi-región con CRDTs**: hoy las playlists viven en un solo `Arc<RwLock<…>>`; producción multi-región requiere CRDTs o event sourcing.
- **Observabilidad**: OpenTelemetry, dashboards Grafana/Datadog para latencia y tasas de error.
- **CI/CD**: Azure SWA + Fly.io/Railway/Kubernetes para deploy automatizado.
- **Tests de integración** que arranquen el servidor real, abran un WS cliente y validen el flujo completo.

### 9.7 Trabajo futuro

- AdminPanel web (formulario de upload + edición desde el frontend).
- Spotify Connect (reproducción coordinada entre dispositivos).
- Letras sincronizadas y cover art animado.
- Colaboración en vivo sobre playlists (cursores y comentarios).
- Recomendaciones con Spotify Audio Features API.
- Tests E2E con Playwright + harness Tokio para el backend.
- Empaquetado: ejecutable nativo con `cargo dist`, cliente desktop con Tauri.
