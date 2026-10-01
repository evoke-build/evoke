//! `evoke try [--save <file>] "<input>"`: decide only, and show every judgment, on stdout; with `--save`, the
//! plan's lines, then the plan written as a file for `run <file>` — the sentence, the engine's answers and the
//! pins they were gathered under — and its line on stderr. In: the parsed command and the environment. Out:
//! `Exit`, 0 for any decision; with `--save`, 3 for a plan that asks and 2 for one that refuses, the plan printed
//! and nothing written. Each input is decided on the session; nothing runs and no log is written. A stdin filter
//! skips a blank line, stops at a read error, and exits with the first non-zero code.

use std::path::Path;

use evoke_core::weave::{Because, Bound, Outcome, Status};
use evoke_core::{Decision, Diagnostic, Fix, pin};

use super::rounds::{described, expanded, refusal_fix};
use super::session::{self, Opening, Session, Woven};
use super::{Decline, Exit, each_line};
use crate::adapter::Adapter;
use crate::args::{Arguments, Command, Inputs};
use crate::hosts::files::{self, Landed};
use crate::hosts::terminal::Text;
use crate::hosts::{Environment, terminal};
use crate::report::sentence::{self, Sentence};
use crate::report::{self, Line, StepLine};

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

/// One input: read into its steps and shown; any decision exits 0. The plan where it holds several steps, then
/// the sentence as it was read: each step with its reflex, its values and what each stands on, the call, what
/// would become of it and why — or, under `--json`, one step's line, or the plan whole on one line, with every
/// adapter call it took. With `--save`, the plan's lines and the file.
fn tried(session: &Session<'_>, adapter: &dyn Adapter, arguments: &Arguments, input: &str) -> Exit {
    let woven = match session.weave(adapter, input, &arguments.tags, Vec::new(), &[]) {
        Ok(woven) => woven,
        Err(exit) => return session.reporter.exit(input, exit),
    };
    if let Some(file) = &arguments.save {
        return saved(session, arguments, input, &woven, file);
    }
    let single = woven.single(&arguments.tags, &session.plan);
    if arguments.json {
        match &single {
            Some(decided) => terminal::result(&report::Line::of(decided).json()),
            None => terminal::result(&report::plan_json(&woven)),
        }
        return Exit::Ran;
    }
    let mut told = Sentence::of(&woven, adapter.declared().gate.as_ref());
    told.asked.more(&woven.trace, woven.rounds);
    let lines = if let Some(decided) = &single {
        let mut line = described(session, decided);
        if let Some(step) = woven.weave.steps.first() {
            line.typed.clone_from(&step.typed);
            line.repair = step.repair;
        }
        vec![line]
    } else {
        if !woven.weave.steps.is_empty() {
            terminal::answer(&report::planned(&woven.weave));
            terminal::answer(&Text::new());
        }
        stepped(session, arguments, &woven)
    };
    terminal::answer(&sentence::reading(Some(&told), &lines, true));
    // A part that matched nothing may have asked for an inactive reflex.
    let abstained = lines
        .iter()
        .any(|line| matches!(line.decision, Decision::Abstain { .. }));
    if abstained && let Some(hint) = report::left_out(session.plan.inactive().keys()) {
        terminal::note(&hint);
    }
    Exit::Ran
}

/// A plan's lines as the run would log them, nothing having run: each playbook's expansion, then each step with
/// what the plan binds into it.
fn stepped(session: &Session<'_>, arguments: &Arguments, woven: &Woven) -> Vec<Line> {
    let of = woven.weave.steps.len();
    let mut lines = expanded(session, woven);
    for step in &woven.weave.steps {
        let Some(decided) = woven.planned(step, &arguments.tags) else {
            continue;
        };
        let mut line = described(session, &decided);
        line.typed.clone_from(&step.typed);
        line.repair = step.repair;
        line.step = Some(StepLine {
            n: step.n,
            of,
            status: Status::Ran,
            why: None,
            bound: woven
                .weave
                .binds
                .iter()
                .filter(|bind| bind.to == step.n)
                .map(|bind| Bound {
                    arg: bind.arg.clone(),
                    from: bind.from,
                    field: bind.field.clone(),
                    value: None,
                })
                .collect(),
            shared: step.shared.clone(),
            beside: step.beside.clone(),
            from: step.from.clone(),
            when: step.when.clone(),
        });
        lines.push(line);
    }
    lines
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
            terminal::result(&report::nothing_to_do_json(input, refused_for(woven)));
        } else {
            terminal::answer(&report::nothing_to_do(refused_for(woven)));
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
                refusals(session, woven);
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

/// A refused plan's lines under `--save`: each reason that names what to look at, else the reflexes left out.
fn refusals(session: &Session<'_>, woven: &Woven) {
    let invoked = session.reporter.command.placeholder();
    let mut named = false;
    for because in &woven.weave.verdict.because {
        if matches!(
            because,
            Because::NoSource { .. }
                | Because::SeveralSources { .. }
                | Because::Nested { .. }
                | Because::TooDeep { .. }
                | Because::TooLong { .. }
                | Because::Excluded { .. }
        ) {
            let problem = Diagnostic {
                reflex: None,
                at: None,
                message: report::verdict(because),
                fix: refusal_fix(session, because, woven),
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

/// Why a plan with no step was refused: the verdict's first reason — nothing to do, or a condition.
pub(super) fn refused_for(woven: &Woven) -> &Because {
    woven
        .weave
        .verdict
        .because
        .first()
        .unwrap_or(&Because::NothingToDo)
}
