//! # sha256
//!
//! SHA-256 cryptographic hash function (FIPS 180-4) implemented from scratch.
//!
//! ## What Is SHA-256?
//!
//! SHA-256 is a member of the SHA-2 family designed by the NSA and published by
//! NIST in 2001. It takes any sequence of bytes and produces a fixed-size 32-byte
//! (256-bit) "fingerprint" called a digest. The same input always produces the
//! same digest. Change even one bit and the digest changes completely — the
//! "avalanche effect". You cannot reverse a digest to the original input.
//!
//! SHA-256 is the workhorse of modern cryptography: TLS, Bitcoin, git, code
//! signing, and password hashing all depend on it. Unlike MD5 (broken 2004) and
//! SHA-1 (broken 2017), SHA-256 remains secure with no known practical attacks.
//!
//! ## How SHA-256 Differs from SHA-1
//!
//! Both use the Merkle-Damgard construction, but SHA-256 is stronger:
//!
//! | Property      | SHA-1          | SHA-256         |
//! |---------------|----------------|-----------------|
//! | State words   | 5 x 32-bit     | 8 x 32-bit      |
//! | Rounds        | 80             | 64               |
//! | Digest size   | 160 bits       | 256 bits         |
//! | Schedule      | linear XOR     | non-linear sigma |
//! | Constants     | 4              | 64               |
//!
//! ## Rust Advantages for Cryptography
//!
//! - `u32::wrapping_add` makes mod-2^32 arithmetic explicit and safe.
//! - `u32::rotate_right` compiles to a single `ror` instruction on x86.
//! - `[u8; 32]` is stack-allocated — no heap allocation for the digest.
//! - No garbage collector pauses during compression.
//!
//! ## FIPS 180-4 Test Vectors
//!
//! ```
//! use coding_adventures_sha256::sha256_hex;
//! assert_eq!(
//!     sha256_hex(b"abc"),
//!     "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
//! );
//! ```

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroize;
use core::fmt;

const BLOCK_LENGTH: usize = 64;
const LENGTH_FIELD_LENGTH: usize = 8;
const SINGLE_BLOCK_MESSAGE_LIMIT: usize = BLOCK_LENGTH - LENGTH_FIELD_LENGTH - 1;
const MAX_MESSAGE_BYTES: u64 = u64::MAX / 8;

// ─── Initial Hash Values ─────────────────────────────────────────────────────
//
// Eight 32-bit words: the first 32 bits of the fractional parts of the square
// roots of the first 8 primes (2, 3, 5, 7, 11, 13, 17, 19).
//
// These are "nothing up my sleeve" numbers — their mathematical origin is
// transparent and verifiable, proving no backdoor is hidden.
//
// Derivation example for H0:
//   sqrt(2) = 1.41421356...
//   fractional part = 0.41421356...
//   * 2^32 = 1779033703.952... -> floor -> 0x6A09E667

const INIT: [u32; 8] = [
    0x6A09E667, // sqrt(2)
    0xBB67AE85, // sqrt(3)
    0x3C6EF372, // sqrt(5)
    0xA54FF53A, // sqrt(7)
    0x510E527F, // sqrt(11)
    0x9B05688C, // sqrt(13)
    0x1F83D9AB, // sqrt(17)
    0x5BE0CD19, // sqrt(19)
];

// ─── Round Constants ─────────────────────────────────────────────────────────
//
// 64 constants: the first 32 bits of the fractional parts of the cube roots
// of the first 64 primes (2, 3, 5, ..., 311).
//
// Having 64 unique constants (vs SHA-1's 4) means each round has its own
// "flavor" of mixing, making the compression function harder to attack.

const K: [u32; 64] = [
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5, 0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3, 0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC, 0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7, 0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13, 0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3, 0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5, 0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208, 0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
];

// ─── Auxiliary Functions ─────────────────────────────────────────────────────
//
// SHA-256 uses six auxiliary functions built from bitwise operations.
//
// Ch(x, y, z) — "Choose": if bit of x is 1, choose y; else choose z.
// Maj(x, y, z) — "Majority": output 1 if at least 2 of 3 inputs are 1.
// big_sigma0(x) — ROTR(2) XOR ROTR(13) XOR ROTR(22) — used on `a` in rounds
// big_sigma1(x) — ROTR(6) XOR ROTR(11) XOR ROTR(25) — used on `e` in rounds
// small_sigma0(x) — ROTR(7) XOR ROTR(18) XOR SHR(3)  — message schedule
// small_sigma1(x) — ROTR(17) XOR ROTR(19) XOR SHR(10) — message schedule
//
// The SHR in the small sigma functions makes the schedule non-invertible,
// which is a key improvement over SHA-1's linear (XOR-only) schedule.

