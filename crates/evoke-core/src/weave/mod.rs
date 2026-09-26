//! One sentence, several reflexes: a request read into steps, each decided by the foundation unchanged, ordered
//! by the request's own words, a step's result threaded into a later step. The reading, the plan and the run are
//! pure: each is a function over the answers a host has gathered so far — the engine's judgments, the foundation's
//! decisions, the bodies' results — and returns either what it needs next or what it made. A host loops: it asks
//! the adapter, decides a text, prompts a person, runs a body, and calls again. In: a `Plan`, the request, the
//! answers or the progress. Out: a `Weave` — steps, bindings, stages, a verdict before anything runs — and an
//! `Executed`, per step what became of it.

pub mod planning;
pub mod reading;
pub mod running;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::Request;
use crate::decide::Decision;
use crate::document::Json;
use crate::manifest::{Effect, Recognizer};
use crate::name::{ArgName, FieldName, LocalName, Tag};
pub use reading::{How, Order, Ref, Split, Where};

/// A text for the foundation to decide: over the reflexes the tags allow, or one reflex alone.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asked {
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub only: Option<LocalName>,
}

/// What the plan needs a host to do next.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Need {
    /// Ask the adapter whether each split point separates two things: `weave.split_<n>`.
    Judge { request: Request },
    /// Ask the adapter which earlier step each reference names: `weave.ref_<k>_<i>`.
    Refer { request: Request },
    /// Decide each text, side by side where the host can.
    Decide { asked: Vec<Asked> },
}

/// What a host gathered for the plan so far.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Answers {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub judged: Option<crate::adapter::Raw>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referred: Option<crate::adapter::Raw>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decided: Vec<(Asked, Decision)>,
}

/// The plan, or what it needs first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Planning {
    Done { weave: Weave },
    Need { need: Need },
}

/// How a segment that matched nothing on its own was settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Repair {
    Narrowed,
    Spliced,
    Merged,
}

/// One step of the plan: a segment's text and the foundation's decision on it, in the order it is to happen.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Step {
    /// From 1, as the plan prints it.
    pub n: usize,
    pub text: String,
    /// Where the step's words end in the request, in characters: what an explicit `then` orders.
    pub end: usize,
    pub decision: Decision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflex: Option<LocalName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<Effect>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub refs: Vec<Ref>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<Repair>,
    /// The steps this one must follow: an explicit `then`, or a binding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<usize>,
}

/// How a bound value reaches its step: answering the step's own ask; the step decided again with the value in
/// its words — where the receiver is optional and the classifier assigns it; or a whole result handed beside the
/// decision, never through its words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Via {
    Fill,
    Rewrite,
    Takes,
}

/// A value of one step's result taken by a later step: which field, into which argument, how; or the whole
/// result, by the name it goes by.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Binding {
    pub from: usize,
    pub to: usize,
    pub arg: ArgName,
    /// The field taken; for `Takes`, the name the whole result goes by — so every reader checks `via` first.
    pub field: FieldName,
    /// The recognizer the field is read with; none for a whole result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<Recognizer>,
    pub via: Via,
    /// The list field of the source's result whose records carry `field`: the step runs once per record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub each: Option<FieldName>,
}

/// Why the plan does not simply run: each names its steps, so a host can say it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Because {
    /// Every part of the request was left out: nothing to do.
    NothingToDo,
    /// A step matches no reflex: the request is refused whole.
    NoReflex { step: usize },
    /// A required argument no binding covers: the step's own question, before anything runs.
    Needs { step: usize, arg: ArgName },
    /// A bare reference over several fields the step could take: never guessed.
    Several {
        step: usize,
        source: usize,
        fields: Vec<FieldName>,
    },
    /// A singular reference over a result of several records: never guessed.
    OneOfMany {
        step: usize,
        source: usize,
        field: FieldName,
    },
    /// A reference to steps whose results the step takes nothing from: run it without them, or not.
    TakesNothing { step: usize, sources: Vec<usize> },
    /// A whole result the step takes by name that no step before it returns: refused, nothing answers it.
    NoSource { step: usize, name: FieldName },
    /// What the step takes one of — a whole result, or a field — that several steps return, or one step once
    /// per record: refused, nothing answers it.
    SeveralSources {
        step: usize,
        name: FieldName,
        sources: Vec<usize>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Run,
    Ask,
    Confirm,
    Refuse,
}

/// The verdict before anything runs, with every reason.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verdict {
    pub outcome: Outcome,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub because: Vec<Because>,
}

