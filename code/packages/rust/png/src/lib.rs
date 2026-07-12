//! # png — Zero-dependency PNG file format encoder/decoder
//!
//! This crate encodes **and decodes** RGBA pixel data as PNG files using only
//! our own `deflate` crate for compression/decompression.  No external dependencies.
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

pub const VERSION: &str = "0.2.0";

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
// PNG decoder
// ---------------------------------------------------------------------------
//
// Overview of decoding steps (mirrors RFC 2083):
//
//   1. Verify the 8-byte magic signature.
//   2. Parse chunks one by one until IEND:
//        [length u32 BE][type 4 bytes][data][CRC-32 u32 BE]
//      The CRC covers `type ++ data` (not `length`).
//   3. IHDR (first chunk, must be 13 bytes) — read image dimensions,
//      bit depth, color type, and reject unsupported modes.
//   4. IDAT — accumulate all IDAT chunk data into a single Vec.
//   5. Decompress the IDAT payload with `deflate::zlib_decompress`.
//   6. Unfilter each scanline.  A scanline is:
//        [filter_byte: u8][pixel_data: width × bpp bytes]
//      where bpp = 3 (RGB) or 4 (RGBA).  Five filter types (0–4) are
//      defined by RFC 2083 §6.  All arithmetic is modulo 256.
//   7. If color_type == 2 (RGB), expand each pixel to RGBA by inserting A=255.
//
// Only 8-bit-per-channel, non-interlaced, RGB or RGBA images are supported —
// the encoder in this crate only produces color_type 6 (RGBA), so the decoder
// handles that path first, then adds RGB (color_type 2) for interop.

// ── Paeth predictor (RFC 2083 §6.6) ────────────────────────────────────────
//
// The Paeth predictor chooses whichever of three neighbours — left (a),
// above (b), or upper-left (c) — is numerically closest to the linear
// predictor p = a + b - c.  This frequently gives a smaller residual than
// the simpler Up or Sub predictors.
//
// Visual layout (one row at a time):
//
//   c  b          ← previous row (above)
//   a  X          ← current row  (X = current byte being predicted)
//
// All values are treated as unsigned 8-bit integers (0–255), but arithmetic
// is done in i32 to avoid overflow.

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let a = a as i32;
    let b = b as i32;
    let c = c as i32;
    // Linear predictor: the value you'd get if the surface were truly flat.
    let p = a + b - c;
    let pa = (p - a).abs(); // distance to left neighbour
    let pb = (p - b).abs(); // distance to above neighbour
    let pc = (p - c).abs(); // distance to upper-left neighbour
    // Pick the closest neighbour as the predictor.
    if pa <= pb && pa <= pc {
        a as u8
    } else if pb <= pc {
        b as u8
    } else {
        c as u8
    }
}

