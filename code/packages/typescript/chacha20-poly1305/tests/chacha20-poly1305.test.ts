// ============================================================================
// ChaCha20-Poly1305 Tests
// ============================================================================
//
// These tests verify correctness against the official RFC 8439 test vectors.
// Each test vector is taken directly from the RFC sections cited.
//
// ============================================================================

import { describe, it, expect } from "vitest";
import {
  chacha20Encrypt,
  poly1305Mac,
  aeadEncrypt,
  aeadDecrypt,
} from "../src/index.js";

/**
 * Helper: convert a hex string to a Uint8Array.
 */
function fromHex(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

/**
 * Helper: convert a Uint8Array to a hex string.
 */
function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Helper: convert a string to a Uint8Array (UTF-8).
 */
function fromString(str: string): Uint8Array {
  return new TextEncoder().encode(str);
}

// ============================================================================
// ChaCha20 Stream Cipher Tests
// ============================================================================

describe("ChaCha20", () => {
  it("RFC 8439 Section 2.4.2 — Sunscreen test vector", () => {
    // This is the canonical ChaCha20 test vector from the RFC.
    // It encrypts a famous graduation speech excerpt.
    const key = fromHex(
      "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    );
    const nonce = fromHex("000000000000004a00000000");
    const counter = 1;
    const plaintext = fromString(
      "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.",
    );

    const expectedCt = fromHex(
      "6e2e359a2568f98041ba0728dd0d6981" +
        "e97e7aec1d4360c20a27afccfd9fae0b" +
        "f91b65c5524733ab8f593dabcd62b357" +
        "1639d624e65152ab8f530c359f0861d8" +
        "07ca0dbf500d6a6156a38e088a22b65e" +
        "52bc514d16ccf806818ce91ab7793736" +
        "5af90bbf74a35be6b40b8eedf2785e42" +
        "874d",
    );

    const ciphertext = chacha20Encrypt(plaintext, key, nonce, counter);
    expect(toHex(ciphertext)).toBe(toHex(expectedCt));

    // Decryption is the same operation (XOR is its own inverse)
    const decrypted = chacha20Encrypt(ciphertext, key, nonce, counter);
    expect(decrypted).toEqual(plaintext);
  });

  it("encrypts empty plaintext", () => {
    const key = new Uint8Array(32);
    const nonce = new Uint8Array(12);
    const result = chacha20Encrypt(new Uint8Array(0), key, nonce, 0);
    expect(result.length).toBe(0);
  });

  it("encrypts single byte", () => {
    const key = new Uint8Array(32);
    key[0] = 1;
    const nonce = new Uint8Array(12);
    const plaintext = new Uint8Array([0x42]);
    const ct = chacha20Encrypt(plaintext, key, nonce, 0);
    expect(ct.length).toBe(1);
    // Decrypt should recover original
    const pt = chacha20Encrypt(ct, key, nonce, 0);
    expect(pt).toEqual(plaintext);
  });

  it("handles multi-block messages (> 64 bytes)", () => {
    // 200 bytes spans 4 blocks (ceil(200/64) = 4)
    const key = new Uint8Array(32);
    for (let i = 0; i < 32; i++) key[i] = i;
    const nonce = new Uint8Array(12);
    nonce[0] = 0x09;
    const plaintext = new Uint8Array(200);
    for (let i = 0; i < 200; i++) plaintext[i] = i % 256;

    const ct = chacha20Encrypt(plaintext, key, nonce, 0);
    expect(ct.length).toBe(200);
    // Verify round-trip
    const pt = chacha20Encrypt(ct, key, nonce, 0);
    expect(pt).toEqual(plaintext);
  });

  it("rejects invalid key length", () => {
    expect(() =>
      chacha20Encrypt(new Uint8Array(10), new Uint8Array(16), new Uint8Array(12), 0),
    ).toThrow("Key must be 32 bytes");
  });

  it("rejects invalid nonce length", () => {
    expect(() =>
      chacha20Encrypt(new Uint8Array(10), new Uint8Array(32), new Uint8Array(8), 0),
    ).toThrow("Nonce must be 12 bytes");
  });

  it("different keys produce different ciphertexts", () => {
    const key1 = new Uint8Array(32);
    key1[0] = 1;
    const key2 = new Uint8Array(32);
    key2[0] = 2;
    const nonce = new Uint8Array(12);
    const plaintext = fromString("Hello, World!");
    const ct1 = chacha20Encrypt(plaintext, key1, nonce, 0);
    const ct2 = chacha20Encrypt(plaintext, key2, nonce, 0);
    expect(toHex(ct1)).not.toBe(toHex(ct2));
  });

  it("different nonces produce different ciphertexts", () => {
    const key = new Uint8Array(32);
    const nonce1 = new Uint8Array(12);
    nonce1[0] = 1;
    const nonce2 = new Uint8Array(12);
    nonce2[0] = 2;
    const plaintext = fromString("Hello, World!");
    const ct1 = chacha20Encrypt(plaintext, key, nonce1, 0);
    const ct2 = chacha20Encrypt(plaintext, key, nonce2, 0);
    expect(toHex(ct1)).not.toBe(toHex(ct2));
  });

  it("different counters produce different ciphertexts", () => {
    const key = new Uint8Array(32);
    const nonce = new Uint8Array(12);
    const plaintext = fromString("Hello, World!");
    const ct1 = chacha20Encrypt(plaintext, key, nonce, 0);
    const ct2 = chacha20Encrypt(plaintext, key, nonce, 1);
    expect(toHex(ct1)).not.toBe(toHex(ct2));
  });
});

// ============================================================================
// Poly1305 MAC Tests
// ============================================================================

describe("Poly1305", () => {
  it("RFC 8439 Section 2.5.2 — Cryptographic Forum Research Group", () => {
    const key = fromHex(
      "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b",
    );
    const message = fromString("Cryptographic Forum Research Group");
    const expectedTag = fromHex("a8061dc1305136c6c22b8baf0c0127a9");

    const tag = poly1305Mac(message, key);
    expect(toHex(tag)).toBe(toHex(expectedTag));
  });

  it("empty message produces a valid tag", () => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 32; i++) key[i] = i;
    const tag = poly1305Mac(new Uint8Array(0), key);
    expect(tag.length).toBe(16);
  });

  it("single byte message", () => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 32; i++) key[i] = i;
    const tag = poly1305Mac(new Uint8Array([0x42]), key);
    expect(tag.length).toBe(16);
  });

  it("different messages produce different tags", () => {
    const key = fromHex(
      "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b",
    );
    const tag1 = poly1305Mac(fromString("Message A"), key);
    const tag2 = poly1305Mac(fromString("Message B"), key);
    expect(toHex(tag1)).not.toBe(toHex(tag2));
  });

  it("different keys produce different tags", () => {
    const key1 = new Uint8Array(32);
    key1[0] = 1;
    const key2 = new Uint8Array(32);
    key2[0] = 2;
    const message = fromString("Same message");
    const tag1 = poly1305Mac(message, key1);
    const tag2 = poly1305Mac(message, key2);
    expect(toHex(tag1)).not.toBe(toHex(tag2));
  });

  it("rejects invalid key length", () => {
    expect(() => poly1305Mac(new Uint8Array(10), new Uint8Array(16))).toThrow(
      "Key must be 32 bytes",
    );
  });

  it("handles message that is exactly 16 bytes", () => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 32; i++) key[i] = i;
    const msg = new Uint8Array(16);
    for (let i = 0; i < 16; i++) msg[i] = i;
    const tag = poly1305Mac(msg, key);
    expect(tag.length).toBe(16);
  });

  it("handles message that is 17 bytes (crosses chunk boundary)", () => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 32; i++) key[i] = i;
    const msg = new Uint8Array(17);
    for (let i = 0; i < 17; i++) msg[i] = i;
    const tag = poly1305Mac(msg, key);
    expect(tag.length).toBe(16);
  });
});

