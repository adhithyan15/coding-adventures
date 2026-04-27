/**
 * CMP05 — DEFLATE lossless compression (1996)
 *
 * DEFLATE is the dominant general-purpose lossless compression algorithm,
 * powering ZIP, gzip, PNG, and HTTP/2 HPACK header compression. It combines:
 *
 *   Pass 1 — LZSS tokenization (CMP02): replace repeated substrings with
 *            back-references into a 4096-byte sliding window.
 *
 *   Pass 2 — Dual canonical Huffman coding (DT27): entropy-code the token
 *            stream with two separate Huffman trees:
 *              LL tree:   literals (0-255), end-of-data (256), length codes (257-284)
 *              Dist tree: distance codes (0-23, for offsets 1-4096)
 *
 * # The Expanded LL Alphabet
 *
 * DEFLATE merges literal bytes and match lengths into one "LL" alphabet:
 *
 *   Symbols 0-255:   literal byte values
 *   Symbol  256:     end-of-data marker
 *   Symbols 257-284: length codes (each covers a range via extra bits)
 *
 * # Wire Format (CMP05)
 *
 *   [4B] original_length    big-endian uint32
 *   [2B] ll_entry_count     big-endian uint16
 *   [2B] dist_entry_count   big-endian uint16 (0 if no matches)
 *   [ll_entry_count × 3B]   (symbol uint16 BE, code_length uint8)
 *   [dist_entry_count × 3B] same format
 *   [remaining bytes]       LSB-first packed bit stream
 */

import { HuffmanTree } from "@coding-adventures/huffman-tree";
import { encode as lzssEncode, type Token } from "@coding-adventures/lzss";

// ---------------------------------------------------------------------------
// Length code table (LL symbols 257-284)
// ---------------------------------------------------------------------------

interface LengthEntry {
  symbol: number;
  base: number;
  extraBits: number;
}

const LENGTH_TABLE: LengthEntry[] = [
  { symbol: 257, base:   3, extraBits: 0 },
  { symbol: 258, base:   4, extraBits: 0 },
  { symbol: 259, base:   5, extraBits: 0 },
  { symbol: 260, base:   6, extraBits: 0 },
  { symbol: 261, base:   7, extraBits: 0 },
  { symbol: 262, base:   8, extraBits: 0 },
  { symbol: 263, base:   9, extraBits: 0 },
  { symbol: 264, base:  10, extraBits: 0 },
  { symbol: 265, base:  11, extraBits: 1 },
  { symbol: 266, base:  13, extraBits: 1 },
  { symbol: 267, base:  15, extraBits: 1 },
  { symbol: 268, base:  17, extraBits: 1 },
  { symbol: 269, base:  19, extraBits: 2 },
  { symbol: 270, base:  23, extraBits: 2 },
  { symbol: 271, base:  27, extraBits: 2 },
  { symbol: 272, base:  31, extraBits: 2 },
  { symbol: 273, base:  35, extraBits: 3 },
  { symbol: 274, base:  43, extraBits: 3 },
  { symbol: 275, base:  51, extraBits: 3 },
  { symbol: 276, base:  59, extraBits: 3 },
  { symbol: 277, base:  67, extraBits: 4 },
  { symbol: 278, base:  83, extraBits: 4 },
  { symbol: 279, base:  99, extraBits: 4 },
  { symbol: 280, base: 115, extraBits: 4 },
  { symbol: 281, base: 131, extraBits: 5 },
  { symbol: 282, base: 163, extraBits: 5 },
  { symbol: 283, base: 195, extraBits: 5 },
  { symbol: 284, base: 227, extraBits: 5 },
];

const LENGTH_BASE = new Map<number, number>();
const LENGTH_EXTRA = new Map<number, number>();
for (const e of LENGTH_TABLE) {
  LENGTH_BASE.set(e.symbol, e.base);
  LENGTH_EXTRA.set(e.symbol, e.extraBits);
}

// ---------------------------------------------------------------------------
// Distance code table (codes 0-23)
// ---------------------------------------------------------------------------

interface DistEntry {
  code: number;
  base: number;
  extraBits: number;
}

