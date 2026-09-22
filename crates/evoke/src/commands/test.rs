//! `evoke test [name]`: every example and test of every active reflex — or of one — decided uncached over the
//! whole set, a few at a time, and judged on its route and asserted arguments; a case that passed at the last run
//! under this plan and fails now is decided twice more and is a regression when two of three fail; the baseline
//! kept per plan in the cache. In: a name or none, the environment. Out: `Exit`, a line per reflex and one per
//! failure; exit 1 when any case failed, with the count. Nothing runs and nothing is logged.

use evoke_core::name::LocalName;
use evoke_core::text::NonEmpty;
use evoke_core::{Case, Fix, Verdict, baseline, cases, judge, regressions};

use super::Exit;
use super::session::{self, Opening, Session};
use crate::adapter::Adapter;
use crate::args::Command;
use crate::hosts::{Environment, Failure, terminal, threads};
use crate::report;

pub fn run(command: &Command, name: Option<&LocalName>, environment: &Environment) -> Exit {
    let session = match session::open(command, false, environment, Opening::Deciding) {
        Ok(session) => session,
        Err(exit) => return exit,
    };
    let input = command.placeholder();
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
    if let Some(name) = name {
        session.plan.running(name).map_err(Exit::Human)?;
    }
    let active = session.plan.active();
    let cases: Vec<Case> = cases(&session.installed)
        .into_iter()
        .filter(|case| active.contains_key(&case.reflex))
        .filter(|case| name.is_none_or(|name| *name == case.reflex))
        .collect();
    let digest = session.plan.digest();
    let before = session
        .state
        .baseline(&digest)
        .map_err(Exit::Failed)?
        .unwrap_or_default();
    let judged = {
        let _busy = terminal::busy("testing");
        let first = threads::try_each(&cases, |case| judged_once(session, adapter, case))?;
        // A case that passed at the last run and failed now is decided twice more, all of them at once.
        let doubted: Vec<bool> = cases
            .iter()
            .zip(&first)
            .map(|(case, verdict)| {
                matches!(before.get(case), Some(Verdict::Pass))
                    && matches!(verdict, Verdict::Fail { .. })
            })
            .collect();
        let doubtful: Vec<&Case> = cases
            .iter()
            .zip(&doubted)
            .filter(|(_, doubted)| **doubted)
            .map(|(case, _)| case)
            .collect();
        let mut again = threads::try_each(&doubtful, |case| {
            Ok([
                judged_once(session, adapter, case)?,
                judged_once(session, adapter, case)?,
            ])
        })?
        .into_iter();
        drop(doubtful);
        let mut judged = Vec::with_capacity(cases.len());
        for ((case, verdict), doubted) in cases.into_iter().zip(first).zip(doubted) {
            let mut verdicts = vec![verdict];
            if doubted {
                verdicts.extend(again.next().expect("every doubtful case was decided again"));
            }
            let verdicts = NonEmpty::try_from(verdicts).expect("a case is judged at least once");
            judged.push((case, verdicts));
        }
        judged
    };
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
    if !verdicts.is_empty() {
        terminal::note(&report::tested(&verdicts, &regressed));
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

/// One uncached decision of the case's utterance, judged.
fn judged_once(session: &Session<'_>, adapter: &dyn Adapter, case: &Case) -> Result<Verdict, Exit> {
    let decided = session.decide_uncached(adapter, case.utterance.text().as_str(), &[])?;
    Ok(judge(case, &decided.decision))
}
