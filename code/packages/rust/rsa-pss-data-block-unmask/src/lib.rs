//! # Bounded RSA-PSS data-block unmasking
//!
//! RFC 8017 EMSA-PSS verification derives `DB` by XORing the complete
//! `maskedDB` field with an equal-length MGF1 mask, then clearing the unused
//! high bits required by `emBits`. This crate performs only that public-byte
//! transformation in caller-owned memory under a caller-owned byte ceiling.
//!
//! It does not run MGF1, validate the resulting data-block structure, hash a
//! message, compare `H`, perform RSA arithmetic, or establish a signature
//! verdict.

#![deny(unsafe_code)]

use core::fmt;

/// Stable invalid-policy categories that retain no caller-supplied values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssDataBlockUnmaskPolicyErrorKind {
    /// A zero ceiling admits no data block.
    ZeroMaximumDataBlockBytes,
}

/// A redacted data-block-unmask policy-construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssDataBlockUnmaskPolicyError {
    kind: RsaPssDataBlockUnmaskPolicyErrorKind,
}

impl RsaPssDataBlockUnmaskPolicyError {
    const fn new(kind: RsaPssDataBlockUnmaskPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssDataBlockUnmaskPolicyErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssDataBlockUnmaskPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssDataBlockUnmaskPolicyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssDataBlockUnmaskPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA-PSS data-block-unmask policy error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for RsaPssDataBlockUnmaskPolicyError {}

/// Validated public work policy for one RSA-PSS data-block unmask operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssDataBlockUnmaskPolicy {
    maximum_data_block_bytes: usize,
}

impl RsaPssDataBlockUnmaskPolicy {
    /// Validate a public data-block byte ceiling.
    pub const fn new(
        maximum_data_block_bytes: usize,
    ) -> Result<Self, RsaPssDataBlockUnmaskPolicyError> {
        if maximum_data_block_bytes == 0 {
            return Err(RsaPssDataBlockUnmaskPolicyError::new(
                RsaPssDataBlockUnmaskPolicyErrorKind::ZeroMaximumDataBlockBytes,
            ));
        }
        Ok(Self {
            maximum_data_block_bytes,
        })
    }

    /// Return the caller-owned data-block work ceiling.
    pub const fn maximum_data_block_bytes(self) -> usize {
        self.maximum_data_block_bytes
    }
}

/// Stable unmask failures that retain no caller bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssDataBlockUnmaskErrorKind {
    /// The complete `maskedDB` input is empty.
    EmptyMaskedDataBlock,
    /// The input exceeds the caller-owned work ceiling.
    MaskedDataBlockTooLongForPolicy,
    /// The MGF1 mask length differs from the complete `maskedDB` length.
    DataBlockMaskLengthMismatch,
    /// The caller-owned output length differs from the complete input length.
    OutputLengthMismatch,
    /// RFC 8017 permits between zero and seven unused high bits.
    InvalidUnusedHighBits,
}

/// A redacted RSA-PSS data-block-unmask failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssDataBlockUnmaskError {
    kind: RsaPssDataBlockUnmaskErrorKind,
}

impl RsaPssDataBlockUnmaskError {
    const fn new(kind: RsaPssDataBlockUnmaskErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssDataBlockUnmaskErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssDataBlockUnmaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssDataBlockUnmaskError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssDataBlockUnmaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS data-block-unmask error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssDataBlockUnmaskError {}

/// Borrowed evidence retaining one complete unmasked RSA-PSS data block.
///
/// This type is deliberately neither `Copy` nor `Clone`. Its `Debug` output is
/// redacted. It proves only that the complete XOR and required high-bit clear
/// were applied; it proves no data-block structure, hash, RSA signature,
/// certificate, or trust verdict.
pub struct UnmaskedRsaPssDataBlock<'output> {
    data_block: &'output [u8],
    unused_high_bits: u8,
}

impl<'output> UnmaskedRsaPssDataBlock<'output> {
    /// Return the complete unmasked data-block bytes.
    pub const fn data_block(&self) -> &'output [u8] {
        self.data_block
    }

    /// Return the number of high bits cleared in the first output octet.
    pub const fn unused_high_bits(&self) -> u8 {
        self.unused_high_bits
    }
}

impl fmt::Debug for UnmaskedRsaPssDataBlock<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("UnmaskedRsaPssDataBlock")
    }
}

/// Unmask one complete RFC 8017 EMSA-PSS data block.
///
/// All lengths and the public unused-bit count are validated before `output`
/// is modified. Success writes `masked_data_block XOR data_block_mask` across
/// the complete output, then clears the selected high bits of its first octet.
/// This API makes no constant-time guarantee; RSA-PSS verification values are
/// public.
pub fn unmask_rsa_pss_data_block<'output>(
    masked_data_block: &[u8],
    data_block_mask: &[u8],
    output: &'output mut [u8],
    unused_high_bits: u8,
    policy: RsaPssDataBlockUnmaskPolicy,
) -> Result<UnmaskedRsaPssDataBlock<'output>, RsaPssDataBlockUnmaskError> {
    if masked_data_block.is_empty() {
        return Err(RsaPssDataBlockUnmaskError::new(
            RsaPssDataBlockUnmaskErrorKind::EmptyMaskedDataBlock,
        ));
    }
    if masked_data_block.len() > policy.maximum_data_block_bytes {
        return Err(RsaPssDataBlockUnmaskError::new(
            RsaPssDataBlockUnmaskErrorKind::MaskedDataBlockTooLongForPolicy,
        ));
    }
    if data_block_mask.len() != masked_data_block.len() {
        return Err(RsaPssDataBlockUnmaskError::new(
            RsaPssDataBlockUnmaskErrorKind::DataBlockMaskLengthMismatch,
        ));
    }
    if output.len() != masked_data_block.len() {
        return Err(RsaPssDataBlockUnmaskError::new(
            RsaPssDataBlockUnmaskErrorKind::OutputLengthMismatch,
        ));
    }
    if unused_high_bits > 7 {
        return Err(RsaPssDataBlockUnmaskError::new(
            RsaPssDataBlockUnmaskErrorKind::InvalidUnusedHighBits,
        ));
    }

    for ((output_octet, masked_octet), mask_octet) in output
        .iter_mut()
        .zip(masked_data_block)
        .zip(data_block_mask)
    {
        *output_octet = masked_octet ^ mask_octet;
    }
    output[0] &= u8::MAX >> unused_high_bits;

    Ok(UnmaskedRsaPssDataBlock {
        data_block: output,
        unused_high_bits,
    })
}
