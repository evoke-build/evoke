//! A plan as a file: the sentence as typed, the engine's raw answers to every question the plan asked, and the
//! pins they were gathered under — the format revision, evoke's version, the adapter's id and gate, the set's
//! digest, each active reflex and vocabulary by the hash of its wire form, a remote reflex by the lock's `h1`.
//! `pin` seals one; `pinned` reads one back, `plan = 1` required; `stale` names the first pin that moved on this
//! machine, in a fixed order, each line ending in its fix; `replan` makes the plan again from the file's answers
//! alone, under the file's gate, and refuses one that does not read the same. Pure. In: the installed set, the
//! compiled plan, the project, the lock, the adapter's declaration, a file's JSON and its path as shown. Out:
//! `Pinned`, `Replanned`, or a `Diagnostic`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Answers, Asked, Need, Planning, Weave, planning, reading};
use crate::adapter::{Declared, Fault, Gate, Key, Prob, Question, Raw, Request};
use crate::call::quoted;
use crate::decide::{self, Decision, Scope};
use crate::diagnostic::{Diagnostic, Fix};
use crate::digest::Digest;
use crate::document::Json;
use crate::name::{LocalName, Tag, VocabName};
use crate::plan::{Installed, Plan};
use crate::project::{self, Location, Lock, LockedAdapter, Project, Reference, Version};
use crate::propose::Proposed;
use crate::text::identity;

/// The file's second line: what the lock's header comment says, in a format that has no comments.
pub const NOTE: &str = "Written by evoke try --save: the sentence and the classifier's answers, in clear. evoke run runs it exactly, or says which pin moved.";

/// A plan file: what `try --save` writes and `run <file>` reads. The sentence and the steps first, the pins, then
/// the answers; no time, no name, no trace.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pinned {
    /// Fixed text, ignored on read.
    #[serde(default)]
    pub note: String,
    /// The format revision, `1`, as `reflex = 1` and `lock = 1`.
    #[serde(rename = "plan")]
    pub revision: Revision,
    /// The sentence as typed: what the re-plan plans, since reading the request as read is not always the same.
    pub input: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<Tag>,
    pub weave: Weave,
    pub evoke: Version,
    pub adapter: LockedAdapter,
    /// The adapter's floors the plan was decided under; absent when it declared none. The gate shapes the plan.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<Gate>,
    /// The plan digest: SHA-256 of the installed set's wire form.
    pub set: Digest,
    /// Every active reflex, by the hash of its item and its pin.
    pub reflexes: IndexMap<LocalName, PinnedReflex>,
    /// Every vocabulary, by the hash of its words.
    pub vocab: IndexMap<VocabName, Digest>,
    /// Every engine answer the plan took, one per text and question set.
    pub answers: Vec<Answer>,
}

/// The one revision a plan file may carry: `1`, refused at any other value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u64", into = "u64")]
pub struct Revision;

impl TryFrom<u64> for Revision {
    type Error = String;

    fn try_from(revision: u64) -> Result<Self, String> {
        if revision == 1 {
            Ok(Self)
        } else {
            Err(format!("plan must be 1, not {revision}"))
        }
    }
}

impl From<Revision> for u64 {
    fn from(_: Revision) -> Self {
        1
    }
}

/// One reflex of the plan's set: the hash of its item — wording, consent, settings — and its pin: a remote one's
/// ref, tag and `h1` from the lock, a local one's path as written, which is never compared, a reflex handed as
/// code nothing but its item. On the wire, told by its keys; a pin with some of a remote's keys and not all is
/// refused.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(try_from = "RawPin")]
pub enum PinnedReflex {
    Remote {
        item: Digest,
        reference: Reference,
        tag: Version,
        h1: Digest,
    },
    Local {
        item: Digest,
        path: String,
    },
    Inline {
        item: Digest,
    },
}

/// A pin's keys as written, each optional, so the shape is judged in one place.
#[derive(Deserialize)]
struct RawPin {
    item: Digest,
    #[serde(default, rename = "ref")]
    reference: Option<String>,
    #[serde(default)]
    tag: Option<Version>,
    #[serde(default)]
    h1: Option<Digest>,
    #[serde(default)]
    path: Option<String>,
}

