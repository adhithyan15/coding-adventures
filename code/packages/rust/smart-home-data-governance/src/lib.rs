//! Bounded privacy, consent, identifier-use, and telemetry-egress policy for D23 integrations.

#![forbid(unsafe_code)]

use smart_home_core::AgentId;
use std::fmt;
use url_parser::Url;

pub const DEFAULT_MAX_GRANTS: usize = 128;
pub const MAX_RESOURCE_BYTES: usize = 256;
pub const MAX_PURPOSE_BYTES: usize = 256;
pub const MAX_CONSENT_REFERENCE_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataCategory {
    CoarseLocation,
    DeviceIdentifier,
    EnvironmentalTelemetry,
    OperationalTelemetry,
    Presence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataOperation {
    Configure,
    Inspect,
    StartEgress,
    StopEgress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataRetention {
    NotApplicable,
    Ephemeral,
    Bounded { maximum_age_ms: u64 },
}

#[derive(Clone, PartialEq, Eq)]
pub enum DataDestination {
    LocalDevice,
    HttpsOrigin(String),
    MqttBroker(String),
}

impl fmt::Debug for DataDestination {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalDevice => formatter.write_str("LocalDevice"),
            Self::HttpsOrigin(_) => formatter.write_str("HttpsOrigin([REDACTED])"),
            Self::MqttBroker(_) => formatter.write_str("MqttBroker([REDACTED])"),
        }
    }
}

impl DataDestination {
    pub fn https_origin(origin: impl Into<String>) -> Result<Self, DataGovernanceError> {
        let origin = origin.into();
        let parsed = Url::parse(&origin).map_err(|_| DataGovernanceError::InvalidDestination)?;
        if parsed.scheme != "https"
            || parsed.host.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
            || has_unsafe_text(&origin)
        {
            return Err(DataGovernanceError::InvalidDestination);
        }
        Ok(Self::HttpsOrigin(origin.trim_end_matches('/').to_string()))
    }

    pub fn mqtt_broker(uri: impl Into<String>) -> Result<Self, DataGovernanceError> {
        let uri = uri.into();
        let parsed = Url::parse(&uri).map_err(|_| DataGovernanceError::InvalidDestination)?;
        if !matches!(parsed.scheme.as_str(), "mqtt" | "mqtts")
            || parsed.host.is_none()
            || parsed.port.is_none()
            || parsed.userinfo.is_some()
            || parsed.query.is_some()
            || parsed.fragment.is_some()
            || !matches!(parsed.path.as_str(), "" | "/")
            || has_unsafe_text(&uri)
        {
            return Err(DataGovernanceError::InvalidDestination);
        }
        Ok(Self::MqttBroker(uri.trim_end_matches('/').to_string()))
    }

