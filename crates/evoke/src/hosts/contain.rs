//! The layers around a body, applied. On Linux: a Landlock ruleset built in the parent from the core's rules,
//! `no_new_privs`, `landlock_restrict_self` and a seccomp filter that closes the network, all in the forked child
//! before `exec`, over `libc`. On macOS: `sandbox-exec -p` with the core's profile in front of the command. The
//! machine's status, probed once per process; the facts the core's rules need — where the runtime, the programs
//! and the declared paths are, whether the runtime holds the network, the resolver's file, the interpreters a
//! program runs through. In: a `Policy`, the runtime, the body's directory, the temporary folder, the state, the
//! environment. Out: `Facts` or what is missing, a `Command` armed, `Contained`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use evoke_core::Contained;
use evoke_core::contain::{Executable, Facts, Found, Platform, Runtime};
use evoke_core::needs::{Key, Lacking, Policy};
#[cfg(not(target_os = "linux"))]
use evoke_core::seatbelt;
use indexmap::IndexMap;

use super::state::State;
use super::{Environment, Failure, failed};

/// The facts the core's rules need, gathered: the runtime as the kernel runs it and whether it holds the network,
/// each declared path's real form and kind, each program's place on `PATH`, the home; a path or a program the
/// machine lacks is what is lacking, before anything runs.
pub fn facts(
    policy: &Policy,
    runtime: Option<&Path>,
    body_dir: &Path,
    tmp: &Path,
    state: &State,
    environment: &Environment,
) -> Result<Result<Facts, Lacking>, Failure> {
    let mut found = IndexMap::new();
    for (key, places) in [(Key::Reads, &policy.reads), (Key::Writes, &policy.writes)] {
        for place in places {
            let path = Path::new(&place.path);
            let Ok(meta) = fs::metadata(path) else {
                return Ok(Err(Lacking::Place {
                    place: place.clone(),
                    key,
                }));
            };
            let real = fs::canonicalize(path).map_err(|error| {
                failed(
                    &format!("finding {}", path.display()),
                    &super::cause(&error),
                )
            })?;
            found.insert(
                place.path.clone(),
                Found {
                    real: utf8(&real)?,
                    dir: meta.is_dir(),
                },
            );
        }
    }
    let mut programs = IndexMap::new();
    for program in &policy.runs {
        let Some(path) = locate(program.as_str(), environment) else {
            return Ok(Err(Lacking::Program {
                program: program.clone(),
            }));
        };
        programs.insert(program.as_str().to_owned(), executable(&path, environment)?);
    }
    let runtime = runtime
        .map(|runtime| runtime_of(runtime, state, environment))
        .transpose()?;
    Ok(Ok(Facts {
        platform: PLATFORM,
        runtime,
        body_dir: utf8(body_dir)?,
        tmp: utf8(tmp)?,
        home: environment.get("HOME").unwrap_or_default().to_owned(),
        resolver: resolver(),
        found,
        programs,
    }))
}

#[cfg(target_os = "linux")]
const PLATFORM: Platform = Platform::Linux;
#[cfg(not(target_os = "linux"))]
const PLATFORM: Platform = Platform::MacOs;

/// A program by its absolute path, or the first executable of its name on `PATH`.
fn locate(program: &str, environment: &Environment) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;
    let runs = |candidate: &Path| {
        candidate
            .metadata()
            .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
    };
    if program.starts_with('/') {
        let path = PathBuf::from(program);
        return runs(&path).then_some(path);
    }
    environment
        .get("PATH")?
        .split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| Path::new(dir).join(program))
        .find(|candidate| runs(candidate))
}

