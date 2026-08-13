/**
 * @coding-adventures/image-codec-png
 *
 * IC18: PNG image encoder and decoder.
 *
 * ## Why PNG is three formats in a trench coat
 *
 * BMP (`image-codec-bmp`) is a header followed by pixels, and that is the whole
 * story. PNG is the opposite: almost nothing about a PNG file is pixels. Reading
 * or writing one means peeling three layers, and each layer is a separate idea
 * that has to be right before the next one means anything.
 *
 * ```
 *   PNG file
 *     |
 *     +-- signature, then a sequence of CHUNKS       <- layer 1: framing
 *     |     each: length, 4-letter type, data, CRC-32
 *     |
 *     +-- the IDAT chunks' data, concatenated
 *     |     is one ZLIB stream                        <- layer 2: RFC 1950
 *     |     header, DEFLATE stream, Adler-32
 *     |
 *     +-- which decompresses to FILTERED scanlines    <- layer 3: RFC 2083
 *           each row: 1 filter byte, then the row's bytes,
 *           each byte predicted from its neighbours
 * ```
 *
 * The compression itself -- the hardest part, RFC 1951 DEFLATE -- is not in this
 * file at all. It lives in `@coding-adventures/zip`, which needed the identical
 * bit stream for ZIP entries and exports it as `rawDeflate`/`rawInflate`. The
 * CRC-32 comes from there too, because PNG chunks and ZIP entries use the same
 * polynomial. So this package is the two layers DEFLATE is wrapped in, plus the
 * filtering.
 *
 * ## Layer 1: chunks
 *
 * Every PNG begins with the same eight bytes, chosen with unusual care:
 *
 * ```
 *   89  P  N  G  \r \n 1A \n
 * ```
 *
 * The high bit in `0x89` catches a transfer that stripped the eighth bit. The
 * `\r\n` catches a transfer that "helpfully" converted line endings. The `1A` is
 * DOS end-of-file, so `TYPE image.png` stops there instead of spraying binary at
 * a terminal. Every one of those is a scar from a real bug.
 *
 * After that, chunks all the way down. A chunk is:
 *
 * ```
 *   length (u32 BE, of DATA only)  type (4 ASCII bytes)  data  CRC-32 (u32 BE)
 * ```
 *
 * and the CRC covers the type AND the data, but not the length. Three chunks
 * matter here: `IHDR` (the header, first), `IDAT` (the pixels, possibly split
 * across several), and `IEND` (the terminator, last, always empty).
 *
 * The chunk type's letter CASE carries meaning -- bit 5 of each byte is a flag.
 * An uppercase first letter means "critical": a decoder that does not understand
 * this chunk must refuse the file rather than skip it. Lowercase means ancillary
 * and safely ignorable, which is why this decoder can walk past `gAMA`, `tEXt`
 * and friends without knowing anything about them, but stops at an unknown
 * uppercase type instead of guessing.
 *
 * ## Layer 2: the zlib wrapper
 *
 * PNG does not embed raw DEFLATE. It embeds a zlib stream (RFC 1950), which is
 * DEFLATE plus two bytes in front and an Adler-32 checksum behind:
 *
 * ```
 *   CMF  FLG   <deflate stream>   Adler-32 (u32 BE)
 *   0x78 0x9C
 * ```
 *
 * `CMF` says "method 8 (deflate), 32 KB window". `FLG` carries a compression
 * hint and, in its low five bits, whatever value makes `CMF * 256 + FLG` a
 * multiple of 31 -- a checksum so weak it is really just a "this is probably
 * zlib" marker.
 *
 * Adler-32 is not CRC-32 and the two are not interchangeable: it is a pair of
 * running sums mod 65521, far cheaper and far weaker, chosen because the CRC on
 * the enclosing chunk already does the real integrity work.
 *
 * ## Layer 3: filtering, which is where the compression actually comes from
 *
 * This is the part that makes PNG better than a zipped BMP, and it is one idea:
 * **a pixel usually resembles the pixel to its left and the pixel above it.** So
 * instead of storing pixels, store how much each one DIFFERS from a prediction.
 * A smooth gradient becomes a long run of zeroes, and DEFLATE eats runs of
 * zeroes for breakfast.
 *
 * Each row picks its own predictor and says which in a leading byte:
 *
 * ```
 *   0 None    the byte itself
 *   1 Sub     byte - byte to the left            (horizontal runs)
 *   2 Up      byte - byte above                  (vertical runs)
 *   3 Average byte - floor((left + above) / 2)   (smooth gradients)
 *   4 Paeth   byte - whichever of left/above/upper-left is closest to
 *             (left + above - upper-left)        (edges and diagonals)
 * ```
 *
 * Two details that are easy to get wrong and produce corrupt-looking output
 * rather than an error:
 *
 * 1. **Filters work on BYTES, not pixels**, and "the byte to the left" means the
 *    byte one whole pixel back -- byte `i - bpp`, not byte `i - 1`. Off the left
 *    edge, it is zero.
 * 2. **Filtering is applied to the row as it will be stored, and undone against
 *    the row as already reconstructed.** The encoder subtracts from original
 *    bytes; the decoder adds to bytes it has already unfiltered. Getting the
 *    direction wrong on one filter yields an image that looks like a smear.
 *
 * Choosing a filter per row uses the heuristic from the PNG spec itself: try all
 * five, sum the absolute values of the results read as signed bytes, keep the
 * smallest. It is a proxy for entropy, it is cheap, and it is what every real
 * encoder does.
 *
 * ## What this reads and writes
 *
 * Writes: 8-bit truecolour with alpha (colour type 6), one `IDAT`, no
 * interlacing -- the exact shape of a `PixelContainer`, so no information is
 * lost or invented.
 *
 * Reads: 8-bit greyscale, truecolour, greyscale+alpha and truecolour+alpha
 * (types 0, 2, 4, 6), non-interlaced, with unknown ancillary chunks skipped.
 * Palette (type 3), 16-bit depths and Adam7 interlacing are refused by name
 * rather than half-supported.
 */