/// The plan: the request in the words' own order, its split points as judged, the steps, what was left out, the
/// bindings, the schedule and the verdict.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawWeave")]
pub struct Weave {
    pub input: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splits: Vec<Split>,
    pub steps: Vec<Step>,
    /// Fragments left out because they begin with a negation: never decided, never run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excluded: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub binds: Vec<Binding>,
    /// Whether a write is among the steps, so none may run beside another.
    pub exclusive: bool,
    /// A stage's steps run together; stages run in order.
    pub stages: Vec<Vec<usize>>,
    pub verdict: Verdict,
}

#[derive(Deserialize)]
struct RawWeave {
    input: String,
    #[serde(default)]
    splits: Vec<Split>,
    steps: Vec<Step>,
    #[serde(default)]
    excluded: Vec<String>,
    #[serde(default)]
    binds: Vec<Binding>,
    exclusive: bool,
    stages: Vec<Vec<usize>>,
    verdict: Verdict,
}

impl TryFrom<RawWeave> for Weave {
    type Error = String;

    /// A plan from the wire holds together: steps numbered from 1 in order, every step in exactly one stage, every
    /// binding from an earlier step to a later one — so the run never reaches for a step that is not there.
    fn try_from(raw: RawWeave) -> Result<Self, String> {
        let count = raw.steps.len();
        if let Some((i, step)) = raw
            .steps
            .iter()
            .enumerate()
            .find(|(i, step)| step.n != i + 1)
        {
            return Err(format!(
                "step {} stands where step {} should",
                step.n,
                i + 1
            ));
        }
        let mut staged = vec![false; count];
        for n in raw.stages.iter().flatten() {
            match n.checked_sub(1).and_then(|i| staged.get_mut(i)) {
                Some(seen) if !*seen => *seen = true,
                Some(_) => return Err(format!("step {n} is in two stages")),
                None => return Err(format!("a stage names step {n}, which the plan lacks")),
            }
        }
        if let Some(i) = staged.iter().position(|seen| !seen) {
            return Err(format!("step {} is in no stage", i + 1));
        }
        for b in &raw.binds {
            if b.from == 0 || b.to > count || b.from >= b.to {
                return Err(format!(
                    "a binding from step {} to step {} names no earlier step of the plan",
                    b.from, b.to
                ));
            }
            // A step run once per record returns one result per round: nothing takes one of them.
            if raw.binds.iter().any(|c| c.to == b.from && c.each.is_some()) {
                return Err(format!(
                    "a binding from step {} takes from a step run once per record",
                    b.from
                ));
            }
        }
        Ok(Self {
            input: raw.input,
            splits: raw.splits,
            steps: raw.steps,
            excluded: raw.excluded,
            binds: raw.binds,
            exclusive: raw.exclusive,
            stages: raw.stages,
            verdict: raw.verdict,
        })
    }
}

impl Weave {
    #[must_use]
    pub fn step(&self, n: usize) -> Option<&Step> {
        self.steps.get(n.checked_sub(1)?)
    }
}

/// A value bound into a step at its turn: the argument it reached and the field it came from; a whole result
/// carries no value here — it stays once in the log, on its source's line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bound {
    pub arg: ArgName,
    pub from: usize,
    pub field: FieldName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// What a body returned: its text, and data when it gave some.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Returned {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Json>,
}

/// One round of a step for a host to take through the foundation's own loop — ask, confirm, run — as `handle`
/// runs a decision: the decision with any bound values in place, the input the body will see, and the whole
/// results the body receives beside them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Handling {
    pub step: usize,
    /// From 0; a step bound to a list runs one round per record.
    pub round: usize,
    pub decision: Decision,
    pub input: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bound: Vec<Bound>,
    /// Per taken argument, its source's whole `data`: the same for every round of the step.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub taken: IndexMap<ArgName, Json>,
}

/// What became of a step, or of one of its rounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ran,
    Failed,
    Declined,
    Refused,
    Skipped,
    Unanswered,
}

/// Why a step did not run, or did not finish.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Why {
    /// An earlier stage did not run whole.
    EarlierStep,
    /// The step it names yielded nothing it can take: no such field, no data, or null.
    NothingToTake { from: usize },
    /// The step it names returned a whole result over the cap, a mebibyte of JSON.
    TooLarge { from: usize },
    /// A step it depends on found nothing: an empty list.
    FoundNothing,
    /// Once the values were in place, the words matched no reflex.
    NoReflex,
    /// Once the values were in place, the words read as another reflex.
    ReadAs { reflex: LocalName },
    /// The weave was cancelled: the round the host ended, and every step that had not finished by then.
    Cancelled,
    /// What the host said: a body's failure, a prompt declined, a question no one answered.
    Said { message: String },
}

/// What a host made of one round.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Handled {
    pub step: usize,
    pub round: usize,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<Why>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Returned>,
}

