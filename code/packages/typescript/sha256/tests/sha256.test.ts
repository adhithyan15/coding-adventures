/**
 * Tests for the SHA-256 implementation.
 *
 * Test vectors come from FIPS 180-4 (the official SHA-2 standard). Any correct
 * SHA-256 implementation must produce exactly these digests for these inputs.
 *
 * We test the one-shot sha256/sha256Hex functions, the streaming SHA256Hasher,
 * output format properties, block boundary edge cases, and large inputs.
 */

import { describe, it, expect } from "vitest";
import { sha256, sha256Hex, toHex, SHA256Hasher, VERSION } from "../src/index";

const enc = new TextEncoder();

// ─── FIPS 180-4 Test Vectors ────────────────────────────────────────────────

describe("FIPS 180-4 test vectors", () => {
  it("empty string", () => {
    expect(sha256Hex(enc.encode(""))).toBe(
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
  });

  it("'abc'", () => {
    expect(sha256Hex(enc.encode("abc"))).toBe(
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
  });

  it("448-bit (56-byte) message", () => {
    const msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    expect(msg.length).toBe(56);
    expect(sha256Hex(enc.encode(msg))).toBe(
      "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
  });

  it("1,000,000 × 'a'", () => {
    const data = enc.encode("a".repeat(1_000_000));
    expect(sha256Hex(data)).toBe(
      "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
  });
});

// ─── Output Format ──────────────────────────────────────────────────────────

describe("output format", () => {
  it("digest is 32 bytes", () => {
    expect(sha256(enc.encode("")).length).toBe(32);
    expect(sha256(enc.encode("hello world")).length).toBe(32);
    expect(sha256(new Uint8Array(1000)).length).toBe(32);
  });

  it("hex string is 64 chars", () => {
    expect(sha256Hex(enc.encode("")).length).toBe(64);
    expect(sha256Hex(enc.encode("hello")).length).toBe(64);
  });

  it("hex string is lowercase", () => {
    const h = sha256Hex(enc.encode("abc"));
    expect(h).toMatch(/^[0-9a-f]{64}$/);
  });

  it("toHex matches sha256Hex", () => {
    const data = enc.encode("test");
    expect(toHex(sha256(data))).toBe(sha256Hex(data));
  });

  it("deterministic", () => {
    const d1 = sha256(enc.encode("hello"));
    const d2 = sha256(enc.encode("hello"));
    expect(d1).toEqual(d2);
  });

  it("exports VERSION", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ─── Avalanche Effect ───────────────────────────────────────────────────────

describe("avalanche", () => {
  it("single-character difference flips many bits", () => {
    const h1 = sha256(enc.encode("hello"));
    const h2 = sha256(enc.encode("helo"));
    expect(h1).not.toEqual(h2);

    // Count differing bits
    let bits = 0;
    for (let i = 0; i < h1.length; i++) {
      let xor = h1[i] ^ h2[i];
      while (xor) {
        bits += xor & 1;
        xor >>= 1;
      }
    }
    // Expect roughly half of 256 bits to differ (should be > 80 easily)
    expect(bits).toBeGreaterThan(40);
  });
});

// ─── Block Boundaries ───────────────────────────────────────────────────────
//
// SHA-256 uses 64-byte blocks. Padding behavior changes at critical
// boundaries: 55 bytes (padding fits in one block), 56 bytes (needs two
// blocks), 64 bytes (exact block boundary), etc.

describe("block boundaries", () => {
  it("55 bytes (padding fits in same block)", () => {
    const data = new Uint8Array(55);
    expect(sha256(data).length).toBe(32);
  });

  it("56 bytes (padding spills to next block)", () => {
    expect(sha256(new Uint8Array(56)).length).toBe(32);
  });

  it("55 and 56 produce different digests", () => {
    expect(sha256(new Uint8Array(55))).not.toEqual(sha256(new Uint8Array(56)));
  });

  it("64 bytes (exact one block)", () => {
    expect(sha256(new Uint8Array(64)).length).toBe(32);
  });

  it("128 bytes (exact two blocks)", () => {
    expect(sha256(new Uint8Array(128)).length).toBe(32);
  });

  it("all boundary sizes produce distinct digests", () => {
    const sizes = [55, 56, 63, 64, 127, 128];
    const digests = new Set(sizes.map((n) => sha256Hex(new Uint8Array(n))));
    expect(digests.size).toBe(6);
  });
});

// ─── Edge Cases ─────────────────────────────────────────────────────────────

describe("edge cases", () => {
  it("null byte differs from empty", () => {
    expect(sha256(new Uint8Array([0x00]))).not.toEqual(sha256(new Uint8Array(0)));
  });

  it("all 256 byte values", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    expect(sha256(data).length).toBe(32);
  });

  it("every single byte produces a unique digest", () => {
    const digests = new Set(
      Array.from({ length: 256 }, (_, i) => sha256Hex(new Uint8Array([i])))
    );
    expect(digests.size).toBe(256);
  });
});

// ─── Streaming API ──────────────────────────────────────────────────────────

describe("streaming API (SHA256Hasher)", () => {
  it("single write matches one-shot", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("abc"));
    expect(h.digest()).toEqual(sha256(enc.encode("abc")));
  });

  it("split at byte boundary", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("ab"));
    h.update(enc.encode("c"));
    expect(h.digest()).toEqual(sha256(enc.encode("abc")));
  });

  it("split at block boundary (64 bytes)", () => {
    const data = new Uint8Array(128);
    const h = new SHA256Hasher();
    h.update(data.subarray(0, 64));
    h.update(data.subarray(64));
    expect(h.digest()).toEqual(sha256(data));
  });

  it("byte-at-a-time matches one-shot", () => {
    const data = new Uint8Array(100);
    for (let i = 0; i < 100; i++) data[i] = i;
    const h = new SHA256Hasher();
    for (const b of data) h.update(new Uint8Array([b]));
    expect(h.digest()).toEqual(sha256(data));
  });

  it("empty hasher matches empty one-shot", () => {
    const h = new SHA256Hasher();
    expect(h.digest()).toEqual(sha256(new Uint8Array(0)));
  });

  it("digest is non-destructive", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("abc"));
    expect(h.digest()).toEqual(h.digest());
  });

  it("hexDigest returns correct string", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("abc"));
    expect(h.hexDigest()).toBe(
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
  });

  it("update is chainable", () => {
    const h = new SHA256Hasher()
      .update(enc.encode("a"))
      .update(enc.encode("b"))
      .update(enc.encode("c"));
    expect(h.hexDigest()).toBe(sha256Hex(enc.encode("abc")));
  });

  it("copy produces independent hasher", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("ab"));
    const h2 = h.copy();
    h2.update(enc.encode("c"));
    h.update(enc.encode("x"));
    expect(h2.digest()).toEqual(sha256(enc.encode("abc")));
    expect(h.digest()).toEqual(sha256(enc.encode("abx")));
  });

  it("streaming 1,000,000 × 'a' in chunks", () => {
    const fullData = enc.encode("a".repeat(1_000_000));
    const h = new SHA256Hasher();
    h.update(fullData.subarray(0, 500_000));
    h.update(fullData.subarray(500_000));
    expect(h.digest()).toEqual(sha256(fullData));
  });

  it("digest after continue updating still works", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("hello"));
    const d1 = h.hexDigest();
    h.update(enc.encode(" world"));
    const d2 = h.hexDigest();
    expect(d1).toBe(sha256Hex(enc.encode("hello")));
    expect(d2).toBe(sha256Hex(enc.encode("hello world")));
  });
});
