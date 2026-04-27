/// IMG04 — Geometric transforms on PixelContainer.
///
/// A geometric transform maps each output pixel (x', y') back to a source
/// coordinate (u, v) and samples the source image.  The reverse-mapping
/// approach avoids holes: for every output pixel there is exactly one lookup
/// into the source, even when the transform magnifies by a large factor.
///
/// ## Coordinate model
///
/// We use the pixel-centre model throughout.  The centre of pixel (x, y)
/// lies at continuous coordinate (x + 0.5, y + 0.5).  This keeps sampling
/// symmetric under scale and avoids the off-by-half-pixel error that arises
/// when treating (0, 0) as the top-left corner of the pixel grid rather than
/// the centre of pixel (0, 0).
///
/// ## Colour-correct interpolation
///
/// Bilinear and bicubic filters decode sRGB bytes to linear light before
/// blending, then re-encode.  Blending in sRGB space is incorrect — see
/// IMG00 §2 for the full derivation.  Nearest-neighbour copies bytes directly
/// without conversion (no blending occurs).
///
/// ## Lossless transforms
///
/// Flips, 90°/180° rotations, crop, and pad rearrange or copy bytes without
/// any floating-point arithmetic.  They are exact and lossless.
module CodingAdventures.ImageGeometricTransforms

open System
open CodingAdventures.PixelContainer

// ── Public type definitions ───────────────────────────────────────────────────

/// Interpolation filter used when sampling a source image at a non-integer
/// coordinate.
///
/// - Nearest  : Copy the nearest pixel.  Fast; aliased on downscale.
/// - Bilinear : 2×2 weighted average in linear light.  Smooth; slightly blurry.
/// - Bicubic  : 4×4 Catmull-Rom spline in linear light.  Sharper than bilinear.
type Interpolation = Nearest | Bilinear | Bicubic

/// How the output canvas is sized when rotating by an arbitrary angle.
///
/// - Fit  : Expand the canvas so the entire rotated source fits inside.
/// - Crop : Keep the original canvas size; corners may be clipped.
type RotateBounds = Fit | Crop

/// What to return when a reverse-mapped coordinate falls outside [0, W) × [0, H).
///
/// - Zero      : Return transparent black (0, 0, 0, 0).
/// - Replicate : Clamp to the nearest edge pixel.
/// - Reflect   : Mirror the image at each edge (period = 2 * dimension).
/// - Wrap      : Tile the image (modular arithmetic).
type OutOfBounds = Zero | Replicate | Reflect | Wrap

/// A four-channel pixel value as a tuple of bytes (R, G, B, A).
/// Used as a colour parameter for operations such as pad.
type Rgba8 = byte * byte * byte * byte

// ── sRGB / linear-light LUT ───────────────────────────────────────────────────
//
// Building this table once at module startup costs 256 multiplications and
// 256 power evaluations instead of paying that cost per-pixel.
//
// Decode formula (IEC 61966-2-1):
//   c = byte / 255
//   c ≤ 0.04045  →  c / 12.92
//   else         →  ((c + 0.055) / 1.055) ^ 2.4

/// 256-entry sRGB-to-linear decode table.
/// Index is the sRGB byte value (0–255); value is the linear-light scalar in [0,1].
let private srgbToLinear: float[] =
    Array.init 256 (fun i ->
        let c = float i / 255.0
        if c <= 0.04045 then c / 12.92
        else ((c + 0.055) / 1.055) ** 2.4)

/// Decode one sRGB byte to a linear-light scalar in [0, 1].
let private decode (b: byte) : float = srgbToLinear[int b]

/// Encode a linear-light scalar in [0, 1] back to an sRGB byte.
/// Values outside [0, 1] are clamped before encoding.
///
/// Encode formula:
///   c ≤ 0.0031308  →  c * 12.92
///   else           →  1.055 * c ^ (1/2.4) − 0.055
///   clamp to [0, 1], multiply by 255, round to nearest integer.
let private encode (v: float) : byte =
    let c = if v <= 0.0031308 then 12.92 * v else 1.055 * v ** (1.0 / 2.4) - 0.055
    byte (min 255.0 (max 0.0 (Math.Round(min 1.0 (max 0.0 c) * 255.0))))

// ── Out-of-bounds coordinate resolution ───────────────────────────────────────
//
// Given an integer coordinate that may lie outside [0, max), map it to a valid
// index (or return None for the Zero policy, which produces transparent black).

