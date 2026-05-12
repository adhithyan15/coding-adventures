//! In-memory smart-home registry for normalized D23 records.
//!
//! This crate is the first registry slice: it stores bridge, device, entity,
//! scene, state, event, and protocol-id indexes without any filesystem, Vault,
//! actor, network, serial, or radio access. Durable D18A-backed storage can sit
//! behind the same operations later.

#![forbid(unsafe_code)]

use smart_home_core::{
    AgentId, AuthorizationDecision, AuthorizationOutcome, Bridge, BridgeId, BridgeTransport,
    CapabilityGrant, CapabilityGrantId, CapabilityGrantStatus, CapabilityId, Device, DeviceEvent,
    DeviceEventType, DeviceId, Entity, EntityId, EntityKind, EventId, Health, IntegrationId,
    ProtocolFamily, ProtocolIdentifier, Scene, SceneId, SceneScope, StateConfidence, StateSnapshot,
    StateSource, Value, ValueKind,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryTarget {
    Bridge(BridgeId),
    Device(DeviceId),
    Entity(EntityId),
    Scene(SceneId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    UnknownBridge(BridgeId),
    UnknownDevice(DeviceId),
    UnknownEntity(EntityId),
    UnknownScene(SceneId),
    UnknownCapabilityGrant(CapabilityGrantId),
    DuplicateEvent(EventId),
    EventBridgeMismatch {
        event_id: EventId,
        bridge_id: BridgeId,
    },
    EventDeviceMismatch {
        event_id: EventId,
        device_id: DeviceId,
    },
    EventEntityMismatch {
        event_id: EventId,
        entity_id: EntityId,
    },
    ProtocolIdentifierConflict {
        family: String,
        kind: String,
        value: String,
        existing: Box<RegistryTarget>,
        attempted: Box<RegistryTarget>,
    },
    DuplicateRefreshSnapshot(EntityId),
    UnexpectedRefreshSnapshot(EntityId),
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBridge(id) => write!(f, "unknown smart-home bridge {id}"),
            Self::UnknownDevice(id) => write!(f, "unknown smart-home device {id}"),
            Self::UnknownEntity(id) => write!(f, "unknown smart-home entity {id}"),
            Self::UnknownScene(id) => write!(f, "unknown smart-home scene {id}"),
            Self::UnknownCapabilityGrant(id) => {
                write!(f, "unknown smart-home capability grant {id}")
            }
            Self::DuplicateEvent(id) => write!(f, "duplicate smart-home event {id}"),
            Self::EventBridgeMismatch {
                event_id,
                bridge_id,
            } => write!(
                f,
                "event {event_id} references unknown bridge {bridge_id}"
            ),
            Self::EventDeviceMismatch {
                event_id,
                device_id,
            } => write!(
                f,
                "event {event_id} references unknown device {device_id}"
            ),
            Self::EventEntityMismatch {
                event_id,
                entity_id,
            } => write!(
                f,
                "event {event_id} references unknown entity {entity_id}"
            ),
            Self::ProtocolIdentifierConflict {
                family,
                kind,
                value,
                existing,
                attempted,
            } => write!(
                f,
                "protocol identifier {family}:{kind}:{value} already maps to {existing:?}, not {attempted:?}"
            ),
            Self::DuplicateRefreshSnapshot(id) => {
                write!(f, "duplicate refresh snapshot for entity {id}")
            }
            Self::UnexpectedRefreshSnapshot(id) => {
                write!(f, "refresh snapshot for entity {id} was not in the refresh plan")
            }
        }
    }
}

impl std::error::Error for RegistryError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistryCounts {
    pub bridges: usize,
    pub devices: usize,
    pub entities: usize,
    pub scenes: usize,
    pub states: usize,
    pub events: usize,
    pub protocol_identifiers: usize,
    pub capability_grants: usize,
    pub authorization_decisions: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegistrySupervisionSummary {
    pub generated_at_ms: u64,
    pub bridges: usize,
    pub attention_bridges: usize,
    pub pairing_candidate_bridges: usize,
    pub devices: usize,
    pub online_devices: usize,
    pub attention_devices: usize,
    pub pairing_candidate_devices: usize,
    pub entities: usize,
    pub state_snapshots: usize,
    pub missing_entity_states: usize,
    pub stale_entity_states: usize,
    pub refresh_targets: usize,
    pub events: usize,
}

impl RegistrySupervisionSummary {
    pub fn has_attention_items(&self) -> bool {
        self.attention_bridges > 0 || self.attention_devices > 0 || self.refresh_targets > 0
    }

    pub fn has_refresh_work(&self) -> bool {
        self.refresh_targets > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeSummary {
    pub bridge_id: BridgeId,
    pub integration_id: IntegrationId,
    pub transport: BridgeTransport,
    pub health: Health,
    pub last_seen_at_ms: Option<u64>,
    pub device_count: usize,
    pub entity_count: usize,
    pub protocol_identifier_count: usize,
    pub metadata_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSummary {
    pub device_id: DeviceId,
    pub bridge_id: BridgeId,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub health: Health,
    pub entity_count: usize,
    pub capability_count: usize,
    pub state_count: usize,
    pub protocol_identifier_count: usize,
    pub metadata_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitySummary {
    pub entity_id: EntityId,
    pub device_id: DeviceId,
    pub kind: EntityKind,
    pub name: String,
    pub capability_ids: Vec<CapabilityId>,
    pub has_state: bool,
    pub state_value_kind: Option<ValueKind>,
    pub state_source: Option<StateSource>,
    pub state_confidence: Option<StateConfidence>,
    pub state_observed_at_ms: Option<u64>,
    pub state_received_at_ms: Option<u64>,
    pub state_expires_at_ms: Option<u64>,
    pub metadata_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneSummary {
    pub scene_id: SceneId,
    pub scope: SceneScope,
    pub action_count: usize,
    pub has_native_ref: bool,
    pub metadata_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StateFreshness {
    #[default]
    Any,
    Present,
    Missing,
    FreshAt(u64),
    StaleAt(u64),
    NeedsRefreshAt(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateRefreshReason {
    Missing,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRefreshTarget {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
    pub kind: EntityKind,
    pub capabilities: Vec<CapabilityId>,
    pub reason: StateRefreshReason,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRefreshPlan {
    pub generated_at_ms: u64,
    pub targets: Vec<StateRefreshTarget>,
}

impl StateRefreshPlan {
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn len(&self) -> usize {
        self.targets.len()
    }

    pub fn targets_for_bridge(&self, bridge_id: &BridgeId) -> Vec<&StateRefreshTarget> {
        self.targets
            .iter()
            .filter(|target| &target.bridge_id == bridge_id)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateRefreshReport {
    pub generated_at_ms: u64,
    pub completed_at_ms: u64,
    pub refreshed: Vec<EntityId>,
    pub missing: Vec<EntityId>,
}

impl StateRefreshReport {
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty()
    }

    pub fn refreshed_count(&self) -> usize {
        self.refreshed.len()
    }

    pub fn missing_count(&self) -> usize {
        self.missing.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeviceSelector {
    pub bridge_id: Option<BridgeId>,
    pub health: Option<Health>,
    pub capability_id: Option<CapabilityId>,
}

impl DeviceSelector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_bridge(mut self, bridge_id: BridgeId) -> Self {
        self.bridge_id = Some(bridge_id);
        self
    }

    pub fn with_health(mut self, health: Health) -> Self {
        self.health = Some(health);
        self
    }

    pub fn with_capability(mut self, capability_id: CapabilityId) -> Self {
        self.capability_id = Some(capability_id);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EntitySelector {
    pub bridge_id: Option<BridgeId>,
    pub device_id: Option<DeviceId>,
    pub kind: Option<EntityKind>,
    pub capability_id: Option<CapabilityId>,
    pub device_health: Option<Health>,
    pub state_freshness: StateFreshness,
}

impl EntitySelector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_bridge(mut self, bridge_id: BridgeId) -> Self {
        self.bridge_id = Some(bridge_id);
        self
    }

    pub fn for_device(mut self, device_id: DeviceId) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn with_kind(mut self, kind: EntityKind) -> Self {
        self.kind = Some(kind);
        self
    }

    pub fn with_capability(mut self, capability_id: CapabilityId) -> Self {
        self.capability_id = Some(capability_id);
        self
    }

    pub fn with_device_health(mut self, health: Health) -> Self {
        self.device_health = Some(health);
        self
    }

    pub fn with_state_freshness(mut self, state_freshness: StateFreshness) -> Self {
        self.state_freshness = state_freshness;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuthorizationDecisionSelector {
    pub principal_id: Option<AgentId>,
    pub outcome: Option<AuthorizationOutcome>,
}

impl AuthorizationDecisionSelector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_principal(mut self, principal_id: AgentId) -> Self {
        self.principal_id = Some(principal_id);
        self
    }

    pub fn with_outcome(mut self, outcome: AuthorizationOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryAccessMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, Copy)]
pub struct SmartHomeRegistryReadView<'a> {
    registry: &'a InMemorySmartHomeRegistry,
}

impl<'a> SmartHomeRegistryReadView<'a> {
    pub fn access_mode(&self) -> RegistryAccessMode {
        RegistryAccessMode::ReadOnly
    }

    pub fn counts(&self) -> RegistryCounts {
        self.registry.counts()
    }

    pub fn supervision_summary_at(&self, now_ms: u64) -> RegistrySupervisionSummary {
        self.registry.supervision_summary_at(now_ms)
    }

    pub fn bridge(&self, bridge_id: &BridgeId) -> Option<&'a Bridge> {
        self.registry.bridge(bridge_id)
    }

    pub fn bridges(&self) -> impl Iterator<Item = &'a Bridge> {
        self.registry.bridges()
    }

    pub fn bridge_summary(&self, bridge_id: &BridgeId) -> Option<BridgeSummary> {
        self.registry.bridge_summary(bridge_id)
    }

    pub fn bridge_summaries(&self) -> Vec<BridgeSummary> {
        self.registry.bridge_summaries()
    }

    pub fn device(&self, device_id: &DeviceId) -> Option<&'a Device> {
        self.registry.device(device_id)
    }

    pub fn devices(&self) -> impl Iterator<Item = &'a Device> {
        self.registry.devices()
    }

    pub fn devices_for_bridge(&self, bridge_id: &BridgeId) -> impl Iterator<Item = &'a Device> {
        self.registry.devices_for_bridge(bridge_id)
    }

    pub fn device_summary(&self, device_id: &DeviceId) -> Option<DeviceSummary> {
        self.registry.device_summary(device_id)
    }

    pub fn device_summaries(&self) -> Vec<DeviceSummary> {
        self.registry.device_summaries()
    }

    pub fn entity(&self, entity_id: &EntityId) -> Option<&'a Entity> {
        self.registry.entity(entity_id)
    }

    pub fn entities(&self) -> impl Iterator<Item = &'a Entity> {
        self.registry.entities()
    }

    pub fn entities_for_device(&self, device_id: &DeviceId) -> impl Iterator<Item = &'a Entity> {
        self.registry.entities_for_device(device_id)
    }

    pub fn entity_summary(&self, entity_id: &EntityId) -> Option<EntitySummary> {
        self.registry.entity_summary(entity_id)
    }

    pub fn entity_summaries(&self) -> Vec<EntitySummary> {
        self.registry.entity_summaries()
    }

    pub fn scene(&self, scene_id: &SceneId) -> Option<&'a Scene> {
        self.registry.scene(scene_id)
    }

    pub fn scenes(&self) -> impl Iterator<Item = &'a Scene> {
        self.registry.scenes()
    }

    pub fn scene_summary(&self, scene_id: &SceneId) -> Option<SceneSummary> {
        self.registry.scene_summary(scene_id)
    }

    pub fn scene_summaries(&self) -> Vec<SceneSummary> {
        self.registry.scene_summaries()
    }

    pub fn state(&self, entity_id: &EntityId) -> Option<&'a StateSnapshot> {
        self.registry.state(entity_id)
    }

    pub fn states(&self) -> impl Iterator<Item = &'a StateSnapshot> {
        self.registry.states()
    }

    pub fn event(&self, event_id: &EventId) -> Option<&'a DeviceEvent> {
        self.registry.event(event_id)
    }

