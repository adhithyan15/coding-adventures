//! Privacy-preserving camera snapshot and stream leases for D23.
//!
//! The broker is deliberately an in-memory policy core. A native host supplies
//! authenticated identity, trusted time, collision-resistant nonce bytes, and
//! the media executor. The broker never opens a socket and never returns the
//! camera endpoint URI to the lease holder.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroizing;
use smart_home_core::{
    AgentId, CapabilityGrant, CapabilityGrantScope, CapabilityId, CapabilityMode, EntityId,
    PrivilegeTier,
};
use smart_home_runtime::SmartHomeRuntime;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::hash::{Hash, Hasher};
use url_parser::Url;

pub const VERSION: &str = "0.2.0";
pub const SNAPSHOT_CAPABILITY_ID: &str = "camera.snapshot";
pub const STREAM_CAPABILITY_ID: &str = "camera.stream";
pub const DEFAULT_MAX_AUDIT_RECORDS: usize = 256;
pub const DEFAULT_MAX_ENDPOINTS: usize = 128;
pub const DEFAULT_MAX_ACTIVE_LEASES: usize = 256;
pub const DEFAULT_MAX_ACTIVE_LEASES_PER_PRINCIPAL: usize = 32;
pub const DEFAULT_MAX_ACTIVE_STREAMS: usize = 64;
pub const DEFAULT_MAX_SNAPSHOT_BYTES: usize = 10 * 1024 * 1024;

/// The media action authorized by a lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CameraMediaKind {
    Snapshot,
    Stream,
}

impl CameraMediaKind {
    pub fn capability_id(self) -> CapabilityId {
        CapabilityId::trusted(match self {
            Self::Snapshot => SNAPSHOT_CAPABILITY_ID,
            Self::Stream => STREAM_CAPABILITY_ID,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Snapshot => "snapshot",
            Self::Stream => "stream",
        }
    }
}

/// Opaque bearer identifier for one camera-media authorization.
///
/// The ID deliberately has no `Display` implementation, and its `Debug`
/// representation is redacted. `as_hex` is an explicit boundary operation for
/// the host protocol. The owned bytes are zeroized when each copy drops.
pub struct CameraMediaLeaseId(Zeroizing<String>);

impl CameraMediaLeaseId {
    pub fn as_hex(&self) -> &str {
        &self.0
    }

    pub fn from_hex(value: &str) -> Option<Self> {
        if value.len() != 32
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return None;
        }
        Some(Self(Zeroizing::new(value.to_owned())))
    }
}

impl Clone for CameraMediaLeaseId {
    fn clone(&self) -> Self {
        Self(Zeroizing::new(self.as_hex().to_owned()))
    }
}

impl fmt::Debug for CameraMediaLeaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "CameraMediaLeaseId(<{}-char redacted>)",
            self.0.len()
        )
    }
}

impl Hash for CameraMediaLeaseId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_hex().hash(state);
    }
}

impl PartialEq for CameraMediaLeaseId {
    fn eq(&self, other: &Self) -> bool {
        self.as_hex() == other.as_hex()
    }
}

impl Eq for CameraMediaLeaseId {}

impl PartialOrd for CameraMediaLeaseId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CameraMediaLeaseId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_hex().cmp(other.as_hex())
    }
}

/// Bounds that keep process-local endpoint and lease state finite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaPolicy {
    pub max_snapshot_ttl_ms: u64,
    pub max_stream_ttl_ms: u64,
    pub max_audit_records: usize,
    pub max_endpoints: usize,
    pub max_active_leases: usize,
    pub max_active_leases_per_principal: usize,
    pub max_active_streams: usize,
    pub max_snapshot_bytes: usize,
    /// Explicit fixture-only escape hatch. Production defaults remain secure.
    pub allow_plaintext_loopback: bool,
}

impl Default for CameraMediaPolicy {
    fn default() -> Self {
        Self {
            max_snapshot_ttl_ms: 30_000,
            max_stream_ttl_ms: 60_000,
            max_audit_records: DEFAULT_MAX_AUDIT_RECORDS,
            max_endpoints: DEFAULT_MAX_ENDPOINTS,
            max_active_leases: DEFAULT_MAX_ACTIVE_LEASES,
            max_active_leases_per_principal: DEFAULT_MAX_ACTIVE_LEASES_PER_PRINCIPAL,
            max_active_streams: DEFAULT_MAX_ACTIVE_STREAMS,
            max_snapshot_bytes: DEFAULT_MAX_SNAPSHOT_BYTES,
            allow_plaintext_loopback: false,
        }
    }
}

impl CameraMediaPolicy {
    fn max_ttl_ms(&self, kind: CameraMediaKind) -> u64 {
        match kind {
            CameraMediaKind::Snapshot => self.max_snapshot_ttl_ms,
            CameraMediaKind::Stream => self.max_stream_ttl_ms,
        }
    }
}

/// Untrusted camera request fields.
///
/// Principal and time are intentionally absent. The native host derives the
/// authenticated principal and supplies a trusted clock separately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaAccessRequest {
    pub entity_id: EntityId,
    pub kind: CameraMediaKind,
    pub purpose: String,
    pub ttl_ms: u64,
}

impl CameraMediaAccessRequest {
    pub fn new(
        entity_id: EntityId,
        kind: CameraMediaKind,
        purpose: impl Into<String>,
        ttl_ms: u64,
    ) -> Self {
        Self {
            entity_id,
            kind,
            purpose: purpose.into(),
            ttl_ms,
        }
    }
}

/// Public lease metadata. The endpoint never appears here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaLease {
    pub lease_id: CameraMediaLeaseId,
    pub principal_id: AgentId,
    pub entity_id: EntityId,
    pub kind: CameraMediaKind,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub endpoint_generation: u64,
}

impl CameraMediaLease {
    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }
}

