# Conclusiones y recomendaciones

## Conclusiones

### Sobre el contraste imperativo vs funcional dentro del mismo sistema

El proyecto demuestra que ambos paradigmas pueden convivir sin que uno
"contamine" al otro. La biblioteca (`library.rs`) usa `HashMap` mutable y
un índice secundario por género que se reconstruye con `let mut` y `for`
loops; las playlists (`playlists/`) usan `im::HashMap` y `im::Vector`,
clonan estado en O(log n) gracias al *structural sharing*, y exponen
funciones puras (`create`, `add_song`, `remove_song`, `filter_songs`,
`sort_by`, `total_duration`).

El pegamento entre los dos mundos está en `ws.rs`. Allí se ve el patrón
**functional core, imperative shell** literalmente como código:

```rust
let next_state = {
    let pl = state.playlists.read().await;
    ops::add_song(&pl, playlist_id, song_id)   // función pura, sin lock
};
{
    let mut pl = state.playlists.write().await;
    *pl = next_state;                          // swap atómico
}
```

El lock se mantiene el menor tiempo posible y la operación de negocio
ocurre fuera del scope del lock. Eso reduce la contención y deja el
módulo funcional limpio de detalles de concurrencia.

### Sobre concurrencia con Tokio

`async/await` permite expresar I/O concurrente con código que se lee como
si fuera secuencial. `Arc<RwLock<…>>` cubre los casos de "muchas lecturas,
pocas escrituras" (la biblioteca rara vez cambia, pero todos los WebSocket
la leen). `tokio::sync::broadcast::channel` resuelve el fan-out: cada
WebSocket se suscribe al canal y recibe los `ServerEvent` que cualquier
otra tarea publique, sin que el publicador conozca quién está suscrito.

Una observación práctica: hay que tener cuidado con sostener un guard de
`RwLock` a través de un `await`. En este proyecto seguimos la convención
de extraer los datos necesarios dentro de un bloque `{ … }` y soltar el
guard antes de llamar a `socket.send`, lo que evita deadlocks.

### Sobre protocolos de red

Elegimos **WebSocket JSON** en lugar del "TCP crudo" del enunciado porque
los navegadores no pueden abrir sockets TCP arbitrarios — un cliente web
puro requiere HTTP o WebSocket. WebSocket es esencialmente TCP con un
handshake HTTP de upgrade, así que conserva la persistencia y el orden de
los mensajes pero suma compatibilidad con el navegador y con la
infraestructura de la web (proxies, TLS automático en Cloudflare Tunnel,
etc.).

Para la transmisión de audio elegimos **HTTP con Range requests**. El
elemento `<audio>` del navegador hace seek nativo emitiendo
automáticamente nuevas peticiones `Range: bytes=X-Y` y el servidor
responde `206 Partial Content`. No tuvimos que escribir código JS para el
buffer ni para el seek — la pila web ya lo trae resuelto.

### Sobre Rust como lenguaje del backend

El compilador atrapa una clase entera de errores que en lenguajes
dinámicos se descubren en producción: usos de variables movidas, datos
compartidos sin sincronización, paths de error sin manejar. La curva de
aprendizaje es real (ownership, borrowing, lifetimes), pero una vez que
el código compila, suele funcionar.

Las crates ecosistema fueron clave: `tokio` + `axum` para el servidor,
`serde` para JSON, `im` para estructuras inmutables, `tower-http`'s
`ServeFile` que ya implementa Range correctamente, `id3` para metadata
de MP3, `rspotify` para Spotify, `dotenvy` para cargar `.env`.

## Dificultades encontradas

- **Curva de Rust** (ownership y lifetimes). Especialmente al combinar
  `async` + locks: tuvimos que aprender el patrón de soltar el guard
  antes de un `await` para no bloquear otras tareas ni provocar
  deadlocks.
- **Resolver merge conflicts** entre las dos personas del equipo cuando
  ambos tocaban `cli.rs` y la rama de Día 2 quería persistir en archivos
  que Persona 2 ya había refactorizado. Se resolvió tomando la estructura
  limpia de `main` y aplicando los TODOs uncovered del módulo de
  persistencia.
