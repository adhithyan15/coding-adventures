//! Roku External Control Protocol discovery and inspection for D23.

#![forbid(unsafe_code)]

use coding_adventures_xml_parser::{parse_xml, XmlElement, XmlNode};
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use udp_client::{send_to_and_collect, UdpDiscoveryEndpoint, UdpError, UdpOptions};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "roku";
pub const PROTOCOL_ID: &str = "roku_ecp";
pub const SSDP_SEARCH_TARGET: &str = "roku:ecp";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub enum RokuError {
    Validation(String),
    Url(UrlError),
    Udp(UdpError),
    Io(String),
    Http(String),
    HttpStatus(u16),
    Xml(String),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    Runtime(RuntimeError),
}

impl fmt::Display for RokuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Roku ECP input: {message}"),
            Self::Url(error) => write!(formatter, "invalid Roku ECP URL: {error}"),
            Self::Udp(error) => write!(formatter, "Roku SSDP failed: {error}"),
            Self::Io(message) => write!(formatter, "Roku LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Roku HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "Roku endpoint returned HTTP {status}"),
            Self::Xml(message) => write!(formatter, "invalid Roku XML: {message}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Roku response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Roku response body is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RokuError {}

impl From<UrlError> for RokuError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<UdpError> for RokuError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<RuntimeError> for RokuError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RokuConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
}

impl RokuConfig {
    pub fn new(bridge_id: BridgeId, base_url: impl Into<String>) -> Result<Self, RokuError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
        {
            return Err(RokuError::Validation(
                "base URL must be credential-free local HTTP".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RokuSsdpCandidate {
    pub location: String,
    pub usn: String,
    pub server: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RokuDeviceInformation {
    pub name: String,
    pub model: String,
    pub serial_number: String,
    pub device_id: Option<String>,
    pub software_version: Option<String>,
    pub power_mode: Option<String>,
    pub is_tv: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RokuApp {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RokuSnapshot {
    pub device: RokuDeviceInformation,
    pub apps: Vec<RokuApp>,
    pub active_app: Option<RokuApp>,
}

pub fn ssdp_search_request() -> Vec<u8> {
    format!(
        "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: {SSDP_SEARCH_TARGET}\r\n\r\n"
    )
    .into_bytes()
}

pub fn discover_ssdp_ipv4(
    timeout: Duration,
    max_responses: usize,
) -> Result<Vec<RokuSsdpCandidate>, RokuError> {
    let endpoint = UdpDiscoveryEndpoint::ssdp_ipv4();
    discover_ssdp(
        endpoint.destination,
        endpoint.options(65_507, Some(timeout), Some(timeout)),
        max_responses,
    )
}

pub fn discover_ssdp(
    destination: SocketAddr,
    options: UdpOptions,
    max_responses: usize,
) -> Result<Vec<RokuSsdpCandidate>, RokuError> {
    let datagrams =
        send_to_and_collect(destination, &ssdp_search_request(), options, max_responses)?;
    let mut candidates = BTreeMap::new();
    for datagram in datagrams {
        if let Ok(candidate) = parse_ssdp_response(&datagram.payload) {
            candidates.entry(candidate.usn.clone()).or_insert(candidate);
        }
    }
    Ok(candidates.into_values().collect())
}

pub fn parse_ssdp_response(bytes: &[u8]) -> Result<RokuSsdpCandidate, RokuError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| RokuError::Validation("SSDP response is not UTF-8".to_string()))?;
    let mut lines = source.split("\r\n");
    let status = lines.next().unwrap_or_default();
    if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
        return Err(RokuError::Validation(
            "SSDP response is not HTTP 200".to_string(),
        ));
    }
    let mut headers = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }
    let st = headers.get("st").map(String::as_str).unwrap_or_default();
    if !st.eq_ignore_ascii_case(SSDP_SEARCH_TARGET) {
        return Err(RokuError::Validation(format!(
            "unexpected SSDP search target `{st}`"
        )));
    }
    let location = required_header(&headers, "location")?;
    RokuConfig::new(BridgeId::trusted("roku.ssdp.validation"), location)?;
    Ok(RokuSsdpCandidate {
        location: location.to_string(),
        usn: required_header(&headers, "usn")?.to_string(),
        server: headers.get("server").cloned(),
    })
}

fn required_header<'a>(
    headers: &'a BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, RokuError> {
    headers
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RokuError::Validation(format!("SSDP response is missing {name}")))
}

pub fn discovery_record(
    candidate: &RokuSsdpCandidate,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, RokuError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&candidate.usn),
        DiscoverySource::Ssdp,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| RokuError::Validation(error.to_string()))?
    .with_display_name("Roku ECP device")
    .with_address(candidate.location.clone())
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("smart_home.discovery.search_target", SSDP_SEARCH_TARGET))
}

