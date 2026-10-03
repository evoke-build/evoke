//! The contract between two versions of a reflex: what a body receives and a call may name; and lint, what a
//! manifest may say to an engine. In: the previous and the next `Manifest`; the locked and the upstream `Effect`; a
//! `Manifest`. Out: `ContractDiff` — its level, every change, every `was` violation — `Consent`, and every
//! `Finding`. Wording is never a change: a description, an ask or an option's meaning may move freely.

use std::cmp::Ordering;

use indexmap::IndexMap;
use serde::Serialize;

use crate::contain::Platform;
use crate::document::KeyPath;
use crate::manifest::{
    Argument, Assertion, Effect, Element, Kind, Manifest, Part, Recognizer, Record, Run, Sentence,
    Source, renames,
};
use crate::name::{ArgName, ConfigKey, FieldName, OptionKey};
use crate::needs::{self, Needs};
use crate::text::identity;
use crate::weave::reading;
use crate::words;

/// What changed in the contract from one version to the next, and how much it matters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContractDiff {
    pub level: Level,
    pub changes: Vec<Change>,
    pub violations: Vec<WasViolation>,
}

/// `Same`: nothing but wording, or a declaration narrowed. `Minor`: additions, a declaration widened, and a config
/// key gone, which leaves a setting orphaned and skipped. `Major`: something a person's files or calls may not
/// survive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Same,
    Minor,
    Major,
}

/// One change to the contract, in the order the diff walks: the previous arguments, the added ones, the body,
/// what it may touch, config, what the result yields, what the reflex takes, then what it returns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Change {
    ArgRemoved {
        arg: ArgName,
    },
    OptionRemoved {
        arg: ArgName,
        key: OptionKey,
    },
    ArgRenamed {
        from: ArgName,
        to: ArgName,
    },
    SourceChanged {
        arg: ArgName,
    },
    RangeChanged {
        arg: ArgName,
    },
    /// The yielded field a pick's ask lists recent values of came, went or moved: what an ask offers changes.
    RecentChanged {
        arg: ArgName,
    },
    RunChanged,
    /// A playbook's steps moved: one added, removed, reworded or reordered. The plan a person's yes covered is
    /// another plan.
    StepsChanged,
    /// The declaration reaches more: what was added, which a person's lock keeps out until they accept it.
    NeedsWidened {
        added: Needs,
    },
    /// The declaration reaches less: what was dropped, which applies at once.
    NeedsNarrowed {
        removed: Needs,
    },
    /// An optional argument made required: a call or an example without it no longer stands.
    Required {
        arg: ArgName,
    },
    /// A config key made secret: a plain setting for it turns the reflex inactive.
    ConfigSecret {
        key: ConfigKey,
        secret: bool,
    },
    ArgAdded {
        arg: ArgName,
    },
    /// A required argument made optional.
    Optional {
        arg: ArgName,
    },
    OptionAdded {
        arg: ArgName,
        key: OptionKey,
    },
    ConfigAdded {
        key: ConfigKey,
    },
    ConfigRemoved {
        key: ConfigKey,
    },
    YieldAdded {
        field: FieldName,
    },
    YieldRemoved {
        field: FieldName,
    },
    YieldChanged {
        field: FieldName,
    },
    /// An argument an earlier step's whole result fills, added: a request that ran the reflex needs a source now.
    TakesAdded {
        arg: ArgName,
        name: FieldName,
    },
    TakesRemoved {
        arg: ArgName,
        name: FieldName,
    },
    /// The name a taken argument takes moved: `name` is the new one.
    TakesChanged {
        arg: ArgName,
        name: FieldName,
    },
    /// The body's whole result named: a later step may take it now.
    ReturnsAdded {
        name: FieldName,
    },
    ReturnsRemoved {
        name: FieldName,
    },
    /// The name the whole result goes by moved: `name` is the new one; a request that took it stands no more.
    ReturnsChanged {
        name: FieldName,
    },
    /// A platform the body runs on, gained: the reflex is active there now.
    PlatformAdded {
        platform: Platform,
    },
    /// A platform dropped: the reflex is inactive there from now on.
    PlatformRemoved {
        platform: Platform,
    },
}

