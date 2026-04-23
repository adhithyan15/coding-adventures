/**
 * @coding-adventures/image-geometric-transforms
 *
 * IMG04: Geometric transforms on PixelContainer.
 *
 * This package implements the full set of spatial transforms needed to
 * manipulate digital images: lossless byte-level ops (flip, rotate 90°, crop,
 * pad) and continuous warps (scale, arbitrary rotation, affine, perspective)
 * that require resampling.
 *
 * ## Inverse-Warp Convention
 *
 * Every continuous transform is implemented as an *inverse warp* (also called
 * "pull-based" or "backward mapping"):
 *
 *   for each output pixel (x', y'):
 *     compute the corresponding source coordinate (u, v)
 *     sample the source image at (u, v)
 *     write the result into the output at (x', y')
 *
 * The alternative — *forward warp* — iterates over source pixels and pushes
 * them into the output.  Forward warps leave holes (unmapped output pixels)
 * whenever the transform compresses the image, and they require atomic writes
 * when mapping is many-to-one.  Inverse warps have neither problem: every
 * output pixel is visited exactly once, and sampling at (u, v) is always
 * well-defined (with an out-of-bounds policy for (u, v) outside the source).
 *
 * ## Pixel-Centre Model (+0.5 / -0.5)
 *
 * A pixel at integer coordinates (x, y) occupies a unit square centred at
 * (x + 0.5, y + 0.5) in continuous space.  When we scale an image by sx in
 * x, the centre of output pixel x' maps to continuous source coordinate:
 *
 *   u = (x' + 0.5) / sx - 0.5
 *
 * Without the +0.5/-0.5 offset the left and right edges of the image
 * would not align correctly: the first output pixel would sample half a pixel
 * to the left of the source image boundary, causing a systematic half-pixel
 * shift.  This is the same model used by OpenGL, Metal, and browser canvases.
 *
 * ## Why Bilinear / Bicubic Operate in Linear Light
 *
 * sRGB bytes store values on a *perceptual* (gamma-compressed) scale:
 * byte value 128 is not half the physical light intensity of byte 255 —
 * it is roughly (128/255)^2.2 ≈ 22 % of 255's intensity.
 *
 * If we blend two sRGB values directly (e.g. averaging 0 and 255 to get 127)
 * we obtain a value that appears too dark.  The correct procedure is:
 *
 *   1. Decode each sRGB byte to a linear-light float in [0, 1] using the
 *      official IEC 61966-2-1 formula.
 *   2. Perform the weighted average in linear space.
 *   3. Re-encode the result back to an sRGB byte.
 *
 * Nearest-neighbour does not blend, so it operates directly on raw bytes.
 * Bilinear and bicubic do blend, so they must round-trip through linear light.
 *
 * ## Catmull-Rom Kernel
 *
 * Catmull-Rom is a piecewise-cubic interpolation kernel parametrised by
 * (B=0, C=0.5) in the Mitchell–Netravali family.  For a distance |d| from the
 * sample point:
 *
 *   |d| < 1:  f(d) =  1.5|d|³ − 2.5|d|² + 1
 *   |d| < 2:  f(d) = −0.5|d|³ + 2.5|d|² − 4|d| + 2
 *   otherwise: 0
 *
 * The kernel exactly reproduces polynomials up to degree 3 and interpolates
 * (passes through) all sampled values, unlike the Mitchell filter which
 * introduces slight blurring to reduce ringing.  For a 4×4 neighbourhood we
 * compute 4 horizontal weights and 4 vertical weights, blend 4 rows of 4
 * pixels each into 4 horizontally-blended values, then blend those vertically.
 */

import {
  PixelContainer,
  createPixelContainer,
  pixelAt,
  setPixel,
} from "@coding-adventures/pixel-container";

// ============================================================================
// Public type aliases
// ============================================================================

/** The interpolation kernel used for continuous-coordinate sampling. */
export type Interpolation = "nearest" | "bilinear" | "bicubic";

/**
 * Controls output dimensions during arbitrary-angle rotation:
 *   - 'fit'  — output is enlarged so that no source pixel is clipped.
 *   - 'crop' — output keeps the original dimensions; corners are cropped.
 */
