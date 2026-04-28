// Interfaz de texto del servidor (CLI). Owner: Persona 2.
//
// Corre en una task paralela (tokio::spawn). Cumple el requerimiento del enunciado:
//   "Interfaz de texto (CLI) propia del servidor — permite agregar canciones
//    desde archivos locales y eliminar canciones existentes"
//
// Comandos:
//   add <ruta-mp3>        — lee tags ID3 y registra en library (+ Spotify si disponible)
//   add-spotify <trk-id>  — registra solo metadata/preview de Spotify (sin MP3)
//   remove <song-id>      — borra (falla si está sonando)
//   list                  — imprime biblioteca
//   playlists             — imprime playlists globales
//   help                  — muestra ayuda
//   quit / exit           — flushea persistencia y sale
//
// I/O: stdin/stdout asíncrono (tokio::io) para no bloquear el runtime mientras
// se espera input del usuario. La salida usa `print!`/`println!` (sync) — el
// flush eventual no compromete al servidor.

use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};

use crate::AppState;
use crate::persistence::{self, LibrarySnapshot, PlaylistsSnapshot};

pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    print_banner();
    print_help();

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    loop {
        prompt();

        let line = match reader.next_line().await? {
            Some(line) => line,
            None => {
                println!("\n[cli] EOF — guardando y saliendo");
                flush_state(&state).await;
                break;
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let mut parts = trimmed.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let args: Vec<&str> = parts.collect();

        match cmd {
            "add" => handle_add(&state, &args).await,
            "add-spotify" => handle_add_spotify(&state, &args).await,
            "remove" => handle_remove(&state, &args).await,
            "list" => handle_list(&state).await,
            "playlists" => handle_playlists(&state).await,
            "help" | "?" => print_help(),
            "quit" | "exit" => {
                println!("[cli] guardando estado…");
                flush_state(&state).await;
                println!("[cli] hasta luego.");
                break;
            }
            other => eprintln!(
                "[cli] comando desconocido: '{other}' (escribe 'help' para ver la lista)"
            ),
        }
    }

    Ok(())
}

fn print_banner() {
    println!("╔═══════════════════════════════════╗");
    println!("║  SpotiCry server — CLI            ║");
    println!("╚═══════════════════════════════════╝");
}

pub fn print_help() {
    println!("Comandos disponibles:");
    println!("  add <ruta> <spotify-id>  MP3 local + metadata y portada de Spotify");
    println!("  add <ruta>               MP3 local con metadata leída de tags ID3 (fallback)");
    println!("  add-spotify <id>         Solo metadata de Spotify, sin MP3 local");
    println!("  remove <song-id>         Elimina una canción (falla si está sonando)");
    println!("  list                     Lista todas las canciones de la biblioteca");
    println!("  playlists                Lista todas las playlists globales");
    println!("  help                     Muestra esta ayuda");
    println!("  quit                     Guarda estado y sale");
}

fn prompt() {
    print!("\nspoticry> ");
    let _ = io::stdout().flush();
}

// ─── handlers ────────────────────────────────────────────────────────────

async fn handle_add(state: &Arc<AppState>, args: &[&str]) {
    let (path_str, track_id) = match args {
        [p] => (*p, None),
        [p, t] => (*p, Some(*t)),
        _ => {
            eprintln!("Uso: add <ruta-mp3> [spotify-track-id]");
            eprintln!("  con track-id: usa metadata + portada de Spotify (recomendado).");
            eprintln!("  sin track-id: lee metadata de los tags ID3 del MP3.");
            return;
        }
    };

    let path = Path::new(path_str);
    if !path.exists() {
        eprintln!("✗ archivo no encontrado: {path_str}");
        return;
    }

    // Modo recomendado: MP3 local + metadata Spotify (incluida portada).
    if let Some(track_id) = track_id {
        let Some(spotify) = state.spotify.as_ref() else {
            eprintln!(
                "✗ Spotify no configurado. Setea SPOTIFY_CLIENT_ID/SECRET en server/.env, \
                 o usa `add <ruta>` sin track-id para leer tags ID3."
            );
            return;
        };
        match spotify.fetch_track(track_id).await {
            Ok(mut song) => {
                // Combinar: file_path local, todo lo demás de Spotify.
                song.file_path = Some(path_str.to_string());
                let title = song.title.clone();
                let artist = song.artist.clone();
                let id = state.library.write().await.add_song(song);
                println!("✓ agregada (id {id}) desde Spotify: {artist} — {title}");
                println!("    archivo: {path_str}");
            }
            Err(e) => eprintln!("✗ error consultando Spotify: {e}"),
        }
        return;
    }

    // Fallback: solo tags ID3.
    match state.library.write().await.add_song_from_file(path) {
        Ok(id) => println!("✓ agregada (id {id}) desde ID3: {path_str}"),
        Err(e) => eprintln!("✗ error agregando '{path_str}': {e}"),
    }
}

async fn handle_add_spotify(state: &Arc<AppState>, args: &[&str]) {
    if args.len() != 1 {
        eprintln!("Uso: add-spotify <track-id>");
        return;
    }
    let track_id = args[0];
    let Some(spotify) = state.spotify.as_ref() else {
        eprintln!(
            "✗ Spotify no configurado: setea SPOTIFY_CLIENT_ID y SPOTIFY_CLIENT_SECRET y reinicia el server."
        );
        return;
    };
    match spotify.fetch_track(track_id).await {
        Ok(song) => {
            let id = state.library.write().await.add_song(song);
            println!("✓ agregada desde Spotify (id {id}): {track_id}");
        }
        Err(e) => eprintln!("✗ error consultando Spotify: {e}"),
    }
}

async fn handle_remove(state: &Arc<AppState>, args: &[&str]) {
    if args.len() != 1 {
        eprintln!("Uso: remove <song-id>");
        return;
    }
    let id: u64 = match args[0].parse() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("ID inválido: '{}' (debe ser un número entero positivo)", args[0]);
            return;
        }
    };
    // remove_song consulta playback::is_playing y devuelve CANNOT_DELETE_PLAYING
    // si la canción está sonando (requisito explícito del enunciado).
    match state.library.write().await.remove_song(id, &state.playback) {
        Ok(()) => println!("✓ eliminada (id {id})"),
        Err(e) => eprintln!("✗ no se pudo eliminar id {id}: {e}"),
    }
}

