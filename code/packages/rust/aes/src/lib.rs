//! # coding_adventures_aes — AES block cipher (FIPS 197)
//!
//! AES (Advanced Encryption Standard) is the most widely deployed symmetric
//! encryption algorithm in the world. Published by NIST in 2001 as FIPS 197,
//! it replaced DES and is used in TLS/HTTPS, WPA2/WPA3 WiFi, disk encryption
//! (BitLocker, LUKS, FileVault), VPNs, and virtually every secure protocol.
//!
//! Designed by Joan Daemen and Vincent Rijmen (Rijndael), AES is a
//! Substitution-Permutation Network (SPN) — a fundamentally different structure
//! from DES's Feistel network. All bytes of the state are transformed on every
//! round, not just half.
//!
//! ## Architecture
//!
//! ```text
//! plaintext (16 bytes)
//!      │
//! AddRoundKey(state, round_key[0])       ← XOR with first key material
//!      │
//! ┌── Nr-1 full rounds ──────────────────────────────────────────────┐
//! │   SubBytes   — non-linear S-box substitution (GF(2^8) inverse)   │
//! │   ShiftRows  — cyclic row shifts (diffusion across columns)       │
//! │   MixColumns — GF(2^8) matrix multiply (diffusion across rows)   │
//! │   AddRoundKey — XOR with round key                               │
//! └───────────────────────────────────────────────────────────────────┘
//!      │
//! SubBytes + ShiftRows + AddRoundKey     ← final round (no MixColumns)
//!      │
//! ciphertext (16 bytes)
//! ```
//!
//! The state is a 4×4 matrix of bytes, indexed state[row][col].
//!
//! ## GF(2^8) Connection
//!
//! AES arithmetic lives in GF(2^8) with irreducible polynomial:
//!   p(x) = x^8 + x^4 + x^3 + x + 1  =  0x11B
//!
//! The S-box maps each byte to its multiplicative inverse in GF(2^8), followed
//! by an affine transformation over GF(2). This is the only non-linear step.
//!
//! ## Key Sizes and Round Counts
//!
//! ```text
//! Key size   Nk (words)   Nr (rounds)   Round keys
//! 128 bits      4             10          11 × 16 bytes
//! 192 bits      6             12          13 × 16 bytes
//! 256 bits      8             14          15 × 16 bytes
//! ```
//!
//! ## Example
//!
//! ```
//! use coding_adventures_aes::{encrypt_block, decrypt_block};
//! let key = [
//!     0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
//!     0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
//! ];
//! let plain = [
//!     0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d,
//!     0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37, 0x07, 0x34,
//! ];
//! let ct = encrypt_block(&plain, &key).unwrap();
//! assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);
//! ```

use std::sync::OnceLock;
use gf256::Field;

// ─────────────────────────────────────────────────────────────────────────────
// AES GF(2^8) field — polynomial 0x11B = x^8 + x^4 + x^3 + x + 1
//
// This is distinct from the Reed-Solomon polynomial 0x11D used by gf256's
// top-level functions. We use the parameterized `Field` struct.
// ─────────────────────────────────────────────────────────────────────────────

/// Lazily-initialized (SBOX, INV_SBOX) pair.
///
/// `OnceLock` provides thread-safe one-time initialization. Both tables are
/// computed together since INV_SBOX is just the inverse of SBOX.
static SBOXES: OnceLock<([u8; 256], [u8; 256])> = OnceLock::new();

/// Get a reference to the global SBOX pair, initializing if needed.
fn get_sboxes() -> &'static ([u8; 256], [u8; 256]) {
    SBOXES.get_or_init(build_sboxes)
}

/// The AES S-box (256 bytes). Publicly accessible after lazy init.
///
/// Access via the `sbox()` helper, or read `SBOX` static after calling any
/// encrypt/decrypt function (which triggers initialization).
///
/// SBOX[0x00] == 0x63, SBOX[0x01] == 0x7c, etc. (FIPS 197 Figure 7)
pub fn sbox() -> &'static [u8; 256] {
    &get_sboxes().0
}

