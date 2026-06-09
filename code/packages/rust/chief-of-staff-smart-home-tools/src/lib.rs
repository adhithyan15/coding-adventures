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
    AgentId, AuthorizationDecision, AuthorizationDecisionLogSummary, AuthorizationOutcome,
    AuthorizationSubject, Bridge, BridgeId, Capability, CapabilityGrant,
    CapabilityGrantInventorySummary, CapabilityGrantScope, CapabilityGrantStatus, CapabilityId,
    CommandResult, CommandStatus, CommandType, CorrelationId, Device, DeviceCommand, DeviceEvent,
    DeviceEventType, DeviceId, EntityId, EntityKind, EventId, Health, IntegrationId, Metadata,
    PrivilegeTier, ProtocolFamily, ProtocolIdentifier, RuntimeKind, Scene, SceneAction, SceneId,
    SceneScope, StateConfidence, StateDelta, StateSnapshot, StateSource, Value, VaultRef,
};
use smart_home_discovery::{
    DiscoveryPairingAction, DiscoveryPairingPlan, DiscoveryPairingPlanOptions,
    DiscoveryPairingPlanSort, DiscoveryPairingPlanSummary, DiscoveryPairingTarget, DiscoveryRecord,
    DiscoveryRecordSummary, DiscoverySignalStatus, DiscoverySignalSummary, DiscoverySource,
    DiscoveryWorkerId, DiscoveryWorkerKind, PairingRequirement,
};
use smart_home_integration_catalog::{
    activation_plan_for_entry, describe_primitive_family, ecosystem_platforms_requiring_primitive,
    ecosystem_survey_sources, entries_requiring_primitive, find_entry, first_party_catalog,
    primitive_backlog_at_or_before_priority, primitive_backlog_with_ecosystem_coverage,
    primitive_family_descriptors, query_integrations, readiness_report_for_plan,
    survey_sources_requiring_primitive, AuthMode, ConnectivityClass, DiscoveryMechanism,
    EcosystemSurveySource, ImplementationStatus, IntegrationActivationPlan,
    IntegrationActivationTarget, IntegrationCatalogEntry, IntegrationCatalogQuery,
    IntegrationCatalogSort, IntegrationCategory, IntegrationPolicySurface,
    IntegrationReadinessReport, PrimitiveBacklogCoverageItem, PrimitiveBacklogItem,
    PrimitiveFamily, PrimitiveFamilyDescriptor, SourceReference,
};
use smart_home_registry::RegistryTopologySummary;
use smart_home_registry::StateRefreshReason;
use smart_home_runtime::{
    BridgeHealthReport, DesiredEntityState, DesiredStateAction, DesiredStateInventorySummary,
    DesiredStateQuery, DesiredStateSort, DiscoveryWorkerQuery, DiscoveryWorkerRunInstruction,
    DiscoveryWorkerSchedulerSnapshot, DiscoveryWorkerSort, PairingSessionStatus,
    ReconciliationReason, RuntimeAuthorizationDecisionQuery, RuntimeAuthorizationDecisionSort,
    RuntimeCapabilityGrantQuery, RuntimeCapabilityGrantScopeKind, RuntimeCapabilityGrantSort,
    RuntimeClearDesiredStateToolOutput, RuntimeClearDesiredStateToolRequest,
    RuntimeCommandToolRequest, RuntimeCompletePairingToolOutput, RuntimeCompletePairingToolRequest,
    RuntimeDiscoverToolOutput, RuntimeDiscoverToolRequest, RuntimeError, RuntimeEvent,
    RuntimeEventCheckpoint, RuntimeEventDeliveryBatch, RuntimeEventFilter, RuntimeEventLogRecord,
    RuntimeEventLogSummary, RuntimeEventQuery, RuntimeEventSort, RuntimePairBridgeToolOutput,
    RuntimePairBridgeToolRequest, RuntimePairingPlanToolRequest, RuntimePairingSession,
    RuntimePairingSessionId, RuntimePairingSessionInventorySummary, RuntimePairingSessionQuery,
    RuntimePairingSessionSort, RuntimePendingWorkSummary, RuntimePollEventsToolOutput,
    RuntimePollEventsToolRequest, RuntimeReadSnapshot, RuntimeReadToolOutput,
    RuntimeReadToolRequest, RuntimeReportEventToolOutput, RuntimeReportEventToolRequest,
    RuntimeRoomQuery, RuntimeRoomSort, RuntimeRoomSummary, RuntimeSetDesiredStateToolOutput,
    RuntimeSetDesiredStateToolRequest, RuntimeSubscribeToolOutput, RuntimeSubscribeToolRequest,
    RuntimeSubscriptionBacklogStatus, RuntimeSubscriptionId, RuntimeSubscriptionInventorySummary,
    RuntimeSubscriptionQuery, RuntimeSubscriptionSnapshot, RuntimeSubscriptionSort,
    RuntimeSupervisionPlan, RuntimeSupervisionPlanSummary, RuntimeSupervisionToolOutput,
    RuntimeSupervisionToolRequest, RuntimeSupervisorSnapshot, RuntimeUnsubscribeToolOutput,
    RuntimeUnsubscribeToolRequest, ScheduledDiscoveryWorkerSnapshot, SmartHomeRuntime,
    SupervisedBridgeWorker, SupervisedWorkerQuery, SupervisedWorkerSort, SupervisionTickReport,
    WorkerHeartbeatDeadline, WorkerHeartbeatSchedule, WorkerRestartInstruction,
    WorkerRestartReason, WorkerStatus,
};
use std::cell::RefCell;
use std::rc::Rc;

pub const SMART_HOME_LIST_BRIDGES_TOOL_ID: &str = "smart_home.list_bridges";
pub const SMART_HOME_DISCOVER_TOOL_ID: &str = "smart_home.discover";
pub const SMART_HOME_LIST_DISCOVERY_WORKERS_TOOL_ID: &str = "smart_home.list_discovery_workers";
pub const SMART_HOME_GET_DISCOVERY_SUMMARY_TOOL_ID: &str = "smart_home.get_discovery_summary";
pub const SMART_HOME_GET_PAIRING_PLAN_TOOL_ID: &str = "smart_home.get_pairing_plan";
pub const SMART_HOME_LIST_DEVICES_TOOL_ID: &str = "smart_home.list_devices";
pub const SMART_HOME_LIST_ROOMS_TOOL_ID: &str = "smart_home.list_rooms";
pub const SMART_HOME_LIST_SCENES_TOOL_ID: &str = "smart_home.list_scenes";
pub const SMART_HOME_DESCRIBE_SCENE_TOOL_ID: &str = "smart_home.describe_scene";
pub const SMART_HOME_GET_STATE_TOOL_ID: &str = "smart_home.get_state";
pub const SMART_HOME_COMMAND_TOOL_ID: &str = "smart_home.command";
pub const SMART_HOME_REPORT_EVENT_TOOL_ID: &str = "smart_home.report_event";
pub const SMART_HOME_SUBSCRIBE_TOOL_ID: &str = "smart_home.subscribe";
pub const SMART_HOME_POLL_EVENTS_TOOL_ID: &str = "smart_home.poll_events";
pub const SMART_HOME_UNSUBSCRIBE_TOOL_ID: &str = "smart_home.unsubscribe";
pub const SMART_HOME_LIST_SUBSCRIPTIONS_TOOL_ID: &str = "smart_home.list_subscriptions";
pub const SMART_HOME_INSPECT_EVENT_LOG_TOOL_ID: &str = "smart_home.inspect_event_log";
pub const SMART_HOME_LIST_AUTHORIZATION_DECISIONS_TOOL_ID: &str =
    "smart_home.list_authorization_decisions";
pub const SMART_HOME_GET_AUTHORIZATION_SUMMARY_TOOL_ID: &str =
    "smart_home.get_authorization_summary";
pub const SMART_HOME_LIST_CAPABILITY_GRANTS_TOOL_ID: &str = "smart_home.list_capability_grants";
pub const SMART_HOME_GET_CAPABILITY_GRANT_SUMMARY_TOOL_ID: &str =
    "smart_home.get_capability_grant_summary";
pub const SMART_HOME_GET_RUNTIME_SNAPSHOT_TOOL_ID: &str = "smart_home.get_runtime_snapshot";
pub const SMART_HOME_GET_TOPOLOGY_SUMMARY_TOOL_ID: &str = "smart_home.get_topology_summary";
pub const SMART_HOME_LIST_DESIRED_STATES_TOOL_ID: &str = "smart_home.list_desired_states";
pub const SMART_HOME_SET_DESIRED_STATE_TOOL_ID: &str = "smart_home.set_desired_state";
pub const SMART_HOME_CLEAR_DESIRED_STATE_TOOL_ID: &str = "smart_home.clear_desired_state";
pub const SMART_HOME_LIST_PAIRING_SESSIONS_TOOL_ID: &str = "smart_home.list_pairing_sessions";
pub const SMART_HOME_LIST_WORKERS_TOOL_ID: &str = "smart_home.list_workers";
pub const SMART_HOME_GET_WORKER_HEARTBEAT_SCHEDULE_TOOL_ID: &str =
    "smart_home.get_worker_heartbeat_schedule";
pub const SMART_HOME_GET_SUPERVISION_PLAN_TOOL_ID: &str = "smart_home.get_supervision_plan";
pub const SMART_HOME_RECONCILE_DESIRED_STATES_TOOL_ID: &str = "smart_home.reconcile_desired_states";
pub const SMART_HOME_RUN_SUPERVISION_TICK_TOOL_ID: &str = "smart_home.run_supervision_tick";
pub const SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID: &str = "smart_home.describe_capabilities";
pub const SMART_HOME_GET_HEALTH_TOOL_ID: &str = "smart_home.get_health";
pub const SMART_HOME_PAIR_BRIDGE_TOOL_ID: &str = "smart_home.pair_bridge";
pub const SMART_HOME_COMPLETE_PAIRING_TOOL_ID: &str = "smart_home.complete_pairing";
pub const SMART_HOME_OBSERVE_SUPERVISION_TOOL_ID: &str = "smart_home.observe_supervision";
pub const SMART_HOME_LIST_INTEGRATIONS_TOOL_ID: &str = "smart_home.list_integrations";
pub const SMART_HOME_DESCRIBE_INTEGRATION_TOOL_ID: &str = "smart_home.describe_integration";
pub const SMART_HOME_LIST_PRIMITIVES_TOOL_ID: &str = "smart_home.list_primitives";
pub const SMART_HOME_DESCRIBE_PRIMITIVE_TOOL_ID: &str = "smart_home.describe_primitive";

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
                SMART_HOME_LIST_INTEGRATIONS_TOOL_ID => {
                    let request = integration_catalog_query(&arguments)?;
                    Ok(list_integrations_output_handler_output(request))
                }
                SMART_HOME_DESCRIBE_INTEGRATION_TOOL_ID => {
                    Ok(describe_integration_output_handler_output(&arguments)?)
                }
                SMART_HOME_LIST_PRIMITIVES_TOOL_ID => {
                    Ok(list_primitives_output_handler_output(&arguments)?)
                }
                SMART_HOME_DESCRIBE_PRIMITIVE_TOOL_ID => {
                    Ok(describe_primitive_output_handler_output(&arguments)?)
                }
                SMART_HOME_DISCOVER_TOOL_ID => {
                    let request = discover_request(&arguments)?;
                    let output = runtime
                        .execute_discover_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(discover_output_handler_output(output))
                }
                SMART_HOME_LIST_DISCOVERY_WORKERS_TOOL_ID => {
                    let query = discovery_worker_query(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::ListDiscoveryWorkers { query },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_discovery_workers"))
                }
                SMART_HOME_GET_DISCOVERY_SUMMARY_TOOL_ID => {
                    let request = discover_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetDiscoverySummary { request },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "get_discovery_summary"))
                }
                SMART_HOME_GET_PAIRING_PLAN_TOOL_ID => {
                    let request = pairing_plan_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetPairingPlan { request },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "get_pairing_plan"))
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
                SMART_HOME_LIST_ROOMS_TOOL_ID => {
                    let query = room_query(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::ListRooms { query },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_rooms"))
                }
                SMART_HOME_LIST_SCENES_TOOL_ID => {
                    let request = list_scenes_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_scenes"))
                }
                SMART_HOME_DESCRIBE_SCENE_TOOL_ID => {
                    let scene_id = SceneId::trusted(required_string(&arguments, "scene_id")?);
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::DescribeScene { scene_id },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "describe_scene"))
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
                SMART_HOME_REPORT_EVENT_TOOL_ID => {
                    let request = report_event_request(&arguments, now_ms)?;
                    let output = runtime
                        .execute_report_event_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(report_event_output_handler_output(output))
                }
                SMART_HOME_SUBSCRIBE_TOOL_ID => {
                    let request = subscribe_request(&arguments)?;
                    let output = runtime
                        .execute_subscribe_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(subscribe_output_handler_output(output))
                }
                SMART_HOME_POLL_EVENTS_TOOL_ID => {
                    let request = poll_events_request(&arguments)?;
                    let output = runtime
                        .execute_poll_events_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(poll_events_output_handler_output(output))
                }
                SMART_HOME_UNSUBSCRIBE_TOOL_ID => {
                    let request = unsubscribe_request(&arguments)?;
                    let output = runtime
                        .execute_unsubscribe_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(unsubscribe_output_handler_output(output))
                }
                SMART_HOME_LIST_SUBSCRIPTIONS_TOOL_ID => {
                    let request = list_subscriptions_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_subscriptions"))
                }
                SMART_HOME_INSPECT_EVENT_LOG_TOOL_ID => {
                    let request = inspect_event_log_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "inspect_event_log"))
                }
                SMART_HOME_LIST_AUTHORIZATION_DECISIONS_TOOL_ID => {
                    let query = authorization_decision_query(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::ListAuthorizationDecisions { query },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(
                        output,
                        "list_authorization_decisions",
                    ))
                }
                SMART_HOME_GET_AUTHORIZATION_SUMMARY_TOOL_ID => {
                    let query = authorization_decision_query(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetAuthorizationSummary { query },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(
                        output,
                        "get_authorization_summary",
                    ))
                }
                SMART_HOME_LIST_CAPABILITY_GRANTS_TOOL_ID => {
                    let query = capability_grant_query(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::ListCapabilityGrants { query },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_capability_grants"))
                }
                SMART_HOME_GET_CAPABILITY_GRANT_SUMMARY_TOOL_ID => {
                    let query = capability_grant_query(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetCapabilityGrantSummary { query },
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(
                        output,
                        "get_capability_grant_summary",
                    ))
                }
                SMART_HOME_GET_RUNTIME_SNAPSHOT_TOOL_ID => {
                    let _ = expect_object(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetRuntimeSnapshot,
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "get_runtime_snapshot"))
                }
                SMART_HOME_GET_TOPOLOGY_SUMMARY_TOOL_ID => {
                    let _ = expect_object(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetTopologySummary,
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "get_topology_summary"))
                }
                SMART_HOME_LIST_DESIRED_STATES_TOOL_ID => {
                    let request = list_desired_states_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_desired_states"))
                }
                SMART_HOME_SET_DESIRED_STATE_TOOL_ID => {
                    let request = set_desired_state_request(&arguments, &principal_id)?;
                    let output = runtime
                        .execute_set_desired_state_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(set_desired_state_output_handler_output(output))
                }
                SMART_HOME_CLEAR_DESIRED_STATE_TOOL_ID => {
                    let request = clear_desired_state_request(&arguments)?;
                    let output = runtime
                        .execute_clear_desired_state_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(clear_desired_state_output_handler_output(output))
                }
                SMART_HOME_LIST_PAIRING_SESSIONS_TOOL_ID => {
                    let request = list_pairing_sessions_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_pairing_sessions"))
                }
                SMART_HOME_LIST_WORKERS_TOOL_ID => {
                    let request = list_workers_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "list_workers"))
                }
                SMART_HOME_GET_WORKER_HEARTBEAT_SCHEDULE_TOOL_ID => {
                    let request = get_worker_heartbeat_schedule_request(&arguments)?;
                    let output = runtime
                        .execute_read_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(
                        output,
                        "get_worker_heartbeat_schedule",
                    ))
                }
                SMART_HOME_GET_SUPERVISION_PLAN_TOOL_ID => {
                    let _ = expect_object(&arguments)?;
                    let output = runtime
                        .execute_read_tool(
                            principal_id,
                            RuntimeReadToolRequest::GetSupervisionPlan,
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(read_output_handler_output(output, "get_supervision_plan"))
                }
                SMART_HOME_RECONCILE_DESIRED_STATES_TOOL_ID => {
                    let _ = expect_object(&arguments)?;
                    let output = runtime
                        .execute_supervision_tool(
                            principal_id,
                            RuntimeSupervisionToolRequest::ReconcileDesiredStates,
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(supervision_tool_output_handler_output(
                        output,
                        "reconcile_desired_states",
                    ))
                }
                SMART_HOME_RUN_SUPERVISION_TICK_TOOL_ID => {
                    let _ = expect_object(&arguments)?;
                    let output = runtime
                        .execute_supervision_tool(
                            principal_id,
                            RuntimeSupervisionToolRequest::RunSupervisionTick,
                            now_ms,
                        )
                        .map_err(runtime_error)?;
                    Ok(supervision_tool_output_handler_output(
                        output,
                        "run_supervision_tick",
                    ))
                }
                SMART_HOME_PAIR_BRIDGE_TOOL_ID => {
                    let request = pair_bridge_request(&arguments)?;
                    let output = runtime
                        .execute_pair_bridge_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(pair_bridge_output_handler_output(output))
                }
                SMART_HOME_COMPLETE_PAIRING_TOOL_ID => {
                    let request = complete_pairing_request(&arguments, now_ms)?;
                    let output = runtime
                        .execute_complete_pairing_tool(principal_id, request, now_ms)
                        .map_err(runtime_error)?;
                    Ok(complete_pairing_output_handler_output(output))
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
            SMART_HOME_LIST_INTEGRATIONS_TOOL_ID,
            "List smart-home integrations",
            "List D23A smart-home integration catalog entries and filter by reusable primitive, protocol, policy, and implementation traits.",
            integration_catalog_query_schema(),
            object_schema(
                vec![
                    SchemaProperty::new(
                        "integrations",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new("count", JsonSchema::Integer),
                    SchemaProperty::new("catalog_count", JsonSchema::Integer),
                ],
                vec!["integrations", "count", "catalog_count"],
                false,
            ),
        ),
        read_definition(
            SMART_HOME_DESCRIBE_INTEGRATION_TOOL_ID,
            "Describe smart-home integration",
            "Describe one D23A integration catalog entry, including activation and optional readiness information.",
            object_schema(
                vec![
                    SchemaProperty::new("integration_id", JsonSchema::String),
                    SchemaProperty::new(
                        "available_primitives",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::String),
                        },
                    ),
                    SchemaProperty::new(
                        "allowed_capability_ids",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::String),
                        },
                    ),
                    SchemaProperty::new(
                        "enabled_integrations",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::String),
                        },
                    ),
                ],
                vec!["integration_id"],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new("integration", JsonSchema::Any),
                    SchemaProperty::new("activation_plan", JsonSchema::Any),
                    SchemaProperty::new("readiness_report", JsonSchema::Any),
                ],
                vec!["integration", "activation_plan", "readiness_report"],
                false,
            ),
        ),
        read_definition(
            SMART_HOME_LIST_PRIMITIVES_TOOL_ID,
            "List smart-home primitives",
            "List D23A reusable smart-home primitive families and their integration backlog coverage.",
            object_schema(
                vec![
                    SchemaProperty::new("priority_at_or_before", JsonSchema::Integer),
                    SchemaProperty::new("include_ecosystem_coverage", JsonSchema::Boolean),
                    SchemaProperty::new("limit", JsonSchema::Integer),
                ],
                vec![],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new(
                        "primitives",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new(
                        "backlog",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new("primitive_count", JsonSchema::Integer),
                    SchemaProperty::new("backlog_count", JsonSchema::Integer),
                ],
                vec!["primitives", "backlog", "primitive_count", "backlog_count"],
                false,
            ),
        ),
        read_definition(
            SMART_HOME_DESCRIBE_PRIMITIVE_TOOL_ID,
            "Describe smart-home primitive",
            "Describe one D23A reusable primitive family, including catalog integrations and ecosystem source coverage.",
            object_schema(
                vec![
                    SchemaProperty::new("primitive", JsonSchema::String),
                    SchemaProperty::new("priority_at_or_before", JsonSchema::Integer),
                ],
                vec!["primitive"],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new("primitive", JsonSchema::Any),
                    SchemaProperty::new(
                        "integrations",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new(
                        "ecosystem_sources",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new("integration_count", JsonSchema::Integer),
                    SchemaProperty::new("source_count", JsonSchema::Integer),
                    SchemaProperty::new("platform_count", JsonSchema::Integer),
                ],
                vec![
                    "primitive",
                    "integrations",
                    "ecosystem_sources",
                    "integration_count",
                    "source_count",
                    "platform_count",
                ],
                false,
            ),
        ),
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
            SMART_HOME_LIST_DISCOVERY_WORKERS_TOOL_ID,
            "List smart-home discovery workers",
            "List scheduled D23 discovery workers and summarize scheduler pressure without running discovery.",
            object_schema(
                vec![
                    SchemaProperty::new("worker_id", JsonSchema::String),
                    SchemaProperty::new("integration_id", JsonSchema::String),
                    SchemaProperty::new("kind", JsonSchema::String),
                    SchemaProperty::new("kinds", string_array_schema()),
                    SchemaProperty::new("source", JsonSchema::String),
                    SchemaProperty::new("sources", string_array_schema()),
                    SchemaProperty::new("status", JsonSchema::String),
                    SchemaProperty::new("statuses", string_array_schema()),
                    SchemaProperty::new("due_before_ms", JsonSchema::Integer),
                    SchemaProperty::new("overdue_at_ms", JsonSchema::Integer),
                    SchemaProperty::new("min_consecutive_failure_count", JsonSchema::Integer),
                    SchemaProperty::new("sort", JsonSchema::String),
                    SchemaProperty::new("limit", JsonSchema::Integer),
                ],
                vec![],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new(
                        "workers",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new("summary", JsonSchema::Any),
                    SchemaProperty::new("count", JsonSchema::Integer),
                ],
                vec!["workers", "summary", "count"],
                false,
            ),
        ),
        read_definition(
            SMART_HOME_GET_DISCOVERY_SUMMARY_TOOL_ID,
            "Get smart-home discovery summary",
            "Summarize D23 discovery candidates and signal freshness without returning candidate payloads.",
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
                    SchemaProperty::new("record_summary", JsonSchema::Any),
                    SchemaProperty::new("signal_summary", JsonSchema::Any),
                ],
                vec![
                    "generated_at_ms",
                    "ttl_ms",
                    "record_summary",
                    "signal_summary",
                ],
                false,
            ),
        ),
        read_definition(
            SMART_HOME_GET_PAIRING_PLAN_TOOL_ID,
            "Get smart-home pairing plan",
            "Read the D23 discovery pairing plan and host action queue without starting a pairing session.",
            object_schema(
                vec![
                    SchemaProperty::new("integration_id", JsonSchema::String),
                    SchemaProperty::new("integration_ids", string_array_schema()),
                    SchemaProperty::new("source", JsonSchema::String),
                    SchemaProperty::new("sources", string_array_schema()),
                    SchemaProperty::new("signal_status", JsonSchema::String),
                    SchemaProperty::new("signal_statuses", string_array_schema()),
                    SchemaProperty::new("pairing_requirement", JsonSchema::String),
                    SchemaProperty::new("pairing_requirements", string_array_schema()),
                    SchemaProperty::new("action", JsonSchema::String),
                    SchemaProperty::new("actions", string_array_schema()),
                    SchemaProperty::new("priority_at_or_before", JsonSchema::Integer),
                    SchemaProperty::new("requires_human_action", JsonSchema::Boolean),
                    SchemaProperty::new("actionable_only", JsonSchema::Boolean),
                    SchemaProperty::new("sort", JsonSchema::String),
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
                        "targets",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new("summary", JsonSchema::Any),
                    SchemaProperty::new("count", JsonSchema::Integer),
                ],
                vec!["generated_at_ms", "ttl_ms", "targets", "summary", "count"],
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
            SMART_HOME_LIST_ROOMS_TOOL_ID,
            "List smart-home rooms",
            "List runtime-derived D23 room topology summaries, including device health, state coverage, and scene action coverage.",
            object_schema(
                vec![
                    SchemaProperty::new("room_id", JsonSchema::String),
                    SchemaProperty::new("attention_only", JsonSchema::Boolean),
                    SchemaProperty::new("state_gaps_only", JsonSchema::Boolean),
                    SchemaProperty::new("sort", JsonSchema::String),
                    SchemaProperty::new("limit", JsonSchema::Integer),
                ],
                vec![],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new(
                        "rooms",
                        JsonSchema::Array {
                            items: Box::new(JsonSchema::Any),
                        },
                    ),
                    SchemaProperty::new("topology", JsonSchema::Any),
                    SchemaProperty::new("count", JsonSchema::Integer),
                ],
                vec!["rooms", "topology", "count"],
                false,
            ),
        ),
        read_definition(
            SMART_HOME_LIST_SCENES_TOOL_ID,
            "List smart-home scenes",
            "List normalized D23 smart-home scenes, optionally filtered by scope, target entity, or target capability.",
            object_schema(
                vec![
                    SchemaProperty::new("scope", JsonSchema::String),
                    SchemaProperty::new("entity_id", JsonSchema::String),
                    SchemaProperty::new("capability_id", JsonSchema::String),
                ],
                vec![],
                false,
            ),
            collection_output_schema("scenes"),
        ),
        read_definition(
            SMART_HOME_DESCRIBE_SCENE_TOOL_ID,
            "Describe smart-home scene",
            "Read one normalized D23 smart-home scene and its target actions.",
            object_schema(
                vec![SchemaProperty::new("scene_id", JsonSchema::String)],
                vec!["scene_id"],
                false,
            ),
            object_schema(
                vec![
                    SchemaProperty::new("scene_id", JsonSchema::String),
                    SchemaProperty::new("scene", JsonSchema::Any),
                ],
                vec!["scene_id", "scene"],
                false,
            ),
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
        report_event_definition(),
        subscribe_definition(),
        poll_events_definition(),
        unsubscribe_definition(),
        list_subscriptions_definition(),
        inspect_event_log_definition(),
        list_authorization_decisions_definition(),
        get_authorization_summary_definition(),
        list_capability_grants_definition(),
        get_capability_grant_summary_definition(),
        get_runtime_snapshot_definition(),
        get_topology_summary_definition(),
        list_desired_states_definition(),
        set_desired_state_definition(),
        clear_desired_state_definition(),
        list_pairing_sessions_definition(),
        list_workers_definition(),
        get_worker_heartbeat_schedule_definition(),
        get_supervision_plan_definition(),
        reconcile_desired_states_definition(),
        run_supervision_tick_definition(),
        pair_bridge_definition(),
        complete_pairing_definition(),
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

fn poll_events_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_POLL_EVENTS_TOOL_ID.to_string(),
        display_name: "Poll smart-home events".to_string(),
        description:
            "Peek or drain queued normalized D23 events for one smart-home runtime subscription."
                .to_string(),
        input_schema: object_schema(
            vec![
                SchemaProperty::new("subscription_id", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
                SchemaProperty::new("peek", JsonSchema::Boolean),
            ],
            vec!["subscription_id"],
            false,
        ),
        output_schema: Some(event_delivery_output_schema()),
        side_effects: ToolSideEffects::Read,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Serialized,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier0,
        required_capabilities: vec!["smart_home:read".to_string()],
        preferred_lock_scope: Some("smart_home.events".to_string()),
        timeout_seconds: Some(5),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "events".to_string(),
        ],
        stability: ToolStability::Experimental,
    }
}

