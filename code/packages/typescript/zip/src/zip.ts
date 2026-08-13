/**
 * zip.ts — CMP09: ZIP archive format (PKZIP, 1989).
 *
 * ZIP bundles one or more files into a single `.zip` archive, compressing each
 * entry independently with DEFLATE (method 8) or storing it verbatim (method 0).
 * The same format underlies Java JARs, Office Open XML (.docx/.xlsx), Android
 * APKs (.apk), Python wheels (.whl), and many more.
 *
 * Architecture:
 * ```
 * ┌─────────────────────────────────────────────────────┐
 * │  [Local File Header + File Data]  ← entry 1         │
 * │  [Local File Header + File Data]  ← entry 2         │
 * │  ...                                                │
 * │  ══════════ Central Directory ══════════            │
 * │  [Central Dir Header]  ← entry 1 (has local offset)│
 * │  [Central Dir Header]  ← entry 2                   │
 * │  [End of Central Directory Record]                  │
 * └─────────────────────────────────────────────────────┘
 * ```
 *
 * DEFLATE Inside ZIP:
 * ZIP method 8 stores raw RFC 1951 DEFLATE — no zlib wrapper. This
 * implementation uses fixed Huffman blocks (BTYPE=01) and the `lzss` package
 * for LZ77 match-finding.
 *
 * Series:
 * ```
 * CMP02 (LZSS,    1982) — LZ77 + flag bits.  ← dependency
 * CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib.
 * CMP09 (ZIP,     1989) — DEFLATE container; universal archive. ← this file
 * ```
 */

import { encode as lzssEncode, type Token as LzssToken } from "@coding-adventures/lzss";

// =============================================================================
// CRC-32
// =============================================================================
//
// CRC-32 uses polynomial 0xEDB88320 (reflected form of 0x04C11DB7).

const CRC_TABLE: Uint32Array = (() => {
  const t = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let k = 0; k < 8; k++) {
      c = c & 1 ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
    }
    t[i] = c >>> 0;
  }
  return t;
})();

/**
 * Compute CRC-32 over `data`, starting from `initial` (0 for a fresh hash).
 * For incremental updates, pass the previous result as `initial`.
 *
 * @example
 * crc32(new TextEncoder().encode("hello world"), 0) === 0x0D4A1185
 */
export function crc32(data: Uint8Array, initial = 0): number {
  let crc = (initial ^ 0xffffffff) >>> 0;
  for (const byte of data) {
    crc = ((CRC_TABLE[(crc ^ byte) & 0xff] ?? 0) ^ (crc >>> 8)) >>> 0;
  }
  return (crc ^ 0xffffffff) >>> 0;
}

// =============================================================================
// RFC 1951 DEFLATE — Bit I/O
// =============================================================================
//
// RFC 1951 packs bits LSB-first. Huffman codes are written MSB-first logically,
// so we bit-reverse them before writing LSB-first. We use BigInt for the
// accumulator so we can safely buffer up to 64 bits without overflow.

function reverseBits(value: number, nbits: number): number {
  let result = 0;
  for (let i = 0; i < nbits; i++) {
    result = ((result << 1) | (value & 1)) >>> 0;
    value >>>= 1;
  }
  return result;
}

class BitWriter {
  private buf: bigint = 0n;
  private bits = 0;
  private out: number[] = [];

  writeLSB(value: number, nbits: number): void {
    this.buf |= BigInt(value >>> 0) << BigInt(this.bits);
    this.bits += nbits;
    while (this.bits >= 8) {
      this.out.push(Number(this.buf & 0xffn));
      this.buf >>= 8n;
      this.bits -= 8;
    }
  }

  writeHuffman(code: number, nbits: number): void {
    this.writeLSB(reverseBits(code, nbits), nbits);
  }

  align(): void {
    if (this.bits > 0) {
      this.out.push(Number(this.buf & 0xffn));
      this.buf = 0n;
      this.bits = 0;
    }
  }

  finish(): Uint8Array {
    this.align();
    return new Uint8Array(this.out);
  }
}

class BitReader {
  private pos = 0;
  private buf = 0n;
  private bits = 0;

  constructor(private readonly data: Uint8Array) {}

  private fill(need: number): boolean {
    while (this.bits < need) {
      if (this.pos >= this.data.length) return false;
      this.buf |= BigInt(this.data[this.pos]!) << BigInt(this.bits);
      this.pos++;
      this.bits += 8;
    }
    return true;
  }

  readLSB(nbits: number): number | null {
    if (nbits === 0) return 0;
    if (!this.fill(nbits)) return null;
    const mask = (1n << BigInt(nbits)) - 1n;
    const val = Number(this.buf & mask);
    this.buf >>= BigInt(nbits);
    this.bits -= nbits;
    return val;
  }

  readMSB(nbits: number): number | null {
    const v = this.readLSB(nbits);
    return v === null ? null : reverseBits(v, nbits);
  }

  align(): void {
    const discard = this.bits % 8;
    if (discard > 0) {
      this.buf >>= BigInt(discard);
      this.bits -= discard;
    }
  }

  /**
   * How many bytes of the input the stream has actually reached.
   *
   * `pos` counts bytes pulled INTO the bit buffer, some of whose bits may still
   * be unread, so whole unread bytes are subtracted back off. A partially-read
   * byte counts as consumed, because the reader can never un-see it.
   *
   * Callers use this to ask "did the compressed data end where the container
   * said it would?" -- a question a format like PNG or gzip has to be able to
   * answer, because the bytes between the end of a DEFLATE stream and the
   * checksum after it are a place to hide things.
   */
  bytesConsumed(): number {
    return this.pos - (this.bits >> 3);
  }
}

