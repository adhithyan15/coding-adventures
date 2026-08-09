//! Authenticated local UniFi Network application and device inspection for D23.

#![forbid(unsafe_code)]

use coding_adventures_sha256::sha256;
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
use smart_home_data_governance::{
    DataCategory, DataDestination, DataGovernanceDecision, DataGovernanceDenial,
    DataGovernancePolicy, DataOperation, DataRetention, DataUseRequest,
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

pub const VERSION: &str = "0.3.0";
pub const INTEGRATION_ID: &str = "unifi";
pub const PROTOCOL_ID: &str = "unifi_network_integration_api";
pub const API_BASE_PATH: &str = "/proxy/network/integration";
pub const INFO_PATH: &str = "/v1/info";
pub const SITES_PATH: &str = "/v1/sites";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const PAGE_LIMIT: usize = 100;
pub const MAX_SITES: usize = 128;
pub const MAX_DEVICES: usize = 2_048;
pub const MAX_CONNECTED_CLIENTS: usize = 8_192;
pub const CLIENT_PRESENCE_RETENTION_MS: u64 = 5 * 60 * 1_000;
pub const MAX_STATISTICS_TARGETS_PER_POLL: usize = 64;
pub const STATISTICS_MIN_POLL_INTERVAL_MS: u64 = 60 * 1_000;
pub const STATISTICS_RETENTION_MS: u64 = 2 * 60 * 1_000;
const MAX_STATISTICS_RADIOS: usize = 64;
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
    PollRateLimited {
        retry_at_ms: u64,
    },
    DataGovernanceDenied(DataGovernanceDenial),
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
            Self::PollRateLimited { retry_at_ms } => write!(
                formatter,
                "UniFi statistics poll is rate limited until {retry_at_ms}"
            ),
            Self::DataGovernanceDenied(reason) => {
                write!(
                    formatter,
                    "UniFi data-governance policy denied the request: {reason:?}"
                )
            }
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

pub struct UniFiPresenceKey {
    bytes: Zeroizing<Vec<u8>>,
}

impl UniFiPresenceKey {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, UniFiError> {
        let bytes = bytes.into();
        if bytes.len() != 32 {
            return Err(UniFiError::Validation(
                "client pseudonymization key must contain exactly 32 bytes".to_string(),
            ));
        }
        Ok(Self {
            bytes: Zeroizing::new(bytes),
        })
    }
}

impl fmt::Debug for UniFiPresenceKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UniFiPresenceKey([REDACTED])")
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
    pub connected_clients: Vec<UniFiConnectedClient>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniFiConnectedClient {
    pub pseudonym: String,
    pub client_type: String,
    pub access_type: Option<String>,
    pub access_authorized: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct UniFiStatisticsTarget {
    pub site_id: String,
    pub device_id: String,
}

impl UniFiStatisticsTarget {
    pub fn new(
        site_id: impl Into<String>,
        device_id: impl Into<String>,
    ) -> Result<Self, UniFiError> {
        let site_id = site_id.into();
        let device_id = device_id.into();
        safe_path_id(&site_id)?;
        safe_path_id(&device_id)?;
        Ok(Self { site_id, device_id })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UniFiRadioStatistics {
    pub frequency_ghz: f64,
    pub tx_retries_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UniFiDeviceStatistics {
    pub target: UniFiStatisticsTarget,
    pub uptime_sec: u64,
    pub load_average_1_min: f64,
    pub load_average_5_min: f64,
    pub load_average_15_min: f64,
    pub cpu_utilization_pct: f64,
    pub memory_utilization_pct: f64,
    pub uplink_tx_rate_bps: Option<u64>,
    pub uplink_rx_rate_bps: Option<u64>,
    pub radios: Vec<UniFiRadioStatistics>,
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

    fn inspect_connected_clients(
        &mut self,
        _plans: &UniFiRequestPlans,
        _api_key: &UniFiApiKey,
        _presence_key: &UniFiPresenceKey,
        _sites: &[UniFiSite],
    ) -> Result<Vec<UniFiConnectedClient>, UniFiError> {
        Err(UniFiError::Validation(
            "transport does not implement connected-client inspection".to_string(),
        ))
    }

    fn inspect_device_statistics(
        &mut self,
        _plans: &UniFiRequestPlans,
        _api_key: &UniFiApiKey,
        _targets: &[UniFiStatisticsTarget],
    ) -> Result<Vec<UniFiDeviceStatistics>, UniFiError> {
        Err(UniFiError::Validation(
            "transport does not implement device-statistics inspection".to_string(),
        ))
    }
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

    fn get_sensitive_json(
        &mut self,
        plan: &LocalHttpRequestPlan,
        api_key: &UniFiApiKey,
        operation: &'static str,
    ) -> Result<SensitiveJson, UniFiError> {
        let response = self.request(plan, api_key)?;
        if response.status != 200 {
            return Err(UniFiError::HttpStatus {
                operation,
                status: response.status,
            });
        }
        Ok(SensitiveJson(serde_json::from_slice(&response.body)?))
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
            connected_clients: Vec::new(),
        })
    }

    fn inspect_connected_clients(
        &mut self,
        plans: &UniFiRequestPlans,
        api_key: &UniFiApiKey,
        presence_key: &UniFiPresenceKey,
        sites: &[UniFiSite],
    ) -> Result<Vec<UniFiConnectedClient>, UniFiError> {
        let mut clients = Vec::new();
        for site in sites {
            let mut site_clients = Vec::new();
            let mut offset = 0usize;
            loop {
                let path = format!("/v1/sites/{}/clients", safe_path_id(&site.id)?);
                let plan = paginated_plan(
                    &plans.endpoint,
                    &plans.api_key_ref,
                    &path,
                    offset,
                    plans.timeout_ms,
                )?;
                let response = self.get_sensitive_json(&plan, api_key, "connected client list")?;
                let page = parse_client_page(&response.0, offset, site, presence_key)?;
                let remaining = MAX_CONNECTED_CLIENTS.saturating_sub(clients.len());
                let finished = append_page(&mut site_clients, page, remaining, "clients")?;
                if finished {
                    break;
                }
                offset = site_clients.len();
            }
            clients.extend(site_clients);
        }
        validate_client_uniqueness(&clients)?;
        Ok(clients)
    }

    fn inspect_device_statistics(
        &mut self,
        plans: &UniFiRequestPlans,
        api_key: &UniFiApiKey,
        targets: &[UniFiStatisticsTarget],
    ) -> Result<Vec<UniFiDeviceStatistics>, UniFiError> {
        validate_statistics_targets(targets)?;
        let mut statistics = Vec::with_capacity(targets.len());
        for target in targets {
            let path = format!(
                "/v1/sites/{}/devices/{}/statistics/latest",
                safe_path_id(&target.site_id)?,
                safe_path_id(&target.device_id)?
            );
            let plan = get_plan(&plans.endpoint, &plans.api_key_ref, &path, plans.timeout_ms)?;
            let response = self.get_sensitive_json(&plan, api_key, "latest device statistics")?;
            statistics.push(parse_device_statistics(&response.0, target)?);
        }
        Ok(statistics)
    }
}

pub struct UniFiClient<T> {
    config: UniFiConfig,
    api_key: UniFiApiKey,
    transport: T,
    plans: UniFiRequestPlans,
    presence_key: Option<UniFiPresenceKey>,
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
            presence_key: None,
        })
    }

    pub fn with_presence_key(mut self, presence_key: UniFiPresenceKey) -> Self {
        self.presence_key = Some(presence_key);
        self
    }

    pub fn inspect(&mut self) -> Result<UniFiSnapshot, UniFiError> {
        self.transport.inspect(&self.plans, &self.api_key)
    }

    pub fn inspect_with_connected_clients(&mut self) -> Result<UniFiSnapshot, UniFiError> {
        let presence_key = self.presence_key.as_ref().ok_or_else(|| {
            UniFiError::Validation(
                "connected-client inspection requires a Vault-leased presence key".to_string(),
            )
        })?;
        let mut snapshot = self.transport.inspect(&self.plans, &self.api_key)?;
        snapshot.connected_clients = self.transport.inspect_connected_clients(
            &self.plans,
            &self.api_key,
            presence_key,
            &snapshot.sites,
        )?;
        Ok(snapshot)
    }

    pub fn inspect_device_statistics(
        &mut self,
        targets: &[UniFiStatisticsTarget],
    ) -> Result<Vec<UniFiDeviceStatistics>, UniFiError> {
        validate_statistics_targets(targets)?;
        self.transport
            .inspect_device_statistics(&self.plans, &self.api_key, targets)
    }
}

