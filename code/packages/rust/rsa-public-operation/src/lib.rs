//! # Bounded RSA public operation
//!
//! RFC 8017 signature verification first applies RSAVP1 to a signature
//! representative and then interprets the recovered modulus-width octets with
//! a scheme such as EMSA-PKCS1-v1_5. This crate owns only the first job.
//!
//! The key arrives as opaque evidence from `x509-rsa-public-key`, so raw ASN.1,
//! key-shape, and strength policy cannot be skipped here. A second caller-owned
//! byte ceiling is checked before arbitrary-precision conversion. The result is
//! restored to the modulus's exact width for a later encoded-message validator.
//!
//! Every operand is public verification data. This implementation therefore
//! does not claim constant-time behavior and must never be used for private RSA
//! exponentiation, signing, or decryption.

#![deny(unsafe_code)]

use bignum_core::BigInteger;
use core::fmt;
use x509_rsa_public_key::ValidatedRsaPublicKey;

/// Stable invalid-limit categories that retain no caller-provided values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPublicOperationLimitsErrorKind {
    ZeroMaximumModulusBytes,
}

/// A redacted operational-limit construction error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPublicOperationLimitsError {
    kind: RsaPublicOperationLimitsErrorKind,
}

impl RsaPublicOperationLimitsError {
    pub const fn kind(self) -> RsaPublicOperationLimitsErrorKind {
        self.kind
    }
}

impl fmt::Display for RsaPublicOperationLimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA public-operation limits error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for RsaPublicOperationLimitsError {}

/// One validated allocation ceiling for RSA public arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPublicOperationLimits {
    maximum_modulus_bytes: usize,
}

impl RsaPublicOperationLimits {
    pub const fn new(maximum_modulus_bytes: usize) -> Result<Self, RsaPublicOperationLimitsError> {
        if maximum_modulus_bytes == 0 {
            return Err(RsaPublicOperationLimitsError {
                kind: RsaPublicOperationLimitsErrorKind::ZeroMaximumModulusBytes,
            });
        }
        Ok(Self {
            maximum_modulus_bytes,
        })
    }

    pub const fn maximum_modulus_bytes(self) -> usize {
        self.maximum_modulus_bytes
    }
}

/// Stable operation-failure categories that retain no verification bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPublicOperationErrorKind {
    ModulusTooLong,
    SignatureLengthMismatch,
    SignatureRepresentativeOutOfRange,
}

/// A redacted RSA public-operation failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPublicOperationError {
    kind: RsaPublicOperationErrorKind,
}

impl RsaPublicOperationError {
    const fn new(kind: RsaPublicOperationErrorKind) -> Self {
        Self { kind }
    }

    pub const fn kind(self) -> RsaPublicOperationErrorKind {
        self.kind
    }
}

impl fmt::Display for RsaPublicOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA public-operation error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPublicOperationError {}

/// Recover one exact modulus-width encoded message with RFC 8017 RSAVP1.
///
/// The key is accepted only after `x509-rsa-public-key` has validated its
/// structure and caller-owned strength policy. The signature must be exactly
/// `k` octets for a `k`-octet modulus, and its integer representative must be
/// less than the modulus. Passing this arithmetic boundary alone does not prove
/// that the returned bytes contain any accepted signature encoding or digest.
pub fn recover_encoded_message(
    public_key: ValidatedRsaPublicKey<'_>,
    signature: &[u8],
    limits: RsaPublicOperationLimits,
) -> Result<Vec<u8>, RsaPublicOperationError> {
    let modulus_bytes = public_key.modulus();
    if modulus_bytes.len() > limits.maximum_modulus_bytes {
        return Err(RsaPublicOperationError::new(
            RsaPublicOperationErrorKind::ModulusTooLong,
        ));
    }
    if signature.len() != modulus_bytes.len() {
        return Err(RsaPublicOperationError::new(
            RsaPublicOperationErrorKind::SignatureLengthMismatch,
        ));
    }

    let modulus = unsigned_from_be_bytes(modulus_bytes);
    let signature_representative = unsigned_from_be_bytes(signature);
    if signature_representative >= modulus {
        return Err(RsaPublicOperationError::new(
            RsaPublicOperationErrorKind::SignatureRepresentativeOutOfRange,
        ));
    }

    let recovered = modular_power(
        signature_representative,
        public_key.public_exponent(),
        &modulus,
    );
    Ok(unsigned_to_fixed_be_bytes(&recovered, modulus_bytes.len()))
}

/// Convert public big-endian octets without asking the generic integer crate to
/// acquire a cryptography-specific byte API. Each step is the paper recurrence
/// `value = value * 256 + octet`, so allocations grow only with the input that
/// already passed the caller's byte ceiling.
fn unsigned_from_be_bytes(bytes: &[u8]) -> BigInteger {
    let radix = BigInteger::from_u64(256);
    bytes.iter().fold(BigInteger::zero(), |value, octet| {
        value
            .mul(&radix)
            .add(&BigInteger::from_u64(u64::from(*octet)))
    })
}

/// Square-and-multiply with reduction after every public multiplication.
///
/// The exponent is a validated public `u64`, so the loop has at most 64
/// iterations. Reducing the base once and each product immediately keeps the
/// multiplication inputs below the public modulus and intermediates near twice
/// its caller-bounded width.
fn modular_power(base: BigInteger, mut exponent: u64, modulus: &BigInteger) -> BigInteger {
    let mut result = BigInteger::one();
    let mut factor = base.rem(modulus);

    while exponent != 0 {
        if exponent & 1 == 1 {
            result = result.mul(&factor).rem(modulus);
        }
        exponent >>= 1;
        if exponent != 0 {
            factor = factor.mul(&factor).rem(modulus);
        }
    }
    result
}

/// Restore the minimal nonnegative integer representation to the exact RSA
/// modulus width. `BigInteger` already owns a reviewed radix formatter; walking
/// its lowercase hexadecimal digits from least significant to most significant
/// avoids a second arbitrary-precision division implementation here.
fn unsigned_to_fixed_be_bytes(value: &BigInteger, width: usize) -> Vec<u8> {
    let hexadecimal = value.to_str_radix(16);
    debug_assert!(hexadecimal.len() <= width.saturating_mul(2));

    let mut output = vec![0u8; width];
    for (digit_index, digit) in hexadecimal.bytes().rev().enumerate() {
        let nibble = match digit {
            b'0'..=b'9' => digit - b'0',
            b'a'..=b'f' => digit - b'a' + 10,
            _ => unreachable!("BigInteger emitted a non-hexadecimal digit"),
        };
        let output_index = width - 1 - digit_index / 2;
        output[output_index] |= nibble << ((digit_index % 2) * 4);
    }
    output
}
