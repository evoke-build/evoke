//! `reflex.toml` as a type. In: a `Document`, TOML or JSON. Out: a `Manifest` that holds every structural rule,
//! its records typed against the arguments they name; or the diagnostics, each on its line.

use std::fmt;

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::diagnostic::{At, Diagnostic};
use crate::document::{self, Diagnostics, Document, Form, Json, KeyPath, Node, Value};
use crate::name::{ArgName, ConfigKey, OptionKey, RelPath, Tag, VocabName, Word};
use crate::text::{Clean, Identity, identity};

/// A reflex's manifest, normalized: `effect` explicit, every table present, records typed. Read, never built.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Manifest {
    pub description: Description,
    pub not_for: Vec<Clean>,
    pub tags: Vec<Tag>,
    pub effect: Effect,
    pub confirm: Template,
    #[serde(skip_serializing_if = "Run::is_inline")]
    pub run: Run,
    pub config: IndexMap<ConfigKey, ConfigSpec>,
    pub args: IndexMap<ArgName, Argument>,
    pub examples: Records,
    pub tests: Records,
    /// Keys the format does not know: reported, never fatal.
    pub unknown: Vec<KeyPath>,
}

/// What the reflex does: a summary line, never empty, and the rest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Description {
    summary: Clean,
    rest: Clean,
}

impl Description {
    /// Splits at the first line feed; `None` when the first line is blank.
    #[must_use]
    pub fn new(text: &Clean) -> Option<Self> {
        let (summary, rest) = text
            .as_str()
            .split_once('\n')
            .unwrap_or((text.as_str(), ""));
        if summary.trim().is_empty() {
            return None;
        }
        // Proven: both are pieces of a clean string.
        Some(Self {
            summary: Clean::new(summary).ok()?,
            rest: Clean::new(rest).ok()?,
        })
    }

    #[must_use]
    pub fn summary(&self) -> &Clean {
        &self.summary
    }

    #[must_use]
    pub fn rest(&self) -> &Clean {
        &self.rest
    }
}

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.rest.as_str().is_empty() {
            f.write_str(self.summary.as_str())
        } else {
            write!(f, "{}\n{}", self.summary, self.rest)
        }
    }
}

impl Serialize for Description {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// What running the reflex does to the world; greater is tighter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    Read,
    Write,
    Destructive,
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Destructive => "destructive",
        })
    }
}

/// The confirm prompt: text with `{placeholder}`s, each naming a required, non-flag argument.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Template(Vec<Piece>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(Clean),
    Arg(ArgName),
}

impl Template {
    #[must_use]
    pub fn new(pieces: Vec<Piece>) -> Self {
        Self(pieces)
    }

    /// Reads the braces of one non-empty line; the error is a fragment to follow the field's name.
    pub fn parse(text: &str) -> Result<Self, String> {
        Clean::line(text)?;
        let mut pieces = Vec::new();
        let mut plain = String::new();
        let mut rest = text;
        while let Some(i) = rest.find(['{', '}']) {
            plain.push_str(&rest[..i]);
            let after = &rest[i + 1..];
            if rest[i..].starts_with('}') {
                return Err("has a stray \"}\"".to_owned());
            }
            let Some(j) = after.find('}').filter(|&j| !after[..j].contains('{')) else {
                return Err("has an unmatched \"{\"".to_owned());
            };
            let name = ArgName::new(&after[..j]).map_err(|_| {
                format!(
                    "has a placeholder {{{}}} that is not an argument name",
                    &after[..j]
                )
            })?;
            if !plain.is_empty() {
                pieces.push(Piece::Text(
                    Clean::new(&plain).map_err(|why| why.to_string())?,
                ));
                plain.clear();
            }
            pieces.push(Piece::Arg(name));
            rest = &after[j + 1..];
        }
        plain.push_str(rest);
        if !plain.is_empty() {
            pieces.push(Piece::Text(
                Clean::new(&plain).map_err(|why| why.to_string())?,
            ));
        }
        Ok(Self(pieces))
    }

    #[must_use]
    pub fn pieces(&self) -> &[Piece] {
        &self.0
    }

    pub fn placeholders(&self) -> impl Iterator<Item = &ArgName> {
        self.0.iter().filter_map(|piece| match piece {
            Piece::Arg(name) => Some(name),
            Piece::Text(_) => None,
        })
    }
}

impl TryFrom<String> for Template {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text).map_err(|why| format!("the template {why}"))
    }
}

impl From<Template> for String {
    fn from(template: Template) -> Self {
        template.to_string()
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for piece in &self.0 {
            match piece {
                Piece::Text(text) => f.write_str(text.as_str())?,
                Piece::Arg(name) => write!(f, "{{{name}}}")?,
            }
        }
        Ok(())
    }
}

/// The body: a function the SDK holds, an entrypoint run in a child, or an argv that never touches a shell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Run {
    Inline,
    File(Entrypoint),
    Argv { program: Clean, rest: Vec<Element> },
}

impl Run {
    #[must_use]
    pub fn is_inline(&self) -> bool {
        matches!(self, Self::Inline)
    }
}

impl Serialize for Run {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Inline => serializer.serialize_unit(),
            Self::File(entrypoint) => serializer.collect_str(entrypoint.path()),
            Self::Argv { program, rest } => serializer.collect_seq(
                std::iter::once(program.to_string()).chain(rest.iter().map(Element::to_string)),
            ),
        }
    }
}

