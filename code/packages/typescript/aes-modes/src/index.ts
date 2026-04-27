/**
 * @coding-adventures/aes-modes
 *
 * AES Modes of Operation — ECB, CBC, CTR, GCM
 *
 * A block cipher like AES operates on fixed 128-bit (16-byte) blocks. To
 * encrypt messages of arbitrary length, you need a **mode of operation** —
 * a recipe that describes how to chain multiple block-cipher calls together.
 *
 * The choice of mode is critical for security:
 *
 *   Mode   | Security          | Properties
 *   -------|-------------------|--------------------------------------------
 *   ECB    | BROKEN            | Each block encrypted independently.
 *          |                   | Identical plaintext blocks → identical
 *          |                   | ciphertext blocks. The "ECB penguin" shows
 *          |                   | image structure leaking through encryption.
 *   -------|-------------------|--------------------------------------------
 *   CBC    | Legacy (padding   | Each block XOR'd with previous ciphertext
 *          | oracle attacks)   | before encrypting. Requires unpredictable
 *          |                   | IV. Vulnerable to POODLE / Lucky 13.
 *   -------|-------------------|--------------------------------------------
 *   CTR    | Modern, secure    | Turns block cipher into stream cipher.
 *          |                   | Encrypt a counter, XOR with plaintext.
 *          |                   | Parallelizable, no padding needed.
 *   -------|-------------------|--------------------------------------------
 *   GCM    | Modern, secure +  | CTR encryption + GHASH authentication.
 *          | authenticated     | Provides both confidentiality and integrity.
 *          |                   | The gold standard for TLS 1.3.
 *
 * This module implements all four modes educationally, wrapping the AES
 * block cipher from @coding-adventures/aes.
 *
 * Public API
 * ==========
 *   ecbEncrypt(plaintext, key) → ciphertext
 *   ecbDecrypt(ciphertext, key) → plaintext
 *   cbcEncrypt(plaintext, key, iv) → ciphertext
 *   cbcDecrypt(ciphertext, key, iv) → plaintext
 *   ctrEncrypt(plaintext, key, nonce) → ciphertext
 *   ctrDecrypt(ciphertext, key, nonce) → plaintext   (same as ctrEncrypt)
 *   gcmEncrypt(plaintext, key, iv, aad) → { ciphertext, tag }
 *   gcmDecrypt(ciphertext, key, iv, aad, tag) → plaintext
 *
 * Dependencies
 * ============
 *   @coding-adventures/aes — provides aesEncryptBlock, aesDecryptBlock
 */

import { aesEncryptBlock, aesDecryptBlock, fromHex, toHex } from "@coding-adventures/aes";

export { fromHex, toHex } from "@coding-adventures/aes";

export const VERSION = "0.1.0";

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/** AES block size in bytes. AES always operates on 128-bit = 16-byte blocks. */
const BLOCK_SIZE = 16;

// ─────────────────────────────────────────────────────────────────────────────
// PKCS#7 Padding
// ─────────────────────────────────────────────────────────────────────────────
//
// Block ciphers need input that is an exact multiple of the block size (16
// bytes for AES). PKCS#7 padding fills the gap:
//
//   - If the plaintext is N bytes short of a full block, append N copies of
//     the byte N. For example, if 3 bytes are needed: append [0x03, 0x03, 0x03].
//   - If the plaintext is already block-aligned, append a full block of 16
//     bytes, each with value 0x10. This ensures unpadding is always unambiguous.
//
// To unpad: read the last byte (call it N), verify the last N bytes all have
// value N, then strip them.
//
// Why not just use the length?  Because the receiver may not know the original
// plaintext length. PKCS#7 is self-describing — the padding bytes encode their
// own length.

/**
 * Apply PKCS#7 padding to a byte array.
 *
 * Adds 1–16 bytes so the result length is a multiple of 16.
 * Each padding byte has the value equal to the number of padding bytes added.
 */
