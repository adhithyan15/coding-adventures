//! Authenticated Blue Iris NVR inspection for D23.

#![forbid(unsafe_code)]

use coding_adventures_md5::sum_md5;
use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandType, Device, DeviceControlCommandType, DeviceId, Entity, EntityId,
    EntityKind, Health, IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, SmartHomeTool,
    StateConfidence, StateSnapshot, StateSource, Value, ValueKind, VaultRef,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_local_http::{
    LocalHttpAuth, LocalHttpEndpoint, LocalHttpError, LocalHttpMethod, LocalHttpRequestPlan,
    LocalHttpRequestTemplate, LocalHttpScheme,
};
use smart_home_runtime::{RuntimeCommandToolRequest, RuntimeError, SmartHomeRuntime};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig, TlsConnector};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.2.0";
pub const INTEGRATION_ID: &str = "blue_iris";
pub const PROTOCOL_ID: &str = "blue_iris_json";
pub const JSON_PATH: &str = "/json";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MIN_PTZ_SPEED: u32 = 1;
pub const MAX_PTZ_SPEED: u32 = 100;
pub const MAX_PTZ_DURATION_MS: u64 = 5_000;
pub const MIN_PTZ_PRESET: u32 = 1;
pub const MAX_PTZ_PRESET: u32 = 20;
const MAX_SESSION_BYTES: usize = 512;
const RECORDING_READBACK_ATTEMPTS: usize = 3;
const RECORDING_READBACK_DELAY: Duration = Duration::from_millis(50);

