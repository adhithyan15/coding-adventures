//! Error types for SWF container reading and parsing.

use std::fmt;

/// Errors that can occur when parsing or decompressing an SWF container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwfError {
    /// File signature is not recognized (expected `FWS`, `CWS`, or `ZWS`).
    InvalidSignature,
    /// SWF version is unsupported (e.g. version 0 or outside supported range).
    UnsupportedVersion,
    /// Stored file length is invalid or inconsistent.
    InvalidFileLength,
    /// Decompression bomb detected (expansion ratio exceeded 50:1 beyond 1 MB).
    DecompressionBombDetected,
    /// Output buffer exceeded the maximum allowed uncompressed byte limit.
    OutputLimitExceeded,
    /// Adler-32 or CRC checksum mismatch in compressed stream.
    ChecksumMismatch,
    /// Unexpected end of input while reading container or tag stream.
    UnexpectedEof,
    /// Malformed tag length or negative long tag length.
    InvalidTagLength,
    /// Invalid bit field or coordinate parameters.
    InvalidBitField,
    /// Corrupted compressed payload or malformed bitstream.
    CorruptedData,
}

impl fmt::Display for SwfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSignature => {
                write!(f, "invalid SWF signature (expected FWS, CWS, or ZWS)")
            }
            Self::UnsupportedVersion => write!(f, "unsupported SWF version"),
            Self::InvalidFileLength => write!(f, "invalid SWF file length header"),
            Self::DecompressionBombDetected => write!(
                f,
                "decompression bomb detected: expansion ratio exceeds 50:1"
            ),
            Self::OutputLimitExceeded => write!(f, "output limit exceeded"),
            Self::ChecksumMismatch => write!(f, "checksum mismatch in compressed SWF container"),
            Self::UnexpectedEof => write!(f, "unexpected end of file reading SWF data"),
            Self::InvalidTagLength => write!(f, "invalid SWF tag length"),
            Self::InvalidBitField => write!(f, "invalid bit field or coordinate parameters"),
            Self::CorruptedData => write!(f, "corrupted compressed stream or malformed payload"),
        }
    }
}

impl std::error::Error for SwfError {}
