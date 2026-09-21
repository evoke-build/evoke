//! `evoke why`: the last decision, explained from the log. In: the environment. Out: `Exit`; the log's last line
//! rendered as `try` would show it, with what came of it.

use evoke_core::{Diagnostic, Fix};

use super::Exit;
use crate::hosts::state::State;
use crate::hosts::{Environment, terminal};
use crate::report::{self, Line};

pub fn run(environment: &Environment) -> Exit {
    let exit = explained(environment);
    if let Some(line) = report::exit(&exit, "evoke \"<input>\"", None) {
        terminal::note(&line);
    }
    exit
}

fn explained(environment: &Environment) -> Exit {
    let state = match State::of(environment) {
        Ok(state) => state,
        Err(failure) => return Exit::Failed(failure),
    };
    let last = match state.last() {
        Ok(Some(last)) => last,
        Ok(None) => {
            return Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message: "nothing has been decided yet".to_owned(),
                fix: Fix::Rerun,
            });
        }
        Err(failure) => return Exit::Failed(failure),
    };
    match Line::parse(&last) {
        Ok(line) => {
            terminal::note(&report::why(&line));
            Exit::Ran
        }
        Err(why) => Exit::Human(Diagnostic {
            reflex: None,
            at: None,
            message: format!("the log's last line does not read: {why}"),
            fix: Fix::Rerun,
        }),
    }
}
