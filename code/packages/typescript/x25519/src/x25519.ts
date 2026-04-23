// ============================================================================
// x25519.ts — X25519 Elliptic Curve Diffie-Hellman (RFC 7748)
// ============================================================================
//
// X25519 is the Diffie-Hellman function on Curve25519, one of the most widely
// used key agreement protocols in modern cryptography. It is used in TLS 1.3,
// SSH, Signal, WireGuard, and many other protocols.
//
// The beauty of X25519 lies in its simplicity: the entire key exchange reduces
// to a single scalar multiplication on an elliptic curve, using only the
// x-coordinate (hence "X" 25519). This is the Montgomery ladder algorithm,
// which is naturally constant-time — a critical property for cryptographic
// implementations.
//
// ## The Math: Curve25519
//
// Curve25519 is a Montgomery curve defined by:
//
//   y^2 = x^3 + 486662x^2 + x   (mod p)
//
// where p = 2^255 - 19 (a prime). The constant 486662 is the curve parameter A.
// The constant a24 = (A - 2) / 4 = 121665 appears in the ladder formulas.
//
// ## Field Arithmetic: GF(2^255 - 19)
//
// All arithmetic happens in GF(p) where p = 2^255 - 19. This is a prime field,
// so we can use standard modular arithmetic. TypeScript's native BigInt gives
// us arbitrary-precision integers, making this straightforward.
//
// ## Montgomery Ladder
//
// The Montgomery ladder computes scalar multiplication Q = [k]P using only
// the x-coordinate of points. It processes the scalar k bit by bit from the
// top, maintaining two points that always differ by P. This differential
// addition chain is naturally constant-time: every bit does the same operations.
//
// ============================================================================

// ---------------------------------------------------------------------------
// The prime field: p = 2^255 - 19
// ---------------------------------------------------------------------------
// This prime was chosen by Daniel Bernstein because:
// 1. It's very close to a power of 2, making reduction fast
// 2. 19 is small, so the "correction" after 2^255 is cheap
// 3. The resulting curve has excellent security properties

const P = (1n << 255n) - 19n;

// ---------------------------------------------------------------------------
// The curve constant a24 = 121665
// ---------------------------------------------------------------------------
// The Montgomery curve y^2 = x^3 + Ax^2 + x has A = 486662.
// The ladder formulas use a24 = (A - 2) / 4 = (486662 - 2) / 4 = 121665.
// The ladder formula computes:
//
//   z_2 = E * (AA + a24 * E)
//
// where E = AA - BB, and AA + a24 * E = AA + ((A-2)/4)(AA - BB)
// which equals ((A+2)*AA + (A-6)*BB) / 4 — the standard differential
// addition formula for Montgomery curves.

const A24 = 121665n;

// ---------------------------------------------------------------------------
// The base point: u = 9
// ---------------------------------------------------------------------------
// The base point for Curve25519 has x-coordinate (u-coordinate) = 9.
// This generates a large prime-order subgroup suitable for Diffie-Hellman.

const BASE_POINT = new Uint8Array(32);
BASE_POINT[0] = 9;

// ---------------------------------------------------------------------------
// Field operations: the building blocks
// ---------------------------------------------------------------------------
// All operations are modulo p. We use TypeScript BigInt which handles
// arbitrary precision automatically.

/**
 * Modular addition: (a + b) mod p
 *
 * Simple addition followed by reduction. Since a, b < p, the sum is at most
 * 2p - 2, so a single subtraction suffices if the result >= p.
 */
function fieldAdd(a: bigint, b: bigint): bigint {
  return (a + b) % P;
}

/**
 * Modular subtraction: (a - b) mod p
 *
 * We add p before subtracting to ensure the result is non-negative.
 * Since a < p and b < p, we have (a - b + p) in range [1, 2p-1],
 * so a single mod p brings it into [0, p-1].
 */
function fieldSub(a: bigint, b: bigint): bigint {
  return (a - b + P) % P;
}

/**
 * Modular multiplication: (a * b) mod p
 *
 * BigInt handles the multi-precision multiplication; we just reduce mod p.
 */
function fieldMul(a: bigint, b: bigint): bigint {
  return (a * b) % P;
}

/**
 * Modular squaring: a^2 mod p
 *
 * A dedicated function because squaring can be optimized (though with BigInt,
 * the runtime handles this). Squaring is the most frequent operation in the
 * inversion routine, so having a named function aids readability.
 */
