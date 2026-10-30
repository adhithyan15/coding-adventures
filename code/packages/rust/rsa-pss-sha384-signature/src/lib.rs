//! # Bounded SHA-384 RSASSA-PSS signatures
//!
//! This crate composes the repository-owned RSA public operation with the
//! fixed SHA-384, MGF1-SHA-384, 48-byte-salt EMSA-PSS validator. It accepts a
//! previously validated RSA public key, one exact signature, an already
//! computed message digest, exact caller-owned scratch, and explicit ceilings.
//!
//! It does not hash messages, parse parameters or certificates, choose an
//! algorithm or issuer, establish trust, run TLS or HTTP, or perform I/O.

#![deny(unsafe_code)]

use core::fmt;
use rsa_pss_sha384::{
    validate_rsa_pss_sha384_encoded_message, RsaPssSha384ErrorKind, RsaPssSha384Limits,
    RsaPssSha384LimitsErrorKind, SHA384_DIGEST_LENGTH,
};
use rsa_public_operation::{
    recover_encoded_message, RsaPublicOperationErrorKind, RsaPublicOperationLimits,
    RsaPublicOperationLimitsErrorKind,
};
use x509_rsa_public_key::ValidatedRsaPublicKey;

/// Stable work-limit failures retaining no caller values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssSha384SignatureLimitsErrorKind {
    /// The RSA public-operation ceiling is invalid.
    PublicOperation(RsaPublicOperationLimitsErrorKind),
    /// The fixed-profile encoded-message ceiling is invalid.
    EncodedMessage(RsaPssSha384LimitsErrorKind),
}

/// A redacted signature-work-limit failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssSha384SignatureLimitsError {
    kind: RsaPssSha384SignatureLimitsErrorKind,
}

impl RsaPssSha384SignatureLimitsError {
    const fn new(kind: RsaPssSha384SignatureLimitsErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> RsaPssSha384SignatureLimitsErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssSha384SignatureLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssSha384SignatureLimitsError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssSha384SignatureLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA-PSS SHA-384 signature limits error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for RsaPssSha384SignatureLimitsError {}

/// Validated public work limits for RSA arithmetic and PSS validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssSha384SignatureLimits {
    public_operation: RsaPublicOperationLimits,
    encoded_message: RsaPssSha384Limits,
}

impl RsaPssSha384SignatureLimits {
    /// Validate independent RSA-modulus and encoded-message byte ceilings.
    pub const fn new(
        maximum_modulus_bytes: usize,
        maximum_encoded_message_bytes: usize,
    ) -> Result<Self, RsaPssSha384SignatureLimitsError> {
        let public_operation = match RsaPublicOperationLimits::new(maximum_modulus_bytes) {
            Ok(limits) => limits,
            Err(error) => {
                return Err(RsaPssSha384SignatureLimitsError::new(
                    RsaPssSha384SignatureLimitsErrorKind::PublicOperation(error.kind()),
                ))
            }
        };
        let encoded_message = match RsaPssSha384Limits::new(maximum_encoded_message_bytes) {
            Ok(limits) => limits,
            Err(error) => {
                return Err(RsaPssSha384SignatureLimitsError::new(
                    RsaPssSha384SignatureLimitsErrorKind::EncodedMessage(error.kind()),
                ))
            }
        };
        Ok(Self {
            public_operation,
            encoded_message,
        })
    }

    /// Return the caller-owned RSA modulus ceiling.
    #[must_use]
    pub const fn maximum_modulus_bytes(self) -> usize {
        self.public_operation.maximum_modulus_bytes()
    }

    /// Return the caller-owned encoded-message ceiling.
    #[must_use]
    pub const fn maximum_encoded_message_bytes(self) -> usize {
        self.encoded_message.maximum_encoded_message_bytes()
    }
}

/// Stable verification failures retaining no key, signature, or digest bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssSha384SignatureErrorKind {
    /// Bounded RSAVP1 failed.
    PublicOperation(RsaPublicOperationErrorKind),
    /// Modulus-width RSAVP1 output has a nonzero prefix outside `emLen`.
    NonzeroEncodedMessagePrefix,
    /// The complete recovered EMSA-PSS encoding is invalid.
    EncodedMessage(RsaPssSha384ErrorKind),
}

/// A redacted SHA-384 RSASSA-PSS verification failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssSha384SignatureError {
    kind: RsaPssSha384SignatureErrorKind,
}

