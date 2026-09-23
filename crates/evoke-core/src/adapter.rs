//! The adapter contract's types: what the core asks, what an adapter declares and answers, and how an adapter fails.
//! In: nothing. Out: `Question`, `Request`, `Raw`, `Gate`, `Limits`, `Declared`, `Fault`.

use std::borrow::Borrow;
use std::fmt;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::Fix;
use crate::name::{AdapterId, ArgName, LocalName, OptionKey, VarName, WeaveName, Word};
use crate::propose::Proposed;
use crate::text::{Clean, Identity, Input};

/// A choice's key exactly as offered: an option key, a word, a candidate `<start>-<end>`, a flag's `yes` or `no`, a
/// local name, `none` or `unstated`. The plan's slot gives it meaning at read.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Key(String);

impl Key {
    pub fn new(text: &str) -> Result<Self, String> {
        if text.is_empty() {
            Err("a key is never empty".to_owned())
        } else {
            Ok(Self(text.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Key {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::new(&text)
    }
}

impl From<&LocalName> for Key {
    fn from(name: &LocalName) -> Self {
        Self(name.to_string())
    }
}

impl From<&OptionKey> for Key {
    fn from(key: &OptionKey) -> Self {
        Self(key.to_string())
    }
}

impl From<&Word> for Key {
    fn from(word: &Word) -> Self {
        Self(word.to_string())
    }
}

impl Borrow<str> for Key {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which question: `route`, `fits.<reflex>`, `<reflex>.<argument>`, or `weave.<name>` — a question `evoke` asks
/// on its own account beside the plan's, which travels through an adapter like any other; on the wire, that
/// string. No reflex is named `fits` or `weave`, so the head settles the kind.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum QuestionId {
    Route,
    Fits(LocalName),
    Arg(LocalName, ArgName),
    Weave(WeaveName),
}

impl QuestionId {
    pub fn parse(text: &str) -> Result<Self, String> {
        if text == "route" {
            return Ok(Self::Route);
        }
        let invalid = || {
            format!(
                "\"{text}\" is not a question id: route, fits.<reflex>, <reflex>.<argument> or weave.<name>"
            )
        };
        let (head, tail) = text.split_once('.').ok_or_else(invalid)?;
        if head == "fits" {
            return LocalName::new(tail).map(Self::Fits).map_err(|_| invalid());
        }
        if head == "weave" {
            return WeaveName::new(tail).map(Self::Weave).map_err(|_| invalid());
        }
        match (LocalName::new(head), ArgName::new(tail)) {
            (Ok(reflex), Ok(arg)) => Ok(Self::Arg(reflex, arg)),
            _ => Err(invalid()),
        }
    }

    /// The reflex the question is about, if any.
    #[must_use]
    pub fn reflex(&self) -> Option<&LocalName> {
        match self {
            Self::Route | Self::Weave(_) => None,
            Self::Fits(reflex) | Self::Arg(reflex, _) => Some(reflex),
        }
    }
}

impl TryFrom<String> for QuestionId {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text)
    }
}

impl From<QuestionId> for String {
    fn from(id: QuestionId) -> Self {
        id.to_string()
    }
}

impl fmt::Display for QuestionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Route => f.write_str("route"),
            Self::Fits(reflex) => write!(f, "fits.{reflex}"),
            Self::Arg(reflex, arg) => write!(f, "{reflex}.{arg}"),
            Self::Weave(name) => write!(f, "weave.{name}"),
        }
    }
}

/// One question for the adapter: a choice over keys, or a yes/no with both sides described.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Question {
    Choice(Choice),
    YesNo { ask: Clean, yes: Text, no: Text },
}

/// A choice: the question, its options in order, and which key means "none of these".
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawChoice")]
pub struct Choice {
    ask: Clean,
    options: IndexMap<Key, Text>,
    #[serde(skip_serializing_if = "Option::is_none")]
    otherwise: Option<Key>,
}

#[derive(Deserialize)]
struct RawChoice {
    ask: Clean,
    options: IndexMap<Key, Text>,
    #[serde(default)]
    otherwise: Option<Key>,
}

