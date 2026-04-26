/**
 * zstd.ts — Zstandard (ZStd) lossless compression algorithm — CMP07.
 *
 * Zstandard (RFC 8878) is a high-ratio, fast compression format created by
 * Yann Collet at Facebook (2015). It combines:
 *
 *   - **LZ77 back-references** (via LZSS token generation) to exploit
 *     repetition in the data — the same "copy from earlier in the output"
 *     trick as DEFLATE, but with a 32 KB window.
 *   - **FSE (Finite State Entropy)** coding instead of Huffman for the
 *     sequence descriptor symbols. FSE is an asymmetric numeral system that
 *     approaches the Shannon entropy limit in a single pass.
 *   - **Predefined decode tables** (RFC 8878 Appendix B) so short frames
 *     need no table description overhead.
 *
 * Frame layout (RFC 8878 §3):
 * ```
 * ┌────────┬─────┬──────────────────────┬────────┬──────────────────┐
 * │ Magic  │ FHD │ Frame_Content_Size   │ Blocks │ [Checksum]       │
 * │ 4 B LE │ 1 B │ 1/2/4/8 B (LE)      │ ...    │ 4 B (optional)   │
 * └────────┴─────┴──────────────────────┴────────┴──────────────────┘
 * ```
 *
 * Each **block** has a 3-byte header:
 * ```
 * bit 0       = Last_Block flag
 * bits [2:1]  = Block_Type  (00=Raw, 01=RLE, 10=Compressed, 11=Reserved)
 * bits [23:3] = Block_Size
 * ```
 *
 * Compression strategy (this implementation):
 * 1. Split data into 128 KB blocks (MAX_BLOCK_SIZE).
 * 2. For each block, try:
 *    a. **RLE** — all bytes identical → 4 bytes total.
 *    b. **Compressed** (LZ77 + FSE) — if output < input length.
 *    c. **Raw** — verbatim copy as fallback.
 *
 * Series:
 * ```
 * CMP00 (LZ77)     — Sliding-window back-references
 * CMP01 (LZ78)     — Explicit dictionary (trie)
 * CMP02 (LZSS)     — LZ77 + flag bits
 * CMP03 (LZW)      — LZ78 + pre-initialised alphabet; GIF
 * CMP04 (Huffman)  — Entropy coding
 * CMP05 (DEFLATE)  — LZ77 + Huffman; ZIP/gzip/PNG/zlib
 * CMP06 (Brotli)   — DEFLATE + context modelling + static dict
 * CMP07 (ZStd)     — LZ77 + FSE; high ratio + speed  ← this file
 * ```
 *
 * @example
 * ```ts
 * import { compress, decompress } from "@coding-adventures/zstd";
 * const data = new TextEncoder().encode("the quick brown fox jumps over the lazy dog");
 * const compressed = compress(data);
 * const original   = decompress(compressed);
 * ```
 */

import { encode as lzssEncode, type Token as LzssToken } from "@coding-adventures/lzss";

// ─── Constants ────────────────────────────────────────────────────────────────

/**
 * ZStd magic number: `0xFD2FB528` (little-endian: `28 B5 2F FD`).
 *
 * Every valid ZStd frame starts with these 4 bytes. The value was chosen to be
 * unlikely to appear at the start of plain-text files.
 */
const MAGIC = 0xFD2FB528;

/**
 * Maximum block size: 128 KB.
 *
 * ZStd allows blocks up to 128 KB. Larger inputs are split across multiple
 * blocks. The spec maximum is `min(WindowSize, 128 KB)`.
 */
const MAX_BLOCK_SIZE = 128 * 1024;

/**
 * Hard output-size cap to guard against decompression bombs.
 *
 * 256 MB is generous for educational use. Real ZStd decoders use the
 * Frame_Content_Size field or dynamic allocation.
 */
const MAX_OUTPUT = 256 * 1024 * 1024;

// ─── LL / ML / OF code tables (RFC 8878 §3.1.1.3) ────────────────────────────
//
// These tables map a *code number* to a (baseline, extra_bits) pair.
//
// For example, LL code 17 means literal_length = 18 + read(1 extra bit),
// covering literal lengths 18 and 19.
//
// The FSE state machine tracks one code number per field; extra bits are
// read directly from the bitstream after state transitions.

/**
 * Literal Length code table: `[baseline, extraBits]` for codes 0..35.
 *
 * Codes 0–15 map directly to the same literal length (0 extra bits).
 * Codes 16+ cover increasing ranges via extra bits.
 *
 * Truth table excerpt:
 * | code | baseline | extraBits | covers  |
 * |------|----------|-----------|---------|
 * |  0   |    0     |     0     | 0       |
 * | 15   |   15     |     0     | 15      |
 * | 16   |   16     |     1     | 16–17   |
 * | 24   |   24     |     2     | 24–27   |
 */
const LL_CODES: ReadonlyArray<readonly [number, number]> = [
  [0, 0], [1, 0], [2, 0], [3, 0], [4, 0], [5, 0],
  [6, 0], [7, 0], [8, 0], [9, 0], [10, 0], [11, 0],
  [12, 0], [13, 0], [14, 0], [15, 0],
  // Grouped ranges start at code 16
  [16, 1], [18, 1], [20, 1], [22, 1],
  [24, 2], [28, 2],
  [32, 3], [40, 3],
  [48, 4], [64, 6],
  [128, 7], [256, 8], [512, 9], [1024, 10], [2048, 11], [4096, 12],
  [8192, 13], [16384, 14], [32768, 15], [65536, 16],
] as const;

/**
 * Match Length code table: `[baseline, extraBits]` for codes 0..52.
 *
 * Minimum match length in ZStd is 3. Code 0 = match length 3.
 *
 * Truth table excerpt:
 * | code | baseline | extraBits | covers  |
 * |------|----------|-----------|---------|
 * |  0   |    3     |     0     | 3       |
 * | 31   |   34     |     0     | 34      |
 * | 32   |   35     |     1     | 35–36   |
 * | 44   |   99     |     5     | 99–130  |
 */
const ML_CODES: ReadonlyArray<readonly [number, number]> = [
  // codes 0..31: individual values 3..34
  [3, 0], [4, 0], [5, 0], [6, 0], [7, 0], [8, 0],
  [9, 0], [10, 0], [11, 0], [12, 0], [13, 0], [14, 0],
  [15, 0], [16, 0], [17, 0], [18, 0], [19, 0], [20, 0],
  [21, 0], [22, 0], [23, 0], [24, 0], [25, 0], [26, 0],
  [27, 0], [28, 0], [29, 0], [30, 0], [31, 0], [32, 0],
  [33, 0], [34, 0],
  // codes 32+: grouped ranges
  [35, 1], [37, 1], [39, 1], [41, 1],
  [43, 2], [47, 2],
  [51, 3], [59, 3],
  [67, 4], [83, 4],
  [99, 5], [131, 7],
  [259, 8], [515, 9], [1027, 10], [2051, 11],
  [4099, 12], [8195, 13], [16387, 14], [32771, 15], [65539, 16],
] as const;