impl<T> fmt::Debug for UniFiClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UniFiClient")
            .field("config", &self.config)
            .field("api_key", &"[REDACTED]")
            .field(
                "presence_key",
                &self.presence_key.as_ref().map(|_| "[REDACTED]"),
            )
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
    pub connected_client_entity_ids: Vec<EntityId>,
}

pub struct UniFiRuntimeIntegration<T> {
    client: UniFiClient<T>,
    data_governance: DataGovernancePolicy,
    last_statistics_poll_at_ms: Option<u64>,
}

impl<T: UniFiTransport> UniFiRuntimeIntegration<T> {
    pub fn new(client: UniFiClient<T>) -> Self {
        Self {
            client,
            data_governance: DataGovernancePolicy::default(),
            last_statistics_poll_at_ms: None,
        }
    }

    pub fn with_data_governance(mut self, data_governance: DataGovernancePolicy) -> Self {
        self.data_governance = data_governance;
        self
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

    pub fn inspect_clients_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledUniFiNetwork, UniFiError> {
        authorize_read(runtime, principal_id.clone(), observed_at_ms)?;
        authorize_client_inspection(
            &self.data_governance,
            &principal_id,
            &self.client.config,
            observed_at_ms,
        )?;
        let snapshot = self.client.inspect_with_connected_clients()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }

    pub fn inspect_statistics_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        targets: &[UniFiStatisticsTarget],
        observed_at_ms: u64,
    ) -> Result<Vec<EntityId>, UniFiError> {
        validate_statistics_targets(targets)?;
        validate_statistics_targets_installed(runtime, &self.client.config, targets)?;
        let retry_at_ms = self
            .last_statistics_poll_at_ms
            .map(|last| last.saturating_add(STATISTICS_MIN_POLL_INTERVAL_MS));
        if retry_at_ms.is_some_and(|retry_at| observed_at_ms < retry_at) {
            return Err(UniFiError::PollRateLimited {
                retry_at_ms: retry_at_ms.unwrap_or(observed_at_ms),
            });
        }
        authorize_read(runtime, principal_id.clone(), observed_at_ms)?;
        authorize_statistics_inspection(
            &self.data_governance,
            &principal_id,
            &self.client.config,
            observed_at_ms,
        )?;
        let statistics = self.client.inspect_device_statistics(targets)?;
        validate_statistics_response(targets, &statistics)?;
        let installed =
            install_device_statistics(runtime, &self.client.config, &statistics, observed_at_ms)?;
        self.last_statistics_poll_at_ms = Some(observed_at_ms);
        Ok(installed)
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
    validate_client_snapshot(&snapshot.connected_clients)?;
    let presence_expires_at_ms = if snapshot.connected_clients.is_empty() {
        None
    } else {
        Some(
            observed_at_ms
                .checked_add(CLIENT_PRESENCE_RETENTION_MS)
                .ok_or_else(|| {
                    UniFiError::Validation("client presence expiry overflows time".to_string())
                })?,
        )
    };
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
        Metadata::new(
            "unifi.pseudonymous_client_count",
            snapshot.connected_clients.len().to_string(),
        ),
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
    let mut connected_client_entity_ids = Vec::with_capacity(snapshot.connected_clients.len());
    for client in &snapshot.connected_clients {
        let device_id = DeviceId::trusted(format!("unifi:client:{}", client.pseudonym));
        let entity_id = EntityId::trusted(format!("unifi:client:{}:presence", client.pseudonym));
        runtime.upsert_device(Device {
            device_id: device_id.clone(),
            bridge_id: config.bridge_id.clone(),
            manufacturer: "Unknown".to_string(),
            model: client.client_type.clone(),
            name: format!("UniFi connected client {}", &client.pseudonym[..8]),
            serial: None,
            firmware_version: None,
            room_id: None,
            entity_ids: vec![entity_id.clone()],
            identifiers: vec![protocol_identifier("client_pseudonym", &client.pseudonym)?],
            health: Health::Online,
            metadata: vec![Metadata::new("unifi.identifier_form", "keyed_pseudonym")],
        })?;
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id,
            kind: EntityKind::NetworkDiagnostic,
            name: format!("UniFi client {} presence", &client.pseudonym[..8]),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("network.client_presence"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: connected_client_value(client),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: presence_expires_at_ms,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![
                Metadata::new("unifi.protocol", PROTOCOL_ID),
                Metadata::new("unifi.identifier_form", "keyed_pseudonym"),
            ],
        })?;
        connected_client_entity_ids.push(entity_id);
    }
    Ok(InstalledUniFiNetwork {
        bridge_id: config.bridge_id.clone(),
        devices: installed,
        connected_client_entity_ids,
    })
}