// =============================================================================
// RFC 1951 DEFLATE — Fixed Huffman Tables
// =============================================================================
//
// RFC 1951 §3.2.6 fixed code lengths:
//   Symbols   0–143: 8-bit codes, starting at 0b00110000 (= 48)
//   Symbols 144–255: 9-bit codes, starting at 0b110010000 (= 400)
//   Symbols 256–279: 7-bit codes, starting at 0b0000000 (= 0)
//   Symbols 280–287: 8-bit codes, starting at 0b11000000 (= 192)
// Distance codes 0–29: 5-bit codes equal to the code number.

function fixedLLEncode(sym: number): [number, number] {
  if (sym <= 143) return [0b00110000 + sym, 8];
  if (sym <= 255) return [0b110010000 + (sym - 144), 9];
  if (sym <= 279) return [sym - 256, 7];
  if (sym <= 287) return [0b11000000 + (sym - 280), 8];
  throw new Error(`fixedLLEncode: invalid symbol ${sym}`);
}

function fixedLLDecode(br: BitReader): number | null {
  const v7 = br.readMSB(7);
  if (v7 === null) return null;
  if (v7 <= 23) return v7 + 256; // 7-bit: 256-279
  const extra = br.readLSB(1);
  if (extra === null) return null;
  const v8 = (v7 << 1) | extra;
  if (v8 >= 48 && v8 <= 191) return v8 - 48;    // literals 0-143
  if (v8 >= 192 && v8 <= 199) return v8 + 88;    // symbols 280-287
  const extra2 = br.readLSB(1);
  if (extra2 === null) return null;
  const v9 = (v8 << 1) | extra2;
  if (v9 >= 400 && v9 <= 511) return v9 - 256;   // literals 144-255
  return null;
}

// =============================================================================
// RFC 1951 DEFLATE — Canonical Huffman decoding (for BTYPE=10)
// =============================================================================
//
// A fixed-Huffman block (BTYPE=01) uses the one table baked into the spec, so
// `fixedLLDecode` above can hard-code it. A DYNAMIC block (BTYPE=10) carries
// its own table, and it carries it in the most compact way imaginable: not the
// codes, just the LENGTH of each symbol's code. Everything else is recoverable,
// because a canonical Huffman code is fully determined by its lengths:
//
//   - sort the symbols by (code length, then symbol number);
//   - hand out codes in that order, counting up in binary;
//   - step the counter left by one bit each time the length increases.
//
// So the whole table is reconstructible from a list of small integers. That is
// why DEFLATE can afford to ship a custom table with every block.
//
// `HuffTable` stores exactly what decoding needs: how many symbols have each
// code length, and the symbols themselves in canonical order.

interface HuffTable {
  /** count[len] = how many symbols use a code of `len` bits. count[0] is unused. */
  count: Int32Array;
  /** Symbols ordered by (length, symbol) -- the canonical assignment order. */
  symbols: Int32Array;
  /**
   * True when the code uses up the whole code space exactly (Kraft sum == 1).
   *
   * An INCOMPLETE code leaves bit patterns that decode to no symbol at all. RFC
   * 1951 permits exactly one such case -- a distance alphabet with a single code,
   * used by a block that never emits a back-reference -- and forbids it
   * everywhere else, so the caller decides rather than this builder.
   */
  complete: boolean;
}

/** The longest code RFC 1951 allows in either alphabet. */
const MAX_CODE_BITS = 15;

/**
 * Build a canonical decode table from one code length per symbol.
 *
 * A length of 0 means "this symbol does not appear in this block" and takes no
 * code at all, which is why `count[0]` is deliberately never consulted.
 */
function buildHuffTable(lengths: ArrayLike<number>): HuffTable {
  const count = new Int32Array(MAX_CODE_BITS + 1);
  for (let i = 0; i < lengths.length; i++) {
    const len = lengths[i]!;
    if (len < 0 || len > MAX_CODE_BITS) throw new Error(`deflate: code length ${len} out of range`);
    if (len > 0) count[len]!++;
  }

  // Kraft's inequality, walked one length at a time.
  //
  // Think of the code space as a single unit that doubles in resolution with
  // each extra bit: there are 2 one-bit codes, 4 two-bit codes, and so on, and
  // every code handed out at length L removes two potential codes at L+1.
  // `left` tracks how much of that space is still unclaimed.
  //
  //   left < 0 at any length  ->  OVER-SUBSCRIBED: more codes were demanded
  //                               than exist. The surplus symbols are simply
  //                               unreachable, so decoding would appear to work
  //                               while quietly disagreeing with every other
  //                               inflater about what the stream means.
  //   left > 0 at the end     ->  INCOMPLETE: some bit patterns decode to
  //                               nothing.
  //
  // Rejecting the first outright and reporting the second is what keeps this
  // decoder from accepting streams zlib refuses. A decompressor that accepts
  // MORE than the reference implementation is not being liberal; it is a place
  // where two programs read the same bytes differently, which is exactly the
  // shape of a content-inspection bypass.
  let left = 1;
  for (let len = 1; len <= MAX_CODE_BITS; len++) {
    left = (left << 1) - count[len]!;
    if (left < 0) throw new Error("deflate: over-subscribed Huffman table");
  }

  // Offset of each length's first symbol inside `symbols`.
  const offsets = new Int32Array(MAX_CODE_BITS + 2);
  for (let len = 1; len <= MAX_CODE_BITS; len++) {
    offsets[len + 1] = offsets[len]! + count[len]!;
  }

  const symbols = new Int32Array(offsets[MAX_CODE_BITS + 1]!);
  for (let sym = 0; sym < lengths.length; sym++) {
    const len = lengths[sym]!;
    if (len > 0) symbols[offsets[len]!++] = sym;
  }

  return { count, symbols, complete: left === 0 };
}

