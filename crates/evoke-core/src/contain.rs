//! The layers that hold a body to its policy, as the text and the rules a host applies: on macOS a Seatbelt
//! profile for `sandbox-exec -p`; on Linux the Landlock rules, one per path; on both Node's permission flags, the
//! inner layer whose errors name a path. In: a `Policy` and the host's `Facts` — the platform, the runtime and
//! whether it holds the network, the body's directory, the private temporary folder, what the host found at each
//! path and program. Out: the profile, the flags, the rules; and `Contained`, whether a machine holds the whole
//! declaration. Pinned by vectors, so neither host writes a line of it.

use std::fmt;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

use crate::needs::{Hosts, Place, Policy};

/// What the host knows that the policy does not: where things are.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    pub platform: Platform,
    /// The runtime a file body runs under; none for an argv body.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<Runtime>,
    /// The body's directory: readable, and its working directory.
    pub body_dir: String,
    /// The private temporary folder made for the run, by its real path: `TMPDIR`, readable and writable.
    pub tmp: String,
    /// The home: the runtime's installation is readable whole unless that would be the home or above it.
    pub home: String,
    /// Linux: the file `/etc/resolv.conf` really is when it links out of `/etc`, which the resolver reads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolver: Option<String>,
    /// What the host found at each of the policy's paths, by the path as the policy spells it.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub found: IndexMap<String, Found>,
    /// Where each program the policy names is, by the name as the policy spells it.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub programs: IndexMap<String, Executable>,
}

/// The runtime a file body runs under: its program, as `process.execPath` names it, and whether its permission
/// model holds the network, which it does from Node 25, the first to know `--allow-net`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Runtime {
    #[serde(flatten)]
    pub program: Executable,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub holds_network: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    MacOs,
}

/// A program as the kernel runs it: its path, and the interpreters the kernel executes to run it — a script's
/// shebang interpreter, then the ELF interpreter that loads a dynamic binary — each of which a rule must let run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Executable {
    pub path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interpreters: Vec<String>,
}

/// A path as the host found it: its real path, links followed, and whether it is a directory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Found {
    pub real: String,
    pub dir: bool,
}

/// One Landlock rule: the path, and the rights beneath it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub path: String,
    pub rights: Vec<Right>,
}

/// The file-system rights Landlock names, ABI 1 to 5.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Right {
    Execute,
    WriteFile,
    ReadFile,
    ReadDir,
    RemoveDir,
    RemoveFile,
    MakeChar,
    MakeDir,
    MakeReg,
    MakeSock,
    MakeFifo,
    MakeBlock,
    MakeSym,
    Refer,
    Truncate,
    IoctlDev,
}

/// Whether the machine holds the whole declaration: fully; partly, with what it does not hold; not at all, with
/// why. Printed once at `add` and `show`, and on every run's line, when it is not full.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Contained {
    Full,
    Partial { why: String },
    None { why: String },
}

impl Contained {
    /// What a Linux kernel at a Landlock ABI holds: everything from ABI 3; before it, not a truncation, and
    /// before ABI 2 not a move across folders either; without Landlock, nothing but what Node holds.
    #[must_use]
    pub fn at_landlock(abi: u32) -> Self {
        match abi {
            0 => Self::None {
                why: "this kernel has no Landlock".to_owned(),
            },
            1 => Self::Partial {
                why: "Landlock ABI 1: a move across folders and truncation are held from ABI 3"
                    .to_owned(),
            },
            2 => Self::Partial {
                why: "Landlock ABI 2: truncation is held from ABI 3".to_owned(),
            },
            _ => Self::Full,
        }
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        matches!(self, Self::Full)
    }
}

/// `contained`, `partly contained (why)`, `not contained (why)`.
impl fmt::Display for Contained {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => f.write_str("contained"),
            Self::Partial { why } => write!(f, "partly contained ({why})"),
            Self::None { why } => write!(f, "not contained ({why})"),
        }
    }
}

/// What the macOS runtime profile reads: the system's roots, coarse on purpose, since they move with each release.
const MAC_ROOTS: [&str; 9] = [
    "/usr",
    "/System",
    "/Library",
    "/private/etc",
    "/private/var/db",
    "/dev",
    "/bin",
    "/sbin",
    "/opt",
];

/// What the Linux runtime profile reads: the roots a runtime and a program need, each skipped by the host when
/// the machine lacks it.
const LINUX_ROOTS: [&str; 9] = [
    "/usr",
    "/lib",
    "/lib64",
    "/bin",
    "/sbin",
    "/etc",
    "/opt",
    "/nix",
    "/proc/self",
];

const READ_DIR: [Right; 2] = [Right::ReadFile, Right::ReadDir];
const WRITE_DIR: [Right; 12] = [
    Right::ReadFile,
    Right::ReadDir,
    Right::WriteFile,
    Right::RemoveDir,
    Right::RemoveFile,
    Right::MakeDir,
    Right::MakeReg,
    Right::MakeSym,
    Right::MakeFifo,
    Right::MakeSock,
    Right::Refer,
    Right::Truncate,
];
const EXECUTE: [Right; 2] = [Right::ReadFile, Right::Execute];
/// The shells macOS's `/bin/sh` re-executes itself as, by `/var/select/sh`.
const SHELLS: [&str; 3] = ["/bin/bash", "/bin/zsh", "/bin/dash"];
const READ_WRITE_FILE: [Right; 3] = [Right::ReadFile, Right::WriteFile, Right::Truncate];

