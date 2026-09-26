//! The run under any plan, against the core's own runner: plans of one to six steps — reads that yield, writes
//! that take, steps of any effect beside them — with bindings plain and over a list, edges from `then` and from
//! what a step takes, the stages the design words, and every way a round can end: a body finishing with data or
//! without, failing, declined, refused, unanswered; a value written into a step's words decided again as the same
//! reflex, another, or none — and one signal mid-stage, every handed round reported as the host ended it. A
//! proptest state machine: the reference is the design's sentences as code, the system under test
//! `weave::running::execute` driven as the hosts drive it — every round of a batch reported, then the run asked
//! again. Each invariant quotes the sentence it checks.

use super::reference::{abstain, decision, exclusive, gate, plan, stages, worst};
use evoke_core::Decision;
use evoke_core::document::Json;
use evoke_core::manifest::{Effect, Recognizer};
use evoke_core::name::{ArgName, FieldName, LocalName};
use evoke_core::weave::running::execute;
use evoke_core::weave::{
    Asked, Binding, Executed, Handled, Handling, Outcome, Progress, Returned, Running, Status,
    Step, Todo, Verdict, Via, Weave, Why,
};
use proptest::collection::vec;
use proptest::prelude::*;
use proptest::sample::select;
use proptest::test_runner::FileFailurePersistence;
use proptest_state_machine::{ReferenceStateMachine, StateMachineTest, prop_state_machine};

/// What a step is: a read that yields an address, a write that asks for one, a write with an optional label, a
/// step that takes part in no binding under the effect the case chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    Contact,
    Mail,
    Timer,
    Lights,
}

impl Role {
    fn reflex(self) -> &'static str {
        match self {
            Self::Contact => "contact",
            Self::Mail => "mail",
            Self::Timer => "timer",
            Self::Lights => "lights",
        }
    }
}

/// What a step takes: from which step, one value or every record's, answering its ask or written into its words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Takes {
    from: usize,
    each: bool,
    via: Via,
}

#[derive(Clone, Debug)]
struct Spec {
    role: Role,
    effect: Effect,
    takes: Option<Takes>,
    then: Vec<usize>,
}

impl Spec {
    /// The steps this one follows: an explicit `then`, or a binding.
    fn after(&self) -> Vec<usize> {
        let mut after = self.then.clone();
        if let Some(takes) = self.takes
            && !after.contains(&takes.from)
        {
            after.push(takes.from);
        }
        after.sort_unstable();
        after
    }
}

/// What a body returned: the declared field, a list of records, a result without the field, none at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Data {
    None,
    Scalar,
    Missing,
    List(u8),
    ListMissing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Became {
    Ran(Data),
    Failed,
    Declined,
    Refused,
    Unanswered,
}

/// How a step's words, its bound value in them, decided again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rewrote {
    Same,
    Other,
    Abstain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Transition {
    /// A handed round of `step` ended so.
    Report { step: usize, became: Became },
    /// The rewritten words of `step`'s next round decided.
    Rewritten { step: usize, as_: Rewrote },
    /// One signal, mid-stage.
    Cancel,
    /// Nothing left to do.
    Rest,
}

/// Why a step was skipped, in the design's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Skip {
    EarlierStep,
    NothingToTake,
    FoundNothing,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum St {
    Pending,
    Done(Status, Option<Skip>),
}

/// The rounds a step has once its sources are known.
enum Rounds {
    Count(usize),
    NothingToTake,
    FoundNothing,
}

/// The reference: the design's sentences over a plan.
#[derive(Clone, Debug)]
struct Model {
    specs: Vec<Spec>,
    stages: Vec<Vec<usize>>,
    st: Vec<St>,
    /// What each step's last round returned.
    data: Vec<Option<Data>>,
    /// The next round of each step, from 0.
    round: Vec<usize>,
    rounds: Vec<Option<usize>>,
    /// Per step, per round: how its rewritten words decided.
    decided: Vec<Vec<Rewrote>>,
    stage: usize,
    /// The steps whose next round is handed and not yet reported.
    batch: Vec<usize>,
    /// The step whose rewritten round awaits its decision.
    awaiting: Option<usize>,
    stopped: bool,
    cancelled: bool,
}

