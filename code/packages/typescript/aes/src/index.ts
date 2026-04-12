/**
 * @coding-adventures/aes
 *
 * AES (Advanced Encryption Standard) block cipher — FIPS 197.
 *
 * AES is the most widely deployed symmetric encryption algorithm in the world.
 * Published by NIST in 2001 as FIPS 197, it replaced DES and is used in:
 *   - TLS/HTTPS (every secure web connection)
 *   - WPA2/WPA3 WiFi
 *   - Disk encryption (BitLocker, LUKS, FileVault)
 *   - VPNs, SSH, and virtually every secure protocol
 *
 * Designed by Joan Daemen and Vincent Rijmen (the algorithm was called Rijndael),
 * AES is a Substitution-Permutation Network (SPN) — fundamentally different from
 * DES's Feistel network. ALL bytes of the state are transformed on every round,
 * not just half. This provides faster diffusion.
 *
 * Architecture
 * ============
 *
 *   plaintext (16 bytes)
 *        │
 *   AddRoundKey(state, round_key[0])       ← XOR with first key material
 *        │
 *   ┌── Nr-1 full rounds ─────────────────────────────────────────────┐
 *   │   SubBytes   — non-linear S-box (GF(2^8) inverse + affine)      │
 *   │   ShiftRows  — cyclic row shifts (diffusion across columns)      │
 *   │   MixColumns — GF(2^8) matrix multiply (diffusion across rows)  │
 *   │   AddRoundKey — XOR with round key                              │
 *   └──────────────────────────────────────────────────────────────────┘
 *        │
 *   SubBytes + ShiftRows + AddRoundKey    ← final round (no MixColumns)
 *        │
 *   ciphertext (16 bytes)
 *
 * The state is a 4×4 byte matrix in column-major order:
 *   state[row][col] = block[row + 4*col]
 *
 * GF(2^8) Connection
 * ==================
 * AES arithmetic lives in GF(2^8) with irreducible polynomial:
 *   p(x) = x^8 + x^4 + x^3 + x + 1  =  0x11B
 *
 * This differs from Reed-Solomon's 0x11D. We use @coding-adventures/gf256's
 * `createField(0x11B)` to create an AES-specific field.
 *
 * The S-box maps each byte to:
 *   1. Its multiplicative inverse in GF(2^8) (0 → 0)
 *   2. An affine transformation over GF(2): XOR rotations + 0x63
 *
 * This two-step design ensures non-linearity (the GF inverse) and eliminates
 * fixed points (the affine constant 0x63 ensures no byte maps to itself).
 *
 * Key Sizes and Round Counts
 * ==========================
 *   Key size   Nk (words)   Nr (rounds)   Round keys
 *   128 bits      4             10          11 × 16 bytes
 *   192 bits      6             12          13 × 16 bytes
 *   256 bits      8             14          15 × 16 bytes
 *
 * Public API
 * ==========
 *   aesEncryptBlock(block, key) → Uint8Array (16 bytes)
 *   aesDecryptBlock(block, key) → Uint8Array (16 bytes)
 *   expandKey(key)              → list of round-key matrices
 *   SBOX                        — 256-entry S-box constant
 *   INV_SBOX                    — 256-entry inverse S-box constant
 */

import { createField } from "@coding-adventures/gf256";

export const VERSION = "0.1.0";

// ─────────────────────────────────────────────────────────────────────────────
// AES GF(2^8) field — polynomial 0x11B = x^8 + x^4 + x^3 + x + 1
//
// Note: The Reed-Solomon polynomial is 0x11D (used by gf256's module-level
// functions). AES uses 0x11B. We use `createField(0x11B)` which uses Russian
// peasant multiplication — this works correctly for any primitive polynomial
// without needing a specific generator.
// ─────────────────────────────────────────────────────────────────────────────

const _AES_FIELD = createField(0x11B);

