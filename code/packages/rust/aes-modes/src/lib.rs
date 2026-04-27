//! # coding_adventures_aes_modes — AES Modes of Operation
//!
//! A block cipher like AES operates on fixed 128-bit (16-byte) blocks. To
//! encrypt messages of arbitrary length, you need a **mode of operation** —
//! a recipe that describes how to chain multiple block-cipher calls together.
//!
//! The choice of mode is critical for security:
//!
//! ```text
//! Mode   | Security          | Properties
//! -------|-------------------|--------------------------------------------
//! ECB    | BROKEN            | Each block encrypted independently.
//!        |                   | Identical plaintext → identical ciphertext.
//! -------|-------------------|--------------------------------------------
//! CBC    | Legacy (padding   | XOR with previous ciphertext before encrypt.
//!        | oracle attacks)   | Requires unpredictable IV.
//! -------|-------------------|--------------------------------------------
//! CTR    | Modern, secure    | Stream cipher: encrypt counter, XOR plaintext.
//!        |                   | Parallelizable, no padding needed.
//! -------|-------------------|--------------------------------------------
//! GCM    | Modern, secure +  | CTR + GHASH authentication tag.
//!        | authenticated     | Gold standard for TLS 1.3.
//! ```
//!
//! ## Public API
//!
//! - `ecb_encrypt(plaintext, key) -> ciphertext`
//! - `ecb_decrypt(ciphertext, key) -> plaintext`
//! - `cbc_encrypt(plaintext, key, iv) -> ciphertext`
//! - `cbc_decrypt(ciphertext, key, iv) -> plaintext`
//! - `ctr_encrypt(plaintext, key, nonce) -> ciphertext`
//! - `ctr_decrypt(ciphertext, key, nonce) -> plaintext`
//! - `gcm_encrypt(plaintext, key, iv, aad) -> (ciphertext, tag)`
//! - `gcm_decrypt(ciphertext, key, iv, aad, tag) -> plaintext`

use coding_adventures_aes::{encrypt_block, decrypt_block};

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// AES block size in bytes. AES always operates on 128-bit = 16-byte blocks.
const BLOCK_SIZE: usize = 16;

// ─────────────────────────────────────────────────────────────────────────────
// PKCS#7 Padding
// ─────────────────────────────────────────────────────────────────────────────
//
// Block ciphers need input that is an exact multiple of the block size (16 bytes
// for AES). PKCS#7 padding fills the gap:
//
//   - If the plaintext is N bytes short of a full block, append N copies of
//     the byte N. Example: 3 bytes short → append [0x03, 0x03, 0x03].
//   - If already block-aligned, append a full block of 16 bytes each = 0x10.
//     This ensures unpadding is always unambiguous.
//
// To unpad: read the last byte N, verify last N bytes all equal N, strip them.

/// Apply PKCS#7 padding. Adds 1–16 bytes so the result length is a multiple of 16.
pub fn pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let pad_len = BLOCK_SIZE - (data.len() % BLOCK_SIZE);
    let mut result = Vec::with_capacity(data.len() + pad_len);
    result.extend_from_slice(data);
    result.extend(std::iter::repeat(pad_len as u8).take(pad_len));
    result
}

/// Remove PKCS#7 padding. Returns an error if the padding is invalid.
pub fn pkcs7_unpad(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.is_empty() || data.len() % BLOCK_SIZE != 0 {
        return Err("Invalid padded data: length must be a positive multiple of 16".into());
    }
    let pad_len = *data.last().unwrap() as usize;
    if pad_len < 1 || pad_len > BLOCK_SIZE {
        return Err("Invalid PKCS#7 padding".into());
    }
    // Constant-time padding validation: accumulate differences with OR
    // so the loop always takes the same time regardless of which byte fails.
    let mut diff: u8 = 0;
    for &b in &data[data.len() - pad_len..] {
        diff |= b ^ (pad_len as u8);
    }
    if diff != 0 {
        return Err("Invalid PKCS#7 padding".into());
    }
    Ok(data[..data.len() - pad_len].to_vec())
}

// ─────────────────────────────────────────────────────────────────────────────
// XOR helper
// ─────────────────────────────────────────────────────────────────────────────

