//! Running a weave: stage by stage, a stage's steps together; a bound step takes its values from the results of
//! the steps it follows — answering its own ask through `fill`, or decided again with the values written into its
//! words — and the whole results it takes beside its decision, never in its words; then the foundation's own
//! loop, which a host takes it through. A step bound to a list of records runs once per record, one at a time.
//! A failure, a refusal or a decline ends the weave after the stage; what never ran is reported so. A round the
//! host ended — the weave cancelled — ends it too: nothing more is handed, and every step that had not finished
//! reads `skipped · cancelled`. Over the progress a host has gathered, stopping at the first thing it lacks. In:
//! a `Plan`, the adapter's gate, the `Weave`, the `Progress` so far. Out: the `Executed`, or what is needed next.

use indexmap::IndexMap;

use super::planning::reflex_of;
use super::{
    Asked, Binding, Bound, Executed, Handled, Handling, Progress, Repair, Returned, Running,
    Status, Step, StepOutcome, Todo, Via, Weave, Why,
};
use crate::adapter::Gate;
use crate::call::Value;
use crate::decide::{Decision, fill, merged, picked};
use crate::document::Json;
use crate::manifest::Recognizer;
use crate::name::ArgName;
use crate::plan::Plan;

/// The most a whole result may carry to its taker, in bytes of JSON: past it the step is skipped, `too_large`.
pub const TAKEN_CAP: usize = 1 << 20;

/// The run of a weave over the progress so far: what became of every step, or what is needed next.
#[must_use]
pub fn execute(plan: &Plan, gate: Option<&Gate>, weave: &Weave, progress: &Progress) -> Running {
    let runner = Runner {
        plan,
        gate,
        weave,
        progress,
        // A round the host ended: the weave is cancelled from here on — nothing more is handed, and every step
        // that had not finished reads skipped · cancelled.
        cancelled: progress
            .handled
            .iter()
            .any(|h| h.why == Some(Why::Cancelled)),
    };
    match runner.run() {
        Ok(executed) => Running::Done { executed },
        Err(todo) => Running::Todo { todo },
    }
}

struct Runner<'a> {
    plan: &'a Plan,
    gate: Option<&'a Gate>,
    weave: &'a Weave,
    progress: &'a Progress,
    cancelled: bool,
}

/// The bound values one round of a step takes.
type Values = Vec<(Binding, String)>;

/// The whole results a step takes, per taken argument: a constant of every round.
type Taken = IndexMap<ArgName, Json>;

/// A step's state as the run walks it.
struct Walk {
    status: Option<(Status, Option<Why>)>,
    bound: Vec<Bound>,
    rounds: Vec<Handled>,
    /// The last result the step gave: what a later step takes.
    result: Option<Returned>,
}

/// What a step's turn found in its sources: its rounds, with the values bound into each and the whole results
/// every round takes; an empty list of rounds when a source found nothing to take; or why the step is skipped.
enum Rounds {
    Ready {
        rounds: Vec<Values>,
        taken: Taken,
        /// The whole results as the step's line shows them: the argument, the source, the name, no value.
        held: Vec<Bound>,
    },
    FoundNothing,
    Skipped(Why),
}

