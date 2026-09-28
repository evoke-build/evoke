//! `evoke test [name]`: every example and test of every active reflex — or of one — decided uncached over the
//! whole set, a few at a time, and judged on its route and asserted arguments; a case that passed at the last run
//! under this plan and fails now is decided twice more and is a regression when two of three fail; the baseline
//! kept per plan in the cache. A playbook's steps too: each filled from the first of its records whose reading
//! fills every slot it holds — never a word of the tool's own; a step no record fills is untested — decided
//! uncached over the whole set, and judged on whether its words route to a reflex, as the plan would decide it.
//! In: a name or none, the environment. Out: `Exit`, a line per reflex and one per failure on stdout, the
//! spinner counting the cases; exit 1 when any case failed, with the count. Nothing runs and nothing is logged.

use evoke_core::manifest::{Sentence, Yield};
use evoke_core::name::{ArgName, LocalName};
use evoke_core::text::NonEmpty;
use evoke_core::{
    Case, Decision, Fix, Plan, Table, Utterance, Verdict, baseline, cases, filled, judge,
    regressions, steps,
};

use super::session::{self, Opening, Session};
use super::{Exit, nothing_installed};
use crate::adapter::Adapter;
use crate::args::Command;
use crate::hosts::{Environment, Failure, terminal, threads};
use crate::report::{self, StepBecame, TestedPlaybook, TestedStep};

pub fn run(command: &Command, name: Option<&LocalName>, environment: &Environment) -> Exit {
    let session = match session::open(command, false, environment, Opening::Deciding) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.stand_in();
    let adapter = match session.adapter(&input) {
        Ok(adapter) => adapter,
        Err(exit) => return session.reporter.exit(&input, exit),
    };
    let exit = tested(&session, &*adapter, name).unwrap_or_else(|exit| exit);
    session.reporter.exit(&input, exit)
}

/// The cases judged, the regressions named against the last baseline, the next baseline kept, the block printed.
fn tested(
    session: &Session<'_>,
    adapter: &dyn Adapter,
    name: Option<&LocalName>,
) -> Result<Exit, Exit> {
    if session.project.reflexes.is_empty() {
        return Err(nothing_installed());
    }
    if let Some(name) = name {
        session.plan.running(name).map_err(Exit::Human)?;
    }
    let active = session.plan.active();
    let wanted = |case: &Case| {
        active.contains_key(&case.reflex) && name.is_none_or(|name| *name == case.reflex)
    };
    let cases: Vec<Case> = cases(&session.installed)
        .into_iter()
        .filter(wanted)
        .collect();
    let playbooks: Vec<Case> = steps(&session.installed)
        .into_iter()
        .filter(wanted)
        .collect();
    if cases.is_empty() && playbooks.is_empty() {
        terminal::note(&report::nothing_to_test(name));
        return Ok(Exit::Ran);
    }
    let digest = session.plan.digest();
    let before = session
        .state
        .baseline(&digest)
        .map_err(Exit::Failed)?
        .unwrap_or_default();
    // The records first, each decided once; then each playbook's steps, filled from what those decisions read.
    let (decisions, first) = {
        let busy = terminal::busy_over("testing", cases.len());
        let decided = threads::try_each(&cases, |case| {
            let decided = decided_once(session, adapter, case.utterance.text().as_str());
            busy.tick();
            decided
        })?;
        let first: Vec<Verdict> = cases
            .iter()
            .zip(&decided)
            .map(|(case, decision)| judge(case, decision))
            .collect();
        (decided, first)
    };
    let (filled_steps, untested) = filled_steps(session, &cases, &decisions, &playbooks);
    let step_first = {
        let busy = terminal::busy_over("testing", filled_steps.len());
        threads::try_each(&filled_steps, |(case, text)| {
            let decision = decided_once(session, adapter, text)?;
            busy.tick();
            Ok((judge(case, &decision), decision))
        })?
    };
    let mut all: Vec<(Case, Option<String>, Verdict)> = cases
        .into_iter()
        .zip(first)
        .map(|(case, verdict)| (case, None, verdict))
        .collect();
    let mut routed: Vec<(Case, Decision)> = Vec::new();
    for ((case, text), (verdict, decision)) in filled_steps.into_iter().zip(step_first) {
        routed.push((case.clone(), decision));
        all.push((case, Some(text), verdict));
    }
    let judged = repeated(session, adapter, &before, all)?;
    let regressed = regressions(&before, &judged);
    let next = baseline(&before, &judged);
    session
        .state
        .keep_baseline(&digest, &next)
        .map_err(Exit::Failed)?;
    let verdicts: Vec<(Case, Verdict)> = judged
        .into_iter()
        .map(|(case, _)| {
            let verdict = next
                .get(&case)
                .cloned()
                .expect("every judged case is in the baseline");
            (case, verdict)
        })
        .collect();
    let records: Vec<(Case, Verdict)> = verdicts
        .iter()
        .filter(|(case, _)| case.from != Table::Steps)
        .cloned()
        .collect();
    let blocks = playbook_blocks(session, &verdicts, &routed, &untested, &regressed);
    if !records.is_empty() || !blocks.is_empty() {
        terminal::answer(&report::tested(&records, &regressed, &blocks));
    }
    let failed = verdicts
        .iter()
        .filter(|(_, verdict)| matches!(verdict, Verdict::Fail { .. }))
        .count();
    if failed == 0 {
        return Ok(Exit::Ran);
    }
    Ok(Exit::Failed(Failure {
        what: format!("{failed} of {} cases failed", verdicts.len()),
        cause: None,
        fix: Fix::Rerun,
    }))
}

