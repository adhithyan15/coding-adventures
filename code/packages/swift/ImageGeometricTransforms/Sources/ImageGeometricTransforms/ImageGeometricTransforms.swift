// ImageGeometricTransforms.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// MARK: - IMG04: Spatial / Geometric Image Transforms
// ============================================================================
//
// A geometric (spatial) transform remaps pixel *positions*, rather than
// changing pixel *values* in place.  The fundamental question is:
//
//   "For output pixel (x', y'), which input pixel (u, v) should I copy?"
//
// This is the **inverse mapping** (output → input).  Inverse mapping is
// always preferred over forward mapping because it guarantees that every
// output pixel is filled — forward mapping can leave holes.
//
// ## Two classes of operations
//
// ### 1. Lossless pixel-copy operations
// Flip, 90°/180° rotation, crop, and pad copy pixels exactly.  No
// colour-space conversion is needed because no channel values are mixed.
//
// ### 2. Continuous-coordinate resampling operations
// Scale, arbitrary rotation, affine warp, and perspective warp compute a
// real-valued source coordinate (u, v) for each output pixel and then
// *sample* the source image at that fractional location using one of three
// interpolation kernels:
//
//   Nearest-neighbour — round (u,v) to the closest integer pixel.
//                       Fastest; produces visible staircase ("pixelated") edges.
//
//   Bilinear          — blend the 2×2 neighbourhood with linear weights.
//                       Smooth at moderate magnification; slight blur.
//
//   Bicubic (Catmull-Rom) — blend the 4×4 neighbourhood with cubic spline
//                       weights.  Sharper than bilinear; may ring slightly
//                       at very high-contrast edges.
//
// ## sRGB and interpolation
//
// Interpolation blends channel values.  Blending in raw sRGB space is
// geometrically incorrect: the sRGB transfer curve is perceptually uniform
// but not linearly proportional to light intensity.  Averaging the bytes
// 0 (black) and 255 (white) gives 128, which looks too dark.
//
// The correct procedure is:
//   1. Decode sRGB bytes to linear-light Float via the sRGB EOTF.
//   2. Perform weighted blending in linear light.
//   3. Re-encode to sRGB bytes via the inverse EOTF.
//
// All interpolating samplers in this module follow this procedure.
//
// ## Out-of-bounds (OOB) strategies
//
// When a sampler requests a coordinate outside the source image:
//
//   .zero      — return transparent black (0,0,0,0).  Good for padding.
//   .replicate — clamp to the nearest border pixel.  Avoids dark halos.
//   .reflect   — mirror the image at each border.  Good for convolutions.
//   .wrap      — tile the image periodically.  Good for textures.
//
// ============================================================================

import Foundation
import PixelContainer

// ============================================================================
// MARK: - Public Types
// ============================================================================

/// Interpolation kernel used when resampling at a fractional source coordinate.
///
/// Choose based on the quality/speed trade-off required:
///   - `.nearest`  — fastest, blocky at large upscales
///   - `.bilinear` — smooth, slight blur, good general purpose
///   - `.bicubic`  — sharpest, based on Catmull-Rom cubic spline
public enum Interpolation {
    case nearest
    case bilinear
    case bicubic
}

/// Controls the output canvas size for arbitrary-angle rotation.
///
///   `.fit`  — enlarge the canvas to contain the entire rotated source image.
///             No pixels are clipped; corners of the source appear against
///             a transparent-black background.
///   `.crop` — keep the same dimensions as the source image.  Corners of
///             the rotated image are clipped to the canvas boundary.
public enum RotateBounds {
    case fit
    case crop
}

/// Determines what colour to return when a sampler looks outside the source.
///
///   `.zero`      — transparent black (0,0,0,0)
///   `.replicate` — nearest border pixel (edge padding)
///   `.reflect`   — mirror-reflect across the border
///   `.wrap`      — tile periodically
public enum OutOfBounds {
    case zero
    case replicate
    case reflect
    case wrap
}

/// Convenience alias: a pixel as (red, green, blue, alpha), each 0–255.
public typealias Rgba8 = (UInt8, UInt8, UInt8, UInt8)

