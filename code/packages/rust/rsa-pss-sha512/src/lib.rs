//! # Exact EMSA-PSS SHA-512 encoded messages
//!
//! This crate validates the complete byte-level RFC 8017 EMSA-PSS encoding for
//! one fixed profile: SHA-512, MGF1-SHA-512, a 64-byte salt, and trailer field
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
use rsa_pss_mgf1_sha512::{
    generate_mgf1_sha512_mask, Mgf1Sha512ErrorKind, Mgf1Sha512Limits, Mgf1Sha512LimitsErrorKind,
};

/// SHA-512 and this fixed PSS profile both use 64 octets.
pub const SHA512_DIGEST_LENGTH: usize = 64;
/// This reviewed profile requires salt length to equal SHA-512 output length.
pub const SHA512_PSS_SALT_LENGTH: usize = SHA512_DIGEST_LENGTH;
const PSS_HASH_INPUT_LENGTH: usize = 8 + SHA512_DIGEST_LENGTH + SHA512_PSS_SALT_LENGTH;

/// Stable work-limit failures retaining no caller values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssSha512LimitsErrorKind {
    /// The encoded-message layout policy rejected the requested ceiling.
    Layout(RsaPssEncodedMessageLayoutPolicyErrorKind),
    /// The MGF1 policy rejected the requested ceiling.
    MaskGeneration(Mgf1Sha512LimitsErrorKind),
}

/// A redacted fixed-profile work-limit failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssSha512LimitsError {
    kind: RsaPssSha512LimitsErrorKind,
}

impl RsaPssSha512LimitsError {
    const fn new(kind: RsaPssSha512LimitsErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> RsaPssSha512LimitsErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssSha512LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssSha512LimitsError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssSha512LimitsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS SHA-512 limits error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssSha512LimitsError {}

/// Validated public work limits for one fixed SHA-512 PSS profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssSha512Limits {
    layout: RsaPssEncodedMessageLayoutPolicy,
    mask_generation: Mgf1Sha512Limits,
}

impl RsaPssSha512Limits {
    /// Validate one encoded-message ceiling for both composed operations.
    pub const fn new(
        maximum_encoded_message_bytes: usize,
    ) -> Result<Self, RsaPssSha512LimitsError> {
        let layout = match RsaPssEncodedMessageLayoutPolicy::new(
            SHA512_DIGEST_LENGTH,
            SHA512_PSS_SALT_LENGTH,
            maximum_encoded_message_bytes,
        ) {
            Ok(layout) => layout,
            Err(error) => {
                return Err(RsaPssSha512LimitsError::new(
                    RsaPssSha512LimitsErrorKind::Layout(error.kind()),
                ))
            }
        };
        let mask_generation = match Mgf1Sha512Limits::new(maximum_encoded_message_bytes) {
            Ok(limits) => limits,
            Err(error) => {
                return Err(RsaPssSha512LimitsError::new(
                    RsaPssSha512LimitsErrorKind::MaskGeneration(error.kind()),
                ))
            }
        };
        Ok(Self {
            layout,
            mask_generation,
        })
    }

    /// Return the caller-owned encoded-message ceiling.
    #[must_use]
    pub const fn maximum_encoded_message_bytes(self) -> usize {
        self.layout.maximum_encoded_message_bytes()
    }
}

/// Stable validation failures retaining no caller bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssSha512ErrorKind {
    /// The public outer encoded-message layout is invalid.
    Layout(RsaPssEncodedMessageLayoutErrorKind),
    /// The caller-owned DB scratch length is not exact.
    DataBlockScratchLengthMismatch,
    /// Bounded SHA-512 MGF1 failed.
    MaskGeneration(Mgf1Sha512ErrorKind),
    /// The complete zero padding is invalid.
    InvalidPadding,
    /// The required `0x01` delimiter is invalid.
    InvalidDelimiter,
    /// Recomputed `H'` differs from encoded `H`.
    HashMismatch,
}

/// A redacted fixed-profile encoded-message failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssSha512Error {
    kind: RsaPssSha512ErrorKind,
}

