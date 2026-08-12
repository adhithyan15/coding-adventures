use actor::ActorSystem;
use embeddable_http_server::HttpServerOptions;
use hue_core::{
    hue_discovery_worker_run_from_mdns_scan_report, HueError, HUE_INTEGRATION_ID,
    HUE_MDNS_SERVICE_TYPE,
};
use smart_home_automation_runtime::AutomationTriggerInput;
use smart_home_controller_runtime::SmartHomeControllerRuntime;
use smart_home_core::IntegrationId;
use smart_home_dashboard_core::parse_dashboard_manifest;
use smart_home_discovery::{
    DiscoverySource, DiscoveryWorkerId, DiscoveryWorkerKind, MdnsWorkerScanReport,
    UdpMdnsWorkerScanExecutor, MDNS_DISCOVERY_SERVICE_TYPE_METADATA_KEY,
};
use smart_home_discovery_service::{
    install_discovery_service_actor, DiscoveryServiceActorState, DiscoveryServiceTick,
};
use smart_home_platform_http::{
    home_assistant_runtime_web_app, SmartHomePlatformHttpConfig, SmartHomePlatformHttpRuntime,
};
use smart_home_runtime::{MdnsDiscoveryRunAdapter, ScheduledDiscoveryWorker};
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use storage_local_folder::LocalFolderStorageBackend;
use web_core::WebServer;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8123";
const DEFAULT_DATA_DIR: &str = ".smart-home";
const DASHBOARD_PENDING_WRITE_BYTES: usize = 256 * 1024;
const HUE_MDNS_ACTOR_ID: &str = "hue-mdns-discovery";
const HUE_MDNS_TICK_SENDER_ID: &str = "smart-home-local-controller";
const HUE_MDNS_WORKER_ID: &str = "hue-mdns";
const HUE_MDNS_INTERVAL_MS: u64 = 30_000;
const HUE_MDNS_RUN_TIMEOUT_MS: u64 = 2_000;
const HUE_MDNS_RETRY_DELAY_MS: u64 = 5_000;
const HUE_MDNS_TTL_MS: u64 = 120_000;
const WORKER_TICK_INTERVAL_MS: u64 = 500;
const USAGE: &str = "Usage: smart-home-local-controller [--bind ADDRESS] [--data-dir PATH] [--dashboard-manifest PATH] [--hue-mdns-interface NAME]\n\
                       \n\
                       Options:\n\
                         --bind ADDRESS   Local listen address (default: 127.0.0.1:8123)\n\
                         --data-dir PATH  Durable runtime folder (default: SMART_HOME_DATA_DIR or .smart-home)\n\
                         --dashboard-manifest PATH  Applied native dashboard manifest (default: SMART_HOME_DASHBOARD_MANIFEST)\n\
                         --hue-mdns-interface NAME  Enable supervised Hue mDNS on this interface\n\
                         -h, --help       Show this help";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControllerConfig {
    bind_addr: String,
    data_dir: PathBuf,
    dashboard_manifest: Option<PathBuf>,
    hue_mdns_interface: Option<String>,
}

type HueDiscoveryService = DiscoveryServiceActorState<
    LocalFolderStorageBackend,
    LocalFolderStorageBackend,
    UdpMdnsWorkerScanExecutor,
    HueMdnsRunAdapter,
>;

#[derive(Debug, Default)]
struct HueMdnsRunAdapter;

impl MdnsDiscoveryRunAdapter for HueMdnsRunAdapter {
    type Error = HueError;

