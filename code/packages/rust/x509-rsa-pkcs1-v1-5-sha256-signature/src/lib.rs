//! # Exact RSA PKCS #1 v1.5 SHA-256 certificate signatures
//!
//! This crate composes one deliberately narrow repository-owned verification
//! path. It accepts a complete decoded certificate and a separately validated
//! issuer RSA public key, requires the exact RFC 3279
//! `sha256WithRSAEncryption` identifier with present NULL parameters, hashes
//! the certificate's preserved `TBSCertificate` DER, applies RSAVP1, and checks
//! the complete PKCS #1 v1.5 SHA-256 encoded message.
//!
//! The caller still owns issuer selection and linkage, algorithm routing, key
//! strength policy, certificate role and path validation, trust, time, names,
//! revocation, TLS, and network effects. Success proves only that this public
//! key verifies this certificate's exact signature over its exact signed bytes.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::decode_null;
use rsa_pkcs1_v1_5_sha256::{validate_sha256_encoded_message, Sha256EncodedMessageErrorKind};
use rsa_public_operation::{
    recover_encoded_message, RsaPublicOperationErrorKind, RsaPublicOperationLimits,
};
use sha256::sha256;
use x509_certificate::X509Certificate;
use x509_rsa_public_key::ValidatedRsaPublicKey;

/// RFC 5754 `sha256WithRSAEncryption` (`1.2.840.113549.1.1.11`).
pub const SHA256_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 11];

/// Stable verification-failure categories that retain no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha256SignatureErrorKind {
    UnsupportedSignatureAlgorithm,
    MissingSignatureAlgorithmParameters,
    InvalidSignatureAlgorithmParameters,
    NonByteAlignedSignature,
    PublicOperation(RsaPublicOperationErrorKind),
    EncodedMessage(Sha256EncodedMessageErrorKind),
}

/// A redacted certificate-signature verification failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha256SignatureError {
    kind: X509RsaSha256SignatureErrorKind,
}

impl X509RsaSha256SignatureError {
    const fn new(kind: X509RsaSha256SignatureErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> X509RsaSha256SignatureErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha256SignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-256 signature error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha256SignatureError {}

/// Borrowed evidence that one public key verified one certificate signature.
///
/// The evidence is deliberately non-copyable and retains the exact certificate
/// and validated issuer-key identities used by verification. Downstream
/// composition can therefore consume those bound values instead of accepting
/// an unrelated certificate or key alongside detached success evidence. It is
/// not evidence that the key belongs to a trusted issuer or that any other
/// certificate policy gate passed.
pub struct VerifiedX509RsaSha256Signature<'certificate, 'encoded, 'key> {
    certificate: &'certificate X509Certificate<'encoded>,
    issuer_public_key: ValidatedRsaPublicKey<'key>,
}

impl<'certificate, 'encoded, 'key> VerifiedX509RsaSha256Signature<'certificate, 'encoded, 'key> {
    /// Return the exact certificate whose signature was verified.
    pub const fn certificate(&self) -> &'certificate X509Certificate<'encoded> {
        self.certificate
    }

    /// Return the exact validated issuer key used for verification.
    pub const fn issuer_public_key(&self) -> ValidatedRsaPublicKey<'key> {
        self.issuer_public_key
    }
}

impl fmt::Debug for VerifiedX509RsaSha256Signature<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha256Signature")
    }
}

/// Verify one exact RSA PKCS #1 v1.5 SHA-256 certificate signature.
///
/// The certificate decoder has already required exact inner/outer signature
/// AlgorithmIdentifier encoding agreement. This boundary additionally enables
/// only `sha256WithRSAEncryption` with canonical present NULL parameters. The
/// issuer key can be constructed only after separate RSA structure and caller
/// strength-policy validation.
pub fn verify_x509_rsa_pkcs1_v1_5_sha256_signature<'certificate, 'encoded, 'key>(
    certificate: &'certificate X509Certificate<'encoded>,
    issuer_public_key: ValidatedRsaPublicKey<'key>,
    operation_limits: RsaPublicOperationLimits,
) -> Result<VerifiedX509RsaSha256Signature<'certificate, 'encoded, 'key>, X509RsaSha256SignatureError>
{
    let signature_algorithm = certificate.signature_algorithm();
    if !signature_algorithm
        .algorithm()
        .equals(SHA256_WITH_RSA_ENCRYPTION_OID)
    {
        return Err(X509RsaSha256SignatureError::new(
            X509RsaSha256SignatureErrorKind::UnsupportedSignatureAlgorithm,
        ));
    }

    let parameters = signature_algorithm.parameters().ok_or_else(|| {
        X509RsaSha256SignatureError::new(
            X509RsaSha256SignatureErrorKind::MissingSignatureAlgorithmParameters,
        )
    })?;
    decode_null(parameters).map_err(|_| {
        X509RsaSha256SignatureError::new(
            X509RsaSha256SignatureErrorKind::InvalidSignatureAlgorithmParameters,
        )
    })?;

    let signature = certificate.signature_value();
    if signature.unused_bits() != 0 {
        return Err(X509RsaSha256SignatureError::new(
            X509RsaSha256SignatureErrorKind::NonByteAlignedSignature,
        ));
    }

    let encoded_message =
        recover_encoded_message(issuer_public_key, signature.bytes(), operation_limits).map_err(
            |error| {
                X509RsaSha256SignatureError::new(X509RsaSha256SignatureErrorKind::PublicOperation(
                    error.kind(),
                ))
            },
        )?;
    let digest = sha256(certificate.tbs_certificate().encoded());
    validate_sha256_encoded_message(&encoded_message, &digest).map_err(|error| {
        X509RsaSha256SignatureError::new(X509RsaSha256SignatureErrorKind::EncodedMessage(
            error.kind(),
        ))
    })?;

    Ok(VerifiedX509RsaSha256Signature {
        certificate,
        issuer_public_key,
    })
}