/**
 * Decode one symbol, reading the stream one bit at a time.
 *
 * This walks lengths from 1 upward, accumulating bits into `code` and asking at
 * each length "is this code inside the block of codes of this length?" -- the
 * canonical layout is what makes that a subtraction rather than a search. The
 * shape of the loop follows Mark Adler's `puff` reference decoder, which is the
 * clearest published statement of it.
 *
 * DEFLATE packs Huffman codes most-significant-bit first, even though it packs
 * everything else LSB-first, which is why the bits are read singly and shifted
 * in from the bottom rather than pulled out in one `readLSB(n)`.
 */
function huffDecode(br: BitReader, table: HuffTable): number {
  let code = 0;   // the bits read so far, as a number
  let first = 0;  // the first canonical code of the current length
  let index = 0;  // where this length's symbols start in table.symbols

  for (let len = 1; len <= MAX_CODE_BITS; len++) {
    const bit = br.readLSB(1);
    if (bit === null) throw new Error("deflate: EOF decoding Huffman symbol");
    code |= bit;
    const n = table.count[len]!;
    if (code - first < n) {
      // In bounds by construction: `code - first` is non-negative by induction
      // on the loop, and is less than `n`, so `index + (code - first)` stays
      // below `index + n`, which is the running total of counts. The throw is
      // for the type checker and for the day the proof stops holding -- this
      // decoder never turns a broken invariant into a plausible byte.
      const sym = table.symbols[index + (code - first)];
      if (sym === undefined) throw new Error("deflate: internal table index out of range");
      return sym;
    }
    index += n;
    first = (first + n) << 1;
    code <<= 1;
  }
  throw new Error("deflate: over-long Huffman code (no symbol within 15 bits)");
}

// The order in which the code-length alphabet's own code lengths are written.
// It is not 0..18 but this permutation, so that the lengths most likely to be
// zero sit at the end and can be omitted entirely via HCLEN.
const CODE_LENGTH_ORDER = [
  16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
] as const;

/**
 * Read a dynamic block's header and return its literal/length and distance
 * tables (RFC 1951 section 3.2.7).
 */
function readDynamicTables(br: BitReader): { ll: HuffTable; dist: HuffTable } {
  const hlit = br.readLSB(5);
  const hdist = br.readLSB(5);
  const hclen = br.readLSB(4);
  if (hlit === null || hdist === null || hclen === null) {
    throw new Error("deflate: EOF reading dynamic block header");
  }
  const numLL = hlit + 257;
  const numDist = hdist + 1;
  const numCodeLen = hclen + 4;

  // The five-bit fields can express 288 and 32, but RFC 1951 defines only 286
  // literal/length symbols and 30 distance codes. Refusing the surplus here
  // means a malformed header fails at the header, rather than a hundred
  // kilobytes later when an unassignable symbol finally turns up.
  if (numLL > 286) throw new Error(`deflate: ${numLL} literal/length codes exceeds the 286 RFC 1951 defines`);
  if (numDist > 30) throw new Error(`deflate: ${numDist} distance codes exceeds the 30 RFC 1951 defines`);

  // Stage 1: the code-length alphabet, three bits per entry, in permuted order.
  const clLengths = new Int32Array(19);
  for (let i = 0; i < numCodeLen; i++) {
    const v = br.readLSB(3);
    if (v === null) throw new Error("deflate: EOF reading code-length code lengths");
    clLengths[CODE_LENGTH_ORDER[i]!] = v;
  }
  const clTable = buildHuffTable(clLengths);
  if (!clTable.complete) throw new Error("deflate: incomplete code-length Huffman table");

  // Stage 2: use it to read the real alphabets' lengths, which are themselves
  // run-length coded -- symbol 16 repeats the previous length, 17 and 18 repeat
  // zero. The two alphabets are read as ONE stream and split afterwards,
  // because a run is allowed to straddle the boundary between them.
  const lengths = new Int32Array(numLL + numDist);
  let i = 0;
  while (i < lengths.length) {
    const sym = huffDecode(br, clTable);
    if (sym < 16) {
      lengths[i++] = sym;
      continue;
    }
    let repeat: number;
    let value: number;
    if (sym === 16) {
      if (i === 0) throw new Error("deflate: code-length repeat with no previous length");
      value = lengths[i - 1]!;
      const extra = br.readLSB(2);
      if (extra === null) throw new Error("deflate: EOF reading repeat count");
      repeat = 3 + extra;
    } else if (sym === 17) {
      value = 0;
      const extra = br.readLSB(3);
      if (extra === null) throw new Error("deflate: EOF reading zero-repeat count");
      repeat = 3 + extra;
    } else if (sym === 18) {
      value = 0;
      const extra = br.readLSB(7);
      if (extra === null) throw new Error("deflate: EOF reading long zero-repeat count");
      repeat = 11 + extra;
    } else {
      throw new Error(`deflate: invalid code-length symbol ${sym}`);
    }
    if (i + repeat > lengths.length) throw new Error("deflate: code-length repeat overruns alphabet");
    for (let r = 0; r < repeat; r++) lengths[i++] = value;
  }

  const ll = buildHuffTable(lengths.subarray(0, numLL));
  // Deliberately stricter than zlib, which extends the single-one-bit-code
  // exception below to this alphabet too. The only block that could use it is
  // one whose entire literal/length alphabet is a lone end-of-block, which no
  // encoder emits and which carries no data. Refusing it errs toward accepting
  // LESS than the reference implementation, which is the safe direction: the
  // danger is reading a stream a scanner rejected, never the reverse.
  if (!ll.complete) throw new Error("deflate: incomplete literal/length Huffman table");

  const dist = buildHuffTable(lengths.subarray(numLL));
  // The one incompleteness RFC 1951 allows, quoted exactly because the
  // near-miss reading is wrong: a block that emits no back-reference still has
  // to declare a distance alphabet, and section 3.2.7 says that if only one
  // distance code is used "it is encoded using one bit, not zero bits; in this
  // case there is a single code length of one."
  //
  // So the exception is keyed on the code's LENGTH, not on the symbol count. A
  // lone distance code of two or more bits leaves a hole and is rejected by
  // zlib, which checks `max != 1` -- and accepting what zlib rejects is the
  // parser differential this whole section exists to close.
  const singleOneBitCode = dist.symbols.length === 1 && dist.count[1] === 1;
  const emptyAlphabet = dist.symbols.length === 0;
  if (!dist.complete && !singleOneBitCode && !emptyAlphabet) {
    throw new Error("deflate: incomplete distance Huffman table");
  }

  return { ll, dist };
}

