//! Live Home Assistant scene and automation definition collection.

#![forbid(unsafe_code)]

use coding_adventures_sha256::sha256_hex;
use http1::parse_response_head;
use http_core::BodyKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use smart_home_home_assistant_migration::{
    HomeAssistantAutomation, HomeAssistantAutomationTrigger, HomeAssistantEntity,
    HomeAssistantExport, HomeAssistantScene, HomeAssistantServiceAction,
    HomeAssistantStateCondition, HomeAssistantTargetState, EXPORT_SCHEMA_VERSION,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};
use url_parser::{percent_encode, Url};

pub const DEFINITION_COLLECTION_SCHEMA_VERSION: u32 = 1;
const AUTOMATION_CONFIG_COMMAND: &str = "automation/config";
const MAX_UNMATCHED_MESSAGES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionCollectorConfig {
    pub websocket_url: String,
    pub rest_base_url: String,
    pub access_token: String,
    pub source_instance_id: String,
    pub collected_at_ms: u64,
    pub io_timeout: Duration,
    pub max_response_bytes: usize,
}

impl DefinitionCollectorConfig {
    pub fn validate(&self) -> Result<(), DefinitionError> {
        if !self.websocket_url.starts_with("ws://") && !self.websocket_url.starts_with("wss://") {
            return Err(DefinitionError::Config(
                "Home Assistant WebSocket URL must use ws:// or wss://".to_string(),
            ));
        }
        let rest = Url::parse(&self.rest_base_url)
            .map_err(|error| DefinitionError::Config(format!("invalid REST URL: {error}")))?;
        if rest.scheme != "http" && rest.scheme != "https" {
            return Err(DefinitionError::Config(
                "Home Assistant REST URL must use http:// or https://".to_string(),
            ));
        }
        if rest.userinfo.is_some() || rest.query.is_some() || rest.fragment.is_some() {
            return Err(DefinitionError::Config(
                "Home Assistant REST URL cannot contain credentials, a query, or a fragment"
                    .to_string(),
            ));
        }
        if self.access_token.trim().is_empty() {
            return Err(DefinitionError::Config(
                "Home Assistant access token is empty".to_string(),
            ));
        }
        if self.source_instance_id.trim().is_empty() {
            return Err(DefinitionError::Config(
                "source instance id is empty".to_string(),
            ));
        }
        if self.io_timeout.is_zero() {
            return Err(DefinitionError::Config(
                "I/O timeout must be greater than zero".to_string(),
            ));
        }
        if self.max_response_bytes == 0 {
            return Err(DefinitionError::Config(
                "maximum response bytes must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionDiagnostic {
    pub severity: DefinitionDiagnosticSeverity,
    pub source_kind: String,
    pub source_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionCollectionSummary {
    pub discovered_scenes: usize,
    pub collected_scenes: usize,
    pub skipped_scenes: usize,
    pub discovered_automations: usize,
    pub collected_automations: usize,
    pub skipped_automations: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DefinitionCollectionReport {
    pub schema_version: u32,
    pub source_instance_id: String,
    pub topology_fingerprint: String,
    pub collected_at_ms: u64,
    pub summary: DefinitionCollectionSummary,
    pub diagnostics: Vec<DefinitionDiagnostic>,
}

/// The report is an extra top-level field, so older migration readers can
/// deserialize this artifact directly as [`HomeAssistantExport`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnrichedHomeAssistantExport {
    #[serde(flatten)]
    pub export: HomeAssistantExport,
    pub definition_collection: DefinitionCollectionReport,
}

#[derive(Debug)]
pub enum DefinitionError {
    Config(String),
    Validation(String),
    Transport(String),
    Protocol(String),
    Decode(String),
    Encode(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    Usage(String),
}

impl fmt::Display for DefinitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => {
                write!(formatter, "invalid collector configuration: {message}")
            }
            Self::Validation(message) => write!(formatter, "invalid definition input: {message}"),
            Self::Transport(message) => {
                write!(formatter, "Home Assistant transport failed: {message}")
            }
            Self::Protocol(message) => {
                write!(formatter, "Home Assistant protocol failed: {message}")
            }
            Self::Decode(message) => {
                write!(formatter, "invalid Home Assistant definition: {message}")
            }
            Self::Encode(message) => {
                write!(formatter, "could not encode enriched export: {message}")
            }
            Self::Io {
                operation,
                path,
                message,
            } => {
                write!(
                    formatter,
                    "could not {operation} {}: {message}",
                    path.display()
                )
            }
            Self::Usage(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DefinitionError {}

pub fn collect_definitions(
    topology: &HomeAssistantExport,
    config: &DefinitionCollectorConfig,
) -> Result<EnrichedHomeAssistantExport, DefinitionError> {
    config.validate()?;
    validate_topology(topology, config)?;

    let mut base = topology.clone();
    base.scenes.clear();
    base.automations.clear();
    let topology_bytes =
        serde_json::to_vec(&base).map_err(|error| DefinitionError::Encode(error.to_string()))?;
    let topology_fingerprint = sha256_hex(&topology_bytes);
    let states = base
        .states
        .iter()
        .map(|state| (state.entity_id.as_str(), state))
        .collect::<BTreeMap<_, _>>();
    let mut diagnostics = Vec::new();

    let mut scene_entities = base
        .entities
        .iter()
        .filter(|entity| entity_domain(&entity.entity_id) == "scene")
        .collect::<Vec<_>>();
    scene_entities.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));
    let mut automation_entities = base
        .entities
        .iter()
        .filter(|entity| entity_domain(&entity.entity_id) == "automation")
        .collect::<Vec<_>>();
    automation_entities.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));

    let mut automations = Vec::new();
    if !automation_entities.is_empty() {
        let (mut socket, _) = connect(config.websocket_url.as_str()).map_err(|error| {
            DefinitionError::Transport(redact(error.to_string(), &config.access_token))
        })?;
        configure_socket_timeout(&mut socket, config.io_timeout)?;
        authenticate(&mut socket, &config.access_token)?;
        for (index, entity) in automation_entities.iter().enumerate() {
            let request_id = u64::try_from(index + 1).map_err(|_| {
                DefinitionError::Validation("too many automations to request".to_string())
            })?;
            match request_automation_config(&mut socket, request_id, &entity.entity_id) {
                Ok(raw) => match normalize_automation(
                    entity,
                    states.get(entity.entity_id.as_str()).copied(),
                    &raw,
                ) {
                    Ok(automation) => automations.push(automation),
                    Err(message) => diagnostics.push(diagnostic(
                        "automation",
                        &entity.entity_id,
                        "unsupported_automation_definition",
                        message,
                    )),
                },
                Err(failure) => diagnostics.push(diagnostic(
                    "automation",
                    &entity.entity_id,
                    "automation_definition_unavailable",
                    failure,
                )),
            }
        }
        let _ = socket.close(None);
    }

    let mut scenes = Vec::new();
    for entity in &scene_entities {
        if entity.platform != "homeassistant" || entity.unique_id == entity.entity_id {
            diagnostics.push(diagnostic(
                "scene",
                &entity.entity_id,
                "scene_definition_unavailable",
                format!(
                    "scene platform `{}` has no editable Home Assistant config identifier",
                    entity.platform
                ),
            ));
            continue;
        }
        match request_scene_config(config, &entity.unique_id) {
            Ok(raw) => match normalize_scene(entity, &raw) {
                Ok(scene) => scenes.push(scene),
                Err(message) => diagnostics.push(diagnostic(
                    "scene",
                    &entity.entity_id,
                    "unsupported_scene_definition",
                    message,
                )),
            },
            Err(SceneRequestError::Unavailable(message)) => diagnostics.push(diagnostic(
                "scene",
                &entity.entity_id,
                "scene_definition_unavailable",
                message,
            )),
            Err(SceneRequestError::Fatal(error)) => return Err(error),
        }
    }

    scenes.sort_by(|left, right| left.scene_id.cmp(&right.scene_id));
    automations.sort_by(|left, right| left.automation_id.cmp(&right.automation_id));
    diagnostics.sort_by(|left, right| {
        (
            &left.source_kind,
            &left.source_id,
            &left.code,
            &left.message,
        )
            .cmp(&(
                &right.source_kind,
                &right.source_id,
                &right.code,
                &right.message,
            ))
    });
    ensure_unique(scenes.iter().map(|scene| scene.scene_id.as_str()), "scene")?;
    ensure_unique(
        automations
            .iter()
            .map(|automation| automation.automation_id.as_str()),
        "automation",
    )?;

    let summary = DefinitionCollectionSummary {
        discovered_scenes: scene_entities.len(),
        collected_scenes: scenes.len(),
        skipped_scenes: scene_entities.len().saturating_sub(scenes.len()),
        discovered_automations: automation_entities.len(),
        collected_automations: automations.len(),
        skipped_automations: automation_entities.len().saturating_sub(automations.len()),
        diagnostics: diagnostics.len(),
    };
    base.scenes = scenes;
    base.automations = automations;

    Ok(EnrichedHomeAssistantExport {
        export: base,
        definition_collection: DefinitionCollectionReport {
            schema_version: DEFINITION_COLLECTION_SCHEMA_VERSION,
            source_instance_id: config.source_instance_id.clone(),
            topology_fingerprint,
            collected_at_ms: config.collected_at_ms,
            summary,
            diagnostics,
        },
    })
}

pub fn write_enriched_export_atomically(
    path: impl AsRef<Path>,
    export: &EnrichedHomeAssistantExport,
) -> Result<(), DefinitionError> {
    let path = path.as_ref();
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| DefinitionError::Config("output path has no file name".to_string()))?;
    let temporary = parent.join(format!(".{file_name}.tmp-{}", std::process::id()));
    let mut encoded = serde_json::to_vec_pretty(export)
        .map_err(|error| DefinitionError::Encode(error.to_string()))?;
    encoded.push(b'\n');
    let result = (|| {
        let mut file = File::create(&temporary).map_err(|error| DefinitionError::Io {
            operation: "create temporary enriched export",
            path: temporary.clone(),
            message: error.to_string(),
        })?;
        file.write_all(&encoded)
            .and_then(|()| file.sync_all())
            .map_err(|error| DefinitionError::Io {
                operation: "write temporary enriched export",
                path: temporary.clone(),
                message: error.to_string(),
            })?;
        fs::rename(&temporary, path).map_err(|error| DefinitionError::Io {
            operation: "replace enriched export",
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn validate_topology(
    topology: &HomeAssistantExport,
    config: &DefinitionCollectorConfig,
) -> Result<(), DefinitionError> {
    if topology.schema_version != EXPORT_SCHEMA_VERSION {
        return Err(DefinitionError::Validation(format!(
            "unsupported topology schema version {}",
            topology.schema_version
        )));
    }
    if topology.source_instance_id != config.source_instance_id {
        return Err(DefinitionError::Validation(format!(
            "topology source `{}` does not match collector source `{}`",
            topology.source_instance_id, config.source_instance_id
        )));
    }
    ensure_unique(
        topology
            .entities
            .iter()
            .map(|entity| entity.entity_id.as_str()),
        "entity",
    )
}

fn ensure_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    resource: &str,
) -> Result<(), DefinitionError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(DefinitionError::Validation(format!(
                "duplicate {resource} id `{value}`"
            )));
        }
    }
    Ok(())
}

fn diagnostic(
    source_kind: &str,
    source_id: &str,
    code: &str,
    message: String,
) -> DefinitionDiagnostic {
    DefinitionDiagnostic {
        severity: DefinitionDiagnosticSeverity::Warning,
        source_kind: source_kind.to_string(),
        source_id: source_id.to_string(),
        code: code.to_string(),
        message,
    }
}

type DefinitionSocket = WebSocket<MaybeTlsStream<TcpStream>>;

fn configure_socket_timeout(
    socket: &mut DefinitionSocket,
    timeout: Duration,
) -> Result<(), DefinitionError> {
    let stream = match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => stream,
        MaybeTlsStream::Rustls(stream) => &mut stream.sock,
        _ => {
            return Err(DefinitionError::Transport(
                "unsupported WebSocket stream backend".to_string(),
            ));
        }
    };
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|error| DefinitionError::Transport(error.to_string()))
}

fn authenticate(socket: &mut DefinitionSocket, access_token: &str) -> Result<(), DefinitionError> {
    let required = read_socket_json(socket)?;
    if required.get("type").and_then(JsonValue::as_str) != Some("auth_required") {
        return Err(DefinitionError::Protocol(
            "server did not begin with auth_required".to_string(),
        ));
    }
    send_socket_json(
        socket,
        &json!({"type": "auth", "access_token": access_token}),
    )?;
    let response = read_socket_json(socket)?;
    match response.get("type").and_then(JsonValue::as_str) {
        Some("auth_ok") => Ok(()),
        Some("auth_invalid") => Err(DefinitionError::Protocol(
            "Home Assistant rejected the access token".to_string(),
        )),
        _ => Err(DefinitionError::Protocol(
            "server did not complete authentication".to_string(),
        )),
    }
}

fn request_automation_config(
    socket: &mut DefinitionSocket,
    id: u64,
    entity_id: &str,
) -> Result<JsonValue, String> {
    send_socket_json(
        socket,
        &json!({"id": id, "type": AUTOMATION_CONFIG_COMMAND, "entity_id": entity_id}),
    )
    .map_err(|error| error.to_string())?;
    for _ in 0..MAX_UNMATCHED_MESSAGES {
        let response = read_socket_json(socket).map_err(|error| error.to_string())?;
        if response.get("id").and_then(JsonValue::as_u64) != Some(id) {
            continue;
        }
        if response.get("type").and_then(JsonValue::as_str) != Some("result") {
            return Err("response was not a result message".to_string());
        }
        if response.get("success").and_then(JsonValue::as_bool) != Some(true) {
            let code = response
                .pointer("/error/code")
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown");
            let message = response
                .pointer("/error/message")
                .and_then(JsonValue::as_str)
                .unwrap_or("definition request failed");
            return Err(format!("Home Assistant returned {code}: {message}"));
        }
        return response
            .get("result")
            .and_then(|result| result.get("config"))
            .cloned()
            .ok_or_else(|| "result did not contain config".to_string());
    }
    Err(format!(
        "no matching result after {MAX_UNMATCHED_MESSAGES} messages"
    ))
}

fn send_socket_json(
    socket: &mut DefinitionSocket,
    value: &JsonValue,
) -> Result<(), DefinitionError> {
    socket
        .send(Message::Text(value.to_string().into()))
        .map_err(|error| DefinitionError::Transport(error.to_string()))
}

fn read_socket_json(socket: &mut DefinitionSocket) -> Result<JsonValue, DefinitionError> {
    loop {
        let message = socket
            .read()
            .map_err(|error| DefinitionError::Transport(error.to_string()))?;
        match message {
            Message::Text(text) => {
                return serde_json::from_str(&text)
                    .map_err(|error| DefinitionError::Protocol(error.to_string()));
            }
            Message::Binary(bytes) => {
                return serde_json::from_slice(&bytes)
                    .map_err(|error| DefinitionError::Protocol(error.to_string()));
            }
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .map_err(|error| DefinitionError::Transport(error.to_string()))?,
            Message::Close(_) => {
                return Err(DefinitionError::Protocol(
                    "Home Assistant closed the WebSocket".to_string(),
                ));
            }
            _ => {}
        }
    }
}

fn normalize_scene(
    entity: &HomeAssistantEntity,
    raw: &JsonValue,
) -> Result<HomeAssistantScene, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "scene config is not an object".to_string())?;
    let name = object
        .get("name")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| "scene config has no name".to_string())?;
    let entities = object
        .get("entities")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "scene config has no entity state map".to_string())?;
    if entities.is_empty() {
        return Err("scene has no target states".to_string());
    }
    let mut states = entities
        .iter()
        .map(|(entity_id, value)| normalize_target_state(entity_id, value))
        .collect::<Result<Vec<_>, _>>()?;
    states.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));
    Ok(HomeAssistantScene {
        scene_id: entity.entity_id.clone(),
        name: name.to_string(),
        area_id: entity.area_id.clone(),
        states,
    })
}

