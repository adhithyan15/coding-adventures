use serde_json::{json, Value as JsonValue};
use smart_home_automation_runtime::SmartHomeAutomationRuntime;
use smart_home_home_assistant_history::{
    apply_history_plan, migrate_live_history, plan_history, HistoryCollectorConfig,
    HomeAssistantHistoryMigrationArtifact,
};
use smart_home_home_assistant_migration::{
    apply_plan, plan_export, HomeAssistantArea, HomeAssistantDevice, HomeAssistantEntity,
    HomeAssistantExport, HomeAssistantState, EXPORT_SCHEMA_VERSION,
};
use smart_home_runtime::SmartHomeRuntime;
use std::collections::BTreeMap;
use std::fs;
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use std::time::Duration;
use tungstenite::{accept, Message, WebSocket};

#[test]
fn live_history_migrates_to_durable_events_and_preserves_current_state() {
    let topology = fixture_topology();
    let (url, server) = fixture_server("history-secret", true, 2);
    let artifact =
        migrate_live_history(&topology, &config(url, 1), false).expect("migrate live history");
    server.join().expect("fixture server");

    assert!(!artifact.dry_run);
    assert_eq!(artifact.history_plan.summary.entities_requested, 2);
    assert_eq!(artifact.history_plan.summary.entities_with_history, 2);
    assert_eq!(artifact.history_plan.summary.source_states, 3);
    assert_eq!(artifact.history_plan.summary.planned_events, 5);
    assert_eq!(artifact.history_plan.summary.errors, 0);
    let receipt = artifact.history_receipt.as_ref().expect("history receipt");
    assert_eq!(receipt.counts.inserted_events, 5);
    assert_eq!(receipt.counts.skipped_identical_events, 0);
    assert_eq!(receipt.counts.restored_current_states, 2);

    let snapshot = artifact
        .runtime_snapshot
        .as_ref()
        .expect("runtime snapshot");
    assert_eq!(snapshot.registry_events.len(), 5);
    assert_eq!(snapshot.runtime_events.len(), 5);
    assert!(snapshot
        .registry_events
        .windows(2)
        .all(|events| events[0].observed_at_ms <= events[1].observed_at_ms));
    let current_light = snapshot
        .states
        .iter()
        .find(|state| state.entity_id.as_str() == "ha:light.kitchen")
        .expect("current light state");
    let planned_light = artifact
        .topology_plan
        .entities
        .iter()
        .find(|entity| entity.entity_id.as_str() == "ha:light.kitchen")
        .and_then(|entity| entity.state.as_ref())
        .expect("planned light state");
    assert_eq!(current_light, planned_light);
}

#[test]
fn history_apply_is_idempotent() {
    let topology_export = fixture_topology();
    let (url, server) = fixture_server("history-secret", true, 1);
    let artifact =
        migrate_live_history(&topology_export, &config(url, 100), true).expect("plan live history");
    server.join().expect("fixture server");

    let topology = plan_export(&topology_export).expect("plan topology");
    let mut runtime = SmartHomeRuntime::new();
    let mut automations = SmartHomeAutomationRuntime::new();
    apply_plan(&topology, &mut runtime, &mut automations).expect("apply topology");
    let first = apply_history_plan(&artifact.history_plan, &topology, &mut runtime)
        .expect("first history apply");
    let mut recollected = artifact.history_export.clone();
    recollected.collected_at_ms += 60_000;
    let recollected_plan = plan_history(&recollected, &topology).expect("plan recollection");
    assert_ne!(
        artifact.history_plan.source_fingerprint,
        recollected_plan.source_fingerprint
    );
    assert_eq!(artifact.history_plan.events, recollected_plan.events);
    let second = apply_history_plan(&recollected_plan, &topology, &mut runtime)
        .expect("second history apply");

    assert_eq!(first.counts.inserted_events, 5);
    assert_eq!(second.counts.inserted_events, 0);
    assert_eq!(second.counts.skipped_identical_events, 5);
    assert_eq!(runtime.durable_snapshot().registry_events.len(), 5);
}

#[test]
fn rejected_auth_does_not_echo_token() {
    let topology = fixture_topology();
    let (url, server) = fixture_server("history-secret", false, 0);
    let error = migrate_live_history(&topology, &config(url, 100), true)
        .expect_err("authentication should fail");
    server.join().expect("fixture server");

    let message = error.to_string();
    assert!(message.contains("rejected"));
    assert!(!message.contains("history-secret"));
}

