//! # sha1
//!
//! SHA-1 cryptographic hash function (FIPS 180-4) implemented from scratch.
//!
//! ## What Is SHA-1?
//!
//! SHA-1 (Secure Hash Algorithm 1) takes any sequence of bytes and produces a
//! fixed-size 20-byte (160-bit) "fingerprint" called a digest. The same input
//! always produces the same digest. Change even one bit of input and the digest
//! changes completely — the "avalanche effect". You cannot reverse a digest back
//! to the original input.
//!
//! We implement SHA-1 from scratch (without the `sha1` crate) so every step of
//! the algorithm is visible and explained.
//!
//! ## The Merkle-Damgård Construction
//!
//! SHA-1 processes data in 512-bit (64-byte) blocks:
//!
//! ```text
//! message ──► [pad] ──► block₀ ──► block₁ ──► ... ──► 20-byte digest
//!                            │           │
//!                    [H₀..H₄]──►compress──►compress──►...
//! ```
//!
//! The "state" is five 32-bit words (H₀..H₄), initialized to fixed constants.
//! For each block, 80 rounds of bit mixing fold the block into the state.
//! The final state is the digest.
//!
//! ## Rust Advantages for Cryptography
//!
//! Rust is ideal for hash function implementation because:
//! - `u32` naturally wraps on overflow (`wrapping_add`, or just `+` in release)
//!   — we use `wrapping_add` explicitly to make overflow intent clear.
//! - No garbage collector pause during compression.
//! - The type system prevents mixing signed and unsigned arithmetic.
//! - `[u8; 20]` is a fixed-size, stack-allocated digest — no heap allocation.
//!
//! ## FIPS 180-4 Test Vectors
//!
//! ```
//! use coding_adventures_sha1::sum1;
//! assert_eq!(
//!     sum1(b"abc").iter().map(|b| format!("{:02x}", b)).collect::<String>(),
//!     "a9993e364706816aba3e25717850c26c9cd0d89d"
//! );
//! ```

// ─── Initialization Constants ─────────────────────────────────────────────────
//
// SHA-1 starts with these five 32-bit words as its initial state. They are
// "nothing up my sleeve" numbers — their obvious counting-sequence structure
// (01234567, 89ABCDEF, … reversed in byte pairs) proves no backdoor is hidden.
//
//   H₀ = 0x67452301 → bytes 67 45 23 01 → reverse: 01 23 45 67
//   H₁ = 0xEFCDAB89 → bytes EF CD AB 89 → reverse: 89 AB CD EF

const INIT: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];

// Round constants — one per 20-round stage, derived from square roots:
//   K₀ = floor(sqrt(2)  × 2^30) = 0x5A827999  (rounds 0–19)
//   K₁ = floor(sqrt(3)  × 2^30) = 0x6ED9EBA1  (rounds 20–39)
//   K₂ = floor(sqrt(5)  × 2^30) = 0x8F1BBCDC  (rounds 40–59)
//   K₃ = floor(sqrt(10) × 2^30) = 0xCA62C1D6  (rounds 60–79)

const K: [u32; 4] = [0x5A827999, 0x6ED9EBA1, 0x8F1BBCDC, 0xCA62C1D6];

// ─── Helper: Circular Left Shift ─────────────────────────────────────────────
//
// rotl(n, x) rotates x left by n bit positions within a 32-bit word.
// Bits that "fall off" the left end reappear on the right.
//
// Example: n=2, x=0b01101001 (8-bit for clarity)
//   Regular:  01101001 << 2 = 10100100  (01 on the left is lost)
//   Circular: 01101001 ROTL 2 = 10100110  (01 wraps around)
//
// Rust's `u32::rotate_left` does exactly this — the standard library provides
// it because ROTL is so common in cryptography. It compiles to a single `rol`
// instruction on x86.
#[inline]
fn rotl(n: u32, x: u32) -> u32 {
    x.rotate_left(n)
}

