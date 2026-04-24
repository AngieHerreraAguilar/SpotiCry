# Guía de Angie — Frontend + 2 módulos Rust + Deploy + Design System (Persona 2)

> Esta guía es **tuya**. Léela completa antes de empezar. Kevin tiene la suya en [guia-kevin.md](guia-kevin.md). Cualquier cambio en el contrato (tipos compartidos, mensajes WS) se habla con Kevin **antes** de codear.

## 1. Contexto rápido

- Proyecto académico: app estilo Spotify.
- Dos módulos: **servidor Rust** (Kevin) + **cliente web React** (tú).
- Comunicación: WebSocket JSON + HTTP Range.
- **Tu scope**: todo el frontend + 2 módulos Rust pequeños (`cli.rs`, `protocol.rs`) + design system + deploy + 3 docs narrativas.
- Tiempo: **4 días**. Ver [ROADMAP.md](../ROADMAP.md) para el plan completo.

## 2. Setup inicial (ya tienes el repo local)

```bash
# El repo ya está clonado en:
cd ~/Documents/SpotiCry

# Confirma que todo sigue funcionando:
cd server && cargo check                    # debe compilar con warnings de "unused"
cd ../client && npm run build               # debe producir dist/ sin errores
cd ..
```

### Configurar el MCP de Figma (una vez)

Antes de poder trabajar el design system con Claude leyendo el Figma directo, hay que registrar el servidor MCP oficial (no requiere Figma Desktop, cuenta free sirve):

```bash
claude mcp add --transport http figma https://mcp.figma.com/mcp
```

Luego **reinicia Claude Code** (cerrar/abrir la app), abre una nueva conversación, escribe `/mcp`, selecciona `figma`, autoriza en el navegador, y listo.

Si el comando falla, edita manualmente `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "figma": {
      "type": "http",
      "url": "https://mcp.figma.com/mcp"
    }
  }
}
```

## 3. Tus archivos vs los de Kevin

### Archivos TUYOS (Angie)

**Backend Rust — 2 módulos de apoyo** (en `server/src/`):

| Archivo | Qué hace | Cuándo |
|---|---|---|
| `cli.rs` | Interfaz de texto del servidor (add/remove/list) | Día 1-2 |
| `protocol.rs` | Tipos `serde` para mensajes WS | Día 2 |

**Frontend React** (en `client/src/`): TODO — `main.jsx`, `App.jsx`, `api/*`, `store/*`, `components/*`, `pages/*`, `styles/*`.

**Docs** (en `docs/`):
- [`problema.md`](problema.md) — Día 4
- [`resultados.md`](resultados.md) — Día 4
- [`conclusiones.md`](conclusiones.md) — Día 4

**Deploy** (Día 4, stretch):
- `.github/workflows/azure-swa.yml`
- Azure Static Web Apps
- Cloudflare Tunnel

### Archivos de Kevin — **NO toques** sin avisarle

- Todo `server/src/` **excepto** `cli.rs` y `protocol.rs`.
- Especialmente `domain.rs` — CONGELADO fin de Día 1. Si necesitas un campo nuevo en `Song`/`Playlist`, se lo pides a Kevin.
- Sus docs: `docs/analisis-tecnico.md`, `docs/concurrencia.md`.

### Doc conjunta — Día 1

[`docs/protocolo.md`](protocolo.md) se escribe entre los dos. **Congelar antes de codear handlers.** Si después hay que cambiarlo, se hace en conjunto y se avisa al otro.

## 4. Convenciones del equipo

- **Una rama por tarea**: `git checkout -b angie/sidebar-responsive`. No hagas commits directo en `main`.
- **Pull request** cuando termines una tarea (`gh pr create`). Kevin revisa, tú revisas las suyas.
- **Commits pequeños** y descriptivos: `feat(player): add seek buttons`, `fix(ws): reconnect on close`.
- **No `git push --force`** a `main`.
- **Antes de pushear frontend**: `npm run build` debe pasar sin errores.
- **Si tocas `protocol.rs`, avisa a Kevin** — rompe sus handlers si cambias los tipos.

## 5. Plan día por día — checklist accionable

### Día 1 — Kickoff + CLI

**Objetivo del día**: `protocolo.md` congelado, `domain.rs` acordado con Kevin, y `cli.rs` funcionando para que Kevin tenga forma de agregar canciones mientras desarrolla lo demás.

- [ ] Sync inicial con Kevin (30 min): congelar juntos [`domain.rs`](../server/src/domain.rs) y [`docs/protocolo.md`](protocolo.md). Llevan la versión actual como base; iteren hasta estar de acuerdo.
- [ ] Configurar MCP de Figma (ver §2) y pasarme el link para extraer tokens.
- [ ] Scaffolding del frontend ya está hecho — revisa que todo se entienda:
  - [ ] Corre `cd client && npm run dev` → abre http://localhost:5173 → debería verse feo pero cargar sin errores (ignora la alerta de WS "not connected" que aparecerá en la consola).
