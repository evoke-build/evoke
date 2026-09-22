//! Running a weave: stage by stage, a stage's steps together; a bound step takes its values from the results of
//! the steps it follows — answering its own ask through `fill`, or decided again with the values written into its
//! words — then the foundation's own loop, which a host takes it through. A step bound to a list of records runs
//! once per record, one at a time. A failure, a refusal or a decline ends the weave after the stage; what never
//! ran is reported so. Over the progress a host has gathered, stopping at the first thing it lacks. In: a
//! `Plan`, the adapter's gate, the `Weave`, the `Progress` so far. Out: the `Executed`, or what is needed next.

use indexmap::IndexMap;

use super::planning::reflex_of;
use super::{
    Asked, Binding, Bound, Executed, Handled, Handling, Progress, Returned, Running, Status, Step,
    StepOutcome, Todo, Via, Weave, Why,
};
use crate::adapter::Gate;
use crate::call::Value;
use crate::decide::{Decision, fill, picked};
use crate::document::Json;
use crate::manifest::Recognizer;
use crate::name::ArgName;
use crate::plan::Plan;

/// The run of a weave over the progress so far: what became of every step, or what is needed next.
#[must_use]
pub fn execute(plan: &Plan, gate: Option<&Gate>, weave: &Weave, progress: &Progress) -> Running {
    let runner = Runner {
        plan,
        gate,
        weave,
        progress,
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
}

/// The bound values one round of a step takes.
type Values = Vec<(Binding, String)>;

/// A step's state as the run walks it.
struct Walk {
    status: Option<(Status, Option<Why>)>,
    bound: Vec<Bound>,
    rounds: Vec<Handled>,
    /// The last result the step gave: what a later step takes.
    result: Option<Returned>,
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
        let mut stopped = false;
        for stage in &self.weave.stages {
            if stopped {
                for &n in stage {
                    walks[n - 1].status = Some((Status::Skipped, Some(Why::EarlierStep)));
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
                let rounds = match rounds_for(&binds, &walks) {
                    None => {
                        walks[n - 1].status = Some((Status::Skipped, Some(Why::NothingToTake)));
                        continue;
                    }
                    Some(rounds) if rounds.is_empty() => {
                        walks[n - 1].status = Some((Status::Skipped, Some(Why::FoundNothing)));
                        continue;
                    }
                    Some(rounds) => rounds,
                };
                let mut pending = false;
                for (i, values) in rounds.iter().enumerate() {
                    let walk = &mut walks[n - 1];
                    if let Some(handled) = self.handled(n, i) {
                        walk.bound.extend(bound_of(values));
                        walk.rounds.push(handled.clone());
                        if handled.status == Status::Ran {
                            walk.result.clone_from(&handled.result);
                            continue;
                        }
                        walk.status = Some((handled.status, handled.why.clone()));
                        break;
                    }
                    match self.round(step, i, values)? {
                        Prepared::Refused(why) => {
                            walk.bound.extend(bound_of(values));
                            walk.status = Some((Status::Refused, Some(why)));
                        }
                        Prepared::Handle(next) => {
                            walk.bound.extend(bound_of(values));
                            handling.push(*next);
                            pending = true;
                        }
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
            if stage.iter().any(|&n| {
                walks[n - 1].status.as_ref().map(|(status, _)| *status) != Some(Status::Ran)
            }) {
                stopped = true;
            }
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

    /// What the host made of a round, when it has.
    fn handled(&self, step: usize, round: usize) -> Option<&Handled> {
        self.progress
            .handled
            .iter()
            .find(|h| h.step == step && h.round == round)
    }

    /// One round prepared for the host: the step's decision as planned, or with its bound values in place —
    /// answering its ask, or decided again with the values in its words, which needs the host first.
    fn round(&self, step: &Step, i: usize, values: &Values) -> Result<Prepared, Todo> {
        let bound = bound_of(values);
        if values.is_empty() {
            return Ok(Prepared::Handle(Box::new(Handling {
                step: step.n,
                round: i,
                decision: step.decision.clone(),
                input: step.text.clone(),
                bound,
            })));
        }
        // A step bound both ways is rewritten: the rewrite reaches a required argument too; so is one whose ask
        // is gone, filled up front by the host.
        if values.iter().all(|(b, _)| b.via == Via::Fill)
            && let Decision::Ask { asking, .. } = &step.decision
        {
            let given: IndexMap<ArgName, Value> = values
                .iter()
                .filter_map(|(b, text)| picked(text, b.kind).map(|value| (b.arg.clone(), value)))
                .collect();
            let decision = fill(self.plan, asking.clone(), given, self.gate);
            return Ok(Prepared::Handle(Box::new(Handling {
                step: step.n,
                round: i,
                decision,
                input: step.text.clone(),
                bound,
            })));
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
            return Err(Todo::Decide { asked });
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
        Ok(Prepared::Handle(Box::new(Handling {
            step: step.n,
            round: i,
            decision: decision.clone(),
            input: text,
            bound,
        })))
    }
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
            value: value.clone(),
        })
        .collect()
}

/// The rounds a step runs: one, with the bound values from the results of the steps it follows; one per record
/// when a binding takes a field of a list's records; none when a source yielded no such field.
fn rounds_for(binds: &[&Binding], walks: &[Walk]) -> Option<Vec<Values>> {
    let mut plain: Values = Vec::new();
    let mut lists: Vec<(Binding, Vec<String>)> = Vec::new();
    for binding in binds {
        let data = walks
            .get(binding.from - 1)
            .and_then(|walk| walk.result.as_ref())
            .and_then(|result| result.data.as_ref())
            .and_then(Json::as_object);
        let Some(each) = &binding.each else {
            let value = data
                .and_then(|data| data.get(binding.field.as_str()))
                .and_then(scalar)?;
            plain.push(((*binding).clone(), value));
            continue;
        };
        let records = data
            .and_then(|data| data.get(each.as_str()))
            .and_then(Json::as_array)?;
        let values: Option<Vec<String>> = records
            .iter()
            .map(|record| record.get(binding.field.as_str()).and_then(scalar))
            .collect();
        lists.push(((*binding).clone(), values?));
    }
    if lists.is_empty() {
        return Some(vec![plain]);
    }
    let count = lists
        .iter()
        .map(|(_, values)| values.len())
        .min()
        .unwrap_or(0);
    Some(
        (0..count)
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
    )
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

/// The step's words with the values in place: over the reference when one was found, else appended. A quoted
/// value is written in quotes; a number, an address or a URL bare, as its recognizer reads it.
#[must_use]
pub fn rewrite(step: &Step, values: &Values) -> String {
    let literal = values
        .iter()
        .map(|(b, value)| {
            if b.kind == Recognizer::Quoted {
                format!("\"{value}\"")
            } else {
                value.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" and ");
    let Some(span) = step.refs.first().map(|r| &r.span) else {
        return format!("{} {literal}", step.text);
    };
    let chars: Vec<char> = step.text.chars().collect();
    let before: String = chars[..span.start.min(chars.len())].iter().collect();
    let after: String = chars[span.end.min(chars.len())..].iter().collect();
    format!("{before}{literal}{after}")
}
