// ============================================================================
// HKDF — HMAC-based Extract-and-Expand Key Derivation Function (RFC 5869)
// ============================================================================
//
// What Is Key Derivation?
// =======================
// Many cryptographic protocols start with some "input keying material" (IKM)
// that is not directly suitable as a cryptographic key. The IKM might come
// from a Diffie-Hellman exchange, a password, a random seed, or any other
// source of entropy. A Key Derivation Function (KDF) transforms this raw
// material into one or more cryptographically strong keys.
//
// HKDF is the most widely used KDF in modern cryptography. It appears in:
//
//   - TLS 1.3 (the key schedule is built entirely on HKDF)
//   - Signal Protocol (Double Ratchet key derivation)
//   - WireGuard (handshake key derivation)
//   - Noise Protocol Framework
//   - Web Crypto API (deriveBits / deriveKey)
//
// Why Two Phases?
// ===============
// HKDF splits key derivation into two distinct phases, each with a clear
// cryptographic purpose:
//
//   1. **Extract** — concentrate the entropy from the IKM into a fixed-size
//      pseudorandom key (PRK). This step "cleans up" non-uniform input.
//
//   2. **Expand** — stretch the PRK into as many output bytes as needed,
//      using an "info" string for domain separation.
//
// This two-phase design was proposed by Hugo Krawczyk in his 2010 paper
// "Cryptographic Extraction and Key Derivation: The HKDF Scheme." The
// separation allows protocols to reuse the same PRK for multiple derived
// keys (e.g., encryption key, MAC key, IV) by varying the info string.
//
// Visual Overview
// ===============
//
//   Input Keying Material (IKM)
//          |
//          v
//   +--------------+
//   |   Extract    |  PRK = HMAC(salt, IKM)
//   |  (compress)  |
//   +--------------+
//          |
//          v
//   Pseudorandom Key (PRK)   [exactly HashLen bytes]
//          |
//          v
//   +--------------+
//   |   Expand     |  OKM = T(1) || T(2) || ... || T(N)
//   |  (stretch)   |  where T(i) = HMAC(PRK, T(i-1) || info || i)
//   +--------------+
//          |
//          v
//   Output Keying Material (OKM)  [L bytes, up to 255 * HashLen]
//
// ============================================================================

import { hmacSHA256, hmacSHA512 } from "@coding-adventures/hmac";

// ============================================================================
// Section 1: Hash Algorithm Configuration
// ============================================================================
//
// HKDF is parameterized by a hash function. We support SHA-256 (32-byte
// output) and SHA-512 (64-byte output). The hash function determines:
//
//   - HashLen: the output size of the hash (and the PRK size)
//   - The maximum output length: 255 * HashLen bytes
//   - The default salt: HashLen zero bytes when no salt is provided
//
// ============================================================================

/** Supported hash algorithm names. */
export type HashAlgorithm = "sha256" | "sha512";

/**
 * Returns the hash output length in bytes for the given algorithm.
 *
 * SHA-256 produces 32 bytes (256 bits), SHA-512 produces 64 bytes (512 bits).
 * This length determines the PRK size, default salt size, and maximum
 * output length of HKDF.
 */
function hashLength(hash: HashAlgorithm): number {
  switch (hash) {
    case "sha256":
      return 32;
    case "sha512":
      return 64;
  }
}

/**
 * Selects the appropriate HMAC function for the given algorithm.
 *
 * Each HMAC variant wraps its hash function per RFC 2104:
 *   HMAC(K, M) = H((K ^ opad) || H((K ^ ipad) || M))
 */
function hmacFunction(
  hash: HashAlgorithm
): (key: Uint8Array, message: Uint8Array) => Uint8Array {
  switch (hash) {
    case "sha256":
      return hmacSHA256;
    case "sha512":
      return hmacSHA512;
  }
}

// ============================================================================
// Section 2: HKDF-Extract
// ============================================================================
//
// The Extract phase takes potentially non-uniform input keying material (IKM)
// and produces a fixed-length pseudorandom key (PRK).
//
// The formula is simply:
//
//   PRK = HMAC-Hash(salt, IKM)
//
// Note the argument order: salt is the HMAC *key*, and IKM is the *message*.
// This is intentional — the salt acts as a randomization value that helps
// the extraction even when the IKM has structure or low entropy.
//
// Salt Handling
// =============
// RFC 5869 Section 2.2 specifies:
//
//   "if not provided, [salt] is set to a string of HashLen zeros"
//
// An empty or missing salt means we use HashLen bytes of 0x00 as the key.
// While HMAC with an all-zero key is not ideal, it still provides a secure
// extraction as proven in Krawczyk's analysis.
//
// ============================================================================

/**
 * HKDF-Extract: compress input keying material into a pseudorandom key.
 *
 * @param salt  Optional salt value (a non-secret random value).
 *              If empty or zero-length, HashLen zero bytes are used.
 * @param ikm   Input keying material — the raw secret to derive from.
 * @param hash  Hash algorithm: "sha256" (default) or "sha512".
 * @returns     PRK — a pseudorandom key of HashLen bytes.
 *
 * @example
 * ```typescript
 * const ikm = new Uint8Array([0x0b, 0x0b, ...]); // 22 bytes
 * const salt = new Uint8Array([0x00, 0x01, ...]); // 13 bytes
 * const prk = hkdfExtract(salt, ikm);
 * // prk is 32 bytes (SHA-256) of concentrated entropy
 * ```
 */
