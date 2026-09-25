//! What a body may touch. `[needs]` is the declaration a manifest carries, contract like `run`: what the body
//! reads and writes, which hosts it reaches, which programs it runs; no key is the tightest declaration. In: the
//! table's node with the arguments and config it may name; a declaration with a call's values, the config's and
//! the home; two declarations. Out: `Needs`; the `Policy` a host holds the body to, strings only; whether one
//! declaration widens another; the `Consent` a lock records; the line and the fix for a path the machine lacks
//! or a body reached past, one for both hosts. Nothing here touches a file system: a host holds the paths.

use std::fmt;

use indexmap::IndexMap;
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::call::Call;
use crate::diagnostic::{At, Diagnostic, Fix};
use crate::document::{self, Diagnostics, Document, Json, KeyPath, Node, Value};
use crate::manifest::{Kind, Run, Source};
use crate::name::{AbsPath, ConfigKey, LocalName, RelPath, ValueName};
use crate::plan::Active;
use crate::text::Clean;

/// The declaration: four lists, each absent when empty. Read from `[needs]`; a lock's and a plan's are narrowed
/// from one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Raw")]
pub struct Needs {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writes: Vec<Entry>,
    #[serde(default, skip_serializing_if = "Hosts::is_none")]
    pub hosts: Hosts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<Program>,
}

/// The declaration as JSON carries it, before a repeat is refused as the table's reader refuses one.
#[derive(Deserialize)]
struct Raw {
    #[serde(default)]
    reads: Vec<Entry>,
    #[serde(default)]
    writes: Vec<Entry>,
    #[serde(default)]
    hosts: Hosts,
    #[serde(default)]
    runs: Vec<Program>,
}

impl TryFrom<Raw> for Needs {
    type Error = String;

    fn try_from(raw: Raw) -> Result<Self, String> {
        fn repeated<T: PartialEq + fmt::Display>(key: Key, items: &[T]) -> Result<(), String> {
            for (n, item) in items.iter().enumerate() {
                if items[..n].contains(item) {
                    return Err(format!("{key} repeats \"{item}\""));
                }
            }
            Ok(())
        }
        repeated(Key::Reads, &raw.reads)?;
        repeated(Key::Writes, &raw.writes)?;
        repeated(Key::Runs, &raw.runs)?;
        Ok(Self {
            reads: raw.reads,
            writes: raw.writes,
            hosts: raw.hosts,
            runs: raw.runs,
        })
    }
}

/// One path the declaration names: under the home, absolute, or the value of an argument or a config key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Entry {
    Home(RelPath),
    Absolute(AbsPath),
    Value(ValueName),
}

/// Which hosts the body may reach: none, or any. A name is held by no kernel, so none is named yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hosts {
    #[default]
    None,
    Any,
}

/// A program the body may run: a name found on `PATH`, or an absolute path; never a relative one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Program(Clean);

/// A key of the table, as a line names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Key {
    Reads,
    Writes,
    Hosts,
    Runs,
}

/// The declaration resolved at a decision: every path absolute, the home expanded, a `{name}` replaced by its
/// value or dropped when unstated. Strings a host holds paths by; nothing checked against a file system.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reads: Vec<Place>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub writes: Vec<Place>,
    #[serde(default, skip_serializing_if = "Hosts::is_none")]
    pub hosts: Hosts,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<Program>,
}

/// A path the body may reach, and the entry it came from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Place {
    pub path: String,
    pub from: Entry,
}

/// What an update does to the needs a person consented to: upstream may keep or narrow them, `removed` what a
/// narrowing dropped, and widens them only through `evoke update --accept`; `locked` is what the lock records
/// meanwhile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Consent {
    Kept { needs: Needs },
    Tightened { needs: Needs, removed: Needs },
    NeedsAccept { locked: Needs, upstream: Needs },
}

/// Where a declaration comes from, so a fix lands where it is written: a local reflex's manifest at its `[needs]`
/// line, or a fetched reflex, whose author's it is; the fix there is the reflex removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Origin {
    Local { at: At },
    Fetched,
}

/// What the machine lacks that the declaration names: a path, or a program `PATH` does not hold.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Lacking {
    Place { place: Place, key: Key },
    Program { program: Program },
}

/// What refused a body, as the loader reports it: Node's permission or the kernel's syscall, and the path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refused {
    pub what: String,
    pub path: String,
}

impl Needs {
    /// Nothing declared: the body's directory and a private temporary folder, nothing else.
    #[must_use]
    pub fn is_none(&self) -> bool {
        self.reads.is_empty()
            && self.writes.is_empty()
            && self.hosts.is_none()
            && self.runs.is_empty()
    }

    fn entries(&self, key: Key) -> &[Entry] {
        match key {
            Key::Reads => &self.reads,
            Key::Writes => &self.writes,
            Key::Hosts | Key::Runs => &[],
        }
    }
}

impl Hosts {
    #[must_use]
    pub fn is_none(&self) -> bool {
        *self == Self::None
    }
}

