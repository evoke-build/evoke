//! A pseudo-terminal for one command: the child gets the slave as stdin, stdout, stderr and controlling terminal;
//! the parent reads the master until the child is gone, typing each answer as its prompt shows.

// The one place in the workspace that needs unsafe: libc's openpty returns raw descriptors, and pre_exec runs
// between fork and exec, where only async-signal-safe calls are allowed — setsid and one ioctl are.
#![allow(unsafe_code)]

use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::FromRawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;

/// Runs the command under a fresh pseudo-terminal; `typed` answers the prompts in order, each once the output so
/// far ends with `> `; a prompt with no answer left gets the end of input. Returns everything the terminal showed,
/// `\r` stripped, and the exit code.
pub fn run(mut command: Command, typed: &[String]) -> (String, i32) {
    let (mut master, slave) = open();
    let (stdout, stderr) = (slave.try_clone().unwrap(), slave.try_clone().unwrap());
    command.stdin(slave).stdout(stdout).stderr(stderr);
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 || libc::ioctl(0, libc::TIOCSCTTY, 0) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().expect("sh spawns");
    // The parent's copies of the slave close with the command, so the master sees the end when the child is gone.
    drop(command);
    let mut output = Vec::new();
    let mut typed = typed.iter();
    let mut chunk = [0; 4096];
    loop {
        match master.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                output.extend_from_slice(&chunk[..n]);
                if output.ends_with(b"> ") {
                    let answer = typed
                        .next()
                        .map_or_else(|| "\x04".to_owned(), |text| format!("{text}\n"));
                    master
                        .write_all(answer.as_bytes())
                        .expect("the terminal takes input");
                }
            }
            // Linux ends a master with EIO once every slave is closed; macOS with EOF.
            Err(error) if error.raw_os_error() == Some(libc::EIO) => break,
            Err(error) => panic!("reading the terminal: {error}"),
        }
    }
    let status = child.wait().expect("the child is waited for");
    (
        String::from_utf8_lossy(&output).replace('\r', ""),
        status.code().unwrap_or(-1),
    )
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
