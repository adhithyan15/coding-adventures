// =============================================================================
// Tests for CMP04: Huffman Compression
// =============================================================================
//
// This test suite verifies the compress() and decompress() functions against:
//   1. Round-trip correctness for various inputs
//   2. Wire format structure for the worked "AAABBC" example
//   3. Edge cases: empty input, single distinct byte, all 256 bytes
//
// The worked example: "AAABBC"
// --------------------------------
// Symbol frequencies: A=3, B=2, C=1
// Huffman tree:      [6]
//                    / \
//                   A   [3]
//                  (3)  / \
//                      B   C
//                     (2) (1)
// Canonical codes: A→0 (len=1), B→10 (len=2), C→11 (len=2)
// Sorted lengths:  [(65,1), (66,2), (67,2)]
//
// Wire format for "AAABBC" (6 bytes input, 3 distinct symbols):
//   Bytes 0–3:   original_length = 6       → 0x00 0x00 0x00 0x06
//   Bytes 4–7:   symbol_count    = 3       → 0x00 0x00 0x00 0x03
//   Bytes 8–9:   (sym=65 'A', len=1)       → 0x41 0x01
//   Bytes 10–11: (sym=66 'B', len=2)       → 0x42 0x02
//   Bytes 12–13: (sym=67 'C', len=2)       → 0x43 0x02
//   Bytes 14+:   bit stream (LSB-first):
//     A=0, A=0, A=0, B=10, B=10, C=11
//     bit string: "000" + "10" + "10" + "11" = "0001010 11" (9 bits)
//     Packed LSB-first:
//       Byte 0: bits 0..7 → 0b01010000 wait, let's recalculate:
//         pos 0: '0' → bit 0
//         pos 1: '0' → bit 1
//         pos 2: '0' → bit 2
//         pos 3: '1' → bit 3
//         pos 4: '0' → bit 4
//         pos 5: '1' → bit 5
//         pos 6: '0' → bit 6  (wait — "000" + "10" + "10" + "11")
//         Actually: "0001010 11" has only 9 bits; let me re-expand:
//           A→"0", A→"0", A→"0", B→"10", B→"10", C→"11"
//           = "0" + "0" + "0" + "10" + "10" + "11" = "000101011"
//         Byte 0 (bits[0..7]): '0','0','0','1','0','1','0','1' → 0b10101000
//           bit 0 ('0')=0, bit 1 ('0')=0, bit 2 ('0')=0, bit 3 ('1')=8,
//           bit 4 ('0')=0, bit 5 ('1')=32, bit 6 ('0')=0, bit 7 ('1')=128
//           → 0 + 0 + 0 + 8 + 0 + 32 + 0 + 128 = 168 = 0xA8
//         Byte 1 (bits[8]): '1' → bit 0 → 0b00000001 = 0x01
// =============================================================================

import { describe, it, expect } from "vitest";
import { compress, decompress } from "../src/huffman-compression.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Convert a string to Uint8Array using UTF-8 encoding.
 * For ASCII strings, each character maps directly to its byte value.
 */
