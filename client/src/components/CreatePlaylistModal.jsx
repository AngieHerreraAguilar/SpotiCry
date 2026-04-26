// Modal para crear playlist. Owner: Persona 2.
// Diseño: Figma "Create Playlist" Desktop (1:2217) / Mobile (132:376) — bottom sheet.
// Solo `name` se envía al backend (es lo único que el contrato actual soporta);
// `mood` y la cover son visuales por ahora — quedan listos para cuando se extienda
// el modelo de Playlist en domain.rs.
import { useEffect, useRef, useState } from 'react';
import { usePlaylists } from '../store/playlists';

const MOODS = ['Noir', 'Rainy', 'Melancholic', 'Deep Focus'];

export function CreatePlaylistModal({ isOpen, onClose }) {
  const create = usePlaylists((s) => s.create);
  const [name, setName] = useState('');
  const [mood, setMood] = useState(null);
  const inputRef = useRef(null);

  useEffect(() => {
    if (isOpen) {
      setName('');
      setMood(null);
      const t = setTimeout(() => inputRef.current?.focus(), 50);
      return () => clearTimeout(t);
    }
  }, [isOpen]);

  useEffect(() => {
    if (!isOpen) return;
    const onKey = (e) => { if (e.key === 'Escape') onClose(); };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const trimmed = name.trim();
  const canSubmit = trimmed.length > 0;

  function handleSubmit(e) {
    e.preventDefault();
    if (!canSubmit) return;
    create(trimmed);
    onClose();
  }

  return (
    <div className="modal-overlay" onClick={onClose} role="dialog" aria-modal="true" aria-labelledby="create-pl-title">
      <form className="modal-card" onClick={(e) => e.stopPropagation()} onSubmit={handleSubmit}>
        <div className="modal-handle" aria-hidden="true" />
        <header className="modal-header">
          <div>
            <h2 id="create-pl-title">New Playlist</h2>
            <p className="modal-subtitle">Name your future sanctuary</p>
          </div>
          <button type="button" className="modal-close" onClick={onClose} aria-label="Cerrar">×</button>
        </header>

        <div className="modal-body">
          <div className="dropzone" onClick={(e) => e.preventDefault()}>
            <div className="dropzone-icon" aria-hidden="true">📷</div>
            <div className="dropzone-title">Add Cover Image</div>
            <div className="dropzone-hint">JPG, PNG O SVG (MAX 50MB)</div>
          </div>

          <div className="form-field">
            <label className="form-label" htmlFor="pl-name">
              Playlist Name<span className="required" aria-hidden="true">*</span>
            </label>
            <input
              id="pl-name"
              ref={inputRef}
              className="form-input"
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Midnight Rain"
              required
              maxLength={80}
            />
          </div>

          <div className="form-field">
            <label className="form-label">Mood Category</label>
            <div className="mood-chips" role="radiogroup" aria-label="Mood Category">
              {MOODS.map((m) => (
                <button
                  type="button"
                  key={m}
                  className={`chip${mood === m ? ' is-selected' : ''}`}
                  onClick={() => setMood(mood === m ? null : m)}
                  role="radio"
                  aria-checked={mood === m}
                >
                  {m}
                </button>
              ))}
            </div>
          </div>
        </div>

        <footer className="modal-footer">
          <button type="submit" className="btn-primary btn-grow" disabled={!canSubmit}>
            Save to Library
          </button>
          <button type="button" className="btn-ghost-outline" onClick={onClose}>
            Discard
          </button>
        </footer>
      </form>
    </div>
  );
}
