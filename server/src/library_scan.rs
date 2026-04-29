// Escaneo automático de la carpeta `library/` al arrancar el server.
//
// Flujo:
// 1. Lee `library/tracks.json` (mapa filename → cómo configurar la canción).
// 2. Lista los .mp3 en `library/` y filtra los que ya están cargados.
// 3. Para cada MP3 nuevo, si tracks.json dice cómo cargarlo (Spotify / manual /
//    ID3 / skip), aplica directo. Si no, queda como "pendiente" para preguntar
//    al usuario en la CLI.
//
// La CLI (cli.rs) procesa los pendientes con un menú simple y guarda la elección
// de vuelta en tracks.json para que el próximo arranque sea automático.

use crate::domain::Song;
use crate::spotify::SpotifyClient;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// Carpeta de MP3s en la raíz del repo. El server corre desde `server/`
// (cwd al hacer `cd server && cargo run`), así que apuntamos un nivel arriba.
// Esto deja `SpotiCry/library/` visible para el profesor sin que tenga que
// entrar a la carpeta de Rust.
pub const LIBRARY_DIR: &str = "../library";
pub const TRACKS_FILE: &str = "../library/tracks.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrackEntry {
    /// Buscar metadata + portada vía Spotify API.
    Spotify { spotify_id: String },
    /// Metadata escrita a mano.
    Manual {
        title: String,
        artist: String,
        album: String,
        genre: String,
        year: u16,
        #[serde(default)]
        duration_secs: u32,
    },
    /// Usar los tags ID3 embebidos en el archivo.
    Id3,
    /// Saltar — no cargar y preguntar de nuevo en el próximo arranque.
    Skip,
}

pub type TracksMap = HashMap<String, TrackEntry>;

#[derive(Debug, Clone)]
pub struct PendingTrack {
    pub filename: String,
    pub abs_path: PathBuf,
}

pub struct ScanResult {
    /// Canciones nuevas listas para cargar (ya tienen Spotify/manual/ID3 resuelto).
    pub configured: Vec<Song>,
    /// MP3 nuevos sin configurar — se le pregunta al usuario por CLI.
    pub pending: Vec<PendingTrack>,
}

pub fn load_tracks_map(path: &Path) -> anyhow::Result<TracksMap> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let txt = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&txt)?)
}

pub fn save_tracks_map(path: &Path, map: &TracksMap) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let txt = serde_json::to_string_pretty(map)?;
    std::fs::write(path, txt)?;
    Ok(())
}

/// Escanea `library_dir` y separa MP3s ya configurados (listos para cargar)
/// vs pendientes (necesitan input del usuario).
///
/// `already_loaded` debe contener los `file_path` que ya están en library.json
/// — así no re-cargamos canciones existentes.
pub async fn scan_dir(
    library_dir: &Path,
    tracks_map: &TracksMap,
    spotify: Option<&Arc<SpotifyClient>>,
    already_loaded: &HashSet<String>,
) -> anyhow::Result<ScanResult> {
    let mut configured = Vec::new();
    let mut pending = Vec::new();

    // Asegurar que library/ exista.
    if !library_dir.exists() {
        std::fs::create_dir_all(library_dir)?;
        return Ok(ScanResult { configured, pending });
    }

    // Recolectar y ordenar alfabéticamente para que el orden de las preguntas
    // en CLI sea predecible (read_dir devuelve orden no-determinista).
    let mut paths: Vec<PathBuf> = std::fs::read_dir(library_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("mp3"))
        .collect();
    paths.sort();

    for path in paths {

        let filename = match path.file_name().and_then(|s| s.to_str()) {
            Some(f) => f.to_string(),
            None => continue,
        };

        let abs_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        let abs_path_str = abs_path.to_string_lossy().to_string();

        // Saltar si ya está cargado (por ruta absoluta).
        if already_loaded.contains(&abs_path_str) {
            continue;
        }

        match tracks_map.get(&filename) {
            Some(TrackEntry::Spotify { spotify_id }) => {
                let Some(client) = spotify else {
                    tracing::warn!(
                        "scan: '{filename}' pide Spotify pero el cliente no está configurado — queda pendiente"
                    );
                    pending.push(PendingTrack { filename, abs_path });
                    continue;
                };
                match client.fetch_track(spotify_id).await {
                    Ok(mut song) => {
                        song.file_path = Some(abs_path_str);
                        configured.push(song);
                    }
                    Err(e) => {
                        tracing::warn!("scan: Spotify falló para '{filename}': {e}");
                        pending.push(PendingTrack { filename, abs_path });
                    }
                }
            }

            Some(TrackEntry::Manual {
                title,
                artist,
                album,
                genre,
                year,
                duration_secs,
            }) => {
                configured.push(Song {
                    id: 0,
                    title: title.clone(),
                    artist: artist.clone(),
                    album: album.clone(),
                    genre: genre.clone(),
                    year: *year,
                    duration_secs: *duration_secs,
                    file_path: Some(abs_path_str),
                    spotify_preview_url: None,
                    cover_url: None,
                });
            }

            Some(TrackEntry::Id3) => match read_id3_song(&abs_path) {
                Ok(song) => configured.push(song),
                Err(e) => {
                    tracing::warn!("scan: ID3 falló para '{filename}': {e}");
                    pending.push(PendingTrack { filename, abs_path });
                }
            },

            Some(TrackEntry::Skip) | None => {
                pending.push(PendingTrack { filename, abs_path });
            }
        }
    }

    Ok(ScanResult { configured, pending })
}

/// Construye un `Song` leyendo los tags ID3 del archivo. Reusable por
/// `scan_dir` y por la CLI cuando el usuario elige `[t]`.
pub fn read_id3_song(path: &Path) -> anyhow::Result<Song> {
    use id3::{Tag, TagLike};
    let tag = Tag::read_from_path(path)?;
    Ok(Song {
        id: 0,
        title: tag.title().unwrap_or("Unknown").to_string(),
        artist: tag.artist().unwrap_or("Unknown").to_string(),
        album: tag.album().unwrap_or("Unknown").to_string(),
        genre: tag.genre().unwrap_or("Unknown").to_string(),
        year: tag.year().unwrap_or(0) as u16,
        duration_secs: 0,
        file_path: Some(path.to_string_lossy().to_string()),
        spotify_preview_url: None,
        cover_url: None,
    })
}

/// Extrae un track ID base62 de Spotify desde una URL completa
/// (`https://open.spotify.com/track/<id>?si=...`) o desde el ID pelado.
pub fn extract_spotify_id(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(idx) = trimmed.find("/track/") {
        let rest = &trimmed[idx + "/track/".len()..];
        let id = rest.split(['?', '/', '#']).next().unwrap_or("");
        if !id.is_empty() {
            return Some(id.to_string());
        }
    }
    Some(trimmed.to_string())
}
