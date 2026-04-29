# Concurrencia: con vs sin sincronización

Esta es la pieza clave del enunciado. Documenta qué pasaría sin sincronización, cómo SpotiCry resuelve el problema, qué se observa empíricamente al someterlo a carga multi-cliente, y qué cambiaría en una variante a escala de producción.

## El problema sin sincronización

Suponga que las playlists vivieran como un `HashMap` plano (sin lock, sin estructura inmutable) y cada cliente que recibe un `playlist.add` por WebSocket lo aplicara directamente:

```text
T0  Estado: playlist.songs = [x, y]
T1  Cliente A: leer playlist.songs            → [x, y]
T2  Cliente B: leer playlist.songs            → [x, y]
T3  Cliente A: escribir [x, y, z]             ✓
T4  Cliente B: escribir [x, y, w]             ✗  (sobreescribe T3, z se pierde)
```

Esto es un *read-modify-write race condition* clásico: dos hilos leen la misma versión del estado, calculan sobre ella en paralelo, y cuando ambos escriben el ganador del último write borra al anterior. La cantidad de "z perdidos" crece con el número de pestañas y la frecuencia de escritura.

Sin sincronización también aparecen inconsistencias parciales: si las playlists se serializaran a JSON mientras una mutación está en curso, el archivo en disco podría quedar con una `Vector` truncada. Y la misma broadcast que reenvía `playlist.snapshot` a los demás clientes podría enviar dos veces el "antes" sin enviar el "después".

## La solución aplicada

SpotiCry usa **dos primitivas distintas** según el perfil del estado, ambas envueltas en `Arc` para compartir entre tasks de Tokio.

### Library (imperativa) — `Arc<RwLock<HashMap<SongId, Song>>>`

Lecturas frecuentes (cada `search`, cada `library.list`, cada arranque de un nuevo cliente) y escrituras puntuales (CLI). `RwLock` deja a múltiples lectores entrar a la vez sin bloquearse y solo serializa al escritor. La región crítica de escritura es corta (`HashMap::insert` + actualizar índice de género), no hace `await`, no llama a otros locks → no hay riesgo de deadlock ni de tener un lock retenido a través de I/O.

Trade-off: `RwLock` puede sufrir *writer starvation* si los lectores son extremadamente densos. Para el escenario académico (decenas de operaciones por segundo a lo sumo) es invisible.

### Playlists (funcional) — `Arc<RwLock<im::HashMap<…>>>` + swap atómico

El módulo [`playlists/`](../server/src/playlists) es funcional puro: sus funciones (`create`, `add_song`, `remove_song`, `reorder`) toman `&State` y devuelven un `State` nuevo. Como `im::HashMap` y `im::Vector` son **estructuras inmutables persistentes con structural sharing**, "construir un estado nuevo" no copia todo el árbol — comparte la mayoría de los nodos con la versión anterior y paga solo O(log n) por la "rama" modificada.

El handler en [`ws.rs`](../server/src/ws.rs) compone el patrón:

```rust
// 1. Leer un snapshot inmutable (clone barato gracias a im).
let snapshot = state.playlists.read().await.clone();

// 2. Computación pura, FUERA del lock. Otros lectores siguen entrando.
let next = ops::add_song(&snapshot, pid, sid);

// 3. Swap único bajo write lock.
*state.playlists.write().await = next;
```

Tres garantías que esto da:

1. **Ningún add se pierde**: cada handler aplica su mutación sobre un snapshot exclusivo (entre el `clone` y el swap, otro hilo puede tener su propio snapshot, pero el último swap *gana* construyendo sobre `next`, no sobre `snapshot.before`). Cuando hay dos handlers concurrentes, las write locks se serializan y los cambios se aplican secuencialmente sin perderse.
2. **Sin lock retenido durante el cómputo**: `ops::add_song` puede ejecutarse mientras otros leen.
3. **Atomicidad observable**: no existe un estado público "intermedio". El snapshot que sale por `state.broadcast.send(PlaylistSnapshot{...})` siempre es coherente.

### Playback — `Arc<Mutex<HashSet<SongId>>>`

Aquí no se quiere lecturas concurrentes (las consultas son puntuales) y las escrituras son cortas (`insert`/`remove`). Un `Mutex<HashSet>` simple basta. Ver [`playback.rs`](../server/src/playback.rs).

### Broadcast — `tokio::sync::broadcast::channel`

El fan-out a todos los clientes WS conectados es nativo de Tokio: cada conexión hace `state.broadcast.subscribe()` al entrar; cada mutación termina con `state.broadcast.send(ServerEvent::PlaylistSnapshot{…})`. El channel hace el copy-on-send para todos los receivers; si uno está lento se pierde un mensaje (configurado con buffer 100) pero el resto sigue.

## Procedimiento experimental

Se diseñó la siguiente prueba para someter el sistema al escenario peor-caso del race read-modify-write:

1. Levantar el servidor: `cd server && cargo run`.
2. Cargar 10–20 canciones por CLI (mezcla `add <ruta>` + `add-spotify <id>`).
3. Levantar el frontend: `cd client && npm run dev`.
4. Abrir 3 pestañas en `http://localhost:5173`.
5. Desde la pestaña #1, crear una playlist vacía llamada "concurrency-test".
6. Esperar 1 s a que el `PlaylistSnapshot` llegue a las 3 pestañas.
7. En cada una de las 3 pestañas, hacer `addSong(playlist_id, song_X_distinta)` en ráfaga (~10 adds por pestaña, en menos de 2 s en total).
8. Cerrar la ráfaga. Esperar 1 s.
9. Comparar:
   - Total esperado: 30 canciones en la playlist.
   - Total observado en la UI de cada pestaña tras el último broadcast.
   - Total observado en `playlists.json` tras el debounce de 500 ms.