const DIST_TABLE: DistEntry[] = [
  { code:  0, base:    1, extraBits:  0 },
  { code:  1, base:    2, extraBits:  0 },
  { code:  2, base:    3, extraBits:  0 },
  { code:  3, base:    4, extraBits:  0 },
  { code:  4, base:    5, extraBits:  1 },
  { code:  5, base:    7, extraBits:  1 },
  { code:  6, base:    9, extraBits:  2 },
  { code:  7, base:   13, extraBits:  2 },
  { code:  8, base:   17, extraBits:  3 },
  { code:  9, base:   25, extraBits:  3 },
  { code: 10, base:   33, extraBits:  4 },
  { code: 11, base:   49, extraBits:  4 },
  { code: 12, base:   65, extraBits:  5 },
  { code: 13, base:   97, extraBits:  5 },
  { code: 14, base:  129, extraBits:  6 },
  { code: 15, base:  193, extraBits:  6 },
  { code: 16, base:  257, extraBits:  7 },
  { code: 17, base:  385, extraBits:  7 },
  { code: 18, base:  513, extraBits:  8 },
  { code: 19, base:  769, extraBits:  8 },
  { code: 20, base: 1025, extraBits:  9 },
  { code: 21, base: 1537, extraBits:  9 },
  { code: 22, base: 2049, extraBits: 10 },
  { code: 23, base: 3073, extraBits: 10 },
];

