//! A reflex directory from a git repository without a checkout: the version tags a remote has, and a tag's tree
//! read with `ls-tree` and `cat-file` from a shallow bare fetch into a scratch repository, removed after — each
//! repository listed and each tag fetched once for a command, however many reflexes it takes from them; and, for
//! `check`, the repository a directory sits in, its own tags, and a tag's tree at that directory, read the same
//! way. In: a `Reference`, a `Tag`; a directory. Out: the tags; `Fetched` — the commit, every reflex tree the
//! reference reaches, every reflex directory the repository holds; a `Repository` and a `Tree`; `Failure`. A
//! symlink or a submodule is refused; the project's own five names inside a reflex directory are skipped, so an
//! author's development project beside a root reflex never ships.

use std::collections::BTreeMap;
use std::io;
use std::os::unix::fs::DirBuilderExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use evoke_core::Version;
use evoke_core::name::RelPath;
use evoke_core::project::{Commit, Reference, Repo};

use super::{Failure, failed};

/// What every git call is told: only https and ssh are spoken — a local mirror reached through
/// `url.<path>.insteadOf` needs `protocol.file.allow` in the person's own configuration — and no hook runs from
/// any repository git touches.
const CONFIG: [&str; 4] = [
    "protocol.allow=never",
    "protocol.https.allow=always",
    "protocol.ssh.allow=always",
    "core.hooksPath=/dev/null",
];

/// The one local transport: a repository on this machine, read by `check`.
const LOCAL: [&str; 2] = ["-c", "protocol.file.allow=always"];

/// The project's own names, skipped inside a reflex directory by fetch and so by `h1`.
const OWNED: [&str; 5] = [
    "evoke.toml",
    "evoke.lock",
    "evoke.d.ts",
    "overlays",
    "vocab",
];

/// A version tag as the remote names it, `v` or not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub version: Version,
    pub name: String,
}

/// A tag's tree as far as the reference reaches.
pub struct Fetched {
    pub commit: Commit,
    /// The reflex directories under the reference: the one it names, else every child that is one.
    pub reflexes: Vec<Tree>,
    /// Every reflex directory in the repository, `None` for its root: what `update` reports as new.
    pub all: Vec<Option<RelPath>>,
}

/// One reflex directory: where it is in the repository, and its files relative to it.
pub struct Tree {
    pub dir: Option<RelPath>,
    pub files: Vec<(RelPath, Vec<u8>)>,
}

/// The repository a directory sits in — its git directory — and the directory's path within it, `None` at the
/// root: what `check` diffs against.
pub struct Repository {
    git_dir: PathBuf,
    prefix: Option<RelPath>,
}

