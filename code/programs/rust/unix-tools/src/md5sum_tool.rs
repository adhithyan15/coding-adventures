//! # md5sum — Compute and Check MD5 Message Digest
//!
//! This module implements the business logic for the `md5sum` command.
//! `md5sum` computes the MD5 hash of data and outputs it as a
//! 32-character hexadecimal string.
//!
//! ## What Is MD5?
//!
//! MD5 (Message-Digest Algorithm 5) is a cryptographic hash function
//! that produces a 128-bit (16-byte) hash value. It maps arbitrary
//! data to a fixed-size fingerprint:
//!
//! ```text
//!     "hello"  → 5d41402abc4b2a76b9719d911017c592
//!     "Hello"  → 8b1a9953c4611296a827abf8c47804d7
//!     ""       → d41d8cd98f00b204e9800998ecf8427e
//! ```
//!
//! **Security warning**: MD5 is cryptographically broken and should
//! NOT be used for security purposes. Use SHA-256 or better for
//! anything security-sensitive. MD5 is still useful for checksums
//! and data integrity verification where collision resistance isn't
//! critical.
//!
//! ## MD5 Algorithm Overview
//!
//! MD5 processes data in 512-bit (64-byte) blocks through four
//! rounds of 16 operations each. The algorithm:
//!
//! 1. **Pad** the message to a multiple of 512 bits
//! 2. **Initialize** four 32-bit state variables (A, B, C, D)
//! 3. **Process** each 512-bit block through 64 operations
//! 4. **Output** the final state as a 128-bit digest
//!
//! Rather than implementing MD5 from scratch (which would be
//! educational but error-prone), we implement it inline for
//! zero external dependencies.

// ---------------------------------------------------------------------------
// MD5 Constants
// ---------------------------------------------------------------------------

/// Per-round shift amounts. MD5 uses 64 operations divided into
/// four rounds of 16. Each operation includes a left rotation by
/// one of these amounts.
const S: [u32; 64] = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20, 5,  9, 14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
];

