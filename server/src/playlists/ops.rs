// Operaciones puras sobre el estado de playlists. Owner: Persona 1.
//
// REGLAS (estrictas):
//   ❌ NO `let mut`
//   ❌ NO `&mut self`, NO mutación en sitio
//   ✅ Cada función que "modifica" toma `&State` y devuelve un `State` nuevo
//   ✅ Usar `.update_with`, `.insert`, `.without` de `im::HashMap` (devuelven copias)
//   ✅ Queries usan `.iter().map().filter().fold().collect()` + closures
//
// Wiring (en ws.rs):
//     let next = ops::add_song(&current, pid, sid);
//     *state.write().await = next;   // swap atómico

use super::state::State;
use crate::domain::{Playlist, PlaylistId, Song, SongId};

// ─────────────────────────────────────────────────────────────────────────
// Mutaciones (devuelven State nuevo)
// ─────────────────────────────────────────────────────────────────────────

// TODO(Persona 1): pub fn create(state: &State, name: String) -> (State, PlaylistId)
//   let id = state.next_id;
//   let pl = Playlist { id, name, songs: im::Vector::new() };
//   (State { playlists: state.playlists.update(id, pl), next_id: id + 1 }, id)

// TODO(Persona 1): pub fn delete(state: &State, pid: PlaylistId) -> State

// TODO(Persona 1): pub fn add_song(state: &State, pid: PlaylistId, sid: SongId) -> State
//   state.playlists.get(&pid).map(|pl| {
//       let new_songs = pl.songs.clone().push_back(sid);     // im::Vector clone O(1)
//       let new_pl = Playlist { songs: new_songs, ..pl.clone() };
//       State { playlists: state.playlists.update(pid, new_pl), ..state.clone() }
//   }).unwrap_or_else(|| state.clone())

// TODO(Persona 1): pub fn remove_song(state: &State, pid: PlaylistId, sid: SongId) -> State

// TODO(Persona 1): pub fn reorder(state: &State, pid: PlaylistId, order: im::Vector<SongId>) -> State

// ─────────────────────────────────────────────────────────────────────────
// Queries (no mutan — obligatorio usar map/filter/fold + closures)
// ─────────────────────────────────────────────────────────────────────────

// TODO(Persona 1): pub fn filter_songs<F: Fn(&Song) -> bool>(
//     p: &Playlist, library: &[Song], pred: F,
// ) -> im::Vector<Song>
//   p.songs.iter()
//     .filter_map(|sid| library.iter().find(|s| s.id == *sid))
//     .filter(|s| pred(s))
//     .cloned()
//     .collect()

// TODO(Persona 1): pub fn sort_by<K: Ord, F: Fn(&Song) -> K>(
//     p: &Playlist, library: &[Song], key: F,
// ) -> im::Vector<Song>

// TODO(Persona 1): pub fn total_duration(p: &Playlist, library: &[Song]) -> u32
//   p.songs.iter()
//     .filter_map(|sid| library.iter().find(|s| s.id == *sid))
//     .fold(0u32, |acc, s| acc + s.duration_secs)

#[allow(dead_code)]
fn _silence_unused() {
    let _: Option<&Playlist> = None;
    let _: PlaylistId = 0;
}
