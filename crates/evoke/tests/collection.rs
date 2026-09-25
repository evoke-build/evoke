//! Every reflex directory under `reflexes/` passes `evoke check` as it is committed: the manifest reads and lints
//! clean, the body loads under the runtime and its declaration, and `reflex.d.ts` is what the core renders today.
//! Each directory is copied under `target/` first, so the check writes nothing into the tree and finds no version
//! tag to diff against: its report is the row alone, and nothing is written. The check runs in a home made for
//! it, holding the paths the declarations name, with stand-ins on `PATH` for the Mac's programs they run — the
//! collection is for a Mac, and a declaration names what must exist before a body starts.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The programs the collection's declarations name, each a stand-in that says nothing.
const PROGRAMS: [&str; 7] = [
    "caffeinate",
    "networksetup",
    "open",
    "osascript",
    "plutil",
    "pmset",
    "screencapture",
];

/// The paths the collection's declarations name under the home.
const PATHS: [&str; 3] = [
    "Desktop",
    "Downloads",
    "Library/Preferences/com.apple.LaunchServices",
];

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

/// A home with every path the declarations name, and `bin/` with a stand-in for every program.
fn home() -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let home = Path::new(env!("CARGO_TARGET_TMPDIR")).join("collection-home");
    let _ = fs::remove_dir_all(&home);
    for path in PATHS {
        fs::create_dir_all(home.join(path)).expect("the home is made");
    }
    fs::write(
        home.join(
            "Library/Preferences/com.apple.LaunchServices/com.apple.launchservices.secure.plist",
        ),
        "{}\n",
    )
    .expect("the plist stands in");
    let bin = home.join("bin");
    fs::create_dir_all(&bin).expect("bin is made");
    for program in PROGRAMS {
        let path = bin.join(program);
        fs::write(&path, "#!/bin/sh\nexit 0\n").expect("a stand-in is written");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("executable");
    }
    home
}

/// The directory of the runtime `node` runs as, first on the `PATH` evoke gets: a version manager's shim, first
/// on this machine's, answers only in its owner's home, and `check` asks the runtime where it is.
fn runtime_dir() -> PathBuf {
    let found = Command::new("node")
        .args(["-p", "process.execPath"])
        .output()
        .expect("node runs");
    assert!(
        found.status.success(),
        "node is not on PATH; the collection's bodies load under it"
    );
    Path::new(String::from_utf8_lossy(&found.stdout).trim())
        .parent()
        .expect("the runtime has a directory")
        .to_path_buf()
}

#[test]
fn every_reflex_checks_clean() {
    let dirs = reflexes();
    assert!(dirs.len() >= 13, "only {} reflexes found", dirs.len());
    let home = home();
    let path = format!(
        "{}:{}:{}",
        runtime_dir().display(),
        home.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    for dir in dirs {
        let name = dir
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        let output = Command::new(env!("CARGO_BIN_EXE_evoke"))
            .arg("check")
            .current_dir(copied(&dir))
            .env("HOME", &home)
            .env("PATH", &path)
            .env("TERM", "dumb")
            .env("NO_COLOR", "1")
            .stdin(Stdio::null())
            .output()
            .expect("evoke runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{name}: {stderr}");
        // The report is the row alone: no lint finding, and no tag to diff against.
        assert_eq!(stdout.lines().count(), 1, "{name}: {stdout}");
        assert!(
            stdout.starts_with(&format!("  {name}  ")),
            "{name}: {stdout}"
        );
        assert!(
            !stderr.lines().any(|line| line.starts_with("+ ")),
            "{name}: reflex.d.ts is stale; run evoke check in reflexes/{name}\n{stderr}"
        );
    }
}