    pub fn events(&self) -> impl Iterator<Item = &'a DeviceEvent> {
        self.registry.events()
    }

    pub fn capability_grant(&self, grant_id: &CapabilityGrantId) -> Option<&'a CapabilityGrant> {
        self.registry.capability_grant(grant_id)
    }

    pub fn capability_grants(&self) -> impl Iterator<Item = &'a CapabilityGrant> {
        self.registry.capability_grants()
    }

    pub fn capability_grants_for_principal(
        &self,
        principal_id: &AgentId,
    ) -> Vec<&'a CapabilityGrant> {
        self.registry.capability_grants_for_principal(principal_id)
    }

    pub fn active_capability_grants_for_principal_at(
        &self,
        principal_id: &AgentId,
        now_ms: u64,
    ) -> Vec<&'a CapabilityGrant> {
        self.registry
            .active_capability_grants_for_principal_at(principal_id, now_ms)
    }

    pub fn authorization_decisions(&self) -> impl Iterator<Item = &'a AuthorizationDecision> {
        self.registry.authorization_decisions()
    }

    pub fn authorization_decisions_for_principal(
        &self,
        principal_id: &AgentId,
    ) -> Vec<&'a AuthorizationDecision> {
        self.registry
            .authorization_decisions_for_principal(principal_id)
    }

    pub fn query_authorization_decisions(
        &self,
        selector: &AuthorizationDecisionSelector,
    ) -> Vec<&'a AuthorizationDecision> {
        self.registry.query_authorization_decisions(selector)
    }

    pub fn query_devices(&self, selector: &DeviceSelector) -> Vec<&'a Device> {
        self.registry.query_devices(selector)
    }

    pub fn query_device_summaries(&self, selector: &DeviceSelector) -> Vec<DeviceSummary> {
        self.registry.query_device_summaries(selector)
    }

    pub fn query_entities(&self, selector: &EntitySelector) -> Vec<&'a Entity> {
        self.registry.query_entities(selector)
    }

    pub fn query_entity_summaries(&self, selector: &EntitySelector) -> Vec<EntitySummary> {
        self.registry.query_entity_summaries(selector)
    }

    pub fn stale_states_at(&self, now_ms: u64) -> Vec<&'a StateSnapshot> {
        self.registry.stale_states_at(now_ms)
    }

    pub fn state_refresh_plan_at(&self, now_ms: u64) -> StateRefreshPlan {
        self.registry.state_refresh_plan_at(now_ms)
    }

    pub fn lookup_protocol(&self, identifier: &ProtocolIdentifier) -> Option<&'a RegistryTarget> {
        self.registry.lookup_protocol(identifier)
    }

    pub fn bridge_by_protocol(&self, identifier: &ProtocolIdentifier) -> Option<&'a Bridge> {
        self.registry.bridge_by_protocol(identifier)
    }

    pub fn device_by_protocol(&self, identifier: &ProtocolIdentifier) -> Option<&'a Device> {
        self.registry.device_by_protocol(identifier)
    }

    pub fn scene_by_protocol(&self, identifier: &ProtocolIdentifier) -> Option<&'a Scene> {
        self.registry.scene_by_protocol(identifier)
    }
}

pub struct SmartHomeRegistryWriteView<'a> {
    registry: &'a mut InMemorySmartHomeRegistry,
}

impl<'a> SmartHomeRegistryWriteView<'a> {
    pub fn access_mode(&self) -> RegistryAccessMode {
        RegistryAccessMode::ReadWrite
    }

