/**
 * Tests for @coding-adventures/wasm-leb128
 *
 * Each test case maps to a specific aspect of the LEB128 specification.
 * Test vectors come from the WebAssembly spec and LLVM's LEB128 documentation.
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  VERSION,
  LEB128Error,
  decodeUnsigned,
  decodeSigned,
  encodeUnsigned,
  encodeSigned,
} from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Metadata
// ─────────────────────────────────────────────────────────────────────────────

describe("wasm-leb128 metadata", () => {
  it("has a version", () => {
    assert.equal(VERSION, "0.1.0");
  });

  it("exports LEB128Error as an Error subclass", () => {
    const err = new LEB128Error("test");
    assert.ok(err instanceof Error);
    assert.ok(err instanceof LEB128Error);
    assert.equal(err.name, "LEB128Error");
    assert.equal(err.message, "test");
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// decodeUnsigned
// ─────────────────────────────────────────────────────────────────────────────

describe("decodeUnsigned", () => {
  // Test case 1: Zero
  // [0x00] → value=0, bytesConsumed=1
  // 0x00 has no continuation bit, payload = 0.
  it("decodes zero", () => {
    const [value, bytes] = decodeUnsigned(new Uint8Array([0x00]));
    assert.equal(value, 0);
    assert.equal(bytes, 1);
  });

  // Test case 2: One-byte unsigned (single byte, no continuation bit)
  // [0x03] → value=3, bytesConsumed=1
  it("decodes one-byte value (3)", () => {
    const [value, bytes] = decodeUnsigned(new Uint8Array([0x03]));
    assert.equal(value, 3);
    assert.equal(bytes, 1);
  });

  // Test case 4: Multi-byte (the canonical ULEB128 example)
  // 624485 in binary: 0b0001_0011_0000_0111_0110_0101 (20 bits)
  //
  //   Split into 7-bit groups, least-significant first:
  //     bits 0–6:   0b110_0101 = 0x65 = 101  → byte 0: 0xE5 (continuation set)
  //     bits 7–13:  0b000_1110 = 0x0E = 14   → byte 1: 0x8E (continuation set)
  //     bits 14–19: 0b10_0110  = 0x26 = 38   → byte 2: 0x26 (last byte)
  //
  //   Encoded: [0xE5, 0x8E, 0x26]  (3 bytes)
  //
  //   Decode verification:
  //     101 | (14 << 7) | (38 << 14) = 101 | 1792 | 622592 = 624485  ✓
  //
  //   Note: A common reference uses [0xE5, 0x8E, 0x26]. Some older sources
  //   incorrectly cite [0xE5, 0x88, 0x26] which decodes to 623717, not 624485.
  it("decodes multi-byte value (624485)", () => {
    const [value, bytes] = decodeUnsigned(
      new Uint8Array([0xe5, 0x8e, 0x26])
    );
    assert.equal(value, 624485);
    assert.equal(bytes, 3);
  });

  // Test case 5: Maximum u32 value (4294967295 = 0xFFFFFFFF)
  // Encoded as [0xFF, 0xFF, 0xFF, 0xFF, 0x0F] — 5 bytes
  //
  //   byte 0: 0xFF → payload=0x7F=127, more follows
  //   byte 1: 0xFF → payload=0x7F=127, more follows
  //   byte 2: 0xFF → payload=0x7F=127, more follows
  //   byte 3: 0xFF → payload=0x7F=127, more follows
  //   byte 4: 0x0F → payload=0x0F=15,  last byte
  //
  //   result = 127 | (127<<7) | (127<<14) | (127<<21) | (15<<28)
  //          = 0x7F | 0x3F80 | 0x1FC000 | 0xFE00000 | 0xF0000000
  //          = 0xFFFFFFFF = 4294967295  ✓
  it("decodes max u32 (4294967295)", () => {
    const [value, bytes] = decodeUnsigned(
      new Uint8Array([0xff, 0xff, 0xff, 0xff, 0x0f])
    );
    assert.equal(value, 4294967295);
    assert.equal(bytes, 5);
  });

  // Test case 10: Non-zero offset
  // Reading from the middle of a byte array
  it("decodes at non-zero offset", () => {
    // Array: [0xAA, 0xE5, 0x8E, 0x26, 0xBB]
    //         ^^^^  ^^^^^^^^^^^^^^^^^^^^^ ^
    //         skip  624485 encoded here   skip
    const data = new Uint8Array([0xaa, 0xe5, 0x8e, 0x26, 0xbb]);
    const [value, bytes] = decodeUnsigned(data, 1);
    assert.equal(value, 624485);
    assert.equal(bytes, 3);
  });

  // Test case 9: Unterminated sequence
  // [0x80, 0x80] — both bytes have continuation bit set, no terminator
  it("throws LEB128Error for unterminated sequence", () => {
    assert.throws(
      () => decodeUnsigned(new Uint8Array([0x80, 0x80])),
      LEB128Error,
    );
  });

  // Empty array should also throw
  it("throws LEB128Error for empty input", () => {
    assert.throws(() => decodeUnsigned(new Uint8Array([])), LEB128Error);
  });

  // Sequence too long (more than 5 bytes with continuation bits)
  it("throws LEB128Error for sequence exceeding 5 bytes", () => {
    assert.throws(
      () => decodeUnsigned(
        new Uint8Array([0x80, 0x80, 0x80, 0x80, 0x80, 0x01]),
      ),
      LEB128Error,
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// decodeSigned
// ─────────────────────────────────────────────────────────────────────────────

describe("decodeSigned", () => {
  // Test case 1: Zero
  it("decodes zero", () => {
    const [value, bytes] = decodeSigned(new Uint8Array([0x00]));
    assert.equal(value, 0);
    assert.equal(bytes, 1);
  });

  // Test case 3: One-byte signed negative
  // [0x7E] → -2, bytesConsumed=1
  //
  //   0x7E = 0b0111_1110
  //   continuation bit = 0 → last byte
  //   payload = 0x7E & 0x7F = 0x7E = 126 = 0b111_1110
  //   bit 6 of 0x7E = 1 → sign extend
  //   result = 126 | -(1 << 7) = 126 | -128 = -2  ✓
  it("decodes one-byte signed negative (-2)", () => {
    const [value, bytes] = decodeSigned(new Uint8Array([0x7e]));
    assert.equal(value, -2);
    assert.equal(bytes, 1);
  });

  // Test case 6: Maximum i32 value (2147483647 = 0x7FFFFFFF)
  // [0xFF, 0xFF, 0xFF, 0xFF, 0x07]
  it("decodes max i32 (2147483647)", () => {
    const [value, bytes] = decodeSigned(
      new Uint8Array([0xff, 0xff, 0xff, 0xff, 0x07])
    );
    assert.equal(value, 2147483647);
    assert.equal(bytes, 5);
  });

  // Test case 7: Minimum i32 value (-2147483648 = 0x80000000)
  // [0x80, 0x80, 0x80, 0x80, 0x78]
  //
  //   byte 0: 0x80 → payload=0, continuation=1
  //   byte 1: 0x80 → payload=0, continuation=1
  //   byte 2: 0x80 → payload=0, continuation=1
  //   byte 3: 0x80 → payload=0, continuation=1
  //   byte 4: 0x78 → payload=0x78=120=0b111_1000, continuation=0
  //   result = 0 | (0<<7) | (0<<14) | (0<<21) | (120<<28)
  //          = 120 * 2^28 = ... but 120<<28 overflows 32 bits.
  //   After sign extension: result is -2147483648.  ✓
  it("decodes min i32 (-2147483648)", () => {
    const [value, bytes] = decodeSigned(
      new Uint8Array([0x80, 0x80, 0x80, 0x80, 0x78])
    );
    assert.equal(value, -2147483648);
    assert.equal(bytes, 5);
  });

  // Test case 10: Non-zero offset for signed
  it("decodes signed at non-zero offset", () => {
    // [0xFF, 0x7E, 0x00] — read 0x7E at offset 1 → -2
    const data = new Uint8Array([0xff, 0x7e, 0x00]);
    const [value, bytes] = decodeSigned(data, 1);
    assert.equal(value, -2);
    assert.equal(bytes, 1);
  });

  // Test case 9: Unterminated for signed
  it("throws LEB128Error for unterminated sequence", () => {
    assert.throws(
      () => decodeSigned(new Uint8Array([0x80, 0x80])),
      LEB128Error,
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// encodeUnsigned
// ─────────────────────────────────────────────────────────────────────────────

describe("encodeUnsigned", () => {
  it("encodes zero", () => {
    assert.deepEqual(encodeUnsigned(0), new Uint8Array([0x00]));
  });

  it("encodes small value (3)", () => {
    assert.deepEqual(encodeUnsigned(3), new Uint8Array([0x03]));
  });

  it("encodes 624485", () => {
    // 624485 = 0b0001_0011_0000_0111_0110_0101
    // 7-bit groups (LE): 0x65=101, 0x0E=14, 0x26=38
    // With continuation bits: [0xE5, 0x8E, 0x26]
    assert.deepEqual(
      encodeUnsigned(624485),
      new Uint8Array([0xe5, 0x8e, 0x26]),
    );
  });

  it("encodes max u32 (4294967295)", () => {
    assert.deepEqual(
      encodeUnsigned(4294967295),
      new Uint8Array([0xff, 0xff, 0xff, 0xff, 0x0f]),
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// encodeSigned
// ─────────────────────────────────────────────────────────────────────────────

describe("encodeSigned", () => {
  it("encodes zero", () => {
    assert.deepEqual(encodeSigned(0), new Uint8Array([0x00]));
  });

  it("encodes positive value (3)", () => {
    assert.deepEqual(encodeSigned(3), new Uint8Array([0x03]));
  });

  it("encodes -2", () => {
    assert.deepEqual(encodeSigned(-2), new Uint8Array([0x7e]));
  });

  it("encodes max i32 (2147483647)", () => {
    assert.deepEqual(
      encodeSigned(2147483647),
      new Uint8Array([0xff, 0xff, 0xff, 0xff, 0x07]),
    );
  });

  it("encodes min i32 (-2147483648)", () => {
    assert.deepEqual(
      encodeSigned(-2147483648),
      new Uint8Array([0x80, 0x80, 0x80, 0x80, 0x78]),
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Test case 8: Round-trip encode → decode (multiple values)
// ─────────────────────────────────────────────────────────────────────────────

describe("round-trip unsigned encode/decode", () => {
  const unsignedValues = [
    0, 1, 63, 64, 127, 128, 255, 256, 16383, 16384, 624485, 1000000,
    0x7fffffff, 0xffffffff,
  ];

  for (const v of unsignedValues) {
    it(`round-trips unsigned ${v}`, () => {
      const encoded = encodeUnsigned(v);
      const [decoded, bytesConsumed] = decodeUnsigned(encoded);
      assert.equal(decoded, v);
      assert.equal(bytesConsumed, encoded.length);
    });
  }
});

// Test case 11: Signed round-trip for negative values
describe("round-trip signed encode/decode", () => {
  const signedValues = [
    0, 1, -1, 63, -64, 64, -65, 127, -128, 128, -129, 2147483647,
    -2147483648, -1000000, -2,
  ];

  for (const v of signedValues) {
    it(`round-trips signed ${v}`, () => {
      const encoded = encodeSigned(v);
      const [decoded, bytesConsumed] = decodeSigned(encoded);
      assert.equal(decoded, v);
      assert.equal(bytesConsumed, encoded.length);
    });
  }
});
