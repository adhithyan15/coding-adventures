//! Bounded local Kodi HTTP JSON-RPC telemetry and media control for D23.

#![forbid(unsafe_code)]

use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde_json::{json, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandType, Device, DeviceId, Entity, EntityId, EntityKind, Health,
    IntegrationId, MediaCommandType, Metadata, ProtocolFamily, ProtocolIdentifier, SmartHomeTool,
    StateConfidence, StateSnapshot, StateSource, Value, ValueKind,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::BTreeSet;
use std::fmt;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv6Addr, SocketAddr, TcpStream};
use std::time::Duration;

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "kodi";
pub const PROTOCOL_ID: &str = "kodi_jsonrpc";
pub const JSON_RPC_PATH: &str = "/jsonrpc";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 1024 * 1024;
pub const MAX_REQUEST_BYTES: usize = 32 * 1024;
pub const MAX_TEXT_BYTES: usize = 256;
pub const MAX_ACTIVE_PLAYERS: usize = 3;

#[derive(Debug, PartialEq)]
pub enum KodiError {
    Validation(String),
    Io(String),
    Http(String),
    HttpStatus(u16),
    AuthenticationRequired,
    ResponseTooLarge {
        limit: usize,
    },
    TruncatedBody {
        expected: usize,
        actual: usize,
    },
    Json(String),
    Rpc {
        code: i64,
    },
    MissingField(&'static str),
    UnexpectedResult(&'static str),
    UnsupportedCommand(CommandType),
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    NoInspectionSnapshot,
    NoActivePlayer,
    AmbiguousActivePlayers,
    Runtime(RuntimeError),
}

impl fmt::Display for KodiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Kodi input: {message}"),
            Self::Io(message) => write!(formatter, "Kodi LAN I/O failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Kodi HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "Kodi endpoint returned HTTP {status}"),
            Self::AuthenticationRequired => {
                formatter.write_str("Kodi endpoint requires unsupported authentication")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Kodi response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Kodi response body is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(message) => write!(formatter, "invalid Kodi JSON-RPC response: {message}"),
            Self::Rpc { code } => write!(formatter, "Kodi JSON-RPC returned error code {code}"),
            Self::MissingField(field) => write!(formatter, "Kodi response is missing {field}"),
            Self::UnexpectedResult(field) => {
                write!(formatter, "Kodi returned an unexpected {field} result")
            }
            Self::UnsupportedCommand(command) => {
                write!(formatter, "Kodi does not support command {command:?}")
            }
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "Kodi command {command_type:?} expects {expected}"
            ),
            Self::NoInspectionSnapshot => {
                formatter.write_str("Kodi must be inspected before command routing")
            }
            Self::NoActivePlayer => formatter.write_str("Kodi has no active player"),
            Self::AmbiguousActivePlayers => {
                formatter.write_str("Kodi has more than one active player")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for KodiError {}

impl From<RuntimeError> for KodiError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KodiConfig {
    pub bridge_id: BridgeId,
    pub endpoint: SocketAddr,
    pub timeout: Duration,
    pub maximum_response_bytes: usize,
}

impl KodiConfig {
    pub fn new(bridge_id: BridgeId, endpoint: SocketAddr) -> Result<Self, KodiError> {
        if endpoint.port() == 0 {
            return Err(KodiError::Validation(
                "endpoint port must be non-zero".to_string(),
            ));
        }
        if !is_local_ip(endpoint.ip()) {
            return Err(KodiError::Validation(
                "endpoint must be a private, link-local, or loopback IP literal".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            endpoint,
            timeout: Duration::from_secs(5),
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn with_maximum_response_bytes(mut self, maximum: usize) -> Self {
        self.maximum_response_bytes = maximum.max(1);
        self
    }

    pub fn endpoint_url(&self) -> String {
        format!("http://{}{}", self.endpoint, JSON_RPC_PATH)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KodiVersion {
    pub major: u32,
    pub minor: u32,
    pub revision: Option<String>,
    pub tag: String,
    pub tag_version: Option<String>,
}

impl fmt::Display for KodiVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}.{}", self.major, self.minor)?;
        if self.tag != "stable" {
            write!(formatter, "-{}", self.tag)?;
        }
        if let Some(tag_version) = &self.tag_version {
            write!(formatter, "-{tag_version}")?;
        }
        if let Some(revision) = &self.revision {
            write!(formatter, "+{revision}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KodiApplicationSnapshot {
    pub name: String,
    pub version: KodiVersion,
    pub volume: u8,
    pub muted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KodiPlayerSnapshot {
    pub player_id: u8,
    pub player_type: String,
    pub player_runtime: String,
    pub speed: i64,
    pub position_percent: f64,
    pub elapsed_ms: u64,
    pub total_ms: u64,
    pub repeat: String,
    pub shuffled: bool,
    pub can_seek: bool,
    pub live: bool,
}

impl KodiPlayerSnapshot {
    pub fn playback_state(&self) -> &'static str {
        if self.speed == 0 {
            "pause"
        } else {
            "play"
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KodiSnapshot {
    pub application: KodiApplicationSnapshot,
    pub players: Vec<KodiPlayerSnapshot>,
}

pub trait KodiTransport {
    fn post(&mut self, config: &KodiConfig, body: &[u8]) -> Result<Vec<u8>, KodiError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct KodiLanTransport;

impl KodiTransport for KodiLanTransport {
    fn post(&mut self, config: &KodiConfig, body: &[u8]) -> Result<Vec<u8>, KodiError> {
        if body.len() > MAX_REQUEST_BYTES {
            return Err(KodiError::Validation(
                "JSON-RPC request exceeds the fixed request limit".to_string(),
            ));
        }
        let request = encode_http_request(config.endpoint, body)?;
        let mut stream = TcpStream::connect_timeout(&config.endpoint, config.timeout)
            .map_err(|error| KodiError::Io(error.to_string()))?;
        stream
            .set_read_timeout(Some(config.timeout))
            .map_err(|error| KodiError::Io(error.to_string()))?;
        stream
            .set_write_timeout(Some(config.timeout))
            .map_err(|error| KodiError::Io(error.to_string()))?;
        stream
            .write_all(&request)
            .map_err(|error| KodiError::Io(error.to_string()))?;
        stream
            .flush()
            .map_err(|error| KodiError::Io(error.to_string()))?;
        let response = read_bounded(&mut stream, config.maximum_response_bytes)?;
        decode_http_response(&response, config.maximum_response_bytes)
    }
}

fn encode_http_request(endpoint: SocketAddr, body: &[u8]) -> Result<Vec<u8>, KodiError> {
    let host = endpoint.to_string();
    if host.contains(['\r', '\n', '\0']) {
        return Err(KodiError::Validation("unsafe endpoint text".to_string()));
    }
    let head = format!(
        "POST {JSON_RPC_PATH} HTTP/1.1\r\nHost: {host}\r\nAccept: application/json\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    if head.len().saturating_add(body.len()) > MAX_REQUEST_BYTES {
        return Err(KodiError::Validation(
            "HTTP request exceeds the fixed request limit".to_string(),
        ));
    }
    let mut request = head.into_bytes();
    request.extend_from_slice(body);
    Ok(request)
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, KodiError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| KodiError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(KodiError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, KodiError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| KodiError::Http(error.to_string()))?;
    match parsed.head.status {
        401 | 403 => return Err(KodiError::AuthenticationRequired),
        status if !(200..300).contains(&status) => return Err(KodiError::HttpStatus(status)),
        _ => {}
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(KodiError::TruncatedBody {
                    expected,
                    actual: input.len(),
                });
            }
            input[..expected].to_vec()
        }
        BodyKind::UntilEof => input.to_vec(),
        BodyKind::Chunked => {
            return Err(KodiError::Http(
                "chunked Kodi responses are unsupported".to_string(),
            ))
        }
    };
    if body.len() > maximum {
        return Err(KodiError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

pub struct KodiClient<T> {
    config: KodiConfig,
    transport: T,
    next_request_id: u64,
}

impl<T: KodiTransport> KodiClient<T> {
    pub fn new(config: KodiConfig, transport: T) -> Self {
        Self {
            config,
            transport,
            next_request_id: 1,
        }
    }

    pub fn config(&self) -> &KodiConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn inspect(&mut self) -> Result<KodiSnapshot, KodiError> {
        let application = parse_application(self.call(
            "Application.GetProperties",
            json!({"properties": ["name", "version", "volume", "muted"]}),
        )?)?;
        let active_players =
            parse_active_players(self.call("Player.GetActivePlayers", json!({}))?)?;
        let mut players = Vec::with_capacity(active_players.len());
        for active in active_players {
            let properties = self.call(
                "Player.GetProperties",
                json!({
                    "playerid": active.player_id,
                    "properties": [
                        "type", "speed", "time", "percentage", "totaltime", "repeat",
                        "shuffled", "canseek", "live"
                    ]
                }),
            )?;
            players.push(parse_player(active, properties)?);
        }
        players.sort_by_key(|player| player.player_id);
        Ok(KodiSnapshot {
            application,
            players,
        })
    }

    fn call(&mut self, method: &'static str, params: JsonValue) -> Result<JsonValue, KodiError> {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.checked_add(1).ok_or_else(|| {
            KodiError::Validation("JSON-RPC request id space is exhausted".to_string())
        })?;
        let body = serde_json::to_vec(&json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params,
        }))
        .map_err(|error| KodiError::Json(error.to_string()))?;
        let response = self.transport.post(&self.config, &body)?;
        parse_rpc_envelope(&response, request_id)
    }
}

fn parse_rpc_envelope(bytes: &[u8], expected_id: u64) -> Result<JsonValue, KodiError> {
    let envelope: JsonValue =
        serde_json::from_slice(bytes).map_err(|error| KodiError::Json(error.to_string()))?;
    let object = envelope
        .as_object()
        .ok_or_else(|| KodiError::Json("envelope must be an object".to_string()))?;
    if object.get("jsonrpc").and_then(JsonValue::as_str) != Some("2.0") {
        return Err(KodiError::Json(
            "envelope must use JSON-RPC 2.0".to_string(),
        ));
    }
    if object.get("id").and_then(JsonValue::as_u64) != Some(expected_id) {
        return Err(KodiError::Json(
            "response id does not match the request".to_string(),
        ));
    }
    if let Some(error) = object.get("error") {
        let code = error
            .as_object()
            .and_then(|value| value.get("code"))
            .and_then(JsonValue::as_i64)
            .ok_or_else(|| KodiError::Json("error must contain an integer code".to_string()))?;
        return Err(KodiError::Rpc { code });
    }
    object
        .get("result")
        .cloned()
        .ok_or(KodiError::MissingField("result"))
}

fn parse_application(value: JsonValue) -> Result<KodiApplicationSnapshot, KodiError> {
    let object = required_object(&value, "application result")?;
    let name = required_bounded_string(object.get("name"), "application name")?;
    let volume = required_percentage(object.get("volume"), "application volume")?;
    let muted = required_bool(object.get("muted"), "application muted")?;
    let version = parse_version(
        object
            .get("version")
            .ok_or(KodiError::MissingField("application version"))?,
    )?;
    Ok(KodiApplicationSnapshot {
        name,
        version,
        volume,
        muted,
    })
}

fn parse_version(value: &JsonValue) -> Result<KodiVersion, KodiError> {
    let object = required_object(value, "application version")?;
    let major = required_u32(object.get("major"), "version major")?;
    let minor = required_u32(object.get("minor"), "version minor")?;
    let tag = required_bounded_string(object.get("tag"), "version tag")?;
    if !matches!(
        tag.as_str(),
        "prealpha" | "alpha" | "beta" | "releasecandidate" | "stable"
    ) {
        return Err(KodiError::Validation(
            "unknown Kodi version tag".to_string(),
        ));
    }
    let revision = optional_string_or_integer(object.get("revision"), "version revision")?;
    let tag_version = optional_bounded_string(object.get("tagversion"), "version tagversion")?;
    Ok(KodiVersion {
        major,
        minor,
        revision,
        tag,
        tag_version,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActivePlayer {
    player_id: u8,
    player_type: String,
    player_runtime: String,
}

fn parse_active_players(value: JsonValue) -> Result<Vec<ActivePlayer>, KodiError> {
    let array = value
        .as_array()
        .ok_or_else(|| KodiError::Json("active players must be an array".to_string()))?;
    if array.len() > MAX_ACTIVE_PLAYERS {
        return Err(KodiError::Validation(
            "active player count exceeds the Kodi schema limit".to_string(),
        ));
    }
    let mut ids = BTreeSet::new();
    let mut players = Vec::with_capacity(array.len());
    for value in array {
        let object = required_object(value, "active player")?;
        let player_id = required_u8_range(object.get("playerid"), "player id", 0, 2)?;
        if !ids.insert(player_id) {
            return Err(KodiError::Validation(
                "active player ids must be unique".to_string(),
            ));
        }
        let player_type = required_bounded_string(object.get("type"), "player type")?;
        if !matches!(player_type.as_str(), "audio" | "video" | "picture") {
            return Err(KodiError::Validation("unknown player type".to_string()));
        }
        let player_runtime = required_bounded_string(object.get("playertype"), "player runtime")?;
        if !matches!(player_runtime.as_str(), "internal" | "external" | "remote") {
            return Err(KodiError::Validation(
                "unknown player runtime type".to_string(),
            ));
        }
        players.push(ActivePlayer {
            player_id,
            player_type,
            player_runtime,
        });
    }
    Ok(players)
}

fn parse_player(active: ActivePlayer, value: JsonValue) -> Result<KodiPlayerSnapshot, KodiError> {
    let object = required_object(&value, "player properties")?;
    let reported_type = required_bounded_string(object.get("type"), "player property type")?;
    if reported_type != active.player_type {
        return Err(KodiError::Validation(
            "active player type changed during inspection".to_string(),
        ));
    }
    let speed = required_i64(object.get("speed"), "player speed")?;
    let position_percent = required_percentage_number(object.get("percentage"), "percentage")?;
    let elapsed_ms = parse_duration_ms(
        object
            .get("time")
            .ok_or(KodiError::MissingField("player time"))?,
    )?;
    let total_ms = parse_duration_ms(
        object
            .get("totaltime")
            .ok_or(KodiError::MissingField("player total time"))?,
    )?;
    if elapsed_ms > total_ms && total_ms != 0 {
        return Err(KodiError::Validation(
            "player elapsed time exceeds total time".to_string(),
        ));
    }
    let repeat = required_bounded_string(object.get("repeat"), "player repeat")?;
    if !matches!(repeat.as_str(), "off" | "one" | "all") {
        return Err(KodiError::Validation("unknown repeat mode".to_string()));
    }
    Ok(KodiPlayerSnapshot {
        player_id: active.player_id,
        player_type: active.player_type,
        player_runtime: active.player_runtime,
        speed,
        position_percent,
        elapsed_ms,
        total_ms,
        repeat,
        shuffled: required_bool(object.get("shuffled"), "player shuffled")?,
        can_seek: required_bool(object.get("canseek"), "player canseek")?,
        live: required_bool(object.get("live"), "player live")?,
    })
}

fn parse_duration_ms(value: &JsonValue) -> Result<u64, KodiError> {
    let object = required_object(value, "duration")?;
    let hours = required_u32(object.get("hours"), "duration hours")?;
    if hours > 1_000_000 {
        return Err(KodiError::Validation(
            "duration hours are too large".to_string(),
        ));
    }
    let minutes = required_u8_range(object.get("minutes"), "duration minutes", 0, 59)?;
    let seconds = required_u8_range(object.get("seconds"), "duration seconds", 0, 59)?;
    let milliseconds =
        required_u16_range(object.get("milliseconds"), "duration milliseconds", 0, 999)?;
    Ok(u64::from(hours) * 3_600_000
        + u64::from(minutes) * 60_000
        + u64::from(seconds) * 1_000
        + u64::from(milliseconds))
}

fn required_object<'a>(
    value: &'a JsonValue,
    field: &'static str,
) -> Result<&'a serde_json::Map<String, JsonValue>, KodiError> {
    value.as_object().ok_or(KodiError::UnexpectedResult(field))
}

fn required_bounded_string(
    value: Option<&JsonValue>,
    field: &'static str,
) -> Result<String, KodiError> {
    let value = value
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or(KodiError::MissingField(field))?;
    if value.len() > MAX_TEXT_BYTES || value.contains(['\r', '\n', '\0']) {
        return Err(KodiError::Validation(format!(
            "unsafe or over-limit {field}"
        )));
    }
    Ok(value.to_string())
}

fn optional_bounded_string(
    value: Option<&JsonValue>,
    field: &'static str,
) -> Result<Option<String>, KodiError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) if value.is_empty() => Ok(None),
        Some(value) => required_bounded_string(Some(value), field).map(Some),
    }
}

fn optional_string_or_integer(
    value: Option<&JsonValue>,
    field: &'static str,
) -> Result<Option<String>, KodiError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) => {
            required_bounded_string(Some(&JsonValue::String(value.clone())), field).map(Some)
        }
        Some(JsonValue::Number(value)) if value.as_i64().is_some() => Ok(Some(value.to_string())),
        _ => Err(KodiError::UnexpectedResult(field)),
    }
}

fn required_bool(value: Option<&JsonValue>, field: &'static str) -> Result<bool, KodiError> {
    value
        .and_then(JsonValue::as_bool)
        .ok_or(KodiError::MissingField(field))
}

fn required_i64(value: Option<&JsonValue>, field: &'static str) -> Result<i64, KodiError> {
    value
        .and_then(JsonValue::as_i64)
        .ok_or(KodiError::MissingField(field))
}

fn required_u32(value: Option<&JsonValue>, field: &'static str) -> Result<u32, KodiError> {
    value
        .and_then(JsonValue::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .ok_or(KodiError::MissingField(field))
}

fn required_u8_range(
    value: Option<&JsonValue>,
    field: &'static str,
    minimum: u8,
    maximum: u8,
) -> Result<u8, KodiError> {
    let value = value
        .and_then(JsonValue::as_u64)
        .and_then(|value| u8::try_from(value).ok())
        .ok_or(KodiError::MissingField(field))?;
    if !(minimum..=maximum).contains(&value) {
        return Err(KodiError::Validation(format!("{field} is out of range")));
    }
    Ok(value)
}

fn required_u16_range(
    value: Option<&JsonValue>,
    field: &'static str,
    minimum: u16,
    maximum: u16,
) -> Result<u16, KodiError> {
    let value = value
        .and_then(JsonValue::as_u64)
        .and_then(|value| u16::try_from(value).ok())
        .ok_or(KodiError::MissingField(field))?;
    if !(minimum..=maximum).contains(&value) {
        return Err(KodiError::Validation(format!("{field} is out of range")));
    }
    Ok(value)
}

fn required_percentage(value: Option<&JsonValue>, field: &'static str) -> Result<u8, KodiError> {
    required_u8_range(value, field, 0, 100)
}

fn required_percentage_number(
    value: Option<&JsonValue>,
    field: &'static str,
) -> Result<f64, KodiError> {
    let value = value
        .and_then(JsonValue::as_f64)
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        .ok_or(KodiError::MissingField(field))?;
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledKodiSystem {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub entity_id: EntityId,
}

pub struct KodiRuntimeIntegration<T> {
    client: KodiClient<T>,
    snapshot: Option<KodiSnapshot>,
    entity_id: Option<EntityId>,
}

impl<T: KodiTransport> KodiRuntimeIntegration<T> {
    pub fn new(client: KodiClient<T>) -> Self {
        Self {
            client,
            snapshot: None,
            entity_id: None,
        }
    }

    pub fn client(&self) -> &KodiClient<T> {
        &self.client
    }

    pub fn client_mut(&mut self) -> &mut KodiClient<T> {
        &mut self.client
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledKodiSystem, KodiError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        let installed = install_snapshot(runtime, self.client.config(), &snapshot, observed_at_ms)?;
        self.entity_id = Some(installed.entity_id.clone());
        self.snapshot = Some(snapshot);
        Ok(installed)
    }

    pub fn dispatch_command_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, KodiError> {
        if self.entity_id.as_ref() != Some(&request.entity_id) {
            return Err(KodiError::InvalidCommandArguments {
                command_type: request.command_type,
                expected: "the installed Kodi media entity",
            });
        }
        let snapshot = self
            .snapshot
            .as_ref()
            .ok_or(KodiError::NoInspectionSnapshot)?;
        let plan = command_plan(snapshot, &request)?;
        let result = runtime.execute_command_tool(principal_id, request, now_ms)?;
        execute_command(&mut self.client, plan)?;
        Ok(result)
    }
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), KodiError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(KodiError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct KodiCommandPlan {
    method: &'static str,
    params: JsonValue,
    expected: KodiCommandPostcondition,
}

#[derive(Debug, Clone, PartialEq)]
enum KodiCommandPostcondition {
    Playback(bool),
    Stopped,
    Volume(u8),
    Muted(bool),
}

fn command_plan(
    snapshot: &KodiSnapshot,
    request: &RuntimeCommandToolRequest,
) -> Result<KodiCommandPlan, KodiError> {
    match request.command_type {
        CommandType::Media(MediaCommandType::SetPlaybackState) => {
            let player_id = single_active_player_id(snapshot)?;
            let Value::Text(state) = &request.arguments else {
                return invalid_command_arguments(
                    request.command_type,
                    "play, pause, or stop text",
                );
            };
            match state.as_str() {
                "play" => Ok(KodiCommandPlan {
                    method: "Player.PlayPause",
                    params: json!({"playerid": player_id, "play": true}),
                    expected: KodiCommandPostcondition::Playback(true),
                }),
                "pause" => Ok(KodiCommandPlan {
                    method: "Player.PlayPause",
                    params: json!({"playerid": player_id, "play": false}),
                    expected: KodiCommandPostcondition::Playback(false),
                }),
                "stop" => Ok(KodiCommandPlan {
                    method: "Player.Stop",
                    params: json!({"playerid": player_id}),
                    expected: KodiCommandPostcondition::Stopped,
                }),
                _ => invalid_command_arguments(request.command_type, "play, pause, or stop text"),
            }
        }
        CommandType::Media(MediaCommandType::SetVolume) => {
            let Value::Percentage(volume) = request.arguments else {
                return invalid_command_arguments(request.command_type, "a percentage volume");
            };
            Ok(KodiCommandPlan {
                method: "Application.SetVolume",
                params: json!({"volume": volume}),
                expected: KodiCommandPostcondition::Volume(volume),
            })
        }
        CommandType::Media(MediaCommandType::SetMute) => {
            let Value::Bool(muted) = request.arguments else {
                return invalid_command_arguments(request.command_type, "a boolean mute state");
            };
            Ok(KodiCommandPlan {
                method: "Application.SetMute",
                params: json!({"mute": muted}),
                expected: KodiCommandPostcondition::Muted(muted),
            })
        }
        command_type => Err(KodiError::UnsupportedCommand(command_type)),
    }
}

fn single_active_player_id(snapshot: &KodiSnapshot) -> Result<u8, KodiError> {
    match snapshot.players.as_slice() {
        [] => Err(KodiError::NoActivePlayer),
        [player] => Ok(player.player_id),
        _ => Err(KodiError::AmbiguousActivePlayers),
    }
}

fn invalid_command_arguments<T>(
    command_type: CommandType,
    expected: &'static str,
) -> Result<T, KodiError> {
    Err(KodiError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn execute_command<T: KodiTransport>(
    client: &mut KodiClient<T>,
    plan: KodiCommandPlan,
) -> Result<(), KodiError> {
    let result = client.call(plan.method, plan.params)?;
    match plan.expected {
        KodiCommandPostcondition::Playback(playing) => {
            let speed = required_object(&result, "playback response")?
                .get("speed")
                .and_then(JsonValue::as_i64)
                .ok_or(KodiError::MissingField("playback response speed"))?;
            if (playing && speed == 0) || (!playing && speed != 0) {
                return Err(KodiError::UnexpectedResult("playback postcondition"));
            }
        }
        KodiCommandPostcondition::Stopped => {
            if result.as_str() != Some("OK") {
                return Err(KodiError::UnexpectedResult("stop postcondition"));
            }
        }
        KodiCommandPostcondition::Volume(expected) => {
            if result.as_u64() != Some(u64::from(expected)) {
                return Err(KodiError::UnexpectedResult("volume postcondition"));
            }
        }
        KodiCommandPostcondition::Muted(expected) => {
            if result.as_bool() != Some(expected) {
                return Err(KodiError::UnexpectedResult("mute postcondition"));
            }
        }
    }
    Ok(())
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &KodiConfig,
    snapshot: &KodiSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledKodiSystem, KodiError> {
    let stable_endpoint = stable_component(&config.endpoint.to_string());
    let device_id = DeviceId::trusted(format!("kodi:{stable_endpoint}"));
    let entity_id = EntityId::trusted(format!("kodi:{stable_endpoint}:player"));
    let protocol = ProtocolFamily::Vendor(PROTOCOL_ID.to_string());

    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.endpoint_url());
    bridge.hardware_model = Some("Kodi JSON-RPC host".to_string());
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![ProtocolIdentifier::new(
        protocol.clone(),
        "bridge_endpoint",
        config.endpoint.to_string(),
    )
    .map_err(|error| KodiError::Validation(error.to_string()))?];
    bridge.metadata = vec![Metadata::new("kodi.transport", "http_jsonrpc_2")];
    runtime.upsert_bridge(bridge)?;

    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: "Kodi Foundation".to_string(),
        model: snapshot.application.name.clone(),
        name: snapshot.application.name.clone(),
        serial: None,
        firmware_version: Some(snapshot.application.version.to_string()),
        room_id: None,
        entity_ids: vec![entity_id.clone()],
        identifiers: vec![ProtocolIdentifier::new(
            protocol,
            "device_endpoint",
            config.endpoint.to_string(),
        )
        .map_err(|error| KodiError::Validation(error.to_string()))?],
        health: Health::Online,
        metadata: vec![Metadata::new(
            "kodi.active_player_count",
            snapshot.players.len().to_string(),
        )],
    })?;

    runtime.upsert_entity(Entity {
        entity_id: entity_id.clone(),
        device_id: device_id.clone(),
        kind: EntityKind::Unknown,
        name: snapshot.application.name.clone(),
        capabilities: vec![
            Capability::new(
                CapabilityId::trusted("media.player_state"),
                CapabilityMode::Observe,
                ValueKind::Object,
            ),
            Capability::media_playback(),
            Capability::media_volume(),
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
        metadata: vec![Metadata::new("kodi.control_surface", "bounded_media")],
    })?;

    Ok(InstalledKodiSystem {
        bridge_id: config.bridge_id.clone(),
        device_id,
        entity_id,
    })
}

fn snapshot_value(snapshot: &KodiSnapshot) -> Value {
    Value::Object(vec![
        (
            "application_name".to_string(),
            Value::Text(snapshot.application.name.clone()),
        ),
        (
            "application_version".to_string(),
            Value::Text(snapshot.application.version.to_string()),
        ),
        (
            "volume".to_string(),
            Value::Percentage(snapshot.application.volume),
        ),
        ("muted".to_string(), Value::Bool(snapshot.application.muted)),
        (
            "active_players".to_string(),
            Value::Array(snapshot.players.iter().map(player_value).collect()),
        ),
    ])
}

fn player_value(player: &KodiPlayerSnapshot) -> Value {
    Value::Object(vec![
        (
            "player_id".to_string(),
            Value::Integer(i64::from(player.player_id)),
        ),
        (
            "player_type".to_string(),
            Value::Text(player.player_type.clone()),
        ),
        (
            "player_runtime".to_string(),
            Value::Text(player.player_runtime.clone()),
        ),
        (
            "playback_state".to_string(),
            Value::Text(player.playback_state().to_string()),
        ),
        ("speed".to_string(), Value::Integer(player.speed)),
        (
            "position_percent".to_string(),
            Value::Number(player.position_percent),
        ),
        (
            "elapsed_ms".to_string(),
            Value::Integer(i64::try_from(player.elapsed_ms).unwrap_or(i64::MAX)),
        ),
        (
            "total_ms".to_string(),
            Value::Integer(i64::try_from(player.total_ms).unwrap_or(i64::MAX)),
        ),
        ("repeat".to_string(), Value::Text(player.repeat.clone())),
        ("shuffled".to_string(), Value::Bool(player.shuffled)),
        ("can_seek".to_string(), Value::Bool(player.can_seek)),
        ("live".to_string(), Value::Bool(player.live)),
    ])
}

fn stable_component(value: &str) -> String {
    let mut normalized = String::new();
    let mut separator = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            normalized.push(character.to_ascii_lowercase());
            separator = false;
        } else if !separator && !normalized.is_empty() {
            normalized.push('-');
            separator = true;
        }
    }
    normalized.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::collections::VecDeque;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    const APPLICATION: &str = r#"{"jsonrpc":"2.0","id":1,"result":{"name":"Kodi","version":{"major":22,"minor":0,"revision":"abc123","tag":"stable","tagversion":""},"volume":35,"muted":false}}"#;
    const ACTIVE: &str = r#"{"jsonrpc":"2.0","id":2,"result":[{"playerid":1,"type":"video","playertype":"internal"}]}"#;
    const PLAYER: &str = r#"{"jsonrpc":"2.0","id":3,"result":{"type":"video","speed":1,"time":{"hours":0,"minutes":1,"seconds":2,"milliseconds":3},"percentage":12.5,"totaltime":{"hours":1,"minutes":2,"seconds":3,"milliseconds":4},"repeat":"off","shuffled":false,"canseek":true,"live":false}}"#;

    #[derive(Debug)]
    struct ScriptedTransport {
        calls: Arc<AtomicUsize>,
        responses: VecDeque<Vec<u8>>,
    }

    impl KodiTransport for ScriptedTransport {
        fn post(&mut self, _config: &KodiConfig, _body: &[u8]) -> Result<Vec<u8>, KodiError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.responses
                .pop_front()
                .ok_or_else(|| KodiError::Io("unexpected scripted transport call".to_string()))
        }
    }

    fn config(port: u16) -> KodiConfig {
        KodiConfig::new(
            BridgeId::trusted("kodi:test"),
            SocketAddr::from(([127, 0, 0, 1], port)),
        )
        .unwrap()
        .with_timeout(Duration::from_secs(2))
    }

    fn scripted_client(calls: Arc<AtomicUsize>) -> KodiClient<ScriptedTransport> {
        KodiClient::new(
            config(8080),
            ScriptedTransport {
                calls,
                responses: [APPLICATION, ACTIVE, PLAYER]
                    .into_iter()
                    .map(|response| response.as_bytes().to_vec())
                    .collect(),
            },
        )
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted(format!("grant:{}", principal.as_str())),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1,
            )
            .with_expiry(100),
        );
    }

    #[test]
    fn local_endpoint_and_rpc_envelope_validation_fail_closed() {
        assert!(KodiConfig::new(
            BridgeId::trusted("public"),
            "203.0.113.8:8080".parse().unwrap()
        )
        .is_err());
        assert!(KodiConfig::new(
            BridgeId::trusted("zero-port"),
            "127.0.0.1:0".parse().unwrap()
        )
        .is_err());
        assert!(KodiConfig::new(
            BridgeId::trusted("private"),
            "192.168.1.8:8080".parse().unwrap()
        )
        .is_ok());
        assert!(parse_rpc_envelope(br#"{"jsonrpc":"2.0","id":9,"result":{}}"#, 8).is_err());
        assert_eq!(
            parse_rpc_envelope(
                br#"{"jsonrpc":"2.0","id":8,"error":{"code":-32601,"message":"secret"}}"#,
                8
            ),
            Err(KodiError::Rpc { code: -32601 })
        );
    }

    #[test]
    fn authorized_snapshot_is_strictly_parsed_and_installed() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut integration = KodiRuntimeIntegration::new(scripted_client(Arc::clone(&calls)));
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:kodi-read");
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 10)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        let entity = runtime.registry().entity(&installed.entity_id).unwrap();
        assert_eq!(entity.name, "Kodi");
        assert_eq!(entity.capabilities.len(), 3);
        let state = entity.state.as_ref().unwrap();
        assert!(format!("{:?}", state.value).contains("12.5"));
        assert_eq!(runtime.registry().bridges().count(), 1);
        assert_eq!(runtime.registry().devices().count(), 1);
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut integration = KodiRuntimeIntegration::new(scripted_client(Arc::clone(&calls)));
        let mut runtime = SmartHomeRuntime::new();
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut runtime,
                AgentId::trusted("agent:kodi-denied-read"),
                10,
            ),
            Err(KodiError::Runtime(RuntimeError::UnauthorizedTool { .. }))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.registry().bridges().count(), 0);
    }

    fn read_request(stream: &mut TcpStream) -> JsonValue {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut content_length = 0usize;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                content_length = value.trim().parse().unwrap();
            }
        }
        let mut body = vec![0u8; content_length];
        reader.read_exact(&mut body).unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn write_json_response(stream: &mut TcpStream, body: &JsonValue) {
        let body = serde_json::to_vec(body).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(&body).unwrap();
    }

    #[test]
    fn loopback_authorized_inspection_and_commands_use_only_fixed_methods() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let methods = Arc::new(Mutex::new(Vec::new()));
        let server_methods = Arc::clone(&methods);
        let server = thread::spawn(move || {
            for _ in 0..7 {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                let id = request.get("id").cloned().unwrap();
                let method = request.get("method").and_then(JsonValue::as_str).unwrap();
                server_methods.lock().unwrap().push(method.to_string());
                let result = match method {
                    "Application.GetProperties" => json!({
                        "name": "Kodi",
                        "version": {"major": 22, "minor": 0, "revision": "loop", "tag": "stable"},
                        "volume": 35,
                        "muted": false
                    }),
                    "Player.GetActivePlayers" => json!([
                        {"playerid": 1, "type": "video", "playertype": "internal"}
                    ]),
                    "Player.GetProperties" => json!({
                        "type": "video",
                        "speed": 1,
                        "time": {"hours": 0, "minutes": 0, "seconds": 1, "milliseconds": 0},
                        "percentage": 1.0,
                        "totaltime": {"hours": 0, "minutes": 2, "seconds": 0, "milliseconds": 0},
                        "repeat": "off",
                        "shuffled": false,
                        "canseek": true,
                        "live": false
                    }),
                    "Player.PlayPause" => {
                        let play = request["params"]["play"].as_bool().unwrap();
                        json!({"speed": if play { 1 } else { 0 }})
                    }
                    "Player.Stop" => json!("OK"),
                    "Application.SetVolume" => request["params"]["volume"].clone(),
                    "Application.SetMute" => request["params"]["mute"].clone(),
                    _ => panic!("unexpected method {method}"),
                };
                write_json_response(
                    &mut stream,
                    &json!({"jsonrpc": "2.0", "id": id, "result": result}),
                );
            }
        });

        let client = KodiClient::new(config(address.port()), KodiLanTransport);
        let mut integration = KodiRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:kodi-loopback");
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 10)
            .unwrap();
        let requests = [
            RuntimeCommandToolRequest::new(
                installed.entity_id.clone(),
                CommandType::Media(MediaCommandType::SetPlaybackState),
                Value::Text("pause".to_string()),
            ),
            RuntimeCommandToolRequest::new(
                installed.entity_id.clone(),
                CommandType::Media(MediaCommandType::SetPlaybackState),
                Value::Text("stop".to_string()),
            ),
            RuntimeCommandToolRequest::new(
                installed.entity_id.clone(),
                CommandType::Media(MediaCommandType::SetVolume),
                Value::Percentage(42),
            ),
            RuntimeCommandToolRequest::new(
                installed.entity_id,
                CommandType::Media(MediaCommandType::SetMute),
                Value::Bool(true),
            ),
        ];
        for (offset, request) in requests.into_iter().enumerate() {
            let result = integration
                .dispatch_command_authorized(
                    &mut runtime,
                    principal.clone(),
                    request,
                    20 + offset as u64,
                )
                .unwrap();
            assert!(result.is_accepted());
        }
        server.join().unwrap();
        assert_eq!(
            methods.lock().unwrap().as_slice(),
            [
                "Application.GetProperties",
                "Player.GetActivePlayers",
                "Player.GetProperties",
                "Player.PlayPause",
                "Player.Stop",
                "Application.SetVolume",
                "Application.SetMute",
            ]
        );
    }

