use embeddable_http_server::HttpServerOptions;
use smart_home_automation_runtime::{
    AutomationAction, AutomationDefinition, AutomationTrigger, SmartHomeAutomationRuntime,
};
use smart_home_core::{AgentId, CommandType, EntityId, Value};
use smart_home_dashboard_core::{
    NativeDashboard, NativeDashboardCard, NativeDashboardCardKind, NativeDashboardManifest,
    NativeDashboardView, DASHBOARD_MANIFEST_SCHEMA_VERSION,
};
use smart_home_platform_http::{
    home_assistant_runtime_web_app, SmartHomePlatformHttpConfig, SmartHomePlatformHttpRuntime,
};
use smart_home_runtime::{RuntimePairingSession, RuntimePairingSessionId};
use smart_home_testkit::hue_lighting_runtime;
use std::env;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use web_core::WebServer;

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8123";
const DASHBOARD_PENDING_WRITE_BYTES: usize = 256 * 1024;
const USAGE: &str = "Usage: cargo run -p smart-home-platform-http --example hue_fixture_controller -- [BIND_ADDR]\n       cargo run -p smart-home-platform-http --example hue_fixture_controller -- --bind 127.0.0.1:8123";

fn main() -> Result<(), Box<dyn Error>> {
    let Some(bind_addr) = bind_addr_from_args(env::args().skip(1))? else {
        println!("{USAGE}");
        return Ok(());
    };
    let mut fixture = hue_lighting_runtime();
    let bridge = fixture
        .registry()
        .bridges()
        .next()
        .cloned()
        .ok_or_else(|| io::Error::other("fixture has no bridge"))?;
    fixture.start_pairing_session(RuntimePairingSession::pending(
        RuntimePairingSessionId::trusted("pairing-fixture-1"),
        &bridge,
        AgentId::trusted("agent:fixture-operator"),
        4_000,
        15_000,
        Vec::new(),
    ))?;

    let mut automations = SmartHomeAutomationRuntime::new();
    automations.upsert_definition(AutomationDefinition {
        automation_id: "fixture-nightly-off".to_string(),
        enabled: true,
        trigger: AutomationTrigger::Schedule {
            every_ms: 86_400_000,
            offset_ms: 0,
        },
        conditions: Vec::new(),
        actions: vec![AutomationAction::Command {
            entity_id: EntityId::trusted("entity-light-1"),
            command_type: CommandType::TurnOff,
            arguments: Value::Bool(false),
            timeout_ms: None,
        }],
    })?;

    let runtime = SmartHomePlatformHttpRuntime::new(
        fixture,
        SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
    )
    .with_now_ms(5_000)
    .with_automation_runtime(Arc::new(Mutex::new(automations)))
    .with_dashboard_manifest(fixture_dashboard_manifest())
    .grant_local_full_access("hue-fixture-controller", 1_000);
    let app = Arc::new(home_assistant_runtime_web_app(runtime));
    let options = dashboard_server_options();

    #[cfg(any(
        target_os = "macos",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "netbsd",
        target_os = "dragonfly"
    ))]
    let mut server = WebServer::bind_kqueue(&bind_addr, options, app)?;

    #[cfg(target_os = "linux")]
    let mut server = WebServer::bind_epoll(&bind_addr, options, app)?;

    #[cfg(target_os = "windows")]
    let mut server = WebServer::bind_windows(&bind_addr, options, app)?;

    println!("{}", launch_guide(server.local_addr()));
    server.serve()?;
    Ok(())
}

fn fixture_dashboard_manifest() -> NativeDashboardManifest {
    NativeDashboardManifest {
        schema_version: DASHBOARD_MANIFEST_SCHEMA_VERSION,
        source_instance_id: "fixture-home-assistant".to_string(),
        generated_at_ms: 5_000,
        dashboards: vec![NativeDashboard {
            dashboard_id: "fixture-overview".to_string(),
            url_path: "lovelace".to_string(),
            title: "Fixture Overview".to_string(),
            icon: None,
            require_admin: false,
            show_in_sidebar: true,
            views: vec![NativeDashboardView {
                view_id: "living-room".to_string(),
                title: "Living Room".to_string(),
                path: Some("living-room".to_string()),
                icon: None,
                cards: vec![NativeDashboardCard {
                    card_id: "living-room-light".to_string(),
                    kind: NativeDashboardCardKind::EntityControl,
                    source_type: "light".to_string(),
                    title: Some("Living Room Light".to_string()),
                    entity_ids: vec!["entity-light-1".to_string()],
                }],
            }],
        }],
        source_resources: Vec::new(),
    }
}

fn dashboard_server_options() -> HttpServerOptions {
    let mut options = HttpServerOptions::default();
    options.tcp.max_pending_write_bytes = DASHBOARD_PENDING_WRITE_BYTES;
    options
}

