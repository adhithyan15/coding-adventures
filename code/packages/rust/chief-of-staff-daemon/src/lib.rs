//! Concrete composition root for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_daemon_api::{BindAddress, DaemonApi, DaemonApiError};
use chief_of_staff_daemon_config::{parse_config, ChiefConfig, ConfigError};
use chief_of_staff_daemon_credential::{load_or_create_credential, CredentialFileError};
use chief_of_staff_daemon_keyring::{load_package_keyring, KeyringLoadError};
use chief_of_staff_daemon_policy::{DenyChannelWiring, LocalAuthError, LocalBearerAuthorizer};
use chief_of_staff_daemon_runtime::{ChiefDaemonRuntime, DaemonRuntimeError, ReconcileSchedule};
use chief_of_staff_host_data_plane::{
    DurableHostDataPlaneDispatcher, UnavailableHostDataPlaneService,
};
use chief_of_staff_orchestrator_core::OrchestratorCore;
use chief_of_staff_process_supervisor::{
    DurableHostLaunchBindings, HostProgram, ProcessSupervisorConfig, ProcessSupervisorError,
    SystemMonotonicClock, UuidV7SessionIdSource,
};
use chief_of_staff_service_reconciler::{ConfigError as ReconcileConfigError, ReconcileConfig};
use coding_adventures_storage_fs::FsStorageBackend;
use coding_adventures_x3dh::generate_identity_keypair;
use process_shutdown::{ShutdownError, ShutdownListener};
use std::env;
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use storage_core::{StorageBackend, StorageError};
use transport_platform::{PlatformError, TransportPlatform};
use websocket_runtime::WebSocketServerOptions;

const MAX_CONFIG_BYTES: usize = 256 * 1024;
const DEFAULT_CONFIG_SUFFIX: &str = ".chief-of-staff/config.toml";
const HEARTBEAT_GRACE_INTERVALS: u64 = 3;

/// Stable payload-blind startup, serving, and teardown failure.
#[derive(Debug)]
pub enum ChiefDaemonError {
    /// More than one config argument was supplied or a path was unsafe.
    InvalidInvocation,
    /// The platform-specific home environment variable was absent or unsafe.
    HomeUnavailable,
    /// The config path could not be inspected, opened, or read.
    ConfigFileUnavailable,
    /// The config path was a symlink or was not a regular file.
    ConfigFileNotRegular,
    /// The config path stopped naming the opened file during validation.
    ConfigFileChanged,
    /// The config file exceeded the parser's documented bound.
    ConfigFileTooLarge,
    /// The config file was not valid UTF-8.
    ConfigFileEncoding,
    /// Strict typed TOML validation failed.
    Config(ConfigError),
    /// A configured trusted package key could not be loaded.
    Keyring(KeyringLoadError),
    /// The local operator credential could not be loaded or created safely.
    Credential(CredentialFileError),
    /// Local bearer policy construction failed.
    Authentication(LocalAuthError),
    /// Durable registry storage initialization failed.
    Storage(StorageError),
    /// Host process supervision configuration was invalid.
    Process(ProcessSupervisorError),
    /// Reconciliation configuration was invalid.
    Reconciliation(ReconcileConfigError),
    /// The host transport provider could not initialize.
    Platform(PlatformError),
    /// The authenticated WebSocket runtime failed.
    Runtime(DaemonRuntimeError),
    /// Native cooperative shutdown installation or restoration failed.
    Shutdown(ShutdownError),
    /// Runtime teardown did not release the sole control-plane reference.
    ControlPlaneRetained,
    /// The control-plane mutex could not be recovered during teardown.
    ControlPlaneRecovery(DaemonApiError),
    /// This target has no supported production transport provider.
    UnsupportedPlatform,
}

