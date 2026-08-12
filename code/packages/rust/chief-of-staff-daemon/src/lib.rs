//! Concrete composition root for the D18 Chief daemon.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_channel_endpoints::{
    MessageId, MessageMetadata, MessageMetadataError, MessageMetadataSource,
};
use chief_of_staff_daemon_api::{BindAddress, DaemonApi, DaemonApiError};
use chief_of_staff_daemon_authority_provisioning::{
    provision_authorities, AuthorityProvisioningError,
};
use chief_of_staff_daemon_config::{
    parse_config, ChiefConfig, ConfigError, SmartHomeListenerConfig, SmartHomeToolGrantConfig,
    SmartHomeToolGrantStatus,
};
use chief_of_staff_daemon_credential::{load_or_create_credential, CredentialFileError};
use chief_of_staff_daemon_keyring::{load_package_keyring, KeyringLoadError};
use chief_of_staff_daemon_policy::{DenyChannelWiring, LocalAuthError, LocalBearerAuthorizer};
use chief_of_staff_daemon_runtime::{ChiefDaemonRuntime, DaemonRuntimeError, ReconcileSchedule};
use chief_of_staff_host_control_protocol::{
    DataPlaneFailure, ModelToolCall, ModelToolDefinition, ModelToolResult,
};
use chief_of_staff_host_data_plane::{
    AuthorityBackedHostDataPlaneService, DurableHostDataPlaneDispatcher, HostDataPlaneDispatcher,
    HostDataPlaneService, ModelToolDispatcher, UnavailableHostDataPlaneService,
};
use chief_of_staff_orchestrator_core::OrchestratorCore;
use chief_of_staff_process_supervisor::{
    DurableHostLaunchBindings, HostProgram, MonotonicClock, ProcessSupervisorConfig,
    ProcessSupervisorError, SystemMonotonicClock, UuidV7SessionIdSource,
};
use chief_of_staff_service_reconciler::{ConfigError as ReconcileConfigError, ReconcileConfig};
use chief_of_staff_smart_home_tools::{
    smart_home_tool_definition, SmartHomeToolBridge, SMART_HOME_COMMAND_TOOL_ID,
    SMART_HOME_COMPLETE_PAIRING_TOOL_ID, SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID,
    SMART_HOME_DISCOVER_TOOL_ID, SMART_HOME_GET_HEALTH_TOOL_ID, SMART_HOME_GET_STATE_TOOL_ID,
    SMART_HOME_LIST_BRIDGES_TOOL_ID, SMART_HOME_LIST_DEVICES_TOOL_ID,
    SMART_HOME_OBSERVE_SUPERVISION_TOOL_ID, SMART_HOME_PAIR_BRIDGE_TOOL_ID,
};
use chief_of_staff_tool_api::{RequestedBy, ToolInvocationRequest};
use coding_adventures_json_serializer::serialize as serialize_json;
use coding_adventures_json_value::{parse as parse_json, JsonValue};
use coding_adventures_storage_fs::FsStorageBackend;
use coding_adventures_x3dh::generate_identity_keypair;
use embeddable_http_server::HttpServerOptions;
use process_shutdown::{ShutdownError, ShutdownListener};
use smart_home_controller_runtime::{ControllerRestoreError, SmartHomeControllerRuntime};
use smart_home_core::{
    AgentId as SmartHomeAgentId, CapabilityGrant, CapabilityGrantId, CapabilityGrantScope,
    CapabilityGrantStatus, PrivilegeTier, SmartHomeTool,
};
use smart_home_platform_http::{
    home_assistant_runtime_web_app, SmartHomePlatformHttpConfig, SmartHomePlatformHttpRuntime,
};
use std::convert::Infallible;
use std::env;
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::fs::{self, File, Metadata};
use std::io::Read;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use storage_core::{StorageBackend, StorageError};
use transport_platform::{PlatformError, TransportPlatform};
use web_core::{WebApp, WebServer};
use websocket_runtime::WebSocketServerOptions;

const MAX_CONFIG_BYTES: usize = 256 * 1024;
const DEFAULT_CONFIG_SUFFIX: &str = ".chief-of-staff/config.toml";
const HEARTBEAT_GRACE_INTERVALS: u64 = 3;
const SMART_HOME_HTTP_PRINCIPAL_ID: &str = "agent:home-assistant-local-api";
const SMART_HOME_HTTP_GRANT_ID: &str = "grant:agent:home-assistant-local-api:local-api-full-access";

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
    /// Explicit channel-key or model-provider authority provisioning failed.
    Authority(AuthorityProvisioningError),
    /// The central smart-home controller could not restore its durable state.
    SmartHome(ControllerRestoreError),
    /// Declared Chief-host smart-home grants could not be validated or committed.
    SmartHomeGrantProvisioning,
    /// The Home Assistant-compatible listener could not bind or serve.
    SmartHomeHttp(PlatformError),
    /// The Home Assistant-compatible listener thread panicked.
    SmartHomeHttpPanicked,
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
            Self::Authority(_) => "chief daemon: data-plane authority provisioning failed",
            Self::SmartHome(_) => "chief daemon: smart-home controller restore failed",
            Self::SmartHomeGrantProvisioning => {
                "chief daemon: smart-home grant provisioning failed"
            }
            Self::SmartHomeHttp(_) => "chief daemon: smart-home HTTP listener failed",
            Self::SmartHomeHttpPanicked => "chief daemon: smart-home HTTP listener panicked",
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

    let backend: Arc<dyn StorageBackend> = Arc::new(FsStorageBackend::new(state_dir.clone()));
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
    let clock: Arc<dyn MonotonicClock> = Arc::new(SystemMonotonicClock::new());
    let launch_bindings = Arc::new(DurableHostLaunchBindings::new(Arc::clone(&backend)));
    let needs_smart_home_controller = config.smart_home().is_some()
        || !config.data_plane().ollama_models().is_empty()
        || !config.data_plane().smart_home_tool_grants().is_empty();
    let smart_home_controller = needs_smart_home_controller
        .then(|| SmartHomeControllerRuntime::restore(FsStorageBackend::new(&state_dir)))
        .transpose()
        .map_err(ChiefDaemonError::SmartHome)?;
    let unix_clock: Arc<dyn UnixTimeClock> = Arc::new(SystemUnixTimeClock);
    let data_plane = compose_host_data_plane_with_controller(
        &config,
        home,
        Arc::clone(&backend),
        Arc::clone(&clock),
        smart_home_controller.clone(),
        Arc::clone(&unix_clock),
    )?;
    let smart_home_http = config
        .smart_home()
        .map(|listener| {
            let controller = smart_home_controller
                .clone()
                .ok_or(ChiefDaemonError::SmartHomeGrantProvisioning)?;
            compose_smart_home_http_service(listener, controller, Arc::clone(&unix_clock))
        })
        .transpose()?;
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
    run_platform(address, api, schedule, smart_home_http)
}

