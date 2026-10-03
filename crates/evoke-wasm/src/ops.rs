//! The op table: one name per function of the core's surface and per adapter mapping. In: the op and one JSON
//! object holding the function's arguments by name, as the vectors write them. Out: `{ ok }` with the result,
//! `{ err }` with the function's own error, or `{ bug }` for an op or an input the SDK should never have sent.

use evoke_adapters::replay;
use evoke_adapters::systemone::{self, Door};
use evoke_core::calendar::Date;
use evoke_core::contain::Platform;
use evoke_core::decide::{Recent, alone};
use evoke_core::document::Text;
use evoke_core::manifest::{ConfigSpec, Effect, Pick, Range, Recognizer};
use evoke_core::name::AdapterId;
use evoke_core::name::{ArgName, ConfigKey, LocalName, RelPath, Tag, VocabName};
use evoke_core::plan::Millis;
use evoke_core::project::Location;
use evoke_core::spoken::heard;
use evoke_core::text::NonEmpty;
use evoke_core::{
    Active, Answer, Baseline, Call, Case, Chosen, Clean, Decision, Declared, Digest, Document,
    Facts, Fault, Fix, Gate, Input, Installed, Judgment, Key, Lesson, Limits, Lock, Logged,
    Manifest, Needs, Overlay, Pinned, Plan, Policy, Prob, Project, Raw, Request, Routed, Scope,
    Utterance, Value, Values, Verdict, Weave, Written, add_entry, argv, baseline, by_name,
    calibrate, call as call_grammar, cases, compile, compose, consent, diff, effective, envelope,
    fill, gate, identity, judge, landlock, lint, listed, lock, log_block, manifest, needs,
    node_flags, overlay, picked, pin, pinned, project, project_dts, propose, read, reference,
    reflex_dts, regressions, remove_entry, render_lock, replan, report, request, resolve, seatbelt,
    set_config, spelled, stale, teach, thieves, vocab_edit, vocabulary, weave, widens,
};
use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value as Json, json};

/// The op table's answer: `{ ok }`, `{ err }`, or `{ bug }` naming the op and what was wrong with the call.
#[must_use]
pub fn call(op: &str, input: &Json) -> Json {
    match answer(op, input) {
        Ok(reply) => reply,
        Err(Bug(message)) => json!({ "bug": format!("{op}: {message}") }),
    }
}

/// A call the SDK should never make: an op that does not exist, or an input that does not read as the op's
/// arguments.
struct Bug(String);

type Reply = Result<Json, Bug>;

/// The op table in six parts — parse and customize, decide and run, the weave and its plan files, needs and
/// containment, tune and author, the adapters — each falling through to the next.
fn answer(op: &str, input: &Json) -> Reply {
    Ok(match op {
        "identity" => ok(identity(text(input, "text")?)),
        "manifest" => result(manifest(document(input, "doc")?)),
        "overlay" => result(overlay(document(input, "doc")?, &arg(input, "of")?)),
        "vocabulary" => result(vocabulary(document(input, "doc")?)),
        "project" => result(project(document(input, "doc")?)),
        "lock" => result(lock(document(input, "doc")?)),
        "render_lock" => ok(render_lock(&arg::<Lock>(input, "lock")?)),
        "location" => ok(arg::<Location>(input, "location")?.to_string()),
        "name" => match text(input, "kind")? {
            "local" => result(LocalName::new(text(input, "text")?)),
            "vocab" => result(VocabName::new(text(input, "text")?)),
            kind => return Err(Bug(format!("kind: \"{kind}\" is neither local nor vocab"))),
        },
        "reference" => result(reference(text(input, "text")?)),
        "digest" => ok(compose(&arg::<Vec<(RelPath, Digest)>>(input, "hashed")?)),
        "version" => ok(env!("CARGO_PKG_VERSION")),
        "fault" => {
            let fault: Fault = arg(input, "fault")?;
            let invoked = opt::<String>(input, "invoked")?.unwrap_or_default();
            ok(json!({ "message": fault.to_string(), "command": fault.fix().command(&invoked) }))
        }
        "fix" => ok(arg::<Fix>(input, "fix")?.command(
            opt::<String>(input, "invoked")?
                .as_deref()
                .unwrap_or_default(),
        )),
        "effective" => {
            let shipped: Manifest = arg(input, "shipped")?;
            ok(effective(&shipped, yours(input, &shipped)?.as_ref()))
        }
        "report" => {
            let previous: Manifest = arg(input, "previous")?;
            let next: Manifest = arg(input, "next")?;
            ok(report(&previous, &next, yours(input, &next)?.as_ref()))
        }
        _ => return decide(op, input),
    })
}

