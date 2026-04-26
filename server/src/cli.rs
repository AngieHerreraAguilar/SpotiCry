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
//
// Estado del wiring real con library/playlists/persistence:
// los handlers tienen comentados, en cada caso, la llamada exacta que Persona 1
// tiene que destapar cuando sus métodos estén implementados. Hoy compilan como
// stubs (`println!("[TODO] ...")`) para que main.rs no se rompa.

use std::io::{self, Write};
use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, BufReader};

use crate::AppState;

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
                // TODO(Persona 1): persistir biblioteca + playlists antes de salir.
                //   crate::persistence::save_library(&*state.library.read().await)?;
                //   crate::persistence::save_playlists(&*state.playlists.read().await)?;
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
    println!("  add <ruta-mp3>        Agrega una canción desde archivo (lee tags ID3)");
    println!("  add-spotify <id>      Agrega desde Spotify (solo metadata + preview)");
    println!("  remove <song-id>      Elimina una canción (falla si está sonando)");
    println!("  list                  Lista todas las canciones de la biblioteca");
    println!("  playlists             Lista todas las playlists globales");
    println!("  help                  Muestra esta ayuda");
    println!("  quit                  Guarda estado y sale");
}

fn prompt() {
    print!("\nspoticry> ");
    let _ = io::stdout().flush();
}

// ─── handlers ────────────────────────────────────────────────────────────

async fn handle_add(state: &Arc<AppState>, args: &[&str]) {
    if args.len() != 1 {
        eprintln!("Uso: add <ruta-mp3>");
        return;
    }
    let path = args[0];
    let _ = state;
    // TODO(Persona 1): destapar cuando library.rs esté listo
    //   use std::path::Path;
    //   match state.library.write().await.add_song_from_file(Path::new(path)) {
    //       Ok(id) => println!("✓ agregada (id {id}): {path}"),
    //       Err(e) => eprintln!("✗ error agregando '{path}': {e}"),
    //   }
    println!("[TODO] add {path}");
}

async fn handle_add_spotify(state: &Arc<AppState>, args: &[&str]) {
    if args.len() != 1 {
        eprintln!("Uso: add-spotify <track-id>");
        return;
    }
    let track_id = args[0];
    let _ = state;
    // TODO(Persona 1): destapar cuando spotify.rs + library.rs estén listos
    //   match state.spotify.fetch_track(track_id).await {
    //       Ok(song) => {
    //           let id = state.library.write().await.add_song(song);
    //           println!("✓ agregada desde Spotify (id {id}): {track_id}");
    //       }
    //       Err(e) => eprintln!("✗ error consultando Spotify: {e}"),
    //   }
    println!("[TODO] add-spotify {track_id}");
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
    let _ = state;
    // TODO(Persona 1): destapar cuando library.rs::remove_song esté listo
    //   match state.library.write().await.remove_song(id) {
    //       Ok(()) => println!("✓ eliminada (id {id})"),
    //       Err(e) => eprintln!("✗ no se pudo eliminar id {id}: {e}"),
    //   }
    // Nota: remove_song debe consultar playback::is_playing(id) y fallar con
    //       CANNOT_DELETE_PLAYING si la canción está en reproducción
    //       (requisito explícito del enunciado).
    println!("[TODO] remove {id}");
}

async fn handle_list(state: &Arc<AppState>) {
    let _ = state;
    // TODO(Persona 1): destapar cuando library.rs::list esté listo
    //   let lib = state.library.read().await;
    //   let songs = lib.list();
    //   if songs.is_empty() {
    //       println!("(biblioteca vacía)");
    //       return;
    //   }
    //   println!("{:<5} {:<32} {:<24} {:<6}", "ID", "TÍTULO", "ARTISTA", "AÑO");
    //   for s in &songs {
    //       println!("{:<5} {:<32} {:<24} {:<6}", s.id, s.title, s.artist, s.year);
    //   }
    println!("[TODO] list");
}

async fn handle_playlists(state: &Arc<AppState>) {
    let _ = state;
    // TODO(Persona 1): destapar cuando playlists/state.rs esté listo
    //   let st = state.playlists.read().await;
    //   if st.playlists.is_empty() {
    //       println!("(no hay playlists)");
    //       return;
    //   }
    //   for pl in st.playlists.values() {
    //       println!("[{}] {} — {} canción(es)", pl.id, pl.name, pl.songs.len());
    //   }
    println!("[TODO] playlists");
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
