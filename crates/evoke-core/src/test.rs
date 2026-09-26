//! Cases from records, a verdict per decision, the regressions against the last baseline, and the thief report.
//! In: `&Installed`; a `Case` with the `Decision` made of its utterance; the last `Baseline` with every case's
//! repeated verdicts; at `add`, the newcomers and the route winner per installed case. Out: every `Case`, examples
//! then tests in file order; a `Verdict`; every `Regression` and the next `Baseline`; every `Theft`.

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::call::Value;
use crate::decide::Decision;
use crate::manifest::{Assertion, Record};
use crate::name::{ArgName, LocalName};
use crate::plan::Installed;
use crate::text::{Clean, Identity, NonEmpty, Utterance};

/// One record as a test: whose it is, what was said, what should come of it, and the table it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Case {
    pub reflex: LocalName,
    pub utterance: Utterance,
    pub expect: Expected,
    pub from: Table,
}

/// Where a case came from: examples are sent to the classifier, tests are held out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    Examples,
    Tests,
}

/// What a record expects, as a decision compares to it: never this reflex, or the route with a claim per argument.
/// On the wire it looks as the record does — `false`, or `{ arg: false | true | "text" }` — and reads back without
/// the manifest, which a typed assertion cannot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expected {
    Never,
    Asserts(IndexMap<ArgName, Claim>),
}

/// One argument's claim: unstated, a flag raised, or the text a decision's value must show — an option key, a word
/// or a span, which all compare as text. What a decision read takes the same shape.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Claim {
    Unstated,
    Flag,
    Text(Clean),
}

/// What a decision made of a case: it passed, or where it first missed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail { mismatch: Mismatch },
}

/// Where a decision missed a case: the route — read as another reflex, or as none — or the first asserted argument
/// read as something else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Mismatch {
    Route { read: Option<LocalName> },
    Arg { arg: ArgName, read: Claim },
}

/// The last run's verdict per case, by reflex and utterance identity; the host keeps one per plan digest, so a
/// switched engine or a changed set starts afresh and reports no phantom regressions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Baseline(IndexMap<LocalName, IndexMap<Identity, Verdict>>);

/// A case that passed at the last run and fails now, two of three uncached repeats.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Regression {
    pub case: Case,
    pub now: Mismatch,
}

/// A phrase an installed reflex claims that a newcomer wins at `add`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Theft {
    pub phrase: Utterance,
    pub owner: LocalName,
    pub thief: LocalName,
}

/// Every record of every reflex with wording: examples then tests, in file order.
#[must_use]
pub fn cases(set: &Installed) -> Vec<Case> {
    let mut cases = Vec::new();
    for (reflex, item) in &set.reflexes {
        let Ok(effective) = &item.wording else {
            continue;
        };
        let tables = [
            (Table::Examples, &effective.manifest.examples),
            (Table::Tests, &effective.manifest.tests),
        ];
        for (from, records) in tables {
            for (id, (text, record)) in records.iter() {
                cases.push(Case {
                    reflex: reflex.clone(),
                    utterance: Utterance::of(text, id),
                    expect: Expected::of(record),
                    from,
                });
            }
        }
    }
    cases
}

/// A case against the decision made of its utterance: a never-case passes unless the route came to its reflex; an
/// asserting case needs the route, then each claim met by what was read — an ask reads its missing arguments as
/// unstated, a confirm is judged by its call, and an argument the record does not name is not compared.
#[must_use]
pub fn judge(case: &Case, decision: &Decision) -> Verdict {
    let (reflex, args) = match decision {
        Decision::Abstain { .. } => (None, None),
        Decision::Run { chosen } | Decision::Confirm { chosen, .. } => {
            (Some(&chosen.call.reflex), Some(&chosen.call.args))
        }
        Decision::Ask { asking, .. } => (Some(&asking.reflex), Some(&asking.args)),
    };
    let routed = reflex == Some(&case.reflex);
    let route = |read: Option<&LocalName>| Verdict::Fail {
        mismatch: Mismatch::Route {
            read: read.cloned(),
        },
    };
    match &case.expect {
        Expected::Never if routed => route(Some(&case.reflex)),
        Expected::Never => Verdict::Pass,
        Expected::Asserts(_) if !routed => route(reflex),
        Expected::Asserts(claims) => {
            for (arg, claim) in claims {
                let read = Claim::read(args.and_then(|args| args.get(arg)));
                if read != *claim {
                    return Verdict::Fail {
                        mismatch: Mismatch::Arg {
                            arg: arg.clone(),
                            read,
                        },
                    };
                }
            }
            Verdict::Pass
        }
    }
}

/// Every case that passed at the last run and failed at least two of its repeats now, with its first miss.
#[must_use]
pub fn regressions(before: &Baseline, judged: &[(Case, NonEmpty<Verdict>)]) -> Vec<Regression> {
    judged
        .iter()
        .filter(|(case, _)| matches!(before.get(case), Some(Verdict::Pass)))
        .filter_map(|(case, verdicts)| {
            let mut misses = verdicts.iter().filter_map(Verdict::mismatch);
            let first = misses.next()?;
            misses.next()?;
            Some(Regression {
                case: case.clone(),
                now: first.clone(),
            })
        })
        .collect()
}