fn normalize_target_state(
    entity_id: &str,
    value: &JsonValue,
) -> Result<HomeAssistantTargetState, String> {
    let (state, attributes) = match value {
        JsonValue::Object(object) => {
            let state = object
                .get("state")
                .and_then(json_scalar_string)
                .ok_or_else(|| format!("scene target `{entity_id}` has no scalar state"))?;
            let attributes = object
                .iter()
                .filter(|(name, _)| name.as_str() != "state")
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect();
            (state, attributes)
        }
        scalar => (
            json_scalar_string(scalar)
                .ok_or_else(|| format!("scene target `{entity_id}` has a non-scalar state"))?,
            BTreeMap::new(),
        ),
    };
    Ok(HomeAssistantTargetState {
        entity_id: entity_id.to_string(),
        state,
        attributes,
    })
}

fn normalize_automation(
    entity: &HomeAssistantEntity,
    state: Option<&smart_home_home_assistant_migration::HomeAssistantState>,
    raw: &JsonValue,
) -> Result<HomeAssistantAutomation, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "automation config is not an object".to_string())?;
    let automation_id = object
        .get("id")
        .and_then(json_scalar_string)
        .unwrap_or_else(|| entity.unique_id.clone());
    let alias = object
        .get("alias")
        .and_then(JsonValue::as_str)
        .or_else(|| {
            state.and_then(|state| {
                state
                    .attributes
                    .get("friendly_name")
                    .and_then(JsonValue::as_str)
            })
        })
        .unwrap_or(&entity.entity_id)
        .to_string();
    let triggers = config_items(object, "triggers", "trigger")?;
    if triggers.len() != 1 {
        return Err(format!(
            "automation must have exactly one migratable trigger, found {}",
            triggers.len()
        ));
    }
    let trigger = normalize_trigger(triggers[0])?;
    let conditions = config_items_or_empty(object, "conditions", "condition")?
        .into_iter()
        .map(normalize_condition)
        .collect::<Result<Vec<_>, _>>()?;
    let actions = config_items(object, "actions", "action")?
        .into_iter()
        .map(normalize_action)
        .collect::<Result<Vec<_>, _>>()?;
    if actions.is_empty() {
        return Err("automation has no actions".to_string());
    }
    Ok(HomeAssistantAutomation {
        automation_id,
        alias,
        enabled: state.is_none_or(|state| state.state != "off"),
        trigger,
        conditions,
        actions,
    })
}

