// # color.rs — White balance, camera colour matrix, and sRGB gamma
//
// ## The Colour Pipeline
//
// RAW sensor data measures photon counts — a device-dependent, linear-light
// value. To produce a perceptually correct JPEG-like image, we need to:
//
//   1. Remove black level (sensor noise floor)
//   2. Normalize to [0.0, 1.0]
//   3. Multiply each channel by its white-balance gain
//   4. Apply a 3×3 camera-to-sRGB colour matrix
//   5. Apply the sRGB gamma curve (linearize then toe-compress)
//   6. Clip to [0, 255] and cast to u8
//
// ## White Balance
//
// Digital cameras record raw Bayer values relative to the illuminant. A pixel
// under a warm incandescent bulb has too much red and not enough blue. White
// balance corrects this by multiplying R and B channels by reciprocal gains:
//
//   wb_r = red_balance / 256.0    (e.g. RedBalance=512 → wb_r=2.0)
//   wb_g = 1.0                    (green channel is the reference)
//   wb_b = blue_balance / 256.0
//
// ## Colour Matrix
//
// The camera's CFA spectral sensitivities are not the same as the CIE XYZ
// colour matching functions. The 3×3 matrix maps from camera RGB to sRGB,
// compensating for this difference. We use the Panasonic Lumix GH5 matrix as
// a representative hardcode — it works reasonably well for all Micro 4/3 models:
//
//   [ 1.512  -0.518   0.006 ]
//   [-0.202   1.590  -0.388 ]
//   [ 0.055  -0.413   1.358 ]
//
// This matrix is from LibRaw / dcraw.c and targets D65 white point.
//
// ## sRGB Gamma
//
// The sRGB standard uses a piecewise transfer function:
//
//   if x ≤ 0.0031308:  srgb(x) = 12.92 × x
//   else:              srgb(x) = 1.055 × x^(1/2.4) − 0.055
//
// This "lifts" the shadows (linear segment) and compresses the highlights
// (power curve), matching the gamma of most displays.

/// Panasonic Lumix GH5 camera-to-sRGB 3×3 colour matrix.
///
/// Rows: [R_out, G_out, B_out]. Columns: [R_in, G_in, B_in].
/// Applied to white-balanced camera-linear values to get display-ready sRGB.
pub const PANASONIC_COLOR_MATRIX: [[f64; 3]; 3] = [
    [ 1.512, -0.518,  0.006],
    [-0.202,  1.590, -0.388],
    [ 0.055, -0.413,  1.358],
];


/// Apply the full camera-to-display colour pipeline.
///
/// ## Parameters
///
/// * `rgb`          — Demosaiced (R, G, B) tuples in camera-linear u16.
/// * `black_level`  — Sensor black level (noise floor). Typically 240 for 12-bit RW2.
/// * `white_level`  — Sensor saturation level. Typically 4095 (2^12 − 1).
/// * `wb`           — White balance multipliers [wb_r, wb_g, wb_b]. Green is
///                    usually 1.0; R and B are the correction factors.
/// * `color_matrix` — 3×3 camera-to-sRGB matrix. Use [`PANASONIC_COLOR_MATRIX`]
///                    for Panasonic bodies.
///
/// ## Returns
///
/// A `Vec<(u8, u8, u8)>` of sRGB display-ready (R, G, B) bytes, length equal
/// to `rgb.len()`.
// Parameter descriptions wrap with hand-aligned indentation to the `—`;
// the alignment is deliberate literate formatting.
#[allow(clippy::doc_overindented_list_items)]
pub fn apply_color_pipeline(
    rgb: Vec<(u16, u16, u16)>,
    black_level: u32,
    white_level: u32,
    wb: [f64; 3],
    color_matrix: [[f64; 3]; 3],
) -> Vec<(u8, u8, u8)> {
    // The shared pipeline's signature matches directly: single black_level u32,
    // white_level u32, pre-normalised wb [f64;3]. No pre-processing needed.
    image_raw_pipeline::apply_color_pipeline(&rgb, black_level, white_level, wb, color_matrix)
}