// ─── FSE predefined distributions (RFC 8878 Appendix B) ──────────────────────
//
// "Predefined_Mode" means no per-frame table description is transmitted.
// The decoder builds the same table from these fixed distributions.
//
// An entry of -1 means "probability 1/table_size" — the symbol gets one slot
// in the decode table, and its encoder state never needs extra bits.

/**
 * Predefined normalised distribution for Literal Length FSE.
 * Table accuracy log = 6 → 64 slots.
 *
 * The values sum to 64 (with -1 entries counting as 1 each).
 */
const LL_NORM: ReadonlyArray<number> = [
   4,  3,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  2,  1,  1,  1,
   2,  2,  2,  2,  2,  2,  2,  2,  2,  3,  2,  1,  1,  1,  1,  1,
  -1, -1, -1, -1,
];
const LL_ACC_LOG = 6; // table_size = 64

/**
 * Predefined normalised distribution for Match Length FSE.
 * Table accuracy log = 6 → 64 slots.
 */
const ML_NORM: ReadonlyArray<number> = [
   1,  4,  3,  2,  2,  2,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
   1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,
   1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1,  1, -1, -1,
  -1, -1, -1, -1, -1,
];
const ML_ACC_LOG = 6;

/**
 * Predefined normalised distribution for Offset FSE.
 * Table accuracy log = 5 → 32 slots.
 */
const OF_NORM: ReadonlyArray<number> = [
   1,  1,  1,  1,  1,  1,  2,  2,  2,  1,  1,  1,  1,  1,  1,  1,
   1,  1,  1,  1,  1,  1,  1,  1, -1, -1, -1, -1, -1,
];
const OF_ACC_LOG = 5; // table_size = 32

// ─── FSE decode table entry ───────────────────────────────────────────────────

/**
 * One cell in the FSE decode table.
 *
 * To decode a symbol from state S:
 *   1. `sym` is the output symbol.
 *   2. Read `nb` bits from the bitstream as `bits`.
 *   3. New state = `base + bits`.
 *
 * Analogy: think of FSE as a vending machine. The current state S selects
 * which product (`sym`) comes out. Then you insert `nb` coins (bits) to get
 * the machine to its next state.
 */
interface FseDe {
  sym: number;   // decoded symbol (0–255)
  nb: number;    // number of extra bits to read for the next state
  base: number;  // base value for the next state computation
}

/**
 * Encode transform for one symbol.
 *
 * Given encoder state S for symbol `s`:
 *   nb_out  = (S + deltaNb) >> 16    (number of bits to emit)
 *   emit low nb_out bits of S
 *   new_S   = stateTbl[(S >> nb_out) + deltaFs]
 *
 * The `deltaNb` and `deltaFs` values are precomputed from the distribution
 * so the hot-path encode loop needs only arithmetic and a table lookup.
 */
interface FseEe {
  /**
   * `(maxBitsOut << 16) - (count << maxBitsOut)`.
   * Used to derive nb_out: `nb_out = (state + deltaNb) >> 16`
   */
  deltaNb: number;
  /**
   * `cumulativeCountBeforeSym - count` (may be negative).
   * Used to index stateTbl: `new_S = stateTbl[(S >> nb_out) + deltaFs]`
   */
  deltaFs: number;
}

// ─── FSE table construction ───────────────────────────────────────────────────

/**
 * Build an FSE decode table from a normalised probability distribution.
 *
 * Algorithm overview:
 *  1. Place symbols with probability -1 (very rare) at the top of the table.
 *  2. Spread remaining symbols using a deterministic step function derived
 *     from the table size. This ensures each symbol occupies the correct
 *     fraction of slots.
 *  3. Assign `nb` (number of state bits) and `base` to each slot so that
 *     the decoder can reconstruct the next state.
 *
 * The step function `step = (sz >> 1) + (sz >> 3) + 3` is co-prime to `sz`
 * when `sz` is a power of two (which it always is in ZStd), ensuring the
 * walk visits every slot exactly once.
 *
 * @param norm  Normalised probability array (entries sum to `1 << accLog`).
 * @param accLog  Accuracy log: table size = `1 << accLog`.
 */
function buildDecodeTable(norm: ReadonlyArray<number>, accLog: number): FseDe[] {
  const sz = 1 << accLog;
  // step is co-prime to sz (a power of two) ensuring a complete cycle
  const step = (sz >> 1) + (sz >> 3) + 3;
  const tbl: FseDe[] = new Array(sz).fill(null).map(() => ({ sym: 0, nb: 0, base: 0 }));
  const symNext: number[] = new Array(norm.length).fill(0);

  // Phase 1: symbols with probability -1 go at the top (high indices).
  // These rare symbols each get exactly 1 slot. Their state transition uses
  // the full accLog bits — they can transition to any state.
  let high = sz - 1;
  for (let s = 0; s < norm.length; s++) {
    if (norm[s] === -1) {
      tbl[high]!.sym = s;
      if (high > 0) high--;
      symNext[s] = 1;
    }
  }

  // Phase 2: spread remaining symbols into the lower portion of the table.
  // Two-pass approach: first symbols with count > 1, then count == 1.
  // This matches the reference implementation's deterministic ordering.
  let pos = 0;
  for (let pass = 0; pass < 2; pass++) {
    for (let s = 0; s < norm.length; s++) {
      const c = norm[s]!;
      if (c <= 0) continue;
      const cnt = c;
      // pass 0 handles cnt > 1, pass 1 handles cnt == 1
      if ((pass === 0) !== (cnt > 1)) continue;
      symNext[s] = cnt;
      for (let i = 0; i < cnt; i++) {
        tbl[pos]!.sym = s;
        pos = (pos + step) & (sz - 1);
        while (pos > high) {
          pos = (pos + step) & (sz - 1);
        }
      }
    }
  }

  // Phase 3: assign nb (bits to read) and base to each slot.
  //
  // For a symbol with count `cnt` occupying slots at indices i₀, i₁, ...:
  //   symNext[s] counts how many times we've assigned this symbol so far.
  //   Start at cnt, increment each time.
  //   nb   = accLog - floor(log2(ns))
  //   base = ns * (1 << nb) - sz
  //
  // This ensures that when we reconstruct state = base + read(nb bits),
  // we land in [sz, 2*sz), the valid encoder state range.
  const sn = [...symNext]; // copy to use as running counter
  for (let i = 0; i < sz; i++) {
    const s = tbl[i]!.sym;
    const ns = sn[s]!;
    sn[s] = ns + 1;
    // floor(log2(ns)) = 31 - Math.clz32(ns)
    const nb = accLog - (31 - Math.clz32(ns));
    const base = (ns << nb) - sz;
    tbl[i]!.nb = nb;
    tbl[i]!.base = base;
  }

  return tbl;
}

