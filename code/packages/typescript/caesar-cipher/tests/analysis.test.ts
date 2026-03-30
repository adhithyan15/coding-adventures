/**
 * # Tests for Caesar Cipher Cryptanalysis Tools
 *
 * These tests verify the brute-force and frequency analysis functions.
 */

import { describe, it, expect } from "vitest";
import {
  bruteForce,
  frequencyAnalysis,
  ENGLISH_FREQUENCIES,
  encrypt,
  type BruteForceResult,
} from "../src/index.js";

// ─── English Frequencies ───────────────────────────────────────────────────────

describe("ENGLISH_FREQUENCIES", () => {
  it("contains exactly 26 entries (one per letter)", () => {
    expect(Object.keys(ENGLISH_FREQUENCIES)).toHaveLength(26);
  });

  it("has all lowercase letters a-z", () => {
    for (let i = 0; i < 26; i++) {
      const letter = String.fromCharCode(97 + i);
      expect(ENGLISH_FREQUENCIES).toHaveProperty(letter);
    }
  });

  it("has frequencies that sum to approximately 1.0", () => {
    const sum = Object.values(ENGLISH_FREQUENCIES).reduce((a, b) => a + b, 0);
    // The sum should be very close to 1.0 (within rounding tolerance).
    expect(sum).toBeGreaterThan(0.99);
    expect(sum).toBeLessThan(1.01);
  });

  it("has 'e' as the most frequent letter", () => {
    const maxFreq = Math.max(...Object.values(ENGLISH_FREQUENCIES));
    expect(ENGLISH_FREQUENCIES["e"]).toBe(maxFreq);
  });

  it("has 'z' as the least frequent letter", () => {
    const minFreq = Math.min(...Object.values(ENGLISH_FREQUENCIES));
    expect(ENGLISH_FREQUENCIES["z"]).toBe(minFreq);
  });

  it("all frequencies are positive", () => {
    for (const freq of Object.values(ENGLISH_FREQUENCIES)) {
      expect(freq).toBeGreaterThan(0);
    }
  });
});

// ─── Brute Force ───────────────────────────────────────────────────────────────

describe("bruteForce", () => {
  it("returns exactly 25 results (shifts 1-25)", () => {
    const results = bruteForce("KHOOR");
    expect(results).toHaveLength(25);
  });

  it("each result has the correct shift value", () => {
    const results = bruteForce("KHOOR");
    for (let i = 0; i < 25; i++) {
      expect(results[i].shift).toBe(i + 1);
    }
  });

  it("contains the correct plaintext among results", () => {
    // "KHOOR" was encrypted from "HELLO" with shift 3.
    // So decrypting with shift 3 should give "HELLO".
    const results = bruteForce("KHOOR");
    const shift3Result = results.find((r) => r.shift === 3);
    expect(shift3Result).toBeDefined();
    expect(shift3Result!.plaintext).toBe("HELLO");
  });

  it("works with lowercase text", () => {
    const results = bruteForce("khoor");
    const shift3Result = results.find((r) => r.shift === 3);
    expect(shift3Result!.plaintext).toBe("hello");
  });

  it("works with mixed case text", () => {
    const results = bruteForce("Khoor Zruog");
    const shift3Result = results.find((r) => r.shift === 3);
    expect(shift3Result!.plaintext).toBe("Hello World");
  });

  it("handles empty string", () => {
    const results = bruteForce("");
    expect(results).toHaveLength(25);
    // Every result should be an empty string.
    for (const result of results) {
      expect(result.plaintext).toBe("");
    }
  });

  it("handles string with no alphabetic characters", () => {
    const results = bruteForce("123 !@#");
    expect(results).toHaveLength(25);
    // Non-alpha chars are unchanged regardless of shift.
    for (const result of results) {
      expect(result.plaintext).toBe("123 !@#");
    }
  });

  it("results are ordered by shift ascending", () => {
    const results = bruteForce("TEST");
    for (let i = 0; i < results.length - 1; i++) {
      expect(results[i].shift).toBeLessThan(results[i + 1].shift);
    }
  });

  it("every result is a valid decryption (round-trip)", () => {
    const original = "HELLO";
    for (let shift = 1; shift <= 25; shift++) {
      const ciphertext = encrypt(original, shift);
      const results = bruteForce(ciphertext);
      const match = results.find((r) => r.shift === shift);
      expect(match).toBeDefined();
      expect(match!.plaintext).toBe(original);
    }
  });
});

