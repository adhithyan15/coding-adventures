use super::{LocalHostError, MAX_PATH_BYTES};
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);
const MAX_TEMPORARY_ATTEMPTS: usize = 64;

pub(super) fn ensure_private_directory(path: &Path) -> Result<(), LocalHostError> {
    validate_absolute(path)?;
    let components: Vec<_> = path.components().collect();
    if components.len() < 2 || components[0] != Component::RootDir {
        return Err(LocalHostError::InvalidPath);
    }

    let root = unsafe {
        libc::open(
            c"/".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    let mut directory = owned_fd(root).map_err(|_| LocalHostError::ParentUnavailable)?;
    for (index, component) in components[1..].iter().enumerate() {
        let Component::Normal(part) = component else {
            return Err(LocalHostError::InvalidPath);
        };
        let name = c_string(part)?;
        let mut raw = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if raw < 0 && std::io::Error::last_os_error().raw_os_error() == Some(libc::ENOENT) {
            if unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::EEXIST) {
                    return Err(LocalHostError::AccessFailed);
                }
            }
            raw = unsafe {
                libc::openat(
                    directory.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
        }
        if raw < 0 {
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ELOOP) | Some(libc::ENOTDIR) => LocalHostError::UnsafeObjectType,
                _ if index + 1 == components.len() - 1 => LocalHostError::AccessFailed,
                _ => LocalHostError::ParentUnavailable,
            });
        }
        directory = owned_fd(raw).map_err(|_| LocalHostError::AccessFailed)?;
    }
    verify_private_directory(&directory)
}

/// Verify or create one owner-private directory whose parent is *not* walked.
///
/// [`ensure_private_directory`] above is the right tool for a root this crate
/// owns end to end — every ancestor from `/` is opened with `O_NOFOLLOW` so a
/// symlink planted anywhere in the path is refused. That is too strict for
/// [`super::LocalVaultPaths::runtime_root`], whose parent is the *system*
/// temporary directory: on macOS both `/tmp` and `/var` are themselves
/// platform-placed symlinks (`/tmp -> private/tmp`), and the recursive walk
/// would reject them with [`LocalHostError::UnsafeObjectType`] before this
/// crate's own directory is ever reached.
///
/// The trust boundary this function draws instead: the *parent* (the system
/// temp directory) is accepted as the platform gives it, exactly as
/// [`super::LocalVaultPaths::resolve`] already trusts `ProjectDirs` to hand
/// back sane platform roots. Only the *leaf* — the directory this crate
/// creates and every later `agent start` or client reuses — is defended: it
/// is opened with `O_NOFOLLOW`, so a symlink swapped in at that exact path is
/// refused rather than followed, and its owner and mode are verified exactly
/// as [`verify_private_directory`] requires of every other private root.
///
/// A parent directory that is world-writable without a sticky bit (an
/// unusual system temp directory) could let another local user pre-create
/// this leaf first. That race is refused, not silently accepted: an
/// existing directory owned by another user fails closed with
/// [`LocalHostError::InsecureOwner`], the same outcome
/// [`ensure_private_directory`] gives a foreign-owned root. The residual is a
/// denial of the agent feature to whichever party loses the race, never a
/// disclosure — no key, passphrase, or socket connection is granted merely
/// because a directory exists.
pub(super) fn ensure_private_runtime_directory(path: &Path) -> Result<(), LocalHostError> {
    validate_absolute(path)?;
    let parent_path = path.parent().ok_or(LocalHostError::InvalidPath)?;
    let Some(leaf) = path.file_name() else {
        return Err(LocalHostError::InvalidPath);
    };
    let parent_c = CString::new(parent_path.as_os_str().as_bytes())
        .map_err(|_| LocalHostError::InvalidPath)?;
    let leaf = c_string(leaf)?;
    // The parent is opened following ordinary path resolution (no
    // `O_NOFOLLOW`): see this function's doc comment for why its ancestry is
    // trusted rather than walked.
    let parent_raw = unsafe { libc::open(parent_c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY) };
    let parent = owned_fd(parent_raw).map_err(|_| LocalHostError::ParentUnavailable)?;
    if unsafe { libc::mkdirat(parent.as_raw_fd(), leaf.as_ptr(), 0o700) } != 0
        && std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST)
    {
        return Err(LocalHostError::AccessFailed);
    }
    let leaf_raw = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            leaf.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if leaf_raw < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ELOOP) | Some(libc::ENOTDIR) => LocalHostError::UnsafeObjectType,
            _ => LocalHostError::AccessFailed,
        });
    }
    let directory = owned_fd(leaf_raw).map_err(|_| LocalHostError::AccessFailed)?;
    verify_private_directory(&directory)
}