/// A path inside the reflex directory ending in `.mts` or `.mjs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entrypoint(RelPath);

impl Entrypoint {
    pub fn new(path: &str) -> Result<Self, String> {
        let path = RelPath::new(path)?;
        let file = path.as_str().rsplit('/').next().unwrap_or_default();
        if matches!(file.rsplit_once('.'), Some((stem, "mts" | "mjs")) if !stem.is_empty()) {
            Ok(Self(path))
        } else {
            Err(format!("\"{path}\" must end in .mts or .mjs"))
        }
    }

    #[must_use]
    pub fn path(&self) -> &RelPath {
        &self.0
    }
}

/// One argv element after the program: passed as written, or an argument's value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Element {
    Literal(Clean),
    Arg(ArgName),
}

impl fmt::Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(text) => f.write_str(text.as_str()),
            Self::Arg(name) => write!(f, "{{{name}}}"),
        }
    }
}

/// A setting the user provides; a secret is set only from an environment variable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigSpec {
    pub about: Clean,
    pub secret: bool,
}

/// An argument: its question, where its values come from, and its former names. Read, never built.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Argument {
    pub ask: Clean,
    pub kind: Kind,
    /// Flat and cumulative; a retired name never returns.
    pub was: Vec<ArgName>,
}

/// A flag, or a value with exactly one source; a flag is optional by nature.
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Flag,
    Value { source: Source, optional: bool },
}

/// Where an argument's values come from: the author, the user or the input.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Options(Options),
    Vocab(VocabName),
    Pick(Pick),
}

/// The author's closed set, never empty: key = what the body receives, value = what it means.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options(IndexMap<OptionKey, Clean>);

impl Options {
    #[must_use]
    pub fn new(options: IndexMap<OptionKey, Clean>) -> Option<Self> {
        if options.is_empty() {
            None
        } else {
            Some(Self(options))
        }
    }

    /// New wording for an existing key; `false` when the key is not one.
    pub fn reword(&mut self, key: &OptionKey, text: Clean) -> bool {
        match self.0.get_mut(key) {
            Some(meaning) => {
                *meaning = text;
                true
            }
            None => false,
        }
    }
}

impl std::ops::Deref for Options {
    type Target = IndexMap<OptionKey, Clean>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A built-in recognizer over the input; a range only where a number exists.
#[derive(Clone, Debug, PartialEq)]
pub enum Pick {
    Number(Option<Range<f64>>),
    Duration(Option<Range<u64>>),
    Email,
    Url,
    Quoted,
}

impl Pick {
    /// Which recognizer proposes the candidates; the range, when there is one, compares them.
    #[must_use]
    pub fn recognizer(&self) -> Recognizer {
        match self {
            Self::Number(_) => Recognizer::Number,
            Self::Duration(_) => Recognizer::Duration,
            Self::Email => Recognizer::Email,
            Self::Url => Recognizer::Url,
            Self::Quoted => Recognizer::Quoted,
        }
    }
}

/// One of the five recognizers, by the name a manifest's `pick` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Recognizer {
    Number,
    Duration,
    Email,
    Url,
    Quoted,
}

impl Recognizer {
    /// The name a manifest's `pick` writes.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Duration => "duration",
            Self::Email => "email",
            Self::Url => "url",
            Self::Quoted => "quoted",
        }
    }

    /// What the pick wants, for `"soon" is not a duration`.
    #[must_use]
    pub fn wants(self) -> &'static str {
        match self {
            Self::Number => "a number",
            Self::Duration => "a duration",
            Self::Email => "an email address",
            Self::Url => "a URL",
            Self::Quoted => "text",
        }
    }
}

/// `[min, max]` on a value, `min ≤ max`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Range<T> {
    min: T,
    max: T,
}

impl<T: PartialOrd + Copy> Range<T> {
    #[must_use]
    pub fn new(min: T, max: T) -> Option<Self> {
        if min <= max {
            Some(Self { min, max })
        } else {
            None
        }
    }

    pub fn min(&self) -> T {
        self.min
    }

    pub fn max(&self) -> T {
        self.max
    }
}

impl<T: Serialize> Serialize for Range<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (&self.min, &self.max).serialize(serializer)
    }
}

impl<'de, T: PartialOrd + Copy + Deserialize<'de>> Deserialize<'de> for Range<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (min, max) = <(T, T)>::deserialize(deserializer)?;
        Self::new(min, max).ok_or_else(|| D::Error::custom("a range has min above max"))
    }
}

impl Serialize for Argument {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("ask", &self.ask)?;
        match &self.kind {
            Kind::Flag => map.serialize_entry("flag", &true)?,
            Kind::Value { source, optional } => {
                match source {
                    Source::Options(options) => map.serialize_entry("options", &**options)?,
                    Source::Vocab(name) => map.serialize_entry("vocab", name)?,
                    Source::Pick(pick) => {
                        map.serialize_entry("pick", &pick.recognizer())?;
                        match pick {
                            Pick::Number(Some(range)) => map.serialize_entry("range", range)?,
                            Pick::Duration(Some(range)) => map.serialize_entry("range", range)?,
                            _ => {}
                        }
                    }
                }
                map.serialize_entry("optional", optional)?;
            }
        }
        map.serialize_entry("was", &self.was)?;
        map.end()
    }
}