impl Model {
    fn new(specs: Vec<Spec>) -> Self {
        let effects: Vec<Effect> = specs.iter().map(|spec| spec.effect).collect();
        let after: Vec<Vec<usize>> = specs.iter().map(Spec::after).collect();
        let n = specs.len();
        let mut model = Self {
            stages: stages(&effects, &after),
            specs,
            st: vec![St::Pending; n],
            data: vec![None; n],
            round: vec![0; n],
            rounds: vec![None; n],
            decided: vec![Vec::new(); n],
            stage: 0,
            batch: Vec::new(),
            awaiting: None,
            stopped: false,
            cancelled: false,
        };
        model.settle();
        model
    }

    fn done(&self) -> bool {
        self.stopped || self.cancelled || self.stage >= self.stages.len()
    }

    fn is_done(&self, n: usize) -> bool {
        matches!(self.st[n - 1], St::Done(..))
    }

    /// «A binding is a declared yield …; a source that found nothing to take, an empty list, skips its takers
    /// clean and stops nothing; … a step skipped because its source yielded nothing to take count[s] as failed.»
    fn rounds_of(&self, n: usize) -> Rounds {
        let Some(takes) = self.specs[n - 1].takes else {
            return Rounds::Count(1);
        };
        match &self.st[takes.from - 1] {
            St::Done(Status::Skipped, Some(Skip::FoundNothing)) => Rounds::FoundNothing,
            St::Done(Status::Ran, _) => match (self.data[takes.from - 1], takes.each) {
                (Some(Data::Scalar), false) => Rounds::Count(1),
                (Some(Data::List(0)), true) => Rounds::FoundNothing,
                (Some(Data::List(k)), true) => Rounds::Count(usize::from(k)),
                _ => Rounds::NothingToTake,
            },
            other => unreachable!("a taker's turn came while its source stood at {other:?}"),
        }
    }

    /// The stage's next batch: «a stage's steps [run] together»; a rewritten round is decided first; a stage
    /// over, «a failure, a decline or a refusal ends the weave after its stage», else the next stage follows.
    fn settle(&mut self) {
        loop {
            if self.done() {
                return;
            }
            self.awaiting = None;
            let mut batch = Vec::new();
            let stage = self.stages[self.stage].clone();
            for &n in &stage {
                if self.is_done(n) {
                    continue;
                }
                if self.rounds[n - 1].is_none() {
                    match self.rounds_of(n) {
                        Rounds::NothingToTake => {
                            self.st[n - 1] = St::Done(Status::Skipped, Some(Skip::NothingToTake));
                            continue;
                        }
                        Rounds::FoundNothing => {
                            self.st[n - 1] = St::Done(Status::Skipped, Some(Skip::FoundNothing));
                            continue;
                        }
                        Rounds::Count(k) => self.rounds[n - 1] = Some(k),
                    }
                }
                if let Some(takes) = self.specs[n - 1].takes
                    && takes.via == Via::Rewrite
                {
                    match self.decided[n - 1].get(self.round[n - 1]) {
                        None => {
                            self.awaiting = Some(n);
                            return;
                        }
                        Some(Rewrote::Same) => {}
                        Some(_) => {
                            self.st[n - 1] = St::Done(Status::Refused, None);
                            continue;
                        }
                    }
                }
                batch.push(n);
            }
            if !batch.is_empty() {
                self.batch = batch;
                return;
            }
            let stopped = stage.iter().any(|&n| {
                !matches!(
                    self.st[n - 1],
                    St::Done(Status::Ran, _) | St::Done(Status::Skipped, Some(Skip::FoundNothing))
                )
            });
            if stopped {
                self.stopped = true;
                for later in &self.stages[self.stage + 1..] {
                    for &n in later {
                        self.st[n - 1] = St::Done(Status::Skipped, Some(Skip::EarlierStep));
                    }
                }
                return;
            }
            self.stage += 1;
        }
    }

