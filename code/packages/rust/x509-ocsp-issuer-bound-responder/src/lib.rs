//! # Exact issuer binding for authenticated OCSP responder candidates
//!
//! An authenticated responder candidate and a valid certificate issuer link are
//! insufficient when their evidence can retain different responder certificate
//! objects. This crate consumes both proofs, rejects cross-certificate mixing,
//! and returns non-copyable evidence retaining the exact authenticated response,
//! responder certificate, and candidate issuer relationship.
//!
//! The fixed result retains fixed SHA-256 response authentication. The additive
//! routed result retains caller-selected SHA-256, SHA-384, or SHA-512 response-
//! signature evidence. Existing entry points keep the responder-certificate
//! issuer link fixed to RSA SHA-256. A separate additive entry point consumes
//! exact certificate-bound policy and routes that link through the repository-
//! owned RSA SHA-256, SHA-384, or SHA-512 verifier. All results prove that one
//! candidate issuer signed the policy-approved responder certificate whose key
//! signed one exact OCSP response. None
//! establishes that the candidate issuer issued a particular target certificate,
//! binds a `CertID` or revocation decision, validates a responder path,
//! establishes trust, checks responder revocation, or grants TLS, HTTP,
//! OAuth-transport, or network authority.

#![deny(unsafe_code)]

use core::fmt;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_certificate_policy::{
    validate_certificate_issuer_link, ValidatedCertificateAlgorithmPolicy,
    ValidatedCertificateRolePolicy, X509CertificateIssuerLinkErrorKind, X509CertificateRole,
};
use x509_name::X509Name;
use x509_ocsp_authenticated_responder::{
    AuthenticatedX509OcspRsaResponder, AuthenticatedX509OcspRsaSha256Responder,
};
use x509_rsa_algorithm_policy::ValidatedX509RsaAlgorithmPolicy;
use x509_rsa_sha256_issuer_link::VerifiedX509RsaSha256IssuerLink;
use x509_rsa_signature_routing::{
    verify_x509_rsa_policy_signature, VerifiedX509RsaPolicySignature, X509RsaSignatureRoute,
    X509RsaSignatureRoutingErrorKind,
};

/// Stable failures that retain no response, certificate, key, or signature bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspIssuerBoundResponderErrorKind {
    /// Authentication and issuer-link evidence retain distinct responder objects.
    EvidenceMismatch,
}

/// A redacted failure while binding responder authentication to an issuer link.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspIssuerBoundResponderError {
    kind: X509OcspIssuerBoundResponderErrorKind,
}

impl X509OcspIssuerBoundResponderError {
    const fn new(kind: X509OcspIssuerBoundResponderErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspIssuerBoundResponderErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspIssuerBoundResponderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP issuer-bound responder error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspIssuerBoundResponderError {}

/// Stable failures for routed responder-certificate issuer authentication.
///
/// Every category is payload-free: certificate, Name, algorithm, key, route,
/// and signature values remain inside consumed evidence or the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRoutedIssuerBoundResponderErrorKind {
    /// Response authentication and child algorithm policy retain different responders.
    ResponderAlgorithmEvidenceMismatch,
    /// Issuer role and RSA algorithm policy retain different candidate issuers.
    IssuerAlgorithmEvidenceMismatch,
    /// The exact candidate issuer does not retain an issuing-CA role.
    IssuerNotCertificateAuthority,
    /// Caller-owned Name policy rejected the responder-to-issuer relationship.
    IssuerName(X509CertificateIssuerLinkErrorKind),
    /// Data-driven routing or its selected repository-owned verifier failed.
    Signature(X509RsaSignatureRoutingErrorKind),
}

/// A redacted routed responder-certificate issuer-link failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRoutedIssuerBoundResponderError {
    kind: X509OcspRoutedIssuerBoundResponderErrorKind,
}

impl X509OcspRoutedIssuerBoundResponderError {
    const fn new(kind: X509OcspRoutedIssuerBoundResponderErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509OcspRoutedIssuerBoundResponderErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspRoutedIssuerBoundResponderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP routed issuer-bound responder error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspRoutedIssuerBoundResponderError {}

/// Non-copyable evidence binding one authenticated responder to one issuer.
///
/// The retained responder proof covers exact `ResponderID`, certificate policy,
/// and response signature evidence. The retained issuer link proves that the
/// same responder certificate was signed by the exact candidate issuer, whose
/// issuing-CA role and RSA public key were validated by that link.
pub struct IssuerBoundX509OcspRsaSha256Responder<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
    'issuer,
    'issuer_encoded,
> {
    authenticated_responder: AuthenticatedX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    >,
    responder_issuer_link:
        VerifiedX509RsaSha256IssuerLink<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>,
}

impl<'response, 'response_encoded, 'responder, 'responder_encoded, 'issuer, 'issuer_encoded>
    IssuerBoundX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return authentication for the exact response and responder certificate.
    pub const fn authenticated_responder(
        &self,
    ) -> &AuthenticatedX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    > {
        &self.authenticated_responder
    }

    /// Return the exact responder-certificate-to-candidate-issuer link.
    pub const fn responder_issuer_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerLink<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>
    {
        &self.responder_issuer_link
    }
}

impl fmt::Debug for IssuerBoundX509OcspRsaSha256Responder<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IssuerBoundX509OcspRsaSha256Responder")
    }
}

