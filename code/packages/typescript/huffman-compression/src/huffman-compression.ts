// =============================================================================
// CMP04: Huffman Compression
// =============================================================================
//
// Huffman (1952) lossless compression and decompression in TypeScript.
// Part of the CMP compression series in the coding-adventures monorepo.
//
// What Is Huffman Compression?
// ----------------------------
//
// Huffman coding is an **entropy coding** algorithm: it assigns variable-length,
// prefix-free binary codes to symbols based on their frequency of occurrence.
// Frequent symbols get short codes; rare symbols get long codes. The resulting
// code is provably optimal — no other prefix-free code can achieve a smaller
// expected bit-length for the same symbol distribution.
//
// Think of Morse code. In Morse, 'E' is '.' (one dot) because 'E' is the most
// common letter in English, while 'Z' is '--..'. Huffman's algorithm does this
// automatically and optimally for ANY alphabet with ANY frequency distribution.
//
// Unlike the LZ-family algorithms (CMP00–CMP03) which exploit **repetition**
// (duplicate substrings), Huffman coding exploits **symbol statistics**. It
// works on individual symbol frequencies, not patterns of repetition. This
// makes it complementary to LZ compression and explains why DEFLATE (CMP05)
// combines both: LZ77 to eliminate repeated substrings, then Huffman to
// optimally encode the remaining symbol stream.
//
// Dependency on DT27
// ------------------
//
// This package does NOT build its own Huffman tree. It imports
// `@coding-adventures/huffman-tree` (DT27) and delegates all tree construction
// and code derivation to that package. This mirrors the pattern used by LZ78
// (CMP01) which delegates trie operations to `trie` (DT13).
//
//   CMP01 (LZ78)    →  uses DT13 (Trie)        for dictionary management
//   CMP04 (Huffman) →  uses DT27 (HuffmanTree)  for code construction/decode
//
// Wire Format (CMP04)
// -------------------
//
//   Bytes 0–3:    original_length  (big-endian uint32)
//   Bytes 4–7:    symbol_count     (big-endian uint32) — number of distinct bytes
//   Bytes 8–8+2N: code-lengths table — N entries, each 2 bytes:
//                   [0] symbol value  (uint8, 0–255)
//                   [1] code length   (uint8, 1–16)
//                 Sorted by (code_length, symbol_value) ascending.
//   Bytes 8+2N+:  bit stream — packed LSB-first, zero-padded to byte boundary.
//
// Why LSB-first packing?
// ----------------------
//
// When we pack bits into bytes, we fill each byte starting from the
// least-significant bit (bit 0) and working up to bit 7. This is the same
// convention used by LZW (CMP03) and GIF compression.
//
// Example packing "10100" (5 bits) into 1 byte:
//   bit 0 ('1') → byte bit 0  (value: 0b00000001)
//   bit 1 ('0') → byte bit 1  (value: 0b00000001)
//   bit 2 ('1') → byte bit 2  (value: 0b00000101)
//   bit 3 ('0') → byte bit 3  (value: 0b00000101)
//   bit 4 ('0') → byte bit 4  (value: 0b00000101)
//   Result byte: 0b00000101 = 0x05 (high 3 bits zero-padded)
//
// The Series: CMP00 -> CMP05
// --------------------------
//
//   CMP00 (LZ77,    1977) — Sliding-window backreferences.
//   CMP01 (LZ78,    1978) — Explicit dictionary (trie), no sliding window.
//   CMP02 (LZSS,    1982) — LZ77 + flag bits; eliminates wasted literals.
//   CMP03 (LZW,     1984) — LZ78 + pre-initialized dict; powers GIF.
//   CMP04 (Huffman, 1952) — Entropy coding; prerequisite for DEFLATE. (this module)
//   CMP05 (DEFLATE, 1996) — LZ77 + Huffman; ZIP/gzip/PNG/zlib standard.
// =============================================================================

import { HuffmanTree } from "@coding-adventures/huffman-tree";

// =============================================================================
// Bit I/O helpers
// =============================================================================

/**
 * Pack a bit string into a Uint8Array, filling each byte from LSB upward.
 *
 * This is the "LSB-first" packing convention used by LZW (CMP03) and GIF:
 * the first bit of the stream occupies bit 0 (the least-significant bit) of
 * the first byte, the second bit occupies bit 1, and so on.
 *
 * Example — packing "000101011" (9 bits):
 *   Byte 0: bits[0..7]
 *     bit 0 ('0') → byte bit 0
 *     bit 1 ('0') → byte bit 1
 *     bit 2 ('0') → byte bit 2
 *     bit 3 ('1') → byte bit 3
 *     bit 4 ('0') → byte bit 4
 *     bit 5 ('1') → byte bit 5
 *     bit 6 ('0') → byte bit 6
 *     bit 7 ('1') → byte bit 7
 *     → 0b10101000 = 0xA8
 *   Byte 1: bit[8] ('1') → 0b00000001 = 0x01
 *
 * @param bits - A string of '0' and '1' characters.
 * @returns Packed bytes, zero-padded to the next byte boundary.
 */
