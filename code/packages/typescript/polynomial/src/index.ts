/**
 * @module polynomial
 *
 * Polynomial arithmetic over real numbers.
 *
 * A polynomial is represented as a readonly number[] where the array index
 * equals the degree of that term's coefficient:
 *
 *   [3, 0, 2]  →  3 + 0·x + 2·x²  =  3 + 2x²
 *   [1, 2, 3]  →  1 + 2x + 3x²
 *   []         →  the zero polynomial
 *
 * This "little-endian" representation makes addition trivially position-aligned
 * and keeps Horner's method natural to read.
 *
 * All functions return normalized polynomials — trailing zeros are stripped.
 * So [1, 0, 0] and [1] both represent the constant 1.
 */

export const VERSION = "0.1.0";

/**
 * Polynomial type: a readonly array where index i holds the coefficient of x^i.
 *
 * The zero polynomial is represented as an empty array [].
 */
export type Polynomial = readonly number[];

// =============================================================================
// Fundamentals
// =============================================================================

/**
 * Remove trailing zeros from a polynomial.
 *
 * Trailing zeros represent zero-coefficient high-degree terms. They do not
 * change the mathematical value but do affect degree comparisons and the
 * stopping condition in polynomial long division.
 *
 * Examples:
 *   normalize([1, 0, 0]) → [1]   (constant polynomial 1)
 *   normalize([0])       → []    (zero polynomial)
 *   normalize([1, 2, 3]) → [1, 2, 3]  (already normalized)
 */
export function normalize(p: Polynomial): Polynomial {
  let len = p.length;
  // Walk backwards until we find a non-zero coefficient.
  while (len > 0 && p[len - 1] === 0) {
    len--;
  }
  // Slice to the new length (or return empty array for zero polynomial).
  return p.slice(0, len);
}

/**
 * Return the degree of a polynomial.
 *
 * The degree is the index of the highest non-zero coefficient.
 * By convention, the zero polynomial has degree -1. This sentinel value
 * allows polynomial long division to terminate cleanly:
 *   the loop condition `degree(remainder) >= degree(divisor)` is false
 *   when remainder is zero.
 *
 * Examples:
 *   degree([3, 0, 2]) → 2   (highest non-zero: index 2, the x² term)
 *   degree([7])       → 0   (constant polynomial; degree 0)
 *   degree([])        → -1  (zero polynomial; degree -1 by convention)
 *   degree([0, 0])    → -1  (normalizes to []; same as zero polynomial)
 */
export function degree(p: Polynomial): number {
  const n = normalize(p);
  return n.length - 1; // -1 when n is empty (zero polynomial)
}

/**
 * Return the zero polynomial [].
 *
 * Zero is the additive identity: add(zero(), p) = p for any p.
 */
export function zero(): Polynomial {
  return [];
}

/**
 * Return the multiplicative identity polynomial [1].
 *
 * Multiplying any polynomial by one() returns that polynomial unchanged.
 */
export function one(): Polynomial {
  return [1];
}

// =============================================================================
// Addition and Subtraction
// =============================================================================

/**
 * Add two polynomials term-by-term.
 *
 * Addition is the simplest operation: add matching coefficients, extending the
 * shorter polynomial with implicit zeros.
 *
 * Visual example:
 *   [1, 2, 3]   =  1 + 2x + 3x²
 * + [4, 5]      =  4 + 5x
 * ─────────────
 *   [5, 7, 3]   =  5 + 7x + 3x²
 *
 * The degree-2 term had no partner in b, so it carried through unchanged.
 */
export function add(a: Polynomial, b: Polynomial): Polynomial {
  // Allocate result as long as the longer input.
  const len = Math.max(a.length, b.length);
  const result: number[] = new Array(len).fill(0);

  for (let i = 0; i < len; i++) {
    const ai = i < a.length ? a[i] : 0;
    const bi = i < b.length ? b[i] : 0;
    result[i] = ai + bi;
  }

  return normalize(result);
}

/**
 * Subtract polynomial b from polynomial a term-by-term.
 *
 * Equivalent to add(a, negate(b)), but implemented directly to avoid
 * creating an intermediate negated polynomial.
 *
 * Visual example:
 *   [5, 7, 3]   =  5 + 7x + 3x²
 * - [1, 2, 3]   =  1 + 2x + 3x²
 * ─────────────
 *   [4, 5, 0]   →  normalize  →  [4, 5]   =  4 + 5x
 *
 * Note: 3x² - 3x² = 0; normalize strips the trailing zero.
 */
