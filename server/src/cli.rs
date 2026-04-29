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
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader, Lines, Stdin};

use crate::AppState;
use crate::domain::Song;
use crate::library_scan::{
    extract_spotify_id, read_id3_song, save_tracks_map, PendingTrack, TrackEntry, TracksMap,
};
use crate::persistence::{self, LibrarySnapshot, PlaylistsSnapshot};
use crate::protocol::ServerEvent;

pub async fn run(
    state: Arc<AppState>,
    pending: Vec<PendingTrack>,
    tracks_path: PathBuf,
    mut tracks_map: TracksMap,
) -> anyhow::Result<()> {
    print_banner();

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();

    // 🔹 Si el scan dejó canciones pendientes, preguntarle al usuario antes
    //    de mostrar el prompt normal. La elección se persiste en tracks.json
    //    para que el próximo arranque sea automático.
    if !pending.is_empty() {
        setup_pending(&state, pending, &tracks_path, &mut tracks_map, &mut reader).await;
    }

    print_help();

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
                let _ = state.save_tx.try_send(());
                broadcast_library(state).await;
            }
            Err(e) => eprintln!("✗ error consultando Spotify: {e}"),
        }
        return;
    }

    // Fallback: solo tags ID3.
    match state.library.write().await.add_song_from_file(path) {
        Ok(id) => {
            println!("✓ agregada (id {id}) desde ID3: {path_str}");
            let _ = state.save_tx.try_send(());
            broadcast_library(state).await;
        }
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
            let _ = state.save_tx.try_send(());
            broadcast_library(state).await;
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
        Ok(()) => {
            println!("✓ eliminada (id {id})");
            let _ = state.save_tx.try_send(());
            broadcast_library(state).await;
        }
        Err(e) => eprintln!("✗ no se pudo eliminar id {id}: {e}"),
    }
}

