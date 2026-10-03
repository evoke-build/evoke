//! The built-in adapters as pure mappings: the System One wire behind four doors, `jev`, `openjev`, `clef` and
//! `clef_flash`, and `replay`. Questions to a request, a response to answers.

#![forbid(unsafe_code)]
// Errors are data with their own types, not prose to document; diagnostics and faults are the surface's errors by design.
#![expect(clippy::missing_errors_doc)]
// An Err of a Diagnostic is large with 64-bit pointers and not on wasm32, so this cannot be an expectation.
#![allow(clippy::result_large_err)]

pub mod replay;
pub mod systemone;
