use serde_json::{json, Value as JsonValue};
use smart_home_home_assistant_definitions::{
    collect_definitions, DefinitionCollectorConfig, EnrichedHomeAssistantExport,
};
use smart_home_home_assistant_migration::{
    plan_export, HomeAssistantArea, HomeAssistantDevice, HomeAssistantEntity, HomeAssistantExport,
    HomeAssistantState, EXPORT_SCHEMA_VERSION,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use tungstenite::{accept, Message};

#[test]
fn collects_deterministic_definitions_and_feeds_existing_migration() {
    let topology = topology_fixture();
    let first = collect_fixture(&topology, supported_automation());
    let second = collect_fixture(&topology, supported_automation());

    assert_eq!(first, second);
    assert_eq!(first.definition_collection.summary.collected_scenes, 1);
    assert_eq!(first.definition_collection.summary.collected_automations, 1);
    assert!(first.definition_collection.diagnostics.is_empty());
    assert_eq!(first.export.scenes[0].states[0].entity_id, "light.desk");
    assert_eq!(
        first.export.automations[0].automation_id,
        "automation-config-1"
    );

    let bytes = serde_json::to_vec(&first).expect("encode enriched export");
    let migration_input: HomeAssistantExport =
        serde_json::from_slice(&bytes).expect("decode as original export contract");
    let plan = plan_export(&migration_input).expect("plan enriched export");
    assert!(!plan.is_blocked(), "diagnostics: {:?}", plan.diagnostics);
    assert_eq!(plan.summary.scenes, 1);
    assert_eq!(plan.summary.automations, 1);
    assert_eq!(plan.summary.automation_actions, 1);
}

#[test]
fn unsupported_automation_is_skipped_with_durable_diagnostic() {
    let mut topology = topology_fixture();
    topology
        .entities
        .retain(|entity| !entity.entity_id.starts_with("scene."));
    topology
        .states
        .retain(|state| !state.entity_id.starts_with("scene."));
    let (ws_url, ws_server) = websocket_server(json!({
        "id": "automation-config-1",
        "alias": "Unsafe wait",
        "triggers": {"trigger": "state", "entity_id": "binary_sensor.motion", "to": "on"},
        "actions": {"delay": 30}
    }));
    let config = config(ws_url, "http://127.0.0.1:9".to_string());

    let result = collect_definitions(&topology, &config).expect("collect with diagnostic");
    ws_server.join().expect("join WebSocket fixture");

    assert!(result.export.automations.is_empty());
    assert_eq!(result.definition_collection.summary.skipped_automations, 1);
    assert_eq!(result.definition_collection.diagnostics.len(), 1);
    assert_eq!(
        result.definition_collection.diagnostics[0].code,
        "unsupported_automation_definition"
    );
}

#[test]
fn rejected_authentication_does_not_echo_token() {
    let mut topology = topology_fixture();
    topology
        .entities
        .retain(|entity| !entity.entity_id.starts_with("scene."));
    topology
        .states
        .retain(|state| !state.entity_id.starts_with("scene."));
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind auth fixture");
    let address = listener.local_addr().expect("auth fixture address");
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept auth fixture");
        let mut socket = accept(stream).expect("upgrade auth fixture");
        send_json(&mut socket, json!({"type": "auth_required"}));
        let auth = read_json(&mut socket);
        assert_eq!(auth["access_token"], "fixture-secret-token");
        send_json(
            &mut socket,
            json!({
                "type": "auth_invalid",
                "message": "fixture-secret-token is invalid"
            }),
        );
    });
    let config = config(
        format!("ws://{address}/api/websocket"),
        "http://127.0.0.1:9".to_string(),
    );

    let error = collect_definitions(&topology, &config).expect_err("reject auth");
    server.join().expect("join auth fixture");

    assert!(!error.to_string().contains("fixture-secret-token"));
    assert!(error.to_string().contains("rejected the access token"));
}

