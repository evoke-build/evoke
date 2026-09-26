//! An overlay read against its manifest, `shipped ⊕ yours`, and what an update means for your file. In: a `Document`
//! and the `Manifest` it words; two manifests. Out: `Overlay`, `Effective` with what is yours, `Report`; or diagnostics.

use std::collections::BTreeSet;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::Diagnostic;
use crate::document::{self, Diagnostics, Document, Form, Json, KeyPath, Node, Value};
use crate::manifest::{
    self, Argument, Description, Effect, Kind, Lookup, Manifest, Owner, Record, Records, Source,
    Template,
};
use crate::name::{ArgName, OptionKey, Tag};
use crate::text::{Clean, Identity};

/// Your wording for one reflex, its names already resolved through `was`. Read, never built.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Overlay {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Description>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_for: Option<Vec<Clean>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<Tag>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<Effect>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confirm: Option<Template>,
    pub args: IndexMap<ArgName, Wording>,
    pub examples: Records,
    pub tests: Records,
    /// Former names the file used, with the name each resolved to.
    pub renamed: Vec<(ArgName, ArgName)>,
    /// Keys that address nothing: skipped, the rest applies.
    pub orphaned: Vec<KeyPath>,
}

/// New wording for an argument: its question, and existing option keys.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Wording {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<Clean>,
    pub options: IndexMap<OptionKey, Clean>,
}

/// `shipped ⊕ yours`, with the key paths your file decided: what `show` marks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Effective {
    pub manifest: Manifest,
    pub yours: BTreeSet<KeyPath>,
}

/// What an update means for your file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub renamed: Vec<(ArgName, ArgName)>,
    /// You override what upstream changed: yours wins, both are shown once.
    pub stale: Vec<KeyPath>,
    pub orphaned: Vec<KeyPath>,
}

/// An overlay from its file, read against the manifest it words; a contract key is a line to fix.
pub fn overlay(doc: Document<'_>, of: &Manifest) -> Result<Overlay, Vec<Diagnostic>> {
    let mut d = Diagnostics::new(Some(&doc.file));
    let form = doc.text.form();
    let value = read(&mut d, document::root(doc)?, of, form);
    d.finish(value)
}

impl Overlay {
    /// An overlay from its wire form, typed against the manifest it words: the one door a JSON value has.
    pub fn from_json(json: &Json, of: &Manifest) -> Result<Self, Vec<Diagnostic>> {
        let mut d = Diagnostics::new(None);
        let value = read(&mut d, document::wire(json), of, Form::Wire);
        d.finish(value)
    }
}

/// The merged manifest: your values replace whole, tables merge by key, the highest layer naming an utterance decides.
#[must_use]
pub fn effective(shipped: &Manifest, yours: Option<&Overlay>) -> Effective {
    let mut manifest = shipped.clone();
    let mut paths = BTreeSet::new();
    let Some(yours) = yours else {
        return Effective {
            manifest,
            yours: paths,
        };
    };
    let mut set = |key: &str| paths.insert(KeyPath::new([key]));
    if let Some(description) = &yours.description {
        manifest.description = description.clone();
        set("description");
    }
    if let Some(not_for) = &yours.not_for {
        manifest.not_for.clone_from(not_for);
        set("not_for");
    }
    if let Some(tags) = &yours.tags {
        manifest.tags.clone_from(tags);
        set("tags");
    }
    if let Some(effect) = yours.effect {
        manifest.effect = effect;
        set("effect");
    }
    if let Some(confirm) = &yours.confirm {
        manifest.confirm = confirm.clone();
        set("confirm");
    }
    for (name, wording) in &yours.args {
        let Some(arg) = manifest.args.get_mut(name) else {
            continue;
        };
        let path = KeyPath::new(["args", name.as_str()]);
        if let Some(ask) = &wording.ask {
            arg.ask = ask.clone();
            paths.insert(path.child("ask"));
        }
        if let Kind::Value {
            source: Source::Options(options),
            ..
        } = &mut arg.kind
        {
            for (key, text) in &wording.options {
                if options.reword(key, text.clone()) {
                    paths.insert(path.child("options").child(key.as_str()));
                }
            }
        }
    }
    let named = |id: &Identity| yours.examples.contains(id) || yours.tests.contains(id);
    manifest.examples.retain(|id, _| !named(id));
    manifest.tests.retain(|id, _| !named(id));
    merge(
        &mut manifest.examples,
        &yours.examples,
        "examples",
        &mut paths,
    );
    merge(&mut manifest.tests, &yours.tests, "tests", &mut paths);
    Effective {
        manifest,
        yours: paths,
    }
}

