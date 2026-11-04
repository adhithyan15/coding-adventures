/**
 * Tests for the SHA-256 implementation.
 *
 * Test vectors come from FIPS 180-4 (the official SHA-2 standard). Any correct
 * SHA-256 implementation must produce exactly these digests for these inputs.
 *
 * We test the one-shot sha256/sha256Hex functions, the streaming SHA256Hasher,
 * output format properties, block boundary edge cases, and large inputs.
 */

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { sha256, sha256Hex, toHex, SHA256Hasher, VERSION } from "../src/index.js";

const enc = new TextEncoder();

// ─── FIPS 180-4 Test Vectors ────────────────────────────────────────────────

describe("FIPS 180-4 test vectors", () => {
  it("empty string", () => {
    assert.equal(
      sha256Hex(enc.encode("")),
      "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    );
  });

  it("'abc'", () => {
    assert.equal(
      sha256Hex(enc.encode("abc")),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
  });

  it("448-bit (56-byte) message", () => {
    const msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    assert.equal(msg.length, 56);
    assert.equal(
      sha256Hex(enc.encode(msg)),
      "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
    );
  });

  it("1,000,000 × 'a'", () => {
    const data = enc.encode("a".repeat(1_000_000));
    assert.equal(
      sha256Hex(data),
      "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
    );
  });
});

// ─── Output Format ──────────────────────────────────────────────────────────

describe("output format", () => {
  it("digest is 32 bytes", () => {
    assert.equal(sha256(enc.encode("")).length, 32);
    assert.equal(sha256(enc.encode("hello world")).length, 32);
    assert.equal(sha256(new Uint8Array(1000)).length, 32);
  });

  it("hex string is 64 chars", () => {
    assert.equal(sha256Hex(enc.encode("")).length, 64);
    assert.equal(sha256Hex(enc.encode("hello")).length, 64);
  });

  it("hex string is lowercase", () => {
    const h = sha256Hex(enc.encode("abc"));
    assert.match(h, /^[0-9a-f]{64}$/);
  });

  it("toHex matches sha256Hex", () => {
    const data = enc.encode("test");
    assert.equal(toHex(sha256(data)), sha256Hex(data));
  });

  it("deterministic", () => {
    const d1 = sha256(enc.encode("hello"));
    const d2 = sha256(enc.encode("hello"));
    assert.deepEqual(d1, d2);
  });

  it("exports VERSION", () => {
    assert.equal(VERSION, "0.1.0");
  });
});

// ─── Avalanche Effect ───────────────────────────────────────────────────────

describe("avalanche", () => {
  it("single-character difference flips many bits", () => {
    const h1 = sha256(enc.encode("hello"));
    const h2 = sha256(enc.encode("helo"));
    assert.notDeepEqual(h1, h2);

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
    assert.ok(bits > 40);
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
    assert.equal(sha256(data).length, 32);
  });

  it("56 bytes (padding spills to next block)", () => {
    assert.equal(sha256(new Uint8Array(56)).length, 32);
  });

  it("55 and 56 produce different digests", () => {
    assert.notDeepEqual(sha256(new Uint8Array(55)), sha256(new Uint8Array(56)));
  });

  it("64 bytes (exact one block)", () => {
    assert.equal(sha256(new Uint8Array(64)).length, 32);
  });

  it("128 bytes (exact two blocks)", () => {
    assert.equal(sha256(new Uint8Array(128)).length, 32);
  });

  it("all boundary sizes produce distinct digests", () => {
    const sizes = [55, 56, 63, 64, 127, 128];
    const digests = new Set(sizes.map((n) => sha256Hex(new Uint8Array(n))));
    assert.equal(digests.size, 6);
  });
});

// ─── Edge Cases ─────────────────────────────────────────────────────────────

describe("edge cases", () => {
  it("null byte differs from empty", () => {
    assert.notDeepEqual(sha256(new Uint8Array([0x00])), sha256(new Uint8Array(0)));
  });

  it("all 256 byte values", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    assert.equal(sha256(data).length, 32);
  });

  it("every single byte produces a unique digest", () => {
    const digests = new Set(
      Array.from({ length: 256 }, (_, i) => sha256Hex(new Uint8Array([i])))
    );
    assert.equal(digests.size, 256);
  });
});

// ─── Streaming API ──────────────────────────────────────────────────────────

describe("streaming API (SHA256Hasher)", () => {
  it("single write matches one-shot", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("abc"));
    assert.deepEqual(h.digest(), sha256(enc.encode("abc")));
  });

  it("split at byte boundary", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("ab"));
    h.update(enc.encode("c"));
    assert.deepEqual(h.digest(), sha256(enc.encode("abc")));
  });

  it("split at block boundary (64 bytes)", () => {
    const data = new Uint8Array(128);
    const h = new SHA256Hasher();
    h.update(data.subarray(0, 64));
    h.update(data.subarray(64));
    assert.deepEqual(h.digest(), sha256(data));
  });

  it("byte-at-a-time matches one-shot", () => {
    const data = new Uint8Array(100);
    for (let i = 0; i < 100; i++) data[i] = i;
    const h = new SHA256Hasher();
    for (const b of data) h.update(new Uint8Array([b]));
    assert.deepEqual(h.digest(), sha256(data));
  });

  it("empty hasher matches empty one-shot", () => {
    const h = new SHA256Hasher();
    assert.deepEqual(h.digest(), sha256(new Uint8Array(0)));
  });

  it("digest is non-destructive", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("abc"));
    assert.deepEqual(h.digest(), h.digest());
  });

  it("hexDigest returns correct string", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("abc"));
    assert.equal(
      h.hexDigest(),
      "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
    );
  });

  it("update is chainable", () => {
    const h = new SHA256Hasher()
      .update(enc.encode("a"))
      .update(enc.encode("b"))
      .update(enc.encode("c"));
    assert.equal(h.hexDigest(), sha256Hex(enc.encode("abc")));
  });

  it("copy produces independent hasher", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("ab"));
    const h2 = h.copy();
    h2.update(enc.encode("c"));
    h.update(enc.encode("x"));
    assert.deepEqual(h2.digest(), sha256(enc.encode("abc")));
    assert.deepEqual(h.digest(), sha256(enc.encode("abx")));
  });

  it("streaming 1,000,000 × 'a' in chunks", () => {
    const fullData = enc.encode("a".repeat(1_000_000));
    const h = new SHA256Hasher();
    h.update(fullData.subarray(0, 500_000));
    h.update(fullData.subarray(500_000));
    assert.deepEqual(h.digest(), sha256(fullData));
  });

  it("digest after continue updating still works", () => {
    const h = new SHA256Hasher();
    h.update(enc.encode("hello"));
    const d1 = h.hexDigest();
    h.update(enc.encode(" world"));
    const d2 = h.hexDigest();
    assert.equal(d1, sha256Hex(enc.encode("hello")));
    assert.equal(d2, sha256Hex(enc.encode("hello world")));
  });
});
