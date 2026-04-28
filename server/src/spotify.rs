// Cliente de Spotify API (solo metadata + preview_url). Owner: Persona 1.
//
// fetch_track usa reqwest directo en vez de rspotify::client::track porque rspotify
// 0.13 declara `popularity: u32` (no Option) y desde 2024 Spotify devuelve `null`
// en muchos tracks vía Client Credentials → la deserialización fallaba con
// "missing field popularity". Acá deserializamos solo los campos que necesitamos
// con `Option<…>` donde corresponde.

use crate::domain::Song;
use rspotify::{ClientCredsSpotify, Credentials, clients::BaseClient};
use serde::Deserialize;

pub struct SpotifyClient {
    spotify: ClientCredsSpotify,
    http: reqwest::Client,
}

#[derive(Deserialize)]
struct TrackResp {
    name: String,
    artists: Vec<ArtistResp>,
    album: AlbumResp,
    duration_ms: u64,
    preview_url: Option<String>,
}

#[derive(Deserialize)]
struct ArtistResp {
    name: String,
}

#[derive(Deserialize)]
struct AlbumResp {
    name: String,
    images: Vec<ImageResp>,
    release_date: Option<String>,
}

#[derive(Deserialize)]
struct ImageResp {
    url: String,
}

#[derive(Deserialize)]
struct SpotifyErrorEnvelope {
    error: SpotifyErrorBody,
}

#[derive(Deserialize)]
struct SpotifyErrorBody {
    status: u16,
    message: String,
}

impl SpotifyClient {
    /// Inicializa cliente con Client Credentials Flow
    pub async fn new() -> anyhow::Result<Self> {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID")?;
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET")?;

        let creds = Credentials::new(&client_id, &client_secret);
        let spotify = ClientCredsSpotify::new(creds);
        spotify.request_token().await?;

        Ok(Self {
            spotify,
            http: reqwest::Client::new(),
        })
    }

    /// Devuelve el access token vigente. Refresca antes de devolverlo si ya
    /// expiró (TTL de Client Credentials = 1 h). Sin esto, un servidor que
    /// corre >1 h vería el siguiente `add-spotify` fallar con HTTP 401.
    async fn access_token(&self) -> anyhow::Result<String> {
        let needs_refresh = {
            let guard = self.spotify.token.lock().await
                .map_err(|e| anyhow::anyhow!("token lock poisoned: {e:?}"))?;
            match guard.as_ref() {
                None => true,
                Some(t) => t.is_expired(),
            }
        };

        if needs_refresh {
            self.spotify.refresh_token().await?;
        }

        let guard = self.spotify.token.lock().await
            .map_err(|e| anyhow::anyhow!("token lock poisoned: {e:?}"))?;
        let token = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("token still missing after refresh"))?;
        Ok(token.access_token.clone())
    }

    /// Extrae el mensaje de error específico de Spotify del body
    /// (`{"error":{"status":N,"message":"..."}}`) en vez de devolver solo el
    /// status code. Cumple la regla de "Read the returned error message and
    /// use it to provide meaningful feedback to the user".
    async fn parse_spotify_error(resp: reqwest::Response) -> anyhow::Error {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if let Ok(parsed) = serde_json::from_str::<SpotifyErrorEnvelope>(&body) {
            anyhow::anyhow!("Spotify {} — {}", parsed.error.status, parsed.error.message)
        } else {
            anyhow::anyhow!("Spotify {} (no body)", status)
        }
    }

    /// Obtiene un track por ID y lo convierte en Song.
    pub async fn fetch_track(&self, id: &str) -> anyhow::Result<Song> {
        let token = self.access_token().await?;

        let resp = self
            .http
            .get(format!("https://api.spotify.com/v1/tracks/{id}"))
            .bearer_auth(token)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(Self::parse_spotify_error(resp).await);
        }

        let track: TrackResp = resp.json().await?;

        let year = track
            .album
            .release_date
            .as_deref()
            .and_then(|s| s.get(0..4).and_then(|y| y.parse::<u16>().ok()))
            .unwrap_or(0);

        Ok(Song {
            id: 0,
            title: track.name,
            artist: track
                .artists
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_default(),
            album: track.album.name,
            genre: "spotify".into(),
            year,
            duration_secs: (track.duration_ms / 1000) as u32,
            file_path: None,
            spotify_preview_url: track.preview_url,
            cover_url: track.album.images.first().map(|i| i.url.clone()),
        })
    }
}
