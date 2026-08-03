//! Authenticated Axis VAPIX discovery and inspection for D23.

#![forbid(unsafe_code)]

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use coding_adventures_zeroize::Zeroizing;
use http1::{parse_response_head, Http1ParseError};
use http_core::{find_header, BodyKind};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use smart_home_core::{
    AgentId, Bridge, BridgeId, BridgeTransport, Capability, CapabilityId, CapabilityMode,
    CommandResult, CommandType, Device, DeviceControlCommandType, DeviceId, Entity, EntityId,
    EntityKind, Health, IntegrationId, Metadata, ProtocolFamily, ProtocolIdentifier, SmartHomeTool,
    StateConfidence, StateSnapshot, StateSource, Value, ValueKind, VaultRef,
};
use smart_home_discovery::{
    run_mdns_ipv4_scan, DiscoveryConfidence, DiscoveryRecord, DiscoverySource, MdnsAdvertisement,
    MdnsScanOptions, MdnsScanResult, PairingRequirement,
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
pub const INTEGRATION_ID: &str = "axis_vapix";
pub const PROTOCOL_ID: &str = "axis_vapix";
pub const VIDEO_MDNS_SERVICE_TYPE: &str = "_axis-video._tcp.local";
pub const NVR_MDNS_SERVICE_TYPE: &str = "_axis-nvr._tcp.local";
pub const BASIC_DEVICE_INFO_PATH: &str = "/axis-cgi/basicdeviceinfo.cgi";
pub const API_DISCOVERY_PATH: &str = "/axis-cgi/apidiscovery.cgi";
pub const PTZ_CONTROL_PATH: &str = "/axis-cgi/com/ptz.cgi";
pub const PTZ_QUEUE_PATH: &str = "/axis-cgi/com/ptzqueue.cgi";
pub const PARAM_PATH: &str = "/axis-cgi/param.cgi";
pub const DEFAULT_MAX_DISCOVERY_RESPONSES: usize = 128;
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MIN_PTZ_SPEED: u32 = 1;
pub const MAX_PTZ_SPEED: u32 = 100;
pub const MAX_PTZ_DURATION_MS: u64 = 5_000;
const PTZ_CAMERA: u32 = 1;

#[derive(Debug)]
pub enum AxisError {
    Validation(String),
    Discovery(String),
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
        code: i64,
        message: String,
    },
    MissingField(&'static str),
    UnknownEntity(EntityId),
    UnsupportedCommand(CommandType),
    InvalidCommandArguments {
        command_type: CommandType,
        expected: &'static str,
    },
    PtzQueueUnavailable,
    Runtime(RuntimeError),
}

impl fmt::Display for AxisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Axis VAPIX input: {message}"),
            Self::Discovery(message) => write!(formatter, "Axis mDNS discovery failed: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Axis VAPIX URL: {error}"),
            Self::Io(message) => write!(formatter, "Axis LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "Axis TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Axis HTTP response: {message}"),
            Self::HttpStatus(status) => write!(formatter, "Axis endpoint returned HTTP {status}"),
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Axis response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Axis response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Axis JSON: {error}"),
            Self::Api { code, message } => {
                write!(formatter, "Axis VAPIX error {code}: {message}")
            }
            Self::MissingField(field) => write!(formatter, "Axis response is missing {field}"),
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown Axis entity {}", entity_id.as_str())
            }
            Self::UnsupportedCommand(command_type) => {
                write!(formatter, "unsupported Axis command {command_type:?}")
            }
            Self::InvalidCommandArguments {
                command_type,
                expected,
            } => write!(
                formatter,
                "invalid arguments for Axis command {command_type:?}; expected {expected}"
            ),
            Self::PtzQueueUnavailable => {
                formatter.write_str("Axis PTZ control queue did not grant exclusive control")
            }
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for AxisError {}

impl From<LocalHttpError> for AxisError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for AxisError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for AxisError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for AxisError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct AxisCredentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl AxisCredentials {
    pub fn new(
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Result<Self, AxisError> {
        let username = username.into();
        let password = password.into();
        if username.trim().is_empty() || password.is_empty() {
            return Err(AxisError::Validation(
                "username and password must not be empty".to_string(),
            ));
        }
        if has_unsafe_http_text(&username) || has_unsafe_http_text(&password) {
            return Err(AxisError::Validation(
                "credentials contain unsafe HTTP text".to_string(),
            ));
        }
        Ok(Self {
            username: Zeroizing::new(username),
            password: Zeroizing::new(password),
        })
    }
}

impl fmt::Debug for AxisCredentials {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AxisCredentials([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub credential_ref: VaultRef,
    pub timeout: Duration,
}

impl AxisConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        credential_ref: VaultRef,
    ) -> Result<Self, AxisError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(AxisError::MissingField("base URL host"))?;
        let secure = parsed.scheme == "https";
        let test_loopback = parsed.scheme == "http" && is_loopback_host(host);
        if (!secure && !test_loopback)
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(AxisError::Validation(
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

    fn endpoint(&self) -> Result<LocalHttpEndpoint, AxisError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(AxisError::MissingField("base URL host"))?;
        let scheme = match parsed.scheme.as_str() {
            "https" => LocalHttpScheme::Https,
            "http" if is_loopback_host(host) => LocalHttpScheme::Http,
            _ => {
                return Err(AxisError::Validation(
                    "Axis endpoint is not an approved HTTPS or loopback origin".to_string(),
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
        .with_metadata(Metadata::new("http.profile", "axis.vapix")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisDeviceInformation {
    pub brand: String,
    pub product_full_name: String,
    pub product_number: String,
    pub product_type: String,
    pub serial_number: String,
    pub firmware_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisApi {
    pub id: String,
    pub version: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisSnapshot {
    pub device: AxisDeviceInformation,
    pub apis: Vec<AxisApi>,
    pub ptz: Option<AxisPtzSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisPtzPosition {
    pub pan: Option<f64>,
    pub tilt: Option<f64>,
    pub zoom: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisPtzPreset {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPtzQueueMode {
    Disabled,
    Required,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AxisPtzSnapshot {
    pub camera: u32,
    pub position: AxisPtzPosition,
    pub presets: Vec<AxisPtzPreset>,
    pub supports_continuous_pan_tilt: bool,
    pub supports_server_presets: bool,
    pub supports_speed: bool,
    pub queue_mode: AxisPtzQueueMode,
}

impl AxisPtzSnapshot {
    fn commandable(&self) -> bool {
        self.supports_speed
            && (self.supports_continuous_pan_tilt
                || (self.supports_server_presets && !self.presets.is_empty()))
    }
}

pub fn scan_video_mdns_ipv4(
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, AxisError> {
    scan_mdns_ipv4(VIDEO_MDNS_SERVICE_TYPE, discovered_at_ms, timeout)
}

pub fn scan_nvr_mdns_ipv4(
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, AxisError> {
    scan_mdns_ipv4(NVR_MDNS_SERVICE_TYPE, discovered_at_ms, timeout)
}

fn scan_mdns_ipv4(
    service_type: &str,
    discovered_at_ms: u64,
    timeout: Duration,
) -> Result<MdnsScanResult, AxisError> {
    let options = MdnsScanOptions::new(service_type, discovered_at_ms, timeout)
        .map_err(|error| AxisError::Discovery(error.to_string()))?
        .with_max_responses(DEFAULT_MAX_DISCOVERY_RESPONSES);
    run_mdns_ipv4_scan(options).map_err(|error| AxisError::Discovery(error.to_string()))
}

pub fn discovery_record(advertisement: &MdnsAdvertisement) -> Result<DiscoveryRecord, AxisError> {
    let service_type = advertisement.service_type.trim_end_matches('.');
    if ![VIDEO_MDNS_SERVICE_TYPE, NVR_MDNS_SERVICE_TYPE]
        .iter()
        .map(|value| value.trim_end_matches('.'))
        .any(|value| value == service_type)
    {
        return Err(AxisError::Validation(format!(
            "unexpected Axis mDNS service type `{}`",
            advertisement.service_type
        )));
    }
    let instance_identity = advertisement
        .instance_name
        .rsplit_once(" - ")
        .map(|(_, identity)| identity)
        .unwrap_or(&advertisement.instance_name);
    let advertised_identity = advertisement
        .txt_value("macaddress")
        .or_else(|| advertisement.txt_value("mac_address"))
        .unwrap_or(instance_identity);
    let native_id = stable_component(advertised_identity);
    if native_id.is_empty() {
        return Err(AxisError::Validation(
            "Axis advertisement has no stable identity".to_string(),
        ));
    }
    let scheme = if advertisement.port == 443 {
        "https"
    } else {
        "http"
    };
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        native_id,
        DiscoverySource::Mdns,
        BridgeTransport::LanHttp,
        advertisement.discovered_at_ms,
    )
    .map_err(|error| AxisError::Discovery(error.to_string()))?
    .with_display_name(advertisement.instance_name.clone())
    .with_address(advertisement.endpoint_with_scheme(scheme))
    .with_confidence(DiscoveryConfidence::Candidate)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata(
        "smart_home.discovery.service_type",
        &advertisement.service_type,
    )
    .with_metadata("axis.discovery.requires_verified_https", "true"))
}

pub trait AxisTransport {
    fn execute(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &AxisCredentials,
        session_cookie: Option<&str>,
    ) -> Result<AxisHttpResponse, AxisError>;
}

pub struct AxisHttpResponse {
    pub body: Vec<u8>,
    session_cookie: Option<Zeroizing<String>>,
}

impl AxisHttpResponse {
    pub fn new(body: Vec<u8>) -> Self {
        Self {
            body,
            session_cookie: None,
        }
    }

    pub fn with_session_cookie(
        body: Vec<u8>,
        session_cookie: impl Into<String>,
    ) -> Result<Self, AxisError> {
        let session_cookie = session_cookie.into();
        if session_cookie.trim().is_empty() || has_unsafe_http_text(&session_cookie) {
            return Err(AxisError::Validation(
                "session cookie is empty or contains unsafe HTTP text".to_string(),
            ));
        }
        Ok(Self {
            body,
            session_cookie: Some(Zeroizing::new(session_cookie)),
        })
    }
}

impl fmt::Debug for AxisHttpResponse {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AxisHttpResponse")
            .field("body_bytes", &self.body.len())
            .field(
                "session_cookie",
                &self.session_cookie.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

pub struct AxisLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    maximum_response_bytes: usize,
}

impl Default for AxisLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl AxisLanTransport {
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
}

impl AxisTransport for AxisLanTransport {
    fn execute(
        &mut self,
        plan: &LocalHttpRequestPlan,
        credentials: &AxisCredentials,
        session_cookie: Option<&str>,
    ) -> Result<AxisHttpResponse, AxisError> {
        let url = Url::parse(&plan.url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(AxisError::MissingField("request URL host"))?;
        let port = url
            .effective_port()
            .ok_or(AxisError::MissingField("request URL port"))?;
        if url.scheme == "http" && !is_loopback_host(host) {
            return Err(AxisError::Validation(
                "Basic credentials may use HTTP only on loopback tests".to_string(),
            ));
        }
        let timeout = Duration::from_millis(plan.timeout_ms.max(1));
        let request = Zeroizing::new(encode_http_request(
            &url,
            plan,
            credentials,
            session_cookie,
        )?);
        let response = match url.scheme.as_str() {
            "http" => {
                let mut stream = connect_tcp(host, port, timeout)?;
                write_request(&mut stream, &request)?;
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
                    .map_err(|error| AxisError::Tls(error.to_string()))?;
                write_request(&mut stream, &request)?;
                let bytes = read_bounded(&mut stream, self.maximum_response_bytes)?;
                stream
                    .close_notify()
                    .map_err(|error| AxisError::Tls(error.to_string()))?;
                bytes
            }
            scheme => {
                return Err(AxisError::Validation(format!(
                    "unsupported Axis URL scheme `{scheme}`"
                )))
            }
        };
        decode_http_response(&response, self.maximum_response_bytes)
    }
}

pub struct AxisClient<T> {
    config: AxisConfig,
    credentials: AxisCredentials,
    endpoint: LocalHttpEndpoint,
    transport: T,
}

impl<T: AxisTransport> AxisClient<T> {
    pub fn new(
        config: AxisConfig,
        credentials: AxisCredentials,
        transport: T,
    ) -> Result<Self, AxisError> {
        let endpoint = config.endpoint()?;
        Ok(Self {
            config,
            credentials,
            endpoint,
            transport,
        })
    }

    pub fn config(&self) -> &AxisConfig {
        &self.config
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn inspect(&mut self) -> Result<AxisSnapshot, AxisError> {
        let properties = self.post_json(
            BASIC_DEVICE_INFO_PATH,
            &json!({"apiVersion":"1.0","context":"smart-home","method":"getAllProperties"}),
        )?;
        let apis = self.post_json(
            API_DISCOVERY_PATH,
            &json!({"apiVersion":"1.0","context":"smart-home","method":"getApiList"}),
        )?;
        let mut snapshot = parse_snapshot(&properties, &apis)?;
        if snapshot.apis.iter().any(|api| api.id == "ptz-control") {
            snapshot.ptz = Some(self.inspect_ptz(PTZ_CAMERA)?);
        }
        Ok(snapshot)
    }

    fn post_json(&mut self, path: &str, body: &JsonValue) -> Result<JsonValue, AxisError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Post, path)?
            .with_accept("application/json")
            .with_content_type("application/json")
            .with_timeout_ms(duration_ms(self.config.timeout))
            .with_idempotent(true)
            .with_auth(LocalHttpAuth::Basic {
                vault_ref: self.config.credential_ref.clone(),
            });
        let plan = template.plan(&self.endpoint, serde_json::to_vec(body)?)?;
        let response = self.transport.execute(&plan, &self.credentials, None)?;
        let value: JsonValue = serde_json::from_slice(&response.body)?;
        if let Some(error) = value.get("error") {
            return Err(AxisError::Api {
                code: error.get("code").and_then(JsonValue::as_i64).unwrap_or(-1),
                message: error
                    .get("message")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("device rejected request")
                    .to_string(),
            });
        }
        Ok(value)
    }

    fn get_text(
        &mut self,
        target: &str,
        session_cookie: Option<&str>,
    ) -> Result<AxisHttpResponse, AxisError> {
        let template = LocalHttpRequestTemplate::new(LocalHttpMethod::Get, target)?
            .with_accept("text/plain")
            .with_timeout_ms(duration_ms(self.config.timeout))
            .with_idempotent(false)
            .with_auth(LocalHttpAuth::Basic {
                vault_ref: self.config.credential_ref.clone(),
            });
        let plan = template.plan(&self.endpoint, Vec::new())?;
        let response = self
            .transport
            .execute(&plan, &self.credentials, session_cookie)?;
        reject_ptz_error(&response.body)?;
        Ok(response)
    }

    fn inspect_ptz(&mut self, camera: u32) -> Result<AxisPtzSnapshot, AxisError> {
        let info = self.get_text(&format!("{PTZ_CONTROL_PATH}?info=1&camera={camera}"), None)?;
        let info = response_text(&info.body)?;
        let supports_continuous_pan_tilt = info
            .lines()
            .any(|line| line.trim_start().starts_with("continuouspantiltmove="));
        let supports_server_presets = info
            .lines()
            .any(|line| line.trim_start().starts_with("gotoserverpresetno="));
        let supports_speed = info
            .lines()
            .any(|line| line.trim_start().starts_with("speed="));
        let position = self.get_text(
            &format!("{PTZ_CONTROL_PATH}?query=position&camera={camera}"),
            None,
        )?;
        let presets = if supports_server_presets {
            let response = self.get_text(
                &format!("{PTZ_CONTROL_PATH}?query=presetposall&camera={camera}"),
                None,
            )?;
            parse_ptz_presets(response_text(&response.body)?)?
        } else {
            Vec::new()
        };
        let queue = self.get_text(
            &format!("{PARAM_PATH}?action=list&group=PTZ.Various.V{camera}.CtlQueueing"),
            None,
        )?;
        Ok(AxisPtzSnapshot {
            camera,
            position: parse_ptz_position(response_text(&position.body)?)?,
            presets,
            supports_continuous_pan_tilt,
            supports_server_presets,
            supports_speed,
            queue_mode: parse_queue_mode(response_text(&queue.body)?)?,
        })
    }

    pub fn recall_ptz_preset(
        &mut self,
        camera: u32,
        preset_id: u32,
        speed: u32,
        queue_mode: AxisPtzQueueMode,
    ) -> Result<(), AxisError> {
        self.with_ptz_control(camera, queue_mode, |client, cookie| {
            client.get_text(
                &format!(
                    "{PTZ_CONTROL_PATH}?gotoserverpresetno={preset_id}&speed={speed}&camera={camera}"
                ),
                cookie,
            )?;
            Ok(())
        })
    }

    pub fn move_ptz_bounded(
        &mut self,
        camera: u32,
        direction: AxisPtzDirection,
        speed: u32,
        duration_ms: u64,
        queue_mode: AxisPtzQueueMode,
    ) -> Result<(), AxisError> {
        self.with_ptz_control(camera, queue_mode, |client, cookie| {
            let (pan, tilt) = direction.velocity(speed);
            client.get_text(
                &format!("{PTZ_CONTROL_PATH}?continuouspantiltmove={pan},{tilt}&camera={camera}"),
                cookie,
            )?;
            thread::sleep(Duration::from_millis(duration_ms));
            client.get_text(
                &format!("{PTZ_CONTROL_PATH}?continuouspantiltmove=0,0&camera={camera}"),
                cookie,
            )?;
            Ok(())
        })
    }

    fn with_ptz_control<R>(
        &mut self,
        camera: u32,
        queue_mode: AxisPtzQueueMode,
        operation: impl FnOnce(&mut Self, Option<&str>) -> Result<R, AxisError>,
    ) -> Result<R, AxisError> {
        if queue_mode == AxisPtzQueueMode::Disabled {
            return operation(self, None);
        }
        let response = self.get_text(
            &format!("{PTZ_QUEUE_PATH}?control=request&camera={camera}"),
            None,
        )?;
        let queue_position = parse_queue_position(response_text(&response.body)?);
        if queue_position != Some(1) {
            if let Some(cookie) = response.session_cookie.as_ref() {
                let _ = self.get_text(
                    &format!("{PTZ_QUEUE_PATH}?control=drop&camera={camera}"),
                    Some(cookie.as_str()),
                );
            }
            return Err(AxisError::PtzQueueUnavailable);
        }
        let cookie = response
            .session_cookie
            .ok_or(AxisError::PtzQueueUnavailable)?;
        let result = operation(self, Some(cookie.as_str()));
        let drop_result = self.get_text(
            &format!("{PTZ_QUEUE_PATH}?control=drop&camera={camera}"),
            Some(cookie.as_str()),
        );
        match (result, drop_result) {
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
            (Ok(value), Ok(_)) => Ok(value),
        }
    }
}

impl<T> fmt::Debug for AxisClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AxisClient")
            .field("config", &self.config)
            .field("credentials", &"[REDACTED]")
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledAxisDevice {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub camera_entity_id: EntityId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisPtzDirection {
    Left,
    Right,
    Up,
    Down,
}

impl AxisPtzDirection {
    fn from_label(label: &str) -> Option<Self> {
        match label {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            _ => None,
        }
    }

    fn velocity(self, speed: u32) -> (i32, i32) {
        let speed = i32::try_from(speed).unwrap_or(i32::MAX);
        match self {
            Self::Left => (-speed, 0),
            Self::Right => (speed, 0),
            Self::Up => (0, speed),
            Self::Down => (0, -speed),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AxisPtzSupport {
    camera: u32,
    preset_ids: BTreeSet<u32>,
    supports_continuous_pan_tilt: bool,
    supports_server_presets: bool,
    queue_mode: AxisPtzQueueMode,
}

pub struct AxisRuntimeIntegration<T> {
    client: AxisClient<T>,
    ptz_entities: BTreeMap<EntityId, AxisPtzSupport>,
}

impl<T: AxisTransport> AxisRuntimeIntegration<T> {
    pub fn new(client: AxisClient<T>) -> Self {
        Self {
            client,
            ptz_entities: BTreeMap::new(),
        }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledAxisDevice, AxisError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        let installed = install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)?;
        self.ptz_entities.clear();
        if let Some(ptz) = snapshot.ptz.as_ref().filter(|ptz| ptz.commandable()) {
            self.ptz_entities.insert(
                installed.camera_entity_id.clone(),
                AxisPtzSupport {
                    camera: ptz.camera,
                    preset_ids: ptz.presets.iter().map(|preset| preset.id).collect(),
                    supports_continuous_pan_tilt: ptz.supports_continuous_pan_tilt,
                    supports_server_presets: ptz.supports_server_presets,
                    queue_mode: ptz.queue_mode,
                },
            );
        }
        Ok(installed)
    }

    pub fn dispatch_command_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        request: RuntimeCommandToolRequest,
        now_ms: u64,
    ) -> Result<CommandResult, AxisError> {
        let support = self
            .ptz_entities
            .get(&request.entity_id)
            .cloned()
            .ok_or_else(|| AxisError::UnknownEntity(request.entity_id.clone()))?;
        match request.command_type {
            CommandType::DeviceControl(DeviceControlCommandType::RecallCameraPtzPreset) => {
                let (preset_id, speed) = ptz_preset_arguments(&request)?;
                if !support.supports_server_presets || !support.preset_ids.contains(&preset_id) {
                    return invalid_command_arguments(
                        request.command_type,
                        "an object with a probed preset_id and speed from 1 through 100",
                    );
                }
                let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
                let mut result = runtime.submit_command(command, now_ms)?;
                self.client.recall_ptz_preset(
                    support.camera,
                    preset_id,
                    speed,
                    support.queue_mode,
                )?;
                result.message = Some(format!(
                    "Axis camera {} accepted PTZ preset {preset_id}",
                    support.camera
                ));
                Ok(result)
            }
            CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz) => {
                if !support.supports_continuous_pan_tilt {
                    return Err(AxisError::UnsupportedCommand(request.command_type));
                }
                let (direction, speed, duration_ms) = ptz_move_arguments(&request)?;
                let command = runtime.authorize_command_tool(principal_id, request, now_ms)?;
                let mut result = runtime.submit_command(command, now_ms)?;
                self.client.move_ptz_bounded(
                    support.camera,
                    direction,
                    speed,
                    duration_ms,
                    support.queue_mode,
                )?;
                result.message = Some(format!(
                    "Axis camera {} completed bounded PTZ {} movement",
                    support.camera,
                    direction.label()
                ));
                Ok(result)
            }
            command_type => Err(AxisError::UnsupportedCommand(command_type)),
        }
    }
}

pub fn paired_discovery_record(
    config: &AxisConfig,
    snapshot: &AxisSnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, AxisError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&snapshot.device.serial_number),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| AxisError::Discovery(error.to_string()))?
    .with_display_name(snapshot.device.product_full_name.clone())
    .with_address(config.base_url.clone())
    .with_hardware_model(snapshot.device.product_number.clone())
    .with_firmware_version(snapshot.device.firmware_version.clone())
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("axis.protocol", PROTOCOL_ID)
    .with_metadata("axis.api_count", snapshot.apis.len().to_string()))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &AxisConfig,
    snapshot: &AxisSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledAxisDevice, AxisError> {
    let native_id = stable_component(&snapshot.device.serial_number);
    if native_id.is_empty() {
        return Err(AxisError::Validation(
            "device serial does not contain a stable identifier".to_string(),
        ));
    }
    let device_id = DeviceId::trusted(format!("axis:{native_id}"));
    let camera_entity_id = EntityId::trusted(format!("axis:{native_id}:camera"));

    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some(snapshot.device.product_number.clone());
    bridge.firmware_version = Some(snapshot.device.firmware_version.clone());
    bridge.auth_ref = Some(config.credential_ref.clone());
    bridge.health = Health::Online;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("https_endpoint", &config.base_url)?];
    bridge.metadata = vec![
        Metadata::new("axis.transport", "authenticated_https_basic"),
        Metadata::new("axis.api_count", snapshot.apis.len().to_string()),
    ];
    runtime.upsert_bridge(bridge)?;

    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: snapshot.device.brand.clone(),
        model: snapshot.device.product_number.clone(),
        name: snapshot.device.product_full_name.clone(),
        serial: Some(snapshot.device.serial_number.clone()),
        firmware_version: Some(snapshot.device.firmware_version.clone()),
        room_id: None,
        entity_ids: vec![camera_entity_id.clone()],
        identifiers: vec![protocol_identifier(
            "serial",
            &snapshot.device.serial_number,
        )?],
        health: Health::Online,
        metadata: vec![Metadata::new(
            "axis.product_type",
            snapshot.device.product_type.clone(),
        )],
    })?;

    let mut capabilities = vec![Capability::new(
        CapabilityId::trusted("camera.device_info"),
        CapabilityMode::Observe,
        ValueKind::Object,
    )];
    if snapshot
        .ptz
        .as_ref()
        .is_some_and(AxisPtzSnapshot::commandable)
    {
        capabilities.push(Capability::camera_ptz());
    }
    runtime.upsert_entity(Entity {
        entity_id: camera_entity_id.clone(),
        device_id: device_id.clone(),
        kind: EntityKind::Camera,
        name: snapshot.device.product_full_name.clone(),
        capabilities,
        state: Some(StateSnapshot {
            entity_id: camera_entity_id.clone(),
            value: snapshot_value(snapshot),
            source: StateSource::Poll,
            observed_at_ms,
            received_at_ms: observed_at_ms,
            expires_at_ms: None,
            confidence: StateConfidence::Confirmed,
        }),
        metadata: vec![Metadata::new("axis.protocol", PROTOCOL_ID)],
    })?;

    Ok(InstalledAxisDevice {
        bridge_id: config.bridge_id.clone(),
        device_id,
        camera_entity_id,
    })
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), AxisError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(AxisError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn parse_snapshot(properties: &JsonValue, apis: &JsonValue) -> Result<AxisSnapshot, AxisError> {
    let properties = properties
        .pointer("/data/propertyList")
        .and_then(JsonValue::as_object)
        .ok_or(AxisError::MissingField("data.propertyList"))?;
    let device = AxisDeviceInformation {
        brand: required_string(properties, "Brand")?,
        product_full_name: required_string(properties, "ProdFullName")?,
        product_number: required_string(properties, "ProdNbr")?,
        product_type: required_string(properties, "ProdType")?,
        serial_number: required_string(properties, "SerialNumber")?,
        firmware_version: required_string(properties, "Version")?,
    };
    let api_list = apis
        .pointer("/data/apiList")
        .and_then(JsonValue::as_array)
        .ok_or(AxisError::MissingField("data.apiList"))?;
    let mut parsed_apis = Vec::with_capacity(api_list.len());
    for api in api_list {
        let api = api
            .as_object()
            .ok_or(AxisError::MissingField("data.apiList[]"))?;
        parsed_apis.push(AxisApi {
            id: required_string(api, "id")?,
            version: required_string(api, "version")?,
            status: required_string(api, "status")?,
        });
    }
    parsed_apis.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.version.cmp(&right.version))
    });
    Ok(AxisSnapshot {
        device,
        apis: parsed_apis,
        ptz: None,
    })
}

fn required_string(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, AxisError> {
    object
        .get(field)
        .and_then(JsonValue::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or(AxisError::MissingField(field))
}

fn snapshot_value(snapshot: &AxisSnapshot) -> Value {
    let mut fields = vec![
        (
            "product_type".to_string(),
            Value::Text(snapshot.device.product_type.clone()),
        ),
        (
            "product_number".to_string(),
            Value::Text(snapshot.device.product_number.clone()),
        ),
        (
            "serial_number".to_string(),
            Value::Text(snapshot.device.serial_number.clone()),
        ),
        (
            "firmware_version".to_string(),
            Value::Text(snapshot.device.firmware_version.clone()),
        ),
        (
            "apis".to_string(),
            Value::Array(
                snapshot
                    .apis
                    .iter()
                    .map(|api| {
                        Value::Object(vec![
                            ("id".to_string(), Value::Text(api.id.clone())),
                            ("version".to_string(), Value::Text(api.version.clone())),
                            ("status".to_string(), Value::Text(api.status.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ];
    if let Some(ptz) = &snapshot.ptz {
        fields.push(("ptz".to_string(), ptz_value(ptz)));
    }
    Value::Object(fields)
}

fn ptz_value(ptz: &AxisPtzSnapshot) -> Value {
    let mut position = Vec::new();
    if let Some(pan) = ptz.position.pan {
        position.push(("pan".to_string(), Value::Number(pan)));
    }
    if let Some(tilt) = ptz.position.tilt {
        position.push(("tilt".to_string(), Value::Number(tilt)));
    }
    if let Some(zoom) = ptz.position.zoom {
        position.push(("zoom".to_string(), Value::Number(zoom)));
    }
    Value::Object(vec![
        ("camera".to_string(), Value::Integer(i64::from(ptz.camera))),
        ("position".to_string(), Value::Object(position)),
        (
            "presets".to_string(),
            Value::Array(
                ptz.presets
                    .iter()
                    .map(|preset| {
                        Value::Object(vec![
                            ("id".to_string(), Value::Integer(i64::from(preset.id))),
                            ("name".to_string(), Value::Text(preset.name.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "control_queue".to_string(),
            Value::Text(
                match ptz.queue_mode {
                    AxisPtzQueueMode::Disabled => "disabled",
                    AxisPtzQueueMode::Required => "required",
                }
                .to_string(),
            ),
        ),
    ])
}

fn response_text(bytes: &[u8]) -> Result<&str, AxisError> {
    std::str::from_utf8(bytes)
        .map_err(|_| AxisError::Http("Axis PTZ response is not UTF-8".to_string()))
}

fn reject_ptz_error(bytes: &[u8]) -> Result<(), AxisError> {
    let text = response_text(bytes)?;
    if text
        .lines()
        .any(|line| line.trim_start().starts_with("Error:"))
    {
        return Err(AxisError::Api {
            code: -1,
            message: text.trim().to_string(),
        });
    }
    Ok(())
}

fn parse_ptz_position(text: &str) -> Result<AxisPtzPosition, AxisError> {
    let values = parse_key_values(text);
    let parse = |field: &'static str| -> Result<Option<f64>, AxisError> {
        values
            .get(field)
            .map(|value| {
                value.parse::<f64>().map_err(|_| {
                    AxisError::Validation(format!("PTZ {field} is not a finite number"))
                })
            })
            .transpose()
            .and_then(|value| {
                if value.is_some_and(f64::is_finite) {
                    Ok(value)
                } else if value.is_some() {
                    Err(AxisError::Validation(format!(
                        "PTZ {field} is not a finite number"
                    )))
                } else {
                    Ok(None)
                }
            })
    };
    let position = AxisPtzPosition {
        pan: parse("pan")?,
        tilt: parse("tilt")?,
        zoom: parse("zoom")?,
    };
    if position.pan.is_none() && position.tilt.is_none() && position.zoom.is_none() {
        return Err(AxisError::MissingField("PTZ position"));
    }
    Ok(position)
}

fn parse_ptz_presets(text: &str) -> Result<Vec<AxisPtzPreset>, AxisError> {
    let mut presets = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = line.trim().split_once('=') else {
            continue;
        };
        let Some(id) = key.strip_prefix("presetposno") else {
            continue;
        };
        let id = id
            .parse::<u32>()
            .map_err(|_| AxisError::Validation("invalid Axis preset identifier".to_string()))?;
        if !value.trim().is_empty() {
            presets.push(AxisPtzPreset {
                id,
                name: value.trim().to_string(),
            });
        }
    }
    presets.sort_by_key(|preset| preset.id);
    Ok(presets)
}

fn parse_queue_mode(text: &str) -> Result<AxisPtzQueueMode, AxisError> {
    let value = text
        .lines()
        .filter_map(|line| line.split_once('=').map(|(_, value)| value.trim()))
        .next()
        .ok_or(AxisError::MissingField("PTZ control queue setting"))?;
    match value.to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => Ok(AxisPtzQueueMode::Required),
        "no" | "false" | "0" => Ok(AxisPtzQueueMode::Disabled),
        _ => Err(AxisError::Validation(
            "unrecognized PTZ control queue setting".to_string(),
        )),
    }
}

fn parse_queue_position(text: &str) -> Option<u32> {
    let marker = "<a name=\"";
    let start = text.find(marker)? + marker.len();
    let end = text[start..].find('"')? + start;
    text[start..end].parse().ok()
}

fn parse_key_values(text: &str) -> BTreeMap<&str, &str> {
    text.lines()
        .filter_map(|line| line.trim().split_once('='))
        .collect()
}

fn ptz_preset_arguments(request: &RuntimeCommandToolRequest) -> Result<(u32, u32), AxisError> {
    match (
        object_u32(&request.arguments, "preset_id"),
        object_u32(&request.arguments, "speed"),
    ) {
        (Some(preset_id), Some(speed)) if (MIN_PTZ_SPEED..=MAX_PTZ_SPEED).contains(&speed) => {
            Ok((preset_id, speed))
        }
        _ => invalid_command_arguments(
            request.command_type,
            "an object with a probed preset_id and speed from 1 through 100",
        ),
    }
}

fn ptz_move_arguments(
    request: &RuntimeCommandToolRequest,
) -> Result<(AxisPtzDirection, u32, u64), AxisError> {
    let direction =
        object_text(&request.arguments, "direction").and_then(AxisPtzDirection::from_label);
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
) -> Result<T, AxisError> {
    Err(AxisError::InvalidCommandArguments {
        command_type,
        expected,
    })
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, AxisError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| AxisError::Validation(error.to_string()))
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

fn has_unsafe_http_text(value: &str) -> bool {
    value.contains(['\r', '\n', '\0'])
}

fn encode_http_request(
    url: &Url,
    plan: &LocalHttpRequestPlan,
    credentials: &AxisCredentials,
    session_cookie: Option<&str>,
) -> Result<Vec<u8>, AxisError> {
    let host = url
        .host
        .as_deref()
        .ok_or(AxisError::MissingField("request URL host"))?;
    let port = url
        .effective_port()
        .ok_or(AxisError::MissingField("request URL port"))?;
    let target = if url.path.is_empty() {
        "/".to_string()
    } else if let Some(query) = &url.query {
        format!("{}?{query}", url.path)
    } else {
        url.path.clone()
    };
    if has_unsafe_http_text(&target) || has_unsafe_http_text(host) {
        return Err(AxisError::Validation(
            "request target contains unsafe HTTP text".to_string(),
        ));
    }
    if session_cookie.is_some_and(has_unsafe_http_text) {
        return Err(AxisError::Validation(
            "session cookie contains unsafe HTTP text".to_string(),
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
    let mut seen = BTreeSet::new();
    for header in &plan.headers {
        if has_unsafe_http_text(&header.name) || has_unsafe_http_text(&header.value) {
            return Err(AxisError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        seen.insert(header.name.to_ascii_lowercase());
        if header.name.eq_ignore_ascii_case("Authorization") {
            let raw = Zeroizing::new(format!(
                "{}:{}",
                credentials.username.as_str(),
                credentials.password.as_str()
            ));
            let encoded = Zeroizing::new(BASE64.encode(raw.as_bytes()));
            request.extend_from_slice(
                format!("Authorization: Basic {}\r\n", encoded.as_str()).as_bytes(),
            );
        } else if header.name.eq_ignore_ascii_case("Cookie") {
            return Err(AxisError::Validation(
                "session cookies must not be stored in Axis request plans".to_string(),
            ));
        } else {
            request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
        }
    }
    if !seen.contains("authorization") {
        return Err(AxisError::Validation(
            "Axis request plan is missing Vault-backed Basic authorization".to_string(),
        ));
    }
    if let Some(cookie) = session_cookie {
        request.extend_from_slice(format!("Cookie: {cookie}\r\n").as_bytes());
    }
    if !seen.contains("content-length") {
        request.extend_from_slice(format!("Content-Length: {}\r\n", plan.body.len()).as_bytes());
    }
    request.extend_from_slice(b"\r\n");
    request.extend_from_slice(&plan.body);
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, AxisError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| AxisError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| AxisError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| AxisError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(AxisError::Io(
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

fn write_request(writer: &mut dyn Write, request: &[u8]) -> Result<(), AxisError> {
    writer
        .write_all(request)
        .map_err(|error| AxisError::Io(error.to_string()))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, AxisError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| AxisError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(AxisError::ResponseTooLarge { limit: maximum });
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
    Ok(bytes)
}

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<AxisHttpResponse, AxisError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| AxisError::Http(error.to_string()))?;
    if !(200..300).contains(&parsed.head.status) {
        return Err(AxisError::HttpStatus(parsed.head.status));
    }
    let input = &bytes[parsed.body_offset..];
    let body = match parsed.body_kind {
        BodyKind::None => Vec::new(),
        BodyKind::ContentLength(expected) => {
            if input.len() < expected {
                return Err(AxisError::TruncatedBody {
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
        return Err(AxisError::ResponseTooLarge { limit: maximum });
    }
    let session_cookie = find_header(&parsed.head.headers, "Set-Cookie")
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            if has_unsafe_http_text(value) {
                Err(AxisError::Http(
                    "Set-Cookie contains unsafe HTTP text".to_string(),
                ))
            } else {
                Ok(Zeroizing::new(value.to_string()))
            }
        })
        .transpose()?;
    Ok(AxisHttpResponse {
        body,
        session_cookie,
    })
}

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, AxisError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| AxisError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| AxisError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| AxisError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(AxisError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| AxisError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(AxisError::Http("truncated chunk".to_string()));
        }
        output.extend_from_slice(&input[cursor..chunk_end]);
        cursor = chunk_end + 2;
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

    const DEVICE_INFO: &str = r#"{"apiVersion":"1.0","context":"smart-home","data":{"propertyList":{"Brand":"AXIS","ProdFullName":"AXIS Q1785-LE Network Camera","ProdNbr":"Q1785-LE","ProdType":"Network Camera","SerialNumber":"ACCC8EAF8C30","Version":"12.1.0"}}}"#;
    const API_LIST: &str = r#"{"apiVersion":"1.0","context":"smart-home","data":{"apiList":[{"id":"systemready","version":"1.0","status":"released"},{"id":"basic-device-info","version":"1.2","status":"released"}]}}"#;
    const PTZ_API_LIST: &str = r#"{"apiVersion":"1.0","context":"smart-home","data":{"apiList":[{"id":"ptz-control","version":"1.0","status":"released"},{"id":"basic-device-info","version":"1.2","status":"released"}]}}"#;

    fn response(body: &str) -> Vec<u8> {
        format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()
    }

    fn text_response(body: &str) -> Vec<u8> {
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()
    }

    fn queue_response(body: &str) -> Vec<u8> {
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nSet-Cookie: ptz=lease-token; Path=/\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).into_bytes()
    }

    fn start_server(
        payloads: Vec<Vec<u8>>,
    ) -> (u16, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let handle = thread::spawn(move || {
            for payload in payloads {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0u8; 4096];
                loop {
                    let read = stream.read(&mut buffer).unwrap();
                    if read == 0 {
                        break;
                    }
                    bytes.extend_from_slice(&buffer[..read]);
                    let header_end = bytes.windows(4).position(|window| window == b"\r\n\r\n");
                    if let Some(header_end) = header_end {
                        let head = String::from_utf8_lossy(&bytes[..header_end]);
                        let length = head
                            .lines()
                            .find_map(|line| line.strip_prefix("Content-Length: "))
                            .and_then(|value| value.parse::<usize>().ok())
                            .unwrap_or(0);
                        if bytes.len() >= header_end + 4 + length {
                            break;
                        }
                    }
                }
                captured
                    .lock()
                    .unwrap()
                    .push(String::from_utf8(bytes).unwrap());
                stream.write_all(&payload).unwrap();
            }
        });
        (port, requests, handle)
    }

    fn config(port: u16) -> AxisConfig {
        AxisConfig::new(
            BridgeId::trusted("axis.test"),
            format!("http://127.0.0.1:{port}"),
            VaultRef::trusted("vault://axis/test"),
        )
        .unwrap()
    }

    fn credentials() -> AxisCredentials {
        AxisCredentials::new("root", "secret").unwrap()
    }

    fn grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:axis-test"),
                principal.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    fn command_grant(runtime: &mut SmartHomeRuntime, principal: &AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:axis-command-test"),
                principal.clone(),
                PrivilegeTier::HumanApproval,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
    }

    #[test]
    fn documented_axis_mdns_services_become_credential_candidates() {
        for service_type in [VIDEO_MDNS_SERVICE_TYPE, NVR_MDNS_SERVICE_TYPE] {
            let advertisement = MdnsAdvertisement::new(
                service_type,
                "AXIS Q1785-LE - ACCC8EAF8C30",
                "axis-accc8eaf8c30.local",
                443,
                1_000,
            )
            .unwrap()
            .with_address("192.0.2.25")
            .unwrap()
            .with_txt("macaddress", "ACCC8EAF8C30")
            .unwrap();
            let record = discovery_record(&advertisement).unwrap();
            assert_eq!(record.native_bridge_id, "accc8eaf8c30");
            assert_eq!(record.address.as_deref(), Some("https://192.0.2.25:443"));
            assert_eq!(record.confidence, DiscoveryConfidence::Candidate);
            assert_eq!(record.pairing_requirement, PairingRequirement::Credentials);
        }

        let advertisement = MdnsAdvertisement::new(
            VIDEO_MDNS_SERVICE_TYPE,
            "AXIS Q1785-LE - ACCC8EAF8C30",
            "axis-accc8eaf8c30.local",
            443,
            1_000,
        )
        .unwrap();
        assert_eq!(
            discovery_record(&advertisement).unwrap().native_bridge_id,
            "accc8eaf8c30"
        );
    }

    #[test]
    fn config_requires_https_outside_loopback_and_rejects_url_credentials() {
        assert!(AxisConfig::new(
            BridgeId::trusted("axis.insecure"),
            "http://192.0.2.25",
            VaultRef::trusted("vault://axis/test")
        )
        .is_err());
        assert!(AxisConfig::new(
            BridgeId::trusted("axis.userinfo"),
            "https://root:secret@axis.local",
            VaultRef::trusted("vault://axis/test")
        )
        .is_err());
        assert!(AxisConfig::new(
            BridgeId::trusted("axis.secure"),
            "https://axis.local",
            VaultRef::trusted("vault://axis/test")
        )
        .is_ok());
    }

    #[test]
    fn response_debug_redacts_transport_only_queue_cookie() {
        let response = AxisHttpResponse::with_session_cookie(Vec::new(), "ptz=secret").unwrap();
        let debug = format!("{response:?}");
        assert!(debug.contains("[REDACTED]"));
        assert!(!debug.contains("secret"));
    }

    #[test]
    fn waiting_queue_request_is_dropped_before_control_is_rejected() {
        let waiting = r#"<a name="2"></a><a name="8"></a><a name="30"></a>"#;
        let (port, requests, handle) =
            start_server(vec![queue_response(waiting), text_response("")]);
        let mut client =
            AxisClient::new(config(port), credentials(), AxisLanTransport::default()).unwrap();
        assert!(matches!(
            client.recall_ptz_preset(1, 3, 50, AxisPtzQueueMode::Required),
            Err(AxisError::PtzQueueUnavailable)
        ));
        handle.join().unwrap();
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].contains("control=drop&camera=1"));
        assert!(requests[1].contains("Cookie: ptz=lease-token"));
    }

    #[test]
    fn real_http_inspection_installs_confirmed_axis_camera() {
        let (port, requests, handle) =
            start_server(vec![response(DEVICE_INFO), response(API_LIST)]);
        let client =
            AxisClient::new(config(port), credentials(), AxisLanTransport::default()).unwrap();
        let mut integration = AxisRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:axis-read");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();

        let device = runtime.registry().device(&installed.device_id).unwrap();
        assert_eq!(device.manufacturer, "AXIS");
        assert_eq!(device.model, "Q1785-LE");
        assert_eq!(device.serial.as_deref(), Some("ACCC8EAF8C30"));
        let camera = runtime
            .registry()
            .entity(&installed.camera_entity_id)
            .unwrap();
        assert_eq!(camera.kind, EntityKind::Camera);
        assert_eq!(
            camera.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        let bridge = runtime.registry().bridge(&installed.bridge_id).unwrap();
        assert_eq!(
            bridge.auth_ref.as_ref().unwrap().as_str(),
            "vault://axis/test"
        );

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].contains(&format!("POST {BASIC_DEVICE_INFO_PATH} HTTP/1.1")));
        assert!(requests[1].contains(&format!("POST {API_DISCOVERY_PATH} HTTP/1.1")));
        assert!(requests
            .iter()
            .all(|request| request.contains("Authorization: Basic cm9vdDpzZWNyZXQ=")));
        assert!(requests.iter().all(|request| !request.contains("vault://")));
    }

    #[test]
    fn real_http_ptz_probe_queue_recall_and_bounded_move_are_exact() {
        let queue_granted = r#"<a name="1"></a><a name="0"></a><a name="30"></a>"#;
        let (port, requests, handle) = start_server(vec![
            response(DEVICE_INFO),
            response(PTZ_API_LIST),
            text_response("continuouspantiltmove=-100..100,-100..100\ngotoserverpresetno=integer\nspeed=integer\nquery=position\n"),
            text_response("pan=12.5\ntilt=-3.0\nzoom=2200\n"),
            text_response("Preset Positions for camera 1\npresetposno1=Home\npresetposno3=Driveway\n"),
            text_response("root.PTZ.Various.V1.CtlQueueing=yes\n"),
            queue_response(queue_granted),
            text_response(""),
            text_response(""),
            queue_response(queue_granted),
            text_response(""),
            text_response(""),
            text_response(""),
        ]);
        let client =
            AxisClient::new(config(port), credentials(), AxisLanTransport::default()).unwrap();
        let mut integration = AxisRuntimeIntegration::new(client);
        let principal = AgentId::trusted("agent:axis-ptz");
        let mut runtime = SmartHomeRuntime::new();
        grant(&mut runtime, &principal);
        command_grant(&mut runtime, &principal);
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal.clone(), 5_000)
            .unwrap();
        let camera = runtime
            .registry()
            .entity(&installed.camera_entity_id)
            .unwrap();
        assert!(camera
            .capabilities
            .iter()
            .any(|capability| capability.capability_id == CapabilityId::trusted("camera.ptz")));

        let denied = RuntimeCommandToolRequest::new(
            installed.camera_entity_id.clone(),
            CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz),
            Value::Object(vec![
                ("direction".to_string(), Value::Text("right".to_string())),
                ("speed".to_string(), Value::Integer(25)),
                ("duration_ms".to_string(), Value::Integer(1)),
            ]),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                AgentId::trusted("agent:axis-denied"),
                denied,
                5_500,
            ),
            Err(AxisError::Runtime(_))
        ));
        let invalid = RuntimeCommandToolRequest::new(
            installed.camera_entity_id.clone(),
            CommandType::DeviceControl(DeviceControlCommandType::RecallCameraPtzPreset),
            Value::Object(vec![
                ("preset_id".to_string(), Value::Integer(99)),
                ("speed".to_string(), Value::Integer(50)),
            ]),
        );
        assert!(matches!(
            integration.dispatch_command_authorized(
                &mut runtime,
                principal.clone(),
                invalid,
                5_600,
            ),
            Err(AxisError::InvalidCommandArguments { .. })
        ));
        let preset = integration
            .dispatch_command_authorized(
                &mut runtime,
                principal.clone(),
                RuntimeCommandToolRequest::new(
                    installed.camera_entity_id.clone(),
                    CommandType::DeviceControl(DeviceControlCommandType::RecallCameraPtzPreset),
                    Value::Object(vec![
                        ("preset_id".to_string(), Value::Integer(3)),
                        ("speed".to_string(), Value::Integer(50)),
                    ]),
                ),
                6_000,
            )
            .unwrap();
        let movement = integration
            .dispatch_command_authorized(
                &mut runtime,
                principal,
                RuntimeCommandToolRequest::new(
                    installed.camera_entity_id.clone(),
                    CommandType::DeviceControl(DeviceControlCommandType::MoveCameraPtz),
                    Value::Object(vec![
                        ("direction".to_string(), Value::Text("left".to_string())),
                        ("speed".to_string(), Value::Integer(25)),
                        ("duration_ms".to_string(), Value::Integer(1)),
                    ]),
                ),
                7_000,
            )
            .unwrap();
        handle.join().unwrap();

        assert!(preset
            .message
            .as_deref()
            .is_some_and(|message| message.contains("preset 3")));
        assert!(movement
            .message
            .as_deref()
            .is_some_and(|message| message.contains("bounded PTZ left")));
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 13);
        assert!(requests[2].starts_with(&format!("GET {PTZ_CONTROL_PATH}?info=1&camera=1 ")));
        assert!(requests[5].contains("group=PTZ.Various.V1.CtlQueueing"));
        assert!(requests[6].contains("control=request&camera=1"));
        assert!(requests[7].contains("gotoserverpresetno=3&speed=50&camera=1"));
        assert!(requests[7].contains("Cookie: ptz=lease-token"));
        assert!(requests[8].contains("control=drop&camera=1"));
        assert!(requests[10].contains("continuouspantiltmove=-25,0&camera=1"));
        assert!(requests[11].contains("continuouspantiltmove=0,0&camera=1"));
        assert!(requests[12].contains("control=drop&camera=1"));
        assert!(requests
            .iter()
            .all(|request| request.contains("Authorization: Basic cm9vdDpzZWNyZXQ=")));
        assert!(requests.iter().all(|request| !request.contains("vault://")));
    }

    #[derive(Debug)]
    struct CountingTransport(Arc<AtomicUsize>);

    impl AxisTransport for CountingTransport {
        fn execute(
            &mut self,
            _plan: &LocalHttpRequestPlan,
            _credentials: &AxisCredentials,
            _session_cookie: Option<&str>,
        ) -> Result<AxisHttpResponse, AxisError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(AxisHttpResponse::new(Vec::new()))
        }
    }

    #[test]
    fn denied_read_reaches_no_transport_or_credentials() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = AxisClient::new(
            AxisConfig::new(
                BridgeId::trusted("axis.denied"),
                "http://127.0.0.1",
                VaultRef::trusted("vault://axis/denied"),
            )
            .unwrap(),
            credentials(),
            CountingTransport(Arc::clone(&calls)),
        )
        .unwrap();
        let mut integration = AxisRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_and_install_authorized(
                &mut SmartHomeRuntime::new(),
                AgentId::trusted("agent:denied"),
                5_000,
            ),
            Err(AxisError::Runtime(_))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn parser_sorts_api_inventory() {
        let properties: JsonValue = serde_json::from_str(DEVICE_INFO).unwrap();
        let apis: JsonValue = serde_json::from_str(API_LIST).unwrap();
        let snapshot = parse_snapshot(&properties, &apis).unwrap();
        assert_eq!(snapshot.device.product_number, "Q1785-LE");
        assert_eq!(snapshot.apis[0].id, "basic-device-info");
        assert_eq!(snapshot.apis[1].id, "systemready");
    }

    #[test]
    fn client_rejects_vapix_errors() {
        let (port, _requests, handle) = start_server(vec![response(
            r#"{"apiVersion":"1.0","error":{"code":1000,"message":"bad input"}}"#,
        )]);
        let mut client =
            AxisClient::new(config(port), credentials(), AxisLanTransport::default()).unwrap();
        assert!(matches!(
            client.inspect(),
            Err(AxisError::Api { code: 1000, message }) if message == "bad input"
        ));
        handle.join().unwrap();
    }

    #[test]
    fn response_bounds_are_enforced() {
        assert!(matches!(
            decode_http_response(&response("{}"), 1),
            Err(AxisError::ResponseTooLarge { limit: 1 })
        ));
    }
}
