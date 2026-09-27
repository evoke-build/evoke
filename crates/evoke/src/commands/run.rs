//! `evoke run [--json] <call> | <file>`. A call by name: no classifier. In: the call as written, whether to print
//! the line, the environment. Out: `Exit`. The call is typed by name against the plan; a read or write call runs
//! at once, a destructive one confirms with `[y]es [n]o` — there is no utterance to teach — and the body runs with
//! an empty input. Under `--json` the decision's line goes to stdout, the result or the error on it, and with no
//! terminal the line stands for the confirm that could not be shown. Nothing is logged: nothing was decided.
//! Ctrl-C while the body runs ends its group, and `evoke` ends as an interrupted process, the line's `error`
//! saying so.
//!
//! A plan file, told from a call by its first word: read, `plan = 1` required; the session opened as a decision's,
//! the adapter unresolved; the first pin that moved refused with its fix; the plan made again from the file's
//! answers under the file's gate, refused when it does not read the same; then the plan printed, one yes over it at
//! this terminal whatever its verdict, and each step at its turn through the foundation's loop as a sentence takes
//! it, the adapter resolved at the first step decided again. Every step's line, printed under `--json` and
//! logged, names the file and its digest.

use std::path::Path;

use evoke_core::plan::Millis;
use evoke_core::{Decision, Digest, Fix, Input, Json, Written, by_name, pinned, replan, stale};
use indexmap::IndexMap;

use super::rounds::{Engine, Rounds};
use super::session::{self, Confirmed, Failed, Opening, Session, Woven};
use super::{Decline, Exit, Reporter, human, needs_terminal};
use crate::args::{Command, Target};
use crate::hosts::{Environment, files, interrupt, terminal};
use crate::report::{self, Line, Paths, PinnedAt};

pub fn run(command: &Command, target: &Target, json: bool, environment: &Environment) -> Exit {
    match target {
        Target::Call(written) => {
            let mut session = match session::open(command, json, environment, Opening::Tuning) {
                Ok(session) => session,
                Err(exit) => return exit,
            };
            called(&mut session, written, json)
        }
        Target::File(file) => planned(command, file, json, environment),
    }
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
    let contained = session.contained.clone();
    let chosen = match decision {
        Decision::Run { chosen } => {
            if !json {
                terminal::note(&report::running(&chosen, &contained));
            }
            chosen
        }
        Decision::Confirm { chosen, prompt, .. } => {
            if !session.has_tty() {
                let human = Exit::Human(needs_terminal("a confirm"));
                if json {
                    terminal::result(&line.json());
                    return human;
                }
                return session.reporter.exit(&input, human);
            }
            let own = report::confirming(&chosen, &prompt, &contained);
            match session.confirmed(&own, &prompt, false) {
                Ok(Some(Confirmed::No) | None) => {
                    if json {
                        terminal::result(&line.json());
                    }
                    return session
                        .reporter
                        .exit(&input, Exit::Declined(Decline::Refused));
                }
                Ok(Some(_)) => chosen,
                Err(exit) => return session.reporter.exit(&input, exit),
            }
        }
        Decision::Ask { .. } | Decision::Abstain { .. } => {
            unreachable!("a call by name runs or confirms")
        }
    };
    let empty = Input::new("").expect("nothing is under the cap");
    line.contained = Some(contained);
    let exit = match session.run(&chosen, &IndexMap::new(), &empty, Millis(0)) {
        Ok(returned) => {
            if !json && !returned.text.is_empty() {
                terminal::result(&returned.text);
            }
            line.result = Some(returned);
            Exit::Ran
        }
        Err(Failed { failure, frames }) => {
            line.error = Some(report::failure(&failure));
            line.frames = frames;
            Exit::Failed(failure)
        }
    };
    if json {
        terminal::result(&line.json());
    }
    if interrupt::interrupted() {
        interrupt::end();
    }
    // A body's failure is the line's own `error`: nothing more prints, so a line stays one object.
    if json && line.error.is_some() {
        return exit;
    }
    session.reporter.exit(&input, exit)
}

/// A plan file run: read and checked before the session opens, since a file that is no plan needs no project;
/// then the pins, the plan again, and the rounds — every line naming the file by its path as shown and the
/// SHA-256 of what was read.
fn planned(command: &Command, file: &str, json: bool, environment: &Environment) -> Exit {
    let reporter = Reporter {
        command,
        json,
        paths: Paths::of(environment),
    };
    let input = command.stand_in();
    let shown = files::shown(Path::new(file), environment);
    let text = match files::read(Path::new(file)) {
        Ok(Some(text)) => text,
        Ok(None) => {
            return reporter.exit(
                &input,
                human(format!("{shown}: there is no such file"), Fix::Help),
            );
        }
        Err(failure) => return reporter.exit(&input, Exit::Failed(failure)),
    };
    let json_value: Json = match serde_json::from_str(&text) {
        Ok(json) => json,
        Err(error) => {
            return reporter.exit(
                &input,
                human(format!("{shown} does not read as JSON: {error}"), Fix::Help),
            );
        }
    };
    let pinned = match pinned(&shown, json_value) {
        Ok(pinned) => pinned,
        Err(problem) => return reporter.exit(&input, Exit::Human(problem)),
    };
    let session = match session::open(command, json, environment, Opening::Deciding) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let moved = stale(
        &shown,
        &pinned,
        &session.installed,
        &session.plan,
        session.lock.as_ref(),
        &session.declared,
    );
    if let Err(problem) = moved {
        return session.reporter.exit(&input, Exit::Human(problem));
    }
    let replanned = match replan(&shown, &pinned, &session.plan) {
        Ok(replanned) => replanned,
        Err(problem) => return session.reporter.exit(&input, Exit::Human(problem)),
    };
    let woven = Woven::replanned(replanned);
    let mut rounds = Rounds {
        session,
        engine: Engine::Later,
        json,
        tags: pinned.tags.clone(),
        pinned: Some(PinnedAt {
            file: shown.clone(),
            id: Digest::of(text.as_bytes()),
        }),
    };
    rounds.run_pinned(&pinned.input, &woven, &shown)
}
