//! # JXL decoder — top-level `decode_jxl` implementation
//!
//! Parses the simplified JXL Modular naked codestream produced by
//! [`encoder::encode`] and reconstructs the original [`PixelContainer`].
//!
//! ## Parsing steps
//!
//! 1. Strip the container via [`container::extract_codestream`] — handles
//!    both the naked (`FF 0A`) and ISOBMFF (`JXL ` signature) forms.
//! 2. Parse the SizeHeader (raw bits, MSB-first).
//! 3. Read the fixed simple header: `num_channels`, `width`, `height`.
//! 4. For each channel decode the sign + magnitude rANS block pair.
//! 5. Reconstruct pixel values from residuals using the gradient predictor.
//! 6. Assemble the result into a [`PixelContainer`].

use crate::bitreader::BitReader;
use crate::container::extract_codestream;
use crate::entropy::decode_channel_residuals;
use crate::modular::reconstruct_values;
use pixel_container::PixelContainer;

// ── SizeHeader decoder ───────────────────────────────────────────────────────

/// Decode one dimension (height or width) from the bit reader.
///
/// Mirrors the encoder's `encode_dim` exactly.
fn decode_dim(br: &mut BitReader) -> Result<u32, String> {
    let div8 = br.read_bit()?;
    if div8 {
        // Compact path: 5-bit field stores (dim/8 − 1).
        let d = br.read_bits(5)? as u32;
        Ok((d + 1) * 8)
    } else {
        // Direct path: 2-bit selector + variable-width dimension.
        let sel = br.read_bits(2)?;
        let bit_count = match sel {
            0 => 9u8,
            1 => 13,
            2 => 18,
            _ => 30,
        };
        let val = br.read_bits(bit_count)? as u32;
        Ok(val + 1) // stored as (dim − 1)
    }
}

/// Parse the SizeHeader from the codestream bytes (starting immediately after
/// the two-byte naked magic has been stripped).
///
/// Returns `(width, height, bytes_consumed)`.
///
/// We handle the full ratio table from the JXL spec so the decoder can
/// potentially accept ratio-encoded files even though our encoder always uses
/// ratio = 0 (explicit width).
fn decode_size_header(cs: &[u8]) -> Result<(u32, u32, usize), String> {
    let mut br = BitReader::new(cs);

    // Height is encoded first.
    let height = decode_dim(&mut br)?;

    // 3-bit ratio field.
    let ratio = br.read_bits(3)?;

    let width = if ratio == 0 {
        // Explicit width.
        decode_dim(&mut br)?
    } else {
        // Predefined aspect ratio — compute width from height.
        // Ratios from JXL spec §4.1 (ratio 1–7).
        match ratio {
            1 => height,              // 1:1 (square)
            2 => (height * 12) / 8,  // 12:8 = 3:2
            3 => (height * 16) / 8,  // 16:8 = 2:1
            4 => (height * 4) / 3,   // 4:3
            5 => (height * 3) / 2,   // 3:2
            6 => height * 2,         // 2:1
            7 => (height * 5) / 4,   // 5:4
            _ => return Err(format!("JXL: unexpected ratio value {}", ratio)),
        }
    };

    // How many whole bytes did the SizeHeader occupy?
    // `bytes_consumed()` rounds up to the next byte boundary.
    let bytes = br.bytes_consumed();

    Ok((width, height, bytes))
}

// ── Main decoder ─────────────────────────────────────────────────────────────

/// Decode a simplified JXL Modular codestream back into a [`PixelContainer`].
///
/// Accepts both naked codestreams (`FF 0A …`) and ISOBMFF containers.
///
/// # Errors
///
/// Returns `Err` with a descriptive message if the data is not valid simplified
/// JXL Modular output (as produced by [`encoder::encode`]), or if any internal
/// buffer is truncated.
pub fn decode(data: &[u8]) -> Result<PixelContainer, String> {
    // ── 1. Strip container / find raw codestream ─────────────────────────
    let cs = extract_codestream(data)?;

    // ── 2. Parse SizeHeader ──────────────────────────────────────────────
    let (width, height, size_bytes) = decode_size_header(cs)?;

    // The rest of the stream starts right after the SizeHeader bytes.
    let rest = &cs[size_bytes..];

    // ── 3. Fixed simple header ───────────────────────────────────────────
    // Layout: [1 byte num_channels] [4 bytes LE width] [4 bytes LE height]
    if rest.len() < 9 {
        return Err(format!(
            "JXL: stream truncated after SizeHeader — need 9 bytes for simple header, got {}",
            rest.len()
        ));
    }
    let num_channels = rest[0] as usize;
    let w = u32::from_le_bytes(rest[1..5].try_into().unwrap());
    let h = u32::from_le_bytes(rest[5..9].try_into().unwrap());

    // The width/height in the simple header must match the SizeHeader.
    if w != width || h != height {
        return Err(format!(
            "JXL: simple header dimension mismatch — SizeHeader says {}×{}, simple header says {}×{}",
            width, height, w, h
        ));
    }
    if num_channels != 3 && num_channels != 4 {
        return Err(format!(
            "JXL: unsupported channel count {} (only 3 or 4 accepted)",
            num_channels
        ));
    }

    // ── 4. Decode residual blocks for each channel ───────────────────────
    let mut pos = 9usize; // offset into `rest`
    let mut channels: Vec<Vec<i32>> = Vec::with_capacity(num_channels);

    for _ in 0..num_channels {
        if pos > rest.len() {
            return Err("JXL: stream truncated before channel residual data".into());
        }
        let (residuals, consumed) = decode_channel_residuals(&rest[pos..])?;
        pos += consumed;

        // ── 5. Reconstruct pixel values from residuals ───────────────────
        if residuals.len() != (width * height) as usize {
            return Err(format!(
                "JXL: residual count {} does not match {}×{}={}",
                residuals.len(),
                width,
                height,
                width * height
            ));
        }
        let values = reconstruct_values(&residuals, width, height);
        channels.push(values);
    }

    // ── 6. Assemble PixelContainer ───────────────────────────────────────
    let mut pc = PixelContainer::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let r = channels[0][idx].clamp(0, 255) as u8;
            let g = channels[1][idx].clamp(0, 255) as u8;
            let b = channels[2][idx].clamp(0, 255) as u8;
            let a = if num_channels == 4 {
                channels[3][idx].clamp(0, 255) as u8
            } else {
                255
            };
            pc.set_pixel(x, y, r, g, b, a);
        }
    }

    Ok(pc)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoder;

    fn roundtrip(p: &PixelContainer) -> PixelContainer {
        let bytes = encoder::encode(p);
        decode(&bytes).unwrap()
    }

    #[test]
    fn decode_bad_magic_err() {
        assert!(decode(b"\x89PNG\r\n\x1a\n").is_err());
    }

    #[test]
    fn decode_too_short_err() {
        assert!(decode(b"\xFF").is_err());
    }

    #[test]
    fn decode_1x1_solid() {
        let mut p = PixelContainer::new(1, 1);
        p.set_pixel(0, 0, 10, 20, 30, 255);
        let q = roundtrip(&p);
        assert_eq!(q.pixel_at(0, 0), (10, 20, 30, 255));
    }

    #[test]
    fn decode_preserves_dimensions() {
        let p = PixelContainer::new(7, 11);
        let q = roundtrip(&p);
        assert_eq!(q.width, 7);
        assert_eq!(q.height, 11);
    }
}
