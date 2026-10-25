//! # Exact delegated OCSP responder candidate authentication
//!
//! A matching `ResponderID`, a policy-approved responder certificate, and a
//! valid response signature are individually insufficient when their evidence
//! can refer to different decoded objects. This crate consumes the first two
//! exact-input proofs, rejects cross-certificate mixing, derives the RSA key
//! from that same retained certificate, and verifies that the matched response
//! was signed by that key with repository-owned RSA PKCS #1 v1.5 SHA-256.
//!
//! The result authenticates one policy-approved delegated responder candidate
//! to one exact response. It does not prove that the target certificate's
//! issuer delegated OCSP authority to the responder, validate a responder path,
//! establish trust, check responder revocation, bind a `CertID` or revocation
//! decision, fetch data, or grant TLS, HTTP, OAuth-transport, or network
//! authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::Asn1Limits;
use rsa_public_operation::RsaPublicOperationLimits;
use x509_ocsp_responder_certificate_policy::ValidatedX509OcspResponderCertificatePolicy;
use x509_ocsp_responder_id_binding::BoundX509OcspResponderCertificate;
use x509_ocsp_rsa_pkcs1_v1_5_sha256_signature::{
    verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature, VerifiedX509OcspRsaSha256Signature,
    X509OcspRsaSha256SignatureErrorKind,
};
use x509_rsa_public_key::{validate_rsa_public_key, RsaPublicKeyErrorKind, RsaPublicKeyPolicy};

/// Caller-owned RSA strength and resource policy for candidate authentication.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRsaSha256ResponderPolicy<'policy> {
    responder_key_policy: RsaPublicKeyPolicy<'policy>,
    responder_key_limits: Asn1Limits,
    public_operation_limits: RsaPublicOperationLimits,
}

impl<'policy> X509OcspRsaSha256ResponderPolicy<'policy> {
    pub const fn new(
        responder_key_policy: RsaPublicKeyPolicy<'policy>,
        responder_key_limits: Asn1Limits,
        public_operation_limits: RsaPublicOperationLimits,
    ) -> Self {
        Self {
            responder_key_policy,
            responder_key_limits,
            public_operation_limits,
        }
    }

    pub const fn responder_key_policy(self) -> RsaPublicKeyPolicy<'policy> {
        self.responder_key_policy
    }

    pub const fn responder_key_limits(self) -> Asn1Limits {
        self.responder_key_limits
    }

    pub const fn public_operation_limits(self) -> RsaPublicOperationLimits {
        self.public_operation_limits
    }
}

/// Stable failures that retain no response, certificate, key, or signature bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspAuthenticatedResponderErrorKind {
    /// Responder-ID and certificate-policy evidence retain different objects.
    EvidenceMismatch,
    /// The exact policy-retained certificate failed caller-owned RSA key policy.
    ResponderPublicKey(RsaPublicKeyErrorKind),
    /// The exact matched response failed verification with that certificate key.
    ResponseSignature(X509OcspRsaSha256SignatureErrorKind),
}

/// A redacted delegated-responder-candidate authentication failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspAuthenticatedResponderError {
    kind: X509OcspAuthenticatedResponderErrorKind,
}

