//! IMG03 — Point Operations on PixelContainer
//!
//! Every function in this crate transforms each pixel independently using
//! the input at that position only (zero neighbourhood radius) -- with one
//! documented exception: `adaptive_threshold_mean` (IMG09), whose output at
//! a pixel depends on a local neighbourhood around it. See its own doc
//! comment and `IMG09-adaptive-threshold.md` for why.
//!
//! The PixelContainer type stores RGBA8 in sRGB colour space (see IC00).
//! Operations that require accurate arithmetic (contrast, gamma, exposure,
//! colour matrix, greyscale, sepia, saturation, hue rotation) decode to
//! linear-light f32, operate, then re-encode to sRGB u8.  Operations whose
//! mapping is exact in sRGB (invert, threshold, channel ops, additive
//! brightness) work directly on u8 bytes.
//!
//! # The sRGB ↔ linear round-trip
//!
//! sRGB decode (u8 → f32 linear):
//!   c = byte / 255.0
//!   if c <= 0.04045  →  c / 12.92
//!   else             →  ((c + 0.055) / 1.055)^2.4
//!
//! sRGB encode (f32 linear → u8):
//!   if c <= 0.0031308  →  c * 12.92
//!   else               →  1.055 * c^(1/2.4) − 0.055
//!   multiply by 255, round, clamp to [0, 255]

use pixel_container::PixelContainer;

// ── sRGB / linear LUTs (built once at startup) ───────────────────────────────

/// 256-entry LUT: sRGB u8 → linear f32.  Index with raw byte value.
static SRGB_TO_LINEAR: std::sync::OnceLock<[f32; 256]> = std::sync::OnceLock::new();

fn srgb_to_linear_lut() -> &'static [f32; 256] {
    SRGB_TO_LINEAR.get_or_init(|| {
        let mut t = [0f32; 256];
        for (i, v) in t.iter_mut().enumerate() {
            let c = i as f32 / 255.0;
            *v = if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055_f32).powf(2.4)
            };
        }
        t
    })
}

#[inline]
fn decode(byte: u8) -> f32 {
    srgb_to_linear_lut()[byte as usize]
}

#[inline]
fn encode(linear: f32) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let srgb = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (srgb * 255.0).round() as u8
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Iterate over each pixel as a mutable (R, G, B, A) tuple of u8.
fn map_pixels(src: &PixelContainer, mut f: impl FnMut(u8, u8, u8, u8) -> (u8, u8, u8, u8)) -> PixelContainer {
    let mut out = PixelContainer::new(src.width, src.height);
    for i in (0..src.data.len()).step_by(4) {
        let (r, g, b, a) = f(src.data[i], src.data[i+1], src.data[i+2], src.data[i+3]);
        out.data[i]   = r;
        out.data[i+1] = g;
        out.data[i+2] = b;
        out.data[i+3] = a;
    }
    out
}

// ── u8-domain operations (no colorspace conversion needed) ────────────────────

/// Invert RGB channels.  Alpha is unchanged.
pub fn invert(src: &PixelContainer) -> PixelContainer {
    map_pixels(src, |r, g, b, a| (255 - r, 255 - g, 255 - b, a))
}

/// Threshold each channel independently: output is 0 or 255.
pub fn threshold(src: &PixelContainer, t: u8) -> PixelContainer {
    map_pixels(src, |r, g, b, a| (
        if r >= t { 255 } else { 0 },
        if g >= t { 255 } else { 0 },
        if b >= t { 255 } else { 0 },
        a,
    ))
}

/// Threshold based on BT.601 sRGB luminance Y = 0.299R + 0.587G + 0.114B.
/// All RGB channels are set to 0 or 255; alpha is unchanged.
pub fn threshold_luminance(src: &PixelContainer, t: u8) -> PixelContainer {
    map_pixels(src, |r, g, b, a| {
        let y = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round() as u8;
        let v = if y >= t { 255 } else { 0 };
        (v, v, v, a)
    })
}