fn merge(into: &mut Records, from: &Records, table: &str, paths: &mut BTreeSet<KeyPath>) {
    for (_, (utterance, record)) in from.iter() {
        paths.insert(KeyPath::new([table, utterance.as_str()]));
        into.insert(utterance.clone(), record.clone());
    }
}

/// What an update from `previous` to `next` means for your overlay, read against `next`.
#[must_use]
pub fn report(previous: &Manifest, next: &Manifest, yours: Option<&Overlay>) -> Report {
    let Some(yours) = yours else {
        return Report::default();
    };
    let mut stale = BTreeSet::new();
    let changed = [
        (
            "description",
            yours.description.is_some() && previous.description != next.description,
        ),
        (
            "not_for",
            yours.not_for.is_some() && previous.not_for != next.not_for,
        ),
        ("tags", yours.tags.is_some() && previous.tags != next.tags),
        (
            "effect",
            yours.effect.is_some() && previous.effect != next.effect,
        ),
        (
            "confirm",
            yours.confirm.is_some() && previous.confirm != next.confirm,
        ),
    ];
    stale.extend(
        changed
            .iter()
            .filter(|(_, changed)| *changed)
            .map(|(key, _)| KeyPath::new([*key])),
    );
    for (name, wording) in &yours.args {
        let path = KeyPath::new(["args", name.as_str()]);
        let now = next.args.get(name);
        let before = before(previous, next, name);
        if wording.ask.is_some() && before.map(|arg| &arg.ask) != now.map(|arg| &arg.ask) {
            stale.insert(path.child("ask"));
        }
        for key in wording.options.keys() {
            if option(before, key) != option(now, key) {
                stale.insert(path.child("options").child(key.as_str()));
            }
        }
    }
    let renames = manifest::renames(previous, next);
    for (table, records) in [("examples", &yours.examples), ("tests", &yours.tests)] {
        for (id, (utterance, _)) in records.iter() {
            let before = (
                record(&previous.examples, id, &renames),
                record(&previous.tests, id, &renames),
            );
            let now = (
                record(&next.examples, id, &IndexMap::new()),
                record(&next.tests, id, &IndexMap::new()),
            );
            if before != now {
                stale.insert(KeyPath::new([table, utterance.as_str()]));
            }
        }
    }
    Report {
        renamed: yours.renamed.clone(),
        stale: stale.into_iter().collect(),
        orphaned: yours.orphaned.clone(),
    }
}

/// The argument `name` of `next` as `previous` had it, under its name then.
fn before<'a>(previous: &'a Manifest, next: &Manifest, name: &ArgName) -> Option<&'a Argument> {
    previous.args.get(name).or_else(|| {
        next.args
            .get(name)?
            .was
            .iter()
            .find_map(|old| previous.args.get(old))
    })
}

fn option<'a>(arg: Option<&'a Argument>, key: &OptionKey) -> Option<&'a Clean> {
    match &arg?.kind {
        Kind::Value {
            source: Source::Options(options),
            ..
        } => options.get(key),
        _ => None,
    }
}

fn record(
    records: &Records,
    id: &Identity,
    renames: &IndexMap<ArgName, ArgName>,
) -> Option<(Clean, Record)> {
    records
        .get(id)
        .map(|(utterance, record)| (utterance.clone(), record.renamed(renames)))
}

