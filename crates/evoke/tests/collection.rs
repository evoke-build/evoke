//! Every reflex directory under `reflexes/` passes `evoke check` as it is committed: the manifest reads and lints
//! clean, the body loads under the runtime, and `reflex.d.ts` is what the core renders today. Each directory is
//! copied under `target/` first, so the check writes nothing into the tree and finds no version tag to diff against.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn reflexes() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../reflexes");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .expect("reflexes/")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.join("reflex.toml").is_file())
        .collect();
    dirs.sort();
    dirs
}

/// The directory's files, copied flat under `target/collection/<name>/`.
fn copied(dir: &Path) -> PathBuf {
    let name = dir.file_name().expect("a name");
    let copy = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("collection")
        .join(name);
    let _ = fs::remove_dir_all(&copy);
    fs::create_dir_all(&copy).expect("target/ is writable");
    for entry in fs::read_dir(dir).expect("the reflex directory") {
        let path = entry.expect("an entry").path();
        if path.is_file() {
            fs::copy(&path, copy.join(path.file_name().expect("a file name"))).expect("copied");
        }
    }
    copy
}

#[test]
fn every_reflex_checks_clean() {
    let dirs = reflexes();
    assert!(dirs.len() >= 13, "only {} reflexes found", dirs.len());
    for dir in dirs {
        let name = dir
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        let output = Command::new(env!("CARGO_BIN_EXE_evoke"))
            .arg("check")
            .current_dir(copied(&dir))
            .env("TERM", "dumb")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .output()
            .expect("evoke runs");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{name}: {stderr}");
        let first = stderr.lines().next().unwrap_or_default();
        assert!(first.starts_with(&format!("  {name}  ")), "{name}: {first}");
        assert!(
            !stderr.lines().any(|line| line.starts_with("+ ")),
            "{name}: reflex.d.ts is stale; run evoke check in reflexes/{name}\n{stderr}"
        );
    }
}