impl TryFrom<RawPin> for PinnedReflex {
    type Error = String;

    fn try_from(raw: RawPin) -> Result<Self, String> {
        match (raw.reference, raw.tag, raw.h1, raw.path) {
            (Some(reference), Some(tag), Some(h1), None) => match project::reference(&reference) {
                Ok((reference, None)) => Ok(Self::Remote {
                    item: raw.item,
                    reference,
                    tag,
                    h1,
                }),
                Ok((_, Some(_))) => Err(format!(
                    "\"{reference}\" carries a pin; the tag is the version"
                )),
                Err(problem) => Err(problem.message),
            },
            (None, None, None, Some(path)) => Ok(Self::Local {
                item: raw.item,
                path,
            }),
            (None, None, None, None) => Ok(Self::Inline { item: raw.item }),
            _ => Err("a pin is a ref with its tag and h1, a path, or the item alone".to_owned()),
        }
    }
}

/// A pin on the wire: the ref as the lock writes it, `acme/books/ledger`.
#[derive(Serialize)]
#[serde(untagged)]
enum WrittenPin<'a> {
    Remote {
        item: Digest,
        #[serde(rename = "ref")]
        reference: String,
        tag: Version,
        h1: Digest,
    },
    Local {
        item: Digest,
        path: &'a str,
    },
    Inline {
        item: Digest,
    },
}

impl Serialize for PinnedReflex {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Remote {
                item,
                reference,
                tag,
                h1,
            } => WrittenPin::Remote {
                item: *item,
                reference: reference.to_string(),
                tag: *tag,
                h1: *h1,
            },
            Self::Local { item, path } => WrittenPin::Local { item: *item, path },
            Self::Inline { item } => WrittenPin::Inline { item: *item },
        }
        .serialize(serializer)
    }
}

impl PinnedReflex {
    #[must_use]
    pub fn item(&self) -> Digest {
        match self {
            Self::Remote { item, .. } | Self::Local { item, .. } | Self::Inline { item } => *item,
        }
    }

    /// The command that installs the reflex as the plan had it: a remote one's ref at its tag under its name; a
    /// local one, or one handed as code, is made again here — the save line.
    fn fix(&self, name: &LocalName, save: Fix) -> Fix {
        match self {
            Self::Remote { reference, tag, .. } => Fix::AddRef {
                reference: format!("{reference}@{tag}"),
                name: Some(name.clone()),
            },
            Self::Local { .. } | Self::Inline { .. } => save,
        }
    }
}

/// One engine answer the plan took: the text it was asked about and the `Raw` as the engine gave it. The weave's
/// own questions stand under the whole request's text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    pub text: String,
    pub raw: Raw,
}

/// The plan made again from a file: the weave, and per text what a log line is rendered from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Replanned {
    pub weave: Weave,
    pub decided: Vec<Decided>,
}

/// One text decided from the file's answers: what was asked, the input's candidates, the raw answers, the
/// decision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Decided {
    pub asked: Asked,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proposed: Vec<Proposed>,
    pub raw: Raw,
    pub decision: Decision,
}

/// The plan sealed: the weave with the sentence it was planned from, the tags, the answers gathered, and the pins
/// read off the installed set, the compiled plan, the project, the lock and the adapter's declaration.
#[must_use]
#[expect(clippy::too_many_arguments)]
pub fn pin(
    installed: &Installed,
    plan: &Plan,
    project: &Project,
    lock: Option<&Lock>,
    declared: &Declared,
    input: &str,
    tags: &[Tag],
    weave: Weave,
    answers: Vec<Answer>,
) -> Pinned {
    let reflexes = plan
        .active()
        .keys()
        .filter_map(|name| {
            let item = hashed(installed.reflexes.get(name)?);
            let locked = lock.and_then(|lock| lock.reflexes.get(name));
            let pinned = match (project.reflexes.get(name), locked) {
                (Some(Location::Local { path }), _) => PinnedReflex::Local {
                    item,
                    path: path.clone(),
                },
                (Some(Location::Remote { .. }), Some(locked)) => PinnedReflex::Remote {
                    item,
                    reference: locked.reference.clone(),
                    tag: locked.tag,
                    h1: locked.h1,
                },
                // A reflex handed as code has no location; a remote one never decides unlocked.
                (Some(Location::Remote { .. }), None) | (None, _) => PinnedReflex::Inline { item },
            };
            Some((name.clone(), pinned))
        })
        .collect();
    let vocab = installed
        .vocab
        .iter()
        .map(|(name, words)| (name.clone(), hashed(words)))
        .collect();
    Pinned {
        note: NOTE.to_owned(),
        revision: Revision,
        input: input.to_owned(),
        tags: tags.to_vec(),
        weave,
        evoke: installed.evoke,
        adapter: LockedAdapter {
            name: project.adapter.clone(),
            id: installed.adapter.clone(),
        },
        gate: declared.gate,
        set: plan.digest(),
        reflexes,
        vocab,
        answers,
    }
}

