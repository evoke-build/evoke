//! The hosts: everything that touches a file, a socket, a clock or the terminal, so the core never does. In: values
//! from the core and the process. Out: facts, or a `Failure` with its fix. A host never constructs a `Diagnostic`.

pub mod files;
pub mod git;
pub mod network;
pub mod processes;
pub mod state;
pub mod store;
pub mod terminal;
pub mod threads;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use evoke_core::Fix;
use evoke_core::name::VarName;
use evoke_core::plan::Millis;

/// A host could not do what it was asked: what was attempted, why, and the command that fixes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub what: String,
    pub cause: Option<String>,
    pub fix: Fix,
}

/// A host's failure with its cause; the fix is to run the command again once the cause is gone.
pub(crate) fn failed(what: &str, cause: &str) -> Failure {
    Failure {
        what: what.to_owned(),
        cause: Some(cause.to_owned()),
        fix: Fix::Rerun,
    }
}

/// An io error as a cause: what it says, without the `(os error N)` its display adds.
pub(crate) fn cause(error: &std::io::Error) -> String {
    let text = error.to_string();
    match text.rfind(" (os error ") {
        Some(at) if text.ends_with(')') => text[..at].to_owned(),
        _ => text,
    }
}

/// The process environment, read once; a variable that is not UTF-8 is as good as unset.
pub struct Environment(pub(crate) BTreeMap<String, String>);

impl Environment {
    #[must_use]
    pub fn of_process() -> Self {
        Self(
            std::env::vars_os()
                .filter_map(|(var, value)| {
                    Some((var.into_string().ok()?, value.into_string().ok()?))
                })
                .collect(),
        )
    }

    #[must_use]
    pub fn get(&self, var: &str) -> Option<&str> {
        self.0.get(var).map(String::as_str)
    }

    /// `evoke`'s directory under an XDG base: the variable when set, else its default under `$HOME`.
    pub fn xdg(&self, var: &str, default: &str) -> Result<PathBuf, Failure> {
        if let Some(base) = self.get(var) {
            return Ok(Path::new(base).join("evoke"));
        }
        match self.get("HOME") {
            Some(home) => Ok(Path::new(home).join(default).join("evoke")),
            None => Err(Failure {
                what: format!("finding {default}/evoke"),
                cause: Some("HOME is not set".to_owned()),
                fix: Fix::ExportKey {
                    var: VarName::new("HOME").expect("HOME is a variable name"),
                },
            }),
        }
    }
}

/// The host's instant, made from the core's duration: when a call — the adapter's, or the body's — must be over,
/// and the clock the call is timed by.
#[derive(Clone, Copy, Debug)]
pub struct Deadline {
    started: Instant,
    at: Instant,
}

impl Deadline {
    #[must_use]
    pub fn after(millis: Millis) -> Self {
        let started = Instant::now();
        Self {
            started,
            at: started + Duration::from_millis(millis.0),
        }
    }

    /// The same deadline, `spent` sooner: what is left of a shared budget once an earlier call took its share.
    #[must_use]
    pub fn less(self, spent: Millis) -> Self {
        let whole = self.at.saturating_duration_since(self.started);
        Self {
            started: self.started,
            at: self.started + whole.saturating_sub(Duration::from_millis(spent.0)),
        }
    }

    /// What is left, or nothing once it passed.
    #[must_use]
    pub fn remaining(&self) -> Duration {
        self.at.saturating_duration_since(Instant::now())
    }

    /// What is left, as the core's duration: what a body is told.
    #[must_use]
    pub fn left(&self) -> Millis {
        Millis(u64::try_from(self.remaining().as_millis()).unwrap_or(u64::MAX))
    }

    /// Milliseconds since it was set: the trace's `ms`.
    #[must_use]
    pub fn elapsed(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_body_run_after_a_slow_prompt_keeps_most_of_the_deadline() {
        // The adapter's share, as the trace recorded it; then the person at the prompt, however long.
        let spent = Millis(1_200);
        std::thread::sleep(Duration::from_millis(50));
        // The body is timed from its run: the plan's deadline less the adapter's share, nothing for the prompt.
        let body = Deadline::after(Millis(30_000)).less(spent);
        let left = body.left().0;
        assert!((28_700..=28_800).contains(&left), "left {left} ms");
    }

    #[test]
    fn an_io_error_is_a_cause_without_its_number() {
        let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
        assert_eq!(cause(&denied), "permission denied");
        let raw = std::io::Error::from_raw_os_error(13);
        assert!(!cause(&raw).contains("os error"));
        let plain = std::io::Error::other("the pipe closed");
        assert_eq!(cause(&plain), "the pipe closed");
    }

    #[test]
    fn a_share_past_the_budget_leaves_nothing() {
        let body = Deadline::after(Millis(30_000)).less(Millis(u64::MAX));
        assert_eq!(body.left(), Millis(0));
        assert!(body.remaining().is_zero());
    }
}