/// Broker-minted stream handle returned after a successful mediated open.
pub struct CameraMediaStreamSessionId(Zeroizing<String>);

impl CameraMediaStreamSessionId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Clone for CameraMediaStreamSessionId {
    fn clone(&self) -> Self {
        Self(Zeroizing::new(self.as_str().to_owned()))
    }
}

impl fmt::Debug for CameraMediaStreamSessionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "CameraMediaStreamSessionId(<{}-char redacted>)",
            self.0.len()
        )
    }
}

impl PartialEq for CameraMediaStreamSessionId {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for CameraMediaStreamSessionId {}

impl PartialOrd for CameraMediaStreamSessionId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CameraMediaStreamSessionId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// Snapshot bytes retained in zeroizing memory and redacted from diagnostics.
pub struct CameraMediaSnapshot(Zeroizing<Vec<u8>>);

impl CameraMediaSnapshot {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Clone for CameraMediaSnapshot {
    fn clone(&self) -> Self {
        Self(Zeroizing::new(self.as_bytes().to_vec()))
    }
}

impl fmt::Debug for CameraMediaSnapshot {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "CameraMediaSnapshot(<{} bytes redacted>)",
            self.0.len()
        )
    }
}

impl PartialEq for CameraMediaSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for CameraMediaSnapshot {}

/// Non-endpoint result returned by the trusted executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraMediaDelivery {
    Snapshot {
        snapshot: CameraMediaSnapshot,
    },
    Stream {
        session_id: CameraMediaStreamSessionId,
        expires_at_ms: u64,
    },
}

impl CameraMediaDelivery {
    pub fn snapshot_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Snapshot { snapshot } => Some(snapshot.as_bytes()),
            Self::Stream { .. } => None,
        }
    }
}

/// Executor-owned result. Stream identifiers are deliberately absent: the
/// service mints those after it has taken ownership of the stream resource.
pub enum CameraMediaExecutionResult<Stream> {
    Snapshot(Zeroizing<Vec<u8>>),
    Stream(Stream),
}

impl<Stream> CameraMediaExecutionResult<Stream> {
    pub fn snapshot(bytes: Vec<u8>) -> Self {
        Self::Snapshot(Zeroizing::new(bytes))
    }

    pub fn stream(resource: Stream) -> Self {
        Self::Stream(resource)
    }
}

/// Closed, endpoint-free executor failure classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMediaExecutionError {
    Unavailable,
    Rejected,
    ResourceLimit,
    Protocol,
}

impl fmt::Display for CameraMediaExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "media transport unavailable",
            Self::Rejected => "media transport rejected the request",
            Self::ResourceLimit => "media transport exceeded a resource limit",
            Self::Protocol => "media transport protocol failure",
        })
    }
}

/// A borrowed endpoint available only during trusted executor dispatch.
pub struct CameraMediaExecution<'a> {
    entity_id: &'a EntityId,
    kind: CameraMediaKind,
    endpoint_uri: &'a str,
    expires_at_ms: u64,
    max_snapshot_bytes: usize,
}

impl fmt::Debug for CameraMediaExecution<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CameraMediaExecution")
            .field("entity_id", self.entity_id)
            .field("kind", &self.kind)
            .field("endpoint_uri", &"[REDACTED]")
            .field("expires_at_ms", &self.expires_at_ms)
            .field("max_snapshot_bytes", &self.max_snapshot_bytes)
            .finish()
    }
}

impl<'a> CameraMediaExecution<'a> {
    pub fn entity_id(&self) -> &EntityId {
        self.entity_id
    }

    pub fn kind(&self) -> CameraMediaKind {
        self.kind
    }

    /// Borrow the endpoint inside the trusted host adapter only.
    pub fn endpoint_uri(&self) -> &str {
        self.endpoint_uri
    }

    pub fn expires_at_ms(&self) -> u64 {
        self.expires_at_ms
    }

    pub fn max_snapshot_bytes(&self) -> usize {
        self.max_snapshot_bytes
    }
}

/// Trusted time provider owned by the native host.
pub trait CameraMediaClock {
    fn now_ms(&self) -> u64;
}

/// Endpoint-free failure from a trusted nonce provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraMediaNonceError;

/// Collision-resistant nonce provider owned by the native host.
pub trait CameraMediaNonceSource {
    fn fill_nonce(&mut self, output: &mut [u8; 16]) -> Result<(), CameraMediaNonceError>;
}

/// Native media host. The endpoint is lent only for this call.
pub trait CameraMediaExecutor {
    type Stream;

    fn deliver(
        &mut self,
        execution: CameraMediaExecution<'_>,
    ) -> Result<CameraMediaExecutionResult<Self::Stream>, CameraMediaExecutionError>;

    /// Close one owned stream. On error the resource remains valid for retry.
    fn close_stream(&mut self, stream: &mut Self::Stream) -> Result<(), CameraMediaExecutionError>;
}