impl RsaPssSha512Error {
    const fn new(kind: RsaPssSha512ErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    #[must_use]
    pub const fn kind(self) -> RsaPssSha512ErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssSha512Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssSha512Error")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssSha512Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "RSA-PSS SHA-512 encoded-message error {:?}",
            self.kind
        )
    }
}

impl std::error::Error for RsaPssSha512Error {}

/// Evidence that one complete recovered encoded message matches this profile.
///
/// This evidence is deliberately neither `Copy` nor `Clone`, retains no input
/// bytes, and proves no RSA signature, certificate, path, or transport verdict.
pub struct ValidatedRsaPssSha512EncodedMessage {
    _private: (),
}

impl fmt::Debug for ValidatedRsaPssSha512EncodedMessage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedRsaPssSha512EncodedMessage")
    }
}

/// Validate one exact RFC 8017 EMSA-PSS SHA-512 encoded message.
///
/// Layout and exact scratch checks complete before scratch mutation. After
/// MGF1 succeeds, scratch contains the public unmasked DB and may remain
/// modified on a later padding, delimiter, or hash failure. The hash comparison
/// consumes all 64 public bytes but makes no constant-time guarantee.
pub fn validate_rsa_pss_sha512_encoded_message(
    encoded_message: &[u8],
    encoded_message_bits: usize,
    expected_message_digest: &[u8; SHA512_DIGEST_LENGTH],
    data_block_scratch: &mut [u8],
    limits: RsaPssSha512Limits,
) -> Result<ValidatedRsaPssSha512EncodedMessage, RsaPssSha512Error> {
    let layout =
        decode_rsa_pss_encoded_message_layout(encoded_message, encoded_message_bits, limits.layout)
            .map_err(|error| RsaPssSha512Error::new(RsaPssSha512ErrorKind::Layout(error.kind())))?;

    let masked_data_block = layout.masked_data_block();
    if data_block_scratch.len() != masked_data_block.len() {
        return Err(RsaPssSha512Error::new(
            RsaPssSha512ErrorKind::DataBlockScratchLengthMismatch,
        ));
    }

    let hash: &[u8; SHA512_DIGEST_LENGTH] = layout
        .hash()
        .try_into()
        .expect("fixed SHA-512 layout returns an exact digest width");
    generate_mgf1_sha512_mask(hash, data_block_scratch, limits.mask_generation).map_err(
        |error| RsaPssSha512Error::new(RsaPssSha512ErrorKind::MaskGeneration(error.kind())),
    )?;
    for (unmasked, masked) in data_block_scratch.iter_mut().zip(masked_data_block) {
        *unmasked ^= masked;
    }
    if layout.unused_high_bits() != 0 {
        data_block_scratch[0] &= u8::MAX >> layout.unused_high_bits();
    }

    let padding_length = data_block_scratch.len() - SHA512_PSS_SALT_LENGTH - 1;
    if data_block_scratch[..padding_length]
        .iter()
        .any(|octet| *octet != 0)
    {
        return Err(RsaPssSha512Error::new(
            RsaPssSha512ErrorKind::InvalidPadding,
        ));
    }
    if data_block_scratch[padding_length] != 0x01 {
        return Err(RsaPssSha512Error::new(
            RsaPssSha512ErrorKind::InvalidDelimiter,
        ));
    }

    let salt = &data_block_scratch[padding_length + 1..];
    let mut hash_input = [0u8; PSS_HASH_INPUT_LENGTH];
    hash_input[8..8 + SHA512_DIGEST_LENGTH].copy_from_slice(expected_message_digest);
    hash_input[8 + SHA512_DIGEST_LENGTH..].copy_from_slice(salt);
    let recomputed_hash = coding_adventures_sha512::sum512(&hash_input);
    let difference = hash
        .iter()
        .zip(recomputed_hash)
        .fold(0u8, |difference, (encoded, recomputed)| {
            difference | (encoded ^ recomputed)
        });
    if difference != 0 {
        return Err(RsaPssSha512Error::new(RsaPssSha512ErrorKind::HashMismatch));
    }

    Ok(ValidatedRsaPssSha512EncodedMessage { _private: () })
}
