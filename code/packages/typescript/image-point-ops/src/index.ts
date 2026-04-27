/**
 * @coding-adventures/image-point-ops
 *
 * IMG03 — Per-pixel point operations on PixelContainer.
 *
 * A point operation transforms each pixel independently using only that
 * pixel's value — no neighbourhood, no frequency domain, no geometry.
 *
 * ## Two domains
 *
 * u8-domain operations (invert, threshold, posterize, channel ops, brightness)
 * work directly on the 8-bit sRGB bytes.  They are correct without any colour-
 * space conversion because they are monotone remappings that do not mix or
 * average channel values.
 *
 * Linear-light operations (contrast, gamma, exposure, greyscale, sepia,
 * colourMatrix, saturate, hueRotate) must blend or weight values — averaging
 * in sRGB is incorrect (see IMG00 §2).  These decode each channel to linear
 * f32 first:
 *
 *   c = byte / 255
 *   linear = c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4
 *
 * Then re-encode after the operation:
 *
 *   encoded = linear <= 0.0031308 ? linear * 12.92 : 1.055 * linear ** (1/2.4) - 0.055
 *   byte = Math.round(Math.min(1, Math.max(0, encoded)) * 255)
 */

import type { PixelContainer } from "@coding-adventures/pixel-container";
import {
  createPixelContainer,
  pixelAt,
  setPixel,
} from "@coding-adventures/pixel-container";

export const VERSION = "0.1.0";

// ── sRGB ↔ linear helpers ──────────────────────────────────────────────────

// Pre-built 256-entry decode LUT — built once, reused everywhere.
const SRGB_TO_LINEAR: Float32Array = (() => {
  const t = new Float32Array(256);
  for (let i = 0; i < 256; i++) {
    const c = i / 255;
    t[i] = c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  }
  return t;
})();

function decode(byte: number): number {
  return SRGB_TO_LINEAR[byte];
}

function encode(linear: number): number {
  const c = linear <= 0.0031308 ? linear * 12.92 : 1.055 * Math.pow(linear, 1 / 2.4) - 0.055;
  return Math.round(Math.min(1, Math.max(0, c)) * 255);
}

// ── Iteration helper ───────────────────────────────────────────────────────

type PixelMapFn = (r: number, g: number, b: number, a: number) => [number, number, number, number];

function mapPixels(src: PixelContainer, fn: PixelMapFn): PixelContainer {
  const out = createPixelContainer(src.width, src.height);
  for (let y = 0; y < src.height; y++) {
    for (let x = 0; x < src.width; x++) {
      const [r, g, b, a] = pixelAt(src, x, y);
      const [or, og, ob, oa] = fn(r, g, b, a);
      setPixel(out, x, y, or, og, ob, oa);
    }
  }
  return out;
}

// ── u8-domain operations ───────────────────────────────────────────────────

/**
 * Invert: flip each RGB channel (255 − v).  Alpha is preserved.
 *
 * Example: white (255,255,255) → black (0,0,0).
 * Mathematically: complement in the 8-bit ring — applying invert twice
 * returns the original image exactly.
 */
export function invert(src: PixelContainer): PixelContainer {
  return mapPixels(src, (r, g, b, a) => [255 - r, 255 - g, 255 - b, a]);
}

/**
 * Threshold: binarise on average luminance.  Pixels with (r+g+b)/3 >= value
 * become white (255,255,255); all others become black (0,0,0).  Alpha is
 * preserved.
 *
 * Use threshold_luminance for a perceptually-weighted alternative.
 */
export function threshold(src: PixelContainer, value: number): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const luma = (r + g + b) / 3;
    const v = luma >= value ? 255 : 0;
    return [v, v, v, a];
  });
}

/**
 * Threshold on Rec. 709 luma: Y = 0.2126 R + 0.7152 G + 0.0722 B.
 * More perceptually accurate than simple average.
 */
export function thresholdLuminance(src: PixelContainer, value: number): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    const v = luma >= value ? 255 : 0;
    return [v, v, v, a];
  });
}

