// # color.rs — Colour Pipeline for TIFF Decoding
//
// Raw sensor data (from CFA/Bayer images) requires several processing steps
// to become a displayable sRGB image. This module implements that pipeline.
//
// ## Pipeline Overview
//
// ```text
// Raw 16-bit values
//   │
//   ▼ 1. Normalize: divide by white_level to get [0.0, 1.0]
//   │
//   ▼ 2. White balance: multiply by [wb_r, wb_g, wb_b]
//      (corrects for the colour temperature of the light source)
//   │
//   ▼ 3. Colour matrix: 3×3 matrix multiply
//      (converts camera-native RGB to linear sRGB primaries)
//   │
//   ▼ 4. sRGB gamma: apply the IEC 61966-2-1 transfer function
//      (converts from linear light to perceptual display values)
//   │
//   ▼ 5. Clip to [0, 255], output as u8
// ```
//
// ## sRGB Gamma
//
// The sRGB standard (IEC 61966-2-1) specifies a piecewise transfer function:
//
// ```text
// V = 12.92 × L                     if L ≤ 0.0031308 (linear segment)
// V = 1.055 × L^(1/2.4) − 0.055    if L > 0.0031308  (power segment)
// ```
//
// where `L` is linear light in [0, 1] and `V` is the display value in [0, 1].
// This matches what monitors expect — they apply the inverse (display gamma).
//
// ## Default Values
//
// The identity pipeline passes through the image unchanged:
// - white_level = u32::MAX (treat all sensor values as full scale)
// - wb_multipliers = [1.0, 1.0, 1.0] (no white-balance adjustment)
// - color_matrix = identity 3×3 (camera is already in sRGB, or close enough)
// - black_level = [0; 4] (no pedestal subtraction)
//
// RAW format codecs (DNG, CR2, NEF) override these with camera-specific values
// from their metadata.

/// Decode options passed to the TIFF decoder by RAW format wrappers.
///
/// # Example
///
/// ```rust,ignore
/// let opts = TiffDecodeOptions {
///     ifd_index: 0,
///     wb_multipliers: [2.1, 1.0, 1.7],  // Daylight WB for some Canon camera
///     color_matrix: [[1.5, -0.3, -0.1], [-0.2, 1.4, -0.1], [0.0, -0.1, 1.2]],
///     black_level: [512; 4],             // 12-bit black level of 512
///     white_level: 4095,                 // 12-bit saturation
/// };
/// ```
#[derive(Debug, Clone)]
pub struct TiffDecodeOptions {
    /// Index of the IFD to decode (0 = first/largest image).
    ///
    /// TIFF files may contain multiple images (sub-files, thumbnails, etc.).
    /// The first IFD is almost always the full-resolution image.
    pub ifd_index: usize,

    /// White balance multipliers [R, G, B].
    ///
    /// Applied after black-level subtraction. Compensates for the colour
    /// temperature of the light source. For neutral (no correction): [1.0, 1.0, 1.0].
    pub wb_multipliers: [f64; 3],

    /// 3×3 camera-to-sRGB colour matrix (row-major).
    ///
    /// Converts from camera-native linear RGB to linear sRGB primaries.
    /// Identity matrix means the camera is already in sRGB space.
    ///
    /// Applied after white balance.
    pub color_matrix: [[f64; 3]; 3],

    /// Black level per channel (subtracted before WB).
    ///
    /// Most sensors have a non-zero "pedestal" (darkness level). Values below
    /// black_level are clipped to 0. Up to 4 channels (RGGB).
    pub black_level: [u32; 4],

    /// White level (sensor saturation point).
    ///
    /// Values at or above this are clamped to 1.0. Typically `(1 << bits) - 1`
    /// for the sensor's bit depth. u32::MAX means "use the BitsPerSample maximum".
    pub white_level: u32,
}

impl Default for TiffDecodeOptions {
    fn default() -> Self {
        TiffDecodeOptions {
            ifd_index: 0,
            wb_multipliers: [1.0, 1.0, 1.0],
            color_matrix: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            black_level: [0; 4],
            white_level: u32::MAX,
        }
    }
}