#[test]
fn cli_collects_live_definitions_without_leaking_token() {
    let topology = topology_fixture();
    let (ws_url, ws_server) = websocket_server(supported_automation());
    let (rest_url, rest_server) = scene_server();
    let directory = std::env::temp_dir().join(format!(
        "smart-home-ha-definitions-cli-{}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("create temp directory");
    let topology_path = directory.join("topology.json");
    let output_path = directory.join("enriched.json");
    fs::write(
        &topology_path,
        serde_json::to_vec_pretty(&topology).expect("encode topology"),
    )
    .expect("write topology");

    let output = Command::new(env!(
        "CARGO_BIN_EXE_smart-home-collect-home-assistant-definitions"
    ))
    .arg(&topology_path)
    .arg(ws_url)
    .arg(rest_url)
    .arg(&output_path)
    .arg("--collected-at-ms")
    .arg("1722000001000")
    .arg("--timeout-ms")
    .arg("2000")
    .env("HOME_ASSISTANT_TOKEN", "fixture-secret-token")
    .output()
    .expect("run definition collector CLI");
    ws_server.join().expect("join WebSocket fixture");
    rest_server.join().expect("join REST fixture");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1/1 scenes"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("fixture-secret-token"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("fixture-secret-token"));
    let enriched: EnrichedHomeAssistantExport =
        serde_json::from_slice(&fs::read(&output_path).expect("read enriched output"))
            .expect("decode enriched output");
    assert_eq!(
        enriched.definition_collection.collected_at_ms,
        1_722_000_001_000
    );
    assert_eq!(enriched.export.scenes.len(), 1);
    assert_eq!(enriched.export.automations.len(), 1);

    fs::remove_file(topology_path).expect("remove topology");
    fs::remove_file(output_path).expect("remove output");
    fs::remove_dir(directory).expect("remove directory");
}

fn collect_fixture(
    topology: &HomeAssistantExport,
    automation: JsonValue,
) -> EnrichedHomeAssistantExport {
    let (ws_url, ws_server) = websocket_server(automation);
    let (rest_url, rest_server) = scene_server();
    let result =
        collect_definitions(topology, &config(ws_url, rest_url)).expect("collect definitions");
    ws_server.join().expect("join WebSocket fixture");
    rest_server.join().expect("join REST fixture");
    result
}

fn websocket_server(config: JsonValue) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind WebSocket fixture");
    let address = listener.local_addr().expect("WebSocket fixture address");
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept WebSocket fixture");
        let mut socket = accept(stream).expect("upgrade WebSocket fixture");
        send_json(&mut socket, json!({"type": "auth_required"}));
        let auth = read_json(&mut socket);
        assert_eq!(auth["type"], "auth");
        assert_eq!(auth["access_token"], "fixture-secret-token");
        send_json(&mut socket, json!({"type": "auth_ok"}));
        let request = read_json(&mut socket);
        assert_eq!(request["type"], "automation/config");
        assert_eq!(request["entity_id"], "automation.night_motion");
        send_json(
            &mut socket,
            json!({
                "id": request["id"],
                "type": "result",
                "success": true,
                "result": {"config": config}
            }),
        );
    });
    (format!("ws://{address}/api/websocket"), handle)
}

fn scene_server() -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind REST fixture");
    let address = listener.local_addr().expect("REST fixture address");
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept REST fixture");
        let request = read_http_head(&mut stream);
        let request = String::from_utf8(request).expect("HTTP request text");
        assert!(request.starts_with("GET /api/config/scene/config/night-config HTTP/1.1\r\n"));
        assert!(request.contains("Authorization: Bearer fixture-secret-token\r\n"));
        let body = json!({
            "id": "night-config",
            "name": "Night",
            "entities": {
                "light.desk": {"state": "off", "transition": 2}
            }
        })
        .to_string();
        let split = body.len() / 2;
        let response = format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{:x}\r\n{}\r\n{:x}\r\n{}\r\n0\r\n\r\n",
            split,
            &body[..split],
            body.len() - split,
            &body[split..]
        );
        stream
            .write_all(response.as_bytes())
            .expect("write REST response");
    });
    (format!("http://{address}"), handle)
}

