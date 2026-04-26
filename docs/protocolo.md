# Protocolo de mensajes

> Owner: Conjunto (P1 + P2). Escribir en Día 1 — **CONGELADO antes de codear handlers**.

## Transport

- **Comandos** (bidireccional): WebSocket JSON en `ws://<host>:8080/ws`
- **Audio** (servidor → cliente): HTTP GET con `Range: bytes=X-Y` en `http://<host>:8080/stream/:id`

## Cliente → Servidor

| op | Parámetros | Descripción |
|---|---|---|
| `search` | `by: "title" \| "genre" \| "year_range"`, `value` | Busca por uno de los 3 criterios |
| `library.list` | — | Solicita la biblioteca completa |
| `playlist.list` | — | Solicita todas las playlists |
| `playlist.create` | `name: string` | Crea playlist global |
| `playlist.delete` | `playlist_id: number` | Borra playlist |
| `playlist.add` | `playlist_id`, `song_id` | Agrega canción a playlist |
| `playlist.remove` | `playlist_id`, `song_id` | Quita canción de playlist |
| `playlist.filter` | `playlist_id`, `by`, `value` | Filtra canciones dentro de playlist |
| `playlist.sort` | `playlist_id`, `by: "title" \| "year" \| "duration"` | Ordena la playlist |
| `play` | `song_id` | Marca canción como "en reproducción" |
| `stop` | `song_id` | Marca canción como parada |

### `value` según `by` para `search`

- `by: "title"` → `value: string` (substring)
- `by: "genre"` → `value: string` (exact match para lookup en índice)
- `by: "year_range"` → `value: [number, number]` (inclusive)

## Servidor → Cliente

| ev | Payload | Descripción |
|---|---|---|
| `library.snapshot` | `songs: Song[]` | Estado completo de la biblioteca |
| `playlist.snapshot` | `playlists: Playlist[]` | Estado completo de playlists |
| `search.result` | `songs: Song[]` | Resultado de `search` |
| `now_playing` | `song_id`, `stream_url` | Alguien inició reproducción |
| `playback.stopped` | `song_id` | Alguien terminó reproducción |
| `error` | `code`, `msg` | Error del servidor |

### Códigos de error

- `CANNOT_DELETE_PLAYING` — intentó borrar canción en reproducción
- `NOT_FOUND` — song/playlist id no existe
- `BAD_REQUEST` — payload inválido
- `INTERNAL` — otro

## Tipos compartidos

```typescript
// Song
{ id, title, artist, album, genre, year, duration_secs,
  file_path?, spotify_preview_url?, cover_url? }
// Playlist
{ id, name, songs: number[] }
```

`file_path` viaja como string (no `PathBuf`) para que `library.json` se lea igual en Mac y Windows. `cover_url` se llena con `track.album.images[0].url` cuando la canción viene de Spotify, o con el frame `APIC` cuando se lee del MP3 local.

## Convenciones del servidor

Tres reglas implícitas que **deben** cumplirse para que el frontend funcione y para que la demo de concurrencia del enunciado salga bien:

### 1. Snapshot inicial automático

Cuando un cliente abre el WebSocket, el servidor le envía `library.snapshot` + `playlist.snapshot` **sin que el cliente los pida**. Evita una ronda inicial y garantiza que el UI tenga datos al renderizar la primera pantalla.

### 2. Broadcast en mutaciones de playlist

Cualquier `playlist.create` / `delete` / `add` / `remove` / `sort` hace que el servidor emita `playlist.snapshot` a **TODOS los clientes conectados**, no solo al que mutó. Esto valida el requisito de "concurrencia compartida" del enunciado y es lo que hace funcionar la demo multi-pestaña que documenta `concurrencia.md`.

### 3. Semántica de `stop`

El cliente envía `stop` cuando **cambia de canción o cierra la app**, NO cuando solo pausa. Pausar es estado local del navegador. Sin esta convención el set "en reproducción" se llena de falsos positivos y el requisito "no borrar canción reproduciéndose" se vuelve impredecible.

## Ejemplos

```json
→ { "op": "search", "by": "year_range", "value": [1990, 2000] }
← { "ev": "search.result", "songs": [...] }

→ { "op": "playlist.create", "name": "Road Trip" }
← { "ev": "playlist.snapshot", "playlists": [{ "id": 3, "name": "Road Trip", "songs": [] }, ...] }

→ { "op": "play", "song_id": 42 }
← { "ev": "now_playing", "song_id": 42, "stream_url": "/stream/42" }
```

## Audio streaming (HTTP)

```
GET /stream/42
Range: bytes=0-65535

HTTP/1.1 206 Partial Content
Content-Range: bytes 0-65535/4500000
Content-Type: audio/mpeg

<bytes>
```

El navegador hace nuevas Range requests automáticamente al hacer seek en `<audio>`.
