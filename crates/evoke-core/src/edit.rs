//! What `teach`, `vocab`, `config`, `add` and `remove` change in the files `evoke` owns. In: an `Utterance`, the `Lesson` it teaches
//! and the `Plan` that types it; a vocabulary's word; a reflex's setting with the spec it is held to. Out: an `Edit`
//! — which owned file, the key path, the value — or the diagnostic that refuses it: a span the utterance does not
//! contain, a word the vocabulary lacks, a value the argument cannot take, a reflex the plan does not run, a secret
//! held plain. A lesson has three doors: `stated`, from a confirmed decision; `typed`, from `evoke teach … <call>`;
//! and `from_json`, the wire's `{ reflex, record }`; each types its values by the plan, names followed through `was`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::adapter::QuestionId;
use crate::call::{Value, Written};
use crate::decide::{Chosen, candidate, words};
use crate::diagnostic::{Diagnostic, Fix};
use crate::document::{Json, KeyPath};
use crate::manifest::{Argument, Assertion, ConfigSpec, Kind, Record, Source};
use crate::name::{ArgName, ConfigKey, LocalName, OptionKey, VocabName, Word};
use crate::plan::Plan;
use crate::project::Location;
use crate::project::Setting;
use crate::text::{Clean, Utterance};
use crate::vocabulary::Meaning;

/// One change to an owned file, which the host applies keeping the file's own shape: a key path set, or removed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Edit {
    Set {
        file: Owned,
        path: KeyPath,
        value: Json,
    },
    Remove {
        file: Owned,
        path: KeyPath,
    },
}

impl Edit {
    /// The file the edit lands in.
    #[must_use]
    pub fn file(&self) -> &Owned {
        match self {
            Self::Set { file, .. } | Self::Remove { file, .. } => file,
        }
    }
}

/// A file `evoke` edits in place; the lock and `evoke.d.ts` are rendered whole, never edited.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Owned {
    Project,
    Overlay { name: LocalName },
    Vocab { name: VocabName },
}

/// The overlay line itself: which reflex an utterance belongs to and what it asserts; `not <name>` is `Record::Never`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Lesson {
    pub reflex: LocalName,
    pub record: Record,
}

impl Lesson {
    /// What the utterance stated: the chosen call's arguments whose value is what their judgment read, so what an
    /// ask supplied — after `unstated`, or after a pick out of range — is dropped. A call by name has no judgments
    /// and is stated in full.
    #[must_use]
    pub fn stated(chosen: &Chosen) -> Self {
        let reflex = &chosen.call.reflex;
        let stated = |name: &ArgName, value: &Value| {
            chosen.judged.as_ref().is_none_or(|judged| {
                judged.judgments().iter().any(|judgment| {
                    matches!(&judgment.question, QuestionId::Arg(of, arg) if of == reflex && arg == name)
                        && judgment.top.as_str() == read_as(value)
                })
            })
        };
        let asserts = chosen
            .call
            .args
            .iter()
            .filter(|(name, value)| stated(name, value))
            .map(|(name, value)| (name.clone(), asserted(value)))
            .collect();
        Self {
            reflex: reflex.clone(),
            record: Record::Asserts(asserts),
        }
    }

    /// `evoke teach … <call>`: each written value typed by its argument's source, under the argument's current
    /// name; a flag is written bare. Whether the utterance bears it out is `teach`'s question.
    pub fn typed(written: Written, plan: &Plan) -> Result<Self, Diagnostic> {
        let reflex = written.reflex;
        let active = plan.running(&reflex)?;
        let mut asserts = IndexMap::new();
        for (name, text) in &written.args {
            let (current, argument) = active.argument(&reflex, name.as_str())?;
            let assertion = match (&argument.kind, text) {
                (Kind::Flag, None) => Ok(Assertion::Flag),
                (Kind::Flag, Some(_)) => Err("is a flag; write it bare".to_owned()),
                (Kind::Value { .. }, None) => Err("takes a value".to_owned()),
                (Kind::Value { source, .. }, Some(text)) => {
                    typed_text(source, text).map_err(|why| format!("= {why}"))
                }
            }
            .map_err(|why| refused(Some(&reflex), format!("{name} {why}"), Fix::Rerun))?;
            asserts.insert(current.clone(), assertion);
        }
        Ok(Self {
            reflex,
            record: Record::Asserts(asserts),
        })
    }