- [ ] Implementa [`cli.rs`](../server/src/cli.rs) — ver §7 para el esqueleto y ejemplos.
- [ ] Empieza a anotar colores/tipografía del Figma para el design system.
- [ ] **Commit + push** al final del día.

### Día 2 — protocol.rs + API cliente + stores

**Objetivo**: el frontend conecta al WebSocket de Kevin y recibe al menos `library.snapshot`.

- [ ] Termina [`cli.rs`](../server/src/cli.rs) — que pueda agregar MP3 y listar.
- [ ] Escribe [`protocol.rs`](../server/src/protocol.rs) — ver §7. Kevin lo revisa para confirmar que los tipos encajan con sus handlers.
- [ ] Revisa que [`client/src/api/ws.js`](../client/src/api/ws.js) y [`api/stream.js`](../client/src/api/stream.js) estén alineados con `protocol.rs` (deberían estarlo porque los escribí así).
- [ ] Revisa los stores ([`store/library.js`](../client/src/store/library.js), `playlists.js`, `player.js`) — están funcionales, solo tocar si quieres cambiar alguna acción.
- [ ] Prueba integración básica: corre el servidor de Kevin (`cd server && cargo run`) + frontend en paralelo, abre DevTools, verifica que se conecta al WS y recibe `library.snapshot`.
- [ ] **Commit + push** al final del día.

### Día 3 — UI completa + design system

**Objetivo**: la app funciona end-to-end en localhost — buscar, reproducir, crear playlists, agregar canciones.

- [ ] Aplica los tokens del Figma a `client/src/styles/app.css` (reemplaza los placeholders `--bg`, `--accent`, etc.).
- [ ] Revisa/mejora los componentes (están funcionales pero simples):
  - [ ] [`SearchBar.jsx`](../client/src/components/SearchBar.jsx) — los 3 criterios ya están.
  - [ ] [`SongList.jsx`](../client/src/components/SongList.jsx) — añade botón "Agregar a playlist" con dropdown.
  - [ ] [`PlaylistView.jsx`](../client/src/components/PlaylistView.jsx) — ordenamiento ya está.
  - [ ] [`Player.jsx`](../client/src/components/Player.jsx) — el seek y el buffer del `<audio>` ya los maneja nativo. Verifica responsive.
  - [ ] [`Sidebar.jsx`](../client/src/components/Sidebar.jsx) — nav en desktop, bottom nav en mobile.
- [ ] Opcional: crea `components/AdminPanel.jsx` con UI web para add/remove canciones (stretch — la CLI ya cumple el enunciado).
- [ ] Prueba responsive: Chrome DevTools → modo mobile → breakpoint 768px.
- [ ] Prueba end-to-end completa con el servidor de Kevin corriendo.
- [ ] **Commit + push** al final del día.

### Día 4 — Deploy + Docs + QA

**Mañana**: docs narrativas.

- [ ] [`problema.md`](problema.md) — 1 página: contexto, problema, objetivos, alcance.
- [ ] [`resultados.md`](resultados.md) — qué se logró + screenshots del frontend funcionando.
- [ ] [`conclusiones.md`](conclusiones.md) — aprendizajes, recomendaciones para producción.

**Tarde** (stretch, si hay tiempo): deploy en la nube.

- [ ] Azure Static Web Apps (ver §9).
- [ ] Cloudflare Tunnel para exponer el backend local (ver §9).
- [ ] `.github/workflows/azure-swa.yml` lo genera Azure automáticamente.

**QA conjunto con Kevin**:
- [ ] Prueba de concurrencia multi-pestaña: abrir 3 navegadores, agregar canciones a la misma playlist en simultáneo, verificar que ninguna se pierde. Kevin documenta en [`concurrencia.md`](concurrencia.md).
- [ ] Prueba "no borrar canción reproduciéndose": reproduces una canción, Kevin intenta `remove` desde la CLI, debe fallar. Documenta screenshot.
- [ ] Revisión cruzada: Kevin usa tu frontend, tú lees su código backend y entiendes la concurrencia.
- [ ] Entrega.

## 6. Patrones para React + Zustand

### Usar el estado global (Zustand)

```jsx
// En cualquier componente:
import { useLibrary } from '../store/library';

function MyComponent() {
  // Solo re-renderiza cuando `songs` cambia (selector):
  const songs = useLibrary((s) => s.songs);
  // Varios valores con selector tupla:
  const { searchResults, clearSearch } = useLibrary();
  // Acción (no re-renderiza por estado):
  const clear = useLibrary((s) => s.clearSearch);

  return <button onClick={clear}>Limpiar</button>;
}
```

