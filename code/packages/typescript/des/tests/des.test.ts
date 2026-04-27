/**
 * Tests for @coding-adventures/des — DES and 3DES block cipher.
 *
 * Coverage targets:
 *   - NIST FIPS 46 / SP 800-20 known-answer test vectors
 *   - Key schedule (expandKey): length, subkey size, determinism
 *   - Single-block encrypt/decrypt
 *   - Round-trip property: decrypt(encrypt(x)) == x
 *   - ECB mode: multi-block, PKCS#7 padding, boundary conditions
 *   - 3DES (TDEA) encrypt/decrypt
 *   - Backward compatibility: K1=K2=K3 reduces to single DES
 *   - Error handling: invalid key/block lengths, bad ciphertext
 */

import { describe, it, expect } from "vitest";
import {
  expandKey,
  desEncryptBlock,
  desDecryptBlock,
  desEcbEncrypt,
  desEcbDecrypt,
  tdeaEncryptBlock,
  tdeaDecryptBlock,
  toHex,
  fromHex,
} from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/** Decode a hex string (spaces ignored) to Uint8Array. */
function h(hex: string): Uint8Array {
  return fromHex(hex.replace(/\s+/g, ""));
}

/** Encode Uint8Array to uppercase hex for comparison with test vectors. */
function enc(bytes: Uint8Array): string {
  return toHex(bytes).toUpperCase();
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility
// ─────────────────────────────────────────────────────────────────────────────

describe("toHex / fromHex", () => {
  it("round-trips correctly", () => {
    const original = h("133457799BBCDFF1");
    expect(toHex(original)).toBe("133457799bbcdff1");
    expect(fromHex(toHex(original))).toEqual(original);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Known-Answer Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("desEncryptBlock — FIPS vectors", () => {
  it("FIPS 46 worked example: key=133457799BBCDFF1, plain=0123456789ABCDEF", () => {
    const key   = h("133457799BBCDFF1");
    const plain = h("0123456789ABCDEF");
    expect(enc(desEncryptBlock(plain, key))).toBe("85E813540F0AB405");
  });

  it("SP 800-20 Table 1 row 0: key=0101…01, plain=95F8…", () => {
    const key   = h("0101010101010101");
    const plain = h("95F8A5E5DD31D900");
    expect(enc(desEncryptBlock(plain, key))).toBe("8000000000000000");
  });

  it("SP 800-20 Table 1 row 1: key=0101…01, plain=DD7F…", () => {
    const key   = h("0101010101010101");
    const plain = h("DD7F121CA5015619");
    expect(enc(desEncryptBlock(plain, key))).toBe("4000000000000000");
  });

  it("SP 800-20 Table 2: key=8001010101010101, plain=0000…", () => {
    const key   = h("8001010101010101");
    const plain = h("0000000000000000");
    expect(enc(desEncryptBlock(plain, key))).toBe("95A8D72813DAA94D");
  });

  it("SP 800-20 Table 2 row 1: key=4001010101010101", () => {
    const key   = h("4001010101010101");
    const plain = h("0000000000000000");
    expect(enc(desEncryptBlock(plain, key))).toBe("0EEC1487DD8C26D5");
  });
});

describe("desDecryptBlock — FIPS vectors", () => {
  it("decrypts FIPS vector 1", () => {
    const key    = h("133457799BBCDFF1");
    const cipher = h("85E813540F0AB405");
    expect(enc(desDecryptBlock(cipher, key))).toBe("0123456789ABCDEF");
  });

  it("round-trip: decrypt(encrypt(x)) == x — FIPS key", () => {
    const key   = h("133457799BBCDFF1");
    const plain = h("0123456789ABCDEF");
    const ct = desEncryptBlock(plain, key);
    expect(desDecryptBlock(ct, key)).toEqual(plain);
  });

  it("round-trips multiple keys", () => {
    const keys = [
      h("133457799BBCDFF0"),
      h("FFFFFFFFFFFFFFFF"),
      h("0000000000000000"),
      h("FEDCBA9876543210"),
    ];
    const plain = h("0123456789ABCDEF");
    for (const key of keys) {
      const ct = desEncryptBlock(plain, key);
      expect(desDecryptBlock(ct, key)).toEqual(plain);
    }
  });

  it("round-trips all byte values in plaintext", () => {
    const key = h("FEDCBA9876543210");
    for (let start = 0; start < 256; start += 16) {
      const block = new Uint8Array(8);
      for (let i = 0; i < 8; i++) block[i] = (start + i) & 0xFF;
      const ct = desEncryptBlock(block, key);
      expect(desDecryptBlock(ct, key)).toEqual(block);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Key Schedule
// ─────────────────────────────────────────────────────────────────────────────

describe("expandKey", () => {
  it("returns exactly 16 subkeys", () => {
    const key = h("0133457799BBCDFF");
    expect(expandKey(key)).toHaveLength(16);
  });

  it("each subkey is 6 bytes (48 bits)", () => {
    const key = h("0133457799BBCDFF");
    for (const sk of expandKey(key)) {
      expect(sk).toHaveLength(6);
    }
  });

  it("different keys produce different subkeys", () => {
    const sk1 = expandKey(h("0133457799BBCDFF"));
    const sk2 = expandKey(h("FEDCBA9876543210"));
    expect(toHex(sk1[0])).not.toBe(toHex(sk2[0]));
  });

  it("all 16 subkeys are not identical (non-degenerate schedule)", () => {
    const key = h("0133457799BBCDFF");
    const subkeys = expandKey(key);
    const hexSet = new Set(subkeys.map(toHex));
    expect(hexSet.size).toBeGreaterThan(1);
  });

  it("throws for key shorter than 8 bytes", () => {
    expect(() => expandKey(new Uint8Array(7))).toThrow(/8 bytes/);
  });

  it("throws for key longer than 8 bytes", () => {
    expect(() => expandKey(new Uint8Array(9))).toThrow(/8 bytes/);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// ECB mode
// ─────────────────────────────────────────────────────────────────────────────

describe("desEcbEncrypt", () => {
  const KEY = h("0133457799BBCDFF");

  it("8-byte input → 16 bytes out (data block + padding block)", () => {
    const ct = desEcbEncrypt(h("0123456789ABCDEF"), KEY);
    expect(ct).toHaveLength(16);
  });

  it("5-byte input → 8 bytes out (padded to one block)", () => {
    const ct = desEcbEncrypt(new TextEncoder().encode("hello"), KEY);
    expect(ct).toHaveLength(8);
  });

  it("16-byte input → 24 bytes out (two data blocks + padding)", () => {
    const ct = desEcbEncrypt(new Uint8Array(16), KEY);
    expect(ct).toHaveLength(24);
  });

  it("empty input → 8 bytes (full padding block)", () => {
    const ct = desEcbEncrypt(new Uint8Array(0), KEY);
    expect(ct).toHaveLength(8);
  });

  it("is deterministic", () => {
    const plain = new TextEncoder().encode("Hello, World!!!");
    const ct1 = desEcbEncrypt(plain, KEY);
    const ct2 = desEcbEncrypt(plain, KEY);
    expect(ct1).toEqual(ct2);
  });

  it("output is a Uint8Array", () => {
    expect(desEcbEncrypt(new Uint8Array(4), KEY)).toBeInstanceOf(Uint8Array);
  });
});

describe("desEcbDecrypt", () => {
  const KEY = h("0133457799BBCDFF");

  it("round-trip short plaintext", () => {
    const plain = new TextEncoder().encode("hello");
    expect(desEcbDecrypt(desEcbEncrypt(plain, KEY), KEY)).toEqual(plain);
  });

  it("round-trip exactly 8 bytes", () => {
    const plain = new TextEncoder().encode("ABCDEFGH");
    expect(desEcbDecrypt(desEcbEncrypt(plain, KEY), KEY)).toEqual(plain);
  });

  it("round-trip multi-block plaintext", () => {
    const plain = new TextEncoder().encode("The quick brown fox jumps");
    expect(desEcbDecrypt(desEcbEncrypt(plain, KEY), KEY)).toEqual(plain);
  });

  it("round-trip empty plaintext", () => {
    const plain = new Uint8Array(0);
    expect(desEcbDecrypt(desEcbEncrypt(plain, KEY), KEY)).toEqual(plain);
  });

  it("round-trip 256-byte plaintext", () => {
    const plain = new Uint8Array(256);
    for (let i = 0; i < 256; i++) plain[i] = i;
    expect(desEcbDecrypt(desEcbEncrypt(plain, KEY), KEY)).toEqual(plain);
  });

  it("throws for ciphertext not a multiple of 8 bytes", () => {
    expect(() => desEcbDecrypt(new Uint8Array(7), KEY)).toThrow(/multiple of 8/);
  });

  it("throws for empty ciphertext", () => {
    expect(() => desEcbDecrypt(new Uint8Array(0), KEY)).toThrow();
  });

  it("throws on corrupted padding", () => {
    const ct = desEcbEncrypt(new TextEncoder().encode("test data"), KEY);
    // Corrupt the last byte
    const corrupted = new Uint8Array(ct);
    corrupted[corrupted.length - 1] ^= 0xFF;
    expect(() => desEcbDecrypt(corrupted, KEY)).toThrow();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Triple DES (TDEA)
// ─────────────────────────────────────────────────────────────────────────────

describe("tdeaEncryptBlock / tdeaDecryptBlock", () => {
  // NIST SP 800-67 EDE ordering: E_K1(D_K2(E_K3(P)))
  const K1    = h("0123456789ABCDEF");
  const K2    = h("23456789ABCDEF01");
  const K3    = h("456789ABCDEF0123");
  const PLAIN = h("6BC1BEE22E409F96");
  const CIPHER = h("3B6423D418DEFC23");

  it("3TDEA encrypt", () => {
    expect(tdeaEncryptBlock(PLAIN, K1, K2, K3)).toEqual(CIPHER);
  });

  it("3TDEA decrypt", () => {
    expect(tdeaDecryptBlock(CIPHER, K1, K2, K3)).toEqual(PLAIN);
  });

  it("round-trip with random-ish keys", () => {
    const k1 = h("FEDCBA9876543210");
    const k2 = h("0F1E2D3C4B5A6978");
    const k3 = h("7869584A3B2C1D0E");
    const plain = h("0123456789ABCDEF");
    const ct = tdeaEncryptBlock(plain, k1, k2, k3);
    expect(tdeaDecryptBlock(ct, k1, k2, k3)).toEqual(plain);
  });

  it("K1=K2=K3 reduces to single DES (EDE backward compatibility)", () => {
    const key = h("0133457799BBCDFF");
    const plain = h("0123456789ABCDEF");
    // 3DES EDE with identical keys = single DES
    // E(K, D(K, E(K, P))) = E(K, P)
    expect(tdeaEncryptBlock(plain, key, key, key)).toEqual(desEncryptBlock(plain, key));
  });

  it("decrypt backward compat: K1=K2=K3 reduces to single DES decrypt", () => {
    const key = h("FEDCBA9876543210");
    const ct  = h("0123456789ABCDEF");
    expect(tdeaDecryptBlock(ct, key, key, key)).toEqual(desDecryptBlock(ct, key));
  });

  it("round-trips blocks with repeated byte patterns", () => {
    const k1 = h("1234567890ABCDEF");
    const k2 = h("FEDCBA0987654321");
    const k3 = h("0F0F0F0F0F0F0F0F");
    for (const val of [0x00, 0xFF, 0xA5, 0x5A]) {
      const plain = new Uint8Array(8).fill(val);
      expect(tdeaDecryptBlock(tdeaEncryptBlock(plain, k1, k2, k3), k1, k2, k3)).toEqual(plain);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Invalid input handling
// ─────────────────────────────────────────────────────────────────────────────

describe("error handling", () => {
  const KEY = h("0133457799BBCDFF");

  it("desEncryptBlock: block too short", () => {
    expect(() => desEncryptBlock(new Uint8Array(7), KEY)).toThrow(/8 bytes/);
  });

  it("desEncryptBlock: block too long", () => {
    expect(() => desEncryptBlock(new Uint8Array(16), KEY)).toThrow(/8 bytes/);
  });

  it("desDecryptBlock: block wrong size", () => {
    expect(() => desDecryptBlock(new Uint8Array(9), KEY)).toThrow(/8 bytes/);
  });

  it("desEncryptBlock: key wrong size", () => {
    expect(() => desEncryptBlock(new Uint8Array(8), new Uint8Array(4))).toThrow(/8 bytes/);
  });

  it("desDecryptBlock: key wrong size", () => {
    expect(() => desDecryptBlock(new Uint8Array(8), new Uint8Array(16))).toThrow(/8 bytes/);
  });
});