pub(super) fn open_private_lock(path: &Path) -> Result<File, LocalHostError> {
    let (parent, name) = open_existing_parent(path)?;
    let raw = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o600,
        )
    };
    if raw < 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ELOOP) | Some(libc::EISDIR) => LocalHostError::UnsafeObjectType,
            _ => LocalHostError::AccessFailed,
        });
    }
    let file = File::from(owned_fd(raw).map_err(|_| LocalHostError::AccessFailed)?);
    verify_private_file(&file)?;
    Ok(file)
}

pub(super) fn load_private_config(
    path: &Path,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, LocalHostError> {
    let (parent, name) = open_existing_parent(path)?;
    verify_private_directory(&parent)?;
    read_private_file(&parent, &name, max_bytes)
}

pub(super) fn create_private_config(
    path: &Path,
    bytes: &[u8],
    max_bytes: usize,
) -> Result<(), LocalHostError> {
    let (parent, name) = open_existing_parent(path)?;
    verify_private_directory(&parent)?;
    if read_private_file(&parent, &name, max_bytes)?.is_some() {
        return Err(LocalHostError::ConfigAlreadyExists);
    }
    let (temporary, temporary_name) = create_temporary(&parent)?;
    let result = (|| {
        persist_temporary(&temporary, bytes)?;
        if unsafe {
            libc::linkat(
                parent.as_raw_fd(),
                temporary_name.as_ptr(),
                parent.as_raw_fd(),
                name.as_ptr(),
                0,
            )
        } != 0
        {
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::EEXIST) => LocalHostError::ConfigAlreadyExists,
                _ => LocalHostError::AccessFailed,
            });
        }
        sync_directory(&parent)?;
        verify_named_private_file(&parent, &name)?;
        Ok(())
    })();
    unlink_temporary(&parent, &temporary_name);
    if result.is_ok() {
        sync_directory(&parent)?;
    }
    result
}

pub(super) fn compare_exchange_private_config(
    path: &Path,
    expected: &[u8],
    replacement: &[u8],
    max_bytes: usize,
) -> Result<(), LocalHostError> {
    let (parent, name) = open_existing_parent(path)?;
    verify_private_directory(&parent)?;
    if read_private_file(&parent, &name, max_bytes)?.as_deref() != Some(expected) {
        return Err(LocalHostError::ConfigConflict);
    }
    let (temporary, temporary_name) = create_temporary(&parent)?;
    let result = (|| {
        persist_temporary(&temporary, replacement)?;
        if unsafe {
            libc::renameat(
                parent.as_raw_fd(),
                temporary_name.as_ptr(),
                parent.as_raw_fd(),
                name.as_ptr(),
            )
        } != 0
        {
            return Err(LocalHostError::AccessFailed);
        }
        sync_directory(&parent)?;
        verify_named_private_file(&parent, &name)
    })();
    if result.is_err() {
        unlink_temporary(&parent, &temporary_name);
    }
    result
}

fn read_private_file(
    parent: &OwnedFd,
    name: &CString,
    max_bytes: usize,
) -> Result<Option<Vec<u8>>, LocalHostError> {
    let raw = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if raw < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Ok(None),
            Some(libc::ELOOP) | Some(libc::EISDIR) => Err(LocalHostError::UnsafeObjectType),
            _ => Err(LocalHostError::AccessFailed),
        };
    }
    let file = File::from(owned_fd(raw).map_err(|_| LocalHostError::AccessFailed)?);
    verify_private_file(&file)?;
    let mut bytes = Vec::new();
    file.take((max_bytes as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| LocalHostError::AccessFailed)?;
    if bytes.is_empty() || bytes.len() > max_bytes {
        return Err(LocalHostError::InvalidConfigBytes);
    }
    Ok(Some(bytes))
}

