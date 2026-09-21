//! The contract between two versions of a reflex: what a body receives and a call may name; and lint, what a
//! manifest may say to an engine. In: the previous and the next `Manifest`; the locked and the upstream `Effect`; a
//! `Manifest`. Out: `ContractDiff` — its level, every change, every `was` violation — `Consent`, and every
//! `Finding`. Wording is never a change: a description, an ask or an option's meaning may move freely.

use std::cmp::Ordering;

use indexmap::IndexMap;
use serde::Serialize;

use crate::document::KeyPath;
use crate::manifest::{Effect, Element, Kind, Manifest, Run, Source, renames};
use crate::name::{ArgName, ConfigKey, OptionKey};

/// What changed in the contract from one version to the next, and how much it matters.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContractDiff {
    pub level: Level,
    pub changes: Vec<Change>,
    pub violations: Vec<WasViolation>,
}

/// `Same`: nothing but wording. `Minor`: additions, and a config key gone, which leaves a setting orphaned and skipped.
/// `Major`: something a person's files or calls may not survive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    Same,
    Minor,
    Major,
}

/// One change to the contract, in the order the diff walks: the previous arguments, the added ones, the body, config.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Change {
    ArgRemoved { arg: ArgName },
    OptionRemoved { arg: ArgName, key: OptionKey },
    ArgRenamed { from: ArgName, to: ArgName },
    SourceChanged { arg: ArgName },
    RangeChanged { arg: ArgName },
    RunChanged,
    ArgAdded { arg: ArgName },
    OptionAdded { arg: ArgName, key: OptionKey },
    ConfigAdded { key: ConfigKey },
    ConfigRemoved { key: ConfigKey },
}

impl Change {
    /// Whether the change can break what a person wrote: an overlay, a call, an example. Additions cannot; neither can
    /// a config key gone.
    fn breaks(&self) -> bool {
        match self {
            Self::ArgRemoved { .. }
            | Self::OptionRemoved { .. }
            | Self::ArgRenamed { .. }
            | Self::SourceChanged { .. }
            | Self::RangeChanged { .. }
            | Self::RunChanged => true,
            Self::ArgAdded { .. }
            | Self::OptionAdded { .. }
            | Self::ConfigAdded { .. }
            | Self::ConfigRemoved { .. } => false,
        }
    }
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

/// The contract diff: every argument of `previous` followed through `was`, then what `next` adds, the body, config.
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
    }
    for name in next.args.keys() {
        if !previous.args.contains_key(name) && !renamed.values().any(|to| to == name) {
            changes.push(Change::ArgAdded { arg: name.clone() });
        }
    }
    if followed(&previous.run, &renamed) != next.run {
        changes.push(Change::RunChanged);
    }
    for key in previous.config.keys() {
        if !next.config.contains_key(key) {
            changes.push(Change::ConfigRemoved { key: key.clone() });
        }
    }
    for key in next.config.keys() {
        if !previous.config.contains_key(key) {
            changes.push(Change::ConfigAdded { key: key.clone() });
        }
    }
    let level = if changes.is_empty() {
        Level::Same
    } else if changes.iter().any(Change::breaks) {
        Level::Major
    } else {
        Level::Minor
    };
    ContractDiff {
        level,
        changes,
        violations: violations(previous, next, &renamed),
    }
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

/// What `lint` finds: a size cap passed, or text that addresses the model instead of describing an action.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LintRule {
    SizeCap,
    AddressesModel,
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

/// Phrases that address a model rather than describe an action, matched case-insensitively; the first found names
/// the finding.
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

/// Lint: every cap passed and every string that addresses the model, in manifest order — the description,
/// `not_for`, each argument's ask and options, then the examples and the tests.
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
    for (name, arg) in &m.args {
        let path = KeyPath::new(["args", name.as_str()]);
        addresses(&mut findings, &path.child("ask"), arg.ask.as_str());
        if let Kind::Value {
            source: Source::Options(options),
            ..
        } = &arg.kind
        {
            let at = path.child("options");
            cap_entries(
                &mut findings,
                &at,
                &format!("args.{name}"),
                "options",
                options.len(),
                OPTIONS,
            );
            for (key, meaning) in options.iter() {
                addresses(&mut findings, &at.child(key.as_str()), meaning.as_str());
            }
        }
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

fn addresses(findings: &mut Vec<Finding>, path: &KeyPath, text: &str) {
    let lower = text.to_lowercase();
    if let Some(phrase) = ADDRESSES_MODEL
        .iter()
        .find(|phrase| lower.contains(*phrase))
    {
        findings.push(Finding {
            rule: LintRule::AddressesModel,
            path: path.clone(),
            message: format!("{path} addresses the model: \"{phrase}\""),
        });
    }
}

/// The changes to one argument that survives, under its current name.
fn compare(name: &ArgName, before: &Kind, after: &Kind) -> Vec<Change> {
    let (before, after) = match (before, after) {
        (Kind::Flag, Kind::Flag) => return Vec::new(),
        (Kind::Value { source: a, .. }, Kind::Value { source: b, .. }) => (a, b),
        _ => return vec![Change::SourceChanged { arg: name.clone() }],
    };
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
