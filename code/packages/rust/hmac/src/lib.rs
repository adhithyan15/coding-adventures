//! # coding_adventures_hmac
//!
//! HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1.
//!
//! ## What Is HMAC?
//!
//! HMAC takes a secret key and a message and produces a fixed-size authentication
//! tag proving both message integrity and authenticity. It is used in TLS 1.2/1.3,
//! JWT (HS256, HS512), WPA2, TOTP/HOTP, and AWS Signature V4.
//!
//! ## Why Not hash(key || message)?
//!
//! Naively prepending the key is vulnerable to **length extension attacks** on
//! Merkle-Damgård hashes (MD5, SHA-1, SHA-256, SHA-512). An attacker who knows
//! `hash(key || message)` can compute `hash(key || message || padding || extra)`
//! without knowing `key`, because they can resume the hash function's state.
//!
//! HMAC defeats this with two nested hash calls under different padded keys:
//!
//! ```text
//! HMAC(K, M) = H((K' XOR opad) || H((K' XOR ipad) || M))
//! ```
//!
//! where `ipad = 0x36` repeated and `opad = 0x5C` repeated to the block size.
//!
//! ## The Algorithm (RFC 2104 §2)
//!
//! 1. Normalize key to `block_size` bytes:
//!    - `len(key) > block_size` → `K' = H(key)`, then zero-pad to `block_size`
//!    - `len(key) ≤ block_size` → zero-pad to `block_size`
//! 2. `inner_key = K' XOR (0x36 * block_size)`
//! 3. `outer_key = K' XOR (0x5C * block_size)`
//! 4. `inner = H(inner_key || message)`
//! 5. Return `H(outer_key || inner)`
//!
//! ## Block Sizes
//!
//! | Algorithm | Block (bytes) | Digest (bytes) |
//! |-----------|--------------|----------------|
//! | MD5       | 64           | 16             |
//! | SHA-1     | 64           | 20             |
//! | SHA-256   | 64           | 32             |
//! | SHA-512   | 128          | 64             |
//!
//! ## RFC 4231 Test Vector (TC1, HMAC-SHA256)
//!
//! ```
//! use coding_adventures_hmac::hmac_sha256_hex;
//! let key = vec![0x0bu8; 20];
//! assert_eq!(
//!     hmac_sha256_hex(&key, b"Hi There"),
//!     "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
//! );
//! ```

use coding_adventures_md5::sum_md5;
use coding_adventures_sha1::sum1;
use coding_adventures_sha256::sha256;
use coding_adventures_sha512::sum512;

// ─── ipad and opad constants (RFC 2104 §2) ───────────────────────────────────
//
// ipad = 0x36 = 0011_0110  (inner pad)
// opad = 0x5C = 0101_1100  (outer pad)
//
// These values were chosen because they differ in 4 of 8 bits — maximum
// Hamming distance for single-byte values — ensuring the inner and outer
// keys are as different as possible even though both are derived from K'.
const IPAD: u8 = 0x36;
const OPAD: u8 = 0x5C;

// ─── Generic HMAC ─────────────────────────────────────────────────────────────

/// Compute HMAC using any hash function.
///
/// # Parameters
/// - `hash_fn`: a function `&[u8] -> Vec<u8>` (e.g. a closure wrapping `sha256`)
/// - `block_size`: internal block size of `hash_fn` in bytes (64 or 128)
/// - `key`: secret key, any length
/// - `message`: data to authenticate, any length
///
/// # Returns
/// The authentication tag as `Vec<u8>`. Same length as `hash_fn`'s output.
pub fn hmac<F>(hash_fn: F, block_size: usize, key: &[u8], message: &[u8]) -> Vec<u8>
where
    F: Fn(&[u8]) -> Vec<u8>,
{
    // Step 1 — normalize key to exactly block_size bytes
    let key_prime = normalize_key(&hash_fn, block_size, key);

    // Step 2 — derive inner and outer padded keys
    let inner_key: Vec<u8> = key_prime.iter().map(|&b| b ^ IPAD).collect();
    let outer_key: Vec<u8> = key_prime.iter().map(|&b| b ^ OPAD).collect();

    // Step 3 — nested hashes
    let mut inner_input = inner_key;
    inner_input.extend_from_slice(message);
    let inner = hash_fn(&inner_input);

    let mut outer_input = outer_key;
    outer_input.extend_from_slice(&inner);
    hash_fn(&outer_input)
}