    pub fn as_read(&self) -> SmartHomeRegistryReadView<'_> {
        SmartHomeRegistryReadView {
            registry: self.registry,
        }
    }

    pub fn upsert_bridge(&mut self, bridge: Bridge) -> Result<Option<Bridge>, RegistryError> {
        self.registry.upsert_bridge(bridge)
    }

    pub fn upsert_device(&mut self, device: Device) -> Result<Option<Device>, RegistryError> {
        self.registry.upsert_device(device)
    }

    pub fn upsert_entity(&mut self, entity: Entity) -> Result<Option<Entity>, RegistryError> {
        self.registry.upsert_entity(entity)
    }

    pub fn upsert_scene(&mut self, scene: Scene) -> Result<Option<Scene>, RegistryError> {
        self.registry.upsert_scene(scene)
    }

    pub fn apply_state_snapshot(
        &mut self,
        snapshot: StateSnapshot,
    ) -> Result<Option<StateSnapshot>, RegistryError> {
        self.registry.apply_state_snapshot(snapshot)
    }

    pub fn record_event(&mut self, event: DeviceEvent) -> Result<(), RegistryError> {
        self.registry.record_event(event)
    }

    pub fn upsert_capability_grant(&mut self, grant: CapabilityGrant) -> Option<CapabilityGrant> {
        self.registry.upsert_capability_grant(grant)
    }

    pub fn update_capability_grant_status(
        &mut self,
        grant_id: &CapabilityGrantId,
        status: CapabilityGrantStatus,
    ) -> Result<CapabilityGrant, RegistryError> {
        self.registry
            .update_capability_grant_status(grant_id, status)
    }

    pub fn record_authorization_decision(&mut self, decision: AuthorizationDecision) -> usize {
        self.registry.record_authorization_decision(decision)
    }

    pub fn apply_state_refresh_results<I>(
        &mut self,
        plan: &StateRefreshPlan,
        snapshots: I,
        completed_at_ms: u64,
    ) -> Result<StateRefreshReport, RegistryError>
    where
        I: IntoIterator<Item = StateSnapshot>,
    {
        self.registry
            .apply_state_refresh_results(plan, snapshots, completed_at_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EventSelector {
    pub bridge_id: Option<BridgeId>,
    pub device_id: Option<DeviceId>,
    pub entity_id: Option<EntityId>,
    pub event_type: Option<DeviceEventType>,
    pub observed_at_or_after_ms: Option<u64>,
    pub received_at_or_after_ms: Option<u64>,
    pub limit: Option<usize>,
}

impl EventSelector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_bridge(mut self, bridge_id: BridgeId) -> Self {
        self.bridge_id = Some(bridge_id);
        self
    }

    pub fn for_device(mut self, device_id: DeviceId) -> Self {
        self.device_id = Some(device_id);
        self
    }

    pub fn for_entity(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    pub fn with_event_type(mut self, event_type: DeviceEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn observed_at_or_after(mut self, observed_at_ms: u64) -> Self {
        self.observed_at_or_after_ms = Some(observed_at_ms);
        self
    }

    pub fn received_at_or_after(mut self, received_at_ms: u64) -> Self {
        self.received_at_or_after_ms = Some(received_at_ms);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemorySmartHomeRegistry {
    bridges: BTreeMap<BridgeId, Bridge>,
    devices: BTreeMap<DeviceId, Device>,
    entities: BTreeMap<EntityId, Entity>,
    scenes: BTreeMap<SceneId, Scene>,
    states: BTreeMap<EntityId, StateSnapshot>,
    events: BTreeMap<EventId, DeviceEvent>,
    capability_grants: BTreeMap<CapabilityGrantId, CapabilityGrant>,
    authorization_decisions: Vec<AuthorizationDecision>,
    event_order: Vec<EventId>,
    bridge_devices: BTreeMap<BridgeId, BTreeSet<DeviceId>>,
    device_entities: BTreeMap<DeviceId, BTreeSet<EntityId>>,
    principal_grants: BTreeMap<AgentId, BTreeSet<CapabilityGrantId>>,
    principal_authorization_decisions: BTreeMap<AgentId, Vec<usize>>,
    protocol_index: BTreeMap<ProtocolIndexKey, RegistryTarget>,
}

impl InMemorySmartHomeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read_view(&self) -> SmartHomeRegistryReadView<'_> {
        SmartHomeRegistryReadView { registry: self }
    }

    pub fn write_view(&mut self) -> SmartHomeRegistryWriteView<'_> {
        SmartHomeRegistryWriteView { registry: self }
    }

    pub fn counts(&self) -> RegistryCounts {
        RegistryCounts {
            bridges: self.bridges.len(),
            devices: self.devices.len(),
            entities: self.entities.len(),
            scenes: self.scenes.len(),
            states: self.states.len(),
            events: self.events.len(),
            protocol_identifiers: self.protocol_index.len(),
            capability_grants: self.capability_grants.len(),
            authorization_decisions: self.authorization_decisions.len(),
        }
    }

    pub fn supervision_summary_at(&self, now_ms: u64) -> RegistrySupervisionSummary {
        let counts = self.counts();
        let attention_bridges = self
            .bridges
            .values()
            .filter(|bridge| health_needs_attention(bridge.health))
            .count();
        let pairing_candidate_bridges = self
            .bridges
            .values()
            .filter(|bridge| health_is_pairing_candidate(bridge.health))
            .count();
        let online_devices = self
            .devices
            .values()
            .filter(|device| device.health == Health::Online)
            .count();
        let attention_devices = self
            .devices
            .values()
            .filter(|device| health_needs_attention(device.health))
            .count();
        let pairing_candidate_devices = self
            .devices
            .values()
            .filter(|device| health_is_pairing_candidate(device.health))
            .count();
        let mut missing_entity_states = 0usize;
        let mut stale_entity_states = 0usize;
        for entity in self.entities.values() {
            match self.state(&entity.entity_id) {
                None => missing_entity_states += 1,
                Some(snapshot) if snapshot.is_stale_at(now_ms) => stale_entity_states += 1,
                Some(_) => {}
            }
        }
        let refresh_targets = missing_entity_states + stale_entity_states;

        RegistrySupervisionSummary {
            generated_at_ms: now_ms,
            bridges: counts.bridges,
            attention_bridges,
            pairing_candidate_bridges,
            devices: counts.devices,
            online_devices,
            attention_devices,
            pairing_candidate_devices,
            entities: counts.entities,
            state_snapshots: counts.states,
            missing_entity_states,
            stale_entity_states,
            refresh_targets,
            events: counts.events,
        }
    }

    pub fn upsert_bridge(&mut self, bridge: Bridge) -> Result<Option<Bridge>, RegistryError> {
        let target = RegistryTarget::Bridge(bridge.bridge_id.clone());
        let old_identifiers = self
            .bridges
            .get(&bridge.bridge_id)
            .map(|old| old.identifiers.clone());
        self.replace_protocol_indexes(
            old_identifiers.as_deref(),
            bridge.identifiers.as_slice(),
            &target,
        )?;
        self.bridge_devices
            .entry(bridge.bridge_id.clone())
            .or_default();
        Ok(self.bridges.insert(bridge.bridge_id.clone(), bridge))
    }

    pub fn bridge(&self, bridge_id: &BridgeId) -> Option<&Bridge> {
        self.bridges.get(bridge_id)
    }

    pub fn bridges(&self) -> impl Iterator<Item = &Bridge> {
        self.bridges.values()
    }

    pub fn bridge_summary(&self, bridge_id: &BridgeId) -> Option<BridgeSummary> {
        self.bridge(bridge_id)
            .map(|bridge| self.summarize_bridge(bridge))
    }

    pub fn bridge_summaries(&self) -> Vec<BridgeSummary> {
        self.bridges()
            .map(|bridge| self.summarize_bridge(bridge))
            .collect()
    }

    pub fn upsert_device(&mut self, device: Device) -> Result<Option<Device>, RegistryError> {
        if !self.bridges.contains_key(&device.bridge_id) {
            return Err(RegistryError::UnknownBridge(device.bridge_id));
        }

        let target = RegistryTarget::Device(device.device_id.clone());
        let old_identifiers = self
            .devices
            .get(&device.device_id)
            .map(|old| old.identifiers.clone());
        self.replace_protocol_indexes(
            old_identifiers.as_deref(),
            device.identifiers.as_slice(),
            &target,
        )?;

        if let Some(old) = self.devices.get(&device.device_id) {
            if old.bridge_id != device.bridge_id {
                remove_from_index_set(&mut self.bridge_devices, &old.bridge_id, &old.device_id);
            }
        }
        self.bridge_devices
            .entry(device.bridge_id.clone())
            .or_default()
            .insert(device.device_id.clone());

        Ok(self.devices.insert(device.device_id.clone(), device))
    }

    pub fn device(&self, device_id: &DeviceId) -> Option<&Device> {
        self.devices.get(device_id)
    }

    pub fn devices(&self) -> impl Iterator<Item = &Device> {
        self.devices.values()
    }

    pub fn devices_for_bridge(&self, bridge_id: &BridgeId) -> impl Iterator<Item = &Device> {
        self.bridge_devices
            .get(bridge_id)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.devices.get(id))
    }

    pub fn device_summary(&self, device_id: &DeviceId) -> Option<DeviceSummary> {
        self.device(device_id)
            .map(|device| self.summarize_device(device))
    }

    pub fn device_summaries(&self) -> Vec<DeviceSummary> {
        self.devices()
            .map(|device| self.summarize_device(device))
            .collect()
    }

    pub fn upsert_entity(&mut self, entity: Entity) -> Result<Option<Entity>, RegistryError> {
        if !self.devices.contains_key(&entity.device_id) {
            return Err(RegistryError::UnknownDevice(entity.device_id));
        }

        if let Some(old) = self.entities.get(&entity.entity_id) {
            if old.device_id != entity.device_id {
                remove_from_index_set(&mut self.device_entities, &old.device_id, &old.entity_id);
                if let Some(parent) = self.devices.get_mut(&old.device_id) {
                    parent.entity_ids.retain(|id| id != &old.entity_id);
                }
            }
        }

        self.device_entities
            .entry(entity.device_id.clone())
            .or_default()
            .insert(entity.entity_id.clone());
        if let Some(parent) = self.devices.get_mut(&entity.device_id) {
            push_unique(&mut parent.entity_ids, entity.entity_id.clone());
        }
        if let Some(state) = &entity.state {
            self.states.insert(entity.entity_id.clone(), state.clone());
        }

        Ok(self.entities.insert(entity.entity_id.clone(), entity))
    }

    pub fn entity(&self, entity_id: &EntityId) -> Option<&Entity> {
        self.entities.get(entity_id)
    }

    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.values()
    }

    pub fn entities_for_device(&self, device_id: &DeviceId) -> impl Iterator<Item = &Entity> {
        self.device_entities
            .get(device_id)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.entities.get(id))
    }

    pub fn entity_summary(&self, entity_id: &EntityId) -> Option<EntitySummary> {
        self.entity(entity_id)
            .map(|entity| self.summarize_entity(entity))
    }

    pub fn entity_summaries(&self) -> Vec<EntitySummary> {
        self.entities()
            .map(|entity| self.summarize_entity(entity))
            .collect()
    }

    pub fn upsert_scene(&mut self, scene: Scene) -> Result<Option<Scene>, RegistryError> {
        for action in &scene.actions {
            if !self.entities.contains_key(&action.entity_id) {
                return Err(RegistryError::UnknownEntity(action.entity_id.clone()));
            }
        }

        let old_native_ref = self
            .scenes
            .get(&scene.scene_id)
            .and_then(|old| old.native_ref.as_ref())
            .cloned()
            .into_iter()
            .collect::<Vec<_>>();
        let new_native_ref = scene.native_ref.clone().into_iter().collect::<Vec<_>>();
        let target = RegistryTarget::Scene(scene.scene_id.clone());
        self.replace_protocol_indexes(Some(old_native_ref.as_slice()), &new_native_ref, &target)?;

        Ok(self.scenes.insert(scene.scene_id.clone(), scene))
    }

    pub fn scene(&self, scene_id: &SceneId) -> Option<&Scene> {
        self.scenes.get(scene_id)
    }

    pub fn scenes(&self) -> impl Iterator<Item = &Scene> {
        self.scenes.values()
    }

    pub fn scene_summary(&self, scene_id: &SceneId) -> Option<SceneSummary> {
        self.scene(scene_id).map(summarize_scene)
    }

    pub fn scene_summaries(&self) -> Vec<SceneSummary> {
        self.scenes().map(summarize_scene).collect()
    }

    pub fn apply_state_snapshot(
        &mut self,
        snapshot: StateSnapshot,
    ) -> Result<Option<StateSnapshot>, RegistryError> {
        let entity = self
            .entities
            .get_mut(&snapshot.entity_id)
            .ok_or_else(|| RegistryError::UnknownEntity(snapshot.entity_id.clone()))?;
        entity.state = Some(snapshot.clone());
        Ok(self.states.insert(snapshot.entity_id.clone(), snapshot))
    }

    pub fn state(&self, entity_id: &EntityId) -> Option<&StateSnapshot> {
        self.states.get(entity_id)
    }

    pub fn states(&self) -> impl Iterator<Item = &StateSnapshot> {
        self.states.values()
    }

    pub fn record_event(&mut self, event: DeviceEvent) -> Result<(), RegistryError> {
        if self.events.contains_key(&event.event_id) {
            return Err(RegistryError::DuplicateEvent(event.event_id));
        }
        if !self.bridges.contains_key(&event.bridge_id) {
            return Err(RegistryError::EventBridgeMismatch {
                event_id: event.event_id,
                bridge_id: event.bridge_id,
            });
        }
        if let Some(device_id) = &event.device_id {
            if !self.devices.contains_key(device_id) {
                return Err(RegistryError::EventDeviceMismatch {
                    event_id: event.event_id.clone(),
                    device_id: device_id.clone(),
                });
            }
        }
        if let Some(entity_id) = &event.entity_id {
            if !self.entities.contains_key(entity_id) {
                return Err(RegistryError::EventEntityMismatch {
                    event_id: event.event_id.clone(),
                    entity_id: entity_id.clone(),
                });
            }
        }

        if let (Some(entity_id), Some(delta)) = (&event.entity_id, &event.state_delta) {
            let snapshot = StateSnapshot {
                entity_id: entity_id.clone(),
                value: Value::Object(vec![(
                    delta.capability_id.as_str().to_string(),
                    delta.value.clone(),
                )]),
                source: match event.event_type {
                    DeviceEventType::Discovered
                    | DeviceEventType::Updated
                    | DeviceEventType::Health => StateSource::EventStream,
                    DeviceEventType::Removed
                    | DeviceEventType::Unavailable
                    | DeviceEventType::Error => StateSource::Manual,
                },
                observed_at_ms: event.observed_at_ms,
                received_at_ms: event.received_at_ms,
                expires_at_ms: None,
                confidence: match event.event_type {
                    DeviceEventType::Removed
                    | DeviceEventType::Unavailable
                    | DeviceEventType::Error => StateConfidence::Stale,
                    _ => StateConfidence::Confirmed,
                },
            };
            self.apply_state_snapshot(snapshot)?;
        }

        self.event_order.push(event.event_id.clone());
        self.events.insert(event.event_id.clone(), event);
        Ok(())
    }

    pub fn event(&self, event_id: &EventId) -> Option<&DeviceEvent> {
        self.events.get(event_id)
    }

    pub fn events(&self) -> impl Iterator<Item = &DeviceEvent> {
        self.event_order.iter().filter_map(|id| self.events.get(id))
    }

    pub fn query_events(&self, selector: &EventSelector) -> Vec<&DeviceEvent> {
        let mut events = self
            .events()
            .filter(|event| event_matches_selector(event, selector))
            .collect::<Vec<_>>();
        if let Some(limit) = selector.limit {
            events.truncate(limit);
        }
        events
    }

    pub fn upsert_capability_grant(&mut self, grant: CapabilityGrant) -> Option<CapabilityGrant> {
        let old = self
            .capability_grants
            .insert(grant.grant_id.clone(), grant.clone());
        if let Some(old) = &old {
            if old.principal_id != grant.principal_id {
                remove_from_index_set(&mut self.principal_grants, &old.principal_id, &old.grant_id);
            }
        }
        self.principal_grants
            .entry(grant.principal_id.clone())
            .or_default()
            .insert(grant.grant_id.clone());
        old
    }

    pub fn capability_grant(&self, grant_id: &CapabilityGrantId) -> Option<&CapabilityGrant> {
        self.capability_grants.get(grant_id)
    }

    pub fn capability_grants(&self) -> impl Iterator<Item = &CapabilityGrant> {
        self.capability_grants.values()
    }

    pub fn capability_grants_for_principal(&self, principal_id: &AgentId) -> Vec<&CapabilityGrant> {
        self.principal_grants
            .get(principal_id)
            .into_iter()
            .flat_map(|grant_ids| grant_ids.iter())
            .filter_map(|grant_id| self.capability_grants.get(grant_id))
            .collect()
    }

    pub fn active_capability_grants_for_principal_at(
        &self,
        principal_id: &AgentId,
        now_ms: u64,
    ) -> Vec<&CapabilityGrant> {
        self.capability_grants_for_principal(principal_id)
            .into_iter()
            .filter(|grant| grant.is_active_at(now_ms))
            .collect()
    }

    pub fn update_capability_grant_status(
        &mut self,
        grant_id: &CapabilityGrantId,
        status: CapabilityGrantStatus,
    ) -> Result<CapabilityGrant, RegistryError> {
        let grant = self
            .capability_grants
            .get_mut(grant_id)
            .ok_or_else(|| RegistryError::UnknownCapabilityGrant(grant_id.clone()))?;
        grant.status = status;
        Ok(grant.clone())
    }

    pub fn record_authorization_decision(&mut self, decision: AuthorizationDecision) -> usize {
        let index = self.authorization_decisions.len();
        self.principal_authorization_decisions
            .entry(decision.principal_id.clone())
            .or_default()
            .push(index);
        self.authorization_decisions.push(decision);
        index
    }

    pub fn authorization_decisions(&self) -> impl Iterator<Item = &AuthorizationDecision> {
        self.authorization_decisions.iter()
    }

    pub fn authorization_decisions_for_principal(
        &self,
        principal_id: &AgentId,
    ) -> Vec<&AuthorizationDecision> {
        self.principal_authorization_decisions
            .get(principal_id)
            .into_iter()
            .flat_map(|indexes| indexes.iter())
            .filter_map(|index| self.authorization_decisions.get(*index))
            .collect()
    }

    pub fn query_authorization_decisions(
        &self,
        selector: &AuthorizationDecisionSelector,
    ) -> Vec<&AuthorizationDecision> {
        self.authorization_decisions
            .iter()
            .filter(|decision| authorization_decision_matches_selector(decision, selector))
            .collect()
    }

    pub fn query_devices(&self, selector: &DeviceSelector) -> Vec<&Device> {
        self.devices
            .values()
            .filter(|device| self.device_matches_selector(device, selector))
            .collect()
    }

    pub fn query_device_summaries(&self, selector: &DeviceSelector) -> Vec<DeviceSummary> {
        self.query_devices(selector)
            .into_iter()
            .map(|device| self.summarize_device(device))
            .collect()
    }

    pub fn query_entities(&self, selector: &EntitySelector) -> Vec<&Entity> {
        self.entities
            .values()
            .filter(|entity| self.entity_matches_selector(entity, selector))
            .collect()
    }

    pub fn query_entity_summaries(&self, selector: &EntitySelector) -> Vec<EntitySummary> {
        self.query_entities(selector)
            .into_iter()
            .map(|entity| self.summarize_entity(entity))
            .collect()
    }

    pub fn stale_states_at(&self, now_ms: u64) -> Vec<&StateSnapshot> {
        self.states
            .values()
            .filter(|snapshot| snapshot.is_stale_at(now_ms))
            .collect()
    }

    pub fn state_refresh_plan_at(&self, now_ms: u64) -> StateRefreshPlan {
        let targets = self
            .entities
            .values()
            .filter_map(|entity| {
                let reason = match self.state(&entity.entity_id) {
                    None => StateRefreshReason::Missing,
                    Some(snapshot) if snapshot.is_stale_at(now_ms) => StateRefreshReason::Stale,
                    Some(_) => return None,
                };
                let device = self.devices.get(&entity.device_id)?;
                Some(StateRefreshTarget {
                    bridge_id: device.bridge_id.clone(),
                    device_id: entity.device_id.clone(),
                    entity_id: entity.entity_id.clone(),
                    kind: entity.kind,
                    capabilities: entity
                        .capabilities
                        .iter()
                        .map(|capability| capability.capability_id.clone())
                        .collect(),
                    reason,
                })
            })
            .collect();
        StateRefreshPlan {
            generated_at_ms: now_ms,
            targets,
        }
    }

    pub fn apply_state_refresh_results<I>(
        &mut self,
        plan: &StateRefreshPlan,
        snapshots: I,
        completed_at_ms: u64,
    ) -> Result<StateRefreshReport, RegistryError>
    where
        I: IntoIterator<Item = StateSnapshot>,
    {
        let planned_entities = plan
            .targets
            .iter()
            .map(|target| target.entity_id.clone())
            .collect::<BTreeSet<_>>();
        let snapshots = snapshots.into_iter().collect::<Vec<_>>();
        let mut seen = BTreeSet::new();

        for snapshot in &snapshots {
            let entity_id = snapshot.entity_id.clone();
            if !planned_entities.contains(&entity_id) {
                return Err(RegistryError::UnexpectedRefreshSnapshot(entity_id));
            }
            if !seen.insert(entity_id.clone()) {
                return Err(RegistryError::DuplicateRefreshSnapshot(entity_id));
            }
        }

        let mut refreshed = Vec::new();
        for snapshot in snapshots {
            let entity_id = snapshot.entity_id.clone();

            self.apply_state_snapshot(snapshot)?;
            refreshed.push(entity_id);
        }

        let missing = plan
            .targets
            .iter()
            .filter(|target| !seen.contains(&target.entity_id))
            .map(|target| target.entity_id.clone())
            .collect();

        Ok(StateRefreshReport {
            generated_at_ms: plan.generated_at_ms,
            completed_at_ms,
            refreshed,
            missing,
        })
    }

    pub fn lookup_protocol(&self, identifier: &ProtocolIdentifier) -> Option<&RegistryTarget> {
        self.protocol_index.get(&ProtocolIndexKey::from(identifier))
    }

    pub fn bridge_by_protocol(&self, identifier: &ProtocolIdentifier) -> Option<&Bridge> {
        match self.lookup_protocol(identifier) {
            Some(RegistryTarget::Bridge(id)) => self.bridges.get(id),
            _ => None,
        }
    }

    pub fn device_by_protocol(&self, identifier: &ProtocolIdentifier) -> Option<&Device> {
        match self.lookup_protocol(identifier) {
            Some(RegistryTarget::Device(id)) => self.devices.get(id),
            _ => None,
        }
    }

    pub fn scene_by_protocol(&self, identifier: &ProtocolIdentifier) -> Option<&Scene> {
        match self.lookup_protocol(identifier) {
            Some(RegistryTarget::Scene(id)) => self.scenes.get(id),
            _ => None,
        }
    }

    fn summarize_bridge(&self, bridge: &Bridge) -> BridgeSummary {
        let device_count = self
            .bridge_devices
            .get(&bridge.bridge_id)
            .map_or(0, BTreeSet::len);
        let entity_count = self
            .devices_for_bridge(&bridge.bridge_id)
            .map(|device| {
                self.device_entities
                    .get(&device.device_id)
                    .map_or(0, BTreeSet::len)
            })
            .sum();

        BridgeSummary {
            bridge_id: bridge.bridge_id.clone(),
            integration_id: bridge.integration_id.clone(),
            transport: bridge.transport,
            health: bridge.health,
            last_seen_at_ms: bridge.last_seen_at_ms,
            device_count,
            entity_count,
            protocol_identifier_count: bridge.identifiers.len(),
            metadata_count: bridge.metadata.len(),
        }
    }

    fn summarize_device(&self, device: &Device) -> DeviceSummary {
        let entities = self
            .entities_for_device(&device.device_id)
            .collect::<Vec<_>>();
        let capability_count = entities
            .iter()
            .map(|entity| entity.capabilities.len())
            .sum();
        let state_count = entities
            .iter()
            .filter(|entity| self.state(&entity.entity_id).is_some())
            .count();

        DeviceSummary {
            device_id: device.device_id.clone(),
            bridge_id: device.bridge_id.clone(),
            manufacturer: device.manufacturer.clone(),
            model: device.model.clone(),
            name: device.name.clone(),
            health: device.health,
            entity_count: entities.len(),
            capability_count,
            state_count,
            protocol_identifier_count: device.identifiers.len(),
            metadata_count: device.metadata.len(),
        }
    }

    fn summarize_entity(&self, entity: &Entity) -> EntitySummary {
        let state = self.state(&entity.entity_id).or(entity.state.as_ref());
        EntitySummary {
            entity_id: entity.entity_id.clone(),
            device_id: entity.device_id.clone(),
            kind: entity.kind,
            name: entity.name.clone(),
            capability_ids: entity
                .capabilities
                .iter()
                .map(|capability| capability.capability_id.clone())
                .collect(),
            has_state: state.is_some(),
            state_value_kind: state.map(|snapshot| value_kind(&snapshot.value)),
            state_source: state.map(|snapshot| snapshot.source),
            state_confidence: state.map(|snapshot| snapshot.confidence),
            state_observed_at_ms: state.map(|snapshot| snapshot.observed_at_ms),
            state_received_at_ms: state.map(|snapshot| snapshot.received_at_ms),
            state_expires_at_ms: state.and_then(|snapshot| snapshot.expires_at_ms),
            metadata_count: entity.metadata.len(),
        }
    }

    fn device_matches_selector(&self, device: &Device, selector: &DeviceSelector) -> bool {
        if selector
            .bridge_id
            .as_ref()
            .is_some_and(|bridge_id| &device.bridge_id != bridge_id)
        {
            return false;
        }
        if selector
            .health
            .is_some_and(|health| device.health != health)
        {
            return false;
        }
        if let Some(capability_id) = &selector.capability_id {
            return self
                .entities_for_device(&device.device_id)
                .any(|entity| entity_has_capability(entity, capability_id));
        }
        true
    }

    fn entity_matches_selector(&self, entity: &Entity, selector: &EntitySelector) -> bool {
        if selector
            .device_id
            .as_ref()
            .is_some_and(|device_id| &entity.device_id != device_id)
        {
            return false;
        }
        if selector.kind.is_some_and(|kind| entity.kind != kind) {
            return false;
        }
        if selector
            .capability_id
            .as_ref()
            .is_some_and(|capability_id| !entity_has_capability(entity, capability_id))
        {
            return false;
        }
        if !state_matches_freshness(self.state(&entity.entity_id), selector.state_freshness) {
            return false;
        }

        let device = self.devices.get(&entity.device_id);
        if selector.bridge_id.is_some() || selector.device_health.is_some() {
            let Some(device) = device else {
                return false;
            };
            if selector
                .bridge_id
                .as_ref()
                .is_some_and(|bridge_id| &device.bridge_id != bridge_id)
            {
                return false;
            }
            if selector
                .device_health
                .is_some_and(|health| device.health != health)
            {
                return false;
            }
        }

        true
    }

    fn replace_protocol_indexes(
        &mut self,
        old_identifiers: Option<&[ProtocolIdentifier]>,
        new_identifiers: &[ProtocolIdentifier],
        target: &RegistryTarget,
    ) -> Result<(), RegistryError> {
        for identifier in new_identifiers {
            let key = ProtocolIndexKey::from(identifier);
            if let Some(existing) = self.protocol_index.get(&key) {
                if existing != target {
                    return Err(RegistryError::ProtocolIdentifierConflict {
                        family: key.family,
                        kind: key.kind,
                        value: key.value,
                        existing: Box::new(existing.clone()),
                        attempted: Box::new(target.clone()),
                    });
                }
            }
        }

        if let Some(old_identifiers) = old_identifiers {
            for identifier in old_identifiers {
                let key = ProtocolIndexKey::from(identifier);
                if self.protocol_index.get(&key) == Some(target) {
                    self.protocol_index.remove(&key);
                }
            }
        }

        for identifier in new_identifiers {
            self.protocol_index
                .insert(ProtocolIndexKey::from(identifier), target.clone());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ProtocolIndexKey {
    family: String,
    kind: String,
    value: String,
}

impl From<&ProtocolIdentifier> for ProtocolIndexKey {
    fn from(identifier: &ProtocolIdentifier) -> Self {
        Self {
            family: protocol_family_key(&identifier.family),
            kind: identifier.kind.clone(),
            value: identifier.value.clone(),
        }
    }
}

fn protocol_family_key(family: &ProtocolFamily) -> String {
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

fn remove_from_index_set<K, V>(map: &mut BTreeMap<K, BTreeSet<V>>, key: &K, value: &V)
where
    K: Ord,
    V: Ord,
{
    if let Some(values) = map.get_mut(key) {
        values.remove(value);
    }
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn entity_has_capability(entity: &Entity, capability_id: &CapabilityId) -> bool {
    entity
        .capabilities
        .iter()
        .any(|capability| &capability.capability_id == capability_id)
}

fn summarize_scene(scene: &Scene) -> SceneSummary {
    SceneSummary {
        scene_id: scene.scene_id.clone(),
        scope: scene.scope,
        action_count: scene.actions.len(),
        has_native_ref: scene.native_ref.is_some(),
        metadata_count: scene.metadata.len(),
    }
}

fn value_kind(value: &Value) -> ValueKind {
    match value {
        Value::Null => ValueKind::Null,
        Value::Bool(_) => ValueKind::Boolean,
        Value::Integer(_) => ValueKind::Integer,
        Value::Number(_) => ValueKind::Number,
        Value::Percentage(_) => ValueKind::Percentage,
        Value::Text(_) => ValueKind::Text,
        Value::Object(_) => ValueKind::Object,
        Value::Array(_) => ValueKind::Array,
    }
}

fn authorization_decision_matches_selector(
    decision: &AuthorizationDecision,
    selector: &AuthorizationDecisionSelector,
) -> bool {
    if selector
        .principal_id
        .as_ref()
        .is_some_and(|principal_id| &decision.principal_id != principal_id)
    {
        return false;
    }
    if selector
        .outcome
        .is_some_and(|outcome| decision.outcome != outcome)
    {
        return false;
    }
    true
}

fn event_matches_selector(event: &DeviceEvent, selector: &EventSelector) -> bool {
    if selector
        .bridge_id
        .as_ref()
        .is_some_and(|bridge_id| &event.bridge_id != bridge_id)
    {
        return false;
    }
    if selector
        .device_id
        .as_ref()
        .is_some_and(|device_id| event.device_id.as_ref() != Some(device_id))
    {
        return false;
    }
    if selector
        .entity_id
        .as_ref()
        .is_some_and(|entity_id| event.entity_id.as_ref() != Some(entity_id))
    {
        return false;
    }
    if selector
        .event_type
        .is_some_and(|event_type| event.event_type != event_type)
    {
        return false;
    }
    if selector
        .observed_at_or_after_ms
        .is_some_and(|observed_at_ms| event.observed_at_ms < observed_at_ms)
    {
        return false;
    }
    if selector
        .received_at_or_after_ms
        .is_some_and(|received_at_ms| event.received_at_ms < received_at_ms)
    {
        return false;
    }
    true
}

fn health_needs_attention(health: Health) -> bool {
    matches!(
        health,
        Health::Degraded
            | Health::Offline
            | Health::AuthFailed
            | Health::Unsupported
            | Health::Removed
    )
}

fn health_is_pairing_candidate(health: Health) -> bool {
    matches!(health, Health::Discoverable | Health::Unpaired)
}

fn state_matches_freshness(snapshot: Option<&StateSnapshot>, freshness: StateFreshness) -> bool {
    match freshness {
        StateFreshness::Any => true,
        StateFreshness::Present => snapshot.is_some(),
        StateFreshness::Missing => snapshot.is_none(),
        StateFreshness::FreshAt(now_ms) => {
            snapshot.is_some_and(|snapshot| !snapshot.is_stale_at(now_ms))
        }
        StateFreshness::StaleAt(now_ms) => {
            snapshot.is_some_and(|snapshot| snapshot.is_stale_at(now_ms))
        }
        StateFreshness::NeedsRefreshAt(now_ms) => {
            snapshot.is_none_or(|snapshot| snapshot.is_stale_at(now_ms))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        AgentId, BridgeTransport, Capability, CapabilityGrant, CapabilityGrantId, CapabilityId,
        EntityKind, IntegrationId, Metadata, PrivilegeTier, ProtocolFamily, SceneAction,
        SceneScope, SmartHomeTool, StateDelta,
    };

    fn bridge(id: &str) -> Bridge {
        let mut bridge = Bridge::new(
            BridgeId::trusted(id),
            IntegrationId::trusted("hue"),
            BridgeTransport::LanHttp,
        );
        bridge.identifiers.push(
            ProtocolIdentifier::new(ProtocolFamily::Hue, "bridge", "bridge-native-1").unwrap(),
        );
        bridge
    }

    fn bridge_with_native(id: &str, native_id: &str) -> Bridge {
        let mut bridge = bridge(id);
        bridge.identifiers =
            vec![ProtocolIdentifier::new(ProtocolFamily::Hue, "bridge", native_id).unwrap()];
        bridge
    }

    fn device(id: &str, bridge_id: &str) -> Device {
        Device {
            device_id: DeviceId::trusted(id),
            bridge_id: BridgeId::trusted(bridge_id),
            manufacturer: "Signify".to_string(),
            model: "Hue bulb".to_string(),
            name: "Kitchen".to_string(),
            serial: None,
            firmware_version: None,
            room_id: None,
            entity_ids: Vec::new(),
            identifiers: vec![ProtocolIdentifier::new(
                ProtocolFamily::Hue,
                "device",
                "device-native-1",
            )
            .unwrap()],
            health: smart_home_core::Health::Online,
            metadata: vec![Metadata::new("fixture", "device")],
        }
    }

    fn device_with_native(id: &str, bridge_id: &str, native_id: &str) -> Device {
        let mut device = device(id, bridge_id);
        device.identifiers =
            vec![ProtocolIdentifier::new(ProtocolFamily::Hue, "device", native_id).unwrap()];
        device
    }

    fn entity(id: &str, device_id: &str) -> Entity {
        Entity {
            entity_id: EntityId::trusted(id),
            device_id: DeviceId::trusted(device_id),
            kind: EntityKind::Light,
            name: "Kitchen Light".to_string(),
            capabilities: vec![Capability::light_on_off()],
            state: None,
            metadata: Vec::new(),
        }
    }

    fn sensor_entity(id: &str, device_id: &str) -> Entity {
        let mut entity = entity(id, device_id);
        entity.kind = EntityKind::Sensor;
        entity.name = "Kitchen Motion".to_string();
        entity.capabilities = vec![Capability::sensor_occupancy()];
        entity
    }

    fn update_event(id: &str, entity_id: &str, observed_at_ms: u64) -> DeviceEvent {
        DeviceEvent {
            event_id: EventId::trusted(id),
            bridge_id: BridgeId::trusted("bridge-1"),
            device_id: Some(DeviceId::trusted("device-1")),
            entity_id: Some(EntityId::trusted(entity_id)),
            observed_at_ms,
            received_at_ms: observed_at_ms + 1,
            event_type: DeviceEventType::Updated,
            state_delta: Some(StateDelta {
                capability_id: CapabilityId::trusted("light.on_off"),
                value: Value::Bool(true),
            }),
            raw_ref: None,
            correlation_id: None,
            metadata: Vec::new(),
        }
    }

    #[test]
    fn registers_bridge_device_and_entity_indexes() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();

        let devices: Vec<_> = registry
            .devices_for_bridge(&BridgeId::trusted("bridge-1"))
            .collect();
        let entities: Vec<_> = registry
            .entities_for_device(&DeviceId::trusted("device-1"))
            .collect();

        assert_eq!(devices.len(), 1);
        assert_eq!(entities.len(), 1);
        assert_eq!(
            registry
                .device(&DeviceId::trusted("device-1"))
                .unwrap()
                .entity_ids,
            vec![EntityId::trusted("entity-1")]
        );
        assert_eq!(registry.counts().entities, 1);
    }

    #[test]
    fn read_view_exposes_query_surface_without_write_access() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();

        let read = registry.read_view();
        let devices: Vec<_> = read
            .devices_for_bridge(&BridgeId::trusted("bridge-1"))
            .map(|device| device.device_id.clone())
            .collect();
        let lights = read.query_entities(&EntitySelector::new().with_kind(EntityKind::Light));

        assert_eq!(read.access_mode(), RegistryAccessMode::ReadOnly);
        assert_eq!(read.counts().entities, 1);
        assert_eq!(devices, vec![DeviceId::trusted("device-1")]);
        assert_eq!(lights[0].entity_id, EntityId::trusted("entity-1"));
    }

    #[test]
    fn read_summaries_expose_compact_registry_shape() {
        let mut registry = InMemorySmartHomeRegistry::new();
        let mut bridge = bridge("bridge-1");
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(1_000);
        bridge.metadata.push(Metadata::new("room", "kitchen"));
        registry.upsert_bridge(bridge).unwrap();

        let mut device = device("device-1", "bridge-1");
        device.metadata.push(Metadata::new("fixture", "ceiling"));
        registry.upsert_device(device).unwrap();

        let mut entity = entity("entity-1", "device-1");
        entity.capabilities.push(Capability::light_brightness());
        entity.metadata.push(Metadata::new("surface", "counter"));
        registry.upsert_entity(entity).unwrap();
        registry
            .apply_state_snapshot(StateSnapshot {
                entity_id: EntityId::trusted("entity-1"),
                value: Value::Percentage(42),
                source: StateSource::Poll,
                observed_at_ms: 1_010,
                received_at_ms: 1_011,
                expires_at_ms: Some(2_000),
                confidence: StateConfidence::Confirmed,
            })
            .unwrap();
        registry
            .upsert_scene(Scene {
                scene_id: SceneId::trusted("scene-1"),
                scope: SceneScope::Room,
                native_ref: Some(
                    ProtocolIdentifier::new(ProtocolFamily::Hue, "scene", "scene-native-1")
                        .unwrap(),
                ),
                actions: vec![SceneAction {
                    entity_id: EntityId::trusted("entity-1"),
                    desired_state: Value::Bool(true),
                }],
                metadata: Vec::new(),
            })
            .unwrap();

        let read = registry.read_view();
        let bridge_summary = read.bridge_summary(&BridgeId::trusted("bridge-1")).unwrap();
        let device_summary = read
            .query_device_summaries(
                &DeviceSelector::new().with_capability(CapabilityId::trusted("light.brightness")),
            )
            .pop()
            .unwrap();
        let entity_summary = read
            .query_entity_summaries(
                &EntitySelector::new().with_state_freshness(StateFreshness::FreshAt(1_500)),
            )
            .pop()
            .unwrap();
        let scene_summary = read.scene_summary(&SceneId::trusted("scene-1")).unwrap();

        assert_eq!(bridge_summary.health, Health::Online);
        assert_eq!(bridge_summary.device_count, 1);
        assert_eq!(bridge_summary.entity_count, 1);
        assert_eq!(bridge_summary.metadata_count, 1);
        assert_eq!(device_summary.entity_count, 1);
        assert_eq!(device_summary.capability_count, 2);
        assert_eq!(device_summary.state_count, 1);
        assert_eq!(
            entity_summary.capability_ids,
            vec![
                CapabilityId::trusted("light.on_off"),
                CapabilityId::trusted("light.brightness")
            ]
        );
        assert!(entity_summary.has_state);
        assert_eq!(entity_summary.state_value_kind, Some(ValueKind::Percentage));
        assert_eq!(entity_summary.state_source, Some(StateSource::Poll));
        assert_eq!(
            entity_summary.state_confidence,
            Some(StateConfidence::Confirmed)
        );
        assert_eq!(entity_summary.state_expires_at_ms, Some(2_000));
        assert_eq!(scene_summary.action_count, 1);
        assert!(scene_summary.has_native_ref);
    }

    #[test]
    fn supervision_summary_counts_health_and_refresh_work() {
        let mut registry = InMemorySmartHomeRegistry::new();
        let mut bridge_1 = bridge("bridge-1");
        bridge_1.health = Health::Online;
        let mut bridge_2 = bridge_with_native("bridge-2", "bridge-native-2");
        bridge_2.health = Health::AuthFailed;
        registry.upsert_bridge(bridge_1).unwrap();
        registry.upsert_bridge(bridge_2).unwrap();

        let mut online_device = device_with_native("device-1", "bridge-1", "device-native-1");
        online_device.health = Health::Online;
        let mut offline_device = device_with_native("device-2", "bridge-1", "device-native-2");
        offline_device.health = Health::Offline;
        let mut unpaired_device = device_with_native("device-3", "bridge-2", "device-native-3");
        unpaired_device.health = Health::Unpaired;
        registry.upsert_device(online_device).unwrap();
        registry.upsert_device(offline_device).unwrap();
        registry.upsert_device(unpaired_device).unwrap();

        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-2", "device-2"))
            .unwrap();
        registry
            .upsert_entity(sensor_entity("entity-3", "device-3"))
            .unwrap();
        registry
            .apply_state_snapshot(StateSnapshot {
                entity_id: EntityId::trusted("entity-1"),
                value: Value::Bool(true),
                source: StateSource::Poll,
                observed_at_ms: 100,
                received_at_ms: 101,
                expires_at_ms: Some(1_000),
                confidence: StateConfidence::Confirmed,
            })
            .unwrap();
        registry
            .apply_state_snapshot(StateSnapshot {
                entity_id: EntityId::trusted("entity-3"),
                value: Value::Bool(false),
                source: StateSource::Poll,
                observed_at_ms: 200,
                received_at_ms: 201,
                expires_at_ms: Some(400),
                confidence: StateConfidence::Confirmed,
            })
            .unwrap();

        let summary = registry.read_view().supervision_summary_at(500);

        assert_eq!(summary.generated_at_ms, 500);
        assert_eq!(summary.bridges, 2);
        assert_eq!(summary.attention_bridges, 1);
        assert_eq!(summary.pairing_candidate_bridges, 0);
        assert_eq!(summary.devices, 3);
        assert_eq!(summary.online_devices, 1);
        assert_eq!(summary.attention_devices, 1);
        assert_eq!(summary.pairing_candidate_devices, 1);
        assert_eq!(summary.entities, 3);
        assert_eq!(summary.state_snapshots, 2);
        assert_eq!(summary.missing_entity_states, 1);
        assert_eq!(summary.stale_entity_states, 1);
        assert_eq!(summary.refresh_targets, 2);
        assert_eq!(summary.events, 0);
        assert!(summary.has_attention_items());
        assert!(summary.has_refresh_work());
    }

    #[test]
    fn write_view_owns_mutations_and_can_be_read_back() {
        let mut registry = InMemorySmartHomeRegistry::new();
        {
            let mut write = registry.write_view();
            assert_eq!(write.access_mode(), RegistryAccessMode::ReadWrite);
            write.upsert_bridge(bridge("bridge-1")).unwrap();
            write.upsert_device(device("device-1", "bridge-1")).unwrap();
            assert_eq!(write.as_read().counts().devices, 1);
        }

        let read = registry.read_view();

        assert_eq!(read.counts().bridges, 1);
        assert_eq!(
            read.device(&DeviceId::trusted("device-1"))
                .unwrap()
                .bridge_id,
            BridgeId::trusted("bridge-1")
        );
    }

    #[test]
    fn protocol_identifier_lookup_points_to_normalized_records() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();

        let hue_device =
            ProtocolIdentifier::new(ProtocolFamily::Hue, "device", "device-native-1").unwrap();

        assert_eq!(
            registry.lookup_protocol(&hue_device),
            Some(&RegistryTarget::Device(DeviceId::trusted("device-1")))
        );
        assert_eq!(
            registry.device_by_protocol(&hue_device).unwrap().name,
            "Kitchen"
        );
    }

    #[test]
    fn duplicate_protocol_identifiers_are_rejected_across_targets() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        let mut duplicate = device("device-2", "bridge-1");
        duplicate.identifiers =
            vec![
                ProtocolIdentifier::new(ProtocolFamily::Hue, "device", "device-native-1").unwrap(),
            ];

        assert!(matches!(
            registry.upsert_device(duplicate),
            Err(RegistryError::ProtocolIdentifierConflict { .. })
        ));
    }

    #[test]
    fn state_snapshots_update_entity_state_and_cache() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();

        let snapshot = StateSnapshot {
            entity_id: EntityId::trusted("entity-1"),
            value: Value::Bool(true),
            source: StateSource::Poll,
            observed_at_ms: 100,
            received_at_ms: 101,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        };
        registry.apply_state_snapshot(snapshot.clone()).unwrap();

        assert_eq!(
            registry.state(&EntityId::trusted("entity-1")),
            Some(&snapshot)
        );
        assert_eq!(
            registry
                .entity(&EntityId::trusted("entity-1"))
                .unwrap()
                .state,
            Some(snapshot)
        );
    }

    #[test]
    fn events_are_recorded_in_arrival_order_and_update_state_from_delta() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();

        registry
            .record_event(DeviceEvent {
                event_id: EventId::trusted("event-1"),
                bridge_id: BridgeId::trusted("bridge-1"),
                device_id: Some(DeviceId::trusted("device-1")),
                entity_id: Some(EntityId::trusted("entity-1")),
                observed_at_ms: 200,
                received_at_ms: 201,
                event_type: DeviceEventType::Updated,
                state_delta: Some(StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(true),
                }),
                raw_ref: None,
                correlation_id: None,
                metadata: Vec::new(),
            })
            .unwrap();

        let event_ids: Vec<_> = registry
            .events()
            .map(|event| event.event_id.clone())
            .collect();
        assert_eq!(event_ids, vec![EventId::trusted("event-1")]);
        assert_eq!(
            registry
                .state(&EntityId::trusted("entity-1"))
                .unwrap()
                .value,
            Value::Object(vec![("light.on_off".to_string(), Value::Bool(true))])
        );
    }

    #[test]
    fn query_events_filters_arrival_log_for_replay_windows() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-2", "device-1"))
            .unwrap();

        registry
            .record_event(update_event("event-1", "entity-1", 100))
            .unwrap();
        registry
            .record_event(update_event("event-2", "entity-2", 110))
            .unwrap();
        registry
            .record_event(DeviceEvent {
                event_id: EventId::trusted("event-3"),
                event_type: DeviceEventType::Health,
                state_delta: None,
                ..update_event("event-3", "entity-1", 120)
            })
            .unwrap();
        registry
            .record_event(update_event("event-4", "entity-1", 130))
            .unwrap();

        let replay = registry.query_events(
            &EventSelector::new()
                .for_bridge(BridgeId::trusted("bridge-1"))
                .for_entity(EntityId::trusted("entity-1"))
                .with_event_type(DeviceEventType::Updated)
                .received_at_or_after(101)
                .with_limit(2),
        );
        assert_eq!(
            replay
                .iter()
                .map(|event| event.event_id.clone())
                .collect::<Vec<_>>(),
            vec![EventId::trusted("event-1"), EventId::trusted("event-4")]
        );

        let after_observed = registry.query_events(
            &EventSelector::new()
                .for_device(DeviceId::trusted("device-1"))
                .observed_at_or_after(115),
        );
        assert_eq!(
            after_observed
                .iter()
                .map(|event| event.event_id.clone())
                .collect::<Vec<_>>(),
            vec![EventId::trusted("event-3"), EventId::trusted("event-4")]
        );
        assert!(registry
            .query_events(&EventSelector::new().with_limit(0))
            .is_empty());
    }

    #[test]
    fn capability_grants_are_indexed_by_principal_and_status() {
        let mut registry = InMemorySmartHomeRegistry::new();
        let principal = AgentId::trusted("agent:lighting-planner");
        let other_principal = AgentId::trusted("agent:energy-saver");
        let read_grant = CapabilityGrant::for_capability(
            CapabilityGrantId::trusted("grant-read"),
            principal.clone(),
            CapabilityId::trusted("smart_home.read"),
            PrivilegeTier::ReadOnly,
            "chief-of-staff",
            1_000,
        );
        let command_grant = CapabilityGrant::for_capability(
            CapabilityGrantId::trusted("grant-command"),
            principal.clone(),
            CapabilityId::trusted("smart_home.command.light"),
            PrivilegeTier::LowRisk,
            "chief-of-staff",
            1_000,
        )
        .with_expiry(2_000);
        let other_grant = CapabilityGrant::for_capability(
            CapabilityGrantId::trusted("grant-other"),
            other_principal.clone(),
            CapabilityId::trusted("smart_home.read"),
            PrivilegeTier::ReadOnly,
            "chief-of-staff",
            1_000,
        );

        assert!(registry.upsert_capability_grant(read_grant).is_none());
        assert!(registry.upsert_capability_grant(command_grant).is_none());
        registry.upsert_capability_grant(other_grant);

        let principal_grant_count = registry.capability_grants_for_principal(&principal).len();
        let active_at_1_500_count = registry
            .active_capability_grants_for_principal_at(&principal, 1_500)
            .len();
        let active_at_2_000_count = registry
            .active_capability_grants_for_principal_at(&principal, 2_000)
            .len();
        let revoked = registry
            .update_capability_grant_status(
                &CapabilityGrantId::trusted("grant-read"),
                CapabilityGrantStatus::Revoked,
            )
            .unwrap();
        let active_after_revoke =
            registry.active_capability_grants_for_principal_at(&principal, 1_500);

        assert_eq!(registry.counts().capability_grants, 3);
        assert_eq!(principal_grant_count, 2);
        assert_eq!(active_at_1_500_count, 2);
        assert_eq!(active_at_2_000_count, 1);
        assert_eq!(revoked.status, CapabilityGrantStatus::Revoked);
        assert_eq!(active_after_revoke.len(), 1);
        assert_eq!(
            registry.capability_grant(&CapabilityGrantId::trusted("grant-command")),
            active_after_revoke.first().copied()
        );
        assert_eq!(
            registry.update_capability_grant_status(
                &CapabilityGrantId::trusted("missing"),
                CapabilityGrantStatus::Revoked,
            ),
            Err(RegistryError::UnknownCapabilityGrant(
                CapabilityGrantId::trusted("missing")
            ))
        );
        assert_eq!(
            registry
                .capability_grants_for_principal(&other_principal)
                .len(),
            1
        );
    }

    #[test]
    fn authorization_decisions_are_recorded_in_order_and_indexed_by_principal() {
        let mut registry = InMemorySmartHomeRegistry::new();
        let principal = AgentId::trusted("agent:lighting-planner");
        let other_principal = AgentId::trusted("agent:energy-saver");
        let read_grant = CapabilityGrant::for_capability(
            CapabilityGrantId::trusted("grant-read"),
            principal.clone(),
            CapabilityId::trusted("smart_home.read"),
            PrivilegeTier::ReadOnly,
            "chief-of-staff",
            1_000,
        );
        let allowed = AuthorizationDecision::for_tool(
            principal.clone(),
            SmartHomeTool::GetState,
            [&read_grant],
            1_500,
        );
        let denied = AuthorizationDecision::for_tool(
            other_principal.clone(),
            SmartHomeTool::Command,
            std::iter::empty::<&CapabilityGrant>(),
            1_501,
        );

        assert_eq!(registry.record_authorization_decision(allowed), 0);
        assert_eq!(registry.record_authorization_decision(denied), 1);

        let all_principals = registry
            .authorization_decisions()
            .map(|decision| decision.principal_id.clone())
            .collect::<Vec<_>>();
        let principal_decisions = registry.authorization_decisions_for_principal(&principal);
        let denied_for_other = registry.query_authorization_decisions(
            &AuthorizationDecisionSelector::new()
                .for_principal(other_principal.clone())
                .with_outcome(AuthorizationOutcome::Denied),
        );

        assert_eq!(registry.counts().authorization_decisions, 2);
        assert_eq!(all_principals, vec![principal.clone(), other_principal]);
        assert_eq!(principal_decisions.len(), 1);
        assert_eq!(
            principal_decisions[0].outcome,
            AuthorizationOutcome::Allowed
        );
        assert_eq!(denied_for_other.len(), 1);
        assert_eq!(
            denied_for_other[0].missing_capabilities,
            vec![CapabilityId::trusted("smart_home.command.light")]
        );
    }

    #[test]
    fn scenes_validate_entity_actions_and_index_native_refs() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();

        let scene_ref =
            ProtocolIdentifier::new(ProtocolFamily::Hue, "scene", "scene-native-1").unwrap();
        registry
            .upsert_scene(Scene {
                scene_id: SceneId::trusted("scene-1"),
                scope: SceneScope::Room,
                native_ref: Some(scene_ref.clone()),
                actions: vec![SceneAction {
                    entity_id: EntityId::trusted("entity-1"),
                    desired_state: Value::Bool(true),
                }],
                metadata: Vec::new(),
            })
            .unwrap();

        assert_eq!(
            registry.scene_by_protocol(&scene_ref).unwrap().scene_id,
            SceneId::trusted("scene-1")
        );
    }

    #[test]
    fn query_devices_filters_by_bridge_health_and_entity_capability() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry
            .upsert_bridge(bridge_with_native("bridge-1", "bridge-native-1"))
            .unwrap();
        registry
            .upsert_bridge(bridge_with_native("bridge-2", "bridge-native-2"))
            .unwrap();

        let mut online_light = device_with_native("device-1", "bridge-1", "device-native-1");
        online_light.health = smart_home_core::Health::Online;
        let mut offline_sensor = device_with_native("device-2", "bridge-1", "device-native-2");
        offline_sensor.health = smart_home_core::Health::Offline;
        let mut other_bridge = device_with_native("device-3", "bridge-2", "device-native-3");
        other_bridge.health = smart_home_core::Health::Online;

        registry.upsert_device(online_light).unwrap();
        registry.upsert_device(offline_sensor).unwrap();
        registry.upsert_device(other_bridge).unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();
        registry
            .upsert_entity(sensor_entity("entity-2", "device-2"))
            .unwrap();

        let selector = DeviceSelector::new()
            .for_bridge(BridgeId::trusted("bridge-1"))
            .with_health(smart_home_core::Health::Online)
            .with_capability(CapabilityId::trusted("light.on_off"));
        let device_ids: Vec<_> = registry
            .query_devices(&selector)
            .into_iter()
            .map(|device| device.device_id.clone())
            .collect();

        assert_eq!(device_ids, vec![DeviceId::trusted("device-1")]);
    }

    #[test]
    fn query_entities_filters_by_kind_capability_health_and_freshness() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        let mut light_device = device_with_native("device-1", "bridge-1", "device-native-1");
        light_device.health = smart_home_core::Health::Online;
        let mut sensor_device = device_with_native("device-2", "bridge-1", "device-native-2");
        sensor_device.health = smart_home_core::Health::Offline;
        registry.upsert_device(light_device).unwrap();
        registry.upsert_device(sensor_device).unwrap();

        let mut light = entity("entity-1", "device-1");
        light.state = Some(StateSnapshot {
            entity_id: EntityId::trusted("entity-1"),
            value: Value::Bool(true),
            source: StateSource::Poll,
            observed_at_ms: 100,
            received_at_ms: 101,
            expires_at_ms: Some(500),
            confidence: StateConfidence::Confirmed,
        });
        registry.upsert_entity(light).unwrap();
        registry
            .upsert_entity(sensor_entity("entity-2", "device-2"))
            .unwrap();

        let fresh_lights = registry.query_entities(
            &EntitySelector::new()
                .with_kind(EntityKind::Light)
                .with_capability(CapabilityId::trusted("light.on_off"))
                .with_device_health(smart_home_core::Health::Online)
                .with_state_freshness(StateFreshness::FreshAt(400)),
        );
        assert_eq!(fresh_lights.len(), 1);
        assert_eq!(fresh_lights[0].entity_id, EntityId::trusted("entity-1"));

        let needs_refresh: Vec<_> = registry
            .query_entities(
                &EntitySelector::new()
                    .for_bridge(BridgeId::trusted("bridge-1"))
                    .with_state_freshness(StateFreshness::NeedsRefreshAt(600)),
            )
            .into_iter()
            .map(|entity| entity.entity_id.clone())
            .collect();
        assert_eq!(
            needs_refresh,
            vec![EntityId::trusted("entity-1"), EntityId::trusted("entity-2")]
        );
        assert_eq!(registry.stale_states_at(600).len(), 1);
    }

    #[test]
    fn state_refresh_plan_lists_missing_and_stale_entity_state() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry
            .upsert_bridge(bridge_with_native("bridge-1", "bridge-native-1"))
            .unwrap();
        registry
            .upsert_bridge(bridge_with_native("bridge-2", "bridge-native-2"))
            .unwrap();
        registry
            .upsert_device(device_with_native(
                "device-1",
                "bridge-1",
                "device-native-1",
            ))
            .unwrap();
        registry
            .upsert_device(device_with_native(
                "device-2",
                "bridge-1",
                "device-native-2",
            ))
            .unwrap();
        registry
            .upsert_device(device_with_native(
                "device-3",
                "bridge-2",
                "device-native-3",
            ))
            .unwrap();

        let mut fresh = entity("entity-1", "device-1");
        fresh.state = Some(StateSnapshot {
            entity_id: EntityId::trusted("entity-1"),
            value: Value::Bool(true),
            source: StateSource::Poll,
            observed_at_ms: 100,
            received_at_ms: 101,
            expires_at_ms: Some(1_000),
            confidence: StateConfidence::Confirmed,
        });
        let mut stale = entity("entity-2", "device-2");
        stale.state = Some(StateSnapshot {
            entity_id: EntityId::trusted("entity-2"),
            value: Value::Bool(false),
            source: StateSource::Poll,
            observed_at_ms: 100,
            received_at_ms: 101,
            expires_at_ms: Some(200),
            confidence: StateConfidence::Confirmed,
        });
        registry.upsert_entity(fresh).unwrap();
        registry.upsert_entity(stale).unwrap();
        registry
            .upsert_entity(sensor_entity("entity-3", "device-3"))
            .unwrap();

        let plan = registry.state_refresh_plan_at(500);

        assert_eq!(plan.generated_at_ms, 500);
        assert_eq!(plan.len(), 2);
        assert_eq!(
            plan.targets_for_bridge(&BridgeId::trusted("bridge-1"))
                .len(),
            1
        );
        assert_eq!(
            plan.targets,
            vec![
                StateRefreshTarget {
                    bridge_id: BridgeId::trusted("bridge-1"),
                    device_id: DeviceId::trusted("device-2"),
                    entity_id: EntityId::trusted("entity-2"),
                    kind: EntityKind::Light,
                    capabilities: vec![CapabilityId::trusted("light.on_off")],
                    reason: StateRefreshReason::Stale,
                },
                StateRefreshTarget {
                    bridge_id: BridgeId::trusted("bridge-2"),
                    device_id: DeviceId::trusted("device-3"),
                    entity_id: EntityId::trusted("entity-3"),
                    kind: EntityKind::Sensor,
                    capabilities: vec![CapabilityId::trusted("sensor.occupancy")],
                    reason: StateRefreshReason::Missing,
                },
            ]
        );
    }

    #[test]
    fn applies_state_refresh_results_and_reports_missing_targets() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();
        registry
            .upsert_entity(sensor_entity("entity-2", "device-1"))
            .unwrap();

        let plan = registry.state_refresh_plan_at(500);
        let report = registry
            .apply_state_refresh_results(
                &plan,
                vec![StateSnapshot {
                    entity_id: EntityId::trusted("entity-1"),
                    value: Value::Bool(true),
                    source: StateSource::Poll,
                    observed_at_ms: 501,
                    received_at_ms: 502,
                    expires_at_ms: Some(1_000),
                    confidence: StateConfidence::Confirmed,
                }],
                503,
            )
            .unwrap();

        assert_eq!(report.generated_at_ms, 500);
        assert_eq!(report.completed_at_ms, 503);
        assert_eq!(report.refreshed, vec![EntityId::trusted("entity-1")]);
        assert_eq!(report.missing, vec![EntityId::trusted("entity-2")]);
        assert_eq!(report.refreshed_count(), 1);
        assert_eq!(report.missing_count(), 1);
        assert!(!report.is_complete());
        assert_eq!(
            registry
                .state(&EntityId::trusted("entity-1"))
                .unwrap()
                .value,
            Value::Bool(true)
        );
        assert!(registry.state(&EntityId::trusted("entity-2")).is_none());
    }

    #[test]
    fn refresh_results_reject_duplicate_snapshots_without_partial_updates() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();

        let plan = registry.state_refresh_plan_at(500);
        let snapshot = StateSnapshot {
            entity_id: EntityId::trusted("entity-1"),
            value: Value::Bool(true),
            source: StateSource::Poll,
            observed_at_ms: 501,
            received_at_ms: 502,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        };

        assert_eq!(
            registry.apply_state_refresh_results(&plan, vec![snapshot.clone(), snapshot], 503),
            Err(RegistryError::DuplicateRefreshSnapshot(EntityId::trusted(
                "entity-1"
            )))
        );
        assert!(registry.state(&EntityId::trusted("entity-1")).is_none());
    }

    #[test]
    fn refresh_results_reject_snapshots_outside_the_plan() {
        let mut registry = InMemorySmartHomeRegistry::new();
        registry.upsert_bridge(bridge("bridge-1")).unwrap();
        registry
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        registry
            .upsert_entity(entity("entity-1", "device-1"))
            .unwrap();
        registry
            .upsert_entity(sensor_entity("entity-2", "device-1"))
            .unwrap();

        let plan = StateRefreshPlan {
            generated_at_ms: 500,
            targets: vec![StateRefreshTarget {
                bridge_id: BridgeId::trusted("bridge-1"),
                device_id: DeviceId::trusted("device-1"),
                entity_id: EntityId::trusted("entity-1"),
                kind: EntityKind::Light,
                capabilities: vec![CapabilityId::trusted("light.on_off")],
                reason: StateRefreshReason::Missing,
            }],
        };
        let snapshot = StateSnapshot {
            entity_id: EntityId::trusted("entity-2"),
            value: Value::Bool(false),
            source: StateSource::Poll,
            observed_at_ms: 501,
            received_at_ms: 502,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        };

        assert_eq!(
            registry.apply_state_refresh_results(&plan, vec![snapshot], 503),
            Err(RegistryError::UnexpectedRefreshSnapshot(EntityId::trusted(
                "entity-2"
            )))
        );
        assert!(registry.state(&EntityId::trusted("entity-2")).is_none());
    }

    #[test]
    fn events_reject_unknown_references() {
        let mut registry = InMemorySmartHomeRegistry::new();

        assert!(matches!(
            registry.record_event(DeviceEvent {
                event_id: EventId::trusted("event-1"),
                bridge_id: BridgeId::trusted("missing"),
                device_id: None,
                entity_id: None,
                observed_at_ms: 0,
                received_at_ms: 0,
                event_type: DeviceEventType::Updated,
                state_delta: None,
                raw_ref: None,
                correlation_id: None,
                metadata: Vec::new(),
            }),
            Err(RegistryError::EventBridgeMismatch { .. })
        ));
    }
}