// ─────────────────────────────────────────────────────────────────────────────
// S-box and inverse S-box generation
//
// SubBytes maps each byte b to:
//   1. inv = b^{-1} in GF(2^8) with 0x11B   (0 maps to 0)
//   2. Affine transform over GF(2):
//      s = inv XOR rot(inv,1) XOR rot(inv,2) XOR rot(inv,3) XOR rot(inv,4) XOR 0x63
//
// The affine constant 0x63 ensures:
//   - No fixed points: SBOX[b] ≠ b for any b
//   - No 0→0 mapping after the GF inverse (0 has no GF inverse, maps to 0 first,
//     but the affine 0x63 XOR would map it to 0x63 without special casing)
//   - Actually: 0 is special-cased to inv=0 before the affine transform
// ─────────────────────────────────────────────────────────────────────────────

/**
 * AES affine transformation over GF(2).
 *
 * For each bit position i (0..7):
 *   s_i = b_i XOR b_{(i+4)%8} XOR b_{(i+5)%8} XOR b_{(i+6)%8} XOR b_{(i+7)%8} XOR c_i
 * where c = 0x63 = 01100011.
 *
 * Equivalent matrix form: s = M·b XOR c, where M is a circulant matrix with
 * first row 11110001. The matrix multiplication mixes all bits of the byte.
 */
function affineTransform(b: number): number {
  let result = 0;
  for (let i = 0; i < 8; i++) {
    const bit =
      ((b >> i) & 1) ^
      ((b >> ((i + 4) % 8)) & 1) ^
      ((b >> ((i + 5) % 8)) & 1) ^
      ((b >> ((i + 6) % 8)) & 1) ^
      ((b >> ((i + 7) % 8)) & 1) ^
      ((0x63 >> i) & 1);
    result |= bit << i;
  }
  return result;
}

/**
 * Build the AES S-box and its inverse at module load time.
 *
 * For each byte b (0..255):
 *   - Compute the multiplicative inverse in GF(2^8) with 0x11B
 *     (0 has no inverse; it maps to 0 by convention)
 *   - Apply the AES affine transformation
 *
 * The inverse S-box is built by inverting: INV_SBOX[SBOX[b]] = b.
 */
function buildSbox(): [number[], number[]] {
  const sbox = new Array<number>(256).fill(0);
  for (let b = 0; b < 256; b++) {
    const inv = b === 0 ? 0 : _AES_FIELD.inverse(b);
    sbox[b] = affineTransform(inv);
  }
  const invSbox = new Array<number>(256).fill(0);
  for (let b = 0; b < 256; b++) {
    invSbox[sbox[b]] = b;
  }
  return [sbox, invSbox];
}

const [_SBOX, _INV_SBOX] = buildSbox();

/**
 * AES S-box: 256-entry substitution table.
 *
 * Spot-check values from FIPS 197 Figure 7:
 *   SBOX[0x00] = 0x63
 *   SBOX[0x01] = 0x7c
 *   SBOX[0xff] = 0x16
 */
export const SBOX: ReadonlyArray<number> = _SBOX;

/**
 * AES inverse S-box: INV_SBOX[SBOX[b]] = b for all b.
 */
export const INV_SBOX: ReadonlyArray<number> = _INV_SBOX;

// ─────────────────────────────────────────────────────────────────────────────
// Round constants (Rcon) for the key schedule
//
// Rcon[i] = 2^{i-1} in GF(2^8) for i = 1..10.
// These are the first byte of a 4-byte word [Rcon_i, 0, 0, 0].
// They break symmetry in the key schedule so no two round keys are identical.
//
// Precomputed: [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36]
// ─────────────────────────────────────────────────────────────────────────────