/// The AES inverse S-box (256 bytes).
///
/// INV_SBOX[SBOX[b]] == b for all b in 0..256.
pub fn inv_sbox() -> &'static [u8; 256] {
    &get_sboxes().1
}

// ─────────────────────────────────────────────────────────────────────────────
// S-box generation
//
// SubBytes maps each byte b to:
//   1. inv = b^{-1} in GF(2^8)  (0 maps to 0)
//   2. affine transformation: each bit s_i = b_i XOR b_{(i+4)%8} XOR ... XOR c_i
//      where c = 0x63 = 0110_0011
//
// This two-step design makes the S-box resistant to linear and differential
// cryptanalysis: the GF inverse ensures non-linearity; the affine transform
// eliminates fixed points (no byte maps to itself).
// ─────────────────────────────────────────────────────────────────────────────

/// AES affine transformation over GF(2).
///
/// For each bit position i (0 = LSB, 7 = MSB):
///   s_i = b_i XOR b_{(i+4)%8} XOR b_{(i+5)%8} XOR b_{(i+6)%8} XOR b_{(i+7)%8} XOR c_i
/// where c = 0x63.
///
/// Implemented via byte rotation and XOR — equivalent to the circulant matrix
/// form in FIPS 197 Section 5.1.1.
fn affine_transform(b: u8) -> u8 {
    // Circular left rotations of b by 1,2,3,4 positions
    let rot = |x: u8, n: u32| x.rotate_left(n);
    b ^ rot(b, 1) ^ rot(b, 2) ^ rot(b, 3) ^ rot(b, 4) ^ 0x63
}

/// Build the AES S-box and inverse S-box.
///
/// For each byte b (0..255):
///   1. Compute the multiplicative inverse in GF(2^8) with polynomial 0x11B
///      (0 has no inverse; it maps to 0 by convention).
///   2. Apply the AES affine transformation.
///
/// The inverse S-box is: INV_SBOX[SBOX[b]] = b.
fn build_sboxes() -> ([u8; 256], [u8; 256]) {
    let field = Field::new(0x11B);
    let mut sbox = [0u8; 256];
    for b in 0u8..=255 {
        let inv = if b == 0 { 0 } else { field.inverse(b) };
        sbox[b as usize] = affine_transform(inv);
    }
    let mut inv_sbox = [0u8; 256];
    for b in 0u8..=255 {
        inv_sbox[sbox[b as usize] as usize] = b;
    }
    (sbox, inv_sbox)
}

// ─────────────────────────────────────────────────────────────────────────────
// Round constants (Rcon) for the key schedule
//
// Rcon[i] = 2^{i-1} in GF(2^8) for i = 1..10 (AES-128 needs 10, AES-256 needs 7).
// These are the first byte of a 4-byte word [Rcon_i, 0, 0, 0].
// They break symmetry in the key schedule so that no two round keys are equal.
// ─────────────────────────────────────────────────────────────────────────────

/// Precomputed Rcon values for the key schedule.
/// Index 0 is unused (NIST is 1-indexed); indices 1..14 cover AES-256.
///
/// Rcon[i] = x^{i-1} in GF(2^8) with polynomial 0x11B.
const RCON: [u8; 15] = [
    0x00, // index 0 unused
    0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36,
    0x6C, 0xD8, 0xAB, 0x4D,
];

// ─────────────────────────────────────────────────────────────────────────────
// Key schedule: expand_key
// ─────────────────────────────────────────────────────────────────────────────