/// Utterances keyed by identity, each with its spelling and what it asserts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Records(IndexMap<Identity, (Clean, Record)>);

impl Records {
    /// Keys by the utterance's identity; a record already under it is replaced and returned.
    pub fn insert(&mut self, utterance: Clean, record: Record) -> Option<(Clean, Record)> {
        self.0
            .insert(identity(utterance.as_str()), (utterance, record))
    }

    #[must_use]
    pub fn get(&self, id: &Identity) -> Option<&(Clean, Record)> {
        self.0.get(id)
    }

    #[must_use]
    pub fn contains(&self, id: &Identity) -> bool {
        self.0.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Identity, &(Clean, Record))> {
        self.0.iter()
    }

    pub fn retain(&mut self, keep: impl FnMut(&Identity, &mut (Clean, Record)) -> bool) {
        self.0.retain(keep);
    }
}

impl Serialize for Records {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_map(
            self.0
                .values()
                .map(|(utterance, record)| (utterance, record)),
        )
    }
}

/// `false`, or the asserted arguments; `{}` asserts the route alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Record {
    Never,
    Asserts(IndexMap<ArgName, Assertion>),
}

impl Record {
    /// The same record with its argument names mapped, for a comparison across a rename.
    #[must_use]
    pub fn renamed(&self, names: &IndexMap<ArgName, ArgName>) -> Self {
        match self {
            Self::Never => Self::Never,
            Self::Asserts(asserts) => Self::Asserts(
                asserts
                    .iter()
                    .map(|(name, a)| (names.get(name).unwrap_or(name).clone(), a.clone()))
                    .collect(),
            ),
        }
    }
}

/// Names `previous` used for arguments `next` renamed: each old name to its current one, through `was`.
pub(crate) fn renames(previous: &Manifest, next: &Manifest) -> IndexMap<ArgName, ArgName> {
    let mut renames = IndexMap::new();
    for (name, arg) in &next.args {
        for old in arg
            .was
            .iter()
            .filter(|old| previous.args.contains_key(*old))
        {
            renames.insert(old.clone(), name.clone());
        }
    }
    renames
}

impl Serialize for Record {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Never => serializer.serialize_bool(false),
            Self::Asserts(asserts) => asserts.serialize(serializer),
        }
    }
}

/// What a record says about one argument, typed by that argument's source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Assertion {
    Unstated,
    Option(OptionKey),
    Word(Word),
    Span(Clean),
    Flag,
}

impl Serialize for Assertion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Unstated => serializer.serialize_bool(false),
            Self::Flag => serializer.serialize_bool(true),
            Self::Option(key) => key.serialize(serializer),
            Self::Word(word) => word.serialize(serializer),
            Self::Span(text) => text.serialize(serializer),
        }
    }
}

/// A manifest from its file, or every line to fix.
pub fn manifest(doc: Document<'_>) -> Result<Manifest, Vec<Diagnostic>> {
    let mut d = Diagnostics::new(Some(&doc.file));
    let form = doc.text.form();
    let value = read(&mut d, document::root(doc)?, form);
    d.finish(value)
}

impl<'de> Deserialize<'de> for Manifest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let json = Json::deserialize(deserializer)?;
        let mut d = Diagnostics::new(None);
        let value = read(&mut d, document::wire(&json), Form::Wire);
        d.finish(value)
            .map_err(|errors| D::Error::custom(document::summary(&errors)))
    }
}

/// Who wrote a file: a shipped manifest may not assert vocabulary arguments; your own files may, and a wire value
/// was typed at its file already.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Owner {
    Shipped,
    Yours,
}

/// What a placeholder's name resolves to: the current name and its argument, one that failed to read, or nothing.
pub(crate) enum Lookup<'a> {
    Unknown,
    Broken,
    Arg(&'a ArgName, &'a Argument),
}

/// Arguments by name; `None` for one that failed to read but still counts as named.
pub(crate) type Known = IndexMap<ArgName, Option<Argument>>;

fn read(d: &mut Diagnostics, root: Node, form: Form) -> Option<Manifest> {
    let mut top = d.table(root)?;
    let root_at = top.at.clone();
    let order = top.keys();
    if form != Form::Wire {
        match top.take("reflex") {
            Some(node) if node.integer() == Some(1) => {}
            Some(node) => d.fail(node.at.as_ref(), "reflex = 1 is required"),
            None => d.fail(root_at.as_ref(), "reflex = 1 is required"),
        }
    }
    let description = if let Some(node) = top.take("description") {
        description(d, &node)
    } else {
        d.fail(root_at.as_ref(), "description is required");
        None
    };
    let not_for = top
        .take("not_for")
        .map_or_else(Vec::new, |node| list(d, node, Diagnostics::line));
    let tags = top.take("tags").map_or_else(Vec::new, |node| tags(d, node));
    let effect = top
        .take("effect")
        .map_or(Some(Effect::Destructive), |node| effect(d, &node));
    let mut unknown = Vec::new();
    let config = top
        .take("config")
        .map_or_else(IndexMap::new, |node| config(d, node, &mut unknown));
    let known = top
        .take("args")
        .map_or_else(IndexMap::new, |node| args(d, node, &mut unknown));
    let mut lookup = |name: &ArgName| match known.get_key_value(name) {
        None => Lookup::Unknown,
        Some((_, None)) => Lookup::Broken,
        Some((current, Some(arg))) => Lookup::Arg(current, arg),
    };
    let confirm = if let Some(node) = top.take("confirm") {
        template(d, &node, &mut lookup)
    } else {
        d.fail(root_at.as_ref(), "confirm is required");
        None
    };
    let run = run_of(d, top.take("run"), form, &known, root_at.as_ref());
    let owner = if form == Form::Wire {
        Owner::Yours
    } else {
        Owner::Shipped
    };
    let examples = top.take("examples").map_or_else(Records::default, |n| {
        records(d, n, &known, &Records::default(), owner)
    });
    let tests = top.take("tests").map_or_else(Records::default, |n| {
        records(d, n, &known, &examples, owner)
    });
    if form == Form::Wire
        && let Some(node) = top.take("unknown")
    {
        match serde_json::from_value(node.json()) {
            Ok(paths) => unknown = paths,
            Err(_) => d.fail(None, "unknown must be a list of key paths"),
        }
    }
    unknown.extend(top.unknown());
    unknown.sort_by_key(|path| {
        order
            .iter()
            .position(|key| Some(key) == path.segments().first())
    });
    let args: Option<IndexMap<ArgName, Argument>> = known
        .into_iter()
        .map(|(name, arg)| Some((name, arg?)))
        .collect();
    Some(Manifest {
        description: description?,
        not_for,
        tags,
        effect: effect?,
        confirm: confirm?,
        run: run?,
        config,
        args: args?,
        examples,
        tests,
        unknown,
    })
}

