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

import assert from "node:assert/strict";
import { describe, test as it } from "node:test";
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
    assert.equal(VERSION, "0.1.0");
  });
});

// ─── FIPS 180-4 Test Vectors ─────────────────────────────────────────────────

describe("FIPS 180-4 test vectors", () => {
  it("empty string", () => {
    assert.equal(
      sha1Hex(text("")),
      "da39a3ee5e6b4b0d3255bfef95601890afd80709",
    );
  });

  it("'abc'", () => {
    assert.equal(
      sha1Hex(text("abc")),
      "a9993e364706816aba3e25717850c26c9cd0d89d",
    );
  });

  it("448-bit (56-byte) message", () => {
    const msg = "abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    assert.equal(msg.length, 56);
    assert.equal(
      sha1Hex(text(msg)),
      "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
    );
  });

  it("one million 'a' characters", () => {
    const data = new Uint8Array(1_000_000).fill(0x61); // 'a'
    assert.equal(sha1Hex(data), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
  });
});

// ─── Return Type and Format ───────────────────────────────────────────────────

describe("output format", () => {
  it("returns Uint8Array", () => {
    assert.ok(sha1(text("test")) instanceof Uint8Array);
  });

  it("digest is exactly 20 bytes", () => {
    assert.equal(sha1(text("")).length, 20);
    assert.equal(sha1(text("hello world")).length, 20);
    assert.equal(sha1(new Uint8Array(1000)).length, 20);
  });

  it("sha1Hex returns 40-char string", () => {
    assert.equal(sha1Hex(text("")).length, 40);
    assert.equal(sha1Hex(text("hello")).length, 40);
  });

  it("sha1Hex is lowercase", () => {
    const hex = sha1Hex(text("abc"));
    assert.equal(hex, hex.toLowerCase());
    assert.ok(/^[0-9a-f]+$/.test(hex));
  });

  it("sha1Hex matches toHex(sha1(data))", () => {
    for (const msg of ["", "abc", "hello world"]) {
      assert.equal(sha1Hex(text(msg)), toHex(sha1(text(msg))));
    }
  });

  it("deterministic — same input same output", () => {
    assert.deepEqual(sha1(text("hello")), sha1(text("hello")));
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
    assert.ok(diff > 20);
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
    assert.equal(r.length, 20);
    assert.deepEqual(r, sha1(bytes(55))); // deterministic
  });

  it("56 bytes — requires second block for padding", () => {
    assert.equal(sha1(bytes(56)).length, 20);
  });

  it("55 and 56 bytes produce different digests", () => {
    assert.notDeepEqual(sha1(bytes(55)), sha1(bytes(56)));
  });

  it("64 bytes — one data block + full padding block", () => {
    assert.equal(sha1(bytes(64)).length, 20);
  });

  it("127 bytes", () => {
    assert.equal(sha1(bytes(127)).length, 20);
  });

  it("128 bytes — two data blocks + full padding block", () => {
    assert.equal(sha1(bytes(128)).length, 20);
  });

  it("all boundary sizes produce distinct digests", () => {
    const sizes = [55, 56, 63, 64, 127, 128];
    const digests = sizes.map((n) => toHex(sha1(bytes(n))));
    const unique = new Set(digests);
    assert.equal(unique.size, 6);
  });
});

// ─── Edge Cases ───────────────────────────────────────────────────────────────

describe("edge cases", () => {
  it("single null byte differs from empty", () => {
    const r = sha1(new Uint8Array([0x00]));
    assert.equal(r.length, 20);
    assert.notDeepEqual(r, sha1(new Uint8Array(0)));
  });

  it("single 0xFF byte", () => {
    assert.equal(sha1(new Uint8Array([0xff])).length, 20);
  });

  it("all 256 byte values", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    assert.equal(sha1(data).length, 20);
  });

  it("every single-byte input produces a unique digest", () => {
    const digests = new Set<string>();
    for (let i = 0; i < 256; i++) {
      digests.add(sha1Hex(new Uint8Array([i])));
    }
    assert.equal(digests.size, 256);
  });

  it("1000 zero bytes", () => {
    assert.equal(sha1(bytes(1000)).length, 20);
  });
});

