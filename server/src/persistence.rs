use crate::domain::{Playlist, Song};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

pub async fn spawn_debouncer(mut rx: Receiver<()>) {
    tokio::spawn(async move {
        while rx.recv().await.is_some() {
            sleep(Duration::from_millis(500)).await;

            // aquí conectas:
            // save_library(...)
            // save_playlists(...)
        }
    });
}