/// Compose the exact production host data plane from validated daemon authority.
///
/// The returned dispatcher reloads durable pipeline authorization for every
/// request. Non-empty data-plane declarations provision the file-backed channel
/// keys and explicit Ollama providers used by [`run`]; absent or empty
/// declarations retain the redacted unavailable service.
pub fn compose_host_data_plane(
    config: &ChiefConfig,
    home: &Path,
    backend: Arc<dyn StorageBackend>,
    clock: Arc<dyn MonotonicClock>,
) -> Result<Arc<dyn HostDataPlaneDispatcher>, ChiefDaemonError> {
    let needs_controller = !config.data_plane().ollama_models().is_empty()
        || !config.data_plane().smart_home_tool_grants().is_empty();
    let controller = if needs_controller {
        let state_dir = config
            .orchestrator()
            .state_dir()
            .resolve(home)
            .map_err(ChiefDaemonError::Config)?;
        Some(
            SmartHomeControllerRuntime::restore(FsStorageBackend::new(state_dir))
                .map_err(ChiefDaemonError::SmartHome)?,
        )
    } else {
        None
    };
    compose_host_data_plane_with_controller(
        config,
        home,
        backend,
        clock,
        controller,
        Arc::new(SystemUnixTimeClock),
    )
}

fn compose_host_data_plane_with_controller(
    config: &ChiefConfig,
    home: &Path,
    backend: Arc<dyn StorageBackend>,
    clock: Arc<dyn MonotonicClock>,
    controller: Option<SmartHomeControllerRuntime<FsStorageBackend>>,
    unix_clock: Arc<dyn UnixTimeClock>,
) -> Result<Arc<dyn HostDataPlaneDispatcher>, ChiefDaemonError> {
    let metadata_source: Arc<dyn MessageMetadataSource> =
        Arc::new(SystemMessageMetadataSource::new(clock));
    let service = compose_data_plane_service_with_controller(
        config,
        home,
        Arc::clone(&backend),
        metadata_source,
        controller,
        unix_clock,
    )?;
    Ok(Arc::new(DurableHostDataPlaneDispatcher::new(
        backend, service,
    )))
}

#[cfg(test)]
fn compose_data_plane_service(
    config: &ChiefConfig,
    home: &Path,
    backend: Arc<dyn StorageBackend>,
    metadata_source: Arc<dyn MessageMetadataSource>,
) -> Result<Arc<dyn HostDataPlaneService>, ChiefDaemonError> {
    let needs_controller = !config.data_plane().ollama_models().is_empty()
        || !config.data_plane().smart_home_tool_grants().is_empty();
    let controller = if needs_controller {
        let state_dir = config
            .orchestrator()
            .state_dir()
            .resolve(home)
            .map_err(ChiefDaemonError::Config)?;
        Some(
            SmartHomeControllerRuntime::restore(FsStorageBackend::new(state_dir))
                .map_err(ChiefDaemonError::SmartHome)?,
        )
    } else {
        None
    };
    compose_data_plane_service_with_controller(
        config,
        home,
        backend,
        metadata_source,
        controller,
        Arc::new(SystemUnixTimeClock),
    )
}

fn compose_data_plane_service_with_controller(
    config: &ChiefConfig,
    home: &Path,
    backend: Arc<dyn StorageBackend>,
    metadata_source: Arc<dyn MessageMetadataSource>,
    controller: Option<SmartHomeControllerRuntime<FsStorageBackend>>,
    unix_clock: Arc<dyn UnixTimeClock>,
) -> Result<Arc<dyn HostDataPlaneService>, ChiefDaemonError> {
    if config.data_plane().channel_keys().is_empty()
        && config.data_plane().ollama_models().is_empty()
        && config.data_plane().smart_home_tool_grants().is_empty()
    {
        return Ok(Arc::new(UnavailableHostDataPlaneService));
    }
    let authorities =
        provision_authorities(config.data_plane(), home).map_err(ChiefDaemonError::Authority)?;
    let (channel_keys, models) = authorities.into_parts();
    if config.data_plane().ollama_models().is_empty()
        && config.data_plane().smart_home_tool_grants().is_empty()
    {
        return Ok(Arc::new(AuthorityBackedHostDataPlaneService::new(
            backend,
            Arc::new(channel_keys),
            Arc::new(models),
            metadata_source,
        )));
    }
    let controller = controller.ok_or(ChiefDaemonError::SmartHomeGrantProvisioning)?;
    provision_smart_home_tool_grants(
        &controller,
        config.data_plane().smart_home_tool_grants(),
        unix_clock.as_ref(),
    )?;
    if config.data_plane().ollama_models().is_empty() {
        return Ok(Arc::new(AuthorityBackedHostDataPlaneService::new(
            backend,
            Arc::new(channel_keys),
            Arc::new(models),
            metadata_source,
        )));
    }
    let bridge = SmartHomeToolBridge::new(
        controller,
        SmartHomeAgentId::trusted("chief-daemon-model-tools"),
    );
    Ok(Arc::new(
        AuthorityBackedHostDataPlaneService::with_model_tools(
            backend,
            Arc::new(channel_keys),
            Arc::new(models),
            Arc::new(D18dSmartHomeModelTools {
                bridge,
                clock: unix_clock,
            }),
            metadata_source,
        ),
    ))
}