/// The deny-default Seatbelt profile around one body: what any process needs, the runtime's own files, the
/// body's directory, the temporary folder, then the declaration; the system's services only with a program to
/// run; the network and the resolver only with any host.
#[must_use]
pub fn seatbelt(policy: &Policy, facts: &Facts) -> String {
    let mut reads: Vec<String> = MAC_ROOTS.iter().map(|root| subpath(root)).collect();
    if let Some(prefix) = facts
        .runtime
        .as_ref()
        .and_then(|runtime| prefix(&runtime.program.path, &facts.home))
    {
        reads.push(subpath(&prefix));
    }
    reads.push(subpath(&facts.body_dir));
    reads.push(subpath(&facts.tmp));
    let mut writes = vec![subpath(&facts.tmp), literal("/dev/null")];
    for place in policy.reads.iter().chain(&policy.writes) {
        let (path, dir) = located(place, facts);
        reads.push(if dir { subpath(&path) } else { literal(&path) });
    }
    for place in &policy.writes {
        let (path, dir) = located(place, facts);
        writes.push(if dir { subpath(&path) } else { literal(&path) });
    }
    let mut execs: IndexSet<String> = facts
        .runtime
        .iter()
        .map(|runtime| runtime.program.path.clone())
        .collect();
    for program in &policy.runs {
        match facts.programs.get(program.as_str()) {
            Some(found) => {
                // A script is read by its interpreter, which `process-exec` alone does not allow.
                reads.push(literal(&found.path));
                execs.extend(
                    std::iter::once(&found.path)
                        .chain(&found.interpreters)
                        .cloned(),
                );
            }
            None => {
                execs.insert(program.as_str().to_owned());
            }
        }
    }
    // `/bin/sh` runs as the shell `/var/select/sh` names: whatever runs through it runs through that one too.
    if execs.contains("/bin/sh") {
        execs.extend(SHELLS.iter().map(|shell| (*shell).to_owned()));
    }
    let execs: Vec<String> = execs.iter().map(|path| literal(path)).collect();
    let mut lines = vec![
        "(version 1)".to_owned(),
        "(deny default)".to_owned(),
        "(allow process-fork)".to_owned(),
        "(allow signal (target same-sandbox))".to_owned(),
        "(allow sysctl-read)".to_owned(),
        "(allow file-read-metadata)".to_owned(),
        "(allow file-read-data (literal \"/\"))".to_owned(),
        format!("(allow file-read* {})", reads.join(" ")),
        format!("(allow file-write* {})", writes.join(" ")),
        format!("(allow process-exec {})", execs.join(" ")),
    ];
    if !policy.runs.is_empty() {
        lines.extend(
            [
                "(allow mach-lookup)",
                "(allow appleevent-send)",
                "(allow lsopen)",
                "(allow iokit-open)",
                "(allow ipc-posix-shm*)",
                "(allow user-preference-read)",
                "(allow distributed-notification-post)",
            ]
            .map(str::to_owned),
        );
    }
    if policy.hosts == Hosts::Any {
        lines.extend(
            [
                "(allow network-outbound)",
                "(allow mach-lookup (global-name \"com.apple.SystemConfiguration.DNSConfiguration\"))",
                "(allow system-socket)",
            ]
            .map(str::to_owned),
        );
    }
    let mut profile = lines.join("\n");
    profile.push('\n');
    profile
}

/// Node's permission flags: the body's directory and the temporary folder, each declared path as spelled and
/// again as its real path when that differs, since Node compares strings; the network only with a host to reach;
/// a child process only with a program to run, without the warning Node prints for it. A runtime that does not
/// hold the network, before Node 25, leaves it to the kernel, and would refuse the flag that opens it.
#[must_use]
pub fn node_flags(policy: &Policy, facts: &Facts) -> Vec<String> {
    let spellings = |place: &Place| -> Vec<String> {
        let mut paths = vec![place.path.clone()];
        if let Some(found) = facts.found.get(&place.path)
            && found.real != place.path
        {
            paths.push(found.real.clone());
        }
        paths
    };
    let mut flags = vec![
        "--permission".to_owned(),
        format!("--allow-fs-read={}", facts.body_dir),
        format!("--allow-fs-read={}", facts.tmp),
        format!("--allow-fs-write={}", facts.tmp),
    ];
    for place in policy.reads.iter().chain(&policy.writes) {
        flags.extend(
            spellings(place)
                .into_iter()
                .map(|path| format!("--allow-fs-read={path}")),
        );
    }
    for place in &policy.writes {
        flags.extend(
            spellings(place)
                .into_iter()
                .map(|path| format!("--allow-fs-write={path}")),
        );
    }
    if policy.hosts == Hosts::Any
        && facts
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.holds_network)
    {
        flags.push("--allow-net".to_owned());
    }
    if !policy.runs.is_empty() {
        // By its type, which every release gives the warning; few give it a code.
        flags.extend(
            ["--allow-child-process", "--disable-warning=SecurityWarning"].map(str::to_owned),
        );
    }
    flags
}

