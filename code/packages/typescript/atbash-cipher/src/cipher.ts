/**
 * cipher.ts -- Core Atbash cipher implementation.
 *
 * The Atbash Cipher
 * =================
 *
 * The Atbash cipher works by reversing the position of each letter in the
 * alphabet. Think of it like reading the alphabet backwards:
 *
 *   Forward:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
 *   Reversed: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
 *
 * So 'A' (position 0) maps to 'Z' (position 25), 'B' (position 1) maps to
 * 'Y' (position 24), and so on.
 *
 * The Formula
 * -----------
 *
 * For any letter at position `p` (where A=0, B=1, ..., Z=25):
 *
 *   new_position = 25 - p
 *
 * For example:
 * - H is at position 7.  25 - 7  = 18, which is S.
 * - E is at position 4.  25 - 4  = 21, which is V.
 * - L is at position 11. 25 - 11 = 14, which is O.
 * - O is at position 14. 25 - 14 = 11, which is L.
 *
 * So "HELLO" becomes "SVOOL".
 *
 * Why It's Self-Inverse
 * ---------------------
 *
 * If we encrypt 'S' (position 18): 25 - 18 = 7, which is 'H'.
 * If we encrypt 'V' (position 21): 25 - 21 = 4, which is 'E'.
 *
 * Encrypting "SVOOL" gives back "HELLO". The cipher undoes itself!
 * This happens because f(f(x)) = 25 - (25 - x) = x.
 *
 * Case Preservation
 * -----------------
 *
 * We preserve the case of each letter. If the input is 'h' (lowercase),
 * we compute the Atbash of 'h' and return the result as lowercase 's'.
 * Non-alphabetic characters (digits, punctuation, spaces) pass through
 * unchanged.
 */

/** ASCII code for uppercase 'A'. */
const UPPER_A = 65;
/** ASCII code for uppercase 'Z'. */
const UPPER_Z = 90;
/** ASCII code for lowercase 'a'. */
const LOWER_A = 97;
/** ASCII code for lowercase 'z'. */
const LOWER_Z = 122;

/**
 * Apply the Atbash substitution to a single character code.
 *
 * The algorithm:
 * 1. Check if the code is for an uppercase (65-90) or lowercase (97-122) letter.
 * 2. If it's a letter, compute its position (0-25), reverse it (25 - pos),
 *    and convert back to a character code.
 * 3. If it's not a letter, return it unchanged.
 *
 * @param code - The character code (from charCodeAt)
 * @returns The Atbash-transformed character code
 */
function atbashCharCode(code: number): number {
  // Uppercase letters: A=65 through Z=90
  if (code >= UPPER_A && code <= UPPER_Z) {
    const position = code - UPPER_A; // A=0, B=1, ..., Z=25
    const newPosition = 25 - position; // Reverse: 0->25, 1->24, ..., 25->0
    return UPPER_A + newPosition;
  }

  // Lowercase letters: a=97 through z=122
  if (code >= LOWER_A && code <= LOWER_Z) {
    const position = code - LOWER_A; // a=0, b=1, ..., z=25
    const newPosition = 25 - position; // Reverse: 0->25, 1->24, ..., 25->0
    return LOWER_A + newPosition;
  }

  // Non-alphabetic characters pass through unchanged
  return code;
}

/**
 * Encrypt text using the Atbash cipher.
 *
 * Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.).
 * Non-alphabetic characters pass through unchanged. Case is preserved.
 *
 * Because the Atbash cipher is self-inverse, this function is identical
 * to {@link decrypt}. Both are provided for API clarity.
 *
 * @param text - The plaintext string to encrypt.
 * @returns The encrypted string with each letter reversed in the alphabet.
 *
 * @example
 * ```ts
 * encrypt("HELLO")             // "SVOOL"
 * encrypt("hello")             // "svool"
 * encrypt("Hello, World! 123") // "Svool, Dliow! 123"
 * ```
 */
export function encrypt(text: string): string {
  // Process each character by its code, apply Atbash, convert back.
  // We use an array of char codes for efficiency.
  let result = "";
  for (let i = 0; i < text.length; i++) {
    result += String.fromCharCode(atbashCharCode(text.charCodeAt(i)));
  }
  return result;
}

/**
 * Decrypt text using the Atbash cipher.
 *
 * Because the Atbash cipher is self-inverse (applying it twice returns
 * the original), decryption is identical to encryption. This function
 * exists for API clarity.
 *
 * @param text - The ciphertext string to decrypt.
 * @returns The decrypted (original) string.
 *
 * @example
 * ```ts
 * decrypt("SVOOL")              // "HELLO"
 * decrypt(encrypt("message"))   // "message"
 * ```
 */
export function decrypt(text: string): string {
  // Decryption IS encryption for Atbash.
  // Proof: f(f(x)) = 25 - (25 - x) = x
  return encrypt(text);
}
