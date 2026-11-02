//! # Bounded SHA-256 MGF1 for RSA-PSS verification
//!
//! RSA-PSS encoded-message verification uses MGF1 to expand the 32-byte hash
//! value `H` into a mask for the data block. RFC 8017 defines each block as:
//!
//! ```text
//! SHA-256(seed || I2OSP(counter, 4))
//! ```
//!
//! This crate owns only that deterministic expansion. It fixes the seed and
//! digest widths to SHA-256, requires a caller-selected work ceiling, validates
//! every length before changing caller-owned output, and allocates no memory.

#![deny(unsafe_code)]

use core::fmt;

/// SHA-256 produces exactly 32 octets.
pub const SHA256_DIGEST_LENGTH: usize = 32;

/// RFC 8017 encodes every MGF1 counter as four network-order octets.
pub const MGF1_COUNTER_LENGTH: usize = 4;

/// The fixed stack input is one SHA-256 seed followed by one MGF1 counter.
const MGF1_HASH_INPUT_LENGTH: usize = SHA256_DIGEST_LENGTH + MGF1_COUNTER_LENGTH;

/// RFC 8017 limits MGF1 output to `2^32 * hLen` octets.
pub const MAXIMUM_MGF1_MASK_LENGTH: u64 = (u32::MAX as u64 + 1) * 32;

/// Stable failures while constructing a caller-owned MGF1 work policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mgf1Sha256LimitsErrorKind {
    /// A zero ceiling admits no useful RSA-PSS mask operation.
    ZeroMaximumMaskBytes,
    /// The ceiling exceeds the maximum representable by RFC 8017 MGF1.
    MaximumMaskTooLong,
}

/// A redacted MGF1 work-policy construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Mgf1Sha256LimitsError {
    kind: Mgf1Sha256LimitsErrorKind,
}

impl Mgf1Sha256LimitsError {
    /// Return the stable redacted failure category.
    pub const fn kind(self) -> Mgf1Sha256LimitsErrorKind {
        self.kind
    }

    const fn new(kind: Mgf1Sha256LimitsErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Debug for Mgf1Sha256LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mgf1Sha256LimitsError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Mgf1Sha256LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SHA-256 MGF1 limits error {:?}", self.kind)
    }
}

impl std::error::Error for Mgf1Sha256LimitsError {}

/// A validated caller-selected work ceiling for one mask operation.
///
/// The ceiling is a denial-of-service boundary, not an RSA modulus or
/// certificate policy. Later composition must select it from independently
/// validated key and protocol constraints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mgf1Sha256Limits {
    maximum_mask_bytes: usize,
}

impl Mgf1Sha256Limits {
    /// Validate a nonzero ceiling that also fits the RFC 8017 MGF1 domain.
    pub const fn new(maximum_mask_bytes: usize) -> Result<Self, Mgf1Sha256LimitsError> {
        if maximum_mask_bytes == 0 {
            return Err(Mgf1Sha256LimitsError::new(
                Mgf1Sha256LimitsErrorKind::ZeroMaximumMaskBytes,
            ));
        }
        if (maximum_mask_bytes as u128) > (MAXIMUM_MGF1_MASK_LENGTH as u128) {
            return Err(Mgf1Sha256LimitsError::new(
                Mgf1Sha256LimitsErrorKind::MaximumMaskTooLong,
            ));
        }
        Ok(Self { maximum_mask_bytes })
    }

    /// Return the maximum caller-authorized output length.
    pub const fn maximum_mask_bytes(self) -> usize {
        self.maximum_mask_bytes
    }
}

/// Stable failures while expanding one SHA-256 MGF1 mask.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mgf1Sha256ErrorKind {
    /// The caller-owned output exceeds the validated work ceiling.
    MaskTooLongForPolicy,
}

/// A redacted SHA-256 MGF1 expansion failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Mgf1Sha256Error {
    kind: Mgf1Sha256ErrorKind,
}

impl Mgf1Sha256Error {
    /// Return the stable redacted failure category.
    pub const fn kind(self) -> Mgf1Sha256ErrorKind {
        self.kind
    }

    const fn new(kind: Mgf1Sha256ErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Debug for Mgf1Sha256Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mgf1Sha256Error")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Mgf1Sha256Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SHA-256 MGF1 error {:?}", self.kind)
    }
}

impl std::error::Error for Mgf1Sha256Error {}

/// Expand one exact 32-byte RSA-PSS hash value into caller-owned mask bytes.
///
/// The counter begins at zero and is encoded in four-byte network order. The
/// output length determines the exact number of SHA-256 invocations. Empty
/// output is valid, performs no hashing, and remains subject to a nonzero work
/// policy. A policy failure occurs before any output byte is changed.
///
/// This function does not validate an EMSA-PSS encoded message, select salt or
/// algorithm parameters, perform RSA arithmetic, or verify a signature.
pub fn generate_mgf1_sha256_mask(
    seed: &[u8; SHA256_DIGEST_LENGTH],
    output: &mut [u8],
    limits: Mgf1Sha256Limits,
) -> Result<(), Mgf1Sha256Error> {
    if output.len() > limits.maximum_mask_bytes {
        return Err(Mgf1Sha256Error::new(
            Mgf1Sha256ErrorKind::MaskTooLongForPolicy,
        ));
    }

    // One fixed stack buffer replaces the conceptual unbounded concatenation
    // in RFC 8017. The seed is public verification data. Only the four-octet
    // counter changes between blocks.
    let mut hash_input = [0u8; MGF1_HASH_INPUT_LENGTH];
    hash_input[..SHA256_DIGEST_LENGTH].copy_from_slice(seed);

    for (counter, output_block) in output.chunks_mut(SHA256_DIGEST_LENGTH).enumerate() {
        // The limits constructor proves there can be at most 2^32 chunks, so
        // every zero-based chunk index fits exactly in the RFC counter.
        hash_input[SHA256_DIGEST_LENGTH..].copy_from_slice(&(counter as u32).to_be_bytes());
        let digest = coding_adventures_sha256::sha256(&hash_input);
        output_block.copy_from_slice(&digest[..output_block.len()]);
    }

    Ok(())
}