/// Resolve a possibly-out-of-range coordinate x in [0, max) according to the
/// given OutOfBounds policy.  Returns Some(resolved) or None (for Zero policy).
let private resolveCoord (x: int) (max: int) (oob: OutOfBounds) : int option =
    match oob with
    | Zero ->
        if x >= 0 && x < max then Some x else None
    | Replicate ->
        Some (min (max - 1) (Math.Max(0, x)))
    | Reflect ->
        // Mirror at both edges.  The repeat period is 2 * max.
        //   0 1 2 3 4  3 2 1  0 1 2 3 4 ...   (max = 5)
        let period = 2 * max
        let r = ((x % period) + period) % period
        Some (if r >= max then period - 1 - r else r)
    | Wrap ->
        Some (((x % max) + max) % max)

// ── Catmull-Rom weight kernel ──────────────────────────────────────────────────
//
// Catmull-Rom is a cubic spline interpolant with tension parameter α = 0.5.
// It passes through the sample points (unlike B-splines) and produces sharper
// results than bilinear interpolation.
//
// Weight formula for distance d from the sample point:
//   |d| < 1  →  1.5|d|³ − 2.5|d|² + 1
//   |d| < 2  →  −0.5|d|³ + 2.5|d|² − 4|d| + 2
//   else     →  0

/// Catmull-Rom kernel weight for distance d from the reconstruction point.
let private catmullRom (d: float) : float =
    let d = abs d
    if d < 1.0 then 1.5 * d ** 3.0 - 2.5 * d ** 2.0 + 1.0
    elif d < 2.0 then -0.5 * d ** 3.0 + 2.5 * d ** 2.0 - 4.0 * d + 2.0
    else 0.0

// ── Sampling functions ─────────────────────────────────────────────────────────
//
// Each sampler takes a PixelContainer and a continuous source coordinate (u, v)
// and returns Rgba8.  Bilinear and bicubic work in linear light.

/// Read one pixel from src, applying the OOB policy.  Returns (r, g, b, a)
/// as floats in linear light (for use in blending), or None if OOB=Zero and
/// the coordinate is out of range.
let private readLinear (src: PixelContainer) (xi: int) (yi: int) (oob: OutOfBounds) : (float * float * float * float) option =
    match resolveCoord xi src.Width oob, resolveCoord yi src.Height oob with
    | Some rx, Some ry ->
        let p = src.GetPixel(rx, ry)
        Some (decode p.R, decode p.G, decode p.B, float p.A / 255.0)
    | _ -> None

/// Nearest-neighbour sampler.  Returns the byte values of the nearest source
/// pixel without any colour-space conversion.
let private sampleNearest (src: PixelContainer) (u: float) (v: float) (oob: OutOfBounds) : Rgba8 =
    let xi = int (Math.Floor u)
    let yi = int (Math.Floor v)
    match resolveCoord xi src.Width oob, resolveCoord yi src.Height oob with
    | Some rx, Some ry ->
        let p = src.GetPixel(rx, ry)
        (p.R, p.G, p.B, p.A)
    | _ -> (0uy, 0uy, 0uy, 0uy)

/// Bilinear sampler.  Blends a 2×2 neighbourhood in linear light.
///
/// The fractional part of (u, v) in pixel-centre coordinates determines the
/// weight given to each of the four surrounding sample points.
let private sampleBilinear (src: PixelContainer) (u: float) (v: float) (oob: OutOfBounds) : Rgba8 =
    // Convert from pixel-centre coordinates to array-index coordinates.
    let u0 = u - 0.5
    let v0 = v - 0.5
    let x0 = int (Math.Floor u0)
    let y0 = int (Math.Floor v0)
    let fx = u0 - float x0   // horizontal fractional weight
    let fy = v0 - float y0   // vertical fractional weight

    // Helper: safely read one of the four samples, returning zeros on OOB=Zero.
    let rd xi yi =
        match readLinear src xi yi oob with
        | Some v -> v
        | None   -> (0.0, 0.0, 0.0, 0.0)

    let r00, g00, b00, a00 = rd  x0      y0
    let r10, g10, b10, a10 = rd (x0 + 1) y0
    let r01, g01, b01, a01 = rd  x0     (y0 + 1)
    let r11, g11, b11, a11 = rd (x0 + 1)(y0 + 1)

    // Bilinear interpolation: lerp in x, then lerp the two results in y.
    let lerp a b t = a + t * (b - a)
    let blendR = lerp (lerp r00 r10 fx) (lerp r01 r11 fx) fy
    let blendG = lerp (lerp g00 g10 fx) (lerp g01 g11 fx) fy
    let blendB = lerp (lerp b00 b10 fx) (lerp b01 b11 fx) fy
    let blendA = lerp (lerp a00 a10 fx) (lerp a01 a11 fx) fy

    (encode blendR, encode blendG, encode blendB, byte (Math.Round(Math.Min(1.0, Math.Max(0.0, blendA)) * 255.0)))