export function hkdfExtract(
  salt: Uint8Array,
  ikm: Uint8Array,
  hash: HashAlgorithm = "sha256"
): Uint8Array {
  // If the salt is empty, RFC 5869 says to use HashLen zero bytes.
  // This ensures HMAC always receives a properly-sized key, even when
  // the caller doesn't provide a salt.
  const effectiveSalt =
    salt.length === 0 ? new Uint8Array(hashLength(hash)) : salt;

  const hmac = hmacFunction(hash);

  // PRK = HMAC-Hash(salt, IKM)
  // The salt is the HMAC key; the IKM is the message.
  return hmac(effectiveSalt, ikm);
}

// ============================================================================
// Section 3: HKDF-Expand
// ============================================================================
//
// The Expand phase takes the PRK (from Extract) and produces output keying
// material of any desired length, up to 255 * HashLen bytes.
//
// The expansion works by chaining HMAC calls:
//
//   T(0) = empty string (zero length)
//   T(1) = HMAC-Hash(PRK, T(0) || info || 0x01)
//   T(2) = HMAC-Hash(PRK, T(1) || info || 0x02)
//   T(3) = HMAC-Hash(PRK, T(2) || info || 0x03)
//   ...
//   T(N) = HMAC-Hash(PRK, T(N-1) || info || N)
//
//   OKM = first L bytes of T(1) || T(2) || ... || T(N)
//
// Each T(i) block is exactly HashLen bytes. The counter byte starts at 1
// and goes up to N = ceil(L / HashLen). Since the counter is a single
// octet (0x01 to 0xFF), the maximum N is 255, giving a maximum output
// of 255 * HashLen bytes.
//
// The "info" parameter provides domain separation — different info strings
// produce completely different output, even from the same PRK. This lets
// a single Extract produce multiple independent keys:
//
//   encryption_key = HKDF-Expand(PRK, "enc", 32)
//   mac_key        = HKDF-Expand(PRK, "mac", 32)
//   iv             = HKDF-Expand(PRK, "iv",  16)
//
// ============================================================================

/**
 * HKDF-Expand: stretch a pseudorandom key into output keying material.
 *
 * @param prk     Pseudorandom key (at least HashLen bytes, typically
 *                from hkdfExtract).
 * @param info    Context and application-specific information (can be
 *                empty). Used for domain separation.
 * @param length  Desired output length in bytes (1 to 255 * HashLen).
 * @param hash    Hash algorithm: "sha256" (default) or "sha512".
 * @returns       OKM — output keying material of exactly `length` bytes.
 *
 * @throws {Error} If length is <= 0 or > 255 * HashLen.
 *
 * @example
 * ```typescript
 * const okm = hkdfExpand(prk, info, 42); // derive 42 bytes
 * ```
 */
export function hkdfExpand(
  prk: Uint8Array,
  info: Uint8Array,
  length: number,
  hash: HashAlgorithm = "sha256"
): Uint8Array {
  const hashLen = hashLength(hash);
  const hmac = hmacFunction(hash);

  // Validate the requested length. The counter is a single byte (1..255),
  // so we can produce at most 255 * HashLen bytes.
  if (length <= 0) {
    throw new Error(
      `HKDF-Expand: length must be > 0, got ${length}`
    );
  }

  const maxLength = 255 * hashLen;
  if (length > maxLength) {
    throw new Error(
      `HKDF-Expand: length ${length} exceeds maximum ${maxLength} ` +
        `(255 * ${hashLen}) for ${hash}`
    );
  }

  // N = ceil(L / HashLen) — number of HMAC blocks needed.
  const n = Math.ceil(length / hashLen);

  // We'll accumulate the output blocks T(1) || T(2) || ... || T(N)
  // into a single buffer, then truncate to exactly `length` bytes.
  const okm = new Uint8Array(n * hashLen);

  // T(0) is the empty string — used as "previous block" for the first
  // iteration.
  let previous = new Uint8Array(0);

  for (let i = 1; i <= n; i++) {
    // Build the HMAC input: T(i-1) || info || counter_byte
    //
    // The counter is a single octet with value i (1-indexed).
    // This ensures each block gets a unique input even when info is empty.
    const input = new Uint8Array(previous.length + info.length + 1);
    input.set(previous, 0);
    input.set(info, previous.length);
    input[previous.length + info.length] = i;

    // T(i) = HMAC-Hash(PRK, T(i-1) || info || i)
    const block = hmac(prk, input);
    okm.set(block, (i - 1) * hashLen);

    // T(i) becomes the "previous" for the next iteration.
    previous = block;
  }

  // Return exactly L bytes (truncating the last block if needed).
  return okm.slice(0, length);
}

// ============================================================================
// Section 4: Combined HKDF (Extract + Expand)
// ============================================================================
//
// Most callers want the full HKDF pipeline: Extract then Expand. This
// convenience function chains both steps.
//
// ============================================================================

/**
 * Full HKDF: extract-then-expand in one call.
 *
 * This is the standard way to derive keying material from input secrets.
 * It first compresses the IKM into a PRK via Extract, then stretches
 * the PRK to the desired length via Expand.
 *
 * @param salt    Optional salt (non-secret random value). Empty = HashLen zeros.
 * @param ikm     Input keying material (the raw secret).
 * @param info    Context/application info for domain separation.
 * @param length  Desired output length in bytes.
 * @param hash    Hash algorithm: "sha256" (default) or "sha512".
 * @returns       OKM — derived keying material of exactly `length` bytes.
 *
 * @example
 * ```typescript
 * // Derive a 32-byte encryption key from a DH shared secret
 * const key = hkdf(salt, sharedSecret, new TextEncoder().encode("enc"), 32);
 * ```
 */
export function hkdf(
  salt: Uint8Array,
  ikm: Uint8Array,
  info: Uint8Array,
  length: number,
  hash: HashAlgorithm = "sha256"
): Uint8Array {
  const prk = hkdfExtract(salt, ikm, hash);
  return hkdfExpand(prk, info, length, hash);
}