/// Expand a 16-, 24-, or 32-byte AES key into round keys.
///
/// Returns a `Vec` of (Nr+1) round keys, each a `[[u8; 4]; 4]` state matrix.
/// The round key at index 0 is used in the initial AddRoundKey;
/// round key Nr is used in the final AddRoundKey.
///
/// ## Key Schedule Algorithm (FIPS 197 Section 5.2)
///
/// - Nk = key length in 32-bit words (4, 6, or 8)
/// - Nr = number of rounds (10, 12, or 14)
/// - Total words needed = 4 × (Nr + 1)
/// - W[i] = W[i-1] XOR W[i-Nk]  for i not a multiple of Nk
/// - W[i] = SubWord(RotWord(W[i-1])) XOR Rcon[i/Nk] XOR W[i-Nk]  when i mod Nk == 0
/// - W[i] = SubWord(W[i-1]) XOR W[i-Nk]  when Nk=8 and i mod Nk == 4
///
/// ## Errors
///
/// Returns `Err` if `key.len()` is not 16, 24, or 32.
pub fn expand_key(key: &[u8]) -> Result<Vec<[[u8; 4]; 4]>, String> {
    let key_len = key.len();
    if key_len != 16 && key_len != 24 && key_len != 32 {
        return Err(format!(
            "AES key must be 16, 24, or 32 bytes; got {}",
            key_len
        ));
    }

    let sb = &get_sboxes().0;

    let nk = key_len / 4;
    let nr = match nk {
        4 => 10,
        6 => 12,
        8 => 14,
        _ => unreachable!(),
    };
    let total_words = 4 * (nr + 1);

    // W is a flat list of 4-byte words
    let mut w: Vec<[u8; 4]> = Vec::with_capacity(total_words);
    for i in 0..nk {
        let mut word = [0u8; 4];
        word.copy_from_slice(&key[4 * i..4 * i + 4]);
        w.push(word);
    }

    for i in nk..total_words {
        let mut temp = w[i - 1];
        if i % nk == 0 {
            // RotWord: left-rotate the 4 bytes
            temp = [temp[1], temp[2], temp[3], temp[0]];
            // SubWord: apply S-box to each byte
            temp = [sb[temp[0] as usize], sb[temp[1] as usize], sb[temp[2] as usize], sb[temp[3] as usize]];
            // XOR with round constant
            temp[0] ^= RCON[i / nk];
        } else if nk == 8 && i % nk == 4 {
            // Extra SubWord for AES-256
            temp = [sb[temp[0] as usize], sb[temp[1] as usize], sb[temp[2] as usize], sb[temp[3] as usize]];
        }
        let prev = w[i - nk];
        w.push([prev[0] ^ temp[0], prev[1] ^ temp[1], prev[2] ^ temp[2], prev[3] ^ temp[3]]);
    }

    // Pack into (Nr+1) round keys, each a 4×4 state (column-major layout).
    // state[row][col] = w[4*rk + col][row]
    let mut round_keys = Vec::with_capacity(nr + 1);
    for rk in 0..=nr {
        let mut state = [[0u8; 4]; 4];
        for col in 0..4 {
            let word = w[4 * rk + col];
            for row in 0..4 {
                state[row][col] = word[row];
            }
        }
        round_keys.push(state);
    }
    Ok(round_keys)
}

// ─────────────────────────────────────────────────────────────────────────────
// State manipulation helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Convert 16 bytes to AES state (4×4 column-major matrix).
///
/// AES loads bytes column by column:
/// ```text
/// state[row][col] = block[row + 4*col]
///
/// block[0]  block[4]  block[8]  block[12]
/// block[1]  block[5]  block[9]  block[13]
/// block[2]  block[6]  block[10] block[14]
/// block[3]  block[7]  block[11] block[15]
/// ```
fn bytes_to_state(block: &[u8; 16]) -> [[u8; 4]; 4] {
    let mut state = [[0u8; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            state[row][col] = block[row + 4 * col];
        }
    }
    state
}

/// Convert AES state back to 16 bytes (column-major).
fn state_to_bytes(state: &[[u8; 4]; 4]) -> [u8; 16] {
    let mut out = [0u8; 16];
    for col in 0..4 {
        for row in 0..4 {
            out[row + 4 * col] = state[row][col];
        }
    }
    out
}

/// XOR state with round key (AddRoundKey step).
fn add_round_key(state: &[[u8; 4]; 4], round_key: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let mut out = [[0u8; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] = state[r][c] ^ round_key[r][c];
        }
    }
    out
}

