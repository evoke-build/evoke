//! The built-in adapters as pure mappings: the System One wire behind two doors, `jev` and `openjev`, and
//! `replay`. Questions to a request, a response to answers.

#![forbid(unsafe_code)]
// Errors are data with their own types, not prose to document; diagnostics and faults are the surface's errors by design.
#![expect(clippy::missing_errors_doc)]

pub mod replay;
pub mod systemone;
