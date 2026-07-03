// # color.rs — White balance, colour matrix, and sRGB gamma pipeline
//
// After demosaicing we have linear camera-RGB values (still sensor-native,
// with a black-level pedestal and no gamma curve).  This module converts them
// into 8-bit sRGB (gamma-corrected) display values via three stages:
//
// ```text
// 1.  Subtract black level  (remove sensor pedestal / dark current)
// 2.  Normalise to [0, 1]   (divide by dynamic range)
// 3.  Apply white balance    (scale R and B to match neutral G)
// 4.  Apply colour matrix    (camera-RGB → sRGB primaries)
// 5.  Apply sRGB gamma       (linear light → perceptual display value)
// 6.  Clip [0, 1] and scale to u8
// ```
//
// ## White balance
//
// The camera's auto-exposure system records the per-channel amplification
// needed to make a neutral (grey) scene look neutral in the output.  The
// recorded multipliers [R_wb, G_wb, B_wb] are raw ADC scale factors.  We
// normalise so that the green channel (the most abundant in the Bayer grid)
// has a multiplier of 1.0:
//
// ```text
// r_norm = R_wb / G_wb
// g_norm = 1.0
// b_norm = B_wb / G_wb
// ```
//
// A pure white pixel (R = G = B at full scale) should remain white after
// WB; the multipliers compensate for the colour temperature of the scene
// illuminant (daylight, tungsten, etc.).
//
// ## Colour matrix
//
// Each camera model has a slightly different spectral response.  The 3×3
// colour matrix maps from the camera's own RGB colour space into the
// standard sRGB colour space (IEC 61966-2-1).  Without this transform,
// colours would be systematically shifted compared to what a calibrated
// monitor displays.
//
// We use a single representative matrix (Fujifilm X-T2) for all cameras
// in v0.1.  Per-model matrices are tracked as a future improvement.
//
// ## sRGB gamma
//
// Human vision is approximately logarithmic: we can distinguish fine
// differences in dark tones but are less sensitive to differences in bright
// tones.  Monitors apply a "gamma" curve to exploit this — storing more
// code values near zero (dark) than near one (bright) wastes bits on
// differences the eye cannot see.
//
// The sRGB transfer function is a piece-wise approximation to a power curve:
//
// ```text
// linear x → display y:
//   y = 12.92 × x                      if x ≤ 0.0031308
//   y = 1.055 × x^(1/2.4) − 0.055     otherwise
// ```
//
// This is the inverse of the encoding that monitors expect.  After applying
// gamma, values in [0, 1] map to 8-bit integers [0, 255].

/// Fujifilm X-T2 colour matrix (camera-native RGB → sRGB).
///
/// Source: dcraw.c, built from the DNG reference data for the X-T2.
/// Rows: [sRGB_R, sRGB_G, sRGB_B] as linear combinations of [cam_R, cam_G, cam_B].
///
/// This matrix is used for all Fujifilm bodies in v0.1.  More accurate
/// per-model matrices are a future improvement (see the `color_matrices`
/// module in the spec's layout, which we fold into this file for simplicity).
pub const FUJI_COLOR_MATRIX: [[f64; 3]; 3] = [
    [ 1.469, -0.491,  0.022],
    [-0.272,  1.559, -0.287],
    [ 0.050, -0.380,  1.330],
];

/// Apply the full colour pipeline to a demosaiced linear-RGB image.
///
/// # Arguments
///
/// * `rgb`          — linear (R, G, B) triples after demosaicing; values in `[0, white_level]`
/// * `black_level`  — per-CFA-plane pedestal `[R, G1, G2, B]`; averaged to one value
/// * `white_level`  — sensor saturation point
/// * `wb`           — raw [R, G, B] WB multipliers from the CFA header
/// * `color_matrix` — 3×3 camera-native-RGB → sRGB matrix
///
/// # Returns
///
/// An `(R, G, B)` 8-bit sRGB value per pixel, clipped to `[0, 255]`.
pub fn apply_color_pipeline(
    rgb: Vec<(u16, u16, u16)>,
    black_level: [u32; 4],
    white_level: u32,
    wb: [u32; 3],
    color_matrix: [[f64; 3]; 3],
) -> Vec<(u8, u8, u8)> {
    // Average the four CFA-plane black levels [R, G1, G2, B].
    // G1 ≈ G2 in practice, so averaging is a reasonable approximation.
    let black_avg = ((black_level[0] as u64
        + black_level[1] as u64
        + black_level[2] as u64
        + black_level[3] as u64) / 4) as u32;

    // Normalise WB so G = 1.0; the shared pipeline expects pre-normalised [f64;3].
    let wb_norm = normalise_wb(wb);

    image_raw_pipeline::apply_color_pipeline(
        &rgb,
        black_avg,
        white_level,
        wb_norm,
        color_matrix,
    )
}

/// Normalise raw WB multipliers [R, G, B] so that G = 1.0.
///
/// Exported so tests can verify the normalisation formula independently.
pub fn normalise_wb(raw_wb: [u32; 3]) -> [f64; 3] {
    let g = raw_wb[1] as f64;
    if g == 0.0 {
        return [1.0, 1.0, 1.0];
    }
    [raw_wb[0] as f64 / g, 1.0, raw_wb[2] as f64 / g]
}