export function pkcs7Pad(data: Uint8Array): Uint8Array {
  const padLen = BLOCK_SIZE - (data.length % BLOCK_SIZE);
  const result = new Uint8Array(data.length + padLen);
  result.set(data);
  for (let i = data.length; i < result.length; i++) {
    result[i] = padLen;
  }
  return result;
}

/**
 * Remove PKCS#7 padding from a byte array.
 *
 * Reads the last byte to determine how many padding bytes to strip.
 * Validates that all padding bytes have the correct value.
 *
 * @throws Error if the padding is invalid (corrupted ciphertext or wrong key).
 */
export function pkcs7Unpad(data: Uint8Array): Uint8Array {
  if (data.length === 0 || data.length % BLOCK_SIZE !== 0) {
    throw new Error("Invalid padded data: length must be a positive multiple of 16");
  }
  const padLen = data[data.length - 1];
  if (padLen < 1 || padLen > BLOCK_SIZE) {
    throw new Error("Invalid PKCS#7 padding");
  }
  // Constant-time padding validation: accumulate differences with OR
  // so the loop always takes the same time regardless of which byte fails.
  let diff = 0;
  for (let i = data.length - padLen; i < data.length; i++) {
    diff |= data[i] ^ padLen;
  }
  if (diff !== 0) {
    throw new Error("Invalid PKCS#7 padding");
  }
  return data.slice(0, data.length - padLen);
}

// ─────────────────────────────────────────────────────────────────────────────
// XOR helper
// ─────────────────────────────────────────────────────────────────────────────

/**
 * XOR two byte arrays of equal length, returning a new array.
 *
 * XOR is the fundamental building block of symmetric cryptography:
 *   - A XOR B XOR B = A  (self-inverse: encrypt = XOR, decrypt = XOR again)
 *   - Uniformly random when either operand is uniformly random
 *   - Bitwise independent (no carries like addition)
 */
function xorBytes(a: Uint8Array, b: Uint8Array): Uint8Array {
  const result = new Uint8Array(a.length);
  for (let i = 0; i < a.length; i++) {
    result[i] = a[i] ^ b[i];
  }
  return result;
}

// ─────────────────────────────────────────────────────────────────────────────
// ECB — Electronic Codebook Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// The simplest mode: encrypt each 16-byte block independently.
//
//   C[i] = AES_encrypt(P[i], key)
//   P[i] = AES_decrypt(C[i], key)
//
// *** ECB IS INSECURE FOR REAL USE ***
//
// Problem: identical plaintext blocks produce identical ciphertext blocks.
// This means patterns in the plaintext are visible in the ciphertext. The
// famous "ECB penguin" demonstrates this — encrypting a bitmap image in
// ECB mode reveals the image structure because regions of the same color
// encrypt to the same ciphertext.
//
// ECB is included here for educational purposes only.

/**
 * Encrypt with AES in ECB mode (INSECURE — educational only).
 *
 * Applies PKCS#7 padding, then encrypts each 16-byte block independently.
 *
 * @param plaintext - arbitrary-length plaintext
 * @param key - 16, 24, or 32 bytes (AES-128, AES-192, AES-256)
 * @returns ciphertext (always a multiple of 16 bytes)
 */
export function ecbEncrypt(plaintext: Uint8Array, key: Uint8Array): Uint8Array {
  const padded = pkcs7Pad(plaintext);
  const result = new Uint8Array(padded.length);

  // Process each block independently — this is exactly what makes ECB weak
  for (let i = 0; i < padded.length; i += BLOCK_SIZE) {
    const block = padded.slice(i, i + BLOCK_SIZE);
    const encrypted = aesEncryptBlock(block, key);
    result.set(encrypted, i);
  }
  return result;
}

/**
 * Decrypt with AES in ECB mode (INSECURE — educational only).
 *
 * Decrypts each 16-byte block independently, then removes PKCS#7 padding.
 *
 * @param ciphertext - must be a multiple of 16 bytes
 * @param key - same key used for encryption
 * @returns original plaintext
 */
