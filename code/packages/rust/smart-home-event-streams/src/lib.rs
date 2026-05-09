//! Deterministic smart-home event stream cursor and supervision primitives.
//!
//! This crate is intentionally transport-neutral. Hue SSE, ESPHome-style
//! WebSocket workers, MQTT subscriptions, cloud push callbacks, serial frames,
//! and radio report loops can all share the same cursor, heartbeat, and
//! reconnect rules while keeping protocol-specific I/O in adapter crates.

#![forbid(unsafe_code)]

use smart_home_core::{BridgeId, EventId, IntegrationId, Metadata};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventStreamId(String);

impl EventStreamId {
    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn for_bridge(integration_id: &IntegrationId, bridge_id: &BridgeId) -> Self {
        Self(format!(
            "{}:{}",
            integration_id.as_str(),
            bridge_id.as_str()
        ))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for EventStreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventStreamTransport {
    ServerSentEvents,
    WebSocket,
    MqttSubscription,
    CloudWebhook,
    SerialFrames,
    RadioReports,
}

impl EventStreamTransport {
    pub fn is_local(self) -> bool {
        !matches!(self, Self::CloudWebhook)
    }

    pub fn is_push(self) -> bool {
        true
    }

    pub fn needs_cursor(self) -> bool {
        matches!(
            self,
            Self::ServerSentEvents
                | Self::WebSocket
                | Self::MqttSubscription
                | Self::SerialFrames
                | Self::RadioReports
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MqttTopicError {
    EmptyFilter,
    HashWildcardMustBeFinal,
    PlusWildcardMustOccupyLevel,
    TopicNameContainsWildcard,
}

impl fmt::Display for MqttTopicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFilter => write!(f, "MQTT topic filter must not be empty"),
            Self::HashWildcardMustBeFinal => {
                write!(f, "MQTT # wildcard must occupy the final topic level")
            }
            Self::PlusWildcardMustOccupyLevel => {
                write!(f, "MQTT + wildcard must occupy an entire topic level")
            }
            Self::TopicNameContainsWildcard => write!(f, "MQTT topic names must not use wildcards"),
        }
    }
}

impl std::error::Error for MqttTopicError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MqttQos {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl MqttQos {
    pub fn level(self) -> u8 {
        match self {
            Self::AtMostOnce => 0,
            Self::AtLeastOnce => 1,
            Self::ExactlyOnce => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MqttTopicFilter(String);

impl MqttTopicFilter {
    pub fn new(value: impl Into<String>) -> Result<Self, MqttTopicError> {
        let value = value.into();
        validate_mqtt_topic_filter(&value)?;
        Ok(Self(value))
    }

    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn has_wildcards(&self) -> bool {
        self.0.split('/').any(|level| level == "+" || level == "#")
    }

    pub fn matches_topic(&self, topic_name: &str) -> Result<bool, MqttTopicError> {
        validate_mqtt_topic_name(topic_name)?;

        let filter_levels: Vec<&str> = self.0.split('/').collect();
        let topic_levels: Vec<&str> = topic_name.split('/').collect();
        let mut topic_index = 0;

        for filter_level in filter_levels {
            if filter_level == "#" {
                return Ok(true);
            }

            let Some(topic_level) = topic_levels.get(topic_index) else {
                return Ok(false);
            };

            if filter_level != "+" && filter_level != *topic_level {
                return Ok(false);
            }

            topic_index += 1;
        }

        Ok(topic_index == topic_levels.len())
    }
}

impl fmt::Display for MqttTopicFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttRetainPolicy {
    DeliverRetained,
    IgnoreRetained,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttSubscriptionSpec {
    pub integration_id: IntegrationId,
    pub broker_id: BridgeId,
    pub stream_id: EventStreamId,
    pub topic_filter: MqttTopicFilter,
    pub qos: MqttQos,
    pub retain_policy: MqttRetainPolicy,
    pub shared_group: Option<String>,
    pub metadata: Vec<Metadata>,
}

impl MqttSubscriptionSpec {
    pub fn new(
        integration_id: IntegrationId,
        broker_id: BridgeId,
        topic_filter: MqttTopicFilter,
    ) -> Self {
        let stream_id = EventStreamId::trusted(format!(
            "{}:{}:{}",
            integration_id.as_str(),
            broker_id.as_str(),
            topic_filter.as_str()
        ));
        Self {
            integration_id,
            broker_id,
            stream_id,
            topic_filter,
            qos: MqttQos::AtLeastOnce,
            retain_policy: MqttRetainPolicy::DeliverRetained,
            shared_group: None,
            metadata: Vec::new(),
        }
    }

    pub fn with_qos(mut self, qos: MqttQos) -> Self {
        self.qos = qos;
        self
    }

    pub fn with_retain_policy(mut self, retain_policy: MqttRetainPolicy) -> Self {
        self.retain_policy = retain_policy;
        self
    }

    pub fn with_shared_group(mut self, shared_group: impl Into<String>) -> Self {
        self.shared_group = Some(shared_group.into());
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.push(metadata);
        self
    }

    pub fn to_event_stream_spec(&self) -> EventStreamSpec {
        let mut spec = EventStreamSpec::new(
            self.integration_id.clone(),
            self.broker_id.clone(),
            EventStreamTransport::MqttSubscription,
        )
        .with_endpoint(format!("mqtt:{}", self.topic_filter.as_str()))
        .with_metadata(Metadata::new(
            "mqtt.topic_filter",
            self.topic_filter.as_str(),
        ))
        .with_metadata(Metadata::new("mqtt.qos", self.qos.level().to_string()))
        .with_metadata(Metadata::new(
            "mqtt.retain_policy",
            match self.retain_policy {
                MqttRetainPolicy::DeliverRetained => "deliver",
                MqttRetainPolicy::IgnoreRetained => "ignore",
            },
        ));

        if let Some(shared_group) = &self.shared_group {
            spec = spec.with_metadata(Metadata::new("mqtt.shared_group", shared_group));
        }

        for metadata in &self.metadata {
            spec = spec.with_metadata(metadata.clone());
        }

        spec.stream_id = self.stream_id.clone();
        spec
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttPublicationRef {
    pub topic_name: String,
    pub qos: MqttQos,
    pub packet_id: Option<u16>,
    pub retained: bool,
    pub duplicate: bool,
}

impl MqttPublicationRef {
    pub fn new(topic_name: impl Into<String>, qos: MqttQos) -> Result<Self, MqttTopicError> {
        let topic_name = topic_name.into();
        validate_mqtt_topic_name(&topic_name)?;
        Ok(Self {
            topic_name,
            qos,
            packet_id: None,
            retained: false,
            duplicate: false,
        })
    }

    pub fn with_packet_id(mut self, packet_id: u16) -> Self {
        self.packet_id = Some(packet_id);
        self
    }

    pub fn retained(mut self, retained: bool) -> Self {
        self.retained = retained;
        self
    }

    pub fn duplicate(mut self, duplicate: bool) -> Self {
        self.duplicate = duplicate;
        self
    }

    pub fn native_cursor(&self) -> String {
        format!(
            "mqtt:{}:qos{}:packet:{}:retained:{}:duplicate:{}",
            self.topic_name,
            self.qos.level(),
            self.packet_id
                .map(|packet_id| packet_id.to_string())
                .unwrap_or_else(|| "-".to_string()),
            self.retained,
            self.duplicate
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventStreamStatus {
    Idle,
    Connecting,
    Healthy,
    Degraded,
    Disconnected,
    BackingOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventStreamRestartReason {
    HeartbeatOverdue,
    ExplicitDisconnect,
    EventGap,
    StaleEvents,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamCursor {
    pub sequence: u64,
    pub native_cursor: Option<String>,
    pub last_event_id: Option<EventId>,
    pub observed_at_ms: u64,
}

impl EventStreamCursor {
    pub fn start(observed_at_ms: u64) -> Self {
        Self {
            sequence: 0,
            native_cursor: None,
            last_event_id: None,
            observed_at_ms,
        }
    }

    pub fn advance(
        &self,
        event_id: EventId,
        native_cursor: Option<String>,
        observed_at_ms: u64,
    ) -> Self {
        Self {
            sequence: self.sequence.saturating_add(1),
            native_cursor,
            last_event_id: Some(event_id),
            observed_at_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamCheckpoint {
    pub stream_id: EventStreamId,
    pub cursor: EventStreamCursor,
}

impl EventStreamCheckpoint {
    pub fn new(stream_id: EventStreamId, cursor: EventStreamCursor) -> Self {
        Self { stream_id, cursor }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub multiplier: u8,
}

impl ReconnectPolicy {
    pub fn new(initial_backoff_ms: u64, max_backoff_ms: u64, multiplier: u8) -> Self {
        Self {
            initial_backoff_ms,
            max_backoff_ms,
            multiplier: multiplier.max(1),
        }
    }

    pub fn delay_for_attempt(self, attempt: u32) -> u64 {
        let mut delay = self.initial_backoff_ms.max(1);
        for _ in 0..attempt {
            delay = delay.saturating_mul(self.multiplier as u64);
            if delay >= self.max_backoff_ms {
                return self.max_backoff_ms.max(1);
            }
        }
        delay.min(self.max_backoff_ms.max(1))
    }
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self::new(500, 30_000, 2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamSpec {
    pub stream_id: EventStreamId,
    pub integration_id: IntegrationId,
    pub bridge_id: BridgeId,
    pub transport: EventStreamTransport,
    pub endpoint: Option<String>,
    pub heartbeat_timeout_ms: u64,
    pub stale_after_ms: u64,
    pub reconnect_policy: ReconnectPolicy,
    pub metadata: Vec<Metadata>,
}

impl EventStreamSpec {
    pub fn new(
        integration_id: IntegrationId,
        bridge_id: BridgeId,
        transport: EventStreamTransport,
    ) -> Self {
        let stream_id = EventStreamId::for_bridge(&integration_id, &bridge_id);
        Self {
            stream_id,
            integration_id,
            bridge_id,
            transport,
            endpoint: None,
            heartbeat_timeout_ms: 30_000,
            stale_after_ms: 120_000,
            reconnect_policy: ReconnectPolicy::default(),
            metadata: Vec::new(),
        }
    }

    pub fn hue_sse(bridge_id: BridgeId, endpoint: impl Into<String>) -> Self {
        Self::new(
            IntegrationId::trusted("hue"),
            bridge_id,
            EventStreamTransport::ServerSentEvents,
        )
        .with_endpoint(endpoint)
        .with_metadata(Metadata::new("http.accept", "text/event-stream"))
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self
    }

    pub fn with_heartbeat_timeout(mut self, heartbeat_timeout_ms: u64) -> Self {
        self.heartbeat_timeout_ms = heartbeat_timeout_ms;
        self
    }

    pub fn with_stale_after(mut self, stale_after_ms: u64) -> Self {
        self.stale_after_ms = stale_after_ms;
        self
    }

    pub fn with_reconnect_policy(mut self, reconnect_policy: ReconnectPolicy) -> Self {
        self.reconnect_policy = reconnect_policy;
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.push(metadata);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamState {
    pub spec: EventStreamSpec,
    pub status: EventStreamStatus,
    pub cursor: EventStreamCursor,
    pub connected_at_ms: Option<u64>,
    pub last_heartbeat_at_ms: Option<u64>,
    pub last_disconnect_at_ms: Option<u64>,
    pub reconnect_attempt: u32,
    pub pending_gap_count: u32,
}

impl EventStreamState {
    pub fn new(spec: EventStreamSpec, now_ms: u64) -> Self {
        Self {
            spec,
            status: EventStreamStatus::Idle,
            cursor: EventStreamCursor::start(now_ms),
            connected_at_ms: None,
            last_heartbeat_at_ms: None,
            last_disconnect_at_ms: None,
            reconnect_attempt: 0,
            pending_gap_count: 0,
        }
    }

    pub fn checkpoint(&self) -> EventStreamCheckpoint {
        EventStreamCheckpoint::new(self.spec.stream_id.clone(), self.cursor.clone())
    }

    pub fn mark_connecting(&mut self) {
        self.status = EventStreamStatus::Connecting;
    }

    pub fn mark_connected(&mut self, now_ms: u64) {
        self.status = EventStreamStatus::Healthy;
        self.connected_at_ms = Some(now_ms);
        self.last_heartbeat_at_ms = Some(now_ms);
        self.last_disconnect_at_ms = None;
        self.reconnect_attempt = 0;
    }

    pub fn mark_heartbeat(&mut self, now_ms: u64) {
        self.last_heartbeat_at_ms = Some(now_ms);
        if matches!(
            self.status,
            EventStreamStatus::Connecting | EventStreamStatus::Degraded
        ) {
            self.status = EventStreamStatus::Healthy;
        }
    }

    pub fn record_event(
        &mut self,
        event_id: EventId,
        native_cursor: Option<String>,
        observed_at_ms: u64,
    ) -> EventStreamCheckpoint {
        self.cursor = self.cursor.advance(event_id, native_cursor, observed_at_ms);
        self.last_heartbeat_at_ms = Some(observed_at_ms);
        self.pending_gap_count = 0;
        self.status = EventStreamStatus::Healthy;
        self.checkpoint()
    }

    pub fn record_gap(&mut self, missing_events: u32, observed_at_ms: u64) {
        self.pending_gap_count = self.pending_gap_count.saturating_add(missing_events);
        self.last_heartbeat_at_ms = Some(observed_at_ms);
        self.status = EventStreamStatus::Degraded;
    }

    pub fn mark_disconnected(&mut self, now_ms: u64) {
        self.status = EventStreamStatus::Disconnected;
        self.last_disconnect_at_ms = Some(now_ms);
        self.reconnect_attempt = self.reconnect_attempt.saturating_add(1);
    }

    pub fn heartbeat_overdue_at(&self, now_ms: u64) -> bool {
        let Some(last_heartbeat_at_ms) = self.last_heartbeat_at_ms else {
            return matches!(
                self.status,
                EventStreamStatus::Connecting | EventStreamStatus::Healthy
            );
        };
        now_ms >= last_heartbeat_at_ms.saturating_add(self.spec.heartbeat_timeout_ms)
    }

    pub fn stale_at(&self, now_ms: u64) -> bool {
        now_ms
            >= self
                .cursor
                .observed_at_ms
                .saturating_add(self.spec.stale_after_ms)
    }

    pub fn next_retry_at_ms(&self) -> Option<u64> {
        self.last_disconnect_at_ms.map(|disconnect| {
            disconnect.saturating_add(
                self.spec
                    .reconnect_policy
                    .delay_for_attempt(self.reconnect_attempt.saturating_sub(1)),
            )
        })
    }

    pub fn ready_to_reconnect_at(&self, now_ms: u64) -> bool {
        matches!(
            self.status,
            EventStreamStatus::Disconnected | EventStreamStatus::BackingOff
        ) && self
            .next_retry_at_ms()
            .is_some_and(|retry_at| now_ms >= retry_at)
    }

    pub fn restart_plan_at(&self, now_ms: u64) -> Option<EventStreamRestartPlan> {
        let reason = if self.pending_gap_count > 0 {
            EventStreamRestartReason::EventGap
        } else if self.heartbeat_overdue_at(now_ms) {
            EventStreamRestartReason::HeartbeatOverdue
        } else if self.status == EventStreamStatus::Disconnected {
            EventStreamRestartReason::ExplicitDisconnect
        } else if self.stale_at(now_ms) {
            EventStreamRestartReason::StaleEvents
        } else {
            return None;
        };

        let attempt = if self.status == EventStreamStatus::Disconnected {
            self.reconnect_attempt
        } else {
            self.reconnect_attempt.saturating_add(1)
        };
        let backoff_ms = self
            .spec
            .reconnect_policy
            .delay_for_attempt(attempt.saturating_sub(1));
        Some(EventStreamRestartPlan {
            stream_id: self.spec.stream_id.clone(),
            integration_id: self.spec.integration_id.clone(),
            bridge_id: self.spec.bridge_id.clone(),
            reason,
            status: self.status,
            checkpoint: self.checkpoint(),
            planned_at_ms: now_ms,
            reconnect_attempt: attempt,
            backoff_ms,
            retry_at_ms: now_ms.saturating_add(backoff_ms),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamRestartPlan {
    pub stream_id: EventStreamId,
    pub integration_id: IntegrationId,
    pub bridge_id: BridgeId,
    pub reason: EventStreamRestartReason,
    pub status: EventStreamStatus,
    pub checkpoint: EventStreamCheckpoint,
    pub planned_at_ms: u64,
    pub reconnect_attempt: u32,
    pub backoff_ms: u64,
    pub retry_at_ms: u64,
}

fn validate_mqtt_topic_filter(value: &str) -> Result<(), MqttTopicError> {
    if value.is_empty() {
        return Err(MqttTopicError::EmptyFilter);
    }

    for level in value.split('/') {
        if level.contains('#') && level != "#" {
            return Err(MqttTopicError::HashWildcardMustBeFinal);
        }
        if level.contains('+') && level != "+" {
            return Err(MqttTopicError::PlusWildcardMustOccupyLevel);
        }
    }

    if let Some(hash_index) = value.split('/').position(|level| level == "#") {
        let level_count = value.split('/').count();
        if hash_index + 1 != level_count {
            return Err(MqttTopicError::HashWildcardMustBeFinal);
        }
    }

    Ok(())
}

fn validate_mqtt_topic_name(value: &str) -> Result<(), MqttTopicError> {
    if value.is_empty() {
        return Err(MqttTopicError::EmptyFilter);
    }
    if value.contains('#') || value.contains('+') {
        return Err(MqttTopicError::TopicNameContainsWildcard);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bridge_id() -> BridgeId {
        BridgeId::trusted("bridge-1")
    }

    #[test]
    fn hue_sse_spec_records_event_stream_shape() {
        let spec = EventStreamSpec::hue_sse(bridge_id(), "https://bridge/api/eventstream/clip/v2");

        assert_eq!(spec.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(spec.transport, EventStreamTransport::ServerSentEvents);
        assert!(spec.transport.is_local());
        assert!(spec.transport.needs_cursor());
        assert_eq!(spec.heartbeat_timeout_ms, 30_000);
        assert_eq!(
            spec.metadata,
            vec![Metadata::new("http.accept", "text/event-stream")]
        );
    }

    #[test]
    fn reconnect_policy_uses_bounded_exponential_backoff() {
        let policy = ReconnectPolicy::new(250, 2_000, 2);

        assert_eq!(policy.delay_for_attempt(0), 250);
        assert_eq!(policy.delay_for_attempt(1), 500);
        assert_eq!(policy.delay_for_attempt(2), 1_000);
        assert_eq!(policy.delay_for_attempt(3), 2_000);
        assert_eq!(policy.delay_for_attempt(10), 2_000);
    }

    #[test]
    fn events_advance_cursor_and_clear_gaps() {
        let spec = EventStreamSpec::hue_sse(bridge_id(), "https://bridge/eventstream");
        let mut state = EventStreamState::new(spec, 1_000);
        state.mark_connected(1_100);
        state.record_gap(3, 1_200);

        let checkpoint = state.record_event(
            EventId::trusted("event-1"),
            Some("native:42".to_string()),
            1_300,
        );

        assert_eq!(state.status, EventStreamStatus::Healthy);
        assert_eq!(state.pending_gap_count, 0);
        assert_eq!(checkpoint.cursor.sequence, 1);
        assert_eq!(
            checkpoint.cursor.native_cursor,
            Some("native:42".to_string())
        );
        assert_eq!(
            checkpoint.cursor.last_event_id,
            Some(EventId::trusted("event-1"))
        );
        assert_eq!(state.last_heartbeat_at_ms, Some(1_300));
    }

    #[test]
    fn heartbeat_overdue_produces_restart_plan_with_checkpoint() {
        let spec = EventStreamSpec::hue_sse(bridge_id(), "https://bridge/eventstream")
            .with_heartbeat_timeout(1_000)
            .with_reconnect_policy(ReconnectPolicy::new(100, 1_000, 2));
        let mut state = EventStreamState::new(spec, 1_000);
        state.mark_connected(1_000);
        state.record_event(EventId::trusted("event-1"), None, 1_100);

        assert!(state.restart_plan_at(1_999).is_none());
        let plan = state.restart_plan_at(2_100).unwrap();

        assert_eq!(plan.reason, EventStreamRestartReason::HeartbeatOverdue);
        assert_eq!(plan.reconnect_attempt, 1);
        assert_eq!(plan.backoff_ms, 100);
        assert_eq!(plan.retry_at_ms, 2_200);
        assert_eq!(plan.checkpoint.cursor.sequence, 1);
        assert_eq!(
            plan.checkpoint.cursor.last_event_id,
            Some(EventId::trusted("event-1"))
        );
    }

    #[test]
    fn disconnect_tracks_retry_window_without_losing_cursor() {
        let spec = EventStreamSpec::new(
            IntegrationId::trusted("esphome"),
            bridge_id(),
            EventStreamTransport::WebSocket,
        )
        .with_reconnect_policy(ReconnectPolicy::new(500, 5_000, 3));
        let mut state = EventStreamState::new(spec, 1_000);
        state.mark_connected(1_000);
        state.record_event(
            EventId::trusted("event-1"),
            Some("frame:9".to_string()),
            1_100,
        );
        state.mark_disconnected(1_200);

        assert_eq!(state.status, EventStreamStatus::Disconnected);
        assert_eq!(state.next_retry_at_ms(), Some(1_700));
        assert!(!state.ready_to_reconnect_at(1_699));
        assert!(state.ready_to_reconnect_at(1_700));

        let plan = state.restart_plan_at(1_300).unwrap();
        assert_eq!(plan.reason, EventStreamRestartReason::ExplicitDisconnect);
        assert_eq!(
            plan.checkpoint.cursor.native_cursor,
            Some("frame:9".to_string())
        );
    }

    #[test]
    fn event_gap_takes_priority_for_restart_reason() {
        let spec = EventStreamSpec::new(
            IntegrationId::trusted("mqtt"),
            BridgeId::trusted("broker-1"),
            EventStreamTransport::MqttSubscription,
        );
        let mut state = EventStreamState::new(spec, 1_000);
        state.mark_connected(1_000);
        state.record_gap(2, 1_100);

        let plan = state.restart_plan_at(1_101).unwrap();

        assert_eq!(plan.reason, EventStreamRestartReason::EventGap);
        assert_eq!(plan.bridge_id, BridgeId::trusted("broker-1"));
        assert_eq!(plan.reconnect_attempt, 1);
    }

    #[test]
    fn mqtt_topic_filters_validate_wildcard_placement() {
        assert_eq!(MqttTopicFilter::new("home/#").unwrap().as_str(), "home/#");
        assert!(MqttTopicFilter::new("home/+/state")
            .unwrap()
            .has_wildcards());
        assert_eq!(
            MqttTopicFilter::new("home/#/state").unwrap_err(),
            MqttTopicError::HashWildcardMustBeFinal
        );
        assert_eq!(
            MqttTopicFilter::new("home/room+1/state").unwrap_err(),
            MqttTopicError::PlusWildcardMustOccupyLevel
        );
    }

    #[test]
    fn mqtt_topic_filters_match_topic_names() {
        let exact = MqttTopicFilter::new("zigbee2mqtt/kitchen_light").unwrap();
        let room_states = MqttTopicFilter::new("home/+/state").unwrap();
        let subtree = MqttTopicFilter::new("home/#").unwrap();

        assert!(exact.matches_topic("zigbee2mqtt/kitchen_light").unwrap());
        assert!(!exact.matches_topic("zigbee2mqtt/hall_light").unwrap());
        assert!(room_states.matches_topic("home/kitchen/state").unwrap());
        assert!(!room_states
            .matches_topic("home/kitchen/light/state")
            .unwrap());
        assert!(subtree.matches_topic("home").unwrap());
        assert!(subtree.matches_topic("home/kitchen/light/state").unwrap());
        assert_eq!(
            subtree.matches_topic("home/+/state").unwrap_err(),
            MqttTopicError::TopicNameContainsWildcard
        );
    }

    #[test]
    fn mqtt_subscription_specs_project_to_event_stream_specs() {
        let subscription = MqttSubscriptionSpec::new(
            IntegrationId::trusted("zigbee2mqtt"),
            BridgeId::trusted("broker-1"),
            MqttTopicFilter::new("zigbee2mqtt/+/availability").unwrap(),
        )
        .with_qos(MqttQos::AtMostOnce)
        .with_retain_policy(MqttRetainPolicy::IgnoreRetained)
        .with_shared_group("chief-of-staff")
        .with_metadata(Metadata::new("mqtt.discovery_family", "availability"));

        let spec = subscription.to_event_stream_spec();

        assert_eq!(spec.stream_id, subscription.stream_id);
        assert_eq!(spec.transport, EventStreamTransport::MqttSubscription);
        assert_eq!(
            spec.endpoint.as_deref(),
            Some("mqtt:zigbee2mqtt/+/availability")
        );
        assert!(spec.metadata.contains(&Metadata::new(
            "mqtt.topic_filter",
            "zigbee2mqtt/+/availability"
        )));
        assert!(spec.metadata.contains(&Metadata::new("mqtt.qos", "0")));
        assert!(spec
            .metadata
            .contains(&Metadata::new("mqtt.retain_policy", "ignore")));
        assert!(spec
            .metadata
            .contains(&Metadata::new("mqtt.shared_group", "chief-of-staff")));
    }

    #[test]
    fn mqtt_publication_refs_build_deterministic_native_cursors() {
        let publication = MqttPublicationRef::new("home/kitchen/temperature", MqttQos::AtLeastOnce)
            .unwrap()
            .with_packet_id(42)
            .retained(true)
            .duplicate(false);

        assert_eq!(
            publication.native_cursor(),
            "mqtt:home/kitchen/temperature:qos1:packet:42:retained:true:duplicate:false"
        );
    }
}
