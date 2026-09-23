//! Machine-local state under XDG, never in the project. Cache, `$XDG_CACHE_HOME/evoke/`: the adapter's answers per
//! plan digest, utterance identity and question set, and `test`'s baseline per plan digest. State,
//! `$XDG_STATE_HOME/evoke/`: the log, one JSON line per decision; the JavaScript runtime's path, as `add` and
//! `sync` record it; trust, `trust.toml`, a digest per blessed root; the REPL's history, one line each. In: keys
//! and values. Out: hits or misses; `Failure`.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};

use evoke_core::{Baseline, Digest, Fix, Raw, Request, identity};
use sha2::{Digest as _, Sha256};
use toml_edit::{DocumentMut, Item, value};

use super::{Environment, Failure};

pub struct State {
    state: PathBuf,
    cache: PathBuf,
}

impl State {
    pub fn of(environment: &Environment) -> Result<Self, Failure> {
        Ok(Self {
            state: environment.xdg("XDG_STATE_HOME", ".local/state")?,
            cache: environment.xdg("XDG_CACHE_HOME", ".cache")?,
        })
    }

    /// The runtime recorded on this machine, or none yet.
    pub fn runtime(&self) -> Result<Option<PathBuf>, Failure> {
        Ok(read(&self.state.join("runtime"))?
            .map(|text| PathBuf::from(text.trim()))
            .filter(|path| !path.as_os_str().is_empty()))
    }

    /// The runtime `add` or `sync` found, recorded for every run after.
    pub fn set_runtime(&self, runtime: &Path) -> Result<(), Failure> {
        write(
            &self.state.join("runtime"),
            &format!("{}\n", runtime.display()),
        )
    }

    /// The REPL's history file, made readable by its owner alone: `$XDG_STATE_HOME/evoke/history`.
    pub fn history(&self) -> Result<PathBuf, Failure> {
        let path = self.state.join("history");
        own(&path)?.map_err(|error| failed(&format!("opening {}", path.display()), &error))?;
        Ok(path)
    }

    /// The digest a root was blessed at, or none.
    pub fn trust(&self, root: &Path) -> Result<Option<Digest>, Failure> {
        let Some(text) = read(&self.state.join("trust.toml"))? else {
            return Ok(None);
        };
        let document: DocumentMut = text.parse().map_err(|error: toml_edit::TomlError| {
            failed("reading trust.toml", &io::Error::other(error.message()))
        })?;
        Ok(document
            .get(&key(root))
            .and_then(Item::as_str)
            .and_then(|digest| Digest::try_from(digest.to_owned()).ok()))
    }

    /// `evoke trust`: the root blessed at this digest, its entry made or replaced.
    pub fn bless(&self, root: &Path, digest: &Digest) -> Result<(), Failure> {
        let path = self.state.join("trust.toml");
        let mut document: DocumentMut =
            read(&path)?
                .unwrap_or_default()
                .parse()
                .map_err(|error: toml_edit::TomlError| {
                    failed("reading trust.toml", &io::Error::other(error.message()))
                })?;
        document[&key(root)] = value(digest.to_string());
        write(&path, &document.to_string())
    }

    /// After a write of `evoke`'s own: the root's entry replaced, never made.
    pub fn rebless(&self, root: &Path, digest: &Digest) -> Result<(), Failure> {
        if self.trust(root)?.is_some() {
            self.bless(root, digest)
        } else {
            Ok(())
        }
    }

    /// The answers cached for a request under a plan; an entry that does not read is a miss.
    pub fn answers(&self, plan: &Digest, request: &Request) -> Result<Option<Raw>, Failure> {
        Ok(read(&self.entry(plan, request))?.and_then(|text| serde_json::from_str(&text).ok()))
    }

    /// Keeps the answers for the request under the plan; written whole, then moved into place.
    pub fn keep(&self, plan: &Digest, request: &Request, answers: &Raw) -> Result<(), Failure> {
        let path = self.entry(plan, request);
        let text = serde_json::to_string(answers).expect("answers serialize");
        let staged = path.with_extension(format!("{}.tmp", std::process::id()));
        write(&staged, &text).and_then(|()| {
            fs::rename(&staged, &path)
                .map_err(|error| failed(&format!("keeping {}", path.display()), &error))
        })
    }

