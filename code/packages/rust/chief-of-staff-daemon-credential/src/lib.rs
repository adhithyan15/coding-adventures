//! Race-resistant owner-only local credential persistence for the D18 Chief daemon.

#![deny(missing_docs)]

use chief_of_staff_daemon_policy::{
    generate_local_credential, LocalAuthError, LocalBearerAuthorizer,
};
use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Display, Formatter};
use std::path::Path;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

const ENCODED_CREDENTIAL_BYTES: usize = 64;
const MAX_PATH_BYTES: usize = 4096;

/// Stable, payload-blind credential-file failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CredentialFileError {
    /// The supplied path was not a bounded absolute path with a regular final name.
    InvalidPath,
    /// The parent directory chain could not be opened without following links.
    ParentUnavailable,
    /// The credential file could not be opened, read, written, or synchronized.
    AccessFailed,
    /// The path resolved to a link, reparse point, directory, or other non-regular object.
    UnsafeFileType,
    /// The existing file is not owned by the current effective user.
    InsecureOwner,
    /// The existing file grants access beyond its owner.
    InsecurePermissions,
    /// The existing file was not exactly one canonical 64-byte lowercase-hex credential.
    InvalidCredential,
    /// The operating-system random source could not generate a fresh credential.
    RandomnessUnavailable,
}

impl Display for CredentialFileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPath => "chief credential: invalid path",
            Self::ParentUnavailable => "chief credential: parent unavailable",
            Self::AccessFailed => "chief credential: access failed",
            Self::UnsafeFileType => "chief credential: unsafe file type",
            Self::InsecureOwner => "chief credential: insecure owner",
            Self::InsecurePermissions => "chief credential: insecure permissions",
            Self::InvalidCredential => "chief credential: invalid credential",
            Self::RandomnessUnavailable => "chief credential: randomness unavailable",
        })
    }
}

impl std::error::Error for CredentialFileError {}

/// Load one existing owner-only credential, or atomically claim an absent file name and create it.
///
/// The path must be absolute and its parent directory must already exist. The adapter never
/// overwrites an existing file. It rejects links and non-regular objects, validates existing
/// ownership and access controls before reading, caps the read at 65 bytes, and returns secret
/// material in a zeroizing allocation.
pub fn load_or_create_credential(path: &Path) -> Result<Zeroizing<String>, CredentialFileError> {
    #[cfg(unix)]
    {
        unix::load_or_create(path)
    }
    #[cfg(windows)]
    {
        windows::load_or_create(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(CredentialFileError::AccessFailed)
    }
}

fn fresh_credential() -> Result<Zeroizing<String>, CredentialFileError> {
    generate_local_credential().map_err(|error| match error {
        LocalAuthError::RandomnessUnavailable => CredentialFileError::RandomnessUnavailable,
        LocalAuthError::InvalidCredentialEncoding | LocalAuthError::AuthenticationFailed => {
            CredentialFileError::InvalidCredential
        }
    })
}

fn validate_credential(bytes: &[u8]) -> Result<Zeroizing<String>, CredentialFileError> {
    if bytes.len() != ENCODED_CREDENTIAL_BYTES {
        return Err(CredentialFileError::InvalidCredential);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| CredentialFileError::InvalidCredential)?;
    LocalBearerAuthorizer::new(text).map_err(|_| CredentialFileError::InvalidCredential)?;
    Ok(Zeroizing::new(text.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let temporary_root = fs::canonicalize(std::env::temp_dir()).unwrap();
            let path = temporary_root.join(format!(
                "chief-credential-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn credential(&self) -> PathBuf {
            self.0.join("operator.credential")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn creates_once_then_loads_the_identical_canonical_credential() {
        let directory = TestDirectory::new("round-trip");
        let path = directory.credential();
        let first = load_or_create_credential(&path).unwrap();
        let second = load_or_create_credential(&path).unwrap();
        assert_eq!(&*first, &*second);
        assert_eq!(fs::read(&path).unwrap(), first.as_bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn concurrent_creators_converge_without_overwrite() {
        let directory = TestDirectory::new("race");
        let path = Arc::new(directory.credential());
        let barrier = Arc::new(Barrier::new(8));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let path = Arc::clone(&path);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    load_or_create_credential(&path).unwrap().to_string()
                })
            })
            .collect();
        let credentials: Vec<_> = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect();
        assert!(credentials
            .iter()
            .all(|credential| credential == &credentials[0]));
    }

    #[test]
    fn invalid_existing_content_is_preserved_and_rejected() {
        let directory = TestDirectory::new("invalid");
        let path = directory.credential();
        load_or_create_credential(&path).unwrap();
        fs::write(&path, b"not-a-credential").unwrap();
        assert!(matches!(
            load_or_create_credential(&path),
            Err(CredentialFileError::InvalidCredential)
        ));
        assert_eq!(fs::read(path).unwrap(), b"not-a-credential");
    }

    #[test]
    fn missing_parent_and_relative_paths_fail_closed() {
        let directory = TestDirectory::new("paths");
        assert!(matches!(
            load_or_create_credential(Path::new("relative.credential")),
            Err(CredentialFileError::InvalidPath)
        ));
        assert!(matches!(
            load_or_create_credential(&directory.0.join("missing/credential")),
            Err(CredentialFileError::ParentUnavailable)
        ));
    }

    #[test]
    fn errors_are_stable_and_payload_blind() {
        let expected = [
            (
                CredentialFileError::InvalidPath,
                "chief credential: invalid path",
            ),
            (
                CredentialFileError::ParentUnavailable,
                "chief credential: parent unavailable",
            ),
            (
                CredentialFileError::AccessFailed,
                "chief credential: access failed",
            ),
            (
                CredentialFileError::UnsafeFileType,
                "chief credential: unsafe file type",
            ),
            (
                CredentialFileError::InsecureOwner,
                "chief credential: insecure owner",
            ),
            (
                CredentialFileError::InsecurePermissions,
                "chief credential: insecure permissions",
            ),
            (
                CredentialFileError::InvalidCredential,
                "chief credential: invalid credential",
            ),
            (
                CredentialFileError::RandomnessUnavailable,
                "chief credential: randomness unavailable",
            ),
        ];
        for (error, message) in expected {
            assert_eq!(error.to_string(), message);
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_directories_and_broad_permissions() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let directory = TestDirectory::new("unsafe");
        let target = directory.0.join("target");
        fs::write(
            &target,
            b"0000000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();

        let link = directory.credential();
        symlink(&target, &link).unwrap();
        assert!(matches!(
            load_or_create_credential(&link),
            Err(CredentialFileError::UnsafeFileType)
        ));
        fs::remove_file(&link).unwrap();

        fs::create_dir(&link).unwrap();
        assert!(matches!(
            load_or_create_credential(&link),
            Err(CredentialFileError::UnsafeFileType)
        ));
        fs::remove_dir(&link).unwrap();

        fs::copy(&target, &link).unwrap();
        fs::set_permissions(&link, fs::Permissions::from_mode(0o640)).unwrap();
        assert!(matches!(
            load_or_create_credential(&link),
            Err(CredentialFileError::InsecurePermissions)
        ));

        let real_parent = directory.0.join("real-parent");
        let linked_parent = directory.0.join("linked-parent");
        fs::create_dir(&real_parent).unwrap();
        symlink(&real_parent, &linked_parent).unwrap();
        assert!(matches!(
            load_or_create_credential(&linked_parent.join("credential")),
            Err(CredentialFileError::ParentUnavailable)
        ));
    }
}
