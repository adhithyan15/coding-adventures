//! Deterministic smart-home event stream cursor and supervision primitives.
//!
//! This crate is intentionally transport-neutral. Hue SSE, ESPHome-style
//! WebSocket workers, MQTT subscriptions, cloud push callbacks, serial frames,
//! and radio report loops can all share the same cursor, heartbeat, and
//! reconnect rules while keeping protocol-specific I/O in adapter crates.

#![forbid(unsafe_code)]

use smart_home_core::{BridgeId, CommandId, CorrelationId, EventId, IntegrationId, Metadata};
use std::{cmp::Ordering, fmt};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStreamError {
    CheckpointStreamMismatch {
        expected: EventStreamId,
        actual: EventStreamId,
    },
}

impl fmt::Display for EventStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CheckpointStreamMismatch { expected, actual } => write!(
                f,
                "checkpoint stream {actual} does not match expected stream {expected}"
            ),
        }
    }
}

impl std::error::Error for EventStreamError {}

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
    EmptyDiscoveryPrefix,
    HashWildcardMustBeFinal,
    PlusWildcardMustOccupyLevel,
    TopicNameContainsWildcard,
    InvalidDiscoveryPathPart { field: &'static str, value: String },
}

impl fmt::Display for MqttTopicError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyFilter => write!(f, "MQTT topic filter must not be empty"),
            Self::EmptyDiscoveryPrefix => {
                write!(f, "Home Assistant MQTT discovery prefix must not be empty")
            }
            Self::HashWildcardMustBeFinal => {
                write!(f, "MQTT # wildcard must occupy the final topic level")
            }
            Self::PlusWildcardMustOccupyLevel => {
                write!(f, "MQTT + wildcard must occupy an entire topic level")
            }
            Self::TopicNameContainsWildcard => write!(f, "MQTT topic names must not use wildcards"),
            Self::InvalidDiscoveryPathPart { field, value } => write!(
                f,
                "Home Assistant MQTT discovery {field} must be a non-empty topic path segment without wildcards or slashes: {value}"
            ),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MqttPayloadFormat {
    RawBytes,
    Utf8Text,
    Json,
    HomeAssistantDiscoveryJson,
}

impl MqttPayloadFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RawBytes => "raw_bytes",
            Self::Utf8Text => "utf8_text",
            Self::Json => "json",
            Self::HomeAssistantDiscoveryJson => "home_assistant_discovery_json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HomeAssistantMqttDiscoveryComponent {
    AlarmControlPanel,
    BinarySensor,
    Button,
    Climate,
    Cover,
    Fan,
    Light,
    Lock,
    Number,
    Scene,
    Select,
    Sensor,
    Switch,
}

impl HomeAssistantMqttDiscoveryComponent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AlarmControlPanel => "alarm_control_panel",
            Self::BinarySensor => "binary_sensor",
            Self::Button => "button",
            Self::Climate => "climate",
            Self::Cover => "cover",
            Self::Fan => "fan",
            Self::Light => "light",
            Self::Lock => "lock",
            Self::Number => "number",
            Self::Scene => "scene",
            Self::Select => "select",
            Self::Sensor => "sensor",
            Self::Switch => "switch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeAssistantMqttDiscoverySpec {
    pub integration_id: IntegrationId,
    pub broker_id: BridgeId,
    pub discovery_prefix: String,
    pub component: HomeAssistantMqttDiscoveryComponent,
    pub object_id: String,
    pub node_id: Option<String>,
    pub state_topic_filter: MqttTopicFilter,
    pub availability_topic_filter: Option<MqttTopicFilter>,
    pub command_topic_name: Option<String>,
    pub qos: MqttQos,
    pub retain_policy: MqttRetainPolicy,
    pub metadata: Vec<Metadata>,
}

impl HomeAssistantMqttDiscoverySpec {
    pub fn new(
        broker_id: BridgeId,
        component: HomeAssistantMqttDiscoveryComponent,
        object_id: impl Into<String>,
        state_topic_filter: MqttTopicFilter,
    ) -> Result<Self, MqttTopicError> {
        let object_id = object_id.into();
        validate_mqtt_discovery_part("object_id", &object_id)?;
        Ok(Self {
            integration_id: IntegrationId::trusted("home_assistant_mqtt"),
            broker_id,
            discovery_prefix: "homeassistant".to_string(),
            component,
            object_id,
            node_id: None,
            state_topic_filter,
            availability_topic_filter: None,
            command_topic_name: None,
            qos: MqttQos::AtLeastOnce,
            retain_policy: MqttRetainPolicy::DeliverRetained,
            metadata: Vec::new(),
        })
    }

    pub fn with_integration(mut self, integration_id: IntegrationId) -> Self {
        self.integration_id = integration_id;
        self
    }

    pub fn with_discovery_prefix(
        mut self,
        discovery_prefix: impl Into<String>,
    ) -> Result<Self, MqttTopicError> {
        let discovery_prefix = discovery_prefix.into();
        validate_mqtt_discovery_prefix(&discovery_prefix)?;
        self.discovery_prefix = discovery_prefix;
        Ok(self)
    }

    pub fn with_node_id(mut self, node_id: impl Into<String>) -> Result<Self, MqttTopicError> {
        let node_id = node_id.into();
        validate_mqtt_discovery_part("node_id", &node_id)?;
        self.node_id = Some(node_id);
        Ok(self)
    }

    pub fn with_availability_topic(mut self, topic_filter: MqttTopicFilter) -> Self {
        self.availability_topic_filter = Some(topic_filter);
        self
    }

