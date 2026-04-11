/**
 * scrypt test suite — RFC 7914 vectors plus property-based checks.
 *
 * Test Strategy
 * =============
 * 1. RFC 7914 §12 Test Vectors — authoritative expected outputs from the spec.
 *    These are our primary correctness guarantee. If these fail, the algorithm
 *    is wrong regardless of what other tests say.
 *
 * 2. scryptHex — verifies the hex convenience wrapper produces the same result
 *    as scrypt() with manual hex encoding.
 *
 * 3. Output length — dkLen parameter is respected exactly.
 *
 * 4. Determinism — same inputs always produce the same output.
 *
 * 5. Sensitivity — different passwords, salts, N, r, or p produce different outputs.
 *    (Confirms that each parameter is actually incorporated.)
 *
 * 6. Error cases — invalid parameters throw descriptive errors.
 */

import { describe, it, expect } from "vitest";
import { scrypt, scryptHex } from "../src/index.js";

// Helper: UTF-8 encode a string to Uint8Array.
function enc(s: string): Uint8Array {
  return new TextEncoder().encode(s);
}

// Helper: decode a hex string to Uint8Array.
function fromHex(hex: string): Uint8Array {
  return new Uint8Array(hex.match(/.{2}/g)!.map((b) => parseInt(b, 16)));
}

// ─── RFC 7914 §12 Test Vectors ────────────────────────────────────────────────
//
// These are the authoritative test vectors from the scrypt specification.
// All implementations MUST produce exactly these outputs.

describe("RFC 7914 scrypt test vectors", () => {
  it("vector 1 — empty password and salt, N=16, r=1, p=1, dkLen=64", () => {
    // RFC 7914 §12, vector 1:
    //   password = ""
    //   salt     = ""
    //   N = 16, r = 1, p = 1
    //   dkLen = 64
    //
    // This vector tests the edge case of empty inputs. The @coding-adventures/pbkdf2
    // package rejects empty passwords, which is why scrypt uses its own internal
    // PBKDF2 that bypasses that guard.
    const expected = fromHex(
      "77d6576238657b203b19ca42c18a0497" +
      "f16b4844e3074ae8dfdffa3fede21442" +
      "fcd0069ded0948f8326a753a0fc81f17" +
      "e8d3e0fb2e0d3628cf35e20c38d18906",
    );
    const result = scrypt(new Uint8Array(0), new Uint8Array(0), 16, 1, 1, 64);
    expect(Array.from(result)).toEqual(Array.from(expected));
  });

  it("vector 2 — password=password, salt=NaCl, N=1024, r=8, p=16, dkLen=64", () => {
    // RFC 7914 §12, vector 2:
    //   password = "password"
    //   salt     = "NaCl"
    //   N = 1024, r = 8, p = 16
    //   dkLen = 64
    //
    // This vector exercises realistic parameters. N=1024, r=8 means each ROMix
    // call uses 1024 × 128 × 8 = 1,048,576 bytes (1 MiB) of scratch-pad memory.
    // p=16 means 16 independent ROMix calls, so peak memory ≈ 16 MiB
    // (though in this sequential implementation they run one at a time).
    //
    // Note: this test takes a few seconds to run because of the large N and p.
    const expected = fromHex(
      "fdbabe1c9d3472007856e7190d01e9fe" +
      "7c6ad7cbc8237830e77376634b373162" +
      "2eaf30d92e22a3886ff109279d9830da" +
      "c727afb94a83ee6d8360cbdfa2cc0640",
    );
    const result = scrypt(enc("password"), enc("NaCl"), 1024, 8, 16, 64);
    expect(Array.from(result)).toEqual(Array.from(expected));
  }, 30000); // 30 second timeout for the heavy vector
});

// ─── scryptHex ────────────────────────────────────────────────────────────────

describe("scryptHex", () => {
  it("returns lowercase hex string matching scrypt bytes", () => {
    // scryptHex should produce the same result as manually encoding scrypt().
    const bytes = scrypt(new Uint8Array(0), new Uint8Array(0), 16, 1, 1, 64);
    const hex = scryptHex(new Uint8Array(0), new Uint8Array(0), 16, 1, 1, 64);
    const expected = Array.from(bytes)
      .map((b) => b.toString(16).padStart(2, "0"))
      .join("");
    expect(hex).toBe(expected);
  });

  it("matches RFC 7914 vector 1 as a hex string", () => {
    const expected =
      "77d6576238657b203b19ca42c18a0497" +
      "f16b4844e3074ae8dfdffa3fede21442" +
      "fcd0069ded0948f8326a753a0fc81f17" +
      "e8d3e0fb2e0d3628cf35e20c38d18906";
    const result = scryptHex(new Uint8Array(0), new Uint8Array(0), 16, 1, 1, 64);
    expect(result).toBe(expected);
  });
});

// ─── Output Length ────────────────────────────────────────────────────────────

