//! # Exact authenticated OCSP target-revocation composition
//!
//! A responder can be authorized by one issuer while a detached revocation
//! result names another response, target, or issuer. This crate consumes the
//! complete repository-owned proofs and rejects that cross-object mixing.
//! Success retains an unambiguous exact signed response, its exact
//! good-and-fresh target status, the issuer-authorized responder, responder
//! validity at that status's observation time, and the target's authenticated
//! issuer link.
//!
//! The fixed result retains fixed SHA-256 response authentication. One additive
//! result routes the response signature while keeping both certificate issuer
//! links fixed to RSA SHA-256. A second additive result consumes a routed
//! responder-certificate issuer proof and routes the target-certificate issuer
//! signature through caller-owned SHA-256, SHA-384, or SHA-512 policy. None
//! establishes issuer trust, validates a complete certificate or responder
//! path, checks responder revocation, interprets OCSP extensions, performs TLS
//! or HTTP, authorizes OAuth transport, or grants network authority.

#![deny(unsafe_code)]

use core::fmt;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate::X509Certificate;
use x509_certificate_policy::{
    validate_certificate_issuer_link, validate_certificate_validity_at,
    ValidatedCertificateAlgorithmPolicy, ValidatedCertificateValidity,
    X509CertificateIssuerLinkErrorKind, X509CertificateValidityErrorKind,
};
use x509_name::X509Name;
use x509_ocsp_basic_response::X509OcspBasicResponse;
use x509_ocsp_issuer_bound_responder::{
    IssuerBoundX509OcspRsaResponder, IssuerBoundX509OcspRsaRoutedIssuerResponder,
    IssuerBoundX509OcspRsaSha256Responder,
};
use x509_ocsp_revocation_policy::ValidatedBoundX509OcspRevocation;
use x509_rsa_algorithm_policy::ValidatedX509RsaAlgorithmPolicy;
use x509_rsa_sha256_issuer_link::VerifiedX509RsaSha256IssuerLink;
use x509_rsa_signature_routing::{
    verify_x509_rsa_policy_signature, VerifiedX509RsaPolicySignature, X509RsaSignatureRoute,
    X509RsaSignatureRoutingErrorKind,
};

/// Stable failures retaining no response, certificate, key, status, time, or signature bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspAuthenticatedTargetRevocationErrorKind {
    /// The signed response contains more than one status entry.
    AmbiguousResponseSet,
    /// The validated `SingleResponse` is not part of the authenticated response.
    ResponseMismatch,
    /// The revocation result and target issuer link retain different targets.
    TargetMismatch,
    /// The responder, revocation result, and target link do not share one issuer.
    IssuerMismatch,
    /// Outer response extensions have no implemented policy at this boundary.
    UnsupportedResponseExtensions,
    /// The responder certificate was not valid at the revocation observation.
    ResponderCertificateValidity(X509CertificateValidityErrorKind),
}

/// A redacted target-specific OCSP composition failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspAuthenticatedTargetRevocationError {
    kind: X509OcspAuthenticatedTargetRevocationErrorKind,
}

impl X509OcspAuthenticatedTargetRevocationError {
    const fn new(kind: X509OcspAuthenticatedTargetRevocationErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspAuthenticatedTargetRevocationErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspAuthenticatedTargetRevocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP authenticated target revocation error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspAuthenticatedTargetRevocationError {}

/// Stable failures for fully routed responder and target issuer links.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRoutedIssuerTargetRevocationErrorKind {
    /// Exact target-revocation composition failed before target-link policy.
    TargetRevocation(X509OcspAuthenticatedTargetRevocationErrorKind),
    /// Caller-owned Name policy rejected the target-to-issuer relationship.
    TargetIssuerName(X509CertificateIssuerLinkErrorKind),
    /// Data-driven target signature routing or verification failed.
    TargetIssuerSignature(X509RsaSignatureRoutingErrorKind),
}

/// A redacted fully routed target-revocation composition failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRoutedIssuerTargetRevocationError {
    kind: X509OcspRoutedIssuerTargetRevocationErrorKind,
}

