//! `evoke try "<input>"`: decide only, and show every judgment, on stdout. In: the parsed command and the
//! environment. Out: `Exit`, 0 for any decision. Each input is decided on the session; nothing runs and no log is
//! written. A stdin filter skips a blank line, stops at a read error, and exits with the first non-zero code.

use evoke_core::{Decision, Fix, Gate};

use super::Exit;
use super::session::{self, Opening, Session};
use crate::adapter::Adapter;
use crate::args::{Arguments, Command, Inputs};
use crate::hosts::{Environment, Failure, terminal};
use crate::report;

pub fn run(command: &Command, arguments: &Arguments, environment: &Environment) -> Exit {
    let session = match session::open(command, arguments.json, environment, Opening::Deciding) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let adapter = match session.adapter(&command.stand_in()) {
        Ok(adapter) => adapter,
        Err(exit) => return session.reporter.exit(&command.stand_in(), exit),
    };
    match &arguments.input {
        Inputs::One(input) => tried(&session, &*adapter, arguments, input),
        Inputs::Stdin => {
            let mut first = Exit::Ran;
            for line in terminal::stdin_lines() {
                let exit = match line {
                    Ok(input) if input.trim().is_empty() => continue,
                    Ok(input) => tried(&session, &*adapter, arguments, &input),
                    Err(error) => {
                        let failed = Exit::Failed(Failure {
                            what: "reading stdin".to_owned(),
                            cause: Some(error.to_string()),
                            fix: Fix::Rerun,
                        });
                        let exit = session.reporter.exit("<input>", failed);
                        return if first == Exit::Ran { exit } else { first };
                    }
                };
                if first == Exit::Ran {
                    first = exit;
                }
            }
            first
        }
        Inputs::Terminal => unreachable!("try always has an input"),
    }
}

/// One input: decided and shown; any decision exits 0.
fn tried(session: &Session<'_>, adapter: &dyn Adapter, arguments: &Arguments, input: &str) -> Exit {
    let decided = match session.decide(adapter, input, &arguments.tags) {
        Ok(decided) => decided,
        Err(exit) => return session.reporter.exit(input, exit),
    };
    if arguments.json {
        terminal::result(&report::Line::of(&decided).json());
    } else {
        let floor = adapter.declared().gate.as_ref().map(Gate::route);
        terminal::answer(&report::tried(&decided, floor));
        if matches!(decided.decision, Decision::Abstain { .. })
            && let Some(hint) = report::left_out(session.plan.inactive().keys())
        {
            terminal::note(&hint);
        }
    }
    Exit::Ran
}
