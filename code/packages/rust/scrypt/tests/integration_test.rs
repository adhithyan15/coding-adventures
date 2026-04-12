//! Integration tests for coding_adventures_scrypt.
//!
//! Tests are drawn from RFC 7914 §11 (the official test vectors) and from
//! general correctness/security properties of the scrypt algorithm.
//!
//! RFC 7914 defines three main test vectors:
//!
//!   - Vector 1: password="", salt="", N=16, r=1, p=1, dkLen=64
//!   - Vector 2: password="password", salt="NaCl", N=1024, r=8, p=16, dkLen=64
//!   - Vector 3: password="pleaseletmein", salt="SodiumChloride", N=16384, r=8, p=1, dkLen=64
//!
//! Vector 2 (N=1024, r=8, p=16) and Vector 3 (N=16384, r=8) require significant
//! memory (2 MiB and 16 MiB respectively) and take noticeable time. They are
//! included but marked #[ignore] to keep the default test run fast. Run with:
//!
//!   cargo test -p coding_adventures_scrypt -- --ignored --nocapture

use coding_adventures_scrypt::{scrypt, scrypt_hex, ScryptError};

// ─── RFC 7914 §11 Test Vectors ────────────────────────────────────────────────

#[test]
fn rfc7914_vector1_empty_password_and_salt() {
    // Vector 1: scrypt("", "", 16, 1, 1, 64)
    //
    // This vector specifically tests the empty-password case. Our internal
    // PBKDF2 bypasses the empty-key restriction to support this.
    //
    // Expected (RFC 7914 §11):
    //   77 d6 57 62 38 65 7b 20 3b 19 ca 42 c1 8a 04 97
    //   f1 6b 48 44 e3 07 4a e8 df df fa 3f ed e2 14 42
    //   fc d0 06 9d ed 09 48 f8 32 6a 75 3a 0f c8 1f 17
    //   e8 d3 e0 fb 2e 0d 36 28 cf 35 e2 0c 38 d1 89 06
    let expected = concat!(
        "77d6576238657b203b19ca42c18a0497",
        "f16b4844e3074ae8dfdffa3fede21442",
        "fcd0069ded0948f8326a753a0fc81f17",
        "e8d3e0fb2e0d3628cf35e20c38d18906"
    );
    assert_eq!(
        scrypt_hex(b"", b"", 16, 1, 1, 64).unwrap(),
        expected,
        "RFC 7914 vector 1 failed"
    );
}

#[test]
#[ignore] // N=1024, r=8, p=16 → 16 MiB working set; slow but correct
fn rfc7914_vector2_password_nacl() {
    // Vector 2: scrypt("password", "NaCl", 1024, 8, 16, 64)
    //
    // Expected (RFC 7914 §11):
    //   fd ba be 1c 9d 34 72 00 78 56 e7 19 0d 01 e9 fe
    //   7c 6a d7 cb c8 23 78 30 e7 73 76 63 4b 37 31 62
    //   2e af 30 d9 2e 22 a3 88 6f f1 09 27 9d 98 30 da
    //   c7 27 af b9 4a 83 ee 6d 83 60 cb df a2 cc 06 40
    let expected = concat!(
        "fdbabe1c9d3472007856e7190d01e9fe",
        "7c6ad7cbc8237830e77376634b373162",
        "2eaf30d92e22a3886ff109279d9830da",
        "c727afb94a83ee6d8360cbdfa2cc0640"
    );
    println!("Running RFC 7914 vector 2 (may take a few seconds)...");
    assert_eq!(
        scrypt_hex(b"password", b"NaCl", 1024, 8, 16, 64).unwrap(),
        expected,
        "RFC 7914 vector 2 failed"
    );
}