export function subtract(a: Polynomial, b: Polynomial): Polynomial {
  const len = Math.max(a.length, b.length);
  const result: number[] = new Array(len).fill(0);

  for (let i = 0; i < len; i++) {
    const ai = i < a.length ? a[i] : 0;
    const bi = i < b.length ? b[i] : 0;
    result[i] = ai - bi;
  }

  return normalize(result);
}

// =============================================================================
// Multiplication
// =============================================================================

/**
 * Multiply two polynomials using polynomial convolution.
 *
 * Each term a[i]·xⁱ of a multiplies each term b[j]·xʲ of b, contributing
 * a[i]·b[j] to the result's coefficient at index i+j.
 *
 * If a has degree m and b has degree n, the result has degree m+n.
 *
 * Visual example:
 *   [1, 2]  =  1 + 2x
 * × [3, 4]  =  3 + 4x
 * ──────────────────────────────
 * result array of length 3, initialized to [0, 0, 0]:
 *   i=0, j=0: result[0] += 1·3 = 3   → [3, 0, 0]
 *   i=0, j=1: result[1] += 1·4 = 4   → [3, 4, 0]
 *   i=1, j=0: result[1] += 2·3 = 6   → [3, 10, 0]
 *   i=1, j=1: result[2] += 2·4 = 8   → [3, 10, 8]
 *
 * Result: [3, 10, 8]  =  3 + 10x + 8x²
 * Verify: (1+2x)(3+4x) = 3+4x+6x+8x² = 3+10x+8x²  ✓
 */
export function multiply(a: Polynomial, b: Polynomial): Polynomial {
  // Multiplying by zero yields zero.
  if (a.length === 0 || b.length === 0) {
    return [];
  }

  // Result degree = deg(a) + deg(b), so length = a.length + b.length - 1.
  const resultLen = a.length + b.length - 1;
  const result: number[] = new Array(resultLen).fill(0);

  for (let i = 0; i < a.length; i++) {
    for (let j = 0; j < b.length; j++) {
      result[i + j] += a[i] * b[j];
    }
  }

  return normalize(result);
}

// =============================================================================
// Division
// =============================================================================

/**
 * Perform polynomial long division, returning [quotient, remainder].
 *
 * Given polynomials a and b (b ≠ zero), finds q and r such that:
 *   a = b × q + r   and   degree(r) < degree(b)
 *
 * The algorithm is the polynomial analog of school long division:
 * 1. Find the leading term of the current remainder.
 * 2. Divide it by the leading term of b to get the next quotient term.
 * 3. Subtract (quotient term) × b from the remainder.
 * 4. Repeat until degree(remainder) < degree(b).
 *
 * Detailed example: divide [5, 1, 3, 2] = 5 + x + 3x² + 2x³  by  [2, 1] = 2 + x
 *
 *   Step 1: remainder = [5, 1, 3, 2], deg=3.  Leading = 2x³, divisor leading = x.
 *           Quotient term: 2x³/x = 2x²  → q[2] = 2
 *           Subtract 2x² × (2+x) = 4x²+2x³ = [0,0,4,2] from remainder:
 *           [5,1,3-4,2-2] = [5,1,-1,0] → normalize → [5,1,-1]
 *
 *   Step 2: remainder = [5,1,-1], deg=2.  Leading = -x², divisor leading = x.
 *           Quotient term: -x²/x = -x  → q[1] = -1
 *           Subtract -x × (2+x) = -2x-x² = [0,-2,-1] from [5,1,-1]:
 *           [5,3,0] → [5,3]
 *
 *   Step 3: remainder = [5,3], deg=1.  Leading = 3x, divisor leading = x.
 *           Quotient term: 3x/x = 3  → q[0] = 3
 *           Subtract 3 × (2+x) = 6+3x = [6,3] from [5,3]:
 *           [-1,0] → [-1]
 *
 *   Step 4: degree([-1]) = 0 < 1 = degree(b). STOP.
 *   Result: q = [3, -1, 2],  r = [-1]
 *   Verify: (x+2)(3-x+2x²) + (-1) = 3x-x²+2x³+6-2x+4x² - 1 = 5+x+3x²+2x³  ✓
 *
 * @throws Error if b is the zero polynomial
 */