impl X509OcspRoutedIssuerTargetRevocationError {
    const fn new(kind: X509OcspRoutedIssuerTargetRevocationErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509OcspRoutedIssuerTargetRevocationErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspRoutedIssuerTargetRevocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP routed-issuer target revocation error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspRoutedIssuerTargetRevocationError {}

/// Non-copyable evidence for one exact authenticated target revocation result.
///
/// The retained responder proof establishes that one exact issuer authorized
/// the delegated responder whose key signed the exact response. The retained
/// revocation proof establishes that one exact `SingleResponse` in that signed
/// response names the exact target and issuer and is good and fresh at the
/// explicit retained observation time. The retained target link establishes
/// that the same issuer signed that same target certificate. The response has
/// exactly one status entry, and the retained validity proves the responder
/// certificate was valid at the revocation observation.
pub struct AuthenticatedX509OcspRsaSha256TargetRevocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
> {
    issuer_bound_responder: IssuerBoundX509OcspRsaSha256Responder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    revocation: ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    target_issuer_link:
        VerifiedX509RsaSha256IssuerLink<'target, 'target_encoded, 'issuer, 'issuer_encoded>,
    responder_validity_at_observation: ValidatedCertificateValidity,
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
    AuthenticatedX509OcspRsaSha256TargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return the exact issuer-authorized authenticated responder proof.
    pub const fn issuer_bound_responder(
        &self,
    ) -> &IssuerBoundX509OcspRsaSha256Responder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.issuer_bound_responder
    }

    /// Return the exact good-and-fresh target/issuer revocation proof.
    pub const fn revocation(
        &self,
    ) -> &ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.revocation
    }

    /// Return the exact authenticated target-to-issuer certificate link.
    pub const fn target_issuer_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerLink<'target, 'target_encoded, 'issuer, 'issuer_encoded> {
        &self.target_issuer_link
    }

    /// Return responder-certificate validity at the revocation observation.
    pub const fn responder_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.responder_validity_at_observation
    }
}

impl fmt::Debug
    for AuthenticatedX509OcspRsaSha256TargetRevocation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthenticatedX509OcspRsaSha256TargetRevocation")
    }
}

/// Non-copyable routed evidence for one exact authenticated target result.
///
/// The retained responder proof preserves caller-routed SHA-256, SHA-384, or
/// SHA-512 response-signature verification. The responder-certificate and
/// target-certificate issuer links remain deliberately fixed to RSA SHA-256.
/// All exact-object, cardinality, extension, and observation-time guarantees
/// are identical to the fixed result.
pub struct AuthenticatedX509OcspRsaTargetRevocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
> {
    issuer_bound_responder: IssuerBoundX509OcspRsaResponder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    revocation: ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    target_issuer_link:
        VerifiedX509RsaSha256IssuerLink<'target, 'target_encoded, 'issuer, 'issuer_encoded>,
    responder_validity_at_observation: ValidatedCertificateValidity,
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
    AuthenticatedX509OcspRsaTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return the exact routed issuer-authorized responder proof.
    pub const fn issuer_bound_responder(
        &self,
    ) -> &IssuerBoundX509OcspRsaResponder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.issuer_bound_responder
    }

    /// Return the exact good-and-fresh target/issuer revocation proof.
    pub const fn revocation(
        &self,
    ) -> &ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.revocation
    }

    /// Return the fixed RSA SHA-256 target-to-issuer certificate link.
    pub const fn target_issuer_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerLink<'target, 'target_encoded, 'issuer, 'issuer_encoded> {
        &self.target_issuer_link
    }

    /// Return responder-certificate validity at the revocation observation.
    pub const fn responder_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.responder_validity_at_observation
    }
}

impl fmt::Debug
    for AuthenticatedX509OcspRsaTargetRevocation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthenticatedX509OcspRsaTargetRevocation")
    }
}

/// Non-copyable evidence with routed response and both certificate issuer links.
///
/// The responder proof retains caller-routed response-signature and responder-
/// certificate issuer-signature verification. The target signature retains
/// caller-routed target-certificate issuer verification. All three routes select
/// only repository-owned RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512
/// verifiers. Exact-object, cardinality, extension, and observation-time
/// guarantees remain identical to the existing target-revocation results.
pub struct AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
> {
    issuer_bound_responder: IssuerBoundX509OcspRsaRoutedIssuerResponder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    revocation: ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    target_issuer_signature:
        VerifiedX509RsaPolicySignature<'target, 'target_encoded, 'issuer, 'issuer_encoded>,
    responder_validity_at_observation: ValidatedCertificateValidity,
}

impl<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
    AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return routed response and responder-certificate issuer evidence.
    #[must_use]
    pub const fn issuer_bound_responder(
        &self,
    ) -> &IssuerBoundX509OcspRsaRoutedIssuerResponder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.issuer_bound_responder
    }

    /// Return the exact good-and-fresh target/issuer revocation proof.
    #[must_use]
    pub const fn revocation(
        &self,
    ) -> &ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    > {
        &self.revocation
    }

    /// Return routed verification of the exact target certificate signature.
    #[must_use]
    pub const fn target_issuer_signature(
        &self,
    ) -> &VerifiedX509RsaPolicySignature<'target, 'target_encoded, 'issuer, 'issuer_encoded> {
        &self.target_issuer_signature
    }

    /// Return responder-certificate validity at the revocation observation.
    #[must_use]
    pub const fn responder_validity_at_observation(&self) -> ValidatedCertificateValidity {
        self.responder_validity_at_observation
    }
}

