/**
 * @coding-adventures/pixel-container
 *
 * IC00: Universal RGBA8 pixel buffer and image codec interface.
 *
 * This is the zero-dependency foundation for the IC image codec series.
 * Every image codec (BMP, PPM, QOI, PNG, ...) depends only on this package.
 * The paint stack (paint-instructions, paint-vm) re-exports these types so
 * that existing imports continue to work unchanged.
 *
 * ## Layout
 *
 * Pixels are stored row-major, top-left origin, RGBA interleaved:
 *
 *   offset = (y * width + x) * 4
 *   data[offset + 0] = R
 *   data[offset + 1] = G
 *   data[offset + 2] = B
 *   data[offset + 3] = A
 *
 * Every pixel is always 4 bytes — channel count and bit depth are fixed at
 * 4 channels, 8 bits each. This keeps codec implementations simple: no
 * conditional logic for RGB vs RGBA, no 16-bit paths.
 *
 * ## Why fixed RGBA8?
 *
 * The three codecs in this series (BMP, PPM, QOI) all operate on RGBA8 pixels.
 * Supporting variable channels and bit depths at the type level would require
 * conditional logic in every encode/decode path, every test, and every caller.
 * Fixed RGBA8 is the "lingua franca" of GPU textures and browser canvases.
 */
export const VERSION = "0.1.0";

// ============================================================================
// PixelContainer — the one data type
// ============================================================================

/**
 * A fixed-format RGBA8 pixel buffer.
 *
 * `data` is a flat Uint8Array of `width × height × 4` bytes.
 * The `data` property is readonly (you cannot swap in a different array), but
 * the array contents are mutable — setPixel() and fillPixels() both work by
 * writing directly into `data[i]`.
 *
 * Example — 2×1 image: one red pixel then one blue pixel:
 *
 *   const c = createPixelContainer(2, 1);
 *   setPixel(c, 0, 0, 255, 0, 0, 255); // (x=0, y=0) → red
 *   setPixel(c, 1, 0, 0, 0, 255, 255); // (x=1, y=0) → blue
 *   pixelAt(c, 0, 0); // → [255, 0, 0, 255]
 *   pixelAt(c, 1, 0); // → [0, 0, 255, 255]
 */
export interface PixelContainer {
  readonly width: number;
  readonly height: number;
  readonly data: Uint8Array; // RGBA8 row-major, offset = (y*width+x)*4
}

// ============================================================================
// ImageCodec — encode/decode contract
// ============================================================================

/**
 * An image codec: converts a PixelContainer to file bytes and back.
 *
 * Encode pipeline:
 *   const pixels = createPixelContainer(320, 240);
 *   fillPixels(pixels, 128, 0, 200, 255);     // purple
 *   const bytes = codec.encode(pixels);        // → BMP / PPM / QOI bytes
 *   fs.writeFileSync("out.bmp", Buffer.from(bytes));
 *
 * Decode pipeline:
 *   const raw = fs.readFileSync("photo.bmp");
 *   const pixels = codec.decode(new Uint8Array(raw));
 *   const [r, g, b, a] = pixelAt(pixels, 10, 20);
 */
export interface ImageCodec {
  readonly mimeType: string; // e.g. "image/bmp", "image/x-portable-pixmap"
  encode(pixels: PixelContainer): Uint8Array;
  decode(bytes: Uint8Array): PixelContainer;
}

// ============================================================================
// createPixelContainer — factory
// ============================================================================

/**
 * Create a new pixel container filled with transparent black (R=G=B=A=0).
 *
 * `width` and `height` must be non-negative integers. A 0×0 container is
 * valid and has an empty data array.
 *
 * Example:
 *   const c = createPixelContainer(320, 240);
 *   console.log(c.data.length); // 307200 (320 * 240 * 4)
 */
export function createPixelContainer(width: number, height: number): PixelContainer {
  return {
    width,
    height,
    data: new Uint8Array(width * height * 4),
  };
}

// ============================================================================
// pixelAt — read one pixel
// ============================================================================

/**
 * Return the RGBA components of the pixel at column `x`, row `y`.
 *
 * Returns [0, 0, 0, 0] for out-of-bounds coordinates so callers do not need
 * to bounds-check before reading border pixels.
 *
 * Example:
 *   const c = createPixelContainer(4, 4);
 *   setPixel(c, 1, 2, 200, 100, 50, 255);
 *   pixelAt(c, 1, 2);  // → [200, 100, 50, 255]
 *   pixelAt(c, 99, 0); // → [0, 0, 0, 0]  (out of bounds)
 */
export function pixelAt(
  c: PixelContainer,
  x: number,
  y: number,
): [number, number, number, number] {
  if (x < 0 || x >= c.width || y < 0 || y >= c.height) {
    return [0, 0, 0, 0];
  }
  const i = (y * c.width + x) * 4;
  return [c.data[i], c.data[i + 1], c.data[i + 2], c.data[i + 3]];
}

// ============================================================================
// setPixel — write one pixel
// ============================================================================

/**
 * Write the RGBA components of the pixel at column `x`, row `y`.
 *
 * No-op for out-of-bounds coordinates.
 *
 * Values are expected in [0, 255]. They are written as-is into the Uint8Array,
 * which automatically clamps them to [0, 255] on write.
 *
 * Example:
 *   setPixel(c, 2, 3, 255, 128, 0, 255); // orange at (2, 3)
 */
export function setPixel(
  c: PixelContainer,
  x: number,
  y: number,
  r: number,
  g: number,
  b: number,
  a: number,
): void {
  if (x < 0 || x >= c.width || y < 0 || y >= c.height) return;
  const i = (y * c.width + x) * 4;
  c.data[i]     = r;
  c.data[i + 1] = g;
  c.data[i + 2] = b;
  c.data[i + 3] = a;
}

// ============================================================================
// fillPixels — flood the whole buffer with one colour
// ============================================================================

/**
 * Set every pixel in the container to the given RGBA colour.
 *
 * Useful for clearing a canvas before drawing, or for creating solid-colour
 * test fixtures:
 *
 *   fillPixels(c, 255, 255, 255, 255); // solid white
 *   fillPixels(c, 0, 0, 0, 0);         // transparent black (clear)
 *   fillPixels(c, 0, 0, 128, 255);     // solid navy blue
 */
export function fillPixels(
  c: PixelContainer,
  r: number,
  g: number,
  b: number,
  a: number,
): void {
  for (let i = 0; i < c.data.length; i += 4) {
    c.data[i]     = r;
    c.data[i + 1] = g;
    c.data[i + 2] = b;
    c.data[i + 3] = a;
  }
}