    fn enabled(&self, cancel: bool) -> Vec<Transition> {
        let mut out = Vec::new();
        if self.cancelled {
            return vec![Transition::Rest];
        }
        if let Some(step) = self.awaiting {
            for as_ in [Rewrote::Same, Rewrote::Other, Rewrote::Abstain] {
                out.push(Transition::Rewritten { step, as_ });
            }
        } else {
            for &step in &self.batch {
                let ways = [
                    Became::Ran(Data::Scalar),
                    Became::Ran(Data::List(0)),
                    Became::Ran(Data::List(1)),
                    Became::Ran(Data::List(3)),
                    Became::Ran(Data::Missing),
                    Became::Ran(Data::ListMissing),
                    Became::Ran(Data::None),
                    Became::Failed,
                    Became::Declined,
                    Became::Refused,
                    Became::Unanswered,
                ];
                for became in ways {
                    out.push(Transition::Report { step, became });
                }
            }
        }
        if cancel && !self.batch.is_empty() {
            out.push(Transition::Cancel);
        }
        if out.is_empty() {
            out.push(Transition::Rest);
        }
        out
    }

    fn allows(&self, transition: &Transition) -> bool {
        match transition {
            Transition::Report { step, .. } => {
                !self.cancelled && self.awaiting.is_none() && self.batch.contains(step)
            }
            Transition::Rewritten { step, .. } => self.awaiting == Some(*step),
            Transition::Cancel => !self.cancelled && !self.batch.is_empty(),
            Transition::Rest => self.done(),
        }
    }

    fn applied(mut self, transition: &Transition) -> Self {
        match transition {
            Transition::Report { step, became } => {
                let n = *step;
                self.batch.retain(|m| *m != n);
                match became {
                    Became::Ran(data) => {
                        self.data[n - 1] = Some(*data);
                        self.round[n - 1] += 1;
                        if self.round[n - 1]
                            >= self.rounds[n - 1].expect("a handed step has rounds")
                        {
                            self.st[n - 1] = St::Done(Status::Ran, None);
                        }
                    }
                    Became::Failed => self.st[n - 1] = St::Done(Status::Failed, None),
                    Became::Declined => self.st[n - 1] = St::Done(Status::Declined, None),
                    Became::Refused => self.st[n - 1] = St::Done(Status::Refused, None),
                    Became::Unanswered => self.st[n - 1] = St::Done(Status::Unanswered, None),
                }
                if self.batch.is_empty() {
                    self.settle();
                }
            }
            Transition::Rewritten { step, as_ } => {
                self.decided[step - 1].push(*as_);
                self.awaiting = None;
                self.settle();
            }
            Transition::Cancel => {
                self.cancelled = true;
                self.batch.clear();
                for st in &mut self.st {
                    if *st == St::Pending {
                        *st = St::Done(Status::Skipped, Some(Skip::Cancelled));
                    }
                }
            }
            Transition::Rest => {}
        }
        self
    }
}

fn role() -> impl Strategy<Value = Role> {
    prop_oneof![
        4 => Just(Role::Contact),
        3 => Just(Role::Mail),
        2 => Just(Role::Timer),
        2 => Just(Role::Lights)
    ]
}

fn effect() -> impl Strategy<Value = Effect> {
    prop_oneof![
        Just(Effect::Read),
        Just(Effect::Write),
        Just(Effect::Destructive)
    ]
}

