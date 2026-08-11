//! Authorized production host for bounded Frigate camera snapshots.

#![forbid(unsafe_code)]

use coding_adventures_csprng::random_array;
use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::SealedStore;
use coding_adventures_zeroize::Zeroizing;
use serde::{Deserialize, Deserializer, Serialize};
use smart_home_camera_media::{
    CameraMediaAccessRequest, CameraMediaClock, CameraMediaConnectionTarget,
    CameraMediaCredentialRegistry, CameraMediaDelivery, CameraMediaError, CameraMediaExecution,
    CameraMediaExecutionError, CameraMediaExecutionResult, CameraMediaExecutor, CameraMediaKind,
    CameraMediaNonceError, CameraMediaNonceSource, CameraMediaPolicy, CameraMediaPrincipalSource,
    CameraMediaService,
};
use smart_home_core::{
    BridgeId, CapabilityId, CapabilityMode, EntityId, EntityKind, IntegrationId, ProtocolFamily,
    VaultRef,
};
use smart_home_frigate_integration::{
    snapshot_uri, FrigateConfig, FrigateCredentials, FrigateError, FrigateLanTransport,
    INTEGRATION_ID, MAX_CREDENTIAL_FIELD_BYTES, PROTOCOL_ID,
};
use smart_home_runtime::SmartHomeRuntime;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tls_platform::{TlsConfig, TlsConnector};
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const FRIGATE_VAULT_NAMESPACE: &str = "smart_home.frigate.credentials";
pub const FRIGATE_VAULT_REF_PREFIX: &str = "vault://smart-home/frigate/";
pub const MAX_CREDENTIAL_PAYLOAD_BYTES: usize = MAX_CREDENTIAL_FIELD_BYTES * 2 + 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrigateCredentialSourceError;

pub trait FrigateCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, FrigateCredentialSourceError>;
}

pub struct FrigateSealedStoreCredentialSource {
    vault: Arc<SealedStore>,
}

impl FrigateSealedStoreCredentialSource {
    pub fn new(vault: Arc<SealedStore>) -> Self {
        Self { vault }
    }
}

impl FrigateCredentialSource for FrigateSealedStoreCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, FrigateCredentialSourceError> {
        let key = frigate_vault_record_key(vault_ref).ok_or(FrigateCredentialSourceError)?;
        let record = self
            .vault
            .get(FRIGATE_VAULT_NAMESPACE, key)
            .map_err(|_| FrigateCredentialSourceError)?
            .ok_or(FrigateCredentialSourceError)?;
        Ok(LeasePayload::new(record.plaintext.into_inner()))
    }
}

pub fn frigate_vault_record_key(vault_ref: &VaultRef) -> Option<&str> {
    vault_ref
        .as_str()
        .strip_prefix(FRIGATE_VAULT_REF_PREFIX)
        .filter(|key| !key.is_empty())
}

#[derive(Clone)]
pub struct FrigateSnapshotEndpoint {
    bridge_id: BridgeId,
    connection_target: CameraMediaConnectionTarget,
}

impl FrigateSnapshotEndpoint {
    pub fn new(bridge_id: BridgeId, connection_target: CameraMediaConnectionTarget) -> Self {
        Self {
            bridge_id,
            connection_target,
        }
    }
}

