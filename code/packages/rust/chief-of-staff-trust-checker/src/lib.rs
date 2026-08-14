//! Transport-independent privilege approval for the D18 Chief of Staff.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use chief_of_staff_tool_api::{ApprovalAssurance, PrivilegeTier};
use core::fmt::{self, Display, Formatter};
use std::collections::BTreeSet;
use std::time::Duration;

const MAX_RESOURCES: usize = 1_026;
const MAX_CONTEXT_IDENTIFIER_BYTES: usize = 128;
const MAX_RESOURCE_IDENTIFIER_BYTES: usize = 320;
/// Canonical Tier 1 notification window before timeout becomes approval.
pub const TIER_1_AUTO_APPROVE_TIMEOUT: Duration = Duration::from_secs(5);
/// Canonical Tier 2 biometric approval window before timeout becomes denial.
pub const TIER_2_BIOMETRIC_TIMEOUT: Duration = Duration::from_secs(30);
/// Canonical Tier 3 hardware-key approval window before timeout becomes denial.
pub const TIER_3_HARDWARE_KEY_TIMEOUT: Duration = Duration::from_secs(60);

/// One exact non-secret resource whose privilege contributes to a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustResource {
    resource_id: String,
    tier: PrivilegeTier,
}

impl TrustResource {
    /// Create a resource with a bounded stable identifier and canonical tier.
    pub fn new(
        resource_id: impl Into<String>,
        tier: PrivilegeTier,
    ) -> Result<Self, TrustRequestError> {
        let resource_id = resource_id.into();
        validate_identifier(&resource_id, MAX_RESOURCE_IDENTIFIER_BYTES)
            .map_err(|()| TrustRequestError::InvalidResourceId)?;
        Ok(Self { resource_id, tier })
    }

    /// Return the non-secret stable resource identifier.
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    /// Return the resource's required privilege tier.
    pub fn tier(&self) -> PrivilegeTier {
        self.tier
    }
}

/// One exact bounded authorization request presented to the trusted provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustRequest {
    request_id: String,
    requested_by: String,
    resources: Vec<TrustResource>,
    effective_tier: PrivilegeTier,
}

/// Validated caller and correlation context reused while exact resources are resolved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustRequestContext {
    request_id: String,
    requested_by: String,
}

impl TrustRequestContext {
    /// Validate the non-secret context for one approval request.
    pub fn new(
        request_id: impl Into<String>,
        requested_by: impl Into<String>,
    ) -> Result<Self, TrustRequestError> {
        let request_id = request_id.into();
        let requested_by = requested_by.into();
        validate_identifier(&request_id, MAX_CONTEXT_IDENTIFIER_BYTES)
            .map_err(|()| TrustRequestError::InvalidRequestId)?;
        validate_identifier(&requested_by, MAX_CONTEXT_IDENTIFIER_BYTES)
            .map_err(|()| TrustRequestError::InvalidRequester)?;
        Ok(Self {
            request_id,
            requested_by,
        })
    }

    /// Return the caller's stable correlation identifier.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Return the non-secret identity that initiated the request.
    pub fn requested_by(&self) -> &str {
        &self.requested_by
    }

    /// Attach an exact resource set and compute its maximum tier.
    pub fn with_resources(
        &self,
        resources: Vec<TrustResource>,
    ) -> Result<TrustRequest, TrustRequestError> {
        TrustRequest::from_context(self.clone(), resources)
    }
}

impl TrustRequest {
    /// Validate an exact non-empty resource set and compute its maximum tier.
    pub fn new(
        request_id: impl Into<String>,
        requested_by: impl Into<String>,
        resources: Vec<TrustResource>,
    ) -> Result<Self, TrustRequestError> {
        let context = TrustRequestContext::new(request_id, requested_by)?;
        Self::from_context(context, resources)
    }

    fn from_context(
        context: TrustRequestContext,
        resources: Vec<TrustResource>,
    ) -> Result<Self, TrustRequestError> {
        if resources.is_empty() {
            return Err(TrustRequestError::NoResources);
        }
        if resources.len() > MAX_RESOURCES {
            return Err(TrustRequestError::TooManyResources);
        }
        let mut identifiers = BTreeSet::new();
        for resource in &resources {
            if !identifiers.insert(resource.resource_id.as_str()) {
                return Err(TrustRequestError::DuplicateResource);
            }
        }
        let effective_tier = resources
            .iter()
            .map(TrustResource::tier)
            .max()
            .expect("non-empty resources have a maximum tier");
        Ok(Self {
            request_id: context.request_id,
            requested_by: context.requested_by,
            resources,
            effective_tier,
        })
    }