#[test]
fn cli_writes_applied_history_artifact() {
    let topology = fixture_topology();
    let directory =
        std::env::temp_dir().join(format!("smart-home-ha-history-cli-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("create test directory");
    let topology_path = directory.join("topology.json");
    let output_path = directory.join("history.json");
    fs::write(
        &topology_path,
        serde_json::to_vec_pretty(&topology).expect("encode topology"),
    )
    .expect("write topology");
    let (url, server) = fixture_server("process-secret", true, 2);

    let output = Command::new(env!(
        "CARGO_BIN_EXE_smart-home-import-home-assistant-history"
    ))
    .arg(&topology_path)
    .arg(url)
    .arg("2026-01-01T00:00:00Z")
    .arg("2026-01-02T00:00:00Z")
    .arg(&output_path)
    .arg("--batch-size")
    .arg("1")
    .arg("--timeout-ms")
    .arg("2000")
    .arg("--collected-at-ms")
    .arg("1767312000000")
    .env("HOME_ASSISTANT_TOKEN", "process-secret")
    .output()
    .expect("run history CLI");
    server.join().expect("fixture server");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("5 D23 events"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("process-secret"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("process-secret"));
    let artifact: HomeAssistantHistoryMigrationArtifact =
        serde_json::from_slice(&fs::read(&output_path).expect("read artifact"))
            .expect("decode artifact");
    assert_eq!(artifact.history_plan.summary.planned_events, 5);
    assert_eq!(
        artifact
            .runtime_snapshot
            .as_ref()
            .expect("runtime snapshot")
            .registry_events
            .len(),
        5
    );

    fs::remove_file(topology_path).expect("remove topology");
    fs::remove_file(output_path).expect("remove artifact");
    fs::remove_dir(directory).expect("remove test directory");
}

fn config(websocket_url: String, batch_size: usize) -> HistoryCollectorConfig {
    HistoryCollectorConfig {
        websocket_url,
        access_token: "history-secret".to_string(),
        source_instance_id: "home-1".to_string(),
        start_time: "2026-01-01T00:00:00Z".to_string(),
        end_time: "2026-01-02T00:00:00Z".to_string(),
        collected_at_ms: 1_767_312_000_000,
        batch_size,
        io_timeout: Duration::from_secs(2),
    }
}

fn fixture_server(
    expected_token: &'static str,
    auth_ok: bool,
    expected_requests: usize,
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
        if !auth_ok {
            send(&mut socket, json!({"type": "auth_invalid"}));
            return;
        }
        send(&mut socket, json!({"type": "auth_ok"}));

        for expected_id in 1..=expected_requests {
            let request = read(&mut socket);
            assert_eq!(request["id"], expected_id as u64);
            assert_eq!(request["type"], "history/history_during_period");
            assert_eq!(request["include_start_time_state"], true);
            assert_eq!(request["significant_changes_only"], false);
            assert_eq!(request["minimal_response"], false);
            assert_eq!(request["no_attributes"], false);
            let ids = request["entity_ids"]
                .as_array()
                .expect("entity id array")
                .iter()
                .map(|value| value.as_str().expect("entity id"))
                .collect::<Vec<_>>();
            let mut result = serde_json::Map::new();
            for entity_id in ids {
                result.insert(entity_id.to_string(), history_for(entity_id));
            }
            send(
                &mut socket,
                json!({
                    "id": expected_id,
                    "type": "result",
                    "success": true,
                    "result": result
                }),
            );
        }
    });
    (format!("ws://{address}/api/websocket"), handle)
}

fn history_for(entity_id: &str) -> JsonValue {
    match entity_id {
        "light.kitchen" => json!([
            {
                "entity_id": entity_id,
                "state": "on",
                "attributes": {"brightness": 128},
                "last_changed": "2026-01-01T01:00:00Z",
                "last_updated": "2026-01-01T01:00:00Z"
            },
            {
                "entity_id": entity_id,
                "state": "off",
                "attributes": {"brightness": 0},
                "last_changed": "2026-01-01T02:00:00Z",
                "last_updated": "2026-01-01T02:00:00Z"
            }
        ]),
        "sensor.temperature" => json!([{
            "entity_id": entity_id,
            "state": "21.5",
            "attributes": {"device_class": "temperature", "unit_of_measurement": "C"},
            "last_changed": "2026-01-01T01:30:00Z",
            "last_updated": "2026-01-01T01:30:00Z"
        }]),
        other => panic!("unexpected fixture entity {other}"),
    }
}

fn send<S: std::io::Read + std::io::Write>(socket: &mut WebSocket<S>, value: JsonValue) {
    socket
        .send(Message::Text(value.to_string().into()))
        .expect("send fixture response");
}

fn read<S: std::io::Read + std::io::Write>(socket: &mut WebSocket<S>) -> JsonValue {
    let message = socket.read().expect("read fixture request");
    serde_json::from_str(message.to_text().expect("fixture text")).expect("fixture JSON")
}

fn fixture_topology() -> HomeAssistantExport {
    HomeAssistantExport {
        schema_version: EXPORT_SCHEMA_VERSION,
        source_instance_id: "home-1".to_string(),
        exported_at_ms: 1_767_312_000_000,
        areas: vec![HomeAssistantArea {
            area_id: "kitchen".to_string(),
            name: "Kitchen".to_string(),
            aliases: vec![],
        }],
        devices: vec![HomeAssistantDevice {
            device_id: "device-1".to_string(),
            area_id: Some("kitchen".to_string()),
            name: Some("Kitchen devices".to_string()),
            name_by_user: None,
            manufacturer: Some("Example".to_string()),
            model: Some("Fixture".to_string()),
            serial_number: None,
            sw_version: None,
            identifiers: vec![("demo".to_string(), "device-1".to_string())],
        }],
        entities: vec![
            HomeAssistantEntity {
                entity_id: "light.kitchen".to_string(),
                device_id: Some("device-1".to_string()),
                area_id: None,
                platform: "demo".to_string(),
                unique_id: "light-1".to_string(),
                name: Some("Kitchen light".to_string()),
                original_name: None,
                disabled_by: None,
            },
            HomeAssistantEntity {
                entity_id: "sensor.temperature".to_string(),
                device_id: Some("device-1".to_string()),
                area_id: None,
                platform: "demo".to_string(),
                unique_id: "temperature-1".to_string(),
                name: Some("Kitchen temperature".to_string()),
                original_name: None,
                disabled_by: None,
            },
        ],
        states: vec![
            HomeAssistantState {
                entity_id: "light.kitchen".to_string(),
                state: "off".to_string(),
                attributes: BTreeMap::new(),
            },
            HomeAssistantState {
                entity_id: "sensor.temperature".to_string(),
                state: "25.0".to_string(),
                attributes: BTreeMap::from([
                    ("device_class".to_string(), json!("temperature")),
                    ("unit_of_measurement".to_string(), json!("C")),
                ]),
            },
        ],
        scenes: vec![],
        automations: vec![],
    }
}