// ─── toHex helper ─────────────────────────────────────────────────────────────

describe("toHex", () => {
  it("converts bytes to lowercase hex", () => {
    const b = new Uint8Array([0x00, 0x0f, 0xff, 0xab]);
    assert.equal(toHex(b), "000fffab");
  });

  it("empty array → empty string", () => {
    assert.equal(toHex(new Uint8Array(0)), "");
  });

  it("zero-pads single-digit hex values", () => {
    assert.equal(toHex(new Uint8Array([0x05])), "05");
  });
});

// ─── Streaming API ────────────────────────────────────────────────────────────

describe("SHA1Hasher streaming", () => {
  it("single update matches oneshot", () => {
    const h = new SHA1Hasher();
    h.update(text("abc"));
    assert.equal(h.hexDigest(), sha1Hex(text("abc")));
  });

  it("two updates split at byte boundary", () => {
    const h = new SHA1Hasher();
    h.update(text("ab"));
    h.update(text("c"));
    assert.equal(toHex(h.digest()), sha1Hex(text("abc")));
  });

  it("split at 64-byte block boundary", () => {
    const data = bytes(128);
    const h = new SHA1Hasher();
    h.update(data.subarray(0, 64));
    h.update(data.subarray(64));
    assert.deepEqual(h.digest(), sha1(data));
  });

  it("byte-at-a-time", () => {
    const data = new Uint8Array(100);
    for (let i = 0; i < 100; i++) data[i] = i;
    const h = new SHA1Hasher();
    for (const byte of data) {
      h.update(new Uint8Array([byte]));
    }
    assert.deepEqual(h.digest(), sha1(data));
  });

  it("empty input", () => {
    const h = new SHA1Hasher();
    assert.deepEqual(h.digest(), sha1(text("")));
  });

  it("digest is non-destructive", () => {
    const h = new SHA1Hasher();
    h.update(text("abc"));
    const d1 = h.digest();
    const d2 = h.digest();
    assert.deepEqual(d1, d2);
  });

  it("update after digest continues correctly", () => {
    const h = new SHA1Hasher();
    h.update(text("ab"));
    h.digest(); // snapshot — must not mutate state
    h.update(text("c"));
    assert.deepEqual(h.digest(), sha1(text("abc")));
  });

  it("hexDigest returns FIPS vector for 'abc'", () => {
    const h = new SHA1Hasher();
    h.update(text("abc"));
    assert.equal(h.hexDigest(), "a9993e364706816aba3e25717850c26c9cd0d89d");
  });

  it("copy is independent", () => {
    const h = new SHA1Hasher();
    h.update(text("ab"));
    const h2 = h.copy();
    h2.update(text("c"));
    h.update(text("x")); // different suffix on original
    assert.deepEqual(h2.digest(), sha1(text("abc")));
    assert.deepEqual(h.digest(), sha1(text("abx")));
  });

  it("copy produces same digest as original", () => {
    const h = new SHA1Hasher();
    h.update(text("abc"));
    const h2 = h.copy();
    assert.deepEqual(h.digest(), h2.digest());
  });

  it("chained update calls work", () => {
    const h = new SHA1Hasher();
    const result = h.update(text("a")).update(text("b")).update(text("c")).hexDigest();
    assert.equal(result, sha1Hex(text("abc")));
  });

  it("streaming million 'a's matches oneshot", () => {
    const data = new Uint8Array(1_000_000).fill(0x61);
    const h = new SHA1Hasher();
    h.update(data.subarray(0, 500_000));
    h.update(data.subarray(500_000));
    assert.deepEqual(h.digest(), sha1(data));
  });
});
