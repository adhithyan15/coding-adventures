//! # sha512
//!
//! SHA-512 cryptographic hash function (FIPS 180-4) implemented from scratch.
//!
//! ## What Is SHA-512?
//!
//! SHA-512 is the 64-bit sibling of SHA-256 in the SHA-2 family. It takes any
//! sequence of bytes and produces a fixed-size 64-byte (512-bit) digest. The
//! same input always produces the same digest. Change even one bit of input and
//! the digest changes completely — the "avalanche effect".
//!
//! On 64-bit platforms, SHA-512 is often *faster* than SHA-256 because it
//! processes 128-byte blocks (vs 64-byte) using native 64-bit arithmetic.
//!
//! ## How It Differs from SHA-256
//!
//! ```text
//!   Property         SHA-256       SHA-512
//!   ────────         ───────       ───────
//!   Word size        32-bit        64-bit
//!   State words      8 × u32       8 × u64
//!   Block size       64 bytes      128 bytes
//!   Rounds           64            80
//!   Digest size      32 bytes      64 bytes
//!   Length field      64-bit       128-bit
//! ```
//!
//! The rotation/shift amounts also differ (tuned for 64-bit words).
//!
//! ## Rust Advantages for SHA-512
//!
//! Rust's native `u64` type is perfect for SHA-512:
//! - `wrapping_add` for modular arithmetic (no overflow panic)
//! - `rotate_right` compiles to a single `ror` instruction on x86-64
//! - `[u8; 64]` is a fixed-size, stack-allocated digest — no heap allocation
//! - No garbage collector means deterministic timing (important for crypto)
//!
//! ## FIPS 180-4 Test Vectors
//!
//! ```
//! use coding_adventures_sha512::hex_string;
//! assert_eq!(
//!     &hex_string(b"abc")[..32],
//!     "ddaf35a193617abacc417349ae204131"
//! );
//! ```

// ─── Initialization Constants ────────────────────────────────────────────────
//
// SHA-512 starts with these eight 64-bit words as its initial state.
// They are the first 64 bits of the fractional parts of the square roots
// of the first 8 primes (2, 3, 5, 7, 11, 13, 17, 19).
//
//   H₀ = frac(sqrt(2))  × 2^64 = 0x6a09e667f3bcc908
//   H₁ = frac(sqrt(3))  × 2^64 = 0xbb67ae8584caa73b
//   ...and so on

const INIT: [u64; 8] = [
    0x6a09e667f3bcc908,
    0xbb67ae8584caa73b,
    0x3c6ef372fe94f82b,
    0xa54ff53a5f1d36f1,
    0x510e527fade682d1,
    0x9b05688c2b3e6c1f,
    0x1f83d9abfb41bd6b,
    0x5be0cd19137e2179,
];

// ─── Round Constants ─────────────────────────────────────────────────────────
//
// 80 constants, one per round. Each is the first 64 bits of the fractional
// part of the cube root of the i-th prime (2, 3, 5, 7, 11, ..., 409).
//
// These "nothing up my sleeve" numbers prove no backdoor is hidden —
// anyone can verify them by computing cube roots of primes.