import {
  type PixelContainer,
  type ImageCodec,
  createPixelContainer,
} from "@coding-adventures/pixel-container";
import { crc32, rawDeflate, rawInflate } from "@coding-adventures/zip";

export { type PixelContainer, type ImageCodec };

// ============================================================================
// Limits
// ============================================================================

/**
 * Largest edge this codec will decode.
 *
 * A PNG header is eight bytes of attacker-controlled integers claiming a size,
 * and `width * height * 4` is allocated on the strength of it. The same ceiling
 * as `image-codec-bmp`, for the same reason.
 */
const MAX_DIMENSION = 16384;

/** PNG's fixed opening bytes. See the header comment for why each one is there. */
const SIGNATURE = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

/** Adler-32's modulus: the largest prime below 65536. */
const ADLER_MOD = 65521;

// ============================================================================
// Adler-32
// ============================================================================

/**
 * The zlib wrapper's checksum (RFC 1950 section 9).
 *
 * Two running sums: `a` accumulates the bytes, `b` accumulates `a`. Because `b`
 * grows with every intermediate value of `a`, reordering the input changes it —
 * which a plain sum would not catch.
 *
 * @example
 * adler32(new TextEncoder().encode("Wikipedia")) === 0x11E60398
 */
export function adler32(data: Uint8Array): number {
  let a = 1;
  let b = 0;
  // Chunked so the sums cannot leave the range where JavaScript numbers are
  // exact before the modulo brings them back down. 5552 is the largest block
  // for which that is guaranteed, and is the value zlib itself uses.
  for (let start = 0; start < data.length; start += 5552) {
    const end = Math.min(start + 5552, data.length);
    for (let i = start; i < end; i++) {
      a += data[i]!;
      b += a;
    }
    a %= ADLER_MOD;
    b %= ADLER_MOD;
  }
  return ((b << 16) | a) >>> 0;
}

// ============================================================================
// Filtering
// ============================================================================

/** The five per-row predictors, in the order RFC 2083 numbers them. */
const FILTER_NONE = 0;
const FILTER_SUB = 1;
const FILTER_UP = 2;
const FILTER_AVERAGE = 3;
const FILTER_PAETH = 4;

/**
 * The Paeth predictor (RFC 2083 section 6.6).
 *
 * Given the byte to the left (`a`), the byte above (`b`) and the byte above-left
 * (`c`), guess that the current byte continues whatever local gradient those
 * three describe. The initial estimate `a + b - c` is the value that would make
 * the four bytes a perfect parallelogram; the function then returns whichever of
 * the three ACTUAL neighbours is closest to it, so the prediction is always a
 * real neighbouring value rather than an interpolation.
 *
 * That is what makes it good at edges: at a boundary the estimate lands nearest
 * whichever side of the edge the pixel is on.
 */
