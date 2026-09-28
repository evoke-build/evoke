//! Cases from records, a verdict per decision, the regressions against the last baseline, and the thief report.
//! In: `&Installed`; a `Case` with the `Decision` made of its utterance; the last `Baseline` with every case's
//! repeated verdicts; at `add`, the newcomers and the route winner per installed case. Out: every `Case`, examples
//! then tests in file order; a `Verdict`; every `Regression` and the next `Baseline`; every `Theft`.

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::adapter::Prob;
use crate::call::Value;
use crate::decide::{Contender, Decision};
use crate::manifest::{Assertion, Record, Sentence};
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

/// Where a case came from: examples are sent to the classifier, tests are held out; a step is a playbook's own
/// sentence, judged on where its words route once its slots are filled from a record's reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    Examples,
    Tests,
    Steps,
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

/// The last run's verdict per case, by reflex and utterance identity — the records in one map, a playbook's
/// steps in another, so a step and a test of the same words keep two verdicts; the host keeps one per plan
/// digest, so a switched engine or a changed set starts afresh and reports no phantom regressions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub records: IndexMap<LocalName, IndexMap<Identity, Verdict>>,
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub steps: IndexMap<LocalName, IndexMap<Identity, Verdict>>,
}

/// A case that passed at the last run and fails now, two of three uncached repeats.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Regression {
    pub case: Case,
    pub now: Mismatch,
}

/// A phrase an installed reflex claims that a newcomer wins at `add` — or, with `fits`, one the newcomer fits
/// over the floor, so every such request would stop at confirm.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Theft {
    pub phrase: Utterance,
    pub owner: LocalName,
    pub thief: LocalName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fits: Option<Prob>,
}

/// An installed case routed over the new set at `add`: who won, and every reflex offered with its `fits`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Routed {
    pub case: Case,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<LocalName>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ranking: Vec<Contender>,
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

/// Every step of every playbook with wording, as cases of the steps table: the playbook as its reflex, the
/// sentence with its slots as its utterance, nothing asserted. Judged on where the sentence routes once filled.
#[must_use]
pub fn steps(set: &Installed) -> Vec<Case> {
    let mut cases = Vec::new();
    for (reflex, item) in &set.reflexes {
        let Ok(effective) = &item.wording else {
            continue;
        };
        for sentence in &effective.manifest.steps {
            let Ok(utterance) = Utterance::new(&sentence.to_string()) else {
                continue;
            };
            cases.push(Case {
                reflex: reflex.clone(),
                utterance,
                expect: Expected::Asserts(IndexMap::new()),
                from: Table::Steps,
            });
        }
    }
    cases
}

/// A step of a playbook, filled from the first decision whose reading fills every slot it holds: the sentence
/// with the decision's values in its words, or none when no decision fills it. Never a word of the tool's own.
#[must_use]
pub fn filled(sentence: &Sentence, decisions: &[&Decision]) -> Option<String> {
    decisions.iter().find_map(|decision| {
        let args = match decision {
            Decision::Run { chosen } | Decision::Confirm { chosen, .. } => &chosen.call.args,
            Decision::Ask { asking, .. } => &asking.args,
            Decision::Abstain { .. } => return None,
        };
        sentence.filled(args).map(|(text, _)| text)
    })
}

/// A case against the decision made of its utterance: a never-case passes unless the route came to its reflex; an
/// asserting case needs the route, then each claim met by what was read — an ask reads its missing arguments as
/// unstated, a confirm is judged by its call, and an argument the record does not name is not compared. A step
/// passes when its words route to a reflex; one that routes to nothing fails, and so does one that routes to the
/// playbook it stands in, which the plan would refuse as nested.
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
    if case.from == Table::Steps {
        return match reflex {
            Some(read) if !routed => {
                let _ = read;
                Verdict::Pass
            }
            read => route(read),
        };
    }
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
        next.table_mut(case.from)
            .entry(case.reflex.clone())
            .or_default()
            .insert(case.utterance.id().clone(), verdict);
    }
    next
}

/// Every installed phrase a newcomer stole — an asserted case of a reflex that is not a newcomer, routed to one —
/// and, under `floor`, every one a newcomer fits at or over it without winning, which holds the phrase at
/// confirm; a phrase reports each thief once.
#[must_use]
pub fn thieves(newcomers: &[LocalName], routed: &[Routed], floor: Option<Prob>) -> Vec<Theft> {
    let mut thefts = Vec::new();
    for routed in routed {
        let case = &routed.case;
        if newcomers.contains(&case.reflex) || !matches!(case.expect, Expected::Asserts(_)) {
            continue;
        }
        let theft = |thief: &LocalName, fits: Option<Prob>| Theft {
            phrase: case.utterance.clone(),
            owner: case.reflex.clone(),
            thief: thief.clone(),
            fits,
        };
        if let Some(winner) = routed.winner.as_ref().filter(|w| newcomers.contains(w)) {
            thefts.push(theft(winner, None));
        }
        let Some(floor) = floor else { continue };
        for contender in &routed.ranking {
            if !newcomers.contains(&contender.reflex)
                || routed.winner.as_ref() == Some(&contender.reflex)
            {
                continue;
            }
            if let Some(fits) = contender.fits
                && fits.get() >= floor.get()
            {
                thefts.push(theft(&contender.reflex, Some(fits)));
            }
        }
    }
    thefts
}

impl Baseline {
    /// The case's verdict at the last run, when it was judged then.
    #[must_use]
    pub fn get(&self, case: &Case) -> Option<&Verdict> {
        self.table(case.from)
            .get(&case.reflex)?
            .get(case.utterance.id())
    }

    /// The map a case's table keeps its verdicts in: the records', or the steps'.
    fn table(&self, from: Table) -> &IndexMap<LocalName, IndexMap<Identity, Verdict>> {
        match from {
            Table::Examples | Table::Tests => &self.records,
            Table::Steps => &self.steps,
        }
    }

    fn table_mut(&mut self, from: Table) -> &mut IndexMap<LocalName, IndexMap<Identity, Verdict>> {
        match from {
            Table::Examples | Table::Tests => &mut self.records,
            Table::Steps => &mut self.steps,
        }
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
        let baseline: Baseline = serde_json::from_str(
            r#"{ "records": { "timer": { "what time is it": { "type": "pass" } } } }"#,
        )
        .unwrap();
        assert_eq!(
            serde_json::to_string(&baseline).unwrap(),
            r#"{"records":{"timer":{"what time is it":{"type":"pass"}}}}"#
        );
        assert_eq!(serde_json::to_string(&Baseline::default()).unwrap(), "{}");
    }
}