export type RotateBounds = "fit" | "crop";

/**
 * How to handle source coordinates that fall outside [0, width) × [0, height):
 *   - 'zero'      — return transparent black (0,0,0,0).
 *   - 'replicate' — clamp to the nearest border pixel.
 *   - 'reflect'   — mirror the image at each border (like a bathroom-tile mirror).
 *   - 'wrap'      — tile the image periodically.
 */
export type OutOfBounds = "zero" | "replicate" | "reflect" | "wrap";

/** A single RGBA8 pixel: four integers in [0, 255]. */
export type Rgba8 = [number, number, number, number];

// ============================================================================
// sRGB ↔ linear-light conversion
// ============================================================================

/**
 * Pre-computed sRGB-to-linear lookup table.
 *
 * Building this once at module load (inside an IIFE) avoids per-pixel
 * branching.  The IEC 61966-2-1 piecewise formula is:
 *
 *   if byte/255 <= 0.04045: linear = (byte/255) / 12.92
 *   else:                    linear = ((byte/255 + 0.055) / 1.055) ^ 2.4
 *
 * Indexing by integer byte value [0, 255] gives O(1) decoding.
 */
const SRGB_TO_LINEAR: Float32Array = (() => {
  const lut = new Float32Array(256);
  for (let i = 0; i < 256; i++) {
    const c = i / 255;
    lut[i] = c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  }
  return lut;
})();

/**
 * Decode an sRGB byte to a linear-light float in [0, 1].
 * Uses the pre-built LUT for speed — no branching, no pow() call.
 */
function decode(b: number): number {
  return SRGB_TO_LINEAR[b & 0xff];
}

/**
 * Encode a linear-light float back to an sRGB byte.
 *
 * The inverse IEC 61966-2-1 formula:
 *   if v <= 0.0031308: out = 12.92 * v
 *   else:              out = 1.055 * v^(1/2.4) − 0.055
 *
 * We clamp to [0, 1] before encoding to handle tiny floating-point overflows
 * that can appear after Catmull-Rom's negative lobes.
 */
function encode(v: number): number {
  const c =
    v <= 0.0031308 ? 12.92 * v : 1.055 * Math.pow(v, 1 / 2.4) - 0.055;
  return Math.round(Math.min(1, Math.max(0, c)) * 255);
}

// ============================================================================
// Out-of-bounds coordinate resolution
// ============================================================================

/**
 * Map a potentially-OOB integer coordinate to a valid in-bounds coordinate,
 * or return null when the OOB policy is 'zero'.
 *
 * @param x   The integer coordinate (may be negative or ≥ max).
 * @param max The dimension size (width or height); valid range is [0, max).
 * @param oob The out-of-bounds policy.
 * @returns   A valid coordinate in [0, max), or null (for 'zero' policy OOB).
 *
 * ### 'zero' policy
 * Returns null for any coordinate outside [0, max).  The caller interprets
 * null as transparent black.  This is the correct policy for rotation where
 * corners of the rotated image don't cover the source.
 *
 * ### 'replicate' policy
 * Clamp: clamp(x, 0, max−1).  Pixels beyond the edge repeat the border colour.
 * This is the best default for scaling because it avoids the dark halo that
 * 'zero' creates at image borders.
 *
 * ### 'reflect' policy
 * Mirror the image at each border.  Period is 2*max.  Visualise the image
 * flanked by mirror images of itself — coordinate x maps into that infinite
 * tiling and the result is folded back.
 *
 * ### 'wrap' policy
 * Tile the image periodically.  Equivalent to taking x modulo max, with
 * correct handling of negative x in JavaScript's % operator.
 */
function resolve(x: number, max: number, oob: OutOfBounds): number | null {
  if (x >= 0 && x < max) return x;

  switch (oob) {
    case "zero":
      return null;

    case "replicate":
      return Math.min(max - 1, Math.max(0, x));

    case "reflect": {
      const period = 2 * max;
      let r = ((x % period) + period) % period;
      if (r >= max) r = period - 1 - r;
      return r;
    }

    case "wrap":
      return ((x % max) + max) % max;
  }
}