    /// Return the caller's stable correlation identifier.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Return the non-secret identity that initiated the request.
    pub fn requested_by(&self) -> &str {
        &self.requested_by
    }

    /// Return every exact resource in caller-provided display order.
    pub fn resources(&self) -> &[TrustResource] {
        &self.resources
    }

    /// Return the maximum privilege tier of every resource.
    pub fn effective_tier(&self) -> PrivilegeTier {
        self.effective_tier
    }
}

/// Stable validation failure for an authorization request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustRequestError {
    /// The request identifier was empty, too long, or contained control bytes.
    InvalidRequestId,
    /// The requester identifier was empty, too long, or contained control bytes.
    InvalidRequester,
    /// A resource identifier was empty, too long, or contained control bytes.
    InvalidResourceId,
    /// At least one resource is required so Tier 0 is never inferred from absence.
    NoResources,
    /// The request exceeded the bounded resource count.
    TooManyResources,
    /// The same resource identifier appeared more than once.
    DuplicateResource,
}

impl Display for TrustRequestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequestId => "trust checker: invalid request id",
            Self::InvalidRequester => "trust checker: invalid requester",
            Self::InvalidResourceId => "trust checker: invalid resource id",
            Self::NoResources => "trust checker: no resources",
            Self::TooManyResources => "trust checker: too many resources",
            Self::DuplicateResource => "trust checker: duplicate resource",
        })
    }
}

impl std::error::Error for TrustRequestError {}

/// Exact user interaction required for one effective tier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalRequirement {
    /// Tier 0 requires no provider interaction.
    None,
    /// Tier 1 sends a notification and treats provider timeout as approval.
    Notification {
        /// Maximum wait before the absence of denial becomes approval.
        timeout: Duration,
    },
    /// Tier 2 requires biometric assurance and fails timeout closed.
    Biometric {
        /// Maximum wait for biometric approval.
        timeout: Duration,
    },
    /// Tier 3 requires hardware-key assurance and fails timeout closed.
    HardwareKey {
        /// Maximum wait for hardware-key approval.
        timeout: Duration,
    },
}

impl ApprovalRequirement {
    /// Return the canonical D18 requirement for a privilege tier.
    pub fn for_tier(tier: PrivilegeTier) -> Self {
        match tier {
            PrivilegeTier::Tier0 => Self::None,
            PrivilegeTier::Tier1 => Self::Notification {
                timeout: TIER_1_AUTO_APPROVE_TIMEOUT,
            },
            PrivilegeTier::Tier2 => Self::Biometric {
                timeout: TIER_2_BIOMETRIC_TIMEOUT,
            },
            PrivilegeTier::Tier3 => Self::HardwareKey {
                timeout: TIER_3_HARDWARE_KEY_TIMEOUT,
            },
        }
    }

    /// Return the minimum assurance for an explicit approval, if any.
    pub fn minimum_assurance(self) -> Option<ApprovalAssurance> {
        match self {
            Self::None => None,
            Self::Notification { .. } => Some(ApprovalAssurance::ExplicitConsent),
            Self::Biometric { .. } => Some(ApprovalAssurance::Biometric),
            Self::HardwareKey { .. } => Some(ApprovalAssurance::HardwareKey),
        }
    }
}

/// Exact non-secret prompt passed to a trusted platform approval adapter.
#[derive(Clone, Copy, Debug)]
pub struct ApprovalPrompt<'a> {
    request: &'a TrustRequest,
    requirement: ApprovalRequirement,
}

impl<'a> ApprovalPrompt<'a> {
    /// Return the validated request, including every exact resource.
    pub fn request(self) -> &'a TrustRequest {
        self.request
    }

    /// Return the canonical interaction and timeout policy.
    pub fn requirement(self) -> ApprovalRequirement {
        self.requirement
    }
}

/// Provider result after completing the requested interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalOutcome {
    /// The user approved with the stated trusted authentication strength.
    Approved(ApprovalAssurance),
    /// The user explicitly denied the request.
    Denied,
    /// The canonical interaction window elapsed without approval or denial.
    TimedOut,
}

/// Replaceable trusted UI/device boundary for non-Tier-0 requests.
pub trait ApprovalProvider {
    /// Concrete adapter failure retained for programmatic recovery.
    type Error;

