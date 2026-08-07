//! HomeWizard Energy API v1 local telemetry integration for D23.

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
pub const INTEGRATION_ID: &str = "homewizard_energy";
pub const PROTOCOL_ID: &str = "homewizard_energy_api_v1";
pub const MDNS_SERVICE_TYPE: &str = "_hwenergy._tcp.local";
pub const DEVICE_INFO_PATH: &str = "/api";
pub const MEASUREMENT_PATH: &str = "/api/v1/data";
pub const DEFAULT_PORT: u16 = 80;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_MAX_DISCOVERY_RESPONSES: usize = 128;

#[derive(Debug)]
pub enum HomeWizardError {
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
    NoMeasurements,
    Runtime(RuntimeError),
}

impl fmt::Display for HomeWizardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid HomeWizard input: {message}"),
            Self::Discovery(message) => write!(formatter, "HomeWizard discovery failed: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid HomeWizard URL: {error}"),
            Self::Io(message) => write!(formatter, "HomeWizard LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid HomeWizard HTTP response: {message}"),
            Self::HttpStatus(status) => {
                write!(formatter, "HomeWizard endpoint returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "HomeWizard response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "HomeWizard response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid HomeWizard JSON: {error}"),
            Self::MissingField(field) => {
                write!(formatter, "HomeWizard response is missing {field}")
            }
            Self::NoMeasurements => {
                formatter.write_str("HomeWizard response contains no numeric measurements")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for HomeWizardError {}

impl From<LocalHttpError> for HomeWizardError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for HomeWizardError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for HomeWizardError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for HomeWizardError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeWizardConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub display_name: String,
    pub timeout: Duration,
}

impl HomeWizardConfig {
    pub fn new(bridge_id: BridgeId, base_url: impl Into<String>) -> Result<Self, HomeWizardError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(HomeWizardError::Validation(
                "base URL must be a credential-free HTTP origin".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            display_name: "HomeWizard Energy".to_string(),
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

    fn endpoint(&self) -> Result<LocalHttpEndpoint, HomeWizardError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(HomeWizardError::MissingField("base URL host"))?;
        Ok(LocalHttpEndpoint::new(
            IntegrationId::trusted(INTEGRATION_ID),
            self.bridge_id.clone(),
            LocalHttpScheme::Http,
            host.to_string(),
        )?
        .with_port(parsed.port.unwrap_or(DEFAULT_PORT))
        .with_metadata(Metadata::new("http.profile", "homewizard.energy-api-v1")))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct HomeWizardMeasurement {
    pub id: String,
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HomeWizardDeviceInfo {
    pub product_type: String,
    pub product_name: String,
    pub serial: String,
    pub firmware_version: String,
    pub api_version: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HomeWizardSnapshot {
    pub device_info: HomeWizardDeviceInfo,
    pub measurements: Vec<HomeWizardMeasurement>,
}

pub fn scan_mdns_ipv4(
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, HomeWizardError> {
    let options = MdnsScanOptions::new(MDNS_SERVICE_TYPE, discovered_at_ms, timeout)
        .map_err(|error| HomeWizardError::Discovery(error.to_string()))?
        .with_max_responses(DEFAULT_MAX_DISCOVERY_RESPONSES);
    run_mdns_ipv4_scan(options).map_err(|error| HomeWizardError::Discovery(error.to_string()))
}

pub fn discovery_record(
    advertisement: &MdnsAdvertisement,
) -> Result<DiscoveryRecord, HomeWizardError> {
    if advertisement.service_type.trim_end_matches('.') != MDNS_SERVICE_TYPE.trim_end_matches('.') {
        return Err(HomeWizardError::Validation(format!(
            "unexpected mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    let api_enabled = advertisement
        .txt_value("api_enabled")
        .ok_or(HomeWizardError::MissingField("mDNS api_enabled"))?;
    if !matches!(api_enabled.to_ascii_lowercase().as_str(), "1" | "true") {
        return Err(HomeWizardError::Validation(
            "HomeWizard Energy API v1 is disabled".to_string(),
        ));
    }
    if advertisement.txt_value("path") != Some("/api/v1") {
        return Err(HomeWizardError::Validation(
            "mDNS path is not HomeWizard Energy API v1".to_string(),
        ));
    }
    let serial = advertisement
        .txt_value("serial")
        .filter(|value| !value.trim().is_empty())
        .ok_or(HomeWizardError::MissingField("mDNS serial"))?;
    let product_name = advertisement
        .txt_value("product_name")
        .unwrap_or(&advertisement.instance_name);
    let product_type = advertisement.txt_value("product_type").unwrap_or_default();
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(serial),
        DiscoverySource::Mdns,
        BridgeTransport::LanHttp,
        advertisement.discovered_at_ms,
    )
    .map_err(|error| HomeWizardError::Discovery(error.to_string()))?
    .with_display_name(product_name)
    .with_address(advertisement.endpoint_with_scheme("http"))
    .with_hardware_model(product_type)
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("smart_home.discovery.service_type", MDNS_SERVICE_TYPE))
}

pub trait HomeWizardTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, HomeWizardError>;
}

#[derive(Debug, Clone)]
pub struct HomeWizardLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for HomeWizardLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl HomeWizardTransport for HomeWizardLanTransport {
    fn execute(&mut self, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, HomeWizardError> {
        let url = Url::parse(&plan.url)?;
        if url.scheme != "http" {
            return Err(HomeWizardError::Validation(
                "HomeWizard transport only permits local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or(HomeWizardError::MissingField("URL host"))?;
        let port = url
            .effective_port()
            .ok_or(HomeWizardError::MissingField("URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = encode_http_request(&url, plan)?;
        let mut stream = connect_tcp(host, port, timeout)?;
        stream
            .write_all(&request)
            .map_err(|error| HomeWizardError::Io(error.to_string()))?;
        let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&bytes, self.maximum_response_bytes)
    }
}

pub struct HomeWizardClient<T> {
    config: HomeWizardConfig,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: HomeWizardTransport> HomeWizardClient<T> {
    pub fn new(config: HomeWizardConfig, transport: T) -> Result<Self, HomeWizardError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &HomeWizardConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<HomeWizardSnapshot, HomeWizardError> {
        let info = self.request_json(DEVICE_INFO_PATH)?;
        let data = self.request_json(MEASUREMENT_PATH)?;
        parse_snapshot(&info, &data)
    }

    fn request_json(&mut self, path: &str) -> Result<JsonValue, HomeWizardError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
            .with_accept("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout));
        let bytes = self
            .transport
            .execute(&template.plan(&self.endpoint, Vec::new())?)?;
        Ok(serde_json::from_slice(&bytes)?)
    }
}

impl<T> fmt::Debug for HomeWizardClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HomeWizardClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledHomeWizardDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_ids: Vec<EntityId>,
}

pub struct HomeWizardRuntimeIntegration<T> {
    client: HomeWizardClient<T>,
}

impl<T: HomeWizardTransport> HomeWizardRuntimeIntegration<T> {
    pub fn new(client: HomeWizardClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledHomeWizardDevice, HomeWizardError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        self.install_snapshot(runtime, &snapshot, observed_at_ms)
    }

    pub fn install_snapshot(
        &self,
        runtime: &mut SmartHomeRuntime,
        snapshot: &HomeWizardSnapshot,
        observed_at_ms: u64,
    ) -> Result<InstalledHomeWizardDevice, HomeWizardError> {
        if snapshot.measurements.is_empty() {
            return Err(HomeWizardError::NoMeasurements);
        }
        let endpoint = self.client.config.endpoint()?;
        let native_id = stable_component(&snapshot.device_info.serial);
        if native_id.is_empty() {
            return Err(HomeWizardError::Validation(
                "device serial does not contain a stable identifier".to_string(),
            ));
        }
        let bridge_id = self.client.config.bridge_id.clone();
        let device_id = DeviceId::trusted(format!("homewizard:{native_id}"));

        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted(INTEGRATION_ID),
            BridgeTransport::LanHttp,
        );
        bridge.address = Some(endpoint.origin());
        bridge.hardware_model = Some(snapshot.device_info.product_type.clone());
        bridge.health = Health::Online;
        bridge.last_seen_at_ms = Some(observed_at_ms);
        bridge.identifiers = vec![protocol_identifier("http_endpoint", &endpoint.origin())?];
        bridge.metadata = vec![Metadata::new("homewizard.transport", "local_http_polling")];
        runtime.upsert_bridge(bridge)?;

        let entities = snapshot
            .measurements
            .iter()
            .map(|measurement| {
                let entity_id = EntityId::trusted(format!(
                    "homewizard:{native_id}:sensor:{}",
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
                        Metadata::new("homewizard.measurement", measurement.id.clone()),
                        Metadata::new("homewizard.scope", measurement.scope.clone()),
                        Metadata::new("homewizard.unit", measurement.unit.clone()),
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
            manufacturer: "HomeWizard".to_string(),
            model: snapshot.device_info.product_type.clone(),
            name: self.client.config.display_name.clone(),
            serial: Some(snapshot.device_info.serial.clone()),
            firmware_version: Some(snapshot.device_info.firmware_version.clone()),
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier("serial", &snapshot.device_info.serial)?],
            health: Health::Online,
            metadata: vec![
                Metadata::new(
                    "homewizard.product_name",
                    &snapshot.device_info.product_name,
                ),
                Metadata::new("homewizard.api_version", &snapshot.device_info.api_version),
            ],
        })?;
        for entity in entities {
            runtime.upsert_entity(entity)?;
        }
        Ok(InstalledHomeWizardDevice {
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
) -> Result<(), HomeWizardError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(HomeWizardError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_snapshot(
    info: &JsonValue,
    data: &JsonValue,
) -> Result<HomeWizardSnapshot, HomeWizardError> {
    let info = info
        .as_object()
        .ok_or(HomeWizardError::MissingField("device information object"))?;
    let device_info = HomeWizardDeviceInfo {
        product_type: required_string(info, "product_type")?,
        product_name: required_string(info, "product_name")?,
        serial: required_string(info, "serial")?,
        firmware_version: required_string(info, "firmware_version")?,
        api_version: required_string(info, "api_version")?,
    };
    if device_info.api_version != "v1" {
        return Err(HomeWizardError::Validation(format!(
            "unsupported Energy API version `{}`",
            device_info.api_version
        )));
    }

    let data = data
        .as_object()
        .ok_or(HomeWizardError::MissingField("measurement object"))?;
    let mut measurements = Vec::new();
    for (field, value) in data {
        if field == "external" || is_metadata_field(field) {
            continue;
        }
        let Some(value) = value.as_f64().filter(|value| value.is_finite()) else {
            continue;
        };
        measurements.push(HomeWizardMeasurement {
            id: stable_component(field),
            name: display_name(field),
            value,
            unit: measurement_unit(field).to_string(),
            scope: "device".to_string(),
        });
    }
    if let Some(external) = data.get("external").and_then(JsonValue::as_array) {
        for meter in external {
            let Some(meter) = meter.as_object() else {
                continue;
            };
            let Some(value) = meter
                .get("value")
                .and_then(JsonValue::as_f64)
                .filter(|value| value.is_finite())
            else {
                continue;
            };
            let unique_id = required_string(meter, "unique_id")?;
            let meter_type = required_string(meter, "type")?;
            let unit = required_string(meter, "unit")?;
            let component = stable_component(&unique_id);
            measurements.push(HomeWizardMeasurement {
                id: format!("external-{component}"),
                name: format!("External {}", display_name(&meter_type)),
                value,
                unit,
                scope: format!("external:{component}"),
            });
        }
    }
    if measurements.is_empty() {
        return Err(HomeWizardError::NoMeasurements);
    }
    Ok(HomeWizardSnapshot {
        device_info,
        measurements,
    })
}

fn required_string(
    value: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, HomeWizardError> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(HomeWizardError::MissingField(field))
}

fn is_metadata_field(field: &str) -> bool {
    field.contains("timestamp")
        || field.contains("version")
        || matches!(field, "active_tariff" | "meter_model" | "unique_id")
}

fn measurement_unit(field: &str) -> &'static str {
    if field == "wifi_strength" {
        "%"
    } else if field.ends_with("_kwh") {
        "kWh"
    } else if field.ends_with("_lpm") {
        "L/min"
    } else if field.ends_with("_m3") {
        "m3"
    } else if field.ends_with("_hz") {
        "Hz"
    } else if field.ends_with("_var") {
        "var"
    } else if field.ends_with("_va") {
        "VA"
    } else if field.ends_with("_w") {
        "W"
    } else if field.ends_with("_v") {
        "V"
    } else if field.ends_with("_a") {
        "A"
    } else if field.ends_with("_count") {
        "count"
    } else if field.ends_with("_factor") {
        "ratio"
    } else {
        "value"
    }
}

fn display_name(field: &str) -> String {
    let field = [
        "_kwh", "_lpm", "_m3", "_hz", "_var", "_va", "_w", "_v", "_a", "_count", "_factor",
    ]
    .iter()
    .find_map(|suffix| field.strip_suffix(suffix))
    .unwrap_or(field);
    let words = field.replace(['_', '-'], " ");
    let mut characters = words.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

fn measurement_value(measurement: &HomeWizardMeasurement) -> Value {
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

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, HomeWizardError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| HomeWizardError::Validation(error.to_string()))
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

fn encode_http_request(url: &Url, plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, HomeWizardError> {
    let host = url
        .host
        .as_deref()
        .ok_or(HomeWizardError::MissingField("URL host"))?;
    let port = url
        .effective_port()
        .ok_or(HomeWizardError::MissingField("URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(HomeWizardError::Validation(
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
            return Err(HomeWizardError::Validation(
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

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, HomeWizardError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| HomeWizardError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| HomeWizardError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| HomeWizardError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(HomeWizardError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, HomeWizardError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| HomeWizardError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(HomeWizardError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, HomeWizardError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| HomeWizardError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(HomeWizardError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(HomeWizardError::TruncatedBody {
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
        return Err(HomeWizardError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, HomeWizardError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| HomeWizardError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| HomeWizardError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| HomeWizardError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(HomeWizardError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| HomeWizardError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(HomeWizardError::Http("truncated chunk".to_string()));
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

    const DEVICE_INFO: &str = r#"{"product_type":"HWE-P1","product_name":"P1 Meter","serial":"3c39e7aabbcc","firmware_version":"5.18","api_version":"v1"}"#;
    const MEASUREMENTS: &str = r#"{"wifi_strength":82,"total_power_import_kwh":1234.567,"active_power_w":321.5,"active_voltage_l1_v":230.1,"active_tariff":1,"external":[{"unique_id":"gas-001","type":"gas_meter","value":42.25,"unit":"m3"}]}"#;

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

    fn config(port: u16) -> HomeWizardConfig {
        HomeWizardConfig::new(
            BridgeId::trusted("homewizard.test"),
            format!("http://127.0.0.1:{port}"),
        )
        .unwrap()
        .with_display_name("Utility Meter")
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:homewizard-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn homewizard_mdns_advertisement_becomes_verified_http_discovery() {
        let advertisement = MdnsAdvertisement::new(
            MDNS_SERVICE_TYPE,
            "P1 Meter",
            "p1meter-3c39e7aabbcc.local",
            80,
            1_000,
        )
        .unwrap()
        .with_address("192.0.2.20")
        .unwrap()
        .with_txt("api_enabled", "1")
        .unwrap()
        .with_txt("path", "/api/v1")
        .unwrap()
        .with_txt("serial", "3c39e7aabbcc")
        .unwrap()
        .with_txt("product_name", "P1 Meter")
        .unwrap()
        .with_txt("product_type", "HWE-P1")
        .unwrap();
        let record = discovery_record(&advertisement).unwrap();
        assert_eq!(record.native_bridge_id, "3c39e7aabbcc");
        assert_eq!(record.address.as_deref(), Some("http://192.0.2.20:80"));
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.hardware_model.as_deref(), Some("HWE-P1"));
    }

    #[test]
    fn disabled_or_wrong_path_mdns_advertisements_are_rejected() {
        for (enabled, path) in [("false", "/api/v1"), ("true", "/api/v2")] {
            let advertisement =
                MdnsAdvertisement::new(MDNS_SERVICE_TYPE, "P1 Meter", "p1meter.local", 80, 1_000)
                    .unwrap()
                    .with_txt("api_enabled", enabled)
                    .unwrap()
                    .with_txt("path", path)
                    .unwrap()
                    .with_txt("serial", "3c39e7aabbcc")
                    .unwrap();
            assert!(discovery_record(&advertisement).is_err());
        }
    }

    #[test]
    fn real_tcp_inspection_installs_device_and_external_meter_sensors() {
        let (port, requests, handle) =
            start_server(vec![response(DEVICE_INFO), response(MEASUREMENTS)]);
        let client =
            HomeWizardClient::new(config(port), HomeWizardLanTransport::default()).unwrap();
        let mut integration = HomeWizardRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:homewizard-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();
        assert_eq!(installed.entity_ids.len(), 5);
        let power = runtime
            .registry()
            .entity(&EntityId::trusted(format!(
                "{}:sensor:active-power-w",
                installed.device_id.as_str()
            )))
            .unwrap();
        assert_eq!(power.kind, EntityKind::Sensor);
        assert_eq!(power.name, "Utility Meter Active power");
        assert_eq!(
            power.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        let device = runtime.registry().device(&installed.device_id).unwrap();
        assert_eq!(device.manufacturer, "HomeWizard");
        assert_eq!(device.model, "HWE-P1");
        assert_eq!(device.serial.as_deref(), Some("3c39e7aabbcc"));
        let requests = requests.lock().unwrap();
        assert!(requests[0].contains(&format!("GET {DEVICE_INFO_PATH} HTTP/1.1")));
        assert!(requests[1].contains(&format!("GET {MEASUREMENT_PATH} HTTP/1.1")));
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl HomeWizardTransport for CountingTransport {
        fn execute(&mut self, _plan: &LocalHttpRequestPlan) -> Result<Vec<u8>, HomeWizardError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = HomeWizardClient::new(
            HomeWizardConfig::new(BridgeId::trusted("homewizard.denied"), "http://127.0.0.1")
                .unwrap(),
            CountingTransport(Arc::clone(&calls)),
        )
        .unwrap();
        let mut integration = HomeWizardRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(HomeWizardError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn parser_rejects_unsupported_versions_and_empty_measurements() {
        let unsupported: JsonValue = serde_json::from_str(
            r#"{"product_type":"HWE-P1","product_name":"P1 Meter","serial":"abc","firmware_version":"5.18","api_version":"v2"}"#,
        )
        .unwrap();
        let empty: JsonValue = serde_json::from_str("{}").unwrap();
        assert!(matches!(
            parse_snapshot(&unsupported, &empty),
            Err(HomeWizardError::Validation(_))
        ));
        let info: JsonValue = serde_json::from_str(DEVICE_INFO).unwrap();
        assert!(matches!(
            parse_snapshot(&info, &empty),
            Err(HomeWizardError::NoMeasurements)
        ));
    }

    #[test]
    fn parser_assigns_units_and_external_meter_scope() {
        let info: JsonValue = serde_json::from_str(DEVICE_INFO).unwrap();
        let data: JsonValue = serde_json::from_str(MEASUREMENTS).unwrap();
        let snapshot = parse_snapshot(&info, &data).unwrap();
        assert_eq!(snapshot.device_info.product_name, "P1 Meter");
        assert!(snapshot
            .measurements
            .iter()
            .any(|measurement| measurement.id == "active-power-w" && measurement.unit == "W"));
        assert!(snapshot.measurements.iter().any(|measurement| {
            measurement.id == "external-gas-001"
                && measurement.unit == "m3"
                && measurement.scope == "external:gas-001"
        }));
    }

    #[test]
    fn response_bounds_are_enforced() {
        assert!(matches!(
            decode_http_response(&response("{}"), 1),
            Err(HomeWizardError::ResponseTooLarge { limit: 1 })
        ));
    }
}