// ============================================================================
// Catmull-Rom cubic weight
// ============================================================================

/**
 * Catmull-Rom weight for distance d from the reconstruction point.
 *
 * This is the (B=0, C=0.5) Mitchell–Netravali kernel, also known as
 * Catmull-Rom.  It is a piecewise cubic:
 *
 *   |d| < 1:  1.5d³ − 2.5d² + 1
 *   |d| < 2: −0.5d³ + 2.5d² − 4d + 2    (with absolute value of d)
 *   else:    0
 *
 * Key properties:
 *   - Interpolating: w(0) = 1, w(n) = 0 for all other integers n.
 *   - Compact support: zero outside [−2, +2].
 *   - C1 continuous: no discontinuity in the first derivative.
 *   - Negative lobes: values slightly outside [0,1] can appear, so encode()
 *     must clamp before converting to a byte.
 *
 * @param d  Signed distance.  Typically in (−2, +2) for 4×4 bicubic sampling.
 */
function catmullRom(d: number): number {
  const a = Math.abs(d);
  if (a >= 2) return 0;
  if (a >= 1)
    return -0.5 * a * a * a + 2.5 * a * a - 4 * a + 2;
  return 1.5 * a * a * a - 2.5 * a * a + 1;
}

// ============================================================================
// Sampling functions
// ============================================================================

/**
 * Sample the image at continuous coordinate (u, v) using nearest-neighbour.
 *
 * Nearest-neighbour rounds to the nearest integer pixel.  No blending
 * occurs — the raw sRGB byte values are returned without any colour-space
 * conversion.  This makes it perfectly lossless for integer-aligned coordinates
 * and very fast for all coordinates.
 *
 * Round-then-resolve means a coordinate at exactly 0.5 is rounded up (Math.round
 * uses "round half away from zero" in JS).
 */
function sampleNearest(
  img: PixelContainer,
  u: number,
  v: number,
  oob: OutOfBounds
): Rgba8 {
  const xi = resolve(Math.round(u), img.width, oob);
  const yi = resolve(Math.round(v), img.height, oob);
  if (xi === null || yi === null) return [0, 0, 0, 0];
  return pixelAt(img, xi, yi);
}

/**
 * Sample the image at (u, v) using bilinear interpolation in linear light.
 *
 * Bilinear interpolation takes a weighted average of the four axis-aligned
 * neighbours of (u, v):
 *
 *   (x0, y0)  (x1, y0)
 *   (x0, y1)  (x1, y1)
 *
 * where x0 = floor(u), x1 = x0+1 (and similarly for y).  Weights are:
 *   wx1 = frac(u) = u − x0   (how far right of x0)
 *   wx0 = 1 − wx1
 *   wy1 = frac(v) = v − y0
 *   wy0 = 1 − wy1
 *
 * The four weights sum to 1, so the result is a convex combination.  We
 * decode to linear light before blending and re-encode afterwards; without
 * this the blended colour would appear too dark (see module comment).
 *
 * Alpha is blended linearly in its natural [0, 1] space without going through
 * the sRGB curve, because alpha is a coverage value (linear by definition).
 */
