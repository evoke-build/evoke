//! Runs `spec/transcripts/<flow>/` against the built binary, as spec/README.md specifies: a throwaway home under
//! `target/`, the JavaScript runtime found on `PATH` recorded in its state, each `$ ` line run by `sh -c` under a
//! pseudo-terminal — or, when the flow says `# no tty`, with no terminal at all, in a session of its own, so the
//! terminal of whoever runs the suite does not reach it — with `TERM=dumb` and `NO_COLOR=1` so the terminal shows
//! plain text, `replay` answering from `EVOKE_ANSWERS`, remotes rebuilt from their trees by the recipe. Every line must
//! match, trailing spaces aside, JSON as JSON with `ms` aside; `[N]` is the exit code; a line `^C` is Ctrl-C typed
//! once the line before it has shown, and the terminal's echo of it. One test per flow; each is turned on by the
//! step that makes it pass.

#[path = "transcripts/pty.rs"]
mod pty;

use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use serde_json::Value as Json;

/// The recipe's author and committer, at 2026-09-19T00:00:00Z.
const AUTHOR: &str = "evoke spec";
const EMAIL: &str = "spec@evoke.build";
const DATE: &str = "1789776000 +0000";

#[test]
fn r#try() {
    flow("try");
}

#[test]
fn r#use() {
    flow("use");
}

#[test]
fn ask() {
    flow("ask");
}

#[test]
fn abstain() {
    flow("abstain");
}

#[test]
fn no_terminal() {
    flow("no-terminal");
}

#[test]
fn why() {
    flow("why");
}

#[test]
fn first_run() {
    flow("first-run");
}

#[test]
fn first_ten() {
    flow("first-ten");
}

#[test]
fn openjev() {
    flow("openjev");
}

#[test]
fn test_nothing() {
    flow("test-nothing");
}

#[test]
fn repl() {
    flow("repl");
}

#[test]
fn teach() {
    flow("teach");
}

#[test]
fn run() {
    flow("run");
}

#[test]
fn show() {
    flow("show");
}

#[test]
fn vocabulary() {
    flow("vocabulary");
}

#[test]
fn config() {
    flow("config");
}

#[test]
fn add() {
    flow("add");
}

#[test]
fn update() {
    flow("update");
}

#[test]
fn untrusted() {
    flow("untrusted");
}

#[test]
fn new() {
    flow("new");
}

#[test]
fn check() {
    flow("check");
}

#[test]
fn test() {
    flow("test");
}

#[test]
fn calibrate() {
    flow("calibrate");
}

#[test]
fn help() {
    flow("help");
}

#[test]
fn weave() {
    flow("weave");
}

#[test]
fn cache() {
    flow("cache");
}

#[test]
fn contained() {
    flow("contained");
}

#[test]
fn cancel() {
    flow("cancel");
}

/// The recipe yields the commit `spec/transcripts/update/home/.config/evoke/evoke.lock` records.
#[test]
fn the_recipe_reproduces_the_locked_commit() {
    let work = throwaway("recipe");
    let trees = tagged(&spec().join("transcripts/update/remote/radhi/home"));
    let commits = bare_repository(&work.join("home"), &trees, &work);
    assert_eq!(
        commits.first().map(String::as_str),
        Some("7a498d11d5375f3cb65c575c3186bc64ecac7f52")
    );
    assert_eq!(commits.len(), 2);
}

fn spec() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// A fresh directory under the build's own `target/transcripts/`, beside the binary under test, so two builds'
/// runs never share a flow's home.
fn throwaway(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_BIN_EXE_evoke"))
        .ancestors()
        .nth(2)
        .expect("the binary sits at target/<profile>/evoke")
        .join("transcripts")
        .join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("the last run's directory goes");
    }
    fs::create_dir_all(&dir).expect("the directory is created");
    dir
}

/// One `$ ` line: the command, what the terminal showed, the exit code.
struct Step {
    command: String,
    expected: Vec<String>,
    code: i32,
}

/// Runs one flow: the home, the environment, the remotes, then every step in order.
fn flow(name: &str) {
    let dir = spec().join("transcripts").join(name);
    let session = fs::read_to_string(dir.join("session.txt")).expect("session.txt");
    let tty = !session.lines().any(|line| line.starts_with("# no tty"));
    let (home, environment) = prepared(name, &dir);
    for step in steps(&session) {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg(&step.command)
            .current_dir(&home)
            .env_clear()
            .envs(environment.iter().map(|(var, value)| (var, value)));
        let ran = if tty {
            pty::run(command, &typed(&step.expected))
        } else {
            piped(command)
        };
        match ran {
            Ok((output, code)) => compare(&step, &output, code),
            Err(why) => panic!("$ {}\n{why}", step.command),
        }
    }
}

