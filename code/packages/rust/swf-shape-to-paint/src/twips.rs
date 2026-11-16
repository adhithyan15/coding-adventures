//! Twips coordinate conversions, checks, and zero-allocation math.

#![deny(unsafe_code)]

use crate::error::ShapeError;

/// Twips per logical screen pixel ($1\text{ px} = 20\text{ twips}$).
pub const TWIPS_PER_PIXEL: f64 = 20.0;
/// Pixels per twip ($1\text{ twip} = 0.05\text{ px}$).
pub const PIXELS_PER_TWIP: f64 = 0.05;

/// Minimum safe twips coordinate (-2^30).
pub const MIN_SAFE_TWIPS: i32 = -1_073_741_824;
/// Maximum safe twips coordinate (2^30 - 1).
pub const MAX_SAFE_TWIPS: i32 = 1_073_741_823;

/// Validates that a twips coordinate is within safe bounds.
#[inline]
pub fn validate_twips(t: i32) -> Result<bool, ShapeError> {
    if (MIN_SAFE_TWIPS..=MAX_SAFE_TWIPS).contains(&t) {
        Ok(true)
    } else {
        Err(ShapeError::ArithmeticOverflow)
    }
}

/// Converts a twips coordinate to floating-point pixels.
#[inline]
pub fn twips_to_pixels_f64(twips: i32) -> f64 {
    twips as f64 / TWIPS_PER_PIXEL
}

/// Converts floating-point pixels to twips (rounded to nearest integer).
#[inline]
pub fn pixels_to_twips_i32(pixels: f64) -> i32 {
    let rounded = (pixels * TWIPS_PER_PIXEL).round();
    if rounded < MIN_SAFE_TWIPS as f64 {
        MIN_SAFE_TWIPS
    } else if rounded > MAX_SAFE_TWIPS as f64 {
        MAX_SAFE_TWIPS
    } else {
        rounded as i32
    }
}

/// Converts a twips 2D point `(x, y)` to pixels `(px, py)`.
#[inline]
pub fn twips_point_to_pixels(x: i32, y: i32) -> (f64, f64) {
    (twips_to_pixels_f64(x), twips_to_pixels_f64(y))
}

/// Safely adds two twips coordinates with overflow protection (CWE-190).
#[inline]
pub fn checked_twips_add(a: i32, b: i32) -> Result<i32, ShapeError> {
    let res = a.checked_add(b).ok_or(ShapeError::ArithmeticOverflow)?;
    validate_twips(res)?;
    Ok(res)
}

/// Safely translates a twips point with delta displacement (CWE-190).
#[inline]
pub fn checked_twips_point_add(x: i32, y: i32, dx: i32, dy: i32) -> Result<(i32, i32), ShapeError> {
    let nx = checked_twips_add(x, dx)?;
    let ny = checked_twips_add(y, dy)?;
    Ok((nx, ny))
}