impl Change {
    /// How much the change matters: major when it can break what a person wrote — an overlay, a call, an example,
    /// a request that takes a yield or a whole result into a later step; minor for an addition, a config key
    /// gone, a declaration widened; a declaration narrowed changes nothing a person holds.
    fn level(&self) -> Level {
        match self {
            Self::ArgRemoved { .. }
            | Self::OptionRemoved { .. }
            | Self::ArgRenamed { .. }
            | Self::SourceChanged { .. }
            | Self::RangeChanged { .. }
            | Self::RecentChanged { .. }
            | Self::RunChanged
            | Self::StepsChanged
            | Self::Required { .. }
            | Self::ConfigSecret { secret: true, .. }
            | Self::YieldRemoved { .. }
            | Self::YieldChanged { .. }
            | Self::TakesAdded { .. }
            | Self::TakesRemoved { .. }
            | Self::TakesChanged { .. }
            | Self::ReturnsRemoved { .. }
            | Self::ReturnsChanged { .. }
            | Self::PlatformRemoved { .. } => Level::Major,
            Self::ConfigSecret { secret: false, .. }
            | Self::ArgAdded { .. }
            | Self::Optional { .. }
            | Self::OptionAdded { .. }
            | Self::ConfigAdded { .. }
            | Self::ConfigRemoved { .. }
            | Self::YieldAdded { .. }
            | Self::ReturnsAdded { .. }
            | Self::PlatformAdded { .. }
            | Self::NeedsWidened { .. } => Level::Minor,
            Self::NeedsNarrowed { .. } => Level::Same,
        }
    }
}

/// The platforms dropped, then those gained; a manifest that names none runs anywhere.
fn platforms(previous: &Manifest, next: &Manifest) -> Vec<Change> {
    let covers =
        |named: &[Platform], platform: Platform| named.is_empty() || named.contains(&platform);
    let mut changes = Vec::new();
    for platform in Platform::ALL {
        match (
            covers(&previous.platforms, platform),
            covers(&next.platforms, platform),
        ) {
            (true, false) => changes.push(Change::PlatformRemoved { platform }),
            (false, true) => changes.push(Change::PlatformAdded { platform }),
            _ => {}
        }
    }
    changes
}

/// `was` is flat and cumulative: a retired name never returns as a live argument, and never leaves the lists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WasViolation {
    Returned { arg: ArgName },
    Dropped { arg: ArgName },
}

/// What an update does to the effect a person consented to: upstream may keep or tighten it, and loosens it only
/// through `evoke update --accept`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Consent {
    Kept { effect: Effect },
    Tightened { effect: Effect },
    NeedsAccept { locked: Effect, upstream: Effect },
}

/// The contract diff: every argument of `previous` followed through `was`, then what `next` adds, the body, what
/// it may touch, config, what it yields.
#[must_use]
pub fn diff(previous: &Manifest, next: &Manifest) -> ContractDiff {
    let renamed = renames(previous, next);
    let mut changes = Vec::new();
    for (name, before) in &previous.args {
        let (current, after) = if let Some(found) = next.args.get_key_value(name) {
            found
        } else if let Some(found) = renamed.get(name).and_then(|to| next.args.get_key_value(to)) {
            changes.push(Change::ArgRenamed {
                from: name.clone(),
                to: found.0.clone(),
            });
            found
        } else {
            changes.push(Change::ArgRemoved { arg: name.clone() });
            continue;
        };
        changes.extend(compare(current, &before.kind, &after.kind));
        if before.recent != after.recent {
            changes.push(Change::RecentChanged {
                arg: current.clone(),
            });
        }
    }
    for name in next.args.keys() {
        if !previous.args.contains_key(name) && !renamed.values().any(|to| to == name) {
            changes.push(Change::ArgAdded { arg: name.clone() });
        }
    }
    if followed(&previous.run, &renamed) != next.run {
        changes.push(Change::RunChanged);
    }
    if previous.steps != next.steps {
        changes.push(Change::StepsChanged);
    }
    let added = needs::added(&next.needs, &previous.needs);
    if !added.is_none() {
        changes.push(Change::NeedsWidened { added });
    }
    let removed = needs::added(&previous.needs, &next.needs);
    if !removed.is_none() {
        changes.push(Change::NeedsNarrowed { removed });
    }
    for (key, before) in &previous.config {
        match next.config.get(key) {
            None => changes.push(Change::ConfigRemoved { key: key.clone() }),
            Some(after) if after.secret != before.secret => changes.push(Change::ConfigSecret {
                key: key.clone(),
                secret: after.secret,
            }),
            Some(_) => {}
        }
    }
    for key in next.config.keys() {
        if !previous.config.contains_key(key) {
            changes.push(Change::ConfigAdded { key: key.clone() });
        }
    }
    for (field, before) in &previous.yields {
        match next.yields.get(field) {
            None => changes.push(Change::YieldRemoved {
                field: field.clone(),
            }),
            Some(after) if after != before => changes.push(Change::YieldChanged {
                field: field.clone(),
            }),
            Some(_) => {}
        }
    }
    for field in next.yields.keys() {
        if !previous.yields.contains_key(field) {
            changes.push(Change::YieldAdded {
                field: field.clone(),
            });
        }
    }
    changes.extend(joined(previous, next));
    changes.extend(platforms(previous, next));
    let level = changes
        .iter()
        .map(Change::level)
        .max()
        .unwrap_or(Level::Same);
    ContractDiff {
        level,
        changes,
        violations: violations(previous, next, &renamed),
    }
}