impl RsaPssSha384SignatureError {
    const fn new(kind: RsaPssSha384SignatureErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> RsaPssSha384SignatureErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssSha384SignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssSha384SignatureError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssSha384SignatureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS SHA-384 signature error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssSha384SignatureError {}

/// Borrowed evidence binding the exact public verification inputs.
///
/// The type is deliberately neither `Copy` nor `Clone`. It binds the validated
/// public key, signature bytes, and already-computed digest used by the exact
/// RSAVP1 and EMSA-PSS operations. It proves no certificate, issuer, path,
/// trust, revocation, TLS, HTTP, OAuth-transport, or live-provider verdict.
pub struct VerifiedRsaPssSha384Signature<'key, 'signature, 'digest> {
    public_key: ValidatedRsaPublicKey<'key>,
    signature: &'signature [u8],
    message_digest: &'digest [u8; SHA384_DIGEST_LENGTH],
}

impl<'key, 'signature, 'digest> VerifiedRsaPssSha384Signature<'key, 'signature, 'digest> {
    /// Return the exact validated public key used for RSAVP1.
    #[must_use]
    pub const fn public_key(&self) -> ValidatedRsaPublicKey<'key> {
        self.public_key
    }

    /// Return the exact signature bytes verified by RSAVP1 and EMSA-PSS.
    #[must_use]
    pub const fn signature(&self) -> &'signature [u8] {
        self.signature
    }

    /// Return the exact already-computed message digest used by EMSA-PSS.
    #[must_use]
    pub const fn message_digest(&self) -> &'digest [u8; SHA384_DIGEST_LENGTH] {
        self.message_digest
    }
}

impl fmt::Debug for VerifiedRsaPssSha384Signature<'_, '_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("VerifiedRsaPssSha384Signature")
    }
}

/// Verify one exact fixed-profile SHA-384 RSASSA-PSS signature.
///
/// The validated public key supplies the exact public modulus bit length, so
/// this composition derives RFC 8017's `emBits = modBits - 1` instead of
/// assuming a byte-aligned modulus. RSAVP1 completes before the fixed-profile
/// encoded-message validator can mutate caller-owned DB scratch. All operands
/// are public verification data; the composed operations make no constant-time
/// guarantee and expose no private RSA operation.
pub fn verify_rsa_pss_sha384_signature<'key, 'signature, 'digest>(
    public_key: ValidatedRsaPublicKey<'key>,
    signature: &'signature [u8],
    expected_message_digest: &'digest [u8; SHA384_DIGEST_LENGTH],
    data_block_scratch: &mut [u8],
    limits: RsaPssSha384SignatureLimits,
) -> Result<VerifiedRsaPssSha384Signature<'key, 'signature, 'digest>, RsaPssSha384SignatureError> {
    let encoded_message = recover_encoded_message(public_key, signature, limits.public_operation)
        .map_err(|error| {
        RsaPssSha384SignatureError::new(RsaPssSha384SignatureErrorKind::PublicOperation(
            error.kind(),
        ))
    })?;
    let encoded_message_bits = public_key
        .modulus_bits()
        .checked_sub(1)
        .expect("validated RSA public key has a nonzero modulus bit length");
    let encoded_message_length =
        encoded_message_bits / 8 + usize::from(!encoded_message_bits.is_multiple_of(8));
    let encoded_message_prefix_length = encoded_message
        .len()
        .checked_sub(encoded_message_length)
        .expect("PSS encoded message cannot exceed the validated modulus width");
    if encoded_message[..encoded_message_prefix_length]
        .iter()
        .any(|octet| *octet != 0)
    {
        return Err(RsaPssSha384SignatureError::new(
            RsaPssSha384SignatureErrorKind::NonzeroEncodedMessagePrefix,
        ));
    }
    let encoded_message = &encoded_message[encoded_message_prefix_length..];
    validate_rsa_pss_sha384_encoded_message(
        encoded_message,
        encoded_message_bits,
        expected_message_digest,
        data_block_scratch,
        limits.encoded_message,
    )
    .map_err(|error| {
        RsaPssSha384SignatureError::new(RsaPssSha384SignatureErrorKind::EncodedMessage(
            error.kind(),
        ))
    })?;

    Ok(VerifiedRsaPssSha384Signature {
        public_key,
        signature,
        message_digest: expected_message_digest,
    })
}