function fieldSquare(a: bigint): bigint {
  return (a * a) % P;
}

/**
 * Modular exponentiation: base^exp mod p
 *
 * Uses the standard square-and-multiply algorithm (binary method).
 * Processes bits of the exponent from most significant to least significant.
 *
 * Time complexity: O(n) squarings and O(n/2) multiplications on average,
 * where n is the bit length of exp.
 *
 * For constant-time operation in production, you'd want a fixed-window or
 * Montgomery ladder exponentiation. For educational purposes, square-and-multiply
 * is clear and correct.
 */
function fieldPow(base: bigint, exp: bigint): bigint {
  let result = 1n;
  base = base % P;
  while (exp > 0n) {
    // If the current bit is 1, multiply result by base
    if (exp & 1n) {
      result = (result * base) % P;
    }
    // Square the base and shift to the next bit
    base = (base * base) % P;
    exp >>= 1n;
  }
  return result;
}

/**
 * Modular inverse: a^(-1) mod p
 *
 * Uses Fermat's little theorem: for prime p and a != 0,
 *   a^(p-1) ≡ 1 (mod p)
 *   therefore a^(p-2) ≡ a^(-1) (mod p)
 *
 * This is elegant: inversion reduces to exponentiation, which we already have.
 * The exponent is p - 2 = 2^255 - 21, a 255-bit number.
 *
 * Production implementations use addition chains optimized for this specific
 * exponent to minimize the number of multiplications. For clarity, we use
 * generic modular exponentiation here.
 */
function fieldInvert(a: bigint): bigint {
  return fieldPow(a, P - 2n);
}

// ---------------------------------------------------------------------------
// Byte encoding/decoding: the wire format
// ---------------------------------------------------------------------------
// X25519 uses little-endian byte encoding for both scalars and field elements.
// A 32-byte array represents a 256-bit number with the least significant byte
// first. This matches the convention on most modern processors (x86 is LE).

/**
 * Decode a 32-byte little-endian array into a BigInt.
 *
 * Byte 0 is the least significant, byte 31 is the most significant.
 * Example: [0x09, 0x00, ..., 0x00] decodes to 9.
 */
function decodeLittleEndian(bytes: Uint8Array): bigint {
  let result = 0n;
  for (let i = 31; i >= 0; i--) {
    result = (result << 8n) | BigInt(bytes[i]);
  }
  return result;
}

/**
 * Encode a BigInt as a 32-byte little-endian array.
 *
 * The value is reduced mod p first (though callers should ensure this).
 * Only the low 256 bits are encoded.
 */
function encodeLittleEndian(n: bigint): Uint8Array {
  const result = new Uint8Array(32);
  let value = n;
  for (let i = 0; i < 32; i++) {
    result[i] = Number(value & 0xffn);
    value >>= 8n;
  }
  return result;
}

/**
 * Decode a u-coordinate from 32 bytes.
 *
 * Per RFC 7748 Section 5, the high bit of the last byte is masked off.
 * This ensures the u-coordinate is in range [0, 2^255 - 1], which is then
 * reduced mod p during field operations.
 *
 * Why mask the high bit? The field is GF(2^255 - 19), so valid elements
 * need at most 255 bits. The 256th bit (bit 255) would put us above p,
 * and masking it ensures interoperability — different implementations agree
 * on the canonical input even if that bit is set randomly.
 */
function decodeUCoordinate(bytes: Uint8Array): bigint {
  const copy = new Uint8Array(bytes);
  copy[31] &= 0x7f; // Mask the high bit (bit 255)
  return decodeLittleEndian(copy);
}

// ---------------------------------------------------------------------------
// Scalar clamping: preparing the private key
// ---------------------------------------------------------------------------
// Before using a 32-byte secret key as a scalar multiplier, we "clamp" it:
//
//   k[0]  &= 248   — Clear the low 3 bits (makes k a multiple of 8)
//   k[31] &= 127   — Clear the high bit (bit 255)
//   k[31] |= 64    — Set bit 254
//
// Why?
//
// 1. Clearing low 3 bits (multiple of 8): The cofactor of Curve25519 is 8.
//    By ensuring k is a multiple of 8, we guarantee that [k]P lands in the
//    prime-order subgroup, regardless of whether P is in the subgroup.
//    This prevents small-subgroup attacks without needing to validate P.
//
// 2. Setting bit 254: This ensures the scalar always has the same bit length,
//    so the Montgomery ladder always runs the same number of iterations.
//    This is a defense against timing side channels — the computation time
//    doesn't leak information about the scalar's bit length.
//
// 3. Clearing bit 255: The scalar need only be 255 bits for a 255-bit curve.
//    Combined with setting bit 254, this puts k in [2^254, 2^255 - 1].