    #[test]
    fn denied_command_reaches_no_additional_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut integration = KodiRuntimeIntegration::new(scripted_client(Arc::clone(&calls)));
        let mut runtime = SmartHomeRuntime::new();
        let installer = AgentId::trusted("agent:kodi-installer");
        grant(&mut runtime, &installer);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, installer, 10)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                AgentId::trusted("agent:kodi-denied-command"),
                RuntimeCommandToolRequest::new(
                    installed.entity_id,
                    CommandType::Media(MediaCommandType::SetVolume),
                    Value::Percentage(20),
                ),
                20,
            ),
            Err(KodiError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn http_bounds_authentication_and_command_postconditions_fail_closed() {
        let unauthorized = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(
            decode_http_response(unauthorized, 1024),
            Err(KodiError::AuthenticationRequired)
        );
        let oversized = b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nxxxx";
        assert_eq!(
            decode_http_response(oversized, 3),
            Err(KodiError::ResponseTooLarge { limit: 3 })
        );
        let snapshot = KodiSnapshot {
            application: KodiApplicationSnapshot {
                name: "Kodi".to_string(),
                version: KodiVersion {
                    major: 22,
                    minor: 0,
                    revision: None,
                    tag: "stable".to_string(),
                    tag_version: None,
                },
                volume: 20,
                muted: false,
            },
            players: Vec::new(),
        };
        let request = RuntimeCommandToolRequest::new(
            EntityId::trusted("kodi:test:player"),
            CommandType::Media(MediaCommandType::SetPlaybackState),
            Value::Text("play".to_string()),
        );
        assert_eq!(
            command_plan(&snapshot, &request),
            Err(KodiError::NoActivePlayer)
        );
    }
}
