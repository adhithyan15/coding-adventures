//! AirGradient local monitor telemetry integration for D23.

#![forbid(unsafe_code)]

use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde_json::{Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_local_http::{
    LocalHttpEndpoint, LocalHttpError, LocalHttpMethod, LocalHttpRequestPlan,
    LocalHttpRequestTemplate, LocalHttpScheme,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "airgradient";
pub const PROTOCOL_ID: &str = "airgradient_local_api";
pub const MEASUREMENT_PATH: &str = "/measures/current";
pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub enum AirGradientError {
    Validation(String),
    LocalHttp(LocalHttpError),
    Url(UrlError),
    Io(String),
    Http(String),
    HttpStatus(u16),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    Json(serde_json::Error),
    MissingField(&'static str),
    NoMeasurements,
    Runtime(RuntimeError),
}

impl fmt::Display for AirGradientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid AirGradient input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid AirGradient URL: {error}"),
            Self::Io(message) => write!(formatter, "AirGradient LAN I/O failed: {message}"),
            Self::Http(message) => {
                write!(formatter, "invalid AirGradient HTTP response: {message}")
            }
            Self::HttpStatus(status) => {
                write!(formatter, "AirGradient endpoint returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "AirGradient response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "AirGradient response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid AirGradient JSON: {error}"),
            Self::MissingField(field) => {
                write!(formatter, "AirGradient response is missing {field}")
            }
            Self::NoMeasurements => {
                formatter.write_str("AirGradient response contains no numeric measurements")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for AirGradientError {}

impl From<LocalHttpError> for AirGradientError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for AirGradientError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for AirGradientError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for AirGradientError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AirGradientConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub display_name: String,
    pub expected_serial: Option<String>,
    pub timeout: Duration,
}

impl AirGradientConfig {
    pub fn new(bridge_id: BridgeId, base_url: impl Into<String>) -> Result<Self, AirGradientError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(AirGradientError::Validation(
                "base URL must be a credential-free HTTP origin".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            display_name: "AirGradient monitor".to_string(),
            expected_serial: None,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn from_serial(
        bridge_id: BridgeId,
        serial: impl Into<String>,
    ) -> Result<Self, AirGradientError> {
        let serial = validate_serial(serial.into())?;
        let mut config = Self::new(bridge_id, format!("http://airgradient_{serial}.local"))?;
        config.expected_serial = Some(serial);
        Ok(config)
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        let display_name = display_name.into();
        if !display_name.trim().is_empty() {
            self.display_name = display_name;
        }
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn endpoint(&self) -> Result<LocalHttpEndpoint, AirGradientError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(AirGradientError::MissingField("base URL host"))?;
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            LocalHttpScheme::Http,
            host.to_string(),
        )?
        .with_port(parsed.port.unwrap_or(DEFAULT_PORT))
        .with_metadata(Metadata::new("http.profile", "airgradient.local-api")))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirGradientMeasurement {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirGradientDeviceInfo {
    pub serial: String,
    pub model: String,
    pub firmware: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AirGradientSnapshot {
    pub device_info: AirGradientDeviceInfo,
    pub measurements: Vec<AirGradientMeasurement>,
}

pub fn discovery_record(
    config: &AirGradientConfig,
    snapshot: &AirGradientSnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, AirGradientError> {
    let endpoint = config.endpoint()?;
    let host = Url::parse(&config.base_url)?
        .host
        .ok_or(AirGradientError::MissingField("base URL host"))?;
    let source = if host.to_ascii_lowercase().ends_with(".local") {
        DiscoverySource::Mdns
    } else {
        DiscoverySource::Manual
    };
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&snapshot.device_info.serial),
        source,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| AirGradientError::Validation(error.to_string()))?
    .with_display_name(&config.display_name)
    .with_address(endpoint.origin())
    .with_hardware_model(&snapshot.device_info.model)
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("airgradient.serial", &snapshot.device_info.serial))
}

pub trait AirGradientTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError>;
}

#[derive(Debug, Clone)]
pub struct AirGradientLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for AirGradientLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl AirGradientTransport for AirGradientLanTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError> {
        let url = Url::parse(&plan.url)?;
        if url.scheme != "http" {
            return Err(AirGradientError::Validation(
                "AirGradient transport only permits local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or(AirGradientError::MissingField("URL host"))?;
        let port = url
            .effective_port()
            .ok_or(AirGradientError::MissingField("URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = encode_http_request(&url, plan)?;
        let mut stream = connect_tcp(host, port, timeout)?;
        stream
            .write_all(&request)
            .map_err(|error| AirGradientError::Io(error.to_string()))?;
        let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&bytes, self.maximum_response_bytes)
    }
}

pub struct AirGradientClient<T> {
    config: AirGradientConfig,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: AirGradientTransport> AirGradientClient<T> {
    pub fn new(config: AirGradientConfig, transport: T) -> Result<Self, AirGradientError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &AirGradientConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<AirGradientSnapshot, AirGradientError> {
        let data = self.request_json(MEASUREMENT_PATH)?;
        let snapshot = parse_snapshot(&data)?;
        if self
            .config
            .expected_serial
            .as_deref()
            .is_some_and(|expected| !expected.eq_ignore_ascii_case(&snapshot.device_info.serial))
        {
            return Err(AirGradientError::Validation(
                "monitor serial does not match the requested mDNS identity".to_string(),
            ));
        }
        Ok(snapshot)
    }

    fn request_json(&mut self, path: &str) -> Result<JsonValue, AirGradientError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
            .with_accept("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, Vec::new())?)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

impl<T> fmt::Debug for AirGradientClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AirGradientClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledAirGradientDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

pub struct AirGradientRuntimeIntegration<T> {
    client: AirGradientClient<T>,
}

impl<T: AirGradientTransport> AirGradientRuntimeIntegration<T> {
    pub fn new(client: AirGradientClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledAirGradientDevice, AirGradientError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &AirGradientSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledAirGradientDevice, AirGradientError> {
        if snapshot.measurements.is_empty() {
            return Err(AirGradientError::NoMeasurements);
        }
        let endpoint = self.client.config.endpoint()?;
        let native_id = stable_component(&snapshot.device_info.serial);
        if native_id.is_empty() {
            return Err(AirGradientError::Validation(
                "device serial does not contain a stable identifier".to_string(),
            ));
        }
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!("airgradient:{native_id}"));

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(endpoint.origin());
        bridge.hardware_model = Some(snapshot.device_info.model.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("http_endpoint", &endpoint.origin())?];
        bridge.metadata = vec![Metadata::new("airgradient.transport", "local_http_polling")];
        runtime.upsert_bridge(bridge)?;

        let entities = snapshot
            .measurements
            .iter()
            .map(|measurement| {
                let entity_id = EntityId::trusted(format!(
                    "airgradient:{native_id}:sensor:{}",
                    stable_component(&measurement.id)
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
                        Metadata::new("airgradient.measurement", measurement.id.clone()),
                        Metadata::new("airgradient.unit", measurement.unit.clone()),
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
            manufacturer: "AirGradient".to_string(),
            model: snapshot.device_info.model.clone(),
            name: self.client.config.display_name.clone(),
            serial: Some(snapshot.device_info.serial.clone()),
            firmware_version: Some(snapshot.device_info.firmware.clone()),
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier("serial", &snapshot.device_info.serial)?],
            health: Health::Online,
            metadata: Vec::new(),
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledAirGradientDevice {
            bridge_id,
            device_id,
            entity_ids,
        })
    }
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), AirGradientError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(AirGradientError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_snapshot(data: &JsonValue) -> Result<AirGradientSnapshot, AirGradientError> {
    let data = data
        .as_object()
        .ok_or(AirGradientError::MissingField("measurement object"))?;
    let device_info = AirGradientDeviceInfo {
        serial: validate_serial(required_string(data, "serialno")?)?,
        model: required_string(data, "model")?,
        firmware: required_string(data, "firmware")?,
    };
    let mut measurements = Vec::new();
    for definition in MEASUREMENT_DEFINITIONS {
        if let Some(value) = data
            .get(definition.field)
            .and_then(JsonValue::as_f64)
            .filter(|value| value.is_finite())
        {
            measurements.push(AirGradientMeasurement {
                id: definition.field.to_string(),
                name: definition.name.to_string(),
                value,
                unit: definition.unit.to_string(),
            });
        }
    }
    if measurements.is_empty() {
        return Err(AirGradientError::NoMeasurements);
    }
    Ok(AirGradientSnapshot {
        device_info,
        measurements,
    })
}

#[derive(Debug, Clone, Copy)]
struct MeasurementDefinition {
    field: &'static str,
    name: &'static str,
    unit: &'static str,
}

const MEASUREMENT_DEFINITIONS: &[MeasurementDefinition] = &[
    MeasurementDefinition {
        field: "rco2",
        name: "Carbon dioxide",
        unit: "ppm",
    },
    MeasurementDefinition {
        field: "pm01",
        name: "PM1.0",
        unit: "ug/m3",
    },
    MeasurementDefinition {
        field: "pm02",
        name: "PM2.5",
        unit: "ug/m3",
    },
    MeasurementDefinition {
        field: "pm02Compensated",
        name: "PM2.5 compensated",
        unit: "ug/m3",
    },
    MeasurementDefinition {
        field: "pm10",
        name: "PM10",
        unit: "ug/m3",
    },
    MeasurementDefinition {
        field: "atmp",
        name: "Temperature",
        unit: "C",
    },
    MeasurementDefinition {
        field: "atmpCompensated",
        name: "Temperature compensated",
        unit: "C",
    },
    MeasurementDefinition {
        field: "rhum",
        name: "Relative humidity",
        unit: "%",
    },
    MeasurementDefinition {
        field: "rhumCompensated",
        name: "Relative humidity compensated",
        unit: "%",
    },
    MeasurementDefinition {
        field: "tvocIndex",
        name: "TVOC index",
        unit: "index",
    },
    MeasurementDefinition {
        field: "noxIndex",
        name: "NOx index",
        unit: "index",
    },
    MeasurementDefinition {
        field: "pm003Count",
        name: "Particle count 0.3um",
        unit: "count/dL",
    },
    MeasurementDefinition {
        field: "wifi",
        name: "Wi-Fi signal",
        unit: "dBm",
    },
];

fn required_string(
    value: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, AirGradientError> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(AirGradientError::MissingField(field))
}

fn measurement_value(measurement: &AirGradientMeasurement) -> Value {
    Value::Object(vec![
        ("value".to_string(), Value::Number(measurement.value)),
        (
            "unit".to_string(),
            Value::Text(measurement.unit.to_string()),
        ),
    ])
}

fn validate_serial(serial: String) -> Result<String, AirGradientError> {
    let serial = serial.trim().to_ascii_lowercase();
    if serial.is_empty()
        || serial.len() > 64
        || !serial.chars().all(|value| value.is_ascii_alphanumeric())
    {
        return Err(AirGradientError::Validation(
            "serial must contain 1 to 64 ASCII letters or digits".to_string(),
        ));
    }
    Ok(serial)
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

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, AirGradientError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| AirGradientError::Validation(error.to_string()))
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

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn has_unsafe_http_text(value: &str) -> bool {
    value.contains(['\r', '\n', '\0'])
}

fn encode_http_request(
    url: &Url,
    plan: &LocalHttpRequestPlan,
) -> Result<Vec<u8>, AirGradientError> {
    let host = url
        .host
        .as_deref()
        .ok_or(AirGradientError::MissingField("URL host"))?;
    let port = url
        .effective_port()
        .ok_or(AirGradientError::MissingField("URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(AirGradientError::Validation(
            "request target contains unsafe HTTP text".to_string(),
        ));
    }
    let host_header = if port == DEFAULT_PORT {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    let mut request = format!(
        "{} {target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n",
        plan.method.as_str()
    )
    .into_bytes();
    let mut seen = BTreeSet::new();
    for header in &plan.headers {
        if has_unsafe_http_text(&header.name) || has_unsafe_http_text(&header.value) {
            return Err(AirGradientError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        seen.insert(header.name.to_ascii_lowercase());
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    if !seen.contains("content-length") {
        request.extend_from_slice(format!("Content-Length: {}\r\n", plan.body.len()).as_bytes());
    }
    request.extend_from_slice(b"\r\n");
    request.extend_from_slice(&plan.body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, AirGradientError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| AirGradientError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| AirGradientError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| AirGradientError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(AirGradientError::Io(
        last_error
            .unwrap_or_else(|| {
                io::Error::new(
                    io::ErrorKind::AddrNotAvailable,
                    "host resolved to no addresses",
                )
            })
            .to_string(),
    ))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, AirGradientError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| AirGradientError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(AirGradientError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, AirGradientError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| AirGradientError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(AirGradientError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(AirGradientError::TruncatedBody {
                    expected,
                    actual: input.len(),
                });
            }
            input[..expected].to_vec()
        }
        BodyKind::UntilEof => input.to_vec(),
        BodyKind::Chunked => decode_chunked(input, maximum)?,
    };
    if body.len() > maximum {
        return Err(AirGradientError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, AirGradientError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| AirGradientError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| AirGradientError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| AirGradientError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(AirGradientError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| AirGradientError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(AirGradientError::Http("truncated chunk".to_string()));
        }
        output.extend_from_slice(&input[cursor..chunk_end]);
        cursor = chunk_end + 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    const MEASUREMENTS: &str = r#"{"wifi":-46,"serialno":"ecda3b1eaaaf","rco2":447,"pm01":3,"pm02":7,"pm10":8,"pm003Count":442,"atmp":25.87,"atmpCompensated":24.47,"rhum":43,"rhumCompensated":49,"tvocIndex":100,"tvocRaw":33051,"noxIndex":1,"noxRaw":16307,"boot":6,"firmware":"3.1.3","model":"I-9PSL"}"#;

    fn response(body: &str) -> Vec<u8> {
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()
    }

    fn start_server(
        payloads: Vec<Vec<u8>>,
    ) -> (u16, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for payload in payloads {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0u8; 4096];
                loop {
                    let read = stream.read(&mut buffer).unwrap();
                    if read == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..read]);
                    if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(bytes).unwrap());
                stream.write_all(&payload).unwrap();
            }
        });
        (port, requests, handle)
    }

    fn config(port: u16) -> AirGradientConfig {
        AirGradientConfig::new(
            BridgeId::trusted("airgradient.test"),
            format!("http://127.0.0.1:{port}"),
        )
        .unwrap()
        .with_display_name("Living Room Air")
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:airgradient-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn documented_mdns_identity_becomes_verified_http_discovery() {
        let config = AirGradientConfig::from_serial(
            BridgeId::trusted("airgradient.discovery"),
            "ECDA3B1EAAAF",
        )
        .unwrap();
        assert_eq!(config.base_url, "http://airgradient_ecda3b1eaaaf.local");
        let snapshot = parse_snapshot(&serde_json::from_str(MEASUREMENTS).unwrap()).unwrap();
        let record = discovery_record(&config, &snapshot, 1_000).unwrap();
        assert_eq!(record.native_bridge_id, "ecda3b1eaaaf");
        assert_eq!(
            record.address.as_deref(),
            Some("http://airgradient_ecda3b1eaaaf.local")
        );
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.source, DiscoverySource::Mdns);
        assert_eq!(record.hardware_model.as_deref(), Some("I-9PSL"));
    }

    #[test]
    fn serial_validation_rejects_unsafe_mdns_names() {
        assert!(AirGradientConfig::from_serial(
            BridgeId::trusted("airgradient.discovery"),
            "../../monitor"
        )
        .is_err());
    }

    #[test]
    fn real_tcp_inspection_installs_environmental_sensors() {
        let (port, requests, handle) = start_server(vec![response(MEASUREMENTS)]);
        let client =
            AirGradientClient::new(config(port), AirGradientLanTransport::default()).unwrap();
        let mut integration = AirGradientRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:airgradient-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(installed.entity_ids.len(), 12);
        let co2 = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:rco2",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(co2.kind, EntityKind::Sensor);
        assert_eq!(co2.name, "Living Room Air Carbon dioxide");
        assert_eq!(
            co2.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        let device = runtime.registry().device(&installed.device_id).unwrap();
        assert_eq!(device.manufacturer, "AirGradient");
        assert_eq!(device.model, "I-9PSL");
        assert_eq!(device.serial.as_deref(), Some("ecda3b1eaaaf"));
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].contains(&format!("GET {MEASUREMENT_PATH} HTTP/1.1")));
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl AirGradientTransport for CountingTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[derive(Debug)]
    struct StaticTransport(Vec<u8>);

    impl AirGradientTransport for StaticTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, AirGradientError> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn discovered_serial_must_match_the_monitor_response() {
        let config = AirGradientConfig::from_serial(
            BridgeId::trusted("airgradient.mismatch"),
            "aaaaaaaaaaaa",
        )
        .unwrap();
        let mut client =
            AirGradientClient::new(config, StaticTransport(MEASUREMENTS.as_bytes().to_vec()))
                .unwrap();
        assert!(matches!(
            client.inspect(),
            Err(AirGradientError::Validation(message))
                if message.contains("does not match")
        ));
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = AirGradientClient::new(
            AirGradientConfig::new(BridgeId::trusted("airgradient.denied"), "http://127.0.0.1")
                .unwrap(),
            CountingTransport(Arc::clone(&calls)),
        )
        .unwrap();
        let mut integration = AirGradientRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(AirGradientError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn parser_requires_identity_and_known_measurements() {
        let missing_identity: JsonValue = serde_json::from_str(r#"{"rco2":447}"#).unwrap();
        assert!(matches!(
            parse_snapshot(&missing_identity),
            Err(AirGradientError::MissingField("serialno"))
        ));
        let empty: JsonValue = serde_json::from_str(
            r#"{"serialno":"ecda3b1eaaaf","model":"I-9PSL","firmware":"3.1.3"}"#,
        )
        .unwrap();
        assert!(matches!(
            parse_snapshot(&empty),
            Err(AirGradientError::NoMeasurements)
        ));
    }

    #[test]
    fn parser_assigns_documented_units_and_ignores_raw_diagnostics() {
        let data: JsonValue = serde_json::from_str(MEASUREMENTS).unwrap();
        let snapshot = parse_snapshot(&data).unwrap();
        assert_eq!(snapshot.device_info.model, "I-9PSL");
        assert!(snapshot
            .measurements
            .iter()
            .any(|measurement| measurement.id == "pm02" && measurement.unit == "ug/m3"));
        assert!(!snapshot
            .measurements
            .iter()
            .any(|measurement| measurement.id == "tvocRaw"));
    }

    #[test]
    fn response_bounds_are_enforced() {
        assert!(matches!(
            decode_http_response(&response("{}"), 1),
            Err(AirGradientError::ResponseTooLarge { limit: 1 })
        ));
    }
}
