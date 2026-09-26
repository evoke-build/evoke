//! One call grammar: `lights room="den" state="off"`. In: a line as typed; a resolved `Call`. Out: a `Written` —
//! the names and values as typed, a bare argument a flag — or the diagnostic that refuses the line; a call's
//! one-line text.

use std::fmt;

use indexmap::IndexMap;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

use crate::diagnostic::{Diagnostic, Fix};
use crate::document::Json;
use crate::name::{ArgName, LocalName, OptionKey, Word};
use crate::propose::PickValue;
use crate::text::Span;

/// A call resolved and complete; on the wire `{ reflex, args, call }`, the last being its rendering.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Call {
    pub reflex: LocalName,
    pub args: IndexMap<ArgName, Value>,
}

impl Serialize for Call {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut call = serializer.serialize_struct("Call", 3)?;
        call.serialize_field("reflex", &self.reflex)?;
        call.serialize_field("args", &self.args)?;
        call.serialize_field("call", &render(self))?;
        call.end()
    }
}

/// An argument's value: a key, a word, a verbatim span, or a flag that is present. Never minted by the model.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Value {
    Option {
        key: OptionKey,
    },
    Word {
        word: Word,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },
    Pick {
        span: Span,
        value: PickValue,
    },
    Flag,
}

/// `~` or `~/…` as a path under the home; anything else as it is.
#[must_use]
pub fn expand_home(value: &str, home: &str) -> String {
    match value {
        "~" => home.to_owned(),
        _ => value
            .strip_prefix("~/")
            .map_or_else(|| value.to_owned(), |rest| format!("{home}/{rest}")),
    }
}

impl Value {
    /// The value as a body receives it: an option key, a word's `value` if set else the word, a pick's number,
    /// seconds or text, `true` for a flag.
    #[must_use]
    pub fn plain(&self) -> Json {
        match self {
            Self::Option { key } => Json::String(key.to_string()),
            Self::Word { word, value } => {
                Json::String(value.clone().unwrap_or_else(|| word.to_string()))
            }
            Self::Pick { value, .. } => match value {
                PickValue::Number { value } => Json::from(*value),
                PickValue::Seconds { value } => Json::from(*value),
                PickValue::Email { value }
                | PickValue::Url { value }
                | PickValue::Quoted { value } => Json::String(value.to_string()),
            },
            Self::Flag => Json::Bool(true),
        }
    }

    /// The value as a body receives it, a word's `value` that names a path under the home expanded: a vocabulary
    /// keeps a path as a person writes it, `~/Desktop`, and the body, an argv and the declaration see one path.
    #[must_use]
    pub fn under_home(&self, home: &str) -> Json {
        match self {
            Self::Word {
                value: Some(value), ..
            } => Json::String(expand_home(value, home)),
            _ => self.plain(),
        }
    }

    /// The value as a person would type it; a flag has none.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Option { key } => Some(key.as_str()),
            Self::Word { word, .. } => Some(word.as_str()),
            Self::Pick { span, .. } => Some(span.text().as_str()),
            Self::Flag => None,
        }
    }
}

/// The call on one line: the name, then each argument as `name="value"` or a bare flag.
#[must_use]
pub fn render(call: &Call) -> String {
    let mut text = call.reflex.to_string();
    for (name, value) in &call.args {
        text.push(' ');
        text.push_str(name.as_str());
        if let Some(value) = value.text() {
            text.push('=');
            text.push_str(&quoted(value));
        }
    }
    text
}

/// A call as typed: `lights room=den state=off`; a bare argument is a flag. Resolved by `by_name` or typed by a
/// lesson, never run as it is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Written {
    pub reflex: LocalName,
    pub args: IndexMap<ArgName, Option<String>>,
}

impl fmt::Display for Written {
    /// The line as the grammar reads it back: a value bare where it can be, else a JSON string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reflex.as_str())?;
        for (arg, value) in &self.args {
            write!(f, " {arg}")?;
            if let Some(value) = value {
                let bare = !value.is_empty()
                    && !value.contains(|c: char| c.is_whitespace() || c == '"' || c == '=');
                if bare {
                    write!(f, "={value}")?;
                } else {
                    write!(f, "={}", quoted(value))?;
                }
            }
        }
        Ok(())
    }
}

/// The grammar's one form: a value is bare, `[^\s"=]+`, or a JSON string.
const GRAMMAR: &str = "<name> (<arg>=<value> | <flag>)*";

/// A call parsed from one line; `evoke run`'s argv elements are joined by a space first.
pub fn call(text: &str) -> Result<Written, Diagnostic> {
    parse(text).map_err(|message| Diagnostic {
        reflex: None,
        at: None,
        message,
        fix: Fix::Rerun,
    })
}

fn parse(text: &str) -> Result<Written, String> {
    let (name, mut rest) = word(text.trim_start());
    if name.is_empty() {
        return Err(format!("a call is {GRAMMAR}"));
    }
    let reflex = LocalName::new(name)?;
    let mut args = IndexMap::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '=')
            .unwrap_or(rest.len());
        let (arg, after) = rest.split_at(end);
        let arg = ArgName::new(arg)?;
        let (value, after) = match after.strip_prefix('=') {
            None => (None, after),
            Some(after) => {
                let (value, after) = value(&arg, after)?;
                (Some(value), after)
            }
        };
        if args.insert(arg.clone(), value).is_some() {
            return Err(format!("{arg} is given twice"));
        }
        rest = after;
    }
    Ok(Written { reflex, args })
}

