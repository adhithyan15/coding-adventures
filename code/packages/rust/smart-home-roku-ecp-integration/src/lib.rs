//! Bounded Roku External Control Protocol discovery, telemetry, and media control for D23.

#![forbid(unsafe_code)]

use coding_adventures_xml_parser::{parse_xml, XmlElement, XmlNode};
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandType, Device, DeviceId, Entity, EntityId, EntityKind, Health,
    IntegrationId, MediaCommandType, Metadata, ProtocolFamily, ProtocolIdentifier, SmartHomeTool,
    StateConfidence, StateSnapshot, StateSource, Value, ValueKind,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv6Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use udp_client::{send_to_and_collect, UdpDiscoveryEndpoint, UdpError, UdpOptions};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "roku";
pub const PROTOCOL_ID: &str = "roku_ecp";
pub const SSDP_SEARCH_TARGET: &str = "roku:ecp";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const PLAY_KEY_PATH: &str = "/keypress/Play";

#[derive(Debug)]
pub enum RokuError {
    Validation(String),
    Url(UrlError),
    Udp(UdpError),
    Io(String),
    Http(String),
    HttpStatus(u16),
    Xml(String),
    ResponseTooLarge {
        limit: usize,
    },
    TruncatedBody {
        expected: usize,
        actual: usize,
    },
    UnsupportedCommand(CommandType),
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    UnexpectedPlaybackState(String),
    PlaybackPostcondition {
        expected: RokuPlaybackState,
        actual: RokuPlaybackState,
    },
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
            Self::UnsupportedCommand(command) => {
                write!(formatter, "Roku does not support command {command:?}")
            }
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "Roku command {command_type:?} expects {expected}"
            ),
            Self::UnexpectedPlaybackState(state) => {
                write!(
                    formatter,
                    "Roku reported unsupported playback state `{state}`"
                )
            }
            Self::PlaybackPostcondition { expected, actual } => write!(
                formatter,
                "Roku playback postcondition failed: expected {}, got {}",
                expected.as_str(),
                actual.as_str()
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
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
        {
            return Err(RokuError::Validation(
                "base URL must be credential-free local HTTP".to_string(),
            ));
        }
        let host = parsed
            .host
            .as_deref()
            .ok_or_else(|| RokuError::Validation("base URL is missing a host".to_string()))?;
        let address = strip_ipv6_brackets(host).parse::<IpAddr>().map_err(|_| {
            RokuError::Validation("base URL host must be an IP literal".to_string())
        })?;
        if !is_local_ip(address) {
            return Err(RokuError::Validation(
                "base URL host must be private, link-local, or loopback".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
        })
    }
}

fn is_local_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            address.is_private() || address.is_link_local() || address.is_loopback()
        }
        IpAddr::V6(address) => {
            address.is_loopback()
                || address.is_unicast_link_local()
                || is_unique_local_ipv6(address)
        }
    }
}

fn is_unique_local_ipv6(address: Ipv6Addr) -> bool {
    address.segments()[0] & 0xfe00 == 0xfc00
}

fn strip_ipv6_brackets(host: &str) -> &str {
    host.strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host)
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
    pub playback_state: RokuPlaybackState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RokuPlaybackState {
    Play,
    Pause,
    Other(String),
}

impl RokuPlaybackState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Other(state) => state,
        }
    }
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
    fn post(&mut self, endpoint: &str) -> Result<Vec<u8>, RokuError>;
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
        self.request("GET", endpoint)
    }

    fn post(&mut self, endpoint: &str) -> Result<Vec<u8>, RokuError> {
        self.request("POST", endpoint)
    }
}

impl RokuLanTransport {
    fn request(&self, method: &str, endpoint: &str) -> Result<Vec<u8>, RokuError> {
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
        let mut stream = connect_tcp(strip_ipv6_brackets(host), port, self.timeout)?;
        stream
            .write_all(&encode_request(method, &url)?)
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
        let playback_state = parse_media_player(&self.get("/query/media-player")?)?;
        Ok(RokuSnapshot {
            device,
            apps,
            active_app,
            playback_state,
        })
    }

    pub fn playback_state(&mut self) -> Result<RokuPlaybackState, RokuError> {
        parse_media_player(&self.get("/query/media-player")?)
    }

    pub fn toggle_playback(&mut self) -> Result<(), RokuError> {
        self.transport
            .post(&format!("{}{PLAY_KEY_PATH}", self.config.base_url))?;
        Ok(())
    }

    fn get(&mut self, path: &str) -> Result<Vec<u8>, RokuError> {
        self.transport
            .get(&format!("{}{path}", self.config.base_url))
    }
}