/// Authenticated identity source installed once by the native host.
pub trait CameraMediaPrincipalSource {
    fn current_principal(&self) -> Option<AgentId>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMediaAuditOutcome {
    EndpointRegistered,
    EndpointRemoved,
    LeaseIssued,
    LeaseDenied,
    LeaseDelivered,
    LeaseExpired,
    LeaseRejected,
    DeliveryFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaAuditRecord {
    pub sequence: u64,
    pub principal_id: Option<AgentId>,
    pub entity_id: EntityId,
    pub kind: CameraMediaKind,
    pub outcome: CameraMediaAuditOutcome,
    pub reason: Option<String>,
    pub occurred_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaBrokerSnapshot {
    pub endpoint_count: usize,
    pub active_lease_count: usize,
    pub active_stream_count: usize,
    pub pending_stream_cleanup_count: usize,
    pub audit_record_count: usize,
    pub issued_lease_count: u64,
    pub denied_lease_count: u64,
    pub redeemed_lease_count: u64,
    pub failed_delivery_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaReconcileReport {
    pub expired_lease_count: usize,
    pub closed_stream_count: usize,
    pub failed_stream_close_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraMediaError {
    InvalidEndpoint(String),
    UnsupportedEndpointScheme(String),
    EndpointCredentialsForbidden,
    InsecureEndpoint,
    UnknownEntity(EntityId),
    MissingCapability {
        entity_id: EntityId,
        capability_id: CapabilityId,
    },
    ReadOnlyCapability {
        entity_id: EntityId,
        capability_id: CapabilityId,
    },
    MissingEndpoint {
        entity_id: EntityId,
        kind: CameraMediaKind,
    },
    EmptyPurpose,
    InvalidTtl {
        requested_ms: u64,
        maximum_ms: u64,
    },
    EndpointQuotaExceeded {
        maximum: usize,
    },
    LeaseQuotaExceeded {
        maximum: usize,
    },
    PrincipalLeaseQuotaExceeded {
        maximum: usize,
    },
    StreamQuotaExceeded {
        maximum: usize,
    },
    TimestampOverflow,
    EndpointGenerationOverflow,
    NonceUnavailable,
    DuplicateLeaseId,
    DuplicateStreamSessionId,
    Unauthenticated,
    Unauthorized {
        principal_id: AgentId,
        entity_id: EntityId,
        capability_id: CapabilityId,
    },
    UnknownLease,
    LeasePrincipalMismatch,
    ExpiredLease,
    EndpointGenerationChanged,
    Execution(CameraMediaExecutionError),
    InvalidDelivery,
    UnknownStreamSession,
    StreamPrincipalMismatch,
}

impl fmt::Display for CameraMediaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint(message) => {
                write!(formatter, "invalid camera endpoint: {message}")
            }
            Self::UnsupportedEndpointScheme(scheme) => {
                write!(formatter, "unsupported camera endpoint scheme `{scheme}`")
            }
            Self::EndpointCredentialsForbidden => {
                formatter.write_str("camera endpoint must not contain user information")
            }
            Self::InsecureEndpoint => formatter.write_str(
                "plaintext camera endpoints are allowed only for explicit loopback fixtures",
            ),
            Self::UnknownEntity(entity_id) => {
                write!(formatter, "unknown camera entity {entity_id}")
            }
            Self::MissingCapability {
                entity_id,
                capability_id,
            } => write!(formatter, "camera entity {entity_id} lacks {capability_id}"),
            Self::ReadOnlyCapability {
                entity_id,
                capability_id,
            } => write!(
                formatter,
                "camera capability {capability_id} on {entity_id} is read-only"
            ),
            Self::MissingEndpoint { entity_id, kind } => write!(
                formatter,
                "camera entity {entity_id} has no {} endpoint",
                kind.as_str()
            ),
            Self::EmptyPurpose => formatter.write_str("camera access purpose must not be empty"),
            Self::InvalidTtl {
                requested_ms,
                maximum_ms,
            } => write!(
                formatter,
                "camera lease TTL {requested_ms}ms exceeds the allowed 1..={maximum_ms}ms range"
            ),
            Self::EndpointQuotaExceeded { maximum } => {
                write!(formatter, "camera endpoint quota of {maximum} is exhausted")
            }
            Self::LeaseQuotaExceeded { maximum } => {
                write!(formatter, "camera lease quota of {maximum} is exhausted")
            }
            Self::PrincipalLeaseQuotaExceeded { maximum } => write!(
                formatter,
                "camera per-principal lease quota of {maximum} is exhausted"
            ),
            Self::StreamQuotaExceeded { maximum } => {
                write!(formatter, "camera stream quota of {maximum} is exhausted")
            }
            Self::TimestampOverflow => formatter.write_str("camera lease expiry overflowed"),
            Self::EndpointGenerationOverflow => {
                formatter.write_str("camera endpoint generation exhausted")
            }
            Self::NonceUnavailable => formatter.write_str("camera lease nonce source unavailable"),
            Self::DuplicateLeaseId => formatter.write_str("camera lease identifier collision"),
            Self::DuplicateStreamSessionId => {
                formatter.write_str("camera stream session identifier collision")
            }
            Self::Unauthenticated => formatter.write_str("camera media host is unauthenticated"),
            Self::Unauthorized {
                principal_id,
                entity_id,
                capability_id,
            } => write!(
                formatter,
                "principal {principal_id} is not authorized for {capability_id} on {entity_id}"
            ),
            Self::UnknownLease => formatter.write_str("unknown camera media lease"),
            Self::LeasePrincipalMismatch => {
                formatter.write_str("camera media lease belongs to another principal")
            }
            Self::ExpiredLease => formatter.write_str("camera media lease expired"),
            Self::EndpointGenerationChanged => {
                formatter.write_str("camera media endpoint changed after lease issuance")
            }
            Self::Execution(error) => write!(formatter, "{error}"),
            Self::InvalidDelivery => {
                formatter.write_str("camera media executor returned an invalid delivery")
            }
            Self::UnknownStreamSession => formatter.write_str("unknown camera stream session"),
            Self::StreamPrincipalMismatch => {
                formatter.write_str("camera stream session belongs to another principal")
            }
        }
    }
}

impl std::error::Error for CameraMediaError {}

struct CameraMediaEndpoint {
    uri: Zeroizing<String>,
    generation: u64,
}

impl fmt::Debug for CameraMediaEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CameraMediaEndpoint")
            .field("uri", &"[REDACTED]")
            .field("generation", &self.generation)
            .finish()
    }
}