/// Bicubic sampler using the Catmull-Rom kernel over a 4×4 neighbourhood.
///
/// For each row of four samples we compute a 1D Catmull-Rom interpolation,
/// then interpolate those four row values vertically with the same kernel.
/// All arithmetic is done in linear light.
let private sampleBicubic (src: PixelContainer) (u: float) (v: float) (oob: OutOfBounds) : Rgba8 =
    let u0 = u - 0.5
    let v0 = v - 0.5
    let x0 = int (Math.Floor u0)
    let y0 = int (Math.Floor v0)
    let fx = u0 - float x0
    let fy = v0 - float y0

    // Weights for the 4 columns (offsets -1, 0, 1, 2 from x0).
    let wx = [| catmullRom (fx + 1.0); catmullRom fx; catmullRom (1.0 - fx); catmullRom (2.0 - fx) |]
    let wy = [| catmullRom (fy + 1.0); catmullRom fy; catmullRom (1.0 - fy); catmullRom (2.0 - fy) |]

    let mutable sumR = 0.0
    let mutable sumG = 0.0
    let mutable sumB = 0.0
    let mutable sumA = 0.0

    for j in 0..3 do
        for i in 0..3 do
            let w = wx[i] * wy[j]
            match readLinear src (x0 - 1 + i) (y0 - 1 + j) oob with
            | Some (r, g, b, a) ->
                sumR <- sumR + w * r
                sumG <- sumG + w * g
                sumB <- sumB + w * b
                sumA <- sumA + w * a
            | None -> ()

    (encode sumR, encode sumG, encode sumB, byte (Math.Round(Math.Min(1.0, Math.Max(0.0, sumA)) * 255.0)))

/// Dispatch to the appropriate sampler based on the Interpolation mode.
let private doSample (src: PixelContainer) (u: float) (v: float) (mode: Interpolation) (oob: OutOfBounds) : Rgba8 =
    match mode with
    | Nearest  -> sampleNearest  src u v oob
    | Bilinear -> sampleBilinear src u v oob
    | Bicubic  -> sampleBicubic  src u v oob

// ── Lossless geometric transforms ─────────────────────────────────────────────
//
// These functions rearrange pixels without any interpolation or colour-space
// conversion.  The output is mathematically identical to the input (modulo
// the rearrangement).

/// Flip the image horizontally (mirror left-to-right).
///
/// For each output pixel (x', y'), the source pixel is (W−1−x', y').
///
/// Applying flipHorizontal twice returns the original image exactly.
let flipHorizontal (src: PixelContainer) : PixelContainer =
    let w, h = src.Width, src.Height
    let out = PixelContainer(w, h)
    for y in 0 .. h - 1 do
        for x in 0 .. w - 1 do
            let p = src.GetPixel(w - 1 - x, y)
            out.SetPixel(x, y, p.R, p.G, p.B, p.A)
    out

/// Flip the image vertically (mirror top-to-bottom).
///
/// For each output pixel (x', y'), the source pixel is (x', H−1−y').
///
/// Applying flipVertical twice returns the original image exactly.
let flipVertical (src: PixelContainer) : PixelContainer =
    let w, h = src.Width, src.Height
    let out = PixelContainer(w, h)
    for y in 0 .. h - 1 do
        for x in 0 .. w - 1 do
            let p = src.GetPixel(x, h - 1 - y)
            out.SetPixel(x, y, p.R, p.G, p.B, p.A)
    out

/// Rotate 90° clockwise.
///
/// Output dimensions: W' = H, H' = W.
/// Mapping: O[x', y'] = I[y', W−1−x']
///   where W is the source width and the output column x' ∈ [0, H).
///
/// A 4× application (0°, 90°, 180°, 270°, 360°) returns the original image.
let rotate90CW (src: PixelContainer) : PixelContainer =
    let w, h = src.Width, src.Height
    // Output width = source height; output height = source width.
    let out = PixelContainer(h, w)
    for y' in 0 .. w - 1 do
        for x' in 0 .. h - 1 do
            // Source pixel: column = y', row = W−1−x'
            let p = src.GetPixel(y', w - 1 - x')
            out.SetPixel(x', y', p.R, p.G, p.B, p.A)
    out