const PRODUCTION_SMART_HOME_MODEL_TOOLS: &[&str] = &[
    SMART_HOME_LIST_BRIDGES_TOOL_ID,
    SMART_HOME_DISCOVER_TOOL_ID,
    SMART_HOME_LIST_DEVICES_TOOL_ID,
    SMART_HOME_GET_STATE_TOOL_ID,
    SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID,
    SMART_HOME_GET_HEALTH_TOOL_ID,
    SMART_HOME_COMMAND_TOOL_ID,
    SMART_HOME_PAIR_BRIDGE_TOOL_ID,
    SMART_HOME_COMPLETE_PAIRING_TOOL_ID,
    SMART_HOME_OBSERVE_SUPERVISION_TOOL_ID,
];

struct SmartHomeHttpService {
    address: SocketAddr,
    app: Arc<WebApp>,
}

fn compose_smart_home_http_service(
    config: &SmartHomeListenerConfig,
    controller: SmartHomeControllerRuntime<FsStorageBackend>,
    clock: Arc<dyn UnixTimeClock>,
) -> Result<SmartHomeHttpService, ChiefDaemonError> {
    let runtime = compose_smart_home_http_runtime(config, controller, clock)?;
    Ok(SmartHomeHttpService {
        address: SocketAddr::new(config.bind(), config.port()),
        app: Arc::new(home_assistant_runtime_web_app(runtime)),
    })
}

fn compose_smart_home_http_runtime<B: StorageBackend + 'static>(
    config: &SmartHomeListenerConfig,
    controller: SmartHomeControllerRuntime<B>,
    clock: Arc<dyn UnixTimeClock>,
) -> Result<SmartHomePlatformHttpRuntime, ChiefDaemonError> {
    provision_smart_home_http_grant(&controller, clock.as_ref())?;
    let request_clock = Arc::clone(&clock);
    let runtime = SmartHomePlatformHttpRuntime::from_shared_runtime(
        controller.runtime_handle(),
        SmartHomePlatformHttpConfig::new(config.instance_name()),
    )
    .with_principal_id(SmartHomeAgentId::trusted(SMART_HOME_HTTP_PRINCIPAL_ID))
    .with_clock(move || request_clock.now_ms().unwrap_or(0))
    .with_automation_runtime(controller.automation_runtime_handle())
    .with_mutation_persistence(controller.runtime_persistence_adapter())
    .with_automation_persistence(controller.automation_persistence_adapter());
    Ok(runtime)
}

fn provision_smart_home_http_grant<B: StorageBackend>(
    controller: &SmartHomeControllerRuntime<B>,
    clock: &dyn UnixTimeClock,
) -> Result<(), ChiefDaemonError> {
    let saved_at_ms = clock
        .now_ms()
        .ok_or(ChiefDaemonError::SmartHomeGrantProvisioning)?;
    let grant_id = CapabilityGrantId::trusted(SMART_HOME_HTTP_GRANT_ID);
    let runtime = controller.runtime_handle();
    let existing = runtime
        .lock()
        .map_err(|_| ChiefDaemonError::SmartHomeGrantProvisioning)?
        .registry()
        .capability_grant(&grant_id)
        .cloned();
    if let Some(grant) = existing {
        let compatible = grant.principal_id
            == SmartHomeAgentId::trusted(SMART_HOME_HTTP_PRINCIPAL_ID)
            && grant.scope == CapabilityGrantScope::AllSmartHome
            && grant.max_tier == PrivilegeTier::HighRisk
            && grant.expires_at_ms.is_none()
            && grant.status == CapabilityGrantStatus::Active
            && grant.metadata.is_empty()
            && grant.granted_at_ms <= saved_at_ms;
        return compatible
            .then_some(())
            .ok_or(ChiefDaemonError::SmartHomeGrantProvisioning);
    }
    let grant = CapabilityGrant::for_all_smart_home(
        grant_id,
        SmartHomeAgentId::trusted(SMART_HOME_HTTP_PRINCIPAL_ID),
        PrivilegeTier::HighRisk,
        "chief-daemon",
        saved_at_ms,
    );
    controller
        .transaction(saved_at_ms, move |runtime, _| {
            runtime.registry_mut().upsert_capability_grant(grant);
            Ok::<(), Infallible>(())
        })
        .map_err(|_| ChiefDaemonError::SmartHomeGrantProvisioning)?;
    Ok(())
}

struct D18dSmartHomeModelTools<B> {
    bridge: SmartHomeToolBridge<B>,
    clock: Arc<dyn UnixTimeClock>,
}

trait UnixTimeClock: Send + Sync {
    fn now_ms(&self) -> Option<u64>;
}

struct SystemUnixTimeClock;

impl UnixTimeClock for SystemUnixTimeClock {
    fn now_ms(&self) -> Option<u64> {
        let milliseconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_millis();
        u64::try_from(milliseconds).ok()
    }
}

fn provision_smart_home_tool_grants<B: StorageBackend>(
    controller: &SmartHomeControllerRuntime<B>,
    declarations: &[SmartHomeToolGrantConfig],
    clock: &dyn UnixTimeClock,
) -> Result<(), ChiefDaemonError> {
    if declarations.is_empty() {
        return Ok(());
    }
    let grants = declarations
        .iter()
        .map(configured_smart_home_tool_grant)
        .collect::<Result<Vec<_>, _>>()?;
    let saved_at_ms = clock
        .now_ms()
        .ok_or(ChiefDaemonError::SmartHomeGrantProvisioning)?;
    if grants.iter().any(|grant| grant.granted_at_ms > saved_at_ms) {
        return Err(ChiefDaemonError::SmartHomeGrantProvisioning);
    }
    let runtime = controller.runtime_handle();
    let already_current = {
        let runtime = runtime
            .lock()
            .map_err(|_| ChiefDaemonError::SmartHomeGrantProvisioning)?;
        grants
            .iter()
            .all(|grant| runtime.registry().capability_grant(&grant.grant_id) == Some(grant))
    };
    if already_current {
        return Ok(());
    }
    controller
        .transaction(saved_at_ms, move |runtime, _| {
            for grant in grants {
                runtime.registry_mut().upsert_capability_grant(grant);
            }
            Ok::<(), Infallible>(())
        })
        .map_err(|_| ChiefDaemonError::SmartHomeGrantProvisioning)?;
    Ok(())
}