/// A flow's ground under `target/transcripts/<name>/`: its home copied, the runtime recorded, its remotes built,
/// and the environment every command runs under.
fn prepared(name: &str, dir: &Path) -> (PathBuf, Vec<(String, String)>) {
    let work = throwaway(name);
    let home = work.join("home");
    let source = if dir.join("home").is_dir() {
        dir.join("home")
    } else {
        spec().join("transcripts/home")
    };
    copy(&source, &home);
    runtime(&home);
    remotes(dir, &home, &work);
    let mut environment = vec![
        ("PATH".to_owned(), path(&home)),
        ("HOME".to_owned(), utf8(&home)),
        ("XDG_CONFIG_HOME".to_owned(), utf8(&home.join(".config"))),
        (
            "XDG_STATE_HOME".to_owned(),
            utf8(&home.join(".local/state")),
        ),
        ("XDG_CACHE_HOME".to_owned(), utf8(&home.join(".cache"))),
        ("GIT_CONFIG_NOSYSTEM".to_owned(), "1".to_owned()),
        // The plain text is what is asserted: no colour, no spinner, no line editor.
        ("TERM".to_owned(), "dumb".to_owned()),
        ("NO_COLOR".to_owned(), "1".to_owned()),
    ];
    let answers = dir.join("answers.toml");
    if answers.is_file() {
        environment.push(("EVOKE_ANSWERS".to_owned(), utf8(&answers)));
    }
    (home, environment)
}

/// What was asked for reaches stdout and what `evoke` says about it stderr. A transcript reads both off one pipe
/// and cannot tell; this reads them apart over the answering commands, on the `try` flow's ground.
#[test]
fn answers_reach_stdout_and_notes_stderr() {
    let (home, environment) = prepared("streams", &spec().join("transcripts/try"));
    let run = |command: &str| {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&home)
            .env_clear()
            .envs(environment.iter().map(|(var, value)| (var, value)))
            .stdin(Stdio::null())
            .output()
            .expect("sh runs");
        (
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };
    let (out, err) = run("evoke try \"kill the lights in the den\"");
    assert!(out.starts_with("  lights 0.91 · none 0.06"), "{out}");
    assert_eq!(err, "");
    let (out, err) = run("evoke \"kill the lights in the den\"");
    assert_eq!(out, "den lights off\n");
    assert_eq!(err, "  lights room=\"den\" state=\"off\"  0.85\n");
    let (out, err) = run("evoke vocab rooms");
    assert!(out.starts_with("  den = "), "{out}");
    assert_eq!(err, "");
    let (out, err) = run("evoke --help");
    assert!(out.starts_with("evoke "), "{out}");
    assert_eq!(err, "");
    let (out, err) = run("evoke show --json");
    assert_eq!(out, "");
    assert!(err.ends_with("  →  evoke --help\n"), "{err}");
}

/// The steps of a session: a `$ ` line opens one, `[N]` closes it with a code, `#` lines are notes.
fn steps(session: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    for line in session.lines() {
        if let Some(command) = line.strip_prefix("$ ") {
            steps.push(Step {
                command: command.to_owned(),
                expected: Vec::new(),
                code: 0,
            });
        } else if let Some(step) = steps.last_mut()
            && !line.starts_with('#')
        {
            match exit_code(line) {
                Some(code) => step.code = code,
                None => step.expected.push(line.to_owned()),
            }
        }
    }
    steps
}

/// `[N]` alone on a line.
fn exit_code(line: &str) -> Option<i32> {
    line.strip_prefix('[')?.strip_suffix(']')?.parse().ok()
}

/// What the harness types, in order: each prompt line's answer after its `> `, and Ctrl-C where a line reads
/// `^C`, once the terminal has shown the line before it.
fn typed(expected: &[String]) -> Vec<pty::Typed> {
    let mut typed = Vec::new();
    for (i, line) in expected.iter().enumerate() {
        if line == "^C" {
            let after = expected[..i].last().cloned().unwrap_or_default();
            typed.push(pty::Typed::Interrupt { after });
        } else if let Some(answer) = prompt(line) {
            typed.push(pty::Typed::Answer(answer.to_owned()));
        }
    }
    typed
}

/// What a line typed at a prompt, when the line is one: the REPL's `> ` at its start, or a question — two spaces
/// in, ending in `?` and two spaces before its choices — with `> ` and the answer at its end. A line of a body's
/// own output holding `> ` is no prompt, and a prompt with nothing after its `>` is answered with the end of input.
fn prompt(line: &str) -> Option<&str> {
    if let Some(rest) = line.strip_prefix("> ") {
        return Some(rest);
    }
    if line.starts_with("  ") && line.contains("?  ") {
        return line.rfind("> ").map(|at| &line[at + 2..]);
    }
    None
}

