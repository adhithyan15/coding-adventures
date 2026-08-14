//! Authorized read-only Network UPS Tools telemetry integration for D23.

#![forbid(unsafe_code)]

use nut_protocol::{
    decode_list_var_response, encode_list_var_request, list_var_end_line, validate_variable_name,
    ListVarResponse, NutProtocolError, MAX_LINE_BYTES, MAX_RESPONSE_BYTES,
};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tcp_client::{connect, ConnectOptions, TcpError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "nut";
pub const PROTOCOL_ID: &str = "nut_upsd";
pub const DEFAULT_PORT: u16 = nut_protocol::DEFAULT_PORT;
pub const MAX_PROFILE_POINTS: usize = 32;
pub const MAX_TEXT_BYTES: usize = 512;
const _: () = assert!(MAX_PROFILE_POINTS <= nut_protocol::MAX_VARIABLES);

#[derive(Debug)]
pub enum NutIntegrationError {
    Validation(String),
    Protocol(NutProtocolError),
    Tcp(TcpError),
    UnexpectedPeer {
        expected: SocketAddr,
        actual: SocketAddr,
    },
    MissingVariable(String),
    InvalidValue {
        point_id: String,
        message: String,
    },
    Runtime(RuntimeError),
}

impl fmt::Display for NutIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid NUT input: {message}"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Tcp(error) => error.fmt(formatter),
            Self::UnexpectedPeer { expected, actual } => write!(
                formatter,
                "NUT TCP peer mismatch: expected {expected}, got {actual}"
            ),
            Self::MissingVariable(name) => write!(formatter, "NUT variable `{name}` is missing"),
            Self::InvalidValue { point_id, message } => {
                write!(formatter, "invalid NUT value for `{point_id}`: {message}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for NutIntegrationError {}

impl From<NutProtocolError> for NutIntegrationError {
    fn from(error: NutProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<TcpError> for NutIntegrationError {
    fn from(error: TcpError) -> Self {
        Self::Tcp(error)
    }
}

impl From<RuntimeError> for NutIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NutValueCodec {
    Decimal,
    Boolean {
        true_value: String,
        false_value: String,
    },
    Text,
}

impl NutValueCodec {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Decimal => "decimal",
            Self::Boolean { .. } => "boolean",
            Self::Text => "text",
        }
    }

    fn validate(&self) -> Result<(), NutIntegrationError> {
        if let Self::Boolean {
            true_value,
            false_value,
        } = self
        {
            validate_safe_text(true_value, "boolean true value", 64)?;
            validate_safe_text(false_value, "boolean false value", 64)?;
            if true_value == false_value {
                return Err(NutIntegrationError::Validation(
                    "boolean true and false values must differ".to_string(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NutPoint {
    pub id: String,
    pub name: String,
    pub variable: String,
    pub codec: NutValueCodec,
    pub unit: String,
}

impl NutPoint {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        variable: impl Into<String>,
        codec: NutValueCodec,
        unit: impl Into<String>,
    ) -> Result<Self, NutIntegrationError> {
        let point = Self {
            id: id.into(),
            name: name.into(),
            variable: variable.into(),
            codec,
            unit: unit.into(),
        };
        point.validate()?;
        Ok(point)
    }

    fn validate(&self) -> Result<(), NutIntegrationError> {
        if stable_component(&self.id).is_empty() {
            return Err(NutIntegrationError::Validation(
                "point id must contain an ASCII letter or digit".to_string(),
            ));
        }
        validate_safe_text(&self.name, "point name", 128)?;
        validate_variable_name(&self.variable)?;
        validate_safe_text(&self.unit, "point unit", 64)?;
        self.codec.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NutDeviceConfig {
    pub bridge_id: BridgeId,
    pub endpoint: SocketAddr,
    pub ups_name: String,
    pub device_key: String,
    pub display_name: String,
    pub manufacturer: String,
    pub model: String,
    pub timeout: Duration,
    pub points: Vec<NutPoint>,
}

impl NutDeviceConfig {
    pub fn new(
        bridge_id: BridgeId,
        endpoint: SocketAddr,
        ups_name: impl Into<String>,
        device_key: impl Into<String>,
        points: Vec<NutPoint>,
    ) -> Result<Self, NutIntegrationError> {
        let config = Self {
            bridge_id,
            endpoint,
            ups_name: ups_name.into(),
            device_key: device_key.into(),
            display_name: "Network UPS".to_string(),
            manufacturer: "Unknown".to_string(),
            model: "NUT UPS".to_string(),
            timeout: Duration::from_secs(3),
            points,
        };
        config.validate()?;
        Ok(config)
    }

    pub fn with_display_name(mut self, value: impl Into<String>) -> Self {
        self.display_name = value.into();
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

    pub fn validate(&self) -> Result<(), NutIntegrationError> {
        validate_local_endpoint(self.endpoint)?;
        nut_protocol::validate_ups_name(&self.ups_name)?;
        validate_safe_text(&self.device_key, "device key", 128)?;
        validate_safe_text(&self.display_name, "display name", 128)?;
        validate_safe_text(&self.manufacturer, "manufacturer", 128)?;
        validate_safe_text(&self.model, "model", 128)?;
        if self.timeout.is_zero() || self.timeout > Duration::from_secs(30) {
            return Err(NutIntegrationError::Validation(
                "timeout must be between 1 nanosecond and 30 seconds".to_string(),
            ));
        }
        if self.points.is_empty() || self.points.len() > MAX_PROFILE_POINTS {
            return Err(NutIntegrationError::Validation(format!(
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
            .collect::<Result<Vec<_>, NutIntegrationError>>()?;
        ids.sort();
        if ids.windows(2).any(|window| window[0] == window[1]) {
            return Err(NutIntegrationError::Validation(
                "point ids must be unique after normalization".to_string(),
            ));
        }
        let mut variables = self
            .points
            .iter()
            .map(|point| point.variable.as_str())
            .collect::<Vec<_>>();
        variables.sort_unstable();
        if variables.windows(2).any(|window| window[0] == window[1]) {
            return Err(NutIntegrationError::Validation(
                "profile variable names must be unique".to_string(),
            ));
        }
        Ok(())
    }

    pub fn endpoint_uri(&self) -> String {
        format!("nut://{}/{}", self.endpoint, self.ups_name)
    }
}

pub struct NutExchangePlan<'a> {
    pub endpoint: SocketAddr,
    pub timeout: Duration,
    pub ups_name: &'a str,
    pub request: &'a [u8],
}

pub trait NutTransport {
    fn exchange(&mut self, plan: NutExchangePlan<'_>) -> Result<Vec<u8>, NutIntegrationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TcpNutTransport;

impl NutTransport for TcpNutTransport {
    fn exchange(&mut self, plan: NutExchangePlan<'_>) -> Result<Vec<u8>, NutIntegrationError> {
        let mut connection = connect(
            &plan.endpoint.ip().to_string(),
            plan.endpoint.port(),
            ConnectOptions {
                connect_timeout: plan.timeout,
                read_timeout: Some(plan.timeout),
                write_timeout: Some(plan.timeout),
                buffer_size: MAX_LINE_BYTES,
            },
        )?;
        let peer = connection.peer_addr()?;
        if peer != plan.endpoint {
            return Err(NutIntegrationError::UnexpectedPeer {
                expected: plan.endpoint,
                actual: peer,
            });
        }
        connection.write_all(plan.request)?;
        connection.flush()?;

        let expected_end = list_var_end_line(plan.ups_name)?;
        let mut response = Vec::new();
        loop {
            let remaining = MAX_RESPONSE_BYTES.saturating_sub(response.len());
            if remaining == 0 {
                return Err(NutIntegrationError::Protocol(
                    NutProtocolError::ResponseTooLarge,
                ));
            }
            let line = connection.read_until_limit(b'\n', remaining.min(MAX_LINE_BYTES + 2))?;
            if line.is_empty() {
                return Err(NutIntegrationError::Validation(
                    "NUT server closed before the list terminator".to_string(),
                ));
            }
            response.extend_from_slice(&line);
            let line = std::str::from_utf8(&line)
                .map_err(|_| NutIntegrationError::Protocol(NutProtocolError::InvalidUtf8))?;
            let line = line.trim_end_matches(['\r', '\n']);
            if line == expected_end || line.starts_with("ERR ") {
                break;
            }
        }
        Ok(response)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NutScalar {
    Number(f64),
    Boolean(bool),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NutMeasurement {
    pub point_id: String,
    pub name: String,
    pub variable: String,
    pub codec: NutValueCodec,
    pub unit: String,
    pub value: NutScalar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NutSnapshot {
    pub endpoint: SocketAddr,
    pub ups_name: String,
    pub measurements: Vec<NutMeasurement>,
}

pub struct NutClient<T> {
    config: NutDeviceConfig,
    transport: T,
}

impl<T: NutTransport> NutClient<T> {
    pub fn new(config: NutDeviceConfig, transport: T) -> Self {
        Self { config, transport }
    }

    pub fn config(&self) -> &NutDeviceConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<NutSnapshot, NutIntegrationError> {
        self.config.validate()?;
        let request = encode_list_var_request(&self.config.ups_name)?;
        let response = self.transport.exchange(NutExchangePlan {
            endpoint: self.config.endpoint,
            timeout: self.config.timeout,
            ups_name: &self.config.ups_name,
            request: &request,
        })?;
        let response = decode_list_var_response(&response, &self.config.ups_name)?;
        measurements(&self.config, response)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledNutDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

pub struct NutRuntimeIntegration<T> {
    client: NutClient<T>,
}

impl<T: NutTransport> NutRuntimeIntegration<T> {
    pub fn new(client: NutClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledNutDevice, NutIntegrationError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &NutSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledNutDevice, NutIntegrationError> {
        validate_snapshot(&self.client.config, snapshot)?;
        let endpoint_uri = self.client.config.endpoint_uri();
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!(
            "nut:{}",
            stable_component(&self.client.config.device_key)
        ));
        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanTcp,
        );
        bridge.address = Some(endpoint_uri.clone());
        bridge.hardware_model = Some("Network UPS Tools server".to_string());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier(
            "ups_endpoint",
            &format!("{}#{}", snapshot.endpoint, snapshot.ups_name),
        )?];
        bridge.metadata = vec![
            Metadata::new("nut.transport", "tcp"),
            Metadata::new("nut.access", "anonymous_read_only"),
            Metadata::new("nut.ups_name", snapshot.ups_name.clone()),
        ];

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
                        Metadata::new("nut.point_id", measurement.point_id.clone()),
                        Metadata::new("nut.variable", measurement.variable.clone()),
                        Metadata::new("nut.codec", measurement.codec.as_str()),
                        Metadata::new("nut.unit", measurement.unit.clone()),
                    ],
                }
            })
            .collect::<Vec<_>>();
        let entity_ids = entities
            .iter()
            .map(|entity| entity.entity_id.clone())
            .collect::<Vec<_>>();
        let device = Device {
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
                &format!(
                    "{}#{}#{}",
                    snapshot.endpoint, snapshot.ups_name, self.client.config.device_key
                ),
            )?],
            health: Health::Online,
            metadata: vec![
                Metadata::new("nut.endpoint", endpoint_uri),
                Metadata::new("nut.profile_points", entity_ids.len().to_string()),
            ],
        };

        runtime.upsert_bridge(bridge)?;
        runtime.upsert_device(device)?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledNutDevice {
            bridge_id,
            device_id,
            entity_ids,
        })
    }
}

fn measurements(
    config: &NutDeviceConfig,
    response: ListVarResponse,
) -> Result<NutSnapshot, NutIntegrationError> {
    let values = response
        .variables
        .into_iter()
        .map(|variable| (variable.name, variable.value))
        .collect::<BTreeMap<_, _>>();
    let measurements = config
        .points
        .iter()
        .map(|point| {
            let value = values
                .get(&point.variable)
                .ok_or_else(|| NutIntegrationError::MissingVariable(point.variable.clone()))?;
            Ok(NutMeasurement {
                point_id: point.id.clone(),
                name: point.name.clone(),
                variable: point.variable.clone(),
                codec: point.codec.clone(),
                unit: point.unit.clone(),
                value: decode_scalar(point, value)?,
            })
        })
        .collect::<Result<Vec<_>, NutIntegrationError>>()?;
    Ok(NutSnapshot {
        endpoint: config.endpoint,
        ups_name: response.ups_name,
        measurements,
    })
}

fn decode_scalar(point: &NutPoint, value: &str) -> Result<NutScalar, NutIntegrationError> {
    match &point.codec {
        NutValueCodec::Decimal => parse_decimal(point, value).map(NutScalar::Number),
        NutValueCodec::Boolean {
            true_value,
            false_value: _,
        } if value == true_value => Ok(NutScalar::Boolean(true)),
        NutValueCodec::Boolean {
            true_value: _,
            false_value,
        } if value == false_value => Ok(NutScalar::Boolean(false)),
        NutValueCodec::Boolean { .. } => Err(invalid_value(
            point,
            "value does not match either configured boolean token",
        )),
        NutValueCodec::Text => safe_measurement_text(point, value).map(NutScalar::Text),
    }
}

fn parse_decimal(point: &NutPoint, value: &str) -> Result<f64, NutIntegrationError> {
    let unsigned = value.strip_prefix('-').unwrap_or(value);
    let valid = !unsigned.is_empty()
        && unsigned.split_once('.').map_or_else(
            || unsigned.bytes().all(|byte| byte.is_ascii_digit()),
            |(integer, fraction)| {
                !integer.is_empty()
                    && !fraction.is_empty()
                    && integer.bytes().all(|byte| byte.is_ascii_digit())
                    && fraction.bytes().all(|byte| byte.is_ascii_digit())
                    && !fraction.contains('.')
            },
        );
    if !valid {
        return Err(invalid_value(
            point,
            "expected canonical base-10 notation without exponent or sign prefix",
        ));
    }
    let parsed = value
        .parse::<f64>()
        .map_err(|error| invalid_value(point, error))?;
    if !parsed.is_finite() {
        return Err(invalid_value(point, "number must be finite"));
    }
    Ok(parsed)
}

fn safe_measurement_text(point: &NutPoint, value: &str) -> Result<String, NutIntegrationError> {
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

fn validate_snapshot(
    config: &NutDeviceConfig,
    snapshot: &NutSnapshot,
) -> Result<(), NutIntegrationError> {
    if snapshot.endpoint != config.endpoint
        || snapshot.ups_name != config.ups_name
        || snapshot.measurements.len() != config.points.len()
        || !snapshot
            .measurements
            .iter()
            .zip(&config.points)
            .all(|(measurement, point)| measurement_matches_point(measurement, point))
    {
        return Err(NutIntegrationError::Validation(
            "snapshot does not match the configured endpoint, UPS, and variable profile"
                .to_string(),
        ));
    }
    Ok(())
}

fn measurement_matches_point(measurement: &NutMeasurement, point: &NutPoint) -> bool {
    measurement.point_id == point.id
        && measurement.name == point.name
        && measurement.variable == point.variable
        && measurement.codec == point.codec
        && measurement.unit == point.unit
        && match &measurement.value {
            NutScalar::Number(value) => value.is_finite(),
            NutScalar::Boolean(_) => true,
            NutScalar::Text(value) => {
                !value.is_empty()
                    && value.len() <= MAX_TEXT_BYTES
                    && !value.chars().any(|character| character.is_control())
            }
        }
}

fn measurement_value(measurement: &NutMeasurement) -> Value {
    let value = match &measurement.value {
        NutScalar::Number(value) => Value::Number(*value),
        NutScalar::Boolean(value) => Value::Bool(*value),
        NutScalar::Text(value) => Value::Text(value.clone()),
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
) -> Result<(), NutIntegrationError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(NutIntegrationError::Runtime(
            RuntimeError::UnauthorizedTool {
                principal_id,
                tool,
                missing_capabilities: decision.missing_capabilities,
            },
        ))
    }
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, NutIntegrationError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| NutIntegrationError::Validation(error.to_string()))
}

fn validate_local_endpoint(endpoint: SocketAddr) -> Result<(), NutIntegrationError> {
    if endpoint.port() == 0 || !is_local_unicast(endpoint.ip()) {
        return Err(NutIntegrationError::Validation(
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

fn validate_safe_text(
    value: &str,
    field: &str,
    max_bytes: usize,
) -> Result<(), NutIntegrationError> {
    if value.is_empty()
        || value.len() > max_bytes
        || value.chars().any(|character| character.is_control())
    {
        return Err(NutIntegrationError::Validation(format!(
            "{field} must be control-free and between 1 and {max_bytes} bytes"
        )));
    }
    Ok(())
}

fn stable_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn invalid_value(point: &NutPoint, message: impl fmt::Display) -> NutIntegrationError {
    NutIntegrationError::InvalidValue {
        point_id: point.id.clone(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    fn charge_point() -> NutPoint {
        NutPoint::new(
            "battery-charge",
            "Battery charge",
            "battery.charge",
            NutValueCodec::Decimal,
            "%",
        )
        .unwrap()
    }

    fn status_point() -> NutPoint {
        NutPoint::new(
            "status",
            "Status",
            "ups.status",
            NutValueCodec::Text,
            "state",
        )
        .unwrap()
    }

    fn config(endpoint: SocketAddr) -> NutDeviceConfig {
        NutDeviceConfig::new(
            BridgeId::trusted("nut.test"),
            endpoint,
            "ups-1",
            "rack-ups",
            vec![charge_point(), status_point()],
        )
        .unwrap()
        .with_display_name("Rack UPS")
        .with_device_identity("Acme", "UPS-1")
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:nut-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[derive(Debug)]
    struct ScriptedTransport {
        calls: Arc<AtomicUsize>,
        response: Vec<u8>,
    }

    impl NutTransport for ScriptedTransport {
        fn exchange(&mut self, _plan: NutExchangePlan<'_>) -> Result<Vec<u8>, NutIntegrationError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.response.clone())
        }
    }

    #[test]
    fn live_tcp_poll_installs_normalized_sensors() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = String::new();
            BufReader::new(stream.try_clone().unwrap())
                .read_line(&mut request)
                .unwrap();
            assert_eq!(request, "LIST VAR ups-1\n");
            stream
                .write_all(b"BEGIN LIST VAR ups-1\nVAR ups-1 battery.charge \"97.5\"\nVAR ups-1 ups.status \"OL\"\nEND LIST VAR ups-1\n")
                .unwrap();
        });

        let principal = AgentId::trusted("agent:nut-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let client = NutClient::new(config(endpoint), TcpNutTransport);
        let mut integration = NutRuntimeIntegration::new(client);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();

        assert_eq!(installed.entity_ids.len(), 2);
        let charge = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:battery-charge",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(
            charge.state.as_ref().unwrap().value,
            Value::Object(vec![
                ("value".to_string(), Value::Number(97.5)),
                ("unit".to_string(), Value::Text("%".to_string())),
            ])
        );
        assert!(runtime
            .registry()
            .bridge(&installed.bridge_id)
            .unwrap()
            .auth_ref
            .is_none());
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = NutClient::new(
            config("127.0.0.1:3493".parse().unwrap()),
            ScriptedTransport {
                calls: Arc::clone(&calls),
                response: Vec::new(),
            },
        );
        let mut integration = NutRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut runtime,
                AgentId::trusted("agent:denied"),
                5_000
            ),
            Err(NutIntegrationError::Runtime(
                RuntimeError::UnauthorizedTool { .. }
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn rejects_public_endpoints_and_duplicate_profiles() {
        assert!(NutDeviceConfig::new(
            BridgeId::trusted("nut.public"),
            "8.8.8.8:3493".parse().unwrap(),
            "ups",
            "device",
            vec![charge_point()],
        )
        .is_err());
        assert!(NutDeviceConfig::new(
            BridgeId::trusted("nut.duplicate"),
            "127.0.0.1:3493".parse().unwrap(),
            "ups",
            "device",
            vec![charge_point(), charge_point()],
        )
        .is_err());
    }

    #[test]
    fn strict_codecs_reject_noncanonical_values() {
        let point = charge_point();
        for value in ["+1", "1e3", ".5", "1.", "NaN"] {
            assert!(decode_scalar(&point, value).is_err(), "accepted {value}");
        }
        let boolean = NutPoint::new(
            "online",
            "Online",
            "ups.online",
            NutValueCodec::Boolean {
                true_value: "yes".to_string(),
                false_value: "no".to_string(),
            },
            "bool",
        )
        .unwrap();
        assert_eq!(
            decode_scalar(&boolean, "yes").unwrap(),
            NutScalar::Boolean(true)
        );
        assert!(decode_scalar(&boolean, "1").is_err());
    }

    #[test]
    fn missing_profile_value_is_atomic() {
        let endpoint = "127.0.0.1:3493".parse().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let response =
            b"BEGIN LIST VAR ups-1\nVAR ups-1 battery.charge \"90\"\nEND LIST VAR ups-1\n".to_vec();
        let principal = AgentId::trusted("agent:nut-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let client = NutClient::new(
            config(endpoint),
            ScriptedTransport {
                calls: Arc::clone(&calls),
                response,
            },
        );
        let mut integration = NutRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(&mut runtime, principal, 5_000),
            Err(NutIntegrationError::MissingVariable(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("nut.test"))
            .is_none());
    }

    #[test]
    fn rejects_mismatched_snapshot_before_runtime_mutation() {
        let endpoint = "127.0.0.1:3493".parse().unwrap();
        let client = NutClient::new(
            config(endpoint),
            ScriptedTransport {
                calls: Arc::new(AtomicUsize::new(0)),
                response: Vec::new(),
            },
        );
        let integration = NutRuntimeIntegration::new(client);
        let snapshot = NutSnapshot {
            endpoint,
            ups_name: "wrong".to_string(),
            measurements: Vec::new(),
        };
        let mut runtime = SmartHomeRuntime::new();
        assert!(integration
            .install_snapshot(&mut runtime, &snapshot, 1_000)
            .is_err());
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("nut.test"))
            .is_none());
    }

    #[test]
    fn bounded_server_errors_are_preserved_as_protocol_failures() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut client = NutClient::new(
            config("127.0.0.1:3493".parse().unwrap()),
            ScriptedTransport {
                calls: Arc::clone(&calls),
                response: b"ERR UNKNOWN-UPS\n".to_vec(),
            },
        );
        assert!(matches!(
            client.inspect(),
            Err(NutIntegrationError::Protocol(
                NutProtocolError::ServerError(_)
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
