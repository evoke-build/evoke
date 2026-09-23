//! The rules of `evoke`, pure: no engine, no I/O.

#![forbid(unsafe_code)]
// Errors are data with their own types, not prose to document; `Diagnostic` is the surface's error by design.
#![expect(clippy::missing_errors_doc)]
// An Err of a Diagnostic is large with 64-bit pointers and not on wasm32, so this cannot be an expectation.
#![allow(clippy::result_large_err)]

pub mod adapter;
pub mod call;
pub mod contract;
pub mod decide;
pub mod diagnostic;
pub mod digest;
pub mod document;
pub mod dts;
pub mod edit;
pub mod manifest;
pub mod name;
pub mod overlay;
pub mod plan;
pub mod project;
pub mod propose;
pub mod run;
pub mod test;
pub mod text;
pub mod vocabulary;
pub mod weave;

pub use adapter::{Declared, Fault, Gate, Key, Limits, Prob, Question, QuestionId, Raw, Request};
pub use call::{Call, Value, Written, call, render};
pub use contract::{
    Change, Consent, ContractDiff, Finding, Level, LintRule, WasViolation, consent, diff, lint,
};
pub use decide::{
    Asking, Cap, Choices, Chosen, Contender, Decision, Judged, Judgment, Missing, Prompt, Reading,
    Scope, Why, Winner, by_name, fill, gate, picked, read, request, validated,
};
pub use diagnostic::{At, Diagnostic, File, Fix};
pub use digest::{Digest, compose, digest};
pub use document::{Document, Json, KeyPath};
pub use dts::{project_dts, reflex_dts};
pub use edit::{
    Edit, Lesson, Owned, VocabChange, add_entry, remove_entry, set_config, teach, vocab_edit,
};
pub use manifest::{Manifest, manifest};
pub use overlay::{Effective, Overlay, Report, effective, overlay, report};
pub use plan::{Active, Held, Installed, Item, Millis, Plan, Slot, compile};
pub use project::{Lock, Project, Reference, Version, lock, project, reference, render_lock};
pub use propose::{PickValue, Proposed, propose};
pub use run::{Envelope, argv, envelope};
pub use test::{
    Baseline, Case, Claim, Expected, Mismatch, Regression, Table, Theft, Verdict, baseline, cases,
    judge, regressions, thieves,
};
pub use text::{Clean, Identity, Input, Span, Utterance, identity};
pub use vocabulary::{Vocabulary, vocabulary};
pub use weave::{Executed, Planning, Running, Weave};
