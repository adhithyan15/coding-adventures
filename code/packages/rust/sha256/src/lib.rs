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
    0x428A2F98, 0x71374491, 0xB5C0FBCF, 0xE9B5DBA5,
    0x3956C25B, 0x59F111F1, 0x923F82A4, 0xAB1C5ED5,
    0xD807AA98, 0x12835B01, 0x243185BE, 0x550C7DC3,
    0x72BE5D74, 0x80DEB1FE, 0x9BDC06A7, 0xC19BF174,
    0xE49B69C1, 0xEFBE4786, 0x0FC19DC6, 0x240CA1CC,
    0x2DE92C6F, 0x4A7484AA, 0x5CB0A9DC, 0x76F988DA,
    0x983E5152, 0xA831C66D, 0xB00327C8, 0xBF597FC7,
    0xC6E00BF3, 0xD5A79147, 0x06CA6351, 0x14292967,
    0x27B70A85, 0x2E1B2138, 0x4D2C6DFC, 0x53380D13,
    0x650A7354, 0x766A0ABB, 0x81C2C92E, 0x92722C85,
    0xA2BFE8A1, 0xA81A664B, 0xC24B8B70, 0xC76C51A3,
    0xD192E819, 0xD6990624, 0xF40E3585, 0x106AA070,
    0x19A4C116, 0x1E376C08, 0x2748774C, 0x34B0BCB5,
    0x391C0CB3, 0x4ED8AA4A, 0x5B9CCA4F, 0x682E6FF3,
    0x748F82EE, 0x78A5636F, 0x84C87814, 0x8CC70208,
    0x90BEFFFA, 0xA4506CEB, 0xBEF9A3F7, 0xC67178F2,
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

// ─── Padding ─────────────────────────────────────────────────────────────────
//
// Extends the message to a multiple of 64 bytes per FIPS 180-4 section 5.1.1:
//   1. Append 0x80
//   2. Append zeros until length = 56 (mod 64)
//   3. Append 64-bit big-endian original bit length

fn pad(data: &[u8]) -> Vec<u8> {
    let byte_len = data.len();
    let bit_len: u64 = (byte_len as u64) * 8;

    let mut padded = data.to_vec();
    padded.push(0x80);

    while padded.len() % 64 != 56 {
        padded.push(0x00);
    }

    padded.extend_from_slice(&bit_len.to_be_bytes());
    padded
}

// ─── Message Schedule ────────────────────────────────────────────────────────
//
// Parse 16 big-endian 32-bit words from the block, then expand to 64 words:
//   W[t] = sigma1(W[t-2]) + W[t-7] + sigma0(W[t-15]) + W[t-16]
//
// The non-linear sigma functions (with SHR) make this schedule much stronger
// than SHA-1's linear XOR-rotate expansion.

fn schedule(block: &[u8]) -> [u32; 64] {
    let mut w = [0u32; 64];
    for i in 0..16 {
        w[i] = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
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

fn compress(state: [u32; 8], block: &[u8]) -> [u32; 8] {
    let w = schedule(block);
    let [h0, h1, h2, h3, h4, h5, h6, h7] = state;
    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
        (h0, h1, h2, h3, h4, h5, h6, h7);

    for t in 0..64 {
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
    let padded = pad(data);
    let mut state = INIT;
    for chunk in padded.chunks(64) {
        state = compress(state, chunk);
    }
    let mut digest = [0u8; 32];
    for (i, &word) in state.iter().enumerate() {
        digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    digest
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

/// Streaming SHA-256 hasher that accepts data in multiple chunks.
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
    buf: Vec<u8>,
    byte_count: u64,
}

impl Sha256Hasher {
    /// Create a new streaming hasher initialized with SHA-256 constants.
    pub fn new() -> Self {
        Sha256Hasher {
            state: INIT,
            buf: Vec::new(),
            byte_count: 0,
        }
    }

    /// Feed more bytes into the hash.
    pub fn update(&mut self, data: &[u8]) {
        self.byte_count += data.len() as u64;
        self.buf.extend_from_slice(data);
        while self.buf.len() >= 64 {
            let block: [u8; 64] = self.buf[..64].try_into().unwrap();
            self.state = compress(self.state, &block);
            self.buf.drain(..64);
        }
    }

    /// Return the 32-byte digest of all data fed so far.
    ///
    /// Non-destructive: internal state is not modified.
    pub fn digest(&self) -> [u8; 32] {
        let bit_len: u64 = self.byte_count * 8;
        let mut tail = self.buf.clone();
        tail.push(0x80);
        while tail.len() % 64 != 56 {
            tail.push(0x00);
        }
        tail.extend_from_slice(&bit_len.to_be_bytes());

        let mut state = self.state;
        for chunk in tail.chunks(64) {
            state = compress(state, chunk);
        }

        let mut digest = [0u8; 32];
        for (i, &word) in state.iter().enumerate() {
            digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    /// Return the 64-character hex digest string.
    pub fn hex_digest(&self) -> String {
        self.digest().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Return an independent copy of the current hasher state.
    pub fn clone_hasher(&self) -> Sha256Hasher {
        Sha256Hasher {
            state: self.state,
            buf: self.buf.clone(),
            byte_count: self.byte_count,
        }
    }
}

impl Default for Sha256Hasher {
    fn default() -> Self {
        Self::new()
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
        assert_eq!(sha256(&vec![0u8; 55]).len(), 32);
    }

    #[test]
    fn block_boundary_56() {
        assert_eq!(sha256(&vec![0u8; 56]).len(), 32);
    }

    #[test]
    fn block_boundary_55_and_56_differ() {
        assert_ne!(sha256(&vec![0u8; 55]), sha256(&vec![0u8; 56]));
    }

    #[test]
    fn block_boundary_64() {
        assert_eq!(sha256(&vec![0u8; 64]).len(), 32);
    }

    #[test]
    fn block_boundary_128() {
        assert_eq!(sha256(&vec![0u8; 128]).len(), 32);
    }

    #[test]
    fn all_boundaries_distinct() {
        let sizes = [55, 56, 63, 64, 127, 128];
        let digests: std::collections::HashSet<[u8; 32]> =
            sizes.iter().map(|&n| sha256(&vec![0u8; n])).collect();
        assert_eq!(digests.len(), 6);
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
}
