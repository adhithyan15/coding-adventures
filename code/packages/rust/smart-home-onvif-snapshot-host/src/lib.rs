//! Authorized production host for bounded ONVIF snapshot delivery.

#![forbid(unsafe_code)]

use coding_adventures_csprng::random_array;
use coding_adventures_vault_leases::LeasePayload;
use coding_adventures_vault_sealed_store::SealedStore;
use coding_adventures_zeroize::Zeroizing;
use serde::{Deserialize, Deserializer, Serialize};
use smart_home_camera_media::{
    CameraMediaAccessRequest, CameraMediaClock, CameraMediaCredentialRegistry, CameraMediaDelivery,
    CameraMediaEndpointRegistry, CameraMediaError, CameraMediaExecutor, CameraMediaKind,
    CameraMediaNonceError, CameraMediaNonceSource, CameraMediaPolicy, CameraMediaPrincipalSource,
    CameraMediaService,
};
use smart_home_camera_media_http_executor::{
    CameraMediaHttpCredentialError, CameraMediaHttpCredentials, CameraMediaHttpExecutor,
};
use smart_home_core::{EntityId, IntegrationId, VaultRef};
use smart_home_onvif_integration::{
    INTEGRATION_ID, MAX_ONVIF_CREDENTIAL_BYTES, MAX_ONVIF_VALUE_BYTES,
};
use smart_home_runtime::SmartHomeRuntime;
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

pub const VERSION: &str = "0.1.0";
pub const MAX_CREDENTIAL_PAYLOAD_BYTES: usize = MAX_ONVIF_CREDENTIAL_BYTES * 2 + 256;
pub const ONVIF_VAULT_NAMESPACE: &str = "smart_home.onvif.credentials";
pub const ONVIF_VAULT_REF_PREFIX: &str = "vault://smart-home/onvif/";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OnvifCredentialSourceError;

pub trait OnvifCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, OnvifCredentialSourceError>;
}

pub struct OnvifSealedStoreCredentialSource {
    vault: Arc<SealedStore>,
}

impl OnvifSealedStoreCredentialSource {
    pub fn new(vault: Arc<SealedStore>) -> Self {
        Self { vault }
    }
}

impl OnvifCredentialSource for OnvifSealedStoreCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, OnvifCredentialSourceError> {
        let key = onvif_vault_record_key(vault_ref).ok_or(OnvifCredentialSourceError)?;
        let record = self
            .vault
            .get(ONVIF_VAULT_NAMESPACE, key)
            .map_err(|_| OnvifCredentialSourceError)?
            .ok_or(OnvifCredentialSourceError)?;
        Ok(LeasePayload::new(record.plaintext.into_inner()))
    }
}

