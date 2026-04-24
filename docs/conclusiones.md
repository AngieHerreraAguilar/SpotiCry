# Conclusiones y recomendaciones

> Owner: Persona 2. Llenar en Día 4.

## Conclusiones

*Qué se aprendió sobre:*

- Paradigmas imperativo vs funcional (especialmente el contraste entre `library.rs`
  imperativo y `playlists/ops.rs` funcional dentro del mismo servidor).
- Concurrencia en Rust con Tokio: `async/await`, `Arc<RwLock>`, broadcast channels.
- Protocolos de red: por qué WebSocket en vez de TCP crudo en un contexto web.
- Streaming de audio: por qué HTTP Range es suficiente para la UX de seek.

## Dificultades encontradas

*Ejemplos típicos a completar según experiencia real:*

- Curva de aprendizaje de Rust (ownership, lifetimes).
- Configurar `rspotify` con Client Credentials flow.
- Hacer que los breakpoints responsive se vieran bien en mobile.

## Recomendaciones / qué cambiaríamos si fuera producción

- **Auth real** (OAuth con Spotify, tokens firmados).
- **Persistencia en BD** (Postgres o Redis) en vez de JSON local.
- **CDN para audio** con segmentación HLS/DASH.
- **Multi-región** con CRDTs para playlists colaborativas.
- **Observabilidad** (OpenTelemetry, dashboards).
- **Deploy automático** (Kubernetes, autoscaling).

## Trabajo futuro

*Features que quedaron fuera por tiempo: AdminPanel web, Spotify Connect, letras,
colaboración en vivo sobre playlists, etc.*
