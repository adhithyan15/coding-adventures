//! # md5
//!
//! MD5 message digest algorithm (RFC 1321) implemented from scratch.
//!
//! ## What Is MD5?
//!
//! MD5 (Message-Digest Algorithm 5) takes any sequence of bytes and produces a
//! fixed-size 16-byte (128-bit) "fingerprint" called a digest. The same input
//! always produces the same digest. Change even one bit of input and the digest
//! changes completely — the "avalanche effect". You cannot reverse a digest back
//! to the original input.
//!
//! MD5 was designed by Ron Rivest in 1991 (RFC 1321) to replace the earlier MD4.
//! Although MD5 is no longer considered cryptographically secure for signatures
//! (collision attacks exist), it remains widely used as a checksum for file
//! integrity verification, and implementing it teaches core concepts shared by
//! all Merkle-Damgård hash functions including SHA-1 and SHA-2.
//!
//! ## Key Difference from SHA-1: Little-Endian
//!
//! MD5 uses **little-endian** byte order throughout, while SHA-1 uses big-endian.
//! This means when we interpret four bytes as a 32-bit integer, byte[0] is the
//! *least* significant byte, not the most significant:
//!
//! ```text
//! Bytes: [0x01, 0x00, 0x00, 0x00]
//!   Big-endian    → 0x00000001 = 1
//!   Little-endian → 0x01000000 = 16777216
//! ```
//!
//! x86 and ARM processors are natively little-endian, so MD5's choice matches
//! the hardware most computers use today. Rust's `u32::from_le_bytes` and
//! `word.to_le_bytes()` handle the conversion correctly.
//!
//! ## The Merkle-Damgård Construction
//!
//! MD5 processes data in 512-bit (64-byte) blocks:
//!
//! ```text
//! message ──► [pad] ──► block₀ ──► block₁ ──► ... ──► 16-byte digest
//!                            │           │
//!                    [A,B,C,D]──►compress──►compress──►...
//! ```
//!
//! The "state" is four 32-bit words (A, B, C, D), initialized to fixed constants.
//! For each block, 64 rounds of bit mixing fold the block into the state.
//! The final state is the digest.
//!
//! ## The T-Table (Sine-Derived Constants)
//!
//! MD5 uses 64 round constants, one per round. Each constant is derived from the
//! absolute value of the sine function:
//!
//! ```text
//! T[i] = floor(abs(sin(i + 1)) × 2^32)   for i = 0..63
//! ```
//!
//! Why sine? It's a "nothing up my sleeve" number — using an unambiguous,
//! independently verifiable formula proves no one smuggled a backdoor into the
//! constants. Sine oscillates between -1 and +1, so `abs(sin(x)) × 2^32` fills
//! the full 32-bit integer range in a pseudo-random, unpredictable pattern.
//!
//! Since Rust `const fn` cannot call `f64::sin()` at compile time, we hardcode
//! the precomputed table. These values are universally agreed upon and form part
//! of the RFC 1321 specification.
//!
//! ## RFC 1321 Test Vectors
//!
//! ```
//! use ca_md5::hex_string;
//! assert_eq!(hex_string(b""), "d41d8cd98f00b204e9800998ecf8427e");
//! assert_eq!(hex_string(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
//! ```

// ─── The T-Table: 64 Sine-Derived Round Constants ────────────────────────────
//
// Computed by: T[i] = floor(abs(sin(i + 1)) × 4294967296.0)   for i in 0..64
//
// These are fixed, universal values from RFC 1321, Appendix T.
// Each hex value was verified against the formula above.
//
// The sine function is transcendental (cannot be expressed as a ratio of
// integers), so these values are effectively arbitrary bits — the exact
// property we want to prevent any algebraic shortcut attacks on the rounds.

