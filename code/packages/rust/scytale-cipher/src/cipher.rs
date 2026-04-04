//! Core Scytale cipher implementation.
//!
//! # The Scytale Cipher
//!
//! The Scytale (pronounced "SKIT-ah-lee") cipher is a *transposition* cipher
//! from ancient Sparta (~700 BCE). The physical Scytale was a wooden rod
//! around which a strip of leather was wrapped. The message was written along
//! the rod's length, then the strip was unwrapped. Only someone with a rod
//! of the same diameter (the key) could read it.
//!
//! # How Encryption Works
//!
//! 1. Write text row-by-row into a grid with `key` columns.
//! 2. Pad the last row with spaces if needed.
//! 3. Read column-by-column to produce ciphertext.
//!
//! # Example
//!
//! ```
//! use scytale_cipher::encrypt;
//!
//! let ct = encrypt("HELLO WORLD", 3).unwrap();
//! assert_eq!(ct, "HLWLEOODL R ");
//! ```

/// A single brute-force decryption result.
#[derive(Debug, Clone, PartialEq)]
pub struct BruteForceResult {
    pub key: usize,
    pub text: String,
}

/// Encrypt text using the Scytale transposition cipher.
///
/// Returns `Err` if key < 2 or key > text length.
pub fn encrypt(text: &str, key: usize) -> Result<String, String> {
    if text.is_empty() {
        return Ok(String::new());
    }

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();

    if key < 2 {
        return Err(format!("Key must be >= 2, got {}", key));
    }
    if key > n {
        return Err(format!("Key must be <= text length ({}), got {}", n, key));
    }

    // Calculate grid dimensions and pad with spaces
    let num_rows = (n + key - 1) / key;
    let padded_len = num_rows * key;

    let mut padded = chars.clone();
    padded.resize(padded_len, ' ');

    // Read column-by-column
    let mut result = String::with_capacity(padded_len);
    for col in 0..key {
        for row in 0..num_rows {
            result.push(padded[row * key + col]);
        }
    }

    Ok(result)
}

/// Decrypt ciphertext that was encrypted with the Scytale cipher.
///
/// Trailing padding spaces are stripped.
/// Returns `Err` if key < 2 or key > text length.
pub fn decrypt(text: &str, key: usize) -> Result<String, String> {
    if text.is_empty() {
        return Ok(String::new());
    }

    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();

    if key < 2 {
        return Err(format!("Key must be >= 2, got {}", key));
    }
    if key > n {
        return Err(format!("Key must be <= text length ({}), got {}", n, key));
    }

    let num_rows = (n + key - 1) / key;

    // Handle uneven grids (when n % key != 0, e.g. during brute-force)
    let full_cols = if n % key == 0 { key } else { n % key };

    // Compute column start indices and lengths
    let mut col_starts = Vec::with_capacity(key);
    let mut col_lens = Vec::with_capacity(key);
    let mut offset = 0;
    for c in 0..key {
        col_starts.push(offset);
        let len = if n % key == 0 || c < full_cols { num_rows } else { num_rows - 1 };
        col_lens.push(len);
        offset += len;
    }

    // Read row-by-row
    let mut result = String::with_capacity(n);
    for row in 0..num_rows {
        for col in 0..key {
            if row < col_lens[col] {
                result.push(chars[col_starts[col] + row]);
            }
        }
    }

    // Strip trailing padding spaces
    Ok(result.trim_end_matches(' ').to_string())
}