/**
 * Build FSE encode tables from a normalised distribution.
 *
 * Returns:
 * - `ee[sym]`: the FseEe transform for each symbol.
 * - `st[slot]`: the encoder state table (slot → output state in [sz, 2*sz)).
 *
 * The FSE encode/decode symmetry:
 * - The decoder assigns (sym, nb, base) to each cell in INDEX ORDER.
 * - For symbol `s`, the j-th cell (ascending index) has `ns = count[s] + j`.
 * - The encoder must use the SAME indexing: slot `cumul[s] + j` maps to
 *   the j-th cell for symbol `s`.
 * - The encoder state after symbol `s` from slot `cumul[s] + j` is
 *   `(j-th cell index for s) + sz`. The decoder at that index reconstructs
 *   the pre-encoding state via `base + read(nb)`.
 */
function buildEncodeTable(norm: ReadonlyArray<number>, accLog: number): [FseEe[], number[]] {
  const sz = 1 << accLog;

  // Step 1: compute cumulative sums.
  // cumul[s] = number of slots before symbol s in the encoder state table.
  const cumul: number[] = new Array(norm.length).fill(0);
  let total = 0;
  for (let s = 0; s < norm.length; s++) {
    cumul[s] = total;
    const c = norm[s]!;
    const cnt = c === -1 ? 1 : Math.max(0, c);
    total += cnt;
  }

  // Step 2: build the spread table (same spreading algorithm as decode table).
  const step = (sz >> 1) + (sz >> 3) + 3;
  const spread: number[] = new Array(sz).fill(0);
  let idxHigh = sz - 1;

  // Phase 1: probability -1 symbols at the high end
  for (let s = 0; s < norm.length; s++) {
    if (norm[s] === -1) {
      spread[idxHigh] = s;
      if (idxHigh > 0) idxHigh--;
    }
  }
  const idxLimit = idxHigh;

  // Phase 2: spread remaining symbols
  let pos2 = 0;
  for (let pass = 0; pass < 2; pass++) {
    for (let s = 0; s < norm.length; s++) {
      const c = norm[s]!;
      if (c <= 0) continue;
      const cnt = c;
      if ((pass === 0) !== (cnt > 1)) continue;
      for (let i = 0; i < cnt; i++) {
        spread[pos2] = s;
        pos2 = (pos2 + step) & (sz - 1);
        while (pos2 > idxLimit) {
          pos2 = (pos2 + step) & (sz - 1);
        }
      }
    }
  }

  // Step 3: build the state table by iterating spread in INDEX ORDER.
  //
  // For each table index `i`, determine which occurrence j of symbol
  // `s = spread[i]` this is. The encode slot = `cumul[s] + j`.
  // Encoder output state = `i + sz` (decoder in state `i` decodes sym s).
  const symOcc: number[] = new Array(norm.length).fill(0);
  const st: number[] = new Array(sz).fill(0);

  for (let i = 0; i < sz; i++) {
    const s = spread[i]!;
    const j = symOcc[s]!;
    symOcc[s] = j + 1;
    const slot = cumul[s]! + j;
    st[slot] = i + sz; // output state = decode index + sz
  }

  // Step 4: build FseEe entries.
  //
  // For symbol s with count c and maxBitsOut mbo:
  //   deltaNb = (mbo << 16) - (c << mbo)
  //   deltaFs = cumul[s] - c
  //
  // Encode step: given encoder state E in [sz, 2*sz):
  //   nb   = (E + deltaNb) >> 16
  //   emit low nb bits of E
  //   new_E = st[(E >> nb) + deltaFs]
  const ee: FseEe[] = new Array(norm.length).fill(null).map(() => ({ deltaNb: 0, deltaFs: 0 }));
  for (let s = 0; s < norm.length; s++) {
    const c = norm[s]!;
    const cnt = c === -1 ? 1 : Math.max(0, c);
    if (cnt === 0) continue;
    // maxBitsOut = ceil(log2(sz / cnt)) = accLog - floor(log2(cnt))
    let mbo: number;
    if (cnt === 1) {
      mbo = accLog;
    } else {
      mbo = accLog - (31 - Math.clz32(cnt));
    }
    ee[s]!.deltaNb = (mbo << 16) - (cnt << mbo);
    ee[s]!.deltaFs = cumul[s]! - cnt;
  }

  return [ee, st];
}

// ─── Reverse bit-writer ───────────────────────────────────────────────────────
//
// ZStd's sequence bitstream is written *backwards* relative to the data flow:
// the encoder writes bits that the decoder will read last, first. This allows
// the decoder to read a forward-only stream while decoding sequences in order.
//
// Byte layout: `[byte0, byte1, ..., byteN]` where `byteN` is the last byte
// written, and it contains a **sentinel bit** (the highest set bit) that marks
// the end of meaningful data. The decoder initialises by finding this sentinel.
//
// Bit layout within each byte: LSB = first bit written.
//
// Example: write bits `1, 0, 1, 1` (4 bits) then flush:
//   reg = 0b1011, bits = 4
//   flush: sentinel at bit 4 → last byte = 0b0001_1011 = 0x1B
//   buf = [0x1B]
//
// The decoder reads: find MSB (bit 4 = sentinel), then read bits 3..0 = 0b1011.
//
// Why BigInt? JavaScript's number type is a 64-bit float, which gives only 53
// bits of integer precision. The bit register can hold up to 64 bits before
// we flush, so we need BigInt for correctness. All shifts and masks below
// use BigInt literals (1n, 0xFFn, etc.) to keep arithmetic in 64-bit land.

class RevBitWriter {
  private buf: number[] = [];
  private reg: bigint = 0n;  // accumulation register (bits fill from LSB)
  private bits = 0;          // number of valid bits in reg

  /**
   * Add the low-order `nb` bits of `val` to the stream.
   *
   * Bits are accumulated LSB-first. When the register fills a full byte,
   * that byte is pushed to the output buffer and the register shifts right.
   */
  addBits(val: bigint, nb: number): void {
    if (nb === 0) return;
    const mask = (1n << BigInt(nb)) - 1n;
    this.reg |= (val & mask) << BigInt(this.bits);
    this.bits += nb;
    while (this.bits >= 8) {
      this.buf.push(Number(this.reg & 0xFFn));
      this.reg >>= 8n;
      this.bits -= 8;
    }
  }

  /**
   * Flush remaining bits with a sentinel and mark the stream end.
   *
   * The sentinel is a `1` bit placed at position `this.bits` in the
   * last byte. The decoder locates it with `Math.clz32` arithmetic.
   *
   * Example: if bits=3 and reg=0b101, then:
   *   sentinel = 1 << 3 = 0b1000
   *   last byte = 0b1000 | 0b0101 = 0b1101 = 0x0D
   */
  flush(): void {
    const sentinel = 1 << this.bits; // bit above all remaining data bits
    const lastByte = (Number(this.reg) & 0xFF) | sentinel;
    this.buf.push(lastByte);
    this.reg = 0n;
    this.bits = 0;
  }

