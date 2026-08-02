//! Wemo UPnP discovery, inspection, and light-switch control for D23.

#![forbid(unsafe_code)]

use coding_adventures_xml_parser::{parse_xml, XmlElement, XmlNode};
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandType, Device, DeviceId, Entity, EntityId, EntityKind, Health,
    IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, SmartHomeTool, StateConfidence,
    StateSnapshot, StateSource, Value, ValueKind,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use udp_client::{send_to_and_collect, UdpDiscoveryEndpoint, UdpError, UdpOptions};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "wemo";
pub const PROTOCOL_ID: &str = "wemo_upnp";
pub const BASICEVENT_SERVICE_TYPE: &str = "urn:Belkin:service:basicevent:1";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug)]
pub enum WemoError {
    Validation(String),
    Url(UrlError),
    Udp(UdpError),
    Io(String),
    Http(String),
    HttpStatus(u16),
    Xml(String),
    ResponseTooLarge { limit: usize },
    TruncatedBody { expected: usize, actual: usize },
    MissingBasicEventService,
    UnsupportedCommand(CommandType),
    UnknownEntity(EntityId),
    Runtime(RuntimeError),
}

impl fmt::Display for WemoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Wemo input: {message}"),
            Self::Url(error) => write!(formatter, "invalid Wemo URL: {error}"),
            Self::Udp(error) => write!(formatter, "Wemo SSDP failed: {error}"),
            Self::Io(message) => write!(formatter, "Wemo LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Wemo HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "Wemo endpoint returned HTTP {status}"),
            Self::Xml(message) => write!(formatter, "invalid Wemo XML: {message}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Wemo response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Wemo response body is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::MissingBasicEventService => {
                formatter.write_str("Wemo description has no basicevent service")
            }
            Self::UnsupportedCommand(command) => {
                write!(formatter, "Wemo integration does not support {command:?}")
            }
            Self::UnknownEntity(entity_id) => write!(formatter, "unknown Wemo entity {entity_id}"),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WemoError {}

impl From<UrlError> for WemoError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<UdpError> for WemoError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<RuntimeError> for WemoError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WemoConfig {
    pub bridge_id: BridgeId,
    pub setup_url: String,
}

impl WemoConfig {
    pub fn new(bridge_id: BridgeId, setup_url: impl Into<String>) -> Result<Self, WemoError> {
        let setup_url = setup_url.into();
        let parsed = Url::parse(&setup_url)?;
        if parsed.scheme != "http"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || parsed.path.is_empty()
        {
            return Err(WemoError::Validation(
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
pub struct WemoSsdpCandidate {
    pub location: String,
    pub usn: String,
    pub server: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WemoService {
    pub service_type: String,
    pub control_url: String,
    pub event_subscription_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WemoDeviceDescription {
    pub friendly_name: String,
    pub model_name: String,
    pub model_number: Option<String>,
    pub serial_number: String,
    pub udn: String,
    pub firmware_version: Option<String>,
    pub basic_event: WemoService,
}

impl WemoDeviceDescription {
    pub fn supports_light_commands(&self) -> bool {
        let model = self.model_name.to_ascii_lowercase();
        model.contains("lightswitch") || model.contains("dimmer")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WemoSnapshot {
    pub device: WemoDeviceDescription,
    pub on: bool,
}

pub fn ssdp_search_request() -> Vec<u8> {
    format!(
        "M-SEARCH * HTTP/1.1\r\nST: {BASICEVENT_SERVICE_TYPE}\r\nMX: 2\r\nMAN: \"ssdp:discover\"\r\nHOST: 239.255.255.250:1900\r\n\r\n"
    )
    .into_bytes()
}

pub fn discover_ssdp_ipv4(
    timeout: Duration,
    max_responses: usize,
) -> Result<Vec<WemoSsdpCandidate>, WemoError> {
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
) -> Result<Vec<WemoSsdpCandidate>, WemoError> {
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

pub fn parse_ssdp_response(bytes: &[u8]) -> Result<WemoSsdpCandidate, WemoError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| WemoError::Validation("SSDP response is not UTF-8".to_string()))?;
    let mut lines = source.split("\r\n");
    let status = lines.next().unwrap_or_default();
    if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
        return Err(WemoError::Validation(
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
    if !st.eq_ignore_ascii_case(BASICEVENT_SERVICE_TYPE) {
        return Err(WemoError::Validation(format!(
            "unexpected SSDP search target `{st}`"
        )));
    }
    let location = required_header(&headers, "location")?;
    WemoConfig::new(BridgeId::trusted("wemo.ssdp.validation"), location)?;
    Ok(WemoSsdpCandidate {
        location: location.to_string(),
        usn: required_header(&headers, "usn")?.to_string(),
        server: headers.get("server").cloned(),
    })
}

fn required_header<'a>(
    headers: &'a BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, WemoError> {
    headers
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| WemoError::Validation(format!("SSDP response is missing {name}")))
}

pub fn discovery_record(
    candidate: &WemoSsdpCandidate,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, WemoError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(candidate.usn.split("::").next().unwrap_or(&candidate.usn)),
        DiscoverySource::Ssdp,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| WemoError::Validation(error.to_string()))?
    .with_display_name("Wemo UPnP device")
    .with_address(candidate.location.clone())
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata(
        "smart_home.discovery.search_target",
        BASICEVENT_SERVICE_TYPE,
    ))
}

pub trait WemoTransport {
    fn get(&mut self, endpoint: &str) -> Result<Vec<u8>, WemoError>;
    fn soap(
        &mut self,
        endpoint: &str,
        soap_action: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, WemoError>;
}

#[derive(Debug, Clone)]
pub struct WemoLanTransport {
    timeout: Duration,
    maximum_response_bytes: usize,
}

impl Default for WemoLanTransport {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl WemoLanTransport {
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn with_maximum_response_bytes(mut self, maximum: usize) -> Self {
        self.maximum_response_bytes = maximum.max(1);
        self
    }

    fn execute(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, WemoError> {
        let url = Url::parse(endpoint)?;
        if url.scheme != "http" {
            return Err(WemoError::Validation(
                "Wemo UPnP requires local HTTP".to_string(),
            ));
        }
        let host = url
            .host
            .as_deref()
            .ok_or_else(|| WemoError::Validation("endpoint is missing a host".to_string()))?;
        let port = url
            .effective_port()
            .ok_or_else(|| WemoError::Validation("endpoint is missing a port".to_string()))?;
        let mut stream = connect_tcp(host, port, self.timeout)?;
        stream
            .write_all(request)
            .map_err(|error| WemoError::Io(error.to_string()))?;
        stream
            .flush()
            .map_err(|error| WemoError::Io(error.to_string()))?;
        let response = read_bounded(&mut stream, self.maximum_response_bytes)?;
        decode_http_response(&response, self.maximum_response_bytes)
    }
}

impl WemoTransport for WemoLanTransport {
    fn get(&mut self, endpoint: &str) -> Result<Vec<u8>, WemoError> {
        let url = Url::parse(endpoint)?;
        self.execute(endpoint, &encode_request(&url, "GET", &[], &[])?)
    }

    fn soap(
        &mut self,
        endpoint: &str,
        soap_action: &str,
        body: &[u8],
    ) -> Result<Vec<u8>, WemoError> {
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

pub struct WemoClient<T> {
    config: WemoConfig,
    transport: T,
    description: Option<WemoDeviceDescription>,
}

impl<T: WemoTransport> WemoClient<T> {
    pub fn new(config: WemoConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            description: None,
        }
    }

    pub fn config(&self) -> &WemoConfig {
        &self.config
    }

    pub fn inspect(&mut self) -> Result<WemoSnapshot, WemoError> {
        let description = parse_device_description(&self.transport.get(&self.config.setup_url)?)?;
        let control_url =
            resolve_control_url(&self.config.setup_url, &description.basic_event.control_url)?;
        let response = self.soap_action(&control_url, "GetBinaryState", None)?;
        let on = parse_binary_state_response(&response)?;
        self.description = Some(description.clone());
        Ok(WemoSnapshot {
            device: description,
            on,
        })
    }

    pub fn set_binary_state(&mut self, on: bool) -> Result<bool, WemoError> {
        let description = self
            .description
            .as_ref()
            .ok_or_else(|| WemoError::Validation("device must be inspected first".to_string()))?;
        let control_url =
            resolve_control_url(&self.config.setup_url, &description.basic_event.control_url)?;
        let response = self.soap_action(
            &control_url,
            "SetBinaryState",
            Some(if on { "1" } else { "0" }),
        )?;
        parse_binary_state_response(&response)
    }

    fn soap_action(
        &mut self,
        control_url: &str,
        action: &str,
        binary_state: Option<&str>,
    ) -> Result<Vec<u8>, WemoError> {
        let argument = binary_state
            .map(|value| format!("<BinaryState>{value}</BinaryState>"))
            .unwrap_or_default();
        let body = format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><s:Envelope xmlns:s=\"http://schemas.xmlsoap.org/soap/envelope/\" s:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\"><s:Body><u:{action} xmlns:u=\"{BASICEVENT_SERVICE_TYPE}\">{argument}</u:{action}></s:Body></s:Envelope>"
        );
        let soap_action = format!("\"{BASICEVENT_SERVICE_TYPE}#{action}\"");
        self.transport
            .soap(control_url, &soap_action, body.as_bytes())
    }
}

pub struct WemoRuntimeIntegration<T> {
    client: WemoClient<T>,
    controllable_entities: BTreeSet<EntityId>,
}

impl<T: WemoTransport> WemoRuntimeIntegration<T> {
    pub fn new(client: WemoClient<T>) -> Self {
        Self {
            client,
            controllable_entities: BTreeSet::new(),
        }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<EntityId, WemoError> {
        let decision = runtime.authorize_tool_for_principal(
            principal_id.clone(),
            SmartHomeTool::GetState,
            observed_at_ms,
        );
        if !decision.missing_capabilities.is_empty() {
            return Err(WemoError::Runtime(RuntimeError::UnauthorizedTool {
                principal_id,
                tool: SmartHomeTool::GetState,
                missing_capabilities: decision.missing_capabilities,
            }));
        }
        let snapshot = self.client.inspect()?;
        let entity_id = install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)?;
        if snapshot.device.supports_light_commands() {
            self.controllable_entities.insert(entity_id.clone());
        }
        Ok(entity_id)
    }

    pub fn dispatch_command(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, WemoError> {
        if !self.controllable_entities.contains(&request.entity_id) {
            return Err(WemoError::UnknownEntity(request.entity_id));
        }
        let on = match request.command_type {
            CommandType::TurnOn => true,
            CommandType::TurnOff => false,
            command => return Err(WemoError::UnsupportedCommand(command)),
        };
        let result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        self.client.set_binary_state(on)?;
        Ok(result)
    }
}

pub fn parse_device_description(bytes: &[u8]) -> Result<WemoDeviceDescription, WemoError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| WemoError::Xml("device description is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| WemoError::Xml(error.to_string()))?;
    let device = descendant(&document.root, "device")
        .ok_or_else(|| WemoError::Xml("description is missing device".to_string()))?;
    let mut services = Vec::new();
    collect_descendants(device, "service", &mut services);
    let basic_event = services
        .into_iter()
        .find_map(|service| {
            let service_type = child_text(service, "serviceType")?;
            if service_type != BASICEVENT_SERVICE_TYPE {
                return None;
            }
            Some(WemoService {
                service_type,
                control_url: child_text(service, "controlURL").unwrap_or_default(),
                event_subscription_url: child_text(service, "eventSubURL"),
            })
        })
        .filter(|service| !service.control_url.is_empty())
        .ok_or(WemoError::MissingBasicEventService)?;
    Ok(WemoDeviceDescription {
        friendly_name: required_child_text(device, "friendlyName")?,
        model_name: required_child_text(device, "modelName")?,
        model_number: child_text(device, "modelNumber"),
        serial_number: required_child_text(device, "serialNumber")?,
        udn: required_child_text(device, "UDN")?,
        firmware_version: child_text(device, "firmwareVersion"),
        basic_event,
    })
}

pub fn parse_binary_state_response(bytes: &[u8]) -> Result<bool, WemoError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| WemoError::Xml("SOAP response is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| WemoError::Xml(error.to_string()))?;
    if descendant(&document.root, "Fault").is_some() {
        return Err(WemoError::Xml("SOAP response contains a fault".to_string()));
    }
    let state = descendant(&document.root, "BinaryState")
        .map(|element| element.text_content())
        .ok_or_else(|| WemoError::Xml("SOAP response is missing BinaryState".to_string()))?;
    match state.trim().split('|').next().unwrap_or_default() {
        "0" => Ok(false),
        "1" | "8" => Ok(true),
        value => Err(WemoError::Xml(format!("unsupported BinaryState `{value}`"))),
    }
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &WemoConfig,
    snapshot: &WemoSnapshot,
    observed_at_ms: u64,
) -> Result<EntityId, WemoError> {
    let native_id = stable_component(&snapshot.device.serial_number);
    if native_id.is_empty() {
        return Err(WemoError::Validation("device serial is empty".to_string()));
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
    bridge.metadata = vec![Metadata::new("wemo.protocol", PROTOCOL_ID)];
    runtime.upsert_bridge(bridge)?;

    let device_id = DeviceId::trusted(format!("wemo:{native_id}"));
    let entity_id = EntityId::trusted(format!("wemo:{native_id}:binary"));
    let is_light = snapshot.device.supports_light_commands();
    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: "Belkin Wemo".to_string(),
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
    let capability = if is_light {
        Capability::new(
            CapabilityId::trusted("light.on_off"),
            CapabilityMode::ObserveAndCommand,
            ValueKind::Boolean,
        )
    } else {
        Capability::new(
            CapabilityId::trusted("switch.binary_state"),
            CapabilityMode::Observe,
            ValueKind::Boolean,
        )
    };
    runtime.upsert_entity(Entity {
        entity_id: entity_id.clone(),
        device_id,
        kind: if is_light {
            EntityKind::Light
        } else {
            EntityKind::Switch
        },
        name: snapshot.device.friendly_name.clone(),
        capabilities: vec![capability],
        state: Some(StateSnapshot {
            entity_id: entity_id.clone(),
            value: Value::Bool(snapshot.on),
            source: StateSource::Poll,
            observed_at_ms,
            received_at_ms: observed_at_ms,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        }),
        metadata: vec![Metadata::new(
            "wemo.control_surface",
            if is_light {
                "light"
            } else {
                "read_only_switch"
            },
        )],
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

fn required_child_text(root: &XmlElement, name: &str) -> Result<String, WemoError> {
    child_text(root, name).ok_or_else(|| WemoError::Xml(format!("device is missing {name}")))
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

fn resolve_control_url(setup_url: &str, control_url: &str) -> Result<String, WemoError> {
    if control_url.starts_with("http://") {
        return Ok(control_url.to_string());
    }
    let setup = Url::parse(setup_url)?;
    let host = setup
        .host
        .as_deref()
        .ok_or_else(|| WemoError::Validation("setup URL is missing a host".to_string()))?;
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

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, WemoError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| WemoError::Validation(error.to_string()))
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
) -> Result<Vec<u8>, WemoError> {
    let host = url
        .host
        .as_deref()
        .ok_or_else(|| WemoError::Validation("endpoint is missing a host".to_string()))?;
    if url.path.contains(['\r', '\n'])
        || headers
            .iter()
            .any(|(name, value)| name.contains(['\r', '\n']) || value.contains(['\r', '\n']))
    {
        return Err(WemoError::Validation(
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

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, WemoError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| WemoError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| WemoError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| WemoError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(WemoError::Io(
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

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, WemoError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| WemoError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(WemoError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, WemoError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| WemoError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(WemoError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(WemoError::TruncatedBody {
                    expected,
                    actual: input.len(),
                });
            }
            input[..expected].to_vec()
        }
        BodyKind::UntilEof => input.to_vec(),
        BodyKind::Chunked => {
            return Err(WemoError::Http(
                "chunked Wemo responses are unsupported".to_string(),
            ))
        }
    };
    if body.len() > maximum {
        return Err(WemoError::ResponseTooLarge { limit: maximum });
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

    const SETUP: &str = r#"<root xmlns="urn:Belkin:device-1-0"><device><deviceType>urn:Belkin:device:controllee:1</deviceType><friendlyName>Hall Light</friendlyName><modelName>LightSwitch</modelName><modelNumber>F7C030</modelNumber><firmwareVersion>WeMo_WW_2.00.11420.PVT</firmwareVersion><serialNumber>221423K0101769</serialNumber><UDN>uuid:Socket-1_0-221423K0101769</UDN><serviceList><service><serviceType>urn:Belkin:service:basicevent:1</serviceType><serviceId>urn:Belkin:serviceId:basicevent1</serviceId><controlURL>/upnp/control/basicevent1</controlURL><eventSubURL>/upnp/event/basicevent1</eventSubURL><SCPDURL>/eventservice.xml</SCPDURL></service></serviceList></device></root>"#;
    const GET_STATE: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:GetBinaryStateResponse xmlns:u="urn:Belkin:service:basicevent:1"><BinaryState>1|1492338954|0</BinaryState></u:GetBinaryStateResponse></s:Body></s:Envelope>"#;
    const SET_STATE: &str = r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"><s:Body><u:SetBinaryStateResponse xmlns:u="urn:Belkin:service:basicevent:1"><BinaryState>0</BinaryState></u:SetBinaryStateResponse></s:Body></s:Envelope>"#;

    #[test]
    fn parses_ssdp_description_and_state() {
        let candidate = parse_ssdp_response(
            b"HTTP/1.1 200 OK\r\nST: urn:Belkin:service:basicevent:1\r\nUSN: uuid:Socket-1_0-221423::urn:Belkin:service:basicevent:1\r\nLOCATION: http://127.0.0.1:49153/setup.xml\r\nSERVER: Unspecified, UPnP/1.0, Unspecified\r\n\r\n",
        )
        .unwrap();
        let description = parse_device_description(SETUP.as_bytes()).unwrap();
        assert_eq!(candidate.location, "http://127.0.0.1:49153/setup.xml");
        assert_eq!(description.friendly_name, "Hall Light");
        assert!(description.supports_light_commands());
        assert!(parse_binary_state_response(GET_STATE.as_bytes()).unwrap());
    }

    #[test]
    fn authorization_denies_before_transport_io() {
        struct CountingTransport(Arc<AtomicUsize>);
        impl WemoTransport for CountingTransport {
            fn get(&mut self, _endpoint: &str) -> Result<Vec<u8>, WemoError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(Vec::new())
            }
            fn soap(
                &mut self,
                _endpoint: &str,
                _soap_action: &str,
                _body: &[u8],
            ) -> Result<Vec<u8>, WemoError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(Vec::new())
            }
        }
        let calls = Arc::new(AtomicUsize::new(0));
        let config = WemoConfig::new(
            BridgeId::trusted("wemo:test"),
            "http://127.0.0.1:49153/setup.xml",
        )
        .unwrap();
        let client = WemoClient::new(config, CountingTransport(Arc::clone(&calls)));
        let mut integration = WemoRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let error = integration
            .inspect_and_install_authorized(&mut runtime, AgentId::trusted("agent:test"), 10)
            .unwrap_err();
        assert!(matches!(
            error,
            WemoError::Runtime(RuntimeError::UnauthorizedTool { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn loopback_discovery_inspection_install_and_command() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let http_address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let http_handle = thread::spawn(move || {
            for body in [SETUP, GET_STATE, SET_STATE] {
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
                "HTTP/1.1 200 OK\r\nST: {BASICEVENT_SERVICE_TYPE}\r\nUSN: uuid:Socket-1_0-221423::urn:Belkin:service:basicevent:1\r\nLOCATION: http://{http_address}/setup.xml\r\nSERVER: Unspecified, UPnP/1.0, Unspecified\r\n\r\n"
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
            WemoConfig::new(BridgeId::trusted("wemo:test"), &candidates[0].location).unwrap();
        let client = WemoClient::new(config, WemoLanTransport::default());
        let mut integration = WemoRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:wemo-test");
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:wemo-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1,
            )
            .with_expiry(100),
        );
        let entity_id = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 10)
            .unwrap();
        let result = integration
            .dispatch_command(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(entity_id, CommandType::TurnOff, Value::Null),
                11,
            )
            .unwrap();
        assert_eq!(result.status, smart_home_core::CommandStatus::Accepted);
        udp_handle.join().unwrap();
        http_handle.join().unwrap();
        let requests = requests.lock().unwrap();
        assert!(requests[0].starts_with("GET /setup.xml HTTP/1.1"));
        assert!(requests[1].contains("GetBinaryState"));
        assert!(requests[2].contains("SetBinaryState"));
        assert!(requests[2].contains("<BinaryState>0</BinaryState>"));
    }
}
