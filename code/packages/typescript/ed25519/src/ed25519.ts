/**
 * Ed25519 Digital Signatures (RFC 8032)
 *
 * Ed25519 is a high-speed, high-security digital signature scheme built on
 * the twisted Edwards curve:
 *
 *   -x^2 + y^2 = 1 + d*x^2*y^2     over GF(2^255 - 19)
 *
 * Why Twisted Edwards Curves?
 * ===========================
 * Edwards curves have a remarkable property: their addition formula is
 * "complete" -- it works for ALL pairs of points, including doubling and
 * the identity. No special cases! This eliminates an entire class of
 * timing side-channel attacks that plague Weierstrass curve implementations.
 *
 * The "twisted" variant (coefficient a = -1 instead of a = 1) enables faster
 * arithmetic while preserving completeness. Ed25519 uses a = -1.
 *
 * Key Properties
 * ==============
 * - 128-bit security level (equivalent to ~3072-bit RSA)
 * - Deterministic signatures (no random nonce needed -- replay-safe)
 * - Fast: ~70 microseconds for signing on modern hardware
 * - Small: 32-byte public keys, 64-byte signatures
 * - Resistant to timing attacks (complete addition formula)
 *
 * How Signing Works (Intuition)
 * =============================
 * Ed25519 uses a Schnorr-like signature scheme:
 *
 * 1. The signer picks a secret scalar `a` and publishes A = a*B (public key)
 * 2. To sign message M:
 *    - Derive a deterministic nonce r from the secret key and message
 *    - Compute R = r*B (commitment point)
 *    - Compute challenge k = SHA-512(R || A || M) mod L
 *    - Compute S = (r + k*a) mod L (response)
 *    - Signature is (R, S) -- 64 bytes total
 *
 * 3. To verify (M, (R,S), A):
 *    - Recompute k = SHA-512(R || A || M) mod L
 *    - Check: S*B == R + k*A
 *
 * Why does verification work?
 *   S*B = (r + k*a)*B = r*B + k*a*B = R + k*A  ✓
 *
 * BigInt Arithmetic
 * =================
 * All field and scalar arithmetic uses JavaScript's native BigInt type.
 * We work modulo p = 2^255 - 19 for field elements and modulo L for scalars.
 * BigInt handles arbitrary-precision integers, so no overflow concerns.
 *
 * Dependencies
 * ============
 * SHA-512 from @coding-adventures/sha512 -- used for:
 * - Key derivation: SHA-512(seed) -> clamped scalar + nonce prefix
 * - Nonce generation: SHA-512(prefix || message) -> deterministic r
 * - Challenge hash: SHA-512(R || A || message) -> k
 */

import { sha512 } from "@coding-adventures/sha512";

// ─── Field Constants ────────────────────────────────────────────────────────
//
// The prime field GF(p) where p = 2^255 - 19. This is a Mersenne-like prime
// chosen for fast modular reduction. Being close to a power of 2 means
// division by p can be done with shifts and a small correction.
//
// d is the curve parameter in -x^2 + y^2 = 1 + d*x^2*y^2.
// It equals -121665/121666 mod p, a ratio chosen to make the curve birationally
// equivalent to the Montgomery curve Curve25519 (used in X25519 key exchange).
//
// L is the order of the base point B -- the number of points in the prime-order
// subgroup. Every scalar is reduced modulo L.

const P = (1n << 255n) - 19n;

const D = 37095705934669439343138083508754565189542113879843219016388785533085940283555n;

const L = 7237005577332262213973186563042994240857116359379907606001950938285454250989n;

// ─── Square Root of -1 ──────────────────────────────────────────────────────
//
// In GF(p), -1 has a square root because p ≡ 5 (mod 8). This constant is
// needed when recovering x from y during point decompression:
//
//   SQRT_M1 = 2^((p-1)/4) mod p
//
// It satisfies SQRT_M1^2 ≡ -1 (mod p).

const SQRT_M1 = 19681161376707505956807079304988542015446066515923890162744021073123829784752n;

// ─── Base Point B ───────────────────────────────────────────────────────────
//
// The generator point of the prime-order subgroup. Its y-coordinate is 4/5
// mod p, and x is chosen to be the positive square root (even).
//
// Every public key is a multiple of B: A = a * B.

const B_Y = 46316835694926478169428394003475163141307993866256225615783033603165251855960n;
const B_X = 15112221349535400772501151409588531511454012693041857206046113283949847762202n;

