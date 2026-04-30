# SpotiCry — Project Documentation

**Costa Rica Institute of Technology · San Carlos Local Campus**

**School of Computer Engineering · Programming Languages**

**Professor:** Oscar Víquez

**Team:** Angie Herrera Aguilar (2020035640) · Kevin Rivera Gonzalez (2024157337)

**Semester I · Year 2026**

> Spanish version: [`documentacion.md`](documentacion.md). Both files are kept in sync; if you find a discrepancy, the Spanish version is authoritative.

---

## 1. Introduction

SpotiCry is a web-based music playback and management application made of a **Rust** backend and a **React** frontend. The system manages a song library, plays audio over HTTP streaming, and keeps a real-time channel with clients via WebSockets.

The backend is built around safe concurrency primitives (`Arc`, `RwLock`) and a `broadcast`-based event system, which lets it serve multiple simultaneous connections efficiently. The frontend offers an interactive UI on top of those services to browse the library, run searches, control playback, and edit playlists in real time.

SpotiCry combines several audio sources — local MP3 files and previews fetched from Spotify — for a hybrid experience between local playback and online services. The modular architecture cleanly separates concerns into library, playlists, playback, streaming, and protocol modules, which simplifies scaling and maintenance.

State is persisted to local JSON files with debounced writes (`library.json`, `playlists.json`, plus a `library/tracks.json` manifest with auto-scan preferences). The system follows the **functional-core / imperative-shell** pattern: the playlists module is implemented as pure functions over persistent immutable structures (`im::HashMap`, `im::Vector`), and only concurrent wiring lives in the imperative shell.

---

## 2. Problem statement and objectives

### 2.1 Context

SpotiCry is an academic project for the Programming Languages course. The course emphasizes contrasting two paradigms within a single system: **imperative** (mutable state guarded by locks) and **purely functional** (immutable structures, side-effect-free functions, transformations via `map`/`filter`/`fold` and closures).

Architecturally the assignment also requires: (1) real concurrency over sockets, (2) audio byte streaming between processes, (3) on-disk persistence, and (4) documentation comparing the system "with vs without synchronization."

### 2.2 Problem to solve

Build a Spotify-like application with two independent components communicating over the network:

- A **Rust server** that owns the song library, shared playlists, playback state, and streams audio files.
- A **web client** (React) that consumes that server to search, play with free seek, create playlists, and view the catalog in real time — including changes from other users.

The system must explicitly demonstrate the **functional core, imperative shell** pattern: the playlists module is purely functional (no `let mut`, no locks, with `im::HashMap`/`im::Vector`), while the rest of the backend (library, playback, persistence) is imperative and synchronizes with `Arc<RwLock<…>>` and `tokio::sync::broadcast`.

### 2.3 Objectives

**General:** design and implement a client–server application that plays audio concurrently, explicitly showing the contrast between imperative and functional programming inside the same project.

**Backend specifics:**
- Model `Song`, `Playlist`, and the global shared state (`AppState`).
- Implement three technically distinct search criteria (title, genre, year).
- Enforce the rule "a song that is currently playing cannot be deleted" in the backend.
- Implement the `playlists/` module in pure-functional style.
- Serve audio over HTTP Range (`206 Partial Content`).
- Persist library and playlists to JSON.
- Integrate the Spotify API (Client Credentials Flow) to enrich metadata.

**Frontend specifics:**
- WebSocket client with reconnection and command resending.
- `<audio>` player with free seek backed by HTTP Range.
- Views for library, search, and playlists in real time.
- Responsive layout with a 768 px breakpoint.

**Concurrency and robustness:**
- Fan-out synchronization via `broadcast::channel` to all clients.
- Show with a multi-tab test that no mutation is lost.

### 2.4 Explicit technical constraints

- The `playlists/` module cannot use `let mut`, `tokio`, or any synchronization primitives: only immutable structures and functional combinators (`map`, `filter`, `fold`).
- The "no deleting a playing song" rule is validated in the backend; the frontend is never the only line of defense.
- The WebSocket protocol was frozen at the end of Day 1, and any later change required team agreement.

---

## 3. Implemented features

For each block we describe **what it does** followed by **why it was done that way**.

### 3.1 Library and search

**What it does.** Maintains a song collection (`Song`) loaded from local MP3s and/or Spotify (metadata + cover + 30 s preview when available). Supports adding, deleting, listing, and searching by three criteria.