    fn worker_run_from_mdns_scan_report(
        &mut self,
        report: &MdnsWorkerScanReport,
    ) -> Result<smart_home_discovery::DiscoveryWorkerRun, Self::Error> {
        hue_discovery_worker_run_from_mdns_scan_report(report)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let Some(config) = config_from_args(
        env::args().skip(1),
        default_data_dir(),
        env::var_os("SMART_HOME_DASHBOARD_MANIFEST").map(PathBuf::from),
    )?
    else {
        println!("{USAGE}");
        return Ok(());
    };

    let controller =
        SmartHomeControllerRuntime::restore(LocalFolderStorageBackend::new(&config.data_dir))?;
    let restored_at_ms = controller.restored_at_ms();
    let shared_runtime = controller.runtime_handle();
    let automation_runtime = controller.automation_runtime_handle();
    let mut runtime = SmartHomePlatformHttpRuntime::from_shared_runtime(
        Arc::clone(&shared_runtime),
        SmartHomePlatformHttpConfig::new("Codex Home"),
    )
    .with_fallible_clock(try_unix_time_ms)
    .with_automation_runtime(Arc::clone(&automation_runtime))
    .with_mutation_persistence(controller.runtime_persistence_adapter())
    .with_automation_persistence(controller.automation_persistence_adapter())
    .grant_local_full_access(
        "smart-home-local-controller",
        try_unix_time_ms().ok_or_else(|| invalid_input("system clock is unavailable"))?,
    );

    if let Some(path) = config.dashboard_manifest.as_ref() {
        let bytes = fs::read(path).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "could not read dashboard manifest {}: {error}",
                    path.display()
                ),
            )
        })?;
        let manifest = parse_dashboard_manifest(&bytes).map_err(io::Error::other)?;
        runtime = runtime.with_dashboard_manifest(manifest);
    }

    controller.save_snapshot(unix_time_ms())?;

    if let Some(interface) = config.hue_mdns_interface.as_deref() {
        let discovery_service = configure_hue_mdns_discovery(
            controller.clone(),
            &config.data_dir,
            interface,
            unix_time_ms(),
        )?;
        spawn_discovery_worker(discovery_service);
    }

    let automation_count = automation_runtime
        .lock()
        .map_err(|_| io::Error::other("automation runtime mutex was poisoned during startup"))?
        .definitions()
        .count();
    spawn_schedule_worker(runtime.clone());
    let app = Arc::new(home_assistant_runtime_web_app(runtime));
    let options = dashboard_server_options();

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    let mut server = WebServer::bind_kqueue(&config.bind_addr, options, app)?;

    #[cfg(target_os = "linux")]
    let mut server = WebServer::bind_epoll(&config.bind_addr, options, app)?;

    #[cfg(target_os = "windows")]
    let mut server = WebServer::bind_windows(&config.bind_addr, options, app)?;

    println!(
        "{}",
        launch_guide(
            server.local_addr(),
            &config.data_dir,
            restored_at_ms,
            automation_count,
        )
    );
    server.serve()?;
    Ok(())
}

fn spawn_schedule_worker(runtime: SmartHomePlatformHttpRuntime) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(WORKER_TICK_INTERVAL_MS));
        if let Err(error) = runtime.evaluate_automations(AutomationTriggerInput::Schedule, false) {
            eprintln!("smart-home automation schedule evaluation failed: {error}");
        }
    });
}

