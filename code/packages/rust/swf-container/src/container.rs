//! High-level SWF container reader, movie structure, and streaming interface.

use crate::container_config::SwfDecompressorConfig;
use crate::deflate::zlib_decompress_bounded;
use crate::error::SwfError;
use crate::header::{SwfHeader, SwfSignature};
use crate::lzma::{lzma_decompress_bounded, ZwsConfig};
use crate::tag::SwfTagIter;

/// An uncompressed, validated SWF movie containing parsed header and body tag bytes.
#[derive(Debug, Clone, PartialEq)]
pub struct SwfMovie {
    header: SwfHeader,
    body: Vec<u8>,
}

impl SwfMovie {
    /// Loads and parses an SWF movie from binary bytes, decompressing if necessary.
    pub fn from_swf_bytes(data: &[u8], config: &SwfDecompressorConfig) -> Result<Self, SwfError> {
        if data.len() < 8 {
            return Err(SwfError::UnexpectedEof);
        }

        let sig = SwfSignature::from_signature_bytes([data[0], data[1], data[2]])?;

        match sig {
            SwfSignature::Fws => {
                if data.len() > config.max_uncompressed_bytes {
                    return Err(SwfError::OutputLimitExceeded);
                }
                let (header, tag_start) = SwfHeader::parse_header(data)?;
                let body = data[tag_start..].to_vec();
                Ok(Self { header, body })
            }
            SwfSignature::Cws => {
                let version = data[3];
                let file_length = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                let zlib_payload = &data[8..];

                let decompressed = zlib_decompress_bounded(
                    zlib_payload,
                    config.max_uncompressed_bytes,
                    config.max_expansion_ratio,
                )?;

                // Combine 8-byte container header with decompressed body for standard parsing
                let mut uncompressed = Vec::with_capacity(8 + decompressed.len());
                uncompressed.extend_from_slice(b"FWS");
                uncompressed.push(version);
                uncompressed.extend_from_slice(&file_length.to_le_bytes());
                uncompressed.extend_from_slice(&decompressed);

                let (mut header, tag_start) = SwfHeader::parse_header(&uncompressed)?;
                header.signature = SwfSignature::Cws;
                let body = uncompressed[tag_start..].to_vec();

                Ok(Self { header, body })
            }
            SwfSignature::Zws => {
                let zws_config = ZwsConfig::parse_zws_header(data)?;
                let expected_uncompressed_body =
                    (zws_config.uncompressed_file_length.saturating_sub(8)) as usize;

                let lzma_payload = &data[17..];
                let decompressed = lzma_decompress_bounded(
                    lzma_payload,
                    expected_uncompressed_body,
                    &zws_config,
                    config.max_uncompressed_bytes,
                    config.max_expansion_ratio,
                )?;

                let mut uncompressed = Vec::with_capacity(8 + decompressed.len());
                uncompressed.extend_from_slice(b"FWS");
                uncompressed.push(zws_config.version);
                uncompressed.extend_from_slice(&zws_config.uncompressed_file_length.to_le_bytes());
                uncompressed.extend_from_slice(&decompressed);

                let (mut header, tag_start) = SwfHeader::parse_header(&uncompressed)?;
                header.signature = SwfSignature::Zws;
                let body = uncompressed[tag_start..].to_vec();

                Ok(Self { header, body })
            }
        }
    }

    /// Header of the SWF movie.
    #[inline]
    pub fn movie_header(&self) -> &SwfHeader {
        &self.header
    }

    /// Streaming iterator yielding tags without heap allocations.
    #[inline]
    pub fn movie_tags<'a>(&'a self) -> SwfTagIter<'a> {
        SwfTagIter::from_payload(&self.body)
    }

    /// Uncompressed body payload bytes containing the tag sequence.
    #[inline]
    pub fn movie_body_bytes(&self) -> &[u8] {
        &self.body
    }
}