fn config_items<'a>(
    object: &'a JsonMap<String, JsonValue>,
    plural: &str,
    singular: &str,
) -> Result<Vec<&'a JsonValue>, String> {
    let value = object
        .get(plural)
        .or_else(|| object.get(singular))
        .ok_or_else(|| format!("config has no `{plural}` or `{singular}` field"))?;
    match value {
        JsonValue::Array(items) => Ok(items.iter().collect()),
        JsonValue::Object(_) => Ok(vec![value]),
        _ => Err(format!("`{plural}` is not an object or array")),
    }
}

fn config_items_or_empty<'a>(
    object: &'a JsonMap<String, JsonValue>,
    plural: &str,
    singular: &str,
) -> Result<Vec<&'a JsonValue>, String> {
    if !object.contains_key(plural) && !object.contains_key(singular) {
        return Ok(Vec::new());
    }
    let values = config_items(object, plural, singular)?;
    if values.len() == 1 && values[0].is_null() {
        Ok(Vec::new())
    } else {
        Ok(values)
    }
}

fn normalize_trigger(raw: &JsonValue) -> Result<HomeAssistantAutomationTrigger, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "trigger is not an object".to_string())?;
    let kind = object
        .get("trigger")
        .or_else(|| object.get("platform"))
        .and_then(JsonValue::as_str)
        .ok_or_else(|| "trigger has no type".to_string())?;
    match kind {
        "state" => {
            if object.contains_key("for") || object.contains_key("from") {
                return Err(
                    "state trigger with `from` or `for` is outside the safe subset".to_string(),
                );
            }
            let entity_id = one_entity_id(object.get("entity_id"), "state trigger")?;
            let to = object.get("to").and_then(json_scalar_string);
            Ok(HomeAssistantAutomationTrigger::State { entity_id, to })
        }
        "time_pattern" => Ok(HomeAssistantAutomationTrigger::Interval {
            every_ms: time_pattern_interval_ms(object)?,
            offset_ms: 0,
        }),
        other => Err(format!("trigger `{other}` is outside the safe subset")),
    }
}