/// The runtime as the kernel runs it, and whether it holds the network: asked of it once per binary, the answer
/// kept in the cache. A runtime gone since it was recorded is the failure `evoke sync` answers, as its start
/// would be.
fn runtime_of(path: &Path, state: &State, environment: &Environment) -> Result<Runtime, Failure> {
    let binary = fs::metadata(path).map_err(|error| super::processes::not_started(path, &error))?;
    let program = executable(path, environment)?;
    let holds_network = if let Some(kept) = state.holds_network(&program.path, &binary)? {
        kept
    } else {
        let asked = super::processes::holds_network(path, environment)?;
        state.keep_holds_network(&program.path, &binary, asked)?;
        asked
    };
    Ok(Runtime {
        program,
        holds_network,
    })
}

/// A program as the kernel runs it: its real path, and the interpreters it runs through.
fn executable(path: &Path, environment: &Environment) -> Result<Executable, Failure> {
    let real = fs::canonicalize(path).map_err(|error| {
        failed(
            &format!("finding {}", path.display()),
            &super::cause(&error),
        )
    })?;
    Ok(Executable {
        interpreters: interpreters_of(&real, environment)?,
        path: utf8(&real)?,
    })
}

/// What the kernel executes to run a program, by real path: a script's shebang interpreter, and the program
/// `env` hands over to, found on `PATH`; then, on Linux, the ELF interpreter that loads a dynamic binary. Nothing
/// for a static binary. Read from the program's head, never the whole of a binary.
fn interpreters_of(program: &Path, environment: &Environment) -> Result<Vec<String>, Failure> {
    let mut interpreters = Vec::new();
    if let Some(words) = shebang(&head(program)?) {
        let mut through = vec![words[0].clone()];
        if words[0].ends_with("/env")
            && let Some(handed) = words.get(1).filter(|word| !word.starts_with('-'))
            && let Some(path) = locate(handed, environment)
        {
            through.push(utf8(&path)?);
        }
        for interpreter in through {
            let real = fs::canonicalize(&interpreter).map_err(|error| {
                failed(&format!("finding {interpreter}"), &super::cause(&error))
            })?;
            interpreters.push(utf8(&real)?);
            if let Some(loader) = elf_interpreter(&real)? {
                interpreters.push(loader);
            }
        }
    } else if let Some(loader) = elf_interpreter(program)? {
        interpreters.push(loader);
    }
    Ok(interpreters)
}

/// A program's first bytes, where a script names its interpreter.
fn head(program: &Path) -> Result<Vec<u8>, Failure> {
    use std::io::Read as _;
    let reading = |error: &std::io::Error| {
        failed(
            &format!("reading {}", program.display()),
            &super::cause(error),
        )
    };
    let file = fs::File::open(program).map_err(|error| reading(&error))?;
    let mut head = Vec::with_capacity(256);
    file.take(256)
        .read_to_end(&mut head)
        .map_err(|error| reading(&error))?;
    Ok(head)
}

/// The words of a script's first line after `#!`: the interpreter by its absolute path, then its arguments,
/// `#!/usr/bin/env node`.
fn shebang(bytes: &[u8]) -> Option<Vec<String>> {
    let rest = bytes.strip_prefix(b"#!")?;
    let line = rest.split(|byte| *byte == b'\n').next()?;
    let words: Vec<String> = line
        .split(u8::is_ascii_whitespace)
        .filter(|word| !word.is_empty())
        .map(|word| String::from_utf8(word.to_vec()).ok())
        .collect::<Option<_>>()?;
    words.first().filter(|word| word.starts_with('/'))?;
    Some(words)
}

/// The file `/etc/resolv.conf` really is, when it links out of `/etc`: what the resolver reads.
fn resolver() -> Option<String> {
    let real = fs::canonicalize("/etc/resolv.conf").ok()?;
    if real.starts_with("/etc") {
        return None;
    }
    real.to_str().map(str::to_owned)
}

fn utf8(path: &Path) -> Result<String, Failure> {
    path.to_str().map(str::to_owned).ok_or_else(|| Failure {
        what: format!("naming {}", path.display()),
        cause: Some("the path is not UTF-8".to_owned()),
        fix: evoke_core::Fix::Rerun,
    })
}