pub(crate) fn description(d: &mut Diagnostics, node: &Node) -> Option<Description> {
    let text = d.clean(node)?;
    Description::new(&text).or_else(|| {
        d.fail(
            node.at.as_ref(),
            format!("{}'s first line, the summary, is empty", node.name()),
        );
        None
    })
}

pub(crate) fn list<T>(
    d: &mut Diagnostics,
    node: Node,
    each: impl Fn(&mut Diagnostics, &Node) -> Option<T>,
) -> Vec<T> {
    let mut items = Vec::new();
    for item in d.array(node).unwrap_or_default() {
        items.extend(each(d, &item));
    }
    items
}

pub(crate) fn tags(d: &mut Diagnostics, node: Node) -> Vec<Tag> {
    let mut tags: Vec<Tag> = Vec::new();
    for item in d.array(node).unwrap_or_default() {
        let Some(text) = d.str(&item) else { continue };
        match Tag::new(text) {
            Ok(tag) if tags.contains(&tag) => {
                d.fail(item.at.as_ref(), format!("tags repeats \"{tag}\""));
            }
            Ok(tag) => tags.push(tag),
            Err(why) => d.fail(item.at.as_ref(), format!("tags: {why}")),
        }
    }
    tags
}

pub(crate) fn effect(d: &mut Diagnostics, node: &Node) -> Option<Effect> {
    match d.str(node)? {
        "read" => Some(Effect::Read),
        "write" => Some(Effect::Write),
        "destructive" => Some(Effect::Destructive),
        _ => {
            d.fail(
                node.at.as_ref(),
                format!("{} must be read, write or destructive", node.name()),
            );
            None
        }
    }
}

fn config(
    d: &mut Diagnostics,
    node: Node,
    unknown: &mut Vec<KeyPath>,
) -> IndexMap<ConfigKey, ConfigSpec> {
    let mut config = IndexMap::new();
    let Some(table) = d.table(node) else {
        return config;
    };
    for (key, node) in table.entries() {
        let name = match ConfigKey::new(&key) {
            Ok(name) => name,
            Err(why) => {
                d.fail(node.at.as_ref(), format!("config.{key}: {why}"));
                continue;
            }
        };
        if let Some(spec) = config_spec(d, node, unknown) {
            config.insert(name, spec);
        }
    }
    config
}

fn config_spec(d: &mut Diagnostics, node: Node, unknown: &mut Vec<KeyPath>) -> Option<ConfigSpec> {
    match node.value {
        Value::Str(_) => d.line(&node).map(|about| ConfigSpec {
            about,
            secret: false,
        }),
        Value::Table(_) => {
            let mut table = d.table(node)?;
            let about = if let Some(node) = table.take("about") {
                d.line(&node)
            } else {
                d.fail(
                    table.at.as_ref(),
                    format!("{}.about is required", table.path),
                );
                None
            };
            let secret = table
                .take("secret")
                .map_or(Some(false), |node| d.bool(&node));
            unknown.extend(table.unknown());
            Some(ConfigSpec {
                about: about?,
                secret: secret?,
            })
        }
        _ => {
            d.fail(
                node.at.as_ref(),
                format!("{} must be a string or {{ about, secret }}", node.name()),
            );
            None
        }
    }
}

pub(crate) fn args(d: &mut Diagnostics, node: Node, unknown: &mut Vec<KeyPath>) -> Known {
    let mut known = Known::new();
    let Some(table) = d.table(node) else {
        return known;
    };
    let mut named = Vec::new();
    for (key, node) in table.entries() {
        match ArgName::new(&key) {
            Ok(name) => named.push((name, node)),
            Err(why) => d.fail(node.at.as_ref(), format!("args.{key}: {why}")),
        }
    }
    let live: Vec<ArgName> = named.iter().map(|(name, _)| name.clone()).collect();
    let mut former = IndexMap::new();
    for (name, node) in named {
        let arg = argument(d, node, &name, &live, &mut former, unknown);
        known.insert(name, arg);
    }
    known
}

