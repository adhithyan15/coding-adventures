/**
 * @module gf256
 *
 * Galois Field GF(2^8) arithmetic.
 *
 * GF(256) is the finite field with 256 elements. The elements are the integers
 * 0..255. Arithmetic uses the primitive polynomial:
 *
 *   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
 *
 * Applications:
 *   - Reed-Solomon error correction (QR codes, CDs, hard drives)
 *   - AES encryption (SubBytes and MixColumns steps use GF(2^8))
 *   - General error-correcting codes
 *
 * Key insight: In GF(2^8), addition IS XOR. Every element is its own
 * additive inverse, so subtraction equals addition.
 *
 * Multiplication is performed via logarithm/antilogarithm tables:
 *   a × b = ALOG[(LOG[a] + LOG[b]) mod 255]
 * which turns multiplication into two table lookups and an addition.
 */

export const VERSION = "0.1.0";

/**
 * GF256 is an integer in the range [0, 255] representing a field element.
 *
 * Each byte represents the polynomial:
 *   b₇x⁷ + b₆x⁶ + ... + b₁x + b₀
 * where each bᵢ ∈ {0, 1} is a bit of the byte.
 */
export type GF256 = number;

/** Additive identity. */
export const ZERO: GF256 = 0;

/** Multiplicative identity. */
export const ONE: GF256 = 1;

/**
 * The primitive (irreducible) polynomial used for modular reduction.
 *
 * p(x) = x^8 + x^4 + x^3 + x^2 + 1
 *
 * In binary: bit 8 = x^8, bit 4 = x^4, bit 3 = x^3, bit 2 = x^2, bit 0 = 1.
 * Binary: 1_0001_1101 = 0x11D = 285.
 *
 * This polynomial is irreducible over GF(2) — it cannot be factored into
 * two lower-degree polynomials. Irreducibility ensures every non-zero element
 * has a multiplicative inverse, making GF(256) a field.
 *
 * It is also primitive — the element g=2 (the polynomial x) generates the
 * full multiplicative group of order 255. This means g^0, g^1, ..., g^254
 * are all 255 non-zero elements of GF(256).
 */
export const PRIMITIVE_POLYNOMIAL = 0x11d;

// =============================================================================
// Log/Antilog Table Construction
// =============================================================================
//
// We precompute two lookup tables at module load time:
//
//   ALOG[i] = g^i mod p(x)   where g = 2 (the generator)
//   LOG[x]  = i such that g^i = x
//
// Table sizes:
//   ALOG: 255 entries (g^0 through g^254; g^255 = g^0 = 1 wraps around)
//   LOG:  256 entries (LOG[0] is undefined/unused; LOG[1]..LOG[255] are valid)
//
// Construction algorithm:
//   Start with value = 1.
//   Each step: multiply by 2 (shift left 1 bit).
//   If the result overflows a byte (bit 8 set), XOR with 0x11D to reduce
//   modulo the primitive polynomial.
//
// Why shift-left = multiply by 2?
//   In GF(2^8), the element "2" is the polynomial x (bit 1 set, all else zero).
//   Multiplying any polynomial f(x) by x shifts all its coefficients up by one
//   degree — which is a left bit-shift. If the degree-8 coefficient becomes 1
//   (i.e., the byte overflows), we reduce modulo p(x) by XOR-ing with 0x11D.

const _LOG: number[] = new Array(256).fill(0);
// ALOG has 256 entries: indices 0..254 are the standard table;
// index 255 is 1 (the multiplicative group wraps: g^255 = g^0 = 1).
// This allows inverse(1) = ALOG[255-0] = ALOG[255] = 1 to work correctly.
const _ALOG: number[] = new Array(256).fill(0);

(function buildTables() {
  let val = 1;
  for (let i = 0; i < 255; i++) {
    _ALOG[i] = val;
    _LOG[val] = i;

    // Multiply val by 2 (the generator g = x).
    val <<= 1;
    // If bit 8 is set, reduce modulo the primitive polynomial.
    if (val >= 256) {
      val ^= PRIMITIVE_POLYNOMIAL;
    }
  }
  // _ALOG[255] = 1: the multiplicative group has order 255, so g^255 = g^0 = 1.
  // This is needed by inverse(1): 255 - LOG[1] = 255 - 0 = 255 → ALOG[255] = 1. ✓
  _ALOG[255] = 1;
  // _LOG[0] is left as 0 — it is never accessed for valid inputs.
})();

/**
 * Antilogarithm table: ALOG[i] = 2^i in GF(256).
 *
 * Used to convert from the discrete-logarithm domain back to field elements.
 * ALOG is a bijection from {0..254} to {1..255} (the non-zero elements).
 *
 * Notable entries:
 *   ALOG[0]  = 1    (2^0 = 1)
 *   ALOG[1]  = 2    (2^1 = 2)
 *   ALOG[7]  = 128  (2^7 = 0x80)
 *   ALOG[8]  = 29   (256 XOR 0x11D = 0x1D = 29; first reduction step)
 */
