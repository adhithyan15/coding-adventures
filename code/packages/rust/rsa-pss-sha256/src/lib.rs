//! # Exact EMSA-PSS SHA-256 encoded messages
//!
//! This crate validates the complete byte-level RFC 8017 EMSA-PSS encoding for
//! one fixed profile: SHA-256, MGF1-SHA-256, a 32-byte salt, and trailer field
//! `0xbc`. The caller supplies one complete recovered encoded message, public
//! `emBits`, the expected message digest, exact DB scratch storage, and a
//! caller-owned work ceiling.
//!
//! The crate performs no RSA arithmetic and grants no signature, certificate,
//! TLS, HTTP, OAuth, or network authority by itself.

#![deny(unsafe_code)]

use core::fmt;
use rsa_pss_encoded_message_layout::{
    decode_rsa_pss_encoded_message_layout, RsaPssEncodedMessageLayoutErrorKind,
    RsaPssEncodedMessageLayoutPolicy, RsaPssEncodedMessageLayoutPolicyErrorKind,
};
use rsa_pss_mgf1_sha256::{
    generate_mgf1_sha256_mask, Mgf1Sha256ErrorKind, Mgf1Sha256Limits, Mgf1Sha256LimitsErrorKind,
};

/// SHA-256 and this fixed PSS profile both use 32 octets.
pub const SHA256_DIGEST_LENGTH: usize = 32;

/// This reviewed profile requires salt length to equal SHA-256 output length.
pub const SHA256_PSS_SALT_LENGTH: usize = SHA256_DIGEST_LENGTH;

/// Eight zero octets, one message digest, and one salt are hashed as `M'`.
const PSS_HASH_INPUT_LENGTH: usize = 8 + SHA256_DIGEST_LENGTH + SHA256_PSS_SALT_LENGTH;

/// Stable invalid-limits categories retaining no caller values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssSha256LimitsErrorKind {
    Layout(RsaPssEncodedMessageLayoutPolicyErrorKind),
    MaskGeneration(Mgf1Sha256LimitsErrorKind),
}

/// A redacted invalid-limits error.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssSha256LimitsError {
    kind: RsaPssSha256LimitsErrorKind,
}

impl RsaPssSha256LimitsError {
    const fn new(kind: RsaPssSha256LimitsErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free category.
    #[must_use]
    pub const fn kind(self) -> RsaPssSha256LimitsErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssSha256LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssSha256LimitsError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssSha256LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS SHA-256 limits error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssSha256LimitsError {}

/// Validated public work limits for one fixed-profile verdict.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssSha256Limits {
    layout: RsaPssEncodedMessageLayoutPolicy,
    mask_generation: Mgf1Sha256Limits,
}

impl RsaPssSha256Limits {
    /// Validate one encoded-message ceiling for both layout and MGF1 work.
    pub const fn new(
        maximum_encoded_message_bytes: usize,
    ) -> Result<Self, RsaPssSha256LimitsError> {
        let layout = match RsaPssEncodedMessageLayoutPolicy::new(
            SHA256_DIGEST_LENGTH,
            SHA256_PSS_SALT_LENGTH,
            maximum_encoded_message_bytes,
        ) {
            Ok(layout) => layout,
            Err(error) => {
                return Err(RsaPssSha256LimitsError::new(
                    RsaPssSha256LimitsErrorKind::Layout(error.kind()),
                ))
            }
        };
        let mask_generation = match Mgf1Sha256Limits::new(maximum_encoded_message_bytes) {
            Ok(limits) => limits,
            Err(error) => {
                return Err(RsaPssSha256LimitsError::new(
                    RsaPssSha256LimitsErrorKind::MaskGeneration(error.kind()),
                ))
            }
        };
        Ok(Self {
            layout,
            mask_generation,
        })
    }

    /// Return the caller-selected encoded-message ceiling.
    #[must_use]
    pub const fn maximum_encoded_message_bytes(self) -> usize {
        self.layout.maximum_encoded_message_bytes()
    }
}

/// Stable encoded-message failures retaining no caller bytes or offsets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssSha256ErrorKind {
    Layout(RsaPssEncodedMessageLayoutErrorKind),
    DataBlockScratchLengthMismatch,
    MaskGeneration(Mgf1Sha256ErrorKind),
    InvalidPadding,
    InvalidDelimiter,
    HashMismatch,
}