fn decide(op: &str, input: &Json) -> Reply {
    Ok(match op {
        "compile" => result(compile(
            &arg::<Installed>(input, "set")?,
            &opt::<Values>(input, "values")?.unwrap_or_default(),
            opt::<Limits>(input, "limits")?.as_ref(),
            opt::<Platform>(input, "platform")?,
        )),
        "propose" => ok(propose(&arg::<Input>(input, "input")?)),
        "words.listed" => ok(listed(
            &arg::<Input>(input, "input")?,
            &arg::<IndexMap<Key, Clean>>(input, "list")?,
        )),
        "words.spelled" => {
            let said = arg::<Input>(input, "input")?;
            ok(spelled(
                &said,
                arg::<Recognizer>(input, "kind")?,
                &propose(&said),
            ))
        }
        "words.spoken" => {
            let said = arg::<Input>(input, "input")?;
            let pick = match (
                arg::<Recognizer>(input, "kind")?.unranged(),
                opt::<Range<f64>>(input, "range")?,
            ) {
                (Pick::Number(_), range) => Pick::Number(range),
                (pick, _) => pick,
            };
            ok(heard(
                &said,
                &pick,
                &opt::<Vec<Clean>>(input, "examples")?.unwrap_or_default(),
                &propose(&said),
            ))
        }
        "request" => result(request(
            &arg::<Plan>(input, "plan")?,
            text(input, "input")?,
            &arg::<Vec<Tag>>(input, "tags")?,
            opt::<LocalName>(input, "only")?.as_ref(),
            opt::<Judgment>(input, "named")?.as_ref(),
            arg::<Scope>(input, "scope")?,
            &opt::<Vec<Recent>>(input, "recent")?.unwrap_or_default(),
        )),
        "read" => result(read(
            &arg::<Plan>(input, "plan")?,
            opt::<Gate>(input, "gate")?.as_ref(),
            &arg::<Request>(input, "request")?,
            arg::<Raw>(input, "raw")?,
        )),
        "gate" => {
            let plan = arg::<Plan>(input, "plan")?;
            let decision = gate(
                &plan,
                arg(input, "reading")?,
                opt::<Gate>(input, "gate")?.as_ref(),
            );
            // A decision no plan stands around is held as a plan holds a step no other step repeats.
            ok(if opt::<bool>(input, "alone")?.unwrap_or(false) {
                alone(&plan, decision)
            } else {
                decision
            })
        }
        "fill" => ok(fill(
            &arg::<Plan>(input, "plan")?,
            arg(input, "asking")?,
            arg::<IndexMap<ArgName, Value>>(input, "given")?,
            opt::<Gate>(input, "gate")?.as_ref(),
        )),
        "by_name" => result(by_name(
            &arg::<Plan>(input, "plan")?,
            arg::<Written>(input, "written")?,
        )),
        "values" => ok(arg::<IndexMap<ArgName, Value>>(input, "args")?
            .iter()
            .map(|(name, value)| (name.clone(), value.plain()))
            .collect::<IndexMap<_, _>>()),
        "picked" => ok(picked(
            text(input, "text")?,
            arg::<Recognizer>(input, "recognizer")?,
        )),
        "call" => result(call_grammar(text(input, "text")?)),
        "envelope" => result(envelope(
            &arg::<Chosen>(input, "chosen")?,
            &arg::<Active>(input, "active")?,
            &opt::<IndexMap<ArgName, Json>>(input, "taken")?.unwrap_or_default(),
            &arg::<Input>(input, "input")?,
            arg::<Millis>(input, "deadline")?,
            text(input, "home")?,
            arg::<Date>(input, "today")?,
        )),
        "argv" => result(argv(
            &arg::<Chosen>(input, "chosen")?,
            &arg::<Active>(input, "active")?,
            text(input, "home")?,
            arg::<Date>(input, "today")?,
        )),
        _ => return weaving(op, input),
    })
}

