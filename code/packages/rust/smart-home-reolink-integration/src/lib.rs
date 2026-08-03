//! Authenticated Reolink camera and NVR CGI integration for D23.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde_json::{json, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandType, Device, DeviceControlCommandType, DeviceId, Entity, EntityId,
    EntityKind, Health, IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, SmartHomeTool,
    StateConfidence, StateSnapshot, StateSource, Value, ValueKind, VaultRef,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig, TlsConnector};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.3.0";
pub const INTEGRATION_ID: &str = "reolink";
pub const PROTOCOL_ID: &str = "reolink_cgi";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MIN_PTZ_SPEED: u32 = 1;
pub const MAX_PTZ_SPEED: u32 = 64;
pub const MAX_PTZ_DURATION_MS: u64 = 5_000;

#[derive(Debug)]
pub enum ReolinkError {
    Validation(String),
    Json(serde_json::Error),
    Url(UrlError),
    Io(String),
    Tls(String),
    Http(String),
    HttpStatus(u16),
    ResponseTooLarge {
        limit: usize,
    },
    TruncatedBody {
        expected: usize,
        actual: usize,
    },
    Api {
        command: String,
        code: i64,
        message: String,
    },
    MissingField {
        command: &'static str,
        field: &'static str,
    },
    NoChannels,
    UnknownEntity(EntityId),
    UnsupportedCommand(CommandType),
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    VerificationFailed {
        channel: u32,
        expected: bool,
        actual: bool,
    },
    Runtime(RuntimeError),
}

impl fmt::Display for ReolinkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Reolink input: {message}"),
            Self::Json(error) => write!(formatter, "invalid Reolink JSON: {error}"),
            Self::Url(error) => write!(formatter, "invalid Reolink URL: {error}"),
            Self::Io(message) => write!(formatter, "Reolink LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "Reolink TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Reolink HTTP response: {message}"),
            Self::HttpStatus(status) => {
                write!(formatter, "Reolink endpoint returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Reolink response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Reolink response body is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Api {
                command,
                code,
                message,
            } => {
                write!(
                    formatter,
                    "Reolink command {command} failed with code {code}: {message}"
                )
            }
            Self::MissingField { command, field } => {
                write!(formatter, "Reolink command {command} is missing {field}")
            }
            Self::NoChannels => formatter.write_str("Reolink device returned no camera channels"),
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown Reolink entity {}", entity_id.as_str())
            }
            Self::UnsupportedCommand(command_type) => {
                write!(formatter, "unsupported Reolink command {command_type:?}")
            }
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "invalid arguments for Reolink command {command_type:?}; expected {expected}"
            ),
            Self::VerificationFailed {
                channel,
                expected,
                actual,
            } => write!(
                formatter,
                "Reolink channel {channel} recording readback was {actual}, expected {expected}"
            ),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ReolinkError {}

impl From<serde_json::Error> for ReolinkError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<UrlError> for ReolinkError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<RuntimeError> for ReolinkError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct ReolinkCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl ReolinkCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, ReolinkError> {
        let username = username.into();
        let password = password.into();
        if username.trim().is_empty() || password.is_empty() {
            return Err(ReolinkError::Validation(
                "username and password must not be empty".to_string(),
            ));
        }
        Ok(Self {
            username: Zeroizing::new(username),
            password: Zeroizing::new(password),
        })
    }
}

impl fmt::Debug for ReolinkCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ReolinkCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub credential_ref: VaultRef,
}

