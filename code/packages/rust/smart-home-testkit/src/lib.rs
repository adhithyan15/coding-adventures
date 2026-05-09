//! Deterministic smart-home fixtures and fake streams for D23 tests.
//!
//! This crate is pure data. It gives runtime and integration packages reusable
//! fixtures without opening sockets, touching radios, reading files, or calling
//! cloud APIs.

#![forbid(unsafe_code)]

use smart_home_core::{
    Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CorrelationId, Device,
    DeviceEvent, DeviceEventType, DeviceId, Entity, EntityId, EntityKind, EventId, Health,
    IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, StateConfidence, StateDelta,
    StateSnapshot, StateSource, Value,
};
use smart_home_registry::{InMemorySmartHomeRegistry, RegistryError};
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
