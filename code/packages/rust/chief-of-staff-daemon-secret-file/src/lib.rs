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

/// Open one secret file and enforce the owner-only policy, without reading
/// it (D18S P2.6d).
///
/// The same race-resistant walk as [`read_owner_only_secret`]. A supervisor
/// passes the returned descriptor to the one broker that needs the key, so
/// the key's bytes enter that broker's address space and no other. This is
/// about where the bytes live, not who could read them: the opener could.
///
/// On Windows the handle shares read only, and the policy is the one
/// [`read_owner_only_secret`] enforces there (D18S P2.8b-1).
pub fn open_owner_only_secret(path: &Path) -> Result<std::fs::File, SecretFileError> {
    #[cfg(unix)]
    {
        unix::open(path)
    }
    #[cfg(windows)]
    {
        windows::open(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(SecretFileError::AccessFailed)
    }
}

/// Check that `path` is a directory only its owner can use: owned by the
/// current effective user, with nothing granted to group or others
/// (`mode & 0o077 == 0`). The walk never follows a link, as for a secret
/// file (D18S P2.6d-3).
///
/// A directory that holds secrets must be like this for P2.6c's hard-link
/// check to mean anything: linking a file needs search permission on its
/// directory. On Windows (D18S P2.8b-1): owned by the user, SYSTEM or
/// Administrators, with every allow entry for one of those three (and
/// CREATOR OWNER, inherit-only), reached without a reparse point.
pub fn check_owner_only_directory(path: &Path) -> Result<(), SecretFileError> {
    #[cfg(unix)]
    {
        unix::check_directory(path)
    }
    #[cfg(windows)]
    {
        windows::check_directory(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(SecretFileError::AccessFailed)
    }
}

/// Read one exact-length secret from an already-open file, after enforcing
/// the owner-only policy on it (D18S P2.6d).
///
/// For a descriptor a broker inherited from [`open_owner_only_secret`] in its
/// supervisor. The policy is checked again here, on the descriptor itself.
/// The read starts at offset 0. On Unix it leaves the file's offset alone,
/// since the offset is shared with the opener; Windows has no read that
/// does, and there the opener's copy is closed when the broker starts. The
/// caller then drops the file.
pub fn read_owner_only_secret_from(
    file: &std::fs::File,
    expected_length: usize,
) -> Result<Zeroizing<Vec<u8>>, SecretFileError> {
    if expected_length == 0 || expected_length > MAX_SECRET_BYTES {
        return Err(SecretFileError::InvalidLength);
    }
    #[cfg(unix)]
    {
        unix::read_from(file, expected_length)
    }
    #[cfg(windows)]
    {
        windows::read_from(file, expected_length)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = file;
        Err(SecretFileError::AccessFailed)
    }
}

/// Test support, with the `test-support` feature only: write `bytes` to a
/// new file at `path` under exactly the owner-only policy this crate
/// enforces (Unix: mode 0600; Windows: the user as owner, a protected DACL
/// holding only the user's ACE). A crate whose tests feed secrets to a
/// loader built on this one creates them here rather than reproducing the
/// policy, which on Windows no portable `std` call can set. Panics on any
/// failure; it is for tests, and production never creates secrets here.
#[cfg(feature = "test-support")]
pub fn write_test_secret(path: &Path, bytes: &[u8]) {
    #[cfg(unix)]
    unix::write_test_secret(path, bytes);
    #[cfg(windows)]
    windows::write_test_secret(path, bytes);
    #[cfg(not(any(unix, windows)))]
    let _ = (path, bytes);
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

    #[cfg(unix)]
    #[test]
    fn only_an_owner_only_directory_reached_without_links_passes() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let directory = TestDirectory::new("owner-only-dir");
        let keys = directory.0.join("keys");
        fs::create_dir(&keys).unwrap();
        fs::set_permissions(&keys, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(check_owner_only_directory(&keys), Ok(()));
        // Search for the group is enough to refuse it, as is any read.
        for mode in [0o710, 0o701, 0o750, 0o755, 0o770] {
            fs::set_permissions(&keys, fs::Permissions::from_mode(mode)).unwrap();
            assert_eq!(
                check_owner_only_directory(&keys),
                Err(SecretFileError::InsecurePermissions),
                "{mode:o}"
            );
        }
        fs::set_permissions(&keys, fs::Permissions::from_mode(0o700)).unwrap();
        // A link to it, a file, a missing path, and a relative path.
        let linked = directory.0.join("linked");
        symlink(&keys, &linked).unwrap();
        assert_eq!(
            check_owner_only_directory(&linked),
            Err(SecretFileError::UnsafeFileType)
        );
        directory.write_secret(&[1; 32]);
        assert_eq!(
            check_owner_only_directory(&directory.secret()),
            Err(SecretFileError::UnsafeFileType)
        );
        assert_eq!(
            check_owner_only_directory(&directory.0.join("missing")),
            Err(SecretFileError::AccessFailed)
        );
        assert_eq!(
            check_owner_only_directory(Path::new("relative/keys")),
            Err(SecretFileError::InvalidPath)
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_opened_secret_is_read_from_its_descriptor_at_offset_zero() {
        use std::io::Read;
        use std::os::unix::fs::PermissionsExt;
        let directory = TestDirectory::new("descriptor");
        directory.write_secret(&[7; 32]);
        let mut file = open_owner_only_secret(&directory.secret()).unwrap();

        // Move the shared offset first: the read must not depend on it.
        let mut one = [0u8; 1];
        file.read_exact(&mut one).unwrap();
        assert_eq!(
            &read_owner_only_secret_from(&file, 32).unwrap()[..],
            &[7; 32]
        );
        // And it must not have moved it either.
        file.read_exact(&mut one).unwrap();
        assert_eq!(one, [7]);

        assert_eq!(
            error(read_owner_only_secret_from(&file, 31)),
            SecretFileError::InvalidLength
        );
        assert_eq!(
            error(read_owner_only_secret_from(&file, 33)),
            SecretFileError::InvalidLength
        );
        assert_eq!(
            error(read_owner_only_secret_from(&file, 0)),
            SecretFileError::InvalidLength
        );

        // The policy is checked again on the descriptor itself.
        fs::set_permissions(directory.secret(), fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            error(read_owner_only_secret_from(&file, 32)),
            SecretFileError::InsecurePermissions
        );
        // A directory is not a secret, however it was opened.
        let not_a_file = fs::File::open(&directory.0).unwrap();
        assert_eq!(
            error(read_owner_only_secret_from(&not_a_file, 32)),
            SecretFileError::UnsafeFileType
        );
    }

    #[cfg(unix)]
    #[test]
    fn opening_refuses_what_reading_refuses() {
        use std::os::unix::fs::PermissionsExt;
        let directory = TestDirectory::new("open-policy");
        directory.write_secret(&[7; 32]);
        fs::set_permissions(directory.secret(), fs::Permissions::from_mode(0o640)).unwrap();
        assert_eq!(
            open_owner_only_secret(&directory.secret()).unwrap_err(),
            SecretFileError::InsecurePermissions
        );
        assert_eq!(
            open_owner_only_secret(Path::new("relative/secret.bin")).unwrap_err(),
            SecretFileError::InvalidPath
        );
        // A FIFO is refused without blocking in open.
        let fifo = directory.0.join("fifo");
        let c_path = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        // SAFETY: a NUL-terminated path.
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
        assert_eq!(
            open_owner_only_secret(&fifo).unwrap_err(),
            SecretFileError::UnsafeFileType
        );
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

    /// Grant Everyone read on `path`, through its own DACL (Windows tests).
    #[cfg(windows)]
    fn grant_everyone(path: &Path) {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{LocalFree, ERROR_SUCCESS};
        use windows_sys::Win32::Security::Authorization::{
            GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
            GRANT_ACCESS, NO_MULTIPLE_TRUSTEE, SE_FILE_OBJECT, TRUSTEE_IS_SID,
            TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
        };
        use windows_sys::Win32::Security::{
            CreateWellKnownSid, WinWorldSid, ACL, DACL_SECURITY_INFORMATION, NO_INHERITANCE,
        };
        use windows_sys::Win32::Storage::FileSystem::FILE_GENERIC_READ;
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // u32 words: a SID is 4-byte aligned.
        let mut everyone = [0u32; 17];
        let mut size = std::mem::size_of_val(&everyone) as u32;
        // SAFETY: a buffer of `size` bytes for the SID; the old DACL and its
        // descriptor are written by the call and freed below; the new ACL
        // is freed after it is set.
        unsafe {
            assert_ne!(
                CreateWellKnownSid(
                    WinWorldSid,
                    std::ptr::null_mut(),
                    everyone.as_mut_ptr().cast(),
                    &mut size
                ),
                0
            );
            let mut old: *mut ACL = std::ptr::null_mut();
            let mut descriptor = std::ptr::null_mut();
            assert_eq!(
                GetNamedSecurityInfoW(
                    wide.as_ptr(),
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    &mut old,
                    std::ptr::null_mut(),
                    &mut descriptor,
                ),
                ERROR_SUCCESS
            );
            let access = EXPLICIT_ACCESS_W {
                grfAccessPermissions: FILE_GENERIC_READ,
                grfAccessMode: GRANT_ACCESS,
                grfInheritance: NO_INHERITANCE,
                Trustee: TRUSTEE_W {
                    pMultipleTrustee: std::ptr::null_mut(),
                    MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
                    ptstrName: everyone.as_mut_ptr().cast(),
                },
            };
            let mut new: *mut ACL = std::ptr::null_mut();
            assert_eq!(SetEntriesInAclW(1, &access, old, &mut new), ERROR_SUCCESS);
            let set = SetNamedSecurityInfoW(
                wide.as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                new,
                std::ptr::null(),
            );
            LocalFree(new as _);
            LocalFree(descriptor as _);
            assert_eq!(set, ERROR_SUCCESS);
        }
    }

    #[cfg(windows)]
    #[test]
    fn an_opened_secret_is_read_from_its_handle_at_offset_zero() {
        let directory = TestDirectory::new("handle");
        directory.write_secret(&[7; 32]);
        let file = open_owner_only_secret(&directory.secret()).unwrap();
        // Twice: each read starts at offset 0, whatever the last one left.
        for _ in 0..2 {
            assert_eq!(
                &read_owner_only_secret_from(&file, 32).unwrap()[..],
                &[7; 32]
            );
        }
        for length in [31, 33, 0] {
            assert_eq!(
                error(read_owner_only_secret_from(&file, length)),
                SecretFileError::InvalidLength,
                "{length}"
            );
        }
        // Shared read only: nothing can write the key while it is open.
        assert!(fs::OpenOptions::new()
            .write(true)
            .open(directory.secret())
            .is_err());
        // The policy is checked again on the handle itself.
        grant_everyone(&directory.secret());
        assert_eq!(
            error(read_owner_only_secret_from(&file, 32)),
            SecretFileError::InsecurePermissions
        );
        // A directory is not a secret, however it was opened.
        use std::os::windows::fs::OpenOptionsExt;
        let not_a_file = fs::OpenOptions::new()
            .read(true)
            .custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_BACKUP_SEMANTICS)
            .open(&directory.0)
            .unwrap();
        assert_eq!(
            error(read_owner_only_secret_from(&not_a_file, 32)),
            SecretFileError::UnsafeFileType
        );
    }

    #[cfg(windows)]
    #[test]
    fn opening_refuses_what_reading_refuses() {
        let directory = TestDirectory::new("open-policy");
        directory.write_secret(&[7; 32]);
        grant_everyone(&directory.secret());
        assert_eq!(
            open_owner_only_secret(&directory.secret()).unwrap_err(),
            SecretFileError::InsecurePermissions
        );
        assert_eq!(
            open_owner_only_secret(Path::new("relative\\secret.bin")).unwrap_err(),
            SecretFileError::InvalidPath
        );
        assert_eq!(
            open_owner_only_secret(&directory.0.join("missing.bin")).unwrap_err(),
            SecretFileError::AccessFailed
        );
    }

    #[cfg(windows)]
    #[test]
    fn only_a_directory_the_user_alone_can_use_passes() {
        let directory = TestDirectory::new("owner-only-dir");
        let keys = directory.0.join("keys");
        fs::create_dir(&keys).unwrap();
        // As created under the user's profile: the user, SYSTEM and
        // Administrators.
        assert_eq!(check_owner_only_directory(&keys), Ok(()));
        // A junction to it is not followed.
        let junction = directory.0.join("junction");
        let made = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&junction)
            .arg(&keys)
            .output()
            .unwrap();
        assert!(made.status.success(), "{made:?}");
        assert_eq!(
            check_owner_only_directory(&junction),
            Err(SecretFileError::UnsafeFileType)
        );
        // A file, a missing path, and a relative path.
        directory.write_secret(&[1; 32]);
        assert_eq!(
            check_owner_only_directory(&directory.secret()),
            Err(SecretFileError::UnsafeFileType)
        );
        assert_eq!(
            check_owner_only_directory(&directory.0.join("missing")),
            Err(SecretFileError::AccessFailed)
        );
        assert_eq!(
            check_owner_only_directory(Path::new("relative\\keys")),
            Err(SecretFileError::InvalidPath)
        );
        // Anyone else allowed in: refused.
        grant_everyone(&keys);
        assert_eq!(
            check_owner_only_directory(&keys),
            Err(SecretFileError::InsecurePermissions)
        );
    }
}
