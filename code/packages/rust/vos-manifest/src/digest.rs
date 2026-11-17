//! # Digests
//!
//! One function, so that every digest in the tools is the same SHA-256 (FIPS 180-4,
//! the repository's own `coding_adventures_sha256`) printed as 64 lowercase hex
//! characters.  The VOS02 rule is: every comparison that decides a gate uses the
//! **full** 64-hex digest.

/// SHA-256 of `bytes`, as 64 lowercase hex characters.
pub fn sha256_hex(bytes: &[u8]) -> String {
    coding_adventures_sha256::sha256_hex(bytes)
}

/// True when `s` is exactly 64 lowercase hex characters.
pub fn is_hex64(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