/// The Landlock rules: the system's roots readable, the resolver's file, the devices a runtime reads, the
/// temporary folder writable, the runtime's prefix readable and its binary and interpreter executable, the
/// body's directory, then the declaration. The host skips a path the machine lacks and masks the rights by
/// the kernel's ABI.
#[must_use]
pub fn landlock(policy: &Policy, facts: &Facts) -> Vec<Rule> {
    let rule = |path: &str, rights: &[Right]| Rule {
        path: path.to_owned(),
        rights: rights.to_vec(),
    };
    let mut rules: Vec<Rule> = LINUX_ROOTS
        .iter()
        .map(|root| rule(root, &READ_DIR))
        .collect();
    if let Some(resolver) = &facts.resolver {
        rules.push(rule(resolver, &[Right::ReadFile]));
    }
    rules.push(rule("/dev/null", &[Right::ReadFile, Right::WriteFile]));
    for device in ["/dev/urandom", "/dev/random", "/dev/zero"] {
        rules.push(rule(device, &[Right::ReadFile]));
    }
    rules.push(rule(&facts.tmp, &WRITE_DIR));
    if let Some(runtime) = &facts.runtime {
        if let Some(prefix) = prefix(&runtime.program.path, &facts.home) {
            rules.push(rule(&prefix, &READ_DIR));
        }
        rules.extend(executable(&runtime.program));
    }
    rules.push(rule(&facts.body_dir, &READ_DIR));
    for place in &policy.reads {
        let (path, dir) = located(place, facts);
        rules.push(rule(
            &path,
            if dir { &READ_DIR } else { &[Right::ReadFile] },
        ));
    }
    for place in &policy.writes {
        let (path, dir) = located(place, facts);
        rules.push(rule(&path, if dir { &WRITE_DIR } else { &READ_WRITE_FILE }));
    }
    for program in &policy.runs {
        if let Some(found) = facts.programs.get(program.as_str()) {
            rules.extend(executable(found));
        }
    }
    rules
}

/// A program executable, and every interpreter the kernel runs it through.
fn executable(found: &Executable) -> Vec<Rule> {
    std::iter::once(&found.path)
        .chain(&found.interpreters)
        .map(|path| Rule {
            path: path.clone(),
            rights: EXECUTE.to_vec(),
        })
        .collect()
}

/// A place by its real path, and whether it is a directory; unfound, as spelled and taken for a directory.
fn located(place: &Place, facts: &Facts) -> (String, bool) {
    facts
        .found
        .get(&place.path)
        .map_or((place.path.clone(), true), |found| {
            (found.real.clone(), found.dir)
        })
}

/// The runtime's installation, readable whole: two directories up from `<prefix>/bin/node`, else the binary's
/// own directory; none when that would be the home, a directory above it, or the root, so a binary copied to
/// `~/bin` runs with its own file alone.
fn prefix(runtime: &str, home: &str) -> Option<String> {
    let (parent, _) = runtime.rsplit_once('/')?;
    let candidate = match parent.rsplit_once('/') {
        Some((above, "bin")) if !above.is_empty() => above,
        _ => parent,
    };
    let home = home.trim_end_matches('/');
    let over_home = candidate.is_empty()
        || candidate == home
        || home
            .strip_prefix(candidate)
            .is_some_and(|rest| rest.starts_with('/'));
    (!over_home).then(|| candidate.to_owned())
}

fn subpath(path: &str) -> String {
    format!("(subpath {})", quoted(path))
}

fn literal(path: &str) -> String {
    format!("(literal {})", quoted(path))
}

/// A string as SBPL writes it.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_reads_off_the_abi() {
        assert_eq!(Contained::at_landlock(8), Contained::Full);
        assert_eq!(
            Contained::at_landlock(2).to_string(),
            "partly contained (Landlock ABI 2: truncation is held from ABI 3)"
        );
        assert_eq!(
            Contained::at_landlock(0).to_string(),
            "not contained (this kernel has no Landlock)"
        );
        assert_eq!(
            serde_json::to_value(Contained::Full).unwrap(),
            serde_json::json!({ "type": "full" })
        );
    }

    #[test]
    fn a_runtime_prefix_is_two_up_and_never_the_home() {
        let home = "/home/me";
        let prefix = |runtime: &str| prefix(runtime, home);
        assert_eq!(prefix("/usr/local/bin/node").as_deref(), Some("/usr/local"));
        assert_eq!(
            prefix("/home/me/.nvm/versions/node/v24.5.0/bin/node").as_deref(),
            Some("/home/me/.nvm/versions/node/v24.5.0")
        );
        assert_eq!(prefix("/bin/node").as_deref(), Some("/bin"));
        assert_eq!(prefix("/home/me/bin/node"), None);
        assert_eq!(prefix("/home/me/node"), None);
        assert_eq!(prefix("/node"), None);
        assert_eq!(quoted("a\"b"), "\"a\\\"b\"");
    }
}