/// The last run's verdicts brought up to date: each judged case takes the majority of its repeats — the first miss
/// when at least half of them missed — and a case not judged this time keeps its verdict.
#[must_use]
pub fn baseline(before: &Baseline, judged: &[(Case, NonEmpty<Verdict>)]) -> Baseline {
    let mut next = before.clone();
    for (case, verdicts) in judged {
        let total = verdicts.iter().count();
        let mut misses = verdicts.iter().filter_map(Verdict::mismatch);
        let first = misses.next();
        let failed = 1 + misses.count();
        let verdict = match first {
            Some(mismatch) if failed * 2 >= total => Verdict::Fail {
                mismatch: mismatch.clone(),
            },
            _ => Verdict::Pass,
        };
        next.0
            .entry(case.reflex.clone())
            .or_default()
            .insert(case.utterance.id().clone(), verdict);
    }
    next
}

/// Every installed phrase a newcomer stole: an asserted case of a reflex that is not a newcomer, routed to one.
#[must_use]
pub fn thieves(newcomers: &[LocalName], routed: &[(Case, Option<LocalName>)]) -> Vec<Theft> {
    routed
        .iter()
        .filter_map(|(case, winner)| {
            let thief = winner.as_ref()?;
            let stolen = newcomers.contains(thief)
                && !newcomers.contains(&case.reflex)
                && matches!(case.expect, Expected::Asserts(_));
            stolen.then(|| Theft {
                phrase: case.utterance.clone(),
                owner: case.reflex.clone(),
                thief: thief.clone(),
            })
        })
        .collect()
}

impl Baseline {
    /// The case's verdict at the last run, when it was judged then.
    #[must_use]
    pub fn get(&self, case: &Case) -> Option<&Verdict> {
        self.0.get(&case.reflex)?.get(case.utterance.id())
    }
}

impl Verdict {
    fn mismatch(&self) -> Option<&Mismatch> {
        match self {
            Self::Pass => None,
            Self::Fail { mismatch } => Some(mismatch),
        }
    }
}

impl Expected {
    fn of(record: &Record) -> Self {
        match record {
            Record::Never => Self::Never,
            Record::Asserts(asserts) => Self::Asserts(
                asserts
                    .iter()
                    .map(|(arg, assertion)| (arg.clone(), Claim::of(assertion)))
                    .collect(),
            ),
        }
    }
}

impl Claim {
    fn of(assertion: &Assertion) -> Self {
        match assertion {
            Assertion::Unstated => Self::Unstated,
            Assertion::Flag => Self::Flag,
            // Proven: an option key and a word are clean by construction.
            Assertion::Option(key) => {
                Self::Text(Clean::new(key.as_str()).expect("an option key is clean"))
            }
            Assertion::Word(word) => {
                Self::Text(Clean::new(word.as_str()).expect("a word is clean"))
            }
            Assertion::Span(text) => Self::Text(text.clone()),
        }
    }

    /// What a decision read for an argument, in a claim's shape: absent is unstated, a flag is raised, and a key,
    /// a word or a span is its text.
    pub(crate) fn read(value: Option<&Value>) -> Self {
        match value {
            None => Self::Unstated,
            Some(Value::Flag) => Self::Flag,
            Some(value) => {
                let text = value.text().unwrap_or_default();
                // Proven: a key, a word and a span are clean by construction.
                Self::Text(Clean::new(text).expect("a value's text is clean"))
            }
        }
    }
}

impl Serialize for Expected {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Never => serializer.serialize_bool(false),
            Self::Asserts(claims) => claims.serialize(serializer),
        }
    }
}

impl Serialize for Claim {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unstated => serializer.serialize_bool(false),
            Self::Flag => serializer.serialize_bool(true),
            Self::Text(text) => serializer.serialize_str(text.as_str()),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawExpected {
    Never(bool),
    Asserts(IndexMap<ArgName, Claim>),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RawClaim {
    Bool(bool),
    Text(String),
}

impl<'de> Deserialize<'de> for Expected {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match RawExpected::deserialize(deserializer)? {
            RawExpected::Never(false) => Ok(Self::Never),
            RawExpected::Never(true) => {
                Err(D::Error::custom("a record is false or a table of claims"))
            }
            RawExpected::Asserts(claims) => Ok(Self::Asserts(claims)),
        }
    }
}

impl<'de> Deserialize<'de> for Claim {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match RawClaim::deserialize(deserializer)? {
            RawClaim::Bool(false) => Ok(Self::Unstated),
            RawClaim::Bool(true) => Ok(Self::Flag),
            RawClaim::Text(text) => Clean::line(&text)
                .map(Self::Text)
                .map_err(|why| D::Error::custom(format!("\"{text}\" {why}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_expectation_round_trips_as_its_record() {
        let expected: Expected =
            serde_json::from_str(r#"{ "state": "on", "brightness": false, "all": true }"#).unwrap();
        let json = serde_json::to_string(&expected).unwrap();
        assert_eq!(json, r#"{"state":"on","brightness":false,"all":true}"#);
        assert_eq!(
            serde_json::from_str::<Expected>("false").unwrap(),
            Expected::Never
        );
        assert!(serde_json::from_str::<Expected>("true").is_err());
    }

    #[test]
    fn a_verdict_and_a_baseline_take_the_wire_form() {
        let fail: Verdict = serde_json::from_str(
            r#"{ "type": "fail", "mismatch": { "type": "arg", "arg": "state", "read": "off" } }"#,
        )
        .unwrap();
        assert!(matches!(
            &fail,
            Verdict::Fail {
                mismatch: Mismatch::Arg { read: Claim::Text(text), .. }
            } if text.as_str() == "off"
        ));
        let baseline: Baseline =
            serde_json::from_str(r#"{ "timer": { "what time is it": { "type": "pass" } } }"#)
                .unwrap();
        assert_eq!(
            serde_json::to_string(&baseline).unwrap(),
            r#"{"timer":{"what time is it":{"type":"pass"}}}"#
        );
    }
}
