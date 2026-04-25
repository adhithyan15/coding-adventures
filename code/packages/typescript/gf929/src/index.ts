/**
 * @module gf929
 *
 * Galois Field GF(929) — prime field arithmetic for PDF417 barcodes.
 *
 * ## What is GF(929)?
 *
 * GF(929) is a **finite field** with exactly 929 elements. Because 929 is a
 * prime number, GF(929) is simply the integers modulo 929 — no polynomial
 * reduction or XOR tricks needed. This is fundamentally different from
 * GF(256), which is a binary extension field (polynomial arithmetic over GF(2)).
 *
 * Mathematically:
 * ```
 * GF(929) = ℤ/929ℤ = { 0, 1, 2, ..., 928 }
 * ```
 *
 * The 929 elements are plain integers. Addition and multiplication are ordinary
 * integer arithmetic, reduced modulo 929.
 *
 * ## Why 929?
 *
 * PDF417 uses 929 distinct codeword values (0–928). For Reed-Solomon error
 * correction to work, the field size must equal the codeword alphabet size.
 * Since 929 is prime, GF(929) exists and has exactly the right size.
 *
 * Verify that 929 is prime: it is not divisible by 2, 3, 5, 7, 11, 13, 17,
 * 19, 23, or 29. (29² = 841 < 929 < 961 = 31², so we only need to check
 * primes up to 29.)
 *
 * ## Field arithmetic
 *
 * ```
 * add(a, b)  = (a + b)       mod 929
 * sub(a, b)  = (a - b + 929) mod 929   ← +929 prevents negative results
 * mul(a, b)  = (a * b)       mod 929
 * inv(b)     = b^{927}       mod 929   ← Fermat's little theorem
 * div(a, b)  = a * inv(b)    mod 929
 * ```
 *
 * In any field of prime order p, Fermat's little theorem guarantees:
 * ```
 * b^{p-1} ≡ 1 (mod p)  for all b ≠ 0
 * b^{p-2} ≡ b^{-1} (mod p)
 * ```
 * For p = 929: b^{927} is the inverse of b.
 *
 * ## Generator element (primitive root)
 *
 * α = 3 is a primitive root of GF(929), meaning the powers
 * 3^0, 3^1, 3^2, ..., 3^{927} visit all 928 non-zero elements exactly once
 * before returning to 3^{928} = 3^0 = 1.
 *
 * This is specified in ISO/IEC 15438:2015, Annex A.4.
 *
 * ## Log/antilog tables
 *
 * For efficiency, we precompute:
 * ```
 * EXP[i] = 3^i mod 929   for i in 0..927  (928 entries)
 * LOG[v] = i such that 3^i = v   for v in 1..928  (index 0 = undefined)
 * ```
 *
 * Multiplication then becomes two table lookups and one addition:
 * ```
 * mul(a, b) = EXP[(LOG[a] + LOG[b]) mod 928]   (when a ≠ 0 and b ≠ 0)
 * ```
 *
 * This is the same idea as log tables for ordinary multiplication:
 * log(a × b) = log(a) + log(b). Here we work in the discrete-log domain
 * modulo 928 (the order of the multiplicative group).
 *
 * ## Applications
 *
 * GF(929) is used exclusively in PDF417 (and MicroPDF417) barcodes. The
 * Reed-Solomon encoder in the pdf417 package depends on this package for
 * all field arithmetic.
 */

export const VERSION = "0.1.0";

/**
 * GF929 is an integer in the range [0, 928] representing a field element.
 *
 * Every integer 0–928 is a valid element of GF(929). The field has 929
 * elements — one for each valid PDF417 codeword value.
 */
export type GF929 = number;

/** The prime modulus. GF(929) = ℤ/929ℤ. */
export const PRIME = 929;

/**
 * The order of the multiplicative group: |GF(929)*| = PRIME - 1 = 928.
 *
 * Every non-zero element satisfies a^{ORDER} ≡ 1 (mod PRIME).
 * The log/antilog tables cycle with this period.
 */
export const ORDER = 928;

/**
 * The primitive root (generator) α = 3.
 *
 * 3 is a primitive root of ℤ/929ℤ: the powers 3^0, 3^1, ..., 3^{927}
 * produce all 928 non-zero elements of GF(929) in some order.
 *
 * Specified in ISO/IEC 15438:2015, Annex A.4.
 *
 * Mnemonic: the generator for GF(256) is 2 (the polynomial x).
 * The generator for GF(929) is 3 (the integer three).
 */
export const ALPHA = 3;