/// SHA-256 over a value's compact JSON: the digest's own recipe, cut per reflex and per vocabulary.
fn hashed<T: Serialize>(value: &T) -> Digest {
    Digest::of(&serde_json::to_vec(value).expect("a wire type serializes"))
}

/// A plan file read: `plan` must be 1 before the shape is trusted, and the refusal names the save line when the
/// file holds a sentence, `evoke --help` otherwise.
pub fn pinned(path: &str, json: Json) -> Result<Pinned, Diagnostic> {
    let fix = match json.get("input").and_then(Json::as_str) {
        Some(input) => Fix::Save {
            file: path.to_owned(),
            input: input.to_owned(),
        },
        None => Fix::Help,
    };
    let refused = |message: String| Diagnostic {
        reflex: None,
        at: None,
        message,
        fix: fix.clone(),
    };
    match json.get("plan") {
        None => return Err(refused("plan = 1 is required".to_owned())),
        Some(Json::Number(revision)) if revision.as_u64() == Some(1) => {}
        Some(Json::Number(revision)) => {
            return Err(refused(format!("plan must be 1, not {revision}")));
        }
        Some(_) => return Err(refused("plan must be 1".to_owned())),
    }
    serde_json::from_value(json)
        .map_err(|error| refused(format!("the plan does not read: {error}")))
}

/// The first pin that moved on this machine, as one line ending in its fix, in a fixed order: evoke's version,
/// the engine's id, the gate, then per reflex of the plan's set not installed here, inactive here, its remote
/// body moved, its item moved; per vocabulary moved; a reflex active here the plan never saw; the digest alone.
/// Nothing when every pin holds.
pub fn stale(
    path: &str,
    pinned: &Pinned,
    installed: &Installed,
    plan: &Plan,
    lock: Option<&Lock>,
    declared: &Declared,
) -> Result<(), Diagnostic> {
    let save = || Fix::Save {
        file: path.to_owned(),
        input: pinned.input.clone(),
    };
    let refused = |message: String, fix: Fix| Diagnostic {
        reflex: None,
        at: None,
        message,
        fix,
    };
    if installed.evoke != pinned.evoke {
        return Err(refused(
            format!(
                "the plan was written by evoke {}; this is {}",
                pinned.evoke, installed.evoke
            ),
            save(),
        ));
    }
    if declared.id != pinned.adapter.id {
        return Err(refused(
            format!(
                "the plan was decided by {}; this project's adapter is {}",
                pinned.adapter.id, declared.id
            ),
            save(),
        ));
    }
    if let Some(message) = gate_moved(pinned.gate.as_ref(), declared.gate.as_ref()) {
        return Err(refused(message, save()));
    }
    for (name, pin) in &pinned.reflexes {
        if let Some(problem) = reflex_moved(name, pin, pinned, installed, plan, lock, &save) {
            return Err(problem);
        }
    }
    for (name, digest) in &pinned.vocab {
        if installed.vocab.get(name).map(hashed) != Some(*digest) {
            return Err(refused(
                format!("{name} differs here from the plan's"),
                save(),
            ));
        }
    }
    if let Some(name) = plan
        .active()
        .keys()
        .find(|name| !pinned.reflexes.contains_key(*name))
    {
        return Err(refused(
            format!("{name} is installed here and was not in the plan's set"),
            Fix::Remove {
                reflex: name.clone(),
            },
        ));
    }
    if plan.digest() != pinned.set {
        return Err(refused(
            "this project's files differ from the plan's".to_owned(),
            save(),
        ));
    }
    Ok(())
}

