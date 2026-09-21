//! `evoke run <call>`: no classifier. In: the call as written, the environment. Out: `Exit`. The call is typed by
//! name against the plan; a read or write call runs at once, a destructive one confirms with `[y]es [n]o` — there
//! is no utterance to teach — and the body runs with an empty input. Nothing is logged: nothing was decided.

use evoke_core::plan::Millis;
use evoke_core::{Decision, Diagnostic, Fix, Input, Written, by_name};

use super::session::{self, Confirmed, Opening, Session, dismiss};
use super::{Decline, Exit};
use crate::args::Command;
use crate::hosts::{Environment, terminal};
use crate::report;

pub fn run(command: &Command, written: &Written, environment: &Environment) -> Exit {
    let mut session = match session::open(command, false, environment, Opening::Tuning) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let exit = called(&mut session, written);
    session.reporter.exit(&command.placeholder(), exit)
}

fn called(session: &mut Session<'_>, written: &Written) -> Exit {
    let decision = match by_name(&session.plan, written.clone()) {
        Ok(decision) => decision,
        Err(refused) => return Exit::Human(refused),
    };
    let warm = match session.warm() {
        Ok(warm) => warm,
        Err(exit) => return exit,
    };
    let chosen = match decision {
        Decision::Run { chosen } => {
            terminal::note(&report::running(&chosen));
            chosen
        }
        Decision::Confirm { chosen, prompt, .. } => {
            if !session.has_tty() {
                dismiss(warm);
                return Exit::Human(Diagnostic {
                    reflex: None,
                    at: None,
                    message: "a confirm needs a terminal".to_owned(),
                    fix: Fix::Rerun,
                });
            }
            let own = report::confirming(&chosen, &prompt);
            match session.confirmed(&own, &prompt, false) {
                Ok(Some(Confirmed::No) | None) => {
                    dismiss(warm);
                    return Exit::Declined(Decline::Refused);
                }
                Ok(Some(_)) => chosen,
                Err(exit) => {
                    dismiss(warm);
                    return exit;
                }
            }
        }
        Decision::Ask { .. } | Decision::Abstain { .. } => {
            unreachable!("a call by name runs or confirms")
        }
    };
    let input = Input::new("").expect("nothing is under the cap");
    match session.run(&chosen, &input, Millis(0), warm) {
        Ok(returned) => {
            if !returned.text.is_empty() {
                terminal::result(&returned.text);
            }
            Exit::Ran
        }
        Err(failure) => Exit::Failed(failure),
    }
}
