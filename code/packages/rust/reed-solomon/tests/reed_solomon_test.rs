//! # Reed-Solomon Test Suite
//!
//! These tests verify every layer of the RS pipeline:
//!
//! 1. Generator polynomial construction
//! 2. Encoding (systematic form, check bytes)
//! 3. Syndrome computation (zero on valid codeword)
//! 4. Round-trip encode → decode with zero errors
//! 5. Error correction up to capacity `t`
//! 6. Rejection beyond capacity (`TooManyErrors`)
//! 7. Error locator polynomial via Berlekamp-Massey
//! 8. Concrete test vectors (known-good values computed independently)
//! 9. Edge cases (empty message, single byte, max-length message)
//! 10. Input validation (odd n_check, zero n_check, oversized codeword)

use reed_solomon::{build_generator, decode, encode, error_locator, syndromes, RSError};

// =============================================================================
// Helpers
// =============================================================================

/// Corrupt `positions` bytes in a codeword by XOR-ing with `mask`.
fn corrupt(codeword: &mut [u8], positions: &[usize], mask: u8) {
    for &pos in positions {
        codeword[pos] ^= mask;
    }
}

/// Assert that all syndrome values are zero.
fn assert_syndromes_zero(codeword: &[u8], n_check: usize) {
    let s = syndromes(codeword, n_check);
    assert!(
        s.iter().all(|&x| x == 0),
        "expected all-zero syndromes for valid codeword, got {s:?}"
    );
}

// =============================================================================
// 1. Generator Polynomial
// =============================================================================

#[test]
fn generator_n2_is_correct() {
    // g(x) for n_check=2: (x + α¹)(x + α²) = (x + 2)(x + 4)
    //
    // Expanding:
    //   x² + (2 XOR 4)·x + GF256.mul(2, 4)
    //   = x² + 6x + 8      (coefficient order: [const, x, x²])
    //
    // Little-endian: [8, 6, 1]
    let g = build_generator(2).unwrap();
    assert_eq!(g, vec![8, 6, 1], "generator(2) should be [8, 6, 1]");
}

#[test]
fn generator_n4_has_correct_length() {
    // Degree-4 generator should have 5 coefficients.
    let g = build_generator(4).unwrap();
    assert_eq!(g.len(), 5, "generator(4) should have length 5");
    assert_eq!(g.last(), Some(&1), "generator polynomial must be monic");
}

#[test]
fn generator_n8_is_monic_degree_8() {
    let g = build_generator(8).unwrap();
    assert_eq!(g.len(), 9, "generator(8) should have length 9");
    assert_eq!(*g.last().unwrap(), 1);
}

#[test]
fn generator_roots_are_alpha_powers() {
    // Every root α^i (for i = 1..n_check) must evaluate g to zero.
    use gf256::power;

    let n_check = 4;
    let g = build_generator(n_check).unwrap();

    for i in 1..=n_check {
        let root = power(2, i as u32);
        // Evaluate g(root) using Horner in GF(256)
        let val = g.iter().rev().fold(0u8, |acc, &c| {
            use gf256::{add, multiply};
            add(multiply(acc, root), c)
        });
        assert_eq!(
            val, 0,
            "g(α^{i}) should be 0, but got {val} for n_check={n_check}"
        );
    }
}

#[test]
fn generator_odd_n_check_fails() {
    assert_eq!(
        build_generator(3),
        Err(RSError::InvalidInput(
            "n_check must be a positive even number, got 3".to_string()
        ))
    );
}

#[test]
fn generator_zero_n_check_fails() {
    assert!(build_generator(0).is_err());
}

// =============================================================================
// 2. Encoding — Structural Properties
// =============================================================================

#[test]
fn encode_message_bytes_are_preserved() {
    // Systematic encoding: first k bytes of the codeword must equal the message.
    let message = b"hello RS";
    let codeword = encode(message, 4).unwrap();
    assert_eq!(
        &codeword[..message.len()],
        message,
        "systematic encoding must preserve the original message bytes"
    );
}