### Enviar comandos al servidor

```jsx
import { sendCmd } from '../api/ws';

sendCmd('playlist.create', { name: 'Rock 80s' });
sendCmd('search', { by: 'year_range', value: [1990, 2000] });
```

### Responsive con CSS

El archivo `styles/app.css` ya tiene el patrón desktop/mobile. Breakpoint en 768px. El sidebar se convierte en bottom nav automáticamente.

Si tocas estilos, usa las variables ya definidas (`--bg`, `--accent`, `--space-3`, etc.).

## 7. Cómo escribir tus 2 módulos de Rust

Kevin es tu reviewer. Pídele ayuda cuando te atores, pero intenta primero.

### `server/src/cli.rs` — interfaz de texto

Esqueleto completo:

```rust
use crate::AppState;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

pub async fn run(state: Arc<AppState>) -> anyhow::Result<()> {
    print_help();
    let stdin = io::stdin();
    loop {
        print!("spoticry> ");
        io::stdout().flush()?;

        let mut line = String::new();
        if stdin.lock().read_line(&mut line)? == 0 {
            break; // EOF
        }
        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "add" if parts.len() == 2 => {
                // Llama a state.library.write().await.add_song_from_file(...)
                // Kevin implementa add_song_from_file en library.rs
                println!("TODO: add {}", parts[1]);
            }
            "remove" if parts.len() == 2 => {
                let id: u64 = parts[1].parse()?;
                // state.library.write().await.remove_song(id)?;
                println!("TODO: remove {}", id);
            }
            "list" => {
                // imprime canciones
                println!("TODO: list");
            }
            "playlists" => {
                println!("TODO: playlists");
            }
            "help" => print_help(),
            "quit" | "exit" => {
                println!("Guardando...");
                // flush persistence
                break;
            }
            cmd => eprintln!("comando desconocido: {cmd} (escribe 'help')"),
        }
    }
    Ok(())
}

fn print_help() {
    println!("Comandos:");
    println!("  add <ruta>         Agrega un MP3 (lee tags ID3)");
    println!("  remove <id>        Elimina una canción (falla si está sonando)");
    println!("  list               Lista la biblioteca");
    println!("  playlists          Lista las playlists");
    println!("  help               Muestra esta ayuda");
    println!("  quit               Guarda y sale");
}
```

**Nota**: cuando escribas la parte real, llamarás a funciones que Kevin implementa en `library.rs`. Si esas funciones aún no existen cuando tú codeas, deja el `println!("TODO: ...")` como placeholder y después los conectas.

### `server/src/protocol.rs`

Ya está escrito por mí — úsalo como referencia. Lo que tienes que hacer en el Día 2:

1. **Léelo** — entiende cada variante de `ClientMsg` y `ServerEvent`.
2. **Compara con [`docs/protocolo.md`](protocolo.md)** — si algo no coincide, actualiza uno u otro (con Kevin).
3. **Corre `cargo check`** en `server/` — debe compilar.
4. Si Kevin te pide cambios (ej: "necesito un campo más en `NowPlaying`"), edítalo y reescribe tests.

El patrón clave: `#[derive(Serialize, Deserialize)]` + `#[serde(tag = "op", rename_all = "snake_case")]` — eso hace que un JSON como `{"op":"play","song_id":42}` se deserialice automáticamente al enum correcto.

## 8. Design system — del Figma al CSS

Una vez configurado el MCP de Figma:

1. Abre una nueva conversación conmigo en Claude Code.
2. Pégame los links de los frames relevantes del Figma.
3. Te extraigo: paleta de colores, tipografía, spacing, radios, shadows como tokens CSS.
4. Reemplazamos los placeholders en [`client/src/styles/app.css`](../client/src/styles/app.css):
   ```css
   :root {
     --bg: #XXXXXX;           /* ← del Figma */
     --accent: #XXXXXX;       /* ← del Figma */
     --text: #XXXXXX;         /* ← del Figma */
     /* etc. */
   }
   ```
5. Ajustes finos de componentes (bordes, iconos, animaciones) pantalla por pantalla.

**Mientras el MCP no funciona**: screenshots de Figma arrastrados al chat. O "Copy as CSS" desde Dev Mode de Figma pegado como texto.

## 9. Deploy (Día 4, stretch)

### Frontend — Azure Static Web Apps

1. Login en https://portal.azure.com (cuenta de estudiante es gratis si tienes).
2. **Create a resource → Static Web App**.
3. Configuración:
   - Subscription: tu plan.
   - Resource group: crea uno nuevo, `spoticry-rg`.
   - Name: `spoticry`.
   - Plan type: **Free**.
   - Region: la más cercana.
   - Deployment: **GitHub** → autoriza → selecciona `AngieHerreraAguilar/SpotiCry`.
   - Build details:
     - Build Preset: **React**
     - App location: `/client`
     - Output location: `dist`
