use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};

use crate::{
    app_state::AppState,
    domain::{Song, SongId},
    playlists::ops,
    protocol::{ClientMsg, ServerEvent, SearchBy, ErrorCode, SortBy},
};

/// Entry point WebSocket
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Maneja conexión viva
async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.broadcast.subscribe();

    // Canciones que este cliente dejó "playing" sin haber enviado `stop`.
    // Si la conexión se cae (cierre de pestaña), las marcamos stopped al final
    // para que la regla "no borrar canción en reproducción" no quede congelada.
    let mut owned_playing: HashSet<SongId> = HashSet::new();

    // 🔹 Snapshot inicial (SIN locks durante await)
    let songs = {
        let lib = state.library.read().await;
        lib.list()
    };

    let playlists_vec = {
        let pl = state.playlists.read().await;
        pl.playlists.values().cloned().collect::<Vec<_>>()
    };

    let _ = socket.send(Message::Text(
        serde_json::to_string(&ServerEvent::LibrarySnapshot { songs }).unwrap()
    )).await;

    let _ = socket.send(Message::Text(
        serde_json::to_string(&ServerEvent::PlaylistSnapshot {
            playlists: playlists_vec,
        }).unwrap()
    )).await;

    // 🔁 Loop principal
    loop {
        tokio::select! {

            // 📩 Cliente
            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {

                    match serde_json::from_str::<ClientMsg>(&text) {

                        Ok(cmd) => {
                            handle_client_msg(cmd, &state, &mut owned_playing).await;
                        }

                        Err(_) => {
                            let _ = socket.send(Message::Text(
                                serde_json::to_string(&ServerEvent::Error {
                                    code: ErrorCode::BadRequest,
                                    msg: "invalid message".into(),
                                }).unwrap()
                            )).await;
                        }
                    }
                }
            }

            // 📡 Broadcast
            Ok(event) = rx.recv() => {
                let _ = socket.send(Message::Text(
                    serde_json::to_string(&event).unwrap()
                )).await;
            }

            // 🔌 Desconexión
            else => break,
        }
    }

    // 🔌 Cleanup: liberar las canciones que este cliente tenía "playing".
    for id in owned_playing.drain() {
        state.playback.mark_stopped(&id);
        let _ = state.broadcast.send(ServerEvent::PlaybackStopped { song_id: id });
    }
}