describe("output length", () => {
  it("produces exactly dkLen bytes", () => {
    for (const dkLen of [1, 16, 32, 48, 64]) {
      const result = scrypt(enc("pw"), enc("salt"), 16, 1, 1, dkLen);
      expect(result.length).toBe(dkLen);
    }
  });

  it("short output is a prefix of longer output", () => {
    // PBKDF2's block construction guarantees this property: the first 32 bytes
    // of a 64-byte key match a standalone 32-byte key derivation.
    const short = scrypt(enc("pw"), enc("salt"), 16, 1, 1, 32);
    const full = scrypt(enc("pw"), enc("salt"), 16, 1, 1, 64);
    expect(Array.from(short)).toEqual(Array.from(full.slice(0, 32)));
  });
});

// ─── Determinism ──────────────────────────────────────────────────────────────

describe("determinism", () => {
  it("same inputs always produce the same output", () => {
    const a = scrypt(enc("secret"), enc("nacl"), 16, 1, 1, 32);
    const b = scrypt(enc("secret"), enc("nacl"), 16, 1, 1, 32);
    expect(Array.from(a)).toEqual(Array.from(b));
  });
});

// ─── Sensitivity to Parameters ────────────────────────────────────────────────

describe("parameter sensitivity", () => {
  const base = () => scrypt(enc("password"), enc("salt"), 16, 1, 1, 32);

  it("different password produces different output", () => {
    const result = scrypt(enc("password2"), enc("salt"), 16, 1, 1, 32);
    expect(Array.from(result)).not.toEqual(Array.from(base()));
  });

  it("different salt produces different output", () => {
    const result = scrypt(enc("password"), enc("salt2"), 16, 1, 1, 32);
    expect(Array.from(result)).not.toEqual(Array.from(base()));
  });

  it("different N produces different output", () => {
    const result = scrypt(enc("password"), enc("salt"), 32, 1, 1, 32);
    expect(Array.from(result)).not.toEqual(Array.from(base()));
  });

  it("different r produces different output", () => {
    const result = scrypt(enc("password"), enc("salt"), 16, 2, 1, 32);
    expect(Array.from(result)).not.toEqual(Array.from(base()));
  });

  it("different p produces different output", () => {
    const result = scrypt(enc("password"), enc("salt"), 16, 1, 2, 32);
    expect(Array.from(result)).not.toEqual(Array.from(base()));
  });

  it("empty password produces different output from non-empty", () => {
    const result = scrypt(new Uint8Array(0), enc("salt"), 16, 1, 1, 32);
    expect(Array.from(result)).not.toEqual(Array.from(base()));
  });

  it("empty salt produces different output from non-empty", () => {
    const result = scrypt(enc("password"), new Uint8Array(0), 16, 1, 1, 32);
    expect(Array.from(result)).not.toEqual(Array.from(base()));
  });
});

// ─── Error Cases ──────────────────────────────────────────────────────────────

describe("error cases", () => {
  it("N=1 throws (N must be >= 2)", () => {
    expect(() => scrypt(enc("pw"), enc("salt"), 1, 1, 1, 32)).toThrow(
      "N must be a power of 2 and >= 2",
    );
  });

  it("N not a power of 2 throws", () => {
    expect(() => scrypt(enc("pw"), enc("salt"), 3, 1, 1, 32)).toThrow(
      "N must be a power of 2 and >= 2",
    );
  });

  it("N=0 throws", () => {
    expect(() => scrypt(enc("pw"), enc("salt"), 0, 1, 1, 32)).toThrow(
      "N must be a power of 2 and >= 2",
    );
  });

  it("N > 2^20 throws", () => {
    expect(() => scrypt(enc("pw"), enc("salt"), 1 << 21, 1, 1, 32)).toThrow(
      "N must not exceed 2^20",
    );
  });

  it("r=0 throws", () => {
    expect(() => scrypt(enc("pw"), enc("salt"), 16, 0, 1, 32)).toThrow(
      "r must be a positive integer",
    );
  });

  it("p=0 throws", () => {
    expect(() => scrypt(enc("pw"), enc("salt"), 16, 1, 0, 32)).toThrow(
      "p must be a positive integer",
    );
  });

  it("dkLen=0 throws", () => {
    expect(() => scrypt(enc("pw"), enc("salt"), 16, 1, 1, 0)).toThrow(
      "dk_len must be a positive integer",
    );
  });

  it("dkLen > 2^20 throws", () => {
    expect(() => scrypt(enc("pw"), enc("salt"), 16, 1, 1, 1 << 21)).toThrow(
      "dk_len must not exceed 2^20",
    );
  });

  it("p*r too large throws", () => {
    // p=2^30, r=2 → p*r = 2^31 > 2^30
    expect(() =>
      scrypt(enc("pw"), enc("salt"), 2, 2, 1 << 30, 32),
    ).toThrow("p * r exceeds limit");
  });
});