fn time_pattern_interval_ms(object: &JsonMap<String, JsonValue>) -> Result<u64, String> {
    let fields = [
        ("seconds", 1_000_u64),
        ("minutes", 60_000),
        ("hours", 3_600_000),
    ];
    let mut interval = None;
    for (field, multiplier) in fields {
        let Some(value) = object.get(field) else {
            continue;
        };
        let Some(text) = json_scalar_string(value) else {
            return Err(format!("time pattern `{field}` is not scalar"));
        };
        if text == "*" {
            continue;
        }
        let Some(step) = text.strip_prefix('/') else {
            return Err(format!(
                "time pattern `{field}` must be wildcard or slash interval"
            ));
        };
        let step = step
            .parse::<u64>()
            .map_err(|_| format!("time pattern `{field}` has invalid interval"))?;
        if step == 0 || interval.is_some() {
            return Err("time pattern must contain one non-zero slash interval".to_string());
        }
        interval = Some(step.saturating_mul(multiplier));
    }
    interval.ok_or_else(|| "time pattern has no slash interval".to_string())
}

fn normalize_condition(raw: &JsonValue) -> Result<HomeAssistantStateCondition, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "condition is not an object".to_string())?;
    if object.get("condition").and_then(JsonValue::as_str) != Some("state") {
        return Err("only state conditions are in the safe subset".to_string());
    }
    if object.contains_key("for") {
        return Err("state conditions with `for` are outside the safe subset".to_string());
    }
    Ok(HomeAssistantStateCondition {
        entity_id: one_entity_id(object.get("entity_id"), "state condition")?,
        state: object
            .get("state")
            .and_then(json_scalar_string)
            .ok_or_else(|| "state condition has no scalar state".to_string())?,
    })
}