/// The command with stdout and stderr on one pipe, as they came, and no terminal anywhere: stdin is nothing, and
/// the session is its own, so a prompt finds no `/dev/tty` however the suite is run; ended whole when the
/// deadline runs out.
fn piped(mut command: Command) -> Result<(String, i32), String> {
    let (mut reader, writer) = std::io::pipe().expect("a pipe opens");
    let stderr = writer.try_clone().expect("the pipe is shared");
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(writer))
        .stderr(Stdio::from(stderr));
    pty::detach(&mut command);
    let mut child = command.spawn().expect("sh spawns");
    drop(command);
    let drained = std::thread::spawn(move || {
        let mut output = Vec::new();
        let _ = reader.read_to_end(&mut output);
        output
    });
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("the child is waited for") {
            break status;
        }
        if started.elapsed() >= pty::DEADLINE {
            pty::end_group(&mut child);
            let output = drained.join().unwrap_or_default();
            return Err(format!(
                "did not finish within {} s; the pipe held:\n{}",
                pty::DEADLINE.as_secs(),
                String::from_utf8_lossy(&output)
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    let output = drained.join().unwrap_or_default();
    Ok((
        String::from_utf8_lossy(&output).into_owned(),
        pty::code(status),
    ))
}

/// Every expected line against the actual one, and the exit code; a mismatch shows both whole.
fn compare(step: &Step, output: &str, code: i32) {
    let actual: Vec<&str> = output.lines().collect();
    let matched = actual.len() == step.expected.len()
        && step
            .expected
            .iter()
            .zip(&actual)
            .all(|(expected, actual)| same(expected, actual))
        && code == step.code;
    assert!(
        matched,
        "$ {}\n--- expected, exit {}\n{}\n--- actual, exit {code}\n{}\n---",
        step.command,
        step.code,
        step.expected.join("\n"),
        actual.join("\n")
    );
}

/// Exactly but for trailing spaces, or as JSON with every `ms` ignored and every number one kind, so `0.85` and
/// `0.850`, `1` and `1.0`, are one value, as the spec states.
fn same(expected: &str, actual: &str) -> bool {
    if expected.starts_with('{') {
        match (
            serde_json::from_str::<Json>(expected),
            serde_json::from_str::<Json>(actual),
        ) {
            (Ok(expected), Ok(actual)) => normalized(expected) == normalized(actual),
            _ => false,
        }
    } else {
        expected.trim_end() == actual.trim_end()
    }
}

/// Every `ms` dropped, every number an `f64`.
fn normalized(value: Json) -> Json {
    match value {
        Json::Object(fields) => fields
            .into_iter()
            .filter(|(key, _)| key != "ms")
            .map(|(key, value)| (key, normalized(value)))
            .collect(),
        Json::Array(items) => items.into_iter().map(normalized).collect(),
        Json::Number(n) => n.as_f64().map_or(Json::Null, Json::from),
        other => other,
    }
}

/// The binary's directory first, then the flow's own `bin/` under its home when it has one — stand-ins for the
/// programs its reflexes declare and run — then whatever `sh`, `git` and the rest are found on.
fn path(home: &Path) -> String {
    let bin = Path::new(env!("CARGO_BIN_EXE_evoke"))
        .parent()
        .expect("the binary has a directory");
    let rest = std::env::var("PATH").unwrap_or_default();
    let own = home.join("bin");
    let runtime = runtime_dir();
    if own.is_dir() {
        format!(
            "{}:{}:{}:{rest}",
            bin.display(),
            own.display(),
            runtime.display()
        )
    } else {
        format!("{}:{}:{rest}", bin.display(), runtime.display())
    }
}

/// The JavaScript runtime `node` names on `PATH`, as the binary it runs as, recorded in the home's state as
/// `evoke sync` will record it: a version manager's shim cannot run contained.
fn runtime(home: &Path) {
    let dir = home.join(".local/state/evoke");
    fs::create_dir_all(&dir).expect("the state directory is created");
    fs::write(dir.join("runtime"), runtime_path()).expect("the runtime is recorded");
}

/// The runtime's real path, `node -p process.execPath`, asked once in the harness's own environment: a version
/// manager's shim answers only in its owner's home, and a flow's home is its own.
fn runtime_path() -> &'static str {
    static PATH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PATH.get_or_init(|| {
        let found = Command::new("node")
            .args(["-p", "process.execPath"])
            .output()
            .expect("node runs");
        assert!(
            found.status.success(),
            "node is not on PATH; the transcripts run file bodies with it"
        );
        String::from_utf8(found.stdout)
            .expect("a path")
            .trim()
            .to_owned()
    })
}

/// The runtime's directory, first on every flow's `PATH` after the binary's and the home's own, so `add` and
/// `sync` find the binary itself, never a shim.
fn runtime_dir() -> PathBuf {
    Path::new(runtime_path())
        .parent()
        .expect("the runtime has a directory")
        .to_path_buf()
}

/// A path as text; everything under `target/` is UTF-8.
fn utf8(path: &Path) -> String {
    path.to_str()
        .expect("a path under target/ is UTF-8")
        .to_owned()
}

/// A directory copied whole.
fn copy(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("the directory is created");
    for entry in fs::read_dir(from).expect("the directory lists") {
        let entry = entry.expect("an entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("the file copies");
        }
    }
}

/// Every `remote/<owner>/<repo>/` as a bare repository beside the home, reached through `url.<path>.insteadOf` in
/// the home's `.gitconfig`.
fn remotes(flow: &Path, home: &Path, work: &Path) {
    let remote = flow.join("remote");
    if !remote.is_dir() {
        return;
    }
    // The mirrors are local paths, a transport `evoke` itself never allows: the person's own configuration does.
    let mut gitconfig = String::from("[protocol \"file\"]\n\tallow = always\n");
    for owner in sorted(&remote) {
        for repo in sorted(&owner) {
            let bare = work
                .join("remotes")
                .join(owner.file_name().expect("an owner"))
                .join(repo.file_name().expect("a repo"));
            bare_repository(&bare, &tagged(&repo), work);
            let _ = writeln!(
                gitconfig,
                "[url \"{}\"]\n\tinsteadOf = https://github.com/{}/{}",
                bare.display(),
                owner.file_name().expect("an owner").to_string_lossy(),
                repo.file_name().expect("a repo").to_string_lossy()
            );
        }
    }
    fs::write(home.join(".gitconfig"), gitconfig).expect(".gitconfig is written");
}

/// A remote's tag directories in version order, each with its tag.
fn tagged(repo: &Path) -> Vec<(String, PathBuf)> {
    let mut tags: Vec<((u32, u32, u32), String, PathBuf)> = sorted(repo)
        .into_iter()
        .map(|tree| {
            let tag = tree
                .file_name()
                .expect("a tag")
                .to_string_lossy()
                .into_owned();
            let mut parts = tag
                .split('.')
                .map(|part| part.parse::<u32>().expect("a version part"));
            let mut next = || parts.next().expect("three parts");
            ((next(), next(), next()), tag, tree)
        })
        .collect();
    tags.sort();
    tags.into_iter().map(|(_, tag, tree)| (tag, tree)).collect()
}

/// The recipe: a bare repository with one commit per tag in version order, each committed by the spec at one
/// instant with the tag as its message, tagged without `v`. Returns the commits in order. `git_home` is git's
/// `HOME`, kept away from the user's own `.gitconfig`. A tree is copied first, links followed, so a tag that
/// links to the collection's own directories commits their files, never the links.
fn bare_repository(bare: &Path, trees: &[(String, PathBuf)], git_home: &Path) -> Vec<String> {
    let git = |args: &[&str], cwd: &Path, index: &Path| -> String {
        let output = Command::new("git")
            .arg("--git-dir")
            .arg(bare)
            .args(args)
            .current_dir(cwd)
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap_or_default())
            .env("HOME", git_home)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_INDEX_FILE", index)
            .env("GIT_AUTHOR_NAME", AUTHOR)
            .env("GIT_AUTHOR_EMAIL", EMAIL)
            .env("GIT_AUTHOR_DATE", DATE)
            .env("GIT_COMMITTER_NAME", AUTHOR)
            .env("GIT_COMMITTER_EMAIL", EMAIL)
            .env("GIT_COMMITTER_DATE", DATE)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };
    fs::create_dir_all(bare).expect("the bare directory is created");
    let index = bare.join("index");
    git(&["init", "--quiet", "--bare"], bare, &index);
    let mut commits: Vec<String> = Vec::new();
    for (tag, linked) in trees {
        let tree = &git_home
            .join("trees")
            .join(bare.file_name().expect("a repo"))
            .join(tag);
        copy(linked, tree);
        git(
            &["--work-tree", &utf8(tree), "add", "--all", "."],
            tree,
            &index,
        );
        let written = git(&["write-tree"], bare, &index);
        let mut args = vec!["commit-tree", &written, "-m", tag];
        if let Some(parent) = commits.last() {
            args.extend(["-p", parent]);
        }
        let commit = git(&args, bare, &index);
        git(&["tag", tag, &commit], bare, &index);
        git(&["update-ref", "refs/heads/main", &commit], bare, &index);
        let _ = fs::remove_file(&index);
        commits.push(commit);
    }
    commits
}

/// A directory's entries in name order.
fn sorted(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
        .map(|entry| entry.expect("an entry").path())
        .collect();
    entries.sort();
    entries
}
