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
}
