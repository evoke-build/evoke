//! A file as the parsers walk it. In: which owned file and its text, TOML or JSON. Out: one spanned tree, so a
//! diagnostic lands on a line and an unknown key is seen, and the diagnostics a walk collects.

use std::fmt;

use serde::{Deserialize, Serialize};
use toml_edit::{Item, TableLike};

use crate::diagnostic::{At, Diagnostic, File, Fix};
use crate::name::LocalName;
use crate::text::Clean;

/// A JSON value that keeps its key order: the wire form of every boundary type.
pub type Json = serde_json::Value;

/// A file to parse: its role, and its text as written or as JSON.
#[derive(Clone, Debug, PartialEq)]
pub struct Document<'a> {
    pub file: File,
    pub text: Text<'a>,
}

/// The text of a document: TOML with lines, or JSON without.
#[derive(Clone, Debug, PartialEq)]
pub enum Text<'a> {
    Toml(&'a str),
    Json(Json),
}

/// What a text allows: a TOML file has lines and needs `run`; a JSON file may leave `run` to the SDK; a wire value
/// has no `reflex` key and carries what parsing found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Form {
    Toml,
    Json,
    Wire,
}

impl Text<'_> {
    pub(crate) fn form(&self) -> Form {
        match self {
            Self::Toml(_) => Form::Toml,
            Self::Json(_) => Form::Json,
        }
    }
}

/// The messages of a list, for a serde error.
pub(crate) fn summary(errors: &[Diagnostic]) -> String {
    errors
        .iter()
        .map(|error| error.message.as_str())
        .collect::<Vec<_>>()
        .join("; ")
}

/// Where in a file, by key: `args.state.ask`, `examples."kill the lights"`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyPath(Vec<String>);

impl KeyPath {
    #[must_use]
    pub fn new(segments: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self(segments.into_iter().map(Into::into).collect())
    }

    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    #[must_use]
    pub fn child(&self, key: &str) -> Self {
        let mut segments = self.0.clone();
        segments.push(key.to_owned());
        Self(segments)
    }
}

impl fmt::Display for KeyPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, segment) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(".")?;
            }
            let plain = segment
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'));
            if plain {
                f.write_str(segment)?;
            } else {
                let escaped = segment.replace('\\', "\\\\").replace('"', "\\\"");
                write!(f, "\"{escaped}\"")?;
            }
        }
        Ok(())
    }
}

/// One value of a document: where it is, what key path names it, and what it holds.
#[derive(Debug)]
pub(crate) struct Node {
    pub(crate) at: Option<At>,
    pub(crate) path: KeyPath,
    pub(crate) value: Value,
}

#[derive(Debug)]
pub(crate) enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Node>),
    Table(Vec<(String, Node)>),
    /// As written; JSON has no datetime, so it crosses as a string.
    Datetime(String),
    Other(&'static str),
}

impl Node {
    pub(crate) fn kind(&self) -> &'static str {
        match &self.value {
            Value::Str(_) => "a string",
            Value::Int(_) => "an integer",
            Value::Float(_) => "a number",
            Value::Bool(_) => "a boolean",
            Value::Array(_) => "an array",
            Value::Table(_) => "a table",
            Value::Datetime(_) => "a datetime",
            Value::Other(kind) => kind,
        }
    }

    /// The path, or `the document` at the root.
    pub(crate) fn name(&self) -> String {
        if self.path.0.is_empty() {
            "the document".to_owned()
        } else {
            self.path.to_string()
        }
    }

    pub(crate) fn str(&self) -> Option<&str> {
        match &self.value {
            Value::Str(text) => Some(text),
            _ => None,
        }
    }

    pub(crate) fn bool(&self) -> Option<bool> {
        match self.value {
            Value::Bool(b) => Some(b),
            _ => None,
        }
    }

    pub(crate) fn integer(&self) -> Option<i64> {
        match self.value {
            Value::Int(i) => Some(i),
            _ => None,
        }
    }

    /// A finite number: what JSON can carry.
    #[allow(clippy::cast_precision_loss)] // a TOML integer read as a number: the design's ranges are small
    pub(crate) fn number(&self) -> Option<f64> {
        match self.value {
            Value::Int(i) => Some(i as f64),
            Value::Float(f) if f.is_finite() => Some(f),
            _ => None,
        }
    }

    /// The value as JSON, for what the core keeps opaque.
    pub(crate) fn json(&self) -> Json {
        match &self.value {
            Value::Str(text) | Value::Datetime(text) => Json::String(text.clone()),
            Value::Int(i) => Json::from(*i),
            Value::Float(f) => Json::from(*f),
            Value::Bool(b) => Json::Bool(*b),
            Value::Array(items) => Json::Array(items.iter().map(Node::json).collect()),
            Value::Table(entries) => entries
                .iter()
                .map(|(key, node)| (key.clone(), node.json()))
                .collect(),
            Value::Other(_) => Json::Null,
        }
    }
}

