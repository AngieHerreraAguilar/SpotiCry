// Helper para URLs de audio. Owner: Persona 2.
// El navegador maneja Range requests automáticamente en el elemento <audio>.

const HTTP_URL = import.meta.env.VITE_HTTP_URL || 'http://localhost:8080';

export function streamUrl(songId) {
  return `${HTTP_URL}/stream/${songId}`;
}