/// Compute the white balance multipliers from IFD RedBalance / BlueBalance tags.
///
/// Both tags store (channel / green) × 256 as a u16. Green is normalised to 1.0.
///
/// If either tag is missing (None), defaults to 1.0 for that channel (neutral).
///
/// # Examples
///
/// ```
/// // RedBalance=512, BlueBalance=256 → [2.0, 1.0, 1.0]
/// use image_codec_rw2::color::white_balance_from_tags;
/// let wb = white_balance_from_tags(Some(512), Some(256));
/// assert!((wb[0] - 2.0).abs() < 1e-9);
/// assert!((wb[1] - 1.0).abs() < 1e-9);
/// assert!((wb[2] - 1.0).abs() < 1e-9);
/// ```
pub fn white_balance_from_tags(red_balance: Option<u32>, blue_balance: Option<u32>) -> [f64; 3] {
    let wb_r = red_balance
        .map(|v| v as f64 / 256.0)
        .unwrap_or(1.0);
    let wb_b = blue_balance
        .map(|v| v as f64 / 256.0)
        .unwrap_or(1.0);
    [wb_r, 1.0, wb_b]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wb_from_tags_both_present() {
        let wb = white_balance_from_tags(Some(512), Some(256));
        assert!((wb[0] - 2.0).abs() < 1e-9, "R: expected 2.0, got {}", wb[0]);
        assert!((wb[1] - 1.0).abs() < 1e-9, "G: expected 1.0, got {}", wb[1]);
        assert!((wb[2] - 1.0).abs() < 1e-9, "B: expected 1.0, got {}", wb[2]);
    }

    #[test]
    fn wb_from_tags_missing() {
        // Both None → neutral [1,1,1]
        let wb = white_balance_from_tags(None, None);
        assert_eq!(wb, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn srgb_gamma_zero() {
        // 0.0 → 0.0 (linear segment)
        assert!((image_raw_pipeline::srgb_gamma(0.0)).abs() < 1e-12);
    }

    #[test]
    fn srgb_gamma_one() {
        // 1.0 → 1.0 (both segments agree at endpoints by construction)
        assert!((image_raw_pipeline::srgb_gamma(1.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn color_pipeline_neutral_identity() {
        // With black_level=0, white_level=255 (pretend 8-bit), identity WB and
        // identity colour matrix, the pipeline should be a near-pass-through
        // (only sRGB gamma is applied).
        let identity = [[1.0f64, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let pixels: Vec<(u16, u16, u16)> = vec![(128, 128, 128)];
        let out = apply_color_pipeline(pixels, 0, 255, [1.0, 1.0, 1.0], identity);
        // After normalise (128/255) + gamma, expect a reasonable mid-grey.
        let (r, g, b) = out[0];
        assert_eq!(r, g);
        assert_eq!(g, b);
        // Should be somewhere around 128 (sRGB gamma ~55% at 0.5 linear).
        assert!(r > 80 && r < 200, "Expected mid-grey, got {r}");
    }

    #[test]
    fn color_pipeline_black_pixel() {
        // A pixel exactly at black_level should produce (0, 0, 0).
        let identity = [[1.0f64, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let pixels: Vec<(u16, u16, u16)> = vec![(240, 240, 240)];
        let out = apply_color_pipeline(pixels, 240, 4095, [1.0, 1.0, 1.0], identity);
        assert_eq!(out[0], (0, 0, 0));
    }

    #[test]
    fn color_pipeline_white_pixel() {
        // A pixel at white_level should produce (255, 255, 255) with identity pipeline.
        let identity = [[1.0f64, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let pixels: Vec<(u16, u16, u16)> = vec![(4095, 4095, 4095)];
        let out = apply_color_pipeline(pixels, 240, 4095, [1.0, 1.0, 1.0], identity);
        assert_eq!(out[0], (255, 255, 255));
    }
}
