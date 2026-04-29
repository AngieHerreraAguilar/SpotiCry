use axum::{
    extract::{Path, State},
    http::{header::CACHE_CONTROL, HeaderMap, HeaderValue, Request, StatusCode},
    response::{IntoResponse, Response},
    body::Body,
};

use std::sync::Arc;

use tower_http::services::ServeFile;
use tower::ServiceExt;

use crate::{
    app_state::AppState,
    domain::SongId,
};

/// Inyecta `Cache-Control: no-cache` para que el navegador revalide cada
/// Range request en vez de reusar respuestas antiguas. Sin esto, Chrome
/// aplica heurística agresiva sobre `/stream/<id>` y puede servir el audio
/// cacheado de un id anterior tras un reset de la library.
fn no_cache(mut response: Response) -> Response {
    response.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    response
}

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

        // mark_playing solo: el `mark_stopped` lo dispara el cliente vía WS (op `stop`)
        // o el cierre del WebSocket. Marcar stopped aquí ejecutaba antes de que el
        // cliente terminara de leer el stream y rompía la regla "no borrar canción
        // reproduciéndose".
        state.playback.mark_playing(&id);

        let service = ServeFile::new(path);

        // 🔥 Crear request copiando headers (Range!)
        let mut request = Request::builder()
            .body(Body::empty())
            .unwrap();

        *request.headers_mut() = headers.clone();

        return match service.oneshot(request).await {
            Ok(res) => no_cache(res.into_response()),
            Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
    }

    // 🔹 3. Caso: Spotify preview (proxy)
    if let Some(url) = song.spotify_preview_url {

        state.playback.mark_playing(&id);

        let client = reqwest::Client::new();

        return match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                let headers = resp.headers().clone();
                let stream = resp.bytes_stream();

                let mut axum_response = (status, Body::from_stream(stream)).into_response();

                // 🔥 copiar headers importantes (content-type, etc)
                *axum_response.headers_mut() = headers;

                no_cache(axum_response)
            }
            Err(_) => StatusCode::BAD_GATEWAY.into_response(),
        };
    }

    // 🔹 4. No hay fuente
    StatusCode::NOT_FOUND.into_response()
}