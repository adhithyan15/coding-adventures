// # pipeline.rs — RAW Colour Development Pipeline
//
// Camera sensor data is not RGB in the sRGB sense. The Bayer array captures
// light through coloured filters, and the resulting 16-bit values are in a
// camera-specific native colour space with a non-zero "black level" pedestal.
// Four steps turn raw sensor values into a displayable sRGB image:
//
// ## Step-by-step
//
// ### 1. Black-level subtraction and normalisation
//
// Sensors have a "dark current" — they output non-zero values even when no
// light hits them. The black level is the value corresponding to absolute
// darkness. Subtracting it and normalising by the white level maps the
// sensor's dynamic range to [0.0, 1.0]:
//
// ```text
// effective_white = white_level - black_level   (at least 1 to avoid ÷0)
// r_norm = saturating_sub(r_raw, black_level) as f64 / effective_white
// ```
//
// `saturating_sub` ensures negative results clamp to 0 rather than wrapping.
//
// ### 2. White balance
//
// The scene illuminant (sun, tungsten, LED, etc.) shifts the colour of
// everything in the frame. White balance multipliers correct this by
// boosting the channels that the illuminant suppressed:
//
// ```text
// r_wb = r_norm * wb[0]
// g_wb = g_norm * wb[1]
// b_wb = b_norm * wb[2]
// ```
//
// For daylight, red and blue are typically boosted (wb[0] > 1, wb[2] > 1)
// while green is kept at 1.0 (green is the reference channel in most
// camera colour science).
//
// ### 3. Camera-to-sRGB colour matrix
//
// Camera sensors use their own native primaries (determined by the physical
// dye filters). A 3×3 matrix converts from camera-native linear RGB to
// standard linear sRGB primaries (IEC 61966-2-1 primary chromaticities):
//
// ```text
// [r', g', b'] = color_matrix × [r_wb, g_wb, b_wb]
// ```
//
// This matrix is camera-specific. RAW codec callers supply it from the
// camera manufacturer's calibration data (TIFF ColorMatrix tag, DNG
// ColorMatrix1/ForwardMatrix1, etc.). The identity matrix is valid when
// the camera is already calibrated to sRGB primaries.
//
// After the matrix multiply, each channel is clamped to [0.0, 1.0] to
// prevent negative values or values above 1 from corrupting the gamma step.
//
// ### 4. sRGB gamma
//
// Linear light values are physically accurate but perceptually non-uniform
// — equal steps in linear light don't look equally spaced to the human
// eye. The sRGB transfer function (IEC 61966-2-1) maps linear light to
// display encoding that monitors decode correctly:
//
// ```text
// r_out = srgb_gamma(r') * 255, rounded and clamped to [0, 255]
// ```

use crate::gamma::srgb_gamma;
use crate::matrix::mat3x3_mul;

