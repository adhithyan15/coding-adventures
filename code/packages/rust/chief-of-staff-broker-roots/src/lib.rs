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
    ///
    /// The file is opened at offset 0, neither truncated nor in append
    /// mode. Truncating in `open` would happen before the checks, to a file
    /// they might then refuse. A caller replacing the contents calls
    /// `set_len` on the returned file, after the checks have passed.
    Write,
}

/// Why a root or a name was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrokerRootError {
    /// This platform has no beneath-resolution primitive (S-P3).
    Unsupported,
    /// The root is not an existing directory.
    RootNotDirectory(PathBuf),
    /// The directory opened is not the one checked: the root's path changed
    /// between the disjointness proof and the open.
    RootMoved(PathBuf),
    /// The kernel could not say which directory the root's descriptor is
    /// (no `/proc` on Linux, `F_GETPATH` failed on macOS), so the proof
    /// cannot be tied to it.
    RootUnverifiable(PathBuf),
    /// The root and a never-grantable path overlap: one contains the other,
    /// or the never-grantable path cannot be compared exactly (see
    /// `canonical_or_nearest`).
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
            Self::RootUnverifiable(root) => write!(
                f,
                "cannot confirm which directory broker root {} opened",
                root.display()
            ),
            Self::RootMoved(root) => write!(
                f,
                "broker root {} changed between its check and its open",
                root.display()
            ),
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
        if !cfg!(any(target_os = "linux", target_os = "macos"))
            || !platform::beneath_primitive_works()
        {
            return Err(BrokerRootError::Unsupported);
        }
        let not_directory = || BrokerRootError::RootNotDirectory(root.to_path_buf());
        let canonical = std::fs::canonicalize(root).map_err(|_| not_directory())?;
        for &given in never_grantable {
            // A path that cannot be compared exactly is treated as an
            // overlap: the proof fails closed.
            let Some(forbidden) = canonical_or_nearest(given) else {
                return Err(BrokerRootError::Overlap {
                    root: canonical,
                    never_grantable: given.to_path_buf(),
                });
            };
            if canonical.starts_with(&forbidden) || forbidden.starts_with(&canonical) {
                return Err(BrokerRootError::Overlap {
                    root: canonical,
                    never_grantable: forbidden,
                });
            }
        }
        let directory = platform::open_root(&canonical).ok_or_else(not_directory)?;
        // The proof above was about a path, and the open was by path too: a
        // parent directory swapped for a symlink in between would leave the
        // descriptor naming a directory nobody checked. So ask the kernel
        // what the descriptor actually is, and refuse unless it is the
        // directory the proof was about.
        match platform::descriptor_path(&directory) {
            Some(opened) if opened == canonical => {}
            Some(_) => return Err(BrokerRootError::RootMoved(canonical)),
            None => return Err(BrokerRootError::RootUnverifiable(canonical)),
        }
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

/// `path` canonicalized. If it does not exist yet, its nearest existing
/// ancestor is canonicalized and the missing tail appended:
///
/// ```text
///   /home/me/.chief/vault/sealed.bin    (vault/ not created yet)
///   └──────┬──────┘└──────┬───────┘
///    exists: canonical   missing: appended as plain names
/// ```
///
/// Only a component that does not exist (`ENOENT`) counts as missing. The
/// tail must be plain names. A `..` in it cannot be resolved without the
/// directories it climbs out of (`missing/../x` might be anywhere once
/// `missing` is a symlink), so such a path is `None`, and the caller treats
/// it as an overlap. A relative path is made absolute first, against the
/// current directory, so it is never compared as a bare relative string.
fn canonical_or_nearest(path: &Path) -> Option<PathBuf> {
    let absolute = std::path::absolute(path).ok()?;
    let mut tail = Vec::new();
    let mut current = absolute.as_path();
    loop {
        match std::fs::canonicalize(current) {
            Ok(canonical) => {
                return Some(
                    tail.iter()
                        .rev()
                        .fold(canonical, |path, part| path.join(part)),
                )
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            // It exists but cannot be resolved (no search permission, a
            // symlink loop, a file used as a directory): where it leads is
            // unknown, so the proof fails closed.
            Err(_) => return None,
        }
        match current.components().next_back() {
            Some(Component::Normal(name)) => {
                tail.push(name);
                current = current.parent()?;
            }
            // `..`, `.`, or a root that will not canonicalize.
            _ => return None,
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

    /// What the kernel says `file` is, by path.
    #[cfg(target_os = "linux")]
    pub(super) fn descriptor_path(file: &File) -> Option<PathBuf> {
        std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd())).ok()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn descriptor_path(file: &File) -> Option<PathBuf> {
        use std::os::unix::ffi::OsStringExt;
        let mut buffer = vec![0u8; libc::PATH_MAX as usize];
        // SAFETY: F_GETPATH writes at most MAXPATHLEN (= PATH_MAX) bytes,
        // NUL included, into a buffer of that size.
        if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETPATH, buffer.as_mut_ptr()) } < 0 {
            return None;
        }
        buffer.truncate(buffer.iter().position(|&byte| byte == 0)?);
        Some(PathBuf::from(std::ffi::OsString::from_vec(buffer)))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(super) fn descriptor_path(_file: &File) -> Option<PathBuf> {
        None
    }

    /// Linux: `openat2` is either there or fails every open (`ENOSYS`
    /// before 5.6), which already fails closed.
    #[cfg(not(target_os = "macos"))]
    pub(super) fn beneath_primitive_works() -> bool {
        true
    }

    /// macOS: `O_NOFOLLOW_ANY` arrived in macOS 11, and an older kernel
    /// ignores an unknown open flag, which would fail open. So prove it is
    /// honored: `/etc` is a symlink to `private/etc` on every macOS, so
    /// with the flag honored, opening `/etc/hosts` fails with `ELOOP`.
    #[cfg(target_os = "macos")]
    pub(super) fn beneath_primitive_works() -> bool {
        // SAFETY: a NUL-terminated literal path; a new descriptor or -1.
        let fd = unsafe {
            libc::open(
                c"/etc/hosts".as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW_ANY,
            )
        };
        if fd >= 0 {
            // SAFETY: the descriptor just opened, closed once.
            unsafe { libc::close(fd) };
            return false;
        }
        std::io::Error::last_os_error().raw_os_error() == Some(libc::ELOOP)
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

    pub(super) fn descriptor_path(_file: &File) -> Option<PathBuf> {
        None
    }

    pub(super) fn beneath_primitive_works() -> bool {
        false
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

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn the_descriptor_path_reveals_a_root_opened_through_a_swapped_parent() {
        // What `BrokerRoot::open` guards against: the root's parent became
        // a symlink after the proof. The open still succeeds (O_NOFOLLOW
        // covers only the last component), but the descriptor's own path
        // is where it really went, and no longer matches.
        let base = std::env::temp_dir().join(format!("broker-roots-unit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("real/root")).unwrap();
        std::os::unix::fs::symlink(base.join("real"), base.join("swapped")).unwrap();
        let real = std::fs::canonicalize(base.join("real/root")).unwrap();

        let through_symlink = base.join("swapped/root");
        let directory = platform::open_root(&through_symlink).unwrap();
        let seen = platform::descriptor_path(&directory);
        assert_eq!(seen.as_deref(), Some(real.as_path()));
        assert_ne!(seen.as_deref(), Some(through_symlink.as_path()));
        assert!(platform::beneath_primitive_works());
        std::fs::remove_dir_all(&base).unwrap();
    }

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
