//! Live Home Assistant Lovelace collection and native dashboard migration.

#![forbid(unsafe_code)]

use coding_adventures_sha256::sha256_hex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as JsonValue};
pub use smart_home_dashboard_core::{
    HomeAssistantDashboardResource, NativeDashboard, NativeDashboardCard, NativeDashboardCardKind,
    NativeDashboardManifest, NativeDashboardView,
};
use smart_home_home_assistant_migration::{HomeAssistantExport, EXPORT_SCHEMA_VERSION};
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File};
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message, WebSocket};

pub const DASHBOARD_MIGRATION_SCHEMA_VERSION: u32 =
    smart_home_dashboard_core::DASHBOARD_MANIFEST_SCHEMA_VERSION;
const MAX_UNMATCHED_MESSAGES: usize = 64;

type HomeAssistantSocket = WebSocket<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardCollectorConfig {
    pub websocket_url: String,
    pub access_token: String,
    pub source_instance_id: String,
    pub collected_at_ms: u64,
    pub io_timeout: Duration,
}

impl DashboardCollectorConfig {
    pub fn validate(&self) -> Result<(), DashboardMigrationError> {
        if !self.websocket_url.starts_with("ws://") && !self.websocket_url.starts_with("wss://") {
            return Err(DashboardMigrationError::Config(
                "Home Assistant WebSocket URL must use ws:// or wss://".to_string(),
            ));
        }
        if self.access_token.trim().is_empty() {
            return Err(DashboardMigrationError::Config(
                "Home Assistant access token is empty".to_string(),
            ));
        }
        if self.source_instance_id.trim().is_empty() {
            return Err(DashboardMigrationError::Config(
                "source instance id is empty".to_string(),
            ));
        }
        if self.io_timeout.is_zero() {
            return Err(DashboardMigrationError::Config(
                "I/O timeout must be greater than zero".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DashboardDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardDiagnostic {
    pub severity: DashboardDiagnosticSeverity,
    pub source_kind: String,
    pub source_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardMigrationSummary {
    pub dashboards_discovered: usize,
    pub dashboards_collected: usize,
    pub views: usize,
    pub cards_discovered: usize,
    pub cards_migrated: usize,
    pub entity_references: usize,
    pub resources: usize,
    pub warnings: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardMigrationPlan {
    pub source_instance_id: String,
    pub source_fingerprint: String,
    pub collected_at_ms: u64,
    pub manifest: NativeDashboardManifest,
    pub diagnostics: Vec<DashboardDiagnostic>,
    pub summary: DashboardMigrationSummary,
}

impl DashboardMigrationPlan {
    pub fn is_blocked(&self) -> bool {
        self.summary.errors > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardMigrationReceipt {
    pub migration_id: String,
    pub source_instance_id: String,
    pub source_fingerprint: String,
    pub applied_at_ms: u64,
    pub dashboards: usize,
    pub views: usize,
    pub cards: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DashboardMigrationArtifact {
    pub schema_version: u32,
    pub dry_run: bool,
    pub plan: DashboardMigrationPlan,
    #[serde(default)]
    pub receipt: Option<DashboardMigrationReceipt>,
}

#[derive(Debug)]
pub enum DashboardMigrationError {
    Config(String),
    Validation(String),
    Transport(String),
    Protocol(String),
    Decode(String),
    Encode(String),
    Blocked(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
    Usage(String),
}

impl fmt::Display for DashboardMigrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => {
                write!(formatter, "invalid collector configuration: {message}")
            }
            Self::Validation(message) => write!(formatter, "invalid dashboard input: {message}"),
            Self::Transport(message) => {
                write!(formatter, "Home Assistant transport failed: {message}")
            }
            Self::Protocol(message) => {
                write!(formatter, "Home Assistant protocol failed: {message}")
            }
            Self::Decode(message) => write!(formatter, "dashboard decode failed: {message}"),
            Self::Encode(message) => write!(formatter, "dashboard encode failed: {message}"),
            Self::Blocked(message) => write!(formatter, "dashboard migration blocked: {message}"),
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

impl std::error::Error for DashboardMigrationError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SourceDashboard {
    #[serde(default)]
    id: Option<String>,
    url_path: String,
    title: String,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    require_admin: bool,
    #[serde(default = "default_true")]
    show_in_sidebar: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CollectedDashboard {
    metadata: SourceDashboard,
    config: JsonValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CollectedSource {
    dashboards: Vec<CollectedDashboard>,
    resources: Vec<HomeAssistantDashboardResource>,
}

pub fn migrate_live_dashboards(
    topology: &HomeAssistantExport,
    config: &DashboardCollectorConfig,
    dry_run: bool,
) -> Result<DashboardMigrationArtifact, DashboardMigrationError> {
    config.validate()?;
    validate_topology(topology, config)?;
    let (source, mut diagnostics, discovered) = collect_source(config)?;
    let plan = plan_collected(topology, config, source, &mut diagnostics, discovered)?;
    if !dry_run && plan.is_blocked() {
        return Err(DashboardMigrationError::Blocked(format!(
            "{} collection or validation errors require review",
            plan.summary.errors
        )));
    }
    let receipt = (!dry_run).then(|| DashboardMigrationReceipt {
        migration_id: format!("ha-dashboard:{}", &plan.source_fingerprint[..16]),
        source_instance_id: plan.source_instance_id.clone(),
        source_fingerprint: plan.source_fingerprint.clone(),
        applied_at_ms: config.collected_at_ms,
        dashboards: plan.summary.dashboards_collected,
        views: plan.summary.views,
        cards: plan.summary.cards_migrated,
    });
    Ok(DashboardMigrationArtifact {
        schema_version: DASHBOARD_MIGRATION_SCHEMA_VERSION,
        dry_run,
        plan,
        receipt,
    })
}

fn validate_topology(
    topology: &HomeAssistantExport,
    config: &DashboardCollectorConfig,
) -> Result<(), DashboardMigrationError> {
    if topology.schema_version != EXPORT_SCHEMA_VERSION {
        return Err(DashboardMigrationError::Validation(format!(
            "unsupported topology schema {}, expected {EXPORT_SCHEMA_VERSION}",
            topology.schema_version
        )));
    }
    if topology.source_instance_id != config.source_instance_id {
        return Err(DashboardMigrationError::Validation(format!(
            "source instance `{}` does not match topology `{}`",
            config.source_instance_id, topology.source_instance_id
        )));
    }
    Ok(())
}

fn collect_source(
    config: &DashboardCollectorConfig,
) -> Result<(CollectedSource, Vec<DashboardDiagnostic>, usize), DashboardMigrationError> {
    let (mut socket, _) = connect(config.websocket_url.as_str()).map_err(|error| {
        DashboardMigrationError::Transport(redact(error.to_string(), &config.access_token))
    })?;
    configure_socket_timeout(&mut socket, config.io_timeout)?;
    authenticate(&mut socket, &config.access_token)?;

    let raw_dashboards = request(&mut socket, 1, json!({"type": "lovelace/dashboards/list"}))?;
    let mut dashboards: Vec<SourceDashboard> = serde_json::from_value(raw_dashboards)
        .map_err(|error| DashboardMigrationError::Decode(error.to_string()))?;
    dashboards.sort_by(|left, right| left.url_path.cmp(&right.url_path));
    let discovered = dashboards.len();

    let raw_resources = request(&mut socket, 2, json!({"type": "lovelace/resources/list"}))?;
    let mut resources = normalize_resources(&raw_resources)?;
    resources.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));

    let mut collected = Vec::new();
    let mut diagnostics = Vec::new();
    for (index, metadata) in dashboards.into_iter().enumerate() {
        let id = u64::try_from(index + 3).map_err(|_| {
            DashboardMigrationError::Validation("too many dashboards to request".to_string())
        })?;
        match request(
            &mut socket,
            id,
            json!({"type": "lovelace/config", "url_path": metadata.url_path}),
        ) {
            Ok(configuration) => collected.push(CollectedDashboard {
                metadata,
                config: configuration,
            }),
            Err(error) => diagnostics.push(diagnostic(
                DashboardDiagnosticSeverity::Error,
                "dashboard",
                &metadata.url_path,
                "dashboard_config_unavailable",
                error.to_string(),
            )),
        }
    }
    Ok((
        CollectedSource {
            dashboards: collected,
            resources,
        },
        diagnostics,
        discovered,
    ))
}

fn normalize_resources(
    raw: &JsonValue,
) -> Result<Vec<HomeAssistantDashboardResource>, DashboardMigrationError> {
    let items = raw.as_array().ok_or_else(|| {
        DashboardMigrationError::Decode("Lovelace resources result is not an array".to_string())
    })?;
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let object = item.as_object().ok_or_else(|| {
                DashboardMigrationError::Decode("Lovelace resource is not an object".to_string())
            })?;
            let url = object
                .get("url")
                .and_then(JsonValue::as_str)
                .ok_or_else(|| {
                    DashboardMigrationError::Decode("resource has no URL".to_string())
                })?;
            let resource_type = object
                .get("res_type")
                .or_else(|| object.get("type"))
                .and_then(JsonValue::as_str)
                .unwrap_or("module");
            let resource_id = object
                .get("id")
                .and_then(JsonValue::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("resource-{}", index + 1));
            Ok(HomeAssistantDashboardResource {
                resource_id,
                url: url.to_string(),
                resource_type: resource_type.to_string(),
            })
        })
        .collect()
}

fn plan_collected(
    topology: &HomeAssistantExport,
    config: &DashboardCollectorConfig,
    source: CollectedSource,
    diagnostics: &mut Vec<DashboardDiagnostic>,
    discovered: usize,
) -> Result<DashboardMigrationPlan, DashboardMigrationError> {
    let mut enabled_entity_ids = topology
        .entities
        .iter()
        .filter(|entity| entity.disabled_by.is_none())
        .map(|entity| entity.entity_id.as_str())
        .collect::<Vec<_>>();
    enabled_entity_ids.sort_unstable();
    let fingerprint_bytes = serde_json::to_vec(&(&source, &enabled_entity_ids))
        .map_err(|error| DashboardMigrationError::Encode(error.to_string()))?;
    let source_fingerprint = sha256_hex(&fingerprint_bytes);
    let known_entities = enabled_entity_ids.into_iter().collect::<BTreeSet<_>>();
    let mut counters = CompileCounters::default();
    let mut dashboards = Vec::new();
    for dashboard in &source.dashboards {
        dashboards.push(compile_dashboard(
            dashboard,
            &known_entities,
            diagnostics,
            &mut counters,
        ));
    }
    for resource in &source.resources {
        diagnostics.push(diagnostic(
            DashboardDiagnosticSeverity::Warning,
            "resource",
            &resource.resource_id,
            "external_resource_requires_manual_review",
            format!(
                "Home Assistant resource `{}` ({}) is preserved but not executed",
                resource.url, resource.resource_type
            ),
        ));
    }
    diagnostics.sort_by(|left, right| {
        left.source_kind
            .cmp(&right.source_kind)
            .then_with(|| left.source_id.cmp(&right.source_id))
            .then_with(|| left.code.cmp(&right.code))
    });
    let warnings = diagnostics
        .iter()
        .filter(|item| item.severity == DashboardDiagnosticSeverity::Warning)
        .count();
    let errors = diagnostics
        .iter()
        .filter(|item| item.severity == DashboardDiagnosticSeverity::Error)
        .count();
    let summary = DashboardMigrationSummary {
        dashboards_discovered: discovered,
        dashboards_collected: dashboards.len(),
        views: counters.views,
        cards_discovered: counters.cards_discovered,
        cards_migrated: counters.cards_migrated,
        entity_references: counters.entity_references,
        resources: source.resources.len(),
        warnings,
        errors,
    };
    Ok(DashboardMigrationPlan {
        source_instance_id: config.source_instance_id.clone(),
        source_fingerprint,
        collected_at_ms: config.collected_at_ms,
        manifest: NativeDashboardManifest {
            schema_version: DASHBOARD_MIGRATION_SCHEMA_VERSION,
            source_instance_id: config.source_instance_id.clone(),
            generated_at_ms: config.collected_at_ms,
            dashboards,
            source_resources: source.resources,
        },
        diagnostics: diagnostics.clone(),
        summary,
    })
}

#[derive(Default)]
struct CompileCounters {
    views: usize,
    cards_discovered: usize,
    cards_migrated: usize,
    entity_references: usize,
}

fn compile_dashboard(
    source: &CollectedDashboard,
    known_entities: &BTreeSet<&str>,
    diagnostics: &mut Vec<DashboardDiagnostic>,
    counters: &mut CompileCounters,
) -> NativeDashboard {
    let mut views = Vec::new();
    let raw_views = source
        .config
        .get("views")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    if source.config.get("strategy").is_some() {
        diagnostics.push(diagnostic(
            DashboardDiagnosticSeverity::Warning,
            "dashboard",
            &source.metadata.url_path,
            "generated_strategy_requires_manual_review",
            "generated dashboard strategies cannot be reproduced deterministically".to_string(),
        ));
    }
    for (view_index, raw_view) in raw_views.iter().enumerate() {
        let view_source_id = format!("{}:view:{}", source.metadata.url_path, view_index + 1);
        let Some(object) = raw_view.as_object() else {
            diagnostics.push(diagnostic(
                DashboardDiagnosticSeverity::Warning,
                "view",
                &view_source_id,
                "invalid_view",
                "view is not an object".to_string(),
            ));
            continue;
        };
        let title = object
            .get("title")
            .and_then(JsonValue::as_str)
            .unwrap_or("Untitled view")
            .to_string();
        let mut raw_cards = object
            .get("cards")
            .and_then(JsonValue::as_array)
            .cloned()
            .unwrap_or_default();
        if let Some(sections) = object.get("sections").and_then(JsonValue::as_array) {
            for section in sections {
                if let Some(cards) = section.get("cards").and_then(JsonValue::as_array) {
                    raw_cards.extend(cards.iter().cloned());
                }
            }
        }
        let mut cards = Vec::new();
        for raw_card in &raw_cards {
            compile_card(
                raw_card,
                &view_source_id,
                known_entities,
                diagnostics,
                counters,
                &mut cards,
            );
        }
        counters.views += 1;
        views.push(NativeDashboardView {
            view_id: format!(
                "dashboard:{}:view:{}",
                source.metadata.url_path,
                view_index + 1
            ),
            title,
            path: string_field(object, "path"),
            icon: string_field(object, "icon"),
            cards,
        });
    }
    NativeDashboard {
        dashboard_id: source
            .metadata
            .id
            .clone()
            .unwrap_or_else(|| source.metadata.url_path.clone()),
        url_path: source.metadata.url_path.clone(),
        title: source.metadata.title.clone(),
        icon: source.metadata.icon.clone(),
        require_admin: source.metadata.require_admin,
        show_in_sidebar: source.metadata.show_in_sidebar,
        views,
    }
}

fn compile_card(
    raw: &JsonValue,
    view_source_id: &str,
    known_entities: &BTreeSet<&str>,
    diagnostics: &mut Vec<DashboardDiagnostic>,
    counters: &mut CompileCounters,
    output: &mut Vec<NativeDashboardCard>,
) {
    counters.cards_discovered += 1;
    let source_id = format!("{view_source_id}:card:{}", counters.cards_discovered);
    let Some(object) = raw.as_object() else {
        diagnostics.push(diagnostic(
            DashboardDiagnosticSeverity::Warning,
            "card",
            &source_id,
            "invalid_card",
            "card is not an object".to_string(),
        ));
        return;
    };
    let card_type = object
        .get("type")
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    if matches!(card_type, "vertical-stack" | "horizontal-stack" | "grid") {
        if let Some(cards) = object.get("cards").and_then(JsonValue::as_array) {
            for card in cards {
                compile_card(
                    card,
                    view_source_id,
                    known_entities,
                    diagnostics,
                    counters,
                    output,
                );
            }
        } else {
            diagnostics.push(diagnostic(
                DashboardDiagnosticSeverity::Warning,
                "card",
                &source_id,
                "empty_layout_container",
                format!("{card_type} card has no cards array"),
            ));
        }
        return;
    }
    if has_unsupported_action(object) {
        diagnostics.push(diagnostic(
            DashboardDiagnosticSeverity::Warning,
            "card",
            &source_id,
            "unsupported_card_action",
            format!("{card_type} card contains an action that requires manual review"),
        ));
        return;
    }
    let (kind, raw_entities) = match card_type {
        "entity" | "light" | "thermostat" | "sensor" | "tile" | "button" => (
            NativeDashboardCardKind::EntityControl,
            object.get("entity").into_iter().collect::<Vec<_>>(),
        ),
        "entities" | "glance" => (
            NativeDashboardCardKind::EntityList,
            object
                .get("entities")
                .and_then(JsonValue::as_array)
                .map(|items| items.iter().collect())
                .unwrap_or_default(),
        ),
        "history-graph" => (
            NativeDashboardCardKind::History,
            object
                .get("entities")
                .and_then(JsonValue::as_array)
                .map(|items| items.iter().collect())
                .unwrap_or_default(),
        ),
        _ => {
            diagnostics.push(diagnostic(
                DashboardDiagnosticSeverity::Warning,
                "card",
                &source_id,
                "unsupported_card_type",
                format!("Lovelace card type `{card_type}` is not in the native subset"),
            ));
            return;
        }
    };
    let mut entities = Vec::new();
    let mut seen_entities = BTreeSet::new();
    for item in raw_entities {
        let source_entity = match item {
            JsonValue::String(value) => Some(value.as_str()),
            JsonValue::Object(row) if row.get("type").is_none() && !has_unsupported_action(row) => {
                row.get("entity").and_then(JsonValue::as_str)
            }
            _ => None,
        };
        let Some(source_entity) = source_entity else {
            diagnostics.push(diagnostic(
                DashboardDiagnosticSeverity::Warning,
                "card",
                &source_id,
                "unsupported_entity_row",
                "card contains an entity row without a scalar entity id".to_string(),
            ));
            continue;
        };
        if !known_entities.contains(source_entity) {
            diagnostics.push(diagnostic(
                DashboardDiagnosticSeverity::Warning,
                "card",
                &source_id,
                "unknown_entity_reference",
                format!("entity `{source_entity}` is not enabled in the reviewed topology"),
            ));
            continue;
        }
        let migrated = format!("ha:{source_entity}");
        if seen_entities.insert(migrated.clone()) {
            entities.push(migrated);
        }
    }
    if entities.is_empty() {
        diagnostics.push(diagnostic(
            DashboardDiagnosticSeverity::Warning,
            "card",
            &source_id,
            "card_has_no_migratable_entities",
            format!("{card_type} card has no migratable entity references"),
        ));
        return;
    }
    counters.cards_migrated += 1;
    counters.entity_references += entities.len();
    output.push(NativeDashboardCard {
        card_id: format!("dashboard-card:{}", counters.cards_migrated),
        kind,
        source_type: card_type.to_string(),
        title: string_field(object, "title").or_else(|| string_field(object, "name")),
        entity_ids: entities,
    });
}

fn has_unsupported_action(object: &serde_json::Map<String, JsonValue>) -> bool {
    ["tap_action", "hold_action", "double_tap_action"]
        .iter()
        .filter_map(|name| object.get(*name))
        .any(|action| {
            action
                .get("action")
                .and_then(JsonValue::as_str)
                .is_some_and(|kind| !matches!(kind, "more-info" | "none"))
        })
}

fn string_field(object: &serde_json::Map<String, JsonValue>, name: &str) -> Option<String> {
    object
        .get(name)
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

fn diagnostic(
    severity: DashboardDiagnosticSeverity,
    source_kind: &str,
    source_id: &str,
    code: &str,
    message: String,
) -> DashboardDiagnostic {
    DashboardDiagnostic {
        severity,
        source_kind: source_kind.to_string(),
        source_id: source_id.to_string(),
        code: code.to_string(),
        message,
    }
}

fn configure_socket_timeout(
    socket: &mut HomeAssistantSocket,
    timeout: Duration,
) -> Result<(), DashboardMigrationError> {
    match socket.get_mut() {
        MaybeTlsStream::Plain(stream) => {
            stream
                .set_read_timeout(Some(timeout))
                .and_then(|()| stream.set_write_timeout(Some(timeout)))
                .map_err(|error| DashboardMigrationError::Transport(error.to_string()))?;
        }
        MaybeTlsStream::Rustls(stream) => {
            stream
                .get_mut()
                .set_read_timeout(Some(timeout))
                .and_then(|()| stream.get_mut().set_write_timeout(Some(timeout)))
                .map_err(|error| DashboardMigrationError::Transport(error.to_string()))?;
        }
        _ => {}
    }
    Ok(())
}

fn authenticate(
    socket: &mut HomeAssistantSocket,
    access_token: &str,
) -> Result<(), DashboardMigrationError> {
    let required = read_socket_json(socket)?;
    if required.get("type").and_then(JsonValue::as_str) != Some("auth_required") {
        return Err(DashboardMigrationError::Protocol(
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
        Some("auth_invalid") => Err(DashboardMigrationError::Protocol(
            "Home Assistant rejected authentication".to_string(),
        )),
        _ => Err(DashboardMigrationError::Protocol(
            "unexpected authentication response".to_string(),
        )),
    }
}

fn request(
    socket: &mut HomeAssistantSocket,
    id: u64,
    mut command: JsonValue,
) -> Result<JsonValue, DashboardMigrationError> {
    command
        .as_object_mut()
        .ok_or_else(|| DashboardMigrationError::Protocol("command is not an object".to_string()))?
        .insert("id".to_string(), JsonValue::from(id));
    send_socket_json(socket, &command)?;
    for _ in 0..MAX_UNMATCHED_MESSAGES {
        let response = read_socket_json(socket)?;
        if response.get("id").and_then(JsonValue::as_u64) != Some(id) {
            continue;
        }
        if response.get("success").and_then(JsonValue::as_bool) != Some(true) {
            let code = response
                .pointer("/error/code")
                .and_then(JsonValue::as_str)
                .unwrap_or("request_failed");
            let message = response
                .pointer("/error/message")
                .and_then(JsonValue::as_str)
                .unwrap_or("request failed");
            return Err(DashboardMigrationError::Protocol(format!(
                "Home Assistant returned {code}: {message}"
            )));
        }
        return response.get("result").cloned().ok_or_else(|| {
            DashboardMigrationError::Protocol("result response has no result".to_string())
        });
    }
    Err(DashboardMigrationError::Protocol(format!(
        "no matching result after {MAX_UNMATCHED_MESSAGES} messages"
    )))
}

fn send_socket_json(
    socket: &mut HomeAssistantSocket,
    value: &JsonValue,
) -> Result<(), DashboardMigrationError> {
    socket
        .send(Message::Text(value.to_string().into()))
        .map_err(|error| DashboardMigrationError::Transport(error.to_string()))
}

fn read_socket_json(
    socket: &mut HomeAssistantSocket,
) -> Result<JsonValue, DashboardMigrationError> {
    loop {
        let message = socket
            .read()
            .map_err(|error| DashboardMigrationError::Transport(error.to_string()))?;
        match message {
            Message::Text(text) => {
                return serde_json::from_str(&text)
                    .map_err(|error| DashboardMigrationError::Protocol(error.to_string()));
            }
            Message::Binary(bytes) => {
                return serde_json::from_slice(&bytes)
                    .map_err(|error| DashboardMigrationError::Protocol(error.to_string()));
            }
            Message::Ping(payload) => socket
                .send(Message::Pong(payload))
                .map_err(|error| DashboardMigrationError::Transport(error.to_string()))?,
            Message::Close(_) => {
                return Err(DashboardMigrationError::Protocol(
                    "Home Assistant closed the WebSocket".to_string(),
                ));
            }
            _ => {}
        }
    }
}

fn redact(message: String, secret: &str) -> String {
    if secret.is_empty() {
        message
    } else {
        message.replace(secret, "[REDACTED]")
    }
}

pub fn write_artifact_atomically(
    path: &Path,
    artifact: &DashboardMigrationArtifact,
) -> Result<(), DashboardMigrationError> {
    let bytes = serde_json::to_vec_pretty(artifact)
        .map_err(|error| DashboardMigrationError::Encode(error.to_string()))?;
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|error| DashboardMigrationError::Io {
        operation: "create output directory",
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("dashboard.json");
    let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let result = (|| {
        let mut file = File::create(&temporary).map_err(|error| DashboardMigrationError::Io {
            operation: "create temporary output",
            path: temporary.clone(),
            message: error.to_string(),
        })?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| DashboardMigrationError::Io {
                operation: "write temporary output",
                path: temporary.clone(),
                message: error.to_string(),
            })?;
        fs::rename(&temporary, path).map_err(|error| DashboardMigrationError::Io {
            operation: "replace dashboard artifact",
            path: path.to_path_buf(),
            message: error.to_string(),
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_home_assistant_migration::HomeAssistantEntity;

    #[test]
    fn standard_cards_and_layouts_compile_to_native_manifest() {
        let topology = topology();
        let source = CollectedSource {
            dashboards: vec![CollectedDashboard {
                metadata: SourceDashboard {
                    id: Some("overview".to_string()),
                    url_path: "lovelace".to_string(),
                    title: "Overview".to_string(),
                    icon: None,
                    require_admin: false,
                    show_in_sidebar: true,
                },
                config: json!({"views": [{
                    "title": "Home",
                    "cards": [
                        {"type": "entities", "entities": ["light.kitchen", {"entity": "sensor.temperature"}]},
                        {"type": "grid", "cards": [
                            {"type": "light", "entity": "light.kitchen"},
                            {"type": "history-graph", "entities": ["sensor.temperature"]}
                        ]}
                    ]
                }]}),
            }],
            resources: Vec::new(),
        };
        let config = config();
        let mut diagnostics = Vec::new();
        let plan = plan_collected(&topology, &config, source, &mut diagnostics, 1)
            .expect("plan dashboard");

        assert_eq!(plan.summary.cards_discovered, 4);
        assert_eq!(plan.summary.cards_migrated, 3);
        assert_eq!(plan.summary.entity_references, 4);
        assert_eq!(plan.manifest.dashboards[0].views[0].cards.len(), 3);
        assert_eq!(
            plan.manifest.dashboards[0].views[0].cards[0].entity_ids,
            vec!["ha:light.kitchen", "ha:sensor.temperature"]
        );
        assert!(plan.diagnostics.is_empty());
    }

    #[test]
    fn unsupported_content_is_durable_and_never_approximated() {
        let topology = topology();
        let source = CollectedSource {
            dashboards: vec![CollectedDashboard {
                metadata: SourceDashboard {
                    id: None,
                    url_path: "energy".to_string(),
                    title: "Energy".to_string(),
                    icon: None,
                    require_admin: true,
                    show_in_sidebar: false,
                },
                config: json!({"views": [{"title": "Energy", "cards": [
                    {"type": "custom:power-flow-card", "entity": "sensor.temperature"},
                    {"type": "entity", "entity": "sensor.missing"},
                    {"type": "button", "entity": "light.kitchen", "tap_action": {"action": "call-service"}}
                ]}]}),
            }],
            resources: vec![HomeAssistantDashboardResource {
                resource_id: "power-flow".to_string(),
                url: "/local/power-flow.js".to_string(),
                resource_type: "module".to_string(),
            }],
        };
        let mut diagnostics = Vec::new();
        let plan = plan_collected(&topology, &config(), source, &mut diagnostics, 1)
            .expect("plan dashboard");

        assert_eq!(plan.summary.cards_discovered, 3);
        assert_eq!(plan.summary.cards_migrated, 0);
        assert_eq!(plan.summary.warnings, 5);
        assert!(plan
            .diagnostics
            .iter()
            .any(|item| item.code == "unsupported_card_type"));
        assert!(plan
            .diagnostics
            .iter()
            .any(|item| item.code == "external_resource_requires_manual_review"));
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

    fn config() -> DashboardCollectorConfig {
        DashboardCollectorConfig {
            websocket_url: "ws://127.0.0.1:8123/api/websocket".to_string(),
            access_token: "secret".to_string(),
            source_instance_id: "home-1".to_string(),
            collected_at_ms: 10,
            io_timeout: Duration::from_secs(1),
        }
    }
}
