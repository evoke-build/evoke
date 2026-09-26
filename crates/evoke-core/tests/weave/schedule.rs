//! The schedule under any plan, against the core's own planner: requests of one to seven parts joined by `and`
//! and `then`, each part a decision the spec's fixtures made, every reflex under an effect the case chose, the
//! engine's judgment of every split point generated — through `weave::planning::plan` as it is; and, over the
//! joins plan, requests of lookups and suspects, every suspect bound by name to the one lookup before it that
//! returns each name, or refused when none does, or two. Each invariant quotes the sentence of the design it
//! checks.

use super::reference::{abstain, decision, exclusive, joins, plan_with, stages};
use evoke_core::Plan;
use evoke_core::adapter::Raw;
use evoke_core::manifest::Effect;
use evoke_core::name::LocalName;
use evoke_core::weave::planning;
use evoke_core::weave::{Answers, Because, Need, Order, Outcome, Planning, Via, Weave};
use indexmap::IndexMap;
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::test_runner::FileFailurePersistence;

/// A part of a request: which reflex decides it, in words that carry no connective, pronoun or determiner, so the
/// reading finds nothing but the parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Lights,
    Timer,
    Contact,
}

impl Part {
    fn text(self) -> &'static str {
        match self {
            Self::Lights => "lights off",
            Self::Timer => "timer 10 minutes",
            Self::Contact => "contact dana",
        }
    }

    fn reflex(self) -> &'static str {
        match self {
            Self::Lights => "lights",
            Self::Timer => "timer",
            Self::Contact => "contact",
        }
    }

    fn of_text(text: &str) -> Self {
        [Self::Lights, Self::Timer, Self::Contact]
            .into_iter()
            .find(|part| part.text() == text)
            .unwrap_or_else(|| panic!("no part reads «{text}»"))
    }
}

/// A connective between two parts: `then` orders, `and` coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Join {
    And,
    Then,
    CommaThen,
    AndThen,
}

impl Join {
    fn words(self) -> &'static str {
        match self {
            Self::And => " and ",
            Self::Then => " then ",
            Self::CommaThen => ", then ",
            Self::AndThen => " and then ",
        }
    }
}

/// A request: its parts, the connectives between them, the engine's judgment of each split point, and the
/// effect each reflex runs under.
#[derive(Clone, Debug)]
struct Request {
    parts: Vec<Part>,
    joins: Vec<Join>,
    judged: Vec<f64>,
    effects: [Effect; 3],
}

impl Request {
    fn text(&self) -> String {
        let mut text = self.parts[0].text().to_owned();
        for (join, part) in self.joins.iter().zip(&self.parts[1..]) {
            text.push_str(join.words());
            text.push_str(part.text());
        }
        text
    }

    fn effect(&self, part: Part) -> Effect {
        match part {
            Part::Lights => self.effects[0],
            Part::Timer => self.effects[1],
            Part::Contact => self.effects[2],
        }
    }
}

fn part() -> impl Strategy<Value = Part> {
    prop_oneof![Just(Part::Lights), Just(Part::Timer), Just(Part::Contact)]
}

fn join() -> impl Strategy<Value = Join> {
    prop_oneof![
        3 => Just(Join::And),
        2 => Just(Join::Then),
        1 => Just(Join::CommaThen),
        1 => Just(Join::AndThen)
    ]
}

fn effect() -> impl Strategy<Value = Effect> {
    prop_oneof![
        Just(Effect::Read),
        Just(Effect::Write),
        Just(Effect::Destructive)
    ]
}

/// The engine's judgment of a split point: mostly firm, some between, a few doubted — never under the floor the
/// planner tries from, so every part is a step.
fn judgment() -> impl Strategy<Value = f64> {
    prop_oneof![6 => 0.8..=1.0, 3 => 0.5..0.8, 1 => 0.35..0.5]
}

fn request() -> impl Strategy<Value = Request> {
    (1usize..=7).prop_flat_map(|n| {
        (
            vec(part(), n),
            vec(join(), n - 1),
            vec(judgment(), n - 1),
            (effect(), effect(), effect()),
        )
            .prop_map(|(parts, joins, judged, (lights, timer, contact))| Request {
                parts,
                joins,
                judged,
                effects: [lights, timer, contact],
            })
    })
}

