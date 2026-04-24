// Tracking de canciones "en reproducción".
// Owner: Persona 1.
//
// Resuelve la distinción "canción consultada vs en reproducción" del enunciado.
// Consultado por library::remove_song() para rechazar el borrado de canciones sonando.

use crate::domain::SongId;
use std::collections::HashSet;
use std::sync::Mutex;

pub struct Playback {
    playing: Mutex<HashSet<SongId>>,
}

impl Playback {
    pub fn new() -> Self {
        Self {
            playing: Mutex::new(HashSet::new()),
        }
    }

    // TODO(Persona 1): pub fn mark_playing(&self, id: SongId)
    // TODO(Persona 1): pub fn mark_stopped(&self, id: SongId)
    // TODO(Persona 1): pub fn is_playing(&self, id: SongId) -> bool
    // TODO(Persona 1): pub fn snapshot(&self) -> Vec<SongId>

    #[allow(dead_code)]
    fn _silence_unused(&self) {
        let _ = &self.playing;
    }
}