function sampleBilinear(
  img: PixelContainer,
  u: number,
  v: number,
  oob: OutOfBounds
): Rgba8 {
  const x0 = Math.floor(u);
  const y0 = Math.floor(v);
  const wx1 = u - x0;
  const wx0 = 1 - wx1;
  const wy1 = v - y0;
  const wy0 = 1 - wy1;

  // Resolve each of the four neighbour coordinates.
  const x0r = resolve(x0, img.width, oob);
  const x1r = resolve(x0 + 1, img.width, oob);
  const y0r = resolve(y0, img.height, oob);
  const y1r = resolve(y0 + 1, img.height, oob);

  // Read pixels, substituting black for out-of-bounds (null) positions.
  const p00 = x0r !== null && y0r !== null ? pixelAt(img, x0r, y0r) : ([0, 0, 0, 0] as Rgba8);
  const p10 = x1r !== null && y0r !== null ? pixelAt(img, x1r, y0r) : ([0, 0, 0, 0] as Rgba8);
  const p01 = x0r !== null && y1r !== null ? pixelAt(img, x0r, y1r) : ([0, 0, 0, 0] as Rgba8);
  const p11 = x1r !== null && y1r !== null ? pixelAt(img, x1r, y1r) : ([0, 0, 0, 0] as Rgba8);

  // Blend R, G, B in linear light.
  const out: Rgba8 = [0, 0, 0, 0];
  for (let c = 0; c < 3; c++) {
    const lin =
      decode(p00[c]) * wx0 * wy0 +
      decode(p10[c]) * wx1 * wy0 +
      decode(p01[c]) * wx0 * wy1 +
      decode(p11[c]) * wx1 * wy1;
    out[c] = encode(lin);
  }
  // Alpha blended linearly.
  out[3] = Math.round(
    (p00[3] * wx0 * wy0 +
      p10[3] * wx1 * wy0 +
      p01[3] * wx0 * wy1 +
      p11[3] * wx1 * wy1)
  );
  return out;
}

/**
 * Sample the image at (u, v) using bicubic (Catmull-Rom) interpolation.
 *
 * Bicubic uses a 4×4 neighbourhood — the 16 pixels surrounding (u, v):
 *
 *   (x-1, y-1)  (x0, y-1)  (x+1, y-1)  (x+2, y-1)
 *   (x-1,  y0)  (x0,  y0)  (x+1,  y0)  (x+2,  y0)
 *   (x-1, y+1)  (x0, y+1)  (x+1, y+1)  (x+2, y+1)
 *   (x-1, y+2)  (x0, y+2)  (x+1, y+2)  (x+2, y+2)
 *
 * where x0 = floor(u), y0 = floor(v).
 *
 * Algorithm (separable filter):
 *   1. Compute Catmull-Rom weights for the 4 columns (horizontal).
 *   2. For each of the 4 rows, blend the 4 pixels horizontally → 1 value.
 *   3. Blend the 4 row results vertically using Catmull-Rom row weights.
 *
 * Everything is done in linear light to avoid gamma-induced artefacts.
 * The Catmull-Rom kernel has negative lobes (values outside [0, 1] are
 * possible); encode() clamps before byte conversion.
 */
function sampleBicubic(
  img: PixelContainer,
  u: number,
  v: number,
  oob: OutOfBounds
): Rgba8 {
  const x0 = Math.floor(u);
  const y0 = Math.floor(v);

  // Horizontal Catmull-Rom weights for columns x0-1 … x0+2.
  const wu: number[] = [];
  for (let dx = -1; dx <= 2; dx++) wu.push(catmullRom(u - (x0 + dx)));

  // Vertical Catmull-Rom weights for rows y0-1 … y0+2.
  const wv: number[] = [];
  for (let dy = -1; dy <= 2; dy++) wv.push(catmullRom(v - (y0 + dy)));

  // Accumulate weighted sum in linear light for each channel.
  const acc = [0, 0, 0, 0];

  for (let dy = -1; dy <= 2; dy++) {
    const yr = resolve(y0 + dy, img.height, oob);
    for (let dx = -1; dx <= 2; dx++) {
      const xr = resolve(x0 + dx, img.width, oob);
      const px: Rgba8 = xr !== null && yr !== null
        ? pixelAt(img, xr, yr)
        : [0, 0, 0, 0];
      const w = wu[dx + 1] * wv[dy + 1];
      acc[0] += decode(px[0]) * w;
      acc[1] += decode(px[1]) * w;
      acc[2] += decode(px[2]) * w;
      acc[3] += (px[3] / 255) * w;  // alpha in [0,1] linear
    }
  }

  return [
    encode(acc[0]),
    encode(acc[1]),
    encode(acc[2]),
    Math.round(Math.min(255, Math.max(0, acc[3] * 255))),
  ];
}