/// Router lógico
async fn handle_client_msg(
    cmd: ClientMsg,
    state: &Arc<AppState>,
    owned_playing: &mut HashSet<SongId>,
) {
    match cmd {

        // 🔍 SEARCH
        ClientMsg::Search { by, value } => {
            let result = {
                let lib = state.library.read().await;

                match by {
                    SearchBy::Title => {
                        lib.search_by_title(value.as_str().unwrap_or(""))
                    }
                    SearchBy::Genre => {
                        lib.search_by_genre(value.as_str().unwrap_or(""))
                    }
                    SearchBy::YearRange => {
                        let range: Vec<u16> =
                            serde_json::from_value(value).unwrap_or_default();

                        if range.len() == 2 {
                            lib.search_by_year_range(range[0], range[1])
                        } else {
                            vec![]
                        }
                    }
                }
            };

            let _ = state.broadcast.send(ServerEvent::SearchResult {
                songs: result,
            });
        }

        // 📚 LIBRARY LIST
        ClientMsg::LibraryList => {
            let songs = {
                let lib = state.library.read().await;
                lib.list()
            };

            let _ = state.broadcast.send(ServerEvent::LibrarySnapshot {
                songs,
            });
        }

        // ▶ PLAY
        ClientMsg::Play { song_id } => {
            // Validación: NotFound si la canción no existe en library.
            let exists = state.library.read().await.get(song_id).is_some();
            if !exists {
                let _ = state.broadcast.send(ServerEvent::Error {
                    code: ErrorCode::NotFound,
                    msg: format!("song {song_id} not found"),
                });
                return;
            }

            state.playback.mark_playing(&song_id);
            owned_playing.insert(song_id);

            let _ = state.broadcast.send(ServerEvent::NowPlaying {
                song_id,
                stream_url: format!("/stream/{}", song_id),
            });
        }

        // ⏹ STOP
        ClientMsg::Stop { song_id } => {
            state.playback.mark_stopped(&song_id);
            owned_playing.remove(&song_id);

            let _ = state.broadcast.send(ServerEvent::PlaybackStopped {
                song_id,
            });
        }

        // 🎼 PLAYLIST LIST
        ClientMsg::PlaylistList => {
            let playlists_vec = {
                let pl = state.playlists.read().await;
                pl.playlists.values().cloned().collect::<Vec<_>>()
            };

            let _ = state.broadcast.send(ServerEvent::PlaylistSnapshot {
                playlists: playlists_vec,
            });
        }

        // ➕ CREATE
        ClientMsg::PlaylistCreate { name } => {
            let result = {
                let pl = state.playlists.read().await;
                ops::create(&pl, name.clone())
            };

            match result {
                Some((next_state, _)) => {
                    {
                        let mut pl = state.playlists.write().await;
                        *pl = next_state.clone();
                    }
                    let playlists_vec = next_state.playlists.values().cloned().collect();
                    let _ = state.broadcast.send(ServerEvent::PlaylistSnapshot {
                        playlists: playlists_vec,
                    });
                    let _ = state.save_tx.try_send(());
                }
                None => {
                    let trimmed = name.trim();
                    let msg = if trimmed.is_empty() {
                        "El nombre no puede estar vacío".to_string()
                    } else {
                        format!("Ya existe una playlist con el nombre '{trimmed}'")
                    };
                    let _ = state.broadcast.send(ServerEvent::Error {
                        code: ErrorCode::BadRequest,
                        msg,
                    });
                }
            }
        }

        // ❌ DELETE
        ClientMsg::PlaylistDelete { playlist_id } => {
            let next_state = {
                let pl = state.playlists.read().await;
                ops::delete(&pl, playlist_id)
            };

            {
                let mut pl = state.playlists.write().await;
                *pl = next_state.clone();
            }

            let playlists_vec = next_state.playlists.values().cloned().collect();

            let _ = state.broadcast.send(ServerEvent::PlaylistSnapshot {
                playlists: playlists_vec,
            });
            let _ = state.save_tx.try_send(());
        }

        // ➕ ADD SONG
        ClientMsg::PlaylistAdd { playlist_id, song_id } => {
            let next_state = {
                let pl = state.playlists.read().await;
                ops::add_song(&pl, playlist_id, song_id)
            };

            {
                let mut pl = state.playlists.write().await;
                *pl = next_state.clone();
            }

            let playlists_vec = next_state.playlists.values().cloned().collect();

            let _ = state.broadcast.send(ServerEvent::PlaylistSnapshot {
                playlists: playlists_vec,
            });
            let _ = state.save_tx.try_send(());
        }

        // ➖ REMOVE SONG
        ClientMsg::PlaylistRemove { playlist_id, song_id } => {
            let next_state = {
                let pl = state.playlists.read().await;
                ops::remove_song(&pl, playlist_id, song_id)
            };

            {
                let mut pl = state.playlists.write().await;
                *pl = next_state.clone();
            }

            let playlists_vec = next_state.playlists.values().cloned().collect();

            let _ = state.broadcast.send(ServerEvent::PlaylistSnapshot {
                playlists: playlists_vec,
            });
            let _ = state.save_tx.try_send(());
        }

        // 🔎 PLAYLIST FILTER (búsqueda con scope a una playlist concreta)
        ClientMsg::PlaylistFilter { playlist_id, by, value } => {
            let songs: Vec<Song> = {
                let lib = state.library.read().await;
                let pl = state.playlists.read().await;
                let library = lib.list();

                match pl.playlists.get(&playlist_id).cloned() {
                    Some(p) => match by {
                        SearchBy::Title => {
                            let q = value.as_str().unwrap_or("").to_lowercase();
                            ops::filter_songs(&p, &library, |s| {
                                s.title.to_lowercase().contains(&q)
                            }).into_iter().collect()
                        }
                        SearchBy::Genre => {
                            let g = value.as_str().unwrap_or("").to_string();
                            ops::filter_songs(&p, &library, |s| s.genre == g)
                                .into_iter().collect()
                        }
                        SearchBy::YearRange => {
                            let range: Vec<u16> =
                                serde_json::from_value(value).unwrap_or_default();
                            if range.len() == 2 {
                                let (from, to) = (range[0], range[1]);
                                ops::filter_songs(&p, &library, |s| {
                                    s.year >= from && s.year <= to
                                }).into_iter().collect()
                            } else {
                                vec![]
                            }
                        }
                    },
                    None => vec![],
                }
            };

            let _ = state.broadcast.send(ServerEvent::SearchResult { songs });
        }

        // 🔃 SORT
        ClientMsg::PlaylistSort { playlist_id, by } => {

            let (library_snapshot, playlist_opt) = {
                let lib = state.library.read().await;
                let pl = state.playlists.read().await;

                (
                    lib.list(),
                    pl.playlists.get(&playlist_id).cloned()
                )
            };

            if let Some(p) = playlist_opt {

                let sorted = match by {
                    SortBy::Title => ops::sort_by(&p, &library_snapshot, |s| s.title.clone()),
                    SortBy::Year => ops::sort_by(&p, &library_snapshot, |s| s.year),
                    SortBy::Duration => ops::sort_by(&p, &library_snapshot, |s| s.duration_secs),
                };

                let next_state = {
                    let pl = state.playlists.read().await;
                    ops::reorder(&pl, playlist_id, sorted.iter().map(|s| s.id).collect())
                };

                {
                    let mut pl = state.playlists.write().await;
                    *pl = next_state.clone();
                }

                let playlists_vec = next_state.playlists.values().cloned().collect();

                let _ = state.broadcast.send(ServerEvent::PlaylistSnapshot {
                    playlists: playlists_vec,
                });
                let _ = state.save_tx.try_send(());
            }
        }
    }
}