//! # Analysis -- breaking the Caesar cipher
//!
//! The Caesar cipher has only 25 possible non-trivial keys (shifts 1..25),
//! making it vulnerable to two classic attacks:
//!
//! 1. **Brute force** -- try every possible shift and present all 25
//!    candidate plaintexts to the analyst.
//!
//! 2. **Frequency analysis** -- compare the letter distribution of the
//!    ciphertext against the known distribution of English and pick the
//!    shift that produces the best statistical match.
//!
//! ## Letter frequencies in English
//!
//! The table below shows the expected frequency of each letter in a large
//! sample of English text.  The letter E is by far the most common at
//! around 12.7%, while Z is the rarest at about 0.07%.
//!
//! ```text
//! Letter | Freq (%)     Letter | Freq (%)
//! -------|--------      -------|--------
//!   A    |  8.167         N    |  6.749
//!   B    |  1.492         O    |  7.507
//!   C    |  2.782         P    |  1.929
//!   D    |  4.253         Q    |  0.095
//!   E    | 12.702         R    |  5.987
//!   F    |  2.228         S    |  6.327
//!   G    |  2.015         T    |  9.056
//!   H    |  6.094         U    |  2.758
//!   I    |  6.966         V    |  0.978
//!   J    |  0.153         W    |  2.360
//!   K    |  0.772         X    |  0.150
//!   L    |  4.025         Y    |  1.974
//!   M    |  2.406         Z    |  0.074
//! ```
//!
//! ## Chi-squared statistic
//!
//! To measure how well a candidate plaintext matches English, we use the
//! chi-squared goodness-of-fit test:
//!
//! ```text
//!          25
//! chi2 =  SUM  (observed_i - expected_i)^2 / expected_i
//!         i=0
//! ```
//!
//! where `observed_i` is the count of the i-th letter in the candidate
//! plaintext and `expected_i` is the count we would expect based on the
//! known English frequencies scaled to the text length.
//!
//! A *lower* chi-squared value indicates a better fit.  We try all 25
//! non-trivial shifts, compute chi-squared for each, and pick the shift
//! that yields the lowest value.

use crate::cipher;

// ===================================================================
// English letter frequencies (A=0, B=1, ... Z=25)
// ===================================================================