/// A plan: one to six steps, each a role; a mail or a timer may take from a contact before it, one value or each
/// record's; a step may follow an earlier one by `then`; lights run under any effect.
fn model() -> impl Strategy<Value = Model> {
    (1usize..=6, prop_oneof![2 => Just(false), 1 => Just(true)]).prop_flat_map(|(n, reads_only)| {
        (
            vec(role(), n),
            // A mail or a timer takes from a contact before it three times in four, so takers are drawn often.
            vec(
                (
                    any::<u8>(),
                    prop_oneof![3 => Just(true), 1 => Just(false)],
                    any::<bool>(),
                    any::<bool>(),
                    effect(),
                ),
                n,
            ),
        )
            .prop_map(move |(roles, choices)| {
                // A third of the plans are reads alone, so «reads may [run side by side]» is drawn often.
                let roles: Vec<Role> = roles
                    .into_iter()
                    .map(|role| {
                        if reads_only && matches!(role, Role::Mail | Role::Timer) {
                            Role::Contact
                        } else {
                            role
                        }
                    })
                    .collect();
                let mut specs = Vec::new();
                for (i, (role, (pick, takes, each, then, effect))) in
                    roles.iter().zip(choices).enumerate()
                {
                    let sources: Vec<usize> = roles[..i]
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| **r == Role::Contact)
                        .map(|(j, _)| j + 1)
                        .collect();
                    let takes = match role {
                        Role::Mail | Role::Timer if takes && !sources.is_empty() => Some(Takes {
                            from: sources[usize::from(pick) % sources.len()],
                            each,
                            via: if *role == Role::Mail {
                                Via::Fill
                            } else {
                                Via::Rewrite
                            },
                        }),
                        _ => None,
                    };
                    let then = if then && i > 0 {
                        vec![usize::from(pick) % i + 1]
                    } else {
                        Vec::new()
                    };
                    let effect = match role {
                        Role::Contact => Effect::Read,
                        Role::Mail | Role::Timer => Effect::Write,
                        Role::Lights if reads_only => Effect::Read,
                        Role::Lights => effect,
                    };
                    specs.push(Spec {
                        role: *role,
                        effect,
                        takes,
                        then,
                    });
                }
                Model::new(specs)
            })
    })
}

/// The machine without a cancel: what the core keeps today.
struct Plain;

impl ReferenceStateMachine for Plain {
    type State = Model;
    type Transition = Transition;

    fn init_state() -> BoxedStrategy<Self::State> {
        model().boxed()
    }

    fn transitions(state: &Self::State) -> BoxedStrategy<Self::Transition> {
        select(state.enabled(false)).boxed()
    }

    fn apply(state: Self::State, transition: &Self::Transition) -> Self::State {
        state.applied(transition)
    }

    fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool {
        state.allows(transition)
    }
}

/// The machine with one signal mid-stage.
struct Cancelling;

impl ReferenceStateMachine for Cancelling {
    type State = Model;
    type Transition = Transition;

    fn init_state() -> BoxedStrategy<Self::State> {
        model().boxed()
    }

    fn transitions(state: &Self::State) -> BoxedStrategy<Self::Transition> {
        select(state.enabled(true)).boxed()
    }

    fn apply(state: Self::State, transition: &Self::Transition) -> Self::State {
        state.applied(transition)
    }

    fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool {
        state.allows(transition)
    }
}

/// The system under test: the core's runner over a weave built from the specs, driven as a host drives it.
struct Sut {
    plan: evoke_core::Plan,
    gate: evoke_core::Gate,
    weave: Weave,
    progress: Progress,
    handed: Vec<Handling>,
    pending: Option<Asked>,
    executed: Option<Executed>,
}

fn name(text: &str) -> LocalName {
    LocalName::new(text).expect("a name")
}