impl Entry {
    /// `~/…`, `/…` or `{name}`; the error is a fragment to follow the key.
    pub fn parse(text: &str) -> Result<Self, String> {
        if let Some(rest) = text.strip_prefix("~/") {
            return RelPath::new(rest)
                .map(Self::Home)
                .map_err(|_| format!("\"{text}\" is not a plain path under your home"));
        }
        if text.starts_with('/') {
            return AbsPath::new(text).map(Self::Absolute);
        }
        if let Some(name) = text
            .strip_prefix('{')
            .and_then(|rest| rest.strip_suffix('}'))
        {
            return ValueName::new(name)
                .map(Self::Value)
                .map_err(|why| format!("{{{name}}}: {why}"));
        }
        Err(format!(
            "\"{text}\" is neither ~/…, an absolute path nor a {{name}}"
        ))
    }
}

impl TryFrom<String> for Entry {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text)
    }
}

impl From<Entry> for String {
    fn from(entry: Entry) -> Self {
        entry.to_string()
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Home(rest) => write!(f, "~/{rest}"),
            Self::Absolute(path) => f.write_str(path.as_str()),
            Self::Value(name) => write!(f, "{{{name}}}"),
        }
    }
}

impl Program {
    /// A name or an absolute path; the error is a fragment to follow the key.
    pub fn new(text: &str) -> Result<Self, String> {
        if text.contains(['{', '}']) {
            return Err(format!(
                "\"{text}\" is not a program; runs takes a name on PATH or an absolute path"
            ));
        }
        if text.contains('/') && !text.starts_with('/') {
            return Err(format!(
                "\"{text}\" is a relative path; name a program on PATH or by its absolute path"
            ));
        }
        Clean::line(text)
            .map(Self)
            .map_err(|why| format!("\"{text}\" {why}"))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl TryFrom<String> for Program {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::new(&text)
    }
}

impl From<Program> for String {
    fn from(program: Program) -> Self {
        program.0.to_string()
    }
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Reads => "reads",
            Self::Writes => "writes",
            Self::Hosts => "hosts",
            Self::Runs => "runs",
        })
    }
}

/// `writes {to} ~/Downloads · hosts *`, as `add` and `show` list a declaration; `none` when nothing is declared.
impl fmt::Display for Needs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        for (key, entries) in [(Key::Reads, &self.reads), (Key::Writes, &self.writes)] {
            if !entries.is_empty() {
                let listed: Vec<String> = entries.iter().map(ToString::to_string).collect();
                parts.push(format!("{key} {}", listed.join(" ")));
            }
        }
        if self.hosts == Hosts::Any {
            parts.push("hosts *".to_owned());
        }
        if !self.runs.is_empty() {
            let listed: Vec<&str> = self.runs.iter().map(Program::as_str).collect();
            parts.push(format!("runs {}", listed.join(" ")));
        }
        if parts.is_empty() {
            f.write_str("none")
        } else {
            f.write_str(&parts.join(" · "))
        }
    }
}

impl Serialize for Hosts {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let listed: &[&str] = match self {
            Self::None => &[],
            Self::Any => &["*"],
        };
        listed.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Hosts {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let listed = Vec::<String>::deserialize(deserializer)?;
        match listed
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .as_slice()
        {
            [] => Ok(Self::None),
            ["*"] => Ok(Self::Any),
            _ => Err(D::Error::custom("hosts takes \"*\" or nothing")),
        }
    }
}

/// What a `{name}` may resolve to, as the manifest reader knows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Named {
    /// An argument that carries a value.
    Arg,
    /// An argument that is a flag: `true` or nothing, never a path.
    Flag,
    Config,
    Both,
    Neither,
}

