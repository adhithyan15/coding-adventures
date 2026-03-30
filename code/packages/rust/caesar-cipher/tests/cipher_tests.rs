//! Integration tests for the cipher module.
//!
//! These tests exercise the public API of `caesar_cipher::cipher` from the
//! outside, just as a downstream consumer would use the crate.

use caesar_cipher::cipher;

// ===================================================================
// Encrypt tests
// ===================================================================

#[test]
fn encrypt_hello_shift_3() {
    // The classic example: HELLO -> KHOOR with shift 3.
    assert_eq!(cipher::encrypt("HELLO", 3), "KHOOR");
}

#[test]
fn encrypt_preserves_lowercase() {
    assert_eq!(cipher::encrypt("hello", 3), "khoor");
}

#[test]
fn encrypt_preserves_mixed_case() {
    assert_eq!(cipher::encrypt("Hello, World!", 3), "Khoor, Zruog!");
}

#[test]
fn encrypt_non_alpha_passthrough() {
    // Digits, spaces, and punctuation should not change.
    assert_eq!(cipher::encrypt("123 !@#", 5), "123 !@#");
}

#[test]
fn encrypt_empty_string() {
    assert_eq!(cipher::encrypt("", 10), "");
}

#[test]
fn encrypt_shift_zero_is_identity() {
    let text = "The quick brown fox jumps over 13 lazy dogs!";
    assert_eq!(cipher::encrypt(text, 0), text);
}

#[test]
fn encrypt_shift_26_is_identity() {
    let text = "Wrap around test";
    assert_eq!(cipher::encrypt(text, 26), text);
}

#[test]
fn encrypt_shift_52_is_identity() {
    // Two full rotations.
    let text = "Double wrap";
    assert_eq!(cipher::encrypt(text, 52), text);
}

#[test]
fn encrypt_negative_shift() {
    // shift -1: A->Z, B->A, C->B
    assert_eq!(cipher::encrypt("ABC", -1), "ZAB");
}

#[test]
fn encrypt_large_negative_shift() {
    // shift -27 is equivalent to shift -1 (mod 26) which is shift 25.
    assert_eq!(cipher::encrypt("ABC", -27), "ZAB");
}

#[test]
fn encrypt_shift_wraps_end_of_alphabet() {
    // X, Y, Z should wrap to A, B, C with shift 3.
    assert_eq!(cipher::encrypt("XYZ", 3), "ABC");
}

#[test]
fn encrypt_full_alphabet_shift_1() {
    assert_eq!(
        cipher::encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 1),
        "BCDEFGHIJKLMNOPQRSTUVWXYZA"
    );
}

#[test]
fn encrypt_full_alphabet_shift_13() {
    assert_eq!(
        cipher::encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 13),
        "NOPQRSTUVWXYZABCDEFGHIJKLM"
    );
}

// ===================================================================
// Decrypt tests
// ===================================================================

#[test]
fn decrypt_basic() {
    assert_eq!(cipher::decrypt("KHOOR", 3), "HELLO");
}

#[test]
fn decrypt_preserves_case() {
    assert_eq!(cipher::decrypt("Khoor, Zruog!", 3), "Hello, World!");
}

#[test]
fn decrypt_empty_string() {
    assert_eq!(cipher::decrypt("", 7), "");
}

// ===================================================================
// Round-trip tests
// ===================================================================

#[test]
fn round_trip_all_shifts() {
    let original = "The Quick Brown Fox Jumps Over The Lazy Dog! 123";
    for shift in -30..=30 {
        let encrypted = cipher::encrypt(original, shift);
        let decrypted = cipher::decrypt(&encrypted, shift);
        assert_eq!(decrypted, original, "Round-trip failed for shift {}", shift);
    }
}

#[test]
fn round_trip_with_unicode() {
    // Non-ASCII characters should pass through unchanged.
    let original = "Cafe\u{0301} costs $3.50 \u{2764}";
    let encrypted = cipher::encrypt(original, 7);
    let decrypted = cipher::decrypt(&encrypted, 7);
    assert_eq!(decrypted, original);
}

// ===================================================================
// ROT13 tests
// ===================================================================

#[test]
fn rot13_basic() {
    assert_eq!(cipher::rot13("Hello"), "Uryyb");
}

#[test]
fn rot13_self_inverse() {
    let texts = vec![
        "Hello, World!",
        "The Quick Brown Fox",
        "abcdefghijklmnopqrstuvwxyz",
        "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        "Mixed Case 123!",
        "",
    ];
    for text in texts {
        assert_eq!(
            cipher::rot13(&cipher::rot13(text)),
            text,
            "ROT13 self-inverse failed for {:?}",
            text
        );
    }
}

#[test]
fn rot13_non_alpha_unchanged() {
    assert_eq!(cipher::rot13("123 !@# ..."), "123 !@# ...");
}

#[test]
fn rot13_is_encrypt_13() {
    let text = "Equivalent to encrypt(text, 13)";
    assert_eq!(cipher::rot13(text), cipher::encrypt(text, 13));
}