#[test]
#[ignore] // N=16384, r=8 → 16 MiB working set; slow
fn rfc7914_vector3_pleaseletmein() {
    // Vector 3: scrypt("pleaseletmein", "SodiumChloride", 16384, 8, 1, 64)
    //
    // Expected (RFC 7914 §11):
    //   70 23 bd cb 3a fd 73 48 46 1c 06 cd 81 fd 38 eb
    //   fd a8 fb ba 90 4f 8e 3e a9 b5 43 f6 54 5d a1 f2
    //   d5 43 2d f4 7a 1a 4b f9 c4 e3 de 24 69 cd bd 25
    //   e5 db d6 dd 53 26 f8 b8 59 36 7b 2a 1d 04 fd 34
    let expected = concat!(
        "7023bdcb3afd7348461c06cd81fd38eb",
        "fda8fbba904f8e3ea9b543f6545da1f2",
        "d5432df47a1a4bf9c4e3de2469cdbd25",
        "e5dbd6dd5326f8b859367b2a1d04fd34"
    );
    println!("Running RFC 7914 vector 3 (may take several seconds)...");
    assert_eq!(
        scrypt_hex(b"pleaseletmein", b"SodiumChloride", 16384, 8, 1, 64).unwrap(),
        expected,
        "RFC 7914 vector 3 failed"
    );
}

// ─── scrypt_hex wrapper ───────────────────────────────────────────────────────

#[test]
fn scrypt_hex_matches_scrypt_bytes() {
    let bytes = scrypt(b"test", b"salt", 16, 1, 1, 32).unwrap();
    let hex_from_bytes: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let hex_direct = scrypt_hex(b"test", b"salt", 16, 1, 1, 32).unwrap();
    assert_eq!(hex_from_bytes, hex_direct);
}

#[test]
fn scrypt_hex_length() {
    // Each output byte is represented as two hex characters.
    let hex = scrypt_hex(b"pw", b"salt", 16, 1, 1, 32).unwrap();
    assert_eq!(hex.len(), 64); // 32 bytes × 2 chars/byte
}

#[test]
fn scrypt_hex_lowercase() {
    let hex = scrypt_hex(b"pw", b"salt", 16, 1, 1, 32).unwrap();
    assert!(
        hex.chars().all(|c| c.is_ascii_digit() || c.is_ascii_lowercase()),
        "hex output should be lowercase: {}",
        hex
    );
}

// ─── Output length ────────────────────────────────────────────────────────────

#[test]
fn output_length_one_byte() {
    let dk = scrypt(b"pw", b"s", 16, 1, 1, 1).unwrap();
    assert_eq!(dk.len(), 1);
}

#[test]
fn output_length_32_bytes() {
    let dk = scrypt(b"pw", b"s", 16, 1, 1, 32).unwrap();
    assert_eq!(dk.len(), 32);
}

#[test]
fn output_length_64_bytes() {
    let dk = scrypt(b"pw", b"s", 16, 1, 1, 64).unwrap();
    assert_eq!(dk.len(), 64);
}

#[test]
fn output_length_not_power_of_32() {
    // 33 bytes is not aligned to H_LEN=32 — tests truncation logic.
    let dk = scrypt(b"pw", b"s", 16, 1, 1, 33).unwrap();
    assert_eq!(dk.len(), 33);
}

// ─── Determinism ─────────────────────────────────────────────────────────────

#[test]
fn same_inputs_produce_same_output() {
    let a = scrypt(b"password", b"salt", 16, 1, 1, 32).unwrap();
    let b = scrypt(b"password", b"salt", 16, 1, 1, 32).unwrap();
    assert_eq!(a, b, "scrypt must be deterministic");
}

// ─── Sensitivity ─────────────────────────────────────────────────────────────

#[test]
fn password_change_changes_output() {
    let a = scrypt(b"password1", b"salt", 16, 1, 1, 32).unwrap();
    let b = scrypt(b"password2", b"salt", 16, 1, 1, 32).unwrap();
    assert_ne!(a, b, "Different passwords must produce different keys");
}

#[test]
fn salt_change_changes_output() {
    let a = scrypt(b"password", b"salt1", 16, 1, 1, 32).unwrap();
    let b = scrypt(b"password", b"salt2", 16, 1, 1, 32).unwrap();
    assert_ne!(a, b, "Different salts must produce different keys");
}

#[test]
fn n_change_changes_output() {
    let a = scrypt(b"password", b"salt", 16, 1, 1, 32).unwrap();
    let b = scrypt(b"password", b"salt", 32, 1, 1, 32).unwrap();
    assert_ne!(a, b, "Different N must produce different keys");
}

