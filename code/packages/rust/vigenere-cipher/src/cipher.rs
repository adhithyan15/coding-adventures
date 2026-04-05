//! Core Vigenere cipher encrypt/decrypt implementation.
//!
//! # The Vigenere Cipher
//!
//! The Vigenere cipher is a *polyalphabetic substitution* cipher invented by
//! Giovan Battista Bellaso in 1553 (commonly misattributed to Blaise de
//! Vigenere). Each plaintext letter is shifted by a different amount
//! determined by the corresponding letter of the repeating keyword.
//!
//! # How Encryption Works
//!
//! ```text
//! Plaintext:  A  T  T  A  C  K  A  T  D  A  W  N
//! Key cycle:  L  E  M  O  N  L  E  M  O  N  L  E
//! Shift:      11 4  12 14 13 11 4  12 14 13 11 4
//! Ciphertext: L  X  F  O  P  V  E  F  R  N  H  R
//! ```
//!
//! # Character Handling
//!
//! - Uppercase letters stay uppercase, lowercase stay lowercase.
//! - Non-alphabetic characters pass through unchanged.
//! - The key position advances only on alphabetic characters.
//! - The key must be non-empty and contain only ASCII letters.
//!
//! # Example
//!
//! ```
//! use vigenere_cipher::encrypt;
//!
//! let ct = encrypt("ATTACKATDAWN", "LEMON").unwrap();
//! assert_eq!(ct, "LXFOPVEFRNHR");
//! ```

/// Validate that a key is non-empty and contains only ASCII letters.
fn validate_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("Key must not be empty".to_string());
    }
    if !key.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err("Key must contain only alphabetic characters".to_string());
    }
    Ok(())
}

/// Encrypt plaintext using the Vigenere cipher.
///
/// Each alphabetic character is shifted forward by the corresponding
/// key letter's value (A/a=0, B/b=1, ..., Z/z=25). Non-alphabetic
/// characters pass through unchanged and do not advance the key position.
///
/// Returns `Err` if the key is empty or contains non-alphabetic characters.
pub fn encrypt(plaintext: &str, key: &str) -> Result<String, String> {
    validate_key(key)?;

    let key_upper: Vec<u8> = key.to_uppercase().bytes().collect();
    let key_len = key_upper.len();
    let mut key_index = 0;
    let mut result = String::with_capacity(plaintext.len());

    for ch in plaintext.chars() {
        if ch.is_ascii_uppercase() {
            // Shift uppercase letter forward by key amount
            let shift = key_upper[key_index % key_len] - b'A';
            let shifted = ((ch as u8 - b'A' + shift) % 26) + b'A';
            result.push(shifted as char);
            key_index += 1;
        } else if ch.is_ascii_lowercase() {
            // Shift lowercase letter forward, preserving case
            let shift = key_upper[key_index % key_len] - b'A';
            let shifted = ((ch as u8 - b'a' + shift) % 26) + b'a';
            result.push(shifted as char);
            key_index += 1;
        } else {
            // Non-alpha passes through, key does NOT advance
            result.push(ch);
        }
    }

    Ok(result)
}

