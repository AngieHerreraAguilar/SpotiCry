// Persistencia a JSON en disco. Owner: Persona 1.
//
// library.json    — lista de canciones (metadata + rutas de MP3)
// playlists.json  — playlists globales
//
// Lectura al arrancar, escritura debounced (~500ms) tras cada mutación exitosa.

use crate::domain::{Playlist, Song};
use serde::{Deserialize, Serialize};
use std::path::Path;

const LIBRARY_FILE: &str = "library.json";
const PLAYLISTS_FILE: &str = "playlists.json";

#[derive(Serialize, Deserialize, Default)]
pub struct LibrarySnapshot {
    pub songs: Vec<Song>,
    pub next_id: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct PlaylistsSnapshot {
    pub playlists: Vec<Playlist>,
    pub next_id: u64,
}

// TODO(Persona 1): pub async fn load_library() -> anyhow::Result<LibrarySnapshot>
// TODO(Persona 1): pub async fn load_playlists() -> anyhow::Result<PlaylistsSnapshot>
// TODO(Persona 1): pub async fn save_library(snap: &LibrarySnapshot) -> anyhow::Result<()>
// TODO(Persona 1): pub async fn save_playlists(snap: &PlaylistsSnapshot) -> anyhow::Result<()>
// TODO(Persona 1): spawn_debouncer(rx) — task que flushea cada 500ms si hay cambios

#[allow(dead_code)]
fn _silence_unused() {
    let _: &Path = Path::new(LIBRARY_FILE);
    let _: &Path = Path::new(PLAYLISTS_FILE);
}