/// Whether this machine holds a declaration, probed once: on Linux by the kernel's Landlock ABI, on macOS by the
/// presence of `sandbox-exec`.
pub fn status() -> Contained {
    static STATUS: OnceLock<Contained> = OnceLock::new();
    STATUS.get_or_init(probe).clone()
}

/// The command armed with the layers: on Linux the ruleset built and the closures set to apply it in the child;
/// on macOS `sandbox-exec` in front of `program`. `arguments` follow the program as given.
#[cfg_attr(
    not(target_os = "linux"),
    expect(
        clippy::unnecessary_wraps,
        reason = "one signature for both systems; the Linux arm builds a ruleset that can fail"
    )
)]
pub fn command(program: &Path, policy: &Policy, facts: &Facts) -> Result<Command, Failure> {
    #[cfg(target_os = "linux")]
    {
        let mut command = Command::new(program);
        linux::arm(&mut command, policy, facts)?;
        Ok(command)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let mut command = Command::new(SANDBOX_EXEC);
        command.arg("-p").arg(seatbelt(policy, facts)).arg(program);
        Ok(command)
    }
}

#[cfg(not(target_os = "linux"))]
const SANDBOX_EXEC: &str = "/usr/bin/sandbox-exec";

#[cfg(target_os = "linux")]
fn probe() -> Contained {
    Contained::at_landlock(linux::abi())
}

#[cfg(not(target_os = "linux"))]
fn probe() -> Contained {
    use std::os::unix::fs::PermissionsExt as _;
    let runs = fs::metadata(SANDBOX_EXEC)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0);
    if runs {
        Contained::Full
    } else {
        Contained::None {
            why: "sandbox-exec is missing".to_owned(),
        }
    }
}

/// The interpreter a program's ELF header names, `PT_INTERP`, by its real path: what the kernel executes to
/// load a dynamically linked program, so a rule that lets the program run lets its interpreter run too. None for
/// a static binary or anything that is no ELF; nothing on macOS, whose loader the system's roots cover.
#[cfg(target_os = "linux")]
fn elf_interpreter(program: &Path) -> Result<Option<String>, Failure> {
    use std::os::unix::fs::FileExt as _;
    let file = fs::File::open(program).map_err(|error| {
        failed(
            &format!("reading {}", program.display()),
            &super::cause(&error),
        )
    })?;
    let at = |at: u64, len: usize| {
        let mut bytes = vec![0; len];
        file.read_exact_at(&mut bytes, at).ok().map(|()| bytes)
    };
    let Some(interpreter) = elf::interpreter(at) else {
        return Ok(None);
    };
    let real = fs::canonicalize(&interpreter)
        .map_err(|error| failed(&format!("finding {interpreter}"), &super::cause(&error)))?;
    utf8(&real).map(Some)
}

#[cfg(not(target_os = "linux"))]
#[expect(clippy::unnecessary_wraps, reason = "one signature for both systems")]
fn elf_interpreter(_program: &Path) -> Result<Option<String>, Failure> {
    Ok(None)
}

/// `PT_INTERP` out of an ELF image, 32- or 64-bit, little-endian, read by offset: the header, the program
/// headers, the one segment; never the whole image.
#[cfg(target_os = "linux")]
mod elf {
    pub fn interpreter(read: impl Fn(u64, usize) -> Option<Vec<u8>>) -> Option<String> {
        let head = read(0, 64)?;
        if head.get(..4) != Some(b"\x7fELF") || head.get(5) != Some(&1) {
            return None;
        }
        let wide = *head.get(4)? == 2;
        let (phoff, phentsize, phnum) = if wide {
            (u64(&head, 0x20)?, u16(&head, 0x36)?, u16(&head, 0x38)?)
        } else {
            (
                u64::from(u32(&head, 0x1c)?),
                u16(&head, 0x2a)?,
                u16(&head, 0x2c)?,
            )
        };
        let (phentsize, phnum) = (usize::from(phentsize), usize::from(phnum));
        let headers = read(
            phoff,
            phentsize.checked_mul(phnum).filter(|len| *len <= 1 << 20)?,
        )?;
        for n in 0..phnum {
            let header = n * phentsize;
            if u32(&headers, header)? != 3 {
                continue;
            }
            let (offset, size) = if wide {
                (u64(&headers, header + 8)?, u64(&headers, header + 32)?)
            } else {
                (
                    u64::from(u32(&headers, header + 4)?),
                    u64::from(u32(&headers, header + 16)?),
                )
            };
            let text = read(
                offset,
                usize::try_from(size).ok().filter(|size| *size <= 4096)?,
            )?;
            let text = text.split(|byte| *byte == 0).next()?;
            return std::str::from_utf8(text).ok().map(str::to_owned);
        }
        None
    }