impl Choice {
    /// A choice whose sentinel, when named, is one of its options.
    pub fn new(
        ask: Clean,
        options: IndexMap<Key, Text>,
        otherwise: Option<Key>,
    ) -> Result<Self, String> {
        match &otherwise {
            Some(key) if !options.contains_key(key) => {
                Err(format!("otherwise names \"{key}\", which is not an option"))
            }
            _ => Ok(Self {
                ask,
                options,
                otherwise,
            }),
        }
    }

    /// A choice closed by its sentinel: the option meaning "none of these", appended last.
    #[must_use]
    pub fn closed(ask: Clean, mut options: IndexMap<Key, Text>, otherwise: (Key, Text)) -> Self {
        let (key, text) = otherwise;
        options.insert(key.clone(), text);
        Self {
            ask,
            options,
            otherwise: Some(key),
        }
    }

    /// The same choice over the options kept; the sentinel always stays.
    #[must_use]
    pub fn narrowed(&self, keep: impl Fn(&Key) -> bool) -> Self {
        let options = self
            .options
            .iter()
            .filter(|(key, _)| keep(key) || Some(*key) == self.otherwise.as_ref())
            .map(|(key, text)| (key.clone(), text.clone()))
            .collect();
        Self {
            ask: self.ask.clone(),
            options,
            otherwise: self.otherwise.clone(),
        }
    }

    #[must_use]
    pub fn ask(&self) -> &Clean {
        &self.ask
    }

    #[must_use]
    pub fn options(&self) -> &IndexMap<Key, Text> {
        &self.options
    }

    #[must_use]
    pub fn otherwise(&self) -> Option<&Key> {
        self.otherwise.as_ref()
    }
}

impl TryFrom<RawChoice> for Choice {
    type Error = String;

    fn try_from(raw: RawChoice) -> Result<Self, String> {
        Self::new(raw.ask, raw.options, raw.otherwise)
    }
}

/// What an option means: a line, or a description with what it is not for and examples that assert it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Text {
    Plain(Clean),
    Rich {
        what: Clean,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        not_for: Vec<Clean>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        examples: Vec<Clean>,
    },
}

impl Text {
    /// What the option is, without its context.
    #[must_use]
    pub fn what(&self) -> &Clean {
        match self {
            Self::Plain(what) | Self::Rich { what, .. } => what,
        }
    }
}

/// The only state an adapter ever sees.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub request: Input,
}

/// One call of `answer`: the state, the questions, and the candidate spans the pick questions were built from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Request {
    pub state: State,
    pub questions: IndexMap<QuestionId, Question>,
    pub proposed: Vec<Proposed>,
}

/// A finite number in `[0, 1]`.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64")]
pub struct Prob(f64);

impl Prob {
    pub const ZERO: Self = Self(0.0);

    #[must_use]
    pub const fn new(p: f64) -> Option<Self> {
        if p.is_finite() && 0.0 <= p && p <= 1.0 {
            Some(Self(p))
        } else {
            None
        }
    }