/// The weave the specs describe, as the planner would have written it.
fn weave_of(model: &Model) -> Weave {
    let steps: Vec<Step> = model
        .specs
        .iter()
        .enumerate()
        .map(|(i, spec)| Step {
            n: i + 1,
            text: format!("{} ({})", spec.role.reflex(), i + 1),
            end: 0,
            decision: decision(spec.role.reflex()),
            reflex: Some(name(spec.role.reflex())),
            effect: Some(spec.effect),
            refs: Vec::new(),
            repair: None,
            after: spec.after(),
        })
        .collect();
    let binds: Vec<Binding> = model
        .specs
        .iter()
        .enumerate()
        .filter_map(|(i, spec)| {
            let takes = spec.takes?;
            let (arg, kind) = match spec.role {
                Role::Mail => ("to", Recognizer::Email),
                _ => ("label", Recognizer::Quoted),
            };
            Some(Binding {
                from: takes.from,
                to: i + 1,
                arg: ArgName::new(arg).expect("an argument name"),
                field: FieldName::new("email").expect("a field name"),
                kind,
                via: takes.via,
                each: takes
                    .each
                    .then(|| FieldName::new("people").expect("a field name")),
            })
        })
        .collect();
    let effects: Vec<Effect> = model.specs.iter().map(|spec| spec.effect).collect();
    Weave {
        input: "the model's request".to_owned(),
        splits: Vec::new(),
        steps,
        excluded: Vec::new(),
        binds,
        exclusive: exclusive(&effects),
        stages: model.stages.clone(),
        verdict: Verdict {
            outcome: Outcome::Run,
            because: Vec::new(),
        },
    }
}

/// What a body returned, as data: the address a taker takes, a list of people, or neither.
fn data_of(step: usize, data: Data) -> Option<Json> {
    let email = |i: usize| format!("p{step}-{i}@example.com");
    match data {
        Data::None => None,
        Data::Scalar => Some(serde_json::json!({ "email": email(0) })),
        Data::Missing => Some(serde_json::json!({ "name": "dana" })),
        Data::List(k) => Some(serde_json::json!({
            "people": (0..usize::from(k)).map(|i| serde_json::json!({ "email": email(i) })).collect::<Vec<_>>()
        })),
        Data::ListMissing => Some(serde_json::json!({ "people": [{ "name": "dana" }] })),
    }
}

fn handled(handling: &Handling, became: Became) -> Handled {
    let said = |message: &str| {
        Some(Why::Said {
            message: message.to_owned(),
        })
    };
    let (status, why, result) = match became {
        Became::Ran(data) => (
            Status::Ran,
            None,
            Some(Returned {
                text: "ok".to_owned(),
                data: data_of(handling.step, data),
            }),
        ),
        Became::Failed => (Status::Failed, said("the body failed"), None),
        Became::Declined => (Status::Declined, said("declined at its confirm"), None),
        Became::Refused => (Status::Refused, Some(Why::NoReflex), None),
        Became::Unanswered => (Status::Unanswered, said("no terminal to ask"), None),
    };
    Handled {
        step: handling.step,
        round: handling.round,
        status,
        why,
        result,
    }
}

impl Sut {
    fn new(model: &Model) -> Self {
        let mut sut = Self {
            plan: plan(),
            gate: gate(),
            weave: weave_of(model),
            progress: Progress::default(),
            handed: Vec::new(),
            pending: None,
            executed: None,
        };
        sut.sync();
        sut
    }

    /// The run asked again, as a host asks once every handed round is reported.
    fn sync(&mut self) {
        self.handed.clear();
        self.pending = None;
        match execute(&self.plan, Some(&self.gate), &self.weave, &self.progress) {
            Running::Done { executed } => self.executed = Some(executed),
            Running::Todo {
                todo: Todo::Handle { handling },
            } => self.handed = handling,
            Running::Todo {
                todo: Todo::Decide { asked, .. },
            } => self.pending = Some(asked),
        }
    }

    fn apply(mut self, transition: &Transition) -> Self {
        match *transition {
            Transition::Report { step, became } => {
                let at = self
                    .handed
                    .iter()
                    .position(|h| h.step == step)
                    .expect("the reported step was handed");
                let handling = self.handed.remove(at);
                self.progress.handled.push(handled(&handling, became));
                if self.handed.is_empty() {
                    self.sync();
                }
            }
            Transition::Rewritten { as_, .. } => {
                let asked = self.pending.take().expect("a decision was wanted");
                let decided: Decision = match as_ {
                    Rewrote::Same => decision("timer"),
                    Rewrote::Other => decision("lights"),
                    Rewrote::Abstain => abstain(),
                };
                self.progress.decided.push((asked, decided));
                self.sync();
            }
            Transition::Cancel => {
                // «Every step that did not finish reads skipped with the reason cancelled»: the host ends every
                // handed round's body and reports each so, then asks again.
                for handling in std::mem::take(&mut self.handed) {
                    self.progress.handled.push(Handled {
                        step: handling.step,
                        round: handling.round,
                        status: Status::Skipped,
                        why: Some(Why::Cancelled),
                        result: None,
                    });
                }
                self.sync();
            }
            Transition::Rest => {}
        }
        self
    }

