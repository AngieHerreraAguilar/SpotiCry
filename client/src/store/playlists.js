// Store Zustand: playlists globales. Owner: Persona 2.
import { create } from 'zustand';
import { sendCmd } from '../api/ws';
import { subscribe } from '../api/ws';

export const usePlaylists = create((set) => ({
  playlists: [],

  setPlaylists: (playlists) => set({ playlists }),

  create: (name) => sendCmd('playlist.create', { name }),
  remove: (playlistId) => sendCmd('playlist.delete', { playlist_id: playlistId }),
  addSong: (playlistId, songId) =>
    sendCmd('playlist.add', { playlist_id: playlistId, song_id: songId }),
  removeSong: (playlistId, songId) =>
    sendCmd('playlist.remove', { playlist_id: playlistId, song_id: songId }),
  sortBy: (playlistId, by) =>
    sendCmd('playlist.sort', { playlist_id: playlistId, by }),
}));

subscribe('playlist.snapshot', (msg) => usePlaylists.getState().setPlaylists(msg.playlists));
