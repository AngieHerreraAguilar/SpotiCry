// Lista de canciones (tabla). Owner: Persona 2.
import { useLibrary } from '../store/library';
import { usePlayer } from '../store/player';

export function SongList() {
  const songs = useLibrary((s) => s.searchResults ?? s.songs);
  const play = usePlayer((s) => s.play);

  if (!songs.length) return <p className="empty">No hay canciones. Agrégalas vía CLI.</p>;

  return (
    <ul className="song-list">
      {songs.map((s) => (
        <li key={s.id} className="song-row" onDoubleClick={() => play(s)}>
          <span className="song-title">{s.title}</span>
          <span className="song-artist">{s.artist}</span>
          <span className="song-album">{s.album}</span>
          <span className="song-genre">{s.genre}</span>
          <span className="song-year">{s.year}</span>
          <button onClick={() => play(s)}>▶</button>
          {/* TODO(Persona 2): botón "Agregar a playlist" con dropdown */}
        </li>
      ))}
    </ul>
  );
}
