use axum::{
    extract::{Path, State},
    http::{Request, StatusCode, HeaderMap},
    response::IntoResponse,
    body::Body,
};

use std::sync::Arc;

use tower_http::services::ServeFile;
use tower::ServiceExt;

use crate::{
    app_state::AppState,
    domain::SongId,
};

/// GET /stream/:id
pub async fn stream_song(
    Path(id): Path<SongId>,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap, // 🔥 IMPORTANTE: para soportar Range
) -> impl IntoResponse {

    // 🔹 1. Buscar canción
    let song = {
        let lib = state.library.read().await;
        lib.get(id).cloned()
    };

    let song = match song {
        Some(s) => s,
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    // 🔹 2. Caso: archivo local
    if let Some(path) = song.file_path {

        state.playback.mark_playing(&id);

        let service = ServeFile::new(path);

        // 🔥 Crear request copiando headers (Range!)
        let mut request = Request::builder()
            .body(Body::empty())
            .unwrap();

        *request.headers_mut() = headers.clone();

        let response = match service.oneshot(request).await {
            Ok(res) => res.into_response(),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        state.playback.mark_stopped(&id);

        return response;
    }

    // 🔹 3. Caso: Spotify preview (proxy)
    if let Some(url) = song.spotify_preview_url {

        state.playback.mark_playing(&id);

        let client = reqwest::Client::new();

        let response = match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                let stream = resp.bytes_stream();

                let mut axum_response = (status, Body::from_stream(stream)).into_response();

                // 🔥 copiar headers importantes (content-type, etc)
                *axum_response.headers_mut() = headers;

                axum_response
            }
            Err(_) => StatusCode::BAD_GATEWAY.into_response(),
        };

        state.playback.mark_stopped(&id);

        return response;
    }

    // 🔹 4. No hay fuente
    StatusCode::NOT_FOUND.into_response()
}