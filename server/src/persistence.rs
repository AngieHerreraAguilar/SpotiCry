use crate::app_state::AppState;
use crate::domain::{Playlist, Song};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

use tokio::sync::mpsc::Receiver;
use tokio::time::{sleep, Duration};

const LIBRARY_FILE: &str = "library.json";
const PLAYLISTS_FILE: &str = "playlists.json";

#[derive(Serialize, Deserialize, Default)]
pub struct LibrarySnapshot {
    pub songs: Vec<Song>,
    pub next_id: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct PlaylistsSnapshot {
    pub playlists: Vec<Playlist>,
    pub next_id: u64,
}

/* =========================
   LOAD
========================= */

pub async fn load_library() -> anyhow::Result<LibrarySnapshot> {
    if !Path::new(LIBRARY_FILE).exists() {
        return Ok(LibrarySnapshot::default());
    }

    let data = tokio::fs::read_to_string(LIBRARY_FILE).await?;
    Ok(serde_json::from_str(&data)?)
}

pub async fn load_playlists() -> anyhow::Result<PlaylistsSnapshot> {
    if !Path::new(PLAYLISTS_FILE).exists() {
        return Ok(PlaylistsSnapshot::default());
    }

    let data = tokio::fs::read_to_string(PLAYLISTS_FILE).await?;
    Ok(serde_json::from_str(&data)?)
}

/* =========================
   SAVE
========================= */

pub async fn save_library(snap: &LibrarySnapshot) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(snap)?;
    tokio::fs::write(LIBRARY_FILE, json).await?;
    Ok(())
}

pub async fn save_playlists(snap: &PlaylistsSnapshot) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(snap)?;
    tokio::fs::write(PLAYLISTS_FILE, json).await?;
    Ok(())
}

/* =========================
   DEBOUNCER
========================= */

/// Drena el canal con coalesce de 500 ms y vuelca library + playlists a JSON.
///
/// Llamada desde `main` con `tokio::spawn(run_debouncer(rx, state))`. Cada
/// mutación en `ws.rs` o `cli.rs` envía `()` por `state.save_tx.try_send(())`.
/// Si llegan varios eventos en ráfaga (p. ej. múltiples adds simultáneos desde
/// distintas pestañas) se agrupan en un solo flush gracias al `try_recv` loop.
pub async fn run_debouncer(mut rx: Receiver<()>, state: Arc<AppState>) {
    while rx.recv().await.is_some() {
        // Coalesce: espera 500ms y consume cualquier evento adicional que haya
        // llegado en ese intervalo. El siguiente flush los cubre a todos.
        sleep(Duration::from_millis(500)).await;
        while rx.try_recv().is_ok() {}

        let (lib_songs, lib_next) = state.library.read().await.to_snapshot();
        let (pls, pl_next) = state.playlists.read().await.to_snapshot();

        let lib_snap = LibrarySnapshot { songs: lib_songs, next_id: lib_next };
        let pl_snap = PlaylistsSnapshot { playlists: pls, next_id: pl_next };

        if let Err(e) = save_library(&lib_snap).await {
            tracing::warn!("debouncer: save_library failed: {e}");
        }
        if let Err(e) = save_playlists(&pl_snap).await {
            tracing::warn!("debouncer: save_playlists failed: {e}");
        }
    }
}