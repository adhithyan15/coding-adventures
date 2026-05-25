// # color.rs — BT.601 RGB ↔ YCbCr colour space conversions
//
// JPEG stores luminance (Y) and two chrominance channels (Cb, Cr) rather than
// raw RGB. Why? Human eyes are far more sensitive to brightness differences than
// to colour differences. By separating them, we can compress the colour channels
// more aggressively and lose less perceptual quality.
//
// ## BT.601 standard
//
// The ITU-R BT.601 standard defines the exact matrix coefficients for
// converting between RGB and YCbCr. These same coefficients are mandated by the
// JFIF specification (the JPEG container format), so every compliant JPEG
// encoder/decoder must use them.
//
// ## YCbCr channel ranges
//
// After conversion from [0, 255] RGB:
//
//   Y  ∈ [0, 255]         — luminance (brightness)
//   Cb ∈ [0, 255], 128-centred — blue chrominance difference (Cb = blue - luma)
//   Cr ∈ [0, 255], 128-centred — red chrominance difference  (Cr = red  - luma)
//
// The 128-offset places the zero point at the middle of the byte range. A
// neutral grey has Cb = Cr = 128. Pure blue has Cb near 255; pure red has Cr
// near 255.
//
// ## Level shift
//
// The DCT encoder subtracts 128 from each sample before the transform ("level
// shift") so that coefficients are centred near zero. The decoder adds 128 back
// after the IDCT ("level un-shift"). This module does NOT perform the level
// shift — that step happens in encoder.rs and decoder.rs, around the DCT call.

/// Convert an 8-bit RGB triple to Y, Cb, Cr (BT.601, JFIF).
///
/// Alpha is ignored — JPEG is an opaque format with no transparency channel.
///
/// # Examples
///
/// ```ignore
/// let (y, cb, cr) = rgb_to_ycbcr(255, 0, 0); // pure red
/// // Y ≈ 76.245, Cb ≈ 84.972, Cr ≈ 255.0 (clamped from 255.5)
/// ```
pub fn rgb_to_ycbcr(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let r = r as f32;
    let g = g as f32;
    let b = b as f32;

    // BT.601 forward matrix (exact JFIF/JPEG coefficients):
    //   Y  = 0.299·R   + 0.587·G   + 0.114·B
    //   Cb = -0.168736·R - 0.331264·G + 0.5·B + 128
    //   Cr =  0.5·R   - 0.418688·G - 0.081312·B + 128
    //
    // The +128 offsets centre Cb and Cr in the [0, 255] byte range.
    let y  =  0.299    * r + 0.587    * g + 0.114    * b;
    let cb = -0.168736 * r - 0.331264 * g + 0.5      * b + 128.0;
    let cr =  0.5      * r - 0.418688 * g - 0.081312 * b + 128.0;

    (y, cb, cr)
}

/// Convert Y, Cb, Cr (BT.601, JFIF) to an 8-bit RGB triple, clamped to [0, 255].
///
/// # Examples
///
/// ```ignore
/// let (r, g, b) = ycbcr_to_rgb(76.245, 84.972, 255.0);
/// // r ≈ 255, g ≈ 0, b ≈ 0
/// ```
pub fn ycbcr_to_rgb(y: f32, cb: f32, cr: f32) -> (u8, u8, u8) {
    // Subtract the 128-centring offset before applying the inverse matrix.
    let cb = cb - 128.0;
    let cr = cr - 128.0;

    // BT.601 inverse matrix (exact JFIF/JPEG coefficients):
    //   R = Y                    + 1.402·Cr
    //   G = Y - 0.344136·Cb     - 0.714136·Cr
    //   B = Y + 1.772·Cb
    //
    // Due to quantisation loss, the resulting values can fall outside [0, 255].
    // We clamp them to keep everything in a valid u8 range.
    let r = y                          + 1.402    * cr;
    let g = y - 0.344136 * cb - 0.714136 * cr;
    let b = y + 1.772    * cb;

    (clamp_u8(r), clamp_u8(g), clamp_u8(b))
}

/// Round `v` to the nearest integer and clamp to [0, 255].
///
/// The rounding step (`.round()`) is critical: without it, accumulated floating-
/// point error can leave values like 254.9999 truncated to 254 instead of 255,
/// causing subtle banding artefacts in solid-colour regions.
#[inline]
fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32, tol: f32) -> bool {
        (a - b).abs() <= tol
    }

    /// Round-tripping pure grey through RGB → YCbCr → RGB should be lossless.
    #[test]
    fn grey_roundtrip() {
        for v in [0u8, 64, 128, 192, 255] {
            let (y, cb, cr) = rgb_to_ycbcr(v, v, v);
            let (r, g, b) = ycbcr_to_rgb(y, cb, cr);
            // Grey pixels have Cb = Cr = 128 exactly.
            assert!(approx(cb, 128.0, 0.5), "grey Cb={cb} expected 128");
            assert!(approx(cr, 128.0, 0.5), "grey Cr={cr} expected 128");
            // After round-trip, we should recover the original value (or very close).
            let diff = (v as i16 - r as i16).abs()
                .max((v as i16 - g as i16).abs())
                .max((v as i16 - b as i16).abs());
            assert!(diff <= 1, "grey {v}: round-trip gave ({r},{g},{b})");
        }
    }

    /// Y for a pure-red pixel should follow the BT.601 coefficient (≈ 76.245).
    #[test]
    fn red_luma_coefficient() {
        let (y, _cb, cr) = rgb_to_ycbcr(255, 0, 0);
        // Y = 0.299 * 255 ≈ 76.245
        assert!(approx(y, 76.245, 0.5), "red Y={y}");
        // Cr should be near the maximum (red → positive Cr)
        assert!(cr > 200.0, "red Cr={cr} expected large positive");
    }

    /// Pure blue should produce large positive Cb and near-zero Cr.
    #[test]
    fn blue_cb_coefficient() {
        let (_y, cb, cr) = rgb_to_ycbcr(0, 0, 255);
        assert!(cb > 200.0, "blue Cb={cb} expected large positive");
        assert!(approx(cr, 128.0 - 20.7, 3.0), "blue Cr={cr}");
    }

    /// Solid-colour 8×8 block round-trip through YCbCr and back.
    #[test]
    fn solid_colour_roundtrip_close() {
        let cases = [(200u8, 50u8, 50u8), (128, 200, 64), (0, 0, 0), (255, 255, 255)];
        for (ri, gi, bi) in cases {
            let (y, cb, cr) = rgb_to_ycbcr(ri, gi, bi);
            let (ro, go, bo) = ycbcr_to_rgb(y, cb, cr);
            let dr = (ri as i16 - ro as i16).abs();
            let dg = (gi as i16 - go as i16).abs();
            let db = (bi as i16 - bo as i16).abs();
            assert!(dr <= 1 && dg <= 1 && db <= 1,
                "({ri},{gi},{bi}) → ({ro},{go},{bo}) diff=({dr},{dg},{db})");
        }
    }
}