/// The request through the core's planner, every need answered: the judgments as generated, each part's
/// decision from the fixtures, and any text decided narrowed to one reflex — a fan-out tried on a doubted part —
/// matching nothing, so every part stays the step it is.
fn planned(request: &Request) -> Weave {
    let plan = plan_with(&[
        ("lights", request.effects[0]),
        ("timer", request.effects[1]),
        ("contact", request.effects[2]),
    ]);
    planned_under(&plan, &request.text(), &request.judged, |text| {
        decision(Part::of_text(text).reflex())
    })
}

/// A request through the planner under a plan, its needs answered: the judgments as given, each part decided
/// by `decide`, a text narrowed to one reflex matching nothing.
fn planned_under(
    plan: &Plan,
    input: &str,
    judged: &[f64],
    decide: impl Fn(&str) -> evoke_core::Decision,
) -> Weave {
    let mut answers = Answers::default();
    loop {
        match planning::plan(plan, input, &[], &answers).expect("the answers validate") {
            Planning::Done { weave } => return weave,
            Planning::Need { need } => match need {
                Need::Judge { request: judge } => {
                    let mut raw = IndexMap::new();
                    for (n, id) in judge.questions.keys().enumerate() {
                        let mut answer = IndexMap::new();
                        answer.insert("yes".to_owned(), judged[n]);
                        raw.insert(id.to_string(), answer);
                    }
                    answers.judged = Some(Raw(raw));
                }
                Need::Refer { .. } => panic!("no part refers back"),
                Need::Decide { asked } => {
                    for asked in asked {
                        let decided = if asked.only.is_some() {
                            abstain()
                        } else {
                            decide(&asked.text)
                        };
                        answers.decided.push((asked, decided));
                    }
                }
            },
        }
    }
}

/// A part of a request over the joins plan: a lookup that returns its name, or the suspect that takes the three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lookup {
    Errors,
    Deploys,
    Logs,
    Suspect,
}

impl Lookup {
    fn text(self) -> &'static str {
        match self {
            Self::Errors => "errors checkout",
            Self::Deploys => "deploys checkout",
            Self::Logs => "logs checkout",
            Self::Suspect => "suspect",
        }
    }

    fn reflex(self) -> &'static str {
        match self {
            Self::Errors => "errors",
            Self::Deploys => "deploys",
            Self::Logs => "logs",
            Self::Suspect => "suspect",
        }
    }

    fn of_text(text: &str) -> Self {
        [Self::Errors, Self::Deploys, Self::Logs, Self::Suspect]
            .into_iter()
            .find(|part| part.text() == text)
            .unwrap_or_else(|| panic!("no part reads «{text}»"))
    }
}

/// A request over the joins plan: lookups and suspects, joined, judged.
#[derive(Clone, Debug)]
struct Joined {
    parts: Vec<Lookup>,
    joins: Vec<Join>,
    judged: Vec<f64>,
}

impl Joined {
    fn text(&self) -> String {
        let mut text = self.parts[0].text().to_owned();
        for (join, part) in self.joins.iter().zip(&self.parts[1..]) {
            text.push_str(join.words());
            text.push_str(part.text());
        }
        text
    }
}

fn lookup() -> impl Strategy<Value = Lookup> {
    prop_oneof![
        2 => Just(Lookup::Errors),
        2 => Just(Lookup::Deploys),
        2 => Just(Lookup::Logs),
        3 => Just(Lookup::Suspect)
    ]
}

/// One to six parts drawn freely; or, a third of the time, the outage's own shape — the three lookups in some
/// order, then the suspect, then what follows — so a suspect bound over all three is drawn often.
fn parts() -> impl Strategy<Value = Vec<Lookup>> {
    prop_oneof![
        2 => vec(lookup(), 1..=6),
        1 => (0u8..6, vec(lookup(), 0..=2)).prop_map(|(order, rest)| {
            let mut three = vec![Lookup::Errors, Lookup::Deploys, Lookup::Logs];
            let first = three.remove(usize::from(order) % 3);
            let second = three.remove(usize::from(order / 3) % 2);
            let mut parts = vec![first, second, three[0], Lookup::Suspect];
            parts.extend(rest);
            parts
        }),
    ]
}

