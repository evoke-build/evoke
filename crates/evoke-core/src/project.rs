//! `evoke.toml` and `evoke.lock` as types, and the ref grammar. In: a `Document`; a ref as typed; a `Lock` to
//! render. Out: `Project`, `Lock`, the lock's text, a `Reference` with its pin; or diagnostics.

use std::fmt::{self, Write as _};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, Fix};
use crate::digest::Digest;
use crate::document::{self, Diagnostics, Document, Json, Node, Table, Value};
use crate::manifest::{self, Effect};
use crate::name::{AdapterId, AdapterName, ConfigKey, LocalName, Owner, RelPath, Segment, VarName};
use crate::needs::{self, Hosts, Needs};
use crate::text::Clean;

/// What you wrote: the adapter that decides, the reflexes you installed, their settings, the adapters' own tables.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// Always stated; a name, never a path.
    pub adapter: AdapterName,
    pub reflexes: IndexMap<LocalName, Location>,
    pub config: IndexMap<LocalName, IndexMap<ConfigKey, Setting>>,
    /// Inert unless selected; then its adapter validates it.
    pub adapters: IndexMap<AdapterName, Json>,
}

/// Where a reflex comes from: a directory as written, resolved by the host, or a repository.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Location {
    Local {
        path: String,
    },
    Remote {
        reference: Reference,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        pin: Option<Version>,
    },
}

/// A repository and a directory in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reference {
    pub repo: Repo,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<RelPath>,
}

/// A repository on GitHub, or by git URL.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Repo {
    #[serde(rename = "github")]
    GitHub {
        owner: Owner,
        name: Segment,
    },
    Url {
        url: GitUrl,
    },
}

/// A git URL over an allow-listed scheme: `https` or `ssh`; a user may stand before an ssh host, as `git@` does,
/// never a password, and never anything before an https host, since a token there is a secret in a project file;
/// no `#`, whitespace or control character anywhere, and no `@` past the host.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct GitUrl(String);

impl GitUrl {
    pub fn new(text: &str) -> Result<Self, String> {
        Clean::line(text).map_err(|why| format!("the URL {why}"))?;
        let Some((scheme, rest)) = text.split_once("://") else {
            return Err(format!("\"{text}\" is not a URL"));
        };
        if !matches!(scheme, "https" | "ssh") {
            return Err(format!(
                "\"{text}\" uses scheme \"{scheme}\"; https and ssh are allowed"
            ));
        }
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let clean = |part: &str| !part.contains(|c: char| c.is_whitespace() || c == '#');
        let shaped = !authority.is_empty()
            && clean(authority)
            && authority.matches('@').count() <= 1
            && !authority.starts_with('@')
            && !authority.ends_with('@')
            && clean(path)
            && !path.contains('@');
        if !shaped {
            return Err(format!(
                "\"{text}\" is not a git URL: <scheme>://[<user>@]<host>/<path>"
            ));
        }
        if let Some((userinfo, _)) = authority.rsplit_once('@')
            && (userinfo.contains(':') || scheme == "https")
        {
            return Err(
                "the URL carries a credential; git's credential helper holds it, never a project file"
                    .to_owned(),
            );
        }
        Ok(Self(text.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for GitUrl {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::new(&text)
    }
}

impl fmt::Display for GitUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A tag `[v]X.Y.Z`, printed without the `v`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    pub fn parse(text: &str) -> Result<Self, String> {
        let digits = text.strip_prefix('v').unwrap_or(text);
        let part = |n: &str| {
            if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) {
                n.parse::<u32>().ok()
            } else {
                None
            }
        };
        let parts: Option<Vec<u32>> = digits.split('.').map(part).collect();
        match parts.as_deref() {
            Some(&[major, minor, patch]) => Ok(Self {
                major,
                minor,
                patch,
            }),
            _ => Err(format!("\"{text}\" is not a version: [v]X.Y.Z")),
        }
    }
}

impl TryFrom<String> for Version {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text)
    }
}

impl From<Version> for String {
    fn from(version: Version) -> Self {
        version.to_string()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A config value as you stored it: plain, or a reference to an environment variable.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Setting {
    Plain { value: String },
    Env { var: VarName },
}

/// What `add`, `update` and `remove` write whole and `sync` realises: the tool's version, the adapter that decided,
/// and every remote reflex at a tag, a commit and a digest, with the effect and the needs consented to. A local
/// reflex is never in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lock {
    pub evoke: Version,
    pub adapter: LockedAdapter,
    pub reflexes: IndexMap<LocalName, Locked>,
}

/// The adapter the lock was written under, by name and id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedAdapter {
    pub name: AdapterName,
    pub id: AdapterId,
}