// ============================================================================
// AEAD Tests
// ============================================================================

describe("AEAD", () => {
  it("RFC 8439 Section 2.8.2 — Full AEAD test vector", () => {
    const key = fromHex(
      "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
    );
    const nonce = fromHex("070000004041424344454647");
    const aad = fromHex("50515253c0c1c2c3c4c5c6c7");
    const plaintext = fromString(
      "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.",
    );

    const expectedCt = fromHex(
      "d31a8d34648e60db7b86afbc53ef7ec2" +
        "a4aded51296e08fea9e2b5a736ee62d6" +
        "3dbea45e8ca9671282fafb69da92728b" +
        "1a71de0a9e060b2905d6a5b67ecd3b36" +
        "92ddbd7f2d778b8c9803aee328091b58" +
        "fab324e4fad675945585808b4831d7bc" +
        "3ff4def08e4b7a9de576d26586cec64b" +
        "6116",
    );
    const expectedTag = fromHex("1ae10b594f09e26a7e902ecbd0600691");

    const [ciphertext, tag] = aeadEncrypt(plaintext, key, nonce, aad);
    expect(toHex(ciphertext)).toBe(toHex(expectedCt));
    expect(toHex(tag)).toBe(toHex(expectedTag));
  });

  it("AEAD encrypt then decrypt round-trip", () => {
    const key = fromHex(
      "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
    );
    const nonce = fromHex("070000004041424344454647");
    const aad = fromHex("50515253c0c1c2c3c4c5c6c7");
    const plaintext = fromString(
      "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.",
    );

    const [ciphertext, tag] = aeadEncrypt(plaintext, key, nonce, aad);
    const decrypted = aeadDecrypt(ciphertext, key, nonce, aad, tag);
    expect(decrypted).toEqual(plaintext);
  });

  it("AEAD decryption fails with tampered ciphertext", () => {
    const key = fromHex(
      "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
    );
    const nonce = fromHex("070000004041424344454647");
    const aad = fromHex("50515253c0c1c2c3c4c5c6c7");
    const plaintext = fromString("Secret message");

    const [ciphertext, tag] = aeadEncrypt(plaintext, key, nonce, aad);

    // Flip a bit in the ciphertext
    const tampered = new Uint8Array(ciphertext);
    tampered[0] ^= 0x01;

    expect(() => aeadDecrypt(tampered, key, nonce, aad, tag)).toThrow(
      "Authentication failed",
    );
  });

  it("AEAD decryption fails with tampered AAD", () => {
    const key = fromHex(
      "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f",
    );
    const nonce = fromHex("070000004041424344454647");
    const aad = fromHex("50515253c0c1c2c3c4c5c6c7");
    const plaintext = fromString("Secret message");

    const [ciphertext, tag] = aeadEncrypt(plaintext, key, nonce, aad);

    const tamperedAad = new Uint8Array(aad);
    tamperedAad[0] ^= 0x01;

    expect(() => aeadDecrypt(ciphertext, key, nonce, tamperedAad, tag)).toThrow(
      "Authentication failed",
    );
  });

  it("AEAD decryption fails with wrong tag", () => {
    const key = new Uint8Array(32);
    const nonce = new Uint8Array(12);
    const aad = new Uint8Array(0);
    const plaintext = fromString("test");

    const [ciphertext] = aeadEncrypt(plaintext, key, nonce, aad);
    const wrongTag = new Uint8Array(16);

    expect(() => aeadDecrypt(ciphertext, key, nonce, aad, wrongTag)).toThrow(
      "Authentication failed",
    );
  });

  it("AEAD with empty plaintext", () => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 32; i++) key[i] = i;
    const nonce = new Uint8Array(12);
    nonce[0] = 7;
    const aad = fromString("header data");

    const [ct, tag] = aeadEncrypt(new Uint8Array(0), key, nonce, aad);
    expect(ct.length).toBe(0);
    expect(tag.length).toBe(16);

    const pt = aeadDecrypt(ct, key, nonce, aad, tag);
    expect(pt.length).toBe(0);
  });

  it("AEAD with empty AAD", () => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 32; i++) key[i] = i;
    const nonce = new Uint8Array(12);
    const aad = new Uint8Array(0);
    const plaintext = fromString("Hello, World!");

    const [ct, tag] = aeadEncrypt(plaintext, key, nonce, aad);
    const pt = aeadDecrypt(ct, key, nonce, aad, tag);
    expect(pt).toEqual(plaintext);
  });

  it("AEAD with both empty plaintext and empty AAD", () => {
    const key = new Uint8Array(32);
    const nonce = new Uint8Array(12);
    const aad = new Uint8Array(0);

    const [ct, tag] = aeadEncrypt(new Uint8Array(0), key, nonce, aad);
    expect(ct.length).toBe(0);
    const pt = aeadDecrypt(ct, key, nonce, aad, tag);
    expect(pt.length).toBe(0);
  });

  it("AEAD rejects invalid key length", () => {
    expect(() =>
      aeadEncrypt(new Uint8Array(0), new Uint8Array(16), new Uint8Array(12), new Uint8Array(0)),
    ).toThrow("Key must be 32 bytes");
  });

  it("AEAD rejects invalid nonce length", () => {
    expect(() =>
      aeadEncrypt(new Uint8Array(0), new Uint8Array(32), new Uint8Array(8), new Uint8Array(0)),
    ).toThrow("Nonce must be 12 bytes");
  });

  it("AEAD decrypt rejects invalid tag length", () => {
    expect(() =>
      aeadDecrypt(
        new Uint8Array(0),
        new Uint8Array(32),
        new Uint8Array(12),
        new Uint8Array(0),
        new Uint8Array(8),
      ),
    ).toThrow("Tag must be 16 bytes");
  });

  it("AEAD with large plaintext (multi-block)", () => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 32; i++) key[i] = i;
    const nonce = new Uint8Array(12);
    nonce[4] = 0xab;
    const aad = fromString("extra data");
    const plaintext = new Uint8Array(500);
    for (let i = 0; i < 500; i++) plaintext[i] = i % 256;

    const [ct, tag] = aeadEncrypt(plaintext, key, nonce, aad);
    expect(ct.length).toBe(500);
    const pt = aeadDecrypt(ct, key, nonce, aad, tag);
    expect(pt).toEqual(plaintext);
  });
});