impl ReolinkConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        credential_ref: VaultRef,
    ) -> Result<Self, ReolinkError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        if !matches!(parsed.scheme.as_str(), "http" | "https") {
            return Err(ReolinkError::Validation(
                "base URL must use http or https".to_string(),
            ));
        }
        if parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
        {
            return Err(ReolinkError::Validation(
                "base URL must contain a host and no credentials, query, or fragment".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            credential_ref,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkDeviceInformation {
    pub name: String,
    pub model: String,
    pub serial: String,
    pub firmware_version: Option<String>,
    pub hardware_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkChannelStatus {
    pub channel: u32,
    pub name: String,
    pub online: bool,
    pub sleeping: bool,
    pub motion: Option<bool>,
    pub recording_enabled: Option<bool>,
    pub ptz_presets: Option<Vec<ReolinkPtzPreset>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkPtzPreset {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReolinkPtzDirection {
    Left,
    Right,
    Up,
    Down,
}

impl ReolinkPtzDirection {
    fn from_label(label: &str) -> Option<Self> {
        match label {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            _ => None,
        }
    }

    fn operation(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Up => "Up",
            Self::Down => "Down",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkSnapshot {
    pub device: ReolinkDeviceInformation,
    pub channels: Vec<ReolinkChannelStatus>,
}

pub trait ReolinkTransport {
    fn post_json(&mut self, endpoint: &str, body: &[u8]) -> Result<Vec<u8>, ReolinkError>;
}

pub struct ReolinkLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    timeout: Duration,
    maximum_response_bytes: usize,
}

impl Default for ReolinkLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl ReolinkLanTransport {
    pub fn new(connector: Box<dyn TlsConnector>, tls_config: TlsConfig) -> Self {
        Self {
            connector,
            tls_config,
            timeout: Duration::from_secs(5),
            maximum_response_bytes: DEFAULT_MAX_RESPONSE_BYTES,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    pub fn with_maximum_response_bytes(mut self, maximum: usize) -> Self {
        self.maximum_response_bytes = maximum.max(1);
        self
    }
}

impl ReolinkTransport for ReolinkLanTransport {
    fn post_json(&mut self, endpoint: &str, body: &[u8]) -> Result<Vec<u8>, ReolinkError> {
        let url = Url::parse(endpoint)?;
        let host = url
            .host
            .as_deref()
            .ok_or_else(|| ReolinkError::Validation("endpoint is missing a host".to_string()))?;
        let port = url
            .effective_port()
            .ok_or_else(|| ReolinkError::Validation("endpoint is missing a port".to_string()))?;
        let request = Zeroizing::new(encode_http_request(&url, body)?);
        let response = match url.scheme.as_str() {
            "http" => {
                let mut stream = connect_tcp(host, port, self.timeout)?;
                write_request(&mut stream, &request)?;
                read_bounded(&mut stream, self.maximum_response_bytes)?
            }
            "https" => {
                let mut config = self.tls_config.clone();
                config.connect_timeout = self.timeout;
                config.read_timeout = Some(self.timeout);
                config.write_timeout = Some(self.timeout);
                let mut stream = self
                    .connector
                    .connect(host, port, &config)
                    .map_err(|error| ReolinkError::Tls(error.to_string()))?;
                write_request(&mut stream, &request)?;
                let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
                stream
                    .close_notify()
                    .map_err(|error| ReolinkError::Tls(error.to_string()))?;
                bytes
            }
            scheme => {
                return Err(ReolinkError::Validation(format!(
                    "unsupported Reolink URL scheme `{scheme}`"
                )))
            }
        };
        decode_http_response(&response, self.maximum_response_bytes)
    }
}

struct ReolinkSession {
    token: Zeroizing<String>,
    lease_seconds: u64,
}

impl fmt::Debug for ReolinkSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReolinkSession")
            .field("token", &"[REDACTED]")
            .field("lease_seconds", &self.lease_seconds)
            .finish()
    }
}

pub struct ReolinkClient<T> {
    config: ReolinkConfig,
    credentials: ReolinkCredentials,
    transport: T,
}

impl<T: ReolinkTransport> ReolinkClient<T> {
    pub fn new(config: ReolinkConfig, credentials: ReolinkCredentials, transport: T) -> Self {
        Self {
            config,
            credentials,
            transport,
        }
    }

    pub fn config(&self) -> &ReolinkConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<ReolinkSnapshot, ReolinkError> {
        let session = self.login()?;
        let result = self.inspect_session(&session);
        let logout = self.logout(&session);
        match (result, logout) {
            (Ok(snapshot), Ok(())) => Ok(snapshot),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    fn inspect_session(
        &mut self,
        session: &ReolinkSession,
    ) -> Result<ReolinkSnapshot, ReolinkError> {
        let device =
            parse_device_information(&self.command("GetDevInfo", Some(session), json!({}))?)?;
        let mut channels =
            parse_channels(&self.command("GetChannelstatus", Some(session), json!({}))?)?;
        if channels.is_empty() {
            return Err(ReolinkError::NoChannels);
        }
        for channel in channels
            .iter_mut()
            .filter(|channel| channel.online && !channel.sleeping)
        {
            let value = self.command(
                "GetMdState",
                Some(session),
                json!({"channel": channel.channel}),
            );
            channel.motion = match value {
                Ok(value) => parse_motion(&value),
                Err(ReolinkError::Api { .. }) => None,
                Err(error) => return Err(error),
            };
            let value = self.command(
                "GetRecV20",
                Some(session),
                json!({"channel": channel.channel}),
            );
            channel.recording_enabled = match value {
                Ok(value) => Some(parse_recording_enabled(&value)?),
                Err(ReolinkError::Api { .. }) => None,
                Err(error) => return Err(error),
            };
            let value = self.command(
                "GetPtzPreset",
                Some(session),
                json!({"channel": channel.channel}),
            );
            channel.ptz_presets = match value {
                Ok(value) => Some(parse_ptz_presets(&value)?),
                Err(ReolinkError::Api { .. }) => None,
                Err(error) => return Err(error),
            };
        }
        Ok(ReolinkSnapshot { device, channels })
    }

    pub fn set_recording_and_verify(
        &mut self,
        channel: u32,
        enabled: bool,
    ) -> Result<(), ReolinkError> {
        let session = self.login()?;
        let result = self.set_recording_session(&session, channel, enabled);
        let logout = self.logout(&session);
        match (result, logout) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), _) => Err(error),
            (Ok(()), Err(error)) => Err(error),
        }
    }

    fn set_recording_session(
        &mut self,
        session: &ReolinkSession,
        channel: u32,
        enabled: bool,
    ) -> Result<(), ReolinkError> {
        self.command(
            "SetRecV20",
            Some(session),
            json!({"Rec": {"channel": channel, "enable": enabled as u8}}),
        )?;
        let confirmed = parse_recording_enabled(&self.command(
            "GetRecV20",
            Some(session),
            json!({"channel": channel}),
        )?)?;
        if confirmed != enabled {
            return Err(ReolinkError::VerificationFailed {
                channel,
                expected: enabled,
                actual: confirmed,
            });
        }
        Ok(())
    }

    pub fn recall_ptz_preset(
        &mut self,
        channel: u32,
        preset_id: u32,
        speed: u32,
    ) -> Result<(), ReolinkError> {
        let session = self.login()?;
        let result = self.command(
            "PtzCtrl",
            Some(&session),
            json!({"channel": channel, "id": preset_id, "op": "ToPos", "speed": speed}),
        );
        let logout = self.logout(&session);
        match (result, logout) {
            (Ok(_), Ok(())) => Ok(()),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    pub fn move_ptz_bounded(
        &mut self,
        channel: u32,
        direction: ReolinkPtzDirection,
        speed: u32,
        duration_ms: u64,
    ) -> Result<(), ReolinkError> {
        let session = self.login()?;
        let result = self.move_ptz_session(&session, channel, direction, speed, duration_ms);
        let logout = self.logout(&session);
        match (result, logout) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(error), _) => Err(error),
            (Ok(()), Err(error)) => Err(error),
        }
    }

    fn move_ptz_session(
        &mut self,
        session: &ReolinkSession,
        channel: u32,
        direction: ReolinkPtzDirection,
        speed: u32,
        duration_ms: u64,
    ) -> Result<(), ReolinkError> {
        self.command(
            "PtzCtrl",
            Some(session),
            json!({"channel": channel, "op": direction.operation(), "speed": speed}),
        )?;
        thread::sleep(Duration::from_millis(duration_ms));
        self.command(
            "PtzCtrl",
            Some(session),
            json!({"channel": channel, "op": "Stop", "speed": speed}),
        )?;
        Ok(())
    }

    fn login(&mut self) -> Result<ReolinkSession, ReolinkError> {
        let value = self.command(
            "Login",
            None,
            json!({
                "User": {
                    "userName": self.credentials.username.as_str(),
                    "password": self.credentials.password.as_str(),
                }
            }),
        )?;
        let token = value
            .pointer("/Token/name")
            .and_then(JsonValue::as_str)
            .ok_or(ReolinkError::MissingField {
                command: "Login",
                field: "value.Token.name",
            })?;
        if token.is_empty()
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(ReolinkError::Validation(
                "login token contains unsafe URL characters".to_string(),
            ));
        }
        Ok(ReolinkSession {
            token: Zeroizing::new(token.to_string()),
            lease_seconds: value
                .pointer("/Token/leaseTime")
                .and_then(JsonValue::as_u64)
                .unwrap_or_default(),
        })
    }

    fn logout(&mut self, session: &ReolinkSession) -> Result<(), ReolinkError> {
        self.command("Logout", Some(session), json!({})).map(|_| ())
    }

    fn command(
        &mut self,
        command: &'static str,
        session: Option<&ReolinkSession>,
        param: JsonValue,
    ) -> Result<JsonValue, ReolinkError> {
        let endpoint = api_endpoint(&self.config.base_url, command, session)?;
        let body = Zeroizing::new(serde_json::to_vec(&json!([{
            "cmd": command,
            "action": 0,
            "param": param,
        }]))?);
        let response = self.transport.post_json(&endpoint, &body)?;
        parse_command_response(command, &response)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledReolinkDevice {
    pub bridge_id: BridgeId,
    pub device_ids: Vec<DeviceId>,
    pub camera_entity_ids: Vec<EntityId>,
    pub recording_entity_ids: Vec<EntityId>,
    pub ptz_entity_ids: Vec<EntityId>,
    pub motion_entity_ids: Vec<EntityId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReolinkPtzSupport {
    channel: u32,
    preset_ids: Vec<u32>,
}

pub struct ReolinkRuntimeIntegration<T> {
    client: ReolinkClient<T>,
    recording_channels: BTreeMap<EntityId, u32>,
    ptz_channels: BTreeMap<EntityId, ReolinkPtzSupport>,
}

impl<T: ReolinkTransport> ReolinkRuntimeIntegration<T> {
    pub fn new(client: ReolinkClient<T>) -> Self {
        Self {
            client,
            recording_channels: BTreeMap::new(),
            ptz_channels: BTreeMap::new(),
        }
    }

    pub fn client(&self) -> &ReolinkClient<T> {
        &self.client
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledReolinkDevice, ReolinkError> {
        let decision = runtime.authorize_tool_for_principal(
            principal_id.clone(),
            SmartHomeTool::GetState,
            observed_at_ms,
        );
        if !decision.missing_capabilities.is_empty() {
            return Err(ReolinkError::Runtime(RuntimeError::UnauthorizedTool {
                principal_id,
                tool: SmartHomeTool::GetState,
                missing_capabilities: decision.missing_capabilities,
            }));
        }
        let snapshot = self.client.inspect()?;
        let installed = install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)?;
        self.recording_channels = snapshot
            .channels
            .iter()
            .filter(|channel| channel.recording_enabled.is_some())
            .map(|channel| {
                (
                    camera_entity_id(&snapshot.device.serial, channel.channel),
                    channel.channel,
                )
            })
            .collect();
        self.ptz_channels = snapshot
            .channels
            .iter()
            .filter_map(|channel| {
                channel.ptz_presets.as_ref().map(|presets| {
                    (
                        camera_entity_id(&snapshot.device.serial, channel.channel),
                        ReolinkPtzSupport {
                            channel: channel.channel,
                            preset_ids: presets.iter().map(|preset| preset.id).collect(),
                        },
                    )
                })
            })
            .collect();
        Ok(installed)
    }

    pub fn dispatch_command_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, ReolinkError> {
        match request.command_type {
            CommandType::DeviceControl(DeviceControlCommandType::SetCameraRecording) => {
                let channel = self
                    .recording_channels
                    .get(&request.entity_id)
                    .copied()
                    .ok_or_else(|| ReolinkError::UnknownEntity(request.entity_id.clone()))?;
                let Value::Bool(enabled) = request.arguments else {
                    return invalid_command_arguments(
                        request.command_type,
                        "a recording-enabled boolean",
                    );
                };
                let entity_id = request.entity_id.clone();
                let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
                let mut result = runtime.submit_command(command, now_ms)?;
                self.client.set_recording_and_verify(channel, enabled)?;
                let mut entity = runtime
                    .registry()
                    .entity(&entity_id)
                    .cloned()
                    .ok_or_else(|| ReolinkError::UnknownEntity(entity_id.clone()))?;
                entity.state = Some(confirmed_recording_state(
                    entity_id,
                    entity.state,
                    enabled,
                    now_ms,
                ));
                runtime.upsert_entity(entity)?;
                result.message = Some(format!(
                    "Reolink confirmed channel {channel} recording {}",
                    if enabled { "enabled" } else { "disabled" }
                ));
                Ok(result)
            }
            CommandType::DeviceControl(DeviceControlCommandType::RecallCameraPtzPreset) => {
                let support = self
                    .ptz_channels
                    .get(&request.entity_id)
                    .cloned()
                    .ok_or_else(|| ReolinkError::UnknownEntity(request.entity_id.clone()))?;
                let (preset_id, speed) = ptz_preset_arguments(&request)?;
                if !support.preset_ids.contains(&preset_id) {
                    return invalid_command_arguments(
                        request.command_type,
                        "an object with a probed preset_id and speed from 1 through 64",
                    );
                }
                let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
                let mut result = runtime.submit_command(command, now_ms)?;
                self.client
                    .recall_ptz_preset(support.channel, preset_id, speed)?;
                result.message = Some(format!(
                    "Reolink channel {} accepted PTZ preset {preset_id}",
                    support.channel
                ));
                Ok(result)
            }
            CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz) => {
                let support = self
                    .ptz_channels
                    .get(&request.entity_id)
                    .cloned()
                    .ok_or_else(|| ReolinkError::UnknownEntity(request.entity_id.clone()))?;
                let (direction, speed, duration_ms) = ptz_move_arguments(&request)?;
                let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
                let mut result = runtime.submit_command(command, now_ms)?;
                self.client
                    .move_ptz_bounded(support.channel, direction, speed, duration_ms)?;
                result.message = Some(format!(
                    "Reolink channel {} completed bounded PTZ {} movement",
                    support.channel,
                    direction.operation().to_ascii_lowercase()
                ));
                Ok(result)
            }
            command_type => Err(ReolinkError::UnsupportedCommand(command_type)),
        }
    }
}

pub fn discovery_record(
    config: &ReolinkConfig,
    snapshot: &ReolinkSnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, ReolinkError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&snapshot.device.serial),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| ReolinkError::Validation(error.to_string()))?
    .with_display_name(snapshot.device.name.clone())
    .with_address(config.base_url.clone())
    .with_hardware_model(snapshot.device.model.clone())
    .with_firmware_version(snapshot.device.firmware_version.clone().unwrap_or_default())
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("reolink.protocol", PROTOCOL_ID)
    .with_metadata("reolink.channel_count", snapshot.channels.len().to_string()))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &ReolinkConfig,
    snapshot: &ReolinkSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledReolinkDevice, ReolinkError> {
    let native_id = stable_component(&snapshot.device.serial);
    if native_id.is_empty() {
        return Err(ReolinkError::Validation(
            "device serial is empty".to_string(),
        ));
    }
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some(snapshot.device.model.clone());
    bridge.firmware_version = snapshot.device.firmware_version.clone();
    bridge.auth_ref = Some(config.credential_ref.clone());
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("serial", &snapshot.device.serial)?];
    bridge.metadata = vec![
        Metadata::new("reolink.transport", "authenticated_http_cgi"),
        Metadata::new("reolink.channel_count", snapshot.channels.len().to_string()),
    ];
    runtime.upsert_bridge(bridge)?;

    let mut installed = InstalledReolinkDevice {
        bridge_id: config.bridge_id.clone(),
        device_ids: Vec::new(),
        camera_entity_ids: Vec::new(),
        recording_entity_ids: Vec::new(),
        ptz_entity_ids: Vec::new(),
        motion_entity_ids: Vec::new(),
    };
    for channel in &snapshot.channels {
        let channel_id = format!("{native_id}:ch{}", channel.channel);
        let device_id = DeviceId::trusted(format!("reolink:{channel_id}"));
        let camera_id = camera_entity_id(&snapshot.device.serial, channel.channel);
        let motion_id = EntityId::trusted(format!("reolink:{channel_id}:motion"));
        let mut entity_ids = vec![camera_id.clone()];
        if channel.motion.is_some() {
            entity_ids.push(motion_id.clone());
        }
        let health = if channel.online {
            Health::Online
        } else {
            Health::Offline
        };
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "Reolink".to_string(),
            model: snapshot.device.model.clone(),
            name: channel.name.clone(),
            serial: Some(format!("{}:ch{}", snapshot.device.serial, channel.channel)),
            firmware_version: snapshot.device.firmware_version.clone(),
            room_id: None,
            entity_ids: entity_ids.clone(),
            identifiers: vec![protocol_identifier(
                "channel",
                &channel.channel.to_string(),
            )?],
            health,
            metadata: vec![Metadata::new("reolink.protocol", PROTOCOL_ID)],
        })?;
        let mut camera_capabilities = vec![Capability::new(
            CapabilityId::trusted("camera.channel_status"),
            CapabilityMode::Observe,
            ValueKind::Object,
        )];
        if channel.recording_enabled.is_some() {
            camera_capabilities.push(Capability::camera_recording());
            installed.recording_entity_ids.push(camera_id.clone());
        }
        if channel.ptz_presets.is_some() {
            camera_capabilities.push(Capability::camera_ptz());
            installed.ptz_entity_ids.push(camera_id.clone());
        }
        runtime.upsert_entity(Entity {
            entity_id: camera_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Camera,
            name: channel.name.clone(),
            capabilities: camera_capabilities,
            state: Some(StateSnapshot {
                entity_id: camera_id.clone(),
                value: channel_state(channel),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new(
                "reolink.channel",
                channel.channel.to_string(),
            )],
        })?;
        if let Some(motion) = channel.motion {
            runtime.upsert_entity(Entity {
                entity_id: motion_id.clone(),
                device_id: device_id.clone(),
                kind: EntityKind::Sensor,
                name: format!("{} Motion", channel.name),
                capabilities: vec![Capability::new(
                    CapabilityId::trusted("sensor.occupancy"),
                    CapabilityMode::Observe,
                    ValueKind::Boolean,
                )],
                state: Some(StateSnapshot {
                    entity_id: motion_id.clone(),
                    value: Value::Bool(motion),
                    source: StateSource::Poll,
                    observed_at_ms,
                    received_at_ms: observed_at_ms,
                    expires_at_ms: None,
                    confidence: StateConfidence::Confirmed,
                }),
                metadata: vec![Metadata::new(
                    "reolink.channel",
                    channel.channel.to_string(),
                )],
            })?;
            installed.motion_entity_ids.push(motion_id);
        }
        installed.device_ids.push(device_id);
        installed.camera_entity_ids.push(camera_id);
    }
    Ok(installed)
}

fn parse_command_response(command: &'static str, bytes: &[u8]) -> Result<JsonValue, ReolinkError> {
    let response: JsonValue = serde_json::from_slice(bytes)?;
    let item =
        response
            .as_array()
            .and_then(|items| items.first())
            .ok_or(ReolinkError::MissingField {
                command,
                field: "response[0]",
            })?;
    if item.get("cmd").and_then(JsonValue::as_str) != Some(command) {
        return Err(ReolinkError::Validation(format!(
            "response command does not match {command}"
        )));
    }
    let code = item.get("code").and_then(JsonValue::as_i64).unwrap_or(-1);
    if code != 0 {
        let message = item
            .pointer("/error/detail")
            .or_else(|| item.pointer("/error/rspCode"))
            .and_then(|value| {
                value
                    .as_str()
                    .map(str::to_string)
                    .or_else(|| Some(value.to_string()))
            })
            .unwrap_or_else(|| "device rejected command".to_string());
        return Err(ReolinkError::Api {
            command: command.to_string(),
            code,
            message,
        });
    }
    item.get("value")
        .cloned()
        .ok_or(ReolinkError::MissingField {
            command,
            field: "value",
        })
}

fn parse_device_information(value: &JsonValue) -> Result<ReolinkDeviceInformation, ReolinkError> {
    let info = value.get("DevInfo").unwrap_or(value);
    let model = required_text(info, "GetDevInfo", "model")?;
    let serial = required_text(info, "GetDevInfo", "serial")?;
    Ok(ReolinkDeviceInformation {
        name: text(info, "name").unwrap_or_else(|| model.clone()),
        model,
        serial,
        firmware_version: text(info, "firmVer"),
        hardware_version: text(info, "hardVer"),
    })
}

fn parse_channels(value: &JsonValue) -> Result<Vec<ReolinkChannelStatus>, ReolinkError> {
    let statuses =
        value
            .get("status")
            .and_then(JsonValue::as_array)
            .ok_or(ReolinkError::MissingField {
                command: "GetChannelstatus",
                field: "value.status",
            })?;
    statuses
        .iter()
        .map(|status| {
            let channel = status
                .get("channel")
                .and_then(JsonValue::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .ok_or(ReolinkError::MissingField {
                    command: "GetChannelstatus",
                    field: "value.status[].channel",
                })?;
            Ok(ReolinkChannelStatus {
                channel,
                name: text(status, "name").unwrap_or_else(|| format!("Camera {}", channel + 1)),
                online: boolean(status.get("online")).unwrap_or(false),
                sleeping: boolean(status.get("sleep")).unwrap_or(false),
                motion: None,
                recording_enabled: None,
                ptz_presets: None,
            })
        })
        .collect()
}

fn parse_motion(value: &JsonValue) -> Option<bool> {
    boolean(value.get("state"))
}

fn parse_recording_enabled(value: &JsonValue) -> Result<bool, ReolinkError> {
    boolean(value.pointer("/Rec/enable")).ok_or(ReolinkError::MissingField {
        command: "GetRecV20",
        field: "value.Rec.enable",
    })
}

fn parse_ptz_presets(value: &JsonValue) -> Result<Vec<ReolinkPtzPreset>, ReolinkError> {
    let presets =
        value
            .get("PtzPreset")
            .and_then(JsonValue::as_array)
            .ok_or(ReolinkError::MissingField {
                command: "GetPtzPreset",
                field: "value.PtzPreset",
            })?;
    presets
        .iter()
        .filter(|preset| boolean(preset.get("enable")).unwrap_or(false))
        .map(|preset| {
            let id = preset
                .get("id")
                .and_then(JsonValue::as_u64)
                .and_then(|id| u32::try_from(id).ok())
                .ok_or(ReolinkError::MissingField {
                    command: "GetPtzPreset",
                    field: "value.PtzPreset[].id",
                })?;
            Ok(ReolinkPtzPreset {
                id,
                name: text(preset, "name").unwrap_or_else(|| format!("Preset {id}")),
            })
        })
        .collect()
}

fn boolean(value: Option<&JsonValue>) -> Option<bool> {
    value.and_then(|value| {
        value
            .as_bool()
            .or_else(|| value.as_i64().map(|number| number != 0))
    })
}

fn text(value: &JsonValue, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

fn required_text(
    value: &JsonValue,
    command: &'static str,
    field: &'static str,
) -> Result<String, ReolinkError> {
    text(value, field)
        .filter(|value| !value.trim().is_empty())
        .ok_or(ReolinkError::MissingField { command, field })
}

fn channel_state(channel: &ReolinkChannelStatus) -> Value {
    let mut fields = vec![
        ("online".to_string(), Value::Bool(channel.online)),
        ("sleeping".to_string(), Value::Bool(channel.sleeping)),
    ];
    if let Some(motion) = channel.motion {
        fields.push(("motion".to_string(), Value::Bool(motion)));
    }
    if let Some(recording_enabled) = channel.recording_enabled {
        fields.push((
            "recording_enabled".to_string(),
            Value::Bool(recording_enabled),
        ));
    }
    if let Some(presets) = &channel.ptz_presets {
        fields.push((
            "ptz_presets".to_string(),
            Value::Array(
                presets
                    .iter()
                    .map(|preset| {
                        Value::Object(vec![
                            ("id".to_string(), Value::Integer(i64::from(preset.id))),
                            ("name".to_string(), Value::Text(preset.name.clone())),
                        ])
                    })
                    .collect(),
            ),
        ));
    }
    Value::Object(fields)
}

fn ptz_preset_arguments(request: &RuntimeCommandToolRequest) -> Result<(u32, u32), ReolinkError> {
    let preset_id = object_u32(&request.arguments, "preset_id");
    let speed = object_u32(&request.arguments, "speed");
    match (preset_id, speed) {
        (Some(preset_id), Some(speed)) if (MIN_PTZ_SPEED..=MAX_PTZ_SPEED).contains(&speed) => {
            Ok((preset_id, speed))
        }
        _ => invalid_command_arguments(
            request.command_type,
            "an object with a probed preset_id and speed from 1 through 64",
        ),
    }
}

fn ptz_move_arguments(
    request: &RuntimeCommandToolRequest,
) -> Result<(ReolinkPtzDirection, u32, u64), ReolinkError> {
    let direction =
        object_text(&request.arguments, "direction").and_then(ReolinkPtzDirection::from_label);
    let speed = object_u32(&request.arguments, "speed");
    let duration_ms = object_u64(&request.arguments, "duration_ms");
    match (direction, speed, duration_ms) {
        (Some(direction), Some(speed), Some(duration_ms))
            if (MIN_PTZ_SPEED..=MAX_PTZ_SPEED).contains(&speed)
                && (1..=MAX_PTZ_DURATION_MS).contains(&duration_ms) =>
        {
            Ok((direction, speed, duration_ms))
        }
        _ => invalid_command_arguments(
            request.command_type,
            "an object with direction left/right/up/down, speed 1 through 64, and duration_ms 1 through 5000",
        ),
    }
}

fn object_field<'a>(value: &'a Value, field: &str) -> Option<&'a Value> {
    let Value::Object(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find_map(|(name, value)| (name == field).then_some(value))
}

fn object_u32(value: &Value, field: &str) -> Option<u32> {
    let Value::Integer(value) = object_field(value, field)? else {
        return None;
    };
    u32::try_from(*value).ok()
}

fn object_u64(value: &Value, field: &str) -> Option<u64> {
    let Value::Integer(value) = object_field(value, field)? else {
        return None;
    };
    u64::try_from(*value).ok()
}

fn object_text<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    let Value::Text(value) = object_field(value, field)? else {
        return None;
    };
    Some(value.as_str())
}

fn invalid_command_arguments<T>(
    command_type: CommandType,
    expected: &'static str,
) -> Result<T, ReolinkError> {
    Err(ReolinkError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn camera_entity_id(serial: &str, channel: u32) -> EntityId {
    EntityId::trusted(format!(
        "reolink:{}:ch{channel}:camera",
        stable_component(serial)
    ))
}

fn confirmed_recording_state(
    entity_id: EntityId,
    previous: Option<StateSnapshot>,
    enabled: bool,
    observed_at_ms: u64,
) -> StateSnapshot {
    let mut fields = match previous.map(|state| state.value) {
        Some(Value::Object(fields)) => fields,
        _ => Vec::new(),
    };
    if let Some((_, value)) = fields
        .iter_mut()
        .find(|(field, _)| field == "recording_enabled")
    {
        *value = Value::Bool(enabled);
    } else {
        fields.push(("recording_enabled".to_string(), Value::Bool(enabled)));
    }
    StateSnapshot {
        entity_id,
        value: Value::Object(fields),
        source: StateSource::Poll,
        observed_at_ms,
        received_at_ms: observed_at_ms,
        expires_at_ms: None,
        confidence: StateConfidence::Confirmed,
    }
}

fn api_endpoint(
    base_url: &str,
    command: &str,
    session: Option<&ReolinkSession>,
) -> Result<String, ReolinkError> {
    if !command.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Err(ReolinkError::Validation(
            "command contains unsafe URL characters".to_string(),
        ));
    }
    let mut endpoint = format!("{base_url}/api.cgi?cmd={command}");
    if let Some(session) = session {
        endpoint.push_str("&token=");
        endpoint.push_str(&session.token);
    }
    Ok(endpoint)
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, ReolinkError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| ReolinkError::Validation(error.to_string()))
}

fn stable_component(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn encode_http_request(url: &Url, body: &[u8]) -> Result<Vec<u8>, ReolinkError> {
    let host = url
        .host
        .as_deref()
        .ok_or_else(|| ReolinkError::Validation("endpoint is missing a host".to_string()))?;
    let target = match &url.query {
        Some(query) => format!("{}?{query}", url.path),
        None => url.path.clone(),
    };
    let host_header = match url.port {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    let mut request = format!(
        "POST {target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\nAccept: application/json\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )
    .into_bytes();
    request.extend_from_slice(body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, ReolinkError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| ReolinkError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| ReolinkError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| ReolinkError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(ReolinkError::Io(
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

fn write_request(writer: &mut dyn Write, request: &[u8]) -> Result<(), ReolinkError> {
    writer
        .write_all(request)
        .map_err(|error| ReolinkError::Io(error.to_string()))?;
    writer
        .flush()
        .map_err(|error| ReolinkError::Io(error.to_string()))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, ReolinkError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| ReolinkError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(ReolinkError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, ReolinkError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| ReolinkError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(ReolinkError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(ReolinkError::TruncatedBody {
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
        return Err(ReolinkError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, ReolinkError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let line_offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| ReolinkError::Http("missing chunk-size terminator".to_string()))?;
        let line_end = cursor + line_offset;
        let size_text = std::str::from_utf8(&input[cursor..line_end])
            .map_err(|_| ReolinkError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| ReolinkError::Http("invalid chunk size".to_string()))?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(ReolinkError::ResponseTooLarge { limit: maximum });
        }
        let end = cursor
            .checked_add(size)
            .ok_or_else(|| ReolinkError::Http("chunk size overflow".to_string()))?;
        if end + 2 > input.len() || &input[end..end + 2] != b"\r\n" {
            return Err(ReolinkError::Http("truncated chunk payload".to_string()));
        }
        output.extend_from_slice(&input[cursor..end]);
        cursor = end + 2;
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

    fn response(value: JsonValue) -> Vec<u8> {
        serde_json::to_vec(&value).unwrap()
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ =
            runtime
                .registry_mut()
                .upsert_capability_grant(CapabilityGrant::for_all_smart_home(
                    CapabilityGrantId::trusted("grant:reolink-test"),
                    principal.clone(),
                    PrivilegeTier::HumanApproval,
                    "test",
                    1,
                ));
    }

    #[test]
    fn credentials_and_session_debug_are_redacted() {
        let credentials = ReolinkCredentials::new("admin", "secret").unwrap();
        assert_eq!(format!("{credentials:?}"), "ReolinkCredentials([REDACTED])");
        let session = ReolinkSession {
            token: Zeroizing::new("session-secret".to_string()),
            lease_seconds: 3_600,
        };
        let rendered = format!("{session:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains("session-secret"));
    }

    #[test]
    fn api_failures_are_not_accepted_as_state() {
        let error = parse_command_response(
            "Login",
            br#"[{"cmd":"Login","code":1,"error":{"rspCode":-6,"detail":"login failed"}}]"#,
        )
        .unwrap_err();
        assert!(matches!(error, ReolinkError::Api { code: 1, .. }));
    }

    #[test]
    fn real_http_inspection_recording_and_bounded_ptz_controls_are_verified() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(Vec::new()));
        let server_captured = Arc::clone(&captured);
        let handle = thread::spawn(move || {
            let responses = [
                json!([{"cmd":"Login","code":0,"value":{"Token":{"name":"token123","leaseTime":3600}}}]),
                json!([{"cmd":"GetDevInfo","code":0,"value":{"DevInfo":{"name":"Front NVR","model":"RLN8-410","serial":"ABC123","firmVer":"v3.5.0","hardVer":"N7MB01"}}}]),
                json!([{"cmd":"GetChannelstatus","code":0,"value":{"status":[{"channel":0,"name":"Porch","online":1,"sleep":0},{"channel":1,"name":"Garage","online":0,"sleep":0}]}}]),
                json!([{"cmd":"GetMdState","code":0,"value":{"state":1}}]),
                json!([{"cmd":"GetRecV20","code":0,"value":{"Rec":{"enable":0}}}]),
                json!([{"cmd":"GetPtzPreset","code":0,"value":{"PtzPreset":[{"enable":1,"id":2,"name":"Driveway"},{"enable":0,"id":3,"name":"Disabled"}]}}]),
                json!([{"cmd":"Logout","code":0,"value":{}}]),
                json!([{"cmd":"Login","code":0,"value":{"Token":{"name":"token456","leaseTime":3600}}}]),
                json!([{"cmd":"SetRecV20","code":0,"value":{"rspCode":200}}]),
                json!([{"cmd":"GetRecV20","code":0,"value":{"Rec":{"enable":1}}}]),
                json!([{"cmd":"Logout","code":0,"value":{}}]),
                json!([{"cmd":"Login","code":0,"value":{"Token":{"name":"token789","leaseTime":3600}}}]),
                json!([{"cmd":"PtzCtrl","code":0,"value":{"rspCode":200}}]),
                json!([{"cmd":"Logout","code":0,"value":{}}]),
                json!([{"cmd":"Login","code":0,"value":{"Token":{"name":"tokenmove","leaseTime":3600}}}]),
                json!([{"cmd":"PtzCtrl","code":0,"value":{"rspCode":200}}]),
                json!([{"cmd":"PtzCtrl","code":0,"value":{"rspCode":200}}]),
                json!([{"cmd":"Logout","code":0,"value":{}}]),
            ];
            for response_value in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0u8; 4096];
                loop {
                    let read = stream.read(&mut buffer).unwrap();
                    bytes.extend_from_slice(&buffer[..read]);
                    if let Some(offset) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
                    {
                        let head = String::from_utf8_lossy(&bytes[..offset + 4]);
                        let length = head
                            .lines()
                            .find_map(|line| line.strip_prefix("Content-Length: "))
                            .and_then(|value| value.parse::<usize>().ok())
                            .unwrap();
                        if bytes.len() >= offset + 4 + length {
                            break;
                        }
                    }
                }
                server_captured.lock().unwrap().push(bytes);
                let body = response(response_value);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .unwrap();
                stream.write_all(&body).unwrap();
            }
        });

        let config = ReolinkConfig::new(
            BridgeId::trusted("reolink.test"),
            format!("http://{address}"),
            VaultRef::trusted("vault://reolink/test"),
        )
        .unwrap();
        let transport = ReolinkLanTransport::default().with_timeout(Duration::from_secs(1));
        let client = ReolinkClient::new(
            config,
            ReolinkCredentials::new("admin", "secret").unwrap(),
            transport,
        );
        let mut integration = ReolinkRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:reolink-test");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 5_000)
            .unwrap();
        let request = RuntimeCommandToolRequest::new(
            installed.recording_entity_ids[0].clone(),
            CommandType::DeviceControl(DeviceControlCommandType::SetCameraRecording),
            Value::Bool(true),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                AgentId::trusted("agent:reolink-denied"),
                request.clone(),
                5_500,
            ),
            Err(ReolinkError::Runtime(_))
        ));
        let denied_move = RuntimeCommandToolRequest::new(
            installed.ptz_entity_ids[0].clone(),
            CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz),
            Value::Object(vec![
                ("direction".to_string(), Value::Text("right".to_string())),
                ("speed".to_string(), Value::Integer(16)),
                ("duration_ms".to_string(), Value::Integer(1)),
            ]),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                AgentId::trusted("agent:reolink-denied"),
                denied_move,
                5_600,
            ),
            Err(ReolinkError::Runtime(_))
        ));
        let invalid_move = RuntimeCommandToolRequest::new(
            installed.ptz_entity_ids[0].clone(),
            CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz),
            Value::Object(vec![
                ("direction".to_string(), Value::Text("left".to_string())),
                ("speed".to_string(), Value::Integer(16)),
                ("duration_ms".to_string(), Value::Integer(5_001)),
            ]),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                principal.clone(),
                invalid_move,
                5_750,
            ),
            Err(ReolinkError::InvalidCommandArguments { .. })
        ));
        let unknown_preset = RuntimeCommandToolRequest::new(
            installed.ptz_entity_ids[0].clone(),
            CommandType::DeviceControl(DeviceControlCommandType::RecallCameraPtzPreset),
            Value::Object(vec![
                ("preset_id".to_string(), Value::Integer(99)),
                ("speed".to_string(), Value::Integer(32)),
            ]),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                principal.clone(),
                unknown_preset,
                5_800,
            ),
            Err(ReolinkError::InvalidCommandArguments { .. })
        ));
        let recording_result = integration
            .dispatch_command_authorized(&mut runtime, principal.clone(), request, 6_000)
            .unwrap();
        let preset_result = integration
            .dispatch_command_authorized(
                &mut runtime,
                principal.clone(),
                RuntimeCommandToolRequest::new(
                    installed.ptz_entity_ids[0].clone(),
                    CommandType::DeviceControl(DeviceControlCommandType::RecallCameraPtzPreset),
                    Value::Object(vec![
                        ("preset_id".to_string(), Value::Integer(2)),
                        ("speed".to_string(), Value::Integer(32)),
                    ]),
                ),
                6_500,
            )
            .unwrap();
        let move_result = integration
            .dispatch_command_authorized(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.ptz_entity_ids[0].clone(),
                    CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz),
                    Value::Object(vec![
                        ("direction".to_string(), Value::Text("left".to_string())),
                        ("speed".to_string(), Value::Integer(16)),
                        ("duration_ms".to_string(), Value::Integer(1)),
                    ]),
                ),
                7_000,
            )
            .unwrap();
        handle.join().unwrap();

        assert_eq!(installed.device_ids.len(), 2);
        assert_eq!(installed.camera_entity_ids.len(), 2);
        assert_eq!(installed.recording_entity_ids.len(), 1);
        assert_eq!(installed.ptz_entity_ids.len(), 1);
        assert_eq!(installed.motion_entity_ids.len(), 1);
        assert!(recording_result
            .message
            .as_deref()
            .is_some_and(|message| message.contains("recording enabled")));
        assert!(preset_result
            .message
            .as_deref()
            .is_some_and(|message| message.contains("preset 2")));
        assert!(move_result
            .message
            .as_deref()
            .is_some_and(|message| message.contains("bounded PTZ left")));
        assert_eq!(
            runtime
                .registry()
                .state(&installed.motion_entity_ids[0])
                .unwrap()
                .value,
            Value::Bool(true)
        );
        let camera_state = &runtime
            .registry()
            .state(&installed.recording_entity_ids[0])
            .unwrap()
            .value;
        assert!(matches!(
            camera_state,
            Value::Object(fields)
                if fields.iter().any(|(field, value)|
                    field == "recording_enabled" && *value == Value::Bool(true))
        ));
        assert!(matches!(
            camera_state,
            Value::Object(fields)
                if fields.iter().any(|(field, value)|
                    field == "ptz_presets"
                        && matches!(value, Value::Array(presets) if presets.len() == 1))
        ));
        let bridge = runtime.registry().bridge(&installed.bridge_id).unwrap();
        assert_eq!(
            bridge.auth_ref.as_ref().unwrap().as_str(),
            "vault://reolink/test"
        );
        let serialized_runtime = format!("{runtime:?}");
        assert!(!serialized_runtime.contains("secret"));
        assert!(!serialized_runtime.contains("token123"));

        let requests = captured.lock().unwrap();
        assert_eq!(requests.len(), 18);
        let request_text = requests
            .iter()
            .map(|request| String::from_utf8_lossy(request).to_string())
            .collect::<Vec<_>>();
        assert!(request_text[0].contains("cmd=Login"));
        assert!(request_text[0].contains("\"password\":\"secret\""));
        assert!(request_text[1].contains("cmd=GetDevInfo&token=token123"));
        assert!(request_text[3].contains("cmd=GetMdState&token=token123"));
        assert!(request_text[4].contains("cmd=GetRecV20&token=token123"));
        assert!(request_text[4].contains("\"channel\":0"));
        assert!(request_text[5].contains("cmd=GetPtzPreset&token=token123"));
        assert!(request_text[6].contains("cmd=Logout&token=token123"));
        assert!(request_text[8].contains("cmd=SetRecV20&token=token456"));
        assert!(request_text[8].contains("\"Rec\":{\"channel\":0,\"enable\":1}"));
        assert!(request_text[9].contains("cmd=GetRecV20&token=token456"));
        assert!(request_text[10].contains("cmd=Logout&token=token456"));
        assert!(request_text[12].contains("cmd=PtzCtrl&token=token789"));
        assert!(request_text[12].contains("\"channel\":0,\"id\":2,\"op\":\"ToPos\",\"speed\":32"));
        assert!(request_text[13].contains("cmd=Logout&token=token789"));
        assert!(request_text[15].contains("cmd=PtzCtrl&token=tokenmove"));
        assert!(request_text[15].contains("\"channel\":0,\"op\":\"Left\",\"speed\":16"));
        assert!(request_text[16].contains("cmd=PtzCtrl&token=tokenmove"));
        assert!(request_text[16].contains("\"channel\":0,\"op\":\"Stop\",\"speed\":16"));
        assert!(request_text[17].contains("cmd=Logout&token=tokenmove"));
    }

    #[derive(Debug)]
    struct CountingTransport {
        calls: Arc<AtomicUsize>,
    }

    impl ReolinkTransport for CountingTransport {
        fn post_json(&mut self, _endpoint: &str, _body: &[u8]) -> Result<Vec<u8>, ReolinkError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(Vec::new())
        }
    }

    #[test]
    fn denied_read_reaches_no_transport_or_credentials() {
        let calls = Arc::new(AtomicUsize::new(0));
        let config = ReolinkConfig::new(
            BridgeId::trusted("reolink.denied"),
            "http://127.0.0.1",
            VaultRef::trusted("vault://reolink/denied"),
        )
        .unwrap();
        let client = ReolinkClient::new(
            config,
            ReolinkCredentials::new("admin", "secret").unwrap(),
            CountingTransport {
                calls: Arc::clone(&calls),
            },
        );
        let mut integration = ReolinkRuntimeIntegration::new(client);
        let error = integration
            .inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            )
            .unwrap_err();
        assert!(matches!(error, ReolinkError::Runtime(_)));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn authenticated_snapshot_becomes_paired_manual_discovery() {
        let config = ReolinkConfig::new(
            BridgeId::trusted("reolink.discovery"),
            "https://192.0.2.10",
            VaultRef::trusted("vault://reolink/discovery"),
        )
        .unwrap();
        let snapshot = ReolinkSnapshot {
            device: ReolinkDeviceInformation {
                name: "NVR".to_string(),
                model: "RLN8-410".to_string(),
                serial: "ABC123".to_string(),
                firmware_version: Some("v3".to_string()),
                hardware_version: None,
            },
            channels: vec![ReolinkChannelStatus {
                channel: 0,
                name: "Porch".to_string(),
                online: true,
                sleeping: false,
                motion: Some(false),
                recording_enabled: Some(true),
                ptz_presets: None,
            }],
        };
        let record = discovery_record(&config, &snapshot, 9_000).unwrap();
        assert_eq!(record.source, DiscoverySource::Manual);
        assert_eq!(record.confidence, DiscoveryConfidence::Paired);
        assert_eq!(record.pairing_requirement, PairingRequirement::Credentials);
        assert_eq!(record.native_bridge_id, "abc123");
    }
}