/**
 * Dispatch to the appropriate sampling function.
 *
 * @param img  Source image.
 * @param u    Continuous source x coordinate.
 * @param v    Continuous source y coordinate.
 * @param mode Interpolation kernel.
 * @param oob  Out-of-bounds policy.
 */
export function sample(
  img: PixelContainer,
  u: number,
  v: number,
  mode: Interpolation,
  oob: OutOfBounds
): Rgba8 {
  switch (mode) {
    case "nearest":
      return sampleNearest(img, u, v, oob);
    case "bilinear":
      return sampleBilinear(img, u, v, oob);
    case "bicubic":
      return sampleBicubic(img, u, v, oob);
  }
}

// ============================================================================
// Lossless transforms — raw byte operations, no sRGB conversion
// ============================================================================

/**
 * Flip an image horizontally (mirror left↔right).
 *
 * Each row's pixel order is reversed.  Because pixels are stored row-major,
 * we simply swap the data at positions (x, y) and (W−1−x, y) for all rows y
 * and columns x in [0, W/2).
 *
 * This is a lossless operation: no interpolation, no colour-space conversion.
 * Double-flipping is the identity.
 */
export function flipHorizontal(src: PixelContainer): PixelContainer {
  const out = createPixelContainer(src.width, src.height);
  for (let y = 0; y < src.height; y++) {
    for (let x = 0; x < src.width; x++) {
      const [r, g, b, a] = pixelAt(src, x, y);
      setPixel(out, src.width - 1 - x, y, r, g, b, a);
    }
  }
  return out;
}

/**
 * Flip an image vertically (mirror top↔bottom).
 *
 * Row order is reversed.  Pixel (x, y) in the source maps to (x, H−1−y)
 * in the output.  Lossless; double-flip is the identity.
 */
export function flipVertical(src: PixelContainer): PixelContainer {
  const out = createPixelContainer(src.width, src.height);
  for (let y = 0; y < src.height; y++) {
    for (let x = 0; x < src.width; x++) {
      const [r, g, b, a] = pixelAt(src, x, y);
      setPixel(out, x, src.height - 1 - y, r, g, b, a);
    }
  }
  return out;
}

/**
 * Rotate an image 90° clockwise.
 *
 * The output dimensions swap: W' = H, H' = W.
 *
 * Derivation using inverse warp (screen-coord convention, y axis pointing down):
 *
 * A 90° CW rotation maps source pixel (x, y) to output pixel:
 *   x' = H − 1 − y    (new column = mirrored old row)
 *   y' = x             (new row = old column)
 *
 * Inverting: given output (x', y'), find source (x, y):
 *   y = H − 1 − x'   (H = src.height)
 *   x = y'
 *
 * So: out[x'][y'] = in[x = y'][y = H−1−x']
 *
 * Lossless; four CW rotations return the original image.
 */
export function rotate90CW(src: PixelContainer): PixelContainer {
  const outW = src.height;
  const outH = src.width;
  const out = createPixelContainer(outW, outH);
  for (let yp = 0; yp < outH; yp++) {
    for (let xp = 0; xp < outW; xp++) {
      // Source: x = y', y = H−1−x'
      const [r, g, b, a] = pixelAt(src, yp, src.height - 1 - xp);
      setPixel(out, xp, yp, r, g, b, a);
    }
  }
  return out;
}

/**
 * Rotate an image 90° counter-clockwise.
 *
 * Output dimensions swap: W' = H, H' = W.
 *
 * Derivation using inverse warp (y axis pointing down):
 *
 * A 90° CCW rotation maps source pixel (x, y) to output pixel:
 *   x' = y             (new column = old row)
 *   y' = W − 1 − x    (new row = mirrored old column; W = src.width)
 *
 * Inverting: given output (x', y'), find source (x, y):
 *   y = x'
 *   x = W − 1 − y'
 *
 * So: out[x'][y'] = in[x = W−1−y'][y = x']
 *
 * Lossless; four CCW rotations return the original image.
 */