export const ALOG: ReadonlyArray<number> = _ALOG;

/**
 * Logarithm table: LOG[x] = i such that 2^i = x in GF(256).
 *
 * LOG[0] is undefined (there is no power of 2 that equals 0 in GF(256)).
 * For x in 1..255: ALOG[LOG[x]] = x.
 *
 * This table is the inverse of ALOG.
 */
export const LOG: ReadonlyArray<number> = _LOG;

// =============================================================================
// Field Operations
// =============================================================================

/**
 * Add two GF(256) elements.
 *
 * In a characteristic-2 field, addition is XOR. This is because each bit
 * represents a GF(2) coefficient, and GF(2) addition is 1+1=0 (mod 2).
 *
 * No overflow, no carry, no tables needed.
 *
 *   add(0x53, 0xCA) = 0x53 XOR 0xCA = 0x99
 *   add(x, x) = 0 for all x  (every element is its own inverse)
 */
export function add(a: GF256, b: GF256): GF256 {
  return a ^ b;
}

/**
 * Subtract two GF(256) elements.
 *
 * In characteristic 2, subtraction equals addition (since -1 = 1).
 * This is the same as XOR.
 *
 * This design simplifies error-correction algorithms: a "syndrome" computed
 * via subtraction uses the same hardware/logic as addition.
 */
export function subtract(a: GF256, b: GF256): GF256 {
  return a ^ b;
}

/**
 * Multiply two GF(256) elements using logarithm/antilogarithm tables.
 *
 * The mathematical identity: a × b = g^(log_g(a) + log_g(b))
 * Where g = 2 is our generator.
 *
 * Special case: if either operand is 0, the result is 0.
 * (Zero has no logarithm; it is not reachable as a power of g.)
 *
 * The modular addition (LOG[a] + LOG[b]) % 255 keeps the exponent within
 * the cyclic group of order 255.
 *
 * Time complexity: O(1) — two table lookups and one addition.
 */
export function multiply(a: GF256, b: GF256): GF256 {
  // The product of anything with zero is zero.
  if (a === 0 || b === 0) return 0;
  return _ALOG[(_LOG[a] + _LOG[b]) % 255];
}

/**
 * Divide a by b in GF(256).
 *
 * a / b = g^(log_g(a) - log_g(b)) = ALOG[(LOG[a] - LOG[b] + 255) % 255]
 *
 * The `+ 255` before the modulo ensures the result is non-negative
 * when LOG[a] < LOG[b]. Without it, JavaScript's `%` operator could
 * return a negative number.
 *
 * Special case: a = 0 → result is 0 (0 / anything = 0).
 *
 * @throws Error if b is 0 (division by zero is undefined in any field)
 */
export function divide(a: GF256, b: GF256): GF256 {
  if (b === 0) throw new Error("GF256: division by zero");
  if (a === 0) return 0;
  return _ALOG[(_LOG[a] - _LOG[b] + 255) % 255];
}

/**
 * Raise a GF(256) element to a non-negative integer power.
 *
 * Uses the logarithm table:
 *   base^exp = ALOG[(LOG[base] * exp) % 255]
 *
 * The modulo 255 reflects the order of the multiplicative group:
 * every non-zero element satisfies g^255 = 1 (Fermat's little theorem
 * for finite fields).
 *
 * Special cases:
 *   0^0 = 1 by convention (consistent with most numeric libraries)
 *   0^n = 0 for n > 0
 *
 * Note: For very large `exp`, the computation `LOG[base] * exp` may
 * overflow a 32-bit integer if exp > 2^23. Use modular arithmetic on
 * exp first if very large exponents are needed.
 */
export function power(base: GF256, exp: number): GF256 {
  if (base === 0) return exp === 0 ? 1 : 0;
  if (exp === 0) return 1;
  return _ALOG[((_LOG[base] * exp) % 255 + 255) % 255];
}

/**
 * Compute the multiplicative inverse of a GF(256) element.
 *
 * The inverse of a satisfies: a × inverse(a) = 1.
 *
 * By the cyclic group property:
 *   a × a^(-1) = 1 = g^0 = g^255
 *   So log(a) + log(a^(-1)) ≡ 0 (mod 255)
 *   Therefore log(a^(-1)) = 255 - log(a)
 *   And a^(-1) = ALOG[255 - LOG[a]]
 *
 * This operation is fundamental to Reed-Solomon decoding and AES SubBytes.
 *
 * @throws Error if a is 0 (zero has no multiplicative inverse)
 */
