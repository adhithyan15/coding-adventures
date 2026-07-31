//! Live Home Assistant history collection and durable D23 event migration.

#![forbid(unsafe_code)]

use chrono::DateTime;
use coding_adventures_sha256::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
use smart_home_automation_runtime::{AutomationRuntimeSnapshot, SmartHomeAutomationRuntime};
use smart_home_core::{
    Capability, DeviceEvent, DeviceEventType, EventId, Metadata, StateDelta, Value, ValueKind,
};
use smart_home_home_assistant_migration::{
    apply_plan, plan_export, HomeAssistantExport, HomeAssistantMigrationPlan, MigrationReceipt,
};
use smart_home_runtime::{RuntimeDurableSnapshot, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

pub const HISTORY_EXPORT_SCHEMA_VERSION: u32 = 1;
pub const HISTORY_ARTIFACT_SCHEMA_VERSION: u32 = 1;
const HISTORY_COMMAND: &str = "history/history_during_period";
const MAX_UNMATCHED_MESSAGES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCollectorConfig {
    pub websocket_url: String,
    pub access_token: String,
    pub source_instance_id: String,
    pub start_time: String,
    pub end_time: String,
    pub collected_at_ms: u64,
    pub batch_size: usize,
    pub io_timeout: Duration,
}

impl HistoryCollectorConfig {
    pub fn validate(&self) -> Result<(), HistoryError> {
        if !self.websocket_url.starts_with("ws://") && !self.websocket_url.starts_with("wss://") {
            return Err(HistoryError::Validation(
                "Home Assistant URL must use ws:// or wss://".to_string(),
            ));
        }
        if self.access_token.trim().is_empty() {
            return Err(HistoryError::Validation(
                "Home Assistant access token is empty".to_string(),
            ));
        }
        if self.source_instance_id.trim().is_empty() {
            return Err(HistoryError::Validation(
                "source instance id is empty".to_string(),
            ));
        }
        if self.batch_size == 0 {
            return Err(HistoryError::Validation(
                "history batch size must be greater than zero".to_string(),
            ));
        }
        if self.io_timeout.is_zero() {
            return Err(HistoryError::Validation(
                "I/O timeout must be greater than zero".to_string(),
            ));
        }
        let start = parse_timestamp_ms(&self.start_time)?;
        let end = parse_timestamp_ms(&self.end_time)?;
        if start >= end {
            return Err(HistoryError::Validation(
                "history start_time must be before end_time".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantHistoryExport {
    pub schema_version: u32,
    pub source_instance_id: String,
    pub collected_at_ms: u64,
    pub start_time: String,
    pub end_time: String,
    pub entities: Vec<HomeAssistantEntityHistory>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantEntityHistory {
    pub entity_id: String,
    pub states: Vec<HomeAssistantHistoricalState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantHistoricalState {
    pub state: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, JsonValue>,
    pub last_changed: String,
    pub last_updated: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryDiagnostic {
    pub severity: HistoryDiagnosticSeverity,
    pub code: String,
    pub source_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryMigrationSummary {
    pub entities_requested: usize,
    pub entities_with_history: usize,
    pub source_states: usize,
    pub planned_events: usize,
    pub warnings: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantHistoryMigrationPlan {
    pub source_instance_id: String,
    pub source_fingerprint: String,
    pub topology_fingerprint: String,
    pub collected_at_ms: u64,
    pub start_time: String,
    pub end_time: String,
    pub events: Vec<DeviceEvent>,
    pub diagnostics: Vec<HistoryDiagnostic>,
    pub summary: HistoryMigrationSummary,
}

impl HomeAssistantHistoryMigrationPlan {
    pub fn is_blocked(&self) -> bool {
        self.summary.errors > 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryApplyCounts {
    pub inserted_events: usize,
    pub skipped_identical_events: usize,
    pub restored_current_states: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryMigrationReceipt {
    pub migration_id: String,
    pub source_instance_id: String,
    pub source_fingerprint: String,
    pub applied_at_ms: u64,
    pub counts: HistoryApplyCounts,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantHistoryMigrationArtifact {
    pub schema_version: u32,
    pub dry_run: bool,
    pub history_export: HomeAssistantHistoryExport,
    pub topology_plan: HomeAssistantMigrationPlan,
    pub history_plan: HomeAssistantHistoryMigrationPlan,
    #[serde(default)]
    pub topology_receipt: Option<MigrationReceipt>,
    #[serde(default)]
    pub history_receipt: Option<HistoryMigrationReceipt>,
    #[serde(default)]
    pub runtime_snapshot: Option<RuntimeDurableSnapshot>,
    #[serde(default)]
    pub automation_snapshot: Option<AutomationRuntimeSnapshot>,
}

#[derive(Debug)]
pub enum HistoryError {
    Usage(String),
    Validation(String),
    Transport(String),
    Protocol(String),
    Decode(String),
    Encode(String),
    Migration(String),
    Runtime(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
}

impl fmt::Display for HistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Validation(message) => f.write_str(message),
            Self::Transport(message) => write!(f, "Home Assistant transport failed: {message}"),
            Self::Protocol(message) => {
                write!(f, "Home Assistant history protocol failed: {message}")
            }
            Self::Decode(message) => write!(f, "could not decode history data: {message}"),
            Self::Encode(message) => write!(f, "could not encode history artifact: {message}"),
            Self::Migration(message) => write!(f, "could not plan topology migration: {message}"),
            Self::Runtime(message) => write!(f, "could not apply history migration: {message}"),
            Self::Io {
                operation,
                path,
                message,
            } => write!(f, "could not {operation} {}: {message}", path.display()),
        }
    }
}

impl std::error::Error for HistoryError {}

pub fn migrate_live_history(
    topology_export: &HomeAssistantExport,
    config: &HistoryCollectorConfig,
    dry_run: bool,
) -> Result<HomeAssistantHistoryMigrationArtifact, HistoryError> {
    config.validate()?;
    if topology_export.source_instance_id != config.source_instance_id {
        return Err(HistoryError::Validation(format!(
            "history source instance {} does not match topology source instance {}",
            config.source_instance_id, topology_export.source_instance_id
        )));
    }
    let topology_plan =
        plan_export(topology_export).map_err(|error| HistoryError::Migration(error.to_string()))?;
    if topology_plan.is_blocked() {
        return Err(HistoryError::Migration(format!(
            "topology plan is blocked by {} errors",
            topology_plan.summary.errors
        )));
    }
    let entity_ids = topology_source_entity_ids(&topology_plan)?;
    let history_export = collect_history(config, &entity_ids)?;
    let history_plan = plan_history(&history_export, &topology_plan)?;

    if dry_run {
        return Ok(HomeAssistantHistoryMigrationArtifact {
            schema_version: HISTORY_ARTIFACT_SCHEMA_VERSION,
            dry_run: true,
            history_export,
            topology_plan,
            history_plan,
            topology_receipt: None,
            history_receipt: None,
            runtime_snapshot: None,
            automation_snapshot: None,
        });
    }

    let mut runtime = SmartHomeRuntime::new();
    let mut automations = SmartHomeAutomationRuntime::new();
    let topology_receipt = apply_plan(&topology_plan, &mut runtime, &mut automations)
        .map_err(|error| HistoryError::Runtime(error.to_string()))?;
    let history_receipt = apply_history_plan(&history_plan, &topology_plan, &mut runtime)?;

    Ok(HomeAssistantHistoryMigrationArtifact {
        schema_version: HISTORY_ARTIFACT_SCHEMA_VERSION,
        dry_run: false,
        history_export,
        topology_plan,
        history_plan,
        topology_receipt: Some(topology_receipt),
        history_receipt: Some(history_receipt),
        runtime_snapshot: Some(runtime.durable_snapshot()),
        automation_snapshot: Some(automations.snapshot()),
    })
}

pub fn collect_history(
    config: &HistoryCollectorConfig,
    entity_ids: &[String],
) -> Result<HomeAssistantHistoryExport, HistoryError> {
    config.validate()?;
    ensure_unique_nonempty(entity_ids)?;
    let (mut socket, _) = connect(config.websocket_url.as_str()).map_err(|error| {
        HistoryError::Transport(redact_token(error.to_string(), &config.access_token))
    })?;
    configure_socket_timeout(&mut socket, config.io_timeout)?;
    authenticate(&mut socket, &config.access_token)?;

    let requested = entity_ids.iter().cloned().collect::<BTreeSet<_>>();
    let mut histories = requested
        .iter()
        .map(|entity_id| (entity_id.clone(), Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (request_id, batch) in (1u64..).zip(entity_ids.chunks(config.batch_size)) {
        let result = request_history(&mut socket, request_id, config, batch)?;
        for (entity_id, raw_states) in result {
            if !requested.contains(&entity_id) {
                return Err(HistoryError::Protocol(format!(
                    "history response included unrequested entity {entity_id}"
                )));
            }
            let target = histories.get_mut(&entity_id).ok_or_else(|| {
                HistoryError::Protocol(format!("missing requested entity {entity_id}"))
            })?;
            for raw in raw_states {
                if raw.entity_id.as_deref().is_some_and(|id| id != entity_id) {
                    return Err(HistoryError::Protocol(format!(
                        "history state entity does not match response key {entity_id}"
                    )));
                }
                parse_timestamp_ms(&raw.last_changed)?;
                parse_timestamp_ms(&raw.last_updated)?;
                target.push(HomeAssistantHistoricalState {
                    state: raw.state,
                    attributes: raw.attributes,
                    last_changed: raw.last_changed,
                    last_updated: raw.last_updated,
                });
            }
        }
    }
    let _ = socket.close(None);

    let mut entities = histories
        .into_iter()
        .map(|(entity_id, mut states)| {
            states.sort_by(|left, right| {
                timestamp_sort_key(&left.last_updated)
                    .cmp(&timestamp_sort_key(&right.last_updated))
                    .then_with(|| left.last_changed.cmp(&right.last_changed))
                    .then_with(|| left.state.cmp(&right.state))
            });
            HomeAssistantEntityHistory { entity_id, states }
        })
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));

    Ok(HomeAssistantHistoryExport {
        schema_version: HISTORY_EXPORT_SCHEMA_VERSION,
        source_instance_id: config.source_instance_id.clone(),
        collected_at_ms: config.collected_at_ms,
        start_time: config.start_time.clone(),
        end_time: config.end_time.clone(),
        entities,
    })
}

pub fn plan_history(
    history: &HomeAssistantHistoryExport,
    topology: &HomeAssistantMigrationPlan,
) -> Result<HomeAssistantHistoryMigrationPlan, HistoryError> {
    if history.schema_version != HISTORY_EXPORT_SCHEMA_VERSION {
        return Err(HistoryError::Validation(format!(
            "unsupported Home Assistant history schema {}",
            history.schema_version
        )));
    }
    if history.source_instance_id != topology.source_instance_id {
        return Err(HistoryError::Validation(
            "history and topology source instances differ".to_string(),
        ));
    }
    let canonical =
        serde_json::to_vec(history).map_err(|error| HistoryError::Encode(error.to_string()))?;
    let source_fingerprint = sha256_hex(&canonical);
    let entity_map = topology
        .entities
        .iter()
        .filter_map(|entity| {
            metadata_value(&entity.metadata, "home_assistant.entity_id")
                .map(|source_id| (source_id.to_string(), entity))
        })
        .collect::<BTreeMap<_, _>>();
    let device_map = topology
        .devices
        .iter()
        .map(|device| (device.device_id.clone(), device))
        .collect::<BTreeMap<_, _>>();
    let mut events = Vec::new();
    let mut event_ids = BTreeSet::new();
    let mut diagnostics = Vec::new();

    for entity_history in &history.entities {
        let Some(entity) = entity_map.get(&entity_history.entity_id) else {
            diagnostics.push(diagnostic(
                HistoryDiagnosticSeverity::Error,
                "unknown_entity",
                &entity_history.entity_id,
                "history entity is absent from the topology migration plan",
            ));
            continue;
        };
        let device = device_map.get(&entity.device_id).ok_or_else(|| {
            HistoryError::Validation(format!(
                "topology entity {} references missing device {}",
                entity.entity_id, entity.device_id
            ))
        })?;
        let Some(primary_capability) = entity.capabilities.first() else {
            diagnostics.push(diagnostic(
                HistoryDiagnosticSeverity::Error,
                "missing_capability",
                &entity_history.entity_id,
                "history entity has no D23 capability",
            ));
            continue;
        };

        for state in &entity_history.states {
            let (value, warning) = project_value(state, primary_capability);
            if let Some(message) = warning {
                diagnostics.push(diagnostic(
                    HistoryDiagnosticSeverity::Warning,
                    "state_value_lossy",
                    &entity_history.entity_id,
                    message,
                ));
            }
            let primary_event = history_event(
                history,
                topology,
                entity,
                device,
                state,
                primary_capability,
                value,
            )?;
            if event_ids.insert(primary_event.event_id.clone()) {
                events.push(primary_event);
            } else {
                diagnostics.push(diagnostic(
                    HistoryDiagnosticSeverity::Info,
                    "duplicate_source_state",
                    &entity_history.entity_id,
                    "an identical historical state was returned more than once",
                ));
            }

            if let Some(brightness_capability) = entity
                .capabilities
                .iter()
                .find(|capability| capability.capability_id.as_str() == "light.brightness")
            {
                if let Some(brightness) = brightness_percentage(&state.attributes) {
                    let brightness_event = history_event(
                        history,
                        topology,
                        entity,
                        device,
                        state,
                        brightness_capability,
                        Value::Percentage(brightness),
                    )?;
                    if event_ids.insert(brightness_event.event_id.clone()) {
                        events.push(brightness_event);
                    }
                }
            }
        }
    }
    events.sort_by(|left, right| {
        left.observed_at_ms
            .cmp(&right.observed_at_ms)
            .then_with(|| left.entity_id.cmp(&right.entity_id))
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
    let summary = HistoryMigrationSummary {
        entities_requested: history.entities.len(),
        entities_with_history: history
            .entities
            .iter()
            .filter(|entity| !entity.states.is_empty())
            .count(),
        source_states: history
            .entities
            .iter()
            .map(|entity| entity.states.len())
            .sum(),
        planned_events: events.len(),
        warnings: diagnostics
            .iter()
            .filter(|item| item.severity == HistoryDiagnosticSeverity::Warning)
            .count(),
        errors: diagnostics
            .iter()
            .filter(|item| item.severity == HistoryDiagnosticSeverity::Error)
            .count(),
    };

    Ok(HomeAssistantHistoryMigrationPlan {
        source_instance_id: history.source_instance_id.clone(),
        source_fingerprint,
        topology_fingerprint: topology.source_fingerprint.clone(),
        collected_at_ms: history.collected_at_ms,
        start_time: history.start_time.clone(),
        end_time: history.end_time.clone(),
        events,
        diagnostics,
        summary,
    })
}

pub fn apply_history_plan(
    plan: &HomeAssistantHistoryMigrationPlan,
    topology: &HomeAssistantMigrationPlan,
    runtime: &mut SmartHomeRuntime,
) -> Result<HistoryMigrationReceipt, HistoryError> {
    if plan.is_blocked() {
        return Err(HistoryError::Runtime(format!(
            "history plan is blocked by {} errors",
            plan.summary.errors
        )));
    }
    if plan.topology_fingerprint != topology.source_fingerprint {
        return Err(HistoryError::Runtime(
            "history plan topology fingerprint does not match topology plan".to_string(),
        ));
    }
    let mut counts = HistoryApplyCounts::default();
    for event in &plan.events {
        if let Some(existing) = runtime.registry().event(&event.event_id) {
            if existing == event {
                counts.skipped_identical_events += 1;
                continue;
            }
            return Err(HistoryError::Runtime(format!(
                "history event id {} conflicts with an existing event",
                event.event_id
            )));
        }
        runtime
            .apply_device_event(event.clone())
            .map_err(|error| HistoryError::Runtime(error.to_string()))?;
        counts.inserted_events += 1;
    }

    for entity in &topology.entities {
        if let Some(state) = &entity.state {
            runtime
                .registry_mut()
                .apply_state_snapshot(state.clone())
                .map_err(|error| HistoryError::Runtime(error.to_string()))?;
            counts.restored_current_states += 1;
        }
    }
    Ok(HistoryMigrationReceipt {
        migration_id: format!("ha-history:{}", plan.source_fingerprint),
        source_instance_id: plan.source_instance_id.clone(),
        source_fingerprint: plan.source_fingerprint.clone(),
        applied_at_ms: plan.collected_at_ms,
        counts,
    })
}

pub fn write_artifact_atomically(
    path: impl AsRef<Path>,
    artifact: &HomeAssistantHistoryMigrationArtifact,
) -> Result<(), HistoryError> {
    let path = path.as_ref();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| HistoryError::Io {
            operation: "create output directory",
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| HistoryError::Validation("output path has no file name".to_string()))?;
    let temporary = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.tmp-{}", std::process::id()));
    let mut body = serde_json::to_vec_pretty(artifact)
        .map_err(|error| HistoryError::Encode(error.to_string()))?;
    body.push(b'\n');
    let result = (|| {
        let mut file = File::create(&temporary).map_err(|error| HistoryError::Io {
            operation: "create temporary artifact",
            path: temporary.clone(),
            message: error.to_string(),
        })?;
        file.write_all(&body)
            .and_then(|()| file.sync_all())
            .map_err(|error| HistoryError::Io {
                operation: "write temporary artifact",
                path: temporary.clone(),
                message: error.to_string(),
            })?;
        fs::rename(&temporary, path).map_err(|error| HistoryError::Io {
            operation: "replace artifact",
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn topology_source_entity_ids(
    topology: &HomeAssistantMigrationPlan,
) -> Result<Vec<String>, HistoryError> {
    let mut ids = topology
        .entities
        .iter()
        .map(|entity| {
            metadata_value(&entity.metadata, "home_assistant.entity_id")
                .map(str::to_string)
                .ok_or_else(|| {
                    HistoryError::Validation(format!(
                        "topology entity {} has no Home Assistant source id",
                        entity.entity_id
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn metadata_value<'a>(metadata: &'a [Metadata], key: &str) -> Option<&'a str> {
    metadata
        .iter()
        .find(|item| item.key == key)
        .map(|item| item.value.as_str())
}

fn history_event(
    history: &HomeAssistantHistoryExport,
    topology: &HomeAssistantMigrationPlan,
    entity: &smart_home_core::Entity,
    device: &smart_home_core::Device,
    state: &HomeAssistantHistoricalState,
    capability: &Capability,
    value: Value,
) -> Result<DeviceEvent, HistoryError> {
    let observed_at_ms = parse_timestamp_ms(&state.last_updated)?;
    let event_type = if matches!(state.state.as_str(), "unknown" | "unavailable") {
        DeviceEventType::Unavailable
    } else {
        DeviceEventType::Updated
    };
    let attributes = serde_json::to_string(&state.attributes)
        .map_err(|error| HistoryError::Encode(error.to_string()))?;
    let identity = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        history.source_instance_id,
        metadata_value(&entity.metadata, "home_assistant.entity_id").unwrap_or("unknown"),
        state.last_changed,
        state.last_updated,
        state.state,
        capability.capability_id,
        attributes,
    );
    let fingerprint = sha256_hex(identity.as_bytes());
    Ok(DeviceEvent {
        event_id: EventId::trusted(format!("ha-history:{fingerprint}")),
        bridge_id: topology.bridge.bridge_id.clone(),
        device_id: Some(device.device_id.clone()),
        entity_id: Some(entity.entity_id.clone()),
        observed_at_ms,
        received_at_ms: observed_at_ms,
        event_type,
        state_delta: Some(StateDelta {
            capability_id: capability.capability_id.clone(),
            value,
        }),
        raw_ref: Some(format!(
            "home-assistant-history:{}:{}",
            history.source_instance_id, state.last_updated
        )),
        correlation_id: None,
        metadata: vec![
            Metadata::new("migration.source", "home_assistant_history"),
            Metadata::new(
                "home_assistant.entity_id",
                metadata_value(&entity.metadata, "home_assistant.entity_id").unwrap_or(""),
            ),
            Metadata::new("home_assistant.state", &state.state),
            Metadata::new("home_assistant.last_changed", &state.last_changed),
            Metadata::new("home_assistant.last_updated", &state.last_updated),
            Metadata::new("home_assistant.attributes", attributes),
        ],
    })
}

fn project_value(
    state: &HomeAssistantHistoricalState,
    capability: &Capability,
) -> (Value, Option<String>) {
    if matches!(state.state.as_str(), "unknown" | "unavailable") {
        return (Value::Null, None);
    }
    match capability.value_kind {
        ValueKind::Null => (Value::Null, None),
        ValueKind::Boolean => (
            Value::Bool(matches!(
                state.state.as_str(),
                "on" | "open" | "detected" | "occupied" | "home" | "locked"
            )),
            None,
        ),
        ValueKind::Integer => state.state.parse::<i64>().map_or_else(
            |_| {
                (
                    Value::Null,
                    Some(format!("state {} is not an integer", state.state)),
                )
            },
            |value| (Value::Integer(value), None),
        ),
        ValueKind::Number => state.state.parse::<f64>().map_or_else(
            |_| {
                (
                    Value::Null,
                    Some(format!("state {} is not a number", state.state)),
                )
            },
            |value| (Value::Number(value), None),
        ),
        ValueKind::Percentage => brightness_percentage(&state.attributes).map_or_else(
            || {
                state.state.parse::<f64>().map_or_else(
                    |_| {
                        (
                            Value::Null,
                            Some(format!("state {} is not a percentage", state.state)),
                        )
                    },
                    |value| {
                        (
                            Value::Percentage(value.round().clamp(0.0, 100.0) as u8),
                            None,
                        )
                    },
                )
            },
            |value| (Value::Percentage(value), None),
        ),
        ValueKind::Text => (Value::Text(state.state.clone()), None),
        ValueKind::Object | ValueKind::Array => (
            Value::Text(state.state.clone()),
            Some("structured capability retained as source text".to_string()),
        ),
    }
}

fn brightness_percentage(attributes: &BTreeMap<String, JsonValue>) -> Option<u8> {
    if let Some(value) = attributes.get("brightness_pct").and_then(json_number) {
        return Some(value.round().clamp(0.0, 100.0) as u8);
    }
    attributes
        .get("brightness")
        .and_then(json_number)
        .map(|value| ((value.clamp(0.0, 255.0) / 255.0) * 100.0).round() as u8)
}

fn json_number(value: &JsonValue) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn diagnostic(
    severity: HistoryDiagnosticSeverity,
    code: impl Into<String>,
    source_id: impl Into<String>,
    message: impl Into<String>,
) -> HistoryDiagnostic {
    HistoryDiagnostic {
        severity,
        code: code.into(),
        source_id: source_id.into(),
        message: message.into(),
    }
}

fn ensure_unique_nonempty(entity_ids: &[String]) -> Result<(), HistoryError> {
    if entity_ids.is_empty() {
        return Err(HistoryError::Validation(
            "history collection requires at least one entity".to_string(),
        ));
    }
    let mut seen = BTreeSet::new();
    for entity_id in entity_ids {
        if entity_id.trim().is_empty() {
            return Err(HistoryError::Validation(
                "history entity id is empty".to_string(),
            ));
        }
        if !seen.insert(entity_id) {
            return Err(HistoryError::Validation(format!(
                "duplicate history entity id {entity_id}"
            )));
        }
    }
    Ok(())
}

fn parse_timestamp_ms(value: &str) -> Result<u64, HistoryError> {
    let timestamp = DateTime::parse_from_rfc3339(value).map_err(|error| {
        HistoryError::Validation(format!("invalid RFC3339 timestamp {value}: {error}"))
    })?;
    u64::try_from(timestamp.timestamp_millis()).map_err(|_| {
        HistoryError::Validation(format!("timestamp {value} is before the Unix epoch"))
    })
}

fn timestamp_sort_key(value: &str) -> u64 {
    parse_timestamp_ms(value).unwrap_or(u64::MAX)
}

#[derive(Debug, Deserialize)]
struct RawHistoryState {
    #[serde(default)]
    entity_id: Option<String>,
    state: String,
    #[serde(default)]
    attributes: BTreeMap<String, JsonValue>,
    last_changed: String,
    last_updated: String,
}

type HistorySocket = WebSocket<MaybeTlsStream<TcpStream>>;

fn configure_socket_timeout(
    socket: &mut HistorySocket,
    timeout: Duration,
) -> Result<(), HistoryError> {
    let stream = match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => stream,
        MaybeTlsStream::Rustls(stream) => &mut stream.sock,
        _ => {
            return Err(HistoryError::Transport(
                "unsupported WebSocket stream backend".to_string(),
            ));
        }
    };
    stream
        .set_read_timeout(Some(timeout))
        .and_then(|()| stream.set_write_timeout(Some(timeout)))
        .map_err(|error| HistoryError::Transport(error.to_string()))
}

fn authenticate(socket: &mut HistorySocket, access_token: &str) -> Result<(), HistoryError> {
    let required = read_json(socket)?;
    if required.get("type").and_then(JsonValue::as_str) != Some("auth_required") {
        return Err(HistoryError::Protocol(
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
        Some("auth_invalid") => Err(HistoryError::Protocol(
            "Home Assistant rejected the access token".to_string(),
        )),
        _ => Err(HistoryError::Protocol(
            "server returned an unexpected authentication response".to_string(),
        )),
    }
}

fn request_history(
    socket: &mut HistorySocket,
    id: u64,
    config: &HistoryCollectorConfig,
    entity_ids: &[String],
) -> Result<BTreeMap<String, Vec<RawHistoryState>>, HistoryError> {
    send_json(
        socket,
        &json!({
            "id": id,
            "type": HISTORY_COMMAND,
            "start_time": config.start_time,
            "end_time": config.end_time,
            "entity_ids": entity_ids,
            "include_start_time_state": true,
            "significant_changes_only": false,
            "minimal_response": false,
            "no_attributes": false
        }),
    )?;
    for _ in 0..MAX_UNMATCHED_MESSAGES {
        let response = read_json(socket)?;
        if response.get("id").and_then(JsonValue::as_u64) != Some(id) {
            continue;
        }
        if response.get("type").and_then(JsonValue::as_str) != Some("result") {
            return Err(HistoryError::Protocol(
                "history command returned a non-result response".to_string(),
            ));
        }
        if response.get("success").and_then(JsonValue::as_bool) != Some(true) {
            let code = response
                .pointer("/error/code")
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown_error");
            return Err(HistoryError::Protocol(format!(
                "history command failed with {code}"
            )));
        }
        let result = response.get("result").cloned().ok_or_else(|| {
            HistoryError::Protocol("history command returned no result".to_string())
        })?;
        return serde_json::from_value(result)
            .map_err(|error| HistoryError::Decode(error.to_string()));
    }
    Err(HistoryError::Protocol(
        "history command did not return a matching response".to_string(),
    ))
}

fn send_json(socket: &mut HistorySocket, value: &JsonValue) -> Result<(), HistoryError> {
    socket
        .send(Message::Text(value.to_string().into()))
        .map_err(|error| HistoryError::Transport(error.to_string()))
}

fn read_json(socket: &mut HistorySocket) -> Result<JsonValue, HistoryError> {
    loop {
        let message = socket
            .read()
            .map_err(|error| HistoryError::Transport(error.to_string()))?;
        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_ref())
                    .map_err(|error| HistoryError::Decode(error.to_string()));
            }
            Message::Close(_) => {
                return Err(HistoryError::Protocol(
                    "connection closed before history collection completed".to_string(),
                ));
            }
            Message::Binary(_) => {
                return Err(HistoryError::Protocol(
                    "server returned an unexpected binary message".to_string(),
                ));
            }
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}

fn redact_token(mut message: String, access_token: &str) -> String {
    if !access_token.is_empty() {
        message = message.replace(access_token, "[redacted]");
    }
    message
}