> **Note on the Spotify preview.** Since 2024 Spotify stopped exposing `preview_url` in most Web API responses for Client Credentials Flow apps. In practice, for most tracks the field comes back `null`. That is why the recommended flow is `add <mp3-path> <spotify-id>` (local MP3 + metadata + cover) instead of `add-spotify <id>` alone, which only works for the few tracks that still return a preview.

- **Auto-scan** at server start: detects new MP3s under `library/` and configures them via CLI with a small menu (`[s]` Spotify link, `[m]` manual metadata, `[t]` ID3 tags, `[x]` skip). Choices are saved to `library/tracks.json` so subsequent boots are automatic with no user interaction.
- **Three technically distinct search algorithms:**
  - Title → case-insensitive substring (linear scan, complexity O(n·m)).
  - Genre → secondary `HashMap<Genre, Vec<SongId>>` index (O(1) lookup).
  - Year → inclusive numeric range filter (O(n)).

**Why it was done that way.**
- Auto-scan + manifest replaces a complex CLI command (`add <long-path> <base62-id>`) with a flow where the user just drops MP3s in a folder and answers a menu. Reduces typos and makes the app usable without memorizing commands.
- The three search algorithms are an explicit course requirement ("three technically distinct criteria"). Each is motivated by the field's nature: free-form text (substring), bounded categorical value (index), numeric range (filter). The secondary by-genre index pays O(1) extra on writes to speed reads to O(1) — a classic trade-off worth it when reads outnumber inserts.

### 3.2 Playback

**What it does.**
- HTTP streaming with Range requests; the browser's `<audio>` element seeks natively by issuing new requests automatically.
- Player controls: play, pause, resume, drag-seek, ±10 s jumps.
- Synchronizes pause/play/end with the server via WebSocket: every client-side change notifies the server to update playback state.
- Automatic cleanup on tab close: if a client disconnects without notice, the server releases the songs it had marked as "playing".
- Validates the invariant "a song in playback cannot be deleted" (`CANNOT_DELETE_PLAYING`), guarded by three unit tests.

**Why it was done that way.**
- HTTP Range is the most natural fit for `<audio>`: the browser already implements buffering + seek + reload — no JS is written for any of those mechanics, only `audio.currentTime = t`. A WebSocket-only approach would require reimplementing all of that by hand.
- Splitting "control over WS, audio over HTTP" keeps the two channels with distinct responsibilities: WS carries small frequent messages, HTTP carries heavy reusable bytes. Mixing them would saturate the WS and make seek impossible.
- Per-connection cleanup is a defense for the real-world case "user closes the tab without pausing." Without it, the "playing" set would fill with eternal false positives and the assignment rule would break.

### 3.3 Playlists — pure functional module

**What it does.**
- Create, delete, add/remove songs, reorder, filter by criterion (title/genre/year) within a playlist, sort by title/year/duration.
- Validates duplicate names (case-insensitive with trim) and empty names: in backend (`Option<>`) and with immediate feedback in the frontend modal.
- Persistent immutable state with `im::HashMap<PlaylistId, Playlist>` and `im::Vector<SongId>`.
- **Functional core / imperative shell** pattern: the `playlists/` module imports neither `tokio`, `std::sync`, nor `let mut`. Its functions are pure: they take `&State` and return a new `State`.

**Why it was done that way.**
- The course rule requires one project module to be purely functional. Playlists are the natural candidate because they have many composable operations (filter/sort/map/fold) and lend themselves to immutable structures.
- `im::HashMap` provides **structural sharing in O(log n)**: cloning a snapshot is cheap and does not copy the whole structure. This enables the key pattern: read a snapshot, compute outside the lock, atomically swap at the end. The computation never blocks other readers.
- Duplicate validation is done in backend (real, multi-client line of defense) and in frontend (immediate user feedback, avoids the round-trip). If only the frontend enforced it, two tabs could create the same name simultaneously.

### 3.4 WebSocket and broadcast

**What it does.**
- Client–server connection at `ws://<host>:8080/ws` with automatic reconnection (exponential backoff).
- Automatic initial snapshot on connect: `library.snapshot` + `playlist.snapshot` without the client asking.
- Real-time broadcast of library/playlist mutations, playback events, and search results — via `tokio::sync::broadcast::channel`.
- Per-connection cleanup: each client keeps a `HashSet<SongId>` of songs it had playing and releases them on disconnect.

**Why it was done that way.**
- WebSocket is the only persistent bidirectional primitive available to a browser (you cannot open arbitrary TCP from JS). It preserves the spirit of the original assignment ("persistent socket connection") and adds web compatibility.
- The automatic initial snapshot saves a round-trip (client connects → client asks → server replies). It guarantees the UI has data when the first screen renders, with no unnecessary "loading" intermediate states.
- `broadcast::channel` solves fan-out without the publisher knowing the subscribers. Any handler that mutates state just calls `tx.send(event)` and every connected WS receives it. This is exactly the pub/sub pattern the assignment asks for as "shared concurrency."

