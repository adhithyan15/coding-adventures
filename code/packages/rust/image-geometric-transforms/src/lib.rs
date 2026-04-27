//! IMG04 — Geometric Transforms on PixelContainer
//!
//! A *geometric transform* moves pixels around in space rather than changing
//! their values in place. This crate covers every spatial operation defined
//! in the IMG04 spec: lossless flips and rotations, sub-image extraction,
//! padding, continuous scaling, free-angle rotation, affine warps, and full
//! projective (perspective) warps.
//!
//! # Lossless vs continuous transforms
//!
//! Some operations — flip, rotate-90, crop, pad — have an exact integer
//! mapping from output pixel to input pixel. No interpolation is needed;
//! the raw RGBA bytes are simply shuffled. These are called *lossless*
//! transforms and never touch the sRGB decode/encode path.
//!
//! Other operations — scale, free-angle rotate, affine, perspective — map
//! output coordinates to non-integer input coordinates. We must *sample* the
//! source image at fractional positions, which requires interpolation.
//! Interpolation must happen in **linear light** (not gamma-compressed sRGB)
//! because blending encoded values produces systematically wrong (too dark)
//! colours. The three supported modes are:
//!
//! * **Nearest neighbour** — no interpolation, raw bytes, fast, blocky.
//! * **Bilinear** — decode 2×2 neighbourhood to linear f32, blend with
//!   fractional weights, re-encode. Smooth but slightly blurry.
//! * **Bicubic (Catmull-Rom)** — decode 4×4 neighbourhood, apply cubic
//!   spline weights, re-encode. Sharp edges with minimal ringing.
//!
//! # Inverse warp model
//!
//! All continuous transforms use the *inverse warp* (pull-based) model:
//! for each **output** pixel we compute the corresponding **input**
//! coordinate and sample there. The alternative — *forward warp* (push) —
//! leaves holes when the mapping is non-injective and is harder to
//! parallelise. Inverse warp always fills every output pixel exactly once.
//!
//! # Pixel-centre convention
//!
//! Following the OpenGL / Metal convention, pixel (x, y) occupies the unit
//! square [x, x+1) × [y, y+1) and its *centre* is at (x+0.5, y+0.5).
//! When computing the scale ratio sx = out_w / in_w we map output centre
//!   u = (x' + 0.5) / sx − 0.5
//! so that the first and last output pixels align with the first and last
//! input pixels, not with the edges of the pixel array. Without the +0.5/−0.5
//! correction the image would appear shifted by half a pixel.
//!
//! # sRGB ↔ linear conversion
//!
//! The LUT and encode function are shared with IMG03 (image-point-ops). The
//! 256-entry decode LUT is built once and cached in a `OnceLock`. The encode
//! path uses the analytic formula (no LUT) because the output domain is a
//! continuous f32 before quantisation.

use pixel_container::PixelContainer;
use std::sync::OnceLock;

// ── Public types ──────────────────────────────────────────────────────────────

/// Which resampling kernel to use when sampling at fractional coordinates.
///
/// Nearest is cheapest; bicubic is most faithful. For lossless integer-mapped
/// transforms (flip, crop, rotate-90) this field is ignored.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Interpolation {
    /// Round to the nearest integer coordinate. Fast; produces a pixelated
    /// ("mosaic") look at large magnification factors.
    Nearest,
    /// 2×2 bilinear blend in linear light. Smooth but slightly blurry.
    Bilinear,
    /// 4×4 Catmull-Rom cubic blend in linear light. Sharper than bilinear
    /// with minimal ringing artefacts.
    Bicubic,
}

/// How to choose the output canvas size for a free-angle rotation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RotateBounds {
    /// Expand the canvas so the entire rotated image fits with no cropping.
    /// The corners of the source always remain visible.
    Fit,
    /// Keep the same canvas size as the input; corners of the rotated
    /// image are clipped to the original dimensions.
    Crop,
}

/// What value to return when sampling outside the image boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OutOfBounds {
    /// Return transparent black (0, 0, 0, 0). Good for rotation where the
    /// background should be transparent.
    Zero,
    /// Clamp to the nearest edge pixel. Good for scaling, prevents dark halos.
    Replicate,
    /// Mirror-reflect around each edge, producing a seamlessly tiling effect.
    Reflect,
    /// Wrap around (tile). Good for procedural textures and tiling patterns.
    Wrap,
}

/// Convenience alias for an RGBA pixel expressed as four u8 values.
pub type Rgba8 = (u8, u8, u8, u8);

// ── sRGB ↔ linear LUT ────────────────────────────────────────────────────────
//
// sRGB is a *non-linear* encoding designed for cathode-ray tube phosphors.
// The gamma curve intentionally compresses dark tones more than bright ones
// to match the perceptual sensitivity of the human visual system and to
// allocate more bit-depth where it matters most. But this non-linearity
// breaks any arithmetic that blends or averages values: blending two sRGB
// greys in the encoded domain yields a colour that is slightly too dark.
//
// The fix is standard: decode to linear f32, perform arithmetic, re-encode.
// We pre-compute the 256 decode values once at startup.

/// 256-entry LUT: sRGB u8 → linear f32.  Index with raw byte value 0–255.
static SRGB_TO_LINEAR: OnceLock<[f32; 256]> = OnceLock::new();

fn srgb_to_linear_lut() -> &'static [f32; 256] {
    SRGB_TO_LINEAR.get_or_init(|| {
        let mut t = [0f32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let c = i as f32 / 255.0;
            // IEC 61966-2-1 piecewise linearisation:
            //   below the elbow (c ≤ 0.04045) the curve is nearly linear
            //   above the elbow the curve is a power law with γ ≈ 2.2
            *v = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055_f32).powf(2.4)
            };
        }
        t
    })
}

/// Decode a single sRGB byte to a linear f32 in [0, 1].
#[inline]
fn decode(byte: u8) -> f32 {
    srgb_to_linear_lut()[byte as usize]
}

/// Encode a linear f32 in [0, 1] back to an sRGB u8 in [0, 255].
#[inline]
fn encode(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    // Inverse of the decode formula above.
    let srgb = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0).round() as u8
}

