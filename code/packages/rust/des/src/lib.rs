//! # coding_adventures_des — DES and 3DES block cipher (FIPS 46-3 / SP 800-67)
//!
//! DES (Data Encryption Standard) was published by NIST in 1977 and was the
//! world's first openly standardized encryption algorithm. It is now completely
//! broken — a 56-bit key can be exhausted in under 24 hours on consumer hardware
//! — but it remains a vital historical and educational subject.
//!
//! ## Architecture
//!
//! ```text
//! plaintext (8 bytes)
//!      │
//! IP (initial permutation)       ← scatters bits for 1970s bus alignment
//!      │
//! ┌── 16 Feistel rounds ──────────────────────────────────────────────┐
//! │   L_i = R_{i-1}                                                   │
//! │   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                             │
//! │                                                                   │
//! │   f(R, K):                                                        │
//! │     E(R)          32→48 bits (expansion, border bits shared)      │
//! │     XOR K_i       48-bit subkey                                   │
//! │     S-boxes       8 × (6 bits → 4 bits) = 32 bits out            │
//! │     P             32→32 bit permutation                           │
//! └───────────────────────────────────────────────────────────────────┘
//!      │
//! FP (final permutation = IP⁻¹)
//!      │
//! ciphertext (8 bytes)
//! ```
//!
//! Decryption is identical — just apply the 16 subkeys in reverse order
//! (K16, K15, …, K1). The function f never needs to be inverted.
//!
//! ## Public API
//!
//! ```
//! use coding_adventures_des::{expand_key, encrypt_block, decrypt_block};
//! let key = [0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1];
//! let plain = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
//! let ct = encrypt_block(&plain, &key);
//! assert_eq!(ct, [0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05]);
//! assert_eq!(decrypt_block(&ct, &key), plain);
//! ```
//!
//! ## Why DES Uses 1-Indexed Tables
//!
//! The FIPS standard defines all permutation tables with 1-indexed positions
//! (e.g., "bit 58 moves to position 1"). In this Rust implementation, all tables
//! are stored as 0-indexed by subtracting 1 from each entry, so they can be used
//! directly as array indices.

// ─────────────────────────────────────────────────────────────────────────────
// Permutation and selection tables
// All tables store 0-indexed positions (original FIPS values minus 1).
// ─────────────────────────────────────────────────────────────────────────────

/// IP — Initial Permutation (0-indexed)
///
/// Input bit 58 (0-indexed: 57) becomes output bit 1 (index 0), etc.
/// This was designed for efficient loading on 8-bit parallel buses of the 1970s.
/// It has no cryptographic significance.
const IP: [u8; 64] = [
    57, 49, 41, 33, 25, 17,  9,  1,
    59, 51, 43, 35, 27, 19, 11,  3,
    61, 53, 45, 37, 29, 21, 13,  5,
    63, 55, 47, 39, 31, 23, 15,  7,
    56, 48, 40, 32, 24, 16,  8,  0,
    58, 50, 42, 34, 26, 18, 10,  2,
    60, 52, 44, 36, 28, 20, 12,  4,
    62, 54, 46, 38, 30, 22, 14,  6,
];

/// FP — Final Permutation / IP⁻¹ (0-indexed)
///
/// Undoes the initial permutation. FP[IP[i]] = i for all valid i.
const FP: [u8; 64] = [
    39,  7, 47, 15, 55, 23, 63, 31,
    38,  6, 46, 14, 54, 22, 62, 30,
    37,  5, 45, 13, 53, 21, 61, 29,
    36,  4, 44, 12, 52, 20, 60, 28,
    35,  3, 43, 11, 51, 19, 59, 27,
    34,  2, 42, 10, 50, 18, 58, 26,
    33,  1, 41,  9, 49, 17, 57, 25,
    32,  0, 40,  8, 48, 16, 56, 24,
];

/// PC-1 — Permuted Choice 1 (0-indexed)
///
/// Drops the 8 parity bits (positions 7,15,23,31,39,47,55,63 in 0-indexed)
/// and reorders the remaining 56 bits into two 28-bit halves C and D.
const PC1: [u8; 56] = [
    56, 48, 40, 32, 24, 16,  8,
     0, 57, 49, 41, 33, 25, 17,
     9,  1, 58, 50, 42, 34, 26,
    18, 10,  2, 59, 51, 43, 35,
    62, 54, 46, 38, 30, 22, 14,
     6, 61, 53, 45, 37, 29, 21,
    13,  5, 60, 52, 44, 36, 28,
    20, 12,  4, 27, 19, 11,  3,
];

/// PC-2 — Permuted Choice 2 (0-indexed)
///
/// Selects 48 of the 56 key bits to form each round subkey.
/// The 8 discarded positions act as a compression step.
const PC2: [u8; 48] = [
    13, 16, 10, 23,  0,  4,
     2, 27, 14,  5, 20,  9,
    22, 18, 11,  3, 25,  7,
    15,  6, 26, 19, 12,  1,
    40, 51, 30, 36, 46, 54,
    29, 39, 50, 44, 32, 47,
    43, 48, 38, 55, 33, 52,
    45, 41, 49, 35, 28, 31,
];

