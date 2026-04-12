/**
 * @coding-adventures/des
 *
 * DES (Data Encryption Standard) and 3DES block cipher — FIPS 46-3 / SP 800-67.
 *
 * History and Significance
 * ========================
 * DES was published by NIST in 1977 as the world's first openly standardized
 * encryption algorithm. Designed by IBM and hardened by the NSA, it uses a
 * 56-bit key (the 8 parity bits in each 64-bit key byte are ignored). A 56-bit
 * key is completely broken by modern hardware — exhaustive search takes under
 * 24 hours on consumer GPUs. NIST deprecated DES in 2005 and withdrew it in 2023.
 *
 * Despite being broken, DES is foundational for understanding:
 *   1. Feistel networks — the structural pattern shared by Blowfish, Twofish, etc.
 *   2. S-boxes — the non-linear heart that resists differential/linear cryptanalysis.
 *   3. Key schedules — how one key expands into many round keys.
 *   4. 3DES — the backward-compatible extension that delayed DES retirement by 20 years.
 *
 * Architecture
 * ============
 *
 *   plaintext (8 bytes)
 *        │
 *   IP (initial permutation)        ← hardware alignment artifact from the 1970s
 *        │
 *   ┌── 16 Feistel rounds ───────────────────────────────────────────────────┐
 *   │   L_i = R_{i-1}                                                        │
 *   │   R_i = L_{i-1} XOR f(R_{i-1}, K_i)                                   │
 *   │                                                                        │
 *   │   f(R, K):                                                             │
 *   │     E(R)  — expand 32 → 48 bits (border bits copied to adjacent group) │
 *   │     XOR K — mix in the 48-bit subkey                                   │
 *   │     S     — 8 × (6 bits → 4 bits) non-linear substitution             │
 *   │     P     — 32-bit permutation (diffusion)                             │
 *   └────────────────────────────────────────────────────────────────────────┘
 *        │
 *   FP (final permutation = IP⁻¹)
 *        │
 *   ciphertext (8 bytes)
 *
 * Decryption is identical — apply the 16 subkeys in reverse order (K16..K1).
 * The Feistel structure means the round function never needs to be inverted.
 *
 * JavaScript Bit-Width Note
 * =========================
 * JavaScript's bitwise operators (|, &, ^, ~, <<, >>) return signed 32-bit integers.
 * To work with unsigned 32-bit values, append `>>> 0` after any bitwise operation.
 * IMPORTANT: `x >> 32` in JavaScript is the same as `x >> 0` because JS shift
 * operators only use the low 5 bits of the shift count. Never shift by 32.
 *
 * This implementation works with bit arrays (number[] of 0s and 1s) to stay
 * close to the DES specification and make the algorithm easy to follow.
 *
 * Public API
 * ==========
 *   expandKey(key)                      → 16 subkeys (each Uint8Array of 6 bytes)
 *   desEncryptBlock(block, key)         → Uint8Array (8 bytes)
 *   desDecryptBlock(block, key)         → Uint8Array (8 bytes)
 *   desEcbEncrypt(plaintext, key)       → Uint8Array (PKCS#7 padded)
 *   desEcbDecrypt(ciphertext, key)      → Uint8Array
 *   tdeaEncryptBlock(block, k1, k2, k3) → Uint8Array (8 bytes)
 *   tdeaDecryptBlock(block, k1, k2, k3) → Uint8Array (8 bytes)
 */

export const VERSION = "0.1.0";

// ─────────────────────────────────────────────────────────────────────────────
// Permutation and selection tables
//
// All DES tables are 1-indexed in the standard. We store them as-is (1-indexed)
// and subtract 1 when indexing. This preserves the standard numbering so you
// can verify them against FIPS 46-3 directly.
// ─────────────────────────────────────────────────────────────────────────────

// IP — Initial Permutation
// Bit 58 of input becomes bit 1 of output, etc.
// Designed for efficient loading on the 8-bit parallel buses of the 1970s —
// has no cryptographic significance.
const _IP: readonly number[] = [
  58, 50, 42, 34, 26, 18, 10,  2,
  60, 52, 44, 36, 28, 20, 12,  4,
  62, 54, 46, 38, 30, 22, 14,  6,
  64, 56, 48, 40, 32, 24, 16,  8,
  57, 49, 41, 33, 25, 17,  9,  1,
  59, 51, 43, 35, 27, 19, 11,  3,
  61, 53, 45, 37, 29, 21, 13,  5,
  63, 55, 47, 39, 31, 23, 15,  7,
];