// ============================================================================
// MARK: - sRGB ↔ Linear LUT
// ============================================================================
//
// The sRGB standard defines a piecewise transfer function ("gamma curve")
// that maps device signal (0–255) to linear light intensity (0–1):
//
//   Decode  c = byte / 255
//           c ≤ 0.04045   →  c / 12.92
//           else          →  ((c + 0.055) / 1.055) ^ 2.4
//
//   Encode  c ≤ 0.0031308 →  12.92 * c
//           else          →  1.055 * c^(1/2.4) − 0.055
//
// We precompute a 256-entry decode LUT at module load to avoid repeated
// calls to `pow` in the inner loops of the interpolation kernels.

/// 256-entry sRGB-byte → linear-Float decode table.
private let srgbToLinear: [Float] = (0..<256).map { i -> Float in
    let c = Float(i) / 255.0
    return c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4)
}

/// Decode one sRGB byte to a linear-light Float in [0, 1].
private func decode(_ b: UInt8) -> Float { srgbToLinear[Int(b)] }

/// Encode a linear-light Float back to a clamped sRGB byte.
///
/// Values below 0 or above 1 are clamped before encoding.
private func encode(_ v: Float) -> UInt8 {
    let c = v <= 0.0031308 ? 12.92 * v : 1.055 * pow(v, 1.0 / 2.4) - 0.055
    let clamped = min(1.0 as Float, max(0.0 as Float, c))
    return UInt8(min(255, max(0, Int((clamped * 255).rounded()))))
}

// ============================================================================
// MARK: - Out-of-Bounds Coordinate Resolution
// ============================================================================
//
// Before reading from the source image at integer coordinates (ix, iy), every
// sampler passes those coordinates through `resolveCoord` to handle the four
// OOB strategies:
//
//   .zero      — nil signals "return transparent black"
//   .replicate — clamp: [0, max−1]
//   .reflect   — fold coordinates back into [0, max−1] with mirror symmetry
//   .wrap      — modulo with positive-remainder correction
//
// The reflect formula uses period = 2 * max:
//
//   r = ((x mod period) + period) mod period
//   if r >= max: r = period − 1 − r
//
// Example with max=4 (pixels 0,1,2,3):
//   x = -1 → r = 3  (mirror: edge pixel 0 reflects to edge pixel 0, -1 → 1)
//             Actually: period=8, r = ((-1 % 8)+8)%8 = 7; 7 >= 4 → 8-1-7 = 0.
//   x =  4 → r = 3  (period=8, r=4; 4>=4 → 8-1-4=3)
//   x =  5 → r = 2  (r=5; 5>=4 → 8-1-5=2)

/// Resolve a 1-D integer coordinate under a given out-of-bounds strategy.
///
/// - Parameters:
///   - x:   The (possibly out-of-range) coordinate.
///   - max:  One past the last valid index (= image width or height).
///   - oob:  The out-of-bounds handling mode.
/// - Returns: A valid coordinate in [0, max), or `nil` for `.zero`.
private func resolveCoord(_ x: Int, max: Int, oob: OutOfBounds) -> Int? {
    switch oob {
    case .zero:
        // If in range, return as-is; otherwise signal "use transparent black".
        return (x >= 0 && x < max) ? x : nil

    case .replicate:
        // Clamp to the nearest valid coordinate.
        // x < 0 → 0;  x >= max → max − 1.
        return min(max - 1, Swift.max(0, x))

    case .reflect:
        // Mirror-pad: fold the coordinate with period = 2 * max.
        let period = 2 * max
        var r = ((x % period) + period) % period
        if r >= max { r = period - 1 - r }
        return r

    case .wrap:
        // Tile periodically with positive-remainder modulo.
        return ((x % max) + max) % max
    }
}

// ============================================================================
// MARK: - Catmull-Rom Cubic Kernel
// ============================================================================
//
// Catmull-Rom is a piecewise cubic spline that passes through its knot points,
// making it suitable for bicubic image interpolation.  The kernel weight for a
// distance `d` (in pixels) is:
//
//   |d| < 1 :  1.5|d|³ − 2.5|d|² + 1
//   |d| < 2 : −0.5|d|³ + 2.5|d|² − 4|d| + 2
//   else    :  0
//
// It integrates (sums over integer d) to 1, so colour is conserved.
// The 4×4 bicubic pass samples at d ∈ {−1, 0, 1, 2} relative to the
// floored source coordinate.

