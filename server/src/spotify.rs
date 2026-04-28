// Cliente de Spotify API (solo metadata + preview_url). Owner: Persona 1.

use crate::domain::Song;
use anyhow::Result;
use rspotify::{
    clients::BaseClient,
    ClientCredsSpotify, Credentials,
    model::TrackId,
};

pub struct SpotifyClient {
    spotify: ClientCredsSpotify,
}

impl SpotifyClient {
    /// Inicializa cliente con Client Credentials Flow
    pub async fn new() -> anyhow::Result<Self> {
        let client_id = std::env::var("SPOTIFY_CLIENT_ID")?;
        let client_secret = std::env::var("SPOTIFY_CLIENT_SECRET")?;

        let creds = Credentials::new(&client_id, &client_secret);

        let spotify = ClientCredsSpotify::new(creds);

        spotify.request_token().await?;

        Ok(Self { spotify })
    }

    /// Obtiene un track por ID y lo convierte en Song
    pub async fn fetch_track(&self, id: &str) -> anyhow::Result<Song> {
        let track_id = TrackId::from_id(id)?;

        let track = self.spotify.track(track_id, None).await?;

        Ok(Song {
            id: 0,
            title: track.name,
            artist: track.artists.first().map(|a| a.name.clone()).unwrap_or_default(),
            album: track.album.name,
            genre: "spotify".into(),
            year: 0,
            duration_secs: track.duration.num_seconds() as u32,
            file_path: None,
            spotify_preview_url: track.preview_url,
            cover_url: track.album.images.first().map(|i| i.url.clone()),
        })
    }

    /// Búsqueda simple (opcional para enriquecer MP3 locales)
pub async fn search(&self, query: &str) -> anyhow::Result<Vec<Song>> {
    let result = self.spotify
        .search(
            query,
            rspotify::model::SearchType::Track,
            None,
            None,
            Some(10),
            None,
        )
        .await?;

    let mut songs = Vec::new();

    if let rspotify::model::search::SearchResult::Tracks(tracks) = result {

        for t in tracks.items {

            let artist = t
                .artists
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            let album = t.album.name.clone();

            let preview = t.preview_url.clone();

            let cover = t
                .album
                .images
                .first()
                .map(|i| i.url.clone());

            songs.push(Song {
                id: 0,
                title: t.name.clone(),
                artist,
                album,
                genre: "spotify".to_string(),
                year: 0,
                duration_secs: t.duration.num_seconds() as u32,
                file_path: None,
                spotify_preview_url: preview,
                cover_url: cover,
            });
        }
    }

    Ok(songs)
}
}