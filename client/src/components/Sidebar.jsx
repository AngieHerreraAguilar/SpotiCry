// Navegación principal. Owner: Persona 2.
// En mobile (< 768px) se convierte en bottom nav (ver CSS).
import { NavLink } from 'react-router-dom';
import { usePlaylists } from '../store/playlists';

export function Sidebar() {
  const playlists = usePlaylists((s) => s.playlists);
  const createPlaylist = usePlaylists((s) => s.create);

  return (
    <aside className="sidebar">
      <nav className="sidebar-nav">
        <NavLink to="/" end>Inicio</NavLink>
        <NavLink to="/search">Buscar</NavLink>
      </nav>
      <div className="sidebar-section">
        <h3>Tus Playlists</h3>
        <button onClick={() => {
          const name = prompt('Nombre de la playlist:');
          if (name) createPlaylist(name);
        }}>+ Nueva</button>
        <ul>
          {playlists.map((pl) => (
            <li key={pl.id}>
              <NavLink to={`/playlist/${pl.id}`}>{pl.name}</NavLink>
            </li>
          ))}
        </ul>
      </div>
    </aside>
  );
}
