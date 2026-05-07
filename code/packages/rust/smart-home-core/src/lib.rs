//! Repository-owned smart-home vocabulary shared by integrations, tools, and
//! Chief of Staff agents.
//!
//! The types in this crate are intentionally protocol-neutral. A Hue light,
//! Zigbee endpoint, Z-Wave node value, Thread/Matter endpoint, or MQTT device
//! can all be projected into the same bridge/device/entity/event/command model.

#![forbid(unsafe_code)]

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmartHomeError {
    EmptyIdentifier { kind: &'static str },
    InvalidPercentage { value: u16 },
    MissingCapability { command_type: CommandType },
}

impl fmt::Display for SmartHomeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier { kind } => write!(f, "{kind} must not be empty"),
            Self::InvalidPercentage { value } => {
                write!(f, "percentage value {value} is outside 0..=100")
            }
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

    pub fn light_color_temperature() -> Self {
        let mut capability = Self::new(
            CapabilityId::trusted("light.color_temperature"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Integer,
        );
        capability.unit = Some("mirek".to_string());
        capability
    }

    pub fn sensor_occupancy() -> Self {
        Self::new(
            CapabilityId::trusted("sensor.occupancy"),
            CapabilityMode::Observe,
            ValueKind::Boolean,
        )
    }

    pub fn input_button() -> Self {
        Self::new(
            CapabilityId::trusted("input.button"),
            CapabilityMode::Observe,
            ValueKind::Text,
        )
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
    ]
    .into_iter()
    .map(SmartHomeTool::descriptor)
    .collect()
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
    fn tool_catalog_exposes_model_facing_smart_home_surface() {
        let catalog = smart_home_tool_catalog();
        let command = catalog
            .iter()
            .find(|tool| tool.tool_id == "smart_home.command")
            .unwrap();

        assert_eq!(catalog.len(), 9);
        assert_eq!(command.side_effects, ToolSideEffects::External);
        assert_eq!(
            command.required_capabilities,
            vec![CapabilityId::trusted("smart_home.command.light")]
        );
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