fn joined() -> impl Strategy<Value = Joined> {
    parts().prop_flat_map(|parts| {
        let n = parts.len();
        (Just(parts), vec(join(), n - 1), vec(judgment(), n - 1)).prop_map(
            |(parts, joins, judged)| Joined {
                parts,
                joins,
                judged,
            },
        )
    })
}

/// «A taker's argument is bound to the one step before it whose reflex returns that name … none, two, or one
/// step run once per record: the request is refused before anything runs.» Per suspect, per name: the one
/// source, or the refusal the plan must carry.
fn bound_by_name(parts: &[Lookup]) -> (Vec<(usize, usize, &'static str)>, Vec<Because>) {
    let mut binds = Vec::new();
    let mut refusals = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        if *part != Lookup::Suspect {
            continue;
        }
        let step = i + 1;
        for (name, returner) in [
            ("errors", Lookup::Errors),
            ("deploys", Lookup::Deploys),
            ("logs", Lookup::Logs),
        ] {
            let sources: Vec<usize> = parts[..i]
                .iter()
                .enumerate()
                .filter(|(_, p)| **p == returner)
                .map(|(j, _)| j + 1)
                .collect();
            let name = evoke_core::name::FieldName::new(name).expect("a name");
            match sources.as_slice() {
                [] => refusals.push(Because::NoSource { step, name }),
                [source] => binds.push((*source, step, name.as_str().to_owned().leak() as &str)),
                _ => refusals.push(Because::SeveralSources {
                    step,
                    name,
                    sources,
                }),
            }
        }
    }
    (binds, refusals)
}

