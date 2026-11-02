//! # Exact EMSA-PSS SHA-384 encoded messages
//!
//! This crate validates the complete byte-level RFC 8017 EMSA-PSS encoding for
//! one fixed profile: SHA-384, MGF1-SHA-384, a 48-byte salt, and trailer field
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
use rsa_pss_mgf1_sha384::{
    generate_mgf1_sha384_mask, Mgf1Sha384ErrorKind, Mgf1Sha384Limits, Mgf1Sha384LimitsErrorKind,
};

/// SHA-384 and this fixed PSS profile both use 48 octets.
pub const SHA384_DIGEST_LENGTH: usize = 48;
/// This reviewed profile requires salt length to equal SHA-384 output length.
pub const SHA384_PSS_SALT_LENGTH: usize = SHA384_DIGEST_LENGTH;
const PSS_HASH_INPUT_LENGTH: usize = 8 + SHA384_DIGEST_LENGTH + SHA384_PSS_SALT_LENGTH;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssSha384LimitsErrorKind {
    Layout(RsaPssEncodedMessageLayoutPolicyErrorKind),
    MaskGeneration(Mgf1Sha384LimitsErrorKind),
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssSha384LimitsError {
    kind: RsaPssSha384LimitsErrorKind,
}

impl RsaPssSha384LimitsError {
    const fn new(kind: RsaPssSha384LimitsErrorKind) -> Self {
        Self { kind }
    }
    #[must_use]
    pub const fn kind(self) -> RsaPssSha384LimitsErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssSha384LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssSha384LimitsError")
            .field("kind", &self.kind)
            .finish()
    }
}
impl fmt::Display for RsaPssSha384LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS SHA-384 limits error {:?}", self.kind)
    }
}
impl std::error::Error for RsaPssSha384LimitsError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssSha384Limits {
    layout: RsaPssEncodedMessageLayoutPolicy,
    mask_generation: Mgf1Sha384Limits,
}

impl RsaPssSha384Limits {
    pub const fn new(
        maximum_encoded_message_bytes: usize,
    ) -> Result<Self, RsaPssSha384LimitsError> {
        let layout = match RsaPssEncodedMessageLayoutPolicy::new(
            SHA384_DIGEST_LENGTH,
            SHA384_PSS_SALT_LENGTH,
            maximum_encoded_message_bytes,
        ) {
            Ok(layout) => layout,
            Err(error) => {
                return Err(RsaPssSha384LimitsError::new(
                    RsaPssSha384LimitsErrorKind::Layout(error.kind()),
                ))
            }
        };
        let mask_generation = match Mgf1Sha384Limits::new(maximum_encoded_message_bytes) {
            Ok(limits) => limits,
            Err(error) => {
                return Err(RsaPssSha384LimitsError::new(
                    RsaPssSha384LimitsErrorKind::MaskGeneration(error.kind()),
                ))
            }
        };
        Ok(Self {
            layout,
            mask_generation,
        })
    }

    #[must_use]
    pub const fn maximum_encoded_message_bytes(self) -> usize {
        self.layout.maximum_encoded_message_bytes()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssSha384ErrorKind {
    Layout(RsaPssEncodedMessageLayoutErrorKind),
    DataBlockScratchLengthMismatch,
    MaskGeneration(Mgf1Sha384ErrorKind),
    InvalidPadding,
    InvalidDelimiter,
    HashMismatch,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssSha384Error {
    kind: RsaPssSha384ErrorKind,
}
impl RsaPssSha384Error {
    const fn new(kind: RsaPssSha384ErrorKind) -> Self {
        Self { kind }
    }
    #[must_use]
    pub const fn kind(self) -> RsaPssSha384ErrorKind {
        self.kind
    }
}
impl fmt::Debug for RsaPssSha384Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssSha384Error")
            .field("kind", &self.kind)
            .finish()
    }
}
impl fmt::Display for RsaPssSha384Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA-PSS SHA-384 encoded-message error {:?}",
            self.kind
        )
    }
}
impl std::error::Error for RsaPssSha384Error {}

pub struct ValidatedRsaPssSha384EncodedMessage {
    _private: (),
}
impl fmt::Debug for ValidatedRsaPssSha384EncodedMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedRsaPssSha384EncodedMessage")
    }
}

/// Validate one exact RFC 8017 EMSA-PSS SHA-384 encoded message.
///
/// Layout and exact scratch checks complete before scratch mutation. After
/// MGF1 succeeds, scratch contains the public unmasked DB and may remain
/// modified on a later padding, delimiter, or hash failure.
pub fn validate_rsa_pss_sha384_encoded_message(
    encoded_message: &[u8],
    encoded_message_bits: usize,
    expected_message_digest: &[u8; SHA384_DIGEST_LENGTH],
    data_block_scratch: &mut [u8],
    limits: RsaPssSha384Limits,
) -> Result<ValidatedRsaPssSha384EncodedMessage, RsaPssSha384Error> {
    let layout =
        decode_rsa_pss_encoded_message_layout(encoded_message, encoded_message_bits, limits.layout)
            .map_err(|error| RsaPssSha384Error::new(RsaPssSha384ErrorKind::Layout(error.kind())))?;

    let masked_data_block = layout.masked_data_block();
    if data_block_scratch.len() != masked_data_block.len() {
        return Err(RsaPssSha384Error::new(
            RsaPssSha384ErrorKind::DataBlockScratchLengthMismatch,
        ));
    }

    let hash: &[u8; SHA384_DIGEST_LENGTH] = layout
        .hash()
        .try_into()
        .expect("fixed SHA-384 layout returns an exact digest width");
    generate_mgf1_sha384_mask(hash, data_block_scratch, limits.mask_generation).map_err(
        |error| RsaPssSha384Error::new(RsaPssSha384ErrorKind::MaskGeneration(error.kind())),
    )?;
    for (unmasked, masked) in data_block_scratch.iter_mut().zip(masked_data_block) {
        *unmasked ^= masked;
    }
    if layout.unused_high_bits() != 0 {
        data_block_scratch[0] &= u8::MAX >> layout.unused_high_bits();
    }

    let padding_length = data_block_scratch.len() - SHA384_PSS_SALT_LENGTH - 1;
    if data_block_scratch[..padding_length]
        .iter()
        .any(|octet| *octet != 0)
    {
        return Err(RsaPssSha384Error::new(
            RsaPssSha384ErrorKind::InvalidPadding,
        ));
    }
    if data_block_scratch[padding_length] != 0x01 {
        return Err(RsaPssSha384Error::new(
            RsaPssSha384ErrorKind::InvalidDelimiter,
        ));
    }

    let salt = &data_block_scratch[padding_length + 1..];
    let mut hash_input = [0u8; PSS_HASH_INPUT_LENGTH];
    hash_input[8..8 + SHA384_DIGEST_LENGTH].copy_from_slice(expected_message_digest);
    hash_input[8 + SHA384_DIGEST_LENGTH..].copy_from_slice(salt);
    let recomputed_hash = coding_adventures_sha384::sha384(&hash_input);
    let difference = hash
        .iter()
        .zip(recomputed_hash)
        .fold(0u8, |difference, (encoded, recomputed)| {
            difference | (encoded ^ recomputed)
        });
    if difference != 0 {
        return Err(RsaPssSha384Error::new(RsaPssSha384ErrorKind::HashMismatch));
    }

    Ok(ValidatedRsaPssSha384EncodedMessage { _private: () })
}
