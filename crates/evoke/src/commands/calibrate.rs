//! `evoke calibrate [<name>] [--repeat <k>] [--json]`: what the numbers meant on the records. Every distinct
//! input of every example and test of every active reflex — or of one — decided uncached over the whole set,
//! once or `k` times, a few at a time; each record judged against its input's decisions; the log read whole,
//! each line under this adapter counted by what became of it. In: a name or none, the repeats, the environment.
//! Out: `Exit`; the report on stdout, or `--json`'s one object; exit 1 when a call was wrong at or over its bar
//! or a bin of a hundred calls is over-confident, with the reason. Nothing runs and nothing is logged.

use evoke_core::calibrate::Logged;
use evoke_core::name::LocalName;
use evoke_core::text::NonEmpty;
use evoke_core::{Case, Decision, Fix, calibrate, cases, log_block};
use serde_json::Value as Json;

use super::session::{self, Opening, Session};
use super::{Exit, nothing_installed};
use crate::adapter::Adapter;
use crate::args::Command;
use crate::hosts::{Environment, Failure, terminal, threads};
use crate::report::{self, Line};

pub fn run(
    command: &Command,
    name: Option<&LocalName>,
    repeat: usize,
    json: bool,
    environment: &Environment,
) -> Exit {
    let session = match session::open(command, json, environment, Opening::Deciding) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.stand_in();
    let adapter = match session.adapter(&input) {
        Ok(adapter) => adapter,
        Err(exit) => return session.reporter.exit(&input, exit),
    };
    let exit = calibrated(&session, &*adapter, name, repeat).unwrap_or_else(|exit| exit);
    session.reporter.exit(&input, exit)
}

/// The records decided, the log read, both blocks computed and printed; the exit from the report's own numbers.
fn calibrated(
    session: &Session<'_>,
    adapter: &dyn Adapter,
    name: Option<&LocalName>,
    repeat: usize,
) -> Result<Exit, Exit> {
    if session.project.reflexes.is_empty() {
        return Err(nothing_installed());
    }
    if let Some(name) = name {
        session.plan.running(name).map_err(Exit::Human)?;
    }
    let active = session.plan.active();
    let cases: Vec<Case> = cases(&session.installed)
        .into_iter()
        .filter(|case| active.contains_key(&case.reflex))
        .filter(|case| name.is_none_or(|name| *name == case.reflex))
        .collect();
    if cases.is_empty() {
        terminal::note(&report::nothing_to_calibrate(name));
        return Ok(Exit::Ran);
    }
    // Each distinct input once, in the records' order; every repeat of it far from the last, as the batch runs.
    let mut inputs: Vec<&Case> = Vec::new();
    for case in &cases {
        if !inputs
            .iter()
            .any(|input| input.utterance.id() == case.utterance.id())
        {
            inputs.push(case);
        }
    }
    let items: Vec<&Case> = (0..repeat).flat_map(|_| inputs.iter().copied()).collect();
    let decisions: Vec<Decision> = {
        let busy = terminal::busy_over("calibrating", items.len());
        threads::try_each(&items, |case| {
            let decided = session.decide_uncached(adapter, case.utterance.text().as_str(), &[])?;
            busy.tick();
            Ok(decided.decision)
        })?
    };
    let judged: Vec<(Case, NonEmpty<Decision>)> = cases
        .iter()
        .map(|case| {
            let at = inputs
                .iter()
                .position(|input| input.utterance.id() == case.utterance.id())
                .expect("every case's input was decided");
            let repeats: Vec<Decision> = (0..repeat)
                .map(|k| decisions[k * inputs.len() + at].clone())
                .collect();
            let repeats = NonEmpty::try_from(repeats).expect("an input is decided at least once");
            (case.clone(), repeats)
        })
        .collect();
    let declared = adapter.declared();
    let calibration = calibrate(&declared.id, declared.gate.as_ref(), &session.plan, &judged);
    let (lines, unread) = logged(session)?;
    let log = log_block(&declared.id, declared.gate.as_ref(), &lines, &cases, unread);
    if session.reporter.json {
        let Ok(Json::Object(mut object)) = serde_json::to_value(&calibration) else {
            unreachable!("a calibration serializes as an object")
        };
        object.insert(
            "log".to_owned(),
            serde_json::to_value(&log).expect("a log block serializes"),
        );
        terminal::result(&Json::Object(object).to_string());
    } else {
        terminal::answer(&report::calibrated(&calibration, &log));
    }
    match calibration.failed() {
        None => Ok(Exit::Ran),
        Some(what) => Ok(Exit::Failed(Failure {
            what,
            cause: None,
            fix: Fix::Rerun,
        })),
    }
}

/// The log's lines as the block reads them, and how many did not read.
fn logged(session: &Session<'_>) -> Result<(Vec<Logged>, usize), Exit> {
    let mut lines = Vec::new();
    let mut unread = 0;
    for text in session.state.lines().map_err(Exit::Failed)? {
        match Line::parse(&text) {
            Ok(line) => lines.push(Logged {
                input: line.input,
                decision: line.decision,
                adapters: line.trace.into_iter().map(|trace| trace.adapter).collect(),
                ran: line.result.is_some(),
                failed: line.error.is_some(),
                status: line.step.map(|step| step.status),
            }),
            Err(_) => unread += 1,
        }
    }
    Ok((lines, unread))
}