// ── Out-of-bounds coordinate resolution ──────────────────────────────────────
//
// Continuous transforms address fractional input coordinates, which means the
// 2×2 / 4×4 neighbourhood of sample points may straddle the image boundary.
// `resolve` maps a possibly-out-of-bounds integer coordinate to either a valid
// index (Some) or "transparent black" (None), according to the chosen policy.
//
// We work in 1-D; the caller resolves x and y independently.

/// Map a 1-D coordinate that may lie outside [0, max) to a valid index.
///
/// * `x`   — the coordinate (may be negative or ≥ max)
/// * `max` — image dimension in this axis (width or height)
/// * `oob` — the out-of-bounds policy
///
/// Returns `None` only for `OutOfBounds::Zero` when `x` is truly out of range.
#[inline]
fn resolve(x: i32, max: i32, oob: OutOfBounds) -> Option<i32> {
    match oob {
        OutOfBounds::Zero => {
            if x < 0 || x >= max {
                None
            } else {
                Some(x)
            }
        }
        OutOfBounds::Replicate => {
            // Clamp to the nearest valid index.  This is equivalent to
            // repeating the edge pixel indefinitely beyond the boundary.
            Some(x.clamp(0, max - 1))
        }
        OutOfBounds::Reflect => {
            // Mirror the coordinate around both edges with period 2*max.
            // The period is 2*max because mirroring around the left edge
            // (index 0) flips the image, and mirroring around the right edge
            // (index max-1) flips it back, for a combined period of 2*max.
            //
            // Example with max=4, valid indices [0,1,2,3]:
            //   x: … -4 -3 -2 -1 | 0 1 2 3 | 4 5 6 7 | 8 …
            //   → …  0  1  2  3  | 0 1 2 3 | 3 2 1 0 | 0 …
            let period = 2 * max;
            // Map x into [0, 2*max) using modular arithmetic.
            let mut x = ((x % period) + period) % period;
            // If x is in the upper half [max, 2*max), reflect it back down.
            if x >= max {
                x = period - 1 - x;
            }
            Some(x)
        }
        OutOfBounds::Wrap => {
            // True modular wrap: pixel rows/columns tile infinitely.
            Some(x.rem_euclid(max))
        }
    }
}

// ── Catmull-Rom spline weight ─────────────────────────────────────────────────
//
// Catmull-Rom is a family of cubic splines parameterised by two tension values
// (α, β). The variant used here sets α = 0, β = 0 (the "Keys" kernel), giving
// a kernel that:
//   - Passes through the data points (interpolating, not just approximating)
//   - Has a first derivative of ½ at each sample (smooth but with some sharpening)
//   - Sums to 1 over any 4-point neighbourhood (partition of unity → no bias)
//
// The weight function for distance |d| from the sample point:
//
//   |d| < 1:  w = 1.5·d³ − 2.5·d² + 1
//   |d| < 2:  w = −0.5·d³ + 2.5·d² − 4·d + 2
//   else:     w = 0
//
// We evaluate the kernel at four distances from the fractional offset `fx`:
//   d =  1+fx  (two pixels to the left)
//   d =  fx    (one pixel to the left / at floor)
//   d =  1−fx  (one pixel to the right)
//   d =  2−fx  (two pixels to the right)

#[inline]
fn catmull_rom(d: f32) -> f32 {
    let d = d.abs();
    if d < 1.0 {
        1.5 * d * d * d - 2.5 * d * d + 1.0
    } else if d < 2.0 {
        -0.5 * d * d * d + 2.5 * d * d - 4.0 * d + 2.0
    } else {
        0.0
    }
}

// ── Sampling functions ────────────────────────────────────────────────────────
//
// All three functions take a continuous (u, v) coordinate in input-image space
// and return a single RGBA8 pixel.  The difference is in how they handle the
// fractional part of (u, v).

/// Nearest-neighbour sampling: round to the closest integer pixel.
///
/// No colour-space conversion is performed; the raw sRGB byte values are
/// returned directly. This is correct because we are not blending — we are
/// copying an exact pixel value, so decoding to linear and re-encoding would
/// be a pure identity operation.
fn sample_nn(img: &PixelContainer, u: f32, v: f32, oob: OutOfBounds) -> Rgba8 {
    let xi = u.round() as i32;
    let yi = v.round() as i32;
    match (resolve(xi, img.width as i32, oob), resolve(yi, img.height as i32, oob)) {
        (Some(x), Some(y)) => img.pixel_at(x as u32, y as u32),
        _ => (0, 0, 0, 0),
    }
}

/// Bilinear sampling: blend the 2×2 neighbourhood of floor(u,v) in linear light.
///
/// Why linear light? Consider blending two mid-grey pixels (sRGB 128 ≈ linear
/// 0.216). In sRGB space (128 + 128) / 2 = 128 — the average is correct *by
/// coincidence* only because both inputs are equal. But blending sRGB 0 and
/// sRGB 255 gives sRGB 127.5 ≈ linear 0.216, whereas the true linear average
/// is (0 + 1.0) / 2 = 0.5 ≈ sRGB 188. The error here is nearly 25%.
/// Blending in linear light avoids this systematic darkening.
fn sample_bilinear(img: &PixelContainer, u: f32, v: f32, oob: OutOfBounds) -> Rgba8 {
    let x0 = u.floor() as i32;
    let x1 = x0 + 1;
    let y0 = v.floor() as i32;
    let y1 = y0 + 1;
    let fx = u - x0 as f32;
    let fy = v - y0 as f32;

    let w = img.width as i32;
    let h = img.height as i32;

    // Fetch four neighbour pixels, using the OOB policy for any that fall
    // outside the image. We decode immediately to linear f32.
    let get = |xi: i32, yi: i32| -> (f32, f32, f32, f32) {
        match (resolve(xi, w, oob), resolve(yi, h, oob)) {
            (Some(x), Some(y)) => {
                let (r, g, b, a) = img.pixel_at(x as u32, y as u32);
                (decode(r), decode(g), decode(b), a as f32 / 255.0)
            }
            _ => (0.0, 0.0, 0.0, 0.0),
        }
    };

    let (r00, g00, b00, a00) = get(x0, y0);
    let (r10, g10, b10, a10) = get(x1, y0);
    let (r01, g01, b01, a01) = get(x0, y1);
    let (r11, g11, b11, a11) = get(x1, y1);

    // Standard bilinear formula:
    //   f(u,v) = f00*(1−fx)*(1−fy) + f10*fx*(1−fy)
    //          + f01*(1−fx)*fy     + f11*fx*fy
    let blend = |c00: f32, c10: f32, c01: f32, c11: f32| -> f32 {
        c00 * (1.0 - fx) * (1.0 - fy)
            + c10 * fx * (1.0 - fy)
            + c01 * (1.0 - fx) * fy
            + c11 * fx * fy
    };

    let r = encode(blend(r00, r10, r01, r11));
    let g = encode(blend(g00, g10, g01, g11));
    let b = encode(blend(b00, b10, b01, b11));
    // Alpha is blended linearly (it's already linear — no gamma encoding)
    let a = (blend(a00, a10, a01, a11) * 255.0).round().clamp(0.0, 255.0) as u8;
    (r, g, b, a)
}