/// Non-copyable evidence binding one routed authenticated responder to one issuer.
///
/// The retained responder proof covers exact `ResponderID`, certificate policy,
/// and caller-routed SHA-256, SHA-384, or SHA-512 response-signature evidence.
/// The retained issuer link is deliberately narrower: it proves that the same
/// responder certificate was signed by the exact candidate issuer through the
/// existing fixed RSA SHA-256 issuer-link boundary. This type does not claim
/// that the responder-certificate issuer-link algorithm is data-driven.
pub struct IssuerBoundX509OcspRsaResponder<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
    'issuer,
    'issuer_encoded,
> {
    authenticated_responder: AuthenticatedX509OcspRsaResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    >,
    responder_issuer_link:
        VerifiedX509RsaSha256IssuerLink<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>,
}

impl<'response, 'response_encoded, 'responder, 'responder_encoded, 'issuer, 'issuer_encoded>
    IssuerBoundX509OcspRsaResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return routed authentication for the exact response and responder certificate.
    pub const fn authenticated_responder(
        &self,
    ) -> &AuthenticatedX509OcspRsaResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    > {
        &self.authenticated_responder
    }

    /// Return the fixed RSA SHA-256 responder-certificate issuer link.
    pub const fn responder_issuer_link(
        &self,
    ) -> &VerifiedX509RsaSha256IssuerLink<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>
    {
        &self.responder_issuer_link
    }
}

impl fmt::Debug for IssuerBoundX509OcspRsaResponder<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IssuerBoundX509OcspRsaResponder")
    }
}

/// Non-copyable evidence binding routed response and responder-link verification.
///
/// The response proof retains the exact matched responder certificate, its
/// certificate policy, and its caller-routed OCSP response signature. The
/// issuer role policy and routed certificate-signature proof retain the same
/// exact candidate issuer. Construction is private so neither endpoint,
/// policy, route, nor selected fixed verifier can be replaced afterward.
pub struct IssuerBoundX509OcspRsaRoutedIssuerResponder<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
    'issuer,
    'issuer_encoded,
> {
    authenticated_responder: AuthenticatedX509OcspRsaResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    >,
    issuer_role_policy: ValidatedCertificateRolePolicy<'issuer, 'issuer_encoded>,
    responder_issuer_signature:
        VerifiedX509RsaPolicySignature<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>,
}

impl<'response, 'response_encoded, 'responder, 'responder_encoded, 'issuer, 'issuer_encoded>
    IssuerBoundX509OcspRsaRoutedIssuerResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return routed authentication for the exact response and responder certificate.
    #[must_use]
    pub const fn authenticated_responder(
        &self,
    ) -> &AuthenticatedX509OcspRsaResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    > {
        &self.authenticated_responder
    }

    /// Return certificate-wide issuing-CA policy for the exact candidate issuer.
    #[must_use]
    pub const fn issuer_role_policy(
        &self,
    ) -> &ValidatedCertificateRolePolicy<'issuer, 'issuer_encoded> {
        &self.issuer_role_policy
    }

    /// Return routed verification of the exact responder certificate signature.
    #[must_use]
    pub const fn responder_issuer_signature(
        &self,
    ) -> &VerifiedX509RsaPolicySignature<'responder, 'responder_encoded, 'issuer, 'issuer_encoded>
    {
        &self.responder_issuer_signature
    }
}

impl fmt::Debug for IssuerBoundX509OcspRsaRoutedIssuerResponder<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("IssuerBoundX509OcspRsaRoutedIssuerResponder")
    }
}

/// Bind one exact authenticated responder to its exact candidate issuer link.
///
/// Object identity is checked before evidence is returned. Byte-equal but
/// separately decoded responder certificates are rejected so a valid issuer
/// link cannot be attached to authentication evidence for another object.
pub fn bind_x509_ocsp_authenticated_responder_to_issuer<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
    'issuer,
    'issuer_encoded,
>(
    authenticated_responder: AuthenticatedX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    >,
    responder_issuer_link: VerifiedX509RsaSha256IssuerLink<
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
) -> Result<
    IssuerBoundX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspIssuerBoundResponderError,
> {
    if !std::ptr::eq(
        authenticated_responder
            .responder_certificate_policy()
            .certificate(),
        responder_issuer_link.child(),
    ) {
        return Err(X509OcspIssuerBoundResponderError::new(
            X509OcspIssuerBoundResponderErrorKind::EvidenceMismatch,
        ));
    }

    Ok(IssuerBoundX509OcspRsaSha256Responder {
        authenticated_responder,
        responder_issuer_link,
    })
}

