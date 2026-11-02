//! # Certificate-bound RSA algorithm and key policy
//!
//! A certificate algorithm profile validates caller-selected identifier,
//! parameter, and encoded bit-length policy for one exact decoded certificate.
//! That evidence deliberately does not parse the subject key. This crate
//! composes it with the repository-owned RSA public-key validator without
//! allowing another certificate or detached SubjectPublicKeyInfo to enter the
//! boundary.
//!
//! The result is public-data policy evidence only. It does not select a
//! signature verifier, authenticate the certificate, assign a role, establish
//! path or trust, check identity or revocation, or grant TLS, HTTP, OAuth, or
//! network authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::Asn1Limits;
use x509_certificate::X509Certificate;
use x509_certificate_policy::ValidatedCertificateAlgorithmPolicy;
use x509_rsa_public_key::{
    validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy, ValidatedRsaPublicKey,
};

/// Stable failures that retain no certificate, key, modulus, exponent, or policy values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaAlgorithmPolicyErrorKind {
    RsaPublicKey(RsaPublicKeyErrorKind),
}

/// A redacted certificate-bound RSA algorithm-policy failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaAlgorithmPolicyError {
    kind: X509RsaAlgorithmPolicyErrorKind,
}

impl X509RsaAlgorithmPolicyError {
    const fn new(kind: X509RsaAlgorithmPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaAlgorithmPolicyErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaAlgorithmPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA algorithm policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaAlgorithmPolicyError {}

/// Non-copyable policy evidence for one exact certificate and its RSA key.
///
/// The algorithm-policy input retains the exact certificate object and the
/// selected caller profile. The RSA key borrows only that certificate's exact
/// SubjectPublicKeyInfo bytes and has passed caller-owned modulus and exponent
/// policy. This remains policy evidence, not a signature or trust verdict.
pub struct ValidatedX509RsaAlgorithmPolicy<'certificate, 'encoded> {
    algorithm_policy: ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded>,
    rsa_public_key: ValidatedRsaPublicKey<'encoded>,
}

impl<'certificate, 'encoded> ValidatedX509RsaAlgorithmPolicy<'certificate, 'encoded> {
    /// Return the exact certificate from which every result was derived.
    #[must_use]
    pub const fn certificate(&self) -> &'certificate X509Certificate<'encoded> {
        self.algorithm_policy.certificate()
    }

    /// Return the consumed data-driven certificate algorithm-policy evidence.
    #[must_use]
    pub const fn algorithm_policy(
        &self,
    ) -> &ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded> {
        &self.algorithm_policy
    }

    /// Return the exact validated RSA public key borrowed from the certificate.
    #[must_use]
    pub const fn rsa_public_key(&self) -> ValidatedRsaPublicKey<'encoded> {
        self.rsa_public_key
    }
}

impl fmt::Debug for ValidatedX509RsaAlgorithmPolicy<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedX509RsaAlgorithmPolicy")
    }
}

/// Validate the exact retained certificate's SubjectPublicKeyInfo as RSA.
///
/// Identifier, parameter, and encoded bit-length selection has already been
/// performed by `algorithm_policy`. This composition derives the SPKI only
/// from the certificate retained by that non-copyable evidence, then applies
/// exact RSA structure plus caller-supplied modulus and exponent policy.
pub fn validate_x509_rsa_algorithm_policy<'certificate, 'encoded, 'policy>(
    algorithm_policy: ValidatedCertificateAlgorithmPolicy<'certificate, 'encoded>,
    rsa_key_policy: RsaPublicKeyPolicy<'policy>,
    rsa_key_limits: Asn1Limits,
) -> Result<ValidatedX509RsaAlgorithmPolicy<'certificate, 'encoded>, X509RsaAlgorithmPolicyError> {
    let subject_public_key_info = algorithm_policy
        .certificate()
        .tbs_certificate()
        .subject_public_key_info();
    let rsa_public_key =
        validate_rsa_public_key(subject_public_key_info, rsa_key_policy, rsa_key_limits).map_err(
            |error| {
                X509RsaAlgorithmPolicyError::new(X509RsaAlgorithmPolicyErrorKind::RsaPublicKey(
                    error.kind(),
                ))
            },
        )?;

    Ok(ValidatedX509RsaAlgorithmPolicy {
        algorithm_policy,
        rsa_public_key,
    })
}
