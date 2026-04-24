// Store Zustand: biblioteca de canciones. Owner: Persona 2.
import { create } from 'zustand';
import { subscribe } from '../api/ws';

export const useLibrary = create((set) => ({
  songs: [],
  searchResults: null, // null = mostrar toda la biblioteca

  setSongs: (songs) => set({ songs }),
  setSearchResults: (songs) => set({ searchResults: songs }),
  clearSearch: () => set({ searchResults: null }),
}));

// TODO(Persona 2): suscribirse a eventos 'library.snapshot' y 'search.result'
subscribe('library.snapshot', (msg) => useLibrary.getState().setSongs(msg.songs));
subscribe('search.result', (msg) => useLibrary.getState().setSearchResults(msg.songs));
