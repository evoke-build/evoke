//! `evoke trust`: the project here blessed at its content — a digest over its four owned paths — so it may run;
//! `evoke`'s own writes re-bless it, a foreign edit leaves it behind; the home project is trusted by construction,
//! and is said to be. In: the environment. Out: `Exit`. No session opens: an untrusted root is what this fixes.

use super::Exit;
use crate::hosts::files;
use crate::hosts::state::State;
use crate::hosts::{Environment, terminal};
use crate::report::{self, Paths};

pub fn run(environment: &Environment) -> Exit {
    match blessed(environment) {
        Ok(Blessed::Home(root)) => {
            terminal::note(&report::home_trusted(&root));
            Exit::Ran
        }
        Ok(Blessed::Now(root)) => {
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

/// The root as shown: the home project, trusted by construction and left as it is, or another, blessed now.
enum Blessed {
    Home(String),
    Now(String),
}

/// The root located and, unless it is the home project, snapshotted, digested and blessed.
fn blessed(environment: &Environment) -> Result<Blessed, Exit> {
    let root = files::locate(environment).map_err(Exit::Failed)?;
    if root.home {
        return Ok(Blessed::Home(files::shown(&root.path, environment)));
    }
    let snapshot = files::snapshot(&root).map_err(Exit::Failed)?;
    State::of(environment)
        .and_then(|state| state.bless(&root.path, &files::digest_of(&snapshot)))
        .map_err(Exit::Failed)?;
    Ok(Blessed::Now(files::shown(&root.path, environment)))
}
