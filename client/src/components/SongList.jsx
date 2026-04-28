// Lista de canciones (tabla). Owner: Persona 2.
// Por defecto muestra la biblioteca/búsqueda; con `songs` explícito se usa
// para playlists. Si se pasa `onRemove`, cada fila gana un botón "Quitar".
import { useLibrary } from '../store/library';
import { usePlayer } from '../store/player';
import { usePlaylists } from '../store/playlists';

function AddToPlaylistMenu({ songId }) {
  const playlists = usePlaylists((s) => s.playlists);
  const addSong = usePlaylists((s) => s.addSong);

  if (playlists.length === 0) return null;

  function handleChange(e) {
    const pid = parseInt(e.target.value, 10);
    if (Number.isFinite(pid)) addSong(pid, songId);
    e.target.selectedIndex = 0;
  }

  return (
    <select
      className="song-add-to"
      defaultValue=""
      onChange={handleChange}
      aria-label="Agregar a playlist"
    >
      <option value="" disabled>+ playlist</option>
      {playlists.map((pl) => (
        <option key={pl.id} value={pl.id}>{pl.name}</option>
      ))}
    </select>
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
            <span className="song-index">{String(idx + 1).padStart(2, '0')}</span>
            <span className="song-title">{s.title}</span>
            <span className="song-artist">{s.artist}</span>
            <span className="song-album">{s.album}</span>
            <span className="song-genre">{s.genre}</span>
            <span className="song-year">{s.year}</span>
            <button onClick={() => play(s)} aria-label={`Reproducir ${s.title}`}>▶</button>
            {!onRemove && <AddToPlaylistMenu songId={s.id} />}
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
