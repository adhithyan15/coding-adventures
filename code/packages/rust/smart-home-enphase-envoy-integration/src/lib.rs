//! Authenticated local Enphase IQ Gateway meter telemetry for D23.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::{Zeroize, Zeroizing};
use http1::{parse_response_head, Http1ParseError};
use http_core::{BodyKind, Header};
use serde_json::{Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind, VaultRef,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_local_http::{
    LocalHttpAuth, LocalHttpEndpoint, LocalHttpError, LocalHttpMethod, LocalHttpRequestPlan,
    LocalHttpRequestTemplate, LocalHttpScheme,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig, TlsConnector};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "enphase_envoy";
pub const PROTOCOL_ID: &str = "enphase_iq_gateway_local_api";
pub const METERS_PATH: &str = "/ivp/meters";
pub const METER_READINGS_PATH: &str = "/ivp/meters/readings";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_METERS: usize = 16;
const MAX_SECRET_BYTES: usize = 16 * 1024;
const MAX_TEXT_BYTES: usize = 1_024;

#[derive(Debug)]
pub enum EnphaseError {
    Validation(String),
    LocalHttp(LocalHttpError),
    Url(UrlError),
    Io(String),
    Tls(String),
    Http(String),
    HttpStatus {
        operation: &'static str,
        status: u16,
    },
    ResponseTooLarge {
        limit: usize,
    },
    TruncatedBody {
        expected: usize,
        actual: usize,
    },
    Json(serde_json::Error),
    MissingField(&'static str),
    Runtime(RuntimeError),
}

impl fmt::Display for EnphaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Enphase input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Enphase URL: {error}"),
            Self::Io(message) => write!(formatter, "Enphase LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "Enphase TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Enphase HTTP response: {message}"),
            Self::HttpStatus { operation, status } => {
                write!(formatter, "Enphase {operation} returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Enphase response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Enphase response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Enphase JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "Enphase response is missing {field}"),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for EnphaseError {}

impl From<LocalHttpError> for EnphaseError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for EnphaseError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for EnphaseError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for EnphaseError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct EnphaseAccessToken {
    token: Zeroizing<String>,
}

impl EnphaseAccessToken {
    pub fn new(token: impl Into<String>) -> Result<Self, EnphaseError> {
        let token = token.into();
        if token.trim().is_empty()
            || token.len() > MAX_SECRET_BYTES
            || token.bytes().any(|byte| byte <= 0x20 || byte == 0x7f)
        {
            return Err(EnphaseError::Validation(
                "access token must be bounded non-whitespace HTTP text".to_string(),
            ));
        }
        Ok(Self {
            token: Zeroizing::new(token),
        })
    }
}

impl fmt::Debug for EnphaseAccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EnphaseAccessToken([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnphaseConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub gateway_serial: String,
    pub token_ref: VaultRef,
    pub timeout: Duration,
}

impl EnphaseConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        gateway_serial: impl Into<String>,
        token_ref: VaultRef,
    ) -> Result<Self, EnphaseError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(EnphaseError::MissingField("base URL host"))?;
        let secure = parsed.scheme == "https";
        let test_loopback = parsed.scheme == "http" && is_loopback_host(host);
        if (!secure && !test_loopback)
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(EnphaseError::Validation(
                "base URL must be a credential-free HTTPS origin; HTTP is test-only on loopback"
                    .to_string(),
            ));
        }
        let gateway_serial = bounded_text(gateway_serial.into(), "gateway serial")?;
        if !gateway_serial
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            return Err(EnphaseError::Validation(
                "gateway serial must contain only decimal digits".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            gateway_serial,
            token_ref,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn endpoint(&self) -> Result<LocalHttpEndpoint, EnphaseError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(EnphaseError::MissingField("base URL host"))?;
        let scheme = match parsed.scheme.as_str() {
            "https" => LocalHttpScheme::Https,
            "http" if is_loopback_host(host) => LocalHttpScheme::Http,
            _ => {
                return Err(EnphaseError::Validation(
                    "Enphase endpoint is not approved".to_string(),
                ))
            }
        };
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            scheme,
            host.to_string(),
        )?
        .with_port(parsed.port.unwrap_or_else(|| scheme.default_port()))
        .with_metadata(Metadata::new(
            "http.profile",
            "enphase.iq-gateway.local-api",
        )))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnphaseMeter {
    pub eid: u64,
    pub state: String,
    pub measurement_type: String,
    pub phase_mode: String,
    pub phase_count: u64,
    pub metering_status: String,
    pub status_flags: Vec<String>,
    pub timestamp: u64,
    pub active_energy_delivered_wh: f64,
    pub active_energy_received_wh: f64,
    pub instantaneous_demand_w: f64,
    pub active_power_w: f64,
    pub apparent_power_va: f64,
    pub reactive_power_var: f64,
    pub power_factor: f64,
    pub voltage_v: f64,
    pub current_a: f64,
    pub frequency_hz: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnphaseSnapshot {
    pub meters: Vec<EnphaseMeter>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnphaseRequestPlans {
    pub meters: LocalHttpRequestPlan,
    pub readings: LocalHttpRequestPlan,
}

pub trait EnphaseTransport {
    fn inspect(
        &mut self,
        plans: &EnphaseRequestPlans,
        token: &EnphaseAccessToken,
    ) -> Result<EnphaseSnapshot, EnphaseError>;
}

pub struct EnphaseLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    maximum_response_bytes: usize,
}

impl Default for EnphaseLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl EnphaseLanTransport {
    pub fn new(connector: Box<dyn TlsConnector>, tls_config: TlsConfig) -> Self {
        Self {
            connector,
            tls_config,
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    pub fn with_maximum_response_bytes(mut self, maximum: usize) -> Self {
        self.maximum_response_bytes = maximum.max(1);
        self
    }

    fn request(
        &mut self,
        plan: &LocalHttpRequestPlan,
        token: &EnphaseAccessToken,
    ) -> Result<HttpResponse, EnphaseError> {
        let request = Zeroizing::new(encode_http_request(plan, token.token.as_str())?);
        let url = Url::parse(&plan.url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(EnphaseError::MissingField("request URL host"))?;
        let port = url
            .effective_port()
            .ok_or(EnphaseError::MissingField("request URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let response = match url.scheme.as_str() {
            "http" if is_loopback_host(host) => {
                let mut stream = connect_tcp(host, port, timeout)?;
                write_request(&mut stream, request.as_slice())?;
                Zeroizing::new(read_bounded(&mut stream, self.maximum_response_bytes)?)
            }
            "https" => {
                let mut config = self.tls_config.clone();
                config.connect_timeout = timeout;
                config.read_timeout = Some(timeout);
                config.write_timeout = Some(timeout);
                let mut stream = self
                    .connector
                    .connect(host, port, &config)
                    .map_err(|error| EnphaseError::Tls(error.to_string()))?;
                write_request(&mut stream, request.as_slice())?;
                let bytes = Zeroizing::new(read_bounded(&mut stream, self.maximum_response_bytes)?);
                stream
                    .close_notify()
                    .map_err(|error| EnphaseError::Tls(error.to_string()))?;
                bytes
            }
            _ => {
                return Err(EnphaseError::Validation(
                    "Enphase transport requires HTTPS or loopback HTTP".to_string(),
                ))
            }
        };
        decode_http_response(response.as_slice(), self.maximum_response_bytes)
    }

    fn get_json(
        &mut self,
        plan: &LocalHttpRequestPlan,
        token: &EnphaseAccessToken,
        operation: &'static str,
    ) -> Result<JsonValue, EnphaseError> {
        let response = self.request(plan, token)?;
        if response.status != 200 {
            return Err(EnphaseError::HttpStatus {
                operation,
                status: response.status,
            });
        }
        Ok(serde_json::from_slice(&response.body)?)
    }
}

impl EnphaseTransport for EnphaseLanTransport {
    fn inspect(
        &mut self,
        plans: &EnphaseRequestPlans,
        token: &EnphaseAccessToken,
    ) -> Result<EnphaseSnapshot, EnphaseError> {
        let meters = self.get_json(&plans.meters, token, "meter inventory")?;
        let readings = self.get_json(&plans.readings, token, "meter readings")?;
        parse_snapshot(&meters, &readings)
    }
}

pub struct EnphaseClient<T> {
    config: EnphaseConfig,
    token: EnphaseAccessToken,
    transport: T,
    plans: EnphaseRequestPlans,
}

impl<T: EnphaseTransport> EnphaseClient<T> {
    pub fn new(
        config: EnphaseConfig,
        token: EnphaseAccessToken,
        transport: T,
    ) -> Result<Self, EnphaseError> {
        let endpoint = config.endpoint()?;
        let timeout_ms = duration_ms(config.timeout);
        let meters = get_plan(&endpoint, &config.token_ref, METERS_PATH, timeout_ms)?;
        let readings = get_plan(
            &endpoint,
            &config.token_ref,
            METER_READINGS_PATH,
            timeout_ms,
        )?;
        Ok(Self {
            config,
            token,
            transport,
            plans: EnphaseRequestPlans { meters, readings },
        })
    }

    pub fn inspect(&mut self) -> Result<EnphaseSnapshot, EnphaseError> {
        self.transport.inspect(&self.plans, &self.token)
    }
}

impl<T> fmt::Debug for EnphaseClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EnphaseClient")
            .field("config", &self.config)
            .field("token", &"[REDACTED]")
            .field("plans", &self.plans)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledEnphaseGateway {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub meter_entity_ids: Vec<EntityId>,
}

pub struct EnphaseRuntimeIntegration<T> {
    client: EnphaseClient<T>,
}

impl<T: EnphaseTransport> EnphaseRuntimeIntegration<T> {
    pub fn new(client: EnphaseClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledEnphaseGateway, EnphaseError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

pub fn paired_discovery_record(
    config: &EnphaseConfig,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, EnphaseError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&config.gateway_serial),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| EnphaseError::Validation(error.to_string()))?
    .with_display_name("Enphase IQ Gateway")
    .with_address(config.base_url.clone())
    .with_hardware_model("IQ Gateway")
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("enphase.protocol", PROTOCOL_ID)
    .with_metadata("enphase.gateway_serial", config.gateway_serial.clone()))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &EnphaseConfig,
    snapshot: &EnphaseSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledEnphaseGateway, EnphaseError> {
    if snapshot.meters.is_empty() {
        return Err(EnphaseError::Validation(
            "meter snapshot must not be empty".to_string(),
        ));
    }
    let serial_component = stable_component(&config.gateway_serial);
    let device_id = DeviceId::trusted(format!("enphase:{serial_component}"));
    let health = aggregate_health(&snapshot.meters);
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some("IQ Gateway".to_string());
    bridge.auth_ref = Some(config.token_ref.clone());
    bridge.health = health;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("https_endpoint", &config.base_url)?];
    bridge.metadata = vec![
        Metadata::new("enphase.transport", "local_bearer_token"),
        Metadata::new("enphase.meter_count", snapshot.meters.len().to_string()),
    ];
    runtime.upsert_bridge(bridge)?;

    let meter_entity_ids = snapshot
        .meters
        .iter()
        .map(|meter| EntityId::trusted(format!("enphase:{serial_component}:meter:{}", meter.eid)))
        .collect::<Vec<_>>();
    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: "Enphase".to_string(),
        model: "IQ Gateway".to_string(),
        name: "Enphase IQ Gateway".to_string(),
        serial: Some(config.gateway_serial.clone()),
        firmware_version: None,
        room_id: None,
        entity_ids: meter_entity_ids.clone(),
        identifiers: vec![protocol_identifier(
            "gateway_serial",
            &config.gateway_serial,
        )?],
        health,
        metadata: vec![Metadata::new(
            "enphase.native_meter_count",
            snapshot.meters.len().to_string(),
        )],
    })?;
    for (meter, entity_id) in snapshot.meters.iter().zip(&meter_entity_ids) {
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Sensor,
            name: format!("Enphase {} meter", display_name(&meter.measurement_type)),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("sensor.measurement"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: meter_value(meter),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![
                Metadata::new("enphase.eid", meter.eid.to_string()),
                Metadata::new("enphase.measurement_type", &meter.measurement_type),
                Metadata::new("enphase.native_state", &meter.state),
                Metadata::new("enphase.metering_status", &meter.metering_status),
            ],
        })?;
    }
    Ok(InstalledEnphaseGateway {
        bridge_id: config.bridge_id.clone(),
        device_id,
        meter_entity_ids,
    })
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), EnphaseError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(EnphaseError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_snapshot(
    meters: &JsonValue,
    readings: &JsonValue,
) -> Result<EnphaseSnapshot, EnphaseError> {
    let meters = meters
        .as_array()
        .ok_or(EnphaseError::MissingField("meter inventory array"))?;
    let readings = readings
        .as_array()
        .ok_or(EnphaseError::MissingField("meter readings array"))?;
    if meters.is_empty() || meters.len() > MAX_METERS || readings.len() > MAX_METERS {
        return Err(EnphaseError::Validation(format!(
            "meter inventory must contain 1-{MAX_METERS} entries"
        )));
    }
    let mut readings_by_eid = BTreeMap::new();
    for reading in readings {
        let reading = reading
            .as_object()
            .ok_or(EnphaseError::MissingField("meter reading object"))?;
        let eid = required_u64(reading, "eid")?;
        if readings_by_eid.insert(eid, reading).is_some() {
            return Err(EnphaseError::Validation(format!(
                "duplicate meter reading EID {eid}"
            )));
        }
    }
    let mut seen = BTreeSet::new();
    let mut parsed = Vec::with_capacity(meters.len());
    for meter in meters {
        let meter = meter
            .as_object()
            .ok_or(EnphaseError::MissingField("meter inventory object"))?;
        let eid = required_u64(meter, "eid")?;
        if !seen.insert(eid) {
            return Err(EnphaseError::Validation(format!(
                "duplicate meter inventory EID {eid}"
            )));
        }
        let reading = readings_by_eid.remove(&eid).ok_or_else(|| {
            EnphaseError::Validation(format!("missing reading for meter EID {eid}"))
        })?;
        parsed.push(parse_meter(meter, reading)?);
    }
    if let Some(unknown) = readings_by_eid.keys().next() {
        return Err(EnphaseError::Validation(format!(
            "reading references unknown meter EID {unknown}"
        )));
    }
    parsed.sort_by_key(|meter| meter.eid);
    Ok(EnphaseSnapshot { meters: parsed })
}

fn parse_meter(
    meter: &JsonMap<String, JsonValue>,
    reading: &JsonMap<String, JsonValue>,
) -> Result<EnphaseMeter, EnphaseError> {
    Ok(EnphaseMeter {
        eid: required_u64(meter, "eid")?,
        state: normalized_text(meter, "state")?,
        measurement_type: normalized_text(meter, "measurementType")?,
        phase_mode: normalized_text(meter, "phaseMode")?,
        phase_count: required_u64(meter, "phaseCount")?,
        metering_status: normalized_text(meter, "meteringStatus")?,
        status_flags: string_array(meter, "statusFlags")?,
        timestamp: required_u64(reading, "timestamp")?,
        active_energy_delivered_wh: required_f64(reading, "actEnergyDlvd")?,
        active_energy_received_wh: required_f64(reading, "actEnergyRcvd")?,
        instantaneous_demand_w: required_f64(reading, "instantaneousDemand")?,
        active_power_w: required_f64(reading, "activePower")?,
        apparent_power_va: required_f64(reading, "apparentPower")?,
        reactive_power_var: required_f64(reading, "reactivePower")?,
        power_factor: required_f64(reading, "pwrFactor")?,
        voltage_v: required_f64(reading, "voltage")?,
        current_a: required_f64(reading, "current")?,
        frequency_hz: required_f64(reading, "freq")?,
    })
}

fn required_u64(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<u64, EnphaseError> {
    object
        .get(field)
        .and_then(JsonValue::as_u64)
        .ok_or(EnphaseError::MissingField(field))
}

fn required_f64(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<f64, EnphaseError> {
    object
        .get(field)
        .and_then(JsonValue::as_f64)
        .filter(|value| value.is_finite())
        .ok_or(EnphaseError::MissingField(field))
}

fn normalized_text(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, EnphaseError> {
    let text = object
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or(EnphaseError::MissingField(field))?
        .trim()
        .to_ascii_lowercase();
    bounded_text(text, field)
}

fn string_array(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Vec<String>, EnphaseError> {
    let values = object
        .get(field)
        .and_then(JsonValue::as_array)
        .ok_or(EnphaseError::MissingField(field))?;
    if values.len() > 64 {
        return Err(EnphaseError::Validation(format!(
            "{field} contains too many values"
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(bounded_text(
            value
                .as_str()
                .ok_or(EnphaseError::MissingField(field))?
                .trim()
                .to_ascii_lowercase(),
            field,
        )?);
    }
    output.sort();
    output.dedup();
    Ok(output)
}

fn bounded_text(value: String, field: &'static str) -> Result<String, EnphaseError> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES || value.contains(['\r', '\n', '\0'])
    {
        return Err(EnphaseError::Validation(format!(
            "{field} must be bounded non-empty text"
        )));
    }
    Ok(value)
}

fn aggregate_health(meters: &[EnphaseMeter]) -> Health {
    if meters
        .iter()
        .all(|meter| meter_health(meter) == Health::Offline)
    {
        Health::Offline
    } else if meters
        .iter()
        .any(|meter| meter_health(meter) != Health::Online)
    {
        Health::Degraded
    } else {
        Health::Online
    }
}

fn meter_health(meter: &EnphaseMeter) -> Health {
    if meter.state != "enabled" {
        Health::Offline
    } else if meter.metering_status == "normal" && meter.status_flags.is_empty() {
        Health::Online
    } else {
        Health::Degraded
    }
}

fn meter_value(meter: &EnphaseMeter) -> Value {
    Value::Object(vec![
        ("eid".to_string(), Value::Number(meter.eid as f64)),
        ("state".to_string(), Value::Text(meter.state.clone())),
        (
            "measurement_type".to_string(),
            Value::Text(meter.measurement_type.clone()),
        ),
        (
            "phase_mode".to_string(),
            Value::Text(meter.phase_mode.clone()),
        ),
        (
            "phase_count".to_string(),
            Value::Number(meter.phase_count as f64),
        ),
        (
            "metering_status".to_string(),
            Value::Text(meter.metering_status.clone()),
        ),
        (
            "status_flags".to_string(),
            Value::Array(
                meter
                    .status_flags
                    .iter()
                    .cloned()
                    .map(Value::Text)
                    .collect(),
            ),
        ),
        (
            "timestamp_s".to_string(),
            Value::Number(meter.timestamp as f64),
        ),
        (
            "active_energy_delivered_wh".to_string(),
            Value::Number(meter.active_energy_delivered_wh),
        ),
        (
            "active_energy_received_wh".to_string(),
            Value::Number(meter.active_energy_received_wh),
        ),
        (
            "instantaneous_demand_w".to_string(),
            Value::Number(meter.instantaneous_demand_w),
        ),
        (
            "active_power_w".to_string(),
            Value::Number(meter.active_power_w),
        ),
        (
            "apparent_power_va".to_string(),
            Value::Number(meter.apparent_power_va),
        ),
        (
            "reactive_power_var".to_string(),
            Value::Number(meter.reactive_power_var),
        ),
        (
            "power_factor".to_string(),
            Value::Number(meter.power_factor),
        ),
        ("voltage_v".to_string(), Value::Number(meter.voltage_v)),
        ("current_a".to_string(), Value::Number(meter.current_a)),
        (
            "frequency_hz".to_string(),
            Value::Number(meter.frequency_hz),
        ),
    ])
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, EnphaseError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| EnphaseError::Validation(error.to_string()))
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

fn display_name(value: &str) -> String {
    let words = value.replace(['_', '-'], " ");
    let mut characters = words.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn get_plan(
    endpoint: &LocalHttpEndpoint,
    token_ref: &VaultRef,
    path: &str,
    timeout_ms: u64,
) -> Result<LocalHttpRequestPlan, EnphaseError> {
    Ok(LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
        .with_accept("application/json")
        .with_timeout_ms(timeout_ms)
        .with_auth(LocalHttpAuth::BearerToken {
            vault_ref: token_ref.clone(),
        })
        .plan(endpoint, Vec::new())?)
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn encode_http_request(plan: &LocalHttpRequestPlan, token: &str) -> Result<Vec<u8>, EnphaseError> {
    if token.is_empty()
        || token.len() > MAX_SECRET_BYTES
        || token.bytes().any(|byte| byte <= 0x20 || byte == 0x7f)
    {
        return Err(EnphaseError::Validation(
            "access token is unsafe for an HTTP header".to_string(),
        ));
    }
    let url = Url::parse(&plan.url)?;
    let host = url
        .host
        .as_deref()
        .ok_or(EnphaseError::MissingField("request URL host"))?;
    let port = url
        .effective_port()
        .ok_or(EnphaseError::MissingField("request URL port"))?;
    let mut target = if url.path.is_empty() {
        "/".to_string()
    } else {
        url.path.clone()
    };
    if let Some(query) = &url.query {
        target.push('?');
        target.push_str(query);
    }
    if host.contains(['\r', '\n', '\0']) || target.contains(['\r', '\n', '\0']) {
        return Err(EnphaseError::Validation(
            "request target contains unsafe HTTP text".to_string(),
        ));
    }
    let default_port = if url.scheme == "https" { 443 } else { 80 };
    let host_header = if port == default_port {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    let mut request = format!(
        "{} {target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n",
        plan.method.as_str()
    )
    .into_bytes();
    for header in &plan.headers {
        if header.name.eq_ignore_ascii_case("Content-Length")
            || header.name.eq_ignore_ascii_case("Authorization")
        {
            continue;
        }
        if header.name.contains(['\r', '\n', '\0']) || header.value.contains(['\r', '\n', '\0']) {
            return Err(EnphaseError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    request.extend_from_slice(
        format!("Authorization: Bearer {token}\r\nContent-Length: 0\r\n\r\n").as_bytes(),
    );
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, EnphaseError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| EnphaseError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| EnphaseError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| EnphaseError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(EnphaseError::Io(
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "no socket addresses resolved".to_string()),
    ))
}

fn write_request<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<(), EnphaseError> {
    writer
        .write_all(bytes)
        .map_err(|error| EnphaseError::Io(error.to_string()))?;
    writer
        .flush()
        .map_err(|error| EnphaseError::Io(error.to_string()))
}

fn read_bounded<R: Read>(reader: &mut R, maximum: usize) -> Result<Vec<u8>, EnphaseError> {
    let mut output = Vec::new();
    let mut chunk = [0u8; 8 * 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(output),
            Ok(count) => {
                if output.len().saturating_add(count) > maximum {
                    output.zeroize();
                    return Err(EnphaseError::ResponseTooLarge { limit: maximum });
                }
                output.extend_from_slice(&chunk[..count]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(output)
            }
            Err(error) => return Err(EnphaseError::Io(error.to_string())),
        }
    }
}

struct HttpResponse {
    status: u16,
    headers: Vec<Header>,
    body: Vec<u8>,
}

impl Drop for HttpResponse {
    fn drop(&mut self) {
        for header in &mut self.headers {
            header.value.zeroize();
        }
        self.body.zeroize();
    }
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<HttpResponse, EnphaseError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| EnphaseError::Http(error.to_string()))?;
    let status = parsed.head.status;
    let mut headers = parsed.head.headers;
    let input = &bytes[parsed.body_offset..];
    let body = match (|| {
        let body = match parsed.body_kind {
            BodyKind::None => Vec::new(),
            BodyKind::ContentLength(expected) => {
                if input.len() < expected {
                    return Err(EnphaseError::TruncatedBody {
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
            return Err(EnphaseError::ResponseTooLarge { limit: maximum });
        }
        Ok(body)
    })() {
        Ok(body) => body,
        Err(error) => {
            for header in &mut headers {
                header.value.zeroize();
            }
            return Err(error);
        }
    };
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

fn decode_chunked(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, EnphaseError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let line_end = find_crlf(bytes, cursor)
            .ok_or_else(|| EnphaseError::Http("incomplete chunk size".to_string()))?;
        let size_text = std::str::from_utf8(&bytes[cursor..line_end])
            .map_err(|_| EnphaseError::Http("chunk size is not ASCII".to_string()))?;
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or_default(), 16)
            .map_err(|_| EnphaseError::Http("invalid chunk size".to_string()))?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(size) > maximum {
            output.zeroize();
            return Err(EnphaseError::ResponseTooLarge { limit: maximum });
        }
        let end = cursor
            .checked_add(size)
            .ok_or_else(|| EnphaseError::Http("chunk length overflow".to_string()))?;
        if end.saturating_add(2) > bytes.len() || &bytes[end..end + 2] != b"\r\n" {
            output.zeroize();
            return Err(EnphaseError::Http("truncated chunk".to_string()));
        }
        output.extend_from_slice(&bytes[cursor..end]);
        cursor = end + 2;
    }
}

fn find_crlf(bytes: &[u8], start: usize) -> Option<usize> {
    bytes
        .get(start..)?
        .windows(2)
        .position(|window| window == b"\r\n")
        .map(|offset| start + offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    const TOKEN: &str = "eyJhbGciOiJFUzI1NiJ9.test.signature";

    fn config(base_url: &str) -> EnphaseConfig {
        EnphaseConfig::new(
            BridgeId::trusted("enphase:bridge"),
            base_url,
            "122233344455",
            VaultRef::trusted("vault://smart-home/enphase/token"),
        )
        .unwrap()
    }

    fn meter_inventory() -> JsonValue {
        serde_json::json!([
            {
                "eid": 704643328_u64,
                "state": "enabled",
                "measurementType": "production",
                "phaseMode": "split",
                "phaseCount": 2,
                "meteringStatus": "normal",
                "statusFlags": []
            },
            {
                "eid": 704643584_u64,
                "state": "enabled",
                "measurementType": "net-consumption",
                "phaseMode": "split",
                "phaseCount": 2,
                "meteringStatus": "warning",
                "statusFlags": ["phase-imbalance"]
            }
        ])
    }

    fn meter_readings() -> JsonValue {
        serde_json::json!([
            {
                "eid": 704643584_u64,
                "timestamp": 1654218661_u64,
                "actEnergyDlvd": 48540.732,
                "actEnergyRcvd": 1244797.861,
                "instantaneousDemand": -0.0,
                "activePower": -0.0,
                "apparentPower": 34.831,
                "reactivePower": -0.0,
                "pwrFactor": 0.0,
                "voltage": 246.338,
                "current": 0.283,
                "freq": 59.188
            },
            {
                "eid": 704643328_u64,
                "timestamp": 1654218661_u64,
                "actEnergyDlvd": 1608426.912,
                "actEnergyRcvd": 4.923,
                "instantaneousDemand": 132.118,
                "activePower": 132.118,
                "apparentPower": 5328.778,
                "reactivePower": -5328.778,
                "pwrFactor": 0.025,
                "voltage": 246.377,
                "current": 43.257,
                "freq": 59.188
            }
        ])
    }

    fn snapshot() -> EnphaseSnapshot {
        parse_snapshot(&meter_inventory(), &meter_readings()).unwrap()
    }

    fn authorized_runtime() -> (SmartHomeRuntime, AgentId) {
        let principal_id = AgentId::trusted("agent:energy");
        let mut runtime = SmartHomeRuntime::new();
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:enphase-test"),
                principal_id.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
        (runtime, principal_id)
    }

    #[test]
    fn production_config_requires_https_and_decimal_serial() {
        assert!(EnphaseConfig::new(
            BridgeId::trusted("bad"),
            "http://envoy.local",
            "122233344455",
            VaultRef::trusted("vault://token"),
        )
        .is_err());
        assert!(EnphaseConfig::new(
            BridgeId::trusted("bad"),
            "https://envoy.local/path",
            "122233344455",
            VaultRef::trusted("vault://token"),
        )
        .is_err());
        assert!(EnphaseConfig::new(
            BridgeId::trusted("bad"),
            "https://envoy.local",
            "serial-1",
            VaultRef::trusted("vault://token"),
        )
        .is_err());
        assert!(config("https://envoy.local").endpoint().is_ok());
    }

    #[test]
    fn parser_matches_native_eids_and_rejects_identity_drift() {
        let parsed = snapshot();
        assert_eq!(parsed.meters.len(), 2);
        assert_eq!(parsed.meters[0].eid, 704643328);
        assert_eq!(parsed.meters[1].measurement_type, "net-consumption");

        let mut readings = meter_readings();
        readings.as_array_mut().unwrap()[0]["eid"] = serde_json::json!(999_u64);
        assert!(parse_snapshot(&meter_inventory(), &readings)
            .unwrap_err()
            .to_string()
            .contains("missing reading"));
    }

    #[test]
    fn token_and_client_debug_are_redacted() {
        let token = EnphaseAccessToken::new(TOKEN).unwrap();
        assert!(!format!("{token:?}").contains(TOKEN));
        let client =
            EnphaseClient::new(config("http://127.0.0.1:1"), token, FixedTransport).unwrap();
        let debug = format!("{client:?}");
        assert!(!debug.contains(TOKEN));
        assert!(debug.contains("[REDACTED]"));
    }

    struct FixedTransport;

    impl EnphaseTransport for FixedTransport {
        fn inspect(
            &mut self,
            _plans: &EnphaseRequestPlans,
            _token: &EnphaseAccessToken,
        ) -> Result<EnphaseSnapshot, EnphaseError> {
            Ok(snapshot())
        }
    }

    #[test]
    fn authorized_snapshot_installs_confirmed_meter_state() {
        let (mut runtime, principal) = authorized_runtime();
        let client = EnphaseClient::new(
            config("http://127.0.0.1:1"),
            EnphaseAccessToken::new(TOKEN).unwrap(),
            FixedTransport,
        )
        .unwrap();
        let installed = EnphaseRuntimeIntegration::new(client)
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        assert_eq!(installed.meter_entity_ids.len(), 2);
        assert_eq!(
            runtime
                .registry()
                .device(&installed.device_id)
                .unwrap()
                .health,
            Health::Degraded
        );
        let state = runtime
            .registry()
            .entity(&installed.meter_entity_ids[0])
            .unwrap()
            .state
            .as_ref()
            .unwrap();
        assert_eq!(state.confidence, StateConfidence::Confirmed);
        assert_eq!(state.source, StateSource::Poll);
    }

    struct CountingTransport {
        calls: usize,
    }

    impl EnphaseTransport for CountingTransport {
        fn inspect(
            &mut self,
            _plans: &EnphaseRequestPlans,
            _token: &EnphaseAccessToken,
        ) -> Result<EnphaseSnapshot, EnphaseError> {
            self.calls += 1;
            Ok(snapshot())
        }
    }

    #[test]
    fn unauthorized_read_stops_before_transport() {
        let principal = AgentId::trusted("agent:denied");
        let mut runtime = SmartHomeRuntime::new();
        let client = EnphaseClient::new(
            config("http://127.0.0.1:1"),
            EnphaseAccessToken::new(TOKEN).unwrap(),
            CountingTransport { calls: 0 },
        )
        .unwrap();
        let mut integration = EnphaseRuntimeIntegration::new(client);
        assert!(integration
            .inspect_and_install_authorized(&mut runtime, principal, 1_000)
            .is_err());
        assert_eq!(integration.client.transport.calls, 0);
    }

    #[test]
    fn bounded_reader_rejects_oversized_payloads() {
        let mut bytes = &b"abcdef"[..];
        assert!(matches!(
            read_bounded(&mut bytes, 4),
            Err(EnphaseError::ResponseTooLarge { limit: 4 })
        ));
    }

    #[test]
    fn loopback_transport_sends_exact_bearer_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [meter_inventory(), meter_readings()] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let request = read_request(&mut stream);
                requests.push(request);
                let body = serde_json::to_vec(&response).unwrap();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
            sender.send(requests).unwrap();
        });

        let config = config(&format!("http://{address}"));
        let token_ref_text = config.token_ref.as_str().to_string();
        let mut client = EnphaseClient::new(
            config,
            EnphaseAccessToken::new(TOKEN).unwrap(),
            EnphaseLanTransport::default(),
        )
        .unwrap();
        let observed = client.inspect().unwrap();
        assert_eq!(observed.meters.len(), 2);
        server.join().unwrap();
        let requests = receiver.recv().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /ivp/meters HTTP/1.1\r\n"));
        assert!(requests[1].starts_with("GET /ivp/meters/readings HTTP/1.1\r\n"));
        for request in requests {
            assert!(request.contains(&format!("Authorization: Bearer {TOKEN}\r\n")));
            assert!(request.contains("Accept: application/json\r\n"));
            assert!(!request.contains(&token_ref_text));
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let count = stream.read(&mut chunk).unwrap();
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8(bytes).unwrap()
    }
}