/// Catmull-Rom cubic weight for distance `d` (in pixels, may be fractional).
private func catmullRom(_ d: Float) -> Float {
    let d = abs(d)
    if d < 1 { return 1.5 * d * d * d - 2.5 * d * d + 1 }
    if d < 2 { return -0.5 * d * d * d + 2.5 * d * d - 4 * d + 2 }
    return 0
}

// ============================================================================
// MARK: - Samplers (private)
// ============================================================================
//
// Each sampler takes a real-valued source coordinate (u, v) and returns the
// interpolated colour as Rgba8.  The alpha channel is interpolated in linear
// space just like the colour channels.

/// Nearest-neighbour sampler.
///
/// Rounds (u, v) to the nearest integer pixel and returns it unchanged.
/// No blending, no colour-space conversion — the fastest possible sampler.
private func sampleNearest(
    _ img: PixelContainer,
    u: Float, v: Float,
    oob: OutOfBounds
) -> Rgba8 {
    let ix = Int(u.rounded())
    let iy = Int(v.rounded())
    guard let rx = resolveCoord(ix, max: Int(img.width),  oob: oob),
          let ry = resolveCoord(iy, max: Int(img.height), oob: oob)
    else { return (0, 0, 0, 0) }
    return pixelAt(img, x: UInt32(rx), y: UInt32(ry))
}

/// Bilinear sampler.
///
/// Blends the 2×2 neighbourhood of (u, v) with linear weights.
/// Decodes each neighbour to linear light, blends, then re-encodes to sRGB.
///
/// Let (ix, iy) = floor(u, v), and (tx, ty) = fractional parts.
/// The four neighbours are at offsets (0,0), (1,0), (0,1), (1,1).
/// The blend is:
///
///   result = (1-tx)(1-ty)*P00 + tx(1-ty)*P10 + (1-tx)ty*P01 + tx*ty*P11
private func sampleBilinear(
    _ img: PixelContainer,
    u: Float, v: Float,
    oob: OutOfBounds
) -> Rgba8 {
    let ix = Int(floor(u))
    let iy = Int(floor(v))
    let tx = u - Float(ix)   // fractional x in [0, 1)
    let ty = v - Float(iy)   // fractional y in [0, 1)

    // Helper: read one neighbour (possibly out-of-bounds) as linear Floats.
    func linearPixel(dx: Int, dy: Int) -> (Float, Float, Float, Float) {
        if let rx = resolveCoord(ix + dx, max: Int(img.width),  oob: oob),
           let ry = resolveCoord(iy + dy, max: Int(img.height), oob: oob) {
            let (r, g, b, a) = pixelAt(img, x: UInt32(rx), y: UInt32(ry))
            return (decode(r), decode(g), decode(b), Float(a) / 255.0)
        }
        return (0, 0, 0, 0)
    }

    let (r00, g00, b00, a00) = linearPixel(dx: 0, dy: 0)
    let (r10, g10, b10, a10) = linearPixel(dx: 1, dy: 0)
    let (r01, g01, b01, a01) = linearPixel(dx: 0, dy: 1)
    let (r11, g11, b11, a11) = linearPixel(dx: 1, dy: 1)

    // Bilinear blend weights.
    let w00 = (1 - tx) * (1 - ty)
    let w10 =      tx  * (1 - ty)
    let w01 = (1 - tx) *      ty
    let w11 =      tx  *      ty

    let r = w00 * r00 + w10 * r10 + w01 * r01 + w11 * r11
    let g = w00 * g00 + w10 * g10 + w01 * g01 + w11 * g11
    let b = w00 * b00 + w10 * b10 + w01 * b01 + w11 * b11
    let a = w00 * a00 + w10 * a10 + w01 * a01 + w11 * a11

    return (encode(r), encode(g), encode(b), UInt8(min(255, max(0, Int((a * 255).rounded())))))
}

