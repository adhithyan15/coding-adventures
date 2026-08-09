//! Authorized production host for bounded Axis camera snapshots.

#![forbid(unsafe_code)]

use coding_adventures_csprng::random_array;
use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::SealedStore;
use coding_adventures_zeroize::Zeroizing;
use serde::{Deserialize, Deserializer, Serialize};
use smart_home_axis_vapix_integration::{
    AxisConfig, INTEGRATION_ID, JPEG_SNAPSHOT_PATH, MAX_CREDENTIAL_FIELD_BYTES,
};
use smart_home_camera_media::{
    CameraMediaAccessRequest, CameraMediaClock, CameraMediaConnectionTarget,
    CameraMediaCredentialRegistry, CameraMediaDelivery, CameraMediaError, CameraMediaExecutor,
    CameraMediaKind, CameraMediaNonceError, CameraMediaNonceSource, CameraMediaPolicy,
    CameraMediaPrincipalSource, CameraMediaService,
};
use smart_home_camera_media_http_executor::{
    CameraMediaHttpCredentialError, CameraMediaHttpCredentials, CameraMediaHttpExecutor,
};
use smart_home_core::{
    BridgeId, CapabilityId, CapabilityMode, EntityId, EntityKind, IntegrationId, VaultRef,
};
use smart_home_runtime::SmartHomeRuntime;
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const AXIS_VAULT_NAMESPACE: &str = "smart_home.axis_vapix.credentials";
pub const AXIS_VAULT_REF_PREFIX: &str = "vault://smart-home/axis-vapix/";
pub const MAX_CREDENTIAL_PAYLOAD_BYTES: usize = MAX_CREDENTIAL_FIELD_BYTES * 2 + 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisCredentialSourceError;

pub trait AxisCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, AxisCredentialSourceError>;
}

pub struct AxisSealedStoreCredentialSource {
    vault: Arc<SealedStore>,
}

impl AxisSealedStoreCredentialSource {
    pub fn new(vault: Arc<SealedStore>) -> Self {
        Self { vault }
    }
}

impl AxisCredentialSource for AxisSealedStoreCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, AxisCredentialSourceError> {
        let key = axis_vault_record_key(vault_ref).ok_or(AxisCredentialSourceError)?;
        let record = self
            .vault
            .get(AXIS_VAULT_NAMESPACE, key)
            .map_err(|_| AxisCredentialSourceError)?
            .ok_or(AxisCredentialSourceError)?;
        Ok(LeasePayload::new(record.plaintext.into_inner()))
    }
}

pub fn axis_vault_record_key(vault_ref: &VaultRef) -> Option<&str> {
    vault_ref
        .as_str()
        .strip_prefix(AXIS_VAULT_REF_PREFIX)
        .filter(|key| !key.is_empty())
}

#[derive(Clone)]
pub struct AxisSnapshotEndpoint {
    bridge_id: BridgeId,
    connection_target: CameraMediaConnectionTarget,
}

impl AxisSnapshotEndpoint {
    pub fn new(bridge_id: BridgeId, connection_target: CameraMediaConnectionTarget) -> Self {
        Self {
            bridge_id,
            connection_target,
        }
    }
}