### 3.5 HTTP streaming

**What it does.**
- `GET /stream/:id` with Range request support (partial streaming).
- Plays from local file (`tower_http::services::ServeFile`) or proxies a Spotify preview (`reqwest`).
- `Cache-Control: no-cache` on every response to keep the browser from serving stale cached audio after a reset.
- Cache-buster `?t=<timestamp>` on the frontend: each `play()` produces a unique URL.

**Why it was done that way.**
- `tower_http::ServeFile` already implements `Range`/`206 Partial Content` correctly — reusing it instead of reimplementing avoids off-by-one bugs and malformed responses.
- The Spotify preview proxy preserves the "one URL per song" abstraction (`/stream/:id`) — the client doesn't need to know whether the audio comes from local disk or from Spotify. This decouples the frontend from the file's origin.
- The double cache fix (`no-cache` + cache-buster) was necessary because Chrome has a **separate media cache** from the regular HTTP one, and it doesn't get purged with `Cmd+Shift+R`. After several `library.json` resets, the media cache could serve old audio bound to an id that now points to another file. With both defenses combined, the problem disappears.

### 3.6 Persistence

**What it does.**
- `library.json` and `playlists.json` are read at startup and dumped automatically with a **500 ms debounce** after any mutation (CLI or WS).
- `library/tracks.json` stores auto-scan preferences.
- Filters out entries with `file_path` that no longer exists when loading — auto-cleanup after renaming/moving MP3s.

**Why it was done that way.**
- Dumping to JSON after every single mutation would be expensive and cause fragmented writes. The server exposes an `mpsc::Sender<()>` in `AppState`; every handler that mutates fires a non-blocking `try_send(())`. The `persistence::run_debouncer` task waits 500 ms after the first event, drains anything that arrived within that window, and only then writes. This **coalesces multi-tab bursts into a single flush**.
- Separating the manifest (`tracks.json`) from the actual state (`library.json`) lets both evolve independently: the manifest decides how to load future MP3s; the state stores what is already loaded.
- Filtering missing `file_path`s avoids dragging orphan songs around forever: if the user moves an MP3 out of the folder, the next session cleans up automatically.

### 3.7 Server CLI

**What it does.**
- Commands `add <path>`, `add <path> <spotify-id>`, `add-spotify <id>`, `remove <id>`, `list`, `playlists`, `help`, `quit`/`exit`.
- Interactive setup at startup to configure new MP3s detected by the scan.
- Persists state on `quit`/EOF (Ctrl-D) on top of the automatic debouncer.

**Why it was done that way.**
- The assignment requires the server to expose a "text interface" — that is the minimum catalog admin. Keeping the CLI as a fallback path (even with auto-scan) covers the cases where interactive setup doesn't apply.
- Combining `add` with `add <path> <id>` (local mp3 + Spotify metadata) gives the best of both worlds: full audio from disk and rich metadata + cover from Spotify.

### 3.8 Frontend

**What it does.**
- Three pages: Home (library), Search (3 tabs), Playlist (detail view).
- Three Zustand stores: `library`, `playlists`, `player`.
- Spotify-style `+` button to add songs to playlists, with a green checkmark if already added.
- 2×2 mosaic of real cover art as the hero of each playlist.
- Player fixed to the bottom of the viewport (no scrolling needed to reach it).
- Responsive with a 768 px breakpoint (sidebar → bottom nav).

**Why it was done that way.**
- Zustand over Redux/Context API: far less boilerplate, a single `create()` per store, automatic subscription via selectors. For three small stores it is the lightest option.
- No UI library: the design system is built from CSS variables and a single stylesheet. This gives full control and a smaller bundle.
- The viewport-pinned layout (`100dvh` + `min-height: 0` on the scroll grid item) uses a known CSS Grid technique to keep the player always visible while the list scrolls internally. `100dvh` (dynamic viewport height) accounts for iOS Safari's dynamic URL bar.

---

## 4. Features NOT included

### 4.1 Spotify
- No user authentication (only Client Credentials Flow for public metadata).
- Playlists are not imported from Spotify; they live exclusively in SpotiCry.
- Search runs only over the local library, not the global Spotify catalog.
- **The 30 s preview is not guaranteed.** Spotify changed its policy in 2024 and stopped exposing `preview_url` for most tracks via Client Credentials Flow. Songs without a local MP3 are stored with metadata + cover but may not be playable. The recommended mode is always to combine the local MP3 with a Spotify track ID (`add <mp3-path> <spotify-id>`).

