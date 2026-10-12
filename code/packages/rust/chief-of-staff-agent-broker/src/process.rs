//! The broker process's entry point (D18S S-K7, P2.6d-1; shared from
//! P2.6d-2c).
//!
//! ```text
//!   start ──► suppress core dumps (P2.6a), before any key exists
//!         ──► read Bootstrap from stdin: the binding, and the key slot table
//!         ──► check descriptors 3..3+n are open, and no other above 2 is
//!         ──► read each key from its descriptor, re-checking the owner-only
//!             policy on the descriptor itself; close the descriptor
//!         ──► Ready { public halves } on stdout
//!         ──► serve requests, one at a time, until Terminate (exit 0)
//! ```
//!
//! Any failure exits non-zero with no output: the supervisor ends the agent
//! when its broker ends. Nothing is written to stderr. The supervisor points
//! it at /dev/null, and an error message is no place for a key.
//!
//! This is the one module of the library that uses `unsafe`: taking
//! ownership of inherited descriptors has no safe API. It used to live in
//! the binary, so the library could be `forbid(unsafe_code)`. But every
//! binary that runs a broker needs it, and two test crates had copied it.
//! One reviewed copy here is better than three, so the library is
//! `deny(unsafe_code)` with this module the single exception.

#![allow(unsafe_code)]

use std::process::ExitCode;

/// Run this process as a broker: stdin and stdout are its pipes to the
/// supervisor, and descriptors 3..3+n are its keys. Every binary that runs
/// a broker is `fn main() -> ExitCode { run_inherited() }`.
///
/// **Call it only as the whole of `main`, in a process the broker launcher
/// exec'd.** It takes ownership of descriptors 3..3+n on the strength of
/// their having been inherited across that exec, which only the launcher
/// arranges. Called anywhere else, where this process already owns one of
/// those numbers (a log file, a socket), it would make a second owner of
/// it. The tripwire (exactly those descriptors open above stderr) and the
/// Bootstrap frame it needs on stdin make that unlikely, not impossible.
pub fn run_inherited() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

#[cfg(unix)]
fn run() -> Result<(), u8> {
    use crate::protocol::FIRST_KEY_DESCRIPTOR;

    crate::serve_process(std::io::stdin().lock(), std::io::stdout().lock(), |count| {
        let count = i32::try_from(count).map_err(|_| 65u8)?;
        // Exactly the slot descriptors were inherited above stdio: each
        // slot's is open, and no other is, at any number up to the
        // descriptor limit. The supervisor's close-everything-else is
        // the guarantee; this is a tripwire that it held.
        for fd in FIRST_KEY_DESCRIPTOR..FIRST_KEY_DESCRIPTOR + count {
            if !descriptor_is_open(fd) {
                return Err(66);
            }
        }
        if (FIRST_KEY_DESCRIPTOR + count..descriptor_limit()).any(descriptor_is_open) {
            return Err(66);
        }
        Ok((FIRST_KEY_DESCRIPTOR..FIRST_KEY_DESCRIPTOR + count)
            .map(adopt)
            .collect())
    })
}

#[cfg(not(unix))]
fn run() -> Result<(), u8> {
    // Inheriting key descriptors is Unix-only so far (D18S step 8 is
    // Windows); refuse to run rather than run without keys.
    Err(69)
}

/// Whether `fd` is an open descriptor in this process.
#[cfg(unix)]
fn descriptor_is_open(fd: i32) -> bool {
    // SAFETY: F_GETFD only reads the descriptor's flags; on a closed
    // descriptor it fails with EBADF and touches nothing.
    unsafe { libc::fcntl(fd, libc::F_GETFD) != -1 }
}

/// One past the highest descriptor number this process could hold, capped
/// so the scan stays cheap: 65,536 checks are well under a millisecond's
/// work each way.
#[cfg(unix)]
fn descriptor_limit() -> i32 {
    // SAFETY: sysconf reads a configuration value and touches nothing.
    let limit = unsafe { libc::sysconf(libc::_SC_OPEN_MAX) };
    if limit <= 0 {
        1024
    } else {
        limit.min(65_536) as i32
    }
}

/// Take ownership of an inherited key descriptor.
#[cfg(unix)]
fn adopt(fd: i32) -> std::fs::File {
    use std::os::fd::FromRawFd;
    // SAFETY: `fd` is open (checked by the caller), and nothing in this
    // process opened or claimed it: it was inherited across exec, and only
    // this function, called once per slot, takes it. So this is its single
    // owner, and dropping the File closes it exactly once.
    unsafe { std::fs::File::from_raw_fd(fd) }
}
