//! Children: the loader warmed at t=0 and fed an envelope, its stdin kept open until it exits; a program spawned with
//! an argv; a body imported by the probe for `check`, never called; all under a scrubbed environment, in their own
//! process group, killed as a group on timeout or when there is nothing to run. In: the runtime's path; an envelope
//! line; an argv with what it adds to the environment; a body's directory and entrypoint; a deadline. Out: what the
//! body returned, whether it loads, or a `Failure`.

// killpg signals the group, which `std` can create but not signal.
#![allow(unsafe_code)]

use std::io::{self, Read, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use evoke_core::manifest::Run;
use evoke_core::name::ConfigKey;
use evoke_core::plan::Millis;
use evoke_core::project::Setting;
use evoke_core::{Envelope, Fix, Json};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{Deadline, Environment, Failure};

/// The JavaScript runtime `add` and `sync` record: `node` on `PATH`, when it is there.
#[must_use]
pub fn find_runtime(environment: &Environment) -> Option<std::path::PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    environment
        .get("PATH")?
        .split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| Path::new(dir).join("node"))
        .find(|candidate| {
            candidate
                .metadata()
                .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        })
}

/// The loader, embedded: the same file the SDK ships.
const LOADER: &str = include_str!("../../runtime/loader.mjs");

/// The probe `check` runs, embedded: the body imported, its default export a function, nothing called.
const PROBE: &str = include_str!("../../runtime/check.mjs");

/// What the scrubbed environment keeps.
const KEPT: [&str; 5] = ["PATH", "HOME", "TMPDIR", "LANG", "TERM"];

/// What a group gets after SIGTERM before SIGKILL; the loader gives a body the same to settle after an abort, so the
/// host waits twice that past the deadline for the loader's own report before ending the group.
const GRACE: Duration = Duration::from_secs(1);

/// What a body returned: its text, and data when it gave some.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Returned {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Json>,
}

/// The loader's one line: a result, or why there is none.
#[derive(Deserialize)]
#[serde(untagged)]
enum Line {
    Returned(Returned),
    Error { error: String },
}

/// The envelope as the loader reads it: the body's absolute path, and config resolved to values.
#[derive(Serialize)]
struct Fed<'a> {
    run: String,
    args: &'a IndexMap<evoke_core::name::ArgName, Json>,
    input: &'a str,
    config: IndexMap<&'a ConfigKey, String>,
    deadline: Millis,
}

/// Each config setting as its value; a variable that is not set is the failure it names.
fn resolved<'a>(
    what: &str,
    envelope: &'a Envelope,
    environment: &Environment,
) -> Result<IndexMap<&'a ConfigKey, String>, Failure> {
    envelope
        .config
        .iter()
        .map(|(key, setting)| {
            let value = match setting {
                Setting::Plain { value } => value.clone(),
                Setting::Env { var } => environment
                    .get(var.as_str())
                    .map(str::to_owned)
                    .ok_or_else(|| Failure {
                        what: what.to_owned(),
                        cause: Some(format!("{var} is not set")),
                        fix: Fix::ExportKey { var: var.clone() },
                    })?,
            };
            Ok((key, value))
        })
        .collect()
}

/// A loader started at t=0, waiting for its envelope.
pub struct Warm {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
}