#[derive(Debug)]
struct CameraMediaBroker {
    policy: CameraMediaPolicy,
    endpoints: BTreeMap<(EntityId, CameraMediaKind), CameraMediaEndpoint>,
    leases: BTreeMap<CameraMediaLeaseId, CameraMediaLease>,
    audit: VecDeque<CameraMediaAuditRecord>,
    next_endpoint_generation: u64,
    next_audit_sequence: u64,
    issued_lease_count: u64,
    denied_lease_count: u64,
    redeemed_lease_count: u64,
    failed_delivery_count: u64,
}

struct PendingCameraMediaExecution {
    lease: CameraMediaLease,
    endpoint_uri: Zeroizing<String>,
}

struct ActiveCameraMediaStream<Stream> {
    principal_id: AgentId,
    entity_id: EntityId,
    kind: CameraMediaKind,
    expires_at_ms: u64,
    resource: Stream,
}

/// Host-owned security facade. Identity, time, entropy, and media execution
/// are installed once and cannot be substituted by an untrusted request.
pub struct CameraMediaService<Clock, Nonce, Principals, Executor>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
{
    broker: CameraMediaBroker,
    clock: Clock,
    nonce_source: Nonce,
    principal_source: Principals,
    executor: Executor,
    streams: BTreeMap<CameraMediaStreamSessionId, ActiveCameraMediaStream<Executor::Stream>>,
    pending_stream_cleanup: Vec<Executor::Stream>,
}

impl<Clock, Nonce, Principals, Executor> CameraMediaService<Clock, Nonce, Principals, Executor>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
{
    pub fn new(
        policy: CameraMediaPolicy,
        clock: Clock,
        nonce_source: Nonce,
        principal_source: Principals,
        executor: Executor,
    ) -> Self {
        Self {
            broker: CameraMediaBroker::new(policy),
            clock,
            nonce_source,
            principal_source,
            executor,
            streams: BTreeMap::new(),
            pending_stream_cleanup: Vec::new(),
        }
    }

    pub fn register_endpoint(
        &mut self,
        entity_id: EntityId,
        kind: CameraMediaKind,
        uri: impl Into<String>,
    ) -> Result<(), CameraMediaError> {
        self.broker
            .register_endpoint_at(self.clock.now_ms(), entity_id, kind, uri)
    }

    pub fn unregister_endpoint(&mut self, entity_id: &EntityId, kind: CameraMediaKind) -> bool {
        self.broker
            .unregister_endpoint_at(self.clock.now_ms(), entity_id, kind)
    }

    pub fn issue_lease(
        &mut self,
        runtime: &SmartHomeRuntime,
        request: CameraMediaAccessRequest,
    ) -> Result<CameraMediaLease, CameraMediaError> {
        let principal_id = self.authenticated_principal()?;
        self.broker.issue_lease_at(
            runtime,
            &principal_id,
            &mut self.nonce_source,
            request,
            self.clock.now_ms(),
        )
    }

    pub fn deliver_lease(
        &mut self,
        runtime: &SmartHomeRuntime,
        lease_id: &CameraMediaLeaseId,
    ) -> Result<CameraMediaDelivery, CameraMediaError> {
        let principal_id = self.authenticated_principal()?;
        let now_ms = self.clock.now_ms();
        let pending = self
            .broker
            .prepare_delivery(runtime, &principal_id, lease_id, now_ms)?;
        if self.pending_stream_cleanup.len() >= self.broker.policy.max_active_streams {
            return Err(self.broker.record_delivery_failure(
                &pending.lease,
                CameraMediaError::StreamQuotaExceeded {
                    maximum: self.broker.policy.max_active_streams,
                },
                now_ms,
            ));
        }
        if pending.lease.kind == CameraMediaKind::Stream
            && self.streams.len() >= self.broker.policy.max_active_streams
        {
            return Err(self.broker.record_delivery_failure(
                &pending.lease,
                CameraMediaError::StreamQuotaExceeded {
                    maximum: self.broker.policy.max_active_streams,
                },
                now_ms,
            ));
        }
        let execution = CameraMediaExecution {
            entity_id: &pending.lease.entity_id,
            kind: pending.lease.kind,
            endpoint_uri: pending.endpoint_uri.as_str(),
            expires_at_ms: pending.lease.expires_at_ms,
            max_snapshot_bytes: self.broker.policy.max_snapshot_bytes,
        };
        let result = match self.executor.deliver(execution) {
            Ok(result) => result,
            Err(error) => {
                return Err(self.broker.record_delivery_failure(
                    &pending.lease,
                    CameraMediaError::Execution(error),
                    now_ms,
                ));
            }
        };
        match (pending.lease.kind, result) {
            (CameraMediaKind::Snapshot, CameraMediaExecutionResult::Snapshot(bytes))
                if bytes.len() <= self.broker.policy.max_snapshot_bytes =>
            {
                self.broker.record_delivery_success(&pending.lease, now_ms);
                Ok(CameraMediaDelivery::Snapshot {
                    snapshot: CameraMediaSnapshot(bytes),
                })
            }
            (CameraMediaKind::Stream, CameraMediaExecutionResult::Stream(resource)) => {
                let mut nonce = [0u8; 16];
                if self.nonce_source.fill_nonce(&mut nonce).is_err() {
                    let error =
                        self.close_or_retain_stream(resource, CameraMediaError::NonceUnavailable);
                    return Err(self
                        .broker
                        .record_delivery_failure(&pending.lease, error, now_ms));
                }
                let session_id = stream_id_from_nonce(nonce);
                if self.streams.contains_key(&session_id) {
                    let error = self.close_or_retain_stream(
                        resource,
                        CameraMediaError::DuplicateStreamSessionId,
                    );
                    return Err(self
                        .broker
                        .record_delivery_failure(&pending.lease, error, now_ms));
                }
                let expires_at_ms = pending.lease.expires_at_ms;
                self.streams.insert(
                    session_id.clone(),
                    ActiveCameraMediaStream {
                        principal_id,
                        entity_id: pending.lease.entity_id.clone(),
                        kind: pending.lease.kind,
                        expires_at_ms,
                        resource,
                    },
                );
                self.broker.record_delivery_success(&pending.lease, now_ms);
                Ok(CameraMediaDelivery::Stream {
                    session_id,
                    expires_at_ms,
                })
            }
            (_, CameraMediaExecutionResult::Stream(resource)) => {
                let error =
                    self.close_or_retain_stream(resource, CameraMediaError::InvalidDelivery);
                Err(self
                    .broker
                    .record_delivery_failure(&pending.lease, error, now_ms))
            }
            _ => Err(self.broker.record_delivery_failure(
                &pending.lease,
                CameraMediaError::InvalidDelivery,
                now_ms,
            )),
        }
    }