    /// Request one exact approval using the supplied interaction and timeout.
    fn request_approval(
        &mut self,
        prompt: ApprovalPrompt<'_>,
    ) -> Result<ApprovalOutcome, Self::Error>;
}

/// Why an authorization request was accepted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorizationBasis {
    /// Tier 0 required no user interaction.
    Tier0,
    /// A trusted provider returned explicit approval at this assurance.
    Approved(ApprovalAssurance),
    /// Tier 1 elapsed without denial and followed the specified auto-approval rule.
    Tier1Timeout,
}

/// Non-secret proof that one exact request passed the trust policy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorizationReceipt {
    request_id: String,
    effective_tier: PrivilegeTier,
    basis: AuthorizationBasis,
}

impl AuthorizationReceipt {
    /// Return the exact request identifier approved by the checker.
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Return the maximum tier authorized by this receipt.
    pub fn effective_tier(&self) -> PrivilegeTier {
        self.effective_tier
    }

    /// Return the policy path that authorized the request.
    pub fn basis(&self) -> AuthorizationBasis {
        self.basis
    }
}

/// Typed fail-closed authorization failure.
#[derive(Debug)]
pub enum TrustCheckerError<ProviderError> {
    /// The trusted provider could not complete its interaction.
    Provider(ProviderError),
    /// The user explicitly denied the request.
    Denied,
    /// A Tier 2 or Tier 3 request timed out.
    TimedOut,
    /// The provider asserted less authentication strength than required.
    InsufficientAssurance {
        /// Minimum assurance required by the effective tier.
        required: ApprovalAssurance,
        /// Assurance actually returned by the provider.
        provided: ApprovalAssurance,
    },
}

impl<E> Display for TrustCheckerError<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Provider(_) => "trust checker: approval provider failed",
            Self::Denied => "trust checker: approval denied",
            Self::TimedOut => "trust checker: approval timed out",
            Self::InsufficientAssurance { .. } => "trust checker: approval assurance insufficient",
        })
    }
}

impl<E> std::error::Error for TrustCheckerError<E> where E: std::error::Error + 'static {}

/// Policy evaluator over one replaceable trusted approval provider.
pub struct TrustChecker<P> {
    provider: P,
}

impl<P> TrustChecker<P> {
    /// Create a checker with an injected provider.
    pub fn new(provider: P) -> Self {
        Self { provider }
    }

    /// Consume the checker and recover its provider.
    pub fn into_provider(self) -> P {
        self.provider
    }
}

impl<P: ApprovalProvider> TrustChecker<P> {
    /// Authorize one exact validated request according to its effective tier.
    pub fn authorize(
        &mut self,
        request: &TrustRequest,
    ) -> Result<AuthorizationReceipt, TrustCheckerError<P::Error>> {
        let requirement = ApprovalRequirement::for_tier(request.effective_tier());
        if requirement == ApprovalRequirement::None {
            return Ok(AuthorizationReceipt {
                request_id: request.request_id().to_string(),
                effective_tier: PrivilegeTier::Tier0,
                basis: AuthorizationBasis::Tier0,
            });
        }
        let outcome = self
            .provider
            .request_approval(ApprovalPrompt {
                request,
                requirement,
            })
            .map_err(TrustCheckerError::Provider)?;
        match outcome {
            ApprovalOutcome::Approved(provided) => {
                let required = requirement
                    .minimum_assurance()
                    .expect("non-Tier-0 requirements have a minimum assurance");
                if provided < required {
                    return Err(TrustCheckerError::InsufficientAssurance { required, provided });
                }
                Ok(AuthorizationReceipt {
                    request_id: request.request_id().to_string(),
                    effective_tier: request.effective_tier(),
                    basis: AuthorizationBasis::Approved(provided),
                })
            }
            ApprovalOutcome::Denied => Err(TrustCheckerError::Denied),
            ApprovalOutcome::TimedOut if request.effective_tier() == PrivilegeTier::Tier1 => {
                Ok(AuthorizationReceipt {
                    request_id: request.request_id().to_string(),
                    effective_tier: PrivilegeTier::Tier1,
                    basis: AuthorizationBasis::Tier1Timeout,
                })
            }
            ApprovalOutcome::TimedOut => Err(TrustCheckerError::TimedOut),
        }
    }
}