function clampScalar(k: Uint8Array): bigint {
  const clamped = new Uint8Array(k);
  clamped[0] &= 248;
  clamped[31] &= 127;
  clamped[31] |= 64;
  return decodeLittleEndian(clamped);
}

// ---------------------------------------------------------------------------
// Conditional swap (cswap): constant-time selection
// ---------------------------------------------------------------------------
// The Montgomery ladder needs to swap two values conditionally based on a bit.
// In a real constant-time implementation, this would use bitwise masking to
// avoid branches. With BigInt, true constant-time is hard to guarantee (the
// runtime may take different paths for different values), but we implement
// the correct logic.
//
// swap = 0: return [a, b] unchanged
// swap = 1: return [b, a] swapped

function cswap(
  swap: bigint,
  a: bigint,
  b: bigint,
): [bigint, bigint] {
  // In a constant-time implementation, you'd compute:
  //   mask = -swap  (all 1s if swap=1, all 0s if swap=0)
  //   dummy = mask & (a ^ b)
  //   a ^= dummy
  //   b ^= dummy
  // This avoids branching. For educational clarity with BigInt:
  const mask = -swap; // 0n or -1n
  const dummy = mask & (a ^ b);
  return [a ^ dummy, b ^ dummy];
}

// ---------------------------------------------------------------------------
// The Montgomery Ladder: the heart of X25519
// ---------------------------------------------------------------------------
// This is the core algorithm that computes [k]u on the Montgomery curve
// using only x-coordinates (u-coordinates in Montgomery terminology).
//
// The ladder maintains two points:
//   (x_2, z_2) — one of the points in projective coordinates
//   (x_3, z_3) — the other point
//
// These points always differ by the base point u. At each step, we either:
//   - Double one and add the other, or
//   - Add one and double the other
//
// The choice depends on the current bit of the scalar k. After processing
// all bits, (x_2, z_2) holds the result in projective coordinates.
// The final affine x-coordinate is x_2 * z_2^(-1) mod p.
//
// ## Why projective coordinates?
//
// In affine coordinates (x, y), every point operation requires a field
// inversion (computing 1/z mod p), which is expensive. Projective coordinates
// represent a point as (X : Z) where x = X/Z, deferring the inversion to
// the very end. This turns ~255 inversions into just 1.

/**
 * Perform the X25519 function: scalar multiplication on Curve25519.
 *
 * @param scalar - 32-byte private scalar (will be clamped)
 * @param uBytes - 32-byte u-coordinate of the input point
 * @returns 32-byte u-coordinate of the resulting point
 * @throws Error if the result is the all-zeros point (identity/low-order)
 */