fn normalize_action(raw: &JsonValue) -> Result<HomeAssistantServiceAction, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| "action is not an object".to_string())?;
    let service = object
        .get("action")
        .or_else(|| object.get("service"))
        .and_then(JsonValue::as_str)
        .ok_or_else(|| "action has no service".to_string())?;
    if !matches!(
        service,
        "scene.turn_on"
            | "light.turn_on"
            | "light.turn_off"
            | "switch.turn_on"
            | "switch.turn_off"
            | "lock.lock"
            | "lock.unlock"
            | "climate.set_temperature"
    ) {
        return Err(format!("service `{service}` is outside the safe subset"));
    }
    let target = object.get("target").and_then(JsonValue::as_object);
    if target
        .is_some_and(|target| target.contains_key("device_id") || target.contains_key("area_id"))
    {
        return Err(
            "device and area action targets require expansion before migration".to_string(),
        );
    }
    let data = object
        .get("data")
        .and_then(JsonValue::as_object)
        .cloned()
        .unwrap_or_default();
    let entity_source = target
        .and_then(|target| target.get("entity_id"))
        .or_else(|| object.get("entity_id"))
        .or_else(|| data.get("entity_id"));
    let target_entity_ids = entity_ids(entity_source, "service action")?;
    if target_entity_ids.is_empty() {
        return Err(format!("service `{service}` has no entity target"));
    }
    let data = data
        .into_iter()
        .filter(|(name, _)| name != "entity_id")
        .collect::<BTreeMap<_, _>>();
    if service == "climate.set_temperature" && !data.contains_key("temperature") {
        return Err("climate.set_temperature has no temperature".to_string());
    }
    Ok(HomeAssistantServiceAction {
        service: service.to_string(),
        target_entity_ids,
        data,
    })
}

