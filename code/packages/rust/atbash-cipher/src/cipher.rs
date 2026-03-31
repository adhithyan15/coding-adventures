//! Core Atbash cipher implementation.
//!
//! The Atbash cipher works by reversing the position of each letter in the
//! alphabet. Think of it like reading the alphabet backwards:
//!
//! ```text
//! Forward:  A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//! Reversed: Z Y X W V U T S R Q P O N M L K J I H G F E D C B A
//! ```
//!
//! ## The Formula
//!
//! For any letter at position `p` (where A=0, B=1, ..., Z=25):
//!
//! ```text
//! new_position = 25 - p
//! ```
//!
//! For example:
//! - H is at position 7.  25 - 7  = 18, which is S.
//! - E is at position 4.  25 - 4  = 21, which is V.
//! - L is at position 11. 25 - 11 = 14, which is O.
//! - O is at position 14. 25 - 14 = 11, which is L.
//!
//! So "HELLO" becomes "SVOOL".
//!
//! ## Why It's Self-Inverse
//!
//! `f(f(x)) = 25 - (25 - x) = x`
//!
//! Encrypting "SVOOL" gives back "HELLO". The cipher undoes itself!

/// Apply the Atbash substitution to a single character.
///
/// The algorithm:
/// 1. Check if the character is an uppercase letter (A-Z) or lowercase (a-z).
/// 2. If it's a letter, compute its position (0-25), reverse it (25 - pos),
///    and convert back to a character.
/// 3. If it's not a letter, return it unchanged.
///
/// # Examples
///
/// ```
/// # use atbash_cipher::cipher::atbash_char;
/// assert_eq!(atbash_char('A'), 'Z');
/// assert_eq!(atbash_char('z'), 'a');
/// assert_eq!(atbash_char('5'), '5');
/// ```
pub fn atbash_char(ch: char) -> char {
    match ch {
        // Uppercase letters: A=65 through Z=90
        'A'..='Z' => {
            let position = ch as u8 - b'A'; // A=0, B=1, ..., Z=25
            let new_position = 25 - position; // Reverse: 0->25, 1->24, ..., 25->0
            (b'A' + new_position) as char // Convert back to a letter
        }

        // Lowercase letters: a=97 through z=122
        'a'..='z' => {
            let position = ch as u8 - b'a'; // a=0, b=1, ..., z=25
            let new_position = 25 - position; // Reverse
            (b'a' + new_position) as char
        }

        // Non-alphabetic characters pass through unchanged
        _ => ch,
    }
}

/// Encrypt text using the Atbash cipher.
///
/// Each letter is replaced by its reverse in the alphabet (A<->Z, B<->Y, etc.).
/// Non-alphabetic characters pass through unchanged. Case is preserved.
///
/// Because the Atbash cipher is self-inverse, this function is identical
/// to [`decrypt`]. Both are provided for API clarity.
///
/// # Examples
///
/// ```
/// use atbash_cipher::encrypt;
///
/// assert_eq!(encrypt("HELLO"), "SVOOL");
/// assert_eq!(encrypt("hello"), "svool");
/// assert_eq!(encrypt("Hello, World! 123"), "Svool, Dliow! 123");
/// ```
pub fn encrypt(text: &str) -> String {
    // Map each character through the Atbash substitution and collect
    // the results into a new String. Rust's char iteration handles
    // UTF-8 correctly, though we only transform ASCII letters.
    text.chars().map(atbash_char).collect()
}

