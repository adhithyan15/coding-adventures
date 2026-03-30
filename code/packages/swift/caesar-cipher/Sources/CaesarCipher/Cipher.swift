// Cipher.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Caesar Cipher
// ============================================================================
//
// The Caesar cipher is the oldest known substitution cipher, named after
// Julius Caesar who reportedly used it to communicate with his generals.
// Suetonius describes how Caesar shifted each letter in his messages by
// three positions in the alphabet: A became D, B became E, and so on.
//
// ============================================================================
// How It Works
// ============================================================================
//
// A substitution cipher replaces each letter in the plaintext with a
// different letter. The Caesar cipher is the simplest form: every letter
// is shifted by the same fixed number of positions.
//
// Given a shift of 3:
//
//   Plain alphabet:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//   Cipher alphabet: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
//
// So the word "HELLO" encrypts to "KHOOR":
//
//   H -> K  (H is position 7, shift +3 = position 10 = K)
//   E -> H  (E is position 4, shift +3 = position  7 = H)
//   L -> O  (L is position 11, shift +3 = position 14 = O)
//   L -> O  (same letter, same shift, same result)
//   O -> R  (O is position 14, shift +3 = position 17 = R)
//
// ============================================================================
// The Math: Modular Arithmetic
// ============================================================================
//
// We number the letters A=0, B=1, ..., Z=25. To encrypt a letter with
// position `p` using shift `s`:
//
//   encrypted_position = (p + s) mod 26
//
// To decrypt:
//
//   decrypted_position = (p - s) mod 26
//
// The "mod 26" handles wrapping: if we shift 'X' (position 23) by 3,
// we get (23 + 3) mod 26 = 0 = 'A'. The alphabet wraps around.
//
// One subtlety: in many programming languages (including Swift), the `%`
// operator can return negative values for negative operands. For example,
// (-1) % 26 gives -1 in Swift, not 25. We need a true mathematical
// modulo that always returns a non-negative result. We handle this by
// adding 26 before taking the remainder:
//
//   ((p + s) % 26 + 26) % 26
//
// This guarantees a result in the range [0, 25] regardless of whether
// `p + s` is negative.
//
// ============================================================================
// Non-Alphabetic Characters
// ============================================================================
//
// The traditional Caesar cipher only operates on letters. Digits, spaces,
// punctuation, and other characters pass through unchanged. This is both
// historically accurate and practical — it preserves the structure of
// the message while hiding the words.
//
// ============================================================================
// Case Preservation
// ============================================================================
//
// We preserve the case of each letter. If the input letter is uppercase,
// the output letter is uppercase. If lowercase, the output is lowercase.
// This is achieved by detecting the case, converting to a 0-based offset
// from either 'A' or 'a', applying the shift, and converting back.
//
// ============================================================================

// MARK: - Core Shift Function

/// Shifts a single character by the given number of positions in the alphabet.
///
/// This is the fundamental building block of the Caesar cipher. It handles:
/// - Uppercase letters (A-Z): shifted within the uppercase range
/// - Lowercase letters (a-z): shifted within the lowercase range
/// - Non-alphabetic characters: returned unchanged
///
/// The shift wraps around using modular arithmetic. A shift of 26 (or any
/// multiple of 26) returns the original character.
///
/// - Parameters:
///   - char: The character to shift.
///   - shift: The number of positions to shift (can be negative).
/// - Returns: The shifted character, or the original if non-alphabetic.
///
/// ## Worked Example
///
/// Shifting 'H' by 3:
/// 1. 'H' is uppercase, so our base is 'A' (Unicode scalar 65).
/// 2. Offset from base: 72 - 65 = 7
/// 3. Apply shift: (7 + 3) % 26 = 10
/// 4. Convert back: 65 + 10 = 75 = 'K'
///
/// Shifting 'z' by 1:
/// 1. 'z' is lowercase, so our base is 'a' (Unicode scalar 97).
/// 2. Offset from base: 122 - 97 = 25
/// 3. Apply shift: (25 + 1) % 26 = 0
/// 4. Convert back: 97 + 0 = 97 = 'a'  (wraps around!)
internal func shiftCharacter(_ char: Character, by shift: Int) -> Character {
    // ── Step 1: Check if the character is a letter ──────────────────────
    // Non-alphabetic characters (digits, spaces, punctuation, emoji) pass
    // through unchanged. The Caesar cipher only operates on letters.
    guard char.isLetter, let scalar = char.unicodeScalars.first else {
        return char
    }

    // ── Step 2: Determine the alphabet base ────────────────────────────
    // We need different base values for uppercase (A=65) and lowercase
    // (a=97) to preserve case in the output.
    let base: UInt32
    if char.isUppercase {
        base = UnicodeScalar("A").value  // 65
    } else {
        base = UnicodeScalar("a").value  // 97
    }

    // ── Step 3: Only shift ASCII letters ───────────────────────────────
    // Letters outside the ASCII range (accented characters, Cyrillic, etc.)
    // pass through unchanged. A production cipher would need to handle
    // these, but the classical Caesar cipher is defined only over the
    // 26-letter Latin alphabet.
    let value = scalar.value
    guard value >= base && value < base + 26 else {
        return char
    }

    // ── Step 4: Calculate the shifted position ─────────────────────────
    // We use the formula: ((offset + shift) % 26 + 26) % 26
    // The `+ 26` ensures we handle negative shifts correctly.
    //
    // Example with negative shift: decrypt 'K' with shift 3
    //   offset = 10 (K is 10th letter, 0-indexed)
    //   (10 - 3) % 26 = 7
    //   7 + 26 = 33
    //   33 % 26 = 7 → 'H'  ✓
    //
    // Example where % alone would fail: decrypt 'A' with shift 3
    //   offset = 0
    //   (0 - 3) % 26 = -3   ← negative! This is wrong without the fix.
    //   -3 + 26 = 23
    //   23 % 26 = 23 → 'X'  ✓
    let offset = Int(value - base)
    let shifted = ((offset + shift) % 26 + 26) % 26

    // ── Step 5: Convert back to a character ─────────────────────────────
    let newScalar = UnicodeScalar(base + UInt32(shifted))!
    return Character(newScalar)
}