fn bind_addr_from_args(
    args: impl IntoIterator<Item = String>,
) -> Result<Option<String>, io::Error> {
    let mut bind_addr = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(None),
            "--bind" => {
                let Some(value) = args.next() else {
                    return Err(invalid_input("--bind requires an address"));
                };
                set_bind_addr(&mut bind_addr, value)?;
            }
            _ if arg.starts_with("--bind=") => {
                set_bind_addr(&mut bind_addr, arg["--bind=".len()..].to_string())?;
            }
            _ if arg.starts_with('-') => {
                return Err(invalid_input(format!("unknown option `{arg}`")));
            }
            _ => set_bind_addr(&mut bind_addr, arg)?,
        }
    }
    Ok(Some(
        bind_addr.unwrap_or_else(|| DEFAULT_BIND_ADDR.to_string()),
    ))
}

fn set_bind_addr(bind_addr: &mut Option<String>, value: String) -> Result<(), io::Error> {
    if bind_addr.is_some() {
        return Err(invalid_input("expected at most one bind address"));
    }
    *bind_addr = Some(value);
    Ok(())
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn launch_guide(local_addr: SocketAddr) -> String {
    let base_url = format!("http://{local_addr}");
    format!(
        "serving smart-home fixture controller\n  Dashboard: {base_url}/\n  Smart Home: {base_url}/smart-home\n  Health: {base_url}/api/smart_home/health\n  Readiness: {base_url}/api/smart_home/readiness\n  Controller handoff: {base_url}/api/smart_home/controller_handoff\n  Smoke plan: {base_url}/api/smart_home/smoke\n  Smoke script: {base_url}/api/smart_home/smoke_script\n  API catalog: {base_url}/api/smart_home/api\n\nSmoke commands:\n  curl {base_url}/api/smart_home/smoke\n  curl {base_url}/api/smart_home/smoke_script\n  SMART_HOME_BASE_URL={base_url} sh -c \"$(curl -fsS {base_url}/api/smart_home/smoke_script)\"\n  curl {base_url}/api/smart_home/controller_handoff\n  curl {base_url}/api/smart_home/bootstrap\n  curl {base_url}/api/smart_home/events?limit=12\n  curl -X POST {base_url}/api/services/light/turn_on -H 'Content-Type: application/json' -d '{{\"entity_id\":\"light.entity_light_1\",\"brightness_pct\":75}}'"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_addr_defaults_to_local_home_assistant_port() {
        assert_eq!(
            bind_addr_from_args(Vec::<String>::new()).expect("default bind addr"),
            Some(DEFAULT_BIND_ADDR.to_string())
        );
    }

    #[test]
    fn bind_addr_accepts_positional_and_flag_forms() {
        assert_eq!(
            bind_addr_from_args(["127.0.0.1:9999".to_string()]).expect("positional bind addr"),
            Some("127.0.0.1:9999".to_string())
        );
        assert_eq!(
            bind_addr_from_args(["--bind".to_string(), "127.0.0.1:7777".to_string()])
                .expect("flag bind addr"),
            Some("127.0.0.1:7777".to_string())
        );
        assert_eq!(
            bind_addr_from_args(["--bind=127.0.0.1:6666".to_string()])
                .expect("inline flag bind addr"),
            Some("127.0.0.1:6666".to_string())
        );
    }

    #[test]
    fn bind_addr_reports_help_without_starting_server() {
        assert_eq!(
            bind_addr_from_args(["--help".to_string()]).expect("help should parse"),
            None
        );
    }

    #[test]
    fn bind_addr_rejects_ambiguous_launches() {
        assert!(
            bind_addr_from_args(["127.0.0.1:1".to_string(), "127.0.0.1:2".to_string()]).is_err()
        );
        assert!(bind_addr_from_args(["--bind".to_string()]).is_err());
        assert!(bind_addr_from_args(["--unknown".to_string()]).is_err());
    }

    #[test]
    fn launch_guide_lists_dashboard_and_smoke_routes() {
        let guide = launch_guide("127.0.0.1:8123".parse().expect("socket addr"));
        assert!(guide.contains("Dashboard: http://127.0.0.1:8123/"));
        assert!(guide.contains("Smart Home: http://127.0.0.1:8123/smart-home"));
        assert!(guide.contains(
            "Controller handoff: http://127.0.0.1:8123/api/smart_home/controller_handoff"
        ));
        assert!(guide.contains("Smoke plan: http://127.0.0.1:8123/api/smart_home/smoke"));
        assert!(guide.contains("Smoke script: http://127.0.0.1:8123/api/smart_home/smoke_script"));
        assert!(guide.contains("curl http://127.0.0.1:8123/api/smart_home/smoke"));
        assert!(guide.contains("curl http://127.0.0.1:8123/api/smart_home/smoke_script"));
        assert!(guide.contains("SMART_HOME_BASE_URL=http://127.0.0.1:8123 sh -c"));
        assert!(guide.contains("curl http://127.0.0.1:8123/api/smart_home/controller_handoff"));
        assert!(guide.contains("curl http://127.0.0.1:8123/api/smart_home/bootstrap"));
        assert!(guide.contains("curl http://127.0.0.1:8123/api/smart_home/events?limit=12"));
        assert!(guide.contains("light.entity_light_1"));
        assert!(guide.contains("brightness_pct"));
    }
}
