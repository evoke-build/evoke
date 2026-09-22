//! The one shape every user-facing problem takes: which reflex, where, what, and the command that fixes it.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::name::{ConfigKey, LocalName, VarName, VocabName};

/// A problem a person has to fix, ending in the command that fixes it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reflex: Option<LocalName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<At>,
    pub message: String,
    pub fix: Fix,
}

/// A line of an owned file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct At {
    pub file: File,
    pub line: u32,
    pub column: u32,
}

/// One of the project's owned files, by role; it displays as its project-relative path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum File {
    Manifest { name: LocalName },
    Overlay { name: LocalName },
    Vocab { name: VocabName },
    Project,
    Lock,
}

/// The closed set of fixing commands, rendered in one place.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Fix {
    VocabAdd {
        vocab: VocabName,
    },
    ConfigSet {
        reflex: LocalName,
        key: ConfigKey,
    },
    ConfigEnv {
        reflex: LocalName,
        key: ConfigKey,
    },
    Update {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reflex: Option<LocalName>,
    },
    Accept {
        reflex: LocalName,
    },
    Trust,
    Remove {
        reflex: LocalName,
    },
    Sync,
    Check,
    EditLine {
        at: At,
    },
    ExportKey {
        var: VarName,
    },
    Rerun,
    Add,
    Show {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reflex: Option<LocalName>,
    },
    /// `evoke add <ref> --as <name>`: a ref to install under a name — one still to choose, when none is given.
    AddRef {
        reference: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<LocalName>,
    },
    /// `evoke teach "<utterance>" not <reflex>`: a phrase that is not this reflex's.
    TeachNot {
        utterance: String,
        reflex: LocalName,
    },
    /// `evoke new <name>`: a reflex directory still to make, under a name still to choose.
    New,
    /// `evoke test`: every record judged, where the theft test at `add` could not finish.
    Test,
    /// `evoke --help`: the arguments did not spell a command.
    Help,
}

impl Fix {
    /// The literal last line of a diagnostic; `invoked` is the argv that was run, which `Rerun` repeats.
    #[must_use]
    pub fn command(&self, invoked: &str) -> String {
        match self {
            Self::VocabAdd { vocab } => format!("evoke vocab {vocab} add <word> \"<meaning>\""),
            Self::ConfigSet { reflex, key } => format!("evoke config {reflex} {key} <value>"),
            Self::ConfigEnv { reflex, key } => format!("evoke config {reflex} {key} --env <VAR>"),
            Self::Update {
                reflex: Some(reflex),
            } => format!("evoke update {reflex}"),
            Self::Update { reflex: None } => "evoke update".to_owned(),
            Self::Accept { reflex } => format!("evoke update --accept {reflex}"),
            Self::Trust => "evoke trust".to_owned(),
            Self::Remove { reflex } => format!("evoke remove {reflex}"),
            Self::Sync => "evoke sync".to_owned(),
            Self::Check => "evoke check".to_owned(),
            Self::EditLine { at } => at.to_string(),
            Self::ExportKey { var } => format!("export {var}=<value>"),
            Self::Rerun => invoked.to_owned(),
            Self::Add => "evoke add evoke-build/reflexes".to_owned(),
            Self::Show {
                reflex: Some(reflex),
            } => format!("evoke show {reflex}"),
            Self::Show { reflex: None } => "evoke show".to_owned(),
            Self::AddRef {
                reference,
                name: Some(name),
            } => format!("evoke add {reference} --as {name}"),
            Self::AddRef {
                reference,
                name: None,
            } => format!("evoke add {reference} --as <name>"),
            Self::TeachNot { utterance, reflex } => format!(
                "evoke teach {} not {reflex}",
                serde_json::Value::String(utterance.clone())
            ),
            Self::New => "evoke new <name>".to_owned(),
            Self::Test => "evoke test".to_owned(),
            Self::Help => "evoke --help".to_owned(),
        }
    }
}