/// Bicubic (Catmull-Rom) sampler.
///
/// Blends a 4×4 neighbourhood using the Catmull-Rom spline kernel.
/// Two-pass implementation: first blend each of the 4 rows horizontally,
/// then blend those 4 row-results vertically.
///
/// For source coordinate u with floor ix:
///   columns: ix−1, ix, ix+1, ix+2   (offsets −1, 0, 1, 2)
///   rows:    iy−1, iy, iy+1, iy+2
///
/// Each weight wx[j] = catmullRom(tx − (j − 1)) where tx = u − ix.
private func sampleBicubic(
    _ img: PixelContainer,
    u: Float, v: Float,
    oob: OutOfBounds
) -> Rgba8 {
    let ix = Int(floor(u))
    let iy = Int(floor(v))
    let tx = u - Float(ix)
    let ty = v - Float(iy)

    // 1-D Catmull-Rom weights for offsets −1, 0, 1, 2.
    let wx = (0..<4).map { j in catmullRom(tx - Float(j - 1)) }
    let wy = (0..<4).map { j in catmullRom(ty - Float(j - 1)) }

    // Helper: decode pixel at (ix+dx, iy+dy) as linear Floats.
    func lp(dx: Int, dy: Int) -> (Float, Float, Float, Float) {
        if let rx = resolveCoord(ix + dx, max: Int(img.width),  oob: oob),
           let ry = resolveCoord(iy + dy, max: Int(img.height), oob: oob) {
            let (r, g, b, a) = pixelAt(img, x: UInt32(rx), y: UInt32(ry))
            return (decode(r), decode(g), decode(b), Float(a) / 255.0)
        }
        return (0, 0, 0, 0)
    }

    // Blend each row horizontally.
    var rowR = [Float](repeating: 0, count: 4)
    var rowG = [Float](repeating: 0, count: 4)
    var rowB = [Float](repeating: 0, count: 4)
    var rowA = [Float](repeating: 0, count: 4)
    for row in 0..<4 {
        for col in 0..<4 {
            let (r, g, b, a) = lp(dx: col - 1, dy: row - 1)
            rowR[row] += wx[col] * r
            rowG[row] += wx[col] * g
            rowB[row] += wx[col] * b
            rowA[row] += wx[col] * a
        }
    }

    // Blend those row-results vertically.
    var r: Float = 0, g: Float = 0, b: Float = 0, a: Float = 0
    for row in 0..<4 {
        r += wy[row] * rowR[row]
        g += wy[row] * rowG[row]
        b += wy[row] * rowB[row]
        a += wy[row] * rowA[row]
    }

    return (encode(r), encode(g), encode(b), UInt8(min(255, max(0, Int((a * 255).rounded())))))
}

/// Dispatch to the selected interpolation kernel.
private func sample(
    _ img: PixelContainer,
    u: Float, v: Float,
    mode: Interpolation,
    oob: OutOfBounds
) -> Rgba8 {
    switch mode {
    case .nearest:  return sampleNearest(img, u: u, v: v, oob: oob)
    case .bilinear: return sampleBilinear(img, u: u, v: v, oob: oob)
    case .bicubic:  return sampleBicubic(img, u: u, v: v, oob: oob)
    }
}

// ============================================================================
// MARK: - Lossless Pixel-Copy Transforms
// ============================================================================
//
// These operations rearrange pixels without mixing values, so they are
// completely lossless — no sRGB conversion is needed.

// ── Flip ──────────────────────────────────────────────────────────────────

/// Flip the image horizontally (mirror left ↔ right).
///
/// Output pixel (x', y') = input pixel (W−1−x', y').
///
/// Applying `flipHorizontal` twice returns the original because
/// W−1−(W−1−x) == x.
public func flipHorizontal(_ src: PixelContainer) -> PixelContainer {
    let W = src.width, H = src.height
    var out = PixelContainer(width: W, height: H)
    for y: UInt32 in 0..<H {
        for x: UInt32 in 0..<W {
            let (r, g, b, a) = pixelAt(src, x: W - 1 - x, y: y)
            setPixel(&out, x: x, y: y, r: r, g: g, b: b, a: a)
        }
    }
    return out
}

/// Flip the image vertically (mirror top ↔ bottom).
///
/// Output pixel (x', y') = input pixel (x', H−1−y').
public func flipVertical(_ src: PixelContainer) -> PixelContainer {
    let W = src.width, H = src.height
    var out = PixelContainer(width: W, height: H)
    for y: UInt32 in 0..<H {
        for x: UInt32 in 0..<W {
            let (r, g, b, a) = pixelAt(src, x: x, y: H - 1 - y)
            setPixel(&out, x: x, y: y, r: r, g: g, b: b, a: a)
        }
    }
    return out
}

