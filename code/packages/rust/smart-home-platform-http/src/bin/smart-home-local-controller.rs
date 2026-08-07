use embeddable_http_server::HttpServerOptions;
use smart_home_automation_runtime::{AutomationTriggerInput, SmartHomeAutomationRuntime};
use smart_home_dashboard_core::parse_dashboard_manifest;
use smart_home_platform_http::{
    home_assistant_runtime_web_app, SmartHomePlatformHttpConfig, SmartHomePlatformHttpRuntime,
};
use smart_home_runtime::SmartHomeRuntime;
use smart_home_runtime_store::SmartHomeRuntimeStore;
use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use storage_local_folder::LocalFolderStorageBackend;
use web_core::WebServer;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8123";
const DEFAULT_DATA_DIR: &str = ".smart-home";
const DASHBOARD_PENDING_WRITE_BYTES: usize = 256 * 1024;
const USAGE: &str = "Usage: smart-home-local-controller [--bind ADDRESS] [--data-dir PATH] [--dashboard-manifest PATH]\n\
                       \n\
                       Options:\n\
                         --bind ADDRESS   Local listen address (default: 127.0.0.1:8123)\n\
                         --data-dir PATH  Durable runtime folder (default: SMART_HOME_DATA_DIR or .smart-home)\n\
                         --dashboard-manifest PATH  Applied native dashboard manifest (default: SMART_HOME_DASHBOARD_MANIFEST)\n\
                         -h, --help       Show this help";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControllerConfig {
    bind_addr: String,
    data_dir: PathBuf,
    dashboard_manifest: Option<PathBuf>,
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

    let store = Arc::new(SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(
        &config.data_dir,
    )));
    let restored = store.load()?;
    let (runtime, automation_definitions, automation_state, restored_at_ms) = match restored {
        Some(restored) => (
            restored.runtime,
            restored.automation_definitions,
            restored.automation_state,
            Some(restored.saved_at_ms),
        ),
        None => (SmartHomeRuntime::new(), Vec::new(), None, None),
    };
    let automation_runtime = Arc::new(Mutex::new(SmartHomeAutomationRuntime::restore(
        &automation_definitions,
        automation_state.as_ref(),
    )?));
    let shared_runtime = Arc::new(Mutex::new(runtime));

    let persistence_store = Arc::clone(&store);
    let persistence_automations = Arc::clone(&automation_runtime);
    let automation_persistence_store = Arc::clone(&store);
    let mut runtime = SmartHomePlatformHttpRuntime::from_shared_runtime(
        Arc::clone(&shared_runtime),
        SmartHomePlatformHttpConfig::new("Codex Home"),
    )
    .with_clock(unix_time_ms)
    .with_automation_runtime(Arc::clone(&automation_runtime))
    .with_mutation_persistence(move |runtime, saved_at_ms| {
        let automations = persistence_automations
            .lock()
            .map_err(|_| "automation runtime mutex was poisoned".to_string())?;
        let definitions = automations
            .durable_definitions()
            .map_err(|error| error.to_string())?;
        let state = automations
            .snapshot_json()
            .map_err(|error| error.to_string())?;
        persistence_store
            .save_with_automation_state(runtime, &definitions, Some(state), saved_at_ms)
            .map(|_| ())
            .map_err(|error| error.to_string())
    })
    .with_automation_persistence(move |runtime, automations, saved_at_ms| {
        let definitions = automations
            .durable_definitions()
            .map_err(|error| error.to_string())?;
        let state = automations
            .snapshot_json()
            .map_err(|error| error.to_string())?;
        automation_persistence_store
            .save_with_automation_state(runtime, &definitions, Some(state), saved_at_ms)
            .map(|_| ())
            .map_err(|error| error.to_string())
    })
    .grant_local_full_access("smart-home-local-controller", unix_time_ms());

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

    {
        let runtime = shared_runtime.lock().map_err(|_| {
            io::Error::other("smart-home runtime mutex was poisoned during startup")
        })?;
        let automations = automation_runtime.lock().map_err(|_| {
            io::Error::other("automation runtime mutex was poisoned during startup")
        })?;
        store.save_with_automation_state(
            &runtime,
            &automations.durable_definitions()?,
            Some(automations.snapshot_json()?),
            unix_time_ms(),
        )?;
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
        thread::sleep(Duration::from_millis(500));
        if let Err(error) = runtime.evaluate_automations(AutomationTriggerInput::Schedule, false) {
            eprintln!("smart-home automation schedule evaluation failed: {error}");
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
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    u64::try_from(milliseconds).unwrap_or(u64::MAX)
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

    fn test_default_dir() -> PathBuf {
        PathBuf::from("/tmp/smart-home-controller-test")
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
