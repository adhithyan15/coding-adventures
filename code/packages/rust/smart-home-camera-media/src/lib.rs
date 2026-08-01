//! Privacy-preserving camera snapshot and stream leases for D23.

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroizing;
use rand::{rngs::OsRng, RngCore};
use smart_home_core::{
    AgentId, CapabilityGrant, CapabilityGrantScope, CapabilityId, CapabilityMode, EntityId,
    PrivilegeTier,
};
use smart_home_runtime::SmartHomeRuntime;
use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use url_parser::Url;

pub const VERSION: &str = "0.1.0";
pub const SNAPSHOT_CAPABILITY_ID: &str = "camera.snapshot";
pub const STREAM_CAPABILITY_ID: &str = "camera.stream";
pub const DEFAULT_MAX_AUDIT_RECORDS: usize = 256;

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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CameraMediaLeaseId(String);

impl CameraMediaLeaseId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for CameraMediaLeaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("CameraMediaLeaseId")
            .field(&self.0)
            .finish()
    }
}

impl fmt::Display for CameraMediaLeaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaPolicy {
    pub max_snapshot_ttl_ms: u64,
    pub max_stream_ttl_ms: u64,
    pub max_audit_records: usize,
}

impl Default for CameraMediaPolicy {
    fn default() -> Self {
        Self {
            max_snapshot_ttl_ms: 30_000,
            max_stream_ttl_ms: 60_000,
            max_audit_records: DEFAULT_MAX_AUDIT_RECORDS,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaAccessRequest {
    pub principal_id: AgentId,
    pub entity_id: EntityId,
    pub kind: CameraMediaKind,
    pub purpose: String,
    pub requested_at_ms: u64,
    pub ttl_ms: u64,
}

impl CameraMediaAccessRequest {
    pub fn new(
        principal_id: AgentId,
        entity_id: EntityId,
        kind: CameraMediaKind,
        purpose: impl Into<String>,
        requested_at_ms: u64,
        ttl_ms: u64,
    ) -> Self {
        Self {
            principal_id,
            entity_id,
            kind,
            purpose: purpose.into(),
            requested_at_ms,
            ttl_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaLease {
    pub lease_id: CameraMediaLeaseId,
    pub principal_id: AgentId,
    pub entity_id: EntityId,
    pub kind: CameraMediaKind,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

impl CameraMediaLease {
    pub fn is_expired_at(&self, now_ms: u64) -> bool {
        now_ms >= self.expires_at_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMediaAuditOutcome {
    EndpointRegistered,
    LeaseIssued,
    LeaseDenied,
    LeaseRedeemed,
    LeaseExpired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaAuditRecord {
    pub sequence: u64,
    pub principal_id: Option<AgentId>,
    pub entity_id: EntityId,
    pub kind: CameraMediaKind,
    pub lease_id: Option<CameraMediaLeaseId>,
    pub outcome: CameraMediaAuditOutcome,
    pub reason: Option<String>,
    pub occurred_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraMediaBrokerSnapshot {
    pub endpoint_count: usize,
    pub active_lease_count: usize,
    pub audit_record_count: usize,
    pub issued_lease_count: u64,
    pub denied_lease_count: u64,
    pub redeemed_lease_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CameraMediaError {
    InvalidEndpoint(String),
    UnsupportedEndpointScheme(String),
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
    Unauthorized {
        principal_id: AgentId,
        entity_id: EntityId,
        capability_id: CapabilityId,
    },
    UnknownLease(CameraMediaLeaseId),
    LeasePrincipalMismatch(CameraMediaLeaseId),
    ExpiredLease(CameraMediaLeaseId),
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
            Self::MissingEndpoint { entity_id, kind } => {
                write!(
                    formatter,
                    "camera entity {entity_id} has no {} endpoint",
                    kind.as_str()
                )
            }
            Self::EmptyPurpose => formatter.write_str("camera access purpose must not be empty"),
            Self::InvalidTtl {
                requested_ms,
                maximum_ms,
            } => write!(
                formatter,
                "camera lease TTL {requested_ms}ms exceeds the allowed 1..={maximum_ms}ms range"
            ),
            Self::Unauthorized {
                principal_id,
                entity_id,
                capability_id,
            } => write!(
                formatter,
                "principal {principal_id} is not authorized for {capability_id} on {entity_id}"
            ),
            Self::UnknownLease(lease_id) => {
                write!(formatter, "unknown camera media lease {lease_id}")
            }
            Self::LeasePrincipalMismatch(lease_id) => {
                write!(
                    formatter,
                    "camera media lease {lease_id} belongs to another principal"
                )
            }
            Self::ExpiredLease(lease_id) => {
                write!(formatter, "camera media lease {lease_id} expired")
            }
        }
    }
}

impl std::error::Error for CameraMediaError {}

struct CameraMediaEndpoint {
    uri: Zeroizing<String>,
}

impl fmt::Debug for CameraMediaEndpoint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CameraMediaEndpoint([REDACTED])")
    }
}

#[derive(Debug)]
pub struct CameraMediaBroker {
    policy: CameraMediaPolicy,
    endpoints: BTreeMap<(EntityId, CameraMediaKind), CameraMediaEndpoint>,
    leases: BTreeMap<CameraMediaLeaseId, CameraMediaLease>,
    audit: VecDeque<CameraMediaAuditRecord>,
    next_audit_sequence: u64,
    issued_lease_count: u64,
    denied_lease_count: u64,
    redeemed_lease_count: u64,
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
            next_audit_sequence: 1,
            issued_lease_count: 0,
            denied_lease_count: 0,
            redeemed_lease_count: 0,
        }
    }

    pub fn register_endpoint(
        &mut self,
        entity_id: EntityId,
        kind: CameraMediaKind,
        uri: impl Into<String>,
        registered_at_ms: u64,
    ) -> Result<(), CameraMediaError> {
        let uri = uri.into();
        validate_endpoint(&uri, kind)?;
        self.endpoints.insert(
            (entity_id.clone(), kind),
            CameraMediaEndpoint {
                uri: Zeroizing::new(uri),
            },
        );
        self.push_audit(CameraMediaAuditRecord {
            sequence: 0,
            principal_id: None,
            entity_id,
            kind,
            lease_id: None,
            outcome: CameraMediaAuditOutcome::EndpointRegistered,
            reason: None,
            occurred_at_ms: registered_at_ms,
        });
        Ok(())
    }

    pub fn issue_lease(
        &mut self,
        runtime: &SmartHomeRuntime,
        request: CameraMediaAccessRequest,
    ) -> Result<CameraMediaLease, CameraMediaError> {
        let capability_id = request.kind.capability_id();
        let result = self.validate_request(runtime, &request, &capability_id);
        if let Err(error) = result {
            self.denied_lease_count = self.denied_lease_count.saturating_add(1);
            self.push_audit(CameraMediaAuditRecord {
                sequence: 0,
                principal_id: Some(request.principal_id.clone()),
                entity_id: request.entity_id.clone(),
                kind: request.kind,
                lease_id: None,
                outcome: CameraMediaAuditOutcome::LeaseDenied,
                reason: Some(error.to_string()),
                occurred_at_ms: request.requested_at_ms,
            });
            return Err(error);
        }

        let expires_at_ms = request.requested_at_ms.saturating_add(request.ttl_ms);
        let lease = CameraMediaLease {
            lease_id: random_lease_id(),
            principal_id: request.principal_id,
            entity_id: request.entity_id,
            kind: request.kind,
            issued_at_ms: request.requested_at_ms,
            expires_at_ms,
        };
        self.leases.insert(lease.lease_id.clone(), lease.clone());
        self.issued_lease_count = self.issued_lease_count.saturating_add(1);
        self.push_audit(CameraMediaAuditRecord {
            sequence: 0,
            principal_id: Some(lease.principal_id.clone()),
            entity_id: lease.entity_id.clone(),
            kind: lease.kind,
            lease_id: Some(lease.lease_id.clone()),
            outcome: CameraMediaAuditOutcome::LeaseIssued,
            reason: None,
            occurred_at_ms: lease.issued_at_ms,
        });
        Ok(lease)
    }

    pub fn redeem_lease(
        &mut self,
        lease_id: &CameraMediaLeaseId,
        principal_id: &AgentId,
        now_ms: u64,
    ) -> Result<Zeroizing<String>, CameraMediaError> {
        let Some(lease) = self.leases.get(lease_id).cloned() else {
            return Err(CameraMediaError::UnknownLease(lease_id.clone()));
        };
        if lease.principal_id != *principal_id {
            return Err(CameraMediaError::LeasePrincipalMismatch(lease_id.clone()));
        }
        if lease.is_expired_at(now_ms) {
            self.leases.remove(lease_id);
            self.push_audit(CameraMediaAuditRecord {
                sequence: 0,
                principal_id: Some(principal_id.clone()),
                entity_id: lease.entity_id,
                kind: lease.kind,
                lease_id: Some(lease_id.clone()),
                outcome: CameraMediaAuditOutcome::LeaseExpired,
                reason: None,
                occurred_at_ms: now_ms,
            });
            return Err(CameraMediaError::ExpiredLease(lease_id.clone()));
        }
        let endpoint = self
            .endpoints
            .get(&(lease.entity_id.clone(), lease.kind))
            .ok_or_else(|| CameraMediaError::MissingEndpoint {
                entity_id: lease.entity_id.clone(),
                kind: lease.kind,
            })?;
        let secret = Zeroizing::new(endpoint.uri.to_string());
        self.leases.remove(lease_id);
        self.redeemed_lease_count = self.redeemed_lease_count.saturating_add(1);
        self.push_audit(CameraMediaAuditRecord {
            sequence: 0,
            principal_id: Some(principal_id.clone()),
            entity_id: lease.entity_id,
            kind: lease.kind,
            lease_id: Some(lease_id.clone()),
            outcome: CameraMediaAuditOutcome::LeaseRedeemed,
            reason: None,
            occurred_at_ms: now_ms,
        });
        Ok(secret)
    }

    pub fn expire_leases(&mut self, now_ms: u64) -> usize {
        let expired = self
            .leases
            .values()
            .filter(|lease| lease.is_expired_at(now_ms))
            .cloned()
            .collect::<Vec<_>>();
        for lease in &expired {
            self.leases.remove(&lease.lease_id);
            self.push_audit(CameraMediaAuditRecord {
                sequence: 0,
                principal_id: Some(lease.principal_id.clone()),
                entity_id: lease.entity_id.clone(),
                kind: lease.kind,
                lease_id: Some(lease.lease_id.clone()),
                outcome: CameraMediaAuditOutcome::LeaseExpired,
                reason: None,
                occurred_at_ms: now_ms,
            });
        }
        expired.len()
    }

    pub fn audit_records(&self) -> impl Iterator<Item = &CameraMediaAuditRecord> {
        self.audit.iter()
    }

    pub fn snapshot(&self) -> CameraMediaBrokerSnapshot {
        CameraMediaBrokerSnapshot {
            endpoint_count: self.endpoints.len(),
            active_lease_count: self.leases.len(),
            audit_record_count: self.audit.len(),
            issued_lease_count: self.issued_lease_count,
            denied_lease_count: self.denied_lease_count,
            redeemed_lease_count: self.redeemed_lease_count,
        }
    }

    fn validate_request(
        &self,
        runtime: &SmartHomeRuntime,
        request: &CameraMediaAccessRequest,
        capability_id: &CapabilityId,
    ) -> Result<(), CameraMediaError> {
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
        let entity = runtime
            .registry()
            .entity(&request.entity_id)
            .ok_or_else(|| CameraMediaError::UnknownEntity(request.entity_id.clone()))?;
        let capability = entity
            .capabilities
            .iter()
            .find(|capability| capability.capability_id == *capability_id)
            .ok_or_else(|| CameraMediaError::MissingCapability {
                entity_id: request.entity_id.clone(),
                capability_id: capability_id.clone(),
            })?;
        if !matches!(
            capability.mode,
            CapabilityMode::Command | CapabilityMode::ObserveAndCommand
        ) {
            return Err(CameraMediaError::ReadOnlyCapability {
                entity_id: request.entity_id.clone(),
                capability_id: capability_id.clone(),
            });
        }
        if !self
            .endpoints
            .contains_key(&(request.entity_id.clone(), request.kind))
        {
            return Err(CameraMediaError::MissingEndpoint {
                entity_id: request.entity_id.clone(),
                kind: request.kind,
            });
        }
        let authorized = runtime
            .registry()
            .capability_grants_for_principal(&request.principal_id)
            .into_iter()
            .any(|grant| grant_covers_camera_access(grant, request, capability_id));
        if !authorized {
            return Err(CameraMediaError::Unauthorized {
                principal_id: request.principal_id.clone(),
                entity_id: request.entity_id.clone(),
                capability_id: capability_id.clone(),
            });
        }
        Ok(())
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

fn grant_covers_camera_access(
    grant: &CapabilityGrant,
    request: &CameraMediaAccessRequest,
    capability_id: &CapabilityId,
) -> bool {
    grant.principal_id == request.principal_id
        && grant.is_active_at(request.requested_at_ms)
        && grant.max_tier >= PrivilegeTier::HumanApproval
        && match &grant.scope {
            CapabilityGrantScope::Capability(granted) => granted == capability_id,
            CapabilityGrantScope::EntityCapability {
                entity_id,
                capability_id: granted,
            } => entity_id == &request.entity_id && granted == capability_id,
            CapabilityGrantScope::AllSmartHome => true,
            CapabilityGrantScope::Tool(_) => false,
        }
}

fn validate_endpoint(uri: &str, kind: CameraMediaKind) -> Result<(), CameraMediaError> {
    let url =
        Url::parse(uri).map_err(|error| CameraMediaError::InvalidEndpoint(error.to_string()))?;
    let allowed = match kind {
        CameraMediaKind::Snapshot => matches!(url.scheme.as_str(), "http" | "https"),
        CameraMediaKind::Stream => {
            matches!(url.scheme.as_str(), "rtsp" | "rtsps" | "http" | "https")
        }
    };
    if !allowed {
        return Err(CameraMediaError::UnsupportedEndpointScheme(url.scheme));
    }
    if url.host.as_deref().is_none_or(str::is_empty) {
        return Err(CameraMediaError::InvalidEndpoint(
            "camera endpoint is missing a host".to_string(),
        ));
    }
    Ok(())
}

fn random_lease_id() -> CameraMediaLeaseId {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    CameraMediaLeaseId(format!(
        "camera-media:{:016x}{:016x}",
        u64::from_be_bytes(bytes[..8].try_into().expect("eight bytes")),
        u64::from_be_bytes(bytes[8..].try_into().expect("eight bytes"))
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_home_core::{
        Bridge, BridgeId, BridgeTransport, Capability, CapabilityGrantId, Device, DeviceId, Entity,
        EntityKind, Health, IntegrationId, Metadata, ProtocolIdentifier, ValueKind,
    };

    fn fixture_runtime() -> (SmartHomeRuntime, EntityId, AgentId) {
        let mut runtime = SmartHomeRuntime::default();
        let bridge_id = BridgeId::trusted("camera-bridge");
        let device_id = DeviceId::trusted("camera-device");
        let entity_id = EntityId::trusted("camera-entity");
        let principal_id = AgentId::trusted("dashboard-user");
        let mut bridge = Bridge::new(
            bridge_id.clone(),
            IntegrationId::trusted("onvif"),
            BridgeTransport::LanHttp,
        );
        bridge.health = Health::Online;
        runtime.upsert_bridge(bridge).unwrap();
        runtime
            .upsert_device(Device {
                device_id: device_id.clone(),
                bridge_id,
                manufacturer: "Fixture".to_string(),
                model: "Camera".to_string(),
                name: "Front Door".to_string(),
                serial: Some("fixture-1".to_string()),
                firmware_version: None,
                room_id: None,
                entity_ids: vec![entity_id.clone()],
                identifiers: Vec::<ProtocolIdentifier>::new(),
                health: Health::Online,
                metadata: Vec::<Metadata>::new(),
            })
            .unwrap();
        runtime
            .upsert_entity(Entity {
                entity_id: entity_id.clone(),
                device_id,
                kind: EntityKind::Camera,
                name: "Front Door".to_string(),
                capabilities: vec![
                    Capability::new(
                        CameraMediaKind::Snapshot.capability_id(),
                        CapabilityMode::Command,
                        ValueKind::Text,
                    ),
                    Capability::new(
                        CameraMediaKind::Stream.capability_id(),
                        CapabilityMode::Command,
                        ValueKind::Text,
                    ),
                ],
                state: None,
                metadata: Vec::new(),
            })
            .unwrap();
        runtime
            .registry_mut()
            .upsert_capability_grant(CapabilityGrant::for_entity_capability(
                CapabilityGrantId::trusted("camera-snapshot-grant"),
                principal_id.clone(),
                entity_id.clone(),
                CameraMediaKind::Snapshot.capability_id(),
                PrivilegeTier::HumanApproval,
                "user",
                1,
            ));
        (runtime, entity_id, principal_id)
    }

    #[test]
    fn authorized_snapshot_lease_is_short_lived_single_use_and_redacted() {
        let (runtime, entity_id, principal_id) = fixture_runtime();
        let mut broker = CameraMediaBroker::default();
        let secret_uri = "http://camera.local/snapshot.jpg?token=secret";
        broker
            .register_endpoint(entity_id.clone(), CameraMediaKind::Snapshot, secret_uri, 10)
            .unwrap();
        assert!(!format!("{broker:?}").contains(secret_uri));

        let lease = broker
            .issue_lease(
                &runtime,
                CameraMediaAccessRequest::new(
                    principal_id.clone(),
                    entity_id,
                    CameraMediaKind::Snapshot,
                    "operator preview",
                    20,
                    5_000,
                ),
            )
            .unwrap();
        let endpoint = broker
            .redeem_lease(&lease.lease_id, &principal_id, 21)
            .unwrap();
        assert_eq!(endpoint.as_str(), secret_uri);
        assert!(matches!(
            broker.redeem_lease(&lease.lease_id, &principal_id, 22),
            Err(CameraMediaError::UnknownLease(_))
        ));
        assert_eq!(broker.snapshot().redeemed_lease_count, 1);
        assert!(broker
            .audit_records()
            .all(|record| !format!("{record:?}").contains(secret_uri)));
    }

    #[test]
    fn stream_requires_its_own_human_approval_grant() {
        let (runtime, entity_id, principal_id) = fixture_runtime();
        let mut broker = CameraMediaBroker::default();
        broker
            .register_endpoint(
                entity_id.clone(),
                CameraMediaKind::Stream,
                "rtsp://camera.local/live",
                10,
            )
            .unwrap();
        let error = broker
            .issue_lease(
                &runtime,
                CameraMediaAccessRequest::new(
                    principal_id,
                    entity_id,
                    CameraMediaKind::Stream,
                    "live view",
                    20,
                    10_000,
                ),
            )
            .unwrap_err();
        assert!(matches!(error, CameraMediaError::Unauthorized { .. }));
        assert_eq!(broker.snapshot().denied_lease_count, 1);
    }

    #[test]
    fn expired_lease_cannot_reveal_endpoint() {
        let (runtime, entity_id, principal_id) = fixture_runtime();
        let mut broker = CameraMediaBroker::default();
        broker
            .register_endpoint(
                entity_id.clone(),
                CameraMediaKind::Snapshot,
                "https://camera.local/snapshot.jpg",
                10,
            )
            .unwrap();
        let lease = broker
            .issue_lease(
                &runtime,
                CameraMediaAccessRequest::new(
                    principal_id.clone(),
                    entity_id,
                    CameraMediaKind::Snapshot,
                    "preview",
                    20,
                    1,
                ),
            )
            .unwrap();
        assert!(matches!(
            broker.redeem_lease(&lease.lease_id, &principal_id, 21),
            Err(CameraMediaError::ExpiredLease(_))
        ));
    }
}
