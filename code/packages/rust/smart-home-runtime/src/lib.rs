//! Deterministic smart-home runtime coordinator.
//!
//! This crate is the first runtime slice above the normalized D23 model. It is
//! intentionally synchronous: actors, transports, and protocol workers can wrap
//! it later, while command validation, event routing, state confidence, and
//! supervision rules remain easy to test.

#![forbid(unsafe_code)]

use smart_home_core::{
    tier_for_command, AgentId, AuthorizationDecision, Bridge, BridgeId, Capability,
    CapabilityGrant, CapabilityGrantScope, CapabilityId, CapabilityMode, CommandId, CommandResult,
    CommandStatus, CommandType, CorrelationId, Device, DeviceCommand, DeviceEvent, DeviceEventType,
    DeviceId, Entity, EntityId, EventId, Health, IntegrationId, Metadata, PrivilegeTier,
    SmartHomeError, SmartHomeTool, StateConfidence, StateDelta, StateSnapshot, StateSource, Value,
    VaultRef,
};
use smart_home_registry::{
    DeviceSelector, InMemorySmartHomeRegistry, RegistryCounts, RegistryError, StateRefreshPlan,
    StateRefreshReason,
};
use std::collections::{BTreeMap, VecDeque};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    Registry(Box<RegistryError>),
    Core(Box<SmartHomeError>),
    UnknownBridge(BridgeId),
    UnknownDevice(DeviceId),
    UnknownEntity(EntityId),
    UnknownPairingSession(RuntimePairingSessionId),
    UnknownSubscription(RuntimeSubscriptionId),
    DuplicatePairingSession(RuntimePairingSessionId),
    DuplicateSubscription(RuntimeSubscriptionId),
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
            Self::UnknownBridge(id) => write!(f, "unknown runtime bridge {id}"),
            Self::UnknownDevice(id) => write!(f, "unknown runtime device {id}"),
            Self::UnknownEntity(id) => write!(f, "unknown runtime entity {id}"),
            Self::UnknownPairingSession(id) => write!(f, "unknown runtime pairing session {id}"),
            Self::UnknownSubscription(id) => write!(f, "unknown runtime subscription {id}"),
            Self::DuplicatePairingSession(id) => write!(f, "duplicate runtime pairing session {id}"),
            Self::DuplicateSubscription(id) => write!(f, "duplicate runtime subscription {id}"),
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEventSort {
    SequenceAsc,
    SequenceDesc,
}

