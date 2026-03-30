/**
 * # Tests for Caesar Cipher Core Operations
 *
 * These tests verify the encrypt, decrypt, and rot13 functions across a wide
 * range of inputs including edge cases like empty strings, negative shifts,
 * large shifts, and non-alphabetic characters.
 */

import { describe, it, expect } from "vitest";
import { encrypt, decrypt, rot13 } from "../src/index.js";

// ─── Encrypt ───────────────────────────────────────────────────────────────────

describe("encrypt", () => {
  it("encrypts uppercase letters with shift 3 (classic Caesar)", () => {
    // The classic example: HELLO -> KHOOR with shift 3.
    // H(7) + 3 = K(10), E(4) + 3 = H(7), L(11) + 3 = O(14), O(14) + 3 = R(17)
    expect(encrypt("HELLO", 3)).toBe("KHOOR");
  });

  it("encrypts lowercase letters with shift 3", () => {
    expect(encrypt("hello", 3)).toBe("khoor");
  });

  it("preserves case (mixed case input)", () => {
    expect(encrypt("Hello World", 3)).toBe("Khoor Zruog");
  });

  it("passes through non-alphabetic characters unchanged", () => {
    // Spaces, digits, punctuation should all pass through.
    expect(encrypt("Hello, World! 123", 3)).toBe("Khoor, Zruog! 123");
  });

  it("handles empty string", () => {
    expect(encrypt("", 5)).toBe("");
  });

  it("handles shift of 0 (identity)", () => {
    expect(encrypt("HELLO", 0)).toBe("HELLO");
  });

  it("handles shift of 26 (full rotation = identity)", () => {
    // Shifting by 26 is the same as shifting by 0.
    expect(encrypt("HELLO", 26)).toBe("HELLO");
  });

  it("handles shifts larger than 26 (wrapping)", () => {
    // Shift of 29 is equivalent to shift of 3 (29 mod 26 = 3).
    expect(encrypt("HELLO", 29)).toBe("KHOOR");
  });

  it("handles negative shifts", () => {
    // Shift of -1 moves each letter back by 1: A -> Z, B -> A, etc.
    expect(encrypt("ABC", -1)).toBe("ZAB");
  });

  it("handles large negative shifts", () => {
    // Shift of -27 is equivalent to shift of -1 (-27 mod 26 = -1).
    expect(encrypt("ABC", -27)).toBe("ZAB");
  });

  it("wraps Z to A with shift 1", () => {
    expect(encrypt("XYZ", 1)).toBe("YZA");
  });

  it("wraps z to a with shift 1", () => {
    expect(encrypt("xyz", 1)).toBe("yza");
  });

  it("handles string with only non-alpha characters", () => {
    expect(encrypt("123!@#", 5)).toBe("123!@#");
  });

  it("handles single character", () => {
    expect(encrypt("A", 1)).toBe("B");
    expect(encrypt("Z", 1)).toBe("A");
    expect(encrypt("a", 1)).toBe("b");
    expect(encrypt("z", 1)).toBe("a");
  });

  it("encrypts the full alphabet", () => {
    expect(encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 13)).toBe(
      "NOPQRSTUVWXYZABCDEFGHIJKLM",
    );
  });
});

// ─── Decrypt ───────────────────────────────────────────────────────────────────

describe("decrypt", () => {
  it("decrypts uppercase letters with shift 3", () => {
    expect(decrypt("KHOOR", 3)).toBe("HELLO");
  });

  it("decrypts lowercase letters with shift 3", () => {
    expect(decrypt("khoor", 3)).toBe("hello");
  });

  it("preserves case during decryption", () => {
    expect(decrypt("Khoor Zruog", 3)).toBe("Hello World");
  });

  it("handles non-alphabetic characters", () => {
    expect(decrypt("Khoor, Zruog! 123", 3)).toBe("Hello, World! 123");
  });

  it("handles empty string", () => {
    expect(decrypt("", 5)).toBe("");
  });

  it("handles shift of 0", () => {
    expect(decrypt("HELLO", 0)).toBe("HELLO");
  });

  it("handles negative shifts (decrypt with negative = encrypt)", () => {
    // Decrypting with shift -3 is like encrypting with shift 3.
    expect(decrypt("HELLO", -3)).toBe("KHOOR");
  });
});

// ─── Round-Trip ────────────────────────────────────────────────────────────────

describe("round-trip (encrypt then decrypt)", () => {
  it("recovers original text for various shifts", () => {
    const original = "The Quick Brown Fox Jumps Over The Lazy Dog!";

    for (const shift of [1, 5, 13, 25, 26, 0, -7, 100]) {
      const encrypted = encrypt(original, shift);
      const decrypted = decrypt(encrypted, shift);
      expect(decrypted).toBe(original);
    }
  });

  it("recovers text with all printable ASCII characters", () => {
    const original = "Hello 123 !@# $%^ &*() World";
    const encrypted = encrypt(original, 17);
    expect(decrypt(encrypted, 17)).toBe(original);
  });
});

// ─── ROT13 ─────────────────────────────────────────────────────────────────────

describe("rot13", () => {
  it("encrypts HELLO to URYYB", () => {
    expect(rot13("HELLO")).toBe("URYYB");
  });

  it("is self-inverse (applying twice returns original)", () => {
    const original = "Hello, World!";
    expect(rot13(rot13(original))).toBe(original);
  });

  it("is equivalent to encrypt with shift 13", () => {
    const text = "The Quick Brown Fox";
    expect(rot13(text)).toBe(encrypt(text, 13));
  });

  it("preserves non-alphabetic characters", () => {
    expect(rot13("Hello! 123")).toBe("Uryyb! 123");
  });

  it("handles empty string", () => {
    expect(rot13("")).toBe("");
  });

  it("transforms the full alphabet correctly", () => {
    expect(rot13("abcdefghijklmnopqrstuvwxyz")).toBe(
      "nopqrstuvwxyzabcdefghijklm",
    );
  });

  it("handles single letters at alphabet boundaries", () => {
    expect(rot13("A")).toBe("N");
    expect(rot13("N")).toBe("A");
    expect(rot13("Z")).toBe("M");
    expect(rot13("M")).toBe("Z");
  });
});