    pub fn with_command_topic(
        mut self,
        topic_name: impl Into<String>,
    ) -> Result<Self, MqttTopicError> {
        let topic_name = topic_name.into();
        validate_mqtt_topic_name(&topic_name)?;
        self.command_topic_name = Some(topic_name);
        Ok(self)
    }

    pub fn with_qos(mut self, qos: MqttQos) -> Self {
        self.qos = qos;
        self
    }

    pub fn with_retain_policy(mut self, retain_policy: MqttRetainPolicy) -> Self {
        self.retain_policy = retain_policy;
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.push(metadata);
        self
    }

    pub fn config_topic_name(&self) -> String {
        match &self.node_id {
            Some(node_id) => format!(
                "{}/{}/{}/{}/config",
                self.discovery_prefix,
                self.component.as_str(),
                node_id,
                self.object_id
            ),
            None => format!(
                "{}/{}/{}/config",
                self.discovery_prefix,
                self.component.as_str(),
                self.object_id
            ),
        }
    }

    pub fn discovery_key(&self) -> String {
        format!(
            "home_assistant_mqtt:{}:{}:{}:{}",
            self.broker_id.as_str(),
            self.discovery_prefix,
            self.component.as_str(),
            self.node_id
                .as_ref()
                .map(|node_id| format!("{node_id}:{}", self.object_id))
                .unwrap_or_else(|| self.object_id.clone())
        )
    }

    pub fn to_subscription_specs(&self) -> Vec<MqttSubscriptionSpec> {
        let mut specs = vec![self.subscription_for(self.state_topic_filter.clone(), "state")];
        if let Some(topic_filter) = &self.availability_topic_filter {
            specs.push(self.subscription_for(topic_filter.clone(), "availability"));
        }
        specs
    }

    pub fn to_config_publication_spec(&self) -> Result<MqttPublicationSpec, MqttTopicError> {
        let mut spec = MqttPublicationSpec::new(
            self.integration_id.clone(),
            self.broker_id.clone(),
            self.config_topic_name(),
        )?
        .with_qos(self.qos)
        .with_retain(true)
        .with_payload_format(MqttPayloadFormat::HomeAssistantDiscoveryJson);
        for metadata in self.discovery_metadata("config") {
            spec = spec.with_metadata(metadata);
        }
        Ok(spec)
    }

    pub fn to_command_publication_spec(
        &self,
        command_id: CommandId,
        correlation_id: CorrelationId,
    ) -> Result<Option<MqttPublicationSpec>, MqttTopicError> {
        let Some(topic_name) = &self.command_topic_name else {
            return Ok(None);
        };
        let mut spec = MqttPublicationSpec::for_command(
            self.integration_id.clone(),
            self.broker_id.clone(),
            topic_name.clone(),
            command_id,
            correlation_id,
        )?
        .with_qos(self.qos)
        .with_payload_format(MqttPayloadFormat::Json);
        for metadata in self.discovery_metadata("command") {
            spec = spec.with_metadata(metadata);
        }
        Ok(Some(spec))
    }

    fn subscription_for(
        &self,
        topic_filter: MqttTopicFilter,
        discovery_role: &'static str,
    ) -> MqttSubscriptionSpec {
        let mut spec = MqttSubscriptionSpec::new(
            self.integration_id.clone(),
            self.broker_id.clone(),
            topic_filter,
        )
        .with_qos(self.qos)
        .with_retain_policy(self.retain_policy);
        for metadata in self.discovery_metadata(discovery_role) {
            spec = spec.with_metadata(metadata);
        }
        spec
    }

    fn discovery_metadata(&self, discovery_role: &'static str) -> Vec<Metadata> {
        let mut metadata = vec![
            Metadata::new("home_assistant.discovery_role", discovery_role),
            Metadata::new("home_assistant.discovery_prefix", &self.discovery_prefix),
            Metadata::new("home_assistant.component", self.component.as_str()),
            Metadata::new("home_assistant.object_id", &self.object_id),
            Metadata::new("home_assistant.config_topic", self.config_topic_name()),
        ];
        if let Some(node_id) = &self.node_id {
            metadata.push(Metadata::new("home_assistant.node_id", node_id));
        }
        metadata.extend(self.metadata.iter().cloned());
        metadata
    }
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
pub struct MqttPublicationSpec {
    pub integration_id: IntegrationId,
    pub broker_id: BridgeId,
    pub topic_name: String,
    pub qos: MqttQos,
    pub retain: bool,
    pub payload_format: MqttPayloadFormat,
    pub command_id: Option<CommandId>,
    pub correlation_id: Option<CorrelationId>,
    pub metadata: Vec<Metadata>,
}

impl MqttPublicationSpec {
    pub fn new(
        integration_id: IntegrationId,
        broker_id: BridgeId,
        topic_name: impl Into<String>,
    ) -> Result<Self, MqttTopicError> {
        let topic_name = topic_name.into();
        validate_mqtt_topic_name(&topic_name)?;
        Ok(Self {
            integration_id,
            broker_id,
            topic_name,
            qos: MqttQos::AtLeastOnce,
            retain: false,
            payload_format: MqttPayloadFormat::Json,
            command_id: None,
            correlation_id: None,
            metadata: Vec::new(),
        })
    }

    pub fn for_command(
        integration_id: IntegrationId,
        broker_id: BridgeId,
        topic_name: impl Into<String>,
        command_id: CommandId,
        correlation_id: CorrelationId,
    ) -> Result<Self, MqttTopicError> {
        Self::new(integration_id, broker_id, topic_name)
            .map(|spec| spec.with_command_context(command_id, correlation_id))
    }