fn create_temporary(parent: &OwnedFd) -> Result<(File, CString), LocalHostError> {
    for _ in 0..MAX_TEMPORARY_ATTEMPTS {
        let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let name = CString::new(format!(
            ".vault-pm.toml.tmp.{}.{}",
            std::process::id(),
            sequence
        ))
        .map_err(|_| LocalHostError::AccessFailed)?;
        let raw = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                0o600,
            )
        };
        if raw >= 0 {
            let file = File::from(owned_fd(raw).map_err(|_| LocalHostError::AccessFailed)?);
            verify_private_file(&file)?;
            return Ok((file, name));
        }
        if std::io::Error::last_os_error().raw_os_error() != Some(libc::EEXIST) {
            return Err(LocalHostError::AccessFailed);
        }
    }
    Err(LocalHostError::AccessFailed)
}

fn persist_temporary(mut file: &File, bytes: &[u8]) -> Result<(), LocalHostError> {
    file.write_all(bytes)
        .map_err(|_| LocalHostError::AccessFailed)?;
    file.sync_all().map_err(|_| LocalHostError::AccessFailed)
}

fn verify_named_private_file(parent: &OwnedFd, name: &CString) -> Result<(), LocalHostError> {
    read_private_file(parent, name, usize::MAX).map(|_| ())
}

fn sync_directory(parent: &OwnedFd) -> Result<(), LocalHostError> {
    if unsafe { libc::fsync(parent.as_raw_fd()) } == 0 {
        Ok(())
    } else {
        Err(LocalHostError::AccessFailed)
    }
}

fn unlink_temporary(parent: &OwnedFd, name: &CString) {
    unsafe {
        libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0);
    }
}

fn open_existing_parent(path: &Path) -> Result<(OwnedFd, CString), LocalHostError> {
    validate_absolute(path)?;
    let components: Vec<_> = path.components().collect();
    if components.len() < 2 || components[0] != Component::RootDir {
        return Err(LocalHostError::InvalidPath);
    }
    let name = match components.last() {
        Some(Component::Normal(name)) => c_string(name)?,
        _ => return Err(LocalHostError::InvalidPath),
    };
    let root = unsafe {
        libc::open(
            c"/".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    let mut directory = owned_fd(root).map_err(|_| LocalHostError::ParentUnavailable)?;
    for component in &components[1..components.len() - 1] {
        let Component::Normal(part) = component else {
            return Err(LocalHostError::InvalidPath);
        };
        let part = c_string(part)?;
        let raw = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                part.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if raw < 0 {
            return Err(match std::io::Error::last_os_error().raw_os_error() {
                Some(libc::ELOOP) | Some(libc::ENOTDIR) => LocalHostError::UnsafeObjectType,
                _ => LocalHostError::ParentUnavailable,
            });
        }
        directory = owned_fd(raw).map_err(|_| LocalHostError::ParentUnavailable)?;
    }
    Ok((directory, name))
}

fn validate_absolute(path: &Path) -> Result<(), LocalHostError> {
    if !path.is_absolute()
        || path.as_os_str().as_bytes().is_empty()
        || path.as_os_str().as_bytes().len() > MAX_PATH_BYTES
        || path
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
    {
        return Err(LocalHostError::InvalidPath);
    }
    Ok(())
}

fn c_string(value: &OsStr) -> Result<CString, LocalHostError> {
    CString::new(value.as_bytes()).map_err(|_| LocalHostError::InvalidPath)
}

fn owned_fd(raw: libc::c_int) -> Result<OwnedFd, ()> {
    if raw < 0 {
        Err(())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(raw) })
    }
}

fn stat(raw: libc::c_int) -> Result<libc::stat, LocalHostError> {
    let mut value = MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(raw, value.as_mut_ptr()) } != 0 {
        return Err(LocalHostError::AccessFailed);
    }
    Ok(unsafe { value.assume_init() })
}