// ─── Modular Arithmetic Helpers ─────────────────────────────────────────────
//
// All arithmetic in Ed25519 happens in one of two modular rings:
//   1. GF(p) for point coordinates (p = 2^255 - 19)
//   2. Z/LZ for scalars (L = order of base point)
//
// JavaScript BigInt can go negative (e.g., 3n - 5n = -2n), so we always
// add the modulus before taking remainder to ensure non-negative results.

/** Modular addition: (a + b) mod m */
function modAdd(a: bigint, b: bigint, m: bigint): bigint {
  return ((a + b) % m + m) % m;
}

/** Modular subtraction: (a - b) mod m */
function modSub(a: bigint, b: bigint, m: bigint): bigint {
  return ((a - b) % m + m) % m;
}

/** Modular multiplication: (a * b) mod m */
function modMul(a: bigint, b: bigint, m: bigint): bigint {
  return ((a * b) % m + m) % m;
}

/** Modular negation: (-a) mod m */
function modNeg(a: bigint, m: bigint): bigint {
  return (((-a) % m) + m) % m;
}

// ─── Modular Exponentiation ─────────────────────────────────────────────────
//
// Computes base^exp mod m using the square-and-multiply algorithm.
//
// This is the workhorse for modular inversion (via Fermat's little theorem)
// and square root computation. For a prime modulus p:
//
//   a^(-1) = a^(p-2) mod p   (Fermat's little theorem)
//   sqrt(a) starts with a^((p+3)/8) mod p   (since p ≡ 5 mod 8)
//
// The algorithm scans the exponent from high bit to low bit:
//   - For each bit: square the accumulator
//   - If the bit is 1: also multiply by base
//
// This takes O(log exp) multiplications -- about 255 for our 255-bit prime.

function modPow(base: bigint, exp: bigint, m: bigint): bigint {
  base = ((base % m) + m) % m;
  let result = 1n;
  while (exp > 0n) {
    if (exp & 1n) {
      result = (result * base) % m;
    }
    base = (base * base) % m;
    exp >>= 1n;
  }
  return result;
}

// ─── Modular Inverse ────────────────────────────────────────────────────────
//
// For a prime p, Fermat's little theorem gives us:
//   a^(p-1) ≡ 1 (mod p)    for any a not divisible by p
//
// Therefore:
//   a * a^(p-2) ≡ 1 (mod p)
//   a^(-1) = a^(p-2) mod p
//
// This is simpler (and constant-time) compared to the extended Euclidean
// algorithm, though slightly slower.

function modInv(a: bigint, m: bigint): bigint {
  return modPow(a, m - 2n, m);
}

// ─── Field Square Root ──────────────────────────────────────────────────────
//
// Computing square roots modulo p = 2^255 - 19.
//
// Since p ≡ 5 (mod 8), we use the Atkin algorithm:
//
//   1. Compute candidate = a^((p+3)/8) mod p
//   2. If candidate^2 ≡ a (mod p), return candidate
//   3. If candidate^2 ≡ -a (mod p), return candidate * SQRT_M1 mod p
//   4. Otherwise, a is not a quadratic residue (no square root exists)
//
// Why does this work?
//
// For p ≡ 5 (mod 8), we have (p+3)/8 is an integer. If a is a QR, then
// a^((p-1)/2) ≡ 1 (mod p). The candidate a^((p+3)/8) satisfies:
//
//   candidate^2 = a^((p+3)/4) = a * a^((p-1)/4)
//
// Now a^((p-1)/4) is a square root of a^((p-1)/2) = 1, so it equals ±1.
// If it's 1, candidate^2 = a (done!). If it's -1, candidate^2 = -a,
// so we multiply by SQRT_M1 (which squares to -1) to fix the sign.

function fieldSqrt(a: bigint): bigint | null {
  // Candidate = a^((p+3)/8) mod p
  const exp = (P + 3n) >> 3n; // (p + 3) / 8
  const candidate = modPow(a, exp, P);

  // Check: candidate^2 ≡ a (mod p)?
  const check = modMul(candidate, candidate, P);
  if (check === ((a % P) + P) % P) {
    return candidate;
  }

  // Check: candidate^2 ≡ -a (mod p)?
  if (check === modNeg(a, P)) {
    return modMul(candidate, SQRT_M1, P);
  }

  // Not a quadratic residue
  return null;
}

