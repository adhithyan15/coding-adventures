/**
 * CMP06 — Brotli lossless compression (2013)
 *
 * Brotli (RFC 7932) is Google's lossless compression algorithm that improves
 * on DEFLATE in three key ways:
 *
 *   1. Context-dependent literal trees — 4 separate Huffman trees for literals,
 *      one per context bucket (space/punct, digit, uppercase, lowercase). Each
 *      tree is tuned to the distribution of bytes that follow a certain category
 *      of byte, achieving better entropy coding.
 *
 *   2. Insert-and-copy commands (ICC) — instead of DEFLATE's flat literal/match
 *      stream, Brotli bundles insert lengths and copy lengths into a single ICC
 *      Huffman symbol. One ICC code replaces what DEFLATE encodes as two symbols.
 *
 *   3. Larger sliding window — 65,535 bytes (vs DEFLATE's 4,096), allowing
 *      matches across longer repeated content.
 *
 * # Bit Stream Layout
 *
 * Per regular command (copy_length >= 4):
 *
 *   [ICC Huffman code]
 *   [insert_extra bits, LSB-first]
 *   [copy_extra bits, LSB-first]
 *   [insert_length × literal Huffman codes, each context-dependent]
 *   [distance Huffman code]
 *   [dist_extra bits, LSB-first]
 *
 * End of stream:
 *
 *   [sentinel ICC code 63]
 *   [flush literals, if any — encoded context-dependently after the sentinel]
 *
 * The sentinel terminates the command loop. Any trailing literal bytes that
 * could not be bundled into a regular ICC command (e.g., all bytes of a
 * purely-literal input) are encoded as "flush literals" directly after the
 * sentinel. The decompressor reads them once it sees ICC=63.
 *
 * # Why Flush Literals?
 *
 * The ICC table only supports insert_length up to 32 (codes 56–62). Inputs
 * with > 32 trailing literals after the last LZ match cannot fit in a single
 * ICC command. Rather than injecting dummy copies (which corrupt the output),
 * flush literals are appended to the bit stream after the sentinel. The
 * decompressor uses original_length to know how many flush literals to read.
 *
 * # Wire Format (CMP06)
 *
 *   Header (10 bytes):
 *     [4B] original_length    big-endian uint32
 *     [1B] icc_entry_count    uint8 (1–64)
 *     [1B] dist_entry_count   uint8 (0–32)
 *     [1B] ctx0_entry_count   uint8
 *     [1B] ctx1_entry_count   uint8
 *     [1B] ctx2_entry_count   uint8
 *     [1B] ctx3_entry_count   uint8
 *
 *   ICC code-length table (icc_entry_count × 2 bytes):
 *     [1B] symbol (0–63), [1B] code_length
 *     sorted by (code_length ASC, symbol ASC)
 *
 *   Distance code-length table (dist_entry_count × 2 bytes):
 *     [1B] symbol (0–31), [1B] code_length
 *     sorted by (code_length ASC, symbol ASC)
 *     omitted entirely if no copy commands exist
 *
 *   Literal tree 0–3 code-length tables (entry_count × 3 bytes each):
 *     [2B] symbol (big-endian uint16, 0–255), [1B] code_length
 *     sorted by (code_length ASC, symbol ASC)
 *     omitted if no literals appeared in that context
 *
 *   Bit stream (remaining bytes):
 *     LSB-first packed bits
 *
 * # Context Modeling
 *
 * The 4 literal context buckets map the last emitted byte to a bucket:
 *
 *   bucket 0 — space or punctuation (catch-all, including stream start)
 *   bucket 1 — digit ('0'–'9')
 *   bucket 2 — uppercase letter ('A'–'Z')
 *   bucket 3 — lowercase letter ('a'–'z')
 */

import { HuffmanTree } from "@coding-adventures/huffman-tree";

// ---------------------------------------------------------------------------
// ICC table (insert-copy code table, 64 entries)
// ---------------------------------------------------------------------------
//
// Each ICC code bundles an insert-length range and a copy-length range.
// The encoder finds the code that covers the desired (insert, copy) pair
// and emits extra bits to select the exact values within the ranges.
//
// The ICC copy-length values have gaps:
//   copy=7 is not representable (between code 2 max=6 and code 3 base=8)
//   copy=12-13 is not representable (between code 4 max=11 and code 5 base=14)
//   etc.
// The encoder must round down to the nearest representable copy length.
//
// insert ranges:
//   codes  0–15: insert=0 (no insert)
//   codes 16–23: insert=1
//   codes 24–31: insert=2
//   codes 32–39: insert=3–4   (base=3, extra=1)
//   codes 40–47: insert=5–8   (base=5, extra=2)
//   codes 48–55: insert=9–16  (base=9, extra=3)
//   codes 56–62: insert=17–32 (base=17, extra=4)
//   code  63:    sentinel (insert=0, copy=0)