  finish(): Uint8Array {
    return new Uint8Array(this.buf);
  }
}

// ─── Reverse bit-reader ───────────────────────────────────────────────────────
//
// Mirrors RevBitWriter: reads bits from the END of the buffer going backwards.
// The stream is laid out so the LAST bits written are at the END of the buffer
// (in the sentinel-containing last byte). The reader initialises at the last
// byte and reads backward toward byte 0.
//
// Register layout: valid bits are LEFT-ALIGNED (packed into the MSB side).
// `readBits(n)` extracts the top n bits and shifts the register left by n.
//
// Why left-aligned? The writer accumulates bits LSB-first. Within each flushed
// byte, bit 0 = earliest written, bit N = latest written. To read the LATEST
// bits first (which were in the highest byte positions / high bits), we need a
// left-aligned register so reading from the top gives highest-position bits
// first.

class RevBitReader {
  private reg: bigint = 0n;  // shift register, valid bits packed at the TOP (MSB side)
  private bits = 0;          // how many valid bits are loaded
  private pos: number;       // index of the next byte to load (decrements toward 0)

  constructor(private readonly data: Uint8Array) {
    if (data.length === 0) throw new Error("RevBitReader: empty bitstream");

    const last = data[data.length - 1]!;
    if (last === 0) throw new Error("RevBitReader: last byte is zero (no sentinel)");

    // Find the sentinel bit: it's the highest set bit in the last byte.
    // sentinelPos = bit index (0=LSB) of the sentinel.
    // floor(log2(last)) = 31 - Math.clz32(last)
    const sentinelPos = 31 - Math.clz32(last); // = floor(log2(last))
    // Data bits are the bits BELOW the sentinel.
    const validBits = sentinelPos;

    // Place the valid bits of the sentinel byte at the TOP of the register.
    // Example: last=0b00011110, sentinel at bit4, validBits=4.
    //   data bits = last & 0b1111 = 0b1110.
    //   After left-shifting to fill top: reg = 0b1110 << (64-4) = left-packed.
    const mask = validBits > 0 ? (1n << BigInt(validBits)) - 1n : 0n;
    this.reg = validBits > 0
      ? (BigInt(last) & mask) << BigInt(64 - validBits)
      : 0n;
    this.bits = validBits;
    this.pos = data.length - 1; // sentinel byte consumed; load from here-1

    this.reload();
  }

  /**
   * Load more bytes into the register from the stream going backward.
   *
   * Each new byte is placed just BELOW the currently loaded bits (i.e., at
   * position `64 - bits - 8` in the left-aligned register).
   */
  private reload(): void {
    while (this.bits <= 56 && this.pos > 0) {
      this.pos--;
      // Place this byte just below existing bits.
      const shift = 64 - this.bits - 8;
      this.reg |= BigInt(this.data[this.pos]!) << BigInt(shift);
      this.bits += 8;
    }
  }

  /**
   * Read `nb` bits from the top of the register (returns 0n if nb === 0).
   *
   * This returns the most recently written bits first (highest stream
   * positions first), mirroring the encoder's backward order.
   */
  readBits(nb: number): bigint {
    if (nb === 0) return 0n;
    // Extract the top `nb` bits.
    const val = this.reg >> BigInt(64 - nb);
    // Shift the register left to consume those bits.
    this.reg = nb === 64 ? 0n : (this.reg << BigInt(nb)) & ((1n << 64n) - 1n);
    this.bits = Math.max(0, this.bits - nb);
    if (this.bits < 24) this.reload();
    return val;
  }
}

// ─── FSE encode/decode helpers ────────────────────────────────────────────────

/**
 * Encode one symbol into the backward bitstream, updating the FSE state.
 *
 * The encoder maintains state in `[sz, 2*sz)`. To emit symbol `sym`:
 *   1. Compute how many bits to flush: `nb = (state + deltaNb) >> 16`
 *   2. Write the low `nb` bits of `state` to the bitstream.
 *   3. New state = `st[(state >> nb) + deltaFs]`
 *
 * After all symbols are encoded, the final state (minus `sz`) is written as
 * `accLog` bits to allow the decoder to initialise.
 *
 * @param state  Current encoder state (in [sz, 2*sz)), mutated in place.
 * @param sym    Symbol to encode (index into ee[]).
 * @param ee     Encode transform table (one entry per symbol).
 * @param st     Encoder state table (slot → output state).
 * @param bw     Reverse bit writer.
 * @returns New state value.
 */
function fseEncodeSym(
  state: number,
  sym: number,
  ee: FseEe[],
  st: number[],
  bw: RevBitWriter,
): number {
  const e = ee[sym]!;
  // Compute how many bits to emit from the current state.
  const nb = ((state + e.deltaNb) >>> 16);
  bw.addBits(BigInt(state), nb);
  // Compute the new state using the state table.
  const slotI = (state >>> nb) + e.deltaFs;
  const slot = Math.max(0, slotI);
  return st[slot]!;
}

/**
 * Decode one symbol from the backward bitstream, updating the FSE state.
 *
 *   1. Look up `de[state]` to get `sym`, `nb`, and `base`.
 *   2. New state = `base + read(nb bits)`.
 *
 * @param state  Current decoder state (index into de[]), mutated in place.
 * @param de     Decode table.
 * @param br     Reverse bit reader.
 * @returns [decoded symbol, new state].
 */
function fseDecodeSym(state: number, de: FseDe[], br: RevBitReader): [number, number] {
  const e = de[state]!;
  const sym = e.sym;
  const nextState = e.base + Number(br.readBits(e.nb));
  return [sym, nextState];
}

// ─── LL/ML/OF code number computation ────────────────────────────────────────

/**
 * Map a literal-length value to its LL code number (0..35).
 *
 * Codes 0–15 are identity. Codes 16+ cover increasing ranges.
 * We scan from the start and track the last code whose baseline ≤ ll.
 */
function llToCode(ll: number): number {
  let code = 0;
  for (let i = 0; i < LL_CODES.length; i++) {
    if (LL_CODES[i]![0] <= ll) {
      code = i;
    } else {
      break;
    }
  }
  return code;
}

/**
 * Map a match-length value to its ML code number (0..52).
 */
function mlToCode(ml: number): number {
  let code = 0;
  for (let i = 0; i < ML_CODES.length; i++) {
    if (ML_CODES[i]![0] <= ml) {
      code = i;
    } else {
      break;
    }
  }
  return code;
}

// ─── Sequence struct ──────────────────────────────────────────────────────────

