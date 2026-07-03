//! Deterministic smart-home fixtures and fake streams for D23 tests.
//!
//! This crate is pure data. It gives runtime and integration packages reusable
//! fixtures without opening sockets, touching radios, reading files, or calling
//! cloud APIs.

#![forbid(unsafe_code)]

use hue_core::{
    hue_discovery_record_from_mdns, hue_discovery_worker_run_from_mdns_scan_report,
    HUE_MDNS_SERVICE_TYPE,
};
use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CommandId, CommandResult,
    CommandStatus, CommandType, CorrelationId, Device, DeviceCommand, DeviceEvent, DeviceEventType,
    DeviceId, Entity, EntityId, EntityKind, EventId, Health, IntegrationId, Metadata,
    ProtocolFamily, ProtocolIdentifier, Scene, SceneAction, SceneId, SceneScope, StateConfidence,
    StateDelta, StateSnapshot, StateSource, Value,
};
use smart_home_discovery::{
    DiscoveryError, DiscoveryRecord, DiscoverySource, DiscoveryWorkerId, DiscoveryWorkerKind,
    DiscoveryWorkerRun, MdnsAdvertisement, MdnsScanNetwork, MdnsScanResult, MdnsWorkerScanExecutor,
    MdnsWorkerScanReport, MdnsWorkerScanRequest,
};
use smart_home_event_streams::{
    EventStreamCheckpoint, EventStreamRestartReason, EventStreamSpec, EventStreamState,
    EventStreamStatus, MqttQos, MqttTopicError, MqttTopicFilter,
};
use smart_home_local_http::{LocalHttpMethod, LocalHttpRequestPlan};
use smart_home_registry::{InMemorySmartHomeRegistry, RegistryError};
use smart_home_runtime::{
    BridgeHealthReport, RuntimeError, ScheduledDiscoveryWorker, SmartHomeRuntime,
};
use std::{collections::VecDeque, time::Duration};

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
    pub scene: Scene,
}

impl SmartHomeFixture {
    pub fn hue_lighting() -> Self {
        let bridge = hue_bridge("bridge-1", "001788fffeabcdef");
        let device = hue_device("device-1", &bridge.bridge_id, "device-native-1");
        let light = light_entity("entity-light-1", &device.device_id);
        let sensor = occupancy_sensor_entity("entity-sensor-1", &device.device_id);
        let scene = room_scene("scene-kitchen-bright", &light.entity_id);

        Self {
            bridge,
            device,
            light,
            sensor,
            scene,
        }
    }

    pub fn entities(&self) -> [&Entity; 2] {
        [&self.light, &self.sensor]
    }

