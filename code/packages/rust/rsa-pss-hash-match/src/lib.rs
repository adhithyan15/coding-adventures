//! # Bounded RSA-PSS hash-field matching
//!
//! RFC 8017 EMSA-PSS verification compares the hash of canonical `M'` with the
//! encoded message's `H` field. This crate performs only that exact public-byte
//! match under a caller-selected digest length and work ceiling.
//!
//! It does not construct or hash `M'`, run MGF1, validate a data block, perform
//! RSA arithmetic, or establish a signature verdict.

#![deny(unsafe_code)]

use core::fmt;

/// Stable invalid-policy categories that retain no caller-supplied values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssHashMatchPolicyErrorKind {
    /// A selected hash must produce at least one octet.
    ZeroHashLength,
    /// A zero ceiling admits no hash comparison.
    ZeroMaximumHashBytes,
    /// The selected hash length exceeds the caller-owned ceiling.
    HashLengthExceedsMaximum,
}

/// A redacted hash-match policy-construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssHashMatchPolicyError {
    kind: RsaPssHashMatchPolicyErrorKind,
}

impl RsaPssHashMatchPolicyError {
    const fn new(kind: RsaPssHashMatchPolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssHashMatchPolicyErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssHashMatchPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssHashMatchPolicyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssHashMatchPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS hash-match policy error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssHashMatchPolicyError {}

/// Validated public sizing policy for one selected RSA-PSS hash profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssHashMatchPolicy {
    hash_length: usize,
    maximum_hash_bytes: usize,
}

impl RsaPssHashMatchPolicy {
    /// Validate a selected digest length and public comparison ceiling.
    pub const fn new(
        hash_length: usize,
        maximum_hash_bytes: usize,
    ) -> Result<Self, RsaPssHashMatchPolicyError> {
        if hash_length == 0 {
            return Err(RsaPssHashMatchPolicyError::new(
                RsaPssHashMatchPolicyErrorKind::ZeroHashLength,
            ));
        }
        if maximum_hash_bytes == 0 {
            return Err(RsaPssHashMatchPolicyError::new(
                RsaPssHashMatchPolicyErrorKind::ZeroMaximumHashBytes,
            ));
        }
        if hash_length > maximum_hash_bytes {
            return Err(RsaPssHashMatchPolicyError::new(
                RsaPssHashMatchPolicyErrorKind::HashLengthExceedsMaximum,
            ));
        }
        Ok(Self {
            hash_length,
            maximum_hash_bytes,
        })
    }

    /// Return the selected digest length.
    pub const fn hash_length(self) -> usize {
        self.hash_length
    }

    /// Return the caller-owned comparison ceiling.
    pub const fn maximum_hash_bytes(self) -> usize {
        self.maximum_hash_bytes
    }
}

/// Stable hash-match failures that retain no caller bytes or lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssHashMatchErrorKind {
    /// The recomputed `H'` length differs from the selected hash profile.
    ComputedHashLengthMismatch,
    /// The encoded message's `H` length differs from the selected profile.
    EncodedHashLengthMismatch,
    /// The complete selected-length `H'` and `H` values differ.
    HashMismatch,
}

/// A redacted RSA-PSS hash-match failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssHashMatchError {
    kind: RsaPssHashMatchErrorKind,
}

impl RsaPssHashMatchError {
    const fn new(kind: RsaPssHashMatchErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssHashMatchErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssHashMatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssHashMatchError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssHashMatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS hash-match error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssHashMatchError {}

/// Borrowed evidence that one complete recomputed `H'` equals encoded `H`.
///
/// This type is deliberately neither `Copy` nor `Clone`. Its `Debug` output is
/// redacted because a generic caller's digests may describe non-public data.
/// It proves no RSA signature, certificate, or trust verdict.
pub struct ValidatedRsaPssHashMatch<'computed, 'encoded> {
    computed_hash: &'computed [u8],
    encoded_hash: &'encoded [u8],
}

impl<'computed, 'encoded> ValidatedRsaPssHashMatch<'computed, 'encoded> {
    /// Return the complete recomputed `H'` bytes.
    pub const fn computed_hash(&self) -> &'computed [u8] {
        self.computed_hash
    }

    /// Return the complete encoded `H` bytes.
    pub const fn encoded_hash(&self) -> &'encoded [u8] {
        self.encoded_hash
    }

    /// Return the selected digest length proven equal on both sides.
    pub const fn hash_length(&self) -> usize {
        self.computed_hash.len()
    }
}

impl fmt::Debug for ValidatedRsaPssHashMatch<'_, '_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ValidatedRsaPssHashMatch")
    }
}

/// Match one exact recomputed `H'` against one exact encoded `H` field.
///
/// Lengths are validated before comparison. The comparison consumes every
/// selected byte and does not report a mismatch position. This API makes no
/// constant-time guarantee; RSA-PSS verification values are public.
pub fn validate_rsa_pss_hash_match<'computed, 'encoded>(
    computed_hash: &'computed [u8],
    encoded_hash: &'encoded [u8],
    policy: RsaPssHashMatchPolicy,
) -> Result<ValidatedRsaPssHashMatch<'computed, 'encoded>, RsaPssHashMatchError> {
    if computed_hash.len() != policy.hash_length {
        return Err(RsaPssHashMatchError::new(
            RsaPssHashMatchErrorKind::ComputedHashLengthMismatch,
        ));
    }
    if encoded_hash.len() != policy.hash_length {
        return Err(RsaPssHashMatchError::new(
            RsaPssHashMatchErrorKind::EncodedHashLengthMismatch,
        ));
    }

    let difference = computed_hash
        .iter()
        .zip(encoded_hash)
        .fold(0u8, |difference, (computed, encoded)| {
            difference | (computed ^ encoded)
        });
    if difference != 0 {
        return Err(RsaPssHashMatchError::new(
            RsaPssHashMatchErrorKind::HashMismatch,
        ));
    }

    Ok(ValidatedRsaPssHashMatch {
        computed_hash,
        encoded_hash,
    })
}
