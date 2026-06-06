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
    Device, EntityId, Health, IntegrationId, Metadata, StateConfidence, StateSnapshot, StateSource,
    Value,
};
use smart_home_discovery::{DiscoveryRecord, DiscoverySource};
use smart_home_runtime::{
    PairingSessionStatus, RuntimeCommandToolRequest, RuntimeDiscoverToolOutput,
    RuntimeDiscoverToolRequest, RuntimeError, RuntimeEventCheckpoint, RuntimeEventFilter,
    RuntimePairBridgeToolOutput, RuntimePairBridgeToolRequest, RuntimePairingSession,
    RuntimePairingSessionId, RuntimeReadToolOutput, RuntimeReadToolRequest,
    RuntimeSubscribeToolOutput, RuntimeSubscribeToolRequest, RuntimeSubscriptionId,
    ScheduledDiscoveryWorkerSnapshot, SmartHomeRuntime,
};
use std::cell::RefCell;
use std::rc::Rc;

pub const SMART_HOME_LIST_BRIDGES_TOOL_ID: &str = "smart_home.list_bridges";
pub const SMART_HOME_DISCOVER_TOOL_ID: &str = "smart_home.discover";
pub const SMART_HOME_LIST_DEVICES_TOOL_ID: &str = "smart_home.list_devices";
pub const SMART_HOME_GET_STATE_TOOL_ID: &str = "smart_home.get_state";
pub const SMART_HOME_COMMAND_TOOL_ID: &str = "smart_home.command";
pub const SMART_HOME_SUBSCRIBE_TOOL_ID: &str = "smart_home.subscribe";
pub const SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID: &str = "smart_home.describe_capabilities";
pub const SMART_HOME_GET_HEALTH_TOOL_ID: &str = "smart_home.get_health";
pub const SMART_HOME_PAIR_BRIDGE_TOOL_ID: &str = "smart_home.pair_bridge";
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
                SMART_HOME_DISCOVER_TOOL_ID => {
                    let request = discover_request(&arguments)?;
                    let output = runtime
                        .execute_discover_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(discover_output_handler_output(output))
                }
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
                SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID => {
                    let entity_id = required_string(&arguments, "entity_id")?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::DescribeCapabilities {
                                entity_id: EntityId::trusted(entity_id),
                            },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "describe_capabilities"))
                }
                SMART_HOME_GET_HEALTH_TOOL_ID => {
                    let bridge_id = optional_string(&arguments, "bridge_id")?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetHealth {
                                bridge_id: bridge_id.map(BridgeId::trusted),
                            },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "get_health"))
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
                SMART_HOME_SUBSCRIBE_TOOL_ID => {
                    let request = subscribe_request(&arguments)?;
                    let output = runtime
                        .execute_subscribe_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(subscribe_output_handler_output(output))
                }
                SMART_HOME_PAIR_BRIDGE_TOOL_ID => {
                    let request = pair_bridge_request(&arguments)?;
                    let output = runtime
                        .execute_pair_bridge_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(pair_bridge_output_handler_output(output))
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
            SMART_HOME_DISCOVER_TOOL_ID,
            "Discover smart-home bridges",
            "List normalized D23 discovery candidates recorded by the smart-home runtime.",
            object_schema(
                vec![
                    SchemaProperty::new("integration_id", JsonSchema::String),
                    SchemaProperty::new("source", JsonSchema::String),
                    SchemaProperty::new("fresh_only", JsonSchema::Boolean),
                    SchemaProperty::new("ttl_ms", JsonSchema::Integer),
                    SchemaProperty::new("limit", JsonSchema::Integer),
                ],
                vec![],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new("generated_at_ms", JsonSchema::Integer),
                    SchemaProperty::new("ttl_ms", JsonSchema::Integer),
                    SchemaProperty::new(
                        "records",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new(
                        "bridge_candidates",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new("count", JsonSchema::Integer),
                    SchemaProperty::new("with_address_count", JsonSchema::Integer),
                    SchemaProperty::new("fresh_count", JsonSchema::Integer),
                    SchemaProperty::new("stale_count", JsonSchema::Integer),
                    SchemaProperty::new("expired_count", JsonSchema::Integer),
                    SchemaProperty::new("next_transition_at_ms", JsonSchema::Any),
                ],
                vec![
                    "generated_at_ms",
                    "ttl_ms",
                    "records",
                    "bridge_candidates",
                    "count",
                    "with_address_count",
                    "fresh_count",
                    "stale_count",
                    "expired_count",
                    "next_transition_at_ms",
                ],
                false,
            ),
        ),
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
        read_definition(
            SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID,
            "Describe smart-home capabilities",
            "Describe the normalized D23 capabilities exposed by one smart-home entity.",
            object_schema(
                vec![SchemaProperty::new("entity_id", JsonSchema::String)],
                vec!["entity_id"],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new("entity_id", JsonSchema::String),
                    SchemaProperty::new(
                        "capabilities",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new("count", JsonSchema::Integer),
                ],
                vec!["entity_id", "capabilities", "count"],
                false,
            ),
        ),
        read_definition(
            SMART_HOME_GET_HEALTH_TOOL_ID,
            "Get smart-home bridge health",
            "Read health snapshots for all bridges or one smart-home bridge.",
            object_schema(
                vec![SchemaProperty::new("bridge_id", JsonSchema::String)],
                vec![],
                false,
            ),
            collection_output_schema("bridges"),
        ),
        command_definition(),
        subscribe_definition(),
        pair_bridge_definition(),
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
                    SchemaProperty::new("discovery_worker_count", JsonSchema::Integer),
                    SchemaProperty::new("discovery_worker_run_count", JsonSchema::Integer),
                    SchemaProperty::new("unhealthy_discovery_worker_count", JsonSchema::Integer),
                    SchemaProperty::new(
                        "discovery_workers_with_failures_count",
                        JsonSchema::Integer,
                    ),
                    SchemaProperty::new("next_discovery_worker_due_at_ms", JsonSchema::Any),
                    SchemaProperty::new(
                        "discovery_workers",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
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
                    "discovery_worker_count",
                    "discovery_worker_run_count",
                    "unhealthy_discovery_worker_count",
                    "discovery_workers_with_failures_count",
                    "next_discovery_worker_due_at_ms",
                    "discovery_workers",
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

fn subscribe_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_SUBSCRIBE_TOOL_ID.to_string(),
        display_name: "Subscribe to smart-home events".to_string(),
        description:
            "Register a checkpointed subscription over normalized D23 smart-home runtime events."
                .to_string(),
        input_schema: object_schema(
            vec![
                SchemaProperty::new("subscription_id", JsonSchema::String),
                SchemaProperty::new("filter", JsonSchema::Any),
                SchemaProperty::new("from_checkpoint", JsonSchema::Integer),
            ],
            vec!["subscription_id"],
            false,
        ),
        output_schema: Some(object_schema(
            vec![
                SchemaProperty::new("subscription_id", JsonSchema::String),
                SchemaProperty::new("replay_from_checkpoint", JsonSchema::Integer),
                SchemaProperty::new("subscribed_at_checkpoint", JsonSchema::Integer),
                SchemaProperty::new("queued_events", JsonSchema::Integer),
            ],
            vec![
                "subscription_id",
                "replay_from_checkpoint",
                "subscribed_at_checkpoint",
                "queued_events",
            ],
            false,
        )),
        side_effects: ToolSideEffects::Read,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Safe,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier0,
        required_capabilities: vec!["smart_home:read".to_string()],
        preferred_lock_scope: None,
        timeout_seconds: Some(5),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "events".to_string(),
        ],
        stability: ToolStability::Experimental,
    }
}