impl Display for ChiefDaemonError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInvocation => "chief daemon: invalid invocation",
            Self::HomeUnavailable => "chief daemon: home directory unavailable",
            Self::ConfigFileUnavailable => "chief daemon: config file unavailable",
            Self::ConfigFileNotRegular => "chief daemon: config path is not a regular file",
            Self::ConfigFileChanged => "chief daemon: config path changed during validation",
            Self::ConfigFileTooLarge => "chief daemon: config file exceeds size limit",
            Self::ConfigFileEncoding => "chief daemon: config file is not UTF-8",
            Self::Config(_) => "chief daemon: config validation failed",
            Self::Keyring(_) => "chief daemon: package keyring failed",
            Self::Credential(_) => "chief daemon: operator credential failed",
            Self::Authentication(_) => "chief daemon: local authentication policy failed",
            Self::Storage(_) => "chief daemon: durable storage failed",
            Self::Process(_) => "chief daemon: process supervision failed",
            Self::Reconciliation(_) => "chief daemon: reconciliation configuration failed",
            Self::Platform(_) => "chief daemon: transport provider failed",
            Self::Runtime(_) => "chief daemon: runtime failed",
            Self::Shutdown(_) => "chief daemon: shutdown listener failed",
            Self::ControlPlaneRetained => "chief daemon: control plane remained retained",
            Self::ControlPlaneRecovery(_) => "chief daemon: control plane recovery failed",
            Self::UnsupportedPlatform => "chief daemon: unsupported platform",
        })
    }
}

impl std::error::Error for ChiefDaemonError {}

/// Resolved absolute startup paths independent of process-global environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StartupPaths {
    home: PathBuf,
    config: PathBuf,
}

impl StartupPaths {
    /// Return the validated absolute home directory.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// Return the validated absolute config-file path.
    pub fn config(&self) -> &Path {
        &self.config
    }
}

/// Resolve zero or one config argument against an explicit home value.
pub fn resolve_startup_paths<I>(
    args: I,
    home: Option<OsString>,
) -> Result<StartupPaths, ChiefDaemonError>
where
    I: IntoIterator<Item = OsString>,
{
    let home = home
        .map(PathBuf::from)
        .ok_or(ChiefDaemonError::HomeUnavailable)?;
    if !is_safe_absolute(&home) {
        return Err(ChiefDaemonError::HomeUnavailable);
    }
    let mut args = args.into_iter();
    let config = match args.next() {
        Some(path) => PathBuf::from(path),
        None => home.join(DEFAULT_CONFIG_SUFFIX),
    };
    if args.next().is_some() || !is_safe_absolute(&config) {
        return Err(ChiefDaemonError::InvalidInvocation);
    }
    Ok(StartupPaths { home, config })
}

/// Load one bounded, stable, regular config file and apply the strict schema.
pub fn load_config_file(path: &Path) -> Result<ChiefConfig, ChiefDaemonError> {
    if !is_safe_absolute(path) {
        return Err(ChiefDaemonError::InvalidInvocation);
    }
    let before = fs::symlink_metadata(path).map_err(|_| ChiefDaemonError::ConfigFileUnavailable)?;
    if !before.file_type().is_file() {
        return Err(ChiefDaemonError::ConfigFileNotRegular);
    }
    let file = open_readonly(path)?;
    let opened = file
        .metadata()
        .map_err(|_| ChiefDaemonError::ConfigFileUnavailable)?;
    let after = fs::symlink_metadata(path).map_err(|_| ChiefDaemonError::ConfigFileUnavailable)?;
    if !opened.file_type().is_file() || !after.file_type().is_file() {
        return Err(ChiefDaemonError::ConfigFileNotRegular);
    }
    if !same_file(&before, &opened) || !same_file(&after, &opened) {
        return Err(ChiefDaemonError::ConfigFileChanged);
    }
    let mut bytes = Vec::with_capacity(MAX_CONFIG_BYTES + 1);
    file.take((MAX_CONFIG_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ChiefDaemonError::ConfigFileUnavailable)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err(ChiefDaemonError::ConfigFileTooLarge);
    }
    let source = std::str::from_utf8(&bytes).map_err(|_| ChiefDaemonError::ConfigFileEncoding)?;
    parse_config(source).map_err(ChiefDaemonError::Config)
}