/// The repository around a directory, or none when it is in none or `git` is not installed.
pub fn repository(dir: &Path) -> Result<Option<Repository>, Failure> {
    let what = format!("finding the repository of {}", dir.display());
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--absolute-git-dir", "--show-prefix"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null());
    let output = match command.output() {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(failed(&what, &format!("git: {error}"))),
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("not a git repository") {
            return Ok(None);
        }
        return Err(failed(&what, &format!("git: {}", last_line(&stderr))));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let git_dir = PathBuf::from(lines.next().unwrap_or_default());
    let prefix = lines
        .next()
        .unwrap_or_default()
        .trim_end_matches('/')
        .to_owned();
    let prefix = if prefix.is_empty() {
        None
    } else {
        Some(RelPath::new(&prefix).map_err(|why| failed(&what, &why))?)
    };
    Ok(Some(Repository { git_dir, prefix }))
}

/// The version tags of a repository on this machine, oldest first.
pub fn local_tags(repository: &Repository) -> Result<Vec<Tag>, Failure> {
    let what = format!("listing the tags of {}", repository.git_dir.display());
    let git_dir = repository.git_dir.display().to_string();
    let mut args = LOCAL.to_vec();
    args.extend([
        "ls-remote",
        "--tags",
        "--refs",
        "--end-of-options",
        &git_dir,
    ]);
    let listing = text(None, &args, &what)?;
    Ok(versions(&listing))
}

/// The tag's tree at the repository's directory, when a `reflex.toml` is there.
pub fn tree_at(repository: &Repository, tag: &Tag) -> Result<Option<Tree>, Failure> {
    let what = format!(
        "reading {} at {}",
        repository.git_dir.display(),
        tag.version
    );
    let dir = Some(repository.git_dir.as_path());
    let mut args = vec![
        "ls-tree".to_owned(),
        "-r".to_owned(),
        "-z".to_owned(),
        format!("refs/tags/{}", tag.name),
    ];
    if let Some(prefix) = &repository.prefix {
        args.push("--".to_owned());
        args.push(prefix.to_string());
    }
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let listing = bytes(dir, &args, &what)?;
    let blobs = blobs(&listing, &what)?;
    let files = files_of(dir, &blobs, repository.prefix.as_ref(), &what)?;
    Ok(files
        .iter()
        .any(|(path, _)| path.as_str() == "reflex.toml")
        .then_some(Tree {
            dir: repository.prefix.clone(),
            files,
        }))
}

/// What one command learned from the remotes: each repository's tags, listed once; each tag's tree, fetched once
/// and read for every reflex the command takes from it. Dropped, its scratch repositories go.
#[derive(Default)]
pub struct Remotes {
    tags: BTreeMap<String, Vec<Tag>>,
    fetched: BTreeMap<(String, Version), Fetch>,
}

/// One tag of one repository as fetched: its commit and every file it lists, still in the scratch repository.
struct Fetch {
    scratch: Scratch,
    commit: Commit,
    blobs: Vec<Blob>,
}

impl Remotes {
    /// The version tags a repository has, oldest first; two spellings of one version are one tag.
    pub fn tags(&mut self, reference: &Reference) -> Result<Vec<Tag>, Failure> {
        let url = url(reference);
        if let Some(tags) = self.tags.get(&url) {
            return Ok(tags.clone());
        }
        let what = format!("listing the tags of {reference}");
        let listing = text(
            None,
            &["ls-remote", "--tags", "--refs", "--end-of-options", &url],
            &what,
        )?;
        let tags = versions(&listing);
        self.tags.insert(url, tags.clone());
        Ok(tags)
    }

    /// The tag's tree under the reference.
    pub fn fetch(&mut self, reference: &Reference, tag: &Tag) -> Result<Fetched, Failure> {
        let what = format!("fetching {reference} {}", tag.version);
        let key = (url(reference), tag.version);
        if !self.fetched.contains_key(&key) {
            let fetched = Fetch::of(&key.0, tag, &what)?;
            self.fetched.insert(key.clone(), fetched);
        }
        self.fetched[&key].under(reference, &what)
    }
}

/// The version tags among an `ls-remote --tags` listing, oldest first; two spellings of one version are one tag.
fn versions(listing: &str) -> Vec<Tag> {
    let mut tags: Vec<Tag> = listing
        .lines()
        .filter_map(|line| {
            let (_, refname) = line.split_once('\t')?;
            let name = refname.strip_prefix("refs/tags/")?;
            Version::parse(name).ok().map(|version| Tag {
                version,
                name: name.to_owned(),
            })
        })
        .collect();
    tags.sort_by_key(|tag| tag.version);
    tags.dedup_by_key(|tag| tag.version);
    tags
}

impl Fetch {
    /// A shallow bare fetch of one tag into a fresh scratch repository, and its listing.
    fn of(url: &str, tag: &Tag, what: &str) -> Result<Self, Failure> {
        let scratch = Scratch::new(what)?;
        let dir = Some(scratch.path.as_path());
        text(dir, &["init", "--quiet", "--bare"], what)?;
        let refspec = format!("refs/tags/{0}:refs/tags/{0}", tag.name);
        text(
            dir,
            &[
                "fetch",
                "--quiet",
                "--depth",
                "1",
                "--no-tags",
                "--end-of-options",
                url,
                &refspec,
            ],
            what,
        )?;
        let peeled = format!("refs/tags/{}^{{commit}}", tag.name);
        let commit = text(
            dir,
            &["rev-parse", "--verify", "--end-of-options", &peeled],
            what,
        )?;
        let commit = Commit::new(commit.trim()).map_err(|why| failed(what, &why))?;
        let listing = bytes(dir, &["ls-tree", "-r", "-z", commit.as_str()], what)?;
        let blobs = blobs(&listing, what)?;
        Ok(Self {
            scratch,
            commit,
            blobs,
        })
    }

    /// The reflex directories the reference reaches — the one it names, else every child that is one — read
    /// from the scratch repository, with every reflex directory the repository holds.
    fn under(&self, reference: &Reference, what: &str) -> Result<Fetched, Failure> {
        let dir = Some(self.scratch.path.as_path());
        let all: Vec<Option<RelPath>> = self
            .blobs
            .iter()
            .filter_map(|blob| {
                let dir = blob.path.strip_suffix("reflex.toml")?;
                if dir.is_empty() {
                    Some(None)
                } else {
                    RelPath::new(dir.strip_suffix('/')?).ok().map(Some)
                }
            })
            .collect();
        let under = reference.dir.as_ref().map(RelPath::as_str);
        let dirs: Vec<Option<RelPath>> = if all
            .iter()
            .any(|dir| dir.as_ref().map(RelPath::as_str) == under)
        {
            vec![reference.dir.clone()]
        } else {
            all.iter()
                .filter(|dir| {
                    dir.as_ref()
                        .is_some_and(|dir| parent_of(dir.as_str()) == under)
                })
                .cloned()
                .collect()
        };
        let mut reflexes = Vec::new();
        for reflex in dirs {
            let files = files_of(dir, &self.blobs, reflex.as_ref(), what)?;
            reflexes.push(Tree { dir: reflex, files });
        }
        Ok(Fetched {
            commit: self.commit.clone(),
            reflexes,
            all,
        })
    }
}

/// The files under one reflex directory, read from the repository, the project's own names skipped.
fn files_of(
    dir: Option<&Path>,
    blobs: &[Blob],
    reflex: Option<&RelPath>,
    what: &str,
) -> Result<Vec<(RelPath, Vec<u8>)>, Failure> {
    let mut files = Vec::new();
    for blob in blobs {
        let Some(inside) = within(&blob.path, reflex) else {
            continue;
        };
        if OWNED.contains(&inside.split('/').next().unwrap_or(inside)) {
            continue;
        }
        let path = RelPath::new(inside).map_err(|why| failed(what, &why))?;
        let bytes = bytes(dir, &["cat-file", "blob", &blob.oid], what)?;
        files.push((path, bytes));
    }
    Ok(files)
}

/// One file of the tree: its object and its path.
#[derive(Debug)]
struct Blob {
    oid: String,
    path: String,
}

/// Every file `ls-tree -r -z` listed; a symlink or a submodule is refused.
fn blobs(listing: &[u8], what: &str) -> Result<Vec<Blob>, Failure> {
    let mut blobs = Vec::new();
    for entry in listing
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
    {
        let entry = String::from_utf8_lossy(entry);
        let Some((meta, path)) = entry.split_once('\t') else {
            return Err(failed(what, &format!("unreadable tree entry \"{entry}\"")));
        };
        let mut meta = meta.split(' ');
        let (Some(mode), Some(kind), Some(oid)) = (meta.next(), meta.next(), meta.next()) else {
            return Err(failed(what, &format!("unreadable tree entry \"{entry}\"")));
        };
        match (mode, kind) {
            ("120000", _) => return Err(failed(what, &format!("{path} is a symbolic link"))),
            ("160000", _) | (_, "commit") => {
                return Err(failed(what, &format!("{path} is a submodule")));
            }
            (_, "blob") => blobs.push(Blob {
                oid: oid.to_owned(),
                path: path.to_owned(),
            }),
            _ => {}
        }
    }
    Ok(blobs)
}

/// The directory a directory sits in, `None` at the root.
fn parent_of(dir: &str) -> Option<&str> {
    dir.rsplit_once('/').map(|(parent, _)| parent)
}

/// A path relative to a reflex directory, when it is under it.
fn within<'p>(path: &'p str, dir: Option<&RelPath>) -> Option<&'p str> {
    match dir {
        None => Some(path),
        Some(dir) => path
            .strip_prefix(dir.as_str())
            .and_then(|rest| rest.strip_prefix('/')),
    }
}