/// The text up to the first space, and the rest.
fn word(text: &str) -> (&str, &str) {
    text.split_at(text.find(char::is_whitespace).unwrap_or(text.len()))
}

/// A value after `=`: a JSON string, or bare text; a space or the end of the line ends it.
fn value<'t>(arg: &ArgName, text: &'t str) -> Result<(String, &'t str), String> {
    if let Some(quoted) = text.strip_prefix('"') {
        let mut escaped = false;
        let close = quoted.char_indices().find(|&(_, c)| {
            let closes = c == '"' && !escaped;
            escaped = c == '\\' && !escaped;
            closes
        });
        let Some((at, _)) = close else {
            return Err(format!("{arg}: the string is not closed"));
        };
        let (string, after) = text.split_at(at + 2);
        let value: String = serde_json::from_str(string)
            .map_err(|_| format!("{arg}: {string} is not a JSON string"))?;
        return ended(arg, value, after);
    }
    let end = text
        .find(|c: char| c.is_whitespace() || c == '"' || c == '=')
        .unwrap_or(text.len());
    if end == 0 {
        return Err(format!("{arg}= needs a value: bare text or a JSON string"));
    }
    let (bare, after) = text.split_at(end);
    ended(arg, bare.to_owned(), after)
}

fn ended<'t>(arg: &ArgName, value: String, after: &'t str) -> Result<(String, &'t str), String> {
    if after.is_empty() || after.starts_with(char::is_whitespace) {
        Ok((value, after))
    } else {
        Err(format!(
            "{arg}: a value is bare text or a JSON string, then a space"
        ))
    }
}

/// A JSON string: what the call grammar and the prompt quote with.
pub(crate) fn quoted(text: &str) -> String {
    Json::String(text.to_owned()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_call_renders_and_serializes_with_its_rendering() {
        let call = Call {
            reflex: LocalName::new("screenshot").unwrap(),
            args: [
                (
                    ArgName::new("area").unwrap(),
                    Value::Option {
                        key: OptionKey::new("window").unwrap(),
                    },
                ),
                (ArgName::new("clipboard").unwrap(), Value::Flag),
            ]
            .into_iter()
            .collect(),
        };
        assert_eq!(render(&call), "screenshot area=\"window\" clipboard");
        let json = serde_json::to_value(&call).unwrap();
        assert_eq!(json["call"], "screenshot area=\"window\" clipboard");
        assert_eq!(serde_json::from_value::<Call>(json).unwrap(), call);
    }

    #[test]
    fn quoted_wraps_and_escapes_its_text() {
        assert_eq!(quoted("say \"hi\""), "\"say \\\"hi\\\"\"");
    }

    fn written(text: &str) -> Vec<(String, Option<String>)> {
        call(text)
            .unwrap()
            .args
            .into_iter()
            .map(|(arg, value)| (arg.to_string(), value))
            .collect()
    }

    #[test]
    fn a_line_reads_as_typed() {
        let bare = call("lights room=den state=off").unwrap();
        assert_eq!(bare.reflex.as_str(), "lights");
        assert_eq!(
            written("  lights room=den   state=off "),
            [
                ("room".to_owned(), Some("den".to_owned())),
                ("state".to_owned(), Some("off".to_owned()))
            ]
        );
        assert_eq!(
            written("timer duration=\"10 minutes\" label=\"say \\\"hi\\\"\" quiet"),
            [
                ("duration".to_owned(), Some("10 minutes".to_owned())),
                ("label".to_owned(), Some("say \"hi\"".to_owned())),
                ("quiet".to_owned(), None)
            ]
        );
        assert!(call("lights").unwrap().args.is_empty());
        let line = "timer duration=\"10 minutes\" label=\"say \\\"hi\\\"\" quiet";
        assert_eq!(call(line).unwrap().to_string(), line);
        assert_eq!(bare.to_string(), "lights room=den state=off");
        let json = serde_json::to_value(call("screenshot area=window clipboard").unwrap()).unwrap();
        assert_eq!(
            json,
            serde_json::json!({ "reflex": "screenshot", "args": { "area": "window", "clipboard": null } })
        );
    }

    #[test]
    fn what_the_grammar_refuses() {
        let refused = |text: &str| call(text).unwrap_err().message;
        assert_eq!(refused(""), "a call is <name> (<arg>=<value> | <flag>)*");
        assert_eq!(
            refused("Lights"),
            "\"Lights\" is not a name: [a-z][a-z0-9_]*"
        );
        assert_eq!(
            refused("lights Room=den"),
            "\"Room\" is not a name: [a-z][a-z0-9_]*"
        );
        assert_eq!(
            refused("lights room=den room=office"),
            "room is given twice"
        );
        assert_eq!(
            refused("lights room="),
            "room= needs a value: bare text or a JSON string"
        );
        assert_eq!(
            refused("lights room=\"den"),
            "room: the string is not closed"
        );
        assert_eq!(
            refused("lights room=\"den\"x"),
            "room: a value is bare text or a JSON string, then a space"
        );
        assert_eq!(
            refused("lights room=den\"x"),
            "room: a value is bare text or a JSON string, then a space"
        );
        assert_eq!(
            refused("lights room=\"\\q\""),
            "room: \"\\q\" is not a JSON string"
        );
        assert_eq!(call("").unwrap_err().fix, Fix::Rerun);
    }
}
