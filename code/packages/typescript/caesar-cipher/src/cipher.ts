/**
 * # Caesar Cipher -- Core Encryption and Decryption
 *
 * The Caesar cipher is one of the oldest known encryption techniques, named after
 * Julius Caesar who reportedly used it to communicate with his generals. The idea
 * is beautifully simple: shift every letter in the message by a fixed number of
 * positions through the alphabet.
 *
 * ## How It Works
 *
 * Imagine the alphabet laid out in a circle, like a clock face with 26 positions
 * instead of 12:
 *
 * ```
 *        A  B  C  D  E  F  G  H  I  J  K  L  M
 *        0  1  2  3  4  5  6  7  8  9  10 11 12
 *
 *        N  O  P  Q  R  S  T  U  V  W  X  Y  Z
 *        13 14 15 16 17 18 19 20 21 22 23 24 25
 * ```
 *
 * To encrypt with a shift of 3 (Caesar's own choice):
 *   - A (position 0) becomes D (position 3)
 *   - B (position 1) becomes E (position 4)
 *   - X (position 23) wraps around to A (position 0)
 *   - Y (position 24) wraps around to B (position 1)
 *   - Z (position 25) wraps around to C (position 2)
 *
 * ## The Math Behind It
 *
 * For encryption: `cipherChar = (plainChar + shift) mod 26`
 * For decryption: `plainChar  = (cipherChar - shift) mod 26`
 *
 * The modulo operation handles the wrap-around. When we go past Z, we circle
 * back to A. This is called "modular arithmetic" and it shows up everywhere
 * in cryptography.
 *
 * ## Worked Example
 *
 * Encrypting "HELLO" with shift 3:
 *
 * | Letter | Position | + Shift | mod 26 | Result |
 * |--------|----------|---------|--------|--------|
 * | H      | 7        | 10      | 10     | K      |
 * | E      | 4        | 7       | 7      | H      |
 * | L      | 11       | 14      | 14     | O      |
 * | L      | 11       | 14      | 14     | O      |
 * | O      | 14       | 17      | 17     | R      |
 *
 * So "HELLO" becomes "KHOOR".
 *
 * ## Character Codes in JavaScript/TypeScript
 *
 * JavaScript represents characters internally using Unicode code points.
 * We use `String.prototype.charCodeAt()` to get the numeric code and
 * `String.fromCharCode()` to convert back. The relevant ranges are:
 *
 * - Uppercase A-Z: codes 65-90
 * - Lowercase a-z: codes 97-122
 *
 * To convert a character to a 0-25 position, we subtract the base code:
 *   - 'A'.charCodeAt(0) = 65, so position = charCode - 65
 *   - 'a'.charCodeAt(0) = 97, so position = charCode - 97
 *
 * @module cipher
 */

// ─── Character Code Constants ──────────────────────────────────────────────────
//
// These constants anchor our alphabet arithmetic. Rather than scattering magic
// numbers throughout the code, we name them clearly:

/** Unicode code point for uppercase 'A' (65). */
const UPPER_A = 65;

/** Unicode code point for uppercase 'Z' (90). */
const UPPER_Z = 90;

/** Unicode code point for lowercase 'a' (97). */
const LOWER_A = 97;

/** Unicode code point for lowercase 'z' (122). */
const LOWER_Z = 122;

/** The number of letters in the English alphabet. */
const ALPHABET_SIZE = 26;

// ─── Helper: Shift a Single Character ──────────────────────────────────────────
//
// This is the core building block. Given a character code and a shift amount,
// it returns the shifted character code if the character is a letter, or the
// original code if it is not.
//
// The key insight: we normalize the character to a 0-25 range, apply the shift
// with modular arithmetic, then convert back. Non-alphabetic characters (spaces,
// punctuation, digits) pass through unchanged -- the Caesar cipher historically
// only operated on letters.

/**
 * Shifts a single character by `shift` positions in the alphabet.
 *
 * If the character is not a letter (A-Z or a-z), it is returned unchanged.
 * The shift wraps around using modulo 26, so shifting 'Z' by 1 gives 'A'.
 *
 * @param charCode - The Unicode code point of the character to shift.
 * @param shift - The number of positions to shift (can be negative).
 * @returns The shifted character code, or the original if not a letter.
 *
 * @example
 * ```ts
 * // 'A' (65) shifted by 3 gives 'D' (68)
 * shiftChar(65, 3); // => 68
 * ```
 */
function shiftChar(charCode: number, shift: number): number {
  // ---- Uppercase letters ----
  // Check if the character falls in the A-Z range (codes 65-90).
  if (charCode >= UPPER_A && charCode <= UPPER_Z) {
    // Step 1: Normalize to 0-25 by subtracting the base code for 'A'.
    //         'A' -> 0, 'B' -> 1, ..., 'Z' -> 25
    const position = charCode - UPPER_A;

    // Step 2: Apply the shift with modular arithmetic.
    //         The `((x % n) + n) % n` pattern handles negative shifts correctly.
    //         In JavaScript, `-1 % 26` gives `-1` (not `25`), so we add 26
    //         before taking the final modulo to ensure a positive result.
    const shifted = ((position + shift) % ALPHABET_SIZE + ALPHABET_SIZE) % ALPHABET_SIZE;

    // Step 3: Convert back to a character code by adding the base.
    return UPPER_A + shifted;
  }

  // ---- Lowercase letters ----
  // Same logic, but anchored at 'a' (code 97) instead of 'A' (code 65).
  if (charCode >= LOWER_A && charCode <= LOWER_Z) {
    const position = charCode - LOWER_A;
    const shifted = ((position + shift) % ALPHABET_SIZE + ALPHABET_SIZE) % ALPHABET_SIZE;
    return LOWER_A + shifted;
  }

  // ---- Non-alphabetic characters ----
  // Digits, spaces, punctuation, emoji, etc. pass through untouched.
  return charCode;
}