impl fmt::Debug
    for AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation")
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn validate_target_revocation_inputs<
    'binding,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
>(
    authenticated_response: &X509OcspBasicResponse<'basic_encoded>,
    responder_certificate: &X509Certificate<'responder_encoded>,
    responder_candidate_issuer: &X509Certificate<'issuer_encoded>,
    revocation: &ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    target_certificate: &X509Certificate<'target_encoded>,
    target_candidate_issuer: &X509Certificate<'issuer_encoded>,
) -> Result<ValidatedCertificateValidity, X509OcspAuthenticatedTargetRevocationError> {
    let cert_id_binding = revocation.cert_id_binding();

    // Without retaining the caller's CertID hash policy, this boundary cannot
    // safely determine whether another entry encoded with a different hash
    // algorithm names the same certificate. Requiring exactly one entry is
    // deliberately stricter than accepting a potentially conflicting set.
    if authenticated_response.response_data().len() != 1 {
        return Err(X509OcspAuthenticatedTargetRevocationError::new(
            X509OcspAuthenticatedTargetRevocationErrorKind::AmbiguousResponseSet,
        ));
    }

    if !authenticated_response
        .response_data()
        .iter()
        .any(|response| std::ptr::eq(response, cert_id_binding.response()))
    {
        return Err(X509OcspAuthenticatedTargetRevocationError::new(
            X509OcspAuthenticatedTargetRevocationErrorKind::ResponseMismatch,
        ));
    }

    if !std::ptr::eq(target_certificate, cert_id_binding.target()) {
        return Err(X509OcspAuthenticatedTargetRevocationError::new(
            X509OcspAuthenticatedTargetRevocationErrorKind::TargetMismatch,
        ));
    }

    if !std::ptr::eq(responder_candidate_issuer, cert_id_binding.issuer())
        || !std::ptr::eq(target_candidate_issuer, cert_id_binding.issuer())
    {
        return Err(X509OcspAuthenticatedTargetRevocationError::new(
            X509OcspAuthenticatedTargetRevocationErrorKind::IssuerMismatch,
        ));
    }

    if authenticated_response
        .response_data()
        .response_extensions()
        .is_some()
    {
        return Err(X509OcspAuthenticatedTargetRevocationError::new(
            X509OcspAuthenticatedTargetRevocationErrorKind::UnsupportedResponseExtensions,
        ));
    }

    validate_certificate_validity_at(responder_certificate, &revocation.observation_time()).map_err(
        |error| {
            X509OcspAuthenticatedTargetRevocationError::new(
                X509OcspAuthenticatedTargetRevocationErrorKind::ResponderCertificateValidity(
                    error.kind(),
                ),
            )
        },
    )
}

/// Compose one exact signed, issuer-authorized, good-and-fresh target result.
///
/// Failure order is stable: response-set cardinality, signed-response
/// membership, target identity, shared issuer identity, unsupported outer
/// response extensions, then responder validity at the revocation
/// observation. Every identity check uses the decoded object address, so
/// byte-equal re-decodes are rejected rather than silently substituted.
#[allow(clippy::type_complexity)]
pub fn authenticate_x509_ocsp_rsa_sha256_target_revocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
>(
    issuer_bound_responder: IssuerBoundX509OcspRsaSha256Responder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    revocation: ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    target_issuer_link: VerifiedX509RsaSha256IssuerLink<
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
) -> Result<
    AuthenticatedX509OcspRsaSha256TargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspAuthenticatedTargetRevocationError,
> {
    let authenticated_response = issuer_bound_responder
        .authenticated_responder()
        .responder_id_binding()
        .response();
    let responder_certificate = issuer_bound_responder
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let responder_candidate_issuer = issuer_bound_responder
        .responder_issuer_link()
        .candidate_issuer();
    let responder_validity_at_observation = validate_target_revocation_inputs(
        authenticated_response,
        responder_certificate,
        responder_candidate_issuer,
        &revocation,
        target_issuer_link.child(),
        target_issuer_link.candidate_issuer(),
    )?;

    Ok(AuthenticatedX509OcspRsaSha256TargetRevocation {
        issuer_bound_responder,
        revocation,
        target_issuer_link,
        responder_validity_at_observation,
    })
}

/// Compose one routed signed, issuer-authorized, good-and-fresh target result.
///
/// The response proof preserves the caller-selected RSA SHA-256, SHA-384, or
/// SHA-512 verification route. Both certificate issuer links remain fixed to
/// RSA SHA-256. Failure order and exact-object checks are identical to the
/// fixed constructor; byte-equal re-decodes are rejected.
#[allow(clippy::type_complexity)]
pub fn authenticate_x509_ocsp_rsa_target_revocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
>(
    issuer_bound_responder: IssuerBoundX509OcspRsaResponder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    revocation: ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    target_issuer_link: VerifiedX509RsaSha256IssuerLink<
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
) -> Result<
    AuthenticatedX509OcspRsaTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspAuthenticatedTargetRevocationError,
> {
    let authenticated_response = issuer_bound_responder
        .authenticated_responder()
        .responder_id_binding()
        .response();
    let responder_certificate = issuer_bound_responder
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let responder_candidate_issuer = issuer_bound_responder
        .responder_issuer_link()
        .candidate_issuer();
    let responder_validity_at_observation = validate_target_revocation_inputs(
        authenticated_response,
        responder_certificate,
        responder_candidate_issuer,
        &revocation,
        target_issuer_link.child(),
        target_issuer_link.candidate_issuer(),
    )?;

    Ok(AuthenticatedX509OcspRsaTargetRevocation {
        issuer_bound_responder,
        revocation,
        target_issuer_link,
        responder_validity_at_observation,
    })
}

/// Compose one exact target result with fully routed RSA issuer signatures.
///
/// The consumed responder proof has already routed both the response signature
/// and responder-certificate issuer signature. This boundary first applies the
/// existing response cardinality, membership, target identity, shared issuer,
/// extension, and responder-validity checks. Only then does it invoke caller-
/// owned target-to-issuer Name policy once and route the exact target
/// certificate signature through a repository-owned fixed verifier.
///
/// This pure public-data composition performs no credential access, edit,
/// signing operation, storage, clock acquisition, provider action, audit or
/// trace emission, I/O, or external effect. It does not construct or validate
/// a certificate path, establish trust, check responder revocation, fetch
/// revocation data, perform TLS or HTTP, authorize OAuth transport, or grant
/// network authority.
#[allow(clippy::type_complexity)]
pub fn authenticate_x509_ocsp_rsa_routed_issuer_target_revocation<
    'binding,
    'basic_response,
    'basic_encoded,
    'single_response,
    'single_encoded,
    'responder,
    'responder_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
>(
    issuer_bound_responder: IssuerBoundX509OcspRsaRoutedIssuerResponder<
        'basic_response,
        'basic_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    revocation: ValidatedBoundX509OcspRevocation<
        'binding,
        'single_response,
        'single_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    target_algorithm_policy: ValidatedCertificateAlgorithmPolicy<'target, 'target_encoded>,
    target_issuer_rsa_algorithm_policy: ValidatedX509RsaAlgorithmPolicy<'issuer, 'issuer_encoded>,
    same_name: impl FnOnce(&X509Name<'target_encoded>, &X509Name<'issuer_encoded>) -> bool,
    target_signature_routes: &[X509RsaSignatureRoute],
    operation_limits: RsaPublicOperationLimits,
) -> Result<
    AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation<
        'binding,
        'basic_response,
        'basic_encoded,
        'single_response,
        'single_encoded,
        'responder,
        'responder_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspRoutedIssuerTargetRevocationError,
> {
    let authenticated_response = issuer_bound_responder
        .authenticated_responder()
        .responder_id_binding()
        .response();
    let responder_certificate = issuer_bound_responder
        .authenticated_responder()
        .responder_certificate_policy()
        .certificate();
    let responder_candidate_issuer = issuer_bound_responder.issuer_role_policy().certificate();
    let responder_validity_at_observation = validate_target_revocation_inputs(
        authenticated_response,
        responder_certificate,
        responder_candidate_issuer,
        &revocation,
        target_algorithm_policy.certificate(),
        target_issuer_rsa_algorithm_policy.certificate(),
    )
    .map_err(|error| {
        X509OcspRoutedIssuerTargetRevocationError::new(
            X509OcspRoutedIssuerTargetRevocationErrorKind::TargetRevocation(error.kind()),
        )
    })?;

    validate_certificate_issuer_link(
        target_algorithm_policy.certificate(),
        target_issuer_rsa_algorithm_policy.certificate(),
        same_name,
    )
    .map_err(|error| {
        X509OcspRoutedIssuerTargetRevocationError::new(
            X509OcspRoutedIssuerTargetRevocationErrorKind::TargetIssuerName(error.kind()),
        )
    })?;

    let target_issuer_signature = verify_x509_rsa_policy_signature(
        target_algorithm_policy,
        target_issuer_rsa_algorithm_policy,
        target_signature_routes,
        operation_limits,
    )
    .map_err(|error| {
        X509OcspRoutedIssuerTargetRevocationError::new(
            X509OcspRoutedIssuerTargetRevocationErrorKind::TargetIssuerSignature(error.kind()),
        )
    })?;

    Ok(AuthenticatedX509OcspRsaRoutedIssuerTargetRevocation {
        issuer_bound_responder,
        revocation,
        target_issuer_signature,
        responder_validity_at_observation,
    })
}
