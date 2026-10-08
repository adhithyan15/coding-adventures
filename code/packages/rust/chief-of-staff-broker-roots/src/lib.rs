//! # Broker roots (D18S S-K5, step 6 P2.6c)
//!
//! The broker acts on paths an agent names. It is the confused deputy in
//! this design: it holds authority the agent does not, so every path is
//! hostile. Two rules from S-K5 make that safe, and this crate is both:
//!
//! ```text
//!   at start                                  on every request
//!   --------------------------------------    ------------------------------------
//!   BrokerRoot::open(root, never_grantable)   root.open_beneath("notes/today.md",
//!     canonicalize the root and each            Access::Read)
//!     never-grantable path                      check the name: relative, no "..",
//!     refuse if either contains the other       no NUL, at most 4 KiB
//!     open the root directory once              resolve it with the kernel's own
//!                                               beneath primitive, from the root fd
//!                                               refuse anything but a regular file
//!                                               with exactly one link
//! ```
//!
//! ## Why the kernel resolves it
//!
//! `realpath()` then `open()` is a race: between the check and the open, a
//! symlink can be swapped in, and the broker opens the vault. So the name is
//! never resolved to a string the broker then opens again. It is handed,
//! once, to a primitive that refuses to leave the root:
//!
//! | Platform | Primitive |
//! |---|---|
//! | Linux | `openat2(RESOLVE_BENEATH \| RESOLVE_NO_SYMLINKS \| RESOLVE_NO_MAGICLINKS \| RESOLVE_NO_XDEV)` |
//! | macOS | `openat` with `O_NOFOLLOW_ANY`, after refusing absolute names and `..` |
//! | others | none yet, so the broker does not offer the operation (S-P3) |
//!
//! `RESOLVE_NO_XDEV` also refuses to cross a mount point, so a bind mount of
//! the vault placed inside a root cannot be walked into. macOS has no
//! equivalent; that is documented, not hidden.
//!
//! ## What it returns
//!
//! A regular file, close-on-exec, open for exactly the access asked. Never a
//! directory: a directory descriptor would hand over its whole subtree, for
//! good, invisibly to any path check (S-K5). And never a file with a second
//! hard link, which could be the vault's own file linked into the root.

#![deny(unsafe_op_in_unsafe_fn)]

use std::fmt;
use std::fs::File;
use std::path::{Component, Path, PathBuf};

/// The longest name `open_beneath` accepts, in bytes.
pub const MAX_NAME_BYTES: usize = 4096;

/// How a brokered file is opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Access {
    /// Read only.
    Read,
    /// Write only. The file must exist: creation is not brokered here.
    Write,
}

/// Why a root or a name was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerRootError {
    /// This platform has no beneath-resolution primitive (S-P3).
    Unsupported,
    /// The root is not an existing directory.
    RootNotDirectory(PathBuf),
    /// The root and a never-grantable path overlap: one contains the other.
    Overlap {
        root: PathBuf,
        never_grantable: PathBuf,
    },
    /// The name is malformed: empty, absolute, too long, NUL, or with a
    /// `.`, `..` or empty component.
    InvalidName(&'static str),
    /// The kernel refused to resolve it beneath the root: an escape, a
    /// symlink, a magic link, a mount crossing, or simply not found.
    Refused(String),
    /// It resolved to something other than a regular file.
    NotRegularFile,
    /// It is a regular file with more than one hard link.
    MultipleLinks,
}

impl fmt::Display for BrokerRootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("no beneath-resolution primitive on this platform"),
            Self::RootNotDirectory(root) => {
                write!(f, "broker root {} is not a directory", root.display())
            }
            Self::Overlap {
                root,
                never_grantable,
            } => write!(
                f,
                "broker root {} overlaps never-grantable {}",
                root.display(),
                never_grantable.display()
            ),
            Self::InvalidName(why) => write!(f, "invalid name: {why}"),
            Self::Refused(why) => write!(f, "refused beneath the root: {why}"),
            Self::NotRegularFile => f.write_str("not a regular file"),
            Self::MultipleLinks => f.write_str("a file with more than one hard link"),
        }
    }
}

impl std::error::Error for BrokerRootError {}

/// A supervisor-chosen directory the broker resolves agent names beneath,
/// proven disjoint from the never-grantable set when it was opened.
#[derive(Debug)]
pub struct BrokerRoot {
    /// The root, canonical.
    path: PathBuf,
    /// The root directory, opened once; every name resolves from it.
    #[cfg_attr(not(unix), allow(dead_code))]
    directory: File,
}