pub fn install_device_statistics(
    runtime: &mut SmartHomeRuntime,
    config: &UniFiConfig,
    statistics: &[UniFiDeviceStatistics],
    observed_at_ms: u64,
) -> Result<Vec<EntityId>, UniFiError> {
    validate_statistics_batch(statistics)?;
    let expires_at_ms = observed_at_ms
        .checked_add(STATISTICS_RETENTION_MS)
        .ok_or_else(|| UniFiError::Validation("statistics expiry overflows time".to_string()))?;
    let mut prepared = Vec::with_capacity(statistics.len());
    for reading in statistics {
        let device_id = normalized_device_id(&reading.target)?;
        let mut device = runtime
            .registry()
            .device(&device_id)
            .filter(|device| device.bridge_id == config.bridge_id)
            .cloned()
            .ok_or_else(|| {
                UniFiError::Validation(format!(
                    "statistics target {} is not an installed UniFi device",
                    device_id.as_str()
                ))
            })?;
        let entity_id = EntityId::trusted(format!("{}:statistics", device_id.as_str()));
        if !device.entity_ids.contains(&entity_id) {
            device.entity_ids.push(entity_id.clone());
        }
        let entity = Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::NetworkDiagnostic,
            name: format!("{} live statistics", device.name),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("network.device_statistics"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: device_statistics_value(reading),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: Some(expires_at_ms),
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![
                Metadata::new("unifi.protocol", PROTOCOL_ID),
                Metadata::new("unifi.statistics_retention", "bounded_two_minutes"),
            ],
        };
        prepared.push((device, entity));
    }
    let mut installed = Vec::with_capacity(prepared.len());
    for (device, entity) in prepared {
        installed.push(entity.entity_id.clone());
        runtime.upsert_device(device)?;
        runtime.upsert_entity(entity)?;
    }
    Ok(installed)
}

fn normalized_device_id(target: &UniFiStatisticsTarget) -> Result<DeviceId, UniFiError> {
    let site_id = stable_component(&target.site_id);
    let device_id = stable_component(&target.device_id);
    if site_id.is_empty() || device_id.is_empty() {
        return Err(UniFiError::Validation(
            "statistics target IDs must have stable components".to_string(),
        ));
    }
    Ok(DeviceId::trusted(format!("unifi:{site_id}:{device_id}")))
}