/// One uncached decision of a text.
fn decided_once(
    session: &Session<'_>,
    adapter: &dyn Adapter,
    text: &str,
) -> Result<Decision, Exit> {
    Ok(session.decide_uncached(adapter, text, &[])?.decision)
}

/// The steps no record fills: each with the first slot it holds.
type Untested = Vec<(Case, ArgName)>;

/// Each playbook's steps as its records' decisions fill them: per step, its case and the filled sentence, from
/// the first record of the playbook whose reading fills every slot; or, with no such record, the first slot
/// the step holds, which leaves it untested.
fn filled_steps(
    session: &Session<'_>,
    cases: &[Case],
    decisions: &[Decision],
    playbooks: &[Case],
) -> (Vec<(Case, String)>, Untested) {
    let mut filled_steps = Vec::new();
    let mut untested = Vec::new();
    for case in playbooks {
        let Some(sentence) = sentence_of(session, case) else {
            continue;
        };
        let own: Vec<&Decision> = cases
            .iter()
            .zip(decisions)
            .filter(|(record, _)| record.reflex == case.reflex)
            .map(|(_, decision)| decision)
            .collect();
        match filled(&sentence, &own) {
            Some(text) => filled_steps.push((case.clone(), text)),
            None => {
                if let Some((slot, _)) = sentence.slots().next() {
                    untested.push((case.clone(), slot.clone()));
                }
            }
        }
    }
    (filled_steps, untested)
}

/// The sentence a step case stands for, as the active playbook holds it.
fn sentence_of(session: &Session<'_>, case: &Case) -> Option<Sentence> {
    session
        .plan
        .active()
        .get(&case.reflex)?
        .steps
        .iter()
        .find(|sentence| Utterance::new(&sentence.to_string()).as_ref() == Ok(&case.utterance))
        .cloned()
}

/// Every case with its repeats: one that passed at the last run and failed now is decided twice more, all of
/// them at once, so the two-of-three rule reads them. A step is decided again on its filled text.
fn repeated(
    session: &Session<'_>,
    adapter: &dyn Adapter,
    before: &evoke_core::Baseline,
    all: Vec<(Case, Option<String>, Verdict)>,
) -> Result<Vec<(Case, NonEmpty<Verdict>)>, Exit> {
    let doubted: Vec<bool> = all
        .iter()
        .map(|(case, _, verdict)| {
            matches!(before.get(case), Some(Verdict::Pass))
                && matches!(verdict, Verdict::Fail { .. })
        })
        .collect();
    let doubtful: Vec<(&Case, String)> = all
        .iter()
        .zip(&doubted)
        .filter(|(_, doubted)| **doubted)
        .map(|((case, text, _), _)| {
            let text = text
                .clone()
                .unwrap_or_else(|| case.utterance.text().to_string());
            (case, text)
        })
        .collect();
    let mut again = threads::try_each(&doubtful, |(case, text)| {
        Ok([
            judge(case, &decided_once(session, adapter, text)?),
            judge(case, &decided_once(session, adapter, text)?),
        ])
    })?
    .into_iter();
    let mut judged = Vec::with_capacity(all.len());
    for ((case, _, verdict), doubted) in all.into_iter().zip(doubted) {
        let mut verdicts = vec![verdict];
        if doubted {
            verdicts.extend(again.next().expect("every doubtful case was decided again"));
        }
        let verdicts = NonEmpty::try_from(verdicts).expect("a case is judged at least once");
        judged.push((case, verdicts));
    }
    Ok(judged)
}

