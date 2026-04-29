# Descripción del problema

## Contexto

SpotiCry es un proyecto académico desarrollado para la asignatura de Lenguajes
de Programación. El curso enfatiza el contraste entre dos paradigmas de
programación dentro de un mismo sistema: **imperativo** (con estado mutable
controlado por locks) y **funcional puro** (estructuras inmutables, funciones
sin efectos, transformaciones con `map`/`filter`/`fold` y closures).

A nivel arquitectónico, el enunciado pide además: (1) concurrencia real con
sockets, (2) transmisión de bytes de audio entre procesos, (3) persistencia
en disco y (4) documentación que compare la operación del sistema "con vs sin
sincronización".

El equipo está formado por dos personas: **Persona 1** trabaja exclusivamente
en el backend Rust, y **Persona 2** lidera el frontend más dos módulos de
apoyo del backend (`cli.rs` y `protocol.rs`) y la documentación narrativa.

## Problema

Construir una aplicación tipo Spotify con dos componentes independientes que
se comunican por red:

- Un **servidor en Rust** que mantiene la biblioteca de canciones, las
  playlists compartidas, el estado de reproducción y sirve los archivos de
  audio por streaming.
- Un **cliente web** (React) que consume ese servidor para buscar, reproducir
  con seek (adelantar/retroceder), crear playlists y ver el catálogo en
  tiempo real, incluyendo cambios hechos por otros usuarios.

El sistema debe demostrar de forma explícita el patrón **functional core,
imperative shell**: el módulo de playlists es funcional puro (sin `let mut`,
sin locks, con `im::HashMap`/`im::Vector`), mientras que el resto del backend
(library, playback, persistencia) es imperativo y se sincroniza con
`Arc<RwLock<…>>` y `tokio::sync::broadcast`.

## Objetivos

### Objetivo general
Diseñar e implementar una aplicación cliente–servidor que reproduzca audio de
forma concurrente, mostrando explícitamente el contraste entre programación
imperativa y funcional dentro del mismo proyecto.

### Objetivos específicos

**Backend (Rust):**
- Modelar `Song`, `Playlist` y el estado global compartido (`AppState`).
- Implementar **tres criterios de búsqueda técnicamente distintos**: título
  (substring), género (lookup en `HashMap<Genre, Vec<SongId>>`) y rango de
  años (filtro numérico).
- Cumplir la regla del enunciado **"no se puede eliminar una canción que está
  sonando"** consultando un `Playback` global desde `library::remove_song`.
- Implementar el módulo `playlists/` en estilo **funcional puro**.
- Servir audio por **HTTP Range** (`206 Partial Content`) para que el `<audio>`
  del navegador haga seek nativo.
- Persistir biblioteca y playlists en JSON al disco entre reinicios.
- Integrar la **API de Spotify** (Client Credentials Flow) para enriquecer
  metadata y permitir agregar canciones por `track-id`.

**Frontend (React + Vite):**
- Cliente **WebSocket** con reconexión que dispara comandos al servidor
  (`sendCmd(op, payload)`).
- Reproductor `<audio>` con seek libre apoyado en HTTP Range.
- Vistas para biblioteca, búsqueda, playlists e indicador de "en reproducción".
- **Responsive** con breakpoint a 768 px (sidebar → bottom nav en móvil).

**Concurrencia y robustez:**
- Sincronización fan-out vía `broadcast::channel` para que cualquier mutación
  de playlist llegue a todos los clientes conectados.
- Demostrar con una prueba multi-pestaña que ninguna mutación se pierde
  cuando varios clientes escriben sobre la misma playlist simultáneamente.

**Documentación:**
- Cinco documentos siguiendo el formato pedido por el curso:
  `problema.md`, `analisis-tecnico.md`, `resultados.md`, `conclusiones.md`,
  `concurrencia.md`, más `protocolo.md` con el contrato de mensajes congelado
  desde el Día 1.

## Alcance

### Sí está dentro del alcance

- Servidor Rust con WebSocket (puerto `8080`, ruta `/ws`) y HTTP streaming
  (`/stream/:id`).
- Frontend React servido por Vite (`localhost:5173` en desarrollo).
- Persistencia en archivos JSON locales (`library.json`, `playlists.json`).
- Spotify **solo para metadata + preview de 30 s** — el audio principal son
  archivos MP3 locales (decisión avalada por el profesor).
- Playlists **globales y compartidas** entre todos los clientes.
- CLI del servidor como interfaz administrativa de texto.
- Tres criterios de búsqueda con algoritmos distintos.
- Tests unitarios del invariante "cannot delete playing song".

### NO está dentro del alcance

- **Autenticación de usuarios**: sin login, sin sesiones, sin tokens propios.
  La decisión es deliberada: las playlists globales compartidas maximizan el
  material a documentar sobre concurrencia, mientras que un sistema de auth
  añade complejidad ortogonal a los objetivos del curso.
- **Base de datos relacional o key-value**: la persistencia es JSON plano
  (`library.json`, `playlists.json`) por ser el mecanismo más simple que aún
  permite ilustrar el ciclo "leer al arrancar / escribir tras mutación con
  debounce" sin oscurecer el problema con configuración de un DBMS.
- **Reproducción simultánea coordinada entre clientes** ("Spotify Connect"):
  no hay un protocolo que sincronice posición de reproducción entre pestañas;
  cada cliente reproduce localmente con su propio `<audio>`.
- **CDN / segmentación HLS o DASH**: el streaming es HTTP Range simple, que
  cumple el "envío de paquetes de bytes" del enunciado sin pedir infraestructura
  externa de distribución.
- **Letras sincronizadas, recomendaciones, radio, álbumes premium**: ninguna
  de estas funciones aporta a los objetivos pedagógicos del proyecto.
- **Ecualizador, efectos de audio o procesamiento DSP**: el `<audio>` del
  navegador se usa tal cual; no hay procesamiento de señal en el servidor.
- **Despliegue automático en producción** (Azure Static Web Apps, Cloudflare
  Tunnel, etc.): la entrega corre en `localhost`. Un screencast acompaña la
  demo si la red del jurado lo requiere.

### Restricciones técnicas explícitas del enunciado

- El módulo `playlists/` **no puede** usar `let mut`, `tokio` ni primitivas
  de sincronización: solo estructuras inmutables y combinadores funcionales.
- La regla "no borrar canción reproduciéndose" se valida en backend, no en
  el cliente — el frontend nunca es la única línea de defensa.
- El protocolo WebSocket queda **congelado al final del Día 1** y cualquier
  cambio posterior requiere acuerdo entre las dos personas del equipo.
