use super::{
    fresh_credential, validate_credential, CredentialFileError, ENCODED_CREDENTIAL_BYTES,
    MAX_PATH_BYTES,
};
use coding_adventures_zeroize::Zeroizing;
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::io::{Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};
use std::time::Duration;

// A losing thread in the create race waits here for the winner to finish
// writing, fsync-ing, and fchmod-ing the file it just created (see
// `create_new` below). 250ms was enough locally but proved too tight on
// loaded/shared CI runners: with 8 threads contending on a 2-vCPU box, the
// winner's `write_all` + two `sync_all` + `fchmod` can occasionally take
// longer than that, so losers exhausted retries and surfaced a spurious
// `AccessFailed` (see lessons.md). 3s of headroom absorbs that CI noise
// while still failing fast on a genuinely stuck/unavailable file.
const PUBLICATION_RETRIES: usize = 3000;

enum OpenFailure {
    Missing,
    Busy,
    Public(CredentialFileError),
}

enum CreateFailure {
    Exists,
    Public(CredentialFileError),
}

pub(super) fn load_or_create(path: &Path) -> Result<Zeroizing<String>, CredentialFileError> {
    let (parent, name) = open_parent(path)?;
    match open_existing(&parent, &name) {
        Ok(file) => load(file),
        Err(OpenFailure::Public(error)) => Err(error),
        Err(OpenFailure::Busy) => Err(CredentialFileError::AccessFailed),
        Err(OpenFailure::Missing) => {
            let credential = fresh_credential()?;
            match create_new(&parent, &name) {
                Ok(mut file) => {
                    verify_created_file(&file)?;
                    file.write_all(credential.as_bytes())
                        .and_then(|()| file.sync_all())
                        .map_err(|_| CredentialFileError::AccessFailed)?;
                    if unsafe { libc::fchmod(file.as_raw_fd(), 0o600) } != 0 {
                        return Err(CredentialFileError::AccessFailed);
                    }
                    file.sync_all()
                        .map_err(|_| CredentialFileError::AccessFailed)?;
                    if unsafe { libc::fsync(parent.as_raw_fd()) } != 0 {
                        return Err(CredentialFileError::AccessFailed);
                    }
                    Ok(credential)
                }
                Err(CreateFailure::Exists) => open_existing(&parent, &name)
                    .map_err(|failure| match failure {
                        OpenFailure::Missing
                        | OpenFailure::Busy
                        | OpenFailure::Public(CredentialFileError::AccessFailed) => {
                            CredentialFileError::AccessFailed
                        }
                        OpenFailure::Public(error) => error,
                    })
                    .and_then(load),
                Err(CreateFailure::Public(error)) => Err(error),
            }
        }
    }
}

fn open_parent(path: &Path) -> Result<(OwnedFd, CString), CredentialFileError> {
    if !path.is_absolute() || path.as_os_str().as_bytes().len() > MAX_PATH_BYTES {
        return Err(CredentialFileError::InvalidPath);
    }
    let components: Vec<_> = path.components().collect();
    if components.len() < 2 || components[0] != Component::RootDir {
        return Err(CredentialFileError::InvalidPath);
    }
    let name = match components.last() {
        Some(Component::Normal(name)) => c_string(name)?,
        _ => return Err(CredentialFileError::InvalidPath),
    };
    let root = unsafe {
        libc::open(
            c"/".as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    let mut directory = owned_fd(root).map_err(|_| CredentialFileError::ParentUnavailable)?;
    for component in &components[1..components.len() - 1] {
        let Component::Normal(part) = component else {
            return Err(CredentialFileError::InvalidPath);
        };
        let part = c_string(part)?;
        let next = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                part.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        directory = owned_fd(next).map_err(|_| CredentialFileError::ParentUnavailable)?;
    }
    Ok((directory, name))
}

fn c_string(value: &OsStr) -> Result<CString, CredentialFileError> {
    CString::new(value.as_bytes()).map_err(|_| CredentialFileError::InvalidPath)
}

fn owned_fd(raw: libc::c_int) -> Result<OwnedFd, ()> {
    if raw < 0 {
        Err(())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(raw) })
    }
}

fn open_existing(parent: &OwnedFd, name: &CString) -> Result<File, OpenFailure> {
    for _ in 0..PUBLICATION_RETRIES {
        match open_existing_once(parent, name) {
            Err(OpenFailure::Busy) => std::thread::sleep(Duration::from_millis(1)),
            result => return result,
        }
    }
    Err(OpenFailure::Public(CredentialFileError::AccessFailed))
}

fn open_existing_once(parent: &OwnedFd, name: &CString) -> Result<File, OpenFailure> {
    let raw = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if raw < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::ENOENT) => Err(OpenFailure::Missing),
            Some(libc::EACCES) => Err(OpenFailure::Busy),
            Some(libc::ELOOP) => Err(OpenFailure::Public(CredentialFileError::UnsafeFileType)),
            _ => Err(OpenFailure::Public(CredentialFileError::AccessFailed)),
        };
    }
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(raw) }))
}

fn create_new(parent: &OwnedFd, name: &CString) -> Result<File, CreateFailure> {
    let raw = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            0o000,
        )
    };
    if raw < 0 {
        return match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::EEXIST) => Err(CreateFailure::Exists),
            Some(libc::ELOOP) => Err(CreateFailure::Public(CredentialFileError::UnsafeFileType)),
            _ => Err(CreateFailure::Public(CredentialFileError::AccessFailed)),
        };
    }
    Ok(File::from(unsafe { OwnedFd::from_raw_fd(raw) }))
}

fn load(file: File) -> Result<Zeroizing<String>, CredentialFileError> {
    verify_file(&file)?;
    let mut bytes = Zeroizing::new(Vec::with_capacity(ENCODED_CREDENTIAL_BYTES + 1));
    file.take((ENCODED_CREDENTIAL_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| CredentialFileError::AccessFailed)?;
    validate_credential(&bytes)
}

fn verify_file(file: &File) -> Result<(), CredentialFileError> {
    let stat = file_stat(file)?;
    verify_identity(&stat)?;
    if stat.st_mode & 0o077 != 0 || stat.st_mode & 0o400 == 0 {
        return Err(CredentialFileError::InsecurePermissions);
    }
    Ok(())
}

fn verify_created_file(file: &File) -> Result<(), CredentialFileError> {
    let stat = file_stat(file)?;
    verify_identity(&stat)?;
    if stat.st_mode & 0o777 != 0 {
        return Err(CredentialFileError::InsecurePermissions);
    }
    Ok(())
}

fn file_stat(file: &File) -> Result<libc::stat, CredentialFileError> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    if unsafe { libc::fstat(file.as_raw_fd(), stat.as_mut_ptr()) } != 0 {
        return Err(CredentialFileError::AccessFailed);
    }
    Ok(unsafe { stat.assume_init() })
}

fn verify_identity(stat: &libc::stat) -> Result<(), CredentialFileError> {
    if stat.st_mode & libc::S_IFMT != libc::S_IFREG {
        return Err(CredentialFileError::UnsafeFileType);
    }
    if stat.st_uid != unsafe { libc::geteuid() } {
        return Err(CredentialFileError::InsecureOwner);
    }
    Ok(())
}
