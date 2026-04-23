// ============================================================================
// HKDF Tests — RFC 5869 Test Vectors + Edge Cases
// ============================================================================
//
// These tests verify the HKDF implementation against all three test cases
// from RFC 5869 (Appendix A), plus additional edge cases for robustness.
//
// Each RFC test vector specifies:
//   - IKM (Input Keying Material)
//   - salt
//   - info
//   - L (desired output length)
//   - PRK (expected pseudorandom key from Extract)
//   - OKM (expected output keying material from Expand)
//
// We verify both the Extract and Expand phases independently, as well
// as the combined hkdf() function.
//
// ============================================================================

import { describe, it, expect } from "vitest";
import { hkdfExtract, hkdfExpand, hkdf } from "../src/index.js";

// ============================================================================
// Helper: Convert hex string to Uint8Array
// ============================================================================
//
// RFC test vectors are given as hex strings. This helper converts them
// to byte arrays for use with our functions.
//
// ============================================================================

function hexToBytes(hex: string): Uint8Array {
  if (hex.length === 0) return new Uint8Array(0);
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// ============================================================================
// RFC 5869 Test Case 1: Basic SHA-256
// ============================================================================
//
// This is the simplest test case: a 22-byte IKM, 13-byte salt, 10-byte
// info, requesting 42 bytes of output.
//
// ============================================================================

describe("RFC 5869 Test Case 1: SHA-256 basic", () => {
  const ikm = hexToBytes("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
  const salt = hexToBytes("000102030405060708090a0b0c");
  const info = hexToBytes("f0f1f2f3f4f5f6f7f8f9");
  const expectedPRK = hexToBytes(
    "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5"
  );
  const expectedOKM = hexToBytes(
    "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf" +
      "34007208d5b887185865"
  );

  it("extract produces correct PRK", () => {
    const prk = hkdfExtract(salt, ikm, "sha256");
    expect(bytesToHex(prk)).toBe(bytesToHex(expectedPRK));
  });

  it("expand produces correct OKM", () => {
    const okm = hkdfExpand(expectedPRK, info, 42, "sha256");
    expect(bytesToHex(okm)).toBe(bytesToHex(expectedOKM));
  });

  it("combined hkdf produces correct OKM", () => {
    const okm = hkdf(salt, ikm, info, 42, "sha256");
    expect(bytesToHex(okm)).toBe(bytesToHex(expectedOKM));
  });
});

// ============================================================================
// RFC 5869 Test Case 2: SHA-256 with longer inputs/outputs
// ============================================================================
//
// This test exercises HKDF with 80-byte IKM, 80-byte salt, 80-byte info,
// and 82 bytes of output (requiring 3 HMAC blocks since ceil(82/32) = 3).
//
// ============================================================================

describe("RFC 5869 Test Case 2: SHA-256 longer inputs", () => {
  const ikm = hexToBytes(
    "000102030405060708090a0b0c0d0e0f" +
      "101112131415161718191a1b1c1d1e1f" +
      "202122232425262728292a2b2c2d2e2f" +
      "303132333435363738393a3b3c3d3e3f" +
      "404142434445464748494a4b4c4d4e4f"
  );
  const salt = hexToBytes(
    "606162636465666768696a6b6c6d6e6f" +
      "707172737475767778797a7b7c7d7e7f" +
      "808182838485868788898a8b8c8d8e8f" +
      "909192939495969798999a9b9c9d9e9f" +
      "a0a1a2a3a4a5a6a7a8a9aaabacadaeaf"
  );
  const info = hexToBytes(
    "b0b1b2b3b4b5b6b7b8b9babbbcbdbebf" +
      "c0c1c2c3c4c5c6c7c8c9cacbcccdcecf" +
      "d0d1d2d3d4d5d6d7d8d9dadbdcdddedf" +
      "e0e1e2e3e4e5e6e7e8e9eaebecedeeef" +
      "f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"
  );
  const expectedPRK = hexToBytes(
    "06a6b88c5853361a06104c9ceb35b45cef760014904671014a193f40c15fc244"
  );
  const expectedOKM = hexToBytes(
    "b11e398dc80327a1c8e7f78c596a4934" +
      "4f012eda2d4efad8a050cc4c19afa97c" +
      "59045a99cac7827271cb41c65e590e09" +
      "da3275600c2f09b8367793a9aca3db71" +
      "cc30c58179ec3e87c14c01d5c1f3434f" +
      "1d87"
  );

  it("extract produces correct PRK", () => {
    const prk = hkdfExtract(salt, ikm, "sha256");
    expect(bytesToHex(prk)).toBe(bytesToHex(expectedPRK));
  });

  it("expand produces correct OKM", () => {
    const okm = hkdfExpand(expectedPRK, info, 82, "sha256");
    expect(bytesToHex(okm)).toBe(bytesToHex(expectedOKM));
  });

  it("combined hkdf produces correct OKM", () => {
    const okm = hkdf(salt, ikm, info, 82, "sha256");
    expect(bytesToHex(okm)).toBe(bytesToHex(expectedOKM));
  });
});

// ============================================================================
// RFC 5869 Test Case 3: SHA-256 with empty salt and info
// ============================================================================
//
// This tests the edge case where both salt and info are empty. When salt
// is empty, HKDF uses HashLen (32) zero bytes as the HMAC key. When info
// is empty, the HMAC input for each block is just T(i-1) || counter_byte.
//
// ============================================================================

describe("RFC 5869 Test Case 3: SHA-256 empty salt and info", () => {
  const ikm = hexToBytes("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
  const salt = new Uint8Array(0);
  const info = new Uint8Array(0);
  const expectedPRK = hexToBytes(
    "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04"
  );
  const expectedOKM = hexToBytes(
    "8da4e775a563c18f715f802a063c5a31" +
      "b8a11f5c5ee1879ec3454e5f3c738d2d" +
      "9d201395faa4b61a96c8"
  );

  it("extract produces correct PRK", () => {
    const prk = hkdfExtract(salt, ikm, "sha256");
    expect(bytesToHex(prk)).toBe(bytesToHex(expectedPRK));
  });

  it("expand produces correct OKM", () => {
    const okm = hkdfExpand(expectedPRK, info, 42, "sha256");
    expect(bytesToHex(okm)).toBe(bytesToHex(expectedOKM));
  });

  it("combined hkdf produces correct OKM", () => {
    const okm = hkdf(salt, ikm, info, 42, "sha256");
    expect(bytesToHex(okm)).toBe(bytesToHex(expectedOKM));
  });
});

// ============================================================================
// Edge Cases
// ============================================================================

describe("Edge cases", () => {
  it("throws when length is 0", () => {
    const prk = new Uint8Array(32);
    const info = new Uint8Array(0);
    expect(() => hkdfExpand(prk, info, 0, "sha256")).toThrow(
      "length must be > 0"
    );
  });

  it("throws when length is negative", () => {
    const prk = new Uint8Array(32);
    const info = new Uint8Array(0);
    expect(() => hkdfExpand(prk, info, -1, "sha256")).toThrow(
      "length must be > 0"
    );
  });

  it("throws when length exceeds maximum for SHA-256", () => {
    // Maximum for SHA-256: 255 * 32 = 8160 bytes
    const prk = new Uint8Array(32);
    const info = new Uint8Array(0);
    expect(() => hkdfExpand(prk, info, 8161, "sha256")).toThrow(
      "exceeds maximum"
    );
  });

  it("throws when length exceeds maximum for SHA-512", () => {
    // Maximum for SHA-512: 255 * 64 = 16320 bytes
    const prk = new Uint8Array(64);
    const info = new Uint8Array(0);
    expect(() => hkdfExpand(prk, info, 16321, "sha512")).toThrow(
      "exceeds maximum"
    );
  });

  it("produces exactly 1 byte when length is 1", () => {
    const ikm = hexToBytes("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
    const salt = new Uint8Array(0);
    const info = new Uint8Array(0);
    const okm = hkdf(salt, ikm, info, 1, "sha256");
    expect(okm.length).toBe(1);
    // First byte of test case 3 OKM
    expect(okm[0]).toBe(0x8d);
  });

  it("SHA-256 max output (255 * 32 = 8160 bytes) does not throw", () => {
    const prk = hkdfExtract(new Uint8Array(0), new Uint8Array(32), "sha256");
    const okm = hkdfExpand(prk, new Uint8Array(0), 8160, "sha256");
    expect(okm.length).toBe(8160);
  });

  it("SHA-512 extract produces 64-byte PRK", () => {
    const ikm = hexToBytes("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
    const prk = hkdfExtract(new Uint8Array(0), ikm, "sha512");
    expect(prk.length).toBe(64);
  });

  it("different info strings produce different output", () => {
    const ikm = hexToBytes("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
    const salt = new Uint8Array(0);
    const info1 = new TextEncoder().encode("encryption");
    const info2 = new TextEncoder().encode("authentication");
    const okm1 = hkdf(salt, ikm, info1, 32);
    const okm2 = hkdf(salt, ikm, info2, 32);
    expect(bytesToHex(okm1)).not.toBe(bytesToHex(okm2));
  });

  it("defaults to sha256 when hash is omitted", () => {
    const ikm = hexToBytes("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b");
    const salt = hexToBytes("000102030405060708090a0b0c");
    const info = hexToBytes("f0f1f2f3f4f5f6f7f8f9");
    // Should match Test Case 1 OKM
    const okm = hkdf(salt, ikm, info, 42);
    expect(bytesToHex(okm)).toBe(
      "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf" +
        "34007208d5b887185865"
    );
  });
});
