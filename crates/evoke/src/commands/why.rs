//! `evoke why`: the last sentence, explained from the log. In: the environment. Out: `Exit`; the log's last
//! input — the sentence and its steps — read out as `try` would show it, with what became of it, on stdout.

use evoke_core::{Diagnostic, Fix};

use super::Exit;
use crate::args::Command;
use crate::hosts::state::State;
use crate::hosts::{Environment, terminal};
use crate::report::sentence::{self, Sentence};
use crate::report::{self, Line, Paths};

pub fn run(environment: &Environment) -> Exit {
    let exit = explained(environment);
    // Nothing decided yet asks for an input; anything else that went wrong is fixed by running `why` again.
    let invoked = match &exit {
        Exit::Human(problem) if problem.message == NOTHING_YET => "evoke \"<input>\"".to_owned(),
        _ => Command::Why.invoked(""),
    };
    if let Some(line) = report::exit(&exit, &invoked, Some(&Paths::of(environment))) {
        terminal::note(&line);
    }
    exit
}

const NOTHING_YET: &str = "nothing has been decided yet";

fn explained(environment: &Environment) -> Exit {
    let state = match State::of(environment) {
        Ok(state) => state,
        Err(failure) => return Exit::Failed(failure),
    };
    let tail = match state.tail() {
        Ok(tail) if tail.lines.is_empty() && tail.sentence.is_none() => {
            return Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message: NOTHING_YET.to_owned(),
                fix: Fix::Rerun,
            });
        }
        Ok(tail) => tail,
        Err(failure) => return Exit::Failed(failure),
    };
    let whole = tail.sentence.as_deref().and_then(Sentence::parse);
    let parsed: Result<Vec<Line>, String> =
        tail.lines.iter().map(|line| Line::parse(line)).collect();
    match parsed {
        Ok(lines) => {
            terminal::answer(&sentence::reading(whole.as_ref(), &lines, false));
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