/// Try all possible Scytale keys and return decryption results.
///
/// Keys range from 2 to len/2.
pub fn brute_force(text: &str) -> Vec<BruteForceResult> {
    let n = text.chars().count();
    if n < 4 {
        return Vec::new();
    }

    let max_key = n / 2;
    let mut results = Vec::with_capacity(max_key - 1);

    for candidate_key in 2..=max_key {
        if let Ok(decrypted) = decrypt(text, candidate_key) {
            results.push(BruteForceResult {
                key: candidate_key,
                text: decrypted,
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_hello_world_key3() {
        assert_eq!(encrypt("HELLO WORLD", 3).unwrap(), "HLWLEOODL R ");
    }

    #[test]
    fn test_encrypt_abcdef_key2() {
        assert_eq!(encrypt("ABCDEF", 2).unwrap(), "ACEBDF");
    }

    #[test]
    fn test_encrypt_abcdef_key3() {
        assert_eq!(encrypt("ABCDEF", 3).unwrap(), "ADBECF");
    }

    #[test]
    fn test_encrypt_key_equals_length() {
        assert_eq!(encrypt("ABCD", 4).unwrap(), "ABCD");
    }

    #[test]
    fn test_encrypt_empty() {
        assert_eq!(encrypt("", 2).unwrap(), "");
    }

    #[test]
    fn test_encrypt_invalid_key_zero() {
        assert!(encrypt("HELLO", 0).is_err());
    }

    #[test]
    fn test_encrypt_invalid_key_one() {
        assert!(encrypt("HELLO", 1).is_err());
    }

    #[test]
    fn test_encrypt_key_too_large() {
        assert!(encrypt("HI", 3).is_err());
    }

    #[test]
    fn test_decrypt_hello_world_key3() {
        assert_eq!(decrypt("HLWLEOODL R ", 3).unwrap(), "HELLO WORLD");
    }

    #[test]
    fn test_decrypt_acebdf_key2() {
        assert_eq!(decrypt("ACEBDF", 2).unwrap(), "ABCDEF");
    }

    #[test]
    fn test_decrypt_empty() {
        assert_eq!(decrypt("", 2).unwrap(), "");
    }

    #[test]
    fn test_decrypt_invalid_key() {
        assert!(decrypt("HELLO", 0).is_err());
        assert!(decrypt("HI", 3).is_err());
    }

    #[test]
    fn test_round_trip() {
        let texts = vec![
            ("HELLO WORLD", 3),
            ("ABCDEF", 2),
            ("ABCDEF", 3),
            ("The quick brown fox", 4),
            ("12345", 2),
        ];
        for (text, key) in texts {
            let ct = encrypt(text, key).unwrap();
            let pt = decrypt(&ct, key).unwrap();
            assert_eq!(pt, text, "Round trip failed for key={}", key);
        }
    }

    #[test]
    fn test_round_trip_all_keys() {
        let text = "The quick brown fox jumps over the lazy dog!";
        let n = text.chars().count();
        for key in 2..=(n / 2) {
            let ct = encrypt(text, key).unwrap();
            let pt = decrypt(&ct, key).unwrap();
            assert_eq!(pt, text, "Round trip failed for key={}", key);
        }
    }

    #[test]
    fn test_brute_force_finds_original() {
        let original = "HELLO WORLD";
        let ct = encrypt(original, 3).unwrap();
        let results = brute_force(&ct);
        let found = results.iter().find(|r| r.key == 3);
        assert!(found.is_some());
        assert_eq!(found.unwrap().text, original);
    }

    #[test]
    fn test_brute_force_returns_all_keys() {
        let results = brute_force("ABCDEFGHIJ");
        let keys: Vec<usize> = results.iter().map(|r| r.key).collect();
        assert_eq!(keys, vec![2, 3, 4, 5]);
    }

    #[test]
    fn test_brute_force_short_text() {
        assert!(brute_force("AB").is_empty());
        assert!(brute_force("ABC").is_empty());
    }

    #[test]
    fn test_padding_stripped() {
        let ct = encrypt("HELLO", 3).unwrap();
        assert_eq!(decrypt(&ct, 3).unwrap(), "HELLO");
    }

    #[test]
    fn test_no_padding_needed() {
        let ct = encrypt("ABCDEF", 2).unwrap();
        assert_eq!(ct.len(), 6);
    }
}