interface IccEntry {
  insertBase: number;
  insertExtra: number;
  copyBase: number;
  copyExtra: number;
}

const ICC_TABLE: IccEntry[] = [
  { insertBase: 0, insertExtra: 0, copyBase:   4, copyExtra: 0 }, //  0
  { insertBase: 0, insertExtra: 0, copyBase:   5, copyExtra: 0 }, //  1
  { insertBase: 0, insertExtra: 0, copyBase:   6, copyExtra: 0 }, //  2
  { insertBase: 0, insertExtra: 0, copyBase:   8, copyExtra: 1 }, //  3
  { insertBase: 0, insertExtra: 0, copyBase:  10, copyExtra: 1 }, //  4
  { insertBase: 0, insertExtra: 0, copyBase:  14, copyExtra: 2 }, //  5
  { insertBase: 0, insertExtra: 0, copyBase:  18, copyExtra: 2 }, //  6
  { insertBase: 0, insertExtra: 0, copyBase:  26, copyExtra: 3 }, //  7
  { insertBase: 0, insertExtra: 0, copyBase:  34, copyExtra: 3 }, //  8
  { insertBase: 0, insertExtra: 0, copyBase:  50, copyExtra: 4 }, //  9
  { insertBase: 0, insertExtra: 0, copyBase:  66, copyExtra: 4 }, // 10
  { insertBase: 0, insertExtra: 0, copyBase:  98, copyExtra: 5 }, // 11
  { insertBase: 0, insertExtra: 0, copyBase: 130, copyExtra: 5 }, // 12
  { insertBase: 0, insertExtra: 0, copyBase: 194, copyExtra: 6 }, // 13
  { insertBase: 0, insertExtra: 0, copyBase: 258, copyExtra: 7 }, // 14  → max 385
  { insertBase: 0, insertExtra: 0, copyBase: 514, copyExtra: 8 }, // 15  → max 769
  { insertBase: 1, insertExtra: 0, copyBase:   4, copyExtra: 0 }, // 16
  { insertBase: 1, insertExtra: 0, copyBase:   5, copyExtra: 0 }, // 17
  { insertBase: 1, insertExtra: 0, copyBase:   6, copyExtra: 0 }, // 18
  { insertBase: 1, insertExtra: 0, copyBase:   8, copyExtra: 1 }, // 19
  { insertBase: 1, insertExtra: 0, copyBase:  10, copyExtra: 1 }, // 20
  { insertBase: 1, insertExtra: 0, copyBase:  14, copyExtra: 2 }, // 21
  { insertBase: 1, insertExtra: 0, copyBase:  18, copyExtra: 2 }, // 22
  { insertBase: 1, insertExtra: 0, copyBase:  26, copyExtra: 3 }, // 23
  { insertBase: 2, insertExtra: 0, copyBase:   4, copyExtra: 0 }, // 24
  { insertBase: 2, insertExtra: 0, copyBase:   5, copyExtra: 0 }, // 25
  { insertBase: 2, insertExtra: 0, copyBase:   6, copyExtra: 0 }, // 26
  { insertBase: 2, insertExtra: 0, copyBase:   8, copyExtra: 1 }, // 27
  { insertBase: 2, insertExtra: 0, copyBase:  10, copyExtra: 1 }, // 28
  { insertBase: 2, insertExtra: 0, copyBase:  14, copyExtra: 2 }, // 29
  { insertBase: 2, insertExtra: 0, copyBase:  18, copyExtra: 2 }, // 30
  { insertBase: 2, insertExtra: 0, copyBase:  26, copyExtra: 3 }, // 31
  { insertBase: 3, insertExtra: 1, copyBase:   4, copyExtra: 0 }, // 32
  { insertBase: 3, insertExtra: 1, copyBase:   5, copyExtra: 0 }, // 33
  { insertBase: 3, insertExtra: 1, copyBase:   6, copyExtra: 0 }, // 34
  { insertBase: 3, insertExtra: 1, copyBase:   8, copyExtra: 1 }, // 35
  { insertBase: 3, insertExtra: 1, copyBase:  10, copyExtra: 1 }, // 36
  { insertBase: 3, insertExtra: 1, copyBase:  14, copyExtra: 2 }, // 37
  { insertBase: 3, insertExtra: 1, copyBase:  18, copyExtra: 2 }, // 38
  { insertBase: 3, insertExtra: 1, copyBase:  26, copyExtra: 3 }, // 39
  { insertBase: 5, insertExtra: 2, copyBase:   4, copyExtra: 0 }, // 40
  { insertBase: 5, insertExtra: 2, copyBase:   5, copyExtra: 0 }, // 41
  { insertBase: 5, insertExtra: 2, copyBase:   6, copyExtra: 0 }, // 42
  { insertBase: 5, insertExtra: 2, copyBase:   8, copyExtra: 1 }, // 43
  { insertBase: 5, insertExtra: 2, copyBase:  10, copyExtra: 1 }, // 44
  { insertBase: 5, insertExtra: 2, copyBase:  14, copyExtra: 2 }, // 45
  { insertBase: 5, insertExtra: 2, copyBase:  18, copyExtra: 2 }, // 46
  { insertBase: 5, insertExtra: 2, copyBase:  26, copyExtra: 3 }, // 47
  { insertBase: 9, insertExtra: 3, copyBase:   4, copyExtra: 0 }, // 48
  { insertBase: 9, insertExtra: 3, copyBase:   5, copyExtra: 0 }, // 49
  { insertBase: 9, insertExtra: 3, copyBase:   6, copyExtra: 0 }, // 50
  { insertBase: 9, insertExtra: 3, copyBase:   8, copyExtra: 1 }, // 51
  { insertBase: 9, insertExtra: 3, copyBase:  10, copyExtra: 1 }, // 52
  { insertBase: 9, insertExtra: 3, copyBase:  14, copyExtra: 2 }, // 53
  { insertBase: 9, insertExtra: 3, copyBase:  18, copyExtra: 2 }, // 54
  { insertBase: 9, insertExtra: 3, copyBase:  26, copyExtra: 3 }, // 55
  { insertBase: 17, insertExtra: 4, copyBase:   4, copyExtra: 0 }, // 56
  { insertBase: 17, insertExtra: 4, copyBase:   5, copyExtra: 0 }, // 57
  { insertBase: 17, insertExtra: 4, copyBase:   6, copyExtra: 0 }, // 58
  { insertBase: 17, insertExtra: 4, copyBase:   8, copyExtra: 1 }, // 59
  { insertBase: 17, insertExtra: 4, copyBase:  10, copyExtra: 1 }, // 60
  { insertBase: 17, insertExtra: 4, copyBase:  14, copyExtra: 2 }, // 61
  { insertBase: 17, insertExtra: 4, copyBase:  18, copyExtra: 2 }, // 62
  { insertBase: 0,  insertExtra: 0, copyBase:   0, copyExtra: 0 }, // 63 sentinel
];