/// The table read: each list's shape, and a `{name}` checked against `named` when given — the manifest reader
/// knows its arguments and config, the lock reader trusts what it wrote. Unknown keys are the caller's to report.
pub(crate) fn read(
    d: &mut Diagnostics,
    node: Node,
    unknown: &mut Vec<KeyPath>,
    named: Option<&dyn Fn(&ValueName) -> Named>,
) -> Needs {
    let Some(mut table) = d.table(node) else {
        return Needs::default();
    };
    let mut needs = Needs::default();
    for (key, into) in [
        (Key::Reads, &mut needs.reads),
        (Key::Writes, &mut needs.writes),
    ] {
        let Some(node) = table.take(&key.to_string()) else {
            continue;
        };
        let path = node.path.clone();
        for item in d.array(node).unwrap_or_default() {
            let Some(text) = d.str(&item) else { continue };
            let at = item.at.as_ref();
            let entry = match Entry::parse(text) {
                Ok(entry) => entry,
                Err(why) => {
                    d.fail(at, format!("{path}: {why}"));
                    continue;
                }
            };
            if let (Entry::Value(name), Some(named)) = (&entry, named) {
                let problem = match named(name) {
                    Named::Arg | Named::Config => None,
                    Named::Flag => Some("a flag, which has no value"),
                    Named::Both => Some("which is both an argument and a config key"),
                    Named::Neither => Some("which is neither an argument nor a config key"),
                };
                if let Some(problem) = problem {
                    d.fail(at, format!("{path} names {{{name}}}, {problem}"));
                    continue;
                }
            }
            if into.contains(&entry) {
                d.fail(at, format!("{path} repeats \"{entry}\""));
            } else {
                into.push(entry);
            }
        }
    }
    if let Some(node) = table.take("hosts") {
        let path = node.path.clone();
        for item in d.array(node).unwrap_or_default() {
            let Some(text) = d.str(&item) else { continue };
            if text == "*" {
                needs.hosts = Hosts::Any;
            } else {
                d.fail(
                    item.at.as_ref(),
                    format!("{path} names \"{text}\"; hosts takes \"*\" or nothing"),
                );
            }
        }
    }
    if let Some(node) = table.take("runs") {
        let path = node.path.clone();
        for item in d.array(node).unwrap_or_default() {
            let Some(text) = d.str(&item) else { continue };
            match Program::new(text) {
                Ok(program) if needs.runs.contains(&program) => {
                    d.fail(item.at.as_ref(), format!("{path} repeats \"{program}\""));
                }
                Ok(program) => needs.runs.push(program),
                Err(why) => d.fail(item.at.as_ref(), format!("{path}: {why}")),
            }
        }
    }
    unknown.extend(table.unknown());
    needs
}

/// The line `[needs]` sits on in a manifest's text, else the document's first line, where the table is added:
/// what a refusal's fix points a local reflex's author at.
#[must_use]
pub fn declared_at(doc: Document<'_>) -> At {
    let first = At {
        file: doc.file.clone(),
        line: 1,
        column: 1,
    };
    let Ok(root) = document::root(doc) else {
        return first;
    };
    let table = match &root.value {
        Value::Table(entries) => entries
            .iter()
            .find(|(key, _)| key == "needs")
            .and_then(|(_, node)| node.at.clone()),
        _ => None,
    };
    table.or(root.at).unwrap_or(first)
}

/// The declaration at a decision: each `{name}` replaced by the call's value as the body receives it or the
/// setting, dropped when the argument is unstated; an argv body's program is a program it runs. A value that is
/// not a path is refused, naming its source.
pub fn resolve(
    needs: &Needs,
    call: &Call,
    active: &Active,
    config: &IndexMap<ConfigKey, String>,
    home: &str,
) -> Result<Policy, Diagnostic> {
    let mut policy = Policy {
        hosts: needs.hosts,
        runs: needs.runs.clone(),
        ..Policy::default()
    };
    // An argv body is its program: it runs, and asks the system what it does, as a declared program would.
    if let Run::Argv { program, .. } = &active.run
        && !policy.runs.contains(program)
    {
        policy.runs.insert(0, program.clone());
    }
    for (key, entries, into) in [
        (Key::Reads, &needs.reads, &mut policy.reads),
        (Key::Writes, &needs.writes, &mut policy.writes),
    ] {
        for entry in entries {
            let text = match entry {
                Entry::Home(rest) => format!("{home}/{rest}"),
                Entry::Absolute(path) => path.to_string(),
                Entry::Value(name) => {
                    let Some(value) = value_of(name, call, active, config, home) else {
                        continue;
                    };
                    if !value.starts_with('/') {
                        return Err(Diagnostic {
                            reflex: Some(call.reflex.clone()),
                            at: None,
                            message: format!(
                                "[needs] {key} names {{{name}}}, whose value {} is not a path",
                                Json::String(value)
                            ),
                            fix: value_fix(name, &call.reflex, active),
                        });
                    }
                    value
                }
            };
            into.push(Place {
                path: text,
                from: entry.clone(),
            });
        }
    }
    Ok(policy)
}

/// A `{name}`'s value at the decision: the argument's value as the body receives it, or the setting as the
/// envelope carries it; none when unstated.
fn value_of(
    name: &ValueName,
    call: &Call,
    active: &Active,
    config: &IndexMap<ConfigKey, String>,
    home: &str,
) -> Option<String> {
    if active.args.contains_key(name.as_str()) {
        return match call.args.get(name.as_str())?.under_home(home) {
            Json::String(text) => Some(text),
            other => Some(other.to_string()),
        };
    }
    config.get(name.as_str()).cloned()
}

/// The command that gives a `{name}` a path: the setting, the vocabulary's word, or the call typed again.
fn value_fix(name: &ValueName, reflex: &LocalName, active: &Active) -> Fix {
    match active
        .args
        .get(name.as_str())
        .map(|argument| &argument.kind)
    {
        Some(Kind::Value {
            source: Source::Vocab(vocab),
            ..
        }) => Fix::VocabAdd {
            vocab: vocab.clone(),
        },
        Some(Kind::Value {
            source: Source::Pick(_),
            ..
        }) => Fix::Rerun,
        Some(_) => Fix::Show {
            reflex: Some(reflex.clone()),
        },
        None => match ConfigKey::new(name.as_str()) {
            Ok(key) => Fix::ConfigSet {
                reflex: reflex.clone(),
                key,
            },
            Err(_) => Fix::Rerun,
        },
    }
}