/**
 * One ZStd sequence: (literal_length, match_length, match_offset).
 *
 * A sequence means: emit `ll` literal bytes from the literals section,
 * then copy `ml` bytes starting `off` positions back in the output buffer.
 * After all sequences, any remaining literals are appended.
 *
 * Example: input = "abcabc"
 *   lits = [a, b, c]
 *   seqs = [{ll:3, ml:3, off:3}]   (copy 3 bytes from 3 back)
 */
interface Seq {
  ll: number;   // literal length
  ml: number;   // match length
  off: number;  // match offset (1-indexed: 1 = last byte written)
}

/**
 * Convert LZSS tokens into ZStd sequences + a flat literals buffer.
 *
 * LZSS produces a stream of `{kind:"literal", byte}` and `{kind:"match", offset, length}`.
 * ZStd groups consecutive literals before each match into a single sequence.
 * Trailing literals (after the last match) go into the buffer without a sequence.
 */
function tokensToSeqs(tokens: LzssToken[]): [Uint8Array, Seq[]] {
  const lits: number[] = [];
  const seqs: Seq[] = [];
  let litRun = 0;

  for (const tok of tokens) {
    if (tok.kind === "literal") {
      lits.push(tok.byte);
      litRun++;
    } else {
      seqs.push({ ll: litRun, ml: tok.length, off: tok.offset });
      litRun = 0;
    }
  }
  // Trailing literals have no sequence; they're emitted after the last sequence
  // during decompression.
  return [new Uint8Array(lits), seqs];
}

// ─── Literals section encoding ────────────────────────────────────────────────
//
// ZStd literals can be Huffman-coded or raw. We use **Raw_Literals** (type=0),
// which is the simplest: no Huffman table, bytes stored verbatim.
//
// Header format depends on literal count:
//   ≤ 31 bytes:   1-byte header  = (lit_len << 3) | 0b000
//   ≤ 4095 bytes: 2-byte LE header = (lit_len << 4) | 0b0100
//   else:         3-byte LE header = (lit_len << 4) | 0b1100
//
// The bottom 2 bits = Literals_Block_Type (0 = Raw).
// The next 2 bits = Size_Format.

/**
 * Encode a raw literals section.
 *
 * Raw_Literals header (RFC 8878 §3.1.1.2.1):
 *   bits [1:0] = Literals_Block_Type = 00 (Raw)
 *   bits [3:2] = Size_Format: 00=1-byte, 01=2-byte, 11=3-byte
 */
function encodeLiteralsSection(lits: Uint8Array): number[] {
  const n = lits.length;
  const out: number[] = [];

  if (n <= 31) {
    // 1-byte header: size_format=00, type=00
    out.push((n << 3) & 0xFF);
  } else if (n <= 4095) {
    // 2-byte LE header: size_format=01, type=00 → bottom nibble = 0b0100
    const hdr = (n << 4) | 0b0100;
    out.push(hdr & 0xFF, (hdr >>> 8) & 0xFF);
  } else {
    // 3-byte LE header: size_format=11, type=00 → bottom nibble = 0b1100
    const hdr = (n << 4) | 0b1100;
    out.push(hdr & 0xFF, (hdr >>> 8) & 0xFF, (hdr >>> 16) & 0xFF);
  }

  for (const b of lits) out.push(b);
  return out;
}

/**
 * Decode a raw literals section.
 *
 * @returns `[literals, bytesConsumed]`
 */
function decodeLiteralsSection(data: Uint8Array, offset: number): [Uint8Array, number] {
  if (offset >= data.length) throw new Error("empty literals section");

  const b0 = data[offset]!;
  const ltype = b0 & 0b11; // bottom 2 bits = Literals_Block_Type

  if (ltype !== 0) {
    throw new Error(`unsupported literals type ${ltype} (only Raw=0 supported)`);
  }

  const sizeFormat = (b0 >>> 2) & 0b11;

  let n: number;
  let headerBytes: number;

  // Raw_Literals size_format encoding:
  //   0b00 or 0b10 → 1-byte header: size = b0[7:3] (5 bits, values 0..31)
  //   0b01          → 2-byte LE header: size in bits [11:4] (12 bits, 0..4095)
  //   0b11          → 3-byte LE header: size in bits [19:4] (20 bits, 0..1MB)
  if (sizeFormat === 0 || sizeFormat === 2) {
    n = b0 >>> 3;
    headerBytes = 1;
  } else if (sizeFormat === 1) {
    if (offset + 2 > data.length) throw new Error("truncated literals header (2-byte)");
    n = ((b0 >>> 4) & 0xF) | (data[offset + 1]! << 4);
    headerBytes = 2;
  } else {
    // sizeFormat === 3
    if (offset + 3 > data.length) throw new Error("truncated literals header (3-byte)");
    n = ((b0 >>> 4) & 0xF) | (data[offset + 1]! << 4) | (data[offset + 2]! << 12);
    headerBytes = 3;
  }

  const start = offset + headerBytes;
  const end = start + n;
  if (end > data.length) {
    throw new Error(`literals data truncated: need ${end}, have ${data.length}`);
  }

  return [data.slice(start, end), headerBytes + n];
}

// ─── Sequences section encoding ───────────────────────────────────────────────
//
// Layout:
//   [sequence_count: 1-3 bytes]
//   [symbol_compression_modes: 1 byte]  (0x00 = all Predefined)
//   [FSE bitstream: variable]
//
// Symbol compression modes byte:
//   bits [7:6] = LL mode
//   bits [5:4] = OF mode
//   bits [3:2] = ML mode
//   bits [1:0] = reserved (0)
// Mode 0 = Predefined. We always write 0x00.
//
// The FSE bitstream is a backward bit-stream (reverse bit writer):
//   - Sequences are encoded in REVERSE ORDER (last first).
//   - For each sequence:
//       OF extra bits, ML extra bits, LL extra bits  (in this order)
//       then FSE symbol for ML, OF, LL               (reverse of decode order)
//   - After all sequences, flush the final FSE states:
//       (state_of - sz_of) as OF_ACC_LOG bits
//       (state_ml - sz_ml) as ML_ACC_LOG bits
//       (state_ll - sz_ll) as LL_ACC_LOG bits
//   - Add sentinel and flush.
//
// The decoder does the mirror:
//   1. Read LL_ACC_LOG bits → initial state_ll
//   2. Read ML_ACC_LOG bits → initial state_ml
//   3. Read OF_ACC_LOG bits → initial state_of
//   4. For each sequence:
//       decode LL symbol (state transition)
//       decode OF symbol
//       decode ML symbol
//       read LL extra bits
//       read ML extra bits
//       read OF extra bits
//   5. Apply sequence to output buffer.