4. **Create** → Azure genera automáticamente `.github/workflows/azure-static-web-apps-<random>.yml` en tu repo.
5. En Azure Portal → tu SWA → **Configuration** → agrega variables:
   - `VITE_WS_URL` = `wss://<tunnel-url>/ws` (paso siguiente)
   - `VITE_HTTP_URL` = `https://<tunnel-url>`

### Backend — Cloudflare Tunnel

El backend corre en tu máquina pero se expone a internet vía un tunnel (gratis, sin abrir puertos).

```bash
# Instalar:
brew install cloudflared

# En una terminal, con cargo run activo en otra:
cloudflared tunnel --url http://localhost:8080
```

Cloudflare imprime una URL del tipo `https://lorem-ipsum.trycloudflare.com`. Copia esa URL y pégala en las env vars de Azure SWA (paso 5 anterior). Luego re-deploy (push a `main` o disparo manual en Actions).

**Limitación**: la URL del tunnel cambia cada vez que reinicias `cloudflared`. Para una URL estable, crea un tunnel nombrado (requiere dominio en Cloudflare — también gratis): https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/get-started/create-remote-tunnel/

### Si el deploy no funciona y se acaba el tiempo

Graba un **screencast** de la app corriendo en localhost como respaldo para la entrega. QuickTime Player → File → New Screen Recording.

## 10. Cómo probar tu código

### Dev server con backend corriendo

Terminal 1 (backend):
```bash
cd server && cargo run
```

Terminal 2 (frontend):
```bash
cd client && npm run dev
```

Abre http://localhost:5173 — abre DevTools (Console + Network + WS tab).

### Sin backend — modo "UI only"

A veces quieres iterar CSS sin depender de Kevin. Edita temporalmente `api/ws.js`:

```js
// Comenta connect() al final y mockea el estado:
import { useLibrary } from '../store/library';
useLibrary.getState().setSongs([
  { id: 1, title: 'Test Song', artist: 'Angie', album: 'Demo', genre: 'rock', year: 1990, duration_secs: 180 }
]);
```

Revierte antes de commitear.

### Responsive

Chrome DevTools → botón de dispositivos (Cmd+Shift+M) → iPhone 12 → verifica que el sidebar se convierte en bottom nav y las columnas del song list se colapsan.

## 11. Puntos de sincronización con Kevin

| Cuándo | Qué |
|---|---|
| Inicio Día 1 | Acordar `domain.rs` y `protocolo.md` juntos. Congelar. |
| Fin Día 1 | Tu `cli.rs` básica + sus `library.rs` + `persistence.rs` listos. Desde la CLI deberían poder agregar una canción y listarla. |
| Fin Día 2 | Su backend responde a tu frontend vía WebSocket con al menos `library.snapshot`. Tu `protocol.rs` está en el repo. |
| Inicio Día 3 | Backend 100% operativo. Si hay bugs Kevin los arregla en el día mientras tú avanzas UI. |
| Día 4 tarde | Demos cruzadas + pruebas de concurrencia multi-pestaña. |

## 12. Recursos

- **React**: https://react.dev/learn
- **Zustand** (store): https://docs.pmnd.rs/zustand
- **React Router v7**: https://reactrouter.com/start/library/installation
- **Vite docs** (env vars, build): https://vite.dev/guide/
- **MDN `<audio>`** (para el Player): https://developer.mozilla.org/en-US/docs/Web/HTML/Element/audio
- **Serde en Rust** (para `protocol.rs`): https://serde.rs/
- **Azure SWA docs**: https://learn.microsoft.com/en-us/azure/static-web-apps/
- **Cloudflare Tunnel**: https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/

## 13. Si te atoras

1. **Frontend**: abre DevTools, Console + Network. Casi siempre el error está ahí.
2. **Rust**: lee el mensaje de error completo, `cargo clippy` da hints. Si hablamos de `let mut` o lifetimes, Kevin es tu mejor recurso.
3. **CSS responsive**: usa el inspector de Chrome, cambia el breakpoint en vivo.
4. **Si después de 20 min no sales**: pregúntale a Kevin o pégame el error en el chat. No pierdas horas pegada a un punto.
5. **Git mess**: `git status` primero, luego decide. Si vas a hacer algo destructivo (`reset --hard`, `push --force`), primero crea un branch de respaldo: `git branch backup-$(date +%s)`.

---

**Objetivo final del viernes**: frontend funcional en localhost, diseño del Figma aplicado, 3 docs escritas, tus 2 módulos Rust integrados, deploy a Azure intentado (o screencast como respaldo). Si llegas a esto, cumpliste tu mitad. 🎯
