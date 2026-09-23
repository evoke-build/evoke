//! `evoke run [--json] <call>`: no classifier. In: the call as written, whether to print the line, the
//! environment. Out: `Exit`. The call is typed by name against the plan; a read or write call runs at once, a
//! destructive one confirms with `[y]es [n]o` — there is no utterance to teach — and the body runs with an empty
//! input. Under `--json` the decision's line goes to stdout, the result or the error on it, and with no terminal
//! the line stands for the confirm that could not be shown. Nothing is logged: nothing was decided.

use evoke_core::plan::Millis;
use evoke_core::{Decision, Input, Written, by_name};

use super::needs_terminal;
use super::session::{self, Confirmed, Opening, Session, dismiss};
use super::{Decline, Exit};
use crate::args::Command;
use crate::hosts::{Environment, terminal};
use crate::report::{self, Line};

pub fn run(command: &Command, written: &Written, json: bool, environment: &Environment) -> Exit {
    let mut session = match session::open(command, json, environment, Opening::Tuning) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    called(&mut session, written, json)
}

/// The call run, its exit reported once: under `--json` a problem is the diagnostic's object, unless the line
/// itself stands for the prompt that could not be shown.
fn called(session: &mut Session<'_>, written: &Written, json: bool) -> Exit {
    let input = session.reporter.command.stand_in();
    let decision = match by_name(&session.plan, written.clone()) {
        Ok(decision) => decision,
        Err(refused) => return session.reporter.exit(&input, Exit::Human(refused)),
    };
    let mut line = Line::unjudged(&decision);
    let warm = match session.warm() {
        Ok(warm) => warm,
        Err(exit) => return session.reporter.exit(&input, exit),
    };
    let chosen = match decision {
        Decision::Run { chosen } => {
            if !json {
                terminal::note(&report::running(&chosen));
            }
            chosen
        }
        Decision::Confirm { chosen, prompt, .. } => {
            if !session.has_tty() {
                dismiss(warm);
                let human = Exit::Human(needs_terminal("a confirm"));
                if json {
                    terminal::result(&line.json());
                    return human;
                }
                return session.reporter.exit(&input, human);
            }
            let own = report::confirming(&chosen, &prompt);
            match session.confirmed(&own, &prompt, false) {
                Ok(Some(Confirmed::No) | None) => {
                    dismiss(warm);
                    if json {
                        terminal::result(&line.json());
                    }
                    return session
                        .reporter
                        .exit(&input, Exit::Declined(Decline::Refused));
                }
                Ok(Some(_)) => chosen,
                Err(exit) => {
                    dismiss(warm);
                    return session.reporter.exit(&input, exit);
                }
            }
        }
        Decision::Ask { .. } | Decision::Abstain { .. } => {
            unreachable!("a call by name runs or confirms")
        }
    };
    let empty = Input::new("").expect("nothing is under the cap");
    let exit = match session.run(&chosen, &empty, Millis(0), warm) {
        Ok(returned) => {
            if !json && !returned.text.is_empty() {
                terminal::result(&returned.text);
            }
            line.result = Some(returned);
            Exit::Ran
        }
        Err(failure) => {
            line.error = Some(report::failure(&failure));
            Exit::Failed(failure)
        }
    };
    if json {
        terminal::result(&line.json());
        // A body's failure is the line's own `error`: nothing more prints, so a line stays one object.
        if line.error.is_some() {
            return exit;
        }
    }
    session.reporter.exit(&input, exit)
}
