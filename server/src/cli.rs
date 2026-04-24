// Interfaz de texto del servidor (CLI). Owner: Persona 2.
//
// Corre en una task paralela (tokio::spawn). Cumple el requerimiento:
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
//   quit                  — flushea persistencia y sale
//
// Este archivo NO usa async/await — lee de stdin bloqueante en una task separada.
// Comunica con el core vía `tokio::sync::mpsc` o un handle al AppState.

use std::io::{self, BufRead, Write};

pub fn print_help() {
    println!("Commands:");
    println!("  add <path>            Add an MP3 file (reads ID3 tags)");
    println!("  add-spotify <id>      Add from Spotify track id (metadata + preview only)");
    println!("  remove <song-id>      Remove a song (fails if currently playing)");
    println!("  list                  List all songs in the library");
    println!("  playlists             List all playlists");
    println!("  help                  Show this help");
    println!("  quit                  Save and exit");
}

// TODO(Persona 2): pub fn run(state: Arc<AppState>)
//   bucle que lee stdin línea por línea, parsea el comando, y delega al módulo correspondiente.
//   usar clap o parseo manual por primera palabra.

#[allow(dead_code)]
fn _silence_unused() {
    let _ = io::stdin();
    let _ = io::stdout();
    let _: fn(&str) -> Option<Vec<String>> = |s| Some(s.split_whitespace().map(String::from).collect());
    let _: fn() -> io::Result<String> = || {
        let stdin = io::stdin();
        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        Ok(line)
    };
    let _: fn() -> io::Result<()> = || {
        let mut out = io::stdout();
        out.flush()
    };
}