fn configured_smart_home_tool_grant(
    declaration: &SmartHomeToolGrantConfig,
) -> Result<CapabilityGrant, ChiefDaemonError> {
    let tool = production_smart_home_tool(declaration.tool_id())
        .ok_or(ChiefDaemonError::SmartHomeGrantProvisioning)?;
    let mut grant = CapabilityGrant::for_tool(
        CapabilityGrantId::trusted(declaration.grant_id()),
        SmartHomeAgentId::trusted(declaration.principal_id()),
        tool,
        declaration.granted_by(),
        declaration.granted_at_ms(),
    );
    if let Some(expires_at_ms) = declaration.expires_at_ms() {
        grant = grant.with_expiry(expires_at_ms);
    }
    grant = grant.with_status(match declaration.status() {
        SmartHomeToolGrantStatus::Pending => CapabilityGrantStatus::Pending,
        SmartHomeToolGrantStatus::Active => CapabilityGrantStatus::Active,
        SmartHomeToolGrantStatus::Revoked => CapabilityGrantStatus::Revoked,
    });
    Ok(grant)
}

fn production_smart_home_tool(tool_id: &str) -> Option<SmartHomeTool> {
    match tool_id {
        SMART_HOME_LIST_BRIDGES_TOOL_ID => Some(SmartHomeTool::ListBridges),
        SMART_HOME_DISCOVER_TOOL_ID => Some(SmartHomeTool::Discover),
        SMART_HOME_LIST_DEVICES_TOOL_ID => Some(SmartHomeTool::ListDevices),
        SMART_HOME_GET_STATE_TOOL_ID => Some(SmartHomeTool::GetState),
        SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID => Some(SmartHomeTool::DescribeCapabilities),
        SMART_HOME_GET_HEALTH_TOOL_ID => Some(SmartHomeTool::GetHealth),
        SMART_HOME_COMMAND_TOOL_ID => Some(SmartHomeTool::Command),
        SMART_HOME_PAIR_BRIDGE_TOOL_ID => Some(SmartHomeTool::PairBridge),
        SMART_HOME_COMPLETE_PAIRING_TOOL_ID => Some(SmartHomeTool::CompletePairing),
        SMART_HOME_OBSERVE_SUPERVISION_TOOL_ID => Some(SmartHomeTool::ObserveSupervision),
        _ => None,
    }
}

impl<B: StorageBackend + 'static> ModelToolDispatcher for D18dSmartHomeModelTools<B> {
    fn definitions(
        &self,
        _binding: &chief_of_staff_pipeline_bindings::HostPipelineBinding,
    ) -> Result<Vec<ModelToolDefinition>, DataPlaneFailure> {
        PRODUCTION_SMART_HOME_MODEL_TOOLS
            .iter()
            .map(|tool_id| {
                let definition =
                    smart_home_tool_definition(tool_id).ok_or(DataPlaneFailure::Internal)?;
                let input_schema_json = serialize_json(&definition.input_json_schema())
                    .map_err(|_| DataPlaneFailure::Internal)?;
                Ok(ModelToolDefinition {
                    name: definition.tool_id,
                    description: definition.description,
                    input_schema: serde_json::from_str(&input_schema_json)
                        .map_err(|_| DataPlaneFailure::Internal)?,
                })
            })
            .collect()
    }

    fn execute(
        &self,
        binding: &chief_of_staff_pipeline_bindings::HostPipelineBinding,
        call: &ModelToolCall,
    ) -> Result<ModelToolResult, DataPlaneFailure> {
        if !PRODUCTION_SMART_HOME_MODEL_TOOLS.contains(&call.name.as_str()) {
            return Err(DataPlaneFailure::Unauthorized);
        }
        let arguments_json =
            serde_json::to_string(&call.arguments).map_err(|_| DataPlaneFailure::InvalidRequest)?;
        let arguments =
            parse_json(&arguments_json).map_err(|_| DataPlaneFailure::InvalidRequest)?;
        let requested_at = self.clock.now_ms().ok_or(DataPlaneFailure::Internal)?;
        let result = self
            .bridge
            .invoke(&ToolInvocationRequest {
                call_id: call.call_id.clone(),
                tool_id: call.name.clone(),
                arguments,
                requested_by: RequestedBy::Agent,
                session_id: None,
                job_id: None,
                agent_id: Some(binding.registration().host_name().as_str().to_string()),
                user_id: None,
                requested_at,
                deadline_at: None,
                idempotency_key: None,
            })
            .map_err(|_| DataPlaneFailure::Internal)?;
        let (output, is_error) = if result.ok {
            (result.output.unwrap_or(JsonValue::Null), false)
        } else {
            let error = result.error.ok_or(DataPlaneFailure::Internal)?;
            (
                JsonValue::Object(vec![
                    (
                        "kind".to_string(),
                        JsonValue::String(error.kind.to_string()),
                    ),
                    ("message".to_string(), JsonValue::String(error.message)),
                    ("details".to_string(), error.details),
                ]),
                true,
            )
        };
        let output_json = serialize_json(&output).map_err(|_| DataPlaneFailure::Internal)?;
        Ok(ModelToolResult {
            call: call.clone(),
            output: serde_json::from_str(&output_json).map_err(|_| DataPlaneFailure::Internal)?,
            is_error,
        })
    }
}

struct SystemMessageMetadataSource {
    clock: Arc<dyn MonotonicClock>,
}

impl SystemMessageMetadataSource {
    fn new(clock: Arc<dyn MonotonicClock>) -> Self {
        Self { clock }
    }
}

impl MessageMetadataSource for SystemMessageMetadataSource {
    fn next_metadata(&self) -> Result<MessageMetadata, MessageMetadataError> {
        let uuid = coding_adventures_uuid::v7()
            .map_err(|_| MessageMetadataError::new("message identity unavailable"))?;
        let message_id = MessageId::from_uuid_v7(uuid.bytes())
            .map_err(|_| MessageMetadataError::new("message identity invalid"))?;
        Ok(MessageMetadata {
            message_id,
            timestamp_ns: self.clock.now_ns(),
        })
    }
}