fn url(reference: &Reference) -> String {
    match &reference.repo {
        Repo::GitHub { owner, name } => format!("https://github.com/{owner}/{name}"),
        Repo::Url { url } => url.to_string(),
    }
}

fn text(dir: Option<&Path>, args: &[&str], what: &str) -> Result<String, Failure> {
    bytes(dir, args, what).map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// One git command, its stdout; a failure carries the last line git wrote. The process environment passes through,
/// so a person's own git configuration and credentials apply, but git never prompts.
fn bytes(dir: Option<&Path>, args: &[&str], what: &str) -> Result<Vec<u8>, Failure> {
    let mut command = Command::new("git");
    if let Some(dir) = dir {
        command.arg("--git-dir").arg(dir);
    }
    for setting in CONFIG {
        command.arg("-c").arg(setting);
    }
    command
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null());
    let output = command
        .output()
        .map_err(|error| failed(what, &format!("git: {error}")))?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(failed(what, &format!("git: {}", last_line(&stderr))))
}

/// What git said last, or that it failed.
fn last_line(stderr: &str) -> &str {
    stderr
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map_or("git failed", str::trim)
}

/// A scratch repository under the temporary directory, removed when the fetch is over. Made exclusively, mode
/// 0700, under a name no one could have placed first: a directory already there — anyone's, in a shared `/tmp` —
/// is never entered, since git would read its configuration and hooks.
struct Scratch {
    path: PathBuf,
}

