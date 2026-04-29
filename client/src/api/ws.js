// Cliente WebSocket singleton. Owner: Persona 2.
//
// - Abre conexión a VITE_WS_URL
// - Expone sendCmd(op, payload) y subscribe(ev, handler)
// - Reconecta con backoff exponencial
// - Corresponde al contrato en docs/protocolo.md

const WS_URL = import.meta.env.VITE_WS_URL || 'ws://localhost:8080/ws';

let socket = null;
const listeners = new Map(); // ev -> Set<handler>
let reconnectAttempt = 0;

function connect() {
  socket = new WebSocket(WS_URL);

  socket.addEventListener('open', () => {
    reconnectAttempt = 0;
    console.info('[ws] connected to', WS_URL);
  });

  socket.addEventListener('message', (e) => {
    try {
      const msg = JSON.parse(e.data);
      const handlers = listeners.get(msg.ev);
      if (handlers) handlers.forEach((h) => h(msg));
    } catch (err) {
      console.warn('[ws] non-JSON message:', e.data);
    }
  });

  socket.addEventListener('close', () => {
    const delay = Math.min(1000 * 2 ** reconnectAttempt, 15000);
    reconnectAttempt += 1;
    setTimeout(connect, delay);
  });

  socket.addEventListener('error', (err) => {
    console.error('[ws] error', err);
  });
}

export function sendCmd(op, payload = {}) {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    console.warn('[ws] not connected; dropping', op);
    return;
  }
  socket.send(JSON.stringify({ op, ...payload }));
}

export function subscribe(ev, handler) {
  if (!listeners.has(ev)) listeners.set(ev, new Set());
  listeners.get(ev).add(handler);
  return () => listeners.get(ev)?.delete(handler);
}

connect();