    pub fn close_stream(
        &mut self,
        session_id: &CameraMediaStreamSessionId,
    ) -> Result<(), CameraMediaError> {
        let principal_id = self.authenticated_principal()?;
        let stream = self
            .streams
            .get(session_id)
            .ok_or(CameraMediaError::UnknownStreamSession)?;
        if stream.principal_id != principal_id {
            return Err(CameraMediaError::StreamPrincipalMismatch);
        }
        let close_result = {
            let stream = self
                .streams
                .get_mut(session_id)
                .expect("stream was checked above");
            self.executor.close_stream(&mut stream.resource)
        };
        close_result.map_err(CameraMediaError::Execution)?;
        self.streams.remove(session_id);
        Ok(())
    }

    /// Close expired streams and streams whose current grant was revoked.
    pub fn reconcile(&mut self, runtime: &SmartHomeRuntime) -> CameraMediaReconcileReport {
        let now_ms = self.clock.now_ms();
        let expired_leases = self.broker.expire_leases_at(now_ms);
        let expired_ids = self
            .streams
            .iter()
            .filter_map(|(session_id, stream)| {
                (now_ms >= stream.expires_at_ms
                    || authorize_camera_access(
                        runtime,
                        &stream.principal_id,
                        &stream.entity_id,
                        stream.kind,
                        now_ms,
                    )
                    .is_err())
                .then_some(session_id.clone())
            })
            .collect::<Vec<_>>();
        let mut closed_stream_count = 0usize;
        let mut failed_stream_close_count = 0usize;
        for session_id in &expired_ids {
            let close_result = self
                .streams
                .get_mut(session_id)
                .map(|stream| self.executor.close_stream(&mut stream.resource));
            match close_result {
                Some(Ok(())) => {
                    self.streams.remove(session_id);
                    closed_stream_count += 1;
                }
                Some(Err(_)) => failed_stream_close_count += 1,
                None => {}
            }
        }
        let mut cleanup_index = 0usize;
        while cleanup_index < self.pending_stream_cleanup.len() {
            match self
                .executor
                .close_stream(&mut self.pending_stream_cleanup[cleanup_index])
            {
                Ok(()) => {
                    self.pending_stream_cleanup.remove(cleanup_index);
                    closed_stream_count += 1;
                }
                Err(_) => {
                    failed_stream_close_count += 1;
                    cleanup_index += 1;
                }
            }
        }
        CameraMediaReconcileReport {
            expired_lease_count: expired_leases,
            closed_stream_count,
            failed_stream_close_count,
        }
    }

    pub fn audit_records(&self) -> impl Iterator<Item = &CameraMediaAuditRecord> {
        self.broker.audit_records()
    }

    pub fn snapshot(&self) -> CameraMediaBrokerSnapshot {
        self.broker
            .snapshot(self.streams.len(), self.pending_stream_cleanup.len())
    }

    fn close_or_retain_stream(
        &mut self,
        mut resource: Executor::Stream,
        operation_error: CameraMediaError,
    ) -> CameraMediaError {
        match self.executor.close_stream(&mut resource) {
            Ok(()) => operation_error,
            Err(close_error) => {
                self.pending_stream_cleanup.push(resource);
                CameraMediaError::Execution(close_error)
            }
        }
    }

    fn authenticated_principal(&self) -> Result<AgentId, CameraMediaError> {
        self.principal_source
            .current_principal()
            .ok_or(CameraMediaError::Unauthenticated)
    }
}

/// Minimal endpoint-registration surface used by native camera integrations.
pub trait CameraMediaEndpointRegistry {
    fn register_camera_endpoint(
        &mut self,
        entity_id: EntityId,
        kind: CameraMediaKind,
        uri: &str,
    ) -> Result<(), CameraMediaError>;
}

impl<Clock, Nonce, Principals, Executor> CameraMediaEndpointRegistry
    for CameraMediaService<Clock, Nonce, Principals, Executor>
where
    Clock: CameraMediaClock,
    Nonce: CameraMediaNonceSource,
    Principals: CameraMediaPrincipalSource,
    Executor: CameraMediaExecutor,
{
    fn register_camera_endpoint(
        &mut self,
        entity_id: EntityId,
        kind: CameraMediaKind,
        uri: &str,
    ) -> Result<(), CameraMediaError> {
        self.register_endpoint(entity_id, kind, uri)
    }
}

impl Default for CameraMediaBroker {
    fn default() -> Self {
        Self::new(CameraMediaPolicy::default())
    }
}

impl CameraMediaBroker {
    pub fn new(policy: CameraMediaPolicy) -> Self {
        Self {
            policy,
            endpoints: BTreeMap::new(),
            leases: BTreeMap::new(),
            audit: VecDeque::new(),
            next_endpoint_generation: 1,
            next_audit_sequence: 1,
            issued_lease_count: 0,
            denied_lease_count: 0,
            redeemed_lease_count: 0,
            failed_delivery_count: 0,
        }
    }