impl Default for RuntimeEventSort {
    fn default() -> Self {
        Self::SequenceAsc
    }
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

/// Read-side query for the runtime event log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeEventQuery {
    pub filter: Option<RuntimeEventFilter>,
    pub from_checkpoint: RuntimeEventCheckpoint,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeSubscriptionSort {
    SubscriptionId,
    QueuedEventsDesc,
}

impl Default for RuntimeSubscriptionSort {
    fn default() -> Self {
        Self::SubscriptionId
    }
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
pub enum SupervisedWorkerSort {
    BridgeId,
    HeartbeatDueAt,
    RestartCountDesc,
    StatusThenBridgeId,
}

impl Default for SupervisedWorkerSort {
    fn default() -> Self {
        Self::BridgeId
    }
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
        let mut snapshot = RuntimeSupervisorSnapshot {
            generated_at_ms: now_ms,
            ..RuntimeSupervisorSnapshot::default()
        };
        for worker in self.workers.values() {
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
    pub fn has_restart_pressure(&self) -> bool {
        self.restart_due_count > 0 || self.unhealthy_count > 0
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesiredStateSort {
    EntityId,
    RequestedByThenEntityId,
    CommandTimeoutDesc,
}

impl Default for DesiredStateSort {
    fn default() -> Self {
        Self::EntityId
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
pub enum RuntimePairingSessionSort {
    SessionId,
    ExpiresAt,
    StartedAtDesc,
    StatusThenExpiresAt,
}

impl Default for RuntimePairingSessionSort {
    fn default() -> Self {
        Self::SessionId
    }
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
}

impl RuntimeSupervisionPlan {
    pub fn is_empty(&self) -> bool {
        self.pairing_sessions_expiring.is_empty()
            && self.state_refresh_plan.is_empty()
            && self.desired_state_drifts.is_empty()
            && self.worker_restart_plan.is_empty()
    }

    pub fn action_count(&self) -> usize {
        self.pairing_sessions_expiring.len()
            + self.state_refresh_plan.len()
            + self.desired_state_drifts.len()
            + self.worker_restart_plan.len()
    }

    pub fn summary(&self) -> RuntimeSupervisionPlanSummary {
        let mut summary = RuntimeSupervisionPlanSummary {
            generated_at_ms: self.generated_at_ms,
            total_actions: self.action_count(),
            pairing_expiry_count: self.pairing_sessions_expiring.len(),
            state_refresh_count: self.state_refresh_plan.len(),
            desired_state_drift_count: self.desired_state_drifts.len(),
            worker_restart_count: self.worker_restart_plan.len(),
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeReadSnapshot {
    pub generated_at_ms: u64,
    pub registry_counts: RegistryCounts,
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
            || self.expiring_pairing_session_count > 0
            || self.stale_optimistic_state_count > 0
            || self.state_refresh_target_count > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeReadToolRequest {
    ListBridges,
    ListDevices {
        bridge_id: Option<BridgeId>,
        health: Option<Health>,
        capability_id: Option<CapabilityId>,
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
    ObserveSupervision,
}

impl RuntimeReadToolRequest {
    pub fn tool(&self) -> SmartHomeTool {
        match self {
            Self::ListBridges => SmartHomeTool::ListBridges,
            Self::ListDevices { .. } => SmartHomeTool::ListDevices,
            Self::GetState { .. } => SmartHomeTool::GetState,
            Self::DescribeCapabilities { .. } => SmartHomeTool::DescribeCapabilities,
            Self::GetHealth { .. } => SmartHomeTool::GetHealth,
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
pub enum RuntimeReadToolOutput {
    Bridges(Vec<Bridge>),
    Devices(Vec<Device>),
    State {
        entity_id: EntityId,
        snapshot: Option<StateSnapshot>,
    },
    Capabilities {
        entity_id: EntityId,
        capabilities: Vec<Capability>,
    },
    Health(Vec<BridgeHealthSnapshot>),
    SupervisionObservation(RuntimeSupervisionObservation),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSubscribeToolOutput {
    pub subscription_id: RuntimeSubscriptionId,
    pub replay_from_checkpoint: RuntimeEventCheckpoint,
    pub subscribed_at_checkpoint: RuntimeEventCheckpoint,
    pub queued_events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePairBridgeToolOutput {
    pub session: RuntimePairingSession,
}

#[derive(Debug, Clone)]
pub struct SmartHomeRuntime {
    registry: InMemorySmartHomeRegistry,
    event_bus: RuntimeEventBus,
    supervisor: RuntimeSupervisor,
    pairing_sessions: BTreeMap<RuntimePairingSessionId, RuntimePairingSession>,
    optimistic_states: BTreeMap<EntityId, StateSnapshot>,
    desired_states: BTreeMap<EntityId, DesiredEntityState>,
}

impl SmartHomeRuntime {
    pub fn new() -> Self {
        Self {
            registry: InMemorySmartHomeRegistry::new(),
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

    pub fn complete_pairing_session(
        &mut self,
        session_id: &RuntimePairingSessionId,
        vault_ref: VaultRef,
        completed_at_ms: u64,
    ) -> Result<RuntimePairingSession, RuntimeError> {
        let session = self
            .pairing_sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| RuntimeError::UnknownPairingSession(session_id.clone()))?;
        if session.status != PairingSessionStatus::PendingUserPresence {
            return Err(RuntimeError::PairingSessionNotPending {
                session_id: session_id.clone(),
                status: session.status,
            });
        }
        if completed_at_ms >= session.expires_at_ms {
            let mut expired = session.clone();
            expired.status = PairingSessionStatus::Expired;
            self.pairing_sessions
                .insert(session_id.clone(), expired.clone());
            return Err(RuntimeError::PairingSessionExpired {
                session_id: session_id.clone(),
                expired_at_ms: session.expires_at_ms,
                now_ms: completed_at_ms,
            });
        }

        let mut completed = session;
        completed.status = PairingSessionStatus::Completed;
        completed.vault_ref = Some(vault_ref.clone());
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
        self.apply_bridge_health(BridgeHealthReport {
            event_id: EventId::trusted(format!(
                "pairing.completed.health:{}:{completed_at_ms}",
                completed.bridge_id.as_str()
            )),
            bridge_id: completed.bridge_id.clone(),
            health: Health::Online,
            observed_at_ms: completed_at_ms,
            received_at_ms: completed_at_ms,
            metadata: vec![Metadata::new(
                "smart_home.pairing_session",
                completed.session_id.as_str(),
            )],
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
            RuntimeReadToolRequest::ObserveSupervision => Ok(
                RuntimeReadToolOutput::SupervisionObservation(self.observe_supervision_at(now_ms)?),
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

    pub fn execute_command_tool(
        &mut self,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, RuntimeError> {
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

        self.submit_authorized_command(&authorization, command, now_ms)
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
        AuthorizationOutcome, BridgeTransport, Capability, CapabilityGrantId, CommandId,
        CorrelationId, EntityKind, IntegrationId, ProtocolFamily, ProtocolIdentifier, StateDelta,
    };
    use smart_home_registry::StateRefreshReason;

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

        let empty = bus.event_log_summary(&RuntimeEventQuery::new().with_limit(0));
        assert_eq!(empty, RuntimeEventLogSummary::empty());
        assert!(!empty.has_events());
        assert!(!empty.has_command_results());
        assert!(!empty.has_supervision_events());
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
        assert_eq!(bus.queued_events(&subscription).unwrap(), 3);

        let drained = bus
            .drain_deliveries(
                &subscription,
                RuntimeEventDeliveryOptions::new().with_limit(2),
            )
            .unwrap();
        assert_eq!(drained.len(), 2);
        assert_eq!(drained.remaining_events, 1);
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
    fn pair_bridge_tool_requires_pair_grants_before_starting_session() {
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
            .complete_pairing_session(
                &session_id,
                VaultRef::trusted("vault://smart-home/hue/bridge-1/app-key"),
                1_200,
            )
            .unwrap();
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
        let state = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::GetState {
                    entity_id: EntityId::trusted("entity-1"),
                },
                1_502,
            )
            .unwrap();
        let capabilities = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::DescribeCapabilities {
                    entity_id: EntityId::trusted("entity-1"),
                },
                1_503,
            )
            .unwrap();
        let health = runtime
            .execute_read_tool(
                principal.clone(),
                RuntimeReadToolRequest::GetHealth {
                    bridge_id: Some(BridgeId::trusted("bridge-1")),
                },
                1_504,
            )
            .unwrap();
        let observation = runtime
            .execute_read_tool(principal, RuntimeReadToolRequest::ObserveSupervision, 1_505)
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
            observation,
            RuntimeReadToolOutput::SupervisionObservation(observation)
                if observation.generated_at_ms == 1_505
                    && observation.worker_restart_count() == 1
                    && observation.due_worker_deadline_count() == 1
                    && observation.next_worker_heartbeat_due_at_ms() == Some(1_100)
        ));
        assert_eq!(runtime.registry().counts().authorization_decisions, 6);
        assert!(runtime
            .registry()
            .authorization_decisions()
            .all(|decision| decision.outcome == AuthorizationOutcome::Allowed));
        assert!(runtime.event_bus().published().is_empty());
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
