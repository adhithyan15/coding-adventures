//! # Cipher -- encrypt and decrypt with the Caesar cipher
//!
//! This module contains the core transformation functions: [`encrypt`],
//! [`decrypt`], and the special case [`rot13`].
//!
//! ## How the shift works
//!
//! We number letters A=0, B=1, ... Z=25.  Encryption with shift `s` maps
//! each letter at position `p` to position `(p + s) mod 26`.  Decryption
//! maps it to `(p - s) mod 26`, which is the same as `(p + 26 - s) mod 26`.
//!
//! ### Truth table for shift = 3
//!
//! ```text
//! Input  | Position | + Shift | mod 26 | Output
//! -------|----------|---------|--------|-------
//!   A    |    0     |    3    |    3   |   D
//!   B    |    1     |    4    |    4   |   E
//!   H    |    7     |   10    |   10   |   K
//!   X    |   23     |   26    |    0   |   A
//!   Y    |   24     |   27    |    1   |   B
//!   Z    |   25     |   28    |    2   |   C
//! ```
//!
//! ### Non-alphabetic characters
//!
//! Digits, spaces, punctuation, and other non-letter characters pass
//! through unchanged.  This matches the historical usage of the cipher,
//! which was only applied to letters.
//!
//! ### Negative shifts
//!
//! A negative shift moves letters backwards through the alphabet.  For
//! example, shift = -1 maps B to A and A to Z.  Internally we normalise
//! every shift into the range 0..25 using modular arithmetic, so shift -1
//! and shift 25 produce the same result.
//!
//! ```text
//! shift = -1
//! normalised = ((-1 % 26) + 26) % 26 = ((-1) + 26) % 26 = 25
//! ```

/// Encrypt `text` using the Caesar cipher with the given `shift`.
///
/// Each ASCII letter is shifted forward through the alphabet by `shift`
/// positions.  Non-alphabetic characters are left unchanged.  Case is
/// preserved: an uppercase input letter produces an uppercase output
/// letter.
///
/// # Worked example
///
/// ```text
/// encrypt("HELLO", 3)
///
/// H (pos  7) -> (7  + 3) % 26 = 10 -> K
/// E (pos  4) -> (4  + 3) % 26 =  7 -> H
/// L (pos 11) -> (11 + 3) % 26 = 14 -> O
/// L (pos 11) -> (11 + 3) % 26 = 14 -> O
/// O (pos 14) -> (14 + 3) % 26 = 17 -> R
///
/// Result: "KHOOR"
/// ```
///
/// # Examples
///
/// ```
/// use caesar_cipher::cipher::encrypt;
///
/// assert_eq!(encrypt("HELLO", 3), "KHOOR");
/// assert_eq!(encrypt("hello", 3), "khoor");
/// assert_eq!(encrypt("Hello, World!", 3), "Khoor, Zruog!");
/// assert_eq!(encrypt("abc", 0), "abc");
/// assert_eq!(encrypt("ABC", 26), "ABC");  // full rotation
/// assert_eq!(encrypt("ABC", -1), "ZAB");  // negative shift
/// ```
pub fn encrypt(text: &str, shift: i32) -> String {
    // ---------------------------------------------------------------
    // Step 1: Normalise the shift into the range 0..25.
    //
    // Rust's `%` operator can return negative values for negative
    // dividends, so we add 26 and take mod 26 again to guarantee a
    // non-negative result.
    //
    //   shift = -1  =>  (-1 % 26) + 26 = 25 + 26 = 51  =>  51 % 26 = 25
    //   shift =  3  =>  ( 3 % 26) + 26 = 3  + 26 = 29  =>  29 % 26 =  3
    //   shift = 29  =>  (29 % 26) + 26 = 3  + 26 = 29  =>  29 % 26 =  3
    // ---------------------------------------------------------------
    let normalised_shift = ((shift % 26) + 26) % 26;

    // ---------------------------------------------------------------
    // Step 2: Transform each character.
    //
    // For each character we check:
    //   - Is it an uppercase ASCII letter?  base = b'A'
    //   - Is it a lowercase ASCII letter?  base = b'a'
    //   - Otherwise, pass it through unchanged.
    //
    // The formula:  new_char = base + (old_position + shift) % 26
    // ---------------------------------------------------------------
    text.chars()
        .map(|ch| shift_char(ch, normalised_shift))
        .collect()
}

