//! Authenticated Blue Iris NVR inspection for D23.

#![forbid(unsafe_code)]

use coding_adventures_md5::sum_md5;
use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::BodyKind;
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode, Device,
    DeviceId, Entity, EntityId, EntityKind, Health, IntegrationId, Metadata, ProtocolFamily,
    ProtocolIdentifier, SmartHomeTool, StateConfidence, StateSnapshot, StateSource, Value,
    ValueKind, VaultRef,
};
use smart_home_discovery::{
    DiscoveryConfidence, DiscoveryRecord, DiscoverySource, PairingRequirement,
};
use smart_home_local_http::{
    LocalHttpAuth, LocalHttpEndpoint, LocalHttpError, LocalHttpMethod, LocalHttpRequestPlan,
    LocalHttpRequestTemplate, LocalHttpScheme,
};
use smart_home_runtime::{RuntimeError, SmartHomeRuntime};
use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig, TlsConnector};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "blue_iris";
pub const PROTOCOL_ID: &str = "blue_iris_json";
pub const JSON_PATH: &str = "/json";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_SESSION_BYTES: usize = 512;

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

pub trait BlueIrisTransport {
    fn inspect(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
    ) -> Result<BlueIrisSnapshot, BlueIrisError>;
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
}

impl BlueIrisTransport for BlueIrisLanTransport {
    fn inspect(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &BlueIrisCredentials,
    ) -> Result<BlueIrisSnapshot, BlueIrisError> {
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
        let server = parse_login_success(&login)?;
        let cameras = self.post_json(plan, &json!({"cmd":"camlist","session":session.as_str()}))?;
        Ok(BlueIrisSnapshot {
            server,
            cameras: parse_camera_list(&cameras)?,
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
}

impl<T: BlueIrisTransport> BlueIrisRuntimeIntegration<T> {
    pub fn new(client: BlueIrisClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledBlueIrisNvr, BlueIrisError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
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
        runtime.upsert_entity(Entity {
            entity_id: camera_entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Camera,
            name: camera.name.clone(),
            capabilities: vec![
                Capability::new(
                    CapabilityId::trusted("camera.health"),
                    CapabilityMode::Observe,
                    ValueKind::Object,
                ),
                Capability::new(
                    CapabilityId::trusted("camera.recording"),
                    CapabilityMode::Observe,
                    ValueKind::Boolean,
                ),
            ],
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
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0], r#"{"cmd":"login"}"#);
        assert!(requests[1].contains(r#""session":"abc123""#));
        assert!(requests[1].contains(r#""response":""#));
        assert!(!requests[1].contains("operator"));
        assert!(!requests[1].contains("secret"));
        assert!(requests[2].contains(r#""cmd":"camlist""#));
        assert!(!format!("{observed:?}").contains("must-not-persist"));
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
