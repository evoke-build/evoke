//! Fetched trees under `$XDG_CACHE_HOME/evoke/store/<h1>/`, kept whole and re-hashed before every use, so a reflex
//! runs only while it still hashes to its lock. In: a tree to keep; a digest to read. Out: what was kept, with its
//! digest and directory; the entry's directory and files, or a miss; `Failure`.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use evoke_core::name::RelPath;
use evoke_core::{Digest, digest};

use super::{Environment, Failure, failed};

pub struct Store {
    dir: PathBuf,
}

/// A tree in the store, by its digest.
pub struct Entry {
    pub h1: Digest,
    pub dir: PathBuf,
    pub files: Vec<(RelPath, Vec<u8>)>,
}

impl Store {
    pub fn of(environment: &Environment) -> Result<Self, Failure> {
        Ok(Self {
            dir: environment.xdg("XDG_CACHE_HOME", ".cache")?.join("store"),
        })
    }

    /// Keeps a tree under its digest: written beside, then moved into place; one already there that still hashes
    /// stands.
    pub fn keep(&self, files: Vec<(RelPath, Vec<u8>)>) -> Result<Entry, Failure> {
        let h1 = hashed(&files);
        let dir = self.entry_dir(&h1);
        if self.entry(&h1)?.is_some() {
            return Ok(Entry { h1, dir, files });
        }
        let what = format!("keeping {}", dir.display());
        let staged = self
            .dir
            .join(format!("{}.{}.tmp", hex(&h1), std::process::id()));
        let _ = fs::remove_dir_all(&staged);
        for (path, bytes) in &files {
            let file = staged.join(path.as_str());
            if let Some(parent) = file.parent() {
                fs::create_dir_all(parent).map_err(|error| failed(&what, &error.to_string()))?;
            }
            fs::write(&file, bytes).map_err(|error| failed(&what, &error.to_string()))?;
        }
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|error| failed(&what, &error.to_string()))?;
        }
        fs::rename(&staged, &dir).map_err(|error| failed(&what, &error.to_string()))?;
        Ok(Entry { h1, dir, files })
    }

    /// The entry's directory and files, when they still hash to the digest; else a miss.
    pub fn entry(&self, h1: &Digest) -> Result<Option<Entry>, Failure> {
        let dir = self.entry_dir(h1);
        if !dir.is_dir() {
            return Ok(None);
        }
        let mut files = Vec::new();
        if !walk(&dir, &dir, &mut files)? {
            return Ok(None);
        }
        if hashed(&files) != *h1 {
            return Ok(None);
        }
        Ok(Some(Entry {
            h1: *h1,
            dir,
            files,
        }))
    }

    fn entry_dir(&self, h1: &Digest) -> PathBuf {
        self.dir.join(hex(h1))
    }
}

/// The digest a tree would be kept under.
#[must_use]
pub fn hashed(files: &[(RelPath, Vec<u8>)]) -> Digest {
    let pairs: Vec<(RelPath, &[u8])> = files
        .iter()
        .map(|(path, bytes)| (path.clone(), bytes.as_slice()))
        .collect();
    digest(&pairs)
}

/// The digest's hex, the directory's name.
fn hex(h1: &Digest) -> String {
    let text = h1.to_string();
    text.strip_prefix("h1:").unwrap_or(&text).to_owned()
}

/// Every file under the entry with its path relative to it; `false` when a path is not one the store wrote.
fn walk(root: &Path, dir: &Path, files: &mut Vec<(RelPath, Vec<u8>)>) -> Result<bool, Failure> {
    let what = format!("reading {}", root.display());
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map_err(|error| failed(&what, &error.to_string()))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, io::Error>>()
        .map_err(|error| failed(&what, &error.to_string()))?;
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if !walk(root, &path, files)? {
                return Ok(false);
            }
            continue;
        }
        let Some(relative) = path
            .strip_prefix(root)
            .ok()
            .and_then(|relative| relative.to_str())
            .and_then(|relative| RelPath::new(relative).ok())
        else {
            return Ok(false);
        };
        let bytes = fs::read(&path).map_err(|error| failed(&what, &error.to_string()))?;
        files.push((relative, bytes));
    }
    Ok(true)
}
