//! D18D Chief of Staff tool handlers for the D23 smart-home runtime.
//!
//! `chief-of-staff-tool-api` owns the model-facing tool contract and
//! `smart-home-runtime` owns device authorization, command validation, state,
//! events, and supervision. This crate keeps the bridge between those surfaces
//! explicit and testable.

#![forbid(unsafe_code)]

use chief_of_staff_tool_api::{
    InMemoryToolRuntime, JsonSchema, PrivilegeTier as ToolPrivilegeTier, SchemaProperty,
    ToolApiError, ToolCallError, ToolConcurrency, ToolDefinition, ToolErrorKind, ToolEventKind,
    ToolExecutionContext, ToolHandlerOutput, ToolIdempotency, ToolSideEffects, ToolStability,
    ToolStreaming,
};
use coding_adventures_json_value::{JsonNumber, JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, Capability, CapabilityId, CommandResult, CommandStatus, CommandType,
    Device, EntityId, Health, StateConfidence, StateSnapshot, StateSource, Value,
};
use smart_home_runtime::{
    RuntimeCommandToolRequest, RuntimeError, RuntimeReadToolOutput, RuntimeReadToolRequest,
    SmartHomeRuntime,
};
use std::cell::RefCell;
use std::rc::Rc;

pub const SMART_HOME_LIST_BRIDGES_TOOL_ID: &str = "smart_home.list_bridges";
pub const SMART_HOME_LIST_DEVICES_TOOL_ID: &str = "smart_home.list_devices";
pub const SMART_HOME_GET_STATE_TOOL_ID: &str = "smart_home.get_state";
pub const SMART_HOME_COMMAND_TOOL_ID: &str = "smart_home.command";
pub const SMART_HOME_OBSERVE_SUPERVISION_TOOL_ID: &str = "smart_home.observe_supervision";

/// Shared, mutable smart-home runtime handle for in-process D18D handlers.
pub type SharedSmartHomeRuntime = Rc<RefCell<SmartHomeRuntime>>;

/// Thin registration helper for smart-home D18D tools.
#[derive(Clone)]
pub struct SmartHomeToolBridge {
    runtime: SharedSmartHomeRuntime,
    default_principal_id: AgentId,
}

impl SmartHomeToolBridge {
    pub fn new(runtime: SharedSmartHomeRuntime, default_principal_id: AgentId) -> Self {
        Self {
            runtime,
            default_principal_id,
        }
    }

    pub fn runtime(&self) -> SharedSmartHomeRuntime {
        self.runtime.clone()
    }

    pub fn default_principal_id(&self) -> &AgentId {
        &self.default_principal_id
    }

    pub fn register_all(&self, tool_runtime: &mut InMemoryToolRuntime) -> Result<(), ToolApiError> {
        for definition in smart_home_tool_definitions() {
            let handler = self.handler_for(&definition.tool_id);
            tool_runtime.register_handler(definition, handler)?;
        }
        Ok(())
    }

    fn handler_for(
        &self,
        tool_id: &str,
    ) -> impl Fn(JsonValue, ToolExecutionContext) -> Result<ToolHandlerOutput, ToolCallError> + 'static
    {
        let runtime = self.runtime.clone();
        let default_principal_id = self.default_principal_id.clone();
        let tool_id = tool_id.to_string();

        move |arguments, context| {
            let principal_id = principal_for_context(&context, &default_principal_id);
            let now_ms = context.requested_at;
            let mut runtime = runtime.borrow_mut();

            match tool_id.as_str() {
                SMART_HOME_LIST_BRIDGES_TOOL_ID => {
                    let _ = expect_object(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::ListBridges,
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_bridges"))
                }
                SMART_HOME_LIST_DEVICES_TOOL_ID => {
                    let request = list_devices_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_devices"))
                }
                SMART_HOME_GET_STATE_TOOL_ID => {
                    let entity_id = required_string(&arguments, "entity_id")?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetState {
                                entity_id: EntityId::trusted(entity_id),
                            },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "get_state"))
                }
                SMART_HOME_COMMAND_TOOL_ID => {
                    let request = command_request(&arguments)?;
                    let result = runtime
                        .execute_command_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(
                        ToolHandlerOutput::new(command_result_json(&result)).with_event(
                            ToolEventKind::Progress,
                            object([
                                ("operation", string("command")),
                                ("status", string(command_status_label(result.status))),
                                ("accepted", JsonValue::Bool(result.status.is_accepted())),
                            ]),
                        ),
                    )
                }
                SMART_HOME_OBSERVE_SUPERVISION_TOOL_ID => {
                    let _ = expect_object(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::ObserveSupervision,
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "observe_supervision"))
                }
                _ => Err(ToolCallError::new(
                    ToolErrorKind::ToolNotFound,
                    format!("unregistered smart-home tool handler `{tool_id}`"),
                )),
            }
        }
    }
}