/// A table's entries in file order; what is not taken is unknown.
pub(crate) struct Table {
    pub(crate) at: Option<At>,
    pub(crate) path: KeyPath,
    entries: Vec<(String, Node)>,
}

impl Table {
    pub(crate) fn take(&mut self, key: &str) -> Option<Node> {
        let index = self.entries.iter().position(|(k, _)| k == key)?;
        Some(self.entries.remove(index).1)
    }

    /// Every listed key present, in file order.
    pub(crate) fn take_any(&mut self, keys: &[&'static str]) -> Vec<(&'static str, Node)> {
        let mut taken = Vec::new();
        let mut i = 0;
        while i < self.entries.len() {
            match keys.iter().find(|&&key| key == self.entries[i].0) {
                Some(&key) => taken.push((key, self.entries.remove(i).1)),
                None => i += 1,
            }
        }
        taken
    }

    pub(crate) fn entries(self) -> Vec<(String, Node)> {
        self.entries
    }

    /// The keys in file order, taken or not.
    pub(crate) fn keys(&self) -> Vec<String> {
        self.entries.iter().map(|(key, _)| key.clone()).collect()
    }

    /// The paths of what was not taken.
    pub(crate) fn unknown(&self) -> Vec<KeyPath> {
        self.entries
            .iter()
            .map(|(_, node)| node.path.clone())
            .collect()
    }
}

/// The tree of a document; a TOML syntax error is one diagnostic at its line.
pub(crate) fn root(doc: Document<'_>) -> Result<Node, Vec<Diagnostic>> {
    match doc.text {
        Text::Toml(text) => {
            let lines = Lines::new(text, &doc.file);
            match toml_edit::Document::parse(text.to_owned()) {
                Ok(parsed) => Ok(from_toml(
                    parsed.as_item(),
                    &lines,
                    lines.at(Some(0..0)),
                    KeyPath::default(),
                )),
                Err(error) => {
                    let at = lines.at(error.span().or(Some(0..0)));
                    Err(vec![Diagnostic {
                        reflex: doc.file.reflex().cloned(),
                        at: at.clone(),
                        message: error.message().to_owned(),
                        fix: at.map_or(Fix::Check, |at| Fix::EditLine { at }),
                    }])
                }
            }
        }
        Text::Json(json) => Ok(from_json(&json, KeyPath::default())),
    }
}

/// The tree of a wire value: no file, no lines.
pub(crate) fn wire(json: &Json) -> Node {
    from_json(json, KeyPath::default())
}

struct Lines<'a> {
    text: &'a str,
    file: &'a File,
    starts: Vec<usize>,
}

impl<'a> Lines<'a> {
    fn new(text: &'a str, file: &'a File) -> Self {
        let starts = std::iter::once(0)
            .chain(text.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Self { text, file, starts }
    }

    /// The line a span starts on, at the line's first non-blank character: the fix is the line.
    fn at(&self, span: Option<std::ops::Range<usize>>) -> Option<At> {
        let start = span?.start.min(self.text.len());
        let line = self.starts.partition_point(|&s| s <= start);
        let rest = &self.text[self.starts[line - 1]..];
        let column = rest
            .chars()
            .take_while(|c| *c != '\n' && c.is_whitespace())
            .count()
            + 1;
        let file = self.file.clone();
        Some(At {
            file,
            line: saturating(line),
            column: saturating(column),
        })
    }
}

fn saturating(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

fn from_toml(item: &Item, lines: &Lines<'_>, at: Option<At>, path: KeyPath) -> Node {
    let value = match item {
        Item::None => Value::Other("nothing"),
        Item::Value(value) => return from_toml_value(value, lines, at, path),
        Item::Table(table) => toml_table(table, lines, &path),
        Item::ArrayOfTables(tables) => Value::Array(
            tables
                .iter()
                .map(|table| Node {
                    at: lines.at(table.span()),
                    path: path.clone(),
                    value: toml_table(table, lines, &path),
                })
                .collect(),
        ),
    };
    Node { at, path, value }
}

fn from_toml_value(
    value: &toml_edit::Value,
    lines: &Lines<'_>,
    at: Option<At>,
    path: KeyPath,
) -> Node {
    use toml_edit::Value as V;
    let value = match value {
        V::String(s) => Value::Str(s.value().clone()),
        V::Integer(i) => Value::Int(*i.value()),
        V::Float(f) => Value::Float(*f.value()),
        V::Boolean(b) => Value::Bool(*b.value()),
        V::Datetime(datetime) => Value::Datetime(datetime.value().to_string()),
        V::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| from_toml_value(item, lines, lines.at(item.span()), path.clone()))
                .collect(),
        ),
        V::InlineTable(table) => toml_table(table, lines, &path),
    };
    Node { at, path, value }
}