impl Runner<'_> {
    fn run(&self) -> Result<Executed, Todo> {
        let mut walks: Vec<Walk> = self
            .weave
            .steps
            .iter()
            .map(|_| Walk {
                status: None,
                bound: Vec::new(),
                rounds: Vec::new(),
                result: None,
            })
            .collect();
        let mut stopped: Option<Why> = None;
        for stage in &self.weave.stages {
            if let Some(why) = &stopped {
                for &n in stage {
                    walks[n - 1].status = Some((Status::Skipped, Some(why.clone())));
                }
                continue;
            }
            let mut handling = Vec::new();
            for &n in stage {
                if walks[n - 1].status.is_some() {
                    continue;
                }
                let step = &self.weave.steps[n - 1];
                let binds: Vec<&Binding> = self.weave.binds.iter().filter(|b| b.to == n).collect();
                let (rounds, taken, held) = match rounds_for(&binds, &walks) {
                    Rounds::Skipped(why) => {
                        walks[n - 1].status = Some((Status::Skipped, Some(why)));
                        continue;
                    }
                    Rounds::FoundNothing => {
                        walks[n - 1].status = Some((Status::Skipped, Some(Why::FoundNothing)));
                        continue;
                    }
                    Rounds::Ready {
                        rounds,
                        taken,
                        held,
                    } => (rounds, taken, held),
                };
                let mut pending = false;
                for (i, values) in rounds.iter().enumerate() {
                    let bound: Vec<Bound> = held.iter().cloned().chain(bound_of(values)).collect();
                    let walk = &mut walks[n - 1];
                    match self.turn(walk, step, i, values, &taken, bound)? {
                        Turn::Ran => continue,
                        Turn::Handed(next) => {
                            handling.push(*next);
                            pending = true;
                        }
                        Turn::Stopped => {}
                    }
                    break;
                }
                let walk = &mut walks[n - 1];
                if walk.status.is_none() && !pending {
                    walk.status = Some((Status::Ran, None));
                }
            }
            if !handling.is_empty() {
                return Err(Todo::Handle { handling });
            }
            stopped = stopped_after(stage, &walks);
        }
        Ok(Executed::of(
            walks
                .into_iter()
                .zip(&self.weave.steps)
                .map(|(walk, step)| {
                    let (status, why) = walk.status.unwrap_or((Status::Ran, None));
                    StepOutcome {
                        step: step.n,
                        status,
                        why,
                        bound: walk.bound,
                        rounds: walk.rounds,
                    }
                })
                .collect(),
        ))
    }

    /// One round at its turn: reported by the host already — its result kept for the next round and for what
    /// takes from it, or the stop it met; cancelled, with every round after it; refused once its values were in
    /// place; or handed to the host.
    fn turn(
        &self,
        walk: &mut Walk,
        step: &Step,
        i: usize,
        values: &Values,
        taken: &Taken,
        bound: Vec<Bound>,
    ) -> Result<Turn, Todo> {
        if let Some(handled) = self.handled(step.n, i) {
            walk.bound.extend(bound);
            walk.rounds.push(handled.clone());
            if handled.status == Status::Ran {
                walk.result.clone_from(&handled.result);
                return Ok(Turn::Ran);
            }
            walk.status = Some((handled.status, handled.why.clone()));
            return Ok(Turn::Stopped);
        }
        if self.cancelled {
            walk.status = Some((Status::Skipped, Some(Why::Cancelled)));
            return Ok(Turn::Stopped);
        }
        match self.round(step, i, values, taken, bound.clone())? {
            Prepared::Refused(why) => {
                walk.bound.extend(bound);
                walk.status = Some((Status::Refused, Some(why)));
                Ok(Turn::Stopped)
            }
            Prepared::Handle(next) => {
                walk.bound.extend(bound);
                Ok(Turn::Handed(next))
            }
        }
    }

    /// What the host made of a round, when it has.
    fn handled(&self, step: usize, round: usize) -> Option<&Handled> {
        self.progress
            .handled
            .iter()
            .find(|h| h.step == step && h.round == round)
    }

    /// One round prepared for the host: the step's decision as planned, or with its bound values in place —
    /// answering its ask, or decided again with the values in its words, which needs the host first — and the
    /// whole results it takes beside it, which never reach its words.
    fn round(
        &self,
        step: &Step,
        i: usize,
        values: &Values,
        taken: &Taken,
        bound: Vec<Bound>,
    ) -> Result<Prepared, Todo> {
        let handling = |decision: Decision, input: String| {
            Prepared::Handle(Box::new(Handling {
                step: step.n,
                round: i,
                decision,
                input,
                bound,
                taken: taken.clone(),
            }))
        };
        if values.is_empty() {
            return Ok(handling(step.decision.clone(), step.text.clone()));
        }
        // A step bound both ways is rewritten: the rewrite reaches a required argument too; so is one whose ask
        // is gone, filled up front by the host.
        if values.iter().all(|(b, _)| b.via == Via::Fill)
            && let Decision::Ask { asking, .. } = &step.decision
        {
            let given: IndexMap<ArgName, Value> = values
                .iter()
                .filter_map(|(b, text)| {
                    let value = picked(text, b.kind?)?;
                    Some((b.arg.clone(), value))
                })
                .collect();
            let decision = self.merged(step, fill(self.plan, asking.clone(), given, self.gate));
            return Ok(handling(decision, step.text.clone()));
        }
        let Some(reflex) = &step.reflex else {
            return Ok(Prepared::Refused(Why::NoReflex));
        };
        let text = rewrite(step, values);
        let asked = Asked {
            text: text.clone(),
            tags: Vec::new(),
            only: Some(reflex.clone()),
        };
        let Some((_, decision)) = self.progress.decided.iter().find(|(a, _)| *a == asked) else {
            return Err(Todo::Decide {
                step: step.n,
                round: i,
                asked,
            });
        };
        if matches!(decision, Decision::Abstain { .. }) {
            return Ok(Prepared::Refused(Why::NoReflex));
        }
        if let Some(read) = reflex_of(decision)
            && read != reflex
        {
            return Ok(Prepared::Refused(Why::ReadAs {
                reflex: read.clone(),
            }));
        }
        Ok(handling(self.merged(step, decision.clone()), text))
    }

    /// A step merged back never runs unasked: its decision at its turn confirms, as the planner's did.
    fn merged(&self, step: &Step, decision: Decision) -> Decision {
        if step.repair == Some(Repair::Merged) {
            merged(self.plan, decision)
        } else {
            decision
        }
    }
}