/// The changes to what the reflex takes, per argument, and to what it returns.
fn joined(previous: &Manifest, next: &Manifest) -> Vec<Change> {
    let mut changes = Vec::new();
    for (arg, before) in &previous.takes {
        match next.takes.get(arg) {
            None => changes.push(Change::TakesRemoved {
                arg: arg.clone(),
                name: before.clone(),
            }),
            Some(after) if after != before => changes.push(Change::TakesChanged {
                arg: arg.clone(),
                name: after.clone(),
            }),
            Some(_) => {}
        }
    }
    for (arg, name) in &next.takes {
        if !previous.takes.contains_key(arg) {
            changes.push(Change::TakesAdded {
                arg: arg.clone(),
                name: name.clone(),
            });
        }
    }
    match (&previous.returns, &next.returns) {
        (None, Some(name)) => changes.push(Change::ReturnsAdded { name: name.clone() }),
        (Some(name), None) => changes.push(Change::ReturnsRemoved { name: name.clone() }),
        (Some(before), Some(after)) if before != after => {
            changes.push(Change::ReturnsChanged {
                name: after.clone(),
            });
        }
        _ => {}
    }
    changes
}

/// The effect a person runs under after an update: theirs, unless upstream tightened it.
#[must_use]
pub fn consent(locked: Effect, upstream: Effect) -> Consent {
    match upstream.cmp(&locked) {
        Ordering::Equal => Consent::Kept { effect: locked },
        Ordering::Greater => Consent::Tightened { effect: upstream },
        Ordering::Less => Consent::NeedsAccept { locked, upstream },
    }
}

/// What `lint` finds. What an engine must not be sent: a size cap passed, text that addresses the model instead
/// of describing an action. What a plan would misread: a step holding a connective the reader splits on, a step
/// stating a word another team would change, a step referring to an earlier one by `that <noun>` where `whether`
/// is meant, a step worded as the playbook's own summary or examples. What a reader of the file would miss:
/// `effect` left out, a quoted argument no example shows in quotes, fewer than three examples, an option or a
/// pick no record leaves out, a confirm that reads no value back or asks `Are you sure`, an option's meaning that
/// repeats its ask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LintRule {
    SizeCap,
    AddressesModel,
    Connective,
    Literal,
    Reference,
    Situation,
    Effect,
    Quoted,
    Examples,
    Unstated,
    Confirm,
    Meaning,
}

/// One thing `lint` found, at the key path it concerns; reported at `add` and by `check`, never a refusal.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub rule: LintRule,
    pub path: KeyPath,
    pub message: String,
}

/// The caps, in characters and entries: what one manifest may send an engine.
const SUMMARY_CHARS: usize = 100;
const DESCRIPTION_CHARS: usize = 1_000;
const NOT_FOR_ENTRIES: usize = 8;
const OPTIONS: usize = 24;
const RECORDS: usize = 40;
const UTTERANCE_CHARS: usize = 200;

/// The fewest examples a manifest shows of its request.
const EXAMPLES: usize = 3;

/// The words every sentence holds, which say nothing of what a step does: left out when a step's words are
/// weighed against the playbook's own.
const FUNCTION_WORDS: [&str; 32] = [
    "a", "an", "the", "of", "to", "for", "in", "on", "at", "by", "with", "from", "into", "and",
    "or", "it", "its", "them", "their", "that", "this", "these", "those", "is", "are", "be", "s",
    "my", "our", "your", "each", "every",
];

/// Phrases that address a model rather than describe an action, matched on whole words, case aside; the first
/// found names the finding.
const ADDRESSES_MODEL: [&str; 25] = [
    "ignore previous",
    "ignore all previous",
    "ignore the above",
    "ignore any",
    "disregard previous",
    "you are a",
    "you are an",
    "you must",
    "you should always",
    "always choose",
    "always select",
    "always pick",
    "always route",
    "always answer",
    "never choose",
    "never select",
    "choose this",
    "select this",
    "pick this",
    "as an ai",
    "as a language model",
    "system prompt",
    "the classifier",
    "the model",
    "the assistant",
];

