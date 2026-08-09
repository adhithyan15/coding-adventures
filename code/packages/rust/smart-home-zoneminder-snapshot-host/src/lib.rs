//! Authorized production host for bounded ZoneMinder single-image delivery.

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
use smart_home_zoneminder_integration::{
    ZoneMinderAccessToken, ZoneMinderConfig, ZoneMinderCredentials, ZoneMinderLanTransport,
    INTEGRATION_ID, MAX_PASSWORD_BYTES, MAX_USERNAME_BYTES, PROTOCOL_ID,
};
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const ZONEMINDER_VAULT_NAMESPACE: &str = "smart_home.zoneminder.credentials";
pub const ZONEMINDER_VAULT_REF_PREFIX: &str = "vault://smart-home/zoneminder/";
pub const MAX_CREDENTIAL_PAYLOAD_BYTES: usize = MAX_USERNAME_BYTES + MAX_PASSWORD_BYTES + 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneMinderCredentialSourceError;

pub trait ZoneMinderCredentialSource {
    fn resolve(
        &self,
        vault_ref: &VaultRef,
    ) -> Result<LeasePayload, ZoneMinderCredentialSourceError>;
}

pub struct ZoneMinderSealedStoreCredentialSource {
    vault: Arc<SealedStore>,
}

impl ZoneMinderSealedStoreCredentialSource {
    pub fn new(vault: Arc<SealedStore>) -> Self {
        Self { vault }
    }
}

impl ZoneMinderCredentialSource for ZoneMinderSealedStoreCredentialSource {
    fn resolve(
        &self,
        vault_ref: &VaultRef,
    ) -> Result<LeasePayload, ZoneMinderCredentialSourceError> {
        let key = zoneminder_vault_record_key(vault_ref).ok_or(ZoneMinderCredentialSourceError)?;
        let record = self
            .vault
            .get(ZONEMINDER_VAULT_NAMESPACE, key)
            .map_err(|_| ZoneMinderCredentialSourceError)?
            .ok_or(ZoneMinderCredentialSourceError)?;
        Ok(LeasePayload::new(record.plaintext.into_inner()))
    }
}

pub fn zoneminder_vault_record_key(vault_ref: &VaultRef) -> Option<&str> {
    vault_ref
        .as_str()
        .strip_prefix(ZONEMINDER_VAULT_REF_PREFIX)
        .filter(|key| !key.is_empty())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ZoneMinderAccessTokenSourceError;

pub trait ZoneMinderAccessTokenSource {
    fn acquire(
        &mut self,
        config: &ZoneMinderConfig,
        credentials: &ZoneMinderCredentials,
    ) -> Result<ZoneMinderAccessToken, ZoneMinderAccessTokenSourceError>;
}

#[derive(Default)]
pub struct ZoneMinderLanAccessTokenSource {
    transport: ZoneMinderLanTransport,
}

impl ZoneMinderLanAccessTokenSource {
    pub fn new(transport: ZoneMinderLanTransport) -> Self {
        Self { transport }
    }
}

impl ZoneMinderAccessTokenSource for ZoneMinderLanAccessTokenSource {
    fn acquire(
        &mut self,
        config: &ZoneMinderConfig,
        credentials: &ZoneMinderCredentials,
    ) -> Result<ZoneMinderAccessToken, ZoneMinderAccessTokenSourceError> {
        self.transport
            .acquire_access_token(config, credentials)
            .map_err(|_| ZoneMinderAccessTokenSourceError)
    }
}

#[derive(Clone)]
pub struct ZoneMinderSnapshotEndpoint {
    bridge_id: BridgeId,
    zms_url: String,
    connection_target: CameraMediaConnectionTarget,
}

impl ZoneMinderSnapshotEndpoint {
    pub fn new(
        bridge_id: BridgeId,
        zms_url: impl Into<String>,
        connection_target: CameraMediaConnectionTarget,
    ) -> Result<Self, ZoneMinderSnapshotHostError> {
        let zms_url = zms_url.into();
        let parsed = parse_credential_free_endpoint(&zms_url)?;
        let host = parsed
            .host
            .as_deref()
            .ok_or(ZoneMinderSnapshotHostError::InvalidEndpoint)?;
        let port = parsed
            .effective_port()
            .ok_or(ZoneMinderSnapshotHostError::InvalidEndpoint)?;
        if !host.eq_ignore_ascii_case(connection_target.canonical_host())
            || port != connection_target.pinned_address().port()
        {
            return Err(ZoneMinderSnapshotHostError::InvalidEndpoint);
        }
        if parsed.scheme == "http"
            && (!is_loopback_host(host) || !connection_target.pinned_address().ip().is_loopback())
        {
            return Err(ZoneMinderSnapshotHostError::InvalidEndpoint);
        }
        Ok(Self {
            bridge_id,
            zms_url,
            connection_target,
        })
    }
}

impl fmt::Debug for ZoneMinderSnapshotEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ZoneMinderSnapshotEndpoint")
            .field("bridge_id", &self.bridge_id)
            .field("zms_url", &"[REDACTED]")
            .field("connection_target", &self.connection_target)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZoneMinderSnapshotRequest {
    pub entity_id: EntityId,
    pub purpose: String,
    pub ttl_ms: u64,
}

impl ZoneMinderSnapshotRequest {
    pub fn new(entity_id: EntityId, purpose: impl Into<String>, ttl_ms: u64) -> Self {
        Self {
            entity_id,
            purpose: purpose.into(),
            ttl_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZoneMinderSnapshotHostError {
    InvalidRequest,
    InvalidEndpoint,
    InvalidTarget,
    MissingCredentialReference,
    CredentialResolutionRejected,
    InvalidCredentialPayload,
    AccessTokenRejected,
    EndpointAlreadyRegistered,
    EndpointRegistrationRejected,
    EndpointRemovalFailed,
    Media(CameraMediaError),
}

impl fmt::Display for ZoneMinderSnapshotHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "invalid ZoneMinder snapshot request",
            Self::InvalidEndpoint => "invalid ZoneMinder snapshot endpoint",
            Self::InvalidTarget => "snapshot target is not an installed ZoneMinder camera",
            Self::MissingCredentialReference => "ZoneMinder bridge has no credential reference",
            Self::CredentialResolutionRejected => "ZoneMinder credential lookup was rejected",
            Self::InvalidCredentialPayload => "ZoneMinder credential payload is invalid",
            Self::AccessTokenRejected => "ZoneMinder access-token acquisition failed",
            Self::EndpointAlreadyRegistered => "ZoneMinder snapshot endpoint is already registered",
            Self::EndpointRegistrationRejected => {
                "ZoneMinder snapshot endpoint registration failed"
            }
            Self::EndpointRemovalFailed => "ZoneMinder snapshot endpoint removal failed",
            Self::Media(error) => return error.fmt(formatter),
        })
    }
}