/// Each playbook's block for the report: its claim, and per step what became of it.
fn playbook_blocks(
    session: &Session<'_>,
    verdicts: &[(Case, Verdict)],
    routed: &[(Case, Decision)],
    untested: &[(Case, ArgName)],
    regressed: &[evoke_core::Regression],
) -> Vec<TestedPlaybook> {
    let mut blocks: Vec<TestedPlaybook> = Vec::new();
    for (name, active) in session.plan.active() {
        if active.steps.is_empty() {
            continue;
        }
        let mut steps = Vec::new();
        for (i, sentence) in active.steps.iter().enumerate() {
            let text = sentence.to_string();
            let is_step = |case: &Case| {
                case.reflex == *name
                    && case.from == Table::Steps
                    && case.utterance.text().as_str() == text
            };
            let became = if let Some((_, decision)) = routed.iter().find(|(case, _)| is_step(case))
            {
                match decision {
                    Decision::Abstain { .. } => StepBecame::NoReflex,
                    Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
                        routes_to(session, name, &chosen.call.reflex)
                    }
                    Decision::Ask { asking, .. } => routes_to(session, name, &asking.reflex),
                }
            } else if let Some((_, slot)) = untested.iter().find(|(case, _)| is_step(case)) {
                StepBecame::Untested { slot: slot.clone() }
            } else {
                continue;
            };
            let regression = regressed.iter().any(|regression| is_step(&regression.case));
            steps.push(TestedStep {
                n: i + 1,
                sentence: text,
                became,
                regression,
            });
        }
        branched(&session.plan, &active.steps, &mut steps);
        if steps.is_empty() || !verdicts.iter().any(|(case, _)| case.reflex == *name) {
            let judged = verdicts.iter().any(|(case, _)| case.reflex == *name);
            if steps.is_empty() && !judged {
                continue;
            }
        }
        blocks.push(TestedPlaybook {
            name: name.clone(),
            claim: active.effect,
            steps,
        });
    }
    blocks
}

/// A step that may not run needs the step before it — the nearest without `when` — to yield the field that picks
/// it as one value: a source that reaches a reflex yielding no such field is said so, and routes no more.
pub(super) fn branched(plan: &Plan, sentences: &[Sentence], steps: &mut [TestedStep]) {
    for (i, sentence) in sentences.iter().enumerate() {
        let Some(when) = sentence.when() else {
            continue;
        };
        let Some(source) = (0..i).rev().find(|&j| sentences[j].when().is_none()) else {
            continue;
        };
        let Some(step) = steps.iter_mut().find(|step| step.n == source + 1) else {
            continue;
        };
        let StepBecame::Routes { reflex, .. } = &step.became else {
            continue;
        };
        let yields = plan
            .active()
            .get(reflex)
            .and_then(|active| active.yields.get(&when.field));
        if !matches!(yields, Some(Yield::Kind(_))) {
            step.became = StepBecame::NoField {
                reflex: reflex.clone(),
                field: when.field.clone(),
            };
        }
    }
}

/// What a step routed to: a reflex with its effect, or the playbook it stands in.
fn routes_to(session: &Session<'_>, playbook: &LocalName, reflex: &LocalName) -> StepBecame {
    if reflex == playbook {
        return StepBecame::Nested;
    }
    let effect = session
        .plan
        .active()
        .get(reflex)
        .map_or(evoke_core::manifest::Effect::Destructive, |active| {
            active.effect
        });
    StepBecame::Routes {
        reflex: reflex.clone(),
        effect,
    }
}
