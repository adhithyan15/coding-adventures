/**
 * Tests for the SHA-1 implementation.
 *
 * Test vectors come from FIPS 180-4 (the official SHA-1 standard). Any correct
 * SHA-1 implementation must produce exactly these digests for these inputs.
 *
 * We also test the streaming API (SHA1Hasher class) to verify it produces the
 * same results as the one-shot sha1() function, and test edge cases like empty
 * input, exact block boundaries, and very long inputs.
 */

import { describe, it, expect } from "vitest";
import { sha1, sha1Hex, toHex, SHA1Hasher, VERSION } from "../src/index.js";

const enc = new TextEncoder();

// ─── Helpers ─────────────────────────────────────────────────────────────────

function text(s: string): Uint8Array {
  return enc.encode(s);
}

function bytes(n: number, fill = 0): Uint8Array {
  return new Uint8Array(n).fill(fill);
}

// ─── Version ─────────────────────────────────────────────────────────────────

describe("version", () => {
  it("exports VERSION", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ─── FIPS 180-4 Test Vectors ─────────────────────────────────────────────────

describe("FIPS 180-4 test vectors", () => {
  it("empty string", () => {
    expect(sha1Hex(text(""))).toBe(
      "da39a3ee5e6b4b0d3255bfef95601890afd80709",
    );
  });

  it("'abc'", () => {
    expect(sha1Hex(text("abc"))).toBe(
      "a9993e364706816aba3e25717850c26c9cd0d89d",
    );
  });

  it("448-bit (56-byte) message", () => {
    const msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    expect(msg.length).toBe(56);
    expect(sha1Hex(text(msg))).toBe(
      "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
    );
  });

  it("one million 'a' characters", () => {
    const data = new Uint8Array(1_000_000).fill(0x61); // 'a'
    expect(sha1Hex(data)).toBe("34aa973cd4c4daa4f61eeb2bdbad27316534016f");
  });
});

// ─── Return Type and Format ───────────────────────────────────────────────────

describe("output format", () => {
  it("returns Uint8Array", () => {
    expect(sha1(text("test"))).toBeInstanceOf(Uint8Array);
  });

  it("digest is exactly 20 bytes", () => {
    expect(sha1(text("")).length).toBe(20);
    expect(sha1(text("hello world")).length).toBe(20);
    expect(sha1(new Uint8Array(1000)).length).toBe(20);
  });

  it("sha1Hex returns 40-char string", () => {
    expect(sha1Hex(text("")).length).toBe(40);
    expect(sha1Hex(text("hello")).length).toBe(40);
  });

  it("sha1Hex is lowercase", () => {
    const hex = sha1Hex(text("abc"));
    expect(hex).toBe(hex.toLowerCase());
    expect(/^[0-9a-f]+$/.test(hex)).toBe(true);
  });

  it("sha1Hex matches toHex(sha1(data))", () => {
    for (const msg of ["", "abc", "hello world"]) {
      expect(sha1Hex(text(msg))).toBe(toHex(sha1(text(msg))));
    }
  });

  it("deterministic — same input same output", () => {
    expect(sha1(text("hello"))).toEqual(sha1(text("hello")));
  });

  it("avalanche — one byte change flips many bits", () => {
    const h1 = sha1(text("hello"));
    const h2 = sha1(text("helo"));
    // XOR the two digests; at least 20 of 160 bits should differ
    let diff = 0;
    for (let i = 0; i < 20; i++) {
      const xor = h1[i] ^ h2[i];
      for (let b = 0; b < 8; b++) {
        diff += (xor >> b) & 1;
      }
    }
    expect(diff).toBeGreaterThan(20);
  });
});

// ─── Block Boundary Tests ─────────────────────────────────────────────────────
//
// SHA-1 processes 64-byte blocks. Block boundaries are the most common source
// of bugs because padding behaves differently near them:
//
//   55 bytes: fits in one block (55 + 1 + 8 = 64)
//   56 bytes: overflows into a second block (padding spills over)
//   64 bytes: one data block + a full padding block
//   128 bytes: two data blocks + one full padding block

describe("block boundaries", () => {
  it("55 bytes — exactly one block after padding", () => {
    const r = sha1(bytes(55));
    expect(r.length).toBe(20);
    expect(r).toEqual(sha1(bytes(55))); // deterministic
  });

  it("56 bytes — requires second block for padding", () => {
    expect(sha1(bytes(56)).length).toBe(20);
  });

  it("55 and 56 bytes produce different digests", () => {
    expect(sha1(bytes(55))).not.toEqual(sha1(bytes(56)));
  });

  it("64 bytes — one data block + full padding block", () => {
    expect(sha1(bytes(64)).length).toBe(20);
  });

  it("127 bytes", () => {
    expect(sha1(bytes(127)).length).toBe(20);
  });

  it("128 bytes — two data blocks + full padding block", () => {
    expect(sha1(bytes(128)).length).toBe(20);
  });

  it("all boundary sizes produce distinct digests", () => {
    const sizes = [55, 56, 63, 64, 127, 128];
    const digests = sizes.map((n) => toHex(sha1(bytes(n))));
    const unique = new Set(digests);
    expect(unique.size).toBe(6);
  });
});

// ─── Edge Cases ───────────────────────────────────────────────────────────────

describe("edge cases", () => {
  it("single null byte differs from empty", () => {
    const r = sha1(new Uint8Array([0x00]));
    expect(r.length).toBe(20);
    expect(r).not.toEqual(sha1(new Uint8Array(0)));
  });

  it("single 0xFF byte", () => {
    expect(sha1(new Uint8Array([0xff])).length).toBe(20);
  });

  it("all 256 byte values", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    expect(sha1(data).length).toBe(20);
  });

  it("every single-byte input produces a unique digest", () => {
    const digests = new Set<string>();
    for (let i = 0; i < 256; i++) {
      digests.add(sha1Hex(new Uint8Array([i])));
    }
    expect(digests.size).toBe(256);
  });

  it("1000 zero bytes", () => {
    expect(sha1(bytes(1000)).length).toBe(20);
  });
});

// ─── toHex helper ─────────────────────────────────────────────────────────────

describe("toHex", () => {
  it("converts bytes to lowercase hex", () => {
    const b = new Uint8Array([0x00, 0x0f, 0xff, 0xab]);
    expect(toHex(b)).toBe("000fffab");
  });

  it("empty array → empty string", () => {
    expect(toHex(new Uint8Array(0))).toBe("");
  });

  it("zero-pads single-digit hex values", () => {
    expect(toHex(new Uint8Array([0x05]))).toBe("05");
  });
});

// ─── Streaming API ────────────────────────────────────────────────────────────

describe("SHA1Hasher streaming", () => {
  it("single update matches oneshot", () => {
    const h = new SHA1Hasher();
    h.update(text("abc"));
    expect(h.hexDigest()).toBe(sha1Hex(text("abc")));
  });

  it("two updates split at byte boundary", () => {
    const h = new SHA1Hasher();
    h.update(text("ab"));
    h.update(text("c"));
    expect(toHex(h.digest())).toBe(sha1Hex(text("abc")));
  });

  it("split at 64-byte block boundary", () => {
    const data = bytes(128);
    const h = new SHA1Hasher();
    h.update(data.subarray(0, 64));
    h.update(data.subarray(64));
    expect(h.digest()).toEqual(sha1(data));
  });

  it("byte-at-a-time", () => {
    const data = new Uint8Array(100);
    for (let i = 0; i < 100; i++) data[i] = i;
    const h = new SHA1Hasher();
    for (const byte of data) {
      h.update(new Uint8Array([byte]));
    }
    expect(h.digest()).toEqual(sha1(data));
  });

  it("empty input", () => {
    const h = new SHA1Hasher();
    expect(h.digest()).toEqual(sha1(text("")));
  });

  it("digest is non-destructive", () => {
    const h = new SHA1Hasher();
    h.update(text("abc"));
    const d1 = h.digest();
    const d2 = h.digest();
    expect(d1).toEqual(d2);
  });

  it("update after digest continues correctly", () => {
    const h = new SHA1Hasher();
    h.update(text("ab"));
    h.digest(); // snapshot — must not mutate state
    h.update(text("c"));
    expect(h.digest()).toEqual(sha1(text("abc")));
  });

  it("hexDigest returns FIPS vector for 'abc'", () => {
    const h = new SHA1Hasher();
    h.update(text("abc"));
    expect(h.hexDigest()).toBe("a9993e364706816aba3e25717850c26c9cd0d89d");
  });

  it("copy is independent", () => {
    const h = new SHA1Hasher();
    h.update(text("ab"));
    const h2 = h.copy();
    h2.update(text("c"));
    h.update(text("x")); // different suffix on original
    expect(h2.digest()).toEqual(sha1(text("abc")));
    expect(h.digest()).toEqual(sha1(text("abx")));
  });

  it("copy produces same digest as original", () => {
    const h = new SHA1Hasher();
    h.update(text("abc"));
    const h2 = h.copy();
    expect(h.digest()).toEqual(h2.digest());
  });

  it("chained update calls work", () => {
    const h = new SHA1Hasher();
    const result = h.update(text("a")).update(text("b")).update(text("c")).hexDigest();
    expect(result).toBe(sha1Hex(text("abc")));
  });

  it("streaming million 'a's matches oneshot", () => {
    const data = new Uint8Array(1_000_000).fill(0x61);
    const h = new SHA1Hasher();
    h.update(data.subarray(0, 500_000));
    h.update(data.subarray(500_000));
    expect(h.digest()).toEqual(sha1(data));
  });
});