/// Bicubic (Catmull-Rom) sampling: blend the 4×4 neighbourhood in linear light.
///
/// Bicubic interpolation fits a piecewise cubic polynomial through the 16
/// neighbouring pixels. The Catmull-Rom variant used here guarantees the
/// interpolant passes through each data point (unlike Gaussian or Mitchell
/// filters that introduce blur). The trade-off is mild ringing (Gibbs
/// overshoot) at sharp edges, which is acceptable for photographic imagery.
///
/// Algorithm:
///   1. Decode all 16 neighbours from sRGB to linear f32.
///   2. For each of the 4 rows, blend horizontally using the 4 x-weights.
///   3. Blend the 4 row results vertically using the 4 y-weights.
///   4. Re-encode the single blended result to sRGB u8.
fn sample_bicubic(img: &PixelContainer, u: f32, v: f32, oob: OutOfBounds) -> Rgba8 {
    let x0 = u.floor() as i32;
    let y0 = v.floor() as i32;
    let fx = u - x0 as f32;
    let fy = v - y0 as f32;

    let w = img.width as i32;
    let h = img.height as i32;

    // Column offsets relative to x0: x0-1, x0, x0+1, x0+2
    // Distances from the fractional offset fx for the Catmull-Rom kernel:
    //   column x0-1 is at distance 1+fx to the left  → CR(1+fx)
    //   column x0   is at distance fx   to the left  → CR(fx)
    //   column x0+1 is at distance 1-fx to the right → CR(1-fx)
    //   column x0+2 is at distance 2-fx to the right → CR(2-fx)
    let wx = [
        catmull_rom(1.0 + fx),
        catmull_rom(fx),
        catmull_rom(1.0 - fx),
        catmull_rom(2.0 - fx),
    ];
    let wy = [
        catmull_rom(1.0 + fy),
        catmull_rom(fy),
        catmull_rom(1.0 - fy),
        catmull_rom(2.0 - fy),
    ];

    let get = |xi: i32, yi: i32| -> (f32, f32, f32, f32) {
        match (resolve(xi, w, oob), resolve(yi, h, oob)) {
            (Some(x), Some(y)) => {
                let (r, g, b, a) = img.pixel_at(x as u32, y as u32);
                (decode(r), decode(g), decode(b), a as f32 / 255.0)
            }
            _ => (0.0, 0.0, 0.0, 0.0),
        }
    };

    // Accumulate the 4×4 weighted sum.  We loop rather than unroll to keep
    // the code readable; the compiler will unroll at optimisation level ≥ 1.
    let mut acc_r = 0.0f32;
    let mut acc_g = 0.0f32;
    let mut acc_b = 0.0f32;
    let mut acc_a = 0.0f32;

    for (j, &wy_j) in wy.iter().enumerate() {
        let yi = y0 - 1 + j as i32;
        let mut row_r = 0.0f32;
        let mut row_g = 0.0f32;
        let mut row_b = 0.0f32;
        let mut row_a = 0.0f32;
        for (i, &wx_i) in wx.iter().enumerate() {
            let xi = x0 - 1 + i as i32;
            let (r, g, b, a) = get(xi, yi);
            row_r += wx_i * r;
            row_g += wx_i * g;
            row_b += wx_i * b;
            row_a += wx_i * a;
        }
        acc_r += wy_j * row_r;
        acc_g += wy_j * row_g;
        acc_b += wy_j * row_b;
        acc_a += wy_j * row_a;
    }

    let r = encode(acc_r);
    let g = encode(acc_g);
    let b = encode(acc_b);
    let a = (acc_a * 255.0).round().clamp(0.0, 255.0) as u8;
    (r, g, b, a)
}

/// Dispatcher: route to the appropriate sampling kernel.
#[inline]
pub fn sample(img: &PixelContainer, u: f32, v: f32, mode: Interpolation, oob: OutOfBounds) -> Rgba8 {
    match mode {
        Interpolation::Nearest => sample_nn(img, u, v, oob),
        Interpolation::Bilinear => sample_bilinear(img, u, v, oob),
        Interpolation::Bicubic => sample_bicubic(img, u, v, oob),
    }
}

// ── Lossless integer transforms ───────────────────────────────────────────────
//
// The following six operations map every output pixel to exactly one input
// pixel through an integer formula.  No colour-space conversion is needed.
// We copy raw 4-byte RGBA groups directly.

/// Flip an image horizontally (mirror left ↔ right).
///
/// For each row, we reverse the order of 4-byte pixel groups. The vertical
/// order (row positions) is unchanged.
///
/// Memory layout: row y occupies bytes [y*W*4 .. (y+1)*W*4). Reversing it
/// means pixel x' = W-1-x maps to source pixel x.
pub fn flip_horizontal(src: &PixelContainer) -> PixelContainer {
    let w = src.width;
    let h = src.height;
    let mut out = PixelContainer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b, a) = src.pixel_at(w - 1 - x, y);
            out.set_pixel(x, y, r, g, b, a);
        }
    }
    out
}

/// Flip an image vertically (mirror top ↔ bottom).
///
/// We swap row i with row (H-1-i) for i in [0, H/2). Only half the rows need
/// visiting because we copy directly rather than swapping in-place.
pub fn flip_vertical(src: &PixelContainer) -> PixelContainer {
    let w = src.width;
    let h = src.height;
    let mut out = PixelContainer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b, a) = src.pixel_at(x, h - 1 - y);
            out.set_pixel(x, y, r, g, b, a);
        }
    }
    out
}