const K: [u64; 80] = [
    0x428a2f98d728ae22, 0x7137449123ef65cd, 0xb5c0fbcfec4d3b2f, 0xe9b5dba58189dbbc,
    0x3956c25bf348b538, 0x59f111f1b605d019, 0x923f82a4af194f9b, 0xab1c5ed5da6d8118,
    0xd807aa98a3030242, 0x12835b0145706fbe, 0x243185be4ee4b28c, 0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f, 0x80deb1fe3b1696b1, 0x9bdc06a725c71235, 0xc19bf174cf692694,
    0xe49b69c19ef14ad2, 0xefbe4786384f25e3, 0x0fc19dc68b8cd5b5, 0x240ca1cc77ac9c65,
    0x2de92c6f592b0275, 0x4a7484aa6ea6e483, 0x5cb0a9dcbd41fbd4, 0x76f988da831153b5,
    0x983e5152ee66dfab, 0xa831c66d2db43210, 0xb00327c898fb213f, 0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2, 0xd5a79147930aa725, 0x06ca6351e003826f, 0x142929670a0e6e70,
    0x27b70a8546d22ffc, 0x2e1b21385c26c926, 0x4d2c6dfc5ac42aed, 0x53380d139d95b3df,
    0x650a73548baf63de, 0x766a0abb3c77b2a8, 0x81c2c92e47edaee6, 0x92722c851482353b,
    0xa2bfe8a14cf10364, 0xa81a664bbc423001, 0xc24b8b70d0f89791, 0xc76c51a30654be30,
    0xd192e819d6ef5218, 0xd69906245565a910, 0xf40e35855771202a, 0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8, 0x1e376c085141ab53, 0x2748774cdf8eeb99, 0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63, 0x4ed8aa4ae3418acb, 0x5b9cca4f7763e373, 0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc, 0x78a5636f43172f60, 0x84c87814a1f0ab72, 0x8cc702081a6439ec,
    0x90befffa23631e28, 0xa4506cebde82bde9, 0xbef9a3f7b2c67915, 0xc67178f2e372532b,
    0xca273eceea26619c, 0xd186b8c721c0c207, 0xeada7dd6cde0eb1e, 0xf57d4f7fee6ed178,
    0x06f067aa72176fba, 0x0a637dc5a2c898a6, 0x113f9804bef90dae, 0x1b710b35131c471b,
    0x28db77f523047d84, 0x32caab7b40c72493, 0x3c9ebe0a15c9bebc, 0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6, 0x597f299cfc657e2a, 0x5fcb6fab3ad6faec, 0x6c44198c4a475817,
];

// ─── SHA-512 Sigma Functions ─────────────────────────────────────────────────
//
// SHA-512 uses four mixing functions, each combining rotations and shifts.
// Capital sigma (Σ) operates on state words; lowercase sigma (σ) operates
// on message schedule words.
//
//   Σ0(x) = ROTR(28,x) XOR ROTR(34,x) XOR ROTR(39,x)
//   Σ1(x) = ROTR(14,x) XOR ROTR(18,x) XOR ROTR(41,x)
//   σ0(x) = ROTR(1,x)  XOR ROTR(8,x)  XOR (x >> 7)
//   σ1(x) = ROTR(19,x) XOR ROTR(61,x) XOR (x >> 6)
//
// Note: σ0 and σ1 use a right SHIFT (not rotation) for their third term.
// A shift discards bits; a rotation preserves them.
//
// Rust's `u64::rotate_right(n)` compiles to a single `ror` instruction.

#[inline]
fn big_sigma0(x: u64) -> u64 {
    x.rotate_right(28) ^ x.rotate_right(34) ^ x.rotate_right(39)
}

#[inline]
fn big_sigma1(x: u64) -> u64 {
    x.rotate_right(14) ^ x.rotate_right(18) ^ x.rotate_right(41)
}

#[inline]
fn small_sigma0(x: u64) -> u64 {
    x.rotate_right(1) ^ x.rotate_right(8) ^ (x >> 7)
}

#[inline]
fn small_sigma1(x: u64) -> u64 {
    x.rotate_right(19) ^ x.rotate_right(61) ^ (x >> 6)
}

// ─── Choice and Majority ─────────────────────────────────────────────────────
//
// Ch(x, y, z) — "Choice": for each bit, x chooses between y and z.
//   If x bit = 1, output y bit. If x bit = 0, output z bit.
//   Formula: (x AND y) XOR (NOT x AND z)
//
// Maj(x, y, z) — "Majority": output the bit value appearing in ≥ 2 of 3 inputs.
//   Formula: (x AND y) XOR (x AND z) XOR (y AND z)

#[inline]
fn ch(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (!x & z)
}

#[inline]
fn maj(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (x & z) ^ (y & z)
}

// ─── Padding ─────────────────────────────────────────────────────────────────
//
// SHA-512 processes 128-byte (1024-bit) blocks. Padding extends the message:
//
//   1. Append 0x80 (the '1' bit followed by seven '0' bits).
//   2. Append 0x00 bytes until length ≡ 112 (mod 128).
//   3. Append the original bit length as a 128-bit big-endian integer.
//
// Why 112 mod 128? We need 16 bytes for the length field (128-bit),
// and 112 + 16 = 128.
//
// For practical messages the bit length fits in 64 bits, so the high
// 8 bytes of the length field are zero.

