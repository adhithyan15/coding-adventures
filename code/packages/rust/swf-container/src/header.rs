//! SWF Header parsing and container signature definitions.

use crate::bit_reader::BitReader;
use crate::error::SwfError;
use crate::rect::SwfRect;

/// 3-byte signature array alias.
pub type SignatureBytes = [u8; 3];

/// SWF container compression signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwfSignature {
    /// `FWS`: Uncompressed SWF file.
    Fws,
    /// `CWS`: ZLIB-compressed SWF file (RFC 1950).
    Cws,
    /// `ZWS`: LZMA-compressed SWF file.
    Zws,
}

impl SwfSignature {
    /// Identifies the SWF signature from a 3-byte prefix.
    pub fn from_signature_bytes(bytes: SignatureBytes) -> Result<Self, SwfError> {
        match &bytes {
            b"FWS" => Ok(Self::Fws),
            b"CWS" => Ok(Self::Cws),
            b"ZWS" => Ok(Self::Zws),
            _ => Err(SwfError::InvalidSignature),
        }
    }

    /// 3-byte ASCII representation of this signature.
    pub fn as_signature_bytes(&self) -> SignatureBytes {
        match self {
            Self::Fws => *b"FWS",
            Self::Cws => *b"CWS",
            Self::Zws => *b"ZWS",
        }
    }
}

/// Parsed SWF container header.
#[derive(Debug, Clone, PartialEq)]
pub struct SwfHeader {
    /// Container compression signature.
    pub signature: SwfSignature,
    /// SWF specification version (1..=50).
    pub version: u8,
    /// Untrusted total uncompressed file length declared in header.
    pub file_length: u32,
    /// Stage dimensions in twips.
    pub stage_rect: SwfRect,
    /// Frame rate in 8.8 fixed-point format (raw).
    pub frame_rate_raw: u16,
    /// Total timeline frame count.
    pub frame_count: u16,
}

impl SwfHeader {
    /// Parses an SWF header from the uncompressed prefix of an SWF movie.
    pub fn parse_header(bytes: &[u8]) -> Result<(Self, usize), SwfError> {
        if bytes.len() < 8 {
            return Err(SwfError::UnexpectedEof);
        }

        let sig_bytes: SignatureBytes = [bytes[0], bytes[1], bytes[2]];
        let signature = SwfSignature::from_signature_bytes(sig_bytes)?;
        let version = bytes[3];
        if version == 0 {
            return Err(SwfError::UnsupportedVersion);
        }

        let file_length = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        let mut reader = BitReader::from_bits(&bytes[8..]);
        let stage_rect = reader.read_rect()?;

        let mut pos = 8 + reader.bytes_consumed();
        if pos + 4 > bytes.len() {
            return Err(SwfError::UnexpectedEof);
        }

        let frame_rate_raw = u16::from_le_bytes([bytes[pos], bytes[pos + 1]]);
        let frame_count = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
        pos += 4;

        let header = Self {
            signature,
            version,
            file_length,
            stage_rect,
            frame_rate_raw,
            frame_count,
        };

        Ok((header, pos))
    }

    /// Container signature (`FWS`, `CWS`, or `ZWS`).
    #[inline]
    pub fn header_signature(&self) -> SwfSignature {
        self.signature
    }

    /// SWF version number.
    #[inline]
    pub fn header_version(&self) -> u8 {
        self.version
    }

    /// Declared file length hint.
    #[inline]
    pub fn header_file_length(&self) -> u32 {
        self.file_length
    }

    /// Stage bounding rectangle in twips.
    #[inline]
    pub fn header_stage_rect(&self) -> SwfRect {
        self.stage_rect
    }

    /// Frame rate in frames per second (fps).
    #[inline]
    pub fn header_frame_rate(&self) -> f64 {
        let int_part = (self.frame_rate_raw >> 8) as f64;
        let frac_part = (self.frame_rate_raw & 0xFF) as f64 / 256.0;
        int_part + frac_part
    }

    /// Raw 8.8 fixed-point frame rate word.
    #[inline]
    pub fn header_frame_rate_raw(&self) -> u16 {
        self.frame_rate_raw
    }

    /// Timeline frame count.
    #[inline]
    pub fn header_frame_count(&self) -> u16 {
        self.frame_count
    }
}
