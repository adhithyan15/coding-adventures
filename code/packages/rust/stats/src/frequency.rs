// # Frequency Analysis
//
// This module provides tools for analyzing the frequency of letters in text.
// These functions are the foundation of classical cryptanalysis — most cipher
// attacks rely on comparing observed letter frequencies against expected ones.

use std::collections::HashMap;

/// # Frequency Count
///
/// Counts how many times each letter (A-Z) appears in the text.
/// Non-alphabetic characters are ignored. Counting is case-insensitive.
///
/// ## Example
///
/// ```text
/// frequency_count("Hello!") => {'H': 1, 'E': 1, 'L': 2, 'O': 1}
/// ```
///
pub fn frequency_count(text: &str) -> HashMap<char, usize> {
    let mut counts = HashMap::new();

    for ch in text.chars() {
        if ch.is_ascii_alphabetic() {
            let upper = ch.to_ascii_uppercase();
            *counts.entry(upper).or_insert(0) += 1;
        }
    }

    counts
}

/// # Frequency Distribution
///
/// Converts raw letter counts into proportions (0.0 to 1.0). This
/// normalizes the data so texts of different lengths can be compared.
///
/// ## Formula
///
/// ```text
/// proportion(letter) = count(letter) / total_letter_count
/// ```
///
pub fn frequency_distribution(text: &str) -> HashMap<char, f64> {
    let counts = frequency_count(text);

    let total: usize = counts.values().sum();

    let mut distribution = HashMap::new();
    if total > 0 {
        for (&letter, &count) in &counts {
            distribution.insert(letter, count as f64 / total as f64);
        }
    }

    distribution
}

/// # Chi-Squared Statistic
///
/// Measures how well observed data matches expected data. A value of 0
/// means perfect agreement; larger values indicate greater divergence.
///
/// ## Formula
///
/// ```text
/// chi2 = sum( (observed_i - expected_i)^2 / expected_i )
/// ```
///
/// ## Example
///
/// ```text
/// chi_squared(&[10.0, 20.0, 30.0], &[20.0, 20.0, 20.0]) = 10.0
/// ```
///
pub fn chi_squared(observed: &[f64], expected: &[f64]) -> f64 {
    assert_eq!(
        observed.len(),
        expected.len(),
        "Observed and expected arrays must have the same length"
    );
    assert!(!observed.is_empty(), "Arrays must not be empty");

    let mut chi2 = 0.0;
    for i in 0..observed.len() {
        assert!(
            expected[i] != 0.0,
            "Expected value at index {} must not be zero",
            i
        );
        let diff = observed[i] - expected[i];
        chi2 += (diff * diff) / expected[i];
    }

    chi2
}

/// # Chi-Squared for Text
///
/// Convenience function that computes chi-squared of a text against
/// an expected frequency table (like ENGLISH_FREQUENCIES).
///
/// This is how you break a Caesar cipher: try all 26 shifts, compute
/// chi-squared for each, and pick the shift with the lowest value.
///
pub fn chi_squared_text(text: &str, expected_freq: &HashMap<char, f64>) -> f64 {
    let counts = frequency_count(text);

    let total: usize = counts.values().sum();
    if total == 0 {
        return 0.0;
    }

    let mut chi2 = 0.0;
    for code in b'A'..=b'Z' {
        let letter = code as char;
        let observed = *counts.get(&letter).unwrap_or(&0) as f64;
        let expected = total as f64 * expected_freq.get(&letter).unwrap_or(&0.0);

        if expected > 0.0 {
            let diff = observed - expected;
            chi2 += (diff * diff) / expected;
        }
    }

    chi2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_count_basic() {
        let counts = frequency_count("Hello!");
        assert_eq!(*counts.get(&'H').unwrap(), 1);
        assert_eq!(*counts.get(&'E').unwrap(), 1);
        assert_eq!(*counts.get(&'L').unwrap(), 2);
        assert_eq!(*counts.get(&'O').unwrap(), 1);
    }

    #[test]
    fn test_frequency_count_ignores_non_alpha() {
        let counts = frequency_count("123!@#");
        assert!(counts.is_empty());
    }

    #[test]
    fn test_frequency_count_empty() {
        let counts = frequency_count("");
        assert!(counts.is_empty());
    }

    #[test]
    fn test_frequency_distribution_proportions() {
        let dist = frequency_distribution("AABB");
        assert!((dist.get(&'A').unwrap() - 0.5).abs() < 1e-10);
        assert!((dist.get(&'B').unwrap() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_frequency_distribution_sums_to_one() {
        let dist = frequency_distribution("HELLO WORLD");
        let sum: f64 = dist.values().sum();
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_frequency_distribution_empty() {
        let dist = frequency_distribution("");
        assert!(dist.is_empty());
    }

    #[test]
    fn test_chi_squared_parity() {
        let result = chi_squared(&[10.0, 20.0, 30.0], &[20.0, 20.0, 20.0]);
        assert!((result - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_chi_squared_perfect_match() {
        let result = chi_squared(&[20.0, 20.0, 20.0], &[20.0, 20.0, 20.0]);
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    #[should_panic(expected = "same length")]
    fn test_chi_squared_mismatched_lengths() {
        chi_squared(&[1.0, 2.0], &[1.0]);
    }

    #[test]
    #[should_panic(expected = "empty")]
    fn test_chi_squared_empty() {
        chi_squared(&[], &[]);
    }

    #[test]
    #[should_panic(expected = "zero")]
    fn test_chi_squared_zero_expected() {
        chi_squared(&[1.0], &[0.0]);
    }

    #[test]
    fn test_chi_squared_text_perfect() {
        let mut freq = HashMap::new();
        freq.insert('A', 1.0);
        let result = chi_squared_text("AAAA", &freq);
        assert!((result - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_chi_squared_text_empty() {
        let freq = HashMap::new();
        assert_eq!(chi_squared_text("", &freq), 0.0);
    }
}