function packBitsLsbFirst(bits: string): Uint8Array {
  const output: number[] = [];
  let buffer = 0;
  let bitPos = 0;

  for (const b of bits) {
    // Each bit character is '0' or '1'.
    // We shift it into the correct bit position within the current byte.
    buffer |= parseInt(b) << bitPos;
    bitPos++;
    if (bitPos === 8) {
      // Current byte is full — push it and reset for the next byte.
      output.push(buffer);
      buffer = 0;
      bitPos = 0;
    }
  }

  if (bitPos > 0) {
    // Final partial byte — remaining high bits are implicitly zero (zero-padded).
    output.push(buffer);
  }

  return new Uint8Array(output);
}

// =============================================================================
// Canonical code reconstruction
// =============================================================================

/** Symbols are single bytes, so a valid table has at most 256 entries. */
const MAX_SYMBOLS = 256;

/** Longest code length the decoder accepts. */
const MAX_CODE_LEN = 56;

/**
 * A binary decoding trie stored in flat typed arrays.
 *
 * `child[2 * node + bit]` is the next node (or -1), and `symbol[node]` is the
 * decoded byte at a leaf (or -1 for an interior node). Walking one bit is two
 * array reads, so decoding is O(1) per bit and builds no strings.
 */
interface DecodeTrie {
  child: Int32Array;
  symbol: Int16Array;
}

/**
 * Reconstruct canonical Huffman codes from a sorted (symbol, length) list and
 * load them into a decoding trie.
 *
 * This is the DEFLATE-style canonical reconstruction algorithm. Given only
 * the code lengths (not the actual tree), it produces the exact same codes
 * that the encoder used.
 *
 * Algorithm:
 *   code = 0
 *   prevLen = lengths[0][1]
 *   for each (symbol, len) in order:
 *     if len > prevLen: code <<= (len - prevLen)   // left-shift for longer codes
 *     assign code (len bits) to symbol
 *     code++
 *     prevLen = len
 *
 * SECURITY: the table is hostile input. We reject
 *   - lengths of 0 or above MAX_CODE_LEN (a zero-length code matches nothing),
 *   - tables not sorted by length (the encoder always sorts), and
 *   - over-subscribed tables (Kraft sum > 1): before each assignment the
 *     running code must still fit in `len` bits, i.e. code >> len === 0.
 *
 * The code is a BigInt on purpose. JavaScript's `<<` operates on 32-bit
 * integers and masks the shift count to 5 bits, so `code << 40` would silently
 * compute `code << 8`, wrapping the table into something the Kraft check
 * cannot see. BigInt shifts are exact for any length up to MAX_CODE_LEN.
 *
 * @param lengths - Array of [symbol, codeLength] pairs sorted by
 *                  (codeLength, symbol) ascending — same order as wire format.
 * @returns The decoding trie and the longest code length.
 */
function canonicalCodesFromLengths(lengths: Array<[number, number]>): DecodeTrie {
  // Each code adds at most `len` nodes; +1 for the root.
  let maxNodes = 1;
  for (const [, len] of lengths) maxNodes += Math.min(len, MAX_CODE_LEN);
  const child = new Int32Array(2 * maxNodes).fill(-1);
  const symbol = new Int16Array(maxNodes).fill(-1);
  let nodeCount = 1;

  let code = 0n;
  let prevLen = lengths.length > 0 ? lengths[0]![1] : 0;

  for (const [sym, len] of lengths) {
    if (len === 0 || len > MAX_CODE_LEN) {
      throw new Error(`invalid code length ${len} in table`);
    }
    if (len < prevLen) {
      throw new Error("code-lengths table is not sorted by length");
    }
    // Moving to a longer code length: shift left to align with new level.
    code <<= BigInt(len - prevLen);
    if (code >> BigInt(len) !== 0n) {
      throw new Error("code-lengths table over-subscribes the code space");
    }

    // Insert the code into the trie, most significant bit first.
    let node = 0;
    for (let i = len - 1; i >= 0; i--) {
      if (symbol[node]! >= 0) {
        throw new Error("code-lengths table is not prefix-free");
      }
      const bit = Number((code >> BigInt(i)) & 1n);
      let next = child[2 * node + bit]!;
      if (next < 0) {
        next = nodeCount++;
        child[2 * node + bit] = next;
      }
      node = next;
    }
    symbol[node] = sym;

    code++;
    prevLen = len;
  }

  return { child, symbol };
}