function paeth(a: number, b: number, c: number): number {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  // Ties break toward `a`, then `b`. The order is normative, not arbitrary:
  // an encoder and decoder that break ties differently produce different images.
  if (pa <= pb && pa <= pc) return a;
  if (pb <= pc) return b;
  return c;
}

/**
 * Apply one filter to a row, writing the filtered bytes into `out`.
 *
 * `raw` is the row as it should appear once decoded; `prior` is the row above,
 * already in raw form (all zeroes for the first row).
 */
function applyFilter(
  type: number,
  raw: Uint8Array,
  prior: Uint8Array,
  bpp: number,
  out: Uint8Array,
): void {
  const n = raw.length;
  for (let i = 0; i < n; i++) {
    const x = raw[i]!;
    const a = i >= bpp ? raw[i - bpp]! : 0;
    const b = prior[i]!;
    const c = i >= bpp ? prior[i - bpp]! : 0;
    let value: number;
    switch (type) {
      case FILTER_SUB: value = x - a; break;
      case FILTER_UP: value = x - b; break;
      case FILTER_AVERAGE: value = x - ((a + b) >> 1); break;
      case FILTER_PAETH: value = x - paeth(a, b, c); break;
      default: value = x; break;
    }
    out[i] = value & 0xff;
  }
}

/**
 * Undo one filter, in place, against a row whose predecessors are already raw.
 *
 * The asymmetry with `applyFilter` is the crux of the format: the encoder
 * predicts from ORIGINAL bytes, the decoder from bytes it has ALREADY restored.
 * Both see the same neighbours, but only because the decoder works strictly
 * left to right and top to bottom.
 */
function undoFilter(type: number, row: Uint8Array, prior: Uint8Array, bpp: number): void {
  const n = row.length;
  switch (type) {
    case FILTER_NONE:
      break;
    case FILTER_SUB:
      for (let i = bpp; i < n; i++) row[i] = (row[i]! + row[i - bpp]!) & 0xff;
      break;
    case FILTER_UP:
      for (let i = 0; i < n; i++) row[i] = (row[i]! + prior[i]!) & 0xff;
      break;
    case FILTER_AVERAGE:
      for (let i = 0; i < n; i++) {
        const a = i >= bpp ? row[i - bpp]! : 0;
        row[i] = (row[i]! + ((a + prior[i]!) >> 1)) & 0xff;
      }
      break;
    case FILTER_PAETH:
      for (let i = 0; i < n; i++) {
        const a = i >= bpp ? row[i - bpp]! : 0;
        const c = i >= bpp ? prior[i - bpp]! : 0;
        row[i] = (row[i]! + paeth(a, prior[i]!, c)) & 0xff;
      }
      break;
    default:
      throw new Error(`PNG: unknown filter type ${type}`);
  }
}

/**
 * Pick a filter for one row using the PNG spec's own heuristic.
 *
 * Sum the filtered bytes read as SIGNED values and keep the smallest total. The
 * signed reading is the point: a filtered byte of 255 means -1, a tiny
 * correction, and treating it as 255 would make the best filter look like the
 * worst. It is a stand-in for "which of these compresses best", chosen because
 * actually running DEFLATE five times per row would cost far more than it saves.
 */
function chooseFilter(
  raw: Uint8Array,
  prior: Uint8Array,
  bpp: number,
  scratch: Uint8Array,
  best: Uint8Array,
): number {
  let bestType = FILTER_NONE;
  let bestScore = Infinity;
  for (const type of [FILTER_NONE, FILTER_SUB, FILTER_UP, FILTER_AVERAGE, FILTER_PAETH]) {
    applyFilter(type, raw, prior, bpp, scratch);
    let score = 0;
    for (let i = 0; i < scratch.length; i++) {
      const v = scratch[i]!;
      score += v < 128 ? v : 256 - v;
    }
    if (score < bestScore) {
      bestScore = score;
      bestType = type;
      best.set(scratch);
    }
  }
  return bestType;
}

// ============================================================================
// Chunk writing
// ============================================================================

