//! The SDK's boundary: the core and the adapters as one WebAssembly module. Three exports — `alloc`, `free` and
//! `call` — move UTF-8 across it in buffers the host allocates and frees; `call` takes an op name and one JSON
//! argument object and answers `{ ok }`, `{ err }` or `{ bug }`. The op table is [`call`], the same function
//! natively, so the vectors pin it here and the SDK pins it again through the built module.

// Errors are data with their own types, as in the core.
#![allow(clippy::missing_errors_doc)]

#[cfg(target_arch = "wasm32")]
mod boundary;
mod ops;

pub use ops::call;
