// ImagePointOps.swift
// Part of coding-adventures — an educational computing stack.
//
// ============================================================================
// MARK: - IMG03: Per-Pixel Point Operations
// ============================================================================
//
// A point operation transforms each pixel independently using only that
// pixel's own value — no neighbouring pixels, no frequency-domain transform.
//
// ## Two domains
//
// u8-domain operations work directly on the UInt8 sRGB bytes.  They are
// correct without colour-space conversion because they are monotone
// remappings that never mix or average channel values.
//
// Linear-light operations decode each byte to a linear-light Float (the
// inverse sRGB transfer), perform the arithmetic, then re-encode.  Averaging
// in sRGB space is incorrect — blending 50% black and 50% white in sRGB gives
// a value that appears too dark (see IMG00 §2 for a worked example).
//
// ## sRGB ↔ Linear Round-Trip
//
//   Decode (UInt8 → Float):
//     c = Float(byte) / 255
//     c ≤ 0.04045  →  c / 12.92
//     else         →  pow((c + 0.055) / 1.055, 2.4)
//
//   Encode (Float → UInt8):
//     c ≤ 0.0031308  →  c * 12.92
//     else           →  1.055 * pow(c, 1/2.4) − 0.055
//     clamp to [0,1], multiply by 255, round
// ============================================================================

import Foundation
import PixelContainer

// ── sRGB / linear LUT ─────────────────────────────────────────────────────

/// 256-entry decode LUT: sRGB byte → linear Float.  Built once, reused everywhere.
private let srgbToLinear: [Float] = {
    var t = [Float](repeating: 0, count: 256)
    for i in 0..<256 {
        let c = Float(i) / 255.0
        t[i] = c <= 0.04045 ? c / 12.92 : pow((c + 0.055) / 1.055, 2.4)
    }
    return t
}()

private func decode(_ byte: UInt8) -> Float {
    srgbToLinear[Int(byte)]
}

private func encode(_ linear: Float) -> UInt8 {
    let c: Float = linear <= 0.0031308 ? linear * 12.92 : 1.055 * pow(linear, 1.0 / 2.4) - 0.055
    let v = Int((min(1, max(0, c)) * 255).rounded())
    return UInt8(min(255, max(0, v)))
}

// ── Iteration helper ───────────────────────────────────────────────────────

private func mapPixels(
    _ src: PixelContainer,
    transform: (UInt8, UInt8, UInt8, UInt8) -> (UInt8, UInt8, UInt8, UInt8)
) -> PixelContainer {
    var out = PixelContainer(width: src.width, height: src.height)
    for y: UInt32 in 0..<src.height {
        for x: UInt32 in 0..<src.width {
            let (r, g, b, a) = pixelAt(src, x: x, y: y)
            let (nr, ng, nb, na) = transform(r, g, b, a)
            setPixel(&out, x: x, y: y, r: nr, g: ng, b: nb, a: na)
        }
    }
    return out
}

// ── u8-domain operations ───────────────────────────────────────────────────

/// Invert: flip each RGB channel (255 − v).  Alpha is preserved.
///
/// Applying `invert` twice returns the original image exactly because
/// `255 − (255 − v) == v` for all integers in [0, 255].
public func invert(_ src: PixelContainer) -> PixelContainer {
    mapPixels(src) { r, g, b, a in (255 - r, 255 - g, 255 - b, a) }
}

/// Threshold: binarise on average luminance.  (r+g+b)/3 >= value → white.
public func threshold(_ src: PixelContainer, value: UInt8) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        let luma = (UInt32(r) + UInt32(g) + UInt32(b)) / 3
        let v: UInt8 = luma >= UInt32(value) ? 255 : 0
        return (v, v, v, a)
    }
}

/// Threshold on Rec. 709 luma: Y = 0.2126 R + 0.7152 G + 0.0722 B.
public func thresholdLuminance(_ src: PixelContainer, value: UInt8) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        let luma = 0.2126 * Float(r) + 0.7152 * Float(g) + 0.0722 * Float(b)
        let v: UInt8 = luma >= Float(value) ? 255 : 0
        return (v, v, v, a)
    }
}