- **Bug crítico de persistencia descubierto en Día 3**: las funciones
  `save_library` / `save_playlists` existían pero no estaban siendo
  invocadas desde ningún lado, así que cualquier cambio se perdía al
  reiniciar el servidor. Se corrigió añadiendo un helper
  `cli::flush_state` que se llama tanto en `quit`/`exit` como al recibir
  EOF (`Ctrl-D`), y agregando `load_playlists` al startup en `main.rs`.
- **Spotify Client Credentials Flow** requería leer la documentación de
  `rspotify` con cuidado para entender que esta variante no necesita
  redirección de OAuth ni servidor de callback — basta con `client_id` y
  `client_secret`. La integración quedó como `Option<Arc<SpotifyClient>>`
  para que el servidor arranque igual aunque las credenciales no estén
  disponibles.
- **Variables de entorno**: tener que prefijar `SPOTIFY_CLIENT_ID=… cargo run`
  era engorroso. Lo resolvimos con `dotenvy::dotenv()` al inicio de
  `main.rs` y un `server/.env.example` documentando el formato.
- **`PlaylistFilter` sin handler**: el match arm en `ws.rs` faltaba y la
  variante caía en `_ => {}`, descartando silenciosamente la operación.
  Se descubrió leyendo las warnings de `cargo check` ("function
  `filter_songs` is never used") al final del Día 3.

## Recomendaciones / qué cambiaríamos si fuera producción

- **Autenticación real**: OAuth con Spotify para identificar usuarios y
  asociar playlists privadas a cada cuenta. Hoy todas las playlists son
  globales y compartidas, lo que sirve para la demo de concurrencia pero
  no es viable comercialmente.
- **Persistencia en una base de datos**: PostgreSQL para la biblioteca y
  Redis para la sesión de reproducción y el estado "en reproducción".
  Reescribir `library.json` completo en cada `quit` no escala.
- **CDN para audio con HLS o DASH**: el streaming Range simple funciona
  bien para un único servidor en localhost, pero un servicio real
  necesitaría segmentación, calidad adaptativa y caché en el borde.
- **Multi-región con CRDTs**: hoy las playlists viven en un único
  `Arc<RwLock<…>>` en memoria. Para colaboración en vivo entre regiones
  habría que reemplazar el RwLock por una estructura libre de conflictos
  (CRDT) o un log de eventos con orden total (event sourcing + Kafka).
- **Observabilidad**: integrar OpenTelemetry para trazas distribuidas y
  un dashboard (Grafana, Datadog) que muestre latencia del WS, p50/p99
  del streaming y tasa de error.
- **Deploy automático**: CI/CD con GitHub Actions empujando el frontend
  a Azure SWA y el backend a un servicio gestionado (Fly.io, Railway,
  o Kubernetes con autoscaling). Hoy el backend corre local y se expone
  con Cloudflare Tunnel — funciona para la demo pero la URL del tunnel
  cambia en cada reinicio.
- **Tests de integración** que arranquen el servidor real, abran un
  WebSocket cliente y validen el flujo completo. Hoy solo tenemos tests
  unitarios de `library.rs`.
- **Fixear el `mark_stopped` prematuro en `http_stream.rs`**: el handler
  llama a `mark_stopped` justo después de devolver el `Response`, no
  cuando la conexión realmente se cierra. La verdad es que ahora se
  apoya en el `ClientMsg::Stop` del WebSocket; convendría unificar el
  modelo o documentar explícitamente que el HTTP no es la fuente de
  verdad del estado de reproducción.

## Trabajo futuro

Funcionalidades que quedaron fuera por tiempo y serían el siguiente paso
natural:

- **AdminPanel web**: un panel en el frontend para hacer add/remove sin
  pasar por la CLI. Hoy la CLI cumple el requisito del enunciado pero la
  UX de un usuario no técnico mejoraría mucho con un formulario.
- **Spotify Connect** (reproducción coordinada entre dispositivos): hoy
  cada cliente reproduce de forma independiente.
- **Letras sincronizadas** y *cover art* animado en el reproductor.
- **Colaboración en vivo** sobre una playlist: ver el cursor de otros
  usuarios mientras editan, comentarios por canción.
- **Recomendaciones**: sistema de "similar a esta canción" usando los
  embeddings que expone Spotify Audio Features API.
- **Pruebas end-to-end** con Playwright para el frontend y un harness
  Tokio para el backend.
- **Empaquetado**: ejecutable nativo del servidor distribuible con
  `cargo dist` y app de escritorio del cliente con Tauri.