impl Policy {
    /// Whether the policy reaches what a body asked for: a path under a declared place, any host, a program by
    /// its name or its path.
    #[must_use]
    pub fn allows(&self, key: Key, what: &str) -> bool {
        let under = |places: &[Place]| {
            places.iter().any(|place| {
                what == place.path
                    || place.path == "/"
                    || what
                        .strip_prefix(place.path.as_str())
                        .is_some_and(|rest| rest.starts_with('/'))
            })
        };
        match key {
            Key::Reads => under(&self.reads) || under(&self.writes),
            Key::Writes => under(&self.writes),
            Key::Hosts => self.hosts == Hosts::Any,
            Key::Runs => self.runs.iter().any(|program| {
                program.as_str() == what
                    || what.rsplit('/').next() == Some(program.as_str())
                    || program.as_str().rsplit('/').next() == Some(what)
            }),
        }
    }
}

/// A path as a person reads it: `~/…` when it is under the home.
#[must_use]
pub fn shown(path: &str, home: &str) -> String {
    if home.is_empty() || home == "/" {
        return path.to_owned();
    }
    match path.strip_prefix(home) {
        Some("") => "~".to_owned(),
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => path.to_owned(),
    }
}

/// A declared path or program the machine lacks, before anything runs: `[needs] writes names ~/notes.txt, which
/// is not there`, and the command that answers it — the value's source when the entry is a `{name}`, else the
/// declaration itself.
#[must_use]
pub fn lacking(
    lacking: &Lacking,
    reflex: &LocalName,
    active: &Active,
    origin: &Origin,
    home: &str,
) -> Diagnostic {
    let (message, fix) = match lacking {
        Lacking::Place { place, key } => (
            format!(
                "[needs] {key} names {}, which is not there",
                shown(&place.path, home)
            ),
            match &place.from {
                Entry::Value(name) => value_fix(name, reflex, active),
                Entry::Home(_) | Entry::Absolute(_) => declaration_fix(reflex, origin),
            },
        ),
        Lacking::Program { program } => (
            if program.as_str().starts_with('/') {
                format!("[needs] runs names {program}, which is not there")
            } else {
                format!("[needs] runs names {program}, which is not on PATH")
            },
            declaration_fix(reflex, origin),
        ),
    };
    Diagnostic {
        reflex: Some(reflex.clone()),
        at: None,
        message,
        fix,
    }
}

/// What a body reached past its declaration, from what the loader reported: Node's permission names the key
/// outright; the kernel's syscall says what was tried — a program spawned, a write, the resolver — and the path
/// the rest, a write's key by whether the policy reads there already. The line names the path and the key,
/// `~/secret.txt is not in [needs] reads`; the fix is the declaration, or `--accept` when `upstream` — the
/// shipped declaration of a fetched reflex, resolved — already allows it. None when the loader could not name
/// what refused the body, and none when the policy allows what was refused: that refusal was not the
/// declaration's, and the body's own error stands.
#[must_use]
pub fn refusal(
    policy: &Policy,
    upstream: Option<&Policy>,
    reflex: &LocalName,
    origin: &Origin,
    refused: &Refused,
    home: &str,
) -> Option<Diagnostic> {
    const WRITES: [&str; 14] = [
        "mkdir",
        "rmdir",
        "unlink",
        "rename",
        "link",
        "symlink",
        "truncate",
        "ftruncate",
        "chmod",
        "fchmod",
        "chown",
        "utime",
        "futime",
        "copyfile",
    ];
    let path = refused.path.clone();
    let (key, path) = match (refused.what.as_str(), refused.what.split_once(' ')) {
        ("FileSystemRead", _) => (Key::Reads, path),
        ("FileSystemWrite", _) => (Key::Writes, path),
        ("ChildProcess", _) => (Key::Runs, String::new()),
        ("resolve" | "connect" | "bind" | "socket", _) => (Key::Hosts, path),
        (_, Some(("spawn" | "spawnSync", program))) => (Key::Runs, program.to_owned()),
        ("", _) => return None,
        (syscall, _) if WRITES.contains(&syscall) => (Key::Writes, path),
        // A path the body may read already: what the kernel refused on it was a write.
        _ if policy.allows(Key::Reads, &path) => (Key::Writes, path),
        _ => (Key::Reads, path),
    };
    let named = if path.is_empty() {
        key == Key::Runs && policy.runs.is_empty()
    } else {
        !policy.allows(key, &path)
    };
    if !named {
        return None;
    }
    let message = if path.is_empty() {
        format!("[needs] {key} names no program")
    } else {
        format!("{} is not in [needs] {key}", shown(&path, home))
    };
    let fix = if upstream.is_some_and(|upstream| !path.is_empty() && upstream.allows(key, &path)) {
        Fix::Accept {
            reflex: reflex.clone(),
        }
    } else {
        declaration_fix(reflex, origin)
    };
    Some(Diagnostic {
        reflex: Some(reflex.clone()),
        at: None,
        message,
        fix,
    })
}