pub fn onvif_vault_record_key(vault_ref: &VaultRef) -> Option<&str> {
    vault_ref
        .as_str()
        .strip_prefix(ONVIF_VAULT_REF_PREFIX)
        .filter(|key| !key.is_empty())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnvifSnapshotRequest {
    pub entity_id: EntityId,
    pub purpose: String,
    pub ttl_ms: u64,
}

impl OnvifSnapshotRequest {
    pub fn new(entity_id: EntityId, purpose: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            entity_id,
            purpose: purpose.into(),
            ttl_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnvifSnapshotHostError {
    InvalidRequest,
    MissingEndpoint,
    InvalidTarget,
    MissingCredentialReference,
    CredentialResolutionRejected,
    InvalidCredentialPayload,
    CredentialRegistrationRejected,
    CredentialRemovalFailed,
    Media(CameraMediaError),
}

impl fmt::Display for OnvifSnapshotHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "invalid ONVIF snapshot request",
            Self::MissingEndpoint => "ONVIF snapshot endpoint is not registered",
            Self::InvalidTarget => "snapshot target is not an installed ONVIF camera",
            Self::MissingCredentialReference => "ONVIF camera has no credential reference",
            Self::CredentialResolutionRejected => "ONVIF credential lookup was rejected",
            Self::InvalidCredentialPayload => "ONVIF credential payload is invalid",
            Self::CredentialRegistrationRejected => {
                "ONVIF snapshot credentials could not be registered"
            }
            Self::CredentialRemovalFailed => "ONVIF snapshot credentials could not be removed",
            Self::Media(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for OnvifSnapshotHostError {}

impl From<CameraMediaError> for OnvifSnapshotHostError {
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

pub fn encode_onvif_credentials(
    username: &str,
    password: &str,
) -> Result<LeasePayload, OnvifSnapshotHostError> {
    validate_credential_fields(username, password)?;
    let bytes = Zeroizing::new(
        serde_json::to_vec(&BorrowedCredentialEnvelope {
            schema_version: 1,
            username,
            password,
        })
        .map_err(|_| OnvifSnapshotHostError::InvalidCredentialPayload)?,
    );
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(OnvifSnapshotHostError::InvalidCredentialPayload);
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

pub struct OnvifSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor
        + CameraMediaCredentialRegistry<
            Credentials = CameraMediaHttpCredentials,
            Error = CameraMediaHttpCredentialError,
        >,
    Credentials: OnvifCredentialSource,
{
    media: CameraMediaService<Clock, Nonce, Principals, Executor>,
    credentials: Credentials,
    max_snapshot_ttl_ms: u64,
}

impl<Clock, Nonce, Principals, Executor, Credentials>
    OnvifSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor
        + CameraMediaCredentialRegistry<
            Credentials = CameraMediaHttpCredentials,
            Error = CameraMediaHttpCredentialError,
        >,
    Credentials: OnvifCredentialSource,
{
    pub fn new(
        policy: CameraMediaPolicy,
        clock: Clock,
        nonce_source: Nonce,
        principal_source: Principals,
        executor: Executor,
        credentials: Credentials,
    ) -> Self {
        let max_snapshot_ttl_ms = policy.max_snapshot_ttl_ms;
        Self {
            media: CameraMediaService::new(policy, clock, nonce_source, principal_source, executor),
            credentials,
            max_snapshot_ttl_ms,
        }
    }

    pub fn deliver_snapshot(
        &mut self,
        runtime: &SmartHomeRuntime,
        request: OnvifSnapshotRequest,
    ) -> Result<CameraMediaDelivery, OnvifSnapshotHostError> {
        self.validate_request(&request)?;
        self.media
            .authorize_access(runtime, &request.entity_id, CameraMediaKind::Snapshot)?;
        if !self
            .media
            .has_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
        {
            return Err(OnvifSnapshotHostError::MissingEndpoint);
        }
        let credential_ref = installed_credential_ref(runtime, &request.entity_id)?;
        let payload = self
            .credentials
            .resolve(&credential_ref)
            .map_err(|_| OnvifSnapshotHostError::CredentialResolutionRejected)?;
        let credentials = decode_credentials(payload.as_bytes())?;
        drop(payload);

        self.media
            .register_executor_credentials(request.entity_id.clone(), credentials)
            .map_err(|_| OnvifSnapshotHostError::CredentialRegistrationRejected)?;
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
            .unregister_executor_credentials(&request.entity_id);
        if !removed {
            return Err(OnvifSnapshotHostError::CredentialRemovalFailed);
        }
        delivery
    }

    pub fn media_snapshot(&self) -> smart_home_camera_media::CameraMediaBrokerSnapshot {
        self.media.snapshot()
    }

    fn validate_request(
        &self,
        request: &OnvifSnapshotRequest,
    ) -> Result<(), OnvifSnapshotHostError> {
        if request.purpose.trim().is_empty()
            || request.ttl_ms == 0
            || request.ttl_ms > self.max_snapshot_ttl_ms
        {
            return Err(OnvifSnapshotHostError::InvalidRequest);
        }
        Ok(())
    }
}

impl<Principals, Credentials>
    OnvifSnapshotHost<
        SystemCameraMediaClock,
        OsCameraMediaNonceSource,
        Principals,
        CameraMediaHttpExecutor,
        Credentials,
    >
where
    Principals: CameraMediaPrincipalSource,
    Credentials: OnvifCredentialSource,
{
    pub fn production(principal_source: Principals, credentials: Credentials) -> Self {
        Self::new(
            CameraMediaPolicy::default(),
            SystemCameraMediaClock,
            OsCameraMediaNonceSource,
            principal_source,
            CameraMediaHttpExecutor::default(),
            credentials,
        )
    }
}

impl<Clock, Nonce, Principals, Executor, Credentials> CameraMediaEndpointRegistry
    for OnvifSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor
        + CameraMediaCredentialRegistry<
            Credentials = CameraMediaHttpCredentials,
            Error = CameraMediaHttpCredentialError,
        >,
    Credentials: OnvifCredentialSource,
{
    fn register_camera_endpoint(
        &mut self,
        entity_id: EntityId,
        kind: CameraMediaKind,
        uri: &str,
    ) -> Result<(), CameraMediaError> {
        self.media.register_endpoint(entity_id, kind, uri)
    }

    fn register_pinned_camera_endpoint(
        &mut self,
        entity_id: EntityId,
        kind: CameraMediaKind,
        uri: &str,
        connection_target: smart_home_camera_media::CameraMediaConnectionTarget,
    ) -> Result<(), CameraMediaError> {
        self.media
            .register_pinned_endpoint(entity_id, kind, uri, connection_target)
    }
}

fn installed_credential_ref(
    runtime: &SmartHomeRuntime,
    entity_id: &EntityId,
) -> Result<VaultRef, OnvifSnapshotHostError> {
    let registry = runtime.registry();
    let entity = registry
        .entity(entity_id)
        .ok_or(OnvifSnapshotHostError::InvalidTarget)?;
    let device = registry
        .device(&entity.device_id)
        .ok_or(OnvifSnapshotHostError::InvalidTarget)?;
    let bridge = registry
        .bridge(&device.bridge_id)
        .ok_or(OnvifSnapshotHostError::InvalidTarget)?;
    if bridge.integration_id != IntegrationId::trusted(INTEGRATION_ID) {
        return Err(OnvifSnapshotHostError::InvalidTarget);
    }
    bridge
        .auth_ref
        .clone()
        .ok_or(OnvifSnapshotHostError::MissingCredentialReference)
}

fn decode_credentials(bytes: &[u8]) -> Result<CameraMediaHttpCredentials, OnvifSnapshotHostError> {
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(OnvifSnapshotHostError::InvalidCredentialPayload);
    }
    let envelope: CredentialEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| OnvifSnapshotHostError::InvalidCredentialPayload)?;
    if envelope.schema_version != 1 {
        return Err(OnvifSnapshotHostError::InvalidCredentialPayload);
    }
    validate_credential_fields(&envelope.username, &envelope.password)?;
    CameraMediaHttpCredentials::new(
        envelope.username.into_inner(),
        envelope.password.into_inner(),
    )
    .map_err(|_| OnvifSnapshotHostError::InvalidCredentialPayload)
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
) -> Result<(), OnvifSnapshotHostError> {
    if username.trim().is_empty()
        || password.is_empty()
        || username.len() > MAX_ONVIF_VALUE_BYTES
        || password.len() > MAX_ONVIF_CREDENTIAL_BYTES
        || username.contains(':')
        || username.contains(['\r', '\n', '\0'])
        || password.contains(['\r', '\n', '\0'])
    {
        return Err(OnvifSnapshotHostError::InvalidCredentialPayload);
    }
    Ok(())
}