impl File {
    /// The reflex the file belongs to, when it belongs to one.
    pub(crate) fn reflex(&self) -> Option<&LocalName> {
        match self {
            Self::Manifest { name } | Self::Overlay { name } => Some(name),
            Self::Vocab { .. } | Self::Project | Self::Lock => None,
        }
    }
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manifest { name } => write!(f, "{name}/reflex.toml"),
            Self::Overlay { name } => write!(f, "overlays/{name}.toml"),
            Self::Vocab { name } => write!(f, "vocab/{name}.toml"),
            Self::Project => f.write_str("evoke.toml"),
            Self::Lock => f.write_str("evoke.lock"),
        }
    }
}

impl fmt::Display for At {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.file, self.line, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(name: &str) -> LocalName {
        LocalName::new(name).unwrap()
    }

    #[test]
    fn every_fix_is_a_literal_command() {
        let key = ConfigKey::new("token").unwrap();
        let at = At {
            file: File::Overlay {
                name: local("lights"),
            },
            line: 12,
            column: 1,
        };
        let fixes = [
            (
                Fix::VocabAdd {
                    vocab: VocabName::new("rooms").unwrap(),
                },
                "evoke vocab rooms add <word> \"<meaning>\"",
            ),
            (
                Fix::ConfigSet {
                    reflex: local("lights"),
                    key: key.clone(),
                },
                "evoke config lights token <value>",
            ),
            (
                Fix::ConfigEnv {
                    reflex: local("lights"),
                    key,
                },
                "evoke config lights token --env <VAR>",
            ),
            (Fix::Update { reflex: None }, "evoke update"),
            (
                Fix::Update {
                    reflex: Some(local("lights")),
                },
                "evoke update lights",
            ),
            (
                Fix::Accept {
                    reflex: local("lights"),
                },
                "evoke update --accept lights",
            ),
            (Fix::Trust, "evoke trust"),
            (
                Fix::Remove {
                    reflex: local("timer"),
                },
                "evoke remove timer",
            ),
            (Fix::Sync, "evoke sync"),
            (Fix::Check, "evoke check"),
            (Fix::EditLine { at }, "overlays/lights.toml:12:1"),
            (
                Fix::ExportKey {
                    var: VarName::new("HUE_TOKEN").unwrap(),
                },
                "export HUE_TOKEN=<value>",
            ),
            (Fix::Rerun, "evoke \"kill the lights\""),
            (Fix::Add, "evoke add evoke-build/reflexes"),
            (
                Fix::Show {
                    reflex: Some(local("lights")),
                },
                "evoke show lights",
            ),
            (Fix::Show { reflex: None }, "evoke show"),
            (
                Fix::AddRef {
                    reference: "radhi/home/lights".to_owned(),
                    name: Some(local("lights")),
                },
                "evoke add radhi/home/lights --as lights",
            ),
            (
                Fix::AddRef {
                    reference: "radhi/timer@1.0.1".to_owned(),
                    name: None,
                },
                "evoke add radhi/timer@1.0.1 --as <name>",
            ),
            (
                Fix::TeachNot {
                    utterance: "set a \"laundry\" timer".to_owned(),
                    reflex: local("awake"),
                },
                "evoke teach \"set a \\\"laundry\\\" timer\" not awake",
            ),
            (Fix::New, "evoke new <name>"),
        ];
        for (fix, command) in fixes {
            assert_eq!(fix.command("evoke \"kill the lights\""), command);
        }
    }

    #[test]
    fn the_tests_and_the_help_are_commands_too() {
        assert_eq!(Fix::Test.command(""), "evoke test");
        assert_eq!(Fix::Help.command("evoke x"), "evoke --help");
    }

    #[test]
    fn files_display_as_project_paths() {
        assert_eq!(
            File::Manifest {
                name: local("lights")
            }
            .to_string(),
            "lights/reflex.toml"
        );
        assert_eq!(
            File::Vocab {
                name: VocabName::new("rooms").unwrap()
            }
            .to_string(),
            "vocab/rooms.toml"
        );
        assert_eq!(File::Project.to_string(), "evoke.toml");
        assert_eq!(File::Lock.to_string(), "evoke.lock");
        let json = serde_json::to_string(&File::Manifest {
            name: local("lights"),
        })
        .unwrap();
        assert_eq!(json, "{\"type\":\"manifest\",\"name\":\"lights\"}");
    }
}
