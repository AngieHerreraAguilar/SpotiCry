# Guía de Kevin — Backend Rust (Persona 1)

> Esta guía es **tuya**. Léela completa antes de empezar. Si algo no está claro, pregunta a Angie **antes** de codear — un malentendido en el contrato entre backend y frontend cuesta horas después.

## 1. Contexto rápido

- Proyecto académico: app estilo Spotify.
- Dos módulos: **servidor Rust** (lo tuyo) + **cliente web React** (lo de Angie).
- Comunicación: WebSocket JSON para comandos + HTTP Range para audio.
- **Tu scope: 100% backend Rust + cliente Spotify + 2 docs técnicas.** No toques frontend ni deploy.
- Tiempo: **4 días**. Ver [ROADMAP.md](../ROADMAP.md) para el plan completo.

## 2. Setup inicial (15 min)

```bash
# 1. Acepta la invitación al repo (GitHub → notifications).
# 2. Clona vía SSH:
git clone git@github.com:AngieHerreraAguilar/SpotiCry.git
cd SpotiCry

# 3. Verifica que Rust y Cargo estén instalados:
cargo --version
# si no: https://rustup.rs  → curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 4. Compila el servidor (tarda la primera vez mientras baja deps):
cd server
cargo check
# Esperado: compila con warnings de "unused" — son los stubs, está OK.

# 5. Confirma que puedes correr:
cargo run
# Esperado: imprime "SpotiCry server starting (stub)" y termina (todavía no hace nada).
```

Si `cargo check` falla, avísale a Angie **antes** de seguir. No intentes "arreglarlo" a ciegas.

## 3. Tus archivos (no toques los de Angie)

### Archivos TUYOS (Rust, en `server/src/`)

| Archivo | Qué hace | Cuándo |
|---|---|---|
| `main.rs` | Bootstrap: arranca CLI + axum | Día 1 |
| `domain.rs` | Tipos `Song`, `Playlist` — **CONGELAR fin Día 1** | Día 1 |
| `library.rs` | Canciones + 3 búsquedas (imperativo) | Día 1 |
| `persistence.rs` | Load/save JSON | Día 1 |
| `playback.rs` | Tracking "en reproducción" | Día 1-2 |
| `ws.rs` | Handler WebSocket (imperative shell) | Día 2 |
| `http_stream.rs` | Streaming HTTP Range | Día 2 |
| `playlists/state.rs` | Estado inmutable | Día 2 |
| `playlists/ops.rs` | **Módulo funcional puro** (ver §6) | Día 2 |
| `spotify.rs` | Cliente Spotify | Día 2-4 |

### Archivos de Angie — **NO tocar** sin avisarle

- `server/src/cli.rs` — ella lo escribe
- `server/src/protocol.rs` — ella lo escribe, pero contiene los tipos que TÚ necesitas en `ws.rs`. Si necesitas cambiar algo, **avísale primero** (rompe el contrato con el frontend).
- Todo lo que está en `client/`, `docs/protocolo.md`, `.github/workflows/`.

### Tus docs (en `docs/`)

- [`analisis-tecnico.md`](analisis-tecnico.md) — arquitectura, stack, patrón functional-core.
- [`concurrencia.md`](concurrencia.md) — **pieza clave del enunciado**: "con vs sin sincronización".

## 4. Convenciones del equipo

- **Una rama por tarea**: `git checkout -b kevin/library-searches`. No hagas commits directo en `main`.
- **Pull request** cuando termines una tarea: `gh pr create`. Angie revisa, tú revisas las suyas.
- **Commits pequeños** y descriptivos en español o inglés (consistente): `feat(library): implement search_by_genre`, `fix(ws): avoid deadlock on broadcast`.
- **No hagas `git push --force`** a `main`. En tu rama está OK si solo estás tú.
- **Antes de pushear**: corre `cargo check && cargo clippy` — si hay warnings nuevos, justifícalos.
- **No cambies `domain.rs` después del Día 1** sin hablar con Angie — romperás los tipos del frontend.

## 5. Plan día por día — checklist accionable

### Día 1 — Kickoff + base de biblioteca