// FP — Final Permutation (IP⁻¹)
// Undoes the initial permutation. FP[IP[i]-1] = i+1.
const _FP: readonly number[] = [
  40,  8, 48, 16, 56, 24, 64, 32,
  39,  7, 47, 15, 55, 23, 63, 31,
  38,  6, 46, 14, 54, 22, 62, 30,
  37,  5, 45, 13, 53, 21, 61, 29,
  36,  4, 44, 12, 52, 20, 60, 28,
  35,  3, 43, 11, 51, 19, 59, 27,
  34,  2, 42, 10, 50, 18, 58, 26,
  33,  1, 41,  9, 49, 17, 57, 25,
];

// PC-1 — Permuted Choice 1
// Drops the 8 parity bits (positions 8,16,24,32,40,48,56,64) and reorders
// the remaining 56 bits into two 28-bit halves C and D.
const _PC1: readonly number[] = [
  57, 49, 41, 33, 25, 17,  9,
   1, 58, 50, 42, 34, 26, 18,
  10,  2, 59, 51, 43, 35, 27,
  19, 11,  3, 60, 52, 44, 36,
  63, 55, 47, 39, 31, 23, 15,
   7, 62, 54, 46, 38, 30, 22,
  14,  6, 61, 53, 45, 37, 29,
  21, 13,  5, 28, 20, 12,  4,
];

// PC-2 — Permuted Choice 2
// Selects 48 of the 56 key bits to form each round subkey.
// The 8 dropped positions act as a compression step.
const _PC2: readonly number[] = [
  14, 17, 11, 24,  1,  5,
   3, 28, 15,  6, 21, 10,
  23, 19, 12,  4, 26,  8,
  16,  7, 27, 20, 13,  2,
  41, 52, 31, 37, 47, 55,
  30, 40, 51, 45, 33, 48,
  44, 49, 39, 56, 34, 53,
  46, 42, 50, 36, 29, 32,
];

// E — Expansion permutation
// Expands the 32-bit right half to 48 bits by sharing border bits between
// adjacent 6-bit groups. This lets the 48-bit subkey mix into every bit.
const _E: readonly number[] = [
  32,  1,  2,  3,  4,  5,
   4,  5,  6,  7,  8,  9,
   8,  9, 10, 11, 12, 13,
  12, 13, 14, 15, 16, 17,
  16, 17, 18, 19, 20, 21,
  20, 21, 22, 23, 24, 25,
  24, 25, 26, 27, 28, 29,
  28, 29, 30, 31, 32,  1,
];

// P — Post-S-box permutation
// Disperses the 32-bit S-box output so that each round affects every bit of
// the next round's input (diffusion).
const _P: readonly number[] = [
  16,  7, 20, 21, 29, 12, 28, 17,
   1, 15, 23, 26,  5, 18, 31, 10,
   2,  8, 24, 14, 32, 27,  3,  9,
  19, 13, 30,  6, 22, 11,  4, 25,
];

// Left-rotation amounts for the key schedule halves C and D.
// Rounds 1, 2, 9, 16 rotate by 1; all others by 2.
// Total over 16 rounds = 28, completing one full cycle of the 28-bit register.
const _SHIFTS: readonly number[] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];

// ─────────────────────────────────────────────────────────────────────────────
// S-Boxes: the core non-linearity of DES
//
// Eight 6-bit → 4-bit substitution boxes. Without S-boxes, DES would be a
// linear transformation solvable with Gaussian elimination. The NSA redesigned
// IBM's originals — in 1990 Biham and Shamir proved they resist differential
// cryptanalysis, a technique the NSA had classified since 1974.
//
// Reading an S-box with 6 input bits b₁b₂b₃b₄b₅b₆:
//   row = 2·b₁ + b₆           (outer bits, range 0–3)
//   col = 8·b₂ + 4·b₃ + 2·b₄ + b₅  (inner bits, range 0–15)
//   output = SBOX[box][row][col]
// ─────────────────────────────────────────────────────────────────────────────

