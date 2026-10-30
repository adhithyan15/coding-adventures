//! # Bounded RSA-PSS data-block validation
//!
//! After MGF1 unmasking and high-bit clearing, RFC 8017 EMSA-PSS verification
//! requires `DB = PS || 0x01 || salt`, where every octet in `PS` is zero.
//! This crate validates only that public structure under a caller-owned bound.
//!
//! It does not run MGF1, clear high bits, hash a message, compare `H`, perform
//! RSA arithmetic, or establish a signature verdict.

#![deny(unsafe_code)]

use core::fmt;

/// RFC 8017 EMSA-PSS's separator between `PS` and the salt.
pub const RSA_PSS_DATA_BLOCK_DELIMITER: u8 = 0x01;

/// Stable invalid-policy categories that retain no caller-supplied values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssDataBlockPolicyErrorKind {
    /// A zero ceiling admits no data block.
    ZeroMaximumDataBlockBytes,
    /// The selected salt plus its delimiter cannot be represented by `usize`.
    LengthOverflow,
    /// The ceiling cannot hold the selected salt and delimiter.
    MaximumDataBlockTooSmall,
}

/// A redacted policy-construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssDataBlockPolicyError {
    kind: RsaPssDataBlockPolicyErrorKind,
}

impl RsaPssDataBlockPolicyError {
    const fn new(kind: RsaPssDataBlockPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssDataBlockPolicyErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssDataBlockPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssDataBlockPolicyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssDataBlockPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS data-block policy error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssDataBlockPolicyError {}

/// Validated public sizing policy for one selected RSA-PSS salt profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssDataBlockPolicy {
    salt_length: usize,
    maximum_data_block_bytes: usize,
    minimum_data_block_bytes: usize,
}

impl RsaPssDataBlockPolicy {
    /// Validate a selected salt length and public data-block work ceiling.
    ///
    /// A zero-length salt is valid. The minimum data block contains the
    /// delimiter followed by the selected salt and may have an empty `PS`.
    pub const fn new(
        salt_length: usize,
        maximum_data_block_bytes: usize,
    ) -> Result<Self, RsaPssDataBlockPolicyError> {
        if maximum_data_block_bytes == 0 {
            return Err(RsaPssDataBlockPolicyError::new(
                RsaPssDataBlockPolicyErrorKind::ZeroMaximumDataBlockBytes,
            ));
        }
        let minimum_data_block_bytes = match salt_length.checked_add(1) {
            Some(length) => length,
            None => {
                return Err(RsaPssDataBlockPolicyError::new(
                    RsaPssDataBlockPolicyErrorKind::LengthOverflow,
                ))
            }
        };
        if maximum_data_block_bytes < minimum_data_block_bytes {
            return Err(RsaPssDataBlockPolicyError::new(
                RsaPssDataBlockPolicyErrorKind::MaximumDataBlockTooSmall,
            ));
        }
        Ok(Self {
            salt_length,
            maximum_data_block_bytes,
            minimum_data_block_bytes,
        })
    }

    /// Return the selected salt length.
    pub const fn salt_length(self) -> usize {
        self.salt_length
    }

    /// Return the caller-owned data-block work ceiling.
    pub const fn maximum_data_block_bytes(self) -> usize {
        self.maximum_data_block_bytes
    }

    /// Return the minimum data-block width allowed by the selected salt.
    pub const fn minimum_data_block_bytes(self) -> usize {
        self.minimum_data_block_bytes
    }
}

/// Stable validation-failure categories that retain no caller bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssDataBlockErrorKind {
    /// The input exceeds the caller-owned work ceiling.
    DataBlockTooLongForPolicy,
    /// The input cannot hold the selected salt and delimiter.
    DataBlockTooShortForSalt,
    /// At least one octet in `PS` is nonzero.
    NonzeroPaddingString,
    /// The separator before the salt is not `0x01`.
    InvalidDelimiter,
}

/// A redacted RSA-PSS data-block validation failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssDataBlockError {
    kind: RsaPssDataBlockErrorKind,
}

impl RsaPssDataBlockError {
    const fn new(kind: RsaPssDataBlockErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssDataBlockErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssDataBlockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssDataBlockError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssDataBlockError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS data-block error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssDataBlockError {}

/// Borrowed evidence that one public unmasked PSS data block is canonical.
///
/// This type is deliberately neither `Copy` nor `Clone`. It retains the exact
/// complete input and salt slice for the later `M'` hash construction, but
/// proves no hash, RSA signature, certificate, or trust verdict.
pub struct ValidatedRsaPssDataBlock<'block> {
    data_block: &'block [u8],
    salt: &'block [u8],
    padding_string_length: usize,
}

impl<'block> ValidatedRsaPssDataBlock<'block> {
    /// Return the complete exact unmasked data block.
    pub const fn data_block(&self) -> &'block [u8] {
        self.data_block
    }

    /// Return the exact borrowed salt selected by policy.
    pub const fn salt(&self) -> &'block [u8] {
        self.salt
    }

    /// Return the number of leading zero octets proven to form `PS`.
    pub const fn padding_string_length(&self) -> usize {
        self.padding_string_length
    }
}

impl fmt::Debug for ValidatedRsaPssDataBlock<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedRsaPssDataBlock")
    }
}

/// Validate one exact unmasked RFC 8017 EMSA-PSS data block.
///
/// The caller must first run the selected MGF1 operation and clear the unused
/// high bits required by `emBits`. This function validates only the resulting
/// `PS || 0x01 || salt` structure and returns the selected salt by borrowing.
pub fn validate_rsa_pss_data_block<'block>(
    data_block: &'block [u8],
    policy: RsaPssDataBlockPolicy,
) -> Result<ValidatedRsaPssDataBlock<'block>, RsaPssDataBlockError> {
    if data_block.len() > policy.maximum_data_block_bytes {
        return Err(RsaPssDataBlockError::new(
            RsaPssDataBlockErrorKind::DataBlockTooLongForPolicy,
        ));
    }
    if data_block.len() < policy.minimum_data_block_bytes {
        return Err(RsaPssDataBlockError::new(
            RsaPssDataBlockErrorKind::DataBlockTooShortForSalt,
        ));
    }

    let delimiter_index = data_block.len() - policy.salt_length - 1;
    if data_block[..delimiter_index]
        .iter()
        .any(|octet| *octet != 0)
    {
        return Err(RsaPssDataBlockError::new(
            RsaPssDataBlockErrorKind::NonzeroPaddingString,
        ));
    }
    if data_block[delimiter_index] != RSA_PSS_DATA_BLOCK_DELIMITER {
        return Err(RsaPssDataBlockError::new(
            RsaPssDataBlockErrorKind::InvalidDelimiter,
        ));
    }

    Ok(ValidatedRsaPssDataBlock {
        data_block,
        salt: &data_block[delimiter_index + 1..],
        padding_string_length: delimiter_index,
    })
}
