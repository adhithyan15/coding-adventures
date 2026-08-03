//! Authenticated Synology Surveillance Station camera health inspection for D23.

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
use std::collections::BTreeSet;
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use tls_platform::{default_connector, TlsConfig, TlsConnector};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.1.0";
pub const INTEGRATION_ID: &str = "synology-surveillance";
pub const PROTOCOL_ID: &str = "synology_surveillance_webapi";
pub const API_INFO_PATH: &str = "/webapi/query.cgi?api=SYNO.API.Info&method=Query&version=1&query=SYNO.API.Auth%2CSYNO.SurveillanceStation.Info%2CSYNO.SurveillanceStation.Camera";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_CAMERAS: usize = 1_024;
const MAX_SECRET_BYTES: usize = 8 * 1024;
const MAX_TEXT_BYTES: usize = 1_024;
const AUTH_API: &str = "SYNO.API.Auth";
const INFO_API: &str = "SYNO.SurveillanceStation.Info";
const CAMERA_API: &str = "SYNO.SurveillanceStation.Camera";

#[derive(Debug)]
pub enum SynologyError {
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
    Api {
        operation: &'static str,
        code: i64,
    },
    UnsupportedApi {
        api: &'static str,
        required: u64,
        minimum: u64,
        maximum: u64,
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

impl fmt::Display for SynologyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Synology input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Synology URL: {error}"),
            Self::Io(message) => write!(formatter, "Synology LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "Synology TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Synology HTTP response: {message}"),
            Self::HttpStatus { operation, status } => {
                write!(formatter, "Synology {operation} returned HTTP {status}")
            }
            Self::Api { operation, code } => {
                write!(formatter, "Synology {operation} returned API error {code}")
            }
            Self::UnsupportedApi {
                api,
                required,
                minimum,
                maximum,
            } => write!(
                formatter,
                "Synology API {api} requires version {required}, device advertises {minimum}..={maximum}"
            ),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Synology response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Synology response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Synology JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "Synology response is missing {field}"),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SynologyError {}

impl From<LocalHttpError> for SynologyError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for SynologyError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for SynologyError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for SynologyError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct SynologyCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl SynologyCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, SynologyError> {
        let username = username.into();
        let password = password.into();
        if username.trim().is_empty() || password.is_empty() {
            return Err(SynologyError::Validation(
                "username and password must not be empty".to_string(),
            ));
        }
        if username.len() > MAX_SECRET_BYTES
            || password.len() > MAX_SECRET_BYTES
            || username.contains('\0')
            || password.contains('\0')
        {
            return Err(SynologyError::Validation(
                "credentials exceed bounds or contain a NUL byte".to_string(),
            ));
        }
        Ok(Self {
            username: Zeroizing::new(username),
            password: Zeroizing::new(password),
        })
    }
}

impl fmt::Debug for SynologyCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SynologyCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologyConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub credential_ref: VaultRef,
    pub timeout: Duration,
}