fn read_http_head(stream: &mut TcpStream) -> Vec<u8> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("set REST timeout");
    let mut request = Vec::new();
    let mut byte = [0_u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        let count = stream.read(&mut byte).expect("read REST request");
        if count == 0 {
            break;
        }
        request.push(byte[0]);
    }
    request
}

fn send_json<S: Read + Write>(socket: &mut tungstenite::WebSocket<S>, value: JsonValue) {
    socket
        .send(Message::Text(value.to_string().into()))
        .expect("send fixture JSON");
}

fn read_json<S: Read + Write>(socket: &mut tungstenite::WebSocket<S>) -> JsonValue {
    let message = socket.read().expect("read fixture JSON");
    serde_json::from_str(message.to_text().expect("fixture text")).expect("decode fixture JSON")
}

fn supported_automation() -> JsonValue {
    json!({
        "id": "automation-config-1",
        "alias": "Night motion",
        "triggers": {
            "trigger": "state",
            "entity_id": "binary_sensor.motion",
            "to": "on"
        },
        "conditions": {
            "condition": "state",
            "entity_id": "light.desk",
            "state": "off"
        },
        "actions": {
            "action": "scene.turn_on",
            "target": {"entity_id": "scene.night"}
        }
    })
}

fn config(websocket_url: String, rest_base_url: String) -> DefinitionCollectorConfig {
    DefinitionCollectorConfig {
        websocket_url,
        rest_base_url,
        access_token: "fixture-secret-token".to_string(),
        source_instance_id: "fixture-home".to_string(),
        collected_at_ms: 1_722_000_001_000,
        io_timeout: Duration::from_secs(2),
        max_response_bytes: 64 * 1024,
    }
}

fn topology_fixture() -> HomeAssistantExport {
    HomeAssistantExport {
        schema_version: EXPORT_SCHEMA_VERSION,
        source_instance_id: "fixture-home".to_string(),
        exported_at_ms: 1_722_000_000_000,
        areas: vec![HomeAssistantArea {
            area_id: "office".to_string(),
            name: "Office".to_string(),
            aliases: vec![],
        }],
        devices: vec![HomeAssistantDevice {
            device_id: "device-1".to_string(),
            area_id: Some("office".to_string()),
            name: Some("Desk".to_string()),
            name_by_user: None,
            manufacturer: Some("Fixture".to_string()),
            model: Some("Lamp".to_string()),
            serial_number: None,
            sw_version: None,
            identifiers: vec![],
        }],
        entities: vec![
            entity("light.desk", "demo", "light-1", Some("device-1")),
            entity("binary_sensor.motion", "demo", "motion-1", Some("device-1")),
            entity("scene.night", "homeassistant", "night-config", None),
            entity(
                "automation.night_motion",
                "automation",
                "automation-config-1",
                None,
            ),
        ],
        states: vec![
            state("light.desk", "off", "Desk lamp"),
            state("binary_sensor.motion", "off", "Motion"),
            state("scene.night", "unknown", "Night"),
            state("automation.night_motion", "on", "Night motion"),
        ],
        scenes: vec![],
        automations: vec![],
    }
}

fn entity(
    entity_id: &str,
    platform: &str,
    unique_id: &str,
    device_id: Option<&str>,
) -> HomeAssistantEntity {
    HomeAssistantEntity {
        entity_id: entity_id.to_string(),
        device_id: device_id.map(str::to_string),
        area_id: if entity_id == "scene.night" {
            Some("office".to_string())
        } else {
            None
        },
        platform: platform.to_string(),
        unique_id: unique_id.to_string(),
        name: None,
        original_name: None,
        disabled_by: None,
    }
}

fn state(entity_id: &str, value: &str, name: &str) -> HomeAssistantState {
    HomeAssistantState {
        entity_id: entity_id.to_string(),
        state: value.to_string(),
        attributes: BTreeMap::from([("friendly_name".to_string(), json!(name))]),
    }
}