// ── 90° / 180° Rotation ───────────────────────────────────────────────────
//
// 90° CW rotation of a W×H source produces an H×W output.
//
// Deriving the inverse map by tracking the four corners:
//
//   Source (W×H) corner   →   Output (H×W) corner
//   top-left    (0,   0)  →   top-right    (H−1, 0)
//   top-right   (W−1, 0)  →   bottom-right (H−1, W−1)
//   bottom-left (0,   H−1)→   top-left     (0,   0)
//   bottom-right(W−1, H−1)→   bottom-left  (0,   W−1)
//
// For output (x', y') where x' ∈ [0, H−1] and y' ∈ [0, W−1]:
//   Inverse map:  x_src = y'   (range [0, W−1], valid for src.width=W)
//                 y_src = H−1−x'  (range [0, H−1], valid for src.height=H)
//
// For 90° CCW (W×H source → H×W output):
//   Inverse map:  x_src = W−1−y'  (range [0, W−1], valid for src.width=W)
//                 y_src = x'       (range [0, H−1], valid for src.height=H)
//
// Note: the spec document contains the formula "O[x'][y'] = I[y'][W−1−x']"
// which uses W where H is required for y_src, and similarly for CCW.  The
// correct expressions use src.height (H) for CW and src.width (W) for CCW.

/// Rotate the image 90° clockwise.
///
/// Output dimensions: W' = src.height, H' = src.width.
/// Inverse map: x_src = y', y_src = src.height − 1 − x'.
public func rotate90CW(_ src: PixelContainer) -> PixelContainer {
    let W = src.width, H = src.height
    // After 90° CW: new width = old height, new height = old width.
    var out = PixelContainer(width: H, height: W)
    for y: UInt32 in 0..<W {           // output row:    0 ..< old width
        for x: UInt32 in 0..<H {       // output column: 0 ..< old height
            // x_src = y (output row → source column)
            // y_src = H-1-x (output column maps into source row, reversed)
            let (r, g, b, a) = pixelAt(src, x: y, y: H - 1 - x)
            setPixel(&out, x: x, y: y, r: r, g: g, b: b, a: a)
        }
    }
    return out
}

/// Rotate the image 90° counter-clockwise.
///
/// Output dimensions: W' = src.height, H' = src.width.
/// Inverse map: x_src = src.width − 1 − y', y_src = x'.
public func rotate90CCW(_ src: PixelContainer) -> PixelContainer {
    let W = src.width, H = src.height
    var out = PixelContainer(width: H, height: W)
    for y: UInt32 in 0..<W {           // output row:    0 ..< old width
        for x: UInt32 in 0..<H {       // output column: 0 ..< old height
            // x_src = W-1-y (output row, reversed, → source column)
            // y_src = x     (output column → source row)
            let (r, g, b, a) = pixelAt(src, x: W - 1 - y, y: x)
            setPixel(&out, x: x, y: y, r: r, g: g, b: b, a: a)
        }
    }
    return out
}

/// Rotate the image 180°.
///
/// Output dimensions are unchanged.
/// Mapping: O[x'][y'] = I[W−1−x'][H−1−y'].
/// Equivalent to flipHorizontal ∘ flipVertical (or vice versa).
public func rotate180(_ src: PixelContainer) -> PixelContainer {
    let W = src.width, H = src.height
    var out = PixelContainer(width: W, height: H)
    for y: UInt32 in 0..<H {
        for x: UInt32 in 0..<W {
            let (r, g, b, a) = pixelAt(src, x: W - 1 - x, y: H - 1 - y)
            setPixel(&out, x: x, y: y, r: r, g: g, b: b, a: a)
        }
    }
    return out
}

// ── Crop ──────────────────────────────────────────────────────────────────

