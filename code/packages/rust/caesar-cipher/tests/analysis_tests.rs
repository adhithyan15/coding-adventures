//! Integration tests for the analysis module.
//!
//! These tests exercise brute-force and frequency-analysis attacks
//! against known ciphertexts.

use caesar_cipher::analysis;
use caesar_cipher::cipher;

// ===================================================================
// Brute-force tests
// ===================================================================

#[test]
fn brute_force_returns_25_results() {
    let results = analysis::brute_force("KHOOR");
    assert_eq!(results.len(), 25, "Should try all 25 non-trivial shifts");
}

#[test]
fn brute_force_shifts_are_1_through_25() {
    let results = analysis::brute_force("TEST");
    for (i, result) in results.iter().enumerate() {
        assert_eq!(
            result.shift,
            (i + 1) as i32,
            "Result at index {} should have shift {}",
            i,
            i + 1
        );
    }
}

#[test]
fn brute_force_contains_correct_plaintext() {
    let plaintext = "HELLO";
    let ciphertext = cipher::encrypt(plaintext, 3);

    let results = analysis::brute_force(&ciphertext);
    // Shift 3 is at index 2 (since we start from shift 1).
    assert_eq!(results[2].shift, 3);
    assert_eq!(results[2].plaintext, plaintext);
}

#[test]
fn brute_force_empty_string() {
    let results = analysis::brute_force("");
    assert_eq!(results.len(), 25);
    for result in &results {
        assert_eq!(result.plaintext, "");
    }
}

#[test]
fn brute_force_non_alpha() {
    // Non-alphabetic characters should be the same in all candidates.
    let results = analysis::brute_force("123!!!");
    for result in &results {
        assert_eq!(result.plaintext, "123!!!");
    }
}

#[test]
fn brute_force_preserves_case() {
    let ciphertext = cipher::encrypt("Hello World", 7);
    let results = analysis::brute_force(&ciphertext);
    // Shift 7 is at index 6.
    assert_eq!(results[6].plaintext, "Hello World");
}

// ===================================================================
// Frequency analysis tests
// ===================================================================

#[test]
fn frequency_analysis_known_english() {
    // A longer English sentence encrypted with shift 3.
    let plaintext = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG";
    let ciphertext = cipher::encrypt(plaintext, 3);

    let (shift, decoded) = analysis::frequency_analysis(&ciphertext);
    assert_eq!(shift, 3, "Expected shift 3, got shift {}", shift);
    assert_eq!(decoded, plaintext);
}

#[test]
fn frequency_analysis_longer_text() {
    let plaintext = "IN CRYPTOGRAPHY A CAESAR CIPHER ALSO KNOWN AS SHIFT CIPHER \
                     IS ONE OF THE SIMPLEST AND MOST WIDELY KNOWN ENCRYPTION TECHNIQUES \
                     IT IS A TYPE OF SUBSTITUTION CIPHER IN WHICH EACH LETTER IN THE \
                     PLAINTEXT IS REPLACED BY A LETTER SOME FIXED NUMBER OF POSITIONS \
                     DOWN THE ALPHABET";
    let shift = 17;
    let ciphertext = cipher::encrypt(plaintext, shift);

    let (detected_shift, decoded) = analysis::frequency_analysis(&ciphertext);
    assert_eq!(detected_shift, shift);
    assert_eq!(decoded, plaintext);
}

#[test]
fn frequency_analysis_shift_13() {
    let plaintext = "THIS IS A TEST OF THE FREQUENCY ANALYSIS FUNCTION";
    let ciphertext = cipher::encrypt(plaintext, 13);

    let (shift, decoded) = analysis::frequency_analysis(&ciphertext);
    assert_eq!(shift, 13);
    assert_eq!(decoded, plaintext);
}

#[test]
fn frequency_analysis_empty_string() {
    // With no letters, the function should still return without panicking.
    let (shift, decoded) = analysis::frequency_analysis("");
    // The specific shift returned for empty input is implementation-defined
    // but the function must not panic.
    assert!(shift >= 0 && shift <= 25);
    assert_eq!(decoded, "");
}

#[test]
fn frequency_analysis_all_same_letter() {
    // "AAAA..." encrypted with shift 4 gives "EEEE..."
    // Frequency analysis may not find the right answer for such degenerate
    // input, but it should not panic.
    let ciphertext = "EEEEEEEEEEE";
    let (shift, _decoded) = analysis::frequency_analysis(ciphertext);
    // Just verify it returns a valid shift.
    assert!(shift >= 1 && shift <= 25);
}

#[test]
fn frequency_analysis_non_alpha_only() {
    // No alphabetic characters at all.  With no frequency signal, the
    // function defaults to shift 1 (the first candidate).  The decoded
    // text is unchanged because there are no letters to shift.
    let (shift, decoded) = analysis::frequency_analysis("12345!@#$%");
    assert_eq!(shift, 1, "Should default to shift 1 when no alpha chars");
    assert_eq!(decoded, "12345!@#$%");
}

// ===================================================================
// English frequency constant tests
// ===================================================================

#[test]
fn english_frequencies_has_26_entries() {
    assert_eq!(analysis::ENGLISH_FREQUENCIES.len(), 26);
}

#[test]
fn english_frequencies_sum_approximately_one() {
    let sum: f64 = analysis::ENGLISH_FREQUENCIES.iter().sum();
    assert!(
        (sum - 1.0).abs() < 0.01,
        "Frequencies should sum to approximately 1.0, got {}",
        sum
    );
}

#[test]
fn english_frequencies_e_is_most_common() {
    // E is at index 4 and should be the highest frequency.
    let e_freq = analysis::ENGLISH_FREQUENCIES[4];
    for (i, &freq) in analysis::ENGLISH_FREQUENCIES.iter().enumerate() {
        if i != 4 {
            assert!(
                e_freq > freq,
                "E ({}) should be more frequent than letter at index {} ({})",
                e_freq,
                i,
                freq
            );
        }
    }
}

#[test]
fn english_frequencies_all_positive() {
    for (i, &freq) in analysis::ENGLISH_FREQUENCIES.iter().enumerate() {
        assert!(
            freq > 0.0,
            "Frequency for letter {} should be positive, got {}",
            (b'A' + i as u8) as char,
            freq
        );
    }
}