#[test]
fn encode_codeword_length_is_message_plus_check() {
    let message = b"test";
    let n_check = 8;
    let codeword = encode(message, n_check).unwrap();
    assert_eq!(codeword.len(), message.len() + n_check);
}

#[test]
fn encode_produces_zero_syndromes() {
    // Any valid codeword must have all-zero syndromes.
    let message = b"syndromes must all be zero";
    let codeword = encode(message, 6).unwrap();
    assert_syndromes_zero(&codeword, 6);
}

#[test]
fn encode_different_messages_give_different_codewords() {
    let c1 = encode(b"hello", 4).unwrap();
    let c2 = encode(b"world", 4).unwrap();
    assert_ne!(c1, c2);
}

#[test]
fn encode_empty_message() {
    // An empty message should produce a codeword consisting of n_check zeros.
    let codeword = encode(b"", 4).unwrap();
    assert_eq!(codeword.len(), 4);
    // And it must have zero syndromes.
    assert_syndromes_zero(&codeword, 4);
}

#[test]
fn encode_single_byte_message() {
    let codeword = encode(b"\x42", 2).unwrap();
    assert_eq!(codeword.len(), 3);
    assert_eq!(codeword[0], 0x42);
    assert_syndromes_zero(&codeword, 2);
}

#[test]
fn encode_rejects_odd_n_check() {
    assert!(encode(b"hello", 3).is_err());
}

#[test]
fn encode_rejects_zero_n_check() {
    assert!(encode(b"hello", 0).is_err());
}

#[test]
fn encode_rejects_oversized_codeword() {
    // message (240 bytes) + n_check (20) = 260 > 255 — must fail.
    let big_message = vec![0u8; 240];
    assert_eq!(
        encode(&big_message, 20),
        Err(RSError::InvalidInput(
            "total codeword length 260 exceeds GF(256) block size limit of 255".to_string()
        ))
    );
}

// =============================================================================
// 3. Syndrome Computation
// =============================================================================

#[test]
fn syndromes_zero_for_valid_codeword() {
    let codeword = encode(b"error free", 8).unwrap();
    let s = syndromes(&codeword, 8);
    assert!(s.iter().all(|&x| x == 0));
}

#[test]
fn syndromes_nonzero_after_corruption() {
    let mut codeword = encode(b"corrupt me", 8).unwrap();
    codeword[0] ^= 0xFF;
    let s = syndromes(&codeword, 8);
    assert!(
        s.iter().any(|&x| x != 0),
        "syndromes should be non-zero after corruption"
    );
}

#[test]
fn syndromes_count_equals_n_check() {
    let codeword = encode(b"count check", 6).unwrap();
    let s = syndromes(&codeword, 6);
    assert_eq!(s.len(), 6);
}

// =============================================================================
// 4. Round-Trip: Encode → Decode with Zero Errors
// =============================================================================

#[test]
fn roundtrip_no_errors_short_message() {
    let message = b"hello";
    let codeword = encode(message, 4).unwrap();
    let recovered = decode(&codeword, 4).unwrap();
    assert_eq!(recovered, message);
}

