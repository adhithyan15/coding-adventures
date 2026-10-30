//! # Exact RSA PKCS #1 v1.5 SHA-512 OCSP signatures
//!
//! A `BasicOCSPResponse` signs the exact canonical DER encoding of its
//! `ResponseData`, not a reserialized structure or the outer response. This
//! crate composes the repository-owned primitives for that narrow check:
//!
//! ```text
//! preserved ResponseData DER ──SHA-512────────────────────┐
//!                                                         ├─ compare
//! signature ──bounded RSAVP1──PKCS #1 v1.5 validation─────┘
//! ```
//!
//! The caller supplies a separately policy-validated RSA public key and owns
//! every decision that established its identity. This crate requires the exact
//! `sha512WithRSAEncryption` identifier with present NULL parameters, rejects
//! non-byte-aligned signatures, and returns non-copyable evidence retaining
//! the exact response and key used by verification.
//!
//! Verification alone does not route algorithms, authorize the responder,
//! match `ResponderID`, bind `CertID` or status to a certificate or path,
//! establish freshness or a revocation decision, establish trust, or grant
//! TLS, HTTP, OAuth transport, or network authority.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::decode_null;
use rsa_pkcs1_v1_5_sha512::{validate_sha512_encoded_message, Sha512EncodedMessageErrorKind};
use rsa_public_operation::{
    recover_encoded_message, RsaPublicOperationErrorKind, RsaPublicOperationLimits,
};
use sha512::sum512;
use x509_ocsp_basic_response::X509OcspBasicResponse;
use x509_rsa_public_key::ValidatedRsaPublicKey;

/// PKCS #1 `sha512WithRSAEncryption` (`1.2.840.113549.1.1.13`).
pub const SHA512_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 13];

/// Stable failure categories that retain no response, key, or signature bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509OcspRsaSha512SignatureErrorKind {
    UnsupportedSignatureAlgorithm,
    MissingSignatureAlgorithmParameters,
    InvalidSignatureAlgorithmParameters,
    NonByteAlignedSignature,
    PublicOperation(RsaPublicOperationErrorKind),
    EncodedMessage(Sha512EncodedMessageErrorKind),
}

/// A redacted OCSP signature-verification failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509OcspRsaSha512SignatureError {
    kind: X509OcspRsaSha512SignatureErrorKind,
}

impl X509OcspRsaSha512SignatureError {
    const fn new(kind: X509OcspRsaSha512SignatureErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free failure category.
    pub const fn kind(self) -> X509OcspRsaSha512SignatureErrorKind {
        self.kind
    }
}

impl fmt::Display for X509OcspRsaSha512SignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 OCSP RSA SHA-512 signature error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509OcspRsaSha512SignatureError {}

/// Evidence that one exact public key verified one exact OCSP response.
///
/// The private fields and non-copyable result keep the verified response and
/// validated key attached to the success fact. Downstream composition can
/// consume those identities instead of accepting a detached Boolean alongside
/// caller-substitutable values. This is cryptographic evidence only; it is not
/// responder authorization, certificate or path validation, freshness,
/// revocation acceptance, or trust.
pub struct VerifiedX509OcspRsaSha512Signature<'response, 'encoded, 'key> {
    response: &'response X509OcspBasicResponse<'encoded>,
    responder_public_key: ValidatedRsaPublicKey<'key>,
}

impl<'response, 'encoded, 'key> VerifiedX509OcspRsaSha512Signature<'response, 'encoded, 'key> {
    /// Return the exact OCSP response whose signature was verified.
    pub const fn response(&self) -> &'response X509OcspBasicResponse<'encoded> {
        self.response
    }

    /// Return the exact separately validated RSA key used by verification.
    ///
    /// This accessor does not claim that the key is authorized for the
    /// response. That binding remains a separate caller-owned policy step.
    pub const fn responder_public_key(&self) -> ValidatedRsaPublicKey<'key> {
        self.responder_public_key
    }
}

impl fmt::Debug for VerifiedX509OcspRsaSha512Signature<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509OcspRsaSha512Signature")
    }
}

/// Verify one exact RSA PKCS #1 v1.5 SHA-512 `BasicOCSPResponse` signature.
///
/// The structural response decoder preserves the canonical signed
/// `ResponseData` encoding and generic signature fields. This boundary enables
/// only `sha512WithRSAEncryption` with canonical present NULL parameters, then
/// composes repository-owned SHA-512, bounded RSAVP1, and complete PKCS #1
/// v1.5 encoded-message validation. The supplied key must already satisfy
/// caller-owned RSA structure and strength policy.
pub fn verify_x509_ocsp_rsa_pkcs1_v1_5_sha512_signature<'response, 'encoded, 'key>(
    response: &'response X509OcspBasicResponse<'encoded>,
    responder_public_key: ValidatedRsaPublicKey<'key>,
    operation_limits: RsaPublicOperationLimits,
) -> Result<
    VerifiedX509OcspRsaSha512Signature<'response, 'encoded, 'key>,
    X509OcspRsaSha512SignatureError,
> {
    let signature_algorithm = response.signature_algorithm();
    if !signature_algorithm
        .algorithm()
        .equals(SHA512_WITH_RSA_ENCRYPTION_OID)
    {
        return Err(X509OcspRsaSha512SignatureError::new(
            X509OcspRsaSha512SignatureErrorKind::UnsupportedSignatureAlgorithm,
        ));
    }

    let parameters = signature_algorithm.parameters().ok_or_else(|| {
        X509OcspRsaSha512SignatureError::new(
            X509OcspRsaSha512SignatureErrorKind::MissingSignatureAlgorithmParameters,
        )
    })?;
    decode_null(parameters).map_err(|_| {
        X509OcspRsaSha512SignatureError::new(
            X509OcspRsaSha512SignatureErrorKind::InvalidSignatureAlgorithmParameters,
        )
    })?;

    let signature = response.signature();
    if signature.unused_bits() != 0 {
        return Err(X509OcspRsaSha512SignatureError::new(
            X509OcspRsaSha512SignatureErrorKind::NonByteAlignedSignature,
        ));
    }

    let encoded_message =
        recover_encoded_message(responder_public_key, signature.bytes(), operation_limits)
            .map_err(|error| {
                X509OcspRsaSha512SignatureError::new(
                    X509OcspRsaSha512SignatureErrorKind::PublicOperation(error.kind()),
                )
            })?;
    let digest = sum512(response.response_data().encoded());
    validate_sha512_encoded_message(&encoded_message, &digest).map_err(|error| {
        X509OcspRsaSha512SignatureError::new(X509OcspRsaSha512SignatureErrorKind::EncodedMessage(
            error.kind(),
        ))
    })?;

    Ok(VerifiedX509OcspRsaSha512Signature {
        response,
        responder_public_key,
    })
}
