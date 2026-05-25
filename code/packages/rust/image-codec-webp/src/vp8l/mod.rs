//! VP8L lossless WebP encode and decode.
//!
//! This module implements the VP8L bitstream format:
//!
//! ```text
//! [signature byte 0x2F]
//! [32-bit header: width-1(14b), height-1(14b), alpha_is_used(1b), version(3b)]
//! [has_transform: 1 bit = 0 (no transforms in v0.1)]
//! [color_cache_code_bits: 4 bits = 0 (no color cache)]
//! [5 Huffman group tables: G, R, B, A, Dist]
//! [pixel data: one G-symbol per pixel, plus R/B/A symbols]
//! ```
//!
//! ## Encoding strategy (v0.1)
//!
//! This release uses **literal-only** encoding:
//! - No transforms (subtract-green, predictor, colour, colour-index).
//! - No LZ77 back-references.
//! - No colour cache.
//!
//! Each pixel is encoded as four Huffman-coded values: green from the G tree,
//! red from the R tree, blue from the B tree, alpha from the A tree.
//! The Dist tree is always a trivial 1-symbol code (distance 0) because no
//! back-references are used.
//!
//! ## Decoding
//!
//! The decoder supports:
//! - Simple 1-symbol and 2-symbol codes.
//! - Complex codes (meta-Huffman format).
//! - Literal pixel decoding (G symbol < 256).
//! - Returns an error on LZ77 symbols (G symbol ≥ 256).

pub mod bitstream;
pub mod huffman;
pub mod lz77;
pub mod transforms;

use bitstream::{BitReader, BitWriter};
use huffman::{
    build_encode_table, lengths_from_frequencies, read_huffman_code, write_huffman_code,
    DIST_ALPHABET_SIZE, G_ALPHABET_SIZE, RGBA_ALPHABET_SIZE,
};
use pixel_container::PixelContainer;

// ---------------------------------------------------------------------------
// VP8L encode
// ---------------------------------------------------------------------------

/// Encode a `PixelContainer` as a VP8L lossless bitstream (without the RIFF
/// wrapper — the caller in `lib.rs`/`riff.rs` adds that).
///
/// The result starts with the VP8L signature byte `0x2F`, followed by the
/// bit-packed bitstream.
pub fn encode(pixels: &PixelContainer) -> Vec<u8> {
    let w = pixels.width as u64;
    let h = pixels.height as u64;

    // ── Step 1: Collect pixel channel frequencies ────────────────────────────
    //
    // PixelContainer stores pixels as [R, G, B, A, R, G, B, A, ...] row-major.

    let mut g_freq = vec![0u32; G_ALPHABET_SIZE];   // green channel literals
    let mut r_freq = vec![0u32; RGBA_ALPHABET_SIZE]; // red channel
    let mut b_freq = vec![0u32; RGBA_ALPHABET_SIZE]; // blue channel
    let mut a_freq = vec![0u32; RGBA_ALPHABET_SIZE]; // alpha channel
    let mut d_freq = vec![0u32; DIST_ALPHABET_SIZE]; // distance (unused)

    for chunk in pixels.data.chunks_exact(4) {
        g_freq[chunk[1] as usize] += 1; // G (green)
        r_freq[chunk[0] as usize] += 1; // R (red)
        b_freq[chunk[2] as usize] += 1; // B (blue)
        a_freq[chunk[3] as usize] += 1; // A (alpha)
    }

    // Dist tree: always trivial (no back-references in v0.1).
    d_freq[0] = 1;

    // ── Step 2: Compute canonical code lengths ───────────────────────────────

    let g_lens = lengths_from_frequencies(&g_freq);
    let r_lens = lengths_from_frequencies(&r_freq);
    let b_lens = lengths_from_frequencies(&b_freq);
    let a_lens = lengths_from_frequencies(&a_freq);
    let d_lens = lengths_from_frequencies(&d_freq);

    // Build the encode tables (symbol → (bit_reversed_code, code_len)).
    // For symbols with length 0 (trivial code), encoding emits 0 bits.
    let g_encode = build_encode_table(&g_lens);
    let r_encode = build_encode_table(&r_lens);
    let b_encode = build_encode_table(&b_lens);
    let a_encode = build_encode_table(&a_lens);

    // ── Step 3: Write bitstream ──────────────────────────────────────────────

    let mut bw = BitWriter::new();

    // Header: (width-1) in 14 bits, (height-1) in 14 bits, alpha=1, version=0.
    bw.write_bits(w - 1, 14);
    bw.write_bits(h - 1, 14);
    bw.write_bits(1, 1); // alpha_is_used = 1
    bw.write_bits(0, 3); // version_number = 0

    // No transforms.
    bw.write_bits(0, 1); // has_transform = 0

    // Color cache code bits = 0 (no color cache).
    bw.write_bits(0, 4);

    // Write Huffman code tables for the 5 groups (G, R, B, A, Dist).
    write_huffman_code(&mut bw, &g_lens);
    write_huffman_code(&mut bw, &r_lens);
    write_huffman_code(&mut bw, &b_lens);
    write_huffman_code(&mut bw, &a_lens);
    write_huffman_code(&mut bw, &d_lens);

    // Write pixel data.
    // For each pixel: emit G (green), then R, B, A.
    // Trivial codes (length 0) emit 0 bits — correct VP8L behaviour.
    for chunk in pixels.data.chunks_exact(4) {
        emit_symbol(&mut bw, chunk[1] as usize, &g_encode); // G (green)
        emit_symbol(&mut bw, chunk[0] as usize, &r_encode); // R (red)
        emit_symbol(&mut bw, chunk[2] as usize, &b_encode); // B (blue)
        emit_symbol(&mut bw, chunk[3] as usize, &a_encode); // A (alpha)
    }

    // Finalise bitstream.
    let payload = bw.finish();

    // Prepend the VP8L signature byte (0x2F).
    let mut result = Vec::with_capacity(1 + payload.len());
    result.push(0x2Fu8);
    result.extend_from_slice(&payload);

    result
}