impl std::error::Error for ZoneMinderSnapshotHostError {}

impl From<CameraMediaError> for ZoneMinderSnapshotHostError {
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

pub fn encode_zoneminder_credentials(
    username: &str,
    password: &str,
) -> Result<LeasePayload, ZoneMinderSnapshotHostError> {
    validate_credential_fields(username, password)?;
    let bytes = Zeroizing::new(
        serde_json::to_vec(&BorrowedCredentialEnvelope {
            schema_version: 1,
            username,
            password,
        })
        .map_err(|_| ZoneMinderSnapshotHostError::InvalidCredentialPayload)?,
    );
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(ZoneMinderSnapshotHostError::InvalidCredentialPayload);
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

pub struct ZoneMinderSnapshotResources<Credentials, Tokens> {
    credentials: Credentials,
    tokens: Tokens,
    endpoint: ZoneMinderSnapshotEndpoint,
}

impl<Credentials, Tokens> ZoneMinderSnapshotResources<Credentials, Tokens> {
    pub fn new(
        credentials: Credentials,
        tokens: Tokens,
        endpoint: ZoneMinderSnapshotEndpoint,
    ) -> Self {
        Self {
            credentials,
            tokens,
            endpoint,
        }
    }
}

pub struct ZoneMinderSnapshotHost<Clock, Nonce, Principals, Executor, Credentials, Tokens>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
    Credentials: ZoneMinderCredentialSource,
    Tokens: ZoneMinderAccessTokenSource,
{
    media: CameraMediaService<Clock, Nonce, Principals, Executor>,
    credentials: Credentials,
    tokens: Tokens,
    endpoint: ZoneMinderSnapshotEndpoint,
    max_snapshot_ttl_ms: u64,
}

impl<Clock, Nonce, Principals, Executor, Credentials, Tokens>
    ZoneMinderSnapshotHost<Clock, Nonce, Principals, Executor, Credentials, Tokens>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
    Credentials: ZoneMinderCredentialSource,
    Tokens: ZoneMinderAccessTokenSource,
{
    pub fn new(
        policy: CameraMediaPolicy,
        clock: Clock,
        nonce_source: Nonce,
        principal_source: Principals,
        executor: Executor,
        resources: ZoneMinderSnapshotResources<Credentials, Tokens>,
    ) -> Self {
        let max_snapshot_ttl_ms = policy.max_snapshot_ttl_ms;
        Self {
            media: CameraMediaService::new(policy, clock, nonce_source, principal_source, executor),
            credentials: resources.credentials,
            tokens: resources.tokens,
            endpoint: resources.endpoint,
            max_snapshot_ttl_ms,
        }
    }

    pub fn deliver_snapshot(
        &mut self,
        runtime: &SmartHomeRuntime,
        request: ZoneMinderSnapshotRequest,
    ) -> Result<CameraMediaDelivery, ZoneMinderSnapshotHostError> {
        self.validate_request(&request)?;
        self.media
            .authorize_access(runtime, &request.entity_id, CameraMediaKind::Snapshot)?;
        if self
            .media
            .has_endpoint(&request.entity_id, CameraMediaKind::Snapshot)
        {
            return Err(ZoneMinderSnapshotHostError::EndpointAlreadyRegistered);
        }
        let target = installed_target(runtime, &request.entity_id, &self.endpoint)?;
        let payload = self
            .credentials
            .resolve(&target.credential_ref)
            .map_err(|_| ZoneMinderSnapshotHostError::CredentialResolutionRejected)?;
        let credentials = decode_credentials(payload.as_bytes())?;
        drop(payload);
        let config =
            ZoneMinderConfig::new(target.bridge_id, target.base_url, target.credential_ref)
                .map_err(|_| ZoneMinderSnapshotHostError::InvalidTarget)?;
        let token = self
            .tokens
            .acquire(&config, &credentials)
            .map_err(|_| ZoneMinderSnapshotHostError::AccessTokenRejected)?;
        drop(credentials);
        let endpoint_uri = snapshot_uri(&self.endpoint.zms_url, target.monitor_id, &token);
        drop(token);
        self.media
            .register_pinned_endpoint(
                request.entity_id.clone(),
                CameraMediaKind::Snapshot,
                endpoint_uri.into_inner(),
                self.endpoint.connection_target.clone(),
            )
            .map_err(|_| ZoneMinderSnapshotHostError::EndpointRegistrationRejected)?;

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
            return Err(ZoneMinderSnapshotHostError::EndpointRemovalFailed);
        }
        delivery
    }

    pub fn media_snapshot(&self) -> smart_home_camera_media::CameraMediaBrokerSnapshot {
        self.media.snapshot()
    }

    fn validate_request(
        &self,
        request: &ZoneMinderSnapshotRequest,
    ) -> Result<(), ZoneMinderSnapshotHostError> {
        if request.purpose.trim().is_empty()
            || request.ttl_ms == 0
            || request.ttl_ms > self.max_snapshot_ttl_ms
        {
            return Err(ZoneMinderSnapshotHostError::InvalidRequest);
        }
        Ok(())
    }
}

impl<Principals, Credentials>
    ZoneMinderSnapshotHost<
        SystemCameraMediaClock,
        OsCameraMediaNonceSource,
        Principals,
        CameraMediaHttpExecutor,
        Credentials,
        ZoneMinderLanAccessTokenSource,
    >
where
    Principals: CameraMediaPrincipalSource,
    Credentials: ZoneMinderCredentialSource,
{
    pub fn production(
        principal_source: Principals,
        credentials: Credentials,
        endpoint: ZoneMinderSnapshotEndpoint,
    ) -> Self {
        Self::new(
            CameraMediaPolicy::default(),
            SystemCameraMediaClock,
            OsCameraMediaNonceSource,
            principal_source,
            CameraMediaHttpExecutor::default(),
            ZoneMinderSnapshotResources::new(
                credentials,
                ZoneMinderLanAccessTokenSource::default(),
                endpoint,
            ),
        )
    }
}

struct InstalledTarget {
    bridge_id: BridgeId,
    base_url: String,
    credential_ref: VaultRef,
    monitor_id: u64,
}

fn installed_target(
    runtime: &SmartHomeRuntime,
    entity_id: &EntityId,
    endpoint: &ZoneMinderSnapshotEndpoint,
) -> Result<InstalledTarget, ZoneMinderSnapshotHostError> {
    let registry = runtime.registry();
    let entity = registry
        .entity(entity_id)
        .ok_or(ZoneMinderSnapshotHostError::InvalidTarget)?;
    let device = registry
        .device(&entity.device_id)
        .ok_or(ZoneMinderSnapshotHostError::InvalidTarget)?;
    let bridge = registry
        .bridge(&device.bridge_id)
        .ok_or(ZoneMinderSnapshotHostError::InvalidTarget)?;
    if entity.kind != EntityKind::Camera
        || bridge.bridge_id != endpoint.bridge_id
        || bridge.integration_id != IntegrationId::trusted(INTEGRATION_ID)
    {
        return Err(ZoneMinderSnapshotHostError::InvalidTarget);
    }
    let monitor_id = device
        .identifiers
        .iter()
        .find(|identifier| {
            identifier.kind == "monitor_id"
                && matches!(
                    &identifier.family,
                    ProtocolFamily::Vendor(family) if family == PROTOCOL_ID
                )
        })
        .and_then(|identifier| identifier.value.parse::<u64>().ok())
        .filter(|monitor_id| *monitor_id != 0)
        .ok_or(ZoneMinderSnapshotHostError::InvalidTarget)?;
    if *entity_id != EntityId::trusted(format!("zoneminder:monitor:{monitor_id}:camera")) {
        return Err(ZoneMinderSnapshotHostError::InvalidTarget);
    }
    let base_url = bridge
        .address
        .clone()
        .ok_or(ZoneMinderSnapshotHostError::InvalidTarget)?;
    if !same_origin(&base_url, &endpoint.zms_url)? {
        return Err(ZoneMinderSnapshotHostError::InvalidTarget);
    }
    Ok(InstalledTarget {
        bridge_id: bridge.bridge_id.clone(),
        base_url,
        credential_ref: bridge
            .auth_ref
            .clone()
            .ok_or(ZoneMinderSnapshotHostError::MissingCredentialReference)?,
        monitor_id,
    })
}

fn decode_credentials(bytes: &[u8]) -> Result<ZoneMinderCredentials, ZoneMinderSnapshotHostError> {
    if bytes.len() > MAX_CREDENTIAL_PAYLOAD_BYTES {
        return Err(ZoneMinderSnapshotHostError::InvalidCredentialPayload);
    }
    let envelope: CredentialEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| ZoneMinderSnapshotHostError::InvalidCredentialPayload)?;
    if envelope.schema_version != 1 {
        return Err(ZoneMinderSnapshotHostError::InvalidCredentialPayload);
    }
    validate_credential_fields(&envelope.username, &envelope.password)?;
    ZoneMinderCredentials::new(
        envelope.username.into_inner(),
        envelope.password.into_inner(),
    )
    .map_err(|_| ZoneMinderSnapshotHostError::InvalidCredentialPayload)
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
) -> Result<(), ZoneMinderSnapshotHostError> {
    if username.trim().is_empty()
        || password.is_empty()
        || username.len() > MAX_USERNAME_BYTES
        || password.len() > MAX_PASSWORD_BYTES
        || username.contains(['\r', '\n', '\0'])
        || password.contains(['\r', '\n', '\0'])
    {
        return Err(ZoneMinderSnapshotHostError::InvalidCredentialPayload);
    }
    Ok(())
}

