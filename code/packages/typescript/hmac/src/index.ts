/**
 * @coding-adventures/hmac
 *
 * HMAC (Hash-based Message Authentication Code) — RFC 2104 / FIPS 198-1.
 *
 * What Is HMAC?
 * =============
 * HMAC takes a secret key and a message and produces a fixed-size authentication
 * tag that proves two things simultaneously:
 *
 *   1. Integrity — the message has not been altered since the tag was created.
 *   2. Authenticity — the sender possesses the secret key.
 *
 * Unlike a plain hash (which anyone can compute), an HMAC tag cannot be forged
 * without the key. HMAC is used everywhere:
 *
 *   - TLS 1.2 PRF (Pseudorandom Function)
 *   - JWT "HS256" and "HS512" signature algorithms
 *   - WPA2 four-way handshake (PBKDF2-HMAC-SHA1)
 *   - TOTP/HOTP one-time passwords (RFC 6238 / 4226)
 *   - AWS Signature Version 4
 *   - Cookie signing in Express.js, Django, Rails
 *
 * Why Not hash(key || message)?
 * ==============================
 * Naively prepending the key looks secure but is vulnerable to the
 * "length extension attack" on Merkle-Damgård hash functions
 * (MD5, SHA-1, SHA-256, SHA-512).
 *
 * Merkle-Damgård hashes keep an internal state that gets "absorbed" block by
 * block. When they output a digest, they output that internal state. This means:
 *
 *   hash(key || msg) ≡ internal_state_after_processing(key || msg)
 *
 * An attacker who knows hash(key || msg) can set that as the starting state and
 * continue hashing — appending arbitrary bytes — without ever knowing `key`.
 *
 *   known: hash(key || msg) = D
 *   attacker computes: hash(key || msg || padding || extra) = D'
 *                      by resuming from state D.
 *
 * This breaks authentication: the attacker extends a valid tag without the key.
 *
 * HMAC fixes this with two nested hash calls under **different** derived keys:
 *
 *   HMAC(K, M) = H((K' ⊕ opad) || H((K' ⊕ ipad) || M))
 *
 * The outer hash takes the inner result as just another message, so the attacker
 * cannot "resume" the outer hash without also knowing K' ⊕ opad — which
 * requires knowing K.
 *
 * The ipad and opad Constants
 * ============================
 * RFC 2104 defines:
 *   ipad = 0x36 repeated (inner pad)
 *   opad = 0x5C repeated (outer pad)
 *
 * Why these specific values? They differ in exactly 4 of 8 bits — the maximum
 * Hamming distance possible for single-byte values when both are XOR'd with the
 * same key. This ensures inner_key and outer_key are as different as possible,
 * even though both are derived from the same K'.
 *
 *   0x36 = 0011_0110
 *   0x5C = 0101_1100
 *   XOR  = 0110_1010  (4 bits differ)
 *
 * The Algorithm (RFC 2104 §2)
 * ============================
 *   1. Normalize K to exactly block_size bytes:
 *        len(K) > block_size → K' = H(K), then zero-pad to block_size
 *        len(K) ≤ block_size → zero-pad to block_size
 *   2. inner_key = K' ⊕ (0x36 × block_size)
 *   3. outer_key = K' ⊕ (0x5C × block_size)
 *   4. inner     = H(inner_key || M)
 *   5. return      H(outer_key || inner)
 *
 * Block Sizes (bytes)
 * ====================
 *   MD5     → block = 64,  digest = 16
 *   SHA-1   → block = 64,  digest = 20
 *   SHA-256 → block = 64,  digest = 32
 *   SHA-512 → block = 128, digest = 64
 *
 * SHA-512 uses 64-bit words (vs 32-bit for SHA-256), so its message schedule
 * processes 128-byte (1024-bit) blocks. This doubles the ipad/opad lengths.
 *
 * RFC 4231 Test Vector TC1 (HMAC-SHA256)
 * ========================================
 *   key = 0x0b × 20
 *   msg = "Hi There"
 *   tag = "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
 */

import { md5 } from "@coding-adventures/md5";
import { sha1 } from "@coding-adventures/sha1";
import { sha256 } from "@coding-adventures/sha256";
import { sha512 } from "@coding-adventures/sha512";

export const VERSION = "0.1.0";

