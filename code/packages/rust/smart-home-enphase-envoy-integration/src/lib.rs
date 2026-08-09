//! Authenticated local Enphase IQ Gateway meter telemetry for D23.

#![forbid(unsafe_code)]

use coding_adventures_sha256::sha256;
use coding_adventures_vault_leases::{LeaseError, LeaseId, LeaseManager};
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
use smart_home_runtime::{
    RetainedDeviceIdentityReplacement, RetainedEntityIdentityReplacement, RuntimeError,
    RuntimeRetainedIdentityMigration, RuntimeRetainedIdentityMigrationReport, SmartHomeRuntime,
};
use smart_home_runtime_store::{
    DurableAutomationDefinition, RuntimeStoreError, SmartHomeRuntimeStore,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use storage_core::{Revision, StorageBackend};
use tls_platform::{default_connector, TlsConfig, TlsConnector};
use url_parser::{Url, UrlError};

pub const VERSION: &str = "0.2.0";
pub const INTEGRATION_ID: &str = "enphase_envoy";
pub const PROTOCOL_ID: &str = "enphase_iq_gateway_local_api";
pub const METERS_PATH: &str = "/ivp/meters";
pub const METER_READINGS_PATH: &str = "/ivp/meters/readings";
pub const INVERTER_PRODUCTION_PATH: &str = "/api/v1/production/inverters";
pub const DEFAULT_MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_METERS: usize = 16;
pub const MAX_INVERTERS: usize = 1_024;
const MAX_SECRET_BYTES: usize = 16 * 1024;
const MAX_TEXT_BYTES: usize = 1_024;

#[derive(Debug)]
pub enum EnphaseError {
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
    DataGovernanceDenied(DataGovernanceDenial),
    Runtime(RuntimeError),
    RuntimeStore(RuntimeStoreError),
    Lease(LeaseError),
}

impl fmt::Display for EnphaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) => write!(formatter, "invalid Enphase input: {message}"),
            Self::LocalHttp(error) => error.fmt(formatter),
            Self::Url(error) => write!(formatter, "invalid Enphase URL: {error}"),
            Self::Io(message) => write!(formatter, "Enphase LAN I/O failed: {message}"),
            Self::Tls(message) => write!(formatter, "Enphase TLS failed: {message}"),
            Self::Http(message) => write!(formatter, "invalid Enphase HTTP response: {message}"),
            Self::HttpStatus { operation, status } => {
                write!(formatter, "Enphase {operation} returned HTTP {status}")
            }
            Self::ResponseTooLarge { limit } => {
                write!(formatter, "Enphase response exceeds {limit} bytes")
            }
            Self::TruncatedBody { expected, actual } => write!(
                formatter,
                "Enphase response is truncated: expected {expected} bytes, got {actual}"
            ),
            Self::Json(error) => write!(formatter, "invalid Enphase JSON: {error}"),
            Self::MissingField(field) => write!(formatter, "Enphase response is missing {field}"),
            Self::DataGovernanceDenied(reason) => {
                write!(
                    formatter,
                    "Enphase data-governance policy denied the request: {reason:?}"
                )
            }
            Self::Runtime(error) => error.fmt(formatter),
            Self::RuntimeStore(error) => error.fmt(formatter),
            Self::Lease(error) => write!(formatter, "Enphase identifier-key lease failed: {error}"),
        }
    }
}

impl std::error::Error for EnphaseError {}

impl From<LocalHttpError> for EnphaseError {
    fn from(error: LocalHttpError) -> Self {
        Self::LocalHttp(error)
    }
}

impl From<UrlError> for EnphaseError {
    fn from(error: UrlError) -> Self {
        Self::Url(error)
    }
}

impl From<serde_json::Error> for EnphaseError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<RuntimeError> for EnphaseError {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

impl From<RuntimeStoreError> for EnphaseError {
    fn from(error: RuntimeStoreError) -> Self {
        Self::RuntimeStore(error)
    }
}

impl From<LeaseError> for EnphaseError {
    fn from(error: LeaseError) -> Self {
        Self::Lease(error)
    }
}

pub struct EnphaseAccessToken {
    token: Zeroizing<String>,
}

impl EnphaseAccessToken {
    pub fn new(token: impl Into<String>) -> Result<Self, EnphaseError> {
        let token = token.into();
        if token.trim().is_empty()
            || token.len() > MAX_SECRET_BYTES
            || token.bytes().any(|byte| byte <= 0x20 || byte == 0x7f)
        {
            return Err(EnphaseError::Validation(
                "access token must be bounded non-whitespace HTTP text".to_string(),
            ));
        }
        Ok(Self {
            token: Zeroizing::new(token),
        })
    }
}

impl fmt::Debug for EnphaseAccessToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EnphaseAccessToken([REDACTED])")
    }
}

pub struct EnphaseIdentifierKey {
    bytes: Zeroizing<Vec<u8>>,
}

impl EnphaseIdentifierKey {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, EnphaseError> {
        let bytes = bytes.into();
        if bytes.len() != 32 {
            return Err(EnphaseError::Validation(
                "identifier pseudonymization key must contain exactly 32 bytes".to_string(),
            ));
        }
        Ok(Self {
            bytes: Zeroizing::new(bytes),
        })
    }
}

impl fmt::Debug for EnphaseIdentifierKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EnphaseIdentifierKey([REDACTED])")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnphaseConfig {
    pub bridge_id: BridgeId,
    pub base_url: String,
    pub gateway_serial: String,
    pub token_ref: VaultRef,
    pub timeout: Duration,
}

