use serde_json::{json, Value as JsonValue};
use smart_home_home_assistant_dashboard_migration::{
    migrate_live_dashboards, DashboardCollectorConfig, DashboardMigrationArtifact,
};
use smart_home_home_assistant_migration::{
    HomeAssistantEntity, HomeAssistantExport, EXPORT_SCHEMA_VERSION,
};
use std::fs;
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tungstenite::{accept, Message, WebSocket};

#[test]
fn live_lovelace_collection_migrates_dashboards_and_resources() {
    let topology = topology();
    let (url, server) = fixture_server("dashboard-secret", false, false);
    let artifact =
        migrate_live_dashboards(&topology, &config(url), false).expect("migrate live dashboards");
    server.join().expect("fixture server");

    assert!(!artifact.dry_run);
    assert!(artifact.receipt.is_some());
    assert_eq!(artifact.plan.summary.dashboards_discovered, 2);
    assert_eq!(artifact.plan.summary.dashboards_collected, 2);
    assert_eq!(artifact.plan.summary.views, 2);
    assert_eq!(artifact.plan.summary.cards_migrated, 3);
    assert_eq!(artifact.plan.summary.resources, 1);
    assert_eq!(artifact.plan.manifest.dashboards[0].url_path, "energy");
    assert_eq!(artifact.plan.manifest.dashboards[1].url_path, "lovelace");
    assert!(artifact.plan.diagnostics.iter().any(|item| {
        item.code == "external_resource_requires_manual_review"
            && item.message.contains("/local/custom.js")
    }));
}

#[test]
fn repeated_collection_has_stable_source_fingerprint() {
    let topology = topology();
    let (first_url, first_server) = fixture_server("dashboard-secret", false, false);
    let first =
        migrate_live_dashboards(&topology, &config(first_url), true).expect("first dashboard plan");
    first_server.join().expect("first fixture server");
    let (second_url, second_server) = fixture_server("dashboard-secret", false, false);
    let mut second_config = config(second_url);
    second_config.collected_at_ms += 10_000;
    let second =
        migrate_live_dashboards(&topology, &second_config, true).expect("second dashboard plan");
    second_server.join().expect("second fixture server");

    assert_eq!(
        first.plan.source_fingerprint,
        second.plan.source_fingerprint
    );
    assert_eq!(
        first.plan.manifest.dashboards,
        second.plan.manifest.dashboards
    );
    assert_ne!(first.plan.collected_at_ms, second.plan.collected_at_ms);
}

#[test]
fn reviewed_topology_changes_source_fingerprint() {
    let topology = topology();
    let (first_url, first_server) = fixture_server("dashboard-secret", false, false);
    let first =
        migrate_live_dashboards(&topology, &config(first_url), true).expect("first dashboard plan");
    first_server.join().expect("first fixture server");

    let mut changed_topology = topology;
    changed_topology.entities[0].disabled_by = Some("user".to_string());
    let (second_url, second_server) = fixture_server("dashboard-secret", false, false);
    let second = migrate_live_dashboards(&changed_topology, &config(second_url), true)
        .expect("changed topology plan");
    second_server.join().expect("second fixture server");

    assert_ne!(
        first.plan.source_fingerprint,
        second.plan.source_fingerprint
    );
    assert!(second.plan.summary.cards_migrated < first.plan.summary.cards_migrated);
}

#[test]
fn rejected_authentication_does_not_echo_token() {
    let topology = topology();
    let (url, server) = fixture_server("dashboard-secret", true, false);
    let error = migrate_live_dashboards(&topology, &config(url), true)
        .expect_err("authentication should fail");
    server.join().expect("fixture server");

    let message = error.to_string();
    assert!(message.contains("rejected"));
    assert!(!message.contains("dashboard-secret"));
}

#[test]
fn unavailable_listed_dashboard_is_reviewable_in_dry_run_and_blocks_apply() {
    let topology = topology();
    let (dry_url, dry_server) = fixture_server("dashboard-secret", false, true);
    let dry_run = migrate_live_dashboards(&topology, &config(dry_url), true)
        .expect("dry run should retain dashboard error");
    dry_server.join().expect("dry-run fixture server");
    assert!(dry_run.plan.is_blocked());
    assert_eq!(dry_run.plan.summary.dashboards_discovered, 2);
    assert_eq!(dry_run.plan.summary.dashboards_collected, 1);
    assert!(dry_run
        .plan
        .diagnostics
        .iter()
        .any(|item| item.code == "dashboard_config_unavailable"));

    let (apply_url, apply_server) = fixture_server("dashboard-secret", false, true);
    let error = migrate_live_dashboards(&topology, &config(apply_url), false)
        .expect_err("applied migration should block");
    apply_server.join().expect("apply fixture server");
    assert!(error
        .to_string()
        .contains("1 collection or validation errors"));
}

