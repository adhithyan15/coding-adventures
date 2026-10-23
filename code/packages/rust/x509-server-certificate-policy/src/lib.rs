//! # Exact-certificate X.509 server policy composition
//!
//! Certificate validation is not one boolean. It is a sequence of policy
//! decisions about one parsed object: critical extensions and role, algorithm
//! identifiers and encoded key sizes, the requested server identity, an
//! explicit time observation, and an explicit Extended Key Usage purpose. If
//! those decisions are returned as detached tokens, callers can accidentally
//! combine successes from different decoded certificates.
//!
//! This crate closes that evidence-mixing gap for the target certificate. It
//! consumes the existing certificate-bound role and algorithm results, checks
//! that both retain the same object, and derives the remaining server checks
//! from that exact certificate. It deliberately does not authenticate an
//! issuer, construct a path, apply anchor constraints, check revocation, or
//! grant TLS or network authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::Asn1Limits;
use tls_server_identity::ReferenceIdentity;
use x509_certificate_policy::{
    validate_certificate_extended_key_purpose, validate_certificate_server_identity,
    validate_certificate_validity_at, ValidatedCertificateAlgorithmPolicy,
    ValidatedCertificatePurpose, ValidatedCertificateRolePolicy,
    ValidatedCertificateServerIdentity, ValidatedCertificateValidity,
    X509CertificatePurposeErrorKind, X509CertificatePurposePolicy, X509CertificateRole,
    X509CertificateServerIdentityErrorKind, X509CertificateValidityErrorKind,
};
use x509_time::X509Time;

/// Stable, payload-free server-certificate policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509ServerCertificatePolicyErrorKind {
    /// The role and algorithm evidence retain different decoded objects.
    EvidenceMismatch,
    /// The retained role policy is not for an end entity.
    NotEndEntity,
    /// The exact certificate did not satisfy server-identity policy.
    ServerIdentity(X509CertificateServerIdentityErrorKind),
    /// The explicit observation was outside the certificate's validity.
    Validity(X509CertificateValidityErrorKind),
    /// The exact requested Extended Key Usage purpose was not authorized.
    Purpose(X509CertificatePurposeErrorKind),
}

/// A redacted server-certificate policy failure.
///
/// No certificate bytes, names, times, or purpose identifiers are retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509ServerCertificatePolicyError {
    kind: X509ServerCertificatePolicyErrorKind,
}

impl X509ServerCertificatePolicyError {
    const fn new(kind: X509ServerCertificatePolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    pub const fn kind(self) -> X509ServerCertificatePolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509ServerCertificatePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 server certificate policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509ServerCertificatePolicyError {}

/// Non-copyable policy evidence for one exact decoded server certificate.
///
/// The retained inputs prove caller-owned critical-extension recognition,
/// end-entity role semantics, and one caller-selected algorithm profile for
/// this exact object. The derived tokens prove one SAN-only identity match, one
/// explicit inclusive validity observation, and one exact Extended Key Usage
/// purpose. This is target policy only, not issuer authentication or path
/// trust.
pub struct ValidatedX509ServerCertificatePolicy<'certificate, 'encoded> {
    role_policy: ValidatedCertificateRolePolicy<'certificate, 'encoded>,
    algorithm_policy: ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded>,
    identity: ValidatedCertificateServerIdentity,
    validity: ValidatedCertificateValidity,
    purpose: ValidatedCertificatePurpose,
}

impl<'certificate, 'encoded> ValidatedX509ServerCertificatePolicy<'certificate, 'encoded> {
    /// Return the exact certificate retained by all policy evidence.
    pub const fn certificate(&self) -> &'certificate x509_certificate::X509Certificate<'encoded> {
        self.role_policy.certificate()
    }

    /// Return the consumed certificate-wide end-entity role policy.
    pub const fn role_policy(&self) -> &ValidatedCertificateRolePolicy<'certificate, 'encoded> {
        &self.role_policy
    }

    /// Return the consumed certificate-bound algorithm policy.
    pub const fn algorithm_policy(
        &self,
    ) -> &ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded> {
        &self.algorithm_policy
    }

    /// Return the SAN-only identity-match result.
    pub const fn identity(&self) -> ValidatedCertificateServerIdentity {
        self.identity
    }

    /// Return the explicit-time validity result.
    pub const fn validity(&self) -> ValidatedCertificateValidity {
        self.validity
    }

    /// Return the exact-purpose authorization result.
    pub const fn purpose(&self) -> ValidatedCertificatePurpose {
        self.purpose
    }
}

impl fmt::Debug for ValidatedX509ServerCertificatePolicy<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509ServerCertificatePolicy")
    }
}

/// Compose target-server policy for one exact decoded certificate object.
///
/// Evidence identity and the end-entity role are checked before any derived
/// server policy. The identity, validity, and purpose checks then use only the
/// certificate retained by `role_policy`; the caller cannot provide another
/// certificate to those steps. `reference`, `observed`, `purpose_policy`, and
/// `extension_limits` remain explicit caller-owned policy and authority.
pub fn validate_x509_server_certificate_policy<'certificate, 'encoded>(
    role_policy: ValidatedCertificateRolePolicy<'certificate, 'encoded>,
    algorithm_policy: ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded>,
    reference: &ReferenceIdentity,
    observed: &X509Time,
    purpose_policy: X509CertificatePurposePolicy<'_>,
    extension_limits: Asn1Limits,
) -> Result<
    ValidatedX509ServerCertificatePolicy<'certificate, 'encoded>,
    X509ServerCertificatePolicyError,
> {
    if !std::ptr::eq(role_policy.certificate(), algorithm_policy.certificate()) {
        return Err(X509ServerCertificatePolicyError::new(
            X509ServerCertificatePolicyErrorKind::EvidenceMismatch,
        ));
    }
    if role_policy.role_extensions().role() != X509CertificateRole::EndEntity {
        return Err(X509ServerCertificatePolicyError::new(
            X509ServerCertificatePolicyErrorKind::NotEndEntity,
        ));
    }

    let certificate = role_policy.certificate();
    let identity = validate_certificate_server_identity(certificate, reference, extension_limits)
        .map_err(|error| {
        X509ServerCertificatePolicyError::new(X509ServerCertificatePolicyErrorKind::ServerIdentity(
            error.kind(),
        ))
    })?;
    let validity = validate_certificate_validity_at(certificate, observed).map_err(|error| {
        X509ServerCertificatePolicyError::new(X509ServerCertificatePolicyErrorKind::Validity(
            error.kind(),
        ))
    })?;
    let purpose =
        validate_certificate_extended_key_purpose(role_policy.role_extensions(), purpose_policy)
            .map_err(|error| {
                X509ServerCertificatePolicyError::new(
                    X509ServerCertificatePolicyErrorKind::Purpose(error.kind()),
                )
            })?;

    Ok(ValidatedX509ServerCertificatePolicy {
        role_policy,
        algorithm_policy,
        identity,
        validity,
        purpose,
    })
}