// Maximum insert length representable in a single ICC code.
const MAX_INSERT_PER_ICC = 32; // codes 56-62: 17 + (1<<4) - 1 = 32

// ---------------------------------------------------------------------------
// Distance code table (codes 0–31, extending CMP05's 24 codes to 32)
// ---------------------------------------------------------------------------
//
// Codes 0–23 are identical to CMP05; codes 24–31 cover offsets up to 65535.

interface DistEntry {
  code: number;
  base: number;
  extraBits: number;
}

const DIST_TABLE: DistEntry[] = [
  { code:  0, base:     1, extraBits:  0 },
  { code:  1, base:     2, extraBits:  0 },
  { code:  2, base:     3, extraBits:  0 },
  { code:  3, base:     4, extraBits:  0 },
  { code:  4, base:     5, extraBits:  1 },
  { code:  5, base:     7, extraBits:  1 },
  { code:  6, base:     9, extraBits:  2 },
  { code:  7, base:    13, extraBits:  2 },
  { code:  8, base:    17, extraBits:  3 },
  { code:  9, base:    25, extraBits:  3 },
  { code: 10, base:    33, extraBits:  4 },
  { code: 11, base:    49, extraBits:  4 },
  { code: 12, base:    65, extraBits:  5 },
  { code: 13, base:    97, extraBits:  5 },
  { code: 14, base:   129, extraBits:  6 },
  { code: 15, base:   193, extraBits:  6 },
  { code: 16, base:   257, extraBits:  7 },
  { code: 17, base:   385, extraBits:  7 },
  { code: 18, base:   513, extraBits:  8 },
  { code: 19, base:   769, extraBits:  8 },
  { code: 20, base:  1025, extraBits:  9 },
  { code: 21, base:  1537, extraBits:  9 },
  { code: 22, base:  2049, extraBits: 10 },
  { code: 23, base:  3073, extraBits: 10 },
  { code: 24, base:  4097, extraBits: 11 },
  { code: 25, base:  6145, extraBits: 11 },
  { code: 26, base:  8193, extraBits: 12 },
  { code: 27, base: 12289, extraBits: 12 },
  { code: 28, base: 16385, extraBits: 13 },
  { code: 29, base: 24577, extraBits: 13 },
  { code: 30, base: 32769, extraBits: 14 },
  { code: 31, base: 49153, extraBits: 14 },
];

