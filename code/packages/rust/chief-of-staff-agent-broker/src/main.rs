//! The broker process (D18S S-K7, P2.6d-1).
//!
//! ```text
//!   start ──► suppress core dumps (P2.6a), before any key exists
//!         ──► read Bootstrap from stdin: the binding, and the key slot table
//!         ──► check descriptors 3..3+n are open, and 3+n is not
//!         ──► read each key from its descriptor, re-checking the owner-only
//!             policy on the descriptor itself; close the descriptor
//!         ──► Ready { public halves } on stdout
//!         ──► serve requests, one at a time, until Terminate (exit 0)
//! ```
//!
//! Any failure exits non-zero with no output: the supervisor ends the agent
//! when its broker ends. Nothing is written to stderr. The supervisor points
//! it at /dev/null, and an error message is no place for a key.

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
    use chief_of_staff_agent_broker::protocol::{
        FromBroker, ToBroker, FIRST_KEY_DESCRIPTOR, KEY_BYTES,
    };
    use chief_of_staff_agent_broker::{ChannelBroker, FramedCallbacks, KeyError, LoadedKeys};

    chief_of_staff_process_hardening::suppress_core_dumps().map_err(|_| 70u8)?;

    let mut frames = FramedCallbacks::new(std::io::stdin().lock(), std::io::stdout().lock());
    let ToBroker::Bootstrap { binding, slots } = frames.receive().map_err(|_| 65u8)? else {
        return Err(65);
    };

    let count = i32::try_from(slots.len()).map_err(|_| 65u8)?;
    // Exactly the slot descriptors were inherited above stdio: each slot's
    // is open, and no other is, at any number up to the descriptor limit.
    // The supervisor's close-everything-else is the guarantee; this is a
    // tripwire that it held.
    for fd in FIRST_KEY_DESCRIPTOR..FIRST_KEY_DESCRIPTOR + count {
        if !descriptor_is_open(fd) {
            return Err(66);
        }
    }
    if (FIRST_KEY_DESCRIPTOR + count..descriptor_limit()).any(descriptor_is_open) {
        return Err(66);
    }
    let files: Vec<std::fs::File> = (FIRST_KEY_DESCRIPTOR..FIRST_KEY_DESCRIPTOR + count)
        .map(adopt)
        .collect();
    let keys = LoadedKeys::load(&binding, &slots, |index| {
        chief_of_staff_daemon_secret_file::read_owner_only_secret_from(&files[index], KEY_BYTES)
            .map_err(|_| KeyError::Unreadable)
    })
    .map_err(|_| 67u8)?;
    // The keys are in this process now; the descriptors are not needed.
    drop(files);

    frames
        .send(&FromBroker::Ready {
            public_keys: keys.public_keys().to_vec(),
        })
        .map_err(|_| 74u8)?;

    let mut broker = ChannelBroker::new(binding, keys);
    loop {
        match frames.receive().map_err(|_| 65u8)? {
            ToBroker::Request(request) => {
                let response = broker.serve(&request, &mut frames).map_err(|_| 65u8)?;
                frames
                    .send(&FromBroker::Response(response))
                    .map_err(|_| 74u8)?;
            }
            ToBroker::Terminate => return Ok(()),
            // A second Bootstrap, or a callback result with no callback
            // outstanding: the peer is not following the protocol.
            _ => return Err(65),
        }
    }
}

#[cfg(not(unix))]
fn run() -> Result<(), u8> {
    // Inheriting key descriptors is Unix-only so far (D18S step 8 is
    // Windows); refuse to run rather than run without keys.
    Err(69)
}

/// Whether `fd` is an open descriptor in this process.
#[cfg(unix)]
#[allow(unsafe_code)]
fn descriptor_is_open(fd: i32) -> bool {
    // SAFETY: F_GETFD only reads the descriptor's flags; on a closed
    // descriptor it fails with EBADF and touches nothing.
    unsafe { libc::fcntl(fd, libc::F_GETFD) != -1 }
}

/// One past the highest descriptor number this process could hold, capped
/// so the scan stays cheap: 65,536 checks are well under a millisecond's
/// work each way.
#[cfg(unix)]
#[allow(unsafe_code)]
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
#[allow(unsafe_code)]
fn adopt(fd: i32) -> std::fs::File {
    use std::os::fd::FromRawFd;
    // SAFETY: `fd` is open (checked by the caller), and nothing in this
    // process opened or claimed it: it was inherited across exec, and only
    // this function, called once per slot, takes it. So this is its single
    // owner, and dropping the File closes it exactly once.
    unsafe { std::fs::File::from_raw_fd(fd) }
}
