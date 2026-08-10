//! Secure platform roots, atomic configuration, and process exclusion.

#![deny(missing_docs)]

use core::fmt::{self, Debug, Display, Formatter};
use directories::ProjectDirs;
use std::fs::{File, TryLockError};
use std::path::{Path, PathBuf};

#[cfg(unix)]
#[path = "unix.rs"]
mod platform;
#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

const APPLICATION_QUALIFIER: &str = "dev";
const APPLICATION_ORGANIZATION: &str = "Coding Adventures";
const APPLICATION_NAME: &str = "vault-pm";
const STATE_DIRECTORY: &str = "application-state";
const OBJECT_DIRECTORY: &str = "objects";
const LOCK_FILE: &str = ".writer.lock";
const CONFIG_FILE: &str = "vault-pm.toml";
const MAX_PATH_BYTES: usize = 4096;
const MAX_CONFIG_BYTES: usize = 64 * 1024;

/// Stable, path-free local-host failure taxonomy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalHostError {
    /// Standard user application directories could not be resolved.
    PlatformUnavailable,
    /// A caller-supplied or resolved path was not bounded and absolute.
    InvalidPath,
    /// A parent directory could not be opened without following links.
    ParentUnavailable,
    /// A directory or lock object could not be created, opened, or inspected.
    AccessFailed,
    /// A path resolved to a link, reparse point, or unexpected object type.
    UnsafeObjectType,
    /// An existing private root or lock file belongs to another user.
    InsecureOwner,
    /// An existing private root or lock file grants access beyond its owner.
    InsecurePermissions,
    /// Another process currently owns the vault writer lock.
    AlreadyLocked,
    /// A caller attempted to create configuration that already exists.
    ConfigAlreadyExists,
    /// Exact expected configuration did not match durable configuration.
    ConfigConflict,
    /// Configuration bytes were empty or exceeded the local V1 bound.
    InvalidConfigBytes,
    /// The current target has no audited local-host implementation.
    UnsupportedPlatform,
}

impl Display for LocalHostError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::PlatformUnavailable => "vault-pm local host: platform unavailable",
            Self::InvalidPath => "vault-pm local host: invalid path",
            Self::ParentUnavailable => "vault-pm local host: parent unavailable",
            Self::AccessFailed => "vault-pm local host: access failed",
            Self::UnsafeObjectType => "vault-pm local host: unsafe object type",
            Self::InsecureOwner => "vault-pm local host: insecure owner",
            Self::InsecurePermissions => "vault-pm local host: insecure permissions",
            Self::AlreadyLocked => "vault-pm local host: already locked",
            Self::ConfigAlreadyExists => "vault-pm local host: config already exists",
            Self::ConfigConflict => "vault-pm local host: config conflict",
            Self::InvalidConfigBytes => "vault-pm local host: invalid config bytes",
            Self::UnsupportedPlatform => "vault-pm local host: unsupported platform",
        })
    }
}

impl std::error::Error for LocalHostError {}

/// Platform-resolved local filesystem layout for one CLI installation.
#[derive(Clone, PartialEq, Eq)]
pub struct LocalVaultPaths {
    config_root: PathBuf,
    data_root: PathBuf,
    cache_root: PathBuf,
    application_state_root: PathBuf,
    object_root: PathBuf,
    lock_file: PathBuf,
    config_file: PathBuf,
}

impl LocalVaultPaths {
    /// Resolve the current user's standard application directories.
    pub fn resolve() -> Result<Self, LocalHostError> {
        let project = ProjectDirs::from(
            APPLICATION_QUALIFIER,
            APPLICATION_ORGANIZATION,
            APPLICATION_NAME,
        )
        .ok_or(LocalHostError::PlatformUnavailable)?;
        Self::from_roots(
            project.config_dir(),
            project.data_local_dir(),
            project.cache_dir(),
        )
    }

    /// Construct a validated layout from injected roots.
    ///
    /// This is the test and explicit-override seam. Every root must be an
    /// absolute, bounded path. No filesystem access occurs until [`Self::prepare`].
    pub fn from_roots(
        config_root: impl Into<PathBuf>,
        data_root: impl Into<PathBuf>,
        cache_root: impl Into<PathBuf>,
    ) -> Result<Self, LocalHostError> {
        let config_root = config_root.into();
        let data_root = data_root.into();
        let cache_root = cache_root.into();
        for path in [&config_root, &data_root, &cache_root] {
            validate_path(path)?;
        }
        let application_state_root = data_root.join(STATE_DIRECTORY);
        let object_root = data_root.join(OBJECT_DIRECTORY);
        let lock_file = data_root.join(LOCK_FILE);
        let config_file = config_root.join(CONFIG_FILE);
        for path in [
            &application_state_root,
            &object_root,
            &lock_file,
            &config_file,
        ] {
            validate_path(path)?;
        }
        Ok(Self {
            config_root,
            data_root,
            cache_root,
            application_state_root,
            object_root,
            lock_file,
            config_file,
        })
    }