// =============================================================================
// Log/Antilog Table Construction
// =============================================================================
//
// We precompute two lookup tables at module load time:
//
//   EXP[i] = ALPHA^i mod PRIME     for i = 0..927  (antilogarithm)
//   LOG[v]  = i such that ALPHA^i = v   for v = 1..928  (logarithm)
//
// Table sizes:
//   EXP: 929 entries. Indices 0..927 hold the standard table.
//        Index 928 is a convenience copy of EXP[0] = 1 (wrap-around support).
//   LOG: 929 entries. LOG[0] is undefined (0 has no discrete log).
//        LOG[1]..LOG[928] are valid.
//
// Construction algorithm:
//   Start with val = 1 (which is ALPHA^0 = 3^0 = 1).
//   Each step i: record EXP[i] = val, LOG[val] = i.
//   Then multiply val by ALPHA modulo PRIME.
//   After 928 steps, val cycles back to 1.
//
// Why ALPHA = 3 works as a primitive root:
//   For α to be a primitive root mod p, α^k mod p must ≠ 1 for all 0 < k < p-1.
//   Equivalently, the smallest k with α^k ≡ 1 (mod p) must equal p-1 = 928.
//   The ISO standard confirms α = 3 is a primitive root mod 929.

const _EXP: number[] = new Array(929).fill(0);
const _LOG: number[] = new Array(929).fill(0);

(function buildTables() {
  let val = 1; // Start at α^0 = 1.
  for (let i = 0; i < ORDER; i++) {
    _EXP[i] = val;
    _LOG[val] = i;
    // Advance: val = val * ALPHA mod PRIME.
    val = (val * ALPHA) % PRIME;
  }
  // Wrap-around convenience: EXP[928] = EXP[0] = 1.
  // This allows mul to use (LOG[a] + LOG[b]) % ORDER without a branch.
  _EXP[ORDER] = _EXP[0]; // = 1
  // _LOG[0] remains 0 — it is never accessed for valid computations.
})();

/**
 * Antilogarithm (exponent) table: EXP[i] = α^i mod 929.
 *
 * Maps from the discrete-log domain (exponent i) back to a field element.
 * EXP is a bijection from {0..927} to {1..928} (all non-zero elements).
 *
 * Notable entries:
 * ```
 * EXP[0]   = 1    (3^0 = 1)
 * EXP[1]   = 3    (3^1 = 3)
 * EXP[2]   = 9    (3^2 = 9)
 * EXP[3]   = 27   (3^3 = 27)
 * EXP[927] = ?    (3^{927} mod 929 — the last non-trivial element)
 * EXP[928] = 1    (convenience copy of EXP[0], for wrap-around in multiply)
 * ```
 */
export const EXP: ReadonlyArray<number> = _EXP;

/**
 * Logarithm table: LOG[v] = i such that α^i = v in GF(929).
 *
 * LOG[0] is undefined (zero has no discrete logarithm — it cannot be expressed
 * as a power of α). For v in 1..928: EXP[LOG[v]] = v.
 *
 * This table is the inverse of EXP (restricted to 1..928).
 */
export const LOG: ReadonlyArray<number> = _LOG;

// =============================================================================
// Field Operations
// =============================================================================
//
// All operations take and return integers in [0, 928].
// The type annotation GF929 = number is a documentation hint, not enforced
// at runtime by TypeScript. Callers are responsible for passing valid elements.

/**
 * Add two GF(929) elements.
 *
 * GF(929) addition is ordinary addition modulo 929. Unlike GF(256), where
 * addition is XOR, GF(929) addition can produce a carry and requires a modulo
 * reduction.
 *
 * The maximum input sum is 928 + 928 = 1856, which fits easily in a 32-bit
 * integer. No overflow risk.
 *
 * ```
 * add(100, 900) = (100 + 900) mod 929 = 1000 mod 929 = 71
 * add(0, 500)   = 500
 * add(928, 1)   = 0       (928 + 1 = 929 ≡ 0 mod 929)
 * ```
 */
export function add(a: GF929, b: GF929): GF929 {
  return (a + b) % PRIME;
}

/**
 * Subtract two GF(929) elements: a - b.
 *
 * To avoid negative intermediate results in JavaScript (which uses signed
 * 32-bit integers for bitwise operations but 64-bit floats for arithmetic),
 * we add PRIME before taking the modulo:
 *
 * ```
 * sub(a, b) = (a - b + 929) mod 929
 * ```
 *
 * This works because a - b + 929 is always in [1, 929+928] = [1, 1857],
 * so the modulo always yields a non-negative result.
 *
 * ```
 * sub(10, 5)  = 5
 * sub(5, 10)  = (5 - 10 + 929) mod 929 = 924
 * sub(0, 1)   = 928   (additive inverse of 1)
 * ```
 */
export function subtract(a: GF929, b: GF929): GF929 {
  return (a - b + PRIME) % PRIME;
}

