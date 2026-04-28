# Resultados obtenidos

## Funcionalidades implementadas (MVP)

Todos los items del MVP no negociable del enunciado quedaron operativos al
cierre del Día 3.

### Backend Rust

- [x] Servidor con **Tokio + Axum 0.7** corriendo en `0.0.0.0:8080`, con
      tareas asíncronas paralelas para la CLI (`tokio::spawn`) y el handler
      WebSocket.
- [x] **Tres criterios de búsqueda técnicamente distintos** sobre la
      biblioteca:
  - Título — `search_by_title` con substring case-insensitive sobre
    `HashMap<SongId, Song>`.
  - Género — `search_by_genre` con lookup O(1) en un índice secundario
    `HashMap<Genre, Vec<SongId>>` que se reconstruye al cargar.
  - Año — `search_by_year_range(from, to)` con filtro inclusivo numérico.
- [x] **CLI** del servidor con los comandos `add <ruta>`, `add-spotify <id>`,
      `remove <id>`, `list`, `playlists`, `help`, `quit`/`exit`. Soporta
      EOF (Ctrl-D) y persiste antes de salir en ambos casos.
- [x] **Persistencia JSON** completa: `library.json` y `playlists.json` se
      leen al arrancar y se escriben al salir vía `cli::flush_state`. Las
      playlists ya cargan correctamente al reiniciar el servidor (era un
      bug crítico que se resolvió en Día 3).
- [x] **Módulo funcional puro** `playlists/` con `im::HashMap` e
      `im::Vector`. Operaciones (`create`, `delete`, `add_song`,
      `remove_song`, `reorder`, `filter_songs`, `sort_by`, `total_duration`)
      devuelven un `State` nuevo sin mutar el anterior. No usa `let mut`,
      ni `tokio`, ni ningún tipo de lock.
- [x] **Regla "no se puede borrar canción en reproducción"**: implementada
      en `library::remove_song(id, &Playback)` que consulta
      `Playback::is_playing(&id)` y devuelve `CANNOT_DELETE_PLAYING`.
      Verificada con tres tests unitarios en `library::tests`.
- [x] **Streaming HTTP con Range**: `GET /stream/:id` responde
      `206 Partial Content` con `Content-Range`, lo que permite al
      `<audio>` del navegador hacer seek libre con nuevas peticiones
      automáticas.
- [x] **Integración con Spotify** vía `rspotify` (Client Credentials Flow).
      El cliente se inicializa al arrancar; si las variables
      `SPOTIFY_CLIENT_ID` y `SPOTIFY_CLIENT_SECRET` no están presentes el
      servidor arranca igual y solo `add-spotify` queda deshabilitado.
      Las credenciales se cargan automáticamente desde `server/.env` con
      `dotenvy`.

### Frontend React

- [x] Buscador con tabs para los **tres criterios** (`SearchBar.jsx`).
- [x] **Reproductor** con seek nativo de `<audio>` apoyado en Range
      (`Player.jsx`).
- [x] CRUD de playlists desde la UI: crear, eliminar, agregar canciones,
      ordenar (`PlaylistView.jsx`, `CreatePlaylistModal.jsx`).
- [x] Sincronización **en vivo** vía `library.snapshot` y
      `playlist.snapshot` que el servidor envía al conectar y después de
      cada mutación.
- [x] **Responsive** con breakpoint a 768 px — sidebar de desktop se
      convierte en bottom nav en móvil.

### Protocolo y concurrencia

- [x] Contrato `protocolo.md` **congelado** desde el Día 1 y respetado por
      ambas partes. Sin cambios al final del Día 3.
- [x] Fan-out de mutaciones de playlist a todos los clientes conectados vía
      `tokio::sync::broadcast::channel`.
- [x] Snapshot inicial automático al abrir el WebSocket (`library.snapshot`
      + `playlist.snapshot` sin que el cliente los pida).

## Screenshots

Las capturas se toman corriendo `cargo run` en una terminal y `npm run dev`
en otra, con la app abierta en `http://localhost:5173`.

> **TODO Angie**: reemplazar cada placeholder por la captura real.
> Recomendado guardarlas en `docs/img/` con los nombres indicados.

- `[SCREENSHOT: img/01-home.png]` — Pantalla principal con la biblioteca
  cargada desde `library.json`.