#[cfg(not(windows))]
fn open_readonly(path: &Path) -> Result<File, ChiefDaemonError> {
    File::open(path).map_err(|_| ChiefDaemonError::ConfigFileUnavailable)
}

#[cfg(windows)]
fn open_readonly(path: &Path) -> Result<File, ChiefDaemonError> {
    use std::os::windows::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ)
        .open(path)
        .map_err(|_| ChiefDaemonError::ConfigFileUnavailable)
}

/// Resolve process environment and run the daemon until shutdown or failure.
pub fn run_from_env() -> Result<(), ChiefDaemonError> {
    let home = platform_home();
    let paths = resolve_startup_paths(env::args_os().skip(1), home)?;
    let config = load_config_file(paths.config())?;
    run(config, paths.home())
}

/// Compose all production adapters from a validated config and explicit home.
pub fn run(config: ChiefConfig, home: &Path) -> Result<(), ChiefDaemonError> {
    if !is_safe_absolute(home) {
        return Err(ChiefDaemonError::HomeUnavailable);
    }
    let state_dir = config
        .orchestrator()
        .state_dir()
        .resolve(home)
        .map_err(ChiefDaemonError::Config)?;
    let credential_path = config
        .orchestrator()
        .credential_path()
        .resolve(home)
        .map_err(ChiefDaemonError::Config)?;
    let host_executable = config
        .host_defaults()
        .executable()
        .resolve(home)
        .map_err(ChiefDaemonError::Config)?;

    let keyring =
        Arc::new(load_package_keyring(config.keyring(), home).map_err(ChiefDaemonError::Keyring)?);
    let credential =
        load_or_create_credential(&credential_path).map_err(ChiefDaemonError::Credential)?;
    let bearer =
        LocalBearerAuthorizer::new(&credential).map_err(ChiefDaemonError::Authentication)?;
    drop(credential);

    let backend: Arc<dyn StorageBackend> = Arc::new(FsStorageBackend::new(state_dir));
    backend.initialize().map_err(ChiefDaemonError::Storage)?;
    let program = HostProgram::new(host_executable, std::iter::empty::<OsString>())
        .map_err(ChiefDaemonError::Process)?;
    let process_config = ProcessSupervisorConfig::new(
        program,
        config.host_defaults().bootstrap_timeout(),
        config.host_defaults().graceful_stop_timeout(),
    )
    .map_err(ChiefDaemonError::Process)?;
    let interval = config.host_defaults().health_check_interval();
    let interval_ns = u64::try_from(interval.as_nanos()).unwrap_or(u64::MAX);
    let reconcile_config =
        ReconcileConfig::new(interval_ns.saturating_mul(HEARTBEAT_GRACE_INTERVALS))
            .map_err(ChiefDaemonError::Reconciliation)?;
    let schedule = ReconcileSchedule::new(interval).map_err(ChiefDaemonError::Runtime)?;
    let clock = Arc::new(SystemMonotonicClock::new());
    let launch_bindings = Arc::new(DurableHostLaunchBindings::new(Arc::clone(&backend)));
    let data_plane = Arc::new(DurableHostDataPlaneDispatcher::new(
        Arc::clone(&backend),
        Arc::new(UnavailableHostDataPlaneService),
    ));
    let core = OrchestratorCore::with_process_supervisor(
        backend,
        process_config,
        keyring,
        launch_bindings,
        data_plane,
        Arc::new(generate_identity_keypair()),
        clock,
        Box::new(UuidV7SessionIdSource),
        reconcile_config,
        DenyChannelWiring,
    );
    let api = Arc::new(DaemonApi::new(core, bearer));
    let address = BindAddress::Ip(SocketAddr::new(
        config.orchestrator().bind(),
        config.orchestrator().port(),
    ));
    run_platform(address, api, schedule)
}