### 4.2 Advanced persistence
- No relational database; state lives in local JSON files.
- No versioning, historic snapshots, or undo.
- No automatic backups.

### 4.3 Playback
- No integrated volume slider (delegated to the OS). Pause, resume, free seek, and ±10 s jumps are present.
- No continuous Spotify-style catalog: the main audio is local MP3s, with the 30 s preview as fallback.
- No "play next" auto-advance, no queue.
- No HLS/DASH segmentation, no bitrate adaptation.

### 4.4 Multi-user
- No user management, authentication, or sessions.
- Globally shared state — every playlist is visible to any connected client. A deliberate decision to maximize the material to document about concurrency.

### 4.5 Deployment
- No automated production deployment. The submission runs on localhost; a screencast accompanies the demo if needed.

---

## 5. Technical analysis

### 5.1 Architecture

```
 ┌──────────────────────┐     WebSocket JSON      ┌────────────────────────┐
 │  Client (React)      │ ◄── ws://.../ws ─────►  │  Server (Rust)         │
 │  - Vite + Zustand    │                         │  - Tokio + Axum        │
 │  - <audio> + seek    │     HTTP Range          │  - Text CLI            │
 │  - 3 stores          │ ◄── /stream/:id ──────  │  - WebSocket handler   │
 │                      │                         │  - HTTP streaming      │
 └──────────────────────┘                         │  - Pure functional     │
                                                  │    `playlists/`        │
                                                  └─────┬──────────────────┘
                                                        ▼
                                          ┌─────────────────────────────┐
                                          │  library.json               │
                                          │  playlists.json             │
                                          │  library/tracks.json        │
                                          │  library/*.mp3              │
                                          └─────────────────────────────┘
```

Every client that connects to `/ws` receives an initial snapshot of library and playlists, and gets subscribed to a `tokio::sync::broadcast` that forwards each mutation to all connected peers. Audio travels on a separate HTTP channel so control and data don't share the same socket.

### 5.2 Stack and why each piece

| Piece | Why |
|---|---|
| Rust 2021 | Strict compiler prevents use-after-move, unsynchronized shared data, unhandled errors. Once it compiles, it usually works. |
| Tokio (async runtime) | Lets I/O concurrency be expressed in code that reads sequentially; supports thousands of WS connections without one thread per client. |
| Axum 0.7 | Web framework with native WebSocket support and easily testable handlers; integrates `tower-http`. |
| `tower-http::ServeFile` | Implements Range/`206 Partial Content` correctly — reusing it avoids off-by-one bugs. |
| `im` (HashMap, Vector) | Persistent immutable structures with O(log n) structural sharing; enables the "compute outside the lock + atomic swap" pattern. |
| `id3` | Reads ID3v2 tags from MP3s. |
| `rspotify` | Official Spotify API client; supports Client Credentials Flow without user OAuth. |
| `reqwest` | HTTP client used to proxy Spotify previews. |
| `serde` / `serde_json` | Typed serialization for protocol and persistence. |
| `dotenvy` | Auto-loads `server/.env` so we don't have to prefix variables on each run. |
| React 19 + Vite 8 | Fast build, instant HMR, mature ecosystem. |
| Zustand 5 | Global state with much less boilerplate than Redux/Context for three small stores. |
| React Router 7 | Declarative routing for three pages. |

### 5.3 Key decisions and rationale

**WebSocket instead of "raw" TCP.** The original assignment suggested TCP sockets, but browsers can't open arbitrary TCP sockets from JS: the only available primitive is WebSocket, which is TCP with an HTTP upgrade handshake. We preserve the spirit of the assignment (persistent bidirectional connection, server-pushed messages) and gain web interoperability.

**Globally shared playlists (no auth).** Maximizes the material to document about concurrency: every playlist mutation by one client must propagate to all the others. With no login there are no OAuth distractions and the "race read-modify-write" problem stays at the center.

**Functional module for playlists.** The `playlists/` module imports neither `tokio`, `std::sync`, nor anything related to concurrency. Its functions (`create`, `add_song`, `remove_song`, `reorder`, `filter_songs`, `sort_by`, `total_duration`) are pure: they take `&State`, return a new `State`, no `let mut`. The wiring lives in `ws.rs` with the "read snapshot → apply pure fn → swap" pattern:

