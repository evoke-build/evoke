//! A pseudo-terminal for one command: the child gets the slave as stdin, stdout, stderr and controlling terminal;
//! the parent reads the master until the child is gone, typing each answer as its prompt shows, and Ctrl-C once
//! the line before it has shown.

// The one place in the workspace that needs unsafe: libc's openpty returns raw descriptors, and pre_exec runs
// between fork and exec, where only async-signal-safe calls are allowed — setsid and one ioctl are.
#![expect(unsafe_code)]

use std::fs::File;
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Command, ExitStatus};
use std::time::{Duration, Instant};

/// The most one step may take: past it, the child's session is ended and the step fails, named.
pub const DEADLINE: Duration = Duration::from_secs(60);

/// What the harness types: an answer, once a prompt shows; or Ctrl-C, once the terminal has shown a line.
pub enum Typed {
    Answer(String),
    Interrupt { after: String },
}

/// Runs the command under a fresh pseudo-terminal; `typed` is typed in order — an answer once the output so far
/// ends with `> `, Ctrl-C once it ends with the line named — and a prompt with no answer left gets the end of
/// input. Returns everything the terminal showed, `\r` stripped, and the exit code — a signal's as a shell
/// reports it, 128 and its number; a step that has not finished within the deadline is the error, with what the
/// terminal showed so far.
pub fn run(mut command: Command, typed: &[Typed]) -> Result<(String, i32), String> {
    let (mut master, slave) = open();
    let (stdout, stderr) = (slave.try_clone().unwrap(), slave.try_clone().unwrap());
    command.stdin(slave).stdout(stdout).stderr(stderr);
    // The request's type follows the libc: a `c_ulong` on Linux, a `u32` on macOS.
    #[cfg(target_os = "macos")]
    let tiocsctty = libc::c_ulong::from(libc::TIOCSCTTY);
    #[cfg(not(target_os = "macos"))]
    let tiocsctty = libc::TIOCSCTTY;
    unsafe {
        command.pre_exec(move || {
            if libc::setsid() == -1 || libc::ioctl(0, tiocsctty, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().expect("sh spawns");
    // The parent's copies of the slave close with the command, so the master sees the end when the child is gone.
    drop(command);
    let mut output = Vec::new();
    let mut typed = typed.iter().peekable();
    let mut chunk = [0; 4096];
    let started = Instant::now();
    loop {
        if !readable(&master, DEADLINE.saturating_sub(started.elapsed())) {
            // The child set up its own session, so its process group is its pid: the whole session ends with it.
            end_group(&mut child);
            return Err(format!(
                "did not finish within {} s; the terminal showed:\n{}",
                DEADLINE.as_secs(),
                String::from_utf8_lossy(&output).replace('\r', "")
            ));
        }
        match master.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                output.extend_from_slice(&chunk[..n]);
                let typing = match typed.peek() {
                    Some(Typed::Answer(text)) if output.ends_with(b"> ") => {
                        typed.next();
                        Some(format!("{text}\n"))
                    }
                    Some(Typed::Interrupt { after }) if shown(&output, after) => {
                        typed.next();
                        Some("\x03".to_owned())
                    }
                    Some(Typed::Interrupt { .. }) | None if output.ends_with(b"> ") => {
                        Some("\x04".to_owned())
                    }
                    _ => None,
                };
                if let Some(typing) = typing {
                    master
                        .write_all(typing.as_bytes())
                        .expect("the terminal takes input");
                }
            }
            // Linux ends a master with EIO once every slave is closed; macOS with EOF.
            Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
            Err(error) => panic!("reading the terminal: {error}"),
        }
    }
    let status = child.wait().expect("the child is waited for");
    let shown = String::from_utf8_lossy(&output).replace('\r', "");
    // macOS's terminal echoes the end of input typed here as `^D` and rubs it out; Linux's echoes nothing, as
    // the flows expect.
    #[cfg(target_os = "macos")]
    let shown = shown.replace("^D\x08\x08", "").replace("^D", "");
    Ok((shown, code(status)))
}

/// Whether the terminal has just shown the line: the output so far, `\r` aside, ends with it and its line end.
fn shown(output: &[u8], line: &str) -> bool {
    let tail = &output[output.len().saturating_sub(line.len() + 8)..];
    String::from_utf8_lossy(tail)
        .replace('\r', "")
        .ends_with(&format!("{line}\n"))
}

/// The exit code as a shell reports it: the code, or 128 and the signal's number for a child a signal ended.
pub fn code(status: ExitStatus) -> i32 {
    status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(-1)
}

/// SIGKILL to the child's process group — its own, made at spawn — and the child reaped.
pub fn end_group(child: &mut std::process::Child) {
    // SAFETY: killpg takes a group id and a signal; the group is the child's own.
    unsafe {
        libc::killpg(
            i32::try_from(child.id()).expect("a pid fits"),
            libc::SIGKILL,
        );
    }
    let _ = child.wait();
}

/// Whether the master has something to read, or has closed, within the wait; false when the wait ran out.
fn readable(master: &File, within: Duration) -> bool {
    let mut polled = libc::pollfd {
        fd: master.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let millis = i32::try_from(within.as_millis()).unwrap_or(i32::MAX);
    loop {
        // SAFETY: one pollfd, owned here, for the length given.
        let ready = unsafe { libc::poll(&raw mut polled, 1, millis) };
        if ready >= 0 {
            return ready > 0;
        }
        let error = io::Error::last_os_error();
        assert!(
            error.kind() == io::ErrorKind::Interrupted,
            "polling the terminal: {error}"
        );
    }
}

/// A master and slave pair.
fn open() -> (File, File) {
    let (mut master, mut slave) = (-1, -1);
    // SAFETY: openpty fills the two descriptors; both are owned by the Files from here on, closed once each.
    let opened = unsafe {
        libc::openpty(
            &raw mut master,
            &raw mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(opened, 0, "openpty: {}", std::io::Error::last_os_error());
    unsafe { (File::from_raw_fd(master), File::from_raw_fd(slave)) }
}
