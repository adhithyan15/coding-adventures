//! The broker, built inside this test crate so its tests can launch it next
//! to the test host. Its body is the production broker's: the library's
//! `serve_process`, with the same descriptor adoption as
//! `chief-of-staff-agent-broker`'s binary.

#![deny(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

#[cfg(unix)]
fn run() -> Result<(), u8> {
    use chief_of_staff_agent_broker::protocol::FIRST_KEY_DESCRIPTOR;
    chief_of_staff_agent_broker::serve_process(
        std::io::stdin().lock(),
        std::io::stdout().lock(),
        |count| {
            let count = i32::try_from(count).map_err(|_| 65u8)?;
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
        },
    )
}

#[cfg(not(unix))]
fn run() -> Result<(), u8> {
    Err(69)
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn descriptor_is_open(fd: i32) -> bool {
    // SAFETY: F_GETFD only reads a descriptor's flags.
    unsafe { libc::fcntl(fd, libc::F_GETFD) != -1 }
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn descriptor_limit() -> i32 {
    // SAFETY: sysconf reads a configuration value.
    let limit = unsafe { libc::sysconf(libc::_SC_OPEN_MAX) };
    if limit <= 0 {
        1024
    } else {
        limit.min(65_536) as i32
    }
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn adopt(fd: i32) -> std::fs::File {
    use std::os::fd::FromRawFd;
    // SAFETY: an inherited descriptor, checked open, that nothing in this
    // process has claimed; adopted once.
    unsafe { std::fs::File::from_raw_fd(fd) }
}
