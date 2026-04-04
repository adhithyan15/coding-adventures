// index.d.ts -- TypeScript type definitions for @coding-adventures/polynomial-native
// ==================================================================================
//
// These type definitions describe the native polynomial functions exposed by the
// Rust addon. The actual implementation is in src/lib.rs; these types exist
// so TypeScript consumers get full IntelliSense and type checking.
//
// ## Polynomial representation
//
// All polynomials are represented as `number[]` in "little-endian" order:
// index 0 = constant term (coefficient of x^0), index 1 = coefficient of x^1, etc.
//
// Examples:
//   [3.0, 0.0, 2.0]  →  3 + 0·x + 2·x²  =  3 + 2x²
//   [1.0, 2.0, 3.0]  →  1 + 2x + 3x²
//   []               →  the zero polynomial
//   [0.0]            →  also the zero polynomial (normalized form)

/**
 * Remove trailing near-zero coefficients from a polynomial.
 *
 * `normalize([1.0, 0.0, 0.0])` returns `[1.0]`
 * `normalize([0.0])` returns `[]`
 */
export function normalize(poly: number[]): number[];

/**
 * Return the degree of a polynomial (index of the highest non-zero coefficient).
 *
 * Returns 0 for the zero polynomial by convention.
 *
 * `degree([3.0, 0.0, 2.0])` returns `2`
 * `degree([7.0])` returns `0`
 * `degree([])` returns `0`
 */
export function degree(poly: number[]): number;

/**
 * Return the zero polynomial `[0.0]` -- the additive identity.
 *
 * `add(zero(), p)` equals `p` for any polynomial `p`.
 */
export function zero(): number[];

/**
 * Return the one polynomial `[1.0]` -- the multiplicative identity.
 *
 * `multiply(one(), p)` equals `p` for any polynomial `p`.
 */
export function one(): number[];

/**
 * Add two polynomials term-by-term.
 *
 * If `a` has degree `m` and `b` has degree `n`, the result has degree ≤ max(m, n).
 *
 * ```typescript
 * add([1, 2, 3], [4, 5])  // [5, 7, 3]  →  5 + 7x + 3x²
 * ```
 */
export function add(a: number[], b: number[]): number[];

/**
 * Subtract polynomial `b` from polynomial `a` term-by-term.
 *
 * ```typescript
 * subtract([5, 7, 3], [1, 2, 3])  // [4, 5]  →  4 + 5x
 * ```
 */
export function subtract(a: number[], b: number[]): number[];

/**
 * Multiply two polynomials using polynomial convolution.
 *
 * If `a` has degree `m` and `b` has degree `n`, the result has degree `m + n`.
 *
 * ```typescript
 * multiply([1, 2], [3, 4])  // [3, 10, 8]  →  3 + 10x + 8x²
 * ```
 */
export function multiply(a: number[], b: number[]): number[];

/**
 * Perform polynomial long division, returning `[quotient, remainder]`.
 *
 * Finds `q` and `r` such that: `dividend = divisor × q + r`
 * where `degree(r) < degree(divisor)`.
 *
 * @throws Error if `divisor` is the zero polynomial
 *
 * ```typescript
 * divmodPoly([5, 1, 3, 2], [2, 1])
 *   // quotient  = [3, -1, 2]  →  3 - x + 2x²
 *   // remainder = [-1]
 * ```
 */
export function divmodPoly(dividend: number[], divisor: number[]): [number[], number[]];

/**
 * Return the quotient of polynomial division (divmodPoly(a, b)[0]).
 *
 * @throws Error if `b` is the zero polynomial
 */
export function divide(a: number[], b: number[]): number[];

/**
 * Return the remainder of polynomial division (divmodPoly(a, b)[1]).
 *
 * Named `modulo` rather than `mod` because `mod` is a reserved word in JavaScript.
 *
 * @throws Error if `b` is the zero polynomial
 */
export function modulo(a: number[], b: number[]): number[];

/**
 * Evaluate a polynomial at `x` using Horner's method.
 *
 * Horner's method computes `a₀ + x(a₁ + x(a₂ + … + x·aₙ))` in O(n) time,
 * avoiding explicit exponentiation.
 *
 * ```typescript
 * evaluate([3, 0, 1], 2)  // 3 + 0*2 + 1*4 = 7
 * ```
 */
export function evaluate(poly: number[], x: number): number;

/**
 * Compute the greatest common divisor of two polynomials.
 *
 * Uses the Euclidean algorithm with polynomial modulo instead of integer %.
 * Returns the highest-degree polynomial that divides both inputs exactly.
 *
 * ```typescript
 * gcd([2, -3, 1], [-2, 1])  // x - 1 (common factor of x²-3x+2 and x-1... approximately)
 * ```
 */
export function gcd(a: number[], b: number[]): number[];
