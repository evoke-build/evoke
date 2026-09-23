//! The project's owned files: where the project is, what it holds, and how an `Edit` lands in one keeping its
//! shape — comments and order survive. In: the environment and the working directory; a path; an `Edit`. Out: the
//! `Root`, a `Snapshot` of the owned texts, a file's text or its absence, an `Edited` file to verify and write;
//! `Failure`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use evoke_core::name::{LocalName, RelPath, VocabName};
use evoke_core::{Digest, Edit, Fix, Json, Owned, digest};
use toml_edit::{DocumentMut, Item, Table};

use super::{Environment, Failure};

/// The home project as first use writes it.
const DEFAULT_PROJECT: &str = "adapter = \"jev\"\n";

/// The project directory, and whether it is the home project — trusted by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Root {
    pub path: PathBuf,
    pub home: bool,
}

/// The owned texts as found: `evoke.toml`, or the home project's default before it is written; `evoke.lock` when
/// there is one; every overlay and vocabulary by name, in name order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub project: String,
    pub lock: Option<String>,
    pub overlays: Vec<(LocalName, String)>,
    pub vocab: Vec<(VocabName, String)>,
}

/// What trust binds to: `h1` over the four owned paths.
#[must_use]
pub fn digest_of(snapshot: &Snapshot) -> Digest {
    let path = |text: String| RelPath::new(&text).expect("an owned path is relative");
    let mut pairs: Vec<(RelPath, &[u8])> =
        vec![(path("evoke.toml".to_owned()), snapshot.project.as_bytes())];
    if let Some(lock) = &snapshot.lock {
        pairs.push((path("evoke.lock".to_owned()), lock.as_bytes()));
    }
    for (name, text) in &snapshot.overlays {
        pairs.push((path(format!("overlays/{name}.toml")), text.as_bytes()));
    }
    for (name, text) in &snapshot.vocab {
        pairs.push((path(format!("vocab/{name}.toml")), text.as_bytes()));
    }
    digest(&pairs)
}

/// The nearest `evoke.toml` upward from the working directory, else the home project.
pub fn locate(environment: &Environment) -> Result<Root, Failure> {
    let home = home(environment)?;
    let cwd =
        std::env::current_dir().map_err(|error| failed("finding the working directory", &error))?;
    let found = cwd.ancestors().find(|dir| dir.join("evoke.toml").is_file());
    Ok(match found {
        Some(dir) => Root {
            home: is_home(dir, &home),
            path: dir.to_path_buf(),
        },
        None => Root {
            path: home,
            home: true,
        },
    })
}

/// `$XDG_CONFIG_HOME/evoke`, else `~/.config/evoke`.
fn home(environment: &Environment) -> Result<PathBuf, Failure> {
    environment.xdg("XDG_CONFIG_HOME", ".config")
}

/// First use: the home project's `evoke.toml`, when it has none. Whether it was written.
pub fn write_home(root: &Root) -> Result<bool, Failure> {
    let path = root.path.join("evoke.toml");
    if !root.home || path.exists() {
        return Ok(false);
    }
    write(&path, DEFAULT_PROJECT).map(|()| true)
}

/// Whether a directory is the home project, however the variables spell it: by the canonical path once it exists.
fn is_home(dir: &Path, home: &Path) -> bool {
    dir == home
        || matches!(
            (fs::canonicalize(dir), fs::canonicalize(home)),
            (Ok(dir), Ok(home)) if dir == home
        )
}

/// The owned texts under the root.
pub fn snapshot(root: &Root) -> Result<Snapshot, Failure> {
    Ok(Snapshot {
        project: read(&root.path.join("evoke.toml"))?.unwrap_or_else(|| DEFAULT_PROJECT.to_owned()),
        lock: read(&root.path.join("evoke.lock"))?,
        overlays: named(&root.path.join("overlays"), LocalName::new)?,
        vocab: named(&root.path.join("vocab"), VocabName::new)?,
    })
}