    pub fn with_qos(mut self, qos: MqttQos) -> Self {
        self.qos = qos;
        self
    }

    pub fn with_retain(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }

    pub fn with_payload_format(mut self, payload_format: MqttPayloadFormat) -> Self {
        self.payload_format = payload_format;
        self
    }

    pub fn with_command_context(
        mut self,
        command_id: CommandId,
        correlation_id: CorrelationId,
    ) -> Self {
        self.command_id = Some(command_id);
        self.correlation_id = Some(correlation_id);
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata.push(metadata);
        self
    }

    pub fn to_publication_ref(&self) -> MqttPublicationRef {
        MqttPublicationRef {
            topic_name: self.topic_name.clone(),
            qos: self.qos,
            packet_id: None,
            retained: self.retain,
            duplicate: false,
        }
    }

    pub fn audit_metadata(&self) -> Vec<Metadata> {
        let mut metadata = vec![
            Metadata::new("mqtt.integration_id", self.integration_id.as_str()),
            Metadata::new("mqtt.broker_id", self.broker_id.as_str()),
            Metadata::new("mqtt.topic_name", &self.topic_name),
            Metadata::new("mqtt.qos", self.qos.level().to_string()),
            Metadata::new("mqtt.retain", self.retain.to_string()),
            Metadata::new("mqtt.payload_format", self.payload_format.as_str()),
        ];

        if let Some(command_id) = &self.command_id {
            metadata.push(Metadata::new("smart_home.command_id", command_id.as_str()));
        }
        if let Some(correlation_id) = &self.correlation_id {
            metadata.push(Metadata::new(
                "smart_home.correlation_id",
                correlation_id.as_str(),
            ));
        }

        metadata.extend(self.metadata.iter().cloned());
        metadata
    }