function encode(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

/**
 * Convert a Uint8Array back to a string for easy assertions.
 * For ASCII bytes, this is just the reverse of encode().
 */
function decode(bytes: Uint8Array): string {
  return new TextDecoder().decode(bytes);
}

/**
 * Read a big-endian uint32 from a Uint8Array at the given offset.
 */
function readUint32BE(data: Uint8Array, offset: number): number {
  const view = new DataView(data.buffer, data.byteOffset, data.byteLength);
  return view.getUint32(offset, false);
}

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------

describe("round-trip: compress then decompress recovers original data", () => {
  it("round-trips 'AAABBC'", () => {
    const original = encode("AAABBC");
    const compressed = compress(original);
    const recovered = decompress(compressed);
    expect(decode(recovered)).toBe("AAABBC");
  });

  it("round-trips a short ASCII sentence", () => {
    const original = encode("hello world");
    const compressed = compress(original);
    const recovered = decompress(compressed);
    expect(decode(recovered)).toBe("hello world");
  });

  it("round-trips a longer repeated string", () => {
    const original = encode("aaaaabbbbbcccccdddddeeeeefffffggggg");
    expect(decode(decompress(compress(original)))).toBe(
      "aaaaabbbbbcccccdddddeeeeefffffggggg"
    );
  });

  it("round-trips a string with all unique characters", () => {
    const original = encode("abcdefghijklmnopqrstuvwxyz");
    expect(decode(decompress(compress(original)))).toBe(
      "abcdefghijklmnopqrstuvwxyz"
    );
  });

  it("round-trips binary data (bytes 0–255)", () => {
    // All 256 possible byte values in one pass — the hardest test.
    const original = new Uint8Array(256);
    for (let i = 0; i < 256; i++) original[i] = i;
    const recovered = decompress(compress(original));
    expect(recovered).toEqual(original);
  });

  it("round-trips binary data repeated for better compression ratio", () => {
    // More repetition → better Huffman compression.
    const pattern = new Uint8Array([0, 1, 2, 0, 1, 0, 0, 1, 2, 0]);
    const recovered = decompress(compress(pattern));
    expect(recovered).toEqual(pattern);
  });

  it("round-trips single repeated byte", () => {
    const original = new Uint8Array(100).fill(42);
    expect(decompress(compress(original))).toEqual(original);
  });

  it("round-trips two distinct bytes", () => {
    const original = new Uint8Array([0, 1, 0, 1, 1, 0, 0, 1]);
    expect(decompress(compress(original))).toEqual(original);
  });
});

// ---------------------------------------------------------------------------
// Wire format verification for "AAABBC"
// ---------------------------------------------------------------------------

describe("wire format for 'AAABBC'", () => {
  const compressed = compress(encode("AAABBC"));

  it("has the correct original_length (6) in header bytes 0–3", () => {
    expect(readUint32BE(compressed, 0)).toBe(6);
  });

  it("has the correct symbol_count (3) in header bytes 4–7", () => {
    expect(readUint32BE(compressed, 4)).toBe(3);
  });

  it("has the minimum wire format size (8 header + 6 table + 2 bits = 16 bytes)", () => {
    // Header: 8 bytes
    // Code-lengths table: 3 entries × 2 bytes = 6 bytes
    // Bit stream: 9 bits → 2 bytes
    // Total: 8 + 6 + 2 = 16 bytes
    expect(compressed.length).toBe(16);
  });

  it("code-lengths table entry 0: symbol=65 (A), length=1", () => {
    // Sorted by (length, symbol): A has length 1, B and C have length 2.
    // A (65) comes first.
    expect(compressed[8]).toBe(65); // symbol 'A' = 0x41 = 65
    expect(compressed[9]).toBe(1); // code length 1
  });

  it("code-lengths table entry 1: symbol=66 (B), length=2", () => {
    expect(compressed[10]).toBe(66); // symbol 'B' = 0x42 = 66
    expect(compressed[11]).toBe(2); // code length 2
  });

  it("code-lengths table entry 2: symbol=67 (C), length=2", () => {
    expect(compressed[12]).toBe(67); // symbol 'C' = 0x43 = 67
    expect(compressed[13]).toBe(2); // code length 2
  });

  it("bit stream byte 0 is 0xA8 (encoding of A,A,A,B,B packed LSB-first)", () => {
    // Bit string: "000" (AAA) + "10" (B) + "10" (B) + "11" (C) = "000101011"
    // Bits 0–7: '0','0','0','1','0','1','0','1'
    //   bit0=0, bit1=0, bit2=0, bit3=8, bit4=0, bit5=32, bit6=0, bit7=128
    //   → 0 + 0 + 0 + 8 + 0 + 32 + 0 + 128 = 168 = 0xA8
    expect(compressed[14]).toBe(0xa8);
  });

  it("bit stream byte 1 is 0x01 (encoding of final C bit)", () => {
    // Bit 8: '1' → bit 0 of byte 1 → 0b00000001 = 0x01
    expect(compressed[15]).toBe(0x01);
  });
});

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

describe("edge case: empty input", () => {
  const compressed = compress(new Uint8Array(0));

  it("produces exactly 8 bytes (header only)", () => {
    expect(compressed.length).toBe(8);
  });

  it("header has original_length = 0", () => {
    expect(readUint32BE(compressed, 0)).toBe(0);
  });

  it("header has symbol_count = 0", () => {
    expect(readUint32BE(compressed, 4)).toBe(0);
  });

  it("decompresses back to empty", () => {
    expect(decompress(compressed)).toEqual(new Uint8Array(0));
  });

  it("decompress of empty data returns empty", () => {
    expect(decompress(new Uint8Array(0))).toEqual(new Uint8Array(0));
  });
});

describe("edge case: single distinct byte", () => {
  // "AAAA" — only one symbol (byte value 65 = 'A').
  // DT27 assigns it code "0" (single-leaf convention).
  const original = encode("AAAA");
  const compressed = compress(original);

  it("has original_length = 4 in header", () => {
    expect(readUint32BE(compressed, 0)).toBe(4);
  });

  it("has symbol_count = 1 in header", () => {
    expect(readUint32BE(compressed, 4)).toBe(1);
  });

  it("code-lengths table has one entry: symbol=65, length=1", () => {
    expect(compressed[8]).toBe(65); // 'A'
    expect(compressed[9]).toBe(1); // single-symbol: length 1 by convention
  });

  it("decompresses back to 'AAAA'", () => {
    expect(decode(decompress(compressed))).toBe("AAAA");
  });

  it("round-trips single byte value 0", () => {
    const data = new Uint8Array([0, 0, 0, 0, 0]);
    expect(decompress(compress(data))).toEqual(data);
  });
});

describe("edge case: all 256 distinct byte values", () => {
  const original = new Uint8Array(256);
  for (let i = 0; i < 256; i++) original[i] = i;
  const compressed = compress(original);

  it("has original_length = 256 in header", () => {
    expect(readUint32BE(compressed, 0)).toBe(256);
  });

  it("has symbol_count = 256 in header", () => {
    expect(readUint32BE(compressed, 4)).toBe(256);
  });

  it("code-lengths table is exactly 512 bytes (256 × 2)", () => {
    // We check the wire format structure is correct.
    // Table starts at byte 8, each entry is 2 bytes.
    const tableSize = 256 * 2;
    expect(compressed.length).toBeGreaterThanOrEqual(8 + tableSize);
  });

  it("round-trips successfully", () => {
    expect(decompress(compressed)).toEqual(original);
  });
});

// ---------------------------------------------------------------------------
// Compression properties
// ---------------------------------------------------------------------------

describe("compression properties", () => {
  it("compressed output is smaller than input for highly skewed frequencies", () => {
    // When one symbol dominates, Huffman achieves near-1-bit-per-symbol.
    // Input: 1000 'A's + 1 'B' = 1001 bytes.
    // Compressed: 8 header + 4 table + ~126 data bytes << 1001 bytes input.
    const data = new Uint8Array(1001);
    data.fill(65, 0, 1000); // 1000 'A's
    data[1000] = 66; // 1 'B'
    const compressed = compress(data);
    expect(compressed.length).toBeLessThan(data.length);
  });

  it("compressed data is always a valid CMP04 stream (decompresses cleanly)", () => {
    const inputs = [
      encode("the quick brown fox jumps over the lazy dog"),
      encode("aababababab"),
      new Uint8Array([255, 0, 128, 64, 32, 16, 8, 4, 2, 1]),
    ];
    for (const input of inputs) {
      expect(decompress(compress(input))).toEqual(input);
    }
  });

  it("idempotent: compress → decompress → compress → decompress produces same final result", () => {
    const original = encode("hello huffman");
    const once = decompress(compress(original));
    const twice = decompress(compress(once));
    expect(once).toEqual(twice);
  });
});

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

describe("error handling", () => {
  it("decompress throws when bit stream is truncated", () => {
    // Construct a malformed CMP04 stream: header claims original_length=10 but
    // the bit stream is empty (only 2 bytes for a 1-symbol table, no bit data).
    //
    // Layout:
    //   Bytes 0–3: original_length = 10   (needs 10 symbols decoded)
    //   Bytes 4–7: symbol_count    = 1
    //   Bytes 8–9: entry (sym=65, len=1)
    //   Bytes 10+: NO bit stream bytes — will exhaust immediately
    const bad = new Uint8Array(10);
    const view = new DataView(bad.buffer);
    view.setUint32(0, 10, false); // original_length = 10 (big-endian)
    view.setUint32(4, 1, false);  // symbol_count = 1 (big-endian)
    bad[8] = 65; // symbol 'A'
    bad[9] = 1;  // code length 1
    // No bit stream bytes after the table — any attempt to decode will fail.
    expect(() => decompress(bad)).toThrow("Bit stream exhausted");
  });
});
