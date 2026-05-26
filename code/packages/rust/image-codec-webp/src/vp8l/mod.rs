//! VP8L lossless WebP encode and decode.
//!
//! This module implements the VP8L bitstream format:
//!
//! ```text
//! [signature byte 0x2F]
//! [32-bit header: width-1(14b), height-1(14b), alpha_is_used(1b), version(3b)]
//! [has_transform: 1 bit = 0 (no transforms in this release)]
//! [color_cache_code_bits: 4 bits = 0 (no color cache)]
//! [5 Huffman group tables: G, R, B, A, Dist]
//! [pixel data: literals and LZ77 back-references]
//! ```
//!
//! ## Encoding strategy
//!
//! - **LZ77 with hash chain**: a 65 536-slot direct-mapped hash table finds
//!   matching pixel runs.  Matches of length ≥ 2 are emitted as back-references;
//!   all others fall back to literals.
//! - No transforms (subtract-green, predictor, colour, colour-index).
//! - No colour cache.
//!
//! Back-references use the 40-symbol Dist prefix alphabet with direct 1D
//! pixel offsets (dist_code = pixel_offset + 120).  Overlapping copies (RLE)
//! are handled by copying one pixel at a time.
//!
//! ## Decoding
//!
//! The decoder supports:
//! - Simple 1-symbol, 2-symbol, and complex (meta-Huffman) code tables.
//! - Literal pixel decoding (G symbol 0..=255).
//! - LZ77 back-references (G symbol 256..=279) with overlapping copy.

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
// LZ77 types and helpers
// ---------------------------------------------------------------------------

/// Hash table size for the LZ77 encoder (one slot per hash value).
const HASH_SIZE: usize = 1 << 16;

/// Maximum copy length for an LZ77 back-reference.
const MAX_LEN: u32 = 128;

/// A single encoded VP8L pixel-stream symbol.
enum Sym {
    /// Literal pixel in R, G, B, A order (PixelContainer layout).
    Lit { r: u8, g: u8, b: u8, a: u8 },
    /// LZ77 back-reference encoded as G-group + Dist prefix codes + extra bits.
    BackRef {
        g_sym: u32, g_nextra: u32, g_extra: u32,
        d_sym: u32, d_nextra: u32, d_extra: u32,
    },
}

/// Read the 4 bytes for pixel `pos` from the raw pixel data and pack to u32.
fn pixel_u32(data: &[u8], pos: usize) -> u32 {
    let b = pos * 4;
    u32::from_le_bytes([data[b], data[b + 1], data[b + 2], data[b + 3]])
}

/// Hash a 32-bit ARGB value to a slot in `[0, HASH_SIZE)`.
fn pixel_hash(v: u32) -> usize {
    ((v.wrapping_mul(0x1E35_A7BD)) >> 16) as usize & (HASH_SIZE - 1)
}