const T: [u32; 64] = [
    // Round 1 (FF): indices 0–15
    0xD76AA478, 0xE8C7B756, 0x242070DB, 0xC1BDCEEE,
    0xF57C0FAF, 0x4787C62A, 0xA8304613, 0xFD469501,
    0x698098D8, 0x8B44F7AF, 0xFFFF5BB1, 0x895CD7BE,
    0x6B901122, 0xFD987193, 0xA679438E, 0x49B40821,
    // Round 2 (GG): indices 16–31
    0xF61E2562, 0xC040B340, 0x265E5A51, 0xE9B6C7AA,
    0xD62F105D, 0x02441453, 0xD8A1E681, 0xE7D3FBC8,
    0x21E1CDE6, 0xC33707D6, 0xF4D50D87, 0x455A14ED,
    0xA9E3E905, 0xFCEFA3F8, 0x676F02D9, 0x8D2A4C8A,
    // Round 3 (HH): indices 32–47
    0xFFFA3942, 0x8771F681, 0x6D9D6122, 0xFDE5380C,
    0xA4BEEA44, 0x4BDECFA9, 0xF6BB4B60, 0xBEBFBC70,
    0x289B7EC6, 0xEAA127FA, 0xD4EF3085, 0x04881D05,
    0xD9D4D039, 0xE6DB99E5, 0x1FA27CF8, 0xC4AC5665,
    // Round 4 (II): indices 48–63
    0xF4292244, 0x432AFF97, 0xAB9423A7, 0xFC93A039,
    0x655B59C3, 0x8F0CCC92, 0xFFEFF47D, 0x85845DD1,
    0x6FA87E4F, 0xFE2CE6E0, 0xA3014314, 0x4E0811A1,
    0xF7537E82, 0xBD3AF235, 0x2AD7D2BB, 0xEB86D391,
];

// ─── Shift Amounts ────────────────────────────────────────────────────────────
//
// Each of the 64 rounds rotates the accumulator left by a specific number of
// bits. The pattern repeats every 16 rounds but shifts within each group:
//
//   Stage 1 (FF): 7, 12, 17, 22 — repeated 4 times
//   Stage 2 (GG): 5,  9, 14, 20 — repeated 4 times
//   Stage 3 (HH): 4, 11, 16, 23 — repeated 4 times
//   Stage 4 (II): 6, 10, 15, 21 — repeated 4 times
//
// These values were chosen by the algorithm's designers to maximize diffusion:
// each bit of input should influence each bit of output after a few rounds.

const S: [u32; 64] = [
    // Stage 1
     7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,  7, 12, 17, 22,
    // Stage 2
     5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,  5,  9, 14, 20,
    // Stage 3
     4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,  4, 11, 16, 23,
    // Stage 4
     6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,  6, 10, 15, 21,
];

// ─── Initialization Constants ─────────────────────────────────────────────────
//
// MD5 starts with these four 32-bit words as its initial state. Like SHA-1,
// they are "nothing up my sleeve" numbers. Written as bytes in little-endian
// order, they spell out a simple counting sequence:
//
//   A = 0x67452301 → LE bytes: 01 23 45 67
//   B = 0xEFCDAB89 → LE bytes: 89 AB CD EF
//   C = 0x98BADCFE → LE bytes: FE DC BA 98
//   D = 0x10325476 → LE bytes: 76 54 32 10
//
// The counting sequence 01, 23, 45, 67, 89, AB, CD, EF, FE, DC, BA, 98, ...
// is clearly not a hidden backdoor — anyone can verify it without trusting
// the algorithm designers.

const INIT: [u32; 4] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476];

// ─── Helper: Circular Left Rotation ──────────────────────────────────────────
//
// rotl(s, x) rotates x left by s bit positions within a 32-bit word.
// Bits that "fall off" the left end wrap around to the right end.
//
// Example (8-bit for clarity, actual is 32-bit):
//   s=2, x=0b11000001
//   shift left 2:  0b00000100  (bits 11 lost off the left)
//   shift right 6: 0b00000011  (bits 11 recovered on the right)
//   OR together:   0b00000111
//
// Rust's `u32::rotate_left` compiles to a single `rol` CPU instruction on x86.

#[inline]
fn rotl(s: u32, x: u32) -> u32 {
    x.rotate_left(s)
}

