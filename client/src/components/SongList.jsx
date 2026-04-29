// Lista de canciones (tabla). Owner: Persona 2.
// Por defecto muestra la biblioteca/búsqueda; con `songs` explícito se usa
// para playlists. Si se pasa `onRemove`, cada fila gana un botón "Quitar".
import { useEffect, useRef, useState } from 'react';
import { useLibrary } from '../store/library';
import { usePlayer } from '../store/player';
import { usePlaylists } from '../store/playlists';

function AddToPlaylistMenu({ songId }) {
  const playlists = usePlaylists((s) => s.playlists);
  const addSong = usePlaylists((s) => s.addSong);
  const [open, setOpen] = useState(false);
  const ref = useRef(null);

  useEffect(() => {
    if (!open) return;
    function onClickOutside(e) {
      if (ref.current && !ref.current.contains(e.target)) setOpen(false);
    }
    document.addEventListener('mousedown', onClickOutside);
    return () => document.removeEventListener('mousedown', onClickOutside);
  }, [open]);

  if (playlists.length === 0) return null;

  return (
    <div className="add-to-playlist" ref={ref}>
      <button
        type="button"
        className="add-to-playlist-btn"
        onClick={(e) => { e.stopPropagation(); setOpen((v) => !v); }}
        aria-label="Agregar a playlist"
        aria-expanded={open}
      >
        +
      </button>
      {open && (
        <div className="add-to-playlist-menu" role="menu">
          <div className="add-to-playlist-header">Agregar a playlist</div>
          {playlists.map((pl) => {
            const already = pl.songs.includes(songId);
            return (
              <button
                key={pl.id}
                type="button"
                className="add-to-playlist-item"
                role="menuitem"
                onClick={(e) => {
                  e.stopPropagation();
                  if (!already) addSong(pl.id, songId);
                  setOpen(false);
                }}
              >
                <span className="add-to-playlist-name">{pl.name}</span>
                {already && <span className="add-to-playlist-check" aria-label="Ya está en esta playlist">✓</span>}
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
}

function CoverThumb({ url, title }) {
  if (url) {
    return <img className="song-cover" src={url} alt="" loading="lazy" />;
  }
  // Fallback con iniciales si la canción no tiene cover_url.
  const letter = (title || '♪').trim().charAt(0).toUpperCase();
  return (
    <div className="song-cover song-cover-fallback" aria-hidden="true">
      {letter}
    </div>
  );
}

export function SongList({ songs: songsProp, onRemove, emptyText }) {
  const fallback = useLibrary((s) => s.searchResults ?? s.songs);
  const songs = songsProp ?? fallback;
  const play = usePlayer((s) => s.play);
  const currentId = usePlayer((s) => s.currentSong?.id);

  if (!songs.length) {
    return <p className="empty">{emptyText ?? 'No hay canciones. Agrégalas vía CLI.'}</p>;
  }

  return (
    <ul className="song-list">
      {songs.map((s, idx) => {
        const isPlaying = currentId === s.id;
        const rowClass = [
          'song-row',
          isPlaying ? 'playing' : '',
          onRemove ? 'has-remove' : '',
        ].filter(Boolean).join(' ');

        return (
          <li key={`${idx}-${s.id}`} className={rowClass} onDoubleClick={() => play(s)}>
            <span className="song-index">{String(s.id).padStart(2, '0')}</span>
            <CoverThumb url={s.cover_url} title={s.title} />
            <span className="song-title">{s.title}</span>
            <span className="song-artist">{s.artist}</span>
            <span className="song-album">{s.album}</span>
            <span className="song-genre">{s.genre}</span>
            {!onRemove && <AddToPlaylistMenu songId={s.id} />}
            <span className="song-year">{s.year}</span>
            <button onClick={() => play(s)} aria-label={`Reproducir ${s.title}`}>▶</button>
            {onRemove && (
              <button
                className="song-remove"
                onClick={() => onRemove(s.id)}
                aria-label={`Quitar ${s.title} de la playlist`}
              >
                Quitar
              </button>
            )}
          </li>
        );
      })}
    </ul>
  );
}
