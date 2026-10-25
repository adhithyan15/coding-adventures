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
//! The fixed RSA SHA-256 result does not establish issuer trust, validate a
//! complete certificate or responder path, check responder revocation,
//! interpret OCSP extensions, perform TLS or HTTP, authorize OAuth transport,
//! or grant network authority.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate_policy::{
    validate_certificate_validity_at, ValidatedCertificateValidity,
    X509CertificateValidityErrorKind,
};
use x509_ocsp_issuer_bound_responder::IssuerBoundX509OcspRsaSha256Responder;
use x509_ocsp_revocation_policy::ValidatedBoundX509OcspRevocation;
use x509_rsa_sha256_issuer_link::VerifiedX509RsaSha256IssuerLink;

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

    if !std::ptr::eq(target_issuer_link.child(), cert_id_binding.target()) {
        return Err(X509OcspAuthenticatedTargetRevocationError::new(
            X509OcspAuthenticatedTargetRevocationErrorKind::TargetMismatch,
        ));
    }

    if !std::ptr::eq(
        issuer_bound_responder
            .responder_issuer_link()
            .candidate_issuer(),
        cert_id_binding.issuer(),
    ) || !std::ptr::eq(
        target_issuer_link.candidate_issuer(),
        cert_id_binding.issuer(),
    ) {
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

    let responder_validity_at_observation = validate_certificate_validity_at(
        issuer_bound_responder
            .authenticated_responder()
            .responder_certificate_policy()
            .certificate(),
        &revocation.observation_time(),
    )
    .map_err(|error| {
        X509OcspAuthenticatedTargetRevocationError::new(
            X509OcspAuthenticatedTargetRevocationErrorKind::ResponderCertificateValidity(
                error.kind(),
            ),
        )
    })?;

    Ok(AuthenticatedX509OcspRsaSha256TargetRevocation {
        issuer_bound_responder,
        revocation,
        target_issuer_link,
        responder_validity_at_observation,
    })
}