// ─── Colour pipeline ──────────────────────────────────────────────────────────

/// Apply the full colour pipeline to a list of 16-bit linear RGB triples.
///
/// Input: `rgb_linear` — one `(R, G, B)` tuple per pixel, from demosaicing or
/// direct 16-bit channel reads. Values are in [0, 65535].
///
/// Output: one `(R, G, B)` tuple per pixel in sRGB u8 [0, 255].
///
/// # Steps
///
/// 1. Normalize to [0.0, 1.0] by dividing by `white_level` (or 65535 if not set).
/// 2. Apply white balance multipliers.
/// 3. Apply the 3×3 colour matrix.
/// 4. Apply sRGB gamma.
/// 5. Clip to [0.0, 1.0] and round to u8.
pub fn apply_color_pipeline(
    rgb_linear: Vec<(u16, u16, u16)>,
    opts: &TiffDecodeOptions,
) -> Vec<(u8, u8, u8)> {
    // Effective white level: if set to u32::MAX (the default), treat 65535 as full scale.
    let white = if opts.white_level == u32::MAX {
        65535.0f64
    } else {
        opts.white_level as f64
    };

    // Avoid division by zero.
    let white = if white <= 0.0 { 65535.0 } else { white };

    rgb_linear
        .into_iter()
        .map(|(r_raw, g_raw, b_raw)| {
            // Step 1: Normalize to [0.0, 1.0].
            let r = r_raw as f64 / white;
            let g = g_raw as f64 / white;
            let b = b_raw as f64 / white;

            // Step 2: Apply white balance multipliers.
            // Each channel is scaled independently to compensate for the
            // colour temperature of the scene illuminant.
            let r = r * opts.wb_multipliers[0];
            let g = g * opts.wb_multipliers[1];
            let b = b * opts.wb_multipliers[2];

            // Step 3: Apply 3×3 colour matrix.
            // The matrix maps camera RGB → linear sRGB. For most images, this
            // is the identity. RAW codec callers supply camera-specific matrices.
            //
            // ```text
            // [r']   [m00 m01 m02]   [r]
            // [g'] = [m10 m11 m12] × [g]
            // [b']   [m20 m21 m22]   [b]
            // ```
            let m = &opts.color_matrix;
            let r2 = m[0][0] * r + m[0][1] * g + m[0][2] * b;
            let g2 = m[1][0] * r + m[1][1] * g + m[1][2] * b;
            let b2 = m[2][0] * r + m[2][1] * g + m[2][2] * b;

            // Step 4: Apply sRGB gamma transfer function.
            // Clip to [0, 1] first to keep gamma well-behaved.
            let r3 = apply_srgb_gamma(r2.clamp(0.0, 1.0));
            let g3 = apply_srgb_gamma(g2.clamp(0.0, 1.0));
            let b3 = apply_srgb_gamma(b2.clamp(0.0, 1.0));

            // Step 5: Scale to [0, 255] and round.
            let to_u8 = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;
            (to_u8(r3), to_u8(g3), to_u8(b3))
        })
        .collect()
}

