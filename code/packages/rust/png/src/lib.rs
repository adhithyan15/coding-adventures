//! # png — Zero-dependency PNG file format encoder
//!
//! This crate encodes RGBA pixel data as PNG files using only our own
//! `deflate` crate for compression.  No external dependencies.
//!
//! ## PNG file structure
//!
//! A PNG file consists of an 8-byte magic signature followed by a sequence
//! of chunks.  Each chunk has:
//!
//! ```text
//! [4 bytes: data length] [4 bytes: type] [N bytes: data] [4 bytes: CRC-32]
//! ```
//!
//! The minimum valid PNG has three chunks:
//!
//! 1. **IHDR** (image header) — dimensions, color type, bit depth
//! 2. **IDAT** (image data) — zlib-compressed filtered pixel data
//! 3. **IEND** (image end) — empty chunk marking the end
//!
//! ## Pixel filtering
//!
//! Before compression, each row is prepended with a filter byte.  Filtering
//! transforms pixel data to make it more compressible.  We use filter type
//! 0 (None) — the simplest filter that just copies bytes unchanged.  This
//! works well for barcodes and simple graphics with large uniform areas.
//!
//! ## CRC-32
//!
//! Every PNG chunk includes a CRC-32 checksum computed over the chunk type
//! and data bytes.  We implement this using the standard polynomial
//! (0xEDB88320 reflected).

pub const VERSION: &str = "0.1.0";

use std::io::{self, BufWriter, Write};

// ---------------------------------------------------------------------------
// CRC-32 (used by every PNG chunk)
// ---------------------------------------------------------------------------
//
// CRC-32 uses the polynomial 0xEDB88320 (bit-reflected form of 0x04C11DB7).
// We precompute a 256-entry lookup table for byte-at-a-time processing.
//
// The algorithm:
//   1. Initialize CRC to 0xFFFFFFFF
//   2. For each byte: CRC = table[(CRC ^ byte) & 0xFF] ^ (CRC >> 8)
//   3. Final CRC = CRC ^ 0xFFFFFFFF

/// Precomputed CRC-32 lookup table (256 entries).
///
/// Each entry is the CRC-32 of a single byte value (0–255).
/// Generated at compile time using the reflected polynomial 0xEDB88320.
const fn make_crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = 0xEDB88320 ^ (crc >> 1);
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = make_crc_table();

/// Compute CRC-32 of a byte slice.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &byte in data {
        crc = CRC_TABLE[((crc ^ byte as u32) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFFFFFF
}

// ---------------------------------------------------------------------------
// PNG chunk writer
// ---------------------------------------------------------------------------

/// Write a PNG chunk: [length][type][data][crc32].
fn write_chunk(output: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    // Length (4 bytes, big-endian) — does NOT include type or CRC
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());

    // Chunk type (4 bytes)
    output.extend_from_slice(chunk_type);

    // Chunk data
    output.extend_from_slice(data);

    // CRC-32 computed over type + data
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    output.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

// ---------------------------------------------------------------------------
// PNG encoder
// ---------------------------------------------------------------------------

/// PNG magic signature — 8 bytes that identify a file as PNG.
///
/// The bytes have specific meanings:
/// - 0x89: High bit set (detects 7-bit transfer corruption)
/// - PNG: ASCII letters
/// - 0x0D 0x0A: DOS line ending (detects newline conversion)
/// - 0x1A: EOF character (stops DOS `type` command)
/// - 0x0A: Unix line ending (detects newline conversion)
const PNG_MAGIC: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// Encode RGBA pixel data as a PNG file.
///
/// # Arguments
///
/// - `width` — image width in pixels
/// - `height` — image height in pixels
/// - `rgba_data` — pixel data in RGBA format (4 bytes per pixel,
///   row-major, top-left origin).  Must be exactly `width * height * 4` bytes.
///
/// # Returns
///
/// Complete PNG file as a byte vector.
pub fn encode_png_rgba(width: u32, height: u32, rgba_data: &[u8]) -> Vec<u8> {
    assert_eq!(
        rgba_data.len(),
        (width as usize) * (height as usize) * 4,
        "RGBA data length must be width * height * 4"
    );

    let mut output = Vec::new();

    // PNG magic signature
    output.extend_from_slice(&PNG_MAGIC);

    // IHDR chunk — image header (13 bytes)
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());   // Width
    ihdr.extend_from_slice(&height.to_be_bytes());   // Height
    ihdr.push(8);   // Bit depth: 8 bits per channel
    ihdr.push(6);   // Color type: 6 = RGBA (truecolor + alpha)
    ihdr.push(0);   // Compression method: 0 = deflate
    ihdr.push(0);   // Filter method: 0 = adaptive filtering
    ihdr.push(0);   // Interlace method: 0 = no interlace
    write_chunk(&mut output, b"IHDR", &ihdr);

    // Prepare filtered pixel data
    //
    // Each row gets a filter byte prepended.  Filter 0 (None) means
    // "copy bytes unchanged."  The filtered data is then zlib-compressed
    // for the IDAT chunk.
    let row_bytes = (width as usize) * 4;
    let mut filtered = Vec::with_capacity((1 + row_bytes) * height as usize);
    for y in 0..height as usize {
        filtered.push(0); // Filter type 0 (None)
        let row_start = y * row_bytes;
        filtered.extend_from_slice(&rgba_data[row_start..row_start + row_bytes]);
    }

    // IDAT chunk — zlib-compressed filtered pixel data
    let compressed = deflate::zlib_compress(&filtered);
    write_chunk(&mut output, b"IDAT", &compressed);

    // IEND chunk — marks the end of the PNG file (empty data)
    write_chunk(&mut output, b"IEND", &[]);

    output
}

