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

/// On Windows (D18S P2.8b-1) the keys arrive as handles on the applier's
/// handle list, named in [`KEY_HANDLES_VARIABLE`]: exactly one per slot, in
/// slot order. A count that differs from the bootstrap's is refused (66)
/// before any key is read, as a stray descriptor is on Unix. Each handle is
/// then read through `read_owner_only_secret_from`, which checks the
/// owner-only policy on it again.
///
/// [`KEY_HANDLES_VARIABLE`]: crate::protocol::KEY_HANDLES_VARIABLE
#[cfg(windows)]
fn run() -> Result<(), u8> {
    use crate::protocol::KEY_HANDLES_VARIABLE;

    crate::serve_process(std::io::stdin().lock(), std::io::stdout().lock(), |count| {
        let named = key_handles(std::env::var_os(KEY_HANDLES_VARIABLE).as_deref(), count)?;
        named.into_iter().map(adopt_handle).collect()
    })
}

#[cfg(not(any(unix, windows)))]
fn run() -> Result<(), u8> {
    // No way to inherit keys is defined here; refuse to run rather than
    // run without them.
    Err(69)
}

/// The key handle values named in `value`: exactly `count` of them, each
/// a non-zero decimal number, none repeated, in slot order. No variable
/// at all is the same as an empty one, and right only for no slots.
///
/// A handle value is a multiple of 4 below 2^32: Windows ignores a
/// value's low two bits, so 312 and 313 name one handle (two owners, one
/// key in two slots), and a value at 2^63 or above is a negative
/// pseudo-handle (the process, the thread, the token). Both are refused.
///
/// ```text
///   count  value          result
///   2      "312,316"      [312, 316]
///   0      (absent) / ""  []
///   2      "312"          refused: one short
///   2      "312,312"      refused: repeated
///   1      "+312" / "0"   refused: not a plain non-zero number
/// ```
#[cfg(any(windows, test))]
fn key_handles(value: Option<&std::ffi::OsStr>, count: usize) -> Result<Vec<usize>, u8> {
    let text = match value {
        None => "",
        Some(value) => value.to_str().ok_or(66u8)?,
    };
    let values = if text.is_empty() {
        Vec::new()
    } else {
        text.split(',')
            .map(|value| {
                if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
                    return Err(66u8);
                }
                match value.parse::<u32>() {
                    Ok(handle) if handle != 0 && handle % 4 == 0 => Ok(handle as usize),
                    _ => Err(66u8),
                }
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut sorted = values.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if values.len() != count || sorted.len() != values.len() {
        return Err(66);
    }
    Ok(values)
}

/// Take ownership of an inherited key handle. A value that names no handle
/// in this process is refused (66); under the applier's strict handle
/// checks it would instead end the process, which refuses it as well.
#[cfg(windows)]
fn adopt_handle(value: usize) -> Result<std::fs::File, u8> {
    use std::os::windows::io::{FromRawHandle, RawHandle};
    use windows_sys::Win32::Foundation::GetHandleInformation;
    let mut flags = 0u32;
    // SAFETY: GetHandleInformation only reads the handle table entry; on a
    // value that names no handle it fails and touches nothing.
    if unsafe { GetHandleInformation(value as _, &mut flags) } == 0 {
        return Err(66);
    }
    // SAFETY: the applier passed this handle on its list and named it
    // here; nothing in this process opened or claimed it, and only this
    // function, once per slot (the values are distinct), takes it. So this
    // is its single owner, and dropping the File closes it exactly once.
    Ok(unsafe { std::fs::File::from_raw_handle(value as RawHandle) })
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

#[cfg(test)]
mod tests {
    use super::key_handles;
    use std::ffi::OsStr;

    #[test]
    fn exactly_the_named_handles_are_taken_in_order() {
        assert_eq!(
            key_handles(Some(OsStr::new("312,316")), 2),
            Ok(vec![312, 316])
        );
        assert_eq!(
            key_handles(Some(OsStr::new("316,312")), 2),
            Ok(vec![316, 312])
        );
        assert_eq!(key_handles(None, 0), Ok(vec![]));
        assert_eq!(key_handles(Some(OsStr::new("")), 0), Ok(vec![]));
    }

    #[test]
    fn a_count_or_a_value_that_is_not_exact_is_refused() {
        for (value, count) in [
            (Some("312"), 2),
            (Some("312,316"), 1),
            (None, 1),
            (Some("312,312"), 2),
            (Some("312,,316"), 3),
            (Some("312,"), 1),
            (Some("+312"), 1),
            (Some("0"), 1),
            (Some(" 312"), 1),
            (Some("0x138"), 1),
            (Some("99999999999999999999999"), 1),
            (Some("312,313"), 2),
            (Some("313"), 1),
            (Some("4294967296"), 1),
            (Some("18446744073709551612"), 1),
        ] {
            assert_eq!(
                key_handles(value.map(OsStr::new), count),
                Err(66),
                "{value:?} {count}"
            );
        }
    }
}