/**
 * Posterize: reduce each channel to `levels` equally-spaced steps.
 *
 * With levels = 2: [0..127] → 0, [128..255] → 255.
 * With levels = 4: four bands — 0, 85, 170, 255.
 *
 * Gives a poster / cartoon look.
 */
export function posterize(src: PixelContainer, levels: number): PixelContainer {
  const step = 255 / (levels - 1);
  return mapPixels(src, (r, g, b, a) => {
    const q = (v: number) => Math.round(Math.round(v / step) * step);
    return [q(r), q(g), q(b), a];
  });
}

/**
 * Swap R and B channels (RGB ↔ BGR).  Useful when an upstream codec emits
 * BGR byte order and you need to normalise to RGB.
 */
export function swapRgbBgr(src: PixelContainer): PixelContainer {
  return mapPixels(src, (r, g, b, a) => [b, g, r, a]);
}

/**
 * Extract a single colour channel: keep only R, G, B, or A and set the
 * others to 0.  Alpha is always preserved.
 *
 * channel: 0=R, 1=G, 2=B, 3=A.
 */
export function extractChannel(src: PixelContainer, channel: 0 | 1 | 2 | 3): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const vals = [r, g, b, a];
    const v = vals[channel];
    return channel === 0 ? [v, 0, 0, a]
         : channel === 1 ? [0, v, 0, a]
         : channel === 2 ? [0, 0, v, a]
         : [r, g, b, v];
  });
}

/**
 * Additive brightness: add a signed offset to each RGB channel and clamp
 * to [0, 255].  Alpha is preserved.
 *
 * offset = +30  →  brighter; offset = −30  →  darker.
 * This is a u8-domain operation (linear shift in sRGB — perceptually non-
 * uniform but fast and lossless on integer data).
 */
export function brightness(src: PixelContainer, offset: number): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const clamp = (v: number) => Math.min(255, Math.max(0, Math.round(v + offset)));
    return [clamp(r), clamp(g), clamp(b), a];
  });
}

// ── Linear-light operations ────────────────────────────────────────────────

/**
 * Contrast: scale each linear channel around mid-grey (0.5 in linear).
 *
 * factor = 1.0  →  identity; < 1.0  →  less contrast; > 1.0  →  more.
 * Clamped to [0, 1] linear after scaling.
 *
 * Formula:  linear_out = 0.5 + factor * (linear_in − 0.5)
 */
export function contrast(src: PixelContainer, factor: number): PixelContainer {
  return mapPixels(src, (r, g, b, a) => [
    encode(0.5 + factor * (decode(r) - 0.5)),
    encode(0.5 + factor * (decode(g) - 0.5)),
    encode(0.5 + factor * (decode(b) - 0.5)),
    a,
  ]);
}

/**
 * Gamma: apply a power-law γ to each linear channel.
 *
 * γ < 1  →  brightens; γ > 1  →  darkens; γ = 1  →  identity.
 * Applied in linear light after sRGB decoding.
 *
 * Formula: linear_out = linear_in ^ γ
 */
export function gamma(src: PixelContainer, g: number): PixelContainer {
  return mapPixels(src, (r, gb, b, a) => [
    encode(Math.pow(decode(r), g)),
    encode(Math.pow(decode(gb), g)),
    encode(Math.pow(decode(b), g)),
    a,
  ]);
}

/**
 * Exposure: multiply linear luminance by 2^stops.
 *
 * +1 stop  →  double the light; −1 stop  →  halve it.
 * Photographers' analogue: adjusting the camera aperture or shutter speed.
 *
 * Formula: linear_out = linear_in × 2^stops
 */
export function exposure(src: PixelContainer, stops: number): PixelContainer {
  const factor = Math.pow(2, stops);
  return mapPixels(src, (r, g, b, a) => [
    encode(decode(r) * factor),
    encode(decode(g) * factor),
    encode(decode(b) * factor),
    a,
  ]);
}

export type GreyscaleMethod = "rec709" | "bt601" | "average";