/// Replace each byte with its S-box value (SubBytes step).
fn sub_bytes(state: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let sb = &get_sboxes().0;
    let mut out = [[0u8; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] = sb[state[r][c] as usize];
        }
    }
    out
}

/// Inverse SubBytes — apply inverse S-box.
fn inv_sub_bytes(state: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let isb = &get_sboxes().1;
    let mut out = [[0u8; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] = isb[state[r][c] as usize];
        }
    }
    out
}

/// Cyclically shift row i left by i positions (ShiftRows step).
///
/// Row 0: no shift
/// Row 1: shift left 1
/// Row 2: shift left 2
/// Row 3: shift left 3
///
/// This ensures that after MixColumns, each output column is a function of
/// all four input columns — providing diffusion across the full state.
fn shift_rows(state: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let mut out = [[0u8; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            out[r][c] = state[r][(c + r) % 4];
        }
    }
    out
}

/// Inverse ShiftRows — shift row i right by i positions.
fn inv_shift_rows(state: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let mut out = [[0u8; 4]; 4];
    for r in 0..4 {
        for c in 0..4 {
            // shift right by r = take from (c + 4 - r) % 4
            out[r][c] = state[r][(c + 4 - r) % 4];
        }
    }
    out
}

/// Multiply b by x (= 2) in GF(2^8) with AES polynomial 0x11B.
/// Equivalent to left-shift by 1, XOR 0x1B if bit 7 was set.
#[inline]
fn xtime(b: u8) -> u8 {
    let hi = b & 0x80;
    let shifted = b << 1;
    if hi != 0 { shifted ^ 0x1B } else { shifted }
}

/// Apply MixColumns to one 4-byte column.
///
/// Multiplies by the AES MixColumns matrix over GF(2^8):
/// ```text
/// [2 3 1 1]   [s0]
/// [1 2 3 1] × [s1]
/// [1 1 2 3]   [s2]
/// [3 1 1 2]   [s3]
/// ```
/// where 2·x = xtime(x) and 3·x = xtime(x) XOR x.
fn mix_col(col: [u8; 4]) -> [u8; 4] {
    let [s0, s1, s2, s3] = col;
    [
        xtime(s0) ^ (xtime(s1) ^ s1) ^ s2 ^ s3,
        s0 ^ xtime(s1) ^ (xtime(s2) ^ s2) ^ s3,
        s0 ^ s1 ^ xtime(s2) ^ (xtime(s3) ^ s3),
        (xtime(s0) ^ s0) ^ s1 ^ s2 ^ xtime(s3),
    ]
}

/// Apply InvMixColumns to one 4-byte column.
///
/// Multiplies by the AES InvMixColumns matrix:
/// ```text
/// [14  11  13   9]
/// [ 9  14  11  13]
/// [13   9  14  11]
/// [11  13   9  14]
/// ```
fn inv_mix_col(col: [u8; 4]) -> [u8; 4] {
    let [s0, s1, s2, s3] = col;
    // Use Russian-peasant (xtime chain) to compute multiplications
    let mul = |a: u8, b: u8| -> u8 {
        // Russian peasant multiplication in GF(2^8) AES field
        let mut result = 0u8;
        let mut aa = a;
        let mut bb = b;
        for _ in 0..8 {
            if bb & 1 != 0 {
                result ^= aa;
            }
            let hi = aa & 0x80;
            aa <<= 1;
            if hi != 0 {
                aa ^= 0x1B;
            }
            bb >>= 1;
        }
        result
    };
    [
        mul(0x0e, s0) ^ mul(0x0b, s1) ^ mul(0x0d, s2) ^ mul(0x09, s3),
        mul(0x09, s0) ^ mul(0x0e, s1) ^ mul(0x0b, s2) ^ mul(0x0d, s3),
        mul(0x0d, s0) ^ mul(0x09, s1) ^ mul(0x0e, s2) ^ mul(0x0b, s3),
        mul(0x0b, s0) ^ mul(0x0d, s1) ^ mul(0x09, s2) ^ mul(0x0e, s3),
    ]
}