export function ecbDecrypt(ciphertext: Uint8Array, key: Uint8Array): Uint8Array {
  if (ciphertext.length === 0 || ciphertext.length % BLOCK_SIZE !== 0) {
    throw new Error("ECB ciphertext must be a non-empty multiple of 16 bytes");
  }
  const result = new Uint8Array(ciphertext.length);

  for (let i = 0; i < ciphertext.length; i += BLOCK_SIZE) {
    const block = ciphertext.slice(i, i + BLOCK_SIZE);
    const decrypted = aesDecryptBlock(block, key);
    result.set(decrypted, i);
  }
  return pkcs7Unpad(result);
}

// ─────────────────────────────────────────────────────────────────────────────
// CBC — Cipher Block Chaining Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// CBC chains blocks together so that identical plaintext blocks produce
// different ciphertext (as long as the IV differs or the position differs):
//
//   Encryption:
//     C[0] = AES_encrypt(P[0] XOR IV, key)
//     C[i] = AES_encrypt(P[i] XOR C[i-1], key)   for i > 0
//
//   Decryption:
//     P[0] = AES_decrypt(C[0], key) XOR IV
//     P[i] = AES_decrypt(C[i], key) XOR C[i-1]   for i > 0
//
// The IV (Initialization Vector) must be:
//   - 16 bytes (one AES block)
//   - Unpredictable (not just unique — must be random)
//   - Never reused with the same key
//
// CBC was the workhorse of SSL/TLS for years but is vulnerable to padding
// oracle attacks (POODLE, Lucky 13) where an attacker can deduce plaintext
// bytes by observing whether the server reports a padding error vs. a
// different error.

/**
 * Encrypt with AES in CBC mode.
 *
 * @param plaintext - arbitrary-length plaintext
 * @param key - 16, 24, or 32 bytes
 * @param iv - exactly 16 bytes, must be unpredictable
 * @returns ciphertext (multiple of 16 bytes)
 */
export function cbcEncrypt(plaintext: Uint8Array, key: Uint8Array, iv: Uint8Array): Uint8Array {
  if (iv.length !== BLOCK_SIZE) {
    throw new Error(`CBC IV must be 16 bytes, got ${iv.length}`);
  }
  const padded = pkcs7Pad(plaintext);
  const result = new Uint8Array(padded.length);
  let prev = iv;

  for (let i = 0; i < padded.length; i += BLOCK_SIZE) {
    const block = padded.slice(i, i + BLOCK_SIZE);
    // XOR with previous ciphertext (or IV for first block)
    const xored = xorBytes(block, prev);
    const encrypted = aesEncryptBlock(xored, key);
    result.set(encrypted, i);
    prev = encrypted;
  }
  return result;
}

/**
 * Decrypt with AES in CBC mode.
 *
 * @param ciphertext - must be a non-empty multiple of 16 bytes
 * @param key - same key used for encryption
 * @param iv - same IV used for encryption (16 bytes)
 * @returns original plaintext
 */
export function cbcDecrypt(ciphertext: Uint8Array, key: Uint8Array, iv: Uint8Array): Uint8Array {
  if (iv.length !== BLOCK_SIZE) {
    throw new Error(`CBC IV must be 16 bytes, got ${iv.length}`);
  }
  if (ciphertext.length === 0 || ciphertext.length % BLOCK_SIZE !== 0) {
    throw new Error("CBC ciphertext must be a non-empty multiple of 16 bytes");
  }
  const result = new Uint8Array(ciphertext.length);
  let prev = iv;

  for (let i = 0; i < ciphertext.length; i += BLOCK_SIZE) {
    const block = ciphertext.slice(i, i + BLOCK_SIZE);
    const decrypted = aesDecryptBlock(block, key);
    // XOR with previous ciphertext (or IV) to recover plaintext
    const plain = xorBytes(decrypted, prev);
    result.set(plain, i);
    prev = block;
  }
  return pkcs7Unpad(result);
}