/// The command that answers the declaration itself: a local reflex's manifest at its `[needs]` line, or the
/// fetched reflex removed.
fn declaration_fix(reflex: &LocalName, origin: &Origin) -> Fix {
    match origin {
        Origin::Local { at } => Fix::EditLine { at: at.clone() },
        Origin::Fetched => Fix::Remove {
            reflex: reflex.clone(),
        },
    }
}

/// `shipped` kept to what `consented` names: every entry in both, in shipped's order; any host only when both say
/// so. What a body runs under.
#[must_use]
pub fn narrowed(shipped: &Needs, consented: &Needs) -> Needs {
    let kept = |key: Key| -> Vec<Entry> {
        shipped
            .entries(key)
            .iter()
            .filter(|entry| consented.entries(key).contains(entry))
            .cloned()
            .collect()
    };
    Needs {
        reads: kept(Key::Reads),
        writes: kept(Key::Writes),
        hosts: if shipped.hosts == Hosts::Any && consented.hosts == Hosts::Any {
            Hosts::Any
        } else {
            Hosts::None
        },
        runs: shipped
            .runs
            .iter()
            .filter(|program| consented.runs.contains(program))
            .cloned()
            .collect(),
    }
}

/// What `of` declares that `over` does not: the entries and hosts added, in `of`'s order.
#[must_use]
pub fn added(of: &Needs, over: &Needs) -> Needs {
    let more = |key: Key| -> Vec<Entry> {
        of.entries(key)
            .iter()
            .filter(|entry| !over.entries(key).contains(entry))
            .cloned()
            .collect()
    };
    Needs {
        reads: more(Key::Reads),
        writes: more(Key::Writes),
        hosts: if of.hosts == Hosts::Any && over.hosts == Hosts::None {
            Hosts::Any
        } else {
            Hosts::None
        },
        runs: of
            .runs
            .iter()
            .filter(|program| !over.runs.contains(program))
            .cloned()
            .collect(),
    }
}

/// Whether `to` reaches anything `from` does not.
#[must_use]
pub fn widens(from: &Needs, to: &Needs) -> bool {
    !added(to, from).is_none()
}