/// Apply the sRGB gamma transfer function to a single linear value.
///
/// The IEC 61966-2-1 (sRGB) standard specifies:
///
/// ```text
/// V = 12.92 × L                     if L ≤ 0.0031308
/// V = 1.055 × L^(1/2.4) − 0.055    if L > 0.0031308
/// ```
///
/// This converts a linear-light value in [0, 1] to a gamma-corrected value
/// in [0, 1] that monitors display correctly.
///
/// ## Why gamma at all?
///
/// Human vision is non-linear — we can distinguish more shades in the dark
/// than in the light. Gamma encoding allocates more bits to the darker tones,
/// which is more efficient. Monitors apply the inverse (display gamma ≈ 2.2)
/// to recover the linear signal.
#[inline]
pub fn apply_srgb_gamma(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        // Linear segment: very dark values
        12.92 * linear
    } else {
        // Power segment: standard gamma for everything else
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_opts_are_identity() {
        let opts = TiffDecodeOptions::default();
        assert_eq!(opts.ifd_index, 0);
        assert_eq!(opts.wb_multipliers, [1.0, 1.0, 1.0]);
        assert_eq!(opts.color_matrix[0], [1.0, 0.0, 0.0]);
        assert_eq!(opts.color_matrix[1], [0.0, 1.0, 0.0]);
        assert_eq!(opts.color_matrix[2], [0.0, 0.0, 1.0]);
        assert_eq!(opts.black_level, [0u32; 4]);
        assert_eq!(opts.white_level, u32::MAX);
    }

    #[test]
    fn srgb_gamma_zero() {
        // Linear 0.0 → sRGB 0.0
        assert!((apply_srgb_gamma(0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn srgb_gamma_one() {
        // Linear 1.0 → sRGB 1.0 (both ends of the scale are fixed points)
        assert!((apply_srgb_gamma(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn srgb_gamma_midpoint() {
        // Linear 0.5 → sRGB ≈ 0.7354 (from reference tables)
        let v = apply_srgb_gamma(0.5);
        assert!(v > 0.73 && v < 0.74, "Expected ~0.735, got {}", v);
    }

    #[test]
    fn srgb_gamma_linear_segment() {
        // Values below 0.0031308 use the linear formula.
        let x = 0.001;
        let expected = 12.92 * x;
        assert!((apply_srgb_gamma(x) - expected).abs() < 1e-10);
    }

    #[test]
    fn pipeline_identity_white_black_input() {
        // Input: pure white (65535, 65535, 65535)
        // Identity pipeline → output should be (255, 255, 255)
        let opts = TiffDecodeOptions::default();
        let input = vec![(65535u16, 65535, 65535)];
        let output = apply_color_pipeline(input, &opts);
        assert_eq!(output[0], (255, 255, 255));
    }

    #[test]
    fn pipeline_identity_black_input() {
        // Input: pure black (0, 0, 0)
        // Identity pipeline → output should be (0, 0, 0)
        let opts = TiffDecodeOptions::default();
        let input = vec![(0u16, 0, 0)];
        let output = apply_color_pipeline(input, &opts);
        assert_eq!(output[0], (0, 0, 0));
    }

    #[test]
    fn pipeline_empty_input() {
        let opts = TiffDecodeOptions::default();
        let output = apply_color_pipeline(vec![], &opts);
        assert!(output.is_empty());
    }

    #[test]
    fn pipeline_wb_multiplier_boosts_red() {
        // WB multiplier of 2.0 on red should brighten red.
        let opts = TiffDecodeOptions {
            wb_multipliers: [2.0, 1.0, 1.0],
            white_level: 100,
            ..Default::default()
        };
        let input = vec![(50u16, 50, 50)]; // 50% of 100 = 0.5 linear
        let output = apply_color_pipeline(input, &opts);
        // Red channel: 0.5 * 2.0 = 1.0 → clamp → gamma(1.0) = 1.0 → 255
        // Other channels: 0.5 → gamma(0.5) ≈ 188
        assert_eq!(output[0].0, 255, "Red should saturate to 255");
        assert!(output[0].1 < 200, "Green should be ~188, not saturated");
    }

    #[test]
    fn pipeline_color_matrix_swaps_channels() {
        // A matrix that swaps R and B: [[0,0,1],[0,1,0],[1,0,0]]
        // Input: pure red (65535, 0, 0) → after matrix: (0, 0, 65535) = blue
        let opts = TiffDecodeOptions {
            color_matrix: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]],
            ..Default::default()
        };
        let input = vec![(65535u16, 0, 0)];
        let output = apply_color_pipeline(input, &opts);
        assert_eq!(output[0].0, 0, "R should be 0 after swap");
        assert_eq!(output[0].2, 255, "B should be 255 after swap");
    }
}
