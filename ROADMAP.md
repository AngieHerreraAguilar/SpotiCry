# SpotiCry — Plan de Implementación

## Contexto

Aplicación estilo Spotify con dos módulos independientes: un servidor en **Rust** (con un módulo de playlists en estilo funcional puro) y un cliente **web React + Vite** responsive. El proyecto cumple requerimientos académicos que enfatizan (1) paradigmas imperativo vs funcional, (2) concurrencia y sockets, (3) transmisión de bytes de audio, (4) documentación comparando operación "con vs sin sincronización".

Decisiones clave ya tomadas con el usuario:
- **Spotify API solo para metadata**; el audio real son archivos MP3 locales. Si una canción no tiene MP3 cargado, se usa el `preview_url` de 30s de Spotify como fallback (avalado por el profesor).
- **Protocolo**: WebSocket JSON (comandos) + HTTP Range (audio). Cambio frente a "TCP crudo" del enunciado se justifica porque los navegadores **no pueden** abrir sockets TCP crudos — WebSocket es TCP con handshake HTTP, el equivalente más cercano en web.
- **Playlists globales compartidas** almacenadas en el servidor (sin auth de usuarios). Esto maximiza el material a documentar sobre concurrencia.
- **Búsqueda**: Título (substring) + Género (lookup `HashMap<Genre, Vec<SongId>>`) + Año (filtro de rango numérico). Tres algoritmos técnicamente distintos.
- **Concurrencia**: Tokio async + Axum (camino más directo dadas las elecciones de protocolo).
- **Persistencia**: archivos JSON en disco (`library.json`, `playlists.json`) leídos al arrancar, escritos al modificar.
- **Despliegue**: Frontend en Azure Static Web Apps vía GitHub Actions; backend corre local y se expone vía Cloudflare Tunnel (gratis, estable, sin abrir puertos).

---

## Arquitectura general

```
 ┌──────────────────────┐        WebSocket JSON          ┌────────────────────────┐
 │  Frontend (React)    │ ◄───── ws://.../ws ──────────► │   Servidor Rust        │
 │  - Vite build        │                                │   - Tokio + Axum       │
 │  - Desplegado en     │        HTTP Range audio        │   - CLI de texto       │
 │    Azure SWA         │ ◄───── GET /stream/:id ─────── │   - WebSocket handler  │
 │  - Zustand state     │                                │   - HTTP audio stream  │
 │  - <audio> + Buffer  │                                │   - Módulo funcional   │
 │                      │                                │     para playlists     │
 └──────────────────────┘                                └──────┬─────────────────┘
                                                                │
                                                                ▼
                                                  ┌─────────────────────────────┐
                                                  │  library.json  (canciones)  │
                                                  │  playlists.json             │
                                                  │  /library/*.mp3  (audio)    │
                                                  └─────────────────────────────┘
```

---

## Estructura del repositorio

```
SpotiCry/
├── server/                 # Backend Rust
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs         # Bootstrap: arranca CLI + servidor axum
│   │   ├── cli.rs          # Interfaz de texto (add/remove canciones)
│   │   ├── domain.rs       # Structs: Song, Playlist, Genre, etc.
│   │   ├── library.rs      # Gestión canciones (imperativo, con RwLock)
│   │   ├── playlists/      # Módulo FUNCIONAL (restricción del enunciado)
│   │   │   ├── mod.rs      # Re-exports
│   │   │   ├── state.rs    # Tipos inmutables (im::HashMap, im::Vector)
│   │   │   └── ops.rs      # Funciones puras: create/add/remove/filter/sort
│   │   ├── playback.rs     # Estado "en reproducción" (Mutex<HashSet<SongId>>)
│   │   ├── persistence.rs  # Lectura/escritura de library.json y playlists.json
│   │   ├── spotify.rs      # Cliente de Spotify API (solo metadata)
│   │   ├── ws.rs           # Handler WebSocket (imperativo, llama al módulo funcional)
│   │   ├── http_stream.rs  # Handler HTTP GET /stream/:id con Range
│   │   └── protocol.rs     # Tipos serde para mensajes WS
│   └── library.json, playlists.json  # se generan al correr
├── client/                 # Frontend React + Vite + JS
│   ├── package.json
│   ├── vite.config.js
│   ├── index.html
│   ├── src/
│   │   ├── main.jsx
│   │   ├── App.jsx         # Shell + routing
│   │   ├── api/
│   │   │   ├── ws.js       # Cliente WebSocket singleton
│   │   │   └── stream.js   # Helpers HTTP Range para audio
│   │   ├── store/
│   │   │   ├── library.js  # Zustand store: canciones
│   │   │   ├── playlists.js
│   │   │   └── player.js   # Estado de reproducción + buffer
│   │   ├── components/
│   │   │   ├── Sidebar.jsx
│   │   │   ├── SongList.jsx
│   │   │   ├── SearchBar.jsx
│   │   │   ├── PlaylistView.jsx
│   │   │   └── Player.jsx  # <audio> + controles seek
│   │   ├── pages/
│   │   │   ├── Home.jsx
│   │   │   ├── Search.jsx
│   │   │   └── Playlist.jsx
│   │   └── styles/
│   └── .github/workflows/azure-swa.yml
├── library/                # MP3s locales (gitignored)
├── docs/
│   ├── problema.md
│   ├── analisis-tecnico.md
│   ├── resultados.md
│   ├── conclusiones.md
│   ├── protocolo.md        # Formato de mensajes
│   └── concurrencia.md     # "Con vs sin sincronización"
└── README.md
```

