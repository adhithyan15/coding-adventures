//! Deterministic smart-home runtime coordinator.
//!
//! This crate is the first runtime slice above the normalized D23 model. It is
//! intentionally synchronous: actors, transports, and protocol workers can wrap
//! it later, while command validation, event routing, state confidence, and
//! supervision rules remain easy to test.

#![forbid(unsafe_code)]

use smart_home_core::{
    tier_for_command, Bridge, BridgeId, CapabilityId, CapabilityMode, CommandId, CommandResult,
    CommandStatus, CommandType, CorrelationId, Device, DeviceCommand, DeviceEvent, DeviceEventType,
    DeviceId, Entity, EntityId, EventId, Health, IntegrationId, Metadata, StateConfidence,
    StateDelta, StateSnapshot, StateSource, Value,
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
    UnsupportedDesiredState {
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
            Self::UnsupportedDesiredState {
                entity_id,
                capability_id,
            } => write!(
                f,
                "entity {entity_id} desired state for capability {capability_id} cannot be mapped to a command"
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
    DesiredStateDrift {
        bridge_id: BridgeId,
        entity_id: EntityId,
        capability_id: CapabilityId,
        reason: ReconciliationReason,
        detected_at_ms: u64,
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
            Self::Supervision => matches!(
                event,
                RuntimeEvent::DesiredStateDrift { .. } | RuntimeEvent::WorkerNeedsRestart { .. }
            ),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationReason {
    MissingState,
    StaleState,
    Drifted,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesiredEntityState {
    pub entity_id: EntityId,
    pub desired: Vec<StateDelta>,
    pub requested_by: String,
    pub command_timeout_ms: u64,
}

impl DesiredEntityState {
    pub fn new(entity_id: EntityId, desired: Vec<StateDelta>) -> Self {
        Self {
            entity_id,
            desired,
            requested_by: "runtime:desired-state".to_string(),
            command_timeout_ms: 5_000,
        }
    }

    pub fn requested_by(mut self, requested_by: impl Into<String>) -> Self {
        self.requested_by = requested_by.into();
        self
    }

    pub fn with_command_timeout(mut self, command_timeout_ms: u64) -> Self {
        self.command_timeout_ms = command_timeout_ms;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DesiredStateAction {
    CommandIssued {
        entity_id: EntityId,
        capability_id: CapabilityId,
        reason: ReconciliationReason,
        command: DeviceCommand,
        result: CommandResult,
    },
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

#[derive(Debug, Clone, PartialEq)]
pub struct SupervisionTickReport {
    pub ticked_at_ms: u64,
    pub expired_entities: Vec<EntityId>,
    pub desired_state_actions: Vec<DesiredStateAction>,
    pub worker_events: Vec<RuntimeEvent>,
}

impl SupervisionTickReport {
    pub fn is_idle(&self) -> bool {
        self.expired_entities.is_empty()
            && self.desired_state_actions.is_empty()
            && self.worker_events.is_empty()
    }

    pub fn action_count(&self) -> usize {
        self.expired_entities.len() + self.desired_state_actions.len() + self.worker_events.len()
    }
}

#[derive(Debug, Clone)]
pub struct SmartHomeRuntime {
    registry: InMemorySmartHomeRegistry,
    event_bus: RuntimeEventBus,
    supervisor: RuntimeSupervisor,
    optimistic_states: BTreeMap<EntityId, StateSnapshot>,
    desired_states: BTreeMap<EntityId, DesiredEntityState>,
}

impl SmartHomeRuntime {
    pub fn new() -> Self {
        Self {
            registry: InMemorySmartHomeRegistry::new(),
            event_bus: RuntimeEventBus::new(),
            supervisor: RuntimeSupervisor::new(),
            optimistic_states: BTreeMap::new(),
            desired_states: BTreeMap::new(),
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

    pub fn desired_state_count(&self) -> usize {
        self.desired_states.len()
    }

    pub fn desired_state(&self, entity_id: &EntityId) -> Option<&DesiredEntityState> {
        self.desired_states.get(entity_id)
    }

    pub fn upsert_desired_state(
        &mut self,
        desired_state: DesiredEntityState,
    ) -> Result<Option<DesiredEntityState>, RuntimeError> {
        let entity = self
            .registry
            .entity(&desired_state.entity_id)
            .ok_or_else(|| RuntimeError::UnknownEntity(desired_state.entity_id.clone()))?;
        validate_desired_state(entity, &desired_state)?;
        Ok(self
            .desired_states
            .insert(desired_state.entity_id.clone(), desired_state))
    }

    pub fn remove_desired_state(&mut self, entity_id: &EntityId) -> Option<DesiredEntityState> {
        self.desired_states.remove(entity_id)
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

    pub fn reconcile_desired_states(
        &mut self,
        now_ms: u64,
    ) -> Result<Vec<DesiredStateAction>, RuntimeError> {
        let mut planned_commands = Vec::new();

        for desired_state in self.desired_states.values() {
            let entity = self
                .registry
                .entity(&desired_state.entity_id)
                .cloned()
                .ok_or_else(|| RuntimeError::UnknownEntity(desired_state.entity_id.clone()))?;
            validate_desired_state(&entity, desired_state)?;
            let device = self
                .registry
                .device(&entity.device_id)
                .cloned()
                .ok_or_else(|| RuntimeError::UnknownDevice(entity.device_id.clone()))?;
            let snapshot = self.registry.state(&desired_state.entity_id).cloned();

            for desired in &desired_state.desired {
                let Some(reason) = desired_state_reason(snapshot.as_ref(), desired, now_ms) else {
                    continue;
                };
                let command = command_for_desired_state(
                    desired_state,
                    desired,
                    reason,
                    now_ms,
                    planned_commands.len(),
                )?;
                planned_commands.push((
                    device.bridge_id.clone(),
                    desired_state.entity_id.clone(),
                    desired.capability_id.clone(),
                    reason,
                    command,
                ));
            }
        }

        let mut actions = Vec::with_capacity(planned_commands.len());
        for (bridge_id, entity_id, capability_id, reason, command) in planned_commands {
            self.event_bus.publish(RuntimeEvent::DesiredStateDrift {
                bridge_id,
                entity_id: entity_id.clone(),
                capability_id: capability_id.clone(),
                reason,
                detected_at_ms: now_ms,
            });
            let result = self.submit_command(command.clone(), now_ms)?;
            actions.push(DesiredStateAction::CommandIssued {
                entity_id,
                capability_id,
                reason,
                command,
                result,
            });
        }

        Ok(actions)
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

    pub fn run_supervision_tick(
        &mut self,
        now_ms: u64,
    ) -> Result<SupervisionTickReport, RuntimeError> {
        let expired_entities = self.expire_optimistic_states(now_ms)?;
        let desired_state_actions = self.reconcile_desired_states(now_ms)?;
        let worker_events = self.reconcile_supervision(now_ms);

        Ok(SupervisionTickReport {
            ticked_at_ms: now_ms,
            expired_entities,
            desired_state_actions,
            worker_events,
        })
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

fn validate_desired_state(
    entity: &Entity,
    desired_state: &DesiredEntityState,
) -> Result<(), RuntimeError> {
    for desired in &desired_state.desired {
        let capability = entity
            .capabilities
            .iter()
            .find(|capability| capability.capability_id == desired.capability_id)
            .ok_or_else(|| RuntimeError::UnsupportedCapability {
                entity_id: entity.entity_id.clone(),
                capability_id: desired.capability_id.clone(),
            })?;
        if !matches!(
            capability.mode,
            CapabilityMode::Command | CapabilityMode::ObserveAndCommand
        ) {
            return Err(RuntimeError::ReadOnlyCapability {
                entity_id: entity.entity_id.clone(),
                capability_id: desired.capability_id.clone(),
            });
        }
    }
    Ok(())
}

fn desired_state_reason(
    snapshot: Option<&StateSnapshot>,
    desired: &StateDelta,
    now_ms: u64,
) -> Option<ReconciliationReason> {
    let Some(snapshot) = snapshot else {
        return Some(ReconciliationReason::MissingState);
    };
    if snapshot.is_stale_at(now_ms) {
        return Some(ReconciliationReason::StaleState);
    }
    match snapshot_value_for(snapshot, &desired.capability_id) {
        None => Some(ReconciliationReason::MissingState),
        Some(current) if current == &desired.value => None,
        Some(_) => Some(ReconciliationReason::Drifted),
    }
}

fn snapshot_value_for<'a>(
    snapshot: &'a StateSnapshot,
    capability_id: &CapabilityId,
) -> Option<&'a Value> {
    match &snapshot.value {
        Value::Object(fields) => fields
            .iter()
            .find(|(key, _)| key == capability_id.as_str())
            .map(|(_, value)| value),
        value => Some(value),
    }
}

fn command_for_desired_state(
    desired_state: &DesiredEntityState,
    desired: &StateDelta,
    reason: ReconciliationReason,
    now_ms: u64,
    sequence: usize,
) -> Result<DeviceCommand, RuntimeError> {
    let command_type = command_type_for_desired_state(&desired_state.entity_id, desired)?;
    let arguments = match command_type {
        CommandType::TurnOn | CommandType::TurnOff => Value::Null,
        CommandType::SetBrightness
        | CommandType::SetColor
        | CommandType::SetColorTemperature
        | CommandType::SetLock
        | CommandType::SetThermostatSetpoint => desired.value.clone(),
        CommandType::RecallScene => Value::Null,
    };
    let command_id = CommandId::trusted(format!(
        "reconcile:{}:{}:{now_ms}:{sequence}",
        desired_state.entity_id.as_str(),
        desired.capability_id.as_str()
    ));
    let correlation_id = CorrelationId::trusted(format!(
        "desired-state:{}:{}:{now_ms}",
        desired_state.entity_id.as_str(),
        desired.capability_id.as_str()
    ));
    let required_capability = command_type.canonical_capability_id().ok_or_else(|| {
        RuntimeError::UnsupportedDesiredState {
            entity_id: desired_state.entity_id.clone(),
            capability_id: desired.capability_id.clone(),
        }
    })?;

    Ok(DeviceCommand {
        command_id,
        entity_id: desired_state.entity_id.clone(),
        command_type,
        arguments,
        requested_by: desired_state.requested_by.clone(),
        idempotency_key: Some(format!(
            "desired-state:{}:{}:{}",
            desired_state.entity_id.as_str(),
            desired.capability_id.as_str(),
            reconciliation_reason_name(reason)
        )),
        required_tier: tier_for_command(command_type),
        required_capabilities: vec![required_capability],
        timeout_ms: desired_state.command_timeout_ms,
        correlation_id,
    })
}

fn command_type_for_desired_state(
    entity_id: &EntityId,
    desired: &StateDelta,
) -> Result<CommandType, RuntimeError> {
    match desired.capability_id.as_str() {
        "light.on_off" => match &desired.value {
            Value::Bool(true) => Ok(CommandType::TurnOn),
            Value::Bool(false) => Ok(CommandType::TurnOff),
            _ => Err(RuntimeError::UnsupportedDesiredState {
                entity_id: entity_id.clone(),
                capability_id: desired.capability_id.clone(),
            }),
        },
        "light.brightness" => Ok(CommandType::SetBrightness),
        "light.color" => Ok(CommandType::SetColor),
        "light.color_temperature" => Ok(CommandType::SetColorTemperature),
        "lock.state" => Ok(CommandType::SetLock),
        "climate.setpoint" => Ok(CommandType::SetThermostatSetpoint),
        _ => Err(RuntimeError::UnsupportedDesiredState {
            entity_id: entity_id.clone(),
            capability_id: desired.capability_id.clone(),
        }),
    }
}

fn reconciliation_reason_name(reason: ReconciliationReason) -> &'static str {
    match reason {
        ReconciliationReason::MissingState => "missing",
        ReconciliationReason::StaleState => "stale",
        ReconciliationReason::Drifted => "drifted",
    }
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
        | RuntimeEvent::DesiredStateDrift { bridge_id, .. }
        | RuntimeEvent::WorkerNeedsRestart { bridge_id, .. } => Some(bridge_id),
        RuntimeEvent::StateExpired { .. } => None,
    }
}

fn event_entity_id(event: &RuntimeEvent) -> Option<&EntityId> {
    match event {
        RuntimeEvent::Device(event) => event.entity_id.as_ref(),
        RuntimeEvent::DesiredStateDrift { entity_id, .. } => Some(entity_id),
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
    fn desired_state_reconciliation_noops_when_state_matches() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        runtime
            .registry_mut()
            .apply_state_snapshot(StateSnapshot {
                entity_id: EntityId::trusted("entity-1"),
                value: Value::Object(vec![("light.on_off".to_string(), Value::Bool(true))]),
                source: StateSource::EventStream,
                observed_at_ms: 1_000,
                received_at_ms: 1_001,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            })
            .unwrap();
        runtime
            .upsert_desired_state(DesiredEntityState::new(
                EntityId::trusted("entity-1"),
                vec![StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(true),
                }],
            ))
            .unwrap();

        let actions = runtime.reconcile_desired_states(2_000).unwrap();

        assert!(actions.is_empty());
        assert_eq!(runtime.event_bus().published().len(), 0);
        assert_eq!(runtime.desired_state_count(), 1);
    }

    #[test]
    fn desired_state_reconciliation_commands_drift_back_to_target() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let subscription = RuntimeSubscriptionId::trusted("supervision");
        runtime
            .event_bus_mut()
            .subscribe(subscription.clone(), RuntimeEventFilter::Supervision)
            .unwrap();
        runtime
            .registry_mut()
            .apply_state_snapshot(StateSnapshot {
                entity_id: EntityId::trusted("entity-1"),
                value: Value::Object(vec![("light.on_off".to_string(), Value::Bool(false))]),
                source: StateSource::EventStream,
                observed_at_ms: 1_000,
                received_at_ms: 1_001,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            })
            .unwrap();
        runtime
            .upsert_desired_state(
                DesiredEntityState::new(
                    EntityId::trusted("entity-1"),
                    vec![StateDelta {
                        capability_id: CapabilityId::trusted("light.on_off"),
                        value: Value::Bool(true),
                    }],
                )
                .requested_by("agent:supervisor"),
            )
            .unwrap();

        let actions = runtime.reconcile_desired_states(2_000).unwrap();
        let deliveries = runtime.event_bus_mut().drain(&subscription).unwrap();
        let snapshot = runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .unwrap();

        assert!(matches!(
            actions.as_slice(),
            [DesiredStateAction::CommandIssued {
                capability_id,
                reason: ReconciliationReason::Drifted,
                command,
                result,
                ..
            }] if capability_id == &CapabilityId::trusted("light.on_off")
                && command.command_type == CommandType::TurnOn
                && command.requested_by == "agent:supervisor"
                && result.status == CommandStatus::Accepted
        ));
        assert!(matches!(
            deliveries.as_slice(),
            [RuntimeEvent::DesiredStateDrift {
                reason: ReconciliationReason::Drifted,
                ..
            }]
        ));
        assert_eq!(snapshot.confidence, StateConfidence::Optimistic);
        assert_eq!(
            snapshot.value,
            Value::Object(vec![("light.on_off".to_string(), Value::Bool(true))])
        );
    }

    #[test]
    fn desired_state_reconciliation_refreshes_missing_or_stale_state() {
        let mut runtime = runtime_with_entity(vec![Capability::light_brightness()]);
        runtime
            .upsert_desired_state(
                DesiredEntityState::new(
                    EntityId::trusted("entity-1"),
                    vec![StateDelta {
                        capability_id: CapabilityId::trusted("light.brightness"),
                        value: Value::Percentage(64),
                    }],
                )
                .with_command_timeout(250),
            )
            .unwrap();

        let missing = runtime.reconcile_desired_states(2_000).unwrap();
        let stale = runtime.reconcile_desired_states(2_250).unwrap();

        assert!(matches!(
            missing.as_slice(),
            [DesiredStateAction::CommandIssued {
                reason: ReconciliationReason::MissingState,
                command,
                ..
            }] if command.command_type == CommandType::SetBrightness
                && command.timeout_ms == 250
        ));
        assert!(matches!(
            stale.as_slice(),
            [DesiredStateAction::CommandIssued {
                reason: ReconciliationReason::StaleState,
                ..
            }]
        ));
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

    #[test]
    fn supervision_tick_runs_expiry_reconciliation_and_worker_restart() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
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
                50,
            ));
        let mut command = command(CommandType::TurnOn, Value::Null);
        command.timeout_ms = 50;
        runtime.submit_command(command, 1_000).unwrap();
        runtime
            .upsert_desired_state(DesiredEntityState::new(
                EntityId::trusted("entity-1"),
                vec![StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(true),
                }],
            ))
            .unwrap();

        let report = runtime.run_supervision_tick(1_050).unwrap();
        let deliveries = runtime.event_bus_mut().drain(&subscription).unwrap();
        let worker = runtime.supervisor().worker(&bridge_id).unwrap();

        assert_eq!(report.ticked_at_ms, 1_050);
        assert_eq!(report.expired_entities, vec![EntityId::trusted("entity-1")]);
        assert!(matches!(
            report.desired_state_actions.as_slice(),
            [DesiredStateAction::CommandIssued {
                reason: ReconciliationReason::StaleState,
                command,
                ..
            }] if command.command_type == CommandType::TurnOn
        ));
        assert!(matches!(
            report.worker_events.as_slice(),
            [RuntimeEvent::WorkerNeedsRestart { bridge_id: event_bridge_id, .. }]
                if event_bridge_id == &bridge_id
        ));
        assert_eq!(report.action_count(), 3);
        assert!(!report.is_idle());
        assert_eq!(worker.status, WorkerStatus::Restarting);
        assert_eq!(worker.restart_count, 1);
        assert_eq!(deliveries.len(), 2);
    }

    #[test]
    fn supervision_tick_reports_idle_when_no_work_is_due() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);

        let report = runtime.run_supervision_tick(1_000).unwrap();

        assert_eq!(report.ticked_at_ms, 1_000);
        assert!(report.is_idle());
        assert_eq!(report.action_count(), 0);
    }
}