// =============================================================================
// Public API
// =============================================================================

/**
 * Compress raw bytes using Huffman coding and return CMP04 wire-format bytes.
 *
 * Algorithm
 * ---------
 * 1. Count symbol frequencies (byte histogram).
 * 2. Build a Huffman tree via DT27 (HuffmanTree.build).
 * 3. Obtain canonical codes via DT27 (tree.canonicalCodeTable).
 * 4. Build the code-lengths table for the wire-format header:
 *    pairs of (symbol, codeLength) sorted by (codeLength, symbol_value).
 * 5. Encode the input byte-by-byte using the canonical code table.
 * 6. Pack the resulting bit string LSB-first into bytes.
 * 7. Assemble: header (8 bytes) + code-lengths table (2N bytes) + bit stream.
 *
 * Wire format output layout:
 *
 *   ┌──────────────┬──────────────┬─────────────────────────┬────────────┐
 *   │ 4 bytes      │ 4 bytes      │ N × 2 bytes             │ variable   │
 *   │ orig_length  │ symbol_count │ [(sym, len), ...]       │ bit stream │
 *   │ big-endian   │ big-endian   │ sorted by (len, sym)    │ LSB-first  │
 *   └──────────────┴──────────────┴─────────────────────────┴────────────┘
 *
 * Edge cases
 * ----------
 * - Empty input: returns an 8-byte header with original_length=0,
 *   symbol_count=0, and no bit data.
 * - Single distinct byte: DT27 assigns it code "0"; each occurrence
 *   encodes to 1 bit.
 *
 * @param data - The raw bytes to compress.
 * @returns Compressed data in CMP04 wire format.
 */
export function compress(data: Uint8Array): Uint8Array {
  const originalLength = data.length;

  // Empty input — 8-byte header only, no bit stream.
  if (originalLength === 0) {
    const header = new Uint8Array(8);
    // All zeros — original_length=0, symbol_count=0.
    return header;
  }

  // Step 1: Count frequencies — build a histogram of byte values.
  // Each key is a byte value (0–255); each value is its count in `data`.
  const freq = new Map<number, number>();
  for (const byte of data) {
    freq.set(byte, (freq.get(byte) ?? 0) + 1);
  }

  // Step 2: Build Huffman tree via DT27.
  // HuffmanTree.build takes [[symbol, frequency], ...] pairs.
  const tree = HuffmanTree.build([...freq.entries()]);

  // Step 3: Canonical code table {symbol -> bit_string}.
  // Canonical codes are DEFLATE-style: same code lengths as the tree, but
  // normalized so the decoder only needs the length table to reconstruct them.
  const table = tree.canonicalCodeTable();

  // Step 4: Build the code-lengths list sorted by (length, symbol) for the header.
  // This is what we store in the wire format — lengths, not codes.
  const lengths: Array<[number, number]> = [...table.entries()]
    .map(([sym, bits]): [number, number] => [sym, bits.length])
    .sort((a, b) => a[1] !== b[1] ? a[1] - b[1] : a[0] - b[0]);

  // Step 5: Encode each byte by concatenating its canonical bit string.
  let bitString = "";
  for (const byte of data) {
    bitString += table.get(byte)!;
  }

  // Step 6: Pack the bit string LSB-first into bytes.
  const bitBytes = packBitsLsbFirst(bitString);

  // Step 7: Assemble wire format.
  //   header:             original_length (4B big-endian) + symbol_count (4B big-endian)
  //   code-lengths table: symbol_count × 2 bytes — each entry: [symbol, length]
  //   bit stream:         variable length
  const symbolCount = lengths.length;
  const headerSize = 8;
  const tableSize = symbolCount * 2;
  const totalSize = headerSize + tableSize + bitBytes.length;

  const output = new Uint8Array(totalSize);
  const view = new DataView(output.buffer);

  // Write 4-byte big-endian original_length at offset 0.
  view.setUint32(0, originalLength, false); // false = big-endian
  // Write 4-byte big-endian symbol_count at offset 4.
  view.setUint32(4, symbolCount, false);

  // Write code-lengths table entries.
  // Each entry is 2 bytes: [symbol_value (uint8), code_length (uint8)].
  for (let i = 0; i < symbolCount; i++) {
    const [sym, len] = lengths[i]!;
    output[headerSize + 2 * i] = sym;
    output[headerSize + 2 * i + 1] = len;
  }

  // Write the packed bit stream after the header and code-lengths table.
  output.set(bitBytes, headerSize + tableSize);

  return output;
}

