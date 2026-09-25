//! The gate measured over the records, and the log read as weak labels. Every input's decisions are judged against
//! every record of its utterance, as `test` judges them: the whole call right by bins of the confidence claimed,
//! the wrong calls at or over each bar per thousand with a bound, each bar's neighbourhood, each judgment on its
//! own by kind, Brier and its parts, the misses, and over repeats the spread, the flips and what moved. In: the
//! adapter's id and gate, the plan, cases with their repeated decisions; the log's lines. Out: `Calibration`,
//! `LogBlock`. Every number carries its count and its interval; a bin under a hundred calls is `thin`; nothing
//! depends on colour, and nothing here is I/O. The arithmetic is Wilson's interval, the one-sided Clopper-Pearson
//! bound by bisection over the binomial sum, and Murphy's parts of the Brier score.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::{Gate, Prob, QuestionId};
use crate::decide::{Decision, Judgment};
use crate::manifest::{Effect, Kind, Source};
use crate::name::{AdapterId, LocalName};
use crate::plan::Plan;
use crate::test::{Case, Claim, Expected, Mismatch, Verdict, judge};
use crate::text::{Identity, Input, NonEmpty};
use crate::weave::Status;

/// Under this many calls a bin proves nothing, and says so.
pub const THIN: usize = 100;

/// The 97.5th percentile of the standard normal: a two-sided interval at 95 %.
const Z: f64 = 1.959_964;
/// A one-sided bound at 95 %.
const ALPHA: f64 = 0.05;

/// The report over every record of the active reflexes, decided once or `repeats` times.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    pub adapter: AdapterId,
    pub records: usize,
    pub reflexes: usize,
    pub inputs: usize,
    pub repeats: usize,
    pub outcomes: Outcomes,
    /// The whole call right, by the confidence claimed; a bin with no call is left out.
    pub bins: Vec<BinRow>,
    /// Calls no record can judge: a `false` record routed to a reflex no record names.
    pub unknown: usize,
    pub abstained: Share,
    pub bars: Bars,
    pub questions: Vec<QuestionRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brier: Option<Brier>,
    pub misses: Vec<Miss>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variance: Option<Variance>,
}

/// How many inputs ended in each outcome.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcomes {
    pub run: usize,
    pub confirm: usize,
    pub ask: usize,
    pub abstain: usize,
}

/// One bin: the calls whose confidence lies in `lo..hi` — `hi` inside the last bin — how many were right, the
/// Wilson interval of that share, the mean confidence claimed; `over_confident` when the claim is above the
/// interval, `thin` under a hundred calls.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BinRow {
    pub lo: Prob,
    pub hi: Prob,
    pub calls: usize,
    pub right: usize,
    pub interval: (Prob, Prob),
    pub claimed: Prob,
    pub thin: bool,
    pub over_confident: bool,
}

/// A count with how many were right, and the Wilson interval of the share.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Share {
    pub count: usize,
    pub right: usize,
    pub interval: (Prob, Prob),
}

/// Each effect's bar, for an effect with a judged call under a gate.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Bars {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read: Option<BarRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write: Option<BarRow>,
}

/// The calls of one effect at or over its bar: how many were wrong, per thousand, and the one-sided bound at
/// 95 %, `at_most` per thousand; then the neighbourhood, at the bar and a step either side.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BarRow {
    pub bar: Prob,
    pub wrong: usize,
    pub calls: usize,
    pub per_thousand: f64,
    pub at_most: f64,
    pub near: Vec<NearRow>,
}

/// At a threshold: how many calls would run, and how many of those are wrong.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NearRow {
    pub at: Prob,
    pub run: usize,
    pub wrong: usize,
}

/// One kind of judgment on its own: the route, or the arguments by source; right when the record names the
/// argument and the decision read it as claimed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuestionRow {
    pub kind: QuestionKind,
    pub judgments: usize,
    pub right: usize,
    pub interval: (Prob, Prob),
    pub claimed: Prob,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuestionKind {
    Route,
    Options,
    Vocab,
    Pick,
    Flag,
}

/// The Brier score over the calls, and Murphy's parts over the bins.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Brier {
    pub brier: f64,
    pub reliability: f64,
    pub resolution: f64,
    pub uncertainty: f64,
}

/// A decision a record proved wrong: the record, what was decided, where it missed; `wrong` counts the repeats
/// that missed the same way.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Miss {
    pub case: Case,
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflex: Option<LocalName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Prob>,
    pub mismatch: Mismatch,
    pub wrong: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Run,
    Confirm,
    Ask,
    Abstain,
}