**Objetivo del día**: al final, desde `cargo run` el servidor debería poder mostrar canciones agregadas vía una mini prueba (CLI de Angie o directo en código).

- [ ] Sync inicial con Angie (30 min): congelar juntos [`domain.rs`](../server/src/domain.rs) y [`docs/protocolo.md`](protocolo.md). Cualquier duda sobre tipos la resuelven acá.
- [ ] Implementar [`library.rs`](../server/src/library.rs):
  - [ ] `Library::new` y `add_song(song: Song)` — inserta en `songs` y actualiza índice `by_genre`.
  - [ ] `add_song_from_file(path: &Path)` — lee ID3 con crate `id3`, construye `Song`, llama a `add_song`.
  - [ ] `remove_song(id)` — borra de ambos (songs y by_genre). **Debe consultar `playback::is_playing(id)`** y fallar con `anyhow::bail!("cannot delete playing song")` si es true.
  - [ ] `search_by_title(substring)` — scan lineal, `to_lowercase().contains(...)`.
  - [ ] `search_by_genre(genre)` — lookup directo en `by_genre` (O(1)).
  - [ ] `search_by_year_range(from, to)` — scan + filter numérico.
  - [ ] `list()`, `get(id)`.
- [ ] Implementar [`persistence.rs`](../server/src/persistence.rs):
  - [ ] `load_library()` → lee `library.json`, devuelve `LibrarySnapshot::default()` si no existe.
  - [ ] `save_library(snap)` → escribe pretty JSON.
  - [ ] Mismo par para `playlists.json`.
- [ ] Implementar [`playback.rs`](../server/src/playback.rs):
  - [ ] `Mutex<HashSet<SongId>>` + `mark_playing`, `mark_stopped`, `is_playing`, `snapshot`.
- [ ] **Commit + push** al final del día.

### Día 2 — Red (WebSocket + HTTP) + módulo funcional

**Objetivo**: al final, desde `websocat ws://localhost:8080/ws` y `curl --range 0-65535 http://localhost:8080/stream/1` se pueden probar comandos y bajar audio.

- [ ] Implementar [`ws.rs`](../server/src/ws.rs):
  - [ ] Función `ws_handler` + `handle_socket`.
  - [ ] `tokio::select!` entre `socket.recv()` y `broadcast.recv()`.
  - [ ] Al conectar, enviar `library.snapshot` + `playlist.snapshot` inicial.
  - [ ] Delegar cada `op` del `ClientMsg` de `protocol.rs` al módulo correspondiente.
- [ ] Implementar [`http_stream.rs`](../server/src/http_stream.rs):
  - [ ] Endpoint `GET /stream/:id`.
  - [ ] Si `file_path.is_some()`, usar `tower_http::services::ServeFile` (maneja Range automáticamente).
  - [ ] Si `spotify_preview_url.is_some()`, proxy con `reqwest` reenviando el body como stream.
  - [ ] Llamar `playback::mark_playing(id)` al abrir, `mark_stopped` al cerrar.
- [ ] Implementar [`playlists/state.rs`](../server/src/playlists/state.rs) y [`playlists/ops.rs`](../server/src/playlists/ops.rs) **en estilo funcional** (ver §6 abajo — reglas estrictas).
- [ ] Integrar todo en [`main.rs`](../server/src/main.rs):
  - [ ] `AppState` con `library`, `playlists`, `playback`, `broadcast` sender.
  - [ ] Arrancar axum con rutas `/ws` y `/stream/:id`.
  - [ ] Spawn de `cli::run(state.clone())` en una task paralela.
- [ ] Implementar [`spotify.rs`](../server/src/spotify.rs) (puede esperar al Día 3 si te quedas corto de tiempo):
  - [ ] `SpotifyClient::new()` lee `SPOTIFY_CLIENT_ID` y `SPOTIFY_CLIENT_SECRET` de env.
  - [ ] `fetch_track(track_id)` → `Song` con `spotify_preview_url`.
- [ ] **Commit + push** al final del día.

### Día 3 — Soporte + inicio de docs

