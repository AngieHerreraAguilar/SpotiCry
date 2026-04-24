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
{ id, title, artist, album, genre, year, duration_secs, file_path?, spotify_preview_url? }
// Playlist
{ id, name, songs: number[] }
```

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
