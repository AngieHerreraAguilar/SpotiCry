// Modal para crear playlist. Owner: Persona 2.
// Diseño: Figma "Create Playlist" Desktop (1:2217) / Mobile (132:376) — bottom sheet.
// Solo `name` se envía al backend (es lo único que el contrato actual soporta).
import { useEffect, useRef, useState } from 'react';
import { usePlaylists } from '../store/playlists';

export function CreatePlaylistModal({ isOpen, onClose }) {
  const create = usePlaylists((s) => s.create);
  const playlists = usePlaylists((s) => s.playlists);
  const [name, setName] = useState('');
  const inputRef = useRef(null);

  useEffect(() => {
    if (isOpen) {
      setName('');
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
  const isDuplicate =
    trimmed.length > 0 &&
    playlists.some((p) => p.name.trim().toLowerCase() === trimmed.toLowerCase());
  const canSubmit = trimmed.length > 0 && !isDuplicate;

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
            <h2 id="create-pl-title">Nueva Playlist</h2>
            <p className="modal-subtitle">Dale un nombre a tu colección</p>
          </div>
          <button type="button" className="modal-close" onClick={onClose} aria-label="Cerrar">×</button>
        </header>

        <div className="modal-body">
          <div className="form-field">
            <label className="form-label" htmlFor="pl-name">
              Nombre<span className="required" aria-hidden="true">*</span>
            </label>
            <input
              id="pl-name"
              ref={inputRef}
              className={`form-input${isDuplicate ? ' has-error' : ''}`}
              type="text"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Ej: Lluvia de medianoche"
              required
              maxLength={80}
              aria-invalid={isDuplicate}
              aria-describedby={isDuplicate ? 'pl-name-error' : undefined}
            />
            {isDuplicate && (
              <p id="pl-name-error" className="form-error" role="alert">
                Ya existe una playlist con ese nombre.
              </p>
            )}
          </div>
        </div>

        <footer className="modal-footer">
          <button type="submit" className="btn-primary btn-grow" disabled={!canSubmit}>
            Guardar
          </button>
          <button type="button" className="btn-ghost-outline" onClick={onClose}>
            Descartar
          </button>
        </footer>
      </form>
    </div>
  );
}