fn serve<P, C, A>(
    platform: P,
    address: BindAddress,
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
) -> Result<(), ChiefDaemonError>
where
    P: TransportPlatform,
    C: chief_of_staff_daemon_api::ChiefControlPlane + Send + 'static,
    A: chief_of_staff_daemon_api::SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    let mut runtime = ChiefDaemonRuntime::bind(
        platform,
        address,
        WebSocketServerOptions::default(),
        Arc::clone(&api),
        schedule,
    )
    .map_err(ChiefDaemonError::Runtime)?;
    let stop = runtime.stop_handle();
    let listener =
        ShutdownListener::install(move |_| stop.stop()).map_err(ChiefDaemonError::Shutdown)?;
    eprintln!("chief daemon listening on {}", runtime.local_addr());
    let runtime_result = runtime.serve();
    let shutdown_result = listener.uninstall();
    drop(runtime);
    let recovery_result = Arc::try_unwrap(api)
        .map_err(|_| ChiefDaemonError::ControlPlaneRetained)
        .and_then(|api| {
            api.into_parts()
                .map(|_| ())
                .map_err(ChiefDaemonError::ControlPlaneRecovery)
        });
    runtime_result.map_err(ChiefDaemonError::Runtime)?;
    shutdown_result.map_err(ChiefDaemonError::Shutdown)?;
    recovery_result
}

#[cfg(target_os = "linux")]
fn run_platform<C, A>(
    address: BindAddress,
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
) -> Result<(), ChiefDaemonError>
where
    C: chief_of_staff_daemon_api::ChiefControlPlane + Send + 'static,
    A: chief_of_staff_daemon_api::SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    let platform = transport_platform::linux::EpollTransportPlatform::new()
        .map_err(ChiefDaemonError::Platform)?;
    serve(platform, address, api, schedule)
}

#[cfg(any(
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly"
))]
fn run_platform<C, A>(
    address: BindAddress,
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
) -> Result<(), ChiefDaemonError>
where
    C: chief_of_staff_daemon_api::ChiefControlPlane + Send + 'static,
    A: chief_of_staff_daemon_api::SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    let platform = transport_platform::bsd::KqueueTransportPlatform::new()
        .map_err(ChiefDaemonError::Platform)?;
    serve(platform, address, api, schedule)
}

#[cfg(target_os = "windows")]
fn run_platform<C, A>(
    address: BindAddress,
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
) -> Result<(), ChiefDaemonError>
where
    C: chief_of_staff_daemon_api::ChiefControlPlane + Send + 'static,
    A: chief_of_staff_daemon_api::SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    let platform = transport_platform::windows::WindowsTransportPlatform::new()
        .map_err(ChiefDaemonError::Platform)?;
    serve(platform, address, api, schedule)
}

#[cfg(not(any(
    target_os = "linux",
    target_os = "macos",
    target_os = "freebsd",
    target_os = "openbsd",
    target_os = "netbsd",
    target_os = "dragonfly",
    target_os = "windows"
)))]
fn run_platform<C, A>(
    _address: BindAddress,
    _api: Arc<DaemonApi<C, A>>,
    _schedule: ReconcileSchedule,
) -> Result<(), ChiefDaemonError> {
    Err(ChiefDaemonError::UnsupportedPlatform)
}

#[cfg(unix)]
fn platform_home() -> Option<OsString> {
    env::var_os("HOME")
}

#[cfg(windows)]
fn platform_home() -> Option<OsString> {
    env::var_os("USERPROFILE")
}

#[cfg(not(any(unix, windows)))]
fn platform_home() -> Option<OsString> {
    None
}

fn is_safe_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

#[cfg(unix)]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(windows)]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    left.file_attributes() == right.file_attributes()
        && left.creation_time() == right.creation_time()
        && left.last_write_time() == right.last_write_time()
        && left.file_size() == right.file_size()
}

#[cfg(not(any(unix, windows)))]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    left.len() == right.len()
        && left.modified().ok() == right.modified().ok()
        && left.created().ok() == right.created().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    const VALID_CONFIG: &str = r#"