/// Why the weave ends after this stage, if it does: a cancelled round, with its own reason; else a step that
/// did not run — a step that found nothing to do skipped clean and stops nothing.
fn stopped_after(stage: &[usize], walks: &[Walk]) -> Option<Why> {
    let status = |n: &usize| walks[n - 1].status.as_ref();
    if stage
        .iter()
        .any(|n| matches!(status(n), Some((Status::Skipped, Some(Why::Cancelled)))))
    {
        return Some(Why::Cancelled);
    }
    let ran = |n: &usize| {
        matches!(
            status(n),
            Some((Status::Ran, _) | (Status::Skipped, Some(Why::FoundNothing)))
        )
    };
    (!stage.iter().all(ran)).then_some(Why::EarlierStep)
}

/// What a round's turn came to: ran, so the next round follows; handed to the host; or the step stopped here.
enum Turn {
    Ran,
    Handed(Box<Handling>),
    Stopped,
}

/// A round ready: for the host to take through the loop, or refused once the values were in place.
enum Prepared {
    Handle(Box<Handling>),
    Refused(Why),
}

fn bound_of(values: &Values) -> Vec<Bound> {
    values
        .iter()
        .map(|(b, value)| Bound {
            arg: b.arg.clone(),
            from: b.from,
            field: b.field.clone(),
            value: Some(value.clone()),
        })
        .collect()
}