/**
 * Decompress CMP04 wire-format bytes and return the original raw bytes.
 *
 * Algorithm
 * ---------
 * 1. Parse the 8-byte header: original_length and symbol_count.
 * 2. Parse the code-lengths table (symbol_count × 2 bytes).
 * 3. Reconstruct canonical codes from the sorted (symbol, length) list.
 * 4. Unpack the LSB-first bit stream.
 * 5. Decode original_length symbols by matching accumulated bits against
 *    the canonical code table (prefix-free, so no separator needed).
 *
 * Prefix-free decoding
 * --------------------
 * Because Huffman codes are prefix-free, we can decode greedily: accumulate
 * bits one at a time until the accumulated string matches a code in the table,
 * emit that symbol, reset the accumulator, and repeat. No symbol's code is a
 * prefix of another's, so we can never match incorrectly.
 *
 * @param data - Compressed bytes produced by `compress`.
 * @returns The original, uncompressed bytes.
 */
export function decompress(data: Uint8Array): Uint8Array {
  if (data.length < 8) {
    return new Uint8Array(0);
  }

  // Step 1: Parse the 8-byte header using DataView for correct big-endian reads.
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  const originalLength = view.getUint32(0, false); // false = big-endian
  const symbolCount = view.getUint32(4, false);

  if (originalLength === 0) {
    return new Uint8Array(0);
  }

  // Step 2: Parse the code-lengths table.
  // Each entry: 2 bytes — symbol (uint8), code_length (uint8).
  // Wire format guarantees entries are sorted by (code_length, symbol_value).
  //
  // SECURITY: the header is attacker-controlled. Symbols are single bytes, so
  // a table can name at most 256 of them; refuse more before sizing anything.
  if (symbolCount > MAX_SYMBOLS) {
    throw new Error(`symbol_count ${symbolCount} exceeds ${MAX_SYMBOLS}`);
  }
  const tableOffset = 8;
  const bitsOffset = tableOffset + 2 * symbolCount;
  if (data.length < bitsOffset) {
    throw new Error("input too short for the code-lengths table");
  }
  const lengths: Array<[number, number]> = [];
  const seen = new Uint8Array(256);
  for (let i = 0; i < symbolCount; i++) {
    const off = tableOffset + 2 * i;
    const symbol = data[off]!;
    const length = data[off + 1]!;
    // A byte value can have only one code; a repeat makes the table ambiguous.
    if (seen[symbol]) {
      throw new Error(`symbol ${symbol} appears twice in code-lengths table`);
    }
    seen[symbol] = 1;
    lengths.push([symbol, length]);
  }

  // Step 3: Reconstruct canonical codes from the sorted lengths list (and
  // validate them) into a decoding trie.
  const { child, symbol: trieSymbol } = canonicalCodesFromLengths(lengths);

  // SECURITY: every code is at least one bit, so N payload bytes hold at most
  // 8*N symbols. Reject a larger declared length BEFORE allocating the output.
  const payload = data.subarray(bitsOffset);
  const totalBits = payload.length * 8;
  if (originalLength > totalBits) {
    throw new Error(
      `Bit stream exhausted: declared length ${originalLength} exceeds the ${totalBits} bits available`
    );
  }

  // Step 4+5: Decode exactly original_length symbols straight from the bytes.
  //
  // Bits are LSB-first within each byte, so bit i of the stream is
  // (payload[i >> 3] >> (i & 7)) & 1 — O(1) with no expanded bit string. We
  // walk the trie one bit at a time; because the code is prefix-free, the
  // first leaf reached is the symbol. A missing child means the bits match no
  // code (an incomplete table fed unmatched bits), so we fail immediately
  // instead of scanning the rest of the input.
  //
  // Example with "AAABBC" (A=0, B=10, C=11):
  //   bits: "0 0 0 10 10 11" -> A A A B B C
  const output = new Uint8Array(originalLength);
  let pos = 0;
  let node = 0;
  let decoded = 0;

  while (decoded < originalLength) {
    if (pos >= totalBits) {
      throw new Error(
        `Bit stream exhausted after ${decoded} symbols; expected ${originalLength}`
      );
    }
    const bit = (payload[pos >> 3]! >> (pos & 7)) & 1;
    pos++;

    node = child[2 * node + bit]!;
    if (node < 0) {
      throw new Error("bit pattern does not match any code");
    }
    const sym = trieSymbol[node]!;
    if (sym >= 0) {
      output[decoded] = sym;
      decoded++;
      node = 0;
    }
  }

  return output;
}