/** Encode sequence count as 1, 2, or 3 bytes. */
function encodeSeqCount(count: number): number[] {
  // RFC 8878 §3.1.1.3.1 layout — byte0 is the FORMAT MARKER:
  //   byte0 < 128            → 1-byte form, count = byte0
  //   byte0 ∈ [128, 254]     → 2-byte form, count = ((byte0 - 128) << 8) | byte1
  //   byte0 == 0xFF          → 3-byte form, count = byte1 + (byte2 << 8) + 0x7F00
  //
  // The decoder branches on byte0 alone, so the encoder MUST write the byte
  // that determines the form first. The previous implementation wrote
  // `[count & 0xFF, (count >> 8) | 0x80]` (low byte first). For any
  // count ≥ 128 whose low byte happened to be < 128 (e.g. count=515 →
  // byte0=0x03), the decoder mis-took the 1-byte path and returned a tiny
  // garbage count, mis-aligning every byte that followed (including the
  // symbol-modes byte). The bug was silent for counts whose low byte was
  // ≥ 128, which is roughly half — so most existing tests still passed.
  if (count === 0) return [0];
  if (count < 128) return [count];
  if (count < 0x7F00) {
    // 2-byte form: byte0 = (count >> 8) | 0x80, byte1 = count & 0xFF.
    // count < 0x7F00 keeps byte0 in [0x80, 0xFE]; counts at or above 0x7F00
    // fall through to the 3-byte form (byte0 = 0xFF).
    return [(count >>> 8) | 0x80, count & 0xFF];
  }
  // 3-byte: first byte = 0xFF, then (count - 0x7F00) as LE u16
  const r = count - 0x7F00;
  return [0xFF, r & 0xFF, (r >>> 8) & 0xFF];
}

/** Decode sequence count from 1, 2, or 3 bytes. Returns [count, bytesConsumed]. */
function decodeSeqCount(data: Uint8Array, offset: number): [number, number] {
  if (offset >= data.length) throw new Error("empty sequence count");
  const b0 = data[offset]!;
  if (b0 < 128) return [b0, 1];
  if (b0 < 0xFF) {
    if (offset + 2 > data.length) throw new Error("truncated sequence count");
    // Equivalent to `((b0 - 128) << 8) | b1` per RFC 8878 §3.1.1.3.1.
    const count = ((b0 & 0x7F) << 8) | data[offset + 1]!;
    return [count, 2];
  }
  // b0 === 0xFF
  if (offset + 3 > data.length) throw new Error("truncated sequence count (3-byte)");
  const count = 0x7F00 + data[offset + 1]! + (data[offset + 2]! << 8);
  return [count, 3];
}

/**
 * Encode the sequences section using predefined FSE tables.
 *
 * Returns the raw FSE bitstream bytes (not including the count or modes byte).
 */
function encodeSequencesSection(seqs: Seq[]): Uint8Array {
  // Build encode tables (precomputed from predefined distributions).
  const [eeLl, stLl] = buildEncodeTable(LL_NORM, LL_ACC_LOG);
  const [eeMl, stMl] = buildEncodeTable(ML_NORM, ML_ACC_LOG);
  const [eeOf, stOf] = buildEncodeTable(OF_NORM, OF_ACC_LOG);

  const szLl = 1 << LL_ACC_LOG;
  const szMl = 1 << ML_ACC_LOG;
  const szOf = 1 << OF_ACC_LOG;

  // Encoder states start at table_size. Range [sz, 2*sz) maps to slot [0, sz).
  let stateLl = szLl;
  let stateMl = szMl;
  let stateOf = szOf;

  const bw = new RevBitWriter();

  // Encode sequences in REVERSE ORDER (the decoder will see them in forward order).
  for (let si = seqs.length - 1; si >= 0; si--) {
    const seq = seqs[si]!;
    const llCode = llToCode(seq.ll);
    const mlCode = mlToCode(seq.ml);

    // Offset encoding: raw = offset + 3 (RFC 8878 §3.1.1.3.2.1)
    // code = floor(log2(raw)); extra = raw - (1 << code)
    const rawOff = seq.off + 3;
    const ofCode = rawOff <= 1 ? 0 : (31 - Math.clz32(rawOff));
    const ofExtra = rawOff - (1 << ofCode);

    // Write extra bits (OF, ML, LL in this order for the backward stream).
    bw.addBits(BigInt(ofExtra), ofCode);
    const mlExtra = seq.ml - ML_CODES[mlCode]![0];
    bw.addBits(BigInt(mlExtra), ML_CODES[mlCode]![1]);
    const llExtra = seq.ll - LL_CODES[llCode]![0];
    bw.addBits(BigInt(llExtra), LL_CODES[llCode]![1]);

    // FSE encode symbols. Since the backward stream reverses write order,
    // we write the REVERSE of the decode order: ML → OF → LL.
    // Decode order is: LL, OF, ML.
    // Encode (reversed): ML, OF, LL (LL is written last = read first by decoder).
    stateMl = fseEncodeSym(stateMl, mlCode, eeMl, stMl, bw);
    stateOf = fseEncodeSym(stateOf, ofCode, eeOf, stOf, bw);
    stateLl = fseEncodeSym(stateLl, llCode, eeLl, stLl, bw);
  }

  // Flush final states (low accLog bits of state - sz).
  // These are read FIRST by the decoder to initialise its states.
  bw.addBits(BigInt(stateOf - szOf), OF_ACC_LOG);
  bw.addBits(BigInt(stateMl - szMl), ML_ACC_LOG);
  bw.addBits(BigInt(stateLl - szLl), LL_ACC_LOG);
  bw.flush();

  return bw.finish();
}

// ─── Block-level compress ─────────────────────────────────────────────────────

/**
 * Compress one block into ZStd compressed block content.
 *
 * Returns `null` if the compressed form is larger than the input (the caller
 * should use a Raw block instead).
 *
 * Block content layout:
 *   [literals section]
 *   [sequence count: 1–3 bytes]
 *   [symbol modes: 1 byte = 0x00 for all-predefined]
 *   [FSE bitstream]
 */
function compressBlock(block: Uint8Array): Uint8Array | null {
  // Use LZSS to generate LZ77 tokens.
  // Window = 32 KB (ZStd standard window), max match = 255, min = 3.
  const tokens = lzssEncode(block, 32768, 255, 3);

  const [lits, seqs] = tokensToSeqs(tokens);

  // If no sequences were found, LZ77 had nothing to compress.
  // A compressed block with 0 sequences still has overhead — fall back to raw.
  if (seqs.length === 0) return null;

  const out: number[] = [];

  // Encode literals section (Raw_Literals).
  out.push(...encodeLiteralsSection(lits));

  // Encode sequence count.
  out.push(...encodeSeqCount(seqs.length));

  // Symbol compression modes byte: 0x00 = all Predefined.
  out.push(0x00);

  // FSE bitstream.
  const bitstream = encodeSequencesSection(seqs);
  for (const b of bitstream) out.push(b);

  if (out.length >= block.length) return null; // not beneficial
  return new Uint8Array(out);
}

