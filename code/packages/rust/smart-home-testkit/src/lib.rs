//! Deterministic smart-home fixtures and fake streams for D23 tests.
//!
//! This crate is pure data. It gives runtime and integration packages reusable
//! fixtures without opening sockets, touching radios, reading files, or calling
//! cloud APIs.

#![forbid(unsafe_code)]

use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CommandId, CommandResult,
    CommandStatus, CommandType, CorrelationId, Device, DeviceCommand, DeviceEvent, DeviceEventType,
    DeviceId, Entity, EntityId, EntityKind, EventId, Health, IntegrationId, Metadata,
    ProtocolFamily, ProtocolIdentifier, StateConfidence, StateDelta, StateSnapshot, StateSource,
    Value,
};
use smart_home_event_streams::{
    EventStreamCheckpoint, EventStreamRestartReason, EventStreamSpec, EventStreamState,
    EventStreamStatus,
};
use smart_home_local_http::{LocalHttpMethod, LocalHttpRequestPlan};
use smart_home_registry::{InMemorySmartHomeRegistry, RegistryError};
use smart_home_runtime::{BridgeHealthReport, RuntimeError, SmartHomeRuntime};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureClock {
    now_ms: u64,
}

impl FixtureClock {
    pub fn new(start_ms: u64) -> Self {
        Self { now_ms: start_ms }
    }

    pub fn now_ms(self) -> u64 {
        self.now_ms
    }

    pub fn advance_ms(&mut self, delta_ms: u64) -> u64 {
        self.now_ms = self.now_ms.saturating_add(delta_ms);
        self.now_ms
    }
}

