//! `evoke trust`: the project here blessed at its content — a digest over its four owned paths — so it may run;
//! `evoke`'s own writes re-bless it, a foreign edit leaves it behind. In: the environment. Out: `Exit`. No session
//! opens: an untrusted root is what this fixes.

use super::Exit;
use crate::hosts::files;
use crate::hosts::state::State;
use crate::hosts::{Environment, terminal};
use crate::report::{self, Paths};

pub fn run(environment: &Environment) -> Exit {
    match blessed(environment) {
        Ok(root) => {
            terminal::note(&report::trusted(&root));
            Exit::Ran
        }
        Err(exit) => {
            if let Some(line) = report::exit(&exit, "evoke trust", Some(&Paths::of(environment))) {
                terminal::note(&line);
            }
            exit
        }
    }
}

/// The root located, snapshotted, digested and blessed; its path as shown.
fn blessed(environment: &Environment) -> Result<String, Exit> {
    let root = files::locate(environment).map_err(Exit::Failed)?;
    let snapshot = files::snapshot(&root).map_err(Exit::Failed)?;
    State::of(environment)
        .and_then(|state| state.bless(&root.path, &files::digest_of(&snapshot)))
        .map_err(Exit::Failed)?;
    Ok(files::shown(&root.path, environment))
}
