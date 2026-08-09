//! Authorized production host for bounded Synology camera snapshots.

#![forbid(unsafe_code)]

use coding_adventures_csprng::random_array;
use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::SealedStore;
use coding_adventures_zeroize::Zeroizing;
use serde::{Deserialize, Deserializer, Serialize};
use smart_home_camera_media::{
    CameraMediaAccessRequest, CameraMediaClock, CameraMediaConnectionTarget, CameraMediaDelivery,
    CameraMediaError, CameraMediaExecutor, CameraMediaKind, CameraMediaNonceError,
    CameraMediaNonceSource, CameraMediaPolicy, CameraMediaPrincipalSource, CameraMediaService,
};
use smart_home_camera_media_http_executor::CameraMediaHttpExecutor;
use smart_home_core::{BridgeId, EntityId, EntityKind, IntegrationId, ProtocolFamily, VaultRef};
use smart_home_runtime::SmartHomeRuntime;
use smart_home_synology_surveillance_integration::{
    SynologyConfig, SynologyCredentials, SynologyLanTransport, SynologySnapshotSession,
    INTEGRATION_ID, MAX_CREDENTIAL_FIELD_BYTES, PROTOCOL_ID,
};
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const SYNOLOGY_VAULT_NAMESPACE: &str = "smart_home.synology_surveillance.credentials";
pub const SYNOLOGY_VAULT_REF_PREFIX: &str = "vault://smart-home/synology-surveillance/";
pub const MAX_CREDENTIAL_PAYLOAD_BYTES: usize = MAX_CREDENTIAL_FIELD_BYTES * 2 + 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SynologyCredentialSourceError;

pub trait SynologyCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, SynologyCredentialSourceError>;
}

pub struct SynologySealedStoreCredentialSource {
    vault: Arc<SealedStore>,
}

impl SynologySealedStoreCredentialSource {
    pub fn new(vault: Arc<SealedStore>) -> Self {
        Self { vault }
    }
}

impl SynologyCredentialSource for SynologySealedStoreCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, SynologyCredentialSourceError> {
        let key = synology_vault_record_key(vault_ref).ok_or(SynologyCredentialSourceError)?;
        let record = self
            .vault
            .get(SYNOLOGY_VAULT_NAMESPACE, key)
            .map_err(|_| SynologyCredentialSourceError)?
            .ok_or(SynologyCredentialSourceError)?;
        Ok(LeasePayload::new(record.plaintext.into_inner()))
    }
}

pub fn synology_vault_record_key(vault_ref: &VaultRef) -> Option<&str> {
    vault_ref
        .as_str()
        .strip_prefix(SYNOLOGY_VAULT_REF_PREFIX)
        .filter(|key| !key.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SynologySnapshotSessionSourceError;

pub trait SynologySnapshotSessionSource {
    type Session;

    /// Setup failures must close any session opened before returning.
    fn open(
        &mut self,
        config: &SynologyConfig,
        credentials: &SynologyCredentials,
        camera_id: u64,
    ) -> Result<Self::Session, SynologySnapshotSessionSourceError>;

    fn endpoint_uri<'a>(&self, session: &'a Self::Session) -> &'a str;

    fn close(
        &mut self,
        config: &SynologyConfig,
        session: Self::Session,
    ) -> Result<(), SynologySnapshotSessionSourceError>;
}

#[derive(Default)]
pub struct SynologyLanSnapshotSessionSource {
    transport: SynologyLanTransport,
}

impl SynologyLanSnapshotSessionSource {
    pub fn new(transport: SynologyLanTransport) -> Self {
        Self { transport }
    }
}

impl SynologySnapshotSessionSource for SynologyLanSnapshotSessionSource {
    type Session = SynologySnapshotSession;

    fn open(
        &mut self,
        config: &SynologyConfig,
        credentials: &SynologyCredentials,
        camera_id: u64,
    ) -> Result<Self::Session, SynologySnapshotSessionSourceError> {
        self.transport
            .open_snapshot_session(config, credentials, camera_id)
            .map_err(|_| SynologySnapshotSessionSourceError)
    }

    fn endpoint_uri<'a>(&self, session: &'a Self::Session) -> &'a str {
        session.endpoint_uri()
    }

    fn close(
        &mut self,
        config: &SynologyConfig,
        session: Self::Session,
    ) -> Result<(), SynologySnapshotSessionSourceError> {
        self.transport
            .close_snapshot_session(config, session)
            .map_err(|_| SynologySnapshotSessionSourceError)
    }
}

