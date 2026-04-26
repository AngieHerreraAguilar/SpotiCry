// Tipos compartidos del dominio.
// Owner: Persona 1. CONGELADO fin de Día 1 — cualquier cambio requiere acuerdo con Persona 2.

use serde::{Deserialize, Serialize};

pub type SongId = u64;
pub type PlaylistId = u64;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Song {
    pub id: SongId,
    /// Metadata — DISTINTO del nombre del archivo (requerimiento del enunciado).
    pub title: String,
    pub artist: String,
    pub album: String,
    pub genre: String,
    pub year: u16,
    pub duration_secs: u32,
    /// Si es `Some`, se sirve el MP3 local. Si es `None`, se usa `spotify_preview_url` (30s).
    /// String (no PathBuf) para que `library.json` serialice igual en Mac y Windows.
    /// Se convierte a `Path` con `Path::new(&s)` cuando se necesite abrir el archivo.
    pub file_path: Option<String>,
    pub spotify_preview_url: Option<String>,
    /// Portada del álbum (de Spotify `track.album.images[0].url` o tag ID3 `APIC`).
    /// La consume el frontend para el mosaic 2×2 del hero de playlists.
    pub cover_url: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Playlist {
    pub id: PlaylistId,
    pub name: String,
    pub songs: im::Vector<SongId>,
}
