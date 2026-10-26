//! # Exact-certificate OCSP responder policy composition
//!
//! OCSP responder authorization depends on several policy decisions about one
//! candidate certificate. Detached success tokens can otherwise be mixed
//! across independently decoded objects. This crate consumes repository-owned
//! certificate-wide role and algorithm evidence, requires both to retain one
//! exact end-entity certificate, and derives explicit-time validity and the
//! fixed RFC 6960 OCSP-signing purpose from that object.
//!
//! This target-certificate policy does not bind a responder identifier, verify
//! a response signature, prove issuer or delegated authorization, construct a
//! path, establish trust, check revocation, or grant TLS or network authority.

#![deny(unsafe_code)]

use core::fmt;
use x509_certificate_policy::{
    validate_certificate_extended_key_purpose, validate_certificate_validity_at,
    ValidatedCertificateAlgorithmPolicy, ValidatedCertificatePurpose,
    ValidatedCertificateRolePolicy, ValidatedCertificateValidity, X509CertificatePurposeErrorKind,
    X509CertificatePurposePolicy, X509CertificateRole, X509CertificateValidityErrorKind,
};
use x509_key_usage::KeyUsageBit;
use x509_time::X509Time;

/// RFC 6960 `id-kp-OCSPSigning` Extended Key Usage object identifier.
pub const OCSP_SIGNING_EXTENDED_KEY_PURPOSE_OID: &[u64] = &[1, 3, 6, 1, 5, 5, 7, 3, 9];

/// Stable, payload-free OCSP responder-certificate policy failure categories.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspResponderCertificatePolicyErrorKind {
    /// The role and algorithm evidence retain different decoded objects.
    EvidenceMismatch,
    /// The retained role policy is not for an end entity.
    NotEndEntity,
    /// A present Key Usage permits neither signature-capable usage.
    IncompatibleKeyUsage,
    /// The explicit observation was outside the certificate's validity.
    Validity(X509CertificateValidityErrorKind),
    /// The fixed OCSP-signing Extended Key Usage purpose was not authorized.
    Purpose(X509CertificatePurposeErrorKind),
}

/// A redacted OCSP responder-certificate policy failure.
///
/// No certificate bytes, times, or purpose identifiers are retained.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspResponderCertificatePolicyError {
    kind: X509OcspResponderCertificatePolicyErrorKind,
}

impl X509OcspResponderCertificatePolicyError {
    const fn new(kind: X509OcspResponderCertificatePolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable failure category.
    pub const fn kind(self) -> X509OcspResponderCertificatePolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspResponderCertificatePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP responder certificate policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspResponderCertificatePolicyError {}

/// Non-copyable policy evidence for one exact decoded responder certificate.
///
/// The retained inputs prove caller-owned critical-extension recognition,
/// end-entity role and compatible present-Key-Usage semantics, and one
/// caller-selected algorithm profile. The derived tokens prove one explicit
/// validity observation and the fixed OCSP-signing purpose for that same
/// object.
pub struct ValidatedX509OcspResponderCertificatePolicy<'certificate, 'encoded> {
    role_policy: ValidatedCertificateRolePolicy<'certificate, 'encoded>,
    algorithm_policy: ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded>,
    validity: ValidatedCertificateValidity,
    purpose: ValidatedCertificatePurpose,
}

impl<'certificate, 'encoded> ValidatedX509OcspResponderCertificatePolicy<'certificate, 'encoded> {
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

    /// Return the explicit-time validity result.
    pub const fn validity(&self) -> ValidatedCertificateValidity {
        self.validity
    }

    /// Return the fixed OCSP-signing purpose result.
    pub const fn purpose(&self) -> ValidatedCertificatePurpose {
        self.purpose
    }
}

impl fmt::Debug for ValidatedX509OcspResponderCertificatePolicy<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509OcspResponderCertificatePolicy")
    }
}

/// Compose candidate-responder policy for one exact decoded certificate.
///
/// Evidence identity and the end-entity role are checked before the derived
/// validity and fixed OCSP-signing-purpose decisions. `observed` is explicit
/// caller-owned policy; this function does not acquire ambient time.
pub fn validate_x509_ocsp_responder_certificate_policy<'certificate, 'encoded>(
    role_policy: ValidatedCertificateRolePolicy<'certificate, 'encoded>,
    algorithm_policy: ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded>,
    observed: &X509Time,
) -> Result<
    ValidatedX509OcspResponderCertificatePolicy<'certificate, 'encoded>,
    X509OcspResponderCertificatePolicyError,
> {
    if !std::ptr::eq(role_policy.certificate(), algorithm_policy.certificate()) {
        return Err(X509OcspResponderCertificatePolicyError::new(
            X509OcspResponderCertificatePolicyErrorKind::EvidenceMismatch,
        ));
    }
    if role_policy.role_extensions().role() != X509CertificateRole::EndEntity {
        return Err(X509OcspResponderCertificatePolicyError::new(
            X509OcspResponderCertificatePolicyErrorKind::NotEndEntity,
        ));
    }
    if let Some(key_usage) = role_policy.role_extensions().key_usage() {
        let permits_signature = key_usage.contains(KeyUsageBit::DigitalSignature)
            || key_usage.contains(KeyUsageBit::ContentCommitment);
        if !permits_signature {
            return Err(X509OcspResponderCertificatePolicyError::new(
                X509OcspResponderCertificatePolicyErrorKind::IncompatibleKeyUsage,
            ));
        }
    }

    let certificate = role_policy.certificate();
    let validity = validate_certificate_validity_at(certificate, observed).map_err(|error| {
        X509OcspResponderCertificatePolicyError::new(
            X509OcspResponderCertificatePolicyErrorKind::Validity(error.kind()),
        )
    })?;
    let purpose_policy = X509CertificatePurposePolicy::new(OCSP_SIGNING_EXTENDED_KEY_PURPOSE_OID)
        .expect("the fixed OCSP-signing purpose OID is non-empty");
    let purpose =
        validate_certificate_extended_key_purpose(role_policy.role_extensions(), purpose_policy)
            .map_err(|error| {
                X509OcspResponderCertificatePolicyError::new(
                    X509OcspResponderCertificatePolicyErrorKind::Purpose(error.kind()),
                )
            })?;

    Ok(ValidatedX509OcspResponderCertificatePolicy {
        role_policy,
        algorithm_policy,
        validity,
        purpose,
    })
}
