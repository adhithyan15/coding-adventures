//! Repository-owned smart-home vocabulary shared by integrations, tools, and
//! Chief of Staff agents.
//!
//! The types in this crate are intentionally protocol-neutral. A Hue light,
//! Zigbee endpoint, Z-Wave node value, Thread/Matter endpoint, or MQTT device
//! can all be projected into the same bridge/device/entity/event/command model.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
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
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuntimeKind {
    InProcessRust,
    RustWorkerProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BridgeTransport {
    LanHttp,
    Mdns,
    Serial,
    Ble,
    Cloud,
    LocalProcess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind {
    Camera,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityMode {
    Observe,
    Command,
    ObserveAndCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolFamily {
    Hue,
    Onvif,
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
            Self::Onvif => "onvif",
            Self::Zigbee => "zigbee",
            Self::ZWave => "zwave",
            Self::Thread => "thread",
            Self::Matter => "matter",
            Self::Mqtt => "mqtt",
            Self::Vendor(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn surface_summary(&self) -> IntegrationSurfaceSummary {
        IntegrationSurfaceSummary {
            integration_id: self.integration_id.clone(),
            display_name: self.display_name.clone(),
            version: self.version.clone(),
            runtime_kind: self.runtime_kind,
            capability_count: self.capabilities.len(),
            discovery_role_count: self.discovery_roles.len(),
            pairing_role_count: self.pairing_roles.len(),
            exposes_capabilities: !self.capabilities.is_empty(),
            supports_discovery: self.is_discoverable(),
            supports_pairing: self.is_pairable(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntegrationSurfaceSummary {
    pub integration_id: IntegrationId,
    pub display_name: String,
    pub version: String,
    pub runtime_kind: RuntimeKind,
    pub capability_count: usize,
    pub discovery_role_count: usize,
    pub pairing_role_count: usize,
    pub exposes_capabilities: bool,
    pub supports_discovery: bool,
    pub supports_pairing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateSource {
    EventStream,
    Poll,
    OptimisticCommand,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateConfidence {
    Confirmed,
    Optimistic,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceEventType {
    Discovered,
    Updated,
    Removed,
    Unavailable,
    Error,
    Health,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateDelta {
    pub capability_id: CapabilityId,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PrivilegeTier {
    ReadOnly,
    LowRisk,
    HumanApproval,
    HighRisk,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandStatus {
    Accepted,
    Rejected,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneScope {
    Room,
    Zone,
    Home,
    Bridge,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneAction {
    pub entity_id: EntityId,
    pub desired_state: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub scene_id: SceneId,
    pub scope: SceneScope,
    pub native_ref: Option<ProtocolIdentifier>,
    pub actions: Vec<SceneAction>,
    pub metadata: Vec<Metadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SmartHomeTool {
    ListIntegrations,
    DescribeIntegration,
    ListPrimitives,
    DescribePrimitive,
    GetIntegrationCatalogSummary,
    GetToolCatalogSummary,
    ListIntegrationPolicySurfaces,
    GetIntegrationPolicySurfaceSummary,
    ListIntegrationPlatformCoverage,
    GetIntegrationPlatformCoverageSummary,
    ListIntegrationPrimitiveCoverage,
    GetIntegrationPrimitiveCoverageSummary,
    ListIntegrationActivationPlans,
    GetIntegrationActivationPlanSummary,
    ListIntegrationActivationCandidates,
    GetIntegrationActivationCandidateSummary,
    ListIntegrationActivationActions,
    GetIntegrationActivationActionSummary,
    ListIntegrationActivationAgenda,
    GetIntegrationActivationAgendaSummary,
    ListIntegrationActivationRunway,
    GetIntegrationActivationRunwaySummary,
    ListIntegrationActivationHealth,
    GetIntegrationActivationHealthSummary,
    ListIntegrationActivationMaintenance,
    GetIntegrationActivationMaintenanceSummary,
    ListIntegrationActivationConstraints,
    GetIntegrationActivationConstraintSummary,
    ListIntegrationActivationReviews,
    GetIntegrationActivationReviewSummary,
    ListIntegrationActivationApprovals,
    GetIntegrationActivationApprovalSummary,
    ListIntegrationActivationDecisions,
    GetIntegrationActivationDecisionSummary,
    ListIntegrationActivationEvidence,
    GetIntegrationActivationEvidenceSummary,
    ListIntegrationActivationEvidenceRemediation,
    GetIntegrationActivationEvidenceRemediationSummary,
    ListIntegrationActivationEvidenceLaneInventory,
    GetIntegrationActivationEvidenceLaneInventorySummary,
    GetIntegrationActivationEvidenceScorecardSummary,
    ListIntegrationActivationDossiers,
    GetIntegrationActivationDossierSummary,
    ListIntegrationActivationReadouts,
    GetIntegrationActivationReadoutSummary,
    ListIntegrationActivationBriefingItems,
    GetIntegrationActivationBriefingSummary,
    ListIntegrationActivationDashboard,
    GetIntegrationActivationDashboardSummary,
    ListIntegrationActivationTimeline,
    GetIntegrationActivationTimelineSummary,
    ListIntegrationActivationForecast,
    GetIntegrationActivationForecastSummary,
    ListIntegrationActivationPlaybook,
    GetIntegrationActivationPlaybookSummary,
    ListIntegrationActivationRunbook,
    GetIntegrationActivationRunbookSummary,
    ListIntegrationActivationHandoff,
    GetIntegrationActivationHandoffSummary,
    ListIntegrationActivationExecution,
    GetIntegrationActivationExecutionSummary,
    ListIntegrationActivationVerification,
    GetIntegrationActivationVerificationSummary,
    ListIntegrationActivationOperatorQueue,
    GetIntegrationActivationOperatorQueueSummary,
    ListIntegrationActivationControlRoom,
    GetIntegrationActivationControlRoomSummary,
    ListIntegrationActivationCommandCenter,
    GetIntegrationActivationCommandCenterSummary,
    ListIntegrationActivationWatchtower,
    GetIntegrationActivationWatchtowerSummary,
    ListIntegrationActivationSentinel,
    GetIntegrationActivationSentinelSummary,
    ListIntegrationActivationAudit,
    GetIntegrationActivationAuditSummary,
    ListIntegrationActivationEscalations,
    GetIntegrationActivationEscalationSummary,
    ListIntegrationActivationResponses,
    GetIntegrationActivationResponseSummary,
    ListIntegrationActivationRemediation,
    GetIntegrationActivationRemediationSummary,
    ListIntegrationActivationClosure,
    GetIntegrationActivationClosureSummary,
    ListIntegrationActivationRelease,
    GetIntegrationActivationReleaseSummary,
    ListIntegrationActivationDelivery,
    GetIntegrationActivationDeliverySummary,
    ListIntegrationActivationDeployment,
    GetIntegrationActivationDeploymentSummary,
    ListIntegrationActivationSafetyGates,
    GetIntegrationActivationSafetySummary,
    ListIntegrationActivationRollback,
    GetIntegrationActivationRollbackSummary,
    ListIntegrationActivationObservability,
    GetIntegrationActivationObservabilitySummary,
    ListIntegrationActivationIncidents,
    GetIntegrationActivationIncidentSummary,
    ListIntegrationActivationGuardrails,
    GetIntegrationActivationGuardrailSummary,
    ListIntegrationActivationAssurance,
    GetIntegrationActivationAssuranceSummary,
    ListIntegrationActivationGovernance,
    GetIntegrationActivationGovernanceSummary,
    ListIntegrationActivationCompliance,
    GetIntegrationActivationComplianceSummary,
    ListIntegrationActivationAttestations,
    GetIntegrationActivationAttestationSummary,
    ListIntegrationActivationEvidenceLedger,
    GetIntegrationActivationEvidenceLedgerSummary,
    ListIntegrationActivationExceptionLedger,
    GetIntegrationActivationExceptionLedgerSummary,
    ListIntegrationActivationWaiverRegister,
    GetIntegrationActivationWaiverRegisterSummary,
    ListIntegrationActivationWaiverReviews,
    GetIntegrationActivationWaiverReviewSummary,
    ListIntegrationActivationWaiverDispositions,
    GetIntegrationActivationWaiverDispositionSummary,
    ListIntegrationActivationWaiverRemediations,
    GetIntegrationActivationWaiverRemediationSummary,
    ListIntegrationActivationWaiverClosures,
    GetIntegrationActivationWaiverClosureSummary,
    ListIntegrationActivationWaiverArchives,
    GetIntegrationActivationWaiverArchiveSummary,
    ListIntegrationActivationWaiverRetention,
    GetIntegrationActivationWaiverRetentionSummary,
    ListIntegrationActivationWaiverExpirations,
    GetIntegrationActivationWaiverExpirationSummary,
    ListIntegrationActivationWaiverDisposals,
    GetIntegrationActivationWaiverDisposalSummary,
    ListIntegrationActivationWaiverTombstones,
    GetIntegrationActivationWaiverTombstoneSummary,
    ListIntegrationActivationWaiverPurges,
    GetIntegrationActivationWaiverPurgeSummary,
    ListIntegrationActivationWaiverErasures,
    GetIntegrationActivationWaiverErasureSummary,
    ListIntegrationActivationWaiverErasureReceipts,
    GetIntegrationActivationWaiverErasureReceiptSummary,
    ListIntegrationActivationWaiverReleaseClosures,
    GetIntegrationActivationWaiverReleaseClosureSummary,
    ListIntegrationActivationWaiverReleaseSignoffs,
    GetIntegrationActivationWaiverReleaseSignoffSummary,
    ListIntegrationActivationWaiverReleaseCertifications,
    GetIntegrationActivationWaiverReleaseCertificationSummary,
    ListIntegrationActivationWaiverReleaseCertificationRemediations,
    GetIntegrationActivationWaiverReleaseCertificationRemediationSummary,
    ListIntegrationActivationRisk,
    GetIntegrationActivationRiskSummary,
    ListIntegrationActivationDependencies,
    GetIntegrationActivationDependencySummary,
    ListIntegrationReadiness,
    GetIntegrationReadinessSummary,
    ListIntegrationReadinessGaps,
    GetIntegrationReadinessGapSummary,
    ListIntegrationMeshPrimitiveReadiness,
    GetIntegrationMeshPrimitiveReadinessSummary,
    ListIntegrationMeshSubstrateStages,
    GetIntegrationMeshSubstrateStageSummary,
    ListIntegrationMeshSubstrateActions,
    GetIntegrationMeshSubstrateActionSummary,
    ListIntegrationMeshSubstratePreflightChecks,
    GetIntegrationMeshSubstratePreflightSummary,
    ListIntegrationMeshPreflightRepairActions,
    GetIntegrationMeshPreflightRepairActionSummary,
    ListIntegrationMeshPreflightRepairBatches,
    GetIntegrationMeshPreflightRepairBatchSummary,
    ListIntegrationMeshPreflightRepairSchedule,
    GetIntegrationMeshPreflightRepairScheduleSummary,
    ListIntegrationMeshPreflightRepairSlotAudits,
    GetIntegrationMeshPreflightRepairSlotAuditSummary,
    ListIntegrationMeshPreflightRepairSlotExecutionTickets,
    GetIntegrationMeshPreflightRepairSlotExecutionTicketSummary,
    ListIntegrationMeshPreflightRepairSlotExecutionWorkOrders,
    GetIntegrationMeshPreflightRepairSlotExecutionWorkOrderSummary,
    ListIntegrationMeshPreflightRepairSlotExecutionWorkOrderGuardrails,
    GetIntegrationMeshPreflightRepairSlotExecutionWorkOrderGuardrailSummary,
    ListIntegrationMeshPreflightRepairSlotExecutionEvidence,
    GetIntegrationMeshPreflightRepairSlotExecutionEvidenceSummary,
    ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviews,
    GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewSummary,
    ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositions,
    GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionSummary,
    ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionActions,
    GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionActionSummary,
    GetIntegrationMeshPreflightReadinessSummary,
    GetIntegrationMeshPreflightRepairReadinessSummary,
    GetIntegrationMeshPreflightBatchReadinessSummary,
    GetIntegrationMeshPreflightScheduleReadinessSummary,
    GetIntegrationMeshPreflightSlotReadinessSummary,
    GetIntegrationMeshPreflightExecutionReadinessSummary,
    GetIntegrationMeshPreflightWorkOrderReadinessSummary,
    GetIntegrationMeshPreflightGuardrailReadinessSummary,
    GetIntegrationMeshReadinessPackageSummary,
    GetIntegrationMeshStageReleaseSummary,
    GetIntegrationMeshActionReadinessSummary,
    GetIntegrationMeshReleaseReadinessSummary,
    ListIntegrationMeshReadinessHandoffs,
    GetIntegrationMeshReadinessHandoffSummary,
    ListIntegrationMeshReleaseReadinessChecks,
    GetIntegrationMeshReleaseReadinessCheckSummary,
    Discover,
    PairBridge,
    CompletePairing,
    ListDiscoveryWorkers,
    GetDiscoverySummary,
    GetPairingPlan,
    ListBridges,
    ListDevices,
    ListDeviceInventoryAudit,
    GetDeviceInventoryAuditSummary,
    ListRoomTopologyAudit,
    GetRoomTopologyAuditSummary,
    ListRooms,
    ListSceneCoverageAudit,
    GetSceneCoverageAuditSummary,
    ListScenes,
    DescribeScene,
    GetState,
    Command,
    ReportEvent,
    Subscribe,
    PollEvents,
    Unsubscribe,
    ListSubscriptions,
    ListEventDeliveryAudit,
    GetEventDeliveryAuditSummary,
    InspectEventLog,
    ListCommandResults,
    GetCommandResultSummary,
    ListCommandRiskAudit,
    GetCommandRiskAuditSummary,
    ListAuthorizationGapAudit,
    GetAuthorizationGapAuditSummary,
    ListAuthorizationDecisions,
    GetAuthorizationSummary,
    ListCapabilityGrants,
    GetCapabilityGrantSummary,
    GetControllerHandoffSummary,
    GetPlatformBrief,
    ListPlatformEvidenceLedger,
    GetPlatformEvidenceLedgerSummary,
    ListPlatformAccessReview,
    GetPlatformAccessReviewSummary,
    ListPlatformEventOpsReview,
    GetPlatformEventOpsReviewSummary,
    GetRuntimeSnapshot,
    GetPendingWorkSummary,
    GetAttentionOverview,
    GetSystemHealthBrief,
    GetOperatorActionBrief,
    GetServiceExecutionReadinessBrief,
    GetServiceExecutionSafetyBrief,
    GetRemediationPlan,
    GetOperationsBrief,
    GetSafetyBrief,
    GetReadinessBrief,
    GetMaintenanceBrief,
    GetIncidentBrief,
    GetRecoveryBrief,
    GetRecoveryReadinessBrief,
    GetCommandLifecycleBrief,
    GetCommandAuditDossier,
    GetCommandResolutionBrief,
    GetMorningBrief,
    GetEscalationBrief,
    GetContinuityBrief,
    GetOperatorReadinessBrief,
    GetShiftHandoffBrief,
    GetCloseoutBrief,
    GetCloseoutReceipt,
    GetCloseoutAuditTrail,
    GetCloseoutArchive,
    GetCloseoutArchiveManifest,
    GetCloseoutRetentionLedger,
    GetTopologySummary,
    ListDesiredStates,
    ListDesiredStateDriftAudit,
    GetDesiredStateDriftAuditSummary,
    ListStateTransitionAudit,
    GetStateTransitionAuditSummary,
    ListSupervisionRemediation,
    GetSupervisionRemediationSummary,
    ListRuntimeMaintenanceWindows,
    GetRuntimeMaintenanceWindowSummary,
    ListRuntimeMaintenanceActions,
    GetRuntimeMaintenanceActionSummary,
    ListRuntimeMaintenancePlans,
    GetRuntimeMaintenancePlanSummary,
    ListRuntimeMaintenanceTickets,
    GetRuntimeMaintenanceTicketSummary,
    ListRuntimeMaintenanceWorkOrders,
    GetRuntimeMaintenanceWorkOrderSummary,
    ListRuntimeMaintenanceWorkOrderGuardrails,
    GetRuntimeMaintenanceWorkOrderGuardrailSummary,
    ListRuntimeMaintenanceWorkOrderEvidence,
    GetRuntimeMaintenanceWorkOrderEvidenceSummary,
    ListRuntimeMaintenanceWorkOrderEvidenceReviews,
    GetRuntimeMaintenanceWorkOrderEvidenceReviewSummary,
    ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositions,
    GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionSummary,
    ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActions,
    GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionSummary,
    ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomes,
    GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeSummary,
    ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadiness,
    GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessSummary,
    ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffs,
    GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffSummary,
    ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffReconciliations,
    GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffReconciliationSummary,
    ListRuntimeMaintenanceCloseoutPackets,
    GetRuntimeMaintenanceCloseoutSummary,
    SetDesiredState,
    ClearDesiredState,
    ListPairingSessions,
    ListWorkers,
    GetWorkerHeartbeatSchedule,
    GetSupervisionPlan,
    ReconcileDesiredStates,
    RunSupervisionTick,
    DescribeCapabilities,
    GetHealth,
    ObserveSupervision,
}

impl SmartHomeTool {
    pub fn descriptor(self) -> ToolDescriptor {
        match self {
            Self::ListIntegrations => read_tool("smart_home.list_integrations"),
            Self::DescribeIntegration => read_tool("smart_home.describe_integration"),
            Self::ListPrimitives => read_tool("smart_home.list_primitives"),
            Self::DescribePrimitive => read_tool("smart_home.describe_primitive"),
            Self::GetIntegrationCatalogSummary => {
                read_tool("smart_home.get_integration_catalog_summary")
            }
            Self::GetToolCatalogSummary => read_tool("smart_home.get_tool_catalog_summary"),
            Self::ListIntegrationPolicySurfaces => {
                read_tool("smart_home.list_integration_policy_surfaces")
            }
            Self::GetIntegrationPolicySurfaceSummary => {
                read_tool("smart_home.get_integration_policy_surface_summary")
            }
            Self::ListIntegrationPlatformCoverage => {
                read_tool("smart_home.list_integration_platform_coverage")
            }
            Self::GetIntegrationPlatformCoverageSummary => {
                read_tool("smart_home.get_integration_platform_coverage_summary")
            }
            Self::ListIntegrationPrimitiveCoverage => {
                read_tool("smart_home.list_integration_primitive_coverage")
            }
            Self::GetIntegrationPrimitiveCoverageSummary => {
                read_tool("smart_home.get_integration_primitive_coverage_summary")
            }
            Self::ListIntegrationActivationPlans => {
                read_tool("smart_home.list_integration_activation_plans")
            }
            Self::GetIntegrationActivationPlanSummary => {
                read_tool("smart_home.get_integration_activation_plan_summary")
            }
            Self::ListIntegrationActivationCandidates => {
                read_tool("smart_home.list_integration_activation_candidates")
            }
            Self::GetIntegrationActivationCandidateSummary => {
                read_tool("smart_home.get_integration_activation_candidate_summary")
            }
            Self::ListIntegrationActivationActions => {
                read_tool("smart_home.list_integration_activation_actions")
            }
            Self::GetIntegrationActivationActionSummary => {
                read_tool("smart_home.get_integration_activation_action_summary")
            }
            Self::ListIntegrationActivationAgenda => {
                read_tool("smart_home.list_integration_activation_agenda")
            }
            Self::GetIntegrationActivationAgendaSummary => {
                read_tool("smart_home.get_integration_activation_agenda_summary")
            }
            Self::ListIntegrationActivationRunway => {
                read_tool("smart_home.list_integration_activation_runway")
            }
            Self::GetIntegrationActivationRunwaySummary => {
                read_tool("smart_home.get_integration_activation_runway_summary")
            }
            Self::ListIntegrationActivationHealth => {
                read_tool("smart_home.list_integration_activation_health")
            }
            Self::GetIntegrationActivationHealthSummary => {
                read_tool("smart_home.get_integration_activation_health_summary")
            }
            Self::ListIntegrationActivationMaintenance => {
                read_tool("smart_home.list_integration_activation_maintenance")
            }
            Self::GetIntegrationActivationMaintenanceSummary => {
                read_tool("smart_home.get_integration_activation_maintenance_summary")
            }
            Self::ListIntegrationActivationConstraints => {
                read_tool("smart_home.list_integration_activation_constraints")
            }
            Self::GetIntegrationActivationConstraintSummary => {
                read_tool("smart_home.get_integration_activation_constraint_summary")
            }
            Self::ListIntegrationActivationReviews => {
                read_tool("smart_home.list_integration_activation_reviews")
            }
            Self::GetIntegrationActivationReviewSummary => {
                read_tool("smart_home.get_integration_activation_review_summary")
            }
            Self::ListIntegrationActivationApprovals => {
                read_tool("smart_home.list_integration_activation_approvals")
            }
            Self::GetIntegrationActivationApprovalSummary => {
                read_tool("smart_home.get_integration_activation_approval_summary")
            }
            Self::ListIntegrationActivationDecisions => {
                read_tool("smart_home.list_integration_activation_decisions")
            }
            Self::GetIntegrationActivationDecisionSummary => {
                read_tool("smart_home.get_integration_activation_decision_summary")
            }
            Self::ListIntegrationActivationEvidence => {
                read_tool("smart_home.list_integration_activation_evidence")
            }
            Self::GetIntegrationActivationEvidenceSummary => {
                read_tool("smart_home.get_integration_activation_evidence_summary")
            }
            Self::ListIntegrationActivationEvidenceRemediation => {
                read_tool("smart_home.list_integration_activation_evidence_remediation")
            }
            Self::GetIntegrationActivationEvidenceRemediationSummary => {
                read_tool("smart_home.get_integration_activation_evidence_remediation_summary")
            }
            Self::ListIntegrationActivationEvidenceLaneInventory => {
                read_tool("smart_home.list_integration_activation_evidence_lane_inventory")
            }
            Self::GetIntegrationActivationEvidenceLaneInventorySummary => {
                read_tool("smart_home.get_integration_activation_evidence_lane_inventory_summary")
            }
            Self::GetIntegrationActivationEvidenceScorecardSummary => {
                read_tool("smart_home.get_integration_activation_evidence_scorecard_summary")
            }
            Self::ListIntegrationActivationDossiers => {
                read_tool("smart_home.list_integration_activation_dossiers")
            }
            Self::GetIntegrationActivationDossierSummary => {
                read_tool("smart_home.get_integration_activation_dossier_summary")
            }
            Self::ListIntegrationActivationReadouts => {
                read_tool("smart_home.list_integration_activation_readouts")
            }
            Self::GetIntegrationActivationReadoutSummary => {
                read_tool("smart_home.get_integration_activation_readout_summary")
            }
            Self::ListIntegrationActivationBriefingItems => {
                read_tool("smart_home.list_integration_activation_briefing_items")
            }
            Self::GetIntegrationActivationBriefingSummary => {
                read_tool("smart_home.get_integration_activation_briefing_summary")
            }
            Self::ListIntegrationActivationDashboard => {
                read_tool("smart_home.list_integration_activation_dashboard")
            }
            Self::GetIntegrationActivationDashboardSummary => {
                read_tool("smart_home.get_integration_activation_dashboard_summary")
            }
            Self::ListIntegrationActivationTimeline => {
                read_tool("smart_home.list_integration_activation_timeline")
            }
            Self::GetIntegrationActivationTimelineSummary => {
                read_tool("smart_home.get_integration_activation_timeline_summary")
            }
            Self::ListIntegrationActivationForecast => {
                read_tool("smart_home.list_integration_activation_forecasts")
            }
            Self::GetIntegrationActivationForecastSummary => {
                read_tool("smart_home.get_integration_activation_forecast_summary")
            }
            Self::ListIntegrationActivationPlaybook => {
                read_tool("smart_home.list_integration_activation_playbook")
            }
            Self::GetIntegrationActivationPlaybookSummary => {
                read_tool("smart_home.get_integration_activation_playbook_summary")
            }
            Self::ListIntegrationActivationRunbook => {
                read_tool("smart_home.list_integration_activation_runbook")
            }
            Self::GetIntegrationActivationRunbookSummary => {
                read_tool("smart_home.get_integration_activation_runbook_summary")
            }
            Self::ListIntegrationActivationHandoff => {
                read_tool("smart_home.list_integration_activation_handoff")
            }
            Self::GetIntegrationActivationHandoffSummary => {
                read_tool("smart_home.get_integration_activation_handoff_summary")
            }
            Self::ListIntegrationActivationExecution => {
                read_tool("smart_home.list_integration_activation_execution")
            }
            Self::GetIntegrationActivationExecutionSummary => {
                read_tool("smart_home.get_integration_activation_execution_summary")
            }
            Self::ListIntegrationActivationVerification => {
                read_tool("smart_home.list_integration_activation_verification")
            }
            Self::GetIntegrationActivationVerificationSummary => {
                read_tool("smart_home.get_integration_activation_verification_summary")
            }
            Self::ListIntegrationActivationOperatorQueue => {
                read_tool("smart_home.list_integration_activation_operator_queue")
            }
            Self::GetIntegrationActivationOperatorQueueSummary => {
                read_tool("smart_home.get_integration_activation_operator_queue_summary")
            }
            Self::ListIntegrationActivationControlRoom => {
                read_tool("smart_home.list_integration_activation_control_room")
            }
            Self::GetIntegrationActivationControlRoomSummary => {
                read_tool("smart_home.get_integration_activation_control_room_summary")
            }
            Self::ListIntegrationActivationCommandCenter => {
                read_tool("smart_home.list_integration_activation_command_center")
            }
            Self::GetIntegrationActivationCommandCenterSummary => {
                read_tool("smart_home.get_integration_activation_command_center_summary")
            }
            Self::ListIntegrationActivationWatchtower => {
                read_tool("smart_home.list_integration_activation_watchtower")
            }
            Self::GetIntegrationActivationWatchtowerSummary => {
                read_tool("smart_home.get_integration_activation_watchtower_summary")
            }
            Self::ListIntegrationActivationSentinel => {
                read_tool("smart_home.list_integration_activation_sentinel")
            }
            Self::GetIntegrationActivationSentinelSummary => {
                read_tool("smart_home.get_integration_activation_sentinel_summary")
            }
            Self::ListIntegrationActivationAudit => {
                read_tool("smart_home.list_integration_activation_audit")
            }
            Self::GetIntegrationActivationAuditSummary => {
                read_tool("smart_home.get_integration_activation_audit_summary")
            }
            Self::ListIntegrationActivationEscalations => {
                read_tool("smart_home.list_integration_activation_escalations")
            }
            Self::GetIntegrationActivationEscalationSummary => {
                read_tool("smart_home.get_integration_activation_escalation_summary")
            }
            Self::ListIntegrationActivationResponses => {
                read_tool("smart_home.list_integration_activation_responses")
            }
            Self::GetIntegrationActivationResponseSummary => {
                read_tool("smart_home.get_integration_activation_response_summary")
            }
            Self::ListIntegrationActivationRemediation => {
                read_tool("smart_home.list_integration_activation_remediation")
            }
            Self::GetIntegrationActivationRemediationSummary => {
                read_tool("smart_home.get_integration_activation_remediation_summary")
            }
            Self::ListIntegrationActivationClosure => {
                read_tool("smart_home.list_integration_activation_closure")
            }
            Self::GetIntegrationActivationClosureSummary => {
                read_tool("smart_home.get_integration_activation_closure_summary")
            }
            Self::ListIntegrationActivationRelease => {
                read_tool("smart_home.list_integration_activation_release")
            }
            Self::GetIntegrationActivationReleaseSummary => {
                read_tool("smart_home.get_integration_activation_release_summary")
            }
            Self::ListIntegrationActivationDelivery => {
                read_tool("smart_home.list_integration_activation_delivery")
            }
            Self::GetIntegrationActivationDeliverySummary => {
                read_tool("smart_home.get_integration_activation_delivery_summary")
            }
            Self::ListIntegrationActivationDeployment => {
                read_tool("smart_home.list_integration_activation_deployment")
            }
            Self::GetIntegrationActivationDeploymentSummary => {
                read_tool("smart_home.get_integration_activation_deployment_summary")
            }
            Self::ListIntegrationActivationSafetyGates => {
                read_tool("smart_home.list_integration_activation_safety_gates")
            }
            Self::GetIntegrationActivationSafetySummary => {
                read_tool("smart_home.get_integration_activation_safety_summary")
            }
            Self::ListIntegrationActivationRollback => {
                read_tool("smart_home.list_integration_activation_rollback")
            }
            Self::GetIntegrationActivationRollbackSummary => {
                read_tool("smart_home.get_integration_activation_rollback_summary")
            }
            Self::ListIntegrationActivationObservability => {
                read_tool("smart_home.list_integration_activation_observability")
            }
            Self::GetIntegrationActivationObservabilitySummary => {
                read_tool("smart_home.get_integration_activation_observability_summary")
            }
            Self::ListIntegrationActivationIncidents => {
                read_tool("smart_home.list_integration_activation_incidents")
            }
            Self::GetIntegrationActivationIncidentSummary => {
                read_tool("smart_home.get_integration_activation_incident_summary")
            }
            Self::ListIntegrationActivationGuardrails => {
                read_tool("smart_home.list_integration_activation_guardrails")
            }
            Self::GetIntegrationActivationGuardrailSummary => {
                read_tool("smart_home.get_integration_activation_guardrail_summary")
            }
            Self::ListIntegrationActivationAssurance => {
                read_tool("smart_home.list_integration_activation_assurance")
            }
            Self::GetIntegrationActivationAssuranceSummary => {
                read_tool("smart_home.get_integration_activation_assurance_summary")
            }
            Self::ListIntegrationActivationGovernance => {
                read_tool("smart_home.list_integration_activation_governance")
            }
            Self::GetIntegrationActivationGovernanceSummary => {
                read_tool("smart_home.get_integration_activation_governance_summary")
            }
            Self::ListIntegrationActivationCompliance => {
                read_tool("smart_home.list_integration_activation_compliance")
            }
            Self::GetIntegrationActivationComplianceSummary => {
                read_tool("smart_home.get_integration_activation_compliance_summary")
            }
            Self::ListIntegrationActivationAttestations => {
                read_tool("smart_home.list_integration_activation_attestations")
            }
            Self::GetIntegrationActivationAttestationSummary => {
                read_tool("smart_home.get_integration_activation_attestation_summary")
            }
            Self::ListIntegrationActivationEvidenceLedger => {
                read_tool("smart_home.list_integration_activation_evidence_ledger")
            }
            Self::GetIntegrationActivationEvidenceLedgerSummary => {
                read_tool("smart_home.get_integration_activation_evidence_ledger_summary")
            }
            Self::ListIntegrationActivationExceptionLedger => {
                read_tool("smart_home.list_integration_activation_exception_ledger")
            }
            Self::GetIntegrationActivationExceptionLedgerSummary => {
                read_tool("smart_home.get_integration_activation_exception_ledger_summary")
            }
            Self::ListIntegrationActivationWaiverRegister => {
                read_tool("smart_home.list_integration_activation_waiver_register")
            }
            Self::GetIntegrationActivationWaiverRegisterSummary => {
                read_tool("smart_home.get_integration_activation_waiver_register_summary")
            }
            Self::ListIntegrationActivationWaiverReviews => {
                read_tool("smart_home.list_integration_activation_waiver_reviews")
            }
            Self::GetIntegrationActivationWaiverReviewSummary => {
                read_tool("smart_home.get_integration_activation_waiver_review_summary")
            }
            Self::ListIntegrationActivationWaiverDispositions => {
                read_tool("smart_home.list_integration_activation_waiver_dispositions")
            }
            Self::GetIntegrationActivationWaiverDispositionSummary => {
                read_tool("smart_home.get_integration_activation_waiver_disposition_summary")
            }
            Self::ListIntegrationActivationWaiverRemediations => {
                read_tool("smart_home.list_integration_activation_waiver_remediations")
            }
            Self::GetIntegrationActivationWaiverRemediationSummary => {
                read_tool("smart_home.get_integration_activation_waiver_remediation_summary")
            }
            Self::ListIntegrationActivationWaiverClosures => {
                read_tool("smart_home.list_integration_activation_waiver_closures")
            }
            Self::GetIntegrationActivationWaiverClosureSummary => {
                read_tool("smart_home.get_integration_activation_waiver_closure_summary")
            }
            Self::ListIntegrationActivationWaiverArchives => {
                read_tool("smart_home.list_integration_activation_waiver_archives")
            }
            Self::GetIntegrationActivationWaiverArchiveSummary => {
                read_tool("smart_home.get_integration_activation_waiver_archive_summary")
            }
            Self::ListIntegrationActivationWaiverRetention => {
                read_tool("smart_home.list_integration_activation_waiver_retention")
            }
            Self::GetIntegrationActivationWaiverRetentionSummary => {
                read_tool("smart_home.get_integration_activation_waiver_retention_summary")
            }
            Self::ListIntegrationActivationWaiverExpirations => {
                read_tool("smart_home.list_integration_activation_waiver_expirations")
            }
            Self::GetIntegrationActivationWaiverExpirationSummary => {
                read_tool("smart_home.get_integration_activation_waiver_expiration_summary")
            }
            Self::ListIntegrationActivationWaiverDisposals => {
                read_tool("smart_home.list_integration_activation_waiver_disposals")
            }
            Self::GetIntegrationActivationWaiverDisposalSummary => {
                read_tool("smart_home.get_integration_activation_waiver_disposal_summary")
            }
            Self::ListIntegrationActivationWaiverTombstones => {
                read_tool("smart_home.list_integration_activation_waiver_tombstones")
            }
            Self::GetIntegrationActivationWaiverTombstoneSummary => {
                read_tool("smart_home.get_integration_activation_waiver_tombstone_summary")
            }
            Self::ListIntegrationActivationWaiverPurges => {
                read_tool("smart_home.list_integration_activation_waiver_purges")
            }
            Self::GetIntegrationActivationWaiverPurgeSummary => {
                read_tool("smart_home.get_integration_activation_waiver_purge_summary")
            }
            Self::ListIntegrationActivationWaiverErasures => {
                read_tool("smart_home.list_integration_activation_waiver_erasures")
            }
            Self::GetIntegrationActivationWaiverErasureSummary => {
                read_tool("smart_home.get_integration_activation_waiver_erasure_summary")
            }
            Self::ListIntegrationActivationWaiverErasureReceipts => {
                read_tool("smart_home.list_integration_activation_waiver_erasure_receipts")
            }
            Self::GetIntegrationActivationWaiverErasureReceiptSummary => {
                read_tool("smart_home.get_integration_activation_waiver_erasure_receipt_summary")
            }
            Self::ListIntegrationActivationWaiverReleaseClosures => {
                read_tool("smart_home.list_integration_activation_waiver_release_closures")
            }
            Self::GetIntegrationActivationWaiverReleaseClosureSummary => {
                read_tool("smart_home.get_integration_activation_waiver_release_closure_summary")
            }
            Self::ListIntegrationActivationWaiverReleaseSignoffs => {
                read_tool("smart_home.list_integration_activation_waiver_release_signoffs")
            }
            Self::GetIntegrationActivationWaiverReleaseSignoffSummary => {
                read_tool("smart_home.get_integration_activation_waiver_release_signoff_summary")
            }
            Self::ListIntegrationActivationWaiverReleaseCertifications => {
                read_tool("smart_home.list_integration_activation_waiver_release_certifications")
            }
            Self::GetIntegrationActivationWaiverReleaseCertificationSummary => read_tool(
                "smart_home.get_integration_activation_waiver_release_certification_summary",
            ),
            Self::ListIntegrationActivationWaiverReleaseCertificationRemediations => read_tool(
                "smart_home.list_integration_activation_waiver_release_certification_remediations",
            ),
            Self::GetIntegrationActivationWaiverReleaseCertificationRemediationSummary => {
                read_tool(
                    "smart_home.get_integration_activation_waiver_release_certification_remediation_summary",
                )
            }
            Self::ListIntegrationActivationRisk => {
                read_tool("smart_home.list_integration_activation_risk")
            }
            Self::GetIntegrationActivationRiskSummary => {
                read_tool("smart_home.get_integration_activation_risk_summary")
            }
            Self::ListIntegrationActivationDependencies => {
                read_tool("smart_home.list_integration_activation_dependencies")
            }
            Self::GetIntegrationActivationDependencySummary => {
                read_tool("smart_home.get_integration_activation_dependency_summary")
            }
            Self::ListIntegrationReadiness => read_tool("smart_home.list_integration_readiness"),
            Self::GetIntegrationReadinessSummary => {
                read_tool("smart_home.get_integration_readiness_summary")
            }
            Self::ListIntegrationReadinessGaps => {
                read_tool("smart_home.list_integration_readiness_gaps")
            }
            Self::GetIntegrationReadinessGapSummary => {
                read_tool("smart_home.get_integration_readiness_gap_summary")
            }
            Self::ListIntegrationMeshPrimitiveReadiness => {
                read_tool("smart_home.list_integration_mesh_primitive_readiness")
            }
            Self::GetIntegrationMeshPrimitiveReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_primitive_readiness_summary")
            }
            Self::ListIntegrationMeshSubstrateStages => {
                read_tool("smart_home.list_integration_mesh_substrate_stages")
            }
            Self::GetIntegrationMeshSubstrateStageSummary => {
                read_tool("smart_home.get_integration_mesh_substrate_stage_summary")
            }
            Self::ListIntegrationMeshSubstrateActions => {
                read_tool("smart_home.list_integration_mesh_substrate_actions")
            }
            Self::GetIntegrationMeshSubstrateActionSummary => {
                read_tool("smart_home.get_integration_mesh_substrate_action_summary")
            }
            Self::ListIntegrationMeshSubstratePreflightChecks => {
                read_tool("smart_home.list_integration_mesh_substrate_preflight_checks")
            }
            Self::GetIntegrationMeshSubstratePreflightSummary => {
                read_tool("smart_home.get_integration_mesh_substrate_preflight_summary")
            }
            Self::ListIntegrationMeshPreflightRepairActions => {
                read_tool("smart_home.list_integration_mesh_preflight_repair_actions")
            }
            Self::GetIntegrationMeshPreflightRepairActionSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_repair_action_summary")
            }
            Self::ListIntegrationMeshPreflightRepairBatches => {
                read_tool("smart_home.list_integration_mesh_preflight_repair_batches")
            }
            Self::GetIntegrationMeshPreflightRepairBatchSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_repair_batch_summary")
            }
            Self::ListIntegrationMeshPreflightRepairSchedule => {
                read_tool("smart_home.list_integration_mesh_preflight_repair_schedule")
            }
            Self::GetIntegrationMeshPreflightRepairScheduleSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_repair_schedule_summary")
            }
            Self::ListIntegrationMeshPreflightRepairSlotAudits => {
                read_tool("smart_home.list_integration_mesh_preflight_repair_slot_audits")
            }
            Self::GetIntegrationMeshPreflightRepairSlotAuditSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_repair_slot_audit_summary")
            }
            Self::ListIntegrationMeshPreflightRepairSlotExecutionTickets => read_tool(
                "smart_home.list_integration_mesh_preflight_repair_slot_execution_tickets",
            ),
            Self::GetIntegrationMeshPreflightRepairSlotExecutionTicketSummary => read_tool(
                "smart_home.get_integration_mesh_preflight_repair_slot_execution_ticket_summary",
            ),
            Self::ListIntegrationMeshPreflightRepairSlotExecutionWorkOrders => read_tool(
                "smart_home.list_integration_mesh_preflight_repair_slot_execution_work_orders",
            ),
            Self::GetIntegrationMeshPreflightRepairSlotExecutionWorkOrderSummary => read_tool(
                "smart_home.get_integration_mesh_preflight_repair_slot_execution_work_order_summary",
            ),
            Self::ListIntegrationMeshPreflightRepairSlotExecutionWorkOrderGuardrails => read_tool(
                "smart_home.list_integration_mesh_preflight_repair_slot_execution_work_order_guardrails",
            ),
            Self::GetIntegrationMeshPreflightRepairSlotExecutionWorkOrderGuardrailSummary => read_tool(
                "smart_home.get_integration_mesh_preflight_repair_slot_execution_work_order_guardrail_summary",
            ),
            Self::ListIntegrationMeshPreflightRepairSlotExecutionEvidence => read_tool(
                "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence",
            ),
            Self::GetIntegrationMeshPreflightRepairSlotExecutionEvidenceSummary => read_tool(
                "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_summary",
            ),
            Self::ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviews => read_tool(
                "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_reviews",
            ),
            Self::GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewSummary => read_tool(
                "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_summary",
            ),
            Self::ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositions => {
                read_tool("smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_review_dispositions")
            }
            Self::GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_summary")
            }
            Self::ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionActions => {
                read_tool("smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_actions")
            }
            Self::GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionActionSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_action_summary")
            }
            Self::GetIntegrationMeshPreflightReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_readiness_summary")
            }
            Self::GetIntegrationMeshPreflightRepairReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_repair_readiness_summary")
            }
            Self::GetIntegrationMeshPreflightBatchReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_batch_readiness_summary")
            }
            Self::GetIntegrationMeshPreflightScheduleReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_schedule_readiness_summary")
            }
            Self::GetIntegrationMeshPreflightSlotReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_slot_readiness_summary")
            }
            Self::GetIntegrationMeshPreflightExecutionReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_execution_readiness_summary")
            }
            Self::GetIntegrationMeshPreflightWorkOrderReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_work_order_readiness_summary")
            }
            Self::GetIntegrationMeshPreflightGuardrailReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_preflight_guardrail_readiness_summary")
            }
            Self::GetIntegrationMeshReadinessPackageSummary => {
                read_tool("smart_home.get_integration_mesh_readiness_package_summary")
            }
            Self::GetIntegrationMeshStageReleaseSummary => {
                read_tool("smart_home.get_integration_mesh_stage_release_summary")
            }
            Self::GetIntegrationMeshActionReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_action_readiness_summary")
            }
            Self::GetIntegrationMeshReleaseReadinessSummary => {
                read_tool("smart_home.get_integration_mesh_release_readiness_summary")
            }
            Self::ListIntegrationMeshReadinessHandoffs => {
                read_tool("smart_home.list_integration_mesh_readiness_handoffs")
            }
            Self::GetIntegrationMeshReadinessHandoffSummary => {
                read_tool("smart_home.get_integration_mesh_readiness_handoff_summary")
            }
            Self::ListIntegrationMeshReleaseReadinessChecks => {
                read_tool("smart_home.list_integration_mesh_release_readiness_checks")
            }
            Self::GetIntegrationMeshReleaseReadinessCheckSummary => {
                read_tool("smart_home.get_integration_mesh_release_readiness_check_summary")
            }
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
            Self::CompletePairing => ToolDescriptor {
                tool_id: "smart_home.complete_pairing",
                side_effects: ToolSideEffects::External,
                required_capabilities: vec![CapabilityId::trusted("smart_home.pair")],
                required_tier: PrivilegeTier::HumanApproval,
            },
            Self::ListDiscoveryWorkers => read_tool("smart_home.list_discovery_workers"),
            Self::GetDiscoverySummary => read_tool("smart_home.get_discovery_summary"),
            Self::GetPairingPlan => read_tool("smart_home.get_pairing_plan"),
            Self::ListBridges => read_tool("smart_home.list_bridges"),
            Self::ListDevices => read_tool("smart_home.list_devices"),
            Self::ListDeviceInventoryAudit => read_tool("smart_home.list_device_inventory_audit"),
            Self::GetDeviceInventoryAuditSummary => {
                read_tool("smart_home.get_device_inventory_audit_summary")
            }
            Self::ListRoomTopologyAudit => read_tool("smart_home.list_room_topology_audit"),
            Self::GetRoomTopologyAuditSummary => {
                read_tool("smart_home.get_room_topology_audit_summary")
            }
            Self::ListRooms => read_tool("smart_home.list_rooms"),
            Self::ListSceneCoverageAudit => read_tool("smart_home.list_scene_coverage_audit"),
            Self::GetSceneCoverageAuditSummary => {
                read_tool("smart_home.get_scene_coverage_audit_summary")
            }
            Self::ListScenes => read_tool("smart_home.list_scenes"),
            Self::DescribeScene => read_tool("smart_home.describe_scene"),
            Self::GetState => read_tool("smart_home.get_state"),
            Self::Command => ToolDescriptor {
                tool_id: "smart_home.command",
                side_effects: ToolSideEffects::External,
                required_capabilities: vec![CapabilityId::trusted("smart_home.command.light")],
                required_tier: PrivilegeTier::LowRisk,
            },
            Self::ReportEvent => ToolDescriptor {
                tool_id: "smart_home.report_event",
                side_effects: ToolSideEffects::External,
                required_capabilities: vec![CapabilityId::trusted("smart_home.ingest")],
                required_tier: PrivilegeTier::LowRisk,
            },
            Self::Subscribe => read_tool("smart_home.subscribe"),
            Self::PollEvents => read_tool("smart_home.poll_events"),
            Self::Unsubscribe => read_tool("smart_home.unsubscribe"),
            Self::ListSubscriptions => read_tool("smart_home.list_subscriptions"),
            Self::InspectEventLog => read_tool("smart_home.inspect_event_log"),
            Self::ListCommandResults => read_tool("smart_home.list_command_results"),
            Self::GetCommandResultSummary => read_tool("smart_home.get_command_result_summary"),
            Self::ListCommandRiskAudit => read_tool("smart_home.list_command_risk_audit"),
            Self::GetCommandRiskAuditSummary => {
                read_tool("smart_home.get_command_risk_audit_summary")
            }
            Self::ListAuthorizationGapAudit => {
                read_tool("smart_home.list_authorization_gap_audit")
            }
            Self::GetAuthorizationGapAuditSummary => {
                read_tool("smart_home.get_authorization_gap_audit_summary")
            }
            Self::ListAuthorizationDecisions => {
                read_tool("smart_home.list_authorization_decisions")
            }
            Self::GetAuthorizationSummary => read_tool("smart_home.get_authorization_summary"),
            Self::ListCapabilityGrants => read_tool("smart_home.list_capability_grants"),
            Self::GetCapabilityGrantSummary => read_tool("smart_home.get_capability_grant_summary"),
            Self::GetControllerHandoffSummary => {
                read_tool("smart_home.get_controller_handoff_summary")
            }
            Self::GetPlatformBrief => read_tool("smart_home.get_platform_brief"),
            Self::ListPlatformEvidenceLedger => {
                read_tool("smart_home.list_platform_evidence_ledger")
            }
            Self::GetPlatformEvidenceLedgerSummary => {
                read_tool("smart_home.get_platform_evidence_ledger_summary")
            }
            Self::ListPlatformAccessReview => read_tool("smart_home.list_platform_access_review"),
            Self::GetPlatformAccessReviewSummary => {
                read_tool("smart_home.get_platform_access_review_summary")
            }
            Self::ListPlatformEventOpsReview => {
                read_tool("smart_home.list_platform_event_ops_review")
            }
            Self::GetPlatformEventOpsReviewSummary => {
                read_tool("smart_home.get_platform_event_ops_review_summary")
            }
            Self::GetRuntimeSnapshot => read_tool("smart_home.get_runtime_snapshot"),
            Self::GetPendingWorkSummary => read_tool("smart_home.get_pending_work_summary"),
            Self::GetAttentionOverview => read_tool("smart_home.get_attention_overview"),
            Self::GetSystemHealthBrief => read_tool("smart_home.get_system_health_brief"),
            Self::GetOperatorActionBrief => read_tool("smart_home.get_operator_action_brief"),
            Self::GetServiceExecutionReadinessBrief => {
                read_tool("smart_home.get_service_execution_readiness_brief")
            }
            Self::GetServiceExecutionSafetyBrief => {
                read_tool("smart_home.get_service_execution_safety_brief")
            }
            Self::GetRemediationPlan => read_tool("smart_home.get_remediation_plan"),
            Self::GetOperationsBrief => read_tool("smart_home.get_operations_brief"),
            Self::GetSafetyBrief => read_tool("smart_home.get_safety_brief"),
            Self::GetReadinessBrief => read_tool("smart_home.get_readiness_brief"),
            Self::GetMaintenanceBrief => read_tool("smart_home.get_maintenance_brief"),
            Self::GetIncidentBrief => read_tool("smart_home.get_incident_brief"),
            Self::GetRecoveryBrief => read_tool("smart_home.get_recovery_brief"),
            Self::GetRecoveryReadinessBrief => {
                read_tool("smart_home.get_recovery_readiness_brief")
            }
            Self::GetCommandLifecycleBrief => read_tool("smart_home.get_command_lifecycle_brief"),
            Self::GetCommandAuditDossier => read_tool("smart_home.get_command_audit_dossier"),
            Self::GetCommandResolutionBrief => {
                read_tool("smart_home.get_command_resolution_brief")
            }
            Self::GetMorningBrief => read_tool("smart_home.get_morning_brief"),
            Self::GetEscalationBrief => read_tool("smart_home.get_escalation_brief"),
            Self::GetContinuityBrief => read_tool("smart_home.get_continuity_brief"),
            Self::GetOperatorReadinessBrief => {
                read_tool("smart_home.get_operator_readiness_brief")
            }
            Self::GetShiftHandoffBrief => read_tool("smart_home.get_shift_handoff_brief"),
            Self::GetCloseoutBrief => read_tool("smart_home.get_closeout_brief"),
            Self::GetCloseoutReceipt => read_tool("smart_home.get_closeout_receipt"),
            Self::GetCloseoutAuditTrail => read_tool("smart_home.get_closeout_audit_trail"),
            Self::GetCloseoutArchive => read_tool("smart_home.get_closeout_archive"),
            Self::GetCloseoutArchiveManifest => {
                read_tool("smart_home.get_closeout_archive_manifest")
            }
            Self::GetCloseoutRetentionLedger => {
                read_tool("smart_home.get_closeout_retention_ledger")
            }
            Self::GetTopologySummary => read_tool("smart_home.get_topology_summary"),
            Self::ListDesiredStates => read_tool("smart_home.list_desired_states"),
            Self::ListDesiredStateDriftAudit => {
                read_tool("smart_home.list_desired_state_drift_audit")
            }
            Self::GetDesiredStateDriftAuditSummary => {
                read_tool("smart_home.get_desired_state_drift_audit_summary")
            }
            Self::ListEventDeliveryAudit => read_tool("smart_home.list_event_delivery_audit"),
            Self::GetEventDeliveryAuditSummary => {
                read_tool("smart_home.get_event_delivery_audit_summary")
            }
            Self::ListStateTransitionAudit => read_tool("smart_home.list_state_transition_audit"),
            Self::GetStateTransitionAuditSummary => {
                read_tool("smart_home.get_state_transition_audit_summary")
            }
            Self::ListSupervisionRemediation => {
                read_tool("smart_home.list_supervision_remediation")
            }
            Self::GetSupervisionRemediationSummary => {
                read_tool("smart_home.get_supervision_remediation_summary")
            }
            Self::ListRuntimeMaintenanceWindows => {
                read_tool("smart_home.list_runtime_maintenance_windows")
            }
            Self::GetRuntimeMaintenanceWindowSummary => {
                read_tool("smart_home.get_runtime_maintenance_window_summary")
            }
            Self::ListRuntimeMaintenanceActions => {
                read_tool("smart_home.list_runtime_maintenance_actions")
            }
            Self::GetRuntimeMaintenanceActionSummary => {
                read_tool("smart_home.get_runtime_maintenance_action_summary")
            }
            Self::ListRuntimeMaintenancePlans => {
                read_tool("smart_home.list_runtime_maintenance_plans")
            }
            Self::GetRuntimeMaintenancePlanSummary => {
                read_tool("smart_home.get_runtime_maintenance_plan_summary")
            }
            Self::ListRuntimeMaintenanceTickets => {
                read_tool("smart_home.list_runtime_maintenance_tickets")
            }
            Self::GetRuntimeMaintenanceTicketSummary => {
                read_tool("smart_home.get_runtime_maintenance_ticket_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrders => {
                read_tool("smart_home.list_runtime_maintenance_work_orders")
            }
            Self::GetRuntimeMaintenanceWorkOrderSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderGuardrails => {
                read_tool("smart_home.list_runtime_maintenance_work_order_guardrails")
            }
            Self::GetRuntimeMaintenanceWorkOrderGuardrailSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_guardrail_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderEvidence => {
                read_tool("smart_home.list_runtime_maintenance_work_order_evidence")
            }
            Self::GetRuntimeMaintenanceWorkOrderEvidenceSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_evidence_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderEvidenceReviews => {
                read_tool("smart_home.list_runtime_maintenance_work_order_evidence_reviews")
            }
            Self::GetRuntimeMaintenanceWorkOrderEvidenceReviewSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_evidence_review_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositions => {
                read_tool("smart_home.list_runtime_maintenance_work_order_evidence_review_dispositions")
            }
            Self::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActions => {
                read_tool("smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_actions")
            }
            Self::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomes => {
                read_tool("smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcomes")
            }
            Self::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadiness => {
                read_tool("smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness")
            }
            Self::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffs => {
                read_tool("smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoffs")
            }
            Self::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_summary")
            }
            Self::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffReconciliations => {
                read_tool("smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_reconciliations")
            }
            Self::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffReconciliationSummary => {
                read_tool("smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_reconciliation_summary")
            }
            Self::ListRuntimeMaintenanceCloseoutPackets => {
                read_tool("smart_home.list_runtime_maintenance_closeout_packets")
            }
            Self::GetRuntimeMaintenanceCloseoutSummary => {
                read_tool("smart_home.get_runtime_maintenance_closeout_summary")
            }
            Self::SetDesiredState => ToolDescriptor {
                tool_id: "smart_home.set_desired_state",
                side_effects: ToolSideEffects::Write,
                required_capabilities: vec![CapabilityId::trusted("smart_home.command.light")],
                required_tier: PrivilegeTier::LowRisk,
            },
            Self::ClearDesiredState => ToolDescriptor {
                tool_id: "smart_home.clear_desired_state",
                side_effects: ToolSideEffects::Write,
                required_capabilities: vec![CapabilityId::trusted("smart_home.command.light")],
                required_tier: PrivilegeTier::LowRisk,
            },
            Self::ListPairingSessions => read_tool("smart_home.list_pairing_sessions"),
            Self::ListWorkers => read_tool("smart_home.list_workers"),
            Self::GetWorkerHeartbeatSchedule => {
                read_tool("smart_home.get_worker_heartbeat_schedule")
            }
            Self::GetSupervisionPlan => read_tool("smart_home.get_supervision_plan"),
            Self::ReconcileDesiredStates => ToolDescriptor {
                tool_id: "smart_home.reconcile_desired_states",
                side_effects: ToolSideEffects::External,
                required_capabilities: vec![CapabilityId::trusted("smart_home.command.light")],
                required_tier: PrivilegeTier::LowRisk,
            },
            Self::RunSupervisionTick => ToolDescriptor {
                tool_id: "smart_home.run_supervision_tick",
                side_effects: ToolSideEffects::External,
                required_capabilities: vec![CapabilityId::trusted("smart_home.command.light")],
                required_tier: PrivilegeTier::LowRisk,
            },
            Self::DescribeCapabilities => read_tool("smart_home.describe_capabilities"),
            Self::GetHealth => read_tool("smart_home.get_health"),
            Self::ObserveSupervision => read_tool("smart_home.observe_supervision"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityGrantStatus {
    Pending,
    Active,
    Revoked,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityGrantScope {
    Tool(SmartHomeTool),
    Capability(CapabilityId),
    EntityCapability {
        entity_id: EntityId,
        capability_id: CapabilityId,
    },
    AllSmartHome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrantInventorySummary {
    pub generated_at_ms: u64,
    pub total_grants: usize,
    pub active_grants: usize,
    pub pending_grants: usize,
    pub revoked_grants: usize,
    pub expired_grants: usize,
    pub tool_grants: usize,
    pub capability_grants: usize,
    pub entity_capability_grants: usize,
    pub all_smart_home_grants: usize,
    pub read_only_tier_grants: usize,
    pub low_risk_tier_grants: usize,
    pub human_approval_tier_grants: usize,
    pub high_risk_tier_grants: usize,
    pub expiring_grants: usize,
    pub unique_principals: usize,
}

impl CapabilityGrantInventorySummary {
    pub fn empty(generated_at_ms: u64) -> Self {
        Self {
            generated_at_ms,
            ..Self::default()
        }
    }

    pub fn from_grants_at<'a, I>(grants: I, now_ms: u64) -> Self
    where
        I: IntoIterator<Item = &'a CapabilityGrant>,
    {
        let mut summary = Self::empty(now_ms);
        let mut principals = BTreeSet::new();
        for grant in grants {
            summary.record_grant_at(grant, now_ms);
            principals.insert(grant.principal_id.clone());
        }
        summary.unique_principals = principals.len();
        summary
    }

    pub fn record_grant_at(&mut self, grant: &CapabilityGrant, now_ms: u64) {
        self.total_grants += 1;
        match grant.status_at(now_ms) {
            CapabilityGrantStatus::Active => self.active_grants += 1,
            CapabilityGrantStatus::Pending => self.pending_grants += 1,
            CapabilityGrantStatus::Revoked => self.revoked_grants += 1,
            CapabilityGrantStatus::Expired => self.expired_grants += 1,
        }
        match &grant.scope {
            CapabilityGrantScope::Tool(_) => self.tool_grants += 1,
            CapabilityGrantScope::Capability(_) => self.capability_grants += 1,
            CapabilityGrantScope::EntityCapability { .. } => self.entity_capability_grants += 1,
            CapabilityGrantScope::AllSmartHome => self.all_smart_home_grants += 1,
        }
        match grant.max_tier {
            PrivilegeTier::ReadOnly => self.read_only_tier_grants += 1,
            PrivilegeTier::LowRisk => self.low_risk_tier_grants += 1,
            PrivilegeTier::HumanApproval => self.human_approval_tier_grants += 1,
            PrivilegeTier::HighRisk => self.high_risk_tier_grants += 1,
        }
        if grant.expires_at_ms.is_some() {
            self.expiring_grants += 1;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_grants == 0
    }

    pub fn has_active_grants(&self) -> bool {
        self.active_grants > 0
    }

    pub fn needs_review(&self) -> bool {
        self.pending_grants > 0 || self.revoked_grants > 0 || self.expired_grants > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationOutcome {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationSubject {
    Tool(SmartHomeTool),
    Command {
        command_id: CommandId,
        entity_id: EntityId,
        command_type: CommandType,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

    pub fn summary(&self) -> AuthorizationDecisionSummary {
        AuthorizationDecisionSummary::from_decision(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthorizationSubjectKind {
    Tool,
    Command,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationDecisionSummary {
    pub subject_kind: AuthorizationSubjectKind,
    pub outcome: AuthorizationOutcome,
    pub required_tier: PrivilegeTier,
    pub required_capability_count: usize,
    pub matched_grant_count: usize,
    pub missing_capability_count: usize,
}

impl AuthorizationDecisionSummary {
    pub fn from_decision(decision: &AuthorizationDecision) -> Self {
        let subject_kind = match &decision.subject {
            AuthorizationSubject::Tool(_) => AuthorizationSubjectKind::Tool,
            AuthorizationSubject::Command { .. } => AuthorizationSubjectKind::Command,
        };
        Self {
            subject_kind,
            outcome: decision.outcome,
            required_tier: decision.required_tier,
            required_capability_count: decision.required_capabilities.len(),
            matched_grant_count: decision.matched_grants.len(),
            missing_capability_count: decision.missing_capabilities.len(),
        }
    }

    pub fn is_allowed(&self) -> bool {
        self.outcome == AuthorizationOutcome::Allowed
    }

    pub fn is_denied(&self) -> bool {
        self.outcome == AuthorizationOutcome::Denied
    }

    pub fn has_missing_capabilities(&self) -> bool {
        self.missing_capability_count > 0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorizationDecisionLogSummary {
    pub total_decisions: usize,
    pub allowed_decisions: usize,
    pub denied_decisions: usize,
    pub tool_decisions: usize,
    pub command_decisions: usize,
    pub read_only_tier_decisions: usize,
    pub low_risk_tier_decisions: usize,
    pub human_approval_tier_decisions: usize,
    pub high_risk_tier_decisions: usize,
    pub decisions_with_missing_capabilities: usize,
    pub total_required_capabilities: usize,
    pub total_matched_grants: usize,
    pub total_missing_capabilities: usize,
}

impl AuthorizationDecisionLogSummary {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_decisions<'a, I>(decisions: I) -> Self
    where
        I: IntoIterator<Item = &'a AuthorizationDecision>,
    {
        let mut summary = Self::empty();
        for decision in decisions {
            summary.record_summary(&decision.summary());
        }
        summary
    }

    pub fn from_summaries<'a, I>(summaries: I) -> Self
    where
        I: IntoIterator<Item = &'a AuthorizationDecisionSummary>,
    {
        let mut summary = Self::empty();
        for decision_summary in summaries {
            summary.record_summary(decision_summary);
        }
        summary
    }

    pub fn record_summary(&mut self, decision_summary: &AuthorizationDecisionSummary) {
        self.total_decisions += 1;
        self.total_required_capabilities += decision_summary.required_capability_count;
        self.total_matched_grants += decision_summary.matched_grant_count;
        self.total_missing_capabilities += decision_summary.missing_capability_count;

        match decision_summary.subject_kind {
            AuthorizationSubjectKind::Tool => self.tool_decisions += 1,
            AuthorizationSubjectKind::Command => self.command_decisions += 1,
        }
        match decision_summary.outcome {
            AuthorizationOutcome::Allowed => self.allowed_decisions += 1,
            AuthorizationOutcome::Denied => self.denied_decisions += 1,
        }
        match decision_summary.required_tier {
            PrivilegeTier::ReadOnly => self.read_only_tier_decisions += 1,
            PrivilegeTier::LowRisk => self.low_risk_tier_decisions += 1,
            PrivilegeTier::HumanApproval => self.human_approval_tier_decisions += 1,
            PrivilegeTier::HighRisk => self.high_risk_tier_decisions += 1,
        }
        if decision_summary.has_missing_capabilities() {
            self.decisions_with_missing_capabilities += 1;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.total_decisions == 0
    }

    pub fn has_denials(&self) -> bool {
        self.denied_decisions > 0
    }

    pub fn has_missing_capabilities(&self) -> bool {
        self.total_missing_capabilities > 0
    }

    pub fn approval_gated_decisions(&self) -> usize {
        self.human_approval_tier_decisions + self.high_risk_tier_decisions
    }
}

pub fn smart_home_tool_catalog() -> Vec<ToolDescriptor> {
    [
        SmartHomeTool::ListIntegrations,
        SmartHomeTool::DescribeIntegration,
        SmartHomeTool::ListPrimitives,
        SmartHomeTool::DescribePrimitive,
        SmartHomeTool::GetIntegrationCatalogSummary,
        SmartHomeTool::GetToolCatalogSummary,
        SmartHomeTool::ListIntegrationPolicySurfaces,
        SmartHomeTool::GetIntegrationPolicySurfaceSummary,
        SmartHomeTool::ListIntegrationPlatformCoverage,
        SmartHomeTool::GetIntegrationPlatformCoverageSummary,
        SmartHomeTool::ListIntegrationPrimitiveCoverage,
        SmartHomeTool::GetIntegrationPrimitiveCoverageSummary,
        SmartHomeTool::ListIntegrationActivationPlans,
        SmartHomeTool::GetIntegrationActivationPlanSummary,
        SmartHomeTool::ListIntegrationActivationCandidates,
        SmartHomeTool::GetIntegrationActivationCandidateSummary,
        SmartHomeTool::ListIntegrationActivationActions,
        SmartHomeTool::GetIntegrationActivationActionSummary,
        SmartHomeTool::ListIntegrationActivationAgenda,
        SmartHomeTool::GetIntegrationActivationAgendaSummary,
        SmartHomeTool::ListIntegrationActivationRunway,
        SmartHomeTool::GetIntegrationActivationRunwaySummary,
        SmartHomeTool::ListIntegrationActivationHealth,
        SmartHomeTool::GetIntegrationActivationHealthSummary,
        SmartHomeTool::ListIntegrationActivationMaintenance,
        SmartHomeTool::GetIntegrationActivationMaintenanceSummary,
        SmartHomeTool::ListIntegrationActivationConstraints,
        SmartHomeTool::GetIntegrationActivationConstraintSummary,
        SmartHomeTool::ListIntegrationActivationReviews,
        SmartHomeTool::GetIntegrationActivationReviewSummary,
        SmartHomeTool::ListIntegrationActivationApprovals,
        SmartHomeTool::GetIntegrationActivationApprovalSummary,
        SmartHomeTool::ListIntegrationActivationDecisions,
        SmartHomeTool::GetIntegrationActivationDecisionSummary,
        SmartHomeTool::ListIntegrationActivationEvidence,
        SmartHomeTool::GetIntegrationActivationEvidenceSummary,
        SmartHomeTool::ListIntegrationActivationEvidenceRemediation,
        SmartHomeTool::GetIntegrationActivationEvidenceRemediationSummary,
        SmartHomeTool::ListIntegrationActivationEvidenceLaneInventory,
        SmartHomeTool::GetIntegrationActivationEvidenceLaneInventorySummary,
        SmartHomeTool::GetIntegrationActivationEvidenceScorecardSummary,
        SmartHomeTool::ListIntegrationActivationDossiers,
        SmartHomeTool::GetIntegrationActivationDossierSummary,
        SmartHomeTool::ListIntegrationActivationReadouts,
        SmartHomeTool::GetIntegrationActivationReadoutSummary,
        SmartHomeTool::ListIntegrationActivationBriefingItems,
        SmartHomeTool::GetIntegrationActivationBriefingSummary,
        SmartHomeTool::ListIntegrationActivationDashboard,
        SmartHomeTool::GetIntegrationActivationDashboardSummary,
        SmartHomeTool::ListIntegrationActivationTimeline,
        SmartHomeTool::GetIntegrationActivationTimelineSummary,
        SmartHomeTool::ListIntegrationActivationForecast,
        SmartHomeTool::GetIntegrationActivationForecastSummary,
        SmartHomeTool::ListIntegrationActivationPlaybook,
        SmartHomeTool::GetIntegrationActivationPlaybookSummary,
        SmartHomeTool::ListIntegrationActivationRunbook,
        SmartHomeTool::GetIntegrationActivationRunbookSummary,
        SmartHomeTool::ListIntegrationActivationHandoff,
        SmartHomeTool::GetIntegrationActivationHandoffSummary,
        SmartHomeTool::ListIntegrationActivationExecution,
        SmartHomeTool::GetIntegrationActivationExecutionSummary,
        SmartHomeTool::ListIntegrationActivationVerification,
        SmartHomeTool::GetIntegrationActivationVerificationSummary,
        SmartHomeTool::ListIntegrationActivationOperatorQueue,
        SmartHomeTool::GetIntegrationActivationOperatorQueueSummary,
        SmartHomeTool::ListIntegrationActivationControlRoom,
        SmartHomeTool::GetIntegrationActivationControlRoomSummary,
        SmartHomeTool::ListIntegrationActivationCommandCenter,
        SmartHomeTool::GetIntegrationActivationCommandCenterSummary,
        SmartHomeTool::ListIntegrationActivationWatchtower,
        SmartHomeTool::GetIntegrationActivationWatchtowerSummary,
        SmartHomeTool::ListIntegrationActivationSentinel,
        SmartHomeTool::GetIntegrationActivationSentinelSummary,
        SmartHomeTool::ListIntegrationActivationAudit,
        SmartHomeTool::GetIntegrationActivationAuditSummary,
        SmartHomeTool::ListIntegrationActivationEscalations,
        SmartHomeTool::GetIntegrationActivationEscalationSummary,
        SmartHomeTool::ListIntegrationActivationResponses,
        SmartHomeTool::GetIntegrationActivationResponseSummary,
        SmartHomeTool::ListIntegrationActivationRemediation,
        SmartHomeTool::GetIntegrationActivationRemediationSummary,
        SmartHomeTool::ListIntegrationActivationClosure,
        SmartHomeTool::GetIntegrationActivationClosureSummary,
        SmartHomeTool::ListIntegrationActivationRelease,
        SmartHomeTool::GetIntegrationActivationReleaseSummary,
        SmartHomeTool::ListIntegrationActivationDelivery,
        SmartHomeTool::GetIntegrationActivationDeliverySummary,
        SmartHomeTool::ListIntegrationActivationDeployment,
        SmartHomeTool::GetIntegrationActivationDeploymentSummary,
        SmartHomeTool::ListIntegrationActivationSafetyGates,
        SmartHomeTool::GetIntegrationActivationSafetySummary,
        SmartHomeTool::ListIntegrationActivationRollback,
        SmartHomeTool::GetIntegrationActivationRollbackSummary,
        SmartHomeTool::ListIntegrationActivationObservability,
        SmartHomeTool::GetIntegrationActivationObservabilitySummary,
        SmartHomeTool::ListIntegrationActivationIncidents,
        SmartHomeTool::GetIntegrationActivationIncidentSummary,
        SmartHomeTool::ListIntegrationActivationGuardrails,
        SmartHomeTool::GetIntegrationActivationGuardrailSummary,
        SmartHomeTool::ListIntegrationActivationAssurance,
        SmartHomeTool::GetIntegrationActivationAssuranceSummary,
        SmartHomeTool::ListIntegrationActivationGovernance,
        SmartHomeTool::GetIntegrationActivationGovernanceSummary,
        SmartHomeTool::ListIntegrationActivationCompliance,
        SmartHomeTool::GetIntegrationActivationComplianceSummary,
        SmartHomeTool::ListIntegrationActivationAttestations,
        SmartHomeTool::GetIntegrationActivationAttestationSummary,
        SmartHomeTool::ListIntegrationActivationEvidenceLedger,
        SmartHomeTool::GetIntegrationActivationEvidenceLedgerSummary,
        SmartHomeTool::ListIntegrationActivationExceptionLedger,
        SmartHomeTool::GetIntegrationActivationExceptionLedgerSummary,
        SmartHomeTool::ListIntegrationActivationWaiverRegister,
        SmartHomeTool::GetIntegrationActivationWaiverRegisterSummary,
        SmartHomeTool::ListIntegrationActivationWaiverReviews,
        SmartHomeTool::GetIntegrationActivationWaiverReviewSummary,
        SmartHomeTool::ListIntegrationActivationWaiverDispositions,
        SmartHomeTool::GetIntegrationActivationWaiverDispositionSummary,
        SmartHomeTool::ListIntegrationActivationWaiverRemediations,
        SmartHomeTool::GetIntegrationActivationWaiverRemediationSummary,
        SmartHomeTool::ListIntegrationActivationWaiverClosures,
        SmartHomeTool::GetIntegrationActivationWaiverClosureSummary,
        SmartHomeTool::ListIntegrationActivationWaiverArchives,
        SmartHomeTool::GetIntegrationActivationWaiverArchiveSummary,
        SmartHomeTool::ListIntegrationActivationWaiverRetention,
        SmartHomeTool::GetIntegrationActivationWaiverRetentionSummary,
        SmartHomeTool::ListIntegrationActivationWaiverExpirations,
        SmartHomeTool::GetIntegrationActivationWaiverExpirationSummary,
        SmartHomeTool::ListIntegrationActivationWaiverDisposals,
        SmartHomeTool::GetIntegrationActivationWaiverDisposalSummary,
        SmartHomeTool::ListIntegrationActivationWaiverTombstones,
        SmartHomeTool::GetIntegrationActivationWaiverTombstoneSummary,
        SmartHomeTool::ListIntegrationActivationWaiverPurges,
        SmartHomeTool::GetIntegrationActivationWaiverPurgeSummary,
        SmartHomeTool::ListIntegrationActivationWaiverErasures,
        SmartHomeTool::GetIntegrationActivationWaiverErasureSummary,
        SmartHomeTool::ListIntegrationActivationWaiverErasureReceipts,
        SmartHomeTool::GetIntegrationActivationWaiverErasureReceiptSummary,
        SmartHomeTool::ListIntegrationActivationWaiverReleaseClosures,
        SmartHomeTool::GetIntegrationActivationWaiverReleaseClosureSummary,
        SmartHomeTool::ListIntegrationActivationWaiverReleaseSignoffs,
        SmartHomeTool::GetIntegrationActivationWaiverReleaseSignoffSummary,
        SmartHomeTool::ListIntegrationActivationWaiverReleaseCertifications,
        SmartHomeTool::GetIntegrationActivationWaiverReleaseCertificationSummary,
        SmartHomeTool::ListIntegrationActivationWaiverReleaseCertificationRemediations,
        SmartHomeTool::GetIntegrationActivationWaiverReleaseCertificationRemediationSummary,
        SmartHomeTool::ListIntegrationActivationRisk,
        SmartHomeTool::GetIntegrationActivationRiskSummary,
        SmartHomeTool::ListIntegrationActivationDependencies,
        SmartHomeTool::GetIntegrationActivationDependencySummary,
        SmartHomeTool::ListIntegrationReadiness,
        SmartHomeTool::GetIntegrationReadinessSummary,
        SmartHomeTool::ListIntegrationReadinessGaps,
        SmartHomeTool::GetIntegrationReadinessGapSummary,
        SmartHomeTool::ListIntegrationMeshPrimitiveReadiness,
        SmartHomeTool::GetIntegrationMeshPrimitiveReadinessSummary,
        SmartHomeTool::ListIntegrationMeshSubstrateStages,
        SmartHomeTool::GetIntegrationMeshSubstrateStageSummary,
        SmartHomeTool::ListIntegrationMeshSubstrateActions,
        SmartHomeTool::GetIntegrationMeshSubstrateActionSummary,
        SmartHomeTool::ListIntegrationMeshSubstratePreflightChecks,
        SmartHomeTool::GetIntegrationMeshSubstratePreflightSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairActions,
        SmartHomeTool::GetIntegrationMeshPreflightRepairActionSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairBatches,
        SmartHomeTool::GetIntegrationMeshPreflightRepairBatchSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSchedule,
        SmartHomeTool::GetIntegrationMeshPreflightRepairScheduleSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSlotAudits,
        SmartHomeTool::GetIntegrationMeshPreflightRepairSlotAuditSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSlotExecutionTickets,
        SmartHomeTool::GetIntegrationMeshPreflightRepairSlotExecutionTicketSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSlotExecutionWorkOrders,
        SmartHomeTool::GetIntegrationMeshPreflightRepairSlotExecutionWorkOrderSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSlotExecutionWorkOrderGuardrails,
        SmartHomeTool::GetIntegrationMeshPreflightRepairSlotExecutionWorkOrderGuardrailSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSlotExecutionEvidence,
        SmartHomeTool::GetIntegrationMeshPreflightRepairSlotExecutionEvidenceSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviews,
        SmartHomeTool::GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositions,
        SmartHomeTool::GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionSummary,
        SmartHomeTool::ListIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionActions,
        SmartHomeTool::GetIntegrationMeshPreflightRepairSlotExecutionEvidenceReviewDispositionActionSummary,
        SmartHomeTool::GetIntegrationMeshPreflightReadinessSummary,
        SmartHomeTool::GetIntegrationMeshPreflightRepairReadinessSummary,
        SmartHomeTool::GetIntegrationMeshPreflightBatchReadinessSummary,
        SmartHomeTool::GetIntegrationMeshPreflightScheduleReadinessSummary,
        SmartHomeTool::GetIntegrationMeshPreflightSlotReadinessSummary,
        SmartHomeTool::GetIntegrationMeshPreflightExecutionReadinessSummary,
        SmartHomeTool::GetIntegrationMeshPreflightWorkOrderReadinessSummary,
        SmartHomeTool::GetIntegrationMeshPreflightGuardrailReadinessSummary,
        SmartHomeTool::GetIntegrationMeshReadinessPackageSummary,
        SmartHomeTool::GetIntegrationMeshStageReleaseSummary,
        SmartHomeTool::GetIntegrationMeshActionReadinessSummary,
        SmartHomeTool::GetIntegrationMeshReleaseReadinessSummary,
        SmartHomeTool::ListIntegrationMeshReadinessHandoffs,
        SmartHomeTool::GetIntegrationMeshReadinessHandoffSummary,
        SmartHomeTool::ListIntegrationMeshReleaseReadinessChecks,
        SmartHomeTool::GetIntegrationMeshReleaseReadinessCheckSummary,
        SmartHomeTool::Discover,
        SmartHomeTool::PairBridge,
        SmartHomeTool::CompletePairing,
        SmartHomeTool::ListDiscoveryWorkers,
        SmartHomeTool::GetDiscoverySummary,
        SmartHomeTool::GetPairingPlan,
        SmartHomeTool::ListBridges,
        SmartHomeTool::ListDevices,
        SmartHomeTool::ListDeviceInventoryAudit,
        SmartHomeTool::GetDeviceInventoryAuditSummary,
        SmartHomeTool::ListRoomTopologyAudit,
        SmartHomeTool::GetRoomTopologyAuditSummary,
        SmartHomeTool::ListRooms,
        SmartHomeTool::ListSceneCoverageAudit,
        SmartHomeTool::GetSceneCoverageAuditSummary,
        SmartHomeTool::ListScenes,
        SmartHomeTool::DescribeScene,
        SmartHomeTool::GetState,
        SmartHomeTool::Command,
        SmartHomeTool::ReportEvent,
        SmartHomeTool::Subscribe,
        SmartHomeTool::PollEvents,
        SmartHomeTool::Unsubscribe,
        SmartHomeTool::ListSubscriptions,
        SmartHomeTool::InspectEventLog,
        SmartHomeTool::ListCommandResults,
        SmartHomeTool::GetCommandResultSummary,
        SmartHomeTool::ListCommandRiskAudit,
        SmartHomeTool::GetCommandRiskAuditSummary,
        SmartHomeTool::ListAuthorizationGapAudit,
        SmartHomeTool::GetAuthorizationGapAuditSummary,
        SmartHomeTool::ListAuthorizationDecisions,
        SmartHomeTool::GetAuthorizationSummary,
        SmartHomeTool::ListCapabilityGrants,
        SmartHomeTool::GetCapabilityGrantSummary,
        SmartHomeTool::GetControllerHandoffSummary,
        SmartHomeTool::GetPlatformBrief,
        SmartHomeTool::ListPlatformEvidenceLedger,
        SmartHomeTool::GetPlatformEvidenceLedgerSummary,
        SmartHomeTool::ListPlatformAccessReview,
        SmartHomeTool::GetPlatformAccessReviewSummary,
        SmartHomeTool::ListPlatformEventOpsReview,
        SmartHomeTool::GetPlatformEventOpsReviewSummary,
        SmartHomeTool::GetRuntimeSnapshot,
        SmartHomeTool::GetPendingWorkSummary,
        SmartHomeTool::GetAttentionOverview,
        SmartHomeTool::GetSystemHealthBrief,
        SmartHomeTool::GetOperatorActionBrief,
        SmartHomeTool::GetServiceExecutionReadinessBrief,
        SmartHomeTool::GetServiceExecutionSafetyBrief,
        SmartHomeTool::GetRemediationPlan,
        SmartHomeTool::GetOperationsBrief,
        SmartHomeTool::GetSafetyBrief,
        SmartHomeTool::GetReadinessBrief,
        SmartHomeTool::GetMaintenanceBrief,
        SmartHomeTool::GetIncidentBrief,
        SmartHomeTool::GetRecoveryBrief,
        SmartHomeTool::GetRecoveryReadinessBrief,
        SmartHomeTool::GetCommandLifecycleBrief,
        SmartHomeTool::GetCommandAuditDossier,
        SmartHomeTool::GetCommandResolutionBrief,
        SmartHomeTool::GetMorningBrief,
        SmartHomeTool::GetEscalationBrief,
        SmartHomeTool::GetContinuityBrief,
        SmartHomeTool::GetOperatorReadinessBrief,
        SmartHomeTool::GetShiftHandoffBrief,
        SmartHomeTool::GetCloseoutBrief,
        SmartHomeTool::GetCloseoutReceipt,
        SmartHomeTool::GetCloseoutAuditTrail,
        SmartHomeTool::GetCloseoutArchive,
        SmartHomeTool::GetCloseoutArchiveManifest,
        SmartHomeTool::GetCloseoutRetentionLedger,
        SmartHomeTool::GetTopologySummary,
        SmartHomeTool::ListDesiredStates,
        SmartHomeTool::ListDesiredStateDriftAudit,
        SmartHomeTool::GetDesiredStateDriftAuditSummary,
        SmartHomeTool::ListEventDeliveryAudit,
        SmartHomeTool::GetEventDeliveryAuditSummary,
        SmartHomeTool::ListStateTransitionAudit,
        SmartHomeTool::GetStateTransitionAuditSummary,
        SmartHomeTool::ListSupervisionRemediation,
        SmartHomeTool::GetSupervisionRemediationSummary,
        SmartHomeTool::ListRuntimeMaintenanceWindows,
        SmartHomeTool::GetRuntimeMaintenanceWindowSummary,
        SmartHomeTool::ListRuntimeMaintenanceActions,
        SmartHomeTool::GetRuntimeMaintenanceActionSummary,
        SmartHomeTool::ListRuntimeMaintenancePlans,
        SmartHomeTool::GetRuntimeMaintenancePlanSummary,
        SmartHomeTool::ListRuntimeMaintenanceTickets,
        SmartHomeTool::GetRuntimeMaintenanceTicketSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrders,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderGuardrails,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderGuardrailSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderEvidence,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderEvidenceSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderEvidenceReviews,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderEvidenceReviewSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositions,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActions,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomes,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadiness,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffs,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffSummary,
        SmartHomeTool::ListRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffReconciliations,
        SmartHomeTool::GetRuntimeMaintenanceWorkOrderEvidenceReviewDispositionActionOutcomeReadinessHandoffReconciliationSummary,
        SmartHomeTool::ListRuntimeMaintenanceCloseoutPackets,
        SmartHomeTool::GetRuntimeMaintenanceCloseoutSummary,
        SmartHomeTool::SetDesiredState,
        SmartHomeTool::ClearDesiredState,
        SmartHomeTool::ListPairingSessions,
        SmartHomeTool::ListWorkers,
        SmartHomeTool::GetWorkerHeartbeatSchedule,
        SmartHomeTool::GetSupervisionPlan,
        SmartHomeTool::ReconcileDesiredStates,
        SmartHomeTool::RunSupervisionTick,
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
    fn integration_descriptor_surface_summary_is_payload_free() {
        let hue = canonical_integration_descriptor(&IntegrationId::trusted("hue")).unwrap();
        let summary = hue.surface_summary();

        assert_eq!(summary.integration_id, IntegrationId::trusted("hue"));
        assert_eq!(summary.display_name, "Philips Hue Bridge");
        assert_eq!(summary.version, "0.1.0");
        assert_eq!(summary.runtime_kind, RuntimeKind::RustWorkerProcess);
        assert_eq!(summary.capability_count, hue.capabilities.len());
        assert_eq!(summary.discovery_role_count, hue.discovery_roles.len());
        assert_eq!(summary.pairing_role_count, hue.pairing_roles.len());
        assert!(summary.exposes_capabilities);
        assert!(summary.supports_discovery);
        assert!(summary.supports_pairing);
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

        assert_eq!(catalog.len(), 322);
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_command_risk_audit"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_desired_state_drift_audit"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(
            |tool| tool.tool_id == "smart_home.list_event_delivery_audit"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_event_delivery_audit_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_attention_overview"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_system_health_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(
            |tool| tool.tool_id == "smart_home.get_operator_action_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_remediation_plan"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_platform_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_platform_evidence_ledger"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_platform_evidence_ledger_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_platform_access_review"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_platform_access_review_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_platform_event_ops_review"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_platform_event_ops_review_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_operations_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_safety_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_readiness_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_maintenance_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_incident_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_recovery_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_morning_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_escalation_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_continuity_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_operator_readiness_brief"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_shift_handoff_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_closeout_brief"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_closeout_receipt"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_closeout_audit_trail"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_closeout_archive"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_closeout_archive_manifest"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_closeout_retention_ledger"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_state_transition_audit"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_state_transition_audit_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_authorization_gap_audit"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_authorization_gap_audit_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_supervision_remediation"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_supervision_remediation_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_windows"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_window_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_actions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_action_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_plans"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_plan_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_tickets"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_ticket_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_orders"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_guardrails"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_guardrail_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_evidence"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_evidence_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_evidence_reviews"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_evidence_review_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_evidence_review_dispositions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_actions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcomes"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoffs"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_reconciliations"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_work_order_evidence_review_disposition_action_outcome_readiness_handoff_reconciliation_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_runtime_maintenance_closeout_packets"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_runtime_maintenance_closeout_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_integrations"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.describe_integration"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_primitives"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.describe_primitive"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_catalog_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_tool_catalog_summary"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_policy_surfaces"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_policy_surface_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_platform_coverage"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_risk"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_risk_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_platform_coverage_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_primitive_coverage"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_primitive_coverage_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_plans"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_plan_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_candidates"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_candidate_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_actions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_action_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_agenda"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_agenda_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_runway"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_runway_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_health"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_health_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_maintenance"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_maintenance_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_constraints"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_constraint_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_reviews"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_review_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_approvals"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_approval_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_decisions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_decision_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_evidence"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_evidence_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_evidence_remediation"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_evidence_remediation_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_evidence_lane_inventory"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_evidence_lane_inventory_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_evidence_scorecard_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_dossiers"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_dossier_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_readouts"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_readout_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_briefing_items"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_briefing_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_dashboard"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_dashboard_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_timeline"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_timeline_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(
            |tool| tool.tool_id == "smart_home.list_integration_readiness"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_readiness_gaps"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_readiness_gap_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        for tool_id in [
            "smart_home.list_integration_mesh_primitive_readiness",
            "smart_home.get_integration_mesh_primitive_readiness_summary",
            "smart_home.list_integration_mesh_substrate_stages",
            "smart_home.get_integration_mesh_substrate_stage_summary",
            "smart_home.list_integration_mesh_substrate_actions",
            "smart_home.get_integration_mesh_substrate_action_summary",
            "smart_home.get_integration_mesh_readiness_package_summary",
            "smart_home.get_integration_mesh_stage_release_summary",
            "smart_home.get_integration_mesh_action_readiness_summary",
            "smart_home.get_integration_mesh_release_readiness_summary",
            "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence",
            "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_summary",
            "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_reviews",
            "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_summary",
            "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_review_dispositions",
            "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_summary",
            "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_actions",
            "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_action_summary",
            "smart_home.get_integration_mesh_preflight_slot_readiness_summary",
            "smart_home.get_integration_mesh_preflight_work_order_readiness_summary",
            "smart_home.get_integration_mesh_preflight_guardrail_readiness_summary",
            "smart_home.list_integration_mesh_readiness_handoffs",
            "smart_home.get_integration_mesh_readiness_handoff_summary",
            "smart_home.list_integration_mesh_release_readiness_checks",
            "smart_home.get_integration_mesh_release_readiness_check_summary",
        ] {
            assert!(catalog.iter().any(|tool| tool.tool_id == tool_id
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        }
        assert_eq!(command.side_effects, ToolSideEffects::External);
        assert_eq!(
            command.required_capabilities,
            vec![CapabilityId::trusted("smart_home.command.light")]
        );
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.reconcile_desired_states"
                && tool.side_effects == ToolSideEffects::External
                && tool.required_capabilities
                    == vec![CapabilityId::trusted("smart_home.command.light")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.run_supervision_tick"
                && tool.side_effects == ToolSideEffects::External
                && tool.required_capabilities
                    == vec![CapabilityId::trusted("smart_home.command.light")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.report_event"
                && tool.side_effects == ToolSideEffects::External
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.ingest")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.observe_supervision"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_discovery_workers"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_discovery_summary"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.complete_pairing"
                && tool.side_effects == ToolSideEffects::External
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.pair")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_pairing_plan"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_device_inventory_audit"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_device_inventory_audit_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_room_topology_audit"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_room_topology_audit_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(
            |tool| tool.tool_id == "smart_home.list_scene_coverage_audit"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_scene_coverage_audit_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_scenes"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_rooms"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.describe_scene"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.poll_events"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.unsubscribe"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_subscriptions"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.inspect_event_log"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_command_results"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(
            |tool| tool.tool_id == "smart_home.get_command_result_summary"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_authorization_decisions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(
            |tool| tool.tool_id == "smart_home.get_authorization_summary"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_capability_grants"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_capability_grant_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_controller_handoff_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_runtime_snapshot"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_pending_work_summary"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_topology_summary"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_desired_states"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.set_desired_state"
                && tool.side_effects == ToolSideEffects::Write
                && tool.required_capabilities
                    == vec![CapabilityId::trusted("smart_home.command.light")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.clear_desired_state"
                && tool.side_effects == ToolSideEffects::Write
                && tool.required_capabilities
                    == vec![CapabilityId::trusted("smart_home.command.light")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_pairing_sessions"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.get_supervision_plan"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog
            .iter()
            .any(|tool| tool.tool_id == "smart_home.list_workers"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_worker_heartbeat_schedule"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_dependencies"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_dependency_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_forecasts"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_forecast_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_playbook"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_playbook_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_runbook"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_runbook_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_handoff"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_handoff_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_execution"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_execution_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_verification"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_verification_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_operator_queue"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_operator_queue_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_control_room"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_control_room_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_command_center"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_command_center_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_watchtower"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_watchtower_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_sentinel"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_sentinel_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_audit"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_audit_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_escalations"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_escalation_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_responses"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_response_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_remediation"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_remediation_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_closure"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_closure_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_release"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_release_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_delivery"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_delivery_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_deployment"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_deployment_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_safety_gates"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_safety_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_rollback"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_rollback_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_observability"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_observability_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_incidents"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_incident_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_guardrails"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_guardrail_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_assurance"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_assurance_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_governance"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_governance_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_compliance"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_compliance_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_attestations"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_attestation_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_evidence_ledger"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_evidence_ledger_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_exception_ledger"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_exception_ledger_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_register"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_register_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_reviews"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_review_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_dispositions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_disposition_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_remediations"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_remediation_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_closures"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_closure_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_archives"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_archive_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_retention"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_retention_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_expirations"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_expiration_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_disposals"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_disposal_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_tombstones"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_tombstone_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_purges"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_purge_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_erasures"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_erasure_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_erasure_receipts"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_erasure_receipt_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_release_closures"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_release_closure_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_release_signoffs"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_release_signoff_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_release_certifications"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_release_certification_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_activation_waiver_release_certification_remediations"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_activation_waiver_release_certification_remediation_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_substrate_preflight_checks"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_substrate_preflight_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_actions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_action_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_batches"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_batch_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_schedule"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_schedule_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_slot_audits"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_slot_audit_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_slot_execution_tickets"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_slot_execution_ticket_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_slot_execution_work_orders"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_slot_execution_work_order_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_slot_execution_work_order_guardrails"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_slot_execution_work_order_guardrail_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_reviews"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_review_dispositions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.list_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_actions"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_slot_execution_evidence_review_disposition_action_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_repair_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_batch_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_schedule_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_slot_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_execution_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_work_order_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_integration_mesh_preflight_guardrail_readiness_summary"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_service_execution_readiness_brief"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_service_execution_safety_brief"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_recovery_readiness_brief"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_command_lifecycle_brief"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
        assert!(catalog.iter().any(
            |tool| tool.tool_id == "smart_home.get_command_audit_dossier"
                && tool.side_effects == ToolSideEffects::Read
                && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]
        ));
        assert!(catalog.iter().any(|tool| tool.tool_id
            == "smart_home.get_command_resolution_brief"
            && tool.side_effects == ToolSideEffects::Read
            && tool.required_capabilities == vec![CapabilityId::trusted("smart_home.read")]));
    }

    #[test]
    fn tool_catalog_summary_counts_risk_tiers_and_capabilities() {
        let summary = smart_home_tool_catalog_summary();
        let pair_bridge = SmartHomeTool::PairBridge.descriptor();

        assert_eq!(summary.total_tools, 322);
        assert_eq!(summary.read_tools, 314);
        assert_eq!(summary.write_tools, 2);
        assert_eq!(summary.external_tools, 6);
        assert_eq!(summary.read_only_tier_tools, 314);
        assert_eq!(summary.low_risk_tier_tools, 6);
        assert_eq!(summary.high_risk_tier_tools, 0);
        assert_eq!(summary.human_approval_tier_tools, 2);
        assert_eq!(summary.total_required_capabilities, 322);
        assert_eq!(summary.risky_tool_count(), 8);
        assert_eq!(summary.approval_gated_tool_count(), 2);
        assert!(pair_bridge.requires_human_approval());
        assert!(SmartHomeTool::CompletePairing
            .descriptor()
            .requires_human_approval());
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
    fn capability_grant_inventory_summary_counts_status_scope_and_tier() {
        let lighting_principal = AgentId::trusted("agent:lighting-planner");
        let installer_principal = AgentId::trusted("agent:installer");
        let grants = vec![
            CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant-read"),
                lighting_principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "chief-of-staff",
                1_000,
            ),
            CapabilityGrant::for_tool(
                CapabilityGrantId::trusted("grant-command"),
                lighting_principal.clone(),
                SmartHomeTool::Command,
                "chief-of-staff",
                1_010,
            )
            .with_expiry(2_000),
            CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("grant-entity-command"),
                lighting_principal,
                EntityId::trusted("entity-light-1"),
                CapabilityId::trusted("light.on_off"),
                PrivilegeTier::LowRisk,
                "chief-of-staff",
                1_020,
            )
            .with_status(CapabilityGrantStatus::Revoked),
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant-installer"),
                installer_principal,
                PrivilegeTier::HumanApproval,
                "chief-of-staff",
                1_030,
            )
            .with_status(CapabilityGrantStatus::Pending),
        ];

        let summary = CapabilityGrantInventorySummary::from_grants_at(&grants, 2_000);

        assert_eq!(summary.generated_at_ms, 2_000);
        assert_eq!(summary.total_grants, 4);
        assert_eq!(summary.active_grants, 1);
        assert_eq!(summary.pending_grants, 1);
        assert_eq!(summary.revoked_grants, 1);
        assert_eq!(summary.expired_grants, 1);
        assert_eq!(summary.tool_grants, 1);
        assert_eq!(summary.capability_grants, 1);
        assert_eq!(summary.entity_capability_grants, 1);
        assert_eq!(summary.all_smart_home_grants, 1);
        assert_eq!(summary.read_only_tier_grants, 1);
        assert_eq!(summary.low_risk_tier_grants, 2);
        assert_eq!(summary.human_approval_tier_grants, 1);
        assert_eq!(summary.high_risk_tier_grants, 0);
        assert_eq!(summary.expiring_grants, 1);
        assert_eq!(summary.unique_principals, 2);
        assert!(summary.has_active_grants());
        assert!(summary.needs_review());
        assert!(CapabilityGrantInventorySummary::empty(3_000).is_empty());
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

    #[test]
    fn authorization_decision_summary_projects_allow_and_deny_shape() {
        let principal = AgentId::trusted("agent:lighting-planner");
        let grant = CapabilityGrant::for_tool(
            CapabilityGrantId::trusted("grant-command"),
            principal.clone(),
            SmartHomeTool::Command,
            "chief-of-staff",
            1_000,
        );
        let allowed =
            AuthorizationDecision::for_tool(principal, SmartHomeTool::Command, [&grant], 1_500);
        let allowed_summary = allowed.summary();

        assert_eq!(allowed_summary.subject_kind, AuthorizationSubjectKind::Tool);
        assert!(allowed_summary.is_allowed());
        assert!(!allowed_summary.is_denied());
        assert_eq!(allowed_summary.required_tier, PrivilegeTier::LowRisk);
        assert_eq!(allowed_summary.required_capability_count, 1);
        assert_eq!(allowed_summary.matched_grant_count, 1);
        assert_eq!(allowed_summary.missing_capability_count, 0);
        assert!(!allowed_summary.has_missing_capabilities());

        let denied_principal = AgentId::trusted("agent:security-agent");
        let low_risk_lock_grant = CapabilityGrant::for_entity_capability(
            CapabilityGrantId::trusted("grant-lock-low"),
            denied_principal.clone(),
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
        let denied = AuthorizationDecision::for_command(
            denied_principal,
            &command,
            [&low_risk_lock_grant],
            1_500,
        );
        let denied_summary = AuthorizationDecisionSummary::from_decision(&denied);

        assert_eq!(
            denied_summary.subject_kind,
            AuthorizationSubjectKind::Command
        );
        assert!(denied_summary.is_denied());
        assert_eq!(denied_summary.required_tier, PrivilegeTier::HighRisk);
        assert_eq!(denied_summary.required_capability_count, 1);
        assert_eq!(denied_summary.matched_grant_count, 0);
        assert_eq!(denied_summary.missing_capability_count, 1);
        assert!(denied_summary.has_missing_capabilities());
    }

    #[test]
    fn authorization_decision_log_summary_counts_outcomes_subjects_and_missing_capabilities() {
        let principal = AgentId::trusted("agent:lighting-planner");
        let grant = CapabilityGrant::for_tool(
            CapabilityGrantId::trusted("grant-command"),
            principal.clone(),
            SmartHomeTool::Command,
            "chief-of-staff",
            1_000,
        );
        let allowed =
            AuthorizationDecision::for_tool(principal, SmartHomeTool::Command, [&grant], 1_500);

        let denied_principal = AgentId::trusted("agent:security-agent");
        let low_risk_lock_grant = CapabilityGrant::for_entity_capability(
            CapabilityGrantId::trusted("grant-lock-low"),
            denied_principal.clone(),
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
        let denied = AuthorizationDecision::for_command(
            denied_principal,
            &command,
            [&low_risk_lock_grant],
            1_500,
        );
        let decisions = vec![allowed, denied];
        let summary = AuthorizationDecisionLogSummary::from_decisions(&decisions);

        assert_eq!(summary.total_decisions, 2);
        assert_eq!(summary.allowed_decisions, 1);
        assert_eq!(summary.denied_decisions, 1);
        assert_eq!(summary.tool_decisions, 1);
        assert_eq!(summary.command_decisions, 1);
        assert_eq!(summary.read_only_tier_decisions, 0);
        assert_eq!(summary.low_risk_tier_decisions, 1);
        assert_eq!(summary.human_approval_tier_decisions, 0);
        assert_eq!(summary.high_risk_tier_decisions, 1);
        assert_eq!(summary.decisions_with_missing_capabilities, 1);
        assert_eq!(summary.total_required_capabilities, 2);
        assert_eq!(summary.total_matched_grants, 1);
        assert_eq!(summary.total_missing_capabilities, 1);
        assert!(!summary.is_empty());
        assert!(summary.has_denials());
        assert!(summary.has_missing_capabilities());
        assert_eq!(summary.approval_gated_decisions(), 1);

        let projected = decisions
            .iter()
            .map(AuthorizationDecision::summary)
            .collect::<Vec<_>>();
        assert_eq!(
            AuthorizationDecisionLogSummary::from_summaries(&projected),
            summary
        );
        assert!(AuthorizationDecisionLogSummary::empty().is_empty());
    }
}
