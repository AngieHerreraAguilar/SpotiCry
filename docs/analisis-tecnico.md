# Análisis técnico de la solución

> Owner: Persona 1. Llenar en Día 3-4.

## Arquitectura

*Diagrama de alto nivel (ver ROADMAP.md sección "Arquitectura general").*

## Stack

- Backend: Rust + Tokio + Axum
- Frontend: React + Vite + Zustand
- Persistencia: JSON en disco
- Protocolo: WebSocket JSON + HTTP Range

## Decisiones clave y justificación

### WebSocket en vez de TCP crudo

*Justificar: los navegadores no pueden abrir sockets TCP crudos. WebSocket es TCP con
handshake HTTP — el equivalente más cercano y único disponible desde un navegador.*

### Playlists globales compartidas

*Justificar: demuestra concurrencia real, sin auth.*

### Módulo funcional de playlists

*Explicar patrón functional-core / imperative-shell. El módulo `playlists/` no importa
`tokio` ni `std::sync`; las funciones puras devuelven un `State` nuevo. El wiring vive
en `ws.rs` con `Arc<RwLock<State>>` y `*state.write().await = next`.*

### 3 criterios de búsqueda técnicamente distintos

- Título → substring match sobre strings (scan lineal)
- Género → lookup O(1) en `HashMap<Genre, Vec<SongId>>`
- Año → filtro numérico con comparación de rangos

## Administración en memoria

*Explicar la diferencia entre `HashMap` mutable (library) y `im::HashMap` inmutable
(playlists), por qué cada elección en su contexto.*

## Manejo de archivos de música

*Cómo se cargan, decodifican con symphonia, y se transmiten vía HTTP Range.*

## Manejo de conflictos y excepciones

- Canción en reproducción no borrable (playback::is_playing)
- Cliente se desconecta durante stream
- Archivo MP3 corrupto o inexistente
- Spotify API rate-limited o sin credenciales