/// Encode RGBA pixel data and write to a file.
///
/// # Safety (path handling)
///
/// The `path` is used directly with `std::fs::File::create`.  The caller
/// is responsible for ensuring the path is safe.
pub fn write_png_rgba(width: u32, height: u32, rgba_data: &[u8], path: &str) -> io::Result<()> {
    let png_data = encode_png_rgba(width, height, rgba_data);
    let file = std::fs::File::create(path)?;
    let mut writer = BufWriter::new(file);
    writer.write_all(&png_data)?;
    writer.flush()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.1.0");
    }

    #[test]
    fn crc32_empty() {
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn crc32_known_value() {
        // CRC-32 of "123456789" is 0xCBF43926 (standard test vector)
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn png_magic_bytes() {
        let data = vec![255, 0, 0, 255]; // 1×1 red pixel
        let png = encode_png_rgba(1, 1, &data);
        assert_eq!(&png[0..8], &PNG_MAGIC);
    }

    #[test]
    fn png_ihdr_chunk() {
        let data = vec![0u8; 4 * 3 * 2]; // 3×2 image
        let png = encode_png_rgba(3, 2, &data);

        // After 8-byte magic, IHDR chunk starts
        // Length should be 13 (big-endian)
        assert_eq!(&png[8..12], &[0, 0, 0, 13]);
        // Type should be "IHDR"
        assert_eq!(&png[12..16], b"IHDR");
        // Width = 3 (big-endian)
        assert_eq!(&png[16..20], &3u32.to_be_bytes());
        // Height = 2 (big-endian)
        assert_eq!(&png[20..24], &2u32.to_be_bytes());
        // Bit depth = 8, Color type = 6 (RGBA)
        assert_eq!(png[24], 8);
        assert_eq!(png[25], 6);
    }

    #[test]
    fn png_ends_with_iend() {
        let data = vec![0u8; 4]; // 1×1 pixel
        let png = encode_png_rgba(1, 1, &data);
        let len = png.len();
        // IEND chunk: length=0, type="IEND", CRC of "IEND"
        assert_eq!(&png[len - 12..len - 8], &[0, 0, 0, 0]); // length = 0
        assert_eq!(&png[len - 8..len - 4], b"IEND");
    }

    /// A 2×2 test image should encode without panicking and produce
    /// a valid PNG structure (magic + IHDR + IDAT + IEND).
    #[test]
    fn encode_2x2_image() {
        let mut data = vec![0u8; 4 * 2 * 2];
        // Red pixel at (0,0)
        data[0] = 255;
        data[3] = 255;
        // Blue pixel at (1,1)
        data[12] = 0;
        data[13] = 0;
        data[14] = 255;
        data[15] = 255;

        let png = encode_png_rgba(2, 2, &data);
        // Should have magic + at least 3 chunks
        assert!(png.len() > 8 + 12 + 13 + 12 + 12);
    }

    #[test]
    #[should_panic(expected = "RGBA data length must be width * height * 4")]
    fn rejects_wrong_data_length() {
        encode_png_rgba(2, 2, &[0u8; 10]);
    }
}