**Angie está integrando el frontend** — tú entras en modo bugfix y empiezas docs.

- [ ] Estar disponible para bugs que Angie reporte (Slack/WhatsApp/lo que usen).
- [ ] Empezar [`docs/analisis-tecnico.md`](analisis-tecnico.md) — al menos las secciones de arquitectura y decisiones clave.
- [ ] Empezar [`docs/concurrencia.md`](concurrencia.md) — explicar `Arc<RwLock>` en library y el patrón inmutable de playlists.
- [ ] Si te queda tiempo: terminar `spotify.rs`.

### Día 4 — Docs + QA

- [ ] Terminar ambas docs técnicas.
- [ ] Correr la prueba del requerimiento: **"no se puede borrar canción reproduciéndose"**.
  1. Agrega un MP3 vía CLI.
  2. Desde el frontend reproduce la canción.
  3. Desde la CLI del servidor, intenta `remove <id>`.
  4. Debe fallar con mensaje claro. Documenta el resultado en `concurrencia.md`.
- [ ] Prueba de concurrencia multi-pestaña con Angie (abrir 3 navegadores, agregar canciones a la misma playlist en simultáneo, verificar que ninguna se pierde).
- [ ] Revisión cruzada: Angie prueba tu backend desde el frontend; tú lees tu propio código.

## 6. Regla de oro del módulo funcional de playlists ⚠️

Esto es **requerimiento explícito del enunciado**. Si lo rompes, pierdes puntos.

### Reglas estrictas en `server/src/playlists/ops.rs`

✅ **Permitido**:
- `let` (sin `mut`)
- Estructuras inmutables: `im::HashMap`, `im::Vector`
- Transformaciones funcionales: `.iter().map(...).filter(...).fold(...).collect()`
- Closures: `|x| x.duration_secs`
- Retornar `State` nuevo cada vez

❌ **Prohibido**:
- `let mut ...`
- `&mut self`
- `.push(...)`, `.insert(...)`, `.remove(...)` sobre una variable — **debe ser una copia nueva**
- Importar `tokio`, `std::sync`, o cualquier tipo de lock
- `for` loops con efectos secundarios (usa `.for_each` solo para side-effect-free iteration — mejor evitar y usar fold/map/collect)

### Ejemplo correcto de `add_song`

```rust
pub fn add_song(state: &State, pid: PlaylistId, sid: SongId) -> State {
    state
        .playlists
        .get(&pid)
        .map(|pl| {
            let new_songs = pl.songs.clone().push_back(sid); // im::Vector clone es O(1)
            let new_pl = Playlist {
                songs: new_songs,
                ..pl.clone()
            };
            State {
                playlists: state.playlists.update(pid, new_pl),
                next_id: state.next_id,
            }
        })
        .unwrap_or_else(|| state.clone())
}
```

Nota: `.update()` en `im::HashMap` devuelve **un HashMap nuevo**, no muta el original. Lo mismo con `.push_back()` en `im::Vector`.

### Ejemplo correcto de query con closures

```rust
pub fn total_duration(p: &Playlist, library: &[Song]) -> u32 {
    p.songs
        .iter()
        .filter_map(|sid| library.iter().find(|s| s.id == *sid))
        .fold(0u32, |acc, s| acc + s.duration_secs) // ← fold obligatorio acá
}
```

### Dónde SÍ está permitido mutar (fuera del módulo)

El "imperative shell" en [`ws.rs`](../server/src/ws.rs):

```rust
// Esto SÍ es permitido (estamos FUERA del módulo funcional):
let snapshot = shared.read().await.clone();
let next = playlists::ops::add_song(&snapshot, pid, sid); // llamada pura
*shared.write().await = next; // swap del estado completo
```

El módulo `playlists/` no sabe que existe un lock. El lock vive en `ws.rs`. Esa es la magia del patrón "functional core, imperative shell".

## 7. Patrones útiles en axum / tokio

### Estado compartido

```rust
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct AppState {
    pub library: Arc<RwLock<library::Library>>,
    pub playlists: Arc<RwLock<playlists::State>>,
    pub playback: Arc<playback::Playback>,
    pub broadcast: tokio::sync::broadcast::Sender<protocol::ServerEvent>,
}

// En el handler:
async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse { ... }
```