#[inline]
fn ch(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (!x & z)
}

#[inline]
fn maj(x: u32, y: u32, z: u32) -> u32 {
    (x & y) ^ (x & z) ^ (y & z)
}

#[inline]
fn big_sigma0(x: u32) -> u32 {
    x.rotate_right(2) ^ x.rotate_right(13) ^ x.rotate_right(22)
}

#[inline]
fn big_sigma1(x: u32) -> u32 {
    x.rotate_right(6) ^ x.rotate_right(11) ^ x.rotate_right(25)
}

#[inline]
fn small_sigma0(x: u32) -> u32 {
    x.rotate_right(7) ^ x.rotate_right(18) ^ (x >> 3)
}

#[inline]
fn small_sigma1(x: u32) -> u32 {
    x.rotate_right(17) ^ x.rotate_right(19) ^ (x >> 10)
}

// ─── Message Schedule ────────────────────────────────────────────────────────
//
// Parse 16 big-endian 32-bit words from the block, then expand to 64 words:
//   W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
//
// The non-linear sigma functions (with SHR) make this schedule much stronger
// than SHA-1's linear XOR-rotate expansion.

fn schedule(block: &[u8; BLOCK_LENGTH]) -> [u32; 64] {
    let mut w = [0u32; 64];
    for (index, word) in w[..16].iter_mut().enumerate() {
        let start = index * 4;
        *word = u32::from_be_bytes([
            block[start],
            block[start + 1],
            block[start + 2],
            block[start + 3],
        ]);
    }
    for t in 16..64 {
        w[t] = small_sigma1(w[t - 2])
            .wrapping_add(w[t - 7])
            .wrapping_add(small_sigma0(w[t - 15]))
            .wrapping_add(w[t - 16]);
    }
    w
}

// ─── Compression Function ────────────────────────────────────────────────────
//
// 64 rounds fold one 64-byte block into the eight-word state.
//
// Each round:
//   T1 = h + Sigma1(e) + Ch(e,f,g) + K[t] + W[t]
//   T2 = Sigma0(a) + Maj(a,b,c)
//   Shift working variables down, inject T1+T2 at top (a) and T1 at middle (e).
//
// Davies-Meyer feed-forward: add compressed output back to input state.

fn compress(state: &mut [u32; 8], block: &[u8; BLOCK_LENGTH]) {
    let mut schedule = schedule(block);
    let mut working = *state;

    for round in 0..64 {
        let mut temporary_1 = working[7]
            .wrapping_add(big_sigma1(working[4]))
            .wrapping_add(ch(working[4], working[5], working[6]))
            .wrapping_add(K[round])
            .wrapping_add(schedule[round]);
        let mut temporary_2 =
            big_sigma0(working[0]).wrapping_add(maj(working[0], working[1], working[2]));

        working = [
            temporary_1.wrapping_add(temporary_2),
            working[0],
            working[1],
            working[2],
            working[3].wrapping_add(temporary_1),
            working[4],
            working[5],
            working[6],
        ];
        temporary_1.zeroize();
        temporary_2.zeroize();
    }

    for (state_word, working_word) in state.iter_mut().zip(working.iter()) {
        let mut updated = (*state_word).wrapping_add(*working_word);
        state_word.zeroize();
        *state_word = updated;
        updated.zeroize();
    }

    scrub_words(&mut working);
    scrub_words(&mut schedule);
}

fn scrub_words<const N: usize>(words: &mut [u32; N]) {
    for word in words {
        word.zeroize();
    }
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Compute the SHA-256 digest of `data`. Returns a `[u8; 32]` array.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha256::sha256;
/// let digest = sha256(b"abc");
/// let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
/// assert_eq!(hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
/// ```
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256Hasher::new();
    hasher
        .try_update(data)
        .expect("one address-space-sized slice fits SHA-256's length field");
    hasher.digest()
}

/// Compute SHA-256 and return the 64-character lowercase hex string.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha256::sha256_hex;
/// assert_eq!(
///     sha256_hex(b"abc"),
///     "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
/// );
/// ```
pub fn sha256_hex(data: &[u8]) -> String {
    sha256(data).iter().map(|b| format!("{:02x}", b)).collect()
}

/// Stable, payload-free failures from checked streaming SHA-256.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sha256Error {
    /// The message's bit length would not fit the FIPS 180-4 64-bit field.
    MessageTooLong,
}

