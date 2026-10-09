//! # A verified executable (D18S S-K1; P2.6d-2a, P2.6d-3)
//!
//! A broker is launched by the supervisor from a binary pinned by its
//! SHA-256. Checking a path's hash and then `exec`ing the path is a race:
//! the file can be replaced in between. So the binary is opened once,
//! hashed *through that descriptor*, and that descriptor is what is
//! executed.
//!
//! [`VerifiedExecutable`] is the open, hashed file. It does not launch
//! anything itself: `chief-of-staff-linux-sandbox`'s
//! `LinuxConfinement::prepare_verified` takes it, re-verifies it, and
//! execs a duplicate of its descriptor under the broker's confinement
//! (`execveat(fd, "", AT_EMPTY_PATH)`), with the keys on 3..3+n.
//!
//! (P2.6d-2a had a second launcher here, `isolate_and_exec`, which placed
//! the descriptors and exec'd without a sandbox. P2.6d-3 replaced it: there
//! is one way to start a broker, and it is confined.)

use std::fs::File;
use std::io;
use std::os::unix::fs::{FileExt, MetadataExt};
use std::path::{Path, PathBuf};

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
/// The only way to get one is [`VerifiedExecutable::open`]. To run one, the
/// confinement re-verifies it and then executes a duplicate of the very
/// descriptor it hashed ([`VerifiedExecutable::descriptor`]).
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

    /// A duplicate of the descriptor that was hashed: same open file, so
    /// the same bytes. Execute this, never [`Self::path`].
    pub fn descriptor(&self) -> io::Result<File> {
        self.file.try_clone()
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