pub struct RokuRuntimeIntegration<T> {
    client: RokuClient<T>,
    entity_id: Option<EntityId>,
}

impl<T: RokuTransport> RokuRuntimeIntegration<T> {
    pub fn new(client: RokuClient<T>) -> Self {
        Self {
            client,
            entity_id: None,
        }
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
        let entity_id = install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)?;
        self.entity_id = Some(entity_id.clone());
        Ok(entity_id)
    }

    pub fn dispatch_command_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, RokuError> {
        if self.entity_id.as_ref() != Some(&request.entity_id) {
            return Err(RokuError::InvalidCommandArguments {
                command_type: request.command_type,
                expected: "the installed Roku media entity",
            });
        }
        let desired = desired_playback_state(&request)?;
        let result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        execute_playback_command(&mut self.client, desired)?;
        Ok(result)
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

pub fn parse_media_player(bytes: &[u8]) -> Result<RokuPlaybackState, RokuError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| RokuError::Xml("media-player response is not UTF-8".to_string()))?;
    let document = parse_xml(source).map_err(|error| RokuError::Xml(error.to_string()))?;
    if document.root.local_name != "player" {
        return Err(RokuError::Xml("expected player root".to_string()));
    }
    let state = document
        .root
        .get_attr(None, "state")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| RokuError::Xml("player is missing state".to_string()))?;
    if state.len() > 64 || state.contains(['\r', '\n', '\0']) {
        return Err(RokuError::Xml(
            "player state is unsafe or over-limit".to_string(),
        ));
    }
    match state {
        "play" => Ok(RokuPlaybackState::Play),
        "pause" => Ok(RokuPlaybackState::Pause),
        _ => Ok(RokuPlaybackState::Other(state.to_string())),
    }
}

fn desired_playback_state(
    request: &RuntimeCommandToolRequest,
) -> Result<RokuPlaybackState, RokuError> {
    if request.command_type != CommandType::Media(MediaCommandType::SetPlaybackState) {
        return Err(RokuError::UnsupportedCommand(request.command_type));
    }
    let Value::Text(state) = &request.arguments else {
        return Err(RokuError::InvalidCommandArguments {
            command_type: request.command_type,
            expected: "play or pause text",
        });
    };
    match state.as_str() {
        "play" => Ok(RokuPlaybackState::Play),
        "pause" => Ok(RokuPlaybackState::Pause),
        _ => Err(RokuError::InvalidCommandArguments {
            command_type: request.command_type,
            expected: "play or pause text",
        }),
    }
}