export function x25519(scalar: Uint8Array, uBytes: Uint8Array): Uint8Array {
  if (scalar.length !== 32) {
    throw new Error("Scalar must be exactly 32 bytes");
  }
  if (uBytes.length !== 32) {
    throw new Error("U-coordinate must be exactly 32 bytes");
  }

  // Step 1: Clamp the scalar and decode the u-coordinate
  const k = clampScalar(scalar);
  const u = decodeUCoordinate(uBytes);

  // Step 2: Initialize the Montgomery ladder
  //
  // We maintain two points in projective coordinates:
  //   Point 2: (x_2 : z_2) = (1 : 0) — the identity (point at infinity)
  //   Point 3: (x_3 : z_3) = (u : 1) — the input point
  //
  // The invariant is: Point 3 - Point 2 = (u : 1) at all times.

  const x_1 = u; // Fixed: the base point's x-coordinate
  let x_2 = 1n;
  let z_2 = 0n;
  let x_3 = u;
  let z_3 = 1n;
  let swap = 0n;

  // Step 3: Process each bit of k from bit 254 down to bit 0
  //
  // Why start at bit 254? After clamping, bit 254 is always 1, and bit 255
  // is always 0. So we start at the highest meaningful bit.
  //
  // At each step:
  //   1. Conditionally swap based on the XOR of this bit and the previous swap
  //   2. Compute the differential addition and doubling formulas
  //   3. Record this bit for the next swap decision

  for (let i = 254; i >= 0; i--) {
    // Extract bit i of the scalar
    const k_i = (k >> BigInt(i)) & 1n;

    // Conditional swap: if k_i differs from the previous bit, swap the points
    swap ^= k_i;
    [x_2, x_3] = cswap(swap, x_2, x_3);
    [z_2, z_3] = cswap(swap, z_2, z_3);
    swap = k_i;

    // --- The Montgomery ladder step ---
    //
    // These formulas compute a simultaneous doubling of one point and
    // differential addition of both points, using the base point x_1.
    //
    // A = x_2 + z_2       — sum of projective coordinates
    // AA = A^2             — squared sum
    // B = x_2 - z_2       — difference of projective coordinates
    // BB = B^2             — squared difference
    // E = AA - BB          — this equals 4*x_2*z_2 (the "mixed" term)
    //
    // For the doubling (result in x_2, z_2):
    //   x_2 = AA * BB = (x_2 + z_2)^2 * (x_2 - z_2)^2
    //   z_2 = E * (AA + a24 * E)
    //
    // For the differential addition (result in x_3, z_3):
    //   C = x_3 + z_3
    //   D = x_3 - z_3
    //   DA = D * A           — cross-multiply
    //   CB = C * B           — cross-multiply
    //   x_3 = (DA + CB)^2
    //   z_3 = x_1 * (DA - CB)^2

    const A = fieldAdd(x_2, z_2);
    const AA = fieldSquare(A);
    const B = fieldSub(x_2, z_2);
    const BB = fieldSquare(B);
    const E = fieldSub(AA, BB);

    const C = fieldAdd(x_3, z_3);
    const D = fieldSub(x_3, z_3);
    const DA = fieldMul(D, A);
    const CB = fieldMul(C, B);

    x_3 = fieldSquare(fieldAdd(DA, CB));
    z_3 = fieldMul(x_1, fieldSquare(fieldSub(DA, CB)));
    x_2 = fieldMul(AA, BB);
    z_2 = fieldMul(E, fieldAdd(AA, fieldMul(A24, E)));
  }

  // Step 4: Final conditional swap
  [x_2, x_3] = cswap(swap, x_2, x_3);
  [z_2, z_3] = cswap(swap, z_2, z_3);

  // Step 5: Convert from projective to affine coordinates
  //
  // The result is x_2 / z_2 mod p.
  // We compute z_2^(-1) using Fermat's little theorem: z_2^(p-2) mod p
  // Then multiply: result = x_2 * z_2^(-1) mod p

  const result = fieldMul(x_2, fieldInvert(z_2));
  const encoded = encodeLittleEndian(result);

  // Step 6: Check for the all-zeros result
  //
  // If the result is all zeros, the input point was a low-order point
  // (one of the small-subgroup elements). This would mean the shared
  // secret has no entropy, so we must reject it.

  let allZero = true;
  for (let i = 0; i < 32; i++) {
    if (encoded[i] !== 0) {
      allZero = false;
      break;
    }
  }
  if (allZero) {
    throw new Error(
      "X25519 produced all-zero output — input is a low-order point",
    );
  }

  return encoded;
}

/**
 * Multiply the scalar by the Curve25519 base point (u = 9).
 *
 * This is the standard operation for generating a public key from a private key.
 * The base point u = 9 generates a large prime-order subgroup of Curve25519.
 *
 * @param scalar - 32-byte private key
 * @returns 32-byte public key (u-coordinate of [scalar]G)
 */
export function x25519Base(scalar: Uint8Array): Uint8Array {
  return x25519(scalar, BASE_POINT);
}

/**
 * Generate a Curve25519 public key from a private key.
 *
 * This is an alias for x25519Base — the public key is [privateKey]G where
 * G is the base point with u-coordinate 9.
 *
 * @param privateKey - 32-byte private key (random bytes)
 * @returns 32-byte public key
 */
export function generateKeypair(privateKey: Uint8Array): Uint8Array {
  return x25519Base(privateKey);
}