/// Decode a PNG file and return its pixel data as RGBA8.
///
/// # Arguments
///
/// - `data` — raw PNG bytes (e.g., the contents of a `.png` file).
///
/// # Returns
///
/// `Ok((width, height, rgba_bytes))` on success, where `rgba_bytes` is
/// row-major RGBA8 (4 bytes per pixel, top-left origin).
///
/// # Errors
///
/// Returns `Err(String)` for any of the following:
///
/// - Invalid magic signature
/// - CRC mismatch on any chunk
/// - Unsupported bit depth, color type, or interlace method
/// - Zlib decompression failure
/// - Corrupt or truncated scanline data
///
/// # Supported image types
///
/// | Color type | Value | Description        |
/// |------------|-------|--------------------|
/// | RGB        |   2   | 3 bytes/pixel       |
/// | RGBA       |   6   | 4 bytes/pixel       |
///
/// Both types are decoded to RGBA8 (Alpha = 255 for RGB).
/// Bit depth must be 8.  Interlacing (Adam7) is not supported.
pub fn decode_png_rgba(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    // ── Step 1: Magic signature ─────────────────────────────────────────────
    //
    // Every PNG file begins with exactly these 8 bytes.  Any other value
    // means this is not (or is a corrupted) PNG.
    if data.len() < 8 || data[0..8] != PNG_MAGIC {
        return Err("PNG decode: invalid magic signature".to_string());
    }

    // ── Step 2: Chunk parsing loop ──────────────────────────────────────────
    //
    // After the magic, the file is a sequence of chunks.  Each chunk has:
    //
    //   [length: u32 BE]  — byte count of the DATA field only
    //   [type:   4 bytes] — ASCII name, e.g. b"IHDR"
    //   [data:   N bytes] — chunk payload (N = length)
    //   [crc:   u32 BE]  — CRC-32 of type ++ data (NOT length)
    //
    // Minimum chunk size = 12 bytes (0-byte data, no length/crc waste).

    let mut pos = 8usize; // skip magic
    let mut width = 0u32;
    let mut height = 0u32;
    let mut color_type = 0u8;
    let mut ihdr_seen = false;
    let mut idat_data: Vec<u8> = Vec::new();

    loop {
        // Need at least 12 bytes for an empty chunk.
        if pos + 12 > data.len() {
            return Err("PNG decode: truncated chunk header".to_string());
        }

        // Parse the 4-byte big-endian length.
        let chunk_len = u32::from_be_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
        ]) as usize;
        pos += 4;

        // The chunk type is 4 ASCII bytes (not NUL-terminated).
        let chunk_type = &data[pos..pos + 4];
        let chunk_type_str = std::str::from_utf8(chunk_type)
            .unwrap_or("<invalid UTF-8>");

        // Make sure the chunk body + CRC are actually present.
        if pos + 4 + chunk_len + 4 > data.len() {
            return Err(format!(
                "PNG decode: chunk '{}' extends beyond end of file",
                chunk_type_str
            ));
        }

        let chunk_data = &data[pos + 4..pos + 4 + chunk_len];

        // CRC-32 is computed over [type bytes] ++ [data bytes].
        // The length field is intentionally excluded.
        let crc_input = &data[pos..pos + 4 + chunk_len];
        let computed_crc = crc32(crc_input);
        let stored_crc = u32::from_be_bytes([
            data[pos + 4 + chunk_len],
            data[pos + 4 + chunk_len + 1],
            data[pos + 4 + chunk_len + 2],
            data[pos + 4 + chunk_len + 3],
        ]);
        if computed_crc != stored_crc {
            return Err(format!(
                "PNG decode: CRC mismatch in {} chunk",
                chunk_type_str
            ));
        }

        // Advance past [type][data][crc].
        pos += 4 + chunk_len + 4;

        // ── Step 3: IHDR ────────────────────────────────────────────────────
        //
        // The Image Header chunk MUST be the very first chunk after the magic.
        // It is always exactly 13 bytes.
        //
        //   Byte  0– 3: width  (u32 BE)
        //   Byte  4– 7: height (u32 BE)
        //   Byte  8:    bit depth
        //   Byte  9:    color type
        //   Byte 10:    compression method (must be 0)
        //   Byte 11:    filter method      (must be 0)
        //   Byte 12:    interlace method   (0 = none, 1 = Adam7)

        if chunk_type == b"IHDR" {
            if ihdr_seen {
                return Err("PNG decode: duplicate IHDR chunk".to_string());
            }
            ihdr_seen = true;

            if chunk_data.len() != 13 {
                return Err(format!(
                    "PNG decode: IHDR has {} bytes (expected 13)",
                    chunk_data.len()
                ));
            }

            width  = u32::from_be_bytes([chunk_data[0], chunk_data[1], chunk_data[2], chunk_data[3]]);
            height = u32::from_be_bytes([chunk_data[4], chunk_data[5], chunk_data[6], chunk_data[7]]);

            if width == 0 || height == 0 {
                return Err("PNG decode: zero dimensions".to_string());
            }

            let bit_depth   = chunk_data[8];
            color_type      = chunk_data[9];
            // compression method (byte 10) is always 0 for standard PNG
            // filter method (byte 11) is always 0 for standard PNG
            let interlace   = chunk_data[12];

            if bit_depth != 8 {
                return Err(format!(
                    "PNG decode: unsupported bit depth {} (only 8 supported)",
                    bit_depth
                ));
            }

            if color_type != 2 && color_type != 6 {
                return Err(format!(
                    "PNG decode: unsupported color type {} (only RGB=2 and RGBA=6 supported)",
                    color_type
                ));
            }

            if interlace != 0 {
                return Err(format!(
                    "PNG decode: unsupported interlace method {} (only non-interlaced supported)",
                    interlace
                ));
            }
        }

        // ── Step 4: IDAT accumulation ────────────────────────────────────────
        //
        // A PNG encoder may split the compressed pixel data across multiple
        // IDAT chunks.  This is legal per RFC 2083 §4.1.3 and is used for
        // streaming output.  We accumulate all IDAT payloads into a single
        // contiguous buffer before decompressing.
        else if chunk_type == b"IDAT" {
            if !ihdr_seen {
                return Err("PNG decode: IDAT before IHDR".to_string());
            }
            idat_data.extend_from_slice(chunk_data);
        }

        // ── IEND — end of image ──────────────────────────────────────────────
        else if chunk_type == b"IEND" {
            break; // stop parsing; everything that matters has been read
        }
        // Any other chunk type (tEXt, gAMA, sRGB, etc.) is silently skipped.
        // This is spec-compliant: ancillary chunks are optional and may be ignored.
    }

    if !ihdr_seen {
        return Err("PNG decode: missing IHDR chunk".to_string());
    }
    if idat_data.is_empty() {
        return Err("PNG decode: missing IDAT chunk".to_string());
    }

    // ── Step 5: Decompression ────────────────────────────────────────────────
    //
    // The IDAT payload is a single zlib stream (RFC 1950) wrapping a DEFLATE
    // stream (RFC 1951).  `deflate::zlib_decompress` handles both layers and
    // verifies the Adler-32 checksum.
    let raw_filtered = deflate::zlib_decompress(&idat_data)
        .map_err(|e| format!("PNG decode: zlib error: {}", e))?;

    // ── Step 6: Unfiltering scanlines ────────────────────────────────────────
    //
    // After decompression the byte sequence is:
    //
    //   For each row (top to bottom):
    //     [filter_byte: u8]              — selects the reconstruction function
    //     [pixel_data: width × bpp bytes] — filtered pixel bytes
    //
    // bpp = bytes per pixel = 3 (RGB) or 4 (RGBA).
    //
    // Reconstruction reverses the filter to produce the original pixel values.
    // All arithmetic is modulo 256 (wrapping_add).
    //
    // Reference values for filter types 1–4:
    //   a = the byte bpp positions to the LEFT in the current row (0 if none)
    //   b = the corresponding byte in the PREVIOUS row (0 for row 0)
    //   c = the byte bpp positions to the LEFT in the PREVIOUS row (0 if none)

    let bpp: usize = match color_type {
        2 => 3, // RGB
        6 => 4, // RGBA
        _ => unreachable!(), // already validated above
    };

    let stride = 1 + (width as usize) * bpp; // filter byte + pixel data
    let expected_len = (height as usize) * stride;

    if raw_filtered.len() != expected_len {
        return Err(format!(
            "PNG decode: decompressed size {} does not match expected {} ({}×{}×{}+{})",
            raw_filtered.len(),
            expected_len,
            height, width, bpp, height
        ));
    }

    // Allocate output buffer for the unfiltered pixel data.
    // Each unfiltered row is exactly `width * bpp` bytes (no filter byte).
    let row_bytes = (width as usize) * bpp;
    let mut pixels: Vec<u8> = vec![0u8; (height as usize) * row_bytes];

    for row_idx in 0..(height as usize) {
        let filter_byte   = raw_filtered[row_idx * stride];
        let raw_row_start = row_idx * stride + 1;
        let out_row_start = row_idx * row_bytes;

        // Helper: offset into the PREVIOUS row for index `i` within row pixels.
        // Returns 0 when row_idx == 0 (no row above).
        // We cannot use a closure here because Rust's borrow checker would see both
        // a shared borrow (for the prev-row read) and an exclusive borrow (for the
        // current-row write) active at the same time on the same Vec.  Instead we
        // compute the previous-row index inline wherever we need it.

        match filter_byte {
            // ── Filter 0: None ───────────────────────────────────────────────
            // No transformation; the raw bytes are the original pixel bytes.
            0 => {
                pixels[out_row_start..out_row_start + row_bytes]
                    .copy_from_slice(&raw_filtered[raw_row_start..raw_row_start + row_bytes]);
            }

            // ── Filter 1: Sub ────────────────────────────────────────────────
            // Each byte is the difference from the byte bpp positions to the left.
            // Reconstruction: out[i] = raw[i] + out[i - bpp]
            1 => {
                for i in 0..row_bytes {
                    let a = if i >= bpp { pixels[out_row_start + i - bpp] } else { 0 };
                    pixels[out_row_start + i] = raw_filtered[raw_row_start + i].wrapping_add(a);
                }
            }

            // ── Filter 2: Up ─────────────────────────────────────────────────
            // Each byte is the difference from the byte directly above.
            // Reconstruction: out[i] = raw[i] + prev_row[i]
            2 => {
                for i in 0..row_bytes {
                    let b = if row_idx > 0 { pixels[(row_idx - 1) * row_bytes + i] } else { 0 };
                    pixels[out_row_start + i] = raw_filtered[raw_row_start + i].wrapping_add(b);
                }
            }

            // ── Filter 3: Average ────────────────────────────────────────────
            // Uses the floor of the average of the left and above bytes.
            // Reconstruction: out[i] = raw[i] + floor((a + b) / 2)
            3 => {
                for i in 0..row_bytes {
                    let a = if i >= bpp { pixels[out_row_start + i - bpp] } else { 0 };
                    let b = if row_idx > 0 { pixels[(row_idx - 1) * row_bytes + i] } else { 0 };
                    let avg = ((a as u16 + b as u16) / 2) as u8;
                    pixels[out_row_start + i] = raw_filtered[raw_row_start + i].wrapping_add(avg);
                }
            }

            // ── Filter 4: Paeth ──────────────────────────────────────────────
            // Uses the Paeth predictor — picks the closest of left, above, upper-left.
            // Reconstruction: out[i] = raw[i] + paeth_predictor(a, b, c)
            4 => {
                for i in 0..row_bytes {
                    let a = if i >= bpp { pixels[out_row_start + i - bpp] } else { 0 };
                    let b = if row_idx > 0 { pixels[(row_idx - 1) * row_bytes + i] } else { 0 };
                    let c = if row_idx > 0 && i >= bpp { pixels[(row_idx - 1) * row_bytes + i - bpp] } else { 0 };
                    pixels[out_row_start + i] = raw_filtered[raw_row_start + i].wrapping_add(paeth_predictor(a, b, c));
                }
            }

            unknown => {
                return Err(format!("PNG decode: unknown filter type {}", unknown));
            }
        }
    }

    // ── Step 7: RGB → RGBA expansion ────────────────────────────────────────
    //
    // The caller always wants RGBA8 (4 bytes per pixel).  If the PNG stores
    // RGB (color_type 2), we insert an opaque alpha byte (255) after each
    // three-byte pixel.  If it's already RGBA, we return pixels as-is.

    let rgba = if color_type == 6 {
        // Already RGBA — no transformation needed.
        pixels
    } else {
        // color_type == 2: RGB → RGBA
        // For each pixel, copy R, G, B from the 3-byte pixel and append A=255.
        let total_pixels = (width as usize) * (height as usize);
        let mut rgba = Vec::with_capacity(total_pixels * 4);
        for px in 0..total_pixels {
            rgba.push(pixels[px * 3]);     // R
            rgba.push(pixels[px * 3 + 1]); // G
            rgba.push(pixels[px * 3 + 2]); // B
            rgba.push(255);                 // A = fully opaque
        }
        rgba
    };

    Ok((width, height, rgba))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_exists() {
        assert_eq!(VERSION, "0.2.0");
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

    // ── decode_png_rgba tests ────────────────────────────────────────────────

    /// Encode a 4×4 RGBA image with known pixel values, decode it, and verify
    /// that every pixel channel is reconstructed exactly.
    ///
    /// This is the primary round-trip test: it exercises the full
    /// encode→filter(None)→zlib_compress→zlib_decompress→unfilter→expand pipeline.
    #[test]
    fn decode_roundtrip_rgba() {
        let width  = 4u32;
        let height = 4u32;
        // Build a 4×4 RGBA image with distinct pixel values so any byte-swap
        // or offset bug would be caught.
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.push((x * 40) as u8);       // R — varies across columns
                pixels.push((y * 60) as u8);        // G — varies across rows
                pixels.push(((x + y) * 20) as u8); // B — diagonal gradient
                pixels.push(255);                   // A — fully opaque
            }
        }

        let encoded = encode_png_rgba(width, height, &pixels);
        let (dec_w, dec_h, dec_pixels) = decode_png_rgba(&encoded)
            .expect("decode_roundtrip_rgba: decode failed");

        assert_eq!(dec_w, width,  "width mismatch");
        assert_eq!(dec_h, height, "height mismatch");
        assert_eq!(dec_pixels, pixels, "pixel data mismatch");
    }

    /// Encode and decode a 1×1 single-pixel image.
    ///
    /// Edge case: smallest possible valid PNG.  Verifies dimensions and the
    /// single pixel value are preserved.
    #[test]
    fn decode_roundtrip_1x1() {
        let pixels = vec![0xDE, 0xAD, 0xBE, 0xFF]; // one RGBA pixel
        let encoded = encode_png_rgba(1, 1, &pixels);
        let (w, h, dec) = decode_png_rgba(&encoded)
            .expect("decode_roundtrip_1x1: decode failed");

        assert_eq!(w, 1);
        assert_eq!(h, 1);
        assert_eq!(dec, pixels);
    }

    /// Passing random bytes that don't start with the PNG magic must return
    /// an error whose message contains "magic".
    #[test]
    fn decode_invalid_magic() {
        let err = decode_png_rgba(b"not a png").unwrap_err();
        assert!(
            err.contains("magic"),
            "expected 'magic' in error, got: {}",
            err
        );
    }

    /// An empty input slice must return an error.
    #[test]
    fn decode_empty_input() {
        let err = decode_png_rgba(&[]).unwrap_err();
        assert!(!err.is_empty(), "expected a non-empty error message");
    }

    /// Encode a 50×50 gradient image and decode it.  Verifies no panics occur
    /// for a moderately large image and that the decoded pixel count is correct.
    #[test]
    fn decode_large_image() {
        let width  = 50u32;
        let height = 50u32;
        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
        for y in 0..height {
            for x in 0..width {
                pixels.push((x * 5) as u8);
                pixels.push((y * 5) as u8);
                pixels.push(128);
                pixels.push(255);
            }
        }

        let encoded = encode_png_rgba(width, height, &pixels);
        let (dec_w, dec_h, dec_pixels) = decode_png_rgba(&encoded)
            .expect("decode_large_image: decode failed");

        assert_eq!(dec_w, width);
        assert_eq!(dec_h, height);
        assert_eq!(
            dec_pixels.len(),
            (width * height * 4) as usize,
            "decoded pixel count mismatch"
        );
        // Full data equality — the encoder uses filter 0 (None), so every byte
        // must survive the round-trip intact.
        assert_eq!(dec_pixels, pixels);
    }

    /// The encoder uses filter 0 (None) for all rows.  Confirm that the
    /// decoded data matches the original pixel-for-pixel, which implicitly
    /// proves the Filter 0 reconstruction path is correct.
    #[test]
    fn decode_filter_none() {
        let width  = 3u32;
        let height = 3u32;
        // 9 distinct pixels so any ordering error is obvious.
        let pixels: Vec<u8> = (0..(width * height * 4) as u8).collect();
        let encoded = encode_png_rgba(width, height, &pixels);
        let (_, _, dec) = decode_png_rgba(&encoded)
            .expect("decode_filter_none: decode failed");
        assert_eq!(dec, pixels, "Filter 0 round-trip failed");
    }

    /// Corrupt the CRC of the first IDAT chunk and confirm that the decoder
    /// returns an error containing "CRC".
    ///
    /// Strategy: encode a valid PNG, then scan for the first "IDAT" marker and
    /// flip a byte in the four CRC bytes that follow its payload.
    #[test]
    fn decode_bad_crc() {
        let pixels: Vec<u8> = vec![100, 150, 200, 255, 50, 75, 100, 255, 200, 100, 50, 255, 0, 128, 255, 255];
        let mut encoded = encode_png_rgba(2, 2, &pixels);

        // Find the first IDAT chunk.  Each chunk is:
        //   [4 bytes length][4 bytes type][N bytes data][4 bytes CRC]
        // We scan for b"IDAT" starting after the 8-byte magic.
        let mut idat_pos = None;
        let mut scan = 8usize;
        while scan + 12 <= encoded.len() {
            let chunk_len = u32::from_be_bytes([
                encoded[scan], encoded[scan+1], encoded[scan+2], encoded[scan+3]
            ]) as usize;
            if &encoded[scan+4..scan+8] == b"IDAT" {
                // CRC starts at scan + 4 (type) + chunk_len (data) + 4 (length field) = scan + 8 + chunk_len.
                idat_pos = Some(scan + 8 + chunk_len);
                break;
            }
            scan += 4 + 4 + chunk_len + 4;
        }

        let crc_start = idat_pos.expect("no IDAT chunk found in encoded PNG");
        // Flip the first byte of the CRC to corrupt it.
        encoded[crc_start] ^= 0xFF;

        let err = decode_png_rgba(&encoded).unwrap_err();
        assert!(
            err.contains("CRC"),
            "expected 'CRC' in error, got: {}",
            err
        );
    }
}
