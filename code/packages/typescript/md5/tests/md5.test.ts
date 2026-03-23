/**
 * MD5 Test Suite
 *
 * Tests are organized by category:
 *   1. Package metadata
 *   2. RFC 1321 official test vectors (the gold standard)
 *   3. toHex utility
 *   4. Output format (length, type)
 *   5. Little-endian encoding verification
 *   6. T-table and algorithm internals (black-box checks via known vectors)
 *   7. Block boundary conditions
 *   8. Edge cases (empty, single byte, large input)
 *   9. MD5Hasher streaming API
 *  10. MD5Hasher copy()
 *  11. md5Hex convenience function
 *  12. Equivalence: streaming == one-shot
 */

import { describe, it, expect } from "vitest";
import { VERSION, md5, md5Hex, toHex, MD5Hasher } from "../src/index.js";

// ─── Helper ──────────────────────────────────────────────────────────────────
// Encode a UTF-8 string to Uint8Array for use in test inputs.

function enc(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

// ─── 1. Package Metadata ─────────────────────────────────────────────────────

describe("package", () => {
  it("exports VERSION 0.1.0", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ─── 2. RFC 1321 Official Test Vectors ───────────────────────────────────────
//
// These vectors are taken directly from RFC 1321 Appendix A.5. Any correct
// MD5 implementation must produce exactly these outputs. If any of these fail,
// the implementation is wrong — not the test.

describe("RFC 1321 test vectors", () => {
  it('md5("") === d41d8cd98f00b204e9800998ecf8427e', () => {
    expect(md5Hex(enc(""))).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it('md5("a") === 0cc175b9c0f1b6a831c399e269772661', () => {
    expect(md5Hex(enc("a"))).toBe("0cc175b9c0f1b6a831c399e269772661");
  });

  it('md5("abc") === 900150983cd24fb0d6963f7d28e17f72', () => {
    expect(md5Hex(enc("abc"))).toBe("900150983cd24fb0d6963f7d28e17f72");
  });

  it('md5("message digest") === f96b697d7cb7938d525a2f31aaf161d0', () => {
    expect(md5Hex(enc("message digest"))).toBe("f96b697d7cb7938d525a2f31aaf161d0");
  });

  it('md5("abcdefghijklmnopqrstuvwxyz") === c3fcd3d76192e4007dfb496cca67e13b', () => {
    expect(md5Hex(enc("abcdefghijklmnopqrstuvwxyz"))).toBe(
      "c3fcd3d76192e4007dfb496cca67e13b"
    );
  });

  it('md5("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789") === d174ab98d277d9f5a5611c2c9f419d9f', () => {
    expect(
      md5Hex(enc("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"))
    ).toBe("d174ab98d277d9f5a5611c2c9f419d9f");
  });

  it('md5("12345678901234567890...") === 57edf4a22be3c955ac49da2e2107b67a', () => {
    expect(
      md5Hex(enc("12345678901234567890123456789012345678901234567890123456789012345678901234567890"))
    ).toBe("57edf4a22be3c955ac49da2e2107b67a");
  });
});

// ─── 3. toHex Utility ────────────────────────────────────────────────────────

describe("toHex", () => {
  it("converts empty array to empty string", () => {
    expect(toHex(new Uint8Array(0))).toBe("");
  });

  it("converts single zero byte to '00'", () => {
    expect(toHex(new Uint8Array([0x00]))).toBe("00");
  });

  it("converts single 0xff byte to 'ff'", () => {
    expect(toHex(new Uint8Array([0xff]))).toBe("ff");
  });

  it("converts multi-byte array correctly", () => {
    expect(toHex(new Uint8Array([0xd4, 0x1d, 0x8c, 0xd9]))).toBe("d41d8cd9");
  });

  it("produces lowercase hex only", () => {
    const hex = toHex(new Uint8Array([0xab, 0xcd, 0xef]));
    expect(hex).toMatch(/^[0-9a-f]+$/);
  });

  it("pads single-digit hex values with leading zero", () => {
    // 0x0f should become "0f", not "f"
    expect(toHex(new Uint8Array([0x0f]))).toBe("0f");
  });
});

// ─── 4. Output Format ────────────────────────────────────────────────────────

describe("md5 output format", () => {
  it("returns a Uint8Array", () => {
    const result = md5(enc("abc"));
    expect(result).toBeInstanceOf(Uint8Array);
  });

  it("returns exactly 16 bytes", () => {
    expect(md5(enc("")).length).toBe(16);
    expect(md5(enc("abc")).length).toBe(16);
    expect(md5(enc("x".repeat(1000))).length).toBe(16);
  });

  it("md5Hex returns a string of length 32", () => {
    expect(md5Hex(enc("abc")).length).toBe(32);
  });

  it("md5Hex returns lowercase hex only", () => {
    expect(md5Hex(enc("abc"))).toMatch(/^[0-9a-f]{32}$/);
  });

  it("md5Hex is consistent with toHex(md5(...))", () => {
    const data = enc("hello world");
    expect(md5Hex(data)).toBe(toHex(md5(data)));
  });
});

// ─── 5. Little-Endian Encoding Verification ──────────────────────────────────
//
// The defining feature of MD5 vs SHA-1 is its little-endian byte order.
// We can verify this directly by checking the first four bytes of the empty
// string digest against what little-endian encoding of the first state word
// would produce.
//
// md5("") = d41d8cd98f00b204e9800998ecf8427e
//
// The first state word after hashing "" is 0xD98C1DD4.
// In little-endian: D4 1D 8C D9 → "d41d8cd9" ✓
// In big-endian:    D9 8C 1D D4 → "d98c1dd4" ✗
//
// This test would FAIL for a big-endian implementation.

describe("little-endian encoding", () => {
  it("empty string digest starts with d4 (little-endian first byte)", () => {
    const digest = md5(enc(""));
    // First byte of d41d8cd98f00b204e9800998ecf8427e is 0xd4
    expect(digest[0]).toBe(0xd4);
  });

  it("empty string digest second byte is 1d", () => {
    const digest = md5(enc(""));
    expect(digest[1]).toBe(0x1d);
  });

  it("empty string produces correct full digest (endianness check)", () => {
    // This vector would be different if big-endian encoding were used.
    expect(md5Hex(enc(""))).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it("'a' digest endianness check", () => {
    // Would be different with big-endian.
    expect(md5Hex(enc("a"))).toBe("0cc175b9c0f1b6a831c399e269772661");
  });
});

// ─── 6. Additional Known Vectors ──────────────────────────────────────────────
//
// These vectors come from multiple independent implementations to cross-check
// our result against the reference.

describe("additional known vectors", () => {
  it('md5("The quick brown fox jumps over the lazy dog")', () => {
    expect(md5Hex(enc("The quick brown fox jumps over the lazy dog"))).toBe(
      "9e107d9d372bb6826bd81d3542a419d6"
    );
  });

  it('md5("The quick brown fox jumps over the lazy dog.") (with period)', () => {
    // One character difference causes completely different output (avalanche effect).
    expect(md5Hex(enc("The quick brown fox jumps over the lazy dog."))).toBe(
      "e4d909c290d0fb1ca068ffaddf22cbd0"
    );
  });

  it('md5("Hello, World!")', () => {
    expect(md5Hex(enc("Hello, World!"))).toBe("65a8e27d8879283831b664bd8b7f0ad4");
  });

  it("md5 of all-zeros 16 bytes", () => {
    expect(md5Hex(new Uint8Array(16))).toBe("4ae71336e44bf9bf79d2752e234818a5");
  });

  it("md5 of all-zeros 64 bytes (one full block of zeros)", () => {
    expect(md5Hex(new Uint8Array(64))).toBe("3b5d3c7d207e37dceeedd301e35e2e58");
  });
});

// ─── 7. Block Boundary Conditions ────────────────────────────────────────────
//
// MD5 processes 64-byte blocks. The padding algorithm is most interesting at
// lengths near multiples of 64:
//
//   55 bytes → fits in one block with 0x80 + 8 length bytes (exactly 64)
//   56 bytes → needs TWO blocks (0x80 pushes it past the 56-byte threshold)
//   63 bytes → needs two blocks
//   64 bytes → exactly one block of data + one block of padding
//   128 bytes → exactly two blocks of data + one block of padding

describe("block boundary conditions", () => {
  it("55-byte input (fits in single padding block)", () => {
    const data = new Uint8Array(55).fill(0x61); // 55 'a's
    const result = md5Hex(data);
    expect(result).toHaveLength(32);
    // Verify with streaming produces same result
    expect(new MD5Hasher().update(data).hexDigest()).toBe(result);
  });

  it("56-byte input (forces extra padding block)", () => {
    // 56 bytes of data: after appending 0x80, we're at 57 bytes.
    // 57 % 64 = 57 > 56, so we need another full 64-byte block just for padding.
    const data = new Uint8Array(56).fill(0x61); // 56 'a's
    const result = md5Hex(data);
    expect(result).toHaveLength(32);
    expect(new MD5Hasher().update(data).hexDigest()).toBe(result);
  });

  it("63-byte input", () => {
    const data = new Uint8Array(63).fill(0x62); // 63 'b's
    const result = md5Hex(data);
    expect(result).toHaveLength(32);
    expect(new MD5Hasher().update(data).hexDigest()).toBe(result);
  });

  it("64-byte input (exactly one block of data)", () => {
    const data = new Uint8Array(64).fill(0x63); // 64 'c's
    const result = md5Hex(data);
    expect(result).toHaveLength(32);
    expect(new MD5Hasher().update(data).hexDigest()).toBe(result);
  });

  it("65-byte input (one full block + 1 overflow byte)", () => {
    const data = new Uint8Array(65).fill(0x64); // 65 'd's
    const result = md5Hex(data);
    expect(result).toHaveLength(32);
    expect(new MD5Hasher().update(data).hexDigest()).toBe(result);
  });

  it("128-byte input (two full blocks of data)", () => {
    const data = new Uint8Array(128).fill(0x65); // 128 'e's
    const result = md5Hex(data);
    expect(result).toHaveLength(32);
    expect(new MD5Hasher().update(data).hexDigest()).toBe(result);
  });
});

// ─── 8. Edge Cases ────────────────────────────────────────────────────────────

describe("edge cases", () => {
  it("empty input", () => {
    expect(md5Hex(new Uint8Array(0))).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it("single byte 0x00", () => {
    expect(md5Hex(new Uint8Array([0x00]))).toBe("93b885adfe0da089cdf634904fd59f71");
  });

  it("single byte 0xff", () => {
    expect(md5Hex(new Uint8Array([0xff]))).toBe("00594fd4f42ba43fc1ca0427a0576295");
  });

  it("binary data with all byte values 0x00–0xff", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    const result = md5Hex(data);
    expect(result).toHaveLength(32);
    expect(result).toMatch(/^[0-9a-f]{32}$/);
  });

  it("same input always produces same output (determinism)", () => {
    const data = enc("determinism test");
    expect(md5Hex(data)).toBe(md5Hex(data));
  });

  it("different inputs produce different outputs (collision resistance)", () => {
    expect(md5Hex(enc("foo"))).not.toBe(md5Hex(enc("bar")));
    expect(md5Hex(enc("hello"))).not.toBe(md5Hex(enc("hello ")));
  });

  it("large input (10,000 bytes)", () => {
    const data = new Uint8Array(10_000).fill(0x41); // 10000 'A's
    const result = md5Hex(data);
    expect(result).toHaveLength(32);
    expect(result).toMatch(/^[0-9a-f]{32}$/);
  });
});

// ─── 9. MD5Hasher Streaming API ───────────────────────────────────────────────

describe("MD5Hasher streaming", () => {
  it("empty update produces same as md5 empty", () => {
    const h = new MD5Hasher();
    h.update(new Uint8Array(0));
    expect(h.hexDigest()).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it("no update at all produces empty hash", () => {
    expect(new MD5Hasher().hexDigest()).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it("single update with 'abc'", () => {
    const h = new MD5Hasher();
    h.update(enc("abc"));
    expect(h.hexDigest()).toBe("900150983cd24fb0d6963f7d28e17f72");
  });

  it("two updates: 'ab' + 'c' === md5('abc')", () => {
    const h = new MD5Hasher();
    h.update(enc("ab")).update(enc("c"));
    expect(h.hexDigest()).toBe("900150983cd24fb0d6963f7d28e17f72");
  });

  it("many single-byte updates", () => {
    const h = new MD5Hasher();
    for (const byte of enc("message digest")) {
      h.update(new Uint8Array([byte]));
    }
    expect(h.hexDigest()).toBe("f96b697d7cb7938d525a2f31aaf161d0");
  });

  it("update returns this for chaining", () => {
    const h = new MD5Hasher();
    const returned = h.update(enc("a"));
    expect(returned).toBe(h);
  });

  it("digest() is non-destructive (can call twice)", () => {
    const h = new MD5Hasher().update(enc("abc"));
    const d1 = h.hexDigest();
    const d2 = h.hexDigest();
    expect(d1).toBe(d2);
  });

  it("update after digest() continues correctly", () => {
    // Hashing "abc" then "def" is the same as hashing "abcdef".
    const h = new MD5Hasher().update(enc("abc"));
    h.hexDigest(); // call digest mid-stream
    h.update(enc("def"));
    expect(h.hexDigest()).toBe(md5Hex(enc("abcdef")));
  });

  it("hexDigest() matches toHex(digest())", () => {
    const h = new MD5Hasher().update(enc("hello"));
    expect(h.hexDigest()).toBe(toHex(h.digest()));
  });

  it("streaming 64-byte chunks matches one-shot", () => {
    // Feed 256 bytes in 64-byte chunks
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i & 0xff;

    const h = new MD5Hasher();
    for (let i = 0; i < 256; i += 64) {
      h.update(data.subarray(i, i + 64));
    }
    expect(h.hexDigest()).toBe(md5Hex(data));
  });

  it("streaming 1-byte chunks matches one-shot for 100 bytes", () => {
    const data = new Uint8Array(100).fill(0x42); // 100 'B's
    const h = new MD5Hasher();
    for (const byte of data) {
      h.update(new Uint8Array([byte]));
    }
    expect(h.hexDigest()).toBe(md5Hex(data));
  });
});

// ─── 10. MD5Hasher copy() ────────────────────────────────────────────────────

describe("MD5Hasher.copy()", () => {
  it("copy() produces same digest as original", () => {
    const h = new MD5Hasher().update(enc("abc"));
    const copy = h.copy();
    expect(copy.hexDigest()).toBe(h.hexDigest());
  });

  it("copy() is independent (modifying original doesn't affect copy)", () => {
    const h = new MD5Hasher().update(enc("abc"));
    const copy = h.copy();
    h.update(enc("def")); // modify original
    // copy should still be at "abc" only
    expect(copy.hexDigest()).toBe("900150983cd24fb0d6963f7d28e17f72");
  });

  it("copy() is independent (modifying copy doesn't affect original)", () => {
    const h = new MD5Hasher().update(enc("abc"));
    const copy = h.copy();
    copy.update(enc("def")); // modify copy
    // original should still be at "abc" only
    expect(h.hexDigest()).toBe("900150983cd24fb0d6963f7d28e17f72");
  });

  it("copy() of fresh hasher produces empty-string digest", () => {
    const copy = new MD5Hasher().copy();
    expect(copy.hexDigest()).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it("copy() allows computing multiple digests from common prefix", () => {
    // Hash a common prefix, then branch.
    const prefix = enc("common prefix ");
    const h = new MD5Hasher().update(prefix);

    const branch1 = h.copy().update(enc("branch1")).hexDigest();
    const branch2 = h.copy().update(enc("branch2")).hexDigest();

    expect(branch1).toBe(md5Hex(enc("common prefix branch1")));
    expect(branch2).toBe(md5Hex(enc("common prefix branch2")));
    expect(branch1).not.toBe(branch2);
  });
});

// ─── 11. md5Hex Convenience Function ─────────────────────────────────────────

describe("md5Hex", () => {
  it("returns 32-char lowercase hex for empty input", () => {
    expect(md5Hex(enc(""))).toBe("d41d8cd98f00b204e9800998ecf8427e");
  });

  it("returns 32-char lowercase hex for 'abc'", () => {
    expect(md5Hex(enc("abc"))).toBe("900150983cd24fb0d6963f7d28e17f72");
  });

  it("is consistent with toHex(md5(...))", () => {
    const data = enc("consistency test");
    expect(md5Hex(data)).toBe(toHex(md5(data)));
  });
});

// ─── 12. Equivalence: Streaming == One-Shot ───────────────────────────────────
//
// A core invariant: however the input is split across multiple update() calls,
// the final digest must equal the one-shot md5() result. We test several
// chunking patterns to stress the buffer management code.

describe("streaming == one-shot equivalence", () => {
  const testData = enc("The quick brown fox jumps over the lazy dog");

  it("single update equals one-shot", () => {
    expect(new MD5Hasher().update(testData).hexDigest()).toBe(md5Hex(testData));
  });

  it("split at byte 10 equals one-shot", () => {
    const h = new MD5Hasher()
      .update(testData.subarray(0, 10))
      .update(testData.subarray(10));
    expect(h.hexDigest()).toBe(md5Hex(testData));
  });

  it("split at byte 32 (half of 64-byte block) equals one-shot", () => {
    const h = new MD5Hasher()
      .update(testData.subarray(0, 32))
      .update(testData.subarray(32));
    expect(h.hexDigest()).toBe(md5Hex(testData));
  });

  it("three chunks equal one-shot", () => {
    const h = new MD5Hasher()
      .update(testData.subarray(0, 15))
      .update(testData.subarray(15, 30))
      .update(testData.subarray(30));
    expect(h.hexDigest()).toBe(md5Hex(testData));
  });

  it("cross-block boundary streaming for 128-byte input", () => {
    const big = new Uint8Array(128);
    for (let i = 0; i < 128; i++) big[i] = i & 0xff;

    // Split: 70 bytes (crosses first 64-byte block boundary) + 58 bytes
    const h = new MD5Hasher()
      .update(big.subarray(0, 70))
      .update(big.subarray(70));
    expect(h.hexDigest()).toBe(md5Hex(big));
  });
});