    pub fn publication_key(&self) -> String {
        format!(
            "mqtt:{}:{}:{}:qos{}:retain:{}:payload:{}:command:{}:correlation:{}",
            self.integration_id.as_str(),
            self.broker_id.as_str(),
            self.topic_name,
            self.qos.level(),
            self.retain,
            self.payload_format.as_str(),
            self.command_id
                .as_ref()
                .map(|command_id| command_id.as_str())
                .unwrap_or("-"),
            self.correlation_id
                .as_ref()
                .map(|correlation_id| correlation_id.as_str())
                .unwrap_or("-")
        )
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventStreamStatus {
    Idle,
    Connecting,
    Healthy,
    Degraded,
    Disconnected,
    BackingOff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventStreamStateSort {
    StreamId,
    IntegrationThenBridge,
    StatusThenStreamId,
    LastObservedDesc,
    HeartbeatDueThenStreamId,
    RetryDueThenStreamId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamStateQuery {
    pub integration_ids: Vec<IntegrationId>,
    pub bridge_ids: Vec<BridgeId>,
    pub statuses: Vec<EventStreamStatus>,
    pub transports: Vec<EventStreamTransport>,
    pub local_only: Option<bool>,
    pub needs_cursor: Option<bool>,
    pub heartbeat_due_at_ms: Option<u64>,
    pub stale_at_ms: Option<u64>,
    pub ready_to_reconnect_at_ms: Option<u64>,
    pub with_pending_gaps: Option<bool>,
    pub with_restart_plan_at_ms: Option<u64>,
    pub sort: EventStreamStateSort,
    pub limit: Option<usize>,
}

impl Default for EventStreamStateQuery {
    fn default() -> Self {
        Self {
            integration_ids: Vec::new(),
            bridge_ids: Vec::new(),
            statuses: Vec::new(),
            transports: Vec::new(),
            local_only: None,
            needs_cursor: None,
            heartbeat_due_at_ms: None,
            stale_at_ms: None,
            ready_to_reconnect_at_ms: None,
            with_pending_gaps: None,
            with_restart_plan_at_ms: None,
            sort: EventStreamStateSort::StreamId,
            limit: None,
        }
    }
}

impl EventStreamStateQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_integration(mut self, integration_id: IntegrationId) -> Self {
        self.integration_ids.push(integration_id);
        self
    }

    pub fn with_bridge(mut self, bridge_id: BridgeId) -> Self {
        self.bridge_ids.push(bridge_id);
        self
    }

    pub fn with_status(mut self, status: EventStreamStatus) -> Self {
        self.statuses.push(status);
        self
    }

    pub fn with_transport(mut self, transport: EventStreamTransport) -> Self {
        self.transports.push(transport);
        self
    }

    pub fn local_only(mut self, local_only: bool) -> Self {
        self.local_only = Some(local_only);
        self
    }

    pub fn needs_cursor(mut self, needs_cursor: bool) -> Self {
        self.needs_cursor = Some(needs_cursor);
        self
    }

    pub fn heartbeat_due_at(mut self, now_ms: u64) -> Self {
        self.heartbeat_due_at_ms = Some(now_ms);
        self
    }

    pub fn stale_at(mut self, now_ms: u64) -> Self {
        self.stale_at_ms = Some(now_ms);
        self
    }

    pub fn ready_to_reconnect_at(mut self, now_ms: u64) -> Self {
        self.ready_to_reconnect_at_ms = Some(now_ms);
        self
    }

    pub fn with_pending_gaps(mut self, has_pending_gaps: bool) -> Self {
        self.with_pending_gaps = Some(has_pending_gaps);
        self
    }

    pub fn with_restart_plan_at(mut self, now_ms: u64) -> Self {
        self.with_restart_plan_at_ms = Some(now_ms);
        self
    }

    pub fn sorted_by(mut self, sort: EventStreamStateSort) -> Self {
        self.sort = sort;
        self
    }

    pub fn limited_to(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn matches_state(&self, state: &EventStreamState) -> bool {
        if !matches_any(&self.integration_ids, &state.spec.integration_id) {
            return false;
        }
        if !matches_any(&self.bridge_ids, &state.spec.bridge_id) {
            return false;
        }
        if !matches_any(&self.statuses, &state.status) {
            return false;
        }
        if !matches_any(&self.transports, &state.spec.transport) {
            return false;
        }
        if let Some(local_only) = self.local_only {
            if state.spec.transport.is_local() != local_only {
                return false;
            }
        }
        if let Some(needs_cursor) = self.needs_cursor {
            if state.spec.transport.needs_cursor() != needs_cursor {
                return false;
            }
        }
        if let Some(now_ms) = self.heartbeat_due_at_ms {
            if !EventStreamHeartbeatDeadline::from_state(state)
                .is_some_and(|deadline| deadline.is_due_at(now_ms))
            {
                return false;
            }
        }
        if let Some(now_ms) = self.stale_at_ms {
            if !state.stale_at(now_ms) {
                return false;
            }
        }
        if let Some(now_ms) = self.ready_to_reconnect_at_ms {
            if !state.ready_to_reconnect_at(now_ms) {
                return false;
            }
        }
        if let Some(has_pending_gaps) = self.with_pending_gaps {
            if (state.pending_gap_count > 0) != has_pending_gaps {
                return false;
            }
        }
        if let Some(now_ms) = self.with_restart_plan_at_ms {
            if state.restart_plan_at(now_ms).is_none() {
                return false;
            }
        }
        true
    }
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

    pub fn resume_from_checkpoint(
        spec: EventStreamSpec,
        checkpoint: EventStreamCheckpoint,
    ) -> Result<Self, EventStreamError> {
        if checkpoint.stream_id != spec.stream_id {
            return Err(EventStreamError::CheckpointStreamMismatch {
                expected: spec.stream_id,
                actual: checkpoint.stream_id,
            });
        }

        Ok(Self {
            spec,
            status: EventStreamStatus::Idle,
            cursor: checkpoint.cursor,
            connected_at_ms: None,
            last_heartbeat_at_ms: None,
            last_disconnect_at_ms: None,
            reconnect_attempt: 0,
            pending_gap_count: 0,
        })
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
pub struct EventStreamHeartbeatDeadline {
    pub stream_id: EventStreamId,
    pub integration_id: IntegrationId,
    pub bridge_id: BridgeId,
    pub status: EventStreamStatus,
    pub last_heartbeat_at_ms: Option<u64>,
    pub heartbeat_timeout_ms: u64,
    pub due_at_ms: u64,
}

impl EventStreamHeartbeatDeadline {
    pub fn from_state(state: &EventStreamState) -> Option<Self> {
        if !matches!(
            state.status,
            EventStreamStatus::Connecting
                | EventStreamStatus::Healthy
                | EventStreamStatus::Degraded
        ) {
            return None;
        }
        let baseline_ms = state
            .last_heartbeat_at_ms
            .or(state.connected_at_ms)
            .unwrap_or(state.cursor.observed_at_ms);
        Some(Self {
            stream_id: state.spec.stream_id.clone(),
            integration_id: state.spec.integration_id.clone(),
            bridge_id: state.spec.bridge_id.clone(),
            status: state.status,
            last_heartbeat_at_ms: state.last_heartbeat_at_ms,
            heartbeat_timeout_ms: state.spec.heartbeat_timeout_ms,
            due_at_ms: baseline_ms.saturating_add(state.spec.heartbeat_timeout_ms),
        })
    }

    pub fn is_due_at(&self, now_ms: u64) -> bool {
        now_ms >= self.due_at_ms
    }

    pub fn overdue_by_ms_at(&self, now_ms: u64) -> u64 {
        now_ms.saturating_sub(self.due_at_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamHeartbeatSchedule {
    pub generated_at_ms: u64,
    pub deadlines: Vec<EventStreamHeartbeatDeadline>,
}

impl EventStreamHeartbeatSchedule {
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

    pub fn due_at(&self, now_ms: u64) -> Vec<&EventStreamHeartbeatDeadline> {
        self.deadlines
            .iter()
            .filter(|deadline| deadline.is_due_at(now_ms))
            .collect()
    }

    pub fn deadlines_for_bridge(&self, bridge_id: &BridgeId) -> Vec<&EventStreamHeartbeatDeadline> {
        self.deadlines
            .iter()
            .filter(|deadline| &deadline.bridge_id == bridge_id)
            .collect()
    }
}

pub fn event_stream_heartbeat_schedule_at<'a, I>(
    states: I,
    now_ms: u64,
) -> EventStreamHeartbeatSchedule
where
    I: IntoIterator<Item = &'a EventStreamState>,
{
    let mut deadlines = states
        .into_iter()
        .filter_map(EventStreamHeartbeatDeadline::from_state)
        .collect::<Vec<_>>();
    deadlines.sort_by(|left, right| {
        left.due_at_ms
            .cmp(&right.due_at_ms)
            .then_with(|| left.stream_id.cmp(&right.stream_id))
    });
    EventStreamHeartbeatSchedule {
        generated_at_ms: now_ms,
        deadlines,
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

impl EventStreamRestartPlan {
    pub fn retry_due_at(&self, now_ms: u64) -> bool {
        now_ms >= self.retry_at_ms
    }

    pub fn wait_ms_at(&self, now_ms: u64) -> u64 {
        self.retry_at_ms.saturating_sub(now_ms)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventStreamRestartSchedule {
    pub generated_at_ms: u64,
    pub plans: Vec<EventStreamRestartPlan>,
}

impl EventStreamRestartSchedule {
    pub fn is_empty(&self) -> bool {
        self.plans.is_empty()
    }

    pub fn len(&self) -> usize {
        self.plans.len()
    }

    pub fn next_retry_at_ms(&self) -> Option<u64> {
        self.plans.iter().map(|plan| plan.retry_at_ms).min()
    }

    pub fn plans_ready_at(&self, now_ms: u64) -> Vec<&EventStreamRestartPlan> {
        self.plans
            .iter()
            .filter(|plan| plan.retry_due_at(now_ms))
            .collect()
    }

    pub fn plans_for_bridge(&self, bridge_id: &BridgeId) -> Vec<&EventStreamRestartPlan> {
        self.plans
            .iter()
            .filter(|plan| &plan.bridge_id == bridge_id)
            .collect()
    }
}

pub fn event_stream_restart_schedule_at<'a, I>(states: I, now_ms: u64) -> EventStreamRestartSchedule
where
    I: IntoIterator<Item = &'a EventStreamState>,
{
    let mut plans = states
        .into_iter()
        .filter_map(|state| state.restart_plan_at(now_ms))
        .collect::<Vec<_>>();
    plans.sort_by(|left, right| {
        left.retry_at_ms
            .cmp(&right.retry_at_ms)
            .then_with(|| restart_reason_rank(left.reason).cmp(&restart_reason_rank(right.reason)))
            .then_with(|| left.stream_id.cmp(&right.stream_id))
    });
    EventStreamRestartSchedule {
        generated_at_ms: now_ms,
        plans,
    }
}

pub fn query_event_stream_states<'a, I>(
    states: I,
    query: &EventStreamStateQuery,
) -> Vec<&'a EventStreamState>
where
    I: IntoIterator<Item = &'a EventStreamState>,
{
    let mut results = states
        .into_iter()
        .filter(|state| query.matches_state(state))
        .collect::<Vec<_>>();

    sort_event_stream_state_results(&mut results, query.sort);
    if let Some(limit) = query.limit {
        results.truncate(limit);
    }

    results
}

fn restart_reason_rank(reason: EventStreamRestartReason) -> u8 {
    match reason {
        EventStreamRestartReason::EventGap => 0,
        EventStreamRestartReason::HeartbeatOverdue => 1,
        EventStreamRestartReason::ExplicitDisconnect => 2,
        EventStreamRestartReason::StaleEvents => 3,
    }
}

fn sort_event_stream_state_results(
    states: &mut Vec<&EventStreamState>,
    sort: EventStreamStateSort,
) {
    match sort {
        EventStreamStateSort::StreamId => {
            states.sort_by(|left, right| compare_by_stream_id(left, right))
        }
        EventStreamStateSort::IntegrationThenBridge => states.sort_by(|left, right| {
            left.spec
                .integration_id
                .cmp(&right.spec.integration_id)
                .then_with(|| left.spec.bridge_id.cmp(&right.spec.bridge_id))
                .then_with(|| compare_by_stream_id(left, right))
        }),
        EventStreamStateSort::StatusThenStreamId => states.sort_by(|left, right| {
            left.status
                .cmp(&right.status)
                .then_with(|| compare_by_stream_id(left, right))
        }),
        EventStreamStateSort::LastObservedDesc => states.sort_by(|left, right| {
            right
                .cursor
                .observed_at_ms
                .cmp(&left.cursor.observed_at_ms)
                .then_with(|| compare_by_stream_id(left, right))
        }),
        EventStreamStateSort::HeartbeatDueThenStreamId => states.sort_by(|left, right| {
            heartbeat_due_sort_key(left)
                .cmp(&heartbeat_due_sort_key(right))
                .then_with(|| compare_by_stream_id(left, right))
        }),
        EventStreamStateSort::RetryDueThenStreamId => states.sort_by(|left, right| {
            retry_due_sort_key(left)
                .cmp(&retry_due_sort_key(right))
                .then_with(|| compare_by_stream_id(left, right))
        }),
    }
}

fn compare_by_stream_id(left: &EventStreamState, right: &EventStreamState) -> Ordering {
    left.spec.stream_id.cmp(&right.spec.stream_id)
}

fn heartbeat_due_sort_key(state: &EventStreamState) -> u64 {
    EventStreamHeartbeatDeadline::from_state(state)
        .map(|deadline| deadline.due_at_ms)
        .unwrap_or(u64::MAX)
}

fn retry_due_sort_key(state: &EventStreamState) -> u64 {
    state.next_retry_at_ms().unwrap_or(u64::MAX)
}

fn matches_any<T: PartialEq>(needles: &[T], value: &T) -> bool {
    needles.is_empty() || needles.iter().any(|needle| needle == value)
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

fn validate_mqtt_discovery_prefix(value: &str) -> Result<(), MqttTopicError> {
    if value.is_empty() {
        return Err(MqttTopicError::EmptyDiscoveryPrefix);
    }
    validate_mqtt_topic_name(value)
}

fn validate_mqtt_discovery_part(field: &'static str, value: &str) -> Result<(), MqttTopicError> {
    if value.is_empty() || value.contains('/') || value.contains('#') || value.contains('+') {
        return Err(MqttTopicError::InvalidDiscoveryPathPart {
            field,
            value: value.to_string(),
        });
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
    fn state_resumes_from_matching_checkpoints_without_replaying_zero() {
        let spec = EventStreamSpec::hue_sse(bridge_id(), "https://bridge/eventstream");
        let checkpoint = EventStreamCheckpoint::new(
            spec.stream_id.clone(),
            EventStreamCursor::start(1_000).advance(
                EventId::trusted("event-42"),
                Some("sse:last-event-id:42".to_string()),
                1_500,
            ),
        );

        let resumed = EventStreamState::resume_from_checkpoint(spec, checkpoint).unwrap();

        assert_eq!(resumed.status, EventStreamStatus::Idle);
        assert_eq!(resumed.cursor.sequence, 1);
        assert_eq!(
            resumed.cursor.native_cursor,
            Some("sse:last-event-id:42".to_string())
        );
        assert_eq!(
            resumed.cursor.last_event_id,
            Some(EventId::trusted("event-42"))
        );
        assert_eq!(resumed.cursor.observed_at_ms, 1_500);
        assert_eq!(resumed.reconnect_attempt, 0);
    }

    #[test]
    fn state_rejects_mismatched_checkpoints() {
        let spec = EventStreamSpec::hue_sse(bridge_id(), "https://bridge/eventstream");
        let checkpoint = EventStreamCheckpoint::new(
            EventStreamId::trusted("mqtt:broker-1"),
            EventStreamCursor::start(1_000),
        );

        let error = EventStreamState::resume_from_checkpoint(spec.clone(), checkpoint).unwrap_err();

        assert_eq!(
            error,
            EventStreamError::CheckpointStreamMismatch {
                expected: spec.stream_id,
                actual: EventStreamId::trusted("mqtt:broker-1")
            }
        );
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
    fn heartbeat_schedule_orders_stream_deadlines_without_mutating() {
        let mut hue = EventStreamState::new(
            EventStreamSpec::hue_sse(bridge_id(), "https://bridge/eventstream")
                .with_heartbeat_timeout(500),
            1_000,
        );
        hue.mark_connected(1_000);
        hue.record_event(EventId::trusted("event-1"), None, 1_100);
        let mut mqtt = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("mqtt"),
                BridgeId::trusted("broker-1"),
                EventStreamTransport::MqttSubscription,
            )
            .with_heartbeat_timeout(100),
            900,
        );
        mqtt.mark_connecting();
        let mut disconnected = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("esphome"),
                BridgeId::trusted("esp-1"),
                EventStreamTransport::WebSocket,
            ),
            1_000,
        );
        disconnected.mark_disconnected(1_050);

        let schedule = event_stream_heartbeat_schedule_at([&hue, &mqtt, &disconnected], 1_200);

        assert_eq!(schedule.generated_at_ms, 1_200);
        assert_eq!(schedule.len(), 2);
        assert!(!schedule.is_empty());
        assert_eq!(schedule.next_due_at_ms(), Some(1_000));
        assert_eq!(schedule.due_at(1_200).len(), 1);
        assert_eq!(
            schedule.due_at(1_200)[0].stream_id,
            EventStreamId::for_bridge(
                &IntegrationId::trusted("mqtt"),
                &BridgeId::trusted("broker-1")
            )
        );
        assert_eq!(schedule.deadlines_for_bridge(&bridge_id()).len(), 1);
        assert_eq!(hue.status, EventStreamStatus::Healthy);
        assert_eq!(mqtt.status, EventStreamStatus::Connecting);
    }

    #[test]
    fn restart_schedule_groups_due_stream_plans() {
        let mut gap = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("mqtt"),
                BridgeId::trusted("broker-1"),
                EventStreamTransport::MqttSubscription,
            )
            .with_reconnect_policy(ReconnectPolicy::new(50, 500, 2)),
            1_000,
        );
        gap.mark_connected(1_000);
        gap.record_gap(2, 1_050);
        let mut overdue = EventStreamState::new(
            EventStreamSpec::hue_sse(bridge_id(), "https://bridge/eventstream")
                .with_heartbeat_timeout(100)
                .with_reconnect_policy(ReconnectPolicy::new(100, 1_000, 2)),
            1_000,
        );
        overdue.mark_connected(1_000);
        let healthy = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("thread"),
                BridgeId::trusted("border-router-1"),
                EventStreamTransport::RadioReports,
            ),
            1_000,
        );

        let schedule = event_stream_restart_schedule_at([&gap, &overdue, &healthy], 1_150);

        assert_eq!(schedule.generated_at_ms, 1_150);
        assert_eq!(schedule.len(), 2);
        assert_eq!(schedule.next_retry_at_ms(), Some(1_200));
        assert!(matches!(
            schedule.plans.as_slice(),
            [first, second]
                if first.reason == EventStreamRestartReason::EventGap
                    && first.bridge_id == BridgeId::trusted("broker-1")
                    && first.wait_ms_at(1_175) == 25
                    && second.reason == EventStreamRestartReason::HeartbeatOverdue
                    && second.bridge_id == bridge_id()
        ));
        assert_eq!(schedule.plans_ready_at(1_200).len(), 1);
        assert_eq!(
            schedule
                .plans_for_bridge(&BridgeId::trusted("broker-1"))
                .len(),
            1
        );
        assert!(!schedule.is_empty());
    }

    #[test]
    fn state_queries_compose_transport_status_deadline_and_limit_filters() {
        let mut hue = EventStreamState::new(
            EventStreamSpec::hue_sse(bridge_id(), "https://bridge/eventstream")
                .with_heartbeat_timeout(500),
            1_000,
        );
        hue.mark_connected(1_000);
        hue.record_event(EventId::trusted("event-1"), None, 1_100);
        let mut mqtt = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("mqtt"),
                BridgeId::trusted("broker-1"),
                EventStreamTransport::MqttSubscription,
            ),
            1_000,
        );
        mqtt.mark_connected(1_000);
        mqtt.record_gap(2, 1_050);
        let mut cloud = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("cloud-hub"),
                BridgeId::trusted("account-1"),
                EventStreamTransport::CloudWebhook,
            ),
            1_000,
        );
        cloud.mark_connected(1_000);

        let query = EventStreamStateQuery::new()
            .local_only(true)
            .needs_cursor(true)
            .with_status(EventStreamStatus::Healthy)
            .with_status(EventStreamStatus::Degraded)
            .heartbeat_due_at(1_600)
            .sorted_by(EventStreamStateSort::HeartbeatDueThenStreamId)
            .limited_to(1);
        let results = query_event_stream_states([&hue, &mqtt, &cloud], &query);

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].spec.integration_id,
            IntegrationId::trusted("hue")
        );
        assert!(query.matches_state(&hue));
        assert!(!query.matches_state(&mqtt));
        assert!(!query.matches_state(&cloud));
    }