// ─── Encrypt ───────────────────────────────────────────────────────────────────
//
// Encryption shifts each letter forward by `shift` positions. This is the
// "encoding" direction of the Caesar cipher.

/**
 * Encrypts a string using the Caesar cipher with the given shift.
 *
 * Each letter is shifted forward by `shift` positions in the alphabet.
 * Non-alphabetic characters (spaces, digits, punctuation) are preserved
 * exactly as they appear. Letter case is also preserved: uppercase letters
 * remain uppercase and lowercase letters remain lowercase.
 *
 * The shift wraps around modulo 26, so a shift of 27 is equivalent to a
 * shift of 1, and a shift of -1 is equivalent to a shift of 25.
 *
 * @param text - The plaintext string to encrypt.
 * @param shift - The number of positions to shift each letter forward.
 *   Can be any integer (positive, negative, or zero).
 * @returns The encrypted ciphertext string.
 *
 * @example
 * ```ts
 * encrypt("HELLO", 3);    // => "KHOOR"
 * encrypt("hello", 3);    // => "khoor"
 * encrypt("Hello!", 3);   // => "Khoor!"
 * encrypt("ABC", 26);     // => "ABC" (full rotation)
 * encrypt("ABC", -1);     // => "ZAB"
 * ```
 */
export function encrypt(text: string, shift: number): string {
  // We process the string character by character. For each character:
  // 1. Get its Unicode code point
  // 2. Shift it (if it's a letter)
  // 3. Convert back to a character
  //
  // We build up the result using an array of character codes for efficiency,
  // then convert the whole array to a string at the end. This avoids repeated
  // string concatenation, which creates a new string object each time.

  const result: number[] = new Array(text.length);

  for (let i = 0; i < text.length; i++) {
    result[i] = shiftChar(text.charCodeAt(i), shift);
  }

  return String.fromCharCode(...result);
}

// ─── Decrypt ───────────────────────────────────────────────────────────────────
//
// Decryption is simply encryption in the reverse direction. If we encrypted
// by shifting forward 3 positions, we decrypt by shifting backward 3 positions.
// Mathematically: decrypt(text, shift) = encrypt(text, -shift).
//
// This elegant symmetry is a hallmark of substitution ciphers.

/**
 * Decrypts a Caesar cipher encrypted string using the given shift.
 *
 * This reverses the encryption by shifting each letter backward by `shift`
 * positions. It is mathematically equivalent to `encrypt(text, -shift)`.
 *
 * @param text - The ciphertext string to decrypt.
 * @param shift - The shift that was used during encryption.
 * @returns The decrypted plaintext string.
 *
 * @example
 * ```ts
 * decrypt("KHOOR", 3);   // => "HELLO"
 * decrypt("khoor", 3);   // => "hello"
 * decrypt("Khoor!", 3);  // => "Hello!"
 * ```
 */
export function decrypt(text: string, shift: number): string {
  // Decryption is encryption with the negated shift. By reusing `encrypt`,
  // we avoid duplicating logic and ensure both directions stay in sync.
  return encrypt(text, -shift);
}

// ─── ROT13 ─────────────────────────────────────────────────────────────────────
//
// ROT13 is a special case of the Caesar cipher where the shift is exactly 13.
// Since 13 is half of 26 (the alphabet size), ROT13 has a remarkable property:
// applying it twice returns the original text. In other words, ROT13 is its
// own inverse:
//
//   rot13(rot13("HELLO")) === "HELLO"
//
// This self-inverse property made ROT13 popular on early internet forums (like
// Usenet) for hiding spoilers and punchlines. You could "decrypt" by simply
// applying the same transformation again.
//
// Truth table for the self-inverse property:
//
//   A <-> N    B <-> O    C <-> P    D <-> Q    E <-> R    F <-> S    G <-> T
//   H <-> U    I <-> V    J <-> W    K <-> X    L <-> Y    M <-> Z

/**
 * Applies ROT13 encoding to a string.
 *
 * ROT13 is a Caesar cipher with a shift of 13. Because 13 is exactly half
 * the alphabet size (26), ROT13 is its own inverse: applying it twice returns
 * the original text.
 *
 * @param text - The string to encode/decode with ROT13.
 * @returns The ROT13-transformed string.
 *
 * @example
 * ```ts
 * rot13("HELLO");          // => "URYYB"
 * rot13("URYYB");          // => "HELLO"
 * rot13(rot13("SECRET"));  // => "SECRET" (self-inverse!)
 * ```
 */
export function rot13(text: string): string {
  return encrypt(text, 13);
}