/// Over repeats: the inputs whose winner or verdict flipped, the spread of the confidence per input, the inputs
/// straddling the bar of their effect, the calls wrong at or over their bar in any repeat, and what moved most.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Variance {
    pub flips: usize,
    pub verdict_flips: usize,
    pub spread: Spread,
    pub straddling: usize,
    pub wrong_at_bar: usize,
    pub moved: Vec<Moved>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Spread {
    pub median: f64,
    pub p90: f64,
    pub max: f64,
}

/// One input whose repeats moved: the confidence's range, the route's, each winner and outcome with its count,
/// and how many repeats were wrong.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Moved {
    pub utterance: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<(Prob, Prob)>,
    pub route: (Prob, Prob),
    pub winners: IndexMap<String, usize>,
    pub outcomes: IndexMap<String, usize>,
    pub wrong: usize,
}

/// One line of the log as the block reads it: what was decided, which adapter answered — none when the cache did
/// — whether the body ran or failed, and a weave step's status.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Logged {
    pub input: Input,
    pub decision: Decision,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub adapters: Vec<AdapterId>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ran: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub failed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
}

/// The log's lines under one adapter, counted by what became of each; the confidence of what stopped at a confirm
/// by segment; the lines whose input is a record, judged; and the lines that did not read.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogBlock {
    pub adapter: AdapterId,
    pub decisions: usize,
    pub ran: usize,
    pub confirmed_ran: usize,
    pub confirmed_stopped: usize,
    pub asked: usize,
    pub abstained: usize,
    pub failed: usize,
    pub skipped: usize,
    pub stopped_by_confidence: Vec<Counted>,
    pub records: Share,
    pub unread: usize,
}

/// A count of confidences in `lo..hi`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Counted {
    pub lo: Prob,
    pub hi: Prob,
    pub count: usize,
}

/// The report: every case with its input's decisions — the cases of one input, by utterance identity, carry the
/// same decisions, and the first is read — judged and binned. Each input counts once, by its first decision;
/// the repeats measure stability, and add to the misses and to `variance`.
#[must_use]
pub fn calibrate(
    adapter: &AdapterId,
    gate: Option<&Gate>,
    plan: &Plan,
    decided: &[(Case, NonEmpty<Decision>)],
) -> Calibration {
    let inputs = grouped(decided);
    let judged: Vec<Vec<Judged<'_>>> = inputs
        .values()
        .map(|input| {
            input
                .decisions
                .iter()
                .map(|decision| Judged::of(&input.cases, decision, Some(plan)))
                .collect()
        })
        .collect();
    let first: Vec<&Judged<'_>> = judged
        .iter()
        .filter_map(|repeats| repeats.first())
        .collect();
    let repeats = judged.iter().map(Vec::len).max().unwrap_or(0);
    let calls: Vec<&Judged<'_>> = first.iter().copied().filter(|j| j.is_call()).collect();
    let known: Vec<(f64, bool)> = calls.iter().filter_map(|j| j.known()).collect();
    let bars = gate.map_or_else(Vec::new, floors);
    let edges = bins(&known.iter().map(|(p, _)| *p).collect::<Vec<_>>(), &bars);
    let mut reflexes: Vec<&LocalName> = decided.iter().map(|(case, _)| &case.reflex).collect();
    reflexes.sort();
    reflexes.dedup();
    let mut outcomes = Outcomes::default();
    for j in &first {
        match j.outcome {
            Outcome::Run => outcomes.run += 1,
            Outcome::Confirm => outcomes.confirm += 1,
            Outcome::Ask => outcomes.ask += 1,
            Outcome::Abstain => outcomes.abstain += 1,
        }
    }
    let abstains: Vec<&Judged<'_>> = first
        .iter()
        .copied()
        .filter(|j| j.outcome == Outcome::Abstain)
        .collect();
    Calibration {
        adapter: adapter.clone(),
        records: decided.len(),
        reflexes: reflexes.len(),
        inputs: inputs.len(),
        repeats,
        outcomes,
        bins: bin_rows(&known, &edges),
        unknown: calls.iter().filter(|j| j.truth.is_none()).count(),
        abstained: share(
            abstains.len(),
            abstains.iter().filter(|j| j.truth == Some(true)).count(),
        ),
        bars: Bars {
            read: gate.and_then(|gate| bar_row(&calls, Effect::Read, gate.read())),
            write: gate.and_then(|gate| bar_row(&calls, Effect::Write, gate.write())),
        },
        questions: question_rows(&first),
        brier: (!known.is_empty()).then(|| brier(&known, &edges)),
        misses: misses(&inputs, &judged),
        variance: (repeats > 1).then(|| variance(gate, &inputs, &judged)),
    }
}