    pub fn kind(&self) -> DataDestinationKind {
        match self {
            Self::LocalDevice => DataDestinationKind::LocalDevice,
            Self::HttpsOrigin(_) => DataDestinationKind::HttpsOrigin,
            Self::MqttBroker(_) => DataDestinationKind::MqttBroker,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataDestinationKind {
    LocalDevice,
    HttpsOrigin,
    MqttBroker,
}

#[derive(Clone, PartialEq, Eq)]
pub struct ConsentReceiptRef(String);

impl ConsentReceiptRef {
    pub fn new(value: impl Into<String>) -> Result<Self, DataGovernanceError> {
        let value = value.into();
        if !value.starts_with("consent://")
            || value.len() <= "consent://".len()
            || value.len() > MAX_CONSENT_REFERENCE_BYTES
            || has_unsafe_text(&value)
        {
            return Err(DataGovernanceError::InvalidConsentReference);
        }
        Ok(Self(value))
    }
}

impl fmt::Debug for ConsentReceiptRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ConsentReceiptRef([REDACTED])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DataPurpose(String);

impl DataPurpose {
    pub fn new(value: impl Into<String>) -> Result<Self, DataGovernanceError> {
        let value = value.into();
        if value.trim().is_empty() || value.len() > MAX_PURPOSE_BYTES || has_unsafe_text(&value) {
            return Err(DataGovernanceError::InvalidPurpose);
        }
        Ok(Self(value))
    }
}

impl fmt::Debug for DataPurpose {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DataPurpose([REDACTED])")
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct DataUseGrant {
    principal_id: AgentId,
    resource_id: String,
    category: DataCategory,
    operation: DataOperation,
    destination: DataDestination,
    purpose: DataPurpose,
    consent_ref: ConsentReceiptRef,
    retention: DataRetention,
    granted_at_ms: u64,
    expires_at_ms: u64,
}

impl DataUseGrant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        principal_id: AgentId,
        resource_id: impl Into<String>,
        category: DataCategory,
        operation: DataOperation,
        destination: DataDestination,
        purpose: DataPurpose,
        consent_ref: ConsentReceiptRef,
        retention: DataRetention,
        granted_at_ms: u64,
        expires_at_ms: u64,
    ) -> Result<Self, DataGovernanceError> {
        let resource_id = resource_id.into();
        if resource_id.trim().is_empty()
            || resource_id.len() > MAX_RESOURCE_BYTES
            || has_unsafe_text(&resource_id)
        {
            return Err(DataGovernanceError::InvalidResource);
        }
        if expires_at_ms <= granted_at_ms
            || operation == DataOperation::StopEgress
            || !request_shape_is_valid(operation, &destination, retention)
        {
            return Err(DataGovernanceError::InvalidGrant);
        }
        Ok(Self {
            principal_id,
            resource_id,
            category,
            operation,
            destination,
            purpose,
            consent_ref,
            retention,
            granted_at_ms,
            expires_at_ms,
        })
    }

    fn matches(&self, request: &DataUseRequest<'_>) -> bool {
        self.principal_id == *request.principal_id
            && self.resource_id == request.resource_id
            && self.category == request.category
            && self.operation == request.operation
            && self.destination == request.destination
            && self.retention == request.retention
            && request.now_ms >= self.granted_at_ms
            && request.now_ms < self.expires_at_ms
    }
}

impl fmt::Debug for DataUseGrant {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DataUseGrant")
            .field("principal_id", &"[REDACTED]")
            .field("resource_id", &"[REDACTED]")
            .field("category", &self.category)
            .field("operation", &self.operation)
            .field("destination_kind", &self.destination.kind())
            .field("purpose", &self.purpose)
            .field("consent_ref", &self.consent_ref)
            .field("retention", &self.retention)
            .field("granted_at_ms", &self.granted_at_ms)
            .field("expires_at_ms", &self.expires_at_ms)
            .finish()
    }
}

pub struct DataUseRequest<'a> {
    pub principal_id: &'a AgentId,
    pub resource_id: &'a str,
    pub category: DataCategory,
    pub operation: DataOperation,
    pub destination: DataDestination,
    pub retention: DataRetention,
    pub now_ms: u64,
}

impl fmt::Debug for DataUseRequest<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DataUseRequest")
            .field("principal_id", &"[REDACTED]")
            .field("resource_id", &"[REDACTED]")
            .field("category", &self.category)
            .field("operation", &self.operation)
            .field("destination_kind", &self.destination.kind())
            .field("retention", &self.retention)
            .field("now_ms", &self.now_ms)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataGovernanceAllowance {
    ExplicitConsent,
    PrivacyProtective,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataGovernanceDenial {
    NoMatchingConsent,
    InvalidRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataGovernanceDecision {
    Allow(DataGovernanceAllowance),
    Deny(DataGovernanceDenial),
}

impl DataGovernanceDecision {
    pub fn is_allowed(self) -> bool {
        matches!(self, Self::Allow(_))
    }
}

pub struct DataGovernancePolicy {
    grants: Vec<DataUseGrant>,
    maximum_grants: usize,
}

impl Default for DataGovernancePolicy {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_GRANTS)
    }
}

impl DataGovernancePolicy {
    pub fn new(maximum_grants: usize) -> Self {
        Self {
            grants: Vec::new(),
            maximum_grants: maximum_grants.max(1),
        }
    }

    pub fn add_grant(&mut self, grant: DataUseGrant) -> Result<(), DataGovernanceError> {
        if self.grants.len() >= self.maximum_grants {
            return Err(DataGovernanceError::GrantCapacityExceeded);
        }
        self.grants.push(grant);
        Ok(())
    }

    pub fn decide(&self, request: &DataUseRequest<'_>) -> DataGovernanceDecision {
        if request.resource_id.trim().is_empty()
            || request.resource_id.len() > MAX_RESOURCE_BYTES
            || has_unsafe_text(request.resource_id)
            || !request_shape_is_valid(request.operation, &request.destination, request.retention)
        {
            return DataGovernanceDecision::Deny(DataGovernanceDenial::InvalidRequest);
        }
        if request.operation == DataOperation::StopEgress {
            return DataGovernanceDecision::Allow(DataGovernanceAllowance::PrivacyProtective);
        }
        if self.grants.iter().any(|grant| grant.matches(request)) {
            DataGovernanceDecision::Allow(DataGovernanceAllowance::ExplicitConsent)
        } else {
            DataGovernanceDecision::Deny(DataGovernanceDenial::NoMatchingConsent)
        }
    }

