// # color.rs — DNG Colour Pipeline
//
// DNG embeds all the data needed to reconstruct correct colours from raw sensor
// values. This module implements the three steps of the DNG colour pipeline:
//
// 1. **White balance** — scale R, G, B channels by the reciprocals of the
//    as-shot neutral values so that a neutral grey maps to equal R = G = B.
//
// 2. **Camera → XYZ D50** — apply a 3×3 matrix that maps linear camera-space
//    RGB to CIE XYZ under the D50 illuminant (the DNG standard white point).
//    Two routes:
//    - ForwardMatrix (preferred): maps camera → XYZ D50 directly.
//    - ColorMatrix (fallback): maps XYZ D50 → camera; we invert it.
//
// 3. **XYZ D50 → linear sRGB** — apply a Bradford-adapted matrix to convert
//    from D50 to D65 (the sRGB white point), producing linear sRGB values.
//    The sRGB gamma curve is applied downstream (in image-codec-tiff's colour
//    pipeline, not here).
//
// ## Reference
//
// The XYZ D50 → sRGB matrix is derived from the ICC colour management
// specification using the Bradford chromatic-adaptation transform. The values
// here match the widely-cited Colour-science Python library constants.

// ─── XYZ D50 → linear sRGB (Bradford-adapted, D65 white point) ───────────────
//
// This is the combined D50→D65 Bradford adaptation followed by the standard
// XYZ→sRGB matrix. Applied after ForwardMatrix (which takes camera RGB to XYZ D50).
//
// Row major: out[i] = sum_k(M[i][k] * in[k])
// Input:  [X, Y, Z] in XYZ D50
// Output: [R, G, B] in linear sRGB (D65), not yet gamma-encoded
//
// Source: ICC colour management, Bradford chromatic adaptation
//         See also: http://www.brucelindbloom.com/index.html?Eqn_ChromAdapt.html
pub const XYZ_D50_TO_SRGB: [[f64; 3]; 3] = [
    [ 3.1338561, -1.6168667, -0.4906146],
    [-0.9787684,  1.9161415,  0.0334540],
    [ 0.0719453, -0.2289914,  1.4052427],
];

// ─── White balance from AsShotNeutral ────────────────────────────────────────

/// Compute white-balance multipliers from the DNG `AsShotNeutral` triple.
///
/// ## What is AsShotNeutral?
///
/// The camera reads a neutral grey card and records [R_n, G_n, B_n] — the raw
/// sensor values (normalised to [0, 1]) that a grey target produced under the
/// shot illuminant. To "white-balance" an image means to scale each channel so
/// that the grey card comes out equal in all three:
///
/// ```text
/// WB_R = 1 / R_n
/// WB_G = 1 / G_n
/// WB_B = 1 / B_n
/// ```
///
/// ## Normalisation
///
/// We normalise so that the green multiplier is 1.0 (since green carries the
/// most luminance information in a Bayer sensor — there are twice as many green
/// photosites as red or blue):
///
/// ```text
/// norm = WB_G = 1 / G_n
/// WB_R_final = WB_R / norm = G_n / R_n
/// WB_G_final = 1.0
/// WB_B_final = WB_B / norm = G_n / B_n
/// ```
///
/// ## Edge cases
///
/// - Empty `neutrals` slice → return identity `[1.0, 1.0, 1.0]`
/// - G channel zero → avoid divide-by-zero, return `[1.0, 1.0, 1.0]`
///
/// ## Example
///
/// A typical AsShotNeutral for daylight might be [0.5, 1.0, 0.7]:
/// ```text
/// WB = [1/0.5, 1/1.0, 1/0.7] = [2.0, 1.0, 1.43]
/// After normalise: [2.0, 1.0, 1.43] (G was already 1.0)
/// ```
pub fn wb_from_as_shot_neutral(neutrals: &[f64]) -> [f64; 3] {
    if neutrals.len() < 3 || neutrals[1] == 0.0 {
        return [1.0, 1.0, 1.0];
    }
    let r = 1.0 / neutrals[0];
    let g = 1.0 / neutrals[1];
    let b = 1.0 / neutrals[2];
    // Normalise so green multiplier = 1.0
    let norm = g;
    [r / norm, 1.0, b / norm]
}

// ─── 3×3 matrix multiplication ───────────────────────────────────────────────