/// Return the D18D definitions for the first smart-home runtime bridge tools.
pub fn smart_home_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        read_definition(
            SMART_HOME_LIST_BRIDGES_TOOL_ID,
            "List smart-home bridges",
            "List known smart-home bridges from the local D23 runtime.",
            empty_object_schema(),
            collection_output_schema("bridges"),
        ),
        read_definition(
            SMART_HOME_LIST_DEVICES_TOOL_ID,
            "List smart-home devices",
            "List smart-home devices, optionally filtered by bridge, health, or capability.",
            object_schema(
                vec![
                    SchemaProperty::new("bridge_id", JsonSchema::String),
                    SchemaProperty::new("health", JsonSchema::String),
                    SchemaProperty::new("capability_id", JsonSchema::String),
                ],
                vec![],
                false,
            ),
            collection_output_schema("devices"),
        ),
        read_definition(
            SMART_HOME_GET_STATE_TOOL_ID,
            "Get smart-home state",
            "Read the latest cached state for one smart-home entity.",
            object_schema(
                vec![SchemaProperty::new("entity_id", JsonSchema::String)],
                vec!["entity_id"],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new("entity_id", JsonSchema::String),
                    SchemaProperty::new("has_state", JsonSchema::Boolean),
                    SchemaProperty::new("state", JsonSchema::Any),
                ],
                vec!["entity_id", "has_state", "state"],
                false,
            ),
        ),
        command_definition(),
        read_definition(
            SMART_HOME_OBSERVE_SUPERVISION_TOOL_ID,
            "Observe smart-home supervision",
            "Read due smart-home supervision work without mutating the runtime.",
            empty_object_schema(),
            object_schema(
                vec![
                    SchemaProperty::new("generated_at_ms", JsonSchema::Integer),
                    SchemaProperty::new("is_idle", JsonSchema::Boolean),
                    SchemaProperty::new("action_count", JsonSchema::Integer),
                    SchemaProperty::new("pairing_expiry_count", JsonSchema::Integer),
                    SchemaProperty::new("state_refresh_count", JsonSchema::Integer),
                    SchemaProperty::new("desired_state_drift_count", JsonSchema::Integer),
                    SchemaProperty::new("worker_restart_count", JsonSchema::Integer),
                    SchemaProperty::new("due_worker_deadline_count", JsonSchema::Integer),
                    SchemaProperty::new("next_worker_heartbeat_due_at_ms", JsonSchema::Any),
                ],
                vec![
                    "generated_at_ms",
                    "is_idle",
                    "action_count",
                    "pairing_expiry_count",
                    "state_refresh_count",
                    "desired_state_drift_count",
                    "worker_restart_count",
                    "due_worker_deadline_count",
                    "next_worker_heartbeat_due_at_ms",
                ],
                false,
            ),
        ),
    ]
}

pub fn smart_home_tool_definition(tool_id: &str) -> Option<ToolDefinition> {
    smart_home_tool_definitions()
        .into_iter()
        .find(|definition| definition.tool_id == tool_id)
}

fn read_definition(
    tool_id: &str,
    display_name: &str,
    description: &str,
    input_schema: JsonSchema,
    output_schema: JsonSchema,
) -> ToolDefinition {
    ToolDefinition {
        tool_id: tool_id.to_string(),
        display_name: display_name.to_string(),
        description: description.to_string(),
        input_schema,
        output_schema: Some(output_schema),
        side_effects: ToolSideEffects::Read,
        idempotency: ToolIdempotency::Always,
        concurrency: ToolConcurrency::Safe,
        streaming: ToolStreaming::None,
        required_tier: ToolPrivilegeTier::Tier0,
        required_capabilities: vec!["smart_home:read".to_string()],
        preferred_lock_scope: None,
        timeout_seconds: Some(5),
        tags: vec!["smart_home".to_string(), "runtime".to_string()],
        stability: ToolStability::Experimental,
    }
}