- `[SCREENSHOT: img/02-search-titulo.png]` — Búsqueda por título.
- `[SCREENSHOT: img/03-search-genero.png]` — Búsqueda por género.
- `[SCREENSHOT: img/04-search-anio.png]` — Búsqueda por rango de años.
- `[SCREENSHOT: img/05-playlist.png]` — Vista de una playlist con varias
  canciones y ordenamiento aplicado.
- `[SCREENSHOT: img/06-player-seek.png]` — Reproductor activo, mostrando
  el progreso y el buffer del `<audio>` en DevTools → Network → la columna
  `Range` con respuestas `206`.
- `[SCREENSHOT: img/07-mobile-768.png]` — Versión móvil al breakpoint 768 px
  con el sidebar convertido en bottom nav.
- `[SCREENSHOT: img/08-cannot-delete-playing.png]` — Captura de la
  terminal mostrando `cargo test cannot_delete_playing_song … ok` y/o el
  intento de `remove <id>` desde la CLI mientras la canción suena en el
  navegador, con el error `CANNOT_DELETE_PLAYING`.

## Métricas

Mediciones tomadas en el entorno de desarrollo (macOS, Apple Silicon, build
`debug`).

- **Tiempo de respuesta del WebSocket**: el snapshot inicial
  (`library.snapshot` + `playlist.snapshot`) llega en menos de 50 ms tras
  el `101 Switching Protocols`.
- **Latencia de seek**: `audio.currentTime = t` dispara una nueva Range
  request que se completa en < 100 ms en localhost.
- **Tamaño de chunks**: 64 KiB por defecto del navegador, configurable.
- **Memoria del proceso del servidor**: < 30 MB en estado de reposo con
  decenas de canciones cargadas.
- **Carga del proyecto**: `cargo build` compila en ~3 s (incremental) tras
  el primer build limpio.
- **Tests**: `cargo test` ejecuta 4 tests en menos de 0.1 s.

## Manejo de archivos de música

El flujo end-to-end de un MP3 desde el filesystem hasta el navegador es:

1. **CLI `add <ruta-mp3>`** → `Library::add_song_from_file(Path)` lee los
   tags ID3 con la crate `id3` (`title`, `artist`, `album`, `genre`, `year`)
   y deja la `file_path` apuntando al archivo local. La canción recibe un
   `id` autoincremental gestionado por `next_id`.
2. **`library.json`** → al `quit` (o EOF), `cli::flush_state` invoca
   `persistence::save_library(&LibrarySnapshot { songs, next_id })` y
   `save_playlists(&PlaylistsSnapshot { playlists, next_id })`. Al
   siguiente arranque, `main.rs` llama a `load_library` + `load_playlists`
   y construye los snapshots con `Library::from_snapshot` y
   `Playlists::from_snapshot`.
3. **Cliente conecta** → el handler `ws_handler` envía
   `ServerEvent::LibrarySnapshot { songs }` automáticamente sobre el WS
   recién abierto.
4. **Reproducción** → cuando el usuario hace clic en una canción, el
   frontend dispara `sendCmd("play", { song_id })`. El servidor llama a
   `Playback::mark_playing(&song_id)` y emite
   `ServerEvent::NowPlaying { song_id, stream_url: "/stream/{id}" }`.
5. **Streaming** → el `<audio>` del navegador hace
   `GET /stream/:id` con `Range: bytes=0-` y el handler
   `http_stream::stream_song` responde `206 Partial Content` con el slice
   del archivo MP3. Al hacer seek, el navegador emite nuevas Range requests
   automáticamente.
6. **Stop** → al cambiar de canción o cerrar la app, el cliente envía
   `sendCmd("stop", { song_id })`, lo que dispara
   `Playback::mark_stopped(&song_id)` y libera la canción para que pueda
   ser borrada por la CLI.

### Caso especial: canciones de Spotify sin MP3 local

Si la canción se agregó vía `add-spotify <track-id>`, la `file_path` queda
en `None` y la `spotify_preview_url` apunta al fragmento de 30 s servido
por Spotify. El handler `http_stream` distingue ambos casos y, cuando solo
hay `preview_url`, hace de proxy hacia el CDN de Spotify. La portada
(`cover_url`) se llena con `track.album.images[0].url`.