fn weaving(op: &str, input: &Json) -> Reply {
    Ok(match op {
        "weave.plan" => result(weave::planning::plan(
            &arg::<Plan>(input, "plan")?,
            opt::<Gate>(input, "gate")?.as_ref(),
            text(input, "input")?,
            &arg::<Vec<Tag>>(input, "tags")?,
            &arg::<weave::Answers>(input, "answers")?,
        )),
        "weave.execute" => ok(weave::running::execute(
            &arg::<Plan>(input, "plan")?,
            opt::<Gate>(input, "gate")?.as_ref(),
            &arg::<Weave>(input, "weave")?,
            &arg::<weave::Progress>(input, "progress")?,
        )),
        "weave.pin" => ok(pin(
            &arg::<Installed>(input, "installed")?,
            &arg::<Plan>(input, "plan")?,
            &arg::<Project>(input, "project")?,
            opt::<Lock>(input, "lock")?.as_ref(),
            &arg::<Declared>(input, "declared")?,
            text(input, "input")?,
            &arg::<Vec<Tag>>(input, "tags")?,
            arg::<Weave>(input, "weave")?,
            arg::<Vec<Answer>>(input, "answers")?,
        )),
        "weave.pinned" => result(pinned(
            text(input, "path")?,
            input
                .get("json")
                .cloned()
                .ok_or_else(|| Bug("json is missing".to_owned()))?,
        )),
        "weave.stale" => result(stale(
            text(input, "path")?,
            &arg::<Pinned>(input, "pinned")?,
            &arg::<Installed>(input, "installed")?,
            &arg::<Plan>(input, "plan")?,
            opt::<Lock>(input, "lock")?.as_ref(),
            &arg::<Declared>(input, "declared")?,
        )),
        "weave.replan" => result(replan(
            text(input, "path")?,
            &arg::<Pinned>(input, "pinned")?,
            &arg::<Plan>(input, "plan")?,
        )),
        _ => return needs_and_contain(op, input),
    })
}

fn needs_and_contain(op: &str, input: &Json) -> Reply {
    Ok(match op {
        "needs.resolve" => result(resolve(
            &arg::<Needs>(input, "needs")?,
            &arg::<Call>(input, "call")?,
            &arg::<Active>(input, "active")?,
            &arg::<IndexMap<ConfigKey, String>>(input, "config")?,
            text(input, "home")?,
            arg::<Date>(input, "today")?,
        )),
        "contain.seatbelt" => ok(seatbelt(
            &arg::<Policy>(input, "policy")?,
            &arg::<Facts>(input, "facts")?,
        )),
        "contain.node_flags" => ok(node_flags(
            &arg::<Policy>(input, "policy")?,
            &arg::<Facts>(input, "facts")?,
        )),
        "contain.landlock" => ok(landlock(
            &arg::<Policy>(input, "policy")?,
            &arg::<Facts>(input, "facts")?,
        )),
        "needs.declared_at" => ok(needs::declared_at(document(input, "doc")?)),
        "needs.lacking" => ok(needs::lacking(
            &arg::<needs::Lacking>(input, "lacking")?,
            &arg::<LocalName>(input, "reflex")?,
            &arg::<Active>(input, "active")?,
            &arg::<needs::Origin>(input, "origin")?,
            text(input, "home")?,
        )),
        "needs.refusal" => ok(needs::refusal(
            &arg::<Policy>(input, "policy")?,
            opt::<Policy>(input, "upstream")?.as_ref(),
            &arg::<LocalName>(input, "reflex")?,
            &arg::<needs::Origin>(input, "origin")?,
            &arg::<needs::Refused>(input, "refused")?,
            text(input, "home")?,
        )),
        _ => return tune(op, input),
    })
}