/**
 * Greyscale: convert to luminance using one of three weighting schemes.
 *
 * rec709 (default, perceptually correct for modern sRGB displays):
 *   Y = 0.2126 R + 0.7152 G + 0.0722 B
 * bt601 (legacy SD-TV):
 *   Y = 0.2989 R + 0.5870 G + 0.1140 B
 * average (fast, equal weights):
 *   Y = (R + G + B) / 3
 *
 * Computed in linear light, re-encoded to sRGB for the output.
 */
export function greyscale(src: PixelContainer, method: GreyscaleMethod = "rec709"): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const lr = decode(r), lg = decode(g), lb = decode(b);
    let y: number;
    if (method === "rec709") y = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
    else if (method === "bt601") y = 0.2989 * lr + 0.5870 * lg + 0.1140 * lb;
    else y = (lr + lg + lb) / 3;
    const out = encode(y);
    return [out, out, out, a];
  });
}

/**
 * Sepia: apply a warm sepia tone matrix (computed in linear light).
 *
 * The sepia matrix desaturates and shifts towards red-orange.  The classic
 * photographic darkroom effect from iron-gall ink development.
 *
 * Output R = 0.393 R + 0.769 G + 0.189 B  (etc.)
 */
export function sepia(src: PixelContainer): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const lr = decode(r), lg = decode(g), lb = decode(b);
    return [
      encode(0.393 * lr + 0.769 * lg + 0.189 * lb),
      encode(0.349 * lr + 0.686 * lg + 0.168 * lb),
      encode(0.272 * lr + 0.534 * lg + 0.131 * lb),
      a,
    ];
  });
}

/**
 * Colour matrix: multiply linear [R, G, B] by a 3×3 matrix.
 *
 * The matrix is stored row-major:
 *   [ m[0][0]  m[0][1]  m[0][2] ]
 *   [ m[1][0]  m[1][1]  m[1][2] ]
 *   [ m[2][0]  m[2][1]  m[2][2] ]
 *
 * Identity:  [[1,0,0],[0,1,0],[0,0,1]]
 * Use this to implement custom channel mixing (e.g. deuteranopia simulation).
 */
export function colourMatrix(
  src: PixelContainer,
  matrix: [[number, number, number], [number, number, number], [number, number, number]],
): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const lr = decode(r), lg = decode(g), lb = decode(b);
    const [m0, m1, m2] = matrix;
    return [
      encode(m0[0] * lr + m0[1] * lg + m0[2] * lb),
      encode(m1[0] * lr + m1[1] * lg + m1[2] * lb),
      encode(m2[0] * lr + m2[1] * lg + m2[2] * lb),
      a,
    ];
  });
}

/**
 * Saturate: scale the saturation of each pixel in linear RGB.
 *
 * factor = 0  →  greyscale; 1  →  identity; > 1  →  hypersaturated.
 *
 * Uses the Rec. 709 luminance weights to compute the grey value, then
 * interpolates between that and the original colour.
 *
 * Formula: out = grey + factor * (linear − grey)
 */
export function saturate(src: PixelContainer, factor: number): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const lr = decode(r), lg = decode(g), lb = decode(b);
    const grey = 0.2126 * lr + 0.7152 * lg + 0.0722 * lb;
    return [
      encode(grey + factor * (lr - grey)),
      encode(grey + factor * (lg - grey)),
      encode(grey + factor * (lb - grey)),
      a,
    ];
  });
}

// ── HSV helpers ────────────────────────────────────────────────────────────

function rgbToHsv(r: number, g: number, b: number): [number, number, number] {
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  const v = max;
  const s = max === 0 ? 0 : delta / max;
  let h = 0;
  if (delta !== 0) {
    if (max === r) h = ((g - b) / delta) % 6;
    else if (max === g) h = (b - r) / delta + 2;
    else h = (r - g) / delta + 4;
    h = (h * 60 + 360) % 360;
  }
  return [h, s, v];
}

