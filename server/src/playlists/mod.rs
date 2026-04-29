// Módulo FUNCIONAL de playlists. Owner: Persona 1.
//
// RESTRICCIÓN DEL ENUNCIADO:
//   Este módulo debe implementarse en estilo funcional puro:
//     ❌ Sin `let mut`, sin mutabilidad en sitio
//     ✅ Estructuras inmutables (im::HashMap, im::Vector)
//     ✅ Transformaciones funcionales: map, filter, fold, collect, reduce, iter
//     ✅ Closures (|x| ...)
//
// Este módulo NO importa `tokio`, `std::sync`, ni ningún tipo de lock.
// El wiring al estado compartido vive en `ws.rs` (imperative shell).

pub mod ops;
pub mod state;