const _RCON: number[] = [0x00]; // index 0 unused; NIST is 1-indexed
{
  let val = 1;
  for (let i = 0; i < 14; i++) {
    _RCON.push(val);
    val = _AES_FIELD.multiply(val, 0x02);
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// MixColumns
//
// Each column of the 4×4 state is treated as a polynomial in GF(2^8) and
// multiplied by the fixed AES MixColumns matrix:
//
//   [2 3 1 1]   [s0]
//   [1 2 3 1] × [s1]
//   [1 1 2 3]   [s2]
//   [3 1 1 2]   [s3]
//
// InvMixColumns uses the inverse matrix:
//   [14  11  13   9]
//   [ 9  14  11  13]
//   [13   9  14  11]
//   [11  13   9  14]
// ─────────────────────────────────────────────────────────────────────────────

/** Multiply b by 2 (x) in GF(2^8) with AES polynomial 0x11B. */
function xtime(b: number): number {
  return _AES_FIELD.multiply(b, 0x02);
}

/** Apply MixColumns to one 4-byte column. */
function mixCol(col: [number, number, number, number]): [number, number, number, number] {
  const [s0, s1, s2, s3] = col;
  // 2·x = xtime(x), 3·x = xtime(x) XOR x
  const t0 = xtime(s0) ^ (xtime(s1) ^ s1) ^ s2 ^ s3;
  const t1 = s0 ^ xtime(s1) ^ (xtime(s2) ^ s2) ^ s3;
  const t2 = s0 ^ s1 ^ xtime(s2) ^ (xtime(s3) ^ s3);
  const t3 = (xtime(s0) ^ s0) ^ s1 ^ s2 ^ xtime(s3);
  return [t0, t1, t2, t3];
}

/** Apply InvMixColumns to one 4-byte column. */
function invMixCol(col: [number, number, number, number]): [number, number, number, number] {
  const [s0, s1, s2, s3] = col;
  const f = _AES_FIELD.multiply.bind(_AES_FIELD);
  // Coefficients: 14=0x0e, 11=0x0b, 13=0x0d, 9=0x09
  const t0 = f(0x0e, s0) ^ f(0x0b, s1) ^ f(0x0d, s2) ^ f(0x09, s3);
  const t1 = f(0x09, s0) ^ f(0x0e, s1) ^ f(0x0b, s2) ^ f(0x0d, s3);
  const t2 = f(0x0d, s0) ^ f(0x09, s1) ^ f(0x0e, s2) ^ f(0x0b, s3);
  const t3 = f(0x0b, s0) ^ f(0x0d, s1) ^ f(0x09, s2) ^ f(0x0e, s3);
  return [t0, t1, t2, t3];
}

// ─────────────────────────────────────────────────────────────────────────────
// State representation
//
// AES state is a 4×4 matrix, indexed state[row][col].
// Bytes are loaded column-major from the block:
//   block[0]  block[4]  block[8]  block[12]
//   block[1]  block[5]  block[9]  block[13]
//   block[2]  block[6]  block[10] block[14]
//   block[3]  block[7]  block[11] block[15]
// ─────────────────────────────────────────────────────────────────────────────

type State = [[number, number, number, number], [number, number, number, number], [number, number, number, number], [number, number, number, number]];

function bytesToState(block: Uint8Array): State {
  return [
    [block[0], block[4], block[8],  block[12]],
    [block[1], block[5], block[9],  block[13]],
    [block[2], block[6], block[10], block[14]],
    [block[3], block[7], block[11], block[15]],
  ];
}

function stateToBytes(state: State): Uint8Array {
  const result = new Uint8Array(16);
  for (let col = 0; col < 4; col++) {
    for (let row = 0; row < 4; row++) {
      result[row + 4 * col] = state[row][col];
    }
  }
  return result;
}

function addRoundKey(state: State, roundKey: State): State {
  return [
    [state[0][0] ^ roundKey[0][0], state[0][1] ^ roundKey[0][1], state[0][2] ^ roundKey[0][2], state[0][3] ^ roundKey[0][3]],
    [state[1][0] ^ roundKey[1][0], state[1][1] ^ roundKey[1][1], state[1][2] ^ roundKey[1][2], state[1][3] ^ roundKey[1][3]],
    [state[2][0] ^ roundKey[2][0], state[2][1] ^ roundKey[2][1], state[2][2] ^ roundKey[2][2], state[2][3] ^ roundKey[2][3]],
    [state[3][0] ^ roundKey[3][0], state[3][1] ^ roundKey[3][1], state[3][2] ^ roundKey[3][2], state[3][3] ^ roundKey[3][3]],
  ];
}

function subBytes(state: State): State {
  return state.map((row) => row.map((b) => _SBOX[b])) as State;
}

function invSubBytes(state: State): State {
  return state.map((row) => row.map((b) => _INV_SBOX[b])) as State;
}

/**
 * ShiftRows: cyclically shift row i left by i positions.
 *
 * Row 0: no shift
 * Row 1: shift left 1
 * Row 2: shift left 2
 * Row 3: shift left 3
 *
 * This ensures that after MixColumns, each output column is a function of all
 * four input columns — providing full cross-column diffusion.
 */
function shiftRows(state: State): State {
  return [
    [state[0][0], state[0][1], state[0][2], state[0][3]],
    [state[1][1], state[1][2], state[1][3], state[1][0]],
    [state[2][2], state[2][3], state[2][0], state[2][1]],
    [state[3][3], state[3][0], state[3][1], state[3][2]],
  ];
}

/** InvShiftRows: shift row i right by i positions. */
function invShiftRows(state: State): State {
  return [
    [state[0][0], state[0][1], state[0][2], state[0][3]],
    [state[1][3], state[1][0], state[1][1], state[1][2]],
    [state[2][2], state[2][3], state[2][0], state[2][1]],
    [state[3][1], state[3][2], state[3][3], state[3][0]],
  ];
}

/** Apply MixColumns to each of the 4 columns. */
function mixColumns(state: State): State {
  const result: State = [
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
  ];
  for (let col = 0; col < 4; col++) {
    const column: [number, number, number, number] = [
      state[0][col], state[1][col], state[2][col], state[3][col],
    ];
    const mixed = mixCol(column);
    for (let row = 0; row < 4; row++) {
      result[row][col] = mixed[row];
    }
  }
  return result;
}

/** Apply InvMixColumns to each of the 4 columns. */
function invMixColumns(state: State): State {
  const result: State = [
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
  ];
  for (let col = 0; col < 4; col++) {
    const column: [number, number, number, number] = [
      state[0][col], state[1][col], state[2][col], state[3][col],
    ];
    const mixed = invMixCol(column);
    for (let row = 0; row < 4; row++) {
      result[row][col] = mixed[row];
    }
  }
  return result;
}

// ─────────────────────────────────────────────────────────────────────────────
// Key schedule: expandKey
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Expand a 16-, 24-, or 32-byte AES key into round keys.
 *
 * Returns an array of (Nr+1) round keys, each a 4×4 State matrix.
 * Round key 0 is used in the initial AddRoundKey; round key Nr is used last.
 *
 * Key schedule algorithm (FIPS 197 Section 5.2):
 *   - Nk = key length in 32-bit words (4, 6, or 8)
 *   - Nr = number of rounds (10, 12, or 14)
 *   - W[i] = W[i-1] XOR W[i-Nk]  (for i not a multiple of Nk)
 *   - W[i] = SubWord(RotWord(W[i-1])) XOR Rcon[i/Nk] XOR W[i-Nk]  (i mod Nk == 0)
 *   - W[i] = SubWord(W[i-1]) XOR W[i-Nk]  (AES-256 extra step: Nk=8 and i mod Nk == 4)
 *
 * @param key - 16, 24, or 32 bytes
 * @returns (Nr+1) round keys, each a 4×4 matrix of bytes
 * @throws Error if key length is not 16, 24, or 32
 */
export function expandKey(key: Uint8Array): State[] {
  const keyLen = key.length;
  if (keyLen !== 16 && keyLen !== 24 && keyLen !== 32) {
    throw new Error(`AES key must be 16, 24, or 32 bytes; got ${keyLen}`);
  }

  const nk = keyLen / 4;
  const nrMap: Record<number, number> = { 4: 10, 6: 12, 8: 14 };
  const nr = nrMap[nk];
  const totalWords = 4 * (nr + 1);

  // W is a flat list of 4-byte words (each word = number[4])
  const w: number[][] = [];
  for (let i = 0; i < nk; i++) {
    w.push([key[4 * i], key[4 * i + 1], key[4 * i + 2], key[4 * i + 3]]);
  }

  for (let i = nk; i < totalWords; i++) {
    let temp = [...w[i - 1]];
    if (i % nk === 0) {
      // RotWord: left-rotate the 4 bytes
      temp = [temp[1], temp[2], temp[3], temp[0]];
      // SubWord: apply S-box to each byte
      temp = temp.map((b) => _SBOX[b]);
      // XOR with round constant
      temp[0] ^= _RCON[i / nk];
    } else if (nk === 8 && i % nk === 4) {
      // Extra SubWord step for AES-256 (Nk=8)
      temp = temp.map((b) => _SBOX[b]);
    }
    w.push(w[i - nk].map((b, j) => b ^ temp[j]));
  }

  // Pack into (Nr+1) round keys, each a 4×4 State (column-major)
  const roundKeys: State[] = [];
  for (let rk = 0; rk <= nr; rk++) {
    const rkWords = w.slice(4 * rk, 4 * rk + 4);
    // state[row][col] = rkWords[col][row]
    const state: State = [
      [rkWords[0][0], rkWords[1][0], rkWords[2][0], rkWords[3][0]],
      [rkWords[0][1], rkWords[1][1], rkWords[2][1], rkWords[3][1]],
      [rkWords[0][2], rkWords[1][2], rkWords[2][2], rkWords[3][2]],
      [rkWords[0][3], rkWords[1][3], rkWords[2][3], rkWords[3][3]],
    ];
    roundKeys.push(state);
  }
  return roundKeys;
}

// ─────────────────────────────────────────────────────────────────────────────
// Core block cipher
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Encrypt a single 128-bit (16-byte) block with AES.
 *
 * Supports all three key sizes:
 *   - 16 bytes (AES-128): 10 rounds
 *   - 24 bytes (AES-192): 12 rounds
 *   - 32 bytes (AES-256): 14 rounds
 *
 * Algorithm (FIPS 197 Section 5.1):
 *   AddRoundKey(state, round_key[0])
 *   for round = 1 to Nr-1:
 *     SubBytes → ShiftRows → MixColumns → AddRoundKey
 *   SubBytes → ShiftRows → AddRoundKey  (final round: no MixColumns)
 *
 * @param block - 16 bytes of plaintext
 * @param key   - 16, 24, or 32 bytes
 * @returns 16 bytes of ciphertext
 */
export function aesEncryptBlock(block: Uint8Array, key: Uint8Array): Uint8Array {
  if (block.length !== 16) {
    throw new Error(`AES block must be 16 bytes, got ${block.length}`);
  }
  const roundKeys = expandKey(key);
  const nr = roundKeys.length - 1;

  let state = bytesToState(block);
  state = addRoundKey(state, roundKeys[0]);

  for (let rnd = 1; rnd < nr; rnd++) {
    state = subBytes(state);
    state = shiftRows(state);
    state = mixColumns(state);
    state = addRoundKey(state, roundKeys[rnd]);
  }

  // Final round: no MixColumns
  state = subBytes(state);
  state = shiftRows(state);
  state = addRoundKey(state, roundKeys[nr]);

  return stateToBytes(state);
}

/**
 * Decrypt a single 128-bit (16-byte) block with AES.
 *
 * Unlike DES (Feistel), AES decryption is NOT the same circuit as encryption.
 * Each operation has a distinct inverse:
 *   InvShiftRows → InvSubBytes → AddRoundKey → InvMixColumns
 *
 * (AddRoundKey is its own inverse since XOR is self-inverse.)
 *
 * @param block - 16 bytes of ciphertext
 * @param key   - 16, 24, or 32 bytes (same key used for encryption)
 * @returns 16 bytes of plaintext
 */
export function aesDecryptBlock(block: Uint8Array, key: Uint8Array): Uint8Array {
  if (block.length !== 16) {
    throw new Error(`AES block must be 16 bytes, got ${block.length}`);
  }
  const roundKeys = expandKey(key);
  const nr = roundKeys.length - 1;

  let state = bytesToState(block);
  state = addRoundKey(state, roundKeys[nr]);

  for (let rnd = nr - 1; rnd >= 1; rnd--) {
    state = invShiftRows(state);
    state = invSubBytes(state);
    state = addRoundKey(state, roundKeys[rnd]);
    state = invMixColumns(state);
  }

  // Final round
  state = invShiftRows(state);
  state = invSubBytes(state);
  state = addRoundKey(state, roundKeys[0]);

  return stateToBytes(state);
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Convert a Uint8Array to a lowercase hex string.
 */
export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/**
 * Convert a hex string (spaces allowed) to Uint8Array.
 */
export function fromHex(hex: string): Uint8Array {
  const clean = hex.replace(/\s+/g, "");
  const result = new Uint8Array(clean.length / 2);
  for (let i = 0; i < result.length; i++) {
    result[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return result;
}