/// Decrypt ciphertext using the Vigenere cipher.
///
/// Identical to encrypt but shifts *backward*. Since modular arithmetic
/// with subtraction can go negative, we add 26 before taking mod 26.
///
/// Returns `Err` if the key is empty or contains non-alphabetic characters.
pub fn decrypt(ciphertext: &str, key: &str) -> Result<String, String> {
    validate_key(key)?;

    let key_upper: Vec<u8> = key.to_uppercase().bytes().collect();
    let key_len = key_upper.len();
    let mut key_index = 0;
    let mut result = String::with_capacity(ciphertext.len());

    for ch in ciphertext.chars() {
        if ch.is_ascii_uppercase() {
            let shift = key_upper[key_index % key_len] - b'A';
            let shifted = ((ch as u8 - b'A' + 26 - shift) % 26) + b'A';
            result.push(shifted as char);
            key_index += 1;
        } else if ch.is_ascii_lowercase() {
            let shift = key_upper[key_index % key_len] - b'A';
            let shifted = ((ch as u8 - b'a' + 26 - shift) % 26) + b'a';
            result.push(shifted as char);
            key_index += 1;
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Encrypt tests
    // ------------------------------------------------------------------

    #[test]
    fn test_encrypt_attackatdawn() {
        assert_eq!(encrypt("ATTACKATDAWN", "LEMON").unwrap(), "LXFOPVEFRNHR");
    }

    #[test]
    fn test_encrypt_preserves_case_and_punctuation() {
        assert_eq!(encrypt("Hello, World!", "key").unwrap(), "Rijvs, Uyvjn!");
    }

    #[test]
    fn test_encrypt_all_lowercase() {
        assert_eq!(encrypt("attackatdawn", "lemon").unwrap(), "lxfopvefrnhr");
    }

    #[test]
    fn test_encrypt_mixed_case_key() {
        assert_eq!(encrypt("ATTACKATDAWN", "LeMoN").unwrap(), "LXFOPVEFRNHR");
    }

    #[test]
    fn test_encrypt_single_char_key() {
        assert_eq!(encrypt("ABC", "B").unwrap(), "BCD");
    }

    #[test]
    fn test_encrypt_skips_non_alpha() {
        assert_eq!(encrypt("A T", "LE").unwrap(), "L X");
    }

    #[test]
    fn test_encrypt_digits_unchanged() {
        assert_eq!(encrypt("Hello 123!", "key").unwrap(), "Rijvs 123!");
    }

    #[test]
    fn test_encrypt_empty() {
        assert_eq!(encrypt("", "key").unwrap(), "");
    }

    #[test]
    fn test_encrypt_empty_key() {
        assert!(encrypt("hello", "").is_err());
    }

    #[test]
    fn test_encrypt_non_alpha_key() {
        assert!(encrypt("hello", "key1").is_err());
        assert!(encrypt("hello", "ke y").is_err());
    }

    // ------------------------------------------------------------------
    // Decrypt tests
    // ------------------------------------------------------------------

    #[test]
    fn test_decrypt_lxfopvefrnhr() {
        assert_eq!(decrypt("LXFOPVEFRNHR", "LEMON").unwrap(), "ATTACKATDAWN");
    }

    #[test]
    fn test_decrypt_preserves_case_and_punctuation() {
        assert_eq!(decrypt("Rijvs, Uyvjn!", "key").unwrap(), "Hello, World!");
    }

    #[test]
    fn test_decrypt_all_lowercase() {
        assert_eq!(decrypt("lxfopvefrnhr", "lemon").unwrap(), "attackatdawn");
    }

    #[test]
    fn test_decrypt_empty() {
        assert_eq!(decrypt("", "key").unwrap(), "");
    }

    #[test]
    fn test_decrypt_empty_key() {
        assert!(decrypt("hello", "").is_err());
    }

    #[test]
    fn test_decrypt_non_alpha_key() {
        assert!(decrypt("hello", "123").is_err());
    }

    // ------------------------------------------------------------------
    // Round-trip tests
    // ------------------------------------------------------------------

    #[test]
    fn test_round_trip() {
        let cases = vec![
            ("ATTACKATDAWN", "LEMON"),
            ("Hello, World!", "key"),
            ("The quick brown fox!", "SECRET"),
            ("abc def ghi", "xyz"),
            ("MiXeD CaSe 123", "AbCdE"),
            ("a", "z"),
            ("ZZZZZZ", "A"),
        ];
        for (text, key) in cases {
            let ct = encrypt(text, key).unwrap();
            let pt = decrypt(&ct, key).unwrap();
            assert_eq!(pt, text, "Round trip failed for key={}", key);
        }
    }
}
