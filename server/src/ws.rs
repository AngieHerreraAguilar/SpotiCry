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
                            handle_client_msg(cmd, &state).await;
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
}

/// Router lógico
async fn handle_client_msg(cmd: ClientMsg, state: &Arc<AppState>) {
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
            state.playback.mark_playing(&song_id);

            let _ = state.broadcast.send(ServerEvent::NowPlaying {
                song_id,
                stream_url: format!("/stream/{}", song_id),
            });
        }

        // ⏹ STOP
        ClientMsg::Stop { song_id } => {
            state.playback.mark_stopped(&song_id);

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
            let (next_state, _) = {
                let pl = state.playlists.read().await;
                ops::create(&pl, name)
            };

            {
                let mut pl = state.playlists.write().await;
                *pl = next_state.clone();
            }

            let playlists_vec = next_state.playlists.values().cloned().collect();

            let _ = state.broadcast.send(ServerEvent::PlaylistSnapshot {
                playlists: playlists_vec,
            });
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
            }
        }

        _ => {}
    }
}