export function divmod(
  a: Polynomial,
  b: Polynomial
): [Polynomial, Polynomial] {
  const nb = normalize(b);
  if (nb.length === 0) {
    throw new Error("polynomial division by zero");
  }

  const na = normalize(a);
  const degA = na.length - 1;
  const degB = nb.length - 1;

  // If a has lower degree than b, quotient is 0, remainder is a.
  if (degA < degB) {
    return [[], na];
  }

  // Work on a mutable copy of the remainder.
  const rem: number[] = [...na];
  // Allocate the quotient with the right degree.
  const quot: number[] = new Array(degA - degB + 1).fill(0);

  // The leading coefficient of the divisor — used to compute each quotient term.
  const leadB = nb[degB];

  // Current degree of the remainder (walks downward as we subtract).
  let degRem = degA;

  while (degRem >= degB) {
    // Leading coefficient of the current remainder.
    const leadRem = rem[degRem];
    // Quotient term coefficient and degree.
    const coeff = leadRem / leadB;
    const power = degRem - degB;
    quot[power] = coeff;

    // Subtract coeff·x^power·b from rem.
    for (let j = 0; j <= degB; j++) {
      rem[power + j] -= coeff * nb[j];
    }

    // The leading term is now zero (by construction). Decrement degRem.
    // We also need to skip any new trailing zeros.
    degRem--;
    while (degRem >= 0 && rem[degRem] === 0) {
      degRem--;
    }
  }

  return [normalize(quot), normalize(rem)];
}

/**
 * Return the quotient of divmod(a, b).
 *
 * @throws Error if b is the zero polynomial
 */
export function divide(a: Polynomial, b: Polynomial): Polynomial {
  return divmod(a, b)[0];
}

/**
 * Return the remainder of divmod(a, b).
 *
 * This is the polynomial "modulo" operation. In GF(2^8) construction, we
 * reduce a high-degree polynomial modulo the primitive polynomial using this.
 *
 * @throws Error if b is the zero polynomial
 */
export function mod(a: Polynomial, b: Polynomial): Polynomial {
  return divmod(a, b)[1];
}

// =============================================================================
// Evaluation
// =============================================================================

/**
 * Evaluate a polynomial at x using Horner's method.
 *
 * Horner's method rewrites the polynomial in nested form:
 *   a₀ + x(a₁ + x(a₂ + ... + x·aₙ))
 *
 * This requires only n additions and n multiplications — no powers of x.
 *
 * Algorithm (reading coefficients from high degree to low):
 *   acc = 0
 *   for i from n downto 0:
 *       acc = acc * x + p[i]
 *   return acc
 *
 * Example: evaluate [3, 1, 2] = 3 + x + 2x² at x = 4:
 *   Start: acc = 0
 *   i=2: acc = 0*4 + 2 = 2
 *   i=1: acc = 2*4 + 1 = 9
 *   i=0: acc = 9*4 + 3 = 39
 *   Verify: 3 + 4 + 2·16 = 3 + 4 + 32 = 39  ✓
 *
 * @param p polynomial to evaluate
 * @param x the point at which to evaluate
 * @returns the numeric value p(x)
 */
export function evaluate(p: Polynomial, x: number): number {
  const n = normalize(p);
  // Zero polynomial evaluates to 0 everywhere.
  if (n.length === 0) return 0;

  let acc = 0;
  // Iterate from high-degree term down to the constant.
  for (let i = n.length - 1; i >= 0; i--) {
    acc = acc * x + n[i];
  }
  return acc;
}

// =============================================================================
// Greatest Common Divisor
// =============================================================================

/**
 * Compute the greatest common divisor of two polynomials.
 *
 * Uses the Euclidean algorithm: repeatedly replace (a, b) with (b, a mod b)
 * until b is the zero polynomial. The last non-zero remainder is the GCD.
 *
 * This is identical to the integer GCD algorithm, with polynomial mod in place
 * of integer mod.
 *
 * Pseudocode:
 *   while b ≠ zero:
 *       a, b = b, a mod b
 *   return normalize(a)
 *
 * Use case: GCD is used in Reed-Solomon decoding (extended Euclidean algorithm)
 * to find the error-locator and error-evaluator polynomials.
 *
 * @param a first polynomial
 * @param b second polynomial
 * @returns the GCD of a and b, normalized
 */
export function gcd(a: Polynomial, b: Polynomial): Polynomial {
  let u = normalize(a);
  let v = normalize(b);

  while (v.length > 0) {
    const r = mod(u, v);
    u = v;
    v = r;
  }

  return normalize(u);
}