async fn handle_list(state: &Arc<AppState>) {
    let lib = state.library.read().await;
    let songs = lib.list();
    if songs.is_empty() {
        println!("(biblioteca vacía)");
        return;
    }
    println!("{:<5} {:<32} {:<24} {:<6}", "ID", "TÍTULO", "ARTISTA", "AÑO");
    for s in &songs {
        println!("{:<5} {:<32} {:<24} {:<6}", s.id, s.title, s.artist, s.year);
    }
}

async fn handle_playlists(state: &Arc<AppState>) {
    let st = state.playlists.read().await;
    if st.playlists.is_empty() {
        println!("(no hay playlists)");
        return;
    }
    for pl in st.playlists.values() {
        println!("[{}] {} — {} canción(es)", pl.id, pl.name, pl.songs.len());
    }
}

// ─── persistencia ────────────────────────────────────────────────────────

/// Vuelca biblioteca + playlists a disco. Llamada en `quit`/`exit` y EOF.
async fn flush_state(state: &Arc<AppState>) {
    let (lib_songs, lib_next_id) = state.library.read().await.to_snapshot();
    let (pls, pl_next_id) = state.playlists.read().await.to_snapshot();

    let lib_snap = LibrarySnapshot { songs: lib_songs, next_id: lib_next_id };
    let pl_snap = PlaylistsSnapshot { playlists: pls, next_id: pl_next_id };

    if let Err(e) = persistence::save_library(&lib_snap).await {
        eprintln!("✗ error guardando library: {e}");
    }
    if let Err(e) = persistence::save_playlists(&pl_snap).await {
        eprintln!("✗ error guardando playlists: {e}");
    }
}

// ─── tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    /// Smoke: comprueba que la lista de comandos en `print_help` está sincronizada
    /// con los `match` arms del loop. Si Kevin agrega un comando nuevo y se le
    /// olvida tocar `print_help`, este test no lo ataja directamente, pero al
    /// menos verifica que la función imprime sin panic.
    #[test]
    fn print_help_no_panic() {
        super::print_help();
    }
}