/// Apply the full four-stage RAW colour development pipeline.
///
/// # Parameters
///
/// - `pixels` — linear RGB triples from demosaicing or multi-channel reads.
///   Values are 16-bit unsigned, range [0, 65535].
/// - `black_level` — sensor pedestal (absolute darkness level). Subtracted
///   before normalisation. Commonly 512 for 12-bit sensors, 4096 for 14-bit.
/// - `white_level` — sensor saturation point (full-scale value). Typically
///   `(1 << bits) - 1`: 4095 for 12-bit, 16383 for 14-bit. Pass `u32::MAX`
///   to treat 65535 as full scale.
/// - `wb` — white balance multipliers [R, G, B]. Neutral = [1.0, 1.0, 1.0].
/// - `color_matrix` — 3×3 camera-to-sRGB colour matrix, row-major.
///   Identity = already in sRGB colour space.
///
/// # Returns
///
/// One `(R, G, B)` u8 triple per input pixel, in sRGB.
///
/// # Example
///
/// ```
/// use image_raw_pipeline::apply_color_pipeline;
///
/// // Identity pipeline: pure white (65535, 65535, 65535) → (255, 255, 255).
/// let white = vec![(65535u16, 65535, 65535)];
/// let id_matrix = [[1.0f64,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]];
/// let out = apply_color_pipeline(&white, 0, 65535, [1.0, 1.0, 1.0], id_matrix);
/// assert_eq!(out[0], (255, 255, 255));
/// ```
pub fn apply_color_pipeline(
    pixels:       &[(u16, u16, u16)],
    black_level:  u32,
    white_level:  u32,
    wb:           [f64; 3],
    color_matrix: [[f64; 3]; 3],
) -> Vec<(u8, u8, u8)> {
    // Effective white: the range from black to saturation.
    // Clamp to at least 1 to avoid division by zero for degenerate inputs.
    let effective_white = (white_level.saturating_sub(black_level) as f64).max(1.0);

    pixels.iter().map(|&(r_raw, g_raw, b_raw)| {
        // Stage 1: subtract black level, normalise to [0, 1].
        let r_norm = (r_raw as u32).saturating_sub(black_level) as f64 / effective_white;
        let g_norm = (g_raw as u32).saturating_sub(black_level) as f64 / effective_white;
        let b_norm = (b_raw as u32).saturating_sub(black_level) as f64 / effective_white;

        // Stage 2: white balance — multiply per channel.
        let r_wb = r_norm * wb[0];
        let g_wb = g_norm * wb[1];
        let b_wb = b_norm * wb[2];

        // Stage 3: camera-to-sRGB colour matrix.
        let [r2, g2, b2] = mat3x3_mul(&color_matrix, [r_wb, g_wb, b_wb]);

        // Clamp to [0, 1] after the matrix — prevents gamma of negative values.
        let r2 = r2.clamp(0.0, 1.0);
        let g2 = g2.clamp(0.0, 1.0);
        let b2 = b2.clamp(0.0, 1.0);

        // Stage 4: sRGB gamma + scale to u8.
        let to_u8 = |v: f64| (srgb_gamma(v) * 255.0).round().clamp(0.0, 255.0) as u8;
        (to_u8(r2), to_u8(g2), to_u8(b2))
    }).collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn id_matrix() -> [[f64; 3]; 3] {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    fn neutral_wb() -> [f64; 3] { [1.0, 1.0, 1.0] }

    // ── Identity pipeline ─────────────────────────────────────────────────

    #[test]
    fn empty_input_returns_empty() {
        let out = apply_color_pipeline(&[], 0, 65535, neutral_wb(), id_matrix());
        assert!(out.is_empty());
    }

    #[test]
    fn pure_white_maps_to_255() {
        let out = apply_color_pipeline(
            &[(65535, 65535, 65535)], 0, 65535, neutral_wb(), id_matrix()
        );
        assert_eq!(out[0], (255, 255, 255));
    }

    #[test]
    fn pure_black_maps_to_0() {
        let out = apply_color_pipeline(
            &[(0, 0, 0)], 0, 65535, neutral_wb(), id_matrix()
        );
        assert_eq!(out[0], (0, 0, 0));
    }

    #[test]
    fn black_level_subtraction() {
        // black_level=32768, white_level=65535
        // Input (32768, 32768, 32768) → after subtraction: (0, 0, 0) → output (0,0,0)
        let out = apply_color_pipeline(
            &[(32768, 32768, 32768)], 32768, 65535, neutral_wb(), id_matrix()
        );
        assert_eq!(out[0], (0, 0, 0));
    }

    #[test]
    fn below_black_level_clamps_to_zero() {
        // Input below black level: saturating_sub → 0.
        let out = apply_color_pipeline(
            &[(100, 100, 100)], 512, 4095, neutral_wb(), id_matrix()
        );
        assert_eq!(out[0], (0, 0, 0));
    }

    #[test]
    fn white_level_normalization_12bit() {
        // For a 12-bit sensor: black=0, white=4095.
        // Input (4095, 4095, 4095) = full scale → should map to (255, 255, 255).
        let out = apply_color_pipeline(
            &[(4095, 4095, 4095)], 0, 4095, neutral_wb(), id_matrix()
        );
        assert_eq!(out[0], (255, 255, 255));
    }

    // ── White balance ─────────────────────────────────────────────────────

    #[test]
    fn wb_boost_red_to_saturation() {
        // WB multiplier 2.0 on R: half-scale input (32768) × 2.0 = 1.0 → 255.
        let out = apply_color_pipeline(
            &[(32768, 32768, 32768)], 0, 65535,
            [2.0, 1.0, 1.0], id_matrix()
        );
        // Red should saturate to 255; green/blue at half-scale (~188 after gamma).
        assert_eq!(out[0].0, 255, "Red should saturate");
        assert!(out[0].1 < 200, "Green should not saturate: {}", out[0].1);
    }

    #[test]
    fn wb_neutral_midgrey_gives_same_rgb() {
        // Neutral WB + identity matrix: R=G=B=half-scale → all channels equal.
        let out = apply_color_pipeline(
            &[(32768, 32768, 32768)], 0, 65535, neutral_wb(), id_matrix()
        );
        let (r, g, b) = out[0];
        assert_eq!(r, g, "R != G for neutral midgrey");
        assert_eq!(g, b, "G != B for neutral midgrey");
    }

    // ── Colour matrix ─────────────────────────────────────────────────────

    #[test]
    fn color_matrix_swaps_r_and_b() {
        // Matrix [[0,0,1],[0,1,0],[1,0,0]] swaps R and B.
        // Input: pure red (65535, 0, 0) → after swap: (0, 0, 65535) = pure blue.
        let swap = [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]];
        let out = apply_color_pipeline(
            &[(65535, 0, 0)], 0, 65535, neutral_wb(), swap
        );
        assert_eq!(out[0].0, 0,   "R should be 0 after R↔B swap");
        assert_eq!(out[0].1, 0,   "G should be 0 after R↔B swap");
        assert_eq!(out[0].2, 255, "B should be 255 after R↔B swap");
    }

    #[test]
    fn color_matrix_identity_preserves_channels() {
        let out = apply_color_pipeline(
            &[(65535, 0, 0)], 0, 65535, neutral_wb(), id_matrix()
        );
        // Pure red stays pure red.
        assert_eq!(out[0].0, 255);
        assert_eq!(out[0].1, 0);
        assert_eq!(out[0].2, 0);
    }

    #[test]
    fn pipeline_multiple_pixels() {
        let pixels = vec![
            (65535_u16, 0, 0),     // red
            (0, 65535, 0),         // green
            (0, 0, 65535),         // blue
        ];
        let out = apply_color_pipeline(&pixels, 0, 65535, neutral_wb(), id_matrix());
        assert_eq!(out[0], (255, 0, 0));
        assert_eq!(out[1], (0, 255, 0));
        assert_eq!(out[2], (0, 0, 255));
    }

    #[test]
    fn pipeline_clamps_wb_overexposure_to_255() {
        // WB multiplier 3.0 on an already-bright pixel: still clamps to 255, not >255.
        let out = apply_color_pipeline(
            &[(50000, 50000, 50000)], 0, 65535,
            [3.0, 3.0, 3.0], id_matrix()
        );
        assert_eq!(out[0], (255, 255, 255));
    }

    #[test]
    fn pipeline_large_image_no_panic() {
        // 1000 pixels — verify no panic and correct length.
        let pixels: Vec<(u16, u16, u16)> = (0..1000).map(|i| {
            let v = ((i * 65) % 65535) as u16;
            (v, v, v)
        }).collect();
        let out = apply_color_pipeline(&pixels, 0, 65535, neutral_wb(), id_matrix());
        assert_eq!(out.len(), 1000);
    }
}