/**
 * Decompress one ZStd compressed block.
 *
 * Reads the literals section, sequences section, and applies the sequences
 * to the output buffer to reconstruct the original data.
 *
 * @param data  The compressed block content (no block header).
 * @param out   Output buffer to append decompressed bytes to.
 */
function decompressBlock(data: Uint8Array, out: number[]): void {
  // ── Literals section ─────────────────────────────────────────────────
  const [lits, litConsumed] = decodeLiteralsSection(data, 0);
  let pos = litConsumed;

  // ── Sequences count ──────────────────────────────────────────────────
  if (pos >= data.length) {
    // Block has only literals, no sequences.
    for (const b of lits) out.push(b);
    return;
  }

  const [nSeqs, scBytes] = decodeSeqCount(data, pos);
  pos += scBytes;

  if (nSeqs === 0) {
    for (const b of lits) out.push(b);
    return;
  }

  // ── Symbol compression modes ─────────────────────────────────────────
  if (pos >= data.length) throw new Error("missing symbol compression modes byte");
  const modesByte = data[pos]!;
  pos++;

  const llMode = (modesByte >>> 6) & 3;
  const ofMode = (modesByte >>> 4) & 3;
  const mlMode = (modesByte >>> 2) & 3;
  if (llMode !== 0 || ofMode !== 0 || mlMode !== 0) {
    throw new Error(
      `unsupported FSE modes: LL=${llMode} OF=${ofMode} ML=${mlMode} (only Predefined=0 supported)`
    );
  }

  // ── FSE bitstream ────────────────────────────────────────────────────
  const bitstream = data.subarray(pos);
  const br = new RevBitReader(bitstream);

  // Build decode tables from predefined distributions.
  const dtLl = buildDecodeTable(LL_NORM, LL_ACC_LOG);
  const dtMl = buildDecodeTable(ML_NORM, ML_ACC_LOG);
  const dtOf = buildDecodeTable(OF_NORM, OF_ACC_LOG);

  // Initialise FSE states from the bitstream.
  // The encoder wrote: state_ll, state_ml, state_of (each as accLog bits).
  // The decoder reads them in the same order (since the backward bitstream
  // reverses write order, the last-written = first-read).
  let stateLl = Number(br.readBits(LL_ACC_LOG));
  let stateMl = Number(br.readBits(ML_ACC_LOG));
  let stateOf = Number(br.readBits(OF_ACC_LOG));

  let litPos = 0;

  for (let i = 0; i < nSeqs; i++) {
    // Decode symbols (state transitions) — order: LL, OF, ML.
    let llCode: number;
    let ofCode: number;
    let mlCode: number;
    [llCode, stateLl] = fseDecodeSym(stateLl, dtLl, br);
    [ofCode, stateOf] = fseDecodeSym(stateOf, dtOf, br);
    [mlCode, stateMl] = fseDecodeSym(stateMl, dtMl, br);

    if (llCode >= LL_CODES.length) throw new Error(`invalid LL code ${llCode}`);
    if (mlCode >= ML_CODES.length) throw new Error(`invalid ML code ${mlCode}`);

    const llInfo = LL_CODES[llCode]!;
    const mlInfo = ML_CODES[mlCode]!;

    // Read extra bits for literal length, match length, and offset.
    const ll = llInfo[0] + Number(br.readBits(llInfo[1]));
    const ml = mlInfo[0] + Number(br.readBits(mlInfo[1]));
    // Offset: raw = (1 << of_code) | extra_bits; offset = raw - 3
    const ofRaw = (1 << ofCode) | Number(br.readBits(ofCode));
    const offset = ofRaw - 3;

    // Emit `ll` literal bytes from the literals buffer.
    const litEnd = litPos + ll;
    if (litEnd > lits.length) {
      throw new Error(
        `literal run ${ll} overflows literals buffer (pos=${litPos} len=${lits.length})`
      );
    }
    // Decompression bomb guard: a crafted block can declare nSeqs ≈ 196 K
    // and ll ≤ 131 K per sequence — without this check, a few-KB compressed
    // block could push gigabytes of output before the outer raw/RLE caps in
    // `decompress` ever fire (those caps don't see inside compressed blocks).
    if (out.length + ll > MAX_OUTPUT) {
      throw new Error(`decompressed size exceeds limit of ${MAX_OUTPUT} bytes`);
    }
    for (let j = litPos; j < litEnd; j++) out.push(lits[j]!);
    litPos = litEnd;

    // Copy `ml` bytes from `offset` back in the output buffer.
    // offset = 0 would be a reference past the end; minimum valid = 1.
    if (offset === 0 || offset > out.length) {
      throw new Error(`bad match offset ${offset} (output len ${out.length})`);
    }
    // Decompression bomb guard: ml ≤ 131 K per sequence; with offset=1 this
    // would silently inflate one byte to 131 K bytes. Multiplied by the
    // sequence count this dwarfs MAX_OUTPUT.
    if (out.length + ml > MAX_OUTPUT) {
      throw new Error(`decompressed size exceeds limit of ${MAX_OUTPUT} bytes`);
    }
    const copyStart = out.length - offset;
    for (let j = 0; j < ml; j++) {
      out.push(out[copyStart + j]!);
    }
  }

  // Append any remaining literals after the last sequence.
  if (out.length + (lits.length - litPos) > MAX_OUTPUT) {
    throw new Error(`decompressed size exceeds limit of ${MAX_OUTPUT} bytes`);
  }
  for (let j = litPos; j < lits.length; j++) out.push(lits[j]!);
}

// ─── Public API ───────────────────────────────────────────────────────────────

/**
 * Compress `data` to ZStd format (RFC 8878).
 *
 * Produces a valid ZStd frame decompressable by the `zstd` CLI or any
 * conforming implementation.
 *
 * @example
 * ```ts
 * const text = new TextEncoder().encode("the quick brown fox ".repeat(20));
 * const compressed = compress(text);
 * console.log(compressed.length < text.length); // true — ≥20% compression
 * ```
 */