// ─── Named variants ───────────────────────────────────────────────────────────

/// HMAC-MD5: 16-byte authentication tag.
///
/// HMAC-MD5 remains secure as a MAC even though MD5 is broken for collision
/// resistance — MAC security is a different property.
pub fn hmac_md5(key: &[u8], message: &[u8]) -> [u8; 16] {
    let result = hmac(|d| sum_md5(d).to_vec(), 64, key, message);
    result.try_into().expect("hmac_md5 must produce 16 bytes")
}

/// HMAC-SHA1: 20-byte authentication tag.
///
/// Used in WPA2 (PBKDF2-HMAC-SHA1), SSH, and TOTP/HOTP.
pub fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    let result = hmac(|d| sum1(d).to_vec(), 64, key, message);
    result.try_into().expect("hmac_sha1 must produce 20 bytes")
}

/// HMAC-SHA256: 32-byte authentication tag.
///
/// The modern default for TLS 1.3, JWT HS256, AWS Signature V4.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let result = hmac(|d| sha256(d).to_vec(), 64, key, message);
    result.try_into().expect("hmac_sha256 must produce 32 bytes")
}

/// HMAC-SHA512: 64-byte authentication tag.
///
/// Used in JWT HS512 and high-security configurations.
/// Note: SHA-512 has a 128-byte block, so ipad/opad are 128 bytes.
pub fn hmac_sha512(key: &[u8], message: &[u8]) -> [u8; 64] {
    let result = hmac(|d| sum512(d).to_vec(), 128, key, message);
    result.try_into().expect("hmac_sha512 must produce 64 bytes")
}

// ─── Hex-string variants ──────────────────────────────────────────────────────

/// HMAC-MD5 as a 32-character lowercase hex string.
pub fn hmac_md5_hex(key: &[u8], message: &[u8]) -> String {
    bytes_to_hex(&hmac_md5(key, message))
}

/// HMAC-SHA1 as a 40-character lowercase hex string.
pub fn hmac_sha1_hex(key: &[u8], message: &[u8]) -> String {
    bytes_to_hex(&hmac_sha1(key, message))
}

/// HMAC-SHA256 as a 64-character lowercase hex string.
pub fn hmac_sha256_hex(key: &[u8], message: &[u8]) -> String {
    bytes_to_hex(&hmac_sha256(key, message))
}