const SOURCES: [&str; 4] = ["options", "vocab", "pick", "flag"];
const COUNT: [&str; 5] = ["no", "one", "two", "three", "four"];

fn argument(
    d: &mut Diagnostics,
    node: Node,
    name: &ArgName,
    live: &[ArgName],
    former: &mut IndexMap<ArgName, ArgName>,
    unknown: &mut Vec<KeyPath>,
) -> Option<Argument> {
    let mut table = d.table(node)?;
    let path = table.path.clone();
    let ask = if let Some(node) = table.take("ask") {
        d.line(&node)
    } else {
        d.fail(table.at.as_ref(), format!("{path}.ask is required"));
        None
    };
    let mut sources = table.take_any(&SOURCES);
    let range = table.take("range");
    let optional = table.take("optional");
    let was = table
        .take("was")
        .map_or_else(Vec::new, |node| was(d, node, name, live, former));
    unknown.extend(table.unknown());
    let kind = match sources.len() {
        0 => {
            d.fail(
                table.at.as_ref(),
                format!("{path} has no source: options, vocab, pick or flag"),
            );
            None
        }
        1 => {
            let (key, node) = sources.remove(0);
            kind(d, key, &node, &path, range, optional)
        }
        n => {
            let names: Vec<&str> = sources.iter().map(|(key, _)| *key).collect();
            let last = sources.last().and_then(|(_, node)| node.at.as_ref());
            d.fail(
                last,
                format!("{path} has {} sources: {}", COUNT[n.min(4)], words(&names)),
            );
            None
        }
    };
    Some(Argument {
        ask: ask?,
        kind: kind?,
        was,
    })
}