/// E — Expansion permutation (0-indexed)
///
/// Expands the 32-bit right half to 48 bits by copying border bits of each
/// 4-bit group into adjacent 6-bit groups. This allows the 48-bit subkey
/// to mix into every bit position.
const E: [u8; 48] = [
    31,  0,  1,  2,  3,  4,
     3,  4,  5,  6,  7,  8,
     7,  8,  9, 10, 11, 12,
    11, 12, 13, 14, 15, 16,
    15, 16, 17, 18, 19, 20,
    19, 20, 21, 22, 23, 24,
    23, 24, 25, 26, 27, 28,
    27, 28, 29, 30, 31,  0,
];

/// P — Post-S-box permutation (0-indexed)
///
/// Disperses the 32-bit S-box output across all bit positions so that each
/// round affects every bit of the next round's input.
const P: [u8; 32] = [
    15,  6, 19, 20, 28, 11, 27, 16,
     0, 14, 22, 25,  4, 17, 30,  9,
     1,  7, 23, 13, 31, 26,  2,  8,
    18, 12, 29,  5, 21, 10,  3, 24,
];

/// Left-rotation amounts for the key schedule halves C and D.
///
/// Total across 16 rounds = 28 (one full rotation of a 28-bit register).
/// Rounds 1, 2, 9, 16 rotate by 1; all others rotate by 2.
const SHIFTS: [u8; 16] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];

// ─────────────────────────────────────────────────────────────────────────────
// S-Boxes: the core non-linearity of DES
//
// Eight substitution boxes, each mapping 6 bits → 4 bits.
// Without S-boxes, DES would be linear and solvable with Gaussian elimination.
//
// Reading an S-box with 6 input bits b₁b₂b₃b₄b₅b₆:
//   row = 2·b₁ + b₆            (outer bits, range 0–3)
//   col = 8·b₂ + 4·b₃ + 2·b₄ + b₅  (inner bits, range 0–15)
//   output = SBOX[box][row][col]
//
// These S-boxes were redesigned by the NSA from IBM's originals. In 1990,
// Biham and Shamir proved they resist differential cryptanalysis — a technique
// the NSA knew about in 1974 but kept classified. The S-boxes were hardened,
// not backdoored.
// ─────────────────────────────────────────────────────────────────────────────

/// DES S-boxes: 8 boxes × 4 rows × 16 columns.
/// Each entry maps a 6-bit input (row=outer bits, col=inner bits) to a 4-bit output.
const SBOXES: [[[u8; 16]; 4]; 8] = [
    // S1
    [
        [14,  4, 13,  1,  2, 15, 11,  8,  3, 10,  6, 12,  5,  9,  0,  7],
        [ 0, 15,  7,  4, 14,  2, 13,  1, 10,  6, 12, 11,  9,  5,  3,  8],
        [ 4,  1, 14,  8, 13,  6,  2, 11, 15, 12,  9,  7,  3, 10,  5,  0],
        [15, 12,  8,  2,  4,  9,  1,  7,  5, 11,  3, 14, 10,  0,  6, 13],
    ],
    // S2
    [
        [15,  1,  8, 14,  6, 11,  3,  4,  9,  7,  2, 13, 12,  0,  5, 10],
        [ 3, 13,  4,  7, 15,  2,  8, 14, 12,  0,  1, 10,  6,  9, 11,  5],
        [ 0, 14,  7, 11, 10,  4, 13,  1,  5,  8, 12,  6,  9,  3,  2, 15],
        [13,  8, 10,  1,  3, 15,  4,  2, 11,  6,  7, 12,  0,  5, 14,  9],
    ],
    // S3
    [
        [10,  0,  9, 14,  6,  3, 15,  5,  1, 13, 12,  7, 11,  4,  2,  8],
        [13,  7,  0,  9,  3,  4,  6, 10,  2,  8,  5, 14, 12, 11, 15,  1],
        [13,  6,  4,  9,  8, 15,  3,  0, 11,  1,  2, 12,  5, 10, 14,  7],
        [ 1, 10, 13,  0,  6,  9,  8,  7,  4, 15, 14,  3, 11,  5,  2, 12],
    ],
    // S4
    [
        [ 7, 13, 14,  3,  0,  6,  9, 10,  1,  2,  8,  5, 11, 12,  4, 15],
        [13,  8, 11,  5,  6, 15,  0,  3,  4,  7,  2, 12,  1, 10, 14,  9],
        [10,  6,  9,  0, 12, 11,  7, 13, 15,  1,  3, 14,  5,  2,  8,  4],
        [ 3, 15,  0,  6, 10,  1, 13,  8,  9,  4,  5, 11, 12,  7,  2, 14],
    ],
    // S5
    [
        [ 2, 12,  4,  1,  7, 10, 11,  6,  8,  5,  3, 15, 13,  0, 14,  9],
        [14, 11,  2, 12,  4,  7, 13,  1,  5,  0, 15, 10,  3,  9,  8,  6],
        [ 4,  2,  1, 11, 10, 13,  7,  8, 15,  9, 12,  5,  6,  3,  0, 14],
        [11,  8, 12,  7,  1, 14,  2, 13,  6, 15,  0,  9, 10,  4,  5,  3],
    ],
    // S6
    [
        [12,  1, 10, 15,  9,  2,  6,  8,  0, 13,  3,  4, 14,  7,  5, 11],
        [10, 15,  4,  2,  7, 12,  9,  5,  6,  1, 13, 14,  0, 11,  3,  8],
        [ 9, 14, 15,  5,  2,  8, 12,  3,  7,  0,  4, 10,  1, 13, 11,  6],
        [ 4,  3,  2, 12,  9,  5, 15, 10, 11, 14,  1,  7,  6,  0,  8, 13],
    ],
    // S7
    [
        [ 4, 11,  2, 14, 15,  0,  8, 13,  3, 12,  9,  7,  5, 10,  6,  1],
        [13,  0, 11,  7,  4,  9,  1, 10, 14,  3,  5, 12,  2, 15,  8,  6],
        [ 1,  4, 11, 13, 12,  3,  7, 14, 10, 15,  6,  8,  0,  5,  9,  2],
        [ 6, 11, 13,  8,  1,  4, 10,  7,  9,  5,  0, 15, 14,  2,  3, 12],
    ],
    // S8
    [
        [13,  2,  8,  4,  6, 15, 11,  1, 10,  9,  3, 14,  5,  0, 12,  7],
        [ 1, 15, 13,  8, 10,  3,  7,  4, 12,  5,  6, 11,  0, 14,  9,  2],
        [ 7, 11,  4,  1,  9, 12, 14,  2,  0,  6, 10, 13, 15,  3,  5,  8],
        [ 2,  1, 14,  7,  4, 10,  8, 13, 15, 12,  9,  0,  3,  5,  6, 11],
    ],
];

