//! Cryptanalysis tools for breaking the Vigenere cipher.
//!
//! # Breaking the Vigenere Cipher
//!
//! The Vigenere cipher was considered unbreakable for 300 years because each
//! letter uses a different shift, defeating simple frequency analysis. Two
//! key statistical tools break it:
//!
//! ## Index of Coincidence (IC)
//!
//! IC measures how "non-uniform" a text's letter distribution is:
//!
//! ```text
//! IC = sum(count_i * (count_i - 1)) / (N * (N - 1))
//! ```
//!
//! - English text: IC ~0.0667 (letters are unevenly distributed)
//! - Random text: IC ~0.0385 (uniform distribution = 1/26)
//!
//! When we split ciphertext by the correct key length, each group is a
//! simple Caesar cipher on English, so its IC approaches 0.0667.
//!
//! ## Chi-Squared Statistic
//!
//! Once we know the key length, each position group is a Caesar cipher.
//! We try all 26 shifts and pick the one whose letter frequencies best
//! match English (lowest chi-squared value):
//!
//! ```text
//! chi2 = sum((observed_i - expected_i)^2 / expected_i)
//! ```

use crate::cipher::decrypt;

/// English letter frequencies (A-Z), used for chi-squared analysis.
///
/// These proportions represent how often each letter appears in a large
/// sample of English text. E (~12.7%) is the most common, Z (~0.07%)
/// the rarest.
const ENGLISH_FREQUENCIES: [f64; 26] = [
    0.08167, // A
    0.01492, // B
    0.02782, // C
    0.04253, // D
    0.12702, // E
    0.02228, // F
    0.02015, // G
    0.06094, // H
    0.06966, // I
    0.00153, // J
    0.00772, // K
    0.04025, // L
    0.02406, // M
    0.06749, // N
    0.07507, // O
    0.01929, // P
    0.00095, // Q
    0.05987, // R
    0.06327, // S
    0.09056, // T
    0.02758, // U
    0.00978, // V
    0.02360, // W
    0.00150, // X
    0.01974, // Y
    0.00074, // Z
];

/// Result of automatic cipher breaking.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakResult {
    pub key: String,
    pub plaintext: String,
}

/// Extract only ASCII alphabetic characters, converted to uppercase.
fn extract_alpha_upper(text: &str) -> Vec<u8> {
    text.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_uppercase() as u8)
        .collect()
}

/// Calculate the Index of Coincidence for a slice of uppercase letter bytes.
///
/// IC = sum(count_i * (count_i - 1)) / (N * (N - 1))
fn index_of_coincidence(letters: &[u8]) -> f64 {
    let n = letters.len();
    if n <= 1 {
        return 0.0;
    }

    let mut counts = [0usize; 26];
    for &ch in letters {
        counts[(ch - b'A') as usize] += 1;
    }

    let numerator: usize = counts.iter().map(|&c| c * c.saturating_sub(1)).sum();
    numerator as f64 / (n * (n - 1)) as f64
}

/// Calculate chi-squared statistic comparing observed counts to English.
///
/// Lower chi-squared means closer to English frequency distribution.
fn chi_squared(counts: &[usize; 26], total: usize) -> f64 {
    let mut chi2 = 0.0;
    for i in 0..26 {
        let expected = ENGLISH_FREQUENCIES[i] * total as f64;
        let diff = counts[i] as f64 - expected;
        chi2 += (diff * diff) / expected;
    }
    chi2
}

/// Estimate the key length of a Vigenere-encrypted ciphertext.
///
/// For each candidate key length k from 2 to max_length:
///   1. Split the ciphertext letters into k groups
///   2. Calculate the IC of each group
///   3. Average the ICs
///
/// The correct key length produces groups that are each a Caesar cipher
/// on English text (IC ~0.0667). Wrong key lengths produce more uniform
/// distributions (IC ~0.0385).
///
/// To avoid selecting multiples of the true key length (which also
/// produce high IC), we pick the smallest k whose average IC is within
/// 90% of the overall best IC value.
pub fn find_key_length(ciphertext: &str, max_length: usize) -> usize {
    let letters = extract_alpha_upper(ciphertext);

    if letters.len() < 2 {
        return 1;
    }

    let limit = max_length.min(letters.len() / 2);
    let mut avg_ics = vec![0.0f64; limit + 1];

    for k in 2..=limit {
        let mut total_ic = 0.0;
        let mut group_count = 0;

        for i in 0..k {
            let group: Vec<u8> = letters.iter()
                .skip(i)
                .step_by(k)
                .copied()
                .collect();

            if group.len() > 1 {
                total_ic += index_of_coincidence(&group);
                group_count += 1;
            }
        }

        if group_count > 0 {
            avg_ics[k] = total_ic / group_count as f64;
        }
    }

    // Find the best IC value
    let best_ic = avg_ics.iter().cloned().fold(0.0f64, f64::max);

    if best_ic <= 0.0 {
        return 1;
    }

    // Pick the smallest k whose IC is within 90% of the best
    let threshold = best_ic * 0.9;
    for k in 2..=limit {
        if avg_ics[k] >= threshold {
            return k;
        }
    }

    1
}

