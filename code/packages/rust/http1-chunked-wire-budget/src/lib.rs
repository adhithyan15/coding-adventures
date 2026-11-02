//! Cumulative encoded-input policy for a future HTTP/1 chunked decoder.
//!
//! A decoded-body ceiling does not bound chunk-size lines, delimiters, the
//! terminal chunk, or trailer framing. This crate tracks the complete chunked
//! transfer-coding's wire length without seeing bytes, allocating buffers, or
//! performing transport effects.

#![forbid(unsafe_code)]

use core::fmt;

/// Caller-owned wire-byte ceiling for one complete chunked transfer-coding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1ChunkedWireLimit {
    maximum_wire_bytes: usize,
}

impl Http1ChunkedWireLimit {
    /// Retain the exact encoded-input ceiling.
    ///
    /// Zero is meaningful, so construction cannot fail and no integer value is
    /// overloaded to mean "unlimited".
    #[must_use]
    pub const fn new(maximum_wire_bytes: usize) -> Self {
        Self { maximum_wire_bytes }
    }

    #[must_use]
    pub const fn maximum_wire_bytes(self) -> usize {
        self.maximum_wire_bytes
    }
}

/// Consumed-and-replaced cumulative state for one chunked transfer-coding.
///
/// This type intentionally implements neither `Clone` nor `Copy`. Advancing a
/// budget consumes the prior value and prevents accidental direct branching.
/// It is not anti-replay or stream-binding evidence: later composition must
/// own exactly one budget and account every consumed wire octet exactly once.
#[derive(Debug, Eq, PartialEq)]
pub struct Http1ChunkedWireBudget {
    limit: Http1ChunkedWireLimit,
    accounted_wire_bytes: usize,
}

impl Http1ChunkedWireBudget {
    /// Start an empty cumulative budget under the supplied exact limit.
    #[must_use]
    pub const fn new(limit: Http1ChunkedWireLimit) -> Self {
        Self {
            limit,
            accounted_wire_bytes: 0,
        }
    }

    #[must_use]
    pub const fn limit(&self) -> Http1ChunkedWireLimit {
        self.limit
    }

    #[must_use]
    pub const fn accounted_wire_bytes(&self) -> usize {
        self.accounted_wire_bytes
    }

    #[must_use]
    pub const fn remaining_wire_bytes(&self) -> usize {
        self.limit.maximum_wire_bytes - self.accounted_wire_bytes
    }

    /// Account for a strictly positive number of newly consumed wire bytes.
    ///
    /// A later decoder must cap each read or slice by `remaining_wire_bytes`
    /// before obtaining or allocating peer input, then call this method once
    /// for every consumed octet. Post-hoc accounting cannot undo an over-read.
    pub fn advance(self, consumed_wire_bytes: usize) -> Result<Self, Http1ChunkedWireBudgetError> {
        if consumed_wire_bytes == 0 {
            return Err(Http1ChunkedWireBudgetError::ZeroLengthAdvance);
        }

        let next_wire_bytes = self
            .accounted_wire_bytes
            .checked_add(consumed_wire_bytes)
            .ok_or(Http1ChunkedWireBudgetError::WireLengthOverflow)?;
        if next_wire_bytes > self.limit.maximum_wire_bytes {
            return Err(Http1ChunkedWireBudgetError::WireBudgetExceeded);
        }

        Ok(Self {
            limit: self.limit,
            accounted_wire_bytes: next_wire_bytes,
        })
    }
}

/// Stable payload-free transition failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ChunkedWireBudgetError {
    ZeroLengthAdvance,
    WireLengthOverflow,
    WireBudgetExceeded,
}

impl fmt::Display for Http1ChunkedWireBudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroLengthAdvance => "HTTP chunked wire accounting requires a positive length",
            Self::WireLengthOverflow => "HTTP chunked wire length overflows the platform size",
            Self::WireBudgetExceeded => "HTTP chunked transfer exceeds its wire-byte limit",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1ChunkedWireBudgetError {}
