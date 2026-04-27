/**
 * Tests for @coding-adventures/aes — AES block cipher.
 *
 * Coverage targets:
 *   - FIPS 197 Appendix B and C known-answer tests (all three key sizes)
 *   - S-box properties: bijection, known values, no fixed points
 *   - Key schedule: correct round count, word structure, first round key == key
 *   - Single-block encrypt/decrypt
 *   - Round-trip: decrypt(encrypt(x)) == x for all key sizes
 *   - Avalanche effect
 *   - Error handling: wrong block/key lengths
 */

import { describe, it, expect } from "vitest";
import {
  aesEncryptBlock,
  aesDecryptBlock,
  expandKey,
  SBOX,
  INV_SBOX,
  toHex,
  fromHex,
} from "../src/index.js";

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function h(hex: string): Uint8Array {
  return fromHex(hex.replace(/\s+/g, ""));
}

// ─────────────────────────────────────────────────────────────────────────────
// FIPS 197 Known-Answer Tests
// ─────────────────────────────────────────────────────────────────────────────

describe("AES-128 — FIPS 197 Appendix B", () => {
  const KEY   = h("2b7e151628aed2a6abf7158809cf4f3c");
  const PLAIN = h("3243f6a8885a308d313198a2e0370734");
  const CIPHER = h("3925841d02dc09fbdc118597196a0b32");

  it("encrypt", () => {
    expect(aesEncryptBlock(PLAIN, KEY)).toEqual(CIPHER);
  });

  it("decrypt", () => {
    expect(aesDecryptBlock(CIPHER, KEY)).toEqual(PLAIN);
  });

  it("round-trip multiple plaintexts", () => {
    for (let start = 0; start < 256; start += 32) {
      const plain = new Uint8Array(16);
      for (let i = 0; i < 16; i++) plain[i] = (start + i) & 0xFF;
      const ct = aesEncryptBlock(plain, KEY);
      expect(aesDecryptBlock(ct, KEY)).toEqual(plain);
    }
  });

  // FIPS 197 Appendix C.1
  it("Appendix C.1: key=000102…0f, plain=001122…ff", () => {
    const key   = h("000102030405060708090a0b0c0d0e0f");
    const plain = h("00112233445566778899aabbccddeeff");
    const ct    = h("69c4e0d86a7b0430d8cdb78070b4c55a");
    expect(aesEncryptBlock(plain, key)).toEqual(ct);
    expect(aesDecryptBlock(ct, key)).toEqual(plain);
  });
});

describe("AES-192 — FIPS 197 Appendix C.2", () => {
  const KEY    = h("000102030405060708090a0b0c0d0e0f1011121314151617");
  const PLAIN  = h("00112233445566778899aabbccddeeff");
  const CIPHER = h("dda97ca4864cdfe06eaf70a0ec0d7191");

  it("encrypt", () => {
    expect(aesEncryptBlock(PLAIN, KEY)).toEqual(CIPHER);
  });

  it("decrypt", () => {
    expect(aesDecryptBlock(CIPHER, KEY)).toEqual(PLAIN);
  });

  it("round-trip multiple plaintexts", () => {
    for (let start = 0; start < 256; start += 32) {
      const plain = new Uint8Array(16);
      for (let i = 0; i < 16; i++) plain[i] = (start + i) & 0xFF;
      const ct = aesEncryptBlock(plain, KEY);
      expect(aesDecryptBlock(ct, KEY)).toEqual(plain);
    }
  });
});