/// Reenvía un `LibrarySnapshot` actualizado a todos los clientes WS conectados.
/// Llamado tras add/remove desde la CLI para que la UI se refresque sin recargar.
async fn broadcast_library(state: &Arc<AppState>) {
    let songs = state.library.read().await.list();
    let _ = state.broadcast.send(ServerEvent::LibrarySnapshot { songs });
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

// ─── setup interactivo de canciones nuevas ──────────────────────────────

/// Procesa los MP3s pendientes del scan inicial preguntándole al usuario
/// cómo cargarlos. Persiste cada elección en `tracks.json` para que los
/// próximos arranques sean 100 % automáticos.
async fn setup_pending(
    state: &Arc<AppState>,
    pending: Vec<PendingTrack>,
    tracks_path: &Path,
    tracks_map: &mut TracksMap,
    reader: &mut Lines<BufReader<Stdin>>,
) {
    println!(
        "\n🔍 Encontré {} canción(es) nueva(s) en library/. Te pregunto cómo cargar cada una:\n",
        pending.len()
    );

    for track in pending {
        println!("📀 {}", track.filename);
        println!("   [s] Pegar enlace de Spotify (recomendado)");
        println!("   [m] Escribir metadata a mano");
        println!("   [t] Leer tags ID3 del archivo");
        println!("   [x] Saltar (te pregunto de nuevo en el próximo arranque)");
        print!("   > ");
        let _ = io::stdout().flush();

        let choice = match reader.next_line().await {
            Ok(Some(line)) => line.trim().to_lowercase(),
            _ => return,
        };

        let result = match choice.as_str() {
            "s" => prompt_spotify(state, &track, reader).await,
            "m" => prompt_manual(state, &track, reader).await,
            "t" => add_via_id3(state, &track).await,
            "x" => {
                println!("   → saltada\n");
                Some(TrackEntry::Skip)
            }
            _ => {
                println!("   ✗ opción inválida, salto\n");
                None
            }
        };

        if let Some(entry) = result {
            tracks_map.insert(track.filename.clone(), entry);
            if let Err(e) = save_tracks_map(tracks_path, tracks_map) {
                eprintln!("   ⚠ no se pudo guardar tracks.json: {e}");
            }
        }
    }

    // Notificar a los clientes (si alguno se conectó durante el setup) que la
    // biblioteca cambió. Disparar también el debouncer de persistencia.
    let songs = state.library.read().await.list();
    let _ = state.broadcast.send(ServerEvent::LibrarySnapshot { songs });
    let _ = state.save_tx.try_send(());

    println!("✓ setup completo. tracks.json actualizado.\n");
}

async fn prompt_spotify(
    state: &Arc<AppState>,
    track: &PendingTrack,
    reader: &mut Lines<BufReader<Stdin>>,
) -> Option<TrackEntry> {
    let Some(spotify) = state.spotify.as_ref() else {
        eprintln!("   ✗ Spotify no configurado (revisa SPOTIFY_CLIENT_ID/SECRET en server/.env)\n");
        return None;
    };

    print!("   Pega la URL o ID de Spotify: ");
    let _ = io::stdout().flush();
    let line = match reader.next_line().await {
        Ok(Some(l)) => l,
        _ => return None,
    };

    let id = match extract_spotify_id(&line) {
        Some(id) => id,
        None => {
            eprintln!("   ✗ entrada vacía\n");
            return None;
        }
    };

    match spotify.fetch_track(&id).await {
        Ok(mut song) => {
            song.file_path = Some(track.abs_path.to_string_lossy().to_string());
            let title = song.title.clone();
            let artist = song.artist.clone();
            let new_id = state.library.write().await.add_song(song);
            println!("   ✓ agregada (id {new_id}) {artist} — {title}\n");
            Some(TrackEntry::Spotify { spotify_id: id })
        }
        Err(e) => {
            eprintln!("   ✗ Spotify rechazó '{id}': {e}\n");
            None
        }
    }
}

async fn prompt_manual(
    state: &Arc<AppState>,
    track: &PendingTrack,
    reader: &mut Lines<BufReader<Stdin>>,
) -> Option<TrackEntry> {
    let title = ask(reader, "   Título: ").await?;
    let artist = ask(reader, "   Artista: ").await?;
    let album = ask(reader, "   Álbum: ").await?;
    let genre = ask(reader, "   Género: ").await?;
    let year_raw = ask(reader, "   Año (ej. 2024): ").await?;
    let year: u16 = year_raw.parse().unwrap_or(0);

    let song = Song {
        id: 0,
        title: title.clone(),
        artist: artist.clone(),
        album: album.clone(),
        genre: genre.clone(),
        year,
        duration_secs: 0,
        file_path: Some(track.abs_path.to_string_lossy().to_string()),
        spotify_preview_url: None,
        cover_url: None,
    };

    let new_id = state.library.write().await.add_song(song);
    println!("   ✓ agregada (id {new_id}) {artist} — {title}\n");

    Some(TrackEntry::Manual {
        title,
        artist,
        album,
        genre,
        year,
        duration_secs: 0,
    })
}

async fn add_via_id3(state: &Arc<AppState>, track: &PendingTrack) -> Option<TrackEntry> {
    match read_id3_song(&track.abs_path) {
        Ok(song) => {
            let title = song.title.clone();
            let artist = song.artist.clone();
            let new_id = state.library.write().await.add_song(song);
            println!("   ✓ agregada (id {new_id}) {artist} — {title} [via ID3]\n");
            Some(TrackEntry::Id3)
        }
        Err(e) => {
            eprintln!("   ✗ no se pudieron leer tags ID3: {e}\n");
            None
        }
    }
}

async fn ask(reader: &mut Lines<BufReader<Stdin>>, prompt: &str) -> Option<String> {
    print!("{prompt}");
    let _ = io::stdout().flush();
    match reader.next_line().await {
        Ok(Some(line)) => Some(line.trim().to_string()),
        _ => None,
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