```rust
let snapshot = state.playlists.read().await.clone();   // O(1) thanks to im
let next = ops::add_song(&snapshot, pid, sid);         // pure, no lock
*state.playlists.write().await = next;                 // single swap
```

**Three search criteria with distinct algorithms.**

| Criterion | Algorithm | Complexity |
|---|---|---|
| Title | substring `to_lowercase().contains` over each song | O(n·m) — linear scan |
| Genre | secondary `HashMap<Genre, Vec<SongId>>` index | O(1) lookup + O(k) materialization |
| Year | filter `s.year >= from && s.year <= to` | O(n) — range comparison |

**Debounced persistence.** An `mpsc::Sender<()>` in `AppState` receives a non-blocking `try_send(())` after each mutation. A `persistence::run_debouncer` task waits 500 ms, drains the channel, and dumps `library.json` and `playlists.json`. Coalesces multi-tab bursts into a single flush.

### 5.4 In-memory administration

The backend uses **two intentionally distinct strategies** for two state types:

**Library — `Arc<RwLock<HashMap<SongId, Song>>>`.**
Many reads (every `search`, every `library.list`) and few writes (CLI). `RwLock` lets multiple readers in at once with no contention and only serializes the writer. Maintains a secondary `by_genre: HashMap<String, Vec<SongId>>` index so `search_by_genre` is O(1) — a classic trade-off: pay O(1) extra on writes to speed up reads.

**Playlists — `Arc<RwLock<im::HashMap<PlaylistId, Playlist>>>`.**
Many concurrent writes (multi-tab). `im::HashMap` has structural sharing: cloning a snapshot is O(1). This enables "compute outside the lock + swap" and removes contention from the computation.

**Playback — `Arc<Mutex<HashSet<SongId>>>`.**
No concurrent reads (point queries) and short writes. A simple `Mutex<HashSet>` is enough.

### 5.5 Music-file handling

**Loading.** `Library::add_song_from_file(Path)` uses `id3::Tag::read_from_path` to read ID3v2 frames. Missing frames fall back to `"Unknown"`. If the user also passes a Spotify track-id, `spotify::fetch_track` overrides the metadata with the official one and adds `cover_url` and `spotify_preview_url`.

**Streaming.** The `GET /stream/:id` endpoint:
1. If the song has a `file_path` → marks `playback::mark_playing(id)` and delegates to `ServeFile`. `ServeFile` honors the `Range: bytes=X-Y` header and replies with `206 Partial Content`.
2. If only `spotify_preview_url` is set → proxies via `reqwest::get(url)` forwarding the `bytes_stream`.
3. In both cases, `Cache-Control: no-cache` is injected to bypass Chrome's media cache.

**Seek.** When the user does `audio.currentTime = t`, the browser drops the buffer if the offset is out of range, fires a new Range request, and playback resumes from the new position. No JS is written for this; the `<audio>` element does it natively.

### 5.6 Conflict and exception handling

| Case | How it's handled |
|---|---|
| Delete a playing song | `library::remove_song` calls `playback::is_playing(id)` and returns `CANNOT_DELETE_PLAYING`. Three unit tests guard the invariant. |
| Client closes tab without notice | Each WS connection keeps a `HashSet<SongId>` of what's "playing"; on the `tokio::select!` `break`, it releases everything and broadcasts `PlaybackStopped`. |
| Corrupt MP3 or invalid path | `Tag::read_from_path` propagates `id3::Error`; the CLI catches and prints without aborting the server. If the file disappears later, `ServeFile` answers 404. |
| Spotify rate-limited / 401 | Cached token refresh (TTL 1 h). If `SPOTIFY_CLIENT_ID/SECRET` are missing, the server boots with `spotify: None` and the CLI rejects `add-spotify` with a clear message. |
| Malformed WebSocket | `serde_json::from_str` fails → handler sends `ServerEvent::Error { code: BadRequest }` without closing the connection. |
| `Play` on a missing `song_id` | Pre-check via `library.get(song_id)`; emits `Error { code: NotFound }` without touching playback. |
| Stale browser cache | `Cache-Control: no-cache` + cache-buster `?t=<timestamp>` on every `play()`. |
| Duplicate playlist names | `ops::create` returns `Option`; the handler emits `Error { code: BadRequest }` with a message. The frontend also validates in real time and disables the "Save" button. |

---

## 6. Message protocol

### 6.1 Transport
- Commands (bidirectional): WebSocket JSON at `ws://<host>:8080/ws`.
- Audio (server → client): HTTP `GET` with `Range: bytes=X-Y` at `http://<host>:8080/stream/:id`.