/// HMAC-SHA512 as a 128-character lowercase hex string.
pub fn hmac_sha512_hex(key: &[u8], message: &[u8]) -> String {
    bytes_to_hex(&hmac_sha512(key, message))
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Normalize key to exactly `block_size` bytes.
/// Long keys are hashed. Short keys are zero-padded.
fn normalize_key<F>(hash_fn: &F, block_size: usize, key: &[u8]) -> Vec<u8>
where
    F: Fn(&[u8]) -> Vec<u8>,
{
    let hashed;
    let key_bytes = if key.len() > block_size {
        hashed = hash_fn(key);
        hashed.as_slice()
    } else {
        key
    };

    let mut result = vec![0u8; block_size];
    let copy_len = key_bytes.len().min(block_size);
    result[..copy_len].copy_from_slice(&key_bytes[..copy_len]);
    result
}

/// Encode a byte slice as a lowercase hex string.
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // RFC 4231 — HMAC-SHA256

    #[test]
    fn hmac_sha256_tc1() {
        let key = vec![0x0bu8; 20];
        assert_eq!(
            hmac_sha256_hex(&key, b"Hi There"),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn hmac_sha256_tc2() {
        assert_eq!(
            hmac_sha256_hex(b"Jefe", b"what do ya want for nothing?"),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn hmac_sha256_tc3() {
        let key = vec![0xaau8; 20];
        let data = vec![0xddu8; 50];
        assert_eq!(
            hmac_sha256_hex(&key, &data),
            "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe"
        );
    }

    #[test]
    fn hmac_sha256_tc6_long_key() {
        let key = vec![0xaau8; 131];
        assert_eq!(
            hmac_sha256_hex(&key, b"Test Using Larger Than Block-Size Key - Hash Key First"),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn hmac_sha256_tc7() {
        let key = vec![0xaau8; 131];
        let data = b"This is a test using a larger than block-size key and a larger than block-size data. The key needs to be hashed before being used by the HMAC algorithm.";
        assert_eq!(
            hmac_sha256_hex(&key, data),
            "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2"
        );
    }

    // RFC 4231 — HMAC-SHA512

    #[test]
    fn hmac_sha512_tc1() {
        let key = vec![0x0bu8; 20];
        assert_eq!(
            hmac_sha512_hex(&key, b"Hi There"),
            "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
        );
    }

    #[test]
    fn hmac_sha512_tc2() {
        assert_eq!(
            hmac_sha512_hex(b"Jefe", b"what do ya want for nothing?"),
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737"
        );
    }

    #[test]
    fn hmac_sha512_tc6_long_key() {
        let key = vec![0xaau8; 131];
        assert_eq!(
            hmac_sha512_hex(&key, b"Test Using Larger Than Block-Size Key - Hash Key First"),
            "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598"
        );
    }

    // RFC 2202 — HMAC-MD5

    #[test]
    fn hmac_md5_tc1() {
        let key = vec![0x0bu8; 16];
        assert_eq!(hmac_md5_hex(&key, b"Hi There"), "9294727a3638bb1c13f48ef8158bfc9d");
    }

    #[test]
    fn hmac_md5_tc2() {
        assert_eq!(
            hmac_md5_hex(b"Jefe", b"what do ya want for nothing?"),
            "750c783e6ab0b503eaa86e310a5db738"
        );
    }

    #[test]
    fn hmac_md5_tc6_long_key() {
        let key = vec![0xaau8; 80];
        assert_eq!(
            hmac_md5_hex(&key, b"Test Using Larger Than Block-Size Key - Hash Key First"),
            "6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd"
        );
    }

    // RFC 2202 — HMAC-SHA1

    #[test]
    fn hmac_sha1_tc1() {
        let key = vec![0x0bu8; 20];
        assert_eq!(
            hmac_sha1_hex(&key, b"Hi There"),
            "b617318655057264e28bc0b6fb378c8ef146be00"
        );
    }

    #[test]
    fn hmac_sha1_tc2() {
        assert_eq!(
            hmac_sha1_hex(b"Jefe", b"what do ya want for nothing?"),
            "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
        );
    }

    #[test]
    fn hmac_sha1_tc6_long_key() {
        let key = vec![0xaau8; 80];
        assert_eq!(
            hmac_sha1_hex(&key, b"Test Using Larger Than Block-Size Key - Hash Key First"),
            "aa4ae5e15272d00e95705637ce8a3b55ed402112"
        );
    }

    // Return lengths

    #[test]
    fn return_lengths() {
        assert_eq!(hmac_md5(b"k", b"m").len(), 16);
        assert_eq!(hmac_sha1(b"k", b"m").len(), 20);
        assert_eq!(hmac_sha256(b"k", b"m").len(), 32);
        assert_eq!(hmac_sha512(b"k", b"m").len(), 64);
    }

    // Key handling

    #[test]
    fn empty_key_and_message() {
        assert_eq!(hmac_sha256(b"", b"").len(), 32);
        assert_eq!(hmac_sha512(b"", b"").len(), 64);
    }

    #[test]
    fn key_longer_than_block_hashed() {
        let k65 = vec![0x01u8; 65];
        let k66 = vec![0x01u8; 66];
        assert_ne!(hmac_sha256(&k65, b"msg"), hmac_sha256(&k66, b"msg"));
    }

    // Authentication properties

    #[test]
    fn deterministic() {
        assert_eq!(hmac_sha256(b"k", b"m"), hmac_sha256(b"k", b"m"));
    }

    #[test]
    fn key_sensitivity() {
        assert_ne!(hmac_sha256(b"k1", b"m"), hmac_sha256(b"k2", b"m"));
    }

    #[test]
    fn message_sensitivity() {
        assert_ne!(hmac_sha256(b"k", b"m1"), hmac_sha256(b"k", b"m2"));
    }

    #[test]
    fn hex_matches_bytes() {
        let tag = hmac_sha256(b"k", b"m");
        let hex = hmac_sha256_hex(b"k", b"m");
        assert_eq!(hex, bytes_to_hex(&tag));
    }
}