fn pad(data: &[u8]) -> Vec<u8> {
    let byte_len = data.len();
    let bit_len: u128 = (byte_len as u128) * 8;

    let mut padded = data.to_vec();
    padded.push(0x80); // the mandatory '1' bit

    // Append zeros until length ≡ 112 (mod 128)
    while padded.len() % 128 != 112 {
        padded.push(0x00);
    }

    // Append 128-bit big-endian length
    padded.extend_from_slice(&bit_len.to_be_bytes());
    padded
}

// ─── Message Schedule ────────────────────────────────────────────────────────
//
// Each 128-byte block is parsed as 16 big-endian 64-bit words (W[0..15]),
// then expanded to 80 words using:
//
//   W[i] = σ1(W[i-2]) + W[i-7] + σ0(W[i-15]) + W[i-16]   (mod 2^64)
//
// This uses the σ functions (rotation + shift combinations) rather than
// SHA-1's simple XOR-and-rotate, providing stronger diffusion.

fn schedule(block: &[u8]) -> [u64; 80] {
    let mut w = [0u64; 80];

    // Parse 16 big-endian 64-bit words
    for i in 0..16 {
        w[i] = u64::from_be_bytes(block[i * 8..i * 8 + 8].try_into().unwrap());
    }

    // Expand from 16 to 80 words
    for i in 16..80 {
        w[i] = small_sigma1(w[i - 2])
            .wrapping_add(w[i - 7])
            .wrapping_add(small_sigma0(w[i - 15]))
            .wrapping_add(w[i - 16]);
    }

    w
}

// ─── Compression Function ────────────────────────────────────────────────────
//
// 80 rounds of mixing fold one 128-byte block into the eight-word state.
//
// Each round computes two temporary values:
//
//   T₁ = h + Σ1(e) + Ch(e,f,g) + K[t] + W[t]
//   T₂ = Σ0(a) + Maj(a,b,c)
//
// Then the eight working variables shift:
//   h=g, g=f, f=e, e=d+T₁, d=c, c=b, b=a, a=T₁+T₂
//
// Davies-Meyer feed-forward: after all 80 rounds, add compressed output
// back to input state to prevent invertibility.

fn compress(state: [u64; 8], block: &[u8]) -> [u64; 8] {
    let w = schedule(block);
    let [h0, h1, h2, h3, h4, h5, h6, h7] = state;
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (h0, h1, h2, h3, h4, h5, h6, h7);

    for t in 0..80 {
        let t1 = h
            .wrapping_add(big_sigma1(e))
            .wrapping_add(ch(e, f, g))
            .wrapping_add(K[t])
            .wrapping_add(w[t]);
        let t2 = big_sigma0(a).wrapping_add(maj(a, b, c));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }

    [
        h0.wrapping_add(a),
        h1.wrapping_add(b),
        h2.wrapping_add(c),
        h3.wrapping_add(d),
        h4.wrapping_add(e),
        h5.wrapping_add(f),
        h6.wrapping_add(g),
        h7.wrapping_add(h),
    ]
}

// ─── Public API ──────────────────────────────────────────────────────────────