fn pair_bridge_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_PAIR_BRIDGE_TOOL_ID.to_string(),
        display_name: "Pair smart-home bridge".to_string(),
        description:
            "Start a D23 bridge-pairing session; secrets remain in the smart-home vault path."
                .to_string(),
        input_schema: object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("bridge_id", JsonSchema::String),
                SchemaProperty::new("expires_at_ms", JsonSchema::Integer),
                SchemaProperty::new("metadata", JsonSchema::Any),
            ],
            vec!["session_id", "bridge_id", "expires_at_ms"],
            false,
        ),
        output_schema: Some(object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("bridge_id", JsonSchema::String),
                SchemaProperty::new("integration_id", JsonSchema::String),
                SchemaProperty::new("requested_by", JsonSchema::String),
                SchemaProperty::new("started_at_ms", JsonSchema::Integer),
                SchemaProperty::new("expires_at_ms", JsonSchema::Integer),
                SchemaProperty::new("status", JsonSchema::String),
                SchemaProperty::new("vault_ref", JsonSchema::Any),
                SchemaProperty::new("metadata", JsonSchema::Any),
            ],
            vec![
                "session_id",
                "bridge_id",
                "integration_id",
                "requested_by",
                "started_at_ms",
                "expires_at_ms",
                "status",
                "vault_ref",
                "metadata",
            ],
            false,
        )),
        side_effects: ToolSideEffects::External,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Serialized,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier2,
        required_capabilities: vec!["smart_home:pair".to_string()],
        preferred_lock_scope: Some("smart_home.pairing".to_string()),
        timeout_seconds: Some(30),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "pairing".to_string(),
        ],
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