impl fmt::Debug for AxisSnapshotEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AxisSnapshotEndpoint")
            .field("bridge_id", &self.bridge_id)
            .field("connection_target", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AxisSnapshotRequest {
    pub entity_id: EntityId,
    pub purpose: String,
    pub ttl_ms: u64,
}

impl AxisSnapshotRequest {
    pub fn new(entity_id: EntityId, purpose: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            entity_id,
            purpose: purpose.into(),
            ttl_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxisSnapshotHostError {
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

impl fmt::Display for AxisSnapshotHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "invalid Axis snapshot request",
            Self::InvalidTarget => "snapshot target is not an installed Axis camera 1",
            Self::MissingCredentialReference => "Axis bridge has no credential reference",
            Self::CredentialResolutionRejected => "Axis credential lookup was rejected",
            Self::InvalidCredentialPayload => "Axis credential payload is invalid",
            Self::CredentialRegistrationRejected => {
                "Axis snapshot credentials could not be registered"
            }
            Self::CredentialRemovalFailed => "Axis snapshot credentials could not be removed",
            Self::EndpointAlreadyRegistered => "Axis snapshot endpoint is already registered",
            Self::EndpointRegistrationRejected => "Axis snapshot endpoint registration failed",
            Self::EndpointRemovalFailed => "Axis snapshot endpoint removal failed",
            Self::Media(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for AxisSnapshotHostError {}

impl From<CameraMediaError> for AxisSnapshotHostError {
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

pub fn encode_axis_credentials(
    username: &str,
    password: &str,
) -> Result<LeasePayload, AxisSnapshotHostError> {
    validate_credential_fields(username, password)?;
    let bytes = Zeroizing::new(
        serde_json::to_vec(&BorrowedCredentialEnvelope {
            schema_version: 1,
            username,
            password,
        })
        .map_err(|_| AxisSnapshotHostError::InvalidCredentialPayload)?,
    );
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(AxisSnapshotHostError::InvalidCredentialPayload);
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

pub struct AxisSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor
        + CameraMediaCredentialRegistry<
            Credentials = CameraMediaHttpCredentials,
            Error = CameraMediaHttpCredentialError,
        >,
    Credentials: AxisCredentialSource,
{
    media: CameraMediaService<Clock, Nonce, Principals, Executor>,
    credentials: Credentials,
    endpoint: AxisSnapshotEndpoint,
    max_snapshot_ttl_ms: u64,
}

impl<Clock, Nonce, Principals, Executor, Credentials>
    AxisSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor
        + CameraMediaCredentialRegistry<
            Credentials = CameraMediaHttpCredentials,
            Error = CameraMediaHttpCredentialError,
        >,
    Credentials: AxisCredentialSource,
{
    pub fn new(
        policy: CameraMediaPolicy,
        clock: Clock,
        nonce_source: Nonce,
        principal_source: Principals,
        executor: Executor,
        credentials: Credentials,
        endpoint: AxisSnapshotEndpoint,
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
        request: AxisSnapshotRequest,
    ) -> Result<CameraMediaDelivery, AxisSnapshotHostError> {
        self.validate_request(&request)?;
        self.media
            .authorize_access(runtime, &request.entity_id, CameraMediaKind::Snapshot)?;
        if self
            .media
            .has_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
        {
            return Err(AxisSnapshotHostError::EndpointAlreadyRegistered);
        }
        let target = installed_target(runtime, &request.entity_id, &self.endpoint)?;
        let payload = self
            .credentials
            .resolve(&target.credential_ref)
            .map_err(|_| AxisSnapshotHostError::CredentialResolutionRejected)?;
        let credentials = decode_credentials(payload.as_bytes())?;
        drop(payload);

        self.media
            .register_pinned_endpoint(
                request.entity_id.clone(),
                CameraMediaKind::Snapshot,
                &target.snapshot_uri,
                self.endpoint.connection_target.clone(),
            )
            .map_err(|_| AxisSnapshotHostError::EndpointRegistrationRejected)?;
        if self
            .media
            .register_executor_credentials(request.entity_id.clone(), credentials)
            .is_err()
        {
            if !self
                .media
                .unregister_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
            {
                return Err(AxisSnapshotHostError::EndpointRemovalFailed);
            }
            return Err(AxisSnapshotHostError::CredentialRegistrationRejected);
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
            return Err(AxisSnapshotHostError::CredentialRemovalFailed);
        }
        if !endpoint_removed {
            return Err(AxisSnapshotHostError::EndpointRemovalFailed);
        }
        delivery
    }

    pub fn media_snapshot(&self) -> smart_home_camera_media::CameraMediaBrokerSnapshot {
        self.media.snapshot()
    }

    fn validate_request(&self, request: &AxisSnapshotRequest) -> Result<(), AxisSnapshotHostError> {
        if request.purpose.trim().is_empty()
            || request.ttl_ms == 0
            || request.ttl_ms > self.max_snapshot_ttl_ms
        {
            return Err(AxisSnapshotHostError::InvalidRequest);
        }
        Ok(())
    }
}

impl<Principals, Credentials>
    AxisSnapshotHost<
        SystemCameraMediaClock,
        OsCameraMediaNonceSource,
        Principals,
        CameraMediaHttpExecutor,
        Credentials,
    >
where
    Principals: CameraMediaPrincipalSource,
    Credentials: AxisCredentialSource,
{
    pub fn production(
        principal_source: Principals,
        credentials: Credentials,
        endpoint: AxisSnapshotEndpoint,
    ) -> Self {
        Self::new(
            CameraMediaPolicy::default(),
            SystemCameraMediaClock,
            OsCameraMediaNonceSource,
            principal_source,
            CameraMediaHttpExecutor::default(),
            credentials,
            endpoint,
        )
    }
}

struct InstalledTarget {
    credential_ref: VaultRef,
    snapshot_uri: String,
}

fn installed_target(
    runtime: &SmartHomeRuntime,
    entity_id: &EntityId,
    endpoint: &AxisSnapshotEndpoint,
) -> Result<InstalledTarget, AxisSnapshotHostError> {
    let registry = runtime.registry();
    let entity = registry
        .entity(entity_id)
        .ok_or(AxisSnapshotHostError::InvalidTarget)?;
    let device = registry
        .device(&entity.device_id)
        .ok_or(AxisSnapshotHostError::InvalidTarget)?;
    let bridge = registry
        .bridge(&device.bridge_id)
        .ok_or(AxisSnapshotHostError::InvalidTarget)?;
    let snapshot_capability = entity.capabilities.iter().any(|capability| {
        capability.capability_id == CapabilityId::trusted("camera.snapshot")
            && capability.mode == CapabilityMode::Command
    });
    let exact_channel = entity
        .metadata
        .iter()
        .any(|metadata| metadata.key == "axis.video_channel" && metadata.value == "1");
    let expected_entity_id = EntityId::trusted(format!("{}:camera", device.device_id.as_str()));
    if entity.kind != EntityKind::Camera
        || *entity_id != expected_entity_id
        || !device.entity_ids.contains(entity_id)
        || bridge.bridge_id != endpoint.bridge_id
        || bridge.integration_id != IntegrationId::trusted(INTEGRATION_ID)
        || !snapshot_capability
        || !exact_channel
    {
        return Err(AxisSnapshotHostError::InvalidTarget);
    }
    let credential_ref = bridge
        .auth_ref
        .clone()
        .ok_or(AxisSnapshotHostError::MissingCredentialReference)?;
    let base_url = bridge
        .address
        .clone()
        .ok_or(AxisSnapshotHostError::InvalidTarget)?;
    let config = AxisConfig::new(bridge.bridge_id.clone(), base_url, credential_ref.clone())
        .map_err(|_| AxisSnapshotHostError::InvalidTarget)?;
    validate_connection_target(&config.base_url, &endpoint.connection_target)?;
    Ok(InstalledTarget {
        credential_ref,
        snapshot_uri: format!("{}{JPEG_SNAPSHOT_PATH}", config.base_url),
    })
}

fn validate_connection_target(
    base_url: &str,
    connection_target: &CameraMediaConnectionTarget,
) -> Result<(), AxisSnapshotHostError> {
    let parsed = Url::parse(base_url).map_err(|_| AxisSnapshotHostError::InvalidTarget)?;
    let host = parsed
        .host
        .as_deref()
        .ok_or(AxisSnapshotHostError::InvalidTarget)?;
    if !host.eq_ignore_ascii_case(connection_target.canonical_host())
        || parsed.effective_port() != Some(connection_target.pinned_address().port())
        || (parsed.scheme == "http"
            && (!is_loopback_host(host) || !connection_target.pinned_address().ip().is_loopback()))
    {
        return Err(AxisSnapshotHostError::InvalidTarget);
    }
    Ok(())
}

fn decode_credentials(bytes: &[u8]) -> Result<CameraMediaHttpCredentials, AxisSnapshotHostError> {
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(AxisSnapshotHostError::InvalidCredentialPayload);
    }
    let envelope: CredentialEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| AxisSnapshotHostError::InvalidCredentialPayload)?;
    if envelope.schema_version != 1 {
        return Err(AxisSnapshotHostError::InvalidCredentialPayload);
    }
    validate_credential_fields(&envelope.username, &envelope.password)?;
    CameraMediaHttpCredentials::new(
        envelope.username.into_inner(),
        envelope.password.into_inner(),
    )
    .map_err(|_| AxisSnapshotHostError::InvalidCredentialPayload)
}

fn deserialize_secret_string<'de, D>(deserializer: D) -> Result<Zeroizing<String>, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer).map(Zeroizing::new)
}

fn validate_credential_fields(username: &str, password: &str) -> Result<(), AxisSnapshotHostError> {
    if username.trim().is_empty()
        || password.is_empty()
        || username.len() > MAX_CREDENTIAL_FIELD_BYTES
        || password.len() > MAX_CREDENTIAL_FIELD_BYTES
        || username.contains(':')
        || username.contains(['\r', '\n', '\0'])
        || password.contains(['\r', '\n', '\0'])
    {
        return Err(AxisSnapshotHostError::InvalidCredentialPayload);
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("127.")
}
