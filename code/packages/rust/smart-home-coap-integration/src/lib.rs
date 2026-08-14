//! Authorized read-only CoAP telemetry integration for D23.

#![forbid(unsafe_code)]

use coap_protocol::{
    decode_response, encode_confirmable_get, encode_empty_ack, CoapError, CoapResponse,
    DecodedResponse, GetRequest, MessageType, RequestContext, ResponseCode,
    CONTENT_FORMAT_APPLICATION_JSON, CONTENT_FORMAT_TEXT_PLAIN, MAX_DATAGRAM_BYTES,
};
use serde_json::Value as JsonValue;
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use udp_client::{UdpClient, UdpError, UdpOptions};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "coap";
pub const PROTOCOL_ID: &str = "coap";
pub const DEFAULT_PORT: u16 = 5_683;
pub const MAX_PROFILE_POINTS: usize = 32;
pub const MAX_PAYLOAD_BYTES: usize = 4_096;
pub const MAX_TEXT_BYTES: usize = 512;

#[derive(Debug)]
pub enum CoapIntegrationError {
    Validation(String),
    Protocol(CoapError),
    Udp(UdpError),
    ResponseCode {
        point_id: String,
        code: ResponseCode,
    },
    ContentFormat {
        point_id: String,
        expected: u16,
        actual: Option<u16>,
    },
    PayloadTooLarge {
        point_id: String,
        actual: usize,
        maximum: usize,
    },
    InvalidPayload {
        point_id: String,
        message: String,
    },
    Runtime(RuntimeError),
}