/// Why one reflex of the plan's set is not as the plan had it here, when it is not: not installed, inactive — the
/// plan's own line — its remote body moved, or its item.
fn reflex_moved(
    name: &LocalName,
    pin: &PinnedReflex,
    pinned: &Pinned,
    installed: &Installed,
    plan: &Plan,
    lock: Option<&Lock>,
    save: &dyn Fn() -> Fix,
) -> Option<Diagnostic> {
    let refused = |message: String, fix: Fix| {
        Some(Diagnostic {
            reflex: None,
            at: None,
            message,
            fix,
        })
    };
    let Some(item) = installed.reflexes.get(name) else {
        let steps = &pinned.weave.steps;
        let step = steps.iter().find(|step| step.reflex.as_ref() == Some(name));
        let wrote = steps
            .iter()
            .any(|step| step.from.iter().any(|from| from.playbook == *name));
        let message = match step {
            Some(step) => format!("step {}'s reflex {name} is not installed here", step.n),
            None if wrote => format!("the plan's playbook {name} is not installed here"),
            None => format!("{name} is in the plan's set and not installed here"),
        };
        return refused(message, pin.fix(name, save()));
    };
    if let Some(problems) = plan.inactive().get(name) {
        return Some(problems.first().clone());
    }
    if let PinnedReflex::Remote {
        reference, tag, h1, ..
    } = pin
        && let Some(locked) = lock.and_then(|lock| lock.reflexes.get(name))
        && locked.h1 != *h1
    {
        let message = if locked.tag == *tag {
            format!("{name}'s body at {tag} differs here from the plan's")
        } else {
            format!(
                "{name} is at {} here; the plan was made over {tag}",
                locked.tag
            )
        };
        return refused(
            message,
            Fix::AddRef {
                reference: format!("{reference}@{tag}"),
                name: Some(name.clone()),
            },
        );
    }
    if hashed(item) != pin.item() {
        return refused(
            format!("{name} differs here from the plan's: its wording, consent or settings moved"),
            save(),
        );
    }
    None
}

/// Why the gate the plan was decided under is not this adapter's, when it is not: absent on one side, or the
/// first floor that differs.
fn gate_moved(file: Option<&Gate>, here: Option<&Gate>) -> Option<String> {
    match (file, here) {
        (None, None) => None,
        (None, Some(_)) => Some(
            "the plan was decided under no gate; this project's adapter declares one".to_owned(),
        ),
        (Some(_), None) => Some(
            "the plan was decided under a gate; this project's adapter declares none".to_owned(),
        ),
        (Some(file), Some(here)) => {
            let floors = [
                ("route", Some(file.route()), Some(here.route())),
                ("fits", file.fits(), here.fits()),
                ("read", Some(file.read()), Some(here.read())),
                ("write", Some(file.write()), Some(here.write())),
            ];
            floors
                .iter()
                .find(|(_, file, here)| file != here)
                .map(|(name, file, here)| {
                    format!(
                        "the plan was decided under another gate: {name} {}, here {}",
                        floor(*file),
                        floor(*here)
                    )
                })
        }
    }
}

fn floor(p: Option<Prob>) -> String {
    p.map_or_else(|| "none".to_owned(), |p| p.to_string())
}