fn unsubscribe_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_UNSUBSCRIBE_TOOL_ID.to_string(),
        display_name: "Unsubscribe from smart-home events".to_string(),
        description:
            "Remove one smart-home runtime subscription and return any queued events left for it."
                .to_string(),
        input_schema: object_schema(
            vec![SchemaProperty::new("subscription_id", JsonSchema::String)],
            vec!["subscription_id"],
            false,
        ),
        output_schema: Some(object_schema(
            vec![
                SchemaProperty::new("subscription_id", JsonSchema::String),
                SchemaProperty::new("unsubscribed", JsonSchema::Boolean),
                SchemaProperty::new("delivered_events", JsonSchema::Integer),
                SchemaProperty::new("remaining_events", JsonSchema::Integer),
                SchemaProperty::new("has_more", JsonSchema::Boolean),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new(
                    "events",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
            ],
            vec![
                "subscription_id",
                "unsubscribed",
                "delivered_events",
                "remaining_events",
                "has_more",
                "summary",
                "events",
            ],
            false,
        )),
        side_effects: ToolSideEffects::Read,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Serialized,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier0,
        required_capabilities: vec!["smart_home:read".to_string()],
        preferred_lock_scope: Some("smart_home.events".to_string()),
        timeout_seconds: Some(5),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "events".to_string(),
        ],
        stability: ToolStability::Experimental,
    }
}

fn list_subscriptions_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_LIST_SUBSCRIPTIONS_TOOL_ID,
        "List smart-home event subscriptions",
        "List active D23 smart-home runtime event subscriptions and summarize queued event pressure.",
        object_schema(
            vec![
                SchemaProperty::new("subscription_id", JsonSchema::String),
                SchemaProperty::new("filter", JsonSchema::Any),
                SchemaProperty::new("min_queued_events", JsonSchema::Integer),
                SchemaProperty::new("sort", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec![],
            false,
        ),
        object_schema(
            vec![
                SchemaProperty::new(
                    "subscriptions",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new("count", JsonSchema::Integer),
            ],
            vec!["subscriptions", "summary", "count"],
            false,
        ),
    )
}

fn inspect_event_log_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_INSPECT_EVENT_LOG_TOOL_ID,
        "Inspect smart-home event log",
        "Query checkpointed D23 smart-home runtime events without creating or draining a subscription.",
        object_schema(
            vec![
                SchemaProperty::new("filter", JsonSchema::Any),
                SchemaProperty::new("from_checkpoint", JsonSchema::Integer),
                SchemaProperty::new("sort", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec![],
            false,
        ),
        object_schema(
            vec![
                SchemaProperty::new(
                    "events",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new("count", JsonSchema::Integer),
            ],
            vec!["events", "summary", "count"],
            false,
        ),
    )
}

fn list_authorization_decisions_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_LIST_AUTHORIZATION_DECISIONS_TOOL_ID,
        "List smart-home authorization decisions",
        "List D23 smart-home tool and command authorization decisions, optionally filtered by Chief principal or outcome.",
        authorization_decision_query_schema(),
        object_schema(
            vec![
                SchemaProperty::new(
                    "decisions",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new("count", JsonSchema::Integer),
            ],
            vec!["decisions", "summary", "count"],
            false,
        ),
    )
}

fn get_authorization_summary_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_GET_AUTHORIZATION_SUMMARY_TOOL_ID,
        "Get smart-home authorization summary",
        "Summarize D23 smart-home authorization decisions without returning individual audit rows.",
        authorization_decision_query_schema(),
        object_schema(
            vec![SchemaProperty::new("summary", JsonSchema::Any)],
            vec!["summary"],
            false,
        ),
    )
}

fn list_capability_grants_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_LIST_CAPABILITY_GRANTS_TOOL_ID,
        "List smart-home capability grants",
        "List D23 smart-home capability grants, optionally filtered by Chief principal, effective status, scope, capability, or entity.",
        capability_grant_query_schema(),
        object_schema(
            vec![
                SchemaProperty::new(
                    "grants",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new("count", JsonSchema::Integer),
            ],
            vec!["grants", "summary", "count"],
            false,
        ),
    )
}

fn get_capability_grant_summary_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_GET_CAPABILITY_GRANT_SUMMARY_TOOL_ID,
        "Get smart-home capability grant summary",
        "Summarize D23 smart-home capability grants without returning individual grant rows.",
        capability_grant_query_schema(),
        object_schema(
            vec![SchemaProperty::new("summary", JsonSchema::Any)],
            vec!["summary"],
            false,
        ),
    )
}

fn authorization_decision_query_schema() -> JsonSchema {
    object_schema(
        vec![
            SchemaProperty::new("principal_id", JsonSchema::String),
            SchemaProperty::new("outcome", JsonSchema::String),
            SchemaProperty::new("sort", JsonSchema::String),
            SchemaProperty::new("limit", JsonSchema::Integer),
        ],
        vec![],
        false,
    )
}

fn capability_grant_query_schema() -> JsonSchema {
    object_schema(
        vec![
            SchemaProperty::new("principal_id", JsonSchema::String),
            SchemaProperty::new("status", JsonSchema::String),
            SchemaProperty::new("scope_kind", JsonSchema::String),
            SchemaProperty::new("capability_id", JsonSchema::String),
            SchemaProperty::new("entity_id", JsonSchema::String),
            SchemaProperty::new("sort", JsonSchema::String),
            SchemaProperty::new("limit", JsonSchema::Integer),
        ],
        vec![],
        false,
    )
}

fn get_runtime_snapshot_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_GET_RUNTIME_SNAPSHOT_TOOL_ID,
        "Get smart-home runtime snapshot",
        "Read compact D23 smart-home runtime counts and pending-work pressure without mutating supervision state.",
        empty_object_schema(),
        object_schema(
            vec![
                SchemaProperty::new("generated_at_ms", JsonSchema::Integer),
                SchemaProperty::new("registry_counts", JsonSchema::Any),
                SchemaProperty::new("discovery_record_count", JsonSchema::Integer),
                SchemaProperty::new("event_bus", JsonSchema::Any),
                SchemaProperty::new("supervisor", JsonSchema::Any),
                SchemaProperty::new("discovery_scheduler", JsonSchema::Any),
                SchemaProperty::new("pairing_session_count", JsonSchema::Integer),
                SchemaProperty::new("expiring_pairing_session_count", JsonSchema::Integer),
                SchemaProperty::new("optimistic_state_count", JsonSchema::Integer),
                SchemaProperty::new("stale_optimistic_state_count", JsonSchema::Integer),
                SchemaProperty::new("desired_state_count", JsonSchema::Integer),
                SchemaProperty::new("desired_capability_count", JsonSchema::Integer),
                SchemaProperty::new("state_refresh_target_count", JsonSchema::Integer),
                SchemaProperty::new("pending_work", JsonSchema::Any),
                SchemaProperty::new("has_pending_work", JsonSchema::Boolean),
            ],
            vec![
                "generated_at_ms",
                "registry_counts",
                "discovery_record_count",
                "event_bus",
                "supervisor",
                "discovery_scheduler",
                "pairing_session_count",
                "expiring_pairing_session_count",
                "optimistic_state_count",
                "stale_optimistic_state_count",
                "desired_state_count",
                "desired_capability_count",
                "state_refresh_target_count",
                "pending_work",
                "has_pending_work",
            ],
            false,
        ),
    )
}

fn get_topology_summary_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_GET_TOPOLOGY_SUMMARY_TOOL_ID,
        "Get smart-home topology summary",
        "Read aggregate D23 registry topology coverage across bridges, devices, rooms, entities, cached states, and scenes.",
        empty_object_schema(),
        object_schema(
            vec![SchemaProperty::new("summary", JsonSchema::Any)],
            vec!["summary"],
            false,
        ),
    )
}

fn list_desired_states_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_LIST_DESIRED_STATES_TOOL_ID,
        "List smart-home desired states",
        "List runtime-owned D23 desired-state targets so Chief of Staff jobs can plan reconciliation without issuing commands.",
        object_schema(
            vec![
                SchemaProperty::new("entity_id", JsonSchema::String),
                SchemaProperty::new("requested_by", JsonSchema::String),
                SchemaProperty::new("capability_id", JsonSchema::String),
                SchemaProperty::new("min_command_timeout_ms", JsonSchema::Integer),
                SchemaProperty::new("max_command_timeout_ms", JsonSchema::Integer),
                SchemaProperty::new("sort", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec![],
            false,
        ),
        object_schema(
            vec![
                SchemaProperty::new(
                    "desired_states",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new("count", JsonSchema::Integer),
            ],
            vec!["desired_states", "summary", "count"],
            false,
        ),
    )
}

fn set_desired_state_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_SET_DESIRED_STATE_TOOL_ID.to_string(),
        display_name: "Set smart-home desired state".to_string(),
        description: "Set or replace a runtime-owned D23 desired-state target without directly issuing a device command."
            .to_string(),
        input_schema: object_schema(
            vec![
                SchemaProperty::new("entity_id", JsonSchema::String),
                SchemaProperty::new("desired", JsonSchema::Any),
                SchemaProperty::new("requested_by", JsonSchema::String),
                SchemaProperty::new("command_timeout_ms", JsonSchema::Integer),
            ],
            vec!["entity_id", "desired"],
            false,
        ),
        output_schema: Some(object_schema(
            vec![
                SchemaProperty::new("entity_id", JsonSchema::String),
                SchemaProperty::new("desired_state", JsonSchema::Any),
                SchemaProperty::new("replaced", JsonSchema::Boolean),
                SchemaProperty::new("previous", JsonSchema::Any),
                SchemaProperty::new("desired_capability_count", JsonSchema::Integer),
            ],
            vec![
                "entity_id",
                "desired_state",
                "replaced",
                "previous",
                "desired_capability_count",
            ],
            false,
        )),
        side_effects: ToolSideEffects::Write,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Serialized,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier1,
        required_capabilities: vec!["smart_home:command".to_string()],
        preferred_lock_scope: Some("smart_home.desired_state".to_string()),
        timeout_seconds: Some(10),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "desired_state".to_string(),
        ],
        stability: ToolStability::Experimental,
    }
}

fn clear_desired_state_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_CLEAR_DESIRED_STATE_TOOL_ID.to_string(),
        display_name: "Clear smart-home desired state".to_string(),
        description:
            "Clear one runtime-owned D23 desired-state target without touching device state."
                .to_string(),
        input_schema: object_schema(
            vec![SchemaProperty::new("entity_id", JsonSchema::String)],
            vec!["entity_id"],
            false,
        ),
        output_schema: Some(object_schema(
            vec![
                SchemaProperty::new("entity_id", JsonSchema::String),
                SchemaProperty::new("removed", JsonSchema::Boolean),
                SchemaProperty::new("desired_state", JsonSchema::Any),
            ],
            vec!["entity_id", "removed", "desired_state"],
            false,
        )),
        side_effects: ToolSideEffects::Write,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Serialized,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier1,
        required_capabilities: vec!["smart_home:command".to_string()],
        preferred_lock_scope: Some("smart_home.desired_state".to_string()),
        timeout_seconds: Some(10),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "desired_state".to_string(),
        ],
        stability: ToolStability::Experimental,
    }
}

fn list_pairing_sessions_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_LIST_PAIRING_SESSIONS_TOOL_ID,
        "List smart-home pairing sessions",
        "List runtime-owned D23 pairing sessions and summarize pending credential ceremonies.",
        object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("bridge_id", JsonSchema::String),
                SchemaProperty::new("integration_id", JsonSchema::String),
                SchemaProperty::new("requested_by", JsonSchema::String),
                SchemaProperty::new("status", JsonSchema::String),
                SchemaProperty::new(
                    "statuses",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::String),
                    },
                ),
                SchemaProperty::new("expires_before_ms", JsonSchema::Integer),
                SchemaProperty::new("expiring_at_ms", JsonSchema::Integer),
                SchemaProperty::new("sort", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec![],
            false,
        ),
        object_schema(
            vec![
                SchemaProperty::new(
                    "sessions",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new("count", JsonSchema::Integer),
            ],
            vec!["sessions", "summary", "count"],
            false,
        ),
    )
}

fn list_workers_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_LIST_WORKERS_TOOL_ID,
        "List smart-home workers",
        "List supervised D23 bridge workers with status, heartbeat, restart, and overdue filters without mutating supervisor state.",
        object_schema(
            vec![
                SchemaProperty::new("bridge_id", JsonSchema::String),
                SchemaProperty::new("integration_id", JsonSchema::String),
                SchemaProperty::new("status", JsonSchema::String),
                SchemaProperty::new(
                    "statuses",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::String),
                    },
                ),
                SchemaProperty::new("heartbeat_due_before_ms", JsonSchema::Integer),
                SchemaProperty::new("overdue_at_ms", JsonSchema::Integer),
                SchemaProperty::new("min_restart_count", JsonSchema::Integer),
                SchemaProperty::new("sort", JsonSchema::String),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec![],
            false,
        ),
        object_schema(
            vec![
                SchemaProperty::new(
                    "workers",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new("count", JsonSchema::Integer),
            ],
            vec!["workers", "summary", "count"],
            false,
        ),
    )
}

fn get_worker_heartbeat_schedule_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_GET_WORKER_HEARTBEAT_SCHEDULE_TOOL_ID,
        "Get smart-home worker heartbeat schedule",
        "Read supervised D23 bridge-worker heartbeat deadlines, optionally filtered by bridge or due window, without mutating supervisor state.",
        object_schema(
            vec![
                SchemaProperty::new("bridge_id", JsonSchema::String),
                SchemaProperty::new("due_at_or_before_ms", JsonSchema::Integer),
                SchemaProperty::new("limit", JsonSchema::Integer),
            ],
            vec![],
            false,
        ),
        object_schema(
            vec![
                SchemaProperty::new("generated_at_ms", JsonSchema::Integer),
                SchemaProperty::new(
                    "deadlines",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("count", JsonSchema::Integer),
                SchemaProperty::new("due_count", JsonSchema::Integer),
                SchemaProperty::new("next_due_at_ms", JsonSchema::Any),
                SchemaProperty::new("is_empty", JsonSchema::Boolean),
            ],
            vec![
                "generated_at_ms",
                "deadlines",
                "count",
                "due_count",
                "next_due_at_ms",
                "is_empty",
            ],
            false,
        ),
    )
}

fn get_supervision_plan_definition() -> ToolDefinition {
    read_definition(
        SMART_HOME_GET_SUPERVISION_PLAN_TOOL_ID,
        "Get smart-home supervision plan",
        "Preview the D23 runtime supervision plan for due pairing expiry, state refresh, reconciliation, worker restart, and discovery work without mutating runtime state.",
        empty_object_schema(),
        object_schema(
            vec![
                SchemaProperty::new("generated_at_ms", JsonSchema::Integer),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new("is_idle", JsonSchema::Boolean),
                SchemaProperty::new("action_count", JsonSchema::Integer),
                SchemaProperty::new(
                    "pairing_sessions_expiring",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::String),
                    },
                ),
                SchemaProperty::new("state_refresh_plan", JsonSchema::Any),
                SchemaProperty::new(
                    "desired_state_drifts",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new("worker_restart_plan", JsonSchema::Any),
                SchemaProperty::new("discovery_worker_run_plan", JsonSchema::Any),
            ],
            vec![
                "generated_at_ms",
                "summary",
                "is_idle",
                "action_count",
                "pairing_sessions_expiring",
                "state_refresh_plan",
                "desired_state_drifts",
                "worker_restart_plan",
                "discovery_worker_run_plan",
            ],
            false,
        ),
    )
}

fn reconcile_desired_states_definition() -> ToolDefinition {
    supervision_command_definition(
        SMART_HOME_RECONCILE_DESIRED_STATES_TOOL_ID,
        "Reconcile smart-home desired states",
        "Run D23 desired-state reconciliation through the runtime authorization and command path.",
        object_schema(
            vec![
                SchemaProperty::new("reconciled_at_ms", JsonSchema::Integer),
                SchemaProperty::new("action_count", JsonSchema::Integer),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new(
                    "actions",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
            ],
            vec!["reconciled_at_ms", "action_count", "summary", "actions"],
            false,
        ),
    )
}

fn run_supervision_tick_definition() -> ToolDefinition {
    supervision_command_definition(
        SMART_HOME_RUN_SUPERVISION_TICK_TOOL_ID,
        "Run smart-home supervision tick",
        "Run one D23 runtime supervision tick for pairing expiry, optimistic-state expiry, desired-state reconciliation, and worker restart events.",
        object_schema(
            vec![
                SchemaProperty::new("ticked_at_ms", JsonSchema::Integer),
                SchemaProperty::new("is_idle", JsonSchema::Boolean),
                SchemaProperty::new("action_count", JsonSchema::Integer),
                SchemaProperty::new("summary", JsonSchema::Any),
                SchemaProperty::new(
                    "expired_pairing_sessions",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::String),
                    },
                ),
                SchemaProperty::new(
                    "expired_entities",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::String),
                    },
                ),
                SchemaProperty::new(
                    "desired_state_actions",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
                SchemaProperty::new(
                    "worker_events",
                    JsonSchema::Array {
                        items: Box::new(JsonSchema::Any),
                    },
                ),
            ],
            vec![
                "ticked_at_ms",
                "is_idle",
                "action_count",
                "summary",
                "expired_pairing_sessions",
                "expired_entities",
                "desired_state_actions",
                "worker_events",
            ],
            false,
        ),
    )
}

