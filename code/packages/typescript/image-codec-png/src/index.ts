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
 * (types 0, 2, 4, 6), non-interlaced, with suggested PLTE validated, tRNS
 * transparency applied, and unknown non-semantic ancillary chunks skipped.
 * Palette (type 3), 16-bit depths and Adam7 interlacing are refused by name
 * rather than half-supported.
 */
import {
  type PixelContainer,
  type ImageCodec,
  createPixelContainer,
} from "@coding-adventures/pixel-container";
import {
  RawInflateError,
  crc32,
  rawDeflate,
  rawInflateCounted,
} from "@coding-adventures/zip";

export { type PixelContainer, type ImageCodec };

// ============================================================================
// Limits
// ============================================================================

/**
 * Largest edge this codec will decode. The same ceiling as `image-codec-bmp`.
 *
 * A PNG header is eight bytes of attacker-controlled integers claiming a size,
 * and `width * height * 4` is allocated on the strength of it.
 */
export const PNG_MAX_DIMENSION = 16384;

/**
 * Largest total pixel count this codec will decode by default: 32 mebipixels,
 * about 8000 x 4000, which is a 128 MiB RGBA buffer.
 *
 * A per-edge cap alone is not enough, and PNG is where that stops being a
 * theoretical distinction. 16384 x 16384 is within the edge cap and is 268
 * million pixels: a 1 GiB container, a 1 GiB filtered buffer, and a transient
 * second copy of the latter while it is sliced to size -- about 3 GiB peak.
 *
 * BMP could survive on the edge cap because its pixels have to BE in the file:
 * demanding a gigabyte of memory costs a gigabyte of upload. PNG compresses,
 * and DEFLATE's ratio reaches 1032:1, so the same demand costs about **one
 * megabyte**. The amplification is the whole difference, and it is why this
 * second ceiling exists here and not there.
 *
 * The number is a judgement, not a law. Decoding costs roughly THREE times the
 * pixel buffer -- the container, the filtered scanlines, and a transient copy
 * while the latter is sized -- so this default admits about 400 MB of peak
 * allocation. That is chosen to still read an ordinary camera photograph while
 * being an order of magnitude below what a per-edge cap alone would allow. An
 * embedder that knows its images are small should say so: this package's own
 * caller draws single letters and passes a ceiling in the thousands.
 */
export const PNG_MAX_PIXELS = 32 * 1024 * 1024;

/** Closed, language-neutral IC18 error identifiers. */
export const PNG_ERROR_CODES = [
  "invalid-max-pixels",
  "invalid-image-dimensions",
  "invalid-pixel-data-length",
  "file-too-short",
  "invalid-signature",
  "truncated-chunk",
  "invalid-chunk-type",
  "chunk-crc-mismatch",
  "chunk-before-ihdr",
  "duplicate-ihdr",
  "invalid-ihdr-length",
  "invalid-dimensions",
  "dimension-limit",
  "pixel-limit",
  "unsupported-feature",
  "invalid-plte",
  "invalid-trns",
  "nonconsecutive-idat",
  "invalid-iend",
  "trailing-data",
  "unknown-critical-chunk",
  "missing-required-chunk",
  "invalid-zlib-header",
  "preset-dictionary",
  "inflate-failed",
  "inflated-length-mismatch",
  "idat-cavity",
  "adler-mismatch",
  "invalid-filter",
] as const;

export type PngErrorCode = (typeof PNG_ERROR_CODES)[number];

/** A portable PNG failure with a stable code independent of its message. */
export class PngError extends Error {
  constructor(
    public readonly code: PngErrorCode,
    message: string,
  ) {
    super(message);
    this.name = "PngError";
  }
}

function fail(code: PngErrorCode, message: string): never {
  throw new PngError(code, message);
}

/** Options for {@link decodePng}. */
export interface DecodePngOptions {
  /**
   * Largest total pixel count to accept, defaulting to {@link PNG_MAX_PIXELS}.
   *
   * Lower it whenever you know roughly how big the images should be -- a
   * library reading files from strangers cannot know its embedder's budget.
   * Must be a positive safe integer no larger than {@link PNG_MAX_PIXELS}.
   */
  maxPixels?: number;
}

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
      fail("invalid-filter", `PNG: unknown filter type ${type}`);
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