/// Rotate 90° counter-clockwise.
///
/// Output dimensions: W' = H, H' = W.
/// Mapping: O[x', y'] = I[H−1−y', x']
///   where H is the source height and x' ∈ [0, H).
let rotate90CCW (src: PixelContainer) : PixelContainer =
    let w, h = src.Width, src.Height
    let out = PixelContainer(h, w)
    for y' in 0 .. w - 1 do
        for x' in 0 .. h - 1 do
            let p = src.GetPixel(h - 1 - y', x')
            out.SetPixel(x', y', p.R, p.G, p.B, p.A)
    out

/// Rotate 180°.
///
/// Equivalent to flipHorizontal ∘ flipVertical, but in one pass.
/// Applying rotate180 twice returns the original image exactly.
let rotate180 (src: PixelContainer) : PixelContainer =
    let w, h = src.Width, src.Height
    let out = PixelContainer(w, h)
    for y in 0 .. h - 1 do
        for x in 0 .. w - 1 do
            let p = src.GetPixel(w - 1 - x, h - 1 - y)
            out.SetPixel(x, y, p.R, p.G, p.B, p.A)
    out

/// Crop a rectangular region from src.
///
/// Parameters:
///   x0, y0 — top-left corner of the crop rectangle (inclusive).
///   w, h   — output dimensions.
///
/// Pixels outside the source bounds read as transparent black (0,0,0,0),
/// which is the behaviour of PixelContainer.GetPixel for out-of-range coords.
let crop (src: PixelContainer) (x0: int) (y0: int) (w: int) (h: int) : PixelContainer =
    let out = PixelContainer(w, h)
    for y in 0 .. h - 1 do
        for x in 0 .. w - 1 do
            let p = src.GetPixel(x0 + x, y0 + y)
            out.SetPixel(x, y, p.R, p.G, p.B, p.A)
    out

/// Add a border of fill pixels around src.
///
/// Parameters:
///   top, right, bottom, left — border widths in pixels (must be ≥ 0).
///   fill — RGBA8 colour for the border region.
///
/// Output dimensions: W' = left + W + right, H' = top + H + bottom.
let pad (src: PixelContainer) (top: int) (right: int) (bottom: int) (left: int) (fill: Rgba8) : PixelContainer =
    let outW = left + src.Width + right
    let outH = top + src.Height + bottom
    let out = PixelContainer(outW, outH)
    let fr, fg, fb, fa = fill
    // Fill the entire canvas with the border colour first, then copy the source.
    out.Fill(fr, fg, fb, fa)
    for y in 0 .. src.Height - 1 do
        for x in 0 .. src.Width - 1 do
            let p = src.GetPixel(x, y)
            out.SetPixel(left + x, top + y, p.R, p.G, p.B, p.A)
    out

// ── Continuous geometric transforms ───────────────────────────────────────────
//
// These functions use reverse mapping: for each output pixel (x', y') we
// compute the corresponding source coordinate (u, v) and call a sampler.

