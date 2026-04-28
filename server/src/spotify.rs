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

    /// Devuelve el access token vigente; rspotify lo refresca solo si está expirado.
    async fn access_token(&self) -> anyhow::Result<String> {
        let guard = self.spotify.token.lock().await
            .map_err(|e| anyhow::anyhow!("token lock poisoned: {e:?}"))?;
        let token = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no token available"))?;
        Ok(token.access_token.clone())
    }

    /// Obtiene un track por ID y lo convierte en Song.
    pub async fn fetch_track(&self, id: &str) -> anyhow::Result<Song> {
        let token = self.access_token().await?;

        let track: TrackResp = self
            .http
            .get(format!("https://api.spotify.com/v1/tracks/{id}"))
            .bearer_auth(token)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

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
