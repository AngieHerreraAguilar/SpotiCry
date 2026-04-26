// Vista de una playlist con hero + action bar + lista. Owner: Persona 2.
// Diseño: Figma "Playlist Detail" Desktop (1:1214) / Mobile (1:1837).
// El cover es un mosaico 2x2 — al no tener imágenes reales por canción todavía,
// muestra placeholders con las iniciales del título de las primeras 4 canciones.
import { useParams } from 'react-router-dom';
import { usePlaylists } from '../store/playlists';
import { useLibrary } from '../store/library';
import { usePlayer } from '../store/player';
import { SongList } from './SongList';

function formatDuration(totalSecs) {
  if (!Number.isFinite(totalSecs) || totalSecs <= 0) return '0 min';
  const h = Math.floor(totalSecs / 3600);
  const m = Math.round((totalSecs % 3600) / 60);
  if (h > 0) return `${h} h ${m} min`;
  return `${m} min`;
}

function initials(title) {
  if (!title) return '♪';
  const words = title.trim().split(/\s+/);
  return (words[0][0] + (words[1]?.[0] ?? '')).toUpperCase();
}

export function PlaylistView() {
  const { id } = useParams();
  const pid = parseInt(id, 10);
  const playlist = usePlaylists((s) => s.playlists.find((p) => p.id === pid));
  const songs = useLibrary((s) => s.songs);
  const play = usePlayer((s) => s.play);
  const sortBy = usePlaylists((s) => s.sortBy);
  const removeSong = usePlaylists((s) => s.removeSong);

  if (!playlist) return <p className="empty">Playlist no encontrada</p>;

  const items = playlist.songs
    .map((sid) => songs.find((s) => s.id === sid))
    .filter(Boolean);

  const totalSecs = items.reduce((acc, s) => acc + (s.duration_secs ?? 0), 0);
  const mosaicSlots = Array.from({ length: 4 }, (_, i) => items[i]);

  function playAll() {
    if (items[0]) play(items[0]);
  }

  return (
    <section className="playlist-view">
      <header className="playlist-hero">
        <div className="playlist-cover-mosaic" aria-hidden="true">
          {mosaicSlots.map((s, i) => (
            <div key={i}>{s ? initials(s.title) : ''}</div>
          ))}
        </div>
        <div className="playlist-meta">
          <span className="playlist-kicker">Public Playlist</span>
          <h1 className="playlist-title">{playlist.name}</h1>
          <p className="playlist-description">
            {items.length > 0
              ? `Una colección de ${items.length} ${items.length === 1 ? 'canción' : 'canciones'}.`
              : 'Aún no tiene canciones. Agrega desde la búsqueda.'}
          </p>
          <div className="playlist-stats">
            <span className="playlist-owner">SpotiCry</span>
            <span className="dot" />
            <span>{items.length} {items.length === 1 ? 'canción' : 'canciones'}</span>
            {totalSecs > 0 && (
              <>
                <span className="dot" />
                <span>{formatDuration(totalSecs)}</span>
              </>
            )}
          </div>
        </div>
      </header>

      <div className="playlist-action-bar">
        <button
          className="btn-play-lg"
          onClick={playAll}
          disabled={!items.length}
          aria-label="Reproducir playlist"
        >
          ▶
        </button>
        <div className="sort-controls">
          <button onClick={() => sortBy(pid, 'title')}>Título</button>
          <button onClick={() => sortBy(pid, 'year')}>Año</button>
          <button onClick={() => sortBy(pid, 'duration')}>Duración</button>
        </div>
      </div>

      <SongList
        songs={items}
        onRemove={(songId) => removeSong(pid, songId)}
        emptyText="Esta playlist está vacía."
      />
    </section>
  );
}