    fn u16(bytes: &[u8], at: usize) -> Option<u16> {
        Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
    }

    fn u32(bytes: &[u8], at: usize) -> Option<u32> {
        Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
    }

    fn u64(bytes: &[u8], at: usize) -> Option<u64> {
        Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
    }
}

/// Landlock, `no_new_privs` and seccomp, as the kernel takes them.
#[cfg(target_os = "linux")]
mod linux {
    // The syscalls Landlock and seccomp are: `std` has no door to them.
    #![expect(unsafe_code)]

    use std::ffi::CString;
    use std::io;
    use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd};
    use std::os::unix::process::CommandExt as _;
    use std::process::Command;
    use std::sync::OnceLock;

    use evoke_core::contain::{Facts, Right, Rule};
    use evoke_core::landlock;
    use evoke_core::needs::{Hosts, Policy};

    use super::super::{Failure, failed};

    const LANDLOCK_CREATE_RULESET_VERSION: u32 = 1;
    const LANDLOCK_RULE_PATH_BENEATH: u32 = 1;
    const NET_BIND_TCP: u64 = 1;
    const NET_CONNECT_TCP: u64 = 2;
    const SCOPE_ABSTRACT_UNIX_SOCKET: u64 = 1;
    const SCOPE_SIGNAL: u64 = 2;
    /// What a rule on a file may carry: the rest are a directory's.
    const FILE_RIGHTS: u64 = bit(Right::Execute)
        | bit(Right::WriteFile)
        | bit(Right::ReadFile)
        | bit(Right::Truncate)
        | bit(Right::IoctlDev);

    #[repr(C)]
    struct RulesetAttr {
        handled_access_fs: u64,
        handled_access_net: u64,
        scoped: u64,
    }

    #[repr(C, packed)]
    struct PathBeneathAttr {
        allowed_access: u64,
        parent_fd: i32,
    }

    /// Each right's bit, as `linux/landlock.h` numbers them.
    const fn bit(right: Right) -> u64 {
        match right {
            Right::Execute => 1 << 0,
            Right::WriteFile => 1 << 1,
            Right::ReadFile => 1 << 2,
            Right::ReadDir => 1 << 3,
            Right::RemoveDir => 1 << 4,
            Right::RemoveFile => 1 << 5,
            Right::MakeChar => 1 << 6,
            Right::MakeDir => 1 << 7,
            Right::MakeReg => 1 << 8,
            Right::MakeSock => 1 << 9,
            Right::MakeFifo => 1 << 10,
            Right::MakeBlock => 1 << 11,
            Right::MakeSym => 1 << 12,
            Right::Refer => 1 << 13,
            Right::Truncate => 1 << 14,
            Right::IoctlDev => 1 << 15,
        }
    }

    /// The file-system rights a kernel at an ABI knows: `refer` from 2, `truncate` from 3, `ioctl_dev` from 5.
    fn known(abi: u32) -> u64 {
        match abi {
            0 => 0,
            1 => 0x1fff,
            2 => 0x3fff,
            3 | 4 => 0x7fff,
            _ => 0xffff,
        }
    }

    /// The kernel's Landlock ABI, probed once: 0 without Landlock, or with it built in but not enabled.
    pub fn abi() -> u32 {
        static ABI: OnceLock<u32> = OnceLock::new();
        *ABI.get_or_init(|| {
            // SAFETY: a version probe takes no attribute; the kernel answers the ABI or an error.
            let answer = unsafe {
                libc::syscall(
                    libc::SYS_landlock_create_ruleset,
                    std::ptr::null::<RulesetAttr>(),
                    0usize,
                    LANDLOCK_CREATE_RULESET_VERSION,
                )
            };
            u32::try_from(answer).unwrap_or(0)
        })
    }

    /// The ruleset built and its rules added here, in the parent; the child, forked, sets `no_new_privs`, adds
    /// the rule on its own `/proc/self`, restricts itself to the ruleset, and installs the seccomp filter that
    /// closes what the declaration leaves closed — syscalls only, nothing allocated after the fork.
    pub fn arm(command: &mut Command, policy: &Policy, facts: &Facts) -> Result<(), Failure> {
        let abi = abi();
        let (ruleset, own) = if abi > 0 {
            let (ruleset, own) = ruleset(abi, policy, facts)?;
            (Some(ruleset), own)
        } else {
            (None, None)
        };
        let mut domains = Vec::new();
        if policy.hosts == Hosts::None {
            domains.extend([libc::AF_INET, libc::AF_INET6]);
        }
        if policy.runs.is_empty() {
            domains.push(libc::AF_UNIX);
        }
        let filter = (!domains.is_empty()).then(|| filter(&domains));
        // SAFETY: the closure runs in the forked child before exec and makes syscalls only: prctl, one rule over
        // a path made in the parent, the Landlock restriction over an fd built there, and the seccomp filter over
        // a buffer moved into it.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
                if let Some(ruleset) = &ruleset {
                    // `/proc/self` is this process's from here on: the rule lands on the body's own entry.
                    if let Some((path, allowed)) = &own {
                        let fd = libc::open(path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC);
                        if fd >= 0 {
                            let rule = PathBeneathAttr {
                                allowed_access: *allowed,
                                parent_fd: fd,
                            };
                            let added = libc::syscall(
                                libc::SYS_landlock_add_rule,
                                ruleset.as_raw_fd(),
                                LANDLOCK_RULE_PATH_BENEATH,
                                &raw const rule,
                                0u32,
                            );
                            let error = (added < 0).then(io::Error::last_os_error);
                            libc::close(fd);
                            if let Some(error) = error {
                                return Err(error);
                            }
                        }
                    }
                    if libc::syscall(libc::SYS_landlock_restrict_self, ruleset.as_raw_fd(), 0u32)
                        < 0
                    {
                        return Err(io::Error::last_os_error());
                    }
                }
                if let Some(filter) = &filter {
                    let program = libc::sock_fprog {
                        len: u16::try_from(filter.len()).expect("a short filter"),
                        filter: filter.as_ptr().cast_mut(),
                    };
                    if libc::prctl(
                        libc::PR_SET_SECCOMP,
                        libc::SECCOMP_MODE_FILTER,
                        &raw const program,
                    ) < 0
                    {
                        return Err(io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        Ok(())
    }

    /// The ruleset: every right the ABI knows handled, TCP handled and unruled when no host is allowed, signals
    /// and abstract sockets scoped from ABI 6; then one rule per path the machine has, its rights masked by the
    /// ABI and, on a file, to a file's. The rule on `/proc/self` is handed back for the child to add, since
    /// opened here it would be the parent's own entry.
    fn ruleset(
        abi: u32,
        policy: &Policy,
        facts: &Facts,
    ) -> Result<(OwnedFd, Option<(CString, u64)>), Failure> {
        let what = "building the sandbox";
        let attr = RulesetAttr {
            handled_access_fs: known(abi),
            handled_access_net: if abi >= 4 && policy.hosts == Hosts::None {
                NET_BIND_TCP | NET_CONNECT_TCP
            } else {
                0
            },
            scoped: if abi >= 6 {
                SCOPE_ABSTRACT_UNIX_SOCKET | SCOPE_SIGNAL
            } else {
                0
            },
        };
        // SAFETY: the attribute is a complete struct of the size given; the kernel answers an fd or an error.
        let fd = unsafe {
            libc::syscall(
                libc::SYS_landlock_create_ruleset,
                &raw const attr,
                std::mem::size_of::<RulesetAttr>(),
                0u32,
            )
        };
        if fd < 0 {
            return Err(failed(what, &os_error()));
        }
        // SAFETY: the kernel just handed this fd to no one else.
        let ruleset = unsafe { OwnedFd::from_raw_fd(i32::try_from(fd).expect("an fd fits")) };
        let mut own = None;
        for Rule { path, rights } in landlock(policy, facts) {
            let Ok(meta) = std::fs::metadata(&path) else {
                continue;
            };
            let mut allowed = rights.iter().fold(0, |acc, right| acc | bit(*right)) & known(abi);
            if !meta.is_dir() {
                allowed &= FILE_RIGHTS;
            }
            let c_path =
                CString::new(path.as_str()).map_err(|_| failed(what, "a path holds a NUL"))?;
            if path == "/proc/self" {
                own = Some((c_path, allowed));
                continue;
            }
            // SAFETY: an O_PATH open of a C string; the fd is closed below.
            let parent = unsafe { libc::open(c_path.as_ptr(), libc::O_PATH | libc::O_CLOEXEC) };
            if parent < 0 {
                continue;
            }
            let rule = PathBeneathAttr {
                allowed_access: allowed,
                parent_fd: parent,
            };
            // SAFETY: the rule is a complete struct over an open fd, added to the ruleset just made.
            let added = unsafe {
                libc::syscall(
                    libc::SYS_landlock_add_rule,
                    ruleset.as_raw_fd(),
                    LANDLOCK_RULE_PATH_BENEATH,
                    &raw const rule,
                    0u32,
                )
            };
            let why = (added < 0).then(os_error);
            // SAFETY: the fd was opened above and is used nowhere else.
            unsafe {
                libc::close(parent);
            }
            if let Some(why) = why {
                return Err(failed(what, &format!("{path}: {why}")));
            }
        }
        Ok((ruleset, own))
    }

    /// The seccomp filter over `socket`: a domain listed answers `EACCES`, so the Internet is closed without a
    /// host — TCP, UDP and the resolver alike, whatever Landlock's ABI holds — and the machine's own sockets,
    /// the session's bus among them, without a program to run. `io_uring`, another door to a socket, is closed
    /// with them. On `x86_64` the x32 numbers, which carry the same architecture, are refused first; a foreign
    /// architecture runs nothing.
    fn filter(domains: &[i32]) -> Vec<libc::sock_filter> {
        // `AUDIT_ARCH_*` from `linux/audit.h`: the machine bits with the 64-bit and little-endian flags.
        #[cfg(target_arch = "aarch64")]
        const ARCH: u32 = 0xc000_00b7;
        #[cfg(target_arch = "x86_64")]
        const ARCH: u32 = 0xc000_003e;
        const LOAD: u32 = libc::BPF_LD | libc::BPF_W | libc::BPF_ABS;
        const EQUAL: u32 = libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K;
        const RETURN: u32 = libc::BPF_RET | libc::BPF_K;
        let number = |syscall: libc::c_long| u32::try_from(syscall).expect("a syscall number");
        let stmt = |code: u32, k: u32| libc::sock_filter {
            code: u16::try_from(code).expect("a BPF code"),
            jt: 0,
            jf: 0,
            k,
        };
        let jump = |code: u32, k: u32, jt: u8, jf: u8| libc::sock_filter {
            code: u16::try_from(code).expect("a BPF code"),
            jt,
            jf,
            k,
        };
        let errno = |errno: i32| libc::SECCOMP_RET_ERRNO | u32::try_from(errno).expect("an errno");
        let refused = u8::try_from(domains.len()).expect("a few domains");
        let mut program = vec![
            stmt(LOAD, 4), // the architecture
            jump(EQUAL, ARCH, 1, 0),
            stmt(RETURN, errno(libc::EPERM)),
            stmt(LOAD, 0), // the syscall number
        ];
        #[cfg(target_arch = "x86_64")]
        program.extend([
            jump(
                libc::BPF_JMP | libc::BPF_JSET | libc::BPF_K,
                0x4000_0000,
                0,
                1,
            ),
            stmt(RETURN, errno(libc::EACCES)),
        ]);
        program.extend([
            jump(EQUAL, number(libc::SYS_io_uring_setup), 0, 1),
            stmt(RETURN, errno(libc::EACCES)),
            jump(EQUAL, number(libc::SYS_socket), 0, refused + 2),
            stmt(LOAD, 16), // the domain, the first argument's low word
        ]);
        for (n, domain) in domains.iter().enumerate() {
            let left = refused - 1 - u8::try_from(n).expect("a few domains");
            let domain = u32::try_from(*domain).expect("a domain");
            program.push(jump(EQUAL, domain, left, u8::from(left == 0)));
        }
        program.extend([
            stmt(RETURN, errno(libc::EACCES)),
            stmt(RETURN, libc::SECCOMP_RET_ALLOW),
        ]);
        program
    }

    fn os_error() -> String {
        super::super::cause(&io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_is_found_by_its_name_or_its_path() {
        let environment = Environment(std::collections::BTreeMap::from([(
            "PATH".to_owned(),
            "/nowhere:/bin:/usr/bin".to_owned(),
        )]));
        let sh = locate("sh", &environment).unwrap();
        assert!(sh.ends_with("bin/sh"));
        assert_eq!(
            locate("/bin/sh", &environment),
            Some(PathBuf::from("/bin/sh"))
        );
        assert_eq!(locate("no-such-program", &environment), None);
        assert_eq!(locate("/nowhere/sh", &environment), None);
    }

    #[test]
    fn the_status_is_probed_once_and_reads_as_one_word() {
        let status = status();
        assert_eq!(status, super::status());
        assert!(status.to_string().contains("contained"));
    }

    #[test]
    fn a_script_runs_through_its_shebang_and_a_dynamic_program_through_its_loader() {
        let script = std::env::temp_dir().join(format!("evoke-shebang-{}", std::process::id()));
        fs::write(&script, "#!/bin/sh -e\necho hi\n").unwrap();
        let environment = Environment(std::collections::BTreeMap::from([(
            "PATH".to_owned(),
            "/bin:/usr/bin".to_owned(),
        )]));
        let interpreters = interpreters_of(&script, &environment).unwrap();
        assert_eq!(
            interpreters[0],
            fs::canonicalize("/bin/sh").unwrap().to_str().unwrap()
        );
        fs::write(&script, "#!/usr/bin/env sh\necho hi\n").unwrap();
        let interpreters = interpreters_of(&script, &environment).unwrap();
        assert!(
            interpreters.iter().any(|path| path.ends_with("/env"))
                && interpreters.contains(
                    &fs::canonicalize("/bin/sh")
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned()
                ),
            "{interpreters:?}"
        );
        let _ = fs::remove_file(&script);
        assert_eq!(
            shebang(b"#!/usr/bin/env node\n"),
            Some(vec!["/usr/bin/env".to_owned(), "node".to_owned()])
        );
        assert_eq!(shebang(b"#!node\n"), None);
        assert_eq!(shebang(b"echo\n"), None);
        #[cfg(target_os = "linux")]
        {
            let sh = fs::canonicalize("/bin/sh").unwrap();
            let loader = elf_interpreter(&sh).unwrap();
            assert!(
                loader.as_deref().is_none_or(|path| path.contains("ld-")),
                "{loader:?}"
            );
            assert_eq!(elf::interpreter(|_, _| Some(b"\x7fELF".to_vec())), None);
        }
    }
}