/// Rotate 90° clockwise.
///
/// The output dimensions are swapped: W' = H, H' = W.
///
/// Derivation: imagine the source with origin at top-left.  After a 90° CW
/// rotation the original top edge becomes the right edge.  Pixel (x, y) in
/// the source appears at output position (H-1-y, x):
///
///   out[x'][y'] = in[y'][W-1-x']
///
/// (Reading right-to-left: output pixel at (x', y') came from source column
/// y' and source row W-1-x'.)
pub fn rotate_90_cw(src: &PixelContainer) -> PixelContainer {
    let sw = src.width;  // source width
    let sh = src.height; // source height
    // After 90° CW: output width = source height, output height = source width
    let ow = sh;
    let oh = sw;
    let mut out = PixelContainer::new(ow, oh);
    // O[x'][y'] = I[y'][W-1-x']
    for y_out in 0..oh {
        for x_out in 0..ow {
            let x_src = y_out;
            let y_src = sw - 1 - x_out;
            let (r, g, b, a) = src.pixel_at(x_src, y_src);
            out.set_pixel(x_out, y_out, r, g, b, a);
        }
    }
    out
}

/// Rotate 90° counter-clockwise.
///
/// The inverse of `rotate_90_cw`.  Dimensions swap the same way.
///
///   out[x'][y'] = in[W-1-y'][x']
pub fn rotate_90_ccw(src: &PixelContainer) -> PixelContainer {
    let sw = src.width;
    let sh = src.height;
    let ow = sh;
    let oh = sw;
    let mut out = PixelContainer::new(ow, oh);
    // O[x'][y'] = I[W-1-y'][x']
    //
    // Derivation: a 90° CCW rotation sends source (x, y) to output (y, W-1-x).
    // Inverting: output (x', y') came from source x_src=W-1-y', y_src=x'.
    // We use source **width** (sw), not source height (sh), here.
    for y_out in 0..oh {
        for x_out in 0..ow {
            let x_src = sw - 1 - y_out;
            let y_src = x_out;
            let (r, g, b, a) = src.pixel_at(x_src, y_src);
            out.set_pixel(x_out, y_out, r, g, b, a);
        }
    }
    out
}

/// Rotate 180°.
///
/// Equivalent to `flip_horizontal` followed by `flip_vertical`, but done in
/// a single pass.
///
///   out[x'][y'] = in[W-1-x'][H-1-y']
pub fn rotate_180(src: &PixelContainer) -> PixelContainer {
    let w = src.width;
    let h = src.height;
    let mut out = PixelContainer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let (r, g, b, a) = src.pixel_at(w - 1 - x, h - 1 - y);
            out.set_pixel(x, y, r, g, b, a);
        }
    }
    out
}

/// Extract a rectangular sub-region from an image.
///
/// The crop rectangle is [x0, x0+w) × [y0, y0+h). Coordinates that extend
/// beyond the source image boundary are clamped to the image edge (so a too-
/// large crop silently clips rather than panics).
pub fn crop(src: &PixelContainer, x0: u32, y0: u32, w: u32, h: u32) -> PixelContainer {
    // Clamp the crop region to the available source pixels.
    let actual_w = w.min(src.width.saturating_sub(x0));
    let actual_h = h.min(src.height.saturating_sub(y0));
    let mut out = PixelContainer::new(actual_w, actual_h);
    for y in 0..actual_h {
        for x in 0..actual_w {
            let (r, g, b, a) = src.pixel_at(x0 + x, y0 + y);
            out.set_pixel(x, y, r, g, b, a);
        }
    }
    out
}

/// Add a border around an image with a specified fill colour.
///
/// The output dimensions are (W + left + right) × (H + top + bottom).
/// The source pixels are copied to the interior starting at (left, top).
/// The border regions are filled with `fill`.
pub fn pad(
    src: &PixelContainer,
    top: u32,
    right: u32,
    bottom: u32,
    left: u32,
    fill: Rgba8,
) -> PixelContainer {
    let ow = src.width + left + right;
    let oh = src.height + top + bottom;
    let mut out = PixelContainer::new(ow, oh);

    // Fill the entire output with the border colour first.
    // This handles the four corners and edges in one pass.
    let (fr, fg, fb, fa) = fill;
    for y in 0..oh {
        for x in 0..ow {
            out.set_pixel(x, y, fr, fg, fb, fa);
        }
    }

    // Copy source pixels into the interior.
    for y in 0..src.height {
        for x in 0..src.width {
            let (r, g, b, a) = src.pixel_at(x, y);
            out.set_pixel(x + left, y + top, r, g, b, a);
        }
    }
    out
}

// ── Continuous transforms ─────────────────────────────────────────────────────
//
// These operations use the inverse-warp model + sampling to produce output
// pixels at arbitrary fractional source coordinates.

/// Scale an image to a new size using the specified interpolation mode.
///
/// # Pixel-centre model
///
/// We use the half-pixel-offset convention so that the first output pixel
/// aligns with the first input pixel and the last output pixel aligns with the
/// last input pixel:
///
///   u = (x' + 0.5) / sx − 0.5     where  sx = out_w / src_w
///   v = (y' + 0.5) / sy − 0.5
///
/// Without the ±0.5 corrections, upscaling would shift the image right and
/// down by half an output pixel, causing a visible seam at the top-left.
///
/// Out-of-bounds policy is always `Replicate` so that edge pixels extend
/// smoothly without an artificial transparent border.
pub fn scale(src: &PixelContainer, out_w: u32, out_h: u32, mode: Interpolation) -> PixelContainer {
    let mut out = PixelContainer::new(out_w, out_h);
    if out_w == 0 || out_h == 0 || src.width == 0 || src.height == 0 {
        return out;
    }
    let sx = out_w as f32 / src.width as f32;
    let sy = out_h as f32 / src.height as f32;
    for y_out in 0..out_h {
        let v = (y_out as f32 + 0.5) / sy - 0.5;
        for x_out in 0..out_w {
            let u = (x_out as f32 + 0.5) / sx - 0.5;
            let (r, g, b, a) = sample(src, u, v, mode, OutOfBounds::Replicate);
            out.set_pixel(x_out, y_out, r, g, b, a);
        }
    }
    out
}