fn supervision_command_definition(
    tool_id: &str,
    display_name: &str,
    description: &str,
    output_schema: JsonSchema,
) -> ToolDefinition {
    ToolDefinition {
        tool_id: tool_id.to_string(),
        display_name: display_name.to_string(),
        description: description.to_string(),
        input_schema: empty_object_schema(),
        output_schema: Some(output_schema),
        side_effects: ToolSideEffects::External,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Serialized,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier1,
        required_capabilities: vec!["smart_home:command".to_string()],
        preferred_lock_scope: Some("smart_home.supervision".to_string()),
        timeout_seconds: Some(15),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "supervision".to_string(),
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

fn complete_pairing_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_COMPLETE_PAIRING_TOOL_ID.to_string(),
        display_name: "Complete smart-home pairing".to_string(),
        description:
            "Complete a D23 bridge-pairing session using a VaultRef and non-secret metadata only."
                .to_string(),
        input_schema: object_schema(
            vec![
                SchemaProperty::new("session_id", JsonSchema::String),
                SchemaProperty::new("vault_ref", JsonSchema::String),
                SchemaProperty::new("completed_at_ms", JsonSchema::Integer),
                SchemaProperty::new("metadata", JsonSchema::Any),
            ],
            vec!["session_id", "vault_ref"],
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

fn report_event_definition() -> ToolDefinition {
    ToolDefinition {
        tool_id: SMART_HOME_REPORT_EVENT_TOOL_ID.to_string(),
        display_name: "Report smart-home event".to_string(),
        description:
            "Ingest one adapter-observed D23 device event or bridge-health report into the smart-home runtime."
                .to_string(),
        input_schema: object_schema(
            vec![
                SchemaProperty::new("event_kind", JsonSchema::String),
                SchemaProperty::new("event_id", JsonSchema::String),
                SchemaProperty::new("bridge_id", JsonSchema::String),
                SchemaProperty::new("device_id", JsonSchema::String),
                SchemaProperty::new("entity_id", JsonSchema::String),
                SchemaProperty::new("event_type", JsonSchema::String),
                SchemaProperty::new("observed_at_ms", JsonSchema::Integer),
                SchemaProperty::new("received_at_ms", JsonSchema::Integer),
                SchemaProperty::new("capability_id", JsonSchema::String),
                SchemaProperty::new("value", JsonSchema::Any),
                SchemaProperty::new("raw_ref", JsonSchema::String),
                SchemaProperty::new("correlation_id", JsonSchema::String),
                SchemaProperty::new("health", JsonSchema::String),
                SchemaProperty::new("metadata", JsonSchema::Any),
            ],
            vec!["event_kind", "event_id", "bridge_id"],
            false,
        ),
        output_schema: Some(object_schema(
            vec![
                SchemaProperty::new("event_kind", JsonSchema::String),
                SchemaProperty::new("event_id", JsonSchema::String),
                SchemaProperty::new("bridge_id", JsonSchema::String),
                SchemaProperty::new("device_id", JsonSchema::Any),
                SchemaProperty::new("entity_id", JsonSchema::Any),
                SchemaProperty::new("event_type", JsonSchema::Any),
                SchemaProperty::new("health", JsonSchema::Any),
                SchemaProperty::new("observed_at_ms", JsonSchema::Integer),
                SchemaProperty::new("received_at_ms", JsonSchema::Integer),
                SchemaProperty::new("state_delta", JsonSchema::Any),
                SchemaProperty::new("metadata", JsonSchema::Any),
            ],
            vec![
                "event_kind",
                "event_id",
                "bridge_id",
                "device_id",
                "entity_id",
                "event_type",
                "health",
                "observed_at_ms",
                "received_at_ms",
                "state_delta",
                "metadata",
            ],
            false,
        )),
        side_effects: ToolSideEffects::External,
        idempotency: ToolIdempotency::Conditional,
        concurrency: ToolConcurrency::Serialized,
        streaming: ToolStreaming::Events,
        required_tier: ToolPrivilegeTier::Tier1,
        required_capabilities: vec!["smart_home:ingest".to_string()],
        preferred_lock_scope: Some("smart_home.events".to_string()),
        timeout_seconds: Some(10),
        tags: vec![
            "smart_home".to_string(),
            "runtime".to_string(),
            "events".to_string(),
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

fn discovery_worker_query(arguments: &JsonValue) -> Result<DiscoveryWorkerQuery, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = DiscoveryWorkerQuery::new();
    if let Some(worker_id) = optional_string(arguments, "worker_id")? {
        query = query.for_worker(DiscoveryWorkerId::trusted(worker_id));
    }
    if let Some(integration_id) = optional_string(arguments, "integration_id")? {
        query = query.for_integration(IntegrationId::trusted(integration_id));
    }
    for kind in optional_string_list(arguments, "kind", "kinds")? {
        query = query.with_kind(parse_discovery_worker_kind(&kind)?);
    }
    for source in optional_string_list(arguments, "source", "sources")? {
        query = query.with_source(parse_discovery_source(&source)?);
    }
    for status in optional_string_list(arguments, "status", "statuses")? {
        query = query.with_status(parse_worker_status(&status)?);
    }
    if let Some(due_before_ms) = optional_u64(arguments, "due_before_ms")? {
        query = query.due_before(due_before_ms);
    }
    if let Some(overdue_at_ms) = optional_u64(arguments, "overdue_at_ms")? {
        query = query.overdue_at(overdue_at_ms);
    }
    if let Some(count) = optional_u64(arguments, "min_consecutive_failure_count")? {
        query = query.min_consecutive_failure_count(count as u32);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_discovery_worker_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(query)
}

fn pairing_plan_request(
    arguments: &JsonValue,
) -> Result<RuntimePairingPlanToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut options = DiscoveryPairingPlanOptions::new();
    for integration_id in optional_string_list(arguments, "integration_id", "integration_ids")? {
        options = options.with_integration(IntegrationId::trusted(integration_id));
    }
    for source in optional_string_list(arguments, "source", "sources")? {
        options = options.with_source(parse_discovery_source(&source)?);
    }
    for status in optional_string_list(arguments, "signal_status", "signal_statuses")? {
        options = options.with_signal_status(parse_discovery_signal_status(&status)?);
    }
    for requirement in
        optional_string_list(arguments, "pairing_requirement", "pairing_requirements")?
    {
        options = options.with_pairing_requirement(parse_pairing_requirement(&requirement)?);
    }
    for action in optional_string_list(arguments, "action", "actions")? {
        options = options.with_action(parse_discovery_pairing_action(&action)?);
    }
    if let Some(priority) = optional_u8(arguments, "priority_at_or_before")? {
        options = options.at_or_before_priority(priority);
    }
    if let Some(requires_human_action) = optional_bool(arguments, "requires_human_action")? {
        options = options.requiring_human_action(requires_human_action);
    }
    if let Some(actionable_only) = optional_bool(arguments, "actionable_only")? {
        options = options.actionable_only(actionable_only);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        options = options.sorted_by(parse_discovery_pairing_plan_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        options = options.limited_to(limit as usize);
    }

    let mut request = RuntimePairingPlanToolRequest::new().with_options(options);
    if let Some(ttl_ms) = optional_u64(arguments, "ttl_ms")? {
        request = request.with_ttl_ms(ttl_ms);
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

fn room_query(arguments: &JsonValue) -> Result<RuntimeRoomQuery, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = RuntimeRoomQuery::new();
    if let Some(room_id) = optional_string(arguments, "room_id")? {
        query = query.for_room(room_id);
    }
    if let Some(attention_only) = optional_bool(arguments, "attention_only")? {
        query = query.attention_only(attention_only);
    }
    if let Some(state_gaps_only) = optional_bool(arguments, "state_gaps_only")? {
        query = query.state_gaps_only(state_gaps_only);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_room_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(query)
}

fn list_scenes_request(arguments: &JsonValue) -> Result<RuntimeReadToolRequest, ToolCallError> {
    Ok(RuntimeReadToolRequest::ListScenes {
        scope: optional_string(arguments, "scope")?
            .map(|value| parse_scene_scope(&value))
            .transpose()?,
        entity_id: optional_string(arguments, "entity_id")?.map(EntityId::trusted),
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

fn report_event_request(
    arguments: &JsonValue,
    now_ms: u64,
) -> Result<RuntimeReportEventToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    match required_string(arguments, "event_kind")?.as_str() {
        "device" | "device_event" => report_device_event_request(arguments, now_ms),
        "bridge_health" | "health" => report_bridge_health_request(arguments, now_ms),
        label => Err(validation_error(format!(
            "unknown report event kind `{label}`"
        ))),
    }
}

fn report_device_event_request(
    arguments: &JsonValue,
    now_ms: u64,
) -> Result<RuntimeReportEventToolRequest, ToolCallError> {
    let state_delta = match optional_string(arguments, "capability_id")? {
        Some(capability_id) => Some(StateDelta {
            capability_id: CapabilityId::trusted(capability_id),
            value: optional_field(arguments, "value")
                .map(json_to_smart_value)
                .transpose()?
                .unwrap_or(Value::Null),
        }),
        None => None,
    };

    Ok(RuntimeReportEventToolRequest::device(DeviceEvent {
        event_id: EventId::trusted(required_string(arguments, "event_id")?),
        bridge_id: BridgeId::trusted(required_string(arguments, "bridge_id")?),
        device_id: optional_string(arguments, "device_id")?.map(DeviceId::trusted),
        entity_id: optional_string(arguments, "entity_id")?.map(EntityId::trusted),
        observed_at_ms: optional_u64(arguments, "observed_at_ms")?.unwrap_or(now_ms),
        received_at_ms: optional_u64(arguments, "received_at_ms")?.unwrap_or(now_ms),
        event_type: parse_device_event_type(&required_string(arguments, "event_type")?)?,
        state_delta,
        raw_ref: optional_string(arguments, "raw_ref")?,
        correlation_id: optional_string(arguments, "correlation_id")?.map(CorrelationId::trusted),
        metadata: optional_metadata(arguments)?,
    }))
}

fn report_bridge_health_request(
    arguments: &JsonValue,
    now_ms: u64,
) -> Result<RuntimeReportEventToolRequest, ToolCallError> {
    Ok(RuntimeReportEventToolRequest::bridge_health(
        BridgeHealthReport {
            event_id: EventId::trusted(required_string(arguments, "event_id")?),
            bridge_id: BridgeId::trusted(required_string(arguments, "bridge_id")?),
            health: parse_health(&required_string(arguments, "health")?)?,
            observed_at_ms: optional_u64(arguments, "observed_at_ms")?.unwrap_or(now_ms),
            received_at_ms: optional_u64(arguments, "received_at_ms")?.unwrap_or(now_ms),
            metadata: optional_metadata(arguments)?,
        },
    ))
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

fn poll_events_request(
    arguments: &JsonValue,
) -> Result<RuntimePollEventsToolRequest, ToolCallError> {
    let subscription_id =
        RuntimeSubscriptionId::trusted(required_string(arguments, "subscription_id")?);
    let mut request = RuntimePollEventsToolRequest::new(subscription_id);
    if let Some(limit) = optional_u64(arguments, "limit")? {
        request = request.with_limit(limit as usize);
    }
    if let Some(peek) = optional_bool(arguments, "peek")? {
        request = request.peek(peek);
    }
    Ok(request)
}

fn unsubscribe_request(
    arguments: &JsonValue,
) -> Result<RuntimeUnsubscribeToolRequest, ToolCallError> {
    Ok(RuntimeUnsubscribeToolRequest::new(
        RuntimeSubscriptionId::trusted(required_string(arguments, "subscription_id")?),
    ))
}

fn list_subscriptions_request(
    arguments: &JsonValue,
) -> Result<RuntimeReadToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = RuntimeSubscriptionQuery::new();
    if let Some(subscription_id) = optional_string(arguments, "subscription_id")? {
        query = query.for_subscription(RuntimeSubscriptionId::trusted(subscription_id));
    }
    if let Some(filter) = optional_field(arguments, "filter") {
        query = query.matching(parse_event_filter(filter)?);
    }
    if let Some(min_queued_events) = optional_u64(arguments, "min_queued_events")? {
        query = query.with_min_queued_events(min_queued_events as usize);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_subscription_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(RuntimeReadToolRequest::ListSubscriptions { query })
}

fn inspect_event_log_request(
    arguments: &JsonValue,
) -> Result<RuntimeReadToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = RuntimeEventQuery::new();
    if let Some(filter) = optional_field(arguments, "filter") {
        query = query.matching(parse_event_filter(filter)?);
    }
    if let Some(from_checkpoint) = optional_u64(arguments, "from_checkpoint")? {
        query = query.from_checkpoint(RuntimeEventCheckpoint::from_next_sequence(from_checkpoint));
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_event_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(RuntimeReadToolRequest::InspectEventLog { query })
}

fn authorization_decision_query(
    arguments: &JsonValue,
) -> Result<RuntimeAuthorizationDecisionQuery, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = RuntimeAuthorizationDecisionQuery::new();
    if let Some(principal_id) = optional_string(arguments, "principal_id")? {
        query = query.for_principal(AgentId::trusted(principal_id));
    }
    if let Some(outcome) = optional_string(arguments, "outcome")? {
        query = query.with_outcome(parse_authorization_outcome(&outcome)?);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_authorization_decision_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(query)
}

fn capability_grant_query(
    arguments: &JsonValue,
) -> Result<RuntimeCapabilityGrantQuery, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = RuntimeCapabilityGrantQuery::new();
    if let Some(principal_id) = optional_string(arguments, "principal_id")? {
        query = query.for_principal(AgentId::trusted(principal_id));
    }
    if let Some(status) = optional_string(arguments, "status")? {
        query = query.with_status(parse_capability_grant_status(&status)?);
    }
    if let Some(scope_kind) = optional_string(arguments, "scope_kind")? {
        query = query.with_scope_kind(parse_capability_grant_scope_kind(&scope_kind)?);
    }
    if let Some(capability_id) = optional_string(arguments, "capability_id")? {
        query = query.with_capability(CapabilityId::trusted(capability_id));
    }
    if let Some(entity_id) = optional_string(arguments, "entity_id")? {
        query = query.for_entity(EntityId::trusted(entity_id));
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_capability_grant_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(query)
}

fn list_desired_states_request(
    arguments: &JsonValue,
) -> Result<RuntimeReadToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = DesiredStateQuery::new();
    if let Some(entity_id) = optional_string(arguments, "entity_id")? {
        query = query.for_entity(EntityId::trusted(entity_id));
    }
    if let Some(requested_by) = optional_string(arguments, "requested_by")? {
        query = query.requested_by(requested_by);
    }
    if let Some(capability_id) = optional_string(arguments, "capability_id")? {
        query = query.with_capability(CapabilityId::trusted(capability_id));
    }
    if let Some(min_command_timeout_ms) = optional_u64(arguments, "min_command_timeout_ms")? {
        query = query.min_command_timeout(min_command_timeout_ms);
    }
    if let Some(max_command_timeout_ms) = optional_u64(arguments, "max_command_timeout_ms")? {
        query = query.max_command_timeout(max_command_timeout_ms);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_desired_state_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(RuntimeReadToolRequest::ListDesiredStates { query })
}

fn set_desired_state_request(
    arguments: &JsonValue,
    principal_id: &AgentId,
) -> Result<RuntimeSetDesiredStateToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    let entity_id = EntityId::trusted(required_string(arguments, "entity_id")?);
    let mut desired_state = DesiredEntityState::new(
        entity_id,
        desired_state_deltas(required_field(arguments, "desired")?)?,
    );
    if let Some(requested_by) = optional_string(arguments, "requested_by")? {
        desired_state = desired_state.requested_by(requested_by);
    } else {
        desired_state = desired_state.requested_by(principal_id.as_str());
    }
    if let Some(command_timeout_ms) = optional_u64(arguments, "command_timeout_ms")? {
        desired_state = desired_state.with_command_timeout(command_timeout_ms);
    }
    Ok(RuntimeSetDesiredStateToolRequest::new(desired_state))
}

fn clear_desired_state_request(
    arguments: &JsonValue,
) -> Result<RuntimeClearDesiredStateToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    Ok(RuntimeClearDesiredStateToolRequest::new(EntityId::trusted(
        required_string(arguments, "entity_id")?,
    )))
}

fn desired_state_deltas(value: &JsonValue) -> Result<Vec<StateDelta>, ToolCallError> {
    match value {
        JsonValue::Array(values) => {
            if values.is_empty() {
                return Err(validation_error("desired must contain at least one delta"));
            }
            values.iter().map(desired_state_delta).collect()
        }
        JsonValue::Object(_) => Ok(vec![desired_state_delta(value)?]),
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) | JsonValue::String(_) => {
            Err(validation_error("desired must be an object or array"))
        }
    }
}

fn desired_state_delta(value: &JsonValue) -> Result<StateDelta, ToolCallError> {
    let _ = expect_object(value)?;
    Ok(StateDelta {
        capability_id: CapabilityId::trusted(required_string(value, "capability_id")?),
        value: json_to_smart_value(required_field(value, "value")?)?,
    })
}

fn list_pairing_sessions_request(
    arguments: &JsonValue,
) -> Result<RuntimeReadToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = RuntimePairingSessionQuery::new();
    if let Some(session_id) = optional_string(arguments, "session_id")? {
        query = query.for_session(RuntimePairingSessionId::trusted(session_id));
    }
    if let Some(bridge_id) = optional_string(arguments, "bridge_id")? {
        query = query.for_bridge(BridgeId::trusted(bridge_id));
    }
    if let Some(integration_id) = optional_string(arguments, "integration_id")? {
        query = query.for_integration(IntegrationId::trusted(integration_id));
    }
    if let Some(requested_by) = optional_string(arguments, "requested_by")? {
        query = query.requested_by(AgentId::trusted(requested_by));
    }
    for status in optional_string_list(arguments, "status", "statuses")? {
        query = query.with_status(parse_pairing_status(&status)?);
    }
    if let Some(expires_before_ms) = optional_u64(arguments, "expires_before_ms")? {
        query = query.expires_before(expires_before_ms);
    }
    if let Some(expiring_at_ms) = optional_u64(arguments, "expiring_at_ms")? {
        query = query.expiring_at(expiring_at_ms);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_pairing_session_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(RuntimeReadToolRequest::ListPairingSessions { query })
}

fn list_workers_request(arguments: &JsonValue) -> Result<RuntimeReadToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = SupervisedWorkerQuery::new();
    if let Some(bridge_id) = optional_string(arguments, "bridge_id")? {
        query = query.for_bridge(BridgeId::trusted(bridge_id));
    }
    if let Some(integration_id) = optional_string(arguments, "integration_id")? {
        query = query.for_integration(IntegrationId::trusted(integration_id));
    }
    for status in optional_string_list(arguments, "status", "statuses")? {
        query = query.with_status(parse_worker_status(&status)?);
    }
    if let Some(heartbeat_due_before_ms) = optional_u64(arguments, "heartbeat_due_before_ms")? {
        query = query.heartbeat_due_before(heartbeat_due_before_ms);
    }
    if let Some(overdue_at_ms) = optional_u64(arguments, "overdue_at_ms")? {
        query = query.overdue_at(overdue_at_ms);
    }
    if let Some(min_restart_count) = optional_u64(arguments, "min_restart_count")? {
        query = query.min_restart_count(u32::try_from(min_restart_count).map_err(|_| {
            validation_error("min_restart_count must be less than or equal to 4294967295")
        })?);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_supervised_worker_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.with_limit(limit as usize);
    }
    Ok(RuntimeReadToolRequest::ListWorkers { query })
}

fn get_worker_heartbeat_schedule_request(
    arguments: &JsonValue,
) -> Result<RuntimeReadToolRequest, ToolCallError> {
    let _ = expect_object(arguments)?;
    Ok(RuntimeReadToolRequest::GetWorkerHeartbeatSchedule {
        bridge_id: optional_string(arguments, "bridge_id")?.map(BridgeId::trusted),
        due_at_or_before_ms: optional_u64(arguments, "due_at_or_before_ms")?,
        limit: optional_u64(arguments, "limit")?.map(|value| value as usize),
    })
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

fn complete_pairing_request(
    arguments: &JsonValue,
    now_ms: u64,
) -> Result<RuntimeCompletePairingToolRequest, ToolCallError> {
    let mut request = RuntimeCompletePairingToolRequest::new(
        RuntimePairingSessionId::trusted(required_string(arguments, "session_id")?),
        VaultRef::trusted(required_string(arguments, "vault_ref")?),
        optional_u64(arguments, "completed_at_ms")?.unwrap_or(now_ms),
    );
    let metadata = optional_metadata(arguments)?;
    if !metadata.is_empty() {
        request = request.with_metadata(metadata);
    }
    Ok(request)
}

fn integration_catalog_query(
    arguments: &JsonValue,
) -> Result<IntegrationCatalogQuery, ToolCallError> {
    let _ = expect_object(arguments)?;
    let mut query = IntegrationCatalogQuery::new();

    for category in optional_string_list(arguments, "category", "categories")? {
        query = query.with_category(parse_integration_category(&category)?);
    }
    for connectivity in optional_string_list(arguments, "connectivity", "connectivity_classes")? {
        query = query.with_connectivity(parse_connectivity_class(&connectivity)?);
    }
    for status in optional_string_list(
        arguments,
        "implementation_status",
        "implementation_statuses",
    )? {
        query = query.with_status(parse_implementation_status(&status)?);
    }
    for primitive in optional_string_list(arguments, "required_primitive", "required_primitives")? {
        query = query.requiring_primitive(parse_primitive_family(&primitive)?);
    }
    for capability_id in optional_string_list(
        arguments,
        "required_capability_id",
        "required_capability_ids",
    )? {
        query = query.requiring_capability(CapabilityId::trusted(capability_id));
    }
    for surface in optional_string_list(arguments, "policy_surface", "policy_surfaces")? {
        query = query.with_policy_surface(parse_policy_surface(&surface)?);
    }
    for mechanism in optional_string_list(arguments, "discovery_mechanism", "discovery_mechanisms")?
    {
        query = query.with_discovery_mechanism(parse_discovery_mechanism(&mechanism)?);
    }
    for mode in optional_string_list(arguments, "auth_mode", "auth_modes")? {
        query = query.with_auth_mode(parse_auth_mode(&mode)?);
    }
    for protocol in optional_string_list(arguments, "protocol_family", "protocol_families")? {
        query = query.with_protocol_family(parse_protocol_family(&protocol)?);
    }
    if let Some(priority) = optional_u8(arguments, "priority_at_or_before")? {
        query = query.at_or_before_priority(priority);
    }
    if let Some(include_virtual_aliases) = optional_bool(arguments, "include_virtual_aliases")? {
        query = query.include_virtual_aliases(include_virtual_aliases);
    }
    if let Some(local_only) = optional_bool(arguments, "local_only")? {
        query = query.local_only(local_only);
    }
    if let Some(cloud_required) = optional_bool(arguments, "cloud_required")? {
        query = query.cloud_required(cloud_required);
    }
    if let Some(sort) = optional_string(arguments, "sort")? {
        query = query.sorted_by(parse_integration_catalog_sort(&sort)?);
    }
    if let Some(limit) = optional_u64(arguments, "limit")? {
        query = query.limited_to(limit as usize);
    }

    Ok(query)
}

fn list_integrations_output_handler_output(query: IntegrationCatalogQuery) -> ToolHandlerOutput {
    let catalog = first_party_catalog();
    let entries = query_integrations(&catalog, &query);
    let count = entries.len();
    ToolHandlerOutput::new(list_integrations_output_json(&catalog, entries)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("list_integrations")),
            ("count", integer(count as i64)),
        ]),
    )
}

fn describe_integration_output_handler_output(
    arguments: &JsonValue,
) -> Result<ToolHandlerOutput, ToolCallError> {
    let _ = expect_object(arguments)?;
    let integration_id = IntegrationId::trusted(required_string(arguments, "integration_id")?);
    let catalog = first_party_catalog();
    let entry = find_entry(&catalog, &integration_id).ok_or_else(|| {
        validation_error(format!(
            "unknown integration_id `{}`",
            integration_id.as_str()
        ))
    })?;
    let available_primitives = optional_primitive_list(arguments, "available_primitives")?;
    let allowed_capabilities = optional_capability_id_list(arguments, "allowed_capability_ids")?;
    let enabled_integrations = optional_integration_id_list(arguments, "enabled_integrations")?;
    let plan = activation_plan_for_entry(entry);
    let report = readiness_report_for_plan(
        &plan,
        &available_primitives,
        &allowed_capabilities,
        &enabled_integrations,
    );

    Ok(
        ToolHandlerOutput::new(describe_integration_output_json(entry, &plan, &report)).with_event(
            ToolEventKind::Progress,
            object([
                ("operation", string("describe_integration")),
                ("integration_id", string(integration_id.as_str())),
                (
                    "activation_ready",
                    JsonValue::Bool(report.activation_ready()),
                ),
            ]),
        ),
    )
}

fn list_primitives_output_handler_output(
    arguments: &JsonValue,
) -> Result<ToolHandlerOutput, ToolCallError> {
    let _ = expect_object(arguments)?;
    let priority = optional_u8(arguments, "priority_at_or_before")?.unwrap_or(u8::MAX);
    let include_ecosystem_coverage =
        optional_bool(arguments, "include_ecosystem_coverage")?.unwrap_or(true);
    let limit = optional_u64(arguments, "limit")?.map(|value| value as usize);
    let output = list_primitives_output_json(
        priority,
        include_ecosystem_coverage,
        limit.unwrap_or(usize::MAX),
    );
    let backlog_count = json_field(&output, "backlog_count")
        .and_then(json_integer)
        .unwrap_or(0);

    Ok(ToolHandlerOutput::new(output).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("list_primitives")),
            ("backlog_count", integer(backlog_count)),
        ]),
    ))
}

fn describe_primitive_output_handler_output(
    arguments: &JsonValue,
) -> Result<ToolHandlerOutput, ToolCallError> {
    let _ = expect_object(arguments)?;
    let primitive = parse_primitive_family(&required_string(arguments, "primitive")?)?;
    let priority = optional_u8(arguments, "priority_at_or_before")?.unwrap_or(u8::MAX);
    let output = describe_primitive_output_json(primitive, priority);
    let integration_count = json_field(&output, "integration_count")
        .and_then(json_integer)
        .unwrap_or(0);

    Ok(ToolHandlerOutput::new(output).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("describe_primitive")),
            ("primitive", string(primitive.as_str())),
            ("integration_count", integer(integration_count)),
        ]),
    ))
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

fn poll_events_output_handler_output(output: RuntimePollEventsToolOutput) -> ToolHandlerOutput {
    let summary = output.batch.summary();
    ToolHandlerOutput::new(poll_events_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("poll_events")),
            (
                "subscription_id",
                string(output.batch.subscription_id.as_str()),
            ),
            ("delivered_events", integer(summary.delivered_events as i64)),
            ("remaining_events", integer(summary.remaining_events as i64)),
        ]),
    )
}

fn unsubscribe_output_handler_output(output: RuntimeUnsubscribeToolOutput) -> ToolHandlerOutput {
    let summary = output.batch.summary();
    ToolHandlerOutput::new(unsubscribe_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("unsubscribe")),
            (
                "subscription_id",
                string(output.batch.subscription_id.as_str()),
            ),
            ("delivered_events", integer(summary.delivered_events as i64)),
            ("remaining_events", integer(summary.remaining_events as i64)),
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

fn complete_pairing_output_handler_output(
    output: RuntimeCompletePairingToolOutput,
) -> ToolHandlerOutput {
    ToolHandlerOutput::new(complete_pairing_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("complete_pairing")),
            ("session_id", string(output.session.session_id.as_str())),
            (
                "status",
                string(pairing_status_label(output.session.status)),
            ),
            (
                "vault_ref",
                output
                    .session
                    .vault_ref
                    .as_ref()
                    .map(|vault_ref| string(vault_ref.as_str()))
                    .unwrap_or(JsonValue::Null),
            ),
        ]),
    )
}

fn report_event_output_handler_output(output: RuntimeReportEventToolOutput) -> ToolHandlerOutput {
    let payload = report_event_output_json(&output);
    ToolHandlerOutput::new(payload).with_event(
        ToolEventKind::Progress,
        match &output {
            RuntimeReportEventToolOutput::Device(event) => object([
                ("operation", string("report_event")),
                ("event_kind", string("device")),
                ("event_id", string(event.event_id.as_str())),
                ("bridge_id", string(event.bridge_id.as_str())),
                (
                    "event_type",
                    string(device_event_type_label(event.event_type)),
                ),
            ]),
            RuntimeReportEventToolOutput::BridgeHealth(report) => object([
                ("operation", string("report_event")),
                ("event_kind", string("bridge_health")),
                ("event_id", string(report.event_id.as_str())),
                ("bridge_id", string(report.bridge_id.as_str())),
                ("health", string(health_label(report.health))),
            ]),
        },
    )
}

fn set_desired_state_output_handler_output(
    output: RuntimeSetDesiredStateToolOutput,
) -> ToolHandlerOutput {
    ToolHandlerOutput::new(set_desired_state_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("set_desired_state")),
            ("entity_id", string(output.desired_state.entity_id.as_str())),
            ("replaced", JsonValue::Bool(output.replaced)),
            (
                "desired_capability_count",
                integer(output.desired_state.desired.len() as i64),
            ),
        ]),
    )
}

