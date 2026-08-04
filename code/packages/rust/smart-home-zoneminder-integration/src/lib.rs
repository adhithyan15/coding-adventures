//! Authenticated ZoneMinder NVR and camera health inspection for D23.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::{Zeroize, Zeroizing};
use http1::{parse_response_head, Http1ParseError};
use http_core::{BodyKind, Header};
use serde_json::Value as JsonValue;
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
pub const INTEGRATION_ID: &str = "zoneminder";
pub const PROTOCOL_ID: &str = "zoneminder_http_api";
pub const LOGIN_PATH: &str = "/api/host/login.json";
pub const VERSION_PATH: &str = "/api/host/getVersion.json";
pub const MONITORS_PATH: &str = "/api/monitors.json";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_MONITORS: usize = 512;
const MAX_TOKEN_BYTES: usize = 16 * 1024;
const MAX_VERSION_BYTES: usize = 256;

#[derive(Debug)]
pub enum ZoneMinderError {
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

impl fmt::Display for ZoneMinderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid ZoneMinder input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid ZoneMinder URL: {error}"),
            Self::Io(message) => write!(formatter, "ZoneMinder LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "ZoneMinder TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid ZoneMinder HTTP response: {message}"),
            Self::HttpStatus { operation, status } => {
                write!(formatter, "ZoneMinder {operation} returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "ZoneMinder response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "ZoneMinder response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid ZoneMinder JSON: {error}"),
            Self::MissingField(field) => {
                write!(formatter, "ZoneMinder response is missing {field}")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ZoneMinderError {}

impl From<LocalHttpError> for ZoneMinderError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for ZoneMinderError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for ZoneMinderError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for ZoneMinderError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct ZoneMinderCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl ZoneMinderCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, ZoneMinderError> {
        let username = username.into();
        let password = password.into();
        if username.trim().is_empty() || password.is_empty() {
            return Err(ZoneMinderError::Validation(
                "username and password must not be empty".to_string(),
            ));
        }
        if username.contains('\0') || password.contains('\0') {
            return Err(ZoneMinderError::Validation(
                "credentials contain a NUL byte".to_string(),
            ));
        }
        Ok(Self {
            username: Zeroizing::new(username),
            password: Zeroizing::new(password),
        })
    }
}

impl fmt::Debug for ZoneMinderCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ZoneMinderCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneMinderConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub credential_ref: VaultRef,
    pub timeout: Duration,
}

impl ZoneMinderConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        credential_ref: VaultRef,
    ) -> Result<Self, ZoneMinderError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(ZoneMinderError::MissingField("base URL host"))?;
        let secure = parsed.scheme == "https";
        let test_loopback = parsed.scheme == "http" && is_loopback_host(host);
        if (!secure && !test_loopback)
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || parsed.path.contains(['\r', '\n', '\0'])
        {
            return Err(ZoneMinderError::Validation(
                "base URL must be a credential-free HTTPS origin or path prefix; HTTP is test-only on loopback"
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

    fn endpoint(&self) -> Result<LocalHttpEndpoint, ZoneMinderError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(ZoneMinderError::MissingField("base URL host"))?;
        let scheme = match parsed.scheme.as_str() {
            "https" => LocalHttpScheme::Https,
            "http" if is_loopback_host(host) => LocalHttpScheme::Http,
            _ => {
                return Err(ZoneMinderError::Validation(
                    "ZoneMinder endpoint is not approved".to_string(),
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
        .with_metadata(Metadata::new(
            "http.profile",
            "zoneminder.authenticated-api",
        )))
    }

    fn api_path(&self, path: &str) -> Result<String, ZoneMinderError> {
        let parsed = Url::parse(&self.base_url)?;
        let prefix = parsed.path.trim_end_matches('/');
        Ok(if prefix.is_empty() {
            path.to_string()
        } else {
            format!("{prefix}{path}")
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneMinderMonitor {
    pub id: u64,
    pub name: String,
    pub enabled: bool,
    pub capturing: String,
    pub analysing: String,
    pub recording: String,
    pub status: String,
    pub capture_fps: Option<f64>,
    pub analysis_fps: Option<f64>,
    pub capture_bandwidth: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneMinderSnapshot {
    pub version: String,
    pub api_version: String,
    pub monitors: Vec<ZoneMinderMonitor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneMinderRequestPlans {
    pub login: LocalHttpRequestPlan,
    pub version: LocalHttpRequestPlan,
    pub monitors: LocalHttpRequestPlan,
}

pub trait ZoneMinderTransport {
    fn inspect(
        &mut self,
        plans: &ZoneMinderRequestPlans,
        credentials: &ZoneMinderCredentials,
    ) -> Result<ZoneMinderSnapshot, ZoneMinderError>;
}

pub struct ZoneMinderLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    maximum_response_bytes: usize,
}

impl Default for ZoneMinderLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl ZoneMinderLanTransport {
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
        access_token: Option<&str>,
    ) -> Result<HttpResponse, ZoneMinderError> {
        let request = Zeroizing::new(encode_http_request(plan, body, access_token)?);
        let url = Url::parse(&plan.url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(ZoneMinderError::MissingField("request URL host"))?;
        let port = url
            .effective_port()
            .ok_or(ZoneMinderError::MissingField("request URL port"))?;
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
                    .map_err(|error| ZoneMinderError::Tls(error.to_string()))?;
                write_request(&mut stream, request.as_slice())?;
                let bytes = Zeroizing::new(read_bounded(&mut stream, self.maximum_response_bytes)?);
                stream
                    .close_notify()
                    .map_err(|error| ZoneMinderError::Tls(error.to_string()))?;
                bytes
            }
            _ => {
                return Err(ZoneMinderError::Validation(
                    "ZoneMinder transport requires HTTPS or loopback HTTP".to_string(),
                ))
            }
        };
        decode_http_response(response.as_slice(), self.maximum_response_bytes)
    }
}

impl ZoneMinderTransport for ZoneMinderLanTransport {
    fn inspect(
        &mut self,
        plans: &ZoneMinderRequestPlans,
        credentials: &ZoneMinderCredentials,
    ) -> Result<ZoneMinderSnapshot, ZoneMinderError> {
        let login_body = Zeroizing::new(format!(
            "user={}&pass={}",
            form_encode(credentials.username.as_str()),
            form_encode(credentials.password.as_str())
        ));
        let login = self.request(&plans.login, login_body.as_bytes(), None)?;
        if login.status != 200 {
            return Err(ZoneMinderError::HttpStatus {
                operation: "login",
                status: login.status,
            });
        }
        let access_token = parse_access_token(&login.body)?;
        let version_response = self.request(&plans.version, &[], Some(access_token.as_str()))?;
        if version_response.status != 200 {
            return Err(ZoneMinderError::HttpStatus {
                operation: "version",
                status: version_response.status,
            });
        }
        let (version, api_version) = parse_version(&version_response.body)?;
        if api_version != "2.0" {
            return Err(ZoneMinderError::Validation(format!(
                "ZoneMinder API {api_version} does not provide the required token contract"
            )));
        }
        let monitors_response = self.request(&plans.monitors, &[], Some(access_token.as_str()))?;
        if monitors_response.status != 200 {
            return Err(ZoneMinderError::HttpStatus {
                operation: "monitor inspection",
                status: monitors_response.status,
            });
        }
        Ok(ZoneMinderSnapshot {
            version,
            api_version,
            monitors: parse_monitors(&monitors_response.body)?,
        })
    }
}

pub struct ZoneMinderClient<T> {
    config: ZoneMinderConfig,
    credentials: ZoneMinderCredentials,
    transport: T,
    plans: ZoneMinderRequestPlans,
}

impl<T: ZoneMinderTransport> ZoneMinderClient<T> {
    pub fn new(
        config: ZoneMinderConfig,
        credentials: ZoneMinderCredentials,
        transport: T,
    ) -> Result<Self, ZoneMinderError> {
        let endpoint = config.endpoint()?;
        let timeout_ms = duration_ms(config.timeout);
        let get = |path: String| {
            LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
                .with_accept("application/json")
                .with_timeout_ms(timeout_ms)
                .with_auth(LocalHttpAuth::None)
                .plan(&endpoint, Vec::new())
                .map_err(ZoneMinderError::from)
        };
        let login =
            LocalHttpRequestTemplate::new(LocalHttpMethod::Post, config.api_path(LOGIN_PATH)?)?
                .with_accept("application/json")
                .with_content_type("application/x-www-form-urlencoded")
                .with_timeout_ms(timeout_ms)
                .with_idempotent(false)
                .with_auth(LocalHttpAuth::None)
                .plan(&endpoint, Vec::new())?;
        let plans = ZoneMinderRequestPlans {
            login,
            version: get(config.api_path(VERSION_PATH)?)?,
            monitors: get(config.api_path(MONITORS_PATH)?)?,
        };
        Ok(Self {
            config,
            credentials,
            transport,
            plans,
        })
    }

    pub fn inspect(&mut self) -> Result<ZoneMinderSnapshot, ZoneMinderError> {
        self.transport.inspect(&self.plans, &self.credentials)
    }
}

impl<T> fmt::Debug for ZoneMinderClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ZoneMinderClient")
            .field("config", &self.config)
            .field("credentials", &"[REDACTED]")
            .field("plans", &self.plans)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledZoneMinderCamera {
    pub device_id: DeviceId,
    pub camera_entity_id: EntityId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledZoneMinderNvr {
    pub bridge_id: BridgeId,
    pub cameras: Vec<InstalledZoneMinderCamera>,
}

pub struct ZoneMinderRuntimeIntegration<T> {
    client: ZoneMinderClient<T>,
}

impl<T: ZoneMinderTransport> ZoneMinderRuntimeIntegration<T> {
    pub fn new(client: ZoneMinderClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledZoneMinderNvr, ZoneMinderError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

pub fn paired_discovery_record(
    config: &ZoneMinderConfig,
    snapshot: &ZoneMinderSnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, ZoneMinderError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&config.base_url),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| ZoneMinderError::Validation(error.to_string()))?
    .with_display_name("ZoneMinder NVR")
    .with_address(config.base_url.clone())
    .with_hardware_model("ZoneMinder NVR")
    .with_firmware_version(snapshot.version.clone())
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("zoneminder.protocol", PROTOCOL_ID)
    .with_metadata("zoneminder.api_version", snapshot.api_version.clone())
    .with_metadata(
        "zoneminder.monitor_count",
        snapshot.monitors.len().to_string(),
    ))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &ZoneMinderConfig,
    snapshot: &ZoneMinderSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledZoneMinderNvr, ZoneMinderError> {
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some("ZoneMinder NVR".to_string());
    bridge.firmware_version = Some(snapshot.version.clone());
    bridge.auth_ref = Some(config.credential_ref.clone());
    bridge.health = aggregate_health(&snapshot.monitors);
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("https_endpoint", &config.base_url)?];
    bridge.metadata = vec![
        Metadata::new("zoneminder.transport", "api_v2_access_token"),
        Metadata::new("zoneminder.api_version", snapshot.api_version.clone()),
        Metadata::new(
            "zoneminder.monitor_count",
            snapshot.monitors.len().to_string(),
        ),
    ];
    runtime.upsert_bridge(bridge)?;

    let mut installed = Vec::with_capacity(snapshot.monitors.len());
    for monitor in &snapshot.monitors {
        let device_id = DeviceId::trusted(format!("zoneminder:monitor:{}", monitor.id));
        let camera_entity_id =
            EntityId::trusted(format!("zoneminder:monitor:{}:camera", monitor.id));
        let health = monitor_health(monitor);
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "ZoneMinder".to_string(),
            model: "Managed camera".to_string(),
            name: monitor.name.clone(),
            serial: None,
            firmware_version: None,
            room_id: None,
            entity_ids: vec![camera_entity_id.clone()],
            identifiers: vec![protocol_identifier("monitor_id", &monitor.id.to_string())?],
            health,
            metadata: vec![Metadata::new(
                "zoneminder.monitor_name",
                monitor.name.clone(),
            )],
        })?;
        runtime.upsert_entity(Entity {
            entity_id: camera_entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Camera,
            name: monitor.name.clone(),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("camera.health"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: camera_entity_id.clone(),
                value: monitor_value(monitor),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("zoneminder.protocol", PROTOCOL_ID)],
        })?;
        installed.push(InstalledZoneMinderCamera {
            device_id,
            camera_entity_id,
        });
    }
    Ok(InstalledZoneMinderNvr {
        bridge_id: config.bridge_id.clone(),
        cameras: installed,
    })
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), ZoneMinderError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(ZoneMinderError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_access_token(body: &[u8]) -> Result<Zeroizing<String>, ZoneMinderError> {
    let value = SecretJson(serde_json::from_slice(body)?);
    let object = value
        .0
        .as_object()
        .ok_or(ZoneMinderError::MissingField("login object"))?;
    let api_version = required_text(object.get("apiversion"), "apiversion", 32)?;
    if api_version != "2.0" {
        return Err(ZoneMinderError::Validation(format!(
            "login returned unsupported API version {api_version}"
        )));
    }
    let expires = required_u64(object.get("access_token_expires"), "access_token_expires")?;
    if expires == 0 {
        return Err(ZoneMinderError::Validation(
            "access token is already expired".to_string(),
        ));
    }
    let token = required_text(object.get("access_token"), "access_token", MAX_TOKEN_BYTES)?;
    if token.bytes().any(|byte| byte <= 0x20 || byte == 0x7f) {
        return Err(ZoneMinderError::Validation(
            "login returned an unsafe access token".to_string(),
        ));
    }
    Ok(Zeroizing::new(token))
}

struct SecretJson(JsonValue);

impl Drop for SecretJson {
    fn drop(&mut self) {
        zeroize_json_strings(&mut self.0);
    }
}

fn zeroize_json_strings(value: &mut JsonValue) {
    match value {
        JsonValue::String(value) => value.zeroize(),
        JsonValue::Array(values) => {
            for value in values {
                zeroize_json_strings(value);
            }
        }
        JsonValue::Object(values) => {
            for value in values.values_mut() {
                zeroize_json_strings(value);
            }
        }
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
}

fn parse_version(body: &[u8]) -> Result<(String, String), ZoneMinderError> {
    if body.len() > MAX_VERSION_BYTES {
        return Err(ZoneMinderError::Validation(
            "version exceeds the accepted bound".to_string(),
        ));
    }
    let value: JsonValue = serde_json::from_slice(body)?;
    let object = value
        .as_object()
        .ok_or(ZoneMinderError::MissingField("version object"))?;
    Ok((
        required_text(object.get("version"), "version", 128)?,
        required_text(object.get("apiversion"), "apiversion", 32)?,
    ))
}

fn parse_monitors(body: &[u8]) -> Result<Vec<ZoneMinderMonitor>, ZoneMinderError> {
    let value: JsonValue = serde_json::from_slice(body)?;
    let monitors = value
        .get("monitors")
        .and_then(JsonValue::as_array)
        .ok_or(ZoneMinderError::MissingField("monitors"))?;
    if monitors.len() > MAX_MONITORS {
        return Err(ZoneMinderError::Validation(format!(
            "monitor count exceeds {MAX_MONITORS}"
        )));
    }
    let mut parsed = Vec::with_capacity(monitors.len());
    let mut ids = BTreeSet::new();
    for entry in monitors {
        let entry = entry
            .as_object()
            .ok_or(ZoneMinderError::MissingField("monitors[]"))?;
        let monitor = entry
            .get("Monitor")
            .and_then(JsonValue::as_object)
            .ok_or(ZoneMinderError::MissingField("monitors[].Monitor"))?;
        let status = entry
            .get("Monitor_Status")
            .and_then(JsonValue::as_object)
            .ok_or(ZoneMinderError::MissingField("monitors[].Monitor_Status"))?;
        let id = required_u64(monitor.get("Id"), "Monitor.Id")?;
        if id == 0 || !ids.insert(id) {
            return Err(ZoneMinderError::Validation(
                "monitor IDs must be positive and unique".to_string(),
            ));
        }
        let monitor_status = required_text(status.get("Status"), "Monitor_Status.Status", 64)?;
        let normalized_status = monitor_status.to_ascii_lowercase();
        if !matches!(
            normalized_status.as_str(),
            "connected" | "unknown" | "notrunning" | "running" | "nosignal" | "signal"
        ) {
            return Err(ZoneMinderError::Validation(format!(
                "unknown monitor status `{monitor_status}`"
            )));
        }
        parsed.push(ZoneMinderMonitor {
            id,
            name: required_text(monitor.get("Name"), "Monitor.Name", 256)?,
            enabled: required_boolish(monitor.get("Enabled"), "Monitor.Enabled")?,
            capturing: required_text(monitor.get("Capturing"), "Monitor.Capturing", 32)?,
            analysing: required_text(monitor.get("Analysing"), "Monitor.Analysing", 32)?,
            recording: required_text(monitor.get("Recording"), "Monitor.Recording", 32)?,
            status: monitor_status,
            capture_fps: optional_nonnegative_number(status.get("CaptureFPS"), "CaptureFPS")?,
            analysis_fps: optional_nonnegative_number(status.get("AnalysisFPS"), "AnalysisFPS")?,
            capture_bandwidth: optional_u64(status.get("CaptureBandwidth"), "CaptureBandwidth")?,
        });
    }
    parsed.sort_by_key(|monitor| monitor.id);
    Ok(parsed)
}

fn required_text(
    value: Option<&JsonValue>,
    field: &'static str,
    maximum: usize,
) -> Result<String, ZoneMinderError> {
    let value = value
        .and_then(JsonValue::as_str)
        .ok_or(ZoneMinderError::MissingField(field))?
        .trim()
        .to_string();
    if value.is_empty() || value.len() > maximum || value.contains(['\r', '\n', '\0']) {
        Err(ZoneMinderError::Validation(format!(
            "{field} is empty, oversized, or unsafe"
        )))
    } else {
        Ok(value)
    }
}

fn optional_nonnegative_number(
    value: Option<&JsonValue>,
    field: &'static str,
) -> Result<Option<f64>, ZoneMinderError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => {
            let parsed = match value {
                JsonValue::Number(value) => value.as_f64(),
                JsonValue::String(value) => value.parse::<f64>().ok(),
                _ => None,
            }
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or_else(|| {
                ZoneMinderError::Validation(format!("{field} must be a finite non-negative number"))
            })?;
            Ok(Some(parsed))
        }
    }
}

fn required_u64(value: Option<&JsonValue>, field: &'static str) -> Result<u64, ZoneMinderError> {
    match value {
        Some(JsonValue::Number(value)) => value.as_u64(),
        Some(JsonValue::String(value)) => value.parse::<u64>().ok(),
        _ => None,
    }
    .ok_or_else(|| ZoneMinderError::Validation(format!("{field} must be an unsigned integer")))
}

fn optional_u64(
    value: Option<&JsonValue>,
    field: &'static str,
) -> Result<Option<u64>, ZoneMinderError> {
    match value {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => required_u64(Some(value), field).map(Some),
    }
}

fn required_boolish(
    value: Option<&JsonValue>,
    field: &'static str,
) -> Result<bool, ZoneMinderError> {
    match value {
        Some(JsonValue::Bool(value)) => Ok(*value),
        Some(JsonValue::Number(value)) if value.as_u64() == Some(0) => Ok(false),
        Some(JsonValue::Number(value)) if value.as_u64() == Some(1) => Ok(true),
        Some(JsonValue::String(value)) if value == "0" => Ok(false),
        Some(JsonValue::String(value)) if value == "1" => Ok(true),
        _ => Err(ZoneMinderError::Validation(format!(
            "{field} must be boolean or 0/1"
        ))),
    }
}

fn monitor_health(monitor: &ZoneMinderMonitor) -> Health {
    let status = monitor.status.to_ascii_lowercase();
    if !monitor.enabled
        || monitor.capturing.eq_ignore_ascii_case("none")
        || matches!(status.as_str(), "notrunning" | "nosignal")
    {
        Health::Offline
    } else if status == "unknown" || monitor.capture_fps == Some(0.0) {
        Health::Degraded
    } else {
        Health::Online
    }
}

fn aggregate_health(monitors: &[ZoneMinderMonitor]) -> Health {
    if monitors.is_empty() {
        return Health::Degraded;
    }
    let health = monitors.iter().map(monitor_health).collect::<Vec<_>>();
    if health.iter().all(|value| *value == Health::Offline) {
        Health::Offline
    } else if health.iter().any(|value| *value != Health::Online) {
        Health::Degraded
    } else {
        Health::Online
    }
}

fn monitor_value(monitor: &ZoneMinderMonitor) -> Value {
    let mut fields = vec![
        ("monitor_id".to_string(), Value::Integer(monitor.id as i64)),
        ("enabled".to_string(), Value::Bool(monitor.enabled)),
        (
            "capturing".to_string(),
            Value::Text(monitor.capturing.clone()),
        ),
        (
            "analysing".to_string(),
            Value::Text(monitor.analysing.clone()),
        ),
        (
            "recording".to_string(),
            Value::Text(monitor.recording.clone()),
        ),
        ("status".to_string(), Value::Text(monitor.status.clone())),
    ];
    if let Some(capture_fps) = monitor.capture_fps {
        fields.push(("capture_fps".to_string(), Value::Number(capture_fps)));
    }
    if let Some(analysis_fps) = monitor.analysis_fps {
        fields.push(("analysis_fps".to_string(), Value::Number(analysis_fps)));
    }
    if let Some(capture_bandwidth) = monitor.capture_bandwidth {
        fields.push((
            "capture_bandwidth_bytes_per_second".to_string(),
            Value::Integer(i64::try_from(capture_bandwidth).unwrap_or(i64::MAX)),
        ));
    }
    Value::Object(fields)
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, ZoneMinderError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| ZoneMinderError::Validation(error.to_string()))
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

fn form_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

fn encode_http_request(
    plan: &LocalHttpRequestPlan,
    body: &[u8],
    access_token: Option<&str>,
) -> Result<Vec<u8>, ZoneMinderError> {
    let url = Url::parse(&plan.url)?;
    let host = url
        .host
        .as_deref()
        .ok_or(ZoneMinderError::MissingField("request URL host"))?;
    let port = url
        .effective_port()
        .ok_or(ZoneMinderError::MissingField("request URL port"))?;
    let mut target = if url.path.is_empty() {
        "/".to_string()
    } else {
        url.path.clone()
    };
    if let Some(access_token) = access_token {
        if access_token.is_empty()
            || access_token.len() > MAX_TOKEN_BYTES
            || access_token
                .bytes()
                .any(|byte| byte <= 0x20 || byte == 0x7f)
        {
            return Err(ZoneMinderError::Validation(
                "access token is empty, oversized, or unsafe".to_string(),
            ));
        }
        target.push_str("?token=");
        target.push_str(&form_encode(access_token));
    }
    if host.contains(['\r', '\n', '\0']) || target.contains(['\r', '\n', '\0']) {
        return Err(ZoneMinderError::Validation(
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
            || header.name.eq_ignore_ascii_case("Authorization")
        {
            continue;
        }
        if header.name.contains(['\r', '\n', '\0']) || header.value.contains(['\r', '\n', '\0']) {
            return Err(ZoneMinderError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    request.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    request.extend_from_slice(body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, ZoneMinderError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| ZoneMinderError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| ZoneMinderError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| ZoneMinderError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(ZoneMinderError::Io(
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

fn write_request(writer: &mut dyn Write, request: &[u8]) -> Result<(), ZoneMinderError> {
    writer
        .write_all(request)
        .map_err(|error| ZoneMinderError::Io(error.to_string()))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, ZoneMinderError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| ZoneMinderError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(ZoneMinderError::ResponseTooLarge { limit: maximum });
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

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<HttpResponse, ZoneMinderError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| ZoneMinderError::Http(error.to_string()))?;
    let status = parsed.head.status;
    let mut headers = parsed.head.headers;
    let input = &bytes[parsed.body_offset..];
    let body = match (|| {
        let body = match parsed.body_kind {
            BodyKind::None => Vec::new(),
            BodyKind::ContentLength(expected) => {
                if input.len() < expected {
                    return Err(ZoneMinderError::TruncatedBody {
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
            return Err(ZoneMinderError::ResponseTooLarge { limit: maximum });
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

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, ZoneMinderError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input
            .get(cursor..)
            .and_then(|tail| tail.windows(2).position(|window| window == b"\r\n"))
            .ok_or_else(|| ZoneMinderError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| ZoneMinderError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| ZoneMinderError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(ZoneMinderError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| ZoneMinderError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(ZoneMinderError::Http("truncated chunk".to_string()));
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

    fn credentials() -> ZoneMinderCredentials {
        ZoneMinderCredentials::new("operator name", "secret&password").unwrap()
    }

    fn config(port: u16) -> ZoneMinderConfig {
        ZoneMinderConfig::new(
            BridgeId::trusted("zoneminder.test"),
            format!("http://127.0.0.1:{port}/zm"),
            VaultRef::trusted("vault://zoneminder/test"),
        )
        .unwrap()
    }

    fn snapshot() -> ZoneMinderSnapshot {
        ZoneMinderSnapshot {
            version: "1.36.33".to_string(),
            api_version: "2.0".to_string(),
            monitors: vec![
                ZoneMinderMonitor {
                    id: 1,
                    name: "Back".to_string(),
                    enabled: true,
                    capturing: "Always".to_string(),
                    analysing: "Always".to_string(),
                    recording: "OnMotion".to_string(),
                    status: "NoSignal".to_string(),
                    capture_fps: Some(0.0),
                    analysis_fps: Some(0.0),
                    capture_bandwidth: Some(0),
                },
                ZoneMinderMonitor {
                    id: 2,
                    name: "Front".to_string(),
                    enabled: true,
                    capturing: "Always".to_string(),
                    analysing: "Always".to_string(),
                    recording: "OnMotion".to_string(),
                    status: "Connected".to_string(),
                    capture_fps: Some(5.0),
                    analysis_fps: Some(1.67),
                    capture_bandwidth: Some(52_095),
                },
            ],
        }
    }

    #[derive(Debug)]
    struct FixedTransport {
        snapshot: ZoneMinderSnapshot,
        calls: Arc<AtomicUsize>,
    }

    impl ZoneMinderTransport for FixedTransport {
        fn inspect(
            &mut self,
            _plans: &ZoneMinderRequestPlans,
            _credentials: &ZoneMinderCredentials,
        ) -> Result<ZoneMinderSnapshot, ZoneMinderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.snapshot.clone())
        }
    }

    fn authorize(runtime: &mut SmartHomeRuntime, principal: AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:zoneminder-test"),
                principal,
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn config_requires_https_outside_loopback_and_allows_path_prefix() {
        assert!(ZoneMinderConfig::new(
            BridgeId::trusted("zoneminder.bad"),
            "http://192.0.2.10/zm",
            VaultRef::trusted("vault://zoneminder/bad")
        )
        .is_err());
        let config = ZoneMinderConfig::new(
            BridgeId::trusted("zoneminder.good"),
            "https://zoneminder.home/zm",
            VaultRef::trusted("vault://zoneminder/good"),
        )
        .unwrap();
        assert_eq!(
            config.api_path(MONITORS_PATH).unwrap(),
            "/zm/api/monitors.json"
        );
        assert!(ZoneMinderConfig::new(
            BridgeId::trusted("zoneminder.embedded"),
            "https://operator:secret@zoneminder.home/zm",
            VaultRef::trusted("vault://zoneminder/embedded")
        )
        .is_err());
    }

    #[test]
    fn credentials_and_client_debug_are_redacted() {
        assert_eq!(
            format!("{:?}", credentials()),
            "ZoneMinderCredentials([REDACTED])"
        );
        let client = ZoneMinderClient::new(
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
        assert!(!debug.contains("secret"));
        assert!(!debug.contains("token="));
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = ZoneMinderClient::new(
            config(8971),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = ZoneMinderRuntimeIntegration::new(client);
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
    fn authorized_snapshot_installs_confirmed_monitor_health() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = ZoneMinderClient::new(
            config(8971),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = ZoneMinderRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:allowed");
        authorize(&mut runtime, principal.clone());
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 2_000)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(installed.cameras.len(), 2);
        assert_eq!(
            runtime
                .registry()
                .device(&installed.cameras[0].device_id)
                .unwrap()
                .health,
            Health::Offline
        );
        let front = runtime
            .registry()
            .entity(&installed.cameras[1].camera_entity_id)
            .unwrap();
        assert_eq!(front.kind, EntityKind::Camera);
        assert_eq!(
            front.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
    }

    #[test]
    fn parser_orders_monitors_and_rejects_duplicate_ids_or_unknown_status() {
        let body = br#"{"monitors":[{"Monitor":{"Id":"2","Name":"Front","Enabled":"1","Capturing":"Always","Analysing":"Always","Recording":"OnMotion"},"Monitor_Status":{"MonitorId":"2","Status":"Connected","CaptureFPS":"5.00","AnalysisFPS":"1.67","CaptureBandwidth":"52095"}},{"Monitor":{"Id":"1","Name":"Back","Enabled":"0","Capturing":"None","Analysing":"None","Recording":"None"},"Monitor_Status":{"MonitorId":"1","Status":"NotRunning","CaptureFPS":"0","AnalysisFPS":"0","CaptureBandwidth":"0"}}]}"#;
        let monitors = parse_monitors(body).unwrap();
        assert_eq!(
            monitors
                .iter()
                .map(|monitor| monitor.id)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        assert_eq!(monitor_health(&monitors[0]), Health::Offline);
        assert_eq!(monitor_health(&monitors[1]), Health::Online);

        let duplicate = br#"{"monitors":[{"Monitor":{"Id":"1","Name":"A","Enabled":"1","Capturing":"Always","Analysing":"Always","Recording":"Always"},"Monitor_Status":{"Status":"Signal"}},{"Monitor":{"Id":"1","Name":"B","Enabled":"1","Capturing":"Always","Analysing":"Always","Recording":"Always"},"Monitor_Status":{"Status":"Signal"}}]}"#;
        assert!(parse_monitors(duplicate).is_err());
        let unknown = br#"{"monitors":[{"Monitor":{"Id":"1","Name":"A","Enabled":"1","Capturing":"Always","Analysing":"Always","Recording":"Always"},"Monitor_Status":{"Status":"Maybe"}}]}"#;
        assert!(parse_monitors(unknown).is_err());
    }

    #[test]
    fn loopback_transport_uses_exact_api_v2_token_flow() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            r#"{"access_token":"secret.jwt.value","access_token_expires":3600,"refresh_token":"refresh.secret","refresh_token_expires":86400,"version":"1.36.33","apiversion":"2.0"}"#,
            r#"{"version":"1.36.33","apiversion":"2.0"}"#,
            r#"{"monitors":[{"Monitor":{"Id":"2","Name":"Front","Enabled":"1","Capturing":"Always","Analysing":"Always","Recording":"OnMotion"},"Monitor_Status":{"MonitorId":"2","Status":"Connected","CaptureFPS":"5.00","AnalysisFPS":"1.67","CaptureBandwidth":"52095"}}]}"#,
        ];
        let handle = thread::spawn(move || {
            for body in responses {
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
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                reader.get_mut().write_all(reply.as_bytes()).unwrap();
            }
        });

        let mut client = ZoneMinderClient::new(
            config(port),
            credentials(),
            ZoneMinderLanTransport::default(),
        )
        .unwrap();
        let observed = client.inspect().unwrap();
        handle.join().unwrap();
        assert_eq!(observed.version, "1.36.33");
        assert_eq!(observed.monitors[0].name, "Front");
        assert!(!format!("{observed:?}").contains("secret.jwt.value"));

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0]
            .0
            .starts_with("POST /zm/api/host/login.json HTTP/1.1"));
        assert!(requests[0]
            .0
            .contains("Content-Type: application/x-www-form-urlencoded"));
        assert_eq!(requests[0].1, "user=operator%20name&pass=secret%26password");
        assert!(requests[1]
            .0
            .starts_with("GET /zm/api/host/getVersion.json?token=secret.jwt.value HTTP/1.1"));
        assert!(requests[2]
            .0
            .starts_with("GET /zm/api/monitors.json?token=secret.jwt.value HTTP/1.1"));
        assert!(requests[1..].iter().all(|(_, body)| body.is_empty()));
        assert!(requests[1..].iter().all(|(head, _)| {
            !head.contains("secret%26password") && !head.contains("operator%20name")
        }));
    }

    #[test]
    fn token_and_response_bounds_are_enforced() {
        let bad_login =
            br#"{"access_token":"bad token","access_token_expires":3600,"apiversion":"2.0"}"#;
        assert!(parse_access_token(bad_login).is_err());
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        assert!(matches!(
            decode_http_response(response, 1),
            Err(ZoneMinderError::ResponseTooLarge { limit: 1 })
        ));
    }
}