/// The rounds a step runs: one, with the bound values from the results of the steps it follows; one per record
/// when a binding takes a field of a list's records; none when a source yielded no such field, whatever another
/// source found; an empty list of rounds when a source found nothing to take. A whole result is a constant of
/// every round: no data or `null` is nothing to take, and more than the cap is not handed.
fn rounds_for(binds: &[&Binding], walks: &[Walk]) -> Rounds {
    let mut plain: Values = Vec::new();
    let mut lists: Vec<(Binding, Vec<String>)> = Vec::new();
    let mut taken = Taken::new();
    let mut held = Vec::new();
    let mut found_nothing = false;
    for binding in binds {
        // A source that found nothing skipped clean; so does what takes from it — once every other source is
        // known to have yielded its field, so the bindings' order decides nothing.
        if walks.get(binding.from - 1).is_some_and(|walk| {
            matches!(
                walk.status,
                Some((Status::Skipped, Some(Why::FoundNothing)))
            )
        }) {
            found_nothing = true;
            continue;
        }
        let nothing = Rounds::Skipped(Why::NothingToTake { from: binding.from });
        let result = walks
            .get(binding.from - 1)
            .and_then(|walk| walk.result.as_ref())
            .and_then(|result| result.data.as_ref());
        if binding.via == Via::Takes {
            let Some(data) = result.filter(|data| !data.is_null()) else {
                return nothing;
            };
            if data.to_string().len() > TAKEN_CAP {
                return Rounds::Skipped(Why::TooLarge { from: binding.from });
            }
            taken.insert(binding.arg.clone(), data.clone());
            held.push(Bound {
                arg: binding.arg.clone(),
                from: binding.from,
                field: binding.field.clone(),
                value: None,
            });
            continue;
        }
        let data = result.and_then(Json::as_object);
        let Some(each) = &binding.each else {
            let Some(value) = data
                .and_then(|data| data.get(binding.field.as_str()))
                .and_then(scalar)
            else {
                return nothing;
            };
            plain.push(((*binding).clone(), value));
            continue;
        };
        let Some(records) = data
            .and_then(|data| data.get(each.as_str()))
            .and_then(Json::as_array)
        else {
            return nothing;
        };
        let values: Option<Vec<String>> = records
            .iter()
            .map(|record| record.get(binding.field.as_str()).and_then(scalar))
            .collect();
        let Some(values) = values else {
            return nothing;
        };
        lists.push(((*binding).clone(), values));
    }
    if found_nothing {
        return Rounds::FoundNothing;
    }
    if lists.is_empty() {
        return Rounds::Ready {
            rounds: vec![plain],
            taken,
            held,
        };
    }
    let count = lists
        .iter()
        .map(|(_, values)| values.len())
        .min()
        .unwrap_or(0);
    // A list with no record is a source that found nothing: the step skips clean.
    if count == 0 {
        return Rounds::FoundNothing;
    }
    Rounds::Ready {
        rounds: (0..count)
            .map(|i| {
                let mut round = plain.clone();
                round.extend(
                    lists
                        .iter()
                        .map(|(binding, values)| (binding.clone(), values[i].clone())),
                );
                round
            })
            .collect(),
        taken,
        held,
    }
}

/// A string or a number of a result's data, as text; anything else is nothing to take.
fn scalar(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Number(number) => Some(
            number
                .as_i64()
                .map_or_else(|| number.to_string(), |i| i.to_string()),
        ),
        _ => None,
    }
}

/// The step's words with the values in place: each over the reference that named its source, several from one
/// source joined with «and»; a value no reference names is appended. A quoted value is written in quotes; a
/// number, an address or a URL bare, as its recognizer reads it.
#[must_use]
pub fn rewrite(step: &Step, values: &Values) -> String {
    let literal = |(binding, value): &(Binding, String)| {
        if binding.kind == Some(Recognizer::Quoted) {
            format!("\"{value}\"")
        } else {
            value.clone()
        }
    };
    let mut over: Vec<Vec<String>> = vec![Vec::new(); step.refs.len()];
    let mut loose: Vec<String> = Vec::new();
    for value in values {
        match step
            .refs
            .iter()
            .position(|r| r.from.contains(&(value.0.from.saturating_sub(1))))
        {
            Some(i) => over[i].push(literal(value)),
            None => loose.push(literal(value)),
        }
    }
    let chars: Vec<char> = step.text.chars().collect();
    let mut spans: Vec<(usize, usize, &Vec<String>)> = step
        .refs
        .iter()
        .zip(&over)
        .filter(|(_, values)| !values.is_empty())
        .map(|(r, values)| {
            (
                r.span.start.min(chars.len()),
                r.span.end.min(chars.len()),
                values,
            )
        })
        .collect();
    // The text is rebuilt once, left to right, so every reference is replaced, not the last one alone.
    spans.sort_by_key(|(start, _, _)| *start);
    let mut text = String::new();
    let mut at = 0;
    for (start, end, values) in spans {
        let start = start.max(at);
        text.extend(&chars[at..start]);
        text.push_str(&values.join(" and "));
        at = end.max(start);
    }
    text.extend(&chars[at..]);
    if loose.is_empty() {
        text
    } else {
        format!("{text} {}", loose.join(" and "))
    }
}