// MARK: - Public API

/// Encrypts plaintext using a Caesar shift.
///
/// Each letter in the input is shifted forward by `shift` positions in the
/// alphabet. Non-alphabetic characters (digits, spaces, punctuation) pass
/// through unchanged. The case of each letter is preserved.
///
/// - Parameters:
///   - text: The plaintext string to encrypt.
///   - shift: The number of positions to shift each letter forward. Can be
///     negative (which is equivalent to decrypting). The shift wraps modulo
///     26, so a shift of 29 is the same as a shift of 3.
/// - Returns: The encrypted ciphertext.
///
/// ## Examples
///
/// ```swift
/// encrypt("HELLO", shift: 3)          // "KHOOR"
/// encrypt("hello", shift: 3)          // "khoor"
/// encrypt("Hello, World!", shift: 13) // "Uryyb, Jbeyq!"
/// encrypt("abc", shift: -1)           // "zab"
/// encrypt("abc", shift: 26)           // "abc" (full rotation)
/// ```
///
/// ## Historical Note
///
/// Caesar himself reportedly used a shift of 3. With the Latin alphabet of
/// his time (which had only 23 letters — no J, U, or W), a shift of 3
/// would turn "VENI VIDI VICI" into "YHQL YLGL YLFL". Our implementation
/// uses the modern 26-letter English alphabet.
public func encrypt(_ text: String, shift: Int) -> String {
    // Map each character through the shift function and collect into a String.
    // Swift's String type is a collection of Characters, so we can use `map`
    // directly. This is clean, functional, and idiomatic Swift.
    return String(text.map { shiftCharacter($0, by: shift) })
}


/// Decrypts ciphertext that was encrypted with a Caesar shift.
///
/// Decryption is simply encryption with the negative shift. If a message
/// was encrypted with shift 3, we decrypt by shifting -3 (or equivalently,
/// shifting 23, since -3 mod 26 = 23).
///
/// - Parameters:
///   - text: The ciphertext to decrypt.
///   - shift: The shift that was used to encrypt the original message.
/// - Returns: The decrypted plaintext.
///
/// ## Examples
///
/// ```swift
/// decrypt("KHOOR", shift: 3)          // "HELLO"
/// decrypt("khoor", shift: 3)          // "hello"
/// decrypt("Uryyb, Jbeyq!", shift: 13) // "Hello, World!"
/// ```
///
/// ## Round-Trip Property
///
/// For any text and shift:
///   decrypt(encrypt(text, shift: s), shift: s) == text
///
/// This is because encrypt shifts forward by `s` and decrypt shifts
/// backward by `s`, and the two operations cancel out.
public func decrypt(_ text: String, shift: Int) -> String {
    // Decryption is encryption with the negated shift. This follows directly
    // from the math: if we encrypted by adding `s`, we decrypt by subtracting
    // `s`. Rather than duplicating logic, we reuse `encrypt` with `-shift`.
    return encrypt(text, shift: -shift)
}


/// Applies ROT13 encoding to the input text.
///
/// ROT13 ("rotate by 13") is a special case of the Caesar cipher with a
/// shift of 13. It has a remarkable property: because 13 is exactly half
/// of 26 (the alphabet size), applying ROT13 twice returns the original
/// text. In other words, ROT13 is its own inverse:
///
///   rot13(rot13(text)) == text
///
/// This makes ROT13 particularly useful for:
/// - Hiding spoilers in online discussions
/// - Obscuring puzzle answers
/// - Simple obfuscation (NOT security — it's trivially reversible)
///
/// - Parameter text: The text to encode/decode.
/// - Returns: The ROT13-transformed text.
///
/// ## Why 13?
///
/// With 26 letters in the alphabet, a shift of 13 maps each letter to
/// a unique partner exactly halfway across the alphabet:
///
///   A ↔ N    B ↔ O    C ↔ P    D ↔ Q    E ↔ R    F ↔ S    G ↔ T
///   H ↔ U    I ↔ V    J ↔ W    K ↔ X    L ↔ Y    M ↔ Z
///
/// Each letter maps to a different letter, and applying the mapping
/// twice returns to the start. No other shift (except 0) has this
/// self-inverse property with a 26-letter alphabet.
///
/// ## Examples
///
/// ```swift
/// rot13("Hello")        // "Uryyb"
/// rot13("Uryyb")        // "Hello"
/// rot13("Why did the chicken cross the road?")
///   // "Jul qvq gur puvpxra pebff gur ebnq?"
/// ```
public func rot13(_ text: String) -> String {
    return encrypt(text, shift: 13)
}
