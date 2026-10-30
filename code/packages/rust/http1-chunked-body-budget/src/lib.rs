//! Cumulative policy for a future HTTP/1 chunked-body decoder.
//!
//! A per-chunk ceiling is not an aggregate response ceiling: many individually
//! acceptable chunks can still exceed the caller's body limit. Tiny chunks can
//! also amplify framing work. This crate tracks both dimensions without seeing
//! wire bytes, allocating body storage, or performing transport effects.

#![forbid(unsafe_code)]

use core::fmt;

/// Caller-owned ceilings for one decoded chunked body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1ChunkedBodyLimits {
    maximum_decoded_body_bytes: usize,
    maximum_non_terminal_chunks: usize,
}

impl Http1ChunkedBodyLimits {
    /// Retain exact byte and non-terminal chunk ceilings.
    ///
    /// Zero is meaningful for either ceiling, so construction cannot fail and
    /// no integer value is overloaded to mean "unlimited".
    #[must_use]
    pub const fn new(
        maximum_decoded_body_bytes: usize,
        maximum_non_terminal_chunks: usize,
    ) -> Self {
        Self {
            maximum_decoded_body_bytes,
            maximum_non_terminal_chunks,
        }
    }

    #[must_use]
    pub const fn maximum_decoded_body_bytes(self) -> usize {
        self.maximum_decoded_body_bytes
    }

    #[must_use]
    pub const fn maximum_non_terminal_chunks(self) -> usize {
        self.maximum_non_terminal_chunks
    }
}

/// Consumed-and-replaced cumulative state for one decoded chunked body.
///
/// This type intentionally implements neither `Clone` nor `Copy`. Advancing a
/// budget consumes the prior value and prevents accidental direct branching.
/// It is not anti-replay or stream-binding evidence: a later composition must
/// own exactly one budget for a complete response body and must not reconstruct
/// it before terminal completion.
#[derive(Debug, Eq, PartialEq)]
pub struct Http1ChunkedBodyBudget {
    limits: Http1ChunkedBodyLimits,
    decoded_body_bytes: usize,
    non_terminal_chunk_count: usize,
}

impl Http1ChunkedBodyBudget {
    /// Start an empty cumulative budget under the supplied exact limits.
    #[must_use]
    pub const fn new(limits: Http1ChunkedBodyLimits) -> Self {
        Self {
            limits,
            decoded_body_bytes: 0,
            non_terminal_chunk_count: 0,
        }
    }

    #[must_use]
    pub const fn limits(&self) -> Http1ChunkedBodyLimits {
        self.limits
    }

    #[must_use]
    pub const fn decoded_body_bytes(&self) -> usize {
        self.decoded_body_bytes
    }

    #[must_use]
    pub const fn non_terminal_chunk_count(&self) -> usize {
        self.non_terminal_chunk_count
    }

    #[must_use]
    pub const fn remaining_decoded_body_bytes(&self) -> usize {
        self.limits.maximum_decoded_body_bytes - self.decoded_body_bytes
    }

    #[must_use]
    pub const fn remaining_non_terminal_chunks(&self) -> usize {
        self.limits.maximum_non_terminal_chunks - self.non_terminal_chunk_count
    }

    /// Account for one already validated, strictly positive data-chunk size.
    ///
    /// The chunk-count limit is checked before decoded-length arithmetic. This
    /// makes a count-exhausted state fail without inspecting an attacker-sized
    /// length beyond the required zero/positive distinction.
    pub fn advance(self, chunk_size: usize) -> Result<Self, Http1ChunkedBodyBudgetError> {
        if chunk_size == 0 {
            return Err(Http1ChunkedBodyBudgetError::ZeroSizedDataChunk);
        }

        let next_chunk_count = self
            .non_terminal_chunk_count
            .checked_add(1)
            .ok_or(Http1ChunkedBodyBudgetError::TooManyChunks)?;
        if next_chunk_count > self.limits.maximum_non_terminal_chunks {
            return Err(Http1ChunkedBodyBudgetError::TooManyChunks);
        }

        let next_decoded_body_bytes = self
            .decoded_body_bytes
            .checked_add(chunk_size)
            .ok_or(Http1ChunkedBodyBudgetError::DecodedLengthOverflow)?;
        if next_decoded_body_bytes > self.limits.maximum_decoded_body_bytes {
            return Err(Http1ChunkedBodyBudgetError::DecodedBodyTooLarge);
        }

        Ok(Self {
            limits: self.limits,
            decoded_body_bytes: next_decoded_body_bytes,
            non_terminal_chunk_count: next_chunk_count,
        })
    }
}

/// Stable payload-free transition failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ChunkedBodyBudgetError {
    ZeroSizedDataChunk,
    TooManyChunks,
    DecodedLengthOverflow,
    DecodedBodyTooLarge,
}

impl fmt::Display for Http1ChunkedBodyBudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::ZeroSizedDataChunk => "zero-sized HTTP data chunk is terminal framing",
            Self::TooManyChunks => "HTTP chunked body exceeds its non-terminal chunk limit",
            Self::DecodedLengthOverflow => "HTTP decoded body length overflows the platform size",
            Self::DecodedBodyTooLarge => "HTTP decoded body exceeds its byte limit",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1ChunkedBodyBudgetError {}