// ─── ipad and opad constants (RFC 2104 §2) ────────────────────────────────────
//
// ipad = 0x36 = 0011_0110  (inner pad — XOR'd with key before inner hash)
// opad = 0x5C = 0101_1100  (outer pad — XOR'd with key before outer hash)
//
// Maximum Hamming distance (4 bits) between the two pads ensures the two
// derived keys are maximally different despite sharing the same source key K'.
//
const IPAD = 0x36;
const OPAD = 0x5c;

// ─── Generic HMAC ─────────────────────────────────────────────────────────────

/**
 * Compute HMAC using any hash function.
 *
 * This is the primitive that all named variants (hmacSHA256, etc.) call.
 * Bring your own hash function to use HMAC with any algorithm.
 *
 * @param hashFn    - One-shot hash: Uint8Array → Uint8Array
 * @param blockSize - Internal block size of hashFn in bytes (64 or 128)
 * @param key       - Secret key, any length
 * @param message   - Data to authenticate, any length
 * @returns Authentication tag as Uint8Array (same length as hashFn output)
 *
 * @example
 * ```ts
 * import { sha256 } from "@coding-adventures/sha256";
 * const tag = hmac(sha256, 64, enc.encode("key"), enc.encode("message"));
 * ```
 */
export function hmac(
  hashFn: (data: Uint8Array) => Uint8Array,
  blockSize: number,
  key: Uint8Array,
  message: Uint8Array,
): Uint8Array {
  // Step 1 — normalize key to exactly blockSize bytes
  const keyPrime = normalizeKey(hashFn, blockSize, key);

  // Step 2 — derive inner and outer padded keys
  const innerKey = xorFill(keyPrime, IPAD);
  const outerKey = xorFill(keyPrime, OPAD);

  // Step 3 — nested hashes
  const innerInput = concat(innerKey, message);
  const inner = hashFn(innerInput);

  const outerInput = concat(outerKey, inner);
  return hashFn(outerInput);
}

// ─── Named variants ───────────────────────────────────────────────────────────

/**
 * HMAC-MD5: 16-byte authentication tag (RFC 2202).
 *
 * HMAC-MD5 remains secure as a MAC even though MD5 is broken for collision
 * resistance. MAC security and collision resistance are different properties.
 * HMAC-MD5 still appears in legacy TLS cipher suites.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * hmacMD5Hex(enc.encode("Jefe"), enc.encode("what do ya want for nothing?"));
 * // "750c783e6ab0b503eaa86e310a5db738"
 * ```
 */
export function hmacMD5(key: Uint8Array, message: Uint8Array): Uint8Array {
  if (key.length === 0) throw new Error("HMAC key must not be empty");
  return hmac(md5, 64, key, message);
}

/**
 * HMAC-SHA1: 20-byte authentication tag (RFC 2202).
 *
 * Used in WPA2 (PBKDF2-HMAC-SHA1), older TLS/SSH handshakes, and TOTP/HOTP.
 * SHA-1 is collision-broken (2017 SHAttered attack) but HMAC-SHA1 remains
 * secure as a MAC — the attack requires a collision, not a MAC forgery.
 *
 * @example
 * ```ts
 * const enc = new TextEncoder();
 * hmacSHA1Hex(enc.encode("Jefe"), enc.encode("what do ya want for nothing?"));
 * // "effcdf6ae5eb2fa2d27416d5f184df9c259a7c79"
 * ```
 */
export function hmacSHA1(key: Uint8Array, message: Uint8Array): Uint8Array {
  if (key.length === 0) throw new Error("HMAC key must not be empty");
  return hmac(sha1, 64, key, message);
}

/**
 * HMAC-SHA256: 32-byte authentication tag (RFC 4231).
 *
 * The modern default for TLS 1.3 record MAC, JWT HS256, AWS Signature V4,
 * and PBKDF2-HMAC-SHA256 password hashing.
 *
 * @example
 * ```ts
 * const key = new Uint8Array(20).fill(0x0b);
 * hmacSHA256Hex(key, new TextEncoder().encode("Hi There"));
 * // "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
 * ```
 */
export function hmacSHA256(key: Uint8Array, message: Uint8Array): Uint8Array {
  if (key.length === 0) throw new Error("HMAC key must not be empty");
  return hmac(sha256, 64, key, message);
}