export function rotate90CCW(src: PixelContainer): PixelContainer {
  const outW = src.height;
  const outH = src.width;
  const out = createPixelContainer(outW, outH);
  for (let yp = 0; yp < outH; yp++) {
    for (let xp = 0; xp < outW; xp++) {
      // Source: x = W−1−y', y = x'
      const [r, g, b, a] = pixelAt(src, src.width - 1 - yp, xp);
      setPixel(out, xp, yp, r, g, b, a);
    }
  }
  return out;
}

/**
 * Rotate an image 180°.
 *
 * Dimensions are preserved.  Pixel (x, y) maps to (W−1−x, H−1−y).
 *
 * Lossless; double application is the identity.
 */
export function rotate180(src: PixelContainer): PixelContainer {
  const out = createPixelContainer(src.width, src.height);
  for (let y = 0; y < src.height; y++) {
    for (let x = 0; x < src.width; x++) {
      const [r, g, b, a] = pixelAt(src, src.width - 1 - x, src.height - 1 - y);
      setPixel(out, x, y, r, g, b, a);
    }
  }
  return out;
}

/**
 * Crop a rectangular region from the source image.
 *
 * Extracts the rectangle with top-left corner (x0, y0), width w, height h.
 * Coordinates that fall outside the source are read as transparent black
 * (the default behaviour of pixelAt for OOB positions).
 *
 * @param src  Source image.
 * @param x0   Left edge of the crop box (inclusive).
 * @param y0   Top edge of the crop box (inclusive).
 * @param w    Width of the output.
 * @param h    Height of the output.
 */
export function crop(
  src: PixelContainer,
  x0: number,
  y0: number,
  w: number,
  h: number
): PixelContainer {
  const out = createPixelContainer(w, h);
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const [r, g, b, a] = pixelAt(src, x0 + x, y0 + y);
      setPixel(out, x, y, r, g, b, a);
    }
  }
  return out;
}

/**
 * Add a solid-colour border around the source image.
 *
 * The output dimensions are:
 *   W' = left + src.width  + right
 *   H' = top  + src.height + bottom
 *
 * The border pixels are filled with `fill`.  The interior (the copied source)
 * is placed at offset (left, top).
 *
 * @param src    Source image.
 * @param top    Number of pixels of border above the image.
 * @param right  Pixels of border to the right.
 * @param bottom Pixels of border below.
 * @param left   Pixels of border to the left.
 * @param fill   RGBA colour for the border.
 */
export function pad(
  src: PixelContainer,
  top: number,
  right: number,
  bottom: number,
  left: number,
  fill: Rgba8
): PixelContainer {
  const outW = left + src.width + right;
  const outH = top + src.height + bottom;
  const out = createPixelContainer(outW, outH);

  // Fill entire canvas with the border colour.
  const [fr, fg, fb, fa] = fill;
  for (let i = 0; i < out.data.length; i += 4) {
    out.data[i]     = fr;
    out.data[i + 1] = fg;
    out.data[i + 2] = fb;
    out.data[i + 3] = fa;
  }

  // Copy source pixels into the interior.
  for (let y = 0; y < src.height; y++) {
    for (let x = 0; x < src.width; x++) {
      const [r, g, b, a] = pixelAt(src, x, y);
      setPixel(out, left + x, top + y, r, g, b, a);
    }
  }

  return out;
}

// ============================================================================
// Continuous transforms — with interpolation
// ============================================================================

/**
 * Scale an image to a new size using the specified interpolation mode.
 *
 * Uses the inverse-warp pixel-centre model.  For output pixel x' in an image
 * scaled by factor sx = outW / src.width, the continuous source coordinate is:
 *
 *   u = (x' + 0.5) / sx − 0.5
 *
 * This ensures that the pixel grid aligns at both edges.  Example: scaling a
 * 2-pixel-wide image (centres at 0.5 and 1.5) to 4 pixels (centres at 0.5,
 * 1.5, 2.5, 3.5 in output space):
 *
 *   sx = 4/2 = 2
 *   x'=0: u = (0.5)/2 − 0.5 = −0.25    (just left of pixel 0)
 *   x'=1: u = (1.5)/2 − 0.5 =  0.25    (just right of pixel 0)
 *   x'=2: u = (2.5)/2 − 0.5 =  0.75    (just left of pixel 1)
 *   x'=3: u = (3.5)/2 − 0.5 =  1.25    (just right of pixel 1)
 *
 * 'replicate' OOB is used so that scaled borders extend cleanly.
 *
 * @param src   Source image.
 * @param outW  Target width.
 * @param outH  Target height.
 * @param mode  Interpolation kernel (default: 'bilinear').
 */
