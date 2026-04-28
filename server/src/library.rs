use crate::domain::{Song, SongId};
use crate::playback::Playback;

use std::collections::HashMap;
#[derive(Clone)]
pub struct Library {
    songs: HashMap<SongId, Song>,
    by_genre: HashMap<String, Vec<SongId>>,
    next_id: u64,
}

impl Library {
    pub fn new(next_id: u64) -> Self {
        Self {
            songs: HashMap::new(),
            by_genre: HashMap::new(),
            next_id,
        }
    }

    fn rebuild_index(songs: &HashMap<SongId, Song>) -> HashMap<String, Vec<SongId>> {
        let mut map = HashMap::new();

        for (id, song) in songs {
            map.entry(song.genre.clone())
                .or_insert_with(Vec::new)
                .push(*id);
        }

        map
    }

    pub fn from_snapshot(songs: Vec<Song>, next_id: u64) -> Self {
        let mut map = HashMap::new();

        for song in songs {
            map.insert(song.id, song);
        }

        let by_genre = Self::rebuild_index(&map);

        Self {
            songs: map,
            by_genre,
            next_id,
        }
    }

    pub fn add_song(&mut self, mut song: Song) -> SongId {
        let id = self.next_id;
        self.next_id += 1;

        song.id = id;

        self.by_genre
            .entry(song.genre.clone())
            .or_insert_with(Vec::new)
            .push(id);

        self.songs.insert(id, song);

        id
    }
    pub fn add_song_from_file(&mut self, path: &std::path::Path) -> anyhow::Result<SongId> {
        
    use id3::Tag;
    use id3::TagLike;

    let tag = Tag::read_from_path(path)?;

    let song = Song {
        id: 0,
        title: tag.title().unwrap_or("Unknown").to_string(),
        artist: tag.artist().unwrap_or("Unknown").to_string(),
        album: tag.album().unwrap_or("Unknown").to_string(),
        genre: tag.genre().unwrap_or("Unknown").to_string(),
        year: tag.year().unwrap_or(0) as u16,
        duration_secs: 0,
        file_path: Some(path.to_string_lossy().to_string()),
        spotify_preview_url: None,
        cover_url: None, // ⚠️ solo si existe en Song
    };

    Ok(self.add_song(song))
}


pub fn remove_song(&mut self, id: SongId, playback: &Playback) -> anyhow::Result<()> {
    // 🔴 REGLA CRÍTICA DEL ENUNCIADO
    if playback.is_playing(&id) {
        anyhow::bail!("CANNOT_DELETE_PLAYING");
    }

    let song = match self.songs.remove(&id) {
        Some(s) => s,
        None => return Ok(()),
    };

    if let Some(ids) = self.by_genre.get_mut(&song.genre) {
        ids.retain(|x| *x != id);

        if ids.is_empty() {
            self.by_genre.remove(&song.genre);
        }
    }

    Ok(())
}

    pub fn search_by_title(&self, q: &str) -> Vec<Song> {
        let q = q.to_lowercase();

        self.songs
            .values()
            .filter(|s| s.title.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    pub fn search_by_genre(&self, genre: &str) -> Vec<Song> {
        self.by_genre
            .get(genre)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.songs.get(id))
            .cloned()
            .collect()
    }

    pub fn search_by_year_range(&self, from: u16, to: u16) -> Vec<Song> {
        self.songs
            .values()
            .filter(|s| s.year >= from && s.year <= to)
            .cloned()
            .collect()
    }

    pub fn list(&self) -> Vec<Song> {
        self.songs.values().cloned().collect()
    }

    pub fn get(&self, id: SongId) -> Option<&Song> {
        self.songs.get(&id)
    }

    pub fn to_snapshot(&self) -> (Vec<Song>, u64) {
        (self.songs.values().cloned().collect(), self.next_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_song() -> Song {
        Song {
            id: 0,
            title: "Test".into(),
            artist: "Tester".into(),
            album: "Album".into(),
            genre: "Rock".into(),
            year: 2020,
            duration_secs: 180,
            file_path: None,
            spotify_preview_url: None,
            cover_url: None,
        }
    }

    /// Requisito explícito del enunciado: no se puede eliminar una canción
    /// que está sonando. La library debe consultar `Playback::is_playing`
    /// y devolver el error `CANNOT_DELETE_PLAYING`.
    #[test]
    fn cannot_delete_playing_song() {
        let mut lib = Library::new(0);
        let id = lib.add_song(sample_song());

        let playback = Playback::new();
        playback.mark_playing(&id);

        let result = lib.remove_song(id, &playback);
        assert!(result.is_err(), "remove_song debe fallar mientras la canción está sonando");
        assert_eq!(
            result.unwrap_err().to_string(),
            "CANNOT_DELETE_PLAYING",
            "el error debe ser exactamente el código del enunciado"
        );

        assert!(lib.get(id).is_some(), "la canción debe seguir en la biblioteca");
    }

    #[test]
    fn can_delete_song_when_not_playing() {
        let mut lib = Library::new(0);
        let id = lib.add_song(sample_song());

        let playback = Playback::new();
        // sin marcarla como playing

        assert!(lib.remove_song(id, &playback).is_ok());
        assert!(lib.get(id).is_none(), "la canción debió ser eliminada");
    }

    #[test]
    fn can_delete_after_stop() {
        let mut lib = Library::new(0);
        let id = lib.add_song(sample_song());

        let playback = Playback::new();
        playback.mark_playing(&id);
        playback.mark_stopped(&id);

        assert!(lib.remove_song(id, &playback).is_ok());
    }
}