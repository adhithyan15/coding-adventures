/// IMG03 — Per-pixel point operations on PixelContainer.
///
/// A point operation transforms each pixel independently using only that
/// pixel's own value.  No neighbouring pixels, no frequency domain, no geometry.
///
/// ## Two domains
///
/// u8-domain operations (Invert, Threshold, Posterize, …) work directly on
/// the sRGB bytes.  Correct without colour-space conversion because they are
/// monotone remappings that never mix or average channel values.
///
/// Linear-light operations (Contrast, Gamma, Exposure, Greyscale, …) decode
/// each byte to a linear-light float, perform the arithmetic, then re-encode.
/// Averaging in sRGB is incorrect — see IMG00 §2.
///
/// ## sRGB ↔ linear round-trip
///
///   Decode (byte → float):
///     c = float byte / 255.0
///     c ≤ 0.04045  →  c / 12.92
///     else         →  ((c + 0.055) / 1.055) ** 2.4
///
///   Encode (float → byte):
///     c ≤ 0.0031308  →  c * 12.92
///     else           →  1.055 * c ** (1/2.4) − 0.055
///     clamp to [0,1], multiply by 255, round
module CodingAdventures.ImagePointOps

open System
open CodingAdventures.PixelContainer

// ── sRGB / linear LUT ─────────────────────────────────────────────────────

/// 256-entry decode LUT: index is sRGB byte, value is linear float.
/// Built once at module startup.
let private srgbToLinear: float[] =
    Array.init 256 (fun i ->
        let c = float i / 255.0
        if c <= 0.04045 then c / 12.92
        else ((c + 0.055) / 1.055) ** 2.4)

let private decode (b: byte) = srgbToLinear[int b]

let private encode (linear: float) =
    let c =
        if linear <= 0.0031308 then linear * 12.92
        else 1.055 * linear ** (1.0 / 2.4) - 0.055
    let clamped = Math.Min(1.0, Math.Max(0.0, c))
    byte (Math.Round(clamped * 255.0))

// ── Iteration helper ───────────────────────────────────────────────────────

let private mapPixels (src: PixelContainer) (fn: byte -> byte -> byte -> byte -> byte * byte * byte * byte) =
    let out = PixelContainer(src.Width, src.Height)
    for y in 0 .. src.Height - 1 do
        for x in 0 .. src.Width - 1 do
            let px = src.GetPixel(x, y)
            let nr, ng, nb, na = fn px.R px.G px.B px.A
            out.SetPixel(x, y, nr, ng, nb, na)
    out

// ── u8-domain operations ───────────────────────────────────────────────────

/// Invert: flip each RGB channel (255 − v).  Alpha is preserved.
/// Applying Invert twice returns the original image exactly.
let invert (src: PixelContainer) =
    mapPixels src (fun r g b a -> (255uy - r, 255uy - g, 255uy - b, a))

/// Threshold: (r+g+b)/3 >= value → white, else black.  Alpha preserved.
let threshold (src: PixelContainer) (value: byte) =
    mapPixels src (fun r g b a ->
        let luma = (int r + int g + int b) / 3
        let v = if luma >= int value then 255uy else 0uy
        (v, v, v, a))

/// Threshold on Rec. 709 luma: Y = 0.2126 R + 0.7152 G + 0.0722 B.
let thresholdLuminance (src: PixelContainer) (value: byte) =
    mapPixels src (fun r g b a ->
        let luma = 0.2126 * float r + 0.7152 * float g + 0.0722 * float b
        let v = if luma >= float value then 255uy else 0uy
        (v, v, v, a))

/// Posterize: reduce each channel to `levels` equally-spaced steps.
let posterize (src: PixelContainer) (levels: int) =
    let step = 255.0 / float (levels - 1)
    let q (v: byte) = byte (Math.Round(Math.Round(float v / step) * step))
    mapPixels src (fun r g b a -> (q r, q g, q b, a))

/// Swap R and B channels (RGB ↔ BGR).
let swapRGBBGR (src: PixelContainer) =
    mapPixels src (fun r g b a -> (b, g, r, a))

/// Channel discriminated union for extractChannel.
type Channel = R | G | B | A

/// Extract one channel; zero the rest.  Alpha is always preserved.
let extractChannel (src: PixelContainer) (ch: Channel) =
    mapPixels src (fun r g b a ->
        match ch with
        | R -> (r, 0uy, 0uy, a)
        | G -> (0uy, g, 0uy, a)
        | B -> (0uy, 0uy, b, a)
        | A -> (r, g, b, a))

/// Additive brightness: add signed offset, clamped to [0, 255].
let brightness (src: PixelContainer) (offset: int) =
    let clamp (v: byte) = byte (Math.Min(255, Math.Max(0, int v + offset)))
    mapPixels src (fun r g b a -> (clamp r, clamp g, clamp b, a))

// ── Linear-light operations ────────────────────────────────────────────────

/// Contrast: scale around linear mid-grey (0.5).
/// factor = 1 → identity; < 1 → less contrast; > 1 → more.
let contrast (src: PixelContainer) (factor: float) =
    mapPixels src (fun r g b a ->
        (encode (0.5 + factor * (decode r - 0.5)),
         encode (0.5 + factor * (decode g - 0.5)),
         encode (0.5 + factor * (decode b - 0.5)),
         a))

/// Gamma: power-law g in linear light.  g < 1 → brighter; g > 1 → darker.
let gamma (src: PixelContainer) (g: float) =
    mapPixels src (fun r gv b a ->
        (encode (decode r ** g),
         encode (decode gv ** g),
         encode (decode b ** g),
         a))

