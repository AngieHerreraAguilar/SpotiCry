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

use id3::TagLike;

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

    pub fn add_song(&mut self, mut song: Song) -> SongId {
        let id = self.next_id();
        song.id = id;
        
        // indexar por género
        self.by_genre
        .entry(song.genre.clone())
        .or_insert_with(Vec::new)
        .push(id);
    
    self.songs.insert(id, song);
    
    id
}

pub fn add_song_from_file(&mut self, path: &Path) -> anyhow::Result<SongId> {
    let tag = id3::Tag::read_from_path(path)?;

    let song = Song {
        id: 0,
        title: tag.title().unwrap_or("Unknown").to_string(),
        artist: tag.artist().unwrap_or("Unknown").to_string(),
        album: tag.album().unwrap_or("Unknown").to_string(),
        genre: tag.genre().unwrap_or("Unknown").to_string(),
        year: tag.year().unwrap_or(0) as u16,
        duration_secs: 0,
        file_path: Some(path.to_path_buf()),
        spotify_preview_url: None,
    };

    Ok(self.add_song(song))
}

pub fn remove_song(&mut self, id: SongId) -> anyhow::Result<()> {
    // 1. Validar si está en reproducción
    if crate::playback::is_playing(id) {
        anyhow::bail!("cannot delete playing song");
    }

    // 2. Eliminar de songs
    if let Some(song) = self.songs.remove(&id) {
        let genre = song.genre;

        // 3. Eliminar del índice by_genre
        if let Some(vec) = self.by_genre.get_mut(&genre) {
            // quitar el id del vector
            vec.retain(|&x| x != id);

            // 4. limpiar si quedó vacío
            if vec.is_empty() {
                self.by_genre.remove(&genre);
            }
        }
    }

    Ok(())
}

pub fn search_by_title(&self, substring: &str) -> Vec<Song> {
    let mut result = Vec::new();
    let query = substring.to_lowercase();

    for song in self.songs.values() {
        if song.title.to_lowercase().contains(&query) {
            result.push(song.clone());
        }
    }

    result
}

pub fn search_by_genre(&self, genre: &str) -> Vec<Song> {
    let mut result = Vec::new();

    if let Some(ids) = self.by_genre.get(genre) {
        for id in ids {
            if let Some(song) = self.songs.get(id) {
                result.push(song.clone());
            }
        }
    }

    result
}

pub fn search_by_year_range(&self, from: u16, to: u16) -> Vec<Song> {
    let mut result = Vec::new();

    for song in self.songs.values() {
        if song.year >= from && song.year <= to {
            result.push(song.clone());
        }
    }

    result
}

pub fn list(&self) -> Vec<Song> {
    let mut result = Vec::new();

    for song in self.songs.values() {
        result.push(song.clone());
    }

    result
}

pub fn get(&self, id: SongId) -> Option<&Song> {
    self.songs.get(&id)
}

// TODO(Persona 1): pub fn update(&mut self, id: SongId, new_song: Song) -> anyhow::Result<()>
//     → debe fallar si playback::is_playing(id) es true

    #[allow(dead_code)]
    fn _silence_unused(&self) {
        let _ = &self.songs;
        let _ = &self.by_genre;
        let _: &Path = Path::new("");
    }
}