    pub fn scenes(&self) -> [&Scene; 1] {
        [&self.scene]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FakeEventStreamSummary {
    pub total_steps: usize,
    pub device_events: usize,
    pub disconnects: usize,
    pub gaps: usize,
    pub missing_events: u64,
    pub first_observed_at_ms: Option<u64>,
    pub last_observed_at_ms: Option<u64>,
}

impl FakeEventStreamSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_steps<'a, I>(steps: I) -> Self
    where
        I: IntoIterator<Item = &'a ScriptedEvent>,
    {
        let mut summary = Self::empty();

        for step in steps {
            let observed_at_ms = scripted_event_observed_at_ms(step);
            summary.total_steps += 1;
            summary.first_observed_at_ms = summary.first_observed_at_ms.or(Some(observed_at_ms));
            summary.last_observed_at_ms = Some(observed_at_ms);

            match step {
                ScriptedEvent::Event(_) => summary.device_events += 1,
                ScriptedEvent::Disconnect { .. } => summary.disconnects += 1,
                ScriptedEvent::Gap { missing_events, .. } => {
                    summary.gaps += 1;
                    summary.missing_events = summary
                        .missing_events
                        .saturating_add(u64::from(*missing_events));
                }
            }
        }

        summary
    }

    pub fn has_disconnects(self) -> bool {
        self.disconnects > 0
    }

    pub fn has_gaps(self) -> bool {
        self.gaps > 0
    }

    pub fn is_empty(self) -> bool {
        self.total_steps == 0
    }
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

    pub fn summary(&self) -> FakeEventStreamSummary {
        FakeEventStreamSummary::from_steps(&self.events)
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

    pub fn stream_summary(&self) -> FakeEventStreamSummary {
        self.stream.summary()
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedMqttSubscription {
    pub subscription_id: String,
    pub topic_filter: MqttTopicFilter,
    pub qos: MqttQos,
    pub metadata: Vec<Metadata>,
}

impl ScriptedMqttSubscription {
    pub fn new(subscription_id: impl Into<String>, topic_filter: MqttTopicFilter) -> Self {
        Self {
            subscription_id: subscription_id.into(),
            topic_filter,
            qos: MqttQos::AtLeastOnce,
            metadata: Vec::new(),
        }
    }

    pub fn with_qos(mut self, qos: MqttQos) -> Self {
        self.qos = qos;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn matches_publication(
        &self,
        publication: &ScriptedMqttPublication,
    ) -> Result<bool, MqttTopicError> {
        self.topic_filter.matches_topic(&publication.topic)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptedMqttDelivery<'a> {
    pub subscription: &'a ScriptedMqttSubscription,
    pub publication: &'a ScriptedMqttPublication,
}

impl ScriptedMqttDelivery<'_> {
    pub fn topic(&self) -> &str {
        &self.publication.topic
    }

    pub fn qos(&self) -> MqttQos {
        self.subscription.qos
    }

    pub fn retained(&self) -> bool {
        self.publication.retained
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FakeMqttBrokerSummary {
    pub total_publications: usize,
    pub retained_publications: usize,
    pub live_publications: usize,
    pub total_payload_bytes: usize,
    pub metadata_entries: usize,
    pub earliest_observed_at_ms: Option<u64>,
    pub latest_observed_at_ms: Option<u64>,
}

impl FakeMqttBrokerSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_publications<'a, I>(publications: I) -> Self
    where
        I: IntoIterator<Item = &'a ScriptedMqttPublication>,
    {
        let mut summary = Self::empty();

        for publication in publications {
            summary.total_publications += 1;
            summary.total_payload_bytes = summary
                .total_payload_bytes
                .saturating_add(publication.payload.len());
            summary.metadata_entries = summary
                .metadata_entries
                .saturating_add(publication.metadata.len());
            if publication.retained {
                summary.retained_publications += 1;
            } else {
                summary.live_publications += 1;
            }
            summary.earliest_observed_at_ms = Some(
                summary
                    .earliest_observed_at_ms
                    .map_or(publication.observed_at_ms, |observed_at_ms| {
                        observed_at_ms.min(publication.observed_at_ms)
                    }),
            );
            summary.latest_observed_at_ms = Some(
                summary
                    .latest_observed_at_ms
                    .map_or(publication.observed_at_ms, |observed_at_ms| {
                        observed_at_ms.max(publication.observed_at_ms)
                    }),
            );
        }

        summary
    }

    pub fn has_retained(self) -> bool {
        self.retained_publications > 0
    }

    pub fn has_payloads(self) -> bool {
        self.total_payload_bytes > 0
    }

    pub fn is_empty(self) -> bool {
        self.total_publications == 0
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

    pub fn deliveries_for_subscription<'a>(
        &'a self,
        subscription: &'a ScriptedMqttSubscription,
    ) -> Result<Vec<ScriptedMqttDelivery<'a>>, MqttTopicError> {
        let mut deliveries = Vec::new();
        for publication in &self.publications {
            if subscription.matches_publication(publication)? {
                deliveries.push(ScriptedMqttDelivery {
                    subscription,
                    publication,
                });
            }
        }
        Ok(deliveries)
    }

    pub fn summary(&self) -> FakeMqttBrokerSummary {
        FakeMqttBrokerSummary::from_publications(&self.publications)
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FakeLocalHttpServerSummary {
    pub total_responses: usize,
    pub get_responses: usize,
    pub post_responses: usize,
    pub put_responses: usize,
    pub patch_responses: usize,
    pub delete_responses: usize,
    pub informational_responses: usize,
    pub success_responses: usize,
    pub redirect_responses: usize,
    pub client_error_responses: usize,
    pub server_error_responses: usize,
    pub other_status_responses: usize,
    pub body_bearing_responses: usize,
    pub bodyless_responses: usize,
    pub total_body_bytes: usize,
    pub metadata_entries: usize,
    pub earliest_observed_at_ms: Option<u64>,
    pub latest_observed_at_ms: Option<u64>,
}

impl FakeLocalHttpServerSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_responses<'a, I>(responses: I) -> Self
    where
        I: IntoIterator<Item = &'a ScriptedLocalHttpResponse>,
    {
        let mut summary = Self::empty();

        for response in responses {
            summary.total_responses += 1;
            match response.method {
                LocalHttpMethod::Get => summary.get_responses += 1,
                LocalHttpMethod::Post => summary.post_responses += 1,
                LocalHttpMethod::Put => summary.put_responses += 1,
                LocalHttpMethod::Patch => summary.patch_responses += 1,
                LocalHttpMethod::Delete => summary.delete_responses += 1,
            }
            match response.status / 100 {
                1 => summary.informational_responses += 1,
                2 => summary.success_responses += 1,
                3 => summary.redirect_responses += 1,
                4 => summary.client_error_responses += 1,
                5 => summary.server_error_responses += 1,
                _ => summary.other_status_responses += 1,
            }
            if response.body.is_empty() {
                summary.bodyless_responses += 1;
            } else {
                summary.body_bearing_responses += 1;
            }
            summary.total_body_bytes = summary.total_body_bytes.saturating_add(response.body.len());
            summary.metadata_entries = summary
                .metadata_entries
                .saturating_add(response.metadata.len());
            summary.earliest_observed_at_ms = Some(
                summary
                    .earliest_observed_at_ms
                    .map_or(response.observed_at_ms, |observed_at_ms| {
                        observed_at_ms.min(response.observed_at_ms)
                    }),
            );
            summary.latest_observed_at_ms = Some(
                summary
                    .latest_observed_at_ms
                    .map_or(response.observed_at_ms, |observed_at_ms| {
                        observed_at_ms.max(response.observed_at_ms)
                    }),
            );
        }

        summary
    }

    pub fn has_errors(self) -> bool {
        self.client_error_responses > 0
            || self.server_error_responses > 0
            || self.other_status_responses > 0
    }

    pub fn has_mutations(self) -> bool {
        self.post_responses > 0
            || self.put_responses > 0
            || self.patch_responses > 0
            || self.delete_responses > 0
    }

    pub fn is_empty(self) -> bool {
        self.total_responses == 0
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

    pub fn summary(&self) -> FakeLocalHttpServerSummary {
        FakeLocalHttpServerSummary::from_responses(&self.responses)
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

pub fn hue_bridge_discovery_record(
    native_id: impl Into<String>,
    discovered_at_ms: u64,
) -> DiscoveryRecord {
    hue_discovery_record_from_mdns(&hue_bridge_mdns_advertisement(native_id, discovered_at_ms))
        .expect("fixture Hue mDNS advertisement maps to a discovery record")
        .with_metadata("fixture", "hue_bridge_discovery")
}

pub fn hue_bridge_mdns_advertisement(
    native_id: impl Into<String>,
    discovered_at_ms: u64,
) -> MdnsAdvertisement {
    let native_id = native_id.into();
    MdnsAdvertisement::new(
        HUE_MDNS_SERVICE_TYPE,
        "Hue Bridge",
        "hue-bridge.local",
        443,
        discovered_at_ms,
    )
    .expect("fixture Hue mDNS advertisement is valid")
    .with_address("192.0.2.10")
    .expect("fixture Hue mDNS address is valid")
    .with_txt("bridgeid", native_id)
    .expect("fixture Hue bridge id TXT is valid")
    .with_txt("modelid", "BSB002")
    .expect("fixture Hue model TXT is valid")
    .with_txt("swversion", "1.66.1960062030")
    .expect("fixture Hue firmware TXT is valid")
}

pub fn hue_bridge_mdns_scan_result(
    native_id: impl Into<String>,
    discovered_at_ms: u64,
) -> MdnsScanResult {
    MdnsScanResult {
        service_type: HUE_MDNS_SERVICE_TYPE.to_string(),
        discovered_at_ms,
        datagram_count: 1,
        advertisements: vec![hue_bridge_mdns_advertisement(native_id, discovered_at_ms)],
        failures: Vec::new(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScriptedMdnsWorkerScanExecutor {
    outcomes: VecDeque<Result<MdnsScanResult, DiscoveryError>>,
    observed_requests: Vec<MdnsWorkerScanRequest>,
}

impl ScriptedMdnsWorkerScanExecutor {
    pub fn new(outcomes: impl IntoIterator<Item = Result<MdnsScanResult, DiscoveryError>>) -> Self {
        Self {
            outcomes: outcomes.into_iter().collect(),
            observed_requests: Vec::new(),
        }
    }

    pub fn push_outcome(mut self, outcome: Result<MdnsScanResult, DiscoveryError>) -> Self {
        self.outcomes.push_back(outcome);
        self
    }

    pub fn record_outcome(&mut self, outcome: Result<MdnsScanResult, DiscoveryError>) {
        self.outcomes.push_back(outcome);
    }

    pub fn observed_requests(&self) -> &[MdnsWorkerScanRequest] {
        &self.observed_requests
    }

    pub fn pending_outcome_count(&self) -> usize {
        self.outcomes.len()
    }
}

impl MdnsWorkerScanExecutor for ScriptedMdnsWorkerScanExecutor {
    fn run_request(
        &mut self,
        request: &MdnsWorkerScanRequest,
    ) -> Result<MdnsScanResult, DiscoveryError> {
        self.observed_requests.push(request.clone());
        self.outcomes.pop_front().unwrap_or_else(|| {
            Err(DiscoveryError::MdnsTransport {
                message: "missing scripted mDNS scan outcome".to_string(),
            })
        })
    }
}

pub fn hue_bridge_mdns_scan_report(
    native_id: impl Into<String>,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> MdnsWorkerScanReport {
    let worker_id = DiscoveryWorkerId::trusted("fixture-hue-discovery-worker");
    let integration_id = IntegrationId::trusted("hue");
    let request = MdnsWorkerScanRequest::new(
        worker_id.clone(),
        integration_id.clone(),
        "fixture-lan0",
        MdnsScanNetwork::Ipv4,
        HUE_MDNS_SERVICE_TYPE,
        completed_at_ms,
        Duration::from_millis(250),
    )
    .expect("fixture Hue mDNS scan request is valid")
    .with_metadata("fixture", "hue_bridge_mdns_scan_report");
    let mut report = MdnsWorkerScanReport::new(
        worker_id,
        integration_id,
        HUE_MDNS_SERVICE_TYPE,
        started_at_ms,
        completed_at_ms,
    )
    .expect("fixture Hue mDNS scan report is valid")
    .with_metadata("fixture", "hue_bridge_mdns_scan_report");
    report
        .push_success(
            request,
            hue_bridge_mdns_scan_result(native_id, completed_at_ms),
        )
        .expect("fixture Hue mDNS scan result matches its request");
    report
}

pub fn hue_bridge_discovery_worker_run(
    native_id: impl Into<String>,
    started_at_ms: u64,
    completed_at_ms: u64,
) -> DiscoveryWorkerRun {
    let report = hue_bridge_mdns_scan_report(native_id, started_at_ms, completed_at_ms);
    let mut run = hue_discovery_worker_run_from_mdns_scan_report(&report)
        .expect("fixture Hue discovery worker run is valid")
        .with_metadata("fixture", "hue_bridge_discovery_worker");
    for record in &mut run.records {
        record
            .metadata
            .push(Metadata::new("fixture", "hue_bridge_discovery"));
    }
    run
}

pub fn hue_bridge_discovery_worker_schedule(
    first_due_at_ms: u64,
    interval_ms: u64,
    run_timeout_ms: u64,
) -> ScheduledDiscoveryWorker {
    ScheduledDiscoveryWorker::new(
        DiscoveryWorkerId::trusted("fixture-hue-discovery-worker"),
        IntegrationId::trusted("hue"),
        DiscoveryWorkerKind::MdnsScan,
        interval_ms,
        run_timeout_ms,
        first_due_at_ms,
    )
    .with_source(DiscoverySource::Mdns)
    .with_network_interface("fixture-lan0")
    .with_metadata("fixture", "hue_bridge_discovery_worker_schedule")
    .with_metadata("smart_home.discovery.service_type", HUE_MDNS_SERVICE_TYPE)
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

pub fn room_scene(id: &'static str, entity_id: &EntityId) -> Scene {
    Scene {
        scene_id: SceneId::trusted(id),
        scope: SceneScope::Room,
        native_ref: Some(protocol_id(ProtocolFamily::Hue, "scene", id)),
        actions: vec![SceneAction {
            entity_id: entity_id.clone(),
            desired_state: Value::Object(vec![
                ("light.on_off".to_string(), Value::Bool(true)),
                ("light.brightness".to_string(), Value::Percentage(80)),
            ]),
        }],
        metadata: vec![
            Metadata::new("fixture", "room_scene"),
            Metadata::new("fixture.room_id", "kitchen"),
        ],
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
    for scene in fixture.scenes() {
        registry.upsert_scene(scene.clone())?;
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
    for scene in fixture.scenes() {
        runtime.upsert_scene(scene.clone())?;
    }
    Ok(runtime)
}

pub fn hue_lighting_runtime() -> SmartHomeRuntime {
    runtime_with_fixture(&SmartHomeFixture::hue_lighting())
        .expect("hue lighting fixture records are internally consistent")
}

pub fn hue_discovery_runtime() -> SmartHomeRuntime {
    let mut runtime = SmartHomeRuntime::new();
    runtime
        .register_discovery_worker_schedule(hue_bridge_discovery_worker_schedule(950, 5_000, 250))
        .expect("Hue discovery fixture worker schedule can be registered");
    let run = hue_bridge_discovery_worker_run("001788fffeabcdef", 950, 1_000);
    runtime
        .record_scheduled_discovery_worker_run(&run, 1_000, 1_000)
        .expect("Hue discovery fixture worker run can advance its schedule");
    runtime
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
    use hue_core::{
        hue_application_credentials_from_registration_response,
        hue_pairing_plan_for_discovered_bridge, hue_pairing_registration_request_plan,
        DiscoveredHueBridge,
    };
    use smart_home_core::{AgentId, VaultRef};
    use smart_home_discovery::{DiscoverySource, DiscoveryWorkerRunStatus, PairingRequirement};
    use smart_home_local_http::LocalHttpEndpoint;
    use smart_home_runtime::{
        PairingSessionStatus, RuntimeEvent, RuntimePairingCompletion, RuntimePairingSession,
        RuntimePairingSessionId, WorkerStatus,
    };

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
    fn fake_event_stream_summary_counts_pending_steps_without_consuming() {
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
            .push_gap(3, 1_150)
            .push_disconnect("test disconnect", 1_200);

        assert_eq!(
            stream.summary(),
            FakeEventStreamSummary {
                total_steps: 4,
                device_events: 1,
                disconnects: 1,
                gaps: 2,
                missing_events: 5,
                first_observed_at_ms: Some(1_000),
                last_observed_at_ms: Some(1_200),
            }
        );
        assert_eq!(stream.len(), 4);
        assert!(stream.summary().has_gaps());
        assert!(stream.summary().has_disconnects());
        assert!(!stream.summary().is_empty());

        assert_eq!(
            stream.next_step(),
            Some(ScriptedEvent::Event(Box::new(event)))
        );

        assert_eq!(
            stream.summary(),
            FakeEventStreamSummary {
                total_steps: 3,
                device_events: 0,
                disconnects: 1,
                gaps: 2,
                missing_events: 5,
                first_observed_at_ms: Some(1_100),
                last_observed_at_ms: Some(1_200),
            }
        );
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

        assert_eq!(driver.stream_summary().total_steps, 3);

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
        assert_eq!(registry.counts().protocol_identifiers, 3);
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
    fn hue_discovery_fixture_records_unpaired_bridge_candidate() {
        let runtime = hue_discovery_runtime();
        let bridge_id = BridgeId::trusted("hue.bridge.001788fffeabcdef");
        let bridge = runtime.registry().bridge(&bridge_id).unwrap();
        let record = runtime
            .discovery()
            .get(&IntegrationId::trusted("hue"), "001788fffeabcdef")
            .unwrap();
        let worker = runtime
            .discovery_worker_schedule(&DiscoveryWorkerId::trusted("fixture-hue-discovery-worker"))
            .unwrap();

        assert_eq!(runtime.discovery_record_count(), 1);
        assert_eq!(bridge.health, Health::Unpaired);
        assert_eq!(bridge.address.as_deref(), Some("https://192.0.2.10"));
        assert_eq!(bridge.transport, BridgeTransport::LanHttp);
        assert_eq!(bridge.hardware_model.as_deref(), Some("BSB002"));
        assert_eq!(record.source, DiscoverySource::Mdns);
        assert_eq!(
            record.pairing_requirement,
            PairingRequirement::PhysicalPresence
        );
        assert!(bridge.metadata.iter().any(|metadata| {
            metadata.key == "fixture" && metadata.value == "hue_bridge_discovery"
        }));
        assert_eq!(worker.status, WorkerStatus::Running);
        assert_eq!(
            worker.last_run_status,
            Some(DiscoveryWorkerRunStatus::Completed)
        );
        assert_eq!(worker.total_run_count, 1);
        assert_eq!(worker.next_due_at_ms, 6_000);
    }

    #[test]
    fn hue_discovery_worker_fixture_reports_canonical_mdns_records() {
        let run = hue_bridge_discovery_worker_run("001788fffediscovered", 975, 1_000);

        assert_eq!(run.worker_id.as_str(), "fixture-hue-discovery-worker");
        assert_eq!(run.len(), 1);
        assert!(!run.has_failures());
        assert_eq!(run.records[0].native_bridge_id, "001788fffediscovered");
        assert_eq!(run.records[0].source, DiscoverySource::Mdns);
        assert!(run.records[0].metadata.iter().any(|metadata| {
            metadata.key == "fixture" && metadata.value == "hue_bridge_discovery"
        }));
        assert!(run.metadata.iter().any(|metadata| {
            metadata.key == "fixture" && metadata.value == "hue_bridge_discovery_worker"
        }));
        assert!(run.metadata.iter().any(|metadata| {
            metadata.key == "hue.discovery.scan_datagram_count" && metadata.value == "1"
        }));
        assert!(run.metadata.iter().any(|metadata| {
            metadata.key == "hue.discovery.scan_report" && metadata.value == "true"
        }));
    }

    #[test]
    fn hue_mdns_scan_report_fixture_reports_interface_scope() {
        let report = hue_bridge_mdns_scan_report("001788fffereport", 975, 1_000);
        let aggregate = report.aggregate_result();

        assert_eq!(report.worker_id.as_str(), "fixture-hue-discovery-worker");
        assert_eq!(report.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(report.completed_scan_count(), 1);
        assert_eq!(report.failed_scan_count(), 0);
        assert_eq!(report.datagram_count(), 1);
        assert_eq!(report.advertisement_count(), 1);
        assert_eq!(aggregate.len(), 1);
        assert_eq!(
            report.successes[0].request.network_interface,
            "fixture-lan0"
        );
        assert_eq!(report.successes[0].request.network, MdnsScanNetwork::Ipv4);
        assert_eq!(
            aggregate.advertisements[0].txt_value("bridgeid"),
            Some("001788fffereport")
        );
    }

    #[test]
    fn hue_discovery_worker_schedule_fixture_reports_due_mdns_scope() {
        let schedule = hue_bridge_discovery_worker_schedule(1_000, 5_000, 250);
        let mut runtime = SmartHomeRuntime::new();
        runtime
            .register_discovery_worker_schedule(schedule)
            .expect("fixture schedule can be registered");

        let idle = runtime.discovery_worker_run_plan_at(999);
        let due = runtime.discovery_worker_run_plan_at(1_000);
        let mdns_scan_plan = runtime
            .discovery_mdns_scan_plan_at(1_000)
            .expect("fixture mDNS scan plan can be projected");
        let worker = runtime
            .discovery_worker_schedule(&DiscoveryWorkerId::trusted("fixture-hue-discovery-worker"))
            .unwrap();

        assert!(idle.is_empty());
        assert_eq!(due.len(), 1);
        assert!(matches!(
            due.instructions.as_slice(),
            [instruction] if instruction.integration_id == IntegrationId::trusted("hue")
                && instruction.kind == DiscoveryWorkerKind::MdnsScan
                && instruction.sources == vec![DiscoverySource::Mdns]
                && instruction.network_interfaces == vec!["fixture-lan0".to_string()]
                && instruction.run_timeout_ms == 250
                && instruction.metadata.iter().any(|metadata| {
                    metadata.key == "smart_home.discovery.service_type"
                        && metadata.value == HUE_MDNS_SERVICE_TYPE
                })
        ));
        assert_eq!(mdns_scan_plan.len(), 2);
        assert!(matches!(
            mdns_scan_plan.requests.as_slice(),
            [ipv4, ipv6] if ipv4.network_interface == "fixture-lan0"
                && ipv4.network == MdnsScanNetwork::Ipv4
                && ipv4.service_type == HUE_MDNS_SERVICE_TYPE
                && ipv4.timeout == Duration::from_millis(250)
                && ipv6.network_interface == "fixture-lan0"
                && ipv6.network == MdnsScanNetwork::Ipv6
        ));
        assert_eq!(worker.interval_ms, 5_000);
        assert_eq!(worker.status, WorkerStatus::Starting);
    }

    #[test]
    fn scripted_mdns_worker_executor_runs_runtime_scan_plan() {
        let schedule = hue_bridge_discovery_worker_schedule(1_000, 5_000, 250);
        let worker_id = DiscoveryWorkerId::trusted("fixture-hue-discovery-worker");
        let mut runtime = SmartHomeRuntime::new();
        runtime
            .register_discovery_worker_schedule(schedule)
            .expect("fixture schedule can be registered");
        let plan = runtime
            .discovery_mdns_scan_plan_at(1_000)
            .expect("fixture mDNS scan plan can be projected");
        let mut executor = ScriptedMdnsWorkerScanExecutor::new([
            Ok(hue_bridge_mdns_scan_result("001788fffescripted", 1_000)),
            Err(DiscoveryError::MdnsTransport {
                message: "IPv6 multicast route is unavailable".to_string(),
            }),
        ]);

        let reports = smart_home_discovery::run_mdns_worker_scan_plan_with_executor(
            &plan,
            1_000,
            1_030,
            &mut executor,
        )
        .expect("scripted executor can run the fixture mDNS scan plan");
        let run = hue_discovery_worker_run_from_mdns_scan_report(&reports[0])
            .expect("scripted Hue report can be converted to a worker run");
        let summary = runtime
            .record_scheduled_discovery_worker_run(&run, 1_030, 500)
            .expect("scripted discovery worker run can be recorded");
        let worker = runtime.discovery_worker_schedule(&worker_id).unwrap();

        assert_eq!(executor.pending_outcome_count(), 0);
        assert_eq!(executor.observed_requests().len(), 2);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].completed_scan_count(), 1);
        assert_eq!(reports[0].failed_scan_count(), 1);
        assert_eq!(reports[0].advertisement_count(), 1);
        assert_eq!(summary.status, DiscoveryWorkerRunStatus::Partial);
        assert_eq!(summary.record_count, 1);
        assert_eq!(summary.failure_count, 1);
        assert_eq!(summary.inserted_count, 1);
        assert_eq!(runtime.discovery_record_count(), 1);
        assert_eq!(worker.status, WorkerStatus::Unhealthy);
        assert_eq!(
            worker.last_run_status,
            Some(DiscoveryWorkerRunStatus::Partial)
        );
        assert_eq!(worker.consecutive_failure_count, 1);
        assert_eq!(worker.next_due_at_ms, 6_030);
    }

    #[test]
    fn hue_mdns_scan_fixture_reports_canonical_advertisements() {
        let scan = hue_bridge_mdns_scan_result("001788fffescan", 1_000);

        assert_eq!(scan.service_type, HUE_MDNS_SERVICE_TYPE);
        assert_eq!(scan.datagram_count, 1);
        assert_eq!(scan.len(), 1);
        assert!(!scan.has_failures());
        assert_eq!(
            scan.advertisements[0].txt_value("bridgeid"),
            Some("001788fffescan")
        );
        assert_eq!(scan.advertisements[0].preferred_address(), "192.0.2.10");
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
    fn fake_mqtt_broker_summary_counts_pending_publications_without_payloads() {
        let retained_state = ScriptedMqttPublication::new(
            "home/kitchen/light/state",
            br#"{"state":"ON"}"#.to_vec(),
            1_000,
        )
        .retained()
        .with_metadata("fixture", "mqtt")
        .with_metadata("entity", "kitchen");
        let availability =
            ScriptedMqttPublication::new("home/kitchen/light/availability", Vec::new(), 950);
        let live_state = ScriptedMqttPublication::new(
            "home/office/light/state",
            br#"{"state":"OFF"}"#.to_vec(),
            1_200,
        )
        .with_metadata("fixture", "mqtt");
        let expected_payload_bytes =
            retained_state.payload.len() + availability.payload.len() + live_state.payload.len();
        let expected_metadata_entries =
            retained_state.metadata.len() + availability.metadata.len() + live_state.metadata.len();
        let mut broker = FakeMqttBroker::new()
            .publish(retained_state.clone())
            .publish(availability.clone())
            .publish(live_state.clone());

        assert_eq!(
            broker.summary(),
            FakeMqttBrokerSummary {
                total_publications: 3,
                retained_publications: 1,
                live_publications: 2,
                total_payload_bytes: expected_payload_bytes,
                metadata_entries: expected_metadata_entries,
                earliest_observed_at_ms: Some(950),
                latest_observed_at_ms: Some(1_200),
            }
        );
        assert!(broker.summary().has_retained());
        assert!(broker.summary().has_payloads());
        assert!(!broker.summary().is_empty());
        assert_eq!(broker.len(), 3);
        assert_eq!(broker.next_publication(), Some(retained_state));
        assert_eq!(broker.next_publication(), Some(availability));
        assert_eq!(broker.next_publication(), Some(live_state));

        let empty = FakeMqttBroker::new().summary();

        assert!(empty.is_empty());
        assert!(!empty.has_retained());
        assert!(!empty.has_payloads());
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
    fn fake_mqtt_subscriptions_deliver_matching_publications_without_consuming_queue() {
        let retained_kitchen_state = ScriptedMqttPublication::new(
            "home/kitchen/light/state",
            br#"{"state":"ON"}"#.to_vec(),
            1_000,
        )
        .retained();
        let command = ScriptedMqttPublication::new(
            "home/kitchen/light/set",
            br#"{"state":"OFF"}"#.to_vec(),
            1_050,
        );
        let office_state = ScriptedMqttPublication::new(
            "home/office/light/state",
            br#"{"state":"OFF"}"#.to_vec(),
            1_100,
        );
        let broker = FakeMqttBroker::new()
            .publish(retained_kitchen_state.clone())
            .publish(command)
            .publish(office_state.clone());
        let subscription = ScriptedMqttSubscription::new(
            "sub-light-states",
            MqttTopicFilter::new("home/+/light/state").unwrap(),
        )
        .with_qos(MqttQos::ExactlyOnce)
        .with_metadata("fixture", "subscription");

        let deliveries = broker.deliveries_for_subscription(&subscription).unwrap();

        assert_eq!(
            deliveries
                .iter()
                .map(|delivery| delivery.topic())
                .collect::<Vec<_>>(),
            vec!["home/kitchen/light/state", "home/office/light/state"]
        );
        assert_eq!(deliveries[0].qos(), MqttQos::ExactlyOnce);
        assert!(deliveries[0].retained());
        assert!(!deliveries[1].retained());
        assert_eq!(
            deliveries[0].subscription.subscription_id,
            "sub-light-states"
        );
        assert_eq!(deliveries[0].subscription.metadata.len(), 1);
        assert_eq!(broker.len(), 3);
        assert_eq!(deliveries[0].publication, &retained_kitchen_state);
        assert_eq!(deliveries[1].publication, &office_state);
    }

    #[test]
    fn fake_mqtt_subscription_delivery_surfaces_invalid_publication_topics() {
        let broker = FakeMqttBroker::new().publish(ScriptedMqttPublication::new(
            "home/+/light/state",
            br#"{"state":"ON"}"#.to_vec(),
            1_000,
        ));
        let subscription =
            ScriptedMqttSubscription::new("sub-all", MqttTopicFilter::new("home/#").unwrap());

        assert_eq!(
            broker
                .deliveries_for_subscription(&subscription)
                .unwrap_err(),
            MqttTopicError::TopicNameContainsWildcard
        );
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
    fn hue_pairing_fixture_completes_runtime_session_without_secret_metadata() {
        let pairing_plan = hue_pairing_plan_for_discovered_bridge(
            DiscoveredHueBridge {
                bridge_id: "001788fffeabcdef".to_string(),
                address: "https://192.0.2.10".to_string(),
                hardware_model: Some("BSB002".to_string()),
                firmware_version: Some("1.66.1960062030".to_string()),
            },
            "chief-of-staff",
            "desktop",
        );
        let endpoint =
            LocalHttpEndpoint::hue_bridge(pairing_plan.bridge_id().clone(), "192.0.2.10")
                .unwrap()
                .accept_invalid_certs(true);
        let request_plan = hue_pairing_registration_request_plan(&pairing_plan, &endpoint).unwrap();
        let response = ScriptedLocalHttpResponse::ok_json(
            LocalHttpMethod::Post,
            request_plan.url.clone(),
            br#"[{"success":{"username":"raw-hue-application-key","clientkey":"client-key-1"}}]"#
                .to_vec(),
            1_250,
        );
        let mut server = FakeLocalHttpServer::new().push_response(response);

        let matched = server.respond_to_plan(&request_plan).unwrap();
        let credentials =
            hue_application_credentials_from_registration_response(&matched.body).unwrap();
        let vault_payload = credentials.vault_secret_json();
        let vault_payload_text = std::str::from_utf8(&vault_payload).unwrap();
        assert!(vault_payload_text.contains("raw-hue-application-key"));

        let handoff = credentials.vault_handoff(
            &pairing_plan,
            VaultRef::trusted("vault://smart-home/hue/001788fffeabcdef/application-key"),
            matched.observed_at_ms,
        );
        assert!(handoff.metadata.iter().all(|metadata| {
            !metadata.value.contains("raw-hue-application-key")
                && !metadata.value.contains("client-key-1")
        }));

        let mut runtime = SmartHomeRuntime::new();
        let mut unpaired_bridge = pairing_plan.bridge.clone();
        unpaired_bridge.health = Health::Unpaired;
        runtime.upsert_bridge(unpaired_bridge).unwrap();
        let session_id = RuntimePairingSessionId::trusted("hue-pairing-1");
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                session_id.clone(),
                &pairing_plan.bridge,
                AgentId::trusted("chief-agent"),
                1_200,
                2_000,
                vec![Metadata::new("pairing_kind", "hue_link_button")],
            ))
            .unwrap();

        let completed = runtime
            .complete_pairing_session_with(
                RuntimePairingCompletion::new(
                    session_id.clone(),
                    handoff.vault_ref.clone(),
                    matched.observed_at_ms,
                )
                .with_metadata(handoff.metadata.clone()),
            )
            .unwrap();
        let bridge = runtime.registry().bridge(pairing_plan.bridge_id()).unwrap();

        assert_eq!(completed.status, PairingSessionStatus::Completed);
        assert_eq!(completed.vault_ref, Some(handoff.vault_ref.clone()));
        assert!(completed.metadata.iter().any(|metadata| {
            metadata.key == "hue.pairing.credential_kind" && metadata.value == "application_key"
        }));
        assert_eq!(bridge.health, Health::Online);
        assert_eq!(bridge.auth_ref.as_ref(), Some(&handoff.vault_ref));
        assert!(matches!(
            runtime.event_bus().published(),
            [RuntimeEvent::Device(event), RuntimeEvent::BridgeHealth { bridge_id, health, .. }]
                if event.event_type == DeviceEventType::Health
                    && bridge_id == pairing_plan.bridge_id()
                    && *health == Health::Online
                    && event.metadata.iter().any(|metadata| {
                        metadata.key == "hue.pairing.phase"
                            && metadata.value == "credential_stored"
                    })
                    && event.metadata.iter().all(|metadata| {
                        !metadata.value.contains("raw-hue-application-key")
                            && !metadata.value.contains("client-key-1")
                    })
        ));
    }

    #[test]
    fn fake_local_http_server_summary_counts_pending_responses_without_bodies() {
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
        let delete_ack = ScriptedLocalHttpResponse::new(
            LocalHttpMethod::Delete,
            "https://192.0.2.10/clip/v2/resource/scene/scene-1",
            204,
            Vec::new(),
            1_150,
        );
        let expected_body_bytes = light_state.body.len()
            + command_accept.body.len()
            + bridge_busy.body.len()
            + delete_ack.body.len();
        let expected_metadata_entries = light_state.metadata.len()
            + command_accept.metadata.len()
            + bridge_busy.metadata.len()
            + delete_ack.metadata.len();
        let mut server = FakeLocalHttpServer::new()
            .push_response(light_state.clone())
            .push_response(command_accept.clone())
            .push_response(bridge_busy.clone())
            .push_response(delete_ack.clone());

        assert_eq!(
            server.summary(),
            FakeLocalHttpServerSummary {
                total_responses: 4,
                get_responses: 2,
                post_responses: 0,
                put_responses: 1,
                patch_responses: 0,
                delete_responses: 1,
                informational_responses: 0,
                success_responses: 3,
                redirect_responses: 0,
                client_error_responses: 0,
                server_error_responses: 1,
                other_status_responses: 0,
                body_bearing_responses: 3,
                bodyless_responses: 1,
                total_body_bytes: expected_body_bytes,
                metadata_entries: expected_metadata_entries,
                earliest_observed_at_ms: Some(1_000),
                latest_observed_at_ms: Some(1_200),
            }
        );
        assert!(server.summary().has_errors());
        assert!(server.summary().has_mutations());
        assert!(!server.summary().is_empty());
        assert_eq!(server.len(), 4);
        assert_eq!(server.next_response(), Some(light_state));
        assert_eq!(server.next_response(), Some(command_accept));
        assert_eq!(server.next_response(), Some(bridge_busy));
        assert_eq!(server.next_response(), Some(delete_ack));

        let empty = FakeLocalHttpServer::new().summary();

        assert!(empty.is_empty());
        assert!(!empty.has_errors());
        assert!(!empty.has_mutations());
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