    /// What holds while the run goes on, and what the run reports at its end.
    fn check(&self, model: &Model) {
        if let Some(n) = model.awaiting {
            let pending = self
                .pending
                .as_ref()
                .expect("the core wants the rewritten words decided");
            assert_eq!(
                pending.only.as_ref().map(LocalName::as_str),
                Some(model.specs[n - 1].role.reflex()),
                "the decision wanted is step {n}'s, narrowed to its reflex"
            );
            assert!(
                self.handed.is_empty(),
                "nothing is handed while a decision is wanted"
            );
            return;
        }
        if model.done() {
            let executed = self.executed.as_ref().expect("the run is over");
            assert!(self.handed.is_empty() && self.pending.is_none());
            let mut statuses = Vec::new();
            for (outcome, st) in executed.steps.iter().zip(&model.st) {
                let St::Done(status, skip) = st else {
                    panic!("step {} is not done in the reference", outcome.step);
                };
                assert_eq!(outcome.status, *status, "step {}'s status", outcome.step);
                match skip {
                    Some(Skip::EarlierStep) => assert_eq!(outcome.why, Some(Why::EarlierStep)),
                    Some(Skip::NothingToTake) => assert_eq!(outcome.why, Some(Why::NothingToTake)),
                    Some(Skip::FoundNothing) => assert_eq!(outcome.why, Some(Why::FoundNothing)),
                    Some(Skip::Cancelled) => assert_eq!(outcome.why, Some(Why::Cancelled)),
                    None => {}
                }
                statuses.push((*status, *skip == Some(Skip::NothingToTake)));
            }
            // «The exit is the worst step's, a step skipped because its source yielded nothing to take counting
            // as failed.»
            assert_eq!(executed.worst, worst(&statuses), "the worst step");
            return;
        }
        // «A write never runs beside anything, reads may»: the batch is the stage's pending rounds, together.
        let handed: Vec<usize> = self.handed.iter().map(|h| h.step).collect();
        assert_eq!(
            handed, model.batch,
            "the rounds handed are the stage's pending ones, in order"
        );
        for handling in &self.handed {
            assert_eq!(
                handling.round,
                model.round[handling.step - 1],
                "step {}'s round",
                handling.step
            );
            assert!(
                model.stages[model.stage].contains(&handling.step),
                "a handed step is of the current stage"
            );
            assert!(!model.is_done(handling.step), "a skipped step never runs");
        }
        if self.handed.len() > 1 {
            assert!(
                self.handed
                    .iter()
                    .all(|h| model.specs[h.step - 1].effect == Effect::Read),
                "several handed together are all reads"
            );
        }
    }
}

struct PlainTest;

impl StateMachineTest for PlainTest {
    type SystemUnderTest = Sut;
    type Reference = Plain;

    fn init_test(ref_state: &Model) -> Self::SystemUnderTest {
        Sut::new(ref_state)
    }

    fn apply(
        state: Self::SystemUnderTest,
        _: &Model,
        transition: Transition,
    ) -> Self::SystemUnderTest {
        state.apply(&transition)
    }

    fn check_invariants(state: &Self::SystemUnderTest, ref_state: &Model) {
        state.check(ref_state);
    }
}

struct CancelTest;

impl StateMachineTest for CancelTest {
    type SystemUnderTest = Sut;
    type Reference = Cancelling;

    fn init_test(ref_state: &Model) -> Self::SystemUnderTest {
        Sut::new(ref_state)
    }

