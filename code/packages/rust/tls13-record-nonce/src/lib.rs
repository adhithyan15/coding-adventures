//! Bounded TLS 1.3 record sequence and per-record nonce derivation.
//!
//! RFC 8446 gives every traffic key and direction its own 64-bit record
//! sequence. The first record uses zero. For every record, TLS encodes that
//! sequence in network byte order, pads it on the left to the AEAD nonce
//! length, and XORs it with the traffic key's static IV.
//!
//! This crate owns only that small state transition. It deliberately does not
//! store the static IV, select an AEAD, encrypt, decrypt, or reset a sequence
//! after a key update. A caller creates separate [`RecordNonceSequence`]
//! values for reading and writing and creates fresh state whenever the traffic
//! key changes.
//!
//! ```
//! use tls13_record_nonce::{NoncePolicy, RecordNonceSequence};
//!
//! let policy = NoncePolicy::new(12, 2)?;
//! let mut sequence = RecordNonceSequence::new(policy);
//! let static_iv = [0xa5; 12];
//! let mut nonce = [0u8; 12];
//!
//! let reservation = sequence.reserve_next_nonce(&static_iv, &mut nonce)?;
//! assert_eq!(reservation.sequence_number(), 0);
//! assert_eq!(nonce, static_iv);
//! # Ok::<(), tls13_record_nonce::Error>(())
//! ```

#![forbid(unsafe_code)]

use core::fmt;

/// RFC 8446 requires the TLS per-record nonce to be at least eight bytes.
pub const MIN_IV_LENGTH: usize = 8;

/// This package's explicit work bound for a caller-selected static IV.
///
/// The bound is a resource policy, not permission for an AEAD to accept every
/// length through 255. Cipher-suite policy must choose the exact permitted
/// length for the negotiated AEAD.
pub const MAX_IV_LENGTH: usize = 255;

/// Stable, redacted failures for policy construction and nonce reservation.
///
/// Variants intentionally retain no IV bytes, nonce bytes, sequence values,
/// configured limits, or caller-provided lengths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    /// The configured IV length is outside [`MIN_IV_LENGTH`] through
    /// [`MAX_IV_LENGTH`].
    InvalidIvLength,
    /// A zero-record policy was requested.
    InvalidRecordLimit,
    /// The borrowed static IV does not match the policy's exact length.
    StaticIvLengthMismatch,
    /// The caller-owned output does not match the policy's exact length.
    OutputLengthMismatch,
    /// The configured per-key record ceiling has been reached.
    SequenceExhausted,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidIvLength => "invalid TLS record IV length",
            Self::InvalidRecordLimit => "invalid TLS record limit",
            Self::StaticIvLengthMismatch => "TLS static IV length mismatch",
            Self::OutputLengthMismatch => "TLS nonce output length mismatch",
            Self::SequenceExhausted => "TLS record sequence exhausted",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Error {}

/// Caller-selected bounds for one traffic key in one direction.
///
/// `record_limit` is a count, not a final sequence number. A limit of one
/// permits only sequence zero. Even the largest accepted count stops after
/// sequence `u64::MAX - 1`, before advancing would wrap the 64-bit state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoncePolicy {
    iv_length: u8,
    record_limit: u64,
}

impl NoncePolicy {
    /// Validate an exact IV length and a nonzero per-key record ceiling.
    pub fn new(iv_length: usize, record_limit: u64) -> Result<Self, Error> {
        if !(MIN_IV_LENGTH..=MAX_IV_LENGTH).contains(&iv_length) {
            return Err(Error::InvalidIvLength);
        }
        if record_limit == 0 {
            return Err(Error::InvalidRecordLimit);
        }

        Ok(Self {
            // The closed bound above proves this conversion cannot truncate.
            iv_length: iv_length as u8,
            record_limit,
        })
    }

    /// The exact static-IV and output length selected by the caller.
    pub const fn iv_length(self) -> usize {
        self.iv_length as usize
    }

    /// The maximum number of records permitted under the traffic key.
    pub const fn record_limit(self) -> u64 {
        self.record_limit
    }
}

/// Evidence that one sequence value was consumed for a per-record nonce.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReservedNonce {
    sequence_number: u64,
}

impl ReservedNonce {
    /// Return the consumed record sequence number.
    pub const fn sequence_number(self) -> u64 {
        self.sequence_number
    }
}

/// Monotonic nonce state for one TLS 1.3 traffic key and direction.
///
/// The type is intentionally neither `Clone` nor `Copy`: duplicating live
/// state could reserve the same nonce twice. There is intentionally no reset
/// operation; a TLS key update requires a fresh value with a fresh static IV.
#[derive(Debug)]
pub struct RecordNonceSequence {
    policy: NoncePolicy,
    next_sequence: u64,
}

impl RecordNonceSequence {
    /// Create fresh state whose first successful reservation uses sequence 0.
    pub const fn new(policy: NoncePolicy) -> Self {
        Self {
            policy,
            next_sequence: 0,
        }
    }

    /// Return the immutable policy bound to this state.
    pub const fn policy(&self) -> NoncePolicy {
        self.policy
    }

    /// Return how many sequence values have been consumed.
    pub const fn records_used(&self) -> u64 {
        self.next_sequence
    }

    /// Return how many reservations remain before the caller must rekey or
    /// terminate the connection.
    pub const fn remaining_records(&self) -> u64 {
        self.policy.record_limit - self.next_sequence
    }

    /// Reserve the next sequence and derive its nonce into caller-owned output.
    ///
    /// Validation is deliberately ordered before the exhaustion shortcut and
    /// before any mutation. On every error, both `output` and this state are
    /// unchanged. On success, the sequence is consumed before control returns
    /// to the caller, so a later AEAD failure cannot accidentally reuse it.
    pub fn reserve_next_nonce(
        &mut self,
        static_iv: &[u8],
        output: &mut [u8],
    ) -> Result<ReservedNonce, Error> {
        let iv_length = self.policy.iv_length();
        if static_iv.len() != iv_length {
            return Err(Error::StaticIvLengthMismatch);
        }
        if output.len() != iv_length {
            return Err(Error::OutputLengthMismatch);
        }

        let sequence_number = self.next_sequence;
        if sequence_number >= self.policy.record_limit {
            return Err(Error::SequenceExhausted);
        }
        let next_sequence = sequence_number
            .checked_add(1)
            .ok_or(Error::SequenceExhausted)?;

        write_nonce(static_iv, sequence_number, output);
        self.next_sequence = next_sequence;

        Ok(ReservedNonce { sequence_number })
    }
}

/// Apply RFC 8446's padded-network-order XOR after public preflight succeeds.
fn write_nonce(static_iv: &[u8], sequence_number: u64, output: &mut [u8]) {
    output.copy_from_slice(static_iv);

    let encoded_sequence = sequence_number.to_be_bytes();
    let sequence_offset = output.len() - encoded_sequence.len();
    for (destination, sequence_octet) in output[sequence_offset..].iter_mut().zip(encoded_sequence)
    {
        *destination ^= sequence_octet;
    }
}

#[cfg(test)]
mod tests {
    use super::write_nonce;

    #[test]
    fn sequence_is_network_order_and_left_padded() {
        let static_iv = [0u8; 12];
        let mut output = [0xff; 12];

        write_nonce(&static_iv, 0x0102_0304_0506_0708, &mut output);

        assert_eq!(
            output,
            [0, 0, 0, 0, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
    }
}
