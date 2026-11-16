//! # SWF Container
//!
//! Deterministic, zero-dependency binary SWF container reader, streaming tag
//! parser, and anti-bomb decompressor (`SWF01`).
//!
//! Provides:
//! - Container signature recognition (`FWS` uncompressed, `CWS` zlib, `ZWS` LZMA).
//! - SWF header parsing (dimensions in twips, 8.8 fixed-point frame rate, frame count).
//! - Strict anti-bomb defenses: hard output limit (default 256 MB) and 50:1 expansion ratio cap beyond 1 MB.
//! - MSB-first `BitReader` for TWIPS `RECT`, `MATRIX`, and `CXFORM`.
//! - Zero-allocation streaming `SwfTagIter` over uncompressed tag streams.

#![deny(unsafe_code)]

pub mod bit_reader;
pub mod capabilities;
pub mod container;
pub mod container_config;
pub mod cxform;
pub mod deflate;
pub mod error;
pub mod header;
pub mod lzma;
pub mod matrix;
pub mod rect;
pub mod tag;

pub use bit_reader::BitReader;
pub use capabilities::PureCapabilities;
pub use container::SwfMovie;
pub use container_config::{
    SwfDecompressorConfig, DEFAULT_MAX_EXPANSION_RATIO, DEFAULT_MAX_UNCOMPRESSED_BYTES,
    EXPANSION_RATIO_THRESHOLD_BYTES,
};
pub use cxform::{RgbaArray, SwfColorTransform};
pub use deflate::{compute_adler32, zlib_decompress_bounded};
pub use error::SwfError;
pub use header::{SignatureBytes, SwfHeader, SwfSignature};
pub use lzma::{lzma_decompress_bounded, LzmaProperties, ZwsConfig};
pub use matrix::SwfMatrix;
pub use rect::SwfRect;
pub use tag::{SwfTag, SwfTagHeader, SwfTagIter, SwfTagKind};

/// Validates that an SWF version is within supported bounds (1..=50).
#[inline]
pub fn validate_version(v: usize) -> Result<bool, SwfError> {
    if (1..=50).contains(&v) {
        Ok(true)
    } else {
        Err(SwfError::UnsupportedVersion)
    }
}

/// Validates that an SWF tag code is within valid 10-bit range (0..=1023).
#[inline]
pub fn validate_tag_code(c: usize) -> Result<bool, SwfError> {
    if c <= 1023 {
        Ok(true)
    } else {
        Err(SwfError::InvalidTagLength)
    }
}

/// Validates that a decompression expansion ratio conforms to security limits.
#[inline]
pub fn validate_ratio(bytes_out: usize, bytes_in: usize) -> Result<bool, SwfError> {
    if bytes_out <= EXPANSION_RATIO_THRESHOLD_BYTES || bytes_out <= bytes_in.saturating_mul(50) {
        Ok(true)
    } else {
        Err(SwfError::DecompressionBombDetected)
    }
}