/** Reject a ceiling that is not a usable number, wherever it is supplied. */
function validateMaxPixels(value: number | undefined): void {
  if (value === undefined) return;
  if (!Number.isSafeInteger(value) || value <= 0 || value > PNG_MAX_PIXELS) {
    fail(
      "invalid-max-pixels",
      `PNG: maxPixels must be a positive safe integer no greater than ${PNG_MAX_PIXELS}`,
    );
  }
}

// ============================================================================
// Chunk writing
// ============================================================================

function u32be(value: number): number[] {
  const v = value >>> 0;
  return [(v >>> 24) & 0xff, (v >>> 16) & 0xff, (v >>> 8) & 0xff, v & 0xff];
}

/**
 * The growing PNG file, held as real bytes.
 *
 * A plain `number[]` would cost four to eight bytes per output byte in V8 and
 * then a full copy on the way out -- the same accounting mistake `zip`'s
 * inflater made on the other side of this package. The input here is the
 * caller's own image rather than a stranger's, so this is a cost rather than a
 * vulnerability, but it is the same fix.
 */
class ByteBuffer {
  private buf = new Uint8Array(1024);
  private len = 0;

  private reserve(extra: number): void {
    if (this.len + extra <= this.buf.length) return;
    let next = this.buf.length;
    while (next < this.len + extra) next *= 2;
    const grown = new Uint8Array(next);
    grown.set(this.buf.subarray(0, this.len));
    this.buf = grown;
  }

  write(bytes: ArrayLike<number>): void {
    this.reserve(bytes.length);
    this.buf.set(bytes as Uint8Array, this.len);
    this.len += bytes.length;
  }

  finish(): Uint8Array {
    return this.buf.slice(0, this.len);
  }
}

/** Append one complete chunk: length, type, data, CRC over type+data. */
function pushChunk(out: ByteBuffer, type: string, data: Uint8Array): void {
  out.write(u32be(data.length));
  // The CRC covers the TYPE and the DATA together, so they are laid out
  // contiguously once and then both written and hashed from the same bytes.
  const typed = new Uint8Array(4 + data.length);
  for (let i = 0; i < 4; i++) typed[i] = type.charCodeAt(i);
  typed.set(data, 4);
  out.write(typed);
  out.write(u32be(crc32(typed)));
}

// ============================================================================
// PngCodec
// ============================================================================

/**
 * PNG image encoder and decoder implementing the `ImageCodec` interface.
 *
 * `ImageCodec.decode` takes only the bytes, so a codec constructed with a
 * tighter `maxPixels` carries it for every call. That is the only way an
 * embedder can express its own budget through the shared interface.
 */
export class PngCodec implements ImageCodec {
  readonly mimeType = "image/png";

  constructor(private readonly options: DecodePngOptions = {}) {
    // Validated here as well as in `decodePng`, so a bad ceiling fails when it
    // is supplied rather than at the first decode. `ZipReader` does the same,
    // and two sibling packages disagreeing about when an option is checked is
    // how one of them ends up with a hole.
    validateMaxPixels(options.maxPixels);
  }

  encode(pixels: PixelContainer): Uint8Array {
    return encodePng(pixels);
  }

  decode(bytes: Uint8Array): PixelContainer {
    return decodePng(bytes, this.options);
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
    fail("invalid-image-dimensions", "PNG: width and height must be non-negative integers");
  }
  if (width === 0 || height === 0) {
    fail(
      "invalid-image-dimensions",
      "PNG: an image must have at least one pixel in each dimension",
    );
  }
  if (pixels.data.length !== width * height * 4) {
    fail(
      "invalid-pixel-data-length",
      `PNG: pixel data is ${pixels.data.length} bytes, expected ${width * height * 4}`,
    );
  }

  const out = new ByteBuffer();
  out.write(SIGNATURE);

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