/// XOR two byte slices of equal length. Panics if lengths differ.
fn xor_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
    a.iter().zip(b.iter()).map(|(x, y)| x ^ y).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// ECB — Electronic Codebook Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// The simplest mode: encrypt each 16-byte block independently.
//
//   C[i] = AES_encrypt(P[i], key)
//
// *** ECB IS INSECURE FOR REAL USE ***
//
// Identical plaintext blocks produce identical ciphertext blocks.
// The "ECB penguin" demonstrates this — encrypting a bitmap image in ECB
// mode reveals the image structure in the ciphertext.

/// Encrypt with AES in ECB mode (INSECURE — educational only).
///
/// Applies PKCS#7 padding, then encrypts each 16-byte block independently.
pub fn ecb_encrypt(plaintext: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    let padded = pkcs7_pad(plaintext);
    let mut result = Vec::with_capacity(padded.len());

    for chunk in padded.chunks(BLOCK_SIZE) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        let encrypted = encrypt_block(&block, key)?;
        result.extend_from_slice(&encrypted);
    }
    Ok(result)
}

/// Decrypt with AES in ECB mode (INSECURE — educational only).
pub fn ecb_decrypt(ciphertext: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    if ciphertext.is_empty() || ciphertext.len() % BLOCK_SIZE != 0 {
        return Err("ECB ciphertext must be a non-empty multiple of 16 bytes".into());
    }
    let mut result = Vec::with_capacity(ciphertext.len());

    for chunk in ciphertext.chunks(BLOCK_SIZE) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        let decrypted = decrypt_block(&block, key)?;
        result.extend_from_slice(&decrypted);
    }
    pkcs7_unpad(&result)
}

// ─────────────────────────────────────────────────────────────────────────────
// CBC — Cipher Block Chaining Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// CBC chains blocks together:
//   C[0] = AES_encrypt(P[0] XOR IV, key)
//   C[i] = AES_encrypt(P[i] XOR C[i-1], key)
//
// The IV must be unpredictable and never reused with the same key.
// Vulnerable to padding oracle attacks (POODLE, Lucky 13).