fn one_entity_id(value: Option<&JsonValue>, context: &str) -> Result<String, String> {
    let values = entity_ids(value, context)?;
    if values.len() != 1 {
        return Err(format!("{context} must reference exactly one entity"));
    }
    values
        .into_iter()
        .next()
        .ok_or_else(|| format!("{context} has no entity"))
}

fn entity_ids(value: Option<&JsonValue>, context: &str) -> Result<Vec<String>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    match value {
        JsonValue::String(value) => Ok(vec![value.clone()]),
        JsonValue::Array(values) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| format!("{context} entity list contains a non-string"))
            })
            .collect(),
        _ => Err(format!("{context} entity target is not a string or list")),
    }
}

fn json_scalar_string(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(value) => Some(value.clone()),
        JsonValue::Bool(true) => Some("on".to_string()),
        JsonValue::Bool(false) => Some("off".to_string()),
        JsonValue::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn entity_domain(entity_id: &str) -> &str {
    entity_id.split_once('.').map_or("", |(domain, _)| domain)
}

enum SceneRequestError {
    Unavailable(String),
    Fatal(DefinitionError),
}

fn request_scene_config(
    config: &DefinitionCollectorConfig,
    config_id: &str,
) -> Result<JsonValue, SceneRequestError> {
    let mut base = Url::parse(&config.rest_base_url).map_err(|error| {
        SceneRequestError::Fatal(DefinitionError::Config(format!(
            "invalid REST URL: {error}"
        )))
    })?;
    let prefix = base.path.trim_end_matches('/');
    base.path = format!(
        "{prefix}/api/config/scene/config/{}",
        percent_encode(config_id)
    );
    base.query = None;
    base.fragment = None;
    let response = http_get_json(&base, config).map_err(SceneRequestError::Fatal)?;
    match response.status {
        200 => serde_json::from_slice(&response.body).map_err(|error| {
            SceneRequestError::Unavailable(format!("scene response was invalid JSON: {error}"))
        }),
        404 => Err(SceneRequestError::Unavailable(
            "Home Assistant config endpoint returned 404".to_string(),
        )),
        401 | 403 => Err(SceneRequestError::Fatal(DefinitionError::Protocol(
            "Home Assistant scene config endpoint requires an administrator token".to_string(),
        ))),
        status => Err(SceneRequestError::Unavailable(format!(
            "Home Assistant config endpoint returned HTTP {status}"
        ))),
    }
}

struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

enum HttpStream {
    Plain(TcpStream),
    Tls(Box<dyn tls_platform::TlsStream>),
}

impl Read for HttpStream {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.read(buffer),
            Self::Tls(stream) => stream.read(buffer),
        }
    }
}