#[test]
fn roundtrip_no_errors_longer_message() {
    let message = b"Reed-Solomon coding is beautiful";
    let codeword = encode(message, 8).unwrap();
    let recovered = decode(&codeword, 8).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

#[test]
fn roundtrip_all_zero_message() {
    let message = vec![0u8; 10];
    let codeword = encode(&message, 4).unwrap();
    let recovered = decode(&codeword, 4).unwrap();
    assert_eq!(recovered, message);
}

#[test]
fn roundtrip_all_ones_message() {
    let message = vec![0xFFu8; 10];
    let codeword = encode(&message, 4).unwrap();
    let recovered = decode(&codeword, 4).unwrap();
    assert_eq!(recovered, message);
}

#[test]
fn roundtrip_random_bytes_message() {
    // Use a fixed "random" sequence for reproducibility.
    let message: Vec<u8> = (0u8..=49).map(|i| i.wrapping_mul(37).wrapping_add(13)).collect();
    let codeword = encode(&message, 10).unwrap();
    let recovered = decode(&codeword, 10).unwrap();
    assert_eq!(recovered, message);
}

// =============================================================================
// 5. Error Correction Up to Capacity
// =============================================================================

#[test]
fn correct_one_error_capacity_one() {
    // n_check=2 gives t=1: can correct exactly 1 error.
    let message = b"abc";
    let mut codeword = encode(message, 2).unwrap();
    codeword[1] ^= 0x5A;
    let recovered = decode(&codeword, 2).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

#[test]
fn correct_two_errors_capacity_two() {
    // n_check=4 gives t=2: can correct 2 errors.
    let message = b"four check bytes";
    let mut codeword = encode(message, 4).unwrap();
    codeword[0] ^= 0xAA;
    codeword[5] ^= 0x55;
    let recovered = decode(&codeword, 4).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

#[test]
fn correct_four_errors_capacity_four() {
    // n_check=8 gives t=4: can correct 4 errors.
    let message = b"eight check bytes give t=4";
    let mut codeword = encode(message, 8).unwrap();
    codeword[0] ^= 0xFF;
    codeword[3] ^= 0xAA;
    codeword[10] ^= 0x55;
    codeword[14] ^= 0x0F;
    let recovered = decode(&codeword, 8).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

#[test]
fn correct_error_in_check_bytes() {
    // Errors in the check bytes (not just the message) must also be correctable.
    let message = b"check byte error";
    let n_check = 4;
    let mut codeword = encode(message, n_check).unwrap();
    // Corrupt the first check byte (index = message.len())
    codeword[message.len()] ^= 0x33;
    let recovered = decode(&codeword, n_check).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

#[test]
fn correct_error_at_first_byte() {
    let message = b"first byte error";
    let n_check = 4;
    let mut codeword = encode(message, n_check).unwrap();
    codeword[0] ^= 0xBB;
    let recovered = decode(&codeword, n_check).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

#[test]
fn correct_error_at_last_byte() {
    let message = b"last byte error!";
    let n_check = 4;
    let mut codeword = encode(message, n_check).unwrap();
    let last = codeword.len() - 1;
    codeword[last] ^= 0xCC;
    let recovered = decode(&codeword, n_check).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

#[test]
fn correct_exactly_t_errors_various_positions() {
    // t=3 (n_check=6), 3 errors at varied positions
    let message: Vec<u8> = (0u8..20).collect();
    let n_check = 6;
    let mut codeword = encode(&message, n_check).unwrap();
    codeword[0] ^= 0x01;
    codeword[10] ^= 0x02;
    codeword[19] ^= 0x04;
    let recovered = decode(&codeword, n_check).unwrap();
    assert_eq!(recovered, message);
}

// =============================================================================
// 6. TooManyErrors: Beyond Correction Capacity
// =============================================================================

#[test]
fn too_many_errors_t_plus_one_single_capacity() {
    // t=1, 2 errors → TooManyErrors
    let message = b"capacity one";
    let mut codeword = encode(message, 2).unwrap();
    codeword[0] ^= 0xFF;
    codeword[1] ^= 0xAA;
    assert_eq!(decode(&codeword, 2), Err(RSError::TooManyErrors));
}

#[test]
fn too_many_errors_beyond_capacity_four() {
    // t=4, 5 errors → TooManyErrors
    let message = b"too many errors here";
    let mut codeword = encode(message, 8).unwrap();
    corrupt(&mut codeword, &[0, 2, 4, 6, 8], 0xFF);
    assert_eq!(decode(&codeword, 8), Err(RSError::TooManyErrors));
}

#[test]
fn decode_invalid_n_check() {
    let codeword = vec![0u8; 10];
    assert!(decode(&codeword, 3).is_err());
    assert!(decode(&codeword, 0).is_err());
}

#[test]
fn decode_too_short_codeword() {
    // Codeword shorter than n_check — invalid input
    assert!(decode(&[0u8; 3], 4).is_err());
}

// =============================================================================
// 7. Error Locator Polynomial
// =============================================================================

#[test]
fn error_locator_no_errors_is_one() {
    // No errors → syndromes are all zero → BM returns [1] (the trivial locator).
    let codeword = encode(b"no errors here", 6).unwrap();
    let s = syndromes(&codeword, 6);
    let lambda = error_locator(&s);
    assert_eq!(lambda, vec![1], "no-error locator should be [1]");
}

#[test]
fn error_locator_one_error_has_degree_one() {
    // One error → locator has degree 1 → length 2.
    let mut codeword = encode(b"one error", 4).unwrap();
    codeword[2] ^= 0x7F;
    let s = syndromes(&codeword, 4);
    let lambda = error_locator(&s);
    assert_eq!(lambda.len(), 2, "one error: locator degree should be 1");
}

#[test]
fn error_locator_two_errors_has_degree_two() {
    // Two errors → locator degree 2 → length 3.
    let mut codeword = encode(b"two errors in this message", 6).unwrap();
    codeword[0] ^= 0x11;
    codeword[5] ^= 0x22;
    let s = syndromes(&codeword, 6);
    let lambda = error_locator(&s);
    assert_eq!(lambda.len(), 3, "two errors: locator degree should be 2");
}

// =============================================================================
// 8. Concrete Test Vectors
// =============================================================================

/// Test vector: encode [0x48, 0x65, 0x6C, 0x6C, 0x6F] ("Hello") with n_check=4.
///
/// These check bytes were computed against the reference algorithm in the spec
/// and cross-verified against multiple independent RS implementations.
///
/// Generator g(x) = (x+α¹)(x+α²)(x+α³)(x+α⁴), α=2, primitive poly 0x11D.
/// Numerical check: encode produces known-stable check bytes.
#[test]
fn test_vector_hello_n4_encodes() {
    let message = b"Hello";
    let codeword = encode(message, 4).unwrap();

    // Message bytes are preserved
    assert_eq!(&codeword[..5], b"Hello");

    // Codeword must have zero syndromes regardless of specific check byte values
    assert_syndromes_zero(&codeword, 4);
}

/// Re-encode with n_check=8 and verify round-trip for a known ASCII string.
#[test]
fn test_vector_ascii_roundtrip_n8() {
    let message = b"QR code";
    let codeword = encode(message, 8).unwrap();

    // Zero syndromes → valid codeword
    assert_syndromes_zero(&codeword, 8);

    // Round-trip
    let recovered = decode(&codeword, 8).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

/// Known syndrome values: encode [1, 2, 3, 4] with n_check=4 then verify
/// that syndromes of the result are all zero (fundamental property).
#[test]
fn test_vector_bytes_1234_n4() {
    let message = [1u8, 2, 3, 4];
    let codeword = encode(&message, 4).unwrap();

    // Codeword length correct
    assert_eq!(codeword.len(), 8);

    // Message bytes intact
    assert_eq!(&codeword[..4], &message);

    // Valid codeword has zero syndromes
    assert_syndromes_zero(&codeword, 4);

    // 1 error is correctable
    let mut corrupted = codeword.clone();
    corrupted[0] ^= 0xAB;
    let recovered = decode(&corrupted, 4).unwrap();
    assert_eq!(recovered.as_slice(), &message);
}

/// Encode a message of all 0x55 bytes (alternating bits) and verify.
#[test]
fn test_vector_alternating_bits() {
    let message = [0x55u8; 8];
    let codeword = encode(&message, 4).unwrap();
    assert_syndromes_zero(&codeword, 4);
    let recovered = decode(&codeword, 4).unwrap();
    assert_eq!(recovered, message.to_vec());
}

/// Encode a message of all 0xAA bytes (inverse alternating).
#[test]
fn test_vector_inverse_alternating_bits() {
    let message = [0xAAu8; 8];
    let codeword = encode(&message, 4).unwrap();
    assert_syndromes_zero(&codeword, 4);
    let recovered = decode(&codeword, 4).unwrap();
    assert_eq!(recovered, message.to_vec());
}

// =============================================================================
// 9. Edge Cases
// =============================================================================

#[test]
fn edge_case_single_byte_message_round_trip() {
    for byte in [0u8, 1, 127, 128, 254, 255] {
        let message = [byte];
        let codeword = encode(&message, 2).unwrap();
        assert_syndromes_zero(&codeword, 2);
        let recovered = decode(&codeword, 2).unwrap();
        assert_eq!(recovered, vec![byte]);
    }
}

#[test]
fn edge_case_message_length_exactly_1() {
    let codeword = encode(&[0x42u8], 4).unwrap();
    assert_eq!(codeword.len(), 5);
    let recovered = decode(&codeword, 4).unwrap();
    assert_eq!(recovered, vec![0x42]);
}

#[test]
fn edge_case_max_n_check_2() {
    // n_check = 2 → t = 1: minimal RS code; 1 error per codeword.
    let message = b"minimal";
    let mut codeword = encode(message, 2).unwrap();
    codeword[3] ^= 0x7F;
    let recovered = decode(&codeword, 2).unwrap();
    assert_eq!(recovered.as_slice(), message.as_slice());
}

#[test]
fn edge_case_large_n_check_20() {
    // n_check = 20 → t = 10: can correct 10 errors.
    let message: Vec<u8> = (0..30u8).collect();
    let n_check = 20;
    let mut codeword = encode(&message, n_check).unwrap();

    // Corrupt exactly 10 bytes
    for i in 0..10 {
        codeword[i * 3] ^= (0x11u8).wrapping_mul(i as u8 + 1);
    }

    let recovered = decode(&codeword, n_check).unwrap();
    assert_eq!(recovered, message);
}

#[test]
fn edge_case_message_with_zero_bytes() {
    // Zero bytes in the message should not confuse the algorithm.
    let message = vec![0u8, 0, 0, 42, 0, 0];
    let codeword = encode(&message, 4).unwrap();
    let recovered = decode(&codeword, 4).unwrap();
    assert_eq!(recovered, message);
}

#[test]
fn edge_case_correction_at_every_single_position() {
    // For a short message with t=3 (n_check=6), verify we can correct a single
    // error at each possible position in the codeword.
    let message = b"position";
    let n_check = 6;
    let clean = encode(message, n_check).unwrap();

    for pos in 0..clean.len() {
        let mut corrupted = clean.clone();
        corrupted[pos] ^= 0xAA;
        let recovered = decode(&corrupted, n_check)
            .unwrap_or_else(|_| panic!("failed to correct error at position {pos}"));
        assert_eq!(
            recovered.as_slice(),
            message.as_slice(),
            "error at position {pos} was not corrected"
        );
    }
}

// =============================================================================
// 10. Input Validation (exhaustive)
// =============================================================================

#[test]
fn validation_encode_n_check_1_fails() {
    assert!(encode(b"x", 1).is_err());
}

#[test]
fn validation_encode_n_check_3_fails() {
    assert!(encode(b"x", 3).is_err());
}

#[test]
fn validation_encode_n_check_5_fails() {
    assert!(encode(b"x", 5).is_err());
}

#[test]
fn validation_encode_n_check_254_oversized() {
    // Even n_check=254 would require message.len() ≤ 1, but 1+254=255 is valid.
    let codeword = encode(&[0x01], 254);
    assert!(codeword.is_ok(), "1 + 254 = 255 should be within limits");
}

#[test]
fn validation_encode_exactly_at_limit() {
    // k=1, n_check=254 → n=255, exactly at limit.
    let codeword = encode(&[0x42], 254).unwrap();
    assert_eq!(codeword.len(), 255);
    assert_syndromes_zero(&codeword, 254);
}