/// Encrypt with AES in CBC mode.
pub fn cbc_encrypt(plaintext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, String> {
    if iv.len() != BLOCK_SIZE {
        return Err(format!("CBC IV must be 16 bytes, got {}", iv.len()));
    }
    let padded = pkcs7_pad(plaintext);
    let mut result = Vec::with_capacity(padded.len());
    let mut prev = iv.to_vec();

    for chunk in padded.chunks(BLOCK_SIZE) {
        let xored = xor_bytes(chunk, &prev);
        let block: [u8; 16] = xored.try_into().unwrap();
        let encrypted = encrypt_block(&block, key)?;
        result.extend_from_slice(&encrypted);
        prev = encrypted.to_vec();
    }
    Ok(result)
}

/// Decrypt with AES in CBC mode.
pub fn cbc_decrypt(ciphertext: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>, String> {
    if iv.len() != BLOCK_SIZE {
        return Err(format!("CBC IV must be 16 bytes, got {}", iv.len()));
    }
    if ciphertext.is_empty() || ciphertext.len() % BLOCK_SIZE != 0 {
        return Err("CBC ciphertext must be a non-empty multiple of 16 bytes".into());
    }
    let mut result = Vec::with_capacity(ciphertext.len());
    let mut prev = iv.to_vec();

    for chunk in ciphertext.chunks(BLOCK_SIZE) {
        let block: [u8; 16] = chunk.try_into().unwrap();
        let decrypted = decrypt_block(&block, key)?;
        let plain = xor_bytes(&decrypted, &prev);
        result.extend_from_slice(&plain);
        prev = chunk.to_vec();
    }
    pkcs7_unpad(&result)
}

// ─────────────────────────────────────────────────────────────────────────────
// CTR — Counter Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// Turns a block cipher into a stream cipher:
//   keystream[i] = AES_encrypt(nonce || counter_i, key)
//   C[i] = P[i] XOR keystream[i]
//
// The nonce is 12 bytes, counter is 4-byte big-endian starting at 1.
// No padding needed. Encryption = decryption.
//
// CRITICAL: Never reuse a nonce with the same key.

/// Build a 16-byte CTR counter block: [nonce (12 bytes)] [counter (4 bytes BE)]
fn build_counter_block(nonce: &[u8], counter: u32) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[..12].copy_from_slice(nonce);
    block[12..16].copy_from_slice(&counter.to_be_bytes());
    block
}

/// Encrypt with AES in CTR mode.
///
/// The nonce must be exactly 12 bytes and unique per message.
/// The output is the same length as the input (no padding).
pub fn ctr_encrypt(plaintext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, String> {
    if nonce.len() != 12 {
        return Err(format!("CTR nonce must be 12 bytes, got {}", nonce.len()));
    }
    let mut result = Vec::with_capacity(plaintext.len());
    let mut counter: u32 = 1;

    for chunk in plaintext.chunks(BLOCK_SIZE) {
        let counter_block = build_counter_block(nonce, counter);
        let keystream = encrypt_block(&counter_block, key)?;
        for (i, &byte) in chunk.iter().enumerate() {
            result.push(byte ^ keystream[i]);
        }
        counter = counter.wrapping_add(1);
    }
    Ok(result)
}

/// Decrypt with AES in CTR mode. Identical to encryption (stream cipher property).
pub fn ctr_decrypt(ciphertext: &[u8], key: &[u8], nonce: &[u8]) -> Result<Vec<u8>, String> {
    ctr_encrypt(ciphertext, key, nonce)
}

// ─────────────────────────────────────────────────────────────────────────────
// GCM — Galois/Counter Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// CTR encryption + GHASH authentication tag. Provides authenticated encryption
// with associated data (AEAD). The gold standard for TLS 1.3.
//
// Architecture:
//   1. H = AES_encrypt(0^128, key)       — hash subkey
//   2. J0 = IV || 0x00000001              — initial counter
//   3. CTR-encrypt plaintext at J0+1
//   4. Tag = GHASH(H, AAD, CT) XOR AES_encrypt(J0, key)
//
// GF(2^128) multiplication uses reducing polynomial:
//   R = x^128 + x^7 + x^2 + x + 1    (high byte = 0xE1)

/// Multiply two 128-bit values in GF(2^128) with the GCM reducing polynomial.
///
/// The reducing polynomial R has high byte 0xE1:
///   R = x^128 + x^7 + x^2 + x + 1
///
/// Algorithm (NIST SP 800-38D, Algorithm 1):
///   Z = 0, V = Y
///   for each bit i of X (MSB first):
///     if bit i is set: Z ^= V
///     carry = LSB of V
///     V >>= 1
///     if carry: V[0] ^= 0xE1
fn gf128_mul(x: &[u8; 16], y: &[u8; 16]) -> [u8; 16] {
    let mut z = [0u8; 16];
    let mut v = *y;

    for i in 0..128 {
        // Check bit i of X (MSB-first: bit 0 is MSB of byte 0)
        let byte_idx = i / 8;
        let bit_idx = 7 - (i % 8);
        if (x[byte_idx] >> bit_idx) & 1 == 1 {
            for j in 0..16 {
                z[j] ^= v[j];
            }
        }

        // Check LSB of V (bit 127 = LSB of byte 15)
        let carry = v[15] & 1;

        // Right-shift V by 1 bit
        for j in (1..16).rev() {
            v[j] = (v[j] >> 1) | ((v[j - 1] & 1) << 7);
        }
        v[0] >>= 1;

        // If carry: XOR with R = 0xE100...00
        if carry == 1 {
            v[0] ^= 0xe1;
        }
    }
    z
}

/// GHASH: universal hash function for GCM.
///
/// Processes AAD and ciphertext through GF(2^128) polynomial evaluation:
///   Y[0] = 0^128
///   Y[i] = (Y[i-1] XOR block[i]) * H
///
/// Input sequence: AAD blocks (padded) || CT blocks (padded) || length block
fn ghash(h: &[u8; 16], aad: &[u8], ciphertext: &[u8]) -> [u8; 16] {
    let mut y = [0u8; 16];

    // Process blocks helper: handles zero-padding of the last block
    let mut process = |data: &[u8]| {
        for chunk in data.chunks(16) {
            let mut block = [0u8; 16];
            block[..chunk.len()].copy_from_slice(chunk);
            // Y = (Y XOR block) * H
            for j in 0..16 {
                block[j] ^= y[j];
            }
            y = gf128_mul(&block, h);
        }
    };

    if !aad.is_empty() {
        process(aad);
    }
    if !ciphertext.is_empty() {
        process(ciphertext);
    }

    // Length block: [len(AAD)*8 as u64 BE || len(CT)*8 as u64 BE]
    let mut len_block = [0u8; 16];
    let aad_bits = (aad.len() as u64) * 8;
    let ct_bits = (ciphertext.len() as u64) * 8;
    len_block[..8].copy_from_slice(&aad_bits.to_be_bytes());
    len_block[8..16].copy_from_slice(&ct_bits.to_be_bytes());

    for j in 0..16 {
        len_block[j] ^= y[j];
    }
    gf128_mul(&len_block, h)
}

/// Increment the 32-bit counter in the last 4 bytes of a 16-byte block (BE).
fn increment_counter(block: &[u8; 16]) -> [u8; 16] {
    let mut result = *block;
    for i in (12..16).rev() {
        result[i] = result[i].wrapping_add(1);
        if result[i] != 0 {
            break;
        }
    }
    result
}

/// Encrypt with AES-GCM. Returns (ciphertext, 16-byte tag).
///
/// GCM provides authenticated encryption with associated data (AEAD):
/// - The plaintext is encrypted (confidentiality)
/// - Both ciphertext AND AAD are authenticated (integrity)
/// - AAD is authenticated but NOT encrypted (useful for headers/metadata)
pub fn gcm_encrypt(
    plaintext: &[u8],
    key: &[u8],
    iv: &[u8],
    aad: &[u8],
) -> Result<(Vec<u8>, [u8; 16]), String> {
    if iv.len() != 12 {
        return Err(format!("GCM IV must be 12 bytes, got {}", iv.len()));
    }

    // Step 1: H = AES_encrypt(0^128, key)
    let zero_block = [0u8; 16];
    let h = encrypt_block(&zero_block, key)?;

    // Step 2: J0 = IV || 0x00000001
    let mut j0 = [0u8; 16];
    j0[..12].copy_from_slice(iv);
    j0[15] = 1;

    // Step 3: CTR-encrypt starting at J0+1
    let mut ciphertext = Vec::with_capacity(plaintext.len());
    let mut counter = j0;
    for chunk in plaintext.chunks(BLOCK_SIZE) {
        counter = increment_counter(&counter);
        let keystream = encrypt_block(&counter, key)?;
        for (i, &byte) in chunk.iter().enumerate() {
            ciphertext.push(byte ^ keystream[i]);
        }
    }

    // Step 4: Tag = GHASH(H, AAD, CT) XOR AES_encrypt(J0, key)
    let ghash_result = ghash(&h, aad, &ciphertext);
    let enc_j0 = encrypt_block(&j0, key)?;
    let mut tag = [0u8; 16];
    for i in 0..16 {
        tag[i] = ghash_result[i] ^ enc_j0[i];
    }

    Ok((ciphertext, tag))
}

/// Decrypt with AES-GCM. Verifies the tag before returning plaintext.
///
/// Returns an error if the tag does not match (authentication failure).
/// This is critical: returning unauthenticated plaintext enables attacks.
pub fn gcm_decrypt(
    ciphertext: &[u8],
    key: &[u8],
    iv: &[u8],
    aad: &[u8],
    tag: &[u8; 16],
) -> Result<Vec<u8>, String> {
    if iv.len() != 12 {
        return Err(format!("GCM IV must be 12 bytes, got {}", iv.len()));
    }

    // Compute H and J0
    let zero_block = [0u8; 16];
    let h = encrypt_block(&zero_block, key)?;
    let mut j0 = [0u8; 16];
    j0[..12].copy_from_slice(iv);
    j0[15] = 1;

    // Verify tag BEFORE decrypting (constant-time comparison)
    let ghash_result = ghash(&h, aad, ciphertext);
    let enc_j0 = encrypt_block(&j0, key)?;
    let mut diff = 0u8;
    for i in 0..16 {
        diff |= (ghash_result[i] ^ enc_j0[i]) ^ tag[i];
    }
    if diff != 0 {
        return Err("GCM authentication failed: tag mismatch".into());
    }

    // CTR-decrypt
    let mut plaintext = Vec::with_capacity(ciphertext.len());
    let mut counter = j0;
    for chunk in ciphertext.chunks(BLOCK_SIZE) {
        counter = increment_counter(&counter);
        let keystream = encrypt_block(&counter, key)?;
        for (i, &byte) in chunk.iter().enumerate() {
            plaintext.push(byte ^ keystream[i]);
        }
    }

    Ok(plaintext)
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: decode hex string to bytes.
    fn from_hex(s: &str) -> Vec<u8> {
        hex::decode(s).unwrap()
    }

    /// Helper: encode bytes to hex string.
    fn to_hex(bytes: &[u8]) -> String {
        hex::encode(bytes)
    }

    // NIST SP 800-38A key and plaintext
    const NIST_KEY_HEX: &str = "2b7e151628aed2a6abf7158809cf4f3c";
    const NIST_PT_BLOCK1_HEX: &str = "6bc1bee22e409f96e93d7e117393172a";
    const NIST_PT_ALL_HEX: &str = "6bc1bee22e409f96e93d7e117393172aae2d8a571e03ac9c9eb76fac45af8e5130c81c46a35ce411e5fbc1191a0a52eff69f2445df4f9b17ad2b417be66c3710";

    // ── PKCS#7 ──

    #[test]
    fn test_pkcs7_pad_aligned() {
        let input = vec![0xaa; 16];
        let padded = pkcs7_pad(&input);
        assert_eq!(padded.len(), 32);
        for &b in &padded[16..] {
            assert_eq!(b, 16);
        }
    }

    #[test]
    fn test_pkcs7_pad_13_bytes() {
        let input = vec![0xbb; 13];
        let padded = pkcs7_pad(&input);
        assert_eq!(padded.len(), 16);
        assert_eq!(padded[13], 3);
        assert_eq!(padded[14], 3);
        assert_eq!(padded[15], 3);
    }

    #[test]
    fn test_pkcs7_roundtrip() {
        let input = vec![1, 2, 3, 4, 5];
        let result = pkcs7_unpad(&pkcs7_pad(&input)).unwrap();
        assert_eq!(result, input);
    }

    #[test]
    fn test_pkcs7_rejects_invalid() {
        let mut bad = vec![0u8; 16];
        bad[15] = 0;
        assert!(pkcs7_unpad(&bad).is_err());

        bad[15] = 2;
        bad[14] = 3; // inconsistent
        assert!(pkcs7_unpad(&bad).is_err());
    }

    // ── ECB ──

    #[test]
    fn test_ecb_nist_block1() {
        let key = from_hex(NIST_KEY_HEX);
        let pt = from_hex(NIST_PT_BLOCK1_HEX);
        let ct = ecb_encrypt(&pt, &key).unwrap();
        assert_eq!(to_hex(&ct[..16]), "3ad77bb40d7a3660a89ecaf32466ef97");
    }

    #[test]
    fn test_ecb_nist_all_blocks() {
        let key = from_hex(NIST_KEY_HEX);
        let pt = from_hex(NIST_PT_ALL_HEX);
        let ct = ecb_encrypt(&pt, &key).unwrap();
        assert_eq!(ct.len(), 80);
        assert_eq!(to_hex(&ct[0..16]), "3ad77bb40d7a3660a89ecaf32466ef97");
        assert_eq!(to_hex(&ct[16..32]), "f5d3d58503b9699de785895a96fdbaaf");
        assert_eq!(to_hex(&ct[32..48]), "43b1cd7f598ece23881b00e3ed030688");
        assert_eq!(to_hex(&ct[48..64]), "7b0c785e27e8ad3f8223207104725dd4");
    }

    #[test]
    fn test_ecb_roundtrip() {
        let key = from_hex(NIST_KEY_HEX);
        let pt = from_hex(NIST_PT_ALL_HEX);
        let ct = ecb_encrypt(&pt, &key).unwrap();
        let result = ecb_decrypt(&ct, &key).unwrap();
        assert_eq!(result, pt);
    }

    #[test]
    fn test_ecb_identical_blocks_produce_identical_ct() {
        let key = from_hex(NIST_KEY_HEX);
        let block = from_hex(NIST_PT_BLOCK1_HEX);
        let mut two_blocks = Vec::new();
        two_blocks.extend_from_slice(&block);
        two_blocks.extend_from_slice(&block);
        let ct = ecb_encrypt(&two_blocks, &key).unwrap();
        assert_eq!(to_hex(&ct[..16]), to_hex(&ct[16..32]));
    }

    // ── CBC ──

    #[test]
    fn test_cbc_nist_block1() {
        let key = from_hex(NIST_KEY_HEX);
        let iv = from_hex("000102030405060708090a0b0c0d0e0f");
        let pt = from_hex(NIST_PT_BLOCK1_HEX);
        let ct = cbc_encrypt(&pt, &key, &iv).unwrap();
        assert_eq!(to_hex(&ct[..16]), "7649abac8119b246cee98e9b12e9197d");
    }

    #[test]
    fn test_cbc_nist_all_blocks() {
        let key = from_hex(NIST_KEY_HEX);
        let iv = from_hex("000102030405060708090a0b0c0d0e0f");
        let pt = from_hex(NIST_PT_ALL_HEX);
        let ct = cbc_encrypt(&pt, &key, &iv).unwrap();
        assert_eq!(ct.len(), 80);
        assert_eq!(to_hex(&ct[0..16]), "7649abac8119b246cee98e9b12e9197d");
        assert_eq!(to_hex(&ct[16..32]), "5086cb9b507219ee95db113a917678b2");
        assert_eq!(to_hex(&ct[32..48]), "73bed6b8e3c1743b7116e69e22229516");
        assert_eq!(to_hex(&ct[48..64]), "3ff1caa1681fac09120eca307586e1a7");
    }

    #[test]
    fn test_cbc_roundtrip() {
        let key = from_hex(NIST_KEY_HEX);
        let iv = from_hex("000102030405060708090a0b0c0d0e0f");
        let pt = from_hex(NIST_PT_ALL_HEX);
        let ct = cbc_encrypt(&pt, &key, &iv).unwrap();
        let result = cbc_decrypt(&ct, &key, &iv).unwrap();
        assert_eq!(result, pt);
    }

    #[test]
    fn test_cbc_rejects_wrong_iv_length() {
        let key = from_hex(NIST_KEY_HEX);
        let pt = from_hex(NIST_PT_BLOCK1_HEX);
        assert!(cbc_encrypt(&pt, &key, &[0u8; 8]).is_err());
    }

    // ── CTR ──

    #[test]
    fn test_ctr_roundtrip() {
        let key = from_hex(NIST_KEY_HEX);
        let nonce = from_hex("f0f1f2f3f4f5f6f7f8f9fafb");
        let pt = from_hex(NIST_PT_ALL_HEX);
        let ct = ctr_encrypt(&pt, &key, &nonce).unwrap();
        let result = ctr_decrypt(&ct, &key, &nonce).unwrap();
        assert_eq!(result, pt);
    }

    #[test]
    fn test_ctr_no_padding() {
        let key = from_hex(NIST_KEY_HEX);
        let nonce = from_hex("000000000000000000000000");
        let pt = vec![0xde, 0xad];
        let ct = ctr_encrypt(&pt, &key, &nonce).unwrap();
        assert_eq!(ct.len(), 2);
        let result = ctr_decrypt(&ct, &key, &nonce).unwrap();
        assert_eq!(result, pt);
    }

    #[test]
    fn test_ctr_empty() {
        let key = from_hex(NIST_KEY_HEX);
        let nonce = from_hex("000000000000000000000000");
        let ct = ctr_encrypt(&[], &key, &nonce).unwrap();
        assert_eq!(ct.len(), 0);
    }

    #[test]
    fn test_ctr_rejects_wrong_nonce() {
        let key = from_hex(NIST_KEY_HEX);
        assert!(ctr_encrypt(&[0], &key, &[0u8; 16]).is_err());
    }

    // ── GCM ──

    #[test]
    fn test_gcm_nist_test_case() {
        let key = from_hex("feffe9928665731c6d6a8f9467308308");
        let iv = from_hex("cafebabefacedbaddecaf888");
        let pt = from_hex("d9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b391aafd255");
        let expected_ct = from_hex("42831ec2217774244b7221b784d0d49ce3aa212f2c02a4e035c17e2329aca12e21d514b25466931c7d8f6a5aac84aa051ba30b396a0aac973d58e091473f5985");
        let expected_tag = from_hex("4d5c2af327cd64a62cf35abd2ba6fab4");

        let (ct, tag) = gcm_encrypt(&pt, &key, &iv, &[]).unwrap();
        assert_eq!(to_hex(&ct), to_hex(&expected_ct));
        assert_eq!(to_hex(&tag), to_hex(&expected_tag));
    }

    #[test]
    fn test_gcm_roundtrip_with_aad() {
        let key = from_hex("feffe9928665731c6d6a8f9467308308");
        let iv = from_hex("cafebabefacedbaddecaf888");
        let pt = from_hex("d9313225f88406e5a55909c5aff5269a");
        let aad = from_hex("feedfacedeadbeeffeedfacedeadbeef");

        let (ct, tag) = gcm_encrypt(&pt, &key, &iv, &aad).unwrap();
        let result = gcm_decrypt(&ct, &key, &iv, &aad, &tag).unwrap();
        assert_eq!(result, pt);
    }

    #[test]
    fn test_gcm_rejects_tampered_ciphertext() {
        let key = from_hex("feffe9928665731c6d6a8f9467308308");
        let iv = from_hex("cafebabefacedbaddecaf888");
        let pt = from_hex("d9313225f88406e5a55909c5aff5269a");

        let (mut ct, tag) = gcm_encrypt(&pt, &key, &iv, &[]).unwrap();
        ct[0] ^= 0x01;
        assert!(gcm_decrypt(&ct, &key, &iv, &[], &tag).is_err());
    }

    #[test]
    fn test_gcm_rejects_tampered_tag() {
        let key = from_hex("feffe9928665731c6d6a8f9467308308");
        let iv = from_hex("cafebabefacedbaddecaf888");
        let pt = from_hex("d9313225f88406e5a55909c5aff5269a");

        let (ct, mut tag) = gcm_encrypt(&pt, &key, &iv, &[]).unwrap();
        tag[0] ^= 0x01;
        assert!(gcm_decrypt(&ct, &key, &iv, &[], &tag).is_err());
    }

    #[test]
    fn test_gcm_empty_pt_empty_aad() {
        let key = from_hex("00000000000000000000000000000000");
        let iv = from_hex("000000000000000000000000");

        let (ct, tag) = gcm_encrypt(&[], &key, &iv, &[]).unwrap();
        assert_eq!(ct.len(), 0);
        assert_eq!(to_hex(&tag), "58e2fccefa7e3061367f1d57a4e7455a");
    }

    #[test]
    fn test_gcm_rejects_wrong_iv() {
        let key = from_hex("feffe9928665731c6d6a8f9467308308");
        assert!(gcm_encrypt(&[], &key, &[0u8; 16], &[]).is_err());
    }

    #[test]
    fn test_gcm_rejects_wrong_aad() {
        let key = from_hex("feffe9928665731c6d6a8f9467308308");
        let iv = from_hex("cafebabefacedbaddecaf888");
        let pt = from_hex("d9313225f88406e5a55909c5aff5269a");
        let aad = from_hex("feedfacedeadbeef");

        let (ct, tag) = gcm_encrypt(&pt, &key, &iv, &aad).unwrap();
        let wrong_aad = from_hex("deadbeeffeedface");
        assert!(gcm_decrypt(&ct, &key, &iv, &wrong_aad, &tag).is_err());
    }
}