/// Run LZ77 matching over the raw pixel data.
///
/// Returns a sequence of `Sym` values (literals and back-references) that
/// encode exactly `num` pixels.  Uses a single-entry hash chain (greedy,
/// no lazy matching).
fn lz77_match(data: &[u8], num: usize) -> Vec<Sym> {
    let mut syms = Vec::with_capacity(num);
    // hash_table[h] = most recent pixel position that mapped to hash h.
    // Initialised to 0 — treated as a miss when pos==0 by the `prev < pos` guard.
    let mut hash_table = vec![0usize; HASH_SIZE];
    let mut pos = 0;

    while pos < num {
        let v = pixel_u32(data, pos);
        let h = pixel_hash(v);
        let prev = hash_table[h];

        let mut used_backref = false;

        if prev < pos && pixel_u32(data, prev) == v {
            let pixel_offset = (pos - prev) as u32;
            // dist_code uses direct 1D encoding (offset ≥ 121 bypasses 2D map).
            let dist_code = pixel_offset + 120;
            if dist_code <= lz77::MAX_DIST_CODE {
                // Count how many consecutive pixels match.
                let max = ((num - pos) as u32).min(MAX_LEN);
                let mut len = 1u32;
                while len < max
                    && pixel_u32(data, pos + len as usize)
                        == pixel_u32(data, prev + len as usize)
                {
                    len += 1;
                }
                if len >= 2 {
                    let (g_sym, g_nextra, g_extra) = lz77::encode_length(len);
                    let (d_sym, d_nextra, d_extra) = lz77::encode_dist_code(dist_code);
                    syms.push(Sym::BackRef { g_sym, g_nextra, g_extra, d_sym, d_nextra, d_extra });
                    // Update hash for every pixel in the matched span.
                    for k in 0..len as usize {
                        let hk = pixel_hash(pixel_u32(data, pos + k));
                        hash_table[hk] = pos + k;
                    }
                    pos += len as usize;
                    used_backref = true;
                }
            }
        }

        if !used_backref {
            hash_table[h] = pos;
            let b = pos * 4;
            syms.push(Sym::Lit { r: data[b], g: data[b + 1], b: data[b + 2], a: data[b + 3] });
            pos += 1;
        }
    }

    syms
}

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
    let num = (pixels.width as usize) * (pixels.height as usize);

    // ── Phase 1: LZ77 match pass ─────────────────────────────────────────────
    let syms = lz77_match(&pixels.data, num);

    // ── Phase 2: Count symbol frequencies ───────────────────────────────────
    let mut g_freq = vec![0u32; G_ALPHABET_SIZE];
    let mut r_freq = vec![0u32; RGBA_ALPHABET_SIZE];
    let mut b_freq = vec![0u32; RGBA_ALPHABET_SIZE];
    let mut a_freq = vec![0u32; RGBA_ALPHABET_SIZE];
    let mut d_freq = vec![0u32; DIST_ALPHABET_SIZE];

    for sym in &syms {
        match sym {
            Sym::Lit { r, g, b, a } => {
                g_freq[*g as usize] += 1;
                r_freq[*r as usize] += 1;
                b_freq[*b as usize] += 1;
                a_freq[*a as usize] += 1;
            }
            Sym::BackRef { g_sym, d_sym, .. } => {
                g_freq[*g_sym as usize] += 1;
                d_freq[*d_sym as usize] += 1;
            }
        }
    }
    // Dist tree must have at least 1 active symbol for the trivial-code path.
    if d_freq.iter().all(|&f| f == 0) {
        d_freq[0] = 1;
    }

    // ── Phase 3: Build canonical Huffman tables ───────────────────────────────
    let g_lens = lengths_from_frequencies(&g_freq);
    let r_lens = lengths_from_frequencies(&r_freq);
    let b_lens = lengths_from_frequencies(&b_freq);
    let a_lens = lengths_from_frequencies(&a_freq);
    let d_lens = lengths_from_frequencies(&d_freq);

    let g_encode = build_encode_table(&g_lens);
    let r_encode = build_encode_table(&r_lens);
    let b_encode = build_encode_table(&b_lens);
    let a_encode = build_encode_table(&a_lens);
    let d_encode = build_encode_table(&d_lens);

    // ── Phase 4: Write bitstream ─────────────────────────────────────────────
    let mut bw = BitWriter::new();

    // Header: (width-1) in 14 bits, (height-1) in 14 bits, alpha=1, version=0.
    bw.write_bits(w - 1, 14);
    bw.write_bits(h - 1, 14);
    bw.write_bits(1, 1); // alpha_is_used = 1
    bw.write_bits(0, 3); // version_number = 0

    bw.write_bits(0, 1); // has_transform = 0
    bw.write_bits(0, 4); // color_cache_code_bits = 0

    write_huffman_code(&mut bw, &g_lens);
    write_huffman_code(&mut bw, &r_lens);
    write_huffman_code(&mut bw, &b_lens);
    write_huffman_code(&mut bw, &a_lens);
    write_huffman_code(&mut bw, &d_lens);

    for sym in &syms {
        match sym {
            Sym::Lit { r, g, b, a } => {
                emit_symbol(&mut bw, *g as usize, &g_encode);
                emit_symbol(&mut bw, *r as usize, &r_encode);
                emit_symbol(&mut bw, *b as usize, &b_encode);
                emit_symbol(&mut bw, *a as usize, &a_encode);
            }
            Sym::BackRef { g_sym, g_nextra, g_extra, d_sym, d_nextra, d_extra } => {
                emit_symbol(&mut bw, *g_sym as usize, &g_encode);
                if *g_nextra > 0 { bw.write_bits(*g_extra as u64, *g_nextra); }
                emit_symbol(&mut bw, *d_sym as usize, &d_encode);
                if *d_nextra > 0 { bw.write_bits(*d_extra as u64, *d_nextra); }
            }
        }
    }

    let payload = bw.finish();
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
    let d_table = read_huffman_code(&mut br, DIST_ALPHABET_SIZE)?;

    // ── Pixel data ───────────────────────────────────────────────────────────
    let pixel_count = (width as usize) * (height as usize);
    let mut data_out = Vec::with_capacity(pixel_count * 4);
    let mut pos = 0usize;

    while pos < pixel_count {
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
                pos += 1;
            }
            256..=279 => {
                // LZ77 back-reference: G sym encodes copy length.
                let (base_len, nextra) = lz77::length_symbol_to_base(g_sym as u32);
                let len_extra = if nextra > 0 { br.read_bits(nextra) } else { 0 };
                let copy_len = (base_len + len_extra) as usize;

                // Distance from Dist group.
                let d_sym = d_table.decode(&mut br)? as u32;
                let d_nextra = lz77::DIST_BITS[d_sym as usize];
                let d_extra = if d_nextra > 0 { br.read_bits(d_nextra) } else { 0 };
                let dist_code = lz77::decode_dist(d_sym, d_extra);
                let pixel_offset = lz77::dist_code_to_offset(dist_code, width);

                if pixel_offset > pos {
                    return Err(format!(
                        "VP8L: back-ref pixel_offset={pixel_offset} > pos={pos}"
                    ));
                }

                // Cap copy_len to not exceed pixel_count.
                let copy_len = copy_len.min(pixel_count - pos);

                // Copy one pixel at a time: supports overlapping (RLE) copies
                // where copy_len > pixel_offset.
                for i in 0..copy_len {
                    let src = (pos - pixel_offset + i) * 4;
                    let r = data_out[src];
                    let g = data_out[src + 1];
                    let b = data_out[src + 2];
                    let a = data_out[src + 3];
                    data_out.push(r);
                    data_out.push(g);
                    data_out.push(b);
                    data_out.push(a);
                }
                pos += copy_len;
            }
            _ => {
                return Err(format!(
                    "VP8L: unrecognized G symbol {g_sym} (color-cache not implemented)"
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
        // All-zero 4×4 image — every pixel is identical, should use LZ77.
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

    #[test]
    fn round_trip_lz77_rle() {
        // 1×10 image, all same color — LZ77 RLE: one literal + one back-ref.
        let mut pixels = PixelContainer::new(1, 10);
        for y in 0..10 {
            pixels.set_pixel(0, y, 200, 100, 50, 255);
        }
        let encoded = encode(&pixels);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn round_trip_lz77_repeating_pattern() {
        // 2-pixel repeating pattern to exercise back-refs with pixel_offset=2.
        let mut pixels = PixelContainer::new(8, 1);
        for x in 0..8u32 {
            if x % 2 == 0 {
                pixels.set_pixel(x, 0, 255, 0, 0, 255);
            } else {
                pixels.set_pixel(x, 0, 0, 0, 255, 255);
            }
        }
        let encoded = encode(&pixels);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded.data, pixels.data);
    }

    #[test]
    fn lz77_encoding_is_smaller_than_literal_for_solid() {
        // A solid 16×16 image should compress better than a random 16×16 image.
        let mut solid = PixelContainer::new(16, 16);
        solid.fill(200, 100, 50, 255);
        let mut varied = PixelContainer::new(16, 16);
        for y in 0..16u32 {
            for x in 0..16u32 {
                varied.set_pixel(x, y, (x * 13 + y * 7) as u8, (x + y * 3) as u8, (x * 3) as u8, 255);
            }
        }
        let solid_bytes = encode(&solid).len();
        let varied_bytes = encode(&varied).len();
        assert!(solid_bytes < varied_bytes,
            "solid image ({solid_bytes}B) should be smaller than varied ({varied_bytes}B)");
    }
}
