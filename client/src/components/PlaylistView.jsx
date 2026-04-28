// Vista de una playlist con hero + action bar + filtro + lista. Owner: Persona 2.
// Diseño: Figma "Playlist Detail" Desktop (1:1214) / Mobile (1:1837).
// El filtro envía la op `playlist.filter` (requisito del proyecto: validar el
// módulo funcional puro de Kevin con closures + map/filter/fold). Mientras se
// define el shape de la respuesta, también filtra localmente para que el UX
// sea inmediato.
import { useMemo, useState } from 'react';
import { useParams } from 'react-router-dom';
import { sendCmd } from '../api/ws';
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

  // Filtro local + sincronización con el backend (módulo funcional)
  const [filterBy, setFilterBy] = useState(null);
  const [filterText, setFilterText] = useState('');
  const [yearFrom, setYearFrom] = useState('');
  const [yearTo, setYearTo] = useState('');

  // Todos los hooks se llaman incondicionalmente; el early return va al final.
  const items = useMemo(() => {
    if (!playlist) return [];
    return playlist.songs
      .map((sid) => songs.find((s) => s.id === sid))
      .filter(Boolean);
  }, [playlist, songs]);

  const displayedItems = useMemo(() => {
    if (!filterBy) return items;
    if (filterBy === 'title') {
      const q = filterText.trim().toLowerCase();
      return q ? items.filter((s) => s.title.toLowerCase().includes(q)) : items;
    }
    if (filterBy === 'genre') {
      const q = filterText.trim().toLowerCase();
      return q ? items.filter((s) => s.genre.toLowerCase() === q) : items;
    }
    if (filterBy === 'year_range') {
      const from = parseInt(yearFrom, 10);
      const to = parseInt(yearTo, 10);
      if (!Number.isFinite(from) || !Number.isFinite(to)) return items;
      return items.filter((s) => s.year >= from && s.year <= to);
    }
    return items;
  }, [items, filterBy, filterText, yearFrom, yearTo]);

  if (!playlist) return <p className="empty">Playlist no encontrada</p>;

  const totalSecs = items.reduce((acc, s) => acc + (s.duration_secs ?? 0), 0);
  const mosaicSlots = Array.from({ length: 4 }, (_, i) => items[i]);
  const isFiltering = filterBy && displayedItems.length !== items.length;

  function applyFilter() {
    if (filterBy === 'year_range') {
      const from = parseInt(yearFrom, 10);
      const to = parseInt(yearTo, 10);
      if (!Number.isFinite(from) || !Number.isFinite(to)) return;
      sendCmd('playlist.filter', { playlist_id: pid, by: 'year_range', value: [from, to] });
    } else if (filterBy && filterText.trim()) {
      sendCmd('playlist.filter', { playlist_id: pid, by: filterBy, value: filterText.trim() });
    }
  }

  function clearFilter() {
    setFilterBy(null);
    setFilterText('');
    setYearFrom('');
    setYearTo('');
  }

  function playAll() {
    if (displayedItems[0]) play(displayedItems[0]);
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
          disabled={!displayedItems.length}
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

      <div className="playlist-filter">
        <div className="search-tabs" role="tablist" aria-label="Filtrar por">
          <button
            type="button"
            className={filterBy === 'title' ? 'active' : ''}
            onClick={() => setFilterBy(filterBy === 'title' ? null : 'title')}
          >
            Título
          </button>
          <button
            type="button"
            className={filterBy === 'genre' ? 'active' : ''}
            onClick={() => setFilterBy(filterBy === 'genre' ? null : 'genre')}
          >
            Género
          </button>
          <button
            type="button"
            className={filterBy === 'year_range' ? 'active' : ''}
            onClick={() => setFilterBy(filterBy === 'year_range' ? null : 'year_range')}
          >
            Año
          </button>
        </div>
        {filterBy === 'year_range' ? (
          <div className="year-range">
            <input
              type="number"
              placeholder="desde"
              value={yearFrom}
              onChange={(e) => setYearFrom(e.target.value)}
            />
            <input
              type="number"
              placeholder="hasta"
              value={yearTo}
              onChange={(e) => setYearTo(e.target.value)}
            />
          </div>
        ) : filterBy ? (
          <input
            type="text"
            placeholder={filterBy === 'genre' ? 'Ej: rock' : 'Buscar en título…'}
            value={filterText}
            onChange={(e) => setFilterText(e.target.value)}
          />
        ) : null}
        {filterBy && (
          <>
            <button type="button" onClick={applyFilter}>Filtrar</button>
            <button type="button" onClick={clearFilter}>Limpiar</button>
          </>
        )}
      </div>

      {isFiltering && (
        <p className="filter-status">
          Mostrando {displayedItems.length} de {items.length} canciones
        </p>
      )}

      <SongList
        songs={displayedItems}
        onRemove={(songId) => removeSong(pid, songId)}
        emptyText={isFiltering ? 'Ninguna canción coincide con el filtro.' : 'Esta playlist está vacía.'}
      />
    </section>
  );
}
