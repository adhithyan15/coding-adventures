//! MQTT broker host and Home Assistant discovery adapter for D23.

#![forbid(unsafe_code)]

use rumqttc::{
    Client, ClientError, ConnAck, ConnectReturnCode, Connection, Event, Incoming, MqttOptions,
    Publish, QoS, RecvTimeoutError,
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandStatus, CommandType, Device, DeviceEvent, DeviceEventType, DeviceId,
    Entity, EntityId, EntityKind, EventId, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, StateDelta, Value, ValueKind, VaultRef,
};
use smart_home_event_streams::{
    EventStreamSpec, EventStreamState, EventStreamTransport, HomeAssistantMqttDiscoveryComponent,
    MqttPayloadFormat, MqttPublicationRef, MqttPublicationSpec, MqttQos, MqttRetainPolicy,
    MqttSubscriptionSpec, MqttTopicError, MqttTopicFilter,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, RuntimeEvent, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::time::Duration;
use zeroize::{Zeroize, ZeroizeOnDrop};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "mqtt";
pub const DEFAULT_DISCOVERY_PREFIX: &str = "homeassistant";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttBrokerConfig {
    pub bridge_id: BridgeId,
    pub host: String,
    pub port: u16,
    pub client_id: String,
    pub discovery_prefix: String,
    pub keep_alive: Duration,
    pub request_capacity: usize,
    pub auth_ref: Option<VaultRef>,
}

impl MqttBrokerConfig {
    pub fn new(
        bridge_id: BridgeId,
        host: impl Into<String>,
        port: u16,
        client_id: impl Into<String>,
    ) -> Self {
        Self {
            bridge_id,
            host: host.into(),
            port,
            client_id: client_id.into(),
            discovery_prefix: DEFAULT_DISCOVERY_PREFIX.to_string(),
            keep_alive: Duration::from_secs(30),
            request_capacity: 32,
            auth_ref: None,
        }
    }

    pub fn with_discovery_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.discovery_prefix = prefix.into();
        self
    }

    pub fn with_auth_ref(mut self, auth_ref: VaultRef) -> Self {
        self.auth_ref = Some(auth_ref);
        self
    }

    pub fn validate(&self) -> Result<(), MqttIntegrationError> {
        if self.host.trim().is_empty() {
            return Err(MqttIntegrationError::Validation(
                "broker host must not be empty".to_string(),
            ));
        }
        if self.port == 0 {
            return Err(MqttIntegrationError::Validation(
                "broker port must be non-zero".to_string(),
            ));
        }
        if self.client_id.trim().is_empty() {
            return Err(MqttIntegrationError::Validation(
                "client id must not be empty".to_string(),
            ));
        }
        if self.discovery_prefix.trim().is_empty()
            || self.discovery_prefix.contains(['/', '+', '#'])
        {
            return Err(MqttIntegrationError::Validation(
                "discovery prefix must be one non-empty MQTT topic level".to_string(),
            ));
        }
        if self.keep_alive.is_zero() {
            return Err(MqttIntegrationError::Validation(
                "keep alive must be non-zero".to_string(),
            ));
        }
        if self.request_capacity < 2 {
            return Err(MqttIntegrationError::Validation(
                "request capacity must be at least two".to_string(),
            ));
        }
        Ok(())
    }

    pub fn address(&self) -> String {
        format!("mqtt://{}:{}", self.host, self.port)
    }
}

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct MqttCredentials {
    username: String,
    password: String,
}