---

## Backend (Rust)

### Dependencias — `server/Cargo.toml`

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
axum = { version = "0.7", features = ["ws", "macros"] }
tower-http = { version = "0.5", features = ["cors", "fs"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
im = { version = "15", features = ["serde"] }   # colecciones inmutables para playlists
arc-swap = "1"                                   # opcional, publicar estado nuevo sin RwLock
symphonia = { version = "0.5", features = ["mp3"] } # decodificación MP3
id3 = "1"                                        # leer tags ID3
reqwest = { version = "0.12", features = ["json"] }
rspotify = "0.13"                                # cliente Spotify oficial
uuid = { version = "1", features = ["v4"] }
anyhow = "1"
thiserror = "1"
dashmap = "6"                                    # HashMap concurrente
tracing = "0.1"
tracing-subscriber = "0.3"
```

### `src/domain.rs` — modelos compartidos

```rust
pub type SongId = u64;
pub type PlaylistId = u64;

#[derive(Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: SongId,
    pub title: String,        // metadata, DISTINTO del file_path (requerimiento)
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: u16,
    pub duration_secs: u32,
    pub file_path: Option<PathBuf>,     // None ⇒ usar spotify_preview_url
    pub spotify_preview_url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    pub songs: im::Vector<SongId>,
}
```

### `src/library.rs` — gestión canciones (imperativo)

Estado compartido: `Arc<RwLock<HashMap<SongId, Song>>>` + índice secundario `HashMap<Genre, Vec<SongId>>` para lookup O(1) del criterio de búsqueda por género. Funciones públicas: `add_song_from_file`, `remove_song`, `search_by_title`, `search_by_genre`, `search_by_year_range`. **Regla**: `remove_song` consulta `playback::is_playing(id)` y retorna error si es true — **cumple "no se puede eliminar canción en reproducción"**.

### `src/playlists/` — **módulo funcional puro** (RESTRICCIÓN del enunciado)

Patrón: **functional core, imperative shell**. El módulo NO importa `tokio` ni `std::sync` ni tiene `let mut`. Usa `im::HashMap` e `im::Vector` para estructuras inmutables persistentes con sharing O(log n).

```rust
// playlists/state.rs
pub struct State {
    pub playlists: im::HashMap<PlaylistId, Playlist>,
}

// playlists/ops.rs — todas puras, devuelven State nuevo
pub fn create(state: &State, name: String) -> (State, PlaylistId);
pub fn add_song(state: &State, pid: PlaylistId, sid: SongId) -> State;
pub fn remove_song(state: &State, pid: PlaylistId, sid: SongId) -> State;
pub fn reorder(state: &State, pid: PlaylistId, order: im::Vector<SongId>) -> State;

// Queries (sin mutar): usan map/filter/fold + closures obligatoriamente
pub fn filter_songs<F: Fn(&Song) -> bool>(p: &Playlist, lib: &[Song], pred: F) -> im::Vector<Song>;
pub fn sort_by<K: Ord, F: Fn(&Song) -> K>(p: &Playlist, lib: &[Song], key: F) -> im::Vector<Song>;
pub fn total_duration(p: &Playlist, lib: &[Song]) -> u32;  // implementado con .fold
```

El wiring al estado global vive en `ws.rs` (shell imperativo):
```rust
let snapshot = shared.read().await.clone();          // clone O(1) de im
let next = playlists::add_song(&snapshot, pid, sid); // pura, sin lock
*shared.write().await = next;                        // swap
```

### `src/playback.rs` — tracking "en reproducción"

`Arc<Mutex<HashSet<SongId>>>`. API: `mark_playing(id)`, `mark_stopped(id)`, `is_playing(id)`. Consultado por `library::remove_song` y expuesto por WebSocket en el evento `now_playing`. Esto resuelve la distinción **"consultada vs en reproducción"** del enunciado.

### `src/cli.rs` — interfaz de texto del servidor

Corre en una task paralela (`tokio::spawn`). Comandos:
- `add <ruta-mp3>` — lee tags ID3, registra en library, opcionalmente consulta Spotify para enriquecer metadata.
- `add-spotify <track-id>` — registra solo metadata + preview_url de Spotify, sin MP3.
- `remove <song-id>` — llama a `library::remove_song` (falla si está sonando).
- `list` — imprime biblioteca.
- `playlists` — imprime playlists globales.
- `quit` — flushea JSON y cierra.

### `src/ws.rs` — handler WebSocket

Axum handler `async fn ws_handler(ws: WebSocketUpgrade, State(app): State<AppState>)`. Loop:
```
while let Some(msg) = socket.recv().await {
    match parse_json::<ClientMsg>(msg)? {
        ClientMsg::Search { by, value } => ...
        ClientMsg::PlaylistCreate { name } => { /* llamada al módulo funcional */ }
        ClientMsg::Play { song_id } => { playback::mark_playing(...); broadcast_now_playing(...) }
        ...
    }
}
```
Los mensajes broadcast (ej: otro cliente crea una playlist) se enrutan vía un `tokio::sync::broadcast::channel` al que cada handler está suscrito.

### `src/http_stream.rs` — streaming de audio

Endpoint `GET /stream/:id`:
1. Si `song.file_path.is_some()`: sirve el archivo local con soporte nativo de `Range` (usa `tower_http::services::ServeFile` o implementación manual que lee `Range: bytes=X-Y` y responde `206 Partial Content` con el slice).
2. Si solo hay `spotify_preview_url`: hace un proxy hacia Spotify (`reqwest::get(preview_url).await`) y pipelinea el body.
3. Antes de servir, llama `playback::mark_playing(id)`. Al cerrar la conexión o recibir un byte range final, `mark_stopped`.

El chunking natural de HTTP Range (64 KB default del navegador) resuelve el "envío de paquetes de bytes" del enunciado. El **buffer local** de adelantar/retroceder que pide el enunciado lo implementa el cliente (el navegador ya bufferea en el `<audio>` element y responde con nuevas Range requests al hacer seek).

### `src/persistence.rs`

Al arrancar: lee `library.json` y `playlists.json` → hidrata estado. Al cada mutación exitosa: escribe JSON (debounced 500ms para no escribir por cada operación). Formato: `serde_json` con pretty print para ser legible en debug.

### `src/spotify.rs`

Cliente `rspotify` con Client Credentials flow (no requiere login de usuario, solo keys de app). Usado **solo desde la CLI** para enriquecer metadata cuando se agrega un MP3, o para registrar canciones "solo preview".

---

## Protocolo de mensajes (`docs/protocolo.md`)

### Cliente → Servidor (WebSocket, JSON)

```json
{ "op": "search", "by": "title" | "genre" | "year_range", "value": "rock" | [1990, 2000] }
{ "op": "library.list" }
{ "op": "playlist.list" }
{ "op": "playlist.create", "name": "Road Trip" }
{ "op": "playlist.delete", "playlist_id": 3 }
{ "op": "playlist.add", "playlist_id": 3, "song_id": 42 }
{ "op": "playlist.remove", "playlist_id": 3, "song_id": 42 }
{ "op": "playlist.filter", "playlist_id": 3, "by": "genre", "value": "rock" }
{ "op": "playlist.sort", "playlist_id": 3, "by": "title" | "year" | "duration" }
{ "op": "play", "song_id": 42 }
{ "op": "stop", "song_id": 42 }
```

### Servidor → Cliente (WebSocket, JSON)

```json
{ "ev": "library.snapshot", "songs": [...] }
{ "ev": "playlist.snapshot", "playlists": [...] }
{ "ev": "search.result", "songs": [...] }
{ "ev": "now_playing", "song_id": 42, "stream_url": "/stream/42" }
{ "ev": "playback.stopped", "song_id": 42 }
{ "ev": "error", "code": "CANNOT_DELETE_PLAYING" | "NOT_FOUND", "msg": "..." }
```

Audio: `GET http://localhost:8080/stream/:id` con `Range: bytes=X-Y` → `206 Partial Content`. El navegador lo maneja nativo con `<audio src="...">`.

---

## Frontend (React + Vite + JS)

### Dependencias — `client/package.json`

```json
{
  "dependencies": {
    "react": "^19",
    "react-dom": "^19",
    "react-router-dom": "^7",
    "zustand": "^5"
  },
  "devDependencies": {
    "vite": "^6",
    "@vitejs/plugin-react": "^4"
  }
}
```

Sin librería de UI — diseño directo con CSS modules según el design system de Figma (ver sección siguiente).

### Módulos clave

- **`api/ws.js`**: abre WebSocket a `VITE_WS_URL`. Expone `sendCmd(op, payload)` y suscribe eventos a listeners. Reconecta con backoff.
- **`api/stream.js`**: construye URL `${VITE_HTTP_URL}/stream/${songId}`. El elemento `<audio>` de HTML5 usa Range automáticamente.
- **`store/player.js`** (Zustand): `{ currentSong, isPlaying, currentTime, duration, seekTo(t), playSong(s), stop() }`. Internamente usa un ref a `<audio>`. **Seek/adelantar/retroceder** = `audio.currentTime = t` (el buffer lo maneja el navegador, el audio element hace nueva Range request si es necesario).
- **`store/library.js`**: lista de canciones; se sincroniza con `library.snapshot` del servidor.
- **`store/playlists.js`**: lista de playlists; acciones que disparan `sendCmd('playlist.xxx', ...)`.
- **`components/SearchBar.jsx`**: tabs "Título / Género / Año" — cada tab envía un `op: search, by: ...` distinto al servidor.
- **`components/Player.jsx`**: barra inferior fija. `<audio>` oculto + controles custom. En mobile se expande.
- **`components/PlaylistView.jsx`**: muestra canciones + botón "Agregar a otra playlist" (cumple "puede agregarse a más de una playlist").
- **Responsive**: CSS container queries + breakpoint a 768px. Sidebar → bottom nav en mobile.

---

## Design System

**Archivo Figma del proyecto**: https://www.figma.com/design/vVqitTtryzsMQLtQrhhRYr/Proyecto-Lenguajes
Pantallas provistas (nodos): `1-2`, `1-120`, `111-84`, `1-1837`, `1-1966`, `132-376`, `1-260`, `1-611`, `1-1214`, `1-2217`.

**Limitación**: el MCP de Figma no está disponible en esta sesión de Claude y las URLs de Figma requieren autenticación (bloquean `WebFetch`). Para construir el design system se necesita una de estas tres vías:
1. **Screenshots/exports PNG** de las 10 pantallas pegados en el chat (más rápido y preciso).
2. **Tokens del Figma exportados a JSON** (vía plugin como Design Tokens / Variables2JSON) o descritos en texto: paleta, tipografía, spacing.
3. **Design system propuesto por mí** (Spotify-inspirado: fondo oscuro, acentos verdes, sidebar de navegación) que luego se ajusta al Figma.

**Timing**: el design system no bloquea Días 1-2 (solo backend). Se resuelve al inicio del Día 3, antes de escribir CSS. Si para entonces no hay assets compartidos, se usa la propuesta Spotify-inspirada como base.

**Lo que se extraerá cuando estén los assets**:
- Paleta de colores → tokens CSS variables (`--bg`, `--surface`, `--accent`, `--text`, etc.)
- Tipografía (familias, escalas, pesos)
- Espaciado y grid
- Componentes reutilizables (botones, inputs, cards, song row)
- Layouts desktop + mobile

Mientras tanto, el frontend se esqueletiza con placeholders y clases semánticas (`.song-card`, `.player-bar`, `.sidebar`, etc.) para que al llegar los tokens el cambio sea localizado y rápido.

---

## Despliegue

### Frontend — Azure Static Web Apps

1. Repositorio GitHub con el proyecto.
2. En Azure Portal → Create Static Web App → conectar a repo → apuntar a `client/` como app root, `dist` como output.
3. Azure genera `.github/workflows/azure-swa.yml` automáticamente. Build command: `npm run build`.
4. Variables de entorno en Azure SWA config: `VITE_WS_URL=wss://tunnel.yourdomain.dev/ws`, `VITE_HTTP_URL=https://tunnel.yourdomain.dev`.

### Backend — local + Cloudflare Tunnel

1. `cargo run` en la máquina del grupo para correr el servidor en `localhost:8080`.
2. Instalar `cloudflared` (free). Ejecutar `cloudflared tunnel --url http://localhost:8080`.
3. Cloudflare devuelve una URL pública `https://xxx.trycloudflare.com` que expone el servidor sin abrir puertos, sin VPS, y con TLS automático (requerido por Azure SWA que es HTTPS).
4. Poner esa URL en las env vars del frontend.

Trade-off documentado: la URL del tunnel cambia cada vez que se reinicia (a menos que se registre un tunnel nombrado — también gratis con una cuenta Cloudflare + un dominio).

---

## Documentación (`docs/`)

Estructura obligatoria del enunciado, ya mapeada a archivos:

- `problema.md` — Descripción del problema y objetivos.
- `analisis-tecnico.md` — Arquitectura, stack, justificación de decisiones (WebSocket vs TCP crudo, playlists en servidor, etc.). Incluye diagrama.
- `resultados.md` — Qué se logró, screenshots, métricas.
- `conclusiones.md` — Qué se aprendió, qué cambiaría.
- `protocolo.md` — Formato de mensajes JSON (tabla con `op`/`ev`/parámetros).
- `concurrencia.md` — **Sección clave del enunciado**: explica el estado compartido (`Arc<RwLock>`), qué pasa sin sincronización (race conditions con ejemplo concreto: dos clientes agregando a la misma playlist → una operación se pierde), cómo el patrón functional-core + `im::HashMap` + swap atómico lo resuelve. Incluye reflexión sobre escala a producción: qué cambiarían (Redis, event sourcing, CRDTs, auth real, S3 para audio, CDN).

Énfasis documental que pide el enunciado y donde los tres miembros deben escribir:
1. **Carga/decodificación/transmisión de audio** — cubrir `http_stream.rs` + Range + cómo el navegador bufferea.
2. **Administración en memoria** — `library.rs` con RwLock vs `playlists/` con `im`. Por qué la elección en cada módulo.
3. **Conflictos y excepciones** — enumerar casos: canción en reproducción no borrable, cliente se desconecta durante stream, archivo MP3 corrupto, Spotify API rate-limited.
4. **Hallazgos con/sin sincronización** — ver `concurrencia.md`.

---

## Verificación end-to-end

### Nivel 1 — backend aislado
```bash
cd server
cargo run
# otra terminal: usar CLI
> add ./sample.mp3
> list
> remove 1
# otra terminal: probar WS con websocat
websocat ws://localhost:8080/ws
{"op":"library.list"}
# otra terminal: probar streaming
curl -H "Range: bytes=0-65535" http://localhost:8080/stream/1 -o chunk.bin
```

### Nivel 2 — frontend en dev
```bash
cd client
VITE_WS_URL=ws://localhost:8080/ws VITE_HTTP_URL=http://localhost:8080 npm run dev
# abrir http://localhost:5173
# probar: buscar por título/género/año, crear playlist, agregar canción, reproducir, seek, adelantar/retroceder
```

### Nivel 3 — concurrencia (crítico para documentación)
Abrir 2-3 pestañas del frontend simultáneamente. Desde cada una, agregar canciones a la misma playlist. Verificar que **ninguna se pierda** (valida el pattern con `im` + swap). Escribir el resultado en `docs/concurrencia.md`.

### Nivel 4 — prueba del requerimiento "no borrar canción reproduciéndose"
Reproducir una canción desde un cliente. Desde la CLI del servidor, intentar `remove <id>` de esa canción → debe fallar con mensaje claro. Detener reproducción → `remove` ahora funciona.

### Nivel 5 — despliegue
Subir frontend a Azure SWA (GitHub Actions hace el build y deploy automático en cada push a `main`). Exponer backend con `cloudflared tunnel`. Verificar que el frontend en Azure conecta al WebSocket de Cloudflare y reproduce audio.

---

## Roadmap comprimido a 4 días

Dado el tiempo disponible (3-4 días), el plan se ejecuta en **4 sprints diarios** con trabajo paralelo. El Día 4 es **colchón**: si se pierde un día, su contenido se absorbe en los anteriores o se recortan stretch goals.

### Alcance: MVP vs Stretch

**MVP no negociable** (cumple el enunciado):
- Backend Rust completo: CLI + biblioteca + búsqueda 3 criterios + streaming + playlists funcional + WebSocket
- **Persistencia JSON** (library.json + playlists.json)
- **Integración Spotify API** (metadata + preview_url fallback) — asignada fija a Persona 1
- Frontend React: buscar, reproducir, seek, crear playlists, agregar canciones
- Documentación obligatoria del enunciado (4 secciones)
- Corriendo en localhost

**Stretch goals** (si sobra tiempo):
- Despliegue en Azure SWA + Cloudflare Tunnel
- Responsive mobile pulido
- `AdminPanel` web completo (la CLI ya cumple el requerimiento)

### Día 1 — Kickoff + Backend base
**Mañana conjunta (2-3h):**
- Crear repo GitHub + scaffolding (`cargo new server`, `npm create vite@latest client`)
- Congelar `domain.rs` y redactar `docs/protocolo.md` (contrato WS)
- Confirmar rutas, nombres, y ejemplos de mensajes

**Tarde paralelo:**
- Persona A: `domain.rs`, `library.rs` con los 3 criterios de búsqueda, `cli.rs` básica
- Persona B: `protocol.rs`, axum bootstrap, `ws.rs` con echo + `library.list`

**Checkpoint fin de día**: desde `websocat` se puede listar canciones agregadas vía CLI.

### Día 2 — Reproducción + Playlists funcional + Frontend arranque
**Paralelo:**
- Persona A: `playback.rs` (tracking reproducción), `http_stream.rs` (Range requests), probar con `curl --range`
- Persona B: `playlists/state.rs` + `playlists/ops.rs` (el **módulo funcional** con `im` + closures + map/filter/fold), integrar en `ws.rs`, arrancar frontend (Vite corriendo, `api/ws.js` conectando)

**Checkpoint fin de día**: 
- Backend: crear playlist, agregar canción, reproducir audio (todo vía `websocat` + `curl`)
- Frontend: página en blanco conecta al WS y recibe `library.snapshot`

### Día 3 — Frontend UI + integración end-to-end
**Paralelo:**
- Persona A: `SongList.jsx`, `SearchBar.jsx` con 3 tabs, `Player.jsx` con `<audio>` + seek, stores `library` y `player`, `api/stream.js`
- Persona B: `App.jsx` + routing, `Sidebar.jsx`, `PlaylistView.jsx`, store `playlists`, CSS base con breakpoint desktop/mobile

**Checkpoint fin de día**: app funciona end-to-end en `localhost:5173` — buscar, reproducir con seek, crear playlists, agregarles canciones.

### Día 4 — Documentación + Deploy (stretch) + QA
**Mañana paralelo:**
- Persona A: `problema.md`, `analisis-tecnico.md`, `resultados.md`
- Persona B: `concurrencia.md` (crítico del enunciado), `conclusiones.md`

**Tarde conjunta (QA):**
- Pruebas de concurrencia multi-pestaña (para `concurrencia.md`)
- Verificar "no se puede borrar canción reproduciéndose"
- Si da tiempo: despliegue Azure SWA + Cloudflare Tunnel (Persona B)
- Si da tiempo: integración Spotify API (Persona A)
- Grabar screencast como respaldo si el deploy falla
- Entrega

### Recortes si el tiempo aprieta

Orden de prioridad para recortar (de primero a último):
1. **Despliegue en nube** → solo localhost + screencast de demo
2. **AdminPanel web** → la CLI ya cumple el requerimiento del enunciado
3. **Responsive mobile pulido** → solo desktop bien terminado
4. **Todo lo demás es MVP, no recortable** (incluyendo persistencia JSON y Spotify API)

---

## División de responsabilidades (2 personas)

**Principio**: el compañero hace **solo backend Rust** (es su preferencia). Tú (usuaria) eres **lead del frontend** y además contribuyes con 2 módulos pequeños del backend como apoyo — así también tocas Rust. La **API de Spotify** queda fija en su lado (no se negocia quién la hace).

### 👤 Persona 1 — Backend only (compañero)

**Backend Rust (todo excepto 2 módulos):**
- [server/Cargo.toml](server/Cargo.toml) — dependencias
- [server/src/main.rs](server/src/main.rs) — bootstrap: arranca CLI + axum
- [server/src/domain.rs](server/src/domain.rs) — tipos `Song`, `Playlist`
- [server/src/library.rs](server/src/library.rs) — gestión canciones + **3 criterios de búsqueda** (imperativo)
- [server/src/playback.rs](server/src/playback.rs) — tracking "en reproducción" (resuelve "no borrar canción sonando")
- [server/src/persistence.rs](server/src/persistence.rs) — JSON load/save (library + playlists)
- [server/src/ws.rs](server/src/ws.rs) — handler WebSocket (shell imperativo + broadcast)
- [server/src/http_stream.rs](server/src/http_stream.rs) — streaming HTTP con Range
- [server/src/playlists/state.rs](server/src/playlists/state.rs) — tipos inmutables (`im::HashMap`)
- [server/src/playlists/ops.rs](server/src/playlists/ops.rs) — **módulo funcional puro** (map/filter/fold/closures)
- [server/src/spotify.rs](server/src/spotify.rs) — **cliente Spotify (fijo en su lado, no stretch)**

**Documentación técnica (2 docs):**
- [docs/analisis-tecnico.md](docs/analisis-tecnico.md) — arquitectura, stack, patrón functional-core, decisiones de memoria
- [docs/concurrencia.md](docs/concurrencia.md) — **pieza clave**: "con vs sin sincronización", `Arc<RwLock>` vs `ArcSwap`, reflexión a producción

### 👤 Persona 2 — Frontend lead + 2 módulos backend de apoyo (tú)

**Backend Rust (2 módulos de apoyo):**
Estos dos módulos son los más auto-contenidos de todo el backend y son excelentes entradas a Rust. No bloquean a Persona 1.
- [server/src/cli.rs](server/src/cli.rs) — **interfaz de texto del servidor** (add/remove/list canciones). Autocontenido, no usa async/await, ideal para aprender Rust básico. Encaja contigo porque conecta con la idea de "interfaz" que ya manejas en el frontend.
- [server/src/protocol.rs](server/src/protocol.rs) — tipos `serde` para mensajes WebSocket. Te obliga a dominar el contrato entre servidor y cliente, lo cual te beneficia directo para el frontend. Código corto (~80-150 líneas).

> **Cómo colaborar**: escribes estos dos archivos con tu compañero como reviewer/mentor. Él valida que compilen y encajen con los demás módulos.

**Frontend React (todo):**
- [client/package.json](client/package.json) + [client/vite.config.js](client/vite.config.js) — scaffolding
- [client/src/main.jsx](client/src/main.jsx) + [client/src/App.jsx](client/src/App.jsx) — shell + routing
- [client/src/api/ws.js](client/src/api/ws.js) — cliente WebSocket con reconexión
- [client/src/api/stream.js](client/src/api/stream.js) — helper HTTP audio
- [client/src/store/library.js](client/src/store/library.js) — estado biblioteca
- [client/src/store/playlists.js](client/src/store/playlists.js) — estado playlists
- [client/src/store/player.js](client/src/store/player.js) — estado reproductor + seek/buffer
- [client/src/components/Sidebar.jsx](client/src/components/Sidebar.jsx) — nav (desktop sidebar / mobile bottom nav)
- [client/src/components/SearchBar.jsx](client/src/components/SearchBar.jsx) — **3 criterios** como tabs
- [client/src/components/SongList.jsx](client/src/components/SongList.jsx)
- [client/src/components/PlaylistView.jsx](client/src/components/PlaylistView.jsx) — vista + ordenamiento
- [client/src/components/Player.jsx](client/src/components/Player.jsx) — `<audio>` + controles + seek
- [client/src/components/AdminPanel.jsx](client/src/components/AdminPanel.jsx) — UI web para add/remove (complementa la CLI)
- [client/src/pages/Home.jsx](client/src/pages/Home.jsx), [Search.jsx](client/src/pages/Search.jsx), [Playlist.jsx](client/src/pages/Playlist.jsx)
- **Design System**: traducción del Figma a tokens CSS + estilos responsive (desktop + mobile breakpoint a 768px)

**Despliegue (stretch Día 4):**
- [.github/workflows/azure-swa.yml](.github/workflows/azure-swa.yml) — CI/CD del frontend
- Azure Static Web Apps + variables de entorno + Cloudflare Tunnel

**Documentación (3 docs narrativas):**
- [docs/problema.md](docs/problema.md) — descripción del problema y objetivos
- [docs/resultados.md](docs/resultados.md) — qué se logró, screenshots
- [docs/conclusiones.md](docs/conclusiones.md) — conclusiones y recomendaciones para producción

### Trabajo conjunto (Día 1)
- [docs/protocolo.md](docs/protocolo.md) — contrato WS escrito conjuntamente **antes** de codear. Tú manejas los tipos del lado JS, Persona 1 los del lado Rust (su `protocol.rs` será lo que tú escribas al final).

### Matriz de días × personas (4 días)

| Día | Persona 1 (solo Backend) | Persona 2 (Frontend + 2 módulos Rust) |
|---|---|---|
| **1 — Kickoff + base** | Scaffolding Rust, `Cargo.toml`, `main.rs`, `domain.rs`, `library.rs` con 3 búsquedas, `persistence.rs`, `playback.rs` | Scaffolding React+Vite, prepara tokens CSS del Figma, co-diseña `protocolo.md` con P1, empieza `cli.rs` (con guía de P1) |
| **2 — Core backend + API cliente** | `http_stream.rs`, `ws.rs` handlers, `playlists/state.rs` + `playlists/ops.rs` (funcional), `spotify.rs` | Termina `cli.rs` y escribe `protocol.rs` con reviews de P1; `api/ws.js`, `api/stream.js`, stores `library`/`playlists`/`player` |
| **3 — UI + integración** | Modo soporte del backend (bugfixes), empieza `analisis-tecnico.md` y `concurrencia.md` | `App.jsx` + routing, `Sidebar.jsx`, `SearchBar.jsx`, `SongList.jsx`, `PlaylistView.jsx`, `Player.jsx`, `AdminPanel.jsx`, design system CSS completo |
| **4 — Deploy + Docs + QA** | Termina `analisis-tecnico.md` + `concurrencia.md`, tests de integración, verifica "no borrar canción sonando" | Azure SWA + Cloudflare (stretch), `problema.md`, `resultados.md`, `conclusiones.md`, QA multi-pestaña con P1 |

### Sincronización

- **Día 1 fin**: `domain.rs` + `protocolo.md` **congelados**. P1 debe haber publicado `domain.rs` para que P2 pueda arrancar `cli.rs` con los tipos correctos.
- **Día 2 mitad**: P1 publica build del servidor con WS respondiendo `library.list` y `playlist.create` para que P2 integre el cliente. `protocol.rs` que escribe P2 debe sincronizar con lo que P1 ya tiene en `ws.rs`.
- **Día 3 inicio**: servidor 100% operativo. P2 ya no depende del código de P1, solo del contrato.
- **Día 4**: demos cruzadas antes de entrega.

### Balance estimado

| | Persona 1 | Persona 2 |
|---|---|---|
| Backend Rust | 11 archivos (100% menos 2 módulos) | 2 archivos (`cli.rs` + `protocol.rs`) + design/review del contrato |
| Frontend | — | ~14 archivos + design system + CSS responsive |
| Deploy | — | GitHub Actions + Azure + Cloudflare |
| Docs | 2 técnicas densas | 3 narrativas + `protocolo.md` conjunto |
| **Total esperado** | **~50%** | **~50%** |

Persona 1 consigue el perfil puro-backend que quiere. Tú aprendes Rust con 2 módulos concretos pero manejables, y eres dueña completa del frontend y despliegue.

---

## Archivos críticos a crear (resumen)

| Archivo | Rol |
|---|---|
| [server/Cargo.toml](server/Cargo.toml) | Dependencias Rust |
| [server/src/main.rs](server/src/main.rs) | Entry point: arranca CLI + axum |
| [server/src/domain.rs](server/src/domain.rs) | Tipos `Song`, `Playlist` |
| [server/src/library.rs](server/src/library.rs) | Gestión canciones imperativa |
| [server/src/playlists/state.rs](server/src/playlists/state.rs) | `State` con `im::HashMap` |
| [server/src/playlists/ops.rs](server/src/playlists/ops.rs) | **Funciones puras con map/filter/fold/closures** |
| [server/src/playback.rs](server/src/playback.rs) | Tracking "en reproducción" |
| [server/src/cli.rs](server/src/cli.rs) | Interfaz texto del servidor |
| [server/src/ws.rs](server/src/ws.rs) | Handler WebSocket (shell imperativo) |
| [server/src/http_stream.rs](server/src/http_stream.rs) | Streaming HTTP con Range |
| [server/src/persistence.rs](server/src/persistence.rs) | JSON load/save |
| [server/src/spotify.rs](server/src/spotify.rs) | Cliente `rspotify` |
| [client/package.json](client/package.json) | Dependencias React |
| [client/src/api/ws.js](client/src/api/ws.js) | Cliente WebSocket |
| [client/src/store/player.js](client/src/store/player.js) | Estado reproductor + seek |
| [client/src/components/Player.jsx](client/src/components/Player.jsx) | `<audio>` + controles |
| [client/src/components/SearchBar.jsx](client/src/components/SearchBar.jsx) | 3 criterios de búsqueda |
| [.github/workflows/azure-swa.yml](.github/workflows/azure-swa.yml) | CI/CD del frontend |
| [docs/concurrencia.md](docs/concurrencia.md) | **Pieza clave de documentación** |
| [docs/protocolo.md](docs/protocolo.md) | Formato de mensajes |