[orchestrator]
bind = "127.0.0.1"
port = 7463
packages_dir = "~/.chief-of-staff/agents/"
state_dir = "~/.chief-of-staff/state/"
credential_path = "~/.chief-of-staff/run/operator.credential"

[keyring]
trusted_keys = [
  { id = "prod-001", path = "~/.chief-of-staff/keys/prod.pub", type = "production" },
]

[hosts.defaults]
restart_policy = "on-failure"
health_check_interval = 5000
executable = "~/.chief-of-staff/bin/chief-of-staff-host"
bootstrap_timeout = 10000
graceful_stop_timeout = 5000

[vault]
storage_path = "~/.chief-of-staff/vault/"
default_lease_ttl = 30
container = true

[privilege]
tier_1_auto_approve_timeout = 5
biometric_timeout = 30
hardware_key_timeout = 60
"#;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = env::temp_dir().join(format!(
                "chief-daemon-app-{}-{}",
                std::process::id(),
                NEXT_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn resolves_default_and_explicit_absolute_config_paths() {
        let directory = TestDir::new();
        let default = resolve_startup_paths(
            Vec::<OsString>::new(),
            Some(directory.0.clone().into_os_string()),
        )
        .unwrap();
        assert_eq!(default.home(), directory.0);
        assert_eq!(default.config(), directory.0.join(DEFAULT_CONFIG_SUFFIX));

        let explicit_path = directory.0.join("custom.toml");
        let explicit = resolve_startup_paths(
            vec![explicit_path.clone().into_os_string()],
            Some(directory.0.clone().into_os_string()),
        )
        .unwrap();
        assert_eq!(explicit.config(), explicit_path);
    }

    #[test]
    fn rejects_missing_home_relative_paths_and_extra_arguments() {
        assert_eq!(
            resolve_startup_paths(Vec::<OsString>::new(), None)
                .unwrap_err()
                .to_string(),
            "chief daemon: home directory unavailable"
        );
        let directory = TestDir::new();
        assert!(matches!(
            resolve_startup_paths(
                vec![OsString::from("relative.toml")],
                Some(directory.0.clone().into_os_string())
            ),
            Err(ChiefDaemonError::InvalidInvocation)
        ));
        assert!(matches!(
            resolve_startup_paths(
                vec![
                    directory.0.join("one").into_os_string(),
                    directory.0.join("two").into_os_string()
                ],
                Some(directory.0.clone().into_os_string())
            ),
            Err(ChiefDaemonError::InvalidInvocation)
        ));
    }

    #[test]
    fn loads_a_bounded_regular_config() {
        let directory = TestDir::new();
        let path = directory.0.join("config.toml");
        fs::write(&path, VALID_CONFIG).unwrap();
        let config = load_config_file(&path).unwrap();
        assert_eq!(config.orchestrator().port(), 7463);
    }

    #[test]
    fn rejects_oversized_and_non_utf8_configs() {
        let directory = TestDir::new();
        let oversized = directory.0.join("oversized.toml");
        fs::write(&oversized, vec![b'x'; MAX_CONFIG_BYTES + 1]).unwrap();
        assert!(matches!(
            load_config_file(&oversized),
            Err(ChiefDaemonError::ConfigFileTooLarge)
        ));
        let encoded = directory.0.join("encoded.toml");
        fs::write(&encoded, [0xff]).unwrap();
        assert!(matches!(
            load_config_file(&encoded),
            Err(ChiefDaemonError::ConfigFileEncoding)
        ));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_final_config_symlink() {
        use std::os::unix::fs::symlink;
        let directory = TestDir::new();
        let target = directory.0.join("target.toml");
        let link = directory.0.join("config.toml");
        fs::write(&target, VALID_CONFIG).unwrap();
        symlink(&target, &link).unwrap();
        assert!(matches!(
            load_config_file(&link),
            Err(ChiefDaemonError::ConfigFileNotRegular)
        ));
    }
}