export function scale(
  src: PixelContainer,
  outW: number,
  outH: number,
  mode: Interpolation = "bilinear"
): PixelContainer {
  const out = createPixelContainer(outW, outH);
  const sx = outW / src.width;
  const sy = outH / src.height;

  for (let yp = 0; yp < outH; yp++) {
    for (let xp = 0; xp < outW; xp++) {
      const u = (xp + 0.5) / sx - 0.5;
      const v = (yp + 0.5) / sy - 0.5;
      const [r, g, b, a] = sample(src, u, v, mode, "replicate");
      setPixel(out, xp, yp, r, g, b, a);
    }
  }

  return out;
}

/**
 * Rotate an image by an arbitrary angle (in radians) around its centre.
 *
 * Implements inverse warp with optional output-size fitting.
 *
 * ### Fit vs Crop
 *
 *   'fit'  — output is the smallest axis-aligned rectangle that contains the
 *             entire rotated source image.  Computed as:
 *               W' = ⌈W·|cos θ| + H·|sin θ|⌉
 *               H' = ⌈W·|sin θ| + H·|cos θ|⌉
 *
 *   'crop' — output keeps the original W×H dimensions; the rotated image is
 *             centred but corners may be cut off.
 *
 * ### Inverse rotation matrix
 *
 * To find the source coordinate (u, v) for output pixel (x', y'), we apply
 * the *inverse* rotation (by −θ) around the centre of the output:
 *
 *   Δx = x' − cxOut,  Δy = y' − cyOut
 *   u  = cxIn + cos(θ)·Δx + sin(θ)·Δy
 *   v  = cyIn − sin(θ)·Δx + cos(θ)·Δy
 *
 * Note the sign: the inverse of a CCW rotation by θ is rotation by −θ,
 * which has cos(−θ) = cos(θ) and sin(−θ) = −sin(θ).  The matrix is:
 *   [ cos  sin ]
 *   [−sin  cos ]
 *
 * 'zero' OOB is used so that background areas outside the rotated image are
 * transparent black rather than border replication.
 *
 * @param src      Source image.
 * @param radians  Rotation angle; positive = counter-clockwise.
 * @param mode     Interpolation kernel (default: 'bilinear').
 * @param bounds   'fit' or 'crop' (default: 'fit').
 */
export function rotate(
  src: PixelContainer,
  radians: number,
  mode: Interpolation = "bilinear",
  bounds: RotateBounds = "fit"
): PixelContainer {
  const W = src.width;
  const H = src.height;
  const cosA = Math.cos(radians);
  const sinA = Math.sin(radians);

  let outW: number;
  let outH: number;

  if (bounds === "fit") {
    outW = Math.ceil(W * Math.abs(cosA) + H * Math.abs(sinA));
    outH = Math.ceil(W * Math.abs(sinA) + H * Math.abs(cosA));
  } else {
    outW = W;
    outH = H;
  }

  const cxIn  = W / 2;
  const cyIn  = H / 2;
  const cxOut = outW / 2;
  const cyOut = outH / 2;

  const out = createPixelContainer(outW, outH);

  for (let yp = 0; yp < outH; yp++) {
    for (let xp = 0; xp < outW; xp++) {
      const dx = xp - cxOut;
      const dy = yp - cyOut;
      const u = cxIn + cosA * dx + sinA * dy;
      const v = cyIn - sinA * dx + cosA * dy;
      const [r, g, b, a] = sample(src, u, v, mode, "zero");
      setPixel(out, xp, yp, r, g, b, a);
    }
  }

  return out;
}