/// The plan made again from the file alone: the sentence as typed planned under the file's gate, every judgment
/// and decision answered from the file's entries by the text's identity and the questions asked, inside one pure
/// function that asks no adapter. A text the file does not answer, an entry short of a question, one naming a key
/// the set does not offer, or a plan that does not read the same as the file's is refused with the save line.
pub fn replan(path: &str, pinned: &Pinned, plan: &Plan) -> Result<Replanned, Diagnostic> {
    let save = || Fix::Save {
        file: path.to_owned(),
        input: pinned.input.clone(),
    };
    let refused = |message: String| Diagnostic {
        reflex: None,
        at: None,
        message,
        fix: save(),
    };
    let unread = |text: &str, fault: &Fault| {
        refused(format!(
            "the file's answer for {} does not read: {fault}",
            quoted(text)
        ))
    };
    let request = reading::canonical(&pinned.input);
    let mut answers = Answers::default();
    let mut decided = Vec::new();
    loop {
        let planning = planning::plan(
            plan,
            pinned.gate.as_ref(),
            &pinned.input,
            &pinned.tags,
            &answers,
        )
        .map_err(|fault| unread(&request, &fault))?;
        let need = match planning {
            Planning::Done { weave } => {
                if let Some(message) = differs(&weave, &pinned.weave) {
                    return Err(refused(message));
                }
                return Ok(Replanned { weave, decided });
            }
            Planning::Need { need } => need,
        };
        match need {
            Need::Judge { request } => {
                answers.judged = Some(answered(pinned, &request).map_err(&refused)?);
            }
            Need::Refer { request } => {
                answers.referred = Some(answered(pinned, &request).map_err(&refused)?);
            }
            Need::Decide { asked } => {
                for asked in asked {
                    // A file holds no memory: no result of any session reaches a text decided again from it.
                    let request = decide::request(
                        plan,
                        &asked.text,
                        &asked.tags,
                        asked.only.as_ref(),
                        Scope::Full,
                        &[],
                    )?;
                    let raw = answered(pinned, &request).map_err(&refused)?;
                    let reading = decide::read(plan, &request, raw.clone())
                        .map_err(|fault| unread(&asked.text, &fault))?;
                    let decision = decide::gate(plan, reading, pinned.gate.as_ref());
                    answers.decided.push((asked.clone(), decision.clone()));
                    decided.push(Decided {
                        asked,
                        proposed: request.proposed,
                        raw,
                        decision,
                    });
                }
            }
        }
    }
}

/// The file's answer to a request: among the answers with the text's identity, the one holding the most of the
/// questions asked, cut to them — as a recording answers — or why the file cannot answer.
fn answered(pinned: &Pinned, request: &Request) -> Result<Raw, String> {
    let text = request.state.request.as_str();
    let wanted = identity(text);
    let entries: Vec<&Answer> = pinned
        .answers
        .iter()
        .filter(|entry| identity(&entry.text) == wanted)
        .collect();
    let asked: Vec<String> = request.questions.keys().map(ToString::to_string).collect();
    let present = |entry: &Answer| {
        asked
            .iter()
            .filter(|id| entry.raw.0.contains_key(*id))
            .count()
    };
    let Some(best) = entries.iter().copied().reduce(|best, entry| {
        if present(entry) > present(best) {
            entry
        } else {
            best
        }
    }) else {
        return Err(format!(
            "the plan asks {}, which the file does not answer",
            quoted(text)
        ));
    };
    if let Some(lacking) = asked.iter().find(|id| !best.raw.0.contains_key(*id)) {
        return Err(format!(
            "the file's answer for {} lacks {lacking}",
            quoted(text)
        ));
    }
    for (id, question) in &request.questions {
        let offered: Vec<&str> = match question {
            Question::Choice(choice) => choice.options().keys().map(Key::as_str).collect(),
            Question::YesNo { .. } => vec!["yes", "no"],
        };
        let answer = &best.raw.0[&id.to_string()];
        if let Some(key) = answer.keys().find(|key| !offered.contains(&key.as_str())) {
            return Err(format!(
                "the file's answer for {} names {key}, which the set does not offer",
                quoted(text)
            ));
        }
    }
    Ok(Raw(asked
        .iter()
        .map(|id| (id.clone(), best.raw.0[id].clone()))
        .collect()))
}