impl fmt::Display for Sha256Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SHA-256 message is too long")
    }
}

impl std::error::Error for Sha256Error {}

/// Fixed-memory streaming SHA-256 hasher that accepts data in multiple chunks.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha256::{Sha256Hasher, sha256};
/// let mut h = Sha256Hasher::new();
/// h.update(b"ab");
/// h.update(b"c");
/// assert_eq!(h.digest(), sha256(b"abc"));
/// ```
pub struct Sha256Hasher {
    state: [u32; 8],
    buffer: [u8; BLOCK_LENGTH],
    buffer_len: usize,
    byte_count: u64,
}

impl Sha256Hasher {
    /// Create a new streaming hasher initialized with SHA-256 constants.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: INIT,
            buffer: [0; BLOCK_LENGTH],
            buffer_len: 0,
            byte_count: 0,
        }
    }

    /// Feed more bytes into the hash through the legacy infallible API.
    ///
    /// New security-sensitive callers should use [`Self::try_update`] so a
    /// cumulative message-length overflow is an ordinary error. This wrapper
    /// preserves the crate's original public API and panics only if repeated
    /// updates exceed FIPS 180-4's representable 64-bit message length.
    pub fn update(&mut self, data: &[u8]) {
        self.try_update(data)
            .expect("SHA-256 message length exceeds FIPS 180-4's 64-bit field");
    }

    /// Feed more bytes into the hash after validating the cumulative length.
    ///
    /// The complete length check happens before retained state changes. On an
    /// error, the chaining state, partial block, and byte count are unchanged.
    ///
    /// # Errors
    ///
    /// Returns [`Sha256Error::MessageTooLong`] if the resulting bit length
    /// would not fit FIPS 180-4's 64-bit length field.
    pub fn try_update(&mut self, data: &[u8]) -> Result<(), Sha256Error> {
        let data_len = u64::try_from(data.len()).map_err(|_| Sha256Error::MessageTooLong)?;
        let next_count = self
            .byte_count
            .checked_add(data_len)
            .filter(|count| *count <= MAX_MESSAGE_BYTES)
            .ok_or(Sha256Error::MessageTooLong)?;

        let mut remaining = data;
        if self.buffer_len != 0 {
            let needed = BLOCK_LENGTH - self.buffer_len;
            let copied = needed.min(remaining.len());
            self.buffer[self.buffer_len..self.buffer_len + copied]
                .copy_from_slice(&remaining[..copied]);
            self.buffer_len += copied;
            remaining = &remaining[copied..];

            if self.buffer_len != BLOCK_LENGTH {
                self.byte_count = next_count;
                return Ok(());
            }

            compress(&mut self.state, &self.buffer);
            self.buffer.zeroize();
            self.buffer_len = 0;
        }

        while remaining.len() >= BLOCK_LENGTH {
            let (block, tail) = remaining.split_at(BLOCK_LENGTH);
            let block: &[u8; BLOCK_LENGTH] = block
                .try_into()
                .expect("split_at returns one complete SHA-256 block");
            compress(&mut self.state, block);
            remaining = tail;
        }

        self.buffer[..remaining.len()].copy_from_slice(remaining);
        self.buffer_len = remaining.len();
        self.byte_count = next_count;
        Ok(())
    }

    /// Return the 32-byte digest of all data fed so far.
    ///
    /// Non-destructive: internal state is not modified.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        let mut state = self.state;
        let mut tail = [0u8; BLOCK_LENGTH * 2];
        tail[..self.buffer_len].copy_from_slice(&self.buffer[..self.buffer_len]);
        tail[self.buffer_len] = 0x80;
        let tail_len = if self.buffer_len <= SINGLE_BLOCK_MESSAGE_LIMIT {
            BLOCK_LENGTH
        } else {
            BLOCK_LENGTH * 2
        };
        let bit_length = self.byte_count * 8;
        tail[tail_len - LENGTH_FIELD_LENGTH..tail_len].copy_from_slice(&bit_length.to_be_bytes());

        for block in tail[..tail_len].as_chunks::<BLOCK_LENGTH>().0 {
            compress(&mut state, block);
        }

        let mut digest = [0u8; 32];
        for (index, word) in state.iter().enumerate() {
            digest[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }

        scrub_words(&mut state);
        tail.zeroize();
        digest
    }

    /// Return the 64-character hex digest string.
    #[must_use]
    pub fn hex_digest(&self) -> String {
        self.digest().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Return an independent fixed-memory copy of the current hasher state.
    #[must_use]
    pub fn clone_hasher(&self) -> Sha256Hasher {
        Sha256Hasher {
            state: self.state,
            buffer: self.buffer,
            buffer_len: self.buffer_len,
            byte_count: self.byte_count,
        }
    }
}

