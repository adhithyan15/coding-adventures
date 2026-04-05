/**
 * cipher.ts -- Core Vigenere cipher encrypt/decrypt implementation.
 *
 * The Vigenere Cipher
 * ===================
 *
 * The Vigenere cipher is a *polyalphabetic substitution* cipher invented by
 * Giovan Battista Bellaso in 1553 (commonly misattributed to Blaise de
 * Vigenere). It was considered unbreakable for 300 years -- "le chiffre
 * indechiffrable" -- until Friedrich Kasiski published a general attack
 * in 1863.
 *
 * How It Works
 * ------------
 *
 * Unlike the Caesar cipher (single shift for all letters), the Vigenere
 * cipher uses a *keyword* to apply a *different* Caesar shift at each
 * position. The keyword repeats cyclically across the plaintext.
 *
 * Example: encrypt("ATTACKATDAWN", "LEMON")
 *
 *     Plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
 *     Key cycle:  L  E  M  O  N  L  E  M  O  N  L  E
 *     Shift:      11 4  12 14 13 11 4  12 14 13 11 4
 *     Ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
 *
 * Each plaintext letter is shifted *forward* by the corresponding key
 * letter's position (A=0, B=1, ..., Z=25). Decryption shifts *backward*.
 *
 * Character Handling Rules
 * ------------------------
 *
 * - Uppercase letters stay uppercase, lowercase stay lowercase.
 * - Non-alphabetic characters (spaces, punctuation, digits) pass through
 *   unchanged and do NOT advance the key position.
 * - The key must be non-empty and contain only alphabetic characters.
 */

/**
 * Validate that a key is non-empty and contains only alphabetic characters.
 *
 * @param key - The encryption/decryption key to validate.
 * @throws {Error} If the key is empty or contains non-alpha characters.
 */
function validateKey(key: string): void {
  if (key.length === 0) {
    throw new Error("Key must not be empty");
  }
  if (!/^[a-zA-Z]+$/.test(key)) {
    throw new Error("Key must contain only alphabetic characters");
  }
}

/**
 * Encrypt plaintext using the Vigenere cipher.
 *
 * The algorithm walks through the plaintext character by character:
 * - If the character is a letter, shift it forward by the current key
 *   letter's value (A/a=0, B/b=1, ..., Z/z=25), then advance the key
 *   position.
 * - If the character is not a letter, emit it unchanged and do NOT
 *   advance the key position.
 *
 * @param plaintext - The text to encrypt.
 * @param key - The alphabetic keyword (case-insensitive).
 * @returns The encrypted ciphertext.
 */
export function encrypt(plaintext: string, key: string): string {
  validateKey(key);

  const upperKey = key.toUpperCase();
  let keyIndex = 0;
  let result = "";

  for (const ch of plaintext) {
    if (isUpperAlpha(ch)) {
      // Shift uppercase letter forward by key amount
      const shift = upperKey.charCodeAt(keyIndex % upperKey.length) - 65;
      const shifted = ((ch.charCodeAt(0) - 65 + shift) % 26) + 65;
      result += String.fromCharCode(shifted);
      keyIndex++;
    } else if (isLowerAlpha(ch)) {
      // Shift lowercase letter forward by key amount (preserve case)
      const shift = upperKey.charCodeAt(keyIndex % upperKey.length) - 65;
      const shifted = ((ch.charCodeAt(0) - 97 + shift) % 26) + 97;
      result += String.fromCharCode(shifted);
      keyIndex++;
    } else {
      // Non-alpha passes through, key does NOT advance
      result += ch;
    }
  }

  return result;
}

/**
 * Decrypt ciphertext using the Vigenere cipher.
 *
 * Identical to encrypt but shifts *backward* instead of forward.
 * Since (encrypt . decrypt) must be the identity, we subtract the
 * key shift and add 26 to handle negative modular arithmetic.
 *
 * @param ciphertext - The text to decrypt.
 * @param key - The alphabetic keyword (case-insensitive).
 * @returns The decrypted plaintext.
 */
export function decrypt(ciphertext: string, key: string): string {
  validateKey(key);

  const upperKey = key.toUpperCase();
  let keyIndex = 0;
  let result = "";

  for (const ch of ciphertext) {
    if (isUpperAlpha(ch)) {
      // Shift uppercase letter backward by key amount
      const shift = upperKey.charCodeAt(keyIndex % upperKey.length) - 65;
      const shifted = ((ch.charCodeAt(0) - 65 - shift + 26) % 26) + 65;
      result += String.fromCharCode(shifted);
      keyIndex++;
    } else if (isLowerAlpha(ch)) {
      // Shift lowercase letter backward by key amount (preserve case)
      const shift = upperKey.charCodeAt(keyIndex % upperKey.length) - 65;
      const shifted = ((ch.charCodeAt(0) - 97 - shift + 26) % 26) + 97;
      result += String.fromCharCode(shifted);
      keyIndex++;
    } else {
      result += ch;
    }
  }

  return result;
}

/** Check if a character is an uppercase ASCII letter (A-Z). */
function isUpperAlpha(ch: string): boolean {
  const code = ch.charCodeAt(0);
  return code >= 65 && code <= 90;
}

/** Check if a character is a lowercase ASCII letter (a-z). */
function isLowerAlpha(ch: string): boolean {
  const code = ch.charCodeAt(0);
  return code >= 97 && code <= 122;
}
