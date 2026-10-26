//! Fixed-memory SHA-384 from FIPS 180-4.
//!
//! SHA-384 uses the SHA-512-family 1024-bit block and 80-round compression
//! function with a distinct initial state, then returns the leftmost 384 bits
//! of the final state. The core API allocates no heap memory.
//!
//! ```
//! use coding_adventures_sha384::{sha384, Sha384};
//!
//! let expected = sha384(b"abc");
//! let mut digest = Sha384::new();
//! digest.update(b"a")?;
//! digest.update(b"bc")?;
//! assert_eq!(digest.digest(), expected);
//! # Ok::<(), coding_adventures_sha384::Sha384Error>(())
//! ```

#![forbid(unsafe_code)]

use coding_adventures_zeroize::Zeroize;
use core::fmt;

const BLOCK_LENGTH: usize = 128;
const LENGTH_FIELD_LENGTH: usize = 16;
const SINGLE_BLOCK_MESSAGE_LIMIT: usize = BLOCK_LENGTH - LENGTH_FIELD_LENGTH - 1;
const MAX_MESSAGE_BYTES: u128 = u128::MAX / 8;

// FIPS 180-4 section 5.3.4.
const INITIAL_STATE: [u64; 8] = [
    0xcbbb9d5dc1059ed8,
    0x629a292a367cd507,
    0x9159015a3070dd17,
    0x152fecd8f70e5939,
    0x67332667ffc00b31,
    0x8eb44a8768581511,
    0xdb0c2e0d64f98fa7,
    0x47b5481dbefa4fa4,
];

// FIPS 180-4 section 4.2.3. SHA-384 and SHA-512 share these constants.
const ROUND_CONSTANTS: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

/// Stable, payload-free failures from streaming SHA-384.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Sha384Error {
    /// The message's bit length would not fit the FIPS 180-4 128-bit field.
    MessageTooLong,
}

impl fmt::Display for Sha384Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SHA-384 message is too long")
    }
}

impl std::error::Error for Sha384Error {}

/// Compute one SHA-384 digest without heap allocation.
#[must_use]
pub fn sha384(data: &[u8]) -> [u8; 48] {
    let mut hasher = Sha384::new();
    hasher
        .update(data)
        .expect("one address-space-sized slice fits SHA-384's length field");
    hasher.digest()
}

/// Incremental fixed-memory SHA-384 state.
///
/// The state owns one partial block. It allocates no heap memory and scrubs its
/// chaining state and partial input on drop.
pub struct Sha384 {
    state: [u64; 8],
    buffer: [u8; BLOCK_LENGTH],
    buffer_len: usize,
    byte_count: u128,
}

impl Sha384 {
    /// Construct an empty SHA-384 state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: INITIAL_STATE,
            buffer: [0; BLOCK_LENGTH],
            buffer_len: 0,
            byte_count: 0,
        }
    }

    /// Absorb more message bytes.
    ///
    /// The total encoded length is checked before any retained state changes.
    ///
    /// # Errors
    ///
    /// Returns [`Sha384Error::MessageTooLong`] if the resulting bit length
    /// would not fit the standard's 128-bit length field.
    pub fn update(&mut self, data: &[u8]) -> Result<(), Sha384Error> {
        let data_len = data.len() as u128;
        let next_count = self
            .byte_count
            .checked_add(data_len)
            .filter(|count| *count <= MAX_MESSAGE_BYTES)
            .ok_or(Sha384Error::MessageTooLong)?;

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
                .expect("split_at returns one complete SHA-384 block");
            compress(&mut self.state, block);
            remaining = tail;
        }

        self.buffer[..remaining.len()].copy_from_slice(remaining);
        self.buffer_len = remaining.len();
        self.byte_count = next_count;
        Ok(())
    }

    /// Return a non-destructive 48-byte digest snapshot.
    #[must_use]
    pub fn digest(&self) -> [u8; 48] {
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

        let mut digest = [0u8; 48];
        for (index, word) in state[..6].iter().enumerate() {
            digest[index * 8..index * 8 + 8].copy_from_slice(&word.to_be_bytes());
        }

        scrub_state(&mut state);
        tail.zeroize();
        digest
    }
}