### Broadcast para notificar a todos los clientes

```rust
let (tx, _) = tokio::sync::broadcast::channel::<ServerEvent>(256);
// cada cliente:
let mut rx = tx.subscribe();
// para notificar:
tx.send(ServerEvent::NowPlaying { ... }).ok();
```

### CORS (para que el frontend localhost:5173 pueda hablar con localhost:8080)

```rust
use tower_http::cors::{Any, CorsLayer};

let cors = CorsLayer::new()
    .allow_origin(Any)
    .allow_methods(Any)
    .allow_headers(Any);

let app = Router::new()
    .route("/ws", get(ws_handler))
    .route("/stream/:id", get(http_stream::stream_handler))
    .layer(cors)
    .with_state(state);
```

## 8. Cómo probar tu código sin el frontend

### CLI interactiva

```bash
cd server && cargo run
# En la consola del servidor:
> add /path/to/song.mp3
> list
> quit
```

### WebSocket manual con `websocat`

```bash
brew install websocat
websocat ws://localhost:8080/ws
# Pega mensajes JSON:
{"op":"library.list"}
{"op":"playlist.create","name":"Rock 80s"}
{"op":"search","by":"genre","value":"rock"}
```

### HTTP Range manual con `curl`

```bash
curl -v -H "Range: bytes=0-65535" http://localhost:8080/stream/1 -o chunk.bin
# Debes ver "HTTP/1.1 206 Partial Content" y un archivo de 65536 bytes.
```

### Tests unitarios de tu módulo funcional

En `server/src/playlists/ops.rs`, al final:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_song_returns_new_state_without_mutating_original() {
        let original = State::new();
        let (state2, pid) = create(&original, "Rock".into());
        let state3 = add_song(&state2, pid, 42);

        // La original sigue vacía
        assert_eq!(original.playlists.len(), 0);
        // state2 tiene la playlist pero sin canciones
        assert_eq!(state2.playlists.get(&pid).unwrap().songs.len(), 0);
        // state3 tiene la canción
        assert_eq!(state3.playlists.get(&pid).unwrap().songs.len(), 1);
    }
}
```

Correr: `cargo test`.

## 9. Puntos de sincronización con Angie

**No saltes estos** — son momentos en que uno bloquea al otro:

| Cuándo | Qué |
|---|---|
| Inicio Día 1 | Acordar juntos los tipos de `domain.rs` y el contrato de `protocolo.md`. Congelar. |
| Mitad Día 2 | Publica tu `ws.rs` respondiendo al menos `library.list` y `playlist.create` para que Angie integre el frontend. Puede ser mock (retornar datos fijos) al principio. |
| Inicio Día 3 | El servidor debe estar funcionando end-to-end. A partir de aquí Angie no debería tener que esperarte; si hay bugs, los arreglas en el día. |
| Día 4 tarde | Demos cruzadas — cada uno prueba lo del otro antes de la entrega. |

## 10. Recursos

- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio tutorial**: https://tokio.rs/tokio/tutorial
- **Axum docs**: https://docs.rs/axum/latest/axum/
- **im crate** (colecciones inmutables): https://docs.rs/im/latest/im/
- **rspotify** (cliente Spotify): https://docs.rs/rspotify/latest/rspotify/
- **id3 crate** (tags MP3): https://docs.rs/id3/latest/id3/

## 11. Si te atoras

1. Lee el mensaje de error completo. Rust es verboso pero preciso.
2. Busca el error literal en Google — casi siempre alguien lo tuvo.
3. `cargo clippy` da hints muy buenos.
4. Si después de 20 min no sales, pregúntale a Angie o pega el error en el grupo. No pierdas horas pegado a un mismo punto.

---

**Objetivo final del viernes**: servidor funcionando, documentación escrita, todo pusheado al repo, prueba "no borrar canción sonando" documentada, prueba multi-pestaña con Angie hecha. Si llegas a esto, cumpliste tu mitad. 🎯