    /// The last `test` run's verdicts under a plan, or none yet; one that does not read is none.
    pub fn baseline(&self, plan: &Digest) -> Result<Option<Baseline>, Failure> {
        Ok(read(&self.baseline_path(plan))?.and_then(|text| serde_json::from_str(&text).ok()))
    }

    /// The verdicts kept for the next run under the plan.
    pub fn keep_baseline(&self, plan: &Digest, baseline: &Baseline) -> Result<(), Failure> {
        let text = serde_json::to_string(baseline).expect("a baseline serializes");
        write(&self.baseline_path(plan), &text)
    }

    /// `baselines/<plan>.json`.
    fn baseline_path(&self, plan: &Digest) -> PathBuf {
        self.cache
            .join("baselines")
            .join(format!("{}.json", hex(plan)))
    }

    /// One more line of the log, a file its owner alone reads: it holds what was typed and what a body returned.
    pub fn log(&self, line: &str) -> Result<(), Failure> {
        let path = self.state.join("log.jsonl");
        own(&path)?
            .and_then(|mut log| writeln!(log, "{line}"))
            .map_err(|error| failed(&format!("appending to {}", path.display()), &error))
    }

    /// The log's last input, as its lines: one decision's, or every line of the last weave — the trailing lines
    /// of one plan, each step's once — in the order they were written; nothing decided yet is empty.
    pub fn tail(&self) -> Result<Vec<String>, Failure> {
        let Some(text) = read(&self.state.join("log.jsonl"))? else {
            return Ok(Vec::new());
        };
        let mut lines: Vec<String> = Vec::new();
        let mut seen: Vec<usize> = Vec::new();
        let mut of: Option<usize> = None;
        for line in text.lines().rev().filter(|line| !line.trim().is_empty()) {
            match (step_of(line), of) {
                (None, None) => {
                    lines.push(line.to_owned());
                    break;
                }
                (Some((n, count)), None) => {
                    of = Some(count);
                    seen.push(n);
                    lines.push(line.to_owned());
                }
                (Some((n, count)), Some(expected)) if count == expected && !seen.contains(&n) => {
                    seen.push(n);
                    lines.push(line.to_owned());
                }
                _ => break,
            }
            if of.is_some_and(|count| seen.len() >= count) {
                break;
            }
        }
        lines.reverse();
        Ok(lines)
    }

    /// `answers/<plan>/<sha256 of the utterance identity and the question ids asked>.json`: one entry per plan,
    /// utterance and question set, so a decision narrowed with `--tag` keeps its own beside the full one.
    fn entry(&self, plan: &Digest, request: &Request) -> PathBuf {
        let mut key = Sha256::new();
        key.update(identity(request.state.request.as_str()).as_str().as_bytes());
        let mut asked: Vec<String> = request.questions.keys().map(ToString::to_string).collect();
        asked.sort();
        for question in asked {
            key.update(b"\n");
            key.update(question.as_bytes());
        }
        let mut name = String::with_capacity(69);
        for byte in key.finalize() {
            let _ = write!(name, "{byte:02x}");
        }
        name.push_str(".json");
        self.cache.join("answers").join(hex(plan)).join(name)
    }
}

/// A line's `step` and `steps` when it is a weave's, read without the line's shape, which `why` parses.
fn step_of(line: &str) -> Option<(usize, usize)> {
    let fields: serde_json::Value = serde_json::from_str(line).ok()?;
    let number = |key: &str| usize::try_from(fields.get(key)?.as_u64()?).ok();
    Some((number("step")?, number("steps")?))
}

/// A digest's hex, without its `h1:`: a directory or file name.
fn hex(digest: &Digest) -> String {
    let text = digest.to_string();
    text.strip_prefix("h1:").unwrap_or(&text).to_owned()
}

/// A root as trust keys it: its canonical absolute path.
fn key(root: &Path) -> String {
    fs::canonicalize(root)
        .unwrap_or_else(|_| root.to_path_buf())
        .display()
        .to_string()
}

/// A file's text, or none when there is no such file.
fn read(path: &Path) -> Result<Option<String>, Failure> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(failed(&format!("reading {}", path.display()), &error)),
    }
}

/// A file written whole — beside its place, then moved in — its directory made first.
fn write(path: &Path, text: &str) -> Result<(), Failure> {
    dir_of(path)?;
    let staged = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&staged, text)
        .and_then(|()| fs::rename(&staged, path))
        .map_err(|error| failed(&format!("writing {}", path.display()), &error))
}

/// A file of the person's own opened for appending, made mode 0600 when it is not there yet, its directory made.
fn own(path: &Path) -> Result<io::Result<fs::File>, Failure> {
    dir_of(path)?;
    Ok(fs::OpenOptions::new()
        .append(true)
        .create(true)
        .mode(0o600)
        .open(path))
}

fn dir_of(path: &Path) -> Result<(), Failure> {
    let dir = path.parent().expect("a state file has a directory");
    fs::create_dir_all(dir).map_err(|error| failed(&format!("creating {}", dir.display()), &error))
}

fn failed(what: &str, error: &io::Error) -> Failure {
    Failure {
        what: what.to_owned(),
        cause: Some(super::cause(error)),
        fix: Fix::Rerun,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use evoke_core::adapter::QuestionId;

    use super::*;

    /// `spec/fixtures/request-kill-the-lights.json`, with `--tag` narrowing it to the questions kept.
    fn request(kept: &[&str]) -> Request {
        let mut request: Request = serde_json::from_str(include_str!(
            "../../../../spec/fixtures/request-kill-the-lights.json"
        ))
        .unwrap();
        request.questions.retain(|id: &QuestionId, _| {
            kept.is_empty() || kept.contains(&id.to_string().as_str())
        });
        request
    }

    #[test]
    fn an_entry_is_keyed_by_the_utterance_and_the_questions_asked() {
        let dir = std::env::temp_dir().join(format!("evoke-state-{}", std::process::id()));
        let state = State::of(&Environment(BTreeMap::from([
            ("XDG_CACHE_HOME".to_owned(), dir.display().to_string()),
            ("XDG_STATE_HOME".to_owned(), dir.display().to_string()),
        ])))
        .unwrap();
        let plan: Digest =
            serde_json::from_value(serde_json::Value::String(format!("h1:{}", "a".repeat(64))))
                .unwrap();
        let full = request(&[]);
        let narrowed = request(&["route", "fits.lights", "lights.room", "lights.state"]);
        let mut spelled = request(&[]);
        spelled.state.request = evoke_core::Input::new("Kill the lights!").unwrap();
        assert_ne!(state.entry(&plan, &full), state.entry(&plan, &narrowed));
        assert_eq!(state.entry(&plan, &full), state.entry(&plan, &spelled));
        let answers: Raw =
            serde_json::from_value(serde_json::json!({ "route": { "lights": 1.0 } })).unwrap();
        state.keep(&plan, &narrowed, &answers).unwrap();
        assert_eq!(state.answers(&plan, &narrowed).unwrap(), Some(answers));
        assert_eq!(state.answers(&plan, &full).unwrap(), None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn the_tail_is_the_last_input_s_lines() {
        let dir = std::env::temp_dir().join(format!("evoke-tail-{}", std::process::id()));
        let state = State::of(&Environment(BTreeMap::from([
            ("XDG_CACHE_HOME".to_owned(), dir.display().to_string()),
            ("XDG_STATE_HOME".to_owned(), dir.display().to_string()),
        ])))
        .unwrap();
        assert!(state.tail().unwrap().is_empty());
        state.log(r#"{"input":"one"}"#).unwrap();
        assert_eq!(state.tail().unwrap(), vec![r#"{"input":"one"}"#]);
        // Two weaves of two steps in a row: the tail is the second one's lines, in order.
        for line in [
            r#"{"step":1,"steps":2,"input":"a"}"#,
            r#"{"step":2,"steps":2,"input":"b"}"#,
            r#"{"step":1,"steps":2,"input":"c"}"#,
            r#"{"step":2,"steps":2,"input":"d"}"#,
        ] {
            state.log(line).unwrap();
        }
        assert_eq!(
            state.tail().unwrap(),
            vec![
                r#"{"step":1,"steps":2,"input":"c"}"#,
                r#"{"step":2,"steps":2,"input":"d"}"#
            ]
        );
        // A weave that logged one step of three, after another: its one line.
        state.log(r#"{"step":2,"steps":3,"input":"e"}"#).unwrap();
        assert_eq!(
            state.tail().unwrap(),
            vec![r#"{"step":2,"steps":3,"input":"e"}"#]
        );
        state.log(r#"{"input":"two"}"#).unwrap();
        assert_eq!(state.tail().unwrap(), vec![r#"{"input":"two"}"#]);
        let _ = fs::remove_dir_all(dir);
    }
}