/**
 * Multiply two GF(929) elements using logarithm/antilogarithm tables.
 *
 * The discrete-log identity: a × b = α^(log(a) + log(b))
 *
 * Addition in the exponent is modulo ORDER = 928 (the multiplicative group
 * order). The result is looked up in the EXP table.
 *
 * Special case: if either operand is 0, the result is 0.
 * Zero has no discrete logarithm (it is the additive identity, not reachable
 * as a power of α).
 *
 * Time complexity: O(1) — two table lookups and one modular addition.
 *
 * ```
 * mul(3, 3)    = 9     (= 3^2)
 * mul(100, 0)  = 0
 * mul(1, 500)  = 500
 * ```
 */
export function multiply(a: GF929, b: GF929): GF929 {
  if (a === 0 || b === 0) return 0;
  return _EXP[(_LOG[a] + _LOG[b]) % ORDER];
}

/**
 * Divide a by b in GF(929): a / b = a × b^{-1}.
 *
 * Division is multiplication by the inverse:
 * ```
 * div(a, b) = EXP[(LOG[a] - LOG[b] + 928) mod 928]
 * ```
 *
 * The + ORDER before the modulo prevents negative results when LOG[a] < LOG[b].
 *
 * Special case: a = 0 → result is 0 (zero divided by anything is zero).
 *
 * @throws Error if b is 0 (division by zero is undefined)
 *
 * ```
 * div(9, 3)  = 3    (9 / 3 = 3)
 * div(1, 3)  = inv(3) = 310   (verify: 3 × 310 = 930 ≡ 1 mod 929)
 * ```
 */
export function divide(a: GF929, b: GF929): GF929 {
  if (b === 0) throw new Error("GF929: division by zero");
  if (a === 0) return 0;
  return _EXP[(_LOG[a] - _LOG[b] + ORDER) % ORDER];
}

/**
 * Raise a GF(929) element to a non-negative integer power.
 *
 * Uses the logarithm table:
 * ```
 * pow(base, exp) = EXP[(LOG[base] × exp) mod 928]
 * ```
 *
 * The modulo 928 reflects the multiplicative group order — every non-zero
 * element satisfies α^{928} = 1 (Fermat's little theorem).
 *
 * Special cases:
 * - `base = 0, exp = 0`: returns 1 by convention
 * - `base = 0, exp > 0`: returns 0
 * - `exp = 0`: returns 1 for any non-zero base
 * - Negative exponents are not supported
 *
 * @throws Error if exp is not a non-negative integer
 *
 * ```
 * pow(3, 0)   = 1
 * pow(3, 1)   = 3
 * pow(3, 928) = 1    (Fermat's little theorem: α^{p-1} ≡ 1 mod p)
 * pow(3, 2)   = 9
 * ```
 */
export function power(base: GF929, exp: number): GF929 {
  if (!Number.isInteger(exp) || exp < 0) {
    throw new Error("GF929: exponent must be a non-negative integer");
  }
  if (base === 0) return exp === 0 ? 1 : 0;
  if (exp === 0) return 1;
  // Use the logarithm table for O(1) exponentiation.
  // (LOG[base] * exp) mod ORDER gives the exponent index into EXP.
  return _EXP[((_LOG[base] * exp) % ORDER + ORDER) % ORDER];
}

/**
 * Compute the multiplicative inverse of a GF(929) element.
 *
 * The inverse of a satisfies: a × inverse(a) = 1.
 *
 * By Fermat's little theorem (for prime p): a^{p-1} ≡ 1 (mod p)
 * Therefore: a × a^{p-2} ≡ 1 (mod p)
 * So: a^{-1} = a^{p-2} = a^{927} (mod 929)
 *
 * In terms of the discrete-log representation:
 * ```
 * log(a^{-1}) = -log(a) mod 928 = (928 - log(a)) mod 928
 * a^{-1} = EXP[(928 - LOG[a]) mod 928]
 * ```
 *
 * Verification: inverse(3) = 310, because 3 × 310 = 930 = 929 + 1 ≡ 1 mod 929.
 *
 * @throws Error if a is 0 (zero has no multiplicative inverse)
 */
export function inverse(a: GF929): GF929 {
  if (a === 0) throw new Error("GF929: zero has no multiplicative inverse");
  // inv(a) = EXP[ORDER - LOG[a]]
  // Because LOG[a] ∈ [0, 927], ORDER - LOG[a] ∈ [1, 928], always valid.
  return _EXP[ORDER - _LOG[a]];
}

/**
 * Return the additive identity (zero element).
 */
export function zero(): GF929 {
  return 0;
}

/**
 * Return the multiplicative identity (one element).
 */
export function one(): GF929 {
  return 1;
}

/**
 * Check whether a value is a valid GF(929) element (integer in [0, 928]).
 *
 * Useful for assertions in calling code:
 * ```ts
 * if (!isElement(x)) throw new Error(`Invalid GF929 element: ${x}`);
 * ```
 */
export function isElement(v: number): v is GF929 {
  return Number.isInteger(v) && v >= 0 && v <= 928;
}
