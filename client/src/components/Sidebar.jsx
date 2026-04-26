// Navegación principal. Owner: Persona 2.
// En mobile (< 768px) se convierte en bottom nav (ver CSS).
import { useState } from 'react';
import { NavLink } from 'react-router-dom';
import { usePlaylists } from '../store/playlists';
import { CreatePlaylistModal } from './CreatePlaylistModal';

export function Sidebar() {
  const playlists = usePlaylists((s) => s.playlists);
  const [showCreate, setShowCreate] = useState(false);

  return (
    <>
      <aside className="sidebar">
        <h1 className="brand">SpotiCry</h1>
        <nav className="sidebar-nav">
          <NavLink to="/" end>Inicio</NavLink>
          <NavLink to="/search">Buscar</NavLink>
        </nav>
        <div className="sidebar-section">
          <h3>Playlists</h3>
          <button onClick={() => setShowCreate(true)}>+ Nueva playlist</button>
          <ul>
            {playlists.map((pl) => (
              <li key={pl.id}>
                <NavLink to={`/playlist/${pl.id}`}>{pl.name}</NavLink>
              </li>
            ))}
          </ul>
        </div>
      </aside>
      <CreatePlaylistModal isOpen={showCreate} onClose={() => setShowCreate(false)} />
    </>
  );
}
