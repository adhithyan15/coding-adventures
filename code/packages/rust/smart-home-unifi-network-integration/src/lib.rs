//! Authenticated local UniFi Network application and device inspection for D23.

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
pub const INTEGRATION_ID: &str = "unifi";
pub const PROTOCOL_ID: &str = "unifi_network_integration_api";
pub const API_BASE_PATH: &str = "/proxy/network/integration";
pub const INFO_PATH: &str = "/v1/info";
pub const SITES_PATH: &str = "/v1/sites";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const PAGE_LIMIT: usize = 100;
pub const MAX_SITES: usize = 128;
pub const MAX_DEVICES: usize = 2_048;
const MAX_SECRET_BYTES: usize = 8 * 1024;
const MAX_TEXT_BYTES: usize = 1_024;

#[derive(Debug)]
pub enum UniFiError {
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

impl fmt::Display for UniFiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid UniFi input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid UniFi URL: {error}"),
            Self::Io(message) => write!(formatter, "UniFi LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "UniFi TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid UniFi HTTP response: {message}"),
            Self::HttpStatus { operation, status } => {
                write!(formatter, "UniFi {operation} returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "UniFi response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "UniFi response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid UniFi JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "UniFi response is missing {field}"),
            Self::Runtime(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for UniFiError {}

impl From<LocalHttpError> for UniFiError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for UniFiError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for UniFiError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for UniFiError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

pub struct UniFiApiKey {
    token: Zeroizing<String>,
}

impl UniFiApiKey {
    pub fn new(token: impl Into<String>) -> Result<Self, UniFiError> {
        let token = token.into();
        if token.trim().is_empty()
            || token.len() > MAX_SECRET_BYTES
            || token.bytes().any(|byte| byte <= 0x20 || byte == 0x7f)
        {
            return Err(UniFiError::Validation(
                "API key must be bounded non-whitespace HTTP text".to_string(),
            ));
        }
        Ok(Self {
            token: Zeroizing::new(token),
        })
    }
}

impl fmt::Debug for UniFiApiKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UniFiApiKey([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniFiConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub api_key_ref: VaultRef,
    pub timeout: Duration,
}

impl UniFiConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        api_key_ref: VaultRef,
    ) -> Result<Self, UniFiError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(UniFiError::MissingField("base URL host"))?;
        let secure = parsed.scheme == "https";
        let test_loopback = parsed.scheme == "http" && is_loopback_host(host);
        if (!secure && !test_loopback)
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(UniFiError::Validation(
                "base URL must be a credential-free HTTPS origin; HTTP is test-only on loopback"
                    .to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            api_key_ref,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn endpoint(&self) -> Result<LocalHttpEndpoint, UniFiError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(UniFiError::MissingField("base URL host"))?;
        let scheme = match parsed.scheme.as_str() {
            "https" => LocalHttpScheme::Https,
            "http" if is_loopback_host(host) => LocalHttpScheme::Http,
            _ => {
                return Err(UniFiError::Validation(
                    "UniFi endpoint is not approved".to_string(),
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
        .with_base_path(API_BASE_PATH)
        .with_metadata(Metadata::new(
            "http.profile",
            "unifi.network.integration-api",
        )))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniFiSite {
    pub id: String,
    pub name: String,
    pub internal_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniFiDevice {
    pub site_id: String,
    pub site_name: String,
    pub id: String,
    pub name: String,
    pub model: String,
    pub mac_address: String,
    pub ip_address: String,
    pub state: String,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniFiSnapshot {
    pub application_version: String,
    pub sites: Vec<UniFiSite>,
    pub devices: Vec<UniFiDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniFiRequestPlans {
    pub endpoint: LocalHttpEndpoint,
    pub api_key_ref: VaultRef,
    pub timeout_ms: u64,
    pub info: LocalHttpRequestPlan,
    pub sites: LocalHttpRequestPlan,
}

pub trait UniFiTransport {
    fn inspect(
        &mut self,
        plans: &UniFiRequestPlans,
        api_key: &UniFiApiKey,
    ) -> Result<UniFiSnapshot, UniFiError>;
}

pub struct UniFiLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    maximum_response_bytes: usize,
}

impl Default for UniFiLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl UniFiLanTransport {
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
        api_key: &UniFiApiKey,
    ) -> Result<HttpResponse, UniFiError> {
        let request = Zeroizing::new(encode_http_request(plan, api_key.token.as_str())?);
        let url = Url::parse(&plan.url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(UniFiError::MissingField("request URL host"))?;
        let port = url
            .effective_port()
            .ok_or(UniFiError::MissingField("request URL port"))?;
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
                    .map_err(|error| UniFiError::Tls(error.to_string()))?;
                write_request(&mut stream, request.as_slice())?;
                let bytes = Zeroizing::new(read_bounded(&mut stream, self.maximum_response_bytes)?);
                stream
                    .close_notify()
                    .map_err(|error| UniFiError::Tls(error.to_string()))?;
                bytes
            }
            _ => {
                return Err(UniFiError::Validation(
                    "UniFi transport requires HTTPS or loopback HTTP".to_string(),
                ))
            }
        };
        decode_http_response(response.as_slice(), self.maximum_response_bytes)
    }

    fn get_json(
        &mut self,
        plan: &LocalHttpRequestPlan,
        api_key: &UniFiApiKey,
        operation: &'static str,
    ) -> Result<JsonValue, UniFiError> {
        let response = self.request(plan, api_key)?;
        if response.status != 200 {
            return Err(UniFiError::HttpStatus {
                operation,
                status: response.status,
            });
        }
        Ok(serde_json::from_slice(&response.body)?)
    }
}

impl UniFiTransport for UniFiLanTransport {
    fn inspect(
        &mut self,
        plans: &UniFiRequestPlans,
        api_key: &UniFiApiKey,
    ) -> Result<UniFiSnapshot, UniFiError> {
        let info = self.get_json(&plans.info, api_key, "application info")?;
        let application_version = parse_application_version(&info)?;

        let mut sites = Vec::new();
        let mut site_offset = 0usize;
        loop {
            let plan = if site_offset == 0 {
                plans.sites.clone()
            } else {
                paginated_plan(
                    &plans.endpoint,
                    &plans.api_key_ref,
                    SITES_PATH,
                    site_offset,
                    plans.timeout_ms,
                )?
            };
            let page = parse_site_page(&self.get_json(&plan, api_key, "site list")?, site_offset)?;
            let finished = append_page(&mut sites, page, MAX_SITES, "sites")?;
            if finished {
                break;
            }
            site_offset = sites.len();
        }

        let mut devices = Vec::new();
        for site in &sites {
            let mut site_devices = Vec::new();
            let mut device_offset = 0usize;
            loop {
                let path = format!("/v1/sites/{}/devices", safe_path_id(&site.id)?);
                let plan = paginated_plan(
                    &plans.endpoint,
                    &plans.api_key_ref,
                    &path,
                    device_offset,
                    plans.timeout_ms,
                )?;
                let page = parse_device_page(
                    &self.get_json(&plan, api_key, "adopted device list")?,
                    device_offset,
                    site,
                )?;
                let remaining = MAX_DEVICES.saturating_sub(devices.len());
                let finished = append_page(&mut site_devices, page, remaining, "devices")?;
                if finished {
                    break;
                }
                device_offset = site_devices.len();
            }
            devices.extend(site_devices);
        }
        validate_snapshot_uniqueness(&sites, &devices)?;
        Ok(UniFiSnapshot {
            application_version,
            sites,
            devices,
        })
    }
}

pub struct UniFiClient<T> {
    config: UniFiConfig,
    api_key: UniFiApiKey,
    transport: T,
    plans: UniFiRequestPlans,
}

impl<T: UniFiTransport> UniFiClient<T> {
    pub fn new(
        config: UniFiConfig,
        api_key: UniFiApiKey,
        transport: T,
    ) -> Result<Self, UniFiError> {
        let endpoint = config.endpoint()?;
        let timeout_ms = duration_ms(config.timeout);
        let info = get_plan(&endpoint, &config.api_key_ref, INFO_PATH, timeout_ms)?;
        let sites = paginated_plan(&endpoint, &config.api_key_ref, SITES_PATH, 0, timeout_ms)?;
        let plans = UniFiRequestPlans {
            endpoint,
            api_key_ref: config.api_key_ref.clone(),
            timeout_ms,
            info,
            sites,
        };
        Ok(Self {
            config,
            api_key,
            transport,
            plans,
        })
    }

    pub fn inspect(&mut self) -> Result<UniFiSnapshot, UniFiError> {
        self.transport.inspect(&self.plans, &self.api_key)
    }
}

impl<T> fmt::Debug for UniFiClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UniFiClient")
            .field("config", &self.config)
            .field("api_key", &"[REDACTED]")
            .field("plans", &self.plans)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledUniFiDevice {
    pub device_id: DeviceId,
    pub network_entity_id: EntityId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledUniFiNetwork {
    pub bridge_id: BridgeId,
    pub devices: Vec<InstalledUniFiDevice>,
}

pub struct UniFiRuntimeIntegration<T> {
    client: UniFiClient<T>,
}

impl<T: UniFiTransport> UniFiRuntimeIntegration<T> {
    pub fn new(client: UniFiClient<T>) -> Self {
        Self { client }
    }

    pub fn inspect_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledUniFiNetwork, UniFiError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }
}

pub fn paired_discovery_record(
    config: &UniFiConfig,
    snapshot: &UniFiSnapshot,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, UniFiError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&config.base_url),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| UniFiError::Validation(error.to_string()))?
    .with_display_name("UniFi Network")
    .with_address(config.base_url.clone())
    .with_hardware_model("UniFi Network application")
    .with_firmware_version(snapshot.application_version.clone())
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("unifi.protocol", PROTOCOL_ID)
    .with_metadata("unifi.site_count", snapshot.sites.len().to_string())
    .with_metadata("unifi.device_count", snapshot.devices.len().to_string()))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &UniFiConfig,
    snapshot: &UniFiSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledUniFiNetwork, UniFiError> {
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some("UniFi Network application".to_string());
    bridge.firmware_version = Some(snapshot.application_version.clone());
    bridge.auth_ref = Some(config.api_key_ref.clone());
    bridge.health = aggregate_health(&snapshot.devices);
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("https_endpoint", &config.base_url)?];
    bridge.metadata = vec![
        Metadata::new("unifi.transport", "local_api_key"),
        Metadata::new("unifi.site_count", snapshot.sites.len().to_string()),
        Metadata::new("unifi.device_count", snapshot.devices.len().to_string()),
    ];
    runtime.upsert_bridge(bridge)?;

    let mut installed = Vec::with_capacity(snapshot.devices.len());
    for native in &snapshot.devices {
        let site_id = stable_component(&native.site_id);
        let native_id = stable_component(&native.id);
        if site_id.is_empty() || native_id.is_empty() {
            return Err(UniFiError::Validation(
                "site and device IDs must have stable components".to_string(),
            ));
        }
        let device_id = DeviceId::trusted(format!("unifi:{site_id}:{native_id}"));
        let network_entity_id = EntityId::trusted(format!("unifi:{site_id}:{native_id}:network"));
        let health = device_health(native);
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "Ubiquiti".to_string(),
            model: native.model.clone(),
            name: native.name.clone(),
            serial: Some(native.mac_address.clone()),
            firmware_version: None,
            room_id: None,
            entity_ids: vec![network_entity_id.clone()],
            identifiers: vec![
                protocol_identifier("device_id", &native.id)?,
                protocol_identifier("mac_address", &native.mac_address)?,
            ],
            health,
            metadata: vec![
                Metadata::new("unifi.site_id", native.site_id.clone()),
                Metadata::new("unifi.site_name", native.site_name.clone()),
                Metadata::new("unifi.native_state", native.state.clone()),
            ],
        })?;
        runtime.upsert_entity(Entity {
            entity_id: network_entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::NetworkDiagnostic,
            name: format!("{} network health", native.name),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("network.health"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: network_entity_id.clone(),
                value: device_value(native),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![Metadata::new("unifi.protocol", PROTOCOL_ID)],
        })?;
        installed.push(InstalledUniFiDevice {
            device_id,
            network_entity_id,
        });
    }
    Ok(InstalledUniFiNetwork {
        bridge_id: config.bridge_id.clone(),
        devices: installed,
    })
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), UniFiError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(UniFiError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

#[derive(Debug)]
struct Page<T> {
    offset: usize,
    total_count: usize,
    data: Vec<T>,
}

fn parse_application_version(value: &JsonValue) -> Result<String, UniFiError> {
    required_text(
        value
            .as_object()
            .ok_or(UniFiError::MissingField("application info object"))?,
        "applicationVersion",
    )
}

fn parse_site_page(
    value: &JsonValue,
    expected_offset: usize,
) -> Result<Page<UniFiSite>, UniFiError> {
    let object = value
        .as_object()
        .ok_or(UniFiError::MissingField("site page object"))?;
    let (offset, _count, total_count, data) = page_fields(object, expected_offset)?;
    let mut sites = Vec::with_capacity(data.len());
    for item in data {
        let object = item
            .as_object()
            .ok_or(UniFiError::MissingField("site data item"))?;
        let id = required_text(object, "id")?;
        safe_path_id(&id)?;
        sites.push(UniFiSite {
            id,
            name: required_text(object, "name")?,
            internal_reference: optional_text(object, "internalReference")?,
        });
    }
    Ok(Page {
        offset,
        total_count,
        data: sites,
    })
}

fn parse_device_page(
    value: &JsonValue,
    expected_offset: usize,
    site: &UniFiSite,
) -> Result<Page<UniFiDevice>, UniFiError> {
    let object = value
        .as_object()
        .ok_or(UniFiError::MissingField("device page object"))?;
    let (offset, _count, total_count, data) = page_fields(object, expected_offset)?;
    let mut devices = Vec::with_capacity(data.len());
    for item in data {
        let object = item
            .as_object()
            .ok_or(UniFiError::MissingField("device data item"))?;
        let id = required_text(object, "id")?;
        safe_path_id(&id)?;
        let state = required_text(object, "state")?.to_ascii_uppercase();
        validate_device_state(&state)?;
        devices.push(UniFiDevice {
            site_id: site.id.clone(),
            site_name: site.name.clone(),
            id,
            name: required_text(object, "name")?,
            model: required_text(object, "model")?,
            mac_address: required_text(object, "macAddress")?,
            ip_address: required_text(object, "ipAddress")?,
            state,
            features: parse_features(object.get("features"))?,
        });
    }
    Ok(Page {
        offset,
        total_count,
        data: devices,
    })
}

fn page_fields(
    object: &JsonMap<String, JsonValue>,
    expected_offset: usize,
) -> Result<(usize, usize, usize, &Vec<JsonValue>), UniFiError> {
    let offset = required_usize(object, "offset")?;
    let count = required_usize(object, "count")?;
    let total_count = required_usize(object, "totalCount")?;
    let data = object
        .get("data")
        .and_then(JsonValue::as_array)
        .ok_or(UniFiError::MissingField("data"))?;
    if offset != expected_offset
        || count != data.len()
        || offset.saturating_add(count) > total_count
    {
        return Err(UniFiError::Validation(
            "pagination metadata does not match the requested page".to_string(),
        ));
    }
    if count == 0 && offset < total_count {
        return Err(UniFiError::Validation(
            "pagination made no progress before totalCount".to_string(),
        ));
    }
    Ok((offset, count, total_count, data))
}

fn append_page<T>(
    target: &mut Vec<T>,
    page: Page<T>,
    maximum: usize,
    label: &str,
) -> Result<bool, UniFiError> {
    if page.offset != target.len() || page.total_count > maximum {
        return Err(UniFiError::Validation(format!(
            "{label} pagination exceeds bounds or is discontinuous"
        )));
    }
    target.extend(page.data);
    if target.len() > maximum || target.len() > page.total_count {
        return Err(UniFiError::Validation(format!(
            "{label} pagination exceeded declared bounds"
        )));
    }
    Ok(target.len() == page.total_count)
}

fn required_usize(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<usize, UniFiError> {
    let value = object
        .get(field)
        .and_then(JsonValue::as_u64)
        .ok_or(UniFiError::MissingField(field))?;
    usize::try_from(value)
        .map_err(|_| UniFiError::Validation(format!("{field} exceeds platform bounds")))
}

fn required_text(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, UniFiError> {
    let value = object
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or(UniFiError::MissingField(field))?;
    validate_text(field, value).map(str::to_string)
}

fn optional_text(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Option<String>, UniFiError> {
    match object.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => {
            let value = value
                .as_str()
                .ok_or_else(|| UniFiError::Validation(format!("{field} must be a string")))?;
            validate_text(field, value).map(|value| Some(value.to_string()))
        }
    }
}

fn validate_text<'a>(field: &str, value: &'a str) -> Result<&'a str, UniFiError> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES || value.contains(['\r', '\n', '\0'])
    {
        Err(UniFiError::Validation(format!(
            "{field} is empty, oversized, or unsafe"
        )))
    } else {
        Ok(value)
    }
}

fn parse_features(value: Option<&JsonValue>) -> Result<Vec<String>, UniFiError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut features = match value {
        JsonValue::Array(values) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .ok_or_else(|| {
                        UniFiError::Validation("features array must contain strings".to_string())
                    })
                    .and_then(|value| validate_text("feature", value).map(str::to_string))
            })
            .collect::<Result<Vec<_>, _>>()?,
        JsonValue::Object(values) => values
            .iter()
            .filter(|(_, value)| !value.is_null())
            .map(|(name, _)| validate_text("feature", name).map(str::to_string))
            .collect::<Result<Vec<_>, _>>()?,
        JsonValue::Null => Vec::new(),
        _ => {
            return Err(UniFiError::Validation(
                "features must be an array or object".to_string(),
            ))
        }
    };
    features.sort();
    features.dedup();
    Ok(features)
}

fn validate_device_state(state: &str) -> Result<(), UniFiError> {
    if matches!(
        state,
        "ONLINE"
            | "OFFLINE"
            | "PENDING_ADOPTION"
            | "UPDATING"
            | "GETTING_READY"
            | "ADOPTING"
            | "DELETING"
            | "CONNECTION_INTERRUPTED"
            | "ISOLATED"
    ) {
        Ok(())
    } else {
        Err(UniFiError::Validation(format!(
            "unknown UniFi device state {state}"
        )))
    }
}

fn validate_snapshot_uniqueness(
    sites: &[UniFiSite],
    devices: &[UniFiDevice],
) -> Result<(), UniFiError> {
    let mut site_ids = BTreeSet::new();
    for site in sites {
        if !site_ids.insert(site.id.as_str()) {
            return Err(UniFiError::Validation(
                "site IDs must be unique".to_string(),
            ));
        }
    }
    let mut device_ids = BTreeSet::new();
    for device in devices {
        if !site_ids.contains(device.site_id.as_str())
            || !device_ids.insert((device.site_id.as_str(), device.id.as_str()))
        {
            return Err(UniFiError::Validation(
                "device IDs must be unique within known sites".to_string(),
            ));
        }
    }
    Ok(())
}

fn safe_path_id(value: &str) -> Result<&str, UniFiError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        Err(UniFiError::Validation(
            "API identifier is unsafe for a path segment".to_string(),
        ))
    } else {
        Ok(value)
    }
}

fn device_health(device: &UniFiDevice) -> Health {
    match device.state.as_str() {
        "ONLINE" => Health::Online,
        "UPDATING" | "GETTING_READY" | "ADOPTING" | "PENDING_ADOPTION" => Health::Degraded,
        "OFFLINE" | "DELETING" | "CONNECTION_INTERRUPTED" | "ISOLATED" => Health::Offline,
        _ => Health::Unknown,
    }
}

fn aggregate_health(devices: &[UniFiDevice]) -> Health {
    if devices.is_empty() {
        return Health::Degraded;
    }
    let health = devices.iter().map(device_health).collect::<Vec<_>>();
    if health.iter().all(|value| *value == Health::Offline) {
        Health::Offline
    } else if health.iter().any(|value| *value != Health::Online) {
        Health::Degraded
    } else {
        Health::Online
    }
}

fn device_value(device: &UniFiDevice) -> Value {
    Value::Object(vec![
        ("site_id".to_string(), Value::Text(device.site_id.clone())),
        (
            "site_name".to_string(),
            Value::Text(device.site_name.clone()),
        ),
        ("device_id".to_string(), Value::Text(device.id.clone())),
        ("model".to_string(), Value::Text(device.model.clone())),
        (
            "mac_address".to_string(),
            Value::Text(device.mac_address.clone()),
        ),
        (
            "ip_address".to_string(),
            Value::Text(device.ip_address.clone()),
        ),
        ("state".to_string(), Value::Text(device.state.clone())),
        (
            "features".to_string(),
            Value::Array(device.features.iter().cloned().map(Value::Text).collect()),
        ),
    ])
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, UniFiError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| UniFiError::Validation(error.to_string()))
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

fn get_plan(
    endpoint: &LocalHttpEndpoint,
    api_key_ref: &VaultRef,
    path: &str,
    timeout_ms: u64,
) -> Result<LocalHttpRequestPlan, UniFiError> {
    Ok(LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
        .with_accept("application/json")
        .with_timeout_ms(timeout_ms)
        .with_auth(LocalHttpAuth::HeaderToken {
            header_name: "X-API-Key".to_string(),
            vault_ref: api_key_ref.clone(),
        })
        .plan(endpoint, Vec::new())?)
}

fn paginated_plan(
    endpoint: &LocalHttpEndpoint,
    api_key_ref: &VaultRef,
    path: &str,
    offset: usize,
    timeout_ms: u64,
) -> Result<LocalHttpRequestPlan, UniFiError> {
    get_plan(
        endpoint,
        api_key_ref,
        &format!("{path}?offset={offset}&limit={PAGE_LIMIT}"),
        timeout_ms,
    )
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn encode_http_request(plan: &LocalHttpRequestPlan, api_key: &str) -> Result<Vec<u8>, UniFiError> {
    if api_key.is_empty()
        || api_key.len() > MAX_SECRET_BYTES
        || api_key.bytes().any(|byte| byte <= 0x20 || byte == 0x7f)
    {
        return Err(UniFiError::Validation(
            "API key is unsafe for an HTTP header".to_string(),
        ));
    }
    let url = Url::parse(&plan.url)?;
    let host = url
        .host
        .as_deref()
        .ok_or(UniFiError::MissingField("request URL host"))?;
    let port = url
        .effective_port()
        .ok_or(UniFiError::MissingField("request URL port"))?;
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
        return Err(UniFiError::Validation(
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
            || header.name.eq_ignore_ascii_case("X-API-Key")
        {
            continue;
        }
        if header.name.contains(['\r', '\n', '\0']) || header.value.contains(['\r', '\n', '\0']) {
            return Err(UniFiError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    request
        .extend_from_slice(format!("X-API-Key: {api_key}\r\nContent-Length: 0\r\n\r\n").as_bytes());
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, UniFiError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| UniFiError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| UniFiError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| UniFiError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(UniFiError::Io(
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

fn write_request(writer: &mut dyn Write, request: &[u8]) -> Result<(), UniFiError> {
    writer
        .write_all(request)
        .map_err(|error| UniFiError::Io(error.to_string()))
}

fn read_bounded(reader: &mut dyn Read, maximum: usize) -> Result<Vec<u8>, UniFiError> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| UniFiError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if read > maximum.saturating_sub(bytes.len()) {
            return Err(UniFiError::ResponseTooLarge { limit: maximum });
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

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<HttpResponse, UniFiError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| UniFiError::Http(error.to_string()))?;
    let status = parsed.head.status;
    let mut headers = parsed.head.headers;
    let input = &bytes[parsed.body_offset..];
    let body = match (|| {
        let body = match parsed.body_kind {
            BodyKind::None => Vec::new(),
            BodyKind::ContentLength(expected) => {
                if input.len() < expected {
                    return Err(UniFiError::TruncatedBody {
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
            return Err(UniFiError::ResponseTooLarge { limit: maximum });
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

fn decode_chunked(input: &[u8], maximum: usize) -> Result<Vec<u8>, UniFiError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let offset = input
            .get(cursor..)
            .and_then(|tail| tail.windows(2).position(|window| window == b"\r\n"))
            .ok_or_else(|| UniFiError::Http("missing chunk-size terminator".to_string()))?;
        let end = cursor + offset;
        let size_text = std::str::from_utf8(&input[cursor..end])
            .map_err(|_| UniFiError::Http("chunk size is not ASCII".to_string()))?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| UniFiError::Http("invalid chunk size".to_string()))?;
        cursor = end + 2;
        if size == 0 {
            return Ok(output);
        }
        if size > maximum.saturating_sub(output.len()) {
            return Err(UniFiError::ResponseTooLarge { limit: maximum });
        }
        let chunk_end = cursor
            .checked_add(size)
            .ok_or_else(|| UniFiError::Http("chunk size overflow".to_string()))?;
        if input.len() < chunk_end + 2 || &input[chunk_end..chunk_end + 2] != b"\r\n" {
            return Err(UniFiError::Http("truncated chunk".to_string()));
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

    fn api_key() -> UniFiApiKey {
        UniFiApiKey::new("secret-api-key").unwrap()
    }

    fn config(port: u16) -> UniFiConfig {
        UniFiConfig::new(
            BridgeId::trusted("unifi.test"),
            format!("http://127.0.0.1:{port}"),
            VaultRef::trusted("vault://unifi/test"),
        )
        .unwrap()
    }

    fn snapshot() -> UniFiSnapshot {
        UniFiSnapshot {
            application_version: "10.4.57".to_string(),
            sites: vec![UniFiSite {
                id: "site-1".to_string(),
                name: "Home".to_string(),
                internal_reference: Some("default".to_string()),
            }],
            devices: vec![
                UniFiDevice {
                    site_id: "site-1".to_string(),
                    site_name: "Home".to_string(),
                    id: "device-ap".to_string(),
                    name: "Upstairs AP".to_string(),
                    model: "U7 Pro".to_string(),
                    mac_address: "00:11:22:33:44:55".to_string(),
                    ip_address: "192.0.2.10".to_string(),
                    state: "ONLINE".to_string(),
                    features: vec!["accessPoint".to_string()],
                },
                UniFiDevice {
                    site_id: "site-1".to_string(),
                    site_name: "Home".to_string(),
                    id: "device-switch".to_string(),
                    name: "Garage Switch".to_string(),
                    model: "USW Lite".to_string(),
                    mac_address: "00:11:22:33:44:66".to_string(),
                    ip_address: "192.0.2.11".to_string(),
                    state: "CONNECTION_INTERRUPTED".to_string(),
                    features: vec!["switching".to_string()],
                },
            ],
        }
    }

    #[derive(Debug)]
    struct FixedTransport {
        snapshot: UniFiSnapshot,
        calls: Arc<AtomicUsize>,
    }

    impl UniFiTransport for FixedTransport {
        fn inspect(
            &mut self,
            _plans: &UniFiRequestPlans,
            _api_key: &UniFiApiKey,
        ) -> Result<UniFiSnapshot, UniFiError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.snapshot.clone())
        }
    }

    fn authorize(runtime: &mut SmartHomeRuntime, principal: AgentId) {
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:unifi-test"),
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
        assert!(UniFiConfig::new(
            BridgeId::trusted("unifi.bad"),
            "http://192.0.2.10",
            VaultRef::trusted("vault://unifi/bad")
        )
        .is_err());
        assert!(UniFiConfig::new(
            BridgeId::trusted("unifi.good"),
            "https://unifi.home",
            VaultRef::trusted("vault://unifi/good")
        )
        .is_ok());
        assert!(UniFiConfig::new(
            BridgeId::trusted("unifi.embedded"),
            "https://operator:secret@unifi.home",
            VaultRef::trusted("vault://unifi/embedded")
        )
        .is_err());
    }

    #[test]
    fn api_key_and_client_debug_are_redacted() {
        assert_eq!(format!("{:?}", api_key()), "UniFiApiKey([REDACTED])");
        let client = UniFiClient::new(
            config(443),
            api_key(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::new(AtomicUsize::new(0)),
            },
        )
        .unwrap();
        let debug = format!("{client:?}");
        assert!(!debug.contains("secret-api-key"));
        assert!(debug.contains("vault://unifi/test"));
        assert!(client
            .plans
            .info
            .headers
            .iter()
            .any(|header| header.name == "X-API-Key" && !header.value.contains("secret-api-key")));
    }

    #[test]
    fn denied_read_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = UniFiClient::new(
            config(443),
            api_key(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = UniFiRuntimeIntegration::new(client);
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
    fn authorized_snapshot_installs_confirmed_network_health() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = UniFiClient::new(
            config(443),
            api_key(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = UniFiRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:allowed");
        authorize(&mut runtime, principal.clone());
        let installed = integration
            .inspect_and_install_authorized(&mut runtime, principal, 2_000)
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(installed.devices.len(), 2);
        let online = runtime
            .registry()
            .entity(&installed.devices[0].network_entity_id)
            .unwrap();
        assert_eq!(online.kind, EntityKind::NetworkDiagnostic);
        assert_eq!(
            online.state.as_ref().unwrap().confidence,
            StateConfidence::Confirmed
        );
        assert_eq!(
            runtime
                .registry()
                .device(&installed.devices[0].device_id)
                .unwrap()
                .health,
            Health::Online
        );
        assert_eq!(
            runtime
                .registry()
                .device(&installed.devices[1].device_id)
                .unwrap()
                .health,
            Health::Offline
        );
    }

    #[test]
    fn parsers_validate_pagination_states_and_feature_shapes() {
        let site_page = serde_json::json!({
            "offset": 0,
            "limit": 100,
            "count": 1,
            "totalCount": 1,
            "data": [{"id": "site-1", "name": "Home", "internalReference": "default"}]
        });
        let sites = parse_site_page(&site_page, 0).unwrap();
        assert_eq!(sites.data[0].name, "Home");

        let array_features = serde_json::json!({
            "offset": 0,
            "limit": 100,
            "count": 1,
            "totalCount": 1,
            "data": [{
                "id": "device-1",
                "name": "AP",
                "model": "U7 Pro",
                "macAddress": "00:11:22:33:44:55",
                "ipAddress": "192.0.2.10",
                "state": "online",
                "features": ["accessPoint"]
            }]
        });
        let devices = parse_device_page(&array_features, 0, &sites.data[0]).unwrap();
        assert_eq!(devices.data[0].state, "ONLINE");
        assert_eq!(devices.data[0].features, vec!["accessPoint"]);

        let bad_offset = serde_json::json!({
            "offset": 2,
            "count": 0,
            "totalCount": 2,
            "data": []
        });
        assert!(parse_site_page(&bad_offset, 0).is_err());
        let bad_state = serde_json::json!({
            "offset": 0,
            "count": 1,
            "totalCount": 1,
            "data": [{
                "id": "device-1",
                "name": "AP",
                "model": "U7 Pro",
                "macAddress": "00:11:22:33:44:55",
                "ipAddress": "192.0.2.10",
                "state": "MYSTERY"
            }]
        });
        assert!(parse_device_page(&bad_state, 0, &sites.data[0]).is_err());
    }

    #[test]
    fn loopback_transport_uses_exact_paths_and_private_api_key_header() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            r#"{"applicationVersion":"10.4.57"}"#,
            r#"{"offset":0,"limit":100,"count":2,"totalCount":2,"data":[{"id":"site-1","internalReference":"default","name":"Home"},{"id":"site-2","internalReference":"lab","name":"Lab"}]}"#,
            r#"{"offset":0,"limit":100,"count":2,"totalCount":2,"data":[{"id":"device-ap","name":"Upstairs AP","model":"U7 Pro","macAddress":"00:11:22:33:44:55","ipAddress":"192.0.2.10","state":"ONLINE","features":["accessPoint"],"interfaces":["radios"]},{"id":"device-switch","name":"Garage Switch","model":"USW Lite","macAddress":"00:11:22:33:44:66","ipAddress":"192.0.2.11","state":"UPDATING","features":{"switching":{}},"interfaces":{"ports":[]}}]}"#,
            r#"{"offset":0,"limit":100,"count":1,"totalCount":1,"data":[{"id":"device-lab","name":"Lab Gateway","model":"UDM","macAddress":"00:11:22:33:44:77","ipAddress":"192.0.2.12","state":"ONLINE","features":{"gateway":{}},"interfaces":{"ports":[]}}]}"#,
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
                server_requests.lock().unwrap().push(head);
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                reader.get_mut().write_all(reply.as_bytes()).unwrap();
            }
        });

        let mut client =
            UniFiClient::new(config(port), api_key(), UniFiLanTransport::default()).unwrap();
        let observed = client.inspect().unwrap();
        handle.join().unwrap();
        assert_eq!(observed.application_version, "10.4.57");
        assert_eq!(observed.sites.len(), 2);
        assert_eq!(observed.devices.len(), 3);
        assert!(!format!("{observed:?}").contains("secret-api-key"));

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].starts_with("GET /proxy/network/integration/v1/info HTTP/1.1"));
        assert!(requests[1]
            .starts_with("GET /proxy/network/integration/v1/sites?offset=0&limit=100 HTTP/1.1"));
        assert!(requests[2].starts_with(
            "GET /proxy/network/integration/v1/sites/site-1/devices?offset=0&limit=100 HTTP/1.1"
        ));
        assert!(requests[3].starts_with(
            "GET /proxy/network/integration/v1/sites/site-2/devices?offset=0&limit=100 HTTP/1.1"
        ));
        assert!(requests
            .iter()
            .all(|head| head.contains("X-API-Key: secret-api-key")));
        assert!(requests
            .iter()
            .all(|head| !head.contains("vault://unifi/test")));
    }

    #[test]
    fn response_and_collection_bounds_are_enforced() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}";
        assert!(matches!(
            decode_http_response(response, 1),
            Err(UniFiError::ResponseTooLarge { limit: 1 })
        ));
        let page = Page {
            offset: 0,
            total_count: MAX_SITES + 1,
            data: Vec::<UniFiSite>::new(),
        };
        assert!(append_page(&mut Vec::new(), page, MAX_SITES, "sites").is_err());
        assert!(UniFiApiKey::new("bad\nkey").is_err());
    }
}
