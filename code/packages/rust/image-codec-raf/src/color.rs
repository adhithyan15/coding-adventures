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
    // ── Step 1: compute the average black level across the four CFA planes ──
    // The four planes are [R, G1, G2, B].  In practice G1 ≈ G2, so averaging
    // all four is a reasonable approximation.
    let black_avg = ((black_level[0] as u64
        + black_level[1] as u64
        + black_level[2] as u64
        + black_level[3] as u64) / 4) as u32;

    // ── Step 2: normalise WB multipliers so G = 1.0 ─────────────────────────
    let g_wb = wb[1] as f64;
    let wb_r = if g_wb > 0.0 { wb[0] as f64 / g_wb } else { 1.0 };
    let wb_g = 1.0_f64;
    let wb_b = if g_wb > 0.0 { wb[2] as f64 / g_wb } else { 1.0 };

    // ── Step 3: dynamic range denominator ───────────────────────────────────
    // After black-level subtraction the maximum value is `white_level − black_avg`.
    // Guard against the degenerate case where white == black.
    let range = if white_level > black_avg {
        (white_level - black_avg) as f64
    } else {
        4095.0 // fallback: assume 12-bit range
    };

    rgb.into_iter().map(|(r_raw, g_raw, b_raw)| {
        // ── subtract black level (floor at 0 to avoid underflow) ────────────
        let r_ped = (r_raw as u32).saturating_sub(black_avg) as f64;
        let g_ped = (g_raw as u32).saturating_sub(black_avg) as f64;
        let b_ped = (b_raw as u32).saturating_sub(black_avg) as f64;

        // ── normalise to [0, 1] ──────────────────────────────────────────────
        let r_lin = (r_ped / range).clamp(0.0, 1.0);
        let g_lin = (g_ped / range).clamp(0.0, 1.0);
        let b_lin = (b_ped / range).clamp(0.0, 1.0);

        // ── apply white balance ──────────────────────────────────────────────
        let r_wb = (r_lin * wb_r).clamp(0.0, 1.0);
        let g_wb2 = (g_lin * wb_g).clamp(0.0, 1.0);
        let b_wb = (b_lin * wb_b).clamp(0.0, 1.0);

        // ── apply 3×3 colour matrix ──────────────────────────────────────────
        // Each output channel is a dot product of [r_wb, g_wb, b_wb] with the
        // corresponding row of the colour matrix.
        let r_srgb = color_matrix[0][0] * r_wb
                   + color_matrix[0][1] * g_wb2
                   + color_matrix[0][2] * b_wb;
        let g_srgb = color_matrix[1][0] * r_wb
                   + color_matrix[1][1] * g_wb2
                   + color_matrix[1][2] * b_wb;
        let b_srgb = color_matrix[2][0] * r_wb
                   + color_matrix[2][1] * g_wb2
                   + color_matrix[2][2] * b_wb;

        // ── apply sRGB gamma and convert to u8 ──────────────────────────────
        let r8 = linear_to_srgb_u8(r_srgb);
        let g8 = linear_to_srgb_u8(g_srgb);
        let b8 = linear_to_srgb_u8(b_srgb);

        (r8, g8, b8)
    }).collect()
}

// ── private helper ────────────────────────────────────────────────────────────

/// Convert a linear-light value in `[0, 1]` to an sRGB gamma-encoded u8.
///
/// The sRGB transfer function (IEC 61966-2-1):
///
/// ```text
/// y = 12.92 × x                   for x ≤ 0.0031308
/// y = 1.055 × x^(1/2.4) − 0.055  for x >  0.0031308
/// ```
///
/// Input is clamped to `[0, 1]` before applying the curve.
#[inline]
fn linear_to_srgb_u8(x: f64) -> u8 {
    let x = x.clamp(0.0, 1.0);
    let y = if x <= 0.003_130_8 {
        12.92 * x
    } else {
        1.055 * x.powf(1.0 / 2.4) - 0.055
    };
    // Scale [0, 1] → [0, 255] with rounding.
    (y * 255.0 + 0.5).min(255.0) as u8
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
