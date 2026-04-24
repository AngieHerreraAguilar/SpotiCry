// Cliente de Spotify API (solo metadata + preview_url). Owner: Persona 1.
//
// Usa rspotify con Client Credentials flow — no requiere login de usuario,
// solo SPOTIFY_CLIENT_ID y SPOTIFY_CLIENT_SECRET como variables de entorno.
//
// Usado desde cli.rs:
//   - `add <mp3>` → enriquece metadata del MP3 consultando Spotify por título/artista
//   - `add-spotify <track-id>` → registra la canción con solo preview_url (sin MP3)

use crate::domain::Song;

pub struct SpotifyClient {
    // TODO(Persona 1): campos (ClientCredsSpotify de rspotify)
}

impl SpotifyClient {
    // TODO(Persona 1): pub async fn new() -> anyhow::Result<Self>
    //   Lee SPOTIFY_CLIENT_ID y SPOTIFY_CLIENT_SECRET de env, hace token request

    // TODO(Persona 1): pub async fn fetch_track(&self, track_id: &str) -> anyhow::Result<Song>
    //   Consulta /tracks/:id, mapea a crate::domain::Song con file_path=None
    //   y spotify_preview_url=Some(..)

    // TODO(Persona 1): pub async fn search(&self, query: &str) -> anyhow::Result<Vec<Song>>
    //   Útil para enriquecer metadata de un MP3 recién agregado
}

#[allow(dead_code)]
fn _silence_unused() {
    let _: Option<Song> = None;
}