    fn apply(
        state: Self::SystemUnderTest,
        _: &Model,
        transition: Transition,
    ) -> Self::SystemUnderTest {
        state.apply(&transition)
    }

    /// What one signal must guarantee, checked once it was sent: no round handed after it, no decision wanted,
    /// the run over, and every step that had not finished reading `skipped · cancelled` — which `check` compares
    /// step by step. The whole's word is the host's: the CLI ends as an interrupted process, the SDK rejects.
    fn check_invariants(state: &Self::SystemUnderTest, ref_state: &Model) {
        state.check(ref_state);
        if !ref_state.cancelled {
            return;
        }
        assert!(
            state.handed.is_empty() && state.pending.is_none(),
            "nothing starts after a cancel"
        );
        let executed = state
            .executed
            .as_ref()
            .expect("the run is over after a cancel");
        assert!(
            executed
                .steps
                .iter()
                .zip(&ref_state.st)
                .all(|(outcome, st)| {
                    *st != St::Done(Status::Skipped, Some(Skip::Cancelled))
                        || (outcome.status == Status::Skipped
                            && outcome.why == Some(Why::Cancelled))
                }),
            "every step that had not finished reads skipped · cancelled"
        );
    }
}

prop_state_machine! {
    #![proptest_config(ProptestConfig { cases: 1024, failure_persistence: Some(Box::new(FileFailurePersistence::Direct("proptest-regressions/weave/run.txt"))), ..ProptestConfig::default() })]

    /// «Reads may run side by side, a write never beside anything; a failure, a decline or a refusal ends the
    /// weave after its stage; a skipped step never runs; the exit is the worst step's.»
    #[test]
    fn the_run_keeps_the_schedule(sequential 1..40 => PlainTest);
}

prop_state_machine! {
    #![proptest_config(ProptestConfig { cases: 512, failure_persistence: Some(Box::new(FileFailurePersistence::Direct("proptest-regressions/weave/run.txt"))), ..ProptestConfig::default() })]

    /// «One signal through every stage and round: every step that did not finish reads skipped with the reason
    /// cancelled» — a cancel stops every body and starts none, whatever the plan and wherever it lands.
    #[test]
    fn a_cancel_stops_every_body_and_starts_none(sequential 1..40 => CancelTest);
}

/// One cancelled round reported is the whole weave cancelled: a round handed beside it that the host said nothing
/// of is never handed again, and reads `skipped · cancelled` like the rest.
#[test]
fn a_cancel_reported_for_one_round_cancels_the_rounds_beside_it() {
    let model = Model::new(vec![
        Spec {
            role: Role::Contact,
            effect: Effect::Read,
            takes: None,
            then: Vec::new(),
        },
        Spec {
            role: Role::Contact,
            effect: Effect::Read,
            takes: None,
            then: Vec::new(),
        },
        Spec {
            role: Role::Contact,
            effect: Effect::Read,
            takes: None,
            then: vec![1],
        },
    ]);
    let mut sut = Sut::new(&model);
    let before: Vec<(usize, usize)> = sut.handed.iter().map(|h| (h.step, h.round)).collect();
    assert_eq!(
        before,
        [(1, 0), (2, 0)],
        "a stage of two reads, handed together"
    );
    let first = sut.handed[0].clone();
    sut.progress
        .handled
        .push(handled(&first, Became::Ran(Data::Scalar)));
    let second = sut.handed[1].clone();
    sut.progress.handled.push(Handled {
        step: second.step,
        round: second.round,
        status: Status::Skipped,
        why: Some(Why::Cancelled),
        result: None,
    });
    sut.sync();
    assert!(
        sut.handed.is_empty() && sut.pending.is_none(),
        "nothing starts after a cancel"
    );
    let executed = sut.executed.expect("the run is over");
    let outcomes: Vec<(Status, Option<Why>)> = executed
        .steps
        .iter()
        .map(|s| (s.status, s.why.clone()))
        .collect();
    assert_eq!(
        outcomes,
        [
            (Status::Ran, None),
            (Status::Skipped, Some(Why::Cancelled)),
            (Status::Skipped, Some(Why::Cancelled)),
        ]
    );
    assert_eq!(
        executed.worst,
        Status::Ran,
        "a skipped step adds nothing of its own; the host says the rest"
    );
}