/// Resolves the names a file uses through `was`, remembering what it found.
struct Resolver<'a> {
    of: &'a Manifest,
    former: IndexMap<ArgName, ArgName>,
    renamed: Vec<(ArgName, ArgName)>,
    orphaned: Vec<KeyPath>,
}

impl<'a> Resolver<'a> {
    fn new(of: &'a Manifest) -> Self {
        let former = of
            .args
            .iter()
            .flat_map(|(name, arg)| arg.was.iter().map(move |old| (old.clone(), name.clone())))
            .collect();
        Self {
            of,
            former,
            renamed: Vec::new(),
            orphaned: Vec::new(),
        }
    }

    /// The current argument a written name reaches, if any.
    fn resolve(&mut self, written: &str) -> Option<(&'a ArgName, &'a Argument)> {
        if let Some(found) = self.of.args.get_key_value(written) {
            return Some(found);
        }
        let (old, current) = self.former.get_key_value(written)?;
        if !self.renamed.iter().any(|(o, _)| o == old) {
            self.renamed.push((old.clone(), current.clone()));
        }
        self.of.args.get_key_value(current.as_str())
    }

    fn lookup(&mut self, name: &ArgName) -> Lookup<'a> {
        self.resolve(name.as_str())
            .map_or(Lookup::Unknown, |(current, arg)| Lookup::Arg(current, arg))
    }

    fn orphan(&mut self, path: KeyPath) {
        if !self.orphaned.contains(&path) {
            self.orphaned.push(path);
        }
    }

    /// What an earlier read found, carried on the wire: seeds the lists, so a second read adds nothing twice.
    fn carried(&mut self, d: &mut Diagnostics, renamed: Option<Node>, orphaned: Option<Node>) {
        if let Some(node) = renamed {
            match serde_json::from_value(node.json()) {
                Ok(renamed) => self.renamed = renamed,
                Err(_) => d.fail(None, "renamed must be a list of [former, current] names"),
            }
        }
        if let Some(node) = orphaned {
            match serde_json::from_value(node.json()) {
                Ok(orphaned) => self.orphaned = orphaned,
                Err(_) => d.fail(None, "orphaned must be a list of key paths"),
            }
        }
    }
}

fn read(d: &mut Diagnostics, root: Node, of: &Manifest, form: Form) -> Option<Overlay> {
    let mut top = d.table(root)?;
    let mut r = Resolver::new(of);
    let description = top
        .take("description")
        .and_then(|node| manifest::description(d, &node));
    let not_for = top
        .take("not_for")
        .map(|node| manifest::list(d, node, Diagnostics::line));
    let tags = top.take("tags").map(|node| manifest::tags(d, node));
    let effect = top.take("effect").and_then(|node| {
        let effect = manifest::effect(d, &node)?;
        if effect < of.effect {
            let message = format!(
                "effect \"{effect}\" loosens \"{}\"; an overlay may only tighten",
                of.effect
            );
            d.fail(node.at.as_ref(), message);
            return None;
        }
        Some(effect)
    });
    for (_, node) in top.take_any(&["run", "needs", "config", "yields"]) {
        d.fail(
            node.at.as_ref(),
            format!(
                "{} is contract, not wording; an overlay cannot change it",
                node.path
            ),
        );
    }
    if form == Form::Wire {
        r.carried(d, top.take("renamed"), top.take("orphaned"));
    }
    let args = top
        .take("args")
        .map_or_else(IndexMap::new, |node| args(d, node, &mut r));
    let confirm = top
        .take("confirm")
        .and_then(|node| manifest::template(d, &node, &mut |name| r.lookup(name)));
    let none = Records::default();
    let examples = top.take("examples").map_or_else(Records::default, |n| {
        records(d, n, "examples", &mut r, &none)
    });
    let tests = top.take("tests").map_or_else(Records::default, |n| {
        records(d, n, "tests", &mut r, &examples)
    });
    for path in top.unknown() {
        r.orphan(path);
    }
    Some(Overlay {
        description,
        not_for,
        tags,
        effect,
        confirm,
        args,
        examples,
        tests,
        renamed: r.renamed,
        orphaned: r.orphaned,
    })
}