fn toml_table(table: &dyn TableLike, lines: &Lines<'_>, path: &KeyPath) -> Value {
    Value::Table(
        table
            .iter()
            .map(|(key, item)| {
                let at = lines.at(table.key(key).and_then(toml_edit::Key::span));
                (key.to_owned(), from_toml(item, lines, at, path.child(key)))
            })
            .collect(),
    )
}

fn from_json(json: &Json, path: KeyPath) -> Node {
    let value = match json {
        Json::Null => Value::Other("null"),
        Json::Bool(b) => Value::Bool(*b),
        Json::Number(n) => n.as_i64().map_or_else(
            || n.as_f64().map_or(Value::Other("a number"), Value::Float),
            Value::Int,
        ),
        Json::String(s) => Value::Str(s.clone()),
        Json::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| from_json(item, path.clone()))
                .collect(),
        ),
        Json::Object(entries) => Value::Table(
            entries
                .iter()
                .map(|(key, item)| (key.clone(), from_json(item, path.child(key))))
                .collect(),
        ),
    };
    Node {
        at: None,
        path,
        value,
    }
}

/// What a walk found wrong, each with its fix: the line when there is one, else `evoke check`.
pub(crate) struct Diagnostics {
    reflex: Option<LocalName>,
    list: Vec<Diagnostic>,
}

impl Diagnostics {
    pub(crate) fn new(file: Option<&File>) -> Self {
        Self {
            reflex: file.and_then(File::reflex).cloned(),
            list: Vec::new(),
        }
    }

    pub(crate) fn fail(&mut self, at: Option<&At>, message: impl Into<String>) {
        let at = at.cloned();
        let fix = at.clone().map_or(Fix::Check, |at| Fix::EditLine { at });
        self.list.push(Diagnostic {
            reflex: self.reflex.clone(),
            at,
            message: message.into(),
            fix,
        });
    }

    pub(crate) fn table(&mut self, node: Node) -> Option<Table> {
        if let Value::Table(entries) = node.value {
            Some(Table {
                at: node.at,
                path: node.path,
                entries,
            })
        } else {
            self.fail(node.at.as_ref(), format!("{} must be a table", node.name()));
            None
        }
    }

    pub(crate) fn array(&mut self, node: Node) -> Option<Vec<Node>> {
        if let Value::Array(items) = node.value {
            Some(items)
        } else {
            self.fail(
                node.at.as_ref(),
                format!("{} must be an array", node.name()),
            );
            None
        }
    }