/// Lint: every finding in the order `evoke show` prints a manifest — the description, `not_for`, `effect`,
/// `confirm`, the steps, each argument, then the examples and the tests.
#[must_use]
pub fn lint(m: &Manifest) -> Vec<Finding> {
    let mut findings = Vec::new();
    let description = KeyPath::new(["description"]);
    let whole = m.description.to_string();
    cap_chars(
        &mut findings,
        &description,
        "the summary",
        m.description.summary().as_str(),
        SUMMARY_CHARS,
    );
    cap_chars(
        &mut findings,
        &description,
        "description",
        &whole,
        DESCRIPTION_CHARS,
    );
    addresses(&mut findings, &description, &whole);
    let not_for = KeyPath::new(["not_for"]);
    cap_entries(
        &mut findings,
        &not_for,
        "not_for",
        "entries",
        m.not_for.len(),
        NOT_FOR_ENTRIES,
    );
    for (i, text) in m.not_for.iter().enumerate() {
        addresses(&mut findings, &not_for.child(&i.to_string()), text.as_str());
    }
    if m.effect_absent {
        findings.push(Finding {
            rule: LintRule::Effect,
            path: KeyPath::new(["effect"]),
            message: "effect is absent, which means destructive; write it".to_owned(),
        });
    }
    lint_confirm(&mut findings, m);
    lint_steps(&mut findings, m);
    for (name, arg) in &m.args {
        lint_argument(&mut findings, m, name, arg);
    }
    let shown = m
        .examples
        .iter()
        .filter(|(_, (_, record))| matches!(record, Record::Asserts(_)))
        .count();
    if shown < EXAMPLES {
        let holds = match shown {
            0 => "no request".to_owned(),
            1 => "1 request".to_owned(),
            n => format!("{n} requests"),
        };
        findings.push(Finding {
            rule: LintRule::Examples,
            path: KeyPath::new(["examples"]),
            message: format!("examples holds {holds}; write three at least"),
        });
    }
    for (table, records) in [("examples", &m.examples), ("tests", &m.tests)] {
        let path = KeyPath::new([table]);
        cap_entries(
            &mut findings,
            &path,
            table,
            "records",
            records.iter().count(),
            RECORDS,
        );
        for (_, (utterance, _)) in records.iter() {
            let at = path.child(utterance.as_str());
            cap_chars(
                &mut findings,
                &at,
                &at.to_string(),
                utterance.as_str(),
                UTTERANCE_CHARS,
            );
            addresses(&mut findings, &at, utterance.as_str());
        }
    }
    findings
}

/// The confirm: the last line before the action reads the call back. One that names none of the required
/// arguments says nothing of this call, and `Are you sure` says nothing at all.
fn lint_confirm(findings: &mut Vec<Finding>, m: &Manifest) {
    let path = KeyPath::new(["confirm"]);
    let text = m.confirm.to_string();
    if words(&text)
        .windows(3)
        .any(|run| run == ["are", "you", "sure"])
    {
        findings.push(Finding {
            rule: LintRule::Confirm,
            path: path.clone(),
            message: "confirm asks \"Are you sure\"; say what the call will do".to_owned(),
        });
    }
    let required: Vec<String> = m
        .args
        .iter()
        .filter(|(_, arg)| {
            matches!(
                arg.kind,
                Kind::Value {
                    optional: false,
                    ..
                }
            )
        })
        .map(|(name, _)| format!("{{{name}}}"))
        .collect();
    if !required.is_empty() && m.confirm.placeholders().next().is_none() {
        findings.push(Finding {
            rule: LintRule::Confirm,
            path,
            message: format!(
                "confirm reads no value back; name {}",
                required.join(" and ")
            ),
        });
    }
}

/// One argument: its ask and its options clear of the model and under the cap; an option's meaning that repeats
/// the ask; a quoted pick no example shows in quotes; an option or a pick no record leaves out.
fn lint_argument(findings: &mut Vec<Finding>, m: &Manifest, name: &ArgName, arg: &Argument) {
    let path = KeyPath::new(["args", name.as_str()]);
    addresses(findings, &path.child("ask"), arg.ask.as_str());
    let Kind::Value { source, .. } = &arg.kind else {
        return;
    };
    match source {
        Source::Options(options) => {
            let at = path.child("options");
            cap_entries(
                findings,
                &at,
                &format!("args.{name}"),
                "options",
                options.len(),
                OPTIONS,
            );
            for (key, meaning) in options.iter() {
                addresses(findings, &at.child(key.as_str()), meaning.as_str());
                if identity(meaning.as_str()) == identity(arg.ask.as_str()) {
                    findings.push(Finding {
                        rule: LintRule::Meaning,
                        path: at.child(key.as_str()),
                        message: format!(
                            "args.{name}.options.{key} repeats the ask; a meaning answers it"
                        ),
                    });
                }
            }
        }
        Source::Pick(pick) => {
            let quotes = |text: &str| text.contains(['"', '\u{201c}', '\u{2018}']);
            if pick.recognizer() == Recognizer::Quoted
                && !m
                    .examples
                    .iter()
                    .any(|(_, (utterance, _))| quotes(utterance.as_str()))
            {
                findings.push(Finding {
                    rule: LintRule::Quoted,
                    path: path.clone(),
                    message: format!("args.{name} reads text in quotes, and no example shows them"),
                });
            }
        }
        Source::Vocab(_) => return,
    }
    let left_out = [&m.examples, &m.tests].into_iter().any(|records| {
        records.iter().any(|(_, (_, record))| {
            matches!(record, Record::Asserts(asserts)
                if asserts.get(name) == Some(&Assertion::Unstated))
        })
    });
    if !left_out {
        findings.push(Finding {
            rule: LintRule::Unstated,
            path,
            message: format!(
                "args.{name} has no record that leaves it out; assert {name} = false once"
            ),
        });
    }
}

