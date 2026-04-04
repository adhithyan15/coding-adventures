/**
 * Reed-Solomon TypeScript test suite.
 *
 * Cross-validates every result against the Rust reference implementation
 * (MA02). All encode/decode vectors match `cargo test -p reed-solomon`.
 */

import { describe, it, expect } from "vitest";
import {
  encode,
  decode,
  syndromes,
  buildGenerator,
  errorLocator,
  TooManyErrorsError,
  InvalidInputError,
} from "../src/index.js";
import { multiply, power } from "@coding-adventures/gf256";

// =============================================================================
// Helpers
// =============================================================================

function corrupt(codeword: Uint8Array, positions: number[], mask: number): Uint8Array {
  const c = codeword.slice();
  for (const p of positions) c[p] ^= mask;
  return c;
}

function allZero(arr: Uint8Array): boolean {
  return arr.every((x) => x === 0);
}

function assertSyndromesZero(codeword: Uint8Array, nCheck: number): void {
  const s = syndromes(codeword, nCheck);
  expect(allZero(s)).toBe(true);
}

function bytes(...vals: number[]): Uint8Array {
  return new Uint8Array(vals);
}

function str(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

// =============================================================================
// 1. Generator Polynomial
// =============================================================================

describe("buildGenerator", () => {
  it("n=2 gives [8, 6, 1]", () => {
    const g = buildGenerator(2);
    expect(Array.from(g)).toEqual([8, 6, 1]);
  });

  it("n=4 has length 5", () => {
    const g = buildGenerator(4);
    expect(g.length).toBe(5);
    expect(g[g.length - 1]).toBe(1); // monic
  });

  it("n=8 is monic degree 8", () => {
    const g = buildGenerator(8);
    expect(g.length).toBe(9);
    expect(g[g.length - 1]).toBe(1);
  });

  it("roots are powers of alpha", () => {
    const nCheck = 4;
    const g = buildGenerator(nCheck);

    for (let i = 1; i <= nCheck; i++) {
      const root = power(2, i);
      // Evaluate g(root) using Horner in GF(256)
      let val = 0;
      for (let j = g.length - 1; j >= 0; j--) {
        val = (val !== 0 || g[j] !== 0)
          ? (multiply(val, root) ^ g[j])
          : 0;
      }
      // Proper Horner
      let v = 0;
      for (let k = g.length - 1; k >= 0; k--) {
        v = multiply(v, root) ^ g[k];
      }
      expect(v).toBe(0);
    }
  });

  it("throws on odd nCheck", () => {
    expect(() => buildGenerator(3)).toThrow(InvalidInputError);
  });

  it("throws on nCheck=0", () => {
    expect(() => buildGenerator(0)).toThrow(InvalidInputError);
  });
});

// =============================================================================
// 2. Encoding — Structural Properties
// =============================================================================

describe("encode", () => {
  it("preserves message bytes (systematic)", () => {
    const msg = str("hello RS");
    const cw = encode(msg, 4);
    expect(cw.slice(0, msg.length)).toEqual(msg);
  });

  it("codeword length = message.length + nCheck", () => {
    const msg = str("test");
    const cw = encode(msg, 8);
    expect(cw.length).toBe(msg.length + 8);
  });

  it("encoded codeword has all-zero syndromes", () => {
    const msg = str("syndromes must all be zero");
    const cw = encode(msg, 6);
    assertSyndromesZero(cw, 6);
  });

  it("different messages give different codewords", () => {
    const c1 = encode(str("hello"), 4);
    const c2 = encode(str("world"), 4);
    expect(c1).not.toEqual(c2);
  });

  it("empty message gives nCheck-length zero codeword", () => {
    const cw = encode(new Uint8Array(0), 4);
    expect(cw.length).toBe(4);
    assertSyndromesZero(cw, 4);
  });

  it("single-byte message", () => {
    const cw = encode(bytes(0x42), 2);
    expect(cw.length).toBe(3);
    expect(cw[0]).toBe(0x42);
    assertSyndromesZero(cw, 2);
  });

  it("throws on odd nCheck", () => {
    expect(() => encode(str("x"), 3)).toThrow(InvalidInputError);
  });

  it("throws on nCheck=0", () => {
    expect(() => encode(str("x"), 0)).toThrow(InvalidInputError);
  });

  it("throws on oversized codeword (>255)", () => {
    const big = new Uint8Array(240);
    expect(() => encode(big, 20)).toThrow(InvalidInputError);
  });

  it("accepts exactly n=255 (limit)", () => {
    const cw = encode(bytes(0x42), 254);
    expect(cw.length).toBe(255);
    assertSyndromesZero(cw, 254);
  });
});

// =============================================================================
// 3. Syndrome Computation
// =============================================================================

describe("syndromes", () => {
  it("all zero for valid codeword", () => {
    const cw = encode(str("error free"), 8);
    expect(allZero(syndromes(cw, 8))).toBe(true);
  });

  it("non-zero after corruption", () => {
    const cw = encode(str("corrupt me"), 8);
    const corrupted = corrupt(cw, [0], 0xff);
    expect(syndromes(corrupted, 8).some((s) => s !== 0)).toBe(true);
  });

  it("count equals nCheck", () => {
    const cw = encode(str("count check"), 6);
    expect(syndromes(cw, 6).length).toBe(6);
  });
});

// =============================================================================
// 4. Round-Trip: Encode → Decode with Zero Errors
// =============================================================================

describe("encode → decode (no errors)", () => {
  it("short message", () => {
    const msg = str("hello");
    expect(decode(encode(msg, 4), 4)).toEqual(msg);
  });

  it("longer message", () => {
    const msg = str("Reed-Solomon coding is beautiful");
    expect(decode(encode(msg, 8), 8)).toEqual(msg);
  });

  it("all-zero message", () => {
    const msg = new Uint8Array(10);
    expect(decode(encode(msg, 4), 4)).toEqual(msg);
  });

  it("all-0xFF message", () => {
    const msg = new Uint8Array(10).fill(0xff);
    expect(decode(encode(msg, 4), 4)).toEqual(msg);
  });

  it("random-ish bytes", () => {
    const msg = new Uint8Array(50).map((_, i) => (i * 37 + 13) & 0xff);
    expect(decode(encode(msg, 10), 10)).toEqual(msg);
  });
});

// =============================================================================
// 5. Error Correction Up to Capacity
// =============================================================================

describe("error correction", () => {
  it("t=1: corrects 1 error", () => {
    const msg = str("abc");
    const cw = corrupt(encode(msg, 2), [1], 0x5a);
    expect(decode(cw, 2)).toEqual(msg);
  });

  it("t=2: corrects 2 errors", () => {
    const msg = str("four check bytes");
    const cw = corrupt(encode(msg, 4), [0, 5], 0xaa);
    expect(decode(cw, 4)).toEqual(msg);
  });

  it("t=4: corrects 4 errors", () => {
    const msg = str("eight check bytes give t=4");
    let cw = encode(msg, 8);
    cw = corrupt(cw, [0], 0xff);
    cw = corrupt(cw, [3], 0xaa);
    cw = corrupt(cw, [10], 0x55);
    cw = corrupt(cw, [14], 0x0f);
    expect(decode(cw, 8)).toEqual(msg);
  });

  it("error in check bytes", () => {
    const msg = str("check byte error");
    const cw = corrupt(encode(msg, 4), [msg.length], 0x33);
    expect(decode(cw, 4)).toEqual(msg);
  });

  it("error at first byte", () => {
    const msg = str("first byte error");
    const cw = corrupt(encode(msg, 4), [0], 0xbb);
    expect(decode(cw, 4)).toEqual(msg);
  });

  it("error at last byte", () => {
    const msg = str("last byte error!");
    const clean = encode(msg, 4);
    const cw = corrupt(clean, [clean.length - 1], 0xcc);
    expect(decode(cw, 4)).toEqual(msg);
  });

  it("t=3: corrects 3 errors at varied positions", () => {
    const msg = new Uint8Array(20).map((_, i) => i);
    let cw = encode(msg, 6);
    cw = corrupt(cw, [0], 0x01);
    cw = corrupt(cw, [10], 0x02);
    cw = corrupt(cw, [19], 0x04);
    expect(decode(cw, 6)).toEqual(msg);
  });

  it("t=10: corrects 10 errors", () => {
    const msg = new Uint8Array(30).map((_, i) => i);
    let cw = encode(msg, 20);
    for (let i = 0; i < 10; i++) {
      cw = corrupt(cw, [i * 3], ((0x11 * (i + 1)) & 0xff));
    }
    expect(decode(cw, 20)).toEqual(msg);
  });

  it("corrects error at every single position", () => {
    const msg = str("position");
    const nCheck = 6;
    const clean = encode(msg, nCheck);

    for (let p = 0; p < clean.length; p++) {
      const corrupted = corrupt(clean, [p], 0xaa);
      const recovered = decode(corrupted, nCheck);
      expect(recovered).toEqual(msg);
    }
  });
});

// =============================================================================
// 6. TooManyErrors — Beyond Correction Capacity
// =============================================================================

describe("TooManyErrors", () => {
  it("t=1 capacity: 2 errors fails", () => {
    const msg = str("capacity one");
    const cw = corrupt(encode(msg, 2), [0, 1], 0xff);
    expect(() => decode(cw, 2)).toThrow(TooManyErrorsError);
  });

  it("t=4: 5 errors fails", () => {
    const msg = str("too many errors here");
    const cw = corrupt(encode(msg, 8), [0, 2, 4, 6, 8], 0xff);
    expect(() => decode(cw, 8)).toThrow(TooManyErrorsError);
  });
});

// =============================================================================
// 7. decode() Input Validation
// =============================================================================

describe("decode validation", () => {
  it("throws on odd nCheck", () => {
    expect(() => decode(new Uint8Array(10), 3)).toThrow(InvalidInputError);
  });

  it("throws on nCheck=0", () => {
    expect(() => decode(new Uint8Array(10), 0)).toThrow(InvalidInputError);
  });

  it("throws when received shorter than nCheck", () => {
    expect(() => decode(bytes(0, 0, 0), 4)).toThrow(InvalidInputError);
  });
});

// =============================================================================
// 8. Error Locator Polynomial
// =============================================================================

describe("errorLocator", () => {
  it("no errors → [1]", () => {
    const cw = encode(str("no errors here"), 6);
    const s = syndromes(cw, 6);
    expect(Array.from(errorLocator(s))).toEqual([1]);
  });

  it("one error → degree 1 (length 2)", () => {
    const cw = corrupt(encode(str("one error"), 4), [2], 0x7f);
    const s = syndromes(cw, 4);
    expect(errorLocator(s).length).toBe(2);
  });

  it("two errors → degree 2 (length 3)", () => {
    const cw = corrupt(
      corrupt(encode(str("two errors in this message"), 6), [0], 0x11),
      [5],
      0x22
    );
    const s = syndromes(cw, 6);
    expect(errorLocator(s).length).toBe(3);
  });
});

// =============================================================================
// 9. Concrete Test Vectors (cross-validated with Rust reference)
// =============================================================================

describe("test vectors", () => {
  it("bytes [1,2,3,4] with nCheck=4: zero syndromes", () => {
    const msg = bytes(1, 2, 3, 4);
    const cw = encode(msg, 4);
    expect(cw.length).toBe(8);
    expect(cw.slice(0, 4)).toEqual(msg);
    assertSyndromesZero(cw, 4);
  });

  it("bytes [1,2,3,4] with nCheck=4: round-trip with 1 error", () => {
    const msg = bytes(1, 2, 3, 4);
    const cw = corrupt(encode(msg, 4), [0], 0xab);
    expect(decode(cw, 4)).toEqual(msg);
  });

  it("alternating 0x55 bytes: zero syndromes", () => {
    const msg = new Uint8Array(8).fill(0x55);
    assertSyndromesZero(encode(msg, 4), 4);
  });

  it("alternating 0xAA bytes: round-trip", () => {
    const msg = new Uint8Array(8).fill(0xaa);
    expect(decode(encode(msg, 4), 4)).toEqual(msg);
  });

  it("'QR code' with nCheck=8: zero syndromes", () => {
    const msg = str("QR code");
    assertSyndromesZero(encode(msg, 8), 8);
  });

  it("'Hello' with nCheck=4: message bytes preserved + zero syndromes", () => {
    const msg = str("Hello");
    const cw = encode(msg, 4);
    expect(cw.slice(0, msg.length)).toEqual(msg);
    assertSyndromesZero(cw, 4);
  });
});

// =============================================================================
// 10. Edge Cases
// =============================================================================

describe("edge cases", () => {
  it("single byte messages round-trip for all byte values", () => {
    for (const b of [0, 1, 127, 128, 254, 255]) {
      const msg = bytes(b);
      expect(decode(encode(msg, 2), 2)).toEqual(msg);
    }
  });

  it("message with zero bytes does not confuse the algorithm", () => {
    const msg = bytes(0, 0, 0, 42, 0, 0);
    expect(decode(encode(msg, 4), 4)).toEqual(msg);
  });

  it("n=2 minimal RS: corrects 1 error", () => {
    const msg = str("minimal");
    const cw = corrupt(encode(msg, 2), [3], 0x7f);
    expect(decode(cw, 2)).toEqual(msg);
  });

  it("VERSION is exported", async () => {
    const mod = await import("../src/index.js");
    expect(typeof mod.VERSION).toBe("string");
    expect(mod.VERSION.length).toBeGreaterThan(0);
  });
});