/// Crop a rectangular region from the source image.
///
/// Returns a new image of size (w × h) whose top-left corner maps to
/// source pixel (x, y).  If the crop window extends beyond the source
/// boundary, out-of-bounds source pixels return transparent black.
///
/// - Parameters:
///   - src: Source image.
///   - x:   Left edge of the crop window (source column).
///   - y:   Top edge of the crop window (source row).
///   - w:   Width of the output image in pixels.
///   - h:   Height of the output image in pixels.
public func crop(
    _ src: PixelContainer,
    x: UInt32, y: UInt32,
    w: UInt32, h: UInt32
) -> PixelContainer {
    var out = PixelContainer(width: w, height: h)
    for oy: UInt32 in 0..<h {
        for ox: UInt32 in 0..<w {
            // Source coordinate: offset into src from the crop origin.
            let sx = x + ox
            let sy = y + oy
            let (r, g, b, a) = pixelAt(src, x: sx, y: sy)
            // pixelAt returns (0,0,0,0) for out-of-bounds; that is exactly
            // what we want for crops that extend past the source boundary.
            setPixel(&out, x: ox, y: oy, r: r, g: g, b: b, a: a)
        }
    }
    return out
}

// ── Pad ───────────────────────────────────────────────────────────────────

/// Add a border of constant-colour padding around the source image.
///
/// The output image is (left + src.width + right) × (top + src.height + bottom).
/// Interior pixels are copied from the source; border pixels are filled with
/// `fill` (default transparent black).
///
/// - Parameters:
///   - src:    Source image.
///   - top:    Rows of padding above the source.
///   - right:  Columns of padding to the right of the source.
///   - bottom: Rows of padding below the source.
///   - left:   Columns of padding to the left of the source.
///   - fill:   Colour for the border pixels.  Default (0, 0, 0, 0).
public func pad(
    _ src: PixelContainer,
    top: UInt32, right: UInt32, bottom: UInt32, left: UInt32,
    fill: Rgba8 = (0, 0, 0, 0)
) -> PixelContainer {
    let outW = left + src.width  + right
    let outH = top  + src.height + bottom
    var out = PixelContainer(width: outW, height: outH)

    // Fill the entire canvas with the border colour first.
    for y: UInt32 in 0..<outH {
        for x: UInt32 in 0..<outW {
            setPixel(&out, x: x, y: y, r: fill.0, g: fill.1, b: fill.2, a: fill.3)
        }
    }

    // Copy source pixels into the interior.
    for sy: UInt32 in 0..<src.height {
        for sx: UInt32 in 0..<src.width {
            let (r, g, b, a) = pixelAt(src, x: sx, y: sy)
            setPixel(&out, x: left + sx, y: top + sy, r: r, g: g, b: b, a: a)
        }
    }
    return out
}

// ============================================================================
// MARK: - Continuous-Coordinate Resampling Transforms
// ============================================================================

// ── Scale ─────────────────────────────────────────────────────────────────
//
// Scaling maps each output pixel (x', y') back to a source coordinate (u, v).
// We use the pixel-centre convention: pixel x occupies [x, x+1), so its centre
// is at x + 0.5.
//
// If the source has width W and the output has width W', the scale factor
// is sx = W' / W (output pixels per source pixel).  The source centre that
// maps to output centre x' + 0.5 is:
//
//   u = (x' + 0.5) / sx − 0.5
//     = (x' + 0.5) * (W / W') − 0.5
//
// This ensures that the leftmost output pixel samples near the leftmost source
// pixel, and the rightmost output pixel samples near the rightmost source
// pixel, with no padding artefact.

/// Resize the image to (outW × outH) using the selected interpolation.
///
/// Uses pixel-centre mapping with `.replicate` out-of-bounds handling so that
/// the single-pixel border of the source is correctly extended rather than
/// replaced with transparent black.
///
/// - Parameters:
///   - src:   Source image.
///   - outW:  Output width in pixels.
///   - outH:  Output height in pixels.
///   - mode:  Interpolation kernel.  Default `.bilinear`.
public func scale(
    _ src: PixelContainer,
    width outW: UInt32,
    height outH: UInt32,
    mode: Interpolation = .bilinear
) -> PixelContainer {
    var out = PixelContainer(width: outW, height: outH)
    // Scale factors: how many source pixels per output pixel.
    let sx = Float(src.width)  / Float(outW)
    let sy = Float(src.height) / Float(outH)

    for oy: UInt32 in 0..<outH {
        for ox: UInt32 in 0..<outW {
            // Pixel-centre mapping: map output centre → source centre.
            let u = (Float(ox) + 0.5) * sx - 0.5
            let v = (Float(oy) + 0.5) * sy - 0.5
            let px = sample(src, u: u, v: v, mode: mode, oob: .replicate)
            setPixel(&out, x: ox, y: oy, r: px.0, g: px.1, b: px.2, a: px.3)
        }
    }
    return out
}

