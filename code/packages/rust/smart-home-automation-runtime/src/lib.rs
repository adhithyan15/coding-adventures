//! Deterministic automation planning and execution for the smart-home runtime.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use smart_home_core::{
    AgentId, CommandResult, CommandType, DeviceEvent, DeviceEventType, EntityId, SceneId, Value,
};
use smart_home_runtime::{RuntimeCommandToolRequest, SmartHomeRuntime};
use smart_home_runtime_store::DurableAutomationDefinition;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const SNAPSHOT_SCHEMA_VERSION: u32 = 1;
const MAX_AUDIT_RECORDS: usize = 1_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationDefinition {
    pub automation_id: String,
    pub enabled: bool,
    pub trigger: AutomationTrigger,
    #[serde(default)]
    pub conditions: Vec<AutomationCondition>,
    pub actions: Vec<AutomationAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationTrigger {
    Schedule {
        every_ms: u64,
        #[serde(default)]
        offset_ms: u64,
    },
    Event {
        event_type: AutomationEventType,
        #[serde(default)]
        entity_id: Option<EntityId>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationEventType {
    Discovered,
    Updated,
    Removed,
    Unavailable,
    Error,
    Health,
}

impl AutomationEventType {
    fn matches(self, event_type: DeviceEventType) -> bool {
        matches!(
            (self, event_type),
            (Self::Discovered, DeviceEventType::Discovered)
                | (Self::Updated, DeviceEventType::Updated)
                | (Self::Removed, DeviceEventType::Removed)
                | (Self::Unavailable, DeviceEventType::Unavailable)
                | (Self::Error, DeviceEventType::Error)
                | (Self::Health, DeviceEventType::Health)
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationCondition {
    StateEquals {
        entity_id: EntityId,
        expected: Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutomationAction {
    Command {
        entity_id: EntityId,
        command_type: CommandType,
        arguments: Value,
        #[serde(default)]
        timeout_ms: Option<u64>,
    },
    Scene {
        scene_id: SceneId,
        #[serde(default)]
        timeout_ms: Option<u64>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum AutomationTriggerInput {
    Schedule,
    Event(Box<DeviceEvent>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlannedAutomationCommand {
    pub entity_id: EntityId,
    pub command_type: CommandType,
    pub arguments: Value,
    pub idempotency_key: String,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutomationAuditOutcome {
    Planned,
    Executed,
    ConditionNotMet,
    AlreadyExecuted,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationAuditRecord {
    pub audit_id: String,
    pub automation_id: String,
    pub trigger_key: String,
    pub evaluated_at_ms: u64,
    pub dry_run: bool,
    pub outcome: AutomationAuditOutcome,
    pub condition_results: Vec<bool>,
    pub planned_commands: Vec<PlannedAutomationCommand>,
    pub command_results: Vec<CommandResult>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationEvaluationReport {
    pub evaluated_at_ms: u64,
    pub dry_run: bool,
    pub records: Vec<AutomationAuditRecord>,
}

impl AutomationEvaluationReport {
    pub fn matched_automation_count(&self) -> usize {
        self.records.len()
    }

    pub fn executed_automation_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.outcome == AutomationAuditOutcome::Executed)
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutomationRuntimeSnapshot {
    pub schema_version: u32,
    pub definitions: Vec<AutomationDefinition>,
    pub completed_trigger_keys: Vec<String>,
    pub audit_records: Vec<AutomationAuditRecord>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmartHomeAutomationRuntime {
    definitions: BTreeMap<String, AutomationDefinition>,
    completed_trigger_keys: BTreeSet<String>,
    audit_records: Vec<AutomationAuditRecord>,
}

impl Default for SmartHomeAutomationRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartHomeAutomationRuntime {
    pub fn new() -> Self {
        Self {
            definitions: BTreeMap::new(),
            completed_trigger_keys: BTreeSet::new(),
            audit_records: Vec::new(),
        }
    }

    pub fn restore(
        durable_definitions: &[DurableAutomationDefinition],
        snapshot: Option<&serde_json::Value>,
    ) -> Result<Self, AutomationError> {
        if let Some(snapshot) = snapshot {
            let snapshot: AutomationRuntimeSnapshot =
                serde_json::from_value(snapshot.clone()).map_err(AutomationError::decode)?;
            return Self::from_snapshot(snapshot);
        }

        let mut runtime = Self::new();
        for definition in durable_definitions {
            let body: DurableDefinitionBody = serde_json::from_value(definition.definition.clone())
                .map_err(AutomationError::decode)?;
            runtime.upsert_definition(AutomationDefinition {
                automation_id: definition.automation_id.clone(),
                enabled: definition.enabled,
                trigger: body.trigger,
                conditions: body.conditions,
                actions: body.actions,
            })?;
        }
        Ok(runtime)
    }

    pub fn from_snapshot(snapshot: AutomationRuntimeSnapshot) -> Result<Self, AutomationError> {
        if snapshot.schema_version != SNAPSHOT_SCHEMA_VERSION {
            return Err(AutomationError::UnsupportedSchema(snapshot.schema_version));
        }
        let mut runtime = Self::new();
        for definition in snapshot.definitions {
            runtime.upsert_definition(definition)?;
        }
        runtime.completed_trigger_keys = snapshot.completed_trigger_keys.into_iter().collect();
        runtime.audit_records = snapshot.audit_records;
        runtime.trim_audit();
        Ok(runtime)
    }

    pub fn snapshot(&self) -> AutomationRuntimeSnapshot {
        AutomationRuntimeSnapshot {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            definitions: self.definitions.values().cloned().collect(),
            completed_trigger_keys: self.completed_trigger_keys.iter().cloned().collect(),
            audit_records: self.audit_records.clone(),
        }
    }

    pub fn snapshot_json(&self) -> Result<serde_json::Value, AutomationError> {
        serde_json::to_value(self.snapshot()).map_err(AutomationError::encode)
    }

    pub fn durable_definitions(&self) -> Result<Vec<DurableAutomationDefinition>, AutomationError> {
        self.definitions
            .values()
            .map(|definition| {
                let body = DurableDefinitionBody {
                    trigger: definition.trigger.clone(),
                    conditions: definition.conditions.clone(),
                    actions: definition.actions.clone(),
                };
                let value = serde_json::to_value(body).map_err(AutomationError::encode)?;
                DurableAutomationDefinition::new(
                    definition.automation_id.clone(),
                    definition.enabled,
                    value,
                )
                .map_err(|error| AutomationError::Validation(error.to_string()))
            })
            .collect()
    }

    pub fn definitions(&self) -> impl Iterator<Item = &AutomationDefinition> {
        self.definitions.values()
    }

    pub fn audit_records(&self) -> &[AutomationAuditRecord] {
        &self.audit_records
    }

    pub fn upsert_definition(
        &mut self,
        definition: AutomationDefinition,
    ) -> Result<Option<AutomationDefinition>, AutomationError> {
        validate_definition(&definition)?;
        Ok(self
            .definitions
            .insert(definition.automation_id.clone(), definition))
    }

    pub fn remove_definition(&mut self, automation_id: &str) -> Option<AutomationDefinition> {
        self.definitions.remove(automation_id)
    }

    pub fn evaluate(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        input: AutomationTriggerInput,
        dry_run: bool,
        now_ms: u64,
    ) -> Result<AutomationEvaluationReport, AutomationError> {
        let definitions = self.definitions.values().cloned().collect::<Vec<_>>();
        let mut records = Vec::new();

        for definition in definitions {
            if !definition.enabled {
                continue;
            }
            let Some(trigger_key) = trigger_key(&definition.trigger, &input, now_ms) else {
                continue;
            };
            let occurrence_key = format!("{}:{trigger_key}", definition.automation_id);
            if self.completed_trigger_keys.contains(&occurrence_key) {
                if dry_run {
                    records.push(audit_record(
                        &definition,
                        &trigger_key,
                        now_ms,
                        true,
                        AutomationAuditOutcome::AlreadyExecuted,
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Some("trigger occurrence was already consumed".to_string()),
                    ));
                }
                continue;
            }

            let condition_results = definition
                .conditions
                .iter()
                .map(|condition| condition_matches(condition, runtime))
                .collect::<Vec<_>>();
            if condition_results.iter().any(|matched| !matched) {
                let record = audit_record(
                    &definition,
                    &trigger_key,
                    now_ms,
                    dry_run,
                    AutomationAuditOutcome::ConditionNotMet,
                    condition_results,
                    Vec::new(),
                    Vec::new(),
                    Some("one or more automation conditions did not match".to_string()),
                );
                if !dry_run {
                    self.completed_trigger_keys.insert(occurrence_key);
                    self.push_audit(record.clone());
                }
                records.push(record);
                continue;
            }

            let planned_commands = plan_actions(&definition, runtime, &trigger_key)?;
            if dry_run {
                records.push(audit_record(
                    &definition,
                    &trigger_key,
                    now_ms,
                    true,
                    AutomationAuditOutcome::Planned,
                    condition_results,
                    planned_commands,
                    Vec::new(),
                    None,
                ));
                continue;
            }

            let previous_runtime = runtime.clone();
            let mut command_results = Vec::new();
            let mut failure = None;
            for command in &planned_commands {
                let mut request = RuntimeCommandToolRequest::new(
                    command.entity_id.clone(),
                    command.command_type,
                    command.arguments.clone(),
                )
                .with_idempotency_key(command.idempotency_key.clone());
                if let Some(timeout_ms) = command.timeout_ms {
                    request = request.with_timeout_ms(timeout_ms);
                }
                match runtime.execute_command_tool(principal_id.clone(), request, now_ms) {
                    Ok(result) => command_results.push(result),
                    Err(error) => {
                        failure = Some(error.to_string());
                        break;
                    }
                }
            }
            if failure.is_some() {
                *runtime = previous_runtime;
            }
            let outcome = if failure.is_some() {
                AutomationAuditOutcome::Failed
            } else {
                AutomationAuditOutcome::Executed
            };
            let record = audit_record(
                &definition,
                &trigger_key,
                now_ms,
                false,
                outcome,
                condition_results,
                planned_commands,
                command_results,
                failure,
            );
            self.completed_trigger_keys.insert(occurrence_key);
            self.push_audit(record.clone());
            records.push(record);
        }

        Ok(AutomationEvaluationReport {
            evaluated_at_ms: now_ms,
            dry_run,
            records,
        })
    }

    fn push_audit(&mut self, record: AutomationAuditRecord) {
        self.audit_records.push(record);
        self.trim_audit();
    }

    fn trim_audit(&mut self) {
        if self.audit_records.len() > MAX_AUDIT_RECORDS {
            self.audit_records
                .drain(..self.audit_records.len() - MAX_AUDIT_RECORDS);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DurableDefinitionBody {
    trigger: AutomationTrigger,
    #[serde(default)]
    conditions: Vec<AutomationCondition>,
    actions: Vec<AutomationAction>,
}

fn validate_definition(definition: &AutomationDefinition) -> Result<(), AutomationError> {
    if definition.automation_id.trim().is_empty() {
        return Err(AutomationError::Validation(
            "automation_id must not be empty".to_string(),
        ));
    }
    if definition.actions.is_empty() {
        return Err(AutomationError::Validation(
            "automation actions must not be empty".to_string(),
        ));
    }
    if let AutomationTrigger::Schedule { every_ms, .. } = definition.trigger {
        if every_ms == 0 {
            return Err(AutomationError::Validation(
                "schedule every_ms must be greater than zero".to_string(),
            ));
        }
    }
    Ok(())
}

fn trigger_key(
    trigger: &AutomationTrigger,
    input: &AutomationTriggerInput,
    now_ms: u64,
) -> Option<String> {
    match (trigger, input) {
        (
            AutomationTrigger::Schedule {
                every_ms,
                offset_ms,
            },
            AutomationTriggerInput::Schedule,
        ) if now_ms >= *offset_ms => Some(format!("schedule:{}", (now_ms - offset_ms) / every_ms)),
        (
            AutomationTrigger::Event {
                event_type,
                entity_id,
            },
            AutomationTriggerInput::Event(event),
        ) if event_type.matches(event.event_type)
            && entity_id
                .as_ref()
                .is_none_or(|expected| event.entity_id.as_ref() == Some(expected)) =>
        {
            Some(format!("event:{}", event.event_id.as_str()))
        }
        _ => None,
    }
}

fn condition_matches(condition: &AutomationCondition, runtime: &SmartHomeRuntime) -> bool {
    match condition {
        AutomationCondition::StateEquals {
            entity_id,
            expected,
        } => runtime
            .registry()
            .state(entity_id)
            .is_some_and(|snapshot| &snapshot.value == expected),
    }
}

fn plan_actions(
    definition: &AutomationDefinition,
    runtime: &SmartHomeRuntime,
    trigger_key: &str,
) -> Result<Vec<PlannedAutomationCommand>, AutomationError> {
    let mut planned = Vec::new();
    for action in &definition.actions {
        match action {
            AutomationAction::Command {
                entity_id,
                command_type,
                arguments,
                timeout_ms,
            } => planned.push(planned_command(
                definition,
                trigger_key,
                planned.len(),
                entity_id.clone(),
                *command_type,
                arguments.clone(),
                *timeout_ms,
            )),
            AutomationAction::Scene {
                scene_id,
                timeout_ms,
            } => {
                let scene = runtime.registry().scene(scene_id).ok_or_else(|| {
                    AutomationError::Validation(format!("unknown scene {}", scene_id.as_str()))
                })?;
                for action in &scene.actions {
                    let (command_type, arguments) = command_for_scene_value(&action.desired_state)?;
                    planned.push(planned_command(
                        definition,
                        trigger_key,
                        planned.len(),
                        action.entity_id.clone(),
                        command_type,
                        arguments,
                        *timeout_ms,
                    ));
                }
            }
        }
    }
    Ok(planned)
}

fn planned_command(
    definition: &AutomationDefinition,
    trigger_key: &str,
    index: usize,
    entity_id: EntityId,
    command_type: CommandType,
    arguments: Value,
    timeout_ms: Option<u64>,
) -> PlannedAutomationCommand {
    PlannedAutomationCommand {
        entity_id,
        command_type,
        arguments,
        idempotency_key: format!(
            "automation:{}:{trigger_key}:action:{index}",
            definition.automation_id
        ),
        timeout_ms,
    }
}

fn command_for_scene_value(value: &Value) -> Result<(CommandType, Value), AutomationError> {
    match value {
        Value::Bool(true) => Ok((CommandType::TurnOn, Value::Null)),
        Value::Bool(false) => Ok((CommandType::TurnOff, Value::Null)),
        Value::Percentage(_) => Ok((CommandType::SetBrightness, value.clone())),
        Value::Object(fields) => {
            if let Some(Value::Bool(on)) = object_field(fields, "on") {
                return Ok((
                    if *on {
                        CommandType::TurnOn
                    } else {
                        CommandType::TurnOff
                    },
                    Value::Null,
                ));
            }
            if let Some(brightness) = object_field(fields, "brightness") {
                return Ok((CommandType::SetBrightness, brightness.clone()));
            }
            Err(AutomationError::Validation(
                "scene object state requires `on` or `brightness`".to_string(),
            ))
        }
        _ => Err(AutomationError::Validation(
            "scene state must be boolean, percentage, or a supported object".to_string(),
        )),
    }
}

fn object_field<'a>(fields: &'a [(String, Value)], name: &str) -> Option<&'a Value> {
    fields
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
}

#[allow(clippy::too_many_arguments)]
fn audit_record(
    definition: &AutomationDefinition,
    trigger_key: &str,
    evaluated_at_ms: u64,
    dry_run: bool,
    outcome: AutomationAuditOutcome,
    condition_results: Vec<bool>,
    planned_commands: Vec<PlannedAutomationCommand>,
    command_results: Vec<CommandResult>,
    message: Option<String>,
) -> AutomationAuditRecord {
    AutomationAuditRecord {
        audit_id: format!(
            "automation-audit:{}:{trigger_key}:{evaluated_at_ms}",
            definition.automation_id
        ),
        automation_id: definition.automation_id.clone(),
        trigger_key: trigger_key.to_string(),
        evaluated_at_ms,
        dry_run,
        outcome,
        condition_results,
        planned_commands,
        command_results,
        message,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomationError {
    Validation(String),
    Encode(String),
    Decode(String),
    UnsupportedSchema(u32),
}

impl AutomationError {
    fn encode(error: serde_json::Error) -> Self {
        Self::Encode(error.to_string())
    }

    fn decode(error: serde_json::Error) -> Self {
        Self::Decode(error.to_string())
    }
}

impl fmt::Display for AutomationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(f, "invalid automation: {message}"),
            Self::Encode(message) => write!(f, "could not encode automation runtime: {message}"),
            Self::Decode(message) => write!(f, "could not decode automation runtime: {message}"),
            Self::UnsupportedSchema(version) => {
                write!(f, "unsupported automation runtime schema version {version}")
            }
        }
    }
}

impl std::error::Error for AutomationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        Bridge, BridgeId, BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId, Device,
        DeviceId, Entity, EntityKind, EventId, Health, IntegrationId, PrivilegeTier, Scene,
        SceneAction, SceneScope, StateConfidence, StateSnapshot, StateSource,
    };

    fn fixture() -> (SmartHomeRuntime, AgentId) {
        let principal = AgentId::trusted("agent:automation");
        let mut runtime = SmartHomeRuntime::new();
        runtime
            .upsert_bridge(Bridge::new(
                BridgeId::trusted("bridge-1"),
                IntegrationId::trusted("hue"),
                BridgeTransport::LanHttp,
            ))
            .unwrap();
        runtime
            .upsert_device(Device {
                device_id: DeviceId::trusted("device-1"),
                bridge_id: BridgeId::trusted("bridge-1"),
                manufacturer: "Signify".to_string(),
                model: "Hue".to_string(),
                name: "Kitchen".to_string(),
                serial: None,
                firmware_version: None,
                room_id: Some("kitchen".to_string()),
                entity_ids: Vec::new(),
                identifiers: Vec::new(),
                health: Health::Online,
                metadata: Vec::new(),
            })
            .unwrap();
        for entity_id in ["light-kitchen", "light-hall"] {
            runtime
                .upsert_entity(Entity {
                    entity_id: EntityId::trusted(entity_id),
                    device_id: DeviceId::trusted("device-1"),
                    kind: EntityKind::Light,
                    name: entity_id.to_string(),
                    capabilities: vec![Capability::light_on_off(), Capability::light_brightness()],
                    state: None,
                    metadata: Vec::new(),
                })
                .unwrap();
        }
        runtime
            .registry_mut()
            .apply_state_snapshot(StateSnapshot {
                entity_id: EntityId::trusted("light-kitchen"),
                value: Value::Bool(true),
                source: StateSource::Poll,
                observed_at_ms: 1,
                received_at_ms: 1,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            })
            .unwrap();
        runtime
            .upsert_scene(Scene {
                scene_id: SceneId::trusted("scene-night"),
                scope: SceneScope::Home,
                native_ref: None,
                actions: vec![
                    SceneAction {
                        entity_id: EntityId::trusted("light-kitchen"),
                        desired_state: Value::Bool(false),
                    },
                    SceneAction {
                        entity_id: EntityId::trusted("light-hall"),
                        desired_state: Value::Percentage(10),
                    },
                ],
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-automation"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                0,
            ));
        (runtime, principal)
    }

    fn schedule_definition() -> AutomationDefinition {
        AutomationDefinition {
            automation_id: "night-mode".to_string(),
            enabled: true,
            trigger: AutomationTrigger::Schedule {
                every_ms: 1_000,
                offset_ms: 500,
            },
            conditions: vec![AutomationCondition::StateEquals {
                entity_id: EntityId::trusted("light-kitchen"),
                expected: Value::Bool(true),
            }],
            actions: vec![AutomationAction::Scene {
                scene_id: SceneId::trusted("scene-night"),
                timeout_ms: Some(2_000),
            }],
        }
    }

    #[test]
    fn schedule_dry_run_expands_scene_without_mutating_or_consuming_trigger() {
        let (mut runtime, principal) = fixture();
        let mut automations = SmartHomeAutomationRuntime::new();
        automations
            .upsert_definition(schedule_definition())
            .unwrap();

        let report = automations
            .evaluate(
                &mut runtime,
                principal,
                AutomationTriggerInput::Schedule,
                true,
                1_600,
            )
            .unwrap();

        assert_eq!(report.records.len(), 1);
        assert_eq!(report.records[0].outcome, AutomationAuditOutcome::Planned);
        assert_eq!(report.records[0].planned_commands.len(), 2);
        assert!(automations.audit_records().is_empty());
        assert_eq!(
            runtime
                .registry()
                .state(&EntityId::trusted("light-kitchen"))
                .unwrap()
                .value,
            Value::Bool(true)
        );
    }

    #[test]
    fn schedule_executes_once_and_snapshot_restores_idempotency_and_audit() {
        let (mut runtime, principal) = fixture();
        let mut automations = SmartHomeAutomationRuntime::new();
        automations
            .upsert_definition(schedule_definition())
            .unwrap();

        let first = automations
            .evaluate(
                &mut runtime,
                principal.clone(),
                AutomationTriggerInput::Schedule,
                false,
                1_600,
            )
            .unwrap();
        let repeated = automations
            .evaluate(
                &mut runtime,
                principal,
                AutomationTriggerInput::Schedule,
                false,
                1_700,
            )
            .unwrap();

        assert_eq!(first.executed_automation_count(), 1);
        assert!(repeated.records.is_empty());
        assert_eq!(automations.audit_records().len(), 1);
        assert_eq!(
            runtime
                .registry()
                .state(&EntityId::trusted("light-kitchen"))
                .unwrap()
                .value,
            Value::Object(vec![("light.on_off".to_string(), Value::Bool(false))])
        );

        let restored = SmartHomeAutomationRuntime::from_snapshot(automations.snapshot()).unwrap();
        assert_eq!(restored.audit_records().len(), 1);
        assert_eq!(restored.completed_trigger_keys.len(), 1);
    }

    #[test]
    fn event_trigger_respects_entity_and_audits_failed_condition_once() {
        let (mut runtime, principal) = fixture();
        let mut automations = SmartHomeAutomationRuntime::new();
        let mut definition = schedule_definition();
        definition.automation_id = "updated-night".to_string();
        definition.trigger = AutomationTrigger::Event {
            event_type: AutomationEventType::Updated,
            entity_id: Some(EntityId::trusted("light-hall")),
        };
        definition.conditions[0] = AutomationCondition::StateEquals {
            entity_id: EntityId::trusted("light-kitchen"),
            expected: Value::Bool(false),
        };
        automations.upsert_definition(definition).unwrap();
        let event = DeviceEvent {
            event_id: EventId::trusted("event-update-1"),
            bridge_id: BridgeId::trusted("bridge-1"),
            device_id: Some(DeviceId::trusted("device-1")),
            entity_id: Some(EntityId::trusted("light-hall")),
            observed_at_ms: 2_000,
            received_at_ms: 2_001,
            event_type: DeviceEventType::Updated,
            state_delta: None,
            raw_ref: None,
            correlation_id: None,
            metadata: Vec::new(),
        };

        let report = automations
            .evaluate(
                &mut runtime,
                principal,
                AutomationTriggerInput::Event(Box::new(event)),
                false,
                2_001,
            )
            .unwrap();

        assert_eq!(report.records.len(), 1);
        assert_eq!(
            report.records[0].outcome,
            AutomationAuditOutcome::ConditionNotMet
        );
        assert_eq!(automations.audit_records().len(), 1);
    }

    #[test]
    fn durable_definitions_round_trip_without_execution_state() {
        let mut automations = SmartHomeAutomationRuntime::new();
        automations
            .upsert_definition(schedule_definition())
            .unwrap();
        let durable = automations.durable_definitions().unwrap();
        let restored = SmartHomeAutomationRuntime::restore(&durable, None).unwrap();

        assert_eq!(
            restored.definitions().cloned().collect::<Vec<_>>(),
            vec![schedule_definition()]
        );
    }
}