pub trait RokuTransport {
    fn get(&mut self, endpoint: &str) -> Result<Vec<u8>, RokuError>;
}

#[derive(Debug, Clone)]
pub struct RokuLanTransport {
    timeout: Duration,
    maximum_response_bytes: usize,
}

impl Default for RokuLanTransport {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl RokuLanTransport {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn with_maximum_response_bytes(mut self, maximum: usize) -> Self {
        self.maximum_response_bytes = maximum.max(1);
        self
    }
}

impl RokuTransport for RokuLanTransport {
    fn get(&mut self, endpoint: &str) -> Result<Vec<u8>, RokuError> {
        let url = Url::parse(endpoint)?;
        if url.scheme != "http" {
            return Err(RokuError::Validation(
                "Roku ECP requires local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or_else(|| RokuError::Validation("endpoint is missing a host".to_string()))?;
        let port = url
            .effective_port()
            .ok_or_else(|| RokuError::Validation("endpoint is missing a port".to_string()))?;
        let mut stream = connect_tcp(host, port, self.timeout)?;
        stream
            .write_all(&encode_get_request(&url)?)
            .map_err(|error| RokuError::Io(error.to_string()))?;
        stream
            .flush()
            .map_err(|error| RokuError::Io(error.to_string()))?;
        let response = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&response, self.maximum_response_bytes)
    }
}

pub struct RokuClient<T> {
    config: RokuConfig,
    transport: T,
}

impl<T: RokuTransport> RokuClient<T> {
    pub fn new(config: RokuConfig, transport: T) -> Self {
        Self { config, transport }
    }

    pub fn config(&self) -> &RokuConfig {
        &self.config
    }

    pub fn inspect(&mut self) -> Result<RokuSnapshot, RokuError> {
        let device = parse_device_information(&self.get("/query/device-info")?)?;
        let apps = parse_apps(&self.get("/query/apps")?)?;
        let active_app = parse_active_app(&self.get("/query/active-app")?)?;
        Ok(RokuSnapshot {
            device,
            apps,
            active_app,
        })
    }

    fn get(&mut self, path: &str) -> Result<Vec<u8>, RokuError> {
        self.transport
            .get(&format!("{}{path}", self.config.base_url))
    }
}

pub struct RokuRuntimeIntegration<T> {
    client: RokuClient<T>,
}

impl<T: RokuTransport> RokuRuntimeIntegration<T> {
    pub fn new(client: RokuClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<EntityId, RokuError> {
        let decision = runtime.authorize_tool_for_principal(
            principal_id.clone(),
            SmartHomeTool::GetState,
            observed_at_ms,
        );
        if !decision.missing_capabilities.is_empty() {
            return Err(RokuError::Runtime(RuntimeError::UnauthorizedTool {
                principal_id,
                tool: SmartHomeTool::GetState,
                missing_capabilities: decision.missing_capabilities,
            }));
        }
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

pub fn parse_device_information(bytes: &[u8]) -> Result<RokuDeviceInformation, RokuError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| RokuError::Xml("device-info is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| RokuError::Xml(error.to_string()))?;
    if document.root.local_name != "device-info" {
        return Err(RokuError::Xml("expected device-info root".to_string()));
    }
    let root = &document.root;
    let model = optional_child_text(root, "model-name")
        .or_else(|| optional_child_text(root, "model-number"))
        .unwrap_or_else(|| "Roku".to_string());
    Ok(RokuDeviceInformation {
        name: optional_child_text(root, "user-device-name")
            .or_else(|| optional_child_text(root, "friendly-device-name"))
            .unwrap_or_else(|| model.clone()),
        model,
        serial_number: required_child_text(root, "serial-number")?,
        device_id: optional_child_text(root, "device-id"),
        software_version: optional_child_text(root, "software-version"),
        power_mode: optional_child_text(root, "power-mode"),
        is_tv: optional_child_text(root, "is-tv").is_some_and(|value| value == "true"),
    })
}

pub fn parse_apps(bytes: &[u8]) -> Result<Vec<RokuApp>, RokuError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| RokuError::Xml("apps response is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| RokuError::Xml(error.to_string()))?;
    if document.root.local_name != "apps" {
        return Err(RokuError::Xml("expected apps root".to_string()));
    }
    document
        .root
        .children
        .iter()
        .filter_map(|node| match node {
            XmlNode::Element(element) if element.local_name == "app" => Some(parse_app(element)),
            _ => None,
        })
        .collect()
}

pub fn parse_active_app(bytes: &[u8]) -> Result<Option<RokuApp>, RokuError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| RokuError::Xml("active-app response is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| RokuError::Xml(error.to_string()))?;
    if document.root.local_name != "active-app" {
        return Err(RokuError::Xml("expected active-app root".to_string()));
    }
    document
        .root
        .children
        .iter()
        .find_map(|node| match node {
            XmlNode::Element(element) if element.local_name == "app" => Some(parse_app(element)),
            _ => None,
        })
        .transpose()
}

fn parse_app(element: &XmlElement) -> Result<RokuApp, RokuError> {
    let id = element
        .get_attr(None, "id")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RokuError::Xml("app is missing id".to_string()))?;
    Ok(RokuApp {
        id: id.to_string(),
        name: element.text_content().trim().to_string(),
        version: element.get_attr(None, "version").map(str::to_string),
    })
}

fn required_child_text(root: &XmlElement, name: &str) -> Result<String, RokuError> {
    optional_child_text(root, name)
        .ok_or_else(|| RokuError::Xml(format!("device-info is missing {name}")))
}

fn optional_child_text(root: &XmlElement, name: &str) -> Option<String> {
    root.get_child(None, name)
        .map(XmlElement::text_content)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &RokuConfig,
    snapshot: &RokuSnapshot,
    observed_at_ms: u64,
) -> Result<EntityId, RokuError> {
    let native_id = stable_component(&snapshot.device.serial_number);
    if native_id.is_empty() {
        return Err(RokuError::Validation("device serial is empty".to_string()));
    }
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some(snapshot.device.model.clone());
    bridge.firmware_version = snapshot.device.software_version.clone();
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier(
        "ecp_device_id",
        snapshot
            .device
            .device_id
            .as_deref()
            .unwrap_or(&snapshot.device.serial_number),
    )?];
    bridge.metadata = vec![Metadata::new("roku.protocol", PROTOCOL_ID)];
    runtime.upsert_bridge(bridge)?;

    let device_id = DeviceId::trusted(format!("roku:{native_id}"));
    let entity_id = EntityId::trusted(format!("roku:{native_id}:media"));
    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: "Roku".to_string(),
        model: snapshot.device.model.clone(),
        name: snapshot.device.name.clone(),
        serial: Some(snapshot.device.serial_number.clone()),
        firmware_version: snapshot.device.software_version.clone(),
        room_id: None,
        entity_ids: vec![entity_id.clone()],
        identifiers: vec![protocol_identifier(
            "serial",
            &snapshot.device.serial_number,
        )?],
        health: Health::Online,
        metadata: vec![Metadata::new(
            "roku.is_tv",
            snapshot.device.is_tv.to_string(),
        )],
    })?;
    runtime.upsert_entity(Entity {
        entity_id: entity_id.clone(),
        device_id,
        kind: EntityKind::Unknown,
        name: snapshot.device.name.clone(),
        capabilities: vec![Capability::new(
            CapabilityId::trusted("media.current_app"),
            CapabilityMode::Observe,
            ValueKind::Object,
        )],
        state: Some(StateSnapshot {
            entity_id: entity_id.clone(),
            value: snapshot_value(snapshot),
            source: StateSource::Poll,
            observed_at_ms,
            received_at_ms: observed_at_ms,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        }),
        metadata: vec![Metadata::new(
            "roku.installed_app_count",
            snapshot.apps.len().to_string(),
        )],
    })?;
    Ok(entity_id)
}

fn snapshot_value(snapshot: &RokuSnapshot) -> Value {
    let active = snapshot.active_app.as_ref().map_or(Value::Null, |app| {
        Value::Object(vec![
            ("id".to_string(), Value::Text(app.id.clone())),
            ("name".to_string(), Value::Text(app.name.clone())),
        ])
    });
    Value::Object(vec![
        ("active_app".to_string(), active),
        (
            "power_mode".to_string(),
            snapshot
                .device
                .power_mode
                .clone()
                .map_or(Value::Null, Value::Text),
        ),
        (
            "installed_app_count".to_string(),
            Value::Integer(i64::try_from(snapshot.apps.len()).unwrap_or(i64::MAX)),
        ),
    ])
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, RokuError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| RokuError::Validation(error.to_string()))
}

fn stable_component(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn encode_get_request(url: &Url) -> Result<Vec<u8>, RokuError> {
    let host = url
        .host
        .as_deref()
        .ok_or_else(|| RokuError::Validation("endpoint is missing a host".to_string()))?;
    if url.path.contains(['\r', '\n']) {
        return Err(RokuError::Validation(
            "unsafe HTTP request path".to_string(),
        ));
    }
    let host_header = url
        .port
        .map_or_else(|| host.to_string(), |port| format!("{host}:{port}"));
    Ok(format!(
        "GET {} HTTP/1.1\r\nHost: {host_header}\r\nAccept: application/xml\r\nConnection: close\r\n\r\n",
        url.path
    )
    .into_bytes())
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, RokuError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| RokuError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| RokuError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| RokuError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(RokuError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, RokuError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| RokuError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(RokuError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, RokuError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| RokuError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(RokuError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(RokuError::TruncatedBody {
                    expected,
                    actual: input.len(),
                });
            }
            input[..expected].to_vec()
        }
        BodyKind::UntilEof => input.to_vec(),
        BodyKind::Chunked => {
            return Err(RokuError::Http(
                "chunked Roku responses are unsupported".to_string(),
            ))
        }
    };
    if body.len() > maximum {
        return Err(RokuError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::{TcpListener, UdpSocket};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    const DEVICE_INFO: &str = r#"<device-info><user-device-name>Living Room Roku</user-device-name><model-name>Roku Ultra</model-name><serial-number>YJ00AB123456</serial-number><device-id>012345678901</device-id><software-version>14.1.4</software-version><power-mode>PowerOn</power-mode><is-tv>false</is-tv></device-info>"#;
    const APPS: &str =
        r#"<apps><app id="12" version="6.0">Netflix</app><app id="13">YouTube</app></apps>"#;
    const ACTIVE_APP: &str = r#"<active-app><app id="12" version="6.0">Netflix</app></active-app>"#;

    #[test]
    fn parses_ssdp_and_rejects_other_targets() {
        let candidate = parse_ssdp_response(
            b"HTTP/1.1 200 OK\r\nST: roku:ecp\r\nUSN: uuid:roku:ecp:YJ00AB\r\nLOCATION: http://127.0.0.1:8060/\r\nSERVER: Roku/14.1\r\n\r\n",
        )
        .unwrap();
        assert_eq!(candidate.location, "http://127.0.0.1:8060/");
        assert!(parse_ssdp_response(
            b"HTTP/1.1 200 OK\r\nST: upnp:rootdevice\r\nUSN: x\r\nLOCATION: http://127.0.0.1/\r\n\r\n"
        )
        .is_err());
    }

    #[test]
    fn parses_device_and_app_xml() {
        let device = parse_device_information(DEVICE_INFO.as_bytes()).unwrap();
        let apps = parse_apps(APPS.as_bytes()).unwrap();
        let active = parse_active_app(ACTIVE_APP.as_bytes()).unwrap().unwrap();
        assert_eq!(device.name, "Living Room Roku");
        assert_eq!(apps.len(), 2);
        assert_eq!(active.name, "Netflix");
    }

    #[test]
    fn authorization_denies_before_transport_io() {
        struct CountingTransport(Arc<AtomicUsize>);
        impl RokuTransport for CountingTransport {
            fn get(&mut self, _endpoint: &str) -> Result<Vec<u8>, RokuError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(Vec::new())
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let config =
            RokuConfig::new(BridgeId::trusted("roku:test"), "http://127.0.0.1:8060").unwrap();
        let client = RokuClient::new(config, CountingTransport(Arc::clone(&calls)));
        let mut integration = RokuRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let error = integration
            .inspect_and_install_authorized(&mut runtime, AgentId::trusted("agent:test"), 10)
            .unwrap_err();
        assert!(matches!(
            error,
            RokuError::Runtime(RuntimeError::UnauthorizedTool { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn loopback_ssdp_http_inspection_and_install() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let http_address = listener.local_addr().unwrap();
        let http_handle = thread::spawn(move || {
            for body in [DEVICE_INFO, APPS, ACTIVE_APP] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 1024];
                let _ = stream.read(&mut request).unwrap();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });

        let udp = UdpSocket::bind("127.0.0.1:0").unwrap();
        let udp_address = udp.local_addr().unwrap();
        let udp_handle = thread::spawn(move || {
            let mut request = [0u8; 2048];
            let (_, source) = udp.recv_from(&mut request).unwrap();
            let response = format!(
                "HTTP/1.1 200 OK\r\nST: roku:ecp\r\nUSN: uuid:roku:ecp:YJ00AB123456\r\nLOCATION: http://{http_address}/\r\nSERVER: Roku/14.1\r\n\r\n"
            );
            udp.send_to(response.as_bytes(), source).unwrap();
        });
        let candidates = discover_ssdp(
            udp_address,
            UdpOptions {
                bind_addr: Some("127.0.0.1:0".parse().unwrap()),
                max_datagram_size: 4096,
                read_timeout: Some(Duration::from_millis(100)),
                write_timeout: Some(Duration::from_millis(100)),
            },
            4,
        )
        .unwrap();
        assert_eq!(candidates.len(), 1);

        let config =
            RokuConfig::new(BridgeId::trusted("roku:test"), &candidates[0].location).unwrap();
        let client = RokuClient::new(config, RokuLanTransport::default());
        let mut integration = RokuRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:test");
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_capability(
                CapabilityGrantId::trusted("grant:roku-test"),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "test",
                1,
            ));
        let entity_id = integration
            .inspect_and_install_authorized(&mut runtime, principal, 10)
            .unwrap();
        assert!(runtime.registry().entity(&entity_id).is_some());
        assert_eq!(runtime.registry().counts().devices, 1);
        udp_handle.join().unwrap();
        http_handle.join().unwrap();
    }
}