    /// Return the root for non-secret CLI configuration.
    pub fn config_root(&self) -> &Path {
        &self.config_root
    }

    /// Return the owner-private root containing all durable local data.
    pub fn data_root(&self) -> &Path {
        &self.data_root
    }

    /// Return the safely disposable cache root.
    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    /// Return the root for bootstrap generations and owner-private state.
    pub fn application_state_root(&self) -> &Path {
        &self.application_state_root
    }

    /// Return the root for encrypted immutable repository objects.
    pub fn object_root(&self) -> &Path {
        &self.object_root
    }

    /// Create or verify the complete local layout without acquiring the writer lock.
    pub fn prepare(&self) -> Result<PreparedLocalVault, LocalHostError> {
        #[cfg(any(unix, windows))]
        {
            for path in self.unique_directories() {
                platform::ensure_private_directory(path)?;
            }
            Ok(PreparedLocalVault {
                paths: self.clone(),
            })
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(LocalHostError::UnsupportedPlatform)
        }
    }

    fn unique_directories(&self) -> Vec<&Path> {
        let mut paths = vec![
            self.config_root(),
            self.data_root(),
            self.cache_root(),
            self.application_state_root(),
            self.object_root(),
        ];
        paths.sort_unstable();
        paths.dedup();
        paths
    }
}

impl Debug for LocalVaultPaths {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalVaultPaths")
            .field("roots", &"<redacted>")
            .finish()
    }
}

/// A local layout whose directories passed native owner-only checks.
pub struct PreparedLocalVault {
    paths: LocalVaultPaths,
}

impl PreparedLocalVault {
    /// Borrow the verified filesystem layout for adapter construction.
    pub const fn paths(&self) -> &LocalVaultPaths {
        &self.paths
    }

    /// Try to acquire exclusive local-process access without waiting.
    pub fn try_acquire_writer(&self) -> Result<LocalWriterGuard, LocalHostError> {
        #[cfg(any(unix, windows))]
        {
            let file = platform::open_private_lock(&self.paths.lock_file)?;
            match file.try_lock() {
                Ok(()) => Ok(LocalWriterGuard {
                    _file: file,
                    config_file: self.paths.config_file.clone(),
                }),
                Err(error) => Err(map_lock_error(error)),
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(LocalHostError::UnsupportedPlatform)
        }
    }
}

fn map_lock_error(error: TryLockError) -> LocalHostError {
    match error {
        TryLockError::WouldBlock => LocalHostError::AlreadyLocked,
        TryLockError::Error(_) => LocalHostError::AccessFailed,
    }
}

impl Debug for PreparedLocalVault {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedLocalVault")
            .field("paths", &"<redacted>")
            .finish()
    }
}

/// RAII ownership of the exclusive local writer lock.
pub struct LocalWriterGuard {
    _file: File,
    config_file: PathBuf,
}

impl LocalWriterGuard {
    /// Load exact durable configuration bytes, or `None` when uninitialized.
    ///
    /// The read is capped at 65,537 bytes and rejects links, non-regular files,
    /// foreign ownership, and broad permissions before returning bytes.
    pub fn load_config(&self) -> Result<Option<Vec<u8>>, LocalHostError> {
        #[cfg(any(unix, windows))]
        {
            platform::load_private_config(&self.config_file, MAX_CONFIG_BYTES)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(LocalHostError::UnsupportedPlatform)
        }
    }

    /// Atomically create first configuration without replacing an existing file.
    pub fn create_config(&self, bytes: &[u8]) -> Result<(), LocalHostError> {
        validate_config_bytes(bytes)?;
        #[cfg(any(unix, windows))]
        {
            platform::create_private_config(&self.config_file, bytes, MAX_CONFIG_BYTES)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(LocalHostError::UnsupportedPlatform)
        }
    }

