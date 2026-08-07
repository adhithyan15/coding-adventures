//! Deterministic smart-home runtime coordinator.
//!
//! This crate is the first runtime slice above the normalized D23 model. It is
//! intentionally synchronous: actors, transports, and protocol workers can wrap
//! it later, while command validation, event routing, state confidence, and
//! supervision rules remain easy to test.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use smart_home_core::{
    tier_for_command, AgentId, AuthorizationDecision, AuthorizationDecisionLogSummary,
    AuthorizationOutcome, Bridge, BridgeId, Capability, CapabilityGrant,
    CapabilityGrantInventorySummary, CapabilityGrantScope, CapabilityGrantStatus, CapabilityId,
    CapabilityMode, CommandId, CommandResult, CommandStatus, CommandType, CorrelationId, Device,
    DeviceCommand, DeviceControlCommandType, DeviceEvent, DeviceEventType, DeviceId, Entity,
    EntityId, EventId, Health,
    IntegrationId, Metadata, PrivilegeTier, Scene, SceneId, SceneScope, SmartHomeError,
    MediaCommandType, SmartHomeTool, StateConfidence, StateDelta, StateSnapshot, StateSource, Value,
    VaultRef,
};
use smart_home_discovery::{
    run_mdns_worker_scan_plan_with_executor, DiscoveryCatalog, DiscoveryError,
    DiscoveryPairingPlan, DiscoveryPairingPlanOptions, DiscoveryPairingPlanSummary,
    DiscoveryRecord, DiscoveryRecordSummary, DiscoverySignalSummary, DiscoverySource,
    DiscoveryUpsert, DiscoveryWorkerFailure, DiscoveryWorkerId, DiscoveryWorkerKind,
    DiscoveryWorkerRun, DiscoveryWorkerRunStatus, DiscoveryWorkerRunSummary, MdnsScanNetwork,
    MdnsWorkerScanExecutor, MdnsWorkerScanPlan, MdnsWorkerScanReport, MdnsWorkerScanRequest,
    UdpMdnsWorkerScanExecutor, MDNS_DISCOVERY_SERVICE_TYPE_METADATA_KEY,
};
use smart_home_integration_catalog::first_party_catalog;
use smart_home_registry::{
    AuthorizationDecisionSelector, DeviceSelector, InMemorySmartHomeRegistry, RegistryCounts,
    RegistryError, RegistryTopologySummary, StateRefreshPlan, StateRefreshReason,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::{fmt, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Registry(Box<RegistryError>),
    Core(Box<SmartHomeError>),
    Discovery(Box<DiscoveryError>),
    UnknownBridge(BridgeId),
    UnknownDevice(DeviceId),
    UnknownEntity(EntityId),
    UnknownScene(SceneId),
    UnknownPairingSession(RuntimePairingSessionId),
    UnknownSubscription(RuntimeSubscriptionId),
    UnknownDiscoveryWorker(DiscoveryWorkerId),
    DuplicatePairingSession(RuntimePairingSessionId),
    DuplicateSubscription(RuntimeSubscriptionId),
    InvalidDiscoveryWorkerSchedule {
        worker_id: DiscoveryWorkerId,
        field: &'static str,
        message: String,
    },
    DiscoveryWorkerRunMismatch {
        worker_id: DiscoveryWorkerId,
        expected_integration_id: IntegrationId,
        actual_integration_id: IntegrationId,
        expected_kind: DiscoveryWorkerKind,
        actual_kind: DiscoveryWorkerKind,
    },
    PairingSessionExpired {
        session_id: RuntimePairingSessionId,
        expired_at_ms: u64,
        now_ms: u64,
    },
    PairingSessionNotPending {
        session_id: RuntimePairingSessionId,
        status: PairingSessionStatus,
    },
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
    UnauthorizedCommand {
        command_id: CommandId,
        principal_id: AgentId,
        required_tier: PrivilegeTier,
        missing_capabilities: Vec<CapabilityId>,
    },
    UnauthorizedTool {
        principal_id: AgentId,
        tool: SmartHomeTool,
        missing_capabilities: Vec<CapabilityId>,
    },
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => write!(f, "{error}"),
            Self::Core(error) => write!(f, "{error}"),
            Self::Discovery(error) => write!(f, "{error}"),
            Self::UnknownBridge(id) => write!(f, "unknown runtime bridge {id}"),
            Self::UnknownDevice(id) => write!(f, "unknown runtime device {id}"),
            Self::UnknownEntity(id) => write!(f, "unknown runtime entity {id}"),
            Self::UnknownScene(id) => write!(f, "unknown runtime scene {id}"),
            Self::UnknownPairingSession(id) => write!(f, "unknown runtime pairing session {id}"),
            Self::UnknownSubscription(id) => write!(f, "unknown runtime subscription {id}"),
            Self::UnknownDiscoveryWorker(id) => write!(f, "unknown discovery worker {id}"),
            Self::DuplicatePairingSession(id) => write!(f, "duplicate runtime pairing session {id}"),
            Self::DuplicateSubscription(id) => write!(f, "duplicate runtime subscription {id}"),
            Self::InvalidDiscoveryWorkerSchedule {
                worker_id,
                field,
                message,
            } => write!(
                f,
                "discovery worker {worker_id} has invalid schedule field `{field}`: {message}"
            ),
            Self::DiscoveryWorkerRunMismatch {
                worker_id,
                expected_integration_id,
                actual_integration_id,
                expected_kind,
                actual_kind,
            } => write!(
                f,
                "discovery worker {worker_id} expected `{expected_integration_id}` {expected_kind} run but received `{actual_integration_id}` {actual_kind}"
            ),
            Self::PairingSessionExpired {
                session_id,
                expired_at_ms,
                now_ms,
            } => write!(
                f,
                "pairing session {session_id} expired at {expired_at_ms} before {now_ms}"
            ),
            Self::PairingSessionNotPending { session_id, status } => write!(
                f,
                "pairing session {session_id} cannot complete while {status:?}"
            ),
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
            Self::UnauthorizedCommand {
                command_id,
                principal_id,
                required_tier,
                missing_capabilities,
            } => write!(
                f,
                "agent {principal_id} is not authorized for command {command_id} at tier {required_tier:?}; missing grants for {missing_capabilities:?}"
            ),
            Self::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities,
            } => write!(
                f,
                "agent {principal_id} is not authorized for tool {tool:?}; missing grants for {missing_capabilities:?}"
            ),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<DiscoveryError> for RuntimeError {
    fn from(error: DiscoveryError) -> Self {
        Self::Discovery(Box::new(error))
    }
}

impl From<RegistryError> for RuntimeError {
    fn from(error: RegistryError) -> Self {
        Self::Registry(Box::new(error))
    }
}

impl From<SmartHomeError> for RuntimeError {
    fn from(error: SmartHomeError) -> Self {
        Self::Core(Box::new(error))
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuntimePairingSessionId(String);

impl RuntimePairingSessionId {
    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuntimePairingSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Replay cursor into the runtime event log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuntimeEventCheckpoint {
    next_sequence: u64,
}

impl RuntimeEventCheckpoint {
    pub fn start() -> Self {
        Self { next_sequence: 0 }
    }

    pub fn from_next_sequence(next_sequence: u64) -> Self {
        Self { next_sequence }
    }

    pub fn next_sequence(self) -> u64 {
        self.next_sequence
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
        self.subscribe_from_checkpoint(subscription_id, filter, self.checkpoint())
    }

    pub fn subscribe_from_checkpoint(
        &mut self,
        subscription_id: RuntimeSubscriptionId,
        filter: RuntimeEventFilter,
        checkpoint: RuntimeEventCheckpoint,
    ) -> Result<(), RuntimeError> {
        if self.subscriptions.contains_key(&subscription_id) {
            return Err(RuntimeError::DuplicateSubscription(subscription_id));
        }
        let replay = self.replay_from(checkpoint, &filter);
        self.subscriptions.insert(subscription_id.clone(), filter);
        self.deliveries
            .insert(subscription_id, replay.into_iter().collect());
        Ok(())
    }

    pub fn has_subscription(&self, subscription_id: &RuntimeSubscriptionId) -> bool {
        self.subscriptions.contains_key(subscription_id)
    }

    pub fn unsubscribe(
        &mut self,
        subscription_id: &RuntimeSubscriptionId,
    ) -> Result<RuntimeEventDeliveryBatch, RuntimeError> {
        if self.subscriptions.remove(subscription_id).is_none() {
            return Err(RuntimeError::UnknownSubscription(subscription_id.clone()));
        }

        let events = self
            .deliveries
            .remove(subscription_id)
            .map(|queue| queue.into_iter().collect::<Vec<_>>())
            .unwrap_or_default();

        Ok(RuntimeEventDeliveryBatch {
            subscription_id: subscription_id.clone(),
            remaining_events: 0,
            events,
        })
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

    pub fn checkpoint(&self) -> RuntimeEventCheckpoint {
        RuntimeEventCheckpoint::from_next_sequence(self.published.len() as u64)
    }

    pub fn replay_from(
        &self,
        checkpoint: RuntimeEventCheckpoint,
        filter: &RuntimeEventFilter,
    ) -> Vec<RuntimeEvent> {
        let start = checkpoint.next_sequence.min(self.published.len() as u64) as usize;
        self.published
            .iter()
            .skip(start)
            .filter(|event| filter.matches(event))
            .cloned()
            .collect()
    }

    pub fn query_events(&self, query: &RuntimeEventQuery) -> Vec<RuntimeEventLogEntry<'_>> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let start = query
            .from_checkpoint
            .next_sequence
            .min(self.published.len() as u64) as usize;
        let mut entries = self
            .published
            .iter()
            .enumerate()
            .skip(start)
            .filter(|(index, _)| {
                query
                    .to_sequence
                    .is_none_or(|to_sequence| *index as u64 <= to_sequence)
            })
            .filter(|(_, event)| {
                query
                    .filter
                    .as_ref()
                    .is_none_or(|filter| filter.matches(event))
            })
            .map(|(index, event)| RuntimeEventLogEntry {
                sequence: index as u64,
                next_checkpoint: RuntimeEventCheckpoint::from_next_sequence(index as u64 + 1),
                event,
            })
            .collect::<Vec<_>>();
        if query.sort == RuntimeEventSort::SequenceDesc {
            entries.reverse();
        }
        apply_limit(&mut entries, query.limit);
        entries
    }

    pub fn event_log_summary(&self, query: &RuntimeEventQuery) -> RuntimeEventLogSummary {
        let entries = self.query_events(query);
        RuntimeEventLogSummary::from_entries(entries.iter().copied())
    }

    pub fn subscription_snapshots(&self) -> Vec<RuntimeSubscriptionSnapshot> {
        self.query_subscriptions(&RuntimeSubscriptionQuery::new())
    }

    pub fn query_subscriptions(
        &self,
        query: &RuntimeSubscriptionQuery,
    ) -> Vec<RuntimeSubscriptionSnapshot> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut snapshots = self
            .subscriptions
            .iter()
            .filter(|(subscription_id, filter)| {
                query
                    .subscription_id
                    .as_ref()
                    .is_none_or(|expected| subscription_id == &expected)
                    && query
                        .filter
                        .as_ref()
                        .is_none_or(|expected| filter == &expected)
            })
            .map(|(subscription_id, filter)| RuntimeSubscriptionSnapshot {
                subscription_id: subscription_id.clone(),
                filter: filter.clone(),
                queued_events: self
                    .deliveries
                    .get(subscription_id)
                    .map_or(0, VecDeque::len),
            })
            .filter(|snapshot| {
                query
                    .min_queued_events
                    .is_none_or(|minimum| snapshot.queued_events >= minimum)
            })
            .collect::<Vec<_>>();
        match query.sort {
            RuntimeSubscriptionSort::SubscriptionId => snapshots.sort_by(|left, right| {
                left.subscription_id
                    .as_str()
                    .cmp(right.subscription_id.as_str())
            }),
            RuntimeSubscriptionSort::QueuedEventsDesc => snapshots.sort_by(|left, right| {
                right.queued_events.cmp(&left.queued_events).then_with(|| {
                    left.subscription_id
                        .as_str()
                        .cmp(right.subscription_id.as_str())
                })
            }),
        }
        apply_limit(&mut snapshots, query.limit);
        snapshots
    }

    pub fn subscription_inventory_summary(
        &self,
        query: &RuntimeSubscriptionQuery,
    ) -> RuntimeSubscriptionInventorySummary {
        let snapshots = self.query_subscriptions(query);
        RuntimeSubscriptionInventorySummary::from_snapshots(&snapshots)
    }

    pub fn queued_events(
        &self,
        subscription_id: &RuntimeSubscriptionId,
    ) -> Result<usize, RuntimeError> {
        self.deliveries
            .get(subscription_id)
            .map(VecDeque::len)
            .ok_or_else(|| RuntimeError::UnknownSubscription(subscription_id.clone()))
    }

    pub fn peek_deliveries(
        &self,
        subscription_id: &RuntimeSubscriptionId,
        options: RuntimeEventDeliveryOptions,
    ) -> Result<RuntimeEventDeliveryBatch, RuntimeError> {
        let queue = self
            .deliveries
            .get(subscription_id)
            .ok_or_else(|| RuntimeError::UnknownSubscription(subscription_id.clone()))?;
        let count = delivery_count(queue.len(), options.limit);
        let events = queue.iter().take(count).cloned().collect::<Vec<_>>();
        Ok(RuntimeEventDeliveryBatch {
            subscription_id: subscription_id.clone(),
            remaining_events: queue.len().saturating_sub(events.len()),
            events,
        })
    }

    pub fn drain_deliveries(
        &mut self,
        subscription_id: &RuntimeSubscriptionId,
        options: RuntimeEventDeliveryOptions,
    ) -> Result<RuntimeEventDeliveryBatch, RuntimeError> {
        let queue = self
            .deliveries
            .get_mut(subscription_id)
            .ok_or_else(|| RuntimeError::UnknownSubscription(subscription_id.clone()))?;
        let count = delivery_count(queue.len(), options.limit);
        let events = queue.drain(..count).collect::<Vec<_>>();
        Ok(RuntimeEventDeliveryBatch {
            subscription_id: subscription_id.clone(),
            remaining_events: queue.len(),
            events,
        })
    }

    pub fn drain(
        &mut self,
        subscription_id: &RuntimeSubscriptionId,
    ) -> Result<Vec<RuntimeEvent>, RuntimeError> {
        Ok(self
            .drain_deliveries(subscription_id, RuntimeEventDeliveryOptions::new())?
            .events)
    }

    pub fn published(&self) -> &[RuntimeEvent] {
        &self.published
    }

    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    pub fn pending_delivery_count(&self) -> usize {
        self.deliveries.values().map(VecDeque::len).sum()
    }

    pub fn snapshot(&self) -> RuntimeEventBusSnapshot {
        let pending_delivery_count = self.pending_delivery_count();
        let backlogged_subscription_count = self
            .deliveries
            .values()
            .filter(|queue| !queue.is_empty())
            .count();
        let max_pending_delivery_count = self
            .deliveries
            .values()
            .map(VecDeque::len)
            .max()
            .unwrap_or(0);
        RuntimeEventBusSnapshot {
            subscription_count: self.subscription_count(),
            pending_delivery_count,
            published_event_count: self.published.len(),
            backlogged_subscription_count,
            max_pending_delivery_count,
        }
    }

    pub fn health_summary(&self) -> RuntimeEventBusHealthSummary {
        RuntimeEventBusHealthSummary {
            snapshot: self.snapshot(),
            subscriptions: self.subscription_inventory_summary(&RuntimeSubscriptionQuery::new()),
            event_log: self.event_log_summary(&RuntimeEventQuery::new()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeEventBusSnapshot {
    pub subscription_count: usize,
    pub pending_delivery_count: usize,
    pub published_event_count: usize,
    pub backlogged_subscription_count: usize,
    pub max_pending_delivery_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEventBusBacklogStatus {
    NoSubscriptions,
    CaughtUp,
    Backlogged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEventBusPressureStatus {
    NoSubscriptions,
    CaughtUp,
    PartiallyBacklogged,
    FullyBacklogged,
}

impl RuntimeEventBusSnapshot {
    pub fn is_idle(&self) -> bool {
        self.pending_delivery_count == 0
    }

    pub fn has_subscriptions(&self) -> bool {
        self.subscription_count > 0
    }

    pub fn has_backlog(&self) -> bool {
        self.pending_delivery_count > 0 || self.has_lagging_subscriptions()
    }

    pub fn has_lagging_subscriptions(&self) -> bool {
        self.backlogged_subscription_count > 0
    }

    pub fn backlog_status(&self) -> RuntimeEventBusBacklogStatus {
        if !self.has_subscriptions() {
            RuntimeEventBusBacklogStatus::NoSubscriptions
        } else if self.has_backlog() {
            RuntimeEventBusBacklogStatus::Backlogged
        } else {
            RuntimeEventBusBacklogStatus::CaughtUp
        }
    }

    pub fn pressure_status(&self) -> RuntimeEventBusPressureStatus {
        if !self.has_subscriptions() {
            RuntimeEventBusPressureStatus::NoSubscriptions
        } else if !self.has_lagging_subscriptions() {
            RuntimeEventBusPressureStatus::CaughtUp
        } else if self.backlogged_subscription_count >= self.subscription_count {
            RuntimeEventBusPressureStatus::FullyBacklogged
        } else {
            RuntimeEventBusPressureStatus::PartiallyBacklogged
        }
    }

    // Explicit `if divisor == 0` guard is intentional (and clearer than checked_div here); allow the 1.97 manual_checked_ops lint.
    #[allow(clippy::manual_checked_ops)]
    pub fn average_pending_deliveries_per_subscription(&self) -> usize {
        if self.subscription_count == 0 {
            0
        } else {
            self.pending_delivery_count / self.subscription_count
        }
    }

    pub fn caught_up_subscription_count(&self) -> usize {
        self.subscription_count
            .saturating_sub(self.backlogged_subscription_count)
    }

    pub fn backlogged_subscription_percent(&self) -> u8 {
        if self.subscription_count == 0 {
            return 0;
        }
        let backlogged = self
            .backlogged_subscription_count
            .min(self.subscription_count);
        ((backlogged.saturating_mul(100) / self.subscription_count).min(100)) as u8
    }

    pub fn exceeds_backlogged_subscription_percent(&self, threshold: u8) -> bool {
        self.backlogged_subscription_percent() > threshold.min(100)
    }

    pub fn exceeds_subscription_backlog_threshold(&self, threshold: usize) -> bool {
        self.max_pending_delivery_count > threshold
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeEventDeliveryOptions {
    pub limit: Option<usize>,
}

impl RuntimeEventDeliveryOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeEventDeliveryBatch {
    pub subscription_id: RuntimeSubscriptionId,
    pub events: Vec<RuntimeEvent>,
    pub remaining_events: usize,
}

impl RuntimeEventDeliveryBatch {
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn has_more(&self) -> bool {
        self.remaining_events > 0
    }

    pub fn summary(&self) -> RuntimeEventDeliverySummary {
        RuntimeEventDeliverySummary::from_batch(self)
    }
}

/// Compact count view over one subscription delivery batch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEventDeliverySummary {
    pub subscription_id: RuntimeSubscriptionId,
    pub delivered_events: usize,
    pub remaining_events: usize,
    pub device_events: usize,
    pub command_results: usize,
    pub bridge_health_events: usize,
    pub state_expired_events: usize,
    pub desired_state_drift_events: usize,
    pub worker_restart_events: usize,
}

impl RuntimeEventDeliverySummary {
    pub fn from_batch(batch: &RuntimeEventDeliveryBatch) -> Self {
        let mut summary = Self {
            subscription_id: batch.subscription_id.clone(),
            delivered_events: batch.events.len(),
            remaining_events: batch.remaining_events,
            device_events: 0,
            command_results: 0,
            bridge_health_events: 0,
            state_expired_events: 0,
            desired_state_drift_events: 0,
            worker_restart_events: 0,
        };

        for event in &batch.events {
            match event {
                RuntimeEvent::Device(_) => summary.device_events += 1,
                RuntimeEvent::CommandResult(_) => summary.command_results += 1,
                RuntimeEvent::BridgeHealth { .. } => summary.bridge_health_events += 1,
                RuntimeEvent::StateExpired { .. } => summary.state_expired_events += 1,
                RuntimeEvent::DesiredStateDrift { .. } => summary.desired_state_drift_events += 1,
                RuntimeEvent::WorkerNeedsRestart { .. } => summary.worker_restart_events += 1,
            }
        }

        summary
    }

    pub fn is_empty(&self) -> bool {
        self.delivered_events == 0
    }

    pub fn has_more(&self) -> bool {
        self.remaining_events > 0
    }

    pub fn has_command_results(&self) -> bool {
        self.command_results > 0
    }

    pub fn has_supervision_events(&self) -> bool {
        self.desired_state_drift_events > 0 || self.worker_restart_events > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RuntimeEventSort {
    #[default]
    SequenceAsc,
    SequenceDesc,
}


/// Borrowed view of one runtime event and its replay cursor position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeEventLogEntry<'a> {
    pub sequence: u64,
    pub next_checkpoint: RuntimeEventCheckpoint,
    pub event: &'a RuntimeEvent,
}

/// Compact count view over a selected runtime event-log window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeEventLogSummary {
    pub total_events: usize,
    pub device_events: usize,
    pub command_results: usize,
    pub bridge_health_events: usize,
    pub state_expired_events: usize,
    pub desired_state_drift_events: usize,
    pub worker_restart_events: usize,
    pub first_sequence: Option<u64>,
    pub latest_sequence: Option<u64>,
    pub next_checkpoint: RuntimeEventCheckpoint,
}

impl Default for RuntimeEventLogSummary {
    fn default() -> Self {
        Self {
            total_events: 0,
            device_events: 0,
            command_results: 0,
            bridge_health_events: 0,
            state_expired_events: 0,
            desired_state_drift_events: 0,
            worker_restart_events: 0,
            first_sequence: None,
            latest_sequence: None,
            next_checkpoint: RuntimeEventCheckpoint::start(),
        }
    }
}

impl RuntimeEventLogSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_entries<'a, I>(entries: I) -> Self
    where
        I: IntoIterator<Item = RuntimeEventLogEntry<'a>>,
    {
        let mut summary = Self::empty();
        for entry in entries {
            summary.total_events += 1;
            summary.first_sequence = Some(
                summary
                    .first_sequence
                    .map(|sequence| sequence.min(entry.sequence))
                    .unwrap_or(entry.sequence),
            );
            summary.latest_sequence = Some(
                summary
                    .latest_sequence
                    .map(|sequence| sequence.max(entry.sequence))
                    .unwrap_or(entry.sequence),
            );
            summary.next_checkpoint = RuntimeEventCheckpoint::from_next_sequence(
                summary
                    .latest_sequence
                    .map(|sequence| sequence.saturating_add(1))
                    .unwrap_or(0),
            );
            match entry.event {
                RuntimeEvent::Device(_) => summary.device_events += 1,
                RuntimeEvent::CommandResult(_) => summary.command_results += 1,
                RuntimeEvent::BridgeHealth { .. } => summary.bridge_health_events += 1,
                RuntimeEvent::StateExpired { .. } => summary.state_expired_events += 1,
                RuntimeEvent::DesiredStateDrift { .. } => summary.desired_state_drift_events += 1,
                RuntimeEvent::WorkerNeedsRestart { .. } => summary.worker_restart_events += 1,
            }
        }
        summary
    }

    pub fn has_events(&self) -> bool {
        self.total_events > 0
    }

    pub fn has_command_results(&self) -> bool {
        self.command_results > 0
    }

    pub fn has_supervision_events(&self) -> bool {
        self.desired_state_drift_events > 0 || self.worker_restart_events > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RuntimeCommandResultSort {
    #[default]
    SequenceAsc,
    SequenceDesc,
    StatusThenSequenceDesc,
}


/// Read-side query for command results already captured in the runtime event log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCommandResultQuery {
    pub command_id: Option<CommandId>,
    pub bridge_id: Option<BridgeId>,
    pub correlation_id: Option<CorrelationId>,
    pub statuses: Vec<CommandStatus>,
    pub from_checkpoint: RuntimeEventCheckpoint,
    pub to_sequence: Option<u64>,
    pub sort: RuntimeCommandResultSort,
    pub limit: Option<usize>,
}

impl RuntimeCommandResultQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_command(mut self, command_id: CommandId) -> Self {
        self.command_id = Some(command_id);
        self
    }

    pub fn for_bridge(mut self, bridge_id: BridgeId) -> Self {
        self.bridge_id = Some(bridge_id);
        self
    }

    pub fn for_correlation(mut self, correlation_id: CorrelationId) -> Self {
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn with_status(mut self, status: CommandStatus) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn from_checkpoint(mut self, checkpoint: RuntimeEventCheckpoint) -> Self {
        self.from_checkpoint = checkpoint;
        self
    }

    pub fn to_sequence(mut self, sequence: u64) -> Self {
        self.to_sequence = Some(sequence);
        self
    }

    pub fn sorted_by(mut self, sort: RuntimeCommandResultSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for RuntimeCommandResultQuery {
    fn default() -> Self {
        Self {
            command_id: None,
            bridge_id: None,
            correlation_id: None,
            statuses: Vec::new(),
            from_checkpoint: RuntimeEventCheckpoint::start(),
            to_sequence: None,
            sort: RuntimeCommandResultSort::default(),
            limit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCommandResultRecord {
    pub sequence: u64,
    pub next_checkpoint: RuntimeEventCheckpoint,
    pub result: CommandResult,
}

impl RuntimeCommandResultRecord {
    pub fn from_entry(entry: RuntimeEventLogEntry<'_>) -> Option<Self> {
        match entry.event {
            RuntimeEvent::CommandResult(result) => Some(Self {
                sequence: entry.sequence,
                next_checkpoint: entry.next_checkpoint,
                result: result.clone(),
            }),
            _ => None,
        }
    }
}

/// Compact count view over selected command results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCommandResultSummary {
    pub total_results: usize,
    pub accepted_results: usize,
    pub rejected_results: usize,
    pub timed_out_results: usize,
    pub failed_results: usize,
    pub first_sequence: Option<u64>,
    pub latest_sequence: Option<u64>,
    pub next_checkpoint: RuntimeEventCheckpoint,
}

impl Default for RuntimeCommandResultSummary {
    fn default() -> Self {
        Self {
            total_results: 0,
            accepted_results: 0,
            rejected_results: 0,
            timed_out_results: 0,
            failed_results: 0,
            first_sequence: None,
            latest_sequence: None,
            next_checkpoint: RuntimeEventCheckpoint::start(),
        }
    }
}

impl RuntimeCommandResultSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_records<'a, I>(records: I) -> Self
    where
        I: IntoIterator<Item = &'a RuntimeCommandResultRecord>,
    {
        let mut summary = Self::empty();
        for record in records {
            summary.total_results += 1;
            summary.first_sequence = Some(
                summary
                    .first_sequence
                    .map(|sequence| sequence.min(record.sequence))
                    .unwrap_or(record.sequence),
            );
            summary.latest_sequence = Some(
                summary
                    .latest_sequence
                    .map(|sequence| sequence.max(record.sequence))
                    .unwrap_or(record.sequence),
            );
            summary.next_checkpoint = RuntimeEventCheckpoint::from_next_sequence(
                summary
                    .latest_sequence
                    .map(|sequence| sequence.saturating_add(1))
                    .unwrap_or(0),
            );
            match record.result.status {
                CommandStatus::Accepted => summary.accepted_results += 1,
                CommandStatus::Rejected => summary.rejected_results += 1,
                CommandStatus::TimedOut => summary.timed_out_results += 1,
                CommandStatus::Failed => summary.failed_results += 1,
            }
        }
        summary
    }

    pub fn has_results(&self) -> bool {
        self.total_results > 0
    }

    pub fn failure_results(&self) -> usize {
        self.rejected_results + self.timed_out_results + self.failed_results
    }

    pub fn has_failures(&self) -> bool {
        self.failure_results() > 0
    }
}

/// Read-side query for the runtime event log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEventQuery {
    pub filter: Option<RuntimeEventFilter>,
    pub from_checkpoint: RuntimeEventCheckpoint,
    pub to_sequence: Option<u64>,
    pub sort: RuntimeEventSort,
    pub limit: Option<usize>,
}

impl RuntimeEventQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn matching(mut self, filter: RuntimeEventFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn from_checkpoint(mut self, checkpoint: RuntimeEventCheckpoint) -> Self {
        self.from_checkpoint = checkpoint;
        self
    }

    pub fn to_sequence(mut self, sequence: u64) -> Self {
        self.to_sequence = Some(sequence);
        self
    }

    pub fn sorted_by(mut self, sort: RuntimeEventSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

impl Default for RuntimeEventQuery {
    fn default() -> Self {
        Self {
            filter: None,
            from_checkpoint: RuntimeEventCheckpoint::start(),
            to_sequence: None,
            sort: RuntimeEventSort::default(),
            limit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSubscriptionSnapshot {
    pub subscription_id: RuntimeSubscriptionId,
    pub filter: RuntimeEventFilter,
    pub queued_events: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSubscriptionBacklogStatus {
    CaughtUp,
    Backlogged,
}

impl RuntimeSubscriptionSnapshot {
    pub fn has_backlog(&self) -> bool {
        self.queued_events > 0
    }

    pub fn is_caught_up(&self) -> bool {
        !self.has_backlog()
    }

    pub fn backlog_status(&self) -> RuntimeSubscriptionBacklogStatus {
        if self.has_backlog() {
            RuntimeSubscriptionBacklogStatus::Backlogged
        } else {
            RuntimeSubscriptionBacklogStatus::CaughtUp
        }
    }

    pub fn exceeds_backlog_threshold(&self, threshold: usize) -> bool {
        self.queued_events > threshold
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeSubscriptionInventorySummary {
    pub total_subscriptions: usize,
    pub all_event_subscriptions: usize,
    pub bridge_subscriptions: usize,
    pub entity_subscriptions: usize,
    pub command_subscriptions: usize,
    pub supervision_subscriptions: usize,
    pub backlogged_subscriptions: usize,
    pub caught_up_subscriptions: usize,
    pub total_queued_events: usize,
    pub max_queued_events: usize,
}

impl RuntimeSubscriptionInventorySummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_snapshots<'a, I>(snapshots: I) -> Self
    where
        I: IntoIterator<Item = &'a RuntimeSubscriptionSnapshot>,
    {
        let mut summary = Self::empty();
        for snapshot in snapshots {
            summary.record_snapshot(snapshot);
        }
        summary
    }

    pub fn record_snapshot(&mut self, snapshot: &RuntimeSubscriptionSnapshot) {
        self.total_subscriptions += 1;
        self.total_queued_events += snapshot.queued_events;
        self.max_queued_events = self.max_queued_events.max(snapshot.queued_events);

        match snapshot.filter {
            RuntimeEventFilter::All => self.all_event_subscriptions += 1,
            RuntimeEventFilter::Bridge(_) => self.bridge_subscriptions += 1,
            RuntimeEventFilter::Entity(_) => self.entity_subscriptions += 1,
            RuntimeEventFilter::Commands => self.command_subscriptions += 1,
            RuntimeEventFilter::Supervision => self.supervision_subscriptions += 1,
        }
        if snapshot.has_backlog() {
            self.backlogged_subscriptions += 1;
        } else {
            self.caught_up_subscriptions += 1;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_subscriptions == 0
    }

    pub fn has_backlog(&self) -> bool {
        self.total_queued_events > 0 || self.backlogged_subscriptions > 0
    }

    pub fn has_command_subscribers(&self) -> bool {
        self.command_subscriptions > 0
    }

    pub fn has_supervision_subscribers(&self) -> bool {
        self.supervision_subscriptions > 0
    }

    // Explicit `if divisor == 0` guard is intentional (and clearer than checked_div here); allow the 1.97 manual_checked_ops lint.
    #[allow(clippy::manual_checked_ops)]
    pub fn average_queued_events_per_subscription(&self) -> usize {
        if self.total_subscriptions == 0 {
            0
        } else {
            self.total_queued_events / self.total_subscriptions
        }
    }

    pub fn backlogged_subscription_percent(&self) -> u8 {
        if self.total_subscriptions == 0 {
            return 0;
        }
        let backlogged = self.backlogged_subscriptions.min(self.total_subscriptions);
        ((backlogged.saturating_mul(100) / self.total_subscriptions).min(100)) as u8
    }

    pub fn exceeds_subscription_backlog_threshold(&self, threshold: usize) -> bool {
        self.max_queued_events > threshold
    }
}

/// Payload-free event-bus health view for replay and subscriber pressure checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeEventBusHealthSummary {
    pub snapshot: RuntimeEventBusSnapshot,
    pub subscriptions: RuntimeSubscriptionInventorySummary,
    pub event_log: RuntimeEventLogSummary,
}

impl RuntimeEventBusHealthSummary {
    pub fn has_stream_coverage(&self) -> bool {
        self.snapshot.has_subscriptions()
    }

    pub fn has_replay_history(&self) -> bool {
        self.event_log.has_events()
    }

    pub fn has_event_pressure(&self) -> bool {
        self.snapshot.has_backlog() || self.subscriptions.has_backlog()
    }

    pub fn is_caught_up(&self) -> bool {
        !self.has_event_pressure()
    }

    pub fn needs_attention(&self) -> bool {
        self.has_event_pressure()
    }

    pub fn has_command_streams(&self) -> bool {
        self.subscriptions.has_command_subscribers()
    }

    pub fn has_supervision_streams(&self) -> bool {
        self.subscriptions.has_supervision_subscribers()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RuntimeSubscriptionSort {
    #[default]
    SubscriptionId,
    QueuedEventsDesc,
}


/// Read-side query for active event-bus subscriptions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeSubscriptionQuery {
    pub subscription_id: Option<RuntimeSubscriptionId>,
    pub filter: Option<RuntimeEventFilter>,
    pub min_queued_events: Option<usize>,
    pub sort: RuntimeSubscriptionSort,
    pub limit: Option<usize>,
}

impl RuntimeSubscriptionQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_subscription(mut self, subscription_id: RuntimeSubscriptionId) -> Self {
        self.subscription_id = Some(subscription_id);
        self
    }

    pub fn matching(mut self, filter: RuntimeEventFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_min_queued_events(mut self, min_queued_events: usize) -> Self {
        self.min_queued_events = Some(min_queued_events);
        self
    }

    pub fn sorted_by(mut self, sort: RuntimeSubscriptionSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
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

impl WorkerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Unhealthy => "unhealthy",
            Self::Restarting => "restarting",
            Self::Stopped => "stopped",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerRestartReason {
    HeartbeatOverdue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerRestartInstruction {
    pub bridge_id: BridgeId,
    pub integration_id: IntegrationId,
    pub reason: WorkerRestartReason,
    pub status: WorkerStatus,
    pub last_heartbeat_at_ms: u64,
    pub heartbeat_timeout_ms: u64,
    pub due_at_ms: u64,
    pub planned_at_ms: u64,
    pub restart_attempt: u32,
}

impl WorkerRestartInstruction {
    pub fn overdue_by_ms(&self) -> u64 {
        self.planned_at_ms.saturating_sub(self.due_at_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerHeartbeatDeadline {
    pub bridge_id: BridgeId,
    pub integration_id: IntegrationId,
    pub status: WorkerStatus,
    pub last_heartbeat_at_ms: u64,
    pub heartbeat_timeout_ms: u64,
    pub due_at_ms: u64,
}

impl WorkerHeartbeatDeadline {
    pub fn from_worker(worker: &SupervisedBridgeWorker) -> Option<Self> {
        let due_at_ms = worker.heartbeat_due_at_ms()?;
        Some(Self {
            bridge_id: worker.bridge_id.clone(),
            integration_id: worker.integration_id.clone(),
            status: worker.status,
            last_heartbeat_at_ms: worker.last_heartbeat_at_ms,
            heartbeat_timeout_ms: worker.heartbeat_timeout_ms,
            due_at_ms,
        })
    }

    pub fn overdue_by_ms_at(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.due_at_ms)
    }

    pub fn is_due_at(&self, now_ms: u64) -> bool {
        now_ms >= self.due_at_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerHeartbeatSchedule {
    pub generated_at_ms: u64,
    pub deadlines: Vec<WorkerHeartbeatDeadline>,
}

impl WorkerHeartbeatSchedule {
    pub fn is_empty(&self) -> bool {
        self.deadlines.is_empty()
    }

    pub fn len(&self) -> usize {
        self.deadlines.len()
    }

    pub fn next_due_at_ms(&self) -> Option<u64> {
        self.deadlines
            .iter()
            .map(|deadline| deadline.due_at_ms)
            .min()
    }

    pub fn due_at(&self, now_ms: u64) -> Vec<&WorkerHeartbeatDeadline> {
        self.deadlines
            .iter()
            .filter(|deadline| deadline.is_due_at(now_ms))
            .collect()
    }

    pub fn deadlines_for_bridge(&self, bridge_id: &BridgeId) -> Vec<&WorkerHeartbeatDeadline> {
        self.deadlines
            .iter()
            .filter(|deadline| &deadline.bridge_id == bridge_id)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerRestartPlan {
    pub generated_at_ms: u64,
    pub instructions: Vec<WorkerRestartInstruction>,
}

impl WorkerRestartPlan {
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn instructions_for_bridge(&self, bridge_id: &BridgeId) -> Vec<&WorkerRestartInstruction> {
        self.instructions
            .iter()
            .filter(|instruction| &instruction.bridge_id == bridge_id)
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum SupervisedWorkerSort {
    #[default]
    BridgeId,
    HeartbeatDueAt,
    RestartCountDesc,
    StatusThenBridgeId,
}


/// Read-side query for supervised integration workers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SupervisedWorkerQuery {
    pub bridge_id: Option<BridgeId>,
    pub integration_id: Option<IntegrationId>,
    pub statuses: Vec<WorkerStatus>,
    pub heartbeat_due_before_ms: Option<u64>,
    pub overdue_at_ms: Option<u64>,
    pub min_restart_count: Option<u32>,
    pub sort: SupervisedWorkerSort,
    pub limit: Option<usize>,
}

impl SupervisedWorkerQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_bridge(mut self, bridge_id: BridgeId) -> Self {
        self.bridge_id = Some(bridge_id);
        self
    }

    pub fn for_integration(mut self, integration_id: IntegrationId) -> Self {
        self.integration_id = Some(integration_id);
        self
    }

    pub fn with_status(mut self, status: WorkerStatus) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn heartbeat_due_before(mut self, heartbeat_due_before_ms: u64) -> Self {
        self.heartbeat_due_before_ms = Some(heartbeat_due_before_ms);
        self
    }

    pub fn overdue_at(mut self, overdue_at_ms: u64) -> Self {
        self.overdue_at_ms = Some(overdue_at_ms);
        self
    }

    pub fn min_restart_count(mut self, min_restart_count: u32) -> Self {
        self.min_restart_count = Some(min_restart_count);
        self
    }

    pub fn sorted_by(mut self, sort: SupervisedWorkerSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandAuthorization {
    pub principal_id: AgentId,
    pub grants: Vec<CapabilityGrant>,
}

impl CommandAuthorization {
    pub fn new(principal_id: AgentId, grants: Vec<CapabilityGrant>) -> Self {
        Self {
            principal_id,
            grants,
        }
    }

    pub fn allows_command_at(&self, command: &DeviceCommand, now_ms: u64) -> bool {
        self.missing_capabilities_for(command, now_ms).is_empty()
    }

    pub fn missing_capabilities_for(
        &self,
        command: &DeviceCommand,
        now_ms: u64,
    ) -> Vec<CapabilityId> {
        command
            .required_capabilities
            .iter()
            .filter(|capability_id| {
                !self.grants.iter().any(|grant| {
                    grant_covers_command_capability(
                        grant,
                        &self.principal_id,
                        command,
                        capability_id,
                        now_ms,
                    )
                })
            })
            .cloned()
            .collect()
    }
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

    pub fn heartbeat_due_at_ms(&self) -> Option<u64> {
        if matches!(
            self.status,
            WorkerStatus::Starting | WorkerStatus::Running | WorkerStatus::Unhealthy
        ) {
            Some(
                self.last_heartbeat_at_ms
                    .saturating_add(self.heartbeat_timeout_ms),
            )
        } else {
            None
        }
    }

    pub fn is_overdue_at(&self, now_ms: u64) -> bool {
        self.heartbeat_due_at_ms()
            .is_some_and(|due_at_ms| now_ms >= due_at_ms)
    }

    pub fn restart_instruction_at(&self, now_ms: u64) -> Option<WorkerRestartInstruction> {
        if !self.is_overdue_at(now_ms) {
            return None;
        }
        let due_at_ms = self
            .heartbeat_due_at_ms()
            .expect("overdue workers always have a heartbeat deadline");
        Some(WorkerRestartInstruction {
            bridge_id: self.bridge_id.clone(),
            integration_id: self.integration_id.clone(),
            reason: WorkerRestartReason::HeartbeatOverdue,
            status: self.status,
            last_heartbeat_at_ms: self.last_heartbeat_at_ms,
            heartbeat_timeout_ms: self.heartbeat_timeout_ms,
            due_at_ms,
            planned_at_ms: now_ms,
            restart_attempt: self.restart_count.saturating_add(1),
        })
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

    pub fn snapshot_at(&self, now_ms: u64) -> RuntimeSupervisorSnapshot {
        RuntimeSupervisorSnapshot::from_workers_at(self.workers.values(), now_ms)
    }

    pub fn query_workers(&self, query: &SupervisedWorkerQuery) -> Vec<&SupervisedBridgeWorker> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut workers = self
            .workers
            .values()
            .filter(|worker| supervised_worker_matches_query(worker, query))
            .collect::<Vec<_>>();
        match query.sort {
            SupervisedWorkerSort::BridgeId => {
                workers.sort_by(|left, right| left.bridge_id.cmp(&right.bridge_id));
            }
            SupervisedWorkerSort::HeartbeatDueAt => workers.sort_by(|left, right| {
                left.heartbeat_due_at_ms()
                    .unwrap_or(u64::MAX)
                    .cmp(&right.heartbeat_due_at_ms().unwrap_or(u64::MAX))
                    .then_with(|| left.bridge_id.cmp(&right.bridge_id))
            }),
            SupervisedWorkerSort::RestartCountDesc => workers.sort_by(|left, right| {
                right
                    .restart_count
                    .cmp(&left.restart_count)
                    .then_with(|| left.bridge_id.cmp(&right.bridge_id))
            }),
            SupervisedWorkerSort::StatusThenBridgeId => workers.sort_by(|left, right| {
                left.status
                    .as_str()
                    .cmp(right.status.as_str())
                    .then_with(|| left.bridge_id.cmp(&right.bridge_id))
            }),
        }
        apply_limit(&mut workers, query.limit);
        workers
    }

    pub fn heartbeat_schedule_at(&self, now_ms: u64) -> WorkerHeartbeatSchedule {
        let mut deadlines: Vec<_> = self
            .workers
            .values()
            .filter_map(WorkerHeartbeatDeadline::from_worker)
            .collect();
        deadlines.sort_by(|left, right| {
            left.due_at_ms
                .cmp(&right.due_at_ms)
                .then_with(|| left.bridge_id.cmp(&right.bridge_id))
        });
        WorkerHeartbeatSchedule {
            generated_at_ms: now_ms,
            deadlines,
        }
    }

    pub fn restart_plan_at(&self, now_ms: u64) -> WorkerRestartPlan {
        let instructions = self
            .workers
            .values()
            .filter_map(|worker| worker.restart_instruction_at(now_ms))
            .collect();
        WorkerRestartPlan {
            generated_at_ms: now_ms,
            instructions,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuntimeSupervisorSnapshot {
    pub generated_at_ms: u64,
    pub worker_count: usize,
    pub starting_count: usize,
    pub running_count: usize,
    pub unhealthy_count: usize,
    pub restarting_count: usize,
    pub stopped_count: usize,
    pub restart_due_count: usize,
}

impl RuntimeSupervisorSnapshot {
    pub fn from_workers_at<'a, I>(workers: I, now_ms: u64) -> Self
    where
        I: IntoIterator<Item = &'a SupervisedBridgeWorker>,
    {
        let mut snapshot = Self {
            generated_at_ms: now_ms,
            ..Self::default()
        };
        for worker in workers {
            snapshot.worker_count += 1;
            if worker.is_overdue_at(now_ms) {
                snapshot.restart_due_count += 1;
            }
            match worker.status {
                WorkerStatus::Starting => snapshot.starting_count += 1,
                WorkerStatus::Running => snapshot.running_count += 1,
                WorkerStatus::Unhealthy => snapshot.unhealthy_count += 1,
                WorkerStatus::Restarting => snapshot.restarting_count += 1,
                WorkerStatus::Stopped => snapshot.stopped_count += 1,
            }
        }
        snapshot
    }

    pub fn has_restart_pressure(&self) -> bool {
        self.restart_due_count > 0 || self.unhealthy_count > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledDiscoveryWorker {
    pub worker_id: DiscoveryWorkerId,
    pub integration_id: IntegrationId,
    pub kind: DiscoveryWorkerKind,
    pub sources: Vec<DiscoverySource>,
    pub network_interfaces: Vec<String>,
    pub status: WorkerStatus,
    pub interval_ms: u64,
    pub run_timeout_ms: u64,
    pub retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
    pub retry_backoff_multiplier: u32,
    pub next_due_at_ms: u64,
    pub last_started_at_ms: Option<u64>,
    pub last_completed_at_ms: Option<u64>,
    pub last_run_status: Option<DiscoveryWorkerRunStatus>,
    pub last_record_count: usize,
    pub last_failure_count: usize,
    pub last_catalog_change_count: usize,
    pub total_run_count: u64,
    pub consecutive_failure_count: u32,
    pub metadata: Vec<Metadata>,
}

impl ScheduledDiscoveryWorker {
    pub fn new(
        worker_id: DiscoveryWorkerId,
        integration_id: IntegrationId,
        kind: DiscoveryWorkerKind,
        interval_ms: u64,
        run_timeout_ms: u64,
        first_due_at_ms: u64,
    ) -> Self {
        Self {
            worker_id,
            integration_id,
            kind,
            sources: Vec::new(),
            network_interfaces: Vec::new(),
            status: WorkerStatus::Starting,
            interval_ms,
            run_timeout_ms,
            retry_delay_ms: interval_ms,
            max_retry_delay_ms: interval_ms,
            retry_backoff_multiplier: 1,
            next_due_at_ms: first_due_at_ms,
            last_started_at_ms: None,
            last_completed_at_ms: None,
            last_run_status: None,
            last_record_count: 0,
            last_failure_count: 0,
            last_catalog_change_count: 0,
            total_run_count: 0,
            consecutive_failure_count: 0,
            metadata: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: DiscoverySource) -> Self {
        if !self.sources.contains(&source) {
            self.sources.push(source);
        }
        self
    }

    pub fn with_network_interface(mut self, network_interface: impl Into<String>) -> Self {
        let network_interface = network_interface.into();
        if !self
            .network_interfaces
            .iter()
            .any(|existing| existing == &network_interface)
        {
            self.network_interfaces.push(network_interface);
        }
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.push(Metadata::new(key, value));
        self
    }

    pub fn with_retry_backoff(
        mut self,
        retry_delay_ms: u64,
        max_retry_delay_ms: u64,
        retry_backoff_multiplier: u32,
    ) -> Self {
        self.retry_delay_ms = retry_delay_ms;
        self.max_retry_delay_ms = max_retry_delay_ms;
        self.retry_backoff_multiplier = retry_backoff_multiplier;
        self
    }

    pub fn is_due_at(&self, now_ms: u64) -> bool {
        now_ms >= self.next_due_at_ms
    }

    pub fn overdue_by_ms_at(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.next_due_at_ms)
    }

    pub fn due_instruction_at(&self, now_ms: u64) -> Option<DiscoveryWorkerRunInstruction> {
        if !self.is_due_at(now_ms) {
            return None;
        }
        Some(DiscoveryWorkerRunInstruction {
            worker_id: self.worker_id.clone(),
            integration_id: self.integration_id.clone(),
            kind: self.kind,
            sources: self.sources.clone(),
            network_interfaces: self.network_interfaces.clone(),
            status: self.status,
            due_at_ms: self.next_due_at_ms,
            planned_at_ms: now_ms,
            interval_ms: self.interval_ms,
            run_timeout_ms: self.run_timeout_ms,
            retry_delay_ms: self.retry_delay_ms,
            max_retry_delay_ms: self.max_retry_delay_ms,
            retry_backoff_multiplier: self.retry_backoff_multiplier,
            consecutive_failure_count: self.consecutive_failure_count,
            metadata: self.metadata.clone(),
        })
    }

    pub fn mark_started_at(&mut self, now_ms: u64) {
        self.status = WorkerStatus::Running;
        self.last_started_at_ms = Some(now_ms);
    }

    pub fn record_run_summary(&mut self, summary: &DiscoveryWorkerRunSummary) {
        self.total_run_count = self.total_run_count.saturating_add(1);
        self.last_started_at_ms = Some(summary.started_at_ms);
        self.last_completed_at_ms = Some(summary.completed_at_ms);
        self.last_run_status = Some(summary.status);
        self.last_record_count = summary.record_count;
        self.last_failure_count = summary.failure_count;
        self.last_catalog_change_count = summary.accepted_count();

        match summary.status {
            DiscoveryWorkerRunStatus::Completed => {
                self.status = WorkerStatus::Running;
                self.consecutive_failure_count = 0;
                self.next_due_at_ms = summary.completed_at_ms.saturating_add(self.interval_ms);
            }
            DiscoveryWorkerRunStatus::Partial | DiscoveryWorkerRunStatus::Failed => {
                self.status = WorkerStatus::Unhealthy;
                self.consecutive_failure_count = self.consecutive_failure_count.saturating_add(1);
                self.next_due_at_ms = summary.completed_at_ms.saturating_add(
                    self.retry_delay_for_failure_count(self.consecutive_failure_count),
                );
            }
        }
    }

    pub fn retry_delay_for_failure_count(&self, consecutive_failure_count: u32) -> u64 {
        if consecutive_failure_count == 0 {
            return 0;
        }

        let mut delay = self.retry_delay_ms;
        for _ in 1..consecutive_failure_count {
            delay = delay
                .saturating_mul(self.retry_backoff_multiplier as u64)
                .min(self.max_retry_delay_ms);
        }
        delay.min(self.max_retry_delay_ms)
    }

    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.interval_ms == 0 {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "interval_ms",
                "must be greater than zero",
            ));
        }
        if self.run_timeout_ms == 0 {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "run_timeout_ms",
                "must be greater than zero",
            ));
        }
        if self.retry_delay_ms == 0 {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "retry_delay_ms",
                "must be greater than zero",
            ));
        }
        if self.max_retry_delay_ms == 0 {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "max_retry_delay_ms",
                "must be greater than zero",
            ));
        }
        if self.max_retry_delay_ms < self.retry_delay_ms {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "max_retry_delay_ms",
                "must be greater than or equal to retry_delay_ms",
            ));
        }
        if self.retry_backoff_multiplier == 0 {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "retry_backoff_multiplier",
                "must be greater than zero",
            ));
        }
        if self.sources.is_empty() {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "sources",
                "must include at least one discovery source",
            ));
        }
        if self.sources.contains(&DiscoverySource::Mdns) && self.network_interfaces.is_empty() {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "network_interfaces",
                "mDNS schedules must name the selected interfaces",
            ));
        }
        if self.sources.contains(&DiscoverySource::Mdns)
            && metadata_value(&self.metadata, MDNS_DISCOVERY_SERVICE_TYPE_METADATA_KEY)
                .is_none_or(str::is_empty)
        {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "metadata.smart_home.discovery.service_type",
                "mDNS schedules must name the DNS-SD service type",
            ));
        }
        if self
            .network_interfaces
            .iter()
            .any(|network_interface| network_interface.trim().is_empty())
        {
            return Err(invalid_discovery_worker_schedule(
                &self.worker_id,
                "network_interfaces",
                "interface names must not be empty",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledDiscoveryWorkerSnapshot {
    pub worker_id: DiscoveryWorkerId,
    pub integration_id: IntegrationId,
    pub kind: DiscoveryWorkerKind,
    pub sources: Vec<DiscoverySource>,
    pub network_interfaces: Vec<String>,
    pub status: WorkerStatus,
    pub interval_ms: u64,
    pub run_timeout_ms: u64,
    pub retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
    pub retry_backoff_multiplier: u32,
    pub current_retry_delay_ms: Option<u64>,
    pub next_due_at_ms: u64,
    pub is_due: bool,
    pub overdue_by_ms: u64,
    pub last_started_at_ms: Option<u64>,
    pub last_completed_at_ms: Option<u64>,
    pub last_run_status: Option<DiscoveryWorkerRunStatus>,
    pub last_record_count: usize,
    pub last_failure_count: usize,
    pub last_catalog_change_count: usize,
    pub total_run_count: u64,
    pub consecutive_failure_count: u32,
    pub metadata: Vec<Metadata>,
}

impl ScheduledDiscoveryWorkerSnapshot {
    pub fn from_worker_at(worker: &ScheduledDiscoveryWorker, now_ms: u64) -> Self {
        Self {
            worker_id: worker.worker_id.clone(),
            integration_id: worker.integration_id.clone(),
            kind: worker.kind,
            sources: worker.sources.clone(),
            network_interfaces: worker.network_interfaces.clone(),
            status: worker.status,
            interval_ms: worker.interval_ms,
            run_timeout_ms: worker.run_timeout_ms,
            retry_delay_ms: worker.retry_delay_ms,
            max_retry_delay_ms: worker.max_retry_delay_ms,
            retry_backoff_multiplier: worker.retry_backoff_multiplier,
            current_retry_delay_ms: if worker.consecutive_failure_count > 0 {
                Some(worker.retry_delay_for_failure_count(worker.consecutive_failure_count))
            } else {
                None
            },
            next_due_at_ms: worker.next_due_at_ms,
            is_due: worker.is_due_at(now_ms),
            overdue_by_ms: worker.overdue_by_ms_at(now_ms),
            last_started_at_ms: worker.last_started_at_ms,
            last_completed_at_ms: worker.last_completed_at_ms,
            last_run_status: worker.last_run_status,
            last_record_count: worker.last_record_count,
            last_failure_count: worker.last_failure_count,
            last_catalog_change_count: worker.last_catalog_change_count,
            total_run_count: worker.total_run_count,
            consecutive_failure_count: worker.consecutive_failure_count,
            metadata: worker.metadata.clone(),
        }
    }

    pub fn has_failure_pressure(&self) -> bool {
        self.status == WorkerStatus::Unhealthy
            || self.last_failure_count > 0
            || self.consecutive_failure_count > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryWorkerRunInstruction {
    pub worker_id: DiscoveryWorkerId,
    pub integration_id: IntegrationId,
    pub kind: DiscoveryWorkerKind,
    pub sources: Vec<DiscoverySource>,
    pub network_interfaces: Vec<String>,
    pub status: WorkerStatus,
    pub due_at_ms: u64,
    pub planned_at_ms: u64,
    pub interval_ms: u64,
    pub run_timeout_ms: u64,
    pub retry_delay_ms: u64,
    pub max_retry_delay_ms: u64,
    pub retry_backoff_multiplier: u32,
    pub consecutive_failure_count: u32,
    pub metadata: Vec<Metadata>,
}

impl DiscoveryWorkerRunInstruction {
    pub fn overdue_by_ms(&self) -> u64 {
        self.planned_at_ms.saturating_sub(self.due_at_ms)
    }

    pub fn mdns_service_type(&self) -> Option<&str> {
        metadata_value(&self.metadata, MDNS_DISCOVERY_SERVICE_TYPE_METADATA_KEY)
    }

    pub fn mdns_scan_requests(&self) -> Result<Vec<MdnsWorkerScanRequest>, RuntimeError> {
        if self.kind != DiscoveryWorkerKind::MdnsScan
            || !self.sources.contains(&DiscoverySource::Mdns)
        {
            return Ok(Vec::new());
        }
        let service_type = self.mdns_service_type().ok_or_else(|| {
            invalid_discovery_worker_schedule(
                &self.worker_id,
                "metadata.smart_home.discovery.service_type",
                "mDNS schedules must name the DNS-SD service type",
            )
        })?;
        let timeout = Duration::from_millis(self.run_timeout_ms);
        let mut requests = Vec::new();
        for network_interface in &self.network_interfaces {
            for network in [MdnsScanNetwork::Ipv4, MdnsScanNetwork::Ipv6] {
                requests.push(
                    MdnsWorkerScanRequest::new(
                        self.worker_id.clone(),
                        self.integration_id.clone(),
                        network_interface.clone(),
                        network,
                        service_type,
                        self.planned_at_ms,
                        timeout,
                    )
                    .map_err(|error| {
                        invalid_discovery_worker_schedule(
                            &self.worker_id,
                            "network_interfaces",
                            error.to_string(),
                        )
                    })?
                    .with_metadata("smart_home.discovery.due_at_ms", self.due_at_ms.to_string())
                    .with_metadata(
                        "smart_home.discovery.planned_at_ms",
                        self.planned_at_ms.to_string(),
                    )
                    .with_metadata("smart_home.discovery.worker_status", self.status.as_str()),
                );
            }
        }
        Ok(requests)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryWorkerRunPlan {
    pub generated_at_ms: u64,
    pub instructions: Vec<DiscoveryWorkerRunInstruction>,
}

impl DiscoveryWorkerRunPlan {
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn instructions_for_worker(
        &self,
        worker_id: &DiscoveryWorkerId,
    ) -> Vec<&DiscoveryWorkerRunInstruction> {
        self.instructions
            .iter()
            .filter(|instruction| &instruction.worker_id == worker_id)
            .collect()
    }

    pub fn mdns_scan_plan(&self) -> Result<MdnsWorkerScanPlan, RuntimeError> {
        let mut plan = MdnsWorkerScanPlan::new(self.generated_at_ms);
        for instruction in &self.instructions {
            for request in instruction.mdns_scan_requests()? {
                plan.push_request(request);
            }
        }
        Ok(plan)
    }
}

pub trait MdnsDiscoveryRunAdapter {
    type Error: fmt::Display;

    fn worker_run_from_mdns_scan_report(
        &mut self,
        report: &MdnsWorkerScanReport,
    ) -> Result<DiscoveryWorkerRun, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverySupervisorRunFailure {
    pub worker_id: DiscoveryWorkerId,
    pub integration_id: IntegrationId,
    pub kind: DiscoveryWorkerKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoverySupervisorRunReport {
    pub started_at_ms: u64,
    pub completed_at_ms: u64,
    pub ttl_ms: u64,
    pub planned_instruction_count: usize,
    pub mdns_request_count: usize,
    pub mdns_report_count: usize,
    pub summaries: Vec<DiscoveryWorkerRunSummary>,
    pub failures: Vec<DiscoverySupervisorRunFailure>,
}

impl DiscoverySupervisorRunReport {
    pub fn new(
        started_at_ms: u64,
        completed_at_ms: u64,
        ttl_ms: u64,
        planned_instruction_count: usize,
        mdns_request_count: usize,
        mdns_report_count: usize,
    ) -> Self {
        Self {
            started_at_ms,
            completed_at_ms,
            ttl_ms,
            planned_instruction_count,
            mdns_request_count,
            mdns_report_count,
            summaries: Vec::new(),
            failures: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.planned_instruction_count == 0
            && self.mdns_request_count == 0
            && self.recorded_run_count() == 0
    }

    pub fn recorded_run_count(&self) -> usize {
        self.summaries.len()
    }

    pub fn completed_run_count(&self) -> usize {
        self.summaries
            .iter()
            .filter(|summary| summary.status == DiscoveryWorkerRunStatus::Completed)
            .count()
    }

    pub fn partial_run_count(&self) -> usize {
        self.summaries
            .iter()
            .filter(|summary| summary.status == DiscoveryWorkerRunStatus::Partial)
            .count()
    }

    pub fn failed_run_count(&self) -> usize {
        self.summaries
            .iter()
            .filter(|summary| summary.status == DiscoveryWorkerRunStatus::Failed)
            .count()
    }

    pub fn catalog_change_count(&self) -> usize {
        self.summaries
            .iter()
            .map(DiscoveryWorkerRunSummary::accepted_count)
            .sum()
    }

    pub fn has_failures(&self) -> bool {
        !self.failures.is_empty()
            || self
                .summaries
                .iter()
                .any(|summary| summary.status != DiscoveryWorkerRunStatus::Completed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum DiscoveryWorkerSort {
    #[default]
    WorkerId,
    NextDueAt,
    StatusThenWorkerId,
    ConsecutiveFailuresDesc,
}


/// Read-side query for scheduled discovery workers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoveryWorkerQuery {
    pub worker_id: Option<DiscoveryWorkerId>,
    pub integration_id: Option<IntegrationId>,
    pub kinds: Vec<DiscoveryWorkerKind>,
    pub sources: Vec<DiscoverySource>,
    pub statuses: Vec<WorkerStatus>,
    pub due_before_ms: Option<u64>,
    pub overdue_at_ms: Option<u64>,
    pub min_consecutive_failure_count: Option<u32>,
    pub sort: DiscoveryWorkerSort,
    pub limit: Option<usize>,
}

impl DiscoveryWorkerQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_worker(mut self, worker_id: DiscoveryWorkerId) -> Self {
        self.worker_id = Some(worker_id);
        self
    }

    pub fn for_integration(mut self, integration_id: IntegrationId) -> Self {
        self.integration_id = Some(integration_id);
        self
    }

    pub fn with_kind(mut self, kind: DiscoveryWorkerKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn with_source(mut self, source: DiscoverySource) -> Self {
        self.sources.push(source);
        self
    }

    pub fn with_status(mut self, status: WorkerStatus) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn due_before(mut self, due_before_ms: u64) -> Self {
        self.due_before_ms = Some(due_before_ms);
        self
    }

    pub fn overdue_at(mut self, overdue_at_ms: u64) -> Self {
        self.overdue_at_ms = Some(overdue_at_ms);
        self
    }

    pub fn min_consecutive_failure_count(mut self, count: u32) -> Self {
        self.min_consecutive_failure_count = Some(count);
        self
    }

    pub fn sorted_by(mut self, sort: DiscoveryWorkerSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiscoveryWorkerSchedulerSnapshot {
    pub generated_at_ms: u64,
    pub worker_count: usize,
    pub due_worker_count: usize,
    pub starting_count: usize,
    pub running_count: usize,
    pub unhealthy_count: usize,
    pub restarting_count: usize,
    pub stopped_count: usize,
    pub workers_with_failures: usize,
}

impl DiscoveryWorkerSchedulerSnapshot {
    pub fn has_due_work(&self) -> bool {
        self.due_worker_count > 0
    }

    pub fn has_worker_pressure(&self) -> bool {
        self.due_worker_count > 0 || self.unhealthy_count > 0 || self.workers_with_failures > 0
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeDiscoveryScheduler {
    workers: BTreeMap<DiscoveryWorkerId, ScheduledDiscoveryWorker>,
}

impl RuntimeDiscoveryScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_worker(
        &mut self,
        worker: ScheduledDiscoveryWorker,
    ) -> Result<Option<ScheduledDiscoveryWorker>, RuntimeError> {
        worker.validate()?;
        Ok(self.workers.insert(worker.worker_id.clone(), worker))
    }

    pub fn worker(&self, worker_id: &DiscoveryWorkerId) -> Option<&ScheduledDiscoveryWorker> {
        self.workers.get(worker_id)
    }

    pub fn mark_started(
        &mut self,
        worker_id: &DiscoveryWorkerId,
        now_ms: u64,
    ) -> Result<ScheduledDiscoveryWorker, RuntimeError> {
        let worker = self
            .workers
            .get_mut(worker_id)
            .ok_or_else(|| RuntimeError::UnknownDiscoveryWorker(worker_id.clone()))?;
        worker.mark_started_at(now_ms);
        Ok(worker.clone())
    }

    pub fn record_run_summary(
        &mut self,
        summary: &DiscoveryWorkerRunSummary,
    ) -> Result<ScheduledDiscoveryWorker, RuntimeError> {
        let worker = self
            .workers
            .get_mut(&summary.worker_id)
            .ok_or_else(|| RuntimeError::UnknownDiscoveryWorker(summary.worker_id.clone()))?;
        worker.record_run_summary(summary);
        Ok(worker.clone())
    }

    pub fn query_workers(&self, query: &DiscoveryWorkerQuery) -> Vec<&ScheduledDiscoveryWorker> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut workers = self
            .workers
            .values()
            .filter(|worker| scheduled_discovery_worker_matches_query(worker, query))
            .collect::<Vec<_>>();
        match query.sort {
            DiscoveryWorkerSort::WorkerId => {
                workers.sort_by(|left, right| left.worker_id.cmp(&right.worker_id));
            }
            DiscoveryWorkerSort::NextDueAt => workers.sort_by(|left, right| {
                left.next_due_at_ms
                    .cmp(&right.next_due_at_ms)
                    .then_with(|| left.worker_id.cmp(&right.worker_id))
            }),
            DiscoveryWorkerSort::StatusThenWorkerId => workers.sort_by(|left, right| {
                left.status
                    .as_str()
                    .cmp(right.status.as_str())
                    .then_with(|| left.worker_id.cmp(&right.worker_id))
            }),
            DiscoveryWorkerSort::ConsecutiveFailuresDesc => workers.sort_by(|left, right| {
                right
                    .consecutive_failure_count
                    .cmp(&left.consecutive_failure_count)
                    .then_with(|| left.worker_id.cmp(&right.worker_id))
            }),
        }
        apply_limit(&mut workers, query.limit);
        workers
    }

    pub fn worker_snapshots_at(&self, now_ms: u64) -> Vec<ScheduledDiscoveryWorkerSnapshot> {
        self.query_workers(&DiscoveryWorkerQuery::new().sorted_by(DiscoveryWorkerSort::NextDueAt))
            .into_iter()
            .map(|worker| ScheduledDiscoveryWorkerSnapshot::from_worker_at(worker, now_ms))
            .collect()
    }

    pub fn run_plan_at(&self, now_ms: u64) -> DiscoveryWorkerRunPlan {
        let mut instructions = self
            .workers
            .values()
            .filter_map(|worker| worker.due_instruction_at(now_ms))
            .collect::<Vec<_>>();
        instructions.sort_by(|left, right| {
            left.due_at_ms
                .cmp(&right.due_at_ms)
                .then_with(|| left.worker_id.cmp(&right.worker_id))
        });
        DiscoveryWorkerRunPlan {
            generated_at_ms: now_ms,
            instructions,
        }
    }

    pub fn snapshot_at(&self, now_ms: u64) -> DiscoveryWorkerSchedulerSnapshot {
        let mut snapshot = DiscoveryWorkerSchedulerSnapshot {
            generated_at_ms: now_ms,
            worker_count: self.workers.len(),
            ..DiscoveryWorkerSchedulerSnapshot::default()
        };
        for worker in self.workers.values() {
            if worker.is_due_at(now_ms) {
                snapshot.due_worker_count += 1;
            }
            if worker.consecutive_failure_count > 0 {
                snapshot.workers_with_failures += 1;
            }
            match worker.status {
                WorkerStatus::Starting => snapshot.starting_count += 1,
                WorkerStatus::Running => snapshot.running_count += 1,
                WorkerStatus::Unhealthy => snapshot.unhealthy_count += 1,
                WorkerStatus::Restarting => snapshot.restarting_count += 1,
                WorkerStatus::Stopped => snapshot.stopped_count += 1,
            }
        }
        snapshot
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReconciliationReason {
    MissingState,
    StaleState,
    Drifted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum DesiredStateSort {
    #[default]
    EntityId,
    RequestedByThenEntityId,
    CommandTimeoutDesc,
}


/// Read-side query for desired-state supervision targets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesiredStateQuery {
    pub entity_id: Option<EntityId>,
    pub requested_by: Option<String>,
    pub capability_id: Option<CapabilityId>,
    pub min_command_timeout_ms: Option<u64>,
    pub max_command_timeout_ms: Option<u64>,
    pub sort: DesiredStateSort,
    pub limit: Option<usize>,
}

impl DesiredStateQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_entity(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    pub fn requested_by(mut self, requested_by: impl Into<String>) -> Self {
        self.requested_by = Some(requested_by.into());
        self
    }

    pub fn with_capability(mut self, capability_id: CapabilityId) -> Self {
        self.capability_id = Some(capability_id);
        self
    }

    pub fn min_command_timeout(mut self, min_command_timeout_ms: u64) -> Self {
        self.min_command_timeout_ms = Some(min_command_timeout_ms);
        self
    }

    pub fn max_command_timeout(mut self, max_command_timeout_ms: u64) -> Self {
        self.max_command_timeout_ms = Some(max_command_timeout_ms);
        self
    }

    pub fn sorted_by(mut self, sort: DesiredStateSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DesiredStateInventorySummary {
    pub total_desired_states: usize,
    pub total_desired_capabilities: usize,
    pub requested_by_count: usize,
    pub min_command_timeout_ms: Option<u64>,
    pub max_command_timeout_ms: Option<u64>,
}

impl DesiredStateInventorySummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_states<'a, I>(states: I) -> Self
    where
        I: IntoIterator<Item = &'a DesiredEntityState>,
    {
        let mut summary = Self::empty();
        let mut requested_by = BTreeSet::new();
        for state in states {
            summary.total_desired_states += 1;
            summary.total_desired_capabilities += state.desired.len();
            requested_by.insert(state.requested_by.clone());
            summary.min_command_timeout_ms = Some(
                summary
                    .min_command_timeout_ms
                    .map_or(state.command_timeout_ms, |current| {
                        current.min(state.command_timeout_ms)
                    }),
            );
            summary.max_command_timeout_ms = Some(
                summary
                    .max_command_timeout_ms
                    .map_or(state.command_timeout_ms, |current| {
                        current.max(state.command_timeout_ms)
                    }),
            );
        }
        summary.requested_by_count = requested_by.len();
        summary
    }

    pub fn has_desired_states(&self) -> bool {
        self.total_desired_states > 0
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

#[derive(Debug, Clone, PartialEq)]
pub struct DesiredStateDriftPlan {
    pub bridge_id: BridgeId,
    pub entity_id: EntityId,
    pub capability_id: CapabilityId,
    pub desired_value: Value,
    pub reason: ReconciliationReason,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PairingSessionStatus {
    PendingUserPresence,
    Completed,
    Expired,
    Cancelled,
}

impl PairingSessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PendingUserPresence => "pending_user_presence",
            Self::Completed => "completed",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePairingSession {
    pub session_id: RuntimePairingSessionId,
    pub bridge_id: BridgeId,
    pub integration_id: IntegrationId,
    pub requested_by: AgentId,
    pub started_at_ms: u64,
    pub expires_at_ms: u64,
    pub status: PairingSessionStatus,
    pub vault_ref: Option<VaultRef>,
    pub metadata: Vec<Metadata>,
}

impl RuntimePairingSession {
    pub fn pending(
        session_id: RuntimePairingSessionId,
        bridge: &Bridge,
        requested_by: AgentId,
        started_at_ms: u64,
        expires_at_ms: u64,
        metadata: Vec<Metadata>,
    ) -> Self {
        Self {
            session_id,
            bridge_id: bridge.bridge_id.clone(),
            integration_id: bridge.integration_id.clone(),
            requested_by,
            started_at_ms,
            expires_at_ms,
            status: PairingSessionStatus::PendingUserPresence,
            vault_ref: None,
            metadata,
        }
    }

    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        self.status == PairingSessionStatus::PendingUserPresence && now_ms >= self.expires_at_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RuntimePairingSessionSort {
    #[default]
    SessionId,
    ExpiresAt,
    StartedAtDesc,
    StatusThenExpiresAt,
}


/// Read-side query for bridge pairing ceremonies.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimePairingSessionQuery {
    pub session_id: Option<RuntimePairingSessionId>,
    pub bridge_id: Option<BridgeId>,
    pub integration_id: Option<IntegrationId>,
    pub requested_by: Option<AgentId>,
    pub statuses: Vec<PairingSessionStatus>,
    pub expires_before_ms: Option<u64>,
    pub expiring_at_ms: Option<u64>,
    pub sort: RuntimePairingSessionSort,
    pub limit: Option<usize>,
}

impl RuntimePairingSessionQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_session(mut self, session_id: RuntimePairingSessionId) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn for_bridge(mut self, bridge_id: BridgeId) -> Self {
        self.bridge_id = Some(bridge_id);
        self
    }

    pub fn for_integration(mut self, integration_id: IntegrationId) -> Self {
        self.integration_id = Some(integration_id);
        self
    }

    pub fn requested_by(mut self, requested_by: AgentId) -> Self {
        self.requested_by = Some(requested_by);
        self
    }

    pub fn with_status(mut self, status: PairingSessionStatus) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn expires_before(mut self, expires_before_ms: u64) -> Self {
        self.expires_before_ms = Some(expires_before_ms);
        self
    }

    pub fn expiring_at(mut self, expiring_at_ms: u64) -> Self {
        self.expiring_at_ms = Some(expiring_at_ms);
        self
    }

    pub fn sorted_by(mut self, sort: RuntimePairingSessionSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimePairingSessionInventorySummary {
    pub total_sessions: usize,
    pub pending_user_presence_sessions: usize,
    pub completed_sessions: usize,
    pub expired_sessions: usize,
    pub cancelled_sessions: usize,
    pub expiring_sessions: usize,
    pub sessions_with_vault_ref: usize,
}

impl RuntimePairingSessionInventorySummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_sessions_at<'a, I>(sessions: I, now_ms: u64) -> Self
    where
        I: IntoIterator<Item = &'a RuntimePairingSession>,
    {
        let mut summary = Self::empty();
        for session in sessions {
            summary.record_session_at(session, now_ms);
        }
        summary
    }

    pub fn record_session_at(&mut self, session: &RuntimePairingSession, now_ms: u64) {
        self.total_sessions += 1;
        match session.status {
            PairingSessionStatus::PendingUserPresence => {
                self.pending_user_presence_sessions += 1;
            }
            PairingSessionStatus::Completed => self.completed_sessions += 1,
            PairingSessionStatus::Expired => self.expired_sessions += 1,
            PairingSessionStatus::Cancelled => self.cancelled_sessions += 1,
        }
        if session.is_expired_at(now_ms) {
            self.expiring_sessions += 1;
        }
        if session.vault_ref.is_some() {
            self.sessions_with_vault_ref += 1;
        }
    }

    pub fn has_pending_user_presence(&self) -> bool {
        self.pending_user_presence_sessions > 0
    }

    pub fn has_expiring_sessions(&self) -> bool {
        self.expiring_sessions > 0
    }

    pub fn has_completed_credentials(&self) -> bool {
        self.sessions_with_vault_ref > 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeSupervisionPlan {
    pub generated_at_ms: u64,
    pub pairing_sessions_expiring: Vec<RuntimePairingSessionId>,
    pub state_refresh_plan: StateRefreshPlan,
    pub desired_state_drifts: Vec<DesiredStateDriftPlan>,
    pub worker_restart_plan: WorkerRestartPlan,
    pub discovery_worker_run_plan: DiscoveryWorkerRunPlan,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RuntimeSupervisionPlanSummary {
    pub generated_at_ms: u64,
    pub total_actions: usize,
    pub pairing_expiry_count: usize,
    pub state_refresh_count: usize,
    pub missing_state_refresh_count: usize,
    pub stale_state_refresh_count: usize,
    pub desired_state_drift_count: usize,
    pub desired_missing_state_count: usize,
    pub desired_stale_state_count: usize,
    pub desired_drifted_state_count: usize,
    pub worker_restart_count: usize,
    pub discovery_worker_run_count: usize,
}

impl RuntimeSupervisionPlanSummary {
    pub fn empty_at(generated_at_ms: u64) -> Self {
        Self {
            generated_at_ms,
            ..Self::default()
        }
    }

    pub fn is_idle(&self) -> bool {
        self.total_actions == 0
    }

    pub fn has_state_refresh_work(&self) -> bool {
        self.state_refresh_count > 0
    }

    pub fn has_reconciliation_work(&self) -> bool {
        self.desired_state_drift_count > 0
    }

    pub fn has_worker_restart_work(&self) -> bool {
        self.worker_restart_count > 0
    }

    pub fn has_discovery_worker_work(&self) -> bool {
        self.discovery_worker_run_count > 0
    }
}

impl RuntimeSupervisionPlan {
    pub fn is_empty(&self) -> bool {
        self.pairing_sessions_expiring.is_empty()
            && self.state_refresh_plan.is_empty()
            && self.desired_state_drifts.is_empty()
            && self.worker_restart_plan.is_empty()
            && self.discovery_worker_run_plan.is_empty()
    }

    pub fn action_count(&self) -> usize {
        self.pairing_sessions_expiring.len()
            + self.state_refresh_plan.len()
            + self.desired_state_drifts.len()
            + self.worker_restart_plan.len()
            + self.discovery_worker_run_plan.len()
    }

    pub fn summary(&self) -> RuntimeSupervisionPlanSummary {
        let mut summary = RuntimeSupervisionPlanSummary {
            generated_at_ms: self.generated_at_ms,
            total_actions: self.action_count(),
            pairing_expiry_count: self.pairing_sessions_expiring.len(),
            state_refresh_count: self.state_refresh_plan.len(),
            desired_state_drift_count: self.desired_state_drifts.len(),
            worker_restart_count: self.worker_restart_plan.len(),
            discovery_worker_run_count: self.discovery_worker_run_plan.len(),
            ..RuntimeSupervisionPlanSummary::default()
        };

        for target in &self.state_refresh_plan.targets {
            match target.reason {
                StateRefreshReason::Missing => summary.missing_state_refresh_count += 1,
                StateRefreshReason::Stale => summary.stale_state_refresh_count += 1,
            }
        }

        for drift in &self.desired_state_drifts {
            match drift.reason {
                ReconciliationReason::MissingState => summary.desired_missing_state_count += 1,
                ReconciliationReason::StaleState => summary.desired_stale_state_count += 1,
                ReconciliationReason::Drifted => summary.desired_drifted_state_count += 1,
            }
        }

        summary
    }

    pub fn drifts_for_entity(&self, entity_id: &EntityId) -> Vec<&DesiredStateDriftPlan> {
        self.desired_state_drifts
            .iter()
            .filter(|drift| &drift.entity_id == entity_id)
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeSupervisionObservation {
    pub generated_at_ms: u64,
    pub plan: RuntimeSupervisionPlan,
    pub heartbeat_schedule: WorkerHeartbeatSchedule,
    pub discovery_scheduler: DiscoveryWorkerSchedulerSnapshot,
    pub discovery_workers: Vec<ScheduledDiscoveryWorkerSnapshot>,
}

impl RuntimeSupervisionObservation {
    pub fn is_idle(&self) -> bool {
        self.plan.is_empty()
    }

    pub fn action_count(&self) -> usize {
        self.plan.action_count()
    }

    pub fn pairing_expiry_count(&self) -> usize {
        self.plan.pairing_sessions_expiring.len()
    }

    pub fn state_refresh_count(&self) -> usize {
        self.plan.state_refresh_plan.len()
    }

    pub fn desired_state_drift_count(&self) -> usize {
        self.plan.desired_state_drifts.len()
    }

    pub fn worker_restart_count(&self) -> usize {
        self.plan.worker_restart_plan.len()
    }

    pub fn discovery_worker_run_count(&self) -> usize {
        self.plan.discovery_worker_run_plan.len()
    }

    pub fn discovery_worker_count(&self) -> usize {
        self.discovery_scheduler.worker_count
    }

    pub fn unhealthy_discovery_worker_count(&self) -> usize {
        self.discovery_scheduler.unhealthy_count
    }

    pub fn discovery_workers_with_failures_count(&self) -> usize {
        self.discovery_scheduler.workers_with_failures
    }

    pub fn next_discovery_worker_due_at_ms(&self) -> Option<u64> {
        self.discovery_workers
            .iter()
            .map(|worker| worker.next_due_at_ms)
            .min()
    }

    pub fn due_worker_deadline_count(&self) -> usize {
        self.heartbeat_schedule.due_at(self.generated_at_ms).len()
    }

    pub fn next_worker_heartbeat_due_at_ms(&self) -> Option<u64> {
        self.heartbeat_schedule.next_due_at_ms()
    }

    pub fn plan_summary(&self) -> RuntimeSupervisionPlanSummary {
        self.plan.summary()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SupervisionTickReport {
    pub ticked_at_ms: u64,
    pub expired_pairing_sessions: Vec<RuntimePairingSessionId>,
    pub expired_entities: Vec<EntityId>,
    pub desired_state_actions: Vec<DesiredStateAction>,
    pub worker_events: Vec<RuntimeEvent>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SupervisionTickSummary {
    pub ticked_at_ms: u64,
    pub total_actions: usize,
    pub expired_pairing_session_count: usize,
    pub expired_entity_count: usize,
    pub desired_state_action_count: usize,
    pub desired_missing_state_count: usize,
    pub desired_stale_state_count: usize,
    pub desired_drifted_state_count: usize,
    pub worker_restart_event_count: usize,
}

impl SupervisionTickSummary {
    pub fn is_idle(&self) -> bool {
        self.total_actions == 0
    }

    pub fn has_pairing_expiry_work(&self) -> bool {
        self.expired_pairing_session_count > 0
    }

    pub fn has_state_expiry_work(&self) -> bool {
        self.expired_entity_count > 0
    }

    pub fn has_reconciliation_work(&self) -> bool {
        self.desired_state_action_count > 0
    }

    pub fn has_worker_restart_work(&self) -> bool {
        self.worker_restart_event_count > 0
    }
}

impl SupervisionTickReport {
    pub fn is_idle(&self) -> bool {
        self.expired_pairing_sessions.is_empty()
            && self.expired_entities.is_empty()
            && self.desired_state_actions.is_empty()
            && self.worker_events.is_empty()
    }

    pub fn action_count(&self) -> usize {
        self.expired_pairing_sessions.len()
            + self.expired_entities.len()
            + self.desired_state_actions.len()
            + self.worker_events.len()
    }

    pub fn summary(&self) -> SupervisionTickSummary {
        let mut summary = SupervisionTickSummary {
            ticked_at_ms: self.ticked_at_ms,
            total_actions: self.action_count(),
            expired_pairing_session_count: self.expired_pairing_sessions.len(),
            expired_entity_count: self.expired_entities.len(),
            desired_state_action_count: self.desired_state_actions.len(),
            worker_restart_event_count: self
                .worker_events
                .iter()
                .filter(|event| matches!(event, RuntimeEvent::WorkerNeedsRestart { .. }))
                .count(),
            ..SupervisionTickSummary::default()
        };

        for action in &self.desired_state_actions {
            match action {
                DesiredStateAction::CommandIssued { reason, .. } => match reason {
                    ReconciliationReason::MissingState => {
                        summary.desired_missing_state_count += 1;
                    }
                    ReconciliationReason::StaleState => {
                        summary.desired_stale_state_count += 1;
                    }
                    ReconciliationReason::Drifted => {
                        summary.desired_drifted_state_count += 1;
                    }
                },
            }
        }

        summary
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeReadSnapshot {
    pub generated_at_ms: u64,
    pub registry_counts: RegistryCounts,
    pub discovery_record_count: usize,
    pub discovery_scheduler: DiscoveryWorkerSchedulerSnapshot,
    pub event_bus: RuntimeEventBusSnapshot,
    pub supervisor: RuntimeSupervisorSnapshot,
    pub pairing_session_count: usize,
    pub expiring_pairing_session_count: usize,
    pub optimistic_state_count: usize,
    pub stale_optimistic_state_count: usize,
    pub desired_state_count: usize,
    pub desired_capability_count: usize,
    pub state_refresh_target_count: usize,
}

impl RuntimeReadSnapshot {
    pub fn pending_work_summary(&self) -> RuntimePendingWorkSummary {
        RuntimePendingWorkSummary {
            event_backlog_count: self.event_bus.pending_delivery_count,
            backlogged_subscription_count: self.event_bus.backlogged_subscription_count,
            discovery_worker_due_count: self.discovery_scheduler.due_worker_count,
            unhealthy_discovery_worker_count: self.discovery_scheduler.unhealthy_count,
            restart_due_count: self.supervisor.restart_due_count,
            unhealthy_worker_count: self.supervisor.unhealthy_count,
            expiring_pairing_session_count: self.expiring_pairing_session_count,
            stale_optimistic_state_count: self.stale_optimistic_state_count,
            state_refresh_target_count: self.state_refresh_target_count,
        }
    }

    pub fn has_pending_work(&self) -> bool {
        !self.pending_work_summary().is_idle()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimePendingWorkSummary {
    pub event_backlog_count: usize,
    pub backlogged_subscription_count: usize,
    pub discovery_worker_due_count: usize,
    pub unhealthy_discovery_worker_count: usize,
    pub restart_due_count: usize,
    pub unhealthy_worker_count: usize,
    pub expiring_pairing_session_count: usize,
    pub stale_optimistic_state_count: usize,
    pub state_refresh_target_count: usize,
}

impl RuntimePendingWorkSummary {
    pub fn is_idle(&self) -> bool {
        !self.has_event_backlog() && !self.has_supervision_pressure()
    }

    pub fn total_pending_work_count(&self) -> usize {
        self.event_backlog_count
            + self.restart_due_count
            + self.unhealthy_worker_count
            + self.discovery_worker_due_count
            + self.unhealthy_discovery_worker_count
            + self.expiring_pairing_session_count
            + self.stale_optimistic_state_count
            + self.state_refresh_target_count
    }

    pub fn has_event_backlog(&self) -> bool {
        self.event_backlog_count > 0 || self.backlogged_subscription_count > 0
    }

    pub fn has_supervision_pressure(&self) -> bool {
        self.restart_due_count > 0
            || self.unhealthy_worker_count > 0
            || self.discovery_worker_due_count > 0
            || self.unhealthy_discovery_worker_count > 0
            || self.expiring_pairing_session_count > 0
            || self.stale_optimistic_state_count > 0
            || self.state_refresh_target_count > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDiscoverToolRequest {
    pub integration_id: Option<IntegrationId>,
    pub source: Option<DiscoverySource>,
    pub fresh_only: bool,
    pub ttl_ms: u64,
    pub limit: Option<usize>,
}

impl RuntimeDiscoverToolRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_integration(mut self, integration_id: IntegrationId) -> Self {
        self.integration_id = Some(integration_id);
        self
    }

    pub fn from_source(mut self, source: DiscoverySource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn fresh_only(mut self, fresh_only: bool) -> Self {
        self.fresh_only = fresh_only;
        self
    }

    pub fn with_ttl_ms(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = ttl_ms;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    fn matches_record(&self, record: &DiscoveryRecord, now_ms: u64) -> bool {
        self.integration_id
            .as_ref()
            .is_none_or(|integration_id| &record.integration_id == integration_id)
            && self.source.is_none_or(|source| record.source == source)
            && (!self.fresh_only || !record.is_stale_at(now_ms, self.ttl_ms))
    }
}

impl Default for RuntimeDiscoverToolRequest {
    fn default() -> Self {
        Self {
            integration_id: None,
            source: None,
            fresh_only: false,
            ttl_ms: 60_000,
            limit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePairingPlanToolRequest {
    pub options: DiscoveryPairingPlanOptions,
    pub ttl_ms: u64,
}

impl RuntimePairingPlanToolRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_options(mut self, options: DiscoveryPairingPlanOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_ttl_ms(mut self, ttl_ms: u64) -> Self {
        self.ttl_ms = ttl_ms;
        self
    }
}

impl Default for RuntimePairingPlanToolRequest {
    fn default() -> Self {
        Self {
            options: DiscoveryPairingPlanOptions::new(),
            ttl_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDiscoverToolOutput {
    pub generated_at_ms: u64,
    pub ttl_ms: u64,
    pub records: Vec<DiscoveryRecord>,
    pub bridge_candidates: Vec<Bridge>,
    pub record_summary: DiscoveryRecordSummary,
    pub signal_summary: DiscoverySignalSummary,
}

impl RuntimeDiscoverToolOutput {
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RuntimeAuthorizationDecisionSort {
    DecidedAtAsc,
    #[default]
    DecidedAtDesc,
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeAuthorizationDecisionQuery {
    pub principal_id: Option<AgentId>,
    pub outcome: Option<AuthorizationOutcome>,
    pub sort: RuntimeAuthorizationDecisionSort,
    pub limit: Option<usize>,
}

impl RuntimeAuthorizationDecisionQuery {
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

    pub fn sorted_by(mut self, sort: RuntimeAuthorizationDecisionSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    fn selector(&self) -> AuthorizationDecisionSelector {
        let mut selector = AuthorizationDecisionSelector::new();
        if let Some(principal_id) = self.principal_id.clone() {
            selector = selector.for_principal(principal_id);
        }
        if let Some(outcome) = self.outcome {
            selector = selector.with_outcome(outcome);
        }
        selector
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCapabilityGrantScopeKind {
    Tool,
    Capability,
    EntityCapability,
    AllSmartHome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RuntimeCapabilityGrantSort {
    #[default]
    GrantId,
    PrincipalId,
    GrantedAtAsc,
    GrantedAtDesc,
    ExpiresAtAsc,
    ExpiresAtDesc,
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeCapabilityGrantQuery {
    pub principal_id: Option<AgentId>,
    pub status: Option<CapabilityGrantStatus>,
    pub scope_kind: Option<RuntimeCapabilityGrantScopeKind>,
    pub capability_id: Option<CapabilityId>,
    pub entity_id: Option<EntityId>,
    pub sort: RuntimeCapabilityGrantSort,
    pub limit: Option<usize>,
}

impl RuntimeCapabilityGrantQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_principal(mut self, principal_id: AgentId) -> Self {
        self.principal_id = Some(principal_id);
        self
    }

    pub fn with_status(mut self, status: CapabilityGrantStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_scope_kind(mut self, scope_kind: RuntimeCapabilityGrantScopeKind) -> Self {
        self.scope_kind = Some(scope_kind);
        self
    }

    pub fn with_capability(mut self, capability_id: CapabilityId) -> Self {
        self.capability_id = Some(capability_id);
        self
    }

    pub fn for_entity(mut self, entity_id: EntityId) -> Self {
        self.entity_id = Some(entity_id);
        self
    }

    pub fn sorted_by(mut self, sort: RuntimeCapabilityGrantSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RuntimeRoomSort {
    #[default]
    RoomId,
    AttentionDesc,
    EntityCountDesc,
    SceneCountDesc,
}


#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RuntimeRoomQuery {
    pub room_id: Option<String>,
    pub attention_only: bool,
    pub state_gaps_only: bool,
    pub sort: RuntimeRoomSort,
    pub limit: Option<usize>,
}

impl RuntimeRoomQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_room(mut self, room_id: impl Into<String>) -> Self {
        self.room_id = Some(room_id.into());
        self
    }

    pub fn attention_only(mut self, attention_only: bool) -> Self {
        self.attention_only = attention_only;
        self
    }

    pub fn state_gaps_only(mut self, state_gaps_only: bool) -> Self {
        self.state_gaps_only = state_gaps_only;
        self
    }

    pub fn sorted_by(mut self, sort: RuntimeRoomSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeRoomSummary {
    pub room_id: String,
    pub device_count: usize,
    pub online_devices: usize,
    pub pairing_candidate_devices: usize,
    pub attention_devices: usize,
    pub entity_count: usize,
    pub commandable_entities: usize,
    pub entities_with_state: usize,
    pub entities_without_state: usize,
    pub stale_entities: usize,
    pub scene_count: usize,
    pub scene_action_count: usize,
}

impl RuntimeRoomSummary {
    pub fn new(room_id: impl Into<String>) -> Self {
        Self {
            room_id: room_id.into(),
            device_count: 0,
            online_devices: 0,
            pairing_candidate_devices: 0,
            attention_devices: 0,
            entity_count: 0,
            commandable_entities: 0,
            entities_with_state: 0,
            entities_without_state: 0,
            stale_entities: 0,
            scene_count: 0,
            scene_action_count: 0,
        }
    }

    fn record_device(&mut self, device: &Device) {
        self.device_count += 1;
        if device.health.is_online() {
            self.online_devices += 1;
        }
        if device.health.is_pairing_candidate() {
            self.pairing_candidate_devices += 1;
        }
        if device.health.needs_attention() {
            self.attention_devices += 1;
        }
    }

    fn record_entity(&mut self, entity: &Entity, state: Option<&StateSnapshot>, now_ms: u64) {
        self.entity_count += 1;
        if entity.capability_summary().has_command_surface() {
            self.commandable_entities += 1;
        }
        match state {
            Some(snapshot) => {
                self.entities_with_state += 1;
                if snapshot.is_stale_at(now_ms) {
                    self.stale_entities += 1;
                }
            }
            None => self.entities_without_state += 1,
        }
    }

    fn record_scene_actions(&mut self, action_count: usize) {
        if action_count == 0 {
            return;
        }
        self.scene_count += 1;
        self.scene_action_count += action_count;
    }

    pub fn has_attention_items(&self) -> bool {
        self.attention_devices > 0
    }

    pub fn has_state_gaps(&self) -> bool {
        self.entities_without_state > 0 || self.stale_entities > 0
    }

    pub fn state_gap_count(&self) -> usize {
        self.entities_without_state + self.stale_entities
    }

    pub fn has_scene_actions(&self) -> bool {
        self.scene_action_count > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeReadToolRequest {
    GetRuntimeSnapshot,
    ListDiscoveryWorkers {
        query: DiscoveryWorkerQuery,
    },
    GetDiscoverySummary {
        request: RuntimeDiscoverToolRequest,
    },
    GetPairingPlan {
        request: RuntimePairingPlanToolRequest,
    },
    ListBridges,
    ListDevices {
        bridge_id: Option<BridgeId>,
        health: Option<Health>,
        capability_id: Option<CapabilityId>,
    },
    ListRooms {
        query: RuntimeRoomQuery,
    },
    ListScenes {
        scope: Option<SceneScope>,
        entity_id: Option<EntityId>,
        capability_id: Option<CapabilityId>,
    },
    DescribeScene {
        scene_id: SceneId,
    },
    GetState {
        entity_id: EntityId,
    },
    DescribeCapabilities {
        entity_id: EntityId,
    },
    GetHealth {
        bridge_id: Option<BridgeId>,
    },
    ListSubscriptions {
        query: RuntimeSubscriptionQuery,
    },
    InspectEventLog {
        query: RuntimeEventQuery,
    },
    ListCommandResults {
        query: RuntimeCommandResultQuery,
    },
    GetCommandResultSummary {
        query: RuntimeCommandResultQuery,
    },
    ListAuthorizationDecisions {
        query: RuntimeAuthorizationDecisionQuery,
    },
    GetAuthorizationSummary {
        query: RuntimeAuthorizationDecisionQuery,
    },
    ListCapabilityGrants {
        query: RuntimeCapabilityGrantQuery,
    },
    GetCapabilityGrantSummary {
        query: RuntimeCapabilityGrantQuery,
    },
    GetTopologySummary,
    ListDesiredStates {
        query: DesiredStateQuery,
    },
    ListPairingSessions {
        query: RuntimePairingSessionQuery,
    },
    ListWorkers {
        query: SupervisedWorkerQuery,
    },
    GetWorkerHeartbeatSchedule {
        bridge_id: Option<BridgeId>,
        due_at_or_before_ms: Option<u64>,
        limit: Option<usize>,
    },
    GetSupervisionPlan,
    ObserveSupervision,
}

impl RuntimeReadToolRequest {
    pub fn tool(&self) -> SmartHomeTool {
        match self {
            Self::GetRuntimeSnapshot => SmartHomeTool::GetRuntimeSnapshot,
            Self::ListDiscoveryWorkers { .. } => SmartHomeTool::ListDiscoveryWorkers,
            Self::GetDiscoverySummary { .. } => SmartHomeTool::GetDiscoverySummary,
            Self::GetPairingPlan { .. } => SmartHomeTool::GetPairingPlan,
            Self::ListBridges => SmartHomeTool::ListBridges,
            Self::ListDevices { .. } => SmartHomeTool::ListDevices,
            Self::ListRooms { .. } => SmartHomeTool::ListRooms,
            Self::ListScenes { .. } => SmartHomeTool::ListScenes,
            Self::DescribeScene { .. } => SmartHomeTool::DescribeScene,
            Self::GetState { .. } => SmartHomeTool::GetState,
            Self::DescribeCapabilities { .. } => SmartHomeTool::DescribeCapabilities,
            Self::GetHealth { .. } => SmartHomeTool::GetHealth,
            Self::ListSubscriptions { .. } => SmartHomeTool::ListSubscriptions,
            Self::InspectEventLog { .. } => SmartHomeTool::InspectEventLog,
            Self::ListCommandResults { .. } => SmartHomeTool::ListCommandResults,
            Self::GetCommandResultSummary { .. } => SmartHomeTool::GetCommandResultSummary,
            Self::ListAuthorizationDecisions { .. } => SmartHomeTool::ListAuthorizationDecisions,
            Self::GetAuthorizationSummary { .. } => SmartHomeTool::GetAuthorizationSummary,
            Self::ListCapabilityGrants { .. } => SmartHomeTool::ListCapabilityGrants,
            Self::GetCapabilityGrantSummary { .. } => SmartHomeTool::GetCapabilityGrantSummary,
            Self::GetTopologySummary => SmartHomeTool::GetTopologySummary,
            Self::ListDesiredStates { .. } => SmartHomeTool::ListDesiredStates,
            Self::ListPairingSessions { .. } => SmartHomeTool::ListPairingSessions,
            Self::ListWorkers { .. } => SmartHomeTool::ListWorkers,
            Self::GetWorkerHeartbeatSchedule { .. } => SmartHomeTool::GetWorkerHeartbeatSchedule,
            Self::GetSupervisionPlan => SmartHomeTool::GetSupervisionPlan,
            Self::ObserveSupervision => SmartHomeTool::ObserveSupervision,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSubscribeToolRequest {
    pub subscription_id: RuntimeSubscriptionId,
    pub filter: RuntimeEventFilter,
    pub from_checkpoint: Option<RuntimeEventCheckpoint>,
}

impl RuntimeSubscribeToolRequest {
    pub fn new(subscription_id: RuntimeSubscriptionId, filter: RuntimeEventFilter) -> Self {
        Self {
            subscription_id,
            filter,
            from_checkpoint: None,
        }
    }

    pub fn with_checkpoint(mut self, checkpoint: RuntimeEventCheckpoint) -> Self {
        self.from_checkpoint = Some(checkpoint);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePollEventsToolRequest {
    pub subscription_id: RuntimeSubscriptionId,
    pub limit: Option<usize>,
    pub peek: bool,
}

impl RuntimePollEventsToolRequest {
    pub fn new(subscription_id: RuntimeSubscriptionId) -> Self {
        Self {
            subscription_id,
            limit: None,
            peek: false,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn peek(mut self, peek: bool) -> Self {
        self.peek = peek;
        self
    }

    fn delivery_options(&self) -> RuntimeEventDeliveryOptions {
        let mut options = RuntimeEventDeliveryOptions::new();
        if let Some(limit) = self.limit {
            options = options.with_limit(limit);
        }
        options
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeUnsubscribeToolRequest {
    pub subscription_id: RuntimeSubscriptionId,
}

impl RuntimeUnsubscribeToolRequest {
    pub fn new(subscription_id: RuntimeSubscriptionId) -> Self {
        Self { subscription_id }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePairBridgeToolRequest {
    pub session_id: RuntimePairingSessionId,
    pub bridge_id: BridgeId,
    pub expires_at_ms: u64,
    pub metadata: Vec<Metadata>,
}

impl RuntimePairBridgeToolRequest {
    pub fn new(
        session_id: RuntimePairingSessionId,
        bridge_id: BridgeId,
        expires_at_ms: u64,
    ) -> Self {
        Self {
            session_id,
            bridge_id,
            expires_at_ms,
            metadata: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: Vec<Metadata>) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCompletePairingToolRequest {
    pub completion: RuntimePairingCompletion,
}

impl RuntimeCompletePairingToolRequest {
    pub fn new(
        session_id: RuntimePairingSessionId,
        vault_ref: VaultRef,
        completed_at_ms: u64,
    ) -> Self {
        Self {
            completion: RuntimePairingCompletion::new(session_id, vault_ref, completed_at_ms),
        }
    }

    pub fn with_metadata(mut self, metadata: Vec<Metadata>) -> Self {
        self.completion = self.completion.with_metadata(metadata);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeReportEventToolRequest {
    Device(DeviceEvent),
    BridgeHealth(BridgeHealthReport),
}

impl RuntimeReportEventToolRequest {
    pub fn device(event: DeviceEvent) -> Self {
        Self::Device(event)
    }

    pub fn bridge_health(report: BridgeHealthReport) -> Self {
        Self::BridgeHealth(report)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeSetDesiredStateToolRequest {
    pub desired_state: DesiredEntityState,
}

impl RuntimeSetDesiredStateToolRequest {
    pub fn new(desired_state: DesiredEntityState) -> Self {
        Self { desired_state }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeClearDesiredStateToolRequest {
    pub entity_id: EntityId,
}

impl RuntimeClearDesiredStateToolRequest {
    pub fn new(entity_id: EntityId) -> Self {
        Self { entity_id }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeCommandToolRequest {
    pub entity_id: EntityId,
    pub command_type: CommandType,
    pub arguments: Value,
    pub idempotency_key: Option<String>,
    pub timeout_ms: Option<u64>,
}

impl RuntimeCommandToolRequest {
    pub fn new(entity_id: EntityId, command_type: CommandType, arguments: Value) -> Self {
        Self {
            entity_id,
            command_type,
            arguments,
            idempotency_key: None,
            timeout_ms: None,
        }
    }

    pub fn with_idempotency_key(mut self, idempotency_key: impl Into<String>) -> Self {
        self.idempotency_key = Some(idempotency_key.into());
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }

    fn into_command(
        self,
        command_id: CommandId,
        requested_by: impl Into<String>,
        correlation_id: CorrelationId,
    ) -> Result<DeviceCommand, RuntimeError> {
        let mut command = DeviceCommand::new(
            command_id,
            self.entity_id,
            self.command_type,
            self.arguments,
            requested_by,
            correlation_id,
        )?;
        command.idempotency_key = self.idempotency_key;
        if let Some(timeout_ms) = self.timeout_ms {
            command.timeout_ms = timeout_ms;
        }
        Ok(command)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSupervisionToolRequest {
    ReconcileDesiredStates,
    RunSupervisionTick,
}

impl RuntimeSupervisionToolRequest {
    pub fn tool(self) -> SmartHomeTool {
        match self {
            Self::ReconcileDesiredStates => SmartHomeTool::ReconcileDesiredStates,
            Self::RunSupervisionTick => SmartHomeTool::RunSupervisionTick,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeSupervisionToolOutput {
    DesiredStateReconciliation {
        reconciled_at_ms: u64,
        actions: Vec<DesiredStateAction>,
    },
    SupervisionTick(SupervisionTickReport),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgeHealthSnapshot {
    pub bridge_id: BridgeId,
    pub integration_id: IntegrationId,
    pub health: Health,
    pub last_seen_at_ms: Option<u64>,
}

impl BridgeHealthSnapshot {
    pub fn from_bridge(bridge: &Bridge) -> Self {
        Self {
            bridge_id: bridge.bridge_id.clone(),
            integration_id: bridge.integration_id.clone(),
            health: bridge.health,
            last_seen_at_ms: bridge.last_seen_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeEventLogRecord {
    pub sequence: u64,
    pub next_checkpoint: RuntimeEventCheckpoint,
    pub event: RuntimeEvent,
}

impl RuntimeEventLogRecord {
    pub fn from_entry(entry: RuntimeEventLogEntry<'_>) -> Self {
        Self {
            sequence: entry.sequence,
            next_checkpoint: entry.next_checkpoint,
            event: entry.event.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeReadToolOutput {
    RuntimeSnapshot(RuntimeReadSnapshot),
    DiscoveryWorkers {
        workers: Vec<ScheduledDiscoveryWorkerSnapshot>,
        summary: DiscoveryWorkerSchedulerSnapshot,
    },
    DiscoverySummary {
        generated_at_ms: u64,
        ttl_ms: u64,
        record_summary: DiscoveryRecordSummary,
        signal_summary: DiscoverySignalSummary,
    },
    PairingPlan {
        ttl_ms: u64,
        plan: DiscoveryPairingPlan,
        summary: DiscoveryPairingPlanSummary,
    },
    Bridges(Vec<Bridge>),
    Devices(Vec<Device>),
    Rooms {
        rooms: Vec<RuntimeRoomSummary>,
        topology: RegistryTopologySummary,
    },
    Scenes(Vec<Scene>),
    Scene {
        scene_id: SceneId,
        scene: Scene,
    },
    State {
        entity_id: EntityId,
        snapshot: Option<StateSnapshot>,
    },
    Capabilities {
        entity_id: EntityId,
        capabilities: Vec<Capability>,
    },
    Health(Vec<BridgeHealthSnapshot>),
    Subscriptions {
        subscriptions: Vec<RuntimeSubscriptionSnapshot>,
        summary: RuntimeSubscriptionInventorySummary,
    },
    EventLog {
        entries: Vec<RuntimeEventLogRecord>,
        summary: RuntimeEventLogSummary,
    },
    CommandResults {
        results: Vec<RuntimeCommandResultRecord>,
        summary: RuntimeCommandResultSummary,
    },
    CommandResultSummary {
        summary: RuntimeCommandResultSummary,
    },
    AuthorizationDecisions {
        decisions: Vec<AuthorizationDecision>,
        summary: AuthorizationDecisionLogSummary,
    },
    AuthorizationSummary {
        summary: AuthorizationDecisionLogSummary,
    },
    CapabilityGrants {
        grants: Vec<CapabilityGrant>,
        summary: CapabilityGrantInventorySummary,
    },
    CapabilityGrantSummary {
        summary: CapabilityGrantInventorySummary,
    },
    TopologySummary {
        summary: RegistryTopologySummary,
    },
    DesiredStates {
        desired_states: Vec<DesiredEntityState>,
        summary: DesiredStateInventorySummary,
    },
    PairingSessions {
        sessions: Vec<RuntimePairingSession>,
        summary: RuntimePairingSessionInventorySummary,
    },
    Workers {
        workers: Vec<SupervisedBridgeWorker>,
        summary: RuntimeSupervisorSnapshot,
    },
    WorkerHeartbeatSchedule(WorkerHeartbeatSchedule),
    SupervisionPlan(RuntimeSupervisionPlan),
    SupervisionObservation(RuntimeSupervisionObservation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSubscribeToolOutput {
    pub subscription_id: RuntimeSubscriptionId,
    pub replay_from_checkpoint: RuntimeEventCheckpoint,
    pub subscribed_at_checkpoint: RuntimeEventCheckpoint,
    pub queued_events: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimePollEventsToolOutput {
    pub batch: RuntimeEventDeliveryBatch,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeUnsubscribeToolOutput {
    pub batch: RuntimeEventDeliveryBatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePairBridgeToolOutput {
    pub session: RuntimePairingSession,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCompletePairingToolOutput {
    pub session: RuntimePairingSession,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeReportEventToolOutput {
    Device(DeviceEvent),
    BridgeHealth(BridgeHealthReport),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeSetDesiredStateToolOutput {
    pub desired_state: DesiredEntityState,
    pub replaced: bool,
    pub previous: Option<DesiredEntityState>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeClearDesiredStateToolOutput {
    pub entity_id: EntityId,
    pub removed: Option<DesiredEntityState>,
}

impl RuntimeClearDesiredStateToolOutput {
    pub fn removed(&self) -> bool {
        self.removed.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePairingCompletion {
    pub session_id: RuntimePairingSessionId,
    pub vault_ref: VaultRef,
    pub completed_at_ms: u64,
    pub metadata: Vec<Metadata>,
}

impl RuntimePairingCompletion {
    pub fn new(
        session_id: RuntimePairingSessionId,
        vault_ref: VaultRef,
        completed_at_ms: u64,
    ) -> Self {
        Self {
            session_id,
            vault_ref,
            completed_at_ms,
            metadata: Vec::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: Vec<Metadata>) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone)]
pub struct SmartHomeRuntime {
    registry: InMemorySmartHomeRegistry,
    discovery: DiscoveryCatalog,
    discovery_scheduler: RuntimeDiscoveryScheduler,
    event_bus: RuntimeEventBus,
    supervisor: RuntimeSupervisor,
    pairing_sessions: BTreeMap<RuntimePairingSessionId, RuntimePairingSession>,
    optimistic_states: BTreeMap<EntityId, StateSnapshot>,
    desired_states: BTreeMap<EntityId, DesiredEntityState>,
}

/// Durable, transport-neutral runtime state.
///
/// Discovery scheduling and live subscriptions are intentionally omitted:
/// they are process-local workers and consumers. Everything needed to rebuild
/// normalized topology, state, history, pending pairing work, and desired
/// state is retained.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuntimeDurableSnapshot {
    pub bridges: Vec<Bridge>,
    pub devices: Vec<Device>,
    pub entities: Vec<Entity>,
    pub scenes: Vec<Scene>,
    pub states: Vec<StateSnapshot>,
    pub registry_events: Vec<DeviceEvent>,
    pub capability_grants: Vec<CapabilityGrant>,
    pub authorization_decisions: Vec<AuthorizationDecision>,
    pub runtime_events: Vec<RuntimeEvent>,
    pub pairing_sessions: Vec<RuntimePairingSession>,
    pub optimistic_states: Vec<StateSnapshot>,
    pub desired_states: Vec<DesiredEntityState>,
}

impl SmartHomeRuntime {
    pub fn new() -> Self {
        Self {
            registry: InMemorySmartHomeRegistry::new(),
            discovery: DiscoveryCatalog::new(),
            discovery_scheduler: RuntimeDiscoveryScheduler::new(),
            event_bus: RuntimeEventBus::new(),
            supervisor: RuntimeSupervisor::new(),
            pairing_sessions: BTreeMap::new(),
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

    pub fn durable_snapshot(&self) -> RuntimeDurableSnapshot {
        RuntimeDurableSnapshot {
            bridges: self.registry.bridges().cloned().collect(),
            devices: self.registry.devices().cloned().collect(),
            entities: self.registry.entities().cloned().collect(),
            scenes: self.registry.scenes().cloned().collect(),
            states: self.registry.states().cloned().collect(),
            registry_events: self.registry.events().cloned().collect(),
            capability_grants: self.registry.capability_grants().cloned().collect(),
            authorization_decisions: self.registry.authorization_decisions().cloned().collect(),
            runtime_events: self.event_bus.published().to_vec(),
            pairing_sessions: self.pairing_sessions.values().cloned().collect(),
            optimistic_states: self.optimistic_states.values().cloned().collect(),
            desired_states: self.desired_states.values().cloned().collect(),
        }
    }

    pub fn restore_durable_snapshot(
        snapshot: RuntimeDurableSnapshot,
    ) -> Result<Self, RuntimeError> {
        let RuntimeDurableSnapshot {
            bridges,
            devices,
            entities,
            scenes,
            states,
            registry_events,
            capability_grants,
            authorization_decisions,
            runtime_events,
            pairing_sessions,
            optimistic_states,
            desired_states,
        } = snapshot;
        let mut runtime = Self::new();

        for bridge in bridges {
            runtime.upsert_bridge(bridge)?;
        }
        for device in devices {
            runtime.upsert_device(device)?;
        }
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        for scene in scenes {
            runtime.upsert_scene(scene)?;
        }
        for event in registry_events {
            runtime.registry.record_event(event)?;
        }
        for state in states {
            runtime.registry.apply_state_snapshot(state)?;
        }
        for grant in capability_grants {
            runtime.registry.upsert_capability_grant(grant);
        }
        for decision in authorization_decisions {
            runtime.registry.record_authorization_decision(decision);
        }
        for event in runtime_events {
            runtime.event_bus.publish(event);
        }
        for session in pairing_sessions {
            runtime
                .pairing_sessions
                .insert(session.session_id.clone(), session);
        }
        for state in optimistic_states {
            runtime
                .optimistic_states
                .insert(state.entity_id.clone(), state);
        }
        for desired_state in desired_states {
            runtime.upsert_desired_state(desired_state)?;
        }

        Ok(runtime)
    }

    pub fn discovery(&self) -> &DiscoveryCatalog {
        &self.discovery
    }

    pub fn discovery_mut(&mut self) -> &mut DiscoveryCatalog {
        &mut self.discovery
    }

    pub fn discovery_scheduler(&self) -> &RuntimeDiscoveryScheduler {
        &self.discovery_scheduler
    }

    pub fn discovery_scheduler_mut(&mut self) -> &mut RuntimeDiscoveryScheduler {
        &mut self.discovery_scheduler
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

    pub fn pairing_session_count(&self) -> usize {
        self.pairing_sessions.len()
    }

    pub fn read_snapshot_at(&self, now_ms: u64) -> RuntimeReadSnapshot {
        RuntimeReadSnapshot {
            generated_at_ms: now_ms,
            registry_counts: self.registry.counts(),
            discovery_record_count: self.discovery.len(),
            discovery_scheduler: self.discovery_scheduler.snapshot_at(now_ms),
            event_bus: self.event_bus.snapshot(),
            supervisor: self.supervisor.snapshot_at(now_ms),
            pairing_session_count: self.pairing_sessions.len(),
            expiring_pairing_session_count: self.pairing_sessions_expiring_at(now_ms).len(),
            optimistic_state_count: self.optimistic_states.len(),
            stale_optimistic_state_count: self
                .optimistic_states
                .values()
                .filter(|snapshot| snapshot.is_stale_at(now_ms))
                .count(),
            desired_state_count: self.desired_states.len(),
            desired_capability_count: self
                .desired_states
                .values()
                .map(|desired_state| desired_state.desired.len())
                .sum(),
            state_refresh_target_count: self.registry.state_refresh_plan_at(now_ms).len(),
        }
    }

    pub fn query_discovery_worker_snapshots_at(
        &self,
        query: &DiscoveryWorkerQuery,
        now_ms: u64,
    ) -> Vec<ScheduledDiscoveryWorkerSnapshot> {
        self.discovery_scheduler
            .query_workers(query)
            .into_iter()
            .map(|worker| ScheduledDiscoveryWorkerSnapshot::from_worker_at(worker, now_ms))
            .collect()
    }

    pub fn discovery_summary_at(
        &self,
        request: &RuntimeDiscoverToolRequest,
        now_ms: u64,
    ) -> (DiscoveryRecordSummary, DiscoverySignalSummary) {
        let records = self
            .discovery
            .records()
            .filter(|record| request.matches_record(record, now_ms))
            .collect::<Vec<_>>();
        let record_summary =
            DiscoveryRecordSummary::from_records(records.iter().copied(), now_ms, request.ttl_ms);
        let signals = records
            .iter()
            .map(|record| record.signal(request.ttl_ms))
            .collect::<Vec<_>>();
        let signal_summary = DiscoverySignalSummary::from_signals(&signals, now_ms);
        (record_summary, signal_summary)
    }

    pub fn discovery_pairing_plan_at(
        &self,
        request: &RuntimePairingPlanToolRequest,
        now_ms: u64,
    ) -> DiscoveryPairingPlan {
        let catalog = first_party_catalog();
        self.discovery.pairing_plan_with_options_at(
            &catalog,
            now_ms,
            request.ttl_ms,
            &request.options,
        )
    }

    pub fn topology_summary(&self) -> RegistryTopologySummary {
        self.registry.topology_summary()
    }

    pub fn room_summaries_at(&self, now_ms: u64) -> Vec<RuntimeRoomSummary> {
        let mut rooms = BTreeMap::new();
        let mut entity_rooms = BTreeMap::new();

        for device in self.registry.devices() {
            let Some(room_id) = device.room_id.as_ref() else {
                continue;
            };
            let room = rooms
                .entry(room_id.clone())
                .or_insert_with(|| RuntimeRoomSummary::new(room_id.clone()));
            room.record_device(device);

            for entity in self.registry.entities_for_device(&device.device_id) {
                room.record_entity(entity, self.registry.state(&entity.entity_id), now_ms);
                entity_rooms.insert(entity.entity_id.clone(), room_id.clone());
            }
        }

        for scene in self.registry.scenes() {
            let mut room_action_counts: BTreeMap<String, usize> = BTreeMap::new();
            for action in &scene.actions {
                if let Some(room_id) = entity_rooms.get(&action.entity_id) {
                    *room_action_counts.entry(room_id.clone()).or_default() += 1;
                }
            }

            for (room_id, action_count) in room_action_counts {
                rooms
                    .entry(room_id.clone())
                    .or_insert_with(|| RuntimeRoomSummary::new(room_id))
                    .record_scene_actions(action_count);
            }
        }

        rooms.into_values().collect()
    }

    pub fn query_room_summaries_at(
        &self,
        query: &RuntimeRoomQuery,
        now_ms: u64,
    ) -> Vec<RuntimeRoomSummary> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut rooms = self
            .room_summaries_at(now_ms)
            .into_iter()
            .filter(|room| room_summary_matches_query(room, query))
            .collect::<Vec<_>>();
        match query.sort {
            RuntimeRoomSort::RoomId => {
                rooms.sort_by(|left, right| left.room_id.cmp(&right.room_id))
            }
            RuntimeRoomSort::AttentionDesc => rooms.sort_by(|left, right| {
                right
                    .attention_devices
                    .cmp(&left.attention_devices)
                    .then_with(|| right.state_gap_count().cmp(&left.state_gap_count()))
                    .then_with(|| left.room_id.cmp(&right.room_id))
            }),
            RuntimeRoomSort::EntityCountDesc => rooms.sort_by(|left, right| {
                right
                    .entity_count
                    .cmp(&left.entity_count)
                    .then_with(|| left.room_id.cmp(&right.room_id))
            }),
            RuntimeRoomSort::SceneCountDesc => rooms.sort_by(|left, right| {
                right
                    .scene_count
                    .cmp(&left.scene_count)
                    .then_with(|| left.room_id.cmp(&right.room_id))
            }),
        }
        apply_limit(&mut rooms, query.limit);
        rooms
    }

    pub fn event_bus_health_summary(&self) -> RuntimeEventBusHealthSummary {
        self.event_bus.health_summary()
    }

    // Explicit descending comparator is clearer than sort_by_key+Reverse here (allow 1.97 unnecessary_sort_by).
    #[allow(clippy::unnecessary_sort_by)]
    pub fn query_command_results(
        &self,
        query: &RuntimeCommandResultQuery,
    ) -> Vec<RuntimeCommandResultRecord> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut event_query = RuntimeEventQuery::new()
            .matching(RuntimeEventFilter::Commands)
            .from_checkpoint(query.from_checkpoint);
        if let Some(sequence) = query.to_sequence {
            event_query = event_query.to_sequence(sequence);
        }
        let mut results = self
            .event_bus
            .query_events(&event_query)
            .into_iter()
            .filter_map(RuntimeCommandResultRecord::from_entry)
            .filter(|record| command_result_matches_query(&record.result, query))
            .collect::<Vec<_>>();
        match query.sort {
            RuntimeCommandResultSort::SequenceAsc => {
                results.sort_by_key(|left| left.sequence);
            }
            RuntimeCommandResultSort::SequenceDesc => {
                results.sort_by(|left, right| right.sequence.cmp(&left.sequence));
            }
            RuntimeCommandResultSort::StatusThenSequenceDesc => results.sort_by(|left, right| {
                command_status_sort_rank(left.result.status)
                    .cmp(&command_status_sort_rank(right.result.status))
                    .then_with(|| right.sequence.cmp(&left.sequence))
            }),
        }
        apply_limit(&mut results, query.limit);
        results
    }

    pub fn command_result_summary(
        &self,
        query: &RuntimeCommandResultQuery,
    ) -> RuntimeCommandResultSummary {
        let results = self.query_command_results(query);
        RuntimeCommandResultSummary::from_records(results.iter())
    }

    pub fn pairing_session(
        &self,
        session_id: &RuntimePairingSessionId,
    ) -> Option<&RuntimePairingSession> {
        self.pairing_sessions.get(session_id)
    }

    pub fn query_pairing_sessions(
        &self,
        query: &RuntimePairingSessionQuery,
    ) -> Vec<&RuntimePairingSession> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut sessions = self
            .pairing_sessions
            .values()
            .filter(|session| pairing_session_matches_query(session, query))
            .collect::<Vec<_>>();
        match query.sort {
            RuntimePairingSessionSort::SessionId => {
                sessions.sort_by(|left, right| left.session_id.cmp(&right.session_id));
            }
            RuntimePairingSessionSort::ExpiresAt => sessions.sort_by(|left, right| {
                left.expires_at_ms
                    .cmp(&right.expires_at_ms)
                    .then_with(|| left.session_id.cmp(&right.session_id))
            }),
            RuntimePairingSessionSort::StartedAtDesc => sessions.sort_by(|left, right| {
                right
                    .started_at_ms
                    .cmp(&left.started_at_ms)
                    .then_with(|| left.session_id.cmp(&right.session_id))
            }),
            RuntimePairingSessionSort::StatusThenExpiresAt => sessions.sort_by(|left, right| {
                left.status
                    .as_str()
                    .cmp(right.status.as_str())
                    .then_with(|| left.expires_at_ms.cmp(&right.expires_at_ms))
                    .then_with(|| left.session_id.cmp(&right.session_id))
            }),
        }
        apply_limit(&mut sessions, query.limit);
        sessions
    }

    pub fn pairing_session_inventory_summary_at(
        &self,
        query: &RuntimePairingSessionQuery,
        now_ms: u64,
    ) -> RuntimePairingSessionInventorySummary {
        RuntimePairingSessionInventorySummary::from_sessions_at(
            self.query_pairing_sessions(query),
            now_ms,
        )
    }

    // Explicit descending comparator is clearer than sort_by_key+Reverse here (allow 1.97 unnecessary_sort_by).
    #[allow(clippy::unnecessary_sort_by)]
    pub fn query_authorization_decisions(
        &self,
        query: &RuntimeAuthorizationDecisionQuery,
    ) -> Vec<&AuthorizationDecision> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let selector = query.selector();
        let mut decisions = self.registry.query_authorization_decisions(&selector);
        match query.sort {
            RuntimeAuthorizationDecisionSort::DecidedAtAsc => {
                decisions.sort_by_key(|left| left.decided_at_ms);
            }
            RuntimeAuthorizationDecisionSort::DecidedAtDesc => {
                decisions.sort_by(|left, right| right.decided_at_ms.cmp(&left.decided_at_ms));
            }
        }
        apply_limit(&mut decisions, query.limit);
        decisions
    }

    pub fn authorization_decision_summary(
        &self,
        query: &RuntimeAuthorizationDecisionQuery,
    ) -> AuthorizationDecisionLogSummary {
        AuthorizationDecisionLogSummary::from_decisions(self.query_authorization_decisions(query))
    }

    pub fn query_capability_grants_at(
        &self,
        query: &RuntimeCapabilityGrantQuery,
        now_ms: u64,
    ) -> Vec<&CapabilityGrant> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut grants = match &query.principal_id {
            Some(principal_id) => self.registry.capability_grants_for_principal(principal_id),
            None => self.registry.capability_grants().collect::<Vec<_>>(),
        }
        .into_iter()
        .filter(|grant| capability_grant_matches_query(grant, query, now_ms))
        .collect::<Vec<_>>();
        match query.sort {
            RuntimeCapabilityGrantSort::GrantId => {
                grants.sort_by(|left, right| left.grant_id.cmp(&right.grant_id));
            }
            RuntimeCapabilityGrantSort::PrincipalId => {
                grants.sort_by(|left, right| {
                    left.principal_id
                        .cmp(&right.principal_id)
                        .then_with(|| left.grant_id.cmp(&right.grant_id))
                });
            }
            RuntimeCapabilityGrantSort::GrantedAtAsc => {
                grants.sort_by(|left, right| {
                    left.granted_at_ms
                        .cmp(&right.granted_at_ms)
                        .then_with(|| left.grant_id.cmp(&right.grant_id))
                });
            }
            RuntimeCapabilityGrantSort::GrantedAtDesc => {
                grants.sort_by(|left, right| {
                    right
                        .granted_at_ms
                        .cmp(&left.granted_at_ms)
                        .then_with(|| left.grant_id.cmp(&right.grant_id))
                });
            }
            RuntimeCapabilityGrantSort::ExpiresAtAsc => {
                grants.sort_by(|left, right| {
                    left.expires_at_ms
                        .cmp(&right.expires_at_ms)
                        .then_with(|| left.grant_id.cmp(&right.grant_id))
                });
            }
            RuntimeCapabilityGrantSort::ExpiresAtDesc => {
                grants.sort_by(|left, right| {
                    right
                        .expires_at_ms
                        .cmp(&left.expires_at_ms)
                        .then_with(|| left.grant_id.cmp(&right.grant_id))
                });
            }
        }
        apply_limit(&mut grants, query.limit);
        grants
    }

    pub fn capability_grant_summary_at(
        &self,
        query: &RuntimeCapabilityGrantQuery,
        now_ms: u64,
    ) -> CapabilityGrantInventorySummary {
        CapabilityGrantInventorySummary::from_grants_at(
            self.query_capability_grants_at(query, now_ms),
            now_ms,
        )
    }

    pub fn desired_state(&self, entity_id: &EntityId) -> Option<&DesiredEntityState> {
        self.desired_states.get(entity_id)
    }

    pub fn query_desired_states(&self, query: &DesiredStateQuery) -> Vec<&DesiredEntityState> {
        if query.limit == Some(0) {
            return Vec::new();
        }

        let mut desired_states = self
            .desired_states
            .values()
            .filter(|desired_state| desired_state_matches_query(desired_state, query))
            .collect::<Vec<_>>();
        match query.sort {
            DesiredStateSort::EntityId => {
                desired_states.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));
            }
            DesiredStateSort::RequestedByThenEntityId => desired_states.sort_by(|left, right| {
                left.requested_by
                    .cmp(&right.requested_by)
                    .then_with(|| left.entity_id.cmp(&right.entity_id))
            }),
            DesiredStateSort::CommandTimeoutDesc => desired_states.sort_by(|left, right| {
                right
                    .command_timeout_ms
                    .cmp(&left.command_timeout_ms)
                    .then_with(|| left.entity_id.cmp(&right.entity_id))
            }),
        }
        apply_limit(&mut desired_states, query.limit);
        desired_states
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

    pub fn upsert_scene(&mut self, scene: Scene) -> Result<Option<Scene>, RuntimeError> {
        self.registry.upsert_scene(scene).map_err(Into::into)
    }

    pub fn record_discovery(
        &mut self,
        record: DiscoveryRecord,
    ) -> Result<DiscoveryUpsert, RuntimeError> {
        let bridge_candidate = record.to_bridge_candidate();
        let upsert = self.discovery.record_preferred(record);
        if !matches!(upsert, DiscoveryUpsert::Ignored(_)) {
            self.registry.upsert_bridge(bridge_candidate)?;
        }
        Ok(upsert)
    }

    pub fn record_discovery_worker_run(
        &mut self,
        run: &DiscoveryWorkerRun,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<DiscoveryWorkerRunSummary, RuntimeError> {
        let mut inserted_count = 0;
        let mut replaced_count = 0;
        let mut ignored_count = 0;

        for record in &run.records {
            match self.record_discovery(record.clone())? {
                DiscoveryUpsert::Inserted => inserted_count += 1,
                DiscoveryUpsert::Replaced(_) => replaced_count += 1,
                DiscoveryUpsert::Ignored(_) => ignored_count += 1,
            }
        }

        Ok(run.summary_at(
            now_ms,
            ttl_ms,
            inserted_count,
            replaced_count,
            ignored_count,
        ))
    }

    pub fn register_discovery_worker_schedule(
        &mut self,
        worker: ScheduledDiscoveryWorker,
    ) -> Result<Option<ScheduledDiscoveryWorker>, RuntimeError> {
        self.discovery_scheduler.register_worker(worker)
    }

    pub fn discovery_worker_schedule(
        &self,
        worker_id: &DiscoveryWorkerId,
    ) -> Option<&ScheduledDiscoveryWorker> {
        self.discovery_scheduler.worker(worker_id)
    }

    pub fn query_discovery_worker_schedules(
        &self,
        query: &DiscoveryWorkerQuery,
    ) -> Vec<&ScheduledDiscoveryWorker> {
        self.discovery_scheduler.query_workers(query)
    }

    pub fn discovery_worker_run_plan_at(&self, now_ms: u64) -> DiscoveryWorkerRunPlan {
        self.discovery_scheduler.run_plan_at(now_ms)
    }

    pub fn discovery_mdns_scan_plan_at(
        &self,
        now_ms: u64,
    ) -> Result<MdnsWorkerScanPlan, RuntimeError> {
        self.discovery_worker_run_plan_at(now_ms).mdns_scan_plan()
    }

    pub fn mark_discovery_worker_started(
        &mut self,
        worker_id: &DiscoveryWorkerId,
        now_ms: u64,
    ) -> Result<ScheduledDiscoveryWorker, RuntimeError> {
        self.discovery_scheduler.mark_started(worker_id, now_ms)
    }

    pub fn record_scheduled_discovery_worker_run(
        &mut self,
        run: &DiscoveryWorkerRun,
        now_ms: u64,
        ttl_ms: u64,
    ) -> Result<DiscoveryWorkerRunSummary, RuntimeError> {
        let scheduled = self
            .discovery_scheduler
            .worker(&run.worker_id)
            .ok_or_else(|| RuntimeError::UnknownDiscoveryWorker(run.worker_id.clone()))?;
        if scheduled.integration_id != run.integration_id || scheduled.kind != run.kind {
            return Err(RuntimeError::DiscoveryWorkerRunMismatch {
                worker_id: run.worker_id.clone(),
                expected_integration_id: scheduled.integration_id.clone(),
                actual_integration_id: run.integration_id.clone(),
                expected_kind: scheduled.kind,
                actual_kind: run.kind,
            });
        }

        let summary = self.record_discovery_worker_run(run, now_ms, ttl_ms)?;
        self.discovery_scheduler.record_run_summary(&summary)?;
        Ok(summary)
    }

    pub fn run_due_mdns_discovery_workers_with_executor<E, A>(
        &mut self,
        started_at_ms: u64,
        completed_at_ms: u64,
        ttl_ms: u64,
        executor: &mut E,
        adapter: &mut A,
    ) -> Result<DiscoverySupervisorRunReport, RuntimeError>
    where
        E: MdnsWorkerScanExecutor + ?Sized,
        A: MdnsDiscoveryRunAdapter,
    {
        let instructions = self
            .discovery_worker_run_plan_at(started_at_ms)
            .instructions
            .into_iter()
            .filter(|instruction| {
                instruction.kind == DiscoveryWorkerKind::MdnsScan
                    && instruction.sources.contains(&DiscoverySource::Mdns)
            })
            .collect::<Vec<_>>();
        for instruction in &instructions {
            self.mark_discovery_worker_started(&instruction.worker_id, started_at_ms)?;
        }

        let planned_instruction_count = instructions.len();
        let mdns_scan_plan = DiscoveryWorkerRunPlan {
            generated_at_ms: started_at_ms,
            instructions,
        }
        .mdns_scan_plan()?;
        let mdns_request_count = mdns_scan_plan.len();
        let scan_reports = run_mdns_worker_scan_plan_with_executor(
            &mdns_scan_plan,
            started_at_ms,
            completed_at_ms,
            executor,
        )?;
        let mut report = DiscoverySupervisorRunReport::new(
            started_at_ms,
            completed_at_ms,
            ttl_ms,
            planned_instruction_count,
            mdns_request_count,
            scan_reports.len(),
        );

        for scan_report in &scan_reports {
            let worker_run = match adapter.worker_run_from_mdns_scan_report(scan_report) {
                Ok(run) => run,
                Err(error) => {
                    let message = error.to_string();
                    report.failures.push(DiscoverySupervisorRunFailure {
                        worker_id: scan_report.worker_id.clone(),
                        integration_id: scan_report.integration_id.clone(),
                        kind: DiscoveryWorkerKind::MdnsScan,
                        message: message.clone(),
                    });
                    failed_mdns_discovery_worker_run_from_report(scan_report, message)?
                }
            };
            report
                .summaries
                .push(self.record_scheduled_discovery_worker_run(
                    &worker_run,
                    completed_at_ms,
                    ttl_ms,
                )?);
        }

        Ok(report)
    }

    pub fn run_due_mdns_discovery_workers<A>(
        &mut self,
        started_at_ms: u64,
        completed_at_ms: u64,
        ttl_ms: u64,
        adapter: &mut A,
    ) -> Result<DiscoverySupervisorRunReport, RuntimeError>
    where
        A: MdnsDiscoveryRunAdapter,
    {
        let mut executor = UdpMdnsWorkerScanExecutor;
        self.run_due_mdns_discovery_workers_with_executor(
            started_at_ms,
            completed_at_ms,
            ttl_ms,
            &mut executor,
            adapter,
        )
    }

    pub fn discovery_record_count(&self) -> usize {
        self.discovery.len()
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

    pub fn start_pairing_session(
        &mut self,
        session: RuntimePairingSession,
    ) -> Result<RuntimePairingSession, RuntimeError> {
        if self.pairing_sessions.contains_key(&session.session_id) {
            return Err(RuntimeError::DuplicatePairingSession(session.session_id));
        }
        if session.started_at_ms >= session.expires_at_ms {
            return Err(RuntimeError::PairingSessionExpired {
                session_id: session.session_id,
                expired_at_ms: session.expires_at_ms,
                now_ms: session.started_at_ms,
            });
        }
        self.registry
            .bridge(&session.bridge_id)
            .ok_or_else(|| RuntimeError::UnknownBridge(session.bridge_id.clone()))?;

        self.pairing_sessions
            .insert(session.session_id.clone(), session.clone());
        Ok(session)
    }

    pub fn execute_pair_bridge_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimePairBridgeToolRequest,
        now_ms: u64,
    ) -> Result<RuntimePairBridgeToolOutput, RuntimeError> {
        let tool = SmartHomeTool::PairBridge;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        let bridge = self
            .registry
            .bridge(&request.bridge_id)
            .cloned()
            .ok_or_else(|| RuntimeError::UnknownBridge(request.bridge_id.clone()))?;
        let session = RuntimePairingSession::pending(
            request.session_id,
            &bridge,
            principal_id,
            now_ms,
            request.expires_at_ms,
            request.metadata,
        );
        Ok(RuntimePairBridgeToolOutput {
            session: self.start_pairing_session(session)?,
        })
    }

    pub fn execute_complete_pairing_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeCompletePairingToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeCompletePairingToolOutput, RuntimeError> {
        let tool = SmartHomeTool::CompletePairing;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        Ok(RuntimeCompletePairingToolOutput {
            session: self.complete_pairing_session_with(request.completion)?,
        })
    }

    pub fn execute_report_event_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeReportEventToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeReportEventToolOutput, RuntimeError> {
        let tool = SmartHomeTool::ReportEvent;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        match request {
            RuntimeReportEventToolRequest::Device(event) => {
                self.apply_device_event(event.clone())?;
                Ok(RuntimeReportEventToolOutput::Device(event))
            }
            RuntimeReportEventToolRequest::BridgeHealth(report) => {
                self.apply_bridge_health(report.clone())?;
                Ok(RuntimeReportEventToolOutput::BridgeHealth(report))
            }
        }
    }

    pub fn execute_set_desired_state_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeSetDesiredStateToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeSetDesiredStateToolOutput, RuntimeError> {
        let tool = SmartHomeTool::SetDesiredState;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        let desired_state = request.desired_state;
        let previous = self.upsert_desired_state(desired_state.clone())?;
        Ok(RuntimeSetDesiredStateToolOutput {
            desired_state,
            replaced: previous.is_some(),
            previous,
        })
    }

    pub fn execute_clear_desired_state_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeClearDesiredStateToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeClearDesiredStateToolOutput, RuntimeError> {
        let tool = SmartHomeTool::ClearDesiredState;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        let entity_id = request.entity_id;
        let removed = self.remove_desired_state(&entity_id);
        Ok(RuntimeClearDesiredStateToolOutput { entity_id, removed })
    }

    pub fn complete_pairing_session(
        &mut self,
        session_id: &RuntimePairingSessionId,
        vault_ref: VaultRef,
        completed_at_ms: u64,
    ) -> Result<RuntimePairingSession, RuntimeError> {
        self.complete_pairing_session_with(RuntimePairingCompletion::new(
            session_id.clone(),
            vault_ref,
            completed_at_ms,
        ))
    }

    pub fn complete_pairing_session_with(
        &mut self,
        completion: RuntimePairingCompletion,
    ) -> Result<RuntimePairingSession, RuntimeError> {
        let RuntimePairingCompletion {
            session_id,
            vault_ref,
            completed_at_ms,
            metadata,
        } = completion;
        let session = self
            .pairing_sessions
            .get(&session_id)
            .cloned()
            .ok_or_else(|| RuntimeError::UnknownPairingSession(session_id.clone()))?;
        if session.status != PairingSessionStatus::PendingUserPresence {
            return Err(RuntimeError::PairingSessionNotPending {
                session_id,
                status: session.status,
            });
        }
        if completed_at_ms >= session.expires_at_ms {
            let mut expired = session.clone();
            expired.status = PairingSessionStatus::Expired;
            self.pairing_sessions.insert(session_id.clone(), expired);
            return Err(RuntimeError::PairingSessionExpired {
                session_id,
                expired_at_ms: session.expires_at_ms,
                now_ms: completed_at_ms,
            });
        }

        let mut completed = session;
        completed.status = PairingSessionStatus::Completed;
        completed.vault_ref = Some(vault_ref.clone());
        completed.metadata.extend(metadata.iter().cloned());
        self.pairing_sessions
            .insert(session_id.clone(), completed.clone());

        let mut bridge = self
            .registry
            .bridge(&completed.bridge_id)
            .cloned()
            .ok_or_else(|| RuntimeError::UnknownBridge(completed.bridge_id.clone()))?;
        bridge.auth_ref = Some(vault_ref);
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(completed_at_ms);
        self.registry.upsert_bridge(bridge)?;
        let mut event_metadata = vec![Metadata::new(
            "smart_home.pairing_session",
            completed.session_id.as_str(),
        )];
        event_metadata.extend(metadata);
        self.apply_bridge_health(BridgeHealthReport {
            event_id: EventId::trusted(format!(
                "pairing.completed.health:{}:{completed_at_ms}",
                completed.bridge_id.as_str()
            )),
            bridge_id: completed.bridge_id.clone(),
            health: Health::Online,
            observed_at_ms: completed_at_ms,
            received_at_ms: completed_at_ms,
            metadata: event_metadata,
        })?;

        Ok(completed)
    }

    pub fn expire_pairing_sessions(&mut self, now_ms: u64) -> Vec<RuntimePairingSessionId> {
        let expired_ids: Vec<_> = self
            .pairing_sessions
            .iter()
            .filter(|(_, session)| session.is_expired_at(now_ms))
            .map(|(session_id, _)| session_id.clone())
            .collect();

        for session_id in &expired_ids {
            if let Some(session) = self.pairing_sessions.get_mut(session_id) {
                session.status = PairingSessionStatus::Expired;
            }
        }

        expired_ids
    }

    pub fn authorize_tool_for_principal(
        &mut self,
        principal_id: AgentId,
        tool: SmartHomeTool,
        now_ms: u64,
    ) -> AuthorizationDecision {
        let grants = self.registry.capability_grants_for_principal(&principal_id);
        let decision = AuthorizationDecision::for_tool(principal_id, tool, grants, now_ms);
        self.registry
            .record_authorization_decision(decision.clone());
        decision
    }

    pub fn execute_discover_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeDiscoverToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeDiscoverToolOutput, RuntimeError> {
        let tool = SmartHomeTool::Discover;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        let mut records = self
            .discovery
            .records()
            .filter(|record| request.matches_record(record, now_ms))
            .cloned()
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            right
                .discovered_at_ms
                .cmp(&left.discovered_at_ms)
                .then_with(|| left.integration_id.cmp(&right.integration_id))
                .then_with(|| left.native_bridge_id.cmp(&right.native_bridge_id))
        });
        apply_limit(&mut records, request.limit);

        let bridge_candidates = records
            .iter()
            .map(DiscoveryRecord::to_bridge_candidate)
            .collect::<Vec<_>>();
        let record_summary =
            DiscoveryRecordSummary::from_records(records.iter(), now_ms, request.ttl_ms);
        let signals = records
            .iter()
            .map(|record| record.signal(request.ttl_ms))
            .collect::<Vec<_>>();
        let signal_summary = DiscoverySignalSummary::from_signals(&signals, now_ms);

        Ok(RuntimeDiscoverToolOutput {
            generated_at_ms: now_ms,
            ttl_ms: request.ttl_ms,
            records,
            bridge_candidates,
            record_summary,
            signal_summary,
        })
    }

    pub fn execute_read_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeReadToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeReadToolOutput, RuntimeError> {
        let tool = request.tool();
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        match request {
            RuntimeReadToolRequest::GetRuntimeSnapshot => Ok(
                RuntimeReadToolOutput::RuntimeSnapshot(self.read_snapshot_at(now_ms)),
            ),
            RuntimeReadToolRequest::ListDiscoveryWorkers { query } => {
                Ok(RuntimeReadToolOutput::DiscoveryWorkers {
                    workers: self.query_discovery_worker_snapshots_at(&query, now_ms),
                    summary: self.discovery_scheduler.snapshot_at(now_ms),
                })
            }
            RuntimeReadToolRequest::GetDiscoverySummary { request } => {
                let (record_summary, signal_summary) = self.discovery_summary_at(&request, now_ms);
                Ok(RuntimeReadToolOutput::DiscoverySummary {
                    generated_at_ms: now_ms,
                    ttl_ms: request.ttl_ms,
                    record_summary,
                    signal_summary,
                })
            }
            RuntimeReadToolRequest::GetPairingPlan { request } => {
                let plan = self.discovery_pairing_plan_at(&request, now_ms);
                let summary = plan.summary();
                Ok(RuntimeReadToolOutput::PairingPlan {
                    ttl_ms: request.ttl_ms,
                    plan,
                    summary,
                })
            }
            RuntimeReadToolRequest::ListBridges => Ok(RuntimeReadToolOutput::Bridges(
                self.registry.bridges().cloned().collect(),
            )),
            RuntimeReadToolRequest::ListDevices {
                bridge_id,
                health,
                capability_id,
            } => {
                let mut selector = DeviceSelector::new();
                if let Some(bridge_id) = bridge_id {
                    selector = selector.for_bridge(bridge_id);
                }
                if let Some(health) = health {
                    selector = selector.with_health(health);
                }
                if let Some(capability_id) = capability_id {
                    selector = selector.with_capability(capability_id);
                }
                Ok(RuntimeReadToolOutput::Devices(
                    self.registry
                        .query_devices(&selector)
                        .into_iter()
                        .cloned()
                        .collect(),
                ))
            }
            RuntimeReadToolRequest::ListRooms { query } => Ok(RuntimeReadToolOutput::Rooms {
                rooms: self.query_room_summaries_at(&query, now_ms),
                topology: self.topology_summary(),
            }),
            RuntimeReadToolRequest::ListScenes {
                scope,
                entity_id,
                capability_id,
            } => Ok(RuntimeReadToolOutput::Scenes(
                self.registry
                    .scenes()
                    .filter(|scene| scope.is_none_or(|scope| scene.scope == scope))
                    .filter(|scene| {
                        entity_id
                            .as_ref()
                            .is_none_or(|entity_id| scene_has_action_for_entity(scene, entity_id))
                    })
                    .filter(|scene| {
                        capability_id.as_ref().is_none_or(|capability_id| {
                            scene_has_action_for_capability(&self.registry, scene, capability_id)
                        })
                    })
                    .cloned()
                    .collect(),
            )),
            RuntimeReadToolRequest::DescribeScene { scene_id } => {
                let scene = self
                    .registry
                    .scene(&scene_id)
                    .ok_or_else(|| RuntimeError::UnknownScene(scene_id.clone()))?
                    .clone();
                Ok(RuntimeReadToolOutput::Scene { scene_id, scene })
            }
            RuntimeReadToolRequest::GetState { entity_id } => {
                if self.registry.entity(&entity_id).is_none() {
                    return Err(RuntimeError::UnknownEntity(entity_id));
                }
                Ok(RuntimeReadToolOutput::State {
                    entity_id: entity_id.clone(),
                    snapshot: self.registry.state(&entity_id).cloned(),
                })
            }
            RuntimeReadToolRequest::DescribeCapabilities { entity_id } => {
                let entity = self
                    .registry
                    .entity(&entity_id)
                    .ok_or_else(|| RuntimeError::UnknownEntity(entity_id.clone()))?;
                Ok(RuntimeReadToolOutput::Capabilities {
                    entity_id,
                    capabilities: entity.capabilities.clone(),
                })
            }
            RuntimeReadToolRequest::GetHealth { bridge_id } => match bridge_id {
                Some(bridge_id) => {
                    let bridge = self
                        .registry
                        .bridge(&bridge_id)
                        .ok_or_else(|| RuntimeError::UnknownBridge(bridge_id.clone()))?;
                    Ok(RuntimeReadToolOutput::Health(vec![
                        BridgeHealthSnapshot::from_bridge(bridge),
                    ]))
                }
                None => Ok(RuntimeReadToolOutput::Health(
                    self.registry
                        .bridges()
                        .map(BridgeHealthSnapshot::from_bridge)
                        .collect(),
                )),
            },
            RuntimeReadToolRequest::ListSubscriptions { query } => {
                let subscriptions = self.event_bus.query_subscriptions(&query);
                let summary =
                    RuntimeSubscriptionInventorySummary::from_snapshots(subscriptions.iter());
                Ok(RuntimeReadToolOutput::Subscriptions {
                    subscriptions,
                    summary,
                })
            }
            RuntimeReadToolRequest::InspectEventLog { query } => {
                let entries = self
                    .event_bus
                    .query_events(&query)
                    .into_iter()
                    .map(RuntimeEventLogRecord::from_entry)
                    .collect::<Vec<_>>();
                let summary = self.event_bus.event_log_summary(&query);
                Ok(RuntimeReadToolOutput::EventLog { entries, summary })
            }
            RuntimeReadToolRequest::ListCommandResults { query } => {
                let results = self.query_command_results(&query);
                let summary = RuntimeCommandResultSummary::from_records(results.iter());
                Ok(RuntimeReadToolOutput::CommandResults { results, summary })
            }
            RuntimeReadToolRequest::GetCommandResultSummary { query } => {
                Ok(RuntimeReadToolOutput::CommandResultSummary {
                    summary: self.command_result_summary(&query),
                })
            }
            RuntimeReadToolRequest::ListAuthorizationDecisions { query } => {
                let decision_refs = self.query_authorization_decisions(&query);
                let summary =
                    AuthorizationDecisionLogSummary::from_decisions(decision_refs.iter().copied());
                let decisions = decision_refs.into_iter().cloned().collect();
                Ok(RuntimeReadToolOutput::AuthorizationDecisions { decisions, summary })
            }
            RuntimeReadToolRequest::GetAuthorizationSummary { query } => {
                Ok(RuntimeReadToolOutput::AuthorizationSummary {
                    summary: self.authorization_decision_summary(&query),
                })
            }
            RuntimeReadToolRequest::ListCapabilityGrants { query } => {
                let grant_refs = self.query_capability_grants_at(&query, now_ms);
                let summary = CapabilityGrantInventorySummary::from_grants_at(
                    grant_refs.iter().copied(),
                    now_ms,
                );
                let grants = grant_refs.into_iter().cloned().collect();
                Ok(RuntimeReadToolOutput::CapabilityGrants { grants, summary })
            }
            RuntimeReadToolRequest::GetCapabilityGrantSummary { query } => {
                Ok(RuntimeReadToolOutput::CapabilityGrantSummary {
                    summary: self.capability_grant_summary_at(&query, now_ms),
                })
            }
            RuntimeReadToolRequest::GetTopologySummary => {
                Ok(RuntimeReadToolOutput::TopologySummary {
                    summary: self.topology_summary(),
                })
            }
            RuntimeReadToolRequest::ListDesiredStates { query } => {
                let desired_state_refs = self.query_desired_states(&query);
                let summary =
                    DesiredStateInventorySummary::from_states(desired_state_refs.iter().copied());
                let desired_states = desired_state_refs.into_iter().cloned().collect();
                Ok(RuntimeReadToolOutput::DesiredStates {
                    desired_states,
                    summary,
                })
            }
            RuntimeReadToolRequest::ListPairingSessions { query } => {
                let session_refs = self.query_pairing_sessions(&query);
                let summary = RuntimePairingSessionInventorySummary::from_sessions_at(
                    session_refs.iter().copied(),
                    now_ms,
                );
                let sessions = session_refs.into_iter().cloned().collect();
                Ok(RuntimeReadToolOutput::PairingSessions { sessions, summary })
            }
            RuntimeReadToolRequest::ListWorkers { query } => {
                let worker_refs = self.supervisor.query_workers(&query);
                let summary =
                    RuntimeSupervisorSnapshot::from_workers_at(worker_refs.iter().copied(), now_ms);
                let workers = worker_refs.into_iter().cloned().collect();
                Ok(RuntimeReadToolOutput::Workers { workers, summary })
            }
            RuntimeReadToolRequest::GetWorkerHeartbeatSchedule {
                bridge_id,
                due_at_or_before_ms,
                limit,
            } => {
                let mut schedule = self.worker_heartbeat_schedule_at(now_ms);
                schedule.deadlines.retain(|deadline| {
                    bridge_id
                        .as_ref()
                        .is_none_or(|bridge_id| &deadline.bridge_id == bridge_id)
                        && due_at_or_before_ms
                            .is_none_or(|due_at_ms| deadline.due_at_ms <= due_at_ms)
                });
                if let Some(limit) = limit {
                    schedule.deadlines.truncate(limit);
                }
                Ok(RuntimeReadToolOutput::WorkerHeartbeatSchedule(schedule))
            }
            RuntimeReadToolRequest::GetSupervisionPlan => Ok(
                RuntimeReadToolOutput::SupervisionPlan(self.supervision_plan_at(now_ms)?),
            ),
            RuntimeReadToolRequest::ObserveSupervision => Ok(
                RuntimeReadToolOutput::SupervisionObservation(self.observe_supervision_at(now_ms)?),
            ),
        }
    }

    pub fn execute_supervision_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeSupervisionToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeSupervisionToolOutput, RuntimeError> {
        let tool = request.tool();
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        match request {
            RuntimeSupervisionToolRequest::ReconcileDesiredStates => {
                Ok(RuntimeSupervisionToolOutput::DesiredStateReconciliation {
                    reconciled_at_ms: now_ms,
                    actions: self.reconcile_desired_states(now_ms)?,
                })
            }
            RuntimeSupervisionToolRequest::RunSupervisionTick => Ok(
                RuntimeSupervisionToolOutput::SupervisionTick(self.run_supervision_tick(now_ms)?),
            ),
        }
    }

    pub fn execute_subscribe_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeSubscribeToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeSubscribeToolOutput, RuntimeError> {
        let tool = SmartHomeTool::Subscribe;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        let subscribed_at_checkpoint = self.event_bus.checkpoint();
        let replay_from_checkpoint = request.from_checkpoint.unwrap_or(subscribed_at_checkpoint);
        let queued_events = self
            .event_bus
            .replay_from(replay_from_checkpoint, &request.filter)
            .len();
        let subscription_id = request.subscription_id;
        self.event_bus.subscribe_from_checkpoint(
            subscription_id.clone(),
            request.filter,
            replay_from_checkpoint,
        )?;

        Ok(RuntimeSubscribeToolOutput {
            subscription_id,
            replay_from_checkpoint,
            subscribed_at_checkpoint,
            queued_events,
        })
    }

    pub fn execute_poll_events_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimePollEventsToolRequest,
        now_ms: u64,
    ) -> Result<RuntimePollEventsToolOutput, RuntimeError> {
        let tool = SmartHomeTool::PollEvents;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        let options = request.delivery_options();
        let batch = if request.peek {
            self.event_bus
                .peek_deliveries(&request.subscription_id, options)?
        } else {
            self.event_bus
                .drain_deliveries(&request.subscription_id, options)?
        };
        Ok(RuntimePollEventsToolOutput { batch })
    }

    pub fn execute_unsubscribe_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeUnsubscribeToolRequest,
        now_ms: u64,
    ) -> Result<RuntimeUnsubscribeToolOutput, RuntimeError> {
        let tool = SmartHomeTool::Unsubscribe;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        Ok(RuntimeUnsubscribeToolOutput {
            batch: self.event_bus.unsubscribe(&request.subscription_id)?,
        })
    }

    pub fn execute_command_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, RuntimeError> {
        let command = self.authorize_command_tool(principal_id, request, now_ms)?;
        self.submit_command(command, now_ms)
    }

    pub fn authorize_command_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<DeviceCommand, RuntimeError> {
        let tool = SmartHomeTool::Command;
        let decision = self.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
        if !decision.missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            });
        }

        let sequence = self.registry.counts().authorization_decisions;
        let command_id = CommandId::trusted(format!(
            "tool:{}:{}:{now_ms}:{sequence}",
            principal_id.as_str(),
            request.entity_id.as_str()
        ));
        let correlation_id = CorrelationId::trusted(format!(
            "tool:{}:{}:{now_ms}:{sequence}",
            principal_id.as_str(),
            request.entity_id.as_str()
        ));
        let grants = self
            .registry
            .capability_grants_for_principal(&principal_id)
            .into_iter()
            .cloned()
            .collect();
        let command = request.into_command(command_id, principal_id.as_str(), correlation_id)?;
        let authorization = CommandAuthorization::new(principal_id, grants);
        let decision = AuthorizationDecision::for_command(
            authorization.principal_id.clone(),
            &command,
            authorization.grants.iter(),
            now_ms,
        );
        let missing_capabilities = decision.missing_capabilities.clone();
        self.registry.record_authorization_decision(decision);
        if !missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedCommand {
                command_id: command.command_id.clone(),
                principal_id: authorization.principal_id,
                required_tier: command.required_tier,
                missing_capabilities,
            });
        }
        self.command_bridge_id(&command)?;
        Ok(command)
    }

    pub fn submit_command(
        &mut self,
        command: DeviceCommand,
        now_ms: u64,
    ) -> Result<CommandResult, RuntimeError> {
        let bridge_id = self.command_bridge_id(&command)?;

        if let Some(snapshot) = optimistic_snapshot_for_command(&command, now_ms) {
            self.registry.apply_state_snapshot(snapshot.clone())?;
            self.optimistic_states
                .insert(command.entity_id.clone(), snapshot);
        }

        let result = CommandResult {
            command_id: command.command_id,
            status: CommandStatus::Accepted,
            bridge_id,
            correlation_id: command.correlation_id,
            message: Some("accepted for integration dispatch".to_string()),
        };
        self.event_bus
            .publish(RuntimeEvent::CommandResult(result.clone()));
        Ok(result)
    }

    fn command_bridge_id(&self, command: &DeviceCommand) -> Result<BridgeId, RuntimeError> {
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

        validate_command_capabilities(&entity, command)?;
        Ok(device.bridge_id)
    }

    pub fn submit_authorized_command(
        &mut self,
        authorization: &CommandAuthorization,
        command: DeviceCommand,
        now_ms: u64,
    ) -> Result<CommandResult, RuntimeError> {
        let decision = AuthorizationDecision::for_command(
            authorization.principal_id.clone(),
            &command,
            authorization.grants.iter(),
            now_ms,
        );
        let missing_capabilities = decision.missing_capabilities.clone();
        self.registry.record_authorization_decision(decision);
        if !missing_capabilities.is_empty() {
            return Err(RuntimeError::UnauthorizedCommand {
                command_id: command.command_id.clone(),
                principal_id: authorization.principal_id.clone(),
                required_tier: command.required_tier,
                missing_capabilities,
            });
        }
        self.submit_command(command, now_ms)
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

    pub fn desired_state_drift_plan_at(
        &self,
        now_ms: u64,
    ) -> Result<Vec<DesiredStateDriftPlan>, RuntimeError> {
        let mut drifts = Vec::new();

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
                drifts.push(DesiredStateDriftPlan {
                    bridge_id: device.bridge_id.clone(),
                    entity_id: desired_state.entity_id.clone(),
                    capability_id: desired.capability_id.clone(),
                    desired_value: desired.value.clone(),
                    reason,
                });
            }
        }

        Ok(drifts)
    }

    pub fn supervision_plan_at(&self, now_ms: u64) -> Result<RuntimeSupervisionPlan, RuntimeError> {
        Ok(RuntimeSupervisionPlan {
            generated_at_ms: now_ms,
            pairing_sessions_expiring: self.pairing_sessions_expiring_at(now_ms),
            state_refresh_plan: self.registry.state_refresh_plan_at(now_ms),
            desired_state_drifts: self.desired_state_drift_plan_at(now_ms)?,
            worker_restart_plan: self.supervisor.restart_plan_at(now_ms),
            discovery_worker_run_plan: self.discovery_scheduler.run_plan_at(now_ms),
        })
    }

    pub fn observe_supervision_at(
        &self,
        now_ms: u64,
    ) -> Result<RuntimeSupervisionObservation, RuntimeError> {
        Ok(RuntimeSupervisionObservation {
            generated_at_ms: now_ms,
            plan: self.supervision_plan_at(now_ms)?,
            heartbeat_schedule: self.supervisor.heartbeat_schedule_at(now_ms),
            discovery_scheduler: self.discovery_scheduler.snapshot_at(now_ms),
            discovery_workers: self.discovery_scheduler.worker_snapshots_at(now_ms),
        })
    }

    pub fn pairing_sessions_expiring_at(&self, now_ms: u64) -> Vec<RuntimePairingSessionId> {
        self.pairing_sessions
            .iter()
            .filter(|(_, session)| session.is_expired_at(now_ms))
            .map(|(session_id, _)| session_id.clone())
            .collect()
    }

    pub fn reconcile_supervision(&mut self, now_ms: u64) -> Vec<RuntimeEvent> {
        let plan = self.supervisor.restart_plan_at(now_ms);

        let mut events = Vec::new();
        for instruction in plan.instructions {
            let bridge_id = instruction.bridge_id;
            let integration_id = instruction.integration_id;
            if self.supervisor.mark_restart_requested(&bridge_id).is_err() {
                continue;
            }
            self.mark_registered_bridge_degraded_for_restart(
                &bridge_id,
                &integration_id,
                instruction.planned_at_ms,
            );
            let event = RuntimeEvent::WorkerNeedsRestart {
                bridge_id,
                integration_id,
                overdue_at_ms: instruction.planned_at_ms,
            };
            self.event_bus.publish(event.clone());
            events.push(event);
        }
        events
    }

    pub fn worker_restart_plan_at(&self, now_ms: u64) -> WorkerRestartPlan {
        self.supervisor.restart_plan_at(now_ms)
    }

    pub fn worker_heartbeat_schedule_at(&self, now_ms: u64) -> WorkerHeartbeatSchedule {
        self.supervisor.heartbeat_schedule_at(now_ms)
    }

    pub fn run_supervision_tick(
        &mut self,
        now_ms: u64,
    ) -> Result<SupervisionTickReport, RuntimeError> {
        let expired_pairing_sessions = self.expire_pairing_sessions(now_ms);
        let expired_entities = self.expire_optimistic_states(now_ms)?;
        let desired_state_actions = self.reconcile_desired_states(now_ms)?;
        let worker_events = self.reconcile_supervision(now_ms);

        Ok(SupervisionTickReport {
            ticked_at_ms: now_ms,
            expired_pairing_sessions,
            expired_entities,
            desired_state_actions,
            worker_events,
        })
    }

    fn mark_registered_bridge_degraded_for_restart(
        &mut self,
        bridge_id: &BridgeId,
        integration_id: &IntegrationId,
        now_ms: u64,
    ) {
        let Some(mut bridge) = self.registry.bridge(bridge_id).cloned() else {
            return;
        };
        bridge.health = Health::Degraded;
        self.registry
            .upsert_bridge(bridge)
            .expect("supervision health update uses an existing bridge");

        let event = DeviceEvent {
            event_id: EventId::trusted(format!(
                "supervision.restart.health:{}:{now_ms}",
                bridge_id.as_str()
            )),
            bridge_id: bridge_id.clone(),
            device_id: None,
            entity_id: None,
            observed_at_ms: now_ms,
            received_at_ms: now_ms,
            event_type: DeviceEventType::Health,
            state_delta: None,
            raw_ref: None,
            correlation_id: None,
            metadata: vec![
                Metadata::new("smart_home.health", health_name(Health::Degraded)),
                Metadata::new("smart_home.supervision.reason", "heartbeat_overdue"),
                Metadata::new(
                    "smart_home.supervision.integration_id",
                    integration_id.as_str(),
                ),
            ],
        };
        self.registry
            .record_event(event.clone())
            .expect("supervision health events reference an existing bridge");
        self.event_bus.publish(RuntimeEvent::Device(event.clone()));
        self.event_bus.publish(RuntimeEvent::BridgeHealth {
            event_id: event.event_id,
            bridge_id: bridge_id.clone(),
            health: Health::Degraded,
            observed_at_ms: now_ms,
            received_at_ms: now_ms,
        });
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

fn scene_has_action_for_entity(scene: &Scene, entity_id: &EntityId) -> bool {
    scene
        .actions
        .iter()
        .any(|action| &action.entity_id == entity_id)
}

fn scene_has_action_for_capability(
    registry: &InMemorySmartHomeRegistry,
    scene: &Scene,
    capability_id: &CapabilityId,
) -> bool {
    scene.actions.iter().any(|action| {
        registry.entity(&action.entity_id).is_some_and(|entity| {
            entity
                .capabilities
                .iter()
                .any(|capability| &capability.capability_id == capability_id)
        })
    })
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

fn grant_covers_command_capability(
    grant: &CapabilityGrant,
    principal_id: &AgentId,
    command: &DeviceCommand,
    capability_id: &CapabilityId,
    now_ms: u64,
) -> bool {
    grant.principal_id == *principal_id
        && grant.is_active_at(now_ms)
        && grant.max_tier >= command.required_tier
        && match &grant.scope {
            CapabilityGrantScope::Tool(tool) => *tool == SmartHomeTool::Command,
            CapabilityGrantScope::Capability(granted) => granted == capability_id,
            CapabilityGrantScope::EntityCapability {
                entity_id,
                capability_id: granted,
            } => entity_id == &command.entity_id && granted == capability_id,
            CapabilityGrantScope::AllSmartHome => true,
        }
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
        | CommandType::SetThermostatSetpoint
        | CommandType::Media(_)
        | CommandType::DeviceControl(_) => desired.value.clone(),
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
        | CommandType::SetThermostatSetpoint
        | CommandType::Media(MediaCommandType::SetPlaybackState)
        | CommandType::Media(MediaCommandType::SetVolume)
        | CommandType::Media(MediaCommandType::SetMute)
        | CommandType::Media(MediaCommandType::SetGroup)
        | CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorMode)
        | CommandType::DeviceControl(DeviceControlCommandType::SetIndicatorBrightness)
        | CommandType::DeviceControl(DeviceControlCommandType::SetDisplayBrightness) => {
            command.arguments.clone()
        }
        CommandType::RecallScene => return None,
        CommandType::Media(_) => return None,
        CommandType::DeviceControl(
            DeviceControlCommandType::CalibrateSensor
            | DeviceControlCommandType::SetTemperatureUnit
            | DeviceControlCommandType::SetParticulateDisplayStandard
            | DeviceControlCommandType::SetAutomaticCo2BaselineDays
            | DeviceControlCommandType::SetGasLearningOffsets
            | DeviceControlCommandType::SetCompensatedDisplay
            | DeviceControlCommandType::TestIndicator
            | DeviceControlCommandType::SetCorrectionProfile
            | DeviceControlCommandType::SetCameraRecording
            | DeviceControlCommandType::RecallCameraPtzPreset
            | DeviceControlCommandType::MoveCameraPtz,
        ) => return None,
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

fn invalid_discovery_worker_schedule(
    worker_id: &DiscoveryWorkerId,
    field: &'static str,
    message: impl Into<String>,
) -> RuntimeError {
    RuntimeError::InvalidDiscoveryWorkerSchedule {
        worker_id: worker_id.clone(),
        field,
        message: message.into(),
    }
}

fn failed_mdns_discovery_worker_run_from_report(
    report: &MdnsWorkerScanReport,
    message: impl Into<String>,
) -> Result<DiscoveryWorkerRun, RuntimeError> {
    let mut run = DiscoveryWorkerRun::new(
        report.worker_id.clone(),
        report.integration_id.clone(),
        DiscoveryWorkerKind::MdnsScan,
        report.started_at_ms,
        report.completed_at_ms,
    );
    run.push_failure(DiscoveryWorkerFailure::new(DiscoverySource::Mdns, message)?);
    Ok(run)
}

fn metadata_value<'a>(metadata: &'a [Metadata], key: &str) -> Option<&'a str> {
    metadata
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.value.as_str().trim())
}

fn scheduled_discovery_worker_matches_query(
    worker: &ScheduledDiscoveryWorker,
    query: &DiscoveryWorkerQuery,
) -> bool {
    query
        .worker_id
        .as_ref()
        .is_none_or(|worker_id| &worker.worker_id == worker_id)
        && query
            .integration_id
            .as_ref()
            .is_none_or(|integration_id| &worker.integration_id == integration_id)
        && (query.kinds.is_empty() || query.kinds.contains(&worker.kind))
        && (query.sources.is_empty()
            || query
                .sources
                .iter()
                .all(|source| worker.sources.contains(source)))
        && (query.statuses.is_empty() || query.statuses.contains(&worker.status))
        && query
            .due_before_ms
            .is_none_or(|deadline| worker.next_due_at_ms <= deadline)
        && query
            .overdue_at_ms
            .is_none_or(|now_ms| worker.is_due_at(now_ms))
        && query
            .min_consecutive_failure_count
            .is_none_or(|minimum| worker.consecutive_failure_count >= minimum)
}

fn supervised_worker_matches_query(
    worker: &SupervisedBridgeWorker,
    query: &SupervisedWorkerQuery,
) -> bool {
    query
        .bridge_id
        .as_ref()
        .is_none_or(|bridge_id| &worker.bridge_id == bridge_id)
        && query
            .integration_id
            .as_ref()
            .is_none_or(|integration_id| &worker.integration_id == integration_id)
        && (query.statuses.is_empty() || query.statuses.contains(&worker.status))
        && query.heartbeat_due_before_ms.is_none_or(|deadline| {
            worker
                .heartbeat_due_at_ms()
                .is_some_and(|due| due <= deadline)
        })
        && query
            .overdue_at_ms
            .is_none_or(|now_ms| worker.is_overdue_at(now_ms))
        && query
            .min_restart_count
            .is_none_or(|minimum| worker.restart_count >= minimum)
}

fn room_summary_matches_query(room: &RuntimeRoomSummary, query: &RuntimeRoomQuery) -> bool {
    query
        .room_id
        .as_ref()
        .is_none_or(|room_id| &room.room_id == room_id)
        && (!query.attention_only || room.has_attention_items())
        && (!query.state_gaps_only || room.has_state_gaps())
}

fn desired_state_matches_query(
    desired_state: &DesiredEntityState,
    query: &DesiredStateQuery,
) -> bool {
    query
        .entity_id
        .as_ref()
        .is_none_or(|entity_id| &desired_state.entity_id == entity_id)
        && query
            .requested_by
            .as_ref()
            .is_none_or(|requested_by| &desired_state.requested_by == requested_by)
        && query.capability_id.as_ref().is_none_or(|capability_id| {
            desired_state
                .desired
                .iter()
                .any(|delta| &delta.capability_id == capability_id)
        })
        && query
            .min_command_timeout_ms
            .is_none_or(|minimum| desired_state.command_timeout_ms >= minimum)
        && query
            .max_command_timeout_ms
            .is_none_or(|maximum| desired_state.command_timeout_ms <= maximum)
}

fn capability_grant_matches_query(
    grant: &CapabilityGrant,
    query: &RuntimeCapabilityGrantQuery,
    now_ms: u64,
) -> bool {
    query
        .status
        .is_none_or(|status| grant.status_at(now_ms) == status)
        && query
            .scope_kind
            .is_none_or(|scope_kind| capability_grant_scope_matches_kind(&grant.scope, scope_kind))
        && query
            .capability_id
            .as_ref()
            .is_none_or(|capability_id| match &grant.scope {
                CapabilityGrantScope::Capability(granted)
                | CapabilityGrantScope::EntityCapability {
                    capability_id: granted,
                    ..
                } => granted == capability_id,
                CapabilityGrantScope::Tool(_) | CapabilityGrantScope::AllSmartHome => false,
            })
        && query
            .entity_id
            .as_ref()
            .is_none_or(|entity_id| match &grant.scope {
                CapabilityGrantScope::EntityCapability {
                    entity_id: granted, ..
                } => granted == entity_id,
                CapabilityGrantScope::Tool(_)
                | CapabilityGrantScope::Capability(_)
                | CapabilityGrantScope::AllSmartHome => false,
            })
}

fn capability_grant_scope_matches_kind(
    scope: &CapabilityGrantScope,
    kind: RuntimeCapabilityGrantScopeKind,
) -> bool {
    matches!(
        (scope, kind),
        (
            CapabilityGrantScope::Tool(_),
            RuntimeCapabilityGrantScopeKind::Tool
        ) | (
            CapabilityGrantScope::Capability(_),
            RuntimeCapabilityGrantScopeKind::Capability
        ) | (
            CapabilityGrantScope::EntityCapability { .. },
            RuntimeCapabilityGrantScopeKind::EntityCapability
        ) | (
            CapabilityGrantScope::AllSmartHome,
            RuntimeCapabilityGrantScopeKind::AllSmartHome
        )
    )
}

fn pairing_session_matches_query(
    session: &RuntimePairingSession,
    query: &RuntimePairingSessionQuery,
) -> bool {
    query
        .session_id
        .as_ref()
        .is_none_or(|session_id| &session.session_id == session_id)
        && query
            .bridge_id
            .as_ref()
            .is_none_or(|bridge_id| &session.bridge_id == bridge_id)
        && query
            .integration_id
            .as_ref()
            .is_none_or(|integration_id| &session.integration_id == integration_id)
        && query
            .requested_by
            .as_ref()
            .is_none_or(|requested_by| &session.requested_by == requested_by)
        && (query.statuses.is_empty() || query.statuses.contains(&session.status))
        && query
            .expires_before_ms
            .is_none_or(|deadline| session.expires_at_ms <= deadline)
        && query
            .expiring_at_ms
            .is_none_or(|now_ms| session.is_expired_at(now_ms))
}

fn command_result_matches_query(result: &CommandResult, query: &RuntimeCommandResultQuery) -> bool {
    query
        .command_id
        .as_ref()
        .is_none_or(|command_id| &result.command_id == command_id)
        && query
            .bridge_id
            .as_ref()
            .is_none_or(|bridge_id| &result.bridge_id == bridge_id)
        && query
            .correlation_id
            .as_ref()
            .is_none_or(|correlation_id| &result.correlation_id == correlation_id)
        && (query.statuses.is_empty() || query.statuses.contains(&result.status))
}

fn command_status_sort_rank(status: CommandStatus) -> u8 {
    match status {
        CommandStatus::Accepted => 0,
        CommandStatus::Rejected => 1,
        CommandStatus::TimedOut => 2,
        CommandStatus::Failed => 3,
    }
}

fn apply_limit<T>(items: &mut Vec<T>, limit: Option<usize>) {
    if let Some(limit) = limit {
        items.truncate(limit);
    }
}

fn delivery_count(queue_len: usize, limit: Option<usize>) -> usize {
    limit.unwrap_or(queue_len).min(queue_len)
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
        AuthorizationOutcome, AuthorizationSubject, BridgeTransport, Capability, CapabilityGrantId,
        CommandId, CorrelationId, EntityKind, IntegrationId, ProtocolFamily, ProtocolIdentifier,
        StateDelta,
    };
    use smart_home_discovery::{
        DiscoveryConfidence, DiscoveryPairingAction, DiscoveryRecord, DiscoverySource,
        DiscoveryUpsert, DiscoveryWorkerFailure, DiscoveryWorkerId, DiscoveryWorkerKind,
        DiscoveryWorkerRun, DiscoveryWorkerRunStatus, MdnsResponsePacket, MdnsScanResult,
        MdnsWorkerScanRequest, PairingRequirement,
    };
    use smart_home_registry::StateRefreshReason;

    #[derive(Debug)]
    struct ScriptedMdnsExecutor {
        outcomes: std::collections::VecDeque<Result<MdnsScanResult, DiscoveryError>>,
        requests: Vec<MdnsWorkerScanRequest>,
    }

    impl ScriptedMdnsExecutor {
        fn new(outcomes: impl IntoIterator<Item = Result<MdnsScanResult, DiscoveryError>>) -> Self {
            Self {
                outcomes: outcomes.into_iter().collect(),
                requests: Vec::new(),
            }
        }
    }

    impl MdnsWorkerScanExecutor for ScriptedMdnsExecutor {
        fn run_request(
            &mut self,
            request: &MdnsWorkerScanRequest,
        ) -> Result<MdnsScanResult, DiscoveryError> {
            self.requests.push(request.clone());
            self.outcomes.pop_front().unwrap_or_else(|| {
                Err(DiscoveryError::MdnsTransport {
                    message: "missing scripted mDNS outcome".to_string(),
                })
            })
        }
    }

    #[derive(Debug)]
    struct ScriptedMdnsRunAdapter {
        outcomes: std::collections::VecDeque<Result<DiscoveryWorkerRun, String>>,
        reports: Vec<MdnsWorkerScanReport>,
    }

    impl ScriptedMdnsRunAdapter {
        fn new(outcomes: impl IntoIterator<Item = Result<DiscoveryWorkerRun, String>>) -> Self {
            Self {
                outcomes: outcomes.into_iter().collect(),
                reports: Vec::new(),
            }
        }
    }

    impl MdnsDiscoveryRunAdapter for ScriptedMdnsRunAdapter {
        type Error = String;

        fn worker_run_from_mdns_scan_report(
            &mut self,
            report: &MdnsWorkerScanReport,
        ) -> Result<DiscoveryWorkerRun, Self::Error> {
            self.reports.push(report.clone());
            self.outcomes
                .pop_front()
                .unwrap_or_else(|| Err("missing scripted discovery worker run".to_string()))
        }
    }

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

    fn bridge_health_runtime_event(event_id: &str, bridge_id: &str, at_ms: u64) -> RuntimeEvent {
        RuntimeEvent::BridgeHealth {
            event_id: EventId::trusted(event_id),
            bridge_id: BridgeId::trusted(bridge_id),
            health: Health::Online,
            observed_at_ms: at_ms,
            received_at_ms: at_ms,
        }
    }

    fn hue_discovery_record(native_bridge_id: &str, discovered_at_ms: u64) -> DiscoveryRecord {
        DiscoveryRecord::new(
            IntegrationId::trusted("hue"),
            ProtocolFamily::Hue,
            native_bridge_id,
            DiscoverySource::Mdns,
            BridgeTransport::Mdns,
            discovered_at_ms,
        )
        .unwrap()
        .with_address("https://192.0.2.10")
        .with_confidence(DiscoveryConfidence::Verified)
        .with_pairing_requirement(PairingRequirement::PhysicalPresence)
    }

    fn hue_cloud_discovery_record(
        native_bridge_id: &str,
        discovered_at_ms: u64,
    ) -> DiscoveryRecord {
        DiscoveryRecord::new(
            IntegrationId::trusted("hue"),
            ProtocolFamily::Hue,
            native_bridge_id,
            DiscoverySource::CloudFallback,
            BridgeTransport::Cloud,
            discovered_at_ms,
        )
        .unwrap()
        .with_address("https://192.0.2.20")
        .with_confidence(DiscoveryConfidence::Candidate)
        .with_pairing_requirement(PairingRequirement::PhysicalPresence)
    }

    fn hue_mdns_discovery_worker(first_due_at_ms: u64) -> ScheduledDiscoveryWorker {
        ScheduledDiscoveryWorker::new(
            DiscoveryWorkerId::trusted("hue-mdns-worker"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            5_000,
            250,
            first_due_at_ms,
        )
        .with_source(DiscoverySource::Mdns)
        .with_network_interface("en0")
        .with_network_interface("bridge0")
        .with_metadata("smart_home.discovery.service_type", "_hue._tcp.local")
    }

    fn device_runtime_event(event_id: &str, at_ms: u64) -> RuntimeEvent {
        RuntimeEvent::Device(DeviceEvent {
            event_id: EventId::trusted(event_id),
            bridge_id: BridgeId::trusted("bridge-1"),
            device_id: Some(DeviceId::trusted("device-1")),
            entity_id: Some(EntityId::trusted("entity-1")),
            observed_at_ms: at_ms,
            received_at_ms: at_ms,
            event_type: DeviceEventType::Updated,
            state_delta: Some(StateDelta {
                capability_id: CapabilityId::trusted("light.on_off"),
                value: Value::Bool(true),
            }),
            raw_ref: None,
            correlation_id: None,
            metadata: Vec::new(),
        })
    }

    fn command_result_runtime_event(command_id: &str) -> RuntimeEvent {
        RuntimeEvent::CommandResult(CommandResult {
            command_id: CommandId::trusted(command_id),
            status: CommandStatus::Accepted,
            bridge_id: BridgeId::trusted("bridge-1"),
            correlation_id: CorrelationId::trusted("corr-1"),
            message: None,
        })
    }

    #[test]
    fn event_bus_replays_from_checkpoint_and_continues_delivery() {
        let mut bus = RuntimeEventBus::new();
        let start = RuntimeEventCheckpoint::start();
        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        bus.publish(bridge_health_runtime_event("health-2", "bridge-2", 1_001));
        let after_two = bus.checkpoint();

        let bridge_one_replay = bus.replay_from(
            start,
            &RuntimeEventFilter::Bridge(BridgeId::trusted("bridge-1")),
        );
        assert_eq!(after_two.next_sequence(), 2);
        assert!(matches!(
            bridge_one_replay.as_slice(),
            [RuntimeEvent::BridgeHealth { event_id, bridge_id, .. }]
                if event_id == &EventId::trusted("health-1")
                    && bridge_id == &BridgeId::trusted("bridge-1")
        ));

        let replaying_subscription = RuntimeSubscriptionId::trusted("bridge-1-replay");
        bus.subscribe_from_checkpoint(
            replaying_subscription.clone(),
            RuntimeEventFilter::Bridge(BridgeId::trusted("bridge-1")),
            start,
        )
        .unwrap();
        assert_eq!(bus.drain(&replaying_subscription).unwrap().len(), 1);

        bus.publish(bridge_health_runtime_event("health-3", "bridge-1", 1_002));
        let future = bus.drain(&replaying_subscription).unwrap();
        assert!(matches!(
            future.as_slice(),
            [RuntimeEvent::BridgeHealth { event_id, .. }]
                if event_id == &EventId::trusted("health-3")
        ));

        let current_subscription = RuntimeSubscriptionId::trusted("current-only");
        bus.subscribe_from_checkpoint(
            current_subscription.clone(),
            RuntimeEventFilter::All,
            bus.checkpoint(),
        )
        .unwrap();
        assert!(bus.drain(&current_subscription).unwrap().is_empty());
    }

    #[test]
    fn event_bus_queries_log_entries_and_subscription_backlogs() {
        let mut bus = RuntimeEventBus::new();
        let bridge_one = RuntimeEventFilter::Bridge(BridgeId::trusted("bridge-1"));
        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        let after_first = bus.checkpoint();
        bus.publish(bridge_health_runtime_event("health-2", "bridge-2", 1_001));
        bus.publish(bridge_health_runtime_event("health-3", "bridge-1", 1_002));
        bus.subscribe_from_checkpoint(
            RuntimeSubscriptionId::trusted("bridge-1-stream"),
            bridge_one.clone(),
            RuntimeEventCheckpoint::start(),
        )
        .unwrap();
        bus.subscribe(
            RuntimeSubscriptionId::trusted("all-current"),
            RuntimeEventFilter::All,
        )
        .unwrap();

        let newest_bridge_events = bus.query_events(
            &RuntimeEventQuery::new()
                .from_checkpoint(after_first)
                .matching(bridge_one.clone())
                .sorted_by(RuntimeEventSort::SequenceDesc)
                .with_limit(1),
        );
        assert_eq!(newest_bridge_events.len(), 1);
        assert_eq!(newest_bridge_events[0].sequence, 2);
        assert_eq!(newest_bridge_events[0].next_checkpoint.next_sequence(), 3);

        let early_events = bus.query_events(&RuntimeEventQuery::new().to_sequence(1));
        assert_eq!(early_events.len(), 2);
        assert_eq!(early_events[0].sequence, 0);
        assert_eq!(early_events[1].sequence, 1);

        let empty_window = bus.query_events(
            &RuntimeEventQuery::new()
                .from_checkpoint(RuntimeEventCheckpoint::from_next_sequence(2))
                .to_sequence(1),
        );
        assert!(empty_window.is_empty());

        let backlogs = bus.query_subscriptions(
            &RuntimeSubscriptionQuery::new()
                .matching(bridge_one)
                .with_min_queued_events(1)
                .sorted_by(RuntimeSubscriptionSort::QueuedEventsDesc),
        );
        assert_eq!(backlogs.len(), 1);
        assert_eq!(
            backlogs[0].subscription_id,
            RuntimeSubscriptionId::trusted("bridge-1-stream")
        );
        assert_eq!(backlogs[0].queued_events, 2);
        assert!(backlogs[0].has_backlog());
    }

    #[test]
    fn event_log_summary_counts_selected_event_kinds() {
        let mut bus = RuntimeEventBus::new();
        bus.publish(device_runtime_event("device-event-1", 1_000));
        bus.publish(command_result_runtime_event("cmd-1"));
        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_002));
        bus.publish(RuntimeEvent::StateExpired {
            entity_id: EntityId::trusted("entity-1"),
            expired_at_ms: 1_003,
        });
        bus.publish(RuntimeEvent::DesiredStateDrift {
            bridge_id: BridgeId::trusted("bridge-1"),
            entity_id: EntityId::trusted("entity-1"),
            capability_id: CapabilityId::trusted("light.on_off"),
            reason: ReconciliationReason::Drifted,
            detected_at_ms: 1_004,
        });
        bus.publish(RuntimeEvent::WorkerNeedsRestart {
            bridge_id: BridgeId::trusted("bridge-1"),
            integration_id: IntegrationId::trusted("hue"),
            overdue_at_ms: 1_005,
        });

        let summary = bus.event_log_summary(&RuntimeEventQuery::new());

        assert_eq!(
            summary,
            RuntimeEventLogSummary {
                total_events: 6,
                device_events: 1,
                command_results: 1,
                bridge_health_events: 1,
                state_expired_events: 1,
                desired_state_drift_events: 1,
                worker_restart_events: 1,
                first_sequence: Some(0),
                latest_sequence: Some(5),
                next_checkpoint: RuntimeEventCheckpoint::from_next_sequence(6),
            }
        );
        assert!(summary.has_events());
        assert!(summary.has_command_results());
        assert!(summary.has_supervision_events());

        let newest_supervision = bus.event_log_summary(
            &RuntimeEventQuery::new()
                .matching(RuntimeEventFilter::Supervision)
                .sorted_by(RuntimeEventSort::SequenceDesc)
                .with_limit(1),
        );
        assert_eq!(newest_supervision.total_events, 1);
        assert_eq!(newest_supervision.worker_restart_events, 1);
        assert_eq!(newest_supervision.first_sequence, Some(5));
        assert_eq!(newest_supervision.latest_sequence, Some(5));
        assert_eq!(
            newest_supervision.next_checkpoint,
            RuntimeEventCheckpoint::from_next_sequence(6)
        );

        let early_summary = bus.event_log_summary(&RuntimeEventQuery::new().to_sequence(2));
        assert_eq!(early_summary.total_events, 3);
        assert_eq!(early_summary.first_sequence, Some(0));
        assert_eq!(early_summary.latest_sequence, Some(2));
        assert_eq!(
            early_summary.next_checkpoint,
            RuntimeEventCheckpoint::from_next_sequence(3)
        );

        let empty = bus.event_log_summary(&RuntimeEventQuery::new().with_limit(0));
        assert_eq!(empty, RuntimeEventLogSummary::empty());
        assert!(!empty.has_events());
        assert!(!empty.has_command_results());
        assert!(!empty.has_supervision_events());
    }

    #[test]
    fn command_result_queries_filter_sort_and_summarize_runtime_events() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        runtime
            .event_bus_mut()
            .publish(command_result_runtime_event("cmd-accepted"));
        runtime
            .event_bus_mut()
            .publish(RuntimeEvent::CommandResult(CommandResult {
                command_id: CommandId::trusted("cmd-failed"),
                status: CommandStatus::Failed,
                bridge_id: BridgeId::trusted("bridge-1"),
                correlation_id: CorrelationId::trusted("corr-failed"),
                message: Some("integration dispatch failed".to_string()),
            }));

        let newest_failure = runtime.query_command_results(
            &RuntimeCommandResultQuery::new()
                .for_bridge(BridgeId::trusted("bridge-1"))
                .for_correlation(CorrelationId::trusted("corr-failed"))
                .with_status(CommandStatus::Failed)
                .sorted_by(RuntimeCommandResultSort::SequenceDesc)
                .with_limit(1),
        );

        assert_eq!(newest_failure.len(), 1);
        assert_eq!(newest_failure[0].sequence, 1);
        assert_eq!(
            newest_failure[0].next_checkpoint,
            RuntimeEventCheckpoint::from_next_sequence(2)
        );
        assert_eq!(
            newest_failure[0].result.command_id,
            CommandId::trusted("cmd-failed")
        );

        let first_result =
            runtime.query_command_results(&RuntimeCommandResultQuery::new().to_sequence(0));
        assert_eq!(first_result.len(), 1);
        assert_eq!(first_result[0].sequence, 0);
        assert_eq!(
            first_result[0].result.command_id,
            CommandId::trusted("cmd-accepted")
        );

        let empty_window = runtime.query_command_results(
            &RuntimeCommandResultQuery::new()
                .from_checkpoint(RuntimeEventCheckpoint::from_next_sequence(1))
                .to_sequence(0),
        );
        assert!(empty_window.is_empty());

        let summary = runtime.command_result_summary(
            &RuntimeCommandResultQuery::new()
                .for_bridge(BridgeId::trusted("bridge-1"))
                .sorted_by(RuntimeCommandResultSort::StatusThenSequenceDesc),
        );

        assert_eq!(
            summary,
            RuntimeCommandResultSummary {
                total_results: 2,
                accepted_results: 1,
                rejected_results: 0,
                timed_out_results: 0,
                failed_results: 1,
                first_sequence: Some(0),
                latest_sequence: Some(1),
                next_checkpoint: RuntimeEventCheckpoint::from_next_sequence(2),
            }
        );
        assert!(summary.has_results());
        assert_eq!(summary.failure_results(), 1);
        assert!(summary.has_failures());

        let empty = runtime.query_command_results(
            &RuntimeCommandResultQuery::new()
                .for_command(CommandId::trusted("unknown"))
                .with_limit(0),
        );
        assert!(empty.is_empty());
    }

    #[test]
    fn event_bus_peeks_and_drains_subscription_deliveries_in_batches() {
        let mut bus = RuntimeEventBus::new();
        let subscription = RuntimeSubscriptionId::trusted("all-events");
        bus.subscribe(subscription.clone(), RuntimeEventFilter::All)
            .unwrap();
        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        bus.publish(bridge_health_runtime_event("health-2", "bridge-1", 1_001));
        bus.publish(bridge_health_runtime_event("health-3", "bridge-1", 1_002));

        assert_eq!(bus.queued_events(&subscription).unwrap(), 3);
        let peeked = bus
            .peek_deliveries(
                &subscription,
                RuntimeEventDeliveryOptions::new().with_limit(2),
            )
            .unwrap();
        assert_eq!(peeked.subscription_id, subscription);
        assert_eq!(peeked.len(), 2);
        assert_eq!(peeked.remaining_events, 1);
        assert!(peeked.has_more());
        assert_eq!(
            peeked.summary(),
            RuntimeEventDeliverySummary {
                subscription_id: subscription.clone(),
                delivered_events: 2,
                remaining_events: 1,
                device_events: 0,
                command_results: 0,
                bridge_health_events: 2,
                state_expired_events: 0,
                desired_state_drift_events: 0,
                worker_restart_events: 0,
            }
        );
        assert!(peeked.summary().has_more());
        assert!(!peeked.summary().is_empty());
        assert!(!peeked.summary().has_command_results());
        assert!(!peeked.summary().has_supervision_events());
        assert_eq!(bus.queued_events(&subscription).unwrap(), 3);

        let drained = bus
            .drain_deliveries(
                &subscription,
                RuntimeEventDeliveryOptions::new().with_limit(2),
            )
            .unwrap();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained.remaining_events, 1);
        assert_eq!(drained.summary().bridge_health_events, 2);
        assert!(drained.summary().has_more());
        assert_eq!(bus.queued_events(&subscription).unwrap(), 1);

        let tail = bus.drain(&subscription).unwrap();
        assert_eq!(tail.len(), 1);
        assert_eq!(bus.queued_events(&subscription).unwrap(), 0);
        assert!(bus
            .queued_events(&RuntimeSubscriptionId::trusted("missing"))
            .is_err());
    }

    #[test]
    fn event_bus_unsubscribes_and_returns_undelivered_events() {
        let mut bus = RuntimeEventBus::new();
        let subscription = RuntimeSubscriptionId::trusted("bridge-events");
        bus.subscribe(
            subscription.clone(),
            RuntimeEventFilter::Bridge(BridgeId::trusted("bridge-1")),
        )
        .unwrap();
        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        bus.publish(bridge_health_runtime_event("health-2", "bridge-2", 1_001));

        assert!(bus.has_subscription(&subscription));
        assert_eq!(bus.subscription_count(), 1);
        assert_eq!(bus.pending_delivery_count(), 1);

        let undelivered = bus.unsubscribe(&subscription).unwrap();

        assert_eq!(undelivered.subscription_id, subscription);
        assert_eq!(undelivered.len(), 1);
        assert_eq!(undelivered.remaining_events, 0);
        assert!(!undelivered.has_more());
        assert_eq!(bus.subscription_count(), 0);
        assert_eq!(bus.pending_delivery_count(), 0);
        assert!(!bus.has_subscription(&subscription));
        assert!(bus.queued_events(&subscription).is_err());
        assert!(bus.unsubscribe(&subscription).is_err());

        bus.publish(bridge_health_runtime_event("health-3", "bridge-1", 1_002));
        assert_eq!(bus.pending_delivery_count(), 0);
        assert_eq!(bus.published().len(), 3);
    }

    #[test]
    fn event_bus_snapshot_counts_log_subscriptions_and_backlog() {
        let mut bus = RuntimeEventBus::new();
        let all_events = RuntimeSubscriptionId::trusted("all-events");
        let bridge_events = RuntimeSubscriptionId::trusted("bridge-events");
        bus.subscribe(all_events.clone(), RuntimeEventFilter::All)
            .unwrap();
        bus.subscribe(
            bridge_events.clone(),
            RuntimeEventFilter::Bridge(BridgeId::trusted("bridge-1")),
        )
        .unwrap();

        let idle = bus.snapshot();
        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        bus.publish(bridge_health_runtime_event("health-2", "bridge-2", 1_001));
        let active = bus.snapshot();

        assert_eq!(idle.subscription_count, 2);
        assert_eq!(idle.pending_delivery_count, 0);
        assert_eq!(idle.backlogged_subscription_count, 0);
        assert_eq!(idle.max_pending_delivery_count, 0);
        assert!(idle.is_idle());
        assert!(idle.has_subscriptions());
        assert!(!idle.has_backlog());
        assert!(!idle.has_lagging_subscriptions());
        assert_eq!(
            idle.backlog_status(),
            RuntimeEventBusBacklogStatus::CaughtUp
        );
        assert_eq!(
            idle.pressure_status(),
            RuntimeEventBusPressureStatus::CaughtUp
        );
        assert_eq!(idle.average_pending_deliveries_per_subscription(), 0);
        assert_eq!(idle.caught_up_subscription_count(), 2);
        assert_eq!(idle.backlogged_subscription_percent(), 0);
        assert!(!idle.exceeds_backlogged_subscription_percent(0));
        assert!(!idle.exceeds_subscription_backlog_threshold(0));
        assert_eq!(active.subscription_count, 2);
        assert_eq!(active.published_event_count, 2);
        assert_eq!(active.pending_delivery_count, 3);
        assert_eq!(active.backlogged_subscription_count, 2);
        assert_eq!(active.max_pending_delivery_count, 2);
        assert!(!active.is_idle());
        assert!(active.has_backlog());
        assert!(active.has_lagging_subscriptions());
        assert_eq!(
            active.backlog_status(),
            RuntimeEventBusBacklogStatus::Backlogged
        );
        assert_eq!(
            active.pressure_status(),
            RuntimeEventBusPressureStatus::FullyBacklogged
        );
        assert_eq!(active.average_pending_deliveries_per_subscription(), 1);
        assert_eq!(active.caught_up_subscription_count(), 0);
        assert_eq!(active.backlogged_subscription_percent(), 100);
        assert!(active.exceeds_backlogged_subscription_percent(50));
        assert!(!active.exceeds_backlogged_subscription_percent(100));
        assert!(active.exceeds_subscription_backlog_threshold(1));
        assert!(!active.exceeds_subscription_backlog_threshold(2));

        bus.drain(&bridge_events).unwrap();
        let partial = bus.snapshot();
        assert_eq!(partial.backlogged_subscription_count, 1);
        assert_eq!(partial.caught_up_subscription_count(), 1);
        assert_eq!(partial.backlogged_subscription_percent(), 50);
        assert_eq!(
            partial.pressure_status(),
            RuntimeEventBusPressureStatus::PartiallyBacklogged
        );
    }

    #[test]
    fn event_bus_snapshot_distinguishes_absent_subscribers_from_caught_up_streams() {
        let mut bus = RuntimeEventBus::new();
        let no_subscribers = bus.snapshot();

        bus.subscribe(
            RuntimeSubscriptionId::trusted("all-events"),
            RuntimeEventFilter::All,
        )
        .unwrap();
        let caught_up = bus.snapshot();

        assert_eq!(
            no_subscribers.backlog_status(),
            RuntimeEventBusBacklogStatus::NoSubscriptions
        );
        assert_eq!(
            no_subscribers.pressure_status(),
            RuntimeEventBusPressureStatus::NoSubscriptions
        );
        assert!(!no_subscribers.has_subscriptions());
        assert!(!no_subscribers.has_lagging_subscriptions());
        assert_eq!(no_subscribers.max_pending_delivery_count, 0);
        assert_eq!(no_subscribers.caught_up_subscription_count(), 0);
        assert_eq!(no_subscribers.backlogged_subscription_percent(), 0);
        assert_eq!(
            no_subscribers.average_pending_deliveries_per_subscription(),
            0
        );
        assert_eq!(
            caught_up.backlog_status(),
            RuntimeEventBusBacklogStatus::CaughtUp
        );
        assert_eq!(
            caught_up.pressure_status(),
            RuntimeEventBusPressureStatus::CaughtUp
        );
        assert!(caught_up.has_subscriptions());
        assert!(!caught_up.has_lagging_subscriptions());
        assert_eq!(caught_up.caught_up_subscription_count(), 1);
        assert_eq!(caught_up.backlogged_subscription_percent(), 0);
    }

    #[test]
    fn subscription_snapshots_classify_backlog_status() {
        let mut bus = RuntimeEventBus::new();
        let all_events = RuntimeSubscriptionId::trusted("all-events");
        let bridge_events = RuntimeSubscriptionId::trusted("bridge-events");
        bus.subscribe(all_events.clone(), RuntimeEventFilter::All)
            .unwrap();
        bus.subscribe(
            bridge_events.clone(),
            RuntimeEventFilter::Bridge(BridgeId::trusted("bridge-1")),
        )
        .unwrap();

        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        bus.drain(&bridge_events).unwrap();

        let snapshots = bus.query_subscriptions(
            &RuntimeSubscriptionQuery::new().sorted_by(RuntimeSubscriptionSort::SubscriptionId),
        );

        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].subscription_id, all_events);
        assert_eq!(
            snapshots[0].backlog_status(),
            RuntimeSubscriptionBacklogStatus::Backlogged
        );
        assert!(snapshots[0].has_backlog());
        assert!(!snapshots[0].is_caught_up());
        assert!(snapshots[0].exceeds_backlog_threshold(0));
        assert_eq!(snapshots[1].subscription_id, bridge_events);
        assert_eq!(
            snapshots[1].backlog_status(),
            RuntimeSubscriptionBacklogStatus::CaughtUp
        );
        assert!(!snapshots[1].has_backlog());
        assert!(snapshots[1].is_caught_up());
        assert!(!snapshots[1].exceeds_backlog_threshold(0));
    }

    #[test]
    fn subscription_inventory_summary_counts_filters_and_backlog_pressure() {
        let mut bus = RuntimeEventBus::new();
        let all_events = RuntimeSubscriptionId::trusted("all-events");
        let bridge_events = RuntimeSubscriptionId::trusted("bridge-events");
        let command_events = RuntimeSubscriptionId::trusted("command-events");
        let supervision_events = RuntimeSubscriptionId::trusted("supervision-events");
        bus.subscribe(all_events.clone(), RuntimeEventFilter::All)
            .unwrap();
        bus.subscribe(
            bridge_events.clone(),
            RuntimeEventFilter::Bridge(BridgeId::trusted("bridge-1")),
        )
        .unwrap();
        bus.subscribe(command_events.clone(), RuntimeEventFilter::Commands)
            .unwrap();
        bus.subscribe(supervision_events.clone(), RuntimeEventFilter::Supervision)
            .unwrap();

        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        bus.publish(command_result_runtime_event("cmd-1"));
        bus.publish(RuntimeEvent::DesiredStateDrift {
            bridge_id: BridgeId::trusted("bridge-1"),
            entity_id: EntityId::trusted("entity-1"),
            capability_id: CapabilityId::trusted("light.on_off"),
            reason: ReconciliationReason::Drifted,
            detected_at_ms: 1_004,
        });
        bus.drain(&bridge_events).unwrap();

        let summary = bus.subscription_inventory_summary(
            &RuntimeSubscriptionQuery::new().sorted_by(RuntimeSubscriptionSort::SubscriptionId),
        );

        assert_eq!(
            summary,
            RuntimeSubscriptionInventorySummary {
                total_subscriptions: 4,
                all_event_subscriptions: 1,
                bridge_subscriptions: 1,
                entity_subscriptions: 0,
                command_subscriptions: 1,
                supervision_subscriptions: 1,
                backlogged_subscriptions: 3,
                caught_up_subscriptions: 1,
                total_queued_events: 5,
                max_queued_events: 3,
            }
        );
        assert!(!summary.is_empty());
        assert!(summary.has_backlog());
        assert!(summary.has_command_subscribers());
        assert!(summary.has_supervision_subscribers());
        assert_eq!(summary.average_queued_events_per_subscription(), 1);
        assert_eq!(summary.backlogged_subscription_percent(), 75);
        assert!(summary.exceeds_subscription_backlog_threshold(2));
        assert!(!summary.exceeds_subscription_backlog_threshold(3));

        let backlogged = bus.subscription_inventory_summary(
            &RuntimeSubscriptionQuery::new().with_min_queued_events(1),
        );
        assert_eq!(backlogged.total_subscriptions, 3);
        assert_eq!(backlogged.caught_up_subscriptions, 0);
        assert_eq!(backlogged.total_queued_events, 5);

        let commands = bus.subscription_inventory_summary(
            &RuntimeSubscriptionQuery::new().matching(RuntimeEventFilter::Commands),
        );
        assert_eq!(commands.total_subscriptions, 1);
        assert_eq!(commands.command_subscriptions, 1);
        assert_eq!(commands.total_queued_events, 1);

        let empty =
            bus.subscription_inventory_summary(&RuntimeSubscriptionQuery::new().with_limit(0));
        assert_eq!(empty, RuntimeSubscriptionInventorySummary::empty());
        assert!(!empty.has_backlog());
        assert_eq!(empty.backlogged_subscription_percent(), 0);
    }

    #[test]
    fn event_bus_health_summary_composes_replay_streams_and_pressure() {
        let mut bus = RuntimeEventBus::new();
        let all_events = RuntimeSubscriptionId::trusted("all-events");
        let command_events = RuntimeSubscriptionId::trusted("command-events");
        let supervision_events = RuntimeSubscriptionId::trusted("supervision-events");
        bus.subscribe(all_events, RuntimeEventFilter::All).unwrap();
        bus.subscribe(command_events, RuntimeEventFilter::Commands)
            .unwrap();
        bus.subscribe(supervision_events.clone(), RuntimeEventFilter::Supervision)
            .unwrap();

        bus.publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        bus.publish(command_result_runtime_event("cmd-1"));
        bus.publish(RuntimeEvent::DesiredStateDrift {
            bridge_id: BridgeId::trusted("bridge-1"),
            entity_id: EntityId::trusted("entity-1"),
            capability_id: CapabilityId::trusted("light.on_off"),
            reason: ReconciliationReason::Drifted,
            detected_at_ms: 1_004,
        });
        bus.drain(&supervision_events).unwrap();

        let summary = bus.health_summary();

        assert_eq!(
            summary.snapshot,
            RuntimeEventBusSnapshot {
                subscription_count: 3,
                pending_delivery_count: 4,
                published_event_count: 3,
                backlogged_subscription_count: 2,
                max_pending_delivery_count: 3,
            }
        );
        assert_eq!(
            summary.subscriptions,
            RuntimeSubscriptionInventorySummary {
                total_subscriptions: 3,
                all_event_subscriptions: 1,
                bridge_subscriptions: 0,
                entity_subscriptions: 0,
                command_subscriptions: 1,
                supervision_subscriptions: 1,
                backlogged_subscriptions: 2,
                caught_up_subscriptions: 1,
                total_queued_events: 4,
                max_queued_events: 3,
            }
        );
        assert_eq!(
            summary.event_log,
            RuntimeEventLogSummary {
                total_events: 3,
                device_events: 0,
                command_results: 1,
                bridge_health_events: 1,
                state_expired_events: 0,
                desired_state_drift_events: 1,
                worker_restart_events: 0,
                first_sequence: Some(0),
                latest_sequence: Some(2),
                next_checkpoint: RuntimeEventCheckpoint::from_next_sequence(3),
            }
        );
        assert!(summary.has_stream_coverage());
        assert!(summary.has_replay_history());
        assert!(summary.has_event_pressure());
        assert!(!summary.is_caught_up());
        assert!(summary.needs_attention());
        assert!(summary.has_command_streams());
        assert!(summary.has_supervision_streams());

        let mut runtime = SmartHomeRuntime::new();
        runtime
            .event_bus_mut()
            .subscribe(
                RuntimeSubscriptionId::trusted("runtime-all"),
                RuntimeEventFilter::All,
            )
            .unwrap();
        runtime
            .event_bus_mut()
            .publish(bridge_health_runtime_event("health-2", "bridge-1", 2_000));

        let runtime_summary = runtime.event_bus_health_summary();

        assert_eq!(runtime_summary.snapshot.subscription_count, 1);
        assert_eq!(runtime_summary.snapshot.pending_delivery_count, 1);
        assert_eq!(runtime_summary.event_log.total_events, 1);
        assert!(runtime_summary.has_event_pressure());
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
    fn authorized_commands_require_active_agent_grants() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:lighting-planner");
        let authorization = CommandAuthorization::new(principal.clone(), Vec::new());

        let error = runtime
            .submit_authorized_command(
                &authorization,
                command(CommandType::TurnOn, Value::Null),
                1_000,
            )
            .unwrap_err();

        assert!(matches!(
            error,
            RuntimeError::UnauthorizedCommand {
                principal_id,
                missing_capabilities,
                ..
            } if principal_id == principal
                && missing_capabilities == vec![CapabilityId::trusted("light.on_off")]
        ));
        assert!(runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .is_none());
        assert!(runtime.event_bus().published().is_empty());
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);
        assert_eq!(runtime.registry().counts().authorization_decisions, 1);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Denied);
        assert_eq!(
            decisions[0].missing_capabilities,
            vec![CapabilityId::trusted("light.on_off")]
        );
    }

    #[test]
    fn authorized_commands_accept_matching_entity_capability_grants() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:lighting-planner");
        let grant = CapabilityGrant::for_entity_capability(
            CapabilityGrantId::trusted("grant-1"),
            principal.clone(),
            EntityId::trusted("entity-1"),
            CapabilityId::trusted("light.on_off"),
            PrivilegeTier::LowRisk,
            "chief-of-staff",
            900,
        )
        .with_expiry(2_000);
        let authorization = CommandAuthorization::new(principal, vec![grant]);
        let turn_on = command(CommandType::TurnOn, Value::Null);

        assert!(authorization.allows_command_at(&turn_on, 1_000));
        let result = runtime
            .submit_authorized_command(&authorization, turn_on, 1_000)
            .unwrap();
        let snapshot_confidence = runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .unwrap()
            .confidence;
        let rejected = runtime
            .submit_authorized_command(
                &authorization,
                command(CommandType::TurnOff, Value::Null),
                2_000,
            )
            .unwrap_err();

        assert_eq!(result.status, CommandStatus::Accepted);
        assert_eq!(snapshot_confidence, StateConfidence::Optimistic);
        assert!(matches!(
            rejected,
            RuntimeError::UnauthorizedCommand {
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("light.on_off")]
        ));
        assert_eq!(runtime.event_bus().published().len(), 1);
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&AgentId::trusted("agent:lighting-planner"));
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Allowed);
        assert_eq!(decisions[1].outcome, AuthorizationOutcome::Denied);
    }

    #[test]
    fn tool_authorization_records_decisions_without_dispatching_commands() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:lighting-planner");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );

        let allowed =
            runtime.authorize_tool_for_principal(principal.clone(), SmartHomeTool::GetState, 1_500);
        let denied =
            runtime.authorize_tool_for_principal(principal.clone(), SmartHomeTool::Command, 1_500);
        let expired =
            runtime.authorize_tool_for_principal(principal.clone(), SmartHomeTool::GetState, 2_000);
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert_eq!(allowed.outcome, AuthorizationOutcome::Allowed);
        assert_eq!(denied.outcome, AuthorizationOutcome::Denied);
        assert_eq!(expired.outcome, AuthorizationOutcome::Denied);
        assert_eq!(runtime.registry().counts().authorization_decisions, 3);
        assert_eq!(decisions.len(), 3);
        assert!(runtime.event_bus().published().is_empty());
        assert_eq!(
            denied.missing_capabilities,
            vec![CapabilityId::trusted("smart_home.command.light")]
        );
        assert_eq!(
            expired.missing_capabilities,
            vec![CapabilityId::trusted("smart_home.read")]
        );
    }

    #[test]
    fn authorization_read_tools_filter_denied_decisions() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:lighting-planner");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );
        let denied =
            runtime.authorize_tool_for_principal(principal.clone(), SmartHomeTool::Command, 1_250);

        let decisions = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListAuthorizationDecisions {
                    query: RuntimeAuthorizationDecisionQuery::new()
                        .for_principal(principal.clone())
                        .with_outcome(AuthorizationOutcome::Denied),
                },
                1_500,
            )
            .unwrap();
        let summary = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::GetAuthorizationSummary {
                    query: RuntimeAuthorizationDecisionQuery::new()
                        .for_principal(principal)
                        .with_outcome(AuthorizationOutcome::Denied),
                },
                1_501,
            )
            .unwrap();

        assert_eq!(denied.outcome, AuthorizationOutcome::Denied);
        assert!(matches!(
            decisions,
            RuntimeReadToolOutput::AuthorizationDecisions { decisions, summary }
                if decisions.len() == 1
                    && decisions[0].subject == AuthorizationSubject::Tool(SmartHomeTool::Command)
                    && decisions[0].missing_capabilities
                        == vec![CapabilityId::trusted("smart_home.command.light")]
                    && summary.total_decisions == 1
                    && summary.denied_decisions == 1
                    && summary.allowed_decisions == 0
        ));
        assert!(matches!(
            summary,
            RuntimeReadToolOutput::AuthorizationSummary { summary }
                if summary.total_decisions == 1
                    && summary.denied_decisions == 1
                    && summary.decisions_with_missing_capabilities == 1
        ));
        assert_eq!(runtime.registry().counts().authorization_decisions, 3);
    }

    #[test]
    fn capability_grant_read_tools_filter_effective_status_and_scope() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:lighting-planner");
        let installer = AgentId::trusted("agent:installer");
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            ));
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("grant-light-command"),
                principal.clone(),
                EntityId::trusted("entity-light-1"),
                CapabilityId::trusted("light.on_off"),
                PrivilegeTier::LowRisk,
                "chief-of-staff",
                1_100,
            )
            .with_expiry(2_000),
        );
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-installer"),
                installer,
                PrivilegeTier::HumanApproval,
                "chief-of-staff",
                1_200,
            )
            .with_status(CapabilityGrantStatus::Pending),
        );

        let grants = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListCapabilityGrants {
                    query: RuntimeCapabilityGrantQuery::new()
                        .for_principal(principal.clone())
                        .with_status(CapabilityGrantStatus::Expired)
                        .with_scope_kind(RuntimeCapabilityGrantScopeKind::EntityCapability)
                        .with_capability(CapabilityId::trusted("light.on_off"))
                        .for_entity(EntityId::trusted("entity-light-1"))
                        .sorted_by(RuntimeCapabilityGrantSort::GrantedAtDesc),
                },
                2_500,
            )
            .unwrap();
        let summary = runtime
            .execute_read_tool(
                principal,
                RuntimeReadToolRequest::GetCapabilityGrantSummary {
                    query: RuntimeCapabilityGrantQuery::new()
                        .with_status(CapabilityGrantStatus::Pending)
                        .with_scope_kind(RuntimeCapabilityGrantScopeKind::AllSmartHome),
                },
                2_501,
            )
            .unwrap();

        assert!(matches!(
            grants,
            RuntimeReadToolOutput::CapabilityGrants { grants, summary }
                if grants.len() == 1
                    && grants[0].grant_id == CapabilityGrantId::trusted("grant-light-command")
                    && summary.total_grants == 1
                    && summary.expired_grants == 1
                    && summary.entity_capability_grants == 1
                    && summary.unique_principals == 1
        ));
        assert!(matches!(
            summary,
            RuntimeReadToolOutput::CapabilityGrantSummary { summary }
                if summary.total_grants == 1
                    && summary.pending_grants == 1
                    && summary.all_smart_home_grants == 1
                    && summary.human_approval_tier_grants == 1
                    && summary.needs_review()
        ));
        assert_eq!(runtime.registry().counts().authorization_decisions, 2);
    }

    #[test]
    fn read_tools_require_smart_home_read_grants() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:observer");

        let error = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListBridges,
                1_000,
            )
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::ListBridges,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Denied);
        assert!(runtime.event_bus().published().is_empty());
    }

    #[test]
    fn discover_tool_requires_smart_home_read_grants() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:observer");
        runtime
            .record_discovery(hue_discovery_record("001788fffeabcdef", 1_000))
            .unwrap();

        let error = runtime
            .execute_discover_tool(principal.clone(), RuntimeDiscoverToolRequest::new(), 1_500)
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::Discover,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Denied);
        assert_eq!(runtime.discovery_record_count(), 1);
    }

    #[test]
    fn discovery_records_reconcile_unpaired_bridge_candidates_for_discover_tool() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:observer");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );
        let record = hue_discovery_record("001788fffeabcdef", 1_100);
        let bridge_id = record.bridge_id();

        let upsert = runtime.record_discovery(record.clone()).unwrap();
        let bridge = runtime.registry().bridge(&bridge_id).unwrap().clone();
        let output = runtime
            .execute_discover_tool(
                principal.clone(),
                RuntimeDiscoverToolRequest::new()
                    .for_integration(IntegrationId::trusted("hue"))
                    .from_source(DiscoverySource::Mdns)
                    .with_ttl_ms(1_000),
                1_500,
            )
            .unwrap();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert_eq!(upsert, DiscoveryUpsert::Inserted);
        assert_eq!(runtime.discovery_record_count(), 1);
        assert_eq!(bridge.bridge_id, bridge_id);
        assert_eq!(bridge.health, Health::Unpaired);
        assert_eq!(bridge.address.as_deref(), Some("https://192.0.2.10"));
        assert_eq!(bridge.auth_ref, None);
        assert!(bridge.metadata.iter().any(|metadata| {
            metadata.key == "smart_home.discovery.source" && metadata.value == "mdns"
        }));
        assert_eq!(output.len(), 1);
        assert_eq!(output.generated_at_ms, 1_500);
        assert_eq!(output.ttl_ms, 1_000);
        assert_eq!(output.records[0], record);
        assert_eq!(output.bridge_candidates[0].bridge_id, bridge_id);
        assert_eq!(output.record_summary.total, 1);
        assert_eq!(output.record_summary.with_address, 1);
        assert_eq!(output.record_summary.fresh, 1);
        assert_eq!(
            output
                .record_summary
                .count_for_source(DiscoverySource::Mdns),
            1
        );
        assert_eq!(
            output
                .record_summary
                .count_for_pairing_requirement(PairingRequirement::PhysicalPresence),
            1
        );
        assert_eq!(output.signal_summary.fresh, 1);
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Allowed);
    }

    #[test]
    fn discover_tool_filters_stale_records_and_limits_results() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:observer");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(3_000),
        );
        runtime
            .record_discovery(hue_discovery_record("001788fffeold", 1_000))
            .unwrap();
        runtime
            .record_discovery(hue_discovery_record("001788fffefresh", 1_900))
            .unwrap();

        let output = runtime
            .execute_discover_tool(
                principal,
                RuntimeDiscoverToolRequest::new()
                    .fresh_only(true)
                    .with_ttl_ms(500)
                    .with_limit(1),
                2_000,
            )
            .unwrap();

        assert_eq!(output.len(), 1);
        assert_eq!(output.records[0].native_bridge_id, "001788fffefresh");
        assert_eq!(output.record_summary.total, 1);
        assert_eq!(output.record_summary.fresh, 1);
        assert_eq!(output.record_summary.stale, 0);
        assert_eq!(output.bridge_candidates[0].health, Health::Unpaired);
    }

    #[test]
    fn runtime_ingests_discovery_worker_runs_as_preferred_bridge_candidates() {
        let mut runtime = SmartHomeRuntime::new();
        let replace_bridge_id = BridgeId::trusted("hue.bridge.001788fffereplace");
        let ignored_bridge_id = BridgeId::trusted("hue.bridge.001788fffeignored");
        runtime
            .record_discovery(hue_cloud_discovery_record("001788fffereplace", 1_000))
            .unwrap();
        runtime
            .record_discovery(hue_discovery_record("001788fffeignored", 1_900))
            .unwrap();

        let mut run = DiscoveryWorkerRun::new(
            DiscoveryWorkerId::trusted("hue-composite-worker"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::Composite,
            1_800,
            1_950,
        );
        run.push_record(hue_discovery_record("001788fffereplace", 1_900))
            .unwrap();
        run.push_record(hue_cloud_discovery_record("001788fffeignored", 1_950))
            .unwrap();
        run.push_failure(
            DiscoveryWorkerFailure::new(DiscoverySource::Mdns, "ignored malformed packet").unwrap(),
        );

        let summary = runtime
            .record_discovery_worker_run(&run, 2_000, 500)
            .unwrap();

        assert_eq!(summary.status, DiscoveryWorkerRunStatus::Partial);
        assert_eq!(summary.record_count, 2);
        assert_eq!(summary.failure_count, 1);
        assert_eq!(summary.inserted_count, 0);
        assert_eq!(summary.replaced_count, 1);
        assert_eq!(summary.ignored_count, 1);
        assert_eq!(summary.accepted_count(), 1);
        assert!(summary.has_catalog_changes());
        assert_eq!(summary.record_summary.total, 2);
        assert_eq!(summary.signal_summary.fresh, 2);
        assert_eq!(runtime.discovery_record_count(), 2);
        assert_eq!(
            runtime
                .discovery()
                .get(&IntegrationId::trusted("hue"), "001788fffereplace")
                .unwrap()
                .source,
            DiscoverySource::Mdns
        );
        assert_eq!(
            runtime
                .registry()
                .bridge(&replace_bridge_id)
                .unwrap()
                .address
                .as_deref(),
            Some("https://192.0.2.10")
        );
        assert_eq!(
            runtime
                .registry()
                .bridge(&ignored_bridge_id)
                .unwrap()
                .address
                .as_deref(),
            Some("https://192.0.2.10")
        );
    }

    #[test]
    fn runtime_supervision_plan_previews_due_discovery_workers_without_mutating() {
        let mut runtime = SmartHomeRuntime::new();
        let worker_id = DiscoveryWorkerId::trusted("hue-mdns-worker");
        runtime
            .register_discovery_worker_schedule(hue_mdns_discovery_worker(1_100))
            .unwrap();

        let early = runtime.supervision_plan_at(1_099).unwrap();
        assert!(early.discovery_worker_run_plan.is_empty());

        let plan = runtime.supervision_plan_at(1_125).unwrap();
        let observation = runtime.observe_supervision_at(1_125).unwrap();
        let summary = plan.summary();
        let scheduled = runtime.discovery_worker_schedule(&worker_id).unwrap();
        let snapshot = runtime.read_snapshot_at(1_125);
        let pending = snapshot.pending_work_summary();
        let queried = runtime.query_discovery_worker_schedules(
            &DiscoveryWorkerQuery::new()
                .for_integration(IntegrationId::trusted("hue"))
                .with_source(DiscoverySource::Mdns)
                .due_before(1_125)
                .sorted_by(DiscoveryWorkerSort::NextDueAt),
        );

        assert_eq!(plan.action_count(), 1);
        assert_eq!(plan.discovery_worker_run_plan.len(), 1);
        assert_eq!(
            plan.discovery_worker_run_plan
                .instructions_for_worker(&worker_id)
                .len(),
            1
        );
        assert!(matches!(
            plan.discovery_worker_run_plan.instructions.as_slice(),
            [instruction] if instruction.worker_id == worker_id
                && instruction.integration_id == IntegrationId::trusted("hue")
                && instruction.kind == DiscoveryWorkerKind::MdnsScan
                && instruction.sources == vec![DiscoverySource::Mdns]
                && instruction.network_interfaces == vec!["en0".to_string(), "bridge0".to_string()]
                && instruction.due_at_ms == 1_100
                && instruction.planned_at_ms == 1_125
                && instruction.overdue_by_ms() == 25
                && instruction.run_timeout_ms == 250
        ));
        let mdns_scan_plan = plan.discovery_worker_run_plan.mdns_scan_plan().unwrap();
        let runtime_mdns_scan_plan = runtime.discovery_mdns_scan_plan_at(1_125).unwrap();
        assert_eq!(mdns_scan_plan, runtime_mdns_scan_plan);
        assert_eq!(mdns_scan_plan.generated_at_ms, 1_125);
        assert_eq!(mdns_scan_plan.len(), 4);
        assert!(matches!(
            mdns_scan_plan.requests.as_slice(),
            [en0_ipv4, en0_ipv6, bridge0_ipv4, bridge0_ipv6]
                if en0_ipv4.worker_id == worker_id
                    && en0_ipv4.network_interface == "en0"
                    && en0_ipv4.network == MdnsScanNetwork::Ipv4
                    && en0_ipv4.service_type == "_hue._tcp.local"
                    && en0_ipv4.discovered_at_ms == 1_125
                    && en0_ipv4.timeout == Duration::from_millis(250)
                    && en0_ipv6.network_interface == "en0"
                    && en0_ipv6.network == MdnsScanNetwork::Ipv6
                    && bridge0_ipv4.network_interface == "bridge0"
                    && bridge0_ipv4.network == MdnsScanNetwork::Ipv4
                    && bridge0_ipv6.network_interface == "bridge0"
                    && bridge0_ipv6.network == MdnsScanNetwork::Ipv6
        ));
        assert_eq!(summary.total_actions, 1);
        assert_eq!(summary.discovery_worker_run_count, 1);
        assert!(summary.has_discovery_worker_work());
        assert_eq!(observation.discovery_worker_run_count(), 1);
        assert_eq!(observation.discovery_worker_count(), 1);
        assert_eq!(observation.unhealthy_discovery_worker_count(), 0);
        assert_eq!(observation.discovery_workers_with_failures_count(), 0);
        assert_eq!(observation.next_discovery_worker_due_at_ms(), Some(1_100));
        assert!(matches!(
            observation.discovery_workers.as_slice(),
            [snapshot] if snapshot.worker_id == worker_id
                && snapshot.integration_id == IntegrationId::trusted("hue")
                && snapshot.kind == DiscoveryWorkerKind::MdnsScan
                && snapshot.status == WorkerStatus::Starting
                && snapshot.is_due
                && snapshot.overdue_by_ms == 25
                && snapshot.next_due_at_ms == 1_100
                && snapshot.last_run_status.is_none()
                && snapshot.total_run_count == 0
                && !snapshot.has_failure_pressure()
        ));
        assert_eq!(scheduled.status, WorkerStatus::Starting);
        assert_eq!(scheduled.total_run_count, 0);
        assert_eq!(snapshot.discovery_scheduler.worker_count, 1);
        assert_eq!(snapshot.discovery_scheduler.due_worker_count, 1);
        assert_eq!(pending.discovery_worker_due_count, 1);
        assert!(pending.has_supervision_pressure());
        assert_eq!(queried.len(), 1);
        assert_eq!(queried[0].worker_id, worker_id);
        assert!(runtime.event_bus().published().is_empty());
    }

    #[test]
    fn runtime_records_scheduled_discovery_worker_runs_and_advances_schedule() {
        let mut runtime = SmartHomeRuntime::new();
        let worker_id = DiscoveryWorkerId::trusted("hue-mdns-worker");
        runtime
            .register_discovery_worker_schedule(hue_mdns_discovery_worker(1_100))
            .unwrap();
        runtime
            .mark_discovery_worker_started(&worker_id, 1_100)
            .unwrap();

        let mut run = DiscoveryWorkerRun::new(
            worker_id.clone(),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            1_100,
            1_180,
        );
        run.push_record(hue_discovery_record("001788fffescheduled", 1_175))
            .unwrap();

        let summary = runtime
            .record_scheduled_discovery_worker_run(&run, 1_200, 500)
            .unwrap();
        let scheduled = runtime.discovery_worker_schedule(&worker_id).unwrap();

        assert_eq!(summary.status, DiscoveryWorkerRunStatus::Completed);
        assert_eq!(summary.inserted_count, 1);
        assert_eq!(scheduled.status, WorkerStatus::Running);
        assert_eq!(
            scheduled.last_run_status,
            Some(DiscoveryWorkerRunStatus::Completed)
        );
        assert_eq!(scheduled.last_started_at_ms, Some(1_100));
        assert_eq!(scheduled.last_completed_at_ms, Some(1_180));
        assert_eq!(scheduled.last_record_count, 1);
        assert_eq!(scheduled.last_failure_count, 0);
        assert_eq!(scheduled.last_catalog_change_count, 1);
        assert_eq!(scheduled.total_run_count, 1);
        assert_eq!(scheduled.consecutive_failure_count, 0);
        assert_eq!(scheduled.next_due_at_ms, 6_180);
        assert_eq!(runtime.discovery_record_count(), 1);
        assert_eq!(runtime.discovery_worker_run_plan_at(6_179).len(), 0);
        assert_eq!(runtime.discovery_worker_run_plan_at(6_180).len(), 1);

        let mut failed_run = DiscoveryWorkerRun::new(
            worker_id.clone(),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            6_180,
            6_220,
        );
        failed_run.push_failure(
            DiscoveryWorkerFailure::new(DiscoverySource::Mdns, "timed out waiting for replies")
                .unwrap(),
        );

        let failed_summary = runtime
            .record_scheduled_discovery_worker_run(&failed_run, 6_250, 500)
            .unwrap();
        let scheduled = runtime.discovery_worker_schedule(&worker_id).unwrap();

        assert_eq!(failed_summary.status, DiscoveryWorkerRunStatus::Failed);
        assert_eq!(scheduled.status, WorkerStatus::Unhealthy);
        assert_eq!(
            scheduled.last_run_status,
            Some(DiscoveryWorkerRunStatus::Failed)
        );
        assert_eq!(scheduled.last_record_count, 0);
        assert_eq!(scheduled.last_failure_count, 1);
        assert_eq!(scheduled.last_catalog_change_count, 0);
        assert_eq!(scheduled.total_run_count, 2);
        assert_eq!(scheduled.consecutive_failure_count, 1);
        assert_eq!(scheduled.next_due_at_ms, 11_220);
        assert_eq!(
            runtime
                .query_discovery_worker_schedules(
                    &DiscoveryWorkerQuery::new()
                        .with_status(WorkerStatus::Unhealthy)
                        .min_consecutive_failure_count(1),
                )
                .len(),
            1
        );
    }

    #[test]
    fn runtime_applies_discovery_worker_retry_backoff_after_failures() {
        let mut runtime = SmartHomeRuntime::new();
        let worker_id = DiscoveryWorkerId::trusted("hue-mdns-worker");
        runtime
            .register_discovery_worker_schedule(
                hue_mdns_discovery_worker(1_000).with_retry_backoff(500, 2_000, 2),
            )
            .unwrap();

        runtime
            .mark_discovery_worker_started(&worker_id, 1_000)
            .unwrap();
        let mut first_failure = DiscoveryWorkerRun::new(
            worker_id.clone(),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            1_000,
            1_100,
        );
        first_failure.push_failure(
            DiscoveryWorkerFailure::new(DiscoverySource::Mdns, "no replies before timeout")
                .unwrap(),
        );
        runtime
            .record_scheduled_discovery_worker_run(&first_failure, 1_100, 500)
            .unwrap();

        let scheduled = runtime.discovery_worker_schedule(&worker_id).unwrap();
        assert_eq!(scheduled.consecutive_failure_count, 1);
        assert_eq!(scheduled.retry_delay_for_failure_count(1), 500);
        assert_eq!(scheduled.next_due_at_ms, 1_600);

        runtime
            .mark_discovery_worker_started(&worker_id, 1_600)
            .unwrap();
        let mut second_failure = DiscoveryWorkerRun::new(
            worker_id.clone(),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            1_600,
            1_700,
        );
        second_failure.push_failure(
            DiscoveryWorkerFailure::new(DiscoverySource::Mdns, "multicast route unavailable")
                .unwrap(),
        );
        runtime
            .record_scheduled_discovery_worker_run(&second_failure, 1_700, 500)
            .unwrap();

        let scheduled = runtime.discovery_worker_schedule(&worker_id).unwrap();
        assert_eq!(scheduled.consecutive_failure_count, 2);
        assert_eq!(scheduled.retry_delay_for_failure_count(2), 1_000);
        assert_eq!(scheduled.next_due_at_ms, 2_700);

        runtime
            .mark_discovery_worker_started(&worker_id, 2_700)
            .unwrap();
        let mut third_failure = DiscoveryWorkerRun::new(
            worker_id.clone(),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            2_700,
            2_800,
        );
        third_failure.push_failure(
            DiscoveryWorkerFailure::new(DiscoverySource::Mdns, "network still unavailable")
                .unwrap(),
        );
        runtime
            .record_scheduled_discovery_worker_run(&third_failure, 2_800, 500)
            .unwrap();

        let observation = runtime.observe_supervision_at(2_801).unwrap();
        assert_eq!(observation.discovery_workers_with_failures_count(), 1);
        assert_eq!(observation.next_discovery_worker_due_at_ms(), Some(4_800));
        assert!(matches!(
            observation.discovery_workers.as_slice(),
            [snapshot] if snapshot.worker_id == worker_id
                && snapshot.status == WorkerStatus::Unhealthy
                && snapshot.retry_delay_ms == 500
                && snapshot.max_retry_delay_ms == 2_000
                && snapshot.retry_backoff_multiplier == 2
                && snapshot.current_retry_delay_ms == Some(2_000)
                && snapshot.next_due_at_ms == 4_800
                && snapshot.consecutive_failure_count == 3
                && snapshot.has_failure_pressure()
        ));

        runtime
            .mark_discovery_worker_started(&worker_id, 4_800)
            .unwrap();
        let mut recovery_run = DiscoveryWorkerRun::new(
            worker_id.clone(),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            4_800,
            4_900,
        );
        recovery_run
            .push_record(hue_discovery_record("001788fffebackoff", 4_850))
            .unwrap();
        runtime
            .record_scheduled_discovery_worker_run(&recovery_run, 4_900, 500)
            .unwrap();

        let scheduled = runtime.discovery_worker_schedule(&worker_id).unwrap();
        assert_eq!(scheduled.status, WorkerStatus::Running);
        assert_eq!(scheduled.consecutive_failure_count, 0);
        assert_eq!(scheduled.next_due_at_ms, 9_900);
        assert_eq!(scheduled.retry_delay_for_failure_count(0), 0);
        assert_eq!(
            runtime
                .observe_supervision_at(4_901)
                .unwrap()
                .discovery_workers[0]
                .current_retry_delay_ms,
            None
        );
    }

    #[test]
    fn runtime_supervised_mdns_discovery_run_executes_and_records_due_workers() {
        let mut runtime = SmartHomeRuntime::new();
        let worker_id = DiscoveryWorkerId::trusted("hue-mdns-worker");
        runtime
            .register_discovery_worker_schedule(hue_mdns_discovery_worker(1_100))
            .unwrap();
        let outcomes = (0..4).map(|_| {
            MdnsScanResult::from_packets("_hue._tcp.local", 1_125, Vec::<MdnsResponsePacket>::new())
        });
        let mut executor = ScriptedMdnsExecutor::new(outcomes);
        let mut run = DiscoveryWorkerRun::new(
            worker_id.clone(),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            1_125,
            1_180,
        );
        run.push_record(hue_discovery_record("001788fffesupervised", 1_175))
            .unwrap();
        let mut adapter = ScriptedMdnsRunAdapter::new([Ok(run)]);

        let report = runtime
            .run_due_mdns_discovery_workers_with_executor(
                1_125,
                1_180,
                500,
                &mut executor,
                &mut adapter,
            )
            .unwrap();
        let scheduled = runtime.discovery_worker_schedule(&worker_id).unwrap();

        assert_eq!(executor.requests.len(), 4);
        assert_eq!(adapter.reports.len(), 1);
        assert_eq!(adapter.reports[0].completed_scan_count(), 4);
        assert_eq!(report.planned_instruction_count, 1);
        assert_eq!(report.mdns_request_count, 4);
        assert_eq!(report.mdns_report_count, 1);
        assert_eq!(report.recorded_run_count(), 1);
        assert_eq!(report.completed_run_count(), 1);
        assert_eq!(report.partial_run_count(), 0);
        assert_eq!(report.failed_run_count(), 0);
        assert_eq!(report.catalog_change_count(), 1);
        assert!(!report.has_failures());
        assert_eq!(runtime.discovery_record_count(), 1);
        assert_eq!(scheduled.status, WorkerStatus::Running);
        assert_eq!(scheduled.last_started_at_ms, Some(1_125));
        assert_eq!(scheduled.last_completed_at_ms, Some(1_180));
        assert_eq!(
            scheduled.last_run_status,
            Some(DiscoveryWorkerRunStatus::Completed)
        );
        assert_eq!(scheduled.next_due_at_ms, 6_180);
    }

    #[test]
    fn runtime_supervised_mdns_discovery_run_records_adapter_failures() {
        let mut runtime = SmartHomeRuntime::new();
        let worker_id = DiscoveryWorkerId::trusted("hue-mdns-worker");
        runtime
            .register_discovery_worker_schedule(hue_mdns_discovery_worker(1_100))
            .unwrap();
        let outcomes = (0..4).map(|_| {
            MdnsScanResult::from_packets("_hue._tcp.local", 1_125, Vec::<MdnsResponsePacket>::new())
        });
        let mut executor = ScriptedMdnsExecutor::new(outcomes);
        let mut adapter =
            ScriptedMdnsRunAdapter::new([Err("unsupported mDNS service type".to_string())]);

        let report = runtime
            .run_due_mdns_discovery_workers_with_executor(
                1_125,
                1_180,
                500,
                &mut executor,
                &mut adapter,
            )
            .unwrap();
        let scheduled = runtime.discovery_worker_schedule(&worker_id).unwrap();

        assert_eq!(executor.requests.len(), 4);
        assert_eq!(adapter.reports.len(), 1);
        assert_eq!(report.planned_instruction_count, 1);
        assert_eq!(report.mdns_request_count, 4);
        assert_eq!(report.mdns_report_count, 1);
        assert_eq!(report.recorded_run_count(), 1);
        assert_eq!(report.completed_run_count(), 0);
        assert_eq!(report.failed_run_count(), 1);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].worker_id, worker_id);
        assert_eq!(report.failures[0].message, "unsupported mDNS service type");
        assert!(report.has_failures());
        assert_eq!(runtime.discovery_record_count(), 0);
        assert_eq!(scheduled.status, WorkerStatus::Unhealthy);
        assert_eq!(
            scheduled.last_run_status,
            Some(DiscoveryWorkerRunStatus::Failed)
        );
        assert_eq!(scheduled.last_record_count, 0);
        assert_eq!(scheduled.last_failure_count, 1);
        assert_eq!(scheduled.consecutive_failure_count, 1);
        assert_eq!(scheduled.next_due_at_ms, 6_180);

        let observation = runtime.observe_supervision_at(1_181).unwrap();
        assert_eq!(observation.discovery_worker_count(), 1);
        assert_eq!(observation.discovery_worker_run_count(), 0);
        assert_eq!(observation.unhealthy_discovery_worker_count(), 1);
        assert_eq!(observation.discovery_workers_with_failures_count(), 1);
        assert_eq!(observation.next_discovery_worker_due_at_ms(), Some(6_180));
        assert!(matches!(
            observation.discovery_workers.as_slice(),
            [snapshot] if snapshot.worker_id == worker_id
                && snapshot.status == WorkerStatus::Unhealthy
                && !snapshot.is_due
                && snapshot.last_run_status == Some(DiscoveryWorkerRunStatus::Failed)
                && snapshot.last_record_count == 0
                && snapshot.last_failure_count == 1
                && snapshot.total_run_count == 1
                && snapshot.consecutive_failure_count == 1
                && snapshot.has_failure_pressure()
        ));
    }

    #[test]
    fn scheduled_discovery_workers_validate_scope_and_run_identity() {
        let mut runtime = SmartHomeRuntime::new();
        let invalid = ScheduledDiscoveryWorker::new(
            DiscoveryWorkerId::trusted("hue-mdns-worker"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            5_000,
            250,
            1_100,
        )
        .with_source(DiscoverySource::Mdns);

        let error = runtime
            .register_discovery_worker_schedule(invalid)
            .unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::InvalidDiscoveryWorkerSchedule {
                field: "network_interfaces",
                ..
            }
        ));

        let missing_service_type = ScheduledDiscoveryWorker::new(
            DiscoveryWorkerId::trusted("hue-mdns-worker"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            5_000,
            250,
            1_100,
        )
        .with_source(DiscoverySource::Mdns)
        .with_network_interface("en0");
        assert!(matches!(
            runtime
                .register_discovery_worker_schedule(missing_service_type)
                .unwrap_err(),
            RuntimeError::InvalidDiscoveryWorkerSchedule {
                field: "metadata.smart_home.discovery.service_type",
                ..
            }
        ));

        let invalid_retry = hue_mdns_discovery_worker(1_100).with_retry_backoff(1_000, 500, 2);
        assert!(matches!(
            runtime.register_discovery_worker_schedule(invalid_retry),
            Err(RuntimeError::InvalidDiscoveryWorkerSchedule {
                field: "max_retry_delay_ms",
                ..
            })
        ));

        runtime
            .register_discovery_worker_schedule(hue_mdns_discovery_worker(1_100))
            .unwrap();

        let unknown_run = DiscoveryWorkerRun::new(
            DiscoveryWorkerId::trusted("unknown-worker"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::MdnsScan,
            1_100,
            1_125,
        );
        assert!(matches!(
            runtime.record_scheduled_discovery_worker_run(&unknown_run, 1_125, 500),
            Err(RuntimeError::UnknownDiscoveryWorker(_))
        ));

        let wrong_kind = DiscoveryWorkerRun::new(
            DiscoveryWorkerId::trusted("hue-mdns-worker"),
            IntegrationId::trusted("hue"),
            DiscoveryWorkerKind::CloudFallback,
            1_100,
            1_125,
        );
        assert!(matches!(
            runtime.record_scheduled_discovery_worker_run(&wrong_kind, 1_125, 500),
            Err(RuntimeError::DiscoveryWorkerRunMismatch { .. })
        ));
    }

    #[test]
    fn subscribe_tool_requires_smart_home_read_grants() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:observer");
        let subscription = RuntimeSubscriptionId::trusted("state-stream");

        let error = runtime
            .execute_subscribe_tool(
                principal.clone(),
                RuntimeSubscribeToolRequest::new(subscription.clone(), RuntimeEventFilter::All),
                1_000,
            )
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::Subscribe,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Denied);
        assert!(matches!(
            runtime.event_bus_mut().drain(&subscription),
            Err(RuntimeError::UnknownSubscription(_))
        ));

        let poll_error = runtime
            .execute_poll_events_tool(
                principal.clone(),
                RuntimePollEventsToolRequest::new(subscription.clone()),
                1_000,
            )
            .unwrap_err();
        assert!(matches!(
            poll_error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::PollEvents,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));

        let unsubscribe_error = runtime
            .execute_unsubscribe_tool(
                principal.clone(),
                RuntimeUnsubscribeToolRequest::new(subscription.clone()),
                1_000,
            )
            .unwrap_err();
        assert!(matches!(
            unsubscribe_error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::Unsubscribe,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
    }

    #[test]
    fn subscribe_tool_registers_checkpointed_runtime_subscriptions() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:observer");
        let subscription = RuntimeSubscriptionId::trusted("bridge-1-stream");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );
        runtime
            .event_bus_mut()
            .publish(bridge_health_runtime_event("health-1", "bridge-1", 1_000));
        runtime
            .event_bus_mut()
            .publish(bridge_health_runtime_event("health-2", "bridge-2", 1_001));

        let output = runtime
            .execute_subscribe_tool(
                principal.clone(),
                RuntimeSubscribeToolRequest::new(
                    subscription.clone(),
                    RuntimeEventFilter::Bridge(BridgeId::trusted("bridge-1")),
                )
                .with_checkpoint(RuntimeEventCheckpoint::start()),
                1_500,
            )
            .unwrap();
        let replay = runtime.event_bus_mut().drain(&subscription).unwrap();
        runtime
            .event_bus_mut()
            .publish(bridge_health_runtime_event("health-3", "bridge-1", 1_600));
        let live = runtime.event_bus_mut().drain(&subscription).unwrap();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert_eq!(output.subscription_id, subscription);
        assert_eq!(
            output.replay_from_checkpoint,
            RuntimeEventCheckpoint::start()
        );
        assert_eq!(output.subscribed_at_checkpoint.next_sequence(), 2);
        assert_eq!(output.queued_events, 1);
        assert!(matches!(
            replay.as_slice(),
            [RuntimeEvent::BridgeHealth { event_id, bridge_id, .. }]
                if event_id == &EventId::trusted("health-1")
                    && bridge_id == &BridgeId::trusted("bridge-1")
        ));
        assert!(matches!(
            live.as_slice(),
            [RuntimeEvent::BridgeHealth { event_id, bridge_id, .. }]
                if event_id == &EventId::trusted("health-3")
                    && bridge_id == &BridgeId::trusted("bridge-1")
        ));
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Allowed);
    }

    #[test]
    fn poll_and_unsubscribe_tools_manage_subscription_deliveries() {
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:observer");
        let subscription = RuntimeSubscriptionId::trusted("command-stream");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(3_000),
        );
        runtime
            .execute_subscribe_tool(
                principal.clone(),
                RuntimeSubscribeToolRequest::new(
                    subscription.clone(),
                    RuntimeEventFilter::Commands,
                ),
                1_100,
            )
            .unwrap();
        runtime
            .event_bus_mut()
            .publish(command_result_runtime_event("command-1"));
        runtime
            .event_bus_mut()
            .publish(command_result_runtime_event("command-2"));

        let peeked = runtime
            .execute_poll_events_tool(
                principal.clone(),
                RuntimePollEventsToolRequest::new(subscription.clone())
                    .with_limit(1)
                    .peek(true),
                1_200,
            )
            .unwrap();
        assert_eq!(peeked.batch.len(), 1);
        assert_eq!(peeked.batch.remaining_events, 1);
        assert!(peeked.batch.has_more());
        assert_eq!(runtime.event_bus().queued_events(&subscription).unwrap(), 2);

        let drained = runtime
            .execute_poll_events_tool(
                principal.clone(),
                RuntimePollEventsToolRequest::new(subscription.clone()).with_limit(1),
                1_300,
            )
            .unwrap();
        assert_eq!(drained.batch.len(), 1);
        assert_eq!(drained.batch.remaining_events, 1);
        assert_eq!(
            drained.batch.summary().command_results,
            1,
            "poll tool should expose compact batch counts for Chief status loops"
        );
        assert_eq!(runtime.event_bus().queued_events(&subscription).unwrap(), 1);

        let unsubscribed = runtime
            .execute_unsubscribe_tool(
                principal.clone(),
                RuntimeUnsubscribeToolRequest::new(subscription.clone()),
                1_400,
            )
            .unwrap();
        assert_eq!(unsubscribed.batch.len(), 1);
        assert_eq!(unsubscribed.batch.remaining_events, 0);
        assert!(!runtime.event_bus().has_subscription(&subscription));
        assert!(matches!(
            runtime.event_bus().queued_events(&subscription),
            Err(RuntimeError::UnknownSubscription(_))
        ));

        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);
        assert_eq!(decisions.len(), 4);
        assert!(decisions
            .iter()
            .all(|decision| decision.outcome == AuthorizationOutcome::Allowed));
    }

    #[test]
    fn pairing_tools_require_pair_grants_before_mutating_sessions() {
        let mut runtime = SmartHomeRuntime::new();
        runtime.upsert_bridge(bridge("bridge-1")).unwrap();
        let principal = AgentId::trusted("agent:installer");
        let session = RuntimePairingSessionId::trusted("pairing-1");

        let error = runtime
            .execute_pair_bridge_tool(
                principal.clone(),
                RuntimePairBridgeToolRequest::new(
                    session.clone(),
                    BridgeId::trusted("bridge-1"),
                    1_500,
                ),
                1_000,
            )
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::PairBridge,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.pair")]
        ));
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Denied);
        assert!(runtime.pairing_session(&session).is_none());

        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                session.clone(),
                runtime
                    .registry()
                    .bridge(&BridgeId::trusted("bridge-1"))
                    .unwrap(),
                principal.clone(),
                1_000,
                1_500,
                Vec::new(),
            ))
            .unwrap();
        let complete_error = runtime
            .execute_complete_pairing_tool(
                principal.clone(),
                RuntimeCompletePairingToolRequest::new(
                    session.clone(),
                    VaultRef::trusted("vault://smart-home/hue/bridge-1/app-key"),
                    1_200,
                ),
                1_200,
            )
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            complete_error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::CompletePairing,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.pair")]
        ));
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[1].outcome, AuthorizationOutcome::Denied);
        assert_eq!(runtime.pairing_session(&session).unwrap().vault_ref, None);
    }

    #[test]
    fn pair_bridge_tool_starts_sessions_and_completion_records_vault_refs_only() {
        let mut runtime = SmartHomeRuntime::new();
        let mut unpaired_bridge = bridge("bridge-1");
        unpaired_bridge.health = Health::Unpaired;
        runtime.upsert_bridge(unpaired_bridge).unwrap();
        let principal = AgentId::trusted("agent:installer");
        let session_id = RuntimePairingSessionId::trusted("pairing-1");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-pair"),
                principal.clone(),
                CapabilityId::trusted("smart_home.pair"),
                PrivilegeTier::HumanApproval,
                "chief-of-staff",
                900,
            )
            .with_expiry(2_000),
        );

        let output = runtime
            .execute_pair_bridge_tool(
                principal.clone(),
                RuntimePairBridgeToolRequest::new(
                    session_id.clone(),
                    BridgeId::trusted("bridge-1"),
                    1_500,
                )
                .with_metadata(vec![Metadata::new("pairing_kind", "hue_link_button")]),
                1_000,
            )
            .unwrap();

        assert_eq!(output.session.session_id, session_id);
        assert_eq!(output.session.requested_by, principal);
        assert_eq!(
            output.session.status,
            PairingSessionStatus::PendingUserPresence
        );
        assert_eq!(output.session.vault_ref, None);
        assert_eq!(runtime.pairing_session_count(), 1);
        assert_eq!(
            runtime
                .registry()
                .bridge(&BridgeId::trusted("bridge-1"))
                .unwrap()
                .auth_ref,
            None
        );

        let completed = runtime
            .execute_complete_pairing_tool(
                principal.clone(),
                RuntimeCompletePairingToolRequest::new(
                    session_id.clone(),
                    VaultRef::trusted("vault://smart-home/hue/bridge-1/app-key"),
                    1_200,
                )
                .with_metadata(vec![Metadata::new("pairing_kind", "hue_link_button")]),
                1_200,
            )
            .unwrap()
            .session;
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap();

        assert_eq!(completed.status, PairingSessionStatus::Completed);
        assert_eq!(
            completed.vault_ref,
            Some(VaultRef::trusted("vault://smart-home/hue/bridge-1/app-key"))
        );
        assert_eq!(
            bridge.auth_ref,
            Some(VaultRef::trusted("vault://smart-home/hue/bridge-1/app-key"))
        );
        assert_eq!(bridge.health, Health::Online);
        assert_eq!(bridge.last_seen_at_ms, Some(1_200));
        assert!(matches!(
            runtime.event_bus().published(),
            [RuntimeEvent::Device(event), RuntimeEvent::BridgeHealth { event_id, bridge_id, health, .. }]
                if event.event_type == DeviceEventType::Health
                    && event.metadata.iter().any(|metadata| {
                        metadata.key == "smart_home.pairing_session"
                            && metadata.value == "pairing-1"
                    })
                    && event.metadata.iter().any(|metadata| {
                        metadata.key == "pairing_kind"
                            && metadata.value == "hue_link_button"
                    })
                    && event.metadata.iter().all(|metadata| {
                        !metadata.value.contains("app-key")
                    })
                    && event_id == &EventId::trusted("pairing.completed.health:bridge-1:1200")
                    && bridge_id == &BridgeId::trusted("bridge-1")
                    && *health == Health::Online
        ));

        let repeat_error = runtime
            .complete_pairing_session(
                &session_id,
                VaultRef::trusted("vault://smart-home/hue/bridge-1/rotated-app-key"),
                1_700,
            )
            .unwrap_err();
        assert!(matches!(
            repeat_error,
            RuntimeError::PairingSessionNotPending {
                status: PairingSessionStatus::Completed,
                ..
            }
        ));
        assert_eq!(
            runtime.pairing_session(&session_id).unwrap().vault_ref,
            Some(VaultRef::trusted("vault://smart-home/hue/bridge-1/app-key"))
        );
        assert_eq!(
            runtime
                .registry()
                .authorization_decisions_for_principal(&principal)
                .len(),
            2
        );
    }

    #[test]
    fn supervision_tick_expires_stale_pairing_sessions_without_credentials() {
        let mut runtime = SmartHomeRuntime::new();
        let mut unpaired_bridge = bridge("bridge-1");
        unpaired_bridge.health = Health::Unpaired;
        runtime.upsert_bridge(unpaired_bridge).unwrap();
        let session_id = RuntimePairingSessionId::trusted("pairing-1");
        let session = RuntimePairingSession::pending(
            session_id.clone(),
            runtime
                .registry()
                .bridge(&BridgeId::trusted("bridge-1"))
                .unwrap(),
            AgentId::trusted("agent:installer"),
            1_000,
            1_100,
            Vec::new(),
        );
        runtime.start_pairing_session(session).unwrap();

        let plan = runtime.supervision_plan_at(1_100).unwrap();
        let report = runtime.run_supervision_tick(1_100).unwrap();
        let session = runtime.pairing_session(&session_id).unwrap();
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap();

        assert_eq!(plan.pairing_sessions_expiring, vec![session_id.clone()]);
        assert_eq!(plan.action_count(), 1);
        assert_eq!(report.expired_pairing_sessions, vec![session_id]);
        assert_eq!(report.action_count(), 1);
        assert!(!report.is_idle());
        assert_eq!(session.status, PairingSessionStatus::Expired);
        assert_eq!(session.vault_ref, None);
        assert_eq!(bridge.health, Health::Unpaired);
        assert_eq!(bridge.auth_ref, None);
    }

    #[test]
    fn pairing_session_queries_filter_status_and_expiry() {
        let mut runtime = SmartHomeRuntime::new();
        runtime.upsert_bridge(bridge("bridge-1")).unwrap();
        let mut second_bridge = bridge("bridge-2");
        second_bridge.identifiers =
            vec![
                ProtocolIdentifier::new(ProtocolFamily::Hue, "bridge", "bridge-native-2").unwrap(),
            ];
        runtime.upsert_bridge(second_bridge).unwrap();
        let installer = AgentId::trusted("agent:installer");
        let bridge_one = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        let bridge_two = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-2"))
            .unwrap()
            .clone();

        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("pairing-1"),
                &bridge_one,
                installer.clone(),
                1_000,
                1_200,
                Vec::new(),
            ))
            .unwrap();
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("pairing-2"),
                &bridge_two,
                installer.clone(),
                1_100,
                1_500,
                Vec::new(),
            ))
            .unwrap();

        let expiring = runtime.query_pairing_sessions(
            &RuntimePairingSessionQuery::new()
                .requested_by(installer)
                .with_status(PairingSessionStatus::PendingUserPresence)
                .expiring_at(1_250)
                .sorted_by(RuntimePairingSessionSort::ExpiresAt),
        );

        assert_eq!(
            expiring
                .iter()
                .map(|session| session.session_id.as_str())
                .collect::<Vec<_>>(),
            vec!["pairing-1"]
        );
        assert_eq!(
            runtime
                .query_pairing_sessions(
                    &RuntimePairingSessionQuery::new()
                        .for_bridge(BridgeId::trusted("bridge-2"))
                        .expires_before(1_600),
                )
                .len(),
            1
        );
    }

    #[test]
    fn pairing_session_inventory_summary_counts_statuses_and_vault_refs() {
        let mut runtime = SmartHomeRuntime::new();
        runtime.upsert_bridge(bridge("bridge-1")).unwrap();
        let mut second_bridge = bridge("bridge-2");
        second_bridge.identifiers =
            vec![
                ProtocolIdentifier::new(ProtocolFamily::Hue, "bridge", "bridge-native-2").unwrap(),
            ];
        runtime.upsert_bridge(second_bridge).unwrap();
        let installer = AgentId::trusted("agent:installer");
        let bridge_one = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        let bridge_two = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-2"))
            .unwrap()
            .clone();
        let pending_session_id = RuntimePairingSessionId::trusted("pairing-1");
        let completed_session_id = RuntimePairingSessionId::trusted("pairing-2");

        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                pending_session_id,
                &bridge_one,
                installer.clone(),
                1_000,
                1_200,
                Vec::new(),
            ))
            .unwrap();
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                completed_session_id.clone(),
                &bridge_two,
                installer,
                1_050,
                1_500,
                Vec::new(),
            ))
            .unwrap();
        runtime
            .complete_pairing_session(
                &completed_session_id,
                VaultRef::trusted("vault://hue/bridge-2"),
                1_100,
            )
            .unwrap();

        let summary =
            runtime.pairing_session_inventory_summary_at(&RuntimePairingSessionQuery::new(), 1_250);

        assert_eq!(
            summary,
            RuntimePairingSessionInventorySummary {
                total_sessions: 2,
                pending_user_presence_sessions: 1,
                completed_sessions: 1,
                expired_sessions: 0,
                cancelled_sessions: 0,
                expiring_sessions: 1,
                sessions_with_vault_ref: 1,
            }
        );
        assert!(summary.has_pending_user_presence());
        assert!(summary.has_expiring_sessions());
        assert!(summary.has_completed_credentials());
    }

    #[test]
    fn command_tool_requires_command_tool_grant_before_dispatch() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:lighting-planner");

        let error = runtime
            .execute_command_tool(
                principal.clone(),
                RuntimeCommandToolRequest::new(
                    EntityId::trusted("entity-1"),
                    CommandType::TurnOn,
                    Value::Null,
                ),
                1_500,
            )
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::Command,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.command.light")]
        ));
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Denied);
        assert!(runtime.event_bus().published().is_empty());
        assert!(runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .is_none());
    }

    #[test]
    fn command_tool_requires_entity_command_grants_after_tool_grant() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:lighting-planner");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-tool-capability"),
                principal.clone(),
                CapabilityId::trusted("smart_home.command.light"),
                PrivilegeTier::LowRisk,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );

        let error = runtime
            .execute_command_tool(
                principal.clone(),
                RuntimeCommandToolRequest::new(
                    EntityId::trusted("entity-1"),
                    CommandType::TurnOn,
                    Value::Null,
                ),
                1_500,
            )
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            error,
            RuntimeError::UnauthorizedCommand {
                principal_id,
                missing_capabilities,
                ..
            } if principal_id == principal
                && missing_capabilities == vec![CapabilityId::trusted("light.on_off")]
        ));
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Allowed);
        assert_eq!(decisions[1].outcome, AuthorizationOutcome::Denied);
        assert!(runtime.event_bus().published().is_empty());
        assert!(runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .is_none());
    }

    #[test]
    fn desired_state_tools_require_command_tool_grant_before_mutating_runtime() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:lighting-planner");

        let set_error = runtime
            .execute_set_desired_state_tool(
                principal.clone(),
                RuntimeSetDesiredStateToolRequest::new(DesiredEntityState::new(
                    EntityId::trusted("entity-1"),
                    vec![StateDelta {
                        capability_id: CapabilityId::trusted("light.on_off"),
                        value: Value::Bool(true),
                    }],
                )),
                1_500,
            )
            .unwrap_err();
        let clear_error = runtime
            .execute_clear_desired_state_tool(
                principal.clone(),
                RuntimeClearDesiredStateToolRequest::new(EntityId::trusted("entity-1")),
                1_501,
            )
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            set_error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::SetDesiredState,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.command.light")]
        ));
        assert!(matches!(
            clear_error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::ClearDesiredState,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.command.light")]
        ));
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Denied);
        assert_eq!(decisions[1].outcome, AuthorizationOutcome::Denied);
        assert_eq!(runtime.desired_state_count(), 0);
    }

    #[test]
    fn desired_state_tools_authorize_set_replace_and_clear() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:lighting-planner");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-desired-state-command"),
                principal.clone(),
                CapabilityId::trusted("smart_home.command.light"),
                PrivilegeTier::LowRisk,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );

        let first_output = runtime
            .execute_set_desired_state_tool(
                principal.clone(),
                RuntimeSetDesiredStateToolRequest::new(
                    DesiredEntityState::new(
                        EntityId::trusted("entity-1"),
                        vec![StateDelta {
                            capability_id: CapabilityId::trusted("light.on_off"),
                            value: Value::Bool(true),
                        }],
                    )
                    .requested_by("agent:scene-planner")
                    .with_command_timeout(750),
                ),
                1_500,
            )
            .unwrap();
        let second_output = runtime
            .execute_set_desired_state_tool(
                principal.clone(),
                RuntimeSetDesiredStateToolRequest::new(
                    DesiredEntityState::new(
                        EntityId::trusted("entity-1"),
                        vec![StateDelta {
                            capability_id: CapabilityId::trusted("light.on_off"),
                            value: Value::Bool(false),
                        }],
                    )
                    .requested_by("agent:scene-planner")
                    .with_command_timeout(900),
                ),
                1_501,
            )
            .unwrap();
        let clear_output = runtime
            .execute_clear_desired_state_tool(
                principal.clone(),
                RuntimeClearDesiredStateToolRequest::new(EntityId::trusted("entity-1")),
                1_502,
            )
            .unwrap();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(!first_output.replaced);
        assert!(first_output.previous.is_none());
        assert_eq!(
            first_output.desired_state.requested_by,
            "agent:scene-planner"
        );
        assert!(second_output.replaced);
        assert_eq!(
            second_output
                .previous
                .as_ref()
                .map(|state| state.command_timeout_ms),
            Some(750)
        );
        assert!(clear_output.removed());
        assert_eq!(
            clear_output
                .removed
                .as_ref()
                .map(|state| state.command_timeout_ms),
            Some(900)
        );
        assert_eq!(runtime.desired_state_count(), 0);
        assert_eq!(decisions.len(), 3);
        assert!(decisions
            .iter()
            .all(|decision| decision.outcome == AuthorizationOutcome::Allowed));
    }

    #[test]
    fn supervision_tool_facade_authorizes_and_reconciles_desired_state() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:supervisor");
        runtime
            .upsert_desired_state(DesiredEntityState::new(
                EntityId::trusted("entity-1"),
                vec![StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(true),
                }],
            ))
            .unwrap();

        let error = runtime
            .execute_supervision_tool(
                principal.clone(),
                RuntimeSupervisionToolRequest::ReconcileDesiredStates,
                1_500,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::ReconcileDesiredStates,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.command.light")]
        ));
        assert!(runtime.event_bus().published().is_empty());

        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-supervision-command"),
                principal.clone(),
                CapabilityId::trusted("smart_home.command.light"),
                PrivilegeTier::LowRisk,
                "user:test",
                1_600,
            )
            .with_expiry(2_000),
        );

        let output = runtime
            .execute_supervision_tool(
                principal.clone(),
                RuntimeSupervisionToolRequest::ReconcileDesiredStates,
                1_700,
            )
            .unwrap();

        let RuntimeSupervisionToolOutput::DesiredStateReconciliation {
            reconciled_at_ms,
            actions,
        } = output
        else {
            panic!("expected desired-state reconciliation output");
        };
        assert_eq!(reconciled_at_ms, 1_700);
        assert!(matches!(
            actions.as_slice(),
            [DesiredStateAction::CommandIssued {
                reason: ReconciliationReason::MissingState,
                ..
            }]
        ));
        assert_eq!(
            runtime
                .registry()
                .authorization_decisions_for_principal(&principal)
                .len(),
            2
        );
        assert_eq!(
            runtime.event_bus().published().len(),
            2,
            "reconciliation publishes drift and command-result events"
        );
    }

    #[test]
    fn command_tool_authorizes_and_dispatches_device_commands() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:lighting-planner");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-command-tool"),
                principal.clone(),
                CapabilityId::trusted("smart_home.command.light"),
                PrivilegeTier::LowRisk,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("grant-entity-command"),
                principal.clone(),
                EntityId::trusted("entity-1"),
                CapabilityId::trusted("light.on_off"),
                PrivilegeTier::LowRisk,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );

        let result = runtime
            .execute_command_tool(
                principal.clone(),
                RuntimeCommandToolRequest::new(
                    EntityId::trusted("entity-1"),
                    CommandType::TurnOn,
                    Value::Null,
                )
                .with_idempotency_key("turn-on:kitchen")
                .with_timeout_ms(1_234),
                1_500,
            )
            .unwrap();
        let snapshot = runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .unwrap();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert_eq!(result.status, CommandStatus::Accepted);
        assert_eq!(result.bridge_id, BridgeId::trusted("bridge-1"));
        assert!(result
            .command_id
            .as_str()
            .starts_with("tool:agent:lighting-planner:entity-1:1500:"));
        assert_eq!(result.command_id.as_str(), result.correlation_id.as_str());
        assert_eq!(
            snapshot.value,
            Value::Object(vec![("light.on_off".to_string(), Value::Bool(true))])
        );
        assert_eq!(snapshot.source, StateSource::OptimisticCommand);
        assert_eq!(snapshot.expires_at_ms, Some(2_734));
        assert!(matches!(
            runtime.event_bus().published(),
            [RuntimeEvent::CommandResult(command_result)]
                if command_result.command_id == result.command_id
        ));
        assert_eq!(decisions.len(), 2);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Allowed);
        assert_eq!(decisions[1].outcome, AuthorizationOutcome::Allowed);
    }

    #[test]
    fn read_tool_facade_returns_registry_snapshots() {
        let mut runtime = runtime_with_entity(vec![
            Capability::light_on_off(),
            Capability::light_brightness(),
        ]);
        let principal = AgentId::trusted("agent:observer");
        let mut kitchen_device = runtime
            .registry()
            .device(&DeviceId::trusted("device-1"))
            .unwrap()
            .clone();
        kitchen_device.room_id = Some("kitchen".to_string());
        runtime.upsert_device(kitchen_device).unwrap();
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );
        runtime
            .supervisor_mut()
            .register_worker(SupervisedBridgeWorker::new(
                BridgeId::trusted("bridge-1"),
                IntegrationId::trusted("hue"),
                1_000,
                100,
            ));
        runtime
            .registry_mut()
            .apply_state_snapshot(StateSnapshot {
                entity_id: EntityId::trusted("entity-1"),
                value: Value::Object(vec![("light.on_off".to_string(), Value::Bool(true))]),
                source: StateSource::EventStream,
                observed_at_ms: 1_100,
                received_at_ms: 1_101,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            })
            .unwrap();
        runtime
            .upsert_scene(Scene {
                scene_id: SceneId::trusted("scene-1"),
                scope: SceneScope::Room,
                native_ref: None,
                actions: vec![smart_home_core::SceneAction {
                    entity_id: EntityId::trusted("entity-1"),
                    desired_state: Value::Bool(true),
                }],
                metadata: vec![Metadata::new("fixture", "runtime_read_tool_scene")],
            })
            .unwrap();
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("pairing-1"),
                &bridge,
                principal.clone(),
                1_100,
                2_000,
                vec![Metadata::new("fixture", "runtime_read_tool_pairing")],
            ))
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
                .requested_by("agent:observer")
                .with_command_timeout(750),
            )
            .unwrap();
        runtime
            .event_bus_mut()
            .subscribe(
                RuntimeSubscriptionId::trusted("commands"),
                RuntimeEventFilter::Commands,
            )
            .unwrap();
        runtime
            .event_bus_mut()
            .publish(command_result_runtime_event("command-1"));

        let bridges = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListBridges,
                1_500,
            )
            .unwrap();
        let devices = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListDevices {
                    bridge_id: Some(BridgeId::trusted("bridge-1")),
                    health: Some(Health::Online),
                    capability_id: Some(CapabilityId::trusted("light.on_off")),
                },
                1_501,
            )
            .unwrap();
        let scenes = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListScenes {
                    scope: Some(SceneScope::Room),
                    entity_id: Some(EntityId::trusted("entity-1")),
                    capability_id: Some(CapabilityId::trusted("light.on_off")),
                },
                1_502,
            )
            .unwrap();
        let scene = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::DescribeScene {
                    scene_id: SceneId::trusted("scene-1"),
                },
                1_503,
            )
            .unwrap();
        let state = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::GetState {
                    entity_id: EntityId::trusted("entity-1"),
                },
                1_504,
            )
            .unwrap();
        let capabilities = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::DescribeCapabilities {
                    entity_id: EntityId::trusted("entity-1"),
                },
                1_505,
            )
            .unwrap();
        let health = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::GetHealth {
                    bridge_id: Some(BridgeId::trusted("bridge-1")),
                },
                1_506,
            )
            .unwrap();
        let subscriptions = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListSubscriptions {
                    query: RuntimeSubscriptionQuery::new()
                        .matching(RuntimeEventFilter::Commands)
                        .with_min_queued_events(1),
                },
                1_507,
            )
            .unwrap();
        let event_log = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::InspectEventLog {
                    query: RuntimeEventQuery::new()
                        .matching(RuntimeEventFilter::Commands)
                        .sorted_by(RuntimeEventSort::SequenceDesc)
                        .with_limit(1),
                },
                1_508,
            )
            .unwrap();
        let command_results = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListCommandResults {
                    query: RuntimeCommandResultQuery::new()
                        .for_bridge(BridgeId::trusted("bridge-1"))
                        .with_status(CommandStatus::Accepted)
                        .sorted_by(RuntimeCommandResultSort::SequenceDesc)
                        .with_limit(1),
                },
                1_509,
            )
            .unwrap();
        let command_result_summary = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::GetCommandResultSummary {
                    query: RuntimeCommandResultQuery::new()
                        .for_command(CommandId::trusted("command-1"))
                        .with_status(CommandStatus::Accepted),
                },
                1_510,
            )
            .unwrap();
        let snapshot = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::GetRuntimeSnapshot,
                1_511,
            )
            .unwrap();
        let desired_states = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListDesiredStates {
                    query: DesiredStateQuery::new()
                        .requested_by("agent:observer")
                        .with_capability(CapabilityId::trusted("light.on_off"))
                        .sorted_by(DesiredStateSort::CommandTimeoutDesc),
                },
                1_512,
            )
            .unwrap();
        let pairing_sessions = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListPairingSessions {
                    query: RuntimePairingSessionQuery::new()
                        .for_bridge(BridgeId::trusted("bridge-1"))
                        .with_status(PairingSessionStatus::PendingUserPresence)
                        .sorted_by(RuntimePairingSessionSort::ExpiresAt),
                },
                1_513,
            )
            .unwrap();
        let supervision_plan = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::GetSupervisionPlan,
                1_514,
            )
            .unwrap();
        let observation = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ObserveSupervision,
                1_515,
            )
            .unwrap();
        let workers = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::ListWorkers {
                    query: SupervisedWorkerQuery::new()
                        .with_status(WorkerStatus::Starting)
                        .overdue_at(1_516)
                        .sorted_by(SupervisedWorkerSort::HeartbeatDueAt),
                },
                1_516,
            )
            .unwrap();
        let heartbeat_schedule = runtime
            .execute_read_tool(
                principal,
                RuntimeReadToolRequest::GetWorkerHeartbeatSchedule {
                    bridge_id: Some(BridgeId::trusted("bridge-1")),
                    due_at_or_before_ms: Some(1_517),
                    limit: Some(1),
                },
                1_517,
            )
            .unwrap();
        let authorization_decisions = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::ListAuthorizationDecisions {
                    query: RuntimeAuthorizationDecisionQuery::new()
                        .for_principal(AgentId::trusted("agent:observer"))
                        .with_outcome(AuthorizationOutcome::Allowed)
                        .sorted_by(RuntimeAuthorizationDecisionSort::DecidedAtDesc)
                        .with_limit(2),
                },
                1_518,
            )
            .unwrap();
        let authorization_summary = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::GetAuthorizationSummary {
                    query: RuntimeAuthorizationDecisionQuery::new()
                        .for_principal(AgentId::trusted("agent:observer"))
                        .with_outcome(AuthorizationOutcome::Allowed),
                },
                1_519,
            )
            .unwrap();
        let capability_grants = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::ListCapabilityGrants {
                    query: RuntimeCapabilityGrantQuery::new()
                        .for_principal(AgentId::trusted("agent:observer"))
                        .with_status(CapabilityGrantStatus::Active)
                        .with_scope_kind(RuntimeCapabilityGrantScopeKind::Capability)
                        .with_capability(CapabilityId::trusted("smart_home.read")),
                },
                1_520,
            )
            .unwrap();
        let capability_grant_summary = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::GetCapabilityGrantSummary {
                    query: RuntimeCapabilityGrantQuery::new()
                        .with_status(CapabilityGrantStatus::Active)
                        .with_scope_kind(RuntimeCapabilityGrantScopeKind::Capability),
                },
                1_521,
            )
            .unwrap();
        let rooms = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::ListRooms {
                    query: RuntimeRoomQuery::new()
                        .for_room("kitchen")
                        .sorted_by(RuntimeRoomSort::SceneCountDesc),
                },
                1_522,
            )
            .unwrap();
        let topology_summary = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::GetTopologySummary,
                1_523,
            )
            .unwrap();
        runtime
            .register_discovery_worker_schedule(hue_mdns_discovery_worker(1_100))
            .unwrap();
        let discovery_workers = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::ListDiscoveryWorkers {
                    query: DiscoveryWorkerQuery::new()
                        .for_integration(IntegrationId::trusted("hue"))
                        .with_kind(DiscoveryWorkerKind::MdnsScan)
                        .with_source(DiscoverySource::Mdns)
                        .overdue_at(1_522)
                        .sorted_by(DiscoveryWorkerSort::NextDueAt),
                },
                1_524,
            )
            .unwrap();
        runtime
            .record_discovery(hue_discovery_record("001788fffediscovered", 1_000))
            .unwrap();
        let discovery_summary = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::GetDiscoverySummary {
                    request: RuntimeDiscoverToolRequest::new()
                        .for_integration(IntegrationId::trusted("hue"))
                        .from_source(DiscoverySource::Mdns)
                        .fresh_only(true)
                        .with_ttl_ms(1_000),
                },
                1_525,
            )
            .unwrap();
        let pairing_plan = runtime
            .execute_read_tool(
                AgentId::trusted("agent:observer"),
                RuntimeReadToolRequest::GetPairingPlan {
                    request: RuntimePairingPlanToolRequest::new()
                        .with_ttl_ms(1_000)
                        .with_options(
                            DiscoveryPairingPlanOptions::new()
                                .with_integration(IntegrationId::trusted("hue"))
                                .with_source(DiscoverySource::Mdns)
                                .with_pairing_requirement(PairingRequirement::PhysicalPresence)
                                .actionable_only(true)
                                .limited_to(1),
                        ),
                },
                1_526,
            )
            .unwrap();

        assert!(matches!(
            bridges,
            RuntimeReadToolOutput::Bridges(bridges) if bridges.len() == 1
                && bridges[0].bridge_id == BridgeId::trusted("bridge-1")
        ));
        assert!(matches!(
            devices,
            RuntimeReadToolOutput::Devices(devices) if devices.len() == 1
                && devices[0].device_id == DeviceId::trusted("device-1")
        ));
        assert!(matches!(
            scenes,
            RuntimeReadToolOutput::Scenes(scenes) if scenes.len() == 1
                && scenes[0].scene_id == SceneId::trusted("scene-1")
        ));
        assert!(matches!(
            scene,
            RuntimeReadToolOutput::Scene {
                scene_id,
                scene,
            } if scene_id == SceneId::trusted("scene-1")
                && scene.actions.len() == 1
        ));
        assert!(matches!(
            state,
            RuntimeReadToolOutput::State {
                entity_id,
                snapshot: Some(snapshot),
            } if entity_id == EntityId::trusted("entity-1")
                && snapshot.confidence == StateConfidence::Confirmed
        ));
        assert!(matches!(
            capabilities,
            RuntimeReadToolOutput::Capabilities {
                entity_id,
                capabilities,
            } if entity_id == EntityId::trusted("entity-1")
                && capabilities.len() == 2
        ));
        assert!(matches!(
            health,
            RuntimeReadToolOutput::Health(health) if health == vec![BridgeHealthSnapshot {
                bridge_id: BridgeId::trusted("bridge-1"),
                integration_id: IntegrationId::trusted("hue"),
                health: Health::Unknown,
                last_seen_at_ms: None,
            }]
        ));
        assert!(matches!(
            subscriptions,
            RuntimeReadToolOutput::Subscriptions {
                subscriptions,
                summary,
            } if subscriptions.len() == 1
                && subscriptions[0].subscription_id == RuntimeSubscriptionId::trusted("commands")
                && subscriptions[0].queued_events == 1
                && summary.command_subscriptions == 1
                && summary.backlogged_subscriptions == 1
        ));
        assert!(matches!(
            event_log,
            RuntimeReadToolOutput::EventLog {
                entries,
                summary,
            } if entries.len() == 1
                && entries[0].sequence == 0
                && matches!(
                    &entries[0].event,
                    RuntimeEvent::CommandResult(result)
                        if result.command_id == CommandId::trusted("command-1")
                )
                && summary.total_events == 1
                && summary.command_results == 1
        ));
        assert!(matches!(
            command_results,
            RuntimeReadToolOutput::CommandResults {
                results,
                summary,
            } if results.len() == 1
                && results[0].sequence == 0
                && results[0].result.command_id == CommandId::trusted("command-1")
                && results[0].result.status == CommandStatus::Accepted
                && summary.total_results == 1
                && summary.accepted_results == 1
                && !summary.has_failures()
        ));
        assert!(matches!(
            command_result_summary,
            RuntimeReadToolOutput::CommandResultSummary { summary }
                if summary.total_results == 1
                    && summary.accepted_results == 1
                    && summary.failure_results() == 0
                    && summary.next_checkpoint == RuntimeEventCheckpoint::from_next_sequence(1)
        ));
        assert!(matches!(
            snapshot,
            RuntimeReadToolOutput::RuntimeSnapshot(snapshot)
                if snapshot.generated_at_ms == 1_511
                    && snapshot.pairing_session_count == 1
                    && snapshot.desired_state_count == 1
                    && snapshot.desired_capability_count == 1
        ));
        assert!(matches!(
            desired_states,
            RuntimeReadToolOutput::DesiredStates {
                desired_states,
                summary,
            } if desired_states.len() == 1
                && desired_states[0].entity_id == EntityId::trusted("entity-1")
                && desired_states[0].requested_by == "agent:observer"
                && summary.total_desired_states == 1
                && summary.total_desired_capabilities == 1
                && summary.requested_by_count == 1
                && summary.max_command_timeout_ms == Some(750)
        ));
        assert!(matches!(
            pairing_sessions,
            RuntimeReadToolOutput::PairingSessions {
                sessions,
                summary,
            } if sessions.len() == 1
                && sessions[0].session_id == RuntimePairingSessionId::trusted("pairing-1")
                && summary.total_sessions == 1
                && summary.pending_user_presence_sessions == 1
                && !summary.has_expiring_sessions()
        ));
        assert!(matches!(
            supervision_plan,
            RuntimeReadToolOutput::SupervisionPlan(plan)
                if plan.generated_at_ms == 1_514
                    && plan.action_count() == 1
                    && plan.summary().worker_restart_count == 1
                    && plan.worker_restart_plan.len() == 1
        ));
        assert!(matches!(
            observation,
            RuntimeReadToolOutput::SupervisionObservation(observation)
                if observation.generated_at_ms == 1_515
                    && observation.worker_restart_count() == 1
                    && observation.due_worker_deadline_count() == 1
                    && observation.next_worker_heartbeat_due_at_ms() == Some(1_100)
        ));
        assert!(matches!(
            workers,
            RuntimeReadToolOutput::Workers { workers, summary }
                if workers.len() == 1
                    && workers[0].bridge_id == BridgeId::trusted("bridge-1")
                    && workers[0].status == WorkerStatus::Starting
                    && summary.generated_at_ms == 1_516
                    && summary.worker_count == 1
                    && summary.restart_due_count == 1
        ));
        assert!(matches!(
            heartbeat_schedule,
            RuntimeReadToolOutput::WorkerHeartbeatSchedule(schedule)
                if schedule.generated_at_ms == 1_517
                    && schedule.len() == 1
                    && schedule.next_due_at_ms() == Some(1_100)
                    && schedule.deadlines[0].bridge_id == BridgeId::trusted("bridge-1")
                    && schedule.deadlines[0].is_due_at(1_517)
        ));
        assert!(matches!(
            authorization_decisions,
            RuntimeReadToolOutput::AuthorizationDecisions { decisions, summary }
                if decisions.len() == 2
                    && decisions[0].decided_at_ms == 1_518
                    && decisions[0].subject == AuthorizationSubject::Tool(
                        SmartHomeTool::ListAuthorizationDecisions
                    )
                    && summary.total_decisions == 2
                    && summary.allowed_decisions == 2
                    && summary.denied_decisions == 0
        ));
        assert!(matches!(
            authorization_summary,
            RuntimeReadToolOutput::AuthorizationSummary { summary }
                if summary.total_decisions == 20
                    && summary.allowed_decisions == 20
                    && summary.denied_decisions == 0
                    && summary.tool_decisions == 20
        ));
        assert!(matches!(
            capability_grants,
            RuntimeReadToolOutput::CapabilityGrants { grants, summary }
                if grants.len() == 1
                    && grants[0].grant_id == CapabilityGrantId::trusted("grant-read")
                    && summary.total_grants == 1
                    && summary.active_grants == 1
                    && summary.capability_grants == 1
                    && summary.read_only_tier_grants == 1
        ));
        assert!(matches!(
            capability_grant_summary,
            RuntimeReadToolOutput::CapabilityGrantSummary { summary }
                if summary.total_grants == 1
                    && summary.active_grants == 1
                    && summary.unique_principals == 1
                    && !summary.needs_review()
        ));
        assert!(matches!(
            rooms,
            RuntimeReadToolOutput::Rooms { rooms, topology }
                if rooms.len() == 1
                    && rooms[0].room_id == "kitchen"
                    && rooms[0].device_count == 1
                    && rooms[0].entity_count == 1
                    && rooms[0].commandable_entities == 1
                    && rooms[0].entities_with_state == 1
                    && rooms[0].scene_count == 1
                    && rooms[0].scene_action_count == 1
                    && topology.devices_with_room == 1
                    && topology.unique_rooms == 1
                    && topology.room_scenes == 1
        ));
        assert!(matches!(
            topology_summary,
            RuntimeReadToolOutput::TopologySummary { summary }
                if summary.bridges == 1
                    && summary.devices == 1
                    && summary.entities == 1
                    && summary.scenes == 1
                    && summary.devices_with_room == 1
                    && summary.unique_rooms == 1
                    && summary.scene_actions == 1
        ));
        assert!(matches!(
            discovery_workers,
            RuntimeReadToolOutput::DiscoveryWorkers { workers, summary }
                if workers.len() == 1
                    && workers[0].worker_id == DiscoveryWorkerId::trusted("hue-mdns-worker")
                    && workers[0].kind == DiscoveryWorkerKind::MdnsScan
                    && workers[0].is_due
                    && summary.generated_at_ms == 1_524
                    && summary.worker_count == 1
                    && summary.due_worker_count == 1
        ));
        assert!(matches!(
            discovery_summary,
            RuntimeReadToolOutput::DiscoverySummary {
                generated_at_ms,
                ttl_ms,
                record_summary,
                signal_summary,
            } if generated_at_ms == 1_525
                && ttl_ms == 1_000
                && record_summary.total == 1
                && record_summary.fresh == 1
                && signal_summary.fresh == 1
        ));
        assert!(matches!(
            pairing_plan,
            RuntimeReadToolOutput::PairingPlan {
                ttl_ms,
                plan,
                summary,
            } if ttl_ms == 1_000
                && plan.generated_at_ms == 1_526
                && plan.targets.len() == 1
                && plan.targets[0].bridge_id
                    == BridgeId::trusted("hue.bridge.001788fffediscovered")
                && plan.targets[0].action == DiscoveryPairingAction::PressPhysicalButton
                && summary.total == 1
                && summary.actionable == 1
                && summary.requires_human_action == 1
        ));
        assert_eq!(runtime.registry().counts().authorization_decisions, 27);
        assert!(runtime
            .registry()
            .authorization_decisions()
            .all(|decision| decision.outcome == AuthorizationOutcome::Allowed));
        assert!(matches!(
            runtime.event_bus().published(),
            [RuntimeEvent::CommandResult(result)]
                if result.command_id == CommandId::trusted("command-1")
        ));
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
    fn report_event_tool_requires_ingest_grants_before_mutating_runtime() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:event-worker");
        let error = runtime
            .execute_report_event_tool(
                principal.clone(),
                RuntimeReportEventToolRequest::device(DeviceEvent {
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
                }),
                1_101,
            )
            .unwrap_err();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            error,
            RuntimeError::UnauthorizedTool {
                tool: SmartHomeTool::ReportEvent,
                missing_capabilities,
                ..
            } if missing_capabilities == vec![CapabilityId::trusted("smart_home.ingest")]
        ));
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].outcome, AuthorizationOutcome::Denied);
        assert!(runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .is_none());
        assert_eq!(runtime.registry().counts().events, 0);
    }

    #[test]
    fn report_event_tool_authorizes_device_and_health_ingest() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let principal = AgentId::trusted("agent:event-worker");
        runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-ingest"),
                principal.clone(),
                CapabilityId::trusted("smart_home.ingest"),
                PrivilegeTier::LowRisk,
                "chief-of-staff",
                1_000,
            )
            .with_expiry(2_000),
        );
        runtime
            .submit_command(command(CommandType::TurnOn, Value::Null), 1_000)
            .unwrap();

        let device_output = runtime
            .execute_report_event_tool(
                principal.clone(),
                RuntimeReportEventToolRequest::device(DeviceEvent {
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
                    raw_ref: Some("event-log://hue/bridge-1/42".to_string()),
                    correlation_id: Some(CorrelationId::trusted("hue-event-42")),
                    metadata: vec![Metadata::new("source", "hue_sse")],
                }),
                1_101,
            )
            .unwrap();
        let health_output = runtime
            .execute_report_event_tool(
                principal.clone(),
                RuntimeReportEventToolRequest::bridge_health(BridgeHealthReport {
                    event_id: EventId::trusted("health-1"),
                    bridge_id: BridgeId::trusted("bridge-1"),
                    health: Health::Offline,
                    observed_at_ms: 1_200,
                    received_at_ms: 1_201,
                    metadata: vec![Metadata::new("source", "heartbeat")],
                }),
                1_201,
            )
            .unwrap();
        let snapshot = runtime
            .registry()
            .state(&EntityId::trusted("entity-1"))
            .unwrap();
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap();
        let decisions = runtime
            .registry()
            .authorization_decisions_for_principal(&principal);

        assert!(matches!(
            device_output,
            RuntimeReportEventToolOutput::Device(DeviceEvent { event_id, .. })
                if event_id == EventId::trusted("event-1")
        ));
        assert!(matches!(
            health_output,
            RuntimeReportEventToolOutput::BridgeHealth(BridgeHealthReport { event_id, health, .. })
                if event_id == EventId::trusted("health-1") && health == Health::Offline
        ));
        assert_eq!(snapshot.confidence, StateConfidence::Confirmed);
        assert_eq!(
            snapshot.value,
            Value::Object(vec![("light.on_off".to_string(), Value::Bool(false))])
        );
        assert_eq!(runtime.optimistic_state_count(), 0);
        assert_eq!(bridge.health, Health::Offline);
        assert_eq!(runtime.registry().counts().events, 2);
        assert_eq!(decisions.len(), 2);
        assert!(decisions
            .iter()
            .all(|decision| decision.outcome == AuthorizationOutcome::Allowed));
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
    fn desired_state_queries_filter_capability_owner_and_timeout() {
        let mut runtime = runtime_with_entity(vec![
            Capability::light_on_off(),
            Capability::light_brightness(),
        ]);
        runtime
            .upsert_entity(light_entity(
                "entity-2",
                "device-1",
                vec![Capability::light_on_off()],
            ))
            .unwrap();
        runtime
            .upsert_desired_state(
                DesiredEntityState::new(
                    EntityId::trusted("entity-1"),
                    vec![StateDelta {
                        capability_id: CapabilityId::trusted("light.brightness"),
                        value: Value::Percentage(42),
                    }],
                )
                .requested_by("agent:scene")
                .with_command_timeout(750),
            )
            .unwrap();
        runtime
            .upsert_desired_state(
                DesiredEntityState::new(
                    EntityId::trusted("entity-2"),
                    vec![StateDelta {
                        capability_id: CapabilityId::trusted("light.on_off"),
                        value: Value::Bool(true),
                    }],
                )
                .requested_by("agent:guard")
                .with_command_timeout(250),
            )
            .unwrap();

        let matches = runtime.query_desired_states(
            &DesiredStateQuery::new()
                .requested_by("agent:scene")
                .with_capability(CapabilityId::trusted("light.brightness"))
                .min_command_timeout(500)
                .sorted_by(DesiredStateSort::CommandTimeoutDesc),
        );

        assert_eq!(
            matches
                .iter()
                .map(|desired| desired.entity_id.as_str())
                .collect::<Vec<_>>(),
            vec!["entity-1"]
        );
        assert_eq!(
            runtime
                .query_desired_states(
                    &DesiredStateQuery::new()
                        .max_command_timeout(500)
                        .sorted_by(DesiredStateSort::RequestedByThenEntityId),
                )
                .iter()
                .map(|desired| desired.requested_by.as_str())
                .collect::<Vec<_>>(),
            vec!["agent:guard"]
        );
    }

    #[test]
    fn runtime_supervision_plan_previews_due_work_without_mutating() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let bridge_id = BridgeId::trusted("bridge-1");
        let entity_id = EntityId::trusted("entity-1");
        runtime
            .supervisor_mut()
            .register_worker(SupervisedBridgeWorker::new(
                bridge_id.clone(),
                IntegrationId::trusted("hue"),
                1_000,
                100,
            ));
        runtime
            .registry_mut()
            .apply_state_snapshot(StateSnapshot {
                entity_id: entity_id.clone(),
                value: Value::Object(vec![("light.on_off".to_string(), Value::Bool(false))]),
                source: StateSource::EventStream,
                observed_at_ms: 1_000,
                received_at_ms: 1_001,
                expires_at_ms: Some(1_100),
                confidence: StateConfidence::Confirmed,
            })
            .unwrap();
        runtime
            .upsert_desired_state(DesiredEntityState::new(
                entity_id.clone(),
                vec![StateDelta {
                    capability_id: CapabilityId::trusted("light.on_off"),
                    value: Value::Bool(true),
                }],
            ))
            .unwrap();

        let plan = runtime.supervision_plan_at(1_125).unwrap();
        let summary = plan.summary();
        let worker = runtime.supervisor().worker(&bridge_id).unwrap();
        let snapshot = runtime.registry().state(&entity_id).unwrap();

        assert_eq!(plan.generated_at_ms, 1_125);
        assert_eq!(plan.action_count(), 3);
        assert!(!plan.is_empty());
        assert_eq!(
            summary,
            RuntimeSupervisionPlanSummary {
                generated_at_ms: 1_125,
                total_actions: 3,
                pairing_expiry_count: 0,
                state_refresh_count: 1,
                missing_state_refresh_count: 0,
                stale_state_refresh_count: 1,
                desired_state_drift_count: 1,
                desired_missing_state_count: 0,
                desired_stale_state_count: 1,
                desired_drifted_state_count: 0,
                worker_restart_count: 1,
                discovery_worker_run_count: 0,
            }
        );
        assert!(!summary.is_idle());
        assert!(summary.has_state_refresh_work());
        assert!(summary.has_reconciliation_work());
        assert!(summary.has_worker_restart_work());
        assert!(matches!(
            plan.state_refresh_plan.targets.as_slice(),
            [target] if target.entity_id == entity_id
                && target.bridge_id == bridge_id
                && target.reason == StateRefreshReason::Stale
        ));
        assert!(matches!(
            plan.desired_state_drifts.as_slice(),
            [DesiredStateDriftPlan {
                bridge_id: drift_bridge_id,
                entity_id: drift_entity_id,
                capability_id,
                desired_value,
                reason: ReconciliationReason::StaleState,
            }] if drift_bridge_id == &bridge_id
                && drift_entity_id == &entity_id
                && capability_id == &CapabilityId::trusted("light.on_off")
                && desired_value == &Value::Bool(true)
        ));
        assert_eq!(plan.drifts_for_entity(&entity_id).len(), 1);
        assert_eq!(plan.worker_restart_plan.len(), 1);
        assert_eq!(worker.status, WorkerStatus::Starting);
        assert_eq!(worker.restart_count, 0);
        assert_eq!(snapshot.confidence, StateConfidence::Confirmed);
        assert!(runtime.event_bus().published().is_empty());
    }

    #[test]
    fn runtime_supervision_observation_combines_plan_and_heartbeat_schedule() {
        let mut runtime = SmartHomeRuntime::new();
        let early_bridge = BridgeId::trusted("bridge-early");
        let late_bridge = BridgeId::trusted("bridge-late");
        runtime
            .supervisor_mut()
            .register_worker(SupervisedBridgeWorker::new(
                early_bridge.clone(),
                IntegrationId::trusted("hue"),
                1_000,
                100,
            ));
        runtime
            .supervisor_mut()
            .register_worker(SupervisedBridgeWorker::new(
                late_bridge.clone(),
                IntegrationId::trusted("thread"),
                1_000,
                500,
            ));

        let observation = runtime.observe_supervision_at(1_125).unwrap();
        let worker = runtime.supervisor().worker(&early_bridge).unwrap();

        assert_eq!(observation.generated_at_ms, 1_125);
        assert_eq!(observation.action_count(), 1);
        assert_eq!(observation.pairing_expiry_count(), 0);
        assert_eq!(observation.state_refresh_count(), 0);
        assert_eq!(observation.desired_state_drift_count(), 0);
        assert_eq!(observation.worker_restart_count(), 1);
        assert_eq!(observation.due_worker_deadline_count(), 1);
        assert_eq!(observation.next_worker_heartbeat_due_at_ms(), Some(1_100));
        assert_eq!(
            observation.plan_summary(),
            RuntimeSupervisionPlanSummary {
                generated_at_ms: 1_125,
                total_actions: 1,
                pairing_expiry_count: 0,
                state_refresh_count: 0,
                missing_state_refresh_count: 0,
                stale_state_refresh_count: 0,
                desired_state_drift_count: 0,
                desired_missing_state_count: 0,
                desired_stale_state_count: 0,
                desired_drifted_state_count: 0,
                worker_restart_count: 1,
                discovery_worker_run_count: 0,
            }
        );
        assert_eq!(observation.heartbeat_schedule.len(), 2);
        assert!(!observation.is_idle());
        assert_eq!(worker.status, WorkerStatus::Starting);
        assert_eq!(worker.restart_count, 0);
        assert!(runtime.event_bus().published().is_empty());
    }

    #[test]
    fn runtime_read_snapshot_summarizes_non_mutating_work_pressure() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);
        let subscription = RuntimeSubscriptionId::trusted("all-events");
        runtime
            .event_bus_mut()
            .subscribe(subscription, RuntimeEventFilter::All)
            .unwrap();
        runtime
            .supervisor_mut()
            .register_worker(SupervisedBridgeWorker::new(
                BridgeId::trusted("bridge-1"),
                IntegrationId::trusted("hue"),
                1_000,
                100,
            ));
        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("bridge-1"))
            .unwrap()
            .clone();
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("pairing-1"),
                &bridge,
                AgentId::trusted("agent:installer"),
                1_000,
                6_000,
                Vec::new(),
            ))
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
        runtime
            .submit_command(command(CommandType::TurnOn, Value::Null), 1_000)
            .unwrap();

        let snapshot = runtime.read_snapshot_at(6_000);
        let worker = runtime
            .supervisor()
            .worker(&BridgeId::trusted("bridge-1"))
            .unwrap();

        assert_eq!(snapshot.generated_at_ms, 6_000);
        assert_eq!(snapshot.registry_counts.bridges, 1);
        assert_eq!(snapshot.registry_counts.devices, 1);
        assert_eq!(snapshot.registry_counts.entities, 1);
        assert_eq!(snapshot.registry_counts.states, 1);
        assert_eq!(snapshot.event_bus.subscription_count, 1);
        assert_eq!(snapshot.event_bus.published_event_count, 1);
        assert_eq!(snapshot.event_bus.pending_delivery_count, 1);
        assert_eq!(snapshot.event_bus.backlogged_subscription_count, 1);
        assert_eq!(snapshot.event_bus.max_pending_delivery_count, 1);
        assert!(snapshot.event_bus.has_lagging_subscriptions());
        assert_eq!(snapshot.supervisor.worker_count, 1);
        assert_eq!(snapshot.supervisor.starting_count, 1);
        assert_eq!(snapshot.supervisor.restart_due_count, 1);
        assert!(snapshot.supervisor.has_restart_pressure());
        assert_eq!(snapshot.pairing_session_count, 1);
        assert_eq!(snapshot.expiring_pairing_session_count, 1);
        assert_eq!(snapshot.optimistic_state_count, 1);
        assert_eq!(snapshot.stale_optimistic_state_count, 1);
        assert_eq!(snapshot.desired_state_count, 1);
        assert_eq!(snapshot.desired_capability_count, 1);
        assert_eq!(snapshot.state_refresh_target_count, 1);
        assert!(snapshot.has_pending_work());
        let pending = snapshot.pending_work_summary();
        assert_eq!(pending.event_backlog_count, 1);
        assert_eq!(pending.backlogged_subscription_count, 1);
        assert_eq!(pending.restart_due_count, 1);
        assert_eq!(pending.unhealthy_worker_count, 0);
        assert_eq!(pending.expiring_pairing_session_count, 1);
        assert_eq!(pending.stale_optimistic_state_count, 1);
        assert_eq!(pending.state_refresh_target_count, 1);
        assert_eq!(pending.total_pending_work_count(), 5);
        assert!(pending.has_event_backlog());
        assert!(pending.has_supervision_pressure());
        assert!(!pending.is_idle());
        assert_eq!(worker.status, WorkerStatus::Starting);
        assert_eq!(worker.restart_count, 0);
        assert_eq!(
            runtime
                .pairing_session(&RuntimePairingSessionId::trusted("pairing-1"))
                .unwrap()
                .status,
            PairingSessionStatus::PendingUserPresence
        );
    }

    #[test]
    fn runtime_read_snapshot_pending_summary_reports_idle_runtime() {
        let runtime = SmartHomeRuntime::new();

        let snapshot = runtime.read_snapshot_at(1_000);
        let pending = snapshot.pending_work_summary();

        assert!(!snapshot.has_pending_work());
        assert!(pending.is_idle());
        assert_eq!(pending.total_pending_work_count(), 0);
        assert!(!pending.has_event_backlog());
        assert!(!pending.has_supervision_pressure());
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
    fn supervisor_builds_restart_plan_without_mutating_workers() {
        let mut supervisor = RuntimeSupervisor::new();
        let bridge_id = BridgeId::trusted("bridge-1");
        supervisor.register_worker(SupervisedBridgeWorker::new(
            bridge_id.clone(),
            IntegrationId::trusted("hue"),
            1_000,
            100,
        ));
        supervisor.register_worker(SupervisedBridgeWorker::new(
            BridgeId::trusted("bridge-2"),
            IntegrationId::trusted("thread"),
            1_000,
            500,
        ));

        let plan = supervisor.restart_plan_at(1_125);
        let worker = supervisor.worker(&bridge_id).unwrap();

        assert_eq!(plan.generated_at_ms, 1_125);
        assert_eq!(plan.len(), 1);
        assert!(!plan.is_empty());
        assert!(matches!(
            plan.instructions.as_slice(),
            [WorkerRestartInstruction {
                bridge_id: instruction_bridge_id,
                integration_id,
                reason: WorkerRestartReason::HeartbeatOverdue,
                status: WorkerStatus::Starting,
                last_heartbeat_at_ms: 1_000,
                heartbeat_timeout_ms: 100,
                due_at_ms: 1_100,
                planned_at_ms: 1_125,
                restart_attempt: 1,
            }] if instruction_bridge_id == &bridge_id
                && integration_id == &IntegrationId::trusted("hue")
        ));
        assert_eq!(plan.instructions[0].overdue_by_ms(), 25);
        assert_eq!(plan.instructions_for_bridge(&bridge_id).len(), 1);
        assert_eq!(worker.status, WorkerStatus::Starting);
        assert_eq!(worker.restart_count, 0);
    }

    #[test]
    fn supervisor_builds_heartbeat_schedule_without_mutating_workers() {
        let mut supervisor = RuntimeSupervisor::new();
        let bridge_early = BridgeId::trusted("bridge-early");
        let bridge_late = BridgeId::trusted("bridge-late");
        let bridge_stopped = BridgeId::trusted("bridge-stopped");
        supervisor.register_worker(SupervisedBridgeWorker::new(
            bridge_late.clone(),
            IntegrationId::trusted("thread"),
            1_200,
            600,
        ));
        supervisor.register_worker(SupervisedBridgeWorker::new(
            bridge_early.clone(),
            IntegrationId::trusted("hue"),
            1_000,
            500,
        ));
        supervisor.register_worker(SupervisedBridgeWorker {
            bridge_id: bridge_stopped.clone(),
            integration_id: IntegrationId::trusted("zwave"),
            status: WorkerStatus::Stopped,
            restart_count: 0,
            last_heartbeat_at_ms: 1_000,
            heartbeat_timeout_ms: 10,
        });

        let schedule = supervisor.heartbeat_schedule_at(1_400);

        assert_eq!(schedule.generated_at_ms, 1_400);
        assert_eq!(schedule.len(), 2);
        assert!(!schedule.is_empty());
        assert_eq!(schedule.next_due_at_ms(), Some(1_500));
        assert!(schedule.due_at(1_499).is_empty());
        assert_eq!(schedule.due_at(1_500).len(), 1);
        assert_eq!(schedule.deadlines[0].bridge_id, bridge_early);
        assert_eq!(schedule.deadlines[0].due_at_ms, 1_500);
        assert_eq!(schedule.deadlines[0].overdue_by_ms_at(1_525), 25);
        assert_eq!(schedule.deadlines[1].bridge_id, bridge_late);
        assert_eq!(schedule.deadlines_for_bridge(&bridge_stopped).len(), 0);
        assert_eq!(
            supervisor
                .worker(&BridgeId::trusted("bridge-early"))
                .unwrap()
                .status,
            WorkerStatus::Starting
        );
    }

    #[test]
    fn supervisor_queries_workers_by_status_deadline_and_restart_count() {
        let mut supervisor = RuntimeSupervisor::new();
        supervisor.register_worker(SupervisedBridgeWorker::new(
            BridgeId::trusted("bridge-1"),
            IntegrationId::trusted("hue"),
            1_000,
            100,
        ));
        supervisor.register_worker(SupervisedBridgeWorker {
            bridge_id: BridgeId::trusted("bridge-2"),
            integration_id: IntegrationId::trusted("zwave"),
            status: WorkerStatus::Restarting,
            restart_count: 2,
            last_heartbeat_at_ms: 900,
            heartbeat_timeout_ms: 500,
        });
        supervisor.register_worker(SupervisedBridgeWorker {
            bridge_id: BridgeId::trusted("bridge-3"),
            integration_id: IntegrationId::trusted("hue"),
            status: WorkerStatus::Running,
            restart_count: 1,
            last_heartbeat_at_ms: 1_100,
            heartbeat_timeout_ms: 500,
        });

        let overdue = supervisor.query_workers(
            &SupervisedWorkerQuery::new()
                .for_integration(IntegrationId::trusted("hue"))
                .overdue_at(1_125)
                .heartbeat_due_before(1_200)
                .sorted_by(SupervisedWorkerSort::HeartbeatDueAt),
        );
        assert_eq!(
            overdue
                .iter()
                .map(|worker| worker.bridge_id.as_str())
                .collect::<Vec<_>>(),
            vec!["bridge-1"]
        );

        let restarted = supervisor.query_workers(
            &SupervisedWorkerQuery::new()
                .min_restart_count(1)
                .sorted_by(SupervisedWorkerSort::RestartCountDesc),
        );
        assert_eq!(
            restarted
                .iter()
                .map(|worker| worker.bridge_id.as_str())
                .collect::<Vec<_>>(),
            vec!["bridge-2", "bridge-3"]
        );
    }

    #[test]
    fn runtime_exposes_worker_heartbeat_schedule() {
        let mut runtime = SmartHomeRuntime::new();
        let bridge_id = BridgeId::trusted("bridge-1");
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

        let schedule = runtime.worker_heartbeat_schedule_at(1_050);

        assert_eq!(schedule.next_due_at_ms(), Some(1_125));
        assert!(schedule.due_at(1_124).is_empty());
        assert_eq!(schedule.due_at(1_125).len(), 1);
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
        let plan = runtime.worker_restart_plan_at(1_126);
        assert!(matches!(
            plan.instructions.as_slice(),
            [WorkerRestartInstruction {
                bridge_id: instruction_bridge_id,
                restart_attempt: 1,
                ..
            }] if instruction_bridge_id == &bridge_id
        ));
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
    fn supervisor_restart_marks_registered_bridge_degraded() {
        let mut runtime = SmartHomeRuntime::new();
        let bridge_id = BridgeId::trusted("bridge-1");
        let subscription = RuntimeSubscriptionId::trusted("bridge-health");
        runtime.upsert_bridge(bridge("bridge-1")).unwrap();
        runtime
            .event_bus_mut()
            .subscribe(
                subscription.clone(),
                RuntimeEventFilter::Bridge(bridge_id.clone()),
            )
            .unwrap();
        runtime
            .supervisor_mut()
            .register_worker(SupervisedBridgeWorker::new(
                bridge_id.clone(),
                IntegrationId::trusted("hue"),
                1_000,
                100,
            ));

        let events = runtime.reconcile_supervision(1_100);
        let deliveries = runtime.event_bus_mut().drain(&subscription).unwrap();
        let bridge = runtime.registry().bridge(&bridge_id).unwrap();
        let health_event = runtime
            .registry()
            .event(&EventId::trusted(
                "supervision.restart.health:bridge-1:1100",
            ))
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(bridge.health, Health::Degraded);
        assert_eq!(runtime.registry().counts().events, 1);
        assert_eq!(health_event.event_type, DeviceEventType::Health);
        assert!(health_event
            .metadata
            .iter()
            .any(|metadata| metadata.key == "smart_home.supervision.reason"
                && metadata.value == "heartbeat_overdue"));
        assert!(deliveries.iter().any(|event| matches!(
            event,
            RuntimeEvent::BridgeHealth {
                health: Health::Degraded,
                ..
            }
        )));
        assert!(deliveries
            .iter()
            .any(|event| matches!(event, RuntimeEvent::WorkerNeedsRestart { .. })));
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
        let bridge = runtime.registry().bridge(&bridge_id).unwrap().clone();
        runtime
            .start_pairing_session(RuntimePairingSession::pending(
                RuntimePairingSessionId::trusted("pairing-1"),
                &bridge,
                AgentId::trusted("agent:installer"),
                1_000,
                1_050,
                Vec::new(),
            ))
            .unwrap();
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
        let summary = report.summary();
        let deliveries = runtime.event_bus_mut().drain(&subscription).unwrap();
        let worker = runtime.supervisor().worker(&bridge_id).unwrap();

        assert_eq!(report.ticked_at_ms, 1_050);
        assert_eq!(
            report.expired_pairing_sessions,
            vec![RuntimePairingSessionId::trusted("pairing-1")]
        );
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
        assert_eq!(report.action_count(), 4);
        assert!(!report.is_idle());
        assert_eq!(
            summary,
            SupervisionTickSummary {
                ticked_at_ms: 1_050,
                total_actions: 4,
                expired_pairing_session_count: 1,
                expired_entity_count: 1,
                desired_state_action_count: 1,
                desired_missing_state_count: 0,
                desired_stale_state_count: 1,
                desired_drifted_state_count: 0,
                worker_restart_event_count: 1,
            }
        );
        assert!(!summary.is_idle());
        assert!(summary.has_pairing_expiry_work());
        assert!(summary.has_state_expiry_work());
        assert!(summary.has_reconciliation_work());
        assert!(summary.has_worker_restart_work());
        assert_eq!(worker.status, WorkerStatus::Restarting);
        assert_eq!(worker.restart_count, 1);
        assert_eq!(deliveries.len(), 2);
        assert_eq!(
            runtime
                .pairing_session(&RuntimePairingSessionId::trusted("pairing-1"))
                .unwrap()
                .status,
            PairingSessionStatus::Expired
        );
    }

    #[test]
    fn supervision_tick_reports_idle_when_no_work_is_due() {
        let mut runtime = runtime_with_entity(vec![Capability::light_on_off()]);

        let report = runtime.run_supervision_tick(1_000).unwrap();

        assert_eq!(report.ticked_at_ms, 1_000);
        assert!(report.is_idle());
        assert_eq!(report.action_count(), 0);
        assert!(report.summary().is_idle());
    }
}