impl Default for Sha384 {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Sha384 {
    fn drop(&mut self) {
        scrub_state(&mut self.state);
        self.buffer.zeroize();
        self.buffer_len.zeroize();
        self.byte_count.zeroize();
    }
}

#[inline]
fn choice(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (!x & z)
}

#[inline]
fn majority(x: u64, y: u64, z: u64) -> u64 {
    (x & y) ^ (x & z) ^ (y & z)
}

#[inline]
fn big_sigma_0(value: u64) -> u64 {
    value.rotate_right(28) ^ value.rotate_right(34) ^ value.rotate_right(39)
}

#[inline]
fn big_sigma_1(value: u64) -> u64 {
    value.rotate_right(14) ^ value.rotate_right(18) ^ value.rotate_right(41)
}

#[inline]
fn small_sigma_0(value: u64) -> u64 {
    value.rotate_right(1) ^ value.rotate_right(8) ^ (value >> 7)
}

#[inline]
fn small_sigma_1(value: u64) -> u64 {
    value.rotate_right(19) ^ value.rotate_right(61) ^ (value >> 6)
}

fn compress(state: &mut [u64; 8], block: &[u8; BLOCK_LENGTH]) {
    let mut schedule = [0u64; 80];
    for (index, word) in schedule[..16].iter_mut().enumerate() {
        let start = index * 8;
        *word = u64::from_be_bytes(
            block[start..start + 8]
                .try_into()
                .expect("a SHA-384 block contains sixteen complete words"),
        );
    }
    for index in 16..80 {
        schedule[index] = small_sigma_1(schedule[index - 2])
            .wrapping_add(schedule[index - 7])
            .wrapping_add(small_sigma_0(schedule[index - 15]))
            .wrapping_add(schedule[index - 16]);
    }

    let mut a = state[0];
    let mut b = state[1];
    let mut c = state[2];
    let mut d = state[3];
    let mut e = state[4];
    let mut f = state[5];
    let mut g = state[6];
    let mut h = state[7];

    for index in 0..80 {
        let mut temporary_1 = h
            .wrapping_add(big_sigma_1(e))
            .wrapping_add(choice(e, f, g))
            .wrapping_add(ROUND_CONSTANTS[index])
            .wrapping_add(schedule[index]);
        let mut temporary_2 = big_sigma_0(a).wrapping_add(majority(a, b, c));
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(temporary_1);
        d = c;
        c = b;
        b = a;
        a = temporary_1.wrapping_add(temporary_2);
        temporary_1.zeroize();
        temporary_2.zeroize();
    }

    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
    state[5] = state[5].wrapping_add(f);
    state[6] = state[6].wrapping_add(g);
    state[7] = state[7].wrapping_add(h);

    scrub_state(&mut schedule);
    a.zeroize();
    b.zeroize();
    c.zeroize();
    d.zeroize();
    e.zeroize();
    f.zeroize();
    g.zeroize();
    h.zeroize();
}

fn scrub_state<const N: usize>(state: &mut [u64; N]) {
    for word in state {
        word.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn fips_empty_message() {
        assert_eq!(
            hex(&sha384(b"")),
            "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da\
             274edebfe76f65fbd51ad2f14898b95b"
        );
    }

    #[test]
    fn fips_abc() {
        assert_eq!(
            hex(&sha384(b"abc")),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
             8086072ba1e7cc2358baeca134c825a7"
        );
    }

    #[test]
    fn fips_896_bit_message_crosses_padding_boundary() {
        let message = b"abcdefghbcdefghicdefghijdefghijkefghijklfghijklmghijklmnhijklmno\
                        ijklmnopjklmnopqklmnopqrlmnopqrsmnopqrstnopqrstu";
        assert_eq!(message.len(), 112);
        assert_eq!(
            hex(&sha384(message)),
            "09330c33f71147e83d192fc782cd1b4753111b173b3b05d22fa08086e3b0f712\
             fcc7c71a557e2db966c3e9fa91746039"
        );
    }

    #[test]
    fn fips_million_a_streams_without_allocating_the_message() {
        let chunk = [b'a'; 1_000];
        let mut hasher = Sha384::new();
        for _ in 0..1_000 {
            hasher.update(&chunk).expect("one million bytes fit");
        }
        assert_eq!(
            hex(&hasher.digest()),
            "9d0e1809716474cb086e834e310a4a1ced149e9c00f248527972cec5704c2a5b\
             07b8b3dc38ecc4ebae97ddd87f3d8985"
        );
    }

    #[test]
    fn every_relevant_partition_matches_one_shot() {
        let message: Vec<u8> = (0..=255).cycle().take(1_037).collect();
        let expected = sha384(&message);
        for chunk_size in [1, 2, 3, 7, 13, 64, 111, 112, 127, 128, 129, 255] {
            let mut hasher = Sha384::new();
            for chunk in message.chunks(chunk_size) {
                hasher.update(chunk).expect("test message fits");
            }
            assert_eq!(hasher.digest(), expected, "chunk size {chunk_size}");
        }
    }

    #[test]
    fn digest_is_non_destructive() {
        let mut hasher = Sha384::new();
        hasher.update(b"ab").expect("short message fits");
        let prefix = hasher.digest();
        assert_eq!(prefix, sha384(b"ab"));
        assert_eq!(hasher.digest(), prefix);
        hasher.update(b"c").expect("short message fits");
        assert_eq!(hasher.digest(), sha384(b"abc"));
    }

    #[test]
    fn empty_updates_do_not_change_the_digest() {
        let mut hasher = Sha384::new();
        hasher.update(b"abc").expect("short message fits");
        let before = hasher.digest();
        hasher.update(b"").expect("empty update fits");
        assert_eq!(hasher.digest(), before);
    }

    #[test]
    fn length_failure_is_payload_free_and_transactional() {
        let mut hasher = Sha384::new();
        hasher.byte_count = MAX_MESSAGE_BYTES;
        let state_before = hasher.state;
        let buffer_before = hasher.buffer;
        let buffer_len_before = hasher.buffer_len;

        assert_eq!(hasher.update(b"secret"), Err(Sha384Error::MessageTooLong));
        assert_eq!(hasher.state, state_before);
        assert_eq!(hasher.buffer, buffer_before);
        assert_eq!(hasher.buffer_len, buffer_len_before);
        assert_eq!(hasher.byte_count, MAX_MESSAGE_BYTES);
        assert_eq!(
            Sha384Error::MessageTooLong.to_string(),
            "SHA-384 message is too long"
        );
    }

    #[test]
    fn digest_has_exact_sha384_width() {
        assert_eq!(sha384(b"width").len(), 48);
    }
}