/// Exposure: multiply linear by 2^stops.
let exposure (src: PixelContainer) (stops: float) =
    let factor = 2.0 ** stops
    mapPixels src (fun r g b a ->
        (encode (decode r * factor),
         encode (decode g * factor),
         encode (decode b * factor),
         a))

/// Luminance weighting method for greyscale.
type GreyscaleMethod = Rec709 | BT601 | Average

/// Greyscale: convert to luminance in linear light.
let greyscale (src: PixelContainer) (method': GreyscaleMethod) =
    let wr, wg, wb =
        match method' with
        | Rec709  -> (0.2126, 0.7152, 0.0722)
        | BT601   -> (0.2989, 0.5870, 0.1140)
        | Average -> (1.0/3.0, 1.0/3.0, 1.0/3.0)
    mapPixels src (fun r g b a ->
        let y = encode (wr * decode r + wg * decode g + wb * decode b)
        (y, y, y, a))

/// Sepia: classic warm sepia tone matrix in linear light.
let sepia (src: PixelContainer) =
    mapPixels src (fun r g b a ->
        let lr, lg, lb = decode r, decode g, decode b
        (encode (0.393*lr + 0.769*lg + 0.189*lb),
         encode (0.349*lr + 0.686*lg + 0.168*lb),
         encode (0.272*lr + 0.534*lg + 0.131*lb),
         a))

/// Colour matrix: multiply linear [R, G, B] by a 3×3 matrix.
/// matrix.[row].[col]  where row 0 = output R, etc.
let colourMatrix (src: PixelContainer) (matrix: float[,]) =
    mapPixels src (fun r g b a ->
        let lr, lg, lb = decode r, decode g, decode b
        (encode (matrix.[0,0]*lr + matrix.[0,1]*lg + matrix.[0,2]*lb),
         encode (matrix.[1,0]*lr + matrix.[1,1]*lg + matrix.[1,2]*lb),
         encode (matrix.[2,0]*lr + matrix.[2,1]*lg + matrix.[2,2]*lb),
         a))

/// Saturate: 0 → greyscale; 1 → identity; > 1 → vivid.
let saturate (src: PixelContainer) (factor: float) =
    mapPixels src (fun r g b a ->
        let lr, lg, lb = decode r, decode g, decode b
        let grey = 0.2126*lr + 0.7152*lg + 0.0722*lb
        (encode (grey + factor*(lr - grey)),
         encode (grey + factor*(lg - grey)),
         encode (grey + factor*(lb - grey)),
         a))

// ── HSV helpers ────────────────────────────────────────────────────────────

let private rgbToHSV r g b =
    let mx = max r (max g b)
    let mn = min r (min g b)
    let delta = mx - mn
    let v = mx
    let s = if mx = 0.0 then 0.0 else delta / mx
    let h =
        if delta = 0.0 then 0.0
        else
            let h0 =
                if mx = r then ((g - b) / delta) % 6.0
                elif mx = g then (b - r) / delta + 2.0
                else (r - g) / delta + 4.0
            (h0 * 60.0 + 360.0) % 360.0
    (h, s, v)

let private hsvToRGB h s v =
    let c = v * s
    let x = c * (1.0 - abs ((h / 60.0) % 2.0 - 1.0))
    let m = v - c
    let r, g, b =
        let sector = int (h / 60.0)
        match sector with
        | 0 -> (c, x, 0.0)
        | 1 -> (x, c, 0.0)
        | 2 -> (0.0, c, x)
        | 3 -> (0.0, x, c)
        | 4 -> (x, 0.0, c)
        | _ -> (c, 0.0, x)
    (r + m, g + m, b + m)

/// Hue rotate: rotate hue by degrees.  360° is identity.
let hueRotate (src: PixelContainer) (degrees: float) =
    mapPixels src (fun r g b a ->
        let h, s, v = rgbToHSV (decode r) (decode g) (decode b)
        let nr, ng, nb = hsvToRGB ((h + degrees + 360.0) % 360.0) s v
        (encode nr, encode ng, encode nb, a))

// ── Colorspace utilities ───────────────────────────────────────────────────

/// Convert sRGB → linear (each byte becomes round(linear * 255)).
let srgbToLinearImage (src: PixelContainer) =
    mapPixels src (fun r g b a ->
        (byte (Math.Round(decode r * 255.0)),
         byte (Math.Round(decode g * 255.0)),
         byte (Math.Round(decode b * 255.0)),
         a))

/// Convert linear → sRGB (inverse of srgbToLinearImage).
let linearToSRGBImage (src: PixelContainer) =
    mapPixels src (fun r g b a ->
        (encode (float r / 255.0),
         encode (float g / 255.0),
         encode (float b / 255.0),
         a))

// ── 1D LUT operations ──────────────────────────────────────────────────────

/// Apply three 256-entry u8→u8 LUTs (one per channel).  Alpha preserved.
let applyLUT1DU8 (src: PixelContainer) (lutR: byte[]) (lutG: byte[]) (lutB: byte[]) =
    mapPixels src (fun r g b a -> (lutR[int r], lutG[int g], lutB[int b], a))

/// Build a 256-entry LUT from a linear-light mapping function f: [0,1]→[0,1].
let buildLUT1DU8 (fn: float -> float) =
    Array.init 256 (fun i -> encode (fn (decode (byte i))))

/// Build a gamma LUT (equivalent to buildLUT1DU8 (fun v -> v ** g)).
let buildGammaLUT (g: float) =
    buildLUT1DU8 (fun v -> v ** g)
