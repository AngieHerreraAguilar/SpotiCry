// Store Zustand: biblioteca de canciones.
//
// El servidor empuja `library.snapshot` automáticamente al conectar y tras
// cada add/remove (CLI o WS), así que `songs` siempre refleja el estado
// global. `searchResults` se llena solo cuando el usuario lanza una búsqueda
// y se limpia con `clearSearch` para volver a mostrar la biblioteca completa.
import { create } from 'zustand';
import { subscribe } from '../api/ws';

export const useLibrary = create((set) => ({
  songs: [],
  searchResults: null, // null = mostrar toda la biblioteca

  setSongs: (songs) => set({ songs }),
  setSearchResults: (songs) => set({ searchResults: songs }),
  clearSearch: () => set({ searchResults: null }),
}));

subscribe('library.snapshot', (msg) => useLibrary.getState().setSongs(msg.songs));
subscribe('search.result', (msg) => useLibrary.getState().setSearchResults(msg.songs));