    /// Install or rotate one process-local endpoint.
    fn register_endpoint_at(
        &mut self,
        now_ms: u64,
        entity_id: EntityId,
        kind: CameraMediaKind,
        uri: impl Into<String>,
    ) -> Result<(), CameraMediaError> {
        let uri = Zeroizing::new(uri.into());
        validate_endpoint(uri.as_str(), kind, self.policy.allow_plaintext_loopback)?;
        let key = (entity_id.clone(), kind);
        if !self.endpoints.contains_key(&key) && self.endpoints.len() >= self.policy.max_endpoints {
            return Err(CameraMediaError::EndpointQuotaExceeded {
                maximum: self.policy.max_endpoints,
            });
        }
        let generation = self.next_endpoint_generation;
        self.next_endpoint_generation = generation
            .checked_add(1)
            .ok_or(CameraMediaError::EndpointGenerationOverflow)?;
        self.endpoints
            .insert(key, CameraMediaEndpoint { uri, generation });
        self.push_audit(CameraMediaAuditRecord {
            sequence: 0,
            principal_id: None,
            entity_id,
            kind,
            outcome: CameraMediaAuditOutcome::EndpointRegistered,
            reason: None,
            occurred_at_ms: now_ms,
        });
        Ok(())
    }

    /// Remove one endpoint without exposing it. Outstanding leases are left in
    /// the table so their next use is consumed and audited as a missing endpoint.
    fn unregister_endpoint_at(
        &mut self,
        now_ms: u64,
        entity_id: &EntityId,
        kind: CameraMediaKind,
    ) -> bool {
        if self.endpoints.remove(&(entity_id.clone(), kind)).is_none() {
            return false;
        }
        self.push_audit(CameraMediaAuditRecord {
            sequence: 0,
            principal_id: None,
            entity_id: entity_id.clone(),
            kind,
            outcome: CameraMediaAuditOutcome::EndpointRemoved,
            reason: None,
            occurred_at_ms: now_ms,
        });
        true
    }

    /// Authorize one future media delivery without exposing the endpoint.
    fn issue_lease_at(
        &mut self,
        runtime: &SmartHomeRuntime,
        principal_id: &AgentId,
        nonce_source: &mut impl CameraMediaNonceSource,
        request: CameraMediaAccessRequest,
        now_ms: u64,
    ) -> Result<CameraMediaLease, CameraMediaError> {
        self.expire_leases_at(now_ms);
        let result = self.issue_lease_inner(runtime, principal_id, nonce_source, &request, now_ms);
        match result {
            Ok(lease) => {
                self.issued_lease_count = self.issued_lease_count.saturating_add(1);
                self.push_audit(CameraMediaAuditRecord {
                    sequence: 0,
                    principal_id: Some(principal_id.clone()),
                    entity_id: lease.entity_id.clone(),
                    kind: lease.kind,
                    outcome: CameraMediaAuditOutcome::LeaseIssued,
                    reason: None,
                    occurred_at_ms: now_ms,
                });
                Ok(lease)
            }
            Err(error) => {
                self.denied_lease_count = self.denied_lease_count.saturating_add(1);
                self.push_audit(CameraMediaAuditRecord {
                    sequence: 0,
                    principal_id: Some(principal_id.clone()),
                    entity_id: request.entity_id,
                    kind: request.kind,
                    outcome: CameraMediaAuditOutcome::LeaseDenied,
                    reason: Some(error.to_string()),
                    occurred_at_ms: now_ms,
                });
                Err(error)
            }
        }
    }

    fn issue_lease_inner(
        &mut self,
        runtime: &SmartHomeRuntime,
        principal_id: &AgentId,
        nonce_source: &mut impl CameraMediaNonceSource,
        request: &CameraMediaAccessRequest,
        now_ms: u64,
    ) -> Result<CameraMediaLease, CameraMediaError> {
        if request.purpose.trim().is_empty() {
            return Err(CameraMediaError::EmptyPurpose);
        }
        let maximum_ms = self.policy.max_ttl_ms(request.kind);
        if request.ttl_ms == 0 || request.ttl_ms > maximum_ms {
            return Err(CameraMediaError::InvalidTtl {
                requested_ms: request.ttl_ms,
                maximum_ms,
            });
        }
        if self.leases.len() >= self.policy.max_active_leases {
            return Err(CameraMediaError::LeaseQuotaExceeded {
                maximum: self.policy.max_active_leases,
            });
        }
        if self
            .leases
            .values()
            .filter(|lease| lease.principal_id == *principal_id)
            .count()
            >= self.policy.max_active_leases_per_principal
        {
            return Err(CameraMediaError::PrincipalLeaseQuotaExceeded {
                maximum: self.policy.max_active_leases_per_principal,
            });
        }
        let grant_expiry = authorize_camera_access(
            runtime,
            principal_id,
            &request.entity_id,
            request.kind,
            now_ms,
        )?;
        let endpoint_generation = self
            .endpoints
            .get(&(request.entity_id.clone(), request.kind))
            .ok_or_else(|| CameraMediaError::MissingEndpoint {
                entity_id: request.entity_id.clone(),
                kind: request.kind,
            })?
            .generation;
        let requested_expiry = now_ms
            .checked_add(request.ttl_ms)
            .ok_or(CameraMediaError::TimestampOverflow)?;
        let expires_at_ms = grant_expiry.map_or(requested_expiry, |grant_expiry| {
            requested_expiry.min(grant_expiry)
        });
        let mut nonce = [0u8; 16];
        nonce_source
            .fill_nonce(&mut nonce)
            .map_err(|_| CameraMediaError::NonceUnavailable)?;
        let lease_id = lease_id_from_nonce(nonce);
        if self.leases.contains_key(&lease_id) {
            return Err(CameraMediaError::DuplicateLeaseId);
        }
        let lease = CameraMediaLease {
            lease_id,
            principal_id: principal_id.clone(),
            entity_id: request.entity_id.clone(),
            kind: request.kind,
            issued_at_ms: now_ms,
            expires_at_ms,
            endpoint_generation,
        };
        self.leases.insert(lease.lease_id.clone(), lease.clone());
        Ok(lease)
    }

