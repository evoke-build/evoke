//! `evoke try [--save <file>] "<input>"`: decide only, and show every judgment, on stdout; with `--save`, the
//! plan's lines, then the plan written as a file for `run <file>` — the sentence, the engine's answers and the
//! pins they were gathered under — and its line on stderr. In: the parsed command and the environment. Out:
//! `Exit`, 0 for any decision; with `--save`, 3 for a plan that asks and 2 for one that refuses, the plan printed
//! and nothing written. Each input is decided on the session; nothing runs and no log is written. A stdin filter
//! skips a blank line, stops at a read error, and exits with the first non-zero code.

use std::path::Path;

use evoke_core::weave::{Because, Outcome};
use evoke_core::{Decision, Diagnostic, Fix, Gate, pin};

use super::rounds::source_fix;
use super::session::{self, Opening, Session, Woven};
use super::{Decline, Exit, each_line};
use crate::adapter::Adapter;
use crate::args::{Arguments, Command, Inputs};
use crate::hosts::files::{self, Landed};
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
/// the plan, then each step's judgments under its number, its word the plan's and the words shared into it named
/// — or, under `--json`, the plan whole on one line, with every adapter call it took. With `--save`, the plan's
/// lines and the file.
fn tried(session: &Session<'_>, adapter: &dyn Adapter, arguments: &Arguments, input: &str) -> Exit {
    let woven = match session.weave(adapter, input, &arguments.tags, Vec::new()) {
        Ok(woven) => woven,
        Err(exit) => return session.reporter.exit(input, exit),
    };
    if let Some(file) = &arguments.save {
        return saved(session, arguments, input, &woven, file);
    }
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
    let mut abstained = false;
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
        if let Some(origin) = report::shared(&step.shared) {
            terminal::answer(&Text::from(format!("  {origin}")));
        }
        abstained |= matches!(decided.decision, Decision::Abstain { .. });
    }
    // A part that matched nothing may have asked for an inactive reflex, as one input's abstain says.
    if abstained && let Some(hint) = report::left_out(session.plan.inactive().keys()) {
        terminal::note(&hint);
    }
    Exit::Ran
}

/// `--save <file>`: the plan's lines — the plan whole on one line under `--json` — then the plan sealed with its
/// pins and written whole, its line on stderr. A plan that asks is refused with the step's question, since a file
/// holds no one's answers, exit 3; one that refuses, exit 2; nothing written either way.
fn saved(
    session: &Session<'_>,
    arguments: &Arguments,
    input: &str,
    woven: &Woven,
    file: &str,
) -> Exit {
    let json = arguments.json;
    if woven.weave.steps.is_empty() {
        if json {
            terminal::result(&report::nothing_to_do_json(input));
        } else {
            terminal::answer(&report::nothing_to_do());
        }
        return Exit::Declined(Decline::Refused);
    }
    if json {
        terminal::result(&report::plan_json(woven));
    } else {
        terminal::answer(&report::planned(&woven.weave));
    }
    let verdict = &woven.weave.verdict;
    match verdict.outcome {
        Outcome::Ask => {
            let message = verdict
                .because
                .iter()
                .find_map(|because| match because {
                    Because::Needs { step, arg } => {
                        Some(format!("step {step} asks {arg}; say it in the sentence"))
                    }
                    Because::Several { .. } | Because::OneOfMany { .. } => {
                        Some(report::verdict(because))
                    }
                    _ => None,
                })
                .unwrap_or_else(|| "the plan asks".to_owned());
            let problem = Exit::Human(Diagnostic {
                reflex: None,
                at: None,
                message,
                fix: Fix::Rerun,
            });
            // Under --json the plan line stands for the problem: its verdict says what asks.
            return if json {
                problem
            } else {
                session.reporter.exit(input, problem)
            };
        }
        Outcome::Refuse => {
            if !json {
                let invoked = session.reporter.command.placeholder();
                let mut named = false;
                for because in &verdict.because {
                    if matches!(
                        because,
                        Because::NoSource { .. } | Because::SeveralSources { .. }
                    ) {
                        let problem = Diagnostic {
                            reflex: None,
                            at: None,
                            message: report::verdict(because),
                            fix: source_fix(session, because, woven),
                        };
                        terminal::note(&report::diagnostic(
                            &problem,
                            &invoked,
                            Some(&session.reporter.paths),
                        ));
                        named = true;
                    }
                }
                if !named && let Some(hint) = report::left_out(session.plan.inactive().keys()) {
                    terminal::note(&hint);
                }
            }
            return Exit::Declined(Decline::Refused);
        }
        Outcome::Run | Outcome::Confirm => {}
    }
    let pinned = pin(
        &session.installed,
        &session.plan,
        &session.project,
        session.lock.as_ref(),
        &session.declared,
        input,
        &arguments.tags,
        woven.weave.clone(),
        woven.answers(),
    );
    let text = format!(
        "{}\n",
        serde_json::to_string_pretty(&pinned).expect("a plan serializes")
    );
    if let Err(failure) = files::write(Path::new(file), &text) {
        return session.reporter.exit(input, Exit::Failed(failure));
    }
    if !json {
        let shown = files::shown(Path::new(file), session.environment());
        terminal::note(&report::written(&shown, &Landed::Whole));
    }
    Exit::Ran
}
