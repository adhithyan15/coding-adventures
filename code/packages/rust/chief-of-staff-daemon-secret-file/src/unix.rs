use super::{SecretFileError, MAX_PATH_BYTES};
use coding_adventures_zeroize::Zeroizing;
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::io::Read;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};

pub(super) fn read(
    path: &Path,
    expected_length: usize,
) -> Result<Zeroizing<Vec<u8>>, SecretFileError> {
    let (parent, name) = open_parent(path)?;
    let raw = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if raw < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ELOOP) => Err(SecretFileError::UnsafeFileType),
            _ => Err(SecretFileError::AccessFailed),
        };
    }
    let file = File::from(unsafe { OwnedFd::from_raw_fd(raw) });
    verify_file(&file)?;
    read_exact_bounded(file, expected_length)
}

fn open_parent(path: &Path) -> Result<(OwnedFd, CString), SecretFileError> {
    if !path.is_absolute() || path.as_os_str().as_bytes().len() > MAX_PATH_BYTES {
        return Err(SecretFileError::InvalidPath);
    }
    let components: Vec<_> = path.components().collect();
    if components.len() < 2 || components[0] != Component::RootDir {
        return Err(SecretFileError::InvalidPath);
    }
    let name = match components.last() {
        Some(Component::Normal(name)) => c_string(name)?,
        _ => return Err(SecretFileError::InvalidPath),
    };
    let root = unsafe {
        libc::open(
            c"/".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    let mut directory = owned_fd(root).map_err(|_| SecretFileError::ParentUnavailable)?;
    for component in &components[1..components.len() - 1] {
        let Component::Normal(part) = component else {
            return Err(SecretFileError::InvalidPath);
        };
        let part = c_string(part)?;
        let next = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                part.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        directory = owned_fd(next).map_err(|_| SecretFileError::ParentUnavailable)?;
    }
    Ok((directory, name))
}

fn c_string(value: &OsStr) -> Result<CString, SecretFileError> {
    CString::new(value.as_bytes()).map_err(|_| SecretFileError::InvalidPath)
}

fn owned_fd(raw: libc::c_int) -> Result<OwnedFd, ()> {
    if raw < 0 {
        Err(())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(raw) })
    }
}

fn verify_file(file: &File) -> Result<(), SecretFileError> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(file.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(SecretFileError::AccessFailed);
    }
    let stat = unsafe { stat.assume_init() };
    if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
        return Err(SecretFileError::UnsafeFileType);
    }
    if stat.st_uid != unsafe { libc::geteuid() } {
        return Err(SecretFileError::InsecureOwner);
    }
    if stat.st_mode & 0o077 != 0 || stat.st_mode & 0o400 == 0 {
        return Err(SecretFileError::InsecurePermissions);
    }
    Ok(())
}

fn read_exact_bounded(
    file: File,
    expected_length: usize,
) -> Result<Zeroizing<Vec<u8>>, SecretFileError> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(expected_length + 1));
    file.take((expected_length + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| SecretFileError::AccessFailed)?;
    if bytes.len() != expected_length {
        return Err(SecretFileError::InvalidLength);
    }
    Ok(bytes)
}

#[cfg(test)]
pub(super) fn write_test_secret(path: &Path, bytes: &[u8]) {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