/// Posterize: reduce each channel to `levels` equally-spaced steps.
public func posterize(_ src: PixelContainer, levels: Int) -> PixelContainer {
    let step = 255.0 / Float(levels - 1)
    let q: (UInt8) -> UInt8 = { v in UInt8(min(255, max(0, (Float(v) / step).rounded() * step))) }
    return mapPixels(src) { r, g, b, a in (q(r), q(g), q(b), a) }
}

/// Swap R and B channels (RGB ↔ BGR).
public func swapRGBBGR(_ src: PixelContainer) -> PixelContainer {
    mapPixels(src) { r, g, b, a in (b, g, r, a) }
}

/// Channel to extract.
public enum Channel: Int {
    case r = 0, g = 1, b = 2, a = 3
}

/// Extract one channel, zeroing the others.  Alpha is always preserved.
public func extractChannel(_ src: PixelContainer, channel: Channel) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        switch channel {
        case .r: return (r, 0, 0, a)
        case .g: return (0, g, 0, a)
        case .b: return (0, 0, b, a)
        case .a: return (r, g, b, a)
        }
    }
}

/// Additive brightness: add signed `offset` to each channel, clamped to [0, 255].
public func brightness(_ src: PixelContainer, offset: Int) -> PixelContainer {
    let clamp: (UInt8) -> UInt8 = { v in UInt8(exactly: min(255, max(0, Int(v) + offset))) ?? (offset > 0 ? 255 : 0) }
    return mapPixels(src) { r, g, b, a in (clamp(r), clamp(g), clamp(b), a) }
}

// ── Linear-light operations ────────────────────────────────────────────────

/// Contrast: scale around linear mid-grey 0.5.
///
/// `factor = 1` → identity;  `factor < 1` → less contrast;  `factor > 1` → more.
public func contrast(_ src: PixelContainer, factor: Float) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        (encode(0.5 + factor * (decode(r) - 0.5)),
         encode(0.5 + factor * (decode(g) - 0.5)),
         encode(0.5 + factor * (decode(b) - 0.5)),
         a)
    }
}

/// Gamma: apply power-law `g` in linear light.  `g < 1` → brighter; `g > 1` → darker.
public func gamma(_ src: PixelContainer, g: Float) -> PixelContainer {
    mapPixels(src) { r, gv, b, a in
        (encode(pow(decode(r), g)), encode(pow(decode(gv), g)), encode(pow(decode(b), g)), a)
    }
}

/// Exposure: multiply linear luminance by 2^stops.
public func exposure(_ src: PixelContainer, stops: Float) -> PixelContainer {
    let factor = pow(2.0 as Float, stops)
    return mapPixels(src) { r, g, b, a in
        (encode(decode(r) * factor), encode(decode(g) * factor), encode(decode(b) * factor), a)
    }
}

/// Luminance weighting scheme for greyscale conversion.
public enum GreyscaleMethod {
    case rec709   // 0.2126 R + 0.7152 G + 0.0722 B  (perceptually correct)
    case bt601    // 0.2989 R + 0.5870 G + 0.1140 B  (legacy SD-TV)
    case average  // (R + G + B) / 3  (equal weights, fast)
}

/// Greyscale: convert to luminance in linear light.
public func greyscale(_ src: PixelContainer, method: GreyscaleMethod = .rec709) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        let lr = decode(r), lg = decode(g), lb = decode(b)
        let y: Float
        switch method {
        case .rec709:  y = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb
        case .bt601:   y = 0.2989 * lr + 0.5870 * lg + 0.1140 * lb
        case .average: y = (lr + lg + lb) / 3
        }
        let out = encode(y)
        return (out, out, out, a)
    }
}

/// Sepia: classic warm sepia tone matrix in linear light.
public func sepia(_ src: PixelContainer) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        let lr = decode(r), lg = decode(g), lb = decode(b)
        return (encode(0.393*lr + 0.769*lg + 0.189*lb),
                encode(0.349*lr + 0.686*lg + 0.168*lb),
                encode(0.272*lr + 0.534*lg + 0.131*lb),
                a)
    }
}

/// Colour matrix: multiply linear [R, G, B] by a 3×3 matrix (row-major).
public func colourMatrix(_ src: PixelContainer, matrix: [[Float]]) -> PixelContainer {
    let m = matrix
    return mapPixels(src) { r, g, b, a in
        let lr = decode(r), lg = decode(g), lb = decode(b)
        return (encode(m[0][0]*lr + m[0][1]*lg + m[0][2]*lb),
                encode(m[1][0]*lr + m[1][1]*lg + m[1][2]*lb),
                encode(m[2][0]*lr + m[2][1]*lg + m[2][2]*lb),
                a)
    }
}