// =============================================================================
// RFC 1951 DEFLATE — Length / Distance Tables
// =============================================================================

type TableEntry = readonly [number, number]; // [base, extraBits]

const LENGTH_TABLE: ReadonlyArray<TableEntry> = [
  [3, 0], [4, 0], [5, 0], [6, 0], [7, 0], [8, 0], [9, 0], [10, 0], // 257-264
  [11, 1], [13, 1], [15, 1], [17, 1],                                 // 265-268
  [19, 2], [23, 2], [27, 2], [31, 2],                                 // 269-272
  [35, 3], [43, 3], [51, 3], [59, 3],                                 // 273-276
  [67, 4], [83, 4], [99, 4], [115, 4],                                // 277-280
  [131, 5], [163, 5], [195, 5], [227, 5],                             // 281-284
  [258, 0],                                                           // 285
];

// Symbol 285 is the odd one out, and leaving it off the table used to make this
// decoder reject perfectly legal streams. RFC 1951 gives length 258 -- the
// longest match DEFLATE can express -- TWO encodings: symbol 284 with five
// extra bits all set (227 + 31), and symbol 285 with no extra bits at all.
// Symbol 285 is the cheaper one, so most encoders in the world emit it, and a
// 258-byte match is exactly what a long run of identical bytes produces. Our
// own writer keeps using 284 (see `encodeLength`, which stops one entry short)
// so its output stays byte-stable; the reader accepts both, because it has to
// read what other people wrote.

/** Number of length symbols the ENCODER will emit: 257-284, never 285. */
const ENCODER_LENGTH_SYMBOLS = 28;

const DIST_TABLE: ReadonlyArray<TableEntry> = [
  [1, 0], [2, 0], [3, 0], [4, 0],
  [5, 1], [7, 1], [9, 2], [13, 2],
  [17, 3], [25, 3], [33, 4], [49, 4],
  [65, 5], [97, 5], [129, 6], [193, 6],
  [257, 7], [385, 7], [513, 8], [769, 8],
  [1025, 9], [1537, 9], [2049, 10], [3073, 10],
  [4097, 11], [6145, 11], [8193, 12], [12289, 12],
  [16385, 13], [24577, 13],
];

function encodeLength(length: number): [number, number, number] {
  for (let i = ENCODER_LENGTH_SYMBOLS - 1; i >= 0; i--) {
    const [base, extra] = LENGTH_TABLE[i]!;
    if (length >= base) return [257 + i, base, extra];
  }
  throw new Error(`encodeLength: unreachable for length=${length}`);
}

function encodeDist(offset: number): [number, number, number] {
  for (let i = DIST_TABLE.length - 1; i >= 0; i--) {
    const [base, extra] = DIST_TABLE[i]!;
    if (offset >= base) return [i, base, extra];
  }
  throw new Error(`encodeDist: unreachable for offset=${offset}`);
}

// =============================================================================
// RFC 1951 DEFLATE — Compress (fixed Huffman, BTYPE=01)
// =============================================================================

/**
 * Compress `data` into a raw RFC 1951 DEFLATE stream -- no ZIP framing, no
 * zlib wrapper, no gzip header. One final fixed-Huffman block (BTYPE=01), or a
 * single empty stored block when there is nothing to compress.
 *
 * Exported because DEFLATE is not a ZIP feature that happens to live here; it
 * is the compressor half of `zlib`, `gzip`, and PNG's `IDAT`. A second copy
 * elsewhere in the repository would be a second place for a bit-packing bug to
 * hide. Wrap it yourself for those formats -- zlib adds a two-byte header and a
 * trailing Adler-32, gzip a ten-byte header and a trailing CRC-32 plus length.
 *
 * @example
 * const raw = rawDeflate(new TextEncoder().encode("hello hello hello"));
 * rawInflate(raw); // the original bytes
 */
export function rawDeflate(data: Uint8Array): Uint8Array {
  return deflateCompress(data);
}

/**
 * Decompress a raw RFC 1951 DEFLATE stream produced by `rawDeflate` or by any
 * other conforming encoder.
 *
 * All three block types are read: stored (BTYPE=00), fixed Huffman (BTYPE=01),
 * and dynamic Huffman (BTYPE=10), which is what general-purpose encoders such
 * as zlib emit for anything but the smallest inputs.
 *
 * **This reads bytes you did not write.** Malformed input always throws --
 * it never returns partial or wrong output -- so callers should be prepared to
 * catch. Output is capped at `maxOutput` bytes, 256 MB by default.
 *
 * Pass a smaller `maxOutput` whenever you know the answer's size, because
 * DEFLATE's expansion ratio reaches 1032:1 and a few hundred kilobytes of
 * hostile input can otherwise demand hundreds of megabytes. `ZipReader` passes
 * the entry's declared uncompressed size for exactly this reason.
 *
 * @param data - a raw DEFLATE stream, with no zlib, gzip, or ZIP framing.
 * @param maxOutput - byte ceiling on the decompressed result.
 *
 * @example
 * rawInflate(rawDeflate(bytes));            // round-trips
 * rawInflate(untrusted, 1 << 20);           // refuse anything over 1 MB
 */