/// One remote reflex as pinned; `needs` absent means none consented to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Locked {
    pub reference: Reference,
    pub tag: Version,
    pub commit: Commit,
    pub h1: Digest,
    pub effect: Effect,
    #[serde(default, skip_serializing_if = "Needs::is_none")]
    pub needs: Needs,
}

/// A commit as git printed it: 40 or 64 lowercase hex; compared, never parsed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct Commit(String);

impl Commit {
    pub fn new(text: &str) -> Result<Self, String> {
        let hex = matches!(text.len(), 40 | 64)
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if hex {
            Ok(Self(text.to_owned()))
        } else {
            Err(format!(
                "\"{text}\" is not a commit: 40 or 64 lowercase hex"
            ))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Commit {
    type Error = String;

    fn try_from(text: String) -> Result<Self, String> {
        Self::new(&text)
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for Location {
    /// As `evoke.toml` writes it: the directory, or the ref with its pin.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local { path } => f.write_str(path),
            Self::Remote {
                reference,
                pin: Some(pin),
            } => write!(f, "{reference}@{pin}"),
            Self::Remote {
                reference,
                pin: None,
            } => write!(f, "{reference}"),
        }
    }
}

impl fmt::Display for Reference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repo {
            Repo::GitHub { owner, name } => write!(f, "{owner}/{name}")?,
            Repo::Url { url } => write!(f, "{url}")?,
        }
        match (&self.repo, &self.dir) {
            (Repo::GitHub { .. }, Some(dir)) => write!(f, "/{dir}"),
            (Repo::Url { .. }, Some(dir)) => write!(f, "#{dir}"),
            (_, None) => Ok(()),
        }
    }
}

/// A ref as typed at `evoke add`: `owner/repo[/dir][@tag]` or `<url>[#dir][@tag]`.
pub fn reference(text: &str) -> Result<(Reference, Option<Version>), Diagnostic> {
    parse_reference(text).map_err(|message| Diagnostic {
        reflex: None,
        at: None,
        message,
        fix: Fix::Rerun,
    })
}

fn parse_reference(text: &str) -> Result<(Reference, Option<Version>), String> {
    if is_local(text) {
        return Err(format!("\"{text}\" is local; a ref names a repository"));
    }
    if text.starts_with('/') || text.starts_with('~') || text == "." || text == ".." {
        return Err(format!(
            "\"{text}\" is a path; a local reflex is written ./dir or ../dir, relative to evoke.toml"
        ));
    }
    if !text.contains("://") && text.contains(':') {
        return Err(format!("\"{text}\" is scp-like; write ssh://<host>/<path>"));
    }
    let (head, pin) = split_pin(text)?;
    if head.contains("://") {
        let (url, dir) = head
            .split_once('#')
            .map_or((head, None), |(url, dir)| (url, Some(dir)));
        let url = GitUrl::new(url)?;
        let dir = dir.map(RelPath::new).transpose()?;
        return Ok((
            Reference {
                repo: Repo::Url { url },
                dir,
            },
            pin,
        ));
    }
    let mut segments = head.split('/');
    let (Some(owner), Some(name)) = (segments.next(), segments.next()) else {
        return Err(format!(
            "\"{text}\" is not a ref: owner/repo[/dir][@tag] or <url>[#dir][@tag]"
        ));
    };
    let owner = Owner::new(owner)?;
    let name = Segment::new(name)?;
    let dir: Vec<Segment> = segments.map(Segment::new).collect::<Result<_, _>>()?;
    let dir = match dir.as_slice() {
        [] => None,
        dir => Some(RelPath::new(
            &dir.iter()
                .map(Segment::as_str)
                .collect::<Vec<_>>()
                .join("/"),
        )?),
    };
    Ok((
        Reference {
            repo: Repo::GitHub { owner, name },
            dir,
        },
        pin,
    ))
}

/// The pin after the last `@`, when there is one; in a URL an `@` before the first `/` of the path names a user,
/// and is left to the URL.
fn split_pin(text: &str) -> Result<(&str, Option<Version>), String> {
    let Some((head, tail)) = text.rsplit_once('@') else {
        return Ok((text, None));
    };
    match Version::parse(tail) {
        Ok(pin) => Ok((head, Some(pin))),
        Err(_) if before_the_path(text, head.len()) => Ok((text, None)),
        Err(_) => Err(format!(
            "\"{text}\": what follows @ must be a version, [v]X.Y.Z"
        )),
    }
}

/// Whether the byte at `at` sits in a URL's authority: after `://` and before the path's first `/`.
fn before_the_path(text: &str, at: usize) -> bool {
    let Some(scheme) = text.find("://") else {
        return false;
    };
    let authority = scheme + 3;
    let path = text[authority..]
        .find('/')
        .map_or(text.len(), |slash| authority + slash);
    at >= authority && at < path
}

fn is_local(text: &str) -> bool {
    text.starts_with("./") || text.starts_with("../")
}

fn location(text: &str) -> Result<Location, String> {
    if is_local(text) {
        Clean::line(text).map_err(|why| format!("\"{text}\" {why}"))?;
        return Ok(Location::Local {
            path: text.to_owned(),
        });
    }
    parse_reference(text).map(|(reference, pin)| Location::Remote { reference, pin })
}

/// A project from `evoke.toml`, or every line to fix.
pub fn project(doc: Document<'_>) -> Result<Project, Vec<Diagnostic>> {
    let mut d = Diagnostics::new(Some(&doc.file));
    let value = read(&mut d, document::root(doc)?);
    d.finish(value)
}

fn read(d: &mut Diagnostics, root: Node) -> Option<Project> {
    let mut top = d.table(root)?;
    let adapter = if let Some(node) = top.take("adapter") {
        d.str(&node).and_then(|text| match AdapterName::new(text) {
            Ok(name) => Some(name),
            Err(why) => {
                d.fail(node.at.as_ref(), format!("adapter: {why}"));
                None
            }
        })
    } else {
        d.fail(top.at.as_ref(), "adapter is required");
        None
    };
    let reflexes = top
        .take("reflexes")
        .map_or_else(IndexMap::new, |node| reflexes(d, node));
    let config = top
        .take("config")
        .map_or_else(IndexMap::new, |node| config(d, node));
    let adapters = top
        .take("adapters")
        .map_or_else(IndexMap::new, |node| adapters(d, node));
    for (key, node) in top.entries() {
        d.fail(
            node.at.as_ref(),
            format!("{key} is not a key of evoke.toml: adapter, reflexes, config, adapters"),
        );
    }
    Some(Project {
        adapter: adapter?,
        reflexes,
        config,
        adapters,
    })
}

fn reflexes(d: &mut Diagnostics, node: Node) -> IndexMap<LocalName, Location> {
    let mut reflexes = IndexMap::new();
    let Some(table) = d.table(node) else {
        return reflexes;
    };
    for (key, node) in table.entries() {
        let name = match LocalName::new(&key) {
            Ok(name) => name,
            Err(why) => {
                d.fail(node.at.as_ref(), format!("reflexes.{key}: {why}"));
                continue;
            }
        };
        let Some(text) = d.str(&node) else { continue };
        match location(text) {
            Ok(location) => {
                reflexes.insert(name, location);
            }
            Err(why) => d.fail(node.at.as_ref(), format!("reflexes.{key}: {why}")),
        }
    }
    reflexes
}

fn config(d: &mut Diagnostics, node: Node) -> IndexMap<LocalName, IndexMap<ConfigKey, Setting>> {
    let mut config = IndexMap::new();
    let Some(table) = d.table(node) else {
        return config;
    };
    for (key, node) in table.entries() {
        let name = match LocalName::new(&key) {
            Ok(name) => name,
            Err(why) => {
                d.fail(node.at.as_ref(), format!("config.{key}: {why}"));
                continue;
            }
        };
        let Some(table) = d.table(node) else { continue };
        let mut settings = IndexMap::new();
        for (key, node) in table.entries() {
            match ConfigKey::new(&key) {
                Ok(key) => {
                    if let Some(setting) = setting(d, node) {
                        settings.insert(key, setting);
                    }
                }
                Err(why) => d.fail(node.at.as_ref(), format!("config.{name}.{key}: {why}")),
            }
        }
        config.insert(name, settings);
    }
    config
}

fn setting(d: &mut Diagnostics, node: Node) -> Option<Setting> {
    if let Value::Str(value) = &node.value
        && let Err(why) = Clean::line(value)
    {
        d.fail(node.at.as_ref(), format!("{} {why}", node.name()));
        return None;
    }
    match node.value {
        Value::Str(value) => Some(Setting::Plain { value }),
        Value::Table(_) => {
            let mut table = d.table(node)?;
            let var = if let Some(node) = table.take("env") {
                d.str(&node).and_then(|text| match VarName::new(text) {
                    Ok(var) => Some(var),
                    Err(why) => {
                        d.fail(node.at.as_ref(), format!("{}: {why}", node.path));
                        None
                    }
                })
            } else {
                d.fail(table.at.as_ref(), format!("{}.env is required", table.path));
                None
            };
            for (_, node) in table.entries() {
                d.fail(
                    node.at.as_ref(),
                    format!(
                        "{} is unknown; a setting is a string or {{ env }}",
                        node.path
                    ),
                );
            }
            Some(Setting::Env { var: var? })
        }
        _ => {
            d.fail(
                node.at.as_ref(),
                format!("{} must be a string or {{ env = \"VAR\" }}", node.name()),
            );
            None
        }
    }
}

fn adapters(d: &mut Diagnostics, node: Node) -> IndexMap<AdapterName, Json> {
    let mut adapters = IndexMap::new();
    let Some(table) = d.table(node) else {
        return adapters;
    };
    for (key, node) in table.entries() {
        match AdapterName::new(&key) {
            Ok(name) if matches!(node.value, Value::Table(_)) => {
                adapters.insert(name, node.json());
            }
            Ok(_) => d.fail(node.at.as_ref(), format!("adapters.{key} must be a table")),
            Err(why) => d.fail(node.at.as_ref(), format!("adapters.{key}: {why}")),
        }
    }
    adapters
}

/// The lock from `evoke.lock`, or every line to fix.
pub fn lock(doc: Document<'_>) -> Result<Lock, Vec<Diagnostic>> {
    let mut d = Diagnostics::new(Some(&doc.file));
    let value = read_lock(&mut d, document::root(doc)?);
    d.finish(value)
}

fn read_lock(d: &mut Diagnostics, root: Node) -> Option<Lock> {
    let mut top = d.table(root)?;
    match top.take("lock") {
        Some(node) => match node.integer() {
            Some(1) => {}
            Some(other) => d.fail(node.at.as_ref(), format!("lock must be 1, not {other}")),
            None => d.fail(node.at.as_ref(), "lock must be 1"),
        },
        None => d.fail(top.at.as_ref(), "lock = 1 is required"),
    }
    let evoke = if let Some(node) = top.take("evoke") {
        d.str(&node).and_then(|text| match Version::parse(text) {
            Ok(version) => Some(version),
            Err(why) => {
                d.fail(node.at.as_ref(), format!("evoke: {why}"));
                None
            }
        })
    } else {
        d.fail(top.at.as_ref(), "evoke is required");
        None
    };
    let adapter = if let Some(node) = top.take("adapter") {
        locked_adapter(d, node)
    } else {
        d.fail(top.at.as_ref(), "adapter is required");
        None
    };
    let reflexes = top
        .take("reflexes")
        .map_or_else(IndexMap::new, |node| locked_reflexes(d, node));
    for (key, node) in top.entries() {
        d.fail(
            node.at.as_ref(),
            format!("{key} is not a key of evoke.lock: lock, evoke, adapter, reflexes"),
        );
    }
    Some(Lock {
        evoke: evoke?,
        adapter: adapter?,
        reflexes,
    })
}

fn locked_adapter(d: &mut Diagnostics, node: Node) -> Option<LockedAdapter> {
    let mut table = d.table(node)?;
    let name = if let Some(node) = table.take("name") {
        d.str(&node).and_then(|text| match AdapterName::new(text) {
            Ok(name) => Some(name),
            Err(why) => {
                d.fail(node.at.as_ref(), format!("adapter.name: {why}"));
                None
            }
        })
    } else {
        d.fail(table.at.as_ref(), "adapter.name is required");
        None
    };
    let id = if let Some(node) = table.take("id") {
        d.str(&node).and_then(|text| match AdapterId::new(text) {
            Ok(id) => Some(id),
            Err(why) => {
                d.fail(node.at.as_ref(), format!("adapter.id: {why}"));
                None
            }
        })
    } else {
        d.fail(table.at.as_ref(), "adapter.id is required");
        None
    };
    for (_, node) in table.entries() {
        d.fail(
            node.at.as_ref(),
            format!("{} is not a key of adapter: name, id", node.path),
        );
    }
    Some(LockedAdapter {
        name: name?,
        id: id?,
    })
}

fn locked_reflexes(d: &mut Diagnostics, node: Node) -> IndexMap<LocalName, Locked> {
    let mut reflexes = IndexMap::new();
    let Some(table) = d.table(node) else {
        return reflexes;
    };
    for (key, node) in table.entries() {
        let name = match LocalName::new(&key) {
            Ok(name) => name,
            Err(why) => {
                d.fail(node.at.as_ref(), format!("reflexes.{key}: {why}"));
                continue;
            }
        };
        if let Some(locked) = locked(d, node) {
            reflexes.insert(name, locked);
        }
    }
    reflexes
}

/// One `[reflexes.<name>]` of the lock: every key required, the ref without a pin.
fn locked(d: &mut Diagnostics, node: Node) -> Option<Locked> {
    let mut table = d.table(node)?;
    let path = table.path.clone();
    let required = |d: &mut Diagnostics, table: &mut Table, key: &str| -> Option<Node> {
        let node = table.take(key);
        if node.is_none() {
            d.fail(table.at.as_ref(), format!("{path}.{key} is required"));
        }
        node
    };
    let reference = required(d, &mut table, "ref").and_then(|node| {
        let text = d.str(&node)?;
        match parse_reference(text) {
            Ok((reference, None)) => Some(reference),
            Ok((_, Some(_))) => {
                d.fail(
                    node.at.as_ref(),
                    format!("{path}.ref: \"{text}\" carries a pin; the lock's tag is the version"),
                );
                None
            }
            Err(why) => {
                d.fail(node.at.as_ref(), format!("{path}.ref: {why}"));
                None
            }
        }
    });
    let tag = required(d, &mut table, "tag").and_then(|node| {
        let text = d.str(&node)?;
        match Version::parse(text) {
            Ok(tag) => Some(tag),
            Err(why) => {
                d.fail(node.at.as_ref(), format!("{path}.tag: {why}"));
                None
            }
        }
    });
    let commit = required(d, &mut table, "commit").and_then(|node| {
        let text = d.str(&node)?;
        match Commit::new(text) {
            Ok(commit) => Some(commit),
            Err(why) => {
                d.fail(node.at.as_ref(), format!("{path}.commit: {why}"));
                None
            }
        }
    });
    let h1 = required(d, &mut table, "h1").and_then(|node| {
        let text = d.str(&node)?;
        match Digest::try_from(text.to_owned()) {
            Ok(h1) => Some(h1),
            Err(why) => {
                d.fail(node.at.as_ref(), format!("{path}.h1: {why}"));
                None
            }
        }
    });
    let effect = required(d, &mut table, "effect").and_then(|node| manifest::effect(d, &node));
    let needs = table.take("needs").map_or_else(Needs::default, |node| {
        let mut unknown = Vec::new();
        let at = node.at.clone();
        let needs = needs::read(d, node, &mut unknown, None);
        for key in unknown {
            d.fail(
                at.as_ref(),
                format!("{key} is not a key of {path}.needs: reads, writes, hosts, runs"),
            );
        }
        needs
    });
    for (_, node) in table.entries() {
        d.fail(
            node.at.as_ref(),
            format!(
                "{} is not a key of {path}: ref, tag, commit, h1, effect, needs",
                node.path
            ),
        );
    }
    Some(Locked {
        reference: reference?,
        tag: tag?,
        commit: commit?,
        h1: h1?,
        effect: effect?,
        needs,
    })
}

/// `evoke.lock` as `add`, `update` and `remove` write it, whole: the keys of each table aligned.
#[must_use]
pub fn render_lock(lock: &Lock) -> String {
    let mut text = String::from(
        "# Written by evoke; evoke sync realises it on another machine and never changes it.\n",
    );
    text.push_str(&aligned(&[
        ("lock", "1".to_owned()),
        ("evoke", quoted(&lock.evoke.to_string())),
    ]));
    text.push_str("\n[adapter]\n");
    text.push_str(&aligned(&[
        ("name", quoted(lock.adapter.name.as_str())),
        ("id", quoted(lock.adapter.id.as_str())),
    ]));
    for (name, locked) in &lock.reflexes {
        let _ = write!(text, "\n[reflexes.{name}]\n");
        let mut pairs = vec![
            ("ref", quoted(&locked.reference.to_string())),
            ("tag", quoted(&locked.tag.to_string())),
            ("commit", quoted(locked.commit.as_str())),
            ("h1", quoted(&locked.h1.to_string())),
            ("effect", quoted(&locked.effect.to_string())),
        ];
        if !locked.needs.is_none() {
            pairs.push(("needs", inline_needs(&locked.needs)));
        }
        text.push_str(&aligned(&pairs));
    }
    text
}

/// `{ writes = ["{to}", "~/Downloads"], hosts = ["*"] }`: the declaration as one inline table, empty lists left out.
fn inline_needs(needs: &Needs) -> String {
    let list = |items: Vec<String>| {
        format!(
            "[{}]",
            items
                .iter()
                .map(|item| quoted(item))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let mut pairs = Vec::new();
    if !needs.reads.is_empty() {
        pairs.push(format!(
            "reads = {}",
            list(needs.reads.iter().map(ToString::to_string).collect())
        ));
    }
    if !needs.writes.is_empty() {
        pairs.push(format!(
            "writes = {}",
            list(needs.writes.iter().map(ToString::to_string).collect())
        ));
    }
    if needs.hosts == Hosts::Any {
        pairs.push("hosts = [\"*\"]".to_owned());
    }
    if !needs.runs.is_empty() {
        pairs.push(format!(
            "runs = {}",
            list(needs.runs.iter().map(ToString::to_string).collect())
        ));
    }
    format!("{{ {} }}", pairs.join(", "))
}

/// `key = value` lines with the `=` aligned.
fn aligned(pairs: &[(&str, String)]) -> String {
    let width = pairs.iter().map(|(key, _)| key.len()).max().unwrap_or(0);
    pairs.iter().fold(String::new(), |mut text, (key, value)| {
        let _ = writeln!(text, "{key:<width$} = {value}");
        text
    })
}

/// A TOML basic string.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::File;
    use crate::document::Text;

    fn parse(text: &str) -> Result<Project, Vec<Diagnostic>> {
        project(Document {
            file: File::Project,
            text: Text::Toml(text),
        })
    }

    fn github(text: &str) -> String {
        let (reference, pin) = reference(text).unwrap();
        format!(
            "{reference} @ {}",
            pin.map_or("unpinned".to_owned(), |pin| pin.to_string())
        )
    }

    #[test]
    fn refs_take_both_forms_with_a_dir_and_a_pin() {
        assert_eq!(github("radhi/home/lights"), "radhi/home/lights @ unpinned");
        assert_eq!(github("radhi/timer@v1.0.1"), "radhi/timer @ 1.0.1");
        assert_eq!(github("radhi/home/a/b@2.0.0"), "radhi/home/a/b @ 2.0.0");
        assert_eq!(
            github("https://example.com/r.git#lights@1.2.0"),
            "https://example.com/r.git#lights @ 1.2.0"
        );
        assert_eq!(
            github("ssh://example.com/r"),
            "ssh://example.com/r @ unpinned"
        );
        assert_eq!(
            github("ssh://git@github.com/radhi/home#lights@1.2.0"),
            "ssh://git@github.com/radhi/home#lights @ 1.2.0"
        );
        assert_eq!(
            github("ssh://git@github.com/radhi/home"),
            "ssh://git@github.com/radhi/home @ unpinned"
        );
        let (reference, _) = reference("radhi/home/lights").unwrap();
        assert_eq!(reference.dir.as_ref().map(RelPath::as_str), Some("lights"));
    }

    #[test]
    fn refs_refuse_what_the_grammar_excludes() {
        let refused = |text: &str| reference(text).unwrap_err();
        assert_eq!(refused("./lights").fix, Fix::Rerun);
        assert_eq!(
            refused("radhi").message,
            "\"radhi\" is not a ref: owner/repo[/dir][@tag] or <url>[#dir][@tag]"
        );
        assert_eq!(
            refused("radhi/home@1.2").message,
            "\"radhi/home@1.2\": what follows @ must be a version, [v]X.Y.Z"
        );
        assert_eq!(
            refused("git@github.com:radhi/home").message,
            "\"git@github.com:radhi/home\" is scp-like; write ssh://<host>/<path>"
        );
        assert_eq!(
            refused("https://example.com/a@b/r").message,
            "\"https://example.com/a@b/r\": what follows @ must be a version, [v]X.Y.Z"
        );
        assert!(
            refused("ssh://git@@github.com/r")
                .message
                .contains("is not a git URL")
        );
        assert_eq!(
            refused("example.com:radhi/home").message,
            "\"example.com:radhi/home\" is scp-like; write ssh://<host>/<path>"
        );
        assert!(
            refused("git://example.com/r")
                .message
                .contains("https and ssh are allowed")
        );
        assert!(
            refused("radhi/--as")
                .message
                .contains("is not a ref segment")
        );
        assert!(
            refused("-radhi/home")
                .message
                .contains("is not a GitHub owner")
        );
        assert!(
            refused("https://example.com/r#../x")
                .message
                .contains("leaves the reflex directory")
        );
    }

    #[test]
    fn versions_parse_and_order() {
        assert!(Version::parse("1.2.0").unwrap() < Version::parse("v2.0.0").unwrap());
        assert!(Version::parse("1.10.0").unwrap() > Version::parse("1.9.9").unwrap());
        assert!(Version::parse("1.2").is_err());
        assert!(Version::parse("1.+2.0").is_err());
        assert_eq!(
            serde_json::to_string(&Version::parse("v1.2.0").unwrap()).unwrap(),
            "\"1.2.0\""
        );
    }

    #[test]
    fn the_home_project_reads_typed() {
        let text = "adapter = \"replay\"\n\n[reflexes]\nlights = \"./lights\"\ntimer  = \"radhi/timer@1.0.1\"\n\n[config.lights]\nbridge = \"10.0.0.2\"\ntoken  = { env = \"HUE_TOKEN\" }\n\n[adapters.engine]\ngate = { write = 0.85 }\n";
        let project = parse(text).unwrap();
        assert_eq!(project.adapter.as_str(), "replay");
        assert_eq!(
            project.reflexes.get("lights"),
            Some(&Location::Local {
                path: "./lights".to_owned()
            })
        );
        assert!(matches!(
            project.reflexes.get("timer"),
            Some(Location::Remote { pin: Some(_), .. })
        ));
        assert_eq!(
            project.config["lights"]["token"],
            Setting::Env {
                var: VarName::new("HUE_TOKEN").unwrap()
            }
        );
        assert_eq!(
            project.adapters["engine"],
            serde_json::json!({ "gate": { "write": 0.85 } })
        );
        let json = serde_json::to_value(&project).unwrap();
        assert_eq!(
            json["reflexes"]["timer"]["reference"]["repo"],
            serde_json::json!({ "type": "github", "owner": "radhi", "name": "timer" })
        );
        assert_eq!(serde_json::from_value::<Project>(json).unwrap(), project);
        let repo = serde_json::json!({ "type": "github", "owner": "a.b", "name": "x" });
        assert!(serde_json::from_value::<Repo>(repo).is_err());
    }

    #[test]
    fn every_problem_is_a_line() {
        let text = "colour = 1\n\n[reflexes]\nfits = \"./x\"\nlights = \"radhi\"\n\n[config.lights]\ntoken = { var = \"X\" }\n";
        let errors: Vec<(u32, String)> = parse(text)
            .unwrap_err()
            .into_iter()
            .map(|e| (e.at.map_or(0, |at| at.line), e.message))
            .collect();
        assert_eq!(
            errors,
            [
                (1, "adapter is required".to_owned()),
                (1, "colour is not a key of evoke.toml: adapter, reflexes, config, adapters".to_owned()),
                (4, "reflexes.fits: \"fits\" is reserved".to_owned()),
                (5, "reflexes.lights: \"radhi\" is not a ref: owner/repo[/dir][@tag] or <url>[#dir][@tag]".to_owned()),
                (8, "config.lights.token.env is required".to_owned()),
                (8, "config.lights.token.var is unknown; a setting is a string or { env }".to_owned()),
            ]
        );
    }
}
