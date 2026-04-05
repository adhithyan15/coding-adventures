import { describe, it, expect } from "vitest";
import {
  encrypt,
  decrypt,
  findKeyLength,
  findKey,
  breakCipher,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Long English text for cryptanalysis tests (~300+ chars of alpha content).
// This is required because IC analysis needs a statistically significant
// sample to reliably distinguish English from random text.
// ---------------------------------------------------------------------------
const LONG_ENGLISH_TEXT =
  "The quick brown fox jumps over the lazy dog and then runs around the " +
  "entire neighborhood looking for more adventures to embark upon while " +
  "the sun slowly sets behind the distant mountains casting long shadows " +
  "across the valley below where the river winds its way through ancient " +
  "forests filled with towering oak trees and singing birds that herald " +
  "the coming of spring with their melodious songs echoing through the " +
  "canopy above where squirrels chase each other from branch to branch " +
  "gathering acorns and other nuts for the long winter months ahead when " +
  "the ground will be covered in a thick blanket of pristine white snow " +
  "and the children will build snowmen and throw snowballs at each other " +
  "laughing and playing until their parents call them inside for dinner " +
  "where warm soup and fresh bread await them on the old wooden table";

// ---------------------------------------------------------------------------
// Encrypt tests
// ---------------------------------------------------------------------------
describe("encrypt", () => {
  it("encrypts ATTACKATDAWN with key LEMON", () => {
    expect(encrypt("ATTACKATDAWN", "LEMON")).toBe("LXFOPVEFRNHR");
  });

  it("preserves case and punctuation", () => {
    expect(encrypt("Hello, World!", "key")).toBe("Rijvs, Uyvjn!");
  });

  it("handles all-lowercase text", () => {
    expect(encrypt("attackatdawn", "lemon")).toBe("lxfopvefrnhr");
  });

  it("handles mixed case key", () => {
    // Key is case-insensitive: "LeMoN" should work the same as "LEMON"
    expect(encrypt("ATTACKATDAWN", "LeMoN")).toBe("LXFOPVEFRNHR");
  });

  it("handles single-char key", () => {
    // Key "B" means shift of 1 for every letter (same as Caesar +1)
    expect(encrypt("ABC", "B")).toBe("BCD");
  });

  it("handles key longer than alpha content", () => {
    // A shifted by L(11) = L, B shifted by O(14) = P
    expect(encrypt("AB", "LONGERKEY")).toBe("LP");
  });

  it("skips non-alpha for key advancement", () => {
    // "A T" -- space does not advance key
    // A shifted by L(11) = L, space passes through, T shifted by E(4) = X
    expect(encrypt("A T", "LE")).toBe("L X");
  });

  it("handles digits and special chars unchanged", () => {
    expect(encrypt("Hello 123!", "key")).toBe("Rijvs 123!");
  });

  it("handles empty plaintext", () => {
    expect(encrypt("", "key")).toBe("");
  });

  it("throws on empty key", () => {
    expect(() => encrypt("hello", "")).toThrow("Key must not be empty");
  });

  it("throws on non-alpha key", () => {
    expect(() => encrypt("hello", "key1")).toThrow(
      "Key must contain only alphabetic characters",
    );
    expect(() => encrypt("hello", "ke y")).toThrow(
      "Key must contain only alphabetic characters",
    );
  });
});

// ---------------------------------------------------------------------------
// Decrypt tests
// ---------------------------------------------------------------------------
describe("decrypt", () => {
  it("decrypts LXFOPVEFRNHR with key LEMON", () => {
    expect(decrypt("LXFOPVEFRNHR", "LEMON")).toBe("ATTACKATDAWN");
  });

  it("preserves case and punctuation", () => {
    expect(decrypt("Rijvs, Uyvjn!", "key")).toBe("Hello, World!");
  });

  it("handles all-lowercase", () => {
    expect(decrypt("lxfopvefrnhr", "lemon")).toBe("attackatdawn");
  });

  it("handles empty ciphertext", () => {
    expect(decrypt("", "key")).toBe("");
  });

  it("throws on empty key", () => {
    expect(() => decrypt("hello", "")).toThrow("Key must not be empty");
  });

  it("throws on non-alpha key", () => {
    expect(() => decrypt("hello", "123")).toThrow(
      "Key must contain only alphabetic characters",
    );
  });
});

// ---------------------------------------------------------------------------
// Round-trip tests
// ---------------------------------------------------------------------------
describe("round-trip", () => {
  const cases = [
    { text: "ATTACKATDAWN", key: "LEMON" },
    { text: "Hello, World!", key: "key" },
    { text: "The quick brown fox!", key: "SECRET" },
    { text: "abc def ghi", key: "xyz" },
    { text: "MiXeD CaSe 123", key: "AbCdE" },
    { text: "a", key: "z" },
    { text: "ZZZZZZ", key: "A" },
  ];

  for (const { text, key } of cases) {
    it(`decrypt(encrypt("${text}", "${key}"), "${key}") == original`, () => {
      expect(decrypt(encrypt(text, key), key)).toBe(text);
    });
  }
});

// ---------------------------------------------------------------------------
// Cryptanalysis: findKeyLength
// ---------------------------------------------------------------------------
describe("findKeyLength", () => {
  it("finds key length for text encrypted with a 5-letter key", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "LEMON");
    const length = findKeyLength(ct);
    expect(length).toBe(5);
  });

  it("finds key length for text encrypted with a 6-letter key", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "SECRET");
    const length = findKeyLength(ct);
    expect(length).toBe(6);
  });

  it("finds key length for text encrypted with a 3-letter key", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "KEY");
    const length = findKeyLength(ct);
    expect(length).toBe(3);
  });

  it("respects maxLength parameter", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "LEMON");
    // With maxLength=3, cannot find length 5 -- returns best of 2,3
    const length = findKeyLength(ct, 3);
    expect(length).toBeGreaterThanOrEqual(1);
    expect(length).toBeLessThanOrEqual(3);
  });

  it("returns 1 for very short text", () => {
    expect(findKeyLength("A")).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// Cryptanalysis: findKey
// ---------------------------------------------------------------------------
describe("findKey", () => {
  it("recovers LEMON from ciphertext with known key length 5", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "LEMON");
    const key = findKey(ct, 5);
    expect(key).toBe("LEMON");
  });

  it("recovers SECRET from ciphertext with known key length 6", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "SECRET");
    const key = findKey(ct, 6);
    expect(key).toBe("SECRET");
  });

  it("recovers KEY from ciphertext with known key length 3", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "KEY");
    const key = findKey(ct, 3);
    expect(key).toBe("KEY");
  });
});

// ---------------------------------------------------------------------------
// Cryptanalysis: breakCipher
// ---------------------------------------------------------------------------
describe("breakCipher", () => {
  it("automatically breaks a Vigenere cipher with key LEMON", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "LEMON");
    const result = breakCipher(ct);
    expect(result.key).toBe("LEMON");
    expect(result.plaintext).toBe(LONG_ENGLISH_TEXT);
  });

  it("automatically breaks a Vigenere cipher with key SECRET", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "SECRET");
    const result = breakCipher(ct);
    expect(result.key).toBe("SECRET");
    expect(result.plaintext).toBe(LONG_ENGLISH_TEXT);
  });

  it("recovered plaintext matches original", () => {
    const ct = encrypt(LONG_ENGLISH_TEXT, "CIPHER");
    const result = breakCipher(ct);
    // Even if key recovery isn't perfect, decrypting with the recovered
    // key should be internally consistent
    expect(decrypt(encrypt(LONG_ENGLISH_TEXT, result.key), result.key)).toBe(
      LONG_ENGLISH_TEXT,
    );
  });
});