fn command_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_COMMAND_TOOL_ID.to_string(),
        display_name: "Command smart-home entity".to_string(),
        description: "Send a low-risk command to a smart-home entity through the D23 runtime."
            .to_string(),
        input_schema: object_schema(
            vec![
                SchemaProperty::new("entity_id", JsonSchema::String),
                SchemaProperty::new("command_type", JsonSchema::String),
                SchemaProperty::new("arguments", JsonSchema::Any),
                SchemaProperty::new("idempotency_key", JsonSchema::String),
                SchemaProperty::new("timeout_ms", JsonSchema::Integer),
            ],
            vec!["entity_id", "command_type"],
            false,
        ),
        output_schema: Some(object_schema(
            vec![
                SchemaProperty::new("command_id", JsonSchema::String),
                SchemaProperty::new("status", JsonSchema::String),
                SchemaProperty::new("accepted", JsonSchema::Boolean),
                SchemaProperty::new("bridge_id", JsonSchema::String),
                SchemaProperty::new("correlation_id", JsonSchema::String),
                SchemaProperty::new("message", JsonSchema::Any),
            ],
            vec![
                "command_id",
                "status",
                "accepted",
                "bridge_id",
                "correlation_id",
                "message",
            ],
            false,
        )),
        side_effects: ToolSideEffects::External,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Serialized,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier1,
        required_capabilities: vec!["smart_home:command".to_string()],
        preferred_lock_scope: Some("smart_home".to_string()),
        timeout_seconds: Some(10),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "command".to_string(),
        ],
        stability: ToolStability::Experimental,
    }
}

fn list_devices_request(arguments: &JsonValue) -> Result<RuntimeReadToolRequest, ToolCallError> {
    Ok(RuntimeReadToolRequest::ListDevices {
        bridge_id: optional_string(arguments, "bridge_id")?.map(BridgeId::trusted),
        health: optional_string(arguments, "health")?
            .map(|value| parse_health(&value))
            .transpose()?,
        capability_id: optional_string(arguments, "capability_id")?.map(CapabilityId::trusted),
    })
}

fn command_request(arguments: &JsonValue) -> Result<RuntimeCommandToolRequest, ToolCallError> {
    let entity_id = EntityId::trusted(required_string(arguments, "entity_id")?);
    let command_type = parse_command_type(&required_string(arguments, "command_type")?)?;
    let argument = optional_field(arguments, "arguments")
        .map(|value| json_to_smart_value_for_command(command_type, value))
        .transpose()?
        .unwrap_or(Value::Null);
    let mut request = RuntimeCommandToolRequest::new(entity_id, command_type, argument);
    if let Some(idempotency_key) = optional_string(arguments, "idempotency_key")? {
        request = request.with_idempotency_key(idempotency_key);
    }
    if let Some(timeout_ms) = optional_u64(arguments, "timeout_ms")? {
        request = request.with_timeout_ms(timeout_ms);
    }
    Ok(request)
}

fn read_output_handler_output(
    output: RuntimeReadToolOutput,
    operation: &'static str,
) -> ToolHandlerOutput {
    ToolHandlerOutput::new(read_output_json(output)).with_event(
        ToolEventKind::Progress,
        object([("operation", string(operation))]),
    )
}

