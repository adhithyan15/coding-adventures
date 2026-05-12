//! Repository-owned smart-home vocabulary shared by integrations, tools, and
//! Chief of Staff agents.
//!
//! The types in this crate are intentionally protocol-neutral. A Hue light,
//! Zigbee endpoint, Z-Wave node value, Thread/Matter endpoint, or MQTT device
//! can all be projected into the same bridge/device/entity/event/command model.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, fmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartHomeError {
    EmptyIdentifier {
        kind: &'static str,
    },
    InvalidPercentage {
        value: u16,
    },
    InvalidMqttTopic {
        kind: &'static str,
        value: String,
        reason: &'static str,
    },
    MissingCapability {
        command_type: CommandType,
    },
}

impl fmt::Display for SmartHomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier { kind } => write!(f, "{kind} must not be empty"),
            Self::InvalidPercentage { value } => {
                write!(f, "percentage value {value} is outside 0..=100")
            }
            Self::InvalidMqttTopic {
                kind,
                value,
                reason,
            } => write!(f, "{kind} `{value}` is invalid: {reason}"),
            Self::MissingCapability { command_type } => {
                write!(
                    f,
                    "no canonical capability for command type {command_type:?}"
                )
            }
        }
    }
}

impl std::error::Error for SmartHomeError {}

