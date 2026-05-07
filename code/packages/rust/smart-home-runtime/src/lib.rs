//! Deterministic smart-home runtime coordinator.
//!
//! This crate is the first runtime slice above the normalized D23 model. It is
//! intentionally synchronous: actors, transports, and protocol workers can wrap
//! it later, while command validation, event routing, state confidence, and
//! supervision rules remain easy to test.

#![forbid(unsafe_code)]

use smart_home_core::{
    Bridge, BridgeId, CapabilityId, CapabilityMode, CommandResult, CommandStatus, CommandType,
    Device, DeviceCommand, DeviceEvent, DeviceEventType, DeviceId, Entity, EntityId, EventId,
    Health, IntegrationId, Metadata, StateConfidence, StateSnapshot, StateSource, Value,
};
use smart_home_registry::{InMemorySmartHomeRegistry, RegistryError};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Registry(RegistryError),
    UnknownBridge(BridgeId),
    UnknownDevice(DeviceId),
    UnknownEntity(EntityId),
    UnknownSubscription(RuntimeSubscriptionId),
    DuplicateSubscription(RuntimeSubscriptionId),
    UnsupportedCapability {
        entity_id: EntityId,
        capability_id: CapabilityId,
    },
    ReadOnlyCapability {
        entity_id: EntityId,
        capability_id: CapabilityId,
    },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => write!(f, "{error}"),
            Self::UnknownBridge(id) => write!(f, "unknown runtime bridge {id}"),
            Self::UnknownDevice(id) => write!(f, "unknown runtime device {id}"),
            Self::UnknownEntity(id) => write!(f, "unknown runtime entity {id}"),
            Self::UnknownSubscription(id) => write!(f, "unknown runtime subscription {id}"),
            Self::DuplicateSubscription(id) => write!(f, "duplicate runtime subscription {id}"),
            Self::UnsupportedCapability {
                entity_id,
                capability_id,
            } => write!(
                f,
                "entity {entity_id} does not expose required capability {capability_id}"
            ),
            Self::ReadOnlyCapability {
                entity_id,
                capability_id,
            } => write!(
                f,
                "entity {entity_id} exposes capability {capability_id} as observe-only"
            ),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<RegistryError> for RuntimeError {
    fn from(error: RegistryError) -> Self {
        Self::Registry(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeSubscriptionId(String);

impl RuntimeSubscriptionId {
    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuntimeSubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEventFilter {
    All,
    Bridge(BridgeId),
    Entity(EntityId),
    Commands,
    Supervision,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeEvent {
    Device(DeviceEvent),
    CommandResult(CommandResult),
    BridgeHealth {
        event_id: EventId,
        bridge_id: BridgeId,
        health: Health,
        observed_at_ms: u64,
        received_at_ms: u64,
    },
    StateExpired {
        entity_id: EntityId,
        expired_at_ms: u64,
    },
    WorkerNeedsRestart {
        bridge_id: BridgeId,
        integration_id: IntegrationId,
        overdue_at_ms: u64,
    },
}

impl RuntimeEventFilter {
    pub fn matches(&self, event: &RuntimeEvent) -> bool {
        match self {
            Self::All => true,
            Self::Bridge(expected) => event_bridge_id(event) == Some(expected),
            Self::Entity(expected) => event_entity_id(event) == Some(expected),
            Self::Commands => matches!(event, RuntimeEvent::CommandResult(_)),
            Self::Supervision => matches!(event, RuntimeEvent::WorkerNeedsRestart { .. }),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeEventBus {
    subscriptions: BTreeMap<RuntimeSubscriptionId, RuntimeEventFilter>,
    deliveries: BTreeMap<RuntimeSubscriptionId, VecDeque<RuntimeEvent>>,
    published: Vec<RuntimeEvent>,
}

impl RuntimeEventBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(
        &mut self,
        subscription_id: RuntimeSubscriptionId,
        filter: RuntimeEventFilter,
    ) -> Result<(), RuntimeError> {
        if self.subscriptions.contains_key(&subscription_id) {
            return Err(RuntimeError::DuplicateSubscription(subscription_id));
        }
        self.subscriptions.insert(subscription_id.clone(), filter);
        self.deliveries.insert(subscription_id, VecDeque::new());
        Ok(())
    }

    pub fn publish(&mut self, event: RuntimeEvent) {
        for (subscription_id, filter) in &self.subscriptions {
            if filter.matches(&event) {
                self.deliveries
                    .entry(subscription_id.clone())
                    .or_default()
                    .push_back(event.clone());
            }
        }
        self.published.push(event);
    }

    pub fn drain(
        &mut self,
        subscription_id: &RuntimeSubscriptionId,
    ) -> Result<Vec<RuntimeEvent>, RuntimeError> {
        let queue = self
            .deliveries
            .get_mut(subscription_id)
            .ok_or_else(|| RuntimeError::UnknownSubscription(subscription_id.clone()))?;
        Ok(queue.drain(..).collect())
    }

    pub fn published(&self) -> &[RuntimeEvent] {
        &self.published
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Starting,
    Running,
    Unhealthy,
    Restarting,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisedBridgeWorker {
    pub bridge_id: BridgeId,
    pub integration_id: IntegrationId,
    pub status: WorkerStatus,
    pub restart_count: u32,
    pub last_heartbeat_at_ms: u64,
    pub heartbeat_timeout_ms: u64,
}

impl SupervisedBridgeWorker {
    pub fn new(
        bridge_id: BridgeId,
        integration_id: IntegrationId,
        started_at_ms: u64,
        heartbeat_timeout_ms: u64,
    ) -> Self {
        Self {
            bridge_id,
            integration_id,
            status: WorkerStatus::Starting,
            restart_count: 0,
            last_heartbeat_at_ms: started_at_ms,
            heartbeat_timeout_ms,
        }
    }

    pub fn mark_heartbeat(&mut self, now_ms: u64) {
        self.status = WorkerStatus::Running;
        self.last_heartbeat_at_ms = now_ms;
    }

    pub fn is_overdue_at(&self, now_ms: u64) -> bool {
        matches!(
            self.status,
            WorkerStatus::Starting | WorkerStatus::Running | WorkerStatus::Unhealthy
        ) && now_ms
            >= self
                .last_heartbeat_at_ms
                .saturating_add(self.heartbeat_timeout_ms)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeSupervisor {
    workers: BTreeMap<BridgeId, SupervisedBridgeWorker>,
}

impl RuntimeSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_worker(
        &mut self,
        worker: SupervisedBridgeWorker,
    ) -> Option<SupervisedBridgeWorker> {
        self.workers.insert(worker.bridge_id.clone(), worker)
    }

    pub fn worker(&self, bridge_id: &BridgeId) -> Option<&SupervisedBridgeWorker> {
        self.workers.get(bridge_id)
    }

    pub fn mark_heartbeat(
        &mut self,
        bridge_id: &BridgeId,
        now_ms: u64,
    ) -> Result<(), RuntimeError> {
        let worker = self
            .workers
            .get_mut(bridge_id)
            .ok_or_else(|| RuntimeError::UnknownBridge(bridge_id.clone()))?;
        worker.mark_heartbeat(now_ms);
        Ok(())
    }

    pub fn workers_needing_restart_at(&self, now_ms: u64) -> Vec<&SupervisedBridgeWorker> {
        self.workers
            .values()
            .filter(|worker| worker.is_overdue_at(now_ms))
            .collect()
    }

    pub fn mark_restart_requested(
        &mut self,
        bridge_id: &BridgeId,
    ) -> Result<SupervisedBridgeWorker, RuntimeError> {
        let worker = self
            .workers
            .get_mut(bridge_id)
            .ok_or_else(|| RuntimeError::UnknownBridge(bridge_id.clone()))?;
        worker.status = WorkerStatus::Restarting;
        worker.restart_count = worker.restart_count.saturating_add(1);
        Ok(worker.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeHealthReport {
    pub event_id: EventId,
    pub bridge_id: BridgeId,
    pub health: Health,
    pub observed_at_ms: u64,
    pub received_at_ms: u64,
    pub metadata: Vec<Metadata>,
}

#[derive(Debug, Clone)]
pub struct SmartHomeRuntime {
    registry: InMemorySmartHomeRegistry,
    event_bus: RuntimeEventBus,
    supervisor: RuntimeSupervisor,
    optimistic_states: BTreeMap<EntityId, StateSnapshot>,
}

impl SmartHomeRuntime {
    pub fn new() -> Self {
        Self {
            registry: InMemorySmartHomeRegistry::new(),
            event_bus: RuntimeEventBus::new(),
            supervisor: RuntimeSupervisor::new(),
            optimistic_states: BTreeMap::new(),
        }
    }

    pub fn registry(&self) -> &InMemorySmartHomeRegistry {
        &self.registry
    }

    pub fn registry_mut(&mut self) -> &mut InMemorySmartHomeRegistry {
        &mut self.registry
    }

    pub fn event_bus(&self) -> &RuntimeEventBus {
        &self.event_bus
    }

    pub fn event_bus_mut(&mut self) -> &mut RuntimeEventBus {
        &mut self.event_bus
    }

    pub fn supervisor(&self) -> &RuntimeSupervisor {
        &self.supervisor
    }

    pub fn supervisor_mut(&mut self) -> &mut RuntimeSupervisor {
        &mut self.supervisor
    }

    pub fn optimistic_state_count(&self) -> usize {
        self.optimistic_states.len()
    }

    pub fn upsert_bridge(&mut self, bridge: Bridge) -> Result<Option<Bridge>, RuntimeError> {
        self.registry.upsert_bridge(bridge).map_err(Into::into)
    }

    pub fn upsert_device(&mut self, device: Device) -> Result<Option<Device>, RuntimeError> {
        self.registry.upsert_device(device).map_err(Into::into)
    }

    pub fn upsert_entity(&mut self, entity: Entity) -> Result<Option<Entity>, RuntimeError> {
        self.registry.upsert_entity(entity).map_err(Into::into)
    }

    pub fn apply_device_event(&mut self, event: DeviceEvent) -> Result<(), RuntimeError> {
        if let Some(entity_id) = &event.entity_id {
            if event.state_delta.is_some() {
                self.optimistic_states.remove(entity_id);
            }
        }
        self.registry.record_event(event.clone())?;
        self.event_bus.publish(RuntimeEvent::Device(event));
        Ok(())
    }

    pub fn apply_bridge_health(&mut self, report: BridgeHealthReport) -> Result<(), RuntimeError> {
        let mut bridge = self
            .registry
            .bridge(&report.bridge_id)
            .cloned()
            .ok_or_else(|| RuntimeError::UnknownBridge(report.bridge_id.clone()))?;
        bridge.health = report.health;
        if report.health == Health::Online {
            bridge.last_seen_at_ms = Some(report.observed_at_ms);
        }
        self.registry.upsert_bridge(bridge)?;

        let mut metadata = report.metadata.clone();
        metadata.push(Metadata::new(
            "smart_home.health",
            health_name(report.health),
        ));
        let event = DeviceEvent {
            event_id: report.event_id.clone(),
            bridge_id: report.bridge_id.clone(),
            device_id: None,
            entity_id: None,
            observed_at_ms: report.observed_at_ms,
            received_at_ms: report.received_at_ms,
            event_type: DeviceEventType::Health,
            state_delta: None,
            raw_ref: None,
            correlation_id: None,
            metadata,
        };
        self.registry.record_event(event.clone())?;
        self.event_bus.publish(RuntimeEvent::Device(event));
        self.event_bus.publish(RuntimeEvent::BridgeHealth {
            event_id: report.event_id,
            bridge_id: report.bridge_id,
            health: report.health,
            observed_at_ms: report.observed_at_ms,
            received_at_ms: report.received_at_ms,
        });
        Ok(())
    }

    pub fn submit_command(
        &mut self,
        command: DeviceCommand,
        now_ms: u64,
    ) -> Result<CommandResult, RuntimeError> {
        let entity = self
            .registry
            .entity(&command.entity_id)
            .cloned()
            .ok_or_else(|| RuntimeError::UnknownEntity(command.entity_id.clone()))?;
        let device = self
            .registry
            .device(&entity.device_id)
            .cloned()
            .ok_or_else(|| RuntimeError::UnknownDevice(entity.device_id.clone()))?;

        validate_command_capabilities(&entity, &command)?;

        if let Some(snapshot) = optimistic_snapshot_for_command(&command, now_ms) {
            self.registry.apply_state_snapshot(snapshot.clone())?;
            self.optimistic_states
                .insert(command.entity_id.clone(), snapshot);
        }

        let result = CommandResult {
            command_id: command.command_id,
            status: CommandStatus::Accepted,
            bridge_id: device.bridge_id,
            correlation_id: command.correlation_id,
            message: Some("accepted for integration dispatch".to_string()),
        };
        self.event_bus
            .publish(RuntimeEvent::CommandResult(result.clone()));
        Ok(result)
    }

    pub fn expire_optimistic_states(&mut self, now_ms: u64) -> Result<Vec<EntityId>, RuntimeError> {
        let stale_ids: Vec<_> = self
            .optimistic_states
            .iter()
            .filter(|(_, snapshot)| snapshot.is_stale_at(now_ms))
            .map(|(entity_id, _)| entity_id.clone())
            .collect();

        for entity_id in &stale_ids {
            if let Some(snapshot) = self.optimistic_states.remove(entity_id) {
                let stale_snapshot = StateSnapshot {
                    confidence: StateConfidence::Stale,
                    received_at_ms: now_ms,
                    ..snapshot
                };
                self.registry.apply_state_snapshot(stale_snapshot)?;
                self.event_bus.publish(RuntimeEvent::StateExpired {
                    entity_id: entity_id.clone(),
                    expired_at_ms: now_ms,
                });
            }
        }

        Ok(stale_ids)
    }

    pub fn reconcile_supervision(&mut self, now_ms: u64) -> Vec<RuntimeEvent> {
        let overdue: Vec<_> = self
            .supervisor
            .workers_needing_restart_at(now_ms)
            .into_iter()
            .map(|worker| (worker.bridge_id.clone(), worker.integration_id.clone()))
            .collect();

        overdue
            .into_iter()
            .filter_map(|(bridge_id, integration_id)| {
                self.supervisor.mark_restart_requested(&bridge_id).ok()?;
                let event = RuntimeEvent::WorkerNeedsRestart {
                    bridge_id,
                    integration_id,
                    overdue_at_ms: now_ms,
                };
                self.event_bus.publish(event.clone());
                Some(event)
            })
            .collect()
    }
}

impl Default for SmartHomeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

pub fn health_name(health: Health) -> &'static str {
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

fn validate_command_capabilities(
    entity: &Entity,
    command: &DeviceCommand,
) -> Result<(), RuntimeError> {
    for required in &command.required_capabilities {
        let capability = entity
            .capabilities
            .iter()
            .find(|capability| capability.capability_id == *required)
            .ok_or_else(|| RuntimeError::UnsupportedCapability {
                entity_id: entity.entity_id.clone(),
                capability_id: required.clone(),
            })?;
        if !matches!(
            capability.mode,
            CapabilityMode::Command | CapabilityMode::ObserveAndCommand
        ) {
            return Err(RuntimeError::ReadOnlyCapability {
                entity_id: entity.entity_id.clone(),
                capability_id: required.clone(),
            });
        }
    }
    Ok(())
}

fn optimistic_snapshot_for_command(command: &DeviceCommand, now_ms: u64) -> Option<StateSnapshot> {
    let capability_id = command.command_type.canonical_capability_id()?;
    let value = match command.command_type {
        CommandType::TurnOn => Value::Bool(true),
        CommandType::TurnOff => Value::Bool(false),
        CommandType::SetBrightness
        | CommandType::SetColor
        | CommandType::SetColorTemperature
        | CommandType::SetLock
        | CommandType::SetThermostatSetpoint => command.arguments.clone(),
        CommandType::RecallScene => return None,
    };

    Some(StateSnapshot {
        entity_id: command.entity_id.clone(),
        value: Value::Object(vec![(capability_id.as_str().to_string(), value)]),
        source: StateSource::OptimisticCommand,
        observed_at_ms: now_ms,
        received_at_ms: now_ms,
        expires_at_ms: Some(now_ms.saturating_add(command.timeout_ms)),
        confidence: StateConfidence::Optimistic,
    })
}

fn event_bridge_id(event: &RuntimeEvent) -> Option<&BridgeId> {
    match event {
        RuntimeEvent::Device(event) => Some(&event.bridge_id),
        RuntimeEvent::CommandResult(result) => Some(&result.bridge_id),
        RuntimeEvent::BridgeHealth { bridge_id, .. }
        | RuntimeEvent::WorkerNeedsRestart { bridge_id, .. } => Some(bridge_id),
        RuntimeEvent::StateExpired { .. } => None,
    }
}

fn event_entity_id(event: &RuntimeEvent) -> Option<&EntityId> {
    match event {
        RuntimeEvent::Device(event) => event.entity_id.as_ref(),
        RuntimeEvent::StateExpired { entity_id, .. } => Some(entity_id),
        RuntimeEvent::CommandResult(_)
        | RuntimeEvent::BridgeHealth { .. }
        | RuntimeEvent::WorkerNeedsRestart { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        BridgeTransport, Capability, CommandId, CorrelationId, EntityKind, IntegrationId,
        ProtocolFamily, ProtocolIdentifier, StateDelta,
    };

    fn bridge(id: &str) -> Bridge {
        let mut bridge = Bridge::new(
            BridgeId::trusted(id),
            IntegrationId::trusted("hue"),
            BridgeTransport::LanHttp,
        );
        bridge
            .identifiers
            .push(ProtocolIdentifier::new(ProtocolFamily::Hue, "bridge", "bridge-native").unwrap());
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
            identifiers: Vec::new(),
            health: Health::Online,
            metadata: Vec::new(),
        }
    }

    fn light_entity(id: &str, device_id: &str, capabilities: Vec<Capability>) -> Entity {
        Entity {
            entity_id: EntityId::trusted(id),
            device_id: DeviceId::trusted(device_id),
            kind: EntityKind::Light,
            name: "Kitchen Light".to_string(),
            capabilities,
            state: None,
            metadata: Vec::new(),
        }
    }

    fn command(command_type: CommandType, arguments: Value) -> DeviceCommand {
        DeviceCommand::new(
            CommandId::trusted("cmd-1"),
            EntityId::trusted("entity-1"),
            command_type,
            arguments,
            "agent:test",
            CorrelationId::trusted("corr-1"),
        )
        .unwrap()
    }

    fn runtime_with_entity(capabilities: Vec<Capability>) -> SmartHomeRuntime {
        let mut runtime = SmartHomeRuntime::new();
        runtime.upsert_bridge(bridge("bridge-1")).unwrap();
        runtime
            .upsert_device(device("device-1", "bridge-1"))
            .unwrap();
        runtime
            .upsert_entity(light_entity("entity-1", "device-1", capabilities))
            .unwrap();
        runtime
    }

    #[test]
    fn unknown_entities_are_rejected_before_dispatch() {
        let mut runtime = SmartHomeRuntime::new();
        let error = runtime
            .submit_command(command(CommandType::TurnOn, Value::Null), 1_000)
            .unwrap_err();

        assert!(matches!(error, RuntimeError::UnknownEntity(_)));
        assert_eq!(runtime.event_bus().published().len(), 0);
    }

    #[test]
    fn unsupported_capabilities_are_rejected() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let error = runtime
            .submit_command(
                command(CommandType::SetBrightness, Value::Percentage(42)),
                1_000,
            )
            .unwrap_err();

        assert!(matches!(error, RuntimeError::UnsupportedCapability { .. }));
        assert!(runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .is_none());
    }

    #[test]
    fn accepted_commands_apply_optimistic_state_and_publish_result() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let subscription = RuntimeSubscriptionId::trusted("commands");
        runtime
            .event_bus_mut()
            .subscribe(subscription.clone(), RuntimeEventFilter::Commands)
            .unwrap();

        let result = runtime
            .submit_command(command(CommandType::TurnOn, Value::Null), 1_000)
            .unwrap();
        let deliveries = runtime.event_bus_mut().drain(&subscription).unwrap();
        let snapshot = runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .unwrap();

        assert_eq!(result.status, CommandStatus::Accepted);
        assert_eq!(snapshot.confidence, StateConfidence::Optimistic);
        assert_eq!(snapshot.source, StateSource::OptimisticCommand);
        assert_eq!(snapshot.expires_at_ms, Some(6_000));
        assert_eq!(deliveries.len(), 1);
        assert_eq!(runtime.optimistic_state_count(), 1);
    }

    #[test]
    fn optimistic_state_expiry_marks_cached_state_stale() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let subscription = RuntimeSubscriptionId::trusted("entity-1");
        runtime
            .event_bus_mut()
            .subscribe(
                subscription.clone(),
                RuntimeEventFilter::Entity(EntityId::trusted("entity-1")),
            )
            .unwrap();
        runtime
            .submit_command(command(CommandType::TurnOn, Value::Null), 1_000)
            .unwrap();

        let expired = runtime.expire_optimistic_states(6_000).unwrap();
        let deliveries = runtime.event_bus_mut().drain(&subscription).unwrap();
        let snapshot = runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .unwrap();

        assert_eq!(expired, vec![EntityId::trusted("entity-1")]);
        assert_eq!(snapshot.confidence, StateConfidence::Stale);
        assert!(matches!(
            deliveries.as_slice(),
            [RuntimeEvent::StateExpired { .. }]
        ));
        assert_eq!(runtime.optimistic_state_count(), 0);
    }

    #[test]
    fn confirmed_events_replace_optimistic_state() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        runtime
            .submit_command(command(CommandType::TurnOn, Value::Null), 1_000)
            .unwrap();

        runtime
            .apply_device_event(DeviceEvent {
                event_id: EventId::trusted("event-1"),
                bridge_id: BridgeId::trusted("bridge-1"),
                device_id: Some(DeviceId::trusted("device-1")),
                entity_id: Some(EntityId::trusted("entity-1")),
                observed_at_ms: 1_100,
                received_at_ms: 1_101,
                event_type: DeviceEventType::Updated,
                state_delta: Some(StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(false),
                }),
                raw_ref: None,
                correlation_id: None,
                metadata: Vec::new(),
            })
            .unwrap();

        let snapshot = runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .unwrap();
        assert_eq!(snapshot.confidence, StateConfidence::Confirmed);
        assert_eq!(runtime.optimistic_state_count(), 0);
    }

    #[test]
    fn health_reports_update_bridge_without_losing_identity() {
        let mut runtime = SmartHomeRuntime::new();
        runtime.upsert_bridge(bridge("bridge-1")).unwrap();

        runtime
            .apply_bridge_health(BridgeHealthReport {
                event_id: EventId::trusted("health-1"),
                bridge_id: BridgeId::trusted("bridge-1"),
                health: Health::Offline,
                observed_at_ms: 2_000,
                received_at_ms: 2_001,
                metadata: Vec::new(),
            })
            .unwrap();

        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap();
        assert_eq!(bridge.health, Health::Offline);
        assert_eq!(bridge.identifiers.len(), 1);
        assert_eq!(runtime.registry().counts().events, 1);
    }

    #[test]
    fn supervisor_marks_overdue_workers_for_restart() {
        let mut runtime = SmartHomeRuntime::new();
        let bridge_id = BridgeId::trusted("bridge-1");
        let subscription = RuntimeSubscriptionId::trusted("supervision");
        runtime
            .event_bus_mut()
            .subscribe(subscription.clone(), RuntimeEventFilter::Supervision)
            .unwrap();
        runtime
            .supervisor_mut()
            .register_worker(SupervisedBridgeWorker::new(
                bridge_id.clone(),
                IntegrationId::trusted("hue"),
                1_000,
                100,
            ));
        runtime
            .supervisor_mut()
            .mark_heartbeat(&bridge_id, 1_025)
            .unwrap();

        assert!(runtime.reconcile_supervision(1_100).is_empty());
        let events = runtime.reconcile_supervision(1_126);
        let deliveries = runtime.event_bus_mut().drain(&subscription).unwrap();
        let worker = runtime.supervisor().worker(&bridge_id).unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(deliveries.len(), 1);
        assert_eq!(worker.status, WorkerStatus::Restarting);
        assert_eq!(worker.restart_count, 1);
        assert!(runtime.reconcile_supervision(1_127).is_empty());
    }
}