fn args(d: &mut Diagnostics, node: Node, r: &mut Resolver<'_>) -> IndexMap<ArgName, Wording> {
    let mut args = IndexMap::new();
    let Some(table) = d.table(node) else {
        return args;
    };
    for (written, node) in table.entries() {
        let Some((current, arg)) = r.resolve(&written) else {
            r.orphan(node.path.clone());
            continue;
        };
        let Some(mut table) = d.table(node) else {
            continue;
        };
        let wording: &mut Wording = args.entry(current.clone()).or_default();
        if let Some(node) = table.take("ask")
            && let Some(ask) = d.line(&node)
        {
            wording.ask = Some(ask);
        }
        if let Some(node) = table.take("options") {
            options(d, node, arg, wording);
        }
        for (_, node) in table.take_any(&["pick", "vocab", "range", "flag", "optional", "was"]) {
            d.fail(
                node.at.as_ref(),
                format!(
                    "{} is contract, not wording; an overlay cannot change it",
                    node.path
                ),
            );
        }
        for path in table.unknown() {
            r.orphan(path);
        }
    }
    args
}

fn options(d: &mut Diagnostics, node: Node, arg: &Argument, wording: &mut Wording) {
    // An empty table rewords nothing, on any argument: the wire form a wording takes.
    if matches!(&node.value, Value::Table(entries) if entries.is_empty()) {
        return;
    }
    let Kind::Value {
        source: Source::Options(shipped),
        ..
    } = &arg.kind
    else {
        d.fail(
            node.at.as_ref(),
            format!(
                "{} is contract, not wording; an overlay cannot change it",
                node.path
            ),
        );
        return;
    };
    let Some(table) = d.table(node) else { return };
    for (key, node) in table.entries() {
        match shipped.get_key_value(key.as_str()) {
            Some((key, _)) => {
                if let Some(text) = d.line(&node) {
                    wording.options.insert(key.clone(), text);
                }
            }
            None => d.fail(
                node.at.as_ref(),
                format!("{} is new; an overlay rewords existing options", node.path),
            ),
        }
    }
}

fn records(
    d: &mut Diagnostics,
    node: Node,
    table: &str,
    r: &mut Resolver<'_>,
    other: &Records,
) -> Records {
    manifest::read_records(
        d,
        node,
        other,
        &mut |d, utterance, name, node| match r.resolve(name) {
            None => {
                r.orphan(KeyPath::new([table, utterance.as_str(), name]));
                None
            }
            Some((current, arg)) => {
                manifest::assertion(d, utterance, name, arg, node, Owner::Yours)
                    .map(|a| (current.clone(), a))
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::File;
    use crate::document::Text;
    use crate::name::LocalName;

    #[test]
    fn a_confirm_follows_a_rename() {
        let name = LocalName::new("lights").unwrap();
        let shipped = manifest::manifest(Document {
            file: File::Manifest { name: name.clone() },
            text: Text::Toml(
                "reflex = 1\ndescription = \"Lights.\"\nconfirm = \"Set {power}?\"\nrun = \"lights.mts\"\n\n[args.power]\nask = \"On or off?\"\noptions = { on = \"On.\", off = \"Off.\" }\nwas = [\"state\"]\n",
            ),
        })
        .unwrap();
        let yours = overlay(
            Document {
                file: File::Overlay { name },
                text: Text::Toml("confirm = \"Lights {state}?\"\n"),
            },
            &shipped,
        )
        .unwrap();
        assert_eq!(
            yours.confirm.as_ref().map(Template::to_string).as_deref(),
            Some("Lights {power}?")
        );
        let renamed: Vec<(&str, &str)> = yours
            .renamed
            .iter()
            .map(|(old, new)| (old.as_str(), new.as_str()))
            .collect();
        assert_eq!(renamed, [("state", "power")]);
    }
}
