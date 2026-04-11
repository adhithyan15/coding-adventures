//! # PBKDF2 — Password-Based Key Derivation Function 2 (RFC 8018)
//!
//! ## What Is PBKDF2?
//!
//! PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
//! function (PRF) — typically HMAC — `c` times per output block. The iteration
//! count `c` is the tunable cost parameter: every password guess during a
//! brute-force attack requires the same `c` PRF calls.
//!
//! Real-world uses:
//! - WPA2 Wi-Fi — PBKDF2-HMAC-SHA1, 4096 iterations
//! - Django password hasher — PBKDF2-HMAC-SHA256, 720,000 iterations (2024)
//! - macOS Keychain — PBKDF2-HMAC-SHA256
//! - LUKS disk encryption — PBKDF2 with configurable hash
//!
//! ## Algorithm (RFC 8018 § 5.2)
//!
//! ```text
//! DK = T_1 || T_2 || ... || T_⌈dkLen/hLen⌉    (first dkLen bytes)
//!
//! T_i = U_1 XOR U_2 XOR ... XOR U_c
//!
//! U_1 = PRF(Password, Salt || INT_32_BE(i))
//! U_j = PRF(Password, U_{j-1})   for j = 2..c
//! ```
//!
//! The block index `i` is encoded as a 4-byte big-endian integer appended to
//! the salt. This makes each block's first U value unique.
//!
//! ## Example
//!
//! ```rust
//! use coding_adventures_pbkdf2::{pbkdf2_hmac_sha256, Pbkdf2Error};
//!
//! let dk = pbkdf2_hmac_sha256(b"password", b"salt", 1, 20).unwrap();
//! // RFC 7914 Appendix B — first 20 bytes:
//! assert_eq!(dk.len(), 20);
//! ```

use coding_adventures_hmac::{hmac_sha1, hmac_sha256, hmac_sha512};

// ─────────────────────────────────────────────────────────────────────────────
// Error type
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that PBKDF2 functions can return.
#[derive(Debug, PartialEq, Eq)]
pub enum Pbkdf2Error {
    /// Password has zero length. An empty password provides no entropy.
    EmptyPassword,
    /// Iteration count is zero or would overflow. Must be ≥ 1.
    InvalidIterations,
    /// Key length is zero or exceeds the 2^20 practical upper bound.
    InvalidKeyLength,
    /// Key length exceeds the practical 2^20 limit (1 MiB). RFC 8018 § 5.2
    /// imposes (2^32−1)×hLen as the hard cap; we apply a tighter bound to
    /// prevent memory-exhaustion DoS and integer-overflow in block counting.
    KeyLengthTooLarge,
    /// The underlying PRF returned an error. This should not happen when the
    /// password is non-empty (validated before calling the PRF), but is
    /// returned rather than panicking if the HMAC implementation ever changes.
    PrfError,
}

impl std::fmt::Display for Pbkdf2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Pbkdf2Error::EmptyPassword => write!(f, "pbkdf2: password must not be empty"),
            Pbkdf2Error::InvalidIterations => write!(f, "pbkdf2: iterations must be positive"),
            Pbkdf2Error::InvalidKeyLength => write!(f, "pbkdf2: key_length must be positive"),
            Pbkdf2Error::KeyLengthTooLarge => write!(f, "pbkdf2: key_length must not exceed 2^20 (1 MiB)"),
            Pbkdf2Error::PrfError => write!(f, "pbkdf2: pseudorandom function returned an error"),
        }
    }
}

impl std::error::Error for Pbkdf2Error {}

// ─────────────────────────────────────────────────────────────────────────────
// Core loop (generic over the PRF closure)
// ─────────────────────────────────────────────────────────────────────────────