fn serve<P, C, A>(
    platform: P,
    address: BindAddress,
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
    smart_home: Option<(P, SmartHomeHttpService)>,
) -> Result<(), ChiefDaemonError>
where
    P: TransportPlatform + Send + 'static,
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
    let mut smart_home_server = smart_home
        .map(|(platform, service)| {
            WebServer::bind(
                platform,
                BindAddress::Ip(service.address),
                HttpServerOptions::default(),
                service.app,
            )
            .map_err(ChiefDaemonError::SmartHomeHttp)
        })
        .transpose()?;
    let daemon_stop = runtime.stop_handle();
    let smart_home_stop = smart_home_server.as_ref().map(WebServer::stop_handle);
    let listener_daemon_stop = daemon_stop.clone();
    let listener_smart_home_stop = smart_home_stop.clone();
    let listener = ShutdownListener::install(move |_| {
        listener_daemon_stop.stop();
        if let Some(stop) = listener_smart_home_stop {
            stop.stop();
        }
    })
    .map_err(ChiefDaemonError::Shutdown)?;
    let smart_home_thread = smart_home_server.take().map(|mut server| {
        let address = server.local_addr();
        let daemon_stop = daemon_stop.clone();
        eprintln!("chief smart-home HTTP listening on {address}");
        thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| server.serve()));
            daemon_stop.stop();
            result
                .map_err(|_| ChiefDaemonError::SmartHomeHttpPanicked)?
                .map_err(ChiefDaemonError::SmartHomeHttp)
        })
    });
    eprintln!("chief daemon listening on {}", runtime.local_addr());
    let runtime_result = runtime.serve();
    if let Some(stop) = smart_home_stop {
        stop.stop();
    }
    let smart_home_result = smart_home_thread
        .map(|thread| {
            thread
                .join()
                .map_err(|_| ChiefDaemonError::SmartHomeHttpPanicked)?
        })
        .transpose();
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
    smart_home_result?;
    shutdown_result.map_err(ChiefDaemonError::Shutdown)?;
    recovery_result
}

#[cfg(target_os = "linux")]
fn run_platform<C, A>(
    address: BindAddress,
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
    smart_home: Option<SmartHomeHttpService>,
) -> Result<(), ChiefDaemonError>
where
    C: chief_of_staff_daemon_api::ChiefControlPlane + Send + 'static,
    A: chief_of_staff_daemon_api::SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    let platform = transport_platform::linux::EpollTransportPlatform::new()
        .map_err(ChiefDaemonError::Platform)?;
    let smart_home = smart_home
        .map(|service| {
            transport_platform::linux::EpollTransportPlatform::new()
                .map(|platform| (platform, service))
                .map_err(ChiefDaemonError::Platform)
        })
        .transpose()?;
    serve(platform, address, api, schedule, smart_home)
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
    smart_home: Option<SmartHomeHttpService>,
) -> Result<(), ChiefDaemonError>
where
    C: chief_of_staff_daemon_api::ChiefControlPlane + Send + 'static,
    A: chief_of_staff_daemon_api::SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    let platform = transport_platform::bsd::KqueueTransportPlatform::new()
        .map_err(ChiefDaemonError::Platform)?;
    let smart_home = smart_home
        .map(|service| {
            transport_platform::bsd::KqueueTransportPlatform::new()
                .map(|platform| (platform, service))
                .map_err(ChiefDaemonError::Platform)
        })
        .transpose()?;
    serve(platform, address, api, schedule, smart_home)
}

