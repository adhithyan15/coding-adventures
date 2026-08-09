//! Authorized production host for bounded Reolink camera snapshots.

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
use smart_home_core::{
    BridgeId, CapabilityId, CapabilityMode, EntityId, EntityKind, Health, IntegrationId, Value,
    VaultRef,
};
use smart_home_reolink_integration::{ReolinkConfig, INTEGRATION_ID, SNAPSHOT_PATH};
use smart_home_runtime::SmartHomeRuntime;
use std::fmt::{self, Write as _};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const REOLINK_VAULT_NAMESPACE: &str = "smart_home.reolink.credentials";
pub const REOLINK_VAULT_REF_PREFIX: &str = "vault://smart-home/reolink/";
pub const MAX_CREDENTIAL_FIELD_BYTES: usize = 1_024;
pub const MAX_CREDENTIAL_PAYLOAD_BYTES: usize = MAX_CREDENTIAL_FIELD_BYTES * 2 + 256;
const SNAPSHOT_REQUEST_TAG: &str = "smart-home-d23";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReolinkCredentialSourceError;

pub trait ReolinkCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, ReolinkCredentialSourceError>;
}

pub struct ReolinkSealedStoreCredentialSource {
    vault: Arc<SealedStore>,
}

impl ReolinkSealedStoreCredentialSource {
    pub fn new(vault: Arc<SealedStore>) -> Self {
        Self { vault }
    }
}

impl ReolinkCredentialSource for ReolinkSealedStoreCredentialSource {
    fn resolve(&self, vault_ref: &VaultRef) -> Result<LeasePayload, ReolinkCredentialSourceError> {
        let key = reolink_vault_record_key(vault_ref).ok_or(ReolinkCredentialSourceError)?;
        let record = self
            .vault
            .get(REOLINK_VAULT_NAMESPACE, key)
            .map_err(|_| ReolinkCredentialSourceError)?
            .ok_or(ReolinkCredentialSourceError)?;
        Ok(LeasePayload::new(record.plaintext.into_inner()))
    }
}

pub fn reolink_vault_record_key(vault_ref: &VaultRef) -> Option<&str> {
    vault_ref
        .as_str()
        .strip_prefix(REOLINK_VAULT_REF_PREFIX)
        .filter(|key| !key.is_empty())
}

#[derive(Clone)]
pub struct ReolinkSnapshotEndpoint {
    bridge_id: BridgeId,
    connection_target: CameraMediaConnectionTarget,
}

impl ReolinkSnapshotEndpoint {
    pub fn new(bridge_id: BridgeId, connection_target: CameraMediaConnectionTarget) -> Self {
        Self {
            bridge_id,
            connection_target,
        }
    }
}

impl fmt::Debug for ReolinkSnapshotEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ReolinkSnapshotEndpoint")
            .field("bridge_id", &self.bridge_id)
            .field("connection_target", &"[REDACTED]")
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReolinkSnapshotRequest {
    pub entity_id: EntityId,
    pub purpose: String,
    pub ttl_ms: u64,
}

