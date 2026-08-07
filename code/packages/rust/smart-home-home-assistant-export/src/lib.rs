//! Live Home Assistant export collection for the D23 migration boundary.

#![forbid(unsafe_code)]

use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use smart_home_home_assistant_migration::{
    HomeAssistantArea, HomeAssistantAutomation, HomeAssistantDevice, HomeAssistantEntity,
    HomeAssistantExport, HomeAssistantScene, HomeAssistantState, EXPORT_SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

const AREA_REGISTRY_COMMAND: &str = "config/area_registry/list";
const DEVICE_REGISTRY_COMMAND: &str = "config/device_registry/list";
const ENTITY_REGISTRY_COMMAND: &str = "config/entity_registry/list";
const STATES_COMMAND: &str = "get_states";
const MAX_UNMATCHED_MESSAGES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeAssistantCollectorConfig {
    pub websocket_url: String,
    pub access_token: String,
    pub source_instance_id: String,
    pub exported_at_ms: u64,
    pub io_timeout: Duration,
}

impl HomeAssistantCollectorConfig {
    pub fn validate(&self) -> Result<(), CollectorError> {
        if !self.websocket_url.starts_with("ws://") && !self.websocket_url.starts_with("wss://") {
            return Err(CollectorError::Config(
                "Home Assistant URL must use ws:// or wss://".to_string(),
            ));
        }
        if self.access_token.trim().is_empty() {
            return Err(CollectorError::Config(
                "Home Assistant access token is empty".to_string(),
            ));
        }
        if self.source_instance_id.trim().is_empty() {
            return Err(CollectorError::Config(
                "source instance id is empty".to_string(),
            ));
        }
        if self.io_timeout.is_zero() {
            return Err(CollectorError::Config(
                "I/O timeout must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum CollectorError {
    Config(String),
    Transport(String),
    Protocol(String),
    Decode {
        resource: &'static str,
        message: String,
    },
    Encode(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    Usage(String),
}

impl CollectorError {
    fn decode(resource: &'static str, error: serde_json::Error) -> Self {
        Self::Decode {
            resource,
            message: error.to_string(),
        }
    }
}

impl fmt::Display for CollectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => write!(f, "invalid collector configuration: {message}"),
            Self::Transport(message) => write!(f, "Home Assistant transport failed: {message}"),
            Self::Protocol(message) => write!(f, "Home Assistant protocol failed: {message}"),
            Self::Decode { resource, message } => {
                write!(f, "invalid Home Assistant {resource} response: {message}")
            }
            Self::Encode(message) => write!(f, "could not encode Home Assistant export: {message}"),
            Self::Io {
                operation,
                path,
                message,
            } => write!(f, "could not {operation} {}: {message}", path.display()),
            Self::Usage(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for CollectorError {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CollectionSummary {
    pub areas: usize,
    pub devices: usize,
    pub entities: usize,
    pub synthetic_entities: usize,
    pub states: usize,
}

pub fn collect_export(
    config: &HomeAssistantCollectorConfig,
) -> Result<(HomeAssistantExport, CollectionSummary), CollectorError> {
    config.validate()?;
    let (mut socket, _) = connect(config.websocket_url.as_str()).map_err(|error| {
        CollectorError::Transport(redact_transport_error(error.to_string(), config))
    })?;
    configure_socket_timeout(&mut socket, config.io_timeout)?;

    authenticate(&mut socket, &config.access_token)?;

    let raw_areas = request(&mut socket, 1, AREA_REGISTRY_COMMAND)?;
    let raw_devices = request(&mut socket, 2, DEVICE_REGISTRY_COMMAND)?;
    let raw_entities = request(&mut socket, 3, ENTITY_REGISTRY_COMMAND)?;
    let raw_states = request(&mut socket, 4, STATES_COMMAND)?;

    let mut areas = decode_areas(raw_areas)?;
    let mut devices = decode_devices(raw_devices)?;
    let mut entities = decode_entities(raw_entities)?;
    let mut states = decode_states(raw_states)?;
    let synthetic_entities = add_synthetic_entities(&mut entities, &states);

    areas.sort_by(|left, right| left.area_id.cmp(&right.area_id));
    devices.sort_by(|left, right| left.device_id.cmp(&right.device_id));
    entities.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));
    states.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));

    ensure_unique(areas.iter().map(|item| item.area_id.as_str()), "area")?;
    ensure_unique(devices.iter().map(|item| item.device_id.as_str()), "device")?;
    ensure_unique(
        entities.iter().map(|item| item.entity_id.as_str()),
        "entity",
    )?;
    ensure_unique(states.iter().map(|item| item.entity_id.as_str()), "state")?;

    let summary = CollectionSummary {
        areas: areas.len(),
        devices: devices.len(),
        entities: entities.len(),
        synthetic_entities,
        states: states.len(),
    };
    let _ = socket.close(None);

    Ok((
        HomeAssistantExport {
            schema_version: EXPORT_SCHEMA_VERSION,
            source_instance_id: config.source_instance_id.clone(),
            exported_at_ms: config.exported_at_ms,
            areas,
            devices,
            entities,
            states,
            scenes: Vec::<HomeAssistantScene>::new(),
            automations: Vec::<HomeAssistantAutomation>::new(),
        },
        summary,
    ))
}

pub fn write_export_atomically(
    path: impl AsRef<Path>,
    export: &HomeAssistantExport,
) -> Result<(), CollectorError> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| CollectorError::Config("output path has no file name".to_string()))?;
    let temporary = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    let mut encoded = serde_json::to_vec_pretty(export)
        .map_err(|error| CollectorError::Encode(error.to_string()))?;
    encoded.push(b'\n');

    let result = (|| {
        let mut file = File::create(&temporary).map_err(|error| CollectorError::Io {
            operation: "create temporary export",
            path: temporary.clone(),
            message: error.to_string(),
        })?;
        file.write_all(&encoded)
            .and_then(|()| file.sync_all())
            .map_err(|error| CollectorError::Io {
                operation: "write temporary export",
                path: temporary.clone(),
                message: error.to_string(),
            })?;
        fs::rename(&temporary, path).map_err(|error| CollectorError::Io {
            operation: "replace export",
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

type HomeAssistantSocket = WebSocket<MaybeTlsStream<TcpStream>>;

fn configure_socket_timeout(
    socket: &mut HomeAssistantSocket,
    timeout: Duration,
) -> Result<(), CollectorError> {
    let stream = match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => stream,
        MaybeTlsStream::Rustls(stream) => &mut stream.sock,
        _ => {
            return Err(CollectorError::Transport(
                "unsupported WebSocket stream backend".to_string(),
            ));
        }
    };
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|error| CollectorError::Transport(error.to_string()))
}

fn authenticate(
    socket: &mut HomeAssistantSocket,
    access_token: &str,
) -> Result<(), CollectorError> {
    let required = read_json(socket)?;
    if required.get("type").and_then(JsonValue::as_str) != Some("auth_required") {
        return Err(CollectorError::Protocol(
            "server did not begin with auth_required".to_string(),
        ));
    }

    send_json(
        socket,
        &json!({"type": "auth", "access_token": access_token}),
    )?;
    let response = read_json(socket)?;
    match response.get("type").and_then(JsonValue::as_str) {
        Some("auth_ok") => Ok(()),
        Some("auth_invalid") => Err(CollectorError::Protocol(
            "Home Assistant rejected the access token".to_string(),
        )),
        _ => Err(CollectorError::Protocol(
            "server returned an unexpected authentication response".to_string(),
        )),
    }
}

fn request(
    socket: &mut HomeAssistantSocket,
    id: u64,
    command: &'static str,
) -> Result<JsonValue, CollectorError> {
    send_json(socket, &json!({"id": id, "type": command}))?;
    for _ in 0..MAX_UNMATCHED_MESSAGES {
        let response = read_json(socket)?;
        if response.get("id").and_then(JsonValue::as_u64) != Some(id) {
            continue;
        }
        if response.get("type").and_then(JsonValue::as_str) != Some("result") {
            return Err(CollectorError::Protocol(format!(
                "{command} returned a non-result response"
            )));
        }
        if response.get("success").and_then(JsonValue::as_bool) != Some(true) {
            let code = response
                .pointer("/error/code")
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown_error");
            return Err(CollectorError::Protocol(format!(
                "{command} failed with {code}"
            )));
        }
        return response.get("result").cloned().ok_or_else(|| {
            CollectorError::Protocol(format!("{command} returned no result payload"))
        });
    }
    Err(CollectorError::Protocol(format!(
        "{command} did not return a matching response"
    )))
}

fn send_json(socket: &mut HomeAssistantSocket, value: &JsonValue) -> Result<(), CollectorError> {
    socket
        .send(Message::Text(value.to_string().into()))
        .map_err(|error| CollectorError::Transport(error.to_string()))
}

fn read_json(socket: &mut HomeAssistantSocket) -> Result<JsonValue, CollectorError> {
    loop {
        let message = socket
            .read()
            .map_err(|error| CollectorError::Transport(error.to_string()))?;
        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_ref())
                    .map_err(|error| CollectorError::decode("protocol", error));
            }
            Message::Close(_) => {
                return Err(CollectorError::Protocol(
                    "connection closed before collection completed".to_string(),
                ));
            }
            Message::Binary(_) => {
                return Err(CollectorError::Protocol(
                    "server returned an unexpected binary message".to_string(),
                ));
            }
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}

fn redact_transport_error(mut message: String, config: &HomeAssistantCollectorConfig) -> String {
    if !config.access_token.is_empty() {
        message = message.replace(&config.access_token, "[redacted]");
    }
    message
}

#[derive(Debug, Deserialize)]
struct RawArea {
    area_id: String,
    name: String,
    #[serde(default)]
    aliases: Vec<String>,
}

fn decode_areas(value: JsonValue) -> Result<Vec<HomeAssistantArea>, CollectorError> {
    let raw: Vec<RawArea> = serde_json::from_value(value)
        .map_err(|error| CollectorError::decode("area registry", error))?;
    Ok(raw
        .into_iter()
        .map(|area| HomeAssistantArea {
            area_id: area.area_id,
            name: area.name,
            aliases: area.aliases,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct RawDevice {
    #[serde(rename = "id")]
    device_id: String,
    #[serde(default)]
    area_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    name_by_user: Option<String>,
    #[serde(default)]
    manufacturer: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    serial_number: Option<String>,
    #[serde(default)]
    sw_version: Option<String>,
    #[serde(default)]
    identifiers: Vec<(String, String)>,
}

fn decode_devices(value: JsonValue) -> Result<Vec<HomeAssistantDevice>, CollectorError> {
    let raw: Vec<RawDevice> = serde_json::from_value(value)
        .map_err(|error| CollectorError::decode("device registry", error))?;
    Ok(raw
        .into_iter()
        .map(|device| HomeAssistantDevice {
            device_id: device.device_id,
            area_id: device.area_id,
            name: device.name,
            name_by_user: device.name_by_user,
            manufacturer: device.manufacturer,
            model: device.model,
            serial_number: device.serial_number,
            sw_version: device.sw_version,
            identifiers: device.identifiers,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct RawEntity {
    entity_id: String,
    #[serde(default)]
    device_id: Option<String>,
    #[serde(default)]
    area_id: Option<String>,
    platform: String,
    #[serde(default)]
    unique_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    original_name: Option<String>,
    #[serde(default)]
    disabled_by: Option<String>,
}

fn decode_entities(value: JsonValue) -> Result<Vec<HomeAssistantEntity>, CollectorError> {
    let raw: Vec<RawEntity> = serde_json::from_value(value)
        .map_err(|error| CollectorError::decode("entity registry", error))?;
    Ok(raw
        .into_iter()
        .map(|entity| HomeAssistantEntity {
            unique_id: entity.unique_id.unwrap_or_else(|| entity.entity_id.clone()),
            entity_id: entity.entity_id,
            device_id: entity.device_id,
            area_id: entity.area_id,
            platform: entity.platform,
            name: entity.name,
            original_name: entity.original_name,
            disabled_by: entity.disabled_by,
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct RawState {
    entity_id: String,
    state: String,
    #[serde(default)]
    attributes: BTreeMap<String, JsonValue>,
}

fn decode_states(value: JsonValue) -> Result<Vec<HomeAssistantState>, CollectorError> {
    let raw: Vec<RawState> =
        serde_json::from_value(value).map_err(|error| CollectorError::decode("states", error))?;
    Ok(raw
        .into_iter()
        .map(|state| HomeAssistantState {
            entity_id: state.entity_id,
            state: state.state,
            attributes: state.attributes,
        })
        .collect())
}

fn add_synthetic_entities(
    entities: &mut Vec<HomeAssistantEntity>,
    states: &[HomeAssistantState],
) -> usize {
    let mut known = entities
        .iter()
        .map(|entity| entity.entity_id.clone())
        .collect::<BTreeSet<_>>();
    let mut added = 0;
    for state in states {
        if known.insert(state.entity_id.clone()) {
            let platform = state
                .entity_id
                .split_once('.')
                .map_or("unknown", |(domain, _)| domain);
            let name = state
                .attributes
                .get("friendly_name")
                .and_then(JsonValue::as_str)
                .map(str::to_string);
            entities.push(HomeAssistantEntity {
                entity_id: state.entity_id.clone(),
                device_id: None,
                area_id: None,
                platform: platform.to_string(),
                unique_id: state.entity_id.clone(),
                name,
                original_name: None,
                disabled_by: None,
            });
            added += 1;
        }
    }
    added
}

fn ensure_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    resource: &str,
) -> Result<(), CollectorError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(CollectorError::Protocol(format!(
                "Home Assistant returned duplicate {resource} id {value}"
            )));
        }
    }
    Ok(())
}

pub fn remove_file_if_exists(path: impl AsRef<Path>) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;
    use tungstenite::accept;

    fn fixture_server(
        auth_ok: bool,
        command_failure: Option<&'static str>,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept fixture");
            let mut socket = accept(stream).expect("upgrade fixture");
            socket
                .send(Message::Text(
                    json!({"type": "auth_required", "ha_version": "2026.7"})
                        .to_string()
                        .into(),
                ))
                .expect("send auth required");
            let auth = read_fixture_json(&mut socket);
            assert_eq!(auth["type"], "auth");
            assert_eq!(auth["access_token"], "secret-token");
            if !auth_ok {
                socket
                    .send(Message::Text(
                        json!({"type": "auth_invalid", "message": "invalid"})
                            .to_string()
                            .into(),
                    ))
                    .expect("send auth invalid");
                return;
            }
            socket
                .send(Message::Text(
                    json!({"type": "auth_ok", "ha_version": "2026.7"})
                        .to_string()
                        .into(),
                ))
                .expect("send auth ok");

            let fixtures = [
                (
                    AREA_REGISTRY_COMMAND,
                    json!([{"area_id": "kitchen", "name": "Kitchen", "aliases": ["Galley"]}]),
                ),
                (
                    DEVICE_REGISTRY_COMMAND,
                    json!([{
                        "id": "device-1",
                        "area_id": "kitchen",
                        "name": "Kitchen Lamp",
                        "manufacturer": "Example",
                        "model": "L1",
                        "identifiers": [["demo", "lamp-1"]]
                    }]),
                ),
                (
                    ENTITY_REGISTRY_COMMAND,
                    json!([{
                        "entity_id": "light.kitchen",
                        "device_id": "device-1",
                        "area_id": null,
                        "platform": "demo",
                        "unique_id": "lamp-1",
                        "original_name": "Lamp"
                    }]),
                ),
                (
                    STATES_COMMAND,
                    json!([
                        {
                            "entity_id": "light.kitchen",
                            "state": "on",
                            "attributes": {"brightness": 127}
                        },
                        {
                            "entity_id": "sensor.unregistered",
                            "state": "21.5",
                            "attributes": {"friendly_name": "Room temperature"}
                        }
                    ]),
                ),
            ];

            for (index, (command, result)) in fixtures.into_iter().enumerate() {
                let request = read_fixture_json(&mut socket);
                assert_eq!(request["id"], (index + 1) as u64);
                assert_eq!(request["type"], command);
                let response = if command_failure == Some(command) {
                    json!({
                        "id": index + 1,
                        "type": "result",
                        "success": false,
                        "error": {"code": "unauthorized", "message": "denied"}
                    })
                } else {
                    json!({
                        "id": index + 1,
                        "type": "result",
                        "success": true,
                        "result": result
                    })
                };
                socket
                    .send(Message::Text(response.to_string().into()))
                    .expect("send fixture result");
                if command_failure == Some(command) {
                    return;
                }
            }
        });
        (format!("ws://{address}/api/websocket"), handle)
    }

    fn read_fixture_json<S: io::Read + io::Write>(socket: &mut WebSocket<S>) -> JsonValue {
        let message = socket.read().expect("read fixture request");
        serde_json::from_str(message.to_text().expect("text fixture request"))
            .expect("fixture request JSON")
    }

    fn config(url: String) -> HomeAssistantCollectorConfig {
        HomeAssistantCollectorConfig {
            websocket_url: url,
            access_token: "secret-token".to_string(),
            source_instance_id: "home-1".to_string(),
            exported_at_ms: 1_722_000_000_000,
            io_timeout: Duration::from_secs(2),
        }
    }

    #[test]
    fn collects_registry_and_state_export_over_websocket() {
        let (url, server) = fixture_server(true, None);
        let (export, summary) = collect_export(&config(url)).expect("collect export");
        server.join().expect("fixture server");

        assert_eq!(export.schema_version, EXPORT_SCHEMA_VERSION);
        assert_eq!(export.source_instance_id, "home-1");
        assert_eq!(export.areas[0].area_id, "kitchen");
        assert_eq!(export.devices[0].identifiers[0].1, "lamp-1");
        assert_eq!(export.entities.len(), 2);
        assert_eq!(export.entities[1].entity_id, "sensor.unregistered");
        assert_eq!(export.entities[1].name.as_deref(), Some("Room temperature"));
        assert_eq!(export.states.len(), 2);
        assert!(export.scenes.is_empty());
        assert!(export.automations.is_empty());
        assert_eq!(
            summary,
            CollectionSummary {
                areas: 1,
                devices: 1,
                entities: 2,
                synthetic_entities: 1,
                states: 2,
            }
        );
    }

    #[test]
    fn rejects_invalid_auth_without_echoing_token() {
        let (url, server) = fixture_server(false, None);
        let error = collect_export(&config(url)).expect_err("auth should fail");
        server.join().expect("fixture server");

        let message = error.to_string();
        assert!(message.contains("rejected"));
        assert!(!message.contains("secret-token"));
    }

    #[test]
    fn reports_failed_registry_command() {
        let (url, server) = fixture_server(true, Some(DEVICE_REGISTRY_COMMAND));
        let error = collect_export(&config(url)).expect_err("command should fail");
        server.join().expect("fixture server");

        assert!(error.to_string().contains(DEVICE_REGISTRY_COMMAND));
        assert!(error.to_string().contains("unauthorized"));
    }

    #[test]
    fn writes_export_atomically() {
        let directory =
            std::env::temp_dir().join(format!("smart-home-ha-export-test-{}", std::process::id()));
        fs::create_dir_all(&directory).expect("create test directory");
        let output = directory.join("export.json");
        let export = HomeAssistantExport {
            schema_version: EXPORT_SCHEMA_VERSION,
            source_instance_id: "home-1".to_string(),
            exported_at_ms: 10,
            areas: vec![],
            devices: vec![],
            entities: vec![],
            states: vec![],
            scenes: vec![],
            automations: vec![],
        };

        write_export_atomically(&output, &export).expect("write export");
        let decoded: HomeAssistantExport =
            serde_json::from_slice(&fs::read(&output).expect("read export"))
                .expect("decode export");
        assert_eq!(decoded, export);
        assert!(!directory
            .join(format!(".export.json.tmp-{}", std::process::id()))
            .exists());

        remove_file_if_exists(&output).expect("remove output");
        fs::remove_dir(directory).expect("remove directory");
    }
}