    /// Atomically validate and consume one lease before external execution.
    fn prepare_delivery(
        &mut self,
        runtime: &SmartHomeRuntime,
        principal_id: &AgentId,
        lease_id: &CameraMediaLeaseId,
        now_ms: u64,
    ) -> Result<PendingCameraMediaExecution, CameraMediaError> {
        let Some(lease) = self.leases.get(lease_id).cloned() else {
            return Err(CameraMediaError::UnknownLease);
        };
        if lease.principal_id != *principal_id {
            return Err(CameraMediaError::LeasePrincipalMismatch);
        }
        if lease.is_expired_at(now_ms) {
            self.leases.remove(lease_id);
            self.push_lease_audit(&lease, CameraMediaAuditOutcome::LeaseExpired, None, now_ms);
            return Err(CameraMediaError::ExpiredLease);
        }
        if let Err(error) =
            authorize_camera_access(runtime, principal_id, &lease.entity_id, lease.kind, now_ms)
        {
            self.leases.remove(lease_id);
            self.push_lease_audit(
                &lease,
                CameraMediaAuditOutcome::LeaseRejected,
                Some(error.to_string()),
                now_ms,
            );
            return Err(error);
        }
        let endpoint = match self.endpoints.get(&(lease.entity_id.clone(), lease.kind)) {
            Some(endpoint) => endpoint,
            None => {
                self.leases.remove(lease_id);
                let error = CameraMediaError::MissingEndpoint {
                    entity_id: lease.entity_id.clone(),
                    kind: lease.kind,
                };
                self.push_lease_audit(
                    &lease,
                    CameraMediaAuditOutcome::LeaseRejected,
                    Some(error.to_string()),
                    now_ms,
                );
                return Err(error);
            }
        };
        if endpoint.generation != lease.endpoint_generation {
            self.leases.remove(lease_id);
            self.push_lease_audit(
                &lease,
                CameraMediaAuditOutcome::LeaseRejected,
                Some(CameraMediaError::EndpointGenerationChanged.to_string()),
                now_ms,
            );
            return Err(CameraMediaError::EndpointGenerationChanged);
        }

        let endpoint_uri = Zeroizing::new(endpoint.uri.as_str().to_owned());
        // Consume before the external effect. A failing host cannot replay the
        // same bearer lease and accidentally duplicate media delivery.
        self.leases.remove(lease_id);
        Ok(PendingCameraMediaExecution {
            lease,
            endpoint_uri,
        })
    }

    fn record_delivery_success(&mut self, lease: &CameraMediaLease, now_ms: u64) {
        self.redeemed_lease_count = self.redeemed_lease_count.saturating_add(1);
        self.push_lease_audit(lease, CameraMediaAuditOutcome::LeaseDelivered, None, now_ms);
    }

    fn record_delivery_failure(
        &mut self,
        lease: &CameraMediaLease,
        error: CameraMediaError,
        now_ms: u64,
    ) -> CameraMediaError {
        self.failed_delivery_count = self.failed_delivery_count.saturating_add(1);
        self.push_lease_audit(
            lease,
            CameraMediaAuditOutcome::DeliveryFailed,
            Some(error.to_string()),
            now_ms,
        );
        error
    }

    fn expire_leases_at(&mut self, now_ms: u64) -> usize {
        let expired = self
            .leases
            .values()
            .filter(|lease| lease.is_expired_at(now_ms))
            .cloned()
            .collect::<Vec<_>>();
        for lease in &expired {
            self.leases.remove(&lease.lease_id);
            self.push_lease_audit(lease, CameraMediaAuditOutcome::LeaseExpired, None, now_ms);
        }
        expired.len()
    }

    fn audit_records(&self) -> impl Iterator<Item = &CameraMediaAuditRecord> {
        self.audit.iter()
    }

    fn snapshot(
        &self,
        active_stream_count: usize,
        pending_stream_cleanup_count: usize,
    ) -> CameraMediaBrokerSnapshot {
        CameraMediaBrokerSnapshot {
            endpoint_count: self.endpoints.len(),
            active_lease_count: self.leases.len(),
            active_stream_count,
            pending_stream_cleanup_count,
            audit_record_count: self.audit.len(),
            issued_lease_count: self.issued_lease_count,
            denied_lease_count: self.denied_lease_count,
            redeemed_lease_count: self.redeemed_lease_count,
            failed_delivery_count: self.failed_delivery_count,
        }
    }

    fn push_lease_audit(
        &mut self,
        lease: &CameraMediaLease,
        outcome: CameraMediaAuditOutcome,
        reason: Option<String>,
        occurred_at_ms: u64,
    ) {
        self.push_audit(CameraMediaAuditRecord {
            sequence: 0,
            principal_id: Some(lease.principal_id.clone()),
            entity_id: lease.entity_id.clone(),
            kind: lease.kind,
            outcome,
            reason,
            occurred_at_ms,
        });
    }

    fn push_audit(&mut self, mut record: CameraMediaAuditRecord) {
        record.sequence = self.next_audit_sequence;
        self.next_audit_sequence = self.next_audit_sequence.saturating_add(1);
        self.audit.push_back(record);
        let limit = self.policy.max_audit_records.max(1);
        while self.audit.len() > limit {
            self.audit.pop_front();
        }
    }
}

