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
}