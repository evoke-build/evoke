//! The schedule as the design words it, as code — the reference the tests compare the core with — and the
//! fixtures both models draw on: the spec's plan, the gate, and one decision per reflex taken from the spec's own
//! weave fixtures, so every step the models generate is a decision the core made. Each function quotes the
//! sentence of the design it stands for.

use evoke_core::adapter::Prob;
use evoke_core::manifest::Effect;
use evoke_core::weave::Status;
use evoke_core::{Decision, Gate, Plan};

/// `spec/fixtures/plan-weave.json`: lights, timer, volume, mail — writes — and contact, a read that yields an
/// address; the plan the weave vectors run under.
pub const PLAN: &str = include_str!("../../../../spec/fixtures/plan-weave.json");
/// `spec/fixtures/plan-joins.json`: errors, deploys and logs, reads that return their names; a suspect that takes
/// the three; incidents, incident and postmortem; a rollback — the plan the join vectors run under.
pub const JOINS: &str = include_str!("../../../../spec/fixtures/plan-joins.json");
const LIGHTS_TIMER: &str = include_str!("../../../../spec/fixtures/weave-lights-timer.json");
const ADDRESS_MAIL: &str = include_str!("../../../../spec/fixtures/weave-address-mail.json");
const SUSPECT: &str = include_str!("../../../../spec/fixtures/weave-suspect.json");

/// The plan as the vectors compile it.
///
/// # Panics
/// When the spec's fixture does not read: the spec changed under the model.
#[must_use]
pub fn plan() -> Plan {
    serde_json::from_str(PLAN).expect("the spec's plan reads")
}

/// The joins plan as the vectors compile it.
///
/// # Panics
/// When the spec's fixture does not read.
#[must_use]
pub fn joins() -> Plan {
    serde_json::from_str(JOINS).expect("the spec's joins plan reads")
}

/// The same plan with one reflex's effect changed: the schedule reads effects off the plan, so a generated one
/// puts each reflex under the effect the case wants.
///
/// # Panics
/// When the edited plan does not read.
#[must_use]
pub fn plan_with(effects: &[(&str, Effect)]) -> Plan {
    let mut json: serde_json::Value = serde_json::from_str(PLAN).expect("the spec's plan reads");
    for (reflex, effect) in effects {
        let word = match effect {
            Effect::Read => "read",
            Effect::Write => "write",
            Effect::Destructive => "destructive",
        };
        json["active"][*reflex]["effect"] = serde_json::Value::from(word);
    }
    serde_json::from_value(json).expect("the edited plan reads")
}

/// The gate the weave vectors run under.
///
/// # Panics
/// Never: the floors are fixed and read under write.
#[must_use]
pub fn gate() -> Gate {
    let p = |value: f64| Prob::new(value).expect("a probability");
    Gate::new(p(0.5), Some(p(0.3)), p(0.6), p(0.8)).expect("read under write")
}

/// A step's decision from a spec fixture: `lights` and `timer` run, `contact` runs, `mail` asks for `to`; the
/// three lookups of the outage run, and so does the `suspect` that takes their results.
///
/// # Panics
/// For a reflex no fixture decides, or a fixture that does not read.
#[must_use]
pub fn decision(reflex: &str) -> Decision {
    let (file, index) = match reflex {
        "lights" => (LIGHTS_TIMER, 0),
        "timer" => (LIGHTS_TIMER, 1),
        "contact" => (ADDRESS_MAIL, 0),
        "mail" => (ADDRESS_MAIL, 1),
        "errors" => (SUSPECT, 0),
        "deploys" => (SUSPECT, 1),
        "logs" => (SUSPECT, 2),
        "suspect" => (SUSPECT, 3),
        other => panic!("no fixture decides {other}"),
    };
    let json: serde_json::Value = serde_json::from_str(file).expect("the fixture reads");
    serde_json::from_value(json["steps"][index]["decision"].clone()).expect("the decision reads")
}

/// A decision that matches nothing: the words read as no reflex.
///
/// # Panics
/// Never: the abstain is a constant.
#[must_use]
pub fn abstain() -> Decision {
    serde_json::from_value(
        serde_json::json!({ "outcome": "abstain", "contenders": [], "judgments": [] }),
    )
    .expect("an abstain reads")
}

