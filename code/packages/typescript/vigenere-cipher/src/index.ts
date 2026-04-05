/**
 * @module @coding-adventures/vigenere-cipher
 *
 * Vigenere cipher -- polyalphabetic substitution cipher with cryptanalysis.
 *
 * The Vigenere cipher (1553) uses a repeating keyword to apply different
 * Caesar shifts at each position. It was considered unbreakable for 300
 * years until Kasiski and Friedman developed statistical attacks using
 * the Index of Coincidence and chi-squared frequency analysis.
 *
 * This package provides:
 * - `encrypt` / `decrypt` -- core Vigenere operations
 * - `findKeyLength` -- IC-based key length estimation
 * - `findKey` -- chi-squared key recovery
 * - `breakCipher` -- fully automatic cryptanalysis
 */

export { encrypt, decrypt } from "./cipher.js";
export { findKeyLength, findKey, breakCipher } from "./analysis.js";
export type { BreakResult } from "./analysis.js";