// ─────────────────────────────────────────────────────────────────────────────
// CTR — Counter Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// CTR mode turns a block cipher into a stream cipher. Instead of encrypting
// the plaintext directly, we encrypt a counter and XOR the result with the
// plaintext:
//
//   keystream[i] = AES_encrypt(nonce || counter_i, key)
//   C[i] = P[i] XOR keystream[i]
//
// The nonce is 12 bytes (96 bits), and the counter is a 4-byte big-endian
// integer starting at 1. This gives us 2^32 blocks = 64 GB per nonce before
// the counter wraps — plenty for most uses.
//
// Advantages:
//   - No padding needed (last block can be partial)
//   - Parallelizable (any block can be computed independently)
//   - Random access (can decrypt block i without decrypting 0..i-1)
//   - Encryption = decryption (both just XOR with the keystream)
//
// Critical requirement: NEVER reuse a nonce with the same key. If you do,
//   C1 XOR C2 = P1 XOR P2  (the keystreams cancel out)
// and an attacker can recover both plaintexts using frequency analysis.

/**
 * Build a 16-byte CTR counter block from a 12-byte nonce and a 4-byte
 * big-endian counter value.
 *
 * Layout: [nonce (12 bytes)] [counter (4 bytes, big-endian)]
 */
function buildCounterBlock(nonce: Uint8Array, counter: number): Uint8Array {
  const block = new Uint8Array(BLOCK_SIZE);
  block.set(nonce);
  // Write counter as 4-byte big-endian in the last 4 bytes
  block[12] = (counter >>> 24) & 0xff;
  block[13] = (counter >>> 16) & 0xff;
  block[14] = (counter >>> 8) & 0xff;
  block[15] = counter & 0xff;
  return block;
}

/**
 * Encrypt with AES in CTR mode.
 *
 * @param plaintext - arbitrary-length plaintext (no padding needed)
 * @param key - 16, 24, or 32 bytes
 * @param nonce - exactly 12 bytes, must be unique per message
 * @returns ciphertext (same length as plaintext)
 */
export function ctrEncrypt(plaintext: Uint8Array, key: Uint8Array, nonce: Uint8Array): Uint8Array {
  if (nonce.length !== 12) {
    throw new Error(`CTR nonce must be 12 bytes, got ${nonce.length}`);
  }
  const result = new Uint8Array(plaintext.length);
  let counter = 1; // Counter starts at 1 (0 is reserved in GCM for the tag)

  for (let i = 0; i < plaintext.length; i += BLOCK_SIZE) {
    // Encrypt the counter block to produce keystream
    const counterBlock = buildCounterBlock(nonce, counter);
    const keystream = aesEncryptBlock(counterBlock, key);

    // XOR plaintext with keystream (handle last partial block)
    const remaining = Math.min(BLOCK_SIZE, plaintext.length - i);
    for (let j = 0; j < remaining; j++) {
      result[i + j] = plaintext[i + j] ^ keystream[j];
    }
    counter++;
  }
  return result;
}

/**
 * Decrypt with AES in CTR mode.
 *
 * CTR decryption is identical to encryption — both just XOR with the
 * keystream generated from the counter. This is a beautiful property of
 * stream ciphers.
 *
 * @param ciphertext - arbitrary-length ciphertext
 * @param key - same key used for encryption
 * @param nonce - same nonce used for encryption (12 bytes)
 * @returns original plaintext
 */
export function ctrDecrypt(ciphertext: Uint8Array, key: Uint8Array, nonce: Uint8Array): Uint8Array {
  // CTR decryption is the same operation as encryption
  return ctrEncrypt(ciphertext, key, nonce);
}

