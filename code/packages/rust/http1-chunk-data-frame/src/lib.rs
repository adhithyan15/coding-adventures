//! Strict borrowed decoding of one non-terminal HTTP/1 chunk-data frame.
//!
//! The hexadecimal size line is caller-owned. This crate validates only the
//! declared positive-size data bytes and their exact trailing CRLF.

#![forbid(unsafe_code)]

use core::fmt;

/// Caller-owned upper bound for one non-terminal chunk.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1ChunkDataFrameLimits {
    maximum_chunk_bytes: usize,
}

impl Http1ChunkDataFrameLimits {
    /// Construct a positive maximum for non-terminal chunks.
    pub fn new(maximum_chunk_bytes: usize) -> Result<Self, Http1ChunkDataFrameError> {
        if maximum_chunk_bytes == 0 {
            return Err(Http1ChunkDataFrameError::InvalidLimits);
        }
        Ok(Self {
            maximum_chunk_bytes,
        })
    }

    #[must_use]
    pub const fn maximum_chunk_bytes(self) -> usize {
        self.maximum_chunk_bytes
    }
}

/// One validated borrowed non-terminal chunk-data frame.
pub struct Http1ChunkDataFrame<'input> {
    chunk_data: &'input [u8],
    encoded_frame: &'input [u8],
    remainder: &'input [u8],
}

impl<'input> Http1ChunkDataFrame<'input> {
    #[must_use]
    pub const fn chunk_data(&self) -> &'input [u8] {
        self.chunk_data
    }

    #[must_use]
    pub const fn encoded_frame(&self) -> &'input [u8] {
        self.encoded_frame
    }

    #[must_use]
    pub const fn remainder(&self) -> &'input [u8] {
        self.remainder
    }
}

impl fmt::Debug for Http1ChunkDataFrame<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1ChunkDataFrame")
            .field("chunk_bytes", &self.chunk_data.len())
            .field("chunk_data", &"[REDACTED]")
            .field("encoded_frame", &"[REDACTED]")
            .field("remainder", &"[REDACTED]")
            .finish()
    }
}

/// Stable payload-free framing failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ChunkDataFrameError {
    InvalidLimits,
    TerminalChunkRequiresSeparateParsing,
    ChunkTooLarge,
    EncodedFrameLengthOverflow,
    IncompleteChunkData,
    IncompleteChunkTerminator,
    InvalidChunkTerminator,
}

impl fmt::Display for Http1ChunkDataFrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTP chunk-data-frame limits",
            Self::TerminalChunkRequiresSeparateParsing => {
                "terminal HTTP chunk requires separate trailer parsing"
            }
            Self::ChunkTooLarge => "HTTP chunk data exceeds the caller limit",
            Self::EncodedFrameLengthOverflow => "HTTP chunk-data frame length overflows",
            Self::IncompleteChunkData => "incomplete HTTP chunk data",
            Self::IncompleteChunkTerminator => "incomplete HTTP chunk-data terminator",
            Self::InvalidChunkTerminator => "invalid HTTP chunk-data terminator",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1ChunkDataFrameError {}

/// Decode one positive-size chunk-data frame with an exact trailing CRLF.
pub fn decode_http1_chunk_data_frame(
    input: &[u8],
    chunk_size: usize,
    limits: Http1ChunkDataFrameLimits,
) -> Result<Http1ChunkDataFrame<'_>, Http1ChunkDataFrameError> {
    if chunk_size == 0 {
        return Err(Http1ChunkDataFrameError::TerminalChunkRequiresSeparateParsing);
    }
    if chunk_size > limits.maximum_chunk_bytes() {
        return Err(Http1ChunkDataFrameError::ChunkTooLarge);
    }
    let encoded_length = chunk_size
        .checked_add(2)
        .ok_or(Http1ChunkDataFrameError::EncodedFrameLengthOverflow)?;
    if input.len() < chunk_size {
        return Err(Http1ChunkDataFrameError::IncompleteChunkData);
    }
    let terminator = &input[chunk_size..];
    if terminator.is_empty() {
        return Err(Http1ChunkDataFrameError::IncompleteChunkTerminator);
    }
    if terminator[0] != b'\r' {
        return Err(Http1ChunkDataFrameError::InvalidChunkTerminator);
    }
    if terminator.len() == 1 {
        return Err(Http1ChunkDataFrameError::IncompleteChunkTerminator);
    }
    if terminator[1] != b'\n' {
        return Err(Http1ChunkDataFrameError::InvalidChunkTerminator);
    }

    Ok(Http1ChunkDataFrame {
        chunk_data: &input[..chunk_size],
        encoded_frame: &input[..encoded_length],
        remainder: &input[encoded_length..],
    })
}
