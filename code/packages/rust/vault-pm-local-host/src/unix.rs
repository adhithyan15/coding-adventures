use super::{LocalHostError, MAX_PATH_BYTES};
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};

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
