use serde_json::{json, Value as JsonValue};
use smart_home_home_assistant_migration::{plan_export, HomeAssistantExport};
use std::fs;
use std::net::TcpListener;
use std::process::Command;
use std::thread;
use tungstenite::{accept, Message};

#[test]
fn cli_collects_live_export_and_writes_artifact() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
    let address = listener.local_addr().expect("fixture address");
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept fixture");
        let mut socket = accept(stream).expect("upgrade fixture");
        send(&mut socket, json!({"type": "auth_required"}));
        let auth = read(&mut socket);
        assert_eq!(auth["access_token"], "process-secret");
        send(&mut socket, json!({"type": "auth_ok"}));

        let results = [
            json!([{"area_id": "office", "name": "Office"}]),
            json!([{"id": "device-1", "name": "Desk Lamp"}]),
            json!([{
                "entity_id": "light.desk",
                "device_id": "device-1",
                "platform": "demo",
                "unique_id": "desk-1"
            }]),
            json!([{
                "entity_id": "light.desk",
                "state": "off",
                "attributes": {"friendly_name": "Desk Lamp"}
            }]),
        ];
        for (index, result) in results.into_iter().enumerate() {
            let request = read(&mut socket);
            assert_eq!(request["id"], (index + 1) as u64);
            send(
                &mut socket,
                json!({
                    "id": index + 1,
                    "type": "result",
                    "success": true,
                    "result": result
                }),
            );
        }
    });

    let directory =
        std::env::temp_dir().join(format!("smart-home-ha-export-cli-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("create output directory");
    let output = directory.join("export.json");
    let command = Command::new(env!("CARGO_BIN_EXE_smart-home-export-home-assistant"))
        .arg(format!("ws://{address}/api/websocket"))
        .arg("process-home")
        .arg(&output)
        .arg("--exported-at-ms")
        .arg("1722000000000")
        .arg("--timeout-ms")
        .arg("2000")
        .env("HOME_ASSISTANT_TOKEN", "process-secret")
        .output()
        .expect("run collector CLI");
    server.join().expect("fixture server");

    assert!(
        command.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&command.stderr)
    );
    assert!(String::from_utf8_lossy(&command.stdout).contains("1 areas"));
    let export: HomeAssistantExport =
        serde_json::from_slice(&fs::read(&output).expect("read export")).expect("decode export");
    assert_eq!(export.source_instance_id, "process-home");
    assert_eq!(export.exported_at_ms, 1_722_000_000_000);
    assert_eq!(export.devices[0].device_id, "device-1");
    assert_eq!(export.entities[0].entity_id, "light.desk");
    assert_eq!(export.states[0].state, "off");
    let plan = plan_export(&export).expect("plan collected export");
    assert!(!plan.is_blocked());
    assert_eq!(plan.summary.devices, 1);
    assert_eq!(plan.summary.entities, 1);
    assert_eq!(plan.summary.states, 1);
    assert!(!String::from_utf8_lossy(&command.stdout).contains("process-secret"));
    assert!(!String::from_utf8_lossy(&command.stderr).contains("process-secret"));

    fs::remove_file(output).expect("remove export");
    fs::remove_dir(directory).expect("remove output directory");
}

fn send<S: std::io::Read + std::io::Write>(
    socket: &mut tungstenite::WebSocket<S>,
    value: JsonValue,
) {
    socket
        .send(Message::Text(value.to_string().into()))
        .expect("send fixture response");
}

fn read<S: std::io::Read + std::io::Write>(socket: &mut tungstenite::WebSocket<S>) -> JsonValue {
    let message = socket.read().expect("read fixture request");
    serde_json::from_str(message.to_text().expect("fixture text")).expect("fixture JSON")
}