impl SynologyConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        credential_ref: VaultRef,
    ) -> Result<Self, SynologyError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(SynologyError::MissingField("base URL host"))?;
        let secure = parsed.scheme == "https";
        let test_loopback = parsed.scheme == "http" && is_loopback_host(host);
        if (!secure && !test_loopback)
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(SynologyError::Validation(
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

    fn endpoint(&self) -> Result<LocalHttpEndpoint, SynologyError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(SynologyError::MissingField("base URL host"))?;
        let scheme = match parsed.scheme.as_str() {
            "https" => LocalHttpScheme::Https,
            "http" if is_loopback_host(host) => LocalHttpScheme::Http,
            _ => {
                return Err(SynologyError::Validation(
                    "Synology endpoint is not approved".to_string(),
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
            "synology.surveillance.webapi",
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologySurveillanceInfo {
    pub version: String,
    pub camera_count: u64,
    pub maximum_camera_count: Option<u64>,
    pub user_privilege: Option<u64>,
    pub allow_snapshot: Option<bool>,
    pub allow_manual_recording: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologyCamera {
    pub id: u64,
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub channel: Option<String>,
    pub status: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologySnapshot {
    pub info: SynologySurveillanceInfo,
    pub cameras: Vec<SynologyCamera>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologyRequestPlans {
    pub api_info: LocalHttpRequestPlan,
    endpoint: LocalHttpEndpoint,
    timeout_ms: u64,
}

pub trait SynologyTransport {
    fn inspect(
        &mut self,
        plans: &SynologyRequestPlans,
        credentials: &SynologyCredentials,
    ) -> Result<SynologySnapshot, SynologyError>;
}

pub struct SynologyLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    maximum_response_bytes: usize,
}

impl Default for SynologyLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl SynologyLanTransport {
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
        override_url: Option<&str>,
        body: &[u8],
    ) -> Result<HttpResponse, SynologyError> {
        let request = Zeroizing::new(encode_http_request(plan, override_url, body)?);
        let url = Url::parse(override_url.unwrap_or(&plan.url))?;
        let host = url
            .host
            .as_deref()
            .ok_or(SynologyError::MissingField("request URL host"))?;
        let port = url
            .effective_port()
            .ok_or(SynologyError::MissingField("request URL port"))?;
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
                    .map_err(|error| SynologyError::Tls(error.to_string()))?;
                write_request(&mut stream, request.as_slice())?;
                let bytes = Zeroizing::new(read_bounded(&mut stream, self.maximum_response_bytes)?);
                stream
                    .close_notify()
                    .map_err(|error| SynologyError::Tls(error.to_string()))?;
                bytes
            }
            _ => {
                return Err(SynologyError::Validation(
                    "Synology transport requires HTTPS or loopback HTTP".to_string(),
                ))
            }
        };
        decode_http_response(response.as_slice(), self.maximum_response_bytes)
    }

    fn api_request(
        &mut self,
        plans: &SynologyRequestPlans,
        method: LocalHttpMethod,
        path: &str,
        parameters: &[(&str, &str)],
        body: &[u8],
        operation: &'static str,
    ) -> Result<JsonValue, SynologyError> {
        let plan = api_plan(plans, method, path, !body.is_empty())?;
        let url = api_url(&plans.endpoint, path, parameters)?;
        let response = self.request(&plan, Some(url.as_str()), body)?;
        if response.status != 200 {
            return Err(SynologyError::HttpStatus {
                operation,
                status: response.status,
            });
        }
        parse_api_data(&response.body, operation)
    }
}

impl SynologyTransport for SynologyLanTransport {
    fn inspect(
        &mut self,
        plans: &SynologyRequestPlans,
        credentials: &SynologyCredentials,
    ) -> Result<SynologySnapshot, SynologyError> {
        let response = self.request(&plans.api_info, None, &[])?;
        if response.status != 200 {
            return Err(SynologyError::HttpStatus {
                operation: "API discovery",
                status: response.status,
            });
        }
        let catalog = parse_api_catalog(&response.body)?;

        let login_body = form_encode(&[
            ("api", AUTH_API),
            ("method", "login"),
            ("version", &catalog.auth.version.to_string()),
            ("account", credentials.username.as_str()),
            ("passwd", credentials.password.as_str()),
            ("session", "SurveillanceStation"),
            ("format", "sid"),
            ("enable_syno_token", "yes"),
        ]);
        let login = self.api_request(
            plans,
            LocalHttpMethod::Post,
            &catalog.auth.path,
            &[],
            login_body.as_bytes(),
            "login",
        )?;
        let session = parse_session(login)?;

        let result = (|| {
            let mut common = vec![
                ("_sid", session.sid.as_str()),
                (
                    "SynoToken",
                    session
                        .synotoken
                        .as_deref()
                        .map_or("", |value| value.as_str()),
                ),
            ];
            if session.synotoken.is_none() {
                common.pop();
            }
            let info_version = catalog.info.version.to_string();
            let mut info_parameters = vec![
                ("api", INFO_API),
                ("method", "GetInfo"),
                ("version", info_version.as_str()),
            ];
            info_parameters.extend(common.iter().copied());
            let info = self.api_request(
                plans,
                LocalHttpMethod::Get,
                &catalog.info.path,
                &info_parameters,
                &[],
                "package info",
            )?;
            let info = parse_surveillance_info(info)?;

            let camera_version = catalog.camera.version.to_string();
            let limit = MAX_CAMERAS.to_string();
            let mut camera_parameters = vec![
                ("api", CAMERA_API),
                ("method", "List"),
                ("version", camera_version.as_str()),
                ("offset", "0"),
                ("limit", limit.as_str()),
                ("basic", "true"),
                ("streamInfo", "false"),
                ("blPrivilege", "true"),
                ("privCamType", "1"),
                ("blIncludeDeletedCam", "false"),
            ];
            camera_parameters.extend(common.iter().copied());
            let cameras = self.api_request(
                plans,
                LocalHttpMethod::Get,
                &catalog.camera.path,
                &camera_parameters,
                &[],
                "camera list",
            )?;
            Ok(SynologySnapshot {
                info,
                cameras: parse_cameras(cameras)?,
            })
        })();

        let auth_version = catalog.auth.version.to_string();
        let mut logout_parameters = vec![
            ("api", AUTH_API),
            ("method", "logout"),
            ("version", auth_version.as_str()),
            ("session", "SurveillanceStation"),
            ("_sid", session.sid.as_str()),
        ];
        if let Some(token) = session.synotoken.as_deref() {
            logout_parameters.push(("SynoToken", token));
        }
        let logout = self.api_request(
            plans,
            LocalHttpMethod::Get,
            &catalog.auth.path,
            &logout_parameters,
            &[],
            "logout",
        );
        match (result, logout) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(snapshot), Ok(_)) => Ok(snapshot),
        }
    }
}

pub struct SynologyClient<T> {
    config: SynologyConfig,
    credentials: SynologyCredentials,
    transport: T,
    plans: SynologyRequestPlans,
}

impl<T: SynologyTransport> SynologyClient<T> {
    pub fn new(
        config: SynologyConfig,
        credentials: SynologyCredentials,
        transport: T,
    ) -> Result<Self, SynologyError> {
        let endpoint = config.endpoint()?;
        let timeout_ms = duration_ms(config.timeout);
        let api_info = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, API_INFO_PATH)?
            .with_accept("application/json")
            .with_timeout_ms(timeout_ms)
            .with_auth(LocalHttpAuth::None)
            .plan(&endpoint, Vec::new())?;
        Ok(Self {
            config,
            credentials,
            transport,
            plans: SynologyRequestPlans {
                api_info,
                endpoint,
                timeout_ms,
            },
        })
    }

    pub fn inspect(&mut self) -> Result<SynologySnapshot, SynologyError> {
        self.transport.inspect(&self.plans, &self.credentials)
    }
}

impl<T> fmt::Debug for SynologyClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SynologyClient")
            .field("config", &self.config)
            .field("credentials", &"[REDACTED]")
            .field("plans", &self.plans)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledSynologyCamera {
    pub device_id: DeviceId,
    pub camera_entity_id: EntityId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledSynologyNvr {
    pub bridge_id: BridgeId,
    pub cameras: Vec<InstalledSynologyCamera>,
}

pub struct SynologyRuntimeIntegration<T> {
    client: SynologyClient<T>,
}

impl<T: SynologyTransport> SynologyRuntimeIntegration<T> {
    pub fn new(client: SynologyClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledSynologyNvr, SynologyError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

pub fn paired_discovery_record(
    config: &SynologyConfig,
    snapshot: &SynologySnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, SynologyError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&config.base_url),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| SynologyError::Validation(error.to_string()))?
    .with_display_name("Synology Surveillance Station")
    .with_address(config.base_url.clone())
    .with_hardware_model("Synology NAS")
    .with_firmware_version(snapshot.info.version.clone())
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("synology.protocol", PROTOCOL_ID)
    .with_metadata("synology.camera_count", snapshot.cameras.len().to_string()))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &SynologyConfig,
    snapshot: &SynologySnapshot,
    observed_at_ms: u64,
) -> Result<InstalledSynologyNvr, SynologyError> {
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some("Synology NAS".to_string());
    bridge.firmware_version = Some(snapshot.info.version.clone());
    bridge.auth_ref = Some(config.credential_ref.clone());
    bridge.health = aggregate_health(&snapshot.cameras);
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("https_endpoint", &config.base_url)?];
    bridge.metadata = vec![
        Metadata::new("synology.transport", "sid_session"),
        Metadata::new("synology.camera_count", snapshot.cameras.len().to_string()),
    ];
    if let Some(privilege) = snapshot.info.user_privilege {
        bridge.metadata.push(Metadata::new(
            "synology.user_privilege",
            privilege.to_string(),
        ));
    }
    runtime.upsert_bridge(bridge)?;

    let mut installed = Vec::with_capacity(snapshot.cameras.len());
    for camera in &snapshot.cameras {
        let device_id = DeviceId::trusted(format!("synology-surveillance:{}", camera.id));
        let camera_entity_id =
            EntityId::trusted(format!("synology-surveillance:{}:camera", camera.id));
        let health = camera_health(camera);
        let mut metadata = vec![Metadata::new("synology.camera_id", camera.id.to_string())];
        if let Some(channel) = &camera.channel {
            metadata.push(Metadata::new("synology.channel", channel.clone()));
        }
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: camera.vendor.clone(),
            model: camera.model.clone(),
            name: camera.name.clone(),
            serial: None,
            firmware_version: None,
            room_id: None,
            entity_ids: vec![camera_entity_id.clone()],
            identifiers: vec![protocol_identifier("camera_id", &camera.id.to_string())?],
            health,
            metadata,
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
            metadata: vec![Metadata::new("synology.protocol", PROTOCOL_ID)],
        })?;
        installed.push(InstalledSynologyCamera {
            device_id,
            camera_entity_id,
        });
    }
    Ok(InstalledSynologyNvr {
        bridge_id: config.bridge_id.clone(),
        cameras: installed,
    })
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), SynologyError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(SynologyError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiDescription {
    path: String,
    version: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiCatalog {
    auth: ApiDescription,
    info: ApiDescription,
    camera: ApiDescription,
}

struct Session {
    sid: Zeroizing<String>,
    synotoken: Option<Zeroizing<String>>,
}

struct SecretJson(JsonValue);

impl Drop for SecretJson {
    fn drop(&mut self) {
        zeroize_json(&mut self.0);
    }
}

fn zeroize_json(value: &mut JsonValue) {
    match value {
        JsonValue::String(value) => value.zeroize(),
        JsonValue::Array(values) => values.iter_mut().for_each(zeroize_json),
        JsonValue::Object(values) => values.values_mut().for_each(zeroize_json),
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => {}
    }
}

fn parse_api_catalog(body: &[u8]) -> Result<ApiCatalog, SynologyError> {
    let data = parse_api_data(body, "API discovery")?;
    let object = data
        .as_object()
        .ok_or(SynologyError::MissingField("API discovery data"))?;
    Ok(ApiCatalog {
        auth: parse_api_description(object, AUTH_API, 3, 6)?,
        info: parse_api_description(object, INFO_API, 1, 5)?,
        camera: parse_api_description(object, CAMERA_API, 9, 9)?,
    })
}

fn parse_api_description(
    object: &JsonMap<String, JsonValue>,
    api: &'static str,
    required: u64,
    supported: u64,
) -> Result<ApiDescription, SynologyError> {
    let description = object
        .get(api)
        .and_then(JsonValue::as_object)
        .ok_or(SynologyError::MissingField("API description"))?;
    let minimum = required_u64(description, "minVersion")?;
    let maximum = required_u64(description, "maxVersion")?;
    let version = maximum.min(supported);
    if minimum > maximum || version < required || version < minimum {
        return Err(SynologyError::UnsupportedApi {
            api,
            required,
            minimum,
            maximum,
        });
    }
    let path = required_text(description, "path")?;
    validate_api_path(&path)?;
    Ok(ApiDescription { path, version })
}

fn parse_session(data: JsonValue) -> Result<Session, SynologyError> {
    let data = SecretJson(data);
    let object = data
        .0
        .as_object()
        .ok_or(SynologyError::MissingField("login data"))?;
    let sid = required_text(object, "sid")?;
    validate_secret("sid", &sid)?;
    let synotoken = match object.get("synotoken") {
        None | Some(JsonValue::Null) => None,
        Some(JsonValue::String(value)) => {
            validate_secret("synotoken", value)?;
            Some(Zeroizing::new(value.clone()))
        }
        Some(_) => {
            return Err(SynologyError::Validation(
                "synotoken must be a string".to_string(),
            ))
        }
    };
    Ok(Session {
        sid: Zeroizing::new(sid),
        synotoken,
    })
}

fn parse_surveillance_info(data: JsonValue) -> Result<SynologySurveillanceInfo, SynologyError> {
    let object = data
        .as_object()
        .ok_or(SynologyError::MissingField("package info data"))?;
    let version = object
        .get("version")
        .and_then(JsonValue::as_object)
        .ok_or(SynologyError::MissingField("version"))?;
    let major = required_u64(version, "major")?;
    let minor = required_u64(version, "minor")?;
    let build = required_u64(version, "build")?;
    Ok(SynologySurveillanceInfo {
        version: format!("{major}.{minor}-{build}"),
        camera_count: required_u64(object, "cameraNumber")?,
        maximum_camera_count: optional_u64(object, "maxCameraSupport")?,
        user_privilege: optional_u64(object, "userPriv")?,
        allow_snapshot: optional_bool(object, "allowSnapshot")?,
        allow_manual_recording: optional_bool(object, "allowManualRec")?,
    })
}

fn parse_cameras(data: JsonValue) -> Result<Vec<SynologyCamera>, SynologyError> {
    let object = data
        .as_object()
        .ok_or(SynologyError::MissingField("camera list data"))?;
    let total = required_u64(object, "total")?;
    if total > MAX_CAMERAS as u64 {
        return Err(SynologyError::Validation(format!(
            "camera count {total} exceeds bounded limit {MAX_CAMERAS}"
        )));
    }
    let values = object
        .get("cameras")
        .and_then(JsonValue::as_array)
        .ok_or(SynologyError::MissingField("cameras"))?;
    if values.len() > MAX_CAMERAS || values.len() as u64 > total {
        return Err(SynologyError::Validation(
            "camera list exceeds its advertised total".to_string(),
        ));
    }
    let mut cameras = Vec::with_capacity(values.len());
    let mut ids = BTreeSet::new();
    for value in values {
        let camera = value
            .as_object()
            .ok_or(SynologyError::MissingField("camera object"))?;
        let id = required_u64(camera, "id")?;
        if id == 0 || !ids.insert(id) {
            return Err(SynologyError::Validation(
                "camera IDs must be non-zero and unique".to_string(),
            ));
        }
        let status = required_u64(camera, "status")?;
        if !(1..=18).contains(&status) {
            return Err(SynologyError::Validation(format!(
                "unknown camera status {status}"
            )));
        }
        let name = match (camera.get("name"), camera.get("newName")) {
            (Some(JsonValue::String(name)), _) | (_, Some(JsonValue::String(name))) => {
                bounded_text("camera name", name)?
            }
            _ => return Err(SynologyError::MissingField("camera name")),
        };
        cameras.push(SynologyCamera {
            id,
            name,
            vendor: required_text(camera, "vendor")?,
            model: required_text(camera, "model")?,
            channel: optional_text(camera, "channel")?,
            status: status as u8,
        });
    }
    cameras.sort_by_key(|camera| camera.id);
    Ok(cameras)
}

fn parse_api_data(body: &[u8], operation: &'static str) -> Result<JsonValue, SynologyError> {
    let value: JsonValue = serde_json::from_slice(body)?;
    let object = value
        .as_object()
        .ok_or(SynologyError::MissingField("API response object"))?;
    match object.get("success").and_then(JsonValue::as_bool) {
        Some(true) => Ok(object
            .get("data")
            .cloned()
            .unwrap_or_else(|| JsonValue::Object(JsonMap::new()))),
        Some(false) => Err(SynologyError::Api {
            operation,
            code: object
                .get("error")
                .and_then(JsonValue::as_object)
                .and_then(|error| error.get("code"))
                .and_then(JsonValue::as_i64)
                .unwrap_or(-1),
        }),
        None => Err(SynologyError::MissingField("success")),
    }
}

fn required_u64(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<u64, SynologyError> {
    object
        .get(field)
        .and_then(JsonValue::as_u64)
        .ok_or(SynologyError::MissingField(field))
}

fn optional_u64(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Option<u64>, SynologyError> {
    match object.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or_else(|| {
            SynologyError::Validation(format!("{field} must be a non-negative integer"))
        }),
    }
}

fn optional_bool(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Option<bool>, SynologyError> {
    match object.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Bool(value)) => Ok(Some(*value)),
        Some(JsonValue::Number(value)) if value.as_u64() == Some(0) => Ok(Some(false)),
        Some(JsonValue::Number(value)) if value.as_u64() == Some(1) => Ok(Some(true)),
        Some(_) => Err(SynologyError::Validation(format!(
            "{field} must be a boolean"
        ))),
    }
}

fn required_text(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, SynologyError> {
    let value = object
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or(SynologyError::MissingField(field))?;
    bounded_text(field, value)
}

fn optional_text(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Option<String>, SynologyError> {
    match object.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::String(value)) if value.is_empty() => Ok(None),
        Some(JsonValue::String(value)) => bounded_text(field, value).map(Some),
        Some(_) => Err(SynologyError::Validation(format!(
            "{field} must be a string"
        ))),
    }
}

fn bounded_text(field: &str, value: &str) -> Result<String, SynologyError> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES || value.contains(['\r', '\n', '\0'])
    {
        Err(SynologyError::Validation(format!(
            "{field} is empty, oversized, or unsafe"
        )))
    } else {
        Ok(value.to_string())
    }
}

fn validate_secret(field: &str, value: &str) -> Result<(), SynologyError> {
    if value.is_empty() || value.len() > MAX_SECRET_BYTES || value.contains(['\r', '\n', '\0']) {
        Err(SynologyError::Validation(format!(
            "{field} is empty, oversized, or unsafe"
        )))
    } else {
        Ok(())
    }
}

fn validate_api_path(path: &str) -> Result<(), SynologyError> {
    if path.is_empty()
        || path.len() > 256
        || path.starts_with('/')
        || path.contains("..")
        || path.contains(['?', '#', '\\', ':', '\r', '\n', '\0'])
        || !path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'))
    {
        Err(SynologyError::Validation(
            "device advertised an unsafe Web API path".to_string(),
        ))
    } else {
        Ok(())
    }
}

fn api_plan(
    plans: &SynologyRequestPlans,
    method: LocalHttpMethod,
    path: &str,
    has_body: bool,
) -> Result<LocalHttpRequestPlan, SynologyError> {
    validate_api_path(path)?;
    let mut template = LocalHttpRequestTemplate::new(method, format!("/webapi/{path}"))?
        .with_accept("application/json")
        .with_timeout_ms(plans.timeout_ms)
        .with_auth(LocalHttpAuth::None);
    if has_body {
        template = template
            .with_content_type("application/x-www-form-urlencoded")
            .with_idempotent(false);
    }
    Ok(template.plan(&plans.endpoint, Vec::new())?)
}

fn api_url(
    endpoint: &LocalHttpEndpoint,
    path: &str,
    parameters: &[(&str, &str)],
) -> Result<Zeroizing<String>, SynologyError> {
    validate_api_path(path)?;
    let mut url = endpoint.url_for_path(&format!("/webapi/{path}"))?;
    if !parameters.is_empty() {
        url.push('?');
        let encoded = form_encode(parameters);
        url.push_str(encoded.as_str());
    }
    Ok(Zeroizing::new(url))
}

fn form_encode(parameters: &[(&str, &str)]) -> Zeroizing<String> {
    let mut output = Zeroizing::new(String::new());
    for (index, (key, value)) in parameters.iter().enumerate() {
        if index > 0 {
            output.push('&');
        }
        push_percent_encoded(&mut output, key);
        output.push('=');
        push_percent_encoded(&mut output, value);
    }
    output
}

fn push_percent_encoded(output: &mut String, value: &str) {
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            output.push(char::from(byte));
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
}

fn camera_health(camera: &SynologyCamera) -> Health {
    match camera.status {
        1 | 5 => Health::Online,
        9 | 11 | 14 => Health::Degraded,
        2..=4 | 6..=8 | 10 | 12..=13 | 15..=18 => Health::Offline,
        _ => Health::Unknown,
    }
}

fn aggregate_health(cameras: &[SynologyCamera]) -> Health {
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

fn camera_status_name(status: u8) -> &'static str {
    match status {
        1 => "normal",
        2 => "deleted",
        3 => "disconnected",
        4 => "unavailable",
        5 => "ready",
        6 => "inaccessible",
        7 => "disabled",
        8 => "unrecognized",
        9 => "setting",
        10 => "server_disconnected",
        11 => "migrating",
        12 => "other",
        13 => "storage_removed",
        14 => "stopping",
        15 => "connect_history_failed",
        16 => "unauthorized",
        17 => "rtsp_error",
        18 => "no_video",
        _ => "unknown",
    }
}

fn camera_value(camera: &SynologyCamera) -> Value {
    let mut fields = vec![
        (
            "status_code".to_string(),
            Value::Integer(camera.status as i64),
        ),
        (
            "status".to_string(),
            Value::Text(camera_status_name(camera.status).to_string()),
        ),
        ("vendor".to_string(), Value::Text(camera.vendor.clone())),
        ("model".to_string(), Value::Text(camera.model.clone())),
    ];
    if let Some(channel) = &camera.channel {
        fields.push(("channel".to_string(), Value::Text(channel.clone())));
    }
    Value::Object(fields)
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, SynologyError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| SynologyError::Validation(error.to_string()))
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
    override_url: Option<&str>,
    body: &[u8],
) -> Result<Vec<u8>, SynologyError> {
    let plan_url = Url::parse(&plan.url)?;
    let url = Url::parse(override_url.unwrap_or(&plan.url))?;
    if plan_url.scheme != url.scheme
        || plan_url.host != url.host
        || plan_url.effective_port() != url.effective_port()
    {
        return Err(SynologyError::Validation(
            "dynamic Web API request changed origin".to_string(),
        ));
    }
    let host = url
        .host
        .as_deref()
        .ok_or(SynologyError::MissingField("request URL host"))?;
    let port = url
        .effective_port()
        .ok_or(SynologyError::MissingField("request URL port"))?;
    let mut target = if url.path.is_empty() {
        "/".to_string()
    } else {
        url.path.clone()
    };
    if let Some(query) = &url.query {
        target.push('?');
        target.push_str(query);
    }
    if host.contains(['\r', '\n', '\0']) || target.contains(['\r', '\n', '\0']) {
        return Err(SynologyError::Validation(
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
        if header.name.eq_ignore_ascii_case("Content-Length") {
            continue;
        }
        if header.name.contains(['\r', '\n', '\0']) || header.value.contains(['\r', '\n', '\0']) {
            return Err(SynologyError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    request.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    request.extend_from_slice(body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, SynologyError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| SynologyError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| SynologyError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| SynologyError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(SynologyError::Io(
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

fn write_request(writer: &mut dyn Write, request: &[u8]) -> Result<(), SynologyError> {
    writer
        .write_all(request)
        .map_err(|error| SynologyError::Io(error.to_string()))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, SynologyError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| SynologyError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(SynologyError::ResponseTooLarge { limit: maximum });
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

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<HttpResponse, SynologyError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| SynologyError::Http(error.to_string()))?;
    let status = parsed.head.status;
    let mut headers = parsed.head.headers;
    let input = &bytes[parsed.body_offset..];
    let body = match (|| {
        let body = match parsed.body_kind {
            BodyKind::None => Vec::new(),
            BodyKind::ContentLength(expected) => {
                if input.len() < expected {
                    return Err(SynologyError::TruncatedBody {
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
            return Err(SynologyError::ResponseTooLarge { limit: maximum });
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

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, SynologyError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input
            .get(cursor..)
            .and_then(|tail| tail.windows(2).position(|window| window == b"\r\n"))
            .ok_or_else(|| SynologyError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| SynologyError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| SynologyError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(SynologyError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| SynologyError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(SynologyError::Http("truncated chunk".to_string()));
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

    fn credentials() -> SynologyCredentials {
        SynologyCredentials::new("operator", "secret-password").unwrap()
    }

    fn config(port: u16) -> SynologyConfig {
        SynologyConfig::new(
            BridgeId::trusted("synology.test"),
            format!("http://127.0.0.1:{port}"),
            VaultRef::trusted("vault://synology/test"),
        )
        .unwrap()
    }

    fn snapshot() -> SynologySnapshot {
        SynologySnapshot {
            info: SynologySurveillanceInfo {
                version: "9.2-11289".to_string(),
                camera_count: 2,
                maximum_camera_count: Some(40),
                user_privilege: Some(4),
                allow_snapshot: Some(true),
                allow_manual_recording: Some(false),
            },
            cameras: vec![
                SynologyCamera {
                    id: 10,
                    name: "Back".to_string(),
                    vendor: "Axis".to_string(),
                    model: "P3265-LV".to_string(),
                    channel: Some("1".to_string()),
                    status: 3,
                },
                SynologyCamera {
                    id: 20,
                    name: "Front".to_string(),
                    vendor: "Synology".to_string(),
                    model: "BC500".to_string(),
                    channel: Some("1".to_string()),
                    status: 1,
                },
            ],
        }
    }

    #[derive(Debug)]
    struct FixedTransport {
        snapshot: SynologySnapshot,
        calls: Arc<AtomicUsize>,
    }

    impl SynologyTransport for FixedTransport {
        fn inspect(
            &mut self,
            _plans: &SynologyRequestPlans,
            _credentials: &SynologyCredentials,
        ) -> Result<SynologySnapshot, SynologyError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.snapshot.clone())
        }
    }

    fn authorize(runtime: &mut SmartHomeRuntime, principal: AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:synology-test"),
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
        assert!(SynologyConfig::new(
            BridgeId::trusted("synology.bad"),
            "http://192.0.2.10:5000",
            VaultRef::trusted("vault://synology/bad")
        )
        .is_err());
        assert!(SynologyConfig::new(
            BridgeId::trusted("synology.good"),
            "https://diskstation.home:5001",
            VaultRef::trusted("vault://synology/good")
        )
        .is_ok());
        assert!(SynologyConfig::new(
            BridgeId::trusted("synology.embedded"),
            "https://operator:secret@diskstation.home:5001",
            VaultRef::trusted("vault://synology/embedded")
        )
        .is_err());
    }

    #[test]
    fn credentials_and_client_debug_are_redacted() {
        assert_eq!(
            format!("{:?}", credentials()),
            "SynologyCredentials([REDACTED])"
        );
        let client = SynologyClient::new(
            config(5001),
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
        assert!(!debug.contains("_sid"));
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = SynologyClient::new(
            config(5001),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = SynologyRuntimeIntegration::new(client);
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
        let client = SynologyClient::new(
            config(5001),
            credentials(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = SynologyRuntimeIntegration::new(client);
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
            Health::Offline
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
    fn api_catalog_fails_closed_on_old_or_unsafe_advertisements() {
        let old = br#"{"success":true,"data":{"SYNO.API.Auth":{"path":"auth.cgi","minVersion":1,"maxVersion":6},"SYNO.SurveillanceStation.Info":{"path":"entry.cgi","minVersion":1,"maxVersion":5},"SYNO.SurveillanceStation.Camera":{"path":"entry.cgi","minVersion":1,"maxVersion":8}}}"#;
        assert!(matches!(
            parse_api_catalog(old),
            Err(SynologyError::UnsupportedApi {
                api: CAMERA_API,
                ..
            })
        ));
        let unsafe_path = br#"{"success":true,"data":{"SYNO.API.Auth":{"path":"https://evil.test/auth.cgi","minVersion":1,"maxVersion":6},"SYNO.SurveillanceStation.Info":{"path":"entry.cgi","minVersion":1,"maxVersion":5},"SYNO.SurveillanceStation.Camera":{"path":"entry.cgi","minVersion":1,"maxVersion":9}}}"#;
        assert!(parse_api_catalog(unsafe_path).is_err());
    }

    #[test]
    fn camera_parser_sorts_ids_and_rejects_unknown_status() {
        let data = serde_json::json!({
            "total": 3,
            "cameras": [
                {"id": 20, "name": "Front", "vendor": "Synology", "model": "BC500", "channel": "1", "status": 1},
                {"id": 10, "newName": "Back", "vendor": "Axis", "model": "P3265-LV", "channel": "1", "status": 9}
            ]
        });
        let cameras = parse_cameras(data).unwrap();
        assert_eq!(
            cameras.iter().map(|camera| camera.id).collect::<Vec<_>>(),
            vec![10, 20]
        );
        assert_eq!(camera_health(&cameras[0]), Health::Degraded);
        let invalid = serde_json::json!({
            "total": 1,
            "cameras": [{"id": 1, "name": "Bad", "vendor": "Test", "model": "Test", "status": 99}]
        });
        assert!(parse_cameras(invalid).is_err());
    }

    #[test]
    fn loopback_transport_discovers_apis_and_keeps_session_material_private() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            r#"{"success":true,"data":{"SYNO.API.Auth":{"path":"auth.cgi","minVersion":1,"maxVersion":6},"SYNO.SurveillanceStation.Info":{"path":"entry.cgi","minVersion":1,"maxVersion":5},"SYNO.SurveillanceStation.Camera":{"path":"entry.cgi","minVersion":1,"maxVersion":9}}}"#,
            r#"{"success":true,"data":{"sid":"secret.sid.value","synotoken":"secret-token"}}"#,
            r#"{"success":true,"data":{"version":{"major":9,"minor":2,"build":11289},"cameraNumber":2,"maxCameraSupport":40,"userPriv":4,"allowSnapshot":true,"allowManualRec":false}}"#,
            r#"{"success":true,"data":{"total":2,"cameras":[{"id":20,"name":"Front","vendor":"Synology","model":"BC500","channel":"1","status":1},{"id":10,"newName":"Back","vendor":"Axis","model":"P3265-LV","channel":"1","status":3}]}}"#,
            r#"{"success":true}"#,
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
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                reader.get_mut().write_all(reply.as_bytes()).unwrap();
            }
        });

        let mut client =
            SynologyClient::new(config(port), credentials(), SynologyLanTransport::default())
                .unwrap();
        let observed = client.inspect().unwrap();
        handle.join().unwrap();
        assert_eq!(observed.info.version, "9.2-11289");
        assert_eq!(observed.cameras[0].id, 10);
        assert!(!format!("{observed:?}").contains("secret.sid.value"));

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[0].0.starts_with("GET /webapi/query.cgi?"));
        assert!(requests[1].0.starts_with("POST /webapi/auth.cgi HTTP/1.1"));
        assert_eq!(
            requests[1].1,
            "api=SYNO.API.Auth&method=login&version=6&account=operator&passwd=secret-password&session=SurveillanceStation&format=sid&enable_syno_token=yes"
        );
        assert!(requests[2].0.starts_with("GET /webapi/entry.cgi?"));
        assert!(requests[2].0.contains("method=GetInfo"));
        assert!(requests[3].0.contains("method=List"));
        assert!(requests[3].0.contains("blPrivilege=true"));
        assert!(requests[4].0.contains("method=logout"));
        assert!(requests[2..]
            .iter()
            .all(|(head, body)| head.contains("_sid=secret.sid.value")
                && head.contains("SynoToken=secret-token")
                && body.is_empty()));
        assert!(requests[2..]
            .iter()
            .all(|(head, _)| { !head.contains("secret-password") && !head.contains("operator") }));
    }

    #[test]
    fn response_bounds_and_api_errors_are_enforced() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        assert!(matches!(
            decode_http_response(response, 1),
            Err(SynologyError::ResponseTooLarge { limit: 1 })
        ));
        let error = br#"{"success":false,"error":{"code":105}}"#;
        assert!(matches!(
            parse_api_data(error, "camera list"),
            Err(SynologyError::Api {
                operation: "camera list",
                code: 105
            })
        ));
    }
}
