/**
 * @coding-adventures/image-codec-qoi
 *
 * IC03: QOI (Quite OK Image) encoder and decoder.
 *
 * ## The QOI Format
 *
 * QOI achieves good compression ratios (often better than PNG for natural
 * images) through six simple operations applied to a stream of RGBA pixels.
 * The encoder scans pixels left-to-right, top-to-bottom and emits the
 * smallest applicable operation for each pixel.
 *
 * ### File layout
 *
 *   Bytes 0–3:   Magic "qoif"
 *   Bytes 4–7:   Width  (u32 big-endian)
 *   Bytes 8–11:  Height (u32 big-endian)
 *   Byte  12:    Channels (3=RGB, 4=RGBA) — informational only; we always write 4
 *   Byte  13:    Colorspace (0=sRGB, 1=linear) — informational only; we always write 0
 *   Bytes 14+:   Encoded pixel stream
 *   Last 8 bytes: End marker 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x01
 *
 * ### The six operations
 *
 * | Tag bytes     | Operation  | Description                                    |
 * |---------------|------------|------------------------------------------------|
 * | 0xFE          | OP_RGB     | 3 explicit RGB bytes; alpha unchanged          |
 * | 0xFF          | OP_RGBA    | 4 explicit RGBA bytes                          |
 * | 00xxxxxx      | OP_INDEX   | Look up pixel in 64-slot hash table            |
 * | 01rrggbb      | OP_DIFF    | Small deltas: r,g,b each in [-2,1] (+2 bias)   |
 * | 10gggggg + next byte | OP_LUMA | dg in [-32,31]; dr-dg, db-dg in [-8,7]   |
 * | 11rrrrrr      | OP_RUN     | Repeat previous pixel 1–62 times (bias -1)    |
 *
 * ### Hash function
 *
 * The 64-slot circular hash table is indexed by:
 *   (r * 3 + g * 5 + b * 7 + a * 11) % 64
 *
 * On each decoded pixel, the table slot at hash(pixel) is updated to that
 * pixel. OP_INDEX replays a slot without updating it (the slot already holds
 * the right value).
 */
import {
  type PixelContainer,
  type ImageCodec,
  createPixelContainer,
} from "@coding-adventures/pixel-container";

export { type PixelContainer, type ImageCodec };

const MAGIC = new Uint8Array([0x71, 0x6f, 0x69, 0x66]); // "qoif"
const END_MARKER = new Uint8Array([0, 0, 0, 0, 0, 0, 0, 1]);

const OP_RGB  = 0xfe;
const OP_RGBA = 0xff;
const TAG_INDEX = 0b00;
const TAG_DIFF  = 0b01;
const TAG_LUMA  = 0b10;
// TAG_RUN = 0b11 (the else branch)

function qoiHash(r: number, g: number, b: number, a: number): number {
  return (r * 3 + g * 5 + b * 7 + a * 11) % 64;
}

// ============================================================================
// QoiCodec
// ============================================================================

/** QOI image encoder and decoder implementing the ImageCodec interface. */
export class QoiCodec implements ImageCodec {
  readonly mimeType = "image/qoi";

  encode(pixels: PixelContainer): Uint8Array {
    return encodeQoi(pixels);
  }

  decode(bytes: Uint8Array): PixelContainer {
    return decodeQoi(bytes);
  }
}

// ============================================================================
// Convenience functions
// ============================================================================

/**
 * Encode a PixelContainer to QOI bytes.
 *
 * @example
 * import { createPixelContainer, fillPixels } from "@coding-adventures/pixel-container";
 * import { encodeQoi } from "@coding-adventures/image-codec-qoi";
 *
 * const c = createPixelContainer(4, 4);
 * fillPixels(c, 100, 150, 200, 255);
 * const qoi = encodeQoi(c);
 * // qoi[0..4] === "qoif"
 */