/// Bind one exact routed authenticated responder to its exact fixed issuer link.
///
/// Object identity is checked before evidence is returned. Byte-equal but
/// separately decoded responder certificates are rejected. The response
/// signature route remains retained by the authentication proof, while the
/// certificate issuer link remains explicitly fixed to RSA SHA-256.
pub fn bind_x509_ocsp_authenticated_rsa_responder_to_issuer<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
    'issuer,
    'issuer_encoded,
>(
    authenticated_responder: AuthenticatedX509OcspRsaResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    >,
    responder_issuer_link: VerifiedX509RsaSha256IssuerLink<
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
) -> Result<
    IssuerBoundX509OcspRsaResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspIssuerBoundResponderError,
> {
    if !std::ptr::eq(
        authenticated_responder
            .responder_certificate_policy()
            .certificate(),
        responder_issuer_link.child(),
    ) {
        return Err(X509OcspIssuerBoundResponderError::new(
            X509OcspIssuerBoundResponderErrorKind::EvidenceMismatch,
        ));
    }

    Ok(IssuerBoundX509OcspRsaResponder {
        authenticated_responder,
        responder_issuer_link,
    })
}

/// Authenticate one routed responder-certificate link to one exact issuer.
///
/// All object-identity and issuing-CA checks run before `same_name` or RSA
/// verification. The Name callback then runs exactly once with the exact
/// responder issuer Name followed by the exact candidate subject Name. Only
/// after that relationship succeeds may caller-owned profile routes select a
/// repository-owned RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 verifier.
///
/// This pure public-data composition performs no credential access, signing,
/// storage, clock acquisition, audit, trace, I/O, or external effect. It does
/// not construct a path, establish trust, decide revocation, or grant TLS,
/// HTTP, OAuth-transport, or network authority.
#[allow(clippy::too_many_arguments)]
pub fn verify_x509_ocsp_authenticated_rsa_responder_routed_issuer_link<
    'response,
    'response_encoded,
    'responder,
    'responder_encoded,
    'issuer,
    'issuer_encoded,
>(
    authenticated_responder: AuthenticatedX509OcspRsaResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
    >,
    responder_algorithm_policy: ValidatedCertificateAlgorithmPolicy<'responder, 'responder_encoded>,
    issuer_role_policy: ValidatedCertificateRolePolicy<'issuer, 'issuer_encoded>,
    issuer_rsa_algorithm_policy: ValidatedX509RsaAlgorithmPolicy<'issuer, 'issuer_encoded>,
    same_name: impl FnOnce(&X509Name<'responder_encoded>, &X509Name<'issuer_encoded>) -> bool,
    routes: &[X509RsaSignatureRoute],
    operation_limits: RsaPublicOperationLimits,
) -> Result<
    IssuerBoundX509OcspRsaRoutedIssuerResponder<
        'response,
        'response_encoded,
        'responder,
        'responder_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspRoutedIssuerBoundResponderError,
> {
    if !core::ptr::eq(
        authenticated_responder
            .responder_certificate_policy()
            .certificate(),
        responder_algorithm_policy.certificate(),
    ) {
        return Err(X509OcspRoutedIssuerBoundResponderError::new(
            X509OcspRoutedIssuerBoundResponderErrorKind::ResponderAlgorithmEvidenceMismatch,
        ));
    }

    if !core::ptr::eq(
        issuer_role_policy.certificate(),
        issuer_rsa_algorithm_policy.certificate(),
    ) {
        return Err(X509OcspRoutedIssuerBoundResponderError::new(
            X509OcspRoutedIssuerBoundResponderErrorKind::IssuerAlgorithmEvidenceMismatch,
        ));
    }
    if issuer_role_policy.role_extensions().role()
        != X509CertificateRole::IssuingCertificateAuthority
    {
        return Err(X509OcspRoutedIssuerBoundResponderError::new(
            X509OcspRoutedIssuerBoundResponderErrorKind::IssuerNotCertificateAuthority,
        ));
    }

    validate_certificate_issuer_link(
        responder_algorithm_policy.certificate(),
        issuer_role_policy.certificate(),
        same_name,
    )
    .map_err(|error| {
        X509OcspRoutedIssuerBoundResponderError::new(
            X509OcspRoutedIssuerBoundResponderErrorKind::IssuerName(error.kind()),
        )
    })?;

    let responder_issuer_signature = verify_x509_rsa_policy_signature(
        responder_algorithm_policy,
        issuer_rsa_algorithm_policy,
        routes,
        operation_limits,
    )
    .map_err(|error| {
        X509OcspRoutedIssuerBoundResponderError::new(
            X509OcspRoutedIssuerBoundResponderErrorKind::Signature(error.kind()),
        )
    })?;

    Ok(IssuerBoundX509OcspRsaRoutedIssuerResponder {
        authenticated_responder,
        issuer_role_policy,
        responder_issuer_signature,
    })
}