fn tune(op: &str, input: &Json) -> Reply {
    Ok(match op {
        "teach" => {
            let plan: Plan = arg(input, "plan")?;
            let utterance: Utterance = arg(input, "utterance")?;
            let lesson = input
                .get("lesson")
                .ok_or_else(|| Bug("lesson is missing".to_owned()))?;
            result(match Lesson::from_json(lesson, &plan) {
                Ok(lesson) => teach(&utterance, lesson, &plan),
                Err(refused) => Err(refused),
            })
        }
        "set_config" => result(set_config(
            arg(input, "name")?,
            arg(input, "key")?,
            arg(input, "setting")?,
            &arg::<ConfigSpec>(input, "spec")?,
        )),
        "vocab_edit" => ok(vocab_edit(arg(input, "name")?, arg(input, "change")?)),
        "add_entry" => ok(add_entry(&arg(input, "name")?, &arg(input, "location")?)),
        "remove_entry" => ok(remove_entry(&arg(input, "name")?)),
        "diff" => ok(diff(
            &arg::<Manifest>(input, "previous")?,
            &arg::<Manifest>(input, "next")?,
        )),
        "consent" => ok(consent(
            arg::<Effect>(input, "locked")?,
            arg::<Effect>(input, "upstream")?,
        )),
        "needs.widens" => ok(widens(
            &arg::<Needs>(input, "from")?,
            &arg::<Needs>(input, "to")?,
        )),
        "needs.consent" => ok(needs::consent(
            &arg::<Needs>(input, "locked")?,
            &arg::<Needs>(input, "upstream")?,
        )),
        "lint" => ok(lint(&arg::<Manifest>(input, "manifest")?)),
        "reflex_dts" => ok(reflex_dts(&arg::<Manifest>(input, "manifest")?)),
        "project_dts" => ok(project_dts(&arg::<Installed>(input, "set")?)),
        "cases" => ok(cases(&arg::<Installed>(input, "set")?)),
        "judge" => ok(judge(
            &arg::<Case>(input, "case")?,
            &arg::<Decision>(input, "decision")?,
        )),
        "regressions" => ok(regressions(
            &arg::<Baseline>(input, "before")?,
            &arg::<Vec<(Case, NonEmpty<Verdict>)>>(input, "judged")?,
        )),
        "baseline" => ok(baseline(
            &arg::<Baseline>(input, "before")?,
            &arg::<Vec<(Case, NonEmpty<Verdict>)>>(input, "judged")?,
        )),
        "thieves" => ok(thieves(
            &arg::<Vec<LocalName>>(input, "newcomers")?,
            &arg::<Vec<Routed>>(input, "routed")?,
            opt::<Prob>(input, "floor")?,
        )),
        "calibrate" => ok(calibrate(
            &arg::<AdapterId>(input, "adapter")?,
            opt::<Gate>(input, "gate")?.as_ref(),
            &arg::<Plan>(input, "plan")?,
            &arg::<Vec<(Case, NonEmpty<Decision>)>>(input, "judged")?,
        )),
        "calibrate.log" => ok(log_block(
            &arg::<AdapterId>(input, "adapter")?,
            opt::<Gate>(input, "gate")?.as_ref(),
            &arg::<Vec<Logged>>(input, "lines")?,
            &arg::<Vec<Case>>(input, "cases")?,
            arg::<usize>(input, "unread")?,
        )),
        _ => return adapters(op, input),
    })
}

