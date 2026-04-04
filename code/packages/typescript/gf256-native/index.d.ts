// index.d.ts -- TypeScript type definitions for @coding-adventures/gf256-native
// ==============================================================================
//
// These type definitions describe the native GF(2^8) operations exposed by the
// Rust addon. The actual implementation is in src/lib.rs; these types exist
// so TypeScript consumers get full IntelliSense and type checking.
//
// ## GF(2^8) overview
//
// GF(2^8) is a finite field with exactly 256 elements (0 through 255).
// - Addition is XOR (so subtract === add)
// - Multiplication uses precomputed log/antilog tables
// - The primitive polynomial is x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285
//
// Elements are represented as JS numbers in the range [0, 255].

/**
 * The additive identity element.
 * `add(ZERO, x) === x` for all `x`.
 */
export const ZERO: number;

/**
 * The multiplicative identity element.
 * `multiply(ONE, x) === x` for all `x`.
 */
export const ONE: number;

/**
 * The primitive (irreducible) polynomial used for modular reduction.
 *
 * `p(x) = x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285`
 *
 * This polynomial is irreducible over GF(2) and primitive (the element 2
 * generates all 255 non-zero elements of the field).
 */
export const PRIMITIVE_POLYNOMIAL: number;

/**
 * Add two GF(256) elements.
 *
 * In characteristic-2, addition is bitwise XOR. Every element is its own
 * additive inverse: `add(x, x) === 0`.
 *
 * ```typescript
 * add(0x53, 0xCA)  // 0x99 = 153
 * add(5, 5)        // 0  (every element cancels itself)
 * ```
 */
export function add(a: number, b: number): number;

/**
 * Subtract two GF(256) elements.
 *
 * In characteristic-2, subtraction equals addition (XOR).
 * `subtract(a, b) === add(a, b)` always.
 */
export function subtract(a: number, b: number): number;

/**
 * Multiply two GF(256) elements using logarithm/antilogarithm tables.
 *
 * `a × b = ALOG[(LOG[a] + LOG[b]) mod 255]`
 *
 * Special case: `multiply(0, x) === 0` for any `x`.
 *
 * ```typescript
 * multiply(2, 3)  // 6 (below overflow threshold)
 * multiply(2, 128) // 29 (overflows byte, reduced modulo primitive poly)
 * ```
 */
export function multiply(a: number, b: number): number;

/**
 * Divide `a` by `b` in GF(256).
 *
 * `a / b = ALOG[(LOG[a] - LOG[b] + 255) mod 255]`
 *
 * Special case: `divide(0, b) === 0` for any non-zero `b`.
 *
 * @throws Error if `b === 0` (division by zero is undefined in any field)
 *
 * ```typescript
 * divide(1, 1)  // 1  (any element divided by itself is 1)
 * divide(6, 3)  // might not be 2 — this is GF(256) not integer arithmetic!
 * ```
 */
export function divide(a: number, b: number): number;

/**
 * Raise a GF(256) element to a non-negative integer power.
 *
 * `base^exp = ALOG[(LOG[base] * exp) mod 255]`
 *
 * Special cases:
 * - `power(0, 0) === 1` (by convention)
 * - `power(0, n) === 0` for `n > 0`
 * - `power(b, 0) === 1` for any non-zero `b`
 *
 * ```typescript
 * power(2, 8)   // first power of 2 that overflows: 29 (= 2^8 mod p(x))
 * power(2, 255) // 1  (the multiplicative group has order 255)
 * ```
 */
export function power(base: number, exp: number): number;

/**
 * Compute the multiplicative inverse of a GF(256) element.
 *
 * `inverse(a) × a === 1`
 *
 * `inverse(a) = ALOG[255 - LOG[a]]`
 *
 * @throws Error if `a === 0` (zero has no multiplicative inverse)
 *
 * ```typescript
 * inverse(1)  // 1  (1 is its own inverse)
 * multiply(a, inverse(a)) === 1  // for any non-zero a
 * ```
 */
export function inverse(a: number): number;