const DIST_BASE = new Map<number, number>();
const DIST_EXTRA = new Map<number, number>();
for (const e of DIST_TABLE) {
  DIST_BASE.set(e.code, e.base);
  DIST_EXTRA.set(e.code, e.extraBits);
}

// ---------------------------------------------------------------------------
// Context modeling
// ---------------------------------------------------------------------------
//
// Determines which of the 4 literal context buckets applies based on the
// last emitted byte. p1 = -1 at stream start → bucket 0 (same as space/punct).
//
// The insight: if the previous byte was a lowercase letter, the next byte
// is almost certainly another letter or space — very different from after
// a digit. Separate trees per context are each precisely calibrated.

function literalContext(p1: number): number {
  if (p1 >= 0x61 && p1 <= 0x7a) return 3; // 'a'–'z' → lowercase
  if (p1 >= 0x41 && p1 <= 0x5a) return 2; // 'A'–'Z' → uppercase
  if (p1 >= 0x30 && p1 <= 0x39) return 1; // '0'–'9' → digit
  return 0;                                // space/punct/other or stream start
}

// ---------------------------------------------------------------------------
// ICC code lookup helpers
// ---------------------------------------------------------------------------

/**
 * Find the ICC code (0–62) that covers both insertLen and copyLen.
 * Returns -1 if no single code covers the pair.
 * Falls back to a copy-only code (insert=0) if insert is too large.
 */
function findIccCode(insertLen: number, copyLen: number): number {
  for (let code = 0; code < 63; code++) {
    const e = ICC_TABLE[code];
    const maxInsert = e.insertBase + (1 << e.insertExtra) - 1;
    const maxCopy = e.copyBase + (1 << e.copyExtra) - 1;
    if (
      insertLen >= e.insertBase && insertLen <= maxInsert &&
      copyLen >= e.copyBase && copyLen <= maxCopy
    ) {
      return code;
    }
  }
  // Fallback: copy-only code (insert=0) for this copy_length.
  for (let code = 0; code < 16; code++) {
    const e = ICC_TABLE[code];
    const maxCopy = e.copyBase + (1 << e.copyExtra) - 1;
    if (copyLen >= e.copyBase && copyLen <= maxCopy) return code;
  }
  return 0;
}

/**
 * Find the largest encodable copy_length <= requested for the given insertLen.
 *
 * The ICC table has gaps (e.g., copy=7 is not representable). This function
 * rounds down to the nearest valid copy length. The LZ matcher uses the
 * returned value to clamp the match length so the encoder always has a valid
 * ICC code available.
 *
 * Examples:
 *   findBestIccCopy(0, 4)   → 4  (exact)
 *   findBestIccCopy(0, 7)   → 6  (gap at 7, rounds down to 6)
 *   findBestIccCopy(0, 258) → 258 (exact)
 */
function findBestIccCopy(insertLen: number, copyLen: number): number {
  let best = 0;
  for (let code = 0; code < 63; code++) {
    const e = ICC_TABLE[code];
    const maxInsert = e.insertBase + (1 << e.insertExtra) - 1;
    if (insertLen < e.insertBase || insertLen > maxInsert) continue;
    const maxCopy = e.copyBase + (1 << e.copyExtra) - 1;
    if (copyLen >= e.copyBase && copyLen <= maxCopy) return copyLen; // exact
    if (maxCopy <= copyLen && maxCopy > best) best = maxCopy;
  }
  return Math.max(best, 4); // minimum match length is 4
}

// ---------------------------------------------------------------------------
// Distance code lookup
// ---------------------------------------------------------------------------

function distCode(distance: number): number {
  for (const e of DIST_TABLE) {
    const maxDist = e.base + (1 << e.extraBits) - 1;
    if (distance <= maxDist) return e.code;
  }
  return 31;
}

// ---------------------------------------------------------------------------
// LZ matching (inline — no LZSS dependency)
// ---------------------------------------------------------------------------
//
// Brotli's command structure (insert+copy bundles) differs from LZSS's flat
// token stream, so we implement matching inline.
//
// Key constraint: we only accept a match when insert_buf.length ≤ MAX_INSERT_PER_ICC
// (32 bytes). Larger insert buffers can't be encoded in a single ICC command,
// so we defer the match and accumulate more bytes into the insert buffer until
// it can be flushed as a literal (after the sentinel).
//
// Match length is clamped to the largest representable copy length ≤ actual
// match length, using findBestIccCopy() to handle ICC table gaps.