    /// `1 - p`: the rest of the distribution.
    #[must_use]
    pub fn complement(self) -> Self {
        Self(1.0 - self.0)
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Prob {
    type Error = String;

    fn try_from(p: f64) -> Result<Self, String> {
        Self::new(p).ok_or_else(|| format!("{p} is not a probability"))
    }
}

impl fmt::Display for Prob {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Per-call ceilings an adapter declares.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<u32>,
}

/// The floors an adapter ships, each meaning P(correct); `read` never above `write`, and no destructive number
/// exists.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawGate")]
pub struct Gate {
    route: Prob,
    #[serde(skip_serializing_if = "Option::is_none")]
    fits: Option<Prob>,
    read: Prob,
    write: Prob,
}

#[derive(Deserialize)]
struct RawGate {
    route: Prob,
    #[serde(default)]
    fits: Option<Prob>,
    read: Prob,
    write: Prob,
}

impl Gate {
    pub fn new(route: Prob, fits: Option<Prob>, read: Prob, write: Prob) -> Result<Self, String> {
        if read > write {
            Err(format!("read {read} is above write {write}"))
        } else {
            Ok(Self {
                route,
                fits,
                read,
                write,
            })
        }
    }

    #[must_use]
    pub fn route(&self) -> Prob {
        self.route
    }

    /// The runner-up's floor; a plan without `fits` questions skips it.
    #[must_use]
    pub fn fits(&self) -> Option<Prob> {
        self.fits
    }

    #[must_use]
    pub fn read(&self) -> Prob {
        self.read
    }

    #[must_use]
    pub fn write(&self) -> Prob {
        self.write
    }
}

impl TryFrom<RawGate> for Gate {
    type Error = String;

    fn try_from(raw: RawGate) -> Result<Self, String> {
        Self::new(raw.route, raw.fits, raw.read, raw.write)
    }
}

/// What an adapter declares about itself.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Declared {
    pub id: AdapterId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<Limits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<Gate>,
}

/// Answers as the adapter gave them: per question, a number per key. `read` validates them against their `Request`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Raw(pub IndexMap<String, IndexMap<String, f64>>);

/// How an adapter fails, or how its answers failed validation; each ends in a fixing command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Fault {
    Transport {
        message: String,
    },
    Status {
        status: u16,
    },
    /// The engine refused the key the named variable holds; the fix is a new value for it.
    Refused {
        credential: VarName,
    },
    Retired {
        id: AdapterId,
    },
    Unanswered {
        question: QuestionId,
    },
    Malformed {
        question: QuestionId,
        message: String,
    },
    Unrecorded {
        identity: Identity,
    },
}

impl fmt::Display for Fault {
    /// The words both hosts report a fault with, before its fix.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transport { message } => f.write_str(message),
            Self::Status { status } => write!(f, "the adapter answered {status}"),
            Self::Refused { credential } => write!(f, "the key in {credential} was refused"),
            Self::Retired { id } => write!(f, "adapter {id} is retired"),
            Self::Unanswered { question } => write!(f, "{question} was not answered"),
            Self::Malformed { question, message } => write!(f, "{question}: {message}"),
            Self::Unrecorded { identity } => write!(f, "\"{identity}\" is not recorded"),
        }
    }
}