/// What a host gathered for the run so far.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Progress {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decided: Vec<(Asked, Decision)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handled: Vec<Handled>,
}

/// What the run needs a host to do next.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Todo {
    /// Decide a step again with its bound values written into its words, narrowed to its reflex: which step,
    /// which of its rounds, and the text.
    Decide {
        step: usize,
        round: usize,
        asked: Asked,
    },
    /// Take these rounds through the foundation's loop — a stage's steps together where the host can.
    Handle { handling: Vec<Handling> },
}

/// The run, or what it needs first.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Running {
    Done { executed: Executed },
    Todo { todo: Todo },
}

/// What became of one step: its status, why it stopped, the values bound into it, and every round the host took.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StepOutcome {
    pub step: usize,
    pub status: Status,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<Why>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bound: Vec<Bound>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rounds: Vec<Handled>,
}

/// The run: per step, what became of it, in plan order; and the whole's status, the worst step's.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Executed {
    pub steps: Vec<StepOutcome>,
    /// The worst status across the steps, as the exit codes rank them: ran, then failed, declined or refused,
    /// unanswered; a skipped step follows what stopped the weave and adds nothing of its own.
    pub worst: Status,
}

impl Executed {
    pub(crate) fn of(steps: Vec<StepOutcome>) -> Self {
        let worst = steps
            .iter()
            .map(|step| step.status)
            .max_by(|a, b| rank(*a).cmp(&rank(*b)))
            .unwrap_or(Status::Ran);
        // A skipped step adds nothing of its own: a weave whose worst is a skip ran.
        let worst = if worst == Status::Skipped {
            Status::Ran
        } else {
            worst
        };
        let failed = steps.iter().any(|step| {
            step.status == Status::Skipped
                && matches!(
                    step.why,
                    Some(Why::NothingToTake { .. } | Why::TooLarge { .. })
                )
        });
        // A step skipped because its source yielded nothing it can take, or more than can be handed, is the
        // source's failure to deliver what its manifest declares: the whole failed, unless something worse
        // stopped it.
        let worst = if failed && rank(worst) < rank(Status::Failed) {
            Status::Failed
        } else {
            worst
        };
        Self { steps, worst }
    }
}

/// The exit codes' order: 0 ran · 1 failed · 2 declined or refused · 3 needs a human. A step skipped after what
/// stopped the weave, or over an empty list, adds nothing of its own.
fn rank(status: Status) -> u8 {
    match status {
        Status::Ran | Status::Skipped => 0,
        Status::Failed => 1,
        Status::Declined | Status::Refused => 2,
        Status::Unanswered => 3,
    }
}

/// What the planner asked to decide a step, as its repair tells: a fragment narrowed to its neighbour's reflex,
/// or spliced into its words, was decided under that reflex alone; any other step over the tags.
#[must_use]
pub fn asked_for(step: &Step, tags: &[Tag]) -> Asked {
    match (step.repair, &step.reflex) {
        (Some(Repair::Narrowed | Repair::Spliced), Some(reflex)) => Asked {
            text: step.text.clone(),
            tags: Vec::new(),
            only: Some(reflex.clone()),
        },
        _ => Asked {
            text: step.text.clone(),
            tags: tags.to_vec(),
            only: None,
        },
    }
}

/// The names a result may yield, a list's record fields included: what a noun may name.
pub(crate) fn field_names(yields: &IndexMap<FieldName, crate::manifest::Yield>) -> Vec<String> {
    yields
        .iter()
        .flat_map(|(field, yield_)| match yield_ {
            crate::manifest::Yield::Kind(_) => vec![field.to_string()],
            crate::manifest::Yield::Each(fields) => std::iter::once(field.to_string())
                .chain(fields.keys().map(ToString::to_string))
                .collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plan from the wire is refused before the run can reach for a step that is not there.
    #[test]
    fn a_plan_from_the_wire_holds_together() {
        let base = serde_json::json!({
            "input": "x", "steps": [], "exclusive": false, "stages": [], "verdict": { "outcome": "refuse" }
        });
        assert!(serde_json::from_value::<Weave>(base.clone()).is_ok());
        let mut staged = base.clone();
        staged["stages"] = serde_json::json!([[1]]);
        assert_eq!(
            serde_json::from_value::<Weave>(staged)
                .unwrap_err()
                .to_string(),
            "a stage names step 1, which the plan lacks"
        );
        let mut bound = base;
        bound["binds"] = serde_json::json!([{ "from": 0, "to": 1, "arg": "to", "field": "email", "kind": "email", "via": "fill" }]);
        assert_eq!(
            serde_json::from_value::<Weave>(bound)
                .unwrap_err()
                .to_string(),
            "a binding from step 0 to step 1 names no earlier step of the plan"
        );
    }
}