/// Layers by longest path over `after`: a step's layer is one more than the deepest step it follows. Steps are
/// numbered from 1; `after[n - 1]` names the steps step `n` follows.
#[must_use]
pub fn layers(after: &[Vec<usize>]) -> Vec<Vec<usize>> {
    fn depth(after: &[Vec<usize>], n: usize, memo: &mut Vec<Option<usize>>) -> usize {
        if let Some(d) = memo[n - 1] {
            return d;
        }
        let d = after[n - 1]
            .iter()
            .map(|m| depth(after, *m, memo) + 1)
            .max()
            .unwrap_or(0);
        memo[n - 1] = Some(d);
        d
    }
    let mut memo = vec![None; after.len()];
    let mut layers: Vec<Vec<usize>> = Vec::new();
    for n in 1..=after.len() {
        let d = depth(after, n, &mut memo);
        if layers.len() <= d {
            layers.resize(d + 1, Vec::new());
        }
        layers[d].push(n);
    }
    layers.retain(|layer| !layer.is_empty());
    layers
}

/// Whether a write is among the steps: «a write never runs beside anything, reads may». One step alone is never
/// beside anything.
#[must_use]
pub fn exclusive(effects: &[Effect]) -> bool {
    effects.len() > 1 && effects.iter().any(|effect| *effect != Effect::Read)
}

/// The stages: «`then` and a reference order the steps; a write never runs beside anything, reads may» — layers by
/// longest path, a layer of reads together when no write is among the steps, else one step at a time in the
/// words' order.
#[must_use]
pub fn stages(effects: &[Effect], after: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let exclusive = exclusive(effects);
    let mut stages = Vec::new();
    for layer in layers(after) {
        let reads = layer.iter().all(|n| effects[n - 1] == Effect::Read);
        if !exclusive && reads {
            stages.push(layer);
        } else {
            stages.extend(layer.into_iter().map(|n| vec![n]));
        }
    }
    stages
}

/// «The exit is the worst step's, a step skipped because its source yielded nothing to take counting as failed»:
/// ran, then failed, then declined or refused, then unanswered; a skipped step adds nothing of its own.
#[must_use]
pub fn worst(statuses: &[(Status, bool)]) -> Status {
    fn rank(status: Status) -> u8 {
        match status {
            Status::Ran | Status::Skipped => 0,
            Status::Failed => 1,
            Status::Declined | Status::Refused => 2,
            Status::Unanswered => 3,
        }
    }
    let mut worst = statuses
        .iter()
        .map(|(status, _)| *status)
        .max_by_key(|status| rank(*status))
        .unwrap_or(Status::Ran);
    if worst == Status::Skipped {
        worst = Status::Ran;
    }
    let nothing_to_take = statuses
        .iter()
        .any(|(status, nothing_to_take)| *status == Status::Skipped && *nothing_to_take);
    if nothing_to_take && rank(worst) < rank(Status::Failed) {
        worst = Status::Failed;
    }
    worst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fixtures_read() {
        assert_eq!(plan().active().len(), 5);
        assert!(matches!(decision("mail"), Decision::Ask { .. }));
        assert!(matches!(decision("contact"), Decision::Run { .. }));
        assert_eq!(
            plan_with(&[("lights", Effect::Read)]).active()["lights"].effect,
            Effect::Read
        );
    }

    #[test]
    fn layers_are_by_longest_path_and_writes_stand_alone() {
        // 3 follows 1 and 2; 4 follows 3.
        let after = vec![vec![], vec![], vec![1, 2], vec![3]];
        assert_eq!(layers(&after), [vec![1, 2], vec![3], vec![4]]);
        let reads = [Effect::Read; 4];
        assert_eq!(stages(&reads, &after), [vec![1, 2], vec![3], vec![4]]);
        let mut one_write = reads;
        one_write[3] = Effect::Write;
        assert_eq!(
            stages(&one_write, &after),
            [vec![1], vec![2], vec![3], vec![4]]
        );
        assert!(!exclusive(&[Effect::Write]));
    }

    #[test]
    fn the_worst_is_the_exit_codes_order() {
        assert_eq!(
            worst(&[(Status::Ran, false), (Status::Skipped, false)]),
            Status::Ran
        );
        assert_eq!(
            worst(&[(Status::Ran, false), (Status::Skipped, true)]),
            Status::Failed
        );
        assert_eq!(
            worst(&[(Status::Declined, false), (Status::Skipped, true)]),
            Status::Declined
        );
        assert_eq!(
            worst(&[(Status::Failed, false), (Status::Unanswered, false)]),
            Status::Unanswered
        );
    }
}