fn cap_chars(findings: &mut Vec<Finding>, path: &KeyPath, what: &str, text: &str, cap: usize) {
    let n = text.chars().count();
    if n > cap {
        findings.push(Finding {
            rule: LintRule::SizeCap,
            path: path.clone(),
            message: format!("{what} is {n} characters; the cap is {cap}"),
        });
    }
}

fn cap_entries(
    findings: &mut Vec<Finding>,
    path: &KeyPath,
    what: &str,
    unit: &str,
    n: usize,
    cap: usize,
) {
    if n > cap {
        findings.push(Finding {
            rule: LintRule::SizeCap,
            path: path.clone(),
            message: format!("{what} has {n} {unit}; the cap is {cap}"),
        });
    }
}

/// A playbook's steps: each under the utterance cap and clear of the model; one holding a connective the reader
/// splits on is flagged, since an expanded sentence is never split and a noun phrase may hold an `and`; so is one
/// stating a channel or an address, which another team would name otherwise; and one whose every word that says
/// something the playbook's own summary or examples hold, since a step near the playbook's sentence reaches the
/// playbook, or nothing.
fn lint_steps(findings: &mut Vec<Finding>, m: &Manifest) {
    let mut own = content(m.description.summary().as_str());
    for (_, (utterance, record)) in m.examples.iter() {
        if matches!(record, Record::Asserts(_)) {
            own.extend(content(utterance.as_str()));
        }
    }
    for (i, sentence) in m.steps.iter().enumerate() {
        let at = KeyPath::new(["steps", &i.to_string()]);
        let text = sentence.to_string();
        cap_chars(
            findings,
            &at,
            &format!("step {}", i + 1),
            &text,
            UTTERANCE_CHARS,
        );
        addresses(findings, &at, &text);
        if let Some(split) = reading::splits(&text, true).first() {
            findings.push(Finding {
                rule: LintRule::Connective,
                path: at.clone(),
                message: format!(
                    "step {} holds \"{}\"; one step is one action",
                    i + 1,
                    split.word
                ),
            });
        }
        if let Some(literal) = literal(&text) {
            findings.push(Finding {
                rule: LintRule::Literal,
                path: at.clone(),
                message: format!(
                    "step {} states \"{literal}\"; a word another team would change is a slot",
                    i + 1
                ),
            });
        }
        // `check that writes land` names an earlier step by `that writes`, and would confirm as taking nothing.
        if let Some(noun) = reading::that_after(&text, "check") {
            findings.push(Finding {
                rule: LintRule::Reference,
                path: at.clone(),
                message: format!(
                    "step {} says \"check that {noun}\"; a step refers by \"that <noun>\": say \"check whether\"",
                    i + 1
                ),
            });
        }
        let said = content(&spoken(sentence));
        if !said.is_empty() && said.iter().all(|word| own.contains(word)) {
            findings.push(Finding {
                rule: LintRule::Situation,
                path: at,
                message: format!(
                    "step {}'s words are the summary's or an example's; word a step apart from the situation",
                    i + 1
                ),
            });
        }
    }
}

/// A step's own words, its slots aside.
fn spoken(sentence: &Sentence) -> String {
    fn write(parts: &[Part], text: &mut String) {
        for part in parts {
            match part {
                Part::Text(piece) => text.push_str(piece.as_str()),
                Part::Slot(_) => text.push(' '),
                Part::Optional(inner) => write(inner, text),
            }
        }
    }
    let mut text = String::new();
    write(sentence.parts(), &mut text);
    text
}

/// A text's words, lower-cased: its runs of letters and digits.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The words of a text that say something, each at its stem: the words every sentence holds left out.
fn content(text: &str) -> Vec<String> {
    words(text)
        .into_iter()
        .filter(|word| !FUNCTION_WORDS.contains(&word.as_str()))
        .map(|word| words::stem_of(&word))
        .collect()
}

/// A channel or an address written into a step's words — `#incident`, `ops@example.com` — which another team would
/// name otherwise: the first one.
fn literal(text: &str) -> Option<String> {
    text.split_whitespace()
        .map(|word| word.trim_matches(|c: char| matches!(c, ',' | '.' | ';' | ':' | '!' | '?')))
        .find(|word| {
            (word.starts_with('#') && word.len() > 1)
                || word
                    .split_once('@')
                    .is_some_and(|(user, host)| !user.is_empty() && host.contains('.'))
        })
        .map(str::to_owned)
}

fn addresses(findings: &mut Vec<Finding>, path: &KeyPath, text: &str) {
    // Whole words: "as an aid" is not "as an ai", and "the models" is not "the model".
    let padded = format!(" {} ", words(text).join(" "));
    if let Some(phrase) = ADDRESSES_MODEL
        .iter()
        .find(|phrase| padded.contains(&format!(" {phrase} ")))
    {
        findings.push(Finding {
            rule: LintRule::AddressesModel,
            path: path.clone(),
            message: format!("{path} addresses the model: \"{phrase}\""),
        });
    }
}

