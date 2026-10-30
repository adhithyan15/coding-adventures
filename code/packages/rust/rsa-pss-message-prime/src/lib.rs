//! # Bounded RSA-PSS `M'` construction
//!
//! RFC 8017 EMSA-PSS verification hashes `M' = 0x00 00 00 00 00 00 00 00 ||
//! mHash || salt`. This crate constructs that exact public byte string in a
//! caller-provided output buffer under caller-selected length policy.
//!
//! It does not hash a message or `M'`, compare `H`, run MGF1, interpret a PSS
//! data block, perform RSA arithmetic, or establish a signature verdict.

#![deny(unsafe_code)]

use core::fmt;

/// The eight zero octets prefixed to `mHash || salt` by RFC 8017.
pub const RSA_PSS_MESSAGE_PRIME_PREFIX_BYTES: usize = 8;

/// Stable invalid-policy categories that retain no caller-supplied values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssMessagePrimePolicyErrorKind {
    /// A selected message hash must produce at least one octet.
    ZeroMessageHashLength,
    /// A zero ceiling admits no `M'` value.
    ZeroMaximumMessagePrimeBytes,
    /// The prefix, selected message hash, and salt lengths overflow `usize`.
    LengthOverflow,
    /// The ceiling cannot hold the selected canonical `M'` value.
    MaximumMessagePrimeTooSmall,
}

/// A redacted policy-construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssMessagePrimePolicyError {
    kind: RsaPssMessagePrimePolicyErrorKind,
}

impl RsaPssMessagePrimePolicyError {
    const fn new(kind: RsaPssMessagePrimePolicyErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssMessagePrimePolicyErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssMessagePrimePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssMessagePrimePolicyError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssMessagePrimePolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS M-prime policy error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssMessagePrimePolicyError {}

/// Validated sizing policy for one selected RSA-PSS hash and salt profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RsaPssMessagePrimePolicy {
    message_hash_length: usize,
    salt_length: usize,
    maximum_message_prime_bytes: usize,
    message_prime_length: usize,
}

impl RsaPssMessagePrimePolicy {
    /// Validate selected `mHash`/salt lengths and an `M'` byte ceiling.
    ///
    /// A zero-length salt is valid. A zero-length message hash is not a usable
    /// hash-algorithm profile.
    pub const fn new(
        message_hash_length: usize,
        salt_length: usize,
        maximum_message_prime_bytes: usize,
    ) -> Result<Self, RsaPssMessagePrimePolicyError> {
        if message_hash_length == 0 {
            return Err(RsaPssMessagePrimePolicyError::new(
                RsaPssMessagePrimePolicyErrorKind::ZeroMessageHashLength,
            ));
        }
        if maximum_message_prime_bytes == 0 {
            return Err(RsaPssMessagePrimePolicyError::new(
                RsaPssMessagePrimePolicyErrorKind::ZeroMaximumMessagePrimeBytes,
            ));
        }
        let prefix_and_hash =
            match RSA_PSS_MESSAGE_PRIME_PREFIX_BYTES.checked_add(message_hash_length) {
                Some(length) => length,
                None => {
                    return Err(RsaPssMessagePrimePolicyError::new(
                        RsaPssMessagePrimePolicyErrorKind::LengthOverflow,
                    ))
                }
            };
        let message_prime_length = match prefix_and_hash.checked_add(salt_length) {
            Some(length) => length,
            None => {
                return Err(RsaPssMessagePrimePolicyError::new(
                    RsaPssMessagePrimePolicyErrorKind::LengthOverflow,
                ))
            }
        };
        if maximum_message_prime_bytes < message_prime_length {
            return Err(RsaPssMessagePrimePolicyError::new(
                RsaPssMessagePrimePolicyErrorKind::MaximumMessagePrimeTooSmall,
            ));
        }
        Ok(Self {
            message_hash_length,
            salt_length,
            maximum_message_prime_bytes,
            message_prime_length,
        })
    }

    /// Return the selected message-hash output length.
    pub const fn message_hash_length(self) -> usize {
        self.message_hash_length
    }

    /// Return the selected salt length.
    pub const fn salt_length(self) -> usize {
        self.salt_length
    }