/// The log's block: the lines under the adapter — a line the cache answered names none, and counts — by what
/// became of each; the confidence of what stopped at a confirm, by segment between the bars; the lines whose
/// input is a record, judged by every record of it. `unread` is what the host could not parse.
#[must_use]
pub fn log_block(
    adapter: &AdapterId,
    gate: Option<&Gate>,
    lines: &[Logged],
    cases: &[Case],
    unread: usize,
) -> LogBlock {
    let mut records: IndexMap<&Identity, Vec<&Case>> = IndexMap::new();
    for case in cases {
        records.entry(case.utterance.id()).or_default().push(case);
    }
    let mut block = LogBlock {
        adapter: adapter.clone(),
        decisions: 0,
        ran: 0,
        confirmed_ran: 0,
        confirmed_stopped: 0,
        asked: 0,
        abstained: 0,
        failed: 0,
        skipped: 0,
        stopped_by_confidence: Vec::new(),
        records: share(0, 0),
        unread,
    };
    let mut stopped = Vec::new();
    let mut judged = (0, 0);
    for line in lines {
        if !line.adapters.is_empty() && !line.adapters.contains(adapter) {
            continue;
        }
        block.decisions += 1;
        let outcome = outcome_of(&line.decision);
        let never_ran = matches!(
            line.status,
            Some(Status::Skipped | Status::Refused | Status::Unanswered)
        );
        match outcome {
            Outcome::Abstain => block.abstained += 1,
            _ if line.failed => block.failed += 1,
            _ if never_ran => block.skipped += 1,
            Outcome::Run if line.ran => block.ran += 1,
            Outcome::Confirm if line.ran => block.confirmed_ran += 1,
            Outcome::Run => block.skipped += 1,
            Outcome::Confirm => {
                block.confirmed_stopped += 1;
                stopped.extend(confidence_of(&line.decision));
            }
            Outcome::Ask => block.asked += 1,
        }
        if let Some(cases) = records.get(&crate::text::identity(line.input.as_str())) {
            let truth = Judged::of(cases, &line.decision, None).truth;
            match truth {
                Some(true) => judged = (judged.0 + 1, judged.1 + 1),
                Some(false) => judged.0 += 1,
                None => {}
            }
        }
    }
    let segments = segments(&gate.map_or_else(Vec::new, floors));
    block.stopped_by_confidence = segments
        .iter()
        .map(|&(lo, hi)| Counted {
            lo: prob(lo),
            hi: prob(hi),
            count: stopped.iter().filter(|p| holds(lo, hi, p.get())).count(),
        })
        .filter(|counted| counted.count > 0)
        .collect();
    block.records = share(judged.0, judged.1);
    block
}

impl Calibration {
    /// Why the report fails, when it does: a call wrong at or over its bar, in any repeat, or a bin of a hundred
    /// calls over-confident at 95 %. What a script gates on.
    #[must_use]
    pub fn failed(&self) -> Option<String> {
        let at_bar = self.variance.as_ref().map_or_else(
            || {
                [&self.bars.read, &self.bars.write]
                    .into_iter()
                    .flatten()
                    .map(|bar| bar.wrong)
                    .sum()
            },
            |variance| variance.wrong_at_bar,
        );
        let over: Vec<String> = self
            .bins
            .iter()
            .filter(|bin| !bin.thin && bin.over_confident)
            .map(|bin| format!("{:.2}–{:.2}", bin.lo.get(), bin.hi.get()))
            .collect();
        let mut reasons = Vec::new();
        match at_bar {
            0 => {}
            1 => reasons.push("1 call wrong at or over its bar".to_owned()),
            n => reasons.push(format!("{n} calls wrong at or over their bar")),
        }
        match over.as_slice() {
            [] => {}
            [one] => reasons.push(format!("the bin {one} is over-confident")),
            many => reasons.push(format!("the bins {} are over-confident", many.join(", "))),
        }
        (!reasons.is_empty()).then(|| reasons.join("; "))
    }
}

/// The records of one input with the decisions made of it.
struct Grouped<'a> {
    cases: Vec<&'a Case>,
    decisions: &'a NonEmpty<Decision>,
}

/// The cases by utterance identity, in the records' order; an input's decisions are its first case's.
fn grouped(decided: &[(Case, NonEmpty<Decision>)]) -> IndexMap<&Identity, Grouped<'_>> {
    let mut inputs: IndexMap<&Identity, Grouped<'_>> = IndexMap::new();
    for (case, decisions) in decided {
        inputs
            .entry(case.utterance.id())
            .or_insert_with(|| Grouped {
                cases: Vec::new(),
                decisions,
            })
            .cases
            .push(case);
    }
    inputs
}