/// Scale src to an output of exactly outW × outH pixels.
///
/// The pixel-centre model is used:  the centre of output pixel (x', y')
/// maps back to source coordinate:
///   u = (x' + 0.5) * W / outW
///   v = (y' + 0.5) * H / outH
///
/// Out-of-bounds lookups use Replicate (edge-extend) for clean borders.
let scale (src: PixelContainer) (outW: int) (outH: int) (mode: Interpolation) : PixelContainer =
    let out = PixelContainer(outW, outH)
    let sw = float src.Width
    let sh = float src.Height
    for y' in 0 .. outH - 1 do
        for x' in 0 .. outW - 1 do
            let u = (float x' + 0.5) * sw / float outW
            let v = (float y' + 0.5) * sh / float outH
            let r, g, b, a = doSample src u v mode Replicate
            out.SetPixel(x', y', r, g, b, a)
    out

/// Rotate src by radians around its centre.
///
/// The canvas is determined by bounds:
///   Fit  — expand so the full rotated image fits; background is transparent black.
///   Crop — keep the original W × H canvas; corners are clipped.
///
/// Out-of-bounds lookups use Zero (transparent black background).
///
/// Reverse mapping for output pixel (x', y'):
///   dx = x' − cx';  dy = y' − cy'          (offset from output centre)
///   u  = cos(θ)·dx − sin(θ)·dy + cx        (rotate back, offset from source centre)
///   v  = sin(θ)·dx + cos(θ)·dy + cy
///   where cx, cy = source centre; cx', cy' = output centre.
let rotate (src: PixelContainer) (radians: float) (mode: Interpolation) (bounds: RotateBounds) : PixelContainer =
    let sw, sh = float src.Width, float src.Height
    let cosA = Math.Cos radians
    let sinA = Math.Sin radians

    // Determine output canvas size.
    let outW, outH =
        match bounds with
        | Crop -> src.Width, src.Height
        | Fit ->
            // The rotated axis-aligned bounding box of the source rectangle.
            let w' = abs (cosA * sw) + abs (sinA * sh)
            let h' = abs (sinA * sw) + abs (cosA * sh)
            int (Math.Ceiling w'), int (Math.Ceiling h')

    let cx  = sw / 2.0
    let cy  = sh / 2.0
    let cx' = float outW / 2.0
    let cy' = float outH / 2.0

    let out = PixelContainer(outW, outH)
    for y' in 0 .. outH - 1 do
        for x' in 0 .. outW - 1 do
            let dx = float x' + 0.5 - cx'
            let dy = float y' + 0.5 - cy'
            // Rotate (dx, dy) back by −radians to get source coordinate.
            let u = cosA * dx + sinA * dy + cx
            let v = -sinA * dx + cosA * dy + cy
            let r, g, b, a = doSample src u v mode Zero
            out.SetPixel(x', y', r, g, b, a)
    out

/// Apply a 2×3 affine transform to src.
///
/// matrix is a 2×3 array[,] encoding the forward mapping from source
/// coordinate (u, v) to output pixel (x', y'):
///
///   u = matrix[0,0]·x' + matrix[0,1]·y' + matrix[0,2]
///   v = matrix[1,0]·x' + matrix[1,1]·y' + matrix[1,2]
///
/// This is the inverse (source-lookup) convention: the matrix rows give the
/// source coordinate as a linear function of the output coordinate.
/// Output canvas is outW × outH.
let affine (src: PixelContainer) (matrix: float[,]) (outW: int) (outH: int) (mode: Interpolation) (oob: OutOfBounds) : PixelContainer =
    let out = PixelContainer(outW, outH)
    for y' in 0 .. outH - 1 do
        for x' in 0 .. outW - 1 do
            let xf = float x' + 0.5
            let yf = float y' + 0.5
            let u = matrix[0, 0] * xf + matrix[0, 1] * yf + matrix[0, 2]
            let v = matrix[1, 0] * xf + matrix[1, 1] * yf + matrix[1, 2]
            let r, g, b, a = doSample src u v mode oob
            out.SetPixel(x', y', r, g, b, a)
    out

/// Apply a perspective (homographic) warp to src.
///
/// h is a 3×3 array[,] representing a homogeneous transformation.  For each
/// output pixel (x', y') the source coordinate is computed as:
///
///   [u'; v'; w'] = H · [x' + 0.5; y' + 0.5; 1]
///   u = u' / w';  v = v' / w'
///
/// If w' is zero or near-zero the output pixel is treated as out-of-bounds.
/// Output canvas is outW × outH.
let perspectiveWarp (src: PixelContainer) (h: float[,]) (outW: int) (outH: int) (mode: Interpolation) (oob: OutOfBounds) : PixelContainer =
    let out = PixelContainer(outW, outH)
    for y' in 0 .. outH - 1 do
        for x' in 0 .. outW - 1 do
            let xf = float x' + 0.5
            let yf = float y' + 0.5
            let u' = h[0, 0] * xf + h[0, 1] * yf + h[0, 2]
            let v' = h[1, 0] * xf + h[1, 1] * yf + h[1, 2]
            let w' = h[2, 0] * xf + h[2, 1] * yf + h[2, 2]
            if abs w' < 1e-10 then
                out.SetPixel(x', y', 0uy, 0uy, 0uy, 0uy)
            else
                let u = u' / w'
                let v = v' / w'
                let r, g, b, a = doSample src u v mode oob
                out.SetPixel(x', y', r, g, b, a)
    out