const MAX_WINDOW = 65535;
const MIN_MATCH = 4;
const MAX_MATCH = 258;

function findLongestMatch(
  data: Uint8Array,
  pos: number
): { offset: number; length: number } {
  const windowStart = Math.max(0, pos - MAX_WINDOW);
  let bestLen = 0;
  let bestOffset = 0;

  for (let start = pos - 1; start >= windowStart; start--) {
    if (data[start] !== data[pos]) continue;
    let matchLen = 0;
    const maxLen = Math.min(MAX_MATCH, data.length - pos);
    while (matchLen < maxLen && data[start + matchLen] === data[pos + matchLen]) {
      matchLen++;
    }
    if (matchLen > bestLen) {
      bestLen = matchLen;
      bestOffset = pos - start;
      if (bestLen === MAX_MATCH) break;
    }
  }

  if (bestLen < MIN_MATCH) return { offset: 0, length: 0 };
  return { offset: bestOffset, length: bestLen };
}

// ---------------------------------------------------------------------------
// Command structure
// ---------------------------------------------------------------------------
//
// Regular commands have copy_length >= 4.
// The sentinel has insert_length=0, copy_length=0, copy_distance=0, literals=[].

interface Command {
  insertLength: number;
  copyLength: number;
  copyDistance: number;
  literals: number[];
}

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------
//
// Brotli uses LSB-first bit packing: bit 0 of the first code goes into
// bit position 0 of the first output byte.

class BitBuilder {
  private buf = 0;
  private bitPos = 0;
  private out: number[] = [];

  writeBitString(s: string): void {
    for (let i = 0; i < s.length; i++) {
      if (s[i] === "1") {
        this.buf |= 1 << this.bitPos;
      }
      this.bitPos++;
      if (this.bitPos === 8) {
        this.out.push(this.buf & 0xff);
        this.buf = 0;
        this.bitPos = 0;
      }
    }
  }

  writeRawBitsLSB(val: number, n: number): void {
    for (let i = 0; i < n; i++) {
      if ((val >> i) & 1) {
        this.buf |= 1 << this.bitPos;
      }
      this.bitPos++;
      if (this.bitPos === 8) {
        this.out.push(this.buf & 0xff);
        this.buf = 0;
        this.bitPos = 0;
      }
    }
  }

  flush(): void {
    if (this.bitPos > 0) {
      this.out.push(this.buf & 0xff);
      this.buf = 0;
      this.bitPos = 0;
    }
  }

  bytes(): Uint8Array {
    return new Uint8Array(this.out);
  }
}

function unpackBits(data: Uint8Array): string {
  let bits = "";
  for (const byte of data) {
    for (let i = 0; i < 8; i++) {
      bits += (byte >> i) & 1;
    }
  }
  return bits;
}

// ---------------------------------------------------------------------------
// Canonical code reconstruction (for decompression)
// ---------------------------------------------------------------------------
//
// Single-symbol trees use code "0" (length 1).

function reconstructCanonicalCodes(
  lengths: [number, number][]
): Map<string, number> {
  const result = new Map<string, number>();
  if (lengths.length === 0) return result;
  if (lengths.length === 1) {
    result.set("0", lengths[0][0]);
    return result;
  }
  let code = 0;
  let prevLen = lengths[0][1];
  for (const [symbol, codeLen] of lengths) {
    if (codeLen > prevLen) {
      code <<= codeLen - prevLen;
    }
    const bitStr = code.toString(2).padStart(codeLen, "0");
    result.set(bitStr, symbol);
    code++;
    prevLen = codeLen;
  }
  return result;
}

// ---------------------------------------------------------------------------
// Pass 1: LZ matching → command list + flush literals
// ---------------------------------------------------------------------------
//
// Returns:
//   commands      — regular ICC commands (copy_length >= 4) + sentinel
//   flushLiterals — trailing bytes that couldn't form an ICC command
//                   (encoded AFTER the sentinel in the bit stream)