impl ReolinkSnapshotRequest {
    pub fn new(entity_id: EntityId, purpose: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            entity_id,
            purpose: purpose.into(),
            ttl_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReolinkSnapshotHostError {
    InvalidRequest,
    InvalidTarget,
    MissingCredentialReference,
    CredentialResolutionRejected,
    InvalidCredentialPayload,
    EndpointAlreadyRegistered,
    EndpointRegistrationRejected,
    EndpointRemovalFailed,
    Media(CameraMediaError),
}

impl fmt::Display for ReolinkSnapshotHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "invalid Reolink snapshot request",
            Self::InvalidTarget => "snapshot target is not an installed Reolink RLC channel",
            Self::MissingCredentialReference => "Reolink bridge has no credential reference",
            Self::CredentialResolutionRejected => "Reolink credential lookup was rejected",
            Self::InvalidCredentialPayload => "Reolink credential payload is invalid",
            Self::EndpointAlreadyRegistered => "Reolink snapshot endpoint is already registered",
            Self::EndpointRegistrationRejected => "Reolink snapshot endpoint registration failed",
            Self::EndpointRemovalFailed => "Reolink snapshot endpoint removal failed",
            Self::Media(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for ReolinkSnapshotHostError {}

impl From<CameraMediaError> for ReolinkSnapshotHostError {
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

pub fn encode_reolink_credentials(
    username: &str,
    password: &str,
) -> Result<LeasePayload, ReolinkSnapshotHostError> {
    validate_credential_fields(username, password)?;
    let bytes = Zeroizing::new(
        serde_json::to_vec(&BorrowedCredentialEnvelope {
            schema_version: 1,
            username,
            password,
        })
        .map_err(|_| ReolinkSnapshotHostError::InvalidCredentialPayload)?,
    );
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(ReolinkSnapshotHostError::InvalidCredentialPayload);
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

pub struct ReolinkSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
    Credentials: ReolinkCredentialSource,
{
    media: CameraMediaService<Clock, Nonce, Principals, Executor>,
    credentials: Credentials,
    endpoint: ReolinkSnapshotEndpoint,
    max_snapshot_ttl_ms: u64,
}

impl<Clock, Nonce, Principals, Executor, Credentials>
    ReolinkSnapshotHost<Clock, Nonce, Principals, Executor, Credentials>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
    Credentials: ReolinkCredentialSource,
{
    pub fn new(
        policy: CameraMediaPolicy,
        clock: Clock,
        nonce_source: Nonce,
        principal_source: Principals,
        executor: Executor,
        credentials: Credentials,
        endpoint: ReolinkSnapshotEndpoint,
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
        request: ReolinkSnapshotRequest,
    ) -> Result<CameraMediaDelivery, ReolinkSnapshotHostError> {
        self.validate_request(&request)?;
        self.media
            .authorize_access(runtime, &request.entity_id, CameraMediaKind::Snapshot)?;
        if self
            .media
            .has_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
        {
            return Err(ReolinkSnapshotHostError::EndpointAlreadyRegistered);
        }
        let target = installed_target(runtime, &request.entity_id, &self.endpoint)?;
        let payload = self
            .credentials
            .resolve(&target.credential_ref)
            .map_err(|_| ReolinkSnapshotHostError::CredentialResolutionRejected)?;
        let credentials = decode_credentials(payload.as_bytes())?;
        drop(payload);
        let snapshot_uri = build_snapshot_uri(&target.base_url, target.channel, &credentials);
        drop(credentials);

        self.media
            .register_pinned_endpoint(
                request.entity_id.clone(),
                CameraMediaKind::Snapshot,
                snapshot_uri,
                self.endpoint.connection_target.clone(),
            )
            .map_err(|_| ReolinkSnapshotHostError::EndpointRegistrationRejected)?;

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
        if !self
            .media
            .unregister_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
        {
            return Err(ReolinkSnapshotHostError::EndpointRemovalFailed);
        }
        delivery
    }

    pub fn media_snapshot(&self) -> smart_home_camera_media::CameraMediaBrokerSnapshot {
        self.media.snapshot()
    }

    fn validate_request(
        &self,
        request: &ReolinkSnapshotRequest,
    ) -> Result<(), ReolinkSnapshotHostError> {
        if request.purpose.trim().is_empty()
            || request.ttl_ms == 0
            || request.ttl_ms > self.max_snapshot_ttl_ms
        {
            return Err(ReolinkSnapshotHostError::InvalidRequest);
        }
        Ok(())
    }
}

impl<Principals, Credentials>
    ReolinkSnapshotHost<
        SystemCameraMediaClock,
        OsCameraMediaNonceSource,
        Principals,
        CameraMediaHttpExecutor,
        Credentials,
    >
where
    Principals: CameraMediaPrincipalSource,
    Credentials: ReolinkCredentialSource,
{
    pub fn production(
        principal_source: Principals,
        credentials: Credentials,
        endpoint: ReolinkSnapshotEndpoint,
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
    base_url: String,
    channel: u32,
}

fn installed_target(
    runtime: &SmartHomeRuntime,
    entity_id: &EntityId,
    endpoint: &ReolinkSnapshotEndpoint,
) -> Result<InstalledTarget, ReolinkSnapshotHostError> {
    let registry = runtime.registry();
    let entity = registry
        .entity(entity_id)
        .ok_or(ReolinkSnapshotHostError::InvalidTarget)?;
    let device = registry
        .device(&entity.device_id)
        .ok_or(ReolinkSnapshotHostError::InvalidTarget)?;
    let bridge = registry
        .bridge(&device.bridge_id)
        .ok_or(ReolinkSnapshotHostError::InvalidTarget)?;
    let channel = entity
        .metadata
        .iter()
        .find(|metadata| metadata.key == "reolink.channel")
        .and_then(|metadata| metadata.value.parse::<u32>().ok())
        .ok_or(ReolinkSnapshotHostError::InvalidTarget)?;
    let snapshot_capability = entity.capabilities.iter().any(|capability| {
        capability.capability_id == CapabilityId::trusted("camera.snapshot")
            && capability.mode == CapabilityMode::Command
    });
    let expected_entity_id = EntityId::trusted(format!("{}:camera", device.device_id.as_str()));
    let current_channel = matches!(
        entity.state.as_ref().map(|state| &state.value),
        Some(Value::Object(fields))
            if fields.iter().any(|(key, value)| key == "online" && *value == Value::Bool(true))
                && fields.iter().any(|(key, value)| key == "sleeping" && *value == Value::Bool(false))
    );
    if entity.kind != EntityKind::Camera
        || *entity_id != expected_entity_id
        || !device.entity_ids.contains(entity_id)
        || device.health != Health::Online
        || !device.model.trim().to_ascii_uppercase().starts_with("RLC-")
        || bridge.bridge_id != endpoint.bridge_id
        || bridge.integration_id != IntegrationId::trusted(INTEGRATION_ID)
        || !snapshot_capability
        || !current_channel
    {
        return Err(ReolinkSnapshotHostError::InvalidTarget);
    }
    let credential_ref = bridge
        .auth_ref
        .clone()
        .ok_or(ReolinkSnapshotHostError::MissingCredentialReference)?;
    let base_url = bridge
        .address
        .clone()
        .ok_or(ReolinkSnapshotHostError::InvalidTarget)?;
    let config = ReolinkConfig::new(bridge.bridge_id.clone(), base_url, credential_ref.clone())
        .map_err(|_| ReolinkSnapshotHostError::InvalidTarget)?;
    validate_connection_target(&config.base_url, &endpoint.connection_target)?;
    Ok(InstalledTarget {
        credential_ref,
        base_url: config.base_url,
        channel,
    })
}

fn validate_connection_target(
    base_url: &str,
    connection_target: &CameraMediaConnectionTarget,
) -> Result<(), ReolinkSnapshotHostError> {
    let parsed = Url::parse(base_url).map_err(|_| ReolinkSnapshotHostError::InvalidTarget)?;
    let host = parsed
        .host
        .as_deref()
        .ok_or(ReolinkSnapshotHostError::InvalidTarget)?;
    if !host.eq_ignore_ascii_case(connection_target.canonical_host())
        || parsed.effective_port() != Some(connection_target.pinned_address().port())
        || !matches!(parsed.path.as_str(), "" | "/")
        || parsed.query.is_some()
        || parsed.fragment.is_some()
        || (parsed.scheme != "https"
            && (parsed.scheme != "http"
                || !is_loopback_host(host)
                || !connection_target.pinned_address().ip().is_loopback()))
    {
        return Err(ReolinkSnapshotHostError::InvalidTarget);
    }
    Ok(())
}

fn build_snapshot_uri(base_url: &str, channel: u32, credentials: &CredentialEnvelope) -> String {
    let username = encode_query_secret(&credentials.username);
    let password = encode_query_secret(&credentials.password);
    let origin = base_url.trim_end_matches('/');
    format!(
        "{origin}{SNAPSHOT_PATH}?cmd=Snap&channel={channel}&rs={SNAPSHOT_REQUEST_TAG}&user={}&password={}",
        username.as_str(),
        password.as_str()
    )
}

fn decode_credentials(bytes: &[u8]) -> Result<CredentialEnvelope, ReolinkSnapshotHostError> {
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(ReolinkSnapshotHostError::InvalidCredentialPayload);
    }
    let envelope: CredentialEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| ReolinkSnapshotHostError::InvalidCredentialPayload)?;
    if envelope.schema_version != 1 {
        return Err(ReolinkSnapshotHostError::InvalidCredentialPayload);
    }
    validate_credential_fields(&envelope.username, &envelope.password)?;
    Ok(envelope)
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
) -> Result<(), ReolinkSnapshotHostError> {
    if username.trim().is_empty()
        || password.is_empty()
        || username.len() > MAX_CREDENTIAL_FIELD_BYTES
        || password.len() > MAX_CREDENTIAL_FIELD_BYTES
        || username.chars().any(char::is_control)
        || password.chars().any(char::is_control)
    {
        return Err(ReolinkSnapshotHostError::InvalidCredentialPayload);
    }
    Ok(())
}

fn encode_query_secret(value: &str) -> Zeroizing<String> {
    let mut encoded = Zeroizing::new(String::with_capacity(value.len()));
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            write!(&mut *encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("127.")
}
