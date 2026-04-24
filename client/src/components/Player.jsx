// Reproductor. Owner: Persona 2.
// <audio> oculto + controles custom. El buffer del elemento HTML
// permite adelantar/retroceder cuantas veces se quiera para la canción actual.
import { useEffect, useRef } from 'react';
import { usePlayer } from '../store/player';

export function Player() {
  const audioRef = useRef(null);
  const setAudioEl = usePlayer((s) => s.setAudioEl);
  const { currentSong, isPlaying, currentTime, duration, togglePause, seekTo, _tick, _setPlaying } = usePlayer();

  useEffect(() => {
    setAudioEl(audioRef.current);
  }, [setAudioEl]);

  return (
    <div className="player-bar">
      <audio
        ref={audioRef}
        onTimeUpdate={(e) => _tick(e.target.currentTime, e.target.duration || 0)}
        onPlay={() => _setPlaying(true)}
        onPause={() => _setPlaying(false)}
      />
      <div className="player-info">
        {currentSong ? (
          <>
            <strong>{currentSong.title}</strong> — {currentSong.artist}
          </>
        ) : (
          <span className="muted">Sin reproducir</span>
        )}
      </div>
      <div className="player-controls">
        <button onClick={() => seekTo(Math.max(0, currentTime - 10))}>« 10s</button>
        <button onClick={togglePause}>{isPlaying ? '⏸' : '▶'}</button>
        <button onClick={() => seekTo(currentTime + 10)}>10s »</button>
      </div>
      <input
        type="range"
        min={0}
        max={duration || 0}
        step={0.1}
        value={currentTime}
        onChange={(e) => seekTo(parseFloat(e.target.value))}
      />
      <span className="player-time">
        {fmt(currentTime)} / {fmt(duration)}
      </span>
    </div>
  );
}

function fmt(sec) {
  if (!Number.isFinite(sec)) return '0:00';
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}