export function encodeQoi(pixels: PixelContainer): Uint8Array {
  const { width, height } = pixels;
  // Worst case: every pixel is OP_RGBA (5 bytes) + 14-byte header + 8-byte end marker.
  const out: number[] = [];

  // Header
  for (const b of MAGIC) out.push(b);
  pushU32BE(out, width);
  pushU32BE(out, height);
  out.push(4); // channels = RGBA
  out.push(0); // colorspace = sRGB

  const hashTable: Array<[number, number, number, number]> = new Array(64).fill([0, 0, 0, 0]);
  let prev: [number, number, number, number] = [0, 0, 0, 255]; // QOI initial state: opaque black
  let run = 0;

  const totalPixels = width * height;

  for (let p = 0; p < totalPixels; p++) {
    const base = p * 4;
    const r = pixels.data[base];
    const g = pixels.data[base + 1];
    const b = pixels.data[base + 2];
    const a = pixels.data[base + 3];

    if (r === prev[0] && g === prev[1] && b === prev[2] && a === prev[3]) {
      // OP_RUN: same pixel as before — accumulate run.
      run++;
      if (run === 62 || p === totalPixels - 1) {
        out.push(0xc0 | (run - 1)); // TAG_RUN (11xxxxxx), bias -1
        run = 0;
      }
      continue;
    }

    if (run > 0) {
      out.push(0xc0 | (run - 1));
      run = 0;
    }

    const hash = qoiHash(r, g, b, a);
    const [hr, hg, hb, ha] = hashTable[hash];

    if (hr === r && hg === g && hb === b && ha === a) {
      // OP_INDEX: pixel is in the hash table.
      out.push(TAG_INDEX << 6 | hash);
    } else {
      hashTable[hash] = [r, g, b, a];

      if (a === prev[3]) {
        // Alpha unchanged — try OP_DIFF or OP_LUMA, fall back to OP_RGB.
        const dr = (r - prev[0]) | 0;
        const dg = (g - prev[1]) | 0;
        const db = (b - prev[2]) | 0;
        const wdr = wrap(dr);
        const wdg = wrap(dg);
        const wdb = wrap(db);

        if (wdr >= -2 && wdr <= 1 && wdg >= -2 && wdg <= 1 && wdb >= -2 && wdb <= 1) {
          // OP_DIFF: small deltas fit in 2 bits each, biased by +2.
          out.push((TAG_DIFF << 6) | ((wdr + 2) << 4) | ((wdg + 2) << 2) | (wdb + 2));
        } else {
          const drdg = wdr - wdg;
          const dbdg = wdb - wdg;
          if (wdg >= -32 && wdg <= 31 && drdg >= -8 && drdg <= 7 && dbdg >= -8 && dbdg <= 7) {
            // OP_LUMA: dg fits in 6 bits, dr-dg and db-dg fit in 4 bits.
            out.push((TAG_LUMA << 6) | (wdg + 32));
            out.push(((drdg + 8) << 4) | (dbdg + 8));
          } else {
            // OP_RGB: three explicit bytes.
            out.push(OP_RGB);
            out.push(r); out.push(g); out.push(b);
          }
        }
      } else {
        // Alpha changed: must use OP_RGBA.
        out.push(OP_RGBA);
        out.push(r); out.push(g); out.push(b); out.push(a);
      }
    }

    prev = [r, g, b, a];
  }

  // End marker
  for (const b of END_MARKER) out.push(b);

  return new Uint8Array(out);
}

/**
 * Decode QOI bytes into a PixelContainer.
 *
 * Throws on invalid input.
 *
 * @example
 * import { decodeQoi } from "@coding-adventures/image-codec-qoi";
 * const pixels = decodeQoi(qoiBytes);
 */