fn snapshot_uri(
    zms_url: &str,
    monitor_id: u64,
    token: &ZoneMinderAccessToken,
) -> Zeroizing<String> {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut uri = Zeroizing::new(format!(
        "{zms_url}?mode=single&monitor={monitor_id}&scale=100&token="
    ));
    for byte in token.as_str().bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            uri.push(char::from(byte));
        } else {
            uri.push('%');
            uri.push(char::from(HEX[usize::from(byte >> 4)]));
            uri.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    uri
}

fn parse_credential_free_endpoint(url: &str) -> Result<Url, ZoneMinderSnapshotHostError> {
    let parsed = Url::parse(url).map_err(|_| ZoneMinderSnapshotHostError::InvalidEndpoint)?;
    let host = parsed
        .host
        .as_deref()
        .ok_or(ZoneMinderSnapshotHostError::InvalidEndpoint)?;
    let secure = parsed.scheme == "https";
    let fixture_loopback = parsed.scheme == "http" && is_loopback_host(host);
    if (!secure && !fixture_loopback)
        || parsed.userinfo.is_some()
        || parsed.query.is_some()
        || parsed.fragment.is_some()
        || parsed.path.is_empty()
        || parsed.path == "/"
        || parsed.path.contains(['\r', '\n', '\0'])
    {
        return Err(ZoneMinderSnapshotHostError::InvalidEndpoint);
    }
    Ok(parsed)
}

fn same_origin(left: &str, right: &str) -> Result<bool, ZoneMinderSnapshotHostError> {
    let left = Url::parse(left).map_err(|_| ZoneMinderSnapshotHostError::InvalidTarget)?;
    let right = parse_credential_free_endpoint(right)?;
    Ok(left.scheme == right.scheme
        && left
            .host
            .as_deref()
            .zip(right.host.as_deref())
            .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
        && left.effective_port() == right.effective_port())
}

fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("127.")
}