/**
 * Apply an affine transform to an image.
 *
 * An affine transform combines any combination of scale, rotation, shear,
 * and translation.  It preserves parallel lines.  The 2×3 matrix encodes:
 *
 *   | u |   | m[0][0]  m[0][1]  m[0][2] |   | x' |
 *   | v | = | m[1][0]  m[1][1]  m[1][2] | × | y' |
 *                                            |  1 |
 *
 * i.e.  u = m[0][0]·x' + m[0][1]·y' + m[0][2]
 *           v = m[1][0]·x' + m[1][1]·y' + m[1][2]
 *
 * This is a forward-mapping description: for each output pixel we compute
 * the source coordinate directly (inverse warp).
 *
 * ### Identity matrix
 *   [[1, 0, 0], [0, 1, 0]] maps every output pixel to the same source pixel.
 *
 * @param src    Source image.
 * @param matrix A 2×3 affine matrix.
 * @param outW   Output width.
 * @param outH   Output height.
 * @param mode   Interpolation (default: 'bilinear').
 * @param oob    Out-of-bounds policy (default: 'zero').
 */
export function affine(
  src: PixelContainer,
  matrix: [[number, number, number], [number, number, number]],
  outW: number,
  outH: number,
  mode: Interpolation = "bilinear",
  oob: OutOfBounds = "zero"
): PixelContainer {
  const out = createPixelContainer(outW, outH);

  for (let yp = 0; yp < outH; yp++) {
    for (let xp = 0; xp < outW; xp++) {
      const u = matrix[0][0] * xp + matrix[0][1] * yp + matrix[0][2];
      const v = matrix[1][0] * xp + matrix[1][1] * yp + matrix[1][2];
      const [r, g, b, a] = sample(src, u, v, mode, oob);
      setPixel(out, xp, yp, r, g, b, a);
    }
  }

  return out;
}

/**
 * Apply a projective (perspective) warp to an image.
 *
 * A homography (projective transform) is a 3×3 matrix H that maps points in
 * homogeneous coordinates.  Given output pixel (x', y'), the homogeneous
 * source point is:
 *
 *   [uh, vh, w]ᵀ = H · [x', y', 1]ᵀ
 *
 * i.e.
 *   uh = h[0][0]·x' + h[0][1]·y' + h[0][2]
 *   vh = h[1][0]·x' + h[1][1]·y' + h[1][2]
 *    w = h[2][0]·x' + h[2][1]·y' + h[2][2]
 *
 * The Euclidean source coordinates are obtained by dividing by w:
 *   u = uh / w
 *   v = vh / w
 *
 * When w = 0 the point is at infinity; we treat it as out-of-bounds.
 *
 * ### Identity homography
 *   [[1,0,0],[0,1,0],[0,0,1]] maps every output pixel to the same source pixel.
 *
 * ### Why perspective?
 * Affine transforms cannot model the foreshortening of a planar surface viewed
 * at an angle (e.g. a billboard in 3D space).  The division by w captures that
 * non-linear distortion.
 *
 * @param src   Source image.
 * @param h     3×3 homography matrix.
 * @param outW  Output width.
 * @param outH  Output height.
 * @param mode  Interpolation (default: 'bilinear').
 * @param oob   Out-of-bounds policy (default: 'zero').
 */
export function perspectiveWarp(
  src: PixelContainer,
  h: [[number, number, number], [number, number, number], [number, number, number]],
  outW: number,
  outH: number,
  mode: Interpolation = "bilinear",
  oob: OutOfBounds = "zero"
): PixelContainer {
  const out = createPixelContainer(outW, outH);

  for (let yp = 0; yp < outH; yp++) {
    for (let xp = 0; xp < outW; xp++) {
      const uh = h[0][0] * xp + h[0][1] * yp + h[0][2];
      const vh = h[1][0] * xp + h[1][1] * yp + h[1][2];
      const w  = h[2][0] * xp + h[2][1] * yp + h[2][2];

      if (w === 0) {
        setPixel(out, xp, yp, 0, 0, 0, 0);
        continue;
      }

      const u = uh / w;
      const v = vh / w;
      const [r, g, b, a] = sample(src, u, v, mode, oob);
      setPixel(out, xp, yp, r, g, b, a);
    }
  }

  return out;
}