/// Compute the SHA-512 digest of `data`. Returns a `[u8; 64]` array.
///
/// This is the one-shot API: hash a complete message in a single call.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha512::sum512;
/// let digest = sum512(b"abc");
/// let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
/// assert!(hex.starts_with("ddaf35a193617aba"));
/// ```
pub fn sum512(data: &[u8]) -> [u8; 64] {
    let padded = pad(data);
    let mut state = INIT;
    for chunk in padded.chunks(128) {
        state = compress(state, chunk);
    }
    // Finalize: write eight 64-bit words as big-endian bytes.
    let mut digest = [0u8; 64];
    for (i, &word) in state.iter().enumerate() {
        digest[i * 8..i * 8 + 8].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// Compute SHA-512 and return the 128-character lowercase hex string.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha512::hex_string;
/// let h = hex_string(b"abc");
/// assert!(h.starts_with("ddaf35a193617aba"));
/// assert_eq!(h.len(), 128);
/// ```
pub fn hex_string(data: &[u8]) -> String {
    sum512(data).iter().map(|b| format!("{:02x}", b)).collect()
}

/// Streaming SHA-512 hasher that accepts data in multiple chunks.
///
/// Useful when the full message is not available at once — for example when
/// reading a large file in chunks or hashing a network stream.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha512::{Digest, sum512};
/// let mut h = Digest::new();
/// h.update(b"ab");
/// h.update(b"c");
/// assert_eq!(h.sum512(), sum512(b"abc"));
/// ```
///
/// Multiple `update` calls are equivalent to a single `sum512(all_data)`.
pub struct Digest {
    state: [u64; 8],
    buf: Vec<u8>,    // partial block (< 128 bytes)
    byte_count: u64, // total bytes fed in
}

impl Digest {
    /// Initialize a new streaming hasher with SHA-512's starting constants.
    pub fn new() -> Self {
        Digest {
            state: INIT,
            buf: Vec::new(),
            byte_count: 0,
        }
    }

    /// Feed more bytes into the hash.
    pub fn update(&mut self, data: &[u8]) {
        self.byte_count += data.len() as u64;
        self.buf.extend_from_slice(data);
        // Compress any complete 128-byte blocks to keep buf small
        while self.buf.len() >= 128 {
            let block: [u8; 128] = self.buf[..128].try_into().unwrap();
            self.state = compress(self.state, &block);
            self.buf.drain(..128);
        }
    }

    /// Return the 64-byte digest of all data fed so far.
    ///
    /// Non-destructive: the internal state is not modified, so you can
    /// continue calling `update` after calling `sum512`.
    pub fn sum512(&self) -> [u8; 64] {
        // Pad the remaining buffer using the TOTAL byte count
        let bit_len: u128 = (self.byte_count as u128) * 8;
        let mut tail = self.buf.clone();
        tail.push(0x80);
        while tail.len() % 128 != 112 {
            tail.push(0x00);
        }
        tail.extend_from_slice(&bit_len.to_be_bytes());

        // Compress the padding tail against a copy of the live state
        let mut state = self.state;
        for chunk in tail.chunks(128) {
            state = compress(state, chunk);
        }

        let mut digest = [0u8; 64];
        for (i, &word) in state.iter().enumerate() {
            digest[i * 8..i * 8 + 8].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    /// Return the 128-character hex string of the digest.
    pub fn hex_digest(&self) -> String {
        self.sum512().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Return an independent copy of the current hasher state.
    ///
    /// Useful for computing multiple digests that share a common prefix.
    pub fn clone_digest(&self) -> Digest {
        Digest {
            state: self.state,
            buf: self.buf.clone(),
            byte_count: self.byte_count,
        }
    }
}

impl Default for Digest {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── FIPS 180-4 Test Vectors ─────────────────────────────────────────────

    #[test]
    fn fips_empty_string() {
        assert_eq!(
            hex_string(b""),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce\
             47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
    }

    #[test]
    fn fips_abc() {
        assert_eq!(
            hex_string(b"abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
    }

    #[test]
    fn fips_896_bit_message() {
        let msg = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmnoijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
        assert_eq!(msg.len(), 112);
        assert_eq!(
            hex_string(msg),
            "8e959b75dae313da8cf4f72814fc143f8f7779c6eb9f7fa17299aeadb6889018\
             501d289e4900f7e4331b99dec4b5433ac7d329eeb6dd26545e96e55b874be909"
        );
    }

    #[test]
    fn fips_million_a() {
        let data = vec![b'a'; 1_000_000];
        assert_eq!(
            hex_string(&data),
            "e718483d0ce769644e2e42c7bc15b4638e1f98b13b2044285632a803afa973eb\
             de0ff244877ea60a4cb0432ce577c31beb009c5c2c49aa2e4eadb217ad8cc09b"
        );
    }

    // ─── Output Format ───────────────────────────────────────────────────────

    #[test]
    fn digest_is_64_bytes() {
        assert_eq!(sum512(b"").len(), 64);
        assert_eq!(sum512(b"hello world").len(), 64);
        assert_eq!(sum512(&vec![0u8; 1000]).len(), 64);
    }

    #[test]
    fn hex_string_is_128_chars() {
        assert_eq!(hex_string(b"").len(), 128);
        assert_eq!(hex_string(b"hello").len(), 128);
    }

    #[test]
    fn hex_string_is_lowercase() {
        let h = hex_string(b"abc");
        assert!(h.chars().all(|c| !c.is_uppercase()));
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn deterministic() {
        assert_eq!(sum512(b"hello"), sum512(b"hello"));
    }

    #[test]
    fn avalanche() {
        let h1 = sum512(b"hello");
        let h2 = sum512(b"helo");
        assert_ne!(h1, h2);
        let bits_different: u32 = h1
            .iter()
            .zip(h2.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        assert!(bits_different > 100, "only {bits_different} bits differed");
    }

    // ─── Block Boundaries ────────────────────────────────────────────────────
    //
    // SHA-512 processes 128-byte blocks. Key boundaries:
    //   111 bytes: fits in one block (111 + 1 + 16 = 128)
    //   112 bytes: overflows into a second block
    //   128 bytes: one data block + full padding block

    #[test]
    fn block_boundary_111() {
        let r = sum512(&vec![0u8; 111]);
        assert_eq!(r.len(), 64);
        assert_eq!(r, sum512(&vec![0u8; 111]));
    }

    #[test]
    fn block_boundary_112() {
        assert_eq!(sum512(&vec![0u8; 112]).len(), 64);
    }

    #[test]
    fn block_boundary_111_and_112_differ() {
        assert_ne!(sum512(&vec![0u8; 111]), sum512(&vec![0u8; 112]));
    }

    #[test]
    fn block_boundary_128() {
        assert_eq!(sum512(&vec![0u8; 128]).len(), 64);
    }

    #[test]
    fn block_boundary_256() {
        assert_eq!(sum512(&vec![0u8; 256]).len(), 64);
    }

    #[test]
    fn all_boundaries_distinct() {
        let sizes = [111, 112, 127, 128, 255, 256];
        let digests: std::collections::HashSet<[u8; 64]> =
            sizes.iter().map(|&n| sum512(&vec![0u8; n])).collect();
        assert_eq!(digests.len(), 6);
    }

    // ─── Edge Cases ──────────────────────────────────────────────────────────

    #[test]
    fn null_byte_differs_from_empty() {
        assert_ne!(sum512(&[0x00]), sum512(b""));
    }

    #[test]
    fn all_byte_values() {
        let data: Vec<u8> = (0u8..=255).collect();
        assert_eq!(sum512(&data).len(), 64);
    }

    #[test]
    fn every_single_byte_unique() {
        let digests: std::collections::HashSet<[u8; 64]> =
            (0u8..=255).map(|i| sum512(&[i])).collect();
        assert_eq!(digests.len(), 256);
    }

    // ─── Streaming API ───────────────────────────────────────────────────────

    #[test]
    fn streaming_single_write() {
        let mut h = Digest::new();
        h.update(b"abc");
        assert_eq!(h.sum512(), sum512(b"abc"));
    }

    #[test]
    fn streaming_split_at_byte() {
        let mut h = Digest::new();
        h.update(b"ab");
        h.update(b"c");
        assert_eq!(h.sum512(), sum512(b"abc"));
    }

    #[test]
    fn streaming_split_at_block() {
        let data = vec![0u8; 256];
        let mut h = Digest::new();
        h.update(&data[..128]);
        h.update(&data[128..]);
        assert_eq!(h.sum512(), sum512(&data));
    }

    #[test]
    fn streaming_byte_at_a_time() {
        let data: Vec<u8> = (0u8..100).collect();
        let mut h = Digest::new();
        for &b in &data {
            h.update(&[b]);
        }
        assert_eq!(h.sum512(), sum512(&data));
    }

    #[test]
    fn streaming_empty() {
        let h = Digest::new();
        assert_eq!(h.sum512(), sum512(b""));
    }

    #[test]
    fn streaming_nondestructive() {
        let mut h = Digest::new();
        h.update(b"abc");
        assert_eq!(h.sum512(), h.sum512());
    }

    #[test]
    fn streaming_hex_digest() {
        let mut h = Digest::new();
        h.update(b"abc");
        assert!(h.hex_digest().starts_with("ddaf35a193617aba"));
    }

    #[test]
    fn streaming_clone_is_independent() {
        let mut h = Digest::new();
        h.update(b"ab");
        let mut h2 = h.clone_digest();
        h2.update(b"c");
        h.update(b"x");
        assert_eq!(h2.sum512(), sum512(b"abc"));
        assert_eq!(h.sum512(), sum512(b"abx"));
    }

    #[test]
    fn streaming_million_a() {
        let data = vec![b'a'; 1_000_000];
        let mut h = Digest::new();
        h.update(&data[..500_000]);
        h.update(&data[500_000..]);
        assert_eq!(h.sum512(), sum512(&data));
    }
}