impl Scratch {
    fn new(what: &str) -> Result<Self, Failure> {
        let temp = std::env::temp_dir();
        let pid = std::process::id();
        for _ in 0..64 {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.subsec_nanos());
            let path = temp.join(format!("evoke-fetch-{pid}-{nanos:09}"));
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => {
                    return Err(failed(
                        what,
                        &format!("creating {}: {error}", path.display()),
                    ));
                }
            }
        }
        Err(failed(
            what,
            &format!("no free scratch directory under {}", temp.display()),
        ))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tree_listing_yields_blobs_and_refuses_links() {
        let listing =
            b"100644 blob abc\tlights/reflex.toml\x00100755 blob def\tlights/lights.mts\x00";
        let found = blobs(listing, "fetching").unwrap();
        assert_eq!(found.len(), 2);
        assert_eq!(found[1].path, "lights/lights.mts");
        let link = b"120000 blob abc\tlights/link\x00";
        let refused = blobs(link, "fetching").unwrap_err();
        assert!(refused.cause.unwrap().contains("symbolic link"));
        let submodule = b"160000 commit abc\tvendor\x00";
        let refused = blobs(submodule, "fetching").unwrap_err();
        assert!(refused.cause.unwrap().contains("submodule"));
    }

    #[test]
    fn a_file_is_placed_under_its_reflex_directory() {
        assert_eq!(parent_of("a/b"), Some("a"));
        assert_eq!(parent_of("a"), None);
        let dir = RelPath::new("lights").unwrap();
        assert_eq!(within("lights/lights.mts", Some(&dir)), Some("lights.mts"));
        assert_eq!(within("lightsx/y", Some(&dir)), None);
        assert_eq!(within("y", None), Some("y"));
    }
}