    #[test]
    fn state_queries_find_supervision_work_and_reconnect_ready_streams() {
        let mut gap = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("mqtt"),
                BridgeId::trusted("broker-1"),
                EventStreamTransport::MqttSubscription,
            )
            .with_reconnect_policy(ReconnectPolicy::new(50, 500, 2)),
            1_000,
        );
        gap.mark_connected(1_000);
        gap.record_gap(2, 1_050);
        let mut disconnected = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("esphome"),
                BridgeId::trusted("esp-1"),
                EventStreamTransport::WebSocket,
            )
            .with_reconnect_policy(ReconnectPolicy::new(500, 5_000, 2)),
            1_000,
        );
        disconnected.mark_connected(1_000);
        disconnected.mark_disconnected(1_200);
        let idle = EventStreamState::new(
            EventStreamSpec::new(
                IntegrationId::trusted("thread"),
                BridgeId::trusted("border-router-1"),
                EventStreamTransport::RadioReports,
            ),
            1_000,
        );

        let supervision = EventStreamStateQuery::new()
            .with_pending_gaps(true)
            .with_restart_plan_at(1_150)
            .sorted_by(EventStreamStateSort::StatusThenStreamId);
        let supervision_results =
            query_event_stream_states([&gap, &disconnected, &idle], &supervision);

        assert_eq!(supervision_results.len(), 1);
        assert_eq!(
            supervision_results[0].spec.bridge_id,
            BridgeId::trusted("broker-1")
        );

        let reconnect = EventStreamStateQuery::new()
            .with_integration(IntegrationId::trusted("esphome"))
            .ready_to_reconnect_at(1_700)
            .sorted_by(EventStreamStateSort::RetryDueThenStreamId);
        let reconnect_results = query_event_stream_states([&gap, &disconnected, &idle], &reconnect);

        assert_eq!(reconnect_results.len(), 1);
        assert_eq!(
            reconnect_results[0].spec.bridge_id,
            BridgeId::trusted("esp-1")
        );
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
    fn home_assistant_mqtt_discovery_builds_config_and_subscription_specs() {
        let discovery = HomeAssistantMqttDiscoverySpec::new(
            BridgeId::trusted("broker-1"),
            HomeAssistantMqttDiscoveryComponent::Light,
            "kitchen_light",
            MqttTopicFilter::new("zigbee2mqtt/kitchen_light/state").unwrap(),
        )
        .unwrap()
        .with_node_id("zigbee2mqtt")
        .unwrap()
        .with_availability_topic(
            MqttTopicFilter::new("zigbee2mqtt/kitchen_light/availability").unwrap(),
        )
        .with_command_topic("zigbee2mqtt/kitchen_light/set")
        .unwrap()
        .with_qos(MqttQos::AtMostOnce)
        .with_retain_policy(MqttRetainPolicy::IgnoreRetained)
        .with_metadata(Metadata::new("ha.unique_id", "kitchen-light-1"));

        assert_eq!(
            discovery.config_topic_name(),
            "homeassistant/light/zigbee2mqtt/kitchen_light/config"
        );
        assert_eq!(
            discovery.discovery_key(),
            "home_assistant_mqtt:broker-1:homeassistant:light:zigbee2mqtt:kitchen_light"
        );

        let subscriptions = discovery.to_subscription_specs();

        assert_eq!(subscriptions.len(), 2);
        assert_eq!(
            subscriptions[0].topic_filter.as_str(),
            "zigbee2mqtt/kitchen_light/state"
        );
        assert_eq!(subscriptions[0].qos, MqttQos::AtMostOnce);
        assert_eq!(
            subscriptions[0].retain_policy,
            MqttRetainPolicy::IgnoreRetained
        );
        assert!(subscriptions[0]
            .metadata
            .contains(&Metadata::new("home_assistant.discovery_role", "state")));
        assert!(subscriptions[1].metadata.contains(&Metadata::new(
            "home_assistant.discovery_role",
            "availability"
        )));
        assert!(subscriptions[1]
            .metadata
            .contains(&Metadata::new("ha.unique_id", "kitchen-light-1")));
    }

    #[test]
    fn home_assistant_mqtt_discovery_projects_publication_specs() {
        let discovery = HomeAssistantMqttDiscoverySpec::new(
            BridgeId::trusted("broker-1"),
            HomeAssistantMqttDiscoveryComponent::Switch,
            "coffee_switch",
            MqttTopicFilter::new("stat/coffee/POWER").unwrap(),
        )
        .unwrap()
        .with_discovery_prefix("ha")
        .unwrap()
        .with_command_topic("cmnd/coffee/POWER")
        .unwrap();

        let config = discovery.to_config_publication_spec().unwrap();

        assert_eq!(config.topic_name, "ha/switch/coffee_switch/config");
        assert_eq!(
            config.payload_format,
            MqttPayloadFormat::HomeAssistantDiscoveryJson
        );
        assert!(config.retain);
        assert!(config
            .audit_metadata()
            .contains(&Metadata::new("home_assistant.discovery_role", "config")));

        let command = discovery
            .to_command_publication_spec(
                CommandId::trusted("cmd-1"),
                CorrelationId::trusted("corr-1"),
            )
            .unwrap()
            .unwrap();

        assert_eq!(command.topic_name, "cmnd/coffee/POWER");
        assert_eq!(command.payload_format, MqttPayloadFormat::Json);
        assert!(command
            .audit_metadata()
            .contains(&Metadata::new("home_assistant.discovery_role", "command")));
        assert!(command
            .audit_metadata()
            .contains(&Metadata::new("smart_home.command_id", "cmd-1")));
    }

    #[test]
    fn home_assistant_mqtt_discovery_validates_path_parts_and_topics() {
        assert_eq!(
            HomeAssistantMqttDiscoverySpec::new(
                BridgeId::trusted("broker-1"),
                HomeAssistantMqttDiscoveryComponent::Sensor,
                "bad/object",
                MqttTopicFilter::new("sensors/temperature").unwrap(),
            )
            .unwrap_err(),
            MqttTopicError::InvalidDiscoveryPathPart {
                field: "object_id",
                value: "bad/object".to_string()
            }
        );

        assert_eq!(
            HomeAssistantMqttDiscoverySpec::new(
                BridgeId::trusted("broker-1"),
                HomeAssistantMqttDiscoveryComponent::Sensor,
                "temperature",
                MqttTopicFilter::new("sensors/temperature").unwrap(),
            )
            .unwrap()
            .with_discovery_prefix("")
            .unwrap_err(),
            MqttTopicError::EmptyDiscoveryPrefix
        );
        assert!(HomeAssistantMqttDiscoverySpec::new(
            BridgeId::trusted("broker-1"),
            HomeAssistantMqttDiscoveryComponent::Sensor,
            "temperature",
            MqttTopicFilter::new("sensors/temperature").unwrap(),
        )
        .unwrap()
        .with_command_topic("cmnd/+/POWER")
        .is_err());
    }

    #[test]
    fn mqtt_publication_specs_capture_command_context_and_audit_metadata() {
        let publication = MqttPublicationSpec::for_command(
            IntegrationId::trusted("zigbee2mqtt"),
            BridgeId::trusted("broker-1"),
            "zigbee2mqtt/kitchen_light/set",
            CommandId::trusted("cmd-1"),
            CorrelationId::trusted("corr-1"),
        )
        .unwrap()
        .with_qos(MqttQos::ExactlyOnce)
        .with_payload_format(MqttPayloadFormat::Json)
        .with_metadata(Metadata::new("mqtt.intent", "set_brightness"));

        let reference = publication.to_publication_ref();
        let metadata = publication.audit_metadata();

        assert_eq!(reference.topic_name, "zigbee2mqtt/kitchen_light/set");
        assert_eq!(reference.qos, MqttQos::ExactlyOnce);
        assert!(!reference.retained);
        assert!(metadata.contains(&Metadata::new(
            "mqtt.topic_name",
            "zigbee2mqtt/kitchen_light/set"
        )));
        assert!(metadata.contains(&Metadata::new("mqtt.payload_format", "json")));
        assert!(metadata.contains(&Metadata::new("smart_home.command_id", "cmd-1")));
        assert!(metadata.contains(&Metadata::new("smart_home.correlation_id", "corr-1")));
        assert!(metadata.contains(&Metadata::new("mqtt.intent", "set_brightness")));
    }

    #[test]
    fn mqtt_publication_specs_reject_wildcard_topics() {
        let err = MqttPublicationSpec::new(
            IntegrationId::trusted("tasmota"),
            BridgeId::trusted("broker-1"),
            "cmnd/+/POWER",
        )
        .unwrap_err();

        assert_eq!(err, MqttTopicError::TopicNameContainsWildcard);
    }

    #[test]
    fn mqtt_publication_keys_are_stable_for_command_deduplication() {
        let key = MqttPublicationSpec::new(
            IntegrationId::trusted("tasmota"),
            BridgeId::trusted("broker-1"),
            "cmnd/kitchen/POWER",
        )
        .unwrap()
        .with_qos(MqttQos::AtMostOnce)
        .with_retain(true)
        .with_command_context(
            CommandId::trusted("cmd-power"),
            CorrelationId::trusted("corr-1"),
        )
        .with_payload_format(MqttPayloadFormat::Utf8Text)
        .publication_key();

        assert_eq!(
            key,
            "mqtt:tasmota:broker-1:cmnd/kitchen/POWER:qos0:retain:true:payload:utf8_text:command:cmd-power:correlation:corr-1"
        );
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