fn execute_playback_command<T: RokuTransport>(
    client: &mut RokuClient<T>,
    desired: RokuPlaybackState,
) -> Result<(), RokuError> {
    let current = client.playback_state()?;
    if current == desired {
        return Ok(());
    }
    let opposite = match &desired {
        RokuPlaybackState::Play => RokuPlaybackState::Pause,
        RokuPlaybackState::Pause => RokuPlaybackState::Play,
        RokuPlaybackState::Other(state) => {
            return Err(RokuError::UnexpectedPlaybackState(state.clone()))
        }
    };
    if current != opposite {
        return Err(RokuError::UnexpectedPlaybackState(
            current.as_str().to_string(),
        ));
    }
    client.toggle_playback()?;
    let actual = client.playback_state()?;
    if actual != desired {
        return Err(RokuError::PlaybackPostcondition {
            expected: desired,
            actual,
        });
    }
    Ok(())
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
        capabilities: vec![
            Capability::new(
                CapabilityId::trusted("media.current_app"),
                CapabilityMode::Observe,
                ValueKind::Object,
            ),
            Capability::media_playback(),
        ],
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
        (
            "playback_state".to_string(),
            Value::Text(snapshot.playback_state.as_str().to_string()),
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

fn encode_request(method: &str, url: &Url) -> Result<Vec<u8>, RokuError> {
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
    let content_length = if method == "POST" {
        "Content-Length: 0\r\n"
    } else {
        ""
    };
    Ok(format!(
        "{method} {} HTTP/1.1\r\nHost: {host_header}\r\nAccept: application/xml\r\n{content_length}Connection: close\r\n\r\n",
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
    use std::collections::VecDeque;
    use std::net::{TcpListener, UdpSocket};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    const DEVICE_INFO: &str = r#"<device-info><user-device-name>Living Room Roku</user-device-name><model-name>Roku Ultra</model-name><serial-number>YJ00AB123456</serial-number><device-id>012345678901</device-id><software-version>14.1.4</software-version><power-mode>PowerOn</power-mode><is-tv>false</is-tv></device-info>"#;
    const APPS: &str =
        r#"<apps><app id="12" version="6.0">Netflix</app><app id="13">YouTube</app></apps>"#;
    const ACTIVE_APP: &str = r#"<active-app><app id="12" version="6.0">Netflix</app></active-app>"#;
    const MEDIA_PLAY: &str =
        r#"<player error="false" state="play"><position>1000 ms</position></player>"#;
    const MEDIA_PAUSE: &str =
        r#"<player error="false" state="pause"><position>1000 ms</position></player>"#;
    const MEDIA_BUFFER: &str = r#"<player error="false" state="buffer"/>"#;

    fn grant_read(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_capability(
                CapabilityGrantId::trusted(format!("grant:read:{}", principal.as_str())),
                principal.clone(),
                CapabilityId::trusted("smart_home.read"),
                PrivilegeTier::ReadOnly,
                "test",
                1,
            ));
    }

    fn grant_all(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ =
            runtime
                .registry_mut()
                .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                    CapabilityGrantId::trusted(format!("grant:all:{}", principal.as_str())),
                    principal.clone(),
                    PrivilegeTier::LowRisk,
                    "test",
                    1,
                ));
    }

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
        assert_eq!(
            parse_media_player(MEDIA_PLAY.as_bytes()).unwrap(),
            RokuPlaybackState::Play
        );
        assert_eq!(
            parse_media_player(br#"<player error="false" state="buffer"/>"#).unwrap(),
            RokuPlaybackState::Other("buffer".to_string())
        );
    }

    #[test]
    fn configured_endpoint_must_be_a_local_ip_literal() {
        assert!(RokuConfig::new(BridgeId::trusted("private"), "http://192.168.1.8:8060").is_ok());
        assert!(RokuConfig::new(BridgeId::trusted("ipv6"), "http://[::1]:8060").is_ok());
        assert!(RokuConfig::new(BridgeId::trusted("public"), "http://203.0.113.8:8060").is_err());
        assert!(RokuConfig::new(BridgeId::trusted("dns"), "http://roku.local:8060").is_err());
    }

    #[test]
    fn authorization_denies_before_transport_io() {
        struct CountingTransport(Arc<AtomicUsize>);
        impl RokuTransport for CountingTransport {
            fn get(&mut self, _endpoint: &str) -> Result<Vec<u8>, RokuError> {
                self.0.fetch_add(1, Ordering::SeqCst);
                Ok(Vec::new())
            }

            fn post(&mut self, _endpoint: &str) -> Result<Vec<u8>, RokuError> {
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
            for body in [DEVICE_INFO, APPS, ACTIVE_APP, MEDIA_PLAY] {
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
        grant_read(&mut runtime, &principal);
        let entity_id = integration
            .inspect_and_install_authorized(&mut runtime, principal, 10)
            .unwrap();
        assert!(runtime.registry().entity(&entity_id).is_some());
        assert_eq!(runtime.registry().counts().devices, 1);
        udp_handle.join().unwrap();
        http_handle.join().unwrap();
    }

    struct ScriptedTransport {
        responses: VecDeque<Vec<u8>>,
        calls: Arc<AtomicUsize>,
        posts: Arc<AtomicUsize>,
    }

    impl RokuTransport for ScriptedTransport {
        fn get(&mut self, _endpoint: &str) -> Result<Vec<u8>, RokuError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.responses
                .pop_front()
                .ok_or_else(|| RokuError::Io("unexpected scripted GET".to_string()))
        }

        fn post(&mut self, endpoint: &str) -> Result<Vec<u8>, RokuError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.posts.fetch_add(1, Ordering::SeqCst);
            assert!(endpoint.ends_with(PLAY_KEY_PATH));
            Ok(Vec::new())
        }
    }

    fn scripted_integration(
        responses: &[&str],
        calls: Arc<AtomicUsize>,
        posts: Arc<AtomicUsize>,
    ) -> RokuRuntimeIntegration<ScriptedTransport> {
        let config =
            RokuConfig::new(BridgeId::trusted("roku:scripted"), "http://127.0.0.1:8060").unwrap();
        RokuRuntimeIntegration::new(RokuClient::new(
            config,
            ScriptedTransport {
                responses: responses
                    .iter()
                    .map(|response| response.as_bytes().to_vec())
                    .collect(),
                calls,
                posts,
            },
        ))
    }

    #[test]
    fn authorized_playback_toggle_is_verified_and_denial_precedes_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let posts = Arc::new(AtomicUsize::new(0));
        let mut integration = scripted_integration(
            &[
                DEVICE_INFO,
                APPS,
                ACTIVE_APP,
                MEDIA_PLAY,
                MEDIA_PLAY,
                MEDIA_PAUSE,
            ],
            Arc::clone(&calls),
            Arc::clone(&posts),
        );
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:roku-command");
        grant_all(&mut runtime, &principal);
        let entity_id = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 10)
            .unwrap();
        let result = integration
            .dispatch_command_authorized(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    entity_id.clone(),
                    CommandType::Media(MediaCommandType::SetPlaybackState),
                    Value::Text("pause".to_string()),
                ),
                20,
            )
            .unwrap();
        assert!(result.is_accepted());
        assert_eq!(calls.load(Ordering::SeqCst), 7);
        assert_eq!(posts.load(Ordering::SeqCst), 1);

        let before_denial = calls.load(Ordering::SeqCst);
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                AgentId::trusted("agent:roku-denied"),
                RuntimeCommandToolRequest::new(
                    entity_id,
                    CommandType::Media(MediaCommandType::SetPlaybackState),
                    Value::Text("play".to_string()),
                ),
                30,
            ),
            Err(RokuError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), before_denial);
    }

    #[test]
    fn loopback_http_command_uses_fixed_key_and_verifies_playback() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let server = thread::spawn(move || {
            let bodies = [
                Some(DEVICE_INFO),
                Some(APPS),
                Some(ACTIVE_APP),
                Some(MEDIA_PLAY),
                Some(MEDIA_PLAY),
                None,
                Some(MEDIA_PAUSE),
            ];
            for body in bodies {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0u8; 2048];
                let read = stream.read(&mut request).unwrap();
                let first_line = String::from_utf8_lossy(&request[..read])
                    .lines()
                    .next()
                    .unwrap()
                    .to_string();
                captured.lock().unwrap().push(first_line);
                let body = body.unwrap_or_default();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });

        let config = RokuConfig::new(
            BridgeId::trusted("roku:loopback-command"),
            format!("http://{address}"),
        )
        .unwrap();
        let mut integration =
            RokuRuntimeIntegration::new(RokuClient::new(config, RokuLanTransport::default()));
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:roku-loopback-command");
        grant_all(&mut runtime, &principal);
        let entity_id = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 10)
            .unwrap();
        let result = integration
            .dispatch_command_authorized(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    entity_id,
                    CommandType::Media(MediaCommandType::SetPlaybackState),
                    Value::Text("pause".to_string()),
                ),
                20,
            )
            .unwrap();
        assert!(result.is_accepted());
        server.join().unwrap();
        assert_eq!(
            requests.lock().unwrap().as_slice(),
            [
                "GET /query/device-info HTTP/1.1",
                "GET /query/apps HTTP/1.1",
                "GET /query/active-app HTTP/1.1",
                "GET /query/media-player HTTP/1.1",
                "GET /query/media-player HTTP/1.1",
                "POST /keypress/Play HTTP/1.1",
                "GET /query/media-player HTTP/1.1",
            ]
        );
    }

    #[test]
    fn matching_playback_is_idempotent_and_failed_postcondition_is_reported() {
        let calls = Arc::new(AtomicUsize::new(0));
        let posts = Arc::new(AtomicUsize::new(0));
        let mut client = RokuClient::new(
            RokuConfig::new(BridgeId::trusted("roku:test"), "http://127.0.0.1:8060").unwrap(),
            ScriptedTransport {
                responses: [MEDIA_PAUSE, MEDIA_PLAY, MEDIA_PLAY, MEDIA_BUFFER]
                    .into_iter()
                    .map(|response| response.as_bytes().to_vec())
                    .collect(),
                calls,
                posts: Arc::clone(&posts),
            },
        );
        execute_playback_command(&mut client, RokuPlaybackState::Pause).unwrap();
        assert_eq!(posts.load(Ordering::SeqCst), 0);
        assert!(matches!(
            execute_playback_command(&mut client, RokuPlaybackState::Pause),
            Err(RokuError::PlaybackPostcondition {
                expected: RokuPlaybackState::Pause,
                actual: RokuPlaybackState::Play,
            })
        ));
        assert_eq!(posts.load(Ordering::SeqCst), 1);
        assert!(matches!(
            execute_playback_command(&mut client, RokuPlaybackState::Pause),
            Err(RokuError::UnexpectedPlaybackState(state)) if state == "buffer"
        ));
        assert_eq!(posts.load(Ordering::SeqCst), 1);
    }
}
