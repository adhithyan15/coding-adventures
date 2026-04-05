/**
 * Tests for the SHA-512 implementation.
 *
 * Test vectors come from FIPS 180-4 (the official SHA-2 standard). Any correct
 * SHA-512 implementation must produce exactly these digests for these inputs.
 *
 * We also test the streaming API (SHA512Hasher class) to verify it produces the
 * same results as the one-shot sha512() function, and test edge cases like empty
 * input, exact block boundaries, and very long inputs.
 */

import { describe, it, expect } from "vitest";
import { sha512, sha512Hex, toHex, SHA512Hasher, VERSION } from "../src/index.js";

const enc = new TextEncoder();

// ─── Helpers ────────────────────────────────────────────────────────────────

function text(s: string): Uint8Array {
  return enc.encode(s);
}

function bytes(n: number, fill = 0): Uint8Array {
  return new Uint8Array(n).fill(fill);
}

// ─── Version ────────────────────────────────────────────────────────────────

describe("version", () => {
  it("exports VERSION", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

// ─── FIPS 180-4 Test Vectors ────────────────────────────────────────────────

describe("FIPS 180-4 test vectors", () => {
  it("empty string", () => {
    expect(sha512Hex(text(""))).toBe(
      "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce" +
      "47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e",
    );
  });

  it("'abc'", () => {
    expect(sha512Hex(text("abc"))).toBe(
      "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a" +
      "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
    );
  });

  it("896-bit (112-byte) message", () => {
    // This is the SHA-512 equivalent of the SHA-256 448-bit test vector.
    // 112 bytes = 896 bits, which exactly fills one block minus the length field.
    const msg =
      "abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmn" +
      "hijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
    expect(msg.length).toBe(112);
    expect(sha512Hex(text(msg))).toBe(
      "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018" +
      "501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909",
    );
  });

  it("one million 'a' characters", { timeout: 60_000 }, () => {
    const data = new Uint8Array(1_000_000).fill(0x61); // 'a'
    expect(sha512Hex(data)).toBe(
      "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973eb" +
      "de0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b",
    );
  });
});

// ─── Return Type and Format ─────────────────────────────────────────────────

describe("output format", () => {
  it("returns Uint8Array", () => {
    expect(sha512(text("test"))).toBeInstanceOf(Uint8Array);
  });

  it("digest is exactly 64 bytes", () => {
    expect(sha512(text("")).length).toBe(64);
    expect(sha512(text("hello world")).length).toBe(64);
    expect(sha512(new Uint8Array(1000)).length).toBe(64);
  });

  it("sha512Hex returns 128-char string", () => {
    expect(sha512Hex(text("")).length).toBe(128);
    expect(sha512Hex(text("hello")).length).toBe(128);
  });

  it("sha512Hex is lowercase", () => {
    const hex = sha512Hex(text("abc"));
    expect(hex).toBe(hex.toLowerCase());
    expect(/^[0-9a-f]+$/.test(hex)).toBe(true);
  });

  it("sha512Hex matches toHex(sha512(data))", () => {
    for (const msg of ["", "abc", "hello world"]) {
      expect(sha512Hex(text(msg))).toBe(toHex(sha512(text(msg))));
    }
  });

  it("deterministic -- same input same output", () => {
    expect(sha512(text("hello"))).toEqual(sha512(text("hello")));
  });

  it("avalanche -- one byte change flips many bits", () => {
    const h1 = sha512(text("hello"));
    const h2 = sha512(text("helo"));
    // XOR the two digests; at least 100 of 512 bits should differ
    let diff = 0;
    for (let i = 0; i < 64; i++) {
      const xor = h1[i] ^ h2[i];
      for (let b = 0; b < 8; b++) {
        diff += (xor >> b) & 1;
      }
    }
    expect(diff).toBeGreaterThan(100);
  });
});

// ─── Block Boundary Tests ───────────────────────────────────────────────────
//
// SHA-512 processes 128-byte blocks. Block boundaries are the most common
// source of bugs because padding behaves differently near them:
//
//   111 bytes: fits in one block (111 + 1 + 16 = 128)
//   112 bytes: overflows into a second block (padding spills over)
//   128 bytes: one data block + a full padding block
//   256 bytes: two data blocks + one full padding block

describe("block boundaries", () => {
  it("111 bytes -- exactly one block after padding", () => {
    const r = sha512(bytes(111));
    expect(r.length).toBe(64);
    expect(r).toEqual(sha512(bytes(111))); // deterministic
  });

  it("112 bytes -- requires second block for padding", () => {
    expect(sha512(bytes(112)).length).toBe(64);
  });

  it("111 and 112 bytes produce different digests", () => {
    expect(sha512(bytes(111))).not.toEqual(sha512(bytes(112)));
  });

  it("128 bytes -- one data block + full padding block", () => {
    expect(sha512(bytes(128)).length).toBe(64);
  });

  it("255 bytes", () => {
    expect(sha512(bytes(255)).length).toBe(64);
  });

  it("256 bytes -- two data blocks + full padding block", () => {
    expect(sha512(bytes(256)).length).toBe(64);
  });

  it("all boundary sizes produce distinct digests", () => {
    const sizes = [111, 112, 127, 128, 255, 256];
    const digests = sizes.map((n) => toHex(sha512(bytes(n))));
    const unique = new Set(digests);
    expect(unique.size).toBe(6);
  });
});

// ─── Edge Cases ─────────────────────────────────────────────────────────────

describe("edge cases", () => {
  it("single null byte differs from empty", () => {
    const r = sha512(new Uint8Array([0x00]));
    expect(r.length).toBe(64);
    expect(r).not.toEqual(sha512(new Uint8Array(0)));
  });

  it("single 0xFF byte", () => {
    expect(sha512(new Uint8Array([0xff])).length).toBe(64);
  });

  it("all 256 byte values", () => {
    const data = new Uint8Array(256);
    for (let i = 0; i < 256; i++) data[i] = i;
    expect(sha512(data).length).toBe(64);
  });

  it("every single-byte input produces a unique digest", () => {
    const digests = new Set<string>();
    for (let i = 0; i < 256; i++) {
      digests.add(sha512Hex(new Uint8Array([i])));
    }
    expect(digests.size).toBe(256);
  });

  it("1000 zero bytes", () => {
    expect(sha512(bytes(1000)).length).toBe(64);
  });
});

// ─── toHex helper ───────────────────────────────────────────────────────────

describe("toHex", () => {
  it("converts bytes to lowercase hex", () => {
    const b = new Uint8Array([0x00, 0x0f, 0xff, 0xab]);
    expect(toHex(b)).toBe("000fffab");
  });

  it("empty array -> empty string", () => {
    expect(toHex(new Uint8Array(0))).toBe("");
  });

  it("zero-pads single-digit hex values", () => {
    expect(toHex(new Uint8Array([0x05]))).toBe("05");
  });
});

// ─── Streaming API ──────────────────────────────────────────────────────────

describe("SHA512Hasher streaming", () => {
  it("single update matches oneshot", () => {
    const h = new SHA512Hasher();
    h.update(text("abc"));
    expect(h.hexDigest()).toBe(sha512Hex(text("abc")));
  });

  it("two updates split at byte boundary", () => {
    const h = new SHA512Hasher();
    h.update(text("ab"));
    h.update(text("c"));
    expect(toHex(h.digest())).toBe(sha512Hex(text("abc")));
  });

  it("split at 128-byte block boundary", () => {
    const data = bytes(256);
    const h = new SHA512Hasher();
    h.update(data.subarray(0, 128));
    h.update(data.subarray(128));
    expect(h.digest()).toEqual(sha512(data));
  });

  it("byte-at-a-time", () => {
    const data = new Uint8Array(100);
    for (let i = 0; i < 100; i++) data[i] = i;
    const h = new SHA512Hasher();
    for (const byte of data) {
      h.update(new Uint8Array([byte]));
    }
    expect(h.digest()).toEqual(sha512(data));
  });

  it("empty input", () => {
    const h = new SHA512Hasher();
    expect(h.digest()).toEqual(sha512(text("")));
  });

  it("digest is non-destructive", () => {
    const h = new SHA512Hasher();
    h.update(text("abc"));
    const d1 = h.digest();
    const d2 = h.digest();
    expect(d1).toEqual(d2);
  });

  it("update after digest continues correctly", () => {
    const h = new SHA512Hasher();
    h.update(text("ab"));
    h.digest(); // snapshot -- must not mutate state
    h.update(text("c"));
    expect(h.digest()).toEqual(sha512(text("abc")));
  });

  it("hexDigest returns FIPS vector for 'abc'", () => {
    const h = new SHA512Hasher();
    h.update(text("abc"));
    expect(h.hexDigest()).toBe(
      "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a" +
      "2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
    );
  });

  it("copy is independent", () => {
    const h = new SHA512Hasher();
    h.update(text("ab"));
    const h2 = h.copy();
    h2.update(text("c"));
    h.update(text("x")); // different suffix on original
    expect(h2.digest()).toEqual(sha512(text("abc")));
    expect(h.digest()).toEqual(sha512(text("abx")));
  });

  it("copy produces same digest as original", () => {
    const h = new SHA512Hasher();
    h.update(text("abc"));
    const h2 = h.copy();
    expect(h.digest()).toEqual(h2.digest());
  });

  it("chained update calls work", () => {
    const h = new SHA512Hasher();
    const result = h.update(text("a")).update(text("b")).update(text("c")).hexDigest();
    expect(result).toBe(sha512Hex(text("abc")));
  });

  it("streaming million 'a's matches oneshot", { timeout: 60_000 }, () => {
    const data = new Uint8Array(1_000_000).fill(0x61);
    const h = new SHA512Hasher();
    h.update(data.subarray(0, 500_000));
    h.update(data.subarray(500_000));
    expect(h.digest()).toEqual(sha512(data));
  });
});
