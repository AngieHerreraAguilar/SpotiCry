// SpotiCry — servidor
//
// Bootstrap: arranca la CLI (task paralela) + servidor axum (WebSocket + HTTP Range).
// Owner: Persona 1 (backend). Coordinado con Persona 2 en Día 1.

mod cli;          // Persona 2
mod domain;       // Persona 1
mod http_stream;  // Persona 1
mod library;      // Persona 1
mod persistence;  // Persona 1
mod playback;     // Persona 1
mod playlists;    // Persona 1 (módulo funcional)
mod protocol;     // Persona 2
mod spotify;      // Persona 1
mod ws;           // Persona 1

use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,spoticry_server=debug".into()),
        )
        .init();

    let state = Arc::new(AppState::load().await?);

    // TODO(Persona 1): spawn CLI loop on a separate task
    // TODO(Persona 1): start axum server with WS + HTTP routes on port 8080
    // TODO(Persona 1): wire graceful shutdown that flushes persistence

    tracing::info!("SpotiCry server starting (stub)");
    let _ = state;
    Ok(())
}

/// Shared application state. Persona 1 fills in concrete types from each module.
pub struct AppState {
    // pub library: Arc<tokio::sync::RwLock<library::Library>>,
    // pub playlists: Arc<tokio::sync::RwLock<playlists::state::State>>,
    // pub playback: Arc<playback::Playback>,
    // pub broadcast: tokio::sync::broadcast::Sender<protocol::ServerEvent>,
}

impl AppState {
    pub async fn load() -> anyhow::Result<Self> {
        Ok(Self {})
    }
}