    pub fn grant_count(&self) -> usize {
        self.grants.len()
    }
}

impl fmt::Debug for DataGovernancePolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DataGovernancePolicy")
            .field("grant_count", &self.grants.len())
            .field("maximum_grants", &self.maximum_grants)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataGovernanceError {
    InvalidConsentReference,
    InvalidDestination,
    InvalidGrant,
    InvalidPurpose,
    InvalidResource,
    GrantCapacityExceeded,
}

impl fmt::Display for DataGovernanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidConsentReference => "invalid consent receipt reference",
            Self::InvalidDestination => "invalid data destination",
            Self::InvalidGrant => "invalid data-use grant",
            Self::InvalidPurpose => "invalid data-use purpose",
            Self::InvalidResource => "invalid governed resource",
            Self::GrantCapacityExceeded => "data-governance grant capacity exceeded",
        })
    }
}

impl std::error::Error for DataGovernanceError {}

fn has_unsafe_text(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn request_shape_is_valid(
    operation: DataOperation,
    destination: &DataDestination,
    retention: DataRetention,
) -> bool {
    matches!(
        (operation, destination, retention),
        (
            DataOperation::Configure,
            DataDestination::LocalDevice,
            DataRetention::NotApplicable
        ) | (
            DataOperation::Inspect,
            DataDestination::LocalDevice,
            DataRetention::Ephemeral
        ) | (
            DataOperation::Inspect,
            DataDestination::LocalDevice,
            DataRetention::Bounded {
                maximum_age_ms: 1..
            }
        ) | (
            DataOperation::StartEgress | DataOperation::StopEgress,
            DataDestination::HttpsOrigin(_) | DataDestination::MqttBroker(_),
            DataRetention::NotApplicable
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn principal() -> AgentId {
        AgentId::trusted("operator")
    }

    fn cloud() -> DataDestination {
        DataDestination::https_origin("https://api.airgradient.com").unwrap()
    }

    fn mqtt() -> DataDestination {
        DataDestination::mqtt_broker("mqtts://broker.example.test:8883").unwrap()
    }

    fn grant() -> DataUseGrant {
        DataUseGrant::new(
            principal(),
            "airgradient:monitor:configuration",
            DataCategory::EnvironmentalTelemetry,
            DataOperation::StartEgress,
            cloud(),
            DataPurpose::new("operator-requested vendor dashboard upload").unwrap(),
            ConsentReceiptRef::new("consent://smart-home/receipt-1").unwrap(),
            DataRetention::NotApplicable,
            100,
            200,
        )
        .unwrap()
    }

    fn request<'a>(principal: &'a AgentId, now_ms: u64) -> DataUseRequest<'a> {
        DataUseRequest {
            principal_id: principal,
            resource_id: "airgradient:monitor:configuration",
            category: DataCategory::EnvironmentalTelemetry,
            operation: DataOperation::StartEgress,
            destination: cloud(),
            retention: DataRetention::NotApplicable,
            now_ms,
        }
    }

    #[test]
    fn policy_denies_without_exact_active_consent() {
        let principal = principal();
        let policy = DataGovernancePolicy::default();
        assert_eq!(
            policy.decide(&request(&principal, 150)),
            DataGovernanceDecision::Deny(DataGovernanceDenial::NoMatchingConsent)
        );
    }

    #[test]
    fn policy_allows_only_exact_active_consent() {
        let principal = principal();
        let mut policy = DataGovernancePolicy::default();
        policy.add_grant(grant()).unwrap();
        assert_eq!(
            policy.decide(&request(&principal, 150)),
            DataGovernanceDecision::Allow(DataGovernanceAllowance::ExplicitConsent)
        );
        assert_eq!(
            policy.decide(&request(&principal, 200)),
            DataGovernanceDecision::Deny(DataGovernanceDenial::NoMatchingConsent)
        );
        let other = AgentId::trusted("other");
        assert_eq!(
            policy.decide(&request(&other, 150)),
            DataGovernanceDecision::Deny(DataGovernanceDenial::NoMatchingConsent)
        );
    }

    #[test]
    fn stopping_egress_is_privacy_protective() {
        let principal = principal();
        let policy = DataGovernancePolicy::default();
        let mut request = request(&principal, 150);
        request.operation = DataOperation::StopEgress;
        assert_eq!(
            policy.decide(&request),
            DataGovernanceDecision::Allow(DataGovernanceAllowance::PrivacyProtective)
        );
    }

    #[test]
    fn operation_and_destination_pairs_are_fail_closed() {
        let principal = principal();
        let policy = DataGovernancePolicy::default();
        let request = DataUseRequest {
            principal_id: &principal,
            resource_id: "airgradient:monitor:configuration",
            category: DataCategory::EnvironmentalTelemetry,
            operation: DataOperation::StopEgress,
            destination: DataDestination::LocalDevice,
            retention: DataRetention::NotApplicable,
            now_ms: 150,
        };
        assert_eq!(
            policy.decide(&request),
            DataGovernanceDecision::Deny(DataGovernanceDenial::InvalidRequest)
        );
        assert_eq!(
            DataUseGrant::new(
                principal,
                "airgradient:monitor:configuration",
                DataCategory::CoarseLocation,
                DataOperation::Configure,
                cloud(),
                DataPurpose::new("operator-selected monitor country").unwrap(),
                ConsentReceiptRef::new("consent://smart-home/receipt-2").unwrap(),
                DataRetention::NotApplicable,
                100,
                200,
            ),
            Err(DataGovernanceError::InvalidGrant)
        );
    }

    #[test]
    fn grant_capacity_is_bounded() {
        let mut policy = DataGovernancePolicy::new(1);
        policy.add_grant(grant()).unwrap();
        assert_eq!(
            policy.add_grant(grant()),
            Err(DataGovernanceError::GrantCapacityExceeded)
        );
    }

    #[test]
    fn sensitive_text_is_redacted_from_debug() {
        let debug = format!("{:?}", grant());
        assert!(!debug.contains("receipt-1"));
        assert!(!debug.contains("operator-requested"));
        assert!(!debug.contains("airgradient:monitor"));
        assert!(!debug.contains("operator"));
        assert!(!format!("{:?}", cloud()).contains("airgradient"));
    }

    #[test]
    fn destinations_and_consent_references_are_strict() {
        assert_eq!(
            DataDestination::https_origin("http://api.airgradient.com"),
            Err(DataGovernanceError::InvalidDestination)
        );
        assert_eq!(
            ConsentReceiptRef::new("receipt-1"),
            Err(DataGovernanceError::InvalidConsentReference)
        );
        assert_eq!(
            DataPurpose::new("line\tbreak"),
            Err(DataGovernanceError::InvalidPurpose)
        );
        for destination in [
            "mqtts://broker.example.test",
            "mqtts://user:secret@broker.example.test:8883",
            "mqtts://broker.example.test:8883/topic",
            "http://broker.example.test:8883",
        ] {
            assert_eq!(
                DataDestination::mqtt_broker(destination),
                Err(DataGovernanceError::InvalidDestination)
            );
        }
        assert!(!format!("{:?}", mqtt()).contains("broker.example.test"));
    }

    #[test]
    fn mqtt_grants_are_exact_and_stop_is_privacy_protective() {
        let principal = principal();
        let mut policy = DataGovernancePolicy::default();
        policy
            .add_grant(
                DataUseGrant::new(
                    principal.clone(),
                    "airgradient:monitor:configuration",
                    DataCategory::EnvironmentalTelemetry,
                    DataOperation::StartEgress,
                    mqtt(),
                    DataPurpose::new("operator-selected MQTT telemetry route").unwrap(),
                    ConsentReceiptRef::new("consent://smart-home/mqtt-1").unwrap(),
                    DataRetention::NotApplicable,
                    100,
                    200,
                )
                .unwrap(),
            )
            .unwrap();
        assert!(policy
            .decide(&DataUseRequest {
                principal_id: &principal,
                resource_id: "airgradient:monitor:configuration",
                category: DataCategory::EnvironmentalTelemetry,
                operation: DataOperation::StartEgress,
                destination: mqtt(),
                retention: DataRetention::NotApplicable,
                now_ms: 150,
            })
            .is_allowed());
        assert_eq!(
            policy.decide(&DataUseRequest {
                principal_id: &principal,
                resource_id: "airgradient:monitor:configuration",
                category: DataCategory::EnvironmentalTelemetry,
                operation: DataOperation::StopEgress,
                destination: mqtt(),
                retention: DataRetention::NotApplicable,
                now_ms: 250,
            }),
            DataGovernanceDecision::Allow(DataGovernanceAllowance::PrivacyProtective)
        );
    }

    #[test]
    fn identifier_inspection_requires_consent_scoped_ephemeral_retention() {
        let principal = principal();
        let mut policy = DataGovernancePolicy::default();
        policy
            .add_grant(
                DataUseGrant::new(
                    principal.clone(),
                    "enphase:gateway:microinverters",
                    DataCategory::DeviceIdentifier,
                    DataOperation::Inspect,
                    DataDestination::LocalDevice,
                    DataPurpose::new("diagnose per-inverter solar production").unwrap(),
                    ConsentReceiptRef::new("consent://smart-home/enphase-inverters-1").unwrap(),
                    DataRetention::Ephemeral,
                    100,
                    200,
                )
                .unwrap(),
            )
            .unwrap();
        let mut request = DataUseRequest {
            principal_id: &principal,
            resource_id: "enphase:gateway:microinverters",
            category: DataCategory::DeviceIdentifier,
            operation: DataOperation::Inspect,
            destination: DataDestination::LocalDevice,
            retention: DataRetention::Ephemeral,
            now_ms: 150,
        };
        assert_eq!(
            policy.decide(&request),
            DataGovernanceDecision::Allow(DataGovernanceAllowance::ExplicitConsent)
        );
        request.retention = DataRetention::Bounded { maximum_age_ms: 1 };
        assert_eq!(
            policy.decide(&request),
            DataGovernanceDecision::Deny(DataGovernanceDenial::NoMatchingConsent)
        );
        request.retention = DataRetention::NotApplicable;
        assert_eq!(
            policy.decide(&request),
            DataGovernanceDecision::Deny(DataGovernanceDenial::InvalidRequest)
        );
        assert_eq!(
            DataUseGrant::new(
                principal,
                "enphase:gateway:microinverters",
                DataCategory::DeviceIdentifier,
                DataOperation::Inspect,
                DataDestination::LocalDevice,
                DataPurpose::new("diagnose per-inverter solar production").unwrap(),
                ConsentReceiptRef::new("consent://smart-home/enphase-inverters-2").unwrap(),
                DataRetention::Bounded { maximum_age_ms: 0 },
                100,
                200,
            ),
            Err(DataGovernanceError::InvalidGrant)
        );
    }

    #[test]
    fn presence_inspection_requires_exact_bounded_retention() {
        let principal = principal();
        let mut policy = DataGovernancePolicy::default();
        policy
            .add_grant(
                DataUseGrant::new(
                    principal.clone(),
                    "unifi:home:connected-clients",
                    DataCategory::Presence,
                    DataOperation::Inspect,
                    DataDestination::LocalDevice,
                    DataPurpose::new("show current home-network presence").unwrap(),
                    ConsentReceiptRef::new("consent://smart-home/unifi-presence-1").unwrap(),
                    DataRetention::Bounded {
                        maximum_age_ms: 300_000,
                    },
                    100,
                    200,
                )
                .unwrap(),
            )
            .unwrap();
        let mut request = DataUseRequest {
            principal_id: &principal,
            resource_id: "unifi:home:connected-clients",
            category: DataCategory::Presence,
            operation: DataOperation::Inspect,
            destination: DataDestination::LocalDevice,
            retention: DataRetention::Bounded {
                maximum_age_ms: 300_000,
            },
            now_ms: 150,
        };
        assert!(policy.decide(&request).is_allowed());
        request.retention = DataRetention::Bounded {
            maximum_age_ms: 300_001,
        };
        assert_eq!(
            policy.decide(&request),
            DataGovernanceDecision::Deny(DataGovernanceDenial::NoMatchingConsent)
        );
    }

    #[test]
    fn operational_telemetry_requires_exact_local_retention() {
        let principal = principal();
        let mut policy = DataGovernancePolicy::default();
        policy
            .add_grant(
                DataUseGrant::new(
                    principal.clone(),
                    "unifi:home:device-statistics",
                    DataCategory::OperationalTelemetry,
                    DataOperation::Inspect,
                    DataDestination::LocalDevice,
                    DataPurpose::new("inspect short-lived network device health metrics").unwrap(),
                    ConsentReceiptRef::new("consent://smart-home/unifi-statistics-1").unwrap(),
                    DataRetention::Bounded {
                        maximum_age_ms: 120_000,
                    },
                    100,
                    200,
                )
                .unwrap(),
            )
            .unwrap();
        let mut request = DataUseRequest {
            principal_id: &principal,
            resource_id: "unifi:home:device-statistics",
            category: DataCategory::OperationalTelemetry,
            operation: DataOperation::Inspect,
            destination: DataDestination::LocalDevice,
            retention: DataRetention::Bounded {
                maximum_age_ms: 120_000,
            },
            now_ms: 150,
        };
        assert!(policy.decide(&request).is_allowed());
        request.retention = DataRetention::Ephemeral;
        assert_eq!(
            policy.decide(&request),
            DataGovernanceDecision::Deny(DataGovernanceDenial::NoMatchingConsent)
        );
    }
}