/// The needs a person runs under after an update: theirs, narrowed to what upstream still declares; what upstream
/// added waits for `evoke update --accept`.
#[must_use]
pub fn consent(locked: &Needs, upstream: &Needs) -> Consent {
    let kept = narrowed(upstream, locked);
    if widens(locked, upstream) {
        Consent::NeedsAccept {
            locked: kept,
            upstream: upstream.clone(),
        }
    } else if kept == *locked {
        Consent::Kept { needs: kept }
    } else {
        Consent::Tightened {
            removed: added(locked, &kept),
            needs: kept,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::File;
    use crate::document::Text;
    use crate::manifest::manifest;

    fn needs(text: &str) -> Needs {
        serde_json::from_str(text).unwrap()
    }

    #[test]
    fn a_repeat_is_refused_from_json_as_from_the_table() {
        let error = serde_json::from_str::<Needs>(r#"{ "reads": ["~/a", "~/a"] }"#).unwrap_err();
        assert_eq!(error.to_string(), "reads repeats \"~/a\"");
    }

    fn parse(tail: &str) -> Result<crate::manifest::Manifest, Vec<Diagnostic>> {
        let text = format!(
            "reflex = 1\ndescription = \"Note.\"\nconfirm = \"Note {{text}}?\"\nrun = \"note.mts\"\n\n[config]\nfile = \"The notes file\"\n\n[args.text]\nask = \"What?\"\npick = \"quoted\"\n\n[args.loud]\nask = \"Loud?\"\nflag = true\n\n{tail}"
        );
        manifest(Document {
            file: File::Manifest {
                name: LocalName::new("note").unwrap(),
            },
            text: Text::Toml(&text),
        })
    }

    #[test]
    fn entries_take_three_forms_and_display_as_written() {
        for text in ["~/Downloads", "/etc/hosts", "{to}", "/"] {
            assert_eq!(Entry::parse(text).unwrap().to_string(), text);
        }
        assert!(Entry::parse("~/").is_err());
        assert!(Entry::parse("~").is_err());
        assert!(Entry::parse("Downloads").is_err());
        assert!(Entry::parse("{To}").is_err());
        assert!(Entry::parse("/a/../b").is_err());
        assert!(Program::new("open").is_ok());
        assert!(Program::new("/usr/bin/open").is_ok());
        assert!(Program::new("bin/open").is_err());
        assert!(Program::new("{tool}").is_err());
    }

    #[test]
    fn the_table_reads_and_names_are_checked() {
        let read = parse("[needs]\nreads = [\"{file}\", \"/etc/hosts\"]\nwrites = [\"~/notes\"]\nhosts = [\"*\"]\nruns = [\"open\"]\n").unwrap();
        assert_eq!(
            read.needs.to_string(),
            "reads {file} /etc/hosts · writes ~/notes · hosts * · runs open"
        );
        let messages = |tail: &str| -> Vec<String> {
            parse(tail)
                .unwrap_err()
                .into_iter()
                .map(|problem| problem.message)
                .collect()
        };
        assert_eq!(
            messages(
                "[needs]\nreads = [\"{nowhere}\", \"{loud}\", \"x\"]\nhosts = [\"example.com\"]\nruns = [\"bin/x\"]\n"
            ),
            [
                "needs.reads names {nowhere}, which is neither an argument nor a config key",
                "needs.reads names {loud}, a flag, which has no value",
                "needs.reads: \"x\" is neither ~/…, an absolute path nor a {name}",
                "needs.hosts names \"example.com\"; hosts takes \"*\" or nothing",
                "needs.runs: \"bin/x\" is a relative path; name a program on PATH or by its absolute path",
            ]
        );
        assert_eq!(
            messages("[needs]\nwrites = [\"~/a\", \"~/a\"]\n"),
            ["needs.writes repeats \"~/a\""]
        );
        let both = manifest(Document {
            file: File::Manifest {
                name: LocalName::new("note").unwrap(),
            },
            text: Text::Toml(
                "reflex = 1\ndescription = \"Note.\"\nconfirm = \"Note?\"\nrun = \"note.mts\"\n\n[config]\ntext = \"Twice\"\n\n[args.text]\nask = \"What?\"\npick = \"quoted\"\n\n[needs]\nreads = [\"{text}\"]\n",
            ),
        })
        .unwrap_err();
        assert_eq!(
            both[0].message,
            "needs.reads names {text}, which is both an argument and a config key"
        );
        let unknown = parse("[needs]\nlistens = [\"x\"]\n").unwrap();
        assert_eq!(unknown.unknown[0].to_string(), "needs.listens");
        assert!(parse("").unwrap().needs.is_none());
    }

    #[test]
    fn the_wire_form_is_the_file_form_and_none_is_absent() {
        let read = parse("[needs]\nwrites = [\"{file}\"]\nhosts = [\"*\"]\n").unwrap();
        let json = serde_json::to_value(&read).unwrap();
        assert_eq!(
            json["needs"],
            serde_json::json!({ "writes": ["{file}"], "hosts": ["*"] })
        );
        assert!(
            serde_json::to_value(parse("").unwrap())
                .unwrap()
                .get("needs")
                .is_none()
        );
        assert!(serde_json::from_str::<Needs>(r#"{ "hosts": ["example.com"] }"#).is_err());
        assert_eq!(needs("{}"), Needs::default());
    }

    #[test]
    fn narrowing_keeps_what_both_name_and_widening_is_what_one_adds() {
        let shipped =
            needs(r#"{ "writes": ["{to}", "~/Downloads"], "hosts": ["*"], "runs": ["open"] }"#);
        let consented = needs(r#"{ "writes": ["~/Downloads"], "runs": ["open", "plutil"] }"#);
        assert_eq!(
            narrowed(&shipped, &consented),
            needs(r#"{ "writes": ["~/Downloads"], "runs": ["open"] }"#)
        );
        assert_eq!(
            added(&shipped, &consented),
            needs(r#"{ "writes": ["{to}"], "hosts": ["*"] }"#)
        );
        assert!(widens(&consented, &shipped));
        assert!(widens(&shipped, &consented));
        assert!(!widens(&shipped, &shipped));
        assert!(!widens(&shipped, &Needs::default()));
    }

    #[test]
    fn consent_keeps_tightens_or_waits() {
        let locked = needs(r#"{ "writes": ["~/Downloads"], "hosts": ["*"] }"#);
        assert_eq!(
            consent(&locked, &locked),
            Consent::Kept {
                needs: locked.clone()
            }
        );
        let narrower = needs(r#"{ "writes": ["~/Downloads"] }"#);
        assert_eq!(
            consent(&locked, &narrower),
            Consent::Tightened {
                needs: narrower.clone(),
                removed: needs(r#"{ "hosts": ["*"] }"#),
            }
        );
        let wider = needs(r#"{ "writes": ["~/Downloads", "{to}"], "hosts": ["*"] }"#);
        assert_eq!(
            consent(&locked, &wider),
            Consent::NeedsAccept {
                locked: locked.clone(),
                upstream: wider
            }
        );
        // Widened one way and narrowed the other: the lock keeps the meet, the addition waits.
        let moved = needs(r#"{ "writes": ["{to}"], "hosts": ["*"] }"#);
        assert_eq!(
            consent(&locked, &moved),
            Consent::NeedsAccept {
                locked: needs(r#"{ "hosts": ["*"] }"#),
                upstream: moved
            }
        );
    }

    #[test]
    fn a_policy_allows_what_it_reaches() {
        let policy = Policy {
            reads: vec![Place {
                path: "/home/me/notes".to_owned(),
                from: Entry::parse("~/notes").unwrap(),
            }],
            writes: vec![Place {
                path: "/home/me/out.txt".to_owned(),
                from: Entry::parse("{file}").unwrap(),
            }],
            hosts: Hosts::None,
            runs: vec![
                Program::new("open").unwrap(),
                Program::new("/usr/bin/plutil").unwrap(),
            ],
        };
        assert!(policy.allows(Key::Reads, "/home/me/notes/today.txt"));
        assert!(policy.allows(Key::Reads, "/home/me/notes"));
        assert!(!policy.allows(Key::Reads, "/home/me/notes2"));
        assert!(policy.allows(Key::Reads, "/home/me/out.txt"));
        assert!(!policy.allows(Key::Writes, "/home/me/notes/today.txt"));
        assert!(!policy.allows(Key::Hosts, "example.com"));
        assert!(policy.allows(Key::Runs, "/usr/bin/open"));
        assert!(policy.allows(Key::Runs, "plutil"));
        assert!(!policy.allows(Key::Runs, "env"));
        let root = Policy {
            reads: vec![Place {
                path: "/".to_owned(),
                from: Entry::parse("/").unwrap(),
            }],
            ..Policy::default()
        };
        assert!(root.allows(Key::Reads, "/etc/hosts"));
    }

    #[test]
    fn the_needs_line_is_found_or_the_first_line_stands_in() {
        let doc = |text: &'static str| Document {
            file: File::Manifest {
                name: LocalName::new("note").unwrap(),
            },
            text: Text::Toml(text),
        };
        let at = declared_at(doc("reflex = 1\n\n[needs]\nwrites = [\"~/x\"]\n"));
        assert_eq!((at.line, at.column), (3, 1));
        let at = declared_at(doc("reflex = 1\ndescription = \"x\"\n"));
        assert_eq!((at.line, at.column), (1, 1));
        let at = declared_at(doc("reflex = \n"));
        assert_eq!((at.line, at.column), (1, 1));
    }

    fn paths(places: &[Place]) -> Vec<&str> {
        places.iter().map(|place| place.path.as_str()).collect()
    }

    /// A download reflex with a place to write, a file from a setting and a word to read from.
    fn download() -> Active {
        serde_json::from_value(serde_json::json!({
            "effect": "write",
            "run": "download.mts",
            "needs": { "reads": ["{place}", "~/notes"], "writes": ["{to}", "{file}", "/tmp/out"], "hosts": ["*"], "runs": ["open"] },
            "confirm": "Download?",
            "args": {
                "to": { "ask": "Where?", "vocab": "places", "optional": true, "was": [] },
                "place": { "ask": "Which?", "vocab": "places", "optional": true, "was": [] },
                "text": { "ask": "What?", "pick": "quoted", "optional": true, "was": [] }
            },
            "yields": {},
            "config": { "file": { "type": "plain", "value": "~/notes.txt" } },
            "tags": []
        }))
        .unwrap()
    }

    #[test]
    fn a_policy_resolves_values_drops_the_unstated_and_refuses_a_value_that_is_no_path() {
        use crate::call::Value;
        use crate::name::{ArgName, Word};
        let active = download();
        let call = |args: &[(&str, Value)]| Call {
            reflex: LocalName::new("download").unwrap(),
            args: args
                .iter()
                .map(|(name, value)| (ArgName::new(name).unwrap(), value.clone()))
                .collect(),
        };
        let word = |word: &str, value: Option<&str>| Value::Word {
            word: Word::new(word).unwrap(),
            value: value.map(str::to_owned),
        };
        let config = IndexMap::from([(ConfigKey::new("file").unwrap(), "~/notes.txt".to_owned())]);
        let policy = resolve(
            &active.needs,
            &call(&[("to", word("desk", Some("/Users/me/Desktop")))]),
            &active,
            &config,
            "/Users/me",
        )
        .unwrap();
        assert_eq!(paths(&policy.reads), ["/Users/me/notes"]);
        assert_eq!(
            paths(&policy.writes),
            ["/Users/me/Desktop", "/Users/me/notes.txt", "/tmp/out"]
        );
        assert_eq!(policy.writes[0].from, Entry::parse("{to}").unwrap());
        assert_eq!(policy.hosts, Hosts::Any);
        assert_eq!(policy.runs, [Program::new("open").unwrap()]);
        let refused = resolve(
            &active.needs,
            &call(&[("to", word("desk", None))]),
            &active,
            &config,
            "/Users/me",
        )
        .unwrap_err();
        assert_eq!(
            refused.message,
            "[needs] writes names {to}, whose value \"desk\" is not a path"
        );
        assert_eq!(
            refused.fix,
            Fix::VocabAdd {
                vocab: crate::name::VocabName::new("places").unwrap()
            }
        );
        let bad = IndexMap::from([(ConfigKey::new("file").unwrap(), "notes.txt".to_owned())]);
        let refused = resolve(&active.needs, &call(&[]), &active, &bad, "/Users/me").unwrap_err();
        assert_eq!(
            refused.fix,
            Fix::ConfigSet {
                reflex: LocalName::new("download").unwrap(),
                key: ConfigKey::new("file").unwrap()
            }
        );
    }

    #[test]
    fn what_the_machine_lacks_names_the_key_and_the_source_of_the_fix() {
        let active = download();
        let download = LocalName::new("download").unwrap();
        let place = Place {
            path: "/Users/me/notes".to_owned(),
            from: Entry::parse("~/notes").unwrap(),
        };
        let lacked = lacking(
            &Lacking::Place {
                place,
                key: Key::Reads,
            },
            &download,
            &active,
            &Origin::Fetched,
            "/Users/me",
        );
        assert_eq!(
            lacked.message,
            "[needs] reads names ~/notes, which is not there"
        );
        assert_eq!(
            lacked.fix,
            Fix::Remove {
                reflex: download.clone()
            }
        );
        let at = At {
            file: File::Manifest {
                name: download.clone(),
            },
            line: 9,
            column: 1,
        };
        let program = lacking(
            &Lacking::Program {
                program: Program::new("plutil").unwrap(),
            },
            &download,
            &active,
            &Origin::Local { at: at.clone() },
            "/Users/me",
        );
        assert_eq!(
            program.message,
            "[needs] runs names plutil, which is not on PATH"
        );
        assert_eq!(program.fix, Fix::EditLine { at });
    }

    #[test]
    fn a_refusal_names_the_path_and_the_key_and_accept_when_upstream_allows_it() {
        let note = LocalName::new("note").unwrap();
        let at = At {
            file: File::Manifest { name: note.clone() },
            line: 9,
            column: 1,
        };
        let local = Origin::Local { at: at.clone() };
        let policy = Policy {
            writes: vec![Place {
                path: "/Users/me/notes".to_owned(),
                from: Entry::parse("~/notes").unwrap(),
            }],
            ..Policy::default()
        };
        let refused = |what: &str, path: &str| Refused {
            what: what.to_owned(),
            path: path.to_owned(),
        };
        let line = |what: &str, path: &str| {
            refusal(
                &policy,
                None,
                &note,
                &local,
                &refused(what, path),
                "/Users/me",
            )
            .unwrap()
            .message
        };
        assert_eq!(
            line("FileSystemRead", "/Users/me/secret.txt"),
            "~/secret.txt is not in [needs] reads"
        );
        assert_eq!(
            line("FileSystemWrite", "/Users/me/pwned.txt"),
            "~/pwned.txt is not in [needs] writes"
        );
        assert_eq!(line("ChildProcess", ""), "[needs] runs names no program");
        assert_eq!(line("spawn env", "env"), "env is not in [needs] runs");
        assert_eq!(
            line("resolve", "example.com"),
            "example.com is not in [needs] hosts"
        );
        assert_eq!(line("mkdir", "/tmp/x"), "/tmp/x is not in [needs] writes");
        assert_eq!(
            line("open", "/Users/me/notes/link"),
            "~/notes/link is not in [needs] writes"
        );
        assert_eq!(
            line("open", "/etc/hosts"),
            "/etc/hosts is not in [needs] reads"
        );
        assert_eq!(
            refusal(&policy, None, &note, &local, &refused("", ""), "/Users/me"),
            None
        );
        assert_eq!(
            refusal(
                &policy,
                None,
                &note,
                &local,
                &refused("FileSystemRead", "/x"),
                "/Users/me"
            )
            .unwrap()
            .fix,
            Fix::EditLine { at }
        );
    }

    #[test]
    fn a_refusal_upstream_allows_is_an_accept_and_one_it_does_not_a_removal() {
        let note = LocalName::new("note").unwrap();
        let policy = Policy::default();
        let refused = |what: &str, path: &str| Refused {
            what: what.to_owned(),
            path: path.to_owned(),
        };
        let upstream = Policy {
            reads: vec![Place {
                path: "/Users/me/secret.txt".to_owned(),
                from: Entry::parse("~/secret.txt").unwrap(),
            }],
            ..Policy::default()
        };
        let accept = refusal(
            &policy,
            Some(&upstream),
            &note,
            &Origin::Fetched,
            &refused("FileSystemRead", "/Users/me/secret.txt"),
            "/Users/me",
        )
        .unwrap();
        assert_eq!(
            accept.fix,
            Fix::Accept {
                reflex: note.clone()
            }
        );
        let remove = refusal(
            &policy,
            Some(&upstream),
            &note,
            &Origin::Fetched,
            &refused("FileSystemRead", "/Users/me/other.txt"),
            "/Users/me",
        )
        .unwrap();
        assert_eq!(remove.fix, Fix::Remove { reflex: note });
        assert_eq!(shown("/Users/me", "/Users/me"), "~");
        assert_eq!(shown("/Users/meow/x", "/Users/me"), "/Users/meow/x");
    }
}