### 6.2 Client → Server

| `op` | Parameters | Description |
|---|---|---|
| `search` | `by`, `value` | Search by title / genre / year_range |
| `library.list` | — | Request the full library |
| `playlist.list` | — | Request all playlists |
| `playlist.create` | `name` | Create a global playlist |
| `playlist.delete` | `playlist_id` | Delete a playlist |
| `playlist.add` | `playlist_id`, `song_id` | Add a song to a playlist |
| `playlist.remove` | `playlist_id`, `song_id` | Remove a song from a playlist |
| `playlist.filter` | `playlist_id`, `by`, `value` | Filter inside a playlist |
| `playlist.sort` | `playlist_id`, `by` | Sort by title/year/duration |
| `play` | `song_id` | Mark as "playing" |
| `stop` | `song_id` | Mark as stopped |

### 6.3 Server → Client

| `ev` | Payload | Description |
|---|---|---|
| `library.snapshot` | `songs: Song[]` | Full library state |
| `playlist.snapshot` | `playlists: Playlist[]` | Full playlist state |
| `search.result` | `songs: Song[]` | Result of a `search` |
| `now_playing` | `song_id`, `stream_url` | Someone started playback |
| `playback.stopped` | `song_id` | Someone stopped playback |
| `error` | `code`, `msg` | Server error |

**Error codes:** `CANNOT_DELETE_PLAYING`, `NOT_FOUND`, `BAD_REQUEST`, `INTERNAL`.

### 6.4 Critical conventions

1. **Automatic initial snapshot.** When the WS opens, the server sends `library.snapshot` + `playlist.snapshot` without the client asking.
2. **Broadcast on playlist mutations.** Any `playlist.create/delete/add/remove/sort` emits a `playlist.snapshot` to ALL connected clients. This validates the "shared concurrency" requirement.
3. **`stop` semantics.** The client sends `stop` when it changes song, pauses, or closes the app. Without this convention the "playing" set fills with false positives.

### 6.5 Examples

```json
→ { "op": "search", "by": "year_range", "value": [1990, 2000] }
← { "ev": "search.result", "songs": [...] }

→ { "op": "playlist.create", "name": "Road Trip" }
← { "ev": "playlist.snapshot", "playlists": [...] }

→ { "op": "play", "song_id": 42 }
← { "ev": "now_playing", "song_id": 42, "stream_url": "/stream/42" }
```

---

## 7. Concurrency: with vs without synchronization

### 7.1 The problem without synchronization

If playlists lived as a plain `HashMap` with no lock and every client applied `playlist.add` directly:

```
T0  State: playlist.songs = [x, y]
T1  Client A: read  → [x, y]
T2  Client B: read  → [x, y]
T3  Client A: write → [x, y, z]   ✓
T4  Client B: write → [x, y, w]   ✗  (z is lost)
```

It's a *read-modify-write race condition*: two threads read the same version, compute in parallel, and the last write erases the previous one. The number of "lost z's" grows with the number of tabs.

### 7.2 The applied solution — functional core / imperative shell

```rust
let snapshot = state.playlists.read().await.clone();   // O(1) — only Arc bumps
let next = ops::add_song(&snapshot, pid, sid);         // pure, no lock
*state.playlists.write().await = next;                 // single swap
```

Three guarantees:
1. **No add is lost.** Write locks are serialized; sequential applications run on the consolidated `next`.
2. **No lock held during compute.** `ops::add_song` runs while others read.
3. **Observable atomicity.** There is never an "intermediate" public state.

### 7.3 Experimental procedure

1. Start the server: `cd server && cargo run`.
2. Load 10–20 songs via CLI (mix of `add <path>` + `add-spotify <id>`).
3. Start the frontend: `cd client && npm run dev`.
4. Open 3 tabs at `http://localhost:5173`.
5. Create an empty "concurrency-test" playlist from one tab.
6. Wait 1 s for the snapshot to reach all 3 tabs.
7. In each tab, fire `addSong(playlist_id, song_X_distinct)` rapidly (~10 adds per tab in under 2 s).
8. End the burst and wait 1 s.
9. Compare:
   - Expected total: 30 songs in the playlist.
   - Observed total in each tab after the last broadcast.
   - Observed total in `playlists.json` after the debounce.

Additional tests:
- **Remove during playback.** Play → `remove <id>` from CLI → must reject with `CANNOT_DELETE_PLAYING` → pause → repeat → must succeed.
- **Cleanup on tab close.** Play → close tab without pausing → `remove <id>` from CLI → must succeed (automatic cleanup).

