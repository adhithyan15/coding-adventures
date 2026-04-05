/**
 * @coding-adventures/image-codec-ppm
 *
 * IC02: PPM P6 image encoder and decoder.
 *
 * ## File Format
 *
 * PPM P6 (Portable Pixmap, binary) is deliberately minimal:
 *
 *   P6\n
 *   <width> <height>\n
 *   255\n
 *   <width * height * 3 raw bytes: R G B per pixel, row-major>
 *
 * There is no compression, no metadata, no alpha channel, and no padding.
 * Three bytes per pixel.
 *
 * ## Alpha handling
 *
 * PPM has no alpha channel:
 * - On encode: the alpha byte is dropped for each pixel.
 * - On decode: every decoded pixel gets A = 255 (fully opaque).
 *
 * ## Comments
 *
 * The PPM spec allows '#'-prefixed comment lines anywhere in the ASCII header.
 * This decoder skips them. The encoder never writes comments.
 *
 * ## Interoperability
 *
 * Files produced by this encoder are accepted by ImageMagick, ffmpeg, and any
 * Netpbm tool. Files produced by those tools can be decoded here.
 */
import {
  type PixelContainer,
  type ImageCodec,
  createPixelContainer,
} from "@coding-adventures/pixel-container";

export { type PixelContainer, type ImageCodec };

// ============================================================================
// PpmCodec
// ============================================================================

/** PPM P6 image encoder and decoder implementing the ImageCodec interface. */
export class PpmCodec implements ImageCodec {
  readonly mimeType = "image/x-portable-pixmap";

  encode(pixels: PixelContainer): Uint8Array {
    return encodePpm(pixels);
  }

  decode(bytes: Uint8Array): PixelContainer {
    return decodePpm(bytes);
  }
}

// ============================================================================
// Convenience functions
// ============================================================================

/**
 * Encode a PixelContainer to PPM P6 bytes.
 *
 * Alpha is dropped (PPM has no alpha channel).
 *
 * @example
 * import { createPixelContainer, setPixel } from "@coding-adventures/pixel-container";
 * import { encodePpm } from "@coding-adventures/image-codec-ppm";
 *
 * const c = createPixelContainer(2, 1);
 * setPixel(c, 0, 0, 255, 0, 0, 255);  // red
 * const ppm = encodePpm(c);
 * // ppm starts with b"P6\n"
 */
export function encodePpm(pixels: PixelContainer): Uint8Array {
  const { width, height } = pixels;
  const header = `P6\n${width} ${height}\n255\n`;
  const headerBytes = new TextEncoder().encode(header);
  const pixelBytes = width * height * 3;

  const out = new Uint8Array(headerBytes.length + pixelBytes);
  out.set(headerBytes, 0);

  let off = headerBytes.length;
  for (let i = 0; i < pixels.data.length; i += 4) {
    out[off++] = pixels.data[i];     // R
    out[off++] = pixels.data[i + 1]; // G
    out[off++] = pixels.data[i + 2]; // B
    // alpha dropped
  }

  return out;
}

/**
 * Decode PPM P6 bytes into a PixelContainer.
 *
 * Decoded pixels have A = 255 (PPM has no alpha channel).
 * Throws on invalid input or unsupported max value.
 *
 * @example
 * import { decodePpm } from "@coding-adventures/image-codec-ppm";
 * const pixels = decodePpm(ppmBytes);
 */
export function decodePpm(bytes: Uint8Array): PixelContainer {
  let pos = 0;

  // Read the magic token: must be "P6".
  const magic = readToken(bytes, { pos: 0 });
  pos = magic.pos;
  if (magic.token !== "P6") throw new Error("PPM: invalid magic, expected P6");

  skipWhitespaceAndComments(bytes, { pos });
  const wRes = readInt(bytes, { pos });
  pos = wRes.pos;
  if (wRes.value === null) throw new Error("PPM: invalid dimensions");
  const width = wRes.value;

  skipWhitespaceAndComments(bytes, { pos });
  const hRes = readInt(bytes, { pos });
  pos = hRes.pos;
  if (hRes.value === null) throw new Error("PPM: invalid dimensions");
  const height = hRes.value;

  const MAX_DIMENSION = 16384;
  if (width <= 0 || height <= 0) throw new Error(`PPM: invalid dimensions (${width}×${height})`);
  if (width > MAX_DIMENSION || height > MAX_DIMENSION) {
    throw new Error(`PPM: dimensions ${width}×${height} exceed maximum ${MAX_DIMENSION}`);
  }

  skipWhitespaceAndComments(bytes, { pos });
  const maxRes = readInt(bytes, { pos });
  pos = maxRes.pos;
  if (maxRes.value === null) throw new Error("PPM: invalid max value");
  if (maxRes.value !== 255) throw new Error(`PPM: unsupported max value ${maxRes.value}, only 255 supported`);

  // Skip exactly one whitespace byte after the max value (spec requirement).
  if (pos >= bytes.length) throw new Error("PPM: pixel data truncated");
  pos += 1;

  const pixelCount = width * height;
  const needed = pixelCount * 3;
  if (bytes.length - pos < needed) throw new Error("PPM: pixel data truncated");

  const container = createPixelContainer(width, height);
  for (let p = 0; p < pixelCount; p++) {
    const r = bytes[pos++];
    const g = bytes[pos++];
    const b = bytes[pos++];
    const base = p * 4;
    container.data[base]     = r;
    container.data[base + 1] = g;
    container.data[base + 2] = b;
    container.data[base + 3] = 255; // alpha = opaque
  }

  return container;
}

// ============================================================================
// Parser helpers
// ============================================================================

/** Mutable position cursor, passed by reference via object. */
interface Cursor { pos: number; }

/** Skip ASCII whitespace and '#'-prefixed comment lines. */
function skipWhitespaceAndComments(bytes: Uint8Array, cur: Cursor): void {
  for (;;) {
    while (cur.pos < bytes.length && isAsciiWhitespace(bytes[cur.pos])) cur.pos++;
    if (cur.pos < bytes.length && bytes[cur.pos] === 0x23 /* '#' */) {
      while (cur.pos < bytes.length && bytes[cur.pos] !== 0x0a /* '\n' */) cur.pos++;
    } else {
      break;
    }
  }
}

function isAsciiWhitespace(b: number): boolean {
  return b === 0x20 || b === 0x09 || b === 0x0d || b === 0x0a;
}

/** Read a whitespace-delimited ASCII token. */
function readToken(bytes: Uint8Array, cur: Cursor): { token: string | null; pos: number } {
  skipWhitespaceAndComments(bytes, cur);
  if (cur.pos >= bytes.length) return { token: null, pos: cur.pos };
  const start = cur.pos;
  const MAX_TOKEN_LEN = 20;
  while (cur.pos < bytes.length && !isAsciiWhitespace(bytes[cur.pos])) {
    cur.pos++;
    if (cur.pos - start > MAX_TOKEN_LEN) throw new Error('PPM: header token too long');
  }
  const token = new TextDecoder().decode(bytes.slice(start, cur.pos));
  return { token, pos: cur.pos };
}

/** Read a decimal integer token. */
function readInt(bytes: Uint8Array, cur: Cursor): { value: number | null; pos: number } {
  const r = readToken(bytes, cur);
  if (r.token === null) return { value: null, pos: r.pos };
  const n = parseInt(r.token, 10);
  if (!Number.isFinite(n)) return { value: null, pos: r.pos };
  return { value: n, pos: r.pos };
}