/// One decision against every record of its input: the whole call's truth — wrong when any record says so,
/// right when any does and none says wrong, else unknown — the first miss, and each judgment's own truth.
struct Judged<'a> {
    outcome: Outcome,
    reflex: Option<&'a LocalName>,
    effect: Option<Effect>,
    confidence: Option<Prob>,
    judgments: Vec<&'a Judgment>,
    truth: Option<bool>,
    miss: Option<(&'a Case, Mismatch)>,
    route_truth: Option<bool>,
    /// Per argument judgment the records name: the kind of its source, and whether it read as claimed.
    args: Vec<(&'a Judgment, QuestionKind, bool)>,
}

impl<'a> Judged<'a> {
    fn of(cases: &[&'a Case], decision: &'a Decision, plan: Option<&Plan>) -> Self {
        let (reflex, args) = match decision {
            Decision::Abstain { .. } => (None, None),
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
                (Some(&chosen.call.reflex), Some(&chosen.call.args))
            }
            Decision::Ask { asking, .. } => (Some(&asking.reflex), Some(&asking.args)),
        };
        let judgments: Vec<&Judgment> = match decision {
            Decision::Abstain { judgments, .. } => judgments.iter().collect(),
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => chosen
                .judged
                .as_ref()
                .map_or_else(Vec::new, |judged| judged.judgments().iter().collect()),
            Decision::Ask { asking, .. } => asking.judged.judgments().iter().collect(),
        };
        let effect = match decision {
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => Some(chosen.effect),
            Decision::Ask { asking, .. } => plan
                .and_then(|plan| plan.active().get(&asking.reflex))
                .map(|active| active.effect),
            Decision::Abstain { .. } => None,
        };
        let mut truths = Vec::new();
        let mut route_truths = Vec::new();
        let mut miss = None;
        let mut named = Vec::new();
        for case in cases {
            let routed = reflex == Some(&case.reflex);
            let verdict = judge(case, decision);
            let (truth, route) = match (&case.expect, &verdict) {
                (Expected::Never, Verdict::Fail { .. }) => (Some(false), Some(false)),
                (Expected::Never, Verdict::Pass) => {
                    let truth = reflex.is_none().then_some(true);
                    (truth, truth)
                }
                (Expected::Asserts(_), Verdict::Pass) => (Some(true), Some(true)),
                (Expected::Asserts(_), Verdict::Fail { .. }) => (Some(false), Some(routed)),
            };
            truths.push(truth);
            route_truths.push(route);
            if let (None, Verdict::Fail { mismatch }) = (&miss, &verdict) {
                miss = Some((*case, mismatch.clone()));
            }
            if let (Expected::Asserts(claims), true, Some(plan)) = (&case.expect, routed, plan) {
                for (arg, claim) in claims {
                    let question = QuestionId::Arg(case.reflex.clone(), arg.clone());
                    let judgment = judgments.iter().copied().find(|j| j.question == question);
                    let kind = plan
                        .active()
                        .get(&case.reflex)
                        .and_then(|active| active.args.get(arg))
                        .map(|argument| kind_of(&argument.kind));
                    if let (Some(judgment), Some(kind)) = (judgment, kind) {
                        let read = Claim::read(args.and_then(|args| args.get(arg)));
                        named.push((judgment, kind, read == *claim));
                    }
                }
            }
        }
        Self {
            outcome: outcome_of(decision),
            reflex,
            effect,
            confidence: confidence_of(decision),
            judgments,
            truth: combined(&truths),
            miss,
            route_truth: combined(&route_truths),
            args: named,
        }
    }

    fn is_call(&self) -> bool {
        self.outcome != Outcome::Abstain
    }

    /// The confidence and the truth, when both are known.
    fn known(&self) -> Option<(f64, bool)> {
        Some((self.confidence?.get(), self.truth?))
    }

    /// Whether the call sits at or over the bar of its effect; none without a bar.
    fn over_bar(&self, gate: Option<&Gate>) -> Option<bool> {
        let bar = match self.effect? {
            Effect::Read => gate?.read(),
            Effect::Write => gate?.write(),
            Effect::Destructive => return None,
        };
        Some(self.confidence? >= bar)
    }

    fn route(&self) -> Option<Prob> {
        self.judgments
            .iter()
            .find(|j| j.question == QuestionId::Route)
            .map(|j| j.p)
    }
}

/// Several records' word on one call: wrong if any says wrong, right if any says right, else unknown.
fn combined(verdicts: &[Option<bool>]) -> Option<bool> {
    if verdicts.contains(&Some(false)) {
        Some(false)
    } else if verdicts.contains(&Some(true)) {
        Some(true)
    } else {
        None
    }
}

fn outcome_of(decision: &Decision) -> Outcome {
    match decision {
        Decision::Abstain { .. } => Outcome::Abstain,
        Decision::Run { .. } => Outcome::Run,
        Decision::Confirm { .. } => Outcome::Confirm,
        Decision::Ask { .. } => Outcome::Ask,
    }
}

fn confidence_of(decision: &Decision) -> Option<Prob> {
    match decision {
        Decision::Abstain { .. } => None,
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => chosen
            .judged
            .as_ref()
            .map(crate::decide::Judged::confidence),
        Decision::Ask { asking, .. } => Some(asking.judged.confidence()),
    }
}

/// The bars the weakest judgment is gated by, ascending: the route's floor and each effect's.
fn floors(gate: &Gate) -> Vec<f64> {
    let mut bars = vec![gate.route().get(), gate.read().get(), gate.write().get()];
    bars.sort_by(f64::total_cmp);
    bars.dedup();
    bars
}

/// The segments between the bars, `0..bar_1`, …, `bar_n..1`.
fn segments(bars: &[f64]) -> Vec<(f64, f64)> {
    let mut edges = vec![0.0];
    edges.extend(bars.iter().copied().filter(|bar| *bar > 0.0 && *bar < 1.0));
    edges.push(1.0);
    edges.windows(2).map(|pair| (pair[0], pair[1])).collect()
}

/// Whether `p` lies in `lo..hi`; 1 lies in the last segment.
fn holds(lo: f64, hi: f64, p: f64) -> bool {
    lo <= p && (p < hi || (hi >= 1.0 && p <= 1.0))
}

/// Equal-mass bins with fixed edges at the bars: each segment between bars is one bin, or split at its quantiles
/// into as many bins of about a hundred as it holds, so a bin never straddles a bar.
fn bins(values: &[f64], bars: &[f64]) -> Vec<(f64, f64)> {
    let mut out = Vec::new();
    for (lo, hi) in segments(bars) {
        let mut inside: Vec<f64> = values
            .iter()
            .copied()
            .filter(|p| holds(lo, hi, *p))
            .collect();
        inside.sort_by(f64::total_cmp);
        let parts = (inside.len() / THIN).max(1);
        let mut cuts = vec![lo];
        for i in 1..parts {
            let cut = inside[inside.len() * i / parts];
            if cut > *cuts.last().expect("a cut") && cut < hi {
                cuts.push(cut);
            }
        }
        cuts.push(hi);
        out.extend(cuts.windows(2).map(|pair| (pair[0], pair[1])));
    }
    out
}

fn bin_rows(known: &[(f64, bool)], edges: &[(f64, f64)]) -> Vec<BinRow> {
    edges
        .iter()
        .filter_map(|&(lo, hi)| {
            let inside: Vec<&(f64, bool)> =
                known.iter().filter(|(p, _)| holds(lo, hi, *p)).collect();
            if inside.is_empty() {
                return None;
            }
            let calls = inside.len();
            let right = inside.iter().filter(|(_, right)| *right).count();
            let claimed = mean(inside.iter().map(|(p, _)| *p));
            let interval = wilson(right, calls);
            Some(BinRow {
                lo: prob(lo),
                hi: prob(hi),
                calls,
                right,
                interval,
                claimed: prob(claimed),
                thin: calls < THIN,
                over_confident: claimed > interval.1.get(),
            })
        })
        .collect()
}

fn share(count: usize, right: usize) -> Share {
    Share {
        count,
        right,
        interval: wilson(right, count),
    }
}

/// The calls of one effect at or over its bar, and the neighbourhood; none when the effect has no judged call.
fn bar_row(calls: &[&Judged<'_>], effect: Effect, bar: Prob) -> Option<BarRow> {
    let pairs: Vec<(f64, bool)> = calls
        .iter()
        .filter(|j| j.effect == Some(effect))
        .filter_map(|j| j.known())
        .collect();
    if pairs.is_empty() {
        return None;
    }
    let count = |at: f64| -> (usize, usize) {
        let over: Vec<bool> = pairs
            .iter()
            .filter(|(p, _)| *p >= at)
            .map(|(_, right)| *right)
            .collect();
        (over.len(), over.iter().filter(|right| !**right).count())
    };
    let (n, wrong) = count(bar.get());
    #[expect(clippy::cast_precision_loss)]
    let per_thousand = if n == 0 {
        0.0
    } else {
        1000.0 * wrong as f64 / n as f64
    };
    let hundredths = (bar.get() * 100.0).round();
    let near = [-10.0, -5.0, 0.0, 5.0, 10.0]
        .into_iter()
        .map(|step| (hundredths + step) / 100.0)
        .filter(|at| (0.0..=1.0).contains(at))
        .map(|at| {
            let (run, wrong) = count(at);
            NearRow {
                at: prob(at),
                run,
                wrong,
            }
        })
        .collect();
    Some(BarRow {
        bar,
        wrong,
        calls: n,
        per_thousand,
        at_most: 1000.0 * upper_bound(wrong, n),
        near,
    })
}

/// Each judgment on its own: the route by its truth over the records, then the arguments the records name, by
/// the kind of their source.
fn question_rows(first: &[&Judged<'_>]) -> Vec<QuestionRow> {
    let mut rows: IndexMap<QuestionKind, Vec<(f64, bool)>> = IndexMap::new();
    for kind in [
        QuestionKind::Route,
        QuestionKind::Options,
        QuestionKind::Vocab,
        QuestionKind::Pick,
        QuestionKind::Flag,
    ] {
        rows.insert(kind, Vec::new());
    }
    for j in first {
        if let (Some(p), Some(right)) = (j.route(), j.route_truth) {
            rows[&QuestionKind::Route].push((p.get(), right));
        }
        for (judgment, kind, right) in &j.args {
            rows[kind].push((judgment.p.get(), *right));
        }
    }
    rows.into_iter()
        .filter(|(_, pairs)| !pairs.is_empty())
        .map(|(kind, pairs)| {
            let right = pairs.iter().filter(|(_, right)| *right).count();
            QuestionRow {
                kind,
                judgments: pairs.len(),
                right,
                interval: wilson(right, pairs.len()),
                claimed: prob(mean(pairs.iter().map(|(p, _)| *p))),
            }
        })
        .collect()
}

/// The kind of an argument's source, as the question rows group them.
fn kind_of(kind: &Kind) -> QuestionKind {
    match kind {
        Kind::Flag => QuestionKind::Flag,
        Kind::Value {
            source: Source::Options(_),
            ..
        } => QuestionKind::Options,
        Kind::Value {
            source: Source::Vocab(_),
            ..
        } => QuestionKind::Vocab,
        Kind::Value {
            source: Source::Pick(_),
            ..
        } => QuestionKind::Pick,
    }
}

/// The Brier score and Murphy's parts over the bins: reliability, resolution, uncertainty.
#[expect(clippy::cast_precision_loss)]
fn brier(known: &[(f64, bool)], edges: &[(f64, f64)]) -> Brier {
    let n = known.len() as f64;
    let score = known
        .iter()
        .map(|(p, right)| (p - f64::from(u8::from(*right))).powi(2))
        .sum::<f64>()
        / n;
    let base = known.iter().filter(|(_, right)| *right).count() as f64 / n;
    let mut reliability = 0.0;
    let mut resolution = 0.0;
    for &(lo, hi) in edges {
        let inside: Vec<&(f64, bool)> = known.iter().filter(|(p, _)| holds(lo, hi, *p)).collect();
        if inside.is_empty() {
            continue;
        }
        let k = inside.len() as f64;
        let claimed = mean(inside.iter().map(|(p, _)| *p));
        let right = inside.iter().filter(|(_, right)| *right).count() as f64 / k;
        reliability += k * (claimed - right).powi(2) / n;
        resolution += k * (right - base).powi(2) / n;
    }
    Brier {
        brier: score,
        reliability,
        resolution,
        uncertainty: base * (1.0 - base),
    }
}

/// Every decision that a record proved wrong, once per record and mismatch, with how many repeats missed so.
fn misses(inputs: &IndexMap<&Identity, Grouped<'_>>, judged: &[Vec<Judged<'_>>]) -> Vec<Miss> {
    let mut misses: Vec<Miss> = Vec::new();
    for (input, repeats) in inputs.values().zip(judged) {
        for j in repeats {
            let Some((case, mismatch)) = &j.miss else {
                continue;
            };
            let same = misses.iter_mut().find(|miss| {
                miss.case.reflex == case.reflex
                    && miss.case.utterance.id() == input.cases[0].utterance.id()
                    && miss.mismatch == *mismatch
            });
            match same {
                Some(miss) => miss.wrong += 1,
                None => misses.push(Miss {
                    case: (*case).clone(),
                    outcome: j.outcome,
                    reflex: j.reflex.cloned(),
                    confidence: j.confidence,
                    mismatch: mismatch.clone(),
                    wrong: 1,
                }),
            }
        }
    }
    misses
}

/// The variance over repeats, per input: the winner's flip, the verdict's, the confidence's spread, the bar
/// straddled; then the inputs that moved most, by spread, at most a dozen.
fn variance(
    gate: Option<&Gate>,
    inputs: &IndexMap<&Identity, Grouped<'_>>,
    judged: &[Vec<Judged<'_>>],
) -> Variance {
    let mut flips = 0;
    let mut verdict_flips = 0;
    let mut straddling = 0;
    let mut wrong_at_bar = 0;
    let mut spreads = Vec::new();
    let mut moved = Vec::new();
    for (input, repeats) in inputs.values().zip(judged) {
        let mut winners: IndexMap<String, usize> = IndexMap::new();
        let mut outcomes: IndexMap<String, usize> = IndexMap::new();
        for j in repeats {
            let winner = j
                .reflex
                .map_or_else(|| "none".to_owned(), ToString::to_string);
            *winners.entry(winner).or_default() += 1;
            *outcomes.entry(word(j.outcome).to_owned()).or_default() += 1;
        }
        let verdicts: Vec<Option<bool>> = repeats.iter().map(|j| j.truth).collect();
        let flipped = winners.len() > 1;
        let verdict_flipped = verdicts.windows(2).any(|pair| pair[0] != pair[1]);
        let confidences: Vec<f64> = repeats
            .iter()
            .filter_map(|j| j.confidence.map(Prob::get))
            .collect();
        let routes: Vec<f64> = repeats
            .iter()
            .filter_map(|j| j.route().map(Prob::get))
            .collect();
        let over: Vec<bool> = repeats.iter().filter_map(|j| j.over_bar(gate)).collect();
        let straddles = over.contains(&true) && over.contains(&false);
        let wrong = repeats.iter().filter(|j| j.truth == Some(false)).count();
        wrong_at_bar += repeats
            .iter()
            .filter(|j| j.truth == Some(false) && j.over_bar(gate) == Some(true))
            .count();
        let spread = range(&confidences).map_or(0.0, |(lo, hi)| hi - lo);
        if !confidences.is_empty() {
            spreads.push(spread);
        }
        flips += usize::from(flipped);
        verdict_flips += usize::from(verdict_flipped);
        straddling += usize::from(straddles);
        if flipped || verdict_flipped || spread > 0.0 {
            moved.push((
                spread,
                Moved {
                    utterance: input.cases[0].utterance.text().to_string(),
                    confidence: range(&confidences).map(|(lo, hi)| (prob(lo), prob(hi))),
                    route: range(&routes)
                        .map_or((Prob::ZERO, Prob::ZERO), |(lo, hi)| (prob(lo), prob(hi))),
                    winners,
                    outcomes,
                    wrong,
                },
            ));
        }
    }
    moved.sort_by(|a, b| b.0.total_cmp(&a.0));
    spreads.sort_by(f64::total_cmp);
    Variance {
        flips,
        verdict_flips,
        spread: Spread {
            median: median(&spreads),
            p90: percentile(&spreads, 0.9),
            max: spreads.last().copied().unwrap_or(0.0),
        },
        straddling,
        wrong_at_bar,
        moved: moved.into_iter().take(12).map(|(_, moved)| moved).collect(),
    }
}

/// The outcome's word.
#[must_use]
pub fn word(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Run => "run",
        Outcome::Confirm => "confirm",
        Outcome::Ask => "ask",
        Outcome::Abstain => "abstain",
    }
}

fn range(values: &[f64]) -> Option<(f64, f64)> {
    let lo = values.iter().copied().min_by(f64::total_cmp)?;
    let hi = values.iter().copied().max_by(f64::total_cmp)?;
    Some((lo, hi))
}

/// The median of sorted values; 0 of none.
fn median(sorted: &[f64]) -> f64 {
    let n = sorted.len();
    if n == 0 {
        0.0
    } else if n % 2 == 1 {
        sorted[n / 2]
    } else {
        f64::midpoint(sorted[n / 2 - 1], sorted[n / 2])
    }
}

/// The value at a share of sorted values, the index floored; 0 of none.
#[expect(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn percentile(sorted: &[f64], share: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[(share * (sorted.len() - 1) as f64) as usize]
}

#[expect(clippy::cast_precision_loss)]
fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, n) = values.fold((0.0, 0usize), |(sum, n), p| (sum + p, n + 1));
    if n == 0 { 0.0 } else { sum / n as f64 }
}

