// # tags.rs — DNG Private Tag Constants
//
// Adobe's Digital Negative (DNG) format extends TIFF 6.0 with private tags
// in the range 50700–51100+. These constants name the most important ones.
//
// All tag IDs are `u16` values that appear in the `tag` field of an IFD entry.
//
// ## Why private tags?
//
// TIFF allows vendors to register private tag ranges. Adobe registered the
// range 50706–51xx for DNG-specific metadata. A standard TIFF reader can
// safely skip these tags; a DNG-aware reader extracts colour calibration data.
//
// ## Tag reference table
//
// | Tag   | Constant                | Type         | Purpose                         |
// |-------|-------------------------|--------------|---------------------------------|
// | 50706 | DNG_VERSION             | BYTE[4]      | DNG spec version                |
// | 50708 | UNIQUE_CAMERA_MODEL     | ASCII        | Camera model string             |
// | 50714 | BLACK_LEVEL             | RATIONAL+    | Sensor black level              |
// | 50717 | WHITE_LEVEL             | SHORT/LONG   | Sensor saturation point         |
// | 50721 | COLOR_MATRIX_1          | SRATIONAL[9] | XYZ D50 → camera (illuminant 1) |
// | 50722 | COLOR_MATRIX_2          | SRATIONAL[9] | XYZ D50 → camera (illuminant 2) |
// | 50728 | AS_SHOT_NEUTRAL         | RATIONAL[3]  | White balance neutrals          |
// | 50778 | CALIBRATION_ILLUMINANT_1| SHORT        | Standard illuminant code        |
// | 50829 | ACTIVE_AREA             | LONG[4]      | Sensor active region            |
// | 50879 | FORWARD_MATRIX_1        | SRATIONAL[9] | Camera → XYZ D50 (illuminant 1) |
// | 50880 | FORWARD_MATRIX_2        | SRATIONAL[9] | Camera → XYZ D50 (illuminant 2) |

/// DNG version encoded as 4 bytes, e.g. [1, 6, 0, 0] = DNG 1.6.
pub const DNG_VERSION: u16 = 50706;

/// Unique camera model string — identifies the sensor calibration target.
pub const UNIQUE_CAMERA_MODEL: u16 = 50708;

/// Sensor black level.
///
/// Values below this threshold are pure shadow — signal that should map to
/// digital zero. Stored as RATIONAL (per CFA plane pattern) or LONG.
/// This crate uses a single scalar extracted from the first value.
pub const BLACK_LEVEL: u16 = 50714;

/// Sensor white level (saturation point).
///
/// The raw sensor value that corresponds to pure white (fully saturated).
/// Stored as SHORT or LONG. Typical values: 4095 (12-bit), 16383 (14-bit),
/// 65535 (16-bit). The TIFF IFD coder stashes it in `extra_tags` for us.
pub const WHITE_LEVEL: u16 = 50717;

/// ColorMatrix1 — maps XYZ D50 colour to camera raw values under illuminant 1.
///
/// Direction: XYZ → camera. To convert camera values to XYZ (the path we need),
/// take the inverse. Stored as SRATIONAL[9] (nine signed rational numbers in
/// row-major order): [[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]].
///
/// When ForwardMatrix1 is present, prefer it (it's the forward direction: camera
/// → XYZ D50, no inversion needed). Fall back to ColorMatrix1 otherwise.
pub const COLOR_MATRIX_1: u16 = 50721;

/// ColorMatrix2 — maps XYZ D50 to camera raw values under illuminant 2.
///
/// Same format as COLOR_MATRIX_1 but calibrated for a different illuminant
/// (often D50 vs. A). Version 0.1 uses only illuminant 1.
pub const COLOR_MATRIX_2: u16 = 50722;

/// AsShotNeutral — white balance triple.
///
/// RATIONAL[3] = [R_neutral, G_neutral, B_neutral]. The sensor read these
/// values for a neutral grey card under the shot illuminant. White-balance
/// multipliers are derived as `1 / neutral`, then normalised so that G = 1.
pub const AS_SHOT_NEUTRAL: u16 = 50728;

/// CalibrationIlluminant1 — standard illuminant code for ColorMatrix1/ForwardMatrix1.
///
/// Standard codes: 17 = D65, 21 = D50, 1 = Daylight, 23 = D55, 20 = D75.
/// Version 0.1 reads this tag but does not interpolate between illuminants.
pub const CALIBRATION_ILLUMINANT_1: u16 = 50778;

/// ActiveArea — the rectangular region of the sensor that contains valid image data.
///
/// LONG[4] = [top, left, bottom, right] in sensor pixel coordinates. Rows/cols
/// outside this area are optical black (used for black-level calibration by the
/// camera firmware). We pass this to the TIFF decoder as crop coordinates.
pub const ACTIVE_AREA: u16 = 50829;

/// ForwardMatrix1 — maps camera raw values to XYZ D50 under illuminant 1.
///
/// Direction: camera → XYZ D50 (forward, so no inversion needed). Stored as
/// SRATIONAL[9]. Then combine with the Bradford-adapted XYZ D50 → sRGB matrix:
/// `camera_to_sRGB = XYZ_D50_TO_SRGB × ForwardMatrix1`.
///
/// Prefer ForwardMatrix over ColorMatrix when both are present — the forward
/// matrix is an explicit characterisation of the sensor's spectral response,
/// rather than an inverted colorimetric measurement.
pub const FORWARD_MATRIX_1: u16 = 50879;

/// ForwardMatrix2 — maps camera raw values to XYZ D50 under illuminant 2.
///
/// Same format as FORWARD_MATRIX_1 but for a second illuminant. Version 0.1
/// uses only illuminant 1 (FORWARD_MATRIX_1).
pub const FORWARD_MATRIX_2: u16 = 50880;
