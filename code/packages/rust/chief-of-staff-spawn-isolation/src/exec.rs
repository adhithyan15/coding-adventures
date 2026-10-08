//! # Launching a verified executable with exactly n descriptors (D18S S-K1,
//! S-I3; P2.6d-2a)
//!
//! A broker is launched by the supervisor, holding its agent's keys on
//! descriptors 3..3+n and nothing else. Two rules from the spec govern how:
//!
//! - **S-K1, verify and execute the same object.** Checking a path's hash
//!   and then `exec`ing the path is a race: the file can be replaced in
//!   between. So the binary is opened once, hashed *through that
//!   descriptor*, and that descriptor is what is executed
//!   (`execveat(fd, "", AT_EMPTY_PATH)`).
//! - **S-I3, nothing inherited but what is meant.** Everything above stderr
//!   is closed at exec, except the n key descriptors, placed exactly at
//!   3..3+n.
//!
//! ```text
//!   parent                                child, between fork and exec
//!   ------                                ----------------------------
//!   re-hash the binary through its fd     refuse a terminal on 0-2; setsid
//!   relocate each key fd, and the         mark every fd from 3 close-on-exec
//!     binary's, above 3+n, close-on-exec  dup2(source[i], 3+i)   ◄─ clears
//!   build argv and envp (no allocation        close-on-exec on 3+i only
//!     may happen in the child)            execveat(binary, "", AT_EMPTY_PATH)
//! ```
//!
//! Relocating every source above 3+n first means no `dup2` can overwrite a
//! source still waiting to be moved, so the order of the moves does not
//! matter.
//!
//! ## The one rule that is not obvious
//!
//! `std::process::Command` keeps a close-on-exec pipe open in the child, to
//! report a failed `exec` to the parent, at whatever low descriptor was
//! free, quite possibly inside 3..3+n. The first `dup2` may therefore close
//! it, and the hook cannot know. So before the first `dup2` the hook fails
//! normally (the spawn returns the error). After it, the hook never
//! returns: a failure exits the child with status 127, and to the parent
//! the spawn looks successful. The caller already treats a broker that
//! exits before its first frame as a failed launch.

use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use coding_adventures_sha256::Sha256Hasher;

/// The largest binary [`VerifiedExecutable`] will hash.
pub const MAX_EXECUTABLE_BYTES: u64 = 128 * 1024 * 1024;

/// Why an executable was not accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyError {
    /// The path is not absolute (S-K1: no search, no working directory).
    NotAbsolute,
    /// It could not be opened, inspected or read.
    Unreadable,
    /// Not a regular file.
    NotRegular,
    /// Group- or world-writable: someone else could change it in place.
    Writable,
    /// Owned by neither root nor this process's user.
    Owner,
    /// Larger than [`MAX_EXECUTABLE_BYTES`].
    TooLarge,
    /// Its size changed while it was being hashed.
    Changed,
    /// Its SHA-256 is not the pinned one.
    DigestMismatch,
}

impl std::fmt::Display for VerifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NotAbsolute => "executable path is not absolute",
            Self::Unreadable => "executable could not be read",
            Self::NotRegular => "executable is not a regular file",
            Self::Writable => "executable is writable by others",
            Self::Owner => "executable is owned by someone else",
            Self::TooLarge => "executable is too large to verify",
            Self::Changed => "executable changed while it was verified",
            Self::DigestMismatch => "executable does not match its pinned digest",
        })
    }
}

impl std::error::Error for VerifyError {}

/// An executable, open, whose contents matched a pinned SHA-256 when it was
/// opened and match it again before every launch.
///
/// The only way to get one is [`VerifiedExecutable::open`], and the only way
/// to run one is [`isolate_and_exec`], which re-verifies and then executes
/// the very descriptor it hashed.
#[derive(Debug)]
pub struct VerifiedExecutable {
    file: File,
    path: PathBuf,
    digest: [u8; 32],
}

impl VerifiedExecutable {
    /// Open `path` and check it against `expected` SHA-256.
    pub fn open(path: &Path, expected: [u8; 32]) -> Result<Self, VerifyError> {
        if !path.is_absolute() {
            return Err(VerifyError::NotAbsolute);
        }
        // O_RDONLY: never O_PATH, which execveat would accept but which
        // cannot be read to hash (S-I4d).
        let file = File::open(path).map_err(|_| VerifyError::Unreadable)?;
        let verified = Self {
            file,
            path: path.to_path_buf(),
            digest: expected,
        };
        verified.verify()?;
        Ok(verified)
    }