// ── Arbitrary-Angle Rotation ──────────────────────────────────────────────
//
// Rotation by angle θ (radians, CCW positive) maps output coordinates to
// source coordinates via an inverse rotation:
//
//   dx = x' − cxOut       (offset from output centre)
//   dy = y' − cyOut
//
//   u = cxIn + cos(θ)*dx + sin(θ)*dy
//   v = cyIn − sin(θ)*dx + cos(θ)*dy
//
// Note: adding sin(θ)*dy (not −sin) gives the inverse (CW) rotation.
//
// For `.fit` bounds, the output canvas is expanded to contain all four
// rotated corners of the source:
//
//   outW = ceil(W |cos θ| + H |sin θ|)
//   outH = ceil(W |sin θ| + H |cos θ|)

/// Rotate the image by `radians` around its centre.
///
/// Positive `radians` rotates counter-clockwise (standard mathematical
/// convention).  The `.fit` bounds mode expands the canvas so no source
/// pixels are clipped; the `.crop` mode keeps the original canvas size.
///
/// Pixels outside the source image are filled with transparent black (`.zero`
/// OOB).  Use `pad` before rotating if you need a different border colour.
///
/// - Parameters:
///   - src:     Source image.
///   - radians: Rotation angle.  Positive = CCW.
///   - mode:    Interpolation kernel.  Default `.bilinear`.
///   - bounds:  Output canvas sizing strategy.  Default `.fit`.
public func rotate(
    _ src: PixelContainer,
    radians: Float,
    mode: Interpolation = .bilinear,
    bounds: RotateBounds = .fit
) -> PixelContainer {
    let W = Float(src.width)
    let H = Float(src.height)
    let cosA = Foundation.cos(radians)
    let sinA = Foundation.sin(radians)

    // Compute output dimensions.
    let outW: UInt32
    let outH: UInt32
    switch bounds {
    case .fit:
        // Enlarge canvas to contain all four rotated corners.
        outW = UInt32(ceil(W * abs(cosA) + H * abs(sinA)))
        outH = UInt32(ceil(W * abs(sinA) + H * abs(cosA)))
    case .crop:
        outW = src.width
        outH = src.height
    }

    let cxOut = Float(outW) / 2.0
    let cyOut = Float(outH) / 2.0
    let cxIn  = W / 2.0
    let cyIn  = H / 2.0

    var out = PixelContainer(width: outW, height: outH)
    for oy: UInt32 in 0..<outH {
        for ox: UInt32 in 0..<outW {
            // Offset from output centre.
            let dx = Float(ox) - cxOut
            let dy = Float(oy) - cyOut

            // Inverse-rotate to find source coordinate.
            // (Using CCW rotation angle θ, the inverse is CW by −θ, but
            //  written out it becomes: u = cx + cos·dx + sin·dy,
            //                          v = cy − sin·dx + cos·dy)
            let u = cxIn + cosA * dx + sinA * dy
            let v = cyIn - sinA * dx + cosA * dy

            let px = sample(src, u: u, v: v, mode: mode, oob: .zero)
            setPixel(&out, x: ox, y: oy, r: px.0, g: px.1, b: px.2, a: px.3)
        }
    }
    return out
}

// ── Affine Warp ────────────────────────────────────────────────────────────
//
// An affine transform maps output pixel (x', y') to source coordinate (u, v)
// via a 2×3 matrix M:
//
//   u = M[0][0]*x' + M[0][1]*y' + M[0][2]
//   v = M[1][0]*x' + M[1][1]*y' + M[1][2]
//
// The 2×3 form is the standard for 2-D affine maps: it encodes a 2×2 linear
// part (rotation, scale, shear) plus a 2×1 translation column.
//
// Common matrices:
//   Identity:  [[1,0,0],[0,1,0]]
//   Scale 2×:  [[0.5,0,0],[0,0.5,0]]   (map output → half source)
//   Translate: [[1,0,tx],[0,1,ty]]