function u32be(value: number): number[] {
  const v = value >>> 0;
  return [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
}

/** Append one complete chunk: length, type, data, CRC over type+data. */
function pushChunk(out: number[], type: string, data: Uint8Array): void {
  out.push(...u32be(data.length));
  const typed = new Uint8Array(4 + data.length);
  for (let i = 0; i < 4; i++) typed[i] = type.charCodeAt(i);
  typed.set(data, 4);
  for (const byte of typed) out.push(byte);
  out.push(...u32be(crc32(typed)));
}

// ============================================================================
// PngCodec
// ============================================================================

/** PNG image encoder and decoder implementing the `ImageCodec` interface. */
export class PngCodec implements ImageCodec {
  readonly mimeType = "image/png";

  encode(pixels: PixelContainer): Uint8Array {
    return encodePng(pixels);
  }

  decode(bytes: Uint8Array): PixelContainer {
    return decodePng(bytes);
  }
}

// ============================================================================
// Convenience functions
// ============================================================================

/**
 * Encode a `PixelContainer` as an 8-bit RGBA PNG.
 *
 * Colour type 6 is chosen because it is exactly what a `PixelContainer` holds:
 * no channel is dropped, no palette is guessed at, and the round trip is
 * lossless by construction rather than by luck.
 *
 * @example
 * import { createPixelContainer, setPixel } from "@coding-adventures/pixel-container";
 * import { encodePng } from "@coding-adventures/image-codec-png";
 *
 * const c = createPixelContainer(2, 1);
 * setPixel(c, 0, 0, 255, 0, 0, 255);  // red
 * setPixel(c, 1, 0, 0, 0, 255, 255);  // blue
 * const png = encodePng(c);
 * // png[1] === 0x50 ('P'), png[2] === 0x4E ('N'), png[3] === 0x47 ('G')
 */
export function encodePng(pixels: PixelContainer): Uint8Array {
  const { width, height } = pixels;
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 0 || height < 0) {
    throw new Error("PNG: width and height must be non-negative integers");
  }
  if (width === 0 || height === 0) {
    throw new Error("PNG: an image must have at least one pixel in each dimension");
  }
  if (pixels.data.length !== width * height * 4) {
    throw new Error(
      `PNG: pixel data is ${pixels.data.length} bytes, expected ${width * height * 4}`,
    );
  }

  const out: number[] = [];
  for (const byte of SIGNATURE) out.push(byte);

  // IHDR: 13 bytes, and the order is fixed by the spec.
  const ihdr = new Uint8Array([
    ...u32be(width),
    ...u32be(height),
    8, // bit depth
    6, // colour type 6 = truecolour with alpha
    0, // compression method: deflate, the only one defined
    0, // filter method: the five adaptive filters, the only one defined
    0, // interlace: none
  ]);
  pushChunk(out, "IHDR", ihdr);

  // Filter every row, building the byte stream that will be compressed.
  const bpp = 4;
  const stride = width * bpp;
  const filtered = new Uint8Array(height * (stride + 1));
  // The row above the first row is defined to be all zero, which is what makes
  // the Up and Paeth filters usable on row 0 without a special case.
  const prior = new Uint8Array(stride);
  const scratch = new Uint8Array(stride);
  const best = new Uint8Array(stride);

  for (let y = 0; y < height; y++) {
    const raw = pixels.data.subarray(y * stride, (y + 1) * stride);
    const type = chooseFilter(raw, prior, bpp, scratch, best);
    const at = y * (stride + 1);
    filtered[at] = type;
    filtered.set(best, at + 1);
    prior.set(raw);
  }

  // The zlib wrapper: two header bytes, the DEFLATE stream, the Adler-32 of the
  // UNCOMPRESSED bytes.
  const deflated = rawDeflate(filtered);
  const idat = new Uint8Array(2 + deflated.length + 4);
  idat[0] = 0x78; // CMF: deflate, 32 KB window
  idat[1] = 0x9c; // FLG: default level, no preset dictionary, and (0x789C % 31) === 0
  idat.set(deflated, 2);
  idat.set(new Uint8Array(u32be(adler32(filtered))), 2 + deflated.length);
  pushChunk(out, "IDAT", idat);

  pushChunk(out, "IEND", new Uint8Array(0));

  return new Uint8Array(out);
}