fn discover_request(arguments: &JsonValue) -> Result<RuntimeDiscoverToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut request = RuntimeDiscoverToolRequest::new();
    if let Some(integration_id) = optional_string(arguments, "integration_id")? {
        request = request.for_integration(IntegrationId::trusted(integration_id));
    }
    if let Some(source) = optional_string(arguments, "source")? {
        request = request.from_source(parse_discovery_source(&source)?);
    }
    if let Some(fresh_only) = optional_bool(arguments, "fresh_only")? {
        request = request.fresh_only(fresh_only);
    }
    if let Some(ttl_ms) = optional_u64(arguments, "ttl_ms")? {
        request = request.with_ttl_ms(ttl_ms);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        request = request.with_limit(limit as usize);
    }
    Ok(request)
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

fn subscribe_request(arguments: &JsonValue) -> Result<RuntimeSubscribeToolRequest, ToolCallError> {
    let subscription_id =
        RuntimeSubscriptionId::trusted(required_string(arguments, "subscription_id")?);
    let filter = optional_field(arguments, "filter")
        .map(parse_event_filter)
        .transpose()?
        .unwrap_or(RuntimeEventFilter::All);
    let mut request = RuntimeSubscribeToolRequest::new(subscription_id, filter);
    if let Some(from_checkpoint) = optional_u64(arguments, "from_checkpoint")? {
        request =
            request.with_checkpoint(RuntimeEventCheckpoint::from_next_sequence(from_checkpoint));
    }
    Ok(request)
}

fn pair_bridge_request(
    arguments: &JsonValue,
) -> Result<RuntimePairBridgeToolRequest, ToolCallError> {
    let mut request = RuntimePairBridgeToolRequest::new(
        RuntimePairingSessionId::trusted(required_string(arguments, "session_id")?),
        BridgeId::trusted(required_string(arguments, "bridge_id")?),
        required_u64(arguments, "expires_at_ms")?,
    );
    let metadata = optional_metadata(arguments)?;
    if !metadata.is_empty() {
        request = request.with_metadata(metadata);
    }
    Ok(request)
}

fn discover_output_handler_output(output: RuntimeDiscoverToolOutput) -> ToolHandlerOutput {
    ToolHandlerOutput::new(discover_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("discover")),
            ("count", integer(output.len() as i64)),
            ("fresh_count", integer(output.record_summary.fresh as i64)),
        ]),
    )
}

fn subscribe_output_handler_output(output: RuntimeSubscribeToolOutput) -> ToolHandlerOutput {
    ToolHandlerOutput::new(subscribe_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("subscribe")),
            ("subscription_id", string(output.subscription_id.as_str())),
            ("queued_events", integer(output.queued_events as i64)),
        ]),
    )
}