export function compress(data: Uint8Array): Uint8Array {
  const out: number[] = [];

  // ── ZStd frame header ────────────────────────────────────────────────
  // Magic number (4 bytes LE): 0xFD2FB528 → [0x28, 0xB5, 0x2F, 0xFD]
  const magic = MAGIC;
  out.push(magic & 0xFF, (magic >>> 8) & 0xFF, (magic >>> 16) & 0xFF, (magic >>> 24) & 0xFF);

  // Frame Header Descriptor (FHD) = 0xE0:
  //   bit 7-6: FCS_Field_Size flag = 11 → 8-byte FCS
  //   bit 5:   Single_Segment_Flag = 1 (no Window_Descriptor)
  //   bit 4:   Content_Checksum_Flag = 0
  //   bit 3-2: reserved = 0
  //   bit 1-0: Dict_ID_Flag = 0
  out.push(0xE0);

  // Frame_Content_Size (8 bytes LE) — the uncompressed size.
  // Used by decoders to pre-allocate the output buffer.
  const len = data.length;
  out.push(
    len & 0xFF, (len >>> 8) & 0xFF, (len >>> 16) & 0xFF, (len >>> 24) & 0xFF,
    0, 0, 0, 0, // high 4 bytes (always 0 for ≤ 4 GB)
  );

  // ── Blocks ───────────────────────────────────────────────────────────
  // Special case: empty input → one empty raw block.
  if (data.length === 0) {
    // Last=1, Type=Raw(00), Size=0 → header = 0b0000_0001 = 0x01
    out.push(0x01, 0x00, 0x00);
    return new Uint8Array(out);
  }

  let offset = 0;
  while (offset < data.length) {
    const end = Math.min(offset + MAX_BLOCK_SIZE, data.length);
    const block = data.subarray(offset, end);
    const last = end === data.length;

    // ── Try RLE block ─────────────────────────────────────────────────
    // If all bytes are identical, encode as a 1-byte RLE block (4 bytes total).
    const isRle = block.length > 0 && block.every(b => b === block[0]);
    if (isRle) {
      // Block header: Last, Type=01 (RLE), Size = original count
      // hdr = (size << 3) | (0b01 << 1) | last
      const hdr = (block.length << 3) | (0b01 << 1) | (last ? 1 : 0);
      out.push(hdr & 0xFF, (hdr >>> 8) & 0xFF, (hdr >>> 16) & 0xFF);
      out.push(block[0]!);
    } else {
      // ── Try compressed block ──────────────────────────────────────
      const compressed = compressBlock(block);
      if (compressed !== null) {
        const hdr = (compressed.length << 3) | (0b10 << 1) | (last ? 1 : 0);
        out.push(hdr & 0xFF, (hdr >>> 8) & 0xFF, (hdr >>> 16) & 0xFF);
        for (const b of compressed) out.push(b);
      } else {
        // ── Raw block (fallback) ─────────────────────────────────
        const hdr = (block.length << 3) | (0b00 << 1) | (last ? 1 : 0);
        out.push(hdr & 0xFF, (hdr >>> 8) & 0xFF, (hdr >>> 16) & 0xFF);
        for (const b of block) out.push(b);
      }
    }

    offset = end;
  }

  return new Uint8Array(out);
}

/**
 * Decompress a ZStd frame, returning the original data.
 *
 * Accepts any valid ZStd frame with:
 * - Single-segment or multi-segment layout
 * - Raw, RLE, or Compressed blocks
 * - Predefined FSE modes (no per-frame table description)
 *
 * Throws `Error` if the input is truncated, has a bad magic number,
 * or contains unsupported features.
 *
 * @example
 * ```ts
 * const original = new TextEncoder().encode("hello, world!");
 * const rt = decompress(compress(original));
 * // rt deepEquals original
 * ```
 */
export function decompress(data: Uint8Array): Uint8Array {
  if (data.length < 5) throw new Error("frame too short");

  // ── Validate magic ───────────────────────────────────────────────────
  const magic =
    data[0]! | (data[1]! << 8) | (data[2]! << 16) | (data[3]! * 0x1000000);
  if ((magic >>> 0) !== MAGIC) {
    throw new Error(`bad magic: 0x${(magic >>> 0).toString(16).padStart(8, '0')} (expected 0xfd2fb528)`);
  }

  let pos = 4;

  // ── Parse Frame Header Descriptor ───────────────────────────────────
  const fhd = data[pos]!;
  pos++;

  // FCS_Field_Size: bits [7:6] of FHD.
  //   00 → 0 bytes (or 1 byte if Single_Segment=1)
  //   01 → 2 bytes (value + 256)
  //   10 → 4 bytes
  //   11 → 8 bytes
  const fcsFlag = (fhd >>> 6) & 3;

  // Single_Segment_Flag: bit 5. When set, Window_Descriptor is omitted.
  const singleSeg = (fhd >>> 5) & 1;

  // Dict_ID_Flag: bits [1:0]. Indicates dict ID byte count.
  const dictFlag = fhd & 3;

  // ── Window Descriptor ────────────────────────────────────────────────
  // Present only if Single_Segment_Flag = 0.
  if (singleSeg === 0) pos++; // skip Window_Descriptor

  // ── Dict ID ──────────────────────────────────────────────────────────
  const dictIdBytes = [0, 1, 2, 4][dictFlag]!;
  pos += dictIdBytes; // skip dict ID (custom dicts not supported)
  if (pos > data.length) {
    throw new Error("zstd: frame header truncated (dict ID field)");
  }

  // ── Frame Content Size ───────────────────────────────────────────────
  let fcsBytes: number;
  if (fcsFlag === 0) {
    fcsBytes = singleSeg === 1 ? 1 : 0;
  } else if (fcsFlag === 1) {
    fcsBytes = 2;
  } else if (fcsFlag === 2) {
    fcsBytes = 4;
  } else {
    fcsBytes = 8;
  }
  pos += fcsBytes; // skip FCS (we trust the blocks)
  if (pos > data.length) {
    throw new Error("zstd: frame header truncated (FCS field)");
  }

  // ── Blocks ───────────────────────────────────────────────────────────
  const out: number[] = [];

  for (;;) {
    if (pos + 3 > data.length) throw new Error("truncated block header");

    // 3-byte little-endian block header.
    const hdr = data[pos]! | (data[pos + 1]! << 8) | (data[pos + 2]! << 16);
    pos += 3;

    const last = (hdr & 1) !== 0;
    const btype = (hdr >>> 1) & 3;
    const bsize = hdr >>> 3;

    if (btype === 0) {
      // Raw block: `bsize` bytes of verbatim content.
      if (pos + bsize > data.length) {
        throw new Error(`raw block truncated: need ${bsize} bytes at pos ${pos}`);
      }
      if (out.length + bsize > MAX_OUTPUT) {
        throw new Error(`decompressed size exceeds limit of ${MAX_OUTPUT} bytes`);
      }
      for (let i = pos; i < pos + bsize; i++) out.push(data[i]!);
      pos += bsize;
    } else if (btype === 1) {
      // RLE block: 1 byte repeated `bsize` times.
      if (pos >= data.length) throw new Error("RLE block missing byte");
      if (out.length + bsize > MAX_OUTPUT) {
        throw new Error(`decompressed size exceeds limit of ${MAX_OUTPUT} bytes`);
      }
      const byte = data[pos]!;
      pos++;
      for (let i = 0; i < bsize; i++) out.push(byte);
    } else if (btype === 2) {
      // Compressed block.
      if (pos + bsize > data.length) {
        throw new Error(`compressed block truncated: need ${bsize} bytes`);
      }
      const blockData = data.subarray(pos, pos + bsize);
      pos += bsize;
      decompressBlock(blockData, out);
    } else {
      throw new Error("reserved block type 3");
    }

    if (last) break;
  }

  return new Uint8Array(out);
}