fn validate_statistics_targets_installed(
    runtime: &SmartHomeRuntime,
    config: &UniFiConfig,
    targets: &[UniFiStatisticsTarget],
) -> Result<(), UniFiError> {
    for target in targets {
        let device_id = normalized_device_id(target)?;
        if !runtime
            .registry()
            .device(&device_id)
            .is_some_and(|device| device.bridge_id == config.bridge_id)
        {
            return Err(UniFiError::Validation(format!(
                "statistics target {} is not an installed UniFi device",
                device_id.as_str()
            )));
        }
    }
    Ok(())
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

fn authorize_client_inspection(
    policy: &DataGovernancePolicy,
    principal_id: &AgentId,
    config: &UniFiConfig,
    now_ms: u64,
) -> Result<(), UniFiError> {
    let resource_id = format!(
        "unifi:{}:connected-clients",
        stable_component(config.bridge_id.as_str())
    );
    for (category, retention) in [
        (DataCategory::DeviceIdentifier, DataRetention::Ephemeral),
        (
            DataCategory::Presence,
            DataRetention::Bounded {
                maximum_age_ms: CLIENT_PRESENCE_RETENTION_MS,
            },
        ),
    ] {
        match policy.decide(&DataUseRequest {
            principal_id,
            resource_id: &resource_id,
            category,
            operation: DataOperation::Inspect,
            destination: DataDestination::LocalDevice,
            retention,
            now_ms,
        }) {
            DataGovernanceDecision::Allow(_) => {}
            DataGovernanceDecision::Deny(reason) => {
                return Err(UniFiError::DataGovernanceDenied(reason))
            }
        }
    }
    Ok(())
}

fn authorize_statistics_inspection(
    policy: &DataGovernancePolicy,
    principal_id: &AgentId,
    config: &UniFiConfig,
    now_ms: u64,
) -> Result<(), UniFiError> {
    let resource_id = format!(
        "unifi:{}:device-statistics",
        stable_component(config.bridge_id.as_str())
    );
    match policy.decide(&DataUseRequest {
        principal_id,
        resource_id: &resource_id,
        category: DataCategory::OperationalTelemetry,
        operation: DataOperation::Inspect,
        destination: DataDestination::LocalDevice,
        retention: DataRetention::Bounded {
            maximum_age_ms: STATISTICS_RETENTION_MS,
        },
        now_ms,
    }) {
        DataGovernanceDecision::Allow(_) => Ok(()),
        DataGovernanceDecision::Deny(reason) => Err(UniFiError::DataGovernanceDenied(reason)),
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

struct SensitiveJson(JsonValue);

impl Drop for SensitiveJson {
    fn drop(&mut self) {
        zeroize_json_strings(&mut self.0);
    }
}

fn zeroize_json_strings(value: &mut JsonValue) {
    match value {
        JsonValue::String(text) => text.zeroize(),
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

fn parse_client_page(
    value: &JsonValue,
    expected_offset: usize,
    site: &UniFiSite,
    presence_key: &UniFiPresenceKey,
) -> Result<Page<UniFiConnectedClient>, UniFiError> {
    let object = value
        .as_object()
        .ok_or(UniFiError::MissingField("client page object"))?;
    let (offset, _count, total_count, data) = page_fields(object, expected_offset)?;
    let mut clients = Vec::with_capacity(data.len());
    for item in data {
        let object = item
            .as_object()
            .ok_or(UniFiError::MissingField("client data item"))?;
        let native_id = Zeroizing::new(required_text(object, "id")?);
        safe_path_id(native_id.as_str())?;
        let (access_type, access_authorized) = parse_client_access(object.get("access"))?;
        clients.push(UniFiConnectedClient {
            pseudonym: connected_client_pseudonym(presence_key, &site.id, native_id.as_str()),
            client_type: required_text(object, "type")?,
            access_type,
            access_authorized,
        });
    }
    Ok(Page {
        offset,
        total_count,
        data: clients,
    })
}

fn parse_client_access(
    value: Option<&JsonValue>,
) -> Result<(Option<String>, Option<bool>), UniFiError> {
    let Some(value) = value else {
        return Ok((None, None));
    };
    if value.is_null() {
        return Ok((None, None));
    }
    let object = value.as_object().ok_or_else(|| {
        UniFiError::Validation("client access must be an object or null".to_string())
    })?;
    let access_type = optional_text(object, "type")?;
    let authorized = match object.get("authorized") {
        None | Some(JsonValue::Null) => None,
        Some(value) => Some(value.as_bool().ok_or_else(|| {
            UniFiError::Validation("client access authorization must be boolean".to_string())
        })?),
    };
    Ok((access_type, authorized))
}

fn parse_device_statistics(
    value: &JsonValue,
    target: &UniFiStatisticsTarget,
) -> Result<UniFiDeviceStatistics, UniFiError> {
    let object = value
        .as_object()
        .ok_or(UniFiError::MissingField("device statistics object"))?;
    let uplink = optional_object(object, "uplink")?;
    let interfaces = object
        .get("interfaces")
        .and_then(JsonValue::as_object)
        .ok_or(UniFiError::MissingField("device statistics interfaces"))?;
    let radios = match interfaces.get("radios") {
        None | Some(JsonValue::Null) => Vec::new(),
        Some(JsonValue::Array(values)) => {
            if values.len() > MAX_STATISTICS_RADIOS {
                return Err(UniFiError::Validation(format!(
                    "device statistics exceed {MAX_STATISTICS_RADIOS} radios"
                )));
            }
            values
                .iter()
                .map(|value| {
                    let radio = value
                        .as_object()
                        .ok_or(UniFiError::MissingField("device statistics radio"))?;
                    Ok(UniFiRadioStatistics {
                        frequency_ghz: required_number_or_text(radio, "frequencyGHz", 0.1, 100.0)?,
                        tx_retries_pct: required_number(radio, "txRetriesPct", 0.0, 100.0)?,
                    })
                })
                .collect::<Result<Vec<_>, UniFiError>>()?
        }
        Some(_) => {
            return Err(UniFiError::Validation(
                "device statistics radios must be an array".to_string(),
            ))
        }
    };
    Ok(UniFiDeviceStatistics {
        target: target.clone(),
        uptime_sec: required_u64(object, "uptimeSec")?,
        load_average_1_min: required_number(object, "loadAverage1Min", 0.0, 1_000_000.0)?,
        load_average_5_min: required_number(object, "loadAverage5Min", 0.0, 1_000_000.0)?,
        load_average_15_min: required_number(object, "loadAverage15Min", 0.0, 1_000_000.0)?,
        cpu_utilization_pct: required_number(object, "cpuUtilizationPct", 0.0, 100.0)?,
        memory_utilization_pct: required_number(object, "memoryUtilizationPct", 0.0, 100.0)?,
        uplink_tx_rate_bps: optional_u64(uplink, "txRateBps")?,
        uplink_rx_rate_bps: optional_u64(uplink, "rxRateBps")?,
        radios,
    })
}

fn optional_object<'a>(
    object: &'a JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Option<&'a JsonMap<String, JsonValue>>, UniFiError> {
    match object.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(JsonValue::Object(value)) => Ok(Some(value)),
        Some(_) => Err(UniFiError::Validation(format!(
            "device statistics {field} must be an object or null"
        ))),
    }
}

fn required_u64(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<u64, UniFiError> {
    object
        .get(field)
        .and_then(JsonValue::as_u64)
        .filter(|value| *value <= i64::MAX as u64)
        .ok_or(UniFiError::MissingField(field))
}

fn optional_u64(
    object: Option<&JsonMap<String, JsonValue>>,
    field: &'static str,
) -> Result<Option<u64>, UniFiError> {
    let Some(object) = object else {
        return Ok(None);
    };
    match object.get(field) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .filter(|number| *number <= i64::MAX as u64)
            .map(Some)
            .ok_or_else(|| {
                UniFiError::Validation(format!(
                    "device statistics {field} must be a non-negative 64-bit integer"
                ))
            }),
    }
}

fn required_number(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
    minimum: f64,
    maximum: f64,
) -> Result<f64, UniFiError> {
    let value = object
        .get(field)
        .and_then(JsonValue::as_f64)
        .ok_or(UniFiError::MissingField(field))?;
    validate_number(field, value, minimum, maximum)
}

fn required_number_or_text(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
    minimum: f64,
    maximum: f64,
) -> Result<f64, UniFiError> {
    let value = object.get(field).ok_or(UniFiError::MissingField(field))?;
    let number = match value {
        JsonValue::Number(_) => value.as_f64(),
        JsonValue::String(text) => text.parse::<f64>().ok(),
        _ => None,
    }
    .ok_or(UniFiError::MissingField(field))?;
    validate_number(field, number, minimum, maximum)
}

fn validate_number(
    field: &'static str,
    value: f64,
    minimum: f64,
    maximum: f64,
) -> Result<f64, UniFiError> {
    if value.is_finite() && value >= minimum && value <= maximum {
        Ok(value)
    } else {
        Err(UniFiError::Validation(format!(
            "device statistics {field} is outside the supported range"
        )))
    }
}

fn connected_client_pseudonym(
    presence_key: &UniFiPresenceKey,
    site_id: &str,
    native_id: &str,
) -> String {
    let mut input = Zeroizing::new(Vec::with_capacity(
        presence_key.bytes.len() + site_id.len() + native_id.len() + 24,
    ));
    input.extend_from_slice(presence_key.bytes.as_slice());
    input.extend_from_slice(b"unifi-client-v1\0");
    input.extend_from_slice(site_id.as_bytes());
    input.push(0);
    input.extend_from_slice(native_id.as_bytes());
    let digest = Zeroizing::new(sha256(input.as_slice()));
    lowercase_hex(&digest[..16])
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
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

fn validate_client_uniqueness(clients: &[UniFiConnectedClient]) -> Result<(), UniFiError> {
    let mut pseudonyms = BTreeSet::new();
    for client in clients {
        if !pseudonyms.insert(client.pseudonym.as_str()) {
            return Err(UniFiError::Validation(
                "connected-client identities must be unique".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_client_snapshot(clients: &[UniFiConnectedClient]) -> Result<(), UniFiError> {
    if clients.len() > MAX_CONNECTED_CLIENTS {
        return Err(UniFiError::Validation(format!(
            "connected clients exceed {MAX_CONNECTED_CLIENTS} entries"
        )));
    }
    for client in clients {
        if client.pseudonym.len() != 32
            || !client
                .pseudonym
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(UniFiError::Validation(
                "connected-client pseudonym must be 128-bit lowercase hexadecimal text".to_string(),
            ));
        }
        validate_text("client type", &client.client_type)?;
        if let Some(access_type) = &client.access_type {
            validate_text("client access type", access_type)?;
        }
    }
    validate_client_uniqueness(clients)
}

fn validate_statistics_targets(targets: &[UniFiStatisticsTarget]) -> Result<(), UniFiError> {
    if targets.is_empty() || targets.len() > MAX_STATISTICS_TARGETS_PER_POLL {
        return Err(UniFiError::Validation(format!(
            "statistics poll must contain 1 to {MAX_STATISTICS_TARGETS_PER_POLL} targets"
        )));
    }
    let mut unique = BTreeSet::new();
    for target in targets {
        safe_path_id(&target.site_id)?;
        safe_path_id(&target.device_id)?;
        if !unique.insert((target.site_id.as_str(), target.device_id.as_str())) {
            return Err(UniFiError::Validation(
                "statistics targets must be unique".to_string(),
            ));
        }
    }
    Ok(())
}

fn validate_statistics_batch(statistics: &[UniFiDeviceStatistics]) -> Result<(), UniFiError> {
    let targets = statistics
        .iter()
        .map(|reading| reading.target.clone())
        .collect::<Vec<_>>();
    validate_statistics_targets(&targets)?;
    for reading in statistics {
        if reading.uptime_sec > i64::MAX as u64
            || reading
                .uplink_tx_rate_bps
                .is_some_and(|value| value > i64::MAX as u64)
            || reading
                .uplink_rx_rate_bps
                .is_some_and(|value| value > i64::MAX as u64)
            || reading.radios.len() > MAX_STATISTICS_RADIOS
        {
            return Err(UniFiError::Validation(
                "device statistics exceed normalized state bounds".to_string(),
            ));
        }
        validate_number(
            "loadAverage1Min",
            reading.load_average_1_min,
            0.0,
            1_000_000.0,
        )?;
        validate_number(
            "loadAverage5Min",
            reading.load_average_5_min,
            0.0,
            1_000_000.0,
        )?;
        validate_number(
            "loadAverage15Min",
            reading.load_average_15_min,
            0.0,
            1_000_000.0,
        )?;
        validate_number("cpuUtilizationPct", reading.cpu_utilization_pct, 0.0, 100.0)?;
        validate_number(
            "memoryUtilizationPct",
            reading.memory_utilization_pct,
            0.0,
            100.0,
        )?;
        for radio in &reading.radios {
            validate_number("frequencyGHz", radio.frequency_ghz, 0.1, 100.0)?;
            validate_number("txRetriesPct", radio.tx_retries_pct, 0.0, 100.0)?;
        }
    }
    Ok(())
}

fn validate_statistics_response(
    targets: &[UniFiStatisticsTarget],
    statistics: &[UniFiDeviceStatistics],
) -> Result<(), UniFiError> {
    validate_statistics_batch(statistics)?;
    if targets.len() != statistics.len()
        || targets
            .iter()
            .zip(statistics)
            .any(|(target, reading)| target != &reading.target)
    {
        return Err(UniFiError::Validation(
            "statistics response does not match the requested targets".to_string(),
        ));
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

fn connected_client_value(client: &UniFiConnectedClient) -> Value {
    let mut fields = vec![
        ("present".to_string(), Value::Bool(true)),
        (
            "connection_type".to_string(),
            Value::Text(client.client_type.clone()),
        ),
    ];
    if let Some(access_type) = &client.access_type {
        fields.push(("access_type".to_string(), Value::Text(access_type.clone())));
    }
    if let Some(authorized) = client.access_authorized {
        fields.push(("access_authorized".to_string(), Value::Bool(authorized)));
    }
    Value::Object(fields)
}

fn device_statistics_value(statistics: &UniFiDeviceStatistics) -> Value {
    let mut fields = vec![
        (
            "uptime_sec".to_string(),
            Value::Integer(statistics.uptime_sec as i64),
        ),
        (
            "load_average_1_min".to_string(),
            Value::Number(statistics.load_average_1_min),
        ),
        (
            "load_average_5_min".to_string(),
            Value::Number(statistics.load_average_5_min),
        ),
        (
            "load_average_15_min".to_string(),
            Value::Number(statistics.load_average_15_min),
        ),
        (
            "cpu_utilization_pct".to_string(),
            Value::Number(statistics.cpu_utilization_pct),
        ),
        (
            "memory_utilization_pct".to_string(),
            Value::Number(statistics.memory_utilization_pct),
        ),
    ];
    if let Some(value) = statistics.uplink_tx_rate_bps {
        fields.push((
            "uplink_tx_rate_bps".to_string(),
            Value::Integer(value as i64),
        ));
    }
    if let Some(value) = statistics.uplink_rx_rate_bps {
        fields.push((
            "uplink_rx_rate_bps".to_string(),
            Value::Integer(value as i64),
        ));
    }
    fields.push((
        "radios".to_string(),
        Value::Array(
            statistics
                .radios
                .iter()
                .map(|radio| {
                    Value::Object(vec![
                        (
                            "frequency_ghz".to_string(),
                            Value::Number(radio.frequency_ghz),
                        ),
                        (
                            "tx_retries_pct".to_string(),
                            Value::Number(radio.tx_retries_pct),
                        ),
                    ])
                })
                .collect(),
        ),
    ));
    Value::Object(fields)
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
    use smart_home_data_governance::{ConsentReceiptRef, DataPurpose, DataUseGrant};
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;

    const PRESENCE_KEY: [u8; 32] = [0x6c; 32];
    const CLIENT_ID: &str = "f27a1d16-6f5d-4bd8-a12c-c9ed87f17c14";
    const CLIENT_NAME: &str = "Private Phone";
    const CLIENT_MAC: &str = "aa:bb:cc:dd:ee:ff";
    const CLIENT_IP: &str = "192.0.2.99";

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
            connected_clients: Vec::new(),
        }
    }

    fn connected_client_page() -> JsonValue {
        serde_json::json!({
            "offset": 0,
            "limit": 100,
            "count": 1,
            "totalCount": 1,
            "data": [{
                "type": "WIRELESS",
                "id": CLIENT_ID,
                "name": CLIENT_NAME,
                "connectedAt": "2026-08-09T09:00:00Z",
                "macAddress": CLIENT_MAC,
                "ipAddress": CLIENT_IP,
                "access": {"type": "STANDARD", "authorized": true}
            }]
        })
    }

    fn statistics_target() -> UniFiStatisticsTarget {
        UniFiStatisticsTarget::new("site-1", "device-ap").unwrap()
    }

    fn statistics_reading() -> UniFiDeviceStatistics {
        UniFiDeviceStatistics {
            target: statistics_target(),
            uptime_sec: 86_400,
            load_average_1_min: 0.5,
            load_average_5_min: 0.4,
            load_average_15_min: 0.3,
            cpu_utilization_pct: 23.5,
            memory_utilization_pct: 61.25,
            uplink_tx_rate_bps: Some(1_024),
            uplink_rx_rate_bps: Some(2_048),
            radios: vec![UniFiRadioStatistics {
                frequency_ghz: 5.0,
                tx_retries_pct: 1.5,
            }],
        }
    }

    fn statistics_json() -> JsonValue {
        serde_json::json!({
            "uptimeSec": 86400,
            "lastHeartbeatAt": "2026-08-09T10:00:00Z",
            "nextHeartbeatAt": "2026-08-09T10:01:00Z",
            "loadAverage1Min": 0.5,
            "loadAverage5Min": 0.4,
            "loadAverage15Min": 0.3,
            "cpuUtilizationPct": 23.5,
            "memoryUtilizationPct": 61.25,
            "uplink": {"txRateBps": 1024, "rxRateBps": 2048},
            "interfaces": {
                "radios": [{"frequencyGHz": "5", "txRetriesPct": 1.5}]
            }
        })
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

    #[derive(Debug)]
    struct StatisticsTransport {
        readings: Vec<UniFiDeviceStatistics>,
        calls: Arc<AtomicUsize>,
    }

    impl UniFiTransport for StatisticsTransport {
        fn inspect(
            &mut self,
            _plans: &UniFiRequestPlans,
            _api_key: &UniFiApiKey,
        ) -> Result<UniFiSnapshot, UniFiError> {
            panic!("statistics tests must not run aggregate inspection")
        }

        fn inspect_device_statistics(
            &mut self,
            _plans: &UniFiRequestPlans,
            _api_key: &UniFiApiKey,
            _targets: &[UniFiStatisticsTarget],
        ) -> Result<Vec<UniFiDeviceStatistics>, UniFiError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.readings.clone())
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

    fn client_policy(principal: &AgentId) -> DataGovernancePolicy {
        let mut policy = DataGovernancePolicy::default();
        for (category, retention, purpose, receipt) in [
            (
                DataCategory::DeviceIdentifier,
                DataRetention::Ephemeral,
                "derive private connected-client identities",
                "consent://unifi/client-identifiers-1",
            ),
            (
                DataCategory::Presence,
                DataRetention::Bounded {
                    maximum_age_ms: CLIENT_PRESENCE_RETENTION_MS,
                },
                "show current home-network presence",
                "consent://unifi/client-presence-1",
            ),
        ] {
            policy
                .add_grant(
                    DataUseGrant::new(
                        principal.clone(),
                        "unifi:unifi-test:connected-clients",
                        category,
                        DataOperation::Inspect,
                        DataDestination::LocalDevice,
                        DataPurpose::new(purpose).unwrap(),
                        ConsentReceiptRef::new(receipt).unwrap(),
                        retention,
                        1_000,
                        20_000,
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        policy
    }

    fn statistics_policy(principal: &AgentId) -> DataGovernancePolicy {
        let mut policy = DataGovernancePolicy::default();
        policy
            .add_grant(
                DataUseGrant::new(
                    principal.clone(),
                    "unifi:unifi-test:device-statistics",
                    DataCategory::OperationalTelemetry,
                    DataOperation::Inspect,
                    DataDestination::LocalDevice,
                    DataPurpose::new("inspect short-lived network device health metrics").unwrap(),
                    ConsentReceiptRef::new("consent://unifi/device-statistics-1").unwrap(),
                    DataRetention::Bounded {
                        maximum_age_ms: STATISTICS_RETENTION_MS,
                    },
                    1_000,
                    200_000,
                )
                .unwrap(),
            )
            .unwrap();
        policy
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
    fn presence_keys_are_strict_and_redacted() {
        assert!(UniFiPresenceKey::new(vec![0x6c; 31]).is_err());
        let key = UniFiPresenceKey::new(PRESENCE_KEY.to_vec()).unwrap();
        assert_eq!(format!("{key:?}"), "UniFiPresenceKey([REDACTED])");
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
    fn connected_client_inspection_without_data_consent_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = UniFiClient::new(
            config(443),
            api_key(),
            FixedTransport {
                snapshot: snapshot(),
                calls: Arc::clone(&calls),
            },
        )
        .unwrap()
        .with_presence_key(UniFiPresenceKey::new(PRESENCE_KEY.to_vec()).unwrap());
        let mut integration = UniFiRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:presence-denied");
        authorize(&mut runtime, principal.clone());
        assert!(matches!(
            integration.inspect_clients_and_install_authorized(&mut runtime, principal, 2_000),
            Err(UniFiError::DataGovernanceDenied(
                DataGovernanceDenial::NoMatchingConsent
            ))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn statistics_without_exact_data_consent_reaches_no_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = UniFiClient::new(
            config(443),
            api_key(),
            StatisticsTransport {
                readings: vec![statistics_reading()],
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut integration = UniFiRuntimeIntegration::new(client);
        let mut runtime = SmartHomeRuntime::new();
        install_snapshot(&mut runtime, &config(443), &snapshot(), 1_000).unwrap();
        let principal = AgentId::trusted("agent:statistics-denied");
        authorize(&mut runtime, principal.clone());
        assert!(matches!(
            integration.inspect_statistics_and_install_authorized(
                &mut runtime,
                principal,
                &[statistics_target()],
                2_000,
            ),
            Err(UniFiError::DataGovernanceDenied(
                DataGovernanceDenial::NoMatchingConsent
            ))
        ));
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
    fn statistics_parser_is_bounded_and_ignores_heartbeat_timestamps() {
        let reading = parse_device_statistics(&statistics_json(), &statistics_target()).unwrap();
        assert_eq!(reading, statistics_reading());
        assert!(validate_statistics_response(
            &[statistics_target()],
            std::slice::from_ref(&reading)
        )
        .is_ok());
        let mut wrong_target = reading.clone();
        wrong_target.target = UniFiStatisticsTarget::new("site-1", "device-switch").unwrap();
        assert!(validate_statistics_response(&[statistics_target()], &[wrong_target]).is_err());
        let debug = format!("{reading:?}");
        assert!(!debug.contains("2026-08-09"));

        let mut invalid_percentage = statistics_json();
        invalid_percentage["cpuUtilizationPct"] = serde_json::json!(100.1);
        assert!(parse_device_statistics(&invalid_percentage, &statistics_target()).is_err());

        let mut too_many_radios = statistics_json();
        too_many_radios["interfaces"]["radios"] = JsonValue::Array(
            (0..=MAX_STATISTICS_RADIOS)
                .map(|_| serde_json::json!({"frequencyGHz": 5, "txRetriesPct": 1}))
                .collect(),
        );
        assert!(parse_device_statistics(&too_many_radios, &statistics_target()).is_err());
    }

    #[test]
    fn governed_statistics_install_expires_and_rate_limits_before_io() {
        let calls = Arc::new(AtomicUsize::new(0));
        let client = UniFiClient::new(
            config(443),
            api_key(),
            StatisticsTransport {
                readings: vec![statistics_reading()],
                calls: Arc::clone(&calls),
            },
        )
        .unwrap();
        let mut runtime = SmartHomeRuntime::new();
        install_snapshot(&mut runtime, &config(443), &snapshot(), 1_000).unwrap();
        let principal = AgentId::trusted("agent:statistics-allowed");
        authorize(&mut runtime, principal.clone());
        let mut integration = UniFiRuntimeIntegration::new(client)
            .with_data_governance(statistics_policy(&principal));

        let installed = integration
            .inspect_statistics_and_install_authorized(
                &mut runtime,
                principal.clone(),
                &[statistics_target()],
                5_000,
            )
            .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(installed.len(), 1);
        let entity = runtime.registry().entity(&installed[0]).unwrap();
        assert_eq!(
            entity.state.as_ref().unwrap().expires_at_ms,
            Some(5_000 + STATISTICS_RETENTION_MS)
        );
        assert!(format!("{entity:?}").contains("cpu_utilization_pct"));

        assert!(matches!(
            integration.inspect_statistics_and_install_authorized(
                &mut runtime,
                principal,
                &[statistics_target()],
                5_000 + STATISTICS_MIN_POLL_INTERVAL_MS - 1,
            ),
            Err(UniFiError::PollRateLimited { .. })
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn client_parser_keeps_only_pseudonymous_presence_fields() {
        let site = UniFiSite {
            id: "site-1".to_string(),
            name: "Home".to_string(),
            internal_reference: None,
        };
        let key = UniFiPresenceKey::new(PRESENCE_KEY.to_vec()).unwrap();
        let sensitive = SensitiveJson(connected_client_page());
        let page = parse_client_page(&sensitive.0, 0, &site, &key).unwrap();
        assert_eq!(page.data.len(), 1);
        let client = &page.data[0];
        assert_eq!(client.pseudonym.len(), 32);
        assert_eq!(client.client_type, "WIRELESS");
        assert_eq!(client.access_type.as_deref(), Some("STANDARD"));
        assert_eq!(client.access_authorized, Some(true));
        let debug = format!("{client:?}");
        for raw in [CLIENT_ID, CLIENT_NAME, CLIENT_MAC, CLIENT_IP] {
            assert!(!debug.contains(raw));
        }

        let duplicate = Page {
            offset: 0,
            total_count: 2,
            data: vec![client.clone(), client.clone()],
        };
        assert!(validate_client_uniqueness(&duplicate.data).is_err());
    }

    #[test]
    fn install_rejects_bad_client_pseudonyms_before_runtime_mutation() {
        let mut snapshot = snapshot();
        snapshot.connected_clients.push(UniFiConnectedClient {
            pseudonym: "short".to_string(),
            client_type: "WIRELESS".to_string(),
            access_type: None,
            access_authorized: None,
        });
        let mut runtime = SmartHomeRuntime::new();
        assert!(
            install_snapshot(&mut runtime, &config(443), &snapshot, 2_000)
                .unwrap_err()
                .to_string()
                .contains("pseudonym")
        );
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("unifi.test"))
            .is_none());
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
    fn loopback_statistics_transport_uses_exact_single_device_path() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let request_head = Arc::new(Mutex::new(String::new()));
        let server_head = Arc::clone(&request_head);
        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream);
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" {
                    break;
                }
                server_head.lock().unwrap().push_str(&line);
            }
            let body = serde_json::to_vec(&statistics_json()).unwrap();
            let response_head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            reader
                .get_mut()
                .write_all(response_head.as_bytes())
                .unwrap();
            reader.get_mut().write_all(&body).unwrap();
        });

        let mut client =
            UniFiClient::new(config(port), api_key(), UniFiLanTransport::default()).unwrap();
        let readings = client
            .inspect_device_statistics(&[statistics_target()])
            .unwrap();
        handle.join().unwrap();
        assert_eq!(readings, vec![statistics_reading()]);
        let request_head = request_head.lock().unwrap();
        assert!(request_head.starts_with(
            "GET /proxy/network/integration/v1/sites/site-1/devices/device-ap/statistics/latest HTTP/1.1"
        ));
        assert!(request_head.contains("X-API-Key: secret-api-key"));
        assert!(!request_head.contains("vault://unifi/test"));
    }

    #[test]
    fn governed_loopback_client_presence_excludes_native_identity_and_expires() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let server_requests = Arc::clone(&requests);
        let responses = vec![
            serde_json::json!({"applicationVersion": "10.4.57"}),
            serde_json::json!({
                "offset": 0,
                "limit": 100,
                "count": 1,
                "totalCount": 1,
                "data": [{"id": "site-1", "internalReference": "default", "name": "Home"}]
            }),
            serde_json::json!({
                "offset": 0,
                "limit": 100,
                "count": 1,
                "totalCount": 1,
                "data": [{
                    "id": "device-ap",
                    "name": "Upstairs AP",
                    "model": "U7 Pro",
                    "macAddress": "00:11:22:33:44:55",
                    "ipAddress": "192.0.2.10",
                    "state": "ONLINE",
                    "features": ["accessPoint"]
                }]
            }),
            connected_client_page(),
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
                server_requests.lock().unwrap().push(head);
                let body = serde_json::to_vec(&response).unwrap();
                let response_head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                reader
                    .get_mut()
                    .write_all(response_head.as_bytes())
                    .unwrap();
                reader.get_mut().write_all(&body).unwrap();
            }
        });

        let client = UniFiClient::new(config(port), api_key(), UniFiLanTransport::default())
            .unwrap()
            .with_presence_key(UniFiPresenceKey::new(PRESENCE_KEY.to_vec()).unwrap());
        let mut runtime = SmartHomeRuntime::new();
        let principal = AgentId::trusted("agent:presence-allowed");
        authorize(&mut runtime, principal.clone());
        let mut integration =
            UniFiRuntimeIntegration::new(client).with_data_governance(client_policy(&principal));
        let installed = integration
            .inspect_clients_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        handle.join().unwrap();

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[3].starts_with(
            "GET /proxy/network/integration/v1/sites/site-1/clients?offset=0&limit=100 HTTP/1.1"
        ));
        assert!(requests
            .iter()
            .all(|head| head.contains("X-API-Key: secret-api-key")));
        for raw in [CLIENT_ID, CLIENT_NAME, CLIENT_MAC, CLIENT_IP] {
            assert!(requests.iter().all(|head| !head.contains(raw)));
        }

        assert_eq!(installed.connected_client_entity_ids.len(), 1);
        let entity = runtime
            .registry()
            .entity(&installed.connected_client_entity_ids[0])
            .unwrap();
        assert_eq!(
            entity.state.as_ref().unwrap().expires_at_ms,
            Some(5_000 + CLIENT_PRESENCE_RETENTION_MS)
        );
        let debug = format!("{entity:?}");
        assert!(debug.contains("keyed_pseudonym"));
        assert!(debug.contains("WIRELESS"));
        for raw in [CLIENT_ID, CLIENT_NAME, CLIENT_MAC, CLIENT_IP] {
            assert!(!debug.contains(raw));
        }
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