impl MqttCredentials {
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MqttValueCodec {
    OnOff {
        payload_on: String,
        payload_off: String,
    },
    Boolean {
        payload_true: String,
        payload_false: String,
    },
    Number,
    Percentage,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttEntityBinding {
    pub config_topic: String,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
    pub entity_kind: EntityKind,
    pub capability_id: CapabilityId,
    pub state_topic: String,
    pub command_topic: Option<String>,
    pub availability_topic: Option<String>,
    pub payload_available: String,
    pub payload_not_available: String,
    pub codec: MqttValueCodec,
    pub value_json_key: Option<String>,
    pub qos: MqttQos,
}

impl MqttEntityBinding {
    pub fn subscription_specs(
        &self,
        bridge_id: &BridgeId,
    ) -> Result<Vec<MqttSubscriptionSpec>, MqttIntegrationError> {
        let mut topics = vec![self.state_topic.as_str()];
        if let Some(availability_topic) = &self.availability_topic {
            topics.push(availability_topic);
        }
        topics
            .into_iter()
            .map(|topic| {
                Ok(MqttSubscriptionSpec::new(
                    IntegrationId::trusted(INTEGRATION_ID),
                    bridge_id.clone(),
                    MqttTopicFilter::new(topic)?,
                )
                .with_qos(self.qos)
                .with_retain_policy(MqttRetainPolicy::DeliverRetained)
                .with_metadata(Metadata::new("mqtt.entity_id", self.entity_id.as_str())))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledMqttEntity {
    pub binding: MqttEntityBinding,
    pub subscriptions: Vec<MqttSubscriptionSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MqttCommandDispatch {
    pub command_result: CommandResult,
    pub publication: MqttPublicationSpec,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MqttIngestOutcome {
    Discovery(InstalledMqttEntity),
    State(DeviceEvent),
    Availability(DeviceEvent),
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttRuntimeIntegration {
    config: MqttBrokerConfig,
    bindings: BTreeMap<EntityId, MqttEntityBinding>,
    next_event_sequence: u64,
}

impl MqttRuntimeIntegration {
    pub fn new(config: MqttBrokerConfig) -> Result<Self, MqttIntegrationError> {
        config.validate()?;
        Ok(Self {
            config,
            bindings: BTreeMap::new(),
            next_event_sequence: 1,
        })
    }

    pub fn config(&self) -> &MqttBrokerConfig {
        &self.config
    }

    pub fn bindings(&self) -> impl Iterator<Item = &MqttEntityBinding> {
        self.bindings.values()
    }

    pub fn install_broker(
        &self,
        runtime: &mut SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<Option<Bridge>, MqttIntegrationError> {
        let mut bridge = Bridge::new(
            self.config.bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LocalProcess,
        );
        bridge.address = Some(self.config.address());
        bridge.hardware_model = Some("MQTT 3.1.1 Broker".to_string());
        bridge.auth_ref = self.config.auth_ref.clone();
        bridge.health = Health::Unknown;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![ProtocolIdentifier::new(
            ProtocolFamily::Mqtt,
            "broker",
            format!("{}:{}", self.config.host, self.config.port),
        )
        .map_err(|error| MqttIntegrationError::Validation(error.to_string()))?];
        bridge.metadata = vec![
            Metadata::new("mqtt.client_id", &self.config.client_id),
            Metadata::new("mqtt.discovery_prefix", &self.config.discovery_prefix),
        ];
        runtime.upsert_bridge(bridge).map_err(Into::into)
    }

    pub fn discovery_subscriptions(
        &self,
    ) -> Result<Vec<MqttSubscriptionSpec>, MqttIntegrationError> {
        [
            format!("{}/+/+/config", self.config.discovery_prefix),
            format!("{}/+/+/+/config", self.config.discovery_prefix),
        ]
        .into_iter()
        .map(|filter| {
            Ok(MqttSubscriptionSpec::new(
                IntegrationId::trusted(INTEGRATION_ID),
                self.config.bridge_id.clone(),
                MqttTopicFilter::new(filter)?,
            )
            .with_qos(MqttQos::AtLeastOnce)
            .with_retain_policy(MqttRetainPolicy::DeliverRetained)
            .with_metadata(Metadata::new("home_assistant.discovery_role", "config")))
        })
        .collect()
    }

    pub fn ingest_publication(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        publication: &Publish,
        observed_at_ms: u64,
    ) -> Result<MqttIngestOutcome, MqttIntegrationError> {
        if discovery_topic_parts(&self.config.discovery_prefix, &publication.topic).is_some() {
            if publication.payload.is_empty() {
                return Ok(MqttIngestOutcome::Ignored);
            }
            return self
                .install_discovery(runtime, &publication.topic, &publication.payload)
                .map(MqttIngestOutcome::Discovery);
        }

        let binding = self
            .bindings
            .values()
            .find(|binding| {
                binding.state_topic == publication.topic
                    || binding.availability_topic.as_deref() == Some(&publication.topic)
            })
            .cloned();
        let Some(binding) = binding else {
            return Ok(MqttIngestOutcome::Ignored);
        };

        if binding.availability_topic.as_deref() == Some(&publication.topic) {
            return self
                .ingest_availability(runtime, &binding, publication, observed_at_ms)
                .map(MqttIngestOutcome::Availability);
        }

        self.ingest_state(runtime, &binding, publication, observed_at_ms)
            .map(MqttIngestOutcome::State)
    }

    pub fn dispatch_command(
        &self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<MqttCommandDispatch, MqttIntegrationError> {
        let binding = self
            .bindings
            .get(&request.entity_id)
            .ok_or_else(|| MqttIntegrationError::UnknownEntity(request.entity_id.clone()))?;
        let command_topic = binding
            .command_topic
            .as_ref()
            .ok_or_else(|| MqttIntegrationError::ReadOnlyEntity(request.entity_id.clone()))?;
        let payload = command_payload(binding, &request)?;
        let command_result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        let publication = MqttPublicationSpec::for_command(
            IntegrationId::trusted(INTEGRATION_ID),
            self.config.bridge_id.clone(),
            command_topic,
            command_result.command_id.clone(),
            command_result.correlation_id.clone(),
        )?
        .with_qos(binding.qos)
        .with_payload_format(MqttPayloadFormat::Utf8Text)
        .with_metadata(Metadata::new("mqtt.entity_id", binding.entity_id.as_str()));
        Ok(MqttCommandDispatch {
            command_result,
            publication,
            payload,
        })
    }

    pub fn record_transport_failure(
        &self,
        runtime: &mut SmartHomeRuntime,
        accepted: &CommandResult,
        message: impl Into<String>,
    ) -> CommandResult {
        let result = CommandResult {
            command_id: accepted.command_id.clone(),
            status: CommandStatus::Failed,
            bridge_id: accepted.bridge_id.clone(),
            correlation_id: accepted.correlation_id.clone(),
            message: Some(message.into()),
        };
        runtime
            .event_bus_mut()
            .publish(RuntimeEvent::CommandResult(result.clone()));
        result
    }

    fn install_discovery(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        config_topic: &str,
        payload: &[u8],
    ) -> Result<InstalledMqttEntity, MqttIntegrationError> {
        let (_, component, object_id) =
            discovery_topic_parts(&self.config.discovery_prefix, config_topic).ok_or_else(
                || {
                    MqttIntegrationError::Validation(format!(
                        "not a Home Assistant discovery topic: {config_topic}"
                    ))
                },
            )?;
        let discovery: HomeAssistantDiscoveryPayload = serde_json::from_slice(payload)?;
        let state_topic = discovery.state_topic.clone().ok_or_else(|| {
            MqttIntegrationError::Validation("discovery config is missing state_topic".to_string())
        })?;
        validate_topic_name(&state_topic)?;
        if let Some(topic) = &discovery.command_topic {
            validate_topic_name(topic)?;
        }
        if let Some(topic) = &discovery.availability_topic {
            validate_topic_name(topic)?;
        }

        let unique_id = discovery
            .unique_id
            .clone()
            .unwrap_or_else(|| format!("{component}:{object_id}"));
        let id = id_fragment(&unique_id);
        let device_id = DeviceId::trusted(format!("mqtt-device:{id}"));
        let entity_id = EntityId::trusted(format!("mqtt-entity:{id}"));
        let name = discovery
            .name
            .clone()
            .or_else(|| {
                discovery
                    .device
                    .as_ref()
                    .and_then(|device| device.name.clone())
            })
            .unwrap_or_else(|| object_id.replace('_', " "));
        let (entity_kind, capability, codec) = discovery_projection(component, &discovery)?;
        let qos = qos_from_level(discovery.qos.unwrap_or(1))?;
        let device_info = discovery.device.unwrap_or_default();
        let identifiers = if device_info.identifiers.is_empty() {
            vec![unique_id.clone()]
        } else {
            device_info.identifiers
        };
        let protocol_identifiers = identifiers
            .into_iter()
            .map(|identifier| {
                ProtocolIdentifier::new(ProtocolFamily::Mqtt, "unique_id", identifier)
                    .map_err(|error| MqttIntegrationError::Validation(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let previous_device = runtime.registry().device(&device_id).cloned();
        let previous_state = runtime
            .registry()
            .entity(&entity_id)
            .and_then(|entity| entity.state.clone());
        let device = Device {
            device_id: device_id.clone(),
            bridge_id: self.config.bridge_id.clone(),
            manufacturer: device_info
                .manufacturer
                .unwrap_or_else(|| "MQTT".to_string()),
            model: device_info.model.unwrap_or_else(|| component.to_string()),
            name: device_info.name.unwrap_or_else(|| name.clone()),
            serial: None,
            firmware_version: device_info.sw_version,
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: protocol_identifiers,
            health: previous_device.map_or_else(
                || {
                    if discovery.availability_topic.is_some() {
                        Health::Unknown
                    } else {
                        Health::Online
                    }
                },
                |device| device.health,
            ),
            metadata: vec![
                Metadata::new("mqtt.config_topic", config_topic),
                Metadata::new("home_assistant.component", component),
            ],
        };
        let entity = Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: entity_kind,
            name,
            capabilities: vec![capability.clone()],
            state: previous_state,
            metadata: vec![
                Metadata::new("mqtt.state_topic", &state_topic),
                Metadata::new("mqtt.config_topic", config_topic),
            ],
        };
        runtime.upsert_device(device)?;
        runtime.upsert_entity(entity)?;

        let binding = MqttEntityBinding {
            config_topic: config_topic.to_string(),
            device_id,
            entity_id: entity_id.clone(),
            entity_kind,
            capability_id: capability.capability_id,
            state_topic,
            command_topic: discovery.command_topic,
            availability_topic: discovery.availability_topic,
            payload_available: discovery
                .payload_available
                .unwrap_or_else(|| "online".to_string()),
            payload_not_available: discovery
                .payload_not_available
                .unwrap_or_else(|| "offline".to_string()),
            codec,
            value_json_key: discovery.value_template.as_deref().and_then(value_json_key),
            qos,
        };
        let subscriptions = binding.subscription_specs(&self.config.bridge_id)?;
        self.bindings.insert(entity_id, binding.clone());
        Ok(InstalledMqttEntity {
            binding,
            subscriptions,
        })
    }

    fn ingest_state(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        binding: &MqttEntityBinding,
        publication: &Publish,
        observed_at_ms: u64,
    ) -> Result<DeviceEvent, MqttIntegrationError> {
        let payload = normalized_payload(&publication.payload, binding.value_json_key.as_deref())?;
        let value = decode_value(&binding.codec, &payload)?;
        let event = self.device_event(
            binding,
            publication,
            observed_at_ms,
            DeviceEventType::Updated,
            Some(StateDelta {
                capability_id: binding.capability_id.clone(),
                value,
            }),
        );
        runtime.apply_device_event(event.clone())?;
        Ok(event)
    }

    fn ingest_availability(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        binding: &MqttEntityBinding,
        publication: &Publish,
        observed_at_ms: u64,
    ) -> Result<DeviceEvent, MqttIntegrationError> {
        let payload = std::str::from_utf8(&publication.payload)
            .map_err(|error| MqttIntegrationError::Payload(error.to_string()))?;
        let health = if payload == binding.payload_available {
            Health::Online
        } else if payload == binding.payload_not_available {
            Health::Offline
        } else {
            Health::Degraded
        };
        let mut device = runtime
            .registry()
            .device(&binding.device_id)
            .cloned()
            .ok_or_else(|| MqttIntegrationError::UnknownEntity(binding.entity_id.clone()))?;
        device.health = health;
        runtime.upsert_device(device)?;
        let event_type = if health == Health::Online {
            DeviceEventType::Health
        } else {
            DeviceEventType::Unavailable
        };
        let event = self.device_event(binding, publication, observed_at_ms, event_type, None);
        runtime.apply_device_event(event.clone())?;
        Ok(event)
    }

    fn device_event(
        &mut self,
        binding: &MqttEntityBinding,
        publication: &Publish,
        observed_at_ms: u64,
        event_type: DeviceEventType,
        state_delta: Option<StateDelta>,
    ) -> DeviceEvent {
        let event = DeviceEvent {
            event_id: EventId::trusted(format!(
                "mqtt-event:{}:{}",
                id_fragment(binding.entity_id.as_str()),
                self.next_event_sequence
            )),
            bridge_id: self.config.bridge_id.clone(),
            device_id: Some(binding.device_id.clone()),
            entity_id: Some(binding.entity_id.clone()),
            observed_at_ms,
            received_at_ms: observed_at_ms,
            event_type,
            state_delta,
            raw_ref: Some(format!(
                "{}/topic/{}",
                self.config.address(),
                publication.topic
            )),
            correlation_id: None,
            metadata: vec![
                Metadata::new("mqtt.topic", &publication.topic),
                Metadata::new("mqtt.qos", qos_to_level(publication.qos).to_string()),
                Metadata::new("mqtt.retain", publication.retain.to_string()),
                Metadata::new("mqtt.duplicate", publication.dup.to_string()),
            ],
        };
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        event
    }
}

pub struct MqttRuntimeHost {
    client: Client,
    connection: Connection,
    integration: MqttRuntimeIntegration,
    runtime: SmartHomeRuntime,
    stream_state: EventStreamState,
    subscribed_topics: BTreeSet<String>,
}

impl fmt::Debug for MqttRuntimeHost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MqttRuntimeHost")
            .field("config", self.integration.config())
            .field("stream_state", &self.stream_state)
            .field("subscribed_topics", &self.subscribed_topics)
            .finish_non_exhaustive()
    }
}

impl MqttRuntimeHost {
    pub fn open(
        config: MqttBrokerConfig,
        credentials: Option<MqttCredentials>,
        mut runtime: SmartHomeRuntime,
        observed_at_ms: u64,
    ) -> Result<Self, MqttIntegrationError> {
        let integration = MqttRuntimeIntegration::new(config)?;
        integration.install_broker(&mut runtime, observed_at_ms)?;
        let mut options = MqttOptions::new(
            &integration.config.client_id,
            &integration.config.host,
            integration.config.port,
        );
        options.set_keep_alive(integration.config.keep_alive);
        if let Some(credentials) = credentials {
            options.set_credentials(credentials.username.clone(), credentials.password.clone());
        }
        let (client, connection) = Client::new(options, integration.config.request_capacity);
        let stream_spec = EventStreamSpec::new(
            IntegrationId::trusted(INTEGRATION_ID),
            integration.config.bridge_id.clone(),
            EventStreamTransport::MqttSubscription,
        )
        .with_endpoint(integration.config.address());
        let mut stream_state = EventStreamState::new(stream_spec, observed_at_ms);
        stream_state.mark_connecting();
        let mut host = Self {
            client,
            connection,
            integration,
            runtime,
            stream_state,
            subscribed_topics: BTreeSet::new(),
        };
        for subscription in host.integration.discovery_subscriptions()? {
            host.subscribe(&subscription)?;
        }
        Ok(host)
    }

    pub fn runtime(&self) -> &SmartHomeRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut SmartHomeRuntime {
        &mut self.runtime
    }

    pub fn integration(&self) -> &MqttRuntimeIntegration {
        &self.integration
    }

    pub fn stream_state(&self) -> &EventStreamState {
        &self.stream_state
    }

    pub fn subscribed_topics(&self) -> &BTreeSet<String> {
        &self.subscribed_topics
    }

    pub fn poll_once(
        &mut self,
        timeout: Duration,
        observed_at_ms: u64,
    ) -> Result<MqttHostOutcome, MqttIntegrationError> {
        match self.connection.recv_timeout(timeout) {
            Err(RecvTimeoutError::Timeout) => Ok(MqttHostOutcome::Idle),
            Err(RecvTimeoutError::Disconnected) => {
                self.mark_disconnected(observed_at_ms)?;
                Err(MqttIntegrationError::Connection(
                    "MQTT request channel disconnected".to_string(),
                ))
            }
            Ok(Err(error)) => {
                self.mark_disconnected(observed_at_ms)?;
                Err(MqttIntegrationError::Connection(error.to_string()))
            }
            Ok(Ok(event)) => self.handle_event(event, observed_at_ms),
        }
    }

    pub fn handle_event(
        &mut self,
        event: Event,
        observed_at_ms: u64,
    ) -> Result<MqttHostOutcome, MqttIntegrationError> {
        match event {
            Event::Incoming(Incoming::ConnAck(ConnAck {
                code: ConnectReturnCode::Success,
                ..
            })) => {
                self.stream_state.mark_connected(observed_at_ms);
                self.set_bridge_health(Health::Online, observed_at_ms)?;
                Ok(MqttHostOutcome::Connected)
            }
            Event::Incoming(Incoming::ConnAck(ConnAck { code, .. })) => {
                self.stream_state.mark_disconnected(observed_at_ms);
                let health = if matches!(
                    code,
                    ConnectReturnCode::BadUserNamePassword | ConnectReturnCode::NotAuthorized
                ) {
                    Health::AuthFailed
                } else {
                    Health::Degraded
                };
                self.set_bridge_health(health, observed_at_ms)?;
                Err(MqttIntegrationError::Connection(format!(
                    "broker refused connection with {code:?}"
                )))
            }
            Event::Incoming(Incoming::Publish(publication)) => {
                self.set_bridge_health(Health::Online, observed_at_ms)?;
                let publication_ref = publication_ref(&publication)?;
                let outcome = self.integration.ingest_publication(
                    &mut self.runtime,
                    &publication,
                    observed_at_ms,
                )?;
                match &outcome {
                    MqttIngestOutcome::Discovery(installed) => {
                        for subscription in &installed.subscriptions {
                            self.subscribe(subscription)?;
                        }
                        self.stream_state.record_event(
                            EventId::trusted(format!(
                                "mqtt-discovery:{}",
                                id_fragment(installed.binding.entity_id.as_str())
                            )),
                            Some(publication_ref.native_cursor()),
                            observed_at_ms,
                        );
                    }
                    MqttIngestOutcome::State(event) | MqttIngestOutcome::Availability(event) => {
                        self.stream_state.record_event(
                            event.event_id.clone(),
                            Some(publication_ref.native_cursor()),
                            observed_at_ms,
                        );
                    }
                    MqttIngestOutcome::Ignored => {
                        self.stream_state.mark_heartbeat(observed_at_ms);
                    }
                }
                Ok(MqttHostOutcome::Publication(Box::new(outcome)))
            }
            Event::Incoming(Incoming::PingResp) => {
                self.stream_state.mark_heartbeat(observed_at_ms);
                self.set_bridge_health(Health::Online, observed_at_ms)?;
                Ok(MqttHostOutcome::Heartbeat)
            }
            _ => Ok(MqttHostOutcome::ProtocolProgress),
        }
    }

    pub fn dispatch_command(
        &mut self,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<MqttCommandDispatch, MqttIntegrationError> {
        let dispatch =
            self.integration
                .dispatch_command(&mut self.runtime, principal_id, request, now_ms)?;
        if let Err(error) = self.client.try_publish(
            dispatch.publication.topic_name.clone(),
            qos_to_rumqtt(dispatch.publication.qos),
            dispatch.publication.retain,
            dispatch.payload.clone(),
        ) {
            self.integration.record_transport_failure(
                &mut self.runtime,
                &dispatch.command_result,
                format!("MQTT publication failed: {error}"),
            );
            return Err(MqttIntegrationError::Client(error));
        }
        Ok(dispatch)
    }

    fn subscribe(
        &mut self,
        subscription: &MqttSubscriptionSpec,
    ) -> Result<(), MqttIntegrationError> {
        let topic = subscription.topic_filter.as_str().to_string();
        if self.subscribed_topics.insert(topic.clone()) {
            self.client
                .try_subscribe(topic, qos_to_rumqtt(subscription.qos))?;
        }
        Ok(())
    }

    fn mark_disconnected(&mut self, observed_at_ms: u64) -> Result<(), MqttIntegrationError> {
        self.stream_state.mark_disconnected(observed_at_ms);
        self.set_bridge_health(Health::Degraded, observed_at_ms)
    }

    fn set_bridge_health(
        &mut self,
        health: Health,
        observed_at_ms: u64,
    ) -> Result<(), MqttIntegrationError> {
        let mut bridge = self
            .runtime
            .registry()
            .bridge(&self.integration.config.bridge_id)
            .cloned()
            .ok_or_else(|| {
                MqttIntegrationError::Validation("MQTT broker is not installed".to_string())
            })?;
        bridge.health = health;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        self.runtime.upsert_bridge(bridge)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MqttHostOutcome {
    Connected,
    Publication(Box<MqttIngestOutcome>),
    Heartbeat,
    ProtocolProgress,
    Idle,
}

#[derive(Debug)]
pub enum MqttIntegrationError {
    Validation(String),
    Payload(String),
    UnknownEntity(EntityId),
    ReadOnlyEntity(EntityId),
    UnsupportedCommand {
        entity_id: EntityId,
        command_type: CommandType,
    },
    Json(serde_json::Error),
    Topic(MqttTopicError),
    Runtime(RuntimeError),
    Client(ClientError),
    Connection(String),
}

impl fmt::Display for MqttIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid MQTT integration: {message}"),
            Self::Payload(message) => write!(formatter, "invalid MQTT payload: {message}"),
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown MQTT entity {}", entity_id.as_str())
            }
            Self::ReadOnlyEntity(entity_id) => {
                write!(formatter, "MQTT entity {} is read-only", entity_id.as_str())
            }
            Self::UnsupportedCommand {
                entity_id,
                command_type,
            } => write!(
                formatter,
                "MQTT entity {} does not support {command_type:?}",
                entity_id.as_str()
            ),
            Self::Json(error) => write!(formatter, "invalid MQTT discovery JSON: {error}"),
            Self::Topic(error) => error.fmt(formatter),
            Self::Runtime(error) => error.fmt(formatter),
            Self::Client(error) => write!(formatter, "MQTT client request failed: {error}"),
            Self::Connection(message) => write!(formatter, "MQTT connection failed: {message}"),
        }
    }
}

impl std::error::Error for MqttIntegrationError {}

impl From<serde_json::Error> for MqttIntegrationError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<MqttTopicError> for MqttIntegrationError {
    fn from(error: MqttTopicError) -> Self {
        Self::Topic(error)
    }
}

impl From<RuntimeError> for MqttIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<ClientError> for MqttIntegrationError {
    fn from(error: ClientError) -> Self {
        Self::Client(error)
    }
}

#[derive(Debug, Deserialize)]
struct HomeAssistantDiscoveryPayload {
    #[serde(default, alias = "uniq_id")]
    unique_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(
        default,
        alias = "stat_t",
        alias = "temperature_state_topic",
        alias = "temp_stat_t"
    )]
    state_topic: Option<String>,
    #[serde(
        default,
        alias = "cmd_t",
        alias = "temperature_command_topic",
        alias = "temp_cmd_t"
    )]
    command_topic: Option<String>,
    #[serde(default, alias = "avty_t")]
    availability_topic: Option<String>,
    #[serde(default)]
    payload_on: Option<String>,
    #[serde(default)]
    payload_off: Option<String>,
    #[serde(default)]
    state_on: Option<String>,
    #[serde(default)]
    state_off: Option<String>,
    #[serde(default)]
    payload_available: Option<String>,
    #[serde(default)]
    payload_not_available: Option<String>,
    #[serde(default, alias = "val_tpl")]
    value_template: Option<String>,
    #[serde(default, alias = "dev_cla")]
    device_class: Option<String>,
    #[serde(default, alias = "unit_of_meas")]
    unit_of_measurement: Option<String>,
    #[serde(default)]
    qos: Option<u8>,
    #[serde(default, alias = "dev")]
    device: Option<HomeAssistantDevicePayload>,
}

#[derive(Debug, Default, Deserialize)]
struct HomeAssistantDevicePayload {
    #[serde(default)]
    identifiers: Vec<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default, alias = "mf")]
    manufacturer: Option<String>,
    #[serde(default, alias = "mdl")]
    model: Option<String>,
    #[serde(default, alias = "sw")]
    sw_version: Option<String>,
}

fn discovery_topic_parts<'a>(prefix: &str, topic: &'a str) -> Option<(&'a str, &'a str, &'a str)> {
    let parts = topic.split('/').collect::<Vec<_>>();
    if parts.first().copied() != Some(prefix) || parts.last().copied() != Some("config") {
        return None;
    }
    match parts.as_slice() {
        [_, component, object_id, _] => Some(("", component, object_id)),
        [_, component, node_id, object_id, _] => Some((node_id, component, object_id)),
        _ => None,
    }
}

fn discovery_projection(
    component: &str,
    discovery: &HomeAssistantDiscoveryPayload,
) -> Result<(EntityKind, Capability, MqttValueCodec), MqttIntegrationError> {
    let payload_on = discovery
        .payload_on
        .clone()
        .or_else(|| discovery.state_on.clone())
        .unwrap_or_else(|| "ON".to_string());
    let payload_off = discovery
        .payload_off
        .clone()
        .or_else(|| discovery.state_off.clone())
        .unwrap_or_else(|| "OFF".to_string());
    let projected = match component {
        "light" => (
            EntityKind::Light,
            Capability::light_on_off(),
            MqttValueCodec::OnOff {
                payload_on,
                payload_off,
            },
        ),
        "switch" => (
            EntityKind::Switch,
            Capability::light_on_off(),
            MqttValueCodec::OnOff {
                payload_on,
                payload_off,
            },
        ),
        "binary_sensor" => {
            let capability = match discovery.device_class.as_deref() {
                Some("motion" | "occupancy" | "presence") => Capability::sensor_occupancy(),
                Some("door" | "garage_door" | "opening" | "window") => Capability::sensor_contact(),
                _ => Capability::new(
                    CapabilityId::trusted("sensor.binary"),
                    CapabilityMode::Observe,
                    ValueKind::Boolean,
                ),
            };
            (
                EntityKind::Sensor,
                capability,
                MqttValueCodec::Boolean {
                    payload_true: payload_on,
                    payload_false: payload_off,
                },
            )
        }
        "sensor" => {
            let mut capability = match discovery.device_class.as_deref() {
                Some("temperature") => Capability::sensor_temperature(),
                Some("humidity") => Capability::sensor_humidity(),
                Some("illuminance") => Capability::sensor_illuminance(),
                _ => Capability::new(
                    CapabilityId::trusted("sensor.value"),
                    CapabilityMode::Observe,
                    ValueKind::Number,
                ),
            };
            if let Some(unit) = &discovery.unit_of_measurement {
                capability.unit = Some(unit.clone());
            }
            (
                EntityKind::Sensor,
                capability,
                if discovery.device_class.as_deref() == Some("humidity") {
                    MqttValueCodec::Percentage
                } else {
                    MqttValueCodec::Number
                },
            )
        }
        "climate" => (
            EntityKind::Thermostat,
            Capability::climate_setpoint(),
            MqttValueCodec::Number,
        ),
        _ => {
            let supported = [
                HomeAssistantMqttDiscoveryComponent::Light.as_str(),
                HomeAssistantMqttDiscoveryComponent::Switch.as_str(),
                HomeAssistantMqttDiscoveryComponent::BinarySensor.as_str(),
                HomeAssistantMqttDiscoveryComponent::Sensor.as_str(),
                HomeAssistantMqttDiscoveryComponent::Climate.as_str(),
            ]
            .join(", ");
            return Err(MqttIntegrationError::Validation(format!(
                "unsupported Home Assistant MQTT component {component}; supported: {supported}"
            )));
        }
    };
    Ok(projected)
}

fn normalized_payload(
    payload: &[u8],
    value_json_key: Option<&str>,
) -> Result<String, MqttIntegrationError> {
    let text = std::str::from_utf8(payload)
        .map_err(|error| MqttIntegrationError::Payload(error.to_string()))?;
    let Some(key) = value_json_key else {
        return Ok(text.trim().to_string());
    };
    let json: JsonValue = serde_json::from_str(text)?;
    let value = json.get(key).ok_or_else(|| {
        MqttIntegrationError::Payload(format!("JSON payload is missing key {key}"))
    })?;
    match value {
        JsonValue::String(value) => Ok(value.clone()),
        JsonValue::Number(value) => Ok(value.to_string()),
        JsonValue::Bool(value) => Ok(value.to_string()),
        _ => Err(MqttIntegrationError::Payload(format!(
            "JSON key {key} must contain a scalar value; got {value}"
        ))),
    }
}

fn decode_value(codec: &MqttValueCodec, payload: &str) -> Result<Value, MqttIntegrationError> {
    match codec {
        MqttValueCodec::OnOff {
            payload_on,
            payload_off,
        }
        | MqttValueCodec::Boolean {
            payload_true: payload_on,
            payload_false: payload_off,
        } => {
            if payload == payload_on {
                Ok(Value::Bool(true))
            } else if payload == payload_off {
                Ok(Value::Bool(false))
            } else {
                Err(MqttIntegrationError::Payload(format!(
                    "expected {payload_on:?} or {payload_off:?}, got {payload:?}"
                )))
            }
        }
        MqttValueCodec::Number => payload
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|error| MqttIntegrationError::Payload(error.to_string())),
        MqttValueCodec::Percentage => payload
            .parse::<u16>()
            .map_err(|error| MqttIntegrationError::Payload(error.to_string()))
            .and_then(|value| {
                Value::percentage(value)
                    .map_err(|error| MqttIntegrationError::Payload(error.to_string()))
            }),
        MqttValueCodec::Text => Ok(Value::Text(payload.to_string())),
    }
}

fn command_payload(
    binding: &MqttEntityBinding,
    request: &RuntimeCommandToolRequest,
) -> Result<Vec<u8>, MqttIntegrationError> {
    let payload = match (&binding.codec, request.command_type, &request.arguments) {
        (MqttValueCodec::OnOff { payload_on, .. }, CommandType::TurnOn, _) => payload_on.clone(),
        (MqttValueCodec::OnOff { payload_off, .. }, CommandType::TurnOff, _) => payload_off.clone(),
        (MqttValueCodec::Percentage, CommandType::SetBrightness, Value::Percentage(value)) => {
            value.to_string()
        }
        (MqttValueCodec::Number, CommandType::SetThermostatSetpoint, Value::Number(value)) => {
            value.to_string()
        }
        (MqttValueCodec::Number, CommandType::SetThermostatSetpoint, Value::Integer(value)) => {
            value.to_string()
        }
        _ => {
            return Err(MqttIntegrationError::UnsupportedCommand {
                entity_id: binding.entity_id.clone(),
                command_type: request.command_type,
            });
        }
    };
    Ok(payload.into_bytes())
}

fn value_json_key(template: &str) -> Option<String> {
    let compact = template
        .trim()
        .trim_start_matches("{{")
        .trim_end_matches("}}")
        .trim();
    compact
        .strip_prefix("value_json.")
        .filter(|key| {
            !key.is_empty()
                && key
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
        })
        .map(ToString::to_string)
}

fn id_fragment(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut previous_dash = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            previous_dash = false;
        } else if !previous_dash {
            output.push('-');
            previous_dash = true;
        }
    }
    output.trim_matches('-').to_string()
}

fn validate_topic_name(topic: &str) -> Result<(), MqttIntegrationError> {
    MqttPublicationRef::new(topic, MqttQos::AtMostOnce)?;
    Ok(())
}

fn qos_from_level(level: u8) -> Result<MqttQos, MqttIntegrationError> {
    match level {
        0 => Ok(MqttQos::AtMostOnce),
        1 => Ok(MqttQos::AtLeastOnce),
        2 => Ok(MqttQos::ExactlyOnce),
        _ => Err(MqttIntegrationError::Validation(format!(
            "MQTT QoS must be 0, 1, or 2; got {level}"
        ))),
    }
}

fn qos_to_rumqtt(qos: MqttQos) -> QoS {
    match qos {
        MqttQos::AtMostOnce => QoS::AtMostOnce,
        MqttQos::AtLeastOnce => QoS::AtLeastOnce,
        MqttQos::ExactlyOnce => QoS::ExactlyOnce,
    }
}

fn qos_to_level(qos: QoS) -> u8 {
    match qos {
        QoS::AtMostOnce => 0,
        QoS::AtLeastOnce => 1,
        QoS::ExactlyOnce => 2,
    }
}

fn publication_ref(publication: &Publish) -> Result<MqttPublicationRef, MqttIntegrationError> {
    let mut publication_ref = MqttPublicationRef::new(
        &publication.topic,
        qos_from_level(qos_to_level(publication.qos))?,
    )?
    .retained(publication.retain)
    .duplicate(publication.dup);
    if publication.pkid != 0 {
        publication_ref = publication_ref.with_packet_id(publication.pkid);
    }
    Ok(publication_ref)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use smart_home_event_streams::EventStreamStatus;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    fn integration() -> MqttRuntimeIntegration {
        MqttRuntimeIntegration::new(MqttBrokerConfig::new(
            BridgeId::trusted("mqtt-broker-1"),
            "mqtt.local",
            1883,
            "smart-home-test",
        ))
        .unwrap()
    }

    fn discovery_publish() -> Publish {
        Publish::new(
            "homeassistant/light/kitchen/config",
            QoS::AtLeastOnce,
            br#"{
                "unique_id":"kitchen-main",
                "name":"Kitchen Main",
                "state_topic":"house/kitchen/light/state",
                "command_topic":"house/kitchen/light/set",
                "availability_topic":"house/kitchen/status",
                "payload_on":"ON",
                "payload_off":"OFF",
                "device":{
                    "identifiers":["shelly-kitchen"],
                    "name":"Kitchen Relay",
                    "manufacturer":"Shelly",
                    "model":"Plus 1"
                }
            }"#,
        )
    }

    fn install_light(
        integration: &mut MqttRuntimeIntegration,
        runtime: &mut SmartHomeRuntime,
    ) -> InstalledMqttEntity {
        integration.install_broker(runtime, 1_000).unwrap();
        match integration
            .ingest_publication(runtime, &discovery_publish(), 1_010)
            .unwrap()
        {
            MqttIngestOutcome::Discovery(installed) => installed,
            outcome => panic!("unexpected outcome: {outcome:?}"),
        }
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-mqtt-test"),
                principal.clone(),
                PrivilegeTier::HighRisk,
                "test",
                0,
            ));
    }

    fn read_mqtt_packet(stream: &mut TcpStream) -> Vec<u8> {
        let mut packet = vec![0_u8; 1];
        stream.read_exact(&mut packet).unwrap();
        let mut multiplier = 1_usize;
        let mut remaining = 0_usize;
        loop {
            let mut encoded = [0_u8; 1];
            stream.read_exact(&mut encoded).unwrap();
            packet.push(encoded[0]);
            remaining += usize::from(encoded[0] & 0x7f) * multiplier;
            if encoded[0] & 0x80 == 0 {
                break;
            }
            multiplier *= 128;
        }
        let header_length = packet.len();
        packet.resize(header_length + remaining, 0);
        stream.read_exact(&mut packet[header_length..]).unwrap();
        packet
    }

    fn acknowledge_subscription(stream: &mut TcpStream, packet: &[u8]) {
        assert_eq!(packet[0], 0x82);
        let remaining_offset = packet
            .iter()
            .skip(1)
            .position(|byte| byte & 0x80 == 0)
            .unwrap()
            + 2;
        let packet_id = &packet[remaining_offset..remaining_offset + 2];
        stream
            .write_all(&[0x90, 0x03, packet_id[0], packet_id[1], 0x01])
            .unwrap();
    }

    fn mqtt_publish(topic: &str, payload: &[u8], retained: bool) -> Vec<u8> {
        let mut remaining = 2 + topic.len() + payload.len();
        let mut packet = vec![if retained { 0x31 } else { 0x30 }];
        loop {
            let mut encoded = (remaining % 128) as u8;
            remaining /= 128;
            if remaining > 0 {
                encoded |= 0x80;
            }
            packet.push(encoded);
            if remaining == 0 {
                break;
            }
        }
        packet.extend_from_slice(&(topic.len() as u16).to_be_bytes());
        packet.extend_from_slice(topic.as_bytes());
        packet.extend_from_slice(payload);
        packet
    }

    #[test]
    fn broker_install_keeps_only_vault_reference() {
        let config = MqttBrokerConfig::new(
            BridgeId::trusted("mqtt-broker-1"),
            "mqtt.local",
            1883,
            "smart-home-test",
        )
        .with_auth_ref(VaultRef::trusted("vault:mqtt/home"));
        let integration = MqttRuntimeIntegration::new(config).unwrap();
        let mut runtime = SmartHomeRuntime::new();
        integration.install_broker(&mut runtime, 1_000).unwrap();

        let bridge = runtime
            .registry()
            .bridge(&BridgeId::trusted("mqtt-broker-1"))
            .unwrap();
        assert_eq!(bridge.address.as_deref(), Some("mqtt://mqtt.local:1883"));
        assert_eq!(
            bridge.auth_ref.as_ref().map(VaultRef::as_str),
            Some("vault:mqtt/home")
        );
        assert_eq!(bridge.health, Health::Unknown);
    }

    #[test]
    fn discovery_installs_commandable_entity_and_subscription_plan() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);

        assert_eq!(installed.binding.entity_kind, EntityKind::Light);
        assert_eq!(
            installed.binding.capability_id,
            CapabilityId::trusted("light.on_off")
        );
        assert_eq!(installed.subscriptions.len(), 2);
        assert!(installed
            .subscriptions
            .iter()
            .any(|spec| spec.topic_filter.as_str() == "house/kitchen/light/state"));
        let device = runtime
            .registry()
            .device(&installed.binding.device_id)
            .unwrap();
        assert_eq!(device.manufacturer, "Shelly");
        assert_eq!(device.model, "Plus 1");
    }

    #[test]
    fn state_and_availability_publications_update_runtime() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);
        let mut state = Publish::new("house/kitchen/light/state", QoS::AtLeastOnce, "ON");
        state.retain = true;
        let event = match integration
            .ingest_publication(&mut runtime, &state, 1_100)
            .unwrap()
        {
            MqttIngestOutcome::State(event) => event,
            outcome => panic!("unexpected outcome: {outcome:?}"),
        };
        assert_eq!(
            event.state_delta.as_ref().map(|delta| &delta.value),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            runtime
                .registry()
                .entity(&installed.binding.entity_id)
                .and_then(|entity| entity.state.as_ref())
                .map(|state| &state.value),
            Some(&Value::Object(vec![(
                "light.on_off".to_string(),
                Value::Bool(true)
            )]))
        );

        let unavailable = Publish::new("house/kitchen/status", QoS::AtLeastOnce, "offline");
        integration
            .ingest_publication(&mut runtime, &unavailable, 1_200)
            .unwrap();
        assert_eq!(
            runtime
                .registry()
                .device(&installed.binding.device_id)
                .map(|device| device.health),
            Some(Health::Offline)
        );

        integration
            .ingest_publication(&mut runtime, &discovery_publish(), 1_300)
            .unwrap();
        assert_eq!(
            runtime
                .registry()
                .entity(&installed.binding.entity_id)
                .and_then(|entity| entity.state.as_ref())
                .map(|state| &state.value),
            Some(&Value::Object(vec![(
                "light.on_off".to_string(),
                Value::Bool(true)
            )]))
        );
        assert_eq!(
            runtime
                .registry()
                .device(&installed.binding.device_id)
                .map(|device| device.health),
            Some(Health::Offline)
        );
    }

    #[test]
    fn authorized_command_becomes_audited_publication() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);
        let principal = AgentId::trusted("agent:mqtt-test");
        grant(&mut runtime, &principal);

        let dispatch = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.binding.entity_id,
                    CommandType::TurnOn,
                    Value::Null,
                ),
                2_000,
            )
            .unwrap();

        assert_eq!(dispatch.payload, b"ON");
        assert_eq!(dispatch.publication.topic_name, "house/kitchen/light/set");
        assert_eq!(dispatch.command_result.status, CommandStatus::Accepted);
        assert_eq!(
            dispatch.publication.command_id,
            Some(dispatch.command_result.command_id)
        );
    }

    #[test]
    fn unauthorized_command_never_creates_a_publication() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);
        let error = integration
            .dispatch_command(
                &mut runtime,
                AgentId::trusted("agent:untrusted"),
                RuntimeCommandToolRequest::new(
                    installed.binding.entity_id,
                    CommandType::TurnOff,
                    Value::Null,
                ),
                2_000,
            )
            .unwrap_err();
        assert!(matches!(
            error,
            MqttIntegrationError::Runtime(RuntimeError::UnauthorizedTool { .. })
        ));
    }

    #[test]
    fn json_value_template_normalizes_sensor_payload() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        integration.install_broker(&mut runtime, 1_000).unwrap();
        let discovery = Publish::new(
            "homeassistant/sensor/office_temperature/config",
            QoS::AtLeastOnce,
            br#"{
                "unique_id":"office-temperature",
                "name":"Office Temperature",
                "state_topic":"house/office/temperature",
                "device_class":"temperature",
                "unit_of_measurement":"C",
                "value_template":"{{ value_json.temperature }}"
            }"#,
        );
        let installed = match integration
            .ingest_publication(&mut runtime, &discovery, 1_010)
            .unwrap()
        {
            MqttIngestOutcome::Discovery(installed) => installed,
            outcome => panic!("unexpected outcome: {outcome:?}"),
        };
        let state = Publish::new(
            "house/office/temperature",
            QoS::AtMostOnce,
            br#"{"temperature":"21.5"}"#,
        );
        integration
            .ingest_publication(&mut runtime, &state, 1_100)
            .unwrap();
        assert_eq!(
            runtime
                .registry()
                .entity(&installed.binding.entity_id)
                .and_then(|entity| entity.state.as_ref())
                .map(|state| &state.value),
            Some(&Value::Object(vec![(
                "sensor.temperature".to_string(),
                Value::Number(21.5)
            )]))
        );
    }

    #[test]
    fn climate_discovery_uses_temperature_topics_for_authorized_setpoints() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        integration.install_broker(&mut runtime, 1_000).unwrap();
        let discovery = Publish::new(
            "homeassistant/climate/hallway/config",
            QoS::AtLeastOnce,
            br#"{
                "unique_id":"hallway-thermostat",
                "name":"Hallway Thermostat",
                "temperature_state_topic":"house/hallway/temperature",
                "temperature_command_topic":"house/hallway/setpoint"
            }"#,
        );
        let installed = match integration
            .ingest_publication(&mut runtime, &discovery, 1_010)
            .unwrap()
        {
            MqttIngestOutcome::Discovery(installed) => installed,
            outcome => panic!("unexpected outcome: {outcome:?}"),
        };
        assert_eq!(installed.binding.entity_kind, EntityKind::Thermostat);
        let principal = AgentId::trusted("agent:climate-test");
        grant(&mut runtime, &principal);
        let dispatch = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.binding.entity_id,
                    CommandType::SetThermostatSetpoint,
                    Value::Number(19.5),
                ),
                2_000,
            )
            .unwrap();
        assert_eq!(dispatch.publication.topic_name, "house/hallway/setpoint");
        assert_eq!(dispatch.payload, b"19.5");
    }

    #[test]
    fn discovery_topic_supports_device_scoped_shape() {
        assert_eq!(
            discovery_topic_parts(
                "homeassistant",
                "homeassistant/sensor/weather_station/outdoor/config"
            ),
            Some(("weather_station", "sensor", "outdoor"))
        );
        assert!(discovery_topic_parts(
            "homeassistant",
            "homeassistant/sensor/weather_station/outdoor/state"
        )
        .is_none());
    }

    #[test]
    fn publication_cursor_preserves_delivery_identity() {
        let mut publication = Publish::new("house/kitchen/state", QoS::AtLeastOnce, "ON");
        publication.pkid = 42;
        publication.retain = true;
        publication.dup = true;
        assert_eq!(
            publication_ref(&publication).unwrap().native_cursor(),
            "mqtt:house/kitchen/state:qos1:packet:42:retained:true:duplicate:true"
        );
    }

    #[test]
    fn transport_failure_is_terminal_and_audited() {
        let mut integration = integration();
        let mut runtime = SmartHomeRuntime::new();
        let installed = install_light(&mut integration, &mut runtime);
        let principal = AgentId::trusted("agent:mqtt-test");
        grant(&mut runtime, &principal);
        let accepted = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.binding.entity_id,
                    CommandType::TurnOn,
                    Value::Null,
                ),
                2_000,
            )
            .unwrap()
            .command_result;
        let failed =
            integration.record_transport_failure(&mut runtime, &accepted, "broker queue closed");
        assert_eq!(failed.status, CommandStatus::Failed);
        assert!(runtime.event_bus().published().iter().any(|event| {
            matches!(
                event,
                RuntimeEvent::CommandResult(result)
                    if result.command_id == failed.command_id
                        && result.status == CommandStatus::Failed
            )
        }));
    }

    #[test]
    fn host_connection_and_discovery_events_advance_live_state() {
        let mut host = MqttRuntimeHost::open(
            MqttBrokerConfig::new(
                BridgeId::trusted("mqtt-broker-1"),
                "mqtt.local",
                1883,
                "smart-home-test",
            ),
            None,
            SmartHomeRuntime::new(),
            1_000,
        )
        .unwrap();
        assert_eq!(host.subscribed_topics().len(), 2);
        assert_eq!(host.stream_state().status, EventStreamStatus::Connecting);

        let connected = host
            .handle_event(
                Event::Incoming(Incoming::ConnAck(ConnAck::new(
                    ConnectReturnCode::Success,
                    false,
                ))),
                1_010,
            )
            .unwrap();
        assert_eq!(connected, MqttHostOutcome::Connected);
        assert_eq!(host.stream_state().status, EventStreamStatus::Healthy);

        let outcome = host
            .handle_event(
                Event::Incoming(Incoming::Publish(discovery_publish())),
                1_020,
            )
            .unwrap();
        assert!(matches!(
            outcome,
            MqttHostOutcome::Publication(outcome)
                if matches!(*outcome, MqttIngestOutcome::Discovery(_))
        ));
        assert_eq!(host.subscribed_topics().len(), 4);
        assert!(host
            .runtime()
            .registry()
            .entity(&EntityId::trusted("mqtt-entity:kitchen-main"))
            .is_some());
    }

    #[test]
    fn refused_connection_marks_auth_failure() {
        let mut host = MqttRuntimeHost::open(
            MqttBrokerConfig::new(
                BridgeId::trusted("mqtt-broker-1"),
                "mqtt.local",
                1883,
                "smart-home-test",
            ),
            None,
            SmartHomeRuntime::new(),
            1_000,
        )
        .unwrap();
        let error = host
            .handle_event(
                Event::Incoming(Incoming::ConnAck(ConnAck::new(
                    ConnectReturnCode::NotAuthorized,
                    false,
                ))),
                1_010,
            )
            .unwrap_err();
        assert!(matches!(error, MqttIntegrationError::Connection(_)));
        assert_eq!(
            host.runtime()
                .registry()
                .bridge(&BridgeId::trusted("mqtt-broker-1"))
                .map(|bridge| bridge.health),
            Some(Health::AuthFailed)
        );
        assert_eq!(host.stream_state().status, EventStreamStatus::Disconnected);
    }

    #[test]
    fn real_tcp_session_discovers_subscribes_and_ingests_state() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let broker = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let connect = read_mqtt_packet(&mut stream);
            assert_eq!(connect[0], 0x10);
            stream.write_all(&[0x20, 0x02, 0x00, 0x00]).unwrap();

            for _ in 0..2 {
                let subscription = read_mqtt_packet(&mut stream);
                acknowledge_subscription(&mut stream, &subscription);
            }
            stream
                .write_all(&mqtt_publish(
                    "homeassistant/light/kitchen/config",
                    &discovery_publish().payload,
                    true,
                ))
                .unwrap();

            for _ in 0..2 {
                let subscription = read_mqtt_packet(&mut stream);
                acknowledge_subscription(&mut stream, &subscription);
            }
            stream
                .write_all(&mqtt_publish("house/kitchen/light/state", b"ON", true))
                .unwrap();
        });
        let mut host = MqttRuntimeHost::open(
            MqttBrokerConfig::new(
                BridgeId::trusted("mqtt-broker-tcp"),
                "127.0.0.1",
                port,
                "smart-home-tcp-test",
            ),
            None,
            SmartHomeRuntime::new(),
            1_000,
        )
        .unwrap();

        let mut discovered = false;
        let mut state_observed = false;
        for sequence in 0..32 {
            match host.poll_once(Duration::from_secs(1), 1_010 + sequence) {
                Ok(MqttHostOutcome::Publication(outcome)) => match *outcome {
                    MqttIngestOutcome::Discovery(_) => discovered = true,
                    MqttIngestOutcome::State(_) => {
                        state_observed = true;
                        break;
                    }
                    _ => {}
                },
                Ok(_) => {}
                Err(error) => panic!("unexpected host error: {error}"),
            }
        }
        broker.join().unwrap();

        assert!(discovered);
        assert!(state_observed);
        assert_eq!(
            host.runtime()
                .registry()
                .entity(&EntityId::trusted("mqtt-entity:kitchen-main"))
                .and_then(|entity| entity.state.as_ref())
                .map(|state| &state.value),
            Some(&Value::Object(vec![(
                "light.on_off".to_string(),
                Value::Bool(true)
            )]))
        );
    }
}
