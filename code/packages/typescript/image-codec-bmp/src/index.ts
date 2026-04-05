/**
 * @coding-adventures/image-codec-bmp
 *
 * IC01: BMP image encoder and decoder.
 *
 * ## File Format
 *
 * BMP (Windows Bitmap) is one of the oldest and simplest raster formats.
 * There is no compression and no colour space metadata — just a header and
 * raw pixel data. This implementation produces 32-bit BGRA files (biBitCount=32,
 * biCompression=BI_RGB) because every pixel gets an alpha channel.
 *
 * ### File layout
 *
 *   Bytes 0–1:   Magic "BM"
 *   Bytes 2–5:   Total file size (u32 LE)
 *   Bytes 6–9:   Reserved (0)
 *   Bytes 10–13: Pixel data offset from start of file (u32 LE) — always 54
 *
 *   Bytes 14–17: BITMAPINFOHEADER size (u32 LE) — always 40
 *   Bytes 18–21: biWidth (i32 LE) — positive
 *   Bytes 22–25: biHeight (i32 LE) — NEGATIVE for top-down layout
 *   Bytes 26–27: biPlanes (u16 LE) — always 1
 *   Bytes 28–29: biBitCount (u16 LE) — always 32
 *   Bytes 30–33: biCompression (u32 LE) — BI_RGB = 0 (uncompressed)
 *   Bytes 34–37: biSizeImage (u32 LE) — pixel data size
 *   Bytes 38–45: biXPelsPerMeter, biYPelsPerMeter (both 0)
 *   Bytes 46–49: biClrUsed (0 = full palette used)
 *   Bytes 50–53: biClrImportant (0)
 *
 *   Bytes 54+: Raw pixel data, BGRA order (B, G, R, A per pixel)
 *
 * ### Top-down vs bottom-up
 *
 * The BMP spec's default is bottom-up (positive biHeight, row 0 in the file
 * is the BOTTOM row of the image). This encoder writes negative biHeight to
 * request top-down layout (row 0 in the file = top of image). The decoder
 * handles both.
 *
 * ### RGBA ↔ BGRA
 *
 * Our PixelContainer stores RGBA. BMP stores BGRA. Encode swaps R↔B on write;
 * decode swaps R↔B on read.
 */
import {
  type PixelContainer,
  type ImageCodec,
  createPixelContainer,
} from "@coding-adventures/pixel-container";

export { type PixelContainer, type ImageCodec };

// ============================================================================
// BmpCodec
// ============================================================================

/** BMP image encoder and decoder implementing the ImageCodec interface. */
export class BmpCodec implements ImageCodec {
  readonly mimeType = "image/bmp";

  encode(pixels: PixelContainer): Uint8Array {
    return encodeBmp(pixels);
  }

  decode(bytes: Uint8Array): PixelContainer {
    return decodeBmp(bytes);
  }
}

// ============================================================================
// Convenience functions
// ============================================================================

/**
 * Encode a PixelContainer to 32-bit BMP bytes.
 *
 * @example
 * import { createPixelContainer, setPixel } from "@coding-adventures/pixel-container";
 * import { encodeBmp } from "@coding-adventures/image-codec-bmp";
 *
 * const c = createPixelContainer(2, 1);
 * setPixel(c, 0, 0, 255, 0, 0, 255);  // red
 * setPixel(c, 1, 0, 0, 0, 255, 255);  // blue
 * const bmp = encodeBmp(c);
 * // bmp[0] === 0x42 ('B'), bmp[1] === 0x4D ('M')
 */