  return out.finish();
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
export function decodePng(bytes: Uint8Array, options: DecodePngOptions = {}): PixelContainer {
  validateMaxPixels(options.maxPixels);
  const maxPixels = options.maxPixels ?? PNG_MAX_PIXELS;
  if (bytes.length < SIGNATURE.length) fail("file-too-short", "PNG: file too short");
  for (let i = 0; i < SIGNATURE.length; i++) {
    if (bytes[i] !== SIGNATURE[i]) fail("invalid-signature", "PNG: invalid signature");
  }

  let width = 0;
  let height = 0;
  let bitDepth = 0;
  let colourType = 0;
  let sawIHDR = false;
  let sawIEND = false;
  let sawPLTE = false;
  let sawTRNS = false;
  let transparentGrey: number | undefined;
  let transparentRgb: readonly [number, number, number] | undefined;
  const idatParts: Uint8Array[] = [];
  // Tracks the IDAT run: once it has started and then stopped, no further IDAT
  // may appear.
  let inIdat = false;
  let idatEnded = false;

  let pos = SIGNATURE.length;
  while (pos < bytes.length) {
    if (pos + 8 > bytes.length) fail("truncated-chunk", "PNG: truncated chunk header");
    const length =
      ((bytes[pos]! << 24) | (bytes[pos + 1]! << 16) | (bytes[pos + 2]! << 8) | bytes[pos + 3]!) >>> 0;
    // Guard before using `length` in any arithmetic: the field is four
    // attacker-chosen bytes and can claim four gigabytes.
    if (length > bytes.length) {
      fail("truncated-chunk", "PNG: chunk length exceeds file size");
    }
    const typeStart = pos + 4;
    const dataStart = typeStart + 4;
    const dataEnd = dataStart + length;
    if (dataEnd + 4 > bytes.length) fail("truncated-chunk", "PNG: truncated chunk data");

    const typeBytes = bytes.subarray(typeStart, typeStart + 4);
    const hasOnlyLetters = typeBytes.every(
      (byte) =>
        (byte >= 0x41 && byte <= 0x5a) ||
        (byte >= 0x61 && byte <= 0x7a),
    );
    if (!hasOnlyLetters || (typeBytes[2]! & 0x20) !== 0) {
      fail("invalid-chunk-type", "PNG: invalid chunk type");
    }

    const type = String.fromCharCode(
      bytes[typeStart]!, bytes[typeStart + 1]!, bytes[typeStart + 2]!, bytes[typeStart + 3]!,
    );

    const declaredCRC =
      ((bytes[dataEnd]! << 24) | (bytes[dataEnd + 1]! << 16) | (bytes[dataEnd + 2]! << 8) | bytes[dataEnd + 3]!) >>> 0;
    const actualCRC = crc32(bytes.subarray(typeStart, dataEnd));
    if (declaredCRC !== actualCRC) {
      fail("chunk-crc-mismatch", `PNG: CRC-32 mismatch in '${type}' chunk`);
    }

    const data = bytes.subarray(dataStart, dataEnd);

    // RFC 2083: IHDR is the first chunk. This is the last member of the family
    // the rules above close -- a `tEXt` ahead of the header is a chunk out of
    // place, libpng refuses it, and accepting what the reference implementation
    // rejects is the differential this decoder exists not to have.
    if (!sawIHDR && type !== "IHDR") {
      fail("chunk-before-ihdr", `PNG: '${type}' chunk before IHDR`);
    }

    if (type === "IHDR") {
      if (sawIHDR) fail("duplicate-ihdr", "PNG: more than one IHDR chunk");
      if (length !== 13) {
        fail("invalid-ihdr-length", `PNG: IHDR must be 13 bytes, got ${length}`);
      }
      width = ((data[0]! << 24) | (data[1]! << 16) | (data[2]! << 8) | data[3]!) >>> 0;
      height = ((data[4]! << 24) | (data[5]! << 16) | (data[6]! << 8) | data[7]!) >>> 0;
      bitDepth = data[8]!;
      colourType = data[9]!;
      const compression = data[10]!;
      const filterMethod = data[11]!;
      const interlace = data[12]!;

      if (width === 0 || height === 0) fail("invalid-dimensions", "PNG: zero width or height");
      if (width > PNG_MAX_DIMENSION || height > PNG_MAX_DIMENSION) {
        fail(
          "dimension-limit",
          `PNG: dimensions ${width}x${height} exceed maximum ${PNG_MAX_DIMENSION}`,
        );
      }
      // Checked here, before anything derived from the dimensions is computed
      // or allocated. Both operands are already below 16384, so the product
      // cannot leave the exactly-representable range.
      if (width * height > maxPixels) {
        fail(
          "pixel-limit",
          `PNG: ${width}x${height} is ${width * height} pixels, above the limit of ${maxPixels}`,
        );
      }
      if (compression !== 0) {
        fail("unsupported-feature", `PNG: unsupported compression method ${compression}`);
      }
      if (filterMethod !== 0) {
        fail("unsupported-feature", `PNG: unsupported filter method ${filterMethod}`);
      }
      if (interlace !== 0) fail("unsupported-feature", "PNG: Adam7 interlacing is not supported");
      if (colourType === 3) {
        fail("unsupported-feature", "PNG: palette images (colour type 3) are not supported");
      }
      if (colourType !== 0 && colourType !== 2 && colourType !== 4 && colourType !== 6) {
        fail("unsupported-feature", `PNG: unknown colour type ${colourType}`);
      }
      if (bitDepth !== 8) {
        fail("unsupported-feature", `PNG: unsupported bit depth ${bitDepth}, only 8 is supported`);
      }
      sawIHDR = true;
    } else if (type === "PLTE") {
      // PLTE is required for palette images, but it is also a legal suggested
      // palette for truecolour images. Palette images themselves remain out of
      // scope; for types 2 and 6 we validate the bounded table and ignore it.
      if (sawPLTE) fail("invalid-plte", "PNG: more than one PLTE chunk");
      if (idatParts.length > 0) fail("invalid-plte", "PNG: PLTE must precede IDAT");
      if (sawTRNS) fail("invalid-plte", "PNG: PLTE must precede tRNS");
      if (colourType !== 2 && colourType !== 6) {
        fail("invalid-plte", `PNG: PLTE is not allowed for colour type ${colourType}`);
      }
      if (length < 3 || length > 768 || length % 3 !== 0) {
        fail("invalid-plte", `PNG: PLTE length ${length} is not 1 to 256 RGB entries`);
      }
      sawPLTE = true;
    } else if (type === "tRNS") {
      // tRNS changes rendered alpha, so it is a known ancillary chunk rather
      // than something the generic ancillary skip may ignore.
      if (sawTRNS) fail("invalid-trns", "PNG: more than one tRNS chunk");
      if (idatParts.length > 0) fail("invalid-trns", "PNG: tRNS must precede IDAT");
      if (colourType === 0) {
        if (length !== 2) fail("invalid-trns", "PNG: greyscale tRNS must be 2 bytes");
        transparentGrey = (data[0]! << 8) | data[1]!;
        if (transparentGrey > 0xff) {
          fail("invalid-trns", "PNG: greyscale tRNS sample exceeds 8-bit depth");
        }
      } else if (colourType === 2) {
        if (length !== 6) fail("invalid-trns", "PNG: truecolour tRNS must be 6 bytes");
        const red = (data[0]! << 8) | data[1]!;
        const green = (data[2]! << 8) | data[3]!;
        const blue = (data[4]! << 8) | data[5]!;
        if (red > 0xff || green > 0xff || blue > 0xff) {
          fail("invalid-trns", "PNG: truecolour tRNS sample exceeds 8-bit depth");
        }
        transparentRgb = [red, green, blue];
      } else {
        fail("invalid-trns", `PNG: tRNS is not allowed for colour type ${colourType}`);
      }
      sawTRNS = true;
    } else if (type === "IDAT") {
      // RFC 2083: multiple IDATs "shall appear consecutively with no other
      // intervening chunks". They are one stream cut into pieces, so a chunk
      // between them is either corruption or someone using the gap.
      if (idatEnded) fail("nonconsecutive-idat", "PNG: IDAT chunks are not consecutive");
      idatParts.push(data);
      inIdat = true;
    } else if (type === "IEND") {
      // IEND is defined as empty and as the LAST chunk. Both are checked, and
      // both are checked because a decoder that stops reading at IEND and looks
      // no further turns the rest of the file into free carriage: bytes that
      // travel inside something every tool calls a valid PNG.
      if (length !== 0) fail("invalid-iend", `PNG: IEND must be empty, got ${length} bytes`);
      if (dataEnd + 4 !== bytes.length) {
        fail("trailing-data", `PNG: ${bytes.length - (dataEnd + 4)} bytes follow IEND`);
      }
      sawIEND = true;
      pos = dataEnd + 4;
      break;
    } else if ((bytes[typeStart]! & 0x20) === 0) {
      // Uppercase first letter: a CRITICAL chunk. The spec says a decoder that
      // does not understand one must refuse the file, because ignoring it would
      // mean showing the user something other than what the file describes.
      fail("unknown-critical-chunk", `PNG: unsupported critical chunk '${type}'`);
    }
    // Anything else is ancillary (lowercase) -- gAMA, pHYs, tEXt and so on --
    // and is skipped by design.

    if (type !== "IDAT" && inIdat) {
      inIdat = false;
      idatEnded = true;
    }

    pos = dataEnd + 4;
  }

  if (!sawIHDR) fail("missing-required-chunk", "PNG: no IHDR chunk");
  if (!sawIEND) fail("missing-required-chunk", "PNG: no IEND chunk");
  if (idatParts.length === 0) fail("missing-required-chunk", "PNG: no IDAT chunk");

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

  if (zlib.length < 6) {
    fail("invalid-zlib-header", "PNG: IDAT too short to be a zlib stream");
  }
  const cmf = zlib[0]!;
  const flg = zlib[1]!;
  if ((cmf & 0x0f) !== 8) {
    fail("invalid-zlib-header", `PNG: zlib compression method ${cmf & 0x0f}, expected 8`);
  }
  if ((cmf >> 4) > 7) {
    fail("invalid-zlib-header", `PNG: zlib CINFO ${cmf >> 4} exceeds the maximum 7`);
  }
  if (((cmf << 8) | flg) % 31 !== 0) {
    fail("invalid-zlib-header", "PNG: corrupt zlib header");
  }
  if ((flg & 0x20) !== 0) {
    fail("preset-dictionary", "PNG: zlib preset dictionary is not supported");
  }

  const channels = colourType === 0 ? 1 : colourType === 2 ? 3 : colourType === 4 ? 2 : 4;
  const stride = width * channels;
  const expected = height * (stride + 1);

  // The cap is the exact size the header promises, so a bomb inside IDAT is
  // stopped at the size this image could possibly need rather than at a
  // generic ceiling.
  const deflateStream = zlib.subarray(2, zlib.length - 4);
  let inflateResult: { output: Uint8Array; bytesConsumed: number };
  try {
    inflateResult = rawInflateCounted(deflateStream, expected);
  } catch (error: unknown) {
    if (error instanceof RawInflateError && error.code === "output-limit-exceeded") {
      fail("inflated-length-mismatch", "PNG: decompressed data exceeds the expected length");
    }
    fail("inflate-failed", "PNG: invalid DEFLATE stream");
  }
  const { output: filtered, bytesConsumed } = inflateResult;
  if (filtered.length !== expected) {
    fail(
      "inflated-length-mismatch",
      `PNG: decompressed ${filtered.length} bytes, expected ${expected}`,
    );
  }
  // DEFLATE says where it ends -- the last block sets BFINAL -- so a stream can
  // finish well before the Adler-32 that follows it, and everything in between
  // is ignored by a decoder that only asks for the pixels. That gap is a place
  // to hide things: a scanner that unpacks the image sees nothing while the
  // bytes ride along inside a file every tool calls a valid PNG. The image is
  // identical either way, which is exactly why it has to be rejected rather
  // than tolerated.
  if (bytesConsumed !== deflateStream.length) {
    fail(
      "idat-cavity",
      `PNG: ${deflateStream.length - bytesConsumed} unused bytes between the ` +
      `compressed data and its checksum`,
    );
  }

  const declaredAdler =
    ((zlib[zlib.length - 4]! << 24) | (zlib[zlib.length - 3]! << 16) |
     (zlib[zlib.length - 2]! << 8) | zlib[zlib.length - 1]!) >>> 0;
  if (adler32(filtered) !== declaredAdler) {
    fail("adler-mismatch", "PNG: Adler-32 mismatch");
  }

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
        container.data[dest + 3] = v === transparentGrey ? 0 : 255;
      } else if (channels === 2) {
        const v = row[src]!;
        container.data[dest] = v;
        container.data[dest + 1] = v;
        container.data[dest + 2] = v;
        container.data[dest + 3] = row[src + 1]!;
      } else if (channels === 3) {
        const red = row[src]!;
        const green = row[src + 1]!;
        const blue = row[src + 2]!;
        container.data[dest] = red;
        container.data[dest + 1] = green;
        container.data[dest + 2] = blue;
        container.data[dest + 3] =
          transparentRgb !== undefined &&
          red === transparentRgb[0] &&
          green === transparentRgb[1] &&
          blue === transparentRgb[2]
            ? 0
            : 255;
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
