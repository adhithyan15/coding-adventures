//! Authorized read-only SNMPv2c telemetry integration for D23.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroizing;
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind, VaultRef,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use snmp_protocol::{
    decode_get_response, encode_get_request, GetRequest, ObjectIdentifier, SnmpError, SnmpValue,
    MAX_COMMUNITY_BYTES, MAX_DATAGRAM_BYTES,
};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use udp_client::{UdpClient, UdpError, UdpOptions};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "snmp";
pub const PROTOCOL_ID: &str = "snmp_v2c";
pub const DEFAULT_PORT: u16 = 161;
pub const MAX_PROFILE_POINTS: usize = 32;
pub const MAX_TEXT_BYTES: usize = 512;
const MAX_EXACT_F64_INTEGER: i64 = 9_007_199_254_740_991;

#[derive(Debug)]
pub enum SnmpIntegrationError {
    Validation(String),
    Protocol(SnmpError),
    Udp(UdpError),
    UnexpectedSource {
        expected: SocketAddr,
        actual: SocketAddr,
    },
    ValueType {
        point_id: String,
        expected: &'static str,
        actual: &'static str,
    },
    InvalidValue {
        point_id: String,
        message: String,
    },
    Runtime(RuntimeError),
}

impl fmt::Display for SnmpIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid SNMP input: {message}"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Udp(error) => error.fmt(formatter),
            Self::UnexpectedSource { expected, actual } => write!(
                formatter,
                "SNMP response source mismatch: expected {expected}, got {actual}"
            ),
            Self::ValueType {
                point_id,
                expected,
                actual,
            } => write!(
                formatter,
                "SNMP point `{point_id}` expected {expected}, got {actual}"
            ),
            Self::InvalidValue { point_id, message } => {
                write!(formatter, "invalid SNMP value for `{point_id}`: {message}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SnmpIntegrationError {}

impl From<SnmpError> for SnmpIntegrationError {
    fn from(error: SnmpError) -> Self {
        Self::Protocol(error)
    }
}

impl From<UdpError> for SnmpIntegrationError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<RuntimeError> for SnmpIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct SnmpCommunity {
    bytes: Zeroizing<Vec<u8>>,
}

impl SnmpCommunity {
    pub fn new(bytes: Vec<u8>) -> Result<Self, SnmpIntegrationError> {
        if bytes.is_empty() || bytes.len() > MAX_COMMUNITY_BYTES {
            return Err(SnmpIntegrationError::Validation(format!(
                "community must contain between 1 and {MAX_COMMUNITY_BYTES} bytes"
            )));
        }
        Ok(Self {
            bytes: Zeroizing::new(bytes),
        })
    }

    pub fn from_text(value: impl Into<String>) -> Result<Self, SnmpIntegrationError> {
        Self::new(value.into().into_bytes())
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn expose(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

impl fmt::Debug for SnmpCommunity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SnmpCommunity")
            .field("bytes", &"[REDACTED]")
            .field("length", &self.bytes.len())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnmpValueCodec {
    Integer,
    IntegerBoolean { true_value: i64, false_value: i64 },
    Utf8Text,
    ObjectIdentifier,
    IpAddress,
    Counter32,
    Gauge32,
    TimeTicksSeconds,
    Counter64Decimal,
}

impl SnmpValueCodec {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::IntegerBoolean { .. } => "integer_boolean",
            Self::Utf8Text => "utf8_text",
            Self::ObjectIdentifier => "object_identifier",
            Self::IpAddress => "ip_address",
            Self::Counter32 => "counter32",
            Self::Gauge32 => "gauge32",
            Self::TimeTicksSeconds => "timeticks_seconds",
            Self::Counter64Decimal => "counter64_decimal",
        }
    }

    fn validate(&self) -> Result<(), SnmpIntegrationError> {
        if let Self::IntegerBoolean {
            true_value,
            false_value,
        } = self
        {
            if true_value == false_value {
                return Err(SnmpIntegrationError::Validation(
                    "boolean true and false integer values must differ".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnmpPoint {
    pub id: String,
    pub name: String,
    pub oid: ObjectIdentifier,
    pub codec: SnmpValueCodec,
    pub unit: String,
}

impl SnmpPoint {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        oid: ObjectIdentifier,
        codec: SnmpValueCodec,
        unit: impl Into<String>,
    ) -> Result<Self, SnmpIntegrationError> {
        let point = Self {
            id: id.into(),
            name: name.into(),
            oid,
            codec,
            unit: unit.into(),
        };
        point.validate()?;
        Ok(point)
    }

    fn validate(&self) -> Result<(), SnmpIntegrationError> {
        if stable_component(&self.id).is_empty() {
            return Err(SnmpIntegrationError::Validation(
                "point id must contain an ASCII letter or digit".to_string(),
            ));
        }
        validate_bounded_text(&self.name, "point name", 128)?;
        validate_bounded_text(&self.unit, "point unit", 64)?;
        self.codec.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnmpDeviceConfig {
    pub bridge_id: BridgeId,
    pub auth_ref: VaultRef,
    pub endpoint: SocketAddr,
    pub device_key: String,
    pub display_name: String,
    pub manufacturer: String,
    pub model: String,
    pub timeout: Duration,
    pub points: Vec<SnmpPoint>,
}

impl SnmpDeviceConfig {
    pub fn new(
        bridge_id: BridgeId,
        auth_ref: VaultRef,
        endpoint: SocketAddr,
        device_key: impl Into<String>,
        points: Vec<SnmpPoint>,
    ) -> Result<Self, SnmpIntegrationError> {
        let device_key = device_key.into();
        let config = Self {
            bridge_id,
            auth_ref,
            endpoint,
            display_name: device_key.clone(),
            device_key,
            manufacturer: "SNMP".to_string(),
            model: "Managed device".to_string(),
            timeout: Duration::from_secs(3),
            points,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = display_name.into();
        self
    }

    pub fn with_device_identity(
        mut self,
        manufacturer: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        self.manufacturer = manufacturer.into();
        self.model = model.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn validate(&self) -> Result<(), SnmpIntegrationError> {
        validate_local_endpoint(self.endpoint)?;
        validate_bounded_text(self.auth_ref.as_str(), "Vault reference", 256)?;
        validate_bounded_text(&self.device_key, "device key", 128)?;
        if stable_component(&self.device_key).is_empty() {
            return Err(SnmpIntegrationError::Validation(
                "device key must contain an ASCII letter or digit".to_string(),
            ));
        }
        validate_bounded_text(&self.display_name, "display name", 128)?;
        validate_bounded_text(&self.manufacturer, "manufacturer", 128)?;
        validate_bounded_text(&self.model, "model", 128)?;
        if self.timeout.is_zero() {
            return Err(SnmpIntegrationError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if self.points.is_empty() || self.points.len() > MAX_PROFILE_POINTS {
            return Err(SnmpIntegrationError::Validation(format!(
                "profile must contain between 1 and {MAX_PROFILE_POINTS} points"
            )));
        }
        let mut ids = self
            .points
            .iter()
            .map(|point| {
                point.validate()?;
                Ok(stable_component(&point.id))
            })
            .collect::<Result<Vec<_>, SnmpIntegrationError>>()?;
        ids.sort();
        if ids.windows(2).any(|window| window[0] == window[1]) {
            return Err(SnmpIntegrationError::Validation(
                "point ids must be unique after normalization".to_string(),
            ));
        }
        let mut oids = self
            .points
            .iter()
            .map(|point| &point.oid)
            .collect::<Vec<_>>();
        oids.sort();
        if oids.windows(2).any(|window| window[0] == window[1]) {
            return Err(SnmpIntegrationError::Validation(
                "point OIDs must be unique".to_string(),
            ));
        }
        Ok(())
    }

    pub fn endpoint_uri(&self) -> String {
        format!("snmp://{}", self.endpoint)
    }
}

pub struct SnmpExchangePlan<'a> {
    pub endpoint: SocketAddr,
    pub timeout: Duration,
    pub request: &'a [u8],
}

pub trait SnmpTransport {
    fn exchange(&mut self, plan: SnmpExchangePlan<'_>) -> Result<Vec<u8>, SnmpIntegrationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpSnmpTransport;

impl SnmpTransport for UdpSnmpTransport {
    fn exchange(&mut self, plan: SnmpExchangePlan<'_>) -> Result<Vec<u8>, SnmpIntegrationError> {
        let mut client = UdpClient::bind(UdpOptions {
            bind_addr: Some(unspecified_for(plan.endpoint)),
            max_datagram_size: MAX_DATAGRAM_BYTES,
            read_timeout: Some(plan.timeout),
            write_timeout: Some(plan.timeout),
        })?;
        client.connect(plan.endpoint)?;
        client.send(plan.request)?;
        let response = client.recv_from()?;
        if response.source != plan.endpoint {
            return Err(SnmpIntegrationError::UnexpectedSource {
                expected: plan.endpoint,
                actual: response.source,
            });
        }
        Ok(response.payload)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SnmpScalar {
    Number(f64),
    Boolean(bool),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnmpMeasurement {
    pub point_id: String,
    pub name: String,
    pub oid: ObjectIdentifier,
    pub codec: SnmpValueCodec,
    pub unit: String,
    pub value: SnmpScalar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SnmpSnapshot {
    pub endpoint: SocketAddr,
    pub measurements: Vec<SnmpMeasurement>,
}

pub struct SnmpClient<T> {
    config: SnmpDeviceConfig,
    community: SnmpCommunity,
    transport: T,
    next_request_id: i32,
}

impl<T: SnmpTransport> SnmpClient<T> {
    pub fn new(config: SnmpDeviceConfig, community: SnmpCommunity, transport: T) -> Self {
        Self {
            config,
            community,
            transport,
            next_request_id: 1,
        }
    }

    pub fn config(&self) -> &SnmpDeviceConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<SnmpSnapshot, SnmpIntegrationError> {
        self.config.validate()?;
        let request = GetRequest::new(
            self.next_request_id,
            self.config
                .points
                .iter()
                .map(|point| point.oid.clone())
                .collect(),
        )?;
        self.next_request_id = self.next_request_id.checked_add(1).unwrap_or(1);
        let encoded = Zeroizing::new(encode_get_request(self.community.expose(), &request)?);
        let response = Zeroizing::new(self.transport.exchange(SnmpExchangePlan {
            endpoint: self.config.endpoint,
            timeout: self.config.timeout,
            request: encoded.as_slice(),
        })?);
        let response = decode_get_response(response.as_slice(), self.community.expose(), &request)?;
        let measurements = self
            .config
            .points
            .iter()
            .zip(response.variable_bindings)
            .map(|(point, binding)| {
                Ok(SnmpMeasurement {
                    point_id: point.id.clone(),
                    name: point.name.clone(),
                    oid: binding.oid,
                    codec: point.codec.clone(),
                    unit: point.unit.clone(),
                    value: decode_scalar(point, binding.value)?,
                })
            })
            .collect::<Result<Vec<_>, SnmpIntegrationError>>()?;
        Ok(SnmpSnapshot {
            endpoint: self.config.endpoint,
            measurements,
        })
    }
}

impl<T> fmt::Debug for SnmpClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SnmpClient")
            .field("config", &self.config)
            .field("community", &self.community)
            .field("next_request_id", &self.next_request_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledSnmpDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

pub struct SnmpRuntimeIntegration<T> {
    client: SnmpClient<T>,
}

impl<T: SnmpTransport> SnmpRuntimeIntegration<T> {
    pub fn new(client: SnmpClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledSnmpDevice, SnmpIntegrationError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &SnmpSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledSnmpDevice, SnmpIntegrationError> {
        if snapshot.endpoint != self.client.config.endpoint
            || snapshot.measurements.len() != self.client.config.points.len()
            || !snapshot
                .measurements
                .iter()
                .zip(&self.client.config.points)
                .all(|(measurement, point)| measurement_matches_point(measurement, point))
        {
            return Err(SnmpIntegrationError::Validation(
                "snapshot does not match the configured endpoint and OID profile".to_string(),
            ));
        }

        let endpoint_uri = self.client.config.endpoint_uri();
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!(
            "snmp:{}",
            stable_component(&self.client.config.device_key)
        ));
        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanUdp,
        );
        bridge.address = Some(endpoint_uri.clone());
        bridge.hardware_model = Some("SNMPv2c endpoint".to_string());
        bridge.auth_ref = Some(self.client.config.auth_ref.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier(
            "udp_endpoint",
            &snapshot.endpoint.to_string(),
        )?];
        bridge.metadata = vec![
            Metadata::new("snmp.version", "2c"),
            Metadata::new("snmp.transport", "udp"),
            Metadata::new("snmp.access", "read_only"),
            Metadata::new("snmp.security", "community_local_unicast"),
        ];
        runtime.upsert_bridge(bridge)?;

        let entities = snapshot
            .measurements
            .iter()
            .map(|measurement| {
                let entity_id = EntityId::trusted(format!(
                    "{}:sensor:{}",
                    device_id.as_str(),
                    stable_component(&measurement.point_id)
                ));
                Entity {
                    entity_id: entity_id.clone(),
                    device_id: device_id.clone(),
                    kind: EntityKind::Sensor,
                    name: format!("{} {}", self.client.config.display_name, measurement.name),
                    capabilities: vec![Capability::new(
                        CapabilityId::trusted("sensor.measurement"),
                        CapabilityMode::Observe,
                        ValueKind::Object,
                    )],
                    state: Some(confirmed_state(
                        entity_id,
                        measurement_value(measurement),
                        observed_at_ms,
                    )),
                    metadata: vec![
                        Metadata::new("snmp.point_id", measurement.point_id.clone()),
                        Metadata::new("snmp.oid", measurement.oid.to_string()),
                        Metadata::new("snmp.codec", measurement.codec.as_str()),
                        Metadata::new("snmp.unit", measurement.unit.clone()),
                    ],
                }
            })
            .collect::<Vec<_>>();
        let entity_ids = entities
            .iter()
            .map(|entity| entity.entity_id.clone())
            .collect::<Vec<_>>();
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: bridge_id.clone(),
            manufacturer: self.client.config.manufacturer.clone(),
            model: self.client.config.model.clone(),
            name: self.client.config.display_name.clone(),
            serial: None,
            firmware_version: None,
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier(
                "endpoint_device",
                &format!("{}#{}", snapshot.endpoint, self.client.config.device_key),
            )?],
            health: Health::Online,
            metadata: vec![
                Metadata::new("snmp.endpoint", endpoint_uri),
                Metadata::new("snmp.profile_points", entity_ids.len().to_string()),
            ],
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledSnmpDevice {
            bridge_id,
            device_id,
            entity_ids,
        })
    }
}

fn decode_scalar(point: &SnmpPoint, value: SnmpValue) -> Result<SnmpScalar, SnmpIntegrationError> {
    match (&point.codec, value) {
        (SnmpValueCodec::Integer, SnmpValue::Integer(value)) => {
            exact_integer(point, value).map(SnmpScalar::Number)
        }
        (
            SnmpValueCodec::IntegerBoolean {
                true_value,
                false_value,
            },
            SnmpValue::Integer(value),
        ) if value == *true_value => Ok(SnmpScalar::Boolean(true)),
        (
            SnmpValueCodec::IntegerBoolean {
                true_value: _,
                false_value,
            },
            SnmpValue::Integer(value),
        ) if value == *false_value => Ok(SnmpScalar::Boolean(false)),
        (SnmpValueCodec::IntegerBoolean { .. }, SnmpValue::Integer(_)) => Err(invalid_value(
            point,
            "integer does not match either configured boolean value",
        )),
        (SnmpValueCodec::Utf8Text, SnmpValue::OctetString(value)) => {
            let value = std::str::from_utf8(&value).map_err(|error| invalid_value(point, error))?;
            safe_text(point, value).map(SnmpScalar::Text)
        }
        (SnmpValueCodec::ObjectIdentifier, SnmpValue::ObjectIdentifier(value)) => {
            Ok(SnmpScalar::Text(value.to_string()))
        }
        (SnmpValueCodec::IpAddress, SnmpValue::IpAddress(value)) => {
            Ok(SnmpScalar::Text(Ipv4Addr::from(value).to_string()))
        }
        (SnmpValueCodec::Counter32, SnmpValue::Counter32(value)) => {
            Ok(SnmpScalar::Number(f64::from(value)))
        }
        (SnmpValueCodec::Gauge32, SnmpValue::Gauge32(value)) => {
            Ok(SnmpScalar::Number(f64::from(value)))
        }
        (SnmpValueCodec::TimeTicksSeconds, SnmpValue::TimeTicks(value)) => {
            Ok(SnmpScalar::Number(f64::from(value) / 100.0))
        }
        (SnmpValueCodec::Counter64Decimal, SnmpValue::Counter64(value)) => {
            Ok(SnmpScalar::Text(value.to_string()))
        }
        (codec, value) => Err(SnmpIntegrationError::ValueType {
            point_id: point.id.clone(),
            expected: codec.as_str(),
            actual: value_kind(&value),
        }),
    }
}

fn exact_integer(point: &SnmpPoint, value: i64) -> Result<f64, SnmpIntegrationError> {
    if (-MAX_EXACT_F64_INTEGER..=MAX_EXACT_F64_INTEGER).contains(&value) {
        Ok(value as f64)
    } else {
        Err(invalid_value(
            point,
            "integer cannot be represented exactly by the normalized number type",
        ))
    }
}

fn safe_text(point: &SnmpPoint, value: &str) -> Result<String, SnmpIntegrationError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value.chars().any(|character| character.is_control())
    {
        return Err(invalid_value(
            point,
            format!("text must be control-free and between 1 and {MAX_TEXT_BYTES} bytes"),
        ));
    }
    Ok(value.to_string())
}

fn value_kind(value: &SnmpValue) -> &'static str {
    match value {
        SnmpValue::Integer(_) => "integer",
        SnmpValue::OctetString(_) => "octet_string",
        SnmpValue::ObjectIdentifier(_) => "object_identifier",
        SnmpValue::IpAddress(_) => "ip_address",
        SnmpValue::Counter32(_) => "counter32",
        SnmpValue::Gauge32(_) => "gauge32",
        SnmpValue::TimeTicks(_) => "timeticks",
        SnmpValue::Counter64(_) => "counter64",
    }
}

fn invalid_value(point: &SnmpPoint, message: impl fmt::Display) -> SnmpIntegrationError {
    SnmpIntegrationError::InvalidValue {
        point_id: point.id.clone(),
        message: message.to_string(),
    }
}

fn measurement_matches_point(measurement: &SnmpMeasurement, point: &SnmpPoint) -> bool {
    measurement.point_id == point.id
        && measurement.name == point.name
        && measurement.oid == point.oid
        && measurement.codec == point.codec
        && measurement.unit == point.unit
        && match &measurement.value {
            SnmpScalar::Number(value) => value.is_finite(),
            SnmpScalar::Boolean(_) => true,
            SnmpScalar::Text(value) => {
                !value.is_empty()
                    && value.len() <= MAX_TEXT_BYTES
                    && !value.chars().any(|character| character.is_control())
            }
        }
}

fn measurement_value(measurement: &SnmpMeasurement) -> Value {
    let value = match &measurement.value {
        SnmpScalar::Number(value) => Value::Number(*value),
        SnmpScalar::Boolean(value) => Value::Bool(*value),
        SnmpScalar::Text(value) => Value::Text(value.clone()),
    };
    Value::Object(vec![
        ("value".to_string(), value),
        ("unit".to_string(), Value::Text(measurement.unit.clone())),
    ])
}

fn confirmed_state(entity_id: EntityId, value: Value, observed_at_ms: u64) -> StateSnapshot {
    StateSnapshot {
        entity_id,
        value,
        source: StateSource::Poll,
        observed_at_ms,
        received_at_ms: observed_at_ms,
        expires_at_ms: None,
        confidence: StateConfidence::Confirmed,
    }
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), SnmpIntegrationError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(SnmpIntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ))
    }
}

fn protocol_identifier(
    kind: &str,
    value: &str,
) -> Result<ProtocolIdentifier, SnmpIntegrationError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| SnmpIntegrationError::Validation(error.to_string()))
}

fn validate_local_endpoint(endpoint: SocketAddr) -> Result<(), SnmpIntegrationError> {
    if endpoint.port() == 0 || !is_local_unicast(endpoint.ip()) {
        return Err(SnmpIntegrationError::Validation(
            "endpoint must be an explicit private, link-local, or loopback unicast address with a non-zero port"
                .to_string(),
        ));
    }
    Ok(())
}

fn is_local_unicast(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !address.is_unspecified()
                && !address.is_multicast()
                && (address.is_private() || address.is_link_local() || address.is_loopback())
        }
        IpAddr::V6(address) => {
            !address.is_unspecified()
                && !address.is_multicast()
                && (address.is_loopback()
                    || address.is_unique_local()
                    || is_ipv6_link_local(address))
        }
    }
}

fn is_ipv6_link_local(address: Ipv6Addr) -> bool {
    (address.segments()[0] & 0xffc0) == 0xfe80
}

fn unspecified_for(endpoint: SocketAddr) -> SocketAddr {
    match endpoint {
        SocketAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        SocketAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
}

fn validate_bounded_text(
    value: &str,
    field: &str,
    maximum: usize,
) -> Result<(), SnmpIntegrationError> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.len() > maximum
        || value.chars().any(|character| character.is_control())
    {
        return Err(SnmpIntegrationError::Validation(format!(
            "{field} must be trimmed, control-free, and between 1 and {maximum} bytes"
        )));
    }
    Ok(())
}

fn stable_component(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            output.push(character);
            separator = false;
        } else if !output.is_empty() && !separator {
            output.push('-');
            separator = true;
        }
    }
    output.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::UdpSocket;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    const TAG_INTEGER: u8 = 0x02;
    const TAG_OCTET_STRING: u8 = 0x04;
    const TAG_OBJECT_IDENTIFIER: u8 = 0x06;
    const TAG_SEQUENCE: u8 = 0x30;
    const TAG_TIME_TICKS: u8 = 0x43;
    const TAG_GET_RESPONSE: u8 = 0xa2;

    fn oid(value: &str) -> ObjectIdentifier {
        ObjectIdentifier::parse(value).unwrap()
    }

    fn uptime_point() -> SnmpPoint {
        SnmpPoint::new(
            "uptime",
            "Uptime",
            oid("1.3.6.1.2.1.1.3.0"),
            SnmpValueCodec::TimeTicksSeconds,
            "s",
        )
        .unwrap()
    }

    fn name_point() -> SnmpPoint {
        SnmpPoint::new(
            "name",
            "Name",
            oid("1.3.6.1.2.1.1.5.0"),
            SnmpValueCodec::Utf8Text,
            "text",
        )
        .unwrap()
    }

    fn config(endpoint: SocketAddr) -> SnmpDeviceConfig {
        SnmpDeviceConfig::new(
            BridgeId::trusted("snmp.test"),
            VaultRef::trusted("vault:snmp/test"),
            endpoint,
            "ups-01",
            vec![uptime_point(), name_point()],
        )
        .unwrap()
        .with_display_name("Rack UPS")
        .with_device_identity("Acme", "UPS-1")
    }

    fn community() -> SnmpCommunity {
        SnmpCommunity::from_text("monitor-only").unwrap()
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:snmp-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    fn push_tlv(tag: u8, value: &[u8], output: &mut Vec<u8>) {
        output.push(tag);
        if value.len() < 128 {
            output.push(value.len() as u8);
        } else {
            output.push(0x82);
            output.extend_from_slice(&(value.len() as u16).to_be_bytes());
        }
        output.extend_from_slice(value);
    }

    fn encode_oid(oid: &ObjectIdentifier) -> Vec<u8> {
        fn subidentifier(mut value: u64, output: &mut Vec<u8>) {
            let mut bytes = [0_u8; 10];
            let mut index = bytes.len();
            index -= 1;
            bytes[index] = (value & 0x7f) as u8;
            value >>= 7;
            while value > 0 {
                index -= 1;
                bytes[index] = ((value & 0x7f) as u8) | 0x80;
                value >>= 7;
            }
            output.extend_from_slice(&bytes[index..]);
        }
        let arcs = oid.arcs();
        let mut bytes = Vec::new();
        subidentifier(arcs[0] as u64 * 40 + arcs[1] as u64, &mut bytes);
        for arc in &arcs[2..] {
            subidentifier(*arc as u64, &mut bytes);
        }
        bytes
    }

    fn response(values: &[(ObjectIdentifier, u8, Vec<u8>)]) -> Vec<u8> {
        let mut bindings = Vec::new();
        for (oid, tag, value) in values {
            let mut binding = Vec::new();
            push_tlv(TAG_OBJECT_IDENTIFIER, &encode_oid(oid), &mut binding);
            push_tlv(*tag, value, &mut binding);
            push_tlv(TAG_SEQUENCE, &binding, &mut bindings);
        }
        let mut pdu = Vec::new();
        push_tlv(TAG_INTEGER, &[1], &mut pdu);
        push_tlv(TAG_INTEGER, &[0], &mut pdu);
        push_tlv(TAG_INTEGER, &[0], &mut pdu);
        push_tlv(TAG_SEQUENCE, &bindings, &mut pdu);
        let mut message = Vec::new();
        push_tlv(TAG_INTEGER, &[1], &mut message);
        push_tlv(TAG_OCTET_STRING, b"monitor-only", &mut message);
        push_tlv(TAG_GET_RESPONSE, &pdu, &mut message);
        let mut encoded = Vec::new();
        push_tlv(TAG_SEQUENCE, &message, &mut encoded);
        encoded
    }

    #[derive(Debug)]
    struct ScriptedTransport {
        calls: Arc<AtomicUsize>,
        response: Vec<u8>,
    }

    impl SnmpTransport for ScriptedTransport {
        fn exchange(
            &mut self,
            _plan: SnmpExchangePlan<'_>,
        ) -> Result<Vec<u8>, SnmpIntegrationError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.response.clone())
        }
    }

    #[test]
    fn live_udp_poll_installs_normalized_sensors() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let endpoint = server.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let mut request = [0_u8; MAX_DATAGRAM_BYTES];
            let (length, source) = server.recv_from(&mut request).unwrap();
            assert!(request[..length]
                .windows(b"monitor-only".len())
                .any(|window| window == b"monitor-only"));
            let response = response(&[
                (oid("1.3.6.1.2.1.1.3.0"), TAG_TIME_TICKS, vec![0x30, 0x39]),
                (
                    oid("1.3.6.1.2.1.1.5.0"),
                    TAG_OCTET_STRING,
                    b"ups-a".to_vec(),
                ),
            ]);
            server.send_to(&response, source).unwrap();
        });

        let principal = AgentId::trusted("agent:snmp-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let client = SnmpClient::new(config(endpoint), community(), UdpSnmpTransport);
        let mut integration = SnmpRuntimeIntegration::new(client);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();

        assert_eq!(installed.entity_ids.len(), 2);
        let bridge = runtime.registry().bridge(&installed.bridge_id).unwrap();
        assert_eq!(
            bridge.auth_ref.as_ref().map(VaultRef::as_str),
            Some("vault:snmp/test")
        );
        let uptime = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:uptime",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(
            uptime.state.as_ref().unwrap().value,
            Value::Object(vec![
                ("value".to_string(), Value::Number(123.45)),
                ("unit".to_string(), Value::Text("s".to_string())),
            ])
        );
        let debug = format!("{:?}", community());
        assert!(debug.contains("REDACTED"));
        assert!(!debug.contains("monitor-only"));
        let runtime_text = format!("{runtime:?}");
        assert!(!runtime_text.contains("monitor-only"));
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let endpoint = "127.0.0.1:161".parse().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let client = SnmpClient::new(
            config(endpoint),
            community(),
            ScriptedTransport {
                calls: Arc::clone(&calls),
                response: Vec::new(),
            },
        );
        let mut integration = SnmpRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut runtime,
                AgentId::trusted("agent:denied"),
                5_000
            ),
            Err(SnmpIntegrationError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn rejects_public_endpoints_duplicate_profiles_and_empty_communities() {
        assert!(SnmpDeviceConfig::new(
            BridgeId::trusted("snmp.public"),
            VaultRef::trusted("vault:snmp/public"),
            "8.8.8.8:161".parse().unwrap(),
            "device",
            vec![uptime_point()],
        )
        .is_err());
        assert!(SnmpDeviceConfig::new(
            BridgeId::trusted("snmp.duplicate"),
            VaultRef::trusted("vault:snmp/duplicate"),
            "127.0.0.1:161".parse().unwrap(),
            "device",
            vec![uptime_point(), uptime_point()],
        )
        .is_err());
        assert!(SnmpCommunity::new(Vec::new()).is_err());
        assert!(SnmpCommunity::new(vec![b'x'; MAX_COMMUNITY_BYTES + 1]).is_err());
    }

    #[test]
    fn strict_codecs_reject_wrong_types_and_inexact_integers() {
        let integer = SnmpPoint::new(
            "load",
            "Load",
            oid("1.3.6.1.4.1.1"),
            SnmpValueCodec::Integer,
            "count",
        )
        .unwrap();
        assert!(matches!(
            decode_scalar(&integer, SnmpValue::OctetString(b"1".to_vec())),
            Err(SnmpIntegrationError::ValueType { .. })
        ));
        assert!(matches!(
            decode_scalar(&integer, SnmpValue::Integer(i64::MAX)),
            Err(SnmpIntegrationError::InvalidValue { .. })
        ));
        let boolean = SnmpPoint::new(
            "online",
            "Online",
            oid("1.3.6.1.4.1.2"),
            SnmpValueCodec::IntegerBoolean {
                true_value: 1,
                false_value: 2,
            },
            "bool",
        )
        .unwrap();
        assert!(decode_scalar(&boolean, SnmpValue::Integer(3)).is_err());
    }

    #[test]
    fn strict_codecs_project_supported_scalars_without_precision_loss() {
        let cases = [
            (
                SnmpValueCodec::IntegerBoolean {
                    true_value: 1,
                    false_value: 2,
                },
                SnmpValue::Integer(1),
                SnmpScalar::Boolean(true),
            ),
            (
                SnmpValueCodec::Counter32,
                SnmpValue::Counter32(u32::MAX),
                SnmpScalar::Number(f64::from(u32::MAX)),
            ),
            (
                SnmpValueCodec::Gauge32,
                SnmpValue::Gauge32(900),
                SnmpScalar::Number(900.0),
            ),
            (
                SnmpValueCodec::Counter64Decimal,
                SnmpValue::Counter64(u64::MAX),
                SnmpScalar::Text(u64::MAX.to_string()),
            ),
            (
                SnmpValueCodec::IpAddress,
                SnmpValue::IpAddress([10, 0, 0, 5]),
                SnmpScalar::Text("10.0.0.5".to_string()),
            ),
            (
                SnmpValueCodec::ObjectIdentifier,
                SnmpValue::ObjectIdentifier(oid("1.3.6.1.4.1.9")),
                SnmpScalar::Text("1.3.6.1.4.1.9".to_string()),
            ),
        ];
        for (index, (codec, value, expected)) in cases.into_iter().enumerate() {
            let point = SnmpPoint::new(
                format!("point-{index}"),
                format!("Point {index}"),
                oid(&format!("1.3.6.1.4.1.9.{index}")),
                codec,
                "value",
            )
            .unwrap();
            assert_eq!(decode_scalar(&point, value).unwrap(), expected);
        }
    }

    #[test]
    fn rejects_mismatched_snapshot_before_runtime_mutation() {
        let endpoint = "127.0.0.1:161".parse().unwrap();
        let client = SnmpClient::new(
            config(endpoint),
            community(),
            ScriptedTransport {
                calls: Arc::new(AtomicUsize::new(0)),
                response: Vec::new(),
            },
        );
        let integration = SnmpRuntimeIntegration::new(client);
        let snapshot = SnmpSnapshot {
            endpoint,
            measurements: vec![SnmpMeasurement {
                point_id: "wrong".to_string(),
                name: "Wrong".to_string(),
                oid: oid("1.3.6.1.2.1.1.3.0"),
                codec: SnmpValueCodec::TimeTicksSeconds,
                unit: "s".to_string(),
                value: SnmpScalar::Number(1.0),
            }],
        };
        let mut runtime = SmartHomeRuntime::new();
        assert!(integration
            .install_snapshot(&mut runtime, &snapshot, 1_000)
            .is_err());
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("snmp.test"))
            .is_none());
    }

    #[test]
    fn scripted_response_requires_exact_profile_types() {
        let endpoint = "127.0.0.1:161".parse().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let response = response(&[
            (oid("1.3.6.1.2.1.1.3.0"), TAG_INTEGER, vec![1]),
            (
                oid("1.3.6.1.2.1.1.5.0"),
                TAG_OCTET_STRING,
                b"ups-a".to_vec(),
            ),
        ]);
        let mut client = SnmpClient::new(
            config(endpoint),
            community(),
            ScriptedTransport {
                calls: Arc::clone(&calls),
                response,
            },
        );
        assert!(matches!(
            client.inspect(),
            Err(SnmpIntegrationError::ValueType { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