Una segunda prueba valida la regla "no borrar canción reproduciéndose":

1. Reproducir una canción desde la pestaña #1.
2. Desde la CLI del servidor, `remove <song_id>` → debe rechazar con `✗ no se pudo eliminar id N: CANNOT_DELETE_PLAYING`.
3. Pausar la canción (botón pause en el reproductor) → enviar `stop` por WS.
4. Repetir `remove` → debe pasar.

Una tercera prueba valida el cleanup al cerrar pestaña sin avisar:

1. Reproducir una canción.
2. Cerrar la pestaña sin pausar.
3. Desde la CLI, `remove <song_id>` → debe pasar (porque el `handle_socket` libera `owned_playing` al hacer `break`).

## Resultado esperado por construcción

Por las garantías del swap atómico:

- **Adds no se pierden**: el total final debe ser exactamente `n_pestañas × adds_por_pestaña` (en este caso, 30).
- **Broadcast es total-order**: las 3 pestañas convergen al mismo conteo final, aunque el orden de los `PlaylistSnapshot` recibidos pueda variar.
- **`playlists.json` es coherente**: al volcar tras el debounce, captura exactamente el estado final en memoria. No quedan "snapshots intermedios" persistidos.

Si una sola canción se perdiera, el modelo de concurrencia estaría roto y habría que revisar el patrón de swap.

## Evidencia recolectada

> Anotar aquí los conteos exactos al correr el procedimiento. Plantilla:

| Prueba | Esperado | Observado | Resultado |
|---|---|---|---|
| Multi-pestaña × 3 × 10 adds | 30 canciones | _(pendiente al ejecutar)_ | _(pendiente)_ |
| Remove durante reproducción | Error `CANNOT_DELETE_PLAYING` | _(pendiente)_ | _(pendiente)_ |
| Remove tras stop | OK | _(pendiente)_ | _(pendiente)_ |
| Cleanup al cerrar pestaña | OK tras cierre | _(pendiente)_ | _(pendiente)_ |

Notas observacionales (latencias percibidas, tamaño de `playlists.json` antes y después, carga de CPU del servidor) van en [`resultados.md`](resultados.md), donde también se anclan las capturas de pantalla.

## Reflexión para producción a gran escala

Lo que hoy resuelven `Arc<RwLock<im::HashMap<...>>>` + `tokio::sync::broadcast` se rompería bajo varias dimensiones cuando la carga deja de ser académica:

### Almacenamiento

- `library.json` y `playlists.json` no escalan: una sola playlist con un millón de canciones convierte cada flush en una escritura de varios MB. La forma natural es **Postgres** para library (queries SQL para los 3 criterios de búsqueda con índices apropiados, `tsvector` para fulltext en título) y **un sistema de eventos** para playlists (event sourcing: cada add es un evento append-only; el estado se proyecta).
- Los MP3 deben vivir en **S3 o equivalente**, no en disco local. El servidor solo emite URLs firmadas con TTL corto. La regla "no borrar reproduciéndose" se hace más sutil: el server no controla cuándo el cliente termina de leer del CDN, así que el lock pasaría a ser por *intención* (sigue habiendo un trabajo de "marcar playing/stopped") y se complementaría con un *hold* sobre el objeto en S3 si el dueño quiere borrar.

### Concurrencia distribuida

- Un solo proceso Rust con `RwLock` no escala horizontalmente. Para multi-instancia se necesita o bien un store con concurrencia propia (Redis con scripts Lua para read-modify-write atómico, Postgres con `SELECT FOR UPDATE`) o un protocolo CRDT en cada réplica si se quiere edición offline-first.
- Con CRDTs (por ejemplo, ORSet o RGA para listas) cada réplica puede aceptar adds sin coordinación y la convergencia es matemáticamente garantizada al merge — esto se aproxima a cómo funcionan Figma y Notion.

### Auth y multi-usuario

- Hoy las playlists son globales. En producción cada playlist tendría un dueño y un set de colaboradores con permisos. La autenticación se haría con **OIDC** (e.g. Spotify OAuth real, Auth0) y cada conexión WS llevaría un JWT firmado.
- Las búsquedas por género dejarían de tener un solo índice global y se segmentarían por tenant.

### Streaming

- HTTP Range plano pierde frente a **HLS o DASH**: el cliente solicita segmentos cortos, hay manifest, hay adaptive bitrate. Bajo conexiones móviles inestables esto reduce buffering.
- Para audio comercial se sumaría DRM (Widevine, FairPlay), claves rotativas, marcas digitales por sesión.

### Observabilidad

- En lugar de `tracing` con `EnvFilter`, se exportaría a **OpenTelemetry** con dashboards de latencia por op (`playlist.add`, `play`, `/stream/:id`), tasas de error por código (`CANNOT_DELETE_PLAYING`, `NOT_FOUND`), y métricas de saturación del broadcast channel.
- La regla "no borrar reproduciéndose" se vuelve testeable en producción midiendo cuántos `remove` reciben `CANNOT_DELETE_PLAYING` por hora — un buen *liveness signal* sin tener que abrir la app.

### Despliegue

- Hoy el servidor es un binario en localhost. En producción iría en **Kubernetes** con un Deployment + autoscaler por número de conexiones WebSocket activas. Las sesiones WS son sticky por nodo (sticky sessions o un pubsub global Redis para reenviar entre instancias).
- El frontend correría en un CDN (Cloudflare, Vercel) con build cacheable y no por servidor.

En síntesis: el patrón **functional-core / imperative-shell** no se reemplaza, se generaliza. La parte funcional pura sigue siendo el motor; lo que cambia es cómo se persiste el estado, cómo se replica entre instancias, y cómo se autoriza cada operación.