    pub(crate) fn str<'n>(&mut self, node: &'n Node) -> Option<&'n str> {
        let text = node.str();
        if text.is_none() {
            self.fail(
                node.at.as_ref(),
                format!("{} must be a string", node.name()),
            );
        }
        text
    }

    pub(crate) fn bool(&mut self, node: &Node) -> Option<bool> {
        let flag = node.bool();
        if flag.is_none() {
            self.fail(
                node.at.as_ref(),
                format!("{} must be true or false", node.name()),
            );
        }
        flag
    }

    /// A clean string; line feeds allowed.
    pub(crate) fn clean(&mut self, node: &Node) -> Option<Clean> {
        let text = self.str(node)?;
        match Clean::new(text) {
            Ok(clean) => Some(clean),
            Err(why) => {
                self.fail(node.at.as_ref(), format!("{} {why}", node.name()));
                None
            }
        }
    }

    /// One non-empty line of clean text.
    pub(crate) fn line(&mut self, node: &Node) -> Option<Clean> {
        let text = self.str(node)?;
        match Clean::line(text) {
            Ok(line) => Some(line),
            Err(why) => {
                self.fail(node.at.as_ref(), format!("{} {why}", node.name()));
                None
            }
        }
    }

    /// The value when nothing failed, else every diagnostic in line order.
    pub(crate) fn finish<T>(mut self, value: Option<T>) -> Result<T, Vec<Diagnostic>> {
        debug_assert!(
            value.is_some() || !self.list.is_empty(),
            "a missing value without a diagnostic"
        );
        match value {
            Some(value) if self.list.is_empty() => Ok(value),
            _ => {
                self.list
                    .sort_by_key(|d| d.at.as_ref().map(|at| (at.line, at.column)));
                Err(self.list)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_file() -> File {
        File::Manifest {
            name: LocalName::new("lights").unwrap(),
        }
    }

    #[test]
    fn keys_carry_their_line_and_column() {
        let text = "a = 1\n\n[args.state]\nask = \"x\"\noptions.on = \"y\"\noptions.off = \"y\"\n  bad = \"z\"\n";
        let doc = Document {
            file: manifest_file(),
            text: Text::Toml(text),
        };
        let mut d = Diagnostics::new(Some(&doc.file));
        let mut top = d.table(root(doc).unwrap()).unwrap();
        assert_eq!(top.at.as_ref().map(|at| (at.line, at.column)), Some((1, 1)));
        let mut args = d.table(top.take("args").unwrap()).unwrap();
        let mut state = d.table(args.take("state").unwrap()).unwrap();
        assert_eq!(state.at.as_ref().map(|at| at.line), Some(3));
        let mut options = d.table(state.take("options").unwrap()).unwrap();
        assert_eq!(options.at.as_ref().map(|at| at.line), Some(5));
        assert_eq!(options.take("on").unwrap().at.map(|at| at.line), Some(5));
        let bad = state.take("bad").unwrap();
        assert_eq!(bad.at.as_ref().map(|at| (at.line, at.column)), Some((7, 3)));
        assert_eq!(
            options
                .take("off")
                .and_then(|off| off.at)
                .map(|at| (at.line, at.column)),
            Some((6, 1))
        );
        assert_eq!(bad.path.to_string(), "args.state.bad");
        assert!(d.finish(Some(())).is_ok());
        assert_eq!(
            KeyPath::new(["examples", "say \"hi\"", "a.b"]).to_string(),
            "examples.\"say \\\"hi\\\"\".\"a.b\""
        );
    }

    #[test]
    fn a_datetime_crosses_as_its_text() {
        let doc = Document {
            file: File::Project,
            text: Text::Toml("since = 2026-09-20\n"),
        };
        let mut d = Diagnostics::new(Some(&doc.file));
        let mut top = d.table(root(doc).unwrap()).unwrap();
        let since = top.take("since").unwrap();
        assert_eq!(since.json(), serde_json::json!("2026-09-20"));
        assert_eq!(since.kind(), "a datetime");
        assert!(since.str().is_none());
    }

    #[test]
    fn a_syntax_error_is_one_diagnostic_on_its_line() {
        let doc = Document {
            file: manifest_file(),
            text: Text::Toml("a = 1\nb = \n"),
        };
        let error = root(doc).unwrap_err().remove(0);
        assert_eq!(error.at.as_ref().map(|at| at.line), Some(2));
        assert_eq!(error.reflex.as_ref().map(LocalName::as_str), Some("lights"));
        assert!(matches!(error.fix, Fix::EditLine { .. }));
    }

    #[test]
    fn json_has_no_lines_and_fixes_with_check() {
        let doc = Document {
            file: File::Project,
            text: Text::Json(serde_json::json!({ "a": [1, 2.5, "x"] })),
        };
        let mut d = Diagnostics::new(Some(&doc.file));
        let mut top = d.table(root(doc).unwrap()).unwrap();
        let a = top.take("a").unwrap();
        assert_eq!(a.json(), serde_json::json!([1, 2.5, "x"]));
        d.str(&a);
        let errors = d.finish(Some(())).unwrap_err();
        assert_eq!(errors[0].message, "a must be a string");
        assert_eq!(errors[0].fix, Fix::Check);
        assert_eq!(errors[0].at, None);
    }
}