    /// Return the caller-owned `M'` work ceiling.
    pub const fn maximum_message_prime_bytes(self) -> usize {
        self.maximum_message_prime_bytes
    }

    /// Return the exact canonical output length for this profile.
    pub const fn message_prime_length(self) -> usize {
        self.message_prime_length
    }
}

/// Stable construction-failure categories that retain no caller values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RsaPssMessagePrimeErrorKind {
    /// The supplied `mHash` length differs from the selected hash profile.
    MessageHashLengthMismatch,
    /// The supplied salt length differs from the selected salt profile.
    SaltLengthMismatch,
    /// The output buffer is not exactly the canonical `M'` length.
    OutputLengthMismatch,
}

/// A redacted RSA-PSS `M'` construction failure.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct RsaPssMessagePrimeError {
    kind: RsaPssMessagePrimeErrorKind,
}

impl RsaPssMessagePrimeError {
    const fn new(kind: RsaPssMessagePrimeErrorKind) -> Self {
        Self { kind }
    }

    /// Return the stable redacted failure category.
    pub const fn kind(self) -> RsaPssMessagePrimeErrorKind {
        self.kind
    }
}

impl fmt::Debug for RsaPssMessagePrimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RsaPssMessagePrimeError")
            .field("kind", &self.kind)
            .finish()
    }
}

impl fmt::Display for RsaPssMessagePrimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "RSA-PSS M-prime error {:?}", self.kind)
    }
}

impl std::error::Error for RsaPssMessagePrimeError {}

/// Borrowed evidence retaining one exact canonical RSA-PSS `M'` value.
///
/// This type is deliberately neither `Copy` nor `Clone`. Its `Debug` output is
/// redacted because a caller may use this primitive with non-public messages,
/// although certificate-signature verification inputs are public.
pub struct EncodedRsaPssMessagePrime<'output> {
    bytes: &'output [u8],
    message_hash: &'output [u8],
    salt: &'output [u8],
}

impl<'output> EncodedRsaPssMessagePrime<'output> {
    /// Return the complete exact `M'` bytes.
    pub const fn bytes(&self) -> &'output [u8] {
        self.bytes
    }

    /// Return the copied `mHash` field inside `M'`.
    pub const fn message_hash(&self) -> &'output [u8] {
        self.message_hash
    }

    /// Return the copied salt field inside `M'`.
    pub const fn salt(&self) -> &'output [u8] {
        self.salt
    }
}

impl fmt::Debug for EncodedRsaPssMessagePrime<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("EncodedRsaPssMessagePrime")
    }
}

/// Construct exact RFC 8017 `M'` bytes in a caller-provided output buffer.
///
/// All input lengths and the exact output length are checked before the output
/// is modified. The successful output is `zeroes(8) || message_hash || salt`.
pub fn encode_rsa_pss_message_prime<'output>(
    message_hash: &[u8],
    salt: &[u8],
    output: &'output mut [u8],
    policy: RsaPssMessagePrimePolicy,
) -> Result<EncodedRsaPssMessagePrime<'output>, RsaPssMessagePrimeError> {
    if message_hash.len() != policy.message_hash_length {
        return Err(RsaPssMessagePrimeError::new(
            RsaPssMessagePrimeErrorKind::MessageHashLengthMismatch,
        ));
    }
    if salt.len() != policy.salt_length {
        return Err(RsaPssMessagePrimeError::new(
            RsaPssMessagePrimeErrorKind::SaltLengthMismatch,
        ));
    }
    if output.len() != policy.message_prime_length {
        return Err(RsaPssMessagePrimeError::new(
            RsaPssMessagePrimeErrorKind::OutputLengthMismatch,
        ));
    }

    let message_hash_start = RSA_PSS_MESSAGE_PRIME_PREFIX_BYTES;
    let salt_start = message_hash_start + policy.message_hash_length;
    output[..message_hash_start].fill(0);
    output[message_hash_start..salt_start].copy_from_slice(message_hash);
    output[salt_start..].copy_from_slice(salt);

    Ok(EncodedRsaPssMessagePrime {
        bytes: output,
        message_hash: &output[message_hash_start..salt_start],
        salt: &output[salt_start..],
    })
}
