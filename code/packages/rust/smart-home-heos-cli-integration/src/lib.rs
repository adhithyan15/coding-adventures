//! HEOS CLI discovery and read-only player inspection for D23.

#![forbid(unsafe_code)]

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
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;
use udp_client::{send_to_and_collect, UdpDiscoveryEndpoint, UdpError, UdpOptions};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "heos";
pub const PROTOCOL_ID: &str = "heos_cli";
pub const SSDP_SEARCH_TARGET: &str = "urn:schemas-denon-com:device:ACT-Denon:1";
pub const DEFAULT_PORT: u16 = 1255;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const DEFAULT_MAX_PLAYERS: usize = 64;

#[derive(Debug)]
pub enum HeosError {
    Validation(String),
    Url(UrlError),
    Udp(UdpError),
    Io(String),
    Json(serde_json::Error),
    ResponseTooLarge { limit: usize },
    MissingField(&'static str),
    CommandFailed { command: String, message: String },
    NoPlayers,
    Runtime(RuntimeError),
}

impl fmt::Display for HeosError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid HEOS input: {message}"),
            Self::Url(error) => write!(formatter, "invalid HEOS URL: {error}"),
            Self::Udp(error) => write!(formatter, "HEOS SSDP failed: {error}"),
            Self::Io(message) => write!(formatter, "HEOS LAN I/O failed: {message}"),
            Self::Json(error) => write!(formatter, "invalid HEOS JSON: {error}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "HEOS response exceeds {limit} bytes")
            }
            Self::MissingField(field) => write!(formatter, "HEOS response is missing {field}"),
            Self::CommandFailed { command, message } => {
                write!(formatter, "HEOS command {command} failed: {message}")
            }
            Self::NoPlayers => formatter.write_str("HEOS system returned no players"),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for HeosError {}

impl From<UrlError> for HeosError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<UdpError> for HeosError {
    fn from(error: UdpError) -> Self {
        Self::Udp(error)
    }
}

impl From<serde_json::Error> for HeosError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for HeosError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeosConfig {
    pub bridge_id: BridgeId,
    pub host: String,
    pub port: u16,
    pub timeout: Duration,
}

impl HeosConfig {
    pub fn new(bridge_id: BridgeId, host: impl Into<String>) -> Result<Self, HeosError> {
        let host = host.into();
        if host.trim().is_empty()
            || host.contains(['/', '\\', '\r', '\n', '\0'])
            || host.contains('@')
        {
            return Err(HeosError::Validation(
                "host must be a bare DNS name or IP address".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            host,
            port: DEFAULT_PORT,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port.max(1);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn endpoint(&self) -> String {
        if self.host.contains(':') && !self.host.starts_with('[') {
            format!("tcp://[{}]:{}", self.host, self.port)
        } else {
            format!("tcp://{}:{}", self.host, self.port)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeosSsdpCandidate {
    pub location: String,
    pub usn: String,
    pub server: Option<String>,
    pub host: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeosPlayer {
    pub pid: String,
    pub name: String,
    pub model: String,
    pub version: String,
    pub network: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HeosNowPlaying {
    pub media_type: Option<String>,
    pub song: Option<String>,
    pub station: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeosPlayerSnapshot {
    pub player: HeosPlayer,
    pub play_state: String,
    pub volume: u8,
    pub muted: bool,
    pub now_playing: HeosNowPlaying,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeosSnapshot {
    pub players: Vec<HeosPlayerSnapshot>,
}

pub fn ssdp_search_request() -> Vec<u8> {
    format!(
        "M-SEARCH * HTTP/1.1\r\nST: {SSDP_SEARCH_TARGET}\r\nMX: 2\r\nMAN: \"ssdp:discover\"\r\nHOST: 239.255.255.250:1900\r\n\r\n"
    )
    .into_bytes()
}

pub fn discover_ssdp_ipv4(
    timeout: Duration,
    max_responses: usize,
) -> Result<Vec<HeosSsdpCandidate>, HeosError> {
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
) -> Result<Vec<HeosSsdpCandidate>, HeosError> {
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

pub fn parse_ssdp_response(bytes: &[u8]) -> Result<HeosSsdpCandidate, HeosError> {
    let source = std::str::from_utf8(bytes)
        .map_err(|_| HeosError::Validation("SSDP response is not UTF-8".to_string()))?;
    let mut lines = source.split("\r\n");
    let status = lines.next().unwrap_or_default();
    if !status.starts_with("HTTP/1.1 200") && !status.starts_with("HTTP/1.0 200") {
        return Err(HeosError::Validation(
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
    let search_target = required_header(&headers, "st")?;
    if !search_target.eq_ignore_ascii_case(SSDP_SEARCH_TARGET) {
        return Err(HeosError::Validation(format!(
            "unexpected SSDP search target `{search_target}`"
        )));
    }
    let location = required_header(&headers, "location")?;
    let parsed = Url::parse(location)?;
    if !matches!(parsed.scheme.as_str(), "http" | "https") || parsed.userinfo.is_some() {
        return Err(HeosError::Validation(
            "SSDP location must be a credential-free HTTP URL".to_string(),
        ));
    }
    let host = parsed
        .host
        .ok_or(HeosError::MissingField("SSDP location host"))?;
    Ok(HeosSsdpCandidate {
        location: location.to_string(),
        usn: required_header(&headers, "usn")?.to_string(),
        server: headers.get("server").cloned(),
        host,
    })
}

fn required_header<'a>(
    headers: &'a BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, HeosError> {
    headers
        .get(name)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| HeosError::Validation(format!("SSDP response is missing {name}")))
}

pub fn discovery_record(
    candidate: &HeosSsdpCandidate,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, HeosError> {
    let native_id = stable_component(candidate.usn.split("::").next().unwrap_or(&candidate.usn));
    if native_id.is_empty() {
        return Err(HeosError::Validation(
            "SSDP USN contains no stable identifier".to_string(),
        ));
    }
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        native_id,
        DiscoverySource::Ssdp,
        BridgeTransport::LanTcp,
        discovered_at_ms,
    )
    .map_err(|error| HeosError::Validation(error.to_string()))?
    .with_display_name("HEOS system")
    .with_address(format!("tcp://{}:{DEFAULT_PORT}", candidate.host))
    .with_confidence(DiscoveryConfidence::Verified)
    .with_pairing_requirement(PairingRequirement::None)
    .with_metadata("smart_home.discovery.search_target", SSDP_SEARCH_TARGET))
}

pub trait HeosTransport {
    fn exchange(
        &mut self,
        host: &str,
        port: u16,
        commands: &[String],
        timeout: Duration,
    ) -> Result<Vec<JsonValue>, HeosError>;
}

#[derive(Debug, Clone)]
pub struct HeosLanTransport {
    pub maximum_response_bytes: usize,
}

impl Default for HeosLanTransport {
    fn default() -> Self {
        Self {
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }
}

impl HeosLanTransport {
    pub fn with_maximum_response_bytes(mut self, maximum: usize) -> Self {
        self.maximum_response_bytes = maximum.max(1);
        self
    }
}

impl HeosTransport for HeosLanTransport {
    fn exchange(
        &mut self,
        host: &str,
        port: u16,
        commands: &[String],
        timeout: Duration,
    ) -> Result<Vec<JsonValue>, HeosError> {
        if commands.is_empty() {
            return Ok(Vec::new());
        }
        if commands
            .iter()
            .any(|command| !command.starts_with("heos://") || command.contains(['\r', '\n', '\0']))
        {
            return Err(HeosError::Validation(
                "command is not a safe HEOS CLI request".to_string(),
            ));
        }
        let mut stream = connect_tcp(host, port, timeout)?;
        for command in commands {
            stream
                .write_all(command.as_bytes())
                .and_then(|_| stream.write_all(b"\r\n"))
                .map_err(|error| HeosError::Io(error.to_string()))?;
        }
        stream
            .flush()
            .map_err(|error| HeosError::Io(error.to_string()))?;
        let mut reader = BufReader::new(stream);
        let mut remaining = self.maximum_response_bytes;
        let mut responses = Vec::with_capacity(commands.len());
        for _ in commands {
            let line = read_bounded_line(&mut reader, remaining)?;
            remaining = remaining.saturating_sub(line.len());
            responses.push(serde_json::from_slice(&line)?);
        }
        Ok(responses)
    }
}

pub struct HeosClient<T> {
    config: HeosConfig,
    transport: T,
}

impl<T: HeosTransport> HeosClient<T> {
    pub fn new(config: HeosConfig, transport: T) -> Self {
        Self { config, transport }
    }

    pub fn config(&self) -> &HeosConfig {
        &self.config
    }

    pub fn inspect(&mut self) -> Result<HeosSnapshot, HeosError> {
        let inventory_command = "heos://player/get_players".to_string();
        let inventory = self.transport.exchange(
            &self.config.host,
            self.config.port,
            std::slice::from_ref(&inventory_command),
            self.config.timeout,
        )?;
        let inventory = inventory
            .first()
            .ok_or(HeosError::MissingField("player inventory response"))?;
        successful_response(inventory, "player/get_players")?;
        let players = parse_players(inventory)?;
        if players.is_empty() {
            return Err(HeosError::NoPlayers);
        }
        if players.len() > DEFAULT_MAX_PLAYERS {
            return Err(HeosError::Validation(format!(
                "player count exceeds {DEFAULT_MAX_PLAYERS}"
            )));
        }

        let mut commands = Vec::with_capacity(players.len() * 4);
        for player in &players {
            let pid = encode_argument(&player.pid);
            commands.extend([
                format!("heos://player/get_play_state?pid={pid}"),
                format!("heos://player/get_now_playing_media?pid={pid}"),
                format!("heos://player/get_volume?pid={pid}"),
                format!("heos://player/get_mute?pid={pid}"),
            ]);
        }
        let details = self.transport.exchange(
            &self.config.host,
            self.config.port,
            &commands,
            self.config.timeout,
        )?;
        if details.len() != commands.len() {
            return Err(HeosError::Validation(
                "transport returned the wrong response count".to_string(),
            ));
        }

        let mut snapshots = Vec::with_capacity(players.len());
        for (player, responses) in players.into_iter().zip(details.chunks_exact(4)) {
            successful_response(&responses[0], "player/get_play_state")?;
            successful_response(&responses[1], "player/get_now_playing_media")?;
            successful_response(&responses[2], "player/get_volume")?;
            successful_response(&responses[3], "player/get_mute")?;
            snapshots.push(HeosPlayerSnapshot {
                player,
                play_state: message_attribute(&responses[0], "state")?,
                now_playing: parse_now_playing(&responses[1])?,
                volume: parse_volume(&responses[2])?,
                muted: parse_mute(&responses[3])?,
            });
        }
        Ok(HeosSnapshot { players: snapshots })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledHeosSystem {
    pub bridge_id: BridgeId,
    pub device_ids: Vec<DeviceId>,
    pub entity_ids: Vec<EntityId>,
}

pub struct HeosRuntimeIntegration<T> {
    client: HeosClient<T>,
}

impl<T: HeosTransport> HeosRuntimeIntegration<T> {
    pub fn new(client: HeosClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledHeosSystem, HeosError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), HeosError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(HeosError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &HeosConfig,
    snapshot: &HeosSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledHeosSystem, HeosError> {
    if snapshot.players.is_empty() {
        return Err(HeosError::NoPlayers);
    }
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanTcp,
    );
    bridge.address = Some(config.endpoint());
    bridge.hardware_model = Some("HEOS CLI host".to_string());
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("cli_endpoint", &config.endpoint())?];
    bridge.metadata = vec![Metadata::new("heos.transport", "tcp_json")];
    runtime.upsert_bridge(bridge)?;

    let mut device_ids = Vec::with_capacity(snapshot.players.len());
    let mut entity_ids = Vec::with_capacity(snapshot.players.len());
    for snapshot in &snapshot.players {
        let native_id = stable_component(&snapshot.player.pid);
        if native_id.is_empty() {
            return Err(HeosError::Validation(
                "player id contains no stable identifier".to_string(),
            ));
        }
        let device_id = DeviceId::trusted(format!("heos:{native_id}"));
        let entity_id = EntityId::trusted(format!("heos:{native_id}:player"));
        let mut metadata = Vec::new();
        if let Some(network) = &snapshot.player.network {
            metadata.push(Metadata::new("heos.network", network));
        }
        if let Some(group_id) = &snapshot.player.group_id {
            metadata.push(Metadata::new("heos.group_id", group_id));
        }
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "Denon/Marantz HEOS".to_string(),
            model: snapshot.player.model.clone(),
            name: snapshot.player.name.clone(),
            serial: Some(snapshot.player.pid.clone()),
            firmware_version: Some(snapshot.player.version.clone()),
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: vec![protocol_identifier("player_id", &snapshot.player.pid)?],
            health: Health::Online,
            metadata,
        })?;
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Unknown,
            name: snapshot.player.name.clone(),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("media.player_state"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: player_value(snapshot),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("heos.control_surface", "read_only_player")],
        })?;
        device_ids.push(device_id);
        entity_ids.push(entity_id);
    }
    Ok(InstalledHeosSystem {
        bridge_id: config.bridge_id.clone(),
        device_ids,
        entity_ids,
    })
}

fn player_value(snapshot: &HeosPlayerSnapshot) -> Value {
    Value::Object(vec![
        (
            "play_state".to_string(),
            Value::Text(snapshot.play_state.clone()),
        ),
        ("volume".to_string(), Value::Percentage(snapshot.volume)),
        ("muted".to_string(), Value::Bool(snapshot.muted)),
        (
            "media_type".to_string(),
            optional_text(&snapshot.now_playing.media_type),
        ),
        (
            "song".to_string(),
            optional_text(&snapshot.now_playing.song),
        ),
        (
            "station".to_string(),
            optional_text(&snapshot.now_playing.station),
        ),
        (
            "album".to_string(),
            optional_text(&snapshot.now_playing.album),
        ),
        (
            "artist".to_string(),
            optional_text(&snapshot.now_playing.artist),
        ),
        (
            "image_url".to_string(),
            optional_text(&snapshot.now_playing.image_url),
        ),
    ])
}

fn optional_text(value: &Option<String>) -> Value {
    value.clone().map(Value::Text).unwrap_or(Value::Null)
}

fn successful_response<'a>(
    value: &'a JsonValue,
    expected_command: &str,
) -> Result<&'a JsonMap<String, JsonValue>, HeosError> {
    let root = value
        .as_object()
        .ok_or(HeosError::MissingField("response object"))?;
    let heos = root
        .get("heos")
        .and_then(JsonValue::as_object)
        .ok_or(HeosError::MissingField("heos"))?;
    let command = required_string(heos, "command")?;
    if command.trim() != expected_command {
        return Err(HeosError::Validation(format!(
            "expected response for {expected_command}, got {command}"
        )));
    }
    if required_string(heos, "result")? != "success" {
        return Err(HeosError::CommandFailed {
            command,
            message: heos
                .get("message")
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown error")
                .to_string(),
        });
    }
    Ok(root)
}

fn parse_players(value: &JsonValue) -> Result<Vec<HeosPlayer>, HeosError> {
    let payload = value
        .get("payload")
        .and_then(JsonValue::as_array)
        .ok_or(HeosError::MissingField("player payload"))?;
    payload
        .iter()
        .map(|value| {
            let player = value
                .as_object()
                .ok_or(HeosError::MissingField("player object"))?;
            Ok(HeosPlayer {
                pid: required_string(player, "pid")?,
                name: required_string(player, "name")?,
                model: required_string(player, "model")?,
                version: required_string(player, "version")?,
                network: optional_string(player, "network"),
                group_id: optional_string(player, "gid"),
            })
        })
        .collect()
}

fn parse_now_playing(value: &JsonValue) -> Result<HeosNowPlaying, HeosError> {
    let payload = value
        .get("payload")
        .and_then(JsonValue::as_object)
        .ok_or(HeosError::MissingField("now-playing payload"))?;
    Ok(HeosNowPlaying {
        media_type: optional_string(payload, "type"),
        song: optional_string(payload, "song"),
        station: optional_string(payload, "station"),
        album: optional_string(payload, "album"),
        artist: optional_string(payload, "artist"),
        image_url: optional_string(payload, "image_url"),
    })
}

fn parse_volume(value: &JsonValue) -> Result<u8, HeosError> {
    let level = message_attribute(value, "level")?;
    level
        .parse::<u8>()
        .ok()
        .filter(|level| *level <= 100)
        .ok_or_else(|| HeosError::Validation(format!("invalid volume `{level}`")))
}

fn parse_mute(value: &JsonValue) -> Result<bool, HeosError> {
    match message_attribute(value, "on")?.as_str() {
        "on" | "1" | "true" => Ok(true),
        "off" | "0" | "false" => Ok(false),
        value => Err(HeosError::Validation(format!(
            "invalid mute state `{value}`"
        ))),
    }
}

fn message_attribute(value: &JsonValue, name: &'static str) -> Result<String, HeosError> {
    let message = value
        .get("heos")
        .and_then(JsonValue::as_object)
        .and_then(|heos| heos.get("message"))
        .and_then(JsonValue::as_str)
        .ok_or(HeosError::MissingField("heos.message"))?;
    message
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find_map(|(key, value)| (key == name).then(|| decode_argument(value)))
        .ok_or(HeosError::MissingField(name))
}

fn required_string(
    value: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, HeosError> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(HeosError::MissingField(field))
}

fn optional_string(value: &JsonMap<String, JsonValue>, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(decode_argument)
}

fn encode_argument(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('&', "%26")
        .replace('=', "%3D")
}

fn decode_argument(value: &str) -> String {
    value
        .replace("%26", "&")
        .replace("%3D", "=")
        .replace("%25", "%")
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, HeosError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| HeosError::Validation(error.to_string()))
}

fn stable_component(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, HeosError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| HeosError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| HeosError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| HeosError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(HeosError::Io(
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

fn read_bounded_line(reader: &mut dyn BufRead, maximum: usize) -> Result<Vec<u8>, HeosError> {
    let mut output = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| HeosError::Io(error.to_string()))?;
        if available.is_empty() {
            return Err(HeosError::Io(
                "connection closed before a complete response".to_string(),
            ));
        }
        let consumed = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |position| position + 1);
        if consumed > maximum.saturating_sub(output.len()) {
            return Err(HeosError::ResponseTooLarge { limit: maximum });
        }
        let complete = available[consumed - 1] == b'\n';
        output.extend_from_slice(&available[..consumed]);
        reader.consume(consumed);
        if complete {
            while output
                .last()
                .is_some_and(|byte| matches!(byte, b'\r' | b'\n'))
            {
                output.pop();
            }
            if output.is_empty() {
                return Err(HeosError::Validation(
                    "HEOS response line is empty".to_string(),
                ));
            }
            return Ok(output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::io::Cursor;
    use std::net::{TcpListener, UdpSocket};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    const PLAYERS: &str = r#"{"heos":{"command":"player/get_players","result":"success","message":""},"payload":[{"name":"Living Room","pid":"1","model":"HEOS 5","version":"3.34.620","network":"wired","lineout":"1"}]}"#;
    const STATE: &str = r#"{"heos":{"command":"player/get_play_state","result":"success","message":"pid=1&state=play"}}"#;
    const MEDIA: &str = r#"{"heos":{"command":"player/get_now_playing_media","result":"success","message":"pid=1"},"payload":{"type":"song","song":"Night Drive","station":"","album":"Road Tests","artist":"The Tests","image_url":"https://example.invalid/cover.jpg"}}"#;
    const VOLUME: &str =
        r#"{"heos":{"command":"player/get_volume","result":"success","message":"pid=1&level=27"}}"#;
    const MUTE: &str =
        r#"{"heos":{"command":"player/get_mute","result":"success","message":"pid=1&on=off"}}"#;

    fn write_responses(mut stream: TcpStream, responses: &[&str]) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        for response in responses {
            let mut command = String::new();
            reader.read_line(&mut command).unwrap();
            assert!(command.starts_with("heos://"));
            writeln!(stream, "{response}\r").unwrap();
        }
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:heos-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1,
            )
            .with_expiry(100),
        );
    }

    #[test]
    fn parses_verified_heos_ssdp_candidate() {
        let candidate = parse_ssdp_response(
            format!(
                "HTTP/1.1 200 OK\r\nST: {SSDP_SEARCH_TARGET}\r\nUSN: uuid:1234::{SSDP_SEARCH_TARGET}\r\nLOCATION: http://192.0.2.30:60006/upnp/desc/aios_device/aios_device.xml\r\nSERVER: Linux UPnP/1.0 Denon/3.0\r\n\r\n"
            )
            .as_bytes(),
        )
        .unwrap();
        assert_eq!(candidate.host, "192.0.2.30");
        let record = discovery_record(&candidate, 10).unwrap();
        assert_eq!(record.confidence, DiscoveryConfidence::Verified);
        assert_eq!(record.transport, BridgeTransport::LanTcp);
        assert_eq!(record.address.as_deref(), Some("tcp://192.0.2.30:1255"));

        let wrong = "HTTP/1.1 200 OK\r\nST: upnp:rootdevice\r\nUSN: uuid:1234\r\nLOCATION: http://192.0.2.30/device.xml\r\n\r\n".to_string();
        assert!(parse_ssdp_response(wrong.as_bytes()).is_err());
    }

    #[test]
    fn parses_player_state_and_heos_escaping() {
        let players: JsonValue = serde_json::from_str(PLAYERS).unwrap();
        successful_response(&players, "player/get_players").unwrap();
        assert_eq!(parse_players(&players).unwrap()[0].model, "HEOS 5");
        let volume: JsonValue = serde_json::from_str(VOLUME).unwrap();
        assert_eq!(parse_volume(&volume).unwrap(), 27);
        let mute: JsonValue = serde_json::from_str(MUTE).unwrap();
        assert!(!parse_mute(&mute).unwrap());
        assert_eq!(decode_argument(&encode_argument("a&b=c%")), "a&b=c%");
    }

    #[test]
    fn response_lines_are_bounded() {
        let mut input = Cursor::new(b"{\"ok\":true}\r\n".to_vec());
        assert_eq!(read_bounded_line(&mut input, 32).unwrap(), b"{\"ok\":true}");
        let mut oversized = Cursor::new(b"12345\r\n".to_vec());
        assert!(matches!(
            read_bounded_line(&mut oversized, 4),
            Err(HeosError::ResponseTooLarge { limit: 4 })
        ));
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl HeosTransport for CountingTransport {
        fn exchange(
            &mut self,
            _host: &str,
            _port: u16,
            _commands: &[String],
            _timeout: Duration,
        ) -> Result<Vec<JsonValue>, HeosError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = HeosClient::new(
            HeosConfig::new(BridgeId::trusted("heos:denied"), "127.0.0.1").unwrap(),
            CountingTransport(Arc::clone(&calls)),
        );
        let mut integration = HeosRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                10,
            ),
            Err(HeosError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn loopback_ssdp_tcp_inspection_installs_player_state() {
        let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
        let tcp_address = tcp.local_addr().unwrap();
        let tcp_handle = thread::spawn(move || {
            let (stream, _) = tcp.accept().unwrap();
            write_responses(stream, &[PLAYERS]);
            let (stream, _) = tcp.accept().unwrap();
            write_responses(stream, &[STATE, MEDIA, VOLUME, MUTE]);
        });

        let udp = UdpSocket::bind("127.0.0.1:0").unwrap();
        let udp_address = udp.local_addr().unwrap();
        let udp_handle = thread::spawn(move || {
            let mut request = [0u8; 2048];
            let (_, source) = udp.recv_from(&mut request).unwrap();
            assert!(String::from_utf8_lossy(&request).contains(SSDP_SEARCH_TARGET));
            let response = format!(
                "HTTP/1.1 200 OK\r\nST: {SSDP_SEARCH_TARGET}\r\nUSN: uuid:1234::{SSDP_SEARCH_TARGET}\r\nLOCATION: http://127.0.0.1:60006/upnp/desc/aios_device/aios_device.xml\r\nSERVER: Linux UPnP/1.0 Denon/3.0\r\n\r\n"
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

        let config = HeosConfig::new(BridgeId::trusted("heos:test"), &candidates[0].host)
            .unwrap()
            .with_port(tcp_address.port());
        let client = HeosClient::new(config, HeosLanTransport::default());
        let mut integration = HeosRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:heos-test");
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 10)
            .unwrap();
        assert_eq!(installed.device_ids.len(), 1);
        let device = runtime.registry().device(&installed.device_ids[0]).unwrap();
        assert_eq!(device.model, "HEOS 5");
        let entity = runtime.registry().entity(&installed.entity_ids[0]).unwrap();
        assert_eq!(entity.kind, EntityKind::Unknown);
        assert_eq!(
            entity.capabilities[0].capability_id.as_str(),
            "media.player_state"
        );
        assert_eq!(
            entity.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        udp_handle.join().unwrap();
        tcp_handle.join().unwrap();
    }

    #[test]
    fn failed_commands_are_rejected() {
        let failed: JsonValue = serde_json::from_str(
            r#"{"heos":{"command":"player/get_players","result":"fail","message":"eid=4&text=Command failed"}}"#,
        )
        .unwrap();
        assert!(matches!(
            successful_response(&failed, "player/get_players"),
            Err(HeosError::CommandFailed { .. })
        ));
    }
}