// ─────────────────────────────────────────────────────────────────────────────
// GCM — Galois/Counter Mode
// ─────────────────────────────────────────────────────────────────────────────
//
// GCM combines CTR-mode encryption with a Galois-field authentication tag.
// It provides both confidentiality (the data is encrypted) and integrity
// (any tampering is detected via the authentication tag).
//
// GCM is the gold standard for authenticated encryption in TLS 1.3, IPsec,
// and most modern protocols. It is specified in NIST SP 800-38D.
//
// Architecture:
//
//   1. Derive H = AES_encrypt(0^128, key)    — the hash subkey
//   2. Build J0 = IV || 0x00000001            — initial counter block
//   3. CTR-encrypt plaintext starting at J0+1
//   4. Compute GHASH over AAD and ciphertext
//   5. Tag = GHASH_result XOR AES_encrypt(J0, key)
//
// The GHASH function is a polynomial evaluation in GF(2^128):
//   GHASH(H, X) = X[1]·H^n XOR X[2]·H^(n-1) XOR ... XOR X[n]·H^1
//
// This can be computed incrementally:
//   Y[0] = 0^128
//   Y[i] = (Y[i-1] XOR X[i]) · H     in GF(2^128)
//
// The GF(2^128) multiplication uses the reducing polynomial:
//   R = x^128 + x^7 + x^2 + x + 1
// Represented as R = 0xE1 << 120 (the high bit of the first byte is 0xE1).

/**
 * Multiply two 128-bit values in GF(2^128).
 *
 * The reducing polynomial is R = x^128 + x^7 + x^2 + x + 1.
 * In the "bit reflection" convention used by GCM, R's high byte is 0xE1.
 *
 * Algorithm (NIST SP 800-38D, Algorithm 1):
 *   Z = 0
 *   V = Y
 *   for i = 0 to 127:
 *     if bit i of X is set:
 *       Z = Z XOR V
 *     if LSB of V is set:
 *       V = (V >> 1) XOR R
 *     else:
 *       V = V >> 1
 *
 * Here "bit 0" is the MSB of byte 0 (big-endian bit numbering), and
 * ">> 1" shifts the entire 128-bit value right by 1 bit.
 */
function gf128Mul(x: Uint8Array, y: Uint8Array): Uint8Array {
  const z = new Uint8Array(16);
  const v = new Uint8Array(16);
  v.set(y);

  for (let i = 0; i < 128; i++) {
    // Check bit i of X (MSB-first: bit 0 is the MSB of byte 0)
    const byteIndex = Math.floor(i / 8);
    const bitIndex = 7 - (i % 8);
    if ((x[byteIndex] >> bitIndex) & 1) {
      // Z = Z XOR V
      for (let j = 0; j < 16; j++) z[j] ^= v[j];
    }

    // Check if LSB of V is set (bit 127 = LSB of byte 15)
    const carry = v[15] & 1;

    // Right-shift V by 1 bit
    for (let j = 15; j > 0; j--) {
      v[j] = (v[j] >>> 1) | ((v[j - 1] & 1) << 7);
    }
    v[0] = v[0] >>> 1;

    // If carry (old LSB was 1): XOR with R = 0xE1000000...
    if (carry) {
      v[0] ^= 0xe1;
    }
  }
  return z;
}

/**
 * GHASH: universal hash function used in GCM.
 *
 * GHASH processes the authenticated data (AAD) and ciphertext, producing
 * a 128-bit hash that is combined with the encrypted counter to form the
 * authentication tag.
 *
 * The input blocks are:
 *   1. AAD blocks (zero-padded to 16 bytes)
 *   2. Ciphertext blocks (zero-padded to 16 bytes)
 *   3. A length block: [len(AAD) in bits || len(CT) in bits] as two
 *      64-bit big-endian integers
 *
 * The hash is computed incrementally:
 *   Y[0] = 0^128
 *   Y[i] = (Y[i-1] XOR X[i]) * H  in GF(2^128)
 *
 * @param h - hash subkey (AES_encrypt(0^128, key))
 * @param aad - additional authenticated data
 * @param ciphertext - encrypted data
 * @returns 16-byte GHASH result
 */