impl fmt::Display for CoapIntegrationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid CoAP input: {message}"),
            Self::Protocol(error) => error.fmt(formatter),
            Self::Udp(error) => error.fmt(formatter),
            Self::ResponseCode { point_id, code } => {
                write!(formatter, "CoAP point `{point_id}` returned {code}")
            }
            Self::ContentFormat {
                point_id,
                expected,
                actual,
            } => write!(
                formatter,
                "CoAP point `{point_id}` expected content format {expected}, got {}",
                actual
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "none".to_string())
            ),
            Self::PayloadTooLarge {
                point_id,
                actual,
                maximum,
            } => write!(
                formatter,
                "CoAP point `{point_id}` returned {actual} payload bytes, exceeding {maximum}"
            ),
            Self::InvalidPayload { point_id, message } => {
                write!(
                    formatter,
                    "invalid CoAP payload for `{point_id}`: {message}"
                )
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CoapIntegrationError {}

impl From<CoapError> for CoapIntegrationError {
    fn from(error: CoapError) -> Self {
        Self::Protocol(error)
    }
}

impl From<UdpError> for CoapIntegrationError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<RuntimeError> for CoapIntegrationError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoapValueCodec {
    TextNumber,
    TextBoolean {
        true_value: String,
        false_value: String,
    },
    Text,
    JsonNumber {
        key: String,
    },
    JsonBoolean {
        key: String,
    },
    JsonText {
        key: String,
    },
}

impl CoapValueCodec {
    pub const fn content_format(&self) -> u16 {
        match self {
            Self::TextNumber | Self::TextBoolean { .. } | Self::Text => CONTENT_FORMAT_TEXT_PLAIN,
            Self::JsonNumber { .. } | Self::JsonBoolean { .. } | Self::JsonText { .. } => {
                CONTENT_FORMAT_APPLICATION_JSON
            }
        }
    }

    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::TextNumber => "text_number",
            Self::TextBoolean { .. } => "text_boolean",
            Self::Text => "text",
            Self::JsonNumber { .. } => "json_number",
            Self::JsonBoolean { .. } => "json_boolean",
            Self::JsonText { .. } => "json_text",
        }
    }

    fn validate(&self) -> Result<(), CoapIntegrationError> {
        match self {
            Self::TextBoolean {
                true_value,
                false_value,
            } => {
                validate_bounded_text(true_value, "true payload", 128)?;
                validate_bounded_text(false_value, "false payload", 128)?;
                if true_value == false_value {
                    return Err(CoapIntegrationError::Validation(
                        "boolean true and false payloads must differ".to_string(),
                    ));
                }
            }
            Self::JsonNumber { key } | Self::JsonBoolean { key } | Self::JsonText { key } => {
                validate_json_key(key)?
            }
            Self::TextNumber | Self::Text => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoapPoint {
    pub id: String,
    pub name: String,
    pub path: String,
    pub codec: CoapValueCodec,
    pub unit: String,
}

impl CoapPoint {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        path: impl Into<String>,
        codec: CoapValueCodec,
        unit: impl Into<String>,
    ) -> Result<Self, CoapIntegrationError> {
        let point = Self {
            id: id.into(),
            name: name.into(),
            path: path.into(),
            codec,
            unit: unit.into(),
        };
        point.validate()?;
        Ok(point)
    }

    fn validate(&self) -> Result<(), CoapIntegrationError> {
        if stable_component(&self.id).is_empty() {
            return Err(CoapIntegrationError::Validation(
                "point id must contain an ASCII letter or digit".to_string(),
            ));
        }
        validate_bounded_text(&self.name, "point name", 128)?;
        validate_bounded_text(&self.unit, "point unit", 64)?;
        let _ = GetRequest::new(&self.path)?;
        self.codec.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoapDeviceConfig {
    pub bridge_id: BridgeId,
    pub endpoint: SocketAddr,
    pub device_key: String,
    pub display_name: String,
    pub manufacturer: String,
    pub model: String,
    pub timeout: Duration,
    pub maximum_payload_bytes: usize,
    pub points: Vec<CoapPoint>,
}

impl CoapDeviceConfig {
    pub fn new(
        bridge_id: BridgeId,
        endpoint: SocketAddr,
        device_key: impl Into<String>,
        points: Vec<CoapPoint>,
    ) -> Result<Self, CoapIntegrationError> {
        let device_key = device_key.into();
        let config = Self {
            bridge_id,
            endpoint,
            display_name: device_key.clone(),
            device_key,
            manufacturer: "CoAP".to_string(),
            model: "Constrained device".to_string(),
            timeout: Duration::from_secs(3),
            maximum_payload_bytes: MAX_PAYLOAD_BYTES,
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

    pub fn with_maximum_payload_bytes(mut self, maximum: usize) -> Self {
        self.maximum_payload_bytes = maximum;
        self
    }

    pub fn validate(&self) -> Result<(), CoapIntegrationError> {
        validate_local_endpoint(self.endpoint)?;
        validate_bounded_text(&self.device_key, "device key", 128)?;
        if stable_component(&self.device_key).is_empty() {
            return Err(CoapIntegrationError::Validation(
                "device key must contain an ASCII letter or digit".to_string(),
            ));
        }
        validate_bounded_text(&self.display_name, "display name", 128)?;
        validate_bounded_text(&self.manufacturer, "manufacturer", 128)?;
        validate_bounded_text(&self.model, "model", 128)?;
        if self.timeout.is_zero() {
            return Err(CoapIntegrationError::Validation(
                "timeout must be non-zero".to_string(),
            ));
        }
        if !(1..=MAX_PAYLOAD_BYTES).contains(&self.maximum_payload_bytes) {
            return Err(CoapIntegrationError::Validation(format!(
                "maximum payload bytes must be between 1 and {MAX_PAYLOAD_BYTES}"
            )));
        }
        if self.points.is_empty() || self.points.len() > MAX_PROFILE_POINTS {
            return Err(CoapIntegrationError::Validation(format!(
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
            .collect::<Result<Vec<_>, CoapIntegrationError>>()?;
        ids.sort();
        if ids.windows(2).any(|window| window[0] == window[1]) {
            return Err(CoapIntegrationError::Validation(
                "point ids must be unique after normalization".to_string(),
            ));
        }
        Ok(())
    }

    pub fn endpoint_uri(&self) -> String {
        format!("coap://{}", self.endpoint)
    }
}

pub struct CoapExchangePlan<'a> {
    pub endpoint: SocketAddr,
    pub timeout: Duration,
    pub context: &'a RequestContext,
    pub request: &'a GetRequest,
}

pub trait CoapTransport {
    fn get(&mut self, plan: CoapExchangePlan<'_>) -> Result<CoapResponse, CoapIntegrationError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UdpCoapTransport;

impl CoapTransport for UdpCoapTransport {
    fn get(&mut self, plan: CoapExchangePlan<'_>) -> Result<CoapResponse, CoapIntegrationError> {
        let request = encode_confirmable_get(plan.context, plan.request)?;
        let mut client = UdpClient::bind(UdpOptions {
            bind_addr: Some(unspecified_for(plan.endpoint)),
            max_datagram_size: MAX_DATAGRAM_BYTES,
            read_timeout: Some(plan.timeout),
            write_timeout: Some(plan.timeout),
        })?;
        client.connect(plan.endpoint)?;
        client.send(&request)?;

        let first = client.recv_from()?;
        let response = match decode_response(&first.payload, plan.context)? {
            DecodedResponse::EmptyAcknowledgement { .. } => {
                let separate = client.recv_from()?;
                match decode_response(&separate.payload, plan.context)? {
                    DecodedResponse::Response(response) => response,
                    DecodedResponse::EmptyAcknowledgement { .. } => {
                        return Err(CoapIntegrationError::Validation(
                            "CoAP exchange returned a second empty acknowledgement".to_string(),
                        ));
                    }
                }
            }
            DecodedResponse::Response(response) => response,
        };
        if response.message_type == MessageType::Confirmable {
            client.send(&encode_empty_ack(response.message_id))?;
        }
        Ok(response)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoapScalar {
    Number(f64),
    Boolean(bool),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoapMeasurement {
    pub point_id: String,
    pub name: String,
    pub path: String,
    pub codec: CoapValueCodec,
    pub unit: String,
    pub value: CoapScalar,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoapSnapshot {
    pub endpoint: SocketAddr,
    pub measurements: Vec<CoapMeasurement>,
}

pub struct CoapClient<T> {
    config: CoapDeviceConfig,
    transport: T,
    next_message_id: u16,
    next_token_nonce: u16,
}

impl<T: CoapTransport> CoapClient<T> {
    pub fn new(config: CoapDeviceConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            next_message_id: 1,
            next_token_nonce: 1,
        }
    }

    pub fn config(&self) -> &CoapDeviceConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<CoapSnapshot, CoapIntegrationError> {
        self.config.validate()?;
        let mut measurements = Vec::with_capacity(self.config.points.len());
        for point in &self.config.points {
            let message_id = self.next_message_id;
            let nonce = self.next_token_nonce;
            self.next_message_id = self.next_message_id.wrapping_add(1);
            self.next_token_nonce = self.next_token_nonce.wrapping_add(1);
            let context = RequestContext::new(
                message_id,
                [message_id.to_be_bytes(), nonce.to_be_bytes()].concat(),
            )?;
            let request = GetRequest::new(&point.path)?.with_accept(point.codec.content_format());
            let response = self.transport.get(CoapExchangePlan {
                endpoint: self.config.endpoint,
                timeout: self.config.timeout,
                context: &context,
                request: &request,
            })?;
            if response.code != ResponseCode::CONTENT {
                return Err(CoapIntegrationError::ResponseCode {
                    point_id: point.id.clone(),
                    code: response.code,
                });
            }
            let expected_format = point.codec.content_format();
            if response.content_format != Some(expected_format) {
                return Err(CoapIntegrationError::ContentFormat {
                    point_id: point.id.clone(),
                    expected: expected_format,
                    actual: response.content_format,
                });
            }
            if response.payload.len() > self.config.maximum_payload_bytes {
                return Err(CoapIntegrationError::PayloadTooLarge {
                    point_id: point.id.clone(),
                    actual: response.payload.len(),
                    maximum: self.config.maximum_payload_bytes,
                });
            }
            measurements.push(CoapMeasurement {
                point_id: point.id.clone(),
                name: point.name.clone(),
                path: point.path.clone(),
                codec: point.codec.clone(),
                unit: point.unit.clone(),
                value: decode_scalar(point, &response.payload)?,
            });
        }
        Ok(CoapSnapshot {
            endpoint: self.config.endpoint,
            measurements,
        })
    }
}

impl<T> fmt::Debug for CoapClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CoapClient")
            .field("config", &self.config)
            .field("next_message_id", &self.next_message_id)
            .field("next_token_nonce", &self.next_token_nonce)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledCoapDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

pub struct CoapRuntimeIntegration<T> {
    client: CoapClient<T>,
}

impl<T: CoapTransport> CoapRuntimeIntegration<T> {
    pub fn new(client: CoapClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledCoapDevice, CoapIntegrationError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &CoapSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledCoapDevice, CoapIntegrationError> {
        if snapshot.endpoint != self.client.config.endpoint
            || snapshot.measurements.len() != self.client.config.points.len()
            || !snapshot
                .measurements
                .iter()
                .zip(&self.client.config.points)
                .all(|(measurement, point)| measurement_matches_point(measurement, point))
        {
            return Err(CoapIntegrationError::Validation(
                "snapshot does not match the configured endpoint and point profile".to_string(),
            ));
        }

        let endpoint_uri = self.client.config.endpoint_uri();
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!(
            "coap:{}",
            stable_component(&self.client.config.device_key)
        ));

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanUdp,
        );
        bridge.address = Some(endpoint_uri.clone());
        bridge.hardware_model = Some("CoAP endpoint".to_string());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier(
            "udp_endpoint",
            &snapshot.endpoint.to_string(),
        )?];
        bridge.metadata = vec![
            Metadata::new("coap.transport", "udp"),
            Metadata::new("coap.access", "read_only"),
            Metadata::new("coap.security", "none_local_unicast"),
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
                        Metadata::new("coap.point_id", measurement.point_id.clone()),
                        Metadata::new("coap.path", measurement.path.clone()),
                        Metadata::new("coap.codec", measurement.codec.as_str()),
                        Metadata::new(
                            "coap.content_format",
                            measurement.codec.content_format().to_string(),
                        ),
                        Metadata::new("coap.unit", measurement.unit.clone()),
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
                Metadata::new("coap.endpoint", endpoint_uri),
                Metadata::new("coap.profile_points", entity_ids.len().to_string()),
            ],
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledCoapDevice {
            bridge_id,
            device_id,
            entity_ids,
        })
    }
}

fn decode_scalar(point: &CoapPoint, payload: &[u8]) -> Result<CoapScalar, CoapIntegrationError> {
    let text = std::str::from_utf8(payload).map_err(|error| invalid_payload(point, error))?;
    match &point.codec {
        CoapValueCodec::TextNumber => parse_number(point, text.trim()).map(CoapScalar::Number),
        CoapValueCodec::TextBoolean {
            true_value,
            false_value,
        } => match text.trim() {
            value if value == true_value => Ok(CoapScalar::Boolean(true)),
            value if value == false_value => Ok(CoapScalar::Boolean(false)),
            _ => Err(invalid_payload(
                point,
                "text does not match the configured boolean payloads",
            )),
        },
        CoapValueCodec::Text => safe_payload_text(point, text).map(CoapScalar::Text),
        CoapValueCodec::JsonNumber { key } => {
            let value = json_field(point, text, key)?;
            value
                .as_f64()
                .filter(|number| number.is_finite())
                .map(CoapScalar::Number)
                .ok_or_else(|| invalid_payload(point, "JSON field is not a finite number"))
        }
        CoapValueCodec::JsonBoolean { key } => json_field(point, text, key)?
            .as_bool()
            .map(CoapScalar::Boolean)
            .ok_or_else(|| invalid_payload(point, "JSON field is not a boolean")),
        CoapValueCodec::JsonText { key } => json_field(point, text, key)?
            .as_str()
            .ok_or_else(|| invalid_payload(point, "JSON field is not text"))
            .and_then(|value| safe_payload_text(point, value).map(CoapScalar::Text)),
    }
}

fn json_field(point: &CoapPoint, text: &str, key: &str) -> Result<JsonValue, CoapIntegrationError> {
    let value =
        serde_json::from_str::<JsonValue>(text).map_err(|error| invalid_payload(point, error))?;
    value
        .as_object()
        .and_then(|object| object.get(key))
        .cloned()
        .ok_or_else(|| invalid_payload(point, format!("missing top-level JSON field `{key}`")))
}

fn parse_number(point: &CoapPoint, value: &str) -> Result<f64, CoapIntegrationError> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
        .ok_or_else(|| invalid_payload(point, "text is not a finite number"))
}

fn safe_payload_text(point: &CoapPoint, value: &str) -> Result<String, CoapIntegrationError> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > MAX_TEXT_BYTES
        || value.chars().any(|character| character.is_control())
    {
        return Err(invalid_payload(
            point,
            format!("text must be control-free and between 1 and {MAX_TEXT_BYTES} bytes"),
        ));
    }
    Ok(value.to_string())
}

fn invalid_payload(point: &CoapPoint, message: impl fmt::Display) -> CoapIntegrationError {
    CoapIntegrationError::InvalidPayload {
        point_id: point.id.clone(),
        message: message.to_string(),
    }
}

fn measurement_matches_point(measurement: &CoapMeasurement, point: &CoapPoint) -> bool {
    measurement.point_id == point.id
        && measurement.name == point.name
        && measurement.path == point.path
        && measurement.codec == point.codec
        && measurement.unit == point.unit
        && match &measurement.value {
            CoapScalar::Number(number) => number.is_finite(),
            CoapScalar::Boolean(_) => true,
            CoapScalar::Text(text) => {
                !text.is_empty()
                    && text.len() <= MAX_TEXT_BYTES
                    && !text.chars().any(|character| character.is_control())
            }
        }
}

fn measurement_value(measurement: &CoapMeasurement) -> Value {
    let value = match &measurement.value {
        CoapScalar::Number(number) => Value::Number(*number),
        CoapScalar::Boolean(value) => Value::Bool(*value),
        CoapScalar::Text(value) => Value::Text(value.clone()),
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
) -> Result<(), CoapIntegrationError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(CoapIntegrationError::Runtime(
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
) -> Result<ProtocolIdentifier, CoapIntegrationError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| CoapIntegrationError::Validation(error.to_string()))
}

fn validate_local_endpoint(endpoint: SocketAddr) -> Result<(), CoapIntegrationError> {
    if endpoint.port() == 0 || !is_local_unicast(endpoint.ip()) {
        return Err(CoapIntegrationError::Validation(
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
) -> Result<(), CoapIntegrationError> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.len() > maximum
        || value.chars().any(|character| character.is_control())
    {
        return Err(CoapIntegrationError::Validation(format!(
            "{field} must be trimmed, control-free, and between 1 and {maximum} bytes"
        )));
    }
    Ok(())
}

fn validate_json_key(key: &str) -> Result<(), CoapIntegrationError> {
    validate_bounded_text(key, "JSON key", 128)?;
    if key.contains(['.', '/', '[', ']']) {
        return Err(CoapIntegrationError::Validation(
            "JSON key must name one exact top-level field".to_string(),
        ));
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

    fn text_point() -> CoapPoint {
        CoapPoint::new(
            "temperature",
            "Temperature",
            "/temperature",
            CoapValueCodec::TextNumber,
            "C",
        )
        .unwrap()
    }

    fn json_point() -> CoapPoint {
        CoapPoint::new(
            "humidity",
            "Humidity",
            "/environment",
            CoapValueCodec::JsonNumber {
                key: "humidity".to_string(),
            },
            "%",
        )
        .unwrap()
    }

    fn config(endpoint: SocketAddr) -> CoapDeviceConfig {
        CoapDeviceConfig::new(
            BridgeId::trusted("coap.test"),
            endpoint,
            "room-sensor-01",
            vec![text_point(), json_point()],
        )
        .unwrap()
        .with_display_name("Room Sensor")
        .with_device_identity("Acme", "C1")
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:coap-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    fn response_for(request: &[u8], format: u16, payload: &[u8]) -> Vec<u8> {
        let token_length = usize::from(request[0] & 0x0f);
        let mut response = vec![
            0x60 | token_length as u8,
            ResponseCode::CONTENT.wire(),
            request[2],
            request[3],
        ];
        response.extend_from_slice(&request[4..4 + token_length]);
        response.push(0xc0 | u8::from(format != 0));
        if format != 0 {
            response.push(format as u8);
        }
        response.push(0xff);
        response.extend_from_slice(payload);
        response
    }

    #[test]
    fn live_udp_poll_installs_normalized_sensors() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let endpoint = server.local_addr().unwrap();
        let handle = thread::spawn(move || {
            for (format, payload) in [
                (CONTENT_FORMAT_TEXT_PLAIN, b"21.5".as_slice()),
                (
                    CONTENT_FORMAT_APPLICATION_JSON,
                    br#"{"humidity":44.25}"#.as_slice(),
                ),
            ] {
                let mut request = [0u8; 1_024];
                let (length, source) = server.recv_from(&mut request).unwrap();
                let response = response_for(&request[..length], format, payload);
                server.send_to(&response, source).unwrap();
            }
        });

        let principal = AgentId::trusted("agent:coap-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let client = CoapClient::new(config(endpoint), UdpCoapTransport);
        let mut integration = CoapRuntimeIntegration::new(client);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();

        assert_eq!(installed.entity_ids.len(), 2);
        let temperature = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:temperature",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(
            temperature.state.as_ref().unwrap().value,
            Value::Object(vec![
                ("value".to_string(), Value::Number(21.5)),
                ("unit".to_string(), Value::Text("C".to_string())),
            ])
        );
        let humidity = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:humidity",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(
            humidity.state.as_ref().unwrap().value,
            Value::Object(vec![
                ("value".to_string(), Value::Number(44.25)),
                ("unit".to_string(), Value::Text("%".to_string())),
            ])
        );
    }

    #[test]
    fn live_udp_separate_confirmable_response_is_acknowledged() {
        let server = UdpSocket::bind("127.0.0.1:0").unwrap();
        server
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let endpoint = server.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let mut request = [0u8; 1_024];
            let (length, source) = server.recv_from(&mut request).unwrap();
            let request = &request[..length];
            server
                .send_to(&[0x60, 0, request[2], request[3]], source)
                .unwrap();
            let token_length = usize::from(request[0] & 0x0f);
            let mut response = vec![0x40 | token_length as u8, 0x45, 0x12, 0x34];
            response.extend_from_slice(&request[4..4 + token_length]);
            response.extend_from_slice(&[0xc0, 0xff]);
            response.extend_from_slice(b"19.75");
            server.send_to(&response, source).unwrap();

            let mut ack = [0u8; 16];
            let (ack_length, ack_source) = server.recv_from(&mut ack).unwrap();
            assert_eq!(ack_source, source);
            assert_eq!(&ack[..ack_length], &[0x60, 0, 0x12, 0x34]);
        });
        let one_point = CoapDeviceConfig::new(
            BridgeId::trusted("coap.separate"),
            endpoint,
            "separate",
            vec![text_point()],
        )
        .unwrap();
        let mut client = CoapClient::new(one_point, UdpCoapTransport);
        let snapshot = client.inspect().unwrap();
        handle.join().unwrap();
        assert_eq!(snapshot.measurements[0].value, CoapScalar::Number(19.75));
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl CoapTransport for CountingTransport {
        fn get(
            &mut self,
            _plan: CoapExchangePlan<'_>,
        ) -> Result<CoapResponse, CoapIntegrationError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(CoapResponse {
                message_type: MessageType::Acknowledgement,
                message_id: 1,
                code: ResponseCode::CONTENT,
                content_format: Some(CONTENT_FORMAT_TEXT_PLAIN),
                payload: b"20".to_vec(),
            })
        }
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let endpoint = SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT));
        let config = CoapDeviceConfig::new(
            BridgeId::trusted("coap.denied"),
            endpoint,
            "denied",
            vec![text_point()],
        )
        .unwrap();
        let client = CoapClient::new(config, CountingTransport(Arc::clone(&calls)));
        let mut integration = CoapRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(CoapIntegrationError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn rejects_public_endpoints_and_duplicate_profiles() {
        let public = SocketAddr::from(([203, 0, 113, 1], DEFAULT_PORT));
        assert!(CoapDeviceConfig::new(
            BridgeId::trusted("coap.public"),
            public,
            "public",
            vec![text_point()],
        )
        .is_err());
        let local = SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT));
        let duplicate = CoapPoint::new(
            "Temperature",
            "Other",
            "/other",
            CoapValueCodec::TextNumber,
            "C",
        )
        .unwrap();
        assert!(CoapDeviceConfig::new(
            BridgeId::trusted("coap.duplicate"),
            local,
            "duplicate",
            vec![text_point(), duplicate],
        )
        .is_err());
    }

    #[test]
    fn strict_codecs_reject_wrong_formats_and_values() {
        let boolean = CoapPoint::new(
            "contact",
            "Contact",
            "/contact",
            CoapValueCodec::TextBoolean {
                true_value: "open".to_string(),
                false_value: "closed".to_string(),
            },
            "state",
        )
        .unwrap();
        assert_eq!(
            decode_scalar(&boolean, b"open").unwrap(),
            CoapScalar::Boolean(true)
        );
        assert!(decode_scalar(&boolean, b"unknown").is_err());
        assert!(decode_scalar(&text_point(), b"NaN").is_err());

        let json_text = CoapPoint::new(
            "mode",
            "Mode",
            "/mode",
            CoapValueCodec::JsonText {
                key: "mode".to_string(),
            },
            "label",
        )
        .unwrap();
        assert_eq!(
            decode_scalar(&json_text, br#"{"mode":"eco"}"#).unwrap(),
            CoapScalar::Text("eco".to_string())
        );
        assert!(decode_scalar(&json_text, br#"{"other":"eco"}"#).is_err());
    }

    #[test]
    fn rejects_mismatched_snapshot_before_runtime_mutation() {
        let endpoint = SocketAddr::from(([127, 0, 0, 1], DEFAULT_PORT));
        let config = CoapDeviceConfig::new(
            BridgeId::trusted("coap.snapshot"),
            endpoint,
            "snapshot",
            vec![text_point()],
        )
        .unwrap();
        let client = CoapClient::new(config, CountingTransport(Arc::new(AtomicUsize::new(0))));
        let integration = CoapRuntimeIntegration::new(client);
        let snapshot = CoapSnapshot {
            endpoint,
            measurements: vec![CoapMeasurement {
                point_id: "different".to_string(),
                name: "Temperature".to_string(),
                path: "/temperature".to_string(),
                codec: CoapValueCodec::TextNumber,
                unit: "C".to_string(),
                value: CoapScalar::Number(20.0),
            }],
        };
        let mut runtime = SmartHomeRuntime::new();
        assert!(integration
            .install_snapshot(&mut runtime, &snapshot, 5_000)
            .is_err());
        assert!(runtime.registry().bridges().next().is_none());
    }
}
