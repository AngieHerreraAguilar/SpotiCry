# Concurrencia: con vs sin sincronización

> Owner: Persona 1. Pieza clave del enunciado. Llenar en Día 3-4 con evidencia de las
> pruebas multi-pestaña del Día 4.

## El problema sin sincronización

*Dos clientes agregan simultáneamente una canción distinta a la misma playlist:*

```
Cliente A leer playlist.songs → [x, y]
Cliente B leer playlist.songs → [x, y]
Cliente A escribir [x, y, z]
Cliente B escribir [x, y, w]   ← sobreescribe: z se pierde
```

*Este es un read-modify-write race condition clásico.*

## La solución aplicada

### Para library (imperativo): `Arc<RwLock<HashMap<...>>>`

*Explicar: lecturas concurrentes, escrituras exclusivas, trade-offs.*

### Para playlists (funcional): `im::HashMap` + swap atómico

*Explicar: snapshot inmutable, pure computation fuera del lock, swap al final.*

```rust
let snapshot = shared.read().await.clone();   // O(1) gracias a im
let next = playlists::add_song(&snapshot, pid, sid);  // pura
*shared.write().await = next;                 // swap único
```

*Ventaja: durante la computación de `add_song`, otros clientes pueden leer el estado
anterior sin bloquearse.*

## Evidencia experimental

*Pegar aquí resultados de la prueba multi-pestaña del Día 4: abrir 3 navegadores,
agregar canciones en simultáneo, verificar que ninguna se pierde.*

## Reflexión para producción a gran escala

*Qué cambiaría si esto fuera una app real para miles de usuarios:*

- **Almacenamiento**: Redis para playlists hot, Postgres para persistencia, S3 para audio
- **Concurrencia**: event sourcing + CRDTs para multi-region
- **Auth**: OAuth con Spotify real, tokens firmados, no client IDs anónimos
- **Streaming**: CDN con origin pull, segmentación HLS/DASH, DRM si es comercial
- **Observabilidad**: OpenTelemetry, dashboards de latencia por ruta, alertas
- **Despliegue**: Kubernetes, autoscaling por CPU/conexiones WS
