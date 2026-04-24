// Streaming de audio por HTTP con Range requests. Owner: Persona 1.
//
// Endpoint: GET /stream/:id
//   - Si song.file_path.is_some() → sirve el MP3 local con soporte Range nativo
//       (usar tower_http::services::ServeFile o implementación manual)
//   - Si solo hay spotify_preview_url → proxy hacia Spotify con reqwest y pipelinea body
//
// Antes de servir: playback.mark_playing(id)
// Al cerrar conexión o terminar stream: playback.mark_stopped(id)
//
// El chunking natural del navegador (64 KB por Range request) + el buffer del
// elemento <audio> del cliente resuelven el requerimiento de "buffer local con
// adelantar/retroceder cuantas veces se quiera para la canción actual".

use crate::domain::SongId;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;

// TODO(Persona 1): pub async fn stream_handler(
//     Path(id): Path<SongId>,
//     headers: HeaderMap,
//     State(state): State<Arc<AppState>>,
// ) -> impl IntoResponse

#[allow(dead_code)]
async fn _silence_unused(_headers: HeaderMap, _id: SongId) -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}
