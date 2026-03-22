//! # sha256sum — Compute and Check SHA-256 Message Digest
//!
//! This module implements the business logic for the `sha256sum` command.
//! SHA-256 computes a 256-bit (32-byte) cryptographic hash, outputting
//! a 64-character hexadecimal string.
//!
//! ## What Is SHA-256?
//!
//! SHA-256 (Secure Hash Algorithm 256-bit) is part of the SHA-2 family
//! designed by the NSA. Unlike MD5, SHA-256 is still considered
//! cryptographically secure for most purposes.
//!
//! ```text
//!     "hello"  → 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
//!     ""       → e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
//! ```
//!
//! ## SHA-256 vs MD5
//!
//! ```text
//!     Property        MD5         SHA-256
//!     ──────────────  ──────────  ────────────
//!     Output size     128 bits    256 bits
//!     Block size      512 bits    512 bits
//!     Rounds          64          64
//!     Security        Broken      Secure
//!     Speed           Faster      Slower
//! ```
//!
//! ## Algorithm Overview
//!
//! SHA-256 processes data in 512-bit (64-byte) blocks:
//!
//! 1. **Pad** the message (similar to MD5 but big-endian length)
//! 2. **Initialize** eight 32-bit state variables (H0..H7)
//! 3. **Expand** each 16-word block into 64 words
//! 4. **Compress** using 64 rounds of mixing
//! 5. **Output** the eight state variables as a 256-bit digest

// ---------------------------------------------------------------------------
// SHA-256 Constants
// ---------------------------------------------------------------------------

/// Initial hash values — the first 32 bits of the fractional parts
/// of the square roots of the first 8 primes (2, 3, 5, 7, 11, 13, 17, 19).
const H_INIT: [u32; 8] = [
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
];