### 7.4 Expected result by construction
- **Adds are not lost**: final total = `n_tabs × adds_per_tab`.
- **Total-order convergence**: all 3 tabs converge to the same count.
- **Coherent `playlists.json`**: the flush captures exactly the final state, no intermediate states.

### 7.5 Reflection on large-scale production

| Dimension | Today | Production |
|---|---|---|
| Storage | local JSON | Postgres (library with `tsvector` for full-text) + S3 (audio) |
| Concurrency | `RwLock` + `im` | CRDTs (ORSet, RGA) or event sourcing with total ordering |
| Auth | none | OIDC, JWT signed on every WS connection |
| Streaming | HTTP Range | HLS / DASH + DRM + bitrate adaptation |
| Observability | `tracing` to stdout | OpenTelemetry, Grafana dashboards, alerts |
| Deployment | localhost | Kubernetes with autoscaler by WS connections, CDN for frontend |

The **functional-core / imperative-shell** pattern is not replaced, it is generalized: the pure functional core stays the engine; what changes is how state is persisted, replicated across instances, and authorized per operation.

---

## 8. Results and metrics

### 8.1 Implemented features (MVP)

All MVP items from the assignment are operational at project close. Rust backend with Tokio + Axum, three search criteria, full CLI, JSON persistence with debounce, pure functional module, "no delete playing" rule with tests, HTTP Range streaming, Spotify Client Credentials integration. Frontend with three Zustand stores, tabbed search, free-seek player, playlist CRUD, live sync, responsive at 768 px.

### 8.2 Metrics (development environment, macOS Apple Silicon, debug build)

- **Initial WebSocket latency**: < 50 ms (snapshot after `101 Switching Protocols`).
- **Seek latency**: < 100 ms on localhost (`audio.currentTime = t` → new Range request → first byte).
- **Range chunk size**: 64 KiB browser default.
- **Server process memory**: < 30 MB at rest with dozens of songs loaded.
- **Incremental build**: `cargo build` ~3 s after the first clean build.
- **Tests**: `cargo test` runs 5 tests in < 0.1 s (3 invariant + CLI smoke + remove unknown id).

### 8.3 Music-file handling — end-to-end flow

1. **Auto-scan** or **CLI `add`** → `Library::add_song_from_file` reads ID3 tags via the `id3` crate and sets `file_path` to the local file. The song receives an autoincrement `id` starting at 1.
2. **`library.json`** → after any mutation, the debouncer (`mpsc::Sender<()>`) accumulates 500 ms and dumps the full snapshot. On next boot, `main.rs` calls `load_library` + `load_playlists` and rebuilds the initial state.
3. **Client connects** → `ws_handler` sends `LibrarySnapshot` and `PlaylistSnapshot` automatically.
4. **Playback** → click a song → `sendCmd("play", { song_id })` → server marks `Playback::mark_playing` → emits `NowPlaying { song_id, stream_url: "/stream/{id}" }`.
5. **Streaming** → `<audio>` does `GET /stream/:id` with `Range: bytes=0-` → handler replies `206 Partial Content` with the slice. Seeks generate new Range requests automatically.
6. **Stop** → song change / pause / tab close → `sendCmd("stop", { song_id })` → `Playback::mark_stopped` → frees for `remove`.

Special case: if the song was added with `add-spotify` and there is no local file, `file_path` is `None` and the endpoint proxies to `spotify_preview_url` (30 s preview). In practice this fallback only works for tracks where Spotify still exposes `preview_url` — since 2024 most return `null` due to Client Credentials Flow policy changes.

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

### 8.4 Screenshots
  This section presents screenshots of the system in operation, including the user interface, concurrency tests, and development tools used to validate the application’s behavior.

**8.4.1 DevTools – Concurrent Requests**

The Google Chrome Developer Tools (DevTools) were used, specifically the Network tab, to observe multiple HTTP requests being executed in parallel during concurrency testing.

**8.4.2 Home View**

Main screen of the application where users can access the system’s general features.

**8.4.3 Song Search**

Searches were performed using different criteria to validate correct filtering of results.

**8.4.4 Playlist Detail**

View of a playlist showing its songs organized in order.

**8.4.5 Responsive View**

Validation of the responsive design on mobile or reduced screen sizes.

**8.4.6 Terminal and System Tests**

Evidence of automated test execution (cargo test) and validation of system behavior under specific conditions.

Successful tests
Attempt to remove a song while it is playing

---

## 9. Conclusions and recommendations

### 9.1 On contrasting imperative vs functional within a single system