impl X509OcspAuthenticatedResponderError {
    const fn new(kind: X509OcspAuthenticatedResponderErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspAuthenticatedResponderErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspAuthenticatedResponderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP authenticated responder error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspAuthenticatedResponderError {}

/// Non-copyable evidence authenticating one responder candidate to one response.
///
/// The retained identifier binding fixes the response and candidate certificate;
/// the retained certificate policy covers critical extensions, end-entity role,
/// compatible present Key Usage, caller-selected algorithm policy, explicit-time
/// validity, and the fixed OCSP-signing EKU. The retained signature proof was
/// produced only after deriving the RSA key from that exact certificate.
pub struct AuthenticatedX509OcspRsaSha256Responder<
    'response,
    'response_encoded,
    'certificate,
    'certificate_encoded,
> {
    responder_id_binding: BoundX509OcspResponderCertificate<
        'response,
        'response_encoded,
        'certificate,
        'certificate_encoded,
    >,
    responder_certificate_policy:
        ValidatedX509OcspResponderCertificatePolicy<'certificate, 'certificate_encoded>,
    response_signature:
        VerifiedX509OcspRsaSha256Signature<'response, 'response_encoded, 'certificate_encoded>,
}

impl<'response, 'response_encoded, 'certificate, 'certificate_encoded>
    AuthenticatedX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'certificate,
        'certificate_encoded,
    >
{
    /// Return the consumed exact `ResponderID` binding.
    pub const fn responder_id_binding(
        &self,
    ) -> &BoundX509OcspResponderCertificate<
        'response,
        'response_encoded,
        'certificate,
        'certificate_encoded,
    > {
        &self.responder_id_binding
    }

    /// Return complete policy for the exact matched responder certificate.
    pub const fn responder_certificate_policy(
        &self,
    ) -> &ValidatedX509OcspResponderCertificatePolicy<'certificate, 'certificate_encoded> {
        &self.responder_certificate_policy
    }

    /// Return signature verification for the exact matched response and key.
    pub const fn response_signature(
        &self,
    ) -> &VerifiedX509OcspRsaSha256Signature<'response, 'response_encoded, 'certificate_encoded>
    {
        &self.response_signature
    }
}

impl fmt::Debug for AuthenticatedX509OcspRsaSha256Responder<'_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AuthenticatedX509OcspRsaSha256Responder")
    }
}

/// Authenticate one exact policy-approved delegated responder candidate and
/// response.
///
/// Evidence identity is checked before key parsing or cryptographic work. The
/// RSA key is then derived from the exact policy-retained certificate under
/// caller-owned strength and resource limits, and that key verifies the exact
/// response retained by the `ResponderID` binding.
pub fn authenticate_x509_ocsp_rsa_sha256_responder<
    'response,
    'response_encoded,
    'certificate,
    'certificate_encoded,
>(
    responder_id_binding: BoundX509OcspResponderCertificate<
        'response,
        'response_encoded,
        'certificate,
        'certificate_encoded,
    >,
    responder_certificate_policy: ValidatedX509OcspResponderCertificatePolicy<
        'certificate,
        'certificate_encoded,
    >,
    policy: X509OcspRsaSha256ResponderPolicy<'_>,
) -> Result<
    AuthenticatedX509OcspRsaSha256Responder<
        'response,
        'response_encoded,
        'certificate,
        'certificate_encoded,
    >,
    X509OcspAuthenticatedResponderError,
> {
    if !std::ptr::eq(
        responder_id_binding.certificate(),
        responder_certificate_policy.certificate(),
    ) {
        return Err(X509OcspAuthenticatedResponderError::new(
            X509OcspAuthenticatedResponderErrorKind::EvidenceMismatch,
        ));
    }

    let responder_public_key = validate_rsa_public_key(
        responder_certificate_policy
            .certificate()
            .tbs_certificate()
            .subject_public_key_info(),
        policy.responder_key_policy,
        policy.responder_key_limits,
    )
    .map_err(|error| {
        X509OcspAuthenticatedResponderError::new(
            X509OcspAuthenticatedResponderErrorKind::ResponderPublicKey(error.kind()),
        )
    })?;

    let response_signature = verify_x509_ocsp_rsa_pkcs1_v1_5_sha256_signature(
        responder_id_binding.response(),
        responder_public_key,
        policy.public_operation_limits,
    )
    .map_err(|error| {
        X509OcspAuthenticatedResponderError::new(
            X509OcspAuthenticatedResponderErrorKind::ResponseSignature(error.kind()),
        )
    })?;

    Ok(AuthenticatedX509OcspRsaSha256Responder {
        responder_id_binding,
        responder_certificate_policy,
        response_signature,
    })
}