#[cfg(target_os = "windows")]
fn run_platform<C, A>(
    address: BindAddress,
    api: Arc<DaemonApi<C, A>>,
    schedule: ReconcileSchedule,
    smart_home: Option<SmartHomeHttpService>,
) -> Result<(), ChiefDaemonError>
where
    C: chief_of_staff_daemon_api::ChiefControlPlane + Send + 'static,
    A: chief_of_staff_daemon_api::SessionAuthorizer + Send + Sync + 'static,
    A::Session: Send + 'static,
{
    let platform = transport_platform::windows::WindowsTransportPlatform::new()
        .map_err(ChiefDaemonError::Platform)?;
    let smart_home = smart_home
        .map(|service| {
            transport_platform::windows::WindowsTransportPlatform::new()
                .map(|platform| (platform, service))
                .map_err(ChiefDaemonError::Platform)
        })
        .transpose()?;
    serve(platform, address, api, schedule, smart_home)
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
    _smart_home: Option<SmartHomeHttpService>,
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
    use chief_of_staff_channel_endpoints::AgentId as ChannelAgentId;
    use chief_of_staff_host_control_protocol::{LaunchBindings, LevelOneModelBinding};
    use chief_of_staff_pipeline_bindings::{HostPipelineBinding, PipelineId};
    use chief_of_staff_service_registry::{HostName, HostRegistration, PackagePath, RestartPolicy};
    use smart_home_core::{
        AgentId, AuthorizationOutcome, Bridge, BridgeId, BridgeTransport, CapabilityGrant,
        CapabilityGrantId, Device, DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId,
        SmartHomeTool,
    };
    use std::convert::Infallible;
    use std::sync::atomic::{AtomicU64, Ordering};
    use storage_core::InMemoryStorageBackend;

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestUnixTimeClock(AtomicU64);

    impl TestUnixTimeClock {
        fn new(now_ms: u64) -> Self {
            Self(AtomicU64::new(now_ms))
        }

        fn set(&self, now_ms: u64) {
            self.0.store(now_ms, Ordering::Relaxed);
        }
    }

    impl UnixTimeClock for TestUnixTimeClock {
        fn now_ms(&self) -> Option<u64> {
            Some(self.0.load(Ordering::Relaxed))
        }
    }

    struct UnavailableUnixTimeClock;

    impl UnixTimeClock for UnavailableUnixTimeClock {
        fn now_ms(&self) -> Option<u64> {
            None
        }
    }

    fn test_model_binding() -> HostPipelineBinding {
        test_model_binding_for("home-host")
    }

    fn test_model_binding_for(host_name: &str) -> HostPipelineBinding {
        let mut pipeline_id = [0; 16];
        pipeline_id[6] = 0x70;
        pipeline_id[8] = 0x80;
        HostPipelineBinding::new(
            PipelineId::new(pipeline_id).unwrap(),
            HostRegistration::new(
                HostName::new(host_name).unwrap(),
                PackagePath::new("/srv/home.agent").unwrap(),
                [7; 32],
                RestartPolicy::Always,
            ),
            ChannelAgentId::new(b"home-agent".to_vec()).unwrap(),
            LaunchBindings::new(
                Vec::new(),
                Some(LevelOneModelBinding::new("test-model", 0.0, 128).unwrap()),
            )
            .unwrap(),
        )
    }

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
    fn configured_authorities_fail_startup_without_disclosing_secret_paths() {
        let directory = TestDir::new();
        let config = parse_config(&format!(
            "{VALID_CONFIG}\n[data_plane]\nchannel_keys = [\n  {{ pipeline_id = \"018f0c10-7b4a-7cc0-8000-000000000001\", agent_id = \"weather\", channel_id = \"018f0c10-7b4a-7cc0-8000-000000000002\", access = \"read\", private_key_path = \"~/missing-private-key.bin\" }},\n]\nollama_models = []\n"
        ))
        .unwrap();
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        let clock: Arc<dyn MonotonicClock> = Arc::new(SystemMonotonicClock::new());
        let metadata: Arc<dyn MessageMetadataSource> =
            Arc::new(SystemMessageMetadataSource::new(clock));
        let Err(error) = compose_data_plane_service(&config, &directory.0, backend, metadata)
        else {
            panic!("missing private authority unexpectedly composed");
        };
        assert!(matches!(error, ChiefDaemonError::Authority(_)));
        assert_eq!(
            error.to_string(),
            "chief daemon: data-plane authority provisioning failed"
        );
        assert!(!error.to_string().contains("missing-private-key.bin"));
    }

    #[test]
    fn production_publish_metadata_is_uuid_v7_and_monotonic() {
        let clock: Arc<dyn MonotonicClock> = Arc::new(SystemMonotonicClock::new());
        let source = SystemMessageMetadataSource::new(clock);
        let first = source.next_metadata().unwrap();
        let second = source.next_metadata().unwrap();
        assert_eq!(first.message_id.as_bytes()[6] >> 4, 7);
        assert_eq!(first.message_id.as_bytes()[8] & 0xc0, 0x80);
        assert!(second.timestamp_ns >= first.timestamp_ns);
    }

    #[test]
    fn production_model_catalog_uses_host_identity_and_current_request_time() {
        let backend = InMemoryStorageBackend::new();
        let controller = SmartHomeControllerRuntime::restore(backend).unwrap();
        let controller_probe = controller.clone();
        controller
            .transaction(1_000, |runtime, _| {
                runtime.registry_mut().upsert_capability_grant(
                    CapabilityGrant::for_tool(
                        CapabilityGrantId::trusted("grant-list-devices"),
                        AgentId::trusted("home-host"),
                        SmartHomeTool::ListDevices,
                        "user:test",
                        1_000,
                    )
                    .with_expiry(2_000),
                );
                Ok::<(), Infallible>(())
            })
            .unwrap();
        let clock = Arc::new(TestUnixTimeClock::new(1_500));
        let tools = D18dSmartHomeModelTools {
            bridge: SmartHomeToolBridge::new(
                controller,
                SmartHomeAgentId::trusted("chief-daemon-model-tools"),
            ),
            clock: clock.clone(),
        };
        let binding = test_model_binding();
        let definitions = tools.definitions(&binding).unwrap();
        assert_eq!(definitions.len(), PRODUCTION_SMART_HOME_MODEL_TOOLS.len());
        assert!(definitions
            .iter()
            .any(|definition| definition.name == SMART_HOME_GET_HEALTH_TOOL_ID));

        let call = ModelToolCall {
            call_id: "call-1".to_string(),
            name: SMART_HOME_LIST_DEVICES_TOOL_ID.to_string(),
            arguments: serde_json::json!({}),
        };
        let result = tools.execute(&binding, &call).unwrap();
        assert_eq!(result.call, call);
        assert!(result.output.is_object());
        assert!(!result.is_error);

        clock.set(2_000);
        let expired_call = ModelToolCall {
            call_id: "call-2".to_string(),
            ..call.clone()
        };
        let expired = tools.execute(&binding, &expired_call).unwrap();
        assert!(expired.is_error);
        assert_eq!(controller_probe.last_saved_at_ms().unwrap(), Some(2_000));

        let runtime = controller_probe.runtime_handle();
        let runtime = runtime.lock().unwrap();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&AgentId::trusted("home-host"));
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Allowed);
        assert_eq!(decisions[0].decided_at_ms, 1_500);
        assert_eq!(decisions[1].outcome, AuthorizationOutcome::Denied);
        assert_eq!(decisions[1].decided_at_ms, 2_000);

        let mut unknown = call;
        unknown.name = "smart_home.uninstalled".to_string();
        assert_eq!(
            tools.execute(&binding, &unknown),
            Err(DataPlaneFailure::Unauthorized)
        );
    }

    #[test]
    fn production_model_tools_fail_closed_when_unix_time_is_unavailable() {
        let controller =
            SmartHomeControllerRuntime::restore(InMemoryStorageBackend::new()).unwrap();
        let controller_probe = controller.clone();
        let tools = D18dSmartHomeModelTools {
            bridge: SmartHomeToolBridge::new(
                controller,
                SmartHomeAgentId::trusted("chief-daemon-model-tools"),
            ),
            clock: Arc::new(UnavailableUnixTimeClock),
        };
        let binding = test_model_binding();
        let call = ModelToolCall {
            call_id: "call-1".to_string(),
            name: SMART_HOME_LIST_DEVICES_TOOL_ID.to_string(),
            arguments: serde_json::json!({}),
        };

        assert_eq!(
            tools.execute(&binding, &call),
            Err(DataPlaneFailure::Internal)
        );
        assert_eq!(
            controller_probe
                .runtime_handle()
                .lock()
                .unwrap()
                .registry()
                .counts()
                .authorization_decisions,
            0
        );
    }

    #[test]
    fn smart_home_http_composition_shares_one_durable_controller() {
        let directory = TestDir::new();
        let state_dir = directory.0.join("smart-home-http-state");
        let controller =
            SmartHomeControllerRuntime::restore(FsStorageBackend::new(&state_dir)).unwrap();
        let config = smart_home_listener_config();
        let clock: Arc<dyn UnixTimeClock> = Arc::new(TestUnixTimeClock::new(1_500));
        let http = compose_smart_home_http_runtime(
            config.smart_home().unwrap(),
            controller.clone(),
            Arc::clone(&clock),
        )
        .unwrap();
        let grant_revision = controller.revision().unwrap().unwrap();

        let bridge_id = BridgeId::trusted("bridge:shared-controller");
        let device_id = DeviceId::trusted("device:shared-controller");
        let entity_id = EntityId::trusted("light.shared_controller");
        controller
            .transaction(1_600, |runtime, _| {
                runtime
                    .upsert_bridge(Bridge::new(
                        bridge_id.clone(),
                        IntegrationId::trusted("chief-test"),
                        BridgeTransport::LanHttp,
                    ))
                    .unwrap();
                runtime
                    .upsert_device(Device {
                        device_id: device_id.clone(),
                        bridge_id,
                        manufacturer: "Coding Adventures".to_string(),
                        model: "Shared Controller".to_string(),
                        name: "Shared Controller Device".to_string(),
                        serial: None,
                        firmware_version: None,
                        room_id: None,
                        entity_ids: vec![entity_id.clone()],
                        identifiers: Vec::new(),
                        health: Health::Online,
                        metadata: Vec::new(),
                    })
                    .unwrap();
                runtime
                    .upsert_entity(Entity {
                        entity_id: entity_id.clone(),
                        device_id,
                        kind: EntityKind::Light,
                        name: "Shared Controller Light".to_string(),
                        capabilities: Vec::new(),
                        state: None,
                        metadata: Vec::new(),
                    })
                    .unwrap();
                Ok::<(), Infallible>(())
            })
            .unwrap();

        let state = http.snapshot();
        assert_eq!(state.entities.len(), 1);
        assert_eq!(state.entities[0].entity_id, entity_id);
        assert_eq!(state.generated_at_ms, 1_500);

        let shared_revision = controller.revision().unwrap().unwrap();
        assert_ne!(shared_revision, grant_revision);
        let second = compose_smart_home_http_runtime(
            config.smart_home().unwrap(),
            controller.clone(),
            clock,
        )
        .unwrap();
        assert_eq!(
            controller.revision().unwrap(),
            Some(shared_revision.clone())
        );
        assert_eq!(second.snapshot().entities[0].entity_id, entity_id);
        drop(second);
        drop(http);
        drop(controller);

        let restored =
            SmartHomeControllerRuntime::restore(FsStorageBackend::new(&state_dir)).unwrap();
        assert_eq!(restored.revision().unwrap(), Some(shared_revision));
        let runtime = restored.runtime_handle();
        let runtime = runtime.lock().unwrap();
        assert!(runtime.registry().entity(&entity_id).is_some());
        assert!(runtime
            .registry()
            .capability_grant(&CapabilityGrantId::trusted(SMART_HOME_HTTP_GRANT_ID))
            .is_some());
    }

    #[test]
    fn smart_home_http_grant_provisioning_fails_closed() {
        let controller =
            SmartHomeControllerRuntime::restore(InMemoryStorageBackend::new()).unwrap();
        assert!(matches!(
            provision_smart_home_http_grant(&controller, &UnavailableUnixTimeClock),
            Err(ChiefDaemonError::SmartHomeGrantProvisioning)
        ));
        assert_eq!(controller.revision().unwrap(), None);

        controller
            .transaction(2_000, |runtime, _| {
                runtime.registry_mut().upsert_capability_grant(
                    CapabilityGrant::for_all_smart_home(
                        CapabilityGrantId::trusted(SMART_HOME_HTTP_GRANT_ID),
                        AgentId::trusted(SMART_HOME_HTTP_PRINCIPAL_ID),
                        PrivilegeTier::HighRisk,
                        "chief-daemon",
                        2_000,
                    ),
                );
                Ok::<(), Infallible>(())
            })
            .unwrap();
        let revision = controller.revision().unwrap();
        assert!(matches!(
            provision_smart_home_http_grant(&controller, &TestUnixTimeClock::new(1_500)),
            Err(ChiefDaemonError::SmartHomeGrantProvisioning)
        ));
        assert_eq!(controller.revision().unwrap(), revision);
    }

    #[test]
    fn configured_host_tool_grants_commit_durably_and_support_revocation() {
        let directory = TestDir::new();
        let state_dir = directory.0.join("smart-home-state");
        let controller =
            SmartHomeControllerRuntime::restore(FsStorageBackend::new(&state_dir)).unwrap();
        let active = configured_tool_grant_config("active", SMART_HOME_LIST_DEVICES_TOOL_ID);
        let clock = Arc::new(TestUnixTimeClock::new(1_500));

        provision_smart_home_tool_grants(
            &controller,
            active.data_plane().smart_home_tool_grants(),
            clock.as_ref(),
        )
        .unwrap();
        let first_revision = controller.revision().unwrap().unwrap();
        assert_eq!(controller.last_saved_at_ms().unwrap(), Some(1_500));
        assert!(matches!(
            provision_smart_home_tool_grants(
                &controller,
                active.data_plane().smart_home_tool_grants(),
                &UnavailableUnixTimeClock
            ),
            Err(ChiefDaemonError::SmartHomeGrantProvisioning)
        ));
        clock.set(999);
        assert!(matches!(
            provision_smart_home_tool_grants(
                &controller,
                active.data_plane().smart_home_tool_grants(),
                clock.as_ref()
            ),
            Err(ChiefDaemonError::SmartHomeGrantProvisioning)
        ));
        assert_eq!(controller.revision().unwrap(), Some(first_revision.clone()));
        clock.set(1_500);
        let grant_id = CapabilityGrantId::trusted("grant-weather-list-devices");
        {
            let runtime = controller.runtime_handle();
            let runtime = runtime.lock().unwrap();
            let grant = runtime.registry().capability_grant(&grant_id).unwrap();
            assert_eq!(grant.principal_id, AgentId::trusted("weather-level-one"));
            assert_eq!(
                grant.scope,
                smart_home_core::CapabilityGrantScope::Tool(SmartHomeTool::ListDevices)
            );
            assert_eq!(grant.expires_at_ms, Some(2_000));
            assert_eq!(grant.status, CapabilityGrantStatus::Active);
        }
        let tools = D18dSmartHomeModelTools {
            bridge: SmartHomeToolBridge::new(
                controller.clone(),
                SmartHomeAgentId::trusted("chief-daemon-model-tools"),
            ),
            clock: clock.clone(),
        };
        let call = ModelToolCall {
            call_id: "configured-grant-call".to_string(),
            name: SMART_HOME_LIST_DEVICES_TOOL_ID.to_string(),
            arguments: serde_json::json!({}),
        };
        assert!(
            !tools
                .execute(&test_model_binding_for("weather-level-one"), &call)
                .unwrap()
                .is_error
        );
        let revision_after_execution = controller.revision().unwrap().unwrap();
        assert_ne!(revision_after_execution, first_revision);

        provision_smart_home_tool_grants(
            &controller,
            active.data_plane().smart_home_tool_grants(),
            clock.as_ref(),
        )
        .unwrap();
        assert_eq!(
            controller.revision().unwrap(),
            Some(revision_after_execution)
        );
        drop(tools);
        drop(controller);

        let restored =
            SmartHomeControllerRuntime::restore(FsStorageBackend::new(&state_dir)).unwrap();
        assert!(restored
            .runtime_handle()
            .lock()
            .unwrap()
            .registry()
            .capability_grant(&grant_id)
            .is_some());

        clock.set(1_600);
        let revoked = configured_tool_grant_config("revoked", SMART_HOME_LIST_DEVICES_TOOL_ID);
        provision_smart_home_tool_grants(
            &restored,
            revoked.data_plane().smart_home_tool_grants(),
            clock.as_ref(),
        )
        .unwrap();
        assert_eq!(restored.last_saved_at_ms().unwrap(), Some(1_600));
        assert_eq!(
            restored
                .runtime_handle()
                .lock()
                .unwrap()
                .registry()
                .capability_grant(&grant_id)
                .unwrap()
                .status,
            CapabilityGrantStatus::Revoked
        );
        let revoked_tools = D18dSmartHomeModelTools {
            bridge: SmartHomeToolBridge::new(
                restored,
                SmartHomeAgentId::trusted("chief-daemon-model-tools"),
            ),
            clock,
        };
        assert!(
            revoked_tools
                .execute(&test_model_binding_for("weather-level-one"), &call)
                .unwrap()
                .is_error
        );
    }

    #[test]
    fn configured_host_tool_grants_reject_uninstalled_tools_and_missing_time() {
        let controller =
            SmartHomeControllerRuntime::restore(InMemoryStorageBackend::new()).unwrap();
        let unknown = configured_tool_grant_config("active", "smart_home.list_scenes");
        assert!(matches!(
            provision_smart_home_tool_grants(
                &controller,
                unknown.data_plane().smart_home_tool_grants(),
                &TestUnixTimeClock::new(1_500)
            ),
            Err(ChiefDaemonError::SmartHomeGrantProvisioning)
        ));

        let active = configured_tool_grant_config("active", SMART_HOME_LIST_DEVICES_TOOL_ID);
        assert!(matches!(
            provision_smart_home_tool_grants(
                &controller,
                active.data_plane().smart_home_tool_grants(),
                &UnavailableUnixTimeClock
            ),
            Err(ChiefDaemonError::SmartHomeGrantProvisioning)
        ));
        assert_eq!(controller.revision().unwrap(), None);
        assert_eq!(
            controller
                .runtime_handle()
                .lock()
                .unwrap()
                .registry()
                .counts()
                .capability_grants,
            0
        );

        let future = configured_tool_grant_config_at(
            "active",
            SMART_HOME_LIST_DEVICES_TOOL_ID,
            2_000,
            3_000,
        );
        assert!(matches!(
            provision_smart_home_tool_grants(
                &controller,
                future.data_plane().smart_home_tool_grants(),
                &TestUnixTimeClock::new(1_500)
            ),
            Err(ChiefDaemonError::SmartHomeGrantProvisioning)
        ));
        assert_eq!(controller.revision().unwrap(), None);
    }

    #[test]
    fn production_composition_provisions_grants_before_models_are_enabled() {
        let directory = TestDir::new();
        let config = configured_tool_grant_config("active", SMART_HOME_LIST_DEVICES_TOOL_ID);
        let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryStorageBackend::new());
        let monotonic: Arc<dyn MonotonicClock> = Arc::new(SystemMonotonicClock::new());
        let metadata: Arc<dyn MessageMetadataSource> =
            Arc::new(SystemMessageMetadataSource::new(monotonic));

        compose_data_plane_service(&config, &directory.0, backend, metadata).unwrap();

        let restored = SmartHomeControllerRuntime::restore(FsStorageBackend::new(
            directory.0.join(".chief-of-staff/state/"),
        ))
        .unwrap();
        let grant_id = CapabilityGrantId::trusted("grant-weather-list-devices");
        assert!(restored
            .runtime_handle()
            .lock()
            .unwrap()
            .registry()
            .capability_grant(&grant_id)
            .is_some());
    }

    fn configured_tool_grant_config(status: &str, tool_id: &str) -> ChiefConfig {
        configured_tool_grant_config_at(status, tool_id, 1_000, 2_000)
    }

    fn smart_home_listener_config() -> ChiefConfig {
        parse_config(&format!(
            "{VALID_CONFIG}\n[smart_home]\nbind = \"127.0.0.1\"\nport = 8123\ninstance_name = \"Chief Smart Home\"\n"
        ))
        .unwrap()
    }

    fn configured_tool_grant_config_at(
        status: &str,
        tool_id: &str,
        granted_at_ms: u64,
        expires_at_ms: u64,
    ) -> ChiefConfig {
        parse_config(&format!(
            "{VALID_CONFIG}\n[data_plane]\nchannel_keys = []\nollama_models = []\nsmart_home_tool_grants = [\n  {{ grant_id = \"grant-weather-list-devices\", principal_id = \"weather-level-one\", tool_id = \"{tool_id}\", granted_by = \"operator:test\", granted_at_ms = {granted_at_ms}, expires_at_ms = {expires_at_ms}, status = \"{status}\" }},\n]\n"
        ))
        .unwrap()
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
