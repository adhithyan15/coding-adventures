//! Sonos UPnP discovery and read-only player inspection for D23.

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
pub const INTEGRATION_ID: &str = "sonos";
pub const PROTOCOL_ID: &str = "sonos_upnp";
pub const ZONE_PLAYER_DEVICE_TYPE: &str = "urn:schemas-upnp-org:device:ZonePlayer:1";
pub const AV_TRANSPORT_SERVICE_TYPE: &str = "urn:schemas-upnp-org:service:AVTransport:1";
pub const RENDERING_CONTROL_SERVICE_TYPE: &str = "urn:schemas-upnp-org:service:RenderingControl:1";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub enum SonosError {
    Validation(String),
    Url(UrlError),
    Udp(UdpError),
    Io(String),
    Http(String),
    HttpStatus(u16),
    Xml(String),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    MissingService(&'static str),
    Runtime(RuntimeError),
}

impl fmt::Display for SonosError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Sonos input: {message}"),
            Self::Url(error) => write!(formatter, "invalid Sonos URL: {error}"),
            Self::Udp(error) => write!(formatter, "Sonos SSDP failed: {error}"),
            Self::Io(message) => write!(formatter, "Sonos LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Sonos HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "Sonos endpoint returned HTTP {status}"),
            Self::Xml(message) => write!(formatter, "invalid Sonos XML: {message}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Sonos response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Sonos response body is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::MissingService(service) => {
                write!(formatter, "Sonos description has no {service} service")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SonosError {}

impl From<UrlError> for SonosError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<UdpError> for SonosError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<RuntimeError> for SonosError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SonosConfig {
    pub bridge_id: BridgeId,
    pub setup_url: String,
}

impl SonosConfig {
    pub fn new(bridge_id: BridgeId, setup_url: impl Into<String>) -> Result<Self, SonosError> {
        let setup_url = setup_url.into();
        let parsed = Url::parse(&setup_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || parsed.path.is_empty()
        {
            return Err(SonosError::Validation(
                "setup URL must be credential-free local HTTP with a path".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            setup_url,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SonosSsdpCandidate {
    pub location: String,
    pub usn: String,
    pub server: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SonosService {
    pub service_type: String,
    pub control_url: String,
    pub event_subscription_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SonosDeviceDescription {
    pub friendly_name: String,
    pub model_name: String,
    pub model_number: Option<String>,
    pub serial_number: String,
    pub udn: String,
    pub firmware_version: Option<String>,
    pub room_name: Option<String>,
    pub av_transport: SonosService,
    pub rendering_control: SonosService,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SonosSnapshot {
    pub device: SonosDeviceDescription,
    pub transport_state: String,
    pub volume: u8,
    pub muted: bool,
    pub track_uri: Option<String>,
    pub track_title: Option<String>,
    pub track_artist: Option<String>,
}

pub fn ssdp_search_request() -> Vec<u8> {
    format!(
        "M-SEARCH * HTTP/1.1\r\nST: {ZONE_PLAYER_DEVICE_TYPE}\r\nMX: 2\r\nMAN: \"ssdp:discover\"\r\nHOST: 239.255.255.250:1900\r\n\r\n"
    )
    .into_bytes()
}

pub fn discover_ssdp_ipv4(
    timeout: Duration,
    max_responses: usize,
) -> Result<Vec<SonosSsdpCandidate>, SonosError> {
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
) -> Result<Vec<SonosSsdpCandidate>, SonosError> {
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

pub fn parse_ssdp_response(bytes: &[u8]) -> Result<SonosSsdpCandidate, SonosError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| SonosError::Validation("SSDP response is not UTF-8".to_string()))?;
    let mut lines = source.split("\r\n");
    let status = lines.next().unwrap_or_default();
    if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
        return Err(SonosError::Validation(
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
    let st = required_header(&headers, "st")?;
    if !st.eq_ignore_ascii_case(ZONE_PLAYER_DEVICE_TYPE) {
        return Err(SonosError::Validation(format!(
            "unexpected SSDP search target `{st}`"
        )));
    }
    let location = required_header(&headers, "location")?;
    SonosConfig::new(BridgeId::trusted("sonos.ssdp.validation"), location)?;
    Ok(SonosSsdpCandidate {
        location: location.to_string(),
        usn: required_header(&headers, "usn")?.to_string(),
        server: headers.get("server").cloned(),
    })
}

fn required_header<'a>(
    headers: &'a BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, SonosError> {
    headers
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SonosError::Validation(format!("SSDP response is missing {name}")))
}

pub fn discovery_record(
    candidate: &SonosSsdpCandidate,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, SonosError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(candidate.usn.split("::").next().unwrap_or(&candidate.usn)),
        DiscoverySource::Ssdp,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| SonosError::Validation(error.to_string()))?
    .with_display_name("Sonos UPnP device")
    .with_address(candidate.location.clone())
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata(
        "smart_home.discovery.search_target",
        ZONE_PLAYER_DEVICE_TYPE,
    ))
}

pub trait SonosTransport {
    fn get(&mut self, endpoint: &str) -> Result<Vec<u8>, SonosError>;
    fn soap(
        &mut self,
        endpoint: &str,
        soap_action: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, SonosError>;
}

#[derive(Debug, Clone)]
pub struct SonosLanTransport {
    timeout: Duration,
    maximum_response_bytes: usize,
}

impl Default for SonosLanTransport {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl SonosLanTransport {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn with_maximum_response_bytes(mut self, maximum: usize) -> Self {
        self.maximum_response_bytes = maximum.max(1);
        self
    }

    fn execute(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, SonosError> {
        let url = Url::parse(endpoint)?;
        if url.scheme != "http" {
            return Err(SonosError::Validation(
                "Sonos UPnP requires local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or_else(|| SonosError::Validation("endpoint is missing a host".to_string()))?;
        let port = url
            .effective_port()
            .ok_or_else(|| SonosError::Validation("endpoint is missing a port".to_string()))?;
        let mut stream = connect_tcp(host, port, self.timeout)?;
        stream
            .write_all(request)
            .map_err(|error| SonosError::Io(error.to_string()))?;
        stream
            .flush()
            .map_err(|error| SonosError::Io(error.to_string()))?;
        let response = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&response, self.maximum_response_bytes)
    }
}

impl SonosTransport for SonosLanTransport {
    fn get(&mut self, endpoint: &str) -> Result<Vec<u8>, SonosError> {
        let url = Url::parse(endpoint)?;
        self.execute(endpoint, &encode_request(&url, "GET", &[], &[])?)
    }

    fn soap(
        &mut self,
        endpoint: &str,
        soap_action: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, SonosError> {
        let url = Url::parse(endpoint)?;
        self.execute(
            endpoint,
            &encode_request(
                &url,
                "POST",
                &[
                    ("Content-Type", "text/xml; charset=\"utf-8\""),
                    ("SOAPACTION", soap_action),
                ],
                body,
            )?,
        )
    }
}

pub struct SonosClient<T> {
    config: SonosConfig,
    transport: T,
    description: Option<SonosDeviceDescription>,
}

impl<T: SonosTransport> SonosClient<T> {
    pub fn new(config: SonosConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            description: None,
        }
    }

    pub fn config(&self) -> &SonosConfig {
        &self.config
    }

    pub fn inspect(&mut self) -> Result<SonosSnapshot, SonosError> {
        let description = parse_device_description(&self.transport.get(&self.config.setup_url)?)?;
        let av_transport_url = resolve_control_url(
            &self.config.setup_url,
            &description.av_transport.control_url,
        )?;
        let rendering_url = resolve_control_url(
            &self.config.setup_url,
            &description.rendering_control.control_url,
        )?;
        let transport = self.soap_action(
            &av_transport_url,
            AV_TRANSPORT_SERVICE_TYPE,
            "GetTransportInfo",
            "<InstanceID>0</InstanceID>",
        )?;
        let position = self.soap_action(
            &av_transport_url,
            AV_TRANSPORT_SERVICE_TYPE,
            "GetPositionInfo",
            "<InstanceID>0</InstanceID>",
        )?;
        let volume = self.soap_action(
            &rendering_url,
            RENDERING_CONTROL_SERVICE_TYPE,
            "GetVolume",
            "<InstanceID>0</InstanceID><Channel>Master</Channel>",
        )?;
        let mute = self.soap_action(
            &rendering_url,
            RENDERING_CONTROL_SERVICE_TYPE,
            "GetMute",
            "<InstanceID>0</InstanceID><Channel>Master</Channel>",
        )?;
        let transport_state = required_response_text(&transport, "CurrentTransportState")?;
        let volume = parse_percentage(&required_response_text(&volume, "CurrentVolume")?)?;
        let muted = parse_boolean(&required_response_text(&mute, "CurrentMute")?)?;
        let track_uri = response_text(&position, "TrackURI")?;
        let metadata = response_text(&position, "TrackMetaData")?;
        let (track_title, track_artist) = metadata
            .as_deref()
            .map(parse_didl_metadata)
            .transpose()?
            .unwrap_or((None, None));
        self.description = Some(description.clone());
        Ok(SonosSnapshot {
            device: description,
            transport_state,
            volume,
            muted,
            track_uri,
            track_title,
            track_artist,
        })
    }

    fn soap_action(
        &mut self,
        control_url: &str,
        service_type: &str,
        action: &str,
        arguments: &str,
    ) -> Result<Vec<u8>, SonosError> {
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"{service_type}\">{arguments}</u:{action}></s:Body></s:Envelope>"
        );
        let soap_action = format!("\"{service_type}#{action}\"");
        self.transport
            .soap(control_url, &soap_action, body.as_bytes())
    }
}

pub struct SonosRuntimeIntegration<T> {
    client: SonosClient<T>,
}

impl<T: SonosTransport> SonosRuntimeIntegration<T> {
    pub fn new(client: SonosClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<EntityId, SonosError> {
        let decision = runtime.authorize_tool_for_principal(
            principal_id.clone(),
            SmartHomeTool::GetState,
            observed_at_ms,
        );
        if !decision.missing_capabilities.is_empty() {
            return Err(SonosError::Runtime(RuntimeError::UnauthorizedTool {
                principal_id,
                tool: SmartHomeTool::GetState,
                missing_capabilities: decision.missing_capabilities,
            }));
        }
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

pub fn parse_device_description(bytes: &[u8]) -> Result<SonosDeviceDescription, SonosError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| SonosError::Xml("device description is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| SonosError::Xml(error.to_string()))?;
    let device = descendant(&document.root, "device")
        .ok_or_else(|| SonosError::Xml("description is missing device".to_string()))?;
    let mut services = Vec::new();
    collect_descendants(device, "service", &mut services);
    let service = |service_type: &'static str| {
        services
            .iter()
            .find_map(|service| {
                let found_type = child_text(service, "serviceType")?;
                if found_type != service_type {
                    return None;
                }
                Some(SonosService {
                    service_type: found_type,
                    control_url: child_text(service, "controlURL").unwrap_or_default(),
                    event_subscription_url: child_text(service, "eventSubURL"),
                })
            })
            .filter(|service| !service.control_url.is_empty())
            .ok_or(SonosError::MissingService(service_type))
    };
    Ok(SonosDeviceDescription {
        friendly_name: required_child_text(device, "friendlyName")?,
        model_name: required_child_text(device, "modelName")?,
        model_number: child_text(device, "modelNumber"),
        serial_number: required_child_text(device, "serialNumber")?,
        udn: required_child_text(device, "UDN")?,
        firmware_version: child_text(device, "softwareVersion")
            .or_else(|| child_text(device, "firmwareVersion")),
        room_name: child_text(device, "roomName"),
        av_transport: service(AV_TRANSPORT_SERVICE_TYPE)?,
        rendering_control: service(RENDERING_CONTROL_SERVICE_TYPE)?,
    })
}

pub fn response_text(bytes: &[u8], name: &str) -> Result<Option<String>, SonosError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| SonosError::Xml("SOAP response is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| SonosError::Xml(error.to_string()))?;
    if descendant(&document.root, "Fault").is_some() {
        return Err(SonosError::Xml(
            "SOAP response contains a fault".to_string(),
        ));
    }
    Ok(descendant(&document.root, name)
        .map(XmlElement::text_content)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && value != "NOT_IMPLEMENTED"))
}

fn required_response_text(bytes: &[u8], name: &str) -> Result<String, SonosError> {
    response_text(bytes, name)?
        .ok_or_else(|| SonosError::Xml(format!("SOAP response is missing {name}")))
}

fn parse_percentage(value: &str) -> Result<u8, SonosError> {
    value
        .parse::<u8>()
        .ok()
        .filter(|value| *value <= 100)
        .ok_or_else(|| SonosError::Xml(format!("invalid Sonos percentage `{value}`")))
}

fn parse_boolean(value: &str) -> Result<bool, SonosError> {
    match value {
        "0" | "false" => Ok(false),
        "1" | "true" => Ok(true),
        _ => Err(SonosError::Xml(format!("invalid Sonos boolean `{value}`"))),
    }
}

pub fn parse_didl_metadata(source: &str) -> Result<(Option<String>, Option<String>), SonosError> {
    if source.trim().is_empty() {
        return Ok((None, None));
    }
    let document = parse_xml(source).map_err(|error| SonosError::Xml(error.to_string()))?;
    Ok((
        descendant(&document.root, "title")
            .map(XmlElement::text_content)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        descendant(&document.root, "creator")
            .or_else(|| descendant(&document.root, "artist"))
            .map(XmlElement::text_content)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
    ))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &SonosConfig,
    snapshot: &SonosSnapshot,
    observed_at_ms: u64,
) -> Result<EntityId, SonosError> {
    let native_id = stable_component(&snapshot.device.serial_number);
    if native_id.is_empty() {
        return Err(SonosError::Validation("device serial is empty".to_string()));
    }
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.setup_url.clone());
    bridge.hardware_model = Some(snapshot.device.model_name.clone());
    bridge.firmware_version = snapshot.device.firmware_version.clone();
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("udn", &snapshot.device.udn)?];
    bridge.metadata = vec![Metadata::new("sonos.protocol", PROTOCOL_ID)];
    runtime.upsert_bridge(bridge)?;

    let device_id = DeviceId::trusted(format!("sonos:{native_id}"));
    let entity_id = EntityId::trusted(format!("sonos:{native_id}:player"));
    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: "Sonos".to_string(),
        model: snapshot.device.model_name.clone(),
        name: snapshot.device.friendly_name.clone(),
        serial: Some(snapshot.device.serial_number.clone()),
        firmware_version: snapshot.device.firmware_version.clone(),
        room_id: None,
        entity_ids: vec![entity_id.clone()],
        identifiers: vec![protocol_identifier(
            "serial",
            &snapshot.device.serial_number,
        )?],
        health: Health::Online,
        metadata: Vec::new(),
    })?;
    let state = Value::Object(vec![
        (
            "transport_state".to_string(),
            Value::Text(snapshot.transport_state.clone()),
        ),
        ("volume".to_string(), Value::Percentage(snapshot.volume)),
        ("muted".to_string(), Value::Bool(snapshot.muted)),
        (
            "track_uri".to_string(),
            snapshot
                .track_uri
                .clone()
                .map(Value::Text)
                .unwrap_or(Value::Null),
        ),
        (
            "track_title".to_string(),
            snapshot
                .track_title
                .clone()
                .map(Value::Text)
                .unwrap_or(Value::Null),
        ),
        (
            "track_artist".to_string(),
            snapshot
                .track_artist
                .clone()
                .map(Value::Text)
                .unwrap_or(Value::Null),
        ),
    ]);
    runtime.upsert_entity(Entity {
        entity_id: entity_id.clone(),
        device_id,
        kind: EntityKind::Unknown,
        name: snapshot
            .device
            .room_name
            .clone()
            .unwrap_or_else(|| snapshot.device.friendly_name.clone()),
        capabilities: vec![Capability::new(
            CapabilityId::trusted("media.player_state"),
            CapabilityMode::Observe,
            ValueKind::Object,
        )],
        state: Some(StateSnapshot {
            entity_id: entity_id.clone(),
            value: state,
            source: StateSource::Poll,
            observed_at_ms,
            received_at_ms: observed_at_ms,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        }),
        metadata: vec![Metadata::new("sonos.control_surface", "read_only_player")],
    })?;
    Ok(entity_id)
}

fn child_text(root: &XmlElement, name: &str) -> Option<String> {
    root.children
        .iter()
        .find_map(|child| match child {
            XmlNode::Element(element) if element.local_name == name => Some(element),
            _ => None,
        })
        .map(|element| element.text_content())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn required_child_text(root: &XmlElement, name: &str) -> Result<String, SonosError> {
    child_text(root, name).ok_or_else(|| SonosError::Xml(format!("device is missing {name}")))
}

fn descendant<'a>(root: &'a XmlElement, name: &str) -> Option<&'a XmlElement> {
    if root.local_name == name {
        return Some(root);
    }
    root.children.iter().find_map(|child| match child {
        XmlNode::Element(element) => descendant(element, name),
        XmlNode::Text(_)
        | XmlNode::CData(_)
        | XmlNode::Comment(_)
        | XmlNode::ProcessingInstruction { .. } => None,
    })
}

fn collect_descendants<'a>(root: &'a XmlElement, name: &str, output: &mut Vec<&'a XmlElement>) {
    if root.local_name == name {
        output.push(root);
    }
    for child in &root.children {
        if let XmlNode::Element(element) = child {
            collect_descendants(element, name, output);
        }
    }
}

fn resolve_control_url(setup_url: &str, control_url: &str) -> Result<String, SonosError> {
    if control_url.starts_with("http://") {
        return Ok(control_url.to_string());
    }
    let setup = Url::parse(setup_url)?;
    let host = setup
        .host
        .as_deref()
        .ok_or_else(|| SonosError::Validation("setup URL is missing a host".to_string()))?;
    let authority = setup
        .port
        .map_or_else(|| host.to_string(), |port| format!("{host}:{port}"));
    let path = if control_url.starts_with('/') {
        control_url.to_string()
    } else {
        let directory = setup.path.rsplit_once('/').map_or("/", |(dir, _)| dir);
        format!("{directory}/{control_url}")
    };
    Ok(format!("http://{authority}{path}"))
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, SonosError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| SonosError::Validation(error.to_string()))
}

fn stable_component(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn encode_request(
    url: &Url,
    method: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) -> Result<Vec<u8>, SonosError> {
    let host = url
        .host
        .as_deref()
        .ok_or_else(|| SonosError::Validation("endpoint is missing a host".to_string()))?;
    if url.path.contains(['\r', '\n'])
        || headers
            .iter()
            .any(|(name, value)| name.contains(['\r', '\n']) || value.contains(['\r', '\n']))
    {
        return Err(SonosError::Validation(
            "unsafe HTTP request text".to_string(),
        ));
    }
    let host_header = url
        .port
        .map_or_else(|| host.to_string(), |port| format!("{host}:{port}"));
    let mut request = format!(
        "{method} {} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\nAccept: text/xml\r\n",
        url.path
    )
    .into_bytes();
    for (name, value) in headers {
        request.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
    }
    if !body.is_empty() {
        request.extend_from_slice(format!("Content-Length: {}\r\n", body.len()).as_bytes());
    }
    request.extend_from_slice(b"\r\n");
    request.extend_from_slice(body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, SonosError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| SonosError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| SonosError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| SonosError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(SonosError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, SonosError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| SonosError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(SonosError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, SonosError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| SonosError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(SonosError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(SonosError::TruncatedBody {
                    expected,
                    actual: input.len(),
                });
            }
            input[..expected].to_vec()
        }
        BodyKind::UntilEof => input.to_vec(),
        BodyKind::Chunked => {
            return Err(SonosError::Http(
                "chunked Sonos responses are unsupported".to_string(),
            ))
        }
    };
    if body.len() > maximum {
        return Err(SonosError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::net::{TcpListener, UdpSocket};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    const SETUP: &str = r#"<root xmlns="urn:schemas-upnp-org:device-1-0"><device><deviceType>urn:schemas-upnp-org:device:ZonePlayer:1</deviceType><friendlyName>Living Room</friendlyName><roomName>Living Room</roomName><modelName>Sonos One</modelName><modelNumber>S13</modelNumber><softwareVersion>82.3-60160</softwareVersion><serialNumber>34-7E-5C-AA-BB-CC:8</serialNumber><UDN>uuid:RINCON_347E5CAABBCC01400</UDN><serviceList><service><serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType><controlURL>/MediaRenderer/AVTransport/Control</controlURL><eventSubURL>/MediaRenderer/AVTransport/Event</eventSubURL></service><service><serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType><controlURL>/MediaRenderer/RenderingControl/Control</controlURL><eventSubURL>/MediaRenderer/RenderingControl/Event</eventSubURL></service></serviceList></device></root>"#;
    const TRANSPORT: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetTransportInfoResponse xmlns:u="urn:schemas-upnp-org:service:AVTransport:1"><CurrentTransportState>PLAYING</CurrentTransportState><CurrentTransportStatus>OK</CurrentTransportStatus><CurrentSpeed>1</CurrentSpeed></u:GetTransportInfoResponse></s:Body></s:Envelope>"#;
    const POSITION: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetPositionInfoResponse xmlns:u="urn:schemas-upnp-org:service:AVTransport:1"><TrackURI>x-sonos-http:track.mp3</TrackURI><TrackMetaData>&lt;DIDL-Lite xmlns:dc=&quot;http://purl.org/dc/elements/1.1/&quot;&gt;&lt;item&gt;&lt;dc:title&gt;Night Drive&lt;/dc:title&gt;&lt;dc:creator&gt;The Tests&lt;/dc:creator&gt;&lt;/item&gt;&lt;/DIDL-Lite&gt;</TrackMetaData></u:GetPositionInfoResponse></s:Body></s:Envelope>"#;
    const VOLUME: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetVolumeResponse xmlns:u="urn:schemas-upnp-org:service:RenderingControl:1"><CurrentVolume>27</CurrentVolume></u:GetVolumeResponse></s:Body></s:Envelope>"#;
    const MUTE: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetMuteResponse xmlns:u="urn:schemas-upnp-org:service:RenderingControl:1"><CurrentMute>0</CurrentMute></u:GetMuteResponse></s:Body></s:Envelope>"#;

    #[test]
    fn parses_ssdp_description_and_player_state() {
        let candidate = parse_ssdp_response(
            b"HTTP/1.1 200 OK\r\nST: urn:schemas-upnp-org:device:ZonePlayer:1\r\nUSN: uuid:RINCON_347E5CAABBCC01400::urn:schemas-upnp-org:device:ZonePlayer:1\r\nLOCATION: http://127.0.0.1:1400/xml/device_description.xml\r\nSERVER: Linux UPnP/1.0 Sonos/82.3-60160\r\n\r\n",
        )
        .unwrap();
        let description = parse_device_description(SETUP.as_bytes()).unwrap();
        assert_eq!(
            candidate.location,
            "http://127.0.0.1:1400/xml/device_description.xml"
        );
        assert_eq!(description.friendly_name, "Living Room");
        assert_eq!(description.room_name.as_deref(), Some("Living Room"));
        assert_eq!(
            required_response_text(TRANSPORT.as_bytes(), "CurrentTransportState").unwrap(),
            "PLAYING"
        );
        assert_eq!(parse_percentage("27").unwrap(), 27);
        assert!(!parse_boolean("0").unwrap());
        let metadata = response_text(POSITION.as_bytes(), "TrackMetaData")
            .unwrap()
            .unwrap();
        assert_eq!(
            parse_didl_metadata(&metadata).unwrap(),
            (
                Some("Night Drive".to_string()),
                Some("The Tests".to_string())
            )
        );
    }

    #[test]
    fn authorization_denies_before_transport_io() {
        struct CountingTransport(Arc<AtomicUsize>);
        impl SonosTransport for CountingTransport {
            fn get(&mut self, _endpoint: &str) -> Result<Vec<u8>, SonosError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(Vec::new())
            }
            fn soap(
                &mut self,
                _endpoint: &str,
                _soap_action: &str,
                _body: &[u8],
            ) -> Result<Vec<u8>, SonosError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(Vec::new())
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let config = SonosConfig::new(
            BridgeId::trusted("sonos:test"),
            "http://127.0.0.1:1400/xml/device_description.xml",
        )
        .unwrap();
        let client = SonosClient::new(config, CountingTransport(Arc::clone(&calls)));
        let mut integration = SonosRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let error = integration
            .inspect_and_install_authorized(&mut runtime, AgentId::trusted("agent:test"), 10)
            .unwrap_err();
        assert!(matches!(
            error,
            SonosError::Runtime(RuntimeError::UnauthorizedTool { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn loopback_discovery_inspection_and_install() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let http_address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let http_handle = thread::spawn(move || {
            for body in [SETUP, TRANSPORT, POSITION, VOLUME, MUTE] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 4096];
                let read = stream.read(&mut request).unwrap();
                server_requests
                    .lock()
                    .unwrap()
                    .push(String::from_utf8_lossy(&request[..read]).to_string());
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
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
                "HTTP/1.1 200 OK\r\nST: {ZONE_PLAYER_DEVICE_TYPE}\r\nUSN: uuid:RINCON_347E5CAABBCC01400::{ZONE_PLAYER_DEVICE_TYPE}\r\nLOCATION: http://{http_address}/xml/device_description.xml\r\nSERVER: Linux UPnP/1.0 Sonos/82.3-60160\r\n\r\n"
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
            SonosConfig::new(BridgeId::trusted("sonos:test"), &candidates[0].location).unwrap();
        let client = SonosClient::new(config, SonosLanTransport::default());
        let mut integration = SonosRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:sonos-test");
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:sonos-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1,
            )
            .with_expiry(100),
        );
        let entity_id = integration
            .inspect_and_install_authorized(&mut runtime, principal, 10)
            .unwrap();
        let entity = runtime.registry().entity(&entity_id).unwrap();
        assert_eq!(entity.kind, EntityKind::Unknown);
        assert_eq!(
            entity.capabilities[0].capability_id.as_str(),
            "media.player_state"
        );
        udp_handle.join().unwrap();
        http_handle.join().unwrap();
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /xml/device_description.xml HTTP/1.1"));
        assert!(requests[1].contains("GetTransportInfo"));
        assert!(requests[2].contains("GetPositionInfo"));
        assert!(requests[3].contains("GetVolume"));
        assert!(requests[4].contains("GetMute"));
    }
}