impl Default for Sha256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Sha256Hasher {
    fn drop(&mut self) {
        scrub_words(&mut self.state);
        self.buffer.zeroize();
        self.buffer_len.zeroize();
        self.byte_count.zeroize();
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── FIPS 180-4 Test Vectors ─────────────────────────────────────────

    #[test]
    fn fips_empty_string() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn fips_abc() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn fips_448_bit_message() {
        let msg = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(msg.len(), 56);
        assert_eq!(
            sha256_hex(msg),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn fips_million_a() {
        let data = vec![b'a'; 1_000_000];
        assert_eq!(
            sha256_hex(&data),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
    }

    // ─── Output Format ───────────────────────────────────────────────────

    #[test]
    fn digest_is_32_bytes() {
        assert_eq!(sha256(b"").len(), 32);
        assert_eq!(sha256(b"hello world").len(), 32);
        assert_eq!(sha256(&vec![0u8; 1000]).len(), 32);
    }

    #[test]
    fn hex_string_is_64_chars() {
        assert_eq!(sha256_hex(b"").len(), 64);
        assert_eq!(sha256_hex(b"hello").len(), 64);
    }

    #[test]
    fn hex_string_is_lowercase() {
        let h = sha256_hex(b"abc");
        assert!(h.chars().all(|c| !c.is_uppercase()));
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn deterministic() {
        assert_eq!(sha256(b"hello"), sha256(b"hello"));
    }

    #[test]
    fn avalanche() {
        let h1 = sha256(b"hello");
        let h2 = sha256(b"helo");
        assert_ne!(h1, h2);
        let bits_different: u32 = h1
            .iter()
            .zip(h2.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        assert!(bits_different > 40, "only {bits_different} bits differed");
    }

    // ─── Block Boundaries ────────────────────────────────────────────────

    #[test]
    fn block_boundary_55() {
        assert_eq!(sha256(&[0u8; 55]).len(), 32);
    }

    #[test]
    fn block_boundary_56() {
        assert_eq!(sha256(&[0u8; 56]).len(), 32);
    }

    #[test]
    fn block_boundary_55_and_56_differ() {
        assert_ne!(sha256(&[0u8; 55]), sha256(&[0u8; 56]));
    }

    #[test]
    fn block_boundary_64() {
        assert_eq!(sha256(&[0u8; 64]).len(), 32);
    }

    #[test]
    fn block_boundary_128() {
        assert_eq!(sha256(&[0u8; 128]).len(), 32);
    }

    #[test]
    fn all_boundaries_distinct() {
        let sizes = [55, 56, 63, 64, 127, 128];
        let digests: std::collections::HashSet<[u8; 32]> =
            sizes.iter().map(|&n| sha256(&vec![0u8; n])).collect();
        assert_eq!(digests.len(), 6);
    }

    #[test]
    fn zero_byte_padding_boundaries_match_independent_sha256_answers() {
        // Python's OpenSSL-backed hashlib independently produced these
        // SHA-256 answers for exactly N zero bytes. The cases straddle both
        // the 56-byte padding threshold and complete 64-byte blocks.
        let cases = [
            (
                55,
                "02779466cdec163811d078815c633f21901413081449002f24aa3e80f0b88ef7",
            ),
            (
                56,
                "d4817aa5497628e7c77e6b606107042bbba3130888c5f47a375e6179be789fbb",
            ),
            (
                63,
                "c7723fa1e0127975e49e62e753db53924c1bd84b8ac1ac08df78d09270f3d971",
            ),
            (
                64,
                "f5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b",
            ),
            (
                65,
                "98ce42deef51d40269d542f5314bef2c7468d401ad5d85168bfab4c0108f75f7",
            ),
            (
                127,
                "15dae5979058bfbf4f9166029b6e340ea3ca374fef578a11dc9e6e923860d7ae",
            ),
            (
                128,
                "38723a2e5e8a17aa7950dc008209944e898f69a7bd10a23c839d341e935fd5ca",
            ),
            (
                129,
                "2c80619d7e7c58257293cda3a878c13e5856f4e06f6f90601276f7b9179c9e07",
            ),
        ];

        for (byte_count, expected) in cases {
            assert_eq!(sha256_hex(&vec![0; byte_count]), expected);
        }
    }

    // ─── Edge Cases ──────────────────────────────────────────────────────

    #[test]
    fn null_byte_differs_from_empty() {
        assert_ne!(sha256(&[0x00]), sha256(b""));
    }

    #[test]
    fn all_byte_values() {
        let data: Vec<u8> = (0u8..=255).collect();
        assert_eq!(sha256(&data).len(), 32);
    }

    #[test]
    fn every_single_byte_unique() {
        let digests: std::collections::HashSet<[u8; 32]> =
            (0u8..=255).map(|i| sha256(&[i])).collect();
        assert_eq!(digests.len(), 256);
    }

    // ─── Streaming API ──────────────────────────────────────────────────

    #[test]
    fn streaming_single_write() {
        let mut h = Sha256Hasher::new();
        h.update(b"abc");
        assert_eq!(h.digest(), sha256(b"abc"));
    }

    #[test]
    fn streaming_split_at_byte() {
        let mut h = Sha256Hasher::new();
        h.update(b"ab");
        h.update(b"c");
        assert_eq!(h.digest(), sha256(b"abc"));
    }

    #[test]
    fn streaming_split_at_block() {
        let data = vec![0u8; 128];
        let mut h = Sha256Hasher::new();
        h.update(&data[..64]);
        h.update(&data[64..]);
        assert_eq!(h.digest(), sha256(&data));
    }

    #[test]
    fn streaming_byte_at_a_time() {
        let data: Vec<u8> = (0u8..100).collect();
        let mut h = Sha256Hasher::new();
        for &b in &data {
            h.update(&[b]);
        }
        assert_eq!(h.digest(), sha256(&data));
    }

    #[test]
    fn streaming_empty() {
        let h = Sha256Hasher::new();
        assert_eq!(h.digest(), sha256(b""));
    }

    #[test]
    fn streaming_nondestructive() {
        let mut h = Sha256Hasher::new();
        h.update(b"abc");
        assert_eq!(h.digest(), h.digest());
    }

    #[test]
    fn streaming_hex_digest() {
        let mut h = Sha256Hasher::new();
        h.update(b"abc");
        assert_eq!(
            h.hex_digest(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn streaming_clone_is_independent() {
        let mut h = Sha256Hasher::new();
        h.update(b"ab");
        let mut h2 = h.clone_hasher();
        h2.update(b"c");
        h.update(b"x");
        assert_eq!(h2.digest(), sha256(b"abc"));
        assert_eq!(h.digest(), sha256(b"abx"));
    }

    #[test]
    fn streaming_million_a() {
        let data = vec![b'a'; 1_000_000];
        let mut h = Sha256Hasher::new();
        h.update(&data[..500_000]);
        h.update(&data[500_000..]);
        assert_eq!(h.digest(), sha256(&data));
    }

    #[test]
    fn checked_streaming_matches_one_shot_at_every_boundary_partition() {
        for byte_count in [0, 1, 55, 56, 63, 64, 65, 127, 128, 129] {
            let data: Vec<u8> = (0..byte_count).map(|index| index as u8).collect();
            let expected = sha256(&data);

            for split in 0..=data.len() {
                let mut hasher = Sha256Hasher::new();
                hasher.try_update(&data[..split]).unwrap();
                hasher.try_update(&data[split..]).unwrap();
                assert_eq!(
                    hasher.digest(),
                    expected,
                    "length {byte_count}, split {split}"
                );
            }
        }
    }

    #[test]
    fn checked_update_rejects_overflow_before_mutating_state() {
        assert_eq!(MAX_MESSAGE_BYTES % BLOCK_LENGTH as u64, 63);
        let mut hasher = Sha256Hasher {
            state: INIT,
            buffer: [0xa5; BLOCK_LENGTH],
            buffer_len: 63,
            byte_count: MAX_MESSAGE_BYTES,
        };
        let expected_state = hasher.state;
        let expected_buffer = hasher.buffer;
        let expected_buffer_len = hasher.buffer_len;
        let expected_byte_count = hasher.byte_count;

        assert_eq!(hasher.try_update(b"x"), Err(Sha256Error::MessageTooLong));
        assert_eq!(hasher.state, expected_state);
        assert_eq!(hasher.buffer, expected_buffer);
        assert_eq!(hasher.buffer_len, expected_buffer_len);
        assert_eq!(hasher.byte_count, expected_byte_count);
    }

    #[test]
    fn checked_update_error_is_stable_and_payload_free() {
        assert_eq!(
            Sha256Error::MessageTooLong.to_string(),
            "SHA-256 message is too long"
        );
    }
}