describe("AES-256 — FIPS 197", () => {
  const KEY    = h("603deb1015ca71be2b73aef0857d7781 1f352c073b6108d72d9810a30914dff4");
  const PLAIN  = h("6bc1bee22e409f96e93d7e117393172a");
  const CIPHER = h("f3eed1bdb5d2a03c064b5a7e3db181f8");

  it("encrypt", () => {
    expect(aesEncryptBlock(PLAIN, KEY)).toEqual(CIPHER);
  });

  it("decrypt", () => {
    expect(aesDecryptBlock(CIPHER, KEY)).toEqual(PLAIN);
  });

  it("round-trip multiple plaintexts", () => {
    for (let start = 0; start < 256; start += 32) {
      const plain = new Uint8Array(16);
      for (let i = 0; i < 16; i++) plain[i] = (start + i) & 0xFF;
      const ct = aesEncryptBlock(plain, KEY);
      expect(aesDecryptBlock(ct, KEY)).toEqual(plain);
    }
  });

  // FIPS 197 Appendix C.3
  it("Appendix C.3: key=000102…1f, plain=001122…ff", () => {
    const key   = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
    const plain = h("00112233445566778899aabbccddeeff");
    const ct    = h("8ea2b7ca516745bfeafc49904b496089");
    expect(aesEncryptBlock(plain, key)).toEqual(ct);
    expect(aesDecryptBlock(ct, key)).toEqual(plain);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// S-box properties
// ─────────────────────────────────────────────────────────────────────────────

describe("SBOX / INV_SBOX properties", () => {
  it("SBOX has exactly 256 entries", () => {
    expect(SBOX.length).toBe(256);
  });

  it("INV_SBOX has exactly 256 entries", () => {
    expect(INV_SBOX.length).toBe(256);
  });

  it("SBOX is a bijection (all 256 outputs distinct)", () => {
    const values = [...SBOX].sort((a, b) => a - b);
    for (let i = 0; i < 256; i++) {
      expect(values[i]).toBe(i);
    }
  });

  it("INV_SBOX is a bijection", () => {
    const values = [...INV_SBOX].sort((a, b) => a - b);
    for (let i = 0; i < 256; i++) {
      expect(values[i]).toBe(i);
    }
  });

  it("INV_SBOX[SBOX[b]] == b for all b", () => {
    for (let b = 0; b < 256; b++) {
      expect(INV_SBOX[SBOX[b]]).toBe(b);
    }
  });

  it("SBOX[0x00] == 0x63 (FIPS 197 Figure 7)", () => {
    expect(SBOX[0x00]).toBe(0x63);
  });

  it("SBOX[0x01] == 0x7c (FIPS 197 Figure 7)", () => {
    expect(SBOX[0x01]).toBe(0x7c);
  });

  it("SBOX[0xff] == 0x16 (FIPS 197 Figure 7)", () => {
    expect(SBOX[0xff]).toBe(0x16);
  });

  it("SBOX[0x53] == 0xed (FIPS 197 Figure 7)", () => {
    expect(SBOX[0x53]).toBe(0xed);
  });

  it("INV_SBOX[0x63] == 0x00", () => {
    expect(INV_SBOX[0x63]).toBe(0x00);
  });

  it("no fixed points: SBOX[b] !== b for all b", () => {
    for (let b = 0; b < 256; b++) {
      expect(SBOX[b]).not.toBe(b);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Key schedule
// ─────────────────────────────────────────────────────────────────────────────

describe("expandKey", () => {
  it("AES-128 produces 11 round keys (Nr+1 = 11)", () => {
    expect(expandKey(new Uint8Array(16))).toHaveLength(11);
  });

  it("AES-192 produces 13 round keys (Nr+1 = 13)", () => {
    expect(expandKey(new Uint8Array(24))).toHaveLength(13);
  });

  it("AES-256 produces 15 round keys (Nr+1 = 15)", () => {
    expect(expandKey(new Uint8Array(32))).toHaveLength(15);
  });

  it("each round key is a 4×4 matrix of valid bytes", () => {
    for (const keyLen of [16, 24, 32]) {
      const rks = expandKey(new Uint8Array(keyLen));
      for (const rk of rks) {
        expect(rk).toHaveLength(4);
        for (const row of rk) {
          expect(row).toHaveLength(4);
          for (const val of row) {
            expect(val).toBeGreaterThanOrEqual(0);
            expect(val).toBeLessThanOrEqual(255);
          }
        }
      }
    }
  });

  it("first round key equals the key bytes (column-major)", () => {
    const key = h("2b7e151628aed2a6abf7158809cf4f3c");
    const rks = expandKey(key);
    // Reconstruct first 16 bytes from round_key[0] column-major
    const reconstructed = new Uint8Array(16);
    for (let col = 0; col < 4; col++) {
      for (let row = 0; row < 4; row++) {
        reconstructed[row + 4 * col] = rks[0][row][col];
      }
    }
    expect(reconstructed).toEqual(key);
  });

  it("different keys produce different round keys", () => {
    const rks1 = expandKey(new Uint8Array(16));
    const rks2 = expandKey(new Uint8Array(16).fill(1));
    // Compare the first round key
    expect(rks1[0]).not.toEqual(rks2[0]);
  });

  it("throws for 15-byte key", () => {
    expect(() => expandKey(new Uint8Array(15))).toThrow(/16, 24, or 32/);
  });

  it("throws for 17-byte key", () => {
    expect(() => expandKey(new Uint8Array(17))).toThrow(/16, 24, or 32/);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Block size validation
// ─────────────────────────────────────────────────────────────────────────────

describe("block size validation", () => {
  const KEY = new Uint8Array(16);

  it("encrypt throws for 15-byte block", () => {
    expect(() => aesEncryptBlock(new Uint8Array(15), KEY)).toThrow(/16 bytes/);
  });

  it("encrypt throws for 17-byte block", () => {
    expect(() => aesEncryptBlock(new Uint8Array(17), KEY)).toThrow(/16 bytes/);
  });

  it("decrypt throws for 15-byte block", () => {
    expect(() => aesDecryptBlock(new Uint8Array(15), KEY)).toThrow(/16 bytes/);
  });

  it("encrypt throws for wrong key size (10 bytes)", () => {
    expect(() => aesEncryptBlock(new Uint8Array(16), new Uint8Array(10))).toThrow(/16, 24, or 32/);
  });

  it("decrypt throws for wrong key size (20 bytes)", () => {
    expect(() => aesDecryptBlock(new Uint8Array(16), new Uint8Array(20))).toThrow(/16, 24, or 32/);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Round-trip tests
// ─────────────────────────────────────────────────────────────────────────────

describe("round-trip correctness", () => {
  it("all-zero key and plaintext", () => {
    for (const keyLen of [16, 24, 32]) {
      const key = new Uint8Array(keyLen);
      const plain = new Uint8Array(16);
      expect(aesDecryptBlock(aesEncryptBlock(plain, key), key)).toEqual(plain);
    }
  });

  it("all-0xFF key and plaintext", () => {
    for (const keyLen of [16, 24, 32]) {
      const key = new Uint8Array(keyLen).fill(0xFF);
      const plain = new Uint8Array(16).fill(0xFF);
      expect(aesDecryptBlock(aesEncryptBlock(plain, key), key)).toEqual(plain);
    }
  });

  it("sequential key and plaintext bytes", () => {
    for (const keyLen of [16, 24, 32]) {
      const key = new Uint8Array(keyLen);
      for (let i = 0; i < keyLen; i++) key[i] = i;
      const plain = new Uint8Array(16);
      for (let i = 0; i < 16; i++) plain[i] = i;
      expect(aesDecryptBlock(aesEncryptBlock(plain, key), key)).toEqual(plain);
    }
  });

  it("avalanche effect: flipping one plaintext bit changes many output bytes", () => {
    const key = new Uint8Array(16);
    for (let i = 0; i < 16; i++) key[i] = i;
    const plain1 = new Uint8Array(16);
    const plain2 = new Uint8Array(16);
    plain2[0] = 0x01;  // flip one bit
    const ct1 = aesEncryptBlock(plain1, key);
    const ct2 = aesEncryptBlock(plain2, key);
    let diffBits = 0;
    for (let i = 0; i < 16; i++) {
      diffBits += (ct1[i] ^ ct2[i]).toString(2).split("1").length - 1;
    }
    expect(diffBits).toBeGreaterThan(32);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Utility
// ─────────────────────────────────────────────────────────────────────────────

describe("toHex / fromHex", () => {
  it("round-trips correctly", () => {
    const original = h("3925841d02dc09fbdc118597196a0b32");
    expect(toHex(fromHex(toHex(original)))).toBe(toHex(original));
  });
});
