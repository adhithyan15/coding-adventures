//! Home Assistant export planning and execution for the D23 smart-home runtime.

#![forbid(unsafe_code)]

use coding_adventures_sha256::sha256_hex;
use serde::{Deserialize, Serialize};
use smart_home_automation_runtime::{
    AutomationAction, AutomationCondition, AutomationDefinition, AutomationEventType,
    AutomationRuntimeSnapshot, AutomationTrigger, SmartHomeAutomationRuntime,
};
use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, CommandType,
    Device, DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata,
    ProtocolFamily, ProtocolIdentifier, Scene, SceneAction, SceneId, SceneScope, StateConfidence,
    StateSnapshot, StateSource, Value, ValueKind,
};
use smart_home_runtime::{RuntimeDurableSnapshot, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub const EXPORT_SCHEMA_VERSION: u32 = 1;
pub const ARTIFACT_SCHEMA_VERSION: u32 = 1;
const INTEGRATION_ID: &str = "home_assistant_migration";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantExport {
    pub schema_version: u32,
    pub source_instance_id: String,
    pub exported_at_ms: u64,
    #[serde(default)]
    pub areas: Vec<HomeAssistantArea>,
    #[serde(default)]
    pub devices: Vec<HomeAssistantDevice>,
    #[serde(default)]
    pub entities: Vec<HomeAssistantEntity>,
    #[serde(default)]
    pub states: Vec<HomeAssistantState>,
    #[serde(default)]
    pub scenes: Vec<HomeAssistantScene>,
    #[serde(default)]
    pub automations: Vec<HomeAssistantAutomation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeAssistantArea {
    pub area_id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeAssistantDevice {
    pub device_id: String,
    #[serde(default)]
    pub area_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub name_by_user: Option<String>,
    #[serde(default)]
    pub manufacturer: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub serial_number: Option<String>,
    #[serde(default)]
    pub sw_version: Option<String>,
    #[serde(default)]
    pub identifiers: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeAssistantEntity {
    pub entity_id: String,
    #[serde(default)]
    pub device_id: Option<String>,
    #[serde(default)]
    pub area_id: Option<String>,
    pub platform: String,
    pub unique_id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub original_name: Option<String>,
    #[serde(default)]
    pub disabled_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantState {
    pub entity_id: String,
    pub state: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantScene {
    pub scene_id: String,
    pub name: String,
    #[serde(default)]
    pub area_id: Option<String>,
    pub states: Vec<HomeAssistantTargetState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantTargetState {
    pub entity_id: String,
    pub state: String,
    #[serde(default)]
    pub attributes: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantAutomation {
    pub automation_id: String,
    pub alias: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub trigger: HomeAssistantAutomationTrigger,
    #[serde(default)]
    pub conditions: Vec<HomeAssistantStateCondition>,
    pub actions: Vec<HomeAssistantServiceAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HomeAssistantAutomationTrigger {
    Interval {
        every_ms: u64,
        #[serde(default)]
        offset_ms: u64,
    },
    State {
        entity_id: String,
        #[serde(default)]
        to: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HomeAssistantStateCondition {
    pub entity_id: String,
    pub state: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantServiceAction {
    pub service: String,
    #[serde(default)]
    pub target_entity_ids: Vec<String>,
    #[serde(default)]
    pub data: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationDiagnostic {
    pub severity: MigrationDiagnosticSeverity,
    pub code: String,
    pub source_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationPlanSummary {
    pub areas: usize,
    pub devices: usize,
    pub synthetic_devices: usize,
    pub entities: usize,
    pub disabled_entities: usize,
    pub generic_entities: usize,
    pub states: usize,
    pub scenes: usize,
    pub scene_actions: usize,
    pub automations: usize,
    pub automation_actions: usize,
    pub warnings: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantMigrationPlan {
    pub source_instance_id: String,
    pub source_fingerprint: String,
    pub exported_at_ms: u64,
    pub bridge: Bridge,
    pub devices: Vec<Device>,
    pub entities: Vec<Entity>,
    pub scenes: Vec<Scene>,
    pub automations: Vec<AutomationDefinition>,
    pub diagnostics: Vec<MigrationDiagnostic>,
    pub summary: MigrationPlanSummary,
}

impl HomeAssistantMigrationPlan {
    pub fn is_blocked(&self) -> bool {
        self.summary.errors > 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationApplyCounts {
    pub inserted_bridges: usize,
    pub replaced_bridges: usize,
    pub inserted_devices: usize,
    pub replaced_devices: usize,
    pub inserted_entities: usize,
    pub replaced_entities: usize,
    pub inserted_scenes: usize,
    pub replaced_scenes: usize,
    pub inserted_automations: usize,
    pub replaced_automations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReceipt {
    pub migration_id: String,
    pub source_instance_id: String,
    pub source_fingerprint: String,
    pub applied_at_ms: u64,
    pub counts: MigrationApplyCounts,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HomeAssistantMigrationArtifact {
    pub schema_version: u32,
    pub dry_run: bool,
    pub plan: HomeAssistantMigrationPlan,
    #[serde(default)]
    pub receipt: Option<MigrationReceipt>,
    #[serde(default)]
    pub runtime_snapshot: Option<RuntimeDurableSnapshot>,
    #[serde(default)]
    pub automation_snapshot: Option<AutomationRuntimeSnapshot>,
}

pub fn migrate_export_bytes(
    bytes: &[u8],
    dry_run: bool,
) -> Result<HomeAssistantMigrationArtifact, MigrationError> {
    let export: HomeAssistantExport =
        serde_json::from_slice(bytes).map_err(MigrationError::decode)?;
    let plan = plan_export(&export)?;
    if dry_run {
        return Ok(HomeAssistantMigrationArtifact {
            schema_version: ARTIFACT_SCHEMA_VERSION,
            dry_run: true,
            plan,
            receipt: None,
            runtime_snapshot: None,
            automation_snapshot: None,
        });
    }

    let mut runtime = SmartHomeRuntime::new();
    let mut automations = SmartHomeAutomationRuntime::new();
    let receipt = apply_plan(&plan, &mut runtime, &mut automations)?;
    Ok(HomeAssistantMigrationArtifact {
        schema_version: ARTIFACT_SCHEMA_VERSION,
        dry_run: false,
        plan,
        receipt: Some(receipt),
        runtime_snapshot: Some(runtime.durable_snapshot()),
        automation_snapshot: Some(automations.snapshot()),
    })
}

pub fn plan_export(
    export: &HomeAssistantExport,
) -> Result<HomeAssistantMigrationPlan, MigrationError> {
    validate_export_header(export)?;
    let canonical = serde_json::to_vec(export).map_err(MigrationError::encode)?;
    let fingerprint = stable_fingerprint(&canonical);
    let source_fragment = id_fragment(&export.source_instance_id);
    let bridge_id = BridgeId::trusted(format!("ha-import:{source_fragment}"));
    let mut bridge = Bridge::new(
        bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LocalProcess,
    );
    bridge.health = Health::Unknown;
    bridge.last_seen_at_ms = Some(export.exported_at_ms);
    bridge.identifiers.push(ProtocolIdentifier::new(
        ProtocolFamily::Vendor("home_assistant".to_string()),
        "instance_id",
        export.source_instance_id.clone(),
    )?);
    bridge.metadata = vec![
        Metadata::new("migration.source", "home_assistant"),
        Metadata::new("migration.source_fingerprint", &fingerprint),
    ];

    let mut diagnostics = Vec::new();
    let areas = unique_ids(
        export.areas.iter().map(|area| area.area_id.as_str()),
        "area",
        &mut diagnostics,
    );
    let device_ids = unique_ids(
        export
            .devices
            .iter()
            .map(|device| device.device_id.as_str()),
        "device",
        &mut diagnostics,
    );
    let _entity_ids = unique_ids(
        export
            .entities
            .iter()
            .map(|entity| entity.entity_id.as_str()),
        "entity",
        &mut diagnostics,
    );
    unique_ids(
        export.scenes.iter().map(|scene| scene.scene_id.as_str()),
        "scene",
        &mut diagnostics,
    );
    unique_ids(
        export
            .automations
            .iter()
            .map(|automation| automation.automation_id.as_str()),
        "automation",
        &mut diagnostics,
    );

    validate_area_references(export, &areas, &mut diagnostics);
    let states = export
        .states
        .iter()
        .map(|state| (state.entity_id.as_str(), state))
        .collect::<BTreeMap<_, _>>();
    let source_devices = export
        .devices
        .iter()
        .map(|device| (device.device_id.as_str(), device))
        .collect::<BTreeMap<_, _>>();
    let source_entities = export
        .entities
        .iter()
        .map(|entity| (entity.entity_id.as_str(), entity))
        .collect::<BTreeMap<_, _>>();

    let mut devices = export
        .devices
        .iter()
        .map(|device| {
            project_device(
                device,
                &bridge_id,
                export.exported_at_ms,
                &export.entities,
                &states,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut synthetic_devices = BTreeMap::new();
    let mut entities = Vec::new();
    let mut disabled_entities = 0;
    let mut generic_entities = 0;

    for source in &export.entities {
        if source.disabled_by.is_some() {
            disabled_entities += 1;
            diagnostics.push(diagnostic(
                MigrationDiagnosticSeverity::Info,
                "entity_disabled",
                &source.entity_id,
                "disabled Home Assistant entity was not imported",
            ));
            continue;
        }
        if source
            .device_id
            .as_ref()
            .is_some_and(|id| !device_ids.contains(id))
        {
            diagnostics.push(diagnostic(
                MigrationDiagnosticSeverity::Error,
                "unknown_device",
                &source.entity_id,
                format!(
                    "entity references missing Home Assistant device {}",
                    source.device_id.as_deref().unwrap_or_default()
                ),
            ));
            continue;
        }

        let device_id = source.device_id.as_ref().map_or_else(
            || {
                let id = DeviceId::trusted(format!(
                    "ha-device:entity:{}",
                    id_fragment(&source.entity_id)
                ));
                synthetic_devices.entry(id.clone()).or_insert_with(|| {
                    synthetic_device(
                        source,
                        id.clone(),
                        &bridge_id,
                        states.get(source.entity_id.as_str()).copied(),
                    )
                });
                id
            },
            |id| DeviceId::trusted(format!("ha-device:{id}")),
        );
        let state = states.get(source.entity_id.as_str()).copied();
        let (kind, capabilities, generic) = projection_for_entity(source, state);
        if generic {
            generic_entities += 1;
            diagnostics.push(diagnostic(
                MigrationDiagnosticSeverity::Warning,
                "generic_entity",
                &source.entity_id,
                format!(
                    "domain `{}` is preserved as observe-only generic state",
                    entity_domain(&source.entity_id)
                ),
            ));
        }
        entities.push(Entity {
            entity_id: migrated_entity_id(&source.entity_id),
            device_id,
            kind,
            name: entity_name(source),
            capabilities,
            state: state.map(|state| project_state(state, export.exported_at_ms)),
            metadata: entity_metadata(source),
        });
    }
    devices.extend(synthetic_devices.into_values());
    devices.sort_by(|left, right| left.device_id.cmp(&right.device_id));
    entities.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));

    let imported_entity_ids = entities
        .iter()
        .map(|entity| entity.entity_id.clone())
        .collect::<BTreeSet<_>>();
    let mut scenes = Vec::new();
    for source in &export.scenes {
        let mut actions = Vec::new();
        for target in &source.states {
            let entity_id = migrated_entity_id(&target.entity_id);
            if !imported_entity_ids.contains(&entity_id) {
                diagnostics.push(diagnostic(
                    MigrationDiagnosticSeverity::Error,
                    "unknown_scene_entity",
                    &source.scene_id,
                    format!(
                        "scene references entity {} that is not importable",
                        target.entity_id
                    ),
                ));
                continue;
            }
            actions.push(SceneAction {
                entity_id,
                desired_state: project_target_state(target),
            });
        }
        scenes.push(Scene {
            scene_id: migrated_scene_id(&source.scene_id),
            scope: if source.area_id.is_some() {
                SceneScope::Room
            } else {
                SceneScope::Home
            },
            native_ref: Some(ProtocolIdentifier::new(
                ProtocolFamily::Vendor("home_assistant".to_string()),
                "scene_id",
                source.scene_id.clone(),
            )?),
            actions,
            metadata: vec![
                Metadata::new("home_assistant.name", &source.name),
                Metadata::new(
                    "home_assistant.area_id",
                    source.area_id.as_deref().unwrap_or(""),
                ),
            ],
        });
    }
    scenes.sort_by(|left, right| left.scene_id.cmp(&right.scene_id));
    let imported_scene_ids = scenes
        .iter()
        .map(|scene| scene.scene_id.clone())
        .collect::<BTreeSet<_>>();

    let mut automations = Vec::new();
    for source in &export.automations {
        match project_automation(
            source,
            &source_entities,
            &imported_entity_ids,
            &imported_scene_ids,
        ) {
            Ok(automation) => automations.push(automation),
            Err(messages) => {
                for message in messages {
                    diagnostics.push(diagnostic(
                        MigrationDiagnosticSeverity::Error,
                        "unsupported_automation",
                        &source.automation_id,
                        message,
                    ));
                }
            }
        }
    }
    automations.sort_by(|left, right| left.automation_id.cmp(&right.automation_id));

    let summary = MigrationPlanSummary {
        areas: export.areas.len(),
        devices: devices.len(),
        synthetic_devices: synthetic_devices_count(&devices, &source_devices),
        entities: entities.len(),
        disabled_entities,
        generic_entities,
        states: entities
            .iter()
            .filter(|entity| entity.state.is_some())
            .count(),
        scenes: scenes.len(),
        scene_actions: scenes.iter().map(|scene| scene.actions.len()).sum(),
        automations: automations.len(),
        automation_actions: automations
            .iter()
            .map(|automation| automation.actions.len())
            .sum(),
        warnings: diagnostics
            .iter()
            .filter(|item| item.severity == MigrationDiagnosticSeverity::Warning)
            .count(),
        errors: diagnostics
            .iter()
            .filter(|item| item.severity == MigrationDiagnosticSeverity::Error)
            .count(),
    };
    Ok(HomeAssistantMigrationPlan {
        source_instance_id: export.source_instance_id.clone(),
        source_fingerprint: fingerprint,
        exported_at_ms: export.exported_at_ms,
        bridge,
        devices,
        entities,
        scenes,
        automations,
        diagnostics,
        summary,
    })
}

pub fn apply_plan(
    plan: &HomeAssistantMigrationPlan,
    runtime: &mut SmartHomeRuntime,
    automations: &mut SmartHomeAutomationRuntime,
) -> Result<MigrationReceipt, MigrationError> {
    if plan.is_blocked() {
        return Err(MigrationError::Blocked {
            errors: plan.summary.errors,
        });
    }

    let mut counts = MigrationApplyCounts::default();
    count_upsert(
        runtime.upsert_bridge(plan.bridge.clone())?.is_some(),
        &mut counts.inserted_bridges,
        &mut counts.replaced_bridges,
    );
    for device in &plan.devices {
        count_upsert(
            runtime.upsert_device(device.clone())?.is_some(),
            &mut counts.inserted_devices,
            &mut counts.replaced_devices,
        );
    }
    for entity in &plan.entities {
        count_upsert(
            runtime.upsert_entity(entity.clone())?.is_some(),
            &mut counts.inserted_entities,
            &mut counts.replaced_entities,
        );
    }
    for scene in &plan.scenes {
        count_upsert(
            runtime.upsert_scene(scene.clone())?.is_some(),
            &mut counts.inserted_scenes,
            &mut counts.replaced_scenes,
        );
    }
    for automation in &plan.automations {
        count_upsert(
            automations.upsert_definition(automation.clone())?.is_some(),
            &mut counts.inserted_automations,
            &mut counts.replaced_automations,
        );
    }

    Ok(MigrationReceipt {
        migration_id: format!("ha-migration:{}", plan.source_fingerprint),
        source_instance_id: plan.source_instance_id.clone(),
        source_fingerprint: plan.source_fingerprint.clone(),
        applied_at_ms: plan.exported_at_ms,
        counts,
    })
}

pub fn write_artifact_atomically(
    path: &Path,
    artifact: &HomeAssistantMigrationArtifact,
) -> Result<(), MigrationError> {
    let body = serde_json::to_vec_pretty(artifact).map_err(MigrationError::encode)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| MigrationError::Io {
            operation: "create output directory",
            path: parent.to_path_buf(),
            message: error.to_string(),
        })?;
    }
    let temporary = temporary_path(path);
    fs::write(&temporary, body).map_err(|error| MigrationError::Io {
        operation: "write temporary artifact",
        path: temporary.clone(),
        message: error.to_string(),
    })?;
    fs::rename(&temporary, path).map_err(|error| MigrationError::Io {
        operation: "replace artifact",
        path: path.to_path_buf(),
        message: error.to_string(),
    })
}

fn validate_export_header(export: &HomeAssistantExport) -> Result<(), MigrationError> {
    if export.schema_version != EXPORT_SCHEMA_VERSION {
        return Err(MigrationError::UnsupportedExportSchema(
            export.schema_version,
        ));
    }
    if export.source_instance_id.trim().is_empty() {
        return Err(MigrationError::Validation(
            "source_instance_id must not be empty".to_string(),
        ));
    }
    Ok(())
}

fn unique_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    kind: &str,
    diagnostics: &mut Vec<MigrationDiagnostic>,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            diagnostics.push(diagnostic(
                MigrationDiagnosticSeverity::Error,
                "empty_identifier",
                kind,
                format!("{kind} identifier must not be empty"),
            ));
        } else if !ids.insert(value.to_string()) {
            diagnostics.push(diagnostic(
                MigrationDiagnosticSeverity::Error,
                "duplicate_identifier",
                value,
                format!("duplicate Home Assistant {kind} identifier"),
            ));
        }
    }
    ids
}

fn validate_area_references(
    export: &HomeAssistantExport,
    areas: &BTreeSet<String>,
    diagnostics: &mut Vec<MigrationDiagnostic>,
) {
    for (source_id, area_id) in export
        .devices
        .iter()
        .filter_map(|device| {
            device
                .area_id
                .as_deref()
                .map(|area| (device.device_id.as_str(), area))
        })
        .chain(export.entities.iter().filter_map(|entity| {
            entity
                .area_id
                .as_deref()
                .map(|area| (entity.entity_id.as_str(), area))
        }))
        .chain(export.scenes.iter().filter_map(|scene| {
            scene
                .area_id
                .as_deref()
                .map(|area| (scene.scene_id.as_str(), area))
        }))
    {
        if !areas.contains(area_id) {
            diagnostics.push(diagnostic(
                MigrationDiagnosticSeverity::Error,
                "unknown_area",
                source_id,
                format!("references missing Home Assistant area {area_id}"),
            ));
        }
    }
}

fn project_device(
    source: &HomeAssistantDevice,
    bridge_id: &BridgeId,
    exported_at_ms: u64,
    entities: &[HomeAssistantEntity],
    states: &BTreeMap<&str, &HomeAssistantState>,
) -> Result<Device, MigrationError> {
    let online = entities
        .iter()
        .filter(|entity| entity.device_id.as_deref() == Some(source.device_id.as_str()))
        .filter_map(|entity| states.get(entity.entity_id.as_str()))
        .any(|state| !matches!(state.state.as_str(), "unavailable" | "unknown"));
    let mut metadata = vec![Metadata::new("home_assistant.device_id", &source.device_id)];
    for (domain, value) in &source.identifiers {
        metadata.push(Metadata::new(
            format!("home_assistant.identifier.{domain}"),
            value,
        ));
    }
    metadata.push(Metadata::new(
        "migration.exported_at_ms",
        exported_at_ms.to_string(),
    ));
    Ok(Device {
        device_id: DeviceId::trusted(format!("ha-device:{}", source.device_id)),
        bridge_id: bridge_id.clone(),
        manufacturer: source
            .manufacturer
            .clone()
            .unwrap_or_else(|| "Unknown".to_string()),
        model: source
            .model
            .clone()
            .unwrap_or_else(|| "Home Assistant device".to_string()),
        name: source
            .name_by_user
            .clone()
            .or_else(|| source.name.clone())
            .unwrap_or_else(|| source.device_id.clone()),
        serial: source.serial_number.clone(),
        firmware_version: source.sw_version.clone(),
        room_id: source
            .area_id
            .as_ref()
            .map(|area| format!("ha-area:{area}")),
        entity_ids: Vec::new(),
        identifiers: vec![ProtocolIdentifier::new(
            ProtocolFamily::Vendor("home_assistant".to_string()),
            "device_registry_id",
            source.device_id.clone(),
        )?],
        health: if online {
            Health::Online
        } else {
            Health::Unknown
        },
        metadata,
    })
}

fn synthetic_device(
    source: &HomeAssistantEntity,
    device_id: DeviceId,
    bridge_id: &BridgeId,
    state: Option<&HomeAssistantState>,
) -> Device {
    Device {
        device_id,
        bridge_id: bridge_id.clone(),
        manufacturer: "Home Assistant".to_string(),
        model: format!("{} entity", entity_domain(&source.entity_id)),
        name: entity_name(source),
        serial: None,
        firmware_version: None,
        room_id: source
            .area_id
            .as_ref()
            .map(|area| format!("ha-area:{area}")),
        entity_ids: Vec::new(),
        identifiers: vec![ProtocolIdentifier {
            family: ProtocolFamily::Vendor("home_assistant".to_string()),
            kind: "entity_id".to_string(),
            value: source.entity_id.clone(),
        }],
        health: state.map_or(Health::Unknown, |state| {
            if matches!(state.state.as_str(), "unavailable" | "unknown") {
                Health::Offline
            } else {
                Health::Online
            }
        }),
        metadata: vec![Metadata::new("migration.synthetic_device", "true")],
    }
}

fn projection_for_entity(
    source: &HomeAssistantEntity,
    state: Option<&HomeAssistantState>,
) -> (EntityKind, Vec<Capability>, bool) {
    let device_class = state
        .and_then(|state| state.attributes.get("device_class"))
        .and_then(serde_json::Value::as_str);
    match entity_domain(&source.entity_id) {
        "light" => (
            EntityKind::Light,
            vec![Capability::light_on_off(), Capability::light_brightness()],
            false,
        ),
        "switch" => (EntityKind::Switch, vec![Capability::light_on_off()], false),
        "lock" => (EntityKind::Lock, vec![Capability::lock_state()], false),
        "climate" => (
            EntityKind::Thermostat,
            vec![Capability::climate_setpoint()],
            false,
        ),
        "binary_sensor" => {
            let capability = match device_class {
                Some("motion" | "occupancy" | "presence") => Capability::sensor_occupancy(),
                Some("door" | "garage_door" | "opening" | "window") => Capability::sensor_contact(),
                _ => Capability::new(
                    CapabilityId::trusted("sensor.binary"),
                    CapabilityMode::Observe,
                    ValueKind::Boolean,
                ),
            };
            (EntityKind::Sensor, vec![capability], false)
        }
        "sensor" => {
            let mut capability = match device_class {
                Some("temperature") => Capability::sensor_temperature(),
                Some("humidity") => Capability::sensor_humidity(),
                Some("illuminance") => Capability::sensor_illuminance(),
                Some("battery") => Capability::sensor_battery(),
                _ => Capability::new(
                    CapabilityId::trusted("sensor.value"),
                    CapabilityMode::Observe,
                    ValueKind::Number,
                ),
            };
            if let Some(unit) = state
                .and_then(|state| state.attributes.get("unit_of_measurement"))
                .and_then(serde_json::Value::as_str)
            {
                capability.unit = Some(unit.to_string());
            }
            (EntityKind::Sensor, vec![capability], false)
        }
        "input_button" | "button" => (
            EntityKind::Input,
            vec![Capability::new(
                CapabilityId::trusted("input.button"),
                CapabilityMode::Command,
                ValueKind::Null,
            )],
            false,
        ),
        domain => (
            EntityKind::Unknown,
            vec![Capability::new(
                CapabilityId::trusted(format!("home_assistant.{domain}.state")),
                CapabilityMode::Observe,
                ValueKind::Text,
            )],
            true,
        ),
    }
}

fn project_state(source: &HomeAssistantState, exported_at_ms: u64) -> StateSnapshot {
    StateSnapshot {
        entity_id: migrated_entity_id(&source.entity_id),
        value: source_state_value(source),
        source: StateSource::Manual,
        observed_at_ms: exported_at_ms,
        received_at_ms: exported_at_ms,
        expires_at_ms: None,
        confidence: if matches!(source.state.as_str(), "unavailable" | "unknown") {
            StateConfidence::Unknown
        } else {
            StateConfidence::Confirmed
        },
    }
}

fn source_state_value(source: &HomeAssistantState) -> Value {
    let domain = entity_domain(&source.entity_id);
    if matches!(source.state.as_str(), "unavailable" | "unknown") {
        return Value::Null;
    }
    if matches!(domain, "light" | "switch" | "binary_sensor") {
        let on = matches!(
            source.state.as_str(),
            "on" | "open" | "detected" | "occupied" | "home"
        );
        if domain == "light" {
            if let Some(brightness) = brightness_percentage(&source.attributes) {
                return Value::Object(vec![
                    ("on".to_string(), Value::Bool(on)),
                    ("brightness".to_string(), Value::Percentage(brightness)),
                ]);
            }
        }
        return Value::Bool(on);
    }
    if domain == "lock" {
        return Value::Bool(source.state == "locked");
    }
    parse_scalar(&source.state)
}

fn project_target_state(source: &HomeAssistantTargetState) -> Value {
    if source.state == "on" {
        if let Some(brightness) = brightness_percentage(&source.attributes) {
            return Value::Object(vec![
                ("on".to_string(), Value::Bool(true)),
                ("brightness".to_string(), Value::Percentage(brightness)),
            ]);
        }
        return Value::Bool(true);
    }
    if source.state == "off" {
        return Value::Bool(false);
    }
    parse_scalar(&source.state)
}

fn project_automation(
    source: &HomeAssistantAutomation,
    source_entities: &BTreeMap<&str, &HomeAssistantEntity>,
    imported_entities: &BTreeSet<EntityId>,
    imported_scenes: &BTreeSet<SceneId>,
) -> Result<AutomationDefinition, Vec<String>> {
    let mut errors = Vec::new();
    let trigger = match &source.trigger {
        HomeAssistantAutomationTrigger::Interval {
            every_ms,
            offset_ms,
        } => {
            if *every_ms == 0 {
                errors.push("interval every_ms must be greater than zero".to_string());
            }
            AutomationTrigger::Schedule {
                every_ms: *every_ms,
                offset_ms: *offset_ms,
            }
        }
        HomeAssistantAutomationTrigger::State { entity_id, to } => {
            validate_automation_entity(entity_id, imported_entities, "trigger", &mut errors);
            let _ = to;
            AutomationTrigger::Event {
                event_type: AutomationEventType::Updated,
                entity_id: Some(migrated_entity_id(entity_id)),
            }
        }
    };

    let mut conditions = Vec::new();
    if let HomeAssistantAutomationTrigger::State {
        entity_id,
        to: Some(expected),
    } = &source.trigger
    {
        conditions.push(AutomationCondition::StateEquals {
            entity_id: migrated_entity_id(entity_id),
            expected: state_value_for_entity(
                source_entities.get(entity_id.as_str()).copied(),
                expected,
            ),
        });
    }
    for condition in &source.conditions {
        validate_automation_entity(
            &condition.entity_id,
            imported_entities,
            "condition",
            &mut errors,
        );
        conditions.push(AutomationCondition::StateEquals {
            entity_id: migrated_entity_id(&condition.entity_id),
            expected: state_value_for_entity(
                source_entities.get(condition.entity_id.as_str()).copied(),
                &condition.state,
            ),
        });
    }

    let mut actions = Vec::new();
    for action in &source.actions {
        project_service_action(
            action,
            source_entities,
            imported_entities,
            imported_scenes,
            &mut actions,
            &mut errors,
        );
    }
    if actions.is_empty() {
        errors.push("automation has no migratable actions".to_string());
    }
    if errors.is_empty() {
        Ok(AutomationDefinition {
            automation_id: format!("ha:{}", source.automation_id),
            enabled: source.enabled,
            trigger,
            conditions,
            actions,
        })
    } else {
        Err(errors)
    }
}

fn project_service_action(
    source: &HomeAssistantServiceAction,
    source_entities: &BTreeMap<&str, &HomeAssistantEntity>,
    imported_entities: &BTreeSet<EntityId>,
    imported_scenes: &BTreeSet<SceneId>,
    actions: &mut Vec<AutomationAction>,
    errors: &mut Vec<String>,
) {
    if source.service == "scene.turn_on" {
        for scene in &source.target_entity_ids {
            let scene_id = migrated_scene_id(scene);
            if imported_scenes.contains(&scene_id) {
                actions.push(AutomationAction::Scene {
                    scene_id,
                    timeout_ms: None,
                });
            } else {
                errors.push(format!("action references unknown scene `{scene}`"));
            }
        }
        return;
    }

    let projection = match source.service.as_str() {
        "light.turn_on" => {
            if let Some(brightness) = brightness_percentage(&source.data) {
                Some((CommandType::SetBrightness, Value::Percentage(brightness)))
            } else {
                Some((CommandType::TurnOn, Value::Null))
            }
        }
        "switch.turn_on" => Some((CommandType::TurnOn, Value::Null)),
        "light.turn_off" | "switch.turn_off" => Some((CommandType::TurnOff, Value::Null)),
        "lock.lock" => Some((CommandType::SetLock, Value::Bool(true))),
        "lock.unlock" => Some((CommandType::SetLock, Value::Bool(false))),
        "climate.set_temperature" => source
            .data
            .get("temperature")
            .and_then(json_number)
            .map(|value| (CommandType::SetThermostatSetpoint, Value::Number(value))),
        _ => None,
    };
    let Some((command_type, arguments)) = projection else {
        errors.push(format!(
            "service `{}` is outside the safe migration subset or lacks required data",
            source.service
        ));
        return;
    };
    if source.target_entity_ids.is_empty() {
        errors.push(format!(
            "service `{}` has no target entities",
            source.service
        ));
        return;
    }
    for entity in &source.target_entity_ids {
        let entity_id = migrated_entity_id(entity);
        if !imported_entities.contains(&entity_id) {
            errors.push(format!("action references unknown entity `{entity}`"));
            continue;
        }
        let domain = source_entities
            .get(entity.as_str())
            .map_or("", |source| entity_domain(&source.entity_id));
        let expected_domain = source
            .service
            .split_once('.')
            .map_or("", |(domain, _)| domain);
        if domain != expected_domain {
            errors.push(format!(
                "service `{}` cannot target `{entity}` with domain `{domain}`",
                source.service
            ));
            continue;
        }
        actions.push(AutomationAction::Command {
            entity_id,
            command_type,
            arguments: arguments.clone(),
            timeout_ms: None,
        });
    }
}

fn validate_automation_entity(
    source_id: &str,
    imported_entities: &BTreeSet<EntityId>,
    role: &str,
    errors: &mut Vec<String>,
) {
    if !imported_entities.contains(&migrated_entity_id(source_id)) {
        errors.push(format!("{role} references unknown entity `{source_id}`"));
    }
}

fn state_value_for_entity(source: Option<&HomeAssistantEntity>, state: &str) -> Value {
    let domain = source.map_or("", |entity| entity_domain(&entity.entity_id));
    match domain {
        "light" | "switch" | "binary_sensor" => Value::Bool(matches!(
            state,
            "on" | "open" | "detected" | "occupied" | "home"
        )),
        "lock" => Value::Bool(state == "locked"),
        _ => parse_scalar(state),
    }
}

fn parse_scalar(value: &str) -> Value {
    if let Ok(integer) = value.parse::<i64>() {
        Value::Integer(integer)
    } else if let Ok(number) = value.parse::<f64>() {
        Value::Number(number)
    } else {
        Value::Text(value.to_string())
    }
}

fn brightness_percentage(values: &BTreeMap<String, serde_json::Value>) -> Option<u8> {
    if let Some(value) = values.get("brightness_pct").and_then(json_number) {
        return Some(value.round().clamp(0.0, 100.0) as u8);
    }
    values
        .get("brightness")
        .and_then(json_number)
        .map(|value| ((value.clamp(0.0, 255.0) / 255.0) * 100.0).round() as u8)
}

fn json_number(value: &serde_json::Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn entity_metadata(source: &HomeAssistantEntity) -> Vec<Metadata> {
    vec![
        Metadata::new("home_assistant.entity_id", &source.entity_id),
        Metadata::new("home_assistant.platform", &source.platform),
        Metadata::new("home_assistant.unique_id", &source.unique_id),
        Metadata::new(
            "home_assistant.area_id",
            source.area_id.as_deref().unwrap_or(""),
        ),
    ]
}

fn entity_name(source: &HomeAssistantEntity) -> String {
    source
        .name
        .clone()
        .or_else(|| source.original_name.clone())
        .unwrap_or_else(|| {
            source
                .entity_id
                .split_once('.')
                .map_or(source.entity_id.as_str(), |(_, name)| name)
                .replace('_', " ")
        })
}

fn entity_domain(entity_id: &str) -> &str {
    entity_id.split_once('.').map_or("", |(domain, _)| domain)
}

fn migrated_entity_id(source_id: &str) -> EntityId {
    EntityId::trusted(format!("ha:{source_id}"))
}

fn migrated_scene_id(source_id: &str) -> SceneId {
    SceneId::trusted(format!("ha:{source_id}"))
}

fn diagnostic(
    severity: MigrationDiagnosticSeverity,
    code: impl Into<String>,
    source_id: impl Into<String>,
    message: impl Into<String>,
) -> MigrationDiagnostic {
    MigrationDiagnostic {
        severity,
        code: code.into(),
        source_id: source_id.into(),
        message: message.into(),
    }
}

fn synthetic_devices_count(
    devices: &[Device],
    source_devices: &BTreeMap<&str, &HomeAssistantDevice>,
) -> usize {
    devices
        .iter()
        .filter(|device| {
            !source_devices.contains_key(
                device
                    .device_id
                    .as_str()
                    .strip_prefix("ha-device:")
                    .unwrap_or_default(),
            )
        })
        .count()
}

fn count_upsert(replaced: bool, inserted: &mut usize, replaced_count: &mut usize) {
    if replaced {
        *replaced_count += 1;
    } else {
        *inserted += 1;
    }
}

fn stable_fingerprint(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

fn id_fragment(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            separator = false;
        } else if !separator && !output.is_empty() {
            output.push('-');
            separator = true;
        }
    }
    while output.ends_with('-') {
        output.pop();
    }
    if output.is_empty() {
        "source".to_string()
    } else {
        output
    }
}

fn temporary_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map_or_else(|| "migration-artifact".into(), |name| name.to_os_string());
    name.push(".tmp");
    path.with_file_name(name)
}

fn default_true() -> bool {
    true
}

#[derive(Debug)]
pub enum MigrationError {
    Usage(String),
    Validation(String),
    UnsupportedExportSchema(u32),
    Blocked {
        errors: usize,
    },
    Decode(String),
    Encode(String),
    Runtime(String),
    Automation(String),
    Io {
        operation: &'static str,
        path: PathBuf,
        message: String,
    },
}

impl MigrationError {
    fn decode(error: serde_json::Error) -> Self {
        Self::Decode(error.to_string())
    }

    fn encode(error: serde_json::Error) -> Self {
        Self::Encode(error.to_string())
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) | Self::Validation(message) => f.write_str(message),
            Self::UnsupportedExportSchema(version) => {
                write!(f, "unsupported Home Assistant export schema {version}")
            }
            Self::Blocked { errors } => {
                write!(f, "migration plan is blocked by {errors} error diagnostics")
            }
            Self::Decode(message) => write!(f, "could not decode Home Assistant export: {message}"),
            Self::Encode(message) => write!(f, "could not encode migration artifact: {message}"),
            Self::Runtime(message) => write!(f, "could not apply runtime migration: {message}"),
            Self::Automation(message) => {
                write!(f, "could not apply automation migration: {message}")
            }
            Self::Io {
                operation,
                path,
                message,
            } => write!(f, "could not {operation} {}: {message}", path.display()),
        }
    }
}

impl std::error::Error for MigrationError {}

impl From<smart_home_core::SmartHomeError> for MigrationError {
    fn from(error: smart_home_core::SmartHomeError) -> Self {
        Self::Validation(error.to_string())
    }
}

impl From<smart_home_runtime::RuntimeError> for MigrationError {
    fn from(error: smart_home_runtime::RuntimeError) -> Self {
        Self::Runtime(error.to_string())
    }
}

impl From<smart_home_automation_runtime::AutomationError> for MigrationError {
    fn from(error: smart_home_automation_runtime::AutomationError) -> Self {
        Self::Automation(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_export() -> HomeAssistantExport {
        HomeAssistantExport {
            schema_version: EXPORT_SCHEMA_VERSION,
            source_instance_id: "home-main".to_string(),
            exported_at_ms: 42_000,
            areas: vec![HomeAssistantArea {
                area_id: "kitchen".to_string(),
                name: "Kitchen".to_string(),
                aliases: Vec::new(),
            }],
            devices: vec![HomeAssistantDevice {
                device_id: "device-light".to_string(),
                area_id: Some("kitchen".to_string()),
                name: Some("Kitchen light".to_string()),
                name_by_user: None,
                manufacturer: Some("Signify".to_string()),
                model: Some("Hue bulb".to_string()),
                serial_number: Some("abc".to_string()),
                sw_version: Some("1.2.3".to_string()),
                identifiers: vec![("hue".to_string(), "lamp-1".to_string())],
            }],
            entities: vec![
                HomeAssistantEntity {
                    entity_id: "light.kitchen".to_string(),
                    device_id: Some("device-light".to_string()),
                    area_id: None,
                    platform: "hue".to_string(),
                    unique_id: "hue-light-1".to_string(),
                    name: Some("Kitchen".to_string()),
                    original_name: None,
                    disabled_by: None,
                },
                HomeAssistantEntity {
                    entity_id: "sensor.outdoor_air_quality".to_string(),
                    device_id: None,
                    area_id: Some("kitchen".to_string()),
                    platform: "rest".to_string(),
                    unique_id: "aqi-1".to_string(),
                    name: None,
                    original_name: Some("Outdoor air quality".to_string()),
                    disabled_by: None,
                },
            ],
            states: vec![
                HomeAssistantState {
                    entity_id: "light.kitchen".to_string(),
                    state: "on".to_string(),
                    attributes: BTreeMap::from([(
                        "brightness".to_string(),
                        serde_json::json!(128),
                    )]),
                },
                HomeAssistantState {
                    entity_id: "sensor.outdoor_air_quality".to_string(),
                    state: "31".to_string(),
                    attributes: BTreeMap::new(),
                },
            ],
            scenes: vec![HomeAssistantScene {
                scene_id: "scene.kitchen_night".to_string(),
                name: "Kitchen night".to_string(),
                area_id: Some("kitchen".to_string()),
                states: vec![HomeAssistantTargetState {
                    entity_id: "light.kitchen".to_string(),
                    state: "on".to_string(),
                    attributes: BTreeMap::from([(
                        "brightness_pct".to_string(),
                        serde_json::json!(20),
                    )]),
                }],
            }],
            automations: vec![HomeAssistantAutomation {
                automation_id: "night-kitchen".to_string(),
                alias: "Night kitchen".to_string(),
                enabled: true,
                trigger: HomeAssistantAutomationTrigger::Interval {
                    every_ms: 86_400_000,
                    offset_ms: 72_000_000,
                },
                conditions: vec![HomeAssistantStateCondition {
                    entity_id: "light.kitchen".to_string(),
                    state: "on".to_string(),
                }],
                actions: vec![HomeAssistantServiceAction {
                    service: "scene.turn_on".to_string(),
                    target_entity_ids: vec!["scene.kitchen_night".to_string()],
                    data: BTreeMap::new(),
                }],
            }],
        }
    }

    #[test]
    fn plans_topology_state_scene_and_automation_without_mutation() {
        let export = fixture_export();
        let artifact = migrate_export_bytes(&serde_json::to_vec(&export).unwrap(), true).unwrap();

        assert!(artifact.dry_run);
        assert!(artifact.receipt.is_none());
        assert!(artifact.runtime_snapshot.is_none());
        assert_eq!(artifact.plan.summary.areas, 1);
        assert_eq!(artifact.plan.summary.devices, 2);
        assert_eq!(artifact.plan.summary.synthetic_devices, 1);
        assert_eq!(artifact.plan.summary.entities, 2);
        assert_eq!(artifact.plan.summary.states, 2);
        assert_eq!(artifact.plan.summary.scenes, 1);
        assert_eq!(artifact.plan.summary.scene_actions, 1);
        assert_eq!(artifact.plan.summary.automations, 1);
        assert_eq!(artifact.plan.summary.errors, 0);
        assert_eq!(
            artifact.plan.entities[0].state.as_ref().unwrap().value,
            Value::Object(vec![
                ("on".to_string(), Value::Bool(true)),
                ("brightness".to_string(), Value::Percentage(50)),
            ])
        );
    }

    #[test]
    fn apply_is_idempotent_and_receipt_is_stable() {
        let plan = plan_export(&fixture_export()).unwrap();
        let mut runtime = SmartHomeRuntime::new();
        let mut automations = SmartHomeAutomationRuntime::new();

        let first = apply_plan(&plan, &mut runtime, &mut automations).unwrap();
        let second = apply_plan(&plan, &mut runtime, &mut automations).unwrap();

        assert_eq!(first.migration_id, second.migration_id);
        assert_eq!(first.counts.inserted_devices, 2);
        assert_eq!(first.counts.inserted_entities, 2);
        assert_eq!(first.counts.inserted_scenes, 1);
        assert_eq!(first.counts.inserted_automations, 1);
        assert_eq!(second.counts.replaced_devices, 2);
        assert_eq!(second.counts.replaced_entities, 2);
        assert_eq!(second.counts.replaced_scenes, 1);
        assert_eq!(second.counts.replaced_automations, 1);
        assert_eq!(runtime.registry().counts().devices, 2);
        assert_eq!(runtime.registry().counts().entities, 2);
        assert_eq!(automations.definitions().count(), 1);
    }

    #[test]
    fn unknown_references_block_apply() {
        let mut export = fixture_export();
        export.scenes[0].states[0].entity_id = "light.missing".to_string();
        export.automations[0].actions[0].target_entity_ids = vec!["scene.missing".to_string()];
        let plan = plan_export(&export).unwrap();

        assert!(plan.is_blocked());
        assert_eq!(plan.summary.errors, 3);
        let error = apply_plan(
            &plan,
            &mut SmartHomeRuntime::new(),
            &mut SmartHomeAutomationRuntime::new(),
        )
        .unwrap_err();
        assert!(matches!(error, MigrationError::Blocked { errors: 3 }));
    }

    #[test]
    fn unsupported_domain_is_preserved_as_observe_only() {
        let mut export = fixture_export();
        export.entities.push(HomeAssistantEntity {
            entity_id: "camera.driveway".to_string(),
            device_id: None,
            area_id: None,
            platform: "generic".to_string(),
            unique_id: "camera-1".to_string(),
            name: Some("Driveway".to_string()),
            original_name: None,
            disabled_by: None,
        });
        export.states.push(HomeAssistantState {
            entity_id: "camera.driveway".to_string(),
            state: "streaming".to_string(),
            attributes: BTreeMap::new(),
        });

        let plan = plan_export(&export).unwrap();
        let camera = plan
            .entities
            .iter()
            .find(|entity| entity.entity_id.as_str() == "ha:camera.driveway")
            .unwrap();
        assert_eq!(camera.kind, EntityKind::Unknown);
        assert_eq!(camera.capabilities[0].mode, CapabilityMode::Observe);
        assert_eq!(plan.summary.generic_entities, 1);
        assert_eq!(plan.summary.warnings, 1);
        assert!(!plan.is_blocked());
    }

    #[test]
    fn automation_service_domain_mismatch_blocks_apply() {
        let mut export = fixture_export();
        export.automations[0].actions = vec![HomeAssistantServiceAction {
            service: "lock.lock".to_string(),
            target_entity_ids: vec!["light.kitchen".to_string()],
            data: BTreeMap::new(),
        }];

        let plan = plan_export(&export).unwrap();

        assert!(plan.is_blocked());
        assert!(plan.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "unsupported_automation"
                && diagnostic.message.contains("cannot target")
        }));
    }

    #[test]
    fn applied_artifact_round_trips_through_atomic_file() {
        let export = fixture_export();
        let artifact = migrate_export_bytes(&serde_json::to_vec(&export).unwrap(), false).unwrap();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("ha-migration-{nanos}"));
        let path = root.join("artifact.json");

        write_artifact_atomically(&path, &artifact).unwrap();
        let restored: HomeAssistantMigrationArtifact =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();

        assert_eq!(restored, artifact);
        assert!(!path.with_file_name("artifact.json.tmp").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn fingerprint_is_deterministic_and_changes_with_source() {
        let first = plan_export(&fixture_export()).unwrap();
        let second = plan_export(&fixture_export()).unwrap();
        let mut changed = fixture_export();
        changed.states[0].state = "off".to_string();
        let changed = plan_export(&changed).unwrap();

        assert_eq!(first.source_fingerprint, second.source_fingerprint);
        assert_ne!(first.source_fingerprint, changed.source_fingerprint);
    }
}