impl fmt::Debug for FrigateSnapshotEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FrigateSnapshotEndpoint")
            .field("bridge_id", &self.bridge_id)
            .field("connection_target", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrigateSnapshotRequest {
    pub entity_id: EntityId,
    pub purpose: String,
    pub ttl_ms: u64,
}

impl FrigateSnapshotRequest {
    pub fn new(entity_id: EntityId, purpose: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            entity_id,
            purpose: purpose.into(),
            ttl_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrigateSnapshotHostError {
    InvalidRequest,
    InvalidTarget,
    MissingCredentialReference,
    CredentialResolutionRejected,
    InvalidCredentialPayload,
    CredentialRegistrationRejected,
    CredentialRemovalFailed,
    EndpointAlreadyRegistered,
    EndpointRegistrationRejected,
    EndpointRemovalFailed,
    Media(CameraMediaError),
}

impl fmt::Display for FrigateSnapshotHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "invalid Frigate snapshot request",
            Self::InvalidTarget => "snapshot target is not an installed Frigate camera",
            Self::MissingCredentialReference => "Frigate bridge has no credential reference",
            Self::CredentialResolutionRejected => "Frigate credential lookup was rejected",
            Self::InvalidCredentialPayload => "Frigate credential payload is invalid",
            Self::CredentialRegistrationRejected => {
                "Frigate snapshot credentials could not be registered"
            }
            Self::CredentialRemovalFailed => "Frigate snapshot credentials could not be removed",
            Self::EndpointAlreadyRegistered => "Frigate snapshot endpoint is already registered",
            Self::EndpointRegistrationRejected => "Frigate snapshot endpoint registration failed",
            Self::EndpointRemovalFailed => "Frigate snapshot endpoint removal failed",
            Self::Media(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for FrigateSnapshotHostError {}

impl From<CameraMediaError> for FrigateSnapshotHostError {
    fn from(value: CameraMediaError) -> Self {
        Self::Media(value)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialEnvelope {
    schema_version: u64,
    #[serde(deserialize_with = "deserialize_secret_string")]
    username: Zeroizing<String>,
    #[serde(deserialize_with = "deserialize_secret_string")]
    password: Zeroizing<String>,
}

#[derive(Serialize)]
struct BorrowedCredentialEnvelope<'a> {
    schema_version: u64,
    username: &'a str,
    password: &'a str,
}

pub fn encode_frigate_credentials(
    username: &str,
    password: &str,
) -> Result<LeasePayload, FrigateSnapshotHostError> {
    validate_credential_fields(username, password)?;
    let bytes = Zeroizing::new(
        serde_json::to_vec(&BorrowedCredentialEnvelope {
            schema_version: 1,
            username,
            password,
        })
        .map_err(|_| FrigateSnapshotHostError::InvalidCredentialPayload)?,
    );
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(FrigateSnapshotHostError::InvalidCredentialPayload);
    }
    Ok(LeasePayload::new(bytes.into_inner()))
}

pub struct FrigateExecutorCredentials {
    config: FrigateConfig,
    camera_name: String,
    credentials: FrigateCredentials,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrigateExecutorCredentialError;

pub struct FrigateSnapshotExecutor {
    transport: FrigateLanTransport,
    credentials: BTreeMap<EntityId, FrigateExecutorCredentials>,
}

impl Default for FrigateSnapshotExecutor {
    fn default() -> Self {
        Self::new(FrigateLanTransport::default())
    }
}

impl FrigateSnapshotExecutor {
    pub fn new(transport: FrigateLanTransport) -> Self {
        Self {
            transport,
            credentials: BTreeMap::new(),
        }
    }

    pub fn with_connector(connector: Box<dyn TlsConnector>, tls_config: TlsConfig) -> Self {
        Self::new(FrigateLanTransport::new(connector, tls_config))
    }
}

impl CameraMediaExecutor for FrigateSnapshotExecutor {
    type Stream = ();

    fn deliver(
        &mut self,
        execution: CameraMediaExecution<'_>,
    ) -> Result<CameraMediaExecutionResult<Self::Stream>, CameraMediaExecutionError> {
        if execution.kind() != CameraMediaKind::Snapshot {
            return Err(CameraMediaExecutionError::Rejected);
        }
        let target = execution
            .connection_target()
            .ok_or(CameraMediaExecutionError::Rejected)?;
        let credentials = self
            .credentials
            .get(execution.entity_id())
            .ok_or(CameraMediaExecutionError::Rejected)?;
        let expected_uri = snapshot_uri(&credentials.config, &credentials.camera_name)
            .map_err(map_execution_error)?;
        if execution.endpoint_uri() != expected_uri {
            return Err(CameraMediaExecutionError::Rejected);
        }
        self.transport
            .fetch_snapshot_pinned(
                &credentials.config,
                &credentials.credentials,
                &credentials.camera_name,
                target.canonical_host(),
                target.pinned_address(),
                execution.max_snapshot_bytes(),
            )
            .map(CameraMediaExecutionResult::snapshot)
            .map_err(map_execution_error)
    }

    fn close_stream(
        &mut self,
        _stream: &mut Self::Stream,
    ) -> Result<(), CameraMediaExecutionError> {
        Err(CameraMediaExecutionError::Rejected)
    }
}

impl CameraMediaCredentialRegistry for FrigateSnapshotExecutor {
    type Credentials = FrigateExecutorCredentials;
    type Error = FrigateExecutorCredentialError;

    fn register_credentials(
        &mut self,
        entity_id: EntityId,
        credentials: Self::Credentials,
    ) -> Result<(), Self::Error> {
        if self.credentials.contains_key(&entity_id) {
            return Err(FrigateExecutorCredentialError);
        }
        self.credentials.insert(entity_id, credentials);
        Ok(())
    }

    fn unregister_credentials(&mut self, entity_id: &EntityId) -> bool {
        self.credentials.remove(entity_id).is_some()
    }
}

fn map_execution_error(error: FrigateError) -> CameraMediaExecutionError {
    match error {
        FrigateError::ResponseTooLarge { .. } => CameraMediaExecutionError::ResourceLimit,
        FrigateError::HttpStatus {
            status: 401 | 403, ..
        } => CameraMediaExecutionError::Rejected,
        FrigateError::Io(_) | FrigateError::Tls(_) => CameraMediaExecutionError::Unavailable,
        _ => CameraMediaExecutionError::Protocol,
    }
}

pub struct SystemCameraMediaClock;

impl CameraMediaClock for SystemCameraMediaClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX)
    }
}

pub struct OsCameraMediaNonceSource;

impl CameraMediaNonceSource for OsCameraMediaNonceSource {
    fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), CameraMediaNonceError> {
        *output = random_array().map_err(|_| CameraMediaNonceError)?;
        Ok(())
    }
}

pub struct FrigateSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor
        + CameraMediaCredentialRegistry<
            Credentials = FrigateExecutorCredentials,
            Error = FrigateExecutorCredentialError,
        >,
    Credentials: FrigateCredentialSource,
{
    media: CameraMediaService<Clock, Nonce, Principals, Executor>,
    credentials: Credentials,
    endpoint: FrigateSnapshotEndpoint,
    max_snapshot_ttl_ms: u64,
}

impl<Clock, Nonce, Principals, Executor, Credentials>
    FrigateSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor
        + CameraMediaCredentialRegistry<
            Credentials = FrigateExecutorCredentials,
            Error = FrigateExecutorCredentialError,
        >,
    Credentials: FrigateCredentialSource,
{
    pub fn new(
        policy: CameraMediaPolicy,
        clock: Clock,
        nonce_source: Nonce,
        principal_source: Principals,
        executor: Executor,
        credentials: Credentials,
        endpoint: FrigateSnapshotEndpoint,
    ) -> Self {
        let max_snapshot_ttl_ms = policy.max_snapshot_ttl_ms;
        Self {
            media: CameraMediaService::new(policy, clock, nonce_source, principal_source, executor),
            credentials,
            endpoint,
            max_snapshot_ttl_ms,
        }
    }

    pub fn deliver_snapshot(
        &mut self,
        runtime: &SmartHomeRuntime,
        request: FrigateSnapshotRequest,
    ) -> Result<CameraMediaDelivery, FrigateSnapshotHostError> {
        self.validate_request(&request)?;
        self.media
            .authorize_access(runtime, &request.entity_id, CameraMediaKind::Snapshot)?;
        if self
            .media
            .has_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
        {
            return Err(FrigateSnapshotHostError::EndpointAlreadyRegistered);
        }
        let target = installed_target(runtime, &request.entity_id, &self.endpoint)?;
        let payload = self
            .credentials
            .resolve(&target.credential_ref)
            .map_err(|_| FrigateSnapshotHostError::CredentialResolutionRejected)?;
        let credentials = decode_credentials(payload.as_bytes())?;
        drop(payload);
        let executor_credentials = FrigateExecutorCredentials {
            config: target.config,
            camera_name: target.camera_name,
            credentials,
        };

        self.media
            .register_pinned_endpoint(
                request.entity_id.clone(),
                CameraMediaKind::Snapshot,
                &target.snapshot_uri,
                self.endpoint.connection_target.clone(),
            )
            .map_err(|_| FrigateSnapshotHostError::EndpointRegistrationRejected)?;
        if self
            .media
            .register_executor_credentials(request.entity_id.clone(), executor_credentials)
            .is_err()
        {
            if !self
                .media
                .unregister_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
            {
                return Err(FrigateSnapshotHostError::EndpointRemovalFailed);
            }
            return Err(FrigateSnapshotHostError::CredentialRegistrationRejected);
        }

        let delivery = (|| {
            let lease = self.media.issue_lease(
                runtime,
                CameraMediaAccessRequest::new(
                    request.entity_id.clone(),
                    CameraMediaKind::Snapshot,
                    request.purpose,
                    request.ttl_ms,
                ),
            )?;
            self.media
                .deliver_lease(runtime, &lease.lease_id)
                .map_err(Into::into)
        })();
        let credentials_removed = self
            .media
            .unregister_executor_credentials(&request.entity_id);
        let endpoint_removed = self
            .media
            .unregister_endpoint(&request.entity_id, CameraMediaKind::Snapshot);
        if !credentials_removed {
            return Err(FrigateSnapshotHostError::CredentialRemovalFailed);
        }
        if !endpoint_removed {
            return Err(FrigateSnapshotHostError::EndpointRemovalFailed);
        }
        delivery
    }

    pub fn media_snapshot(&self) -> smart_home_camera_media::CameraMediaBrokerSnapshot {
        self.media.snapshot()
    }

    fn validate_request(
        &self,
        request: &FrigateSnapshotRequest,
    ) -> Result<(), FrigateSnapshotHostError> {
        if request.purpose.trim().is_empty()
            || request.ttl_ms == 0
            || request.ttl_ms > self.max_snapshot_ttl_ms
        {
            return Err(FrigateSnapshotHostError::InvalidRequest);
        }
        Ok(())
    }
}

impl<Principals, Credentials>
    FrigateSnapshotHost<
        SystemCameraMediaClock,
        OsCameraMediaNonceSource,
        Principals,
        FrigateSnapshotExecutor,
        Credentials,
    >
where
    Principals: CameraMediaPrincipalSource,
    Credentials: FrigateCredentialSource,
{
    pub fn production(
        principal_source: Principals,
        credentials: Credentials,
        endpoint: FrigateSnapshotEndpoint,
    ) -> Self {
        Self::new(
            CameraMediaPolicy::default(),
            SystemCameraMediaClock,
            OsCameraMediaNonceSource,
            principal_source,
            FrigateSnapshotExecutor::default(),
            credentials,
            endpoint,
        )
    }
}

struct InstalledTarget {
    credential_ref: VaultRef,
    config: FrigateConfig,
    camera_name: String,
    snapshot_uri: String,
}

fn installed_target(
    runtime: &SmartHomeRuntime,
    entity_id: &EntityId,
    endpoint: &FrigateSnapshotEndpoint,
) -> Result<InstalledTarget, FrigateSnapshotHostError> {
    let registry = runtime.registry();
    let entity = registry
        .entity(entity_id)
        .ok_or(FrigateSnapshotHostError::InvalidTarget)?;
    let device = registry
        .device(&entity.device_id)
        .ok_or(FrigateSnapshotHostError::InvalidTarget)?;
    let bridge = registry
        .bridge(&device.bridge_id)
        .ok_or(FrigateSnapshotHostError::InvalidTarget)?;
    let camera_name = device
        .identifiers
        .iter()
        .find(|identifier| {
            identifier.kind == "camera_name"
                && matches!(
                    &identifier.family,
                    ProtocolFamily::Vendor(family) if family == PROTOCOL_ID
                )
        })
        .map(|identifier| identifier.value.clone())
        .filter(|name| !name.trim().is_empty())
        .ok_or(FrigateSnapshotHostError::InvalidTarget)?;
    let snapshot_capability = entity.capabilities.iter().any(|capability| {
        capability.capability_id == CapabilityId::trusted("camera.snapshot")
            && capability.mode == CapabilityMode::Command
    });
    let exact_metadata = device
        .metadata
        .iter()
        .any(|metadata| metadata.key == "frigate.camera_name" && metadata.value == camera_name);
    let expected_entity_id =
        EntityId::trusted(format!("frigate:{}:camera", stable_component(&camera_name)));
    if entity.kind != EntityKind::Camera
        || *entity_id != expected_entity_id
        || !device.entity_ids.contains(entity_id)
        || bridge.bridge_id != endpoint.bridge_id
        || bridge.integration_id != IntegrationId::trusted(INTEGRATION_ID)
        || !snapshot_capability
        || !exact_metadata
    {
        return Err(FrigateSnapshotHostError::InvalidTarget);
    }
    let credential_ref = bridge
        .auth_ref
        .clone()
        .ok_or(FrigateSnapshotHostError::MissingCredentialReference)?;
    let base_url = bridge
        .address
        .clone()
        .ok_or(FrigateSnapshotHostError::InvalidTarget)?;
    let config = FrigateConfig::new(bridge.bridge_id.clone(), base_url, credential_ref.clone())
        .map_err(|_| FrigateSnapshotHostError::InvalidTarget)?;
    validate_connection_target(&config.base_url, &endpoint.connection_target)?;
    let snapshot_uri =
        snapshot_uri(&config, &camera_name).map_err(|_| FrigateSnapshotHostError::InvalidTarget)?;
    Ok(InstalledTarget {
        credential_ref,
        config,
        camera_name,
        snapshot_uri,
    })
}

fn validate_connection_target(
    base_url: &str,
    connection_target: &CameraMediaConnectionTarget,
) -> Result<(), FrigateSnapshotHostError> {
    let parsed = Url::parse(base_url).map_err(|_| FrigateSnapshotHostError::InvalidTarget)?;
    let host = parsed
        .host
        .as_deref()
        .ok_or(FrigateSnapshotHostError::InvalidTarget)?;
    if !host.eq_ignore_ascii_case(connection_target.canonical_host())
        || parsed.effective_port() != Some(connection_target.pinned_address().port())
        || (parsed.scheme == "http"
            && (!is_loopback_host(host) || !connection_target.pinned_address().ip().is_loopback()))
    {
        return Err(FrigateSnapshotHostError::InvalidTarget);
    }
    Ok(())
}

fn decode_credentials(bytes: &[u8]) -> Result<FrigateCredentials, FrigateSnapshotHostError> {
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(FrigateSnapshotHostError::InvalidCredentialPayload);
    }
    let envelope: CredentialEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| FrigateSnapshotHostError::InvalidCredentialPayload)?;
    if envelope.schema_version != 1 {
        return Err(FrigateSnapshotHostError::InvalidCredentialPayload);
    }
    validate_credential_fields(&envelope.username, &envelope.password)?;
    FrigateCredentials::new(
        envelope.username.into_inner(),
        envelope.password.into_inner(),
    )
    .map_err(|_| FrigateSnapshotHostError::InvalidCredentialPayload)
}

fn deserialize_secret_string<'de, D>(deserializer: D) -> Result<Zeroizing<String>, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(Zeroizing::new)
}

fn validate_credential_fields(
    username: &str,
    password: &str,
) -> Result<(), FrigateSnapshotHostError> {
    if username.trim().is_empty()
        || password.is_empty()
        || username.len() > MAX_CREDENTIAL_FIELD_BYTES
        || password.len() > MAX_CREDENTIAL_FIELD_BYTES
        || username.contains(['\r', '\n', '\0'])
        || password.contains(['\r', '\n', '\0'])
    {
        return Err(FrigateSnapshotHostError::InvalidCredentialPayload);
    }
    Ok(())
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

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("127.")
}