impl Fault {
    #[must_use]
    pub fn fix(&self) -> Fix {
        match self {
            Self::Retired { .. } => Fix::Update { reflex: None },
            Self::Refused { credential } => Fix::ExportKey {
                var: credential.clone(),
            },
            Self::Transport { .. }
            | Self::Status { .. }
            | Self::Unanswered { .. }
            | Self::Malformed { .. }
            | Self::Unrecorded { .. } => Fix::Rerun,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(p: f64) -> Prob {
        Prob::new(p).unwrap()
    }

    #[test]
    fn question_ids_are_their_strings() {
        for text in ["route", "fits.timer", "timer.duration", "weave.split_0"] {
            let id = QuestionId::parse(text).unwrap();
            assert_eq!(id.to_string(), text);
            assert_eq!(serde_json::to_string(&id).unwrap(), format!("\"{text}\""));
        }
        assert!(QuestionId::parse("fits").is_err());
        assert!(QuestionId::parse("weave").is_err());
        assert!(QuestionId::parse("weave.Split").is_err());
        assert!(QuestionId::parse("Lights.room").is_err());
        assert!(QuestionId::parse("a.b.c").is_err());
        assert!(matches!(
            QuestionId::parse("weave.split_0").unwrap(),
            QuestionId::Weave(_)
        ));
        assert_eq!(QuestionId::parse("weave.split_0").unwrap().reflex(), None);
        assert_eq!(
            QuestionId::parse("fits.timer")
                .unwrap()
                .reflex()
                .map(LocalName::as_str),
            Some("timer")
        );
    }

    #[test]
    fn a_gate_keeps_read_at_or_below_write() {
        assert!(Gate::new(p(0.5), Some(p(0.3)), p(0.6), p(0.8)).is_ok());
        assert_eq!(
            Gate::new(p(0.5), None, p(0.9), p(0.8)).unwrap_err(),
            "read 0.9 is above write 0.8"
        );
        assert!(
            serde_json::from_str::<Gate>("{\"route\":0.5,\"read\":0.9,\"write\":0.8}").is_err()
        );
        assert!(serde_json::from_str::<Prob>("1.5").is_err());
        assert!(serde_json::from_str::<Prob>("-0.0").is_ok());
    }

    #[test]
    fn a_choice_names_its_sentinel_among_its_options() {
        let ask = Clean::new("Which?").unwrap();
        let options: IndexMap<Key, Text> = [(
            Key::new("a").unwrap(),
            Text::Plain(Clean::new("A.").unwrap()),
        )]
        .into_iter()
        .collect();
        assert!(
            Choice::new(
                ask.clone(),
                options.clone(),
                Some(Key::new("none").unwrap())
            )
            .is_err()
        );
        assert!(Choice::new(ask, options, Some(Key::new("a").unwrap())).is_ok());
        let json = "{\"type\":\"choice\",\"ask\":\"Which?\",\"options\":{\"a\":\"A.\"},\"otherwise\":\"b\"}";
        assert!(serde_json::from_str::<Question>(json).is_err());
    }

    #[test]
    fn a_text_is_a_string_or_an_object_without_empty_lists() {
        let rich = Text::Rich {
            what: Clean::new("On.").unwrap(),
            not_for: Vec::new(),
            examples: vec![Clean::new("lights on").unwrap()],
        };
        assert_eq!(
            serde_json::to_value(&rich).unwrap(),
            serde_json::json!({ "what": "On.", "examples": ["lights on"] })
        );
        assert_eq!(
            serde_json::from_value::<Text>(serde_json::json!("On."))
                .unwrap()
                .what()
                .as_str(),
            "On."
        );
    }

    #[test]
    fn every_fault_has_its_fix() {
        let question = QuestionId::Route;
        let rerun = [
            Fault::Transport {
                message: "reset".to_owned(),
            },
            Fault::Status { status: 503 },
            Fault::Unanswered {
                question: question.clone(),
            },
            Fault::Malformed {
                question,
                message: "no probabilities".to_owned(),
            },
            Fault::Unrecorded {
                identity: crate::text::identity("what time is it"),
            },
        ];
        for fault in rerun {
            assert_eq!(fault.fix(), Fix::Rerun, "{fault}");
        }
        let key = VarName::new("TYPESAFE_API_KEY").unwrap();
        assert_eq!(
            Fault::Refused {
                credential: key.clone()
            }
            .fix(),
            Fix::ExportKey { var: key }
        );
        assert_eq!(
            Fault::Retired {
                id: AdapterId::new("engine-1").unwrap()
            }
            .fix(),
            Fix::Update { reflex: None }
        );
    }

    #[test]
    fn a_fault_prints_and_serializes() {
        let id = QuestionId::Route;
        assert_eq!(
            Fault::Retired {
                id: AdapterId::new("engine-1").unwrap()
            }
            .fix(),
            Fix::Update { reflex: None }
        );
        assert_eq!(Fault::Unanswered { question: id }.fix(), Fix::Rerun);
        assert_eq!(
            serde_json::to_value(Fault::Status { status: 429 }).unwrap(),
            serde_json::json!({ "type": "status", "status": 429 })
        );
        let credential = VarName::new("TYPESAFE_API_KEY").unwrap();
        let refused = Fault::Refused {
            credential: credential.clone(),
        };
        assert_eq!(
            refused.to_string(),
            "the key in TYPESAFE_API_KEY was refused"
        );
        assert_eq!(refused.fix(), Fix::ExportKey { var: credential });
        assert_eq!(
            serde_json::to_value(&refused).unwrap(),
            serde_json::json!({ "type": "refused", "credential": "TYPESAFE_API_KEY" })
        );
    }
}