    /// Atomically replace configuration only when exact durable bytes match.
    pub fn compare_exchange_config(
        &self,
        expected: &[u8],
        replacement: &[u8],
    ) -> Result<(), LocalHostError> {
        validate_config_bytes(expected)?;
        validate_config_bytes(replacement)?;
        #[cfg(any(unix, windows))]
        {
            platform::compare_exchange_private_config(
                &self.config_file,
                expected,
                replacement,
                MAX_CONFIG_BYTES,
            )
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(LocalHostError::UnsupportedPlatform)
        }
    }
}

impl Debug for LocalWriterGuard {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalWriterGuard")
            .finish_non_exhaustive()
    }
}

fn validate_path(path: &Path) -> Result<(), LocalHostError> {
    if !path.is_absolute()
        || path.as_os_str().is_empty()
        || path.as_os_str().as_encoded_bytes().len() > MAX_PATH_BYTES
        || path.as_os_str().as_encoded_bytes().contains(&0)
        || path.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        return Err(LocalHostError::InvalidPath);
    }
    Ok(())
}

fn validate_config_bytes(bytes: &[u8]) -> Result<(), LocalHostError> {
    if bytes.is_empty() || bytes.len() > MAX_CONFIG_BYTES {
        Err(LocalHostError::InvalidConfigBytes)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "vault-pm-local-host-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&root).unwrap();
            Self(fs::canonicalize(root).unwrap())
        }

        fn paths(&self) -> LocalVaultPaths {
            LocalVaultPaths::from_roots(
                self.0.join("config"),
                self.0.join("data"),
                self.0.join("cache"),
            )
            .unwrap()
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolves_absolute_platform_paths_without_touching_them() {
        let paths = LocalVaultPaths::resolve().unwrap();
        for path in [
            paths.config_root(),
            paths.data_root(),
            paths.cache_root(),
            paths.application_state_root(),
            paths.object_root(),
        ] {
            assert!(path.is_absolute());
        }
    }

    #[test]
    fn prepares_separate_application_and_object_roots() {
        let directory = TestDirectory::new("layout");
        let paths = directory.paths();
        let prepared = paths.prepare().unwrap();
        for path in [
            paths.config_root(),
            paths.data_root(),
            paths.cache_root(),
            paths.application_state_root(),
            paths.object_root(),
        ] {
            assert!(path.is_dir());
        }
        assert_eq!(prepared.paths(), &paths);
        assert_ne!(paths.application_state_root(), paths.object_root());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in [
                paths.config_root(),
                paths.data_root(),
                paths.cache_root(),
                paths.application_state_root(),
                paths.object_root(),
            ] {
                assert_eq!(
                    fs::metadata(path).unwrap().permissions().mode() & 0o777,
                    0o700
                );
            }
        }
    }

    #[test]
    fn writer_lock_excludes_then_releases_other_openers() {
        let directory = TestDirectory::new("lock");
        let first = directory.paths().prepare().unwrap();
        let second = directory.paths().prepare().unwrap();
        let guard = first.try_acquire_writer().unwrap();
        assert!(format!("{guard:?}").starts_with("LocalWriterGuard"));
        assert!(matches!(
            second.try_acquire_writer(),
            Err(LocalHostError::AlreadyLocked)
        ));
        drop(guard);
        second.try_acquire_writer().unwrap();
        assert!(second.paths.lock_file.is_file());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&second.paths.lock_file)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn configuration_create_load_and_compare_exchange_are_exact_and_durable() {
        let directory = TestDirectory::new("config-roundtrip");
        let paths = directory.paths();
        let prepared = paths.prepare().unwrap();
        let guard = prepared.try_acquire_writer().unwrap();
        let initial = b"version = 1\nactive_vault = \"personal\"\n";
        let replacement = b"version = 1\nactive_vault = \"work\"\n";

        assert_eq!(guard.load_config().unwrap(), None);
        guard.create_config(initial).unwrap();
        assert_eq!(guard.load_config().unwrap().as_deref(), Some(&initial[..]));
        assert_eq!(
            guard.create_config(b"do not replace").unwrap_err(),
            LocalHostError::ConfigAlreadyExists
        );
        assert_eq!(guard.load_config().unwrap().as_deref(), Some(&initial[..]));

        guard.compare_exchange_config(initial, replacement).unwrap();
        assert_eq!(
            guard.load_config().unwrap().as_deref(),
            Some(&replacement[..])
        );
        assert_eq!(
            guard
                .compare_exchange_config(initial, b"stale replacement")
                .unwrap_err(),
            LocalHostError::ConfigConflict
        );
        assert_eq!(
            guard.load_config().unwrap().as_deref(),
            Some(&replacement[..])
        );
        assert!(
            !fs::read_dir(paths.config_root()).unwrap().any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".vault-pm.toml.tmp."))
        );

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&paths.config_file)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        drop(guard);
        let reopened = prepared.try_acquire_writer().unwrap();
        assert_eq!(
            reopened.load_config().unwrap().as_deref(),
            Some(&replacement[..])
        );
    }

    #[test]
    fn configuration_rejects_unbounded_or_missing_compare_exchange_inputs() {
        let directory = TestDirectory::new("config-bounds");
        let prepared = directory.paths().prepare().unwrap();
        let guard = prepared.try_acquire_writer().unwrap();
        let oversized = vec![b'x'; MAX_CONFIG_BYTES + 1];

        assert_eq!(
            guard.create_config(b"").unwrap_err(),
            LocalHostError::InvalidConfigBytes
        );
        assert_eq!(
            guard.create_config(&oversized).unwrap_err(),
            LocalHostError::InvalidConfigBytes
        );
        assert_eq!(
            guard
                .compare_exchange_config(b"expected", b"replacement")
                .unwrap_err(),
            LocalHostError::ConfigConflict
        );
        assert_eq!(
            guard
                .compare_exchange_config(b"", b"replacement")
                .unwrap_err(),
            LocalHostError::InvalidConfigBytes
        );
        assert_eq!(
            guard.compare_exchange_config(b"expected", b"").unwrap_err(),
            LocalHostError::InvalidConfigBytes
        );
        assert_eq!(guard.load_config().unwrap(), None);
        let maximum = vec![b'x'; MAX_CONFIG_BYTES];
        guard.create_config(&maximum).unwrap();
        assert_eq!(guard.load_config().unwrap(), Some(maximum));
    }

    #[test]
    fn invalid_paths_and_diagnostics_are_closed() {
        assert_eq!(
            LocalVaultPaths::from_roots("relative", "/absolute/data", "/absolute/cache"),
            Err(LocalHostError::InvalidPath)
        );
        let too_long = PathBuf::from(format!("/{}", "x".repeat(MAX_PATH_BYTES)));
        assert_eq!(
            LocalVaultPaths::from_roots(&too_long, "/absolute/data", "/absolute/cache"),
            Err(LocalHostError::InvalidPath)
        );
        let directory = TestDirectory::new("redaction");
        let paths = directory.paths();
        let rendered = format!("{paths:?}");
        assert!(!rendered.contains(directory.0.to_string_lossy().as_ref()));
        let prepared = paths.prepare().unwrap();
        assert!(!format!("{prepared:?}").contains(directory.0.to_string_lossy().as_ref()));

        let expected = [
            (
                LocalHostError::PlatformUnavailable,
                "vault-pm local host: platform unavailable",
            ),
            (
                LocalHostError::InvalidPath,
                "vault-pm local host: invalid path",
            ),
            (
                LocalHostError::ParentUnavailable,
                "vault-pm local host: parent unavailable",
            ),
            (
                LocalHostError::AccessFailed,
                "vault-pm local host: access failed",
            ),
            (
                LocalHostError::UnsafeObjectType,
                "vault-pm local host: unsafe object type",
            ),
            (
                LocalHostError::InsecureOwner,
                "vault-pm local host: insecure owner",
            ),
            (
                LocalHostError::InsecurePermissions,
                "vault-pm local host: insecure permissions",
            ),
            (
                LocalHostError::AlreadyLocked,
                "vault-pm local host: already locked",
            ),
            (
                LocalHostError::ConfigAlreadyExists,
                "vault-pm local host: config already exists",
            ),
            (
                LocalHostError::ConfigConflict,
                "vault-pm local host: config conflict",
            ),
            (
                LocalHostError::InvalidConfigBytes,
                "vault-pm local host: invalid config bytes",
            ),
            (
                LocalHostError::UnsupportedPlatform,
                "vault-pm local host: unsupported platform",
            ),
        ];
        for (error, message) in expected {
            assert_eq!(error.to_string(), message);
        }
        assert_eq!(
            map_lock_error(TryLockError::Error(std::io::Error::other("test"))),
            LocalHostError::AccessFailed
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_existing_links_and_broad_private_roots() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let directory = TestDirectory::new("unix-policy");
        let paths = directory.paths();
        fs::create_dir(paths.config_root()).unwrap();
        fs::set_permissions(paths.config_root(), fs::Permissions::from_mode(0o750)).unwrap();
        assert_eq!(
            paths.prepare().unwrap_err(),
            LocalHostError::InsecurePermissions
        );

        fs::remove_dir(paths.config_root()).unwrap();
        let target = directory.0.join("target");
        fs::create_dir(&target).unwrap();
        symlink(&target, paths.config_root()).unwrap();
        assert_eq!(
            paths.prepare().unwrap_err(),
            LocalHostError::UnsafeObjectType
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_unsafe_existing_lock_objects_without_replacing_them() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let broad = TestDirectory::new("broad-lock");
        let broad_prepared = broad.paths().prepare().unwrap();
        fs::write(&broad_prepared.paths.lock_file, b"preserve").unwrap();
        fs::set_permissions(
            &broad_prepared.paths.lock_file,
            fs::Permissions::from_mode(0o640),
        )
        .unwrap();
        assert_eq!(
            broad_prepared.try_acquire_writer().unwrap_err(),
            LocalHostError::InsecurePermissions
        );
        assert_eq!(
            fs::read(&broad_prepared.paths.lock_file).unwrap(),
            b"preserve"
        );

        let linked = TestDirectory::new("linked-lock");
        let linked_prepared = linked.paths().prepare().unwrap();
        let target = linked.0.join("target.lock");
        fs::write(&target, b"target").unwrap();
        symlink(&target, &linked_prepared.paths.lock_file).unwrap();
        assert_eq!(
            linked_prepared.try_acquire_writer().unwrap_err(),
            LocalHostError::UnsafeObjectType
        );
        assert_eq!(fs::read(target).unwrap(), b"target");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_unsafe_existing_configuration_without_following_or_replacing_it() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let linked = TestDirectory::new("linked-config");
        let linked_paths = linked.paths();
        let linked_prepared = linked_paths.prepare().unwrap();
        let linked_guard = linked_prepared.try_acquire_writer().unwrap();
        let target = linked.0.join("outside-config");
        fs::write(&target, b"preserve target").unwrap();
        symlink(&target, &linked_paths.config_file).unwrap();
        assert_eq!(
            linked_guard.load_config().unwrap_err(),
            LocalHostError::UnsafeObjectType
        );
        assert_eq!(
            linked_guard.create_config(b"replacement").unwrap_err(),
            LocalHostError::UnsafeObjectType
        );
        assert_eq!(fs::read(&target).unwrap(), b"preserve target");

        let broad = TestDirectory::new("broad-config");
        let broad_paths = broad.paths();
        let broad_prepared = broad_paths.prepare().unwrap();
        let broad_guard = broad_prepared.try_acquire_writer().unwrap();
        fs::write(&broad_paths.config_file, b"preserve config").unwrap();
        fs::set_permissions(&broad_paths.config_file, fs::Permissions::from_mode(0o640)).unwrap();
        assert_eq!(
            broad_guard.load_config().unwrap_err(),
            LocalHostError::InsecurePermissions
        );
        assert_eq!(
            broad_guard.create_config(b"replacement").unwrap_err(),
            LocalHostError::InsecurePermissions
        );
        assert_eq!(
            fs::read(&broad_paths.config_file).unwrap(),
            b"preserve config"
        );

        let invalid = TestDirectory::new("invalid-config-bytes");
        let invalid_paths = invalid.paths();
        let invalid_prepared = invalid_paths.prepare().unwrap();
        let invalid_guard = invalid_prepared.try_acquire_writer().unwrap();
        fs::write(&invalid_paths.config_file, b"").unwrap();
        fs::set_permissions(
            &invalid_paths.config_file,
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        assert_eq!(
            invalid_guard.load_config().unwrap_err(),
            LocalHostError::InvalidConfigBytes
        );
        fs::write(&invalid_paths.config_file, vec![b'x'; MAX_CONFIG_BYTES + 1]).unwrap();
        assert_eq!(
            invalid_guard.load_config().unwrap_err(),
            LocalHostError::InvalidConfigBytes
        );

        let typed = TestDirectory::new("typed-config");
        let typed_paths = typed.paths();
        let typed_prepared = typed_paths.prepare().unwrap();
        let typed_guard = typed_prepared.try_acquire_writer().unwrap();
        fs::create_dir(&typed_paths.config_file).unwrap();
        assert_eq!(
            typed_guard.load_config().unwrap_err(),
            LocalHostError::UnsafeObjectType
        );
    }
}