    /// The path it was opened from, for messages only: it is never
    /// executed by path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Hash it again through its descriptor.
    pub fn verify(&self) -> Result<(), VerifyError> {
        let before = self.file.metadata().map_err(|_| VerifyError::Unreadable)?;
        if !before.file_type().is_file() {
            return Err(VerifyError::NotRegular);
        }
        if before.mode() & 0o022 != 0 {
            return Err(VerifyError::Writable);
        }
        // SAFETY: geteuid has no preconditions and touches no memory.
        let me = unsafe { libc::geteuid() };
        if before.uid() != 0 && before.uid() != me {
            return Err(VerifyError::Owner);
        }
        if before.len() > MAX_EXECUTABLE_BYTES {
            return Err(VerifyError::TooLarge);
        }
        let mut hasher = Sha256Hasher::new();
        let mut buffer = vec![0u8; 64 * 1024];
        let mut offset = 0u64;
        loop {
            let read = match self.file.read_at(&mut buffer, offset) {
                Ok(read) => read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => return Err(VerifyError::Unreadable),
            };
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            offset += read as u64;
            if offset > MAX_EXECUTABLE_BYTES {
                return Err(VerifyError::TooLarge);
            }
        }
        let after = self.file.metadata().map_err(|_| VerifyError::Unreadable)?;
        if offset != before.len() || after.len() != before.len() {
            return Err(VerifyError::Changed);
        }
        if hasher.digest() != self.digest {
            return Err(VerifyError::DigestMismatch);
        }
        Ok(())
    }
}

/// argv or envp for `execveat`: the strings, and the NULL-terminated
/// pointer array into them, both built in the parent.
struct CArray {
    _strings: Vec<CString>,
    pointers: Vec<*const libc::c_char>,
}

impl CArray {
    /// An entry with an interior NUL is refused, not cut: what would run is
    /// not what was asked for.
    fn new(entries: impl Iterator<Item = Vec<u8>>) -> io::Result<Self> {
        let strings = entries
            .map(|entry| {
                CString::new(entry).map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))
            })
            .collect::<io::Result<Vec<_>>>()?;
        let mut pointers: Vec<*const libc::c_char> =
            strings.iter().map(|entry| entry.as_ptr()).collect();
        pointers.push(std::ptr::null());
        Ok(Self {
            _strings: strings,
            pointers,
        })
    }
}

// SAFETY: the pointers point into `_strings`, which this value owns and
// never changes; the forked child only reads them.
unsafe impl Send for CArray {}
// SAFETY: as above.
unsafe impl Sync for CArray {}

/// `fd` duplicated at or above `floor`, close-on-exec.
fn relocate(fd: RawFd, floor: RawFd) -> io::Result<OwnedFd> {
    // SAFETY: F_DUPFD_CLOEXEC returns a new descriptor, or -1.
    let moved = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, floor) };
    if moved < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: a fresh descriptor nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(moved) })
}

/// Arrange for `command` to run `program`, isolated as [`crate::isolate`]
/// does, holding exactly `inherited` at descriptors 3, 4, ... in order.
///
/// - `program` is re-verified now, and its own descriptor is what is
///   executed; the command's program name becomes `argv[0]` only.
/// - The child's environment is exactly the command's explicitly set
///   variables, nothing inherited. Call `env_clear()` and set what it needs.
/// - Do not also call [`crate::isolate`], and set no process group.
/// - A failure after the descriptors are placed exits the child with status
///   127 instead of failing the spawn (see the module documentation).
pub fn isolate_and_exec<'c>(
    command: &'c mut Command,
    program: &VerifiedExecutable,
    inherited: Vec<OwnedFd>,
) -> io::Result<&'c mut Command> {
    program
        .verify()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let count = RawFd::try_from(inherited.len())
        .ok()
        .filter(|count| *count <= 1024)
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let floor = 3 + count;
    let sources: Vec<OwnedFd> = inherited
        .iter()
        .map(|fd| relocate(fd.as_raw_fd(), floor))
        .collect::<io::Result<_>>()?;
    // The caller's own copies close now; only the relocated, close-on-exec
    // ones remain in this process, so concurrent spawns cannot inherit them.
    drop(inherited);
    let binary = relocate(program.file.as_raw_fd(), floor)?;

    let argv = CArray::new(
        std::iter::once(command.get_program().as_bytes().to_vec()).chain(
            command
                .get_args()
                .map(|argument| argument.as_bytes().to_vec()),
        ),
    )?;
    let envp = CArray::new(command.get_envs().filter_map(|(key, value)| {
        value.map(|value| {
            let mut entry = key.as_bytes().to_vec();
            entry.push(b'=');
            entry.extend_from_slice(value.as_bytes());
            entry
        })
    }))?;
    let limit = crate::unix::descriptor_limit();

    command.stderr(Stdio::null());
    // SAFETY: the closure runs in the forked child, before exec. It calls
    // only async-signal-safe functions (those of `isolate_in_child`, then
    // `dup2`, the raw `execveat` syscall, and `_exit`), allocates nothing,
    // and reads only what was built above in the parent.
    unsafe {
        command.pre_exec(move || {
            // Capture the arrays whole: their wrapper is what is Send.
            let (argv, envp) = (&argv, &envp);
            crate::unix::isolate_in_child(limit)?;
            for (index, source) in sources.iter().enumerate() {
                if libc::dup2(source.as_raw_fd(), 3 + index as RawFd) < 0 {
                    // std's exec-error pipe may already be gone: never
                    // return from here.
                    libc::_exit(127);
                }
            }
            libc::syscall(
                libc::SYS_execveat,
                binary.as_raw_fd(),
                c"".as_ptr(),
                argv.pointers.as_ptr(),
                envp.pointers.as_ptr(),
                libc::AT_EMPTY_PATH,
            );
            libc::_exit(127);
        });
    }
    Ok(command)
}