// ─── Padding ─────────────────────────────────────────────────────────────────
//
// The compression function needs exactly 64-byte blocks. Padding extends
// the message per RFC 1321 §3.1–3.2:
//
//   1. Append 0x80 (the '1' bit followed by seven '0' bits).
//   2. Append 0x00 bytes until length ≡ 56 (mod 64).
//   3. Append original bit length as a 64-bit **little-endian** integer.
//
// CRITICAL: The length is appended in little-endian byte order — the key
// difference from SHA-1 which uses big-endian. This is not a bug; it is
// specified in RFC 1321 §3.2.
//
// Example — "abc" (3 bytes = 24 bits):
//   61 62 63 80 [52 zero bytes] 18 00 00 00 00 00 00 00
//                                ^^ 24 = 0x18, in little-endian
//
// The padded length is always a multiple of 64, so the compression loop
// will process whole blocks with no remainder.

fn pad(data: &[u8]) -> Vec<u8> {
    let byte_len = data.len();
    // Bit length encoded as little-endian 64-bit integer (RFC 1321 §3.2)
    let bit_len: u64 = (byte_len as u64) * 8;

    let mut padded = data.to_vec();
    padded.push(0x80); // mandatory "1" bit followed by seven "0" bits

    // Append zeros until length ≡ 56 (mod 64)
    // We need 8 bytes at the end for the length field, hence 64 - 8 = 56
    while padded.len() % 64 != 56 {
        padded.push(0x00);
    }

    // Append 64-bit little-endian bit length (NOT big-endian like SHA-1!)
    padded.extend_from_slice(&bit_len.to_le_bytes());
    padded
}

// ─── Compression Function ─────────────────────────────────────────────────────
//
// 64 rounds of mixing fold one 64-byte block into the four-word state.
//
// Four stages of 16 rounds each, each using a different auxiliary function
// operating on three of the four state words (b, c, d):
//
//   Stage  Rounds  Name  f(b,c,d)               g (word index)  Purpose
//   ─────  ──────  ────  ─────────────────────  ─────────────── ────────────
//     1    0–15    FF    (b & c) | (!b & d)      i               Multiplexer
//     2    16–31   GG    (d & b) | (!d & c)      (5i + 1) % 16  Multiplexer
//     3    32–47   HH    b ^ c ^ d               (3i + 5) % 16  Parity
//     4    48–63   II    c ^ (b | !d)            (7i) % 16      Near-identity
//
// The multiplexer function in Stage 1: `(b & c) | (!b & d)` means
//   "if bit of b is 1, output the corresponding bit of c; else output d".
// This is the fundamental Boolean 2-to-1 multiplexer (MUX) gate.
//
// Each round update (Davies-Meyer feed-forward style):
//   temp = b + ROTL(s, a + f + m[g] + T[i])
//   (a, b, c, d) ← (d, temp, b, c)
//
// Note how (a,b,c,d) rotate: a→d position, b stays (enriched), old b→c, old c→d.
// This means every word participates in every round, just in a different role.
//
// CRITICAL: All additions use `wrapping_add` — we deliberately want mod-2^32
// arithmetic (unsigned overflow). Without wrapping, debug builds would panic
// on overflow. In release builds `+` on u32 also wraps, but being explicit
// makes the intent clear to every reader.
//
// Block parsing: `u32::from_le_bytes` — little-endian! Bytes arrive in memory
// order; byte 0 is the least-significant byte of word 0.