/// The changes to one argument that survives, under its current name: its source, and whether it may be left out.
fn compare(name: &ArgName, before: &Kind, after: &Kind) -> Vec<Change> {
    let (before, after, optional) = match (before, after) {
        (Kind::Flag, Kind::Flag) => return Vec::new(),
        (
            Kind::Value {
                source: a,
                optional: was,
            },
            Kind::Value {
                source: b,
                optional: is,
            },
        ) => (a, b, (*was, *is)),
        _ => return vec![Change::SourceChanged { arg: name.clone() }],
    };
    let mut changes = match optional {
        (true, false) => vec![Change::Required { arg: name.clone() }],
        (false, true) => vec![Change::Optional { arg: name.clone() }],
        _ => Vec::new(),
    };
    changes.extend(sourced(name, before, after));
    changes
}

/// The changes to an argument's source: option keys that came and went, a range or a recognizer that moved.
fn sourced(name: &ArgName, before: &Source, after: &Source) -> Vec<Change> {
    match (before, after) {
        (Source::Options(before), Source::Options(after)) => {
            let removed = before.keys().filter(|key| !after.contains_key(*key));
            let added = after.keys().filter(|key| !before.contains_key(*key));
            removed
                .map(|key| Change::OptionRemoved {
                    arg: name.clone(),
                    key: key.clone(),
                })
                .chain(added.map(|key| Change::OptionAdded {
                    arg: name.clone(),
                    key: key.clone(),
                }))
                .collect()
        }
        (Source::Vocab(before), Source::Vocab(after)) if before == after => Vec::new(),
        (Source::Pick(before), Source::Pick(after))
            if before.recognizer() == after.recognizer() =>
        {
            if before == after {
                Vec::new()
            } else {
                vec![Change::RangeChanged { arg: name.clone() }]
            }
        }
        _ => vec![Change::SourceChanged { arg: name.clone() }],
    }
}

/// The body as `next` would write it: a placeholder follows its rename, so a rename alone is not a body change.
fn followed(run: &Run, renamed: &IndexMap<ArgName, ArgName>) -> Run {
    let Run::Argv { program, rest } = run else {
        return run.clone();
    };
    Run::Argv {
        program: program.clone(),
        rest: rest
            .iter()
            .map(|element| match element {
                Element::Arg(name) => Element::Arg(renamed.get(name).unwrap_or(name).clone()),
                Element::Literal(_) => element.clone(),
            })
            .collect(),
    }
}