impl BrokerRoot {
    /// Open `root`, refusing it unless it is disjoint from every path in
    /// `never_grantable` (S-I6's set: the vault and its directory, the
    /// audit log and its directory, the runtime image, the plan files...).
    ///
    /// Disjoint means neither contains the other, compared after both are
    /// canonicalized, so a symlink cannot disguise either. A never-grantable
    /// path that does not exist yet is compared through its nearest
    /// existing ancestor.
    pub fn open(root: &Path, never_grantable: &[&Path]) -> Result<Self, BrokerRootError> {
        if !cfg!(any(target_os = "linux", target_os = "macos")) {
            return Err(BrokerRootError::Unsupported);
        }
        let not_directory = || BrokerRootError::RootNotDirectory(root.to_path_buf());
        let canonical = std::fs::canonicalize(root).map_err(|_| not_directory())?;
        for forbidden in never_grantable {
            let forbidden = canonical_or_nearest(forbidden);
            if canonical.starts_with(&forbidden) || forbidden.starts_with(&canonical) {
                return Err(BrokerRootError::Overlap {
                    root: canonical,
                    never_grantable: forbidden,
                });
            }
        }
        let directory = platform::open_root(&canonical).ok_or_else(not_directory)?;
        Ok(Self {
            path: canonical,
            directory,
        })
    }

    /// The root, canonical.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Open the agent-named file `name` strictly beneath this root.
    pub fn open_beneath(&self, name: &str, access: Access) -> Result<File, BrokerRootError> {
        validate_name(name)?;
        let file = platform::open_beneath(self, name, access)?;
        let metadata = file
            .metadata()
            .map_err(|error| BrokerRootError::Refused(error.to_string()))?;
        if !metadata.is_file() {
            return Err(BrokerRootError::NotRegularFile);
        }
        if link_count(&metadata) != 1 {
            return Err(BrokerRootError::MultipleLinks);
        }
        platform::clear_nonblocking(&file)?;
        Ok(file)
    }
}

/// `path` canonicalized; if it does not exist, its nearest existing
/// ancestor canonicalized, with the missing tail appended.
fn canonical_or_nearest(path: &Path) -> PathBuf {
    let mut tail = Vec::new();
    let mut current = path.to_path_buf();
    loop {
        if let Ok(canonical) = std::fs::canonicalize(&current) {
            return tail
                .iter()
                .rev()
                .fold(canonical, |path, part| path.join(part));
        }
        match (
            current.file_name().map(|name| name.to_owned()),
            current.parent(),
        ) {
            (Some(name), Some(parent)) => {
                tail.push(name);
                current = parent.to_path_buf();
            }
            // Nothing of it exists: compare it as given.
            _ => return path.to_path_buf(),
        }
    }
}

/// A name must be a plain relative path: no root, no `.` or `..`, no empty
/// component, no NUL, at most [`MAX_NAME_BYTES`].
fn validate_name(name: &str) -> Result<(), BrokerRootError> {
    if name.is_empty() {
        return Err(BrokerRootError::InvalidName("empty"));
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(BrokerRootError::InvalidName("too long"));
    }
    if name.contains('\0') {
        return Err(BrokerRootError::InvalidName("NUL"));
    }
    if name.starts_with('/') {
        return Err(BrokerRootError::InvalidName("absolute"));
    }
    for part in name.split('/') {
        if part.is_empty() || part == "." || part == ".." {
            return Err(BrokerRootError::InvalidName("an empty, . or .. component"));
        }
    }
    // Belt and braces: what `Path` itself sees must agree.
    if !Path::new(name)
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(BrokerRootError::InvalidName("not a plain relative path"));
    }
    Ok(())
}

#[cfg(unix)]
fn link_count(metadata: &std::fs::Metadata) -> u64 {
    std::os::unix::fs::MetadataExt::nlink(metadata)
}