// ─── Extended Point Representation ──────────────────────────────────────────
//
// Ed25519 uses "extended coordinates" (X, Y, Z, T) where:
//   x = X/Z,  y = Y/Z,  T = X*Y/Z
//
// This avoids expensive modular inversions during point addition. Instead of
// dividing, we track a common denominator Z and only invert once at the end
// (during point encoding).
//
// The identity (neutral element) is the point (0, 1) in affine coordinates,
// which becomes (0, 1, 1, 0) in extended coordinates.
//
// Why the extra T coordinate?
// The addition formula needs x1*y1 + x2*y2 terms. Without T, we'd compute
// (X1/Z1)*(Y1/Z1) each time -- expensive. With T = X*Y/Z pre-cached, we
// replace a multiplication + inversion with just using T.

interface ExtendedPoint {
  X: bigint;
  Y: bigint;
  Z: bigint;
  T: bigint;
}

/** The identity point: (0, 1) in affine = (0, 1, 1, 0) in extended. */
const IDENTITY: ExtendedPoint = { X: 0n, Y: 1n, Z: 1n, T: 0n };

/**
 * The base point B in extended coordinates.
 * B has order L (the large prime subgroup order).
 */
const BASE_POINT: ExtendedPoint = {
  X: B_X,
  Y: B_Y,
  Z: 1n,
  T: modMul(B_X, B_Y, P),
};

// ─── Point Addition ─────────────────────────────────────────────────────────
//
// Add two points in extended coordinates on the twisted Edwards curve
// -x^2 + y^2 = 1 + d*x^2*y^2 (where a = -1).
//
// This is the "unified" addition formula from Hisil et al. (2008):
//
//   A = X1*X2           (cross-multiply x numerators)
//   B = Y1*Y2           (cross-multiply y numerators)
//   C = T1*d*T2         (the "twist" -- uses curve parameter d)
//   D = Z1*Z2           (combine denominators)
//   E = (X1+Y1)*(X2+Y2) - A - B   (Karatsuba-like trick for x1*y2 + x2*y1)
//   F = D - C           (denominator for x3)
//   G = D + C           (denominator for y3)
//   H = B + A           (note: B + A because a = -1 gives B - a*A = B + A)
//
//   X3 = E*F,  Y3 = G*H,  T3 = E*H,  Z3 = F*G
//
// Wait, why B + A and not B - A?
// In the general twisted Edwards curve ax^2 + y^2 = 1 + dx^2y^2,
// the addition formula has H = B - a*A. Since a = -1: H = B - (-1)*A = B + A.
//
// This formula is "complete": it works for any two input points, including
// P + P (doubling), P + (-P) (gives identity), and P + identity.
// No special cases needed! This is a huge advantage for security.

function pointAdd(p1: ExtendedPoint, p2: ExtendedPoint): ExtendedPoint {
  const A = modMul(p1.X, p2.X, P);
  const B = modMul(p1.Y, p2.Y, P);
  const C = modMul(modMul(p1.T, D, P), p2.T, P);
  const DD = modMul(p1.Z, p2.Z, P);
  const E = modSub(modMul(modAdd(p1.X, p1.Y, P), modAdd(p2.X, p2.Y, P), P), modAdd(A, B, P), P);
  const F = modSub(DD, C, P);
  const G = modAdd(DD, C, P);
  const H = modAdd(B, A, P); // B + A because a = -1

  return {
    X: modMul(E, F, P),
    Y: modMul(G, H, P),
    Z: modMul(F, G, P),
    T: modMul(E, H, P),
  };
}

// ─── Point Doubling ─────────────────────────────────────────────────────────
//
// Doubling a point is a special case of addition (P + P) but can be done
// with fewer multiplications using dedicated formulas.
//
//   A = X1^2
//   B = Y1^2
//   C = 2*Z1^2
//   D = -A              (because a = -1 in the twisted Edwards equation)
//   E = (X1+Y1)^2 - A - B
//   G = D + B
//   F = G - C
//   H = D - B
//
//   X3 = E*F,  Y3 = G*H,  T3 = E*H,  Z3 = F*G
//
// Note: D = -A = a*X1^2 where a = -1. This is where the "twisted" part
// affects the formula -- standard Edwards (a=1) would have D = A.