/// Every name `previous` had retired: back in use as a live argument, or gone from every `was` while its argument
/// survives — an argument removed takes its former names with it.
fn violations(
    previous: &Manifest,
    next: &Manifest,
    renamed: &IndexMap<ArgName, ArgName>,
) -> Vec<WasViolation> {
    let returned = previous
        .args
        .values()
        .flat_map(|arg| &arg.was)
        .filter(|old| next.args.contains_key(*old))
        .map(|old| WasViolation::Returned { arg: old.clone() });
    let dropped = previous
        .args
        .iter()
        .filter(|(name, _)| next.args.contains_key(*name) || renamed.contains_key(*name))
        .flat_map(|(_, arg)| &arg.was)
        .filter(|old| !next.args.values().any(|arg| arg.was.contains(old)))
        .map(|old| WasViolation::Dropped { arg: old.clone() });
    returned.chain(dropped).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::File;
    use crate::document::{Document, Text};
    use crate::manifest::manifest;
    use crate::name::LocalName;

    fn parse(args: &str, run: &str) -> Manifest {
        let text = format!(
            "reflex = 1\ndescription = \"Lights.\"\nconfirm = \"Lights?\"\nrun = {run}\n{args}"
        );
        manifest(Document {
            file: File::Manifest {
                name: LocalName::new("lights").unwrap(),
            },
            text: Text::Toml(&text),
        })
        .unwrap()
    }

    /// A reflex that keeps every rule lint holds, and a playbook that does: what each test below breaks once.
    const REFLEX: &str = r#"reflex = 1

description = """
Set the lights of the house.
On or off, and nothing else changes."""
effect  = "write"
confirm = "Switch the lights {state}?"
run     = "lights.mts"

[args.state]
ask = "What should the lights do?"
options.on  = "Switch on."
options.off = "Switch off."

[args.label]
ask      = "Under which name?"
pick     = "quoted"
optional = true

[examples]
"lights on"                        = { state = "on" }
"kill the lights"                  = { state = "off" }
'lights off, call it "bedtime"'    = { state = "off", label = "bedtime" }

[tests]
"do something with the lights" = { state = false, label = false }
"#;

    const PLAYBOOK: &str = r#"reflex = 1

description = """
Handle a service that is down or failing.
Finds the errors and the deploys, then rolls back the release behind them."""
effect  = "destructive"
confirm = "Run the outage plan for {service}?"
steps   = ["check {service}'s errors", "list {service}'s deploys", "roll {service} back from that release"]

[args.service]
ask   = "Which service is down?"
vocab = "services"

[examples]
"payments is down"       = {}
"search is broken"       = {}
"checkout keeps failing" = {}
"#;

    fn read(text: &str) -> Manifest {
        manifest(Document {
            file: File::Manifest {
                name: LocalName::new("lights").unwrap(),
            },
            text: Text::Toml(text),
        })
        .unwrap()
    }

    /// The rules a manifest trips, in the order lint found them.
    fn rules(text: &str) -> Vec<LintRule> {
        lint(&read(text))
            .into_iter()
            .map(|finding| finding.rule)
            .collect()
    }

    /// The one finding of a manifest that breaks one rule once.
    fn found(text: &str) -> Finding {
        let mut findings = lint(&read(text));
        assert_eq!(findings.len(), 1, "{findings:#?}");
        findings.remove(0)
    }

    #[test]
    fn a_manifest_that_keeps_every_rule_is_clean() {
        assert_eq!(rules(REFLEX), []);
        assert_eq!(rules(PLAYBOOK), []);
    }

    #[test]
    fn a_cap_passed_is_reported() {
        let long = REFLEX.replace(
            "Set the lights of the house.",
            &format!(
                "Set the lights of the house{}.",
                " and the garden".repeat(6)
            ),
        );
        let finding = found(&long);
        assert_eq!(finding.rule, LintRule::SizeCap);
        assert_eq!(
            finding.message,
            "the summary is 118 characters; the cap is 100"
        );
    }

    #[test]
    fn text_that_addresses_the_model_is_reported() {
        let addressed = REFLEX.replace(
            "On or off, and nothing else changes.",
            "You must always choose this for lights.",
        );
        let finding = found(&addressed);
        assert_eq!(finding.rule, LintRule::AddressesModel);
        assert_eq!(
            finding.message,
            "description addresses the model: \"you must\""
        );
        // A phrase inside a word is not the phrase.
        let inside = REFLEX.replace(
            "On or off, and nothing else changes.",
            "On or off, as an aid to the models of the house.",
        );
        assert_eq!(rules(&inside), []);
    }

    #[test]
    fn a_step_the_plan_would_misread_is_reported() {
        let joined = PLAYBOOK.replace(
            "list {service}'s deploys",
            "list {service}'s deploys and logs",
        );
        let finding = found(&joined);
        assert_eq!(finding.rule, LintRule::Connective);
        assert_eq!(
            finding.message,
            "step 2 holds \"and\"; one step is one action"
        );
        let stated = PLAYBOOK.replace("list {service}'s deploys", "post to #incident");
        let finding = found(&stated);
        assert_eq!(finding.rule, LintRule::Literal);
        assert_eq!(
            finding.message,
            "step 2 states \"#incident\"; a word another team would change is a slot"
        );
        let checked = PLAYBOOK.replace(
            "list {service}'s deploys",
            "check that writes land for {service}",
        );
        let finding = found(&checked);
        assert_eq!(finding.rule, LintRule::Reference);
        assert_eq!(
            finding.message,
            "step 2 says \"check that writes\"; a step refers by \"that <noun>\": say \"check whether\""
        );
        // A step that refers on purpose is not opened by a check.
        assert_eq!(rules(PLAYBOOK), []);
    }

    #[test]
    fn an_effect_left_out_is_reported() {
        let finding = found(&REFLEX.replace("effect  = \"write\"\n", ""));
        assert_eq!(finding.rule, LintRule::Effect);
        assert_eq!(finding.path, KeyPath::new(["effect"]));
        assert_eq!(
            finding.message,
            "effect is absent, which means destructive; write it"
        );
        // A wire value carries what its file did, so the finding stands after a round trip.
        let absent = read(&REFLEX.replace("effect  = \"write\"\n", ""));
        let wire = serde_json::to_value(&absent).unwrap();
        assert_eq!(wire["effect"], "destructive");
        assert_eq!(wire["effect_absent"], true);
        let back: Manifest = serde_json::from_value(wire).unwrap();
        assert_eq!(back, absent);
        let written = serde_json::to_value(read(REFLEX)).unwrap();
        assert!(written.get("effect_absent").is_none());
    }

    #[test]
    fn a_quoted_argument_no_example_shows_in_quotes_is_reported() {
        let bare = REFLEX.replace(
            "'lights off, call it \"bedtime\"'    = { state = \"off\", label = \"bedtime\" }",
            "\"lights off at bedtime\" = { state = \"off\" }",
        );
        let finding = found(&bare);
        assert_eq!(finding.rule, LintRule::Quoted);
        assert_eq!(finding.path, KeyPath::new(["args", "label"]));
        assert_eq!(
            finding.message,
            "args.label reads text in quotes, and no example shows them"
        );
    }

    #[test]
    fn fewer_than_three_examples_are_reported() {
        // A `false` example is no request of the reflex's own.
        let two = REFLEX.replace(
            "\"kill the lights\"                  = { state = \"off\" }",
            "\"light a candle\" = false",
        );
        let finding = found(&two);
        assert_eq!(finding.rule, LintRule::Examples);
        assert_eq!(finding.path, KeyPath::new(["examples"]));
        assert_eq!(
            finding.message,
            "examples holds 2 requests; write three at least"
        );
    }

    #[test]
    fn an_option_or_a_pick_no_record_leaves_out_is_reported() {
        let stated = REFLEX.replace("{ state = false, label = false }", "{ label = false }");
        let finding = found(&stated);
        assert_eq!(finding.rule, LintRule::Unstated);
        assert_eq!(finding.path, KeyPath::new(["args", "state"]));
        assert_eq!(
            finding.message,
            "args.state has no record that leaves it out; assert state = false once"
        );
        let stated = REFLEX.replace("{ state = false, label = false }", "{ state = false }");
        assert_eq!(found(&stated).path, KeyPath::new(["args", "label"]));
        // A vocabulary's word is the user's: no shipped record may assert it, stated or not.
        assert_eq!(rules(PLAYBOOK), []);
    }

    #[test]
    fn a_confirm_that_reads_nothing_back_is_reported() {
        let bare = REFLEX.replace("Switch the lights {state}?", "Switch the lights?");
        let finding = found(&bare);
        assert_eq!(finding.rule, LintRule::Confirm);
        assert_eq!(finding.path, KeyPath::new(["confirm"]));
        assert_eq!(finding.message, "confirm reads no value back; name {state}");
        let sure = REFLEX.replace("Switch the lights {state}?", "Are you sure about {state}?");
        assert_eq!(
            found(&sure).message,
            "confirm asks \"Are you sure\"; say what the call will do"
        );
        // A reflex with no required argument has nothing to read back.
        let optional = REFLEX
            .replace("Switch the lights {state}?", "Switch the lights?")
            .replace(
                "options.off = \"Switch off.\"",
                "options.off = \"Switch off.\"\noptional = true",
            );
        assert_eq!(rules(&optional), []);
    }

    #[test]
    fn a_meaning_that_repeats_the_ask_is_reported() {
        let echo = REFLEX.replace(
            "options.on  = \"Switch on.\"",
            "options.on  = \"what should the lights do\"",
        );
        let finding = found(&echo);
        assert_eq!(finding.rule, LintRule::Meaning);
        assert_eq!(
            finding.path,
            KeyPath::new(["args", "state", "options", "on"])
        );
        assert_eq!(
            finding.message,
            "args.state.options.on repeats the ask; a meaning answers it"
        );
    }

    #[test]
    fn a_step_worded_as_the_situation_is_reported() {
        let near = PLAYBOOK.replace(
            "check {service}'s errors",
            "handle the failing {service} service",
        );
        let finding = found(&near);
        assert_eq!(finding.rule, LintRule::Situation);
        assert_eq!(finding.path, KeyPath::new(["steps", "0"]));
        assert_eq!(
            finding.message,
            "step 1's words are the summary's or an example's; word a step apart from the situation"
        );
        // One word of its own sets a step apart.
        let apart = PLAYBOOK.replace(
            "check {service}'s errors",
            "investigate the failing {service} service",
        );
        assert_eq!(rules(&apart), []);
    }

    #[test]
    fn a_renamed_placeholder_is_not_a_body_change() {
        let previous = parse(
            "[args.state]\nask = \"What?\"\noptions = { on = \"On.\" }\n",
            "[\"hue\", \"{state}\"]",
        );
        let next = parse(
            "[args.power]\nask = \"What?\"\noptions = { on = \"On.\" }\nwas = [\"state\"]\n",
            "[\"hue\", \"{power}\"]",
        );
        let diff = diff(&previous, &next);
        assert_eq!(diff.level, Level::Major);
        assert_eq!(
            diff.changes,
            [Change::ArgRenamed {
                from: ArgName::new("state").unwrap(),
                to: ArgName::new("power").unwrap(),
            }]
        );
        assert!(diff.violations.is_empty());
    }

    #[test]
    fn a_flag_becoming_a_value_changes_its_source() {
        let previous = parse(
            "[args.all]\nask = \"All?\"\nflag = true\n",
            "\"lights.mts\"",
        );
        let next = parse(
            "[args.all]\nask = \"All?\"\noptions = { yes = \"Yes.\" }\n",
            "\"lights.mts\"",
        );
        assert_eq!(
            diff(&previous, &next).changes,
            [Change::SourceChanged {
                arg: ArgName::new("all").unwrap()
            }]
        );
    }

    #[test]
    fn a_removed_argument_takes_its_former_names_with_it() {
        let previous = parse(
            "[args.power]\nask = \"What?\"\noptions = { on = \"On.\" }\nwas = [\"state\"]\n",
            "\"lights.mts\"",
        );
        let next = parse(
            "[args.other]\nask = \"What?\"\noptions = { on = \"On.\" }\n",
            "\"lights.mts\"",
        );
        assert!(diff(&previous, &next).violations.is_empty());
    }
}