/// A step with two bindings reads the same whatever their order: a source that yielded no field to take is the
/// failure, whatever another source found.
#[test]
fn two_bindings_read_the_same_whatever_their_order() {
    let worst_with = |binds: Vec<Binding>| {
        let mut model = Model::new(vec![
            Spec {
                role: Role::Contact,
                effect: Effect::Read,
                takes: None,
                then: Vec::new(),
            },
            Spec {
                role: Role::Contact,
                effect: Effect::Read,
                takes: None,
                then: Vec::new(),
            },
            Spec {
                role: Role::Mail,
                effect: Effect::Write,
                takes: None,
                then: vec![1, 2],
            },
        ]);
        model.stages = vec![vec![1], vec![2], vec![3]];
        let mut sut = Sut::new(&model);
        sut.weave.binds = binds;
        sut.weave.stages = model.stages.clone();
        sut.sync();
        let first = sut.handed[0].clone();
        sut.progress
            .handled
            .push(handled(&first, Became::Ran(Data::List(0))));
        sut.sync();
        let second = sut.handed[0].clone();
        sut.progress
            .handled
            .push(handled(&second, Became::Ran(Data::Missing)));
        sut.sync();
        sut.executed.expect("the run is over").worst
    };
    let bind = |from: usize, arg: &str, each: bool| Binding {
        from,
        to: 3,
        arg: ArgName::new(arg).expect("an argument name"),
        field: FieldName::new("email").expect("a field name"),
        kind: Recognizer::Email,
        via: Via::Fill,
        each: each.then(|| FieldName::new("people").expect("a field name")),
    };
    let found_nothing_first = worst_with(vec![bind(1, "to", true), bind(2, "subject", false)]);
    let nothing_to_take_first = worst_with(vec![bind(2, "subject", false), bind(1, "to", true)]);
    assert_eq!(found_nothing_first, Status::Failed);
    assert_eq!(nothing_to_take_first, Status::Failed);
}

/// The generator reaches every shape the invariants speak of; a shape it never drew would prove nothing.
#[test]
fn the_generator_reaches_every_shape() {
    use proptest::strategy::ValueTree;
    use proptest::test_runner::TestRunner;
    let mut runner = TestRunner::deterministic();
    let mut seen = [0usize; 6];
    for _ in 0..300 {
        let model = model().new_tree(&mut runner).expect("a plan").current();
        let specs = &model.specs;
        seen[0] += usize::from(
            specs
                .iter()
                .any(|s| s.takes.is_some_and(|t| t.via == Via::Rewrite)),
        );
        seen[1] += usize::from(specs.iter().any(|s| s.takes.is_some_and(|t| t.each)));
        seen[2] += usize::from(specs.iter().any(|s| !s.then.is_empty()));
        seen[3] += usize::from(model.stages.iter().any(|stage| stage.len() > 1));
        seen[4] += usize::from(specs.iter().any(|s| s.effect == Effect::Destructive));
        seen[5] += usize::from(
            specs
                .iter()
                .any(|s| s.takes.is_some_and(|t| t.via == Via::Fill && !t.each)),
        );
    }
    let names = [
        "a rewritten taker",
        "a taker over a list",
        "a then edge",
        "reads side by side",
        "a destructive step",
        "a filled taker",
    ];
    for (count, name) in seen.iter().zip(names) {
        assert!(*count >= 15, "{name} drawn {count} times of 300");
    }
    eprintln!(
        "shapes drawn of 300 plans: {:?}",
        seen.iter()
            .zip(names)
            .map(|(c, n)| format!("{n} {c}"))
            .collect::<Vec<_>>()
    );
}
