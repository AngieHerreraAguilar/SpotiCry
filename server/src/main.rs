// SpotiCry — servidor
//
// Bootstrap: CLI (task paralela) + servidor axum (WebSocket + HTTP Range)

mod cli;
mod domain;
mod http_stream;
mod library;
mod library_scan;
mod persistence;
mod playback;
mod playlists;
mod protocol;
mod spotify;
mod ws;
mod app_state;

use std::path::Path;
use std::sync::Arc;

use axum::{Router, routing::get};
use tokio::sync::{broadcast, mpsc};

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

    // 🔹 Canal de notificación al debouncer de persistencia.
    //    Capacidad pequeña: como el debouncer hace coalesce, basta con que
    //    quepa al menos 1 evento; 32 da margen para ráfagas multi-pestaña.
    let (save_tx, save_rx) = mpsc::channel::<()>(32);

    // 🔹 Cargar persistencia
    let lib_snap = persistence::load_library().await?;
    let pl_snap = persistence::load_playlists().await?;

    // Limpiar entradas con file_path apuntando a archivos que ya no existen
    // (suelen quedar tras renombrar/mover MP3s). Las canciones solo-Spotify
    // (sin file_path) se conservan.
    let original_count = lib_snap.songs.len();
    let lib_songs_clean: Vec<_> = lib_snap.songs
        .into_iter()
        .filter(|s| match &s.file_path {
            Some(p) => Path::new(p).exists(),
            None => true,
        })
        .collect();
    let cleaned = original_count - lib_songs_clean.len();
    if cleaned > 0 {
        tracing::info!("✓ limpiadas {cleaned} canción(es) con file_path inexistente");
    }

    let mut library = Library::from_snapshot(lib_songs_clean, lib_snap.next_id);
    let playlists = Playlists::from_snapshot(pl_snap.playlists, pl_snap.next_id);
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

    // 🔹 Auto-scan de library/: detecta MP3s nuevos antes de arrancar HTTP.
    //    `configured` se carga directo; `pending` se le pregunta al usuario en CLI.
    let library_dir = std::path::PathBuf::from(library_scan::LIBRARY_DIR);
    let tracks_path = std::path::PathBuf::from(library_scan::TRACKS_FILE);
    let tracks_map = library_scan::load_tracks_map(&tracks_path).unwrap_or_default();

    let already_loaded: std::collections::HashSet<String> = library
        .list()
        .iter()
        .filter_map(|s| s.file_path.clone())
        .collect();

    let scan = library_scan::scan_dir(
        &library_dir,
        &tracks_map,
        spotify.as_ref(),
        &already_loaded,
    )
    .await?;

    let scan_loaded = scan.configured.len();
    for song in scan.configured {
        let title = song.title.clone();
        let artist = song.artist.clone();
        let id = library.add_song(song);
        tracing::info!("✓ scan: cargada (id {id}) {artist} — {title}");
    }

    let pending_tracks = scan.pending;

    // ¿Hubo cambios respecto a lo que está en disco? Si sí, forzar un flush
    // tras crear el debouncer para que `library.json` quede consistente sin
    // esperar a que el usuario haga otra acción.
    let dirty_after_boot = scan_loaded > 0 || cleaned > 0;

    // 🔹 Crear estado global
    let state = Arc::new(AppState::new(
        library,
        playlists,
        playback,
        tx,
        spotify,
        save_tx,
    ));

    // 🔹 Debouncer de persistencia: vuelca library.json + playlists.json
    //    cuando ws/cli notifican una mutación. Sin esto, los cambios por
    //    WebSocket se perdían si el server caía sin `quit` en CLI.
    tokio::spawn(persistence::run_debouncer(save_rx, state.clone()));

    if dirty_after_boot {
        let _ = state.save_tx.try_send(());
    }

    // 🔹 CLI en paralelo (no bloquea el server)
    let cli_state = state.clone();
    let cli_tracks_path = tracks_path.clone();
    let cli_tracks_map = tracks_map.clone();

    tokio::spawn(async move {
        if let Err(e) = crate::cli::run(
            cli_state,
            pending_tracks,
            cli_tracks_path,
            cli_tracks_map,
        ).await {
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