/// IMG09 — Locally-adaptive threshold via the Bradley/Roth integral-image
/// method (Bradley & Roth, 2007): a pixel is foreground (dark) when its
/// luminance falls more than a fraction `t` below the mean luminance of a
/// `window` x `window` neighbourhood around it, computed in O(1) per pixel
/// via a summed-area table -- the whole operation is O(width x height), not
/// O(width x height x window^2). Unlike `threshold`/`threshold_luminance`'s
/// single scalar cutoff, this adapts to uneven lighting (a shadow across
/// half a photographed document doesn't push that whole half to the wrong
/// side of one global cutoff). See `IMG09-adaptive-threshold.md`.
///
/// `window` is clamped to at least 1 (never a divide-by-zero); the
/// neighbourhood is clipped at the image edges (a smaller effective window
/// there), never a panic or a wrap-around. `t` is clamped to `[0.0, 1.0]`;
/// `NaN` is treated as `0.0` since `f64::clamp` passes `NaN` through
/// unchanged rather than clamping it. RGB is set to `0` (foreground/dark)
/// or `255` (background/light), matching `threshold_luminance`'s output
/// convention; alpha is unchanged. A `0`-width or `0`-height `src` returns
/// an equally-empty result, never a panic.
///
/// This is the one function in this crate whose output at a pixel depends
/// on its neighbours, not just its own value -- see the module doc. It's
/// also the one function here with a peak memory footprint above its
/// output size: one extra full-image summed-area-table buffer (8
/// bytes/pixel, `u64`), inherent to the O(1)-per-pixel technique this
/// algorithm is built on. A caller processing untrusted, unbounded image
/// dimensions should size-check before calling, the same way it would
/// before allocating the `PixelContainer` itself.
pub fn adaptive_threshold_mean(src: &PixelContainer, window: u32, t: f64) -> PixelContainer {
    let width = src.width;
    let height = src.height;
    let mut out = PixelContainer::new(width, height);
    if width == 0 || height == 0 {
        return out;
    }

    let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    let radius = (window.max(1) / 2) as i64;

    let w = width as usize;
    let h = height as usize;

    // BT.601 luminance per pixel, matching threshold_luminance's formula.
    // Computed inline (here and again in the classification pass below)
    // rather than materialized into its own full-image buffer -- caught in
    // security review: this function already needs one extra full-image
    // scratch buffer beyond its output (the summed-area table itself, 8
    // bytes/pixel, inherent to the O(1)-per-pixel technique) on top of the
    // caller's own image; a second one (this luminance buffer, 4
    // bytes/pixel) was avoidable extra peak-memory amplification for a
    // caller-controlled image size, and cheap to avoid (BT.601 is three
    // multiplies and two adds -- recomputing it is not a meaningful cost
    // next to the integral-image lookups either pass already does).
    let luminance = |x: u32, y: u32| -> u32 {
        let (r, g, b, _a) = src.pixel_at(x, y);
        (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round() as u32
    };

    // Summed-area table: (w+1) x (h+1), row 0 / column 0 are a zero border.
    let stride = w + 1;
    let mut integral = vec![0u64; stride * (h + 1)];
    for y in 0..h {
        let mut row_sum = 0u64;
        for x in 0..w {
            row_sum += luminance(x as u32, y as u32) as u64;
            integral[(y + 1) * stride + (x + 1)] = integral[y * stride + (x + 1)] + row_sum;
        }
    }

    for y in 0..h {
        let y0 = (y as i64 - radius).max(0) as usize;
        let y1 = (y as i64 + radius).min(h as i64 - 1) as usize;
        for x in 0..w {
            let x0 = (x as i64 - radius).max(0) as usize;
            let x1 = (x as i64 + radius).min(w as i64 - 1) as usize;
            let area = ((x1 - x0 + 1) * (y1 - y0 + 1)) as u64;
            // Add the two positive terms before subtracting the two negative
            // ones: BR - TR - BL + TL is mathematically always >= 0 (it's a
            // sum of non-negative luminance values), but evaluated strictly
            // left-to-right in u64 the intermediate `BR - TR - BL` step can
            // go negative and panic even though the final combined value
            // never would -- TR and BL individually cover more rows/columns
            // than the two positive terms restrict to on their own.
            let sum = (integral[(y1 + 1) * stride + (x1 + 1)] + integral[y0 * stride + x0])
                - (integral[y0 * stride + (x1 + 1)] + integral[(y1 + 1) * stride + x0]);
            let mean = sum as f64 / area as f64;
            let dark = (luminance(x as u32, y as u32) as f64) < mean * (1.0 - t);
            let v = if dark { 0 } else { 255 };
            let (_, _, _, a) = src.pixel_at(x as u32, y as u32);
            out.set_pixel(x as u32, y as u32, v, v, v, a);
        }
    }

    out
}

/// Quantise each channel to `levels` distinct values.
/// `levels` must be >= 2.
pub fn posterize(src: &PixelContainer, levels: u8) -> PixelContainer {
    assert!(levels >= 2, "posterize: levels must be >= 2");
    let step = 255.0 / (levels - 1) as f32;
    map_pixels(src, |r, g, b, a| {
        let q = |c: u8| ((c as f32 / step).round() * step).round().clamp(0.0, 255.0) as u8;
        (q(r), q(g), q(b), a)
    })
}

/// Swap red and blue channels (RGB ↔ BGR).  Green and alpha unchanged.
pub fn swap_rgb_bgr(src: &PixelContainer) -> PixelContainer {
    map_pixels(src, |r, g, b, a| (b, g, r, a))
}

#[derive(Clone, Copy)]
pub enum Channel { R, G, B, A }

/// Extract a single channel as a greyscale image (all RGB set to that channel's value).
/// Alpha is set to 255 (fully opaque).
pub fn extract_channel(src: &PixelContainer, ch: Channel) -> PixelContainer {
    map_pixels(src, |r, g, b, a| {
        let v = match ch { Channel::R => r, Channel::G => g, Channel::B => b, Channel::A => a };
        (v, v, v, 255)
    })
}

/// Additive brightness shift in sRGB u8.  `delta` ∈ [-255, 255].
/// Clamps to [0, 255]; alpha unchanged.
pub fn brightness(src: &PixelContainer, delta: i16) -> PixelContainer {
    map_pixels(src, |r, g, b, a| {
        let adj = |c: u8| (c as i16 + delta).clamp(0, 255) as u8;
        (adj(r), adj(g), adj(b), a)
    })
}

// ── Linear-light operations ───────────────────────────────────────────────────

/// Contrast stretch/compress around the sRGB midpoint (128).
/// `factor` > 0.0: expand; 0.0 = no change; < 0.0 (> -1.0): compress.
/// Uses the classic Photoshop-style formula; operates in sRGB u8 (approximate).
pub fn contrast(src: &PixelContainer, factor: f32) -> PixelContainer {
    let f = (259.0 * (factor * 255.0 + 255.0)) / (255.0 * (259.0 - factor * 255.0));
    map_pixels(src, |r, g, b, a| {
        let adj = |c: u8| ((f * (c as f32 - 128.0) + 128.0).round().clamp(0.0, 255.0)) as u8;
        (adj(r), adj(g), adj(b), a)
    })
}

/// Apply a power-law gamma curve in linear light.
/// γ < 1 brightens midtones; γ > 1 darkens; γ = 1 is identity.
pub fn gamma(src: &PixelContainer, gamma: f32) -> PixelContainer {
    map_pixels(src, |r, g, b, a| (
        encode(decode(r).powf(gamma)),
        encode(decode(g).powf(gamma)),
        encode(decode(b).powf(gamma)),
        a,
    ))
}

/// Multiply linear-light values by 2^ev_stops (exposure adjustment).
/// ev_stops = +1 doubles luminance; -1 halves it.
pub fn exposure(src: &PixelContainer, ev_stops: f32) -> PixelContainer {
    let scale = 2f32.powf(ev_stops);
    map_pixels(src, |r, g, b, a| (
        encode((decode(r) * scale).clamp(0.0, 1.0)),
        encode((decode(g) * scale).clamp(0.0, 1.0)),
        encode((decode(b) * scale).clamp(0.0, 1.0)),
        a,
    ))
}

#[derive(Clone, Copy)]
pub enum LuminanceWeights {
    /// Rec.709 / sRGB: 0.2126 R + 0.7152 G + 0.0722 B  (linear light)
    Rec709,
    /// BT.601 standard definition: 0.299 R + 0.587 G + 0.114 B
    Bt601,
    /// Simple average: (R + G + B) / 3
    Average,
}

/// Convert to greyscale in linear light using the specified luminance weights.
pub fn greyscale(src: &PixelContainer, weights: LuminanceWeights) -> PixelContainer {
    let (wr, wg, wb) = match weights {
        LuminanceWeights::Rec709  => (0.2126_f32, 0.7152_f32, 0.0722_f32),
        LuminanceWeights::Bt601   => (0.299_f32,  0.587_f32,  0.114_f32),
        LuminanceWeights::Average => (1.0/3.0,    1.0/3.0,    1.0/3.0),
    };
    map_pixels(src, |r, g, b, a| {
        let y = wr * decode(r) + wg * decode(g) + wb * decode(b);
        let v = encode(y);
        (v, v, v, a)
    })
}

/// Sepia tone: desaturate then tint with warm brown in linear light.
pub fn sepia(src: &PixelContainer) -> PixelContainer {
    map_pixels(src, |r, g, b, a| {
        let y = 0.2126 * decode(r) + 0.7152 * decode(g) + 0.0722 * decode(b);
        (
            encode((y * 1.351).clamp(0.0, 1.0)),
            encode((y * 1.203).clamp(0.0, 1.0)),
            encode((y * 0.937).clamp(0.0, 1.0)),
            a,
        )
    })
}

/// Apply a 3×3 colour matrix in linear light.
/// `matrix[row][col]` — applied as: out_rgb = M × in_rgb.
/// Alpha is unchanged.
pub fn colour_matrix(src: &PixelContainer, matrix: &[[f32; 3]; 3]) -> PixelContainer {
    map_pixels(src, |r, g, b, a| {
        let (rl, gl, bl) = (decode(r), decode(g), decode(b));
        let ro = matrix[0][0]*rl + matrix[0][1]*gl + matrix[0][2]*bl;
        let go = matrix[1][0]*rl + matrix[1][1]*gl + matrix[1][2]*bl;
        let bo = matrix[2][0]*rl + matrix[2][1]*gl + matrix[2][2]*bl;
        (encode(ro.clamp(0.0,1.0)), encode(go.clamp(0.0,1.0)), encode(bo.clamp(0.0,1.0)), a)
    })
}

/// Adjust saturation by a scalar factor in linear light via a colour matrix.
/// `factor` = 0 → greyscale; 1 → no change; > 1 → oversaturated.
pub fn saturate(src: &PixelContainer, factor: f32) -> PixelContainer {
    let (yr, yg, yb) = (0.2126_f32, 0.7152_f32, 0.0722_f32);
    let m = [
        [yr + factor*(1.0-yr),  yg - factor*yg,       yb - factor*yb      ],
        [yr - factor*yr,        yg + factor*(1.0-yg),  yb - factor*yb      ],
        [yr - factor*yr,        yg - factor*yg,        yb + factor*(1.0-yb)],
    ];
    colour_matrix(src, &m)
}

/// Rotate hue by `degrees` in HSV space (converted from/to linear light).
pub fn hue_rotate(src: &PixelContainer, degrees: f32) -> PixelContainer {
    map_pixels(src, |r, g, b, a| {
        let (rl, gl, bl) = (decode(r), decode(g), decode(b));
        let (h, s, v) = rgb_to_hsv(rl, gl, bl);
        let h2 = (h + degrees).rem_euclid(360.0);
        let (ro, go, bo) = hsv_to_rgb(h2, s, v);
        (encode(ro), encode(go), encode(bo), a)
    })
}

// ── Colorspace conversion ─────────────────────────────────────────────────────

/// Convert sRGB u8 → linear f32 and store back as u8 (quantised).
/// The result is a linear-light image packed into RGBA8.
pub fn srgb_to_linear_image(src: &PixelContainer) -> PixelContainer {
    map_pixels(src, |r, g, b, a| (
        (decode(r) * 255.0).round() as u8,
        (decode(g) * 255.0).round() as u8,
        (decode(b) * 255.0).round() as u8,
        a,
    ))
}

/// Convert linear f32 (packed as u8) → sRGB u8.
pub fn linear_to_srgb_image(src: &PixelContainer) -> PixelContainer {
    map_pixels(src, |r, g, b, a| (
        encode(r as f32 / 255.0),
        encode(g as f32 / 255.0),
        encode(b as f32 / 255.0),
        a,
    ))
}

// ── 1D LUT application ────────────────────────────────────────────────────────

/// Apply per-channel 256-entry u8 LUTs to an RGBA8 image.
/// Alpha is unchanged.
pub fn apply_lut1d_u8(
    src: &PixelContainer,
    r_lut: &[u8; 256],
    g_lut: &[u8; 256],
    b_lut: &[u8; 256],
) -> PixelContainer {
    map_pixels(src, |r, g, b, a| (r_lut[r as usize], g_lut[g as usize], b_lut[b as usize], a))
}

/// Build a 256-entry u8 LUT from a closure mapping [0,255] → [0,255].
pub fn build_lut1d_u8(f: impl Fn(u8) -> u8) -> [u8; 256] {
    std::array::from_fn(|i| f(i as u8))
}

/// Build a gamma LUT (sRGB encode/decode aware): output[i] = encode(decode(i/255)^γ)*255.
pub fn build_gamma_lut(gamma: f32) -> [u8; 256] {
    build_lut1d_u8(|i| encode(decode(i).powf(gamma)))
}

// ── Private HSV helpers ───────────────────────────────────────────────────────

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let cmax = r.max(g).max(b);
    let cmin = r.min(g).min(b);
    let delta = cmax - cmin;
    let h = if delta < 1e-6 {
        0.0
    } else if cmax == r {
        60.0 * (((g - b) / delta).rem_euclid(6.0))
    } else if cmax == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let s = if cmax < 1e-6 { 0.0 } else { delta / cmax };
    (h, s, cmax)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 { (c,x,0.0) } else if h < 120.0 { (x,c,0.0) }
        else if h < 180.0 { (0.0,c,x) } else if h < 240.0 { (0.0,x,c) }
        else if h < 300.0 { (x,0.0,c) } else { (c,0.0,x) };
    ((r1+m).clamp(0.0,1.0), (g1+m).clamp(0.0,1.0), (b1+m).clamp(0.0,1.0))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(r: u8, g: u8, b: u8, a: u8) -> PixelContainer {
        let mut pc = PixelContainer::new(2, 2);
        pc.fill(r, g, b, a);
        pc
    }

    #[test]
    fn test_invert_rgb() {
        let img = solid(100, 150, 200, 255);
        let out = invert(&img);
        let (r, g, b, a) = out.pixel_at(0, 0);
        assert_eq!((r, g, b, a), (155, 105, 55, 255));
    }

    #[test]
    fn test_invert_preserves_alpha() {
        let img = solid(0, 0, 0, 128);
        let out = invert(&img);
        assert_eq!(out.pixel_at(0, 0).3, 128);
    }

    #[test]
    fn test_threshold_above() {
        let img = solid(200, 100, 50, 255);
        let out = threshold(&img, 128);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert_eq!((r, g, b), (255, 0, 0));
    }

    #[test]
    fn test_posterize_two_levels() {
        let img = solid(200, 100, 50, 255);
        let out = posterize(&img, 2);
        let (r, g, b, _) = out.pixel_at(0, 0);
        // with 2 levels, step=255: values >= 127.5 → 255, < 127.5 → 0
        assert_eq!((r, g, b), (255, 0, 0));
    }

    #[test]
    fn test_swap_rgb_bgr() {
        let img = solid(255, 0, 0, 255); // pure red
        let out = swap_rgb_bgr(&img);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert_eq!((r, g, b), (0, 0, 255)); // becomes pure blue
    }

    #[test]
    fn test_brightness_clamps() {
        let img = solid(250, 10, 10, 255);
        let out = brightness(&img, 20);
        let (r, g, _b, _) = out.pixel_at(0, 0);
        assert_eq!(r, 255); // clamped
        assert_eq!(g, 30);
    }

    #[test]
    fn test_gamma_identity() {
        let img = solid(128, 64, 200, 255);
        let out = gamma(&img, 1.0);
        // γ=1 is identity (within rounding)
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert!((r as i16 - 128).abs() <= 1);
        assert!((g as i16 - 64).abs() <= 1);
        assert!((b as i16 - 200).abs() <= 1);
    }

    #[test]
    fn test_gamma_brightens_midtones() {
        let img = solid(128, 128, 128, 255);
        let out = gamma(&img, 0.5); // γ < 1 brightens
        let (r, _, _, _) = out.pixel_at(0, 0);
        assert!(r > 128, "γ < 1 should brighten midtones");
    }

    #[test]
    fn test_exposure_doubles_light() {
        let img = solid(100, 100, 100, 255);
        let out = exposure(&img, 1.0); // +1 EV: double the linear light
        let (r, _, _, _) = out.pixel_at(0, 0);
        assert!(r > 100, "+1 EV should brighten");
    }

    #[test]
    fn test_greyscale_white_stays_white() {
        let img = solid(255, 255, 255, 255);
        let out = greyscale(&img, LuminanceWeights::Rec709);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn test_greyscale_black_stays_black() {
        let img = solid(0, 0, 0, 255);
        let out = greyscale(&img, LuminanceWeights::Rec709);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert_eq!((r, g, b), (0, 0, 0));
    }

    #[test]
    fn test_sepia_neutral_grey() {
        // neutral grey in → warm brown out (R > G > B)
        let img = solid(128, 128, 128, 255);
        let out = sepia(&img);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert!(r > g && g > b, "sepia should produce warm brown (R>G>B)");
    }

    #[test]
    fn test_colour_matrix_identity() {
        let m = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let img = solid(100, 150, 200, 255);
        let out = colour_matrix(&img, &m);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert!((r as i16 - 100).abs() <= 1);
        assert!((g as i16 - 150).abs() <= 1);
        assert!((b as i16 - 200).abs() <= 1);
    }

    #[test]
    fn test_saturate_zero_gives_grey() {
        let img = solid(200, 100, 50, 255);
        let out = saturate(&img, 0.0);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert_eq!(r, g);
        assert_eq!(g, b);
    }

    #[test]
    fn test_hue_rotate_360_identity() {
        let img = solid(200, 100, 50, 255);
        let out = hue_rotate(&img, 360.0);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert!((r as i16 - 200).abs() <= 2);
        assert!((g as i16 - 100).abs() <= 2);
        assert!((b as i16 - 50).abs() <= 2);
    }

    #[test]
    fn test_apply_lut1d_invert() {
        let img = solid(100, 150, 200, 255);
        let lut: [u8; 256] = std::array::from_fn(|i| 255 - i as u8);
        let out = apply_lut1d_u8(&img, &lut, &lut, &lut);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert_eq!((r, g, b), (155, 105, 55));
    }

    #[test]
    fn test_build_gamma_lut_identity() {
        let lut = build_gamma_lut(1.0);
        // With γ=1 the LUT should round-trip through decode/encode ≈ identity
        for i in 0u8..=255 {
            assert!((lut[i as usize] as i16 - i as i16).abs() <= 1,
                "lut[{}] = {} expected ~{}", i, lut[i as usize], i);
        }
    }

    #[test]
    fn test_threshold_luminance_white() {
        let img = solid(255, 255, 255, 255);
        let out = threshold_luminance(&img, 128);
        let (r, g, b, _) = out.pixel_at(0, 0);
        assert_eq!((r, g, b), (255, 255, 255));
    }

    #[test]
    fn test_extract_channel_red() {
        let img = solid(200, 100, 50, 255);
        let out = extract_channel(&img, Channel::R);
        let (r, g, b, a) = out.pixel_at(0, 0);
        assert_eq!((r, g, b, a), (200, 200, 200, 255));
    }

    #[test]
    fn test_dimensions_preserved() {
        let img = PixelContainer::new(7, 13);
        assert_eq!(invert(&img).width, 7);
        assert_eq!(invert(&img).height, 13);
    }

    // ── adaptive_threshold_mean (IMG09) ─────────────────────────────────────

    /// A small mark (a few dark pixels) inside a uniform-luminance patch.
    fn patch_with_mark(w: u32, h: u32, bg: u8, mark_v: u8, mark_x: u32, mark_y: u32) -> PixelContainer {
        let mut pc = PixelContainer::new(w, h);
        pc.fill(bg, bg, bg, 255);
        pc.set_pixel(mark_x, mark_y, mark_v, mark_v, mark_v, 255);
        pc
    }

    #[test]
    fn test_adaptive_threshold_solves_what_global_cannot() {
        // Left half: bright background (220) with a moderately-dark mark
        // (100). Right half: dark background (90) with a very dark mark
        // (20). Crucially, the left mark (100) is *brighter* than the
        // right background (90) -- so no single global cutoff t can
        // satisfy "left mark < t" (needs t > 100) and "right background
        // >= t" (needs t <= 90) at the same time; that range is empty.
        // Adaptive thresholding, which compares each pixel to its own
        // local mean rather than one global number, has no such
        // contradiction.
        let w = 20u32;
        let h = 10u32;
        let mut img = PixelContainer::new(w, h);
        for y in 0..h {
            for x in 0..w {
                let bg = if x < w / 2 { 220 } else { 90 };
                img.set_pixel(x, y, bg, bg, bg, 255);
            }
        }
        let (left_mark_x, left_mark_y) = (3, 5);
        let (right_mark_x, right_mark_y) = (16, 5);
        img.set_pixel(left_mark_x, left_mark_y, 100, 100, 100, 255);
        img.set_pixel(right_mark_x, right_mark_y, 20, 20, 20, 255);

        // No global threshold works: prove it first.
        let mut any_global_works = false;
        for t in 0u16..=255 {
            let out = threshold_luminance(&img, t as u8);
            let left_dark = out.pixel_at(left_mark_x, left_mark_y).0 == 0;
            let right_bg_light = out.pixel_at(w - 2, h / 2).0 == 255;
            if left_dark && right_bg_light {
                any_global_works = true;
                break;
            }
        }
        assert!(!any_global_works, "test setup should defeat every global cutoff");

        let out = adaptive_threshold_mean(&img, 7, 0.15);
        assert_eq!(out.pixel_at(left_mark_x, left_mark_y).0, 0, "left mark should be foreground");
        assert_eq!(out.pixel_at(right_mark_x, right_mark_y).0, 0, "right mark should be foreground");
        assert_eq!(out.pixel_at(1, 1).0, 255, "left background should stay background");
        assert_eq!(out.pixel_at(w - 2, h / 2).0, 255, "right background should stay background");
    }

    #[test]
    fn test_adaptive_threshold_uniform_image_matches_global_regression() {
        let img = patch_with_mark(12, 12, 200, 30, 6, 6);
        let adaptive = adaptive_threshold_mean(&img, 5, 0.15);
        // On a uniformly-lit image a correctly-chosen global cutoff between
        // the mark (30) and the background (200) should agree with adaptive.
        let global = threshold_luminance(&img, 115);
        for y in 0..12 {
            for x in 0..12 {
                assert_eq!(
                    adaptive.pixel_at(x, y).0,
                    global.pixel_at(x, y).0,
                    "mismatch at ({x},{y})"
                );
            }
        }
    }

    #[test]
    fn test_adaptive_threshold_edges_do_not_panic() {
        let img = patch_with_mark(9, 9, 180, 40, 0, 0);
        let out = adaptive_threshold_mean(&img, 9, 0.15);
        // Every corner and edge midpoint just needs to be a valid binary
        // value (0 or 255), proving the clipped window didn't corrupt them.
        let probes = [(0, 0), (8, 0), (0, 8), (8, 8), (4, 0), (0, 4), (8, 4), (4, 8)];
        for (x, y) in probes {
            let v = out.pixel_at(x, y).0;
            assert!(v == 0 || v == 255, "({x},{y}) = {v} is not a valid binary value");
        }
    }

    #[test]
    fn test_adaptive_threshold_degenerate_inputs_do_not_panic() {
        let empty_w = PixelContainer::new(0, 5);
        let out = adaptive_threshold_mean(&empty_w, 5, 0.15);
        assert_eq!((out.width, out.height), (0, 5));

        let empty_h = PixelContainer::new(5, 0);
        let out = adaptive_threshold_mean(&empty_h, 5, 0.15);
        assert_eq!((out.width, out.height), (5, 0));

        let both_zero = PixelContainer::new(0, 0);
        let out = adaptive_threshold_mean(&both_zero, 5, 0.15);
        assert_eq!((out.width, out.height), (0, 0));

        let img = patch_with_mark(6, 6, 200, 30, 3, 3);
        let _ = adaptive_threshold_mean(&img, 0, 0.15); // window=0 clamped to 1
        let _ = adaptive_threshold_mean(&img, 5, f64::NAN);
        let _ = adaptive_threshold_mean(&img, 5, -1.0);
        let _ = adaptive_threshold_mean(&img, 5, 2.0);
    }

    #[test]
    fn test_adaptive_threshold_window_zero_is_all_background() {
        // window=0 clamps to 1: every pixel's "local mean" is itself, and
        // lum < lum * (1.0 - t) is never true for t >= 0 and lum >= 0 --
        // deterministically all-background, not a special case in the code.
        let img = patch_with_mark(6, 6, 200, 30, 3, 3);
        let out = adaptive_threshold_mean(&img, 0, 0.15);
        for y in 0..6 {
            for x in 0..6 {
                assert_eq!(out.pixel_at(x, y).0, 255);
            }
        }
    }
}