/// Why the plan made again is not the file's, when it is not: the first step that differs, else the plan whole.
fn differs(again: &Weave, file: &Weave) -> Option<String> {
    if again == file {
        return None;
    }
    let step = again
        .steps
        .iter()
        .zip(&file.steps)
        .position(|(again, file)| again != file)
        .or_else(|| {
            (again.steps.len() != file.steps.len())
                .then_some(again.steps.len().min(file.steps.len()))
        })
        .map(|i| i + 1);
    Some(match step {
        Some(n) => format!("the plan does not read the same as the file: step {n}"),
        None => "the plan does not read the same as the file".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pin on the wire is told by its keys: a remote one's ref, a local one's path, or the item alone.
    #[test]
    fn a_pinned_reflex_is_told_by_its_keys() {
        let item = format!("h1:{}", "a".repeat(64));
        let remote: PinnedReflex = serde_json::from_value(serde_json::json!({
            "item": item, "ref": "acme/books/ledger", "tag": "1.2.0", "h1": format!("h1:{}", "b".repeat(64))
        }))
        .unwrap();
        assert!(matches!(remote, PinnedReflex::Remote { .. }));
        assert_eq!(
            serde_json::to_value(&remote).unwrap()["ref"],
            serde_json::json!("acme/books/ledger")
        );
        let local: PinnedReflex =
            serde_json::from_value(serde_json::json!({ "item": item, "path": "./ledger" }))
                .unwrap();
        assert!(matches!(local, PinnedReflex::Local { .. }));
        let inline: PinnedReflex =
            serde_json::from_value(serde_json::json!({ "item": item })).unwrap();
        assert!(matches!(inline, PinnedReflex::Inline { .. }));
        assert!(
            serde_json::from_value::<PinnedReflex>(serde_json::json!({
                "item": item, "ref": "acme/books/ledger@1.2.0", "tag": "1.2.0", "h1": format!("h1:{}", "b".repeat(64))
            }))
            .is_err()
        );
        // Some of a remote's keys and not all is no pin: a file that lost its h1 fails closed.
        assert!(
            serde_json::from_value::<PinnedReflex>(serde_json::json!({
                "item": item, "ref": "acme/books/ledger", "tag": "1.2.0"
            }))
            .is_err()
        );
    }

    /// The revision is checked before the shape: a file without `plan` ends in the help, one with a sentence in
    /// the save line.
    #[test]
    fn the_revision_is_read_before_the_shape() {
        let none = pinned("x.plan.json", serde_json::json!({ "lock": 1 })).unwrap_err();
        assert_eq!(none.message, "plan = 1 is required");
        assert_eq!(none.fix, Fix::Help);
        let later = pinned(
            "x.plan.json",
            serde_json::json!({ "plan": 2, "input": "kill the lights" }),
        )
        .unwrap_err();
        assert_eq!(later.message, "plan must be 1, not 2");
        assert_eq!(
            later.fix,
            Fix::Save {
                file: "x.plan.json".to_owned(),
                input: "kill the lights".to_owned()
            }
        );
        let text = pinned("x.plan.json", serde_json::json!({ "plan": "1" })).unwrap_err();
        assert_eq!(text.message, "plan must be 1");
        let shape = pinned(
            "x.plan.json",
            serde_json::json!({ "plan": 1, "input": "kill the lights" }),
        )
        .unwrap_err();
        assert!(shape.message.starts_with("the plan does not read: "));
    }

    #[test]
    fn a_gate_that_moved_names_its_first_floor() {
        let gate = |read: f64| {
            Gate::new(
                Prob::new(0.5).unwrap(),
                Some(Prob::new(0.3).unwrap()),
                Prob::new(read).unwrap(),
                Prob::new(0.8).unwrap(),
            )
            .unwrap()
        };
        assert_eq!(gate_moved(None, None), None);
        assert_eq!(gate_moved(Some(&gate(0.6)), Some(&gate(0.6))), None);
        assert_eq!(
            gate_moved(Some(&gate(0.6)), Some(&gate(0.5))).as_deref(),
            Some("the plan was decided under another gate: read 0.6, here 0.5")
        );
        assert_eq!(
            gate_moved(None, Some(&gate(0.6))).as_deref(),
            Some("the plan was decided under no gate; this project's adapter declares one")
        );
        assert_eq!(
            gate_moved(Some(&gate(0.6)), None).as_deref(),
            Some("the plan was decided under a gate; this project's adapter declares none")
        );
    }
}
