//! # Bounded SHA-384 MGF1 for RSA-PSS verification
//!
//! RSA-PSS encoded-message verification uses MGF1 to expand the 48-byte hash
//! value `H` into a mask for the data block. RFC 8017 defines each block as
//! `SHA-384(seed || I2OSP(counter, 4))`. This crate owns only that deterministic
//! expansion, with a caller-selected work ceiling and caller-owned output.

#![deny(unsafe_code)]

use core::fmt;

/// SHA-384 produces exactly 48 octets.
pub const SHA384_DIGEST_LENGTH: usize = 48;
/// RFC 8017 encodes every MGF1 counter as four network-order octets.
pub const MGF1_COUNTER_LENGTH: usize = 4;
const MGF1_HASH_INPUT_LENGTH: usize = SHA384_DIGEST_LENGTH + MGF1_COUNTER_LENGTH;
/// RFC 8017 limits MGF1 output to `2^32 * hLen` octets.
pub const MAXIMUM_MGF1_MASK_LENGTH: u64 = (u32::MAX as u64 + 1) * 48;

/// Stable failures while constructing a caller-owned MGF1 work policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mgf1Sha384LimitsErrorKind {
    /// A zero ceiling admits no useful RSA-PSS mask operation.
    ZeroMaximumMaskBytes,
    /// The ceiling exceeds the maximum representable by RFC 8017 MGF1.
    MaximumMaskTooLong,
}

/// A redacted MGF1 work-policy construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Mgf1Sha384LimitsError {
    kind: Mgf1Sha384LimitsErrorKind,
}

impl Mgf1Sha384LimitsError {
    /// Return the stable redacted failure category.
    pub const fn kind(self) -> Mgf1Sha384LimitsErrorKind {
        self.kind
    }

    const fn new(kind: Mgf1Sha384LimitsErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Debug for Mgf1Sha384LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mgf1Sha384LimitsError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Mgf1Sha384LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SHA-384 MGF1 limits error {:?}", self.kind)
    }
}

impl std::error::Error for Mgf1Sha384LimitsError {}

/// A validated caller-selected work ceiling for one mask operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Mgf1Sha384Limits {
    maximum_mask_bytes: usize,
}

impl Mgf1Sha384Limits {
    /// Validate a nonzero ceiling that also fits the RFC 8017 MGF1 domain.
    pub const fn new(maximum_mask_bytes: usize) -> Result<Self, Mgf1Sha384LimitsError> {
        if maximum_mask_bytes == 0 {
            return Err(Mgf1Sha384LimitsError::new(
                Mgf1Sha384LimitsErrorKind::ZeroMaximumMaskBytes,
            ));
        }
        if (maximum_mask_bytes as u128) > (MAXIMUM_MGF1_MASK_LENGTH as u128) {
            return Err(Mgf1Sha384LimitsError::new(
                Mgf1Sha384LimitsErrorKind::MaximumMaskTooLong,
            ));
        }
        Ok(Self { maximum_mask_bytes })
    }

    /// Return the maximum caller-authorized output length.
    pub const fn maximum_mask_bytes(self) -> usize {
        self.maximum_mask_bytes
    }
}

/// Stable failures while expanding one SHA-384 MGF1 mask.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mgf1Sha384ErrorKind {
    /// The caller-owned output exceeds the validated work ceiling.
    MaskTooLongForPolicy,
}

/// A redacted SHA-384 MGF1 expansion failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Mgf1Sha384Error {
    kind: Mgf1Sha384ErrorKind,
}

impl Mgf1Sha384Error {
    /// Return the stable redacted failure category.
    pub const fn kind(self) -> Mgf1Sha384ErrorKind {
        self.kind
    }

    const fn new(kind: Mgf1Sha384ErrorKind) -> Self {
        Self { kind }
    }
}

impl fmt::Debug for Mgf1Sha384Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Mgf1Sha384Error")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for Mgf1Sha384Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "SHA-384 MGF1 error {:?}", self.kind)
    }
}

impl std::error::Error for Mgf1Sha384Error {}

/// Expand one exact 48-byte RSA-PSS hash value into caller-owned mask bytes.
///
/// The counter begins at zero and is encoded in four-byte network order. Empty
/// output is valid and performs no hashing. A policy failure occurs before any
/// output byte is changed. This function performs no RSA arithmetic or
/// signature verification.
pub fn generate_mgf1_sha384_mask(
    seed: &[u8; SHA384_DIGEST_LENGTH],
    output: &mut [u8],
    limits: Mgf1Sha384Limits,
) -> Result<(), Mgf1Sha384Error> {
    if output.len() > limits.maximum_mask_bytes {
        return Err(Mgf1Sha384Error::new(
            Mgf1Sha384ErrorKind::MaskTooLongForPolicy,
        ));
    }

    let mut hash_input = [0u8; MGF1_HASH_INPUT_LENGTH];
    hash_input[..SHA384_DIGEST_LENGTH].copy_from_slice(seed);

    for (counter, output_block) in output.chunks_mut(SHA384_DIGEST_LENGTH).enumerate() {
        hash_input[SHA384_DIGEST_LENGTH..].copy_from_slice(&(counter as u32).to_be_bytes());
        let digest = coding_adventures_sha384::sha384(&hash_input);
        output_block.copy_from_slice(&digest[..output_block.len()]);
    }

    Ok(())
}
