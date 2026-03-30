/**
 * # @coding-adventures/caesar-cipher
 *
 * A complete implementation of the Caesar cipher with both encryption/decryption
 * and cryptanalysis tools (brute force and frequency analysis).
 *
 * The Caesar cipher is one of the simplest and most widely known encryption
 * techniques. It is a type of substitution cipher in which each letter in the
 * plaintext is replaced by a letter a fixed number of positions down the alphabet.
 *
 * This package is part of the coding-adventures monorepo, a ground-up
 * implementation of the computing stack from transistors to operating systems.
 *
 * @packageDocumentation
 */

// ─── Core Cipher Operations ───────────────────────────────────────────────────
// These are the fundamental encrypt/decrypt operations. The `rot13` function
// is a convenience wrapper for the special case of shift=13.

export { encrypt, decrypt, rot13 } from "./cipher.js";

// ─── Cryptanalysis Tools ──────────────────────────────────────────────────────
// These tools break the Caesar cipher. Brute force tries all 25 shifts;
// frequency analysis uses statistics to find the most likely shift.

export {
  bruteForce,
  frequencyAnalysis,
  ENGLISH_FREQUENCIES,
  type BruteForceResult,
} from "./analysis.js";