    /// A lesson from its wire form, `{ reflex, record }`, each assertion under its argument's current name and typed
    /// by its source. Whether the utterance bears it out is `teach`'s question.
    pub fn from_json(json: &Json, plan: &Plan) -> Result<Self, Diagnostic> {
        let reflex: LocalName = serde_json::from_value(json["reflex"].clone())
            .map_err(|error| refused(None, format!("the lesson's reflex: {error}"), Fix::Rerun))?;
        let active = plan.running(&reflex)?;
        let record = match &json["record"] {
            Json::Bool(false) => Record::Never,
            Json::Object(entries) => {
                let mut asserts = IndexMap::new();
                for (name, value) in entries {
                    let name = ArgName::new(name)
                        .map_err(|why| refused(Some(&reflex), why, Fix::Rerun))?;
                    let (current, argument) = active.argument(&reflex, name.as_str())?;
                    let assertion = typed(&argument.kind, value).map_err(|why| {
                        refused(Some(&reflex), format!("{name} {why}"), Fix::Rerun)
                    })?;
                    asserts.insert(current.clone(), assertion);
                }
                Record::Asserts(asserts)
            }
            _ => {
                return Err(refused(
                    Some(&reflex),
                    "the lesson's record must be a table of assertions or false".to_owned(),
                    Fix::Rerun,
                ));
            }
        };
        Ok(Self { reflex, record })
    }
}

/// The overlay line for the utterance: `[examples] "<utterance>" = <record>`, once every assertion holds against the
/// plan and the utterance.
pub fn teach(utterance: &Utterance, lesson: Lesson, plan: &Plan) -> Result<Edit, Diagnostic> {
    let active = plan.running(&lesson.reflex)?;
    if let Record::Asserts(asserts) = &lesson.record {
        for (name, assertion) in asserts {
            let (_, argument) = active.argument(&lesson.reflex, name.as_str())?;
            holds(&lesson.reflex, name, argument, assertion, utterance, plan)?;
        }
    }
    Ok(Edit::Set {
        file: Owned::Overlay {
            name: lesson.reflex,
        },
        path: KeyPath::new(["examples", utterance.text().as_str()]),
        value: line(&lesson.record),
    })
}

/// `evoke config <name> <key> <value> | --env VAR`: the setting under `[config.<name>]`, held to the key's spec — a
/// secret is set only from a variable.
pub fn set_config(
    name: LocalName,
    key: ConfigKey,
    setting: Setting,
    spec: &ConfigSpec,
) -> Result<Edit, Diagnostic> {
    let value = match setting {
        Setting::Plain { .. } if spec.secret => {
            let message = format!("config \"{key}\" is a secret; it is set from a variable");
            return Err(Diagnostic {
                reflex: Some(name.clone()),
                at: None,
                message,
                fix: Fix::ConfigEnv { reflex: name, key },
            });
        }
        Setting::Plain { value } => Json::String(value),
        Setting::Env { var } => serde_json::json!({ "env": var }),
    };
    Ok(Edit::Set {
        file: Owned::Project,
        path: KeyPath::new(["config", name.as_str(), key.as_str()]),
        value,
    })
}

/// What `evoke vocab <name> add | remove` does to the file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VocabChange {
    Add { word: Word, meaning: Meaning },
    Remove { word: Word },
}

/// The vocabulary line: `word = "meaning"`, `word = { what, value }` when the body receives a value, or the word
/// removed. Adding an existing word replaces its meaning.
#[must_use]
pub fn vocab_edit(name: VocabName, change: VocabChange) -> Edit {
    let file = Owned::Vocab { name };
    match change {
        VocabChange::Add { word, meaning } => Edit::Set {
            file,
            path: KeyPath::new([word.as_str()]),
            value: match meaning.value {
                None => Json::String(meaning.what.to_string()),
                Some(value) => serde_json::json!({ "what": meaning.what, "value": value }),
            },
        },
        VocabChange::Remove { word } => Edit::Remove {
            file,
            path: KeyPath::new([word.as_str()]),
        },
    }
}

/// The overlay line's value: `false`, or a table of strings and booleans under argument names.
fn line(record: &Record) -> Json {
    serde_json::to_value(record).expect("a record is names, strings and booleans, which JSON holds")
}