/// Precomputed constants derived from the sine function.
/// K[i] = floor(2^32 * |sin(i + 1)|) for i = 0..63
///
/// These constants ensure that each round operation uses a
/// different "random-looking" constant, making the hash more
/// resistant to patterns in the input.
const K: [u32; 64] = [
    0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
    0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
    0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
    0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
    0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
    0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
    0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
    0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
    0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
    0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
    0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
    0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
    0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
    0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
    0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
    0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compute the MD5 hash of a byte slice and return it as a
/// 32-character lowercase hexadecimal string.
///
/// # Algorithm Steps
///
/// ```text
///     1. Pad the message:
///        - Append a 1 bit (0x80 byte)
///        - Append zeros until length ≡ 56 (mod 64)
///        - Append original length as 64-bit little-endian
///
///     2. Process each 64-byte block:
///        - Split into sixteen 32-bit words
///        - Run 64 operations updating A, B, C, D
///
///     3. Output A, B, C, D as little-endian bytes → hex string
/// ```
///
/// # Example
///
/// ```text
///     compute_md5(b"hello") → "5d41402abc4b2a76b9719d911017c592"
///     compute_md5(b"")      → "d41d8cd98f00b204e9800998ecf8427e"
/// ```
pub fn compute_md5(data: &[u8]) -> String {
    // --- Step 1: Pad the message ---
    let padded = md5_pad(data);

    // --- Step 2: Initialize state ---
    // These are the "magic" initial values from the MD5 spec (RFC 1321).
    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    // --- Step 3: Process each 64-byte block ---
    for chunk in padded.chunks(64) {
        // Parse the block into sixteen 32-bit words (little-endian)
        let mut m = [0u32; 16];
        for i in 0..16 {
            m[i] = u32::from_le_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }

        let mut a = a0;
        let mut b = b0;
        let mut c = c0;
        let mut d = d0;

        // --- 64 rounds of mixing ---
        for i in 0..64 {
            let (f, g) = match i {
                // Round 1: F(B, C, D) = (B & C) | (~B & D)
                0..=15 => ((b & c) | ((!b) & d), i),
                // Round 2: G(B, C, D) = (D & B) | (~D & C)
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                // Round 3: H(B, C, D) = B ^ C ^ D
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                // Round 4: I(B, C, D) = C ^ (B | ~D)
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };

            let temp = f
                .wrapping_add(a)
                .wrapping_add(K[i])
                .wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(temp.rotate_left(S[i]));
        }

        // Add this block's result to the running total
        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    // --- Step 4: Output as hex ---
    let digest = [
        a0.to_le_bytes(),
        b0.to_le_bytes(),
        c0.to_le_bytes(),
        d0.to_le_bytes(),
    ]
    .concat();

    digest.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Pad a message according to the MD5 specification.
///
/// ```text
///     Original message:  [data bytes]
///     After padding:     [data bytes] [0x80] [zeros...] [length as u64 LE]
///     Total length:      multiple of 64 bytes
/// ```
fn md5_pad(data: &[u8]) -> Vec<u8> {
    let orig_len_bits = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();

    // Append the 0x80 byte (a 1 bit followed by zeros)
    padded.push(0x80);

    // Pad with zeros until length ≡ 56 (mod 64)
    while padded.len() % 64 != 56 {
        padded.push(0);
    }

    // Append the original length as a 64-bit little-endian integer
    padded.extend_from_slice(&orig_len_bits.to_le_bytes());

    padded
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_empty_string() {
        // The MD5 of an empty string is a well-known constant
        assert_eq!(compute_md5(b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn md5_hello() {
        assert_eq!(
            compute_md5(b"hello"),
            "5d41402abc4b2a76b9719d911017c592"
        );
    }

    #[test]
    fn md5_hello_world() {
        assert_eq!(
            compute_md5(b"hello world"),
            "5eb63bbbe01eeed093cb22bb8f5acdc3"
        );
    }

    #[test]
    fn md5_abc() {
        assert_eq!(
            compute_md5(b"abc"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
    }

    #[test]
    fn md5_single_char() {
        assert_eq!(
            compute_md5(b"a"),
            "0cc175b9c0f1b6a831c399e269772661"
        );
    }

    #[test]
    fn md5_longer_message() {
        // "The quick brown fox jumps over the lazy dog"
        assert_eq!(
            compute_md5(b"The quick brown fox jumps over the lazy dog"),
            "9e107d9d372bb6826bd81d3542a419d6"
        );
    }

    #[test]
    fn md5_deterministic() {
        let data = b"test data for determinism";
        let hash1 = compute_md5(data);
        let hash2 = compute_md5(data);
        assert_eq!(hash1, hash2, "MD5 should be deterministic");
    }

    #[test]
    fn md5_different_inputs_different_hashes() {
        let hash1 = compute_md5(b"hello");
        let hash2 = compute_md5(b"Hello");
        assert_ne!(hash1, hash2, "different inputs should produce different hashes");
    }

    #[test]
    fn md5_output_length() {
        let hash = compute_md5(b"test");
        assert_eq!(hash.len(), 32, "MD5 hex string should be 32 chars");
    }

    #[test]
    fn md5_all_hex_chars() {
        let hash = compute_md5(b"test");
        assert!(
            hash.chars().all(|c| c.is_ascii_hexdigit()),
            "MD5 output should be all hex characters"
        );
    }

    #[test]
    fn md5_padding_boundary() {
        // Test with exactly 55 bytes (one byte before padding boundary)
        let data = vec![b'A'; 55];
        let hash = compute_md5(&data);
        assert_eq!(hash.len(), 32);

        // Test with exactly 56 bytes (at padding boundary)
        let data = vec![b'A'; 56];
        let hash = compute_md5(&data);
        assert_eq!(hash.len(), 32);

        // Test with exactly 64 bytes (one full block)
        let data = vec![b'A'; 64];
        let hash = compute_md5(&data);
        assert_eq!(hash.len(), 32);
    }
}
