// Handler WebSocket (axum). Owner: Persona 1.
//
// Este es el "imperative shell" del patrón functional-core/imperative-shell:
// recibe comandos, delega a los módulos correspondientes (library imperativo,
// playlists funcional), y publica eventos al resto de clientes por broadcast.

use crate::protocol::ClientMsg;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;

// TODO(Persona 1): pub async fn ws_handler(
//     ws: WebSocketUpgrade,
//     State(state): State<Arc<AppState>>,
// ) -> impl IntoResponse
// {
//     ws.on_upgrade(|socket| handle_socket(socket, state))
// }

// TODO(Persona 1): async fn handle_socket(socket: WebSocket, state: Arc<AppState>)
//   1. Suscribirse al broadcast channel (state.broadcast.subscribe())
//   2. Bucle tokio::select! entre:
//        - socket.recv()       → parse ClientMsg, delegar
//        - broadcast.recv()    → reenviar al cliente como ServerEvent
//   3. Al conectarse, enviar library.snapshot + playlist.snapshot inicial.

// Delegación por operación:
// - search             → library.rs
// - library.list       → library.rs
// - playlist.*         → playlists/ops.rs (módulo funcional) + swap de estado
// - play / stop        → playback.rs + broadcast NowPlaying/PlaybackStopped

#[allow(dead_code)]
async fn _silence_unused(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|mut socket: WebSocket| async move {
        while let Some(Ok(msg)) = socket.recv().await {
            if let Message::Text(text) = msg {
                let _: Result<ClientMsg, _> = serde_json::from_str(&text);
            }
        }
    })
}
