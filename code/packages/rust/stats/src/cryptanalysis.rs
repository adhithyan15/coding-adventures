// # Cryptanalysis Helpers
//
// This module provides tools for analyzing the structure of text that
// are essential in breaking classical ciphers:
//
// - **Index of Coincidence (IC):** Distinguishes English from random text.
// - **Shannon Entropy:** Measures information content per symbol.
// - **English Frequencies:** Standard letter frequency table for English.

use std::collections::HashMap;

use crate::frequency::frequency_count;

/// # English Letter Frequencies
///
/// Standard frequencies of each letter (A-Z) in typical English text.
/// These come from large-corpus analysis and are the foundation of
/// frequency-based cryptanalysis.
///
/// The mnemonic "ETAOIN SHRDLU" captures the most common letters.
///
pub fn english_frequencies() -> HashMap<char, f64> {
    let mut freq = HashMap::new();
    freq.insert('A', 0.08167);
    freq.insert('B', 0.01492);
    freq.insert('C', 0.02782);
    freq.insert('D', 0.04253);
    freq.insert('E', 0.12702);
    freq.insert('F', 0.02228);
    freq.insert('G', 0.02015);
    freq.insert('H', 0.06094);
    freq.insert('I', 0.06966);
    freq.insert('J', 0.00153);
    freq.insert('K', 0.00772);
    freq.insert('L', 0.04025);
    freq.insert('M', 0.02406);
    freq.insert('N', 0.06749);
    freq.insert('O', 0.07507);
    freq.insert('P', 0.01929);
    freq.insert('Q', 0.00095);
    freq.insert('R', 0.05987);
    freq.insert('S', 0.06327);
    freq.insert('T', 0.09056);
    freq.insert('U', 0.02758);
    freq.insert('V', 0.00978);
    freq.insert('W', 0.02360);
    freq.insert('X', 0.00150);
    freq.insert('Y', 0.01974);
    freq.insert('Z', 0.00074);
    freq
}

/// # Index of Coincidence (IC)
///
/// The IC measures the probability that two randomly chosen letters from
/// a text are the same.
///
/// ## Formula
///
/// ```text
/// IC = sum(n_i * (n_i - 1)) / (N * (N - 1))
/// ```
///
/// ## Expected Values
///
/// | Text Type        | IC Value          |
/// |-----------------|-------------------|
/// | English text     | ~0.0667           |
/// | Random (uniform) | ~0.0385 (= 1/26) |
///
/// ## Example
///
/// ```text
/// index_of_coincidence("AABB")
/// // A=2, B=2, N=4
/// // IC = (2*1 + 2*1) / (4*3) = 4/12 = 0.333...
/// ```
///
pub fn index_of_coincidence(text: &str) -> f64 {
    let counts = frequency_count(text);

    let n: usize = counts.values().sum();

    if n < 2 {
        return 0.0;
    }

    let numerator: usize = counts.values().map(|&c| c * (c.saturating_sub(1))).sum();
    let denominator = n * (n - 1);

    numerator as f64 / denominator as f64
}

/// # Shannon Entropy
///
/// Shannon entropy measures the average "information content" per symbol.
/// It answers: "How many bits do we need, on average, to encode each symbol?"
///
/// ## Formula
///
/// ```text
/// H = -sum(p_i * log2(p_i))
/// ```
///
/// ## Expected Values
///
/// | Distribution      | Entropy            |
/// |------------------|--------------------|
/// | Uniform 26 chars  | log2(26) ~ 4.700   |
/// | English text      | ~4.0 - 4.5         |
/// | Single letter     | 0.0                |
///
pub fn entropy(text: &str) -> f64 {
    let counts = frequency_count(text);

    let total: usize = counts.values().sum();
    if total == 0 {
        return 0.0;
    }

    let total_f = total as f64;
    let mut h = 0.0;

    for &count in counts.values() {
        if count > 0 {
            let p = count as f64 / total_f;
            h -= p * p.log2();
        }
    }

    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ic_parity_aabb() {
        let ic = index_of_coincidence("AABB");
        assert!((ic - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_ic_single_letter_repeated() {
        let ic = index_of_coincidence("AAAA");
        assert!((ic - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_ic_too_short() {
        assert_eq!(index_of_coincidence("A"), 0.0);
        assert_eq!(index_of_coincidence(""), 0.0);
    }

    #[test]
    fn test_ic_uniform_alphabet() {
        let ic = index_of_coincidence("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        assert_eq!(ic, 0.0);
    }

    #[test]
    fn test_entropy_uniform_26() {
        let alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let h = entropy(alphabet);
        assert!((h - 26.0_f64.log2()).abs() < 0.01);
    }

    #[test]
    fn test_entropy_single_letter() {
        assert!((entropy("AAAA") - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_entropy_two_equal() {
        let h = entropy("AABB");
        assert!((h - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_entropy_empty() {
        assert_eq!(entropy(""), 0.0);
    }

    #[test]
    fn test_english_frequencies_has_26() {
        let freq = english_frequencies();
        assert_eq!(freq.len(), 26);
    }

    #[test]
    fn test_english_frequencies_sum_to_one() {
        let freq = english_frequencies();
        let sum: f64 = freq.values().sum();
        assert!((sum - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_english_frequencies_e_most_common() {
        let freq = english_frequencies();
        assert!(freq[&'E'] > freq[&'T']);
    }
}