#[derive(Debug)]
pub enum BlueIrisError {
    Validation(String),
    LocalHttp(LocalHttpError),
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
    Json(serde_json::Error),
    Api {
        command: &'static str,
        reason: String,
    },
    MissingField(&'static str),
    UnknownEntity(EntityId),
    UnsupportedCommand(CommandType),
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    PermissionDenied(&'static str),
    VerificationFailed(String),
    Runtime(RuntimeError),
}

impl fmt::Display for BlueIrisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Blue Iris input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Blue Iris URL: {error}"),
            Self::Io(message) => write!(formatter, "Blue Iris LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "Blue Iris TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Blue Iris HTTP response: {message}"),
            Self::HttpStatus(status) => {
                write!(formatter, "Blue Iris endpoint returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Blue Iris response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Blue Iris response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Blue Iris JSON: {error}"),
            Self::Api { command, reason } => {
                write!(formatter, "Blue Iris {command} failed: {reason}")
            }
            Self::MissingField(field) => write!(formatter, "Blue Iris response is missing {field}"),
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown Blue Iris entity {}", entity_id.as_str())
            }
            Self::UnsupportedCommand(command_type) => {
                write!(formatter, "unsupported Blue Iris command {command_type:?}")
            }
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "invalid arguments for Blue Iris command {command_type:?}; expected {expected}"
            ),
            Self::PermissionDenied(permission) => {
                write!(
                    formatter,
                    "Blue Iris session does not grant {permission} permission"
                )
            }
            Self::VerificationFailed(message) => {
                write!(
                    formatter,
                    "Blue Iris postcondition was not verified: {message}"
                )
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BlueIrisError {}

impl From<LocalHttpError> for BlueIrisError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for BlueIrisError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for BlueIrisError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for BlueIrisError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct BlueIrisCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl BlueIrisCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, BlueIrisError> {
        let username = username.into();
        let password = password.into();
        if username.trim().is_empty() || password.is_empty() {
            return Err(BlueIrisError::Validation(
                "username and password must not be empty".to_string(),
            ));
        }
        if username.contains('\0') || password.contains('\0') {
            return Err(BlueIrisError::Validation(
                "credentials contain a NUL byte".to_string(),
            ));
        }
        Ok(Self {
            username: Zeroizing::new(username),
            password: Zeroizing::new(password),
        })
    }
}

impl fmt::Debug for BlueIrisCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BlueIrisCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueIrisConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub credential_ref: VaultRef,
    pub timeout: Duration,
}

impl BlueIrisConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        credential_ref: VaultRef,
    ) -> Result<Self, BlueIrisError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(BlueIrisError::MissingField("base URL host"))?;
        let secure = parsed.scheme == "https";
        let test_loopback = parsed.scheme == "http" && is_loopback_host(host);
        if (!secure && !test_loopback)
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(BlueIrisError::Validation(
                "base URL must be a credential-free HTTPS origin; HTTP is test-only on loopback"
                    .to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            credential_ref,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn endpoint(&self) -> Result<LocalHttpEndpoint, BlueIrisError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(BlueIrisError::MissingField("base URL host"))?;
        let scheme = match parsed.scheme.as_str() {
            "https" => LocalHttpScheme::Https,
            "http" if is_loopback_host(host) => LocalHttpScheme::Http,
            _ => {
                return Err(BlueIrisError::Validation(
                    "Blue Iris endpoint is not approved".to_string(),
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
        .with_metadata(Metadata::new("http.profile", "blue-iris.json")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueIrisServer {
    pub name: String,
    pub version: String,
    pub admin: bool,
    pub ptz_allowed: bool,
    pub clip_create_allowed: bool,
    pub timezone_minutes: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueIrisCamera {
    pub short_name: String,
    pub name: String,
    pub number: Option<i64>,
    pub enabled: bool,
    pub online: bool,
    pub no_signal: bool,
    pub paused: bool,
    pub motion: bool,
    pub triggered: bool,
    pub alerting: bool,
    pub recording: bool,
    pub manual_recording: bool,
    pub ptz: bool,
    pub audio: bool,
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueIrisSnapshot {
    pub server: BlueIrisServer,
    pub cameras: Vec<BlueIrisCamera>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlueIrisPtzDirection {
    Left,
    Right,
    Up,
    Down,
}

impl BlueIrisPtzDirection {
    fn from_label(label: &str) -> Option<Self> {
        match label {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Up => "up",
            Self::Down => "down",
        }
    }

    fn joystick(self, speed: u32) -> u32 {
        let native_speed = speed.saturating_mul(15).div_ceil(100).clamp(1, 15);
        match self {
            Self::Left => (1 << 10) | (native_speed << 4),
            Self::Right => (1 << 9) | (native_speed << 4),
            Self::Up => (1 << 11) | native_speed,
            Self::Down => (1 << 12) | native_speed,
        }
    }
}

pub trait BlueIrisTransport {
    fn inspect(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
    ) -> Result<BlueIrisSnapshot, BlueIrisError>;

    fn set_manual_recording(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
        camera: &str,
        enabled: bool,
    ) -> Result<BlueIrisCamera, BlueIrisError>;

    fn recall_ptz_preset(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
        camera: &str,
        preset: u32,
    ) -> Result<(), BlueIrisError>;

    fn move_ptz_bounded(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
        camera: &str,
        direction: BlueIrisPtzDirection,
        speed: u32,
        duration_ms: u64,
    ) -> Result<(), BlueIrisError>;
}

pub struct BlueIrisLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    maximum_response_bytes: usize,
}

impl Default for BlueIrisLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl BlueIrisLanTransport {
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

    fn post_json(
        &mut self,
        plan: &LocalHttpRequestPlan,
        body: &JsonValue,
    ) -> Result<JsonValue, BlueIrisError> {
        let bytes = Zeroizing::new(serde_json::to_vec(body)?);
        let request = Zeroizing::new(encode_http_request(plan, bytes.as_slice())?);
        let url = Url::parse(&plan.url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(BlueIrisError::MissingField("request URL host"))?;
        let port = url
            .effective_port()
            .ok_or(BlueIrisError::MissingField("request URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let response = match url.scheme.as_str() {
            "http" if is_loopback_host(host) => {
                let mut stream = connect_tcp(host, port, timeout)?;
                write_request(&mut stream, request.as_slice())?;
                read_bounded(&mut stream, self.maximum_response_bytes)?
            }
            "https" => {
                let mut config = self.tls_config.clone();
                config.connect_timeout = timeout;
                config.read_timeout = Some(timeout);
                config.write_timeout = Some(timeout);
                let mut stream = self
                    .connector
                    .connect(host, port, &config)
                    .map_err(|error| BlueIrisError::Tls(error.to_string()))?;
                write_request(&mut stream, request.as_slice())?;
                let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
                stream
                    .close_notify()
                    .map_err(|error| BlueIrisError::Tls(error.to_string()))?;
                bytes
            }
            _ => {
                return Err(BlueIrisError::Validation(
                    "Blue Iris transport requires HTTPS or loopback HTTP".to_string(),
                ))
            }
        };
        Ok(serde_json::from_slice(&decode_http_response(
            &response,
            self.maximum_response_bytes,
        )?)?)
    }

    fn login(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
    ) -> Result<(Zeroizing<String>, BlueIrisServer), BlueIrisError> {
        let challenge = self.post_json(plan, &json!({"cmd":"login"}))?;
        let session = parse_login_challenge(&challenge)?;
        let response = challenge_response(credentials, session.as_str());
        let login = self.post_json(
            plan,
            &json!({
                "cmd":"login",
                "session":session.as_str(),
                "response":response.as_str(),
            }),
        )?;
        Ok((session, parse_login_success(&login)?))
    }

    fn authenticated<R>(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
        operation: impl FnOnce(&mut Self, &str, &BlueIrisServer) -> Result<R, BlueIrisError>,
    ) -> Result<R, BlueIrisError> {
        let (session, server) = self.login(plan, credentials)?;
        let result = operation(self, session.as_str(), &server);
        let logout = self
            .post_json(plan, &json!({"cmd":"logout","session":session.as_str()}))
            .and_then(|value| require_success("logout", &value));
        match (result, logout) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(value), Ok(())) => Ok(value),
        }
    }

    fn camera_list(
        &mut self,
        plan: &LocalHttpRequestPlan,
        session: &str,
    ) -> Result<Vec<BlueIrisCamera>, BlueIrisError> {
        let cameras = self.post_json(plan, &json!({"cmd":"camlist","session":session}))?;
        parse_camera_list(&cameras)
    }

    fn stop_ptz(
        &mut self,
        plan: &LocalHttpRequestPlan,
        session: &str,
        camera: &str,
    ) -> Result<(), BlueIrisError> {
        let stop = self.post_json(
            plan,
            &json!({"cmd":"ptz","session":session,"camera":camera,"button":64,"updown":0}),
        )?;
        require_success("ptz stop", &stop)
    }
}

impl BlueIrisTransport for BlueIrisLanTransport {
    fn inspect(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
    ) -> Result<BlueIrisSnapshot, BlueIrisError> {
        self.authenticated(plan, credentials, |transport, session, server| {
            Ok(BlueIrisSnapshot {
                server: server.clone(),
                cameras: transport.camera_list(plan, session)?,
            })
        })
    }

    fn set_manual_recording(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
        camera: &str,
        enabled: bool,
    ) -> Result<BlueIrisCamera, BlueIrisError> {
        self.authenticated(plan, credentials, |transport, session, server| {
            if !server.clip_create_allowed {
                return Err(BlueIrisError::PermissionDenied("clip-create"));
            }
            let response = transport.post_json(
                plan,
                &json!({"cmd":"camconfig","session":session,"camera":camera,"manrec":enabled}),
            )?;
            require_success("camconfig", &response)?;
            for attempt in 0..RECORDING_READBACK_ATTEMPTS {
                let observed = transport
                    .camera_list(plan, session)?
                    .into_iter()
                    .find(|candidate| candidate.short_name == camera)
                    .ok_or_else(|| {
                        BlueIrisError::VerificationFailed(format!(
                            "camera `{camera}` disappeared from camlist"
                        ))
                    })?;
                if observed.manual_recording == enabled {
                    return Ok(observed);
                }
                if attempt + 1 < RECORDING_READBACK_ATTEMPTS {
                    thread::sleep(RECORDING_READBACK_DELAY);
                }
            }
            Err(BlueIrisError::VerificationFailed(format!(
                "camera `{camera}` manual recording did not become {enabled}"
            )))
        })
    }

    fn recall_ptz_preset(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
        camera: &str,
        preset: u32,
    ) -> Result<(), BlueIrisError> {
        self.authenticated(plan, credentials, |transport, session, server| {
            if !server.ptz_allowed {
                return Err(BlueIrisError::PermissionDenied("PTZ"));
            }
            let response = transport.post_json(
                plan,
                &json!({"cmd":"ptz","session":session,"camera":camera,"button":100 + preset}),
            )?;
            require_success("ptz", &response)
        })
    }

    fn move_ptz_bounded(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
        camera: &str,
        direction: BlueIrisPtzDirection,
        speed: u32,
        duration_ms: u64,
    ) -> Result<(), BlueIrisError> {
        self.authenticated(plan, credentials, |transport, session, server| {
            if !server.ptz_allowed {
                return Err(BlueIrisError::PermissionDenied("PTZ"));
            }
            let start = transport.post_json(
                plan,
                &json!({
                    "cmd":"ptz",
                    "session":session,
                    "camera":camera,
                    "joystick":direction.joystick(speed),
                    "updown":1,
                }),
            );
            if let Err(error) = start.and_then(|value| require_success("ptz", &value)) {
                let _ = transport.stop_ptz(plan, session, camera);
                return Err(error);
            }
            thread::sleep(Duration::from_millis(duration_ms));
            transport.stop_ptz(plan, session, camera)
        })
    }
}

pub struct BlueIrisClient<T> {
    config: BlueIrisConfig,
    credentials: BlueIrisCredentials,
    transport: T,
    plan: LocalHttpRequestPlan,
}

impl<T: BlueIrisTransport> BlueIrisClient<T> {
    pub fn new(
        config: BlueIrisConfig,
        credentials: BlueIrisCredentials,
        transport: T,
    ) -> Result<Self, BlueIrisError> {
        let endpoint = config.endpoint()?;
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Post, JSON_PATH)?
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_timeout_ms(duration_ms(config.timeout))
            .with_idempotent(false)
            .with_auth(LocalHttpAuth::None);
        let plan = template.plan(&endpoint, Vec::new())?;
        Ok(Self {
            config,
            credentials,
            transport,
            plan,
        })
    }

    pub fn inspect(&mut self) -> Result<BlueIrisSnapshot, BlueIrisError> {
        self.transport.inspect(&self.plan, &self.credentials)
    }

    pub fn set_manual_recording_and_verify(
        &mut self,
        camera: &str,
        enabled: bool,
    ) -> Result<BlueIrisCamera, BlueIrisError> {
        self.transport
            .set_manual_recording(&self.plan, &self.credentials, camera, enabled)
    }

    pub fn recall_ptz_preset(&mut self, camera: &str, preset: u32) -> Result<(), BlueIrisError> {
        if !(MIN_PTZ_PRESET..=MAX_PTZ_PRESET).contains(&preset) {
            return Err(BlueIrisError::Validation(format!(
                "PTZ preset must be between {MIN_PTZ_PRESET} and {MAX_PTZ_PRESET}"
            )));
        }
        self.transport
            .recall_ptz_preset(&self.plan, &self.credentials, camera, preset)
    }

    pub fn move_ptz_bounded(
        &mut self,
        camera: &str,
        direction: BlueIrisPtzDirection,
        speed: u32,
        duration_ms: u64,
    ) -> Result<(), BlueIrisError> {
        if !(MIN_PTZ_SPEED..=MAX_PTZ_SPEED).contains(&speed)
            || !(1..=MAX_PTZ_DURATION_MS).contains(&duration_ms)
        {
            return Err(BlueIrisError::Validation(
                "PTZ movement requires speed 1 through 100 and duration_ms 1 through 5000"
                    .to_string(),
            ));
        }
        self.transport.move_ptz_bounded(
            &self.plan,
            &self.credentials,
            camera,
            direction,
            speed,
            duration_ms,
        )
    }
}

impl<T> fmt::Debug for BlueIrisClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BlueIrisClient")
            .field("config", &self.config)
            .field("credentials", &"[REDACTED]")
            .field("plan", &self.plan)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledBlueIrisCamera {
    pub device_id: DeviceId,
    pub camera_entity_id: EntityId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledBlueIrisNvr {
    pub bridge_id: BridgeId,
    pub cameras: Vec<InstalledBlueIrisCamera>,
}

pub struct BlueIrisRuntimeIntegration<T> {
    client: BlueIrisClient<T>,
    command_entities: BTreeMap<EntityId, BlueIrisCommandSupport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BlueIrisCommandSupport {
    camera: String,
    recording: bool,
    ptz: bool,
}

impl<T: BlueIrisTransport> BlueIrisRuntimeIntegration<T> {
    pub fn new(client: BlueIrisClient<T>) -> Self {
        Self {
            client,
            command_entities: BTreeMap::new(),
        }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledBlueIrisNvr, BlueIrisError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        let installed = install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)?;
        self.command_entities = installed
            .cameras
            .iter()
            .zip(snapshot.cameras.iter())
            .map(|(installed, camera)| {
                (
                    installed.camera_entity_id.clone(),
                    BlueIrisCommandSupport {
                        camera: camera.short_name.clone(),
                        recording: snapshot.server.clip_create_allowed,
                        ptz: snapshot.server.ptz_allowed && camera.ptz,
                    },
                )
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
    ) -> Result<CommandResult, BlueIrisError> {
        let support = self
            .command_entities
            .get(&request.entity_id)
            .cloned()
            .ok_or_else(|| BlueIrisError::UnknownEntity(request.entity_id.clone()))?;
        match request.command_type {
            CommandType::DeviceControl(DeviceControlCommandType::SetCameraRecording) => {
                if !support.recording {
                    return Err(BlueIrisError::PermissionDenied("clip-create"));
                }
                let Value::Bool(enabled) = request.arguments else {
                    return invalid_command_arguments(
                        request.command_type,
                        "a manual-recording boolean",
                    );
                };
                let entity_id = request.entity_id.clone();
                let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
                let mut result = runtime.submit_command(command, now_ms)?;
                let observed = self
                    .client
                    .set_manual_recording_and_verify(&support.camera, enabled)?;
                let mut entity = runtime
                    .registry()
                    .entity(&entity_id)
                    .cloned()
                    .ok_or_else(|| BlueIrisError::UnknownEntity(entity_id.clone()))?;
                entity.state = Some(StateSnapshot {
                    entity_id: entity_id.clone(),
                    value: camera_value(&observed),
                    source: StateSource::Poll,
                    observed_at_ms: now_ms,
                    received_at_ms: now_ms,
                    expires_at_ms: None,
                    confidence: StateConfidence::Confirmed,
                });
                runtime.upsert_entity(entity)?;
                result.message = Some(format!(
                    "Blue Iris confirmed camera {} manual recording {}",
                    support.camera,
                    if enabled { "started" } else { "stopped" }
                ));
                Ok(result)
            }
            CommandType::DeviceControl(DeviceControlCommandType::RecallCameraPtzPreset) => {
                if !support.ptz {
                    return Err(BlueIrisError::PermissionDenied("PTZ"));
                }
                let preset = ptz_preset_argument(&request)?;
                let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
                let mut result = runtime.submit_command(command, now_ms)?;
                self.client.recall_ptz_preset(&support.camera, preset)?;
                result.message = Some(format!(
                    "Blue Iris camera {} accepted PTZ preset {preset}",
                    support.camera
                ));
                Ok(result)
            }
            CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz) => {
                if !support.ptz {
                    return Err(BlueIrisError::PermissionDenied("PTZ"));
                }
                let (direction, speed, duration_ms) = ptz_move_arguments(&request)?;
                let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
                let mut result = runtime.submit_command(command, now_ms)?;
                self.client
                    .move_ptz_bounded(&support.camera, direction, speed, duration_ms)?;
                result.message = Some(format!(
                    "Blue Iris camera {} completed bounded PTZ {} movement",
                    support.camera,
                    direction.label()
                ));
                Ok(result)
            }
            command_type => Err(BlueIrisError::UnsupportedCommand(command_type)),
        }
    }
}

pub fn paired_discovery_record(
    config: &BlueIrisConfig,
    snapshot: &BlueIrisSnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, BlueIrisError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&format!("{}-{}", snapshot.server.name, config.base_url)),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| BlueIrisError::Validation(error.to_string()))?
    .with_display_name(snapshot.server.name.clone())
    .with_address(config.base_url.clone())
    .with_hardware_model("Blue Iris NVR")
    .with_firmware_version(snapshot.server.version.clone())
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("blue_iris.protocol", PROTOCOL_ID)
    .with_metadata("blue_iris.camera_count", snapshot.cameras.len().to_string()))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &BlueIrisConfig,
    snapshot: &BlueIrisSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledBlueIrisNvr, BlueIrisError> {
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some("Blue Iris NVR".to_string());
    bridge.firmware_version = Some(snapshot.server.version.clone());
    bridge.auth_ref = Some(config.credential_ref.clone());
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("https_endpoint", &config.base_url)?];
    bridge.metadata = vec![
        Metadata::new("blue_iris.transport", "authenticated_json_session"),
        Metadata::new("blue_iris.system_name", snapshot.server.name.clone()),
        Metadata::new("blue_iris.camera_count", snapshot.cameras.len().to_string()),
    ];
    runtime.upsert_bridge(bridge)?;

    let mut installed = Vec::with_capacity(snapshot.cameras.len());
    for camera in &snapshot.cameras {
        let native_id = stable_component(&camera.short_name);
        if native_id.is_empty() {
            return Err(BlueIrisError::Validation(
                "camera short name has no stable identifier".to_string(),
            ));
        }
        let device_id = DeviceId::trusted(format!("blue-iris:{native_id}"));
        let camera_entity_id = EntityId::trusted(format!("blue-iris:{native_id}:camera"));
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "Blue Iris".to_string(),
            model: "Managed camera".to_string(),
            name: camera.name.clone(),
            serial: None,
            firmware_version: None,
            room_id: None,
            entity_ids: vec![camera_entity_id.clone()],
            identifiers: vec![protocol_identifier(
                "camera_short_name",
                &camera.short_name,
            )?],
            health: camera_health(camera),
            metadata: vec![Metadata::new(
                "blue_iris.camera_short_name",
                camera.short_name.clone(),
            )],
        })?;
        let mut capabilities = vec![Capability::new(
            CapabilityId::trusted("camera.health"),
            CapabilityMode::Observe,
            ValueKind::Object,
        )];
        if snapshot.server.clip_create_allowed {
            capabilities.push(Capability::camera_recording());
        } else {
            capabilities.push(Capability::new(
                CapabilityId::trusted("camera.recording"),
                CapabilityMode::Observe,
                ValueKind::Boolean,
            ));
        }
        if snapshot.server.ptz_allowed && camera.ptz {
            capabilities.push(Capability::camera_ptz());
        }
        runtime.upsert_entity(Entity {
            entity_id: camera_entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Camera,
            name: camera.name.clone(),
            capabilities,
            state: Some(StateSnapshot {
                entity_id: camera_entity_id.clone(),
                value: camera_value(camera),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("blue_iris.protocol", PROTOCOL_ID)],
        })?;
        installed.push(InstalledBlueIrisCamera {
            device_id,
            camera_entity_id,
        });
    }
    Ok(InstalledBlueIrisNvr {
        bridge_id: config.bridge_id.clone(),
        cameras: installed,
    })
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), BlueIrisError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(BlueIrisError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_login_challenge(value: &JsonValue) -> Result<Zeroizing<String>, BlueIrisError> {
    if value.get("result").and_then(JsonValue::as_str) != Some("fail") {
        return Err(api_error("login challenge", value));
    }
    let session = value
        .get("session")
        .and_then(JsonValue::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(BlueIrisError::MissingField("session"))?;
    if session.len() > MAX_SESSION_BYTES {
        return Err(BlueIrisError::Validation(
            "session exceeds the accepted bound".to_string(),
        ));
    }
    Ok(Zeroizing::new(session.to_string()))
}

fn challenge_response(credentials: &BlueIrisCredentials, session: &str) -> Zeroizing<String> {
    let input = Zeroizing::new(format!(
        "{}:{session}:{}",
        credentials.username.as_str(),
        credentials.password.as_str()
    ));
    let digest = sum_md5(input.as_bytes());
    Zeroizing::new(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn parse_login_success(value: &JsonValue) -> Result<BlueIrisServer, BlueIrisError> {
    require_success("login", value)?;
    let data = value
        .get("data")
        .and_then(JsonValue::as_object)
        .ok_or(BlueIrisError::MissingField("data"))?;
    Ok(BlueIrisServer {
        name: required_string_any(
            data,
            &["system name", "systemName", "name"],
            "data.system name",
        )?,
        version: required_string_any(data, &["version"], "data.version")?,
        admin: bool_field(data, "admin"),
        ptz_allowed: bool_field(data, "ptz"),
        clip_create_allowed: bool_field(data, "clipcreate"),
        timezone_minutes: data.get("tzone").and_then(JsonValue::as_i64),
    })
}

fn parse_camera_list(value: &JsonValue) -> Result<Vec<BlueIrisCamera>, BlueIrisError> {
    require_success("camlist", value)?;
    let entries = value
        .get("data")
        .and_then(JsonValue::as_array)
        .ok_or(BlueIrisError::MissingField("data"))?;
    let mut cameras = Vec::new();
    let mut names = BTreeSet::new();
    for entry in entries {
        let object = entry
            .as_object()
            .ok_or(BlueIrisError::MissingField("data[]"))?;
        if object.contains_key("group") {
            continue;
        }
        let short_name = required_string_any(object, &["optionValue"], "data[].optionValue")?;
        if !names.insert(short_name.clone()) {
            return Err(BlueIrisError::Validation(format!(
                "duplicate camera short name `{short_name}`"
            )));
        }
        cameras.push(BlueIrisCamera {
            name: required_string_any(object, &["optionDisplay"], "data[].optionDisplay")?,
            short_name,
            number: object.get("number").and_then(JsonValue::as_i64),
            enabled: bool_field(object, "isEnabled"),
            online: bool_field(object, "isOnline"),
            no_signal: bool_field(object, "isNoSignal"),
            paused: bool_field(object, "isPaused"),
            motion: bool_field(object, "isMotion"),
            triggered: bool_field(object, "isTriggered"),
            alerting: bool_field(object, "isAlerting"),
            recording: bool_field(object, "isRecording"),
            manual_recording: bool_field(object, "isManRec"),
            ptz: bool_field(object, "ptz"),
            audio: bool_field(object, "audio"),
            hidden: bool_field(object, "hidden"),
        });
    }
    cameras.sort_by(|left, right| left.short_name.cmp(&right.short_name));
    Ok(cameras)
}

fn require_success(command: &'static str, value: &JsonValue) -> Result<(), BlueIrisError> {
    if value.get("result").and_then(JsonValue::as_str) == Some("success") {
        Ok(())
    } else {
        Err(api_error(command, value))
    }
}

fn api_error(command: &'static str, value: &JsonValue) -> BlueIrisError {
    let reason = value
        .pointer("/data/reason")
        .and_then(JsonValue::as_str)
        .or_else(|| value.get("reason").and_then(JsonValue::as_str))
        .unwrap_or("device rejected request")
        .to_string();
    BlueIrisError::Api { command, reason }
}

fn required_string_any(
    object: &JsonMap<String, JsonValue>,
    fields: &[&str],
    label: &'static str,
) -> Result<String, BlueIrisError> {
    fields
        .iter()
        .find_map(|field| {
            object
                .get(*field)
                .and_then(JsonValue::as_str)
                .filter(|value| !value.trim().is_empty())
        })
        .map(str::to_string)
        .ok_or(BlueIrisError::MissingField(label))
}

fn bool_field(object: &JsonMap<String, JsonValue>, field: &str) -> bool {
    object
        .get(field)
        .and_then(JsonValue::as_bool)
        .unwrap_or(false)
}

fn camera_health(camera: &BlueIrisCamera) -> Health {
    if camera.online && !camera.no_signal {
        Health::Online
    } else {
        Health::Offline
    }
}

fn camera_value(camera: &BlueIrisCamera) -> Value {
    let mut fields = vec![
        (
            "short_name".to_string(),
            Value::Text(camera.short_name.clone()),
        ),
        ("enabled".to_string(), Value::Bool(camera.enabled)),
        ("online".to_string(), Value::Bool(camera.online)),
        ("no_signal".to_string(), Value::Bool(camera.no_signal)),
        ("paused".to_string(), Value::Bool(camera.paused)),
        ("motion".to_string(), Value::Bool(camera.motion)),
        ("triggered".to_string(), Value::Bool(camera.triggered)),
        ("alerting".to_string(), Value::Bool(camera.alerting)),
        ("recording".to_string(), Value::Bool(camera.recording)),
        (
            "manual_recording".to_string(),
            Value::Bool(camera.manual_recording),
        ),
        ("ptz".to_string(), Value::Bool(camera.ptz)),
        ("audio".to_string(), Value::Bool(camera.audio)),
        ("hidden".to_string(), Value::Bool(camera.hidden)),
    ];
    if let Some(number) = camera.number {
        fields.push(("number".to_string(), Value::Integer(number)));
    }
    Value::Object(fields)
}

fn ptz_preset_argument(request: &RuntimeCommandToolRequest) -> Result<u32, BlueIrisError> {
    let preset = object_u32(&request.arguments, "preset_id");
    match preset {
        Some(preset) if (MIN_PTZ_PRESET..=MAX_PTZ_PRESET).contains(&preset) => Ok(preset),
        _ => invalid_command_arguments(
            request.command_type,
            "an object with preset_id from 1 through 20",
        ),
    }
}

fn ptz_move_arguments(
    request: &RuntimeCommandToolRequest,
) -> Result<(BlueIrisPtzDirection, u32, u64), BlueIrisError> {
    let direction =
        object_text(&request.arguments, "direction").and_then(BlueIrisPtzDirection::from_label);
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
            "an object with direction left/right/up/down, speed 1 through 100, and duration_ms 1 through 5000",
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
    Some(value)
}

fn invalid_command_arguments<T>(
    command_type: CommandType,
    expected: &'static str,
) -> Result<T, BlueIrisError> {
    Err(BlueIrisError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, BlueIrisError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| BlueIrisError::Validation(error.to_string()))
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

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn encode_http_request(plan: &LocalHttpRequestPlan, body: &[u8]) -> Result<Vec<u8>, BlueIrisError> {
    let url = Url::parse(&plan.url)?;
    let host = url
        .host
        .as_deref()
        .ok_or(BlueIrisError::MissingField("request URL host"))?;
    let port = url
        .effective_port()
        .ok_or(BlueIrisError::MissingField("request URL port"))?;
    let target = if url.path.is_empty() {
        "/"
    } else {
        url.path.as_str()
    };
    if host.contains(['\r', '\n', '\0']) || target.contains(['\r', '\n', '\0']) {
        return Err(BlueIrisError::Validation(
            "request target contains unsafe HTTP text".to_string(),
        ));
    }
    let default_port = if url.scheme == "https" { 443 } else { 80 };
    let host_header = if port == default_port {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    let mut request =
        format!("POST {target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n")
            .into_bytes();
    for header in &plan.headers {
        if header.name.eq_ignore_ascii_case("Content-Length") {
            continue;
        }
        if header.name.contains(['\r', '\n', '\0']) || header.value.contains(['\r', '\n', '\0']) {
            return Err(BlueIrisError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    request.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    request.extend_from_slice(body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, BlueIrisError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| BlueIrisError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| BlueIrisError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| BlueIrisError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(BlueIrisError::Io(
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

fn write_request(writer: &mut dyn Write, request: &[u8]) -> Result<(), BlueIrisError> {
    writer
        .write_all(request)
        .map_err(|error| BlueIrisError::Io(error.to_string()))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, BlueIrisError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| BlueIrisError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(BlueIrisError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, BlueIrisError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| BlueIrisError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(BlueIrisError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(BlueIrisError::TruncatedBody {
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
        return Err(BlueIrisError::ResponseTooLarge { limit: maximum });
    }
    Ok(body)
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, BlueIrisError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input
            .get(cursor..)
            .and_then(|tail| tail.windows(2).position(|window| window == b"\r\n"))
            .ok_or_else(|| BlueIrisError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| BlueIrisError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| BlueIrisError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(BlueIrisError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| BlueIrisError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(BlueIrisError::Http("truncated chunk".to_string()));
        }
        output.extend_from_slice(&input[cursor..chunk_end]);
        cursor = chunk_end + 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn credentials() -> BlueIrisCredentials {
        BlueIrisCredentials::new("operator", "secret").unwrap()
    }

    fn config(port: u16) -> BlueIrisConfig {
        BlueIrisConfig::new(
            BridgeId::trusted("blue-iris.test"),
            format!("http://127.0.0.1:{port}"),
            VaultRef::trusted("vault://blue-iris/test"),
        )
        .unwrap()
    }

    fn snapshot() -> BlueIrisSnapshot {
        BlueIrisSnapshot {
            server: BlueIrisServer {
                name: "Home NVR".to_string(),
                version: "6.0.9.8".to_string(),
                admin: false,
                ptz_allowed: true,
                clip_create_allowed: false,
                timezone_minutes: Some(420),
            },
            cameras: vec![BlueIrisCamera {
                short_name: "front".to_string(),
                name: "Front Door".to_string(),
                number: Some(1),
                enabled: true,
                online: true,
                no_signal: false,
                paused: false,
                motion: true,
                triggered: false,
                alerting: false,
                recording: true,
                manual_recording: false,
                ptz: false,
                audio: true,
                hidden: false,
            }],
        }
    }

    fn commandable_snapshot() -> BlueIrisSnapshot {
        let mut snapshot = snapshot();
        snapshot.server.clip_create_allowed = true;
        snapshot.cameras[0].ptz = true;
        snapshot
    }

    fn authorize(runtime: &mut SmartHomeRuntime, principal: AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:blue-iris-test"),
                principal,
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    fn authorize_commands(runtime: &mut SmartHomeRuntime, principal: AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:blue-iris-command-test"),
                principal,
                PrivilegeTier::HumanApproval,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    fn start_json_server(
        responses: Vec<JsonValue>,
    ) -> (u16, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for response in responses {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    head.push_str(&line);
                }
                let length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                let mut body = vec![0u8; length];
                reader.read_exact(&mut body).unwrap();
                server_requests
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(body).unwrap());
                let body = serde_json::to_vec(&response).unwrap();
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let stream = reader.get_mut();
                stream.write_all(reply.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        (port, requests, handle)
    }

    #[derive(Debug)]
    struct FixedTransport {
        snapshot: BlueIrisSnapshot,
        calls: Arc<AtomicUsize>,
    }

    impl BlueIrisTransport for FixedTransport {
        fn inspect(
            &mut self,
            _plan: &LocalHttpRequestPlan,
            _credentials: &BlueIrisCredentials,
        ) -> Result<BlueIrisSnapshot, BlueIrisError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.snapshot.clone())
        }

        fn set_manual_recording(
            &mut self,
            _plan: &LocalHttpRequestPlan,
            _credentials: &BlueIrisCredentials,
            camera: &str,
            enabled: bool,
        ) -> Result<BlueIrisCamera, BlueIrisError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let mut camera = self
                .snapshot
                .cameras
                .iter()
                .find(|candidate| candidate.short_name == camera)
                .cloned()
                .ok_or_else(|| BlueIrisError::VerificationFailed("camera missing".to_string()))?;
            camera.manual_recording = enabled;
            camera.recording = enabled;
            Ok(camera)
        }

        fn recall_ptz_preset(
            &mut self,
            _plan: &LocalHttpRequestPlan,
            _credentials: &BlueIrisCredentials,
            _camera: &str,
            _preset: u32,
        ) -> Result<(), BlueIrisError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        fn move_ptz_bounded(
            &mut self,
            _plan: &LocalHttpRequestPlan,
            _credentials: &BlueIrisCredentials,
            _camera: &str,
            _direction: BlueIrisPtzDirection,
            _speed: u32,
            _duration_ms: u64,
        ) -> Result<(), BlueIrisError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn config_requires_https_outside_loopback() {
        assert!(BlueIrisConfig::new(
            BridgeId::trusted("blue-iris.bad"),
            "http://192.0.2.10:81",
            VaultRef::trusted("vault://blue-iris/bad")
        )
        .is_err());
        assert!(BlueIrisConfig::new(
            BridgeId::trusted("blue-iris.good"),
            "https://nvr.home:443",
            VaultRef::trusted("vault://blue-iris/good")
        )
        .is_ok());
    }

    #[test]
    fn credentials_and_client_debug_are_redacted() {
        assert_eq!(
            format!("{:?}", credentials()),
            "BlueIrisCredentials([REDACTED])"
        );
        let client = BlueIrisClient::new(
            config(81),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::new(AtomicUsize::new(0)),
            },
        )
        .unwrap();
        let debug = format!("{client:?}");
        assert!(!debug.contains("operator"));
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = BlueIrisClient::new(
            config(81),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = BlueIrisRuntimeIntegration::new(client);
        assert!(integration
            .inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                2_000
            )
            .is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn authorized_snapshot_installs_confirmed_camera_state() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = BlueIrisClient::new(
            config(81),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = BlueIrisRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:allowed");
        authorize(&mut runtime, principal.clone());
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 2_000)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(installed.cameras.len(), 1);
        let entity = runtime
            .registry()
            .entity(&installed.cameras[0].camera_entity_id)
            .unwrap();
        assert_eq!(entity.kind, EntityKind::Camera);
        assert_eq!(
            entity.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        assert_eq!(
            entity.state.as_ref().unwrap().value,
            camera_value(&snapshot().cameras[0])
        );
    }

    #[test]
    fn runtime_authorizes_validates_and_confirms_blue_iris_controls() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = BlueIrisClient::new(
            config(81),
            credentials(),
            FixedTransport {
                snapshot: commandable_snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = BlueIrisRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:controls");
        authorize(&mut runtime, principal.clone());
        authorize_commands(&mut runtime, principal.clone());
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 2_000)
            .unwrap();
        let entity_id = installed.cameras[0].camera_entity_id.clone();
        let entity = runtime.registry().entity(&entity_id).unwrap();
        assert!(entity.capabilities.iter().any(
            |capability| capability.capability_id == CapabilityId::trusted("camera.recording")
        ));
        assert!(entity
            .capabilities
            .iter()
            .any(|capability| capability.capability_id == CapabilityId::trusted("camera.ptz")));

        let recording = RuntimeCommandToolRequest::new(
            entity_id.clone(),
            CommandType::DeviceControl(DeviceControlCommandType::SetCameraRecording),
            Value::Bool(true),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                AgentId::trusted("agent:denied-control"),
                recording.clone(),
                2_500,
            ),
            Err(BlueIrisError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let invalid_move = RuntimeCommandToolRequest::new(
            entity_id.clone(),
            CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz),
            Value::Object(vec![
                ("direction".to_string(), Value::Text("right".to_string())),
                ("speed".to_string(), Value::Integer(50)),
                ("duration_ms".to_string(), Value::Integer(5_001)),
            ]),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                principal.clone(),
                invalid_move,
                2_600,
            ),
            Err(BlueIrisError::InvalidCommandArguments { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let recording_result = integration
            .dispatch_command_authorized(&mut runtime, principal.clone(), recording, 3_000)
            .unwrap();
        let preset_result = integration
            .dispatch_command_authorized(
                &mut runtime,
                principal.clone(),
                RuntimeCommandToolRequest::new(
                    entity_id.clone(),
                    CommandType::DeviceControl(DeviceControlCommandType::RecallCameraPtzPreset),
                    Value::Object(vec![("preset_id".to_string(), Value::Integer(4))]),
                ),
                4_000,
            )
            .unwrap();
        let move_result = integration
            .dispatch_command_authorized(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    entity_id.clone(),
                    CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz),
                    Value::Object(vec![
                        ("direction".to_string(), Value::Text("right".to_string())),
                        ("speed".to_string(), Value::Integer(50)),
                        ("duration_ms".to_string(), Value::Integer(1)),
                    ]),
                ),
                5_000,
            )
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 4);
        assert!(recording_result
            .message
            .as_deref()
            .is_some_and(|message| message.contains("manual recording started")));
        assert!(preset_result
            .message
            .as_deref()
            .is_some_and(|message| message.contains("preset 4")));
        assert!(move_result
            .message
            .as_deref()
            .is_some_and(|message| message.contains("bounded PTZ right")));
        let state = &runtime.registry().state(&entity_id).unwrap().value;
        assert!(matches!(
            state,
            Value::Object(fields)
                if fields.iter().any(|(field, value)|
                    field == "manual_recording" && *value == Value::Bool(true))
        ));
    }

    #[test]
    fn parser_skips_groups_and_sorts_cameras() {
        let value = json!({"result":"success","data":[
            {"optionDisplay":"All cameras","optionValue":"index","group":["back","front"]},
            {"optionDisplay":"Front Door","optionValue":"front","isEnabled":true,"isOnline":true},
            {"optionDisplay":"Back Yard","optionValue":"back","isEnabled":true,"isNoSignal":true}
        ]});
        let cameras = parse_camera_list(&value).unwrap();
        assert_eq!(
            cameras
                .iter()
                .map(|camera| camera.short_name.as_str())
                .collect::<Vec<_>>(),
            vec!["back", "front"]
        );
        assert!(cameras[1].online);
    }

    #[test]
    fn challenge_hash_matches_documented_md5_shape() {
        let credentials = BlueIrisCredentials::new("user", "password").unwrap();
        assert_eq!(
            challenge_response(&credentials, "session").as_str(),
            "1ebbcff1aa924ce5f580a766e5ee8eef"
        );
    }

    #[test]
    fn loopback_transport_performs_private_login_and_camlist_exchange() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            json!({"result":"fail","session":"abc123"}),
            json!({"result":"success","session":"abc123","data":{"system name":"Home NVR","version":"6.0.9.8","admin":false,"ptz":true,"clipcreate":false,"tzone":420,"license":"must-not-persist"}}),
            json!({"result":"success","data":[{"optionDisplay":"Front Door","optionValue":"front","number":1,"isEnabled":true,"isOnline":true,"isNoSignal":false,"isRecording":true,"ptz":false,"audio":true}]}),
            json!({"result":"success"}),
        ];
        let handle = thread::spawn(move || {
            for response in responses {
                let (stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream);
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    head.push_str(&line);
                }
                let length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("Content-Length: "))
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                let mut body = vec![0u8; length];
                reader.read_exact(&mut body).unwrap();
                server_requests
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(body).unwrap());
                let body = serde_json::to_vec(&response).unwrap();
                let reply = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                let stream = reader.get_mut();
                stream.write_all(reply.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
        });
        let mut client =
            BlueIrisClient::new(config(port), credentials(), BlueIrisLanTransport::default())
                .unwrap();
        let observed = client.inspect().unwrap();
        handle.join().unwrap();
        assert_eq!(observed.server.name, "Home NVR");
        assert_eq!(observed.cameras[0].short_name, "front");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert_eq!(requests[0], r#"{"cmd":"login"}"#);
        assert!(requests[1].contains(r#""session":"abc123""#));
        assert!(requests[1].contains(r#""response":""#));
        assert!(!requests[1].contains("operator"));
        assert!(!requests[1].contains("secret"));
        assert!(requests[2].contains(r#""cmd":"camlist""#));
        assert!(requests[3].contains(r#""cmd":"logout""#));
        assert!(!format!("{observed:?}").contains("must-not-persist"));
    }

    #[test]
    fn loopback_transport_proves_recording_readback_and_bounded_ptz_exchange() {
        let login = |session: &str, clip_create: bool| {
            vec![
                json!({"result":"fail","session":session}),
                json!({"result":"success","session":session,"data":{"system name":"Home NVR","version":"6.0.9.8","ptz":true,"clipcreate":clip_create}}),
            ]
        };
        let mut responses = login("recording", true);
        responses.extend([
            json!({"result":"success","data":{"manrec":true}}),
            json!({"result":"success","data":[{"optionDisplay":"Front Door","optionValue":"front","isEnabled":true,"isOnline":true,"isRecording":true,"isManRec":true,"ptz":true}]}),
            json!({"result":"success"}),
        ]);
        responses.extend(login("preset", false));
        responses.extend([json!({"result":"success"}), json!({"result":"success"})]);
        responses.extend(login("move", false));
        responses.extend([
            json!({"result":"success"}),
            json!({"result":"success"}),
            json!({"result":"success"}),
        ]);
        let (port, requests, handle) = start_json_server(responses);
        let mut client =
            BlueIrisClient::new(config(port), credentials(), BlueIrisLanTransport::default())
                .unwrap();
        let observed = client
            .set_manual_recording_and_verify("front", true)
            .unwrap();
        client.recall_ptz_preset("front", 4).unwrap();
        client
            .move_ptz_bounded("front", BlueIrisPtzDirection::Right, 50, 1)
            .unwrap();
        handle.join().unwrap();

        assert!(observed.manual_recording);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 14);
        assert!(requests[2].contains(r#""cmd":"camconfig""#));
        assert!(requests[2].contains(r#""camera":"front""#));
        assert!(requests[2].contains(r#""manrec":true"#));
        assert!(requests[3].contains(r#""cmd":"camlist""#));
        assert!(requests[4].contains(r#""cmd":"logout""#));
        assert!(requests[7].contains(r#""button":104"#));
        assert!(requests[8].contains(r#""cmd":"logout""#));
        assert!(requests[11].contains(r#""joystick":640"#));
        assert!(requests[11].contains(r#""updown":1"#));
        assert!(requests[12].contains(r#""button":64"#));
        assert!(requests[12].contains(r#""updown":0"#));
        assert!(requests[13].contains(r#""cmd":"logout""#));
        assert!(requests.iter().all(|request| !request.contains("secret")));
    }

    #[test]
    fn failed_ptz_start_attempts_stop_before_logout() {
        let responses = vec![
            json!({"result":"fail","session":"move-failure"}),
            json!({"result":"success","session":"move-failure","data":{"system name":"Home NVR","version":"6.0.9.8","ptz":true,"clipcreate":false}}),
            json!({"result":"fail","data":{"reason":"response lost after dispatch"}}),
            json!({"result":"success"}),
            json!({"result":"success"}),
        ];
        let (port, requests, handle) = start_json_server(responses);
        let mut client =
            BlueIrisClient::new(config(port), credentials(), BlueIrisLanTransport::default())
                .unwrap();
        assert!(matches!(
            client.move_ptz_bounded("front", BlueIrisPtzDirection::Left, 25, 1),
            Err(BlueIrisError::Api { command: "ptz", .. })
        ));
        handle.join().unwrap();

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[2].contains(r#""updown":1"#));
        assert!(requests[3].contains(r#""button":64"#));
        assert!(requests[3].contains(r#""updown":0"#));
        assert!(requests[4].contains(r#""cmd":"logout""#));
    }

    #[test]
    fn response_bounds_are_enforced() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        assert!(matches!(
            decode_http_response(response, 1),
            Err(BlueIrisError::ResponseTooLarge { limit: 1 })
        ));
    }
}
