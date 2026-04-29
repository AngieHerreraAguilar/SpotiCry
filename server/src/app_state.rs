// Estado global compartido del servidor.
// Owner: Persona 1.
//
// Este struct conecta TODO el backend:
//
// - Library (canciones)
// - Playlists (módulo funcional)
// - Playback (estado runtime)
// - Broadcast (eventos WS)
//
// Se comparte entre:
// - WebSocket (ws.rs)
// - HTTP streaming (http_stream.rs)
// - CLI (opcional)
//
// IMPORTANTE:
// - Se usa Arc para compartir entre threads
// - Se usa Mutex para estructuras mutables
// - Playback ya es thread-safe internamente

use std::sync::Arc;

use tokio::sync::{RwLock, broadcast, mpsc};

use crate::{
    library::Library,
    playback::Playback,
    playlists::state::State as Playlists,
    protocol::ServerEvent,
    spotify::SpotifyClient,
};

#[derive(Clone)]
pub struct AppState {
    /// Biblioteca de canciones (lecturas concurrentes)
    pub library: Arc<RwLock<Library>>,

    /// Playlists (estado funcional)
    pub playlists: Arc<RwLock<Playlists>>,

    /// Estado de reproducción
    pub playback: Arc<Playback>,

    /// Canal broadcast WS
    pub broadcast: broadcast::Sender<ServerEvent>,

    /// Cliente Spotify. `None` si SPOTIFY_CLIENT_ID/SECRET no estaban presentes
    /// al arrancar — el resto del server arranca igual; solo `add-spotify` falla.
    pub spotify: Option<Arc<SpotifyClient>>,

    /// Notifica al debouncer de persistencia que hay una mutación pendiente
    /// de volcar a disco. `try_send` no-bloqueante: si el canal está lleno,
    /// el siguiente flush ya cubre la mutación porque el debouncer relee el
    /// estado completo, no un diff.
    pub save_tx: mpsc::Sender<()>,
}

impl AppState {
    pub fn new(
        library: Library,
        playlists: Playlists,
        playback: Playback,
        broadcast: broadcast::Sender<ServerEvent>,
        spotify: Option<Arc<SpotifyClient>>,
        save_tx: mpsc::Sender<()>,
    ) -> Self {
        Self {
            library: Arc::new(RwLock::new(library)),
            playlists: Arc::new(RwLock::new(playlists)),
            playback: Arc::new(playback),
            broadcast,
            spotify,
            save_tx,
        }
    }
}