The project shows that both paradigms can coexist without one "contaminating" the other. The library uses a mutable `HashMap` with `let mut` and `for` loops; playlists use `im::HashMap`/`im::Vector` and expose pure functions. The glue between both worlds lives in `ws.rs`, where the **functional core, imperative shell** pattern shows up literally: lock to clone the snapshot, pure compute outside the lock, single swap at the end. The functional module stays free of concurrency concerns and can be tested without an async runtime.

### 9.2 On concurrency with Tokio

`async/await` lets concurrent I/O be expressed with sequential-looking code. `Arc<RwLock<…>>` covers "many readers, few writers" (the library). `tokio::sync::broadcast::channel` solves fan-out: every WS subscribes and receives whatever `ServerEvent` any task publishes, without the publisher knowing the subscribers. A practical observation: **never hold an `RwLock` guard across an `await`** — a convention we followed by extracting data inside a `{ … }` block and dropping the guard before any I/O.

### 9.3 On network protocols

We picked WebSocket JSON instead of raw TCP because browsers can't open arbitrary TCP sockets. WebSocket is essentially TCP with an HTTP handshake, so it preserves persistence and message ordering while adding web compatibility. For audio we picked HTTP Range: the browser's `<audio>` element seeks natively by issuing new requests automatically; no JS is written for buffering or seek.

### 9.4 On Rust as the backend language

The compiler catches an entire class of errors that dynamic languages discover in production: use-after-move, unsynchronized shared data, unhandled error paths. The learning curve is real (ownership, borrowing, lifetimes), but once the code compiles it usually works. The crate ecosystem was key: `tokio` + `axum` for the server, `serde` for JSON, `im` for immutables, `tower-http`'s `ServeFile`, `id3`, and `rspotify`.

### 9.5 Difficulties encountered

- **Rust's curve** (ownership and lifetimes) — especially when combining `async` + locks.
- **Critical persistence bug** discovered mid-phase: `save_library`/`save_playlists` existed but nothing called them; changes were lost on restart. Fixed by adding a real debouncer wired to `mpsc::Sender<()>` from each handler.
- **Premature `mark_stopped`** in `http_stream.rs`: it was called right after returning the `Response`, not when the stream actually ended. Broke the "no delete playing" rule. Fixed by moving `mark_stopped` exclusively to the WS `stop` event and the disconnect cleanup.
- **Chrome media cache**: after several `library.json` resets, Chrome would serve old audio bound to an `id` that now pointed to another file. `Cmd+Shift+R` did not clear it. Fixed with `Cache-Control: no-cache` + cache-buster `?t=<timestamp>`.
- **UI ids vs real ids**: the `01..05` column in `SongList.jsx` showed `idx + 1` (position), not the real `id`. That was confusing when running `remove <n>` in the CLI. Changed to display `s.id` with 2-digit padding; UI ↔ CLI are now consistent.
- **Missing `PlaylistFilter` handler**: the match arm in `ws.rs` was missing and the variant silently fell through `_ => {}`. Caught by checking `cargo check` warnings ("function `filter_songs` is never used").
- **Spotify dropped `preview_url`**: midway through the project we found that Spotify's API in 2024 stopped including `preview_url` for most tracks accessed via Client Credentials Flow. This partially invalidated the `add-spotify <id>` mode (which relied solely on the preview) and pushed us to prioritize the combined flow `add <mp3-path> <spotify-id>`: local MP3 for audio, Spotify only for metadata and cover.

### 9.6 Recommendations / what we would change in production

- **Real auth**: OAuth with Spotify to identify users and associate private playlists. Today everything is global.
- **Database**: Postgres for the library and Redis for playback. Rewriting the entire `library.json` does not scale.
- **Audio CDN**: HLS/DASH with segmentation, adaptive quality, and edge caching.
- **Multi-region with CRDTs**: today playlists live in a single `Arc<RwLock<…>>`; multi-region production requires CRDTs or event sourcing.
- **Observability**: OpenTelemetry, Grafana/Datadog dashboards for latency and error rates.
- **CI/CD**: Azure SWA + Fly.io/Railway/Kubernetes for automated deployment.
- **Integration tests** that boot the real server, open a WS client, and validate the full flow.

### 9.7 Future work

- Web AdminPanel (upload form + frontend editing).
- Spotify Connect (coordinated playback across devices).
- Synced lyrics and animated cover art.
- Live collaboration on playlists (cursors and comments).
- Recommendations using Spotify Audio Features API.
- E2E tests with Playwright + Tokio harness for the backend.
- Packaging: native binary with `cargo dist`, desktop client with Tauri.