/// A redacted EMSA-PSS SHA-256 validation failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssSha256Error {
    kind: RsaPssSha256ErrorKind,
}

impl RsaPssSha256Error {
    const fn new(kind: RsaPssSha256ErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable payload-free category.
    #[must_use]
    pub const fn kind(self) -> RsaPssSha256ErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssSha256Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssSha256Error")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssSha256Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA-PSS SHA-256 encoded-message error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for RsaPssSha256Error {}

/// Opaque evidence that one complete encoded message matched one digest.
///
/// This verdict proves only the byte-level fixed-profile EMSA-PSS encoding.
/// A later verifier must derive the encoded message through an exact RSA
/// public operation and retain the corresponding key and signed input.
pub struct ValidatedRsaPssSha256EncodedMessage {
    _private: (),
}

impl fmt::Debug for ValidatedRsaPssSha256EncodedMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedRsaPssSha256EncodedMessage")
    }
}

/// Validate one exact RFC 8017 EMSA-PSS SHA-256 encoded message.
///
/// The scratch length must equal the decoded `maskedDB` length. Layout and
/// scratch checks complete before scratch mutation. After MGF1 succeeds,
/// scratch is overwritten with the public unmasked DB and may remain modified
/// on a later padding, delimiter, or hash failure.
pub fn validate_rsa_pss_sha256_encoded_message(
    encoded_message: &[u8],
    encoded_message_bits: usize,
    expected_message_digest: &[u8; SHA256_DIGEST_LENGTH],
    data_block_scratch: &mut [u8],
    limits: RsaPssSha256Limits,
) -> Result<ValidatedRsaPssSha256EncodedMessage, RsaPssSha256Error> {
    let layout =
        decode_rsa_pss_encoded_message_layout(encoded_message, encoded_message_bits, limits.layout)
            .map_err(|error| RsaPssSha256Error::new(RsaPssSha256ErrorKind::Layout(error.kind())))?;

    let masked_data_block = layout.masked_data_block();
    if data_block_scratch.len() != masked_data_block.len() {
        return Err(RsaPssSha256Error::new(
            RsaPssSha256ErrorKind::DataBlockScratchLengthMismatch,
        ));
    }

    let hash: &[u8; SHA256_DIGEST_LENGTH] = layout
        .hash()
        .try_into()
        .expect("fixed SHA-256 layout returns an exact digest width");
    generate_mgf1_sha256_mask(hash, data_block_scratch, limits.mask_generation).map_err(
        |error| RsaPssSha256Error::new(RsaPssSha256ErrorKind::MaskGeneration(error.kind())),
    )?;
    for (unmasked, masked) in data_block_scratch.iter_mut().zip(masked_data_block) {
        *unmasked ^= masked;
    }
    if layout.unused_high_bits() != 0 {
        data_block_scratch[0] &= u8::MAX >> layout.unused_high_bits();
    }

    let padding_length = data_block_scratch.len() - SHA256_PSS_SALT_LENGTH - 1;
    if data_block_scratch[..padding_length]
        .iter()
        .any(|octet| *octet != 0)
    {
        return Err(RsaPssSha256Error::new(
            RsaPssSha256ErrorKind::InvalidPadding,
        ));
    }
    if data_block_scratch[padding_length] != 0x01 {
        return Err(RsaPssSha256Error::new(
            RsaPssSha256ErrorKind::InvalidDelimiter,
        ));
    }

    let salt = &data_block_scratch[padding_length + 1..];
    let mut hash_input = [0u8; PSS_HASH_INPUT_LENGTH];
    hash_input[8..8 + SHA256_DIGEST_LENGTH].copy_from_slice(expected_message_digest);
    hash_input[8 + SHA256_DIGEST_LENGTH..].copy_from_slice(salt);
    let recomputed_hash = coding_adventures_sha256::sha256(&hash_input);
    let difference = hash
        .iter()
        .zip(recomputed_hash)
        .fold(0u8, |difference, (encoded, recomputed)| {
            difference | (encoded ^ recomputed)
        });
    if difference != 0 {
        return Err(RsaPssSha256Error::new(RsaPssSha256ErrorKind::HashMismatch));
    }

    Ok(ValidatedRsaPssSha256EncodedMessage { _private: () })
}