/// Expected frequency of each letter in English text, as a fraction
/// (not a percentage).  Index 0 is A, index 25 is Z.
///
/// These values come from large-corpus analysis of English text and are
/// widely cited in cryptography literature.
///
/// ```text
/// ENGLISH_FREQUENCIES[0]  = 0.08167  // A
/// ENGLISH_FREQUENCIES[4]  = 0.12702  // E  (most common)
/// ENGLISH_FREQUENCIES[25] = 0.00074  // Z  (least common)
/// ```
pub const ENGLISH_FREQUENCIES: [f64; 26] = [
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

// ===================================================================
// Brute-force attack
// ===================================================================

/// One candidate result from a brute-force attack.
///
/// Contains the shift that was tried and the resulting plaintext.
#[derive(Debug, Clone, PartialEq)]
pub struct BruteForceResult {
    /// The shift value that was applied to decrypt.
    pub shift: i32,
    /// The plaintext produced by decrypting with this shift.
    pub plaintext: String,
}

/// Try all 25 non-trivial shifts and return the candidate plaintexts.
///
/// Shift 0 is excluded because it is the identity (ciphertext ==
/// plaintext).  The results are returned in order from shift 1 to
/// shift 25.
///
/// # How it works
///
/// ```text
/// for shift in 1..=25:
///     candidate = decrypt(ciphertext, shift)
///     results.push(BruteForceResult { shift, candidate })
/// ```
///
/// With only 25 candidates, a human analyst can quickly scan the list
/// and identify the one that reads as coherent English.  For automated
/// detection, see [`frequency_analysis`].
///
/// # Examples
///
/// ```
/// use caesar_cipher::analysis::brute_force;
///
/// let results = brute_force("KHOOR");
/// // Shift 3 should produce "HELLO"
/// assert_eq!(results[2].shift, 3);
/// assert_eq!(results[2].plaintext, "HELLO");
/// assert_eq!(results.len(), 25);
/// ```
pub fn brute_force(ciphertext: &str) -> Vec<BruteForceResult> {
    (1..=25)
        .map(|shift| BruteForceResult {
            shift,
            plaintext: cipher::decrypt(ciphertext, shift),
        })
        .collect()
}

// ===================================================================
// Frequency analysis attack
// ===================================================================

/// Use chi-squared frequency analysis to find the most likely shift.
///
/// Returns a tuple of `(best_shift, best_plaintext)`.  The function
/// tries all 25 non-trivial shifts, scores each candidate against the
/// known English letter frequency distribution, and returns the one
/// with the lowest chi-squared value.
///
/// # Algorithm step by step
///
/// 1. For each shift `s` in 1..=25:
///    a. Decrypt the ciphertext with shift `s`.
///    b. Count the frequency of each letter A..Z in the candidate.
///    c. Compute the expected count for each letter:
///       `expected_i = total_letters * ENGLISH_FREQUENCIES[i]`
///    d. Compute chi-squared:
///       `chi2 = SUM (observed_i - expected_i)^2 / expected_i`
/// 2. Return the shift with the smallest chi-squared value.
///
/// # Limitations
///
/// - Works best on longer texts (50+ characters).  Short texts may not
///   have enough statistical signal.
/// - Assumes the plaintext is English.  Other languages have different
///   letter frequency distributions.
/// - If the ciphertext contains no alphabetic characters, falls back to
///   shift 1 (since there is no frequency signal at all).
///
/// # Worked example
///
/// ```text
/// ciphertext = "KHOOR ZRUOG"  (encrypted with shift 3)
///
/// shift 1 -> "JGNNQ YQTNF"  chi2 = 84.2  (poor fit)
/// shift 2 -> "IFMMP XPSME"  chi2 = 71.5  (poor fit)
/// shift 3 -> "HELLO WORLD"  chi2 =  8.1  (good fit!) <-- winner
/// shift 4 -> "GDKKN VNQKC"  chi2 = 92.3  (poor fit)
/// ...
/// ```
///
/// # Examples
///
/// ```
/// use caesar_cipher::analysis::frequency_analysis;
///
/// // Use a longer text for reliable frequency analysis
/// let ciphertext = "WKH TXLFN EURZQ IRA MXPSV RYHU WKH ODCB GRJ";
/// let (shift, plaintext) = frequency_analysis(ciphertext);
/// assert_eq!(shift, 3);
/// assert_eq!(plaintext, "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG");
/// ```
pub fn frequency_analysis(ciphertext: &str) -> (i32, String) {
    // Start with shift 1 as the default so that even when all candidates
    // tie (e.g. no alphabetic characters), we return a valid shift and
    // the correctly "decrypted" (unchanged) text.
    let first_candidate = cipher::decrypt(ciphertext, 1);
    let mut best_shift = 1_i32;
    let mut best_score = chi_squared(&first_candidate);
    let mut best_plaintext = first_candidate;

    for shift in 2..=25 {
        let candidate = cipher::decrypt(ciphertext, shift);
        let score = chi_squared(&candidate);

        if score < best_score {
            best_score = score;
            best_shift = shift;
            best_plaintext = candidate;
        }
    }

    (best_shift, best_plaintext)
}

// ===================================================================
// Internal helpers
// ===================================================================

/// Count the occurrences of each letter A..Z in `text` (case-insensitive).
///
/// Returns an array of 26 counts, where index 0 is A and index 25 is Z.
///
/// ```text
/// letter_counts("Hello") => [0,0,0,0,1,0,0,1,0,0,0,2,0,0,1,0,0,0,0,0,0,0,0,0,0,0]
///                            A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
/// ```
fn letter_counts(text: &str) -> [usize; 26] {
    let mut counts = [0_usize; 26];
    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            let index = ch.to_ascii_uppercase() as usize - 'A' as usize;
            counts[index] += 1;
        }
    }
    counts
}

/// Compute the chi-squared statistic comparing the letter distribution
/// of `text` against the expected English distribution.
///
/// A lower value means a closer fit to English.
///
/// ```text
/// chi2 = SUM_i  (observed_i - expected_i)^2 / expected_i
/// ```
///
/// If the text has no alphabetic characters, returns `f64::MAX` so that
/// this candidate is never chosen as the best fit.
fn chi_squared(text: &str) -> f64 {
    let counts = letter_counts(text);
    let total: usize = counts.iter().sum();

    // No letters at all -> no frequency signal.
    if total == 0 {
        return f64::MAX;
    }

    let total_f = total as f64;

    counts
        .iter()
        .enumerate()
        .map(|(i, &observed)| {
            let expected = total_f * ENGLISH_FREQUENCIES[i];
            // Guard against division by zero (extremely rare in practice,
            // since all 26 English frequencies are non-zero).
            if expected < 1e-10 {
                0.0
            } else {
                let diff = observed as f64 - expected;
                diff * diff / expected
            }
        })
        .sum()
}

// ===================================================================
// Unit tests
// ===================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letter_counts_basic() {
        let counts = letter_counts("Hello");
        assert_eq!(counts[4], 1);  // E
        assert_eq!(counts[7], 1);  // H
        assert_eq!(counts[11], 2); // L
        assert_eq!(counts[14], 1); // O
    }

    #[test]
    fn letter_counts_ignores_non_alpha() {
        let counts = letter_counts("A1B2C3!!!");
        assert_eq!(counts[0], 1); // A
        assert_eq!(counts[1], 1); // B
        assert_eq!(counts[2], 1); // C
        let total: usize = counts.iter().sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn chi_squared_empty_is_max() {
        assert_eq!(chi_squared("123!!!"), f64::MAX);
    }

    #[test]
    fn frequencies_sum_to_one() {
        let sum: f64 = ENGLISH_FREQUENCIES.iter().sum();
        assert!((sum - 1.0).abs() < 0.01, "Frequencies should sum to ~1.0, got {}", sum);
    }
}