export function rawInflate(data: Uint8Array, maxOutput?: number): Uint8Array {
  return deflateDecompress(data, maxOutput).output;
}

/** What {@link rawInflateCounted} returns. */
export interface InflateResult {
  output: Uint8Array;
  /**
   * How many bytes of `data` the stream actually used, including a final
   * partially-consumed byte.
   */
  bytesConsumed: number;
}

/**
 * As {@link rawInflate}, but also reporting how much of the input was used.
 *
 * DEFLATE says where it ends -- the last block sets BFINAL -- so a stream can
 * finish well before its container claims, and every byte after that point is
 * ignored by a decompressor that only asks for the data. That gap is a place to
 * hide things: a scanner that unpacks the stream sees nothing, while the bytes
 * still travel inside a file that every tool calls valid.
 *
 * A container that knows where its compressed data should end -- PNG's `IDAT`
 * before its Adler-32, gzip before its CRC -- can compare that boundary against
 * this number and refuse the difference.
 *
 * @example
 * const { output, bytesConsumed } = rawInflateCounted(stream);
 * if (bytesConsumed !== stream.length) throw new Error("trailing data");
 */
export function rawInflateCounted(data: Uint8Array, maxOutput?: number): InflateResult {
  return deflateDecompress(data, maxOutput);
}

function deflateCompress(data: Uint8Array): Uint8Array {
  const bw = new BitWriter();

  if (data.length === 0) {
    bw.writeLSB(1, 1);       // BFINAL=1
    bw.writeLSB(0, 2);       // BTYPE=00 (stored)
    bw.align();
    bw.writeLSB(0x0000, 16); // LEN=0
    bw.writeLSB(0xffff, 16); // NLEN=~0
    return bw.finish();
  }

  const tokens: LzssToken[] = lzssEncode(data, 32768, 255, 3);

  bw.writeLSB(1, 1); // BFINAL
  bw.writeLSB(1, 1); // BTYPE bit 0 = 1
  bw.writeLSB(0, 1); // BTYPE bit 1 = 0  → BTYPE = 01

  for (const tok of tokens) {
    if (tok.kind === "literal") {
      const [code, nbits] = fixedLLEncode(tok.byte);
      bw.writeHuffman(code, nbits);
    } else {
      const [sym, baseLen, extraLenBits] = encodeLength(tok.length);
      const [code, nbits] = fixedLLEncode(sym);
      bw.writeHuffman(code, nbits);
      if (extraLenBits > 0) bw.writeLSB(tok.length - baseLen, extraLenBits);

      const [distCode, baseDist, extraDistBits] = encodeDist(tok.offset);
      bw.writeHuffman(distCode, 5);
      if (extraDistBits > 0) bw.writeLSB(tok.offset - baseDist, extraDistBits);
    }
  }

  const [eobCode, eobBits] = fixedLLEncode(256);
  bw.writeHuffman(eobCode, eobBits);
  return bw.finish();
}

// =============================================================================
// RFC 1951 DEFLATE — Decompress
// =============================================================================

const MAX_OUTPUT = 256 * 1024 * 1024;

/**
 * The growing output of an inflate, held as real bytes.
 *
 * This used to be a plain `number[]`, and the difference is not cosmetic. A
 * JavaScript array of small integers costs four to eight bytes per element in
 * V8, so a cap of "256 million entries" was really a cap of one to two
 * GIGABYTES of backing store -- plus a transient second copy each time the
 * array doubled. On a container with a normal heap limit the process died
 * before the limit it was supposedly enforcing was ever reached, which turns a
 * catchable error into a crash.
 *
 * That matters here because DEFLATE is a compression format and compression
 * formats have bombs. The theoretical ceiling is 1032:1 -- a two-bit symbol
 * pair can copy 258 bytes -- so a few hundred kilobytes of hostile input can
 * demand hundreds of megabytes of output. Counting the cap in bytes makes the
 * limit mean what it says.
 */
class ByteSink {
  private buf: Uint8Array;
  private len = 0;

  constructor(private readonly limit: number) {
    this.buf = new Uint8Array(Math.min(1024, Math.max(limit, 1)));
  }

  get length(): number {
    return this.len;
  }

  private reserve(extra: number): void {
    if (this.len + extra > this.limit) {
      throw new Error("deflate: output size limit exceeded");
    }
    if (this.len + extra <= this.buf.length) return;
    let next = this.buf.length;
    while (next < this.len + extra) next *= 2;
    if (next > this.limit) next = this.limit;
    const grown = new Uint8Array(next);
    grown.set(this.buf.subarray(0, this.len));
    this.buf = grown;
  }

  push(byte: number): void {
    this.reserve(1);
    this.buf[this.len++] = byte;
  }

  /**
   * Copy `length` bytes from `offset` back in the output.
   *
   * Deliberately byte-at-a-time: an overlapping copy, where `offset` is smaller
   * than `length`, is legal DEFLATE and is exactly how it expresses a run. A
   * bulk `set()` of a pre-sliced source would read the region as it was before
   * the copy started and silently produce different bytes.
   */
  copyBack(offset: number, length: number): void {
    this.reserve(length);
    for (let i = 0; i < length; i++) {
      this.buf[this.len] = this.buf[this.len - offset]!;
      this.len++;
    }
  }

  finish(): Uint8Array {
    return this.buf.slice(0, this.len);
  }
}