/// A file's text, or `None` when there is no such file.
pub fn read(path: &Path) -> Result<Option<String>, Failure> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(failed(&format!("reading {}", path.display()), &error)),
    }
}

/// A file written whole — beside its place, then moved in, so a crash mid-write leaves the old file whole — its
/// directory made first. A symlink is followed: the file it names is what is replaced, and the link stays.
pub fn write(path: &Path, text: &str) -> Result<(), Failure> {
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)
            .map_err(|error| failed(&format!("creating {}", dir.display()), &error))?;
    }
    let staged = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&staged, text)
        .and_then(|()| fs::rename(&staged, &path))
        .map_err(|error| failed(&format!("writing {}", path.display()), &error))
}

/// An owned file with an edit applied, not yet written: the caller re-parses `text` through the core, then writes.
pub struct Edited {
    pub path: PathBuf,
    /// The file as shown: `overlays/lights.toml`.
    pub shown: String,
    pub text: String,
    pub landed: Landed,
}

/// What the edit left: the line as it lands, `[examples] "kill the lights" = { state = "off" }`, or the key that
/// went, `[config.lights] bridge`.
pub enum Landed {
    Set(String),
    Removed(String),
}

/// The edit applied to the file's current text, or to an empty file; the shape of what was there is kept.
pub fn edited(root: &Root, edit: &Edit) -> Result<Edited, Failure> {
    let (file, path) = match edit {
        Edit::Set { file, path, .. } | Edit::Remove { file, path } => (file, path),
    };
    let shown = match file {
        Owned::Project => "evoke.toml".to_owned(),
        Owned::Overlay { name } => format!("overlays/{name}.toml"),
        Owned::Vocab { name } => format!("vocab/{name}.toml"),
    };
    let full = root.path.join(&shown);
    let text = read(&full)?.unwrap_or_default();
    let mut document: DocumentMut = text.parse().map_err(|error: toml_edit::TomlError| {
        failed(
            &format!("parsing {shown}"),
            &io::Error::other(error.message()),
        )
    })?;
    let (key, tables) = path.segments().split_last().expect("an edit names a key");
    let mut table = document.as_table_mut();
    for (depth, name) in tables.iter().enumerate() {
        // A table made on the way to the last one is implicit: `[config.lights]` lands without a bare `[config]`.
        let entry = table.entry(name).or_insert_with(|| {
            let mut made = Table::new();
            made.set_implicit(depth + 1 < tables.len());
            Item::Table(made)
        });
        table = entry.as_table_mut().ok_or_else(|| {
            failed(
                &format!("editing {shown}"),
                &io::Error::other(format!("{name} is not a table")),
            )
        })?;
    }
    let landed = match edit {
        Edit::Set { value, .. } => {
            let item = Item::Value(toml_value(value));
            table.insert(key, item);
            let rendered = table
                .get_key_value(key)
                .map(|(key, item)| format!("{} = {}", key.display_repr(), item.to_string().trim()))
                .unwrap_or_default();
            Landed::Set(heading(tables, &rendered))
        }
        Edit::Remove { .. } => {
            let rendered = table
                .get_key_value(key)
                .map_or_else(|| key.clone(), |(key, _)| key.display_repr().into_owned());
            table.remove(key);
            Landed::Removed(heading(tables, &rendered))
        }
    };
    Ok(Edited {
        path: full,
        shown,
        text: document.to_string(),
        landed,
    })
}

/// `[table] <line>`, or the line alone at the top.
fn heading(tables: &[String], line: &str) -> String {
    if tables.is_empty() {
        line.to_owned()
    } else {
        format!("[{}] {line}", tables.join("."))
    }
}