/// Decrypt text using the Atbash cipher.
///
/// Because the Atbash cipher is self-inverse (applying it twice returns
/// the original), decryption is identical to encryption. This function
/// exists for API clarity.
///
/// # Examples
///
/// ```
/// use atbash_cipher::{encrypt, decrypt};
///
/// assert_eq!(decrypt("SVOOL"), "HELLO");
/// assert_eq!(decrypt(&encrypt("secret message")), "secret message");
/// ```
pub fn decrypt(text: &str) -> String {
    // Decryption IS encryption for Atbash.
    // Proof: f(f(x)) = 25 - (25 - x) = x
    encrypt(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Basic Encryption ---

    #[test]
    fn test_encrypt_hello_uppercase() {
        assert_eq!(encrypt("HELLO"), "SVOOL");
    }

    #[test]
    fn test_encrypt_hello_lowercase() {
        assert_eq!(encrypt("hello"), "svool");
    }

    #[test]
    fn test_encrypt_mixed_case_with_punctuation() {
        assert_eq!(encrypt("Hello, World! 123"), "Svool, Dliow! 123");
    }

    #[test]
    fn test_encrypt_full_uppercase_alphabet() {
        assert_eq!(
            encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            "ZYXWVUTSRQPONMLKJIHGFEDCBA"
        );
    }

    #[test]
    fn test_encrypt_full_lowercase_alphabet() {
        assert_eq!(
            encrypt("abcdefghijklmnopqrstuvwxyz"),
            "zyxwvutsrqponmlkjihgfedcba"
        );
    }

    // --- Case Preservation ---

    #[test]
    fn test_uppercase_stays_uppercase() {
        assert_eq!(encrypt("ABC"), "ZYX");
    }

    #[test]
    fn test_lowercase_stays_lowercase() {
        assert_eq!(encrypt("abc"), "zyx");
    }

    #[test]
    fn test_mixed_case_preserved() {
        assert_eq!(encrypt("AbCdEf"), "ZyXwVu");
    }

    // --- Non-Alpha Passthrough ---

    #[test]
    fn test_digits_unchanged() {
        assert_eq!(encrypt("12345"), "12345");
    }

    #[test]
    fn test_punctuation_unchanged() {
        assert_eq!(encrypt("!@#$%"), "!@#$%");
    }

    #[test]
    fn test_spaces_unchanged() {
        assert_eq!(encrypt("   "), "   ");
    }

    #[test]
    fn test_mixed_alpha_and_digits() {
        assert_eq!(encrypt("A1B2C3"), "Z1Y2X3");
    }

    #[test]
    fn test_newlines_and_tabs() {
        assert_eq!(encrypt("A\nB\tC"), "Z\nY\tX");
    }

    // --- Self-Inverse Property ---

    #[test]
    fn test_self_inverse_hello() {
        assert_eq!(encrypt(&encrypt("HELLO")), "HELLO");
    }

    #[test]
    fn test_self_inverse_lowercase() {
        assert_eq!(encrypt(&encrypt("hello")), "hello");
    }

    #[test]
    fn test_self_inverse_mixed() {
        let input = "Hello, World! 123";
        assert_eq!(encrypt(&encrypt(input)), input);
    }

    #[test]
    fn test_self_inverse_full_alphabet() {
        let alpha = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        assert_eq!(encrypt(&encrypt(alpha)), alpha);
    }

    #[test]
    fn test_self_inverse_empty() {
        assert_eq!(encrypt(&encrypt("")), "");
    }

    #[test]
    fn test_self_inverse_long_text() {
        let text = "The quick brown fox jumps over the lazy dog! 42";
        assert_eq!(encrypt(&encrypt(text)), text);
    }

    // --- Edge Cases ---

    #[test]
    fn test_empty_string() {
        assert_eq!(encrypt(""), "");
    }

    #[test]
    fn test_single_letters() {
        assert_eq!(encrypt("A"), "Z");
        assert_eq!(encrypt("Z"), "A");
        assert_eq!(encrypt("M"), "N");
        assert_eq!(encrypt("N"), "M");
        assert_eq!(encrypt("a"), "z");
        assert_eq!(encrypt("z"), "a");
    }

    #[test]
    fn test_single_digit() {
        assert_eq!(encrypt("5"), "5");
    }

    #[test]
    fn test_no_letter_maps_to_itself() {
        // 25 - p == p only when p == 12.5, which is not an integer
        for i in 0..26u8 {
            let upper = (b'A' + i) as char;
            assert_ne!(
                encrypt(&upper.to_string()),
                upper.to_string(),
                "{} maps to itself!",
                upper
            );

            let lower = (b'a' + i) as char;
            assert_ne!(
                encrypt(&lower.to_string()),
                lower.to_string(),
                "{} maps to itself!",
                lower
            );
        }
    }

    // --- Decrypt ---

    #[test]
    fn test_decrypt_svool() {
        assert_eq!(decrypt("SVOOL"), "HELLO");
    }

    #[test]
    fn test_decrypt_lowercase() {
        assert_eq!(decrypt("svool"), "hello");
    }

    #[test]
    fn test_decrypt_is_encrypt_inverse() {
        let texts = vec!["HELLO", "hello", "Hello, World! 123", "", "42"];
        for text in texts {
            assert_eq!(decrypt(&encrypt(text)), text);
        }
    }

    #[test]
    fn test_encrypt_decrypt_equivalence() {
        let texts = vec!["HELLO", "svool", "Test!", ""];
        for text in texts {
            assert_eq!(encrypt(text), decrypt(text));
        }
    }
}