fn verify_private_directory(directory: &OwnedFd) -> Result<(), LocalHostError> {
    let value = stat(directory.as_raw_fd())?;
    if value.st_mode & libc::S_IFMT != libc::S_IFDIR {
        return Err(LocalHostError::UnsafeObjectType);
    }
    verify_owner_permissions(&value, 0o700)
}

fn verify_private_file(file: &File) -> Result<(), LocalHostError> {
    let value = stat(file.as_raw_fd())?;
    if value.st_mode & libc::S_IFMT != libc::S_IFREG {
        return Err(LocalHostError::UnsafeObjectType);
    }
    verify_owner_permissions(&value, 0o600)
}

fn verify_owner_permissions(
    value: &libc::stat,
    required_owner: libc::mode_t,
) -> Result<(), LocalHostError> {
    if value.st_uid != unsafe { libc::geteuid() } {
        return Err(LocalHostError::InsecureOwner);
    }
    let permissions = value.st_mode & 0o777;
    if permissions & 0o077 != 0 || permissions & required_owner != required_owner {
        return Err(LocalHostError::InsecurePermissions);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, OpenOptions};
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "vault-pm-local-host-unix-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn low_level_validation_and_type_checks_fail_closed() {
        assert_eq!(
            validate_absolute(Path::new("relative")),
            Err(LocalHostError::InvalidPath)
        );
        assert!(c_string(OsStr::from_bytes(b"bad\0name")).is_err());
        assert!(owned_fd(-1).is_err());
        assert_eq!(stat(-1).unwrap_err(), LocalHostError::AccessFailed);

        let directory = TestDirectory::new("types");
        let regular_path = directory.0.join("regular");
        let regular = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&regular_path)
            .unwrap();
        fs::set_permissions(&regular_path, fs::Permissions::from_mode(0o600)).unwrap();
        let regular_fd: OwnedFd = regular.into();
        assert_eq!(
            verify_private_directory(&regular_fd),
            Err(LocalHostError::UnsafeObjectType)
        );

        let directory_file = File::open(&directory.0).unwrap();
        assert_eq!(
            verify_private_file(&directory_file),
            Err(LocalHostError::UnsafeObjectType)
        );

        let mut foreign: libc::stat = unsafe { std::mem::zeroed() };
        foreign.st_uid = unsafe { libc::geteuid() }.wrapping_add(1);
        foreign.st_mode = 0o700;
        assert_eq!(
            verify_owner_permissions(&foreign, 0o700),
            Err(LocalHostError::InsecureOwner)
        );
        foreign.st_uid = unsafe { libc::geteuid() };
        foreign.st_mode = 0o500;
        assert_eq!(
            verify_owner_permissions(&foreign, 0o700),
            Err(LocalHostError::InsecurePermissions)
        );

        assert_eq!(
            ensure_private_directory(Path::new("/")),
            Err(LocalHostError::InvalidPath)
        );
        let oversized_name = directory.0.join("x".repeat(300));
        assert_eq!(
            open_private_lock(&oversized_name).unwrap_err(),
            LocalHostError::AccessFailed
        );
        assert_eq!(
            load_private_config(&oversized_name, 64).unwrap_err(),
            LocalHostError::AccessFailed
        );

        let mut pipe = [-1; 2];
        assert_eq!(unsafe { libc::pipe(pipe.as_mut_ptr()) }, 0);
        let read_end = owned_fd(pipe[0]).unwrap();
        let write_end = owned_fd(pipe[1]).unwrap();
        assert_eq!(sync_directory(&read_end), Err(LocalHostError::AccessFailed));
        drop(write_end);
    }

    #[test]
    fn missing_and_unsearchable_parents_are_closed() {
        let directory = TestDirectory::new("parents");
        assert_eq!(
            open_private_lock(&directory.0.join("missing/lock")).unwrap_err(),
            LocalHostError::ParentUnavailable
        );

        let blocked = directory.0.join("blocked");
        fs::create_dir(&blocked).unwrap();
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o500)).unwrap();
        assert_eq!(
            ensure_private_directory(&blocked.join("child")),
            Err(LocalHostError::AccessFailed)
        );
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o700)).unwrap();
    }
}