/// A JSON value as TOML: an object is an inline table, so a record lands on one line.
fn toml_value(value: &Json) -> toml_edit::Value {
    match value {
        Json::Null => toml_edit::Value::from(""),
        Json::Bool(b) => toml_edit::Value::from(*b),
        Json::Number(n) => n.as_i64().map_or_else(
            || toml_edit::Value::from(n.as_f64().unwrap_or_default()),
            toml_edit::Value::from,
        ),
        Json::String(s) => toml_edit::Value::from(s.as_str()),
        Json::Array(items) => toml_edit::Value::Array(items.iter().map(toml_value).collect()),
        Json::Object(entries) => {
            let mut table = toml_edit::InlineTable::new();
            for (key, value) in entries {
                table.insert(key, toml_value(value));
            }
            toml_edit::Value::InlineTable(table)
        }
    }
}

/// A directory as `evoke.toml` names a local reflex: relative to the root, `.`, `./dir` or `../dir`, whatever way it
/// was typed from the working directory; none when there is no such directory. A symlinked directory keeps the
/// name it was typed by.
pub fn relative(root: &Path, typed: &str) -> Result<Option<String>, Failure> {
    let dir = match resolved(Path::new(typed)) {
        Ok(dir) => dir,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failed(&format!("finding {typed}"), &error)),
    };
    if !dir.is_dir() {
        return Ok(None);
    }
    let root = fs::canonicalize(root)
        .map_err(|error| failed(&format!("finding {}", root.display()), &error))?;
    let mut base = root.components().peekable();
    let mut target = dir.components().peekable();
    while base.peek().is_some() && base.peek() == target.peek() {
        base.next();
        target.next();
    }
    let up = base.count();
    let rest: Vec<String> = target
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect();
    let mut path = if up == 0 {
        ".".to_owned()
    } else {
        "..".to_owned()
    };
    for _ in 1..up {
        path.push_str("/..");
    }
    for segment in rest {
        path.push('/');
        path.push_str(&segment);
    }
    Ok(Some(path))
}

/// A path made absolute: its parent canonical, its last name as typed, so a symlink keeps the name it is known
/// by; `.` and `..` themselves canonical. What is named must be there.
fn resolved(path: &Path) -> io::Result<PathBuf> {
    let Some(name) = path.file_name() else {
        return fs::canonicalize(path);
    };
    fs::symlink_metadata(path)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(fs::canonicalize(parent)?.join(name))
}

/// The path as a person reads it: `~/…` when it is under `$HOME`, however `$HOME` is spelled.
#[must_use]
pub fn shown(path: &Path, environment: &Environment) -> String {
    let rest = environment.get("HOME").and_then(|home| {
        path.strip_prefix(home).ok().or_else(|| {
            fs::canonicalize(home)
                .ok()
                .and_then(|home| path.strip_prefix(home).ok())
        })
    });
    rest.map_or_else(|| path.to_path_buf(), |rest| Path::new("~").join(rest))
        .display()
        .to_string()
}

