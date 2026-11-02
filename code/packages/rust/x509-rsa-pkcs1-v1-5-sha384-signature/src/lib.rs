//! # Exact RSA PKCS #1 v1.5 SHA-384 certificate signatures
//!
//! This crate composes one deliberately narrow repository-owned verification
//! path. It accepts a complete decoded certificate and a separately validated
//! issuer RSA public key, requires the exact `sha384WithRSAEncryption`
//! identifier with present NULL parameters, hashes the certificate's preserved
//! `TBSCertificate` DER, applies RSAVP1, and checks the complete PKCS #1 v1.5
//! SHA-384 encoded message.
//!
//! The caller still owns issuer selection and linkage, algorithm routing, key
//! strength policy, certificate role and path validation, trust, time, names,
//! revocation, TLS, and network effects. Success proves only that this public
//! key verifies this certificate's exact signature over its exact signed bytes.

#![deny(unsafe_code)]

use core::fmt;
use der_asn1::decode_null;
use rsa_pkcs1_v1_5_sha384::{validate_sha384_encoded_message, Sha384EncodedMessageErrorKind};
use rsa_public_operation::{
    recover_encoded_message, RsaPublicOperationErrorKind, RsaPublicOperationLimits,
};
use sha384::sha384;
use x509_certificate::X509Certificate;
use x509_rsa_public_key::ValidatedRsaPublicKey;

/// `sha384WithRSAEncryption` (`1.2.840.113549.1.1.12`).
pub const SHA384_WITH_RSA_ENCRYPTION_OID: &[u64] = &[1, 2, 840, 113_549, 1, 1, 12];

/// Stable verification-failure categories that retain no certificate bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum X509RsaSha384SignatureErrorKind {
    UnsupportedSignatureAlgorithm,
    MissingSignatureAlgorithmParameters,
    InvalidSignatureAlgorithmParameters,
    NonByteAlignedSignature,
    PublicOperation(RsaPublicOperationErrorKind),
    EncodedMessage(Sha384EncodedMessageErrorKind),
}

/// A redacted certificate-signature verification failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct X509RsaSha384SignatureError {
    kind: X509RsaSha384SignatureErrorKind,
}

impl X509RsaSha384SignatureError {
    const fn new(kind: X509RsaSha384SignatureErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> X509RsaSha384SignatureErrorKind {
        self.kind
    }
}

impl fmt::Display for X509RsaSha384SignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "X.509 RSA SHA-384 signature error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for X509RsaSha384SignatureError {}

/// Borrowed evidence that one public key verified one certificate signature.
///
/// The evidence is deliberately non-copyable and retains the exact certificate
/// and validated issuer-key identities used by verification. It proves neither
/// issuer linkage nor certificate-path trust.
pub struct VerifiedX509RsaSha384Signature<'certificate, 'encoded, 'key> {
    certificate: &'certificate X509Certificate<'encoded>,
    issuer_public_key: ValidatedRsaPublicKey<'key>,
}

impl<'certificate, 'encoded, 'key> VerifiedX509RsaSha384Signature<'certificate, 'encoded, 'key> {
    /// Return the exact certificate whose signature was verified.
    #[must_use]
    pub const fn certificate(&self) -> &'certificate X509Certificate<'encoded> {
        self.certificate
    }

    /// Return the exact validated issuer key used for verification.
    #[must_use]
    pub const fn issuer_public_key(&self) -> ValidatedRsaPublicKey<'key> {
        self.issuer_public_key
    }
}

impl fmt::Debug for VerifiedX509RsaSha384Signature<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedX509RsaSha384Signature")
    }
}

/// Verify one exact RSA PKCS #1 v1.5 SHA-384 certificate signature.
///
/// The certificate decoder has already required exact inner/outer signature
/// AlgorithmIdentifier encoding agreement. This boundary additionally enables
/// only `sha384WithRSAEncryption` with canonical present NULL parameters. The
/// issuer key can be constructed only after separate RSA structure and caller
/// strength-policy validation.
pub fn verify_x509_rsa_pkcs1_v1_5_sha384_signature<'certificate, 'encoded, 'key>(
    certificate: &'certificate X509Certificate<'encoded>,
    issuer_public_key: ValidatedRsaPublicKey<'key>,
    operation_limits: RsaPublicOperationLimits,
) -> Result<VerifiedX509RsaSha384Signature<'certificate, 'encoded, 'key>, X509RsaSha384SignatureError>
{
    let signature_algorithm = certificate.signature_algorithm();
    if !signature_algorithm
        .algorithm()
        .equals(SHA384_WITH_RSA_ENCRYPTION_OID)
    {
        return Err(X509RsaSha384SignatureError::new(
            X509RsaSha384SignatureErrorKind::UnsupportedSignatureAlgorithm,
        ));
    }

    let parameters = signature_algorithm.parameters().ok_or_else(|| {
        X509RsaSha384SignatureError::new(
            X509RsaSha384SignatureErrorKind::MissingSignatureAlgorithmParameters,
        )
    })?;
    decode_null(parameters).map_err(|_| {
        X509RsaSha384SignatureError::new(
            X509RsaSha384SignatureErrorKind::InvalidSignatureAlgorithmParameters,
        )
    })?;

    let signature = certificate.signature_value();
    if signature.unused_bits() != 0 {
        return Err(X509RsaSha384SignatureError::new(
            X509RsaSha384SignatureErrorKind::NonByteAlignedSignature,
        ));
    }

    let encoded_message =
        recover_encoded_message(issuer_public_key, signature.bytes(), operation_limits).map_err(
            |error| {
                X509RsaSha384SignatureError::new(X509RsaSha384SignatureErrorKind::PublicOperation(
                    error.kind(),
                ))
            },
        )?;
    let digest = sha384(certificate.tbs_certificate().encoded());
    validate_sha384_encoded_message(&encoded_message, &digest).map_err(|error| {
        X509RsaSha384SignatureError::new(X509RsaSha384SignatureErrorKind::EncodedMessage(
            error.kind(),
        ))
    })?;

    Ok(VerifiedX509RsaSha384Signature {
        certificate,
        issuer_public_key,
    })
}