/**
 * Decode the body of one compressed block, given whatever pair of decoders the
 * block type supplies.
 *
 * Fixed and dynamic blocks differ ONLY in how a symbol is read off the bit
 * stream. Everything after that -- literal, end-of-block, or a length followed
 * by a distance and a copy from the output already produced -- is identical, so
 * it is written once here and handed the two readers.
 */
function decodeHuffmanBlock(
  br: BitReader,
  out: ByteSink,
  readSymbol: () => number,
  readDistCode: () => number,
): void {
  for (;;) {
    const sym = readSymbol();
    if (sym < 256) {
      out.push(sym);
    } else if (sym === 256) {
      return;
    } else if (sym >= 257 && sym <= 285) {
      const entry = LENGTH_TABLE[sym - 257];
      if (!entry) throw new Error(`deflate: invalid length sym ${sym}`);
      const [baseLen, extraLenBits] = entry;
      const extraLen = br.readLSB(extraLenBits);
      if (extraLen === null) throw new Error("deflate: EOF reading length extra bits");
      const length = baseLen + extraLen;

      const distCode = readDistCode();
      const distEntry = DIST_TABLE[distCode];
      if (!distEntry) throw new Error(`deflate: invalid dist code ${distCode}`);
      const [baseDist, extraDistBits] = distEntry;
      const extraDist = br.readLSB(extraDistBits);
      if (extraDist === null) throw new Error("deflate: EOF reading distance extra bits");
      const offset = baseDist + extraDist;

      if (offset > out.length) {
        throw new Error(`deflate: back-reference offset ${offset} > output len ${out.length}`);
      }
      out.copyBack(offset, length);
    } else {
      throw new Error(`deflate: invalid LL symbol ${sym}`);
    }
  }
}

function deflateDecompress(data: Uint8Array, maxOutput: number = MAX_OUTPUT): InflateResult {
  if (!Number.isFinite(maxOutput) || maxOutput < 0) {
    throw new Error("deflate: maxOutput must be a non-negative finite number");
  }
  const br = new BitReader(data);
  const out = new ByteSink(maxOutput);

  for (;;) {
    const bfinal = br.readLSB(1);
    if (bfinal === null) throw new Error("deflate: unexpected EOF reading BFINAL");
    const btype = br.readLSB(2);
    if (btype === null) throw new Error("deflate: unexpected EOF reading BTYPE");

    if (btype === 0) {
      // Stored block
      br.align();
      const lenVal = br.readLSB(16);
      if (lenVal === null) throw new Error("deflate: EOF reading stored LEN");
      const nlen = br.readLSB(16);
      if (nlen === null) throw new Error("deflate: EOF reading stored NLEN");
      if ((nlen ^ 0xffff) !== lenVal) throw new Error(`deflate: LEN/NLEN mismatch: ${lenVal} vs ${nlen}`);
      for (let i = 0; i < lenVal; i++) {
        const b = br.readLSB(8);
        if (b === null) throw new Error("deflate: EOF inside stored block data");
        out.push(b);
      }
    } else if (btype === 1) {
      // Fixed Huffman block: the table is the one in the spec, and distance
      // codes are a plain 5-bit value rather than a Huffman code.
      decodeHuffmanBlock(
        br,
        out,
        () => {
          const sym = fixedLLDecode(br);
          if (sym === null) throw new Error("deflate: EOF decoding fixed Huffman symbol");
          return sym;
        },
        () => {
          const distCode = br.readMSB(5);
          if (distCode === null) throw new Error("deflate: EOF reading distance code");
          return distCode;
        },
      );
    } else if (btype === 2) {
      // Dynamic Huffman block: both alphabets are described in the block header.
      const { ll, dist } = readDynamicTables(br);
      decodeHuffmanBlock(br, out, () => huffDecode(br, ll), () => huffDecode(br, dist));
    } else {
      throw new Error("deflate: reserved BTYPE=11");
    }

    if (bfinal === 1) break;
  }
  return { output: out.finish(), bytesConsumed: br.bytesConsumed() };
}

// =============================================================================
// MS-DOS Date / Time Encoding
// =============================================================================

/**
 * Encode a timestamp into the 32-bit MS-DOS datetime used by ZIP headers.
 *
 * @example
 * dosDatetime(1980, 1, 1) >>> 16 === 33  // date field
 * dosDatetime(1980, 1, 1) & 0xFFFF === 0  // time field
 */
export function dosDatetime(
  year: number, month: number, day: number,
  hour = 0, minute = 0, second = 0
): number {
  const t = (hour << 11) | (minute << 5) | (second >>> 1);
  const d = (Math.max(0, year - 1980) << 9) | (month << 5) | day;
  return (((d & 0xffff) << 16) | (t & 0xffff)) >>> 0;
}

/** Fixed timestamp for 1980-01-01 00:00:00. */
export const DOS_EPOCH: number = dosDatetime(1980, 1, 1);

// =============================================================================
// ZIP Write — ZipWriter
// =============================================================================

interface CdRecord {
  name: Uint8Array;
  method: number;
  crc: number;
  compressedSize: number;
  uncompressedSize: number;
  localOffset: number;
  externalAttrs: number;
}

/** Builds a ZIP archive incrementally in memory. */
export class ZipWriter {
  private buf: number[] = [];
  private entries: CdRecord[] = [];

  /** Add a file entry. Compress with DEFLATE if it reduces size. */
  addFile(name: string, data: Uint8Array, compress = true): void {
    this.addEntry(name, data, compress, 0o100644);
  }

  /** Add a directory entry (name should end with '/'). */
  addDirectory(name: string): void {
    this.addEntry(name, new Uint8Array(0), false, 0o040755);
  }