fn compress(state: [u32; 4], block: &[u8]) -> [u32; 4] {
    // Parse the 64-byte block as 16 little-endian 32-bit words
    // `try_into().unwrap()` converts &[u8] slice to [u8; 4] — checked at runtime
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u32::from_le_bytes(block[i * 4..i * 4 + 4].try_into().unwrap());
    }

    let [init_a, init_b, init_c, init_d] = state;
    let (mut a, mut b, mut c, mut d) = (init_a, init_b, init_c, init_d);

    for i in 0..64 {
        // Compute auxiliary function f and word index g based on current stage
        let (f, g) = if i < 16 {
            // Stage 1 — FF: multiplexer selects c when b=1, d when b=0
            // `!b` is bitwise NOT for u32 in Rust (not logical NOT)
            ((b & c) | (!b & d), i)
        } else if i < 32 {
            // Stage 2 — GG: reversed multiplexer (d selects between b and c)
            ((d & b) | (!d & c), (5 * i + 1) % 16)
        } else if i < 48 {
            // Stage 3 — HH: parity (XOR of all three — 1 iff odd number of 1s)
            (b ^ c ^ d, (3 * i + 5) % 16)
        } else {
            // Stage 4 — II: near-identity (c XOR (b OR NOT-d))
            // NOT-d applied before OR ensures II is not the same as HH
            (c ^ (b | !d), (7 * i) % 16)
        };

        // Core round update:
        //   1. Accumulate a + f + m[g] + T[i]  (all mod 2^32)
        //   2. Rotate left by S[i] bits
        //   3. Add b (mod 2^32) → this becomes the new b
        //   4. Rotate register set: (a,b,c,d) ← (d, temp, b, c)
        let temp = b.wrapping_add(rotl(
            S[i],
            a.wrapping_add(f)
                .wrapping_add(m[g])
                .wrapping_add(T[i]),
        ));
        // Shift all registers one position: d→a, temp→b, b→c, c→d
        a = d;
        d = c;
        c = b;
        b = temp;
    }

    // Davies-Meyer feed-forward: add the original state back to the compressed
    // output. This prevents the compression function from being easily inverted —
    // even if an attacker can invert the 64 rounds, they'd need to subtract the
    // original state, which they may not know.
    [
        init_a.wrapping_add(a),
        init_b.wrapping_add(b),
        init_c.wrapping_add(c),
        init_d.wrapping_add(d),
    ]
}

// ─── Finalization ─────────────────────────────────────────────────────────────
//
// Converts the four 32-bit state words into 16 bytes for the final digest.
//
// CRITICAL: Output bytes are written in **little-endian** order.
// Word A = 0x67452301 → bytes 01 23 45 67 (not 67 45 23 01!)
//
// This means the first byte of the MD5 digest is the *least-significant*
// byte of word A. This is the opposite of SHA-1/SHA-256, which are big-endian.

