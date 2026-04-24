// Tipos serde para el protocolo WebSocket. Owner: Persona 2.
//
// CONGELADO fin de Día 1 junto con docs/protocolo.md.
// Cualquier cambio requiere actualizar el cliente JS y avisar a Persona 1.

use crate::domain::{PlaylistId, Song, SongId};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════
// Cliente → Servidor
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ClientMsg {
    Search {
        by: SearchBy,
        #[serde(default)]
        value: serde_json::Value, // string para title/genre, [u16,u16] para year_range
    },
    #[serde(rename = "library.list")]
    LibraryList,
    #[serde(rename = "playlist.list")]
    PlaylistList,
    #[serde(rename = "playlist.create")]
    PlaylistCreate { name: String },
    #[serde(rename = "playlist.delete")]
    PlaylistDelete { playlist_id: PlaylistId },
    #[serde(rename = "playlist.add")]
    PlaylistAdd { playlist_id: PlaylistId, song_id: SongId },
    #[serde(rename = "playlist.remove")]
    PlaylistRemove { playlist_id: PlaylistId, song_id: SongId },
    #[serde(rename = "playlist.filter")]
    PlaylistFilter {
        playlist_id: PlaylistId,
        by: SearchBy,
        value: serde_json::Value,
    },
    #[serde(rename = "playlist.sort")]
    PlaylistSort { playlist_id: PlaylistId, by: SortBy },
    Play { song_id: SongId },
    Stop { song_id: SongId },
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum SearchBy {
    Title,
    Genre,
    YearRange,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    Title,
    Year,
    Duration,
}

// ═══════════════════════════════════════════════════════════════════════
// Servidor → Cliente
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "ev", rename_all = "snake_case")]
pub enum ServerEvent {
    #[serde(rename = "library.snapshot")]
    LibrarySnapshot { songs: Vec<Song> },
    #[serde(rename = "playlist.snapshot")]
    PlaylistSnapshot {
        playlists: Vec<crate::domain::Playlist>,
    },
    #[serde(rename = "search.result")]
    SearchResult { songs: Vec<Song> },
    NowPlaying { song_id: SongId, stream_url: String },
    #[serde(rename = "playback.stopped")]
    PlaybackStopped { song_id: SongId },
    Error { code: ErrorCode, msg: String },
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    CannotDeletePlaying,
    NotFound,
    BadRequest,
    Internal,
}
