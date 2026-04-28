// Estado inmutable de playlists. Owner: Persona 1.
// Todos los tipos son `Clone` con structural sharing O(log n) gracias a `im`.

use crate::domain::{Playlist, PlaylistId};

#[derive(Clone, Default)]
pub struct State {
    pub playlists: im::HashMap<PlaylistId, Playlist>,
    pub next_id: PlaylistId,
}

impl State {
    pub fn new() -> Self {
        Self {
            playlists: im::HashMap::new(),
            next_id: 1,
        }
    }

    pub fn to_snapshot(&self) -> (Vec<Playlist>, PlaylistId) {
        (self.playlists.values().cloned().collect(), self.next_id)
    }

    pub fn from_snapshot(playlists: Vec<Playlist>, next_id: PlaylistId) -> Self {
        let map = playlists.into_iter().map(|p| (p.id, p)).collect();
        Self {
            playlists: map,
            next_id: if next_id == 0 { 1 } else { next_id },
        }
    }
}