// ---------------------------------------------------------------------------
// VP8L decode
// ---------------------------------------------------------------------------

/// Decode a VP8L bitstream (starting with the 0x2F signature byte).
///
/// Returns a `PixelContainer` on success, or a descriptive error string.
pub fn decode(data: &[u8]) -> Result<PixelContainer, String> {
    if data.is_empty() {
        return Err("VP8L: empty bitstream".to_string());
    }

    // ── Signature byte ───────────────────────────────────────────────────────
    if data[0] != 0x2F {
        return Err(format!(
            "VP8L: bad signature byte 0x{:02X} (expected 0x2F)",
            data[0]
        ));
    }

    let mut br = BitReader::new(&data[1..]);

    // ── Header ───────────────────────────────────────────────────────────────
    let width = br.read_bits(14) + 1;
    let height = br.read_bits(14) + 1;
    let _alpha_is_used = br.read_bits(1);
    let version = br.read_bits(3);

    if version != 0 {
        return Err(format!(
            "VP8L: unsupported version {version} (expected 0)"
        ));
    }

    // ── Transform section ────────────────────────────────────────────────────
    let mut applied_transforms: Vec<u8> = Vec::new();
    loop {
        let has_transform = br.read_bits(1);
        if has_transform == 0 {
            break;
        }
        let transform_type = br.read_bits(2);
        applied_transforms.push(transform_type as u8);

        match transform_type {
            2 => { /* SubtractGreen: no extra data in the bitstream */ }
            0 | 1 | 3 => {
                return Err(format!(
                    "VP8L: transform type {transform_type} not yet implemented in decoder"
                ));
            }
            _ => unreachable!(),
        }
    }

    // ── Color cache ──────────────────────────────────────────────────────────
    let color_cache_code_bits = br.read_bits(4);
    if color_cache_code_bits > 0 {
        return Err(format!(
            "VP8L: color cache (code_bits={color_cache_code_bits}) not yet implemented"
        ));
    }

    // ── Huffman code tables ──────────────────────────────────────────────────
    let g_table = read_huffman_code(&mut br, G_ALPHABET_SIZE)?;
    let r_table = read_huffman_code(&mut br, RGBA_ALPHABET_SIZE)?;
    let b_table = read_huffman_code(&mut br, RGBA_ALPHABET_SIZE)?;
    let a_table = read_huffman_code(&mut br, RGBA_ALPHABET_SIZE)?;
    let _d_table = read_huffman_code(&mut br, DIST_ALPHABET_SIZE)?;

    // ── Pixel data ───────────────────────────────────────────────────────────
    let pixel_count = (width as usize) * (height as usize);
    let mut data_out = Vec::with_capacity(pixel_count * 4);

    for _ in 0..pixel_count {
        let g_sym = g_table.decode(&mut br)?;

        match g_sym {
            0..=255 => {
                // Literal pixel: G is green; read R, B, A from their trees.
                let g = g_sym as u8;
                let r = r_table.decode(&mut br)? as u8;
                let b = b_table.decode(&mut br)? as u8;
                let a = a_table.decode(&mut br)? as u8;
                // PixelContainer stores pixels as [R, G, B, A].
                data_out.push(r);
                data_out.push(g);
                data_out.push(b);
                data_out.push(a);
            }
            256..=u16::MAX => {
                // LZ77 back-reference or color-cache — not implemented in v0.1.
                return Err(format!(
                    "VP8L LZ77 back-references not yet implemented (G symbol={g_sym})"
                ));
            }
        }
    }

    // ── Apply inverse transforms ─────────────────────────────────────────────
    let mut pixels = PixelContainer::from_data(width, height, data_out);

    // Transforms are inverted in reverse order of encoding.
    for &t in applied_transforms.iter().rev() {
        match t {
            2 => transforms::inverse_subtract_green(&mut pixels),
            _ => unreachable!(),
        }
    }

    Ok(pixels)
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Emit one Huffman symbol into the bitstream.
///
/// If `encode_table[symbol].1 == 0` (trivial code), nothing is written.
fn emit_symbol(bw: &mut BitWriter, symbol: usize, encode_table: &[(u64, u32)]) {
    let (bits, count) = encode_table[symbol];
    if count > 0 {
        bw.write_bits(bits, count);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_produces_vp8l_signature() {
        let pixels = PixelContainer::new(2, 2);
        let bs = encode(&pixels);
        assert_eq!(bs[0], 0x2F, "VP8L signature byte must be 0x2F");
    }

    #[test]
    fn round_trip_blank() {
        let pixels = PixelContainer::new(4, 4);
        let encoded = encode(&pixels);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.width, 4);
        assert_eq!(decoded.height, 4);
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_solid_color() {
        let mut pixels = PixelContainer::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                pixels.set_pixel(x, y, 200, 100, 50, 255);
            }
        }
        let encoded = encode(&pixels);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_gradient() {
        let mut pixels = PixelContainer::new(8, 8);
        for y in 0..8u32 {
            for x in 0..8u32 {
                pixels.set_pixel(x, y, (x * 30) as u8, (y * 30) as u8, 128, 255);
            }
        }
        let encoded = encode(&pixels);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.width, 8);
        assert_eq!(decoded.height, 8);
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn decode_bad_signature() {
        let bad = vec![0x00u8, 0x00, 0x00];
        let result = decode(&bad);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("signature"));
    }

    #[test]
    fn decode_empty() {
        let result = decode(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn round_trip_1x1_red() {
        let mut pixels = PixelContainer::new(1, 1);
        pixels.set_pixel(0, 0, 255, 0, 0, 255);
        let encoded = encode(&pixels);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.pixel_at(0, 0), (255, 0, 0, 255));
    }

    #[test]
    fn round_trip_varying_alpha() {
        let mut pixels = PixelContainer::new(4, 1);
        pixels.set_pixel(0, 0, 10, 20, 30, 0);
        pixels.set_pixel(1, 0, 10, 20, 30, 85);
        pixels.set_pixel(2, 0, 10, 20, 30, 170);
        pixels.set_pixel(3, 0, 10, 20, 30, 255);
        let encoded = encode(&pixels);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }
}
