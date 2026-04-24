// Store Zustand: reproductor de audio. Owner: Persona 2.
//
// El buffer local de "adelantar/retroceder cuantas veces se quiera" del enunciado
// lo maneja el navegador nativamente vía el elemento <audio>:
//   audio.currentTime = t  ⇒  el navegador hace nueva Range request si es necesario,
//                             o usa el buffer ya descargado en memoria.
import { create } from 'zustand';
import { sendCmd, subscribe } from '../api/ws';
import { streamUrl } from '../api/stream';

export const usePlayer = create((set, get) => ({
  currentSong: null,
  isPlaying: false,
  currentTime: 0,
  duration: 0,
  audioEl: null, // <audio> ref (set por <Player>)

  setAudioEl: (el) => set({ audioEl: el }),

  play: (song) => {
    const audio = get().audioEl;
    if (!audio) return;
    audio.src = streamUrl(song.id);
    audio.play();
    sendCmd('play', { song_id: song.id });
    set({ currentSong: song, isPlaying: true });
  },

  togglePause: () => {
    const audio = get().audioEl;
    if (!audio) return;
    if (audio.paused) audio.play();
    else audio.pause();
  },

  stop: () => {
    const audio = get().audioEl;
    const id = get().currentSong?.id;
    if (audio) {
      audio.pause();
      audio.currentTime = 0;
    }
    if (id) sendCmd('stop', { song_id: id });
    set({ isPlaying: false, currentSong: null });
  },

  seekTo: (seconds) => {
    const audio = get().audioEl;
    if (audio) audio.currentTime = seconds;
  },

  _tick: (currentTime, duration) => set({ currentTime, duration }),
  _setPlaying: (isPlaying) => set({ isPlaying }),
}));

subscribe('now_playing', (msg) => {
  console.info('[player] now_playing event', msg);
});
subscribe('playback.stopped', () => usePlayer.getState()._setPlaying(false));