#[test]
fn cli_writes_applied_artifact_atomically() {
    let directory = std::env::temp_dir().join(format!(
        "smart-home-ha-dashboard-cli-{}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("create test directory");
    let topology_path = directory.join("topology.json");
    let output_path = directory.join("dashboard.json");
    fs::write(
        &topology_path,
        serde_json::to_vec_pretty(&topology()).expect("encode topology"),
    )
    .expect("write topology");
    let (url, server) = fixture_server("process-secret", false, false);

    let output = Command::new(env!(
        "CARGO_BIN_EXE_smart-home-migrate-home-assistant-dashboards"
    ))
    .arg(&topology_path)
    .arg(url)
    .arg(&output_path)
    .arg("--timeout-ms")
    .arg("2000")
    .arg("--collected-at-ms")
    .arg("1767312000000")
    .env("HOME_ASSISTANT_TOKEN", "process-secret")
    .output()
    .expect("run dashboard CLI");
    server.join().expect("fixture server");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("3/3 cards"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("process-secret"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("process-secret"));
    let artifact: DashboardMigrationArtifact =
        serde_json::from_slice(&fs::read(&output_path).expect("read artifact"))
            .expect("decode artifact");
    assert_eq!(artifact.plan.summary.dashboards_collected, 2);
    assert!(artifact.receipt.is_some());
    assert!(!directory.join(".dashboard.json.tmp").exists());

    fs::remove_file(topology_path).expect("remove topology");
    fs::remove_file(output_path).expect("remove output");
    fs::remove_dir(directory).expect("remove test directory");
}

fn fixture_server(
    expected_token: &'static str,
    reject_auth: bool,
    fail_energy_config: bool,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let address = listener.local_addr().expect("fixture address");
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept fixture");
        let mut socket = accept(stream).expect("upgrade fixture");
        send(&mut socket, json!({"type": "auth_required"}));
        let auth = read(&mut socket);
        assert_eq!(auth["type"], "auth");
        assert_eq!(auth["access_token"], expected_token);
        if reject_auth {
            send(&mut socket, json!({"type": "auth_invalid"}));
            return;
        }
        send(&mut socket, json!({"type": "auth_ok"}));

        let dashboards = read(&mut socket);
        assert_eq!(dashboards["id"], 1);
        assert_eq!(dashboards["type"], "lovelace/dashboards/list");
        send(
            &mut socket,
            result(
                1,
                json!([
                    {"id": "overview", "url_path": "lovelace", "title": "Overview", "show_in_sidebar": true},
                    {"id": "energy", "url_path": "energy", "title": "Energy", "icon": "mdi:flash", "require_admin": true}
                ]),
            ),
        );

        let resources = read(&mut socket);
        assert_eq!(resources["id"], 2);
        assert_eq!(resources["type"], "lovelace/resources/list");
        send(
            &mut socket,
            result(
                2,
                json!([{"id": "custom", "url": "/local/custom.js", "res_type": "module"}]),
            ),
        );

        let energy = read(&mut socket);
        assert_eq!(energy["id"], 3);
        assert_eq!(energy["url_path"], "energy");
        if fail_energy_config {
            send(
                &mut socket,
                json!({"id": 3, "type": "result", "success": false, "error": {"code": "config_not_found", "message": "No config found"}}),
            );
        } else {
            send(
                &mut socket,
                result(
                    3,
                    json!({"views": [{"title": "Power", "cards": [
                        {"type": "history-graph", "entities": ["sensor.temperature"]}
                    ]}]}),
                ),
            );
        }

        let overview = read(&mut socket);
        assert_eq!(overview["id"], 4);
        assert_eq!(overview["url_path"], "lovelace");
        send(
            &mut socket,
            result(
                4,
                json!({"views": [{"title": "Home", "cards": [
                    {"type": "entities", "entities": ["light.kitchen", "sensor.temperature"]},
                    {"type": "light", "entity": "light.kitchen"}
                ]}]}),
            ),
        );
    });
    (format!("ws://{address}/api/websocket"), handle)
}

fn result(id: u64, result: JsonValue) -> JsonValue {
    json!({"id": id, "type": "result", "success": true, "result": result})
}

fn send(socket: &mut WebSocket<std::net::TcpStream>, value: JsonValue) {
    socket
        .send(Message::Text(value.to_string().into()))
        .expect("send fixture response");
}

fn read(socket: &mut WebSocket<std::net::TcpStream>) -> JsonValue {
    let message = socket.read().expect("read fixture request");
    serde_json::from_str(message.to_text().expect("text fixture request"))
        .expect("fixture request JSON")
}

fn config(websocket_url: String) -> DashboardCollectorConfig {
    DashboardCollectorConfig {
        websocket_url,
        access_token: "dashboard-secret".to_string(),
        source_instance_id: "home-1".to_string(),
        collected_at_ms: 1_767_312_000_000,
        io_timeout: Duration::from_secs(2),
    }
}

fn topology() -> HomeAssistantExport {
    HomeAssistantExport {
        schema_version: EXPORT_SCHEMA_VERSION,
        source_instance_id: "home-1".to_string(),
        exported_at_ms: 1,
        areas: Vec::new(),
        devices: Vec::new(),
        entities: vec![entity("light.kitchen"), entity("sensor.temperature")],
        states: Vec::new(),
        scenes: Vec::new(),
        automations: Vec::new(),
    }
}

fn entity(entity_id: &str) -> HomeAssistantEntity {
    HomeAssistantEntity {
        entity_id: entity_id.to_string(),
        device_id: None,
        area_id: None,
        platform: "fixture".to_string(),
        unique_id: entity_id.to_string(),
        name: None,
        original_name: None,
        disabled_by: None,
    }
}
