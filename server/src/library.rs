// Gestión de la biblioteca de canciones (estilo imperativo).
// Owner: Persona 1.
//
// Estado: `Arc<RwLock<HashMap<SongId, Song>>>` + índice secundario `HashMap<Genre, Vec<SongId>>`
// para cumplir los 3 criterios de búsqueda técnicamente distintos:
//   1. Título       → substring match sobre strings (scan lineal)
//   2. Género       → lookup O(1) en el índice secundario
//   3. Año (rango)  → filtro numérico con comparación de rangos
//
// Regla clave (enunciado): remove_song consulta playback::is_playing() y falla si está sonando.

use crate::domain::{Song, SongId};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Library {
    songs: HashMap<SongId, Song>,
    by_genre: HashMap<String, Vec<SongId>>,
    next_id: AtomicU64,
}

impl Library {
    pub fn new() -> Self {
        Self {
            songs: HashMap::new(),
            by_genre: HashMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn next_id(&self) -> SongId {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    // TODO(Persona 1): pub fn add_song_from_file(&mut self, path: &Path) -> anyhow::Result<SongId>
    // TODO(Persona 1): pub fn add_song(&mut self, song: Song)  — usado por Spotify y carga inicial
    // TODO(Persona 1): pub fn remove_song(&mut self, id: SongId) -> anyhow::Result<()>
    //     → debe fallar si playback::is_playing(id) es true
    // TODO(Persona 1): pub fn search_by_title(&self, substring: &str) -> Vec<Song>
    // TODO(Persona 1): pub fn search_by_genre(&self, genre: &str) -> Vec<Song>
    // TODO(Persona 1): pub fn search_by_year_range(&self, from: u16, to: u16) -> Vec<Song>
    // TODO(Persona 1): pub fn list(&self) -> Vec<Song>
    // TODO(Persona 1): pub fn get(&self, id: SongId) -> Option<&Song>

    #[allow(dead_code)]
    fn _silence_unused(&self) {
        let _ = &self.songs;
        let _ = &self.by_genre;
        let _: &Path = Path::new("");
    }
}