function ghash(h: Uint8Array, aad: Uint8Array, ciphertext: Uint8Array): Uint8Array {
  let y = new Uint8Array(16);

  // Helper: process a data buffer in 16-byte blocks (zero-pad the last block)
  function processBlocks(data: Uint8Array): void {
    for (let i = 0; i < data.length; i += 16) {
      const block = new Uint8Array(16);
      const remaining = Math.min(16, data.length - i);
      for (let j = 0; j < remaining; j++) {
        block[j] = data[i + j];
      }
      // Y = (Y XOR block) * H
      const xored = xorBytes(y, block);
      y = gf128Mul(xored, h);
    }
  }

  // Process AAD blocks
  if (aad.length > 0) {
    processBlocks(aad);
  }

  // Process ciphertext blocks
  if (ciphertext.length > 0) {
    processBlocks(ciphertext);
  }

  // Process length block: [len(AAD)*8 as u64_be || len(CT)*8 as u64_be]
  // Lengths are in BITS, encoded as 64-bit big-endian integers
  const lenBlock = new Uint8Array(16);
  const aadBits = aad.length * 8;
  const ctBits = ciphertext.length * 8;
  // AAD length in bits (big-endian u64, bytes 0-7)
  // JavaScript numbers can represent integers up to 2^53 safely
  lenBlock[4] = (aadBits >>> 24) & 0xff;
  lenBlock[5] = (aadBits >>> 16) & 0xff;
  lenBlock[6] = (aadBits >>> 8) & 0xff;
  lenBlock[7] = aadBits & 0xff;
  // Ciphertext length in bits (big-endian u64, bytes 8-15)
  lenBlock[12] = (ctBits >>> 24) & 0xff;
  lenBlock[13] = (ctBits >>> 16) & 0xff;
  lenBlock[14] = (ctBits >>> 8) & 0xff;
  lenBlock[15] = ctBits & 0xff;

  y = gf128Mul(xorBytes(y, lenBlock), h);

  return y;
}

/**
 * Increment a 32-bit counter stored in the last 4 bytes of a 16-byte block
 * (big-endian). The first 12 bytes (the nonce/IV portion) are unchanged.
 *
 * This is the inc32() function from NIST SP 800-38D.
 */
function incrementCounter(block: Uint8Array): Uint8Array {
  const result = new Uint8Array(16);
  result.set(block);
  // Increment the last 4 bytes as a big-endian 32-bit integer
  for (let i = 15; i >= 12; i--) {
    result[i] = (result[i] + 1) & 0xff;
    if (result[i] !== 0) break; // No carry, done
  }
  return result;
}

/**
 * Encrypt with AES-GCM (Galois/Counter Mode).
 *
 * GCM provides authenticated encryption with associated data (AEAD):
 *   - The plaintext is encrypted (confidentiality)
 *   - Both the ciphertext AND the AAD are authenticated (integrity)
 *   - The AAD is authenticated but NOT encrypted (useful for headers)
 *
 * @param plaintext - arbitrary-length data to encrypt
 * @param key - 16, 24, or 32 bytes
 * @param iv - 12 bytes (96 bits), must be unique per message
 * @param aad - additional authenticated data (not encrypted, but authenticated)
 * @returns { ciphertext, tag } where tag is 16 bytes
 */
