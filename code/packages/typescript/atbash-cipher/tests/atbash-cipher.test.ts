/**
 * Comprehensive tests for the Atbash cipher implementation.
 *
 * These tests verify that the Atbash cipher correctly reverses the alphabet
 * for both uppercase and lowercase letters, preserves non-alphabetic
 * characters, and satisfies the self-inverse property.
 */

import { describe, it, expect } from "vitest";
import { VERSION, encrypt, decrypt } from "../src/index.js";

describe("atbash-cipher", () => {
  it("has a version", () => {
    expect(VERSION).toBe("0.1.0");
  });
});

describe("encrypt", () => {
  // --- Basic Encryption ---

  it("encrypts HELLO to SVOOL", () => {
    // H(7)->S(18), E(4)->V(21), L(11)->O(14), L(11)->O(14), O(14)->L(11)
    expect(encrypt("HELLO")).toBe("SVOOL");
  });

  it("encrypts hello to svool (case preservation)", () => {
    expect(encrypt("hello")).toBe("svool");
  });

  it("encrypts mixed case with punctuation", () => {
    expect(encrypt("Hello, World! 123")).toBe("Svool, Dliow! 123");
  });

  it("reverses full uppercase alphabet", () => {
    expect(encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ")).toBe("ZYXWVUTSRQPONMLKJIHGFEDCBA");
  });

  it("reverses full lowercase alphabet", () => {
    expect(encrypt("abcdefghijklmnopqrstuvwxyz")).toBe("zyxwvutsrqponmlkjihgfedcba");
  });

  // --- Case Preservation ---

  it("preserves uppercase", () => {
    expect(encrypt("ABC")).toBe("ZYX");
  });

  it("preserves lowercase", () => {
    expect(encrypt("abc")).toBe("zyx");
  });

  it("preserves mixed case", () => {
    expect(encrypt("AbCdEf")).toBe("ZyXwVu");
  });

  // --- Non-Alpha Passthrough ---

  it("passes digits through unchanged", () => {
    expect(encrypt("12345")).toBe("12345");
  });

  it("passes punctuation through unchanged", () => {
    expect(encrypt("!@#$%")).toBe("!@#$%");
  });

  it("passes spaces through unchanged", () => {
    expect(encrypt("   ")).toBe("   ");
  });

  it("passes mixed alpha and digits correctly", () => {
    expect(encrypt("A1B2C3")).toBe("Z1Y2X3");
  });

  it("passes newlines and tabs through", () => {
    expect(encrypt("A\nB\tC")).toBe("Z\nY\tX");
  });

  // --- Edge Cases ---

  it("handles empty string", () => {
    expect(encrypt("")).toBe("");
  });

  it("handles single uppercase letters", () => {
    expect(encrypt("A")).toBe("Z");
    expect(encrypt("Z")).toBe("A");
    expect(encrypt("M")).toBe("N");
    expect(encrypt("N")).toBe("M");
  });

  it("handles single lowercase letters", () => {
    expect(encrypt("a")).toBe("z");
    expect(encrypt("z")).toBe("a");
  });

  it("handles single digit", () => {
    expect(encrypt("5")).toBe("5");
  });

  it("no letter maps to itself", () => {
    // 25 - p == p only when p == 12.5, which is not an integer
    for (let i = 0; i < 26; i++) {
      const upper = String.fromCharCode(65 + i);
      expect(encrypt(upper)).not.toBe(upper);

      const lower = String.fromCharCode(97 + i);
      expect(encrypt(lower)).not.toBe(lower);
    }
  });
});

describe("self-inverse property", () => {
  // The most important mathematical property: encrypt(encrypt(x)) == x

  it("is self-inverse for HELLO", () => {
    expect(encrypt(encrypt("HELLO"))).toBe("HELLO");
  });

  it("is self-inverse for lowercase", () => {
    expect(encrypt(encrypt("hello"))).toBe("hello");
  });

  it("is self-inverse for mixed input", () => {
    expect(encrypt(encrypt("Hello, World! 123"))).toBe("Hello, World! 123");
  });

  it("is self-inverse for full alphabet", () => {
    const alpha = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    expect(encrypt(encrypt(alpha))).toBe(alpha);
  });

  it("is self-inverse for empty string", () => {
    expect(encrypt(encrypt(""))).toBe("");
  });

  it("is self-inverse for long text", () => {
    const text = "The quick brown fox jumps over the lazy dog! 42";
    expect(encrypt(encrypt(text))).toBe(text);
  });
});

describe("decrypt", () => {
  it("decrypts SVOOL to HELLO", () => {
    expect(decrypt("SVOOL")).toBe("HELLO");
  });

  it("decrypts svool to hello", () => {
    expect(decrypt("svool")).toBe("hello");
  });

  it("is the inverse of encrypt", () => {
    const texts = ["HELLO", "hello", "Hello, World! 123", "", "42"];
    for (const text of texts) {
      expect(decrypt(encrypt(text))).toBe(text);
    }
  });

  it("produces same output as encrypt (they are identical)", () => {
    const texts = ["HELLO", "svool", "Test!", ""];
    for (const text of texts) {
      expect(encrypt(text)).toBe(decrypt(text));
    }
  });
});