#[cfg(not(unix))]
fn link_count(_metadata: &std::fs::Metadata) -> u64 {
    1
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;

    fn refused() -> BrokerRootError {
        BrokerRootError::Refused(std::io::Error::last_os_error().to_string())
    }

    pub(super) fn open_root(canonical: &Path) -> Option<File> {
        let path = CString::new(canonical.as_os_str().as_bytes()).ok()?;
        // SAFETY: a NUL-terminated path; a new descriptor or -1.
        let fd = unsafe {
            libc::open(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return None;
        }
        // SAFETY: a fresh descriptor the kernel just returned.
        Some(unsafe { File::from_raw_fd(fd) })
    }

    /// The open flags for `access`: close-on-exec, no controlling
    /// terminal, and non-blocking until the file is known to be regular, so
    /// a FIFO planted in the root cannot hang the broker in `open`.
    fn flags(access: Access) -> libc::c_int {
        let mode = match access {
            Access::Read => libc::O_RDONLY,
            Access::Write => libc::O_WRONLY,
        };
        mode | libc::O_CLOEXEC | libc::O_NOCTTY | libc::O_NONBLOCK
    }

    #[cfg(target_os = "linux")]
    pub(super) fn open_beneath(
        root: &BrokerRoot,
        name: &str,
        access: Access,
    ) -> Result<File, BrokerRootError> {
        #[repr(C)]
        struct OpenHow {
            flags: u64,
            mode: u64,
            resolve: u64,
        }
        const RESOLVE_NO_XDEV: u64 = 0x01;
        const RESOLVE_NO_MAGICLINKS: u64 = 0x02;
        const RESOLVE_NO_SYMLINKS: u64 = 0x04;
        const RESOLVE_BENEATH: u64 = 0x08;
        let name = CString::new(name).map_err(|_| BrokerRootError::InvalidName("NUL"))?;
        let how = OpenHow {
            flags: flags(access) as u64,
            mode: 0,
            resolve: RESOLVE_BENEATH
                | RESOLVE_NO_SYMLINKS
                | RESOLVE_NO_MAGICLINKS
                | RESOLVE_NO_XDEV,
        };
        // SAFETY: the root's descriptor, a NUL-terminated name, and an
        // open_how of the size passed; a new descriptor or -1.
        let fd = unsafe {
            libc::syscall(
                libc::SYS_openat2,
                root.directory.as_raw_fd(),
                name.as_ptr(),
                &how as *const OpenHow,
                std::mem::size_of::<OpenHow>(),
            )
        };
        if fd < 0 {
            return Err(refused());
        }
        // SAFETY: a fresh descriptor the kernel just returned.
        Ok(unsafe { File::from_raw_fd(fd as libc::c_int) })
    }

    #[cfg(target_os = "macos")]
    pub(super) fn open_beneath(
        root: &BrokerRoot,
        name: &str,
        access: Access,
    ) -> Result<File, BrokerRootError> {
        // `validate_name` already refused absolute names and `..`, so with
        // no symlink anywhere (O_NOFOLLOW_ANY) the walk stays beneath.
        let name = CString::new(name).map_err(|_| BrokerRootError::InvalidName("NUL"))?;
        // SAFETY: the root's descriptor and a NUL-terminated name.
        let fd = unsafe {
            libc::openat(
                root.directory.as_raw_fd(),
                name.as_ptr(),
                flags(access) | libc::O_NOFOLLOW_ANY,
            )
        };
        if fd < 0 {
            return Err(refused());
        }
        // SAFETY: a fresh descriptor the kernel just returned.
        Ok(unsafe { File::from_raw_fd(fd) })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(super) fn open_beneath(
        _root: &BrokerRoot,
        _name: &str,
        _access: Access,
    ) -> Result<File, BrokerRootError> {
        Err(BrokerRootError::Unsupported)
    }

    /// Back to blocking I/O, now the file is known to be regular.
    pub(super) fn clear_nonblocking(file: &File) -> Result<(), BrokerRootError> {
        let fd = file.as_raw_fd();
        // SAFETY: F_GETFL and F_SETFL on a descriptor this function borrows.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags & !libc::O_NONBLOCK) } < 0 {
            return Err(refused());
        }
        Ok(())
    }
}

#[cfg(not(unix))]
mod platform {
    use super::*;

    pub(super) fn open_root(_canonical: &Path) -> Option<File> {
        None
    }

    pub(super) fn open_beneath(
        _root: &BrokerRoot,
        _name: &str,
        _access: Access,
    ) -> Result<File, BrokerRootError> {
        Err(BrokerRootError::Unsupported)
    }

    pub(super) fn clear_nonblocking(_file: &File) -> Result<(), BrokerRootError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_must_be_plain_relative_paths() {
        for good in ["a", "notes/today.md", "a/b/c.txt", "..hidden", "x..y"] {
            assert!(validate_name(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "/etc/passwd",
            "../vault",
            "a/../../vault",
            "a/./b",
            ".",
            "a//b",
            "a/",
            "a\0b",
        ] {
            assert!(validate_name(bad).is_err(), "{bad:?}");
        }
        assert!(validate_name(&"a".repeat(MAX_NAME_BYTES)).is_ok());
        assert!(validate_name(&"a".repeat(MAX_NAME_BYTES + 1)).is_err());
    }
}