// ─── Frequency Analysis ────────────────────────────────────────────────────────

describe("frequencyAnalysis", () => {
  // A sufficiently long English passage for reliable frequency analysis.
  // This is a well-known pangram repeated to give enough statistical data.
  const englishText =
    "The quick brown fox jumps over the lazy dog. " +
    "Pack my box with five dozen liquor jugs. " +
    "How vexingly quick daft zebras jump. " +
    "The five boxing wizards jump quickly. " +
    "Sphinx of black quartz judge my vow. " +
    "Two driven jocks help fax my big quiz. " +
    "The jay pig fox and zebra quickly moved the vast herd of bison westward.";

  it("correctly identifies shift 3 on English text", () => {
    const ciphertext = encrypt(englishText, 3);
    const result = frequencyAnalysis(ciphertext);
    expect(result.shift).toBe(3);
    expect(result.plaintext).toBe(englishText);
  });

  it("correctly identifies shift 13 (ROT13) on English text", () => {
    const ciphertext = encrypt(englishText, 13);
    const result = frequencyAnalysis(ciphertext);
    expect(result.shift).toBe(13);
    expect(result.plaintext).toBe(englishText);
  });

  it("correctly identifies shift 7 on English text", () => {
    const ciphertext = encrypt(englishText, 7);
    const result = frequencyAnalysis(ciphertext);
    expect(result.shift).toBe(7);
    expect(result.plaintext).toBe(englishText);
  });

  it("correctly identifies shift 20 on English text", () => {
    const ciphertext = encrypt(englishText, 20);
    const result = frequencyAnalysis(ciphertext);
    expect(result.shift).toBe(20);
    expect(result.plaintext).toBe(englishText);
  });

  it("returns shift 0 for unencrypted English text", () => {
    const result = frequencyAnalysis(englishText);
    expect(result.shift).toBe(0);
    expect(result.plaintext).toBe(englishText);
  });

  it("handles all-uppercase text", () => {
    const upper = englishText.toUpperCase();
    const ciphertext = encrypt(upper, 5);
    const result = frequencyAnalysis(ciphertext);
    expect(result.shift).toBe(5);
    expect(result.plaintext).toBe(upper);
  });

  it("returns a result object with shift and plaintext", () => {
    const result = frequencyAnalysis("KHOOR ZRUOG");
    expect(result).toHaveProperty("shift");
    expect(result).toHaveProperty("plaintext");
    expect(typeof result.shift).toBe("number");
    expect(typeof result.plaintext).toBe("string");
  });

  it("works on shorter but recognizable English text", () => {
    // "HELLO WORLD" encrypted with shift 3 -> "KHOOR ZRUOG"
    // This is short but the frequency distribution may still work.
    const ciphertext = encrypt("HELLO WORLD THIS IS A TEST OF THE CIPHER", 3);
    const result = frequencyAnalysis(ciphertext);
    expect(result.shift).toBe(3);
  });

  it("handles empty string gracefully", () => {
    const result = frequencyAnalysis("");
    expect(result).toHaveProperty("shift");
    expect(result).toHaveProperty("plaintext");
    expect(result.plaintext).toBe("");
  });

  it("handles non-alphabetic input gracefully", () => {
    const result = frequencyAnalysis("123 456 !@#");
    expect(result).toHaveProperty("shift");
    // With no letters, the plaintext should just be the input unchanged
    // (all characters pass through decryption).
    expect(result.plaintext).toBe("123 456 !@#");
  });
});
