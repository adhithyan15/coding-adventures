/**
 * pbkdf2 — PBKDF2 (Password-Based Key Derivation Function 2) — RFC 8018.
 *
 * ## What Is PBKDF2?
 *
 * PBKDF2 derives a cryptographic key from a password by applying a pseudorandom
 * function (PRF) — typically HMAC — `c` times per output block. The iteration
 * count `c` is the tunable cost: every brute-force guess requires the same `c`
 * PRF calls as the original derivation.
 *
 * Real-world uses:
 * - WPA2 Wi-Fi: PBKDF2-HMAC-SHA1, 4096 iterations
 * - Django: PBKDF2-HMAC-SHA256, 720,000 iterations (2024)
 * - macOS Keychain: PBKDF2-HMAC-SHA256
 *
 * ## Algorithm (RFC 8018 § 5.2)
 *
 * ```
 * DK = T_1 || T_2 || ... (first dkLen bytes)
 *
 * T_i = U_1 XOR U_2 XOR ... XOR U_c
 *
 * U_1 = PRF(Password, Salt || INT_32_BE(i))
 * U_j = PRF(Password, U_{j-1})   for j = 2..c
 * ```
 *
 * INT_32_BE(i) is the block counter encoded as a 4-byte big-endian integer
 * appended to the salt. This makes each block's first U value unique.
 *
 * ## Security Notes
 *
 * OWASP 2023 minimums:
 * - HMAC-SHA256: 600,000 iterations
 * - HMAC-SHA1:   1,300,000 iterations
 *
 * For new systems consider Argon2id (memory-hard, resists GPU attacks).
 */

import { hmacSHA1, hmacSHA256, hmacSHA512 } from "@coding-adventures/hmac";

export const VERSION = "0.1.0";

// ─────────────────────────────────────────────────────────────────────────────
// Core loop
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Generic PBKDF2 — used by all public convenience functions.
 *
 * @param prf       PRF(key, msg) → Uint8Array of length hLen
 * @param hLen      Output byte length of prf
 * @param password  Secret being stretched — becomes the HMAC key
 * @param salt      Unique random value per credential (≥16 bytes recommended)
 * @param iterations Number of PRF calls per block
 * @param keyLength  Number of derived bytes to produce
 */
function pbkdf2Core(
  prf: (key: Uint8Array, msg: Uint8Array) => Uint8Array,
  hLen: number,
  password: Uint8Array,
  salt: Uint8Array,
  iterations: number,
  keyLength: number,
): Uint8Array {
  if (password.length === 0) {
    throw new Error("PBKDF2 password must not be empty");
  }
  if (iterations <= 0 || !Number.isInteger(iterations)) {
    throw new Error("PBKDF2 iterations must be a positive integer");
  }
  if (keyLength <= 0 || !Number.isInteger(keyLength)) {
    throw new Error("PBKDF2 keyLength must be a positive integer");
  }

  // How many hLen-sized blocks do we need?
  const numBlocks = Math.ceil(keyLength / hLen);
  const dk = new Uint8Array(numBlocks * hLen);

  // blockIdx: 4-byte buffer for the big-endian block counter.
  const blockIdx = new Uint8Array(4);
  const view = new DataView(blockIdx.buffer);

  for (let i = 1; i <= numBlocks; i++) {
    // Seed = Salt || INT_32_BE(i)
    view.setUint32(0, i, false); // big-endian
    const seed = concat(salt, blockIdx);

    // U_1 = PRF(Password, Seed)
    let u = prf(password, seed);

    // t accumulates the XOR of all U values.
    const t = new Uint8Array(u);

    // U_j = PRF(Password, U_{j-1}), XOR into t.
    for (let j = 1; j < iterations; j++) {
      u = prf(password, u);
      for (let k = 0; k < hLen; k++) {
        t[k] ^= u[k];
      }
    }

    dk.set(t, (i - 1) * hLen);
  }

  return dk.slice(0, keyLength);
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

function concat(a: Uint8Array, b: Uint8Array): Uint8Array {
  const out = new Uint8Array(a.length + b.length);
  out.set(a, 0);
  out.set(b, a.length);
  return out;
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/**
 * PBKDF2 with HMAC-SHA1 as the PRF.
 *
 * hLen = 20 bytes. Used in WPA2 (4096 iterations).
 * For new systems prefer {@link pbkdf2HmacSHA256}.
 *
 * @example
 * // RFC 6070 test vector
 * pbkdf2HmacSHA1Hex(enc("password"), enc("salt"), 1, 20)
 * // → "0c60c80f961f0e71f3a9b524af6012062fe037a6"
 */
export function pbkdf2HmacSHA1(
  password: Uint8Array,
  salt: Uint8Array,
  iterations: number,
  keyLength: number,
): Uint8Array {
  return pbkdf2Core(
    (key, msg) => hmacSHA1(key, msg),
    20,
    password,
    salt,
    iterations,
    keyLength,
  );
}

/**
 * PBKDF2 with HMAC-SHA256 as the PRF.
 *
 * hLen = 32 bytes. Recommended for new systems (OWASP 2023: ≥ 600,000 iterations).
 *
 * @example
 * // RFC 7914 Appendix B
 * pbkdf2HmacSHA256Hex(enc("passwd"), enc("salt"), 1, 64)
 * // → "55ac046e56e3089fec1691c22544b605..."
 */
export function pbkdf2HmacSHA256(
  password: Uint8Array,
  salt: Uint8Array,
  iterations: number,
  keyLength: number,
): Uint8Array {
  return pbkdf2Core(
    (key, msg) => hmacSHA256(key, msg),
    32,
    password,
    salt,
    iterations,
    keyLength,
  );
}

/**
 * PBKDF2 with HMAC-SHA512 as the PRF.
 *
 * hLen = 64 bytes. Suitable for high-security applications.
 */
export function pbkdf2HmacSHA512(
  password: Uint8Array,
  salt: Uint8Array,
  iterations: number,
  keyLength: number,
): Uint8Array {
  return pbkdf2Core(
    (key, msg) => hmacSHA512(key, msg),
    64,
    password,
    salt,
    iterations,
    keyLength,
  );
}

/** Like {@link pbkdf2HmacSHA1} but returns a lowercase hex string. */
export function pbkdf2HmacSHA1Hex(
  password: Uint8Array,
  salt: Uint8Array,
  iterations: number,
  keyLength: number,
): string {
  return toHex(pbkdf2HmacSHA1(password, salt, iterations, keyLength));
}

/** Like {@link pbkdf2HmacSHA256} but returns a lowercase hex string. */
export function pbkdf2HmacSHA256Hex(
  password: Uint8Array,
  salt: Uint8Array,
  iterations: number,
  keyLength: number,
): string {
  return toHex(pbkdf2HmacSHA256(password, salt, iterations, keyLength));
}

/** Like {@link pbkdf2HmacSHA512} but returns a lowercase hex string. */
export function pbkdf2HmacSHA512Hex(
  password: Uint8Array,
  salt: Uint8Array,
  iterations: number,
  keyLength: number,
): string {
  return toHex(pbkdf2HmacSHA512(password, salt, iterations, keyLength));
}