fn clear_desired_state_output_handler_output(
    output: RuntimeClearDesiredStateToolOutput,
) -> ToolHandlerOutput {
    ToolHandlerOutput::new(clear_desired_state_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string("clear_desired_state")),
            ("entity_id", string(output.entity_id.as_str())),
            ("removed", JsonValue::Bool(output.removed())),
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

fn supervision_tool_output_handler_output(
    output: RuntimeSupervisionToolOutput,
    operation: &'static str,
) -> ToolHandlerOutput {
    let action_count = supervision_tool_action_count(&output);
    ToolHandlerOutput::new(supervision_tool_output_json(&output)).with_event(
        ToolEventKind::Progress,
        object([
            ("operation", string(operation)),
            ("action_count", integer(action_count as i64)),
            ("is_idle", JsonValue::Bool(action_count == 0)),
        ]),
    )
}

fn supervision_tool_action_count(output: &RuntimeSupervisionToolOutput) -> usize {
    match output {
        RuntimeSupervisionToolOutput::DesiredStateReconciliation { actions, .. } => actions.len(),
        RuntimeSupervisionToolOutput::SupervisionTick(report) => report.action_count(),
    }
}

fn supervision_tool_output_json(output: &RuntimeSupervisionToolOutput) -> JsonValue {
    match output {
        RuntimeSupervisionToolOutput::DesiredStateReconciliation {
            reconciled_at_ms,
            actions,
        } => object([
            ("reconciled_at_ms", integer(*reconciled_at_ms as i64)),
            ("action_count", integer(actions.len() as i64)),
            ("summary", desired_state_action_summary_json(actions)),
            (
                "actions",
                JsonValue::Array(actions.iter().map(desired_state_action_json).collect()),
            ),
        ]),
        RuntimeSupervisionToolOutput::SupervisionTick(report) => {
            supervision_tick_report_json(report)
        }
    }
}

fn read_output_json(output: RuntimeReadToolOutput) -> JsonValue {
    match output {
        RuntimeReadToolOutput::RuntimeSnapshot(snapshot) => runtime_read_snapshot_json(&snapshot),
        RuntimeReadToolOutput::DiscoveryWorkers { workers, summary } => object([
            (
                "workers",
                JsonValue::Array(workers.iter().map(discovery_worker_snapshot_json).collect()),
            ),
            ("summary", discovery_worker_scheduler_summary_json(&summary)),
            ("count", integer(workers.len() as i64)),
        ]),
        RuntimeReadToolOutput::DiscoverySummary {
            generated_at_ms,
            ttl_ms,
            record_summary,
            signal_summary,
        } => object([
            ("generated_at_ms", integer(generated_at_ms as i64)),
            ("ttl_ms", integer(ttl_ms as i64)),
            (
                "record_summary",
                discovery_record_summary_json(&record_summary),
            ),
            (
                "signal_summary",
                discovery_signal_summary_json(&signal_summary),
            ),
        ]),
        RuntimeReadToolOutput::PairingPlan {
            ttl_ms,
            plan,
            summary,
        } => pairing_plan_output_json(ttl_ms, &plan, &summary),
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
        RuntimeReadToolOutput::Rooms { rooms, topology } => object([
            (
                "rooms",
                JsonValue::Array(rooms.iter().map(room_summary_json).collect()),
            ),
            ("topology", topology_summary_json(&topology)),
            ("count", integer(rooms.len() as i64)),
        ]),
        RuntimeReadToolOutput::Scenes(scenes) => object([
            (
                "scenes",
                JsonValue::Array(scenes.iter().map(scene_json).collect()),
            ),
            ("count", integer(scenes.len() as i64)),
        ]),
        RuntimeReadToolOutput::Scene { scene_id, scene } => object([
            ("scene_id", string(scene_id.as_str())),
            ("scene", scene_json(&scene)),
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
        RuntimeReadToolOutput::Subscriptions {
            subscriptions,
            summary,
        } => object([
            (
                "subscriptions",
                JsonValue::Array(
                    subscriptions
                        .iter()
                        .map(subscription_snapshot_json)
                        .collect(),
                ),
            ),
            ("summary", subscription_inventory_summary_json(&summary)),
            ("count", integer(subscriptions.len() as i64)),
        ]),
        RuntimeReadToolOutput::EventLog { entries, summary } => object([
            (
                "events",
                JsonValue::Array(entries.iter().map(event_log_record_json).collect()),
            ),
            ("summary", event_log_summary_json(&summary)),
            ("count", integer(entries.len() as i64)),
        ]),
        RuntimeReadToolOutput::AuthorizationDecisions { decisions, summary } => object([
            (
                "decisions",
                JsonValue::Array(decisions.iter().map(authorization_decision_json).collect()),
            ),
            ("summary", authorization_decision_log_summary_json(&summary)),
            ("count", integer(decisions.len() as i64)),
        ]),
        RuntimeReadToolOutput::AuthorizationSummary { summary } => {
            object([("summary", authorization_decision_log_summary_json(&summary))])
        }
        RuntimeReadToolOutput::CapabilityGrants { grants, summary } => object([
            (
                "grants",
                JsonValue::Array(
                    grants
                        .iter()
                        .map(|grant| capability_grant_json(grant, summary.generated_at_ms))
                        .collect(),
                ),
            ),
            ("summary", capability_grant_inventory_summary_json(&summary)),
            ("count", integer(grants.len() as i64)),
        ]),
        RuntimeReadToolOutput::CapabilityGrantSummary { summary } => {
            object([("summary", capability_grant_inventory_summary_json(&summary))])
        }
        RuntimeReadToolOutput::TopologySummary { summary } => {
            object([("summary", topology_summary_json(&summary))])
        }
        RuntimeReadToolOutput::DesiredStates {
            desired_states,
            summary,
        } => object([
            (
                "desired_states",
                JsonValue::Array(desired_states.iter().map(desired_state_json).collect()),
            ),
            ("summary", desired_state_inventory_summary_json(&summary)),
            ("count", integer(desired_states.len() as i64)),
        ]),
        RuntimeReadToolOutput::PairingSessions { sessions, summary } => object([
            (
                "sessions",
                JsonValue::Array(sessions.iter().map(pairing_session_json).collect()),
            ),
            ("summary", pairing_session_inventory_summary_json(&summary)),
            ("count", integer(sessions.len() as i64)),
        ]),
        RuntimeReadToolOutput::Workers { workers, summary } => object([
            (
                "workers",
                JsonValue::Array(
                    workers
                        .iter()
                        .map(|worker| supervised_worker_json(worker, summary.generated_at_ms))
                        .collect(),
                ),
            ),
            ("summary", supervisor_snapshot_json(&summary)),
            ("count", integer(workers.len() as i64)),
        ]),
        RuntimeReadToolOutput::WorkerHeartbeatSchedule(schedule) => {
            worker_heartbeat_schedule_json(&schedule)
        }
        RuntimeReadToolOutput::SupervisionPlan(plan) => runtime_supervision_plan_json(&plan),
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

fn poll_events_output_json(output: &RuntimePollEventsToolOutput) -> JsonValue {
    event_delivery_batch_json(&output.batch)
}

fn unsubscribe_output_json(output: &RuntimeUnsubscribeToolOutput) -> JsonValue {
    let batch = &output.batch;
    object([
        ("subscription_id", string(batch.subscription_id.as_str())),
        ("unsubscribed", JsonValue::Bool(true)),
        ("delivered_events", integer(batch.len() as i64)),
        ("remaining_events", integer(batch.remaining_events as i64)),
        ("has_more", JsonValue::Bool(batch.has_more())),
        ("summary", event_delivery_summary_json(batch)),
        (
            "events",
            JsonValue::Array(batch.events.iter().map(runtime_event_json).collect()),
        ),
    ])
}

fn pair_bridge_output_json(output: &RuntimePairBridgeToolOutput) -> JsonValue {
    pairing_session_json(&output.session)
}

fn complete_pairing_output_json(output: &RuntimeCompletePairingToolOutput) -> JsonValue {
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

fn discovery_record_summary_json(summary: &DiscoveryRecordSummary) -> JsonValue {
    object([
        ("total", integer(summary.total as i64)),
        ("with_address", integer(summary.with_address as i64)),
        ("fresh", integer(summary.fresh as i64)),
        ("stale", integer(summary.stale as i64)),
        ("expired", integer(summary.expired as i64)),
        ("is_empty", JsonValue::Bool(summary.is_empty())),
        (
            "by_source",
            JsonValue::Array(
                summary
                    .by_source
                    .iter()
                    .map(|(source, count)| {
                        object([
                            ("source", string(source.as_str())),
                            ("count", integer(*count as i64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "by_confidence",
            JsonValue::Array(
                summary
                    .by_confidence
                    .iter()
                    .map(|(confidence, count)| {
                        object([
                            ("confidence", string(confidence.as_str())),
                            ("count", integer(*count as i64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "by_pairing_requirement",
            JsonValue::Array(
                summary
                    .by_pairing_requirement
                    .iter()
                    .map(|(requirement, count)| {
                        object([
                            ("pairing_requirement", string(requirement.as_str())),
                            ("count", integer(*count as i64)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

fn discovery_signal_summary_json(summary: &DiscoverySignalSummary) -> JsonValue {
    object([
        ("fresh", integer(summary.fresh as i64)),
        ("stale", integer(summary.stale as i64)),
        ("expired", integer(summary.expired as i64)),
        (
            "next_transition_at_ms",
            summary
                .next_transition_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "has_stale_or_expired_signals",
            JsonValue::Bool(summary.stale > 0 || summary.expired > 0),
        ),
    ])
}

fn pairing_plan_output_json(
    ttl_ms: u64,
    plan: &DiscoveryPairingPlan,
    summary: &DiscoveryPairingPlanSummary,
) -> JsonValue {
    object([
        ("generated_at_ms", integer(plan.generated_at_ms as i64)),
        ("ttl_ms", integer(ttl_ms as i64)),
        (
            "targets",
            JsonValue::Array(plan.targets.iter().map(pairing_target_json).collect()),
        ),
        ("summary", pairing_plan_summary_json(summary)),
        ("count", integer(plan.targets.len() as i64)),
    ])
}

fn pairing_target_json(target: &DiscoveryPairingTarget) -> JsonValue {
    object([
        ("fingerprint", string(target.fingerprint.as_str())),
        ("bridge_id", string(target.bridge_id.as_str())),
        ("integration_id", string(target.integration_id.as_str())),
        ("native_bridge_id", string(&target.native_bridge_id)),
        ("display_name", optional_string_json(&target.display_name)),
        ("priority", integer(target.priority as i64)),
        ("source", string(target.source.as_str())),
        ("confidence", string(target.confidence.as_str())),
        ("signal_status", string(target.signal_status.as_str())),
        (
            "pairing_requirement",
            string(target.pairing_requirement.as_str()),
        ),
        ("action", string(target.action.as_str())),
        (
            "requires_human_action",
            JsonValue::Bool(target.requires_human_action()),
        ),
        ("is_actionable", JsonValue::Bool(target.is_actionable())),
        ("address", optional_string_json(&target.address)),
        ("discovered_at_ms", integer(target.discovered_at_ms as i64)),
    ])
}

fn pairing_plan_summary_json(summary: &DiscoveryPairingPlanSummary) -> JsonValue {
    object([
        ("generated_at_ms", integer(summary.generated_at_ms as i64)),
        ("total", integer(summary.total as i64)),
        ("actionable", integer(summary.actionable as i64)),
        ("ready", integer(summary.ready as i64)),
        (
            "requires_human_action",
            integer(summary.requires_human_action as i64),
        ),
        (
            "blocked_unknown_requirement",
            integer(summary.blocked_unknown_requirement as i64),
        ),
        ("fresh", integer(summary.fresh as i64)),
        ("stale", integer(summary.stale as i64)),
        ("is_empty", JsonValue::Bool(summary.is_empty())),
        (
            "by_source",
            JsonValue::Array(
                summary
                    .by_source
                    .iter()
                    .map(|(source, count)| {
                        object([
                            ("source", string(source.as_str())),
                            ("count", integer(*count as i64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "by_pairing_requirement",
            JsonValue::Array(
                summary
                    .by_pairing_requirement
                    .iter()
                    .map(|(requirement, count)| {
                        object([
                            ("pairing_requirement", string(requirement.as_str())),
                            ("count", integer(*count as i64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "by_action",
            JsonValue::Array(
                summary
                    .by_action
                    .iter()
                    .map(|(action, count)| {
                        object([
                            ("action", string(action.as_str())),
                            ("count", integer(*count as i64)),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "next_actionable_target",
            summary
                .next_actionable_target
                .as_ref()
                .map(pairing_target_json)
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn discovery_worker_scheduler_summary_json(
    summary: &DiscoveryWorkerSchedulerSnapshot,
) -> JsonValue {
    object([
        ("generated_at_ms", integer(summary.generated_at_ms as i64)),
        ("worker_count", integer(summary.worker_count as i64)),
        ("due_worker_count", integer(summary.due_worker_count as i64)),
        ("starting_count", integer(summary.starting_count as i64)),
        ("running_count", integer(summary.running_count as i64)),
        ("unhealthy_count", integer(summary.unhealthy_count as i64)),
        ("restarting_count", integer(summary.restarting_count as i64)),
        ("stopped_count", integer(summary.stopped_count as i64)),
        (
            "workers_with_failures",
            integer(summary.workers_with_failures as i64),
        ),
        ("has_due_work", JsonValue::Bool(summary.has_due_work())),
        (
            "has_worker_pressure",
            JsonValue::Bool(summary.has_worker_pressure()),
        ),
    ])
}

fn runtime_read_snapshot_json(snapshot: &RuntimeReadSnapshot) -> JsonValue {
    object([
        ("generated_at_ms", integer(snapshot.generated_at_ms as i64)),
        (
            "registry_counts",
            object([
                ("bridges", integer(snapshot.registry_counts.bridges as i64)),
                ("devices", integer(snapshot.registry_counts.devices as i64)),
                (
                    "entities",
                    integer(snapshot.registry_counts.entities as i64),
                ),
                ("scenes", integer(snapshot.registry_counts.scenes as i64)),
                ("states", integer(snapshot.registry_counts.states as i64)),
                ("events", integer(snapshot.registry_counts.events as i64)),
                (
                    "protocol_identifiers",
                    integer(snapshot.registry_counts.protocol_identifiers as i64),
                ),
                (
                    "capability_grants",
                    integer(snapshot.registry_counts.capability_grants as i64),
                ),
                (
                    "authorization_decisions",
                    integer(snapshot.registry_counts.authorization_decisions as i64),
                ),
            ]),
        ),
        (
            "discovery_record_count",
            integer(snapshot.discovery_record_count as i64),
        ),
        (
            "discovery_scheduler",
            object([
                (
                    "generated_at_ms",
                    integer(snapshot.discovery_scheduler.generated_at_ms as i64),
                ),
                (
                    "worker_count",
                    integer(snapshot.discovery_scheduler.worker_count as i64),
                ),
                (
                    "due_worker_count",
                    integer(snapshot.discovery_scheduler.due_worker_count as i64),
                ),
                (
                    "starting_count",
                    integer(snapshot.discovery_scheduler.starting_count as i64),
                ),
                (
                    "running_count",
                    integer(snapshot.discovery_scheduler.running_count as i64),
                ),
                (
                    "unhealthy_count",
                    integer(snapshot.discovery_scheduler.unhealthy_count as i64),
                ),
                (
                    "restarting_count",
                    integer(snapshot.discovery_scheduler.restarting_count as i64),
                ),
                (
                    "stopped_count",
                    integer(snapshot.discovery_scheduler.stopped_count as i64),
                ),
                (
                    "workers_with_failures",
                    integer(snapshot.discovery_scheduler.workers_with_failures as i64),
                ),
                (
                    "has_due_work",
                    JsonValue::Bool(snapshot.discovery_scheduler.has_due_work()),
                ),
                (
                    "has_worker_pressure",
                    JsonValue::Bool(snapshot.discovery_scheduler.has_worker_pressure()),
                ),
            ]),
        ),
        (
            "event_bus",
            object([
                (
                    "subscription_count",
                    integer(snapshot.event_bus.subscription_count as i64),
                ),
                (
                    "pending_delivery_count",
                    integer(snapshot.event_bus.pending_delivery_count as i64),
                ),
                (
                    "published_event_count",
                    integer(snapshot.event_bus.published_event_count as i64),
                ),
                (
                    "backlogged_subscription_count",
                    integer(snapshot.event_bus.backlogged_subscription_count as i64),
                ),
                (
                    "max_pending_delivery_count",
                    integer(snapshot.event_bus.max_pending_delivery_count as i64),
                ),
                (
                    "average_pending_deliveries_per_subscription",
                    integer(
                        snapshot
                            .event_bus
                            .average_pending_deliveries_per_subscription()
                            as i64,
                    ),
                ),
                (
                    "has_backlog",
                    JsonValue::Bool(snapshot.event_bus.has_backlog()),
                ),
                (
                    "has_lagging_subscriptions",
                    JsonValue::Bool(snapshot.event_bus.has_lagging_subscriptions()),
                ),
            ]),
        ),
        ("supervisor", supervisor_snapshot_json(&snapshot.supervisor)),
        (
            "pairing_session_count",
            integer(snapshot.pairing_session_count as i64),
        ),
        (
            "expiring_pairing_session_count",
            integer(snapshot.expiring_pairing_session_count as i64),
        ),
        (
            "optimistic_state_count",
            integer(snapshot.optimistic_state_count as i64),
        ),
        (
            "stale_optimistic_state_count",
            integer(snapshot.stale_optimistic_state_count as i64),
        ),
        (
            "desired_state_count",
            integer(snapshot.desired_state_count as i64),
        ),
        (
            "desired_capability_count",
            integer(snapshot.desired_capability_count as i64),
        ),
        (
            "state_refresh_target_count",
            integer(snapshot.state_refresh_target_count as i64),
        ),
        (
            "pending_work",
            pending_work_summary_json(&snapshot.pending_work_summary()),
        ),
        (
            "has_pending_work",
            JsonValue::Bool(snapshot.has_pending_work()),
        ),
    ])
}

fn room_summary_json(room: &RuntimeRoomSummary) -> JsonValue {
    object([
        ("room_id", string(&room.room_id)),
        ("device_count", integer(room.device_count as i64)),
        ("online_devices", integer(room.online_devices as i64)),
        (
            "pairing_candidate_devices",
            integer(room.pairing_candidate_devices as i64),
        ),
        ("attention_devices", integer(room.attention_devices as i64)),
        ("entity_count", integer(room.entity_count as i64)),
        (
            "commandable_entities",
            integer(room.commandable_entities as i64),
        ),
        (
            "entities_with_state",
            integer(room.entities_with_state as i64),
        ),
        (
            "entities_without_state",
            integer(room.entities_without_state as i64),
        ),
        ("stale_entities", integer(room.stale_entities as i64)),
        ("state_gap_count", integer(room.state_gap_count() as i64)),
        ("scene_count", integer(room.scene_count as i64)),
        (
            "scene_action_count",
            integer(room.scene_action_count as i64),
        ),
        (
            "has_attention_items",
            JsonValue::Bool(room.has_attention_items()),
        ),
        ("has_state_gaps", JsonValue::Bool(room.has_state_gaps())),
        (
            "has_scene_actions",
            JsonValue::Bool(room.has_scene_actions()),
        ),
    ])
}

fn topology_summary_json(summary: &RegistryTopologySummary) -> JsonValue {
    object([
        ("bridges", integer(summary.bridges as i64)),
        ("devices", integer(summary.devices as i64)),
        ("entities", integer(summary.entities as i64)),
        ("scenes", integer(summary.scenes as i64)),
        ("lan_http_bridges", integer(summary.lan_http_bridges as i64)),
        ("mdns_bridges", integer(summary.mdns_bridges as i64)),
        ("serial_bridges", integer(summary.serial_bridges as i64)),
        ("ble_bridges", integer(summary.ble_bridges as i64)),
        ("cloud_bridges", integer(summary.cloud_bridges as i64)),
        (
            "local_process_bridges",
            integer(summary.local_process_bridges as i64),
        ),
        ("online_bridges", integer(summary.online_bridges as i64)),
        (
            "pairing_candidate_bridges",
            integer(summary.pairing_candidate_bridges as i64),
        ),
        (
            "attention_bridges",
            integer(summary.attention_bridges as i64),
        ),
        ("online_devices", integer(summary.online_devices as i64)),
        (
            "pairing_candidate_devices",
            integer(summary.pairing_candidate_devices as i64),
        ),
        (
            "attention_devices",
            integer(summary.attention_devices as i64),
        ),
        (
            "devices_with_entities",
            integer(summary.devices_with_entities as i64),
        ),
        (
            "devices_without_entities",
            integer(summary.devices_without_entities as i64),
        ),
        (
            "devices_with_room",
            integer(summary.devices_with_room as i64),
        ),
        (
            "devices_without_room",
            integer(summary.devices_without_room as i64),
        ),
        ("unique_rooms", integer(summary.unique_rooms as i64)),
        ("light_entities", integer(summary.light_entities as i64)),
        (
            "light_group_entities",
            integer(summary.light_group_entities as i64),
        ),
        ("switch_entities", integer(summary.switch_entities as i64)),
        ("sensor_entities", integer(summary.sensor_entities as i64)),
        ("lock_entities", integer(summary.lock_entities as i64)),
        (
            "thermostat_entities",
            integer(summary.thermostat_entities as i64),
        ),
        ("scene_entities", integer(summary.scene_entities as i64)),
        ("input_entities", integer(summary.input_entities as i64)),
        (
            "bridge_health_entities",
            integer(summary.bridge_health_entities as i64),
        ),
        (
            "network_diagnostic_entities",
            integer(summary.network_diagnostic_entities as i64),
        ),
        ("unknown_entities", integer(summary.unknown_entities as i64)),
        (
            "entities_with_state",
            integer(summary.entities_with_state as i64),
        ),
        (
            "entities_without_state",
            integer(summary.entities_without_state as i64),
        ),
        (
            "total_capabilities",
            integer(summary.total_capabilities as i64),
        ),
        ("room_scenes", integer(summary.room_scenes as i64)),
        ("zone_scenes", integer(summary.zone_scenes as i64)),
        ("home_scenes", integer(summary.home_scenes as i64)),
        ("bridge_scenes", integer(summary.bridge_scenes as i64)),
        ("custom_scenes", integer(summary.custom_scenes as i64)),
        ("scene_actions", integer(summary.scene_actions as i64)),
        ("has_topology", JsonValue::Bool(summary.has_topology())),
        (
            "has_pairing_candidates",
            JsonValue::Bool(summary.has_pairing_candidates()),
        ),
        (
            "has_attention_items",
            JsonValue::Bool(summary.has_attention_items()),
        ),
        (
            "has_devices_without_entities",
            JsonValue::Bool(summary.has_devices_without_entities()),
        ),
        ("has_state_gaps", JsonValue::Bool(summary.has_state_gaps())),
        (
            "has_scene_actions",
            JsonValue::Bool(summary.has_scene_actions()),
        ),
        (
            "has_multi_transport_bridges",
            JsonValue::Bool(summary.has_multi_transport_bridges()),
        ),
    ])
}

fn supervised_worker_json(worker: &SupervisedBridgeWorker, now_ms: u64) -> JsonValue {
    let heartbeat_due_at_ms = worker.heartbeat_due_at_ms();
    object([
        ("bridge_id", string(worker.bridge_id.as_str())),
        ("integration_id", string(worker.integration_id.as_str())),
        ("status", string(worker.status.as_str())),
        ("restart_count", integer(worker.restart_count as i64)),
        (
            "last_heartbeat_at_ms",
            integer(worker.last_heartbeat_at_ms as i64),
        ),
        (
            "heartbeat_timeout_ms",
            integer(worker.heartbeat_timeout_ms as i64),
        ),
        (
            "heartbeat_due_at_ms",
            heartbeat_due_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "has_heartbeat_deadline",
            JsonValue::Bool(heartbeat_due_at_ms.is_some()),
        ),
        ("is_overdue", JsonValue::Bool(worker.is_overdue_at(now_ms))),
        (
            "overdue_by_ms",
            heartbeat_due_at_ms
                .map(|due_at_ms| integer(now_ms.saturating_sub(due_at_ms) as i64))
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn supervisor_snapshot_json(snapshot: &RuntimeSupervisorSnapshot) -> JsonValue {
    object([
        ("generated_at_ms", integer(snapshot.generated_at_ms as i64)),
        ("worker_count", integer(snapshot.worker_count as i64)),
        ("starting_count", integer(snapshot.starting_count as i64)),
        ("running_count", integer(snapshot.running_count as i64)),
        ("unhealthy_count", integer(snapshot.unhealthy_count as i64)),
        (
            "restarting_count",
            integer(snapshot.restarting_count as i64),
        ),
        ("stopped_count", integer(snapshot.stopped_count as i64)),
        (
            "restart_due_count",
            integer(snapshot.restart_due_count as i64),
        ),
        (
            "has_restart_pressure",
            JsonValue::Bool(snapshot.has_restart_pressure()),
        ),
    ])
}

fn worker_heartbeat_schedule_json(schedule: &WorkerHeartbeatSchedule) -> JsonValue {
    object([
        ("generated_at_ms", integer(schedule.generated_at_ms as i64)),
        (
            "deadlines",
            JsonValue::Array(
                schedule
                    .deadlines
                    .iter()
                    .map(|deadline| {
                        worker_heartbeat_deadline_json(deadline, schedule.generated_at_ms)
                    })
                    .collect(),
            ),
        ),
        ("count", integer(schedule.len() as i64)),
        (
            "due_count",
            integer(schedule.due_at(schedule.generated_at_ms).len() as i64),
        ),
        (
            "next_due_at_ms",
            schedule
                .next_due_at_ms()
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        ("is_empty", JsonValue::Bool(schedule.is_empty())),
    ])
}

fn worker_heartbeat_deadline_json(deadline: &WorkerHeartbeatDeadline, now_ms: u64) -> JsonValue {
    object([
        ("bridge_id", string(deadline.bridge_id.as_str())),
        ("integration_id", string(deadline.integration_id.as_str())),
        ("status", string(deadline.status.as_str())),
        (
            "last_heartbeat_at_ms",
            integer(deadline.last_heartbeat_at_ms as i64),
        ),
        (
            "heartbeat_timeout_ms",
            integer(deadline.heartbeat_timeout_ms as i64),
        ),
        ("due_at_ms", integer(deadline.due_at_ms as i64)),
        ("is_due", JsonValue::Bool(deadline.is_due_at(now_ms))),
        (
            "overdue_by_ms",
            integer(deadline.overdue_by_ms_at(now_ms) as i64),
        ),
    ])
}

fn pending_work_summary_json(summary: &RuntimePendingWorkSummary) -> JsonValue {
    object([
        (
            "event_backlog_count",
            integer(summary.event_backlog_count as i64),
        ),
        (
            "backlogged_subscription_count",
            integer(summary.backlogged_subscription_count as i64),
        ),
        (
            "discovery_worker_due_count",
            integer(summary.discovery_worker_due_count as i64),
        ),
        (
            "unhealthy_discovery_worker_count",
            integer(summary.unhealthy_discovery_worker_count as i64),
        ),
        (
            "restart_due_count",
            integer(summary.restart_due_count as i64),
        ),
        (
            "unhealthy_worker_count",
            integer(summary.unhealthy_worker_count as i64),
        ),
        (
            "expiring_pairing_session_count",
            integer(summary.expiring_pairing_session_count as i64),
        ),
        (
            "stale_optimistic_state_count",
            integer(summary.stale_optimistic_state_count as i64),
        ),
        (
            "state_refresh_target_count",
            integer(summary.state_refresh_target_count as i64),
        ),
        (
            "total_pending_work_count",
            integer(summary.total_pending_work_count() as i64),
        ),
        ("is_idle", JsonValue::Bool(summary.is_idle())),
        (
            "has_event_backlog",
            JsonValue::Bool(summary.has_event_backlog()),
        ),
        (
            "has_supervision_pressure",
            JsonValue::Bool(summary.has_supervision_pressure()),
        ),
    ])
}

fn desired_state_json(desired_state: &DesiredEntityState) -> JsonValue {
    object([
        ("entity_id", string(desired_state.entity_id.as_str())),
        (
            "desired",
            JsonValue::Array(desired_state.desired.iter().map(state_delta_json).collect()),
        ),
        ("requested_by", string(&desired_state.requested_by)),
        (
            "command_timeout_ms",
            integer(desired_state.command_timeout_ms as i64),
        ),
        (
            "desired_capability_count",
            integer(desired_state.desired.len() as i64),
        ),
    ])
}

fn set_desired_state_output_json(output: &RuntimeSetDesiredStateToolOutput) -> JsonValue {
    object([
        ("entity_id", string(output.desired_state.entity_id.as_str())),
        ("desired_state", desired_state_json(&output.desired_state)),
        ("replaced", JsonValue::Bool(output.replaced)),
        (
            "previous",
            output
                .previous
                .as_ref()
                .map(desired_state_json)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "desired_capability_count",
            integer(output.desired_state.desired.len() as i64),
        ),
    ])
}

fn clear_desired_state_output_json(output: &RuntimeClearDesiredStateToolOutput) -> JsonValue {
    object([
        ("entity_id", string(output.entity_id.as_str())),
        ("removed", JsonValue::Bool(output.removed())),
        (
            "desired_state",
            output
                .removed
                .as_ref()
                .map(desired_state_json)
                .unwrap_or(JsonValue::Null),
        ),
    ])
}

fn desired_state_inventory_summary_json(summary: &DesiredStateInventorySummary) -> JsonValue {
    object([
        (
            "total_desired_states",
            integer(summary.total_desired_states as i64),
        ),
        (
            "total_desired_capabilities",
            integer(summary.total_desired_capabilities as i64),
        ),
        (
            "requested_by_count",
            integer(summary.requested_by_count as i64),
        ),
        (
            "min_command_timeout_ms",
            summary
                .min_command_timeout_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "max_command_timeout_ms",
            summary
                .max_command_timeout_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "has_desired_states",
            JsonValue::Bool(summary.has_desired_states()),
        ),
    ])
}

fn desired_state_action_summary_json(actions: &[DesiredStateAction]) -> JsonValue {
    let mut missing_state_count = 0;
    let mut stale_state_count = 0;
    let mut drifted_state_count = 0;
    for action in actions {
        let DesiredStateAction::CommandIssued { reason, .. } = action;
        match reason {
            ReconciliationReason::MissingState => missing_state_count += 1,
            ReconciliationReason::StaleState => stale_state_count += 1,
            ReconciliationReason::Drifted => drifted_state_count += 1,
        }
    }

    object([
        ("action_count", integer(actions.len() as i64)),
        ("missing_state_count", integer(missing_state_count)),
        ("stale_state_count", integer(stale_state_count)),
        ("drifted_state_count", integer(drifted_state_count)),
        ("is_idle", JsonValue::Bool(actions.is_empty())),
    ])
}

fn desired_state_action_json(action: &DesiredStateAction) -> JsonValue {
    match action {
        DesiredStateAction::CommandIssued {
            entity_id,
            capability_id,
            reason,
            command,
            result,
        } => object([
            ("action_type", string("command_issued")),
            ("entity_id", string(entity_id.as_str())),
            ("capability_id", string(capability_id.as_str())),
            ("reason", string(reconciliation_reason_label(*reason))),
            ("command", device_command_json(command)),
            ("result", command_result_json(result)),
        ]),
    }
}

fn device_command_json(command: &DeviceCommand) -> JsonValue {
    object([
        ("command_id", string(command.command_id.as_str())),
        ("entity_id", string(command.entity_id.as_str())),
        (
            "command_type",
            string(command_type_label(command.command_type)),
        ),
        ("arguments", smart_value_to_json(&command.arguments)),
        ("requested_by", string(&command.requested_by)),
        (
            "idempotency_key",
            command
                .idempotency_key
                .as_ref()
                .map(|value| string(value))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "required_capabilities",
            JsonValue::Array(
                command
                    .required_capabilities
                    .iter()
                    .map(|capability_id| string(capability_id.as_str()))
                    .collect(),
            ),
        ),
        ("timeout_ms", integer(command.timeout_ms as i64)),
        ("correlation_id", string(command.correlation_id.as_str())),
    ])
}

fn supervision_tick_report_json(report: &SupervisionTickReport) -> JsonValue {
    object([
        ("ticked_at_ms", integer(report.ticked_at_ms as i64)),
        ("is_idle", JsonValue::Bool(report.is_idle())),
        ("action_count", integer(report.action_count() as i64)),
        ("summary", supervision_tick_summary_json(report)),
        (
            "expired_pairing_sessions",
            JsonValue::Array(
                report
                    .expired_pairing_sessions
                    .iter()
                    .map(|session_id| string(session_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "expired_entities",
            JsonValue::Array(
                report
                    .expired_entities
                    .iter()
                    .map(|entity_id| string(entity_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "desired_state_actions",
            JsonValue::Array(
                report
                    .desired_state_actions
                    .iter()
                    .map(desired_state_action_json)
                    .collect(),
            ),
        ),
        (
            "worker_events",
            JsonValue::Array(
                report
                    .worker_events
                    .iter()
                    .map(runtime_event_json)
                    .collect(),
            ),
        ),
    ])
}

fn supervision_tick_summary_json(report: &SupervisionTickReport) -> JsonValue {
    let summary = report.summary();
    object([
        ("ticked_at_ms", integer(summary.ticked_at_ms as i64)),
        ("total_actions", integer(summary.total_actions as i64)),
        (
            "expired_pairing_session_count",
            integer(summary.expired_pairing_session_count as i64),
        ),
        (
            "expired_entity_count",
            integer(summary.expired_entity_count as i64),
        ),
        (
            "desired_state_action_count",
            integer(summary.desired_state_action_count as i64),
        ),
        (
            "desired_missing_state_count",
            integer(summary.desired_missing_state_count as i64),
        ),
        (
            "desired_stale_state_count",
            integer(summary.desired_stale_state_count as i64),
        ),
        (
            "desired_drifted_state_count",
            integer(summary.desired_drifted_state_count as i64),
        ),
        (
            "worker_restart_event_count",
            integer(summary.worker_restart_event_count as i64),
        ),
        ("is_idle", JsonValue::Bool(summary.is_idle())),
        (
            "has_pairing_expiry_work",
            JsonValue::Bool(summary.has_pairing_expiry_work()),
        ),
        (
            "has_state_expiry_work",
            JsonValue::Bool(summary.has_state_expiry_work()),
        ),
        (
            "has_reconciliation_work",
            JsonValue::Bool(summary.has_reconciliation_work()),
        ),
        (
            "has_worker_restart_work",
            JsonValue::Bool(summary.has_worker_restart_work()),
        ),
    ])
}

fn pairing_session_inventory_summary_json(
    summary: &RuntimePairingSessionInventorySummary,
) -> JsonValue {
    object([
        ("total_sessions", integer(summary.total_sessions as i64)),
        (
            "pending_user_presence_sessions",
            integer(summary.pending_user_presence_sessions as i64),
        ),
        (
            "completed_sessions",
            integer(summary.completed_sessions as i64),
        ),
        ("expired_sessions", integer(summary.expired_sessions as i64)),
        (
            "cancelled_sessions",
            integer(summary.cancelled_sessions as i64),
        ),
        (
            "expiring_sessions",
            integer(summary.expiring_sessions as i64),
        ),
        (
            "sessions_with_vault_ref",
            integer(summary.sessions_with_vault_ref as i64),
        ),
        (
            "has_pending_user_presence",
            JsonValue::Bool(summary.has_pending_user_presence()),
        ),
        (
            "has_expiring_sessions",
            JsonValue::Bool(summary.has_expiring_sessions()),
        ),
        (
            "has_completed_credentials",
            JsonValue::Bool(summary.has_completed_credentials()),
        ),
    ])
}

fn runtime_supervision_plan_json(plan: &RuntimeSupervisionPlan) -> JsonValue {
    object([
        ("generated_at_ms", integer(plan.generated_at_ms as i64)),
        ("summary", supervision_plan_summary_json(&plan.summary())),
        ("is_idle", JsonValue::Bool(plan.is_empty())),
        ("action_count", integer(plan.action_count() as i64)),
        (
            "pairing_sessions_expiring",
            JsonValue::Array(
                plan.pairing_sessions_expiring
                    .iter()
                    .map(|session_id| string(session_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "state_refresh_plan",
            object([
                (
                    "generated_at_ms",
                    integer(plan.state_refresh_plan.generated_at_ms as i64),
                ),
                (
                    "targets",
                    JsonValue::Array(
                        plan.state_refresh_plan
                            .targets
                            .iter()
                            .map(|target| {
                                object([
                                    ("bridge_id", string(target.bridge_id.as_str())),
                                    ("device_id", string(target.device_id.as_str())),
                                    ("entity_id", string(target.entity_id.as_str())),
                                    ("kind", string(entity_kind_label(target.kind))),
                                    (
                                        "capabilities",
                                        JsonValue::Array(
                                            target
                                                .capabilities
                                                .iter()
                                                .map(|capability_id| string(capability_id.as_str()))
                                                .collect(),
                                        ),
                                    ),
                                    ("reason", string(state_refresh_reason_label(target.reason))),
                                ])
                            })
                            .collect(),
                    ),
                ),
                (
                    "count",
                    integer(plan.state_refresh_plan.targets.len() as i64),
                ),
            ]),
        ),
        (
            "desired_state_drifts",
            JsonValue::Array(
                plan.desired_state_drifts
                    .iter()
                    .map(|drift| {
                        object([
                            ("bridge_id", string(drift.bridge_id.as_str())),
                            ("entity_id", string(drift.entity_id.as_str())),
                            ("capability_id", string(drift.capability_id.as_str())),
                            ("desired_value", smart_value_to_json(&drift.desired_value)),
                            ("reason", string(reconciliation_reason_label(drift.reason))),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "worker_restart_plan",
            object([
                (
                    "generated_at_ms",
                    integer(plan.worker_restart_plan.generated_at_ms as i64),
                ),
                (
                    "instructions",
                    JsonValue::Array(
                        plan.worker_restart_plan
                            .instructions
                            .iter()
                            .map(worker_restart_instruction_json)
                            .collect(),
                    ),
                ),
                (
                    "count",
                    integer(plan.worker_restart_plan.instructions.len() as i64),
                ),
            ]),
        ),
        (
            "discovery_worker_run_plan",
            object([
                (
                    "generated_at_ms",
                    integer(plan.discovery_worker_run_plan.generated_at_ms as i64),
                ),
                (
                    "instructions",
                    JsonValue::Array(
                        plan.discovery_worker_run_plan
                            .instructions
                            .iter()
                            .map(discovery_worker_run_instruction_json)
                            .collect(),
                    ),
                ),
                (
                    "count",
                    integer(plan.discovery_worker_run_plan.instructions.len() as i64),
                ),
            ]),
        ),
    ])
}

fn supervision_plan_summary_json(summary: &RuntimeSupervisionPlanSummary) -> JsonValue {
    object([
        ("generated_at_ms", integer(summary.generated_at_ms as i64)),
        ("total_actions", integer(summary.total_actions as i64)),
        (
            "pairing_expiry_count",
            integer(summary.pairing_expiry_count as i64),
        ),
        (
            "state_refresh_count",
            integer(summary.state_refresh_count as i64),
        ),
        (
            "missing_state_refresh_count",
            integer(summary.missing_state_refresh_count as i64),
        ),
        (
            "stale_state_refresh_count",
            integer(summary.stale_state_refresh_count as i64),
        ),
        (
            "desired_state_drift_count",
            integer(summary.desired_state_drift_count as i64),
        ),
        (
            "desired_missing_state_count",
            integer(summary.desired_missing_state_count as i64),
        ),
        (
            "desired_stale_state_count",
            integer(summary.desired_stale_state_count as i64),
        ),
        (
            "desired_drifted_state_count",
            integer(summary.desired_drifted_state_count as i64),
        ),
        (
            "worker_restart_count",
            integer(summary.worker_restart_count as i64),
        ),
        (
            "discovery_worker_run_count",
            integer(summary.discovery_worker_run_count as i64),
        ),
        ("is_idle", JsonValue::Bool(summary.is_idle())),
        (
            "has_state_refresh_work",
            JsonValue::Bool(summary.has_state_refresh_work()),
        ),
        (
            "has_reconciliation_work",
            JsonValue::Bool(summary.has_reconciliation_work()),
        ),
        (
            "has_worker_restart_work",
            JsonValue::Bool(summary.has_worker_restart_work()),
        ),
        (
            "has_discovery_worker_work",
            JsonValue::Bool(summary.has_discovery_worker_work()),
        ),
    ])
}

fn worker_restart_instruction_json(instruction: &WorkerRestartInstruction) -> JsonValue {
    object([
        ("bridge_id", string(instruction.bridge_id.as_str())),
        (
            "integration_id",
            string(instruction.integration_id.as_str()),
        ),
        (
            "reason",
            string(worker_restart_reason_label(instruction.reason)),
        ),
        ("status", string(instruction.status.as_str())),
        (
            "last_heartbeat_at_ms",
            integer(instruction.last_heartbeat_at_ms as i64),
        ),
        (
            "heartbeat_timeout_ms",
            integer(instruction.heartbeat_timeout_ms as i64),
        ),
        ("due_at_ms", integer(instruction.due_at_ms as i64)),
        ("planned_at_ms", integer(instruction.planned_at_ms as i64)),
        (
            "restart_attempt",
            integer(instruction.restart_attempt as i64),
        ),
        ("overdue_by_ms", integer(instruction.overdue_by_ms() as i64)),
    ])
}

fn discovery_worker_run_instruction_json(instruction: &DiscoveryWorkerRunInstruction) -> JsonValue {
    object([
        ("worker_id", string(instruction.worker_id.as_str())),
        (
            "integration_id",
            string(instruction.integration_id.as_str()),
        ),
        ("kind", string(instruction.kind.as_str())),
        ("status", string(instruction.status.as_str())),
        (
            "sources",
            JsonValue::Array(
                instruction
                    .sources
                    .iter()
                    .map(|source| string(source.as_str()))
                    .collect(),
            ),
        ),
        (
            "network_interfaces",
            JsonValue::Array(instruction.network_interfaces.iter().map(string).collect()),
        ),
        ("due_at_ms", integer(instruction.due_at_ms as i64)),
        ("planned_at_ms", integer(instruction.planned_at_ms as i64)),
        ("interval_ms", integer(instruction.interval_ms as i64)),
        ("run_timeout_ms", integer(instruction.run_timeout_ms as i64)),
        ("retry_delay_ms", integer(instruction.retry_delay_ms as i64)),
        (
            "max_retry_delay_ms",
            integer(instruction.max_retry_delay_ms as i64),
        ),
        (
            "retry_backoff_multiplier",
            integer(instruction.retry_backoff_multiplier as i64),
        ),
        (
            "consecutive_failure_count",
            integer(instruction.consecutive_failure_count as i64),
        ),
        ("overdue_by_ms", integer(instruction.overdue_by_ms() as i64)),
        (
            "mdns_service_type",
            instruction
                .mdns_service_type()
                .map(string)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "metadata",
            JsonValue::Array(instruction.metadata.iter().map(metadata_json).collect()),
        ),
    ])
}

fn list_integrations_output_json(
    catalog: &[IntegrationCatalogEntry],
    entries: Vec<&IntegrationCatalogEntry>,
) -> JsonValue {
    object([
        (
            "integrations",
            JsonValue::Array(
                entries
                    .iter()
                    .map(|entry| integration_entry_json(entry))
                    .collect(),
            ),
        ),
        ("count", integer(entries.len() as i64)),
        ("catalog_count", integer(catalog.len() as i64)),
    ])
}

fn describe_integration_output_json(
    entry: &IntegrationCatalogEntry,
    plan: &IntegrationActivationPlan,
    report: &IntegrationReadinessReport,
) -> JsonValue {
    object([
        ("integration", integration_entry_json(entry)),
        ("activation_plan", activation_plan_json(plan)),
        ("readiness_report", readiness_report_json(report)),
    ])
}

fn list_primitives_output_json(
    priority_at_or_before: u8,
    include_ecosystem_coverage: bool,
    limit: usize,
) -> JsonValue {
    let catalog = first_party_catalog();
    let sources = ecosystem_survey_sources();
    let primitives = primitive_family_descriptors();
    let mut backlog = if include_ecosystem_coverage {
        primitive_backlog_with_ecosystem_coverage(&catalog, &sources, priority_at_or_before)
            .iter()
            .map(primitive_backlog_coverage_json)
            .collect::<Vec<_>>()
    } else {
        primitive_backlog_at_or_before_priority(&catalog, priority_at_or_before)
            .iter()
            .map(primitive_backlog_json)
            .collect::<Vec<_>>()
    };
    backlog.truncate(limit);

    object([
        (
            "primitives",
            JsonValue::Array(primitives.iter().map(primitive_descriptor_json).collect()),
        ),
        ("backlog", JsonValue::Array(backlog.clone())),
        ("primitive_count", integer(primitives.len() as i64)),
        ("backlog_count", integer(backlog.len() as i64)),
    ])
}

fn describe_primitive_output_json(
    primitive: PrimitiveFamily,
    priority_at_or_before: u8,
) -> JsonValue {
    let catalog = first_party_catalog();
    let sources = ecosystem_survey_sources();
    let descriptor = describe_primitive_family(primitive);
    let integrations = entries_requiring_primitive(&catalog, primitive)
        .into_iter()
        .filter(|entry| entry.priority <= priority_at_or_before)
        .collect::<Vec<_>>();
    let ecosystem_sources = survey_sources_requiring_primitive(&sources, primitive);
    let platforms = ecosystem_platforms_requiring_primitive(&sources, primitive);

    object([
        ("primitive", primitive_descriptor_json(&descriptor)),
        (
            "integrations",
            JsonValue::Array(
                integrations
                    .iter()
                    .map(|entry| integration_entry_summary_json(entry))
                    .collect(),
            ),
        ),
        (
            "ecosystem_sources",
            JsonValue::Array(
                ecosystem_sources
                    .iter()
                    .map(|source| ecosystem_source_json(*source))
                    .collect(),
            ),
        ),
        ("integration_count", integer(integrations.len() as i64)),
        ("source_count", integer(ecosystem_sources.len() as i64)),
        ("platform_count", integer(platforms.len() as i64)),
    ])
}

fn integration_entry_json(entry: &IntegrationCatalogEntry) -> JsonValue {
    object([
        ("integration_id", string(entry.integration_id.as_str())),
        ("display_name", string(&entry.display_name)),
        ("summary", string(&entry.summary)),
        (
            "category",
            string(integration_category_label(entry.category)),
        ),
        (
            "connectivity",
            string(connectivity_class_label(entry.connectivity)),
        ),
        (
            "runtime_kind",
            string(runtime_kind_label(entry.runtime_kind)),
        ),
        (
            "implementation_status",
            string(implementation_status_label(entry.implementation_status)),
        ),
        ("priority", integer(entry.priority as i64)),
        (
            "discovery_mechanisms",
            JsonValue::Array(
                entry
                    .discovery_mechanisms
                    .iter()
                    .map(|mechanism| string(discovery_mechanism_label(*mechanism)))
                    .collect(),
            ),
        ),
        (
            "auth_modes",
            JsonValue::Array(
                entry
                    .auth_modes
                    .iter()
                    .map(|mode| string(auth_mode_label(*mode)))
                    .collect(),
            ),
        ),
        (
            "required_capabilities",
            JsonValue::Array(
                entry
                    .required_capabilities
                    .iter()
                    .map(|capability_id| string(capability_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "target_entity_kinds",
            JsonValue::Array(
                entry
                    .target_entity_kinds
                    .iter()
                    .map(|kind| string(entity_kind_label(*kind)))
                    .collect(),
            ),
        ),
        (
            "supported_protocols",
            JsonValue::Array(
                entry
                    .supported_protocols
                    .iter()
                    .map(|protocol| string(protocol.as_str()))
                    .collect(),
            ),
        ),
        (
            "depends_on_integrations",
            JsonValue::Array(
                entry
                    .depends_on_integrations
                    .iter()
                    .map(|integration_id| string(integration_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "virtual_target",
            entry
                .virtual_target
                .as_ref()
                .map(|integration_id| string(integration_id.as_str()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "virtual_iot_standards",
            JsonValue::Array(
                entry
                    .virtual_iot_standards
                    .iter()
                    .map(|protocol| string(protocol.as_str()))
                    .collect(),
            ),
        ),
        (
            "required_primitives",
            JsonValue::Array(
                entry
                    .required_primitives
                    .iter()
                    .map(|primitive| string(primitive.as_str()))
                    .collect(),
            ),
        ),
        (
            "policy_surfaces",
            JsonValue::Array(
                entry
                    .policy_surfaces()
                    .iter()
                    .map(|surface| string(surface.as_str()))
                    .collect(),
            ),
        ),
        (
            "highest_policy_tier",
            string(privilege_tier_label(entry.highest_policy_tier())),
        ),
        (
            "source_refs",
            JsonValue::Array(
                entry
                    .source_refs
                    .iter()
                    .map(source_reference_json)
                    .collect(),
            ),
        ),
        (
            "notes",
            JsonValue::Array(entry.notes.iter().map(string).collect()),
        ),
    ])
}

fn integration_entry_summary_json(entry: &IntegrationCatalogEntry) -> JsonValue {
    object([
        ("integration_id", string(entry.integration_id.as_str())),
        ("display_name", string(&entry.display_name)),
        (
            "category",
            string(integration_category_label(entry.category)),
        ),
        (
            "implementation_status",
            string(implementation_status_label(entry.implementation_status)),
        ),
        ("priority", integer(entry.priority as i64)),
    ])
}

fn activation_plan_json(plan: &IntegrationActivationPlan) -> JsonValue {
    object([
        (
            "requested_integration_id",
            string(plan.requested_integration_id.as_str()),
        ),
        ("display_name", string(&plan.display_name)),
        (
            "activation_target",
            activation_target_json(&plan.activation_target),
        ),
        (
            "implementation_status",
            string(implementation_status_label(plan.implementation_status)),
        ),
        ("priority", integer(plan.priority as i64)),
        (
            "runtime_kind",
            string(runtime_kind_label(plan.runtime_kind)),
        ),
        (
            "required_primitives",
            JsonValue::Array(
                plan.required_primitives
                    .iter()
                    .map(|primitive| string(primitive.as_str()))
                    .collect(),
            ),
        ),
        (
            "required_capabilities",
            JsonValue::Array(
                plan.required_capabilities
                    .iter()
                    .map(|capability_id| string(capability_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "auth_modes",
            JsonValue::Array(
                plan.auth_modes
                    .iter()
                    .map(|mode| string(auth_mode_label(*mode)))
                    .collect(),
            ),
        ),
        (
            "discovery_mechanisms",
            JsonValue::Array(
                plan.discovery_mechanisms
                    .iter()
                    .map(|mechanism| string(discovery_mechanism_label(*mechanism)))
                    .collect(),
            ),
        ),
        (
            "depends_on_integrations",
            JsonValue::Array(
                plan.depends_on_integrations
                    .iter()
                    .map(|integration_id| string(integration_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "policy_surfaces",
            JsonValue::Array(
                plan.policy_surfaces
                    .iter()
                    .map(|surface| string(surface.as_str()))
                    .collect(),
            ),
        ),
        (
            "highest_policy_tier",
            string(privilege_tier_label(plan.highest_policy_tier)),
        ),
        (
            "requires_human_review",
            JsonValue::Bool(plan.requires_human_review()),
        ),
        ("local_only", JsonValue::Bool(plan.local_only)),
        ("cloud_required", JsonValue::Bool(plan.cloud_required)),
    ])
}

fn readiness_report_json(report: &IntegrationReadinessReport) -> JsonValue {
    object([
        (
            "requested_integration_id",
            string(report.requested_integration_id.as_str()),
        ),
        ("display_name", string(&report.display_name)),
        (
            "activation_target",
            activation_target_json(&report.activation_target),
        ),
        ("priority", integer(report.priority as i64)),
        (
            "missing_primitives",
            JsonValue::Array(
                report
                    .missing_primitives
                    .iter()
                    .map(|primitive| string(primitive.as_str()))
                    .collect(),
            ),
        ),
        (
            "missing_capabilities",
            JsonValue::Array(
                report
                    .missing_capabilities
                    .iter()
                    .map(|capability_id| string(capability_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "missing_dependencies",
            JsonValue::Array(
                report
                    .missing_dependencies
                    .iter()
                    .map(|integration_id| string(integration_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "activation_ready",
            JsonValue::Bool(report.activation_ready()),
        ),
        (
            "requires_human_review",
            JsonValue::Bool(report.requires_human_review),
        ),
        (
            "highest_policy_tier",
            string(privilege_tier_label(report.highest_policy_tier)),
        ),
        ("local_only", JsonValue::Bool(report.local_only)),
        ("cloud_required", JsonValue::Bool(report.cloud_required)),
    ])
}

fn activation_target_json(target: &IntegrationActivationTarget) -> JsonValue {
    match target {
        IntegrationActivationTarget::Direct => object([("kind", string("direct"))]),
        IntegrationActivationTarget::DelegatedIntegration(integration_id) => object([
            ("kind", string("delegated_integration")),
            ("integration_id", string(integration_id.as_str())),
        ]),
        IntegrationActivationTarget::DelegatedStandards(protocols) => object([
            ("kind", string("delegated_standards")),
            (
                "protocols",
                JsonValue::Array(
                    protocols
                        .iter()
                        .map(|protocol| string(protocol.as_str()))
                        .collect(),
                ),
            ),
        ]),
    }
}

fn primitive_descriptor_json(descriptor: &PrimitiveFamilyDescriptor) -> JsonValue {
    object([
        ("primitive", string(descriptor.primitive.as_str())),
        ("display_name", string(descriptor.display_name)),
        ("summary", string(descriptor.summary)),
    ])
}

fn primitive_backlog_json(item: &PrimitiveBacklogItem) -> JsonValue {
    object([
        ("primitive", string(item.primitive.as_str())),
        ("highest_priority", integer(item.highest_priority as i64)),
        ("entry_count", integer(item.entry_count as i64)),
        (
            "integration_ids",
            JsonValue::Array(
                item.integration_ids
                    .iter()
                    .map(|integration_id| string(integration_id.as_str()))
                    .collect(),
            ),
        ),
    ])
}

fn primitive_backlog_coverage_json(item: &PrimitiveBacklogCoverageItem) -> JsonValue {
    object([
        ("primitive", string(item.primitive.as_str())),
        ("highest_priority", integer(item.highest_priority as i64)),
        ("entry_count", integer(item.entry_count as i64)),
        (
            "integration_ids",
            JsonValue::Array(
                item.integration_ids
                    .iter()
                    .map(|integration_id| string(integration_id.as_str()))
                    .collect(),
            ),
        ),
        ("source_count", integer(item.source_count as i64)),
        (
            "platforms",
            JsonValue::Array(
                item.platforms
                    .iter()
                    .map(|platform| string(platform.as_str()))
                    .collect(),
            ),
        ),
        ("platform_count", integer(item.platform_count() as i64)),
    ])
}

fn ecosystem_source_json(source: &EcosystemSurveySource) -> JsonValue {
    object([
        ("platform", string(source.platform.as_str())),
        ("display_name", string(source.display_name)),
        ("source_url", string(source.source_url)),
        ("source_surface", string(source.source_surface)),
        ("contributes", string(source.contributes)),
        (
            "primitive_hints",
            JsonValue::Array(
                source
                    .primitive_hints
                    .iter()
                    .map(|primitive| string(primitive.as_str()))
                    .collect(),
            ),
        ),
    ])
}

fn source_reference_json(reference: &SourceReference) -> JsonValue {
    object([
        ("label", string(&reference.label)),
        ("url", string(&reference.url)),
        (
            "external_id",
            reference
                .external_id
                .as_ref()
                .map(string)
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

fn protocol_identifier_json(identifier: &ProtocolIdentifier) -> JsonValue {
    object([
        ("family", string(protocol_family_label(&identifier.family))),
        ("kind", string(&identifier.kind)),
        ("value", string(&identifier.value)),
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

fn scene_json(scene: &Scene) -> JsonValue {
    object([
        ("scene_id", string(scene.scene_id.as_str())),
        ("scope", string(scene_scope_label(scene.scope))),
        (
            "native_ref",
            scene
                .native_ref
                .as_ref()
                .map(protocol_identifier_json)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "actions",
            JsonValue::Array(scene.actions.iter().map(scene_action_json).collect()),
        ),
        ("action_count", integer(scene.actions.len() as i64)),
        (
            "metadata",
            JsonValue::Array(scene.metadata.iter().map(metadata_json).collect()),
        ),
    ])
}

fn scene_action_json(action: &SceneAction) -> JsonValue {
    object([
        ("entity_id", string(action.entity_id.as_str())),
        ("desired_state", smart_value_to_json(&action.desired_state)),
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

fn report_event_output_json(output: &RuntimeReportEventToolOutput) -> JsonValue {
    match output {
        RuntimeReportEventToolOutput::Device(event) => object([
            ("event_kind", string("device")),
            ("event_id", string(event.event_id.as_str())),
            ("bridge_id", string(event.bridge_id.as_str())),
            (
                "device_id",
                event
                    .device_id
                    .as_ref()
                    .map(|device_id| string(device_id.as_str()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "entity_id",
                event
                    .entity_id
                    .as_ref()
                    .map(|entity_id| string(entity_id.as_str()))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "event_type",
                string(device_event_type_label(event.event_type)),
            ),
            ("health", JsonValue::Null),
            ("observed_at_ms", integer(event.observed_at_ms as i64)),
            ("received_at_ms", integer(event.received_at_ms as i64)),
            (
                "state_delta",
                event
                    .state_delta
                    .as_ref()
                    .map(state_delta_json)
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "metadata",
                JsonValue::Array(event.metadata.iter().map(metadata_json).collect()),
            ),
        ]),
        RuntimeReportEventToolOutput::BridgeHealth(report) => object([
            ("event_kind", string("bridge_health")),
            ("event_id", string(report.event_id.as_str())),
            ("bridge_id", string(report.bridge_id.as_str())),
            ("device_id", JsonValue::Null),
            ("entity_id", JsonValue::Null),
            ("event_type", string("health")),
            ("health", string(health_label(report.health))),
            ("observed_at_ms", integer(report.observed_at_ms as i64)),
            ("received_at_ms", integer(report.received_at_ms as i64)),
            ("state_delta", JsonValue::Null),
            (
                "metadata",
                JsonValue::Array(report.metadata.iter().map(metadata_json).collect()),
            ),
        ]),
    }
}

fn event_delivery_batch_json(batch: &RuntimeEventDeliveryBatch) -> JsonValue {
    object([
        ("subscription_id", string(batch.subscription_id.as_str())),
        ("delivered_events", integer(batch.len() as i64)),
        ("remaining_events", integer(batch.remaining_events as i64)),
        ("has_more", JsonValue::Bool(batch.has_more())),
        ("summary", event_delivery_summary_json(batch)),
        (
            "events",
            JsonValue::Array(batch.events.iter().map(runtime_event_json).collect()),
        ),
    ])
}

fn event_delivery_summary_json(batch: &RuntimeEventDeliveryBatch) -> JsonValue {
    let summary = batch.summary();
    object([
        ("subscription_id", string(summary.subscription_id.as_str())),
        ("delivered_events", integer(summary.delivered_events as i64)),
        ("remaining_events", integer(summary.remaining_events as i64)),
        ("device_events", integer(summary.device_events as i64)),
        ("command_results", integer(summary.command_results as i64)),
        (
            "bridge_health_events",
            integer(summary.bridge_health_events as i64),
        ),
        (
            "state_expired_events",
            integer(summary.state_expired_events as i64),
        ),
        (
            "desired_state_drift_events",
            integer(summary.desired_state_drift_events as i64),
        ),
        (
            "worker_restart_events",
            integer(summary.worker_restart_events as i64),
        ),
        ("has_more", JsonValue::Bool(summary.has_more())),
        (
            "has_command_results",
            JsonValue::Bool(summary.has_command_results()),
        ),
        (
            "has_supervision_events",
            JsonValue::Bool(summary.has_supervision_events()),
        ),
    ])
}

fn subscription_snapshot_json(snapshot: &RuntimeSubscriptionSnapshot) -> JsonValue {
    object([
        ("subscription_id", string(snapshot.subscription_id.as_str())),
        ("filter", event_filter_json(&snapshot.filter)),
        ("queued_events", integer(snapshot.queued_events as i64)),
        ("has_backlog", JsonValue::Bool(snapshot.has_backlog())),
        (
            "backlog_status",
            string(subscription_backlog_status_label(snapshot.backlog_status())),
        ),
    ])
}

fn subscription_inventory_summary_json(summary: &RuntimeSubscriptionInventorySummary) -> JsonValue {
    object([
        (
            "total_subscriptions",
            integer(summary.total_subscriptions as i64),
        ),
        (
            "all_event_subscriptions",
            integer(summary.all_event_subscriptions as i64),
        ),
        (
            "bridge_subscriptions",
            integer(summary.bridge_subscriptions as i64),
        ),
        (
            "entity_subscriptions",
            integer(summary.entity_subscriptions as i64),
        ),
        (
            "command_subscriptions",
            integer(summary.command_subscriptions as i64),
        ),
        (
            "supervision_subscriptions",
            integer(summary.supervision_subscriptions as i64),
        ),
        (
            "backlogged_subscriptions",
            integer(summary.backlogged_subscriptions as i64),
        ),
        (
            "caught_up_subscriptions",
            integer(summary.caught_up_subscriptions as i64),
        ),
        (
            "total_queued_events",
            integer(summary.total_queued_events as i64),
        ),
        (
            "max_queued_events",
            integer(summary.max_queued_events as i64),
        ),
        (
            "average_queued_events_per_subscription",
            integer(summary.average_queued_events_per_subscription() as i64),
        ),
        (
            "backlogged_subscription_percent",
            integer(summary.backlogged_subscription_percent() as i64),
        ),
        ("has_backlog", JsonValue::Bool(summary.has_backlog())),
    ])
}

fn event_log_record_json(record: &RuntimeEventLogRecord) -> JsonValue {
    object([
        ("sequence", integer(record.sequence as i64)),
        (
            "next_checkpoint",
            integer(record.next_checkpoint.next_sequence() as i64),
        ),
        ("event", runtime_event_json(&record.event)),
    ])
}

fn event_log_summary_json(summary: &RuntimeEventLogSummary) -> JsonValue {
    object([
        ("total_events", integer(summary.total_events as i64)),
        ("device_events", integer(summary.device_events as i64)),
        ("command_results", integer(summary.command_results as i64)),
        (
            "bridge_health_events",
            integer(summary.bridge_health_events as i64),
        ),
        (
            "state_expired_events",
            integer(summary.state_expired_events as i64),
        ),
        (
            "desired_state_drift_events",
            integer(summary.desired_state_drift_events as i64),
        ),
        (
            "worker_restart_events",
            integer(summary.worker_restart_events as i64),
        ),
        (
            "first_sequence",
            summary
                .first_sequence
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "latest_sequence",
            summary
                .latest_sequence
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "next_checkpoint",
            integer(summary.next_checkpoint.next_sequence() as i64),
        ),
        ("has_events", JsonValue::Bool(summary.has_events())),
        (
            "has_command_results",
            JsonValue::Bool(summary.has_command_results()),
        ),
        (
            "has_supervision_events",
            JsonValue::Bool(summary.has_supervision_events()),
        ),
    ])
}

fn capability_grant_json(grant: &CapabilityGrant, now_ms: u64) -> JsonValue {
    object([
        ("grant_id", string(grant.grant_id.as_str())),
        ("principal_id", string(grant.principal_id.as_str())),
        (
            "scope_kind",
            string(capability_grant_scope_label(&grant.scope)),
        ),
        ("scope", capability_grant_scope_json(&grant.scope)),
        ("max_tier", string(privilege_tier_label(grant.max_tier))),
        ("granted_by", string(&grant.granted_by)),
        ("granted_at_ms", integer(grant.granted_at_ms as i64)),
        (
            "expires_at_ms",
            grant
                .expires_at_ms
                .map(|value| integer(value as i64))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "status",
            string(capability_grant_status_label(grant.status)),
        ),
        (
            "effective_status",
            string(capability_grant_status_label(grant.status_at(now_ms))),
        ),
        ("effective_status_at_ms", integer(now_ms as i64)),
        ("is_active", JsonValue::Bool(grant.is_active_at(now_ms))),
        (
            "metadata",
            JsonValue::Array(grant.metadata.iter().map(metadata_json).collect()),
        ),
    ])
}

fn capability_grant_scope_json(scope: &CapabilityGrantScope) -> JsonValue {
    match scope {
        CapabilityGrantScope::Tool(tool) => object([
            ("scope_kind", string("tool")),
            ("tool_id", string(tool.descriptor().tool_id)),
        ]),
        CapabilityGrantScope::Capability(capability_id) => object([
            ("scope_kind", string("capability")),
            ("capability_id", string(capability_id.as_str())),
        ]),
        CapabilityGrantScope::EntityCapability {
            entity_id,
            capability_id,
        } => object([
            ("scope_kind", string("entity_capability")),
            ("entity_id", string(entity_id.as_str())),
            ("capability_id", string(capability_id.as_str())),
        ]),
        CapabilityGrantScope::AllSmartHome => object([("scope_kind", string("all_smart_home"))]),
    }
}

fn capability_grant_inventory_summary_json(summary: &CapabilityGrantInventorySummary) -> JsonValue {
    object([
        ("generated_at_ms", integer(summary.generated_at_ms as i64)),
        ("total_grants", integer(summary.total_grants as i64)),
        ("active_grants", integer(summary.active_grants as i64)),
        ("pending_grants", integer(summary.pending_grants as i64)),
        ("revoked_grants", integer(summary.revoked_grants as i64)),
        ("expired_grants", integer(summary.expired_grants as i64)),
        ("tool_grants", integer(summary.tool_grants as i64)),
        (
            "capability_grants",
            integer(summary.capability_grants as i64),
        ),
        (
            "entity_capability_grants",
            integer(summary.entity_capability_grants as i64),
        ),
        (
            "all_smart_home_grants",
            integer(summary.all_smart_home_grants as i64),
        ),
        (
            "read_only_tier_grants",
            integer(summary.read_only_tier_grants as i64),
        ),
        (
            "low_risk_tier_grants",
            integer(summary.low_risk_tier_grants as i64),
        ),
        (
            "human_approval_tier_grants",
            integer(summary.human_approval_tier_grants as i64),
        ),
        (
            "high_risk_tier_grants",
            integer(summary.high_risk_tier_grants as i64),
        ),
        ("expiring_grants", integer(summary.expiring_grants as i64)),
        (
            "unique_principals",
            integer(summary.unique_principals as i64),
        ),
        ("is_empty", JsonValue::Bool(summary.is_empty())),
        (
            "has_active_grants",
            JsonValue::Bool(summary.has_active_grants()),
        ),
        ("needs_review", JsonValue::Bool(summary.needs_review())),
    ])
}

fn authorization_decision_json(decision: &AuthorizationDecision) -> JsonValue {
    object([
        ("principal_id", string(decision.principal_id.as_str())),
        (
            "subject_kind",
            string(authorization_subject_label(&decision.subject)),
        ),
        ("subject", authorization_subject_json(&decision.subject)),
        (
            "outcome",
            string(authorization_outcome_label(decision.outcome)),
        ),
        (
            "required_tier",
            string(privilege_tier_label(decision.required_tier)),
        ),
        (
            "required_capabilities",
            JsonValue::Array(
                decision
                    .required_capabilities
                    .iter()
                    .map(|capability_id| string(capability_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "matched_grants",
            JsonValue::Array(
                decision
                    .matched_grants
                    .iter()
                    .map(|grant_id| string(grant_id.as_str()))
                    .collect(),
            ),
        ),
        (
            "missing_capabilities",
            JsonValue::Array(
                decision
                    .missing_capabilities
                    .iter()
                    .map(|capability_id| string(capability_id.as_str()))
                    .collect(),
            ),
        ),
        ("decided_at_ms", integer(decision.decided_at_ms as i64)),
        ("allowed", JsonValue::Bool(decision.is_allowed())),
    ])
}

fn authorization_subject_json(subject: &AuthorizationSubject) -> JsonValue {
    match subject {
        AuthorizationSubject::Tool(tool) => object([
            ("subject_kind", string("tool")),
            ("tool_id", string(tool.descriptor().tool_id)),
        ]),
        AuthorizationSubject::Command {
            command_id,
            entity_id,
            command_type,
        } => object([
            ("subject_kind", string("command")),
            ("command_id", string(command_id.as_str())),
            ("entity_id", string(entity_id.as_str())),
            ("command_type", string(command_type_label(*command_type))),
        ]),
    }
}

fn authorization_decision_log_summary_json(summary: &AuthorizationDecisionLogSummary) -> JsonValue {
    object([
        ("total_decisions", integer(summary.total_decisions as i64)),
        (
            "allowed_decisions",
            integer(summary.allowed_decisions as i64),
        ),
        ("denied_decisions", integer(summary.denied_decisions as i64)),
        ("tool_decisions", integer(summary.tool_decisions as i64)),
        (
            "command_decisions",
            integer(summary.command_decisions as i64),
        ),
        (
            "read_only_tier_decisions",
            integer(summary.read_only_tier_decisions as i64),
        ),
        (
            "low_risk_tier_decisions",
            integer(summary.low_risk_tier_decisions as i64),
        ),
        (
            "human_approval_tier_decisions",
            integer(summary.human_approval_tier_decisions as i64),
        ),
        (
            "high_risk_tier_decisions",
            integer(summary.high_risk_tier_decisions as i64),
        ),
        (
            "decisions_with_missing_capabilities",
            integer(summary.decisions_with_missing_capabilities as i64),
        ),
        (
            "total_required_capabilities",
            integer(summary.total_required_capabilities as i64),
        ),
        (
            "total_matched_grants",
            integer(summary.total_matched_grants as i64),
        ),
        (
            "total_missing_capabilities",
            integer(summary.total_missing_capabilities as i64),
        ),
        ("is_empty", JsonValue::Bool(summary.is_empty())),
        ("has_denials", JsonValue::Bool(summary.has_denials())),
        (
            "has_missing_capabilities",
            JsonValue::Bool(summary.has_missing_capabilities()),
        ),
        (
            "approval_gated_decisions",
            integer(summary.approval_gated_decisions() as i64),
        ),
    ])
}

fn event_filter_json(filter: &RuntimeEventFilter) -> JsonValue {
    match filter {
        RuntimeEventFilter::All => object([("filter_type", string("all"))]),
        RuntimeEventFilter::Bridge(bridge_id) => object([
            ("filter_type", string("bridge")),
            ("bridge_id", string(bridge_id.as_str())),
        ]),
        RuntimeEventFilter::Entity(entity_id) => object([
            ("filter_type", string("entity")),
            ("entity_id", string(entity_id.as_str())),
        ]),
        RuntimeEventFilter::Commands => object([("filter_type", string("commands"))]),
        RuntimeEventFilter::Supervision => object([("filter_type", string("supervision"))]),
    }
}

fn runtime_event_json(event: &RuntimeEvent) -> JsonValue {
    match event {
        RuntimeEvent::Device(event) => object([
            ("event_kind", string("device")),
            ("event", device_event_json(event)),
        ]),
        RuntimeEvent::CommandResult(result) => object([
            ("event_kind", string("command_result")),
            ("command_result", command_result_json(result)),
        ]),
        RuntimeEvent::BridgeHealth {
            event_id,
            bridge_id,
            health,
            observed_at_ms,
            received_at_ms,
        } => object([
            ("event_kind", string("bridge_health")),
            ("event_id", string(event_id.as_str())),
            ("bridge_id", string(bridge_id.as_str())),
            ("health", string(health_label(*health))),
            ("observed_at_ms", integer(*observed_at_ms as i64)),
            ("received_at_ms", integer(*received_at_ms as i64)),
        ]),
        RuntimeEvent::StateExpired {
            entity_id,
            expired_at_ms,
        } => object([
            ("event_kind", string("state_expired")),
            ("entity_id", string(entity_id.as_str())),
            ("expired_at_ms", integer(*expired_at_ms as i64)),
        ]),
        RuntimeEvent::DesiredStateDrift {
            bridge_id,
            entity_id,
            capability_id,
            reason,
            detected_at_ms,
        } => object([
            ("event_kind", string("desired_state_drift")),
            ("bridge_id", string(bridge_id.as_str())),
            ("entity_id", string(entity_id.as_str())),
            ("capability_id", string(capability_id.as_str())),
            ("reason", string(reconciliation_reason_label(*reason))),
            ("detected_at_ms", integer(*detected_at_ms as i64)),
        ]),
        RuntimeEvent::WorkerNeedsRestart {
            bridge_id,
            integration_id,
            overdue_at_ms,
        } => object([
            ("event_kind", string("worker_needs_restart")),
            ("bridge_id", string(bridge_id.as_str())),
            ("integration_id", string(integration_id.as_str())),
            ("overdue_at_ms", integer(*overdue_at_ms as i64)),
        ]),
    }
}

fn device_event_json(event: &DeviceEvent) -> JsonValue {
    object([
        ("event_id", string(event.event_id.as_str())),
        ("bridge_id", string(event.bridge_id.as_str())),
        (
            "device_id",
            event
                .device_id
                .as_ref()
                .map(|value| string(value.as_str()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "entity_id",
            event
                .entity_id
                .as_ref()
                .map(|value| string(value.as_str()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "event_type",
            string(device_event_type_label(event.event_type)),
        ),
        ("observed_at_ms", integer(event.observed_at_ms as i64)),
        ("received_at_ms", integer(event.received_at_ms as i64)),
        (
            "state_delta",
            event
                .state_delta
                .as_ref()
                .map(state_delta_json)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "raw_ref",
            event
                .raw_ref
                .as_ref()
                .map(string)
                .unwrap_or(JsonValue::Null),
        ),
        (
            "correlation_id",
            event
                .correlation_id
                .as_ref()
                .map(|value| string(value.as_str()))
                .unwrap_or(JsonValue::Null),
        ),
        (
            "metadata",
            JsonValue::Array(event.metadata.iter().map(metadata_json).collect()),
        ),
    ])
}

fn state_delta_json(delta: &StateDelta) -> JsonValue {
    object([
        ("capability_id", string(delta.capability_id.as_str())),
        ("value", smart_value_to_json(&delta.value)),
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

fn parse_subscription_sort(label: &str) -> Result<RuntimeSubscriptionSort, ToolCallError> {
    match label {
        "subscription_id" | "id" => Ok(RuntimeSubscriptionSort::SubscriptionId),
        "queued_events_desc" | "backlog_desc" => Ok(RuntimeSubscriptionSort::QueuedEventsDesc),
        _ => Err(validation_error(format!(
            "unknown subscription sort `{label}`"
        ))),
    }
}

fn parse_event_sort(label: &str) -> Result<RuntimeEventSort, ToolCallError> {
    match label {
        "sequence_asc" | "oldest_first" => Ok(RuntimeEventSort::SequenceAsc),
        "sequence_desc" | "newest_first" => Ok(RuntimeEventSort::SequenceDesc),
        _ => Err(validation_error(format!("unknown event sort `{label}`"))),
    }
}

fn parse_room_sort(label: &str) -> Result<RuntimeRoomSort, ToolCallError> {
    match label {
        "room_id" | "id" => Ok(RuntimeRoomSort::RoomId),
        "attention_desc" | "attention" => Ok(RuntimeRoomSort::AttentionDesc),
        "entity_count_desc" | "entities_desc" => Ok(RuntimeRoomSort::EntityCountDesc),
        "scene_count_desc" | "scenes_desc" => Ok(RuntimeRoomSort::SceneCountDesc),
        _ => Err(validation_error(format!("unknown room sort `{label}`"))),
    }
}

fn parse_authorization_decision_sort(
    label: &str,
) -> Result<RuntimeAuthorizationDecisionSort, ToolCallError> {
    match label {
        "decided_at_asc" | "oldest_first" => Ok(RuntimeAuthorizationDecisionSort::DecidedAtAsc),
        "decided_at_desc" | "newest_first" => Ok(RuntimeAuthorizationDecisionSort::DecidedAtDesc),
        _ => Err(validation_error(format!(
            "unknown authorization decision sort `{label}`"
        ))),
    }
}

fn parse_authorization_outcome(label: &str) -> Result<AuthorizationOutcome, ToolCallError> {
    match label {
        "allowed" | "allow" => Ok(AuthorizationOutcome::Allowed),
        "denied" | "deny" => Ok(AuthorizationOutcome::Denied),
        _ => Err(validation_error(format!(
            "unknown authorization outcome `{label}`"
        ))),
    }
}

fn parse_capability_grant_status(label: &str) -> Result<CapabilityGrantStatus, ToolCallError> {
    match label {
        "pending" => Ok(CapabilityGrantStatus::Pending),
        "active" => Ok(CapabilityGrantStatus::Active),
        "revoked" => Ok(CapabilityGrantStatus::Revoked),
        "expired" => Ok(CapabilityGrantStatus::Expired),
        _ => Err(validation_error(format!(
            "unknown capability grant status `{label}`"
        ))),
    }
}

fn parse_capability_grant_scope_kind(
    label: &str,
) -> Result<RuntimeCapabilityGrantScopeKind, ToolCallError> {
    match label {
        "tool" => Ok(RuntimeCapabilityGrantScopeKind::Tool),
        "capability" => Ok(RuntimeCapabilityGrantScopeKind::Capability),
        "entity_capability" | "entity" => Ok(RuntimeCapabilityGrantScopeKind::EntityCapability),
        "all_smart_home" | "all" => Ok(RuntimeCapabilityGrantScopeKind::AllSmartHome),
        _ => Err(validation_error(format!(
            "unknown capability grant scope kind `{label}`"
        ))),
    }
}

fn parse_capability_grant_sort(label: &str) -> Result<RuntimeCapabilityGrantSort, ToolCallError> {
    match label {
        "grant_id" | "id" => Ok(RuntimeCapabilityGrantSort::GrantId),
        "principal_id" | "principal" => Ok(RuntimeCapabilityGrantSort::PrincipalId),
        "granted_at_asc" | "oldest_first" => Ok(RuntimeCapabilityGrantSort::GrantedAtAsc),
        "granted_at_desc" | "newest_first" => Ok(RuntimeCapabilityGrantSort::GrantedAtDesc),
        "expires_at_asc" | "expires_first" => Ok(RuntimeCapabilityGrantSort::ExpiresAtAsc),
        "expires_at_desc" | "expires_last" => Ok(RuntimeCapabilityGrantSort::ExpiresAtDesc),
        _ => Err(validation_error(format!(
            "unknown capability grant sort `{label}`"
        ))),
    }
}

fn parse_desired_state_sort(label: &str) -> Result<DesiredStateSort, ToolCallError> {
    match label {
        "entity_id" | "entity" => Ok(DesiredStateSort::EntityId),
        "requested_by_then_entity_id" | "requested_by" => {
            Ok(DesiredStateSort::RequestedByThenEntityId)
        }
        "command_timeout_desc" | "timeout_desc" => Ok(DesiredStateSort::CommandTimeoutDesc),
        _ => Err(validation_error(format!(
            "unknown desired state sort `{label}`"
        ))),
    }
}

fn parse_pairing_session_sort(label: &str) -> Result<RuntimePairingSessionSort, ToolCallError> {
    match label {
        "session_id" | "id" => Ok(RuntimePairingSessionSort::SessionId),
        "expires_at" | "expires_at_asc" => Ok(RuntimePairingSessionSort::ExpiresAt),
        "started_at_desc" | "newest_first" => Ok(RuntimePairingSessionSort::StartedAtDesc),
        "status_then_expires_at" | "status" => Ok(RuntimePairingSessionSort::StatusThenExpiresAt),
        _ => Err(validation_error(format!(
            "unknown pairing session sort `{label}`"
        ))),
    }
}

fn parse_worker_status(label: &str) -> Result<WorkerStatus, ToolCallError> {
    match label {
        "starting" => Ok(WorkerStatus::Starting),
        "running" => Ok(WorkerStatus::Running),
        "unhealthy" => Ok(WorkerStatus::Unhealthy),
        "restarting" => Ok(WorkerStatus::Restarting),
        "stopped" => Ok(WorkerStatus::Stopped),
        _ => Err(validation_error(format!("unknown worker status `{label}`"))),
    }
}

fn parse_discovery_worker_kind(label: &str) -> Result<DiscoveryWorkerKind, ToolCallError> {
    match label {
        "mdns_scan" | "mdns" => Ok(DiscoveryWorkerKind::MdnsScan),
        "cloud_fallback" | "cloud" => Ok(DiscoveryWorkerKind::CloudFallback),
        "manual_seed" | "manual" => Ok(DiscoveryWorkerKind::ManualSeed),
        "composite" => Ok(DiscoveryWorkerKind::Composite),
        "simulator" => Ok(DiscoveryWorkerKind::Simulator),
        _ => Err(validation_error(format!(
            "unknown discovery worker kind `{label}`"
        ))),
    }
}

fn parse_discovery_worker_sort(label: &str) -> Result<DiscoveryWorkerSort, ToolCallError> {
    match label {
        "worker_id" | "worker" => Ok(DiscoveryWorkerSort::WorkerId),
        "next_due_at" | "next_due_at_ms" | "due_at" => Ok(DiscoveryWorkerSort::NextDueAt),
        "status_then_worker_id" | "status" => Ok(DiscoveryWorkerSort::StatusThenWorkerId),
        "consecutive_failures_desc" | "failures_desc" => {
            Ok(DiscoveryWorkerSort::ConsecutiveFailuresDesc)
        }
        _ => Err(validation_error(format!(
            "unknown discovery worker sort `{label}`"
        ))),
    }
}

fn parse_supervised_worker_sort(label: &str) -> Result<SupervisedWorkerSort, ToolCallError> {
    match label {
        "bridge_id" | "bridge" => Ok(SupervisedWorkerSort::BridgeId),
        "heartbeat_due_at" | "heartbeat_due_at_ms" | "due_at" => {
            Ok(SupervisedWorkerSort::HeartbeatDueAt)
        }
        "restart_count_desc" | "restarts_desc" => Ok(SupervisedWorkerSort::RestartCountDesc),
        "status_then_bridge_id" | "status" => Ok(SupervisedWorkerSort::StatusThenBridgeId),
        _ => Err(validation_error(format!("unknown worker sort `{label}`"))),
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

fn subscription_backlog_status_label(status: RuntimeSubscriptionBacklogStatus) -> &'static str {
    match status {
        RuntimeSubscriptionBacklogStatus::CaughtUp => "caught_up",
        RuntimeSubscriptionBacklogStatus::Backlogged => "backlogged",
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

fn parse_device_event_type(label: &str) -> Result<DeviceEventType, ToolCallError> {
    match label {
        "discovered" => Ok(DeviceEventType::Discovered),
        "updated" => Ok(DeviceEventType::Updated),
        "removed" => Ok(DeviceEventType::Removed),
        "unavailable" => Ok(DeviceEventType::Unavailable),
        "error" => Ok(DeviceEventType::Error),
        "health" => Ok(DeviceEventType::Health),
        _ => Err(validation_error(format!(
            "unknown device event type `{label}`"
        ))),
    }
}

fn parse_scene_scope(label: &str) -> Result<SceneScope, ToolCallError> {
    match label {
        "room" => Ok(SceneScope::Room),
        "zone" => Ok(SceneScope::Zone),
        "home" => Ok(SceneScope::Home),
        "bridge" => Ok(SceneScope::Bridge),
        "custom" => Ok(SceneScope::Custom),
        _ => Err(validation_error(format!("unknown scene scope `{label}`"))),
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

fn parse_discovery_signal_status(label: &str) -> Result<DiscoverySignalStatus, ToolCallError> {
    match label {
        "fresh" => Ok(DiscoverySignalStatus::Fresh),
        "stale" => Ok(DiscoverySignalStatus::Stale),
        "expired" => Ok(DiscoverySignalStatus::Expired),
        _ => Err(validation_error(format!(
            "unknown discovery signal status `{label}`"
        ))),
    }
}

fn parse_pairing_requirement(label: &str) -> Result<PairingRequirement, ToolCallError> {
    match label {
        "unknown" => Ok(PairingRequirement::Unknown),
        "none" | "ready" => Ok(PairingRequirement::None),
        "physical_presence" | "button" | "link_button" => Ok(PairingRequirement::PhysicalPresence),
        "local_code" | "code" => Ok(PairingRequirement::LocalCode),
        "credentials" => Ok(PairingRequirement::Credentials),
        "oauth2" | "oauth" => Ok(PairingRequirement::OAuth2),
        "certificate" | "cert" => Ok(PairingRequirement::Certificate),
        "radio_inclusion" | "inclusion" => Ok(PairingRequirement::RadioInclusion),
        "mqtt_credentials" | "mqtt" => Ok(PairingRequirement::MqttCredentials),
        _ => Err(validation_error(format!(
            "unknown pairing requirement `{label}`"
        ))),
    }
}

fn parse_discovery_pairing_action(label: &str) -> Result<DiscoveryPairingAction, ToolCallError> {
    match label {
        "ready" => Ok(DiscoveryPairingAction::Ready),
        "press_physical_button" | "press_button" | "link_button" => {
            Ok(DiscoveryPairingAction::PressPhysicalButton)
        }
        "enter_local_code" | "local_code" => Ok(DiscoveryPairingAction::EnterLocalCode),
        "provide_credentials" | "credentials" => Ok(DiscoveryPairingAction::ProvideCredentials),
        "complete_oauth2" | "oauth2" | "oauth" => Ok(DiscoveryPairingAction::CompleteOAuth2),
        "install_certificate" | "certificate" => Ok(DiscoveryPairingAction::InstallCertificate),
        "start_radio_inclusion" | "radio_inclusion" => {
            Ok(DiscoveryPairingAction::StartRadioInclusion)
        }
        "configure_mqtt_credentials" | "mqtt_credentials" | "mqtt" => {
            Ok(DiscoveryPairingAction::ConfigureMqttCredentials)
        }
        "investigate_unknown_requirement" | "unknown" => {
            Ok(DiscoveryPairingAction::InvestigateUnknownRequirement)
        }
        _ => Err(validation_error(format!(
            "unknown discovery pairing action `{label}`"
        ))),
    }
}

fn parse_discovery_pairing_plan_sort(
    label: &str,
) -> Result<DiscoveryPairingPlanSort, ToolCallError> {
    match label {
        "plan_rank" | "rank" => Ok(DiscoveryPairingPlanSort::PlanRank),
        "newest_first" | "newest" => Ok(DiscoveryPairingPlanSort::NewestFirst),
        "integration_then_bridge" | "integration" => {
            Ok(DiscoveryPairingPlanSort::IntegrationThenBridge)
        }
        "source_preference" | "source" => Ok(DiscoveryPairingPlanSort::SourcePreference),
        _ => Err(validation_error(format!(
            "unknown discovery pairing plan sort `{label}`"
        ))),
    }
}

fn parse_integration_category(label: &str) -> Result<IntegrationCategory, ToolCallError> {
    match label {
        "protocol_standard" => Ok(IntegrationCategory::ProtocolStandard),
        "local_hub" => Ok(IntegrationCategory::LocalHub),
        "local_device" => Ok(IntegrationCategory::LocalDevice),
        "bluetooth_profile" => Ok(IntegrationCategory::BluetoothProfile),
        "cloud_hub" => Ok(IntegrationCategory::CloudHub),
        "camera_media" => Ok(IntegrationCategory::CameraMedia),
        "energy_climate" => Ok(IntegrationCategory::EnergyClimate),
        "notification_channel" => Ok(IntegrationCategory::NotificationChannel),
        "data_service" => Ok(IntegrationCategory::DataService),
        "helper_calculated" => Ok(IntegrationCategory::HelperCalculated),
        "virtual_alias" => Ok(IntegrationCategory::VirtualAlias),
        "system_hardware" => Ok(IntegrationCategory::SystemHardware),
        _ => Err(validation_error(format!(
            "unknown integration category `{label}`"
        ))),
    }
}

fn parse_connectivity_class(label: &str) -> Result<ConnectivityClass, ToolCallError> {
    match label {
        "local_push" => Ok(ConnectivityClass::LocalPush),
        "local_polling" => Ok(ConnectivityClass::LocalPolling),
        "cloud_push" => Ok(ConnectivityClass::CloudPush),
        "cloud_polling" => Ok(ConnectivityClass::CloudPolling),
        "calculated" => Ok(ConnectivityClass::Calculated),
        "assumed_state" => Ok(ConnectivityClass::AssumedState),
        _ => Err(validation_error(format!(
            "unknown connectivity class `{label}`"
        ))),
    }
}

fn parse_implementation_status(label: &str) -> Result<ImplementationStatus, ToolCallError> {
    match label {
        "cataloged" => Ok(ImplementationStatus::Cataloged),
        "specified" => Ok(ImplementationStatus::Specified),
        "scaffolded" => Ok(ImplementationStatus::Scaffolded),
        "simulated" => Ok(ImplementationStatus::Simulated),
        "first_party_runtime" => Ok(ImplementationStatus::FirstPartyRuntime),
        "production_ready" => Ok(ImplementationStatus::ProductionReady),
        "delegated_to_standard" => Ok(ImplementationStatus::DelegatedToStandard),
        "unsupported" => Ok(ImplementationStatus::Unsupported),
        _ => Err(validation_error(format!(
            "unknown implementation status `{label}`"
        ))),
    }
}

fn parse_primitive_family(label: &str) -> Result<PrimitiveFamily, ToolCallError> {
    match label {
        "normalized_model" => Ok(PrimitiveFamily::NormalizedModel),
        "discovery_index" => Ok(PrimitiveFamily::DiscoveryIndex),
        "mdns" => Ok(PrimitiveFamily::Mdns),
        "ssdp" => Ok(PrimitiveFamily::Ssdp),
        "dhcp" => Ok(PrimitiveFamily::Dhcp),
        "local_http" => Ok(PrimitiveFamily::LocalHttp),
        "websocket" => Ok(PrimitiveFamily::WebSocket),
        "server_sent_events" => Ok(PrimitiveFamily::ServerSentEvents),
        "mqtt" => Ok(PrimitiveFamily::Mqtt),
        "bluetooth_low_energy" => Ok(PrimitiveFamily::BluetoothLowEnergy),
        "usb" => Ok(PrimitiveFamily::Usb),
        "serial_controller" => Ok(PrimitiveFamily::SerialController),
        "radio_802154" => Ok(PrimitiveFamily::Radio802154),
        "zwave_serial_api" => Ok(PrimitiveFamily::ZWaveSerialApi),
        "matter_commissioning" => Ok(PrimitiveFamily::MatterCommissioning),
        "homekit_pairing" => Ok(PrimitiveFamily::HomeKitPairing),
        "cloud_api" => Ok(PrimitiveFamily::CloudApi),
        "webhook" => Ok(PrimitiveFamily::Webhook),
        "oauth2" => Ok(PrimitiveFamily::OAuth2),
        "local_pairing" => Ok(PrimitiveFamily::LocalPairing),
        "local_token" => Ok(PrimitiveFamily::LocalToken),
        "certificate_pairing" => Ok(PrimitiveFamily::CertificatePairing),
        "radio_network_key" => Ok(PrimitiveFamily::RadioNetworkKey),
        "mqtt_credentials" => Ok(PrimitiveFamily::MqttCredentials),
        "camera_media" => Ok(PrimitiveFamily::CameraMedia),
        "energy_telemetry" => Ok(PrimitiveFamily::EnergyTelemetry),
        "calculated_state" => Ok(PrimitiveFamily::CalculatedState),
        "command_mapping" => Ok(PrimitiveFamily::CommandMapping),
        "capability_policy" => Ok(PrimitiveFamily::CapabilityPolicy),
        "vault_lease" => Ok(PrimitiveFamily::VaultLease),
        "supervision" => Ok(PrimitiveFamily::Supervision),
        "test_simulator" => Ok(PrimitiveFamily::TestSimulator),
        _ => Err(validation_error(format!("unknown primitive `{label}`"))),
    }
}

fn parse_policy_surface(label: &str) -> Result<IntegrationPolicySurface, ToolCallError> {
    match label {
        "local_actuation" => Ok(IntegrationPolicySurface::LocalActuation),
        "entry_access" => Ok(IntegrationPolicySurface::EntryAccess),
        "climate_control" => Ok(IntegrationPolicySurface::ClimateControl),
        "camera_media" => Ok(IntegrationPolicySurface::CameraMedia),
        "energy_management" => Ok(IntegrationPolicySurface::EnergyManagement),
        "credential_lease" => Ok(IntegrationPolicySurface::CredentialLease),
        "credentialed_cloud" => Ok(IntegrationPolicySurface::CredentialedCloud),
        "radio_network_management" => Ok(IntegrationPolicySurface::RadioNetworkManagement),
        "network_infrastructure" => Ok(IntegrationPolicySurface::NetworkInfrastructure),
        _ => Err(validation_error(format!(
            "unknown policy surface `{label}`"
        ))),
    }
}

fn parse_discovery_mechanism(label: &str) -> Result<DiscoveryMechanism, ToolCallError> {
    match label {
        "mdns" => Ok(DiscoveryMechanism::Mdns),
        "ssdp" => Ok(DiscoveryMechanism::Ssdp),
        "bluetooth" => Ok(DiscoveryMechanism::Bluetooth),
        "usb" => Ok(DiscoveryMechanism::Usb),
        "dhcp" => Ok(DiscoveryMechanism::Dhcp),
        "mqtt" => Ok(DiscoveryMechanism::Mqtt),
        "manual" => Ok(DiscoveryMechanism::Manual),
        "cloud_account" => Ok(DiscoveryMechanism::CloudAccount),
        "webhook" => Ok(DiscoveryMechanism::Webhook),
        "file_config" => Ok(DiscoveryMechanism::FileConfig),
        _ => Err(validation_error(format!(
            "unknown discovery mechanism `{label}`"
        ))),
    }
}

fn parse_auth_mode(label: &str) -> Result<AuthMode, ToolCallError> {
    match label {
        "none" => Ok(AuthMode::None),
        "local_pairing" => Ok(AuthMode::LocalPairing),
        "local_token" => Ok(AuthMode::LocalToken),
        "username_password" => Ok(AuthMode::UsernamePassword),
        "oauth2" => Ok(AuthMode::OAuth2),
        "api_key" => Ok(AuthMode::ApiKey),
        "certificate" => Ok(AuthMode::Certificate),
        "radio_network_key" => Ok(AuthMode::RadioNetworkKey),
        "mqtt_credentials" => Ok(AuthMode::MqttCredentials),
        _ => Err(validation_error(format!("unknown auth mode `{label}`"))),
    }
}

fn parse_protocol_family(label: &str) -> Result<ProtocolFamily, ToolCallError> {
    match label {
        "hue" => Ok(ProtocolFamily::Hue),
        "zigbee" => Ok(ProtocolFamily::Zigbee),
        "zwave" | "z_wave" | "z-wave" => Ok(ProtocolFamily::ZWave),
        "thread" => Ok(ProtocolFamily::Thread),
        "matter" => Ok(ProtocolFamily::Matter),
        "mqtt" => Ok(ProtocolFamily::Mqtt),
        value if value.starts_with("vendor:") && value.len() > "vendor:".len() => {
            Ok(ProtocolFamily::Vendor(value["vendor:".len()..].to_string()))
        }
        _ => Err(validation_error(format!(
            "unknown protocol family `{label}`"
        ))),
    }
}

fn protocol_family_label(family: &ProtocolFamily) -> String {
    match family {
        ProtocolFamily::Hue => "hue".to_string(),
        ProtocolFamily::Zigbee => "zigbee".to_string(),
        ProtocolFamily::ZWave => "zwave".to_string(),
        ProtocolFamily::Thread => "thread".to_string(),
        ProtocolFamily::Matter => "matter".to_string(),
        ProtocolFamily::Mqtt => "mqtt".to_string(),
        ProtocolFamily::Vendor(value) => format!("vendor:{value}"),
    }
}

fn parse_integration_catalog_sort(label: &str) -> Result<IntegrationCatalogSort, ToolCallError> {
    match label {
        "priority_then_name" => Ok(IntegrationCatalogSort::PriorityThenName),
        "name" => Ok(IntegrationCatalogSort::Name),
        "category_then_priority" => Ok(IntegrationCatalogSort::CategoryThenPriority),
        "status_then_priority" => Ok(IntegrationCatalogSort::StatusThenPriority),
        _ => Err(validation_error(format!(
            "unknown integration sort `{label}`"
        ))),
    }
}

fn integration_category_label(category: IntegrationCategory) -> &'static str {
    match category {
        IntegrationCategory::ProtocolStandard => "protocol_standard",
        IntegrationCategory::LocalHub => "local_hub",
        IntegrationCategory::LocalDevice => "local_device",
        IntegrationCategory::BluetoothProfile => "bluetooth_profile",
        IntegrationCategory::CloudHub => "cloud_hub",
        IntegrationCategory::CameraMedia => "camera_media",
        IntegrationCategory::EnergyClimate => "energy_climate",
        IntegrationCategory::NotificationChannel => "notification_channel",
        IntegrationCategory::DataService => "data_service",
        IntegrationCategory::HelperCalculated => "helper_calculated",
        IntegrationCategory::VirtualAlias => "virtual_alias",
        IntegrationCategory::SystemHardware => "system_hardware",
    }
}

fn connectivity_class_label(connectivity: ConnectivityClass) -> &'static str {
    match connectivity {
        ConnectivityClass::LocalPush => "local_push",
        ConnectivityClass::LocalPolling => "local_polling",
        ConnectivityClass::CloudPush => "cloud_push",
        ConnectivityClass::CloudPolling => "cloud_polling",
        ConnectivityClass::Calculated => "calculated",
        ConnectivityClass::AssumedState => "assumed_state",
    }
}

fn implementation_status_label(status: ImplementationStatus) -> &'static str {
    match status {
        ImplementationStatus::Cataloged => "cataloged",
        ImplementationStatus::Specified => "specified",
        ImplementationStatus::Scaffolded => "scaffolded",
        ImplementationStatus::Simulated => "simulated",
        ImplementationStatus::FirstPartyRuntime => "first_party_runtime",
        ImplementationStatus::ProductionReady => "production_ready",
        ImplementationStatus::DelegatedToStandard => "delegated_to_standard",
        ImplementationStatus::Unsupported => "unsupported",
    }
}

fn discovery_mechanism_label(mechanism: DiscoveryMechanism) -> &'static str {
    match mechanism {
        DiscoveryMechanism::Mdns => "mdns",
        DiscoveryMechanism::Ssdp => "ssdp",
        DiscoveryMechanism::Bluetooth => "bluetooth",
        DiscoveryMechanism::Usb => "usb",
        DiscoveryMechanism::Dhcp => "dhcp",
        DiscoveryMechanism::Mqtt => "mqtt",
        DiscoveryMechanism::Manual => "manual",
        DiscoveryMechanism::CloudAccount => "cloud_account",
        DiscoveryMechanism::Webhook => "webhook",
        DiscoveryMechanism::FileConfig => "file_config",
    }
}

fn auth_mode_label(mode: AuthMode) -> &'static str {
    match mode {
        AuthMode::None => "none",
        AuthMode::LocalPairing => "local_pairing",
        AuthMode::LocalToken => "local_token",
        AuthMode::UsernamePassword => "username_password",
        AuthMode::OAuth2 => "oauth2",
        AuthMode::ApiKey => "api_key",
        AuthMode::Certificate => "certificate",
        AuthMode::RadioNetworkKey => "radio_network_key",
        AuthMode::MqttCredentials => "mqtt_credentials",
    }
}

fn runtime_kind_label(kind: RuntimeKind) -> &'static str {
    match kind {
        RuntimeKind::InProcessRust => "in_process_rust",
        RuntimeKind::RustWorkerProcess => "rust_worker_process",
    }
}

fn scene_scope_label(scope: SceneScope) -> &'static str {
    match scope {
        SceneScope::Room => "room",
        SceneScope::Zone => "zone",
        SceneScope::Home => "home",
        SceneScope::Bridge => "bridge",
        SceneScope::Custom => "custom",
    }
}

fn entity_kind_label(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Light => "light",
        EntityKind::LightGroup => "light_group",
        EntityKind::Switch => "switch",
        EntityKind::Sensor => "sensor",
        EntityKind::Lock => "lock",
        EntityKind::Thermostat => "thermostat",
        EntityKind::Scene => "scene",
        EntityKind::Input => "input",
        EntityKind::BridgeHealth => "bridge_health",
        EntityKind::NetworkDiagnostic => "network_diagnostic",
        EntityKind::Unknown => "unknown",
    }
}

fn privilege_tier_label(tier: PrivilegeTier) -> &'static str {
    match tier {
        PrivilegeTier::ReadOnly => "read_only",
        PrivilegeTier::LowRisk => "low_risk",
        PrivilegeTier::HumanApproval => "human_approval",
        PrivilegeTier::HighRisk => "high_risk",
    }
}

fn capability_grant_status_label(status: CapabilityGrantStatus) -> &'static str {
    match status {
        CapabilityGrantStatus::Pending => "pending",
        CapabilityGrantStatus::Active => "active",
        CapabilityGrantStatus::Revoked => "revoked",
        CapabilityGrantStatus::Expired => "expired",
    }
}

fn capability_grant_scope_label(scope: &CapabilityGrantScope) -> &'static str {
    match scope {
        CapabilityGrantScope::Tool(_) => "tool",
        CapabilityGrantScope::Capability(_) => "capability",
        CapabilityGrantScope::EntityCapability { .. } => "entity_capability",
        CapabilityGrantScope::AllSmartHome => "all_smart_home",
    }
}

fn authorization_outcome_label(outcome: AuthorizationOutcome) -> &'static str {
    match outcome {
        AuthorizationOutcome::Allowed => "allowed",
        AuthorizationOutcome::Denied => "denied",
    }
}

fn authorization_subject_label(subject: &AuthorizationSubject) -> &'static str {
    match subject {
        AuthorizationSubject::Tool(_) => "tool",
        AuthorizationSubject::Command { .. } => "command",
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

fn device_event_type_label(event_type: DeviceEventType) -> &'static str {
    match event_type {
        DeviceEventType::Discovered => "discovered",
        DeviceEventType::Updated => "updated",
        DeviceEventType::Removed => "removed",
        DeviceEventType::Unavailable => "unavailable",
        DeviceEventType::Error => "error",
        DeviceEventType::Health => "health",
    }
}

fn reconciliation_reason_label(reason: ReconciliationReason) -> &'static str {
    match reason {
        ReconciliationReason::MissingState => "missing",
        ReconciliationReason::StaleState => "stale",
        ReconciliationReason::Drifted => "drifted",
    }
}

fn state_refresh_reason_label(reason: StateRefreshReason) -> &'static str {
    match reason {
        StateRefreshReason::Missing => "missing",
        StateRefreshReason::Stale => "stale",
    }
}

fn worker_restart_reason_label(reason: WorkerRestartReason) -> &'static str {
    match reason {
        WorkerRestartReason::HeartbeatOverdue => "heartbeat_overdue",
    }
}

fn pairing_status_label(status: PairingSessionStatus) -> &'static str {
    status.as_str()
}

fn command_type_label(command_type: CommandType) -> &'static str {
    match command_type {
        CommandType::TurnOn => "turn_on",
        CommandType::TurnOff => "turn_off",
        CommandType::SetBrightness => "set_brightness",
        CommandType::SetColor => "set_color",
        CommandType::SetColorTemperature => "set_color_temperature",
        CommandType::RecallScene => "recall_scene",
        CommandType::SetLock => "set_lock",
        CommandType::SetThermostatSetpoint => "set_thermostat_setpoint",
    }
}

fn parse_pairing_status(label: &str) -> Result<PairingSessionStatus, ToolCallError> {
    match label {
        "pending_user_presence" | "pending" => Ok(PairingSessionStatus::PendingUserPresence),
        "completed" => Ok(PairingSessionStatus::Completed),
        "expired" => Ok(PairingSessionStatus::Expired),
        "cancelled" | "canceled" => Ok(PairingSessionStatus::Cancelled),
        _ => Err(validation_error(format!(
            "unknown pairing session status `{label}`"
        ))),
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

fn optional_string_list(
    value: &JsonValue,
    singular_field: &str,
    plural_field: &str,
) -> Result<Vec<String>, ToolCallError> {
    let mut values = Vec::new();
    if let Some(value) = optional_string(value, singular_field)? {
        values.push(value);
    }
    match optional_field(value, plural_field) {
        Some(JsonValue::String(value)) => values.push(value.clone()),
        Some(JsonValue::Array(items)) => {
            for item in items {
                match item {
                    JsonValue::String(value) => values.push(value.clone()),
                    _ => {
                        return Err(validation_error(format!(
                            "{plural_field} must contain string values"
                        )))
                    }
                }
            }
        }
        Some(JsonValue::Null) | None => {}
        Some(_) => {
            return Err(validation_error(format!(
                "{plural_field} must be a string or string array"
            )))
        }
    }
    Ok(values)
}

fn optional_primitive_list(
    value: &JsonValue,
    field: &str,
) -> Result<Vec<PrimitiveFamily>, ToolCallError> {
    optional_string_list_field(value, field)?
        .iter()
        .map(|label| parse_primitive_family(label))
        .collect()
}

fn optional_capability_id_list(
    value: &JsonValue,
    field: &str,
) -> Result<Vec<CapabilityId>, ToolCallError> {
    Ok(optional_string_list_field(value, field)?
        .into_iter()
        .map(CapabilityId::trusted)
        .collect())
}

fn optional_integration_id_list(
    value: &JsonValue,
    field: &str,
) -> Result<Vec<IntegrationId>, ToolCallError> {
    Ok(optional_string_list_field(value, field)?
        .into_iter()
        .map(IntegrationId::trusted)
        .collect())
}

fn optional_string_list_field(
    value: &JsonValue,
    field: &str,
) -> Result<Vec<String>, ToolCallError> {
    match optional_field(value, field) {
        Some(JsonValue::String(value)) => Ok(vec![value.clone()]),
        Some(JsonValue::Array(items)) => items
            .iter()
            .map(|item| match item {
                JsonValue::String(value) => Ok(value.clone()),
                _ => Err(validation_error(format!("{field} must contain strings"))),
            })
            .collect(),
        Some(JsonValue::Null) | None => Ok(Vec::new()),
        Some(_) => Err(validation_error(format!(
            "{field} must be a string or string array"
        ))),
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

fn optional_u8(value: &JsonValue, field: &str) -> Result<Option<u8>, ToolCallError> {
    optional_u64(value, field)?
        .map(|value| {
            u8::try_from(value)
                .map_err(|_| validation_error(format!("{field} must be less than or equal to 255")))
        })
        .transpose()
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

fn json_field<'a>(value: &'a JsonValue, name: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find_map(|(field_name, value)| (field_name == name).then_some(value))
}

fn json_integer(value: &JsonValue) -> Option<i64> {
    match value {
        JsonValue::Number(JsonNumber::Integer(value)) => Some(*value),
        _ => None,
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

fn required_field<'a>(value: &'a JsonValue, field: &str) -> Result<&'a JsonValue, ToolCallError> {
    optional_field(value, field).ok_or_else(|| validation_error(format!("{field} is required")))
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

fn string_array_schema() -> JsonSchema {
    JsonSchema::Array {
        items: Box::new(JsonSchema::String),
    }
}

fn integration_catalog_query_schema() -> JsonSchema {
    let string_array = || JsonSchema::Array {
        items: Box::new(JsonSchema::String),
    };
    object_schema(
        vec![
            SchemaProperty::new("category", JsonSchema::String),
            SchemaProperty::new("categories", string_array()),
            SchemaProperty::new("connectivity", JsonSchema::String),
            SchemaProperty::new("connectivity_classes", string_array()),
            SchemaProperty::new("implementation_status", JsonSchema::String),
            SchemaProperty::new("implementation_statuses", string_array()),
            SchemaProperty::new("required_primitive", JsonSchema::String),
            SchemaProperty::new("required_primitives", string_array()),
            SchemaProperty::new("required_capability_id", JsonSchema::String),
            SchemaProperty::new("required_capability_ids", string_array()),
            SchemaProperty::new("policy_surface", JsonSchema::String),
            SchemaProperty::new("policy_surfaces", string_array()),
            SchemaProperty::new("discovery_mechanism", JsonSchema::String),
            SchemaProperty::new("discovery_mechanisms", string_array()),
            SchemaProperty::new("auth_mode", JsonSchema::String),
            SchemaProperty::new("auth_modes", string_array()),
            SchemaProperty::new("protocol_family", JsonSchema::String),
            SchemaProperty::new("protocol_families", string_array()),
            SchemaProperty::new("priority_at_or_before", JsonSchema::Integer),
            SchemaProperty::new("include_virtual_aliases", JsonSchema::Boolean),
            SchemaProperty::new("local_only", JsonSchema::Boolean),
            SchemaProperty::new("cloud_required", JsonSchema::Boolean),
            SchemaProperty::new("sort", JsonSchema::String),
            SchemaProperty::new("limit", JsonSchema::Integer),
        ],
        Vec::new(),
        false,
    )
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

fn event_delivery_output_schema() -> JsonSchema {
    object_schema(
        vec![
            SchemaProperty::new("subscription_id", JsonSchema::String),
            SchemaProperty::new("delivered_events", JsonSchema::Integer),
            SchemaProperty::new("remaining_events", JsonSchema::Integer),
            SchemaProperty::new("has_more", JsonSchema::Boolean),
            SchemaProperty::new("summary", JsonSchema::Any),
            SchemaProperty::new(
                "events",
                JsonSchema::Array {
                    items: Box::new(JsonSchema::Any),
                },
            ),
        ],
        vec![
            "subscription_id",
            "delivered_events",
            "remaining_events",
            "has_more",
            "summary",
            "events",
        ],
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
    use smart_home_core::{CapabilityGrant, CapabilityGrantId};
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

        assert_eq!(definitions.len(), 41);
        assert!(export.ok());
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_LIST_INTEGRATIONS_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_DESCRIBE_INTEGRATION_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_LIST_PRIMITIVES_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_DESCRIBE_PRIMITIVE_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_DISCOVER_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_LIST_DISCOVERY_WORKERS_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_GET_DISCOVERY_SUMMARY_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_GET_PAIRING_PLAN_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_LIST_ROOMS_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_LIST_SCENES_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_DESCRIBE_SCENE_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_COMMAND_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_PAIR_BRIDGE_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_COMPLETE_PAIRING_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_REPORT_EVENT_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_SUBSCRIBE_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_POLL_EVENTS_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_UNSUBSCRIBE_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_LIST_SUBSCRIPTIONS_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_INSPECT_EVENT_LOG_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_LIST_AUTHORIZATION_DECISIONS_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_GET_AUTHORIZATION_SUMMARY_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_LIST_CAPABILITY_GRANTS_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_GET_CAPABILITY_GRANT_SUMMARY_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_GET_RUNTIME_SNAPSHOT_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_GET_TOPOLOGY_SUMMARY_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_LIST_DESIRED_STATES_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_SET_DESIRED_STATE_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_CLEAR_DESIRED_STATE_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_LIST_PAIRING_SESSIONS_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_LIST_WORKERS_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_GET_WORKER_HEARTBEAT_SCHEDULE_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_GET_SUPERVISION_PLAN_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_RECONCILE_DESIRED_STATES_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_RUN_SUPERVISION_TICK_TOOL_ID));
        assert!(export
            .tool_ids()
            .contains(&SMART_HOME_DESCRIBE_CAPABILITIES_TOOL_ID));
        assert!(export.tool_ids().contains(&SMART_HOME_GET_HEALTH_TOOL_ID));
        assert_eq!(
            export.summary.required_capability_count("smart_home:read"),
            33
        );
        assert_eq!(
            export
                .summary
                .required_capability_count("smart_home:command"),
            5
        );
        assert_eq!(
            export.summary.required_capability_count("smart_home:pair"),
            2
        );
        assert_eq!(
            export
                .summary
                .required_capability_count("smart_home:ingest"),
            1
        );
        assert!(smart_home_tool_definition(SMART_HOME_GET_STATE_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_LIST_DISCOVERY_WORKERS_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_GET_DISCOVERY_SUMMARY_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_GET_PAIRING_PLAN_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_COMPLETE_PAIRING_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_REPORT_EVENT_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_LIST_ROOMS_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_LIST_SCENES_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_DESCRIBE_SCENE_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_GET_TOPOLOGY_SUMMARY_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_SET_DESIRED_STATE_TOOL_ID).is_some());
        assert!(smart_home_tool_definition(SMART_HOME_CLEAR_DESIRED_STATE_TOOL_ID).is_some());
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
        runtime
            .borrow_mut()
            .supervisor_mut()
            .register_worker(SupervisedBridgeWorker::new(
                BridgeId::trusted("bridge-1"),
                IntegrationId::trusted("hue"),
                1_000,
                750,
            ));
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

        let list_integrations_request = request(
            "call-list-integrations",
            SMART_HOME_LIST_INTEGRATIONS_TOOL_ID,
            object([
                ("required_primitive", string("local_http")),
                ("priority_at_or_before", integer(0)),
                ("limit", integer(2)),
            ]),
            990,
        );
        let list_integrations_trace = tool_runtime.invoke_with_events(&list_integrations_request);
        assert!(list_integrations_trace.result.ok);
        let list_integrations_output = list_integrations_trace.result.output.as_ref().unwrap();
        assert_eq!(list_integrations_trace.summary().progress_event_count, 1);
        let integration_count =
            integer_value(field(list_integrations_output, "count").unwrap()).unwrap();
        assert!((1..=2).contains(&integration_count));
        assert_eq!(
            array_len(field(list_integrations_output, "integrations").unwrap()),
            Some(integration_count as usize)
        );

        let describe_integration_request = request(
            "call-describe-integration",
            SMART_HOME_DESCRIBE_INTEGRATION_TOOL_ID,
            object([
                ("integration_id", string("hue")),
                (
                    "available_primitives",
                    JsonValue::Array(vec![string("mdns"), string("local_http")]),
                ),
                (
                    "allowed_capability_ids",
                    JsonValue::Array(vec![string("smart_home.read")]),
                ),
            ]),
            991,
        );
        let describe_integration_trace =
            tool_runtime.invoke_with_events(&describe_integration_request);
        assert!(describe_integration_trace.result.ok);
        let describe_integration_output =
            describe_integration_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(
                field(describe_integration_output, "integration").unwrap(),
                "integration_id"
            ),
            Some(&string("hue"))
        );
        assert_eq!(
            field(
                field(describe_integration_output, "readiness_report").unwrap(),
                "activation_ready"
            ),
            Some(&JsonValue::Bool(false))
        );

        let list_primitives_request = request(
            "call-list-primitives",
            SMART_HOME_LIST_PRIMITIVES_TOOL_ID,
            object([
                ("priority_at_or_before", integer(0)),
                ("include_ecosystem_coverage", JsonValue::Bool(true)),
                ("limit", integer(3)),
            ]),
            992,
        );
        let list_primitives_trace = tool_runtime.invoke_with_events(&list_primitives_request);
        assert!(list_primitives_trace.result.ok);
        let list_primitives_output = list_primitives_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(list_primitives_output, "backlog_count"),
            Some(&integer(3))
        );
        assert_eq!(
            array_len(field(list_primitives_output, "backlog").unwrap()),
            Some(3)
        );

        let describe_primitive_request = request(
            "call-describe-primitive",
            SMART_HOME_DESCRIBE_PRIMITIVE_TOOL_ID,
            object([("primitive", string("mdns"))]),
            993,
        );
        let describe_primitive_trace = tool_runtime.invoke_with_events(&describe_primitive_request);
        assert!(describe_primitive_trace.result.ok);
        let describe_primitive_output = describe_primitive_trace.result.output.as_ref().unwrap();
        assert!(
            integer_value(field(describe_primitive_output, "integration_count").unwrap()).unwrap()
                >= 1
        );
        assert_eq!(
            field(
                field(describe_primitive_output, "primitive").unwrap(),
                "primitive"
            ),
            Some(&string("mdns"))
        );

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

        let list_rooms_request = request(
            "call-list-rooms",
            SMART_HOME_LIST_ROOMS_TOOL_ID,
            object([
                ("room_id", string("kitchen")),
                ("sort", string("scenes_desc")),
            ]),
            1_001,
        );
        let list_rooms_trace = tool_runtime.invoke_with_events(&list_rooms_request);
        assert!(list_rooms_trace.result.ok);
        let list_rooms_output = list_rooms_trace.result.output.as_ref().unwrap();
        assert_eq!(field(list_rooms_output, "count"), Some(&integer(1)));
        let room_summary = array_item(field(list_rooms_output, "rooms").unwrap(), 0).unwrap();
        assert_eq!(field(room_summary, "room_id"), Some(&string("kitchen")));
        assert_eq!(field(room_summary, "device_count"), Some(&integer(1)));
        assert_eq!(field(room_summary, "entity_count"), Some(&integer(2)));
        assert_eq!(field(room_summary, "scene_count"), Some(&integer(1)));
        assert_eq!(field(room_summary, "scene_action_count"), Some(&integer(1)));
        assert_eq!(
            field(
                field(list_rooms_output, "topology").unwrap(),
                "unique_rooms"
            ),
            Some(&integer(1))
        );

        let list_scenes_request = request(
            "call-list-scenes",
            SMART_HOME_LIST_SCENES_TOOL_ID,
            object([
                ("scope", string("room")),
                ("capability_id", string("light.on_off")),
            ]),
            1_001,
        );
        let list_scenes_trace = tool_runtime.invoke_with_events(&list_scenes_request);
        assert!(list_scenes_trace.result.ok);
        let list_scenes_output = list_scenes_trace.result.output.as_ref().unwrap();
        assert_eq!(field(list_scenes_output, "count"), Some(&integer(1)));
        let scene_summary = array_item(field(list_scenes_output, "scenes").unwrap(), 0).unwrap();
        assert_eq!(
            field(scene_summary, "scene_id"),
            Some(&string("scene-kitchen-bright"))
        );
        assert_eq!(field(scene_summary, "scope"), Some(&string("room")));

        let describe_scene_request = request(
            "call-describe-scene",
            SMART_HOME_DESCRIBE_SCENE_TOOL_ID,
            object([("scene_id", string("scene-kitchen-bright"))]),
            1_002,
        );
        let describe_scene_trace = tool_runtime.invoke_with_events(&describe_scene_request);
        assert!(describe_scene_trace.result.ok);
        let describe_scene_output = describe_scene_trace.result.output.as_ref().unwrap();
        let scene = field(describe_scene_output, "scene").unwrap();
        assert_eq!(
            field(describe_scene_output, "scene_id"),
            Some(&string("scene-kitchen-bright"))
        );
        assert_eq!(field(scene, "action_count"), Some(&integer(1)));
        assert_eq!(
            field(
                array_item(field(scene, "actions").unwrap(), 0).unwrap(),
                "entity_id"
            ),
            Some(&string("entity-light-1"))
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

        let list_discovery_workers_request = request(
            "call-list-discovery-workers",
            SMART_HOME_LIST_DISCOVERY_WORKERS_TOOL_ID,
            object([
                ("integration_id", string("hue")),
                ("kind", string("mdns")),
                ("source", string("mdns")),
                ("overdue_at_ms", integer(1_055)),
                ("sort", string("next_due_at")),
                ("limit", integer(1)),
            ]),
            1_055,
        );
        let list_discovery_workers_trace =
            tool_runtime.invoke_with_events(&list_discovery_workers_request);
        assert!(list_discovery_workers_trace.result.ok);
        let list_discovery_workers_output =
            list_discovery_workers_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(list_discovery_workers_output, "count"),
            Some(&integer(1))
        );
        let discovery_worker =
            array_item(field(list_discovery_workers_output, "workers").unwrap(), 0).unwrap();
        assert_eq!(
            field(discovery_worker, "worker_id"),
            Some(&string("hue-mdns-worker"))
        );
        assert_eq!(
            field(discovery_worker, "is_due"),
            Some(&JsonValue::Bool(true))
        );
        assert_eq!(
            field(
                field(list_discovery_workers_output, "summary").unwrap(),
                "due_worker_count"
            ),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                field(list_discovery_workers_output, "summary").unwrap(),
                "has_due_work"
            ),
            Some(&JsonValue::Bool(true))
        );

        let discovery_summary_request = request(
            "call-get-discovery-summary",
            SMART_HOME_GET_DISCOVERY_SUMMARY_TOOL_ID,
            object([
                ("integration_id", string("hue")),
                ("source", string("mdns")),
                ("fresh_only", JsonValue::Bool(true)),
                ("ttl_ms", integer(1_000)),
            ]),
            1_056,
        );
        let discovery_summary_trace = tool_runtime.invoke_with_events(&discovery_summary_request);
        assert!(discovery_summary_trace.result.ok);
        let discovery_summary_output = discovery_summary_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(
                field(discovery_summary_output, "record_summary").unwrap(),
                "total"
            ),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                field(discovery_summary_output, "signal_summary").unwrap(),
                "fresh"
            ),
            Some(&integer(1))
        );

        let pairing_plan_request = request(
            "call-get-pairing-plan",
            SMART_HOME_GET_PAIRING_PLAN_TOOL_ID,
            object([
                ("integration_id", string("hue")),
                ("source", string("mdns")),
                ("pairing_requirement", string("physical_presence")),
                ("actionable_only", JsonValue::Bool(true)),
                ("ttl_ms", integer(1_000)),
                ("limit", integer(1)),
            ]),
            1_057,
        );
        let pairing_plan_trace = tool_runtime.invoke_with_events(&pairing_plan_request);
        assert!(pairing_plan_trace.result.ok);
        let pairing_plan_output = pairing_plan_trace.result.output.as_ref().unwrap();
        assert_eq!(field(pairing_plan_output, "count"), Some(&integer(1)));
        let pairing_target = array_item(field(pairing_plan_output, "targets").unwrap(), 0).unwrap();
        assert_eq!(
            field(pairing_target, "bridge_id"),
            Some(&string("hue.bridge.001788fffediscovered"))
        );
        assert_eq!(
            field(pairing_target, "action"),
            Some(&string("press_physical_button"))
        );
        assert_eq!(
            field(pairing_target, "requires_human_action"),
            Some(&JsonValue::Bool(true))
        );
        assert_eq!(
            field(field(pairing_plan_output, "summary").unwrap(), "actionable"),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                field(pairing_plan_output, "summary").unwrap(),
                "requires_human_action"
            ),
            Some(&integer(1))
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

        let list_workers_request = request(
            "call-list-workers",
            SMART_HOME_LIST_WORKERS_TOOL_ID,
            object([
                ("status", string("starting")),
                ("sort", string("heartbeat_due_at")),
            ]),
            1_025,
        );
        let list_workers_trace = tool_runtime.invoke_with_events(&list_workers_request);
        assert!(list_workers_trace.result.ok);
        let list_workers_output = list_workers_trace.result.output.as_ref().unwrap();
        assert_eq!(field(list_workers_output, "count"), Some(&integer(1)));
        assert_eq!(
            field(
                field(list_workers_output, "summary").unwrap(),
                "worker_count"
            ),
            Some(&integer(1))
        );
        let worker_output = array_item(field(list_workers_output, "workers").unwrap(), 0).unwrap();
        assert_eq!(field(worker_output, "bridge_id"), Some(&string("bridge-1")));
        assert_eq!(field(worker_output, "status"), Some(&string("starting")));
        assert_eq!(
            field(worker_output, "heartbeat_due_at_ms"),
            Some(&integer(1_750))
        );
        assert_eq!(
            field(worker_output, "is_overdue"),
            Some(&JsonValue::Bool(false))
        );

        let heartbeat_schedule_request = request(
            "call-worker-heartbeat-schedule",
            SMART_HOME_GET_WORKER_HEARTBEAT_SCHEDULE_TOOL_ID,
            object([
                ("bridge_id", string("bridge-1")),
                ("due_at_or_before_ms", integer(2_000)),
            ]),
            1_026,
        );
        let heartbeat_schedule_trace = tool_runtime.invoke_with_events(&heartbeat_schedule_request);
        assert!(heartbeat_schedule_trace.result.ok);
        let heartbeat_schedule_output = heartbeat_schedule_trace.result.output.as_ref().unwrap();
        assert_eq!(field(heartbeat_schedule_output, "count"), Some(&integer(1)));
        assert_eq!(
            field(heartbeat_schedule_output, "due_count"),
            Some(&integer(0))
        );
        assert_eq!(
            field(heartbeat_schedule_output, "next_due_at_ms"),
            Some(&integer(1_750))
        );
        let deadline_output =
            array_item(field(heartbeat_schedule_output, "deadlines").unwrap(), 0).unwrap();
        assert_eq!(field(deadline_output, "due_at_ms"), Some(&integer(1_750)));
        assert_eq!(
            field(deadline_output, "is_due"),
            Some(&JsonValue::Bool(false))
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

        let complete_pairing_request = request(
            "call-complete-pairing",
            SMART_HOME_COMPLETE_PAIRING_TOOL_ID,
            object([
                ("session_id", string("pairing-session-1")),
                (
                    "vault_ref",
                    string("vault://smart-home/hue/bridge-1/app-key"),
                ),
                ("completed_at_ms", integer(1_050)),
                (
                    "metadata",
                    object([("credential_kind", string("application_key"))]),
                ),
            ]),
            1_050,
        );
        let complete_pairing_trace = tool_runtime.invoke_with_events(&complete_pairing_request);
        assert!(complete_pairing_trace.result.ok);
        let complete_pairing_output = complete_pairing_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(complete_pairing_output, "status"),
            Some(&string("completed"))
        );
        assert_eq!(
            field(complete_pairing_output, "vault_ref"),
            Some(&string("vault://smart-home/hue/bridge-1/app-key"))
        );

        let set_desired_state_request = request(
            "call-set-desired-state",
            SMART_HOME_SET_DESIRED_STATE_TOOL_ID,
            object([
                ("entity_id", string("entity-light-1")),
                (
                    "desired",
                    JsonValue::Array(vec![object([
                        ("capability_id", string("light.on_off")),
                        ("value", JsonValue::Bool(true)),
                    ])]),
                ),
                ("requested_by", string("agent:scene-planner")),
                ("command_timeout_ms", integer(750)),
            ]),
            1_099,
        );
        let set_desired_state_trace = tool_runtime.invoke_with_events(&set_desired_state_request);
        assert!(set_desired_state_trace.result.ok);
        assert_eq!(set_desired_state_trace.summary().progress_event_count, 1);
        let set_desired_state_output = set_desired_state_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(set_desired_state_output, "entity_id"),
            Some(&string("entity-light-1"))
        );
        assert_eq!(
            field(set_desired_state_output, "replaced"),
            Some(&JsonValue::Bool(false))
        );
        assert_eq!(
            field(set_desired_state_output, "desired_capability_count"),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                field(set_desired_state_output, "desired_state").unwrap(),
                "requested_by"
            ),
            Some(&string("agent:scene-planner"))
        );

        let command_request = request(
            "call-turn-on",
            SMART_HOME_COMMAND_TOOL_ID,
            object([
                ("entity_id", string("entity-light-1")),
                ("command_type", string("turn_on")),
                ("idempotency_key", string("demo-turn-on")),
                ("timeout_ms", integer(750)),
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

        let report_event_request = request(
            "call-report-event",
            SMART_HOME_REPORT_EVENT_TOOL_ID,
            object([
                ("event_kind", string("device")),
                ("event_id", string("hue-event-1")),
                ("bridge_id", string("bridge-1")),
                ("device_id", string("device-1")),
                ("entity_id", string("entity-light-1")),
                ("event_type", string("updated")),
                ("observed_at_ms", integer(1_101)),
                ("received_at_ms", integer(1_101)),
                ("capability_id", string("light.on_off")),
                ("value", JsonValue::Bool(true)),
                ("raw_ref", string("event-log://hue/bridge-1/1")),
                ("correlation_id", string("hue-sse-1")),
                ("metadata", object([("source", string("hue_sse"))])),
            ]),
            1_101,
        );
        let report_event_trace = tool_runtime.invoke_with_events(&report_event_request);
        assert!(report_event_trace.result.ok);
        assert_eq!(report_event_trace.summary().progress_event_count, 1);
        let report_event_output = report_event_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(report_event_output, "event_kind"),
            Some(&string("device"))
        );
        assert_eq!(
            field(report_event_output, "event_type"),
            Some(&string("updated"))
        );
        assert_eq!(
            field(
                field(report_event_output, "state_delta").unwrap(),
                "capability_id"
            ),
            Some(&string("light.on_off"))
        );

        let report_health_request = request(
            "call-report-health",
            SMART_HOME_REPORT_EVENT_TOOL_ID,
            object([
                ("event_kind", string("bridge_health")),
                ("event_id", string("hue-health-1")),
                ("bridge_id", string("bridge-1")),
                ("health", string("online")),
                ("observed_at_ms", integer(1_102)),
                ("received_at_ms", integer(1_102)),
                ("metadata", object([("source", string("heartbeat"))])),
            ]),
            1_102,
        );
        let report_health_trace = tool_runtime.invoke_with_events(&report_health_request);
        assert!(report_health_trace.result.ok);
        assert_eq!(report_health_trace.summary().progress_event_count, 1);
        let report_health_output = report_health_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(report_health_output, "event_kind"),
            Some(&string("bridge_health"))
        );
        assert_eq!(
            field(report_health_output, "health"),
            Some(&string("online"))
        );

        let runtime_snapshot_request = request(
            "call-runtime-snapshot",
            SMART_HOME_GET_RUNTIME_SNAPSHOT_TOOL_ID,
            object([]),
            1_100,
        );
        let runtime_snapshot_trace = tool_runtime.invoke_with_events(&runtime_snapshot_request);
        assert!(runtime_snapshot_trace.result.ok);
        let runtime_snapshot_output = runtime_snapshot_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(runtime_snapshot_output, "pairing_session_count"),
            Some(&integer(1))
        );
        assert_eq!(
            field(runtime_snapshot_output, "desired_state_count"),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                field(runtime_snapshot_output, "pending_work").unwrap(),
                "event_backlog_count"
            ),
            Some(&integer(1))
        );

        let topology_summary_request = request(
            "call-topology-summary",
            SMART_HOME_GET_TOPOLOGY_SUMMARY_TOOL_ID,
            object([]),
            1_100,
        );
        let topology_summary_trace = tool_runtime.invoke_with_events(&topology_summary_request);
        assert!(topology_summary_trace.result.ok);
        let topology_summary_output = topology_summary_trace.result.output.as_ref().unwrap();
        let topology_summary = field(topology_summary_output, "summary").unwrap();
        assert_eq!(
            field(topology_summary, "devices_with_room"),
            Some(&integer(1))
        );
        assert_eq!(field(topology_summary, "unique_rooms"), Some(&integer(1)));
        assert_eq!(field(topology_summary, "room_scenes"), Some(&integer(1)));
        assert_eq!(
            field(topology_summary, "has_scene_actions"),
            Some(&JsonValue::Bool(true))
        );

        let supervision_plan_request = request(
            "call-supervision-plan",
            SMART_HOME_GET_SUPERVISION_PLAN_TOOL_ID,
            object([]),
            1_100,
        );
        let supervision_plan_trace = tool_runtime.invoke_with_events(&supervision_plan_request);
        assert!(supervision_plan_trace.result.ok);
        let supervision_plan_output = supervision_plan_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(supervision_plan_output, "action_count"),
            Some(&integer(2))
        );
        assert_eq!(
            field(
                field(supervision_plan_output, "summary").unwrap(),
                "total_actions"
            ),
            Some(&integer(2))
        );
        assert_eq!(
            field(
                field(supervision_plan_output, "summary").unwrap(),
                "discovery_worker_run_count"
            ),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                field(supervision_plan_output, "discovery_worker_run_plan").unwrap(),
                "count"
            ),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                array_item(
                    field(
                        field(supervision_plan_output, "discovery_worker_run_plan").unwrap(),
                        "instructions"
                    )
                    .unwrap(),
                    0
                )
                .unwrap(),
                "worker_id"
            ),
            Some(&string("hue-mdns-worker"))
        );

        let desired_states_request = request(
            "call-list-desired-states",
            SMART_HOME_LIST_DESIRED_STATES_TOOL_ID,
            object([
                ("capability_id", string("light.on_off")),
                ("sort", string("command_timeout_desc")),
            ]),
            1_100,
        );
        let desired_states_trace = tool_runtime.invoke_with_events(&desired_states_request);
        assert!(desired_states_trace.result.ok);
        let desired_states_output = desired_states_trace.result.output.as_ref().unwrap();
        assert_eq!(field(desired_states_output, "count"), Some(&integer(1)));
        assert_eq!(
            field(
                array_item(field(desired_states_output, "desired_states").unwrap(), 0).unwrap(),
                "requested_by"
            ),
            Some(&string("agent:scene-planner"))
        );
        assert_eq!(
            field(
                field(desired_states_output, "summary").unwrap(),
                "total_desired_capabilities"
            ),
            Some(&integer(1))
        );

        let pairing_sessions_request = request(
            "call-list-pairing-sessions",
            SMART_HOME_LIST_PAIRING_SESSIONS_TOOL_ID,
            object([
                ("bridge_id", string("bridge-1")),
                ("status", string("completed")),
                ("sort", string("expires_at")),
            ]),
            1_100,
        );
        let pairing_sessions_trace = tool_runtime.invoke_with_events(&pairing_sessions_request);
        assert!(pairing_sessions_trace.result.ok);
        let pairing_sessions_output = pairing_sessions_trace.result.output.as_ref().unwrap();
        assert_eq!(field(pairing_sessions_output, "count"), Some(&integer(1)));
        assert_eq!(
            field(
                array_item(field(pairing_sessions_output, "sessions").unwrap(), 0).unwrap(),
                "session_id"
            ),
            Some(&string("pairing-session-1"))
        );
        assert_eq!(
            field(
                field(pairing_sessions_output, "summary").unwrap(),
                "completed_sessions"
            ),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                field(pairing_sessions_output, "summary").unwrap(),
                "sessions_with_vault_ref"
            ),
            Some(&integer(1))
        );

        let list_subscriptions_request = request(
            "call-list-subscriptions",
            SMART_HOME_LIST_SUBSCRIPTIONS_TOOL_ID,
            object([
                ("filter", object([("filter_type", string("commands"))])),
                ("min_queued_events", integer(1)),
                ("sort", string("queued_events_desc")),
            ]),
            1_101,
        );
        let list_subscriptions_trace = tool_runtime.invoke_with_events(&list_subscriptions_request);
        assert!(list_subscriptions_trace.result.ok);
        let list_subscriptions_output = list_subscriptions_trace.result.output.as_ref().unwrap();
        assert_eq!(field(list_subscriptions_output, "count"), Some(&integer(1)));
        assert_eq!(
            field(
                array_item(
                    field(list_subscriptions_output, "subscriptions").unwrap(),
                    0
                )
                .unwrap(),
                "subscription_id"
            ),
            Some(&string("commands"))
        );
        assert_eq!(
            field(
                field(list_subscriptions_output, "summary").unwrap(),
                "total_queued_events"
            ),
            Some(&integer(1))
        );
        assert_eq!(
            field(
                field(list_subscriptions_output, "summary").unwrap(),
                "backlogged_subscriptions"
            ),
            Some(&integer(1))
        );

        let inspect_event_log_request = request(
            "call-inspect-event-log",
            SMART_HOME_INSPECT_EVENT_LOG_TOOL_ID,
            object([
                ("filter", object([("filter_type", string("commands"))])),
                ("sort", string("sequence_desc")),
                ("limit", integer(1)),
            ]),
            1_102,
        );
        let inspect_event_log_trace = tool_runtime.invoke_with_events(&inspect_event_log_request);
        assert!(inspect_event_log_trace.result.ok);
        let inspect_event_log_output = inspect_event_log_trace.result.output.as_ref().unwrap();
        assert_eq!(field(inspect_event_log_output, "count"), Some(&integer(1)));
        assert_eq!(
            field(
                field(
                    array_item(field(inspect_event_log_output, "events").unwrap(), 0).unwrap(),
                    "event"
                )
                .unwrap(),
                "event_kind"
            ),
            Some(&string("command_result"))
        );
        assert_eq!(
            field(
                field(inspect_event_log_output, "summary").unwrap(),
                "command_results"
            ),
            Some(&integer(1))
        );

        let list_authorization_request = request(
            "call-list-authorization-decisions",
            SMART_HOME_LIST_AUTHORIZATION_DECISIONS_TOOL_ID,
            object([
                ("principal_id", string(AGENT_ID)),
                ("outcome", string("allowed")),
                ("sort", string("newest_first")),
                ("limit", integer(1)),
            ]),
            1_103,
        );
        let list_authorization_trace = tool_runtime.invoke_with_events(&list_authorization_request);
        assert!(list_authorization_trace.result.ok);
        let list_authorization_output = list_authorization_trace.result.output.as_ref().unwrap();
        assert_eq!(field(list_authorization_output, "count"), Some(&integer(1)));
        let latest_decision =
            array_item(field(list_authorization_output, "decisions").unwrap(), 0).unwrap();
        assert_eq!(
            field(latest_decision, "subject_kind"),
            Some(&string("tool"))
        );
        assert_eq!(
            field(field(latest_decision, "subject").unwrap(), "tool_id"),
            Some(&string(SMART_HOME_LIST_AUTHORIZATION_DECISIONS_TOOL_ID))
        );
        assert_eq!(
            field(
                field(list_authorization_output, "summary").unwrap(),
                "allowed_decisions"
            ),
            Some(&integer(1))
        );

        let authorization_summary_request = request(
            "call-authorization-summary",
            SMART_HOME_GET_AUTHORIZATION_SUMMARY_TOOL_ID,
            object([
                ("principal_id", string(AGENT_ID)),
                ("outcome", string("denied")),
            ]),
            1_104,
        );
        let authorization_summary_trace =
            tool_runtime.invoke_with_events(&authorization_summary_request);
        assert!(authorization_summary_trace.result.ok);
        let authorization_summary_output =
            authorization_summary_trace.result.output.as_ref().unwrap();
        let authorization_summary = field(authorization_summary_output, "summary").unwrap();
        assert_eq!(
            field(authorization_summary, "total_decisions"),
            Some(&integer(0))
        );
        assert_eq!(
            field(authorization_summary, "has_denials"),
            Some(&JsonValue::Bool(false))
        );

        let list_capability_grants_request = request(
            "call-list-capability-grants",
            SMART_HOME_LIST_CAPABILITY_GRANTS_TOOL_ID,
            object([
                ("principal_id", string(AGENT_ID)),
                ("status", string("active")),
                ("scope_kind", string("all_smart_home")),
                ("sort", string("newest_first")),
                ("limit", integer(1)),
            ]),
            1_105,
        );
        let list_capability_grants_trace =
            tool_runtime.invoke_with_events(&list_capability_grants_request);
        assert!(list_capability_grants_trace.result.ok);
        let list_capability_grants_output =
            list_capability_grants_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(list_capability_grants_output, "count"),
            Some(&integer(1))
        );
        let grant = array_item(field(list_capability_grants_output, "grants").unwrap(), 0).unwrap();
        assert_eq!(field(grant, "grant_id"), Some(&string("grant-smart-home")));
        assert_eq!(field(grant, "effective_status"), Some(&string("active")));
        assert_eq!(
            field(field(grant, "scope").unwrap(), "scope_kind"),
            Some(&string("all_smart_home"))
        );
        assert_eq!(
            field(
                field(list_capability_grants_output, "summary").unwrap(),
                "human_approval_tier_grants"
            ),
            Some(&integer(1))
        );

        let capability_grant_summary_request = request(
            "call-capability-grant-summary",
            SMART_HOME_GET_CAPABILITY_GRANT_SUMMARY_TOOL_ID,
            object([
                ("principal_id", string(AGENT_ID)),
                ("status", string("active")),
            ]),
            1_106,
        );
        let capability_grant_summary_trace =
            tool_runtime.invoke_with_events(&capability_grant_summary_request);
        assert!(capability_grant_summary_trace.result.ok);
        let capability_grant_summary_output = capability_grant_summary_trace
            .result
            .output
            .as_ref()
            .unwrap();
        let capability_grant_summary = field(capability_grant_summary_output, "summary").unwrap();
        assert_eq!(
            field(capability_grant_summary, "total_grants"),
            Some(&integer(1))
        );
        assert_eq!(
            field(capability_grant_summary, "has_active_grants"),
            Some(&JsonValue::Bool(true))
        );

        let poll_request = request(
            "call-poll-events",
            SMART_HOME_POLL_EVENTS_TOOL_ID,
            object([
                ("subscription_id", string("commands")),
                ("limit", integer(1)),
            ]),
            1_103,
        );
        let poll_trace = tool_runtime.invoke_with_events(&poll_request);
        assert!(poll_trace.result.ok);
        let poll_output = poll_trace.result.output.as_ref().unwrap();
        assert_eq!(field(poll_output, "delivered_events"), Some(&integer(1)));
        assert_eq!(field(poll_output, "remaining_events"), Some(&integer(0)));
        assert_eq!(array_len(field(poll_output, "events").unwrap()), Some(1));
        assert_eq!(
            field(
                array_item(field(poll_output, "events").unwrap(), 0).unwrap(),
                "event_kind"
            ),
            Some(&string("command_result"))
        );
        assert_eq!(
            field(field(poll_output, "summary").unwrap(), "command_results"),
            Some(&integer(1))
        );

        let state_request = request(
            "call-get-state",
            SMART_HOME_GET_STATE_TOOL_ID,
            object([("entity_id", string("entity-light-1"))]),
            1_104,
        );
        let state_trace = tool_runtime.invoke_with_events(&state_request);
        assert!(state_trace.result.ok);
        let state_output = state_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(state_output, "has_state"),
            Some(&JsonValue::Bool(true))
        );
        assert_eq!(
            field(field(state_output, "state").unwrap(), "source"),
            Some(&string("event_stream"))
        );
        assert_eq!(
            field(field(state_output, "state").unwrap(), "confidence"),
            Some(&string("confirmed"))
        );

        let unsubscribe_request = request(
            "call-unsubscribe",
            SMART_HOME_UNSUBSCRIBE_TOOL_ID,
            object([("subscription_id", string("commands"))]),
            1_105,
        );
        let unsubscribe_trace = tool_runtime.invoke_with_events(&unsubscribe_request);
        assert!(unsubscribe_trace.result.ok);
        let unsubscribe_output = unsubscribe_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(unsubscribe_output, "unsubscribed"),
            Some(&JsonValue::Bool(true))
        );
        assert_eq!(
            field(unsubscribe_output, "delivered_events"),
            Some(&integer(0))
        );

        let reconcile_request = request(
            "call-reconcile-desired-states",
            SMART_HOME_RECONCILE_DESIRED_STATES_TOOL_ID,
            object([]),
            1_900,
        );
        let reconcile_trace = tool_runtime.invoke_with_events(&reconcile_request);
        assert!(reconcile_trace.result.ok);
        assert_eq!(reconcile_trace.summary().progress_event_count, 1);
        let reconcile_output = reconcile_trace.result.output.as_ref().unwrap();
        assert_eq!(field(reconcile_output, "action_count"), Some(&integer(0)));
        assert_eq!(
            field(
                field(reconcile_output, "summary").unwrap(),
                "stale_state_count"
            ),
            Some(&integer(0))
        );
        assert_eq!(
            array_len(field(reconcile_output, "actions").unwrap()),
            Some(0)
        );

        let clear_desired_state_request = request(
            "call-clear-desired-state",
            SMART_HOME_CLEAR_DESIRED_STATE_TOOL_ID,
            object([("entity_id", string("entity-light-1"))]),
            1_905,
        );
        let clear_desired_state_trace =
            tool_runtime.invoke_with_events(&clear_desired_state_request);
        assert!(clear_desired_state_trace.result.ok);
        assert_eq!(clear_desired_state_trace.summary().progress_event_count, 1);
        let clear_desired_state_output = clear_desired_state_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(clear_desired_state_output, "entity_id"),
            Some(&string("entity-light-1"))
        );
        assert_eq!(
            field(clear_desired_state_output, "removed"),
            Some(&JsonValue::Bool(true))
        );
        assert_eq!(
            field(
                field(clear_desired_state_output, "desired_state").unwrap(),
                "requested_by"
            ),
            Some(&string("agent:scene-planner"))
        );

        let supervision_tick_request = request(
            "call-run-supervision-tick",
            SMART_HOME_RUN_SUPERVISION_TICK_TOOL_ID,
            object([]),
            2_100,
        );
        let supervision_tick_trace = tool_runtime.invoke_with_events(&supervision_tick_request);
        assert!(supervision_tick_trace.result.ok);
        let supervision_tick_output = supervision_tick_trace.result.output.as_ref().unwrap();
        assert_eq!(
            field(supervision_tick_output, "action_count"),
            Some(&integer(1))
        );
        assert_eq!(
            array_len(field(supervision_tick_output, "expired_pairing_sessions").unwrap()),
            Some(0)
        );
        assert_eq!(
            array_len(field(supervision_tick_output, "expired_entities").unwrap()),
            Some(0)
        );
        assert_eq!(
            array_len(field(supervision_tick_output, "desired_state_actions").unwrap()),
            Some(0)
        );
        assert_eq!(
            array_len(field(supervision_tick_output, "worker_events").unwrap()),
            Some(1)
        );
        assert_eq!(
            field(
                field(supervision_tick_output, "summary").unwrap(),
                "expired_pairing_session_count"
            ),
            Some(&integer(0))
        );
        assert_eq!(
            field(
                field(supervision_tick_output, "summary").unwrap(),
                "worker_restart_event_count"
            ),
            Some(&integer(1))
        );

        let mut journal = ToolExecutionJournal::new();
        journal.record_trace(list_integrations_request, list_integrations_trace);
        journal.record_trace(describe_integration_request, describe_integration_trace);
        journal.record_trace(list_primitives_request, list_primitives_trace);
        journal.record_trace(describe_primitive_request, describe_primitive_trace);
        journal.record_trace(list_request, list_trace);
        journal.record_trace(list_rooms_request, list_rooms_trace);
        journal.record_trace(list_scenes_request, list_scenes_trace);
        journal.record_trace(describe_scene_request, describe_scene_trace);
        journal.record_trace(discover_request, discover_trace);
        journal.record_trace(list_discovery_workers_request, list_discovery_workers_trace);
        journal.record_trace(discovery_summary_request, discovery_summary_trace);
        journal.record_trace(pairing_plan_request, pairing_plan_trace);
        journal.record_trace(capabilities_request, capabilities_trace);
        journal.record_trace(health_request, health_trace);
        journal.record_trace(list_workers_request, list_workers_trace);
        journal.record_trace(heartbeat_schedule_request, heartbeat_schedule_trace);
        journal.record_trace(supervision_request, supervision_trace);
        journal.record_trace(subscribe_request, subscribe_trace);
        journal.record_trace(pair_request, pair_trace);
        journal.record_trace(complete_pairing_request, complete_pairing_trace);
        journal.record_trace(set_desired_state_request, set_desired_state_trace);
        journal.record_trace(command_request, command_trace);
        journal.record_trace(report_event_request, report_event_trace);
        journal.record_trace(report_health_request, report_health_trace);
        journal.record_trace(runtime_snapshot_request, runtime_snapshot_trace);
        journal.record_trace(topology_summary_request, topology_summary_trace);
        journal.record_trace(supervision_plan_request, supervision_plan_trace);
        journal.record_trace(desired_states_request, desired_states_trace);
        journal.record_trace(pairing_sessions_request, pairing_sessions_trace);
        journal.record_trace(list_subscriptions_request, list_subscriptions_trace);
        journal.record_trace(inspect_event_log_request, inspect_event_log_trace);
        journal.record_trace(list_authorization_request, list_authorization_trace);
        journal.record_trace(authorization_summary_request, authorization_summary_trace);
        journal.record_trace(list_capability_grants_request, list_capability_grants_trace);
        journal.record_trace(
            capability_grant_summary_request,
            capability_grant_summary_trace,
        );
        journal.record_trace(poll_request, poll_trace);
        journal.record_trace(state_request, state_trace);
        journal.record_trace(unsubscribe_request, unsubscribe_trace);
        journal.record_trace(reconcile_request, reconcile_trace);
        journal.record_trace(clear_desired_state_request, clear_desired_state_trace);
        journal.record_trace(supervision_tick_request, supervision_tick_trace);

        let journal_summary = journal.summary();
        assert_eq!(journal_summary.invocation_count, 41);
        assert_eq!(journal_summary.completed_count, 41);
        assert_eq!(journal.audit_records().len(), 41);

        let runtime = runtime.borrow();
        assert_eq!(runtime.optimistic_state_count(), 0);
        assert_eq!(runtime.desired_state_count(), 0);
        assert_eq!(runtime.pairing_session_count(), 1);
        assert!(matches!(
            runtime
                .event_bus()
                .queued_events(&RuntimeSubscriptionId::trusted("commands")),
            Err(RuntimeError::UnknownSubscription(_))
        ));
        assert_eq!(
            runtime.registry().counts().authorization_decisions,
            38,
            "read, subscribe, poll, unsubscribe, pairing, ingest, and desired-state calls record tool authorization, while command records tool and command authorization"
        );
        assert_eq!(
            runtime
                .registry()
                .bridge(&BridgeId::trusted("bridge-1"))
                .unwrap()
                .auth_ref,
            Some(VaultRef::trusted("vault://smart-home/hue/bridge-1/app-key"))
        );
    }

    #[test]
    fn smart_home_handler_reports_runtime_authorization_denials() {
        let runtime = Rc::new(RefCell::new(hue_lighting_runtime()));
        let bridge = SmartHomeToolBridge::new(runtime.clone(), AgentId::trusted(AGENT_ID));
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

        let denied_report = tool_runtime.invoke(&request(
            "call-report-denied",
            SMART_HOME_REPORT_EVENT_TOOL_ID,
            object([
                ("event_kind", string("device")),
                ("event_id", string("denied-event-1")),
                ("bridge_id", string("bridge-1")),
                ("event_type", string("updated")),
            ]),
            1_000,
        ));

        assert!(!denied_report.ok);
        assert_eq!(
            denied_report.error.as_ref().map(|error| error.kind),
            Some(ToolErrorKind::ToolPermissionDenied)
        );

        let denied_set_desired_state = tool_runtime.invoke(&request(
            "call-set-desired-state-denied",
            SMART_HOME_SET_DESIRED_STATE_TOOL_ID,
            object([
                ("entity_id", string("entity-light-1")),
                (
                    "desired",
                    object([
                        ("capability_id", string("light.on_off")),
                        ("value", JsonValue::Bool(true)),
                    ]),
                ),
            ]),
            1_000,
        ));

        assert!(!denied_set_desired_state.ok);
        assert_eq!(
            denied_set_desired_state
                .error
                .as_ref()
                .map(|error| error.kind),
            Some(ToolErrorKind::ToolPermissionDenied)
        );

        let denied_clear_desired_state = tool_runtime.invoke(&request(
            "call-clear-desired-state-denied",
            SMART_HOME_CLEAR_DESIRED_STATE_TOOL_ID,
            object([("entity_id", string("entity-light-1"))]),
            1_000,
        ));

        assert!(!denied_clear_desired_state.ok);
        assert_eq!(
            denied_clear_desired_state
                .error
                .as_ref()
                .map(|error| error.kind),
            Some(ToolErrorKind::ToolPermissionDenied)
        );
        assert_eq!(runtime.borrow().registry().counts().events, 0);
        assert_eq!(runtime.borrow().desired_state_count(), 0);
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

        let unknown_primitive = tool_runtime.invoke_with_events(&request(
            "call-invalid-primitive",
            SMART_HOME_DESCRIBE_PRIMITIVE_TOOL_ID,
            object([("primitive", string("time_travel"))]),
            1_000,
        ));
        assert!(!unknown_primitive.result.ok);
        assert_eq!(
            unknown_primitive
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

    fn integer_value(value: &JsonValue) -> Option<i64> {
        let JsonValue::Number(JsonNumber::Integer(value)) = value else {
            return None;
        };
        Some(*value)
    }
}