impl Write for HttpStream {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Plain(stream) => stream.write(buffer),
            Self::Tls(stream) => stream.write(buffer),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Plain(stream) => stream.flush(),
            Self::Tls(stream) => stream.flush(),
        }
    }
}

fn http_get_json(
    url: &Url,
    config: &DefinitionCollectorConfig,
) -> Result<HttpResponse, DefinitionError> {
    let host = url
        .host
        .as_deref()
        .ok_or_else(|| DefinitionError::Config("REST URL has no host".to_string()))?;
    let port = url
        .effective_port()
        .ok_or_else(|| DefinitionError::Config("REST URL has no port".to_string()))?;
    let mut stream = match url.scheme.as_str() {
        "http" => HttpStream::Plain(connect_plain(host, port, config.io_timeout)?),
        "https" => {
            let mut tls = TlsConfig::https_default();
            tls.connect_timeout = config.io_timeout;
            tls.handshake_timeout = config.io_timeout;
            tls.read_timeout = Some(config.io_timeout);
            tls.write_timeout = Some(config.io_timeout);
            HttpStream::Tls(
                default_connector()
                    .connect(host, port, &tls)
                    .map_err(|error| DefinitionError::Transport(error.to_string()))?,
            )
        }
        scheme => {
            return Err(DefinitionError::Config(format!(
                "unsupported REST URL scheme `{scheme}`"
            )));
        }
    };
    let host_header = if url.port.is_some() {
        format!("{host}:{port}")
    } else {
        host.to_string()
    };
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {host_header}\r\nAuthorization: Bearer {}\r\nAccept: application/json\r\nConnection: close\r\n\r\n",
        url.path, config.access_token
    );
    stream
        .write_all(request.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|error| DefinitionError::Transport(error.to_string()))?;
    let bytes = read_bounded(&mut stream, config.max_response_bytes)
        .map_err(|error| DefinitionError::Transport(error.to_string()))?;
    decode_http_response(&bytes, config.max_response_bytes)
}