export function encodeBmp(pixels: PixelContainer): Uint8Array {
  const { width, height } = pixels;
  const pixelBytes = width * height * 4;
  const fileSize = 54 + pixelBytes;
  const buf = new Uint8Array(fileSize);
  const view = new DataView(buf.buffer);

  // BITMAPFILEHEADER
  buf[0] = 0x42; buf[1] = 0x4d; // "BM"
  view.setUint32(2, fileSize, true);   // bfSize
  view.setUint32(6, 0, true);          // bfReserved1+2
  view.setUint32(10, 54, true);        // bfOffBits

  // BITMAPINFOHEADER
  view.setUint32(14, 40, true);        // biSize
  view.setInt32(18, width, true);      // biWidth
  view.setInt32(22, -height, true);    // biHeight (negative = top-down)
  view.setUint16(26, 1, true);         // biPlanes
  view.setUint16(28, 32, true);        // biBitCount
  view.setUint32(30, 0, true);         // biCompression (BI_RGB)
  view.setUint32(34, pixelBytes, true);// biSizeImage
  view.setUint32(38, 0, true);         // biXPelsPerMeter
  view.setUint32(42, 0, true);         // biYPelsPerMeter
  view.setUint32(46, 0, true);         // biClrUsed
  view.setUint32(50, 0, true);         // biClrImportant

  // Pixel data: RGBA → BGRA
  let off = 54;
  for (let i = 0; i < pixels.data.length; i += 4) {
    buf[off++] = pixels.data[i + 2]; // B
    buf[off++] = pixels.data[i + 1]; // G
    buf[off++] = pixels.data[i];     // R
    buf[off++] = pixels.data[i + 3]; // A
  }

  return buf;
}

/**
 * Decode BMP bytes into a PixelContainer.
 *
 * Only 32-bit BGRA BI_RGB files are supported. Throws on invalid input.
 *
 * @example
 * import { decodeBmp } from "@coding-adventures/image-codec-bmp";
 * const pixels = decodeBmp(bmpBytes);
 * // pixels.width, pixels.height, pixels.data
 */
export function decodeBmp(bytes: Uint8Array): PixelContainer {
  if (bytes.length < 54) throw new Error("BMP: file too short");
  if (bytes[0] !== 0x42 || bytes[1] !== 0x4d) throw new Error("BMP: invalid magic");

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const pixelOffset = view.getUint32(10, true);
  if (pixelOffset < 54) throw new Error("BMP: pixel offset is before end of header");

  const biWidth  = view.getInt32(18, true);
  const biHeight = view.getInt32(22, true);
  if (biWidth <= 0) throw new Error("BMP: invalid width");
  if (biHeight === -2147483648) throw new Error("BMP: invalid height");

  const width    = biWidth;
  const height   = Math.abs(biHeight);
  const topDown  = biHeight < 0;
  if (height === 0) throw new Error("BMP: invalid height");

  const bitCount    = view.getUint16(28, true);
  const compression = view.getUint32(30, true);
  if (bitCount !== 32) throw new Error(`BMP: unsupported bit depth ${bitCount}, only 32 supported`);
  if (compression !== 0) throw new Error(`BMP: unsupported compression ${compression}`);

  const MAX_DIMENSION = 16384;
  if (width > MAX_DIMENSION || height > MAX_DIMENSION) {
    throw new Error(`BMP: dimensions ${width}×${height} exceed maximum ${MAX_DIMENSION}`);
  }
  const totalPixels = width * height;
  if (totalPixels > MAX_DIMENSION * MAX_DIMENSION) {
    throw new Error(`BMP: image too large`);
  }

  const pixelBytes = width * height * 4;
  const pixelEnd   = pixelOffset + pixelBytes;
  if (bytes.length < pixelEnd) throw new Error("BMP: pixel data truncated");

  const container = createPixelContainer(width, height);

  for (let row = 0; row < height; row++) {
    const destRow = topDown ? row : height - 1 - row;
    for (let col = 0; col < width; col++) {
      const fileIdx = pixelOffset + (row * width + col) * 4;
      const b = bytes[fileIdx];
      const g = bytes[fileIdx + 1];
      const r = bytes[fileIdx + 2];
      const a = bytes[fileIdx + 3];
      const destIdx = (destRow * width + col) * 4;
      container.data[destIdx]     = r;
      container.data[destIdx + 1] = g;
      container.data[destIdx + 2] = b;
      container.data[destIdx + 3] = a;
    }
  }

  return container;
}
