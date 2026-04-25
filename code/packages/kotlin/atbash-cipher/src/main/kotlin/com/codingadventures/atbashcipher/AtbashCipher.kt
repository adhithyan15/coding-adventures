// ============================================================================
// AtbashCipher.kt — The ancient mirror substitution cipher
// ============================================================================
//
// Atbash is one of the oldest known ciphers, originating in ancient Hebrew
// writing (the name comes from the first two and last two letters of the
// Hebrew alphabet: Aleph-Tav-Beth-Shin). It was used to encode portions of
// the Hebrew Bible, including Jeremiah 25:26 and 51:41 where "Sheshach" is
// understood to mean "Babel."
//
// The principle is elegantly simple: reverse the alphabet. A becomes Z,
// B becomes Y, C becomes X, and so on. The mapping is a perfect mirror:
//
//   A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//   ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕ ↕
//   Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
//
// Self-inverse property:
// ---------------------
// Atbash has a beautiful mathematical property: applying it twice returns the
// original text. This means encrypt and decrypt are the same operation:
//
//   encrypt("HELLO") = "SVOOL"
//   encrypt("SVOOL") = "HELLO"   ← applying Atbash to the ciphertext recovers plaintext
//
// Why this works: if A is position 0 and Z is position 25, then Atbash maps
// position i to position (25 - i). Applying it again gives (25 - (25 - i)) = i.
// The operation is its own inverse.
//
// Security:
// ---------
// Atbash provides NO security by modern standards. It is a monoalphabetic
// substitution cipher with a fixed, known key. Every letter always maps to the
// same substitute, so frequency analysis immediately breaks it. Atbash is a
// useful teaching tool, not a security mechanism.
//

package com.codingadventures.atbashcipher

/**
 * The Atbash cipher — a monoalphabetic substitution cipher that mirrors the alphabet.
 *
 * Maps each letter to its mirror image: A↔Z, B↔Y, C↔X, and so on. Non-alphabetic
 * characters (digits, spaces, punctuation) pass through unchanged. Case is preserved.
 *
 * The cipher is self-inverse: `decrypt(encrypt(text)) == text` for any input,
 * and in fact `encrypt` and `decrypt` perform the exact same operation.
 *
 * ```kotlin
 * AtbashCipher.encrypt("Hello, World!")  // → "Svool, Dliow!"
 * AtbashCipher.decrypt("Svool, Dliow!")  // → "Hello, World!"
 * AtbashCipher.encrypt("ABCXYZ")         // → "ZYXCBA"
 * ```
 */
object AtbashCipher {

    // =========================================================================
    // Core mapping
    // =========================================================================
    //
    // The Atbash mapping is equivalent to reflecting each letter around the
    // midpoint of the alphabet. For any letter at position i (0-indexed from A):
    //
    //   mapped position = 25 - i
    //
    // Examples:
    //   A (position  0) → Z (position 25)
    //   B (position  1) → Y (position 24)
    //   M (position 12) → N (position 13)
    //   Z (position 25) → A (position  0)

    /**
     * Apply the Atbash mirror mapping to a single character.
     *
     * Returns the mirror image if alphabetic, or the character unchanged if not.
     */
    private fun mapChar(c: Char): Char = when {
        c in 'A'..'Z' -> ('A'.code + 'Z'.code - c.code).toChar()
        c in 'a'..'z' -> ('a'.code + 'z'.code - c.code).toChar()
        else -> c
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encrypt a string using the Atbash cipher.
     *
     * Maps every letter to its mirror image in the alphabet. Non-alphabetic
     * characters are copied unchanged. Case is preserved.
     *
     * Truth table for a few letters:
     * ```
     *   A → Z    B → Y    C → X    D → W    E → V    F → U
     *   G → T    H → S    I → R    J → Q    K → P    L → O
     *   M → N    N → M    O → L    P → K    Q → J    R → I
     *   S → H    T → G    U → F    V → E    W → D    X → C
     *   Y → B    Z → A
     * ```
     *
     * @param text the plaintext to encrypt
     * @return the ciphertext with letters mirrored
     */
    fun encrypt(text: String): String = text.map { mapChar(it) }.joinToString("")

    /**
     * Decrypt a string that was encrypted with the Atbash cipher.
     *
     * Identical to [encrypt] because Atbash is its own inverse:
     * applying the mirror mapping twice returns the original text.
     *
     * @param text the ciphertext to decrypt
     * @return the original plaintext
     */
    fun decrypt(text: String): String = encrypt(text)
}