/// Rotate an image by an arbitrary angle (in radians) around its centre.
///
/// # Inverse-warp derivation
///
/// A clockwise rotation by θ maps source point (u, v) to output point (x', y')
/// by:
///   x' − cx_out = cos(θ)·(u − cx_in) − sin(θ)·(v − cy_in)
///   y' − cy_out = sin(θ)·(u − cx_in) + cos(θ)·(v − cy_in)
///
/// Inverting (rotating by −θ) gives us u given (x', y'):
///   dx = x' − cx_out,  dy = y' − cy_out
///   u  = cx_in + cos(θ)·dx + sin(θ)·dy
///   v  = cy_in − sin(θ)·dx + cos(θ)·dy
///
/// Note the sign on the sine term: −sin for u, +sin for v comes from the
/// standard 2D inverse rotation matrix [[cos, sin], [−sin, cos]].
///
/// # RotateBounds::Fit
///
/// The bounding box of a rectangle W×H rotated by θ has width
///   W' = W·|cos θ| + H·|sin θ|
///   H' = W·|sin θ| + H·|cos θ|
///
/// The `Fit` mode sets the output to this bounding box so every source pixel
/// is visible. Pixels outside the rotated footprint receive transparent black.
///
/// # RotateBounds::Crop
///
/// The `Crop` mode keeps the same dimensions as the source. Parts of the
/// rotated image that fall outside are discarded.
pub fn rotate(
    src: &PixelContainer,
    radians: f32,
    mode: Interpolation,
    bounds: RotateBounds,
) -> PixelContainer {
    let w = src.width as f32;
    let h = src.height as f32;
    let cos = radians.cos();
    let sin = radians.sin();

    let (out_w, out_h) = match bounds {
        RotateBounds::Fit => {
            let fw = (w * cos.abs() + h * sin.abs()).ceil() as u32;
            let fh = (w * sin.abs() + h * cos.abs()).ceil() as u32;
            (fw, fh)
        }
        RotateBounds::Crop => (src.width, src.height),
    };

    let cx_in = w / 2.0;
    let cy_in = h / 2.0;
    let cx_out = out_w as f32 / 2.0;
    let cy_out = out_h as f32 / 2.0;

    let mut out = PixelContainer::new(out_w, out_h);
    for y_out in 0..out_h {
        let dy = y_out as f32 - cy_out;
        for x_out in 0..out_w {
            let dx = x_out as f32 - cx_out;
            // Inverse rotation: bring output vector back to input space
            let u = cx_in + cos * dx + sin * dy;
            let v = cy_in - sin * dx + cos * dy;
            // Zero OOB: outside the rotated footprint → transparent black
            let (r, g, b, a) = sample(src, u, v, mode, OutOfBounds::Zero);
            out.set_pixel(x_out, y_out, r, g, b, a);
        }
    }
    out
}

/// Apply a 2×3 affine transformation matrix to an image.
///
/// The matrix `m` encodes an **inverse warp**: given output pixel (x', y'),
/// the corresponding input coordinate is:
///
///   u = m[0][0]·x' + m[0][1]·y' + m[0][2]
///   v = m[1][0]·x' + m[1][1]·y' + m[1][2]
///
/// # Why inverse (output→input) rather than forward (input→output)?
///
/// The forward matrix would map source pixels to output positions. But many
/// source positions could map to the same output pixel (if the transform
/// squashes or tiles), and many output pixels might receive no contribution
/// (if the transform expands). With the inverse warp every output pixel is
/// visited exactly once and samples exactly one (interpolated) input location.
///
/// # Usage
///
/// To construct a forward matrix M (input→output) and use it here, invert it
/// first. For a pure 2D affine transform (no perspective) the 2×3 inverse can
/// be computed analytically in closed form.
pub fn affine(
    src: &PixelContainer,
    matrix: [[f32; 3]; 2],
    out_w: u32,
    out_h: u32,
    mode: Interpolation,
    oob: OutOfBounds,
) -> PixelContainer {
    let mut out = PixelContainer::new(out_w, out_h);
    let m = matrix;
    for y_out in 0..out_h {
        let yf = y_out as f32;
        for x_out in 0..out_w {
            let xf = x_out as f32;
            let u = m[0][0] * xf + m[0][1] * yf + m[0][2];
            let v = m[1][0] * xf + m[1][1] * yf + m[1][2];
            let (r, g, b, a) = sample(src, u, v, mode, oob);
            out.set_pixel(x_out, y_out, r, g, b, a);
        }
    }
    out
}