function pointDouble(p: ExtendedPoint): ExtendedPoint {
  const A = modMul(p.X, p.X, P);
  const B = modMul(p.Y, p.Y, P);
  const C = modMul(2n, modMul(p.Z, p.Z, P), P);
  const DD = modNeg(A, P); // D = -A because a = -1
  const E = modSub(modMul(modAdd(p.X, p.Y, P), modAdd(p.X, p.Y, P), P), modAdd(A, B, P), P);
  const G = modAdd(DD, B, P);
  const F = modSub(G, C, P);
  const H = modSub(DD, B, P);

  return {
    X: modMul(E, F, P),
    Y: modMul(G, H, P),
    Z: modMul(F, G, P),
    T: modMul(E, H, P),
  };
}

// ─── Scalar Multiplication ──────────────────────────────────────────────────
//
// Compute n * P (add P to itself n times) using the double-and-add algorithm.
//
// This is the elliptic curve equivalent of modular exponentiation:
//   - "Square" becomes "double"
//   - "Multiply" becomes "add"
//
// We scan the scalar n from high bit to low bit:
//   - For each bit: double the accumulator
//   - If the bit is 1: also add P
//
// For a 255-bit scalar, this takes ~255 doublings and ~127 additions
// (on average, half the bits are 1).
//
// WARNING: This naive implementation is NOT constant-time! A production
// implementation would use a Montgomery ladder or fixed-window method
// to avoid timing side channels. For educational purposes, clarity wins.

function scalarMul(n: bigint, point: ExtendedPoint): ExtendedPoint {
  // Reduce scalar modulo L (subgroup order)
  n = ((n % L) + L) % L;
  if (n === 0n) return IDENTITY;

  let result: ExtendedPoint = IDENTITY;
  let temp: ExtendedPoint = point;

  while (n > 0n) {
    if (n & 1n) {
      result = pointAdd(result, temp);
    }
    temp = pointDouble(temp);
    n >>= 1n;
  }
  return result;
}

// ─── Point Encoding ─────────────────────────────────────────────────────────
//
// Ed25519 encodes a point as 32 bytes:
//   1. Normalize: compute affine coordinates x = X/Z, y = Y/Z
//   2. Encode y as 32 bytes in little-endian
//   3. Set the high bit of the last byte to the low bit of x (the "sign")
//
// Why encode y and store x's sign?
// On the curve, each y value has at most two corresponding x values
// (x and -x, which have different low bits since p is odd). So we can
// recover x from y and one extra bit. This gives us point compression:
// 32 bytes instead of 64.
//
// Little-endian encoding matches the convention of Curve25519/X25519.

function encodePoint(point: ExtendedPoint): Uint8Array {
  const zInv = modInv(point.Z, P);
  const x = modMul(point.X, zInv, P);
  const y = modMul(point.Y, zInv, P);

  const encoded = new Uint8Array(32);
  let yy = y;
  for (let i = 0; i < 32; i++) {
    encoded[i] = Number(yy & 0xFFn);
    yy >>= 8n;
  }
  // Set high bit of last byte to low bit of x (the "sign" bit)
  encoded[31] |= Number((x & 1n) << 7n);
  return encoded;
}

// ─── Point Decoding ─────────────────────────────────────────────────────────
//
// Decode a 32-byte compressed point back to extended coordinates.
//
// Steps:
//   1. Extract the sign bit from the high bit of byte 31
//   2. Decode y from the remaining 255 bits (little-endian)
//   3. Compute x^2 = (y^2 - 1) * inverse(d*y^2 + 1) mod p
//   4. Compute x = sqrt(x^2) using the field square root
//   5. If x's low bit doesn't match the sign bit, negate x
//   6. Return the point in extended coordinates
//
// This can fail if:
//   - y >= p (not a valid field element)
//   - x^2 is not a quadratic residue (no valid point with this y)
//   - y = 0 and sign bit is set with x = 0 (degenerate)

function decodePoint(bytes: Uint8Array): ExtendedPoint | null {
  if (bytes.length !== 32) return null;

  // Extract sign bit (high bit of last byte)
  const sign = (bytes[31] >> 7) & 1;

  // Decode y (little-endian, 255 bits)
  let y = 0n;
  for (let i = 31; i >= 0; i--) {
    y = (y << 8n) | BigInt(bytes[i]);
  }
  // Clear the sign bit from y
  y &= (1n << 255n) - 1n;

  if (y >= P) return null;

  // Compute x^2 = (y^2 - 1) / (d*y^2 + 1) mod p
  const y2 = modMul(y, y, P);
  const numerator = modSub(y2, 1n, P);
  const denominator = modAdd(modMul(D, y2, P), 1n, P);
  const x2 = modMul(numerator, modInv(denominator, P), P);

  // x^2 = 0 => x = 0
  if (x2 === 0n) {
    if (sign !== 0) return null; // sign bit set but x = 0
    return { X: 0n, Y: y, Z: 1n, T: 0n };
  }

  // Compute x = sqrt(x^2)
  let x = fieldSqrt(x2);
  if (x === null) return null;

  // Correct sign: if x's low bit doesn't match the sign bit, negate x
  if (Number(x & 1n) !== sign) {
    x = modNeg(x, P);
  }

  return { X: x, Y: y, Z: 1n, T: modMul(x, y, P) };
}