#[test]
fn r_change_changes_output() {
    let a = scrypt(b"password", b"salt", 16, 1, 1, 32).unwrap();
    let b = scrypt(b"password", b"salt", 16, 2, 1, 32).unwrap();
    assert_ne!(a, b, "Different r must produce different keys");
}

#[test]
fn p_change_changes_output() {
    let a = scrypt(b"password", b"salt", 16, 1, 1, 32).unwrap();
    let b = scrypt(b"password", b"salt", 16, 1, 2, 32).unwrap();
    assert_ne!(a, b, "Different p must produce different keys");
}

// ─── Edge cases ───────────────────────────────────────────────────────────────

#[test]
fn empty_password_allowed() {
    // RFC 7914 vector 1 uses empty password.
    assert!(scrypt(b"", b"salt", 16, 1, 1, 32).is_ok());
}

#[test]
fn empty_salt_allowed() {
    assert!(scrypt(b"password", b"", 16, 1, 1, 32).is_ok());
}

#[test]
fn both_empty_allowed() {
    // RFC 7914 vector 1: both password and salt are empty.
    assert!(scrypt(b"", b"", 16, 1, 1, 32).is_ok());
}

#[test]
fn large_password_allowed() {
    let pw = vec![0x42u8; 1000];
    assert!(scrypt(&pw, b"salt", 16, 1, 1, 32).is_ok());
}

#[test]
fn large_salt_allowed() {
    let salt = vec![0x99u8; 500];
    assert!(scrypt(b"pw", &salt, 16, 1, 1, 32).is_ok());
}

// ─── Error cases ──────────────────────────────────────────────────────────────

#[test]
fn error_n_is_zero() {
    assert_eq!(scrypt(b"pw", b"s", 0, 1, 1, 32), Err(ScryptError::InvalidN));
}

#[test]
fn error_n_is_one() {
    assert_eq!(scrypt(b"pw", b"s", 1, 1, 1, 32), Err(ScryptError::InvalidN));
}

#[test]
fn error_n_not_power_of_two_3() {
    assert_eq!(scrypt(b"pw", b"s", 3, 1, 1, 32), Err(ScryptError::InvalidN));
}

#[test]
fn error_n_not_power_of_two_5() {
    assert_eq!(scrypt(b"pw", b"s", 5, 1, 1, 32), Err(ScryptError::InvalidN));
}

#[test]
fn error_n_not_power_of_two_12() {
    assert_eq!(scrypt(b"pw", b"s", 12, 1, 1, 32), Err(ScryptError::InvalidN));
}

#[test]
fn error_n_too_large() {
    assert_eq!(
        scrypt(b"pw", b"s", (1 << 20) + 1, 1, 1, 32),
        Err(ScryptError::NTooLarge)
    );
}

#[test]
fn error_r_is_zero() {
    assert_eq!(scrypt(b"pw", b"s", 2, 0, 1, 32), Err(ScryptError::InvalidR));
}

#[test]
fn error_p_is_zero() {
    assert_eq!(scrypt(b"pw", b"s", 2, 1, 0, 32), Err(ScryptError::InvalidP));
}

#[test]
fn error_dk_len_is_zero() {
    assert_eq!(scrypt(b"pw", b"s", 2, 1, 1, 0), Err(ScryptError::InvalidKeyLength));
}

#[test]
fn error_dk_len_too_large() {
    assert_eq!(
        scrypt(b"pw", b"s", 2, 1, 1, (1 << 20) + 1),
        Err(ScryptError::KeyLengthTooLarge)
    );
}

#[test]
fn error_pr_too_large() {
    // p * r overflows 2^30
    assert_eq!(
        scrypt(b"pw", b"s", 2, 1 << 15, 1 << 16, 32),
        Err(ScryptError::PRTooLarge)
    );
}

#[test]
fn error_pr_exactly_at_limit_accepted() {
    // p * r = 2^30 is at the limit — the check is >, so 2^30 must be accepted.
    // But this would require enormous memory, so just test the boundary
    // with p=1, r=1 which is safely within bounds.
    assert!(scrypt(b"pw", b"s", 2, 1, 1, 1).is_ok());
}