/// Round constants — the first 32 bits of the fractional parts
/// of the cube roots of the first 64 primes (2..311).
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the SHA-256 hash of a byte slice and return it as a
/// 64-character lowercase hexadecimal string.
///
/// # Algorithm Steps
///
/// ```text
///     1. Pad: append 0x80, zeros, then 64-bit big-endian length
///     2. For each 64-byte block:
///        a. Expand 16 words → 64 words using σ0 and σ1
///        b. Initialize a..h from current hash state
///        c. 64 rounds of compression
///        d. Add compressed values back to hash state
///     3. Output H0..H7 as big-endian bytes → hex string
/// ```
///
/// # Example
///
/// ```text
///     compute_sha256(b"hello")
///     → "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
/// ```
pub fn compute_sha256(data: &[u8]) -> String {
    // --- Step 1: Pad the message ---
    let padded = sha256_pad(data);

    // --- Step 2: Initialize hash state ---
    let mut h = H_INIT;

    // --- Step 3: Process each 64-byte block ---
    for chunk in padded.chunks(64) {
        // --- Step 3a: Create message schedule (expand 16 words → 64) ---
        let mut w = [0u32; 64];

        // First 16 words come directly from the block (big-endian)
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }

        // Words 16..63 are derived from earlier words using
        // two "small sigma" functions:
        //
        //   σ0(x) = ROTR7(x) ^ ROTR18(x) ^ SHR3(x)
        //   σ1(x) = ROTR17(x) ^ ROTR19(x) ^ SHR10(x)
        //
        //   w[i] = σ1(w[i-2]) + w[i-7] + σ0(w[i-15]) + w[i-16]
        for i in 16..64 {
            let s0 = small_sigma0(w[i - 15]);
            let s1 = small_sigma1(w[i - 2]);
            w[i] = s1
                .wrapping_add(w[i - 7])
                .wrapping_add(s0)
                .wrapping_add(w[i - 16]);
        }

        // --- Step 3b: Initialize working variables ---
        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        // --- Step 3c: 64 rounds of compression ---
        //
        // Each round computes:
        //   Σ1 = ROTR6(e) ^ ROTR11(e) ^ ROTR25(e)
        //   Ch = (e & f) ^ (~e & g)                  "choose"
        //   temp1 = h + Σ1 + Ch + K[i] + w[i]
        //   Σ0 = ROTR2(a) ^ ROTR13(a) ^ ROTR22(a)
        //   Maj = (a & b) ^ (a & c) ^ (b & c)       "majority"
        //   temp2 = Σ0 + Maj
        //
        // Then shift: h=g, g=f, f=e, e=d+temp1, d=c, c=b, b=a, a=temp1+temp2
        for i in 0..64 {
            let big_s1 = big_sigma1(e);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(big_s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);

            let big_s0 = big_sigma0(a);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = big_s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        // --- Step 3d: Add compressed values back to hash state ---
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    // --- Step 4: Output as hex ---
    let digest: Vec<u8> = h.iter().flat_map(|x| x.to_be_bytes()).collect();
    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

// ---------------------------------------------------------------------------
// SHA-256 Helper Functions
// ---------------------------------------------------------------------------

/// Pad a message according to the SHA-256 specification.
///
/// Unlike MD5 (which uses little-endian length), SHA-256 appends
/// the original length as a 64-bit BIG-endian integer.
///
/// ```text
///     [data] [0x80] [zeros...] [length as u64 BE]
///     Total length: multiple of 64 bytes
/// ```
fn sha256_pad(data: &[u8]) -> Vec<u8> {
    let orig_len_bits = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();

    // Append 0x80
    padded.push(0x80);

    // Pad with zeros until length ≡ 56 (mod 64)
    while padded.len() % 64 != 56 {
        padded.push(0);
    }

    // Append length as 64-bit big-endian
    padded.extend_from_slice(&orig_len_bits.to_be_bytes());

    padded
}

/// Small sigma 0: σ0(x) = ROTR7(x) ^ ROTR18(x) ^ SHR3(x)
///
/// Used in the message schedule expansion (words 16..63).
fn small_sigma0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

/// Small sigma 1: σ1(x) = ROTR17(x) ^ ROTR19(x) ^ SHR10(x)
///
/// Used in the message schedule expansion (words 16..63).
fn small_sigma1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

/// Big Sigma 0: Σ0(x) = ROTR2(x) ^ ROTR13(x) ^ ROTR22(x)
///
/// Used in the compression function (applied to 'a').
fn big_sigma0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

/// Big Sigma 1: Σ1(x) = ROTR6(x) ^ ROTR11(x) ^ ROTR25(x)
///
/// Used in the compression function (applied to 'e').
fn big_sigma1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_empty_string() {
        assert_eq!(
            compute_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_hello() {
        assert_eq!(
            compute_sha256(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_hello_world() {
        assert_eq!(
            compute_sha256(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn sha256_abc() {
        assert_eq!(
            compute_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn sha256_single_char() {
        assert_eq!(
            compute_sha256(b"a"),
            "ca978112ca1bbdcafac231b39a23dc4da786eff8147c4e72b9807785afee48bb"
        );
    }

    #[test]
    fn sha256_quick_brown_fox() {
        assert_eq!(
            compute_sha256(b"The quick brown fox jumps over the lazy dog"),
            "d7a8fbb307d7809469ca9abcb0082e4f8d5651e46d3cdb762d02d0bf37c9e592"
        );
    }

    #[test]
    fn sha256_deterministic() {
        let data = b"test data for determinism";
        let hash1 = compute_sha256(data);
        let hash2 = compute_sha256(data);
        assert_eq!(hash1, hash2, "SHA-256 should be deterministic");
    }

    #[test]
    fn sha256_different_inputs() {
        let hash1 = compute_sha256(b"hello");
        let hash2 = compute_sha256(b"Hello");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn sha256_output_length() {
        let hash = compute_sha256(b"test");
        assert_eq!(hash.len(), 64, "SHA-256 hex string should be 64 chars");
    }

    #[test]
    fn sha256_all_hex_chars() {
        let hash = compute_sha256(b"test");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "SHA-256 output should be all hex characters"
        );
    }

    #[test]
    fn sha256_padding_boundary() {
        // 55 bytes (one byte before padding boundary)
        let data = vec![b'A'; 55];
        let hash = compute_sha256(&data);
        assert_eq!(hash.len(), 64);

        // 56 bytes (at padding boundary — forces extra block)
        let data = vec![b'A'; 56];
        let hash = compute_sha256(&data);
        assert_eq!(hash.len(), 64);

        // 64 bytes (one full block)
        let data = vec![b'A'; 64];
        let hash = compute_sha256(&data);
        assert_eq!(hash.len(), 64);
    }

    #[test]
    fn sha256_longer_than_one_block() {
        // 128 bytes — two full blocks
        let data = vec![b'B'; 128];
        let hash = compute_sha256(&data);
        assert_eq!(hash.len(), 64);
        // Should be deterministic
        assert_eq!(hash, compute_sha256(&data));
    }
}