function buildCommands(
  data: Uint8Array
): { commands: Command[]; flushLiterals: number[] } {
  const commands: Command[] = [];
  let insertBuf: number[] = [];
  let pos = 0;

  while (pos < data.length) {
    const { offset, length } = findLongestMatch(data, pos);

    if (length >= MIN_MATCH && insertBuf.length <= MAX_INSERT_PER_ICC) {
      // Only take LZ match when insert_buf fits in one ICC command (≤ 32 bytes).
      // Clamp copy length to nearest representable value (handles ICC table gaps).
      const actualCopy = findBestIccCopy(insertBuf.length, length);

      commands.push({
        insertLength: insertBuf.length,
        copyLength: actualCopy,
        copyDistance: offset,
        literals: [...insertBuf],
      });
      insertBuf = [];
      pos += actualCopy;
    } else {
      // No match (or insert_buf too large): accumulate as literal.
      insertBuf.push(data[pos]);
      pos++;
    }
  }

  // Any remaining bytes in insert_buf become flush literals.
  // They are encoded AFTER the sentinel ICC=63 in the bit stream.
  const flushLiterals = insertBuf.slice();

  // Append sentinel.
  commands.push({ insertLength: 0, copyLength: 0, copyDistance: 0, literals: [] });

  return { commands, flushLiterals };
}

// ---------------------------------------------------------------------------
// Public API: compress
// ---------------------------------------------------------------------------

/**
 * Compress data using Brotli (CMP06) and return wire-format bytes.
 *
 * Bit stream layout:
 *   For each regular command (copy_length >= 4):
 *     [ICC] [insert_extra] [copy_extra] [literals] [dist] [dist_extra]
 *   Then:
 *     [sentinel ICC=63] [flush literals, if any]
 *
 * Flush literals are trailing bytes that could not be bundled into a regular
 * ICC command (e.g., the entire input for purely-literal data). They follow
 * the sentinel in the bit stream and are decoded using the same per-context
 * literal Huffman trees.
 *
 * @param data - The raw bytes to compress.
 * @returns Compressed bytes in CMP06 wire format.
 */
