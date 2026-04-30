# SpotiCry

Aplicación web tipo Spotify con backend en **Rust** (Tokio + Axum) y frontend en **React**.
Proyecto académico — Lenguajes de Programación, Instituto Tecnológico de Costa Rica.

> 📖 Documentación técnica completa: [`docs/documentacion.md`](docs/documentacion.md) (español) · [`docs/documentation.en.md`](docs/documentation.en.md) (English).

---

## ¿Qué hace?

- 🎵 **Biblioteca de canciones** con tres búsquedas técnicamente distintas: título (substring), género (índice O(1)), año (rango).
- 🔊 **Reproductor en el navegador** con seek libre, ±10 s, pausa/play, vía HTTP Range.
- 📂 **Playlists globales** compartidas entre todos los clientes en tiempo real (módulo funcional puro con `im::HashMap` + `im::Vector`).
- 🌐 **Tiempo real** vía WebSocket: snapshot inicial automático + broadcast de mutaciones.
- 💾 **Persistencia** en `library.json` y `playlists.json` con debounce de 500 ms.
- 🎧 **Integración Spotify** (Client Credentials) para metadata y portadas oficiales.
- 📱 **Responsive** desktop + móvil (breakpoint 768 px).

---

## 👨‍🏫 Para el profesor / revisor

Pasos exactos para clonar y ver la demo funcionando. **No necesitas conseguir música ni buscar IDs de Spotify** — el repo trae cargadas 5 canciones con sus IDs de Spotify ya guardados en [`library/tracks.json`](library/tracks.json). Lo único que tenés que hacer es pegar las credenciales de Spotify que te compartí por privado.

### 1. Requisitos en tu máquina

