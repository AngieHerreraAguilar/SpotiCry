// Vista de una playlist + ordenamiento. Owner: Persona 2.
import { useParams } from 'react-router-dom';
import { usePlaylists } from '../store/playlists';
import { useLibrary } from '../store/library';
import { usePlayer } from '../store/player';

export function PlaylistView() {
  const { id } = useParams();
  const pid = parseInt(id, 10);
  const playlist = usePlaylists((s) => s.playlists.find((p) => p.id === pid));
  const songs = useLibrary((s) => s.songs);
  const play = usePlayer((s) => s.play);
  const sortBy = usePlaylists((s) => s.sortBy);
  const removeSong = usePlaylists((s) => s.removeSong);

  if (!playlist) return <p>Playlist no encontrada</p>;

  const items = playlist.songs
    .map((sid) => songs.find((s) => s.id === sid))
    .filter(Boolean);

  return (
    <section className="playlist-view">
      <header>
        <h2>{playlist.name}</h2>
        <div className="sort-controls">
          <button onClick={() => sortBy(pid, 'title')}>Ordenar por título</button>
          <button onClick={() => sortBy(pid, 'year')}>Por año</button>
          <button onClick={() => sortBy(pid, 'duration')}>Por duración</button>
        </div>
      </header>
      <ul className="song-list">
        {items.map((s) => (
          <li key={s.id} className="song-row">
            <span className="song-title">{s.title}</span>
            <span className="song-artist">{s.artist}</span>
            <button onClick={() => play(s)}>▶</button>
            <button onClick={() => removeSong(pid, s.id)}>Quitar</button>
          </li>
        ))}
      </ul>
    </section>
  );
}