fn validate_identifier(value: &str, maximum_bytes: usize) -> Result<(), ()> {
    if value.is_empty() || value.len() > maximum_bytes || value.chars().any(char::is_control) {
        return Err(());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct ProviderFailure;

    impl Display for ProviderFailure {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("private provider detail")
        }
    }

    impl std::error::Error for ProviderFailure {}

    #[derive(Default)]
    struct RecordingProvider {
        outcomes: VecDeque<Result<ApprovalOutcome, ProviderFailure>>,
        prompts: Vec<(String, PrivilegeTier, ApprovalRequirement, Vec<String>)>,
    }

    impl RecordingProvider {
        fn returning(outcome: Result<ApprovalOutcome, ProviderFailure>) -> Self {
            Self {
                outcomes: VecDeque::from([outcome]),
                prompts: Vec::new(),
            }
        }
    }

    impl ApprovalProvider for RecordingProvider {
        type Error = ProviderFailure;

        fn request_approval(
            &mut self,
            prompt: ApprovalPrompt<'_>,
        ) -> Result<ApprovalOutcome, Self::Error> {
            self.prompts.push((
                prompt.request().request_id().to_string(),
                prompt.request().effective_tier(),
                prompt.requirement(),
                prompt
                    .request()
                    .resources()
                    .iter()
                    .map(|resource| resource.resource_id().to_string())
                    .collect(),
            ));
            self.outcomes.pop_front().expect("configured outcome")
        }
    }

    fn request(tiers: &[PrivilegeTier]) -> TrustRequest {
        TrustRequest::new(
            "request-1",
            "operator:local",
            tiers
                .iter()
                .enumerate()
                .map(|(index, tier)| {
                    TrustResource::new(format!("resource-{index}"), *tier).unwrap()
                })
                .collect(),
        )
        .unwrap()
    }

    #[test]
    fn validates_exact_bounded_resources_and_computes_maximum_tier() {
        let request = request(&[
            PrivilegeTier::Tier0,
            PrivilegeTier::Tier2,
            PrivilegeTier::Tier1,
        ]);
        assert_eq!(request.request_id(), "request-1");
        assert_eq!(request.requested_by(), "operator:local");
        assert_eq!(request.effective_tier(), PrivilegeTier::Tier2);
        assert_eq!(request.resources().len(), 3);
        let context = TrustRequestContext::new("request-2", "operator:local").unwrap();
        let contextual = context
            .with_resources(vec![
                TrustResource::new("resource", PrivilegeTier::Tier1).unwrap()
            ])
            .unwrap();
        assert_eq!(contextual.request_id(), "request-2");
        assert_eq!(contextual.requested_by(), "operator:local");

        assert_eq!(
            TrustRequest::new(
                "bad\nrequest",
                "operator",
                vec![TrustResource::new("resource", PrivilegeTier::Tier0).unwrap()]
            ),
            Err(TrustRequestError::InvalidRequestId)
        );
        assert_eq!(
            TrustRequest::new(
                "request",
                "bad\noperator",
                vec![TrustResource::new("resource", PrivilegeTier::Tier0).unwrap()]
            ),
            Err(TrustRequestError::InvalidRequester)
        );

        assert_eq!(
            TrustRequest::new("request", "operator", Vec::new()),
            Err(TrustRequestError::NoResources)
        );
        assert_eq!(
            TrustRequest::new(
                "request",
                "operator",
                vec![
                    TrustResource::new("same", PrivilegeTier::Tier0).unwrap(),
                    TrustResource::new("same", PrivilegeTier::Tier3).unwrap(),
                ],
            ),
            Err(TrustRequestError::DuplicateResource)
        );
        assert_eq!(
            TrustRequest::new(
                "request",
                "operator",
                (0..=MAX_RESOURCES)
                    .map(|index| {
                        TrustResource::new(format!("resource-{index}"), PrivilegeTier::Tier0)
                            .unwrap()
                    })
                    .collect(),
            ),
            Err(TrustRequestError::TooManyResources)
        );
        assert_eq!(
            TrustResource::new("bad\nresource", PrivilegeTier::Tier0),
            Err(TrustRequestError::InvalidResourceId)
        );
        assert!(TrustResource::new("r".repeat(320), PrivilegeTier::Tier0).is_ok());
        assert_eq!(
            TrustResource::new("r".repeat(321), PrivilegeTier::Tier0),
            Err(TrustRequestError::InvalidResourceId)
        );
    }

    #[test]
    fn tier_zero_bypasses_the_provider() {
        let mut checker = TrustChecker::new(RecordingProvider::default());
        let receipt = checker
            .authorize(&request(&[PrivilegeTier::Tier0]))
            .unwrap();
        assert_eq!(receipt.effective_tier(), PrivilegeTier::Tier0);
        assert_eq!(receipt.basis(), AuthorizationBasis::Tier0);
        assert_eq!(receipt.request_id(), "request-1");
        assert!(checker.into_provider().prompts.is_empty());
    }

    #[test]
    fn provider_receives_exact_request_and_canonical_requirement() {
        let provider = RecordingProvider::returning(Ok(ApprovalOutcome::Approved(
            ApprovalAssurance::Biometric,
        )));
        let mut checker = TrustChecker::new(provider);
        let receipt = checker
            .authorize(&request(&[PrivilegeTier::Tier1, PrivilegeTier::Tier2]))
            .unwrap();
        assert_eq!(receipt.effective_tier(), PrivilegeTier::Tier2);
        assert_eq!(
            receipt.basis(),
            AuthorizationBasis::Approved(ApprovalAssurance::Biometric)
        );
        assert_eq!(
            checker.into_provider().prompts,
            vec![(
                "request-1".to_string(),
                PrivilegeTier::Tier2,
                ApprovalRequirement::Biometric {
                    timeout: Duration::from_secs(30)
                },
                vec!["resource-0".to_string(), "resource-1".to_string()]
            )]
        );
    }

    #[test]
    fn tier_one_timeout_auto_approves_but_explicit_denial_fails() {
        let mut timeout =
            TrustChecker::new(RecordingProvider::returning(Ok(ApprovalOutcome::TimedOut)));
        assert_eq!(
            timeout
                .authorize(&request(&[PrivilegeTier::Tier1]))
                .unwrap()
                .basis(),
            AuthorizationBasis::Tier1Timeout
        );

        let mut denied =
            TrustChecker::new(RecordingProvider::returning(Ok(ApprovalOutcome::Denied)));
        assert!(matches!(
            denied.authorize(&request(&[PrivilegeTier::Tier1])),
            Err(TrustCheckerError::Denied)
        ));
    }

    #[test]
    fn tier_two_and_three_timeouts_fail_closed() {
        for tier in [PrivilegeTier::Tier2, PrivilegeTier::Tier3] {
            let mut checker =
                TrustChecker::new(RecordingProvider::returning(Ok(ApprovalOutcome::TimedOut)));
            assert!(matches!(
                checker.authorize(&request(&[tier])),
                Err(TrustCheckerError::TimedOut)
            ));
        }
    }

    #[test]
    fn weak_assurance_and_provider_failure_remain_typed_and_redacted() {
        let mut weak = TrustChecker::new(RecordingProvider::returning(Ok(
            ApprovalOutcome::Approved(ApprovalAssurance::ExplicitConsent),
        )));
        let error = weak
            .authorize(&request(&[PrivilegeTier::Tier3]))
            .unwrap_err();
        assert!(matches!(
            error,
            TrustCheckerError::InsufficientAssurance {
                required: ApprovalAssurance::HardwareKey,
                provided: ApprovalAssurance::ExplicitConsent
            }
        ));
        assert_eq!(
            error.to_string(),
            "trust checker: approval assurance insufficient"
        );

        let mut failed = TrustChecker::new(RecordingProvider::returning(Err(ProviderFailure)));
        let error = failed
            .authorize(&request(&[PrivilegeTier::Tier1]))
            .unwrap_err();
        assert!(matches!(
            error,
            TrustCheckerError::Provider(ProviderFailure)
        ));
        assert_eq!(error.to_string(), "trust checker: approval provider failed");
        assert!(!error.to_string().contains("private provider detail"));
    }

    #[test]
    fn canonical_requirements_match_the_d18_policy() {
        assert_eq!(
            ApprovalRequirement::for_tier(PrivilegeTier::Tier0),
            ApprovalRequirement::None
        );
        assert_eq!(
            ApprovalRequirement::for_tier(PrivilegeTier::Tier1),
            ApprovalRequirement::Notification {
                timeout: TIER_1_AUTO_APPROVE_TIMEOUT
            }
        );
        assert_eq!(
            ApprovalRequirement::for_tier(PrivilegeTier::Tier2),
            ApprovalRequirement::Biometric {
                timeout: TIER_2_BIOMETRIC_TIMEOUT
            }
        );
        assert_eq!(
            ApprovalRequirement::for_tier(PrivilegeTier::Tier3),
            ApprovalRequirement::HardwareKey {
                timeout: TIER_3_HARDWARE_KEY_TIMEOUT
            }
        );
    }
}