fn authorize_camera_access(
    runtime: &SmartHomeRuntime,
    principal_id: &AgentId,
    entity_id: &EntityId,
    kind: CameraMediaKind,
    now_ms: u64,
) -> Result<Option<u64>, CameraMediaError> {
    let capability_id = kind.capability_id();
    let entity = runtime
        .registry()
        .entity(entity_id)
        .ok_or_else(|| CameraMediaError::UnknownEntity(entity_id.clone()))?;
    let capability = entity
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == capability_id)
        .ok_or_else(|| CameraMediaError::MissingCapability {
            entity_id: entity_id.clone(),
            capability_id: capability_id.clone(),
        })?;
    if !matches!(
        capability.mode,
        CapabilityMode::Command | CapabilityMode::ObserveAndCommand
    ) {
        return Err(CameraMediaError::ReadOnlyCapability {
            entity_id: entity_id.clone(),
            capability_id,
        });
    }

    let mut found = false;
    let mut latest_expiry = Some(0u64);
    for grant in runtime
        .registry()
        .capability_grants_for_principal(principal_id)
    {
        if grant_covers_camera_access(grant, principal_id, entity_id, &capability_id, now_ms) {
            found = true;
            match grant.expires_at_ms {
                None => return Ok(None),
                Some(expiry) => {
                    latest_expiry = Some(latest_expiry.unwrap_or_default().max(expiry));
                }
            }
        }
    }
    if found {
        Ok(latest_expiry)
    } else {
        Err(CameraMediaError::Unauthorized {
            principal_id: principal_id.clone(),
            entity_id: entity_id.clone(),
            capability_id,
        })
    }
}

fn grant_covers_camera_access(
    grant: &CapabilityGrant,
    principal_id: &AgentId,
    entity_id: &EntityId,
    capability_id: &CapabilityId,
    now_ms: u64,
) -> bool {
    grant.principal_id == *principal_id
        && grant.is_active_at(now_ms)
        && grant.max_tier >= PrivilegeTier::HumanApproval
        && match &grant.scope {
            CapabilityGrantScope::Capability(granted) => granted == capability_id,
            CapabilityGrantScope::EntityCapability {
                entity_id: granted_entity,
                capability_id: granted,
            } => granted_entity == entity_id && granted == capability_id,
            CapabilityGrantScope::AllSmartHome => true,
            CapabilityGrantScope::Tool(_) => false,
        }
}

fn validate_endpoint(
    uri: &str,
    kind: CameraMediaKind,
    allow_plaintext_loopback: bool,
) -> Result<(), CameraMediaError> {
    let url =
        Url::parse(uri).map_err(|error| CameraMediaError::InvalidEndpoint(error.to_string()))?;
    if url.userinfo.is_some() {
        return Err(CameraMediaError::EndpointCredentialsForbidden);
    }
    if url.fragment.is_some() {
        return Err(CameraMediaError::InvalidEndpoint(
            "fragments are not allowed".to_string(),
        ));
    }
    let host = url
        .host
        .as_deref()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| CameraMediaError::InvalidEndpoint("missing host".to_string()))?;
    let scheme_allowed = match kind {
        CameraMediaKind::Snapshot => matches!(url.scheme.as_str(), "http" | "https"),
        CameraMediaKind::Stream => {
            matches!(url.scheme.as_str(), "rtsp" | "rtsps" | "http" | "https")
        }
    };
    if !scheme_allowed {
        return Err(CameraMediaError::UnsupportedEndpointScheme(url.scheme));
    }
    let secure = matches!(url.scheme.as_str(), "https" | "rtsps");
    if !secure && url.query.is_some() {
        return Err(CameraMediaError::InsecureEndpoint);
    }
    if !secure && !(allow_plaintext_loopback && is_loopback_host(host)) {
        return Err(CameraMediaError::InsecureEndpoint);
    }
    Ok(())
}

fn is_loopback_host(host: &str) -> bool {
    host == "localhost"
        || host == "[::1]"
        || host == "127.0.0.1"
        || host.strip_prefix("127.").is_some_and(|suffix| {
            let octets = suffix.split('.').collect::<Vec<_>>();
            octets.len() == 3
                && octets
                    .iter()
                    .all(|octet| !octet.is_empty() && octet.parse::<u8>().is_ok())
        })
}

fn lease_id_from_nonce(bytes: [u8; 16]) -> CameraMediaLeaseId {
    CameraMediaLeaseId(Zeroizing::new(format!(
        "{:016x}{:016x}",
        u64::from_be_bytes(bytes[..8].try_into().expect("eight bytes")),
        u64::from_be_bytes(bytes[8..].try_into().expect("eight bytes"))
    )))
}

fn stream_id_from_nonce(bytes: [u8; 16]) -> CameraMediaStreamSessionId {
    CameraMediaStreamSessionId(Zeroizing::new(format!(
        "{:016x}{:016x}",
        u64::from_be_bytes(bytes[..8].try_into().expect("eight bytes")),
        u64::from_be_bytes(bytes[8..].try_into().expect("eight bytes"))
    )))
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn endpoint_generation_overflow_fails_before_installation() {
        let mut broker = CameraMediaBroker {
            next_endpoint_generation: u64::MAX,
            ..CameraMediaBroker::default()
        };
        assert_eq!(
            broker.register_endpoint_at(
                7,
                EntityId::trusted("camera"),
                CameraMediaKind::Snapshot,
                "https://camera.local/snapshot.jpg",
            ),
            Err(CameraMediaError::EndpointGenerationOverflow)
        );
        assert_eq!(broker.snapshot(0, 0).endpoint_count, 0);
    }

    #[test]
    fn plaintext_loopback_detection_rejects_hostname_confusion() {
        assert!(is_loopback_host("127.1.2.3"));
        assert!(!is_loopback_host("127.evil.local.com"));
        assert!(!is_loopback_host("127.999.0.1"));
    }
}