fn adapters(op: &str, input: &Json) -> Reply {
    Ok(match op {
        "systemone.settings" => result(systemone::settings(
            arg::<Door>(input, "door")?,
            opt::<Json>(input, "table")?.as_ref(),
        )),
        "systemone.address" => result(systemone::address(
            arg::<Door>(input, "door")?,
            opt::<String>(input, "account")?.as_deref(),
        )),
        "systemone.request" => ok(systemone::request(
            arg::<Door>(input, "door")?,
            &arg::<Request>(input, "request")?,
        )),
        "systemone.answers" => result(systemone::answers(
            arg::<Door>(input, "door")?,
            arg(input, "status")?,
            text(input, "body")?,
        )),
        "replay.recording" => result(replay::recording(text(input, "toml")?)),
        "replay.render" => ok(replay::render(&arg(input, "recording")?)),
        "replay.answer" => result(replay::answer(
            &arg(input, "recording")?,
            &arg::<Request>(input, "request")?,
        )),
        _ => return Err(Bug("no such op".to_owned())),
    })
}

fn ok<T: Serialize>(value: T) -> Json {
    json!({ "ok": value })
}

fn result<T: Serialize, E: Serialize>(result: Result<T, E>) -> Json {
    match result {
        Ok(value) => json!({ "ok": value }),
        Err(error) => json!({ "err": error }),
    }
}

/// The argument by name, entered through its type.
fn arg<T: DeserializeOwned>(input: &Json, name: &str) -> Result<T, Bug> {
    let value = input
        .get(name)
        .ok_or_else(|| Bug(format!("{name} is missing")))?;
    serde_json::from_value(value.clone()).map_err(|error| Bug(format!("{name}: {error}")))
}

/// An optional argument: absent or null is none.
fn opt<T: DeserializeOwned>(input: &Json, name: &str) -> Result<Option<T>, Bug> {
    match input.get(name) {
        None | Some(Json::Null) => Ok(None),
        Some(_) => arg(input, name).map(Some),
    }
}

fn text<'a>(input: &'a Json, name: &str) -> Result<&'a str, Bug> {
    input
        .get(name)
        .and_then(Json::as_str)
        .ok_or_else(|| Bug(format!("{name} is missing or not a string")))
}

/// `{ file, toml | json }`, as the vectors write a file.
fn document<'a>(input: &'a Json, name: &str) -> Result<Document<'a>, Bug> {
    let doc = input
        .get(name)
        .ok_or_else(|| Bug(format!("{name} is missing")))?;
    let file = arg(doc, "file").map_err(|Bug(why)| Bug(format!("{name}.{why}")))?;
    let text = match (doc.get("toml").and_then(Json::as_str), doc.get("json")) {
        (Some(toml), _) => Text::Toml(toml),
        (None, Some(json)) => Text::Json(json.clone()),
        (None, None) => return Err(Bug(format!("{name} is neither toml nor json"))),
    };
    Ok(Document { file, text })
}

/// The optional `yours` of a customize op, typed against the manifest it words.
fn yours(input: &Json, of: &Manifest) -> Result<Option<Overlay>, Bug> {
    input
        .get("yours")
        .map(|yours| {
            Overlay::from_json(yours, of).map_err(|problems| {
                Bug(format!(
                    "yours does not read against the manifest: {}",
                    problems
                        .iter()
                        .map(|problem| problem.message.as_str())
                        .collect::<Vec<_>>()
                        .join("; ")
                ))
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reply_is_ok_err_or_bug() {
        assert_eq!(
            call("identity", &json!({ "text": "Kill the lights!" })),
            json!({ "ok": "kill the lights" })
        );
        assert_eq!(
            call("reference", &json!({ "text": "git@github.com:radhi/home" }))["err"]["fix"],
            json!({ "type": "rerun" })
        );
        assert_eq!(
            call("identity", &json!({})),
            json!({ "bug": "identity: text is missing or not a string" })
        );
        assert_eq!(
            call("compile", &json!({ "set": 1 }))["bug"]
                .as_str()
                .map(|bug| bug.starts_with("compile: set:")),
            Some(true)
        );
        assert_eq!(
            call("decide", &json!({})),
            json!({ "bug": "decide: no such op" })
        );
        assert_eq!(
            call("version", &json!({})),
            json!({ "ok": env!("CARGO_PKG_VERSION") })
        );
    }
}