#[derive(Clone)]
pub struct SynologySnapshotEndpoint {
    bridge_id: BridgeId,
    connection_target: CameraMediaConnectionTarget,
}

impl SynologySnapshotEndpoint {
    pub fn new(bridge_id: BridgeId, connection_target: CameraMediaConnectionTarget) -> Self {
        Self {
            bridge_id,
            connection_target,
        }
    }
}

impl fmt::Debug for SynologySnapshotEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SynologySnapshotEndpoint")
            .field("bridge_id", &self.bridge_id)
            .field("connection_target", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynologySnapshotRequest {
    pub entity_id: EntityId,
    pub purpose: String,
    pub ttl_ms: u64,
}

impl SynologySnapshotRequest {
    pub fn new(entity_id: EntityId, purpose: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            entity_id,
            purpose: purpose.into(),
            ttl_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynologySnapshotHostError {
    InvalidRequest,
    InvalidTarget,
    MissingCredentialReference,
    CredentialResolutionRejected,
    InvalidCredentialPayload,
    SessionSetupRejected,
    SessionLogoutFailed,
    EndpointAlreadyRegistered,
    EndpointRegistrationRejected,
    EndpointRemovalFailed,
    Media(CameraMediaError),
}

impl fmt::Display for SynologySnapshotHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "invalid Synology snapshot request",
            Self::InvalidTarget => "snapshot target is not an installed Synology camera",
            Self::MissingCredentialReference => "Synology bridge has no credential reference",
            Self::CredentialResolutionRejected => "Synology credential lookup was rejected",
            Self::InvalidCredentialPayload => "Synology credential payload is invalid",
            Self::SessionSetupRejected => "Synology snapshot session setup failed",
            Self::SessionLogoutFailed => "Synology snapshot session logout failed",
            Self::EndpointAlreadyRegistered => "Synology snapshot endpoint is already registered",
            Self::EndpointRegistrationRejected => "Synology snapshot endpoint registration failed",
            Self::EndpointRemovalFailed => "Synology snapshot endpoint removal failed",
            Self::Media(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for SynologySnapshotHostError {}

impl From<CameraMediaError> for SynologySnapshotHostError {
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

pub fn encode_synology_credentials(
    username: &str,
    password: &str,
) -> Result<LeasePayload, SynologySnapshotHostError> {
    validate_credential_fields(username, password)?;
    let bytes = Zeroizing::new(
        serde_json::to_vec(&BorrowedCredentialEnvelope {
            schema_version: 1,
            username,
            password,
        })
        .map_err(|_| SynologySnapshotHostError::InvalidCredentialPayload)?,
    );
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(SynologySnapshotHostError::InvalidCredentialPayload);
    }
    Ok(LeasePayload::new(bytes.into_inner()))
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

pub struct SynologySnapshotResources<Credentials, Sessions> {
    credentials: Credentials,
    sessions: Sessions,
    endpoint: SynologySnapshotEndpoint,
}

impl<Credentials, Sessions> SynologySnapshotResources<Credentials, Sessions> {
    pub fn new(
        credentials: Credentials,
        sessions: Sessions,
        endpoint: SynologySnapshotEndpoint,
    ) -> Self {
        Self {
            credentials,
            sessions,
            endpoint,
        }
    }
}

pub struct SynologySnapshotHost<Clock, Nonce, Principals, Executor, Credentials, Sessions>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
    Credentials: SynologyCredentialSource,
    Sessions: SynologySnapshotSessionSource,
{
    media: CameraMediaService<Clock, Nonce, Principals, Executor>,
    credentials: Credentials,
    sessions: Sessions,
    endpoint: SynologySnapshotEndpoint,
    max_snapshot_ttl_ms: u64,
}

impl<Clock, Nonce, Principals, Executor, Credentials, Sessions>
    SynologySnapshotHost<Clock, Nonce, Principals, Executor, Credentials, Sessions>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
    Credentials: SynologyCredentialSource,
    Sessions: SynologySnapshotSessionSource,
{
    pub fn new(
        policy: CameraMediaPolicy,
        clock: Clock,
        nonce_source: Nonce,
        principal_source: Principals,
        executor: Executor,
        resources: SynologySnapshotResources<Credentials, Sessions>,
    ) -> Self {
        let max_snapshot_ttl_ms = policy.max_snapshot_ttl_ms;
        Self {
            media: CameraMediaService::new(policy, clock, nonce_source, principal_source, executor),
            credentials: resources.credentials,
            sessions: resources.sessions,
            endpoint: resources.endpoint,
            max_snapshot_ttl_ms,
        }
    }

    pub fn deliver_snapshot(
        &mut self,
        runtime: &SmartHomeRuntime,
        request: SynologySnapshotRequest,
    ) -> Result<CameraMediaDelivery, SynologySnapshotHostError> {
        self.validate_request(&request)?;
        self.media
            .authorize_access(runtime, &request.entity_id, CameraMediaKind::Snapshot)?;
        if self
            .media
            .has_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
        {
            return Err(SynologySnapshotHostError::EndpointAlreadyRegistered);
        }
        let target = installed_target(runtime, &request.entity_id, &self.endpoint)?;
        let config = SynologyConfig::new(
            target.bridge_id.clone(),
            target.base_url.clone(),
            target.credential_ref.clone(),
        )
        .map_err(|_| SynologySnapshotHostError::InvalidTarget)?;
        let payload = self
            .credentials
            .resolve(&target.credential_ref)
            .map_err(|_| SynologySnapshotHostError::CredentialResolutionRejected)?;
        let credentials = decode_credentials(payload.as_bytes())?;
        drop(payload);
        let session = self
            .sessions
            .open(&config, &credentials, target.camera_id)
            .map_err(|_| SynologySnapshotHostError::SessionSetupRejected)?;
        drop(credentials);

        let registration = self.media.register_pinned_endpoint(
            request.entity_id.clone(),
            CameraMediaKind::Snapshot,
            self.sessions.endpoint_uri(&session),
            self.endpoint.connection_target.clone(),
        );
        if registration.is_err() {
            if self.sessions.close(&config, session).is_err() {
                return Err(SynologySnapshotHostError::SessionLogoutFailed);
            }
            return Err(SynologySnapshotHostError::EndpointRegistrationRejected);
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
        let removed = self
            .media
            .unregister_endpoint(&request.entity_id, CameraMediaKind::Snapshot);
        let logged_out = self.sessions.close(&config, session);
        if !removed {
            return Err(SynologySnapshotHostError::EndpointRemovalFailed);
        }
        if logged_out.is_err() {
            return Err(SynologySnapshotHostError::SessionLogoutFailed);
        }
        delivery
    }

    pub fn media_snapshot(&self) -> smart_home_camera_media::CameraMediaBrokerSnapshot {
        self.media.snapshot()
    }

    fn validate_request(
        &self,
        request: &SynologySnapshotRequest,
    ) -> Result<(), SynologySnapshotHostError> {
        if request.purpose.trim().is_empty()
            || request.ttl_ms == 0
            || request.ttl_ms > self.max_snapshot_ttl_ms
        {
            return Err(SynologySnapshotHostError::InvalidRequest);
        }
        Ok(())
    }
}

impl<Principals, Credentials>
    SynologySnapshotHost<
        SystemCameraMediaClock,
        OsCameraMediaNonceSource,
        Principals,
        CameraMediaHttpExecutor,
        Credentials,
        SynologyLanSnapshotSessionSource,
    >
where
    Principals: CameraMediaPrincipalSource,
    Credentials: SynologyCredentialSource,
{
    pub fn production(
        principal_source: Principals,
        credentials: Credentials,
        endpoint: SynologySnapshotEndpoint,
    ) -> Self {
        Self::new(
            CameraMediaPolicy::default(),
            SystemCameraMediaClock,
            OsCameraMediaNonceSource,
            principal_source,
            CameraMediaHttpExecutor::default(),
            SynologySnapshotResources::new(
                credentials,
                SynologyLanSnapshotSessionSource::default(),
                endpoint,
            ),
        )
    }
}

struct InstalledTarget {
    bridge_id: BridgeId,
    base_url: String,
    credential_ref: VaultRef,
    camera_id: u64,
}

fn installed_target(
    runtime: &SmartHomeRuntime,
    entity_id: &EntityId,
    endpoint: &SynologySnapshotEndpoint,
) -> Result<InstalledTarget, SynologySnapshotHostError> {
    let registry = runtime.registry();
    let entity = registry
        .entity(entity_id)
        .ok_or(SynologySnapshotHostError::InvalidTarget)?;
    let device = registry
        .device(&entity.device_id)
        .ok_or(SynologySnapshotHostError::InvalidTarget)?;
    let bridge = registry
        .bridge(&device.bridge_id)
        .ok_or(SynologySnapshotHostError::InvalidTarget)?;
    if entity.kind != EntityKind::Camera
        || bridge.bridge_id != endpoint.bridge_id
        || bridge.integration_id != IntegrationId::trusted(INTEGRATION_ID)
    {
        return Err(SynologySnapshotHostError::InvalidTarget);
    }
    let camera_id = device
        .identifiers
        .iter()
        .find(|identifier| {
            identifier.kind == "camera_id"
                && matches!(
                    &identifier.family,
                    ProtocolFamily::Vendor(family) if family == PROTOCOL_ID
                )
        })
        .and_then(|identifier| identifier.value.parse::<u64>().ok())
        .filter(|camera_id| *camera_id != 0)
        .ok_or(SynologySnapshotHostError::InvalidTarget)?;
    if *entity_id != EntityId::trusted(format!("synology-surveillance:{camera_id}:camera")) {
        return Err(SynologySnapshotHostError::InvalidTarget);
    }
    let base_url = bridge
        .address
        .clone()
        .ok_or(SynologySnapshotHostError::InvalidTarget)?;
    validate_connection_target(&base_url, &endpoint.connection_target)?;
    Ok(InstalledTarget {
        bridge_id: bridge.bridge_id.clone(),
        base_url,
        credential_ref: bridge
            .auth_ref
            .clone()
            .ok_or(SynologySnapshotHostError::MissingCredentialReference)?,
        camera_id,
    })
}

fn validate_connection_target(
    base_url: &str,
    connection_target: &CameraMediaConnectionTarget,
) -> Result<(), SynologySnapshotHostError> {
    let parsed = Url::parse(base_url).map_err(|_| SynologySnapshotHostError::InvalidTarget)?;
    let host = parsed
        .host
        .as_deref()
        .ok_or(SynologySnapshotHostError::InvalidTarget)?;
    if !host.eq_ignore_ascii_case(connection_target.canonical_host())
        || parsed.effective_port() != Some(connection_target.pinned_address().port())
        || (parsed.scheme == "http"
            && (!is_loopback_host(host) || !connection_target.pinned_address().ip().is_loopback()))
    {
        return Err(SynologySnapshotHostError::InvalidTarget);
    }
    Ok(())
}

fn decode_credentials(bytes: &[u8]) -> Result<SynologyCredentials, SynologySnapshotHostError> {
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(SynologySnapshotHostError::InvalidCredentialPayload);
    }
    let envelope: CredentialEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| SynologySnapshotHostError::InvalidCredentialPayload)?;
    if envelope.schema_version != 1 {
        return Err(SynologySnapshotHostError::InvalidCredentialPayload);
    }
    validate_credential_fields(&envelope.username, &envelope.password)?;
    SynologyCredentials::new(
        envelope.username.into_inner(),
        envelope.password.into_inner(),
    )
    .map_err(|_| SynologySnapshotHostError::InvalidCredentialPayload)
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
) -> Result<(), SynologySnapshotHostError> {
    if username.trim().is_empty()
        || password.is_empty()
        || username.len() > MAX_CREDENTIAL_FIELD_BYTES
        || password.len() > MAX_CREDENTIAL_FIELD_BYTES
        || username.contains(['\r', '\n', '\0'])
        || password.contains(['\r', '\n', '\0'])
    {
        return Err(SynologySnapshotHostError::InvalidCredentialPayload);
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("127.")
}