- **Rust + Cargo** ≥ 1.75 ([rustup.rs](https://rustup.rs))
- **Node.js** ≥ 20 + **npm** ≥ 10 ([nodejs.org](https://nodejs.org))

### 2. Clonar y configurar credenciales

```bash
git clone git@github.com:AngieHerreraAguilar/SpotiCry.git
cd SpotiCry
```

Crea `server/.env` (en la carpeta `server/`, no en la raíz) y pega las dos variables que te compartí por privado:

```env
SPOTIFY_CLIENT_ID=…
SPOTIFY_CLIENT_SECRET=…
```

> ⚠ **Si estás en Windows** y abres el archivo con Notepad, asegúrate de guardarlo con line endings **LF** y no **CRLF** — un `\r` invisible al final del valor rompe la autenticación con Spotify. La forma fácil: abrir con VS Code, mirar la esquina inferior derecha, click en `CRLF` → `LF`, guardar.

### 3. Instalar dependencias

```bash
# Backend
cd server
cargo build          # primera compilación, ~1–2 min

# Frontend (en otra terminal)
cd client
npm install          # ~30 s
```

### 4. Primer arranque del server (paso especial — leer)

La biblioteca cargada en `server/library.json` apunta a las rutas absolutas de la máquina donde se generó. En **tu** máquina esos paths no existen, así que el server las limpia silenciosamente la primera vez. Después el auto-scan vuelve a cargar las 5 canciones usando los IDs de Spotify del manifest, esta vez con tus paths locales.

```bash
cd server
cargo run
```

Lo que vas a ver:

1. `✓ Spotify conectado (Client Credentials)` — confirma que tu `.env` se cargó bien. Si dice `Spotify deshabilitado`, revisa el archivo (line endings, espacios extra, archivo en la carpeta correcta).
2. `✓ limpiadas 5 canción(es) con file_path inexistente` — esperado, son los paths viejos.
3. `🔍 Encontré 5 canción(es) nueva(s) en library/` seguido de 5 líneas `✓ scan: cargada (id N) Artista — Título` — el manifest tiene las elecciones guardadas, así que carga automático sin preguntarte nada.
4. `🚀 Server running on http://localhost:8080` — listo.

### 5. Levantar el frontend

En otra terminal:

```bash
cd client
npm run dev
```

Abre [http://localhost:5173](http://localhost:5173) y ya deberías ver las 5 canciones con sus portadas oficiales.

### 6. Qué probar

| Funcionalidad | Cómo |
|---|---|
| Reproducir + seek libre | Doble click en una canción · arrastra la barra de progreso |
| Saltar ±10 s | Botones `« 10s` / `10s »` del reproductor |
| 3 búsquedas distintas | Sidebar → `Buscar` → tabs Título / Género / Año |
| Crear playlist | Sidebar → `+ Nueva playlist` |
| Agregar canción a playlist | Botón `+` en cada fila de la biblioteca |
| Concurrencia (broadcast) | Abre **2 ventanas** en localhost:5173 — crea/edita en una, ves el cambio en la otra |
| "No borrar canción reproduciéndose" | Reproduce una → en la terminal del server: `remove <id>` → debe rechazar con `CANNOT_DELETE_PLAYING` |
| Tests del backend | `cd server && cargo test` (5 tests verdes en < 1 s) |

### 7. Documentación técnica

- 📄 [`docs/documentacion.md`](docs/documentacion.md) — versión completa en español
- 📄 [`docs/documentation.en.md`](docs/documentation.en.md) — English version

Cubren: problema, funcionalidades, análisis técnico, protocolo, concurrencia con vs sin sincronización, métricas y conclusiones.

---

## Requisitos

| Herramienta | Versión mínima |
|---|---|
| Rust + Cargo | 1.75 (edition 2021) |
| Node.js | 20.x |
| npm | 10.x |

Opcional: una app registrada en el [Spotify Developer Dashboard](https://developer.spotify.com/dashboard) para obtener `Client ID` y `Client Secret`. Sin esto, el server arranca igual pero los comandos `add-spotify` y `add <ruta> <id>` quedan deshabilitados (puedes seguir usando `add <ruta>` con tags ID3).

---

## Setup inicial

```bash
# 1. Clonar
git clone <url-del-repo>
cd SpotiCry

# 2. Backend — instalar dependencias
cd server
cp .env.example .env       # si no existe, créalo (ver siguiente bloque)

# 3. Frontend — instalar dependencias
cd ../client
npm install
```

### Configurar Spotify (opcional pero recomendado)

Crea `server/.env` con:

```env
SPOTIFY_CLIENT_ID=tu_client_id
SPOTIFY_CLIENT_SECRET=tu_client_secret
```

### Configurar URLs del frontend (solo si cambias puertos)

Por defecto el cliente se conecta a `ws://localhost:8080/ws` y `http://localhost:8080`. Si lo necesitas distinto, crea `client/.env`:

```env
VITE_WS_URL=ws://localhost:8080/ws
VITE_HTTP_URL=http://localhost:8080
```

---

## Cómo correrlo en local

Necesitas **dos terminales**.

**Terminal 1 — Backend (puerto 8080):**

```bash
cd server
cargo run
```

Verás el banner `╔═ SpotiCry server — CLI ═╗`. Si dejaste MP3s nuevos en `library/`, te preguntará cómo cargar cada uno (Spotify / manual / ID3 / saltar). El menú es interactivo y la elección se guarda en `library/tracks.json` para arranques siguientes.

**Terminal 2 — Frontend (puerto 5173):**

```bash
cd client
npm run dev
```

Abre [http://localhost:5173](http://localhost:5173). Al cargar deberías ver la biblioteca con un snapshot vivo del backend.

---

## Cómo agregar canciones

### Opción A — Drop & answer (recomendado)

1. Coloca uno o más `.mp3` en la carpeta `library/`.
2. Reinicia el server (`Ctrl-C` y `cargo run` de nuevo).
3. El auto-scan los detectará. Por cada uno responde:
   - `s` Spotify (pega el enlace o ID — recibe metadata oficial + portada)
   - `m` Manual (escribes título, artista, álbum, género, año)
   - `t` ID3 (lee los tags embebidos del MP3)
   - `x` Saltar (te vuelve a preguntar el próximo arranque)
4. La elección se persiste; el siguiente arranque será 100 % automático.

### Opción B — CLI manual (mientras el server corre)

En la terminal del backend:

```
spoticry> add library/cancion.mp3 4cOdK2wGLETKBW3PvgPWqT      # MP3 + metadata Spotify
spoticry> add library/cancion.mp3                              # MP3 + tags ID3
spoticry> add-spotify 4cOdK2wGLETKBW3PvgPWqT                   # solo Spotify (sin MP3)
spoticry> remove 3                                              # eliminar id 3
spoticry> list                                                  # ver biblioteca
spoticry> playlists                                             # ver playlists
spoticry> help                                                  # ayuda
spoticry> quit                                                  # guardar y salir
```

> ⚠ **Sobre el preview de Spotify**: desde 2024 Spotify devuelve `preview_url = null` en la mayoría de los tracks vía Client Credentials. Por eso el flujo recomendado es siempre `add <ruta-mp3> <spotify-id>` (audio completo del MP3 local + metadata bonita de Spotify) en vez de `add-spotify <id>` solo.

---

## Cómo usar la app

| Acción | Cómo |
|---|---|
| Reproducir una canción | Doble click en la fila o click en el botón ▶ |
| Pausar / reanudar | Botón `⏸` / `▶` del reproductor (siempre fijo abajo) |
| Saltar ±10 s | Botones `« 10s` y `10s »` del reproductor |
| Buscar | Sidebar → `Buscar` → elige tab (Título / Género / Año) |
| Crear playlist | Sidebar → `+ Nueva playlist` → nombre |
| Agregar canción a playlist | Botón `+` en cada fila → elegir playlist (✓ si ya está) |
| Eliminar canción de playlist | Entrar a la playlist → `Quitar` en la fila |
| Filtrar dentro de una playlist | Tabs `Título / Género / Año` en la vista de la playlist |
| Eliminar playlist | Botón rojo en la action-bar de la playlist |

**Móvil**: a menos de 768 px la sidebar se convierte en bottom-nav y el reproductor se compacta automáticamente.

---

## Tests

```bash
# Backend (5 tests, < 100 ms)
cd server
cargo test

# Frontend
# (no hay suite de tests; el enunciado no la pedía)
```

---

## Build de producción

```bash
# Frontend
cd client
npm run build      # genera client/dist/

# Backend
cd ../server
cargo build --release   # binario en server/target/release/spoticry-server
```

---

## Despliegue

El proyecto **se ejecuta localmente para la entrega académica**. No hay deploy automatizado.

Si se quisiera subirlo a la nube, las piezas naturales serían:

- **Frontend**: Azure Static Web Apps, Vercel o Netlify (build `npm run build`, servir `dist/`).
- **Backend**: Cloudflare Tunnel, Fly.io o Railway (binario release de Rust). Una alternativa rápida para demos es `cloudflared tunnel`, pero la URL pública cambia en cada reinicio salvo que se use un tunnel nombrado.
- **Variables de entorno**: pasar `SPOTIFY_CLIENT_ID/SECRET` al backend y `VITE_WS_URL/VITE_HTTP_URL` al build del frontend (apuntando al dominio público del backend con `wss://` y `https://`).

---

## Estructura del proyecto

```
SpotiCry/
├── server/            # Backend Rust (Tokio + Axum)
│   ├── src/
│   │   ├── main.rs            # bootstrap
│   │   ├── domain.rs          # Song, Playlist, ids (CONGELADO)
│   │   ├── protocol.rs        # tipos serde del WebSocket (CONGELADO)
│   │   ├── ws.rs              # handler WebSocket + broadcast
│   │   ├── http_stream.rs     # GET /stream/:id (Range)
│   │   ├── library.rs         # canciones + 3 búsquedas
│   │   ├── library_scan.rs    # auto-scan de library/
│   │   ├── playlists/         # módulo FUNCIONAL PURO (im)
│   │   ├── persistence.rs     # JSON + debouncer
│   │   ├── playback.rs        # tracking "playing"
│   │   ├── spotify.rs         # cliente Spotify Web API
│   │   ├── app_state.rs       # estado global compartido
│   │   └── cli.rs             # interfaz de texto del server
│   └── Cargo.toml
│
├── client/            # Frontend React (Vite + Zustand)
│   └── src/
│       ├── App.jsx, main.jsx
│       ├── api/        # ws.js (cliente WebSocket), stream.js (URL helper)
│       ├── store/      # zustand: library, playlists, player
│       ├── pages/      # Home, Search, Playlist
│       ├── components/ # Sidebar, Player, SongList, PlaylistView, …
│       └── styles/app.css
│
├── library/           # MP3s + tracks.json (manifest del scan)
├── docs/
│   ├── documentacion.md       # documentación técnica completa (ES)
│   └── documentation.en.md    # full technical documentation (EN)
└── README.md          # este archivo
```

---

## Documentación

La documentación técnica vive en `docs/`:

- **[`docs/documentacion.md`](docs/documentacion.md)** — versión en español. Cubre introducción, problema, funcionalidades implementadas y NO implementadas, análisis técnico (arquitectura, stack, decisiones, manejo de memoria, archivos de música, conflictos), protocolo de mensajes, concurrencia con vs sin sincronización, resultados/métricas, conclusiones y trabajo futuro.
- **[`docs/documentation.en.md`](docs/documentation.en.md)** — English version, same content.

---

## Equipo

- **Angie Herrera Aguilar** (2020035640) — Frontend + `cli.rs` / `protocol.rs`
- **Kevin Rivera Gonzalez** (2024157337) — Backend Rust

Profesor: **Oscar Víquez** · Semestre I · Año 2026.

---

## Licencia

Proyecto académico sin licencia comercial.