/// Generic PBKDF2 — used internally by all public convenience functions.
///
/// `prf(key, msg)` must return exactly `h_len` bytes or propagate an error.
/// Using a fallible closure (`Result`) instead of a panicking one ensures that
/// PRF failures surface as a `Pbkdf2Error::PrfError` rather than an
/// unrecoverable panic — important because library panics cannot be caught
/// across FFI boundaries.
fn pbkdf2_core<F>(
    prf: F,
    h_len: usize,
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    key_length: usize,
) -> Result<Vec<u8>, Pbkdf2Error>
where
    F: Fn(&[u8], &[u8]) -> Result<Vec<u8>, Pbkdf2Error>,
{
    if password.is_empty() {
        return Err(Pbkdf2Error::EmptyPassword);
    }
    if iterations == 0 {
        return Err(Pbkdf2Error::InvalidIterations);
    }
    if key_length == 0 {
        return Err(Pbkdf2Error::InvalidKeyLength);
    }
    // Enforce a practical upper bound to prevent memory-exhaustion DoS and to
    // guarantee the num_blocks cast to u32 cannot truncate silently.
    // RFC 8018 § 5.2 hard cap is (2^32−1)×hLen; we apply 2^20 (1 MiB).
    if key_length > 1 << 20 {
        return Err(Pbkdf2Error::KeyLengthTooLarge);
    }

    // Number of h_len-sized output blocks needed.
    // Safe: key_length ≤ 2^20 and h_len ≤ 64, so num_blocks ≤ 2^20 / 1 = 2^20 ≤ u32::MAX.
    let num_blocks = key_length.div_ceil(h_len);
    let mut dk = Vec::with_capacity(num_blocks * h_len);

    for i in 1u32..=(num_blocks as u32) {
        // Seed = Salt || INT_32_BE(i)
        // INT_32_BE encodes the block counter as a 4-byte big-endian integer.
        let mut seed = Vec::with_capacity(salt.len() + 4);
        seed.extend_from_slice(salt);
        seed.extend_from_slice(&i.to_be_bytes());

        // U_1 = PRF(Password, Seed)
        let u = prf(password, &seed)?;

        // t accumulates the XOR of all U values for this block.
        let mut t = u.clone();

        // U_j = PRF(Password, U_{j-1}), XOR each into t.
        let mut prev = u;
        for _ in 1..iterations {
            let next = prf(password, &prev)?;
            for (a, b) in t.iter_mut().zip(next.iter()) {
                *a ^= b;
            }
            prev = next;
        }

        dk.extend_from_slice(&t);
    }

    dk.truncate(key_length);
    Ok(dk)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API — concrete PRF variants
// ─────────────────────────────────────────────────────────────────────────────

/// PBKDF2 with HMAC-SHA1 as the PRF.
///
/// hLen = 20 bytes (160-bit SHA-1 output).
/// Used in WPA2 (4096 iterations). For new systems prefer SHA-256.
///
/// # RFC 6070 test vector
/// ```rust
/// use coding_adventures_pbkdf2::pbkdf2_hmac_sha1_hex;
/// let h = pbkdf2_hmac_sha1_hex(b"password", b"salt", 1, 20).unwrap();
/// assert_eq!(h, "0c60c80f961f0e71f3a9b524af6012062fe037a6");
/// ```
pub fn pbkdf2_hmac_sha1(
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    key_length: usize,
) -> Result<Vec<u8>, Pbkdf2Error> {
    // Validate before entering the loop to avoid calling HMAC with an empty
    // key (which would also error, but Pbkdf2Error is cleaner here).
    if password.is_empty() {
        return Err(Pbkdf2Error::EmptyPassword);
    }
    pbkdf2_core(
        |key, msg| hmac_sha1(key, msg).map(|v| v.to_vec()).map_err(|_| Pbkdf2Error::PrfError),
        20,
        password,
        salt,
        iterations,
        key_length,
    )
}

/// PBKDF2 with HMAC-SHA256 as the PRF.
///
/// hLen = 32 bytes (256-bit SHA-256 output).
/// Recommended for new systems (OWASP 2023: ≥ 600,000 iterations).
///
/// # RFC 7914 test vector
/// ```rust
/// use coding_adventures_pbkdf2::pbkdf2_hmac_sha256_hex;
/// let h = pbkdf2_hmac_sha256_hex(b"passwd", b"salt", 1, 64).unwrap();
/// assert!(h.starts_with("55ac046e56e3089f"));
/// ```
pub fn pbkdf2_hmac_sha256(
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    key_length: usize,
) -> Result<Vec<u8>, Pbkdf2Error> {
    if password.is_empty() {
        return Err(Pbkdf2Error::EmptyPassword);
    }
    pbkdf2_core(
        |key, msg| hmac_sha256(key, msg).map(|v| v.to_vec()).map_err(|_| Pbkdf2Error::PrfError),
        32,
        password,
        salt,
        iterations,
        key_length,
    )
}

/// PBKDF2 with HMAC-SHA512 as the PRF.
///
/// hLen = 64 bytes (512-bit SHA-512 output).
pub fn pbkdf2_hmac_sha512(
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    key_length: usize,
) -> Result<Vec<u8>, Pbkdf2Error> {
    if password.is_empty() {
        return Err(Pbkdf2Error::EmptyPassword);
    }
    pbkdf2_core(
        |key, msg| hmac_sha512(key, msg).map(|v| v.to_vec()).map_err(|_| Pbkdf2Error::PrfError),
        64,
        password,
        salt,
        iterations,
        key_length,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Hex convenience wrappers
// ─────────────────────────────────────────────────────────────────────────────

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Like `pbkdf2_hmac_sha1` but returns a lowercase hex string.
pub fn pbkdf2_hmac_sha1_hex(
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    key_length: usize,
) -> Result<String, Pbkdf2Error> {
    pbkdf2_hmac_sha1(password, salt, iterations, key_length).map(|dk| to_hex(&dk))
}

/// Like `pbkdf2_hmac_sha256` but returns a lowercase hex string.
pub fn pbkdf2_hmac_sha256_hex(
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    key_length: usize,
) -> Result<String, Pbkdf2Error> {
    pbkdf2_hmac_sha256(password, salt, iterations, key_length).map(|dk| to_hex(&dk))
}

/// Like `pbkdf2_hmac_sha512` but returns a lowercase hex string.
pub fn pbkdf2_hmac_sha512_hex(
    password: &[u8],
    salt: &[u8],
    iterations: usize,
    key_length: usize,
) -> Result<String, Pbkdf2Error> {
    pbkdf2_hmac_sha512(password, salt, iterations, key_length).map(|dk| to_hex(&dk))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn from_hex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    // ── RFC 6070 PBKDF2-HMAC-SHA1 ──────────────────────────────────────────

    #[test]
    fn rfc6070_sha1_c1() {
        let dk = pbkdf2_hmac_sha1(b"password", b"salt", 1, 20).unwrap();
        assert_eq!(dk, from_hex("0c60c80f961f0e71f3a9b524af6012062fe037a6"));
    }

    #[test]
    fn rfc6070_sha1_c4096() {
        let dk = pbkdf2_hmac_sha1(b"password", b"salt", 4096, 20).unwrap();
        assert_eq!(dk, from_hex("4b007901b765489abead49d926f721d065a429c1"));
    }

    #[test]
    fn rfc6070_sha1_long_password_salt() {
        let dk = pbkdf2_hmac_sha1(
            b"passwordPASSWORDpassword",
            b"saltSALTsaltSALTsaltSALTsaltSALTsalt",
            4096,
            25,
        )
        .unwrap();
        assert_eq!(
            dk,
            from_hex("3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038")
        );
    }

    #[test]
    fn rfc6070_sha1_null_bytes() {
        let dk = pbkdf2_hmac_sha1(b"pass\x00word", b"sa\x00lt", 4096, 16).unwrap();
        assert_eq!(dk, from_hex("56fa6aa75548099dcc37d7f03425e0c3"));
    }

    // ── RFC 7914 PBKDF2-HMAC-SHA256 ────────────────────────────────────────

    #[test]
    fn rfc7914_sha256_c1_64bytes() {
        let dk = pbkdf2_hmac_sha256(b"passwd", b"salt", 1, 64).unwrap();
        let expected = from_hex(
            "55ac046e56e3089fec1691c22544b605\
             f94185216dde0465e68b9d57c20dacbc\
             49ca9cccf179b645991664b39d77ef31\
             7c71b845b1e30bd509112041d3a19783",
        );
        assert_eq!(dk, expected);
    }

    #[test]
    fn sha256_output_length() {
        let dk = pbkdf2_hmac_sha256(b"key", b"salt", 1, 32).unwrap();
        assert_eq!(dk.len(), 32);
    }

    #[test]
    fn sha256_truncation_consistency() {
        let short = pbkdf2_hmac_sha256(b"key", b"salt", 1, 16).unwrap();
        let full = pbkdf2_hmac_sha256(b"key", b"salt", 1, 32).unwrap();
        assert_eq!(short, full[..16]);
    }

    #[test]
    fn sha256_multi_block() {
        let dk64 = pbkdf2_hmac_sha256(b"password", b"salt", 1, 64).unwrap();
        let dk32 = pbkdf2_hmac_sha256(b"password", b"salt", 1, 32).unwrap();
        assert_eq!(dk64.len(), 64);
        assert_eq!(&dk64[..32], &dk32[..]);
    }

    // ── SHA-512 sanity checks ──────────────────────────────────────────────

    #[test]
    fn sha512_output_length() {
        let dk = pbkdf2_hmac_sha512(b"secret", b"nacl", 1, 64).unwrap();
        assert_eq!(dk.len(), 64);
    }

    #[test]
    fn sha512_truncation() {
        let short = pbkdf2_hmac_sha512(b"secret", b"nacl", 1, 32).unwrap();
        let full = pbkdf2_hmac_sha512(b"secret", b"nacl", 1, 64).unwrap();
        assert_eq!(short, full[..32]);
    }

    // ── Hex variants ──────────────────────────────────────────────────────

    #[test]
    fn hex_sha1_rfc6070() {
        let h = pbkdf2_hmac_sha1_hex(b"password", b"salt", 1, 20).unwrap();
        assert_eq!(h, "0c60c80f961f0e71f3a9b524af6012062fe037a6");
    }

    #[test]
    fn hex_sha256_matches_bytes() {
        let dk = pbkdf2_hmac_sha256(b"passwd", b"salt", 1, 32).unwrap();
        let h = pbkdf2_hmac_sha256_hex(b"passwd", b"salt", 1, 32).unwrap();
        assert_eq!(h, to_hex(&dk));
    }

    // ── Validation ────────────────────────────────────────────────────────

    #[test]
    fn empty_password_returns_error() {
        assert_eq!(
            pbkdf2_hmac_sha256(b"", b"salt", 1, 32),
            Err(Pbkdf2Error::EmptyPassword)
        );
        assert_eq!(
            pbkdf2_hmac_sha1(b"", b"salt", 1, 20),
            Err(Pbkdf2Error::EmptyPassword)
        );
        assert_eq!(
            pbkdf2_hmac_sha512(b"", b"salt", 1, 64),
            Err(Pbkdf2Error::EmptyPassword)
        );
    }

    #[test]
    fn zero_iterations_returns_error() {
        assert_eq!(
            pbkdf2_hmac_sha256(b"pw", b"salt", 0, 32),
            Err(Pbkdf2Error::InvalidIterations)
        );
    }

    #[test]
    fn zero_key_length_returns_error() {
        assert_eq!(
            pbkdf2_hmac_sha256(b"pw", b"salt", 1, 0),
            Err(Pbkdf2Error::InvalidKeyLength)
        );
    }

    #[test]
    fn empty_salt_is_allowed() {
        let dk = pbkdf2_hmac_sha256(b"password", b"", 1, 32).unwrap();
        assert_eq!(dk.len(), 32);
    }

    #[test]
    fn deterministic() {
        let a = pbkdf2_hmac_sha256(b"secret", b"nacl", 100, 32).unwrap();
        let b = pbkdf2_hmac_sha256(b"secret", b"nacl", 100, 32).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn different_salts() {
        let a = pbkdf2_hmac_sha256(b"password", b"salt1", 1, 32).unwrap();
        let b = pbkdf2_hmac_sha256(b"password", b"salt2", 1, 32).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_passwords() {
        let a = pbkdf2_hmac_sha256(b"password1", b"salt", 1, 32).unwrap();
        let b = pbkdf2_hmac_sha256(b"password2", b"salt", 1, 32).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn different_iterations() {
        let a = pbkdf2_hmac_sha256(b"password", b"salt", 1, 32).unwrap();
        let b = pbkdf2_hmac_sha256(b"password", b"salt", 2, 32).unwrap();
        assert_ne!(a, b);
    }
}
