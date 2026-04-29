use super::state::State;
use crate::domain::{Playlist, PlaylistId, Song, SongId};

// ─────────────────────────────────────────────────────────────────────────
// Mutaciones (devuelven State nuevo)
// ─────────────────────────────────────────────────────────────────────────

/// Crea una playlist nueva. Devuelve `None` si el nombre está vacío
/// (tras trim) o si ya existe otra playlist con el mismo nombre
/// (case-insensitive). Sigue siendo función pura: solo lee `state`.
pub fn create(state: &State, name: String) -> Option<(State, PlaylistId)> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let needle = trimmed.to_lowercase();
    let duplicate = state.playlists.values()
        .any(|p| p.name.trim().to_lowercase() == needle);
    if duplicate {
        return None;
    }

    let id = state.next_id;
    let pl = Playlist {
        id,
        name: trimmed.to_string(),
        songs: im::Vector::new(),
    };

    Some((
        State {
            playlists: state.playlists.update(id, pl),
            next_id: id + 1,
        },
        id,
    ))
}

pub fn delete(state: &State, pid: PlaylistId) -> State {
    State {
        playlists: state.playlists.without(&pid),
        ..state.clone()
    }
}

pub fn add_song(state: &State, pid: PlaylistId, sid: SongId) -> State {
    state.playlists.get(&pid).map(|pl| {
        let new_songs = {
            let mut v = pl.songs.clone();
            v.push_back(sid);
            v
        };

        let new_pl = Playlist {
            songs: new_songs,
            ..pl.clone()
        };

        State {
            playlists: state.playlists.update(pid, new_pl),
            ..state.clone()
        }
    }).unwrap_or_else(|| state.clone())
}

pub fn remove_song(state: &State, pid: PlaylistId, sid: SongId) -> State {
    state.playlists.get(&pid).map(|pl| {
        let new_songs = pl.songs
            .iter()
            .filter(|s| **s != sid)
            .cloned()
            .collect::<im::Vector<SongId>>();

        let new_pl = Playlist {
            songs: new_songs,
            ..pl.clone()
        };

        State {
            playlists: state.playlists.update(pid, new_pl),
            ..state.clone()
        }
    }).unwrap_or_else(|| state.clone())
}

pub fn reorder(state: &State, pid: PlaylistId, order: im::Vector<SongId>) -> State {
    state.playlists.get(&pid).map(|pl| {
        let new_pl = Playlist {
            songs: order,
            ..pl.clone()
        };

        State {
            playlists: state.playlists.update(pid, new_pl),
            ..state.clone()
        }
    }).unwrap_or_else(|| state.clone())
}

// ─────────────────────────────────────────────────────────────────────────
// Queries (no mutan — usando map/filter/fold)
// ─────────────────────────────────────────────────────────────────────────

pub fn filter_songs<F: Fn(&Song) -> bool>(
    p: &Playlist,
    library: &[Song],
    pred: F,
) -> im::Vector<Song> {
    p.songs
        .iter()
        .filter_map(|sid| library.iter().find(|s| s.id == *sid))
        .filter(|s| pred(s))
        .cloned()
        .collect()
}

pub fn sort_by<K: Ord, F: Fn(&Song) -> K>(
    p: &Playlist,
    library: &[Song],
    key: F,
) -> im::Vector<Song> {
    let mut songs: Vec<Song> = p.songs
        .iter()
        .filter_map(|sid| library.iter().find(|s| s.id == *sid))
        .cloned()
        .collect();

    songs.sort_by_key(|s| key(s));

    songs.into_iter().collect()
}

/// Suma las duraciones de las canciones de la playlist.
///
/// Hoy el frontend computa este total localmente en JS. Se mantiene en Rust
/// porque es el ejemplo canónico de `fold` sobre una estructura inmutable
/// (`im::Vector`) — uno de los combinadores funcionales que el enunciado pide
/// demostrar en el módulo de playlists.
#[allow(dead_code)]
pub fn total_duration(p: &Playlist, library: &[Song]) -> u32 {
    p.songs
        .iter()
        .filter_map(|sid| library.iter().find(|s| s.id == *sid))
        .fold(0u32, |acc, s| acc + s.duration_secs)
}