// ─── Byte/Hex Helpers ───────────────────────────────────────────────────────

/** Convert a hex string to Uint8Array. */
function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

/** Convert Uint8Array to lowercase hex string. */
function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

/** Decode a 32-byte little-endian integer from bytes. */
function bytesToScalar(bytes: Uint8Array): bigint {
  let n = 0n;
  for (let i = bytes.length - 1; i >= 0; i--) {
    n = (n << 8n) | BigInt(bytes[i]);
  }
  return n;
}

/** Encode a scalar as 32 bytes little-endian. */
function scalarToBytes(n: bigint): Uint8Array {
  const bytes = new Uint8Array(32);
  let val = ((n % L) + L) % L;
  for (let i = 0; i < 32; i++) {
    bytes[i] = Number(val & 0xFFn);
    val >>= 8n;
  }
  return bytes;
}

// ─── Key Clamping ───────────────────────────────────────────────────────────
//
// Ed25519 "clamps" the private scalar derived from SHA-512(seed):
//
//   1. Clear the lowest 3 bits → makes the scalar a multiple of 8
//      (cofactor of the curve). This ensures the public key is in
//      the prime-order subgroup, preventing small-subgroup attacks.
//
//   2. Clear bit 255 → keeps the scalar below 2^255
//
//   3. Set bit 254 → ensures the scalar has a fixed bit length.
//      This makes the scalar multiplication take the same number of
//      steps regardless of the secret key, reducing timing leaks.
//
// After clamping, the scalar is in the range [2^254, 2^255) and divisible by 8.

function clampScalar(hash: Uint8Array): Uint8Array {
  const clamped = new Uint8Array(32);
  clamped.set(hash.subarray(0, 32));
  clamped[0] &= 248;   // Clear lowest 3 bits (multiple of 8)
  clamped[31] &= 127;  // Clear bit 255
  clamped[31] |= 64;   // Set bit 254
  return clamped;
}

// ─── Public API ─────────────────────────────────────────────────────────────

/**
 * Generate an Ed25519 keypair from a 32-byte seed.
 *
 * The seed is the true secret. It is expanded via SHA-512 into:
 *   - First 32 bytes → clamped scalar `a` (the private scalar)
 *   - Last 32 bytes → nonce prefix (used during signing)
 *
 * The public key is A = a * B (scalar multiplication of base point).
 *
 * Returns:
 *   - publicKey: 32 bytes (compressed point A)
 *   - secretKey: 64 bytes (seed || publicKey) -- needed for signing
 *
 * @param seed - 32-byte random seed (keep this secret!)
 * @returns { publicKey, secretKey }
 *
 * @example
 * ```ts
 * const seed = crypto.getRandomValues(new Uint8Array(32));
 * const { publicKey, secretKey } = generateKeypair(seed);
 * ```
 */
export function generateKeypair(seed: Uint8Array): {
  publicKey: Uint8Array;
  secretKey: Uint8Array;
} {
  const hash = sha512(seed);
  const clamped = clampScalar(hash);
  const a = bytesToScalar(clamped);

  // A = a * B
  const A = scalarMul(a, BASE_POINT);
  const publicKey = encodePoint(A);

  // Secret key = seed || public key (64 bytes)
  const secretKey = new Uint8Array(64);
  secretKey.set(seed, 0);
  secretKey.set(publicKey, 32);

  return { publicKey, secretKey };
}