fn read_output_json(output: RuntimeReadToolOutput) -> JsonValue {
    match output {
        RuntimeReadToolOutput::Bridges(bridges) => object([
            (
                "bridges",
                JsonValue::Array(bridges.iter().map(bridge_json).collect()),
            ),
            ("count", integer(bridges.len() as i64)),
        ]),
        RuntimeReadToolOutput::Devices(devices) => object([
            (
                "devices",
                JsonValue::Array(devices.iter().map(device_json).collect()),
            ),
            ("count", integer(devices.len() as i64)),
        ]),
        RuntimeReadToolOutput::State {
            entity_id,
            snapshot,
        } => object([
            ("entity_id", string(entity_id.as_str())),
            ("has_state", JsonValue::Bool(snapshot.is_some())),
            (
                "state",
                snapshot
                    .as_ref()
                    .map(state_snapshot_json)
                    .unwrap_or(JsonValue::Null),
            ),
        ]),
        RuntimeReadToolOutput::Capabilities {
            entity_id,
            capabilities,
        } => object([
            ("entity_id", string(entity_id.as_str())),
            (
                "capabilities",
                JsonValue::Array(capabilities.iter().map(capability_json).collect()),
            ),
            ("count", integer(capabilities.len() as i64)),
        ]),
        RuntimeReadToolOutput::Health(health) => object([
            (
                "bridges",
                JsonValue::Array(
                    health
                        .iter()
                        .map(|snapshot| {
                            object([
                                ("bridge_id", string(snapshot.bridge_id.as_str())),
                                ("integration_id", string(snapshot.integration_id.as_str())),
                                ("health", string(health_label(snapshot.health))),
                                (
                                    "last_seen_at_ms",
                                    snapshot
                                        .last_seen_at_ms
                                        .map(|value| integer(value as i64))
                                        .unwrap_or(JsonValue::Null),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("count", integer(health.len() as i64)),
        ]),
        RuntimeReadToolOutput::SupervisionObservation(observation) => object([
            (
                "generated_at_ms",
                integer(observation.generated_at_ms as i64),
            ),
            ("is_idle", JsonValue::Bool(observation.is_idle())),
            ("action_count", integer(observation.action_count() as i64)),
            (
                "pairing_expiry_count",
                integer(observation.pairing_expiry_count() as i64),
            ),
            (
                "state_refresh_count",
                integer(observation.state_refresh_count() as i64),
            ),
            (
                "desired_state_drift_count",
                integer(observation.desired_state_drift_count() as i64),
            ),
            (
                "worker_restart_count",
                integer(observation.worker_restart_count() as i64),
            ),
            (
                "due_worker_deadline_count",
                integer(observation.due_worker_deadline_count() as i64),
            ),
            (
                "next_worker_heartbeat_due_at_ms",
                observation
                    .next_worker_heartbeat_due_at_ms()
                    .map(|value| integer(value as i64))
                    .unwrap_or(JsonValue::Null),
            ),
        ]),
    }
}

fn bridge_json(bridge: &Bridge) -> JsonValue {
    object([
        ("bridge_id", string(bridge.bridge_id.as_str())),
        ("integration_id", string(bridge.integration_id.as_str())),
        ("transport", string(bridge.transport.as_str())),
        ("health", string(health_label(bridge.health))),
        (
            "address",
            bridge
                .address
                .as_ref()
                .map(|value| string(value))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "last_seen_at_ms",
            bridge
                .last_seen_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn device_json(device: &Device) -> JsonValue {
    object([
        ("device_id", string(device.device_id.as_str())),
        ("bridge_id", string(device.bridge_id.as_str())),
        ("manufacturer", string(&device.manufacturer)),
        ("model", string(&device.model)),
        ("name", string(&device.name)),
        ("health", string(health_label(device.health))),
        (
            "room_id",
            device
                .room_id
                .as_ref()
                .map(|value| string(value))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn capability_json(capability: &Capability) -> JsonValue {
    object([
        ("capability_id", string(capability.capability_id.as_str())),
        ("mode", string(capability_mode_label(capability.mode))),
        (
            "value_kind",
            string(value_kind_label(capability.value_kind)),
        ),
    ])
}

fn state_snapshot_json(snapshot: &StateSnapshot) -> JsonValue {
    object([
        ("entity_id", string(snapshot.entity_id.as_str())),
        ("value", smart_value_to_json(&snapshot.value)),
        ("source", string(state_source_label(snapshot.source))),
        (
            "confidence",
            string(state_confidence_label(snapshot.confidence)),
        ),
        ("observed_at_ms", integer(snapshot.observed_at_ms as i64)),
        ("received_at_ms", integer(snapshot.received_at_ms as i64)),
        (
            "expires_at_ms",
            snapshot
                .expires_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn command_result_json(result: &CommandResult) -> JsonValue {
    object([
        ("command_id", string(result.command_id.as_str())),
        ("status", string(command_status_label(result.status))),
        ("accepted", JsonValue::Bool(result.status.is_accepted())),
        ("bridge_id", string(result.bridge_id.as_str())),
        ("correlation_id", string(result.correlation_id.as_str())),
        (
            "message",
            result
                .message
                .as_ref()
                .map(|message| string(message))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn json_to_smart_value_for_command(
    command_type: CommandType,
    value: &JsonValue,
) -> Result<Value, ToolCallError> {
    match command_type {
        CommandType::TurnOn | CommandType::TurnOff | CommandType::RecallScene => Ok(Value::Null),
        CommandType::SetBrightness => match json_to_smart_value(value)? {
            Value::Integer(value) if (0..=100).contains(&value) => {
                Ok(Value::Percentage(value as u8))
            }
            Value::Percentage(value) => Ok(Value::Percentage(value)),
            other => Ok(other),
        },
        CommandType::SetColor
        | CommandType::SetColorTemperature
        | CommandType::SetLock
        | CommandType::SetThermostatSetpoint => json_to_smart_value(value),
    }
}

fn json_to_smart_value(value: &JsonValue) -> Result<Value, ToolCallError> {
    match value {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(value) => Ok(Value::Bool(*value)),
        JsonValue::String(value) => Ok(Value::Text(value.clone())),
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(Value::Integer(*value)),
        JsonValue::Number(JsonNumber::Float(value)) if value.is_finite() => {
            Ok(Value::Number(*value))
        }
        JsonValue::Number(JsonNumber::Float(_)) => Err(validation_error("number must be finite")),
        JsonValue::Array(values) => values
            .iter()
            .map(json_to_smart_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        JsonValue::Object(fields) => fields
            .iter()
            .map(|(key, value)| Ok((key.clone(), json_to_smart_value(value)?)))
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Object),
    }
}

fn smart_value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Null => JsonValue::Null,
        Value::Bool(value) => JsonValue::Bool(*value),
        Value::Integer(value) => integer(*value),
        Value::Number(value) => JsonValue::Number(JsonNumber::Float(*value)),
        Value::Percentage(value) => integer(i64::from(*value)),
        Value::Text(value) => string(value),
        Value::Object(fields) => JsonValue::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), smart_value_to_json(value)))
                .collect(),
        ),
        Value::Array(values) => JsonValue::Array(values.iter().map(smart_value_to_json).collect()),
    }
}

fn principal_for_context(
    context: &ToolExecutionContext,
    default_principal_id: &AgentId,
) -> AgentId {
    context
        .agent_id
        .as_ref()
        .map(|agent_id| AgentId::trusted(agent_id.clone()))
        .unwrap_or_else(|| default_principal_id.clone())
}

fn parse_command_type(label: &str) -> Result<CommandType, ToolCallError> {
    match label {
        "turn_on" => Ok(CommandType::TurnOn),
        "turn_off" => Ok(CommandType::TurnOff),
        "set_brightness" => Ok(CommandType::SetBrightness),
        "set_color" => Ok(CommandType::SetColor),
        "set_color_temperature" => Ok(CommandType::SetColorTemperature),
        "recall_scene" => Ok(CommandType::RecallScene),
        "set_lock" => Ok(CommandType::SetLock),
        "set_thermostat_setpoint" => Ok(CommandType::SetThermostatSetpoint),
        _ => Err(validation_error(format!("unknown command_type `{label}`"))),
    }
}

fn parse_health(label: &str) -> Result<Health, ToolCallError> {
    match label {
        "unknown" => Ok(Health::Unknown),
        "discoverable" => Ok(Health::Discoverable),
        "unpaired" => Ok(Health::Unpaired),
        "online" => Ok(Health::Online),
        "degraded" => Ok(Health::Degraded),
        "offline" => Ok(Health::Offline),
        "auth_failed" => Ok(Health::AuthFailed),
        "unsupported" => Ok(Health::Unsupported),
        "removed" => Ok(Health::Removed),
        _ => Err(validation_error(format!("unknown health `{label}`"))),
    }
}

fn health_label(health: Health) -> &'static str {
    match health {
        Health::Unknown => "unknown",
        Health::Discoverable => "discoverable",
        Health::Unpaired => "unpaired",
        Health::Online => "online",
        Health::Degraded => "degraded",
        Health::Offline => "offline",
        Health::AuthFailed => "auth_failed",
        Health::Unsupported => "unsupported",
        Health::Removed => "removed",
    }
}

fn command_status_label(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::Accepted => "accepted",
        CommandStatus::Rejected => "rejected",
        CommandStatus::TimedOut => "timed_out",
        CommandStatus::Failed => "failed",
    }
}

fn state_source_label(source: StateSource) -> &'static str {
    match source {
        StateSource::EventStream => "event_stream",
        StateSource::Poll => "poll",
        StateSource::OptimisticCommand => "optimistic_command",
        StateSource::Manual => "manual",
    }
}

fn state_confidence_label(confidence: StateConfidence) -> &'static str {
    match confidence {
        StateConfidence::Confirmed => "confirmed",
        StateConfidence::Optimistic => "optimistic",
        StateConfidence::Stale => "stale",
        StateConfidence::Unknown => "unknown",
    }
}

fn capability_mode_label(mode: smart_home_core::CapabilityMode) -> &'static str {
    match mode {
        smart_home_core::CapabilityMode::Observe => "observe",
        smart_home_core::CapabilityMode::Command => "command",
        smart_home_core::CapabilityMode::ObserveAndCommand => "observe_and_command",
    }
}

fn value_kind_label(kind: smart_home_core::ValueKind) -> &'static str {
    match kind {
        smart_home_core::ValueKind::Null => "null",
        smart_home_core::ValueKind::Boolean => "boolean",
        smart_home_core::ValueKind::Integer => "integer",
        smart_home_core::ValueKind::Number => "number",
        smart_home_core::ValueKind::Percentage => "percentage",
        smart_home_core::ValueKind::Text => "text",
        smart_home_core::ValueKind::Object => "object",
        smart_home_core::ValueKind::Array => "array",
    }
}

trait BridgeTransportLabel {
    fn as_str(self) -> &'static str;
}

impl BridgeTransportLabel for smart_home_core::BridgeTransport {
    fn as_str(self) -> &'static str {
        match self {
            smart_home_core::BridgeTransport::LanHttp => "lan_http",
            smart_home_core::BridgeTransport::Mdns => "mdns",
            smart_home_core::BridgeTransport::Serial => "serial",
            smart_home_core::BridgeTransport::Ble => "ble",
            smart_home_core::BridgeTransport::Cloud => "cloud",
            smart_home_core::BridgeTransport::LocalProcess => "local_process",
        }
    }
}

fn runtime_error(error: RuntimeError) -> ToolCallError {
    let kind = match error {
        RuntimeError::UnauthorizedCommand { .. } | RuntimeError::UnauthorizedTool { .. } => {
            ToolErrorKind::ToolPermissionDenied
        }
        RuntimeError::DuplicatePairingSession(_)
        | RuntimeError::PairingSessionNotPending { .. }
        | RuntimeError::PairingSessionExpired { .. } => ToolErrorKind::ToolConflict,
        _ => ToolErrorKind::ToolExecutionError,
    };
    ToolCallError {
        kind,
        message: error.to_string(),
        details: JsonValue::Null,
    }
}

fn required_string(value: &JsonValue, field: &str) -> Result<String, ToolCallError> {
    match optional_field(value, field) {
        Some(JsonValue::String(value)) => Ok(value.clone()),
        Some(_) => Err(validation_error(format!("{field} must be a string"))),
        None => Err(validation_error(format!("{field} is required"))),
    }
}

fn optional_string(value: &JsonValue, field: &str) -> Result<Option<String>, ToolCallError> {
    match optional_field(value, field) {
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(JsonValue::Null) | None => Ok(None),
        Some(_) => Err(validation_error(format!("{field} must be a string"))),
    }
}

fn optional_u64(value: &JsonValue, field: &str) -> Result<Option<u64>, ToolCallError> {
    match optional_field(value, field) {
        Some(JsonValue::Number(JsonNumber::Integer(value))) if *value >= 0 => {
            Ok(Some(*value as u64))
        }
        Some(JsonValue::Null) | None => Ok(None),
        Some(_) => Err(validation_error(format!(
            "{field} must be a non-negative integer"
        ))),
    }
}

fn optional_field<'a>(value: &'a JsonValue, field: &str) -> Option<&'a JsonValue> {
    let fields = match value {
        JsonValue::Object(fields) => fields,
        _ => return None,
    };
    fields
        .iter()
        .find_map(|(name, value)| (name == field).then_some(value))
}

fn expect_object(value: &JsonValue) -> Result<&[(String, JsonValue)], ToolCallError> {
    match value {
        JsonValue::Object(fields) => Ok(fields),
        _ => Err(validation_error("arguments must be an object")),
    }
}

fn validation_error(message: impl Into<String>) -> ToolCallError {
    ToolCallError::new(ToolErrorKind::ToolValidationError, message)
}

fn object<const N: usize>(fields: [(&str, JsonValue); N]) -> JsonValue {
    JsonValue::Object(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

fn string(value: impl AsRef<str>) -> JsonValue {
    JsonValue::String(value.as_ref().to_string())
}

fn integer(value: i64) -> JsonValue {
    JsonValue::Number(JsonNumber::Integer(value))
}

fn object_schema(
    properties: Vec<SchemaProperty>,
    required: Vec<&str>,
    allow_unknown_fields: bool,
) -> JsonSchema {
    JsonSchema::Object {
        properties,
        required: required.into_iter().map(str::to_string).collect(),
        allow_unknown_fields,
    }
}

fn empty_object_schema() -> JsonSchema {
    object_schema(Vec::new(), Vec::new(), false)
}

fn collection_output_schema(field_name: &str) -> JsonSchema {
    object_schema(
        vec![
            SchemaProperty::new(
                field_name,
                JsonSchema::Array {
                    items: Box::new(JsonSchema::Any),
                },
            ),
            SchemaProperty::new("count", JsonSchema::Integer),
        ],
        vec![field_name, "count"],
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chief_of_staff_tool_api::{
        RequestedBy, ToolCatalogExport, ToolExecutionJournal, ToolInvocationRequest,
        ToolValidationReport,
    };
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use smart_home_testkit::hue_lighting_runtime;

    const AGENT_ID: &str = "agent:chief-smart-home";

    #[test]
    fn smart_home_tool_definitions_are_valid() {
        let definitions = smart_home_tool_definitions();
        let export = ToolCatalogExport::from_definitions(definitions.iter());

        assert_eq!(definitions.len(), 5);
        assert!(export.ok());
        assert!(export.tool_ids().contains(&SMART_HOME_COMMAND_TOOL_ID));
        assert_eq!(
            export.summary.required_capability_count("smart_home:read"),
            4
        );
        assert_eq!(
            export
                .summary
                .required_capability_count("smart_home:command"),
            1
        );
        assert!(smart_home_tool_definition(SMART_HOME_GET_STATE_TOOL_ID).is_some());
        assert!(smart_home_tool_definition("smart_home.unknown").is_none());
    }

    #[test]
    fn chief_of_staff_runtime_drives_smart_home_light_end_to_end() {
        let runtime = Rc::new(RefCell::new(hue_lighting_runtime()));
        runtime.borrow_mut().registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-smart-home"),
                AgentId::trusted(AGENT_ID),
                PrivilegeTier::LowRisk,
                "user:test",
                1_000,
            ),
        );

        let bridge = SmartHomeToolBridge::new(runtime.clone(), AgentId::trusted(AGENT_ID));
        let mut tool_runtime = InMemoryToolRuntime::new();
        bridge.register_all(&mut tool_runtime).unwrap();

        let list_request = request(
            "call-list-devices",
            SMART_HOME_LIST_DEVICES_TOOL_ID,
            object([]),
            1_000,
        );
        let list_trace = tool_runtime.invoke_with_events(&list_request);
        assert!(list_trace.result.ok);
        assert_eq!(list_trace.summary().terminal_event_count, 1);
        assert_eq!(
            field(list_trace.result.output.as_ref().unwrap(), "count"),
            Some(&integer(1))
        );

        let command_request = request(
            "call-turn-on",
            SMART_HOME_COMMAND_TOOL_ID,
            object([
                ("entity_id", string("entity-light-1")),
                ("command_type", string("turn_on")),
                ("idempotency_key", string("demo-turn-on")),
            ]),
            1_100,
        );
        let command_trace = tool_runtime.invoke_with_events(&command_request);
        assert!(command_trace.result.ok);
        assert_eq!(
            field(command_trace.result.output.as_ref().unwrap(), "status"),
            Some(&string("accepted"))
        );
        assert_eq!(command_trace.summary().progress_event_count, 1);

        let state_request = request(
            "call-get-state",
            SMART_HOME_GET_STATE_TOOL_ID,
            object([("entity_id", string("entity-light-1"))]),
            1_101,
        );
        let state_trace = tool_runtime.invoke_with_events(&state_request);
        assert!(state_trace.result.ok);
        let state_output = state_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(state_output, "has_state"),
            Some(&JsonValue::Bool(true))
        );

        let mut journal = ToolExecutionJournal::new();
        journal.record_trace(list_request, list_trace);
        journal.record_trace(command_request, command_trace);
        journal.record_trace(state_request, state_trace);

        let journal_summary = journal.summary();
        assert_eq!(journal_summary.invocation_count, 3);
        assert_eq!(journal_summary.completed_count, 3);
        assert_eq!(journal.audit_records().len(), 3);

        let runtime = runtime.borrow();
        assert_eq!(runtime.optimistic_state_count(), 1);
        assert_eq!(
            runtime.registry().counts().authorization_decisions,
            4,
            "read calls record tool authorization, while command records tool and command authorization"
        );
    }

    #[test]
    fn smart_home_handler_reports_runtime_authorization_denials() {
        let runtime = Rc::new(RefCell::new(hue_lighting_runtime()));
        let bridge = SmartHomeToolBridge::new(runtime, AgentId::trusted(AGENT_ID));
        let mut tool_runtime = InMemoryToolRuntime::new();
        bridge.register_all(&mut tool_runtime).unwrap();

        let denied = tool_runtime.invoke(&request(
            "call-denied",
            SMART_HOME_COMMAND_TOOL_ID,
            object([
                ("entity_id", string("entity-light-1")),
                ("command_type", string("turn_on")),
            ]),
            1_000,
        ));

        assert!(!denied.ok);
        assert_eq!(
            denied.error.as_ref().map(|error| error.kind),
            Some(ToolErrorKind::ToolPermissionDenied)
        );
    }

    #[test]
    fn malformed_smart_home_tool_calls_fail_d18d_validation_before_handler() {
        let runtime = Rc::new(RefCell::new(hue_lighting_runtime()));
        let bridge = SmartHomeToolBridge::new(runtime, AgentId::trusted(AGENT_ID));
        let mut tool_runtime = InMemoryToolRuntime::new();
        bridge.register_all(&mut tool_runtime).unwrap();

        let report: ToolValidationReport = tool_runtime.validate(&request(
            "call-invalid",
            SMART_HOME_COMMAND_TOOL_ID,
            object([("command_type", string("turn_on"))]),
            1_000,
        ));
        assert!(!report.ok);

        let trace = tool_runtime.invoke_with_events(&request(
            "call-invalid",
            SMART_HOME_COMMAND_TOOL_ID,
            object([("command_type", string("turn_on"))]),
            1_000,
        ));
        assert!(!trace.result.ok);
        assert_eq!(
            trace.result.error.as_ref().map(|error| error.kind),
            Some(ToolErrorKind::ToolValidationError)
        );
    }

    fn request(
        call_id: &str,
        tool_id: &str,
        arguments: JsonValue,
        requested_at: u64,
    ) -> ToolInvocationRequest {
        ToolInvocationRequest {
            call_id: call_id.to_string(),
            tool_id: tool_id.to_string(),
            arguments,
            requested_by: RequestedBy::Agent,
            session_id: Some("session-smart-home".to_string()),
            job_id: Some("job-evening-lights".to_string()),
            agent_id: Some(AGENT_ID.to_string()),
            user_id: Some("user:test".to_string()),
            requested_at,
            deadline_at: None,
            idempotency_key: None,
        }
    }

    fn field<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
        let JsonValue::Object(fields) = value else {
            return None;
        };
        fields
            .iter()
            .find_map(|(field_name, value)| (field_name == name).then_some(value))
    }
}