/// «`then` … order[s] the steps»: everything before a `then` before everything after it, and nothing else
/// orders, since no part refers back. Per step, the steps it follows, sorted.
fn ordered_by_then(weave: &Weave) -> Vec<Vec<usize>> {
    let mut after: Vec<Vec<usize>> = vec![Vec::new(); weave.steps.len()];
    for split in weave
        .splits
        .iter()
        .filter(|split| split.order == Order::Then)
    {
        let before: Vec<usize> = weave
            .steps
            .iter()
            .filter(|step| step.end <= split.start)
            .map(|step| step.n)
            .collect();
        for step in weave.steps.iter().filter(|step| !before.contains(&step.n)) {
            for b in &before {
                if !after[step.n - 1].contains(b) {
                    after[step.n - 1].push(*b);
                }
            }
        }
    }
    for edges in &mut after {
        edges.sort_unstable();
    }
    after
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 1024, failure_persistence: Some(Box::new(FileFailurePersistence::Direct("proptest-regressions/weave/schedule.txt"))), ..ProptestConfig::default() })]

    #[test]
    fn the_schedule_is_the_designs_under_any_plan(request in request()) {
        let weave = planned(&request);
        // «Every sentence is read for its steps»: one step per part, in the words' order, decided as its reflex.
        prop_assert_eq!(weave.steps.len(), request.parts.len());
        for (step, part) in weave.steps.iter().zip(&request.parts) {
            prop_assert_eq!(step.text.as_str(), part.text());
            prop_assert_eq!(step.reflex.as_ref().map(LocalName::as_str), Some(part.reflex()));
            prop_assert_eq!(step.effect, Some(request.effect(*part)));
        }
        // «`then` and a reference order the steps.»
        let after = ordered_by_then(&weave);
        for step in &weave.steps {
            prop_assert_eq!(&step.after, &after[step.n - 1], "step {}'s edges", step.n);
        }
        let effects: Vec<Effect> = weave.steps.iter().map(|step| step.effect.expect("every step has an effect")).collect();
        // «A write never runs beside anything, reads may.»
        prop_assert_eq!(weave.exclusive, exclusive(&effects));
        prop_assert_eq!(&weave.stages, &stages(&effects, &after));
        for stage in &weave.stages {
            if stage.len() > 1 {
                prop_assert!(stage.iter().all(|n| effects[n - 1] == Effect::Read), "a stage of several holds a write: {stage:?}");
            }
        }
        // A step's stage comes after the stages of every step it follows.
        let stage_of = |n: usize| weave.stages.iter().position(|stage| stage.contains(&n)).expect("every step is staged");
        for step in &weave.steps {
            for earlier in &step.after {
                prop_assert!(stage_of(*earlier) < stage_of(step.n), "step {} is staged before step {}, which it follows", step.n, earlier);
            }
        }
        // Every step in exactly one stage.
        let mut staged: Vec<usize> = weave.stages.iter().flatten().copied().collect();
        staged.sort_unstable();
        prop_assert_eq!(staged, (1..=weave.steps.len()).collect::<Vec<_>>());
        // «The verdict — run, ask, confirm, refuse — stands before anything runs»: every part decided, it runs.
        prop_assert_eq!(weave.verdict.outcome, Outcome::Run);
    }

    /// «Dependencies come from words or a declared name»: over the joins plan, every suspect takes each of the
    /// three names from the one lookup before it that returns it, the edges and the stages following; none, or
    /// two, refuses the plan before anything runs, and the words never choose.
    #[test]
    fn joins_bind_by_name_under_any_plan(request in joined()) {
        let plan = joins();
        let weave = planned_under(&plan, &request.text(), &request.judged, |text| decision(Lookup::of_text(text).reflex()));
        prop_assert_eq!(weave.steps.len(), request.parts.len());
        for (step, part) in weave.steps.iter().zip(&request.parts) {
            prop_assert_eq!(step.reflex.as_ref().map(LocalName::as_str), Some(part.reflex()));
        }
        let (binds, refusals) = bound_by_name(&request.parts);
        // Every binding is a whole result by name, `takes`, and none carries a kind or a list.
        let planned: Vec<(usize, usize, &str)> = weave.binds.iter().map(|b| {
            prop_assert_eq!(b.via, Via::Takes);
            prop_assert!(b.kind.is_none() && b.each.is_none());
            prop_assert_eq!(b.arg.as_str(), b.field.as_str(), "the suspect's arguments are named as the results are");
            Ok((b.from, b.to, b.field.as_str()))
        }).collect::<Result<_, _>>()?;
        prop_assert_eq!(&planned, &binds);
        // «`then` and a reference order the steps», and so does a name: a suspect follows every source it takes.
        let mut after = ordered_by_then(&weave);
        for (from, to, _) in &binds {
            if !after[to - 1].contains(from) {
                after[to - 1].push(*from);
            }
        }
        for edges in &mut after {
            edges.sort_unstable();
        }
        for step in &weave.steps {
            prop_assert_eq!(&step.after, &after[step.n - 1], "step {}'s edges", step.n);
        }
        // Reads all: the stages are layers by longest path, together.
        let effects: Vec<Effect> = weave.steps.iter().map(|step| step.effect.expect("every step has an effect")).collect();
        prop_assert!(!weave.exclusive);
        prop_assert_eq!(&weave.stages, &stages(&effects, &after));
        // The verdict: refused for exactly the reasons the names give, in the steps' order; else it runs.
        let carried: Vec<&Because> = weave.verdict.because.iter().filter(|b| matches!(b, Because::NoSource { .. } | Because::SeveralSources { .. })).collect();
        prop_assert_eq!(carried, refusals.iter().collect::<Vec<_>>());
        prop_assert_eq!(weave.verdict.outcome, if refusals.is_empty() { Outcome::Run } else { Outcome::Refuse });
    }
}

/// The joins generator draws the shapes its invariants speak of: a suspect bound over all three, one refused for
/// want of a source, one refused over two sources, and a chain of two suspects.
#[test]
fn the_joins_generator_reaches_every_shape() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let mut seen = [0usize; 4];
    for _ in 0..300 {
        let request = joined().new_tree(&mut runner).expect("a request").current();
        let (binds, refusals) = bound_by_name(&request.parts);
        let suspects: Vec<usize> = request
            .parts
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == Lookup::Suspect)
            .map(|(i, _)| i + 1)
            .collect();
        seen[0] += usize::from(
            suspects
                .iter()
                .any(|s| binds.iter().filter(|(_, to, _)| to == s).count() == 3),
        );
        seen[1] += usize::from(
            refusals
                .iter()
                .any(|r| matches!(r, Because::NoSource { .. })),
        );
        seen[2] += usize::from(
            refusals
                .iter()
                .any(|r| matches!(r, Because::SeveralSources { .. })),
        );
        seen[3] += usize::from(suspects.len() >= 2 && refusals.is_empty());
    }
    let names = [
        "a suspect over three",
        "a name no step returns",
        "a name two steps return",
        "two suspects bound",
    ];
    for (count, name) in seen.iter().zip(names) {
        assert!(*count >= 15, "{name} drawn {count} times of 300");
    }
}