export function compress(data: Uint8Array): Uint8Array {
  const originalLength = data.length;

  // ── Empty input special case ───────────────────────────────────────────────
  // Encode: header(10) + ICC table(2 bytes: sentinel code 63, length 1)
  //       + bit stream(1 byte: "0" padded = 0x00)
  if (originalLength === 0) {
    const out = new Uint8Array(13);
    const view = new DataView(out.buffer);
    view.setUint32(0, 0, false);
    out[4] = 1;   // icc_entry_count = 1
    out[5] = 0;   // dist_entry_count = 0
    out[6] = 0;   // ctx0_entry_count = 0
    out[7] = 0;   // ctx1_entry_count = 0
    out[8] = 0;   // ctx2_entry_count = 0
    out[9] = 0;   // ctx3_entry_count = 0
    out[10] = 63; // ICC symbol = 63
    out[11] = 1;  // code_length = 1
    out[12] = 0;  // bit stream: "0" + 7 padding zeros
    return out;
  }

  // ── Pass 1: LZ matching → commands + flush literals ───────────────────────
  const { commands, flushLiterals } = buildCommands(data);

  // ── Pass 2a: Tally frequencies ─────────────────────────────────────────────
  //
  // Walk commands to count literal frequencies per context bucket, ICC code
  // frequencies, and distance code frequencies.
  // Also tally flush literal frequencies (after simulating the full
  // command phase to get the correct p1 at flush time).

  const litFreq: Map<number, number>[] = [
    new Map(), new Map(), new Map(), new Map(),
  ];
  const iccFreq = new Map<number, number>();
  const distFreq = new Map<number, number>();

  let history: number[] = []; // simulated output for p1 tracking

  for (const cmd of commands) {
    if (cmd.copyLength === 0) break; // sentinel — regular commands always have copy > 0

    const icc = findIccCode(cmd.insertLength, cmd.copyLength);
    iccFreq.set(icc, (iccFreq.get(icc) ?? 0) + 1);
    const dc = distCode(cmd.copyDistance);
    distFreq.set(dc, (distFreq.get(dc) ?? 0) + 1);

    for (const byte of cmd.literals) {
      const p1 = history.length > 0 ? history[history.length - 1] : -1;
      const ctx = literalContext(p1);
      litFreq[ctx].set(byte, (litFreq[ctx].get(byte) ?? 0) + 1);
      history.push(byte);
    }

    // Simulate copy.
    const start = history.length - cmd.copyDistance;
    for (let i = 0; i < cmd.copyLength; i++) {
      history.push(history[start + i]);
    }
  }

  // Sentinel always counts.
  iccFreq.set(63, (iccFreq.get(63) ?? 0) + 1);

  // Tally flush literals using the p1 at end of regular commands.
  let p1Flush = history.length > 0 ? history[history.length - 1] : -1;
  for (const byte of flushLiterals) {
    const ctx = literalContext(p1Flush);
    litFreq[ctx].set(byte, (litFreq[ctx].get(byte) ?? 0) + 1);
    p1Flush = byte;
  }

  // ── Pass 2b: Build Huffman trees ───────────────────────────────────────────
  const iccTree = HuffmanTree.build([...iccFreq.entries()]);
  const iccCodeTable = iccTree.canonicalCodeTable();

  let distCodeTable = new Map<number, string>();
  if (distFreq.size > 0) {
    const distTree = HuffmanTree.build([...distFreq.entries()]);
    distCodeTable = distTree.canonicalCodeTable();
  }

  const litCodeTables: Map<number, string>[] = [];
  for (let ctx = 0; ctx < 4; ctx++) {
    if (litFreq[ctx].size > 0) {
      const tree = HuffmanTree.build([...litFreq[ctx].entries()]);
      litCodeTables.push(tree.canonicalCodeTable());
    } else {
      litCodeTables.push(new Map());
    }
  }

  // ── Pass 2c: Encode bit stream ─────────────────────────────────────────────
  //
  // Per regular command: ICC code → insert extras → copy extras
  //                    → insert literals → dist code → dist extras
  //
  // Then: sentinel ICC=63 → flush literals (context-dependent)

  const bb = new BitBuilder();
  const hist2: number[] = [];

  for (const cmd of commands) {
    if (cmd.copyLength === 0) {
      // Sentinel.
      bb.writeBitString(iccCodeTable.get(63)!);

      // Flush literals after the sentinel.
      let p1f = hist2.length > 0 ? hist2[hist2.length - 1] : -1;
      for (const byte of flushLiterals) {
        const ctx = literalContext(p1f);
        bb.writeBitString(litCodeTables[ctx].get(byte)!);
        p1f = byte;
      }
      break;
    }

    const icc = findIccCode(cmd.insertLength, cmd.copyLength);
    const e = ICC_TABLE[icc];

    // 1. ICC Huffman code.
    bb.writeBitString(iccCodeTable.get(icc)!);

    // 2. Insert extra bits (LSB-first).
    bb.writeRawBitsLSB(cmd.insertLength - e.insertBase, e.insertExtra);

    // 3. Copy extra bits (LSB-first).
    bb.writeRawBitsLSB(cmd.copyLength - e.copyBase, e.copyExtra);

    // 4. Insert literals using per-context Huffman trees.
    for (const byte of cmd.literals) {
      const p1 = hist2.length > 0 ? hist2[hist2.length - 1] : -1;
      const ctx = literalContext(p1);
      bb.writeBitString(litCodeTables[ctx].get(byte)!);
      hist2.push(byte);
    }

    // 5. Distance code + extra bits.
    const dc = distCode(cmd.copyDistance);
    bb.writeBitString(distCodeTable.get(dc)!);
    bb.writeRawBitsLSB(cmd.copyDistance - DIST_BASE.get(dc)!, DIST_EXTRA.get(dc)!);

    // Simulate copy for p1 tracking.
    const start = hist2.length - cmd.copyDistance;
    for (let i = 0; i < cmd.copyLength; i++) {
      hist2.push(hist2[start + i]);
    }
  }

  bb.flush();
  const packedBits = bb.bytes();

  // ── Assemble wire format ───────────────────────────────────────────────────
  function sortedPairs(table: Map<number, string>): [number, number][] {
    const pairs: [number, number][] = [...table.entries()].map(
      ([sym, code]) => [sym, code.length] as [number, number]
    );
    pairs.sort(([sa, la], [sb, lb]) => la !== lb ? la - lb : sa - sb);
    return pairs;
  }

  const iccPairs = sortedPairs(iccCodeTable);
  const distPairs = sortedPairs(distCodeTable);
  const litPairs: [number, number][][] = litCodeTables.map(sortedPairs);

  const totalSize =
    10 +
    iccPairs.length * 2 +
    distPairs.length * 2 +
    litPairs[0].length * 3 +
    litPairs[1].length * 3 +
    litPairs[2].length * 3 +
    litPairs[3].length * 3 +
    packedBits.length;

  const out = new Uint8Array(totalSize);
  const view = new DataView(out.buffer);

  view.setUint32(0, originalLength, false);
  out[4] = iccPairs.length;
  out[5] = distPairs.length;
  out[6] = litPairs[0].length;
  out[7] = litPairs[1].length;
  out[8] = litPairs[2].length;
  out[9] = litPairs[3].length;

  let off = 10;
  for (const [sym, len] of iccPairs) { out[off++] = sym; out[off++] = len; }
  for (const [sym, len] of distPairs) { out[off++] = sym; out[off++] = len; }
  for (let ctx = 0; ctx < 4; ctx++) {
    for (const [sym, len] of litPairs[ctx]) {
      view.setUint16(off, sym, false);
      out[off + 2] = len;
      off += 3;
    }
  }
  out.set(packedBits, off);

  return out;
}