export function inverse(a: GF256): GF256 {
  if (a === 0) throw new Error("GF256: zero has no multiplicative inverse");
  return _ALOG[255 - _LOG[a]];
}

/**
 * Return the additive identity (zero element).
 */
export function zero(): GF256 {
  return 0;
}

/**
 * Return the multiplicative identity (one element).
 */
export function one(): GF256 {
  return 1;
}

// =============================================================================
// GF256Field — parameterizable field factory
// =============================================================================
//
// The functions above are fixed to the Reed-Solomon polynomial 0x11D.
// AES uses the polynomial 0x11B. `createField` builds an independent field
// object for any primitive polynomial, reusing the same algorithm.
//
// Usage:
//   const aes = createField(0x11B);
//   aes.multiply(0x53, 0xCA);  // → 1  (AES GF(2^8) inverses)

/**
 * A GF(2^8) field configured for a specific primitive polynomial.
 *
 * Instances are created via `createField`. All operations are O(1) table lookups.
 */
export interface GF256FieldInstance {
  readonly polynomial: number;
  add(a: GF256, b: GF256): GF256;
  subtract(a: GF256, b: GF256): GF256;
  multiply(a: GF256, b: GF256): GF256;
  divide(a: GF256, b: GF256): GF256;
  power(base: GF256, exp: number): GF256;
  inverse(a: GF256): GF256;
}

/**
 * Create a GF(2^8) field for the given primitive polynomial.
 *
 * The module-level functions are fixed to the Reed-Solomon polynomial `0x11D`.
 * Use `createField(0x11B)` for the AES polynomial.
 *
 * @param polynomial The irreducible polynomial as an integer with the degree-8
 *   term included: `0x11B` for AES, `0x11D` for Reed-Solomon.
 * @returns An object with the same API as the module-level functions, but using
 *   tables built for `polynomial`.
 *
 * @example
 * ```ts
 * const aes = createField(0x11B);
 * aes.multiply(0x53, 0xCA);  // → 1   (AES GF(2^8) inverses)
 * aes.multiply(0x57, 0x83);  // → 0xC1  (FIPS 197 Appendix B)
 * ```
 */
export function createField(polynomial: number): GF256FieldInstance {
  // Russian peasant (shift-and-XOR) multiplication for GF(2^8).
  //
  // Log/antilog tables require a *primitive* generator g such that g^1..g^255
  // visits all 255 non-zero elements. g=2 works for 0x11D (Reed-Solomon) but
  // is NOT primitive for 0x11B (AES uses g=0x03 per FIPS 197 §4.1). Using
  // g=2 with 0x11B leaves most log entries at 0, producing wrong results.
  //
  // Russian peasant multiplication needs no generator assumption:
  //   for each bit of b (LSB first):
  //     if bit set: result ^= a
  //     carry = a & 0x80; a = (a << 1) & 0xFF
  //     if carry: a ^= reduce   (reduce = polynomial & 0xFF = low-byte constant)
  const reduce = polynomial & 0xFF;

  function gfMul(a: GF256, b: GF256): GF256 {
    let result = 0;
    let aa = a;
    let bb = b;
    for (let i = 0; i < 8; i++) {
      if (bb & 1) result ^= aa;
      const hi = aa & 0x80;
      aa = (aa << 1) & 0xFF;
      if (hi) aa ^= reduce;
      bb >>= 1;
    }
    return result;
  }

  // Raise base to exp via repeated squaring.
  // inverse(a) = power(a, 254) since a^255 = 1 in GF(2^8).
  function gfPow(base: GF256, exp: number): GF256 {
    if (base === 0) return exp === 0 ? 1 : 0;
    if (exp === 0) return 1;
    let result = 1;
    let b = base;
    let e = exp;
    while (e > 0) {
      if (e & 1) result = gfMul(result, b);
      b = gfMul(b, b);
      e >>= 1;
    }
    return result;
  }

  return {
    polynomial,

    // add/subtract are polynomial-independent (always XOR); included for symmetry.
    add(a: GF256, b: GF256): GF256 { return a ^ b; },
    subtract(a: GF256, b: GF256): GF256 { return a ^ b; },

    multiply(a: GF256, b: GF256): GF256 {
      return gfMul(a, b);
    },

    divide(a: GF256, b: GF256): GF256 {
      if (b === 0) throw new Error("GF256Field: division by zero");
      return gfMul(a, gfPow(b, 254));
    },

    power(base: GF256, exp: number): GF256 {
      if (!Number.isInteger(exp) || exp < 0) {
        throw new Error("GF256Field: exponent must be a non-negative integer");
      }
      return gfPow(base, exp);
    },

    inverse(a: GF256): GF256 {
      if (a === 0) throw new Error("GF256Field: zero has no multiplicative inverse");
      return gfPow(a, 254);
    },
  };
}