/// The key the classifier answers with to read a value: an option key, a word, a candidate's `<start>-<end>`, a
/// flag's `yes`.
fn read_as(value: &Value) -> String {
    match value {
        Value::Option { key } => key.as_str().to_owned(),
        Value::Word { word, .. } => word.as_str().to_owned(),
        Value::Pick { span, .. } => candidate(span).to_string(),
        Value::Flag => "yes".to_owned(),
    }
}

/// A value the classifier read, as the overlay would assert it.
fn asserted(value: &Value) -> Assertion {
    match value {
        Value::Option { key } => Assertion::Option(key.clone()),
        Value::Word { word, .. } => Assertion::Word(word.clone()),
        Value::Pick { span, .. } => Assertion::Span(span.text().clone()),
        Value::Flag => Assertion::Flag,
    }
}

/// A raw assertion typed by the argument's source; the error is a fragment to follow the argument's name.
fn typed(kind: &Kind, value: &Json) -> Result<Assertion, String> {
    match (kind, value) {
        (_, Json::Bool(false)) => Ok(Assertion::Unstated),
        (Kind::Flag, Json::Bool(true)) => Ok(Assertion::Flag),
        (Kind::Flag, _) => Err("is a flag, which takes true or false".to_owned()),
        (Kind::Value { source, .. }, Json::String(text)) => {
            typed_text(source, text).map_err(|why| format!("= {why}"))
        }
        (Kind::Value { .. }, _) => Err("takes a string or false".to_owned()),
    }
}

/// A string typed by a value's source: an option key, a word, or a span's text.
fn typed_text(source: &Source, text: &str) -> Result<Assertion, String> {
    match source {
        Source::Options(_) => OptionKey::new(text).map(Assertion::Option),
        Source::Vocab(_) => Word::new(text).map(Assertion::Word),
        Source::Pick(_) => Clean::line(text)
            .map(Assertion::Span)
            .map_err(|why| format!("\"{text}\" {why}")),
    }
}

/// Whether the assertion holds: the key among the options, the word in the vocabulary as compiled, the span in the
/// utterance, a flag on a flag.
fn holds(
    reflex: &LocalName,
    name: &ArgName,
    argument: &Argument,
    assertion: &Assertion,
    utterance: &Utterance,
    plan: &Plan,
) -> Result<(), Diagnostic> {
    let source = match &argument.kind {
        Kind::Flag => None,
        Kind::Value { source, .. } => Some(source),
    };
    let (message, fix) = match (source, assertion) {
        (_, Assertion::Unstated) | (None, Assertion::Flag) => return Ok(()),
        (Some(Source::Options(options)), Assertion::Option(key)) => {
            if options.contains_key(key) {
                return Ok(());
            }
            let keys: Vec<&str> = options.keys().map(OptionKey::as_str).collect();
            (
                format!("\"{key}\" is not an option of {name}: {}", keys.join(", ")),
                Fix::Show {
                    reflex: Some(reflex.clone()),
                },
            )
        }
        (Some(Source::Vocab(vocabulary)), Assertion::Word(word)) => {
            if words(plan, reflex, name).contains_key(word) {
                return Ok(());
            }
            (
                format!("\"{word}\" is not in vocabulary \"{vocabulary}\""),
                Fix::VocabAdd {
                    vocab: vocabulary.clone(),
                },
            )
        }
        (Some(Source::Pick(_)), Assertion::Span(text)) => {
            if utterance.text().as_str().contains(text.as_str()) {
                return Ok(());
            }
            (
                format!("\"{text}\" is not in \"{}\"", utterance.text()),
                Fix::Rerun,
            )
        }
        (source, _) => {
            let takes = match source {
                None => "true or false",
                Some(Source::Options(_)) => "an option key",
                Some(Source::Vocab(_)) => "a word",
                Some(Source::Pick(_)) => "a span of the utterance",
            };
            (format!("{name} takes {takes}"), Fix::Rerun)
        }
    };
    Err(refused(Some(reflex), message, fix))
}

fn refused(reflex: Option<&LocalName>, message: String, fix: Fix) -> Diagnostic {
    Diagnostic {
        reflex: reflex.cloned(),
        at: None,
        message,
        fix,
    }
}