fn configure_hue_mdns_discovery(
    controller: SmartHomeControllerRuntime<LocalFolderStorageBackend>,
    data_dir: &std::path::Path,
    interface: &str,
    now_ms: u64,
) -> Result<HueDiscoveryService, Box<dyn Error>> {
    let mut service = DiscoveryServiceActorState::open(
        controller,
        LocalFolderStorageBackend::new(data_dir),
        UdpMdnsWorkerScanExecutor,
        HueMdnsRunAdapter,
        HUE_MDNS_TTL_MS,
        now_ms,
    )?;
    let worker_id = DiscoveryWorkerId::trusted(HUE_MDNS_WORKER_ID);
    let worker = ScheduledDiscoveryWorker::new(
        worker_id.clone(),
        IntegrationId::trusted(HUE_INTEGRATION_ID),
        DiscoveryWorkerKind::MdnsScan,
        HUE_MDNS_INTERVAL_MS,
        HUE_MDNS_RUN_TIMEOUT_MS,
        now_ms,
    )
    .with_source(DiscoverySource::Mdns)
    .with_network_interface(interface)
    .with_retry_backoff(HUE_MDNS_RETRY_DELAY_MS, HUE_MDNS_INTERVAL_MS, 2)
    .with_metadata(
        MDNS_DISCOVERY_SERVICE_TYPE_METADATA_KEY,
        HUE_MDNS_SERVICE_TYPE,
    );
    let configuration_matches = {
        let runtime = service.runtime_handle();
        let runtime = runtime.lock().map_err(|_| {
            io::Error::other("smart-home runtime mutex was poisoned during Hue discovery setup")
        })?;
        runtime
            .discovery_worker_schedule(&worker_id)
            .is_some_and(|existing| hue_worker_configuration_matches(existing, &worker))
    };
    if !configuration_matches {
        service.register_worker(worker, now_ms)?;
    }
    Ok(service)
}

fn hue_worker_configuration_matches(
    existing: &ScheduledDiscoveryWorker,
    desired: &ScheduledDiscoveryWorker,
) -> bool {
    existing.integration_id == desired.integration_id
        && existing.kind == desired.kind
        && existing.sources == desired.sources
        && existing.network_interfaces == desired.network_interfaces
        && existing.interval_ms == desired.interval_ms
        && existing.run_timeout_ms == desired.run_timeout_ms
        && existing.retry_delay_ms == desired.retry_delay_ms
        && existing.max_retry_delay_ms == desired.max_retry_delay_ms
        && existing.retry_backoff_multiplier == desired.retry_backoff_multiplier
        && existing.metadata == desired.metadata
}

fn spawn_discovery_worker(service: HueDiscoveryService) {
    thread::spawn(move || {
        let mut system = ActorSystem::new();
        if let Err(error) = install_discovery_service_actor(&mut system, HUE_MDNS_ACTOR_ID, service)
        {
            eprintln!("smart-home Hue mDNS actor installation failed: {error}");
            return;
        }
        loop {
            thread::sleep(Duration::from_millis(WORKER_TICK_INTERVAL_MS));
            let now_ms = unix_time_ms();
            let message = match DiscoveryServiceTick::new(now_ms, now_ms)
                .and_then(|tick| tick.into_message(HUE_MDNS_TICK_SENDER_ID))
            {
                Ok(message) => message,
                Err(error) => {
                    eprintln!("smart-home Hue mDNS tick construction failed: {error}");
                    continue;
                }
            };
            if let Err(error) = system.send(HUE_MDNS_ACTOR_ID, message) {
                eprintln!("smart-home Hue mDNS tick delivery failed: {error}");
                return;
            }
            if let Err(error) = system.process_next(HUE_MDNS_ACTOR_ID) {
                eprintln!("smart-home Hue mDNS actor processing failed: {error}");
                return;
            }
        }
    });
}

