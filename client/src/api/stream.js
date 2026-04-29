// Helper para URLs de audio. Owner: Persona 2.
// El navegador maneja Range requests automáticamente en el elemento <audio>.

const HTTP_URL = import.meta.env.VITE_HTTP_URL || 'http://localhost:8080';

// `t` es un cache-buster por sesión: cada llamada a play() produce una URL
// distinta para que la caché de medios de Chrome (que no se purga con
// Cmd+Shift+R) no pueda servir audio antiguo si en una sesión previa el
// mismo `songId` apuntaba a otro archivo. Dentro de la misma reproducción,
// la URL no cambia, así que los Range requests siguen funcionando.
export function streamUrl(songId) {
  return `${HTTP_URL}/stream/${songId}?t=${Date.now()}`;
}