export function decodeQoi(bytes: Uint8Array): PixelContainer {
  if (bytes.length < 22) throw new Error("QOI: file too short");

  for (let i = 0; i < 4; i++) {
    if (bytes[i] !== MAGIC[i]) throw new Error("QOI: invalid magic");
  }

  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const width  = view.getUint32(4, false);  // big-endian
  const height = view.getUint32(8, false);
  if (width === 0 || height === 0) throw new Error("QOI: invalid dimensions");

  const MAX_DIMENSION = 16384;
  if (width > MAX_DIMENSION || height > MAX_DIMENSION) {
    throw new Error(`QOI: dimensions ${width}×${height} exceed maximum ${MAX_DIMENSION}`);
  }

  const totalPixels = width * height;

  // Pre-flight: reject if payload cannot possibly hold this many pixels.
  // One QOI_OP_RUN byte covers at most 62 pixels. So minimum payload bytes =
  // ceil(totalPixels / 62). If available bytes < that, it's definitely truncated.
  const payloadLen = bytes.length - 22;
  if (totalPixels > payloadLen * 62) throw new Error("QOI: pixel data truncated");

  const container = createPixelContainer(width, height);
  const hashTable: Array<[number, number, number, number]> = new Array(64).fill([0, 0, 0, 0]);
  let prev: [number, number, number, number] = [0, 0, 0, 255];

  let pos = 14;
  let pixelsWritten = 0;

  while (pixelsWritten < totalPixels) {
    if (pos >= bytes.length) throw new Error("QOI: unexpected end of data");
    const tag = bytes[pos++];

    let r: number, g: number, b: number, a: number;

    if (tag === OP_RGB) {
      if (pos + 3 > bytes.length) throw new Error("QOI: unexpected end of data");
      r = bytes[pos++]; g = bytes[pos++]; b = bytes[pos++]; a = prev[3];
    } else if (tag === OP_RGBA) {
      if (pos + 4 > bytes.length) throw new Error("QOI: unexpected end of data");
      r = bytes[pos++]; g = bytes[pos++]; b = bytes[pos++]; a = bytes[pos++];
    } else {
      const tagBits = tag >> 6;
      if (tagBits === TAG_INDEX) {
        const idx = tag & 0x3f;
        [r, g, b, a] = hashTable[idx];
        const base = pixelsWritten * 4;
        container.data[base]     = r;
        container.data[base + 1] = g;
        container.data[base + 2] = b;
        container.data[base + 3] = a;
        pixelsWritten++;
        prev = [r, g, b, a];
        continue; // do NOT update hash table — slot already correct
      } else if (tagBits === TAG_DIFF) {
        const dr = ((tag >> 4) & 0x3) - 2;
        const dg = ((tag >> 2) & 0x3) - 2;
        const db = ((tag >> 0) & 0x3) - 2;
        r = (prev[0] + dr) & 0xff;
        g = (prev[1] + dg) & 0xff;
        b = (prev[2] + db) & 0xff;
        a = prev[3];
      } else if (tagBits === TAG_LUMA) {
        if (pos >= bytes.length) throw new Error("QOI: unexpected end of data");
        const next = bytes[pos++];
        const dg   = (tag & 0x3f) - 32;
        const drdg = ((next >> 4) & 0xf) - 8;
        const dbdg = ((next >> 0) & 0xf) - 8;
        const dr   = drdg + dg;
        const db2  = dbdg + dg;
        r = (prev[0] + dr) & 0xff;
        g = (prev[1] + dg) & 0xff;
        b = (prev[2] + db2) & 0xff;
        a = prev[3];
      } else {
        // OP_RUN
        const runLen = (tag & 0x3f) + 1;
        const actual = Math.min(runLen, totalPixels - pixelsWritten);
        [r, g, b, a] = prev;
        for (let i = 0; i < actual; i++) {
          const base = pixelsWritten * 4;
          container.data[base]     = r;
          container.data[base + 1] = g;
          container.data[base + 2] = b;
          container.data[base + 3] = a;
          pixelsWritten++;
        }
        // Note: RUN does not update hash table.
        continue;
      }
    }

    hashTable[qoiHash(r, g, b, a)] = [r, g, b, a];
    const base = pixelsWritten * 4;
    container.data[base]     = r;
    container.data[base + 1] = g;
    container.data[base + 2] = b;
    container.data[base + 3] = a;
    pixelsWritten++;
    prev = [r, g, b, a];
  }

  return container;
}

// ============================================================================
// Helpers
// ============================================================================

function pushU32BE(out: number[], n: number): void {
  out.push((n >>> 24) & 0xff);
  out.push((n >>> 16) & 0xff);
  out.push((n >>> 8)  & 0xff);
  out.push((n >>> 0)  & 0xff);
}

/**
 * Wrap an RGB delta into the signed [-128, 127] range.
 * Channel arithmetic is modular u8, so deltas near ±128 wrap around.
 * e.g. prev=1, curr=255 → raw delta = 254, but the "real" delta is -2.
 */
function wrap(delta: number): number {
  // Bring into [-128, 127] using signed 8-bit interpretation.
  const d = ((delta & 0xff) + 128) & 0xff;
  return d - 128;
}