/// `a`, `a and b`, `a, b and c`.
fn words(names: &[&str]) -> String {
    match names.split_last() {
        Some((last, [])) => (*last).to_owned(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}

fn kind(
    d: &mut Diagnostics,
    key: &str,
    node: &Node,
    path: &KeyPath,
    range: Option<Node>,
    optional: Option<Node>,
) -> Option<Kind> {
    if key == "flag" {
        if node.bool() != Some(true) {
            d.fail(
                node.at.as_ref(),
                format!("{path}.flag must be true; omit it otherwise"),
            );
            return None;
        }
        if let Some(optional) = optional {
            d.fail(
                optional.at.as_ref(),
                format!("{path} is a flag, which takes no optional"),
            );
            return None;
        }
        if let Some(range) = range {
            d.fail(
                range.at.as_ref(),
                format!("{path} has range, which only number and duration take"),
            );
            return None;
        }
        return Some(Kind::Flag);
    }
    let optional = optional.map_or(Some(false), |node| d.bool(&node));
    let source = match key {
        "options" => options(d, node, path).map(Source::Options),
        "vocab" => {
            let text = d.str(node)?;
            match VocabName::new(text) {
                Ok(name) => Some(Source::Vocab(name)),
                Err(why) => {
                    d.fail(node.at.as_ref(), format!("{path}.vocab: {why}"));
                    None
                }
            }
        }
        _ => {
            return Some(Kind::Value {
                source: Source::Pick(pick(d, node, path, range)?),
                optional: optional?,
            });
        }
    };
    if let Some(range) = range {
        d.fail(
            range.at.as_ref(),
            format!("{path} has range, which only number and duration take"),
        );
        return None;
    }
    Some(Kind::Value {
        source: source?,
        optional: optional?,
    })
}

fn options(d: &mut Diagnostics, node: &Node, path: &KeyPath) -> Option<Options> {
    let Value::Table(entries) = &node.value else {
        d.fail(node.at.as_ref(), format!("{path}.options must be a table"));
        return None;
    };
    let mut options = IndexMap::new();
    for (key, node) in entries {
        match OptionKey::new(key) {
            Ok(key) => {
                if let Some(meaning) = d.line(node) {
                    options.insert(key, meaning);
                }
            }
            Err(why) => d.fail(node.at.as_ref(), format!("option key {why}")),
        }
    }
    Options::new(options).or_else(|| {
        d.fail(node.at.as_ref(), format!("{path} has no options"));
        None
    })
}

fn pick(d: &mut Diagnostics, node: &Node, path: &KeyPath, range: Option<Node>) -> Option<Pick> {
    let pick = match d.str(node)? {
        "number" => Pick::Number(None),
        "duration" => Pick::Duration(None),
        "email" => Pick::Email,
        "url" => Pick::Url,
        "quoted" => Pick::Quoted,
        _ => {
            d.fail(
                node.at.as_ref(),
                format!("{path}.pick must be number, duration, email, url or quoted"),
            );
            return None;
        }
    };
    match (pick, range) {
        (pick, None) => Some(pick),
        (Pick::Number(_), Some(range)) => {
            range_of(d, range, Node::number, "two numbers").map(|r| Pick::Number(Some(r)))
        }
        (Pick::Duration(_), Some(range)) => {
            let seconds = |node: &Node| node.integer().and_then(|i| u64::try_from(i).ok());
            range_of(d, range, seconds, "two whole numbers of seconds")
                .map(|r| Pick::Duration(Some(r)))
        }
        (_, Some(range)) => {
            d.fail(
                range.at.as_ref(),
                format!("{path} has range, which only number and duration take"),
            );
            None
        }
    }
}

fn range_of<T: PartialOrd + Copy>(
    d: &mut Diagnostics,
    node: Node,
    read: impl Fn(&Node) -> Option<T>,
    what: &str,
) -> Option<Range<T>> {
    let at = node.at.clone();
    let path = node.path.clone();
    let items = d.array(node)?;
    let bounds = match items.as_slice() {
        [min, max] => read(min).zip(read(max)),
        _ => None,
    };
    let Some((min, max)) = bounds else {
        d.fail(at.as_ref(), format!("{path} must be [min, max], {what}"));
        return None;
    };
    Range::new(min, max).or_else(|| {
        d.fail(at.as_ref(), format!("{path} has min above max"));
        None
    })
}

fn was(
    d: &mut Diagnostics,
    node: Node,
    name: &ArgName,
    live: &[ArgName],
    former: &mut IndexMap<ArgName, ArgName>,
) -> Vec<ArgName> {
    let path = node.path.clone();
    let mut list = Vec::new();
    for item in d.array(node).unwrap_or_default() {
        let Some(text) = d.str(&item) else { continue };
        let at = item.at.as_ref();
        let old = match ArgName::new(text) {
            Ok(old) => old,
            Err(why) => {
                d.fail(at, format!("{path}: {why}"));
                continue;
            }
        };
        match former.get(&old) {
            Some(first) if first == name => d.fail(at, format!("{path} repeats {old}")),
            Some(first) => d.fail(
                at,
                format!("{old} is a former name of both {first} and {name}"),
            ),
            None => {
                former.insert(old.clone(), name.clone());
                if live.contains(&old) {
                    d.fail(at, format!("{path} names {old}, a live argument"));
                } else {
                    list.push(old);
                }
            }
        }
    }
    list
}

/// A confirm template with its placeholders resolved to current names; one that is not a required, non-flag
/// argument is a diagnostic.
pub(crate) fn template<'k>(
    d: &mut Diagnostics,
    node: &Node,
    lookup: &mut dyn FnMut(&ArgName) -> Lookup<'k>,
) -> Option<Template> {
    let template = match Template::parse(d.str(node)?) {
        Ok(template) => template,
        Err(why) => {
            d.fail(node.at.as_ref(), format!("{} {why}", node.name()));
            return None;
        }
    };
    let mut ok = true;
    let mut pieces = Vec::new();
    for piece in template.pieces() {
        let Piece::Arg(name) = piece else {
            pieces.push(piece.clone());
            continue;
        };
        let (current, problem) = match lookup(name) {
            Lookup::Unknown => (name, Some("which is not an argument")),
            Lookup::Broken => (name, None),
            Lookup::Arg(current, arg) => match arg.kind {
                Kind::Flag => (current, Some("a flag")),
                Kind::Value { optional: true, .. } => (current, Some("an optional argument")),
                Kind::Value {
                    optional: false, ..
                } => (current, None),
            },
        };
        if let Some(problem) = problem {
            d.fail(
                node.at.as_ref(),
                format!("{} names {{{name}}}, {problem}", node.name()),
            );
            ok = false;
        }
        pieces.push(Piece::Arg(current.clone()));
    }
    ok.then(|| Template::new(pieces))
}

pub(crate) fn run_of(
    d: &mut Diagnostics,
    node: Option<Node>,
    form: Form,
    known: &Known,
    root_at: Option<&At>,
) -> Option<Run> {
    let Some(node) = node else {
        if form == Form::Toml {
            d.fail(root_at, "run is required");
            return None;
        }
        return Some(Run::Inline);
    };
    match &node.value {
        Value::Str(text) => match Entrypoint::new(text) {
            Ok(entrypoint) => Some(Run::File(entrypoint)),
            Err(why) => {
                d.fail(node.at.as_ref(), format!("run: {why}"));
                None
            }
        },
        Value::Array(_) => argv(d, node, known),
        _ => {
            d.fail(node.at.as_ref(), "run must be an entrypoint or an argv");
            None
        }
    }
}

fn argv(d: &mut Diagnostics, node: Node, known: &Known) -> Option<Run> {
    let at = node.at.clone();
    let at = at.as_ref();
    let items = d.array(node)?;
    let Some((first, rest)) = items.split_first() else {
        d.fail(at, "run must name a program");
        return None;
    };
    let program = d.str(first).and_then(|text| match placeholder(text) {
        Some(name) => {
            d.fail(
                at,
                format!("run's first element must be a literal, not {{{name}}}"),
            );
            None
        }
        None => literal(d, at, text),
    });
    let mut elements = Vec::new();
    let mut ok = true;
    for item in rest {
        let at = item.at.as_ref();
        let Some(text) = d.str(item) else {
            ok = false;
            continue;
        };
        let element = match placeholder(text).map(ArgName::new) {
            Some(Ok(name)) => match known.get(&name) {
                None => {
                    d.fail(
                        at,
                        format!("run names {{{name}}}, which is not an argument"),
                    );
                    None
                }
                Some(Some(Argument {
                    kind: Kind::Flag, ..
                })) => {
                    d.fail(
                        at,
                        format!("run names {{{name}}}, a flag; flags need a file body"),
                    );
                    None
                }
                Some(_) => Some(Element::Arg(name)),
            },
            Some(Err(_)) => {
                d.fail(
                    at,
                    format!("run element \"{text}\" is neither a literal nor a {{placeholder}}"),
                );
                None
            }
            None => literal(d, at, text).map(Element::Literal),
        };
        match element {
            Some(element) => elements.push(element),
            None => ok = false,
        }
    }
    ok.then_some(Run::Argv {
        program: program?,
        rest: elements,
    })
}

/// The name inside a whole-element `{placeholder}`.
fn placeholder(text: &str) -> Option<&str> {
    text.strip_prefix('{')?.strip_suffix('}')
}

fn literal(d: &mut Diagnostics, at: Option<&At>, text: &str) -> Option<Clean> {
    let literal = if text.contains(['{', '}']) {
        Err("is neither a literal nor a {placeholder}".to_owned())
    } else {
        Clean::line(text)
    };
    literal
        .map_err(|why| d.fail(at, format!("run element \"{text}\" {why}")))
        .ok()
}

fn records(
    d: &mut Diagnostics,
    node: Node,
    known: &Known,
    other: &Records,
    owner: Owner,
) -> Records {
    read_records(
        d,
        node,
        other,
        &mut |d, utterance, name, node| match known.get_key_value(name) {
            None => {
                d.fail(
                    node.at.as_ref(),
                    format!("\"{utterance}\" asserts {name}, which is not an argument"),
                );
                None
            }
            Some((_, None)) => None,
            Some((current, Some(arg))) => {
                assertion(d, utterance, name, arg, node, owner).map(|a| (current.clone(), a))
            }
        },
    )
}

/// Resolves and types one assertion of an utterance, by the argument name as written; or reports why it cannot.
pub(crate) type Typer<'a> =
    dyn FnMut(&mut Diagnostics, &Clean, &str, &Node) -> Option<(ArgName, Assertion)> + 'a;

/// A records table; `typed` resolves and types each assertion, or reports why it cannot.
pub(crate) fn read_records(
    d: &mut Diagnostics,
    node: Node,
    other: &Records,
    typed: &mut Typer<'_>,
) -> Records {
    let mut records = Records::default();
    let Some(table) = d.table(node) else {
        return records;
    };
    for (key, node) in table.entries() {
        let at = node.at.clone();
        let Some(utterance) = utterance(d, &key, at.as_ref()) else {
            continue;
        };
        let Some(record) = record(d, &utterance, node, typed) else {
            continue;
        };
        let id = identity(utterance.as_str());
        match records.get(&id).or_else(|| other.get(&id)) {
            Some((earlier, _)) => d.fail(
                at.as_ref(),
                format!("\"{utterance}\" repeats \"{earlier}\""),
            ),
            None => {
                records.insert(utterance, record);
            }
        }
    }
    records
}

fn utterance(d: &mut Diagnostics, key: &str, at: Option<&At>) -> Option<Clean> {
    Clean::line(key)
        .map_err(|why| d.fail(at, format!("utterance \"{key}\" {why}")))
        .ok()
}

fn record(
    d: &mut Diagnostics,
    utterance: &Clean,
    node: Node,
    typed: &mut Typer<'_>,
) -> Option<Record> {
    match node.value {
        Value::Bool(false) => Some(Record::Never),
        Value::Table(entries) => {
            let mut asserts = IndexMap::new();
            for (name, node) in &entries {
                if let Some((name, assertion)) = typed(d, utterance, name, node) {
                    asserts.insert(name, assertion);
                }
            }
            Some(Record::Asserts(asserts))
        }
        _ => {
            d.fail(
                node.at.as_ref(),
                format!("\"{utterance}\" must be a table of assertions or false"),
            );
            None
        }
    }
}

/// One asserted value, typed against its argument; `name` is the name as written, for the message.
pub(crate) fn assertion(
    d: &mut Diagnostics,
    utterance: &Clean,
    name: &str,
    arg: &Argument,
    node: &Node,
    owner: Owner,
) -> Option<Assertion> {
    let at = node.at.as_ref();
    let source = match &arg.kind {
        Kind::Flag => None,
        Kind::Value { source, .. } => Some(source),
    };
    if owner == Owner::Shipped && matches!(source, Some(Source::Vocab(_))) {
        d.fail(
            at,
            format!("\"{utterance}\" asserts {name}, a vocab argument; only your own files may"),
        );
        return None;
    }
    let problem = match (source, &node.value) {
        (_, Value::Bool(false)) => return Some(Assertion::Unstated),
        (None, Value::Bool(true)) => return Some(Assertion::Flag),
        (Some(Source::Options(options)), Value::Str(text)) => {
            if let Some((key, _)) = options.get_key_value(text.as_str()) {
                return Some(Assertion::Option(key.clone()));
            }
            let keys: Vec<&str> = options.keys().map(OptionKey::as_str).collect();
            format!(
                "\"{utterance}\" asserts {name} = \"{text}\"; the options are {}",
                keys.join(", ")
            )
        }
        (Some(Source::Vocab(_)), Value::Str(text)) => match Word::new(text) {
            Ok(word) => return Some(Assertion::Word(word)),
            Err(why) => format!("\"{utterance}\" asserts {name} = \"{text}\"; {why}"),
        },
        (Some(Source::Pick(_)), Value::Str(text)) => match Clean::new(text) {
            // A non-empty piece of the utterance, which is clean itself.
            Ok(span) if !text.is_empty() && utterance.as_str().contains(text.as_str()) => {
                return Some(Assertion::Span(span));
            }
            _ => format!(
                "\"{utterance}\" asserts {name} = \"{text}\", which is not in the utterance"
            ),
        },
        (source, value) => {
            let shown = match value {
                Value::Str(text) => format!("\"{text}\""),
                Value::Bool(b) => b.to_string(),
                _ => node.kind().to_owned(),
            };
            let takes = match source {
                None => "is a flag, which takes true or false",
                Some(Source::Options(_)) => "takes an option key",
                Some(Source::Vocab(_)) => "takes a word",
                Some(Source::Pick(_)) => "takes a span of the utterance",
            };
            format!("\"{utterance}\" asserts {name} = {shown}; {name} {takes}")
        }
    };
    d.fail(at, problem);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::File;
    use crate::document::Text;
    use crate::name::LocalName;

    fn parse(text: &str) -> Result<Manifest, Vec<Diagnostic>> {
        manifest(Document {
            file: File::Manifest {
                name: LocalName::new("lights").unwrap(),
            },
            text: Text::Toml(text),
        })
    }

    const HEAD: &str =
        "reflex = 1\ndescription = \"Lights.\"\nconfirm = \"Lights?\"\nrun = \"lights.mts\"\n";

    #[test]
    fn templates_read_and_render_their_placeholders() {
        let template = Template::parse("Set the {room} lights {state}?").unwrap();
        let names: Vec<&str> = template.placeholders().map(ArgName::as_str).collect();
        assert_eq!(names, ["room", "state"]);
        assert_eq!(template.to_string(), "Set the {room} lights {state}?");
        assert_eq!(
            Template::parse("Lights {").unwrap_err(),
            "has an unmatched \"{\""
        );
        assert_eq!(
            Template::parse("Lights }").unwrap_err(),
            "has a stray \"}\""
        );
        assert!(Template::parse("{for}").is_err());
        assert!(Template::parse("No placeholders.").is_ok());
        assert_eq!(Template::parse("").unwrap_err(), "is empty");
        assert!(serde_json::from_str::<Template>("\"two\\nlines\"").is_err());
    }

    #[test]
    fn entrypoints_end_in_mts_or_mjs_after_the_last_slash() {
        assert!(Entrypoint::new("a/b.mjs").is_ok());
        assert!(Entrypoint::new("a.mts/b").is_err());
        assert!(Entrypoint::new(".mts").is_err());
        assert!(Entrypoint::new("a.ts").is_err());
    }

    #[test]
    fn a_range_holds_finite_numbers() {
        let text = format!(
            "{HEAD}[args.level]\nask = \"How much?\"\npick = \"number\"\nrange = [1, inf]\n"
        );
        let errors = parse(&text).unwrap_err();
        assert_eq!(
            errors[0].message,
            "args.level.range must be [min, max], two numbers"
        );
    }

    #[test]
    fn file_order_survives() {
        let text = format!(
            "{HEAD}[examples]\n\"z\" = {{}}\n\"a\" = {{}}\n\n[args.z]\nask = \"Z?\"\noptions = {{ c = \"C\", b = \"B\", a = \"A\" }}\n\n[args.a]\nask = \"A?\"\nflag = true\n"
        );
        let manifest = parse(&text).unwrap();
        let names: Vec<&str> = manifest.args.keys().map(ArgName::as_str).collect();
        assert_eq!(names, ["z", "a"]);
        let Kind::Value {
            source: Source::Options(options),
            ..
        } = &manifest.args["z"].kind
        else {
            panic!("z takes options");
        };
        let keys: Vec<&str> = options.keys().map(OptionKey::as_str).collect();
        assert_eq!(keys, ["c", "b", "a"]);
        let ids: Vec<&str> = manifest
            .examples
            .iter()
            .map(|(id, _)| id.as_str())
            .collect();
        assert_eq!(ids, ["z", "a"]);
    }

    #[test]
    fn the_wire_form_carries_what_your_files_asserted() {
        let json = serde_json::json!({
            "description": "Lights.",
            "not_for": [],
            "tags": [],
            "effect": "write",
            "confirm": "Lights?",
            "run": "lights.mts",
            "config": {},
            "args": { "room": { "ask": "Which room?", "vocab": "rooms", "optional": false, "was": [] } },
            "examples": { "lights off in the den": { "room": "den" } },
            "tests": {},
            "unknown": []
        });
        let manifest: Manifest = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(serde_json::to_value(&manifest).unwrap(), json);
    }

    #[test]
    fn a_description_splits_at_its_first_line() {
        let description =
            Description::new(&Clean::new("Lock the screen.\nEverything keeps running.").unwrap())
                .unwrap();
        assert_eq!(description.summary().as_str(), "Lock the screen.");
        assert_eq!(description.rest().as_str(), "Everything keeps running.");
        assert!(Description::new(&Clean::new("\nLock.").unwrap()).is_none());
        assert!(Description::new(&Clean::new("").unwrap()).is_none());
    }

    #[test]
    fn effects_order_from_loose_to_tight() {
        assert!(Effect::Read < Effect::Write && Effect::Write < Effect::Destructive);
    }

    #[test]
    fn a_range_keeps_min_at_or_below_max() {
        assert!(Range::new(1.0, 100.0).is_some());
        assert!(Range::new(2, 1).is_none());
        assert!(serde_json::from_str::<Range<u64>>("[5, 1]").is_err());
        assert_eq!(
            serde_json::to_string(&Range::new(1, 2).unwrap()).unwrap(),
            "[1,2]"
        );
    }
}
