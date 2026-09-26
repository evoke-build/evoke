//! Every name grammar as a newtype, and the reserved words. In: `&str`. Out: a name that holds its grammar, or why not.

use std::borrow::Borrow;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::text::Clean;

/// Never a local name, an option key or a vocabulary word: the sentinels every choice carries.
pub const RESERVED: [&str; 2] = ["none", "unstated"];

const JAVASCRIPT: [&str; 48] = [
    "arguments",
    "await",
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "eval",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "implements",
    "import",
    "in",
    "instanceof",
    "interface",
    "let",
    "new",
    "null",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "static",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "yield",
];

macro_rules! name {
    ($(#[$doc:meta])* $Name:ident, $check:expr) => {
        $(#[$doc])*
        #[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(try_from = "String")]
        pub struct $Name(String);

        impl $Name {
            /// The name, or the reason it is not one: a fragment starting with the quoted value.
            pub fn new(text: &str) -> Result<Self, String> {
                $check(text).map(|()| Self(text.to_owned()))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<String> for $Name {
            type Error = String;

            fn try_from(text: String) -> Result<Self, String> {
                Self::new(&text)
            }
        }

        impl Borrow<str> for $Name {
            fn borrow(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $Name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

name!(
    /// The `[reflexes]` key, the overlay's file name, a route option: a name, never reserved, never `fits` or
    /// `weave` — the heads of the question ids that are no reflex's.
    LocalName,
    |s: &str| {
        name(s)?;
        if RESERVED.contains(&s) || s == "fits" || s == "weave" { Err(format!("\"{s}\" is reserved")) } else { Ok(()) }
    }
);
name!(
    /// A question `evoke` asks on its own account, beside the plan's: `weave.<name>`.
    WeaveName,
    name
);
name!(
    /// An argument: a name a JavaScript body can destructure.
    ArgName,
    |s: &str| {
        name(s)?;
        if JAVASCRIPT.contains(&s) { Err(format!("\"{s}\" is a JavaScript reserved word")) } else { Ok(()) }
    }
);
name!(
    /// An option's key, which the body receives: one clean line, never reserved.
    OptionKey,
    |s: &str| {
        line(s)?;
        unreserved(s)
    }
);
name!(
    /// A vocabulary word: one clean line, trimmed, never reserved; spaces allowed.
    Word,
    |s: &str| {
        line(s)?;
        if s.trim() != s {
            return Err(format!("\"{s}\" has leading or trailing whitespace"));
        }
        unreserved(s)
    }
);
name!(
    /// A vocabulary: `vocab/<name>.toml`.
    VocabName,
    name
);
name!(
    /// A key under `[config]`.
    ConfigKey,
    name
);
name!(
    /// A field of a result's `data`, as `[yields]` names it.
    FieldName,
    name
);
name!(
    /// A scope for `--tag`.
    Tag,
    name
);
name!(
    /// An adapter, by the name it is resolved under.
    AdapterName,
    name
);
name!(
    /// What an adapter declares itself as; compared, never parsed.
    AdapterId,
    |s: &str| if s.is_empty() { Err("\"\" is empty".to_owned()) } else { Ok(()) }
);
name!(
    /// An environment variable.
    VarName,
    |s: &str| {
        let head = s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_');
        if head && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            Ok(())
        } else {
            Err(format!("\"{s}\" is not an environment variable name: [A-Za-z_][A-Za-z0-9_]*"))
        }
    }
);
name!(
    /// A path inside one reflex directory: relative, plain segments, no `..`.
    RelPath,
    |s: &str| {
        if s.starts_with('/') {
            return Err(format!("\"{s}\" is absolute; a path is relative to the reflex directory"));
        }
        if s.split('/').any(|segment| segment == "..") {
            return Err(format!("\"{s}\" leaves the reflex directory"));
        }
        if s.is_empty() || s.split('/').any(|segment| segment.is_empty() || segment == ".") {
            return Err(format!("\"{s}\" is not a plain relative path"));
        }
        // One clean line: a name a terminal shows as it is, and one `h1` lists without ambiguity.
        crate::text::Clean::line(s).map_err(|why| format!("\"{s}\" {why}"))?;
        Ok(())
    }
);
name!(
    /// A `{name}` in `[needs]`: an argument or a config key, whose value fills the entry at the decision.
    ValueName,
    name
);
name!(
    /// An absolute path as `[needs]` names one: `/`, or plain segments under it, no `.` or `..`, one clean line.
    AbsPath,
    |s: &str| {
        let Some(rest) = s.strip_prefix('/') else {
            return Err(format!("\"{s}\" is not an absolute path"));
        };
        if !rest.is_empty() && rest.split('/').any(|segment| segment.is_empty() || segment == "." || segment == "..") {
            return Err(format!("\"{s}\" is not a plain absolute path"));
        }
        crate::text::Clean::line(s).map_err(|why| format!("\"{s}\" {why}"))?;
        Ok(())
    }
);
name!(
    /// A GitHub owner, user or organization: `[A-Za-z0-9][A-Za-z0-9-]*`.
    Owner,
    |s: &str| {
        let head = s.starts_with(|c: char| c.is_ascii_alphanumeric());
        if head && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            Ok(())
        } else {
            Err(format!("\"{s}\" is not a GitHub owner: [A-Za-z0-9][A-Za-z0-9-]*"))
        }
    }
);
name!(
    /// One segment of a ref: never option-shaped.
    Segment,
    |s: &str| {
        let head = s.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '.' || c == '_');
        let dots = s == "." || s == "..";
        if head && !dots && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')) {
            Ok(())
        } else {
            Err(format!("\"{s}\" is not a ref segment: [A-Za-z0-9._][A-Za-z0-9._-]*"))
        }
    }
);

fn name(s: &str) -> Result<(), String> {
    let head = s.starts_with(|c: char| c.is_ascii_lowercase());
    if head
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        Ok(())
    } else {
        Err(format!("\"{s}\" is not a name: [a-z][a-z0-9_]*"))
    }
}

fn line(s: &str) -> Result<(), String> {
    Clean::line(s)
        .map(drop)
        .map_err(|why| format!("\"{s}\" {why}"))
}

fn unreserved(s: &str) -> Result<(), String> {
    if RESERVED.contains(&s) {
        Err(format!("\"{s}\" is reserved"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammars() {
        assert!(LocalName::new("lights_2").is_ok());
        assert_eq!(
            LocalName::new("Lights").unwrap_err(),
            "\"Lights\" is not a name: [a-z][a-z0-9_]*"
        );
        assert_eq!(LocalName::new("fits").unwrap_err(), "\"fits\" is reserved");
        assert_eq!(
            LocalName::new("weave").unwrap_err(),
            "\"weave\" is reserved"
        );
        assert!(WeaveName::new("split_0").is_ok());
        assert!(WeaveName::new("split.0").is_err());
        assert_eq!(
            ArgName::new("for").unwrap_err(),
            "\"for\" is a JavaScript reserved word"
        );
        assert!(ArgName::new("fits").is_ok());
        assert_eq!(OptionKey::new("none").unwrap_err(), "\"none\" is reserved");
        assert!(OptionKey::new("30 percent").is_ok());
        assert_eq!(
            Word::new(" den").unwrap_err(),
            "\" den\" has leading or trailing whitespace"
        );
        assert_eq!(Word::new("").unwrap_err(), "\"\" is empty");
        assert!(Word::new("the snug").is_ok());
        assert!(VarName::new("HUE_TOKEN").is_ok());
        assert!(VarName::new("1X").is_err());
        assert!(RelPath::new("lights.mts").is_ok());
        assert!(RelPath::new("a/b.mjs").is_ok());
        assert!(RelPath::new("/a").is_err());
        assert!(RelPath::new("../a").is_err());
        assert!(RelPath::new("a//b").is_err());
        assert!(ValueName::new("to").is_ok());
        assert!(ValueName::new("To").is_err());
        assert!(AbsPath::new("/").is_ok());
        assert!(AbsPath::new("/etc/hosts").is_ok());
        assert!(AbsPath::new("/etc/").is_err());
        assert!(AbsPath::new("/a/../b").is_err());
        assert!(AbsPath::new("etc").is_err());
        assert!(Owner::new("evoke-build").is_ok());
        assert!(Owner::new("-radhi").is_err());
        assert!(Owner::new("a.b").is_err());
        assert!(Segment::new("radhi").is_ok());
        assert!(Segment::new("--as").is_err());
        assert!(AdapterId::new("").is_err());
    }

    #[test]
    fn serde_enters_through_the_constructor() {
        assert!(serde_json::from_str::<ArgName>("\"class\"").is_err());
        assert_eq!(
            serde_json::to_string(&ArgName::new("room").unwrap()).unwrap(),
            "\"room\""
        );
    }
}
