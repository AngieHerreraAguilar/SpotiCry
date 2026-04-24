import { SearchBar } from '../components/SearchBar';
import { SongList } from '../components/SongList';

export function Search() {
  return (
    <section className="page-search">
      <h2>Buscar</h2>
      <SearchBar />
      <SongList />
    </section>
  );
}