/// Decrypt `text` that was encrypted with the Caesar cipher using `shift`.
///
/// Decryption is the inverse of encryption: we shift each letter
/// *backwards* by `shift` positions.  This is equivalent to encrypting
/// with shift `26 - shift`.
///
/// # Round-trip property
///
/// For any text `t` and shift `s`:
///
/// ```text
/// decrypt(encrypt(t, s), s) == t
/// ```
///
/// # Examples
///
/// ```
/// use caesar_cipher::cipher::{encrypt, decrypt};
///
/// let original = "Attack at dawn!";
/// let encrypted = encrypt(original, 7);
/// assert_eq!(decrypt(&encrypted, 7), original);
/// ```
pub fn decrypt(text: &str, shift: i32) -> String {
    // Decryption with shift `s` is encryption with shift `-s`.
    // Our `encrypt` function already handles negative shifts via
    // normalisation, so this one-liner is both correct and clear.
    encrypt(text, -shift)
}

/// Apply ROT13 -- a special Caesar cipher with shift 13.
///
/// ROT13 is its own inverse because 13 + 13 = 26, a full rotation:
///
/// ```text
/// rot13(rot13(text)) == text
/// ```
///
/// ROT13 was historically popular on Usenet for hiding spoilers and
/// punchlines.  It provides no real security but is useful for light
/// obfuscation.
///
/// # Truth table (selected letters)
///
/// ```text
/// Input | Output    Input | Output
/// ------|-------    ------|-------
///   A   |   N        N   |   A
///   B   |   O        O   |   B
///   H   |   U        U   |   H
///   M   |   Z        Z   |   M
/// ```
///
/// # Examples
///
/// ```
/// use caesar_cipher::cipher::rot13;
///
/// assert_eq!(rot13("Hello"), "Uryyb");
/// assert_eq!(rot13("Uryyb"), "Hello");  // self-inverse
/// assert_eq!(rot13("123!"), "123!");     // non-alpha unchanged
/// ```
pub fn rot13(text: &str) -> String {
    encrypt(text, 13)
}

// ===================================================================
// Internal helper
// ===================================================================

/// Shift a single character by `normalised_shift` positions (0..25).
///
/// This is the workhorse of the module.  It handles uppercase letters,
/// lowercase letters, and non-alphabetic characters separately.
///
/// ```text
///  ch = 'H', normalised_shift = 3
///  base = b'A' = 65
///  position = 72 - 65 = 7
///  new_position = (7 + 3) % 26 = 10
///  new_byte = 65 + 10 = 75 = 'K'
/// ```
fn shift_char(ch: char, normalised_shift: i32) -> char {
    if ch.is_ascii_uppercase() {
        // 'A' is byte 65.  We subtract it to get position 0..25,
        // add the shift, wrap with mod 26, then add 'A' back.
        let base = b'A' as i32;
        let position = ch as i32 - base;
        let new_position = (position + normalised_shift) % 26;
        (base + new_position) as u8 as char
    } else if ch.is_ascii_lowercase() {
        // Same logic with 'a' (byte 97) as the base.
        let base = b'a' as i32;
        let position = ch as i32 - base;
        let new_position = (position + normalised_shift) % 26;
        (base + new_position) as u8 as char
    } else {
        // Non-alphabetic: digits, spaces, punctuation, emoji, etc.
        ch
    }
}

// ===================================================================
// Unit tests (module-level, fast sanity checks)
// ===================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_basic() {
        assert_eq!(encrypt("HELLO", 3), "KHOOR");
    }

    #[test]
    fn decrypt_basic() {
        assert_eq!(decrypt("KHOOR", 3), "HELLO");
    }

    #[test]
    fn rot13_self_inverse() {
        let text = "The Quick Brown Fox";
        assert_eq!(rot13(&rot13(text)), text);
    }

    #[test]
    fn shift_zero_is_identity() {
        assert_eq!(encrypt("abc XYZ 123!", 0), "abc XYZ 123!");
    }

    #[test]
    fn shift_26_is_identity() {
        assert_eq!(encrypt("abc XYZ 123!", 26), "abc XYZ 123!");
    }

    #[test]
    fn negative_shift() {
        assert_eq!(encrypt("ABC", -1), "ZAB");
    }
}
