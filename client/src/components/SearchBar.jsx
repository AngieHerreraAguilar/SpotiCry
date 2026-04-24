// Barra de búsqueda con los 3 criterios del enunciado (tabs). Owner: Persona 2.
//   - Título      → op:'search', by:'title', value: string
//   - Género      → op:'search', by:'genre', value: string
//   - Año (rango) → op:'search', by:'year_range', value: [from, to]
import { useState } from 'react';
import { sendCmd } from '../api/ws';
import { useLibrary } from '../store/library';

export function SearchBar() {
  const [mode, setMode] = useState('title');
  const [query, setQuery] = useState('');
  const [yearFrom, setYearFrom] = useState('');
  const [yearTo, setYearTo] = useState('');
  const clearSearch = useLibrary((s) => s.clearSearch);

  function submit(e) {
    e.preventDefault();
    if (mode === 'year_range') {
      const from = parseInt(yearFrom, 10);
      const to = parseInt(yearTo, 10);
      if (Number.isFinite(from) && Number.isFinite(to)) {
        sendCmd('search', { by: 'year_range', value: [from, to] });
      }
    } else if (query.trim()) {
      sendCmd('search', { by: mode, value: query.trim() });
    }
  }

  return (
    <form className="search-bar" onSubmit={submit}>
      <div className="search-tabs">
        <button type="button" className={mode === 'title' ? 'active' : ''} onClick={() => setMode('title')}>Título</button>
        <button type="button" className={mode === 'genre' ? 'active' : ''} onClick={() => setMode('genre')}>Género</button>
        <button type="button" className={mode === 'year_range' ? 'active' : ''} onClick={() => setMode('year_range')}>Año</button>
      </div>
      {mode === 'year_range' ? (
        <div className="year-range">
          <input type="number" placeholder="desde" value={yearFrom} onChange={(e) => setYearFrom(e.target.value)} />
          <input type="number" placeholder="hasta" value={yearTo} onChange={(e) => setYearTo(e.target.value)} />
        </div>
      ) : (
        <input type="text" placeholder={`Buscar por ${mode}…`} value={query} onChange={(e) => setQuery(e.target.value)} />
      )}
      <button type="submit">Buscar</button>
      <button type="button" onClick={clearSearch}>Limpiar</button>
    </form>
  );
}