/**
 * Sign a message with an Ed25519 secret key.
 *
 * The signing process is deterministic -- the same message and key always
 * produce the same signature. This eliminates the catastrophic failure mode
 * of ECDSA where a weak random nonce leaks the private key.
 *
 * Algorithm:
 *   1. Hash the seed: h = SHA-512(seed)
 *   2. Clamp h[0..31] to get scalar a
 *   3. Use h[32..63] as the nonce prefix
 *   4. r = SHA-512(prefix || message) mod L   (deterministic nonce)
 *   5. R = r * B                               (commitment point)
 *   6. k = SHA-512(R || A || message) mod L   (challenge)
 *   7. S = (r + k * a) mod L                  (response)
 *   8. Return R || S (64 bytes)
 *
 * @param message - The message to sign (arbitrary bytes)
 * @param secretKey - 64-byte secret key from generateKeypair
 * @returns 64-byte signature (R || S)
 */
export function sign(message: Uint8Array, secretKey: Uint8Array): Uint8Array {
  // Extract seed and public key from secret key
  const seed = secretKey.subarray(0, 32);
  const publicKey = secretKey.subarray(32, 64);

  // Hash the seed
  const hash = sha512(seed);
  const clamped = clampScalar(hash);
  const a = bytesToScalar(clamped);
  const prefix = hash.subarray(32, 64);

  // Step 1: Deterministic nonce r = SHA-512(prefix || message) mod L
  //
  // This is the critical security innovation of EdDSA over ECDSA:
  // the nonce is derived from the secret key and message, not from
  // a random number generator. Even if the RNG is broken, the nonce
  // is unpredictable to an attacker who doesn't know the secret key.
  const rHash = sha512(concat(prefix, message));
  const r = bytesToScalar(rHash) % L;

  // Step 2: R = r * B (commitment)
  const R = encodePoint(scalarMul(r, BASE_POINT));

  // Step 3: k = SHA-512(R || A || message) mod L (challenge)
  const kHash = sha512(concat(R, publicKey, message));
  const k = bytesToScalar(kHash) % L;

  // Step 4: S = (r + k * a) mod L (response)
  const S = ((r + k * a) % L + L) % L;

  // Signature = R || S (32 + 32 = 64 bytes)
  const signature = new Uint8Array(64);
  signature.set(R, 0);
  signature.set(scalarToBytes(S), 32);
  return signature;
}

/**
 * Verify an Ed25519 signature.
 *
 * Verification checks the equation:
 *   S * B = R + SHA-512(R || A || message) * A
 *
 * This works because the signer computed S = r + k*a, so:
 *   S * B = (r + k*a) * B = r*B + k*a*B = R + k*A  ✓
 *
 * Security note: we must verify that R and A are valid curve points,
 * and that S is in the range [0, L). A malicious signature with
 * S >= L could pass verification but wouldn't be canonical.
 *
 * @param message - The message that was signed
 * @param signature - 64-byte signature (R || S)
 * @param publicKey - 32-byte public key
 * @returns true if the signature is valid
 */
export function verify(
  message: Uint8Array,
  signature: Uint8Array,
  publicKey: Uint8Array,
): boolean {
  if (signature.length !== 64) return false;
  if (publicKey.length !== 32) return false;

  // Decode R (first 32 bytes of signature)
  const Rbytes = signature.subarray(0, 32);
  const Rpoint = decodePoint(Rbytes);
  if (!Rpoint) return false;

  // Decode S (last 32 bytes of signature)
  const S = bytesToScalar(signature.subarray(32, 64));
  if (S >= L) return false; // S must be reduced mod L

  // Decode public key A
  const Apoint = decodePoint(publicKey);
  if (!Apoint) return false;

  // Compute k = SHA-512(R || A || message) mod L
  const kHash = sha512(concat(Rbytes, publicKey, message));
  const k = bytesToScalar(kHash) % L;

  // Check: S * B == R + k * A
  //
  // We rearrange to avoid computing both sides independently:
  //   S * B - k * A should equal R
  //
  // But it's cleaner to just compute both sides and compare the
  // encoded points (which are canonical).
  const lhs = scalarMul(S, BASE_POINT);
  const rhs = pointAdd(Rpoint, scalarMul(k, Apoint));

  return bytesToHex(encodePoint(lhs)) === bytesToHex(encodePoint(rhs));
}

// ─── Internal Helpers ───────────────────────────────────────────────────────

/** Concatenate multiple Uint8Arrays into one. */
function concat(...arrays: Uint8Array[]): Uint8Array {
  const totalLen = arrays.reduce((sum, a) => sum + a.length, 0);
  const result = new Uint8Array(totalLen);
  let offset = 0;
  for (const arr of arrays) {
    result.set(arr, offset);
    offset += arr.length;
  }
  return result;
}

// Re-export helpers for testing
export { hexToBytes, bytesToHex };