  private addEntry(name: string, data: Uint8Array, compress: boolean, unixMode: number): void {
    const nameBytes = new TextEncoder().encode(name);
    const checksum = crc32(data);
    const uncompressedSize = data.length;

    let method: number;
    let fileData: Uint8Array;
    if (compress && data.length > 0) {
      const compressed = deflateCompress(data);
      if (compressed.length < data.length) {
        method = 8; fileData = compressed;
      } else {
        method = 0; fileData = data;
      }
    } else {
      method = 0; fileData = data;
    }

    const compressedSize = fileData.length;
    const localOffset = this.buf.length;
    const versionNeeded = method === 8 ? 20 : 10;
    const flags = 0x0800;

    // Local File Header
    this.pushLE32(0x04034b50);
    this.pushLE16(versionNeeded);
    this.pushLE16(flags);
    this.pushLE16(method);
    this.pushLE16(DOS_EPOCH & 0xffff);         // mod_time
    this.pushLE16((DOS_EPOCH >>> 16) & 0xffff); // mod_date
    this.pushLE32(checksum);
    this.pushLE32(compressedSize);
    this.pushLE32(uncompressedSize);
    this.pushLE16(nameBytes.length);
    this.pushLE16(0); // extra_field_length = 0
    for (const b of nameBytes) this.buf.push(b);
    for (const b of fileData) this.buf.push(b);

    this.entries.push({ name: nameBytes, method, crc: checksum, compressedSize, uncompressedSize, localOffset, externalAttrs: (unixMode << 16) >>> 0 });
  }

  /** Append Central Directory and EOCD; return the archive as Uint8Array. */
  finish(): Uint8Array {
    const cdOffset = this.buf.length;
    const cdStart = this.buf.length;
    for (const e of this.entries) {
      const versionNeeded = e.method === 8 ? 20 : 10;
      this.pushLE32(0x02014b50);
      this.pushLE16(0x031e);                            // version_made_by
      this.pushLE16(versionNeeded);
      this.pushLE16(0x0800);                            // flags (UTF-8)
      this.pushLE16(e.method);
      this.pushLE16(DOS_EPOCH & 0xffff);                // mod_time
      this.pushLE16((DOS_EPOCH >>> 16) & 0xffff);       // mod_date
      this.pushLE32(e.crc);
      this.pushLE32(e.compressedSize);
      this.pushLE32(e.uncompressedSize);
      this.pushLE16(e.name.length);
      this.pushLE16(0); // extra_len
      this.pushLE16(0); // comment_len
      this.pushLE16(0); // disk_start
      this.pushLE16(0); // internal_attrs
      this.pushLE32(e.externalAttrs);
      this.pushLE32(e.localOffset);
      for (const b of e.name) this.buf.push(b);
    }
    const cdSize = this.buf.length - cdStart;

    this.pushLE32(0x06054b50); // EOCD signature
    this.pushLE16(0);
    this.pushLE16(0);
    this.pushLE16(this.entries.length);
    this.pushLE16(this.entries.length);
    this.pushLE32(cdSize);
    this.pushLE32(cdOffset);
    this.pushLE16(0);

    return new Uint8Array(this.buf);
  }

  private pushLE16(v: number): void {
    this.buf.push(v & 0xff, (v >>> 8) & 0xff);
  }

  private pushLE32(v: number): void {
    const u = v >>> 0;
    this.buf.push(u & 0xff, (u >>> 8) & 0xff, (u >>> 16) & 0xff, (u >>> 24) & 0xff);
  }
}

// =============================================================================
// ZIP Read — ZipEntry and ZipReader
// =============================================================================

/** Metadata for a single entry inside a ZIP archive. */
export interface ZipEntry {
  readonly name: string;
  readonly size: number;
  readonly compressedSize: number;
  readonly method: number;
  readonly crc32: number;
  readonly isDirectory: boolean;
  readonly localOffset: number;
}

/** Reads entries from an in-memory ZIP archive. */
/** Reader options. */
export interface ZipReaderOptions {
  /**
   * Byte ceiling on any single DEFLATED entry's decompressed size. Defaults to
   * 256 MB. Must be finite and non-negative.
   *
   * The reader always takes the SMALLER of this and the size the archive
   * declares, so lowering it is always safe and raising it is the only way to
   * read an entry bigger than the default.
   *
   * `Infinity` is rejected rather than read as "no limit". It would pass the
   * `Math.min` against the archive's declared size and leave the CEILING equal
   * to four bytes the archive chose -- which is precisely the attacker-chosen
   * limit this option exists to prevent. To read something enormous, name a
   * number.
   *
   * Stored entries (method 0) are not affected: their bytes are already resident
   * in the archive, so there is no amplification to bound.
   */
  maxOutput?: number;
}

export class ZipReader {
  private readonly entries_: ZipEntry[] = [];
  private readonly maxOutput: number;

