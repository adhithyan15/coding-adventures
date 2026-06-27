//! Home Assistant-compatible local HTTP API routes for the smart-home platform.
//!
//! The crate builds `web-core::WebApp` routes over runtime-owned smart-home
//! registry snapshots. It deliberately uses the repo's own HTTP server stack;
//! service calls are wired through runtime command authorization instead of a
//! parallel mutation path.

#![forbid(unsafe_code)]

use serde_json::Value as JsonValue;
use smart_home_core::{
    AgentId, AuthorizationDecision, AuthorizationOutcome, AuthorizationSubject, Bridge,
    BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId, CapabilityId, CapabilityMode,
    CommandResult, CommandStatus, CommandType, Device, DeviceEvent, DeviceEventType, Entity,
    EntityId, EntityKind, Health, PrivilegeTier, Scene, StateConfidence, StateDelta, StateSource,
    Value, ValueKind,
};
use smart_home_runtime::{
    DesiredEntityState, DesiredStateQuery, RuntimeAuthorizationDecisionQuery,
    RuntimeClearDesiredStateToolOutput, RuntimeClearDesiredStateToolRequest,
    RuntimeCommandResultQuery, RuntimeCommandResultRecord, RuntimeCommandResultSort,
    RuntimeCommandToolRequest, RuntimeError, RuntimeEvent, RuntimeEventCheckpoint,
    RuntimeEventFilter, RuntimeEventLogEntry, RuntimeEventQuery, RuntimeEventSort,
    RuntimeReadSnapshot, RuntimeRoomQuery, RuntimeRoomSort, RuntimeRoomSummary,
    RuntimeSetDesiredStateToolOutput, RuntimeSetDesiredStateToolRequest, SmartHomeRuntime,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use web_core::{WebApp, WebRequest, WebResponse};

pub const VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartHomePlatformHttpConfig {
    pub location_name: String,
    pub unit_system: String,
    pub time_zone: String,
    pub version: String,
}

impl SmartHomePlatformHttpConfig {
    pub fn new(location_name: impl Into<String>) -> Self {
        Self {
            location_name: location_name.into(),
            unit_system: "metric".to_string(),
            time_zone: "UTC".to_string(),
            version: VERSION.to_string(),
        }
    }

    pub fn with_unit_system(mut self, unit_system: impl Into<String>) -> Self {
        self.unit_system = unit_system.into();
        self
    }

    pub fn with_time_zone(mut self, time_zone: impl Into<String>) -> Self {
        self.time_zone = time_zone.into();
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmartHomePlatformHttpState {
    pub config: SmartHomePlatformHttpConfig,
    pub entities: Vec<Entity>,
    pub scenes: Vec<Scene>,
    pub event_types: Vec<String>,
    pub generated_at_ms: u64,
}

impl SmartHomePlatformHttpState {
    pub fn from_runtime(
        runtime: &SmartHomeRuntime,
        config: SmartHomePlatformHttpConfig,
        event_types: impl IntoIterator<Item = impl Into<String>>,
        generated_at_ms: u64,
    ) -> Self {
        let mut event_types = event_types.into_iter().map(Into::into).collect::<Vec<_>>();
        event_types.sort();
        event_types.dedup();

        Self {
            config,
            entities: runtime.registry().entities().cloned().collect(),
            scenes: runtime.registry().scenes().cloned().collect(),
            event_types,
            generated_at_ms,
        }
    }

    pub fn summary(&self) -> SmartHomePlatformHttpSummary {
        SmartHomePlatformHttpSummary::from_state(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SmartHomePlatformHttpSummary {
    pub state_count: usize,
    pub known_state_count: usize,
    pub unknown_state_count: usize,
    pub stale_state_count: usize,
    pub optimistic_state_count: usize,
    pub service_count: usize,
    pub event_type_count: usize,
    pub scene_count: usize,
}

impl SmartHomePlatformHttpSummary {
    pub fn from_state(state: &SmartHomePlatformHttpState) -> Self {
        let mut summary = Self {
            state_count: state.entities.len(),
            event_type_count: state.event_types.len(),
            scene_count: state.scenes.len(),
            service_count: platform_services(state).len(),
            ..Self::default()
        };

        for entity in &state.entities {
            match &entity.state {
                Some(snapshot) if snapshot.confidence == StateConfidence::Stale => {
                    summary.stale_state_count += 1;
                }
                Some(snapshot) if snapshot.confidence == StateConfidence::Optimistic => {
                    summary.optimistic_state_count += 1;
                    summary.known_state_count += 1;
                }
                Some(_) => summary.known_state_count += 1,
                None => summary.unknown_state_count += 1,
            }
        }

        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartHomePlatformService {
    pub domain: String,
    pub service: String,
    pub description: String,
    pub target_entity_ids: Vec<String>,
    pub target_scene_ids: Vec<String>,
    pub capability_ids: Vec<String>,
}

#[derive(Clone)]
pub struct SmartHomePlatformHttpRuntime {
    runtime: Arc<Mutex<SmartHomeRuntime>>,
    config: SmartHomePlatformHttpConfig,
    event_types: Vec<String>,
    principal_id: AgentId,
    now_ms: u64,
}

impl SmartHomePlatformHttpRuntime {
    pub fn new(runtime: SmartHomeRuntime, config: SmartHomePlatformHttpConfig) -> Self {
        Self::from_shared_runtime(Arc::new(Mutex::new(runtime)), config)
    }

    pub fn from_shared_runtime(
        runtime: Arc<Mutex<SmartHomeRuntime>>,
        config: SmartHomePlatformHttpConfig,
    ) -> Self {
        Self {
            runtime,
            config,
            event_types: default_event_types(),
            principal_id: AgentId::trusted("agent:home-assistant-local-api"),
            now_ms: 0,
        }
    }

    pub fn with_event_types(
        mut self,
        event_types: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.event_types = sorted_unique_strings(event_types);
        self
    }

    pub fn with_principal_id(mut self, principal_id: AgentId) -> Self {
        self.principal_id = principal_id;
        self
    }

    pub fn with_now_ms(mut self, now_ms: u64) -> Self {
        self.now_ms = now_ms;
        self
    }

    pub fn grant_local_full_access(
        self,
        granted_by: impl Into<String>,
        granted_at_ms: u64,
    ) -> Self {
        let grant = CapabilityGrant::for_all_smart_home(
            CapabilityGrantId::trusted(format!(
                "grant:{}:local-api-full-access",
                self.principal_id.as_str()
            )),
            self.principal_id.clone(),
            PrivilegeTier::HighRisk,
            granted_by,
            granted_at_ms,
        );
        self.runtime
            .lock()
            .expect("smart-home runtime mutex should not be poisoned")
            .registry_mut()
            .upsert_capability_grant(grant);
        self
    }

    pub fn snapshot(&self) -> SmartHomePlatformHttpState {
        let runtime = self
            .runtime
            .lock()
            .expect("smart-home runtime mutex should not be poisoned");
        SmartHomePlatformHttpState::from_runtime(
            &runtime,
            self.config.clone(),
            self.event_types.clone(),
            self.now_ms,
        )
    }
}

pub fn home_assistant_web_app(state: SmartHomePlatformHttpState) -> WebApp {
    let state = Arc::new(state);
    let mut app = WebApp::new();

    app.get("/api/", move |_| {
        WebResponse::json(api_root_json().into_bytes())
    });

    {
        let state = Arc::clone(&state);
        app.get("/api/config", move |_| {
            WebResponse::json(config_json(&state).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/states", move |_| {
            WebResponse::json(states_json(&state.entities, state.generated_at_ms).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/states/:entity_id", move |request| {
            let Some(entity_id) = request.route_params.get("entity_id") else {
                return WebResponse::new(400, br#"{"error":"missing entity_id"}"#.to_vec())
                    .with_content_type("application/json");
            };
            match state
                .entities
                .iter()
                .find(|entity| entity.entity_id.as_str() == entity_id)
            {
                Some(entity) => {
                    WebResponse::json(state_json(entity, state.generated_at_ms).into_bytes())
                }
                None => WebResponse::new(404, br#"{"error":"entity not found"}"#.to_vec())
                    .with_content_type("application/json"),
            }
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/services", move |_| {
            WebResponse::json(services_json(&platform_services(&state)).into_bytes())
        });
    }

    {
        let state = Arc::clone(&state);
        app.get("/api/events", move |_| {
            WebResponse::json(events_json(&state.event_types).into_bytes())
        });
    }

    app
}

pub fn home_assistant_runtime_web_app(runtime: SmartHomePlatformHttpRuntime) -> WebApp {
    let mut app = WebApp::new();

    app.get("/api/", move |_| {
        WebResponse::json(api_root_json().into_bytes())
    });

    {
        let runtime = runtime.clone();
        app.get("/api/config", move |_| {
            let state = runtime.snapshot();
            WebResponse::json(config_json(&state).into_bytes())
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/states", move |_| {
            let state = runtime.snapshot();
            WebResponse::json(states_json(&state.entities, state.generated_at_ms).into_bytes())
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/states/:entity_id", move |request| {
            let Some(entity_id) = request.route_params.get("entity_id") else {
                return json_error(400, "missing entity_id");
            };
            let state = runtime.snapshot();
            match state
                .entities
                .iter()
                .find(|entity| entity_matches_external_id(entity, entity_id))
            {
                Some(entity) => {
                    WebResponse::json(state_json(entity, state.generated_at_ms).into_bytes())
                }
                None => json_error(404, "entity not found"),
            }
        });
    }

    {
        let runtime = runtime.clone();
        app.post("/api/states/:entity_id", move |request| {
            set_desired_state_response(&runtime, request, true)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/services", move |_| {
            let state = runtime.snapshot();
            WebResponse::json(services_json(&platform_services(&state)).into_bytes())
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/events", move |_| {
            let state = runtime.snapshot();
            WebResponse::json(events_json(&state.event_types).into_bytes())
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/history/period", move |request| {
            home_assistant_history_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/history/period/:start_time", move |request| {
            home_assistant_history_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/runtime", move |_| {
            runtime_snapshot_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/dashboard", move |_| {
            runtime_dashboard_response(&runtime)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/entities", move |request| {
            runtime_entities_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/entities/:entity_id", move |request| {
            runtime_entity_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/devices", move |request| {
            runtime_devices_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/devices/:device_id", move |request| {
            runtime_device_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/bridges", move |request| {
            runtime_bridges_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/bridges/:bridge_id", move |request| {
            runtime_bridge_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/rooms", move |request| {
            runtime_rooms_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/events", move |request| {
            runtime_events_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/command_results", move |request| {
            runtime_command_results_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/authorization_decisions", move |request| {
            runtime_authorization_decisions_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/desired_states", move |request| {
            runtime_desired_states_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.post(
            "/api/smart_home/desired_states/:entity_id",
            move |request| set_desired_state_response(&runtime, request, false),
        );
    }

    {
        let runtime = runtime.clone();
        app.delete(
            "/api/smart_home/desired_states/:entity_id",
            move |request| clear_desired_state_response(&runtime, request),
        );
    }

    {
        let runtime = runtime.clone();
        app.get("/api/smart_home/state_history", move |request| {
            runtime_state_history_response(&runtime, request)
        });
    }

    {
        let runtime = runtime.clone();
        app.post("/api/services/:domain/:service", move |request| {
            service_call_response(&runtime, request)
        });
    }

    app
}

pub fn platform_services(state: &SmartHomePlatformHttpState) -> Vec<SmartHomePlatformService> {
    let mut services = BTreeMap::<(String, String), SmartHomePlatformService>::new();

    for entity in &state.entities {
        let domain = entity_domain(entity.kind).to_string();
        for capability in entity
            .capabilities
            .iter()
            .filter(|capability| capability_allows_command(capability))
        {
            for service in services_for_capability(&domain, capability) {
                let key = (domain.clone(), service.to_string());
                let entry = services
                    .entry(key)
                    .or_insert_with(|| SmartHomePlatformService {
                        domain: domain.clone(),
                        service: service.to_string(),
                        description: format!("{} {}", service.replace('_', " "), domain),
                        target_entity_ids: Vec::new(),
                        target_scene_ids: Vec::new(),
                        capability_ids: Vec::new(),
                    });
                push_unique_string(&mut entry.target_entity_ids, entity.entity_id.as_str());
                push_unique_string(&mut entry.capability_ids, capability.capability_id.as_str());
            }
        }
    }

    if !state.scenes.is_empty() {
        let entry = services
            .entry(("scene".to_string(), "turn_on".to_string()))
            .or_insert_with(|| SmartHomePlatformService {
                domain: "scene".to_string(),
                service: "turn_on".to_string(),
                description: "activate scene".to_string(),
                target_entity_ids: Vec::new(),
                target_scene_ids: Vec::new(),
                capability_ids: vec!["scene.recall".to_string()],
            });
        for scene in &state.scenes {
            push_unique_string(&mut entry.target_scene_ids, scene.scene_id.as_str());
        }
    }

    services.into_values().collect()
}

fn config_json(state: &SmartHomePlatformHttpState) -> String {
    let summary = state.summary();
    format!(
        "{{\"location_name\":{},\"unit_system\":{},\"time_zone\":{},\"version\":{},\"components\":[\"smart_home\"],\"state_count\":{},\"service_count\":{},\"event_type_count\":{},\"generated_at_ms\":{}}}",
        json_string(&state.config.location_name),
        json_string(&state.config.unit_system),
        json_string(&state.config.time_zone),
        json_string(&state.config.version),
        summary.state_count,
        summary.service_count,
        summary.event_type_count,
        state.generated_at_ms,
    )
}

fn api_root_json() -> String {
    "{\"message\":\"API running.\"}".to_string()
}

fn states_json(entities: &[Entity], now_ms: u64) -> String {
    format!(
        "[{}]",
        entities
            .iter()
            .map(|entity| state_json(entity, now_ms))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn state_json(entity: &Entity, now_ms: u64) -> String {
    let (state_value, last_changed_ms, last_updated_ms, source, confidence, stale) =
        match &entity.state {
            Some(snapshot) => (
                value_json(&snapshot.value),
                snapshot.observed_at_ms,
                snapshot.received_at_ms,
                state_source_label(snapshot.source),
                state_confidence_label(snapshot.confidence),
                snapshot.is_stale_at(now_ms),
            ),
            None => (json_string("unknown"), 0, 0, "unknown", "unknown", true),
        };

    let capability_ids = entity
        .capabilities
        .iter()
        .map(|capability| json_string(capability.capability_id.as_str()))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\"entity_id\":{},\"state\":{},\"attributes\":{{\"friendly_name\":{},\"device_id\":{},\"domain\":{},\"entity_kind\":{},\"home_assistant_entity_id\":{},\"capability_count\":{},\"capabilities\":[{}],\"stale\":{}}},\"last_changed_ms\":{},\"last_updated_ms\":{},\"context\":{{\"source\":{},\"confidence\":{}}}}}",
        json_string(entity.entity_id.as_str()),
        state_value,
        json_string(&entity.name),
        json_string(entity.device_id.as_str()),
        json_string(entity_domain(entity.kind)),
        json_string(entity_kind_label(entity.kind)),
        json_string(home_assistant_entity_id(entity)),
        entity.capabilities.len(),
        capability_ids,
        stale,
        last_changed_ms,
        last_updated_ms,
        json_string(source),
        json_string(confidence),
    )
}

fn services_json(services: &[SmartHomePlatformService]) -> String {
    let mut domains = BTreeMap::<&str, Vec<&SmartHomePlatformService>>::new();
    for service in services {
        domains.entry(&service.domain).or_default().push(service);
    }

    format!(
        "[{}]",
        domains
            .into_iter()
            .map(|(domain, services)| {
                format!(
                    "{{\"domain\":{},\"services\":[{}]}}",
                    json_string(domain),
                    services
                        .into_iter()
                        .map(service_json)
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn service_json(service: &SmartHomePlatformService) -> String {
    format!(
        "{{\"service\":{},\"description\":{},\"target_entity_ids\":[{}],\"target_scene_ids\":[{}],\"capability_ids\":[{}]}}",
        json_string(&service.service),
        json_string(&service.description),
        json_string_array(&service.target_entity_ids),
        json_string_array(&service.target_scene_ids),
        json_string_array(&service.capability_ids),
    )
}

fn events_json(event_types: &[String]) -> String {
    format!(
        "[{}]",
        event_types
            .iter()
            .map(|event_type| {
                format!(
                    "{{\"event\":{},\"description\":{}}}",
                    json_string(event_type),
                    json_string(format!("{event_type} platform event")),
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn runtime_snapshot_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(
        runtime_snapshot_json(&runtime_guard.read_snapshot_at(runtime.now_ms)).into_bytes(),
    )
}

fn runtime_dashboard_response(runtime: &SmartHomePlatformHttpRuntime) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    WebResponse::json(runtime_dashboard_json(runtime, &runtime_guard).into_bytes())
}

fn runtime_entities_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entities = match runtime_entities(&runtime_guard, request) {
        Ok(entities) => entities,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(
        entities_registry_json(&entities, &runtime_guard, runtime.now_ms).into_bytes(),
    )
}

fn runtime_entity_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("entity_id") else {
        return json_error(400, "missing entity_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity = match runtime_guard
        .registry()
        .entities()
        .find(|entity| entity_matches_external_id(entity, target))
    {
        Some(entity) => entity,
        None => {
            return api_error_response(ApiError::not_found(format!("entity `{target}` not found")));
        }
    };
    WebResponse::json(entity_registry_json(entity, &runtime_guard, runtime.now_ms).into_bytes())
}

fn runtime_devices_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let devices = match runtime_devices(&runtime_guard, request) {
        Ok(devices) => devices,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(devices_registry_json(&devices, &runtime_guard, runtime.now_ms).into_bytes())
}

fn runtime_device_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("device_id") else {
        return json_error(400, "missing device_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let device = match runtime_guard
        .registry()
        .devices()
        .find(|device| device.device_id.as_str() == target)
    {
        Some(device) => device,
        None => {
            return api_error_response(ApiError::not_found(format!("device `{target}` not found")));
        }
    };
    WebResponse::json(device_registry_json(device, &runtime_guard, runtime.now_ms).into_bytes())
}

fn runtime_bridges_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let bridges = match runtime_bridges(&runtime_guard, request) {
        Ok(bridges) => bridges,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(bridges_registry_json(&bridges, &runtime_guard, runtime.now_ms).into_bytes())
}

fn runtime_bridge_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("bridge_id") else {
        return json_error(400, "missing bridge_id");
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let bridge = match runtime_guard
        .registry()
        .bridges()
        .find(|bridge| bridge.bridge_id.as_str() == target)
    {
        Some(bridge) => bridge,
        None => {
            return api_error_response(ApiError::not_found(format!("bridge `{target}` not found")));
        }
    };
    WebResponse::json(bridge_registry_json(bridge, &runtime_guard, runtime.now_ms).into_bytes())
}

fn runtime_rooms_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_room_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let rooms = runtime_guard.query_room_summaries_at(&query, runtime.now_ms);
    WebResponse::json(rooms_json(&rooms, &runtime_guard).into_bytes())
}

fn runtime_events_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_event_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entries = runtime_guard.event_bus().query_events(&query);
    let summary = runtime_guard.event_bus().event_log_summary(&query);
    WebResponse::json(runtime_event_log_json(&entries, &summary).into_bytes())
}

fn runtime_command_results_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_command_result_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let records = runtime_guard.query_command_results(&query);
    let summary = runtime_guard.command_result_summary(&query);
    WebResponse::json(command_results_audit_json(&records, &summary).into_bytes())
}

fn runtime_authorization_decisions_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let query = match runtime_authorization_decision_query(request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let decisions = runtime_guard.query_authorization_decisions(&query);
    let summary = runtime_guard.authorization_decision_summary(&query);
    WebResponse::json(authorization_decisions_json(&decisions, &summary).into_bytes())
}

fn runtime_desired_states_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let query = match desired_state_query(&runtime_guard, request) {
        Ok(query) => query,
        Err(error) => return api_error_response(error),
    };
    let desired_states = runtime_guard.query_desired_states(&query);
    WebResponse::json(desired_states_json(&desired_states, &runtime_guard).into_bytes())
}

fn runtime_state_history_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let events = match state_history_events(&runtime_guard, request) {
        Ok(events) => events,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(state_history_json(&events, &runtime_guard).into_bytes())
}

fn home_assistant_history_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let events = match state_history_events(&runtime_guard, request) {
        Ok(events) => events,
        Err(error) => return api_error_response(error),
    };
    WebResponse::json(home_assistant_history_json(&events, &runtime_guard).into_bytes())
}

fn runtime_snapshot_json(snapshot: &RuntimeReadSnapshot) -> String {
    let pending = snapshot.pending_work_summary();
    format!(
        "{{\"generated_at_ms\":{},\"registry\":{{\"bridges\":{},\"devices\":{},\"entities\":{},\"scenes\":{},\"states\":{},\"events\":{},\"protocol_identifiers\":{},\"capability_grants\":{},\"authorization_decisions\":{}}},\"event_bus\":{{\"subscription_count\":{},\"pending_delivery_count\":{},\"published_event_count\":{},\"backlogged_subscription_count\":{},\"max_pending_delivery_count\":{}}},\"discovery\":{{\"record_count\":{},\"worker_count\":{},\"due_worker_count\":{},\"unhealthy_worker_count\":{},\"workers_with_failures\":{}}},\"supervisor\":{{\"worker_count\":{},\"restart_due_count\":{},\"unhealthy_count\":{},\"running_count\":{}}},\"desired_state\":{{\"target_count\":{},\"capability_count\":{}}},\"pairing\":{{\"session_count\":{},\"expiring_session_count\":{}}},\"optimistic_state\":{{\"target_count\":{},\"stale_target_count\":{}}},\"pending_work\":{{\"total\":{},\"event_backlog_count\":{},\"backlogged_subscription_count\":{},\"discovery_worker_due_count\":{},\"unhealthy_discovery_worker_count\":{},\"restart_due_count\":{},\"unhealthy_worker_count\":{},\"expiring_pairing_session_count\":{},\"stale_optimistic_state_count\":{},\"state_refresh_target_count\":{}}}}}",
        snapshot.generated_at_ms,
        snapshot.registry_counts.bridges,
        snapshot.registry_counts.devices,
        snapshot.registry_counts.entities,
        snapshot.registry_counts.scenes,
        snapshot.registry_counts.states,
        snapshot.registry_counts.events,
        snapshot.registry_counts.protocol_identifiers,
        snapshot.registry_counts.capability_grants,
        snapshot.registry_counts.authorization_decisions,
        snapshot.event_bus.subscription_count,
        snapshot.event_bus.pending_delivery_count,
        snapshot.event_bus.published_event_count,
        snapshot.event_bus.backlogged_subscription_count,
        snapshot.event_bus.max_pending_delivery_count,
        snapshot.discovery_record_count,
        snapshot.discovery_scheduler.worker_count,
        snapshot.discovery_scheduler.due_worker_count,
        snapshot.discovery_scheduler.unhealthy_count,
        snapshot.discovery_scheduler.workers_with_failures,
        snapshot.supervisor.worker_count,
        snapshot.supervisor.restart_due_count,
        snapshot.supervisor.unhealthy_count,
        snapshot.supervisor.running_count,
        snapshot.desired_state_count,
        snapshot.desired_capability_count,
        snapshot.pairing_session_count,
        snapshot.expiring_pairing_session_count,
        snapshot.optimistic_state_count,
        snapshot.stale_optimistic_state_count,
        pending.total_pending_work_count(),
        pending.event_backlog_count,
        pending.backlogged_subscription_count,
        pending.discovery_worker_due_count,
        pending.unhealthy_discovery_worker_count,
        pending.restart_due_count,
        pending.unhealthy_worker_count,
        pending.expiring_pairing_session_count,
        pending.stale_optimistic_state_count,
        pending.state_refresh_target_count,
    )
}

fn runtime_dashboard_json(
    runtime: &SmartHomePlatformHttpRuntime,
    runtime_guard: &SmartHomeRuntime,
) -> String {
    let state = SmartHomePlatformHttpState::from_runtime(
        runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms,
    );
    let state_summary = state.summary();
    let snapshot = runtime_guard.read_snapshot_at(runtime.now_ms);
    let topology = runtime_guard.topology_summary();
    let pending = snapshot.pending_work_summary();
    let rooms = runtime_guard.query_room_summaries_at(
        &RuntimeRoomQuery::new()
            .sorted_by(RuntimeRoomSort::AttentionDesc)
            .with_limit(50),
        runtime.now_ms,
    );
    let mut bridges = runtime_guard.registry().bridges().collect::<Vec<_>>();
    let mut devices = runtime_guard.registry().devices().collect::<Vec<_>>();
    let mut entities = runtime_guard.registry().entities().collect::<Vec<_>>();
    let desired_query = DesiredStateQuery::new().with_limit(50);
    let desired_states = runtime_guard.query_desired_states(&desired_query);
    let event_query = RuntimeEventQuery::new();
    let event_summary = runtime_guard.event_bus().event_log_summary(&event_query);
    let command_query = RuntimeCommandResultQuery::new()
        .sorted_by(RuntimeCommandResultSort::SequenceDesc)
        .with_limit(50);
    let command_summary = runtime_guard.command_result_summary(&command_query);
    let authorization_query = RuntimeAuthorizationDecisionQuery::new().with_limit(50);
    let authorization_summary = runtime_guard.authorization_decision_summary(&authorization_query);

    bridges.sort_by(|left, right| left.bridge_id.as_str().cmp(right.bridge_id.as_str()));
    devices.sort_by(|left, right| left.device_id.as_str().cmp(right.device_id.as_str()));
    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));

    format!(
        "{{\"generated_at_ms\":{},\"config\":{},\"summary\":{{\"state_count\":{},\"known_state_count\":{},\"unknown_state_count\":{},\"stale_state_count\":{},\"optimistic_state_count\":{},\"service_count\":{},\"event_type_count\":{},\"bridge_count\":{},\"device_count\":{},\"entity_count\":{},\"room_count\":{},\"scene_count\":{},\"desired_state_count\":{},\"pending_work_total\":{},\"has_attention\":{},\"has_state_gaps\":{},\"has_pairing_candidates\":{}}},\"runtime\":{},\"topology\":{{\"bridges\":{},\"devices\":{},\"entities\":{},\"scenes\":{},\"online_bridges\":{},\"attention_bridges\":{},\"online_devices\":{},\"attention_devices\":{},\"devices_with_room\":{},\"devices_without_room\":{},\"unique_rooms\":{},\"entities_with_state\":{},\"entities_without_state\":{},\"total_capabilities\":{},\"scene_actions\":{}}},\"bridges\":{},\"devices\":{},\"entities\":{},\"rooms\":{},\"desired_states\":{},\"events\":{{\"summary\":{}}},\"command_results\":{{\"summary\":{}}},\"authorization_decisions\":{{\"summary\":{}}}}}",
        runtime.now_ms,
        config_json(&state),
        state_summary.state_count,
        state_summary.known_state_count,
        state_summary.unknown_state_count,
        state_summary.stale_state_count,
        state_summary.optimistic_state_count,
        state_summary.service_count,
        state_summary.event_type_count,
        topology.bridges,
        topology.devices,
        topology.entities,
        topology.unique_rooms,
        topology.scenes,
        snapshot.desired_state_count,
        pending.total_pending_work_count(),
        topology.has_attention_items(),
        pending.state_refresh_target_count > 0,
        topology.has_pairing_candidates(),
        runtime_snapshot_json(&snapshot),
        topology.bridges,
        topology.devices,
        topology.entities,
        topology.scenes,
        topology.online_bridges,
        topology.attention_bridges,
        topology.online_devices,
        topology.attention_devices,
        topology.devices_with_room,
        topology.devices_without_room,
        topology.unique_rooms,
        topology.entities_with_state,
        topology.entities_without_state,
        topology.total_capabilities,
        topology.scene_actions,
        bridges_registry_json(&bridges, runtime_guard, runtime.now_ms),
        devices_registry_json(&devices, runtime_guard, runtime.now_ms),
        entities_registry_json(&entities, runtime_guard, runtime.now_ms),
        rooms_json(&rooms, runtime_guard),
        desired_states_json(&desired_states, runtime_guard),
        runtime_event_summary_json(&event_summary),
        command_result_summary_json(&command_summary),
        authorization_decision_summary_json(&authorization_summary),
    )
}

fn runtime_event_log_json(
    entries: &[RuntimeEventLogEntry<'_>],
    summary: &smart_home_runtime::RuntimeEventLogSummary,
) -> String {
    format!(
        "{{\"summary\":{},\"events\":[{}]}}",
        runtime_event_summary_json(summary),
        entries
            .iter()
            .map(|entry| runtime_event_entry_json(entry))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn runtime_event_summary_json(summary: &smart_home_runtime::RuntimeEventLogSummary) -> String {
    format!(
        "{{\"total_events\":{},\"device_events\":{},\"command_results\":{},\"bridge_health_events\":{},\"state_expired_events\":{},\"desired_state_drift_events\":{},\"worker_restart_events\":{},\"first_sequence\":{},\"latest_sequence\":{},\"next_sequence\":{}}}",
        summary.total_events,
        summary.device_events,
        summary.command_results,
        summary.bridge_health_events,
        summary.state_expired_events,
        summary.desired_state_drift_events,
        summary.worker_restart_events,
        optional_u64_json(summary.first_sequence),
        optional_u64_json(summary.latest_sequence),
        summary.next_checkpoint.next_sequence(),
    )
}

fn runtime_event_entry_json(entry: &RuntimeEventLogEntry<'_>) -> String {
    format!(
        "{{\"sequence\":{},\"next_sequence\":{},\"event\":{}}}",
        entry.sequence,
        entry.next_checkpoint.next_sequence(),
        runtime_event_json(entry.event),
    )
}

fn runtime_event_json(event: &RuntimeEvent) -> String {
    match event {
        RuntimeEvent::Device(event) => device_event_json(event),
        RuntimeEvent::CommandResult(result) => format!(
            "{{\"kind\":\"command_result\",\"result\":{}}}",
            command_result_json(result)
        ),
        RuntimeEvent::BridgeHealth {
            event_id,
            bridge_id,
            health,
            observed_at_ms,
            received_at_ms,
        } => format!(
            "{{\"kind\":\"bridge_health\",\"event_id\":{},\"bridge_id\":{},\"health\":{},\"observed_at_ms\":{},\"received_at_ms\":{}}}",
            json_string(event_id.as_str()),
            json_string(bridge_id.as_str()),
            json_string(format!("{health:?}").to_ascii_lowercase()),
            observed_at_ms,
            received_at_ms,
        ),
        RuntimeEvent::StateExpired {
            entity_id,
            expired_at_ms,
        } => format!(
            "{{\"kind\":\"state_expired\",\"entity_id\":{},\"expired_at_ms\":{}}}",
            json_string(entity_id.as_str()),
            expired_at_ms,
        ),
        RuntimeEvent::DesiredStateDrift {
            bridge_id,
            entity_id,
            capability_id,
            reason,
            detected_at_ms,
        } => format!(
            "{{\"kind\":\"desired_state_drift\",\"bridge_id\":{},\"entity_id\":{},\"capability_id\":{},\"reason\":{},\"detected_at_ms\":{}}}",
            json_string(bridge_id.as_str()),
            json_string(entity_id.as_str()),
            json_string(capability_id.as_str()),
            json_string(format!("{reason:?}").to_ascii_lowercase()),
            detected_at_ms,
        ),
        RuntimeEvent::WorkerNeedsRestart {
            bridge_id,
            integration_id,
            overdue_at_ms,
        } => format!(
            "{{\"kind\":\"worker_needs_restart\",\"bridge_id\":{},\"integration_id\":{},\"overdue_at_ms\":{}}}",
            json_string(bridge_id.as_str()),
            json_string(integration_id.as_str()),
            overdue_at_ms,
        ),
    }
}

fn device_event_json(event: &DeviceEvent) -> String {
    format!(
        "{{\"kind\":\"device_event\",\"event_id\":{},\"bridge_id\":{},\"device_id\":{},\"entity_id\":{},\"event_type\":{},\"observed_at_ms\":{},\"received_at_ms\":{},\"state_delta\":{},\"raw_ref\":{},\"correlation_id\":{}}}",
        json_string(event.event_id.as_str()),
        json_string(event.bridge_id.as_str()),
        event
            .device_id
            .as_ref()
            .map(|device_id| json_string(device_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
        event
            .entity_id
            .as_ref()
            .map(|entity_id| json_string(entity_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
        json_string(device_event_type_label(event.event_type)),
        event.observed_at_ms,
        event.received_at_ms,
        event
            .state_delta
            .as_ref()
            .map(state_delta_json)
            .unwrap_or_else(|| "null".to_string()),
        event
            .raw_ref
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        event
            .correlation_id
            .as_ref()
            .map(|correlation_id| json_string(correlation_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
    )
}

fn command_results_audit_json(
    records: &[RuntimeCommandResultRecord],
    summary: &smart_home_runtime::RuntimeCommandResultSummary,
) -> String {
    format!(
        "{{\"summary\":{},\"results\":[{}]}}",
        command_result_summary_json(summary),
        records
            .iter()
            .map(|record| {
                format!(
                    "{{\"sequence\":{},\"next_sequence\":{},\"result\":{}}}",
                    record.sequence,
                    record.next_checkpoint.next_sequence(),
                    command_result_json(&record.result),
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn command_result_summary_json(
    summary: &smart_home_runtime::RuntimeCommandResultSummary,
) -> String {
    format!(
        "{{\"total_results\":{},\"accepted_results\":{},\"rejected_results\":{},\"timed_out_results\":{},\"failed_results\":{},\"first_sequence\":{},\"latest_sequence\":{},\"next_sequence\":{}}}",
        summary.total_results,
        summary.accepted_results,
        summary.rejected_results,
        summary.timed_out_results,
        summary.failed_results,
        optional_u64_json(summary.first_sequence),
        optional_u64_json(summary.latest_sequence),
        summary.next_checkpoint.next_sequence(),
    )
}

fn authorization_decisions_json(
    decisions: &[&AuthorizationDecision],
    summary: &smart_home_core::AuthorizationDecisionLogSummary,
) -> String {
    format!(
        "{{\"summary\":{},\"decisions\":[{}]}}",
        authorization_decision_summary_json(summary),
        decisions
            .iter()
            .map(|decision| authorization_decision_json(decision))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn authorization_decision_summary_json(
    summary: &smart_home_core::AuthorizationDecisionLogSummary,
) -> String {
    format!(
        "{{\"total_decisions\":{},\"allowed_decisions\":{},\"denied_decisions\":{},\"tool_decisions\":{},\"command_decisions\":{},\"read_only_tier_decisions\":{},\"low_risk_tier_decisions\":{},\"human_approval_tier_decisions\":{},\"high_risk_tier_decisions\":{},\"decisions_with_missing_capabilities\":{},\"total_required_capabilities\":{},\"total_matched_grants\":{},\"total_missing_capabilities\":{}}}",
        summary.total_decisions,
        summary.allowed_decisions,
        summary.denied_decisions,
        summary.tool_decisions,
        summary.command_decisions,
        summary.read_only_tier_decisions,
        summary.low_risk_tier_decisions,
        summary.human_approval_tier_decisions,
        summary.high_risk_tier_decisions,
        summary.decisions_with_missing_capabilities,
        summary.total_required_capabilities,
        summary.total_matched_grants,
        summary.total_missing_capabilities,
    )
}

fn authorization_decision_json(decision: &AuthorizationDecision) -> String {
    format!(
        "{{\"principal_id\":{},\"subject\":{},\"outcome\":{},\"required_tier\":{},\"required_capabilities\":[{}],\"matched_grants\":[{}],\"missing_capabilities\":[{}],\"decided_at_ms\":{}}}",
        json_string(decision.principal_id.as_str()),
        authorization_subject_json(&decision.subject),
        json_string(authorization_outcome_label(decision.outcome)),
        json_string(privilege_tier_label(decision.required_tier)),
        json_id_array(decision.required_capabilities.iter().map(|id| id.as_str())),
        json_id_array(decision.matched_grants.iter().map(|id| id.as_str())),
        json_id_array(decision.missing_capabilities.iter().map(|id| id.as_str())),
        decision.decided_at_ms,
    )
}

fn authorization_subject_json(subject: &AuthorizationSubject) -> String {
    match subject {
        AuthorizationSubject::Tool(tool) => {
            format!(
                "{{\"kind\":\"tool\",\"tool_id\":{}}}",
                json_string(tool.descriptor().tool_id)
            )
        }
        AuthorizationSubject::Command {
            command_id,
            entity_id,
            command_type,
        } => format!(
            "{{\"kind\":\"command\",\"command_id\":{},\"entity_id\":{},\"command_type\":{}}}",
            json_string(command_id.as_str()),
            json_string(entity_id.as_str()),
            json_string(command_type_label(*command_type)),
        ),
    }
}

fn entities_registry_json(entities: &[&Entity], runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let stateful_entities = entities
        .iter()
        .filter(|entity| entity.state.is_some())
        .count();
    let stale_entities = entities
        .iter()
        .filter(|entity| {
            entity
                .state
                .as_ref()
                .is_none_or(|snapshot| snapshot.is_stale_at(now_ms))
        })
        .count();
    let commandable_entities = entities
        .iter()
        .filter(|entity| entity.capabilities.iter().any(capability_allows_command))
        .count();
    let capability_count = entities
        .iter()
        .map(|entity| entity.capabilities.len())
        .sum::<usize>();

    format!(
        "{{\"summary\":{{\"total_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"commandable_entities\":{},\"capability_count\":{}}},\"entities\":[{}]}}",
        entities.len(),
        stateful_entities,
        stale_entities,
        commandable_entities,
        capability_count,
        entities
            .iter()
            .map(|entity| entity_registry_json(entity, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
    )
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EntityInventoryCounts {
    total_entities: usize,
    commandable_entities: usize,
    stateful_entities: usize,
    stale_entities: usize,
    capability_count: usize,
}

impl EntityInventoryCounts {
    fn add(&mut self, other: Self) {
        self.total_entities += other.total_entities;
        self.commandable_entities += other.commandable_entities;
        self.stateful_entities += other.stateful_entities;
        self.stale_entities += other.stale_entities;
        self.capability_count += other.capability_count;
    }
}

fn devices_registry_json(devices: &[&Device], runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let mut entity_counts = EntityInventoryCounts::default();
    for device in devices {
        entity_counts.add(device_inventory_counts(device, runtime, now_ms));
    }

    format!(
        "{{\"summary\":{{\"total_devices\":{},\"online_devices\":{},\"pairing_candidate_devices\":{},\"attention_devices\":{},\"total_entities\":{},\"commandable_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"capability_count\":{}}},\"devices\":[{}]}}",
        devices.len(),
        devices
            .iter()
            .filter(|device| device.health.is_online())
            .count(),
        devices
            .iter()
            .filter(|device| device.health.is_pairing_candidate())
            .count(),
        devices
            .iter()
            .filter(|device| device.health.needs_attention())
            .count(),
        entity_counts.total_entities,
        entity_counts.commandable_entities,
        entity_counts.stateful_entities,
        entity_counts.stale_entities,
        entity_counts.capability_count,
        devices
            .iter()
            .map(|device| device_registry_json(device, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn device_registry_json(device: &Device, runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let entities = device_entities(device, runtime);
    let entity_counts = entity_inventory_counts(&entities, now_ms);
    let entity_ids = entities
        .iter()
        .map(|entity| entity.entity_id.as_str())
        .collect::<Vec<_>>();
    let home_assistant_entity_ids = entities
        .iter()
        .map(|entity| home_assistant_entity_id(entity))
        .collect::<Vec<_>>();
    let mut capability_ids = Vec::<String>::new();
    for entity in &entities {
        for capability in &entity.capabilities {
            push_unique_string(&mut capability_ids, capability.capability_id.as_str());
        }
    }

    format!(
        "{{\"device_id\":{},\"bridge_id\":{},\"name\":{},\"manufacturer\":{},\"model\":{},\"serial\":{},\"firmware_version\":{},\"room_id\":{},\"health\":{},\"entity_count\":{},\"commandable_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"capability_count\":{},\"entity_ids\":[{}],\"home_assistant_entity_ids\":[{}],\"capability_ids\":[{}]}}",
        json_string(device.device_id.as_str()),
        json_string(device.bridge_id.as_str()),
        json_string(&device.name),
        json_string(&device.manufacturer),
        json_string(&device.model),
        optional_str_json(device.serial.as_deref()),
        optional_str_json(device.firmware_version.as_deref()),
        optional_str_json(device.room_id.as_deref()),
        json_string(health_label(device.health)),
        entity_counts.total_entities,
        entity_counts.commandable_entities,
        entity_counts.stateful_entities,
        entity_counts.stale_entities,
        entity_counts.capability_count,
        json_id_array(entity_ids),
        json_string_array(&home_assistant_entity_ids),
        json_string_array(&capability_ids),
    )
}

fn bridges_registry_json(bridges: &[&Bridge], runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let mut total_devices = 0usize;
    let mut entity_counts = EntityInventoryCounts::default();
    let mut room_ids = Vec::<String>::new();
    for bridge in bridges {
        let devices = bridge_devices(bridge, runtime);
        total_devices += devices.len();
        for device in devices {
            if let Some(room_id) = &device.room_id {
                push_unique_string(&mut room_ids, room_id);
            }
            entity_counts.add(device_inventory_counts(device, runtime, now_ms));
        }
    }
    room_ids.sort();

    format!(
        "{{\"summary\":{{\"total_bridges\":{},\"online_bridges\":{},\"pairing_candidate_bridges\":{},\"attention_bridges\":{},\"total_devices\":{},\"total_entities\":{},\"commandable_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"capability_count\":{},\"room_count\":{}}},\"bridges\":[{}]}}",
        bridges.len(),
        bridges
            .iter()
            .filter(|bridge| bridge.health.is_online())
            .count(),
        bridges
            .iter()
            .filter(|bridge| bridge.health.is_pairing_candidate())
            .count(),
        bridges
            .iter()
            .filter(|bridge| bridge.health.needs_attention())
            .count(),
        total_devices,
        entity_counts.total_entities,
        entity_counts.commandable_entities,
        entity_counts.stateful_entities,
        entity_counts.stale_entities,
        entity_counts.capability_count,
        room_ids.len(),
        bridges
            .iter()
            .map(|bridge| bridge_registry_json(bridge, runtime, now_ms))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn bridge_registry_json(bridge: &Bridge, runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let devices = bridge_devices(bridge, runtime);
    let mut entity_counts = EntityInventoryCounts::default();
    let mut room_ids = Vec::<String>::new();
    for device in &devices {
        if let Some(room_id) = &device.room_id {
            push_unique_string(&mut room_ids, room_id);
        }
        entity_counts.add(device_inventory_counts(device, runtime, now_ms));
    }
    room_ids.sort();
    let device_ids = devices
        .iter()
        .map(|device| device.device_id.as_str())
        .collect::<Vec<_>>();

    format!(
        "{{\"bridge_id\":{},\"integration_id\":{},\"transport\":{},\"address\":{},\"hardware_model\":{},\"firmware_version\":{},\"health\":{},\"last_seen_at_ms\":{},\"device_count\":{},\"online_devices\":{},\"pairing_candidate_devices\":{},\"attention_devices\":{},\"entity_count\":{},\"commandable_entities\":{},\"stateful_entities\":{},\"stale_entities\":{},\"capability_count\":{},\"room_count\":{},\"room_ids\":[{}],\"device_ids\":[{}]}}",
        json_string(bridge.bridge_id.as_str()),
        json_string(bridge.integration_id.as_str()),
        json_string(bridge_transport_label(bridge.transport)),
        optional_str_json(bridge.address.as_deref()),
        optional_str_json(bridge.hardware_model.as_deref()),
        optional_str_json(bridge.firmware_version.as_deref()),
        json_string(health_label(bridge.health)),
        optional_u64_json(bridge.last_seen_at_ms),
        devices.len(),
        devices
            .iter()
            .filter(|device| device.health.is_online())
            .count(),
        devices
            .iter()
            .filter(|device| device.health.is_pairing_candidate())
            .count(),
        devices
            .iter()
            .filter(|device| device.health.needs_attention())
            .count(),
        entity_counts.total_entities,
        entity_counts.commandable_entities,
        entity_counts.stateful_entities,
        entity_counts.stale_entities,
        entity_counts.capability_count,
        room_ids.len(),
        json_string_array(&room_ids),
        json_id_array(device_ids),
    )
}

fn bridge_devices<'a>(bridge: &Bridge, runtime: &'a SmartHomeRuntime) -> Vec<&'a Device> {
    let mut devices = runtime
        .registry()
        .devices_for_bridge(&bridge.bridge_id)
        .collect::<Vec<_>>();
    devices.sort_by(|left, right| left.device_id.as_str().cmp(right.device_id.as_str()));
    devices
}

fn device_entities<'a>(device: &Device, runtime: &'a SmartHomeRuntime) -> Vec<&'a Entity> {
    let mut entities = runtime
        .registry()
        .entities_for_device(&device.device_id)
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));
    entities
}

fn device_inventory_counts(
    device: &Device,
    runtime: &SmartHomeRuntime,
    now_ms: u64,
) -> EntityInventoryCounts {
    let entities = device_entities(device, runtime);
    entity_inventory_counts(&entities, now_ms)
}

fn entity_inventory_counts(entities: &[&Entity], now_ms: u64) -> EntityInventoryCounts {
    EntityInventoryCounts {
        total_entities: entities.len(),
        commandable_entities: entities
            .iter()
            .filter(|entity| entity.capabilities.iter().any(capability_allows_command))
            .count(),
        stateful_entities: entities
            .iter()
            .filter(|entity| entity.state.is_some())
            .count(),
        stale_entities: entities
            .iter()
            .filter(|entity| {
                entity
                    .state
                    .as_ref()
                    .is_none_or(|snapshot| snapshot.is_stale_at(now_ms))
            })
            .count(),
        capability_count: entities
            .iter()
            .map(|entity| entity.capabilities.len())
            .sum(),
    }
}

fn rooms_json(rooms: &[RuntimeRoomSummary], runtime: &SmartHomeRuntime) -> String {
    let topology = runtime.topology_summary();
    let state_gap_rooms = rooms.iter().filter(|room| room.has_state_gaps()).count();
    let attention_rooms = rooms
        .iter()
        .filter(|room| room.has_attention_items())
        .count();
    let scene_rooms = rooms.iter().filter(|room| room.has_scene_actions()).count();

    format!(
        "{{\"summary\":{{\"total_rooms\":{},\"attention_rooms\":{},\"state_gap_rooms\":{},\"scene_rooms\":{},\"total_devices\":{},\"total_entities\":{},\"total_scenes\":{},\"topology_unique_rooms\":{}}},\"topology\":{{\"bridges\":{},\"devices\":{},\"entities\":{},\"scenes\":{},\"devices_with_room\":{},\"devices_without_room\":{},\"unique_rooms\":{},\"scene_actions\":{},\"room_scenes\":{}}},\"rooms\":[{}]}}",
        rooms.len(),
        attention_rooms,
        state_gap_rooms,
        scene_rooms,
        rooms.iter().map(|room| room.device_count).sum::<usize>(),
        rooms.iter().map(|room| room.entity_count).sum::<usize>(),
        rooms.iter().map(|room| room.scene_count).sum::<usize>(),
        topology.unique_rooms,
        topology.bridges,
        topology.devices,
        topology.entities,
        topology.scenes,
        topology.devices_with_room,
        topology.devices_without_room,
        topology.unique_rooms,
        topology.scene_actions,
        topology.room_scenes,
        rooms
            .iter()
            .map(room_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn room_json(room: &RuntimeRoomSummary) -> String {
    format!(
        "{{\"room_id\":{},\"device_count\":{},\"online_devices\":{},\"pairing_candidate_devices\":{},\"attention_devices\":{},\"entity_count\":{},\"commandable_entities\":{},\"entities_with_state\":{},\"entities_without_state\":{},\"stale_entities\":{},\"state_gap_count\":{},\"scene_count\":{},\"scene_action_count\":{},\"has_attention\":{},\"has_state_gaps\":{},\"has_scene_actions\":{}}}",
        json_string(&room.room_id),
        room.device_count,
        room.online_devices,
        room.pairing_candidate_devices,
        room.attention_devices,
        room.entity_count,
        room.commandable_entities,
        room.entities_with_state,
        room.entities_without_state,
        room.stale_entities,
        room.state_gap_count(),
        room.scene_count,
        room.scene_action_count,
        room.has_attention_items(),
        room.has_state_gaps(),
        room.has_scene_actions(),
    )
}

fn entity_registry_json(entity: &Entity, runtime: &SmartHomeRuntime, now_ms: u64) -> String {
    let device = runtime.registry().device(&entity.device_id);
    let bridge_id = device.map(|device| device.bridge_id.as_str());
    let manufacturer = device.map(|device| device.manufacturer.as_str());
    let model = device.map(|device| device.model.as_str());
    let room_id = device.and_then(|device| device.room_id.as_deref());
    let has_state = entity.state.is_some();
    let stale = entity
        .state
        .as_ref()
        .is_none_or(|snapshot| snapshot.is_stale_at(now_ms));
    let state_confidence = entity
        .state
        .as_ref()
        .map(|snapshot| json_string(state_confidence_label(snapshot.confidence)));
    let summary = entity.capability_summary();

    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"device_id\":{},\"bridge_id\":{},\"name\":{},\"domain\":{},\"entity_kind\":{},\"room_id\":{},\"manufacturer\":{},\"model\":{},\"has_state\":{},\"stale\":{},\"state_confidence\":{},\"capability_summary\":{{\"total\":{},\"observable\":{},\"commandable\":{},\"ranged\":{}}},\"capabilities\":[{}]}}",
        json_string(entity.entity_id.as_str()),
        json_string(home_assistant_entity_id(entity)),
        json_string(entity.device_id.as_str()),
        optional_str_json(bridge_id),
        json_string(&entity.name),
        json_string(entity_domain(entity.kind)),
        json_string(entity_kind_label(entity.kind)),
        optional_str_json(room_id),
        optional_str_json(manufacturer),
        optional_str_json(model),
        has_state,
        stale,
        state_confidence.unwrap_or_else(|| "null".to_string()),
        summary.total_capabilities,
        summary.observable_capabilities(),
        summary.commandable_capabilities(),
        summary.ranged_capabilities,
        entity
            .capabilities
            .iter()
            .map(capability_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn capability_json(capability: &Capability) -> String {
    format!(
        "{{\"capability_id\":{},\"mode\":{},\"value_kind\":{},\"unit\":{},\"min\":{},\"max\":{},\"step\":{},\"observable\":{},\"commandable\":{}}}",
        json_string(capability.capability_id.as_str()),
        json_string(capability_mode_label(capability.mode)),
        json_string(value_kind_label(capability.value_kind)),
        capability
            .unit
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        optional_f64_json(capability.min),
        optional_f64_json(capability.max),
        optional_f64_json(capability.step),
        matches!(
            capability.mode,
            CapabilityMode::Observe | CapabilityMode::ObserveAndCommand
        ),
        capability_allows_command(capability),
    )
}

fn desired_states_json(
    desired_states: &[&DesiredEntityState],
    runtime: &SmartHomeRuntime,
) -> String {
    let desired_capability_count = desired_states
        .iter()
        .map(|desired_state| desired_state.desired.len())
        .sum::<usize>();
    format!(
        "{{\"summary\":{{\"total_desired_states\":{},\"total_desired_capabilities\":{}}},\"desired_states\":[{}]}}",
        desired_states.len(),
        desired_capability_count,
        desired_states
            .iter()
            .map(|desired_state| desired_state_json(desired_state, runtime))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn desired_state_json(desired_state: &DesiredEntityState, runtime: &SmartHomeRuntime) -> String {
    let home_assistant_entity_id = runtime
        .registry()
        .entity(&desired_state.entity_id)
        .map(home_assistant_entity_id)
        .unwrap_or_else(|| home_assistant_entity_id_for(&desired_state.entity_id));
    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"requested_by\":{},\"command_timeout_ms\":{},\"desired\":[{}]}}",
        json_string(desired_state.entity_id.as_str()),
        json_string(home_assistant_entity_id),
        json_string(&desired_state.requested_by),
        desired_state.command_timeout_ms,
        desired_state
            .desired
            .iter()
            .map(state_delta_json)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn runtime_event_query(request: &WebRequest) -> Result<RuntimeEventQuery, ApiError> {
    let mut query = RuntimeEventQuery::new()
        .from_checkpoint(RuntimeEventCheckpoint::from_next_sequence(
            query_u64(request, "from_sequence")?.unwrap_or(0),
        ))
        .with_limit(query_limit(request, 50, 500)?);

    if query_string(request, "sort").is_some_and(|sort| sort == "desc") {
        query = query.sorted_by(RuntimeEventSort::SequenceDesc);
    }
    if let Some(kind) = query_string(request, "kind") {
        query = query.matching(match kind {
            "all" => RuntimeEventFilter::All,
            "commands" | "command_results" => RuntimeEventFilter::Commands,
            "supervision" => RuntimeEventFilter::Supervision,
            other => {
                return Err(ApiError::bad_request(format!(
                    "unsupported event kind `{other}`"
                )));
            }
        });
    }

    Ok(query)
}

fn runtime_command_result_query(
    request: &WebRequest,
) -> Result<RuntimeCommandResultQuery, ApiError> {
    let mut query = RuntimeCommandResultQuery::new()
        .from_checkpoint(RuntimeEventCheckpoint::from_next_sequence(
            query_u64(request, "from_sequence")?.unwrap_or(0),
        ))
        .sorted_by(RuntimeCommandResultSort::SequenceDesc)
        .with_limit(query_limit(request, 50, 500)?);
    if let Some(status) = query_string(request, "status") {
        query = query.with_status(command_status_from_label(status)?);
    }
    Ok(query)
}

fn runtime_authorization_decision_query(
    request: &WebRequest,
) -> Result<RuntimeAuthorizationDecisionQuery, ApiError> {
    let mut query =
        RuntimeAuthorizationDecisionQuery::new().with_limit(query_limit(request, 50, 500)?);
    if let Some(principal_id) = query_string(request, "principal_id") {
        query = query.for_principal(AgentId::trusted(principal_id));
    }
    if let Some(outcome) = query_string(request, "outcome") {
        query = query.with_outcome(authorization_outcome_from_label(outcome)?);
    }
    Ok(query)
}

fn desired_state_query(
    runtime: &SmartHomeRuntime,
    request: &WebRequest,
) -> Result<DesiredStateQuery, ApiError> {
    let mut query = DesiredStateQuery::new().with_limit(query_limit(request, 100, 500)?);
    if let Some(entity_id) = query_string(request, "entity_id") {
        query = query.for_entity(runtime_entity_id(runtime, entity_id)?);
    }
    if let Some(requested_by) = query_string(request, "requested_by") {
        query = query.requested_by(requested_by);
    }
    if let Some(capability_id) = query_string(request, "capability_id") {
        query = query.with_capability(CapabilityId::trusted(capability_id));
    }
    Ok(query)
}

fn runtime_entities<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a Entity>, ApiError> {
    let domain = query_string(request, "domain");
    let kind = query_string(request, "kind")
        .map(entity_kind_from_label)
        .transpose()?;
    let capability_id = query_string(request, "capability_id");
    let commandable = query_bool(request, "commandable")?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut entities = runtime
        .registry()
        .entities()
        .filter(|entity| domain.is_none_or(|domain| entity_domain(entity.kind) == domain))
        .filter(|entity| kind.is_none_or(|kind| entity.kind == kind))
        .filter(|entity| {
            capability_id.is_none_or(|capability_id| {
                entity
                    .capabilities
                    .iter()
                    .any(|capability| capability.capability_id.as_str() == capability_id)
            })
        })
        .filter(|entity| {
            commandable.is_none_or(|commandable| {
                entity.capabilities.iter().any(capability_allows_command) == commandable
            })
        })
        .collect::<Vec<_>>();

    entities.sort_by(|left, right| left.entity_id.as_str().cmp(right.entity_id.as_str()));
    entities.truncate(limit);
    Ok(entities)
}

fn runtime_devices<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a Device>, ApiError> {
    let bridge_id = query_string(request, "bridge_id");
    let room_id = query_string(request, "room_id");
    let manufacturer = query_string(request, "manufacturer");
    let health = query_string(request, "health")
        .map(health_from_label)
        .transpose()?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut devices = runtime
        .registry()
        .devices()
        .filter(|device| bridge_id.is_none_or(|bridge_id| device.bridge_id.as_str() == bridge_id))
        .filter(|device| room_id.is_none_or(|room_id| device.room_id.as_deref() == Some(room_id)))
        .filter(|device| {
            manufacturer
                .is_none_or(|manufacturer| device.manufacturer.eq_ignore_ascii_case(manufacturer))
        })
        .filter(|device| health.is_none_or(|health| device.health == health))
        .collect::<Vec<_>>();

    devices.sort_by(|left, right| left.device_id.as_str().cmp(right.device_id.as_str()));
    devices.truncate(limit);
    Ok(devices)
}

fn runtime_bridges<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a Bridge>, ApiError> {
    let integration_id = query_string(request, "integration_id");
    let transport = query_string(request, "transport")
        .map(bridge_transport_from_label)
        .transpose()?;
    let health = query_string(request, "health")
        .map(health_from_label)
        .transpose()?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut bridges = runtime
        .registry()
        .bridges()
        .filter(|bridge| {
            integration_id
                .is_none_or(|integration_id| bridge.integration_id.as_str() == integration_id)
        })
        .filter(|bridge| transport.is_none_or(|transport| bridge.transport == transport))
        .filter(|bridge| health.is_none_or(|health| bridge.health == health))
        .collect::<Vec<_>>();

    bridges.sort_by(|left, right| left.bridge_id.as_str().cmp(right.bridge_id.as_str()));
    bridges.truncate(limit);
    Ok(bridges)
}

fn runtime_room_query(request: &WebRequest) -> Result<RuntimeRoomQuery, ApiError> {
    let mut query = RuntimeRoomQuery::new().with_limit(query_limit(request, 100, 1_000)?);
    if let Some(room_id) = query_string(request, "room_id") {
        query = query.for_room(room_id);
    }
    if query_bool(request, "attention_only")?.unwrap_or(false) {
        query = query.attention_only(true);
    }
    if query_bool(request, "state_gaps_only")?.unwrap_or(false) {
        query = query.state_gaps_only(true);
    }
    if let Some(sort) = query_string(request, "sort") {
        query = query.sorted_by(room_sort_from_label(sort)?);
    }
    Ok(query)
}

fn state_history_events<'a>(
    runtime: &'a SmartHomeRuntime,
    request: &WebRequest,
) -> Result<Vec<&'a DeviceEvent>, ApiError> {
    let entity_id = history_entity_filter(request)
        .map(|entity_id| runtime_entity_id(runtime, entity_id))
        .transpose()?;
    let event_type = query_string(request, "event_type")
        .map(device_event_type_from_label)
        .transpose()?;
    let observed_at_or_after_ms = query_u64(request, "observed_at_or_after_ms")?;
    let received_at_or_after_ms = query_u64(request, "received_at_or_after_ms")?;
    let limit = query_limit(request, 100, 1_000)?;

    let mut events = runtime
        .registry()
        .events()
        .filter(|event| {
            entity_id
                .as_ref()
                .is_none_or(|entity_id| event.entity_id.as_ref() == Some(entity_id))
        })
        .filter(|event| event_type.is_none_or(|event_type| event.event_type == event_type))
        .filter(|event| {
            observed_at_or_after_ms
                .is_none_or(|observed_at_ms| event.observed_at_ms >= observed_at_ms)
        })
        .filter(|event| {
            received_at_or_after_ms
                .is_none_or(|received_at_ms| event.received_at_ms >= received_at_ms)
        })
        .collect::<Vec<_>>();

    if query_string(request, "sort").is_some_and(|sort| sort == "desc") {
        events.reverse();
    }
    events.truncate(limit);
    Ok(events)
}

fn history_entity_filter<'a>(request: &'a WebRequest) -> Option<&'a str> {
    query_string(request, "entity_id").or_else(|| query_string(request, "filter_entity_id"))
}

fn runtime_entity_id(runtime: &SmartHomeRuntime, value: &str) -> Result<EntityId, ApiError> {
    runtime
        .registry()
        .entities()
        .find(|entity| entity_matches_external_id(entity, value))
        .map(|entity| entity.entity_id.clone())
        .ok_or_else(|| ApiError::not_found(format!("entity `{value}` not found")))
}

fn runtime_entity(runtime: &SmartHomeRuntime, value: &str) -> Result<Entity, ApiError> {
    runtime
        .registry()
        .entities()
        .find(|entity| entity_matches_external_id(entity, value))
        .cloned()
        .ok_or_else(|| ApiError::not_found(format!("entity `{value}` not found")))
}

fn home_assistant_entity_id_for_runtime(
    runtime: &SmartHomeRuntime,
    entity_id: &EntityId,
) -> String {
    runtime
        .registry()
        .entity(entity_id)
        .map(home_assistant_entity_id)
        .unwrap_or_else(|| home_assistant_entity_id_for(entity_id))
}

fn state_history_json(events: &[&DeviceEvent], runtime: &SmartHomeRuntime) -> String {
    let mut entity_ids = Vec::<String>::new();
    let mut state_delta_count = 0usize;
    let mut first_observed_at_ms = None;
    let mut latest_observed_at_ms = None;

    for event in events {
        if let Some(entity_id) = &event.entity_id {
            push_unique_string(&mut entity_ids, entity_id.as_str());
        }
        if event.state_delta.is_some() {
            state_delta_count += 1;
        }
        first_observed_at_ms = Some(
            first_observed_at_ms
                .map(|current: u64| current.min(event.observed_at_ms))
                .unwrap_or(event.observed_at_ms),
        );
        latest_observed_at_ms = Some(
            latest_observed_at_ms
                .map(|current: u64| current.max(event.observed_at_ms))
                .unwrap_or(event.observed_at_ms),
        );
    }

    format!(
        "{{\"summary\":{{\"total_events\":{},\"entity_count\":{},\"state_delta_count\":{},\"first_observed_at_ms\":{},\"latest_observed_at_ms\":{}}},\"events\":[{}]}}",
        events.len(),
        entity_ids.len(),
        state_delta_count,
        optional_u64_json(first_observed_at_ms),
        optional_u64_json(latest_observed_at_ms),
        events
            .iter()
            .map(|event| state_history_event_json(event, runtime))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn home_assistant_history_json(events: &[&DeviceEvent], runtime: &SmartHomeRuntime) -> String {
    let mut by_entity = BTreeMap::<String, Vec<&DeviceEvent>>::new();
    for event in events {
        let key = event
            .entity_id
            .as_ref()
            .and_then(|entity_id| runtime.registry().entity(entity_id))
            .map(home_assistant_entity_id)
            .unwrap_or_else(|| "unknown.unknown".to_string());
        by_entity.entry(key).or_default().push(event);
    }

    format!(
        "[{}]",
        by_entity
            .into_values()
            .map(|events| {
                format!(
                    "[{}]",
                    events
                        .iter()
                        .map(|event| home_assistant_history_event_json(event, runtime))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn home_assistant_history_event_json(event: &DeviceEvent, runtime: &SmartHomeRuntime) -> String {
    let entity = event
        .entity_id
        .as_ref()
        .and_then(|entity_id| runtime.registry().entity(entity_id));
    let home_assistant_entity_id = entity
        .map(home_assistant_entity_id)
        .unwrap_or_else(|| "unknown.unknown".to_string());
    let canonical_entity_id = event.entity_id.as_ref().map(|entity_id| entity_id.as_str());
    let (state, capability_id, state_delta_value) = match &event.state_delta {
        Some(delta) => (
            value_json(&delta.value),
            json_string(delta.capability_id.as_str()),
            value_json(&delta.value),
        ),
        None => (
            json_string(device_event_type_label(event.event_type)),
            "null".to_string(),
            "null".to_string(),
        ),
    };

    format!(
        "{{\"entity_id\":{},\"state\":{},\"attributes\":{{\"canonical_entity_id\":{},\"event_id\":{},\"bridge_id\":{},\"device_id\":{},\"event_type\":{},\"capability_id\":{},\"state_delta_value\":{},\"raw_ref\":{}}},\"last_changed_ms\":{},\"last_updated_ms\":{},\"context\":{{\"source\":\"event_stream\",\"correlation_id\":{}}}}}",
        json_string(home_assistant_entity_id),
        state,
        canonical_entity_id
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        json_string(event.event_id.as_str()),
        json_string(event.bridge_id.as_str()),
        event
            .device_id
            .as_ref()
            .map(|device_id| json_string(device_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
        json_string(device_event_type_label(event.event_type)),
        capability_id,
        state_delta_value,
        event
            .raw_ref
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        event.observed_at_ms,
        event.received_at_ms,
        event
            .correlation_id
            .as_ref()
            .map(|correlation_id| json_string(correlation_id.as_str()))
            .unwrap_or_else(|| "null".to_string()),
    )
}

fn state_history_event_json(event: &DeviceEvent, runtime: &SmartHomeRuntime) -> String {
    let home_assistant_entity_id = event.entity_id.as_ref().and_then(|entity_id| {
        runtime
            .registry()
            .entity(entity_id)
            .map(home_assistant_entity_id)
    });
    format!(
        "{{\"home_assistant_entity_id\":{},\"event\":{}}}",
        home_assistant_entity_id
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
        device_event_json(event),
    )
}

#[derive(Debug, Clone, PartialEq)]
struct ServiceCall {
    target_entity_ids: Vec<String>,
    target_scene_ids: Vec<String>,
    body: JsonValue,
    idempotency_key: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
struct ServiceCommand {
    entity_id: EntityId,
    command_type: CommandType,
    arguments: Value,
    idempotency_key: Option<String>,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiError {
    status: u16,
    message: String,
}

impl ApiError {
    fn new(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, message)
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, message)
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403, message)
    }
}

fn set_desired_state_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
    allow_home_assistant_state_body: bool,
) -> WebResponse {
    let Some(target) = request.route_params.get("entity_id") else {
        return json_error(400, "missing entity_id");
    };

    let mut runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity = match runtime_entity(&runtime_guard, target) {
        Ok(entity) => entity,
        Err(error) => return api_error_response(error),
    };
    let desired_state = match parse_desired_state_request(
        request.body(),
        &entity,
        runtime.principal_id.as_str(),
        allow_home_assistant_state_body,
    ) {
        Ok(desired_state) => desired_state,
        Err(error) => return api_error_response(error),
    };

    let output = match runtime_guard.execute_set_desired_state_tool(
        runtime.principal_id.clone(),
        RuntimeSetDesiredStateToolRequest::new(desired_state),
        runtime.now_ms,
    ) {
        Ok(output) => output,
        Err(error) => return api_error_response(runtime_error_to_api_error(error)),
    };
    let query = DesiredStateQuery::new().for_entity(entity.entity_id.clone());
    let desired_states = runtime_guard.query_desired_states(&query);
    WebResponse::json(set_desired_state_json(&output, &desired_states, &runtime_guard).into_bytes())
}

fn clear_desired_state_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let Some(target) = request.route_params.get("entity_id") else {
        return json_error(400, "missing entity_id");
    };

    let mut runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let entity_id = match runtime_entity_id(&runtime_guard, target) {
        Ok(entity_id) => entity_id,
        Err(error) => return api_error_response(error),
    };

    let output = match runtime_guard.execute_clear_desired_state_tool(
        runtime.principal_id.clone(),
        RuntimeClearDesiredStateToolRequest::new(entity_id),
        runtime.now_ms,
    ) {
        Ok(output) => output,
        Err(error) => return api_error_response(runtime_error_to_api_error(error)),
    };
    let query = DesiredStateQuery::new().for_entity(output.entity_id.clone());
    let desired_states = runtime_guard.query_desired_states(&query);
    WebResponse::json(
        clear_desired_state_json(&output, &desired_states, &runtime_guard).into_bytes(),
    )
}

fn parse_desired_state_request(
    body: &[u8],
    entity: &Entity,
    default_requested_by: &str,
    allow_home_assistant_state_body: bool,
) -> Result<DesiredEntityState, ApiError> {
    let body = parse_json_body(body)?;
    let desired = if let Some(value) = body.get("desired_state").or_else(|| body.get("desired")) {
        desired_state_deltas_from_json(value)?
    } else if allow_home_assistant_state_body {
        home_assistant_state_deltas(entity, &body)?
    } else {
        return Err(ApiError::bad_request(
            "desired-state request requires a desired_state object",
        ));
    };

    if desired.is_empty() {
        return Err(ApiError::bad_request(
            "desired-state request must include at least one capability",
        ));
    }

    let requested_by = json_string_field(&body, "requested_by")
        .unwrap_or_else(|| default_requested_by.to_string());
    let mut desired_state =
        DesiredEntityState::new(entity.entity_id.clone(), desired).requested_by(requested_by);
    if let Some(timeout_ms) =
        json_u64_field(&body, "command_timeout_ms").or_else(|| json_u64_field(&body, "timeout_ms"))
    {
        desired_state = desired_state.with_command_timeout(timeout_ms);
    }
    Ok(desired_state)
}

fn parse_json_body(body: &[u8]) -> Result<JsonValue, ApiError> {
    if body.is_empty() {
        return Err(ApiError::bad_request("JSON body is required"));
    }
    serde_json::from_slice(body)
        .map_err(|error| ApiError::bad_request(format!("invalid JSON body: {error}")))
}

fn desired_state_deltas_from_json(value: &JsonValue) -> Result<Vec<StateDelta>, ApiError> {
    let fields = value
        .as_object()
        .ok_or_else(|| ApiError::bad_request("desired_state must be an object"))?;
    let mut deltas = Vec::new();
    for (capability_id, value) in fields {
        deltas.push(StateDelta {
            capability_id: CapabilityId::trusted(capability_id.clone()),
            value: json_capability_value(capability_id, value)?,
        });
    }
    deltas.sort_by(|left, right| {
        left.capability_id
            .as_str()
            .cmp(right.capability_id.as_str())
    });
    Ok(deltas)
}

fn home_assistant_state_deltas(
    entity: &Entity,
    body: &JsonValue,
) -> Result<Vec<StateDelta>, ApiError> {
    let attributes = body.get("attributes").unwrap_or(body);
    let mut deltas = Vec::new();
    match entity_domain(entity.kind) {
        "light" => {
            if let Some(state) = json_string_field(body, "state") {
                match state.as_str() {
                    "on" => deltas.push(state_delta("light.on_off", Value::Bool(true))),
                    "off" => deltas.push(state_delta("light.on_off", Value::Bool(false))),
                    other => {
                        return Err(ApiError::bad_request(format!(
                            "unsupported light state `{other}`"
                        )));
                    }
                }
            }
            if let Some(value) = brightness_value(attributes)? {
                deltas.push(state_delta("light.brightness", value));
            }
            if let Some(value) = color_temperature_value(attributes)? {
                deltas.push(state_delta("light.color_temperature", value));
            }
            if let Some(value) = color_value(attributes)? {
                deltas.push(state_delta("light.color", value));
            }
        }
        "lock" => {
            let state = json_string_field(body, "state")
                .ok_or_else(|| ApiError::bad_request("lock desired state requires state"))?;
            match state.as_str() {
                "locked" | "unlocked" => deltas.push(state_delta("lock.state", Value::Text(state))),
                other => {
                    return Err(ApiError::bad_request(format!(
                        "unsupported lock state `{other}`"
                    )));
                }
            }
        }
        "climate" => {
            let value = number_or_integer_field(attributes, "temperature")
                .or_else(|| number_or_integer_field(body, "temperature"))
                .ok_or_else(|| {
                    ApiError::bad_request("climate desired state requires temperature")
                })?;
            deltas.push(state_delta("climate.setpoint", value));
        }
        domain => {
            return Err(ApiError::bad_request(format!(
                "Home Assistant state body is not supported for domain `{domain}`; use desired_state"
            )));
        }
    }

    deltas.sort_by(|left, right| {
        left.capability_id
            .as_str()
            .cmp(right.capability_id.as_str())
    });
    deltas.dedup_by(|left, right| left.capability_id == right.capability_id);
    Ok(deltas)
}

fn state_delta(capability_id: impl Into<String>, value: Value) -> StateDelta {
    StateDelta {
        capability_id: CapabilityId::trusted(capability_id.into()),
        value,
    }
}

fn json_capability_value(capability_id: &str, value: &JsonValue) -> Result<Value, ApiError> {
    match capability_id {
        "light.on_off" => match value {
            JsonValue::Bool(value) => Ok(Value::Bool(*value)),
            JsonValue::String(value) if value == "on" => Ok(Value::Bool(true)),
            JsonValue::String(value) if value == "off" => Ok(Value::Bool(false)),
            _ => Err(ApiError::bad_request("light.on_off must be boolean")),
        },
        "light.brightness" => json_percentage_value(value, capability_id),
        "light.color_temperature" => json_i64_value(value, capability_id).map(Value::Integer),
        "lock.state" => value
            .as_str()
            .map(|state| Value::Text(state.to_string()))
            .ok_or_else(|| ApiError::bad_request("lock.state must be a string")),
        "climate.setpoint" => json_number_or_integer_value(value, capability_id),
        _ => json_value_to_value(value),
    }
}

fn json_percentage_value(value: &JsonValue, field: &str) -> Result<Value, ApiError> {
    let value = value
        .as_u64()
        .ok_or_else(|| ApiError::bad_request(format!("{field} must be an integer percentage")))?;
    if value > 100 {
        return Err(ApiError::bad_request(format!(
            "{field} must be between 0 and 100"
        )));
    }
    Ok(Value::Percentage(value as u8))
}

fn json_i64_value(value: &JsonValue, field: &str) -> Result<i64, ApiError> {
    value
        .as_i64()
        .ok_or_else(|| ApiError::bad_request(format!("{field} must be an integer")))
}

fn json_number_or_integer_value(value: &JsonValue, field: &str) -> Result<Value, ApiError> {
    value
        .as_i64()
        .map(Value::Integer)
        .or_else(|| value.as_f64().map(Value::Number))
        .ok_or_else(|| ApiError::bad_request(format!("{field} must be numeric")))
}

fn json_value_to_value(value: &JsonValue) -> Result<Value, ApiError> {
    match value {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(value) => Ok(Value::Bool(*value)),
        JsonValue::Number(value) => value
            .as_i64()
            .map(Value::Integer)
            .or_else(|| value.as_f64().map(Value::Number))
            .ok_or_else(|| ApiError::bad_request("JSON number is not representable")),
        JsonValue::String(value) => Ok(Value::Text(value.clone())),
        JsonValue::Array(values) => values
            .iter()
            .map(json_value_to_value)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        JsonValue::Object(fields) => {
            let mut fields = fields
                .iter()
                .map(|(key, value)| Ok((key.clone(), json_value_to_value(value)?)))
                .collect::<Result<Vec<_>, ApiError>>()?;
            fields.sort_by(|left, right| left.0.cmp(&right.0));
            Ok(Value::Object(fields))
        }
    }
}

fn set_desired_state_json(
    output: &RuntimeSetDesiredStateToolOutput,
    desired_states: &[&DesiredEntityState],
    runtime: &SmartHomeRuntime,
) -> String {
    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"replaced\":{},\"desired_state\":{},\"previous\":{},\"desired_states\":{}}}",
        json_string(output.desired_state.entity_id.as_str()),
        json_string(home_assistant_entity_id_for_runtime(
            runtime,
            &output.desired_state.entity_id,
        )),
        output.replaced,
        desired_state_json(&output.desired_state, runtime),
        output
            .previous
            .as_ref()
            .map(|desired_state| desired_state_json(desired_state, runtime))
            .unwrap_or_else(|| "null".to_string()),
        desired_states_json(desired_states, runtime),
    )
}

fn clear_desired_state_json(
    output: &RuntimeClearDesiredStateToolOutput,
    desired_states: &[&DesiredEntityState],
    runtime: &SmartHomeRuntime,
) -> String {
    format!(
        "{{\"entity_id\":{},\"home_assistant_entity_id\":{},\"removed\":{},\"removed_desired_state\":{},\"desired_states\":{}}}",
        json_string(output.entity_id.as_str()),
        json_string(home_assistant_entity_id_for_runtime(
            runtime,
            &output.entity_id,
        )),
        output.removed(),
        output
            .removed
            .as_ref()
            .map(|desired_state| desired_state_json(desired_state, runtime))
            .unwrap_or_else(|| "null".to_string()),
        desired_states_json(desired_states, runtime),
    )
}

fn service_call_response(
    runtime: &SmartHomePlatformHttpRuntime,
    request: &WebRequest,
) -> WebResponse {
    let domain = match request.route_params.get("domain") {
        Some(domain) => domain.as_str(),
        None => return json_error(400, "missing domain"),
    };
    let service = match request.route_params.get("service") {
        Some(service) => service.as_str(),
        None => return json_error(400, "missing service"),
    };

    let call = match parse_service_call(request.body()) {
        Ok(call) => call,
        Err(error) => return api_error_response(error),
    };

    let mut runtime_guard = runtime
        .runtime
        .lock()
        .expect("smart-home runtime mutex should not be poisoned");
    let before = SmartHomePlatformHttpState::from_runtime(
        &runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms,
    );
    let commands = match service_commands(&before, domain, service, &call) {
        Ok(commands) => commands,
        Err(error) => return api_error_response(error),
    };

    let mut results = Vec::new();
    for command in commands {
        let mut request = RuntimeCommandToolRequest::new(
            command.entity_id,
            command.command_type,
            command.arguments,
        );
        if let Some(idempotency_key) = command.idempotency_key {
            request = request.with_idempotency_key(idempotency_key);
        }
        if let Some(timeout_ms) = command.timeout_ms {
            request = request.with_timeout_ms(timeout_ms);
        }

        match runtime_guard.execute_command_tool(
            runtime.principal_id.clone(),
            request,
            runtime.now_ms,
        ) {
            Ok(result) => results.push(result),
            Err(error) => return api_error_response(runtime_error_to_api_error(error)),
        }
    }

    let after = SmartHomePlatformHttpState::from_runtime(
        &runtime_guard,
        runtime.config.clone(),
        runtime.event_types.clone(),
        runtime.now_ms,
    );
    WebResponse::json(service_call_json(domain, service, &results, &after).into_bytes())
}

fn parse_service_call(body: &[u8]) -> Result<ServiceCall, ApiError> {
    let body = if body.is_empty() {
        JsonValue::Object(Default::default())
    } else {
        serde_json::from_slice(body)
            .map_err(|error| ApiError::bad_request(format!("invalid JSON body: {error}")))?
    };

    let mut target_entity_ids = Vec::new();
    let mut target_scene_ids = Vec::new();
    collect_string_values(&body, "entity_id", &mut target_entity_ids);
    collect_string_values(&body, "entity_ids", &mut target_entity_ids);
    collect_string_values(&body, "scene_id", &mut target_scene_ids);
    collect_string_values(&body, "scene_ids", &mut target_scene_ids);

    if let Some(target) = body.get("target") {
        collect_string_values(target, "entity_id", &mut target_entity_ids);
        collect_string_values(target, "entity_ids", &mut target_entity_ids);
        collect_string_values(target, "scene_id", &mut target_scene_ids);
        collect_string_values(target, "scene_ids", &mut target_scene_ids);
    }

    target_entity_ids.sort();
    target_entity_ids.dedup();
    target_scene_ids.sort();
    target_scene_ids.dedup();

    Ok(ServiceCall {
        idempotency_key: json_string_field(&body, "idempotency_key"),
        timeout_ms: json_u64_field(&body, "timeout_ms"),
        target_entity_ids,
        target_scene_ids,
        body,
    })
}

fn service_commands(
    state: &SmartHomePlatformHttpState,
    domain: &str,
    service: &str,
    call: &ServiceCall,
) -> Result<Vec<ServiceCommand>, ApiError> {
    if domain == "scene" && service == "turn_on" {
        return scene_service_commands(state, call);
    }

    let entities = target_entities(state, domain, call)?;
    let mut commands = Vec::new();
    for entity in entities {
        commands.extend(entity_service_commands(domain, service, entity, call)?);
    }
    Ok(commands)
}

fn target_entities<'a>(
    state: &'a SmartHomePlatformHttpState,
    domain: &str,
    call: &ServiceCall,
) -> Result<Vec<&'a Entity>, ApiError> {
    if call.target_entity_ids.is_empty() {
        return Err(ApiError::bad_request(
            "service call requires an entity target",
        ));
    }

    let mut entities = Vec::new();
    for target in &call.target_entity_ids {
        let entity = state
            .entities
            .iter()
            .find(|entity| entity_matches_external_id(entity, target))
            .ok_or_else(|| ApiError::not_found(format!("entity target `{target}` not found")))?;
        if entity_domain(entity.kind) != domain {
            return Err(ApiError::bad_request(format!(
                "entity target `{target}` is not in domain `{domain}`"
            )));
        }
        entities.push(entity);
    }
    Ok(entities)
}

fn scene_service_commands(
    state: &SmartHomePlatformHttpState,
    call: &ServiceCall,
) -> Result<Vec<ServiceCommand>, ApiError> {
    if call.target_scene_ids.is_empty() && call.target_entity_ids.is_empty() {
        return Err(ApiError::bad_request(
            "scene.turn_on requires a scene target",
        ));
    }

    let mut commands = Vec::new();
    for target in call
        .target_scene_ids
        .iter()
        .chain(call.target_entity_ids.iter())
    {
        let scene = state
            .scenes
            .iter()
            .find(|scene| scene_matches_external_id(scene, target))
            .ok_or_else(|| ApiError::not_found(format!("scene target `{target}` not found")))?;
        for action in &scene.actions {
            for delta in state_deltas_from_value(&action.desired_state)? {
                let (command_type, arguments) =
                    command_from_capability_value(&action.entity_id, &delta)?;
                commands.push(ServiceCommand {
                    entity_id: action.entity_id.clone(),
                    command_type,
                    arguments,
                    idempotency_key: call.idempotency_key.clone(),
                    timeout_ms: call.timeout_ms,
                });
            }
        }
    }
    Ok(commands)
}

fn entity_service_commands(
    domain: &str,
    service: &str,
    entity: &Entity,
    call: &ServiceCall,
) -> Result<Vec<ServiceCommand>, ApiError> {
    let mut commands = Vec::new();
    match (domain, service) {
        ("light", "turn_on") => {
            commands.push(service_command(
                entity,
                CommandType::TurnOn,
                Value::Null,
                call,
            ));
            if let Some(value) = brightness_value(&call.body)? {
                commands.push(service_command(
                    entity,
                    CommandType::SetBrightness,
                    value,
                    call,
                ));
            }
            if let Some(value) = color_temperature_value(&call.body)? {
                commands.push(service_command(
                    entity,
                    CommandType::SetColorTemperature,
                    value,
                    call,
                ));
            }
            if let Some(value) = color_value(&call.body)? {
                commands.push(service_command(entity, CommandType::SetColor, value, call));
            }
        }
        ("light", "turn_off") => {
            commands.push(service_command(
                entity,
                CommandType::TurnOff,
                Value::Null,
                call,
            ));
        }
        ("light", "set_brightness") => {
            let value = brightness_value(&call.body)?.ok_or_else(|| {
                ApiError::bad_request("light.set_brightness requires brightness_pct or brightness")
            })?;
            commands.push(service_command(
                entity,
                CommandType::SetBrightness,
                value,
                call,
            ));
        }
        ("light", "set_color_temperature") => {
            let value = color_temperature_value(&call.body)?.ok_or_else(|| {
                ApiError::bad_request(
                    "light.set_color_temperature requires color_temp, color_temp_kelvin, or kelvin",
                )
            })?;
            commands.push(service_command(
                entity,
                CommandType::SetColorTemperature,
                value,
                call,
            ));
        }
        ("light", "set_color") => {
            let value = color_value(&call.body)?
                .ok_or_else(|| ApiError::bad_request("light.set_color requires rgb_color"))?;
            commands.push(service_command(entity, CommandType::SetColor, value, call));
        }
        ("lock", "lock") => {
            commands.push(service_command(
                entity,
                CommandType::SetLock,
                Value::Text("locked".to_string()),
                call,
            ));
        }
        ("lock", "unlock") => {
            commands.push(service_command(
                entity,
                CommandType::SetLock,
                Value::Text("unlocked".to_string()),
                call,
            ));
        }
        ("climate", "set_temperature") => {
            let value = number_or_integer_field(&call.body, "temperature").ok_or_else(|| {
                ApiError::bad_request("climate.set_temperature requires temperature")
            })?;
            commands.push(service_command(
                entity,
                CommandType::SetThermostatSetpoint,
                value,
                call,
            ));
        }
        _ => {
            return Err(ApiError::bad_request(format!(
                "unsupported service `{domain}.{service}`"
            )));
        }
    }

    Ok(commands)
}

fn service_command(
    entity: &Entity,
    command_type: CommandType,
    arguments: Value,
    call: &ServiceCall,
) -> ServiceCommand {
    ServiceCommand {
        entity_id: entity.entity_id.clone(),
        command_type,
        arguments,
        idempotency_key: call.idempotency_key.clone(),
        timeout_ms: call.timeout_ms,
    }
}

fn state_deltas_from_value(value: &Value) -> Result<Vec<StateDelta>, ApiError> {
    match value {
        Value::Object(fields) => Ok(fields
            .iter()
            .map(|(capability_id, value)| StateDelta {
                capability_id: CapabilityId::trusted(capability_id.clone()),
                value: value.clone(),
            })
            .collect()),
        _ => Err(ApiError::bad_request(
            "scene action desired_state must be an object",
        )),
    }
}

fn command_from_capability_value(
    entity_id: &EntityId,
    delta: &StateDelta,
) -> Result<(CommandType, Value), ApiError> {
    match delta.capability_id.as_str() {
        "light.on_off" => match delta.value {
            Value::Bool(true) => Ok((CommandType::TurnOn, Value::Null)),
            Value::Bool(false) => Ok((CommandType::TurnOff, Value::Null)),
            _ => Err(ApiError::bad_request(format!(
                "entity {entity_id} light.on_off scene value must be boolean"
            ))),
        },
        "light.brightness" => Ok((CommandType::SetBrightness, delta.value.clone())),
        "light.color" => Ok((CommandType::SetColor, delta.value.clone())),
        "light.color_temperature" => Ok((CommandType::SetColorTemperature, delta.value.clone())),
        "lock.state" => Ok((CommandType::SetLock, delta.value.clone())),
        "climate.setpoint" => Ok((CommandType::SetThermostatSetpoint, delta.value.clone())),
        capability_id => Err(ApiError::bad_request(format!(
            "entity {entity_id} desired state for capability `{capability_id}` cannot be mapped"
        ))),
    }
}

fn brightness_value(body: &JsonValue) -> Result<Option<Value>, ApiError> {
    if let Some(value) = json_u64_field(body, "brightness_pct") {
        if value > 100 {
            return Err(ApiError::bad_request(
                "brightness_pct must be between 0 and 100",
            ));
        }
        return Ok(Some(Value::Percentage(value as u8)));
    }

    if let Some(value) = json_u64_field(body, "brightness") {
        if value > 255 {
            return Err(ApiError::bad_request(
                "brightness must be between 0 and 255",
            ));
        }
        let percentage = ((value * 100) + 127) / 255;
        return Ok(Some(Value::Percentage(percentage as u8)));
    }

    Ok(None)
}

fn color_temperature_value(body: &JsonValue) -> Result<Option<Value>, ApiError> {
    for field in ["color_temp_kelvin", "kelvin", "color_temp"] {
        if let Some(value) = json_u64_field(body, field) {
            return Ok(Some(Value::Integer(value as i64)));
        }
    }
    Ok(None)
}

fn color_value(body: &JsonValue) -> Result<Option<Value>, ApiError> {
    let Some(rgb) = body.get("rgb_color") else {
        return Ok(None);
    };
    let values = rgb
        .as_array()
        .ok_or_else(|| ApiError::bad_request("rgb_color must be an array"))?;
    if values.len() != 3 {
        return Err(ApiError::bad_request("rgb_color must have three channels"));
    }
    let mut channels = Vec::new();
    for value in values {
        let channel = value
            .as_u64()
            .ok_or_else(|| ApiError::bad_request("rgb_color channels must be integers"))?;
        if channel > 255 {
            return Err(ApiError::bad_request(
                "rgb_color channels must be between 0 and 255",
            ));
        }
        channels.push(Value::Integer(channel as i64));
    }
    Ok(Some(Value::Array(channels)))
}

fn number_or_integer_field(body: &JsonValue, field: &str) -> Option<Value> {
    body.get(field).and_then(|value| {
        value
            .as_i64()
            .map(Value::Integer)
            .or_else(|| value.as_f64().map(Value::Number))
    })
}

fn collect_string_values(value: &JsonValue, field: &str, output: &mut Vec<String>) {
    let Some(value) = value.get(field) else {
        return;
    };
    match value {
        JsonValue::String(value) => output.push(value.clone()),
        JsonValue::Array(values) => {
            for value in values {
                if let Some(value) = value.as_str() {
                    output.push(value.to_string());
                }
            }
        }
        _ => {}
    }
}

fn json_string_field(value: &JsonValue, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

fn json_u64_field(value: &JsonValue, field: &str) -> Option<u64> {
    value.get(field).and_then(JsonValue::as_u64)
}

fn service_call_json(
    domain: &str,
    service: &str,
    results: &[CommandResult],
    state: &SmartHomePlatformHttpState,
) -> String {
    format!(
        "{{\"domain\":{},\"service\":{},\"result_count\":{},\"results\":[{}],\"states\":{}}}",
        json_string(domain),
        json_string(service),
        results.len(),
        results
            .iter()
            .map(command_result_json)
            .collect::<Vec<_>>()
            .join(","),
        states_json(&state.entities, state.generated_at_ms),
    )
}

fn command_result_json(result: &CommandResult) -> String {
    format!(
        "{{\"command_id\":{},\"status\":{},\"bridge_id\":{},\"correlation_id\":{},\"message\":{}}}",
        json_string(result.command_id.as_str()),
        json_string(command_status_label(result.status)),
        json_string(result.bridge_id.as_str()),
        json_string(result.correlation_id.as_str()),
        result
            .message
            .as_ref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_string()),
    )
}

fn state_delta_json(delta: &StateDelta) -> String {
    format!(
        "{{\"capability_id\":{},\"value\":{}}}",
        json_string(delta.capability_id.as_str()),
        value_json(&delta.value),
    )
}

fn query_string<'a>(request: &'a WebRequest, key: &str) -> Option<&'a str> {
    request.query_params.get(key).map(String::as_str)
}

fn query_u64(request: &WebRequest, key: &str) -> Result<Option<u64>, ApiError> {
    query_string(request, key)
        .map(|value| {
            value
                .parse::<u64>()
                .map_err(|_| ApiError::bad_request(format!("{key} must be an unsigned integer")))
        })
        .transpose()
}

fn query_bool(request: &WebRequest, key: &str) -> Result<Option<bool>, ApiError> {
    query_string(request, key)
        .map(|value| match value {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(ApiError::bad_request(format!("{key} must be a boolean"))),
        })
        .transpose()
}

fn query_limit(request: &WebRequest, default: usize, max: usize) -> Result<usize, ApiError> {
    let Some(value) = query_string(request, "limit") else {
        return Ok(default.min(max));
    };
    let limit = value
        .parse::<usize>()
        .map_err(|_| ApiError::bad_request("limit must be an unsigned integer"))?;
    Ok(limit.min(max))
}

fn command_status_label(status: CommandStatus) -> &'static str {
    match status {
        CommandStatus::Accepted => "accepted",
        CommandStatus::Rejected => "rejected",
        CommandStatus::TimedOut => "timed_out",
        CommandStatus::Failed => "failed",
    }
}

fn command_status_from_label(status: &str) -> Result<CommandStatus, ApiError> {
    match status {
        "accepted" => Ok(CommandStatus::Accepted),
        "rejected" => Ok(CommandStatus::Rejected),
        "timed_out" => Ok(CommandStatus::TimedOut),
        "failed" => Ok(CommandStatus::Failed),
        other => Err(ApiError::bad_request(format!(
            "unsupported command status `{other}`"
        ))),
    }
}

fn authorization_outcome_label(outcome: AuthorizationOutcome) -> &'static str {
    match outcome {
        AuthorizationOutcome::Allowed => "allowed",
        AuthorizationOutcome::Denied => "denied",
    }
}

fn authorization_outcome_from_label(outcome: &str) -> Result<AuthorizationOutcome, ApiError> {
    match outcome {
        "allowed" => Ok(AuthorizationOutcome::Allowed),
        "denied" => Ok(AuthorizationOutcome::Denied),
        other => Err(ApiError::bad_request(format!(
            "unsupported authorization outcome `{other}`"
        ))),
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

fn capability_mode_label(mode: CapabilityMode) -> &'static str {
    match mode {
        CapabilityMode::Observe => "observe",
        CapabilityMode::Command => "command",
        CapabilityMode::ObserveAndCommand => "observe_and_command",
    }
}

fn value_kind_label(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Null => "null",
        ValueKind::Boolean => "boolean",
        ValueKind::Integer => "integer",
        ValueKind::Number => "number",
        ValueKind::Percentage => "percentage",
        ValueKind::Text => "text",
        ValueKind::Object => "object",
        ValueKind::Array => "array",
    }
}

fn bridge_transport_label(transport: BridgeTransport) -> &'static str {
    match transport {
        BridgeTransport::LanHttp => "lan_http",
        BridgeTransport::Mdns => "mdns",
        BridgeTransport::Serial => "serial",
        BridgeTransport::Ble => "ble",
        BridgeTransport::Cloud => "cloud",
        BridgeTransport::LocalProcess => "local_process",
    }
}

fn bridge_transport_from_label(transport: &str) -> Result<BridgeTransport, ApiError> {
    match transport {
        "lan_http" | "lan-http" | "http" => Ok(BridgeTransport::LanHttp),
        "mdns" => Ok(BridgeTransport::Mdns),
        "serial" => Ok(BridgeTransport::Serial),
        "ble" => Ok(BridgeTransport::Ble),
        "cloud" => Ok(BridgeTransport::Cloud),
        "local_process" | "local-process" => Ok(BridgeTransport::LocalProcess),
        other => Err(ApiError::bad_request(format!(
            "unsupported bridge transport `{other}`"
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

fn health_from_label(health: &str) -> Result<Health, ApiError> {
    match health {
        "unknown" => Ok(Health::Unknown),
        "discoverable" => Ok(Health::Discoverable),
        "unpaired" => Ok(Health::Unpaired),
        "online" => Ok(Health::Online),
        "degraded" => Ok(Health::Degraded),
        "offline" => Ok(Health::Offline),
        "auth_failed" | "auth-failed" => Ok(Health::AuthFailed),
        "unsupported" => Ok(Health::Unsupported),
        "removed" => Ok(Health::Removed),
        other => Err(ApiError::bad_request(format!(
            "unsupported health `{other}`"
        ))),
    }
}

fn room_sort_from_label(sort: &str) -> Result<RuntimeRoomSort, ApiError> {
    match sort {
        "room_id" | "id" => Ok(RuntimeRoomSort::RoomId),
        "attention" | "attention_desc" => Ok(RuntimeRoomSort::AttentionDesc),
        "entity_count" | "entity_count_desc" => Ok(RuntimeRoomSort::EntityCountDesc),
        "scene_count" | "scene_count_desc" => Ok(RuntimeRoomSort::SceneCountDesc),
        other => Err(ApiError::bad_request(format!(
            "unsupported room sort `{other}`"
        ))),
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

fn device_event_type_from_label(event_type: &str) -> Result<DeviceEventType, ApiError> {
    match event_type {
        "discovered" => Ok(DeviceEventType::Discovered),
        "updated" => Ok(DeviceEventType::Updated),
        "removed" => Ok(DeviceEventType::Removed),
        "unavailable" => Ok(DeviceEventType::Unavailable),
        "error" => Ok(DeviceEventType::Error),
        "health" => Ok(DeviceEventType::Health),
        other => Err(ApiError::bad_request(format!(
            "unsupported device event type `{other}`"
        ))),
    }
}

fn runtime_error_to_api_error(error: RuntimeError) -> ApiError {
    match error {
        RuntimeError::UnauthorizedCommand { .. } | RuntimeError::UnauthorizedTool { .. } => {
            ApiError::forbidden(error.to_string())
        }
        RuntimeError::UnknownEntity(_) | RuntimeError::UnknownScene(_) => {
            ApiError::not_found(error.to_string())
        }
        RuntimeError::UnsupportedCapability { .. }
        | RuntimeError::ReadOnlyCapability { .. }
        | RuntimeError::UnsupportedDesiredState { .. } => ApiError::bad_request(error.to_string()),
        _ => ApiError::new(500, error.to_string()),
    }
}

fn api_error_response(error: ApiError) -> WebResponse {
    json_error(error.status, error.message)
}

fn json_error(status: u16, message: impl AsRef<str>) -> WebResponse {
    WebResponse::new(
        status,
        format!("{{\"error\":{}}}", json_string(message.as_ref())).into_bytes(),
    )
    .with_content_type("application/json")
}

fn default_event_types() -> Vec<String> {
    sorted_unique_strings([
        "call_service",
        "command_result",
        "state_changed",
        "state_expired",
    ])
}

fn sorted_unique_strings(values: impl IntoIterator<Item = impl Into<String>>) -> Vec<String> {
    let mut values = values.into_iter().map(Into::into).collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn services_for_capability(domain: &str, capability: &Capability) -> Vec<&'static str> {
    match capability.capability_id.as_str() {
        "light.on_off" => vec!["turn_on", "turn_off"],
        "light.brightness" => vec!["set_brightness"],
        "light.color" => vec!["set_color"],
        "light.color_temperature" => vec!["set_color_temperature"],
        "lock.state" => vec!["lock", "unlock"],
        "climate.setpoint" => vec!["set_temperature"],
        "scene.recall" => vec!["turn_on"],
        _ if domain == "input" => vec!["set_value"],
        _ => vec!["set_value"],
    }
}

fn capability_allows_command(capability: &Capability) -> bool {
    matches!(
        capability.mode,
        CapabilityMode::Command | CapabilityMode::ObserveAndCommand
    )
}

fn entity_domain(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Light => "light",
        EntityKind::LightGroup => "light",
        EntityKind::Switch => "switch",
        EntityKind::Sensor => "sensor",
        EntityKind::Lock => "lock",
        EntityKind::Thermostat => "climate",
        EntityKind::Scene => "scene",
        EntityKind::Input => "input",
        EntityKind::BridgeHealth => "binary_sensor",
        EntityKind::NetworkDiagnostic => "diagnostic",
        EntityKind::Unknown => "unknown",
    }
}

fn entity_matches_external_id(entity: &Entity, target: &str) -> bool {
    entity.entity_id.as_str() == target || home_assistant_entity_id(entity) == target
}

fn scene_matches_external_id(scene: &Scene, target: &str) -> bool {
    scene.scene_id.as_str() == target || home_assistant_scene_id(scene) == target
}

fn home_assistant_entity_id(entity: &Entity) -> String {
    format!(
        "{}.{}",
        entity_domain(entity.kind),
        object_id(entity.entity_id.as_str())
    )
}

fn home_assistant_entity_id_for(entity_id: &EntityId) -> String {
    format!("entity.{}", object_id(entity_id.as_str()))
}

fn home_assistant_scene_id(scene: &Scene) -> String {
    format!("scene.{}", object_id(scene.scene_id.as_str()))
}

fn object_id(value: &str) -> String {
    let mut object_id = String::new();
    let mut previous_was_separator = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            object_id.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if !previous_was_separator {
            object_id.push('_');
            previous_was_separator = true;
        }
    }
    let object_id = object_id.trim_matches('_');
    if object_id.is_empty() {
        "unnamed".to_string()
    } else {
        object_id.to_string()
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

fn entity_kind_from_label(kind: &str) -> Result<EntityKind, ApiError> {
    match kind {
        "light" => Ok(EntityKind::Light),
        "light_group" => Ok(EntityKind::LightGroup),
        "switch" => Ok(EntityKind::Switch),
        "sensor" => Ok(EntityKind::Sensor),
        "lock" => Ok(EntityKind::Lock),
        "thermostat" | "climate" => Ok(EntityKind::Thermostat),
        "scene" => Ok(EntityKind::Scene),
        "input" => Ok(EntityKind::Input),
        "bridge_health" | "binary_sensor" => Ok(EntityKind::BridgeHealth),
        "network_diagnostic" | "diagnostic" => Ok(EntityKind::NetworkDiagnostic),
        "unknown" => Ok(EntityKind::Unknown),
        other => Err(ApiError::bad_request(format!(
            "unsupported entity kind `{other}`"
        ))),
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

fn value_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Integer(value) => value.to_string(),
        Value::Number(value) if value.is_finite() => value.to_string(),
        Value::Number(_) => "null".to_string(),
        Value::Percentage(value) => value.to_string(),
        Value::Text(value) => json_string(value),
        Value::Object(fields) => format!(
            "{{{}}}",
            fields
                .iter()
                .map(|(key, value)| format!("{}:{}", json_string(key), value_json(value)))
                .collect::<Vec<_>>()
                .join(",")
        ),
        Value::Array(values) => format!(
            "[{}]",
            values.iter().map(value_json).collect::<Vec<_>>().join(",")
        ),
    }
}

fn json_string_array(values: &[String]) -> String {
    values.iter().map(json_string).collect::<Vec<_>>().join(",")
}

fn json_id_array<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    values
        .into_iter()
        .map(json_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn optional_u64_json(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn optional_f64_json(value: Option<f64>) -> String {
    match value {
        Some(value) if value.is_finite() => value.to_string(),
        _ => "null".to_string(),
    }
}

fn optional_str_json(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_string())
}

fn json_string(value: impl AsRef<str>) -> String {
    let mut escaped = String::from("\"");
    for ch in value.as_ref().chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped.push('"');
    escaped
}

fn push_unique_string(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use embeddable_http_server::{HttpRequest, HttpServerOptions};
    use http_core::{Header, HttpVersion, RequestHead};
    use smart_home_core::{BridgeId, DeviceId, EventId};
    use smart_home_testkit::hue_lighting_runtime;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{SocketAddr, TcpStream};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use tcp_runtime::{ConnectionId, TcpConnectionInfo};
    use web_core::WebServer;

    fn request(method: &str, target: &str) -> HttpRequest {
        request_with_body(method, target, "")
    }

    fn request_with_body(method: &str, target: &str, body: &str) -> HttpRequest {
        let mut headers = vec![Header {
            name: "Host".to_string(),
            value: "localhost".to_string(),
        }];
        if !body.is_empty() {
            headers.push(Header {
                name: "Content-Type".to_string(),
                value: "application/json".to_string(),
            });
            headers.push(Header {
                name: "Content-Length".to_string(),
                value: body.len().to_string(),
            });
        }

        HttpRequest {
            connection: TcpConnectionInfo {
                id: ConnectionId(0),
                peer_addr: SocketAddr::from(([127, 0, 0, 1], 10_000)),
                local_addr: SocketAddr::from(([127, 0, 0, 1], 8123)),
            },
            head: RequestHead {
                method: method.to_string(),
                target: target.to_string(),
                version: HttpVersion { major: 1, minor: 1 },
                headers,
            },
            body: body.as_bytes().to_vec(),
        }
    }

    fn response_body(response: web_core::WebResponse) -> String {
        String::from_utf8(response.body).expect("json response is utf8")
    }

    fn http_get(port: u16, path: &str) -> (u16, String) {
        http_request(port, "GET", path, "")
    }

    fn http_post(port: u16, path: &str, body: &str) -> (u16, String) {
        http_request(port, "POST", path, body)
    }

    fn http_request(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");

        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(request.as_bytes()).expect("write request");

        let mut reader = BufReader::new(&stream);
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .expect("read status line");
        let status = status_line
            .split_whitespace()
            .nth(1)
            .expect("status code")
            .parse()
            .expect("parse status code");

        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read header");
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if trimmed.to_ascii_lowercase().starts_with("content-length:") {
                content_length = trimmed
                    .split_once(':')
                    .map(|(_, value)| value.trim().parse().unwrap_or(0))
                    .unwrap_or(0);
            }
        }

        let mut body = vec![0; content_length];
        reader.read_exact(&mut body).expect("read response body");
        (
            status,
            String::from_utf8(body).expect("json response is utf8"),
        )
    }

    fn start_server(app: WebApp) -> (u16, tcp_runtime::StopHandle) {
        let app = Arc::new(app);

        #[cfg(any(
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd",
            target_os = "dragonfly"
        ))]
        let mut server = WebServer::bind_kqueue(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
        .expect("bind kqueue");

        #[cfg(target_os = "linux")]
        let mut server = WebServer::bind_epoll(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
        .expect("bind epoll");

        #[cfg(target_os = "windows")]
        let mut server = WebServer::bind_windows(
            "127.0.0.1:0",
            HttpServerOptions::default(),
            Arc::clone(&app),
        )
        .expect("bind windows");

        let port = server.local_addr().port();
        let stop = server.stop_handle();
        thread::spawn(move || {
            let _ = server.serve();
        });
        thread::sleep(Duration::from_millis(20));
        (port, stop)
    }

    fn fixture_state() -> SmartHomePlatformHttpState {
        let runtime = hue_lighting_runtime();
        SmartHomePlatformHttpState::from_runtime(
            &runtime,
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
            ["state_changed", "call_service"],
            5_000,
        )
    }

    fn fixture_runtime(grant_access: bool) -> SmartHomePlatformHttpRuntime {
        let runtime = SmartHomePlatformHttpRuntime::new(
            hue_lighting_runtime(),
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
        )
        .with_event_types(["state_changed", "call_service", "command_result"])
        .with_now_ms(5_000);

        if grant_access {
            runtime.grant_local_full_access("test", 1_000)
        } else {
            runtime
        }
    }

    fn fixture_runtime_with_desired_state() -> SmartHomePlatformHttpRuntime {
        let mut runtime = hue_lighting_runtime();
        runtime
            .upsert_desired_state(
                DesiredEntityState::new(
                    EntityId::trusted("entity-light-1"),
                    vec![StateDelta {
                        capability_id: CapabilityId::trusted("light.on_off"),
                        value: Value::Bool(true),
                    }],
                )
                .requested_by("agent:chief-of-staff")
                .with_command_timeout(2_500),
            )
            .expect("fixture desired state should validate");

        SmartHomePlatformHttpRuntime::new(
            runtime,
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
        )
        .with_now_ms(5_000)
    }

    fn fixture_runtime_with_state_history() -> SmartHomePlatformHttpRuntime {
        let mut runtime = hue_lighting_runtime();
        runtime
            .apply_device_event(DeviceEvent {
                event_id: EventId::trusted("event-light-1-on"),
                bridge_id: BridgeId::trusted("bridge-1"),
                device_id: Some(DeviceId::trusted("device-1")),
                entity_id: Some(EntityId::trusted("entity-light-1")),
                observed_at_ms: 2_000,
                received_at_ms: 2_010,
                event_type: DeviceEventType::Updated,
                state_delta: Some(StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(true),
                }),
                raw_ref: Some("event-log://fixture/light/1".to_string()),
                correlation_id: None,
                metadata: Vec::new(),
            })
            .expect("fixture event should validate");

        SmartHomePlatformHttpRuntime::new(
            runtime,
            SmartHomePlatformHttpConfig::new("Codex Home").with_time_zone("America/Los_Angeles"),
        )
        .with_now_ms(5_000)
    }

    #[test]
    fn platform_http_summary_counts_runtime_snapshot_shape() {
        let state = fixture_state();
        let summary = state.summary();

        assert_eq!(summary.state_count, 2);
        assert_eq!(summary.scene_count, 1);
        assert_eq!(summary.unknown_state_count, 2);
        assert_eq!(summary.event_type_count, 2);
        assert!(summary.service_count >= 4);
    }

    #[test]
    fn home_assistant_web_app_serves_config_states_services_and_events() {
        let state = fixture_state();
        let app = home_assistant_web_app(state);

        let root = response_body(app.handle(request("GET", "/api/")).into());
        assert_eq!(root, r#"{"message":"API running."}"#);

        let config = response_body(app.handle(request("GET", "/api/config")).into());
        assert!(config.contains(r#""location_name":"Codex Home""#));
        assert!(config.contains(r#""state_count":2"#));

        let states = response_body(app.handle(request("GET", "/api/states")).into());
        assert!(states.contains(r#""entity_id":"entity-light-1""#));
        assert!(states.contains(r#""domain":"light""#));
        assert!(states.contains(r#""state":"unknown""#));

        let one_state = response_body(
            app.handle(request("GET", "/api/states/entity-light-1"))
                .into(),
        );
        assert!(one_state.contains(r#""friendly_name":"Kitchen Light""#));
        assert!(one_state.contains(r#""light.on_off""#));
        assert!(one_state.contains(r#""light.brightness""#));
        assert!(one_state.contains(r#""light.color_temperature""#));

        let services = response_body(app.handle(request("GET", "/api/services")).into());
        assert!(services.contains(r#""domain":"light""#));
        assert!(services.contains(r#""service":"turn_on""#));
        assert!(services.contains(r#""service":"set_brightness""#));
        assert!(services.contains(r#""domain":"scene""#));

        let events = response_body(app.handle(request("GET", "/api/events")).into());
        assert!(events.contains(r#""event":"call_service""#));
        assert!(events.contains(r#""event":"state_changed""#));
    }

    #[test]
    fn home_assistant_web_app_serves_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_web_app(fixture_state()));
        let (status, body) = http_get(port, "/api/states/entity-light-1");
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""domain":"light""#));
        assert!(body.contains(r#""friendly_name":"Kitchen Light""#));
    }

    #[test]
    fn runtime_web_app_dispatches_authorized_light_service_calls() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"light.entity_light_1","brightness_pct":75,"idempotency_key":"ha:turn-on:kitchen"}"#,
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""domain":"light""#));
        assert!(body.contains(r#""service":"turn_on""#));
        assert!(body.contains(r#""result_count":2"#));
        assert!(body.contains(r#""status":"accepted""#));

        let state = response_body(
            app.handle(request("GET", "/api/states/light.entity_light_1"))
                .into(),
        );
        assert!(state.contains(r#""confidence":"optimistic""#));
        assert!(state.contains(r#""light.brightness":75"#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_audit_routes() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));

        let snapshot = response_body(app.handle(request("GET", "/api/smart_home/runtime")).into());
        assert!(snapshot.contains(r#""registry":{"bridges":1"#));
        assert!(snapshot.contains(r#""event_bus":{"subscription_count":0"#));
        assert!(snapshot.contains(r#""pending_work":{"total":"#));
        assert!(snapshot.contains(r#""state_refresh_target_count":2"#));

        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"entity-light-1"}"#,
            ))
            .into();
        assert_eq!(response.status, 200);

        let command_results = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/command_results?status=accepted&limit=5",
            ))
            .into(),
        );
        assert!(command_results.contains(r#""total_results":1"#));
        assert!(command_results.contains(r#""status":"accepted""#));
        assert!(command_results.contains(r#""sequence":0"#));

        let events = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/events?kind=commands&sort=desc&limit=5",
            ))
            .into(),
        );
        assert!(events.contains(r#""command_results":1"#));
        assert!(events.contains(r#""kind":"command_result""#));

        let decisions = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/authorization_decisions?outcome=allowed&limit=5",
            ))
            .into(),
        );
        assert!(decisions.contains(r#""allowed_decisions":2"#));
        assert!(decisions.contains(r#""principal_id":"agent:home-assistant-local-api""#));
        assert!(decisions.contains(r#""kind":"command""#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_overview() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"entity-light-1"}"#,
            ))
            .into();
        assert_eq!(response.status, 200);

        let dashboard = response_body(
            app.handle(request("GET", "/api/smart_home/dashboard"))
                .into(),
        );

        assert!(dashboard.contains(r#""generated_at_ms":5000"#));
        assert!(dashboard.contains(r#""config":{"location_name":"Codex Home""#));
        assert!(dashboard.contains(r#""summary":{"state_count":2"#));
        assert!(dashboard.contains(r#""bridge_count":1"#));
        assert!(dashboard.contains(r#""device_count":1"#));
        assert!(dashboard.contains(r#""entity_count":2"#));
        assert!(dashboard.contains(r#""room_count":1"#));
        assert!(dashboard.contains(r#""scene_count":1"#));
        assert!(dashboard.contains(r#""pending_work_total":"#));
        assert!(dashboard.contains(r#""has_state_gaps":true"#));
        assert!(dashboard.contains(r#""runtime":{"generated_at_ms":5000"#));
        assert!(dashboard.contains(r#""topology":{"bridges":1"#));
        assert!(dashboard.contains(r#""bridges":{"summary":{"total_bridges":1"#));
        assert!(dashboard.contains(r#""devices":{"summary":{"total_devices":1"#));
        assert!(dashboard.contains(r#""entities":{"summary":{"total_entities":2"#));
        assert!(dashboard.contains(r#""rooms":{"summary":{"total_rooms":1"#));
        assert!(dashboard.contains(r#""desired_states":{"summary":{"total_desired_states":0"#));
        assert!(dashboard.contains(r#""events":{"summary":{"total_events":1"#));
        assert!(dashboard.contains(r#""command_results":{"summary":{"total_results":1"#));
        assert!(dashboard.contains(r#""authorization_decisions":{"summary":{"total_decisions":2"#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_entity_registry() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/entities?domain=light&capability_id=light.brightness&commandable=true",
            ))
            .into(),
        );

        assert!(body.contains(r#""total_entities":1"#));
        assert!(body.contains(r#""commandable_entities":1"#));
        assert!(body.contains(r#""capability_count":3"#));
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""device_id":"device-1""#));
        assert!(body.contains(r#""bridge_id":"bridge-1""#));
        assert!(body.contains(r#""room_id":"kitchen""#));
        assert!(body.contains(r#""manufacturer":"Signify""#));
        assert!(body.contains(r#""model":"Hue bulb""#));
        assert!(body.contains(r#""capability_id":"light.brightness""#));
        assert!(body.contains(r#""mode":"observe_and_command""#));
        assert!(body.contains(r#""value_kind":"percentage""#));
        assert!(body.contains(r#""min":0"#));
        assert!(body.contains(r#""max":100"#));

        let one_response: web_core::WebResponse = app
            .handle(request(
                "GET",
                "/api/smart_home/entities/light.entity_light_1",
            ))
            .into();
        let one_body = response_body(one_response.clone());
        assert_eq!(one_response.status, 200);
        assert!(one_body.contains(r#""name":"Kitchen Light""#));
        assert!(one_body.contains(r#""domain":"light""#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_room_summaries() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/rooms?room_id=kitchen&state_gaps_only=true&sort=scene_count",
            ))
            .into(),
        );

        assert!(body.contains(r#""total_rooms":1"#));
        assert!(body.contains(r#""state_gap_rooms":1"#));
        assert!(body.contains(r#""scene_rooms":1"#));
        assert!(body.contains(r#""topology_unique_rooms":1"#));
        assert!(body.contains(r#""devices_with_room":1"#));
        assert!(body.contains(r#""room_id":"kitchen""#));
        assert!(body.contains(r#""device_count":1"#));
        assert!(body.contains(r#""online_devices":1"#));
        assert!(body.contains(r#""entity_count":2"#));
        assert!(body.contains(r#""commandable_entities":1"#));
        assert!(body.contains(r#""entities_without_state":2"#));
        assert!(body.contains(r#""state_gap_count":2"#));
        assert!(body.contains(r#""scene_count":1"#));
        assert!(body.contains(r#""scene_action_count":1"#));
        assert!(body.contains(r#""has_state_gaps":true"#));
        assert!(body.contains(r#""has_scene_actions":true"#));
    }

    #[test]
    fn runtime_web_app_serves_dashboard_ready_device_and_bridge_registry() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let devices = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/devices?bridge_id=bridge-1&room_id=kitchen&health=online",
            ))
            .into(),
        );

        assert!(devices.contains(r#""total_devices":1"#));
        assert!(devices.contains(r#""online_devices":1"#));
        assert!(devices.contains(r#""total_entities":2"#));
        assert!(devices.contains(r#""commandable_entities":1"#));
        assert!(devices.contains(r#""stale_entities":2"#));
        assert!(devices.contains(r#""capability_count":4"#));
        assert!(devices.contains(r#""device_id":"device-1""#));
        assert!(devices.contains(r#""bridge_id":"bridge-1""#));
        assert!(devices.contains(r#""name":"Kitchen""#));
        assert!(devices.contains(r#""manufacturer":"Signify""#));
        assert!(devices.contains(r#""model":"Hue bulb""#));
        assert!(devices.contains(r#""serial":"device-native-1""#));
        assert!(devices.contains(r#""firmware_version":"1.0.0""#));
        assert!(devices.contains(r#""room_id":"kitchen""#));
        assert!(devices.contains(r#""health":"online""#));
        assert!(devices.contains(r#""entity_ids":["entity-light-1","entity-sensor-1"]"#));
        assert!(devices.contains(
            r#""home_assistant_entity_ids":["light.entity_light_1","sensor.entity_sensor_1"]"#
        ));
        assert!(devices.contains(r#""capability_ids":["light.on_off","light.brightness","light.color_temperature","sensor.occupancy"]"#));

        let device_response: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/devices/device-1"))
            .into();
        let device = response_body(device_response.clone());
        assert_eq!(device_response.status, 200);
        assert!(device.contains(r#""device_id":"device-1""#));
        assert!(device.contains(r#""entity_count":2"#));

        let bridges = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/bridges?integration_id=hue&transport=lan_http&health=online",
            ))
            .into(),
        );

        assert!(bridges.contains(r#""total_bridges":1"#));
        assert!(bridges.contains(r#""online_bridges":1"#));
        assert!(bridges.contains(r#""total_devices":1"#));
        assert!(bridges.contains(r#""room_count":1"#));
        assert!(bridges.contains(r#""bridge_id":"bridge-1""#));
        assert!(bridges.contains(r#""integration_id":"hue""#));
        assert!(bridges.contains(r#""transport":"lan_http""#));
        assert!(bridges.contains(r#""address":"https://192.0.2.10""#));
        assert!(bridges.contains(r#""hardware_model":"BSB002""#));
        assert!(bridges.contains(r#""firmware_version":"1.66.1960062030""#));
        assert!(bridges.contains(r#""last_seen_at_ms":1000"#));
        assert!(bridges.contains(r#""device_count":1"#));
        assert!(bridges.contains(r#""entity_count":2"#));
        assert!(bridges.contains(r#""room_ids":["kitchen"]"#));
        assert!(bridges.contains(r#""device_ids":["device-1"]"#));

        let bridge_response: web_core::WebResponse = app
            .handle(request("GET", "/api/smart_home/bridges/bridge-1"))
            .into();
        let bridge = response_body(bridge_response.clone());
        assert_eq!(bridge_response.status, 200);
        assert!(bridge.contains(r#""bridge_id":"bridge-1""#));
        assert!(bridge.contains(r#""commandable_entities":1"#));
    }

    #[test]
    fn runtime_web_app_serves_desired_state_targets() {
        let app = home_assistant_runtime_web_app(fixture_runtime_with_desired_state());
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/desired_states?entity_id=light.entity_light_1",
            ))
            .into(),
        );

        assert!(body.contains(r#""total_desired_states":1"#));
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""requested_by":"agent:chief-of-staff""#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));
    }

    #[test]
    fn runtime_web_app_sets_desired_state_through_runtime_authorization() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/desired_states/light.entity_light_1",
                r#"{"desired_state":{"light.on_off":true,"light.brightness":80},"requested_by":"agent:dashboard","command_timeout_ms":3000}"#,
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""replaced":false"#));
        assert!(body.contains(r#""requested_by":"agent:dashboard""#));
        assert!(body.contains(r#""command_timeout_ms":3000"#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));
        assert!(body.contains(r#""capability_id":"light.brightness""#));

        let desired_states = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/desired_states?entity_id=light.entity_light_1",
            ))
            .into(),
        );
        assert!(desired_states.contains(r#""total_desired_states":1"#));
        assert!(desired_states.contains(r#""total_desired_capabilities":2"#));
    }

    #[test]
    fn runtime_web_app_posts_home_assistant_state_as_desired_state() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/states/light.entity_light_1",
                r#"{"state":"on","attributes":{"brightness":191,"color_temp_kelvin":2700}}"#,
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""requested_by":"agent:home-assistant-local-api""#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));
        assert!(body.contains(r#""capability_id":"light.brightness""#));
        assert!(body.contains(r#""capability_id":"light.color_temperature""#));
        assert!(body.contains(r#""value":75"#));
        assert!(body.contains(r#""value":2700"#));
    }

    #[test]
    fn runtime_web_app_clears_desired_state_through_runtime_authorization() {
        let app = home_assistant_runtime_web_app(
            fixture_runtime_with_desired_state().grant_local_full_access("test", 1_000),
        );
        let response: web_core::WebResponse = app
            .handle(request(
                "DELETE",
                "/api/smart_home/desired_states/light.entity_light_1",
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""entity_id":"entity-light-1""#));
        assert!(body.contains(r#""removed":true"#));
        assert!(body.contains(r#""removed_desired_state":{"entity_id":"entity-light-1""#));
        assert!(body.contains(r#""total_desired_states":0"#));
    }

    #[test]
    fn runtime_web_app_rejects_desired_state_without_runtime_grants() {
        let app = home_assistant_runtime_web_app(fixture_runtime(false));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/smart_home/desired_states/entity-light-1",
                r#"{"desired_state":{"light.on_off":true}}"#,
            ))
            .into();

        assert_eq!(response.status, 403);
        assert!(response_body(response).contains("not authorized"));
    }

    #[test]
    fn runtime_web_app_serves_state_history_with_alias_filters() {
        let app = home_assistant_runtime_web_app(fixture_runtime_with_state_history());
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/smart_home/state_history?entity_id=light.entity_light_1&event_type=updated",
            ))
            .into(),
        );

        assert!(body.contains(r#""total_events":1"#));
        assert!(body.contains(r#""entity_count":1"#));
        assert!(body.contains(r#""state_delta_count":1"#));
        assert!(body.contains(r#""home_assistant_entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""event_id":"event-light-1-on""#));
        assert!(body.contains(r#""event_type":"updated""#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));
        assert!(body.contains(r#""value":true"#));
    }

    #[test]
    fn runtime_web_app_serves_home_assistant_history_period_route() {
        let app = home_assistant_runtime_web_app(fixture_runtime_with_state_history());
        let body = response_body(
            app.handle(request(
                "GET",
                "/api/history/period?filter_entity_id=light.entity_light_1",
            ))
            .into(),
        );

        assert!(body.starts_with("[["));
        assert!(body.contains(r#""entity_id":"light.entity_light_1""#));
        assert!(body.contains(r#""state":true"#));
        assert!(body.contains(r#""canonical_entity_id":"entity-light-1""#));
        assert!(body.contains(r#""event_id":"event-light-1-on""#));
        assert!(body.contains(r#""capability_id":"light.on_off""#));

        let period_body = response_body(
            app.handle(request(
                "GET",
                "/api/history/period/2000?filter_entity_id=light.entity_light_1",
            ))
            .into(),
        );
        assert_eq!(period_body, body);
    }

    #[test]
    fn runtime_web_app_rejects_service_calls_without_runtime_grants() {
        let app = home_assistant_runtime_web_app(fixture_runtime(false));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/light/turn_on",
                r#"{"entity_id":"entity-light-1"}"#,
            ))
            .into();

        assert_eq!(response.status, 403);
        assert!(response_body(response).contains("not authorized"));
    }

    #[test]
    fn runtime_web_app_expands_scene_turn_on_into_commands() {
        let app = home_assistant_runtime_web_app(fixture_runtime(true));
        let response: web_core::WebResponse = app
            .handle(request_with_body(
                "POST",
                "/api/services/scene/turn_on",
                r#"{"entity_id":"scene.scene_kitchen_bright"}"#,
            ))
            .into();

        let body = response_body(response.clone());
        assert_eq!(response.status, 200);
        assert!(body.contains(r#""domain":"scene""#));
        assert!(body.contains(r#""result_count":2"#));
        assert!(body.contains(r#""status":"accepted""#));
    }

    #[test]
    fn runtime_web_app_serves_post_services_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_runtime_web_app(fixture_runtime(true)));
        let (status, body) = http_post(
            port,
            "/api/services/light/set_brightness",
            r#"{"entity_id":"entity-light-1","brightness":128}"#,
        );
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains(r#""service":"set_brightness""#));
        assert!(body.contains(r#""result_count":1"#));
        assert!(body.contains(r#""status":"accepted""#));
    }

    #[test]
    fn runtime_web_app_serves_runtime_snapshot_over_repo_http_server() {
        let (port, stop) = start_server(home_assistant_runtime_web_app(fixture_runtime(true)));
        let (status, body) = http_get(port, "/api/smart_home/runtime");
        stop.stop();

        assert_eq!(status, 200);
        assert!(body.contains(r#""registry":{"bridges":1"#));
        assert!(body.contains(r#""desired_state":{"target_count":0"#));
    }

    #[test]
    fn home_assistant_web_app_reports_missing_state_as_json_404() {
        let app = home_assistant_web_app(fixture_state());
        let response: web_core::WebResponse = app
            .handle(request("GET", "/api/states/missing.entity"))
            .into();

        assert_eq!(response.status, 404);
        assert_eq!(response_body(response), r#"{"error":"entity not found"}"#);
    }

    #[test]
    fn value_json_escapes_strings_and_projects_nested_values() {
        let value = Value::Object(vec![
            ("name".to_string(), Value::Text("Kitchen \"A\"".to_string())),
            (
                "levels".to_string(),
                Value::Array(vec![Value::Percentage(50)]),
            ),
        ]);

        assert_eq!(
            value_json(&value),
            r#"{"name":"Kitchen \"A\"","levels":[50]}"#
        );
    }
}
