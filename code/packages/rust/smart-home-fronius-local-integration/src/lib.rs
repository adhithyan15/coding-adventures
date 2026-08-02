//! Fronius Solar API v1 local telemetry integration for D23.

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
    run_mdns_ipv4_scan, DiscoveryConfidence, DiscoveryRecord, DiscoverySource, MdnsAdvertisement,
    MdnsScanOptions, MdnsScanResult, PairingRequirement,
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
pub const INTEGRATION_ID: &str = "fronius";
pub const PROTOCOL_ID: &str = "fronius_solar_api_v1";
pub const MDNS_SERVICE_TYPE: &str = "_http._tcp.local";
pub const POWER_FLOW_PATH: &str = "/solar_api/v1/GetPowerFlowRealtimeData.fcgi";
pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_MAX_DISCOVERY_RESPONSES: usize = 128;

#[derive(Debug)]
pub enum FroniusError {
    Validation(String),
    Discovery(String),
    LocalHttp(LocalHttpError),
    Url(UrlError),
    Io(String),
    Http(String),
    HttpStatus(u16),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    Json(serde_json::Error),
    MissingField(&'static str),
    ApiStatus { code: i64, message: String },
    NoMeasurements,
    Runtime(RuntimeError),
}

impl fmt::Display for FroniusError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Fronius input: {message}"),
            Self::Discovery(message) => write!(formatter, "Fronius discovery failed: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Fronius URL: {error}"),
            Self::Io(message) => write!(formatter, "Fronius LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Fronius HTTP response: {message}"),
            Self::HttpStatus(status) => {
                write!(formatter, "Fronius endpoint returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Fronius response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Fronius response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Fronius JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "Fronius response is missing {field}"),
            Self::ApiStatus { code, message } => {
                write!(
                    formatter,
                    "Fronius Solar API returned status {code}: {message}"
                )
            }
            Self::NoMeasurements => {
                formatter.write_str("Fronius Power Flow response contains no measurements")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for FroniusError {}

impl From<LocalHttpError> for FroniusError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for FroniusError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for FroniusError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for FroniusError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FroniusConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub display_name: String,
    pub timeout: Duration,
}

impl FroniusConfig {
    pub fn new(bridge_id: BridgeId, base_url: impl Into<String>) -> Result<Self, FroniusError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(FroniusError::Validation(
                "base URL must be a credential-free HTTP origin".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            display_name: "Fronius Solar System".to_string(),
            timeout: Duration::from_secs(5),
        })
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

    fn endpoint(&self) -> Result<LocalHttpEndpoint, FroniusError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(FroniusError::MissingField("base URL host"))?;
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            LocalHttpScheme::Http,
            host.to_string(),
        )?
        .with_port(parsed.port.unwrap_or(DEFAULT_PORT))
        .with_metadata(Metadata::new(
            "http.profile",
            "fronius.solar-api-v1.power-flow",
        )))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FroniusMeasurement {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub unit: &'static str,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FroniusSnapshot {
    pub timestamp: String,
    pub measurements: Vec<FroniusMeasurement>,
}

pub fn scan_mdns_ipv4(
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, FroniusError> {
    let options = MdnsScanOptions::new(MDNS_SERVICE_TYPE, discovered_at_ms, timeout)
        .map_err(|error| FroniusError::Discovery(error.to_string()))?
        .with_max_responses(DEFAULT_MAX_DISCOVERY_RESPONSES);
    run_mdns_ipv4_scan(options).map_err(|error| FroniusError::Discovery(error.to_string()))
}

pub fn discovery_record(
    advertisement: &MdnsAdvertisement,
) -> Result<DiscoveryRecord, FroniusError> {
    if advertisement.service_type.trim_end_matches('.') != MDNS_SERVICE_TYPE.trim_end_matches('.') {
        return Err(FroniusError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    let manufacturer = advertisement
        .txt_value("manufacturer")
        .or_else(|| advertisement.txt_value("mf"))
        .unwrap_or_default();
    let model = advertisement
        .txt_value("model")
        .or_else(|| advertisement.txt_value("md"))
        .unwrap_or_default();
    let name_match = contains_fronius(&advertisement.instance_name)
        || contains_fronius(&advertisement.host_name);
    let txt_match = contains_fronius(manufacturer) || contains_fronius(model);
    if !name_match && !txt_match {
        return Err(FroniusError::Validation(
            "HTTP advertisement is not identified as Fronius".to_string(),
        ));
    }
    let native_id = stable_component(&advertisement.host_name);
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        native_id,
        DiscoverySource::Mdns,
        BridgeTransport::LanHttp,
        advertisement.discovered_at_ms,
    )
    .map_err(|error| FroniusError::Discovery(error.to_string()))?
    .with_display_name(&advertisement.instance_name)
    .with_address(advertisement.endpoint_with_scheme("http"))
    .with_hardware_model(model)
    .with_confidence(if txt_match {
        DiscoveryConfidence::Verified
    } else {
        DiscoveryConfidence::Candidate
    })
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE))
}

pub trait FroniusTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, FroniusError>;
}

#[derive(Debug, Clone)]
pub struct FroniusLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for FroniusLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl FroniusTransport for FroniusLanTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, FroniusError> {
        let url = Url::parse(&plan.url)?;
        if url.scheme != "http" {
            return Err(FroniusError::Validation(
                "Fronius transport only permits local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or(FroniusError::MissingField("URL host"))?;
        let port = url
            .effective_port()
            .ok_or(FroniusError::MissingField("URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = encode_http_request(&url, plan)?;
        let mut stream = connect_tcp(host, port, timeout)?;
        stream
            .write_all(&request)
            .map_err(|error| FroniusError::Io(error.to_string()))?;
        let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&bytes, self.maximum_response_bytes)
    }
}

pub struct FroniusClient<T> {
    config: FroniusConfig,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: FroniusTransport> FroniusClient<T> {
    pub fn new(config: FroniusConfig, transport: T) -> Result<Self, FroniusError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &FroniusConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<FroniusSnapshot, FroniusError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, POWER_FLOW_PATH)?
            .with_accept("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, Vec::new())?)?;
        let value = serde_json::from_slice(&bytes)?;
        parse_snapshot(&value)
    }
}

impl<T> fmt::Debug for FroniusClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FroniusClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledFroniusDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

pub struct FroniusRuntimeIntegration<T> {
    client: FroniusClient<T>,
}

impl<T: FroniusTransport> FroniusRuntimeIntegration<T> {
    pub fn new(client: FroniusClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledFroniusDevice, FroniusError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &FroniusSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledFroniusDevice, FroniusError> {
        if snapshot.measurements.is_empty() {
            return Err(FroniusError::NoMeasurements);
        }
        let endpoint = self.client.config.endpoint()?;
        let native_id = stable_component(&endpoint.origin());
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!("fronius:{native_id}"));

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(endpoint.origin());
        bridge.hardware_model = Some("Fronius Solar API v1".to_string());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("http_endpoint", &endpoint.origin())?];
        bridge.metadata = vec![Metadata::new("fronius.transport", "local_http_polling")];
        runtime.upsert_bridge(bridge)?;

        let entities = snapshot
            .measurements
            .iter()
            .map(|measurement| {
                let entity_id = EntityId::trusted(format!(
                    "fronius:{native_id}:sensor:{}",
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
                        Metadata::new("fronius.measurement", measurement.id.clone()),
                        Metadata::new("fronius.scope", measurement.scope.clone()),
                        Metadata::new("fronius.unit", measurement.unit),
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
            manufacturer: "Fronius".to_string(),
            model: "Solar API v1".to_string(),
            name: self.client.config.display_name.clone(),
            serial: None,
            firmware_version: None,
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier("site_endpoint", &endpoint.origin())?],
            health: Health::Online,
            metadata: vec![Metadata::new("fronius.timestamp", &snapshot.timestamp)],
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledFroniusDevice {
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
) -> Result<(), FroniusError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(FroniusError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_snapshot(value: &JsonValue) -> Result<FroniusSnapshot, FroniusError> {
    let head = object_field(value, "Head")?;
    let status = object_field_from(head, "Status", "Head.Status")?;
    let status_code = status
        .get("Code")
        .and_then(JsonValue::as_i64)
        .ok_or(FroniusError::MissingField("Head.Status.Code"))?;
    if status_code != 0 {
        let reason = status
            .get("Reason")
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown error");
        let user_message = status
            .get("UserMessage")
            .and_then(JsonValue::as_str)
            .unwrap_or_default();
        let message = if user_message.is_empty() {
            reason.to_string()
        } else {
            format!("{reason}: {user_message}")
        };
        return Err(FroniusError::ApiStatus {
            code: status_code,
            message,
        });
    }
    let timestamp = head
        .get("Timestamp")
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_string();
    let body = object_field(value, "Body")?;
    let data = object_field_from(body, "Data", "Body.Data")?;
    let mut measurements = Vec::new();
    if let Some(site) = data.get("Site").and_then(JsonValue::as_object) {
        push_measurement(
            &mut measurements,
            site,
            "P_PV",
            "pv-power",
            "PV power",
            "W",
            "site",
        );
        push_measurement(
            &mut measurements,
            site,
            "P_Load",
            "load-power",
            "Load power",
            "W",
            "site",
        );
        push_measurement(
            &mut measurements,
            site,
            "P_Grid",
            "grid-power",
            "Grid power",
            "W",
            "site",
        );
        push_measurement(
            &mut measurements,
            site,
            "P_Akku",
            "battery-power",
            "Battery power",
            "W",
            "site",
        );
        push_measurement(
            &mut measurements,
            site,
            "rel_Autonomy",
            "autonomy",
            "Autonomy",
            "%",
            "site",
        );
        push_measurement(
            &mut measurements,
            site,
            "rel_SelfConsumption",
            "self-consumption",
            "Self consumption",
            "%",
            "site",
        );
        push_measurement(
            &mut measurements,
            site,
            "E_Day",
            "energy-day",
            "Energy today",
            "Wh",
            "site",
        );
        push_measurement(
            &mut measurements,
            site,
            "E_Year",
            "energy-year",
            "Energy this year",
            "Wh",
            "site",
        );
        push_measurement(
            &mut measurements,
            site,
            "E_Total",
            "energy-total",
            "Energy total",
            "Wh",
            "site",
        );
    }
    if let Some(inverters) = data.get("Inverters").and_then(JsonValue::as_object) {
        for (inverter_id, inverter) in inverters {
            let Some(inverter) = inverter.as_object() else {
                continue;
            };
            let scope = format!("inverter:{inverter_id}");
            let prefix = format!("inverter-{}", stable_component(inverter_id));
            push_measurement_owned(
                &mut measurements,
                inverter,
                "P",
                format!("{prefix}-power"),
                format!("Inverter {inverter_id} power"),
                "W",
                &scope,
            );
            push_measurement_owned(
                &mut measurements,
                inverter,
                "E_Day",
                format!("{prefix}-energy-day"),
                format!("Inverter {inverter_id} energy today"),
                "Wh",
                &scope,
            );
            push_measurement_owned(
                &mut measurements,
                inverter,
                "E_Year",
                format!("{prefix}-energy-year"),
                format!("Inverter {inverter_id} energy this year"),
                "Wh",
                &scope,
            );
            push_measurement_owned(
                &mut measurements,
                inverter,
                "E_Total",
                format!("{prefix}-energy-total"),
                format!("Inverter {inverter_id} energy total"),
                "Wh",
                &scope,
            );
        }
    }
    if measurements.is_empty() {
        return Err(FroniusError::NoMeasurements);
    }
    Ok(FroniusSnapshot {
        timestamp,
        measurements,
    })
}

fn push_measurement(
    measurements: &mut Vec<FroniusMeasurement>,
    source: &JsonMap<String, JsonValue>,
    field: &str,
    id: &str,
    name: &str,
    unit: &'static str,
    scope: &str,
) {
    push_measurement_owned(
        measurements,
        source,
        field,
        id.to_string(),
        name.to_string(),
        unit,
        scope,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_measurement_owned(
    measurements: &mut Vec<FroniusMeasurement>,
    source: &JsonMap<String, JsonValue>,
    field: &str,
    id: String,
    name: String,
    unit: &'static str,
    scope: &str,
) {
    if let Some(value) = source.get(field).and_then(JsonValue::as_f64) {
        if value.is_finite() {
            measurements.push(FroniusMeasurement {
                id,
                name,
                value,
                unit,
                scope: scope.to_string(),
            });
        }
    }
}

fn measurement_value(measurement: &FroniusMeasurement) -> Value {
    Value::Object(vec![
        ("value".to_string(), Value::Number(measurement.value)),
        (
            "unit".to_string(),
            Value::Text(measurement.unit.to_string()),
        ),
        ("scope".to_string(), Value::Text(measurement.scope.clone())),
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

fn object_field<'a>(
    value: &'a JsonValue,
    field: &'static str,
) -> Result<&'a JsonMap<String, JsonValue>, FroniusError> {
    value
        .get(field)
        .and_then(JsonValue::as_object)
        .ok_or(FroniusError::MissingField(field))
}

fn object_field_from<'a>(
    value: &'a JsonMap<String, JsonValue>,
    field: &str,
    path: &'static str,
) -> Result<&'a JsonMap<String, JsonValue>, FroniusError> {
    value
        .get(field)
        .and_then(JsonValue::as_object)
        .ok_or(FroniusError::MissingField(path))
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, FroniusError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| FroniusError::Validation(error.to_string()))
}

fn contains_fronius(value: &str) -> bool {
    value.to_ascii_lowercase().contains("fronius")
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

fn encode_http_request(url: &Url, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, FroniusError> {
    let host = url
        .host
        .as_deref()
        .ok_or(FroniusError::MissingField("URL host"))?;
    let port = url
        .effective_port()
        .ok_or(FroniusError::MissingField("URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(FroniusError::Validation(
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
            return Err(FroniusError::Validation(
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

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, FroniusError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| FroniusError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| FroniusError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| FroniusError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(FroniusError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, FroniusError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| FroniusError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(FroniusError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, FroniusError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| FroniusError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(FroniusError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(FroniusError::TruncatedBody {
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
        return Err(FroniusError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, FroniusError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| FroniusError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| FroniusError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| FroniusError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(FroniusError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| FroniusError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(FroniusError::Http("truncated chunk".to_string()));
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

    const SNAPSHOT: &str = r#"{"Body":{"Data":{"Inverters":{"1":{"DT":123,"E_Day":5400.0,"E_Total":1200000.0,"E_Year":730000.0,"P":3210.5}},"Site":{"E_Day":5600.0,"E_Total":1250000.0,"E_Year":750000.0,"P_Akku":-250.0,"P_Grid":-410.0,"P_Load":2800.0,"P_PV":3460.0,"rel_Autonomy":85.4,"rel_SelfConsumption":91.2}}},"Head":{"RequestArguments":{},"Status":{"Code":0,"Reason":"","UserMessage":""},"Timestamp":"2026-08-02T12:00:00+00:00"}}"#;

    fn response(body: &str) -> Vec<u8> {
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()
    }

    fn start_server(payload: Vec<u8>) -> (u16, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let handle = thread::spawn(move || {
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
        });
        (port, requests, handle)
    }

    fn config(port: u16) -> FroniusConfig {
        FroniusConfig::new(
            BridgeId::trusted("fronius.test"),
            format!("http://127.0.0.1:{port}"),
        )
        .unwrap()
        .with_display_name("Roof Solar")
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:fronius-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn fronius_mdns_advertisement_becomes_candidate_http_discovery() {
        let advertisement = MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "Fronius Datamanager",
            "fronius-240.local",
            80,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.20")
        .unwrap();
        let record = discovery_record(&advertisement).unwrap();
        assert_eq!(record.native_bridge_id, "fronius-240-local");
        assert_eq!(record.address.as_deref(), Some("http://192.0.2.20:80"));
        assert_eq!(record.confidence, DiscoveryConfidence::Candidate);

        let other =
            MdnsAdvertisement::new(MDNS_SERVICE_TYPE, "printer", "printer.local", 80, 1_000)
                .unwrap();
        assert!(discovery_record(&other).is_err());
    }

    #[test]
    fn explicit_fronius_txt_identity_is_verified() {
        let advertisement =
            MdnsAdvertisement::new(MDNS_SERVICE_TYPE, "solar", "solar.local", 80, 1_000)
                .unwrap()
                .with_txt("manufacturer", "Fronius International")
                .unwrap();
        assert_eq!(
            discovery_record(&advertisement).unwrap().confidence,
            DiscoveryConfidence::Verified
        );
    }

    #[test]
    fn real_tcp_inspection_installs_site_and_inverter_sensors() {
        let (port, requests, handle) = start_server(response(SNAPSHOT));
        let client = FroniusClient::new(config(port), FroniusLanTransport::default()).unwrap();
        let mut integration = FroniusRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:fronius-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(installed.entity_ids.len(), 13);
        let pv = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:pv-power",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(pv.kind, EntityKind::Sensor);
        assert_eq!(pv.name, "Roof Solar PV power");
        assert_eq!(
            pv.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        assert!(requests.lock().unwrap()[0].contains(&format!("GET {POWER_FLOW_PATH} HTTP/1.1")));
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl FroniusTransport for CountingTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, FroniusError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = FroniusClient::new(
            FroniusConfig::new(BridgeId::trusted("fronius.denied"), "http://127.0.0.1").unwrap(),
            CountingTransport(Arc::clone(&calls)),
        )
        .unwrap();
        let mut integration = FroniusRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(FroniusError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn parser_rejects_api_errors_and_empty_measurements() {
        let error: JsonValue = serde_json::from_str(
            r#"{"Body":{"Data":{}},"Head":{"Status":{"Code":6,"Reason":"Request argument missing","UserMessage":"Scope"}}}"#,
        )
        .unwrap();
        assert!(matches!(
            parse_snapshot(&error),
            Err(FroniusError::ApiStatus { code: 6, .. })
        ));
        let empty: JsonValue = serde_json::from_str(
            r#"{"Body":{"Data":{"Site":{}}},"Head":{"Status":{"Code":0},"Timestamp":"now"}}"#,
        )
        .unwrap();
        assert!(matches!(
            parse_snapshot(&empty),
            Err(FroniusError::NoMeasurements)
        ));
    }

    #[test]
    fn response_bounds_are_enforced() {
        assert!(matches!(
            decode_http_response(&response("{}"), 1),
            Err(FroniusError::ResponseTooLarge { limit: 1 })
        ));
    }
}