/// Apply MixColumns to all 4 columns of the state.
fn mix_columns(state: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let mut out = [[0u8; 4]; 4];
    for col in 0..4 {
        let column = [state[0][col], state[1][col], state[2][col], state[3][col]];
        let mixed = mix_col(column);
        for row in 0..4 {
            out[row][col] = mixed[row];
        }
    }
    out
}

/// Apply InvMixColumns to all 4 columns of the state.
fn inv_mix_columns(state: &[[u8; 4]; 4]) -> [[u8; 4]; 4] {
    let mut out = [[0u8; 4]; 4];
    for col in 0..4 {
        let column = [state[0][col], state[1][col], state[2][col], state[3][col]];
        let mixed = inv_mix_col(column);
        for row in 0..4 {
            out[row][col] = mixed[row];
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Core block cipher
// ─────────────────────────────────────────────────────────────────────────────

/// Encrypt a single 128-bit (16-byte) block with AES.
///
/// Supports all three key sizes:
///   - 16 bytes (AES-128): 10 rounds
///   - 24 bytes (AES-192): 12 rounds
///   - 32 bytes (AES-256): 14 rounds
///
/// ## Algorithm (FIPS 197 Section 5.1)
///
/// ```text
/// AddRoundKey(state, round_key[0])
/// for round = 1 to Nr-1:
///   SubBytes → ShiftRows → MixColumns → AddRoundKey
/// SubBytes → ShiftRows → AddRoundKey  (final round: no MixColumns)
/// ```
///
/// ## Errors
///
/// Returns `Err` if `block.len() != 16` or key length is not 16, 24, or 32.
///
/// ## Example
///
/// ```
/// use coding_adventures_aes::encrypt_block;
/// let key: Vec<u8> = (0..16).collect();
/// let plain = [0u8; 16];
/// let ct = encrypt_block(&plain, &key).unwrap();
/// assert_eq!(ct.len(), 16);
/// ```
pub fn encrypt_block(block: &[u8; 16], key: &[u8]) -> Result<[u8; 16], String> {
    let round_keys = expand_key(key)?;
    let nr = round_keys.len() - 1;

    let mut state = bytes_to_state(block);
    state = add_round_key(&state, &round_keys[0]);

    for rnd in 1..nr {
        state = sub_bytes(&state);
        state = shift_rows(&state);
        state = mix_columns(&state);
        state = add_round_key(&state, &round_keys[rnd]);
    }

    // Final round: no MixColumns
    state = sub_bytes(&state);
    state = shift_rows(&state);
    state = add_round_key(&state, &round_keys[nr]);

    Ok(state_to_bytes(&state))
}

/// Decrypt a single 128-bit (16-byte) block with AES.
///
/// Unlike DES (Feistel), AES decryption is NOT the same circuit as encryption.
/// It uses the inverse of each operation, applied in reverse:
/// InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns.
///
/// (Note: AddRoundKey is its own inverse since XOR is self-inverse.)
///
/// ## Errors
///
/// Returns `Err` if `block.len() != 16` or key length is not 16, 24, or 32.
pub fn decrypt_block(block: &[u8; 16], key: &[u8]) -> Result<[u8; 16], String> {
    let round_keys = expand_key(key)?;
    let nr = round_keys.len() - 1;

    let mut state = bytes_to_state(block);
    state = add_round_key(&state, &round_keys[nr]);

    for rnd in (1..nr).rev() {
        state = inv_shift_rows(&state);
        state = inv_sub_bytes(&state);
        state = add_round_key(&state, &round_keys[rnd]);
        state = inv_mix_columns(&state);
    }

    // Final round
    state = inv_shift_rows(&state);
    state = inv_sub_bytes(&state);
    state = add_round_key(&state, &round_keys[0]);

    Ok(state_to_bytes(&state))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn h(s: &str) -> Vec<u8> {
        let s = s.replace(' ', "");
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn h16(s: &str) -> [u8; 16] {
        let v = h(s);
        let mut arr = [0u8; 16];
        arr.copy_from_slice(&v);
        arr
    }

    // ─── FIPS 197 Known-Answer Tests ────────────────────────────────────────

    #[test]
    fn aes128_fips_appendix_b_encrypt() {
        // FIPS 197 Appendix B
        let key = h("2b7e151628aed2a6abf7158809cf4f3c");
        let plain = h16("3243f6a8885a308d313198a2e0370734");
        assert_eq!(
            encrypt_block(&plain, &key).unwrap(),
            h16("3925841d02dc09fbdc118597196a0b32")
        );
    }

    #[test]
    fn aes128_fips_appendix_b_decrypt() {
        let key = h("2b7e151628aed2a6abf7158809cf4f3c");
        let ct = h16("3925841d02dc09fbdc118597196a0b32");
        assert_eq!(
            decrypt_block(&ct, &key).unwrap(),
            h16("3243f6a8885a308d313198a2e0370734")
        );
    }

    #[test]
    fn aes128_appendix_c1() {
        // FIPS 197 Appendix C.1
        let key = h("000102030405060708090a0b0c0d0e0f");
        let plain = h16("00112233445566778899aabbccddeeff");
        assert_eq!(
            encrypt_block(&plain, &key).unwrap(),
            h16("69c4e0d86a7b0430d8cdb78070b4c55a")
        );
    }

    #[test]
    fn aes192_fips_appendix_c2_encrypt() {
        // FIPS 197 Appendix C.2
        let key = h("000102030405060708090a0b0c0d0e0f1011121314151617");
        let plain = h16("00112233445566778899aabbccddeeff");
        assert_eq!(
            encrypt_block(&plain, &key).unwrap(),
            h16("dda97ca4864cdfe06eaf70a0ec0d7191")
        );
    }

    #[test]
    fn aes192_fips_appendix_c2_decrypt() {
        let key = h("000102030405060708090a0b0c0d0e0f1011121314151617");
        let ct = h16("dda97ca4864cdfe06eaf70a0ec0d7191");
        assert_eq!(
            decrypt_block(&ct, &key).unwrap(),
            h16("00112233445566778899aabbccddeeff")
        );
    }

    #[test]
    fn aes256_fips_vector_encrypt() {
        // From spec / SE01
        let key = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
        let plain = h16("6bc1bee22e409f96e93d7e117393172a");
        assert_eq!(
            encrypt_block(&plain, &key).unwrap(),
            h16("f3eed1bdb5d2a03c064b5a7e3db181f8")
        );
    }

    #[test]
    fn aes256_fips_vector_decrypt() {
        let key = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
        let ct = h16("f3eed1bdb5d2a03c064b5a7e3db181f8");
        assert_eq!(
            decrypt_block(&ct, &key).unwrap(),
            h16("6bc1bee22e409f96e93d7e117393172a")
        );
    }

    #[test]
    fn aes256_fips_appendix_c3() {
        // FIPS 197 Appendix C.3
        let key = h("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f");
        let plain = h16("00112233445566778899aabbccddeeff");
        let ct = h16("8ea2b7ca516745bfeafc49904b496089");
        assert_eq!(encrypt_block(&plain, &key).unwrap(), ct);
        assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);
    }

    // ─── S-box properties ────────────────────────────────────────────────────

    #[test]
    fn sbox_length() {
        assert_eq!(sbox().len(), 256);
    }

    #[test]
    fn inv_sbox_length() {
        assert_eq!(inv_sbox().len(), 256);
    }

    #[test]
    fn sbox_is_bijection() {
        // S-box must be a permutation (all 256 outputs distinct).
        let mut seen = [false; 256];
        for &v in sbox().iter() {
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&s| s), "S-box is not a bijection");
    }

    #[test]
    fn inv_sbox_is_bijection() {
        let mut seen = [false; 256];
        for &v in inv_sbox().iter() {
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&s| s), "INV_SBOX is not a bijection");
    }

    #[test]
    fn sbox_inv_sbox_inverse() {
        // INV_SBOX[SBOX[b]] == b for all b
        let sb = sbox();
        let isb = inv_sbox();
        for b in 0u8..=255 {
            assert_eq!(isb[sb[b as usize] as usize], b, "failed at b={}", b);
        }
    }

    #[test]
    fn sbox_known_values_fips197_figure7() {
        // Spot-check against FIPS 197 Figure 7
        assert_eq!(sbox()[0x00], 0x63);
        assert_eq!(sbox()[0x01], 0x7c);
        assert_eq!(sbox()[0xff], 0x16);
        assert_eq!(sbox()[0x53], 0xed);
    }

    #[test]
    fn inv_sbox_known_values() {
        assert_eq!(inv_sbox()[0x63], 0x00);
        assert_eq!(inv_sbox()[0x7c], 0x01);
    }

    #[test]
    fn sbox_no_fixed_points() {
        // No byte maps to itself (the affine constant 0x63 prevents this).
        for b in 0u8..=255 {
            assert_ne!(sbox()[b as usize], b, "Fixed point at {:#04x}", b);
        }
    }

    // ─── Key schedule tests ──────────────────────────────────────────────────

    #[test]
    fn expand_key_aes128_round_count() {
        let key: Vec<u8> = (0..16).collect();
        assert_eq!(expand_key(&key).unwrap().len(), 11); // Nr+1 = 11
    }

    #[test]
    fn expand_key_aes192_round_count() {
        let key: Vec<u8> = (0..24).collect();
        assert_eq!(expand_key(&key).unwrap().len(), 13); // Nr+1 = 13
    }

    #[test]
    fn expand_key_aes256_round_count() {
        let key: Vec<u8> = (0..32).collect();
        assert_eq!(expand_key(&key).unwrap().len(), 15); // Nr+1 = 15
    }

    #[test]
    fn expand_key_round_key_shape() {
        for key_len in [16usize, 24, 32] {
            let key: Vec<u8> = (0..key_len as u8).collect();
            let rks = expand_key(&key).unwrap();
            for rk in &rks {
                assert_eq!(rk.len(), 4);
                for row in rk {
                    assert_eq!(row.len(), 4);
                }
            }
        }
    }

    #[test]
    fn expand_key_first_round_key_equals_key() {
        // The first round key must equal the key bytes (column-major).
        let key = h("2b7e151628aed2a6abf7158809cf4f3c");
        let rks = expand_key(&key).unwrap();
        let rk0 = rks[0];
        let reconstructed: Vec<u8> = (0..4).flat_map(|col| (0..4).map(move |row| rk0[row][col])).collect();
        assert_eq!(reconstructed, key);
    }

    #[test]
    fn expand_key_different_keys_differ() {
        let rks1 = expand_key(&(0..16u8).collect::<Vec<_>>()).unwrap();
        let rks2 = expand_key(&(1..17u8).collect::<Vec<_>>()).unwrap();
        assert_ne!(rks1[0], rks2[0]);
    }

    #[test]
    fn expand_key_invalid_length() {
        assert!(expand_key(&(0..15u8).collect::<Vec<_>>()).is_err());
        assert!(expand_key(&(0..17u8).collect::<Vec<_>>()).is_err());
        assert!(expand_key(&(0..20u8).collect::<Vec<_>>()).is_err());
    }

    // ─── Round-trip tests ────────────────────────────────────────────────────

    #[test]
    fn roundtrip_aes128_fips_vector() {
        let key = h("2b7e151628aed2a6abf7158809cf4f3c");
        let plain = h16("3243f6a8885a308d313198a2e0370734");
        let ct = encrypt_block(&plain, &key).unwrap();
        assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);
    }

    #[test]
    fn roundtrip_aes192() {
        let key = h("000102030405060708090a0b0c0d0e0f1011121314151617");
        for start in (0u8..=240).step_by(16) {
            let plain: [u8; 16] = core::array::from_fn(|i| start.wrapping_add(i as u8));
            let ct = encrypt_block(&plain, &key).unwrap();
            assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);
        }
    }

    #[test]
    fn roundtrip_aes256() {
        let key = h("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4");
        for start in (0u8..=240).step_by(16) {
            let plain: [u8; 16] = core::array::from_fn(|i| start.wrapping_add(i as u8));
            let ct = encrypt_block(&plain, &key).unwrap();
            assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);
        }
    }

    #[test]
    fn roundtrip_all_zeros() {
        for key_len in [16usize, 24, 32] {
            let key = vec![0u8; key_len];
            let plain = [0u8; 16];
            let ct = encrypt_block(&plain, &key).unwrap();
            assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);
        }
    }

    #[test]
    fn roundtrip_all_ff() {
        for key_len in [16usize, 24, 32] {
            let key = vec![0xFFu8; key_len];
            let plain = [0xFFu8; 16];
            let ct = encrypt_block(&plain, &key).unwrap();
            assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);
        }
    }

    #[test]
    fn roundtrip_identity_key_and_plain() {
        for key_len in [16usize, 24, 32] {
            let key: Vec<u8> = (0..key_len as u8).collect();
            let plain: [u8; 16] = core::array::from_fn(|i| i as u8);
            let ct = encrypt_block(&plain, &key).unwrap();
            assert_eq!(decrypt_block(&ct, &key).unwrap(), plain);
        }
    }

    #[test]
    fn avalanche_effect() {
        // Changing one plaintext bit should change many output bytes.
        let key: Vec<u8> = (0..16).collect();
        let plain1 = [0u8; 16];
        let mut plain2 = [0u8; 16];
        plain2[0] = 0x01;
        let ct1 = encrypt_block(&plain1, &key).unwrap();
        let ct2 = encrypt_block(&plain2, &key).unwrap();
        let diff_bits: u32 = ct1.iter().zip(ct2.iter()).map(|(&a, &b)| (a ^ b).count_ones()).sum();
        assert!(diff_bits > 32, "Only {} bits differ — poor diffusion", diff_bits);
    }

    // ─── Error handling ──────────────────────────────────────────────────────

    #[test]
    fn encrypt_wrong_key_length() {
        let plain = [0u8; 16];
        assert!(encrypt_block(&plain, &(0..10u8).collect::<Vec<_>>()).is_err());
        assert!(encrypt_block(&plain, &(0..20u8).collect::<Vec<_>>()).is_err());
    }

    #[test]
    fn decrypt_wrong_key_length() {
        let ct = [0u8; 16];
        assert!(decrypt_block(&ct, &(0..15u8).collect::<Vec<_>>()).is_err());
        assert!(decrypt_block(&ct, &(0..17u8).collect::<Vec<_>>()).is_err());
    }

    // ─── Additional edge cases ───────────────────────────────────────────────

    #[test]
    fn encrypt_is_deterministic() {
        let key: Vec<u8> = (0..16).collect();
        let plain = [0xABu8; 16];
        assert_eq!(
            encrypt_block(&plain, &key).unwrap(),
            encrypt_block(&plain, &key).unwrap()
        );
    }

    #[test]
    fn different_keys_different_ciphertext() {
        let plain = [0u8; 16];
        let key1: Vec<u8> = (0..16u8).collect();
        let key2: Vec<u8> = (1..17u8).collect();
        assert_ne!(
            encrypt_block(&plain, &key1).unwrap(),
            encrypt_block(&plain, &key2).unwrap()
        );
    }

    #[test]
    fn different_plaintexts_different_ciphertext() {
        let key: Vec<u8> = (0..16).collect();
        let ct1 = encrypt_block(&[0u8; 16], &key).unwrap();
        let ct2 = encrypt_block(&[1u8; 16], &key).unwrap();
        assert_ne!(ct1, ct2);
    }
}