impl EnphaseConfig {
    pub fn new(
        bridge_id: BridgeId,
        base_url: impl Into<String>,
        gateway_serial: impl Into<String>,
        token_ref: VaultRef,
    ) -> Result<Self, EnphaseError> {
        let base_url = base_url.into().trim_end_matches('/').to_string();
        let parsed = Url::parse(&base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(EnphaseError::MissingField("base URL host"))?;
        let secure = parsed.scheme == "https";
        let test_loopback = parsed.scheme == "http" && is_loopback_host(host);
        if (!secure && !test_loopback)
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
        {
            return Err(EnphaseError::Validation(
                "base URL must be a credential-free HTTPS origin; HTTP is test-only on loopback"
                    .to_string(),
            ));
        }
        let gateway_serial = bounded_text(gateway_serial.into(), "gateway serial")?;
        if !gateway_serial
            .chars()
            .all(|character| character.is_ascii_digit())
        {
            return Err(EnphaseError::Validation(
                "gateway serial must contain only decimal digits".to_string(),
            ));
        }
        Ok(Self {
            bridge_id,
            base_url,
            gateway_serial,
            token_ref,
            timeout: Duration::from_secs(5),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout.max(Duration::from_millis(1));
        self
    }

    fn endpoint(&self) -> Result<LocalHttpEndpoint, EnphaseError> {
        let parsed = Url::parse(&self.base_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(EnphaseError::MissingField("base URL host"))?;
        let scheme = match parsed.scheme.as_str() {
            "https" => LocalHttpScheme::Https,
            "http" if is_loopback_host(host) => LocalHttpScheme::Http,
            _ => {
                return Err(EnphaseError::Validation(
                    "Enphase endpoint is not approved".to_string(),
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
            "enphase.iq-gateway.local-api",
        )))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnphaseMeter {
    pub eid: u64,
    pub state: String,
    pub measurement_type: String,
    pub phase_mode: String,
    pub phase_count: u64,
    pub metering_status: String,
    pub status_flags: Vec<String>,
    pub timestamp: u64,
    pub active_energy_delivered_wh: f64,
    pub active_energy_received_wh: f64,
    pub instantaneous_demand_w: f64,
    pub active_power_w: f64,
    pub apparent_power_va: f64,
    pub reactive_power_var: f64,
    pub power_factor: f64,
    pub voltage_v: f64,
    pub current_a: f64,
    pub frequency_hz: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnphaseSnapshot {
    pub meters: Vec<EnphaseMeter>,
    pub inverters: Vec<EnphaseInverter>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnphaseInverter {
    pub pseudonym: String,
    pub last_report_date: u64,
    pub device_type: u64,
    pub last_report_watts: f64,
    pub max_report_watts: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnphaseInverterIdentityRotation {
    pub source_pseudonym: String,
    pub destination_pseudonym: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnphaseIdentifierKeyRotationReport {
    pub rotated_inverters: usize,
    pub migration: RuntimeRetainedIdentityMigrationReport,
    pub revision: Revision,
}

pub struct EnphaseIdentifierKeyRotationRequest<'a> {
    pub principal_id: AgentId,
    pub source_key_lease_id: &'a LeaseId,
    pub destination_key_lease_id: &'a LeaseId,
    pub automation_definitions: &'a [DurableAutomationDefinition],
    pub automation_state: Option<JsonValue>,
    pub observed_at_ms: u64,
    pub expected_revision: Revision,
}

impl<'a> EnphaseIdentifierKeyRotationRequest<'a> {
    pub fn new(
        principal_id: AgentId,
        source_key_lease_id: &'a LeaseId,
        destination_key_lease_id: &'a LeaseId,
        observed_at_ms: u64,
        expected_revision: Revision,
    ) -> Self {
        Self {
            principal_id,
            source_key_lease_id,
            destination_key_lease_id,
            automation_definitions: &[],
            automation_state: None,
            observed_at_ms,
            expected_revision,
        }
    }

    pub fn with_automation_context(
        mut self,
        automation_definitions: &'a [DurableAutomationDefinition],
        automation_state: Option<JsonValue>,
    ) -> Self {
        self.automation_definitions = automation_definitions;
        self.automation_state = automation_state;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnphaseRequestPlans {
    pub meters: LocalHttpRequestPlan,
    pub readings: LocalHttpRequestPlan,
    pub inverters: LocalHttpRequestPlan,
}

pub trait EnphaseTransport {
    fn inspect(
        &mut self,
        plans: &EnphaseRequestPlans,
        token: &EnphaseAccessToken,
    ) -> Result<EnphaseSnapshot, EnphaseError>;

    fn inspect_inverters(
        &mut self,
        _plan: &LocalHttpRequestPlan,
        _token: &EnphaseAccessToken,
        _identifier_key: &EnphaseIdentifierKey,
        _gateway_serial: &str,
    ) -> Result<Vec<EnphaseInverter>, EnphaseError> {
        Err(EnphaseError::Validation(
            "transport does not implement per-inverter inspection".to_string(),
        ))
    }

    fn inspect_inverter_identity_rotation(
        &mut self,
        _plan: &LocalHttpRequestPlan,
        _token: &EnphaseAccessToken,
        _source_key: &EnphaseIdentifierKey,
        _destination_key: &EnphaseIdentifierKey,
        _gateway_serial: &str,
    ) -> Result<Vec<EnphaseInverterIdentityRotation>, EnphaseError> {
        Err(EnphaseError::Validation(
            "transport does not implement per-inverter identity rotation".to_string(),
        ))
    }
}

pub struct EnphaseLanTransport {
    connector: Box<dyn TlsConnector>,
    tls_config: TlsConfig,
    maximum_response_bytes: usize,
}

impl Default for EnphaseLanTransport {
    fn default() -> Self {
        Self::new(default_connector(), TlsConfig::https_default())
    }
}

impl EnphaseLanTransport {
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
        token: &EnphaseAccessToken,
    ) -> Result<HttpResponse, EnphaseError> {
        let request = Zeroizing::new(encode_http_request(plan, token.token.as_str())?);
        let url = Url::parse(&plan.url)?;
        let host = url
            .host
            .as_deref()
            .ok_or(EnphaseError::MissingField("request URL host"))?;
        let port = url
            .effective_port()
            .ok_or(EnphaseError::MissingField("request URL port"))?;
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
                    .map_err(|error| EnphaseError::Tls(error.to_string()))?;
                write_request(&mut stream, request.as_slice())?;
                let bytes = Zeroizing::new(read_bounded(&mut stream, self.maximum_response_bytes)?);
                stream
                    .close_notify()
                    .map_err(|error| EnphaseError::Tls(error.to_string()))?;
                bytes
            }
            _ => {
                return Err(EnphaseError::Validation(
                    "Enphase transport requires HTTPS or loopback HTTP".to_string(),
                ))
            }
        };
        decode_http_response(response.as_slice(), self.maximum_response_bytes)
    }

    fn get_json(
        &mut self,
        plan: &LocalHttpRequestPlan,
        token: &EnphaseAccessToken,
        operation: &'static str,
    ) -> Result<JsonValue, EnphaseError> {
        let response = self.request(plan, token)?;
        if response.status != 200 {
            return Err(EnphaseError::HttpStatus {
                operation,
                status: response.status,
            });
        }
        Ok(serde_json::from_slice(&response.body)?)
    }

    fn get_sensitive_json(
        &mut self,
        plan: &LocalHttpRequestPlan,
        token: &EnphaseAccessToken,
        operation: &'static str,
    ) -> Result<SensitiveJson, EnphaseError> {
        let response = self.request(plan, token)?;
        if response.status != 200 {
            return Err(EnphaseError::HttpStatus {
                operation,
                status: response.status,
            });
        }
        Ok(SensitiveJson(serde_json::from_slice(&response.body)?))
    }
}

impl EnphaseTransport for EnphaseLanTransport {
    fn inspect(
        &mut self,
        plans: &EnphaseRequestPlans,
        token: &EnphaseAccessToken,
    ) -> Result<EnphaseSnapshot, EnphaseError> {
        let meters = self.get_json(&plans.meters, token, "meter inventory")?;
        let readings = self.get_json(&plans.readings, token, "meter readings")?;
        parse_snapshot(&meters, &readings)
    }

    fn inspect_inverters(
        &mut self,
        plan: &LocalHttpRequestPlan,
        token: &EnphaseAccessToken,
        identifier_key: &EnphaseIdentifierKey,
        gateway_serial: &str,
    ) -> Result<Vec<EnphaseInverter>, EnphaseError> {
        let response = self.get_sensitive_json(plan, token, "inverter production")?;
        parse_inverters(&response.0, identifier_key, gateway_serial)
    }

    fn inspect_inverter_identity_rotation(
        &mut self,
        plan: &LocalHttpRequestPlan,
        token: &EnphaseAccessToken,
        source_key: &EnphaseIdentifierKey,
        destination_key: &EnphaseIdentifierKey,
        gateway_serial: &str,
    ) -> Result<Vec<EnphaseInverterIdentityRotation>, EnphaseError> {
        let response = self.get_sensitive_json(plan, token, "inverter production")?;
        parse_inverter_identity_rotation(&response.0, source_key, destination_key, gateway_serial)
    }
}

pub struct EnphaseClient<T> {
    config: EnphaseConfig,
    token: EnphaseAccessToken,
    transport: T,
    plans: EnphaseRequestPlans,
    identifier_key: Option<EnphaseIdentifierKey>,
}

impl<T: EnphaseTransport> EnphaseClient<T> {
    pub fn new(
        config: EnphaseConfig,
        token: EnphaseAccessToken,
        transport: T,
    ) -> Result<Self, EnphaseError> {
        let endpoint = config.endpoint()?;
        let timeout_ms = duration_ms(config.timeout);
        let meters = get_plan(&endpoint, &config.token_ref, METERS_PATH, timeout_ms)?;
        let readings = get_plan(
            &endpoint,
            &config.token_ref,
            METER_READINGS_PATH,
            timeout_ms,
        )?;
        let inverters = get_plan(
            &endpoint,
            &config.token_ref,
            INVERTER_PRODUCTION_PATH,
            timeout_ms,
        )?;
        Ok(Self {
            config,
            token,
            transport,
            plans: EnphaseRequestPlans {
                meters,
                readings,
                inverters,
            },
            identifier_key: None,
        })
    }

    pub fn with_identifier_key(mut self, identifier_key: EnphaseIdentifierKey) -> Self {
        self.identifier_key = Some(identifier_key);
        self
    }

    pub fn inspect(&mut self) -> Result<EnphaseSnapshot, EnphaseError> {
        self.transport.inspect(&self.plans, &self.token)
    }

    pub fn inspect_with_inverters(&mut self) -> Result<EnphaseSnapshot, EnphaseError> {
        let identifier_key = self.identifier_key.as_ref().ok_or_else(|| {
            EnphaseError::Validation(
                "per-inverter inspection requires a Vault-leased identifier key".to_string(),
            )
        })?;
        let mut snapshot = self.transport.inspect(&self.plans, &self.token)?;
        snapshot.inverters = self.transport.inspect_inverters(
            &self.plans.inverters,
            &self.token,
            identifier_key,
            &self.config.gateway_serial,
        )?;
        Ok(snapshot)
    }

    fn inspect_inverter_identity_rotation(
        &mut self,
        source_key: &EnphaseIdentifierKey,
        destination_key: &EnphaseIdentifierKey,
    ) -> Result<Vec<EnphaseInverterIdentityRotation>, EnphaseError> {
        self.transport.inspect_inverter_identity_rotation(
            &self.plans.inverters,
            &self.token,
            source_key,
            destination_key,
            &self.config.gateway_serial,
        )
    }
}

impl<T> fmt::Debug for EnphaseClient<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EnphaseClient")
            .field("config", &self.config)
            .field("token", &"[REDACTED]")
            .field(
                "identifier_key",
                &self.identifier_key.as_ref().map(|_| "[REDACTED]"),
            )
            .field("plans", &self.plans)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledEnphaseGateway {
    pub bridge_id: BridgeId,
    pub device_id: DeviceId,
    pub meter_entity_ids: Vec<EntityId>,
    pub inverter_entity_ids: Vec<EntityId>,
}

pub struct EnphaseRuntimeIntegration<T> {
    client: EnphaseClient<T>,
    data_governance: DataGovernancePolicy,
}

impl<T: EnphaseTransport> EnphaseRuntimeIntegration<T> {
    pub fn new(client: EnphaseClient<T>) -> Self {
        Self {
            client,
            data_governance: DataGovernancePolicy::default(),
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
    ) -> Result<InstalledEnphaseGateway, EnphaseError> {
        authorize_read(runtime, principal_id, observed_at_ms)?;
        let snapshot = self.client.inspect()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }

    pub fn inspect_inverters_and_install_authorized(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        principal_id: AgentId,
        observed_at_ms: u64,
    ) -> Result<InstalledEnphaseGateway, EnphaseError> {
        authorize_read(runtime, principal_id.clone(), observed_at_ms)?;
        authorize_identifier_inspection(
            &self.data_governance,
            &principal_id,
            &self.client.config.gateway_serial,
            observed_at_ms,
        )?;
        let snapshot = self.client.inspect_with_inverters()?;
        install_snapshot(runtime, &self.client.config, &snapshot, observed_at_ms)
    }

    pub fn rotate_inverter_identifier_key_authorized<B, L>(
        &mut self,
        runtime: &mut SmartHomeRuntime,
        store: &SmartHomeRuntimeStore<B>,
        leases: &L,
        request: EnphaseIdentifierKeyRotationRequest<'_>,
    ) -> Result<EnphaseIdentifierKeyRotationReport, EnphaseError>
    where
        B: StorageBackend,
        L: LeaseManager + ?Sized,
    {
        if request.source_key_lease_id == request.destination_key_lease_id {
            return Err(EnphaseError::Validation(
                "source and destination identifier-key leases must be distinct".to_string(),
            ));
        }
        authorize_read(
            runtime,
            request.principal_id.clone(),
            request.observed_at_ms,
        )?;
        authorize_identifier_inspection(
            &self.data_governance,
            &request.principal_id,
            &self.client.config.gateway_serial,
            request.observed_at_ms,
        )?;

        let mut payloads = leases.consume_many(&[
            request.source_key_lease_id.clone(),
            request.destination_key_lease_id.clone(),
        ])?;
        let destination_payload = payloads
            .pop()
            .expect("two requested leases return two ordered payloads");
        let source_payload = payloads
            .pop()
            .expect("two requested leases return two ordered payloads");
        let source_key = EnphaseIdentifierKey::new(source_payload.as_bytes().to_vec())?;
        let destination_key = EnphaseIdentifierKey::new(destination_payload.as_bytes().to_vec())?;
        drop(source_payload);
        drop(destination_payload);
        if source_key.bytes.as_slice() == destination_key.bytes.as_slice() {
            return Err(EnphaseError::Validation(
                "destination identifier key must differ from the source key".to_string(),
            ));
        }

        let destination_namespace =
            gateway_identity_pseudonym(&destination_key, &self.client.config.gateway_serial);
        let rotations = self
            .client
            .inspect_inverter_identity_rotation(&source_key, &destination_key)?;
        drop(source_key);
        drop(destination_key);

        let rotated_inverters = rotations.len();
        let migration = build_inverter_identity_migration(
            runtime,
            &self.client.config,
            &rotations,
            &destination_namespace,
        )?;
        let (migration, revision) = store.migrate_retained_identities(
            runtime,
            &migration,
            request.automation_definitions,
            request.automation_state,
            request.observed_at_ms,
            request.expected_revision,
        )?;
        Ok(EnphaseIdentifierKeyRotationReport {
            rotated_inverters,
            migration,
            revision,
        })
    }
}

pub fn paired_discovery_record(
    config: &EnphaseConfig,
    discovered_at_ms: u64,
) -> Result<DiscoveryRecord, EnphaseError> {
    Ok(DiscoveryRecord::new(
        IntegrationId::trusted(INTEGRATION_ID),
        ProtocolFamily::Vendor(PROTOCOL_ID.to_string()),
        stable_component(&config.gateway_serial),
        DiscoverySource::Manual,
        BridgeTransport::LanHttp,
        discovered_at_ms,
    )
    .map_err(|error| EnphaseError::Validation(error.to_string()))?
    .with_display_name("Enphase IQ Gateway")
    .with_address(config.base_url.clone())
    .with_hardware_model("IQ Gateway")
    .with_confidence(DiscoveryConfidence::Paired)
    .with_pairing_requirement(PairingRequirement::Credentials)
    .with_metadata("enphase.protocol", PROTOCOL_ID)
    .with_metadata("enphase.gateway_serial", config.gateway_serial.clone()))
}

pub fn install_snapshot(
    runtime: &mut SmartHomeRuntime,
    config: &EnphaseConfig,
    snapshot: &EnphaseSnapshot,
    observed_at_ms: u64,
) -> Result<InstalledEnphaseGateway, EnphaseError> {
    if snapshot.meters.is_empty() {
        return Err(EnphaseError::Validation(
            "meter snapshot must not be empty".to_string(),
        ));
    }
    validate_inverter_snapshot(&snapshot.inverters)?;
    let serial_component = stable_component(&config.gateway_serial);
    let device_id = DeviceId::trusted(format!("enphase:{serial_component}"));
    let health = aggregate_health(&snapshot.meters);
    let mut bridge = Bridge::new(
        config.bridge_id.clone(),
        IntegrationId::trusted(INTEGRATION_ID),
        BridgeTransport::LanHttp,
    );
    bridge.address = Some(config.base_url.clone());
    bridge.hardware_model = Some("IQ Gateway".to_string());
    bridge.auth_ref = Some(config.token_ref.clone());
    bridge.health = health;
    bridge.last_seen_at_ms = Some(observed_at_ms);
    bridge.identifiers = vec![protocol_identifier("https_endpoint", &config.base_url)?];
    bridge.metadata = vec![
        Metadata::new("enphase.transport", "local_bearer_token"),
        Metadata::new("enphase.meter_count", snapshot.meters.len().to_string()),
        Metadata::new(
            "enphase.inverter_count",
            snapshot.inverters.len().to_string(),
        ),
    ];
    runtime.upsert_bridge(bridge)?;

    let meter_entity_ids = snapshot
        .meters
        .iter()
        .map(|meter| EntityId::trusted(format!("enphase:{serial_component}:meter:{}", meter.eid)))
        .collect::<Vec<_>>();
    let inverter_entity_ids = snapshot
        .inverters
        .iter()
        .map(|inverter| {
            EntityId::trusted(format!(
                "enphase:{serial_component}:inverter:{}",
                inverter.pseudonym
            ))
        })
        .collect::<Vec<_>>();
    let mut entity_ids = meter_entity_ids.clone();
    entity_ids.extend(inverter_entity_ids.iter().cloned());
    runtime.upsert_device(Device {
        device_id: device_id.clone(),
        bridge_id: config.bridge_id.clone(),
        manufacturer: "Enphase".to_string(),
        model: "IQ Gateway".to_string(),
        name: "Enphase IQ Gateway".to_string(),
        serial: Some(config.gateway_serial.clone()),
        firmware_version: None,
        room_id: None,
        entity_ids,
        identifiers: vec![protocol_identifier(
            "gateway_serial",
            &config.gateway_serial,
        )?],
        health,
        metadata: vec![
            Metadata::new(
                "enphase.native_meter_count",
                snapshot.meters.len().to_string(),
            ),
            Metadata::new(
                "enphase.pseudonymous_inverter_count",
                snapshot.inverters.len().to_string(),
            ),
        ],
    })?;
    for (meter, entity_id) in snapshot.meters.iter().zip(&meter_entity_ids) {
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Sensor,
            name: format!("Enphase {} meter", display_name(&meter.measurement_type)),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("sensor.measurement"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: meter_value(meter),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![
                Metadata::new("enphase.eid", meter.eid.to_string()),
                Metadata::new("enphase.measurement_type", &meter.measurement_type),
                Metadata::new("enphase.native_state", &meter.state),
                Metadata::new("enphase.metering_status", &meter.metering_status),
            ],
        })?;
    }
    for (inverter, entity_id) in snapshot.inverters.iter().zip(&inverter_entity_ids) {
        runtime.upsert_entity(Entity {
            entity_id: entity_id.clone(),
            device_id: device_id.clone(),
            kind: EntityKind::Sensor,
            name: format!("Enphase microinverter {}", &inverter.pseudonym[..8]),
            capabilities: vec![Capability::new(
                CapabilityId::trusted("sensor.measurement"),
                CapabilityMode::Observe,
                ValueKind::Object,
            )],
            state: Some(StateSnapshot {
                entity_id: entity_id.clone(),
                value: inverter_value(inverter),
                source: StateSource::Poll,
                observed_at_ms,
                received_at_ms: observed_at_ms,
                expires_at_ms: None,
                confidence: StateConfidence::Confirmed,
            }),
            metadata: vec![
                Metadata::new("enphase.identifier_form", "keyed_pseudonym"),
                Metadata::new("enphase.inverter_pseudonym", &inverter.pseudonym),
                Metadata::new("enphase.device_type", inverter.device_type.to_string()),
            ],
        })?;
    }
    Ok(InstalledEnphaseGateway {
        bridge_id: config.bridge_id.clone(),
        device_id,
        meter_entity_ids,
        inverter_entity_ids,
    })
}

fn validate_inverter_snapshot(inverters: &[EnphaseInverter]) -> Result<(), EnphaseError> {
    if inverters.len() > MAX_INVERTERS {
        return Err(EnphaseError::Validation(format!(
            "inverter production exceeds {MAX_INVERTERS} entries"
        )));
    }
    let mut seen = BTreeSet::new();
    for inverter in inverters {
        if inverter.pseudonym.len() != 32
            || !inverter
                .pseudonym
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(EnphaseError::Validation(
                "microinverter pseudonym must be 128-bit lowercase hexadecimal text".to_string(),
            ));
        }
        if !seen.insert(&inverter.pseudonym) {
            return Err(EnphaseError::Validation(
                "duplicate microinverter pseudonym".to_string(),
            ));
        }
        if !inverter.last_report_watts.is_finite()
            || inverter.last_report_watts < 0.0
            || !inverter.max_report_watts.is_finite()
            || inverter.max_report_watts < 0.0
            || inverter.last_report_watts > inverter.max_report_watts
        {
            return Err(EnphaseError::Validation(
                "microinverter power readings are invalid".to_string(),
            ));
        }
    }
    Ok(())
}

fn authorize_read(
    runtime: &mut SmartHomeRuntime,
    principal_id: AgentId,
    now_ms: u64,
) -> Result<(), EnphaseError> {
    let tool = SmartHomeTool::GetState;
    let decision = runtime.authorize_tool_for_principal(principal_id.clone(), tool, now_ms);
    if decision.missing_capabilities.is_empty() {
        Ok(())
    } else {
        Err(EnphaseError::Runtime(RuntimeError::UnauthorizedTool {
            principal_id,
            tool,
            missing_capabilities: decision.missing_capabilities,
        }))
    }
}

fn authorize_identifier_inspection(
    policy: &DataGovernancePolicy,
    principal_id: &AgentId,
    gateway_serial: &str,
    now_ms: u64,
) -> Result<(), EnphaseError> {
    let resource_id = format!(
        "enphase:{}:microinverters",
        stable_component(gateway_serial)
    );
    match policy.decide(&DataUseRequest {
        principal_id,
        resource_id: &resource_id,
        category: DataCategory::DeviceIdentifier,
        operation: DataOperation::Inspect,
        destination: DataDestination::LocalDevice,
        retention: DataRetention::Ephemeral,
        now_ms,
    }) {
        DataGovernanceDecision::Allow(_) => Ok(()),
        DataGovernanceDecision::Deny(reason) => Err(EnphaseError::DataGovernanceDenied(reason)),
    }
}

fn parse_snapshot(
    meters: &JsonValue,
    readings: &JsonValue,
) -> Result<EnphaseSnapshot, EnphaseError> {
    let meters = meters
        .as_array()
        .ok_or(EnphaseError::MissingField("meter inventory array"))?;
    let readings = readings
        .as_array()
        .ok_or(EnphaseError::MissingField("meter readings array"))?;
    if meters.is_empty() || meters.len() > MAX_METERS || readings.len() > MAX_METERS {
        return Err(EnphaseError::Validation(format!(
            "meter inventory must contain 1-{MAX_METERS} entries"
        )));
    }
    let mut readings_by_eid = BTreeMap::new();
    for reading in readings {
        let reading = reading
            .as_object()
            .ok_or(EnphaseError::MissingField("meter reading object"))?;
        let eid = required_u64(reading, "eid")?;
        if readings_by_eid.insert(eid, reading).is_some() {
            return Err(EnphaseError::Validation(format!(
                "duplicate meter reading EID {eid}"
            )));
        }
    }
    let mut seen = BTreeSet::new();
    let mut parsed = Vec::with_capacity(meters.len());
    for meter in meters {
        let meter = meter
            .as_object()
            .ok_or(EnphaseError::MissingField("meter inventory object"))?;
        let eid = required_u64(meter, "eid")?;
        if !seen.insert(eid) {
            return Err(EnphaseError::Validation(format!(
                "duplicate meter inventory EID {eid}"
            )));
        }
        let reading = readings_by_eid.remove(&eid).ok_or_else(|| {
            EnphaseError::Validation(format!("missing reading for meter EID {eid}"))
        })?;
        parsed.push(parse_meter(meter, reading)?);
    }
    if let Some(unknown) = readings_by_eid.keys().next() {
        return Err(EnphaseError::Validation(format!(
            "reading references unknown meter EID {unknown}"
        )));
    }
    parsed.sort_by_key(|meter| meter.eid);
    Ok(EnphaseSnapshot {
        meters: parsed,
        inverters: Vec::new(),
    })
}

fn parse_meter(
    meter: &JsonMap<String, JsonValue>,
    reading: &JsonMap<String, JsonValue>,
) -> Result<EnphaseMeter, EnphaseError> {
    Ok(EnphaseMeter {
        eid: required_u64(meter, "eid")?,
        state: normalized_text(meter, "state")?,
        measurement_type: normalized_text(meter, "measurementType")?,
        phase_mode: normalized_text(meter, "phaseMode")?,
        phase_count: required_u64(meter, "phaseCount")?,
        metering_status: normalized_text(meter, "meteringStatus")?,
        status_flags: string_array(meter, "statusFlags")?,
        timestamp: required_u64(reading, "timestamp")?,
        active_energy_delivered_wh: required_f64(reading, "actEnergyDlvd")?,
        active_energy_received_wh: required_f64(reading, "actEnergyRcvd")?,
        instantaneous_demand_w: required_f64(reading, "instantaneousDemand")?,
        active_power_w: required_f64(reading, "activePower")?,
        apparent_power_va: required_f64(reading, "apparentPower")?,
        reactive_power_var: required_f64(reading, "reactivePower")?,
        power_factor: required_f64(reading, "pwrFactor")?,
        voltage_v: required_f64(reading, "voltage")?,
        current_a: required_f64(reading, "current")?,
        frequency_hz: required_f64(reading, "freq")?,
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

fn parse_inverters(
    data: &JsonValue,
    identifier_key: &EnphaseIdentifierKey,
    gateway_serial: &str,
) -> Result<Vec<EnphaseInverter>, EnphaseError> {
    let values = data
        .as_array()
        .ok_or(EnphaseError::MissingField("inverter production array"))?;
    if values.len() > MAX_INVERTERS {
        return Err(EnphaseError::Validation(format!(
            "inverter production exceeds {MAX_INVERTERS} entries"
        )));
    }
    let mut seen = BTreeSet::new();
    let mut inverters = Vec::with_capacity(values.len());
    for value in values {
        let (serial, last_report_date, device_type, last_report_watts, max_report_watts) =
            parse_inverter_record(value)?;
        let pseudonym = inverter_pseudonym(identifier_key, gateway_serial, serial);
        if !seen.insert(pseudonym.clone()) {
            return Err(EnphaseError::Validation(
                "duplicate microinverter identifier".to_string(),
            ));
        }
        inverters.push(EnphaseInverter {
            pseudonym,
            last_report_date,
            device_type,
            last_report_watts,
            max_report_watts,
        });
    }
    inverters.sort_by(|left, right| left.pseudonym.cmp(&right.pseudonym));
    Ok(inverters)
}

fn parse_inverter_identity_rotation(
    data: &JsonValue,
    source_key: &EnphaseIdentifierKey,
    destination_key: &EnphaseIdentifierKey,
    gateway_serial: &str,
) -> Result<Vec<EnphaseInverterIdentityRotation>, EnphaseError> {
    let values = data
        .as_array()
        .ok_or(EnphaseError::MissingField("inverter production array"))?;
    if values.is_empty() || values.len() > MAX_INVERTERS {
        return Err(EnphaseError::Validation(format!(
            "inverter rotation response must contain 1-{MAX_INVERTERS} entries"
        )));
    }
    let mut source_pseudonyms = BTreeSet::new();
    let mut destination_pseudonyms = BTreeSet::new();
    let mut rotations = Vec::with_capacity(values.len());
    for value in values {
        let (serial, _, _, _, _) = parse_inverter_record(value)?;
        let source_pseudonym = inverter_pseudonym(source_key, gateway_serial, serial);
        let destination_pseudonym = inverter_pseudonym(destination_key, gateway_serial, serial);
        if source_pseudonym == destination_pseudonym {
            return Err(EnphaseError::Validation(
                "identifier-key rotation produced an unchanged pseudonym".to_string(),
            ));
        }
        if !source_pseudonyms.insert(source_pseudonym.clone()) {
            return Err(EnphaseError::Validation(
                "duplicate source microinverter identifier".to_string(),
            ));
        }
        if !destination_pseudonyms.insert(destination_pseudonym.clone()) {
            return Err(EnphaseError::Validation(
                "duplicate destination microinverter identifier".to_string(),
            ));
        }
        rotations.push(EnphaseInverterIdentityRotation {
            source_pseudonym,
            destination_pseudonym,
        });
    }
    rotations.sort_by(|left, right| left.source_pseudonym.cmp(&right.source_pseudonym));
    Ok(rotations)
}

fn parse_inverter_record(value: &JsonValue) -> Result<(&str, u64, u64, f64, f64), EnphaseError> {
    let inverter = value
        .as_object()
        .ok_or(EnphaseError::MissingField("inverter production object"))?;
    let serial = inverter
        .get("serialNumber")
        .and_then(JsonValue::as_str)
        .ok_or(EnphaseError::MissingField("serialNumber"))?;
    if serial.is_empty() || serial.len() > 64 || !serial.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(EnphaseError::Validation(
            "microinverter serial must be bounded decimal text".to_string(),
        ));
    }
    let last_report_watts = required_nonnegative_f64(inverter, "lastReportWatts")?;
    let max_report_watts = required_nonnegative_f64(inverter, "maxReportWatts")?;
    if last_report_watts > max_report_watts {
        return Err(EnphaseError::Validation(
            "microinverter last report exceeds its maximum report".to_string(),
        ));
    }
    Ok((
        serial,
        required_u64(inverter, "lastReportDate")?,
        required_u64(inverter, "devType")?,
        last_report_watts,
        max_report_watts,
    ))
}

fn required_nonnegative_f64(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<f64, EnphaseError> {
    required_f64(object, field).and_then(|value| {
        if value >= 0.0 {
            Ok(value)
        } else {
            Err(EnphaseError::Validation(format!(
                "{field} must not be negative"
            )))
        }
    })
}

fn inverter_pseudonym(
    identifier_key: &EnphaseIdentifierKey,
    gateway_serial: &str,
    inverter_serial: &str,
) -> String {
    let mut input = Zeroizing::new(Vec::with_capacity(
        identifier_key.bytes.len() + gateway_serial.len() + inverter_serial.len() + 24,
    ));
    input.extend_from_slice(identifier_key.bytes.as_slice());
    input.extend_from_slice(b"enphase-inverter-v1\0");
    input.extend_from_slice(gateway_serial.as_bytes());
    input.push(0);
    input.extend_from_slice(inverter_serial.as_bytes());
    let digest = Zeroizing::new(sha256(input.as_slice()));
    lowercase_hex(&digest[..16])
}

fn gateway_identity_pseudonym(
    identifier_key: &EnphaseIdentifierKey,
    gateway_serial: &str,
) -> String {
    let mut input = Zeroizing::new(Vec::with_capacity(
        identifier_key.bytes.len() + gateway_serial.len() + 32,
    ));
    input.extend_from_slice(identifier_key.bytes.as_slice());
    input.extend_from_slice(b"enphase-gateway-identity-v1\0");
    input.extend_from_slice(gateway_serial.as_bytes());
    let digest = Zeroizing::new(sha256(input.as_slice()));
    lowercase_hex(&digest[..16])
}

fn build_inverter_identity_migration(
    runtime: &SmartHomeRuntime,
    config: &EnphaseConfig,
    rotations: &[EnphaseInverterIdentityRotation],
    destination_namespace: &str,
) -> Result<RuntimeRetainedIdentityMigration, EnphaseError> {
    if rotations.is_empty() {
        return Err(EnphaseError::Validation(
            "identifier-key rotation requires at least one microinverter".to_string(),
        ));
    }
    let source_devices = runtime
        .registry()
        .devices()
        .filter(|device| {
            device.bridge_id == config.bridge_id
                && device.identifiers.iter().any(|identifier| {
                    identifier.family == ProtocolFamily::Vendor(PROTOCOL_ID.to_string())
                        && identifier.kind == "gateway_serial"
                        && identifier.value == config.gateway_serial
                })
        })
        .collect::<Vec<_>>();
    let [source_device] = source_devices.as_slice() else {
        return Err(EnphaseError::Validation(
            "rotation requires exactly one installed Enphase gateway for the configured serial"
                .to_string(),
        ));
    };

    let destination_device_id = DeviceId::trusted(format!("enphase:{destination_namespace}"));
    let mut destinations_by_source = rotations
        .iter()
        .map(|rotation| {
            (
                rotation.source_pseudonym.clone(),
                rotation.destination_pseudonym.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    if destinations_by_source.len() != rotations.len() {
        return Err(EnphaseError::Validation(
            "rotation response contains duplicate source pseudonyms".to_string(),
        ));
    }

    let mut entity_replacements = Vec::with_capacity(source_device.entity_ids.len());
    let mut destination_entity_ids = Vec::with_capacity(source_device.entity_ids.len());
    let mut matched_inverters = 0usize;
    for source_entity_id in &source_device.entity_ids {
        let source_entity = runtime.registry().entity(source_entity_id).ok_or_else(|| {
            EnphaseError::Validation(format!(
                "installed gateway references missing entity {source_entity_id}"
            ))
        })?;
        let mut replacement = source_entity.clone();
        replacement.device_id = destination_device_id.clone();
        replacement.state = None;

        let destination_entity_id = if metadata_value(
            &source_entity.metadata,
            "enphase.identifier_form",
        ) == Some("keyed_pseudonym")
        {
            let source_pseudonym =
                metadata_value(&source_entity.metadata, "enphase.inverter_pseudonym").ok_or_else(
                    || {
                        EnphaseError::Validation(format!(
                            "inverter entity {source_entity_id} is missing its source pseudonym"
                        ))
                    },
                )?;
            let destination_pseudonym =
                destinations_by_source.remove(source_pseudonym).ok_or_else(|| {
                    EnphaseError::Validation(format!(
                        "inverter response does not correspond to installed entity {source_entity_id}"
                    ))
                })?;
            replacement.name = format!("Enphase microinverter {}", &destination_pseudonym[..8]);
            set_metadata(
                &mut replacement.metadata,
                "enphase.inverter_pseudonym",
                &destination_pseudonym,
            );
            matched_inverters = matched_inverters.saturating_add(1);
            EntityId::trusted(format!(
                "enphase:{destination_namespace}:inverter:{destination_pseudonym}"
            ))
        } else if let Some(eid) = metadata_value(&source_entity.metadata, "enphase.eid") {
            if eid.is_empty() || !eid.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(EnphaseError::Validation(format!(
                    "meter entity {source_entity_id} has an invalid native EID"
                )));
            }
            EntityId::trusted(format!("enphase:{destination_namespace}:meter:{eid}"))
        } else {
            return Err(EnphaseError::Validation(format!(
                "installed Enphase entity {source_entity_id} has no governed identity form"
            )));
        };
        replacement.entity_id = destination_entity_id.clone();
        destination_entity_ids.push(destination_entity_id);
        entity_replacements.push(RetainedEntityIdentityReplacement::new(
            source_entity_id.clone(),
            replacement,
        ));
    }
    if matched_inverters != rotations.len() || !destinations_by_source.is_empty() {
        return Err(EnphaseError::Validation(
            "inverter response does not exactly match the installed pseudonymous inverter set"
                .to_string(),
        ));
    }

    let mut replacement_device = (**source_device).clone();
    replacement_device.device_id = destination_device_id;
    replacement_device.entity_ids = destination_entity_ids;
    set_metadata(
        &mut replacement_device.metadata,
        "enphase.identifier_namespace",
        destination_namespace,
    );
    Ok(RuntimeRetainedIdentityMigration::new(vec![
        RetainedDeviceIdentityReplacement::new(
            source_device.device_id.clone(),
            replacement_device,
            entity_replacements,
        ),
    ]))
}

fn metadata_value<'a>(metadata: &'a [Metadata], key: &str) -> Option<&'a str> {
    metadata
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.value.as_str())
}

fn set_metadata(metadata: &mut Vec<Metadata>, key: &str, value: &str) {
    if let Some(entry) = metadata.iter_mut().find(|entry| entry.key == key) {
        entry.value = value.to_string();
    } else {
        metadata.push(Metadata::new(key, value));
    }
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

fn required_u64(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<u64, EnphaseError> {
    object
        .get(field)
        .and_then(JsonValue::as_u64)
        .ok_or(EnphaseError::MissingField(field))
}

fn required_f64(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<f64, EnphaseError> {
    object
        .get(field)
        .and_then(JsonValue::as_f64)
        .filter(|value| value.is_finite())
        .ok_or(EnphaseError::MissingField(field))
}

fn normalized_text(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<String, EnphaseError> {
    let text = object
        .get(field)
        .and_then(JsonValue::as_str)
        .ok_or(EnphaseError::MissingField(field))?
        .trim()
        .to_ascii_lowercase();
    bounded_text(text, field)
}

fn string_array(
    object: &JsonMap<String, JsonValue>,
    field: &'static str,
) -> Result<Vec<String>, EnphaseError> {
    let values = object
        .get(field)
        .and_then(JsonValue::as_array)
        .ok_or(EnphaseError::MissingField(field))?;
    if values.len() > 64 {
        return Err(EnphaseError::Validation(format!(
            "{field} contains too many values"
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        output.push(bounded_text(
            value
                .as_str()
                .ok_or(EnphaseError::MissingField(field))?
                .trim()
                .to_ascii_lowercase(),
            field,
        )?);
    }
    output.sort();
    output.dedup();
    Ok(output)
}

fn bounded_text(value: String, field: &'static str) -> Result<String, EnphaseError> {
    if value.trim().is_empty() || value.len() > MAX_TEXT_BYTES || value.contains(['\r', '\n', '\0'])
    {
        return Err(EnphaseError::Validation(format!(
            "{field} must be bounded non-empty text"
        )));
    }
    Ok(value)
}

fn aggregate_health(meters: &[EnphaseMeter]) -> Health {
    if meters
        .iter()
        .all(|meter| meter_health(meter) == Health::Offline)
    {
        Health::Offline
    } else if meters
        .iter()
        .any(|meter| meter_health(meter) != Health::Online)
    {
        Health::Degraded
    } else {
        Health::Online
    }
}

fn meter_health(meter: &EnphaseMeter) -> Health {
    if meter.state != "enabled" {
        Health::Offline
    } else if meter.metering_status == "normal" && meter.status_flags.is_empty() {
        Health::Online
    } else {
        Health::Degraded
    }
}

fn meter_value(meter: &EnphaseMeter) -> Value {
    Value::Object(vec![
        ("eid".to_string(), Value::Number(meter.eid as f64)),
        ("state".to_string(), Value::Text(meter.state.clone())),
        (
            "measurement_type".to_string(),
            Value::Text(meter.measurement_type.clone()),
        ),
        (
            "phase_mode".to_string(),
            Value::Text(meter.phase_mode.clone()),
        ),
        (
            "phase_count".to_string(),
            Value::Number(meter.phase_count as f64),
        ),
        (
            "metering_status".to_string(),
            Value::Text(meter.metering_status.clone()),
        ),
        (
            "status_flags".to_string(),
            Value::Array(
                meter
                    .status_flags
                    .iter()
                    .cloned()
                    .map(Value::Text)
                    .collect(),
            ),
        ),
        (
            "timestamp_s".to_string(),
            Value::Number(meter.timestamp as f64),
        ),
        (
            "active_energy_delivered_wh".to_string(),
            Value::Number(meter.active_energy_delivered_wh),
        ),
        (
            "active_energy_received_wh".to_string(),
            Value::Number(meter.active_energy_received_wh),
        ),
        (
            "instantaneous_demand_w".to_string(),
            Value::Number(meter.instantaneous_demand_w),
        ),
        (
            "active_power_w".to_string(),
            Value::Number(meter.active_power_w),
        ),
        (
            "apparent_power_va".to_string(),
            Value::Number(meter.apparent_power_va),
        ),
        (
            "reactive_power_var".to_string(),
            Value::Number(meter.reactive_power_var),
        ),
        (
            "power_factor".to_string(),
            Value::Number(meter.power_factor),
        ),
        ("voltage_v".to_string(), Value::Number(meter.voltage_v)),
        ("current_a".to_string(), Value::Number(meter.current_a)),
        (
            "frequency_hz".to_string(),
            Value::Number(meter.frequency_hz),
        ),
    ])
}

fn inverter_value(inverter: &EnphaseInverter) -> Value {
    Value::Object(vec![
        (
            "last_report_date_s".to_string(),
            Value::Number(inverter.last_report_date as f64),
        ),
        (
            "device_type".to_string(),
            Value::Number(inverter.device_type as f64),
        ),
        (
            "last_report_watts".to_string(),
            Value::Number(inverter.last_report_watts),
        ),
        (
            "max_report_watts".to_string(),
            Value::Number(inverter.max_report_watts),
        ),
    ])
}

fn protocol_identifier(kind: &str, value: &str) -> Result<ProtocolIdentifier, EnphaseError> {
    ProtocolIdentifier::new(ProtocolFamily::Vendor(PROTOCOL_ID.to_string()), kind, value)
        .map_err(|error| EnphaseError::Validation(error.to_string()))
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

fn display_name(value: &str) -> String {
    let words = value.replace(['_', '-'], " ");
    let mut characters = words.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().collect::<String>() + characters.as_str(),
        None => String::new(),
    }
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn get_plan(
    endpoint: &LocalHttpEndpoint,
    token_ref: &VaultRef,
    path: &str,
    timeout_ms: u64,
) -> Result<LocalHttpRequestPlan, EnphaseError> {
    Ok(LocalHttpRequestTemplate::new(LocalHttpMethod::Get, path)?
        .with_accept("application/json")
        .with_timeout_ms(timeout_ms)
        .with_auth(LocalHttpAuth::BearerToken {
            vault_ref: token_ref.clone(),
        })
        .plan(endpoint, Vec::new())?)
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn encode_http_request(plan: &LocalHttpRequestPlan, token: &str) -> Result<Vec<u8>, EnphaseError> {
    if token.is_empty()
        || token.len() > MAX_SECRET_BYTES
        || token.bytes().any(|byte| byte <= 0x20 || byte == 0x7f)
    {
        return Err(EnphaseError::Validation(
            "access token is unsafe for an HTTP header".to_string(),
        ));
    }
    let url = Url::parse(&plan.url)?;
    let host = url
        .host
        .as_deref()
        .ok_or(EnphaseError::MissingField("request URL host"))?;
    let port = url
        .effective_port()
        .ok_or(EnphaseError::MissingField("request URL port"))?;
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
        return Err(EnphaseError::Validation(
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
            || header.name.eq_ignore_ascii_case("Authorization")
        {
            continue;
        }
        if header.name.contains(['\r', '\n', '\0']) || header.value.contains(['\r', '\n', '\0']) {
            return Err(EnphaseError::Validation(
                "request header contains unsafe HTTP text".to_string(),
            ));
        }
        request.extend_from_slice(format!("{}: {}\r\n", header.name, header.value).as_bytes());
    }
    request.extend_from_slice(
        format!("Authorization: Bearer {token}\r\nContent-Length: 0\r\n\r\n").as_bytes(),
    );
    Ok(request)
}

fn connect_tcp(host: &str, port: u16, timeout: Duration) -> Result<TcpStream, EnphaseError> {
    let addresses = (host, port)
        .to_socket_addrs()
        .map_err(|error| EnphaseError::Io(error.to_string()))?
        .collect::<Vec<_>>();
    let mut last_error = None;
    for address in addresses {
        match TcpStream::connect_timeout(&address, timeout) {
            Ok(stream) => {
                stream
                    .set_read_timeout(Some(timeout))
                    .map_err(|error| EnphaseError::Io(error.to_string()))?;
                stream
                    .set_write_timeout(Some(timeout))
                    .map_err(|error| EnphaseError::Io(error.to_string()))?;
                return Ok(stream);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(EnphaseError::Io(
        last_error
            .map(|error| error.to_string())
            .unwrap_or_else(|| "no socket addresses resolved".to_string()),
    ))
}

fn write_request<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<(), EnphaseError> {
    writer
        .write_all(bytes)
        .map_err(|error| EnphaseError::Io(error.to_string()))?;
    writer
        .flush()
        .map_err(|error| EnphaseError::Io(error.to_string()))
}

fn read_bounded<R: Read>(reader: &mut R, maximum: usize) -> Result<Vec<u8>, EnphaseError> {
    let mut output = Vec::new();
    let mut chunk = [0u8; 8 * 1024];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(output),
            Ok(count) => {
                if output.len().saturating_add(count) > maximum {
                    output.zeroize();
                    return Err(EnphaseError::ResponseTooLarge { limit: maximum });
                }
                output.extend_from_slice(&chunk[..count]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(output)
            }
            Err(error) => return Err(EnphaseError::Io(error.to_string())),
        }
    }
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

fn decode_http_response(bytes: &[u8], maximum: usize) -> Result<HttpResponse, EnphaseError> {
    let parsed = parse_response_head(bytes)
        .map_err(|error: Http1ParseError| EnphaseError::Http(error.to_string()))?;
    let status = parsed.head.status;
    let mut headers = parsed.head.headers;
    let input = &bytes[parsed.body_offset..];
    let body = match (|| {
        let body = match parsed.body_kind {
            BodyKind::None => Vec::new(),
            BodyKind::ContentLength(expected) => {
                if input.len() < expected {
                    return Err(EnphaseError::TruncatedBody {
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
            return Err(EnphaseError::ResponseTooLarge { limit: maximum });
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

fn decode_chunked(bytes: &[u8], maximum: usize) -> Result<Vec<u8>, EnphaseError> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let line_end = find_crlf(bytes, cursor)
            .ok_or_else(|| EnphaseError::Http("incomplete chunk size".to_string()))?;
        let size_text = std::str::from_utf8(&bytes[cursor..line_end])
            .map_err(|_| EnphaseError::Http("chunk size is not ASCII".to_string()))?;
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or_default(), 16)
            .map_err(|_| EnphaseError::Http("invalid chunk size".to_string()))?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        if output.len().saturating_add(size) > maximum {
            output.zeroize();
            return Err(EnphaseError::ResponseTooLarge { limit: maximum });
        }
        let end = cursor
            .checked_add(size)
            .ok_or_else(|| EnphaseError::Http("chunk length overflow".to_string()))?;
        if end.saturating_add(2) > bytes.len() || &bytes[end..end + 2] != b"\r\n" {
            output.zeroize();
            return Err(EnphaseError::Http("truncated chunk".to_string()));
        }
        output.extend_from_slice(&bytes[cursor..end]);
        cursor = end + 2;
    }
}

fn find_crlf(bytes: &[u8], start: usize) -> Option<usize> {
    bytes
        .get(start..)?
        .windows(2)
        .position(|window| window == b"\r\n")
        .map(|offset| start + offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use coding_adventures_vault_leases::{InMemoryLeaseManager, LeasePayload, LeaseStatus};
    use smart_home_core::{CapabilityGrant, CapabilityGrantId, PrivilegeTier};
    use smart_home_data_governance::{ConsentReceiptRef, DataPurpose, DataUseGrant};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};
    use storage_local_folder::LocalFolderStorageBackend;

    const TOKEN: &str = "eyJhbGciOiJFUzI1NiJ9.test.signature";
    const IDENTIFIER_KEY: [u8; 32] = [0x5a; 32];
    const ROTATED_IDENTIFIER_KEY: [u8; 32] = [0x6b; 32];
    const INVERTER_SERIAL_1: &str = "121935144671";
    const INVERTER_SERIAL_2: &str = "121935144623";

    fn config(base_url: &str) -> EnphaseConfig {
        EnphaseConfig::new(
            BridgeId::trusted("enphase:bridge"),
            base_url,
            "122233344455",
            VaultRef::trusted("vault://smart-home/enphase/token"),
        )
        .unwrap()
    }

    fn meter_inventory() -> JsonValue {
        serde_json::json!([
            {
                "eid": 704643328_u64,
                "state": "enabled",
                "measurementType": "production",
                "phaseMode": "split",
                "phaseCount": 2,
                "meteringStatus": "normal",
                "statusFlags": []
            },
            {
                "eid": 704643584_u64,
                "state": "enabled",
                "measurementType": "net-consumption",
                "phaseMode": "split",
                "phaseCount": 2,
                "meteringStatus": "warning",
                "statusFlags": ["phase-imbalance"]
            }
        ])
    }

    fn meter_readings() -> JsonValue {
        serde_json::json!([
            {
                "eid": 704643584_u64,
                "timestamp": 1654218661_u64,
                "actEnergyDlvd": 48540.732,
                "actEnergyRcvd": 1244797.861,
                "instantaneousDemand": -0.0,
                "activePower": -0.0,
                "apparentPower": 34.831,
                "reactivePower": -0.0,
                "pwrFactor": 0.0,
                "voltage": 246.338,
                "current": 0.283,
                "freq": 59.188
            },
            {
                "eid": 704643328_u64,
                "timestamp": 1654218661_u64,
                "actEnergyDlvd": 1608426.912,
                "actEnergyRcvd": 4.923,
                "instantaneousDemand": 132.118,
                "activePower": 132.118,
                "apparentPower": 5328.778,
                "reactivePower": -5328.778,
                "pwrFactor": 0.025,
                "voltage": 246.377,
                "current": 43.257,
                "freq": 59.188
            }
        ])
    }

    fn inverter_production() -> JsonValue {
        serde_json::json!([
            {
                "serialNumber": INVERTER_SERIAL_1,
                "lastReportDate": 1654171836_u64,
                "devType": 1_u64,
                "lastReportWatts": 15.0,
                "maxReportWatts": 38.0
            },
            {
                "serialNumber": INVERTER_SERIAL_2,
                "lastReportDate": 1654171766_u64,
                "devType": 1_u64,
                "lastReportWatts": 5.0,
                "maxReportWatts": 5.0
            }
        ])
    }

    fn snapshot() -> EnphaseSnapshot {
        parse_snapshot(&meter_inventory(), &meter_readings()).unwrap()
    }

    fn authorized_runtime() -> (SmartHomeRuntime, AgentId) {
        let principal_id = AgentId::trusted("agent:energy");
        let mut runtime = SmartHomeRuntime::new();
        let _ = runtime.registry_mut().upsert_capability_grant(
            CapabilityGrant::for_all_smart_home(
                CapabilityGrantId::trusted("grant:enphase-test"),
                principal_id.clone(),
                PrivilegeTier::LowRisk,
                "test",
                1_000,
            )
            .with_expiry(20_000),
        );
        (runtime, principal_id)
    }

    fn identifier_policy(principal_id: &AgentId) -> DataGovernancePolicy {
        let mut policy = DataGovernancePolicy::default();
        policy
            .add_grant(
                DataUseGrant::new(
                    principal_id.clone(),
                    "enphase:122233344455:microinverters",
                    DataCategory::DeviceIdentifier,
                    DataOperation::Inspect,
                    DataDestination::LocalDevice,
                    DataPurpose::new("diagnose per-inverter solar production").unwrap(),
                    ConsentReceiptRef::new("consent://enphase/inverter-inspection-1").unwrap(),
                    DataRetention::Ephemeral,
                    1_000,
                    20_000,
                )
                .unwrap(),
            )
            .unwrap();
        policy
    }

    fn temp_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "smart-home-enphase-{name}-{}-{unique}",
            std::process::id()
        ))
    }

    fn installed_inverter_runtime(
        base_url: &str,
    ) -> (SmartHomeRuntime, AgentId, InstalledEnphaseGateway) {
        let (mut runtime, principal) = authorized_runtime();
        let key = EnphaseIdentifierKey::new(IDENTIFIER_KEY.to_vec()).unwrap();
        let values = SensitiveJson(inverter_production());
        let mut installed_snapshot = snapshot();
        installed_snapshot.inverters = parse_inverters(&values.0, &key, "122233344455").unwrap();
        let installed =
            install_snapshot(&mut runtime, &config(base_url), &installed_snapshot, 4_000).unwrap();
        (runtime, principal, installed)
    }

    #[test]
    fn production_config_requires_https_and_decimal_serial() {
        assert!(EnphaseConfig::new(
            BridgeId::trusted("bad"),
            "http://envoy.local",
            "122233344455",
            VaultRef::trusted("vault://token"),
        )
        .is_err());
        assert!(EnphaseConfig::new(
            BridgeId::trusted("bad"),
            "https://envoy.local/path",
            "122233344455",
            VaultRef::trusted("vault://token"),
        )
        .is_err());
        assert!(EnphaseConfig::new(
            BridgeId::trusted("bad"),
            "https://envoy.local",
            "serial-1",
            VaultRef::trusted("vault://token"),
        )
        .is_err());
        assert!(config("https://envoy.local").endpoint().is_ok());
    }

    #[test]
    fn parser_matches_native_eids_and_rejects_identity_drift() {
        let parsed = snapshot();
        assert_eq!(parsed.meters.len(), 2);
        assert_eq!(parsed.meters[0].eid, 704643328);
        assert_eq!(parsed.meters[1].measurement_type, "net-consumption");

        let mut readings = meter_readings();
        readings.as_array_mut().unwrap()[0]["eid"] = serde_json::json!(999_u64);
        assert!(parse_snapshot(&meter_inventory(), &readings)
            .unwrap_err()
            .to_string()
            .contains("missing reading"));
    }

    #[test]
    fn inverter_parser_pseudonymizes_serials_and_rejects_identity_drift() {
        let key = EnphaseIdentifierKey::new(IDENTIFIER_KEY.to_vec()).unwrap();
        let values = SensitiveJson(inverter_production());
        let parsed = parse_inverters(&values.0, &key, "122233344455").unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].pseudonym.len(), 32);
        assert_eq!(
            inverter_pseudonym(&key, "122233344455", INVERTER_SERIAL_1),
            inverter_pseudonym(&key, "122233344455", INVERTER_SERIAL_1)
        );
        assert!(parsed
            .iter()
            .all(|inverter| !inverter.pseudonym.contains(INVERTER_SERIAL_1)
                && !inverter.pseudonym.contains(INVERTER_SERIAL_2)));
        let other_key = EnphaseIdentifierKey::new(vec![0x6b; 32]).unwrap();
        let reparsed = parse_inverters(&values.0, &other_key, "122233344455").unwrap();
        assert_ne!(parsed[0].pseudonym, reparsed[0].pseudonym);

        let mut duplicate = inverter_production();
        duplicate.as_array_mut().unwrap()[1]["serialNumber"] =
            JsonValue::String(INVERTER_SERIAL_1.to_string());
        let error = parse_inverters(&duplicate, &key, "122233344455").unwrap_err();
        assert!(error.to_string().contains("duplicate microinverter"));
        assert!(!error.to_string().contains(INVERTER_SERIAL_1));

        let oversized = JsonValue::Array(vec![JsonValue::Null; MAX_INVERTERS + 1]);
        assert!(parse_inverters(&oversized, &key, "122233344455")
            .unwrap_err()
            .to_string()
            .contains("exceeds"));
    }

    #[test]
    fn identifier_keys_are_strict_and_redacted() {
        assert!(EnphaseIdentifierKey::new(vec![0x5a; 31]).is_err());
        let key = EnphaseIdentifierKey::new(IDENTIFIER_KEY.to_vec()).unwrap();
        assert_eq!(format!("{key:?}"), "EnphaseIdentifierKey([REDACTED])");
    }

    #[test]
    fn install_rejects_invalid_pseudonyms_before_runtime_mutation() {
        let (mut runtime, _) = authorized_runtime();
        let mut snapshot = snapshot();
        snapshot.inverters.push(EnphaseInverter {
            pseudonym: "short".to_string(),
            last_report_date: 1,
            device_type: 1,
            last_report_watts: 1.0,
            max_report_watts: 2.0,
        });
        assert!(install_snapshot(
            &mut runtime,
            &config("http://127.0.0.1:1"),
            &snapshot,
            5_000
        )
        .unwrap_err()
        .to_string()
        .contains("pseudonym"));
        assert!(runtime
            .registry()
            .bridge(&BridgeId::trusted("enphase.test"))
            .is_none());
    }

    #[test]
    fn token_and_client_debug_are_redacted() {
        let token = EnphaseAccessToken::new(TOKEN).unwrap();
        assert!(!format!("{token:?}").contains(TOKEN));
        let client = EnphaseClient::new(config("http://127.0.0.1:1"), token, FixedTransport)
            .unwrap()
            .with_identifier_key(EnphaseIdentifierKey::new(IDENTIFIER_KEY.to_vec()).unwrap());
        let debug = format!("{client:?}");
        assert!(!debug.contains(TOKEN));
        assert!(!debug.contains("5a5a5a"));
        assert!(debug.contains("[REDACTED]"));
    }

    struct FixedTransport;

    impl EnphaseTransport for FixedTransport {
        fn inspect(
            &mut self,
            _plans: &EnphaseRequestPlans,
            _token: &EnphaseAccessToken,
        ) -> Result<EnphaseSnapshot, EnphaseError> {
            Ok(snapshot())
        }
    }

    #[test]
    fn authorized_snapshot_installs_confirmed_meter_state() {
        let (mut runtime, principal) = authorized_runtime();
        let client = EnphaseClient::new(
            config("http://127.0.0.1:1"),
            EnphaseAccessToken::new(TOKEN).unwrap(),
            FixedTransport,
        )
        .unwrap();
        let installed = EnphaseRuntimeIntegration::new(client)
            .inspect_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();
        assert_eq!(installed.meter_entity_ids.len(), 2);
        assert_eq!(
            runtime
                .registry()
                .device(&installed.device_id)
                .unwrap()
                .health,
            Health::Degraded
        );
        let state = runtime
            .registry()
            .entity(&installed.meter_entity_ids[0])
            .unwrap()
            .state
            .as_ref()
            .unwrap();
        assert_eq!(state.confidence, StateConfidence::Confirmed);
        assert_eq!(state.source, StateSource::Poll);
    }

    struct CountingTransport {
        calls: usize,
    }

    impl EnphaseTransport for CountingTransport {
        fn inspect(
            &mut self,
            _plans: &EnphaseRequestPlans,
            _token: &EnphaseAccessToken,
        ) -> Result<EnphaseSnapshot, EnphaseError> {
            self.calls += 1;
            Ok(snapshot())
        }
    }

    #[test]
    fn unauthorized_read_stops_before_transport() {
        let principal = AgentId::trusted("agent:denied");
        let mut runtime = SmartHomeRuntime::new();
        let client = EnphaseClient::new(
            config("http://127.0.0.1:1"),
            EnphaseAccessToken::new(TOKEN).unwrap(),
            CountingTransport { calls: 0 },
        )
        .unwrap();
        let mut integration = EnphaseRuntimeIntegration::new(client);
        assert!(integration
            .inspect_and_install_authorized(&mut runtime, principal, 1_000)
            .is_err());
        assert_eq!(integration.client.transport.calls, 0);
    }

    #[test]
    fn inverter_inspection_without_identifier_consent_reaches_no_transport() {
        let (mut runtime, principal) = authorized_runtime();
        let client = EnphaseClient::new(
            config("http://127.0.0.1:1"),
            EnphaseAccessToken::new(TOKEN).unwrap(),
            CountingTransport { calls: 0 },
        )
        .unwrap()
        .with_identifier_key(EnphaseIdentifierKey::new(IDENTIFIER_KEY.to_vec()).unwrap());
        let mut integration = EnphaseRuntimeIntegration::new(client);
        assert!(matches!(
            integration.inspect_inverters_and_install_authorized(&mut runtime, principal, 5_000,),
            Err(EnphaseError::DataGovernanceDenied(
                DataGovernanceDenial::NoMatchingConsent
            ))
        ));
        assert_eq!(integration.client.transport.calls, 0);
    }

    struct RotationCountingTransport {
        calls: usize,
    }

    impl EnphaseTransport for RotationCountingTransport {
        fn inspect(
            &mut self,
            _plans: &EnphaseRequestPlans,
            _token: &EnphaseAccessToken,
        ) -> Result<EnphaseSnapshot, EnphaseError> {
            Ok(snapshot())
        }

        fn inspect_inverter_identity_rotation(
            &mut self,
            _plan: &LocalHttpRequestPlan,
            _token: &EnphaseAccessToken,
            source_key: &EnphaseIdentifierKey,
            destination_key: &EnphaseIdentifierKey,
            gateway_serial: &str,
        ) -> Result<Vec<EnphaseInverterIdentityRotation>, EnphaseError> {
            self.calls = self.calls.saturating_add(1);
            parse_inverter_identity_rotation(
                &inverter_production(),
                source_key,
                destination_key,
                gateway_serial,
            )
        }
    }

    #[test]
    fn rotation_without_identifier_consent_consumes_no_key_and_reaches_no_transport() {
        let (mut runtime, principal, _) = installed_inverter_runtime("http://127.0.0.1:1");
        let root = temp_root("rotation-consent-denial");
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let revision = store.save(&runtime, &[], 4_000).unwrap();
        let leases = InMemoryLeaseManager::new();
        let source_lease = leases
            .issue(LeasePayload::new(IDENTIFIER_KEY.to_vec()), 60_000)
            .unwrap();
        let destination_lease = leases
            .issue(LeasePayload::new(ROTATED_IDENTIFIER_KEY.to_vec()), 60_000)
            .unwrap();
        let client = EnphaseClient::new(
            config("http://127.0.0.1:1"),
            EnphaseAccessToken::new(TOKEN).unwrap(),
            RotationCountingTransport { calls: 0 },
        )
        .unwrap();
        let mut integration = EnphaseRuntimeIntegration::new(client);

        assert!(matches!(
            integration.rotate_inverter_identifier_key_authorized(
                &mut runtime,
                &store,
                &leases,
                EnphaseIdentifierKeyRotationRequest::new(
                    principal,
                    &source_lease,
                    &destination_lease,
                    5_000,
                    revision,
                ),
            ),
            Err(EnphaseError::DataGovernanceDenied(
                DataGovernanceDenial::NoMatchingConsent
            ))
        ));
        assert_eq!(integration.client.transport.calls, 0);
        assert_eq!(
            leases.consume(&source_lease).unwrap().as_bytes(),
            IDENTIFIER_KEY
        );
        assert_eq!(
            leases.consume(&destination_lease).unwrap().as_bytes(),
            ROTATED_IDENTIFIER_KEY
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rotation_parser_derives_exact_old_and_new_pseudonyms_without_serials() {
        let source_key = EnphaseIdentifierKey::new(IDENTIFIER_KEY.to_vec()).unwrap();
        let destination_key = EnphaseIdentifierKey::new(ROTATED_IDENTIFIER_KEY.to_vec()).unwrap();
        let values = SensitiveJson(inverter_production());

        let rotations = parse_inverter_identity_rotation(
            &values.0,
            &source_key,
            &destination_key,
            "122233344455",
        )
        .unwrap();

        assert_eq!(rotations.len(), 2);
        assert!(rotations.iter().all(|rotation| {
            rotation.source_pseudonym.len() == 32
                && rotation.destination_pseudonym.len() == 32
                && rotation.source_pseudonym != rotation.destination_pseudonym
                && !format!("{rotation:?}").contains(INVERTER_SERIAL_1)
                && !format!("{rotation:?}").contains(INVERTER_SERIAL_2)
        }));
    }

    #[test]
    fn rotation_rejects_stale_automation_identity_without_swapping_live_state() {
        let (mut runtime, principal, installed) = installed_inverter_runtime("http://127.0.0.1:1");
        let root = temp_root("rotation-stale-automation");
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let revision = store.save(&runtime, &[], 4_000).unwrap();
        let leases = InMemoryLeaseManager::new();
        let source_lease = leases
            .issue(LeasePayload::new(IDENTIFIER_KEY.to_vec()), 60_000)
            .unwrap();
        let destination_lease = leases
            .issue(LeasePayload::new(ROTATED_IDENTIFIER_KEY.to_vec()), 60_000)
            .unwrap();
        let definitions = vec![DurableAutomationDefinition::new(
            "automation:stale-inverter",
            true,
            serde_json::json!({
                "entity_id": installed.inverter_entity_ids[0].to_string()
            }),
        )
        .unwrap()];
        let client = EnphaseClient::new(
            config("http://127.0.0.1:1"),
            EnphaseAccessToken::new(TOKEN).unwrap(),
            RotationCountingTransport { calls: 0 },
        )
        .unwrap();
        let mut integration = EnphaseRuntimeIntegration::new(client)
            .with_data_governance(identifier_policy(&principal));

        let error = integration
            .rotate_inverter_identifier_key_authorized(
                &mut runtime,
                &store,
                &leases,
                EnphaseIdentifierKeyRotationRequest::new(
                    principal,
                    &source_lease,
                    &destination_lease,
                    5_000,
                    revision,
                )
                .with_automation_context(&definitions, None),
            )
            .unwrap_err();

        assert!(matches!(
            error,
            EnphaseError::RuntimeStore(RuntimeStoreError::Validation {
                field: "automation_definitions",
                ..
            })
        ));
        assert_eq!(integration.client.transport.calls, 1);
        assert!(runtime.registry().device(&installed.device_id).is_some());
        assert!(installed
            .inverter_entity_ids
            .iter()
            .all(|entity_id| runtime.registry().entity(entity_id).is_some()));
        let restored = store.load().unwrap().unwrap();
        assert!(restored
            .runtime
            .registry()
            .device(&installed.device_id)
            .is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn governed_rotation_consumes_two_keys_reads_once_and_persists_atomically() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base_url = format!("http://{address}");
        let (sender, receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let request = read_request(&mut stream);
            sender.send(request).unwrap();
            let body = serde_json::to_vec(&inverter_production()).unwrap();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            stream.write_all(&body).unwrap();
        });

        let (mut runtime, principal, installed) = installed_inverter_runtime(&base_url);
        let root = temp_root("identifier-rotation");
        let store = SmartHomeRuntimeStore::new(LocalFolderStorageBackend::new(root.clone()));
        let initial_revision = store.save(&runtime, &[], 4_000).unwrap();
        let leases = InMemoryLeaseManager::new();
        let source_lease = leases
            .issue(LeasePayload::new(IDENTIFIER_KEY.to_vec()), 60_000)
            .unwrap();
        let destination_lease = leases
            .issue(LeasePayload::new(ROTATED_IDENTIFIER_KEY.to_vec()), 60_000)
            .unwrap();
        let expected_namespace = gateway_identity_pseudonym(
            &EnphaseIdentifierKey::new(ROTATED_IDENTIFIER_KEY.to_vec()).unwrap(),
            "122233344455",
        );
        let expected_device_id = DeviceId::trusted(format!("enphase:{expected_namespace}"));
        let client = EnphaseClient::new(
            config(&base_url),
            EnphaseAccessToken::new(TOKEN).unwrap(),
            EnphaseLanTransport::default(),
        )
        .unwrap();
        let mut integration = EnphaseRuntimeIntegration::new(client)
            .with_data_governance(identifier_policy(&principal));

        let report = integration
            .rotate_inverter_identifier_key_authorized(
                &mut runtime,
                &store,
                &leases,
                EnphaseIdentifierKeyRotationRequest::new(
                    principal,
                    &source_lease,
                    &destination_lease,
                    5_000,
                    initial_revision,
                ),
            )
            .unwrap();
        server.join().unwrap();

        assert_eq!(report.rotated_inverters, 2);
        assert_eq!(report.migration.migrated_devices, 1);
        assert_eq!(report.migration.migrated_entities, 4);
        let requests = receiver.try_iter().collect::<Vec<_>>();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /api/v1/production/inverters HTTP/1.1\r\n"));
        assert!(requests[0].contains(&format!("Authorization: Bearer {TOKEN}\r\n")));
        assert_eq!(
            leases.lookup(&source_lease).unwrap().status_at(0),
            LeaseStatus::Revoked
        );
        assert_eq!(
            leases.lookup(&destination_lease).unwrap().status_at(0),
            LeaseStatus::Revoked
        );
        assert!(runtime.registry().device(&installed.device_id).is_none());
        assert!(installed
            .inverter_entity_ids
            .iter()
            .all(|entity_id| runtime.registry().entity(entity_id).is_none()));
        let replacement = runtime.registry().device(&expected_device_id).unwrap();
        assert_eq!(replacement.entity_ids.len(), 4);
        assert_eq!(
            metadata_value(&replacement.metadata, "enphase.identifier_namespace"),
            Some(expected_namespace.as_str())
        );
        let debug = format!("{:?}", runtime.registry());
        assert!(!debug.contains(INVERTER_SERIAL_1));
        assert!(!debug.contains(INVERTER_SERIAL_2));

        let restored = store.load().unwrap().unwrap();
        assert_eq!(restored.revision, report.revision);
        assert!(restored
            .runtime
            .registry()
            .device(&expected_device_id)
            .is_some());
        assert!(restored
            .runtime
            .registry()
            .device(&installed.device_id)
            .is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bounded_reader_rejects_oversized_payloads() {
        let mut bytes = &b"abcdef"[..];
        assert!(matches!(
            read_bounded(&mut bytes, 4),
            Err(EnphaseError::ResponseTooLarge { limit: 4 })
        ));
    }

    #[test]
    fn loopback_transport_sends_exact_bearer_requests() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [meter_inventory(), meter_readings()] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let request = read_request(&mut stream);
                requests.push(request);
                let body = serde_json::to_vec(&response).unwrap();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
            sender.send(requests).unwrap();
        });

        let config = config(&format!("http://{address}"));
        let token_ref_text = config.token_ref.as_str().to_string();
        let mut client = EnphaseClient::new(
            config,
            EnphaseAccessToken::new(TOKEN).unwrap(),
            EnphaseLanTransport::default(),
        )
        .unwrap();
        let observed = client.inspect().unwrap();
        assert_eq!(observed.meters.len(), 2);
        server.join().unwrap();
        let requests = receiver.recv().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].starts_with("GET /ivp/meters HTTP/1.1\r\n"));
        assert!(requests[1].starts_with("GET /ivp/meters/readings HTTP/1.1\r\n"));
        for request in requests {
            assert!(request.contains(&format!("Authorization: Bearer {TOKEN}\r\n")));
            assert!(request.contains("Accept: application/json\r\n"));
            assert!(!request.contains(&token_ref_text));
        }
    }

    #[test]
    fn governed_loopback_inverter_inspection_installs_only_pseudonymous_identity() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        let server = thread::spawn(move || {
            let mut requests = Vec::new();
            for response in [meter_inventory(), meter_readings(), inverter_production()] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                requests.push(read_request(&mut stream));
                let body = serde_json::to_vec(&response).unwrap();
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                stream.write_all(head.as_bytes()).unwrap();
                stream.write_all(&body).unwrap();
            }
            sender.send(requests).unwrap();
        });

        let config = config(&format!("http://{address}"));
        let token_ref_text = config.token_ref.as_str().to_string();
        let client = EnphaseClient::new(
            config,
            EnphaseAccessToken::new(TOKEN).unwrap(),
            EnphaseLanTransport::default(),
        )
        .unwrap()
        .with_identifier_key(EnphaseIdentifierKey::new(IDENTIFIER_KEY.to_vec()).unwrap());
        let (mut runtime, principal) = authorized_runtime();
        let mut integration = EnphaseRuntimeIntegration::new(client)
            .with_data_governance(identifier_policy(&principal));
        let installed = integration
            .inspect_inverters_and_install_authorized(&mut runtime, principal, 5_000)
            .unwrap();

        server.join().unwrap();
        let requests = receiver.recv().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].starts_with("GET /ivp/meters HTTP/1.1\r\n"));
        assert!(requests[1].starts_with("GET /ivp/meters/readings HTTP/1.1\r\n"));
        assert!(requests[2].starts_with("GET /api/v1/production/inverters HTTP/1.1\r\n"));
        for request in requests {
            assert!(request.contains(&format!("Authorization: Bearer {TOKEN}\r\n")));
            assert!(!request.contains(&token_ref_text));
            assert!(!request.contains(INVERTER_SERIAL_1));
            assert!(!request.contains(INVERTER_SERIAL_2));
        }

        assert_eq!(installed.inverter_entity_ids.len(), 2);
        let expected = inverter_pseudonym(
            &EnphaseIdentifierKey::new(IDENTIFIER_KEY.to_vec()).unwrap(),
            "122233344455",
            INVERTER_SERIAL_1,
        );
        assert!(installed
            .inverter_entity_ids
            .iter()
            .any(|entity_id| entity_id.as_str().ends_with(&expected)));
        for entity_id in &installed.inverter_entity_ids {
            let debug = format!("{:?}", runtime.registry().entity(entity_id).unwrap());
            assert!(!debug.contains(INVERTER_SERIAL_1));
            assert!(!debug.contains(INVERTER_SERIAL_2));
            assert!(debug.contains("keyed_pseudonym"));
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 1024];
        loop {
            let count = stream.read(&mut chunk).unwrap();
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
            if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8(bytes).unwrap()
    }
}