// ─────────────────────────────────────────────────────────────────────────────
// Bit manipulation helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Convert an 8-byte slice to a 64-bit array of bits (MSB first within each byte).
///
/// Byte 0 bit 7 → bits[0], byte 0 bit 0 → bits[7], byte 1 bit 7 → bits[8], etc.
fn bytes_to_bits(data: &[u8]) -> Vec<u8> {
    let mut bits = Vec::with_capacity(data.len() * 8);
    for &byte in data {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1);
        }
    }
    bits
}

/// Convert a bit array (MSB first) back to bytes.
fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(bits.len() / 8);
    for chunk in bits.chunks(8) {
        let mut byte = 0u8;
        for &bit in chunk {
            byte = (byte << 1) | bit;
        }
        result.push(byte);
    }
    result
}

/// Apply a permutation table (0-indexed positions) to a bit vector.
fn permute(bits: &[u8], table: &[u8]) -> Vec<u8> {
    table.iter().map(|&pos| bits[pos as usize]).collect()
}

/// Left-rotate a 28-bit key half by `n` positions.
fn left_rotate_28(half: &[u8], n: u8) -> Vec<u8> {
    let n = n as usize;
    let mut result = Vec::with_capacity(28);
    result.extend_from_slice(&half[n..]);
    result.extend_from_slice(&half[..n]);
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Key schedule: expand_key
// ─────────────────────────────────────────────────────────────────────────────

/// Derive the 16 DES round subkeys from an 8-byte key.
///
/// The DES key is 64 bits wide but only 56 bits are key material — bits at
/// positions 7, 15, 23, 31, 39, 47, 55, 63 (0-indexed) are parity bits and
/// are dropped by PC-1. This function accepts any 8-byte key and ignores parity.
///
/// Returns an array of 16 subkeys, each 6 bytes (48 bits).
///
/// ## Key Schedule Algorithm
///
/// 1. PC-1: 64-bit key → 56 bits (drop parity), split into C₀ (28) and D₀ (28)
/// 2. For each round i = 1..16:
///    - C_i = LeftRotate(C_{i-1}, SHIFTS[i])
///    - D_i = LeftRotate(D_{i-1}, SHIFTS[i])
///    - K_i = PC-2(C_i ∥ D_i)   (56 → 48 bits)
pub fn expand_key(key: &[u8; 8]) -> [[u8; 6]; 16] {
    let key_bits = bytes_to_bits(key);
    let permuted = permute(&key_bits, &PC1); // 64 → 56 bits
    let mut c = permuted[..28].to_vec();
    let mut d = permuted[28..].to_vec();

    let mut subkeys = [[0u8; 6]; 16];
    for (i, &shift) in SHIFTS.iter().enumerate() {
        c = left_rotate_28(&c, shift);
        d = left_rotate_28(&d, shift);

        let mut cd = c.clone();
        cd.extend_from_slice(&d);
        let subkey_bits = permute(&cd, &PC2); // 56 → 48 bits
        let subkey_bytes = bits_to_bytes(&subkey_bits);
        subkeys[i].copy_from_slice(&subkey_bytes);
    }
    subkeys
}

// ─────────────────────────────────────────────────────────────────────────────
// Round function f(R, K)
// ─────────────────────────────────────────────────────────────────────────────

/// DES round function f(R, K):
///
/// 1. E(R)   — expand 32-bit right half to 48 bits
/// 2. XOR    — mix in the 48-bit round subkey
/// 3. S      — 8 S-boxes, each 6 bits → 4 bits, total 48 → 32 bits
/// 4. P      — final 32-bit permutation
///
/// The S-boxes are the only non-linear step. Without them, DES would be
/// entirely linear and solvable with a system of linear equations over GF(2).
fn feistel_f(right: &[u8], subkey: &[u8; 6]) -> Vec<u8> {
    // Step 1: Expand R from 32 → 48 bits using expansion permutation E
    let expanded = permute(right, &E);

    // Step 2: XOR with the 48-bit subkey
    let subkey_bits = bytes_to_bits(subkey);
    let xored: Vec<u8> = expanded.iter().zip(subkey_bits.iter()).map(|(&a, &b)| a ^ b).collect();

    // Step 3: Apply S-boxes (8 × 6-bit → 4-bit substitutions)
    // Each group of 6 bits selects a row (outer bits b1,b6) and column (inner bits b2..b5).
    let mut sbox_out: Vec<u8> = Vec::with_capacity(32);
    for box_idx in 0..8 {
        let chunk = &xored[box_idx * 6..(box_idx + 1) * 6];
        // Row = outer bits: first (b1) and last (b6)
        let row = ((chunk[0] << 1) | chunk[5]) as usize;
        // Col = inner 4 bits: b2 b3 b4 b5
        let col = ((chunk[1] << 3) | (chunk[2] << 2) | (chunk[3] << 1) | chunk[4]) as usize;
        let val = SBOXES[box_idx][row][col];
        // Convert 4-bit value to bits (MSB first)
        for bit_pos in (0..4).rev() {
            sbox_out.push((val >> bit_pos) & 1);
        }
    }

    // Step 4: P permutation (32 → 32 bit reordering for diffusion)
    permute(&sbox_out, &P)
}

// ─────────────────────────────────────────────────────────────────────────────
// Core block cipher
// ─────────────────────────────────────────────────────────────────────────────

/// Encrypt or decrypt a single 8-byte block using the provided subkeys.
///
/// Encryption: pass subkeys in order (K1..K16)
/// Decryption: pass subkeys in reverse order (K16..K1)
///
/// This is the beauty of the Feistel structure — decryption requires no
/// inverse round function, just reversed subkeys. The same hardware handles
/// both directions.
fn des_block(block: &[u8; 8], subkeys: &[[u8; 6]; 16]) -> [u8; 8] {
    let mut bits = bytes_to_bits(block);

    // Initial permutation — rearranges bits for historical hardware reasons
    bits = permute(&bits, &IP);

    // Split into L₀ (left 32 bits) and R₀ (right 32 bits)
    let (mut left, mut right) = (bits[..32].to_vec(), bits[32..].to_vec());

    // 16 Feistel rounds:
    //   L_i = R_{i-1}
    //   R_i = L_{i-1} XOR f(R_{i-1}, K_i)
    for subkey in subkeys {
        let f_out = feistel_f(&right, subkey);
        let new_right: Vec<u8> = left.iter().zip(f_out.iter()).map(|(&l, &f)| l ^ f).collect();
        left = right;
        right = new_right;
    }

    // Swap halves before final permutation (standard DES step)
    let mut combined = right;
    combined.extend_from_slice(&left);

    // Final permutation (IP⁻¹) — undoes the initial permutation
    let result_bits = permute(&combined, &FP);
    let result_bytes = bits_to_bytes(&result_bits);
    let mut out = [0u8; 8];
    out.copy_from_slice(&result_bytes);
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Public block cipher API
// ─────────────────────────────────────────────────────────────────────────────

/// Encrypt a single 64-bit (8-byte) block using DES.
///
/// # Arguments
///
/// * `block` — 8 bytes of plaintext
/// * `key`   — 8 bytes (64 bits, of which 56 are key material; 8 are parity)
///
/// # Returns
///
/// 8 bytes of ciphertext
///
/// Note: This is the raw block cipher. For variable-length data, use
/// `ecb_encrypt` (ECB mode with PKCS#7 padding) — but ECB mode is insecure
/// for most purposes. See CBC, CTR, or GCM for real applications.
///
/// # Example
///
/// ```
/// use coding_adventures_des::encrypt_block;
/// let key = [0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1];
/// let plain = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
/// assert_eq!(encrypt_block(&plain, &key), [0x85, 0xE8, 0x13, 0x54, 0x0F, 0x0A, 0xB4, 0x05]);
/// ```
pub fn encrypt_block(block: &[u8; 8], key: &[u8; 8]) -> [u8; 8] {
    let subkeys = expand_key(key);
    des_block(block, &subkeys)
}

/// Decrypt a single 64-bit (8-byte) block using DES.
///
/// Decryption is encryption with the subkeys in reverse order — a direct
/// consequence of the Feistel structure's self-inverse property.
///
/// # Arguments
///
/// * `block` — 8 bytes of ciphertext
/// * `key`   — 8 bytes (same key used for encryption)
///
/// # Returns
///
/// 8 bytes of plaintext
///
/// # Example
///
/// ```
/// use coding_adventures_des::{encrypt_block, decrypt_block};
/// let key = [0x13, 0x34, 0x57, 0x79, 0x9B, 0xBC, 0xDF, 0xF1];
/// let plain = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
/// let ct = encrypt_block(&plain, &key);
/// assert_eq!(decrypt_block(&ct, &key), plain);
/// ```
pub fn decrypt_block(block: &[u8; 8], key: &[u8; 8]) -> [u8; 8] {
    let mut subkeys = expand_key(key);
    subkeys.reverse();
    des_block(block, &subkeys)
}

// ─────────────────────────────────────────────────────────────────────────────
// ECB mode (educational only)
// ─────────────────────────────────────────────────────────────────────────────

/// Apply PKCS#7 padding to reach a multiple of `block_size`.
///
/// Appends N bytes each with value N, where N is the number of bytes needed
/// to reach the next block boundary (1 ≤ N ≤ block_size).
///
/// If the data is already block-aligned, a full padding block is added so
/// that unpadding is always unambiguous.
///
/// Example: 5 bytes, block_size=8 → append 3 bytes of value 0x03.
fn pkcs7_pad(data: &[u8], block_size: usize) -> Vec<u8> {
    let pad_len = block_size - (data.len() % block_size);
    let mut result = data.to_vec();
    result.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    result
}

/// Remove PKCS#7 padding. Returns an error if padding is invalid.
fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() {
        return Err("Cannot unpad empty data".to_string());
    }
    let pad_len = *data.last().unwrap() as usize;
    if pad_len == 0 || pad_len > 8 {
        return Err(format!("Invalid PKCS#7 padding byte: {}", pad_len));
    }
    if data.len() < pad_len {
        return Err("Padding length exceeds data length".to_string());
    }
    let padding_slice = &data[data.len() - pad_len..];
    if !padding_slice.iter().all(|&b| b == pad_len as u8) {
        return Err("Invalid PKCS#7 padding (bytes do not match)".to_string());
    }
    Ok(data[..data.len() - pad_len].to_vec())
}

/// Encrypt variable-length plaintext with DES in ECB mode (PKCS#7 padding).
///
/// WARNING: ECB mode is insecure for most purposes. Identical 8-byte
/// plaintext blocks always produce identical ciphertext blocks, leaking
/// data patterns. Use CBC or CTR mode for real data.
///
/// This function exists for compatibility with historical data and as an
/// educational demonstration of ECB's weakness.
///
/// # Example
///
/// ```
/// use coding_adventures_des::{ecb_encrypt, ecb_decrypt};
/// let key = [0x01, 0x33, 0x45, 0x77, 0x99, 0xBB, 0xCD, 0xFF];
/// let plain = b"hello";
/// let ct = ecb_encrypt(plain, &key);
/// assert_eq!(ecb_decrypt(&ct, &key).unwrap(), plain);
/// ```
pub fn ecb_encrypt(plaintext: &[u8], key: &[u8; 8]) -> Vec<u8> {
    let subkeys = expand_key(key);
    let padded = pkcs7_pad(plaintext, 8);
    let mut result = Vec::with_capacity(padded.len());
    for chunk in padded.chunks(8) {
        let mut block = [0u8; 8];
        block.copy_from_slice(chunk);
        result.extend_from_slice(&des_block(&block, &subkeys));
    }
    result
}

/// Decrypt variable-length ciphertext with DES in ECB mode.
///
/// # Arguments
///
/// * `ciphertext` — bytes (must be a non-empty multiple of 8 bytes)
/// * `key`        — 8 bytes
///
/// # Returns
///
/// Plaintext with PKCS#7 padding removed, or an error string.
pub fn ecb_decrypt(ciphertext: &[u8], key: &[u8; 8]) -> Result<Vec<u8>, String> {
    if ciphertext.len() % 8 != 0 {
        return Err("DES ECB ciphertext length must be a multiple of 8 bytes".to_string());
    }
    if ciphertext.is_empty() {
        return Err("Ciphertext must not be empty".to_string());
    }
    let mut subkeys = expand_key(key);
    subkeys.reverse();
    let mut result = Vec::with_capacity(ciphertext.len());
    for chunk in ciphertext.chunks(8) {
        let mut block = [0u8; 8];
        block.copy_from_slice(chunk);
        result.extend_from_slice(&des_block(&block, &subkeys));
    }
    pkcs7_unpad(&result)
}

// ─────────────────────────────────────────────────────────────────────────────
// Triple DES (3DES / TDEA)
// ─────────────────────────────────────────────────────────────────────────────

/// Encrypt one 8-byte block with Triple DES (3TDEA / EDE mode).
///
/// Algorithm (NIST SP 800-67): C = E_K1(D_K2(E_K3(P)))
///
/// Applied right-to-left to plaintext:
///   1. Encrypt with K3
///   2. Decrypt with K2
///   3. Encrypt with K1
///
/// The EDE (Encrypt-Decrypt-Encrypt) structure provides backward
/// compatibility: if K1 = K2 = K3 = K, then 3DES reduces to single DES:
///   E(K, D(K, E(K, P))) = E(K, P)    since D(K, E(K, x)) = x.
///
/// Effective security: ~112 bits (168-bit key reduced by meet-in-the-middle).
///
/// NIST deprecated 3DES for new applications in 2017 and disallowed
/// it entirely in 2023 due to the SWEET32 attack on 64-bit block sizes.
///
/// # Example
///
/// ```
/// use coding_adventures_des::{tdea_encrypt_block, tdea_decrypt_block};
/// let k1 = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];
/// let k2 = [0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01];
/// let k3 = [0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x01, 0x23];
/// let plain = [0x6B, 0xC1, 0xBE, 0xE2, 0x2E, 0x40, 0x9F, 0x96];
/// let ct = tdea_encrypt_block(&plain, &k1, &k2, &k3);
/// assert_eq!(tdea_decrypt_block(&ct, &k1, &k2, &k3), plain);
/// ```
pub fn tdea_encrypt_block(block: &[u8; 8], k1: &[u8; 8], k2: &[u8; 8], k3: &[u8; 8]) -> [u8; 8] {
    let step1 = encrypt_block(block, k3); // E_K3(P)
    let step2 = decrypt_block(&step1, k2); // D_K2(E_K3(P))
    encrypt_block(&step2, k1)             // E_K1(D_K2(E_K3(P)))
}

/// Decrypt one 8-byte block with Triple DES (3TDEA / EDE mode).
///
/// Algorithm (NIST SP 800-67): P = D_K3(E_K2(D_K1(C)))
///
/// Applied right-to-left to ciphertext:
///   1. Decrypt with K1
///   2. Encrypt with K2
///   3. Decrypt with K3
pub fn tdea_decrypt_block(block: &[u8; 8], k1: &[u8; 8], k2: &[u8; 8], k3: &[u8; 8]) -> [u8; 8] {
    let step1 = decrypt_block(block, k1); // D_K1(C)
    let step2 = encrypt_block(&step1, k2); // E_K2(D_K1(C))
    decrypt_block(&step2, k3)             // D_K3(E_K2(D_K1(C)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn h(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    fn h8(s: &str) -> [u8; 8] {
        let v = h(s);
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&v);
        arr
    }

    // ─── NIST / FIPS 46 Known-Answer Tests ──────────────────────────────────

    #[test]
    fn fips_vector_1_encrypt() {
        // Classic DES example from Stallings / FIPS 46 worked example.
        // Key = 133457799BBCDFF1 (parity bits at bit 8 of each byte).
        assert_eq!(
            encrypt_block(&h8("0123456789ABCDEF"), &h8("133457799BBCDFF1")),
            h8("85E813540F0AB405")
        );
    }

    #[test]
    fn fips_vector_1_decrypt() {
        assert_eq!(
            decrypt_block(&h8("85E813540F0AB405"), &h8("133457799BBCDFF1")),
            h8("0123456789ABCDEF")
        );
    }

    #[test]
    fn sp800_20_table1_row0() {
        // SP 800-20 Table 1 — plaintext variable, key = 0101...01
        assert_eq!(
            encrypt_block(&h8("95F8A5E5DD31D900"), &h8("0101010101010101")),
            h8("8000000000000000")
        );
    }

    #[test]
    fn sp800_20_table1_row1() {
        assert_eq!(
            encrypt_block(&h8("DD7F121CA5015619"), &h8("0101010101010101")),
            h8("4000000000000000")
        );
    }

    #[test]
    fn sp800_20_table1_row2() {
        assert_eq!(
            encrypt_block(&h8("2E8653104F3834EA"), &h8("0101010101010101")),
            h8("2000000000000000")
        );
    }

    #[test]
    fn sp800_20_table2_key_variable_row0() {
        // SP 800-20 Table 2 — key variable, plaintext = 00...00
        assert_eq!(
            encrypt_block(&h8("0000000000000000"), &h8("8001010101010101")),
            h8("95A8D72813DAA94D")
        );
    }

    #[test]
    fn sp800_20_table2_key_variable_row1() {
        assert_eq!(
            encrypt_block(&h8("0000000000000000"), &h8("4001010101010101")),
            h8("0EEC1487DD8C26D5")
        );
    }

    // ─── Round-trip tests ────────────────────────────────────────────────────

    #[test]
    fn roundtrip_fips_vector() {
        let key = h8("133457799BBCDFF1");
        let plain = h8("0123456789ABCDEF");
        let ct = encrypt_block(&plain, &key);
        assert_eq!(decrypt_block(&ct, &key), plain);
    }

    #[test]
    fn roundtrip_all_byte_values() {
        let key = h8("FEDCBA9876543210");
        for start in (0u8..=248).step_by(8) {
            let block: [u8; 8] = [start, start+1, start+2, start+3, start+4, start+5, start+6, start+7];
            assert_eq!(decrypt_block(&encrypt_block(&block, &key), &key), block);
        }
    }

    #[test]
    fn roundtrip_multiple_keys() {
        let plain = h8("0123456789ABCDEF");
        for key in [
            h8("133457799BBCDFF0"),
            h8("FFFFFFFFFFFFFFFF"),
            h8("0000000000000000"),
            h8("FEDCBA9876543210"),
        ] {
            assert_eq!(decrypt_block(&encrypt_block(&plain, &key), &key), plain);
        }
    }

    #[test]
    fn parity_bit_only_key_roundtrip() {
        // Key with only the parity bit set in byte 8 position.
        let key = h8("0000000000000080");
        let plain = h8("0000000000000000");
        let ct = encrypt_block(&plain, &key);
        assert_eq!(decrypt_block(&ct, &key), plain);
    }

    // ─── expand_key tests ────────────────────────────────────────────────────

    #[test]
    fn expand_key_returns_16_subkeys() {
        let key = h8("0133457799BBCDFF");
        let subkeys = expand_key(&key);
        assert_eq!(subkeys.len(), 16);
    }

    #[test]
    fn expand_key_subkeys_are_6_bytes() {
        let key = h8("0133457799BBCDFF");
        for sk in expand_key(&key) {
            assert_eq!(sk.len(), 6);
        }
    }

    #[test]
    fn expand_key_different_keys_different_subkeys() {
        let sk1 = expand_key(&h8("0133457799BBCDFF"));
        let sk2 = expand_key(&h8("FEDCBA9876543210"));
        assert_ne!(sk1[0], sk2[0]);
    }

    #[test]
    fn expand_key_subkeys_not_all_same() {
        // A degenerate key schedule with all-equal subkeys would be broken.
        let key = h8("0133457799BBCDFF");
        let subkeys = expand_key(&key);
        // Not all 16 should be identical
        let first = subkeys[0];
        let all_same = subkeys.iter().all(|&sk| sk == first);
        assert!(!all_same, "All subkeys are the same — degenerate key schedule");
    }

    // ─── ECB mode tests ──────────────────────────────────────────────────────

    #[test]
    fn ecb_single_block_exact_size() {
        // 8-byte input → 16 bytes out (1 data block + 1 full padding block)
        let key = h8("0133457799BBCDFF");
        let plain = h8("0123456789ABCDEF");
        let ct = ecb_encrypt(&plain, &key);
        assert_eq!(ct.len(), 16);
    }

    #[test]
    fn ecb_sub_block_size() {
        // Less than 8 bytes → padded to 8 bytes → 8 bytes ciphertext
        let key = h8("0133457799BBCDFF");
        let ct = ecb_encrypt(b"hello", &key);
        assert_eq!(ct.len(), 8);
    }

    #[test]
    fn ecb_multi_block() {
        // 16 bytes input → 24 bytes out (2 data blocks + 1 padding block)
        let key = h8("0133457799BBCDFF");
        let plain: Vec<u8> = (0..16).collect();
        let ct = ecb_encrypt(&plain, &key);
        assert_eq!(ct.len(), 24);
    }

    #[test]
    fn ecb_empty_input() {
        // Empty input → 8 bytes (full padding block)
        let key = h8("0133457799BBCDFF");
        let ct = ecb_encrypt(b"", &key);
        assert_eq!(ct.len(), 8);
    }

    #[test]
    fn ecb_deterministic() {
        let key = h8("0133457799BBCDFF");
        let plain = b"Hello, World!!!";
        assert_eq!(ecb_encrypt(plain, &key), ecb_encrypt(plain, &key));
    }

    #[test]
    fn ecb_roundtrip_short() {
        let key = h8("0133457799BBCDFF");
        let plain = b"hello".as_ref();
        assert_eq!(ecb_decrypt(&ecb_encrypt(plain, &key), &key).unwrap(), plain);
    }

    #[test]
    fn ecb_roundtrip_exact_block() {
        let key = h8("0133457799BBCDFF");
        let plain = b"ABCDEFGH".as_ref();
        assert_eq!(ecb_decrypt(&ecb_encrypt(plain, &key), &key).unwrap(), plain);
    }

    #[test]
    fn ecb_roundtrip_multi_block() {
        let key = h8("0133457799BBCDFF");
        let plain = b"The quick brown fox jumps".as_ref();
        assert_eq!(ecb_decrypt(&ecb_encrypt(plain, &key), &key).unwrap(), plain);
    }

    #[test]
    fn ecb_roundtrip_empty() {
        let key = h8("0133457799BBCDFF");
        let result = ecb_decrypt(&ecb_encrypt(b"", &key), &key).unwrap();
        assert_eq!(result, b"");
    }

    #[test]
    fn ecb_roundtrip_large() {
        let key = h8("0133457799BBCDFF");
        let plain: Vec<u8> = (0u8..=255).collect();
        assert_eq!(ecb_decrypt(&ecb_encrypt(&plain, &key), &key).unwrap(), plain);
    }

    #[test]
    fn ecb_decrypt_invalid_length() {
        let key = h8("0133457799BBCDFF");
        let result = ecb_decrypt(&[0u8; 7], &key);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("multiple of 8"));
    }

    #[test]
    fn ecb_decrypt_empty_ciphertext() {
        let key = h8("0133457799BBCDFF");
        let result = ecb_decrypt(&[], &key);
        assert!(result.is_err());
    }

    #[test]
    fn ecb_decrypt_bad_padding() {
        let key = h8("0133457799BBCDFF");
        let ct = ecb_encrypt(b"test data", &key);
        // Flip the last byte to corrupt the padding block
        let mut corrupted = ct.clone();
        *corrupted.last_mut().unwrap() ^= 0xFF;
        assert!(ecb_decrypt(&corrupted, &key).is_err());
    }

    // ─── 3DES (TDEA) tests ───────────────────────────────────────────────────

    #[test]
    fn tdea_encrypt_nist_vector() {
        // NIST SP 800-67 EDE ordering: E_K1(D_K2(E_K3(P)))
        let k1 = h8("0123456789ABCDEF");
        let k2 = h8("23456789ABCDEF01");
        let k3 = h8("456789ABCDEF0123");
        let plain = h8("6BC1BEE22E409F96");
        assert_eq!(tdea_encrypt_block(&plain, &k1, &k2, &k3), h8("3B6423D418DEFC23"));
    }

    #[test]
    fn tdea_decrypt_nist_vector() {
        let k1 = h8("0123456789ABCDEF");
        let k2 = h8("23456789ABCDEF01");
        let k3 = h8("456789ABCDEF0123");
        assert_eq!(
            tdea_decrypt_block(&h8("3B6423D418DEFC23"), &k1, &k2, &k3),
            h8("6BC1BEE22E409F96")
        );
    }

    #[test]
    fn tdea_roundtrip_random_keys() {
        let k1 = h8("FEDCBA9876543210");
        let k2 = h8("0F1E2D3C4B5A6978");
        let k3 = h8("7869584A3B2C1D0E");
        let plain = h8("0123456789ABCDEF");
        let ct = tdea_encrypt_block(&plain, &k1, &k2, &k3);
        assert_eq!(tdea_decrypt_block(&ct, &k1, &k2, &k3), plain);
    }

    #[test]
    fn tdea_backward_compat_k1_eq_k2_eq_k3() {
        // When K1=K2=K3, 3DES EDE reduces to single DES.
        // EDE(K,K,K): E(K, D(K, E(K, P))) = E(K, P) since D(K,E(K,x)) = x
        let key = h8("0133457799BBCDFF");
        let plain = h8("0123456789ABCDEF");
        assert_eq!(
            tdea_encrypt_block(&plain, &key, &key, &key),
            encrypt_block(&plain, &key)
        );
    }

    #[test]
    fn tdea_decrypt_backward_compat() {
        let key = h8("FEDCBA9876543210");
        let ct = h8("0123456789ABCDEF");
        assert_eq!(
            tdea_decrypt_block(&ct, &key, &key, &key),
            decrypt_block(&ct, &key)
        );
    }

    #[test]
    fn tdea_roundtrip_all_same_block() {
        let k1 = h8("1234567890ABCDEF");
        let k2 = h8("FEDCBA0987654321");
        let k3 = h8("0F0F0F0F0F0F0F0F");
        for &val in &[0x00u8, 0xFF, 0xA5, 0x5A] {
            let plain = [val; 8];
            assert_eq!(tdea_decrypt_block(&tdea_encrypt_block(&plain, &k1, &k2, &k3), &k1, &k2, &k3), plain);
        }
    }

    // ─── Additional coverage ─────────────────────────────────────────────────

    #[test]
    fn sp800_20_all_zero_key_known_answer() {
        // First SP 800-20 Table 2 entry
        let key = h8("8001010101010101");
        let plain = h8("0000000000000000");
        assert_eq!(encrypt_block(&plain, &key), h8("95A8D72813DAA94D"));
    }

    #[test]
    fn encrypt_decrypt_all_zeros_key() {
        let key = [0u8; 8];
        let plain = [0u8; 8];
        let ct = encrypt_block(&plain, &key);
        assert_eq!(decrypt_block(&ct, &key), plain);
    }

    #[test]
    fn encrypt_decrypt_all_ff() {
        let key = [0xFFu8; 8];
        let plain = [0xFFu8; 8];
        let ct = encrypt_block(&plain, &key);
        assert_eq!(decrypt_block(&ct, &key), plain);
    }

    #[test]
    fn ecb_roundtrip_exactly_two_blocks() {
        let key = h8("FEDCBA9876543210");
        let plain = [0xAAu8; 16]; // exactly 2 blocks
        assert_eq!(ecb_decrypt(&ecb_encrypt(&plain, &key), &key).unwrap(), plain);
    }
}
