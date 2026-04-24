# Resultados obtenidos

> Owner: Persona 2. Llenar en Día 4.

## Funcionalidades implementadas (MVP)

- [ ] Backend Rust con Tokio + Axum
- [ ] 3 criterios de búsqueda técnicamente distintos (título, género, año)
- [ ] CLI del servidor (add/remove/list)
- [ ] Persistencia JSON (library + playlists)
- [ ] Módulo funcional de playlists (immutable, map/filter/fold, closures)
- [ ] Regla "no se puede borrar canción en reproducción"
- [ ] Streaming HTTP con Range requests
- [ ] Integración Spotify API (metadata + preview fallback)
- [ ] Frontend React con buscador, reproductor y playlists
- [ ] Buffer local de seek en el reproductor
- [ ] Responsive desktop + mobile

## Screenshots

*Incluir capturas de la app funcionando:*

- Pantalla principal (biblioteca)
- Búsqueda por los 3 criterios
- Vista de playlist con ordenamiento
- Reproductor con seek
- Versión mobile

## Métricas

*Si aplica: tiempo de respuesta del WS, latencia de seek, uso de memoria, etc.*

## Manejo de archivos de música

*Flujo completo: MP3 → ID3 tags → library → library.json → stream endpoint → navegador.*