// ---------------------------------------------------------------------------
// Public API: decompress
// ---------------------------------------------------------------------------

/**
 * Decompress CMP06 wire-format data and return the original bytes.
 *
 * Decodes the bit stream as follows:
 *   Loop:
 *     Read ICC symbol.
 *     If ICC == 63 (sentinel):
 *       Read flush literals until output.length == originalLength.
 *       Break.
 *     Decode insert_length and copy_length from extra bits.
 *     Emit insert_length literals (per-context Huffman trees).
 *     If copy_length > 0: decode distance, copy bytes.
 *
 * @param data - Compressed bytes produced by compress().
 * @returns Original uncompressed bytes.
 */
export function decompress(data: Uint8Array): Uint8Array {
  if (data.length < 10) return new Uint8Array(0);

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const originalLength = view.getUint32(0, false);
  const iccEntryCount = data[4];
  const distEntryCount = data[5];
  const litEntryCounts = [data[6], data[7], data[8], data[9]];

  if (originalLength === 0) return new Uint8Array(0);

  let off = 10;

  // Parse ICC code-length table.
  const iccLengths: [number, number][] = [];
  for (let i = 0; i < iccEntryCount; i++) {
    iccLengths.push([data[off], data[off + 1]]);
    off += 2;
  }

  // Parse distance code-length table.
  const distLengths: [number, number][] = [];
  for (let i = 0; i < distEntryCount; i++) {
    distLengths.push([data[off], data[off + 1]]);
    off += 2;
  }

  // Parse 4 literal code-length tables.
  const litLengths: [number, number][][] = [[], [], [], []];
  for (let ctx = 0; ctx < 4; ctx++) {
    for (let i = 0; i < litEntryCounts[ctx]; i++) {
      const sym = view.getUint16(off, false);
      const clen = data[off + 2];
      litLengths[ctx].push([sym, clen]);
      off += 3;
    }
  }

  // Reconstruct canonical Huffman reverse maps.
  const iccRevMap = reconstructCanonicalCodes(iccLengths);
  const distRevMap = reconstructCanonicalCodes(distLengths);
  const litRevMaps = litLengths.map(reconstructCanonicalCodes);

  // Unpack bit stream.
  const bits = unpackBits(data.slice(off));
  let bitPos = 0;

  function readBits(n: number): number {
    let val = 0;
    for (let i = 0; i < n; i++) {
      if (bits[bitPos + i] === "1") val |= 1 << i;
    }
    bitPos += n;
    return val;
  }

  function nextHuffmanSymbol(revMap: Map<string, number>): number {
    let acc = "";
    while (true) {
      acc += bits[bitPos++];
      const sym = revMap.get(acc);
      if (sym !== undefined) return sym;
    }
  }

  // Decode.
  const output: number[] = [];
  let p1 = -1;

  while (true) {
    const icc = nextHuffmanSymbol(iccRevMap);

    if (icc === 63) {
      // Sentinel: read any flush literals that follow in the bit stream.
      // The encoder emitted them directly after the sentinel ICC code.
      while (output.length < originalLength) {
        const ctx = literalContext(p1);
        const byte = nextHuffmanSymbol(litRevMaps[ctx]);
        output.push(byte);
        p1 = byte;
      }
      break;
    }

    const e = ICC_TABLE[icc];
    const insertLength = e.insertBase + readBits(e.insertExtra);
    const copyLength = e.copyBase + readBits(e.copyExtra);

    // Emit insert_length literal bytes.
    for (let i = 0; i < insertLength; i++) {
      const ctx = literalContext(p1);
      const byte = nextHuffmanSymbol(litRevMaps[ctx]);
      output.push(byte);
      p1 = byte;
    }

    // Perform copy.
    if (copyLength > 0) {
      const dc = nextHuffmanSymbol(distRevMap);
      const copyDistance = DIST_BASE.get(dc)! + readBits(DIST_EXTRA.get(dc)!);
      const start = output.length - copyDistance;
      for (let i = 0; i < copyLength; i++) {
        const b = output[start + i];
        output.push(b);
        p1 = b;
      }
    }
  }

  return new Uint8Array(output.slice(0, originalLength));
}
