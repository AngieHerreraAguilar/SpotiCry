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

    pub fn mark_playing(&self, id: &SongId) {
        let mut set = self.playing.lock().unwrap();
        set.insert(*id);
    }

    pub fn mark_stopped(&self, id: &SongId) {
        let mut set = self.playing.lock().unwrap();
        set.remove(id);
    }

    pub fn is_playing(&self, id: &SongId) -> bool {
        let set = self.playing.lock().unwrap();
        set.contains(id)
    }

    pub fn snapshot(&self) -> Vec<SongId> {
        let set = self.playing.lock().unwrap();
        set.iter().cloned().collect()
    }
}

// ✔ MVP global state (aceptado en Día 1)
static PLAYBACK: Lazy<Playback> = Lazy::new(Playback::new);

pub fn global() -> &'static Playback {
    &PLAYBACK
}