export function gcmEncrypt(
  plaintext: Uint8Array,
  key: Uint8Array,
  iv: Uint8Array,
  aad: Uint8Array = new Uint8Array(0),
): { ciphertext: Uint8Array; tag: Uint8Array } {
  if (iv.length !== 12) {
    throw new Error(`GCM IV must be 12 bytes, got ${iv.length}`);
  }

  // Step 1: Compute hash subkey H = AES_encrypt(0^128, key)
  // This key is used in all GHASH computations for this encryption
  const zeroBlock = new Uint8Array(16);
  const h = aesEncryptBlock(zeroBlock, key);

  // Step 2: Build initial counter J0 = IV || 0x00000001
  // J0 is used to encrypt the authentication tag (not the plaintext)
  const j0 = new Uint8Array(16);
  j0.set(iv);
  j0[15] = 1;

  // Step 3: CTR-encrypt the plaintext starting at J0+1
  // Each block of plaintext is XOR'd with AES_encrypt(counter, key)
  const ciphertext = new Uint8Array(plaintext.length);
  let counter = j0;

  for (let i = 0; i < plaintext.length; i += BLOCK_SIZE) {
    counter = incrementCounter(counter);
    const keystream = aesEncryptBlock(counter, key);
    const remaining = Math.min(BLOCK_SIZE, plaintext.length - i);
    for (let j = 0; j < remaining; j++) {
      ciphertext[i + j] = plaintext[i + j] ^ keystream[j];
    }
  }

  // Step 4: Compute GHASH over AAD and ciphertext
  const ghashResult = ghash(h, aad, ciphertext);

  // Step 5: Tag = GHASH_result XOR AES_encrypt(J0, key)
  // Encrypting J0 ensures the tag depends on the key (GHASH alone uses H,
  // which is derived from the key, but the final XOR adds another layer)
  const encJ0 = aesEncryptBlock(j0, key);
  const tag = xorBytes(ghashResult, encJ0);

  return { ciphertext, tag };
}

/**
 * Decrypt with AES-GCM (Galois/Counter Mode).
 *
 * Verifies the authentication tag before returning the plaintext. If the
 * tag does not match (indicating tampering or a wrong key), an error is
 * thrown and no plaintext is returned.
 *
 * This is critical for security: returning unauthenticated plaintext
 * enables attacks like the Forbidden Attack on GCM nonce reuse.
 *
 * @param ciphertext - encrypted data
 * @param key - same key used for encryption
 * @param iv - same IV used for encryption (12 bytes)
 * @param aad - same AAD used for encryption
 * @param tag - 16-byte authentication tag from encryption
 * @returns original plaintext
 * @throws Error if the tag does not match (authentication failure)
 */
export function gcmDecrypt(
  ciphertext: Uint8Array,
  key: Uint8Array,
  iv: Uint8Array,
  aad: Uint8Array,
  tag: Uint8Array,
): Uint8Array {
  if (iv.length !== 12) {
    throw new Error(`GCM IV must be 12 bytes, got ${iv.length}`);
  }
  if (tag.length !== 16) {
    throw new Error(`GCM tag must be 16 bytes, got ${tag.length}`);
  }

  // Step 1: Compute H and J0 (same as encryption)
  const zeroBlock = new Uint8Array(16);
  const h = aesEncryptBlock(zeroBlock, key);
  const j0 = new Uint8Array(16);
  j0.set(iv);
  j0[15] = 1;

  // Step 2: Verify the tag BEFORE decrypting
  // This prevents releasing unauthenticated plaintext
  const ghashResult = ghash(h, aad, ciphertext);
  const encJ0 = aesEncryptBlock(j0, key);
  const expectedTag = xorBytes(ghashResult, encJ0);

  // Constant-time comparison to prevent timing attacks
  let diff = 0;
  for (let i = 0; i < 16; i++) {
    diff |= expectedTag[i] ^ tag[i];
  }
  if (diff !== 0) {
    throw new Error("GCM authentication failed: tag mismatch");
  }

  // Step 3: CTR-decrypt (same as CTR-encrypt with counter starting at J0+1)
  const plaintext = new Uint8Array(ciphertext.length);
  let counter = j0;

  for (let i = 0; i < ciphertext.length; i += BLOCK_SIZE) {
    counter = incrementCounter(counter);
    const keystream = aesEncryptBlock(counter, key);
    const remaining = Math.min(BLOCK_SIZE, ciphertext.length - i);
    for (let j = 0; j < remaining; j++) {
      plaintext[i + j] = ciphertext[i + j] ^ keystream[j];
    }
  }

  return plaintext;
}
