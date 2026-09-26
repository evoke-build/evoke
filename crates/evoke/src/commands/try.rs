//! `evoke try "<input>"`: decide only, and show every judgment, on stdout. In: the parsed command and the
//! environment. Out: `Exit`, 0 for any decision. Each input is decided on the session; nothing runs and no log is
//! written. A stdin filter skips a blank line, stops at a read error, and exits with the first non-zero code.

use evoke_core::{Decision, Gate};

use super::Exit;
use super::each_line;
use super::session::{self, Opening, Session};
use crate::adapter::Adapter;
use crate::args::{Arguments, Command, Inputs};
use crate::hosts::terminal::Text;
use crate::hosts::{Environment, terminal};
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
        Inputs::Stdin => each_line(arguments.json, |input| {
            tried(&session, &*adapter, arguments, input)
        }),
        Inputs::Terminal => unreachable!("try always has an input"),
    }
}

/// One input: read into its steps and shown; any decision exits 0. One step is shown as it always was; more are
/// the plan, then each step's judgments under its number, its word the plan's — or, under `--json`, the plan
/// whole on one line, with every adapter call it took.
fn tried(session: &Session<'_>, adapter: &dyn Adapter, arguments: &Arguments, input: &str) -> Exit {
    let woven = match session.weave(adapter, input, &arguments.tags, Vec::new()) {
        Ok(woven) => woven,
        Err(exit) => return session.reporter.exit(input, exit),
    };
    let floor = adapter.declared().gate.as_ref().map(Gate::route);
    if let Some(decided) = woven.single(&arguments.tags) {
        if arguments.json {
            terminal::result(&report::Line::of(&decided).json());
        } else {
            terminal::answer(&report::tried(&decided, floor));
            if matches!(decided.decision, Decision::Abstain { .. })
                && let Some(hint) = report::left_out(session.plan.inactive().keys())
            {
                terminal::note(&hint);
            }
        }
        return Exit::Ran;
    }
    if arguments.json {
        terminal::result(&report::plan_json(&woven));
        return Exit::Ran;
    }
    if woven.weave.steps.is_empty() {
        terminal::answer(&report::nothing_to_do());
        return Exit::Ran;
    }
    terminal::answer(&report::planned(&woven.weave));
    let of = woven.weave.steps.len();
    for step in &woven.weave.steps {
        let Some(decided) = woven.planned(step, &arguments.tags) else {
            continue;
        };
        terminal::answer(&report::step(
            step.n,
            of,
            Text::from(report::quoted(&step.text)),
        ));
        terminal::answer(&report::tried(&decided, floor));
    }
    Exit::Ran
}
