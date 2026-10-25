//! # Exact OCSP `CertID` certificate binding
//!
//! This crate binds one exact decoded OCSP `SingleResponse` to one exact
//! target certificate and one exact candidate issuer through caller-owned,
//! data-driven hash policy. It deliberately grants no issuer-link,
//! responder-authorization, signature-verification, path, trust, freshness,
//! revocation, TLS, HTTP, OAuth-transport, or network authority.

#![deny(unsafe_code)]

use core::fmt;
use ct_compare::{ct_eq, ct_eq_fixed};
use x509_algorithm_identifier::X509AlgorithmIdentifier;
use x509_certificate::X509Certificate;
use x509_ocsp_single_response::X509OcspSingleResponse;

/// The exact issuer-certificate input requested from caller-owned hash policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspCertIdHashInput<'input> {
    /// Canonical DER encoding of the issuer certificate's subject Name,
    /// including the outer SEQUENCE tag and length.
    IssuerNameDer(&'input [u8]),
    /// Issuer certificate `subjectPublicKey` BIT STRING contents, excluding the
    /// DER tag, length, and unused-bits count octet.
    IssuerSubjectPublicKey(&'input [u8]),
}

impl<'input> X509OcspCertIdHashInput<'input> {
    /// Return the exact public bytes to hash for this identifier component.
    pub const fn bytes(self) -> &'input [u8] {
        match self {
            Self::IssuerNameDer(bytes) | Self::IssuerSubjectPublicKey(bytes) => bytes,
        }
    }
}

/// Stable failure categories retaining no response or certificate payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspCertIdBindingErrorKind {
    InvalidHashLength,
    SerialNumberMismatch,
    IssuerNameHashPolicyRejected,
    IssuerNameHashMismatch,
    IssuerKeyHashPolicyRejected,
    IssuerKeyHashMismatch,
}

/// A redacted OCSP certificate-identifier binding failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspCertIdBindingError {
    kind: X509OcspCertIdBindingErrorKind,
}

impl X509OcspCertIdBindingError {
    const fn new(kind: X509OcspCertIdBindingErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspCertIdBindingErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspCertIdBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "X.509 OCSP CertID binding error {:?}", self.kind)
    }
}

impl std::error::Error for X509OcspCertIdBindingError {}

/// Non-copyable evidence retaining the exact response, target, and issuer.
///
/// This value proves only that the response `CertID` matched these certificate
/// objects under the caller-selected hash policy. It is not issuer-link
/// authentication, OCSP response verification, or responder authorization.
pub struct BoundX509OcspCertId<
    'response,
    'response_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
> {
    response: &'response X509OcspSingleResponse<'response_encoded>,
    target: &'target X509Certificate<'target_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
}

impl<'response, 'response_encoded, 'target, 'target_encoded, 'issuer, 'issuer_encoded>
    BoundX509OcspCertId<
        'response,
        'response_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >
{
    /// Return the exact response whose `CertID` matched.
    pub const fn response(&self) -> &'response X509OcspSingleResponse<'response_encoded> {
        self.response
    }

    /// Return the exact target certificate whose serial matched.
    pub const fn target(&self) -> &'target X509Certificate<'target_encoded> {
        self.target
    }

    /// Return the exact issuer certificate whose identity hashes matched.
    pub const fn issuer(&self) -> &'issuer X509Certificate<'issuer_encoded> {
        self.issuer
    }
}

impl fmt::Debug for BoundX509OcspCertId<'_, '_, '_, '_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("BoundX509OcspCertId")
    }
}

/// Bind one exact OCSP `SingleResponse` to exact target and issuer certificates.
///
/// Failure order is stable: public fixed hash lengths, target serial, issuer
/// Name hash policy and comparison, then issuer key hash policy and comparison.
/// `hash_policy` is invoked at most twice, with the Name input first and key
/// input second. Both invocations receive the exact `CertID` algorithm.
///
/// Success does not authorize the algorithm, prove that the issuer signed the
/// target, verify the OCSP response, or authorize its responder. Those remain
/// separate policy and cryptographic composition boundaries.
pub fn bind_x509_ocsp_cert_id_to_certificates<
    'response,
    'response_encoded,
    'target,
    'target_encoded,
    'issuer,
    'issuer_encoded,
    const HASH_LENGTH: usize,
    HashPolicy,
>(
    response: &'response X509OcspSingleResponse<'response_encoded>,
    target: &'target X509Certificate<'target_encoded>,
    issuer: &'issuer X509Certificate<'issuer_encoded>,
    mut hash_policy: HashPolicy,
) -> Result<
    BoundX509OcspCertId<
        'response,
        'response_encoded,
        'target,
        'target_encoded,
        'issuer,
        'issuer_encoded,
    >,
    X509OcspCertIdBindingError,
>
where
    HashPolicy: for<'input> FnMut(
        X509AlgorithmIdentifier<'response_encoded>,
        X509OcspCertIdHashInput<'input>,
    ) -> Option<[u8; HASH_LENGTH]>,
{
    let cert_id = response.cert_id();
    if HASH_LENGTH == 0
        || cert_id.issuer_name_hash().len() != HASH_LENGTH
        || cert_id.issuer_key_hash().len() != HASH_LENGTH
    {
        return Err(X509OcspCertIdBindingError::new(
            X509OcspCertIdBindingErrorKind::InvalidHashLength,
        ));
    }

    if !ct_eq(
        cert_id.serial_number().encoded_value(),
        target.tbs_certificate().serial_number().encoded_value(),
    ) {
        return Err(X509OcspCertIdBindingError::new(
            X509OcspCertIdBindingErrorKind::SerialNumberMismatch,
        ));
    }

    let expected_name_hash: &[u8; HASH_LENGTH] = cert_id
        .issuer_name_hash()
        .try_into()
        .expect("length was checked above");
    let computed_name_hash = hash_policy(
        cert_id.hash_algorithm(),
        X509OcspCertIdHashInput::IssuerNameDer(issuer.tbs_certificate().subject().encoded()),
    )
    .ok_or_else(|| {
        X509OcspCertIdBindingError::new(
            X509OcspCertIdBindingErrorKind::IssuerNameHashPolicyRejected,
        )
    })?;
    if !ct_eq_fixed(&computed_name_hash, expected_name_hash) {
        return Err(X509OcspCertIdBindingError::new(
            X509OcspCertIdBindingErrorKind::IssuerNameHashMismatch,
        ));
    }

    let expected_key_hash: &[u8; HASH_LENGTH] = cert_id
        .issuer_key_hash()
        .try_into()
        .expect("length was checked above");
    let issuer_subject_public_key = issuer
        .tbs_certificate()
        .subject_public_key_info()
        .subject_public_key();
    let computed_key_hash = hash_policy(
        cert_id.hash_algorithm(),
        X509OcspCertIdHashInput::IssuerSubjectPublicKey(issuer_subject_public_key.bytes()),
    )
    .ok_or_else(|| {
        X509OcspCertIdBindingError::new(X509OcspCertIdBindingErrorKind::IssuerKeyHashPolicyRejected)
    })?;
    if !ct_eq_fixed(&computed_key_hash, expected_key_hash) {
        return Err(X509OcspCertIdBindingError::new(
            X509OcspCertIdBindingErrorKind::IssuerKeyHashMismatch,
        ));
    }

    Ok(BoundX509OcspCertId {
        response,
        target,
        issuer,
    })
}
