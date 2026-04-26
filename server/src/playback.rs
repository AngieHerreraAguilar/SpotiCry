// Tracking de canciones "en reproducción".
// Owner: Persona 1.
//
// Permite marcar canciones como reproduciéndose,
// detenerlas y consultar su estado.
//
// Implementado como estado global sincronizado para
// poder ser accedido desde cualquier módulo (ej: library).

use crate::domain::SongId;
use std::collections::HashSet;
use std::sync::Mutex;
use once_cell::sync::Lazy;

pub struct Playback {
    playing: Mutex<HashSet<SongId>>,
}

impl Playback {
    pub fn new() -> Self {
        Self {
            playing: Mutex::new(HashSet::new()),
        }
    }

    pub fn mark_playing(&self, id: SongId) {
        let mut set = self.playing.lock().unwrap();
        set.insert(id);
    }

    pub fn mark_stopped(&self, id: SongId) {
        let mut set = self.playing.lock().unwrap();
        set.remove(&id);
    }

    pub fn is_playing(&self, id: SongId) -> bool {
        let set = self.playing.lock().unwrap();
        set.contains(&id)
    }

    pub fn snapshot(&self) -> Vec<SongId> {
        let set = self.playing.lock().unwrap();
        set.iter().cloned().collect()
    }
}

// 🌍 Estado global accesible desde todo el crate
static PLAYBACK: Lazy<Playback> = Lazy::new(|| Playback::new());

/// Función global para consultar si una canción está en reproducción
pub fn is_playing(id: SongId) -> bool {
    PLAYBACK.is_playing(id)
}

/// Marca una canción como en reproducción
pub fn mark_playing(id: SongId) {
    PLAYBACK.mark_playing(id)
}

/// Marca una canción como detenida
pub fn mark_stopped(id: SongId) {
    PLAYBACK.mark_stopped(id)
}

/// Obtiene snapshot de canciones en reproducción
pub fn snapshot() -> Vec<SongId> {
    PLAYBACK.snapshot()
}