fn connect_plain(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, DefinitionError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| DefinitionError::Transport(error.to_string()))?
        .collect::<Vec<SocketAddr>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .and_then(|()| stream.set_write_timeout(Some(timeout)))
                    .map_err(|error| DefinitionError::Transport(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(DefinitionError::Transport(last_error.map_or_else(
        || "host resolved to no addresses".to_string(),
        |error| error.to_string(),
    )))
}

fn read_bounded(reader: &mut dyn Read, max_bytes: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(max_bytes as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > max_bytes {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("response exceeds {max_bytes} bytes"),
        ));
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], max_bytes: usize) -> Result<HttpResponse, DefinitionError> {
    let parsed =
        parse_response_head(bytes).map_err(|error| DefinitionError::Protocol(error.to_string()))?;
    let available = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if available.len() < expected {
                return Err(DefinitionError::Protocol(format!(
                    "HTTP body was truncated: expected {expected}, received {}",
                    available.len()
                )));
            }
            available[..expected].to_vec()
        }
        BodyKind::UntilEof => available.to_vec(),
        BodyKind::Chunked => decode_chunked_body(available, max_bytes)?,
    };
    if body.len() > max_bytes {
        return Err(DefinitionError::Protocol(format!(
            "HTTP body exceeds {max_bytes} bytes"
        )));
    }
    Ok(HttpResponse {
        status: parsed.head.status,
        body,
    })
}

fn decode_chunked_body(input: &[u8], max_bytes: usize) -> Result<Vec<u8>, DefinitionError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let line_end = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| cursor + offset)
            .ok_or_else(|| {
                DefinitionError::Protocol("missing chunk-size terminator".to_string())
            })?;
        let size_text = std::str::from_utf8(&input[cursor..line_end])
            .map_err(|_| DefinitionError::Protocol("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| DefinitionError::Protocol("invalid chunk size".to_string()))?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > max_bytes.saturating_sub(output.len()) {
            return Err(DefinitionError::Protocol(format!(
                "chunked HTTP body exceeds {max_bytes} bytes"
            )));
        }
        let end = cursor
            .checked_add(size)
            .ok_or_else(|| DefinitionError::Protocol("chunk size overflow".to_string()))?;
        if end + 2 > input.len() || &input[end..end + 2] != b"\r\n" {
            return Err(DefinitionError::Protocol(
                "truncated chunk payload".to_string(),
            ));
        }
        output.extend_from_slice(&input[cursor..end]);
        cursor = end + 2;
    }
}

fn redact(message: String, secret: &str) -> String {
    if secret.is_empty() {
        message
    } else {
        message.replace(secret, "[REDACTED]")
    }
}
