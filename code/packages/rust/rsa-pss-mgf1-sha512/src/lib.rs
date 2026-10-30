//! # Bounded SHA-512 MGF1 for RSA-PSS verification
//!
//! RSA-PSS encoded-message verification uses MGF1 to expand the 64-byte hash
//! value `H` into a mask for the data block. RFC 8017 defines each block as
//! `SHA-512(seed || I2OSP(counter, 4))`. This crate owns only that deterministic
//! expansion, with a caller-selected work ceiling and caller-owned output.

#![deny(unsafe_code)]

use core::fmt;

/// SHA-512 produces exactly 64 octets.
pub const SHA512_DIGEST_LENGTH: usize = 64;
/// RFC 8017 encodes every MGF1 counter as four network-order octets.
pub const MGF1_COUNTER_LENGTH: usize = 4;
const MGF1_HASH_INPUT_LENGTH: usize = SHA512_DIGEST_LENGTH + MGF1_COUNTER_LENGTH;
/// RFC 8017 limits MGF1 output to `2^32 * hLen` octets.
pub const MAXIMUM_MGF1_MASK_LENGTH: u64 = (u32::MAX as u64 + 1) * 64;

/// Stable failures while constructing a caller-owned MGF1 work policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mgf1Sha512LimitsErrorKind {
    /// A zero ceiling admits no useful RSA-PSS mask operation.
    ZeroMaximumMaskBytes,
    /// The ceiling exceeds the maximum representable by RFC 8017 MGF1.
    MaximumMaskTooLong,
}

/// A redacted MGF1 work-policy construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Mgf1Sha512LimitsError {
    kind: Mgf1Sha512LimitsErrorKind,
}

impl Mgf1Sha512LimitsError {
    /// Return the stable redacted failure category.
    pub const fn kind(self) -> Mgf1Sha512LimitsErrorKind {
        self.kind
    }

    const fn new(kind: Mgf1Sha512LimitsErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Debug for Mgf1Sha512LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mgf1Sha512LimitsError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Mgf1Sha512LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SHA-512 MGF1 limits error {:?}", self.kind)
    }
}

impl std::error::Error for Mgf1Sha512LimitsError {}

/// A validated caller-selected work ceiling for one mask operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mgf1Sha512Limits {
    maximum_mask_bytes: usize,
}

impl Mgf1Sha512Limits {
    /// Validate a nonzero ceiling that also fits the RFC 8017 MGF1 domain.
    pub const fn new(maximum_mask_bytes: usize) -> Result<Self, Mgf1Sha512LimitsError> {
        if maximum_mask_bytes == 0 {
            return Err(Mgf1Sha512LimitsError::new(
                Mgf1Sha512LimitsErrorKind::ZeroMaximumMaskBytes,
            ));
        }
        if (maximum_mask_bytes as u128) > (MAXIMUM_MGF1_MASK_LENGTH as u128) {
            return Err(Mgf1Sha512LimitsError::new(
                Mgf1Sha512LimitsErrorKind::MaximumMaskTooLong,
            ));
        }
        Ok(Self { maximum_mask_bytes })
    }

    /// Return the maximum caller-authorized output length.
    pub const fn maximum_mask_bytes(self) -> usize {
        self.maximum_mask_bytes
    }
}

/// Stable failures while expanding one SHA-512 MGF1 mask.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mgf1Sha512ErrorKind {
    /// The caller-owned output exceeds the validated work ceiling.
    MaskTooLongForPolicy,
}

/// A redacted SHA-512 MGF1 expansion failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Mgf1Sha512Error {
    kind: Mgf1Sha512ErrorKind,
}

impl Mgf1Sha512Error {
    /// Return the stable redacted failure category.
    pub const fn kind(self) -> Mgf1Sha512ErrorKind {
        self.kind
    }

    const fn new(kind: Mgf1Sha512ErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Debug for Mgf1Sha512Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mgf1Sha512Error")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Mgf1Sha512Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SHA-512 MGF1 error {:?}", self.kind)
    }
}

impl std::error::Error for Mgf1Sha512Error {}

/// Expand one exact 64-byte RSA-PSS hash value into caller-owned mask bytes.
///
/// The counter begins at zero and is encoded in four-byte network order. Empty
/// output is valid and performs no hashing. A policy failure occurs before any
/// output byte is changed. This function performs no RSA arithmetic or
/// signature verification.
pub fn generate_mgf1_sha512_mask(
    seed: &[u8; SHA512_DIGEST_LENGTH],
    output: &mut [u8],
    limits: Mgf1Sha512Limits,
) -> Result<(), Mgf1Sha512Error> {
    if output.len() > limits.maximum_mask_bytes {
        return Err(Mgf1Sha512Error::new(
            Mgf1Sha512ErrorKind::MaskTooLongForPolicy,
        ));
    }

    let mut hash_input = [0u8; MGF1_HASH_INPUT_LENGTH];
    hash_input[..SHA512_DIGEST_LENGTH].copy_from_slice(seed);

    for (counter, output_block) in output.chunks_mut(SHA512_DIGEST_LENGTH).enumerate() {
        hash_input[SHA512_DIGEST_LENGTH..].copy_from_slice(&(counter as u32).to_be_bytes());
        let digest = coding_adventures_sha512::sum512(&hash_input);
        output_block.copy_from_slice(&digest[..output_block.len()]);
    }

    Ok(())
}