/// Starts the runtime with the loader, so it is ready when the decision is. The loader warms Node's type
/// stripper through an API still marked experimental: its warning is off, so a body's stderr is the body's.
pub fn warm(runtime: &Path, environment: &Environment) -> Result<Warm, Failure> {
    let mut command = scrubbed(runtime, environment);
    command
        .args([
            "--disable-warning=ExperimentalWarning",
            "--input-type=module",
            "-e",
            LOADER,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = command
        .spawn()
        .map_err(|error| not_started(runtime, &error))?;
    let stdin = child.stdin.take().expect("stdin is piped");
    let stdout = child.stdout.take().expect("stdout is piped");
    Ok(Warm {
        child,
        stdin,
        stdout,
    })
}

impl Warm {
    /// The envelope in — its body under `dir`, its config resolved — the result out, within the deadline; the
    /// child is reaped either way. `what` names the reflex, for the failure.
    pub fn feed(
        mut self,
        what: &str,
        envelope: &Envelope,
        dir: &Path,
        environment: &Environment,
        deadline: Deadline,
    ) -> Result<Returned, Failure> {
        let run = match &envelope.run {
            Run::File(entrypoint) => dir.join(entrypoint.path().as_str()),
            Run::Inline | Run::Argv { .. } => {
                self.dismiss();
                return Err(failed(what, "the reflex has no file to run"));
            }
        };
        let config = match resolved(what, envelope, environment) {
            Ok(config) => config,
            Err(failure) => {
                self.dismiss();
                return Err(failure);
            }
        };
        let line = serde_json::to_string(&Fed {
            run: run.display().to_string(),
            args: &envelope.args,
            input: envelope.input.as_str(),
            config,
            deadline: envelope.deadline,
        })
        .expect("the envelope serializes");
        let fed = self
            .stdin
            .write_all(line.as_bytes())
            .and_then(|()| self.stdin.write_all(b"\n"))
            .and_then(|()| self.stdin.flush());
        if let Err(error) = fed {
            self.dismiss();
            return Err(failed(what, &format!("feeding the envelope: {error}")));
        }
        let read = collect(self.stdout, &mut self.child, deadline);
        // Stdin stays open until the child is gone: a body's life is bounded by its parent's.
        drop(self.stdin);
        let (output, status) = read.map_err(|why| failed(what, &why))?;
        let line = output.lines().next().unwrap_or_default();
        match serde_json::from_str::<Line>(line) {
            Ok(Line::Returned(returned)) => Ok(returned),
            Ok(Line::Error { error }) => Err(failed(what, &error)),
            Err(_) if line.is_empty() => Err(failed(what, &ended(status, "without a result"))),
            Err(_) => Err(failed(
                what,
                &format!("the result line is not JSON: {}", shortened(line)),
            )),
        }
    }

    /// Nothing to run: the group ends and is reaped.
    pub fn dismiss(mut self) {
        drop(self.stdin);
        drop(self.stdout);
        end(&mut self.child);
    }
}

/// Whether a body loads: imported by the runtime with a function as its default export, or why not — in the
/// author's terms, Node's own stderr having said where.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probed {
    Loads,
    Refused(String),
}

/// The body `run` names under `dir`, imported by the probe under the scrubbed environment within the deadline;
/// `run` stays relative, so the reason names the file as the author does.
pub fn probe(
    runtime: &Path,
    dir: &Path,
    run: &str,
    environment: &Environment,
    deadline: Deadline,
) -> Result<Probed, Failure> {
    let what = format!("loading {run}");
    let mut command = scrubbed(runtime, environment);
    command
        .args(["--input-type=module", "-e", PROBE, "--", run])
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = command
        .spawn()
        .map_err(|error| not_started(runtime, &error))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let (output, status) =
        collect(stdout, &mut child, deadline).map_err(|why| failed(&what, &why))?;
    if status.success() {
        return Ok(Probed::Loads);
    }
    let line = output.lines().next().unwrap_or_default();
    match serde_json::from_str::<Line>(line) {
        Ok(Line::Error { error }) => Ok(Probed::Refused(error)),
        _ => Err(failed(&what, &ended(status, "without a reason"))),
    }
}

/// A program with its argv; config reaches it as `EVOKE_CONFIG_<KEY>` and the input as `EVOKE_INPUT`; its stdout
/// is the text.
pub fn program(
    what: &str,
    argv: &[String],
    envelope: &Envelope,
    environment: &Environment,
    deadline: Deadline,
) -> Result<Returned, Failure> {
    let (program, rest) = argv.split_first().expect("an argv has a program");
    let config = resolved(what, envelope, environment)?;
    let mut command = scrubbed(Path::new(program), environment);
    command
        .args(rest)
        .envs(config.iter().map(|(key, value)| {
            (
                format!("EVOKE_CONFIG_{}", key.as_str().to_uppercase()),
                value,
            )
        }))
        .env("EVOKE_INPUT", envelope.input.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut child = command
        .spawn()
        .map_err(|error| failed(what, &format!("{program}: {}", super::cause(&error))))?;
    let stdout = child.stdout.take().expect("stdout is piped");
    let (output, status) =
        collect(stdout, &mut child, deadline).map_err(|why| failed(what, &why))?;
    if !status.success() {
        return Err(failed(what, &ended(status, "")));
    }
    Ok(Returned {
        text: output.strip_suffix('\n').unwrap_or(&output).to_owned(),
        data: None,
    })
}

/// The first characters of a line that did not read, so a body's whole output never lands in a diagnostic.
fn shortened(line: &str) -> String {
    const SHOWN: usize = 120;
    let mut chars = line.chars();
    let head: String = chars.by_ref().take(SHOWN).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

/// A command under the scrubbed environment, in its own group; what a caller adds comes on top.
fn scrubbed(program: &Path, environment: &Environment) -> Command {
    let mut command = Command::new(program);
    command.env_clear().process_group(0);
    for var in KEPT {
        if let Some(value) = environment.get(var) {
            command.env(var, value);
        }
    }
    command
}

/// Everything the child writes until it closes stdout, then its status; past the deadline and the loader's grace
/// — for the output, and again for the exit, so a child that closes stdout and lingers is ended too — the group
/// is ended and the wait is the failure.
fn collect(
    mut stdout: ChildStdout,
    child: &mut Child,
    deadline: Deadline,
) -> Result<(String, ExitStatus), String> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut output = Vec::new();
        let read = stdout.read_to_end(&mut output).map(|_| output);
        let _ = sender.send(read);
    });
    match receiver.recv_timeout(deadline.remaining() + 2 * GRACE) {
        Ok(Ok(output)) => {
            let Some(status) = exited(child, deadline.remaining() + 2 * GRACE) else {
                end(child);
                return Err("did not finish before the deadline".to_owned());
            };
            Ok((String::from_utf8_lossy(&output).into_owned(), status))
        }
        Ok(Err(error)) => {
            end(child);
            Err(format!("reading the result: {error}"))
        }
        Err(_) => {
            end(child);
            Err("did not finish before the deadline".to_owned())
        }
    }
}