/// Apply a 2×3 affine matrix to warp the source into an (outW × outH) canvas.
///
/// Matrix layout (row-major, 2 rows × 3 columns):
///
/// ```
/// [ m00  m01  m02 ]     u = m00*x' + m01*y' + m02
/// [ m10  m11  m12 ]     v = m10*x' + m11*y' + m12
/// ```
///
/// - Parameters:
///   - src:    Source image.
///   - matrix: 2×3 Float array (outer index = row, inner = column).
///   - outW:   Output width.
///   - outH:   Output height.
///   - mode:   Interpolation.  Default `.bilinear`.
///   - oob:    Out-of-bounds handling.  Default `.replicate`.
public func affine(
    _ src: PixelContainer,
    matrix: [[Float]],
    width outW: UInt32,
    height outH: UInt32,
    mode: Interpolation = .bilinear,
    oob: OutOfBounds = .replicate
) -> PixelContainer {
    var out = PixelContainer(width: outW, height: outH)
    let m = matrix
    for oy: UInt32 in 0..<outH {
        for ox: UInt32 in 0..<outW {
            let xf = Float(ox)
            let yf = Float(oy)
            let u = m[0][0] * xf + m[0][1] * yf + m[0][2]
            let v = m[1][0] * xf + m[1][1] * yf + m[1][2]
            let px = sample(src, u: u, v: v, mode: mode, oob: oob)
            setPixel(&out, x: ox, y: oy, r: px.0, g: px.1, b: px.2, a: px.3)
        }
    }
    return out
}

// ── Perspective Warp ──────────────────────────────────────────────────────
//
// A perspective (projective) transform is the most general linear map between
// planes.  It is described by a 3×3 homogeneous matrix H.
//
// Given output pixel (x', y'), the source coordinate is:
//
//   w  = H[2][0]*x' + H[2][1]*y' + H[2][2]
//   u  = (H[0][0]*x' + H[0][1]*y' + H[0][2]) / w
//   v  = (H[1][0]*x' + H[1][1]*y' + H[1][2]) / w
//
// When w = 1 for all (x', y'), this reduces to an affine transform (the
// 3×3 bottom row is [0, 0, 1]).
//
// The division by w is the perspective division; it is what produces the
// "vanishing point" effect where parallel lines appear to converge.

/// Apply a 3×3 homogeneous perspective matrix to warp the source.
///
/// Matrix layout (row-major, 3×3):
///
/// ```
/// [ h00  h01  h02 ]     w = h20*x' + h21*y' + h22
/// [ h10  h11  h12 ]     u = (h00*x' + h01*y' + h02) / w
/// [ h20  h21  h22 ]     v = (h10*x' + h11*y' + h12) / w
/// ```
///
/// - Parameters:
///   - src:    Source image.
///   - h:      3×3 Float array.
///   - outW:   Output width.
///   - outH:   Output height.
///   - mode:   Interpolation.  Default `.bilinear`.
///   - oob:    Out-of-bounds handling.  Default `.replicate`.
public func perspectiveWarp(
    _ src: PixelContainer,
    matrix h: [[Float]],
    width outW: UInt32,
    height outH: UInt32,
    mode: Interpolation = .bilinear,
    oob: OutOfBounds = .replicate
) -> PixelContainer {
    var out = PixelContainer(width: outW, height: outH)
    for oy: UInt32 in 0..<outH {
        for ox: UInt32 in 0..<outW {
            let xf = Float(ox)
            let yf = Float(oy)
            let w = h[2][0] * xf + h[2][1] * yf + h[2][2]
            // Guard against degenerate (w ≈ 0) projective points.
            guard abs(w) > 1e-9 else { continue }
            let u = (h[0][0] * xf + h[0][1] * yf + h[0][2]) / w
            let v = (h[1][0] * xf + h[1][1] * yf + h[1][2]) / w
            let px = sample(src, u: u, v: v, mode: mode, oob: oob)
            setPixel(&out, x: ox, y: oy, r: px.0, g: px.1, b: px.2, a: px.3)
        }
    }
    return out
}