function hsvToRgb(h: number, s: number, v: number): [number, number, number] {
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  let r = 0, g = 0, b = 0;
  if (h < 60) { r = c; g = x; }
  else if (h < 120) { r = x; g = c; }
  else if (h < 180) { g = c; b = x; }
  else if (h < 240) { g = x; b = c; }
  else if (h < 300) { r = x; b = c; }
  else { r = c; b = x; }
  return [r + m, g + m, b + m];
}

/**
 * Hue rotate: rotate the hue of each pixel by `degrees`.
 *
 * Performed in linear-light HSV space.  +90 rotates all hues 90° around the
 * colour wheel (e.g. red → yellow → green → …).
 * 360° is an identity transform.
 */
export function hueRotate(src: PixelContainer, degrees: number): PixelContainer {
  return mapPixels(src, (r, g, b, a) => {
    const [h, s, v] = rgbToHsv(decode(r), decode(g), decode(b));
    const [nr, ng, nb] = hsvToRgb((h + degrees + 360) % 360, s, v);
    return [encode(nr), encode(ng), encode(nb), a];
  });
}

// ── Colorspace utilities ───────────────────────────────────────────────────

/**
 * Convert a PixelContainer from sRGB to linear light.
 *
 * Output pixels store linear f32 in a Float32Array laid out identically to
 * the u8 buffer but with 4 floats per pixel instead of 4 bytes.  Because
 * PixelContainer is fixed at u8, the linear values are rounded and encoded
 * back to u8 using a linear (not gamma) encoding — i.e. the bytes represent
 * linear light directly, NOT sRGB.
 *
 * In practice: returns a new PixelContainer where each byte is the linear
 * value × 255 (clamped), suitable for doing arithmetic directly on the bytes.
 */
export function srgbToLinearImage(src: PixelContainer): PixelContainer {
  return mapPixels(src, (r, g, b, a) => [
    Math.round(decode(r) * 255),
    Math.round(decode(g) * 255),
    Math.round(decode(b) * 255),
    a,
  ]);
}

/**
 * Convert a PixelContainer from linear to sRGB encoding.
 * The inverse of srgbToLinearImage.
 */
export function linearToSrgbImage(src: PixelContainer): PixelContainer {
  return mapPixels(src, (r, g, b, a) => [
    encode(r / 255),
    encode(g / 255),
    encode(b / 255),
    a,
  ]);
}

// ── 1D LUT operations ──────────────────────────────────────────────────────

/**
 * Apply a 256-entry u8→u8 LUT to the R, G, and B channels independently.
 * Alpha is always preserved.
 *
 * A LUT (Look-Up Table) is a precomputed mapping: for each possible input
 * byte value 0–255, the table stores the corresponding output byte.
 * This is faster than recomputing the function per pixel because it reduces
 * any per-pixel operation to a single array lookup.
 *
 * Three separate LUTs let you apply different curves to each channel
 * (e.g. split-tone colour grading).
 */
export function applyLut1dU8(
  src: PixelContainer,
  lutR: Uint8Array,
  lutG: Uint8Array,
  lutB: Uint8Array,
): PixelContainer {
  return mapPixels(src, (r, g, b, a) => [lutR[r], lutG[g], lutB[b], a]);
}

/**
 * Build a 256-entry LUT from a mapping function f: [0,1] → [0,1] operating
 * in linear light.
 *
 * The function is sampled at each sRGB-decoded input value and the output is
 * re-encoded to sRGB bytes.  This lets you compile any linear-light function
 * (gamma, tone curve, etc.) into a fast u8 LUT.
 */
export function buildLut1dU8(fn: (linearIn: number) => number): Uint8Array {
  const lut = new Uint8Array(256);
  for (let i = 0; i < 256; i++) {
    lut[i] = encode(fn(decode(i)));
  }
  return lut;
}

/**
 * Build a gamma LUT: each input byte is decoded to linear, raised to power γ,
 * then re-encoded.  Equivalent to buildLut1dU8(v => v ** gamma).
 *
 * γ < 1 → brightens; γ > 1 → darkens; γ = 1 → identity.
 */
export function buildGammaLut(g: number): Uint8Array {
  return buildLut1dU8((v) => Math.pow(v, g));
}