fn finalize(state: [u32; 4]) -> [u8; 16] {
    let mut digest = [0u8; 16];
    for (i, &word) in state.iter().enumerate() {
        // to_le_bytes(): byte 0 is the least-significant byte of `word`
        digest[i * 4..i * 4 + 4].copy_from_slice(&word.to_le_bytes());
    }
    digest
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Compute the MD5 digest of `data`. Returns a `[u8; 16]` array.
///
/// This is the one-shot API: hash a complete message in a single call.
/// It pads the data, compresses each 64-byte block, and returns the 16-byte
/// digest in little-endian word order as specified by RFC 1321.
///
/// # Examples
///
/// ```
/// use ca_md5::sum_md5;
/// // RFC 1321 test vector: empty string
/// let digest = sum_md5(b"");
/// let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
/// assert_eq!(hex, "d41d8cd98f00b204e9800998ecf8427e");
/// ```
pub fn sum_md5(data: &[u8]) -> [u8; 16] {
    let padded = pad(data);
    let mut state = INIT;
    for chunk in padded.chunks(64) {
        state = compress(state, chunk);
    }
    finalize(state)
}

/// Compute MD5 and return the 32-character lowercase hex string.
///
/// # Examples
///
/// ```
/// use ca_md5::hex_string;
/// assert_eq!(hex_string(b""), "d41d8cd98f00b204e9800998ecf8427e");
/// assert_eq!(hex_string(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
/// ```
pub fn hex_string(data: &[u8]) -> String {
    sum_md5(data).iter().map(|b| format!("{:02x}", b)).collect()
}

/// Streaming MD5 hasher that accepts data in multiple chunks.
///
/// Useful when the full message is not available at once — for example when
/// reading a large file in chunks or hashing a network stream.
///
/// Internally, this accumulates data in a buffer. Complete 64-byte blocks
/// are compressed immediately to keep memory usage constant regardless of
/// total message size. The remaining partial block (< 64 bytes) is held
/// until `sum_md5()` is called, at which point padding is applied.
///
/// # Examples
///
/// ```
/// use ca_md5::{Digest, sum_md5};
/// let mut h = Digest::new();
/// h.update(b"ab");
/// h.update(b"c");
/// assert_eq!(h.sum_md5(), sum_md5(b"abc"));
/// ```
pub struct Digest {
    /// Current four-word compression state (updated as complete blocks arrive)
    state: [u32; 4],
    /// Partial block buffer — holds bytes not yet compressed (0..63 bytes)
    buf: Vec<u8>,
    /// Total number of bytes fed in (needed to compute the padding length field)
    byte_count: u64,
}

impl Digest {
    /// Initialize a new streaming hasher with MD5's four starting constants.
    pub fn new() -> Self {
        Digest {
            state: INIT,
            buf: Vec::new(),
            byte_count: 0,
        }
    }

    /// Feed more bytes into the hash.
    ///
    /// Any complete 64-byte blocks are compressed immediately.
    /// Remaining bytes are buffered until `sum_md5()` or `hex_digest()` is called.
    pub fn update(&mut self, data: &[u8]) {
        self.byte_count += data.len() as u64;
        self.buf.extend_from_slice(data);
        // Compress all complete 64-byte blocks eagerly to keep buf small
        while self.buf.len() >= 64 {
            let block: [u8; 64] = self.buf[..64].try_into().unwrap();
            self.state = compress(self.state, &block);
            self.buf.drain(..64);
        }
    }

    /// Return the 16-byte MD5 digest of all data fed so far.
    ///
    /// Non-destructive: the internal state is not modified, so you can
    /// continue calling `update` after `sum_md5()` and get a valid digest
    /// of all bytes seen including the new ones.
    ///
    /// Internally, this clones the state and buffer, applies padding to the
    /// clone, and returns the finalized digest without touching `self`.
    pub fn sum_md5(&self) -> [u8; 16] {
        // Padding appended to a COPY of the buffer — self is not modified.
        // The total bit count uses self.byte_count, which tracks all bytes
        // ever passed to update(), including those already compressed.
        let bit_len: u64 = self.byte_count * 8;
        let mut tail = self.buf.clone();
        tail.push(0x80);
        while tail.len() % 64 != 56 {
            tail.push(0x00);
        }
        // Little-endian length — the MD5-specific requirement
        tail.extend_from_slice(&bit_len.to_le_bytes());

        // Compress the padding tail against a copy of the live state
        let mut state = self.state;
        for chunk in tail.chunks(64) {
            state = compress(state, chunk);
        }

        finalize(state)
    }

    /// Return the 32-character lowercase hex string of the digest.
    pub fn hex_digest(&self) -> String {
        self.sum_md5().iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Return an independent copy of the current hasher state.
    ///
    /// Useful for computing multiple digests that share a common prefix.
    /// After cloning, updates to either digest are completely independent.
    ///
    /// # Examples
    ///
    /// ```
    /// use ca_md5::Digest;
    /// let mut h = Digest::new();
    /// h.update(b"hello");
    /// let mut h2 = h.clone_digest();   // diverge here
    /// h.update(b" world");
    /// h2.update(b" rust");
    /// // h  hashes "hello world"
    /// // h2 hashes "hello rust"
    /// assert_ne!(h.sum_md5(), h2.sum_md5());
    /// ```
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

    // ─── RFC 1321 Test Vectors ────────────────────────────────────────────────
    //
    // These are the official MD5 test vectors from RFC 1321, Appendix A.5.
    // Any correct MD5 implementation must produce exactly these outputs.
    // We verify them here so that any regression in the algorithm is caught
    // immediately.

    #[test]
    fn rfc1321_empty_string() {
        // Empty string — tests that padding works when there is no data at all.
        // Padded form: 0x80 followed by 55 zeros followed by 0x00×8 (zero bit length)
        assert_eq!(hex_string(b""), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn rfc1321_a() {
        assert_eq!(hex_string(b"a"), "0cc175b9c0f1b6a831c399e269772661");
    }

    #[test]
    fn rfc1321_abc() {
        assert_eq!(hex_string(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn rfc1321_message_digest() {
        assert_eq!(
            hex_string(b"message digest"),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
    }

    #[test]
    fn rfc1321_lowercase_alphabet() {
        assert_eq!(
            hex_string(b"abcdefghijklmnopqrstuvwxyz"),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
    }

    #[test]
    fn rfc1321_mixed_alphanumeric() {
        assert_eq!(
            hex_string(b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
    }

    #[test]
    fn rfc1321_numeric_string() {
        assert_eq!(
            hex_string(b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    // ─── Output Format ────────────────────────────────────────────────────────

    #[test]
    fn digest_is_16_bytes() {
        // MD5 always produces exactly 128 bits = 16 bytes
        assert_eq!(sum_md5(b"").len(), 16);
        assert_eq!(sum_md5(b"hello world").len(), 16);
        assert_eq!(sum_md5(&vec![0u8; 1000]).len(), 16);
    }

    #[test]
    fn hex_string_is_32_chars() {
        // Each byte = 2 hex digits → 16 bytes = 32 characters
        assert_eq!(hex_string(b"").len(), 32);
        assert_eq!(hex_string(b"hello").len(), 32);
        assert_eq!(hex_string(&vec![0xffu8; 64]).len(), 32);
    }

    #[test]
    fn hex_string_is_lowercase_hex() {
        // Hex digits must be lowercase (0–9, a–f), never uppercase A–F
        let h = hex_string(b"abc");
        assert!(h.chars().all(|c| !c.is_uppercase()), "found uppercase chars in: {h}");
        assert!(
            h.chars().all(|c| c.is_ascii_hexdigit()),
            "found non-hex chars in: {h}"
        );
    }

    #[test]
    fn deterministic() {
        // Same input → identical output on every call, forever
        assert_eq!(sum_md5(b"hello"), sum_md5(b"hello"));
        assert_eq!(hex_string(b"test"), hex_string(b"test"));
    }

    #[test]
    fn avalanche_effect() {
        // Changing one character should flip roughly half of all 128 output bits.
        // We assert at least 20 bits differ — well below the expected ~64.
        let h1 = sum_md5(b"hello");
        let h2 = sum_md5(b"helo");
        assert_ne!(h1, h2);
        let bits_different: u32 = h1
            .iter()
            .zip(h2.iter())
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        assert!(bits_different > 20, "only {bits_different} bits differed — weak avalanche");
    }

    // ─── Little-Endian Verification ───────────────────────────────────────────
    //
    // These tests specifically verify that the implementation uses little-endian
    // byte order for block parsing and output, as required by RFC 1321.

    #[test]
    fn little_endian_output_byte_order() {
        // The digest of "a" is known to be 0cc175b9c0f1b6a831c399e269772661.
        // If we were big-endian, the word order would be reversed.
        let d = sum_md5(b"a");
        let hex: String = d.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "0cc175b9c0f1b6a831c399e269772661");
        // First byte of digest is 0x0c — the LSB of the first 32-bit word
        assert_eq!(d[0], 0x0c);
        assert_eq!(d[1], 0xc1);
        assert_eq!(d[2], 0x75);
        assert_eq!(d[3], 0xb9);
    }

    #[test]
    fn little_endian_matches_all_rfc_vectors() {
        // If any vector fails, the byte-order is almost certainly wrong.
        // A big-endian implementation would produce completely different outputs.
        assert_eq!(hex_string(b""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(hex_string(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    // ─── Block Boundaries ─────────────────────────────────────────────────────
    //
    // MD5 processes 64-byte blocks. Messages at and around these boundaries
    // exercise the padding logic. Particularly important:
    //
    //   55 bytes: fits in one block (55 + 1 pad byte + 8 length = 64)
    //   56 bytes: needs two blocks (56 + 1 pad forces overflow into block 2)
    //   63 bytes: one byte short of a full block
    //   64 bytes: exactly one full block — padding goes into a second block
    //  127 bytes: one byte short of two full blocks
    //  128 bytes: exactly two full blocks — padding in a third block

    #[test]
    fn block_boundary_55() {
        // 55 bytes: fits in exactly one 64-byte block after padding
        // 55 + 1 (0x80) + 0 zeros + 8 (length) = 64 ✓
        let d = sum_md5(&vec![0u8; 55]);
        assert_eq!(d.len(), 16);
        assert_eq!(d, sum_md5(&vec![0u8; 55])); // deterministic
    }

    #[test]
    fn block_boundary_56() {
        // 56 bytes: requires TWO 64-byte blocks after padding
        // 56 + 1 (0x80) = 57, need 56 mod 64 → must pad to 120 + 8 = 128
        assert_eq!(sum_md5(&vec![0u8; 56]).len(), 16);
    }

    #[test]
    fn block_boundary_55_and_56_differ() {
        // The two messages are different so their digests must differ
        assert_ne!(sum_md5(&vec![0u8; 55]), sum_md5(&vec![0u8; 56]));
    }

    #[test]
    fn block_boundary_63() {
        assert_eq!(sum_md5(&vec![0u8; 63]).len(), 16);
    }

    #[test]
    fn block_boundary_64() {
        // 64 bytes exactly: the data fills one block, padding goes into a second
        assert_eq!(sum_md5(&vec![0u8; 64]).len(), 16);
    }

    #[test]
    fn block_boundary_128() {
        assert_eq!(sum_md5(&vec![0u8; 128]).len(), 16);
    }

    #[test]
    fn all_boundaries_produce_distinct_digests() {
        // All boundary sizes must produce different digests (they're distinct inputs)
        let sizes = [0usize, 55, 56, 63, 64, 127, 128];
        let digests: std::collections::HashSet<[u8; 16]> =
            sizes.iter().map(|&n| sum_md5(&vec![0u8; n])).collect();
        assert_eq!(
            digests.len(),
            sizes.len(),
            "two boundary inputs produced the same digest — collision!"
        );
    }

    // ─── Edge Cases ───────────────────────────────────────────────────────────

    #[test]
    fn null_byte_differs_from_empty() {
        // A single zero byte is a different message from the empty string
        assert_ne!(sum_md5(&[0x00]), sum_md5(b""));
    }

    #[test]
    fn all_zero_bytes_256() {
        // Large all-zero buffer — exercises multi-block compression
        let d = sum_md5(&vec![0u8; 256]);
        assert_eq!(d.len(), 16);
    }

    #[test]
    fn all_byte_values_0_to_255() {
        // All 256 possible byte values in sequence — comprehensive round coverage
        let data: Vec<u8> = (0u8..=255).collect();
        let d = sum_md5(&data);
        assert_eq!(d.len(), 16);
        // Known MD5 for bytes 0x00..0xFF in sequence
        let hex: String = d.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "e2c865db4162bed963bfaa9ef6ac18f0");
    }

    #[test]
    fn every_single_byte_unique() {
        // Each single byte 0x00..0xFF must produce a distinct 16-byte digest.
        // A collision among single-byte inputs would be catastrophic.
        let digests: std::collections::HashSet<[u8; 16]> =
            (0u8..=255).map(|i| sum_md5(&[i])).collect();
        assert_eq!(digests.len(), 256, "collision found among single-byte inputs!");
    }

    #[test]
    fn large_input() {
        // 1 MB of data — tests many block compressions in a loop
        let data = vec![0x42u8; 1_000_000];
        let d = sum_md5(&data);
        assert_eq!(d.len(), 16);
    }

    // ─── Streaming API ────────────────────────────────────────────────────────
    //
    // The streaming Digest must produce the exact same output as the one-shot
    // sum_md5(), regardless of how the input is split across update() calls.

    #[test]
    fn streaming_single_write() {
        let mut h = Digest::new();
        h.update(b"abc");
        assert_eq!(h.sum_md5(), sum_md5(b"abc"));
    }

    #[test]
    fn streaming_two_writes() {
        let mut h = Digest::new();
        h.update(b"ab");
        h.update(b"c");
        assert_eq!(h.sum_md5(), sum_md5(b"abc"));
    }

    #[test]
    fn streaming_split_across_block_boundary() {
        // Split a 128-byte message exactly at the 64-byte block boundary
        let data = vec![0u8; 128];
        let mut h = Digest::new();
        h.update(&data[..64]);
        h.update(&data[64..]);
        assert_eq!(h.sum_md5(), sum_md5(&data));
    }

    #[test]
    fn streaming_byte_at_a_time() {
        // Worst case for streaming: one byte per update() call
        let data: Vec<u8> = (0u8..100).collect();
        let mut h = Digest::new();
        for &byte in &data {
            h.update(&[byte]);
        }
        assert_eq!(h.sum_md5(), sum_md5(&data));
    }

    #[test]
    fn streaming_empty() {
        // Streaming with zero update() calls must equal sum_md5(b"")
        let h = Digest::new();
        assert_eq!(h.sum_md5(), sum_md5(b""));
    }

    #[test]
    fn streaming_nondestructive() {
        // Calling sum_md5() twice on the same Digest must return the same result
        let mut h = Digest::new();
        h.update(b"hello world");
        assert_eq!(h.sum_md5(), h.sum_md5());
    }

    #[test]
    fn streaming_can_continue_after_sum() {
        // After calling sum_md5(), we can add more data and get a fresh digest
        let mut h = Digest::new();
        h.update(b"hello");
        let d1 = h.sum_md5();
        h.update(b" world");
        let d2 = h.sum_md5();
        assert_eq!(d1, sum_md5(b"hello"));
        assert_eq!(d2, sum_md5(b"hello world"));
        assert_ne!(d1, d2);
    }

    #[test]
    fn streaming_hex_digest() {
        let mut h = Digest::new();
        h.update(b"abc");
        assert_eq!(h.hex_digest(), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn streaming_clone_is_independent() {
        // Clone diverges: updates to the clone do not affect the original and vice versa
        let mut h = Digest::new();
        h.update(b"ab");
        let mut h2 = h.clone_digest();
        h2.update(b"c");   // h2 → "abc"
        h.update(b"x");    // h  → "abx"
        assert_eq!(h2.sum_md5(), sum_md5(b"abc"));
        assert_eq!(h.sum_md5(), sum_md5(b"abx"));
    }

    #[test]
    fn streaming_rfc_vectors() {
        // All RFC 1321 vectors must work through the streaming API too
        let cases: &[(&[u8], &str)] = &[
            (b"", "d41d8cd98f00b204e9800998ecf8427e"),
            (b"a", "0cc175b9c0f1b6a831c399e269772661"),
            (b"abc", "900150983cd24fb0d6963f7d28e17f72"),
            (b"message digest", "f96b697d7cb7938d525a2f31aaf161d0"),
        ];
        for (input, expected) in cases {
            let mut h = Digest::new();
            h.update(input);
            assert_eq!(
                h.hex_digest(),
                *expected,
                "streaming failed for input {:?}",
                String::from_utf8_lossy(input)
            );
        }
    }

    #[test]
    fn streaming_large_chunked() {
        // 1 MB split into two halves — tests multi-block streaming
        let data = vec![b'a'; 1_000_000];
        let mut h = Digest::new();
        h.update(&data[..500_000]);
        h.update(&data[500_000..]);
        assert_eq!(h.sum_md5(), sum_md5(&data));
    }

    // ─── sum_md5 / hex_string API parity ─────────────────────────────────────

    #[test]
    fn hex_string_matches_sum_md5() {
        // hex_string() must be exactly sum_md5() formatted as hex
        let inputs: &[&[u8]] = &[b"", b"abc", b"hello world", &[0xff, 0x00, 0x80]];
        for input in inputs {
            let from_bytes: String = sum_md5(input)
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect();
            assert_eq!(
                from_bytes,
                hex_string(input),
                "mismatch for {:?}",
                input
            );
        }
    }
}