fn default_data_dir() -> PathBuf {
    env::var_os("SMART_HOME_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_DATA_DIR))
}

fn config_from_args(
    args: impl IntoIterator<Item = String>,
    default_data_dir: PathBuf,
    default_dashboard_manifest: Option<PathBuf>,
) -> Result<Option<ControllerConfig>, io::Error> {
    let mut bind_addr = DEFAULT_BIND_ADDR.to_string();
    let mut data_dir = default_data_dir;
    let mut dashboard_manifest = default_dashboard_manifest;
    let mut hue_mdns_interface = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--bind" => {
                bind_addr = required_value(&mut args, "--bind")?;
            }
            "--data-dir" => {
                data_dir = PathBuf::from(required_value(&mut args, "--data-dir")?);
            }
            "--dashboard-manifest" => {
                dashboard_manifest = Some(PathBuf::from(required_value(
                    &mut args,
                    "--dashboard-manifest",
                )?));
            }
            "--hue-mdns-interface" => {
                hue_mdns_interface = Some(required_value(&mut args, "--hue-mdns-interface")?);
            }
            _ if arg.starts_with("--bind=") => {
                bind_addr = non_empty_value(&arg["--bind=".len()..], "--bind")?.to_string();
            }
            _ if arg.starts_with("--data-dir=") => {
                data_dir =
                    PathBuf::from(non_empty_value(&arg["--data-dir=".len()..], "--data-dir")?);
            }
            _ if arg.starts_with("--dashboard-manifest=") => {
                dashboard_manifest = Some(PathBuf::from(non_empty_value(
                    &arg["--dashboard-manifest=".len()..],
                    "--dashboard-manifest",
                )?));
            }
            _ if arg.starts_with("--hue-mdns-interface=") => {
                hue_mdns_interface = Some(
                    non_empty_value(
                        &arg["--hue-mdns-interface=".len()..],
                        "--hue-mdns-interface",
                    )?
                    .to_string(),
                );
            }
            _ => return Err(invalid_input(format!("unknown argument `{arg}`"))),
        }
    }
    if data_dir.as_os_str().is_empty() {
        return Err(invalid_input("--data-dir requires a non-empty path"));
    }
    Ok(Some(ControllerConfig {
        bind_addr,
        data_dir,
        dashboard_manifest,
        hue_mdns_interface,
    }))
}

fn required_value(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, io::Error> {
    let value = args
        .next()
        .ok_or_else(|| invalid_input(format!("{option} requires a value")))?;
    non_empty_value(&value, option)?;
    Ok(value)
}

fn non_empty_value<'a>(value: &'a str, option: &str) -> Result<&'a str, io::Error> {
    if value.is_empty() {
        Err(invalid_input(format!(
            "{option} requires a non-empty value"
        )))
    } else {
        Ok(value)
    }
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn unix_time_ms() -> u64 {
    try_unix_time_ms().expect("system clock should be available while workers are running")
}

fn try_unix_time_ms() -> Option<u64> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    u64::try_from(milliseconds).ok()
}

fn dashboard_server_options() -> HttpServerOptions {
    let mut options = HttpServerOptions::default();
    options.tcp.max_pending_write_bytes = DASHBOARD_PENDING_WRITE_BYTES;
    options
}