/// A number in `[0, 1]` as a probability, rounding error clamped.
fn prob(p: f64) -> Prob {
    Prob::new(p.clamp(0.0, 1.0)).expect("a share is a probability")
}

/// The Wilson interval at 95 % of `k` right in `n`; `(0, 0)` of none.
#[expect(clippy::cast_precision_loss)]
fn wilson(k: usize, n: usize) -> (Prob, Prob) {
    if n == 0 {
        return (Prob::ZERO, Prob::ZERO);
    }
    let (k, n) = (k as f64, n as f64);
    let p = k / n;
    let d = 1.0 + Z * Z / n;
    let centre = (p + Z * Z / (2.0 * n)) / d;
    let half = Z * (p * (1.0 - p) / n + Z * Z / (4.0 * n * n)).sqrt() / d;
    (prob(centre - half), prob(centre + half))
}

/// The one-sided Clopper-Pearson bound at 95 %: the largest rate under which `k` or fewer wrong in `n` has
/// probability `ALPHA`, by bisection over the binomial sum; 1 of none, or of all wrong.
fn upper_bound(k: usize, n: usize) -> f64 {
    if n == 0 || k >= n {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if at_most(k, n, mid) > ALPHA {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    hi
}

/// P(X ≤ k) for X binomial over `n` at `p`, each term from the one before, so every platform sums the same.
#[expect(clippy::cast_precision_loss)]
fn at_most(k: usize, n: usize, p: f64) -> f64 {
    let mut term = (1.0 - p).powi(i32::try_from(n).unwrap_or(i32::MAX));
    let mut sum = term;
    for i in 0..k {
        term *= (n - i) as f64 / (i + 1) as f64 * p / (1.0 - p);
        sum += term;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wilson_and_the_bound_meet_the_known_cases() {
        let (lo, hi) = wilson(8, 10);
        assert!((lo.get() - 0.4902).abs() < 0.001 && (hi.get() - 0.9433).abs() < 0.001);
        let (lo, hi) = wilson(50, 100);
        assert!((lo.get() - 0.4038).abs() < 0.001 && (hi.get() - 0.5962).abs() < 0.001);
        assert_eq!(wilson(0, 0), (Prob::ZERO, Prob::ZERO));
        assert!((1000.0 * upper_bound(0, 300) - 9.937).abs() < 0.01);
        assert!((1000.0 * upper_bound(0, 3000) - 0.9985).abs() < 0.001);
        assert!((upper_bound(1, 100) - 0.046_56).abs() < 0.0005);
        assert!((upper_bound(1, 2) - 0.975).abs() < 0.001);
        assert!((upper_bound(5, 5) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn the_bins_have_fixed_edges_and_split_at_a_hundred() {
        let bars = [0.5, 0.6, 0.8];
        assert_eq!(
            bins(&[0.1, 0.55, 0.65, 0.9, 1.0], &bars),
            vec![(0.0, 0.5), (0.5, 0.6), (0.6, 0.8), (0.8, 1.0)]
        );
        assert!(holds(0.8, 1.0, 1.0) && holds(0.8, 1.0, 0.8) && !holds(0.8, 1.0, 0.79));
        let many: Vec<f64> = (0..400).map(|i| 0.8 + 0.2 * f64::from(i) / 400.0).collect();
        let top: Vec<(f64, f64)> = bins(&many, &bars)
            .into_iter()
            .filter(|(lo, _)| *lo >= 0.8)
            .collect();
        assert_eq!(top.len(), 4);
        for (lo, hi) in top {
            let inside = many.iter().filter(|p| holds(lo, hi, **p)).count();
            assert!((99..=101).contains(&inside), "{inside} in {lo}..{hi}");
        }
        assert_eq!(bins(&[], &[]), vec![(0.0, 1.0)]);
    }

    #[test]
    fn brier_decomposes_as_murphy_says() {
        let constant: Vec<(f64, bool)> = (0..10).map(|i| (0.8, i < 7)).collect();
        let parts = brier(&constant, &[(0.0, 1.0)]);
        assert!((parts.brier - 0.22).abs() < 1e-9);
        assert!((parts.reliability - 0.01).abs() < 1e-9);
        assert!(parts.resolution.abs() < 1e-9);
        assert!((parts.uncertainty - 0.21).abs() < 1e-9);
        let mut perfect = vec![(1.0, true); 5];
        perfect.extend(vec![(0.0, false); 5]);
        let parts = brier(&perfect, &[(0.0, 0.5), (0.5, 1.0)]);
        assert!(parts.brier.abs() < f64::EPSILON);
        assert!((parts.resolution - 0.25).abs() < 1e-9);
    }

    #[test]
    fn the_middle_values_of_a_spread() {
        assert!((median(&[0.0, 0.1, 0.3]) - 0.1).abs() < f64::EPSILON);
        assert!((median(&[0.0, 0.2]) - 0.1).abs() < f64::EPSILON);
        assert!((percentile(&[0.0, 0.1, 0.2, 0.3, 0.4], 0.9) - 0.3).abs() < f64::EPSILON);
        assert!(median(&[]).abs() < f64::EPSILON && percentile(&[], 0.9).abs() < f64::EPSILON);
    }
}