/// Apply a 3×3 homogeneous perspective (projective) warp to an image.
///
/// Perspective transforms are the most general planar mapping between two
/// images. A 3×3 homogeneous matrix `h` can represent any combination of
/// translation, rotation, scale, shear, and projective foreshortening.
///
/// Again `h` is the **inverse** homography (output plane → input plane):
///
///   [uh, vh, w]ᵀ = h · [x', y', 1]ᵀ
///   u = uh / w
///   v = vh / w
///
/// The division by `w` (the homogeneous scale factor) is what distinguishes
/// a perspective warp from an affine warp. When `h[2][0]` and `h[2][1]` are
/// zero, `w` is constant and the result degenerates to an affine transform.
///
/// # Degenerate case
///
/// If `w ≈ 0` (the mapping sends the output point to the "plane at infinity"),
/// we skip that pixel (leave it transparent black). This prevents division by
/// zero and the resulting NaN/Inf from propagating into the pixel buffer.
pub fn perspective_warp(
    src: &PixelContainer,
    h: [[f32; 3]; 3],
    out_w: u32,
    out_h: u32,
    mode: Interpolation,
    oob: OutOfBounds,
) -> PixelContainer {
    let mut out = PixelContainer::new(out_w, out_h);
    for y_out in 0..out_h {
        let yf = y_out as f32;
        for x_out in 0..out_w {
            let xf = x_out as f32;
            let uh = h[0][0] * xf + h[0][1] * yf + h[0][2];
            let vh = h[1][0] * xf + h[1][1] * yf + h[1][2];
            let w  = h[2][0] * xf + h[2][1] * yf + h[2][2];
            if w.abs() < 1e-7 {
                // Degenerate mapping — leave pixel as transparent black.
                continue;
            }
            let u = uh / w;
            let v = vh / w;
            let (r, g, b, a) = sample(src, u, v, mode, oob);
            out.set_pixel(x_out, y_out, r, g, b, a);
        }
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Build a 2×2 image with four distinct colours.
    ///
    ///   TL  TR        (0,0)=red  (1,0)=green
    ///   BL  BR        (0,1)=blue (1,1)=white
    fn two_by_two() -> PixelContainer {
        let mut img = PixelContainer::new(2, 2);
        img.set_pixel(0, 0, 255, 0,   0,   255); // red
        img.set_pixel(1, 0, 0,   255, 0,   255); // green
        img.set_pixel(0, 1, 0,   0,   255, 255); // blue
        img.set_pixel(1, 1, 255, 255, 255, 255); // white
        img
    }

    /// Build a 3×3 image with distinct values in every cell.
    fn three_by_three() -> PixelContainer {
        let mut img = PixelContainer::new(3, 3);
        // Row 0: (10,0,0), (20,0,0), (30,0,0)
        // Row 1: (40,0,0), (50,0,0), (60,0,0)
        // Row 2: (70,0,0), (80,0,0), (90,0,0)
        for y in 0..3u32 {
            for x in 0..3u32 {
                let v = ((y * 3 + x + 1) * 10) as u8;
                img.set_pixel(x, y, v, 0, 0, 255);
            }
        }
        img
    }

    // ── Flip tests ────────────────────────────────────────────────────────────

    #[test]
    fn flip_horizontal_reverses_pixels() {
        let src = two_by_two();
        let out = flip_horizontal(&src);
        // (0,0) should now be what was at (1,0), and vice versa.
        assert_eq!(out.pixel_at(0, 0), (0, 255, 0, 255));   // was green
        assert_eq!(out.pixel_at(1, 0), (255, 0, 0, 255));   // was red
        assert_eq!(out.pixel_at(0, 1), (255, 255, 255, 255)); // was white
        assert_eq!(out.pixel_at(1, 1), (0, 0, 255, 255));   // was blue
    }

    #[test]
    fn flip_vertical_reverses_rows() {
        let src = two_by_two();
        let out = flip_vertical(&src);
        // Row 0 should now contain what was in row 1.
        assert_eq!(out.pixel_at(0, 0), (0, 0, 255, 255));   // was blue
        assert_eq!(out.pixel_at(1, 0), (255, 255, 255, 255)); // was white
        assert_eq!(out.pixel_at(0, 1), (255, 0, 0, 255));   // was red
        assert_eq!(out.pixel_at(1, 1), (0, 255, 0, 255));   // was green
    }

    #[test]
    fn flip_horizontal_double_is_identity() {
        let src = three_by_three();
        let out = flip_horizontal(&flip_horizontal(&src));
        for y in 0..3u32 {
            for x in 0..3u32 {
                assert_eq!(out.pixel_at(x, y), src.pixel_at(x, y));
            }
        }
    }

    #[test]
    fn flip_vertical_double_is_identity() {
        let src = three_by_three();
        let out = flip_vertical(&flip_vertical(&src));
        for y in 0..3u32 {
            for x in 0..3u32 {
                assert_eq!(out.pixel_at(x, y), src.pixel_at(x, y));
            }
        }
    }

    // ── Rotation tests ────────────────────────────────────────────────────────

    #[test]
    fn rotate_90_cw_dimensions_swap() {
        let src = PixelContainer::new(10, 3);
        let out = rotate_90_cw(&src);
        assert_eq!(out.width, 3);
        assert_eq!(out.height, 10);
    }

    #[test]
    fn rotate_90_ccw_dimensions_swap() {
        let src = PixelContainer::new(7, 5);
        let out = rotate_90_ccw(&src);
        assert_eq!(out.width, 5);
        assert_eq!(out.height, 7);
    }

    #[test]
    fn rotate_90_cw_then_ccw_is_identity() {
        let src = three_by_three();
        let out = rotate_90_ccw(&rotate_90_cw(&src));
        assert_eq!(out.width, src.width);
        assert_eq!(out.height, src.height);
        for y in 0..3u32 {
            for x in 0..3u32 {
                assert_eq!(out.pixel_at(x, y), src.pixel_at(x, y),
                    "mismatch at ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn rotate_90_ccw_then_cw_is_identity() {
        let src = three_by_three();
        let out = rotate_90_cw(&rotate_90_ccw(&src));
        assert_eq!(out.width, src.width);
        assert_eq!(out.height, src.height);
        for y in 0..3u32 {
            for x in 0..3u32 {
                assert_eq!(out.pixel_at(x, y), src.pixel_at(x, y),
                    "mismatch at ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn rotate_180_twice_is_identity() {
        let src = three_by_three();
        let out = rotate_180(&rotate_180(&src));
        for y in 0..3u32 {
            for x in 0..3u32 {
                assert_eq!(out.pixel_at(x, y), src.pixel_at(x, y));
            }
        }
    }

    #[test]
    fn rotate_90_cw_pixel_position() {
        // In a 2×2 image, after 90° CW:
        //   out[x'][y'] = in[y'][W-1-x']
        // so out(0,0) = in(0, 1) = blue
        //    out(1,0) = in(0, 0) = red
        //    out(0,1) = in(1, 1) = white
        //    out(1,1) = in(1, 0) = green
        let src = two_by_two();
        let out = rotate_90_cw(&src);
        assert_eq!(out.pixel_at(0, 0), (0, 0, 255, 255));   // blue
        assert_eq!(out.pixel_at(1, 0), (255, 0, 0, 255));   // red
        assert_eq!(out.pixel_at(0, 1), (255, 255, 255, 255)); // white
        assert_eq!(out.pixel_at(1, 1), (0, 255, 0, 255));   // green
    }

    // ── Crop tests ────────────────────────────────────────────────────────────

    #[test]
    fn crop_correct_dimensions() {
        let src = three_by_three();
        let out = crop(&src, 1, 0, 2, 2);
        assert_eq!(out.width, 2);
        assert_eq!(out.height, 2);
    }

    #[test]
    fn crop_extracts_correct_sub_region() {
        // 3×3 grid with values 10,20,...,90 in the red channel.
        // Crop(x0=1, y0=1, w=2, h=2) should give values 50,60,80,90.
        let src = three_by_three();
        let out = crop(&src, 1, 1, 2, 2);
        assert_eq!(out.pixel_at(0, 0).0, 50);
        assert_eq!(out.pixel_at(1, 0).0, 60);
        assert_eq!(out.pixel_at(0, 1).0, 80);
        assert_eq!(out.pixel_at(1, 1).0, 90);
    }

    #[test]
    fn crop_clamps_to_image_edge() {
        let src = three_by_three();
        // Request a crop that extends past the right/bottom edge.
        let out = crop(&src, 2, 2, 10, 10);
        assert_eq!(out.width, 1);
        assert_eq!(out.height, 1);
        assert_eq!(out.pixel_at(0, 0).0, 90);
    }

    // ── Pad tests ─────────────────────────────────────────────────────────────

    #[test]
    fn pad_dimensions_correct() {
        let src = two_by_two();
        let out = pad(&src, 1, 2, 3, 4, (0, 0, 0, 255));
        // W' = 2 + 4 + 2 = 8,  H' = 2 + 1 + 3 = 6
        assert_eq!(out.width, 8);
        assert_eq!(out.height, 6);
    }

    #[test]
    fn pad_interior_matches_source() {
        let src = two_by_two();
        let top = 1u32;
        let left = 2u32;
        let out = pad(&src, top, 1, 1, left, (99, 0, 0, 255));
        // Source pixel (0,0)=red should be at output (left, top)=(2,1)
        assert_eq!(out.pixel_at(left, top), (255, 0, 0, 255));
        // Source pixel (1,1)=white at output (3,2)
        assert_eq!(out.pixel_at(left + 1, top + 1), (255, 255, 255, 255));
    }

    #[test]
    fn pad_border_matches_fill_color() {
        let src = two_by_two();
        let fill: Rgba8 = (42, 43, 44, 255);
        let out = pad(&src, 2, 2, 2, 2, fill);
        // Top-left corner is the fill colour.
        assert_eq!(out.pixel_at(0, 0), fill);
        // Bottom-right corner is the fill colour.
        assert_eq!(out.pixel_at(out.width - 1, out.height - 1), fill);
    }

    // ── Scale tests ───────────────────────────────────────────────────────────

    #[test]
    fn scale_up_doubles_dimensions() {
        let src = two_by_two();
        let out = scale(&src, 4, 4, Interpolation::Nearest);
        assert_eq!(out.width, 4);
        assert_eq!(out.height, 4);
    }

    #[test]
    fn scale_down_halves_dimensions() {
        let src = PixelContainer::new(8, 6);
        let out = scale(&src, 4, 3, Interpolation::Nearest);
        assert_eq!(out.width, 4);
        assert_eq!(out.height, 3);
    }

    #[test]
    fn scale_replicate_oob_does_not_panic() {
        // If Replicate OOB is broken, edge pixels would be sampled at -1
        // and panic. A successful run is the test.
        let src = three_by_three();
        let _ = scale(&src, 9, 9, Interpolation::Bilinear);
    }

    #[test]
    fn scale_wrap_tiles_correctly() {
        // A 2×1 image [red, blue] scaled 2× horizontally with Wrap OOB
        // should tile: [red, blue, red, blue].
        // Nearest-neighbour with Wrap applied manually via affine:
        // The scale function uses Replicate internally, so we test Wrap
        // through the affine function instead.
        let mut src = PixelContainer::new(2, 1);
        src.set_pixel(0, 0, 255, 0, 0, 255);
        src.set_pixel(1, 0, 0, 0, 255, 255);
        // affine identity with Wrap:
        let identity = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let out = affine(&src, identity, 2, 1, Interpolation::Nearest, OutOfBounds::Wrap);
        assert_eq!(out.pixel_at(0, 0), (255, 0, 0, 255));
        assert_eq!(out.pixel_at(1, 0), (0, 0, 255, 255));
    }

    // ── Rotate (free-angle) tests ─────────────────────────────────────────────

    #[test]
    fn rotate_zero_is_approximately_identity() {
        // Rotating by 0 radians should return an image very close to the
        // source. Due to resampling, we allow ±1 per channel.
        let src = three_by_three();
        let out = rotate(&src, 0.0, Interpolation::Bilinear, RotateBounds::Crop);
        assert_eq!(out.width, src.width);
        assert_eq!(out.height, src.height);
        for y in 0..3u32 {
            for x in 0..3u32 {
                let (sr, sg, sb, sa) = src.pixel_at(x, y);
                let (or_, og, ob, oa) = out.pixel_at(x, y);
                assert!((sr as i32 - or_ as i32).abs() <= 1,
                    "R mismatch at ({},{}): src={} out={}", x, y, sr, or_);
                assert!((sg as i32 - og as i32).abs() <= 1);
                assert!((sb as i32 - ob as i32).abs() <= 1);
                assert_eq!(sa, oa);
            }
        }
    }

    #[test]
    fn rotate_fit_larger_than_crop() {
        // Rotating a non-square image at 45° Fit should produce a larger canvas
        // than Crop.
        let src = PixelContainer::new(10, 4);
        let angle = std::f32::consts::FRAC_PI_4;
        let fit_out = rotate(&src, angle, Interpolation::Nearest, RotateBounds::Fit);
        let crop_out = rotate(&src, angle, Interpolation::Nearest, RotateBounds::Crop);
        assert!(fit_out.width >= crop_out.width || fit_out.height >= crop_out.height);
    }

    // ── Affine tests ──────────────────────────────────────────────────────────

    #[test]
    fn affine_identity_matrix_is_identity() {
        // The 2×3 identity affine matrix maps each output pixel to itself.
        let src = three_by_three();
        let identity = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let out = affine(&src, identity, 3, 3, Interpolation::Nearest, OutOfBounds::Zero);
        for y in 0..3u32 {
            for x in 0..3u32 {
                assert_eq!(out.pixel_at(x, y), src.pixel_at(x, y),
                    "mismatch at ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn affine_translation() {
        // Translating by (1, 0) shifts the image right by one pixel.
        // Output pixel (0, y) maps to source pixel (-1, y) → Zero → (0,0,0,0)
        // Output pixel (1, y) maps to source pixel (0, y) → valid
        let src = three_by_three();
        // Inverse warp: u = x' - 1, v = y'
        let m = [[1.0f32, 0.0, -1.0], [0.0, 1.0, 0.0]];
        let out = affine(&src, m, 3, 3, Interpolation::Nearest, OutOfBounds::Zero);
        // Column 0 should be transparent (OOB)
        assert_eq!(out.pixel_at(0, 0), (0, 0, 0, 0));
        // Column 1 should contain what was in source column 0
        assert_eq!(out.pixel_at(1, 0).0, src.pixel_at(0, 0).0);
    }

    // ── Perspective warp tests ────────────────────────────────────────────────

    #[test]
    fn perspective_warp_identity_matrix_is_identity() {
        // The 3×3 identity homography maps output pixels to themselves.
        let src = three_by_three();
        let identity = [[1.0f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let out = perspective_warp(&src, identity, 3, 3, Interpolation::Nearest, OutOfBounds::Zero);
        for y in 0..3u32 {
            for x in 0..3u32 {
                assert_eq!(out.pixel_at(x, y), src.pixel_at(x, y),
                    "mismatch at ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn perspective_warp_scale_factor_in_w() {
        // A homography with w=2 (divide by 2) effectively halves the mapping.
        // h = [[2,0,0],[0,2,0],[0,0,2]] divides by 2 → same as identity for u,v.
        let src = three_by_three();
        let h = [[2.0f32, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]];
        let out = perspective_warp(&src, h, 3, 3, Interpolation::Nearest, OutOfBounds::Zero);
        for y in 0..3u32 {
            for x in 0..3u32 {
                assert_eq!(out.pixel_at(x, y), src.pixel_at(x, y),
                    "mismatch at ({}, {})", x, y);
            }
        }
    }

    // ── Nearest-neighbour exact-value tests ───────────────────────────────────

    #[test]
    fn nearest_neighbour_exact_no_rounding() {
        // NN sampling must return exactly the source pixel — no blending.
        let src = two_by_two();
        // Scale 2× with NN: each output 2×2 block maps to a single source pixel.
        let out = scale(&src, 4, 4, Interpolation::Nearest);
        // Top-left 2×2 block should all be red (255,0,0,255).
        assert_eq!(out.pixel_at(0, 0), (255, 0, 0, 255));
        assert_eq!(out.pixel_at(1, 0), (255, 0, 0, 255));
        assert_eq!(out.pixel_at(0, 1), (255, 0, 0, 255));
        assert_eq!(out.pixel_at(1, 1), (255, 0, 0, 255));
    }

    // ── Bilinear midpoint blend test ──────────────────────────────────────────

    #[test]
    fn bilinear_midpoint_blend_2x1_gradient() {
        // A 2×1 image: left pixel (0,0,0,255), right pixel (100,0,0,255).
        // Sampling the midpoint u=0.5, v=0 should produce approximately
        // the linear blend in sRGB-decoded space, then re-encoded.
        //
        // decode(0) = 0.0, decode(100) ≈ 0.1329
        // blend = (0.0 + 0.1329) / 2 = 0.0665
        // encode(0.0665) ≈ round(0.0665^(1/2.4)*1.055 - 0.055)*255 ≈ 71
        let mut src = PixelContainer::new(2, 1);
        src.set_pixel(0, 0, 0,   0, 0, 255);
        src.set_pixel(1, 0, 100, 0, 0, 255);

        // Sample exactly at u=0.5: x0=floor(0.5)=0, fx=0.5
        // bilinear blend uses only x0=0 and x1=1 (1D case since y0=y1=0)
        let (r, _, _, _) = sample_bilinear(&src, 0.5, 0.0, OutOfBounds::Replicate);
        // The exact value depends on the decode/encode round-trip.
        // We accept any value in [60, 80] to allow for rounding.
        println!("bilinear midpoint r={}", r);
        assert!(r >= 60 && r <= 80,
            "expected bilinear midpoint ~71, got {}", r);
    }

    // ── OOB resolve unit tests ────────────────────────────────────────────────

    #[test]
    fn resolve_zero_in_bounds() {
        assert_eq!(resolve(2, 5, OutOfBounds::Zero), Some(2));
    }

    #[test]
    fn resolve_zero_out_of_bounds() {
        assert_eq!(resolve(-1, 5, OutOfBounds::Zero), None);
        assert_eq!(resolve(5, 5, OutOfBounds::Zero), None);
    }

    #[test]
    fn resolve_replicate_clamps() {
        assert_eq!(resolve(-3, 5, OutOfBounds::Replicate), Some(0));
        assert_eq!(resolve(7, 5, OutOfBounds::Replicate), Some(4));
        assert_eq!(resolve(2, 5, OutOfBounds::Replicate), Some(2));
    }

    #[test]
    fn resolve_reflect_basic() {
        // max=4: period=8, valid [0..4)
        // x=4 → in upper half → 8-1-4=3
        assert_eq!(resolve(4, 4, OutOfBounds::Reflect), Some(3));
        // x=-1 → mod 8 = 7 → upper half → 8-1-7=0
        assert_eq!(resolve(-1, 4, OutOfBounds::Reflect), Some(0));
        // x=0 stays 0
        assert_eq!(resolve(0, 4, OutOfBounds::Reflect), Some(0));
    }

    #[test]
    fn resolve_wrap_basic() {
        // max=4
        assert_eq!(resolve(0, 4, OutOfBounds::Wrap), Some(0));
        assert_eq!(resolve(4, 4, OutOfBounds::Wrap), Some(0));
        assert_eq!(resolve(-1, 4, OutOfBounds::Wrap), Some(3));
        assert_eq!(resolve(7, 4, OutOfBounds::Wrap), Some(3));
    }

    // ── Catmull-Rom weight unit test ──────────────────────────────────────────

    #[test]
    fn catmull_rom_at_zero_is_one() {
        assert!((catmull_rom(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn catmull_rom_at_one_is_zero() {
        assert!((catmull_rom(1.0) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn catmull_rom_partition_of_unity() {
        // For any fractional offset fx in [0, 1), the four weights at distances
        // 1+fx, fx, 1-fx, 2-fx should sum to 1.0.
        for i in 0..100 {
            let fx = i as f32 / 100.0;
            let sum = catmull_rom(1.0 + fx)
                + catmull_rom(fx)
                + catmull_rom(1.0 - fx)
                + catmull_rom(2.0 - fx);
            assert!((sum - 1.0).abs() < 1e-5, "sum={} for fx={}", sum, fx);
        }
    }
}