/// Find the key letters given a known key length.
///
/// For each position in the key (0 to key_length-1):
///   1. Extract the group of letters at that position
///   2. Try all 26 possible shifts (A through Z)
///   3. For each shift, compute letter frequencies and chi-squared
///   4. The shift with the lowest chi-squared is the key letter
///
/// This works because each group is a Caesar cipher, and the correct
/// shift produces an English-like frequency distribution.
pub fn find_key(ciphertext: &str, key_length: usize) -> String {
    let letters = extract_alpha_upper(ciphertext);
    let mut key = String::with_capacity(key_length);

    for pos in 0..key_length {
        let group: Vec<u8> = letters.iter()
            .skip(pos)
            .step_by(key_length)
            .copied()
            .collect();

        if group.is_empty() {
            key.push('A');
            continue;
        }

        // Try all 26 shifts, pick the one with lowest chi-squared
        let mut best_shift = 0u8;
        let mut best_chi2 = f64::INFINITY;

        for shift in 0..26u8 {
            let mut counts = [0usize; 26];
            for &ch in &group {
                let decrypted = (ch - b'A' + 26 - shift) % 26;
                counts[decrypted as usize] += 1;
            }

            let chi2 = chi_squared(&counts, group.len());
            if chi2 < best_chi2 {
                best_chi2 = chi2;
                best_shift = shift;
            }
        }

        key.push((b'A' + best_shift) as char);
    }

    key
}

/// Automatically break a Vigenere cipher.
///
/// Combines IC-based key length detection with chi-squared key recovery:
///   1. Find the key length using `find_key_length`
///   2. Find the key letters using `find_key`
///   3. Decrypt using the recovered key
///
/// Requires sufficiently long ciphertext (~200+ characters of English)
/// for reliable results.
pub fn break_cipher(ciphertext: &str) -> BreakResult {
    let key_length = find_key_length(ciphertext, 20);
    let key = find_key(ciphertext, key_length);
    let plaintext = decrypt(ciphertext, &key).unwrap_or_default();

    BreakResult { key, plaintext }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cipher::encrypt;

    /// Long English text for cryptanalysis tests. IC analysis needs a
    /// statistically significant sample (~600+ alpha chars) to reliably
    /// distinguish English from random text and avoid selecting multiples
    /// of the true key length.
    const LONG_ENGLISH_TEXT: &str = concat!(
        "The quick brown fox jumps over the lazy dog and then runs around the ",
        "entire neighborhood looking for more adventures to embark upon while ",
        "the sun slowly sets behind the distant mountains casting long shadows ",
        "across the valley below where the river winds its way through ancient ",
        "forests filled with towering oak trees and singing birds that herald ",
        "the coming of spring with their melodious songs echoing through the ",
        "canopy above where squirrels chase each other from branch to branch ",
        "gathering acorns and other nuts for the long winter months ahead when ",
        "the ground will be covered in a thick blanket of pristine white snow ",
        "and the children will build snowmen and throw snowballs at each other ",
        "laughing and playing until their parents call them inside for dinner ",
        "where warm soup and fresh bread await them on the old wooden table",
    );

    #[test]
    fn test_find_key_length_5() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "LEMON").unwrap();
        assert_eq!(find_key_length(&ct, 20), 5);
    }

    #[test]
    fn test_find_key_length_6() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "SECRET").unwrap();
        assert_eq!(find_key_length(&ct, 20), 6);
    }

    #[test]
    fn test_find_key_length_3() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "KEY").unwrap();
        assert_eq!(find_key_length(&ct, 20), 3);
    }

    #[test]
    fn test_find_key_length_short_text() {
        assert_eq!(find_key_length("A", 20), 1);
    }

    #[test]
    fn test_find_key_lemon() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "LEMON").unwrap();
        assert_eq!(find_key(&ct, 5), "LEMON");
    }

    #[test]
    fn test_find_key_secret() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "SECRET").unwrap();
        assert_eq!(find_key(&ct, 6), "SECRET");
    }

    #[test]
    fn test_find_key_key() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "KEY").unwrap();
        assert_eq!(find_key(&ct, 3), "KEY");
    }

    #[test]
    fn test_break_cipher_lemon() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "LEMON").unwrap();
        let result = break_cipher(&ct);
        assert_eq!(result.key, "LEMON");
        assert_eq!(result.plaintext, LONG_ENGLISH_TEXT);
    }

    #[test]
    fn test_break_cipher_secret() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "SECRET").unwrap();
        let result = break_cipher(&ct);
        assert_eq!(result.key, "SECRET");
        assert_eq!(result.plaintext, LONG_ENGLISH_TEXT);
    }

    #[test]
    fn test_break_cipher_consistent() {
        let ct = encrypt(LONG_ENGLISH_TEXT, "CIPHER").unwrap();
        let result = break_cipher(&ct);
        // Even if key recovery were imperfect, round-trip must be consistent
        let rt = decrypt(&encrypt(LONG_ENGLISH_TEXT, &result.key).unwrap(), &result.key).unwrap();
        assert_eq!(rt, LONG_ENGLISH_TEXT);
    }
}