/**
 * HMAC-SHA512: 64-byte authentication tag (RFC 4231).
 *
 * Used in JWT HS512 and high-security configurations where 256-bit security
 * margin is desired. SHA-512 uses a 128-byte block size — twice that of
 * SHA-256 — so the ipad/opad arrays are 128 bytes.
 *
 * @example
 * ```ts
 * const key = new Uint8Array(20).fill(0x0b);
 * hmacSHA512Hex(key, new TextEncoder().encode("Hi There"));
 * // "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854"
 * ```
 */
export function hmacSHA512(key: Uint8Array, message: Uint8Array): Uint8Array {
  if (key.length === 0) throw new Error("HMAC key must not be empty");
  return hmac(sha512, 128, key, message);
}

// ─── Hex-string variants ──────────────────────────────────────────────────────

/** HMAC-MD5 as a 32-character lowercase hex string. */
export function hmacMD5Hex(key: Uint8Array, message: Uint8Array): string {
  return toHex(hmacMD5(key, message));
}

/** HMAC-SHA1 as a 40-character lowercase hex string. */
export function hmacSHA1Hex(key: Uint8Array, message: Uint8Array): string {
  return toHex(hmacSHA1(key, message));
}

/** HMAC-SHA256 as a 64-character lowercase hex string. */
export function hmacSHA256Hex(key: Uint8Array, message: Uint8Array): string {
  return toHex(hmacSHA256(key, message));
}

/** HMAC-SHA512 as a 128-character lowercase hex string. */
export function hmacSHA512Hex(key: Uint8Array, message: Uint8Array): string {
  return toHex(hmacSHA512(key, message));
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/**
 * Normalize key to exactly blockSize bytes.
 * Long keys are hashed first. Short (and hashed) keys are zero-padded.
 */
function normalizeKey(
  hashFn: (data: Uint8Array) => Uint8Array,
  blockSize: number,
  key: Uint8Array,
): Uint8Array {
  const effective = key.length > blockSize ? hashFn(key) : key;
  const result = new Uint8Array(blockSize); // zero-initialized
  result.set(effective.subarray(0, blockSize));
  return result;
}

/**
 * XOR every byte in data with constant fill value.
 * Used to derive inner_key (fill=0x36) and outer_key (fill=0x5C).
 */
function xorFill(data: Uint8Array, fill: number): Uint8Array {
  const out = new Uint8Array(data.length);
  for (let i = 0; i < data.length; i++) {
    out[i] = data[i] ^ fill;
  }
  return out;
}

/**
 * Concatenate two Uint8Arrays into a new array.
 * Used to form (inner_key || message) and (outer_key || inner).
 */
function concat(a: Uint8Array, b: Uint8Array): Uint8Array {
  const out = new Uint8Array(a.length + b.length);
  out.set(a, 0);
  out.set(b, a.length);
  return out;
}

/**
 * Encode bytes as a lowercase hex string.
 * Each byte becomes exactly two hex characters: 0x0a → "0a".
 */
export function toHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// ─── Constant-time tag verification ──────────────────────────────────────────

/**
 * Compare two HMAC tags in constant time.
 *
 * Use this instead of `===` or array comparison when checking whether a
 * received tag matches an expected tag. Short-circuit comparison leaks timing
 * information about how many bytes match — over many requests an attacker can
 * use these timing differences to reconstruct the expected tag byte by byte
 * (a **timing attack**).
 *
 * This implementation uses a bitwise OR accumulator that processes ALL bytes
 * regardless of where the first mismatch occurs.
 *
 * @param expected - Tag produced locally using the secret key
 * @param actual   - Tag received from an untrusted source
 * @returns `true` iff `expected` and `actual` are byte-for-byte identical
 *
 * @example
 * ```ts
 * const key = new TextEncoder().encode("secret");
 * const tag = hmacSHA256(key, new TextEncoder().encode("message"));
 * verify(tag, tag); // true
 * verify(tag, new Uint8Array(32)); // false
 * ```
 */
export function verify(expected: Uint8Array, actual: Uint8Array): boolean {
  if (expected.length !== actual.length) return false;
  let diff = 0;
  for (let i = 0; i < expected.length; i++) {
    diff |= expected[i] ^ actual[i];
  }
  return diff === 0;
}