/// The runtime would not start. Gone since it was recorded — a Node removed or moved — is `evoke sync`, which
/// records the one on `PATH` again.
fn not_started(runtime: &Path, error: &io::Error) -> Failure {
    let what = format!("starting {}", runtime.display());
    if error.kind() == io::ErrorKind::NotFound {
        return Failure {
            what,
            cause: Some("it is gone".to_owned()),
            fix: Fix::Sync,
        };
    }
    failed(&what, &super::cause(error))
}

/// A child's output — stdout and stderr drained as it runs, so neither pipe fills — and its status, within the
/// duration; past it the child's group is ended and there is none. The child was spawned in a group of its own,
/// with both streams piped.
pub(super) fn output_within(mut child: Child, within: Duration) -> Option<Output> {
    let stdout = child.stdout.take().map(drained);
    let stderr = child.stderr.take().map(drained);
    let Some(status) = exited(&mut child, within) else {
        end(&mut child);
        return None;
    };
    let bytes = |reader: Option<thread::JoinHandle<Vec<u8>>>| {
        reader
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default()
    };
    Some(Output {
        status,
        stdout: bytes(stdout),
        stderr: bytes(stderr),
    })
}

/// A pipe read to its end on a thread of its own.
fn drained<R: Read + Send + 'static>(mut reader: R) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        bytes
    })
}

/// SIGTERM to the group, a grace, SIGKILL; the child reaped.
fn end(child: &mut Child) {
    let group = i32::try_from(child.id()).expect("a pid fits");
    // SAFETY: killpg takes a group id and a signal; the group is the child's own, made at spawn.
    unsafe {
        libc::killpg(group, libc::SIGTERM);
    }
    if exited(child, GRACE).is_none() {
        unsafe {
            libc::killpg(group, libc::SIGKILL);
        }
        let _ = child.wait();
    }
}

/// The child's status when it exits within the duration, polled; none when it is still there.
fn exited(child: &mut Child, within: Duration) -> Option<ExitStatus> {
    let until = Instant::now() + within;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() >= until => return None,
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(_) => return None,
        }
    }
}

fn ended(status: ExitStatus, and: &str) -> String {
    let how = match status.code() {
        Some(code) => format!("exited {code}"),
        None => "was killed".to_owned(),
    };
    if and.is_empty() {
        how
    } else {
        format!("{how} {and}")
    }
}

fn failed(what: &str, cause: &str) -> Failure {
    Failure {
        what: what.to_owned(),
        cause: Some(cause.to_owned()),
        fix: Fix::Rerun,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(script: &str) -> Child {
        Command::new("sh")
            .args(["-c", script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .expect("sh spawns")
    }

    #[test]
    fn a_child_within_the_bound_gives_its_output_and_one_past_it_nothing() {
        let output = output_within(sh("echo out; echo err >&2"), Duration::from_secs(5)).unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"out\n");
        assert_eq!(output.stderr, b"err\n");
        let started = Instant::now();
        assert!(output_within(sh("sleep 30"), Duration::from_millis(200)).is_none());
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_runtime_that_is_gone_is_a_sync() {
        let gone = not_started(
            Path::new("/nowhere/node"),
            &io::Error::from(io::ErrorKind::NotFound),
        );
        assert_eq!(gone.what, "starting /nowhere/node");
        assert_eq!(gone.fix, Fix::Sync);
        let denied = not_started(
            Path::new("/nowhere/node"),
            &io::Error::from(io::ErrorKind::PermissionDenied),
        );
        assert_eq!(denied.fix, Fix::Rerun);
    }
}
