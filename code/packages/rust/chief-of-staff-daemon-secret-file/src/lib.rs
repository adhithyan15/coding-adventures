//! Race-resistant owner-only secret-file loading for D18 production adapters.

#![deny(missing_docs)]

use coding_adventures_zeroize::Zeroizing;
use core::fmt::{self, Display, Formatter};
use std::path::Path;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

const MAX_PATH_BYTES: usize = 4096;
const MAX_SECRET_BYTES: usize = 64 * 1024;

/// Stable payload-blind failure while loading one operator-owned secret file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretFileError {
    /// The supplied path was not a bounded absolute path with a regular final name.
    InvalidPath,
    /// The parent directory chain could not be opened without following links.
    ParentUnavailable,
    /// The secret file could not be opened, inspected, or read.
    AccessFailed,
    /// The path resolved to a link, reparse point, directory, or other non-regular object.
    UnsafeFileType,
    /// The file is not owned by the current effective user.
    InsecureOwner,
    /// The file grants access beyond its owner.
    InsecurePermissions,
    /// The requested or stored secret length violated the exact bounded contract.
    InvalidLength,
}

impl Display for SecretFileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidPath => "chief secret file: invalid path",
            Self::ParentUnavailable => "chief secret file: parent unavailable",
            Self::AccessFailed => "chief secret file: access failed",
            Self::UnsafeFileType => "chief secret file: unsafe file type",
            Self::InsecureOwner => "chief secret file: insecure owner",
            Self::InsecurePermissions => "chief secret file: insecure permissions",
            Self::InvalidLength => "chief secret file: invalid length",
        })
    }
}

impl std::error::Error for SecretFileError {}

/// Read one existing exact-length secret after enforcing platform owner-only policy.
///
/// The path must be absolute and the expected length must be between 1 byte and
/// 64 KiB. Parent traversal and the final open never follow links. Returned bytes
/// are wiped on drop; errors never include paths or file content.
pub fn read_owner_only_secret(
    path: &Path,
    expected_length: usize,
) -> Result<Zeroizing<Vec<u8>>, SecretFileError> {
    if expected_length == 0 || expected_length > MAX_SECRET_BYTES {
        return Err(SecretFileError::InvalidLength);
    }
    #[cfg(unix)]
    {
        unix::read(path, expected_length)
    }
    #[cfg(windows)]
    {
        windows::read(path, expected_length)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(SecretFileError::AccessFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "chief-secret-file-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }

        fn secret(&self) -> PathBuf {
            self.0.join("secret.bin")
        }

        fn write_secret(&self, bytes: &[u8]) {
            #[cfg(unix)]
            unix::write_test_secret(&self.secret(), bytes);
            #[cfg(windows)]
            windows::write_test_secret(&self.secret(), bytes);
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn error(result: Result<Zeroizing<Vec<u8>>, SecretFileError>) -> SecretFileError {
        match result {
            Err(error) => error,
            Ok(_) => panic!("secret read unexpectedly succeeded"),
        }
    }

    #[test]
    fn reads_only_the_exact_secret_into_zeroizing_storage() {
        let directory = TestDirectory::new("exact");
        directory.write_secret(&[0x42; 32]);
        let secret = read_owner_only_secret(&directory.secret(), 32).unwrap();
        assert_eq!(secret.as_slice(), &[0x42; 32]);
        assert_eq!(
            error(read_owner_only_secret(&directory.secret(), 31)),
            SecretFileError::InvalidLength
        );
        assert_eq!(
            error(read_owner_only_secret(&directory.secret(), 33)),
            SecretFileError::InvalidLength
        );
    }

    #[test]
    fn invalid_lengths_paths_and_objects_fail_closed() {
        let directory = TestDirectory::new("invalid");
        assert_eq!(
            error(read_owner_only_secret(&directory.secret(), 0)),
            SecretFileError::InvalidLength
        );
        assert_eq!(
            error(read_owner_only_secret(
                &directory.secret(),
                MAX_SECRET_BYTES + 1,
            )),
            SecretFileError::InvalidLength
        );
        assert_eq!(
            error(read_owner_only_secret(Path::new("relative.bin"), 32)),
            SecretFileError::InvalidPath
        );
        assert_eq!(
            error(read_owner_only_secret(&directory.secret(), 32)),
            SecretFileError::AccessFailed
        );
        fs::create_dir(directory.secret()).unwrap();
        assert_eq!(
            error(read_owner_only_secret(&directory.secret(), 32)),
            SecretFileError::UnsafeFileType
        );
    }

    #[test]
    fn errors_are_stable_and_payload_blind() {
        let messages = [
            (
                SecretFileError::InvalidPath,
                "chief secret file: invalid path",
            ),
            (
                SecretFileError::ParentUnavailable,
                "chief secret file: parent unavailable",
            ),
            (
                SecretFileError::AccessFailed,
                "chief secret file: access failed",
            ),
            (
                SecretFileError::UnsafeFileType,
                "chief secret file: unsafe file type",
            ),
            (
                SecretFileError::InsecureOwner,
                "chief secret file: insecure owner",
            ),
            (
                SecretFileError::InsecurePermissions,
                "chief secret file: insecure permissions",
            ),
            (
                SecretFileError::InvalidLength,
                "chief secret file: invalid length",
            ),
        ];
        for (error, message) in messages {
            assert_eq!(error.to_string(), message);
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_link_traversal_and_broad_permissions() {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::{symlink, PermissionsExt};

        let directory = TestDirectory::new("unix-policy");
        directory.write_secret(&[0x55; 32]);
        fs::set_permissions(directory.secret(), fs::Permissions::from_mode(0o640)).unwrap();
        assert_eq!(
            error(read_owner_only_secret(&directory.secret(), 32)),
            SecretFileError::InsecurePermissions
        );

        let target = directory.0.join("target.bin");
        fs::rename(directory.secret(), &target).unwrap();
        symlink(&target, directory.secret()).unwrap();
        assert_eq!(
            error(read_owner_only_secret(&directory.secret(), 32)),
            SecretFileError::UnsafeFileType
        );

        let real_parent = directory.0.join("real-parent");
        let linked_parent = directory.0.join("linked-parent");
        fs::create_dir(&real_parent).unwrap();
        symlink(&real_parent, &linked_parent).unwrap();
        assert_eq!(
            error(read_owner_only_secret(
                &linked_parent.join("secret.bin"),
                32,
            )),
            SecretFileError::ParentUnavailable
        );

        let fifo = directory.0.join("secret.fifo");
        let fifo_path = CString::new(fifo.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo_path.as_ptr(), 0o600) }, 0);
        assert_eq!(
            error(read_owner_only_secret(&fifo, 32)),
            SecretFileError::UnsafeFileType
        );
    }
}