fn pair_bridge_output_handler_output(output: RuntimePairBridgeToolOutput) -> ToolHandlerOutput {
    ToolHandlerOutput::new(pair_bridge_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("pair_bridge")),
            ("session_id", string(output.session.session_id.as_str())),
            (
                "status",
                string(pairing_status_label(output.session.status)),
            ),
        ]),
    )
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
                "discovery_worker_count",
                integer(observation.discovery_worker_count() as i64),
            ),
            (
                "discovery_worker_run_count",
                integer(observation.discovery_worker_run_count() as i64),
            ),
            (
                "unhealthy_discovery_worker_count",
                integer(observation.unhealthy_discovery_worker_count() as i64),
            ),
            (
                "discovery_workers_with_failures_count",
                integer(observation.discovery_workers_with_failures_count() as i64),
            ),
            (
                "next_discovery_worker_due_at_ms",
                observation
                    .next_discovery_worker_due_at_ms()
                    .map(|value| integer(value as i64))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "discovery_workers",
                JsonValue::Array(
                    observation
                        .discovery_workers
                        .iter()
                        .map(discovery_worker_snapshot_json)
                        .collect(),
                ),
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

fn subscribe_output_json(output: &RuntimeSubscribeToolOutput) -> JsonValue {
    object([
        ("subscription_id", string(output.subscription_id.as_str())),
        (
            "replay_from_checkpoint",
            integer(output.replay_from_checkpoint.next_sequence() as i64),
        ),
        (
            "subscribed_at_checkpoint",
            integer(output.subscribed_at_checkpoint.next_sequence() as i64),
        ),
        ("queued_events", integer(output.queued_events as i64)),
    ])
}

fn pair_bridge_output_json(output: &RuntimePairBridgeToolOutput) -> JsonValue {
    pairing_session_json(&output.session)
}

fn discover_output_json(output: &RuntimeDiscoverToolOutput) -> JsonValue {
    object([
        ("generated_at_ms", integer(output.generated_at_ms as i64)),
        ("ttl_ms", integer(output.ttl_ms as i64)),
        (
            "records",
            JsonValue::Array(output.records.iter().map(discovery_record_json).collect()),
        ),
        (
            "bridge_candidates",
            JsonValue::Array(output.bridge_candidates.iter().map(bridge_json).collect()),
        ),
        ("count", integer(output.record_summary.total as i64)),
        (
            "with_address_count",
            integer(output.record_summary.with_address as i64),
        ),
        ("fresh_count", integer(output.record_summary.fresh as i64)),
        ("stale_count", integer(output.record_summary.stale as i64)),
        (
            "expired_count",
            integer(output.record_summary.expired as i64),
        ),
        (
            "next_transition_at_ms",
            output
                .signal_summary
                .next_transition_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn discovery_record_json(record: &DiscoveryRecord) -> JsonValue {
    object([
        ("fingerprint", string(record.fingerprint().as_str())),
        ("bridge_id", string(record.bridge_id().as_str())),
        ("integration_id", string(record.integration_id.as_str())),
        ("native_bridge_id", string(&record.native_bridge_id)),
        ("display_name", optional_string_json(&record.display_name)),
        ("source", string(record.source.as_str())),
        ("transport", string(record.transport.as_str())),
        ("address", optional_string_json(&record.address)),
        (
            "network_interface",
            optional_string_json(&record.network_interface),
        ),
        (
            "hardware_model",
            optional_string_json(&record.hardware_model),
        ),
        (
            "firmware_version",
            optional_string_json(&record.firmware_version),
        ),
        ("confidence", string(record.confidence.as_str())),
        (
            "pairing_requirement",
            string(record.pairing_requirement.as_str()),
        ),
        ("discovered_at_ms", integer(record.discovered_at_ms as i64)),
        (
            "expires_at_ms",
            record
                .expires_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "metadata",
            JsonValue::Array(record.metadata.iter().map(metadata_json).collect()),
        ),
    ])
}

fn pairing_session_json(session: &RuntimePairingSession) -> JsonValue {
    object([
        ("session_id", string(session.session_id.as_str())),
        ("bridge_id", string(session.bridge_id.as_str())),
        ("integration_id", string(session.integration_id.as_str())),
        ("requested_by", string(session.requested_by.as_str())),
        ("started_at_ms", integer(session.started_at_ms as i64)),
        ("expires_at_ms", integer(session.expires_at_ms as i64)),
        ("status", string(pairing_status_label(session.status))),
        (
            "vault_ref",
            session
                .vault_ref
                .as_ref()
                .map(|vault_ref| string(vault_ref.as_str()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "metadata",
            JsonValue::Array(session.metadata.iter().map(metadata_json).collect()),
        ),
    ])
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

fn metadata_json(metadata: &Metadata) -> JsonValue {
    object([
        ("key", string(&metadata.key)),
        ("value", string(&metadata.value)),
    ])
}

fn discovery_worker_snapshot_json(snapshot: &ScheduledDiscoveryWorkerSnapshot) -> JsonValue {
    object([
        ("worker_id", string(snapshot.worker_id.as_str())),
        ("integration_id", string(snapshot.integration_id.as_str())),
        ("kind", string(snapshot.kind.as_str())),
        ("status", string(snapshot.status.as_str())),
        (
            "sources",
            JsonValue::Array(
                snapshot
                    .sources
                    .iter()
                    .map(|source| string(source.as_str()))
                    .collect(),
            ),
        ),
        (
            "network_interfaces",
            JsonValue::Array(snapshot.network_interfaces.iter().map(string).collect()),
        ),
        ("is_due", JsonValue::Bool(snapshot.is_due)),
        ("overdue_by_ms", integer(snapshot.overdue_by_ms as i64)),
        ("next_due_at_ms", integer(snapshot.next_due_at_ms as i64)),
        ("interval_ms", integer(snapshot.interval_ms as i64)),
        ("run_timeout_ms", integer(snapshot.run_timeout_ms as i64)),
        ("retry_delay_ms", integer(snapshot.retry_delay_ms as i64)),
        (
            "max_retry_delay_ms",
            integer(snapshot.max_retry_delay_ms as i64),
        ),
        (
            "retry_backoff_multiplier",
            integer(snapshot.retry_backoff_multiplier as i64),
        ),
        (
            "current_retry_delay_ms",
            snapshot
                .current_retry_delay_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "last_started_at_ms",
            snapshot
                .last_started_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "last_completed_at_ms",
            snapshot
                .last_completed_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "last_run_status",
            snapshot
                .last_run_status
                .map(|status| string(status.as_str()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "last_record_count",
            integer(snapshot.last_record_count as i64),
        ),
        (
            "last_failure_count",
            integer(snapshot.last_failure_count as i64),
        ),
        (
            "last_catalog_change_count",
            integer(snapshot.last_catalog_change_count as i64),
        ),
        ("total_run_count", integer(snapshot.total_run_count as i64)),
        (
            "consecutive_failure_count",
            integer(snapshot.consecutive_failure_count as i64),
        ),
        (
            "has_failure_pressure",
            JsonValue::Bool(snapshot.has_failure_pressure()),
        ),
        (
            "metadata",
            JsonValue::Array(snapshot.metadata.iter().map(metadata_json).collect()),
        ),
    ])
}

fn optional_string_json(value: &Option<String>) -> JsonValue {
    value.as_ref().map(string).unwrap_or(JsonValue::Null)
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

fn parse_event_filter(value: &JsonValue) -> Result<RuntimeEventFilter, ToolCallError> {
    match value {
        JsonValue::Null => Ok(RuntimeEventFilter::All),
        JsonValue::String(label) => parse_event_filter_label(label, None, None),
        JsonValue::Object(_) => parse_event_filter_object(value),
        _ => Err(validation_error("filter must be a string or object")),
    }
}

fn parse_event_filter_object(value: &JsonValue) -> Result<RuntimeEventFilter, ToolCallError> {
    reject_unsupported_filter_field(value, "device_id")?;
    reject_unsupported_filter_field(value, "device_ids")?;
    reject_unsupported_filter_field(value, "capability_id")?;
    reject_unsupported_filter_field(value, "capability_ids")?;

    let filter_type = optional_string(value, "filter_type")?
        .or(optional_string(value, "kind")?)
        .or(optional_string(value, "type")?);
    let bridge_id = optional_single_filter_string(value, "bridge_id", "bridge_ids")?;
    let entity_id = optional_single_filter_string(value, "entity_id", "entity_ids")?;

    if let Some(filter_type) = filter_type {
        return parse_event_filter_label(&filter_type, bridge_id, entity_id);
    }
    if let Some(entity_id) = entity_id {
        return Ok(RuntimeEventFilter::Entity(EntityId::trusted(entity_id)));
    }
    if let Some(bridge_id) = bridge_id {
        return Ok(RuntimeEventFilter::Bridge(BridgeId::trusted(bridge_id)));
    }
    Ok(RuntimeEventFilter::All)
}

fn parse_event_filter_label(
    label: &str,
    bridge_id: Option<String>,
    entity_id: Option<String>,
) -> Result<RuntimeEventFilter, ToolCallError> {
    match label {
        "all" => Ok(RuntimeEventFilter::All),
        "bridge" => bridge_id
            .map(|bridge_id| RuntimeEventFilter::Bridge(BridgeId::trusted(bridge_id)))
            .ok_or_else(|| validation_error("bridge filter requires bridge_id")),
        "entity" => entity_id
            .map(|entity_id| RuntimeEventFilter::Entity(EntityId::trusted(entity_id)))
            .ok_or_else(|| validation_error("entity filter requires entity_id")),
        "commands" | "command_results" => Ok(RuntimeEventFilter::Commands),
        "supervision" => Ok(RuntimeEventFilter::Supervision),
        _ => Err(validation_error(format!("unknown event filter `{label}`"))),
    }
}

fn optional_single_filter_string(
    value: &JsonValue,
    singular_field: &str,
    plural_field: &str,
) -> Result<Option<String>, ToolCallError> {
    if let Some(value) = optional_string(value, singular_field)? {
        return Ok(Some(value));
    }
    match optional_field(value, plural_field) {
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(JsonValue::Array(values)) => match values.as_slice() {
            [] => Ok(None),
            [JsonValue::String(value)] => Ok(Some(value.clone())),
            [_] => Err(validation_error(format!(
                "{plural_field} must contain string values"
            ))),
            _ => Err(validation_error(format!(
                "{plural_field} supports one value in this runtime slice"
            ))),
        },
        Some(JsonValue::Null) | None => Ok(None),
        Some(_) => Err(validation_error(format!(
            "{plural_field} must be a string array"
        ))),
    }
}

fn reject_unsupported_filter_field(value: &JsonValue, field: &str) -> Result<(), ToolCallError> {
    if optional_field(value, field).is_some() {
        return Err(validation_error(format!(
            "{field} filters are not supported by this runtime slice"
        )));
    }
    Ok(())
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

fn parse_discovery_source(label: &str) -> Result<DiscoverySource, ToolCallError> {
    match label {
        "mdns" => Ok(DiscoverySource::Mdns),
        "ssdp" => Ok(DiscoverySource::Ssdp),
        "bluetooth" => Ok(DiscoverySource::Bluetooth),
        "usb" => Ok(DiscoverySource::Usb),
        "dhcp" => Ok(DiscoverySource::Dhcp),
        "mqtt" => Ok(DiscoverySource::Mqtt),
        "manual" => Ok(DiscoverySource::Manual),
        "cloud_fallback" => Ok(DiscoverySource::CloudFallback),
        "webhook" => Ok(DiscoverySource::Webhook),
        "simulator" => Ok(DiscoverySource::Simulator),
        _ => Err(validation_error(format!(
            "unknown discovery source `{label}`"
        ))),
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

fn pairing_status_label(status: PairingSessionStatus) -> &'static str {
    status.as_str()
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
        | RuntimeError::DuplicateSubscription(_)
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

fn required_u64(value: &JsonValue, field: &str) -> Result<u64, ToolCallError> {
    optional_u64(value, field)?.ok_or_else(|| validation_error(format!("{field} is required")))
}

fn optional_string(value: &JsonValue, field: &str) -> Result<Option<String>, ToolCallError> {
    match optional_field(value, field) {
        Some(JsonValue::String(value)) => Ok(Some(value.clone())),
        Some(JsonValue::Null) | None => Ok(None),
        Some(_) => Err(validation_error(format!("{field} must be a string"))),
    }
}

fn optional_bool(value: &JsonValue, field: &str) -> Result<Option<bool>, ToolCallError> {
    match optional_field(value, field) {
        Some(JsonValue::Bool(value)) => Ok(Some(*value)),
        Some(JsonValue::Null) | None => Ok(None),
        Some(_) => Err(validation_error(format!("{field} must be a boolean"))),
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

fn optional_metadata(value: &JsonValue) -> Result<Vec<Metadata>, ToolCallError> {
    match optional_field(value, "metadata") {
        Some(JsonValue::Object(fields)) => fields
            .iter()
            .map(|(key, value)| Ok(Metadata::new(key.clone(), metadata_scalar(value)?)))
            .collect(),
        Some(JsonValue::Array(entries)) => {
            let mut metadata = Vec::new();
            for entry in entries {
                metadata.push(metadata_entry(entry)?);
            }
            Ok(metadata)
        }
        Some(JsonValue::Null) | None => Ok(Vec::new()),
        Some(_) => Err(validation_error("metadata must be an object or array")),
    }
}

fn metadata_entry(value: &JsonValue) -> Result<Metadata, ToolCallError> {
    Ok(Metadata::new(
        required_string(value, "key")?,
        required_string(value, "value")?,
    ))
}

fn metadata_scalar(value: &JsonValue) -> Result<String, ToolCallError> {
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        JsonValue::Bool(value) => Ok(value.to_string()),
        JsonValue::Number(JsonNumber::Integer(value)) => Ok(value.to_string()),
        JsonValue::Number(JsonNumber::Float(value)) if value.is_finite() => Ok(value.to_string()),
        JsonValue::Number(JsonNumber::Float(_)) => {
            Err(validation_error("metadata number must be finite"))
        }
        JsonValue::Null | JsonValue::Array(_) | JsonValue::Object(_) => {
            Err(validation_error("metadata values must be scalar"))
        }
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
    use smart_home_discovery::{
        DiscoveryWorkerId, DiscoveryWorkerKind, MDNS_DISCOVERY_SERVICE_TYPE_METADATA_KEY,
    };
    use smart_home_runtime::ScheduledDiscoveryWorker;
    use smart_home_testkit::{hue_bridge_discovery_record, hue_lighting_runtime};

    const AGENT_ID: &str = "agent:chief-smart-home";

    #[test]
    fn smart_home_tool_definitions_are_valid() {
        let definitions = smart_home_tool_definitions();
        let export = ToolCatalogExport::from_definitions(definitions.iter());

        assert_eq!(definitions.len(), 10);
        assert!(export.ok());
        assert!(export.tool_ids().contains(&SMART_HOME_DISCOVER_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_COMMAND_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_PAIR_BRIDGE_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_SUBSCRIBE_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_GET_HEALTH_TOOL_ID));
        assert_eq!(
            export.summary.required_capability_count("smart_home:read"),
            8
        );
        assert_eq!(
            export
                .summary
                .required_capability_count("smart_home:command"),
            1
        );
        assert_eq!(
            export.summary.required_capability_count("smart_home:pair"),
            1
        );
        assert!(smart_home_tool_definition(SMART_HOME_GET_STATE_TOOL_ID).is_some());
        assert!(smart_home_tool_definition("smart_home.unknown").is_none());
    }

    #[test]
    fn chief_of_staff_runtime_drives_smart_home_light_end_to_end() {
        let runtime = Rc::new(RefCell::new(hue_lighting_runtime()));
        runtime
            .borrow_mut()
            .record_discovery(hue_bridge_discovery_record("001788fffediscovered", 1_000))
            .unwrap();
        runtime
            .borrow_mut()
            .register_discovery_worker_schedule(
                ScheduledDiscoveryWorker::new(
                    DiscoveryWorkerId::trusted("hue-mdns-worker"),
                    IntegrationId::trusted("hue"),
                    DiscoveryWorkerKind::MdnsScan,
                    5_000,
                    250,
                    1_050,
                )
                .with_source(DiscoverySource::Mdns)
                .with_network_interface("en0")
                .with_metadata(MDNS_DISCOVERY_SERVICE_TYPE_METADATA_KEY, "_hue._tcp.local"),
            )
            .unwrap();
        runtime.borrow_mut().registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-smart-home"),
                AgentId::trusted(AGENT_ID),
                PrivilegeTier::HumanApproval,
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

        let discover_request = request(
            "call-discover",
            SMART_HOME_DISCOVER_TOOL_ID,
            object([
                ("integration_id", string("hue")),
                ("source", string("mdns")),
                ("fresh_only", JsonValue::Bool(true)),
                ("ttl_ms", integer(1_000)),
            ]),
            1_005,
        );
        let discover_trace = tool_runtime.invoke_with_events(&discover_request);
        assert!(discover_trace.result.ok);
        assert_eq!(
            field(discover_trace.result.output.as_ref().unwrap(), "count"),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                discover_trace.result.output.as_ref().unwrap(),
                "with_address_count"
            ),
            Some(&integer(1))
        );
        assert_eq!(
            array_len(
                field(
                    discover_trace.result.output.as_ref().unwrap(),
                    "bridge_candidates"
                )
                .unwrap()
            ),
            Some(1)
        );

        let capabilities_request = request(
            "call-describe-capabilities",
            SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID,
            object([("entity_id", string("entity-light-1"))]),
            1_010,
        );
        let capabilities_trace = tool_runtime.invoke_with_events(&capabilities_request);
        assert!(capabilities_trace.result.ok);
        assert_eq!(
            field(capabilities_trace.result.output.as_ref().unwrap(), "count"),
            Some(&integer(3))
        );

        let health_request = request(
            "call-get-health",
            SMART_HOME_GET_HEALTH_TOOL_ID,
            object([("bridge_id", string("bridge-1"))]),
            1_020,
        );
        let health_trace = tool_runtime.invoke_with_events(&health_request);
        assert!(health_trace.result.ok);
        assert_eq!(
            field(health_trace.result.output.as_ref().unwrap(), "count"),
            Some(&integer(1))
        );

        let supervision_request = request(
            "call-observe-supervision",
            SMART_HOME_OBSERVE_SUPERVISION_TOOL_ID,
            object([]),
            1_060,
        );
        let supervision_trace = tool_runtime.invoke_with_events(&supervision_request);
        assert!(supervision_trace.result.ok);
        let supervision_output = supervision_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(supervision_output, "discovery_worker_count"),
            Some(&integer(1))
        );
        assert_eq!(
            field(supervision_output, "discovery_worker_run_count"),
            Some(&integer(1))
        );
        assert_eq!(
            field(supervision_output, "unhealthy_discovery_worker_count"),
            Some(&integer(0))
        );
        assert_eq!(
            field(supervision_output, "next_discovery_worker_due_at_ms"),
            Some(&integer(1_050))
        );
        assert_eq!(
            array_len(field(supervision_output, "discovery_workers").unwrap()),
            Some(1)
        );
        let discovery_worker_output =
            array_item(field(supervision_output, "discovery_workers").unwrap(), 0).unwrap();
        assert_eq!(
            field(discovery_worker_output, "retry_delay_ms"),
            Some(&integer(5_000))
        );
        assert_eq!(
            field(discovery_worker_output, "max_retry_delay_ms"),
            Some(&integer(5_000))
        );
        assert_eq!(
            field(discovery_worker_output, "retry_backoff_multiplier"),
            Some(&integer(1))
        );
        assert_eq!(
            field(discovery_worker_output, "current_retry_delay_ms"),
            Some(&JsonValue::Null)
        );

        let subscribe_request = request(
            "call-subscribe",
            SMART_HOME_SUBSCRIBE_TOOL_ID,
            object([
                ("subscription_id", string("commands")),
                ("filter", object([("filter_type", string("commands"))])),
            ]),
            1_030,
        );
        let subscribe_trace = tool_runtime.invoke_with_events(&subscribe_request);
        assert!(subscribe_trace.result.ok);
        assert_eq!(
            field(
                subscribe_trace.result.output.as_ref().unwrap(),
                "queued_events"
            ),
            Some(&integer(0))
        );

        let pair_request = request(
            "call-pair",
            SMART_HOME_PAIR_BRIDGE_TOOL_ID,
            object([
                ("session_id", string("pairing-session-1")),
                ("bridge_id", string("bridge-1")),
                ("expires_at_ms", integer(2_000)),
                (
                    "metadata",
                    object([("initiated_by", string("chief-of-staff-test"))]),
                ),
            ]),
            1_040,
        );
        let pair_trace = tool_runtime.invoke_with_events(&pair_request);
        assert!(pair_trace.result.ok);
        assert_eq!(
            field(pair_trace.result.output.as_ref().unwrap(), "status"),
            Some(&string("pending_user_presence"))
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
        journal.record_trace(discover_request, discover_trace);
        journal.record_trace(capabilities_request, capabilities_trace);
        journal.record_trace(health_request, health_trace);
        journal.record_trace(supervision_request, supervision_trace);
        journal.record_trace(subscribe_request, subscribe_trace);
        journal.record_trace(pair_request, pair_trace);
        journal.record_trace(command_request, command_trace);
        journal.record_trace(state_request, state_trace);

        let journal_summary = journal.summary();
        assert_eq!(journal_summary.invocation_count, 9);
        assert_eq!(journal_summary.completed_count, 9);
        assert_eq!(journal.audit_records().len(), 9);

        let runtime = runtime.borrow();
        assert_eq!(runtime.optimistic_state_count(), 1);
        assert_eq!(runtime.pairing_session_count(), 1);
        assert_eq!(
            runtime
                .event_bus()
                .queued_events(&RuntimeSubscriptionId::trusted("commands"))
                .unwrap(),
            1
        );
        assert_eq!(
            runtime.registry().counts().authorization_decisions,
            10,
            "read, subscribe, and pair calls record tool authorization, while command records tool and command authorization"
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

        let denied_pair = tool_runtime.invoke(&request(
            "call-pair-denied",
            SMART_HOME_PAIR_BRIDGE_TOOL_ID,
            object([
                ("session_id", string("pairing-session-1")),
                ("bridge_id", string("bridge-1")),
                ("expires_at_ms", integer(2_000)),
            ]),
            1_000,
        ));

        assert!(!denied_pair.ok);
        assert_eq!(
            denied_pair.error.as_ref().map(|error| error.kind),
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

        let unsupported_filter = tool_runtime.invoke_with_events(&request(
            "call-invalid-subscribe",
            SMART_HOME_SUBSCRIBE_TOOL_ID,
            object([
                ("subscription_id", string("sub-1")),
                (
                    "filter",
                    object([("device_ids", JsonValue::Array(vec![string("device-1")]))]),
                ),
            ]),
            1_000,
        ));
        assert!(!unsupported_filter.result.ok);
        assert_eq!(
            unsupported_filter
                .result
                .error
                .as_ref()
                .map(|error| error.kind),
            Some(ToolErrorKind::ToolValidationError)
        );

        let unsupported_source = tool_runtime.invoke_with_events(&request(
            "call-invalid-discover",
            SMART_HOME_DISCOVER_TOOL_ID,
            object([("source", string("radio_magic"))]),
            1_000,
        ));
        assert!(!unsupported_source.result.ok);
        assert_eq!(
            unsupported_source
                .result
                .error
                .as_ref()
                .map(|error| error.kind),
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

    fn array_len(value: &JsonValue) -> Option<usize> {
        let JsonValue::Array(values) = value else {
            return None;
        };
        Some(values.len())
    }

    fn array_item(value: &JsonValue, index: usize) -> Option<&JsonValue> {
        let JsonValue::Array(values) = value else {
            return None;
        };
        values.get(index)
    }
}