/// Saturate: 0 → greyscale; 1 → identity; >1 → vivid.
public func saturate(_ src: PixelContainer, factor: Float) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        let lr = decode(r), lg = decode(g), lb = decode(b)
        let grey = 0.2126*lr + 0.7152*lg + 0.0722*lb
        return (encode(grey + factor*(lr - grey)),
                encode(grey + factor*(lg - grey)),
                encode(grey + factor*(lb - grey)),
                a)
    }
}

// ── HSV helpers ────────────────────────────────────────────────────────────

private func rgbToHSV(_ r: Float, _ g: Float, _ b: Float) -> (Float, Float, Float) {
    let mx = max(r, max(g, b))
    let mn = min(r, min(g, b))
    let delta = mx - mn
    let v = mx
    let s: Float = mx == 0 ? 0 : delta / mx
    var h: Float = 0
    if delta != 0 {
        if mx == r { h = ((g - b) / delta).truncatingRemainder(dividingBy: 6) }
        else if mx == g { h = (b - r) / delta + 2 }
        else { h = (r - g) / delta + 4 }
        h = (h * 60 + 360).truncatingRemainder(dividingBy: 360)
    }
    return (h, s, v)
}

private func hsvToRGB(_ h: Float, _ s: Float, _ v: Float) -> (Float, Float, Float) {
    let c = v * s
    let x = c * (1 - abs((h / 60).truncatingRemainder(dividingBy: 2) - 1))
    let m = v - c
    var (r, g, b): (Float, Float, Float)
    switch Int(h / 60) {
    case 0: (r, g, b) = (c, x, 0)
    case 1: (r, g, b) = (x, c, 0)
    case 2: (r, g, b) = (0, c, x)
    case 3: (r, g, b) = (0, x, c)
    case 4: (r, g, b) = (x, 0, c)
    default: (r, g, b) = (c, 0, x)
    }
    return (r + m, g + m, b + m)
}

/// Hue rotate: rotate hue by `degrees`.  360° is identity.
public func hueRotate(_ src: PixelContainer, degrees: Float) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        let (h, s, v) = rgbToHSV(decode(r), decode(g), decode(b))
        let (nr, ng, nb) = hsvToRGB((h + degrees + 360).truncatingRemainder(dividingBy: 360), s, v)
        return (encode(nr), encode(ng), encode(nb), a)
    }
}

// ── Colorspace utilities ───────────────────────────────────────────────────

/// Convert sRGB → linear (each byte becomes linear × 255 rounded).
public func srgbToLinearImage(_ src: PixelContainer) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        let toLinByte: (UInt8) -> UInt8 = { v in UInt8(min(255, max(0, Int((decode(v) * 255).rounded())))) }
        return (toLinByte(r), toLinByte(g), toLinByte(b), a)
    }
}

/// Convert linear → sRGB (inverse of srgbToLinearImage).
public func linearToSRGBImage(_ src: PixelContainer) -> PixelContainer {
    mapPixels(src) { r, g, b, a in
        (encode(Float(r) / 255), encode(Float(g) / 255), encode(Float(b) / 255), a)
    }
}

// ── 1D LUT operations ──────────────────────────────────────────────────────

/// Apply three 256-entry u8→u8 LUTs (one per channel).  Alpha preserved.
public func applyLUT1DU8(
    _ src: PixelContainer,
    lutR: [UInt8], lutG: [UInt8], lutB: [UInt8]
) -> PixelContainer {
    mapPixels(src) { r, g, b, a in (lutR[Int(r)], lutG[Int(g)], lutB[Int(b)], a) }
}

/// Build a 256-entry LUT from a linear-light mapping function f: [0,1]→[0,1].
public func buildLUT1DU8(_ fn: (Float) -> Float) -> [UInt8] {
    (0..<256).map { i in encode(fn(decode(UInt8(i)))) }
}

/// Build a gamma LUT (equivalent to buildLUT1DU8 { v in pow(v, g) }).
public func buildGammaLUT(g: Float) -> [UInt8] {
    buildLUT1DU8 { v in pow(v, g) }
}
