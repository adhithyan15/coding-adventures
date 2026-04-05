/**
 * Comprehensive tests for the Scytale cipher implementation.
 */

import { describe, it, expect } from "vitest";
import { encrypt, decrypt, bruteForce } from "../src/index.js";

describe("encrypt", () => {
  it("encrypts HELLO WORLD with key=3", () => {
    expect(encrypt("HELLO WORLD", 3)).toBe("HLWLEOODL R ");
  });

  it("encrypts ABCDEF with key=2", () => {
    expect(encrypt("ABCDEF", 2)).toBe("ACEBDF");
  });

  it("encrypts ABCDEF with key=3", () => {
    expect(encrypt("ABCDEF", 3)).toBe("ADBECF");
  });

  it("encrypts ABCDEFGH with key=4", () => {
    expect(encrypt("ABCDEFGH", 4)).toBe("AEBFCGDH");
  });

  it("handles key equal to text length", () => {
    expect(encrypt("ABCD", 4)).toBe("ABCD");
  });

  it("returns empty string for empty input", () => {
    expect(encrypt("", 2)).toBe("");
  });

  it("throws for key < 2", () => {
    expect(() => encrypt("HELLO", 1)).toThrow("Key must be >= 2");
  });

  it("throws for key > text length", () => {
    expect(() => encrypt("HI", 3)).toThrow("Key must be <= text length");
  });
});

describe("decrypt", () => {
  it("decrypts HELLO WORLD with key=3", () => {
    expect(decrypt("HLWLEOODL R ", 3)).toBe("HELLO WORLD");
  });

  it("decrypts ACEBDF with key=2", () => {
    expect(decrypt("ACEBDF", 2)).toBe("ABCDEF");
  });

  it("returns empty string for empty input", () => {
    expect(decrypt("", 2)).toBe("");
  });

  it("throws for key < 2", () => {
    expect(() => decrypt("HELLO", 0)).toThrow("Key must be >= 2");
  });

  it("throws for key > text length", () => {
    expect(() => decrypt("HI", 3)).toThrow("Key must be <= text length");
  });
});

describe("round trip", () => {
  it("round-trips HELLO WORLD", () => {
    expect(decrypt(encrypt("HELLO WORLD", 3), 3)).toBe("HELLO WORLD");
  });

  it("round-trips with various keys", () => {
    const text = "The quick brown fox jumps over the lazy dog!";
    for (let key = 2; key <= Math.floor(text.length / 2); key++) {
      expect(decrypt(encrypt(text, key), key)).toBe(text);
    }
  });

  it("round-trips with punctuation", () => {
    const text = "Hello, World! 123";
    expect(decrypt(encrypt(text, 4), 4)).toBe(text);
  });
});

describe("padding", () => {
  it("adds no padding when evenly divisible", () => {
    expect(encrypt("ABCDEF", 2).length).toBe(6);
  });

  it("adds padding when not evenly divisible", () => {
    expect(encrypt("HELLO", 3).length).toBe(6);
  });

  it("strips padding on decrypt", () => {
    const ct = encrypt("HELLO", 3);
    expect(decrypt(ct, 3)).toBe("HELLO");
  });
});

describe("bruteForce", () => {
  it("finds the original text", () => {
    const ct = encrypt("HELLO WORLD", 3);
    const results = bruteForce(ct);
    const found = results.find((r) => r.key === 3);
    expect(found).toBeDefined();
    expect(found!.text).toBe("HELLO WORLD");
  });

  it("returns all keys 2 to n/2", () => {
    const results = bruteForce("ABCDEFGHIJ");
    expect(results.map((r) => r.key)).toEqual([2, 3, 4, 5]);
  });

  it("returns empty array for short text", () => {
    expect(bruteForce("AB")).toEqual([]);
    expect(bruteForce("ABC")).toEqual([]);
  });

  it("each result has key and text", () => {
    const results = bruteForce("ABCDEFGH");
    for (const r of results) {
      expect(typeof r.key).toBe("number");
      expect(typeof r.text).toBe("string");
    }
  });
});