impl Default for FixtureClock {
    fn default() -> Self {
        Self::new(1_000)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SmartHomeFixture {
    pub bridge: Bridge,
    pub device: Device,
    pub light: Entity,
    pub sensor: Entity,
}

impl SmartHomeFixture {
    pub fn hue_lighting() -> Self {
        let bridge = hue_bridge("bridge-1", "001788fffeabcdef");
        let device = hue_device("device-1", &bridge.bridge_id, "device-native-1");
        let light = light_entity("entity-light-1", &device.device_id);
        let sensor = occupancy_sensor_entity("entity-sensor-1", &device.device_id);

        Self {
            bridge,
            device,
            light,
            sensor,
        }
    }

    pub fn entities(&self) -> [&Entity; 2] {
        [&self.light, &self.sensor]
    }

    pub fn install_in_registry(
        &self,
        registry: &mut InMemorySmartHomeRegistry,
    ) -> Result<(), RegistryError> {
        install_fixture_in_registry(registry, self)
    }

    pub fn to_registry(&self) -> Result<InMemorySmartHomeRegistry, RegistryError> {
        registry_with_fixture(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptedEvent {
    Event(Box<DeviceEvent>),
    Disconnect { reason: String, at_ms: u64 },
    Gap { missing_events: u32, at_ms: u64 },
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FakeEventStream {
    events: VecDeque<ScriptedEvent>,
}

impl FakeEventStream {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_event(mut self, event: DeviceEvent) -> Self {
        self.events.push_back(ScriptedEvent::Event(Box::new(event)));
        self
    }

    pub fn push_disconnect(mut self, reason: impl Into<String>, at_ms: u64) -> Self {
        self.events.push_back(ScriptedEvent::Disconnect {
            reason: reason.into(),
            at_ms,
        });
        self
    }

    pub fn push_gap(mut self, missing_events: u32, at_ms: u64) -> Self {
        self.events.push_back(ScriptedEvent::Gap {
            missing_events,
            at_ms,
        });
        self
    }

    pub fn next_step(&mut self) -> Option<ScriptedEvent> {
        self.events.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FakeEventStreamStepReport {
    pub step: ScriptedEvent,
    pub checkpoint: EventStreamCheckpoint,
    pub status: EventStreamStatus,
    pub restart_reason: Option<EventStreamRestartReason>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FakeEventStreamDriver {
    stream: FakeEventStream,
    state: EventStreamState,
}

impl FakeEventStreamDriver {
    pub fn new(spec: EventStreamSpec, stream: FakeEventStream, connected_at_ms: u64) -> Self {
        let mut state = EventStreamState::new(spec, connected_at_ms);
        state.mark_connected(connected_at_ms);
        Self { stream, state }
    }

    pub fn hue_sse(
        fixture: &SmartHomeFixture,
        stream: FakeEventStream,
        connected_at_ms: u64,
    ) -> Self {
        Self::new(hue_sse_stream_spec(fixture), stream, connected_at_ms)
    }

    pub fn state(&self) -> &EventStreamState {
        &self.state
    }

    pub fn stream_len(&self) -> usize {
        self.stream.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stream.is_empty()
    }

    pub fn next_step(&mut self) -> Option<FakeEventStreamStepReport> {
        let step = self.stream.next_step()?;
        let observed_at_ms = scripted_event_observed_at_ms(&step);

        match &step {
            ScriptedEvent::Event(event) => {
                self.state.record_event(
                    event.event_id.clone(),
                    Some(format!("fixture:{}", event.event_id.as_str())),
                    event.observed_at_ms,
                );
            }
            ScriptedEvent::Disconnect { at_ms, .. } => {
                self.state.mark_disconnected(*at_ms);
            }
            ScriptedEvent::Gap {
                missing_events,
                at_ms,
            } => {
                self.state.record_gap(*missing_events, *at_ms);
            }
        }

        let restart_reason = self
            .state
            .restart_plan_at(observed_at_ms)
            .map(|plan| plan.reason);

        Some(FakeEventStreamStepReport {
            step,
            checkpoint: self.state.checkpoint(),
            status: self.state.status,
            restart_reason,
        })
    }

    pub fn next_runtime_step(
        &mut self,
        runtime: &mut SmartHomeRuntime,
    ) -> Result<Option<FakeEventStreamStepReport>, RuntimeError> {
        let Some(report) = self.next_step() else {
            return Ok(None);
        };

        match &report.step {
            ScriptedEvent::Event(event) => runtime.apply_device_event((**event).clone())?,
            ScriptedEvent::Disconnect { reason, at_ms } => {
                runtime.apply_bridge_health(BridgeHealthReport {
                    event_id: EventId::trusted(format!(
                        "fixture.disconnect:{}:{at_ms}",
                        self.state.spec.bridge_id.as_str()
                    )),
                    bridge_id: self.state.spec.bridge_id.clone(),
                    health: Health::Degraded,
                    observed_at_ms: *at_ms,
                    received_at_ms: *at_ms,
                    metadata: vec![
                        Metadata::new("fixture", "event_stream_disconnect"),
                        Metadata::new("fixture.disconnect.reason", reason),
                    ],
                })?;
            }
            ScriptedEvent::Gap { .. } => {}
        }

        Ok(Some(report))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FakeCommandBus {
    commands: VecDeque<DeviceCommand>,
    results: VecDeque<CommandResult>,
}

impl FakeCommandBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_command(mut self, command: DeviceCommand) -> Self {
        self.commands.push_back(command);
        self
    }

    pub fn push_result(mut self, result: CommandResult) -> Self {
        self.results.push_back(result);
        self
    }

    pub fn record_command(&mut self, command: DeviceCommand) {
        self.commands.push_back(command);
    }

    pub fn record_result(&mut self, result: CommandResult) {
        self.results.push_back(result);
    }

    pub fn next_command(&mut self) -> Option<DeviceCommand> {
        self.commands.pop_front()
    }

    pub fn next_result(&mut self) -> Option<CommandResult> {
        self.results.pop_front()
    }

    pub fn pending_command_count(&self) -> usize {
        self.commands.len()
    }

    pub fn pending_result_count(&self) -> usize {
        self.results.len()
    }

    pub fn is_idle(&self) -> bool {
        self.commands.is_empty() && self.results.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedMqttPublication {
    pub topic: String,
    pub payload: Vec<u8>,
    pub retained: bool,
    pub observed_at_ms: u64,
    pub metadata: Vec<Metadata>,
}

impl ScriptedMqttPublication {
    pub fn new(topic: impl Into<String>, payload: impl Into<Vec<u8>>, observed_at_ms: u64) -> Self {
        Self {
            topic: topic.into(),
            payload: payload.into(),
            retained: false,
            observed_at_ms,
            metadata: Vec::new(),
        }
    }

    pub fn retained(mut self) -> Self {
        self.retained = true;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn payload_utf8(&self) -> Option<&str> {
        std::str::from_utf8(&self.payload).ok()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MqttPublicationSort {
    #[default]
    OriginalOrder,
    Topic,
    ObservedAtAsc,
    ObservedAtDesc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttPublicationQuery {
    pub topic: Option<String>,
    pub topic_prefix: Option<String>,
    pub retained: Option<bool>,
    pub observed_after_ms: Option<u64>,
    pub metadata: Vec<Metadata>,
    pub sort: MqttPublicationSort,
    pub limit: Option<usize>,
}

impl Default for MqttPublicationQuery {
    fn default() -> Self {
        Self {
            topic: None,
            topic_prefix: None,
            retained: None,
            observed_after_ms: None,
            metadata: Vec::new(),
            sort: MqttPublicationSort::OriginalOrder,
            limit: None,
        }
    }
}

impl MqttPublicationQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_topic(mut self, topic: impl Into<String>) -> Self {
        self.topic = Some(topic.into());
        self
    }

    pub fn with_topic_prefix(mut self, topic_prefix: impl Into<String>) -> Self {
        self.topic_prefix = Some(topic_prefix.into());
        self
    }

    pub fn retained(mut self, retained: bool) -> Self {
        self.retained = Some(retained);
        self
    }

    pub fn observed_after(mut self, observed_after_ms: u64) -> Self {
        self.observed_after_ms = Some(observed_after_ms);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn sorted_by(mut self, sort: MqttPublicationSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FakeMqttBroker {
    publications: VecDeque<ScriptedMqttPublication>,
}

impl FakeMqttBroker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn publish(mut self, publication: ScriptedMqttPublication) -> Self {
        self.publications.push_back(publication);
        self
    }

    pub fn record_publication(&mut self, publication: ScriptedMqttPublication) {
        self.publications.push_back(publication);
    }

    pub fn next_publication(&mut self) -> Option<ScriptedMqttPublication> {
        self.publications.pop_front()
    }

    pub fn query_publications(
        &self,
        query: &MqttPublicationQuery,
    ) -> Vec<&ScriptedMqttPublication> {
        let mut publications = self
            .publications
            .iter()
            .filter(|publication| mqtt_publication_matches(publication, query))
            .collect::<Vec<_>>();
        sort_mqtt_publications(&mut publications, query.sort);
        if let Some(limit) = query.limit {
            publications.truncate(limit);
        }
        publications
    }

    pub fn len(&self) -> usize {
        self.publications.len()
    }

    pub fn is_empty(&self) -> bool {
        self.publications.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedLocalHttpResponse {
    pub method: LocalHttpMethod,
    pub url: String,
    pub status: u16,
    pub body: Vec<u8>,
    pub observed_at_ms: u64,
    pub metadata: Vec<Metadata>,
}

impl ScriptedLocalHttpResponse {
    pub fn new(
        method: LocalHttpMethod,
        url: impl Into<String>,
        status: u16,
        body: impl Into<Vec<u8>>,
        observed_at_ms: u64,
    ) -> Self {
        Self {
            method,
            url: url.into(),
            status,
            body: body.into(),
            observed_at_ms,
            metadata: Vec::new(),
        }
    }

    pub fn ok_json(
        method: LocalHttpMethod,
        url: impl Into<String>,
        body: impl Into<Vec<u8>>,
        observed_at_ms: u64,
    ) -> Self {
        Self::new(method, url, 200, body, observed_at_ms)
            .with_metadata("content_type", "application/json")
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn body_utf8(&self) -> Option<&str> {
        std::str::from_utf8(&self.body).ok()
    }

    pub fn matches_plan(&self, plan: &LocalHttpRequestPlan) -> bool {
        self.method == plan.method && self.url == plan.url
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LocalHttpResponseSort {
    #[default]
    OriginalOrder,
    Url,
    Status,
    ObservedAtAsc,
    ObservedAtDesc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalHttpResponseQuery {
    pub method: Option<LocalHttpMethod>,
    pub url: Option<String>,
    pub url_prefix: Option<String>,
    pub status: Option<u16>,
    pub status_class: Option<u16>,
    pub observed_after_ms: Option<u64>,
    pub metadata: Vec<Metadata>,
    pub sort: LocalHttpResponseSort,
    pub limit: Option<usize>,
}

impl Default for LocalHttpResponseQuery {
    fn default() -> Self {
        Self {
            method: None,
            url: None,
            url_prefix: None,
            status: None,
            status_class: None,
            observed_after_ms: None,
            metadata: Vec::new(),
            sort: LocalHttpResponseSort::OriginalOrder,
            limit: None,
        }
    }
}

impl LocalHttpResponseQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_method(mut self, method: LocalHttpMethod) -> Self {
        self.method = Some(method);
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_url_prefix(mut self, url_prefix: impl Into<String>) -> Self {
        self.url_prefix = Some(url_prefix.into());
        self
    }

    pub fn with_status(mut self, status: u16) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_status_class(mut self, status_class: u16) -> Self {
        self.status_class = Some(status_class);
        self
    }

    pub fn observed_after(mut self, observed_after_ms: u64) -> Self {
        self.observed_after_ms = Some(observed_after_ms);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn sorted_by(mut self, sort: LocalHttpResponseSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FakeLocalHttpServer {
    responses: VecDeque<ScriptedLocalHttpResponse>,
}

impl FakeLocalHttpServer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_response(mut self, response: ScriptedLocalHttpResponse) -> Self {
        self.responses.push_back(response);
        self
    }

    pub fn record_response(&mut self, response: ScriptedLocalHttpResponse) {
        self.responses.push_back(response);
    }

    pub fn next_response(&mut self) -> Option<ScriptedLocalHttpResponse> {
        self.responses.pop_front()
    }

    pub fn respond_to_plan(
        &mut self,
        plan: &LocalHttpRequestPlan,
    ) -> Option<ScriptedLocalHttpResponse> {
        let index = self
            .responses
            .iter()
            .position(|response| response.matches_plan(plan))?;
        self.responses.remove(index)
    }

    pub fn query_responses(
        &self,
        query: &LocalHttpResponseQuery,
    ) -> Vec<&ScriptedLocalHttpResponse> {
        let mut responses = self
            .responses
            .iter()
            .filter(|response| local_http_response_matches(response, query))
            .collect::<Vec<_>>();
        sort_local_http_responses(&mut responses, query.sort);
        if let Some(limit) = query.limit {
            responses.truncate(limit);
        }
        responses
    }

    pub fn len(&self) -> usize {
        self.responses.len()
    }

    pub fn is_empty(&self) -> bool {
        self.responses.is_empty()
    }
}

pub fn hue_bridge(id: &'static str, native_id: &'static str) -> Bridge {
    let mut bridge = Bridge::new(
        BridgeId::trusted(id),
        IntegrationId::trusted("hue"),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some("https://192.0.2.10".to_string());
    bridge.hardware_model = Some("BSB002".to_string());
    bridge.firmware_version = Some("1.66.1960062030".to_string());
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(1_000);
    bridge
        .identifiers
        .push(protocol_id(ProtocolFamily::Hue, "bridge", native_id));
    bridge.metadata.push(Metadata::new("fixture", "hue_bridge"));
    bridge
}

pub fn hue_device(id: &'static str, bridge_id: &BridgeId, native_id: &'static str) -> Device {
    Device {
        device_id: DeviceId::trusted(id),
        bridge_id: bridge_id.clone(),
        manufacturer: "Signify".to_string(),
        model: "Hue bulb".to_string(),
        name: "Kitchen".to_string(),
        serial: Some(native_id.to_string()),
        firmware_version: Some("1.0.0".to_string()),
        room_id: Some("kitchen".to_string()),
        entity_ids: Vec::new(),
        identifiers: vec![protocol_id(ProtocolFamily::Hue, "device", native_id)],
        health: Health::Online,
        metadata: vec![Metadata::new("fixture", "hue_device")],
    }
}

pub fn light_entity(id: &'static str, device_id: &DeviceId) -> Entity {
    Entity {
        entity_id: EntityId::trusted(id),
        device_id: device_id.clone(),
        kind: EntityKind::Light,
        name: "Kitchen Light".to_string(),
        capabilities: vec![
            Capability::light_on_off(),
            Capability::light_brightness(),
            Capability::light_color_temperature(),
        ],
        state: None,
        metadata: vec![Metadata::new("fixture", "light_entity")],
    }
}

pub fn occupancy_sensor_entity(id: &'static str, device_id: &DeviceId) -> Entity {
    Entity {
        entity_id: EntityId::trusted(id),
        device_id: device_id.clone(),
        kind: EntityKind::Sensor,
        name: "Kitchen Motion".to_string(),
        capabilities: vec![Capability::sensor_occupancy()],
        state: None,
        metadata: vec![Metadata::new("fixture", "occupancy_sensor")],
    }
}

pub fn turn_on_command(
    command_id: &'static str,
    entity_id: &EntityId,
    requested_by: &'static str,
    correlation_id: &'static str,
) -> DeviceCommand {
    DeviceCommand::new(
        CommandId::trusted(command_id),
        entity_id.clone(),
        CommandType::TurnOn,
        Value::Null,
        requested_by,
        CorrelationId::trusted(correlation_id),
    )
    .expect("fixture turn-on commands use a canonical capability")
}

pub fn accepted_command_result(command: &DeviceCommand, bridge_id: &BridgeId) -> CommandResult {
    CommandResult {
        command_id: command.command_id.clone(),
        status: CommandStatus::Accepted,
        bridge_id: bridge_id.clone(),
        correlation_id: command.correlation_id.clone(),
        message: Some("fixture accepted command".to_string()),
    }
}

pub fn confirmed_state(entity_id: &EntityId, value: Value, observed_at_ms: u64) -> StateSnapshot {
    StateSnapshot {
        entity_id: entity_id.clone(),
        value,
        source: StateSource::Poll,
        observed_at_ms,
        received_at_ms: observed_at_ms,
        expires_at_ms: None,
        confidence: StateConfidence::Confirmed,
    }
}

pub fn stale_state(entity_id: &EntityId, value: Value, observed_at_ms: u64) -> StateSnapshot {
    StateSnapshot {
        confidence: StateConfidence::Stale,
        ..confirmed_state(entity_id, value, observed_at_ms)
    }
}

pub fn optimistic_state(
    entity_id: &EntityId,
    value: Value,
    observed_at_ms: u64,
    expires_at_ms: u64,
) -> StateSnapshot {
    StateSnapshot {
        source: StateSource::OptimisticCommand,
        expires_at_ms: Some(expires_at_ms),
        confidence: StateConfidence::Optimistic,
        ..confirmed_state(entity_id, value, observed_at_ms)
    }
}

pub fn light_on_event(
    event_id: &'static str,
    bridge_id: &BridgeId,
    device_id: &DeviceId,
    entity_id: &EntityId,
    observed_at_ms: u64,
) -> DeviceEvent {
    state_delta_event(
        event_id,
        bridge_id,
        device_id,
        entity_id,
        CapabilityId::trusted("light.on_off"),
        Value::Bool(true),
        observed_at_ms,
    )
}

pub fn state_delta_event(
    event_id: &'static str,
    bridge_id: &BridgeId,
    device_id: &DeviceId,
    entity_id: &EntityId,
    capability_id: CapabilityId,
    value: Value,
    observed_at_ms: u64,
) -> DeviceEvent {
    DeviceEvent {
        event_id: EventId::trusted(event_id),
        bridge_id: bridge_id.clone(),
        device_id: Some(device_id.clone()),
        entity_id: Some(entity_id.clone()),
        observed_at_ms,
        received_at_ms: observed_at_ms,
        event_type: DeviceEventType::Updated,
        state_delta: Some(StateDelta {
            capability_id,
            value,
        }),
        raw_ref: None,
        correlation_id: None,
        metadata: vec![Metadata::new("fixture", "state_delta")],
    }
}

pub fn unavailable_event(
    event_id: &'static str,
    bridge_id: &BridgeId,
    device_id: &DeviceId,
    observed_at_ms: u64,
) -> DeviceEvent {
    DeviceEvent {
        event_id: EventId::trusted(event_id),
        bridge_id: bridge_id.clone(),
        device_id: Some(device_id.clone()),
        entity_id: None,
        observed_at_ms,
        received_at_ms: observed_at_ms,
        event_type: DeviceEventType::Unavailable,
        state_delta: None,
        raw_ref: None,
        correlation_id: None,
        metadata: vec![Metadata::new("fixture", "unavailable")],
    }
}

pub fn error_event(
    event_id: &'static str,
    bridge_id: &BridgeId,
    message: &'static str,
    observed_at_ms: u64,
) -> DeviceEvent {
    DeviceEvent {
        event_id: EventId::trusted(event_id),
        bridge_id: bridge_id.clone(),
        device_id: None,
        entity_id: None,
        observed_at_ms,
        received_at_ms: observed_at_ms,
        event_type: DeviceEventType::Error,
        state_delta: None,
        raw_ref: Some(message.to_string()),
        correlation_id: Some(CorrelationId::trusted("fixture-error")),
        metadata: vec![Metadata::new("fixture", "error")],
    }
}

pub fn install_fixture_in_registry(
    registry: &mut InMemorySmartHomeRegistry,
    fixture: &SmartHomeFixture,
) -> Result<(), RegistryError> {
    registry.upsert_bridge(fixture.bridge.clone())?;
    registry.upsert_device(fixture.device.clone())?;
    for entity in fixture.entities() {
        registry.upsert_entity(entity.clone())?;
    }
    Ok(())
}

pub fn registry_with_fixture(
    fixture: &SmartHomeFixture,
) -> Result<InMemorySmartHomeRegistry, RegistryError> {
    let mut registry = InMemorySmartHomeRegistry::new();
    install_fixture_in_registry(&mut registry, fixture)?;
    Ok(registry)
}

pub fn hue_lighting_registry() -> InMemorySmartHomeRegistry {
    SmartHomeFixture::hue_lighting()
        .to_registry()
        .expect("hue lighting fixture records are internally consistent")
}

pub fn runtime_with_fixture(fixture: &SmartHomeFixture) -> Result<SmartHomeRuntime, RuntimeError> {
    let mut runtime = SmartHomeRuntime::new();
    runtime.upsert_bridge(fixture.bridge.clone())?;
    runtime.upsert_device(fixture.device.clone())?;
    for entity in fixture.entities() {
        runtime.upsert_entity(entity.clone())?;
    }
    Ok(runtime)
}

pub fn hue_lighting_runtime() -> SmartHomeRuntime {
    runtime_with_fixture(&SmartHomeFixture::hue_lighting())
        .expect("hue lighting fixture records are internally consistent")
}

pub fn hue_sse_stream_spec(fixture: &SmartHomeFixture) -> EventStreamSpec {
    EventStreamSpec::hue_sse(
        fixture.bridge.bridge_id.clone(),
        fixture
            .bridge
            .address
            .as_deref()
            .unwrap_or("https://192.0.2.10")
            .trim_end_matches('/')
            .to_string()
            + "/eventstream/clip/v2",
    )
    .with_heartbeat_timeout(1_000)
    .with_stale_after(5_000)
    .with_metadata(Metadata::new("fixture", "hue_sse_stream"))
}

pub fn hue_sse_stream_state(fixture: &SmartHomeFixture, connected_at_ms: u64) -> EventStreamState {
    let mut state = EventStreamState::new(hue_sse_stream_spec(fixture), connected_at_ms);
    state.mark_connected(connected_at_ms);
    state
}

fn scripted_event_observed_at_ms(step: &ScriptedEvent) -> u64 {
    match step {
        ScriptedEvent::Event(event) => event.observed_at_ms,
        ScriptedEvent::Disconnect { at_ms, .. } | ScriptedEvent::Gap { at_ms, .. } => *at_ms,
    }
}

fn mqtt_publication_matches(
    publication: &ScriptedMqttPublication,
    query: &MqttPublicationQuery,
) -> bool {
    if let Some(topic) = query.topic.as_deref() {
        if publication.topic != topic {
            return false;
        }
    }
    if let Some(topic_prefix) = query.topic_prefix.as_deref() {
        if !publication.topic.starts_with(topic_prefix) {
            return false;
        }
    }
    if let Some(retained) = query.retained {
        if publication.retained != retained {
            return false;
        }
    }
    if let Some(observed_after_ms) = query.observed_after_ms {
        if publication.observed_at_ms <= observed_after_ms {
            return false;
        }
    }
    query.metadata.iter().all(|required| {
        publication
            .metadata
            .iter()
            .any(|metadata| metadata.key == required.key && metadata.value == required.value)
    })
}

fn sort_mqtt_publications(
    publications: &mut Vec<&ScriptedMqttPublication>,
    sort: MqttPublicationSort,
) {
    match sort {
        MqttPublicationSort::OriginalOrder => {}
        MqttPublicationSort::Topic => publications.sort_by(|left, right| {
            left.topic
                .cmp(&right.topic)
                .then_with(|| left.observed_at_ms.cmp(&right.observed_at_ms))
        }),
        MqttPublicationSort::ObservedAtAsc => publications.sort_by(|left, right| {
            left.observed_at_ms
                .cmp(&right.observed_at_ms)
                .then_with(|| left.topic.cmp(&right.topic))
        }),
        MqttPublicationSort::ObservedAtDesc => publications.sort_by(|left, right| {
            right
                .observed_at_ms
                .cmp(&left.observed_at_ms)
                .then_with(|| left.topic.cmp(&right.topic))
        }),
    }
}

fn local_http_response_matches(
    response: &ScriptedLocalHttpResponse,
    query: &LocalHttpResponseQuery,
) -> bool {
    if let Some(method) = query.method {
        if response.method != method {
            return false;
        }
    }
    if let Some(url) = query.url.as_deref() {
        if response.url != url {
            return false;
        }
    }
    if let Some(url_prefix) = query.url_prefix.as_deref() {
        if !response.url.starts_with(url_prefix) {
            return false;
        }
    }
    if let Some(status) = query.status {
        if response.status != status {
            return false;
        }
    }
    if let Some(status_class) = query.status_class {
        if response.status / 100 != status_class {
            return false;
        }
    }
    if let Some(observed_after_ms) = query.observed_after_ms {
        if response.observed_at_ms <= observed_after_ms {
            return false;
        }
    }
    query.metadata.iter().all(|required| {
        response
            .metadata
            .iter()
            .any(|metadata| metadata.key == required.key && metadata.value == required.value)
    })
}

fn sort_local_http_responses(
    responses: &mut Vec<&ScriptedLocalHttpResponse>,
    sort: LocalHttpResponseSort,
) {
    match sort {
        LocalHttpResponseSort::OriginalOrder => {}
        LocalHttpResponseSort::Url => responses.sort_by(|left, right| {
            left.url
                .cmp(&right.url)
                .then_with(|| left.observed_at_ms.cmp(&right.observed_at_ms))
        }),
        LocalHttpResponseSort::Status => responses.sort_by(|left, right| {
            left.status
                .cmp(&right.status)
                .then_with(|| left.url.cmp(&right.url))
        }),
        LocalHttpResponseSort::ObservedAtAsc => responses.sort_by(|left, right| {
            left.observed_at_ms
                .cmp(&right.observed_at_ms)
                .then_with(|| left.url.cmp(&right.url))
        }),
        LocalHttpResponseSort::ObservedAtDesc => responses.sort_by(|left, right| {
            right
                .observed_at_ms
                .cmp(&left.observed_at_ms)
                .then_with(|| left.url.cmp(&right.url))
        }),
    }
}

fn protocol_id(
    family: ProtocolFamily,
    kind: &'static str,
    value: &'static str,
) -> ProtocolIdentifier {
    ProtocolIdentifier::new(family, kind, value).expect("fixture protocol ids are valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hue_fixture_projects_normalized_bridge_device_and_entities() {
        let fixture = SmartHomeFixture::hue_lighting();

        assert_eq!(fixture.bridge.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(fixture.bridge.health, Health::Online);
        assert_eq!(fixture.device.bridge_id, fixture.bridge.bridge_id);
        assert_eq!(fixture.light.kind, EntityKind::Light);
        assert_eq!(fixture.sensor.kind, EntityKind::Sensor);
        assert_eq!(fixture.entities().len(), 2);
    }

    #[test]
    fn state_helpers_cover_fresh_stale_and_optimistic_shapes() {
        let entity_id = EntityId::trusted("entity-light-1");
        let confirmed = confirmed_state(&entity_id, Value::Bool(true), 1_000);
        let stale = stale_state(&entity_id, Value::Bool(false), 1_000);
        let optimistic = optimistic_state(&entity_id, Value::Bool(true), 1_000, 1_500);

        assert!(!confirmed.is_stale_at(2_000));
        assert!(stale.is_stale_at(1_001));
        assert!(!optimistic.is_stale_at(1_499));
        assert!(optimistic.is_stale_at(1_500));
        assert_eq!(optimistic.source, StateSource::OptimisticCommand);
    }

    #[test]
    fn fake_event_stream_preserves_script_order() {
        let fixture = SmartHomeFixture::hue_lighting();
        let event = light_on_event(
            "event-1",
            &fixture.bridge.bridge_id,
            &fixture.device.device_id,
            &fixture.light.entity_id,
            1_000,
        );
        let mut stream = FakeEventStream::new()
            .push_event(event.clone())
            .push_gap(2, 1_100)
            .push_disconnect("test disconnect", 1_200);

        assert_eq!(stream.len(), 3);
        assert_eq!(
            stream.next_step(),
            Some(ScriptedEvent::Event(Box::new(event)))
        );
        assert_eq!(
            stream.next_step(),
            Some(ScriptedEvent::Gap {
                missing_events: 2,
                at_ms: 1_100
            })
        );
        assert_eq!(
            stream.next_step(),
            Some(ScriptedEvent::Disconnect {
                reason: "test disconnect".to_string(),
                at_ms: 1_200
            })
        );
        assert!(stream.is_empty());
    }

    #[test]
    fn hue_sse_stream_fixture_builds_connected_state() {
        let fixture = SmartHomeFixture::hue_lighting();
        let state = hue_sse_stream_state(&fixture, 1_000);

        assert_eq!(state.spec.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(state.spec.bridge_id, fixture.bridge.bridge_id);
        assert_eq!(
            state.spec.endpoint.as_deref(),
            Some("https://192.0.2.10/eventstream/clip/v2")
        );
        assert_eq!(state.status, EventStreamStatus::Healthy);
        assert_eq!(state.connected_at_ms, Some(1_000));
        assert_eq!(state.last_heartbeat_at_ms, Some(1_000));
    }

    #[test]
    fn fake_event_stream_driver_updates_stream_state_and_runtime() {
        let fixture = SmartHomeFixture::hue_lighting();
        let event = light_on_event(
            "event-1",
            &fixture.bridge.bridge_id,
            &fixture.device.device_id,
            &fixture.light.entity_id,
            1_100,
        );
        let stream = FakeEventStream::new()
            .push_event(event.clone())
            .push_gap(2, 1_200)
            .push_disconnect("bridge closed stream", 1_300);
        let mut driver = FakeEventStreamDriver::hue_sse(&fixture, stream, 1_000);
        let mut runtime = runtime_with_fixture(&fixture).unwrap();

        let first = driver.next_runtime_step(&mut runtime).unwrap().unwrap();

        assert_eq!(first.status, EventStreamStatus::Healthy);
        assert_eq!(first.checkpoint.cursor.sequence, 1);
        assert_eq!(
            runtime
                .registry()
                .state(&fixture.light.entity_id)
                .unwrap()
                .value,
            Value::Object(vec![("light.on_off".to_string(), Value::Bool(true))])
        );
        assert_eq!(runtime.event_bus().published().len(), 1);

        let gap = driver.next_runtime_step(&mut runtime).unwrap().unwrap();

        assert_eq!(gap.status, EventStreamStatus::Degraded);
        assert_eq!(gap.restart_reason, Some(EventStreamRestartReason::EventGap));
        assert_eq!(runtime.event_bus().published().len(), 1);

        let disconnect = driver.next_runtime_step(&mut runtime).unwrap().unwrap();

        assert_eq!(disconnect.status, EventStreamStatus::Disconnected);
        assert!(driver.is_empty());
        assert_eq!(
            runtime
                .registry()
                .bridge(&fixture.bridge.bridge_id)
                .unwrap()
                .health,
            Health::Degraded
        );
        assert_eq!(runtime.event_bus().published().len(), 3);
    }

    #[test]
    fn runtime_fixture_seeds_runtime_without_bespoke_setup() {
        let fixture = SmartHomeFixture::hue_lighting();
        let runtime = runtime_with_fixture(&fixture).unwrap();

        assert_eq!(runtime.registry().counts().bridges, 1);
        assert_eq!(runtime.registry().counts().devices, 1);
        assert_eq!(runtime.registry().counts().entities, 2);
        assert!(runtime.event_bus().published().is_empty());
    }

    #[test]
    fn fixture_clock_advances_saturating_time() {
        let mut clock = FixtureClock::new(u64::MAX - 5);

        assert_eq!(clock.advance_ms(3), u64::MAX - 2);
        assert_eq!(clock.advance_ms(10), u64::MAX);
    }

    #[test]
    fn event_helpers_mark_runtime_failure_modes() {
        let fixture = SmartHomeFixture::hue_lighting();
        let unavailable = unavailable_event(
            "event-unavailable",
            &fixture.bridge.bridge_id,
            &fixture.device.device_id,
            1_000,
        );
        let error = error_event("event-error", &fixture.bridge.bridge_id, "boom", 1_001);

        assert_eq!(unavailable.event_type, DeviceEventType::Unavailable);
        assert_eq!(error.event_type, DeviceEventType::Error);
        assert_eq!(error.raw_ref.as_deref(), Some("boom"));
    }

    #[test]
    fn fixture_installs_normalized_records_into_registry() {
        let fixture = SmartHomeFixture::hue_lighting();
        let registry = fixture.to_registry().unwrap();

        assert_eq!(registry.counts().bridges, 1);
        assert_eq!(registry.counts().devices, 1);
        assert_eq!(registry.counts().entities, 2);
        assert_eq!(registry.counts().protocol_identifiers, 2);
        assert_eq!(
            registry
                .bridge(&fixture.bridge.bridge_id)
                .unwrap()
                .bridge_id,
            fixture.bridge.bridge_id
        );
        assert_eq!(
            registry
                .device(&fixture.device.device_id)
                .unwrap()
                .entity_ids,
            vec![
                fixture.light.entity_id.clone(),
                fixture.sensor.entity_id.clone()
            ]
        );
        assert_eq!(
            registry.entity(&fixture.light.entity_id).unwrap().kind,
            EntityKind::Light
        );
        assert_eq!(
            registry.entity(&fixture.sensor.entity_id).unwrap().kind,
            EntityKind::Sensor
        );
    }

    #[test]
    fn fake_command_bus_preserves_command_result_order() {
        let fixture = SmartHomeFixture::hue_lighting();
        let command = turn_on_command(
            "command-1",
            &fixture.light.entity_id,
            "agent:test",
            "corr-1",
        );
        let result = accepted_command_result(&command, &fixture.bridge.bridge_id);
        let mut bus = FakeCommandBus::new()
            .push_command(command.clone())
            .push_result(result.clone());

        assert_eq!(bus.pending_command_count(), 1);
        assert_eq!(bus.pending_result_count(), 1);
        assert_eq!(bus.next_command(), Some(command));
        assert_eq!(bus.next_result(), Some(result));
        assert!(bus.is_idle());
    }

    #[test]
    fn fake_mqtt_broker_preserves_publication_order() {
        let first = ScriptedMqttPublication::new(
            "home/kitchen/light/state",
            br#"{"state":"ON"}"#.to_vec(),
            1_000,
        )
        .retained()
        .with_metadata("fixture", "mqtt");
        let second = ScriptedMqttPublication::new(
            "home/kitchen/light/brightness",
            br#"{"brightness":42}"#.to_vec(),
            1_100,
        );
        let mut broker = FakeMqttBroker::new()
            .publish(first.clone())
            .publish(second.clone());

        assert_eq!(broker.len(), 2);
        assert_eq!(broker.next_publication(), Some(first.clone()));
        assert_eq!(first.payload_utf8(), Some(r#"{"state":"ON"}"#));
        assert_eq!(broker.next_publication(), Some(second));
        assert!(broker.is_empty());
    }

    #[test]
    fn fake_mqtt_broker_queries_publications_without_consuming_queue() {
        let retained_state = ScriptedMqttPublication::new(
            "home/kitchen/light/state",
            br#"{"state":"ON"}"#.to_vec(),
            1_000,
        )
        .retained()
        .with_metadata("fixture", "mqtt")
        .with_metadata("entity", "kitchen");
        let live_state = ScriptedMqttPublication::new(
            "home/office/light/state",
            br#"{"state":"OFF"}"#.to_vec(),
            1_200,
        )
        .with_metadata("fixture", "mqtt");
        let command = ScriptedMqttPublication::new(
            "home/kitchen/light/set",
            br#"{"state":"OFF"}"#.to_vec(),
            1_100,
        )
        .with_metadata("fixture", "mqtt");
        let mut broker = FakeMqttBroker::new()
            .publish(retained_state.clone())
            .publish(command.clone())
            .publish(live_state.clone());

        let state_publications = broker.query_publications(
            &MqttPublicationQuery::new()
                .with_topic_prefix("home/")
                .observed_after(1_050)
                .sorted_by(MqttPublicationSort::ObservedAtDesc)
                .limited_to(2),
        );

        assert_eq!(
            state_publications
                .iter()
                .map(|publication| publication.topic.as_str())
                .collect::<Vec<_>>(),
            vec!["home/office/light/state", "home/kitchen/light/set"]
        );

        let retained = broker.query_publications(
            &MqttPublicationQuery::new()
                .retained(true)
                .with_metadata("entity", "kitchen"),
        );

        assert_eq!(retained, vec![&retained_state]);
        assert_eq!(broker.len(), 3);
        assert_eq!(broker.next_publication(), Some(retained_state));
        assert_eq!(broker.next_publication(), Some(command));
        assert_eq!(broker.next_publication(), Some(live_state));
    }

    #[test]
    fn fake_local_http_server_matches_planned_requests_without_sockets() {
        let fixture = SmartHomeFixture::hue_lighting();
        let endpoint = smart_home_local_http::LocalHttpEndpoint::hue_bridge(
            fixture.bridge.bridge_id.clone(),
            "192.0.2.10",
        )
        .unwrap();
        let plan = smart_home_local_http::LocalHttpRequestTemplate::new(
            LocalHttpMethod::Get,
            "/clip/v2/resource/light",
        )
        .unwrap()
        .plan(&endpoint, Vec::new())
        .unwrap();
        let response = ScriptedLocalHttpResponse::ok_json(
            LocalHttpMethod::Get,
            plan.url.clone(),
            br#"{"data":[]}"#.to_vec(),
            1_000,
        )
        .with_metadata("fixture", "hue_clip");
        let mut server = FakeLocalHttpServer::new().push_response(response.clone());

        let matched = server.respond_to_plan(&plan).unwrap();

        assert_eq!(matched, response);
        assert_eq!(matched.body_utf8(), Some(r#"{"data":[]}"#));
        assert!(server.is_empty());
    }

    #[test]
    fn fake_local_http_server_queries_responses_without_consuming_queue() {
        let light_state = ScriptedLocalHttpResponse::ok_json(
            LocalHttpMethod::Get,
            "https://192.0.2.10/clip/v2/resource/light",
            br#"{"data":[{"id":"light-1"}]}"#.to_vec(),
            1_000,
        )
        .with_metadata("fixture", "hue_clip");
        let command_accept = ScriptedLocalHttpResponse::new(
            LocalHttpMethod::Put,
            "https://192.0.2.10/clip/v2/resource/light/light-1",
            202,
            br#"{"errors":[]}"#.to_vec(),
            1_100,
        )
        .with_metadata("fixture", "hue_clip");
        let bridge_busy = ScriptedLocalHttpResponse::new(
            LocalHttpMethod::Get,
            "https://192.0.2.10/clip/v2/resource/bridge",
            503,
            br#"{"errors":[{"description":"busy"}]}"#.to_vec(),
            1_200,
        )
        .with_metadata("fixture", "hue_clip")
        .with_metadata("failure", "bridge_busy");
        let mut server = FakeLocalHttpServer::new()
            .push_response(light_state.clone())
            .push_response(command_accept.clone())
            .push_response(bridge_busy.clone());

        let recent_successes = server.query_responses(
            &LocalHttpResponseQuery::new()
                .with_url_prefix("https://192.0.2.10/clip/v2")
                .with_status_class(2)
                .observed_after(1_050)
                .sorted_by(LocalHttpResponseSort::ObservedAtDesc),
        );

        assert_eq!(recent_successes, vec![&command_accept]);

        let failures = server.query_responses(
            &LocalHttpResponseQuery::new()
                .with_method(LocalHttpMethod::Get)
                .with_status(503)
                .with_metadata("failure", "bridge_busy")
                .limited_to(1),
        );

        assert_eq!(failures, vec![&bridge_busy]);
        assert_eq!(server.len(), 3);
        assert_eq!(server.next_response(), Some(light_state));
        assert_eq!(server.next_response(), Some(command_accept));
        assert_eq!(server.next_response(), Some(bridge_busy));
        assert!(server.is_empty());
    }

    #[test]
    fn hue_lighting_registry_is_ready_for_event_replay() {
        let fixture = SmartHomeFixture::hue_lighting();
        let mut registry = hue_lighting_registry();
        let event = light_on_event(
            "event-1",
            &fixture.bridge.bridge_id,
            &fixture.device.device_id,
            &fixture.light.entity_id,
            2_000,
        );

        registry.record_event(event).unwrap();

        let state = registry.state(&fixture.light.entity_id).unwrap();
        assert_eq!(registry.counts().events, 1);
        assert_eq!(state.confidence, StateConfidence::Confirmed);
        assert_eq!(
            state.value,
            Value::Object(vec![("light.on_off".to_string(), Value::Bool(true))])
        );
    }
}
