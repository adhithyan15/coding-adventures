//! Authenticated Frigate NVR and camera health inspection for D23.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::{Zeroize, Zeroizing};
use http1::{parse_response_head, Http1ParseError};
use http_core::{BodyKind, Header};
use serde_json::{Map as JsonMap, Value as JsonValue};
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
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig, TlsConnector};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "frigate";
pub const PROTOCOL_ID: &str = "frigate_http_api";
pub const LOGIN_PATH: &str = "/api/login";
pub const VERSION_PATH: &str = "/api/version";
pub const STATS_PATH: &str = "/api/stats";
pub const LOGOUT_PATH: &str = "/api/logout";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_COOKIE_BYTES: usize = 8 * 1024;
const MAX_VERSION_BYTES: usize = 256;

#[derive(Debug)]
pub enum FrigateError {
    Validation(String),
    LocalHttp(LocalHttpError),
    Url(UrlError),
    Io(String),
    Tls(String),
    Http(String),
    HttpStatus {
        operation: &'static str,
        status: u16,
    },
    ResponseTooLarge {
        limit: usize,
    },
    TruncatedBody {
        expected: usize,
        actual: usize,
    },
    Json(serde_json::Error),
    MissingField(&'static str),
    Runtime(RuntimeError),
}

impl fmt::Display for FrigateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Frigate input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Frigate URL: {error}"),
            Self::Io(message) => write!(formatter, "Frigate LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "Frigate TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Frigate HTTP response: {message}"),
            Self::HttpStatus { operation, status } => {
                write!(formatter, "Frigate {operation} returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Frigate response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Frigate response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Frigate JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "Frigate response is missing {field}"),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for FrigateError {}

impl From<LocalHttpError> for FrigateError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for FrigateError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for FrigateError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for FrigateError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct FrigateCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl FrigateCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, FrigateError> {
        let username = username.into();
        let password = password.into();
        if username.trim().is_empty() || password.is_empty() {
            return Err(FrigateError::Validation(
                "username and password must not be empty".to_string(),
            ));
        }
        if username.contains('\0') || password.contains('\0') {
            return Err(FrigateError::Validation(
                "credentials contain a NUL byte".to_string(),
            ));
        }
        Ok(Self {
            username: Zeroizing::new(username),
            password: Zeroizing::new(password),
        })
    }
}

impl fmt::Debug for FrigateCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FrigateCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrigateConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub credential_ref: VaultRef,
    pub timeout: Duration,
}

impl FrigateConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        credential_ref: VaultRef,
    ) -> Result<Self, FrigateError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(FrigateError::MissingField("base URL host"))?;
        let secure = parsed.scheme == "https";
        let test_loopback = parsed.scheme == "http" && is_loopback_host(host);
        if (!secure && !test_loopback)
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(FrigateError::Validation(
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

    fn endpoint(&self) -> Result<LocalHttpEndpoint, FrigateError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(FrigateError::MissingField("base URL host"))?;
        let scheme = match parsed.scheme.as_str() {
            "https" => LocalHttpScheme::Https,
            "http" if is_loopback_host(host) => LocalHttpScheme::Http,
            _ => {
                return Err(FrigateError::Validation(
                    "Frigate endpoint is not approved".to_string(),
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
        .with_metadata(Metadata::new("http.profile", "frigate.authenticated-api")))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrigateCameraStats {
    pub name: String,
    pub camera_fps: f64,
    pub process_fps: f64,
    pub skipped_fps: f64,
    pub detection_fps: f64,
    pub detection_enabled: bool,
    pub connection_quality: Option<String>,
    pub expected_fps: Option<f64>,
    pub reconnects_last_hour: Option<i64>,
    pub stalls_last_hour: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrigateSnapshot {
    pub version: String,
    pub cameras: Vec<FrigateCameraStats>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrigateRequestPlans {
    pub login: LocalHttpRequestPlan,
    pub version: LocalHttpRequestPlan,
    pub stats: LocalHttpRequestPlan,
    pub logout: LocalHttpRequestPlan,
}

pub trait FrigateTransport {
    fn inspect(
        &mut self,
        plans: &FrigateRequestPlans,
        credentials: &FrigateCredentials,
    ) -> Result<FrigateSnapshot, FrigateError>;
}

pub struct FrigateLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    maximum_response_bytes: usize,
}

impl Default for FrigateLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl FrigateLanTransport {
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

    fn request(
        &mut self,
        plan: &LocalHttpRequestPlan,
        body: &[u8],
        cookie: Option<&str>,
    ) -> Result<HttpResponse, FrigateError> {
        let request = Zeroizing::new(encode_http_request(plan, body, cookie)?);
        let url = Url::parse(&plan.url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(FrigateError::MissingField("request URL host"))?;
        let port = url
            .effective_port()
            .ok_or(FrigateError::MissingField("request URL port"))?;
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let response = match url.scheme.as_str() {
            "http" if is_loopback_host(host) => {
                let mut stream = connect_tcp(host, port, timeout)?;
                write_request(&mut stream, request.as_slice())?;
                Zeroizing::new(read_bounded(&mut stream, self.maximum_response_bytes)?)
            }
            "https" => {
                let mut config = self.tls_config.clone();
                config.connect_timeout = timeout;
                config.read_timeout = Some(timeout);
                config.write_timeout = Some(timeout);
                let mut stream = self
                    .connector
                    .connect(host, port, &config)
                    .map_err(|error| FrigateError::Tls(error.to_string()))?;
                write_request(&mut stream, request.as_slice())?;
                let bytes = Zeroizing::new(read_bounded(&mut stream, self.maximum_response_bytes)?);
                stream
                    .close_notify()
                    .map_err(|error| FrigateError::Tls(error.to_string()))?;
                bytes
            }
            _ => {
                return Err(FrigateError::Validation(
                    "Frigate transport requires HTTPS or loopback HTTP".to_string(),
                ))
            }
        };
        decode_http_response(response.as_slice(), self.maximum_response_bytes)
    }

    fn logout(&mut self, plan: &LocalHttpRequestPlan, cookie: &str) -> Result<(), FrigateError> {
        let response = self.request(plan, &[], Some(cookie))?;
        if (200..300).contains(&response.status) || response.status == 303 {
            Ok(())
        } else {
            Err(FrigateError::HttpStatus {
                operation: "logout",
                status: response.status,
            })
        }
    }
}

impl FrigateTransport for FrigateLanTransport {
    fn inspect(
        &mut self,
        plans: &FrigateRequestPlans,
        credentials: &FrigateCredentials,
    ) -> Result<FrigateSnapshot, FrigateError> {
        let fields = BTreeMap::from([
            ("password", credentials.password.as_str()),
            ("user", credentials.username.as_str()),
        ]);
        let login_body = Zeroizing::new(serde_json::to_vec(&fields)?);
        let login = self.request(&plans.login, login_body.as_slice(), None)?;
        if login.status != 200 {
            return Err(FrigateError::HttpStatus {
                operation: "login",
                status: login.status,
            });
        }
        let cookie = extract_session_cookie(&login.headers)?;

        let result = (|| {
            let version_response = self.request(&plans.version, &[], Some(cookie.as_str()))?;
            if version_response.status != 200 {
                return Err(FrigateError::HttpStatus {
                    operation: "version",
                    status: version_response.status,
                });
            }
            let version = parse_version(&version_response.body)?;
            let stats_response = self.request(&plans.stats, &[], Some(cookie.as_str()))?;
            if stats_response.status != 200 {
                return Err(FrigateError::HttpStatus {
                    operation: "stats",
                    status: stats_response.status,
                });
            }
            Ok(FrigateSnapshot {
                version,
                cameras: parse_stats(&stats_response.body)?,
            })
        })();
        let logout = self.logout(&plans.logout, cookie.as_str());
        match (result, logout) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(snapshot), Ok(())) => Ok(snapshot),
        }
    }
}

pub struct FrigateClient<T> {
    config: FrigateConfig,
    credentials: FrigateCredentials,
    transport: T,
    plans: FrigateRequestPlans,
}

impl<T: FrigateTransport> FrigateClient<T> {
    pub fn new(
        config: FrigateConfig,
        credentials: FrigateCredentials,
        transport: T,
    ) -> Result<Self, FrigateError> {
        let endpoint = config.endpoint()?;
        let timeout_ms = duration_ms(config.timeout);
        let get = |path| {
            LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
                .with_accept("application/json")
                .with_timeout_ms(timeout_ms)
                .with_auth(LocalHttpAuth::None)
                .plan(&endpoint, Vec::new())
                .map_err(FrigateError::from)
        };
        let login = LocalHttpRequestTemplate::new(LocalHttpMethod::Post, LOGIN_PATH)?
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_timeout_ms(timeout_ms)
            .with_idempotent(false)
            .with_auth(LocalHttpAuth::None)
            .plan(&endpoint, Vec::new())?;
        let plans = FrigateRequestPlans {
            login,
            version: get(VERSION_PATH)?,
            stats: get(STATS_PATH)?,
            logout: get(LOGOUT_PATH)?,
        };
        Ok(Self {
            config,
            credentials,
            transport,
            plans,
        })
    }

    pub fn inspect(&mut self) -> Result<FrigateSnapshot, FrigateError> {
        self.transport.inspect(&self.plans, &self.credentials)
    }
}

impl<T> fmt::Debug for FrigateClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FrigateClient")
            .field("config", &self.config)
            .field("credentials", &"[REDACTED]")
            .field("plans", &self.plans)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledFrigateCamera {
    pub device_id: DeviceId,
    pub camera_entity_id: EntityId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledFrigateNvr {
    pub bridge_id: BridgeId,
    pub cameras: Vec<InstalledFrigateCamera>,
}

pub struct FrigateRuntimeIntegration<T> {
    client: FrigateClient<T>,
}

impl<T: FrigateTransport> FrigateRuntimeIntegration<T> {
    pub fn new(client: FrigateClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledFrigateNvr, FrigateError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

pub fn paired_discovery_record(
    config: &FrigateConfig,
    snapshot: &FrigateSnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, FrigateError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&config.base_url),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| FrigateError::Validation(error.to_string()))?
    .with_display_name("Frigate NVR")
    .with_address(config.base_url.clone())
    .with_hardware_model("Frigate NVR")
    .with_firmware_version(snapshot.version.clone())
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("frigate.protocol", PROTOCOL_ID)
    .with_metadata("frigate.camera_count", snapshot.cameras.len().to_string()))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &FrigateConfig,
    snapshot: &FrigateSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledFrigateNvr, FrigateError> {
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some("Frigate NVR".to_string());
    bridge.firmware_version = Some(snapshot.version.clone());
    bridge.auth_ref = Some(config.credential_ref.clone());
    bridge.health = aggregate_health(&snapshot.cameras);
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("https_endpoint", &config.base_url)?];
    bridge.metadata = vec![
        Metadata::new("frigate.transport", "authenticated_cookie_session"),
        Metadata::new("frigate.camera_count", snapshot.cameras.len().to_string()),
    ];
    runtime.upsert_bridge(bridge)?;

    let mut installed = Vec::with_capacity(snapshot.cameras.len());
    for camera in &snapshot.cameras {
        let native_id = stable_component(&camera.name);
        if native_id.is_empty() {
            return Err(FrigateError::Validation(
                "camera name has no stable identifier".to_string(),
            ));
        }
        let device_id = DeviceId::trusted(format!("frigate:{native_id}"));
        let camera_entity_id = EntityId::trusted(format!("frigate:{native_id}:camera"));
        let health = camera_health(camera);
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "Frigate".to_string(),
            model: "Managed camera".to_string(),
            name: camera.name.clone(),
            serial: None,
            firmware_version: None,
            room_id: None,
            entity_ids: vec![camera_entity_id.clone()],
            identifiers: vec![protocol_identifier("camera_name", &camera.name)?],
            health,
            metadata: vec![Metadata::new("frigate.camera_name", camera.name.clone())],
        })?;
        runtime.upsert_entity(Entity {
            entity_id: camera_entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Camera,
            name: camera.name.clone(),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("camera.health"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: camera_entity_id.clone(),
                value: camera_value(camera),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("frigate.protocol", PROTOCOL_ID)],
        })?;
        installed.push(InstalledFrigateCamera {
            device_id,
            camera_entity_id,
        });
    }
    Ok(InstalledFrigateNvr {
        bridge_id: config.bridge_id.clone(),
        cameras: installed,
    })
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), FrigateError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(FrigateError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn extract_session_cookie(headers: &[Header]) -> Result<Zeroizing<String>, FrigateError> {
    for header in headers {
        if !header.name.eq_ignore_ascii_case("Set-Cookie") {
            continue;
        }
        let pair = header.value.split(';').next().unwrap_or_default().trim();
        let Some((name, value)) = pair.split_once('=') else {
            continue;
        };
        if pair.len() > MAX_COOKIE_BYTES
            || name.is_empty()
            || value.is_empty()
            || !name.bytes().all(is_cookie_name_byte)
            || value
                .bytes()
                .any(|byte| byte <= 0x20 || byte == 0x7f || byte == b';')
        {
            return Err(FrigateError::Validation(
                "login returned an unsafe session cookie".to_string(),
            ));
        }
        return Ok(Zeroizing::new(pair.to_string()));
    }
    Err(FrigateError::MissingField("login Set-Cookie session"))
}

fn is_cookie_name_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'!' | b'#'
                | b'$'
                | b'%'
                | b'&'
                | b'\''
                | b'*'
                | b'+'
                | b'-'
                | b'.'
                | b'^'
                | b'_'
                | b'`'
                | b'|'
                | b'~'
        )
}

fn parse_version(body: &[u8]) -> Result<String, FrigateError> {
    if body.len() > MAX_VERSION_BYTES {
        return Err(FrigateError::Validation(
            "version exceeds the accepted bound".to_string(),
        ));
    }
    if let Ok(JsonValue::String(version)) = serde_json::from_slice::<JsonValue>(body) {
        return validate_version(version);
    }
    let version = std::str::from_utf8(body)
        .map_err(|_| FrigateError::Validation("version is not UTF-8".to_string()))?
        .trim()
        .to_string();
    validate_version(version)
}

fn validate_version(version: String) -> Result<String, FrigateError> {
    if version.is_empty() || version.contains(['\r', '\n', '\0']) {
        Err(FrigateError::Validation(
            "version is empty or unsafe".to_string(),
        ))
    } else {
        Ok(version)
    }
}

fn parse_stats(body: &[u8]) -> Result<Vec<FrigateCameraStats>, FrigateError> {
    let value: JsonValue = serde_json::from_slice(body)?;
    let cameras = value
        .get("cameras")
        .and_then(JsonValue::as_object)
        .ok_or(FrigateError::MissingField("cameras"))?;
    let mut parsed = Vec::with_capacity(cameras.len());
    let mut names = BTreeSet::new();
    for (name, value) in cameras {
        if name.trim().is_empty() || !names.insert(name.clone()) {
            return Err(FrigateError::Validation(
                "camera names must be non-empty and unique".to_string(),
            ));
        }
        let object = value
            .as_object()
            .ok_or(FrigateError::MissingField("cameras.<name>"))?;
        let connection_quality = optional_quality(object)?;
        parsed.push(FrigateCameraStats {
            name: name.clone(),
            camera_fps: required_nonnegative_number(object, "camera_fps")?,
            process_fps: required_nonnegative_number(object, "process_fps")?,
            skipped_fps: required_nonnegative_number(object, "skipped_fps")?,
            detection_fps: required_nonnegative_number(object, "detection_fps")?,
            detection_enabled: required_bool(object, "detection_enabled")?,
            connection_quality,
            expected_fps: optional_nonnegative_number(object, "expected_fps")?,
            reconnects_last_hour: optional_nonnegative_integer(object, "reconnects_last_hour")?,
            stalls_last_hour: optional_nonnegative_integer(object, "stalls_last_hour")?,
        });
    }
    parsed.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(parsed)
}

fn required_nonnegative_number(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<f64, FrigateError> {
    let value = object
        .get(field)
        .and_then(JsonValue::as_f64)
        .ok_or(FrigateError::MissingField(field))?;
    if value.is_finite() && value >= 0.0 {
        Ok(value)
    } else {
        Err(FrigateError::Validation(format!(
            "{field} must be a finite non-negative number"
        )))
    }
}

fn optional_nonnegative_number(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Option<f64>, FrigateError> {
    match object.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(_) => required_nonnegative_number(object, field).map(Some),
    }
}

fn optional_nonnegative_integer(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Option<i64>, FrigateError> {
    match object.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .filter(|value| *value >= 0)
            .map(Some)
            .ok_or_else(|| {
                FrigateError::Validation(format!("{field} must be a non-negative integer"))
            }),
    }
}

fn required_bool(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<bool, FrigateError> {
    object
        .get(field)
        .and_then(JsonValue::as_bool)
        .ok_or(FrigateError::MissingField(field))
}

fn optional_quality(object: &JsonMap<String, JsonValue>) -> Result<Option<String>, FrigateError> {
    let Some(value) = object.get("connection_quality") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let quality = value
        .as_str()
        .ok_or_else(|| FrigateError::Validation("connection_quality must be a string".to_string()))?
        .to_ascii_lowercase();
    if matches!(quality.as_str(), "excellent" | "fair" | "poor" | "unusable") {
        Ok(Some(quality))
    } else {
        Err(FrigateError::Validation(format!(
            "unknown connection_quality `{quality}`"
        )))
    }
}

fn camera_health(camera: &FrigateCameraStats) -> Health {
    if camera.camera_fps < 0.01 || camera.connection_quality.as_deref() == Some("unusable") {
        Health::Offline
    } else if camera.connection_quality.as_deref() == Some("poor")
        || camera.reconnects_last_hour.unwrap_or(0) > 0
        || camera.stalls_last_hour.unwrap_or(0) > 0
    {
        Health::Degraded
    } else {
        Health::Online
    }
}

fn aggregate_health(cameras: &[FrigateCameraStats]) -> Health {
    if cameras.is_empty() {
        return Health::Degraded;
    }
    let health = cameras.iter().map(camera_health).collect::<Vec<_>>();
    if health.iter().all(|value| *value == Health::Offline) {
        Health::Offline
    } else if health.iter().any(|value| *value != Health::Online) {
        Health::Degraded
    } else {
        Health::Online
    }
}

fn camera_value(camera: &FrigateCameraStats) -> Value {
    let mut fields = vec![
        ("camera_fps".to_string(), Value::Number(camera.camera_fps)),
        ("process_fps".to_string(), Value::Number(camera.process_fps)),
        ("skipped_fps".to_string(), Value::Number(camera.skipped_fps)),
        (
            "detection_fps".to_string(),
            Value::Number(camera.detection_fps),
        ),
        (
            "detection_enabled".to_string(),
            Value::Bool(camera.detection_enabled),
        ),
    ];
    if let Some(quality) = &camera.connection_quality {
        fields.push((
            "connection_quality".to_string(),
            Value::Text(quality.clone()),
        ));
    }
    if let Some(expected_fps) = camera.expected_fps {
        fields.push(("expected_fps".to_string(), Value::Number(expected_fps)));
    }
    if let Some(reconnects) = camera.reconnects_last_hour {
        fields.push((
            "reconnects_last_hour".to_string(),
            Value::Integer(reconnects),
        ));
    }
    if let Some(stalls) = camera.stalls_last_hour {
        fields.push(("stalls_last_hour".to_string(), Value::Integer(stalls)));
    }
    Value::Object(fields)
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, FrigateError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| FrigateError::Validation(error.to_string()))
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

fn encode_http_request(
    plan: &LocalHttpRequestPlan,
    body: &[u8],
    cookie: Option<&str>,
) -> Result<Vec<u8>, FrigateError> {
    let url = Url::parse(&plan.url)?;
    let host = url
        .host
        .as_deref()
        .ok_or(FrigateError::MissingField("request URL host"))?;
    let port = url
        .effective_port()
        .ok_or(FrigateError::MissingField("request URL port"))?;
    let target = if url.path.is_empty() {
        "/"
    } else {
        url.path.as_str()
    };
    if host.contains(['\r', '\n', '\0']) || target.contains(['\r', '\n', '\0']) {
        return Err(FrigateError::Validation(
            "request target contains unsafe HTTP text".to_string(),
        ));
    }
    let default_port = if url.scheme == "https" { 443 } else { 80 };
    let host_header = if port == default_port {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    let mut request = format!(
        "{} {target} HTTP/1.1\r\nHost: {host_header}\r\nConnection: close\r\n",
        plan.method.as_str()
    )
    .into_bytes();
    for header in &plan.headers {
        if header.name.eq_ignore_ascii_case("Content-Length")
            || header.name.eq_ignore_ascii_case("Cookie")
        {
            continue;
        }
        if header.name.contains(['\r', '\n', '\0']) || header.value.contains(['\r', '\n', '\0']) {
            return Err(FrigateError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    if let Some(cookie) = cookie {
        if cookie.len() > MAX_COOKIE_BYTES || cookie.contains(['\r', '\n', '\0', ';']) {
            return Err(FrigateError::Validation(
                "session cookie contains unsafe HTTP text".to_string(),
            ));
        }
        request.extend_from_slice(format!("Cookie: {cookie}\r\n").as_bytes());
    }
    request.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    request.extend_from_slice(body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, FrigateError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| FrigateError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| FrigateError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| FrigateError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(FrigateError::Io(
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

fn write_request(writer: &mut dyn Write, request: &[u8]) -> Result<(), FrigateError> {
    writer
        .write_all(request)
        .map_err(|error| FrigateError::Io(error.to_string()))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, FrigateError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| FrigateError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(FrigateError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

struct HttpResponse {
    status: u16,
    headers: Vec<Header>,
    body: Vec<u8>,
}

impl Drop for HttpResponse {
    fn drop(&mut self) {
        for header in &mut self.headers {
            header.value.zeroize();
        }
        self.body.zeroize();
    }
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<HttpResponse, FrigateError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| FrigateError::Http(error.to_string()))?;
    let status = parsed.head.status;
    let mut headers = parsed.head.headers;
    let input = &bytes[parsed.body_offset..];
    let body = match (|| {
        let body = match parsed.body_kind {
            BodyKind::None => Vec::new(),
            BodyKind::ContentLength(expected) => {
                if input.len() < expected {
                    return Err(FrigateError::TruncatedBody {
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
            return Err(FrigateError::ResponseTooLarge { limit: maximum });
        }
        Ok(body)
    })() {
        Ok(body) => body,
        Err(error) => {
            for header in &mut headers {
                header.value.zeroize();
            }
            return Err(error);
        }
    };
    Ok(HttpResponse {
        status,
        headers,
        body,
    })
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, FrigateError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input
            .get(cursor..)
            .and_then(|tail| tail.windows(2).position(|window| window == b"\r\n"))
            .ok_or_else(|| FrigateError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| FrigateError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| FrigateError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(FrigateError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| FrigateError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(FrigateError::Http("truncated chunk".to_string()));
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

    fn credentials() -> FrigateCredentials {
        FrigateCredentials::new("operator", "secret-password").unwrap()
    }

    fn config(port: u16) -> FrigateConfig {
        FrigateConfig::new(
            BridgeId::trusted("frigate.test"),
            format!("http://127.0.0.1:{port}"),
            VaultRef::trusted("vault://frigate/test"),
        )
        .unwrap()
    }

    fn snapshot() -> FrigateSnapshot {
        FrigateSnapshot {
            version: "0.15.1".to_string(),
            cameras: vec![
                FrigateCameraStats {
                    name: "back".to_string(),
                    camera_fps: 5.0,
                    process_fps: 5.0,
                    skipped_fps: 0.0,
                    detection_fps: 1.2,
                    detection_enabled: true,
                    connection_quality: Some("poor".to_string()),
                    expected_fps: Some(5.0),
                    reconnects_last_hour: Some(2),
                    stalls_last_hour: Some(1),
                },
                FrigateCameraStats {
                    name: "front".to_string(),
                    camera_fps: 5.0,
                    process_fps: 5.0,
                    skipped_fps: 0.0,
                    detection_fps: 2.5,
                    detection_enabled: true,
                    connection_quality: Some("excellent".to_string()),
                    expected_fps: Some(5.0),
                    reconnects_last_hour: Some(0),
                    stalls_last_hour: Some(0),
                },
            ],
        }
    }

    #[derive(Debug)]
    struct FixedTransport {
        snapshot: FrigateSnapshot,
        calls: Arc<AtomicUsize>,
    }

    impl FrigateTransport for FixedTransport {
        fn inspect(
            &mut self,
            _plans: &FrigateRequestPlans,
            _credentials: &FrigateCredentials,
        ) -> Result<FrigateSnapshot, FrigateError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.snapshot.clone())
        }
    }

    fn authorize(runtime: &mut SmartHomeRuntime, principal: AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:frigate-test"),
                principal,
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn config_requires_https_outside_loopback() {
        assert!(FrigateConfig::new(
            BridgeId::trusted("frigate.bad"),
            "http://192.0.2.10:8971",
            VaultRef::trusted("vault://frigate/bad")
        )
        .is_err());
        assert!(FrigateConfig::new(
            BridgeId::trusted("frigate.good"),
            "https://frigate.home:8971",
            VaultRef::trusted("vault://frigate/good")
        )
        .is_ok());
        assert!(FrigateConfig::new(
            BridgeId::trusted("frigate.embedded"),
            "https://operator:secret@frigate.home:8971",
            VaultRef::trusted("vault://frigate/embedded")
        )
        .is_err());
    }

    #[test]
    fn credentials_and_client_debug_are_redacted() {
        assert_eq!(
            format!("{:?}", credentials()),
            "FrigateCredentials([REDACTED])"
        );
        let client = FrigateClient::new(
            config(8971),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::new(AtomicUsize::new(0)),
            },
        )
        .unwrap();
        let debug = format!("{client:?}");
        assert!(!debug.contains("operator"));
        assert!(!debug.contains("secret-password"));
        assert!(!debug.contains("Cookie"));
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = FrigateClient::new(
            config(8971),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = FrigateRuntimeIntegration::new(client);
        assert!(integration
            .inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                2_000,
            )
            .is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn authorized_snapshot_installs_confirmed_health_state() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = FrigateClient::new(
            config(8971),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = FrigateRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:allowed");
        authorize(&mut runtime, principal.clone());
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 2_000)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(installed.cameras.len(), 2);
        let back = runtime
            .registry()
            .entity(&installed.cameras[0].camera_entity_id)
            .unwrap();
        assert_eq!(back.kind, EntityKind::Camera);
        assert_eq!(
            back.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        assert_eq!(
            runtime
                .registry()
                .device(&installed.cameras[0].device_id)
                .unwrap()
                .health,
            Health::Degraded
        );
        assert_eq!(
            runtime
                .registry()
                .device(&installed.cameras[1].device_id)
                .unwrap()
                .health,
            Health::Online
        );
    }

    #[test]
    fn stats_parser_sorts_cameras_and_rejects_unknown_quality() {
        let body = br#"{"cameras":{"front":{"camera_fps":5.0,"process_fps":5.0,"skipped_fps":0.0,"detection_fps":2.0,"detection_enabled":true,"connection_quality":"excellent"},"back":{"camera_fps":0.0,"process_fps":0.0,"skipped_fps":0.0,"detection_fps":0.0,"detection_enabled":false,"connection_quality":"unusable"}}}"#;
        let cameras = parse_stats(body).unwrap();
        assert_eq!(
            cameras
                .iter()
                .map(|camera| camera.name.as_str())
                .collect::<Vec<_>>(),
            vec!["back", "front"]
        );
        assert_eq!(camera_health(&cameras[0]), Health::Offline);
        let invalid = br#"{"cameras":{"front":{"camera_fps":5.0,"process_fps":5.0,"skipped_fps":0.0,"detection_fps":2.0,"detection_enabled":true,"connection_quality":"mystery"}}}"#;
        assert!(parse_stats(invalid).is_err());
    }

    #[test]
    fn loopback_transport_keeps_login_and_cookie_material_private() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            (
                "200 OK",
                vec![(
                    "Set-Cookie",
                    "frigate-token=secret.jwt.value; Path=/; HttpOnly; Secure; SameSite=strict",
                )],
                r#"{"message":"Login successful"}"#.to_string(),
            ),
            ("200 OK", vec![], r#""0.15.1""#.to_string()),
            (
                "200 OK",
                vec![],
                r#"{"cameras":{"front":{"camera_fps":5.0,"process_fps":5.0,"skipped_fps":0.0,"detection_fps":2.5,"detection_enabled":true,"connection_quality":"excellent","expected_fps":5.0,"reconnects_last_hour":0,"stalls_last_hour":0},"back":{"camera_fps":4.8,"process_fps":4.7,"skipped_fps":0.1,"detection_fps":1.0,"detection_enabled":true,"connection_quality":"fair","expected_fps":5.0,"reconnects_last_hour":0,"stalls_last_hour":0}}}"#.to_string(),
            ),
            (
                "303 See Other",
                vec![(
                    "Set-Cookie",
                    "frigate-token=; Path=/; Max-Age=0; HttpOnly; Secure",
                )],
                String::new(),
            ),
        ];
        let handle = thread::spawn(move || {
            for (status, headers, body) in responses {
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
                    .unwrap_or("0")
                    .parse::<usize>()
                    .unwrap();
                let mut request_body = vec![0u8; length];
                reader.read_exact(&mut request_body).unwrap();
                server_requests
                    .lock()
                    .unwrap()
                    .push((head, String::from_utf8(request_body).unwrap()));
                let mut reply = format!("HTTP/1.1 {status}\r\n");
                for (name, value) in headers {
                    reply.push_str(&format!("{name}: {value}\r\n"));
                }
                reply.push_str(&format!(
                    "Content-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                ));
                let stream = reader.get_mut();
                stream.write_all(reply.as_bytes()).unwrap();
                stream.write_all(body.as_bytes()).unwrap();
            }
        });

        let mut client =
            FrigateClient::new(config(port), credentials(), FrigateLanTransport::default())
                .unwrap();
        let observed = client.inspect().unwrap();
        handle.join().unwrap();
        assert_eq!(observed.version, "0.15.1");
        assert_eq!(observed.cameras[0].name, "back");
        assert!(!format!("{observed:?}").contains("secret.jwt.value"));

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].0.starts_with("POST /api/login HTTP/1.1"));
        assert_eq!(
            requests[0].1,
            r#"{"password":"secret-password","user":"operator"}"#
        );
        assert!(requests[1].0.starts_with("GET /api/version HTTP/1.1"));
        assert!(requests[2].0.starts_with("GET /api/stats HTTP/1.1"));
        assert!(requests[3].0.starts_with("GET /api/logout HTTP/1.1"));
        assert!(requests[1..].iter().all(|(head, body)| head
            .contains("Cookie: frigate-token=secret.jwt.value")
            && body.is_empty()));
        assert!(requests[1..]
            .iter()
            .all(|(head, _)| !head.contains("secret-password") && !head.contains("operator")));
    }

    #[test]
    fn cookie_and_response_bounds_are_enforced() {
        assert!(extract_session_cookie(&[Header {
            name: "Set-Cookie".to_string(),
            value: "frigate-token=bad value; Path=/".to_string(),
        }])
        .is_err());
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        assert!(matches!(
            decode_http_response(response, 1),
            Err(FrigateError::ResponseTooLarge { limit: 1 })
        ));
    }
}
