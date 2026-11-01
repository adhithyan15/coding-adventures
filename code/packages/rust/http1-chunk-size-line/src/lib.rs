//! Strict bounded HTTP/1 chunk-size-line decoding.
//!
//! This crate decodes only the hexadecimal size line. It intentionally stops
//! before chunk data, aggregate length, terminal-chunk, and trailer handling.

#![forbid(unsafe_code)]

use core::fmt;

/// Repository ceiling for one chunk-size line, excluding CRLF.
pub const MAX_HTTP1_CHUNK_SIZE_LINE_BYTES: usize = 8 * 1024;

/// Caller-owned line and chunk-size limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Http1ChunkSizeLineLimits {
    maximum_line_bytes: u16,
    maximum_chunk_bytes: usize,
}

impl Http1ChunkSizeLineLimits {
    pub fn new(
        maximum_line_bytes: usize,
        maximum_chunk_bytes: usize,
    ) -> Result<Self, Http1ChunkSizeLineError> {
        if maximum_line_bytes == 0 || maximum_line_bytes > MAX_HTTP1_CHUNK_SIZE_LINE_BYTES {
            return Err(Http1ChunkSizeLineError::InvalidLimits);
        }
        Ok(Self {
            maximum_line_bytes: maximum_line_bytes as u16,
            maximum_chunk_bytes,
        })
    }

    #[must_use]
    pub const fn maximum_line_bytes(self) -> usize {
        self.maximum_line_bytes as usize
    }

    #[must_use]
    pub const fn maximum_chunk_bytes(self) -> usize {
        self.maximum_chunk_bytes
    }
}

/// One validated borrowed chunk-size line and unconsumed input.
pub struct Http1ChunkSizeLine<'input> {
    chunk_size: usize,
    encoded_line: &'input [u8],
    remainder: &'input [u8],
}

impl<'input> Http1ChunkSizeLine<'input> {
    #[must_use]
    pub const fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    #[must_use]
    pub const fn encoded_line(&self) -> &'input [u8] {
        self.encoded_line
    }

    #[must_use]
    pub const fn remainder(&self) -> &'input [u8] {
        self.remainder
    }
}

impl fmt::Debug for Http1ChunkSizeLine<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Http1ChunkSizeLine")
            .field("chunk_size", &self.chunk_size)
            .field("encoded_line", &"[REDACTED]")
            .field("remainder", &"[REDACTED]")
            .finish()
    }
}

/// Stable payload-free decoding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Http1ChunkSizeLineError {
    InvalidLimits,
    IncompleteLine,
    LineTooLong,
    InvalidLineEnding,
    EmptyChunkSize,
    InvalidChunkSize,
    ChunkSizeOverflow,
    ChunkTooLarge,
    ChunkExtensionsUnsupported,
}

impl fmt::Display for Http1ChunkSizeLineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidLimits => "invalid HTTP chunk-size-line limits",
            Self::IncompleteLine => "incomplete HTTP chunk-size line",
            Self::LineTooLong => "HTTP chunk-size line exceeds its byte limit",
            Self::InvalidLineEnding => "invalid HTTP chunk-size line ending",
            Self::EmptyChunkSize => "HTTP chunk size is empty",
            Self::InvalidChunkSize => "invalid HTTP chunk size",
            Self::ChunkSizeOverflow => "HTTP chunk size overflows the platform size",
            Self::ChunkTooLarge => "HTTP chunk size exceeds the caller limit",
            Self::ChunkExtensionsUnsupported => "HTTP chunk extensions are unsupported",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for Http1ChunkSizeLineError {}

/// Decode one exact CRLF-terminated chunk-size line.
pub fn decode_http1_chunk_size_line(
    input: &[u8],
    limits: Http1ChunkSizeLineLimits,
) -> Result<Http1ChunkSizeLine<'_>, Http1ChunkSizeLineError> {
    let mut index = 0usize;
    while index < input.len() && index <= limits.maximum_line_bytes() {
        match input[index] {
            b'\r' => {
                if index + 1 >= input.len() {
                    return Err(Http1ChunkSizeLineError::IncompleteLine);
                }
                if input[index + 1] != b'\n' {
                    return Err(Http1ChunkSizeLineError::InvalidLineEnding);
                }
                let size_bytes = &input[..index];
                if size_bytes.is_empty() {
                    return Err(Http1ChunkSizeLineError::EmptyChunkSize);
                }
                if size_bytes.contains(&b';') {
                    return Err(Http1ChunkSizeLineError::ChunkExtensionsUnsupported);
                }
                let chunk_size = parse_hexadecimal_size(size_bytes)?;
                if chunk_size > limits.maximum_chunk_bytes() {
                    return Err(Http1ChunkSizeLineError::ChunkTooLarge);
                }
                let end = index + 2;
                return Ok(Http1ChunkSizeLine {
                    chunk_size,
                    encoded_line: &input[..end],
                    remainder: &input[end..],
                });
            }
            b'\n' => return Err(Http1ChunkSizeLineError::InvalidLineEnding),
            _ => index += 1,
        }
    }

    if index > limits.maximum_line_bytes() {
        Err(Http1ChunkSizeLineError::LineTooLong)
    } else {
        Err(Http1ChunkSizeLineError::IncompleteLine)
    }
}

fn parse_hexadecimal_size(input: &[u8]) -> Result<usize, Http1ChunkSizeLineError> {
    input.iter().try_fold(0usize, |size, byte| {
        let digit = match byte {
            b'0'..=b'9' => usize::from(*byte - b'0'),
            b'a'..=b'f' => usize::from(*byte - b'a') + 10,
            b'A'..=b'F' => usize::from(*byte - b'A') + 10,
            _ => return Err(Http1ChunkSizeLineError::InvalidChunkSize),
        };
        size.checked_mul(16)
            .and_then(|size| size.checked_add(digit))
            .ok_or(Http1ChunkSizeLineError::ChunkSizeOverflow)
    })
}
