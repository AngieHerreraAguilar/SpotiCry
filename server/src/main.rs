// SpotiCry — servidor
//
// Bootstrap: CLI (task paralela) + servidor axum (WebSocket + HTTP Range)

mod cli;
mod domain;
mod http_stream;
mod library;
mod persistence;
mod playback;
mod playlists;
mod protocol;
mod spotify;
mod ws;
mod app_state;

use std::sync::Arc;

use axum::{Router, routing::get};
use tokio::sync::broadcast;

use crate::{
    app_state::AppState,
    ws::ws_handler,
    http_stream::stream_song,
    library::Library,
    playback::Playback,
    playlists::state::State as Playlists,
    spotify::SpotifyClient,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Carga server/.env si existe (provee SPOTIFY_CLIENT_ID/SECRET sin prefijar el comando).
    let _ = dotenvy::dotenv();

    // 🔹 Logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,spoticry_server=debug".into()),
        )
        .init();

    // 🔹 Broadcast channel
    let (tx, _) = broadcast::channel(100);

    // 🔹 Cargar persistencia
    let snap = persistence::load_library().await?;

    let library = Library::from_snapshot(
        snap.songs,
        snap.next_id,
    );

    let playlists = Playlists::new();
    let playback = Playback::new();

    // 🔹 Spotify (opcional). Sin SPOTIFY_CLIENT_ID/SECRET el server arranca igual,
    //    solo `add-spotify` queda deshabilitado.
    let spotify = match SpotifyClient::new().await {
        Ok(c) => {
            tracing::info!("✓ Spotify conectado (Client Credentials)");
            Some(Arc::new(c))
        }
        Err(e) => {
            tracing::warn!(
                "Spotify deshabilitado ({e}). Setea SPOTIFY_CLIENT_ID y SPOTIFY_CLIENT_SECRET para habilitar add-spotify."
            );
            None
        }
    };

    // 🔹 Crear estado global
    let state = Arc::new(AppState::new(
        library,
        playlists,
        playback,
        tx,
        spotify,
    ));

    // 🔹 CLI en paralelo (no bloquea el server)
    let cli_state = state.clone();

    tokio::spawn(async move {
        if let Err(e) = crate::cli::run(cli_state).await {
            tracing::error!("CLI error: {:?}", e);
        }
    });

    // 🔹 Router Axum
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/stream/:id", get(stream_song))
        .with_state(state);

    // 🔹 Servidor (puerto 8080 requerido)
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;

    tracing::info!("🚀 Server running on http://localhost:8080");

    axum::serve(listener, app).await?;

    Ok(())
}