/// Every `<name>.toml` in a directory whose stem is a name, with its text, in name order; no directory, nothing.
/// A file whose stem is not a name is not an owned file and is left alone.
fn named<N>(
    dir: &Path,
    name: impl Fn(&str) -> Result<N, String>,
) -> Result<Vec<(N, String)>, Failure> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(failed(&format!("listing {}", dir.display()), &error)),
    };
    let mut paths: Vec<PathBuf> = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()
        .map_err(|error| failed(&format!("listing {}", dir.display()), &error))?;
    paths.sort();
    let mut found = Vec::new();
    for path in paths {
        let stem = path
            .file_name()
            .and_then(|file| file.to_str())
            .and_then(|file| file.strip_suffix(".toml"))
            .and_then(|stem| name(stem).ok());
        if let Some(name) = stem
            && let Some(text) = read(&path)?
        {
            found.push((name, text));
        }
    }
    Ok(found)
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
    use super::*;

    fn environment(pairs: &[(&str, &str)]) -> Environment {
        Environment(
            pairs
                .iter()
                .map(|(var, value)| ((*var).to_owned(), (*value).to_owned()))
                .collect(),
        )
    }

    /// This crate's directory, spelled with a `..` in it: an existing home a variable may spell any way.
    fn crooked_home() -> String {
        format!("{}/../evoke", env!("CARGO_MANIFEST_DIR"))
    }

    /// A fresh directory under the system's temporary one, removed with the guard.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("evoke-files-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_write_follows_a_symlink_and_leaves_it_standing() {
        let scratch = Scratch::new("link");
        let real = scratch.0.join("real.toml");
        let link = scratch.0.join("link.toml");
        fs::write(&real, "before").unwrap();
        std::os::unix::fs::symlink(&real, &link).unwrap();
        write(&link, "after").unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&real).unwrap(), "after");
    }

    #[test]
    fn a_symlinked_directory_keeps_the_name_it_was_typed_by() {
        let scratch = Scratch::new("dir");
        let root = scratch.0.join("project");
        fs::create_dir_all(root.join("real")).unwrap();
        std::os::unix::fs::symlink(root.join("real"), root.join("link")).unwrap();
        let typed = root.join("link");
        assert_eq!(
            relative(&root, typed.to_str().unwrap()).unwrap().as_deref(),
            Some("./link")
        );
        assert_eq!(
            relative(&root, root.join("real").to_str().unwrap())
                .unwrap()
                .as_deref(),
            Some("./real")
        );
        assert_eq!(
            relative(&root, root.join("none").to_str().unwrap()).unwrap(),
            None
        );
    }

    #[test]
    fn the_home_project_follows_xdg_then_home() {
        let env = environment(&[("HOME", "/Users/me"), ("XDG_CONFIG_HOME", "/Users/me/cfg")]);
        assert_eq!(home(&env).unwrap(), Path::new("/Users/me/cfg/evoke"));
        let env = environment(&[("HOME", "/Users/me")]);
        assert_eq!(home(&env).unwrap(), Path::new("/Users/me/.config/evoke"));
        assert!(matches!(
            home(&environment(&[])),
            Err(Failure {
                fix: Fix::ExportKey { .. },
                ..
            })
        ));
    }

    #[test]
    fn home_is_known_however_it_is_spelled() {
        let physical = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(is_home(physical, Path::new(&crooked_home())));
        assert!(!is_home(physical, Path::new("/Users/me/.config/evoke")));
    }

    #[test]
    fn a_local_reflex_is_named_relative_to_the_root() {
        let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let src = crate_dir.join("src").display().to_string();
        assert_eq!(relative(crate_dir, &src).unwrap(), Some("./src".to_owned()));
        let hosts = crate_dir.join("src/hosts").display().to_string();
        assert_eq!(
            relative(&crate_dir.join("src/commands"), &hosts).unwrap(),
            Some("../hosts".to_owned())
        );
        assert_eq!(
            relative(
                &crate_dir.join("src/commands"),
                &crate_dir.display().to_string()
            )
            .unwrap(),
            Some("../..".to_owned())
        );
        assert_eq!(
            relative(crate_dir, &crate_dir.display().to_string()).unwrap(),
            Some(".".to_owned())
        );
        assert_eq!(relative(crate_dir, "./no-such-directory").unwrap(), None);
        assert_eq!(
            relative(
                crate_dir,
                &crate_dir.join("Cargo.toml").display().to_string()
            )
            .unwrap(),
            None
        );
    }

    #[test]
    fn a_path_under_home_shows_as_tilde() {
        let env = environment(&[("HOME", "/Users/me")]);
        assert_eq!(shown(Path::new("/Users/me/app"), &env), "~/app");
        assert_eq!(shown(Path::new("/srv/app"), &env), "/srv/app");
        let env = environment(&[("HOME", &crooked_home())]);
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert_eq!(shown(&src, &env), "~/src");
    }
}