const _SBOXES: readonly (readonly (readonly number[])[])[] = [
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

/**
 * Convert a Uint8Array to an array of bits (MSB first within each byte).
 *
 * Example: [0b10110010] → [1,0,1,1,0,0,1,0]
 */
function bytesToBits(data: Uint8Array): number[] {
  const bits: number[] = [];
  for (const byte of data) {
    for (let i = 7; i >= 0; i--) {
      bits.push((byte >> i) & 1);
    }
  }
  return bits;
}

/**
 * Convert an array of bits (MSB first) back to a Uint8Array.
 *
 * The bit array length must be a multiple of 8.
 */
function bitsToBytes(bits: number[]): Uint8Array {
  const result = new Uint8Array(bits.length / 8);
  for (let i = 0; i < result.length; i++) {
    let byte = 0;
    for (let j = 0; j < 8; j++) {
      byte = (byte << 1) | bits[i * 8 + j];
    }
    result[i] = byte;
  }
  return result;
}

/**
 * Apply a permutation table (1-indexed positions) to a bit array.
 *
 * The output length equals the number of entries in `table`.
 * Output bit i = input bit table[i]-1.
 */
function permute(bits: number[], table: readonly number[]): number[] {
  return table.map((pos) => bits[pos - 1]);
}

/**
 * Left-rotate a 28-bit array by n positions.
 *
 * The key schedule uses 28-bit halves (C and D), each rotated independently.
 * Bits that fall off the left end wrap around to the right.
 */
function leftRotate28(half: number[], n: number): number[] {
  return [...half.slice(n), ...half.slice(0, n)];
}

// ─────────────────────────────────────────────────────────────────────────────
// Key schedule: expandKey
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Derive the 16 DES round subkeys from an 8-byte key.
 *
 * The DES key is 64 bits but only 56 are key material — bits at positions
 * 8, 16, 24, 32, 40, 48, 56, 64 are parity bits, dropped by PC-1.
 *
 * Key schedule algorithm:
 *   1. PC-1: 64 bits → 56 bits (drop parity), split into C₀ (28) and D₀ (28)
 *   2. For each round i = 1..16:
 *        C_i = LeftRotate(C_{i-1}, SHIFTS[i])
 *        D_i = LeftRotate(D_{i-1}, SHIFTS[i])
 *        K_i = PC-2(C_i ∥ D_i)   (56 → 48 bits, 6 bytes)
 *
 * @param key - exactly 8 bytes
 * @returns array of 16 subkeys, each a 6-byte Uint8Array (48 bits)
 * @throws Error if key is not exactly 8 bytes
 */
export function expandKey(key: Uint8Array): Uint8Array[] {
  if (key.length !== 8) {
    throw new Error(`DES key must be exactly 8 bytes, got ${key.length}`);
  }

  const keyBits = bytesToBits(key);
  const permuted = permute(keyBits, _PC1);    // 64 → 56 bits
  let c = permuted.slice(0, 28);
  let d = permuted.slice(28);

  const subkeys: Uint8Array[] = [];
  for (const shift of _SHIFTS) {
    c = leftRotate28(c, shift);
    d = leftRotate28(d, shift);
    const subkeyBits = permute([...c, ...d], _PC2);  // 56 → 48 bits
    subkeys.push(bitsToBytes(subkeyBits));
  }
  return subkeys;
}

// ─────────────────────────────────────────────────────────────────────────────
// Round function f(R, K)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * DES round function f(R, K).
 *
 * This is the heart of each Feistel round:
 *   1. E(R)  — expand 32-bit right half to 48 bits
 *   2. XOR K — mix in the 48-bit round subkey
 *   3. S     — 8 S-boxes (6 bits each → 4 bits) = 32 bits total
 *   4. P     — final 32-bit permutation
 *
 * The S-boxes are the ONLY non-linear step. Without them, DES reduces to
 * a linear system over GF(2) solvable with Gaussian elimination.
 */
function feistelF(right: number[], subkey: Uint8Array): number[] {
  // Step 1: Expand R from 32 → 48 bits
  const expanded = permute(right, _E);

  // Step 2: XOR with subkey (48 bits)
  const subkeyBits = bytesToBits(subkey);
  const xored = expanded.map((bit, i) => bit ^ subkeyBits[i]);

  // Step 3: Apply 8 S-boxes (each 6-bit input → 4-bit output)
  const sboxOut: number[] = [];
  for (let boxIdx = 0; boxIdx < 8; boxIdx++) {
    const chunk = xored.slice(boxIdx * 6, boxIdx * 6 + 6);
    // Row = outer bits (first and last of the 6-bit chunk)
    const row = (chunk[0] << 1) | chunk[5];
    // Col = inner 4 bits
    const col = (chunk[1] << 3) | (chunk[2] << 2) | (chunk[3] << 1) | chunk[4];
    const val = _SBOXES[boxIdx][row][col];
    // Convert 4-bit value to bits (MSB first)
    for (let bitPos = 3; bitPos >= 0; bitPos--) {
      sboxOut.push((val >> bitPos) & 1);
    }
  }

  // Step 4: P permutation
  return permute(sboxOut, _P);
}

// ─────────────────────────────────────────────────────────────────────────────
// Core block cipher
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Encrypt or decrypt a single 8-byte block using the provided subkey list.
 *
 * Encryption: pass subkeys in order [K1..K16]
 * Decryption: pass subkeys in reverse order [K16..K1]
 *
 * The Feistel structure means decryption requires no inverse round function —
 * the same hardware handles both directions. This was a deliberate design goal:
 * a single chip could be used for both encryption and decryption in the 1970s.
 */
function desBlock(block: Uint8Array, subkeys: Uint8Array[]): Uint8Array {
  if (block.length !== 8) {
    throw new Error(`DES block must be exactly 8 bytes, got ${block.length}`);
  }

  let bits = bytesToBits(block);

  // Initial permutation
  bits = permute(bits, _IP);

  // Split into L₀ and R₀
  let left = bits.slice(0, 32);
  let right = bits.slice(32);

  // 16 Feistel rounds
  for (const subkey of subkeys) {
    const fOut = feistelF(right, subkey);
    const newRight = left.map((bit, i) => bit ^ fOut[i]);
    left = right;
    right = newRight;
  }

  // Swap halves before final permutation (standard DES step)
  const combined = [...right, ...left];

  // Final permutation (IP⁻¹)
  return bitsToBytes(permute(combined, _FP));
}

/**
 * Encrypt a single 64-bit (8-byte) block using DES.
 *
 * @param block - 8 bytes of plaintext
 * @param key   - 8 bytes (64 bits; 56 are key material, 8 are parity)
 * @returns 8 bytes of ciphertext
 *
 * WARNING: Use only for educational purposes or legacy compatibility.
 * DES with a 56-bit key is completely broken by modern hardware.
 */
export function desEncryptBlock(block: Uint8Array, key: Uint8Array): Uint8Array {
  const subkeys = expandKey(key);
  return desBlock(block, subkeys);
}

/**
 * Decrypt a single 64-bit (8-byte) block using DES.
 *
 * Decryption is encryption with subkeys reversed — a direct consequence of
 * the Feistel network's self-inverse property.
 *
 * @param block - 8 bytes of ciphertext
 * @param key   - 8 bytes (same key used for encryption)
 * @returns 8 bytes of plaintext
 */
export function desDecryptBlock(block: Uint8Array, key: Uint8Array): Uint8Array {
  const subkeys = expandKey(key);
  return desBlock(block, subkeys.reverse());
}

// ─────────────────────────────────────────────────────────────────────────────
// ECB mode (educational only)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * PKCS#7 padding: append N bytes each with value N, where N is the number of
 * bytes needed to reach the next block boundary (1 ≤ N ≤ blockSize).
 *
 * If the data is already block-aligned, a full block of padding is added so
 * that unpadding is always unambiguous.
 *
 * Example: 5 bytes, blockSize=8 → append 3 bytes of value 0x03.
 */
function pkcs7Pad(data: Uint8Array, blockSize: number): Uint8Array {
  const padLen = blockSize - (data.length % blockSize);
  const padded = new Uint8Array(data.length + padLen);
  padded.set(data, 0);
  padded.fill(padLen, data.length);
  return padded;
}

/**
 * Remove PKCS#7 padding. Throws if padding is invalid.
 */
function pkcs7Unpad(data: Uint8Array): Uint8Array {
  if (data.length === 0) {
    throw new Error("Cannot unpad empty data");
  }
  const padLen = data[data.length - 1];
  if (padLen === 0 || padLen > 8) {
    throw new Error(`Invalid PKCS#7 padding byte: ${padLen}`);
  }
  if (data.length < padLen) {
    throw new Error("Padding length exceeds data length");
  }
  for (let i = data.length - padLen; i < data.length; i++) {
    if (data[i] !== padLen) {
      throw new Error("Invalid PKCS#7 padding (bytes do not match)");
    }
  }
  return data.slice(0, data.length - padLen);
}

/**
 * Encrypt variable-length plaintext with DES in ECB mode (PKCS#7 padding).
 *
 * WARNING: ECB mode is insecure for most purposes. Identical 8-byte plaintext
 * blocks always produce identical ciphertext blocks, leaking data patterns.
 * The canonical demonstration is the "ECB penguin": encrypt a bitmap in ECB
 * mode and the image structure remains visible in the ciphertext.
 *
 * This function exists for:
 *   - Compatibility with historical DES ECB data
 *   - Educational demonstration of ECB's weakness
 *   - As a stepping stone to understanding modes of operation
 */
export function desEcbEncrypt(plaintext: Uint8Array, key: Uint8Array): Uint8Array {
  const subkeys = expandKey(key);
  const padded = pkcs7Pad(plaintext, 8);
  const result = new Uint8Array(padded.length);
  for (let i = 0; i < padded.length; i += 8) {
    result.set(desBlock(padded.subarray(i, i + 8), subkeys), i);
  }
  return result;
}

/**
 * Decrypt variable-length ciphertext with DES in ECB mode.
 *
 * @param ciphertext - bytes (must be a multiple of 8)
 * @param key - 8 bytes
 * @returns plaintext with PKCS#7 padding removed
 */
export function desEcbDecrypt(ciphertext: Uint8Array, key: Uint8Array): Uint8Array {
  if (ciphertext.length === 0) {
    throw new Error("Ciphertext must not be empty");
  }
  if (ciphertext.length % 8 !== 0) {
    throw new Error("DES ECB ciphertext length must be a multiple of 8 bytes");
  }
  const subkeys = expandKey(key).reverse();
  const result = new Uint8Array(ciphertext.length);
  for (let i = 0; i < ciphertext.length; i += 8) {
    result.set(desBlock(ciphertext.subarray(i, i + 8), subkeys), i);
  }
  return pkcs7Unpad(result);
}

// ─────────────────────────────────────────────────────────────────────────────
// Triple DES (3DES / TDEA)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Encrypt one 8-byte block with Triple DES (3TDEA / EDE mode).
 *
 * Algorithm (NIST SP 800-67): C = E_K1(D_K2(E_K3(P)))
 *
 * Applied right-to-left to plaintext:
 *   1. Encrypt with K3
 *   2. Decrypt with K2
 *   3. Encrypt with K1
 *
 * The EDE (Encrypt-Decrypt-Encrypt) structure gives backward compatibility:
 * if K1 = K2 = K3 = K, then 3DES reduces to single DES since D(K, E(K, x)) = x.
 *
 * Effective security: ~112 bits (168-bit key reduced by meet-in-the-middle).
 *
 * NIST deprecated 3DES for new applications in 2017 and disallowed it entirely
 * in 2023 due to the SWEET32 attack on 64-bit block sizes.
 */
export function tdeaEncryptBlock(
  block: Uint8Array,
  k1: Uint8Array,
  k2: Uint8Array,
  k3: Uint8Array,
): Uint8Array {
  const step1 = desEncryptBlock(block, k3);   // E_K3(P)
  const step2 = desDecryptBlock(step1, k2);   // D_K2(E_K3(P))
  return desEncryptBlock(step2, k1);           // E_K1(D_K2(E_K3(P)))
}

/**
 * Decrypt one 8-byte block with Triple DES (3TDEA / EDE mode).
 *
 * Algorithm (NIST SP 800-67): P = D_K3(E_K2(D_K1(C)))
 *
 * Applied right-to-left to ciphertext:
 *   1. Decrypt with K1
 *   2. Encrypt with K2
 *   3. Decrypt with K3
 */
export function tdeaDecryptBlock(
  block: Uint8Array,
  k1: Uint8Array,
  k2: Uint8Array,
  k3: Uint8Array,
): Uint8Array {
  const step1 = desDecryptBlock(block, k1);   // D_K1(C)
  const step2 = desEncryptBlock(step1, k2);   // E_K2(D_K1(C))
  return desDecryptBlock(step2, k3);           // D_K3(E_K2(D_K1(C)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility: hex conversion (useful in tests and REPL sessions)
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Convert a Uint8Array to a lowercase hex string.
 *
 *   toHex(new Uint8Array([0x85, 0xE8])) → "85e8"
 */
export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Convert a hex string (uppercase or lowercase, no spaces) to Uint8Array.
 *
 *   fromHex("85e813540f0ab405") → Uint8Array([0x85, 0xe8, ...])
 */
export function fromHex(hex: string): Uint8Array {
  const clean = hex.replace(/\s+/g, "");
  const result = new Uint8Array(clean.length / 2);
  for (let i = 0; i < result.length; i++) {
    result[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return result;
}