/// `evoke add`: the `[reflexes]` line naming where a reflex comes from, as `evoke.toml` writes it.
#[must_use]
pub fn add_entry(name: &LocalName, location: &Location) -> Edit {
    Edit::Set {
        file: Owned::Project,
        path: KeyPath::new(["reflexes", name.as_str()]),
        value: Json::String(location.to_string()),
    }
}

/// `evoke remove`: that line gone. Your overlay, vocabulary and settings stay: they are yours.
#[must_use]
pub fn remove_entry(name: &LocalName) -> Edit {
    Edit::Remove {
        file: Owned::Project,
        path: KeyPath::new(["reflexes", name.as_str()]),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::adapter::{Key, Prob};
    use crate::call::Call;
    use crate::decide::{Contender, Judged, Judgment, Reading};
    use crate::manifest::Effect;
    use crate::propose::PickValue;
    use crate::text::{Input, Span};

    fn name(text: &str) -> ArgName {
        ArgName::new(text).unwrap()
    }

    fn judgment(question: &str, top: &str, p: f64) -> Judgment {
        Judgment {
            question: QuestionId::parse(question).unwrap(),
            top: Key::new(top).unwrap(),
            p: Prob::new(p).unwrap(),
        }
    }

    /// `lights room="den" state="off" brightness=50`, the brightness a span `0-2` of what a person typed; judged
    /// as given, or by name when `None`.
    fn chosen(judgments: Option<Vec<Judgment>>) -> Chosen {
        let reflex = LocalName::new("lights").unwrap();
        let typed = Input::new("50").unwrap();
        let judged = judgments.map(|judgments| {
            Judged::of(&Reading {
                ranking: vec![Contender {
                    reflex: reflex.clone(),
                    route: Prob::new(0.9).unwrap(),
                    fits: None,
                }],
                judgments,
                winner: None,
            })
            .unwrap()
        });
        Chosen {
            call: Call {
                reflex,
                args: [
                    (
                        name("room"),
                        Value::Word {
                            word: Word::new("den").unwrap(),
                            value: None,
                        },
                    ),
                    (
                        name("state"),
                        Value::Option {
                            key: OptionKey::new("off").unwrap(),
                        },
                    ),
                    (
                        name("brightness"),
                        Value::Pick {
                            span: Span::of(&typed, 0, 2).unwrap(),
                            value: PickValue::Number { value: 50.0 },
                        },
                    ),
                ]
                .into_iter()
                .collect(),
            },
            effect: Effect::Write,
            judged,
        }
    }

    fn stated(chosen: &Chosen) -> Vec<String> {
        let Record::Asserts(asserts) = Lesson::stated(chosen).record else {
            panic!("a stated lesson asserts");
        };
        asserts.keys().map(ToString::to_string).collect()
    }

    #[test]
    fn a_stated_lesson_drops_what_an_ask_supplied() {
        // The room read unstated; the brightness read `18-21`, out of range: both were asked for.
        let asked = chosen(Some(vec![
            judgment("route", "lights", 0.9),
            judgment("lights.room", "unstated", 0.7),
            judgment("lights.state", "off", 0.58),
            judgment("lights.brightness", "18-21", 0.8),
        ]));
        assert_eq!(stated(&asked), ["state"]);
    }

    #[test]
    fn a_stated_lesson_keeps_what_was_read() {
        let read = chosen(Some(vec![
            judgment("route", "lights", 0.9),
            judgment("lights.room", "den", 0.7),
            judgment("lights.state", "off", 0.58),
            judgment("lights.brightness", "0-2", 0.8),
        ]));
        assert_eq!(stated(&read), ["room", "state", "brightness"]);
    }

    #[test]
    fn a_call_by_name_is_stated_in_full() {
        assert_eq!(stated(&chosen(None)), ["room", "state", "brightness"]);
    }

    #[test]
    fn a_lesson_from_json_follows_was() {
        let mut plan: Json =
            serde_json::from_str(include_str!("../../../spec/fixtures/plan.json")).unwrap();
        plan["active"]["lights"]["args"]["state"]["was"] = json!(["power"]);
        let plan: Plan = serde_json::from_value(plan).unwrap();
        let lesson = json!({ "reflex": "lights", "record": { "power": "off" } });
        let lesson = Lesson::from_json(&lesson, &plan).unwrap();
        assert_eq!(
            lesson.record,
            Record::Asserts(
                [(
                    name("state"),
                    Assertion::Option(OptionKey::new("off").unwrap())
                )]
                .into_iter()
                .collect()
            )
        );
    }

    fn plan() -> Plan {
        serde_json::from_str(include_str!("../../../spec/fixtures/plan.json")).unwrap()
    }

    fn typed_lesson(text: &str) -> Result<Lesson, Diagnostic> {
        Lesson::typed(crate::call::call(text).unwrap(), &plan())
    }

    #[test]
    fn a_typed_lesson_asserts_what_was_written() {
        let lesson = typed_lesson("lights state=off brightness=\"30 percent\"").unwrap();
        assert_eq!(
            serde_json::to_value(&lesson).unwrap(),
            json!({ "reflex": "lights", "record": { "state": "off", "brightness": "30 percent" } })
        );
        let refused = |text: &str| typed_lesson(text).unwrap_err();
        assert_eq!(refused("lights state").message, "state takes a value");
        assert_eq!(
            refused("lights colour=blue").message,
            "lights has no argument colour"
        );
        assert_eq!(
            refused("lights colour=blue").fix,
            Fix::Show {
                reflex: Some(LocalName::new("lights").unwrap())
            }
        );
        assert_eq!(refused("volume level=3").message, "volume is not installed");
        assert_eq!(refused("volume level=3").fix, Fix::Show { reflex: None });
    }

    #[test]
    fn a_setting_is_held_to_its_spec() {
        let lights = LocalName::new("lights").unwrap();
        let token = ConfigKey::new("token").unwrap();
        let secret = ConfigSpec {
            about: Clean::new("Hue API key").unwrap(),
            secret: true,
        };
        let plain = Setting::Plain {
            value: "hue-example".to_owned(),
        };
        let refused = set_config(lights.clone(), token.clone(), plain, &secret).unwrap_err();
        assert_eq!(
            refused.message,
            "config \"token\" is a secret; it is set from a variable"
        );
        assert_eq!(
            refused.fix,
            Fix::ConfigEnv {
                reflex: lights.clone(),
                key: token.clone()
            }
        );
        let env = Setting::Env {
            var: crate::name::VarName::new("HUE_TOKEN").unwrap(),
        };
        let edit = set_config(lights, token, env, &secret).unwrap();
        assert_eq!(
            serde_json::to_value(&edit).unwrap(),
            json!({ "type": "set", "file": { "type": "project" }, "path": ["config", "lights", "token"], "value": { "env": "HUE_TOKEN" } })
        );
    }

    #[test]
    fn a_vocabulary_line_is_the_meaning_or_its_table() {
        let rooms = || VocabName::new("rooms").unwrap();
        let den = Word::new("den").unwrap();
        let plain = vocab_edit(
            rooms(),
            VocabChange::Add {
                word: den.clone(),
                meaning: Meaning {
                    what: Clean::new("The TV room.").unwrap(),
                    value: None,
                },
            },
        );
        assert_eq!(
            serde_json::to_value(&plain).unwrap(),
            json!({ "type": "set", "file": { "type": "vocab", "name": "rooms" }, "path": ["den"], "value": "The TV room." })
        );
        let removed = vocab_edit(rooms(), VocabChange::Remove { word: den });
        assert_eq!(
            serde_json::to_value(&removed).unwrap(),
            json!({ "type": "remove", "file": { "type": "vocab", "name": "rooms" }, "path": ["den"] })
        );
    }

    #[test]
    fn a_raw_assertion_is_typed_by_its_source() {
        assert_eq!(
            typed(&Kind::Flag, &Json::Bool(true)).unwrap(),
            Assertion::Flag
        );
        assert_eq!(
            typed(&Kind::Flag, &Json::from("yes")).unwrap_err(),
            "is a flag, which takes true or false"
        );
        let vocab = Kind::Value {
            source: Source::Vocab(VocabName::new("rooms").unwrap()),
            optional: false,
        };
        assert_eq!(
            typed(&vocab, &Json::Bool(false)).unwrap(),
            Assertion::Unstated
        );
        assert_eq!(
            typed(&vocab, &Json::from(" den")).unwrap_err(),
            "= \" den\" has leading or trailing whitespace"
        );
        assert_eq!(
            typed(&vocab, &Json::from(1)).unwrap_err(),
            "takes a string or false"
        );
    }
}