macro_rules! id_type {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, SmartHomeError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(SmartHomeError::EmptyIdentifier { kind: $kind });
                }
                Ok(Self(value))
            }

            pub fn trusted(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

id_type!(IntegrationId, "integration id");
id_type!(BridgeId, "bridge id");
id_type!(DeviceId, "device id");
id_type!(EntityId, "entity id");
id_type!(SceneId, "scene id");
id_type!(CapabilityId, "capability id");
id_type!(CommandId, "command id");
id_type!(EventId, "event id");
id_type!(CorrelationId, "correlation id");
id_type!(VaultRef, "vault reference");
id_type!(AgentId, "agent id");
id_type!(CapabilityGrantId, "capability grant id");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    InProcessRust,
    RustWorkerProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeTransport {
    LanHttp,
    Mdns,
    Serial,
    Ble,
    Cloud,
    LocalProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Unknown,
    Discoverable,
    Unpaired,
    Online,
    Degraded,
    Offline,
    AuthFailed,
    Unsupported,
    Removed,
}

impl Health {
    pub fn is_online(self) -> bool {
        matches!(self, Self::Online)
    }

    pub fn is_pairing_candidate(self) -> bool {
        matches!(self, Self::Discoverable | Self::Unpaired)
    }

    pub fn needs_attention(self) -> bool {
        matches!(
            self,
            Self::Degraded | Self::Offline | Self::AuthFailed | Self::Unsupported | Self::Removed
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Light,
    LightGroup,
    Switch,
    Sensor,
    Lock,
    Thermostat,
    Scene,
    Input,
    BridgeHealth,
    NetworkDiagnostic,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityMode {
    Observe,
    Command,
    ObserveAndCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    Null,
    Boolean,
    Integer,
    Number,
    Percentage,
    Text,
    Object,
    Array,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Number(f64),
    Percentage(u8),
    Text(String),
    Object(Vec<(String, Value)>),
    Array(Vec<Value>),
}

impl Value {
    pub fn percentage(value: u16) -> Result<Self, SmartHomeError> {
        if value > 100 {
            return Err(SmartHomeError::InvalidPercentage { value });
        }
        Ok(Self::Percentage(value as u8))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Capability {
    pub capability_id: CapabilityId,
    pub mode: CapabilityMode,
    pub value_kind: ValueKind,
    pub unit: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
}

impl Capability {
    pub fn new(capability_id: CapabilityId, mode: CapabilityMode, value_kind: ValueKind) -> Self {
        Self {
            capability_id,
            mode,
            value_kind,
            unit: None,
            min: None,
            max: None,
            step: None,
        }
    }

    pub fn with_range(mut self, min: f64, max: f64, step: Option<f64>) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self.step = step;
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn light_on_off() -> Self {
        Self::new(
            CapabilityId::trusted("light.on_off"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Boolean,
        )
    }

    pub fn light_brightness() -> Self {
        Self::new(
            CapabilityId::trusted("light.brightness"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Percentage,
        )
        .with_range(0.0, 100.0, Some(1.0))
    }

    pub fn light_color() -> Self {
        Self::new(
            CapabilityId::trusted("light.color"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Object,
        )
    }

    pub fn light_color_temperature() -> Self {
        Self::new(
            CapabilityId::trusted("light.color_temperature"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Integer,
        )
        .with_unit("mirek")
    }

    pub fn scene_recall() -> Self {
        Self::new(
            CapabilityId::trusted("scene.recall"),
            CapabilityMode::Command,
            ValueKind::Null,
        )
    }

    pub fn lock_state() -> Self {
        Self::new(
            CapabilityId::trusted("lock.state"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Text,
        )
    }

    pub fn climate_setpoint() -> Self {
        Self::new(
            CapabilityId::trusted("climate.setpoint"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Number,
        )
        .with_unit("temperature")
    }

    pub fn sensor_occupancy() -> Self {
        Self::new(
            CapabilityId::trusted("sensor.occupancy"),
            CapabilityMode::Observe,
            ValueKind::Boolean,
        )
    }

    pub fn sensor_contact() -> Self {
        Self::new(
            CapabilityId::trusted("sensor.contact"),
            CapabilityMode::Observe,
            ValueKind::Boolean,
        )
    }

    pub fn sensor_temperature() -> Self {
        Self::new(
            CapabilityId::trusted("sensor.temperature"),
            CapabilityMode::Observe,
            ValueKind::Number,
        )
        .with_unit("temperature")
    }

    pub fn sensor_humidity() -> Self {
        Self::new(
            CapabilityId::trusted("sensor.humidity"),
            CapabilityMode::Observe,
            ValueKind::Percentage,
        )
        .with_range(0.0, 100.0, Some(1.0))
    }

    pub fn sensor_illuminance() -> Self {
        Self::new(
            CapabilityId::trusted("sensor.illuminance"),
            CapabilityMode::Observe,
            ValueKind::Number,
        )
        .with_unit("lux")
    }

    pub fn sensor_battery() -> Self {
        Self::new(
            CapabilityId::trusted("sensor.battery"),
            CapabilityMode::Observe,
            ValueKind::Percentage,
        )
        .with_range(0.0, 100.0, Some(1.0))
    }

    pub fn input_button() -> Self {
        Self::new(
            CapabilityId::trusted("input.button"),
            CapabilityMode::Observe,
            ValueKind::Text,
        )
    }
}

pub fn canonical_capability_catalog() -> Vec<Capability> {
    vec![
        Capability::light_on_off(),
        Capability::light_brightness(),
        Capability::light_color(),
        Capability::light_color_temperature(),
        Capability::scene_recall(),
        Capability::lock_state(),
        Capability::climate_setpoint(),
        Capability::sensor_occupancy(),
        Capability::sensor_contact(),
        Capability::sensor_temperature(),
        Capability::sensor_humidity(),
        Capability::sensor_illuminance(),
        Capability::sensor_battery(),
        Capability::input_button(),
    ]
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapabilitySurfaceSummary {
    pub total_capabilities: usize,
    pub observe_only_capabilities: usize,
    pub command_only_capabilities: usize,
    pub observe_and_command_capabilities: usize,
    pub null_values: usize,
    pub boolean_values: usize,
    pub integer_values: usize,
    pub number_values: usize,
    pub percentage_values: usize,
    pub text_values: usize,
    pub object_values: usize,
    pub array_values: usize,
    pub ranged_capabilities: usize,
}

impl CapabilitySurfaceSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_capabilities<'a, I>(capabilities: I) -> Self
    where
        I: IntoIterator<Item = &'a Capability>,
    {
        let mut summary = Self::empty();
        for capability in capabilities {
            summary.total_capabilities += 1;
            match capability.mode {
                CapabilityMode::Observe => summary.observe_only_capabilities += 1,
                CapabilityMode::Command => summary.command_only_capabilities += 1,
                CapabilityMode::ObserveAndCommand => {
                    summary.observe_and_command_capabilities += 1;
                }
            }
            match capability.value_kind {
                ValueKind::Null => summary.null_values += 1,
                ValueKind::Boolean => summary.boolean_values += 1,
                ValueKind::Integer => summary.integer_values += 1,
                ValueKind::Number => summary.number_values += 1,
                ValueKind::Percentage => summary.percentage_values += 1,
                ValueKind::Text => summary.text_values += 1,
                ValueKind::Object => summary.object_values += 1,
                ValueKind::Array => summary.array_values += 1,
            }
            if capability.min.is_some() && capability.max.is_some() {
                summary.ranged_capabilities += 1;
            }
        }
        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_capabilities == 0
    }

    pub fn observable_capabilities(&self) -> usize {
        self.observe_only_capabilities + self.observe_and_command_capabilities
    }

    pub fn commandable_capabilities(&self) -> usize {
        self.command_only_capabilities + self.observe_and_command_capabilities
    }

    pub fn has_observe_surface(&self) -> bool {
        self.observable_capabilities() > 0
    }

    pub fn has_command_surface(&self) -> bool {
        self.commandable_capabilities() > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolFamily {
    Hue,
    Zigbee,
    ZWave,
    Thread,
    Matter,
    Mqtt,
    Vendor(String),
}

impl ProtocolFamily {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Hue => "hue",
            Self::Zigbee => "zigbee",
            Self::ZWave => "zwave",
            Self::Thread => "thread",
            Self::Matter => "matter",
            Self::Mqtt => "mqtt",
            Self::Vendor(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolIdentifier {
    pub family: ProtocolFamily,
    pub kind: String,
    pub value: String,
}

impl ProtocolIdentifier {
    pub fn new(
        family: ProtocolFamily,
        kind: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, SmartHomeError> {
        let kind = kind.into();
        if kind.trim().is_empty() {
            return Err(SmartHomeError::EmptyIdentifier {
                kind: "protocol identifier kind",
            });
        }
        let value = value.into();
        if value.trim().is_empty() {
            return Err(SmartHomeError::EmptyIdentifier {
                kind: "protocol identifier value",
            });
        }
        Ok(Self {
            family,
            kind,
            value,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MqttTopicName(String);

impl MqttTopicName {
    pub fn new(value: impl Into<String>) -> Result<Self, SmartHomeError> {
        let value = value.into();
        validate_mqtt_topic_name(&value)?;
        Ok(Self(value))
    }

    pub fn trusted(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_protocol_identifier(&self, kind: impl Into<String>) -> ProtocolIdentifier {
        ProtocolIdentifier {
            family: ProtocolFamily::Mqtt,
            kind: kind.into(),
            value: self.0.clone(),
        }
    }
}

impl fmt::Display for MqttTopicName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MqttTopicFilter(String);

impl MqttTopicFilter {
    pub fn new(value: impl Into<String>) -> Result<Self, SmartHomeError> {
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

    pub fn matches(&self, topic: &MqttTopicName) -> bool {
        mqtt_filter_matches_topic(&self.0, topic.as_str())
    }
}

impl fmt::Display for MqttTopicFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttQualityOfService {
    AtMostOnce,
    AtLeastOnce,
    ExactlyOnce,
}

impl MqttQualityOfService {
    pub fn level(self) -> u8 {
        match self {
            Self::AtMostOnce => 0,
            Self::AtLeastOnce => 1,
            Self::ExactlyOnce => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqttTopicRole {
    Discovery,
    Availability,
    State,
    Command,
    Event,
}

impl MqttTopicRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Discovery => "discovery",
            Self::Availability => "availability",
            Self::State => "state",
            Self::Command => "command",
            Self::Event => "event",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MqttTopicBinding {
    pub role: MqttTopicRole,
    pub topic: MqttTopicName,
    pub qos: MqttQualityOfService,
    pub retain: bool,
}

impl MqttTopicBinding {
    pub fn new(role: MqttTopicRole, topic: MqttTopicName) -> Self {
        Self {
            role,
            topic,
            qos: MqttQualityOfService::AtLeastOnce,
            retain: false,
        }
    }

    pub fn with_qos(mut self, qos: MqttQualityOfService) -> Self {
        self.qos = qos;
        self
    }

    pub fn with_retain(mut self, retain: bool) -> Self {
        self.retain = retain;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub key: String,
    pub value: String,
}

impl Metadata {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationDescriptor {
    pub integration_id: IntegrationId,
    pub display_name: String,
    pub version: String,
    pub runtime_kind: RuntimeKind,
    pub capabilities: Vec<CapabilityId>,
    pub discovery_roles: Vec<String>,
    pub pairing_roles: Vec<String>,
}

impl IntegrationDescriptor {
    pub fn new(
        integration_id: IntegrationId,
        display_name: impl Into<String>,
        version: impl Into<String>,
        runtime_kind: RuntimeKind,
    ) -> Self {
        Self {
            integration_id,
            display_name: display_name.into(),
            version: version.into(),
            runtime_kind,
            capabilities: Vec::new(),
            discovery_roles: Vec::new(),
            pairing_roles: Vec::new(),
        }
    }

    pub fn with_capabilities<I>(mut self, capabilities: I) -> Self
    where
        I: IntoIterator<Item = CapabilityId>,
    {
        self.capabilities = capabilities.into_iter().collect();
        self
    }

    pub fn with_discovery_roles<I, S>(mut self, discovery_roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.discovery_roles = discovery_roles.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_pairing_roles<I, S>(mut self, pairing_roles: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.pairing_roles = pairing_roles.into_iter().map(Into::into).collect();
        self
    }

    pub fn supports_capability(&self, capability_id: &CapabilityId) -> bool {
        self.capabilities.contains(capability_id)
    }

    pub fn supports_discovery_role(&self, role: &str) -> bool {
        self.discovery_roles
            .iter()
            .any(|candidate| candidate == role)
    }

    pub fn supports_pairing_role(&self, role: &str) -> bool {
        self.pairing_roles.iter().any(|candidate| candidate == role)
    }

    pub fn is_discoverable(&self) -> bool {
        !self.discovery_roles.is_empty()
    }

    pub fn is_pairable(&self) -> bool {
        !self.pairing_roles.is_empty()
    }

    pub fn capability_count(&self) -> usize {
        self.capabilities.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegrationCatalogSummary {
    pub total_integrations: usize,
    pub in_process_rust_integrations: usize,
    pub rust_worker_process_integrations: usize,
    pub total_capability_mappings: usize,
    pub unique_capabilities: usize,
    pub total_discovery_roles: usize,
    pub total_pairing_roles: usize,
    pub discoverable_integrations: usize,
    pub pairable_integrations: usize,
}

impl IntegrationCatalogSummary {
    pub fn empty() -> Self {
        Self {
            total_integrations: 0,
            in_process_rust_integrations: 0,
            rust_worker_process_integrations: 0,
            total_capability_mappings: 0,
            unique_capabilities: 0,
            total_discovery_roles: 0,
            total_pairing_roles: 0,
            discoverable_integrations: 0,
            pairable_integrations: 0,
        }
    }

    pub fn from_descriptors<'a, I>(descriptors: I) -> Self
    where
        I: IntoIterator<Item = &'a IntegrationDescriptor>,
    {
        let mut summary = Self::empty();
        let mut capabilities = BTreeSet::new();

        for descriptor in descriptors {
            summary.total_integrations += 1;
            summary.total_capability_mappings += descriptor.capabilities.len();
            summary.total_discovery_roles += descriptor.discovery_roles.len();
            summary.total_pairing_roles += descriptor.pairing_roles.len();

            match descriptor.runtime_kind {
                RuntimeKind::InProcessRust => summary.in_process_rust_integrations += 1,
                RuntimeKind::RustWorkerProcess => summary.rust_worker_process_integrations += 1,
            }

            if descriptor.is_discoverable() {
                summary.discoverable_integrations += 1;
            }
            if descriptor.is_pairable() {
                summary.pairable_integrations += 1;
            }

            capabilities.extend(descriptor.capabilities.iter().cloned());
        }

        summary.unique_capabilities = capabilities.len();
        summary
    }

    pub fn all_integrations_discoverable(&self) -> bool {
        self.total_integrations == self.discoverable_integrations
    }

    pub fn all_integrations_pairable(&self) -> bool {
        self.total_integrations == self.pairable_integrations
    }
}

pub fn canonical_integration_catalog() -> Vec<IntegrationDescriptor> {
    vec![
        IntegrationDescriptor::new(
            IntegrationId::trusted("hue"),
            "Philips Hue Bridge",
            "0.1.0",
            RuntimeKind::RustWorkerProcess,
        )
        .with_capabilities(capability_ids([
            "light.on_off",
            "light.brightness",
            "light.color",
            "light.color_temperature",
            "scene.recall",
            "sensor.battery",
        ]))
        .with_discovery_roles(["mdns", "lan-http", "hue-bridge"])
        .with_pairing_roles(["link-button", "vault-token"]),
        IntegrationDescriptor::new(
            IntegrationId::trusted("zigbee"),
            "Zigbee Coordinator",
            "0.1.0",
            RuntimeKind::RustWorkerProcess,
        )
        .with_capabilities(capability_ids([
            "light.on_off",
            "light.brightness",
            "light.color",
            "light.color_temperature",
            "lock.state",
            "climate.setpoint",
            "sensor.occupancy",
            "sensor.contact",
            "sensor.temperature",
            "sensor.humidity",
            "sensor.illuminance",
            "sensor.battery",
            "input.button",
        ]))
        .with_discovery_roles(["serial-adapter", "network-steering", "permit-join"])
        .with_pairing_roles(["install-code", "touchlink", "permit-join"]),
        IntegrationDescriptor::new(
            IntegrationId::trusted("zwave"),
            "Z-Wave Controller",
            "0.1.0",
            RuntimeKind::RustWorkerProcess,
        )
        .with_capabilities(capability_ids([
            "light.on_off",
            "light.brightness",
            "lock.state",
            "climate.setpoint",
            "sensor.contact",
            "sensor.temperature",
            "sensor.humidity",
            "sensor.battery",
        ]))
        .with_discovery_roles(["serial-controller", "node-interview"])
        .with_pairing_roles(["inclusion", "smart-start"]),
        IntegrationDescriptor::new(
            IntegrationId::trusted("thread"),
            "Thread Border Router",
            "0.1.0",
            RuntimeKind::RustWorkerProcess,
        )
        .with_capabilities(capability_ids([
            "sensor.occupancy",
            "sensor.contact",
            "sensor.temperature",
            "sensor.humidity",
            "sensor.battery",
        ]))
        .with_discovery_roles(["border-router", "mesh-diagnostic"])
        .with_pairing_roles(["commissioning-dataset", "joiner"]),
        IntegrationDescriptor::new(
            IntegrationId::trusted("matter"),
            "Matter Controller",
            "0.1.0",
            RuntimeKind::RustWorkerProcess,
        )
        .with_capabilities(capability_ids([
            "light.on_off",
            "light.brightness",
            "light.color",
            "light.color_temperature",
            "lock.state",
            "climate.setpoint",
            "sensor.occupancy",
            "sensor.contact",
            "sensor.temperature",
            "sensor.humidity",
            "sensor.illuminance",
            "sensor.battery",
            "input.button",
        ]))
        .with_discovery_roles(["mdns", "fabric", "commissionable-node"])
        .with_pairing_roles(["commissioning-code", "fabric-join"]),
        IntegrationDescriptor::new(
            IntegrationId::trusted("mqtt"),
            "MQTT Bridge",
            "0.1.0",
            RuntimeKind::RustWorkerProcess,
        )
        .with_capabilities(capability_ids([
            "light.on_off",
            "light.brightness",
            "light.color",
            "light.color_temperature",
            "scene.recall",
            "lock.state",
            "climate.setpoint",
            "sensor.occupancy",
            "sensor.contact",
            "sensor.temperature",
            "sensor.humidity",
            "sensor.illuminance",
            "sensor.battery",
            "input.button",
        ]))
        .with_discovery_roles(["broker-subscribe", "home-assistant-discovery"])
        .with_pairing_roles(["broker-credentials", "topic-namespace"]),
    ]
}

pub fn canonical_integration_descriptor(
    integration_id: &IntegrationId,
) -> Option<IntegrationDescriptor> {
    canonical_integration_catalog()
        .into_iter()
        .find(|descriptor| &descriptor.integration_id == integration_id)
}

pub fn canonical_integrations_for_capability(
    capability_id: &CapabilityId,
) -> Vec<IntegrationDescriptor> {
    canonical_integration_catalog()
        .into_iter()
        .filter(|descriptor| descriptor.supports_capability(capability_id))
        .collect()
}

pub fn canonical_integration_catalog_summary() -> IntegrationCatalogSummary {
    let catalog = canonical_integration_catalog();
    IntegrationCatalogSummary::from_descriptors(catalog.iter())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bridge {
    pub bridge_id: BridgeId,
    pub integration_id: IntegrationId,
    pub transport: BridgeTransport,
    pub address: Option<String>,
    pub hardware_model: Option<String>,
    pub firmware_version: Option<String>,
    pub auth_ref: Option<VaultRef>,
    pub health: Health,
    pub last_seen_at_ms: Option<u64>,
    pub identifiers: Vec<ProtocolIdentifier>,
    pub metadata: Vec<Metadata>,
}

impl Bridge {
    pub fn new(
        bridge_id: BridgeId,
        integration_id: IntegrationId,
        transport: BridgeTransport,
    ) -> Self {
        Self {
            bridge_id,
            integration_id,
            transport,
            address: None,
            hardware_model: None,
            firmware_version: None,
            auth_ref: None,
            health: Health::Unknown,
            last_seen_at_ms: None,
            identifiers: Vec::new(),
            metadata: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub device_id: DeviceId,
    pub bridge_id: BridgeId,
    pub manufacturer: String,
    pub model: String,
    pub name: String,
    pub serial: Option<String>,
    pub firmware_version: Option<String>,
    pub room_id: Option<String>,
    pub entity_ids: Vec<EntityId>,
    pub identifiers: Vec<ProtocolIdentifier>,
    pub health: Health,
    pub metadata: Vec<Metadata>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub entity_id: EntityId,
    pub device_id: DeviceId,
    pub kind: EntityKind,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub state: Option<StateSnapshot>,
    pub metadata: Vec<Metadata>,
}

impl Entity {
    pub fn capability_summary(&self) -> CapabilitySurfaceSummary {
        CapabilitySurfaceSummary::from_capabilities(&self.capabilities)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateSource {
    EventStream,
    Poll,
    OptimisticCommand,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateConfidence {
    Confirmed,
    Optimistic,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateSnapshot {
    pub entity_id: EntityId,
    pub value: Value,
    pub source: StateSource,
    pub observed_at_ms: u64,
    pub received_at_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub confidence: StateConfidence,
}

impl StateSnapshot {
    pub fn is_stale_at(&self, now_ms: u64) -> bool {
        self.confidence == StateConfidence::Stale
            || self.expires_at_ms.is_some_and(|expires| now_ms >= expires)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SmartHomeInventorySummary {
    pub total_bridges: usize,
    pub online_bridges: usize,
    pub pairing_candidate_bridges: usize,
    pub bridges_needing_attention: usize,
    pub total_devices: usize,
    pub online_devices: usize,
    pub pairing_candidate_devices: usize,
    pub devices_needing_attention: usize,
    pub total_entities: usize,
    pub entities_with_state: usize,
    pub stale_entities: usize,
    pub commandable_entities: usize,
}

impl SmartHomeInventorySummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_inventory<'a, BI, DI, EI>(
        bridges: BI,
        devices: DI,
        entities: EI,
        now_ms: u64,
    ) -> Self
    where
        BI: IntoIterator<Item = &'a Bridge>,
        DI: IntoIterator<Item = &'a Device>,
        EI: IntoIterator<Item = &'a Entity>,
    {
        let mut summary = Self::empty();

        for bridge in bridges {
            summary.total_bridges += 1;
            if bridge.health.is_online() {
                summary.online_bridges += 1;
            }
            if bridge.health.is_pairing_candidate() {
                summary.pairing_candidate_bridges += 1;
            }
            if bridge.health.needs_attention() {
                summary.bridges_needing_attention += 1;
            }
        }

        for device in devices {
            summary.total_devices += 1;
            if device.health.is_online() {
                summary.online_devices += 1;
            }
            if device.health.is_pairing_candidate() {
                summary.pairing_candidate_devices += 1;
            }
            if device.health.needs_attention() {
                summary.devices_needing_attention += 1;
            }
        }

        for entity in entities {
            summary.total_entities += 1;
            if let Some(state) = entity.state.as_ref() {
                summary.entities_with_state += 1;
                if state.is_stale_at(now_ms) {
                    summary.stale_entities += 1;
                }
            }
            if entity.capability_summary().has_command_surface() {
                summary.commandable_entities += 1;
            }
        }

        summary
    }

    pub fn is_empty(&self) -> bool {
        self.total_bridges == 0 && self.total_devices == 0 && self.total_entities == 0
    }

    pub fn has_pairing_candidates(&self) -> bool {
        self.pairing_candidate_bridges > 0 || self.pairing_candidate_devices > 0
    }

    pub fn needs_attention(&self) -> bool {
        self.bridges_needing_attention > 0 || self.devices_needing_attention > 0
    }

    pub fn has_stale_state(&self) -> bool {
        self.stale_entities > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceEventType {
    Discovered,
    Updated,
    Removed,
    Unavailable,
    Error,
    Health,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateDelta {
    pub capability_id: CapabilityId,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceEvent {
    pub event_id: EventId,
    pub bridge_id: BridgeId,
    pub device_id: Option<DeviceId>,
    pub entity_id: Option<EntityId>,
    pub observed_at_ms: u64,
    pub received_at_ms: u64,
    pub event_type: DeviceEventType,
    pub state_delta: Option<StateDelta>,
    pub raw_ref: Option<String>,
    pub correlation_id: Option<CorrelationId>,
    pub metadata: Vec<Metadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    TurnOn,
    TurnOff,
    SetBrightness,
    SetColor,
    SetColorTemperature,
    RecallScene,
    SetLock,
    SetThermostatSetpoint,
}

impl CommandType {
    pub fn canonical_capability_id(self) -> Option<CapabilityId> {
        match self {
            Self::TurnOn | Self::TurnOff => Some(CapabilityId::trusted("light.on_off")),
            Self::SetBrightness => Some(CapabilityId::trusted("light.brightness")),
            Self::SetColor => Some(CapabilityId::trusted("light.color")),
            Self::SetColorTemperature => Some(CapabilityId::trusted("light.color_temperature")),
            Self::RecallScene => Some(CapabilityId::trusted("scene.recall")),
            Self::SetLock => Some(CapabilityId::trusted("lock.state")),
            Self::SetThermostatSetpoint => Some(CapabilityId::trusted("climate.setpoint")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegeTier {
    ReadOnly,
    LowRisk,
    HumanApproval,
    HighRisk,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceCommand {
    pub command_id: CommandId,
    pub entity_id: EntityId,
    pub command_type: CommandType,
    pub arguments: Value,
    pub requested_by: String,
    pub idempotency_key: Option<String>,
    pub required_tier: PrivilegeTier,
    pub required_capabilities: Vec<CapabilityId>,
    pub timeout_ms: u64,
    pub correlation_id: CorrelationId,
}

impl DeviceCommand {
    pub fn new(
        command_id: CommandId,
        entity_id: EntityId,
        command_type: CommandType,
        arguments: Value,
        requested_by: impl Into<String>,
        correlation_id: CorrelationId,
    ) -> Result<Self, SmartHomeError> {
        let capability = command_type
            .canonical_capability_id()
            .ok_or(SmartHomeError::MissingCapability { command_type })?;
        Ok(Self {
            command_id,
            entity_id,
            command_type,
            arguments,
            requested_by: requested_by.into(),
            idempotency_key: None,
            required_tier: tier_for_command(command_type),
            required_capabilities: vec![capability],
            timeout_ms: 5_000,
            correlation_id,
        })
    }
}

pub fn tier_for_command(command_type: CommandType) -> PrivilegeTier {
    match command_type {
        CommandType::SetLock => PrivilegeTier::HighRisk,
        CommandType::SetThermostatSetpoint => PrivilegeTier::HumanApproval,
        CommandType::TurnOn
        | CommandType::TurnOff
        | CommandType::SetBrightness
        | CommandType::SetColor
        | CommandType::SetColorTemperature
        | CommandType::RecallScene => PrivilegeTier::LowRisk,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandStatus {
    Accepted,
    Rejected,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub command_id: CommandId,
    pub status: CommandStatus,
    pub bridge_id: BridgeId,
    pub correlation_id: CorrelationId,
    pub message: Option<String>,
}

impl CommandStatus {
    pub fn is_accepted(self) -> bool {
        matches!(self, Self::Accepted)
    }

    pub fn is_rejected(self) -> bool {
        matches!(self, Self::Rejected)
    }

    pub fn is_failure(self) -> bool {
        matches!(self, Self::Rejected | Self::TimedOut | Self::Failed)
    }

    pub fn timed_out(self) -> bool {
        matches!(self, Self::TimedOut)
    }
}

impl CommandResult {
    pub fn is_accepted(&self) -> bool {
        self.status.is_accepted()
    }

    pub fn is_rejected(&self) -> bool {
        self.status.is_rejected()
    }

    pub fn is_failure(&self) -> bool {
        self.status.is_failure()
    }

    pub fn timed_out(&self) -> bool {
        self.status.timed_out()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneScope {
    Room,
    Zone,
    Home,
    Bridge,
    Custom,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneAction {
    pub entity_id: EntityId,
    pub desired_state: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub scene_id: SceneId,
    pub scope: SceneScope,
    pub native_ref: Option<ProtocolIdentifier>,
    pub actions: Vec<SceneAction>,
    pub metadata: Vec<Metadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSideEffects {
    None,
    Read,
    Write,
    External,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolDescriptor {
    pub tool_id: &'static str,
    pub side_effects: ToolSideEffects,
    pub required_capabilities: Vec<CapabilityId>,
    pub required_tier: PrivilegeTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartHomeToolCatalogSummary {
    pub total_tools: usize,
    pub read_tools: usize,
    pub write_tools: usize,
    pub external_tools: usize,
    pub read_only_tier_tools: usize,
    pub low_risk_tier_tools: usize,
    pub high_risk_tier_tools: usize,
    pub human_approval_tier_tools: usize,
    pub total_required_capabilities: usize,
}

impl SmartHomeToolCatalogSummary {
    pub fn empty() -> Self {
        Self {
            total_tools: 0,
            read_tools: 0,
            write_tools: 0,
            external_tools: 0,
            read_only_tier_tools: 0,
            low_risk_tier_tools: 0,
            high_risk_tier_tools: 0,
            human_approval_tier_tools: 0,
            total_required_capabilities: 0,
        }
    }

    pub fn from_descriptors<'a, I>(descriptors: I) -> Self
    where
        I: IntoIterator<Item = &'a ToolDescriptor>,
    {
        let mut summary = Self::empty();
        for descriptor in descriptors {
            summary.total_tools += 1;
            summary.total_required_capabilities += descriptor.required_capabilities.len();
            match descriptor.side_effects {
                ToolSideEffects::None => {}
                ToolSideEffects::Read => summary.read_tools += 1,
                ToolSideEffects::Write => summary.write_tools += 1,
                ToolSideEffects::External => summary.external_tools += 1,
            }
            match descriptor.required_tier {
                PrivilegeTier::ReadOnly => summary.read_only_tier_tools += 1,
                PrivilegeTier::LowRisk => summary.low_risk_tier_tools += 1,
                PrivilegeTier::HighRisk => summary.high_risk_tier_tools += 1,
                PrivilegeTier::HumanApproval => summary.human_approval_tier_tools += 1,
            }
        }
        summary
    }

    pub fn risky_tool_count(&self) -> usize {
        self.write_tools + self.external_tools
    }

    pub fn approval_gated_tool_count(&self) -> usize {
        self.human_approval_tier_tools
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartHomeTool {
    Discover,
    PairBridge,
    ListBridges,
    ListDevices,
    GetState,
    Command,
    Subscribe,
    DescribeCapabilities,
    GetHealth,
    ObserveSupervision,
}

impl SmartHomeTool {
    pub fn descriptor(self) -> ToolDescriptor {
        match self {
            Self::Discover => ToolDescriptor {
                tool_id: "smart_home.discover",
                side_effects: ToolSideEffects::Read,
                required_capabilities: vec![CapabilityId::trusted("smart_home.read")],
                required_tier: PrivilegeTier::ReadOnly,
            },
            Self::PairBridge => ToolDescriptor {
                tool_id: "smart_home.pair_bridge",
                side_effects: ToolSideEffects::External,
                required_capabilities: vec![CapabilityId::trusted("smart_home.pair")],
                required_tier: PrivilegeTier::HumanApproval,
            },
            Self::ListBridges => read_tool("smart_home.list_bridges"),
            Self::ListDevices => read_tool("smart_home.list_devices"),
            Self::GetState => read_tool("smart_home.get_state"),
            Self::Command => ToolDescriptor {
                tool_id: "smart_home.command",
                side_effects: ToolSideEffects::External,
                required_capabilities: vec![CapabilityId::trusted("smart_home.command.light")],
                required_tier: PrivilegeTier::LowRisk,
            },
            Self::Subscribe => read_tool("smart_home.subscribe"),
            Self::DescribeCapabilities => read_tool("smart_home.describe_capabilities"),
            Self::GetHealth => read_tool("smart_home.get_health"),
            Self::ObserveSupervision => read_tool("smart_home.observe_supervision"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityGrantStatus {
    Pending,
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityGrantScope {
    Tool(SmartHomeTool),
    Capability(CapabilityId),
    EntityCapability {
        entity_id: EntityId,
        capability_id: CapabilityId,
    },
    AllSmartHome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGrant {
    pub grant_id: CapabilityGrantId,
    pub principal_id: AgentId,
    pub scope: CapabilityGrantScope,
    pub max_tier: PrivilegeTier,
    pub granted_by: String,
    pub granted_at_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub status: CapabilityGrantStatus,
    pub metadata: Vec<Metadata>,
}

impl CapabilityGrant {
    pub fn new(
        grant_id: CapabilityGrantId,
        principal_id: AgentId,
        scope: CapabilityGrantScope,
        max_tier: PrivilegeTier,
        granted_by: impl Into<String>,
        granted_at_ms: u64,
    ) -> Self {
        Self {
            grant_id,
            principal_id,
            scope,
            max_tier,
            granted_by: granted_by.into(),
            granted_at_ms,
            expires_at_ms: None,
            status: CapabilityGrantStatus::Active,
            metadata: Vec::new(),
        }
    }

    pub fn for_tool(
        grant_id: CapabilityGrantId,
        principal_id: AgentId,
        tool: SmartHomeTool,
        granted_by: impl Into<String>,
        granted_at_ms: u64,
    ) -> Self {
        let descriptor = tool.descriptor();
        Self::new(
            grant_id,
            principal_id,
            CapabilityGrantScope::Tool(tool),
            descriptor.required_tier,
            granted_by,
            granted_at_ms,
        )
    }

    pub fn for_capability(
        grant_id: CapabilityGrantId,
        principal_id: AgentId,
        capability_id: CapabilityId,
        max_tier: PrivilegeTier,
        granted_by: impl Into<String>,
        granted_at_ms: u64,
    ) -> Self {
        Self::new(
            grant_id,
            principal_id,
            CapabilityGrantScope::Capability(capability_id),
            max_tier,
            granted_by,
            granted_at_ms,
        )
    }

    pub fn for_entity_capability(
        grant_id: CapabilityGrantId,
        principal_id: AgentId,
        entity_id: EntityId,
        capability_id: CapabilityId,
        max_tier: PrivilegeTier,
        granted_by: impl Into<String>,
        granted_at_ms: u64,
    ) -> Self {
        Self::new(
            grant_id,
            principal_id,
            CapabilityGrantScope::EntityCapability {
                entity_id,
                capability_id,
            },
            max_tier,
            granted_by,
            granted_at_ms,
        )
    }

    pub fn for_all_smart_home(
        grant_id: CapabilityGrantId,
        principal_id: AgentId,
        max_tier: PrivilegeTier,
        granted_by: impl Into<String>,
        granted_at_ms: u64,
    ) -> Self {
        Self::new(
            grant_id,
            principal_id,
            CapabilityGrantScope::AllSmartHome,
            max_tier,
            granted_by,
            granted_at_ms,
        )
    }

    pub fn with_expiry(mut self, expires_at_ms: u64) -> Self {
        self.expires_at_ms = Some(expires_at_ms);
        self
    }

    pub fn with_status(mut self, status: CapabilityGrantStatus) -> Self {
        self.status = status;
        self
    }

    pub fn status_at(&self, now_ms: u64) -> CapabilityGrantStatus {
        if self.status == CapabilityGrantStatus::Active
            && self.expires_at_ms.is_some_and(|expires| now_ms >= expires)
        {
            CapabilityGrantStatus::Expired
        } else {
            self.status
        }
    }

    pub fn is_active_at(&self, now_ms: u64) -> bool {
        self.status_at(now_ms) == CapabilityGrantStatus::Active
    }

    pub fn covers_capability(&self, capability_id: &CapabilityId) -> bool {
        match &self.scope {
            CapabilityGrantScope::Capability(granted) => granted == capability_id,
            CapabilityGrantScope::EntityCapability {
                capability_id: granted,
                ..
            } => granted == capability_id,
            CapabilityGrantScope::AllSmartHome => true,
            CapabilityGrantScope::Tool(_) => false,
        }
    }

    pub fn covers_tool(&self, tool: SmartHomeTool) -> bool {
        match &self.scope {
            CapabilityGrantScope::Tool(granted) => *granted == tool,
            CapabilityGrantScope::AllSmartHome => true,
            CapabilityGrantScope::Capability(_) | CapabilityGrantScope::EntityCapability { .. } => {
                false
            }
        }
    }

    pub fn allows_tool_at(&self, tool: SmartHomeTool, principal_id: &AgentId, now_ms: u64) -> bool {
        self.principal_id == *principal_id
            && self.is_active_at(now_ms)
            && self.max_tier >= tool.descriptor().required_tier
            && self.covers_tool(tool)
    }
}

impl ToolDescriptor {
    pub fn is_satisfied_by<'a, I>(&self, principal_id: &AgentId, grants: I, now_ms: u64) -> bool
    where
        I: IntoIterator<Item = &'a CapabilityGrant>,
    {
        let grants = grants.into_iter().collect::<Vec<_>>();
        self.required_capabilities.iter().all(|required| {
            grant_covers_descriptor_capability(self, principal_id, &grants, required, now_ms)
        })
    }

    pub fn requires_human_approval(&self) -> bool {
        self.required_tier == PrivilegeTier::HumanApproval
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationOutcome {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationSubject {
    Tool(SmartHomeTool),
    Command {
        command_id: CommandId,
        entity_id: EntityId,
        command_type: CommandType,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationDecision {
    pub principal_id: AgentId,
    pub subject: AuthorizationSubject,
    pub outcome: AuthorizationOutcome,
    pub required_tier: PrivilegeTier,
    pub required_capabilities: Vec<CapabilityId>,
    pub matched_grants: Vec<CapabilityGrantId>,
    pub missing_capabilities: Vec<CapabilityId>,
    pub decided_at_ms: u64,
}

impl AuthorizationDecision {
    pub fn for_tool<'a, I>(
        principal_id: AgentId,
        tool: SmartHomeTool,
        grants: I,
        decided_at_ms: u64,
    ) -> Self
    where
        I: IntoIterator<Item = &'a CapabilityGrant>,
    {
        let descriptor = tool.descriptor();
        let grants = grants.into_iter().collect::<Vec<_>>();
        let (matched_grants, missing_capabilities) =
            evaluate_required_capabilities(&descriptor, &principal_id, &grants, decided_at_ms);
        let outcome = if missing_capabilities.is_empty() {
            AuthorizationOutcome::Allowed
        } else {
            AuthorizationOutcome::Denied
        };
        Self {
            principal_id,
            subject: AuthorizationSubject::Tool(tool),
            outcome,
            required_tier: descriptor.required_tier,
            required_capabilities: descriptor.required_capabilities,
            matched_grants,
            missing_capabilities,
            decided_at_ms,
        }
    }

    pub fn for_command<'a, I>(
        principal_id: AgentId,
        command: &DeviceCommand,
        grants: I,
        decided_at_ms: u64,
    ) -> Self
    where
        I: IntoIterator<Item = &'a CapabilityGrant>,
    {
        let grants = grants.into_iter().collect::<Vec<_>>();
        let (matched_grants, missing_capabilities) =
            evaluate_command_capabilities(command, &principal_id, &grants, decided_at_ms);
        let outcome = if missing_capabilities.is_empty() {
            AuthorizationOutcome::Allowed
        } else {
            AuthorizationOutcome::Denied
        };
        Self {
            principal_id,
            subject: AuthorizationSubject::Command {
                command_id: command.command_id.clone(),
                entity_id: command.entity_id.clone(),
                command_type: command.command_type,
            },
            outcome,
            required_tier: command.required_tier,
            required_capabilities: command.required_capabilities.clone(),
            matched_grants,
            missing_capabilities,
            decided_at_ms,
        }
    }

    pub fn is_allowed(&self) -> bool {
        self.outcome == AuthorizationOutcome::Allowed
    }
}

pub fn smart_home_tool_catalog() -> Vec<ToolDescriptor> {
    [
        SmartHomeTool::Discover,
        SmartHomeTool::PairBridge,
        SmartHomeTool::ListBridges,
        SmartHomeTool::ListDevices,
        SmartHomeTool::GetState,
        SmartHomeTool::Command,
        SmartHomeTool::Subscribe,
        SmartHomeTool::DescribeCapabilities,
        SmartHomeTool::GetHealth,
        SmartHomeTool::ObserveSupervision,
    ]
    .into_iter()
    .map(SmartHomeTool::descriptor)
    .collect()
}

pub fn smart_home_tool_catalog_summary() -> SmartHomeToolCatalogSummary {
    let catalog = smart_home_tool_catalog();
    SmartHomeToolCatalogSummary::from_descriptors(catalog.iter())
}

fn evaluate_required_capabilities(
    descriptor: &ToolDescriptor,
    principal_id: &AgentId,
    grants: &[&CapabilityGrant],
    now_ms: u64,
) -> (Vec<CapabilityGrantId>, Vec<CapabilityId>) {
    let mut matched_grants = Vec::new();
    let mut missing_capabilities = Vec::new();
    for capability_id in &descriptor.required_capabilities {
        let matches = grants
            .iter()
            .filter(|grant| {
                grant_covers_descriptor_capability(
                    descriptor,
                    principal_id,
                    &[*grant],
                    capability_id,
                    now_ms,
                )
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            missing_capabilities.push(capability_id.clone());
        } else {
            for grant in matches {
                push_unique_grant_id(&mut matched_grants, grant.grant_id.clone());
            }
        }
    }
    (matched_grants, missing_capabilities)
}

fn evaluate_command_capabilities(
    command: &DeviceCommand,
    principal_id: &AgentId,
    grants: &[&CapabilityGrant],
    now_ms: u64,
) -> (Vec<CapabilityGrantId>, Vec<CapabilityId>) {
    let mut matched_grants = Vec::new();
    let mut missing_capabilities = Vec::new();
    for capability_id in &command.required_capabilities {
        let matches = grants
            .iter()
            .filter(|grant| {
                grant_covers_command_capability(grant, principal_id, command, capability_id, now_ms)
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            missing_capabilities.push(capability_id.clone());
        } else {
            for grant in matches {
                push_unique_grant_id(&mut matched_grants, grant.grant_id.clone());
            }
        }
    }
    (matched_grants, missing_capabilities)
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

fn grant_covers_descriptor_capability(
    descriptor: &ToolDescriptor,
    principal_id: &AgentId,
    grants: &[&CapabilityGrant],
    capability_id: &CapabilityId,
    now_ms: u64,
) -> bool {
    grants.iter().any(|grant| {
        grant.principal_id == *principal_id
            && grant.is_active_at(now_ms)
            && grant.max_tier >= descriptor.required_tier
            && (grant.covers_capability(capability_id)
                || grant_covers_tool_descriptor(grant, descriptor))
    })
}

fn grant_covers_tool_descriptor(grant: &CapabilityGrant, descriptor: &ToolDescriptor) -> bool {
    match &grant.scope {
        CapabilityGrantScope::Tool(tool) => tool.descriptor().tool_id == descriptor.tool_id,
        CapabilityGrantScope::AllSmartHome => true,
        CapabilityGrantScope::Capability(_) | CapabilityGrantScope::EntityCapability { .. } => {
            false
        }
    }
}

fn push_unique_grant_id(values: &mut Vec<CapabilityGrantId>, value: CapabilityGrantId) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn capability_ids<const N: usize>(values: [&str; N]) -> Vec<CapabilityId> {
    values.into_iter().map(CapabilityId::trusted).collect()
}

fn validate_mqtt_topic_name(value: &str) -> Result<(), SmartHomeError> {
    if value.is_empty() {
        return Err(invalid_mqtt_topic(
            "mqtt topic name",
            value,
            "must not be empty",
        ));
    }
    if value.contains('\0') {
        return Err(invalid_mqtt_topic(
            "mqtt topic name",
            value,
            "must not contain null bytes",
        ));
    }
    if value.contains('+') || value.contains('#') {
        return Err(invalid_mqtt_topic(
            "mqtt topic name",
            value,
            "wildcards are only valid in topic filters",
        ));
    }
    Ok(())
}

fn validate_mqtt_topic_filter(value: &str) -> Result<(), SmartHomeError> {
    if value.is_empty() {
        return Err(invalid_mqtt_topic(
            "mqtt topic filter",
            value,
            "must not be empty",
        ));
    }
    if value.contains('\0') {
        return Err(invalid_mqtt_topic(
            "mqtt topic filter",
            value,
            "must not contain null bytes",
        ));
    }

    let levels = value.split('/').collect::<Vec<_>>();
    for (index, level) in levels.iter().enumerate() {
        if level.contains('#') && *level != "#" {
            return Err(invalid_mqtt_topic(
                "mqtt topic filter",
                value,
                "`#` must occupy an entire topic level",
            ));
        }
        if *level == "#" && index + 1 != levels.len() {
            return Err(invalid_mqtt_topic(
                "mqtt topic filter",
                value,
                "`#` must be the final topic level",
            ));
        }
        if level.contains('+') && *level != "+" {
            return Err(invalid_mqtt_topic(
                "mqtt topic filter",
                value,
                "`+` must occupy an entire topic level",
            ));
        }
    }
    Ok(())
}

fn invalid_mqtt_topic(kind: &'static str, value: &str, reason: &'static str) -> SmartHomeError {
    SmartHomeError::InvalidMqttTopic {
        kind,
        value: value.to_string(),
        reason,
    }
}

fn mqtt_filter_matches_topic(filter: &str, topic: &str) -> bool {
    let filter_levels = filter.split('/').collect::<Vec<_>>();
    let topic_levels = topic.split('/').collect::<Vec<_>>();
    let mut topic_index = 0usize;

    for filter_level in &filter_levels {
        match *filter_level {
            "#" => return true,
            "+" => {
                if topic_index >= topic_levels.len() {
                    return false;
                }
                topic_index += 1;
            }
            literal => {
                if topic_levels.get(topic_index) != Some(&literal) {
                    return false;
                }
                topic_index += 1;
            }
        }
    }

    topic_index == topic_levels.len()
}

fn read_tool(tool_id: &'static str) -> ToolDescriptor {
    ToolDescriptor {
        tool_id,
        side_effects: ToolSideEffects::Read,
        required_capabilities: vec![CapabilityId::trusted("smart_home.read")],
        required_tier: PrivilegeTier::ReadOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_reject_empty_values() {
        assert_eq!(
            BridgeId::new("   "),
            Err(SmartHomeError::EmptyIdentifier { kind: "bridge id" })
        );
        assert_eq!(
            EntityId::new("light.kitchen").unwrap().as_str(),
            "light.kitchen"
        );
    }

    #[test]
    fn protocol_identifiers_keep_native_ids_out_of_entity_ids() {
        let hue = ProtocolIdentifier::new(
            ProtocolFamily::Hue,
            "light",
            "25a6d2a2-5f19-452e-a944-9d0b75fb3b2d",
        )
        .unwrap();
        let zigbee =
            ProtocolIdentifier::new(ProtocolFamily::Zigbee, "ieee_address", "0x00124b0024c8abcd")
                .unwrap();

        assert_ne!(hue.family, zigbee.family);
        assert_eq!(hue.kind, "light");
        assert_eq!(zigbee.kind, "ieee_address");
    }

    #[test]
    fn mqtt_topic_names_reject_filter_wildcards() {
        assert_eq!(
            MqttTopicName::new("home/+/state"),
            Err(SmartHomeError::InvalidMqttTopic {
                kind: "mqtt topic name",
                value: "home/+/state".to_string(),
                reason: "wildcards are only valid in topic filters",
            })
        );
        assert_eq!(
            MqttTopicName::new("home/kitchen/light/state")
                .unwrap()
                .as_str(),
            "home/kitchen/light/state"
        );
    }

    #[test]
    fn mqtt_topic_filters_validate_wildcard_shape() {
        assert!(MqttTopicFilter::new("home/+/state").is_ok());
        assert!(MqttTopicFilter::new("home/#").is_ok());
        assert!(MqttTopicFilter::new("home/#/state").is_err());
        assert!(MqttTopicFilter::new("home/te+st/state").is_err());
    }

    #[test]
    fn mqtt_topic_filters_match_topic_names() {
        let kitchen_state = MqttTopicName::new("home/kitchen/light/state").unwrap();
        let kitchen_command = MqttTopicName::new("home/kitchen/light/set").unwrap();

        assert!(MqttTopicFilter::new("home/+/light/state")
            .unwrap()
            .matches(&kitchen_state));
        assert!(MqttTopicFilter::new("home/#")
            .unwrap()
            .matches(&kitchen_state));
        assert!(!MqttTopicFilter::new("home/+/light/state")
            .unwrap()
            .matches(&kitchen_command));
    }

    #[test]
    fn mqtt_topic_bindings_capture_role_qos_and_retain_policy() {
        let binding = MqttTopicBinding::new(
            MqttTopicRole::State,
            MqttTopicName::new("home/kitchen/light/state").unwrap(),
        )
        .with_qos(MqttQualityOfService::AtLeastOnce)
        .with_retain(true);

        assert_eq!(binding.role.as_str(), "state");
        assert_eq!(binding.qos.level(), 1);
        assert!(binding.retain);
        assert_eq!(
            binding
                .topic
                .as_protocol_identifier("state_topic")
                .family
                .as_str(),
            "mqtt"
        );
    }

    #[test]
    fn command_constructor_sets_policy_shape() {
        let command = DeviceCommand::new(
            CommandId::trusted("cmd-1"),
            EntityId::trusted("entity.light.kitchen"),
            CommandType::SetBrightness,
            Value::percentage(42).unwrap(),
            "agent:lighting-planner",
            CorrelationId::trusted("corr-1"),
        )
        .unwrap();

        assert_eq!(command.required_tier, PrivilegeTier::LowRisk);
        assert_eq!(
            command.required_capabilities,
            vec![CapabilityId::trusted("light.brightness")]
        );
    }

    #[test]
    fn high_risk_commands_are_tiered_differently() {
        assert_eq!(
            tier_for_command(CommandType::SetLock),
            PrivilegeTier::HighRisk
        );
        assert_eq!(
            tier_for_command(CommandType::SetThermostatSetpoint),
            PrivilegeTier::HumanApproval
        );
    }

    #[test]
    fn state_snapshot_knows_staleness() {
        let snapshot = StateSnapshot {
            entity_id: EntityId::trusted("entity.light.kitchen"),
            value: Value::Bool(true),
            source: StateSource::OptimisticCommand,
            observed_at_ms: 1_000,
            received_at_ms: 1_001,
            expires_at_ms: Some(2_000),
            confidence: StateConfidence::Optimistic,
        };

        assert!(!snapshot.is_stale_at(1_999));
        assert!(snapshot.is_stale_at(2_000));
    }

    #[test]
    fn capability_surface_summary_counts_modes_and_value_shapes() {
        let entity = Entity {
            entity_id: EntityId::trusted("entity.light.kitchen"),
            device_id: DeviceId::trusted("device.bridge.light-1"),
            kind: EntityKind::Light,
            name: "Kitchen".to_string(),
            capabilities: vec![
                Capability::light_on_off(),
                Capability::light_brightness(),
                Capability::sensor_occupancy(),
                Capability::new(
                    CapabilityId::trusted("diagnostic.payload"),
                    CapabilityMode::Observe,
                    ValueKind::Object,
                ),
                Capability::new(
                    CapabilityId::trusted("input.mode"),
                    CapabilityMode::Command,
                    ValueKind::Text,
                ),
            ],
            state: None,
            metadata: Vec::new(),
        };

        let summary = entity.capability_summary();

        assert_eq!(
            summary,
            CapabilitySurfaceSummary {
                total_capabilities: 5,
                observe_only_capabilities: 2,
                command_only_capabilities: 1,
                observe_and_command_capabilities: 2,
                null_values: 0,
                boolean_values: 2,
                integer_values: 0,
                number_values: 0,
                percentage_values: 1,
                text_values: 1,
                object_values: 1,
                array_values: 0,
                ranged_capabilities: 1,
            }
        );
        assert_eq!(summary.observable_capabilities(), 4);
        assert_eq!(summary.commandable_capabilities(), 3);
        assert!(summary.has_observe_surface());
        assert!(summary.has_command_surface());
        assert!(!summary.is_empty());

        let empty = CapabilitySurfaceSummary::from_capabilities([]);
        assert!(empty.is_empty());
        assert!(!empty.has_observe_surface());
        assert!(!empty.has_command_surface());
    }

    #[test]
    fn health_helpers_classify_pairing_online_and_attention_states() {
        assert!(Health::Online.is_online());
        assert!(!Health::Degraded.is_online());
        assert!(Health::Discoverable.is_pairing_candidate());
        assert!(Health::Unpaired.is_pairing_candidate());
        assert!(!Health::Online.is_pairing_candidate());
        assert!(Health::Degraded.needs_attention());
        assert!(Health::Offline.needs_attention());
        assert!(Health::AuthFailed.needs_attention());
        assert!(Health::Unsupported.needs_attention());
        assert!(Health::Removed.needs_attention());
        assert!(!Health::Unknown.needs_attention());
    }

    #[test]
    fn command_result_helpers_classify_acceptance_and_failures() {
        let accepted = CommandResult {
            command_id: CommandId::trusted("cmd-accepted"),
            status: CommandStatus::Accepted,
            bridge_id: BridgeId::trusted("bridge-hue"),
            correlation_id: CorrelationId::trusted("corr-accepted"),
            message: None,
        };
        let rejected = CommandResult {
            command_id: CommandId::trusted("cmd-rejected"),
            status: CommandStatus::Rejected,
            bridge_id: BridgeId::trusted("bridge-hue"),
            correlation_id: CorrelationId::trusted("corr-rejected"),
            message: Some("missing grant".to_string()),
        };
        let timed_out = CommandResult {
            command_id: CommandId::trusted("cmd-timeout"),
            status: CommandStatus::TimedOut,
            bridge_id: BridgeId::trusted("bridge-hue"),
            correlation_id: CorrelationId::trusted("corr-timeout"),
            message: None,
        };

        assert!(accepted.is_accepted());
        assert!(!accepted.is_failure());
        assert!(rejected.is_rejected());
        assert!(rejected.is_failure());
        assert!(timed_out.is_failure());
        assert!(timed_out.timed_out());
        assert!(CommandStatus::Failed.is_failure());
        assert!(!CommandStatus::Failed.timed_out());
    }

    #[test]
    fn inventory_summary_counts_health_state_and_command_surfaces() {
        let mut hue_bridge = Bridge::new(
            BridgeId::trusted("bridge-hue"),
            IntegrationId::trusted("hue"),
            BridgeTransport::LanHttp,
        );
        hue_bridge.health = Health::Online;
        let mut pairing_bridge = Bridge::new(
            BridgeId::trusted("bridge-zigbee"),
            IntegrationId::trusted("zigbee"),
            BridgeTransport::Serial,
        );
        pairing_bridge.health = Health::Unpaired;
        let mut failed_bridge = Bridge::new(
            BridgeId::trusted("bridge-zwave"),
            IntegrationId::trusted("zwave"),
            BridgeTransport::Serial,
        );
        failed_bridge.health = Health::AuthFailed;
        let bridges = vec![hue_bridge, pairing_bridge, failed_bridge];

        let devices = vec![
            Device {
                device_id: DeviceId::trusted("device-light"),
                bridge_id: BridgeId::trusted("bridge-hue"),
                manufacturer: "Acme".to_string(),
                model: "Light".to_string(),
                name: "Kitchen".to_string(),
                serial: None,
                firmware_version: None,
                room_id: Some("kitchen".to_string()),
                entity_ids: vec![EntityId::trusted("entity-light")],
                identifiers: Vec::new(),
                health: Health::Online,
                metadata: Vec::new(),
            },
            Device {
                device_id: DeviceId::trusted("device-sensor"),
                bridge_id: BridgeId::trusted("bridge-zigbee"),
                manufacturer: "Acme".to_string(),
                model: "Sensor".to_string(),
                name: "Hall".to_string(),
                serial: None,
                firmware_version: None,
                room_id: None,
                entity_ids: vec![EntityId::trusted("entity-sensor")],
                identifiers: Vec::new(),
                health: Health::Discoverable,
                metadata: Vec::new(),
            },
            Device {
                device_id: DeviceId::trusted("device-lock"),
                bridge_id: BridgeId::trusted("bridge-zwave"),
                manufacturer: "Acme".to_string(),
                model: "Lock".to_string(),
                name: "Front door".to_string(),
                serial: None,
                firmware_version: None,
                room_id: None,
                entity_ids: vec![EntityId::trusted("entity-lock")],
                identifiers: Vec::new(),
                health: Health::Offline,
                metadata: Vec::new(),
            },
        ];

        let entities = vec![
            Entity {
                entity_id: EntityId::trusted("entity-light"),
                device_id: DeviceId::trusted("device-light"),
                kind: EntityKind::Light,
                name: "Kitchen light".to_string(),
                capabilities: vec![Capability::light_on_off()],
                state: Some(StateSnapshot {
                    entity_id: EntityId::trusted("entity-light"),
                    value: Value::Bool(true),
                    source: StateSource::EventStream,
                    observed_at_ms: 1_000,
                    received_at_ms: 1_000,
                    expires_at_ms: Some(3_000),
                    confidence: StateConfidence::Confirmed,
                }),
                metadata: Vec::new(),
            },
            Entity {
                entity_id: EntityId::trusted("entity-sensor"),
                device_id: DeviceId::trusted("device-sensor"),
                kind: EntityKind::Sensor,
                name: "Hall sensor".to_string(),
                capabilities: vec![Capability::sensor_temperature()],
                state: Some(StateSnapshot {
                    entity_id: EntityId::trusted("entity-sensor"),
                    value: Value::Number(21.5),
                    source: StateSource::Poll,
                    observed_at_ms: 500,
                    received_at_ms: 600,
                    expires_at_ms: Some(1_500),
                    confidence: StateConfidence::Confirmed,
                }),
                metadata: Vec::new(),
            },
            Entity {
                entity_id: EntityId::trusted("entity-scene"),
                device_id: DeviceId::trusted("device-light"),
                kind: EntityKind::Scene,
                name: "Dinner".to_string(),
                capabilities: vec![Capability::scene_recall()],
                state: None,
                metadata: Vec::new(),
            },
        ];

        let summary =
            SmartHomeInventorySummary::from_inventory(&bridges, &devices, &entities, 2_000);

        assert_eq!(
            summary,
            SmartHomeInventorySummary {
                total_bridges: 3,
                online_bridges: 1,
                pairing_candidate_bridges: 1,
                bridges_needing_attention: 1,
                total_devices: 3,
                online_devices: 1,
                pairing_candidate_devices: 1,
                devices_needing_attention: 1,
                total_entities: 3,
                entities_with_state: 2,
                stale_entities: 1,
                commandable_entities: 2,
            }
        );
        assert!(!summary.is_empty());
        assert!(summary.has_pairing_candidates());
        assert!(summary.needs_attention());
        assert!(summary.has_stale_state());

        let empty = SmartHomeInventorySummary::from_inventory(
            Vec::<Bridge>::new().iter(),
            Vec::<Device>::new().iter(),
            Vec::<Entity>::new().iter(),
            2_000,
        );
        assert!(empty.is_empty());
        assert!(!empty.has_pairing_candidates());
        assert!(!empty.needs_attention());
        assert!(!empty.has_stale_state());
    }

    #[test]
    fn canonical_capability_catalog_covers_first_integration_families() {
        let catalog = canonical_capability_catalog();
        let ids = catalog
            .iter()
            .map(|capability| capability.capability_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(catalog.len(), 14);
        assert_eq!(ids[0], "light.on_off");
        assert!(ids.contains(&"light.color"));
        assert!(ids.contains(&"scene.recall"));
        assert!(ids.contains(&"lock.state"));
        assert!(ids.contains(&"climate.setpoint"));
        assert!(ids.contains(&"sensor.contact"));
        assert!(ids.contains(&"sensor.temperature"));
        assert!(ids.contains(&"sensor.humidity"));
        assert!(ids.contains(&"sensor.illuminance"));
        assert!(ids.contains(&"sensor.battery"));

        let scene_recall = catalog
            .iter()
            .find(|capability| capability.capability_id.as_str() == "scene.recall")
            .unwrap();
        assert_eq!(scene_recall.mode, CapabilityMode::Command);
        assert_eq!(scene_recall.value_kind, ValueKind::Null);

        let illuminance = catalog
            .iter()
            .find(|capability| capability.capability_id.as_str() == "sensor.illuminance")
            .unwrap();
        assert_eq!(illuminance.unit.as_deref(), Some("lux"));
    }

    #[test]
    fn command_capabilities_are_present_in_canonical_capability_catalog() {
        let catalog_ids = canonical_capability_catalog()
            .into_iter()
            .map(|capability| capability.capability_id)
            .collect::<Vec<_>>();
        let command_types = [
            CommandType::TurnOn,
            CommandType::TurnOff,
            CommandType::SetBrightness,
            CommandType::SetColor,
            CommandType::SetColorTemperature,
            CommandType::RecallScene,
            CommandType::SetLock,
            CommandType::SetThermostatSetpoint,
        ];

        for command_type in command_types {
            let capability_id = command_type.canonical_capability_id().unwrap();
            assert!(
                catalog_ids.contains(&capability_id),
                "missing catalog capability for {command_type:?}"
            );
        }
    }

    #[test]
    fn canonical_integration_catalog_covers_initial_protocol_families() {
        let catalog = canonical_integration_catalog();
        let ids = catalog
            .iter()
            .map(|descriptor| descriptor.integration_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ids,
            vec!["hue", "zigbee", "zwave", "thread", "matter", "mqtt"]
        );
        assert!(catalog
            .iter()
            .all(|descriptor| !descriptor.capabilities.is_empty()));
        assert!(catalog
            .iter()
            .all(|descriptor| !descriptor.discovery_roles.is_empty()));
        assert!(catalog
            .iter()
            .all(|descriptor| !descriptor.pairing_roles.is_empty()));

        let hue = canonical_integration_descriptor(&IntegrationId::trusted("hue")).unwrap();
        assert_eq!(hue.display_name, "Philips Hue Bridge");
        assert!(hue.supports_discovery_role("mdns"));
        assert!(hue.supports_pairing_role("link-button"));
        assert!(hue.supports_capability(&CapabilityId::trusted("light.color_temperature")));

        let zwave = canonical_integration_descriptor(&IntegrationId::trusted("zwave")).unwrap();
        assert!(zwave.supports_capability(&CapabilityId::trusted("lock.state")));
        assert!(zwave.supports_pairing_role("smart-start"));
        assert!(canonical_integration_descriptor(&IntegrationId::trusted("missing")).is_none());
    }

    #[test]
    fn canonical_integrations_can_be_filtered_by_capability() {
        let lock_integrations =
            canonical_integrations_for_capability(&CapabilityId::trusted("lock.state"))
                .into_iter()
                .map(|descriptor| descriptor.integration_id.as_str().to_string())
                .collect::<Vec<_>>();
        let scene_integrations =
            canonical_integrations_for_capability(&CapabilityId::trusted("scene.recall"))
                .into_iter()
                .map(|descriptor| descriptor.integration_id.as_str().to_string())
                .collect::<Vec<_>>();

        assert_eq!(lock_integrations, vec!["zigbee", "zwave", "matter", "mqtt"]);
        assert_eq!(scene_integrations, vec!["hue", "mqtt"]);
    }

    #[test]
    fn canonical_integration_catalog_summary_counts_runtime_and_capability_coverage() {
        let summary = canonical_integration_catalog_summary();
        let hue = canonical_integration_descriptor(&IntegrationId::trusted("hue")).unwrap();

        assert_eq!(summary.total_integrations, 6);
        assert_eq!(summary.in_process_rust_integrations, 0);
        assert_eq!(summary.rust_worker_process_integrations, 6);
        assert_eq!(summary.total_capability_mappings, 59);
        assert_eq!(summary.unique_capabilities, 14);
        assert_eq!(summary.total_discovery_roles, 15);
        assert_eq!(summary.total_pairing_roles, 13);
        assert_eq!(summary.discoverable_integrations, 6);
        assert_eq!(summary.pairable_integrations, 6);
        assert!(summary.all_integrations_discoverable());
        assert!(summary.all_integrations_pairable());
        assert!(hue.is_discoverable());
        assert!(hue.is_pairable());
        assert_eq!(hue.capability_count(), 6);
    }

    #[test]
    fn tool_catalog_exposes_model_facing_smart_home_surface() {
        let catalog = smart_home_tool_catalog();
        let command = catalog
            .iter()
            .find(|tool| tool.tool_id == "smart_home.command")
            .unwrap();

        assert_eq!(catalog.len(), 10);
        assert_eq!(command.side_effects, ToolSideEffects::External);
        assert_eq!(
            command.required_capabilities,
            vec![CapabilityId::trusted("smart_home.command.light")]
        );
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.observe_supervision"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
    }

    #[test]
    fn tool_catalog_summary_counts_risk_tiers_and_capabilities() {
        let summary = smart_home_tool_catalog_summary();
        let pair_bridge = SmartHomeTool::PairBridge.descriptor();

        assert_eq!(summary.total_tools, 10);
        assert_eq!(summary.read_tools, 8);
        assert_eq!(summary.write_tools, 0);
        assert_eq!(summary.external_tools, 2);
        assert_eq!(summary.read_only_tier_tools, 8);
        assert_eq!(summary.low_risk_tier_tools, 1);
        assert_eq!(summary.high_risk_tier_tools, 0);
        assert_eq!(summary.human_approval_tier_tools, 1);
        assert_eq!(summary.total_required_capabilities, 10);
        assert_eq!(summary.risky_tool_count(), 2);
        assert_eq!(summary.approval_gated_tool_count(), 1);
        assert!(pair_bridge.requires_human_approval());
        assert!(!SmartHomeTool::Command
            .descriptor()
            .requires_human_approval());
    }

    #[test]
    fn capability_grants_gate_tool_descriptors_by_agent_tier_and_time() {
        let principal = AgentId::trusted("agent:lighting-planner");
        let other_principal = AgentId::trusted("agent:other");
        let get_state = SmartHomeTool::GetState.descriptor();
        let command = SmartHomeTool::Command.descriptor();
        let read_grant = CapabilityGrant::for_capability(
            CapabilityGrantId::trusted("grant-read"),
            principal.clone(),
            CapabilityId::trusted("smart_home.read"),
            PrivilegeTier::ReadOnly,
            "chief-of-staff",
            1_000,
        );
        let command_grant = CapabilityGrant::for_tool(
            CapabilityGrantId::trusted("grant-command"),
            principal.clone(),
            SmartHomeTool::Command,
            "chief-of-staff",
            1_000,
        )
        .with_expiry(2_000);
        let grants = vec![read_grant, command_grant];

        assert!(get_state.is_satisfied_by(&principal, &grants, 1_500));
        assert!(command.is_satisfied_by(&principal, &grants, 1_999));
        assert!(!command.is_satisfied_by(&principal, &grants, 2_000));
        assert!(!command.is_satisfied_by(&other_principal, &grants, 1_500));
        assert_eq!(grants[1].status_at(2_000), CapabilityGrantStatus::Expired);
    }

    #[test]
    fn authorization_decisions_record_allowed_tool_grants() {
        let principal = AgentId::trusted("agent:lighting-planner");
        let grant = CapabilityGrant::for_tool(
            CapabilityGrantId::trusted("grant-command"),
            principal.clone(),
            SmartHomeTool::Command,
            "chief-of-staff",
            1_000,
        );

        let decision =
            AuthorizationDecision::for_tool(principal, SmartHomeTool::Command, [&grant], 1_500);

        assert!(decision.is_allowed());
        assert_eq!(decision.outcome, AuthorizationOutcome::Allowed);
        assert_eq!(
            decision.subject,
            AuthorizationSubject::Tool(SmartHomeTool::Command)
        );
        assert_eq!(decision.required_tier, PrivilegeTier::LowRisk);
        assert_eq!(
            decision.required_capabilities,
            vec![CapabilityId::trusted("smart_home.command.light")]
        );
        assert_eq!(
            decision.matched_grants,
            vec![CapabilityGrantId::trusted("grant-command")]
        );
        assert!(decision.missing_capabilities.is_empty());
    }

    #[test]
    fn authorization_decisions_record_command_denials() {
        let principal = AgentId::trusted("agent:security-agent");
        let low_risk_lock_grant = CapabilityGrant::for_entity_capability(
            CapabilityGrantId::trusted("grant-lock-low"),
            principal.clone(),
            EntityId::trusted("entity.lock.front-door"),
            CapabilityId::trusted("lock.state"),
            PrivilegeTier::LowRisk,
            "chief-of-staff",
            1_000,
        );
        let command = DeviceCommand::new(
            CommandId::trusted("cmd-lock"),
            EntityId::trusted("entity.lock.front-door"),
            CommandType::SetLock,
            Value::Text("locked".to_string()),
            "agent:security-agent",
            CorrelationId::trusted("corr-lock"),
        )
        .unwrap();

        let decision =
            AuthorizationDecision::for_command(principal, &command, [&low_risk_lock_grant], 1_500);

        assert!(!decision.is_allowed());
        assert_eq!(decision.outcome, AuthorizationOutcome::Denied);
        assert_eq!(
            decision.subject,
            AuthorizationSubject::Command {
                command_id: CommandId::trusted("cmd-lock"),
                entity_id: EntityId::trusted("entity.lock.front-door"),
                command_type: CommandType::SetLock,
            }
        );
        assert_eq!(decision.required_tier, PrivilegeTier::HighRisk);
        assert!(decision.matched_grants.is_empty());
        assert_eq!(
            decision.missing_capabilities,
            vec![CapabilityId::trusted("lock.state")]
        );
    }
}