  constructor(private readonly data: Uint8Array, options: ZipReaderOptions = {}) {
    const cap = options.maxOutput ?? MAX_OUTPUT;
    // Validated HERE rather than left to the inflater, so both entry points
    // treat the same value the same way. NaN and negatives would propagate
    // through `Math.min` and be caught downstream; Infinity would NOT -- it
    // would leave the archive's own declared size as the effective ceiling.
    if (!Number.isFinite(cap) || cap < 0) {
      throw new Error("zip: maxOutput must be a non-negative finite number");
    }
    this.maxOutput = cap;
    const eocdOffset = this.findEOCD();
    if (eocdOffset === null) throw new Error("zip: no End of Central Directory record found");

    const cdOffset = readLE32(data, eocdOffset + 16);
    const cdSize = readLE32(data, eocdOffset + 12);
    if (cdOffset === null || cdSize === null) throw new Error("zip: EOCD too short");
    if (cdOffset + cdSize > data.length) throw new Error(`zip: Central Directory out of bounds`);

    let pos = cdOffset;
    while (pos + 4 <= cdOffset + cdSize) {
      const sig = readLE32(data, pos);
      if (sig !== 0x02014b50) break;

      const method = readLE16(data, pos + 10)!;
      const crc32v = readLE32(data, pos + 16)!;
      const compressedSize = readLE32(data, pos + 20)!;
      const size = readLE32(data, pos + 24)!;
      const nameLen = readLE16(data, pos + 28)!;
      const extraLen = readLE16(data, pos + 30)!;
      const commentLen = readLE16(data, pos + 32)!;
      const localOffset = readLE32(data, pos + 42)!;

      const nameStart = pos + 46;
      const nameEnd = nameStart + nameLen;
      if (nameEnd > data.length) throw new Error("zip: CD entry name out of bounds");
      const name = new TextDecoder().decode(data.slice(nameStart, nameEnd));

      this.entries_.push({ name, size, compressedSize, method, crc32: crc32v, isDirectory: name.endsWith("/"), localOffset });
      pos = nameEnd + extraLen + commentLen;
    }
  }

  entries(): ZipEntry[] { return [...this.entries_]; }

  read(entry: ZipEntry): Uint8Array {
    if (entry.isDirectory) return new Uint8Array(0);

    const localFlags = readLE16(this.data, entry.localOffset + 6);
    if (localFlags === null) throw new Error("zip: local header out of bounds");
    if (localFlags & 1) throw new Error(`zip: entry '${entry.name}' is encrypted`);

    const lhNameLen = readLE16(this.data, entry.localOffset + 26)!;
    const lhExtraLen = readLE16(this.data, entry.localOffset + 28)!;
    const dataStart = entry.localOffset + 30 + lhNameLen + lhExtraLen;
    const dataEnd = dataStart + entry.compressedSize;
    if (dataEnd > this.data.length) throw new Error(`zip: entry '${entry.name}' data out of bounds`);

    const compressed = this.data.slice(dataStart, dataEnd);

    let decompressed: Uint8Array;
    if (entry.method === 0) {
      decompressed = compressed;
    } else if (entry.method === 8) {
      // The central directory already told us how big this entry decompresses
      // to, and the code below truncates to it anyway -- so inflating up to the
      // global ceiling first would be doing a zip bomb's work for it.
      //
      // But `entry.size` is four bytes the ARCHIVE chose, not a fact: it is read
      // straight off the central directory and can say 4 GiB. Trusting it alone
      // would replace a fixed ceiling with an attacker-chosen one, and the
      // CRC-32 that finally catches the lie only runs after the memory has
      // already been committed. So the declared size is an OPTIMISATION and the
      // reader's own ceiling stays the LIMIT; whichever is smaller wins.
      decompressed = deflateDecompress(compressed, Math.min(entry.size, this.maxOutput)).output;
    } else {
      throw new Error(`zip: unsupported compression method ${entry.method} for '${entry.name}'`);
    }

    if (decompressed.length > entry.size) {
      decompressed = decompressed.slice(0, entry.size);
    }

    const actualCRC = crc32(decompressed);
    if (actualCRC !== entry.crc32) {
      throw new Error(`zip: CRC-32 mismatch for '${entry.name}': expected ${entry.crc32.toString(16)}, got ${actualCRC.toString(16)}`);
    }

    return decompressed;
  }

  readByName(name: string): Uint8Array {
    const entry = this.entries_.find(e => e.name === name);
    if (!entry) throw new Error(`zip: entry '${name}' not found`);
    return this.read(entry);
  }

  private findEOCD(): number | null {
    const eocdSig = 0x06054b50;
    const maxComment = 65535;
    const eocdMinSize = 22;
    const data = this.data;
    if (data.length < eocdMinSize) return null;
    const scanStart = Math.max(0, data.length - eocdMinSize - maxComment);
    for (let i = data.length - eocdMinSize; i >= scanStart; i--) {
      if (readLE32(data, i) === eocdSig) {
        const commentLen = readLE16(data, i + 20);
        if (commentLen !== null && i + eocdMinSize + commentLen === data.length) return i;
      }
    }
    return null;
  }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/**
 * Compress a list of `(name, data)` pairs into a ZIP archive.
 *
 * @example
 * const archive = zipBytes([["hello.txt", new TextEncoder().encode("Hello!")]]);
 */
export function zipBytes(entries: Array<[string, Uint8Array]>, compress = true): Uint8Array {
  const w = new ZipWriter();
  for (const [name, data] of entries) w.addFile(name, data, compress);
  return w.finish();
}

/**
 * Decompress all file entries from a ZIP archive.
 *
 * @example
 * const files = unzip(archive);
 * files.get("hello.txt")  // Uint8Array
 */
export function unzip(data: Uint8Array): Map<string, Uint8Array> {
  const reader = new ZipReader(data);
  const out = new Map<string, Uint8Array>();
  for (const entry of reader.entries()) {
    if (!entry.isDirectory) out.set(entry.name, reader.read(entry));
  }
  return out;
}

// =============================================================================
// Little-endian helpers
// =============================================================================

function readLE16(data: Uint8Array, offset: number): number | null {
  if (offset + 2 > data.length) return null;
  return (data[offset]! | (data[offset + 1]! << 8)) & 0xffff;
}

function readLE32(data: Uint8Array, offset: number): number | null {
  if (offset + 4 > data.length) return null;
  return ((data[offset]! | (data[offset + 1]! << 8) | (data[offset + 2]! << 16) | (data[offset + 3]! << 24)) >>> 0);
}