/**
 * Decode PNG bytes into a `PixelContainer`.
 *
 * Accepts 8-bit greyscale, truecolour, greyscale+alpha and truecolour+alpha,
 * non-interlaced. Anything else is refused by name — a decoder that silently
 * mis-reads a palette image is worse than one that says it cannot read it.
 *
 * @example
 * const pixels = decodePng(pngBytes);
 * pixels.width; pixels.height; pixels.data;
 */
export function decodePng(bytes: Uint8Array): PixelContainer {
  if (bytes.length < SIGNATURE.length) throw new Error("PNG: file too short");
  for (let i = 0; i < SIGNATURE.length; i++) {
    if (bytes[i] !== SIGNATURE[i]) throw new Error("PNG: invalid signature");
  }

  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colourType = 0;
  let sawIHDR = false;
  let sawIEND = false;
  const idatParts: Uint8Array[] = [];

  let pos = SIGNATURE.length;
  while (pos < bytes.length) {
    if (pos + 8 > bytes.length) throw new Error("PNG: truncated chunk header");
    const length =
      ((bytes[pos]! << 24) | (bytes[pos + 1]! << 16) | (bytes[pos + 2]! << 8) | bytes[pos + 3]!) >>> 0;
    // Guard before using `length` in any arithmetic: the field is four
    // attacker-chosen bytes and can claim four gigabytes.
    if (length > bytes.length) throw new Error("PNG: chunk length exceeds file size");
    const typeStart = pos + 4;
    const dataStart = typeStart + 4;
    const dataEnd = dataStart + length;
    if (dataEnd + 4 > bytes.length) throw new Error("PNG: truncated chunk data");

    const type = String.fromCharCode(
      bytes[typeStart]!, bytes[typeStart + 1]!, bytes[typeStart + 2]!, bytes[typeStart + 3]!,
    );

    const declaredCRC =
      ((bytes[dataEnd]! << 24) | (bytes[dataEnd + 1]! << 16) | (bytes[dataEnd + 2]! << 8) | bytes[dataEnd + 3]!) >>> 0;
    const actualCRC = crc32(bytes.subarray(typeStart, dataEnd));
    if (declaredCRC !== actualCRC) {
      throw new Error(`PNG: CRC-32 mismatch in '${type}' chunk`);
    }

    const data = bytes.subarray(dataStart, dataEnd);

    if (type === "IHDR") {
      if (sawIHDR) throw new Error("PNG: more than one IHDR chunk");
      if (length !== 13) throw new Error(`PNG: IHDR must be 13 bytes, got ${length}`);
      width = ((data[0]! << 24) | (data[1]! << 16) | (data[2]! << 8) | data[3]!) >>> 0;
      height = ((data[4]! << 24) | (data[5]! << 16) | (data[6]! << 8) | data[7]!) >>> 0;
      bitDepth = data[8]!;
      colourType = data[9]!;
      const compression = data[10]!;
      const filterMethod = data[11]!;
      const interlace = data[12]!;

      if (width === 0 || height === 0) throw new Error("PNG: zero width or height");
      if (width > MAX_DIMENSION || height > MAX_DIMENSION) {
        throw new Error(`PNG: dimensions ${width}x${height} exceed maximum ${MAX_DIMENSION}`);
      }
      if (compression !== 0) throw new Error(`PNG: unsupported compression method ${compression}`);
      if (filterMethod !== 0) throw new Error(`PNG: unsupported filter method ${filterMethod}`);
      if (interlace !== 0) throw new Error("PNG: Adam7 interlacing is not supported");
      if (colourType === 3) throw new Error("PNG: palette images (colour type 3) are not supported");
      if (colourType !== 0 && colourType !== 2 && colourType !== 4 && colourType !== 6) {
        throw new Error(`PNG: unknown colour type ${colourType}`);
      }
      if (bitDepth !== 8) {
        throw new Error(`PNG: unsupported bit depth ${bitDepth}, only 8 is supported`);
      }
      sawIHDR = true;
    } else if (type === "IDAT") {
      if (!sawIHDR) throw new Error("PNG: IDAT before IHDR");
      idatParts.push(data);
    } else if (type === "IEND") {
      sawIEND = true;
      pos = dataEnd + 4;
      break;
    } else if ((bytes[typeStart]! & 0x20) === 0) {
      // Uppercase first letter: a CRITICAL chunk. The spec says a decoder that
      // does not understand one must refuse the file, because ignoring it would
      // mean showing the user something other than what the file describes.
      throw new Error(`PNG: unsupported critical chunk '${type}'`);
    }
    // Anything else is ancillary (lowercase) -- gAMA, pHYs, tEXt and so on --
    // and is skipped by design.

    pos = dataEnd + 4;
  }

  if (!sawIHDR) throw new Error("PNG: no IHDR chunk");
  if (!sawIEND) throw new Error("PNG: no IEND chunk");
  if (idatParts.length === 0) throw new Error("PNG: no IDAT chunk");

  // The IDATs are one zlib stream that happens to be split across chunks, so
  // they are joined BEFORE anything is parsed. A split may fall anywhere,
  // including mid-symbol.
  let zlibLength = 0;
  for (const part of idatParts) zlibLength += part.length;
  const zlib = new Uint8Array(zlibLength);
  {
    let at = 0;
    for (const part of idatParts) {
      zlib.set(part, at);
      at += part.length;
    }
  }

  if (zlib.length < 6) throw new Error("PNG: IDAT too short to be a zlib stream");
  const cmf = zlib[0]!;
  const flg = zlib[1]!;
  if ((cmf & 0x0f) !== 8) throw new Error(`PNG: zlib compression method ${cmf & 0x0f}, expected 8`);
  if (((cmf << 8) | flg) % 31 !== 0) throw new Error("PNG: corrupt zlib header");
  if ((flg & 0x20) !== 0) throw new Error("PNG: zlib preset dictionary is not supported");

  const channels = colourType === 0 ? 1 : colourType === 2 ? 3 : colourType === 4 ? 2 : 4;
  const stride = width * channels;
  const expected = height * (stride + 1);

  // The cap is the exact size the header promises, so a bomb inside IDAT is
  // stopped at the size this image could possibly need rather than at a
  // generic ceiling.
  const filtered = rawInflate(zlib.subarray(2, zlib.length - 4), expected);
  if (filtered.length !== expected) {
    throw new Error(`PNG: decompressed ${filtered.length} bytes, expected ${expected}`);
  }

  const declaredAdler =
    ((zlib[zlib.length - 4]! << 24) | (zlib[zlib.length - 3]! << 16) |
     (zlib[zlib.length - 2]! << 8) | zlib[zlib.length - 1]!) >>> 0;
  if (adler32(filtered) !== declaredAdler) throw new Error("PNG: Adler-32 mismatch");

  // Unfilter in place, row by row, each against the row already restored above.
  const bpp = channels; // 8-bit, so one byte per channel
  const container = createPixelContainer(width, height);
  const prior = new Uint8Array(stride);

  for (let y = 0; y < height; y++) {
    const at = y * (stride + 1);
    const filterType = filtered[at]!;
    const row = filtered.subarray(at + 1, at + 1 + stride);
    undoFilter(filterType, row, prior, bpp);

    // Widen whatever channel layout the file used into the RGBA the container
    // holds. Greyscale copies one value into R, G and B; a missing alpha is
    // opaque.
    const destRow = y * width * 4;
    for (let x = 0; x < width; x++) {
      const src = x * channels;
      const dest = destRow + x * 4;
      if (channels === 1) {
        const v = row[src]!;
        container.data[dest] = v;
        container.data[dest + 1] = v;
        container.data[dest + 2] = v;
        container.data[dest + 3] = 255;
      } else if (channels === 2) {
        const v = row[src]!;
        container.data[dest] = v;
        container.data[dest + 1] = v;
        container.data[dest + 2] = v;
        container.data[dest + 3] = row[src + 1]!;
      } else if (channels === 3) {
        container.data[dest] = row[src]!;
        container.data[dest + 1] = row[src + 1]!;
        container.data[dest + 2] = row[src + 2]!;
        container.data[dest + 3] = 255;
      } else {
        container.data[dest] = row[src]!;
        container.data[dest + 1] = row[src + 1]!;
        container.data[dest + 2] = row[src + 2]!;
        container.data[dest + 3] = row[src + 3]!;
      }
    }

    // Copied rather than rebound: `row` is a view into `filtered`, and the next
    // iteration unfilters against `prior` while writing into the row after it.
    prior.set(row);
  }

  return container;
}