const DIST_BASE = new Map<number, number>();
const DIST_EXTRA = new Map<number, number>();
for (const e of DIST_TABLE) {
  DIST_BASE.set(e.code, e.base);
  DIST_EXTRA.set(e.code, e.extraBits);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function lengthSymbol(length: number): number {
  for (const e of LENGTH_TABLE) {
    const maxLen = e.base + (1 << e.extraBits) - 1;
    if (length <= maxLen) return e.symbol;
  }
  return 284;
}

function distCode(offset: number): number {
  for (const e of DIST_TABLE) {
    const maxDist = e.base + (1 << e.extraBits) - 1;
    if (offset <= maxDist) return e.code;
  }
  return 23;
}

// ---------------------------------------------------------------------------
// Bit I/O
// ---------------------------------------------------------------------------

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
// Public API: compress
// ---------------------------------------------------------------------------

/**
 * Compress data using DEFLATE (CMP05) and return wire-format bytes.
 *
 * @param data - The raw bytes to compress.
 * @returns Compressed bytes in CMP05 wire format.
 */
export function compress(data: Uint8Array): Uint8Array {
  const originalLength = data.length;

  if (originalLength === 0) {
    // Empty input: LL tree has only symbol 256 (end-of-data), code "0".
    const out = new Uint8Array(12);
    const view = new DataView(out.buffer);
    view.setUint32(0, 0, false);
    view.setUint16(4, 1, false); // ll_entry_count = 1
    view.setUint16(6, 0, false); // dist_entry_count = 0
    view.setUint16(8, 256, false); // symbol = 256
    out[10] = 1; // code_length = 1
    out[11] = 0x00; // bit stream: "0" padded
    return out;
  }

  // ── Pass 1: LZSS tokenization ────────────────────────────────────────────
  const tokens: Token[] = lzssEncode(data, 4096, 255, 3);

  // ── Pass 2a: Tally frequencies ───────────────────────────────────────────
  const llFreq = new Map<number, number>();
  const distFreq = new Map<number, number>();

  for (const tok of tokens) {
    if (tok.kind === "literal") {
      llFreq.set(tok.byte, (llFreq.get(tok.byte) ?? 0) + 1);
    } else {
      const sym = lengthSymbol(tok.length);
      llFreq.set(sym, (llFreq.get(sym) ?? 0) + 1);
      const dc = distCode(tok.offset);
      distFreq.set(dc, (distFreq.get(dc) ?? 0) + 1);
    }
  }
  llFreq.set(256, (llFreq.get(256) ?? 0) + 1);

  // ── Pass 2b: Build canonical Huffman trees ───────────────────────────────
  const llTree = HuffmanTree.build([...llFreq.entries()]);
  const llCodeTable = llTree.canonicalCodeTable(); // Map<number, string>

  let distCodeTable = new Map<number, string>();
  if (distFreq.size > 0) {
    const distTree = HuffmanTree.build([...distFreq.entries()]);
    distCodeTable = distTree.canonicalCodeTable();
  }

  // ── Pass 2c: Encode token stream ─────────────────────────────────────────
  const bb = new BitBuilder();
  for (const tok of tokens) {
    if (tok.kind === "literal") {
      const code = llCodeTable.get(tok.byte)!;
      bb.writeBitString(code);
    } else {
      const sym = lengthSymbol(tok.length);
      const code = llCodeTable.get(sym)!;
      bb.writeBitString(code);
      const extra = LENGTH_EXTRA.get(sym)!;
      const extraVal = tok.length - LENGTH_BASE.get(sym)!;
      bb.writeRawBitsLSB(extraVal, extra);

      const dc = distCode(tok.offset);
      const dcode = distCodeTable.get(dc)!;
      bb.writeBitString(dcode);
      const dextra = DIST_EXTRA.get(dc)!;
      const dextraVal = tok.offset - DIST_BASE.get(dc)!;
      bb.writeRawBitsLSB(dextraVal, dextra);
    }
  }
  bb.writeBitString(llCodeTable.get(256)!);
  bb.flush();
  const packedBits = bb.bytes();

  // ── Assemble wire format ─────────────────────────────────────────────────
  const llPairs: [number, number][] = [...llCodeTable.entries()].map(
    ([sym, code]) => [sym, code.length]
  );
  llPairs.sort(([sa, la], [sb, lb]) => la !== lb ? la - lb : sa - sb);

  const distPairs: [number, number][] = [...distCodeTable.entries()].map(
    ([sym, code]) => [sym, code.length]
  );
  distPairs.sort(([sa, la], [sb, lb]) => la !== lb ? la - lb : sa - sb);

  const headerSize = 8;
  const llTableSize = llPairs.length * 3;
  const distTableSize = distPairs.length * 3;
  const totalSize = headerSize + llTableSize + distTableSize + packedBits.length;
  const out = new Uint8Array(totalSize);
  const view = new DataView(out.buffer);

  view.setUint32(0, originalLength, false);
  view.setUint16(4, llPairs.length, false);
  view.setUint16(6, distPairs.length, false);

  let offset = 8;
  for (const [sym, len] of llPairs) {
    view.setUint16(offset, sym, false);
    out[offset + 2] = len;
    offset += 3;
  }
  for (const [sym, len] of distPairs) {
    view.setUint16(offset, sym, false);
    out[offset + 2] = len;
    offset += 3;
  }
  out.set(packedBits, offset);

  return out;
}

// ---------------------------------------------------------------------------
// Public API: decompress
// ---------------------------------------------------------------------------

/**
 * Decompress CMP05 wire-format data and return the original bytes.
 *
 * @param data - Compressed bytes produced by compress().
 * @returns Original uncompressed bytes.
 */
export function decompress(data: Uint8Array): Uint8Array {
  if (data.length < 8) return new Uint8Array(0);

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const originalLength = view.getUint32(0, false);
  const llEntryCount = view.getUint16(4, false);
  const distEntryCount = view.getUint16(6, false);

  if (originalLength === 0) return new Uint8Array(0);

  let off = 8;

  // Parse LL code-length table.
  const llLengths: [number, number][] = [];
  for (let i = 0; i < llEntryCount; i++) {
    const sym = view.getUint16(off, false);
    const clen = data[off + 2];
    llLengths.push([sym, clen]);
    off += 3;
  }

  // Parse dist code-length table.
  const distLengths: [number, number][] = [];
  for (let i = 0; i < distEntryCount; i++) {
    const sym = view.getUint16(off, false);
    const clen = data[off + 2];
    distLengths.push([sym, clen]);
    off += 3;
  }

  // Reconstruct canonical codes.
  const llRevMap = reconstructCanonicalCodes(llLengths);
  const distRevMap = reconstructCanonicalCodes(distLengths);

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

  // Decode token stream.
  const output: number[] = [];
  while (true) {
    const llSym = nextHuffmanSymbol(llRevMap);

    if (llSym === 256) {
      break; // end-of-data
    } else if (llSym < 256) {
      output.push(llSym); // literal byte
    } else {
      // Length code 257-284.
      const extra = LENGTH_EXTRA.get(llSym)!;
      const length = LENGTH_BASE.get(llSym)! + readBits(extra);

      const distSym = nextHuffmanSymbol(distRevMap);
      const dextra = DIST_EXTRA.get(distSym)!;
      const distOffset = DIST_BASE.get(distSym)! + readBits(dextra);

      // Copy byte-by-byte (supports overlapping matches).
      const start = output.length - distOffset;
      for (let i = 0; i < length; i++) {
        output.push(output[start + i]);
      }
    }
  }

  return new Uint8Array(output);
}