// ─── Padding ─────────────────────────────────────────────────────────────────
//
// The compression function needs exactly 64-byte blocks. Padding extends
// the message per FIPS 180-4 §5.1.1:
//
//   1. Append 0x80 (the '1' bit followed by seven '0' bits).
//   2. Append 0x00 bytes until length ≡ 56 (mod 64).
//   3. Append original bit length as a 64-bit big-endian integer.
//
// Example — "abc" (3 bytes = 24 bits):
//   61 62 63 80 [52 zero bytes] 00 00 00 00 00 00 00 18
//                                                   ^^ 24 in hex
//
// Why return Vec<u8>? The padded message is longer than the input and the
// length is not known at compile time, so we heap-allocate it.
fn pad(data: &[u8]) -> Vec<u8> {
    let byte_len = data.len();
    let bit_len: u64 = (byte_len as u64) * 8;

    let mut padded = data.to_vec();
    padded.push(0x80); // the mandatory '1' bit

    // Append zeros until length ≡ 56 (mod 64)
    while padded.len() % 64 != 56 {
        padded.push(0x00);
    }

    // Append 64-bit big-endian length
    padded.extend_from_slice(&bit_len.to_be_bytes());
    padded
}

// ─── Message Schedule ─────────────────────────────────────────────────────────
//
// Each 64-byte block is parsed as 16 big-endian 32-bit words, then expanded
// to 80 words using:
//
//   W[i] = ROTL(1, W[i-3] XOR W[i-8] XOR W[i-14] XOR W[i-16])  for i ≥ 16
//
// Why expand to 80? More words → more mixing → better avalanche.
//
// `u32::from_be_bytes` reads a 4-byte slice as a big-endian integer.
// Rust's `.try_into().unwrap()` converts a `&[u8]` slice to `[u8; 4]` —
// the compiler checks the size at compile time.
fn schedule(block: &[u8]) -> [u32; 80] {
    let mut w = [0u32; 80];
    for i in 0..16 {
        w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
    }
    for i in 16..80 {
        w[i] = rotl(1, w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]);
    }
    w
}

