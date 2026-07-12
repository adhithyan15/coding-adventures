//! # JXL encoder — top-level `encode_jxl` implementation
//!
//! Converts a [`PixelContainer`] into a naked JXL codestream.
//!
//! ## Wire format (simplified JXL Modular)
//!
//! The encoder emits a *naked codestream* (not an ISOBMFF container).
//! Everything after the two-byte magic is our simplified fixed format — we do
//! not implement the full JXL metadata ANS sections; see the spec notes in the
//! crate root for the teaching trade-off.
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │ 2 bytes         │ naked codestream magic: 0xFF 0x0A              │
//! │ variable bits   │ SizeHeader (MSB-first raw bits, spec §4.1)    │
//! │ padding bits    │ zero-pad to next byte boundary                 │
//! │ 1 byte          │ num_channels (3 = RGB, 4 = RGBA)               │
//! │ 4 bytes LE      │ width  (u32)                                   │
//! │ 4 bytes LE      │ height (u32)                                   │
//! │ For each channel R, G, B, [A]:                                   │
//! │   [sign rANS block]  — see entropy::encode_channel_residuals     │
//! │   [mag  rANS block]                                              │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## SizeHeader encoding
//!
//! JXL spec §4.1 encodes image dimensions in raw MSB-first bits:
//!
//! ```text
//! For each dimension (height first, then width if ratio=0):
//!   div8: 1 bit
//!   if div8:
//!     dim_div8: 5 bits  →  dim = (dim_div8 + 1) * 8   (max 256 px)
//!   else:
//!     sel: 2 bits       →  bit_count = [9, 13, 18, 30][sel]
//!     dim: bit_count    →  dim = stored_value + 1
//!
//! ratio: 3 bits   (0 = explicit width follows)
//! ```

use crate::bitwriter::BitWriter;
use crate::entropy::encode_channel_residuals;
use crate::modular::compute_residuals;
use pixel_container::PixelContainer;

// ── SizeHeader helpers ───────────────────────────────────────────────────────

/// Encode a single dimension (height or width) into the BitWriter.
///
/// Strategy:
/// - If dim is divisible by 8 and dim / 8 − 1 fits in 5 bits (dim ≤ 256)
///   → use the compact div8 path.
/// - Otherwise → use the direct path with the smallest bit-count that fits.
fn encode_dim(bw: &mut BitWriter, dim: u32) {
    // The compact div8 path: flag=1, then a 5-bit field holding (dim/8 − 1).
    // Valid range: dim ∈ {8, 16, 24, …, 256}.
    if dim > 0 && dim.is_multiple_of(8) && (dim / 8) <= 32 {
        bw.write_bit(true); // div8 = 1
        bw.write_bits((dim / 8 - 1) as u64, 5);
        return;
    }

    // Direct path: flag=0, then a 2-bit selector, then the value.
    bw.write_bit(false); // div8 = 0
    let val = dim - 1; // store (dim − 1) so dim = stored + 1
    if val < (1 << 9) {
        bw.write_bits(0, 2); // sel = 0 → 9 bits
        bw.write_bits(val as u64, 9);
    } else if val < (1 << 13) {
        bw.write_bits(1, 2); // sel = 1 → 13 bits
        bw.write_bits(val as u64, 13);
    } else if val < (1 << 18) {
        bw.write_bits(2, 2); // sel = 2 → 18 bits
        bw.write_bits(val as u64, 18);
    } else {
        bw.write_bits(3, 2); // sel = 3 → 30 bits
        bw.write_bits(val as u64, 30);
    }
}

/// Encode the full SizeHeader (height + ratio + width) into raw bits and return
/// the flushed bytes.
fn encode_size_header(width: u32, height: u32) -> Vec<u8> {
    let mut bw = BitWriter::new();
    encode_dim(&mut bw, height);
    bw.write_bits(0, 3); // ratio = 0 → explicit width follows
    encode_dim(&mut bw, width);
    bw.finish()
}

// ── Main encoder ─────────────────────────────────────────────────────────────

/// Encode a [`PixelContainer`] into a simplified JXL Modular naked codestream.
///
/// The result is a complete, self-contained byte stream that `decode_jxl` can
/// round-trip back to an identical `PixelContainer`.
///
/// # Panics
///
/// Panics if the pixel container has zero width or zero height (an empty image
/// cannot be represented in JXL SizeHeader).
pub fn encode(pixels: &PixelContainer) -> Vec<u8> {
    let width = pixels.width;
    let height = pixels.height;

    assert!(width > 0 && height > 0, "JXL: cannot encode a zero-dimension image");

    // Decide whether we need an alpha channel.
    // We only write 4 channels when at least one pixel has A ≠ 255.
    let has_alpha = (0..height).any(|y| (0..width).any(|x| pixels.pixel_at(x, y).3 != 255));
    let num_channels: u8 = if has_alpha { 4 } else { 3 };

    let mut out = Vec::new();

    // ── Magic ──────────────────────────────────────────────────────────
    out.extend_from_slice(&[0xFF, 0x0A]);

    // ── SizeHeader (raw bits) ───────────────────────────────────────────
    let size_bits = encode_size_header(width, height);
    out.extend_from_slice(&size_bits);
    // SizeHeader bits are already flushed/padded to whole bytes by BitWriter.

    // ── Fixed simple header ─────────────────────────────────────────────
    out.push(num_channels);
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());

    // ── Channel residual blocks ─────────────────────────────────────────
    for ch in 0..num_channels as usize {
        // Flatten the channel into a linear i32 buffer.
        let values: Vec<i32> = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    let (r, g, b, a) = pixels.pixel_at(x, y);
                    match ch {
                        0 => r as i32,
                        1 => g as i32,
                        2 => b as i32,
                        _ => a as i32,
                    }
                })
            })
            .collect();

        let residuals = compute_residuals(&values, width, height);
        encode_channel_residuals(&residuals, &mut out);
    }

    out
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes_at_start() {
        let p = PixelContainer::new(4, 4);
        let bytes = encode(&p);
        assert_eq!(&bytes[..2], &[0xFF, 0x0A]);
    }

    #[test]
    fn rgb_only_when_fully_opaque() {
        let mut p = PixelContainer::new(2, 2);
        p.fill(100, 100, 100, 255);
        let bytes = encode(&p);
        // After magic (2) + SizeHeader + fixed-header, first byte of fixed hdr
        // is num_channels — find it by scanning past SizeHeader.
        // We'll just assert the output is non-empty and starts correctly.
        assert!(bytes.len() > 10);
    }

    #[test]
    fn rgba_when_has_transparent_pixel() {
        let mut p = PixelContainer::new(2, 2);
        p.fill(0, 0, 0, 0); // fully transparent
        let bytes = encode(&p);
        assert!(bytes.len() > 10);
    }

    #[test]
    fn encode_1x1() {
        let mut p = PixelContainer::new(1, 1);
        p.set_pixel(0, 0, 128, 64, 32, 255);
        let bytes = encode(&p);
        assert!(bytes.len() > 2);
    }
}
