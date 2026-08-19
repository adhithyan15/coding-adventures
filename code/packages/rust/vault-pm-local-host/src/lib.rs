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
const AGENT_SOCKET_FILE: &str = "agent.sock";
const MAX_PATH_BYTES: usize = 4096;
const MAX_CONFIG_BYTES: usize = 64 * 1024;

/// Fixed prefix on the runtime directory basename, so a stray directory in a
/// shared system temporary root is at least self-describing.
const RUNTIME_DIRECTORY_PREFIX: &str = "vault-pm-";

/// How many hex characters of the data-root fingerprint name the runtime
/// directory.
///
/// Eight bytes (sixteen hex characters) is not a security boundary — nothing
/// about the agent's confidentiality depends on this name being unguessable,
/// since [`PreparedLocalVault::ensure_runtime_root`] and every accepted
/// connection are authorized by owner-only permissions and peer credentials,
/// never by path secrecy. It exists only so two different data roots (two
/// vault-pm installations, or two sandboxed test homes) resolve to two
/// different runtime directories without coordinating, while keeping the
/// whole path short enough for `sockaddr_un.sun_path`, which POSIX bounds to
/// roughly 100 bytes on Linux and macOS alike — far less than the 4096-byte
/// ceiling every other path in this crate accepts. `application-state`
/// nested under a verbose platform data directory (for example macOS's
/// `~/Library/Application Support/...`) can already approach that limit by
/// itself, which is why the runtime root is resolved beside the system
/// temporary directory instead of beneath [`LocalVaultPaths::data_root`].
const RUNTIME_FINGERPRINT_HEX_CHARS: usize = 16;

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
    runtime_root: PathBuf,
    agent_socket_path: PathBuf,
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
        let runtime_root = runtime_root_for(&data_root);
        let agent_socket_path = runtime_root.join(AGENT_SOCKET_FILE);
        for path in [
            &application_state_root,
            &object_root,
            &lock_file,
            &config_file,
            &runtime_root,
            &agent_socket_path,
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
            runtime_root,
            agent_socket_path,
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

    /// Return the owner-private root for the local agent's runtime socket
    /// (VLT-PM00 §14.2, VLT-PM48).
    ///
    /// This is *not* a subdirectory of [`Self::data_root`]. See
    /// [`RUNTIME_FINGERPRINT_HEX_CHARS`] for why: a Unix domain socket path is
    /// bound by `sockaddr_un.sun_path`, roughly 100 bytes on Linux and macOS,
    /// and a verbose platform data directory can already consume most of that
    /// budget on its own. The directory is deterministic per data root — a
    /// short fingerprint of [`Self::data_root`], never a random or
    /// process-specific value — so repeated calls against the same
    /// installation (or the same sandboxed test home) always agree on where
    /// the socket lives, without persisting anything beside the filesystem
    /// path itself.
    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }

    /// Return the exact path the local agent binds its Unix domain socket at.
    ///
    /// Always `runtime_root().join("agent.sock")`. No Windows named-pipe path
    /// exists yet; Phase 1B's agent (VLT-PM48) is Unix-only, and Windows
    /// support is an explicitly deferred follow-up.
    pub fn agent_socket_path(&self) -> &Path {
        &self.agent_socket_path
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

    /// The runtime root is deliberately *not* one of these.
    ///
    /// [`Self::prepare`] verifies or creates every directory an ordinary
    /// one-shot command needs, and no ordinary command touches the agent.
    /// Creating a runtime directory on every invocation would mean every
    /// `vault-pm` process — including ones that will never run `agent start`
    /// — reaches into the system temporary root, which is exactly the kind of
    /// unconditional side effect this crate otherwise avoids. Instead
    /// [`PreparedLocalVault::ensure_runtime_root`] verifies or creates it
    /// lazily, the same way [`LocalWriterGuard`] is only reached by commands
    /// that actually need durable configuration.
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

    /// Verify or create the owner-private runtime directory, lazily.
    ///
    /// This is the local agent's own entry point into this crate: it is
    /// called only by `agent start` and by a client that is about to dial the
    /// socket, never by [`Self::try_acquire_writer`] or any ordinary
    /// authenticated command. The directory is created with the same
    /// owner-only permissions and the same refusal of symlinks and
    /// foreign-owned existing objects as every other private root this crate
    /// resolves — see [`LocalHostError::InsecureOwner`] and
    /// [`LocalHostError::UnsafeObjectType`].
    pub fn ensure_runtime_root(&self) -> Result<&Path, LocalHostError> {
        #[cfg(any(unix, windows))]
        {
            platform::ensure_private_runtime_directory(&self.paths.runtime_root)?;
            Ok(&self.paths.runtime_root)
        }
        #[cfg(not(any(unix, windows)))]
        {
            Err(LocalHostError::UnsupportedPlatform)
        }
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

/// Derive the short, deterministic runtime directory for one data root.
///
/// `std::env::temp_dir()` rather than [`ProjectDirs::runtime_dir`] on
/// purpose: the latter resolves from `XDG_RUNTIME_DIR` alone, a single
/// process-wide value that does not vary with the data root. Two
/// [`LocalVaultPaths`] built from two different data roots — two real
/// installations sharing a login session, or two sandboxed test homes run in
/// the same process — would otherwise resolve to the exact same socket path
/// and silently share one agent. Hashing the data root instead means every
/// distinct installation gets its own runtime directory using nothing but
/// the same roots this crate already receives, with no extra state to keep
/// in sync.
///
/// The fingerprint is not a secret and is not a security boundary — see
/// [`RUNTIME_FINGERPRINT_HEX_CHARS`]. It only has to avoid *accidental*
/// collision between unrelated data roots, which a truncated SHA-256 digest
/// does far past any realistic installation count.
fn runtime_root_for(data_root: &Path) -> PathBuf {
    let digest = coding_adventures_sha256::sha256_hex(data_root.as_os_str().as_encoded_bytes());
    std::env::temp_dir().join(format!(
        "{RUNTIME_DIRECTORY_PREFIX}{}",
        &digest[..RUNTIME_FINGERPRINT_HEX_CHARS]
    ))
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
    fn runtime_root_is_short_deterministic_and_never_under_data_root() {
        let first = TestDirectory::new("runtime-a").paths();
        let again =
            LocalVaultPaths::from_roots(first.config_root(), first.data_root(), first.cache_root())
                .unwrap();
        let second = TestDirectory::new("runtime-b").paths();

        // Deterministic: the same data root always resolves to the same
        // runtime root and socket path, across independently constructed
        // instances.
        assert_eq!(first.runtime_root(), again.runtime_root());
        assert_eq!(first.agent_socket_path(), again.agent_socket_path());

        // Distinct: two different data roots never collide, which is the
        // whole reason this is not a fixed XDG-only path.
        assert_ne!(first.runtime_root(), second.runtime_root());

        // Never nested under the (potentially verbose) data root.
        assert!(!first.runtime_root().starts_with(first.data_root()));
        assert_eq!(
            first.agent_socket_path(),
            first.runtime_root().join("agent.sock")
        );

        // Short enough to fit `sockaddr_un.sun_path` (~100 bytes on Linux and
        // macOS) even beside a realistically long system temp directory.
        assert!(
            first.agent_socket_path().as_os_str().len() < 100,
            "socket path {:?} is too long for sockaddr_un.sun_path",
            first.agent_socket_path()
        );
    }

    #[test]
    fn runtime_root_is_created_owner_only_and_reused() {
        let directory = TestDirectory::new("runtime-ensure");
        let prepared = directory.paths().prepare().unwrap();
        assert!(!prepared.paths().runtime_root().exists());

        let created = prepared.ensure_runtime_root().unwrap().to_path_buf();
        assert_eq!(created, prepared.paths().runtime_root());
        assert!(created.is_dir());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&created).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }

        // Idempotent: an already-verified directory is accepted again rather
        // than re-created or rejected.
        assert_eq!(prepared.ensure_runtime_root().unwrap(), created);
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