fn launch_guide(
    local_addr: SocketAddr,
    data_dir: &std::path::Path,
    restored_at_ms: Option<u64>,
    automation_count: usize,
) -> String {
    let base_url = format!("http://{local_addr}");
    let restore = restored_at_ms
        .map(|saved_at_ms| format!("restored snapshot saved at {saved_at_ms} ms"))
        .unwrap_or_else(|| "initialized a new runtime snapshot".to_string());
    format!(
        "serving durable smart-home local controller\n  Dashboard: {base_url}/\n  API: {base_url}/api/smart_home/api\n  Health: {base_url}/api/smart_home/health\n  Data: {}\n  Restore: {restore}\n  Durable automations: {automation_count}",
        data_dir.display(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_automation_runtime::{
        AutomationAction, AutomationAuditOutcome, AutomationDefinition, AutomationTrigger,
    };
    use smart_home_core::{CommandType, EntityId, Value};

    fn test_default_dir() -> PathBuf {
        PathBuf::from("/tmp/smart-home-controller-test")
    }

    fn temporary_data_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("test clock should be after unix epoch")
            .as_nanos();
        env::temp_dir().join(format!(
            "smart-home-local-controller-{}-{name}-{nanos}",
            std::process::id()
        ))
    }

    #[test]
    fn central_controller_adapters_share_and_restore_automation_state() {
        let data_dir = temporary_data_dir("central-owner");
        let controller =
            SmartHomeControllerRuntime::restore(LocalFolderStorageBackend::new(&data_dir))
                .expect("central controller should start");
        let runtime = SmartHomePlatformHttpRuntime::from_shared_runtime(
            controller.runtime_handle(),
            SmartHomePlatformHttpConfig::new("Test Home"),
        )
        .with_now_ms(1_000)
        .with_automation_runtime(controller.automation_runtime_handle())
        .with_mutation_persistence(controller.runtime_persistence_adapter())
        .with_automation_persistence(controller.automation_persistence_adapter())
        .grant_local_full_access("controller-test", 900);
        controller
            .save_snapshot(900)
            .expect("startup state should persist");

        runtime
            .upsert_automation_definition(AutomationDefinition {
                automation_id: "central-schedule".to_string(),
                enabled: true,
                trigger: AutomationTrigger::Schedule {
                    every_ms: 1_000,
                    offset_ms: 0,
                },
                conditions: Vec::new(),
                actions: vec![AutomationAction::Command {
                    entity_id: EntityId::trusted("missing-entity"),
                    command_type: CommandType::TurnOn,
                    arguments: Value::Null,
                    timeout_ms: None,
                }],
            })
            .expect("HTTP adapter should update the central automation owner");
        let report = runtime
            .evaluate_automations(AutomationTriggerInput::Schedule, false)
            .expect("schedule worker should use the central owner");
        assert_eq!(report.records.len(), 1);
        assert_eq!(report.records[0].outcome, AutomationAuditOutcome::Failed);
        assert_eq!(
            controller
                .automation_runtime_handle()
                .lock()
                .expect("automation state")
                .snapshot()
                .audit_records
                .len(),
            1
        );
        drop(runtime);
        drop(controller);

        let restored =
            SmartHomeControllerRuntime::restore(LocalFolderStorageBackend::new(&data_dir))
                .expect("central state should restore");
        let automations = restored.automation_runtime_handle();
        let automations = automations.lock().expect("restored automation state");
        assert_eq!(automations.definitions().count(), 1);
        assert_eq!(automations.snapshot().audit_records.len(), 1);

        fs::remove_dir_all(data_dir).expect("temporary controller data should be removable");
    }

    #[test]
    fn config_defaults_to_loopback_and_supplied_data_dir() {
        assert_eq!(
            config_from_args(Vec::<String>::new(), test_default_dir(), None)
                .expect("default config"),
            Some(ControllerConfig {
                bind_addr: DEFAULT_BIND_ADDR.to_string(),
                data_dir: test_default_dir(),
                dashboard_manifest: None,
                hue_mdns_interface: None,
            })
        );
    }

    #[test]
    fn config_accepts_explicit_bind_and_data_dir() {
        assert_eq!(
            config_from_args(
                [
                    "--bind=127.0.0.1:9123".to_string(),
                    "--data-dir".to_string(),
                    "/tmp/codex-home".to_string(),
                ],
                test_default_dir(),
                None,
            )
            .expect("explicit config"),
            Some(ControllerConfig {
                bind_addr: "127.0.0.1:9123".to_string(),
                data_dir: PathBuf::from("/tmp/codex-home"),
                dashboard_manifest: None,
                hue_mdns_interface: None,
            })
        );
    }

    #[test]
    fn config_accepts_dashboard_manifest_argument_and_default() {
        assert_eq!(
            config_from_args(
                ["--dashboard-manifest=/tmp/dashboard.json".to_string()],
                test_default_dir(),
                None,
            )
            .expect("manifest config")
            .unwrap()
            .dashboard_manifest,
            Some(PathBuf::from("/tmp/dashboard.json"))
        );
        assert_eq!(
            config_from_args(
                Vec::<String>::new(),
                test_default_dir(),
                Some(PathBuf::from("/tmp/default-dashboard.json")),
            )
            .expect("default manifest config")
            .unwrap()
            .dashboard_manifest,
            Some(PathBuf::from("/tmp/default-dashboard.json"))
        );
    }

    #[test]
    fn config_accepts_explicit_hue_mdns_interface() {
        let config = config_from_args(
            ["--hue-mdns-interface".to_string(), "en7".to_string()],
            test_default_dir(),
            None,
        )
        .expect("mDNS interface config")
        .unwrap();
        assert_eq!(config.hue_mdns_interface.as_deref(), Some("en7"));

        let config = config_from_args(
            ["--hue-mdns-interface=en0".to_string()],
            test_default_dir(),
            None,
        )
        .expect("equals-form mDNS interface config")
        .unwrap();
        assert_eq!(config.hue_mdns_interface.as_deref(), Some("en0"));
    }

    #[test]
    fn hue_mdns_worker_is_owned_by_the_central_runtime() {
        let data_dir = temporary_data_dir("hue-mdns-owner");
        let controller =
            SmartHomeControllerRuntime::restore(LocalFolderStorageBackend::new(&data_dir))
                .expect("central controller should start");
        let service = configure_hue_mdns_discovery(controller.clone(), &data_dir, "en7", 1_000)
            .expect("Hue mDNS service should configure");

        let worker_id = DiscoveryWorkerId::trusted(HUE_MDNS_WORKER_ID);
        let runtime = controller.runtime_handle();
        let runtime = runtime.lock().expect("controller runtime");
        let worker = runtime
            .discovery_worker_schedule(&worker_id)
            .expect("central runtime should own the Hue worker");
        assert_eq!(worker.network_interfaces, vec!["en7".to_string()]);
        drop(runtime);
        let first_revision = controller.revision().expect("controller revision");

        drop(service);
        let service = configure_hue_mdns_discovery(controller.clone(), &data_dir, "en7", 2_000)
            .expect("matching Hue mDNS config should reopen");
        assert_eq!(
            controller.revision().expect("unchanged revision"),
            first_revision
        );
        drop(service);

        let service = configure_hue_mdns_discovery(controller.clone(), &data_dir, "en0", 3_000)
            .expect("changed Hue mDNS interface should update");
        let runtime = service.runtime_handle();
        let runtime = runtime.lock().expect("updated runtime");
        let worker = runtime
            .discovery_worker_schedule(&worker_id)
            .expect("updated Hue worker");
        assert_eq!(worker.network_interfaces, vec!["en0".to_string()]);
        assert_eq!(worker.next_due_at_ms, 3_000);

        drop(runtime);
        drop(service);
        drop(controller);
        fs::remove_dir_all(data_dir).expect("temporary controller data should be removable");
    }

    #[test]
    fn config_handles_help_and_rejects_invalid_options() {
        assert_eq!(
            config_from_args(["--help".to_string()], test_default_dir(), None).expect("help"),
            None
        );
        assert!(config_from_args(["--bind".to_string()], test_default_dir(), None).is_err());
        assert!(config_from_args(["--data-dir=".to_string()], test_default_dir(), None).is_err());
        assert!(config_from_args(
            ["--dashboard-manifest=".to_string()],
            test_default_dir(),
            None,
        )
        .is_err());
        assert!(config_from_args(
            ["--hue-mdns-interface=".to_string()],
            test_default_dir(),
            None,
        )
        .is_err());
        assert!(config_from_args(["--unknown".to_string()], test_default_dir(), None).is_err());
    }

    #[test]
    fn launch_guide_reports_durable_restore_context() {
        let guide = launch_guide(
            "127.0.0.1:8123".parse().expect("socket address"),
            std::path::Path::new("/tmp/codex-home"),
            Some(42),
            3,
        );
        assert!(guide.contains("durable smart-home local controller"));
        assert!(guide.contains("http://127.0.0.1:8123/api/smart_home/api"));
        assert!(guide.contains("Data: /tmp/codex-home"));
        assert!(guide.contains("restored snapshot saved at 42 ms"));
        assert!(guide.contains("Durable automations: 3"));
    }
}