#[cfg(test)]
mod tests {
    use super::super::reading::Ref;
    use super::super::{Binding, Weave};
    use super::*;

    /// «email them» from the address weave, given a second reference and two bound values: both are written
    /// over their references, left to right.
    #[test]
    fn every_bound_reference_is_written_over() {
        let weave: Weave = serde_json::from_str(include_str!(
            "../../../../spec/fixtures/weave-address-mail.json"
        ))
        .unwrap();
        let mut step = weave.steps[1].clone();
        step.text = "email them about them".to_owned();
        let second: Ref = serde_json::from_value(serde_json::json!({
            "span": { "start": 17, "end": 21, "text": "them" }, "from": [1], "how": "pronoun"
        }))
        .unwrap();
        step.refs.push(second);
        let bind = |from: usize, arg: &str| -> Binding {
            serde_json::from_value(serde_json::json!({
                "from": from, "to": 3, "arg": arg, "field": "email", "kind": "email", "via": "rewrite"
            }))
            .unwrap()
        };
        let values: Values = vec![
            (bind(1, "to"), "dana@example.com".to_owned()),
            (bind(2, "cc"), "bob@example.com".to_owned()),
        ];
        assert_eq!(
            rewrite(&step, &values),
            "email dana@example.com about bob@example.com"
        );
    }

    /// The suspect's plan over the joins fixture, its three sources ran: a whole result at the cap is handed,
    /// one byte over it skips the taker, `too_large` naming the source, and the whole counts as failed.
    #[test]
    fn a_whole_result_over_the_cap_is_not_handed() {
        let plan: Plan =
            serde_json::from_str(include_str!("../../../../spec/fixtures/plan-joins.json"))
                .unwrap();
        let weave: Weave =
            serde_json::from_str(include_str!("../../../../spec/fixtures/weave-suspect.json"))
                .unwrap();
        let ran = |step: usize, data: Json| Handled {
            step,
            round: 0,
            status: Status::Ran,
            why: None,
            result: Some(Returned {
                text: "ok".to_owned(),
                data: Some(data),
            }),
        };
        // `{"logs":"<text>"}` is the text's length plus eleven bytes of JSON around it.
        let logs = |text_len: usize| serde_json::json!({ "logs": "x".repeat(text_len) });
        let progress_with = |logs: Json| Progress {
            decided: Vec::new(),
            handled: vec![
                ran(1, serde_json::json!({ "rate": 0.084 })),
                ran(2, serde_json::json!({ "deploys": [] })),
                ran(3, logs),
            ],
        };
        let at_cap = execute(&plan, None, &weave, &progress_with(logs(TAKEN_CAP - 11)));
        let Running::Todo {
            todo: Todo::Handle { handling },
        } = at_cap
        else {
            panic!("a result at the cap is handed");
        };
        assert_eq!(handling[0].taken.len(), 3);
        let over = execute(&plan, None, &weave, &progress_with(logs(TAKEN_CAP - 10)));
        let Running::Done { executed } = over else {
            panic!("a result over the cap skips the taker");
        };
        assert_eq!(executed.steps[3].status, Status::Skipped);
        assert_eq!(executed.steps[3].why, Some(Why::TooLarge { from: 3 }));
        assert_eq!(executed.worst, Status::Failed);
    }
}