/// Multiply two 3×3 matrices: `C = A × B`.
///
/// Standard row-by-column matrix multiplication. Used to combine
/// ForwardMatrix (camera → XYZ D50) with XYZ_D50_TO_SRGB to get a single
/// camera → sRGB matrix.
///
/// ## Algorithm
///
/// For each output element `C[i][j]`:
///   ```text
///   C[i][j] = sum over k of A[i][k] * B[k][j]
///   ```
///
/// This is O(27) floating-point multiplications for 3×3.
pub fn matrix_multiply(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    // Decompose B column-by-column: each column of A×B is mat3x3_mul(A, col_j(B)).
    // col_j(B) = [B[0][j], B[1][j], B[2][j]]  (B is row-major)
    let c0 = image_raw_pipeline::mat3x3_mul(a, [b[0][0], b[1][0], b[2][0]]);
    let c1 = image_raw_pipeline::mat3x3_mul(a, [b[0][1], b[1][1], b[2][1]]);
    let c2 = image_raw_pipeline::mat3x3_mul(a, [b[0][2], b[1][2], b[2][2]]);
    // Reassemble: result[i][j] = c_j[i]
    [
        [c0[0], c1[0], c2[0]],
        [c0[1], c1[1], c2[1]],
        [c0[2], c1[2], c2[2]],
    ]
}

// ─── ForwardMatrix path ───────────────────────────────────────────────────────

/// Compute the combined camera → sRGB matrix via ForwardMatrix.
///
/// `ForwardMatrix` maps camera RGB → XYZ D50. Then `XYZ_D50_TO_SRGB` maps
/// XYZ D50 → linear sRGB. The combined matrix is:
///
/// ```text
/// camera_to_sRGB = XYZ_D50_TO_SRGB × ForwardMatrix
/// ```
///
/// ## Why ForwardMatrix (not ColorMatrix)?
///
/// `ForwardMatrix` is an explicit characterisation of the sensor's spectral
/// response — it's the "good" direction (no inversion needed). ColorMatrix
/// is the inverse direction (XYZ → camera) and must be inverted before use.
/// ForwardMatrix tends to be more numerically stable.
///
/// ## Example
///
/// ```rust
/// use image_codec_dng::color::camera_to_srgb_via_forward;
/// let id = [[1.0f64,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]];
/// let m = camera_to_srgb_via_forward(&id);
/// // Result = XYZ_D50_TO_SRGB (ForwardMatrix=identity means camera IS XYZ D50)
/// assert_eq!(m.len(), 3);
/// ```
pub fn camera_to_srgb_via_forward(forward: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    matrix_multiply(&XYZ_D50_TO_SRGB, forward)
}

// ─── 3×3 matrix inversion ─────────────────────────────────────────────────────

/// Invert a 3×3 matrix using the cofactor (adjugate) method.
///
/// ## Mathematical background
///
/// The inverse of a matrix M is:  inv(M) = adj(M) / det(M)
///
/// Where `adj(M)` is the transpose of the cofactor matrix, and `det(M)` is
/// the determinant. The determinant is computed by cofactor expansion along
/// the first row:
///
/// ```text
/// det = M[0][0] * (M[1][1]*M[2][2] - M[1][2]*M[2][1])
///     - M[0][1] * (M[1][0]*M[2][2] - M[1][2]*M[2][0])
///     + M[0][2] * (M[1][0]*M[2][1] - M[1][1]*M[2][0])
/// ```
///
/// ## When is this needed?
///
/// When a DNG file contains `ColorMatrix1` (direction: XYZ D50 → camera) but
/// no `ForwardMatrix1`. We need the camera → XYZ D50 direction, so we invert
/// `ColorMatrix1`. Then we multiply by `XYZ_D50_TO_SRGB` to get camera → sRGB.
///
/// ## Singular matrices
///
/// If `|det| < 1e-10`, the matrix is singular (non-invertible) — returns `None`.
/// The caller should fall back to an identity matrix in that case.
///
/// ## Example
///
/// ```rust
/// use image_codec_dng::color::invert_3x3;
/// let id = [[1.0f64,0.0,0.0],[0.0,1.0,0.0],[0.0,0.0,1.0]];
/// let inv = invert_3x3(&id).unwrap();
/// // Identity inverse is identity
/// assert!((inv[0][0] - 1.0).abs() < 1e-9);
/// ```
pub fn invert_3x3(m: &[[f64; 3]; 3]) -> Option<[[f64; 3]; 3]> {
    image_raw_pipeline::invert_3x3(m)
}
