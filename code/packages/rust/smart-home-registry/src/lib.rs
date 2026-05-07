//! In-memory smart-home registry for normalized D23 records.
//!
//! This crate is the first registry slice: it stores bridge, device, entity,
//! scene, state, event, and protocol-id indexes without any filesystem, Vault,
//! actor, network, serial, or radio access. Durable D18A-backed storage can sit
//! behind the same operations later.

#![forbid(unsafe_code)]

use smart_home_core::{
    Bridge, BridgeId, CapabilityId, Device, DeviceEvent, DeviceEventType, DeviceId, Entity,
    EntityId, EntityKind, EventId, Health, ProtocolFamily, ProtocolIdentifier, Scene, SceneId,
    StateConfidence, StateSnapshot, StateSource, Value,
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
        existing: RegistryTarget,
        attempted: RegistryTarget,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateFreshness {
    Any,
    Present,
    Missing,
    FreshAt(u64),
    StaleAt(u64),
    NeedsRefreshAt(u64),
}

impl Default for StateFreshness {
    fn default() -> Self {
        Self::Any
    }
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

#[derive(Debug, Clone, Default)]
pub struct InMemorySmartHomeRegistry {
    bridges: BTreeMap<BridgeId, Bridge>,
    devices: BTreeMap<DeviceId, Device>,
    entities: BTreeMap<EntityId, Entity>,
    scenes: BTreeMap<SceneId, Scene>,
    states: BTreeMap<EntityId, StateSnapshot>,
    events: BTreeMap<EventId, DeviceEvent>,
    event_order: Vec<EventId>,
    bridge_devices: BTreeMap<BridgeId, BTreeSet<DeviceId>>,
    device_entities: BTreeMap<DeviceId, BTreeSet<EntityId>>,
    protocol_index: BTreeMap<ProtocolIndexKey, RegistryTarget>,
}

impl InMemorySmartHomeRegistry {
    pub fn new() -> Self {
        Self::default()
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

    pub fn query_devices(&self, selector: &DeviceSelector) -> Vec<&Device> {
        self.devices
            .values()
            .filter(|device| self.device_matches_selector(device, selector))
            .collect()
    }

    pub fn query_entities(&self, selector: &EntitySelector) -> Vec<&Entity> {
        self.entities
            .values()
            .filter(|entity| self.entity_matches_selector(entity, selector))
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
                        existing: existing.clone(),
                        attempted: target.clone(),
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
            snapshot.map_or(true, |snapshot| snapshot.is_stale_at(now_ms))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        BridgeTransport, Capability, CapabilityId, EntityKind, IntegrationId, Metadata,
        ProtocolFamily, SceneAction, SceneScope, StateDelta,
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
