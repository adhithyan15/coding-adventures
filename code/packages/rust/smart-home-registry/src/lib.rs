//! In-memory smart-home registry for normalized D23 records.
//!
//! This crate is the first registry slice: it stores bridge, device, entity,
//! scene, state, event, and protocol-id indexes without any filesystem, Vault,
//! actor, network, serial, or radio access. Durable D18A-backed storage can sit
//! behind the same operations later.

#![forbid(unsafe_code)]

use smart_home_core::{
    Bridge, BridgeId, Device, DeviceEvent, DeviceEventType, DeviceId, Entity, EntityId, EventId,
    ProtocolFamily, ProtocolIdentifier, Scene, SceneId, StateConfidence, StateSnapshot,
    StateSource, Value,
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