// ─── Compression Function ─────────────────────────────────────────────────────
//
// 80 rounds of mixing fold one 64-byte block into the five-word state.
//
// Four stages of 20 rounds each, each using a different auxiliary function:
//
//   Stage  Rounds  f(b,c,d)                    Purpose
//   ─────  ──────  ──────────────────────────  ─────────────────
//     1    0–19    (b & c) | (!b & d)          Selector / mux
//     2    20–39   b ^ c ^ d                   Parity
//     3    40–59   (b&c) | (b&d) | (c&d)       Majority vote
//     4    60–79   b ^ c ^ d                   Parity again
//
// Each round:
//   temp = ROTL(5, a) + f(b,c,d) + e + K + W[t]   (wrapping mod 2^32)
//   shift: e=d, d=c, c=ROTL(30,b), b=a, a=temp
//
// Davies-Meyer feed-forward: after all 80 rounds, add compressed output
// back to input state to prevent invertibility.
//
// `wrapping_add` makes the intent explicit: we want mod 2^32 arithmetic.
fn compress(state: [u32; 5], block: &[u8]) -> [u32; 5] {
    let w = schedule(block);
    let [h0, h1, h2, h3, h4] = state;
    let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);

    for t in 0..80 {
        let (f, k) = match t {
            // Selector: if b=1 output c, if b=0 output d
            0..=19 => ((b & c) | (!b & d), K[0]),
            // Parity: 1 if an odd number of inputs are 1
            20..=39 => (b ^ c ^ d, K[1]),
            // Majority: 1 if at least 2 of the 3 inputs are 1
            40..=59 => ((b & c) | (b & d) | (c & d), K[2]),
            // Parity again (same formula, different constant)
            _ => (b ^ c ^ d, K[3]),
        };

        let temp = rotl(5, a)
            .wrapping_add(f)
            .wrapping_add(e)
            .wrapping_add(k)
            .wrapping_add(w[t]);
        e = d;
        d = c;
        c = rotl(30, b);
        b = a;
        a = temp;
    }

    [
        h0.wrapping_add(a),
        h1.wrapping_add(b),
        h2.wrapping_add(c),
        h3.wrapping_add(d),
        h4.wrapping_add(e),
    ]
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Compute the SHA-1 digest of `data`. Returns a `[u8; 20]` array.
///
/// This is the one-shot API: hash a complete message in a single call.
///
/// We name this `sum1` (not `sum`) to avoid clashing with any `Sum` traits.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha1::sum1;
/// let digest = sum1(b"abc");
/// let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
/// assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
/// ```
pub fn sum1(data: &[u8]) -> [u8; 20] {
    let padded = pad(data);
    let mut state = INIT;
    for chunk in padded.chunks(64) {
        state = compress(state, chunk);
    }
    // Finalize: write five 32-bit words as big-endian bytes.
    // `to_be_bytes()` converts a u32 to its 4 big-endian bytes.
    let mut digest = [0u8; 20];
    for (i, &word) in state.iter().enumerate() {
        digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// Compute SHA-1 and return the 40-character lowercase hex string.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha1::hex_string;
/// assert_eq!(
///     hex_string(b"abc"),
///     "a9993e364706816aba3e25717850c26c9cd0d89d"
/// );
/// ```
pub fn hex_string(data: &[u8]) -> String {
    sum1(data).iter().map(|b| format!("{:02x}", b)).collect()
}

/// Streaming SHA-1 hasher that accepts data in multiple chunks.
///
/// Useful when the full message is not available at once — for example when
/// reading a large file in chunks or hashing a network stream.
///
/// # Examples
///
/// ```
/// use coding_adventures_sha1::{Digest, sum1};
/// let mut h = Digest::new();
/// h.update(b"ab");
/// h.update(b"c");
/// assert_eq!(h.sum1(), sum1(b"abc"));
/// ```
///
/// Multiple `update` calls are equivalent to a single `sum1(all_data)`.
pub struct Digest {
    state: [u32; 5],
    buf: Vec<u8>,   // partial block (< 64 bytes)
    byte_count: u64, // total bytes fed in
}

impl Digest {
    /// Initialize a new streaming hasher with SHA-1's starting constants.
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
        // Compress any complete 64-byte blocks to keep buf small
        while self.buf.len() >= 64 {
            let block: [u8; 64] = self.buf[..64].try_into().unwrap();
            self.state = compress(self.state, &block);
            self.buf.drain(..64);
        }
    }

    /// Return the 20-byte digest of all data fed so far.
    ///
    /// Non-destructive: the internal state is not modified, so you can
    /// continue calling `update` after calling `sum1`.
    pub fn sum1(&self) -> [u8; 20] {
        // Pad the remaining buffer using the TOTAL byte count
        let bit_len: u64 = self.byte_count * 8;
        let mut tail = self.buf.clone();
        tail.push(0x80);
        while tail.len() % 64 != 56 {
            tail.push(0x00);
        }
        tail.extend_from_slice(&bit_len.to_be_bytes());

        // Compress the padding tail against a copy of the live state
        let mut state = self.state;
        for chunk in tail.chunks(64) {
            state = compress(state, chunk);
        }

        let mut digest = [0u8; 20];
        for (i, &word) in state.iter().enumerate() {
            digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    /// Return the 40-character hex string of the digest.
    pub fn hex_digest(&self) -> String {
        self.sum1().iter().map(|b| format!("{:02x}", b)).collect()
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── FIPS 180-4 Test Vectors ─────────────────────────────────────────────

    #[test]
    fn fips_empty_string() {
        assert_eq!(hex_string(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn fips_abc() {
        assert_eq!(
            hex_string(b"abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }

    #[test]
    fn fips_448_bit_message() {
        let msg = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(msg.len(), 56);
        assert_eq!(hex_string(msg), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
    }

    #[test]
    fn fips_million_a() {
        let data = vec![b'a'; 1_000_000];
        assert_eq!(hex_string(&data), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
    }

    // ─── Output Format ───────────────────────────────────────────────────────

    #[test]
    fn digest_is_20_bytes() {
        assert_eq!(sum1(b"").len(), 20);
        assert_eq!(sum1(b"hello world").len(), 20);
        assert_eq!(sum1(&vec![0u8; 1000]).len(), 20);
    }

    #[test]
    fn hex_string_is_40_chars() {
        assert_eq!(hex_string(b"").len(), 40);
        assert_eq!(hex_string(b"hello").len(), 40);
    }

    #[test]
    fn hex_string_is_lowercase() {
        let h = hex_string(b"abc");
        assert!(h.chars().all(|c| !c.is_uppercase()));
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn deterministic() {
        assert_eq!(sum1(b"hello"), sum1(b"hello"));
    }

    #[test]
    fn avalanche() {
        let h1 = sum1(b"hello");
        let h2 = sum1(b"helo");
        assert_ne!(h1, h2);
        let bits_different: u32 = h1
            .iter()
            .zip(h2.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        assert!(bits_different > 20, "only {bits_different} bits differed");
    }

    // ─── Block Boundaries ────────────────────────────────────────────────────

    #[test]
    fn block_boundary_55() {
        let r = sum1(&vec![0u8; 55]);
        assert_eq!(r.len(), 20);
        assert_eq!(r, sum1(&vec![0u8; 55]));
    }

    #[test]
    fn block_boundary_56() {
        assert_eq!(sum1(&vec![0u8; 56]).len(), 20);
    }

    #[test]
    fn block_boundary_55_and_56_differ() {
        assert_ne!(sum1(&vec![0u8; 55]), sum1(&vec![0u8; 56]));
    }

    #[test]
    fn block_boundary_64() {
        assert_eq!(sum1(&vec![0u8; 64]).len(), 20);
    }

    #[test]
    fn block_boundary_128() {
        assert_eq!(sum1(&vec![0u8; 128]).len(), 20);
    }

    #[test]
    fn all_boundaries_distinct() {
        let sizes = [55, 56, 63, 64, 127, 128];
        let digests: std::collections::HashSet<[u8; 20]> =
            sizes.iter().map(|&n| sum1(&vec![0u8; n])).collect();
        assert_eq!(digests.len(), 6);
    }

    // ─── Edge Cases ──────────────────────────────────────────────────────────

    #[test]
    fn null_byte_differs_from_empty() {
        assert_ne!(sum1(&[0x00]), sum1(b""));
    }

    #[test]
    fn all_byte_values() {
        let data: Vec<u8> = (0u8..=255).collect();
        assert_eq!(sum1(&data).len(), 20);
    }

    #[test]
    fn every_single_byte_unique() {
        let digests: std::collections::HashSet<[u8; 20]> =
            (0u8..=255).map(|i| sum1(&[i])).collect();
        assert_eq!(digests.len(), 256);
    }

    // ─── Streaming API ───────────────────────────────────────────────────────

    #[test]
    fn streaming_single_write() {
        let mut h = Digest::new();
        h.update(b"abc");
        assert_eq!(h.sum1(), sum1(b"abc"));
    }

    #[test]
    fn streaming_split_at_byte() {
        let mut h = Digest::new();
        h.update(b"ab");
        h.update(b"c");
        assert_eq!(h.sum1(), sum1(b"abc"));
    }

    #[test]
    fn streaming_split_at_block() {
        let data = vec![0u8; 128];
        let mut h = Digest::new();
        h.update(&data[..64]);
        h.update(&data[64..]);
        assert_eq!(h.sum1(), sum1(&data));
    }

    #[test]
    fn streaming_byte_at_a_time() {
        let data: Vec<u8> = (0u8..100).collect();
        let mut h = Digest::new();
        for &b in &data {
            h.update(&[b]);
        }
        assert_eq!(h.sum1(), sum1(&data));
    }

    #[test]
    fn streaming_empty() {
        let h = Digest::new();
        assert_eq!(h.sum1(), sum1(b""));
    }

    #[test]
    fn streaming_nondestructive() {
        let mut h = Digest::new();
        h.update(b"abc");
        assert_eq!(h.sum1(), h.sum1());
    }

    #[test]
    fn streaming_hex_digest() {
        let mut h = Digest::new();
        h.update(b"abc");
        assert_eq!(h.hex_digest(), "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn streaming_clone_is_independent() {
        let mut h = Digest::new();
        h.update(b"ab");
        let mut h2 = h.clone_digest();
        h2.update(b"c");
        h.update(b"x");
        assert_eq!(h2.sum1(), sum1(b"abc"));
        assert_eq!(h.sum1(), sum1(b"abx"));
    }

    #[test]
    fn streaming_million_a() {
        let data = vec![b'a'; 1_000_000];
        let mut h = Digest::new();
        h.update(&data[..500_000]);
        h.update(&data[500_000..]);
        assert_eq!(h.sum1(), sum1(&data));
    }
}
