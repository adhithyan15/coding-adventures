// polynomial_native.test.ts -- Comprehensive tests for the native polynomial addon
// =================================================================================
//
// These tests verify that the Rust polynomial implementation is correctly
// exposed to JavaScript via the N-API node-bridge. Every public function
// is tested, including edge cases like empty arrays (zero polynomial),
// single-element arrays (constant polynomial), division by zero, and
// Horner's method evaluation.
//
// ## Polynomial representation reminder
//
// Arrays are "little-endian": index 0 = constant term.
//   [3, 0, 2] = 3 + 0·x + 2·x² = 3 + 2x²
//   [1, 2, 3] = 1 + 2x + 3x²

import { describe, it, expect } from "vitest";
import {
  normalize,
  degree,
  zero,
  one,
  add,
  subtract,
  multiply,
  divmodPoly,
  divide,
  modulo,
  evaluate,
  gcd,
} from "../index.js";

// ---------------------------------------------------------------------------
// normalize
// ---------------------------------------------------------------------------

describe("normalize", () => {
  it("strips trailing zeros from a polynomial", () => {
    expect(normalize([1.0, 0.0, 0.0])).toEqual([1.0]);
  });

  it("turns [0.0] into [] (zero polynomial becomes empty)", () => {
    expect(normalize([0.0])).toEqual([]);
  });

  it("leaves an already-normalized polynomial unchanged", () => {
    expect(normalize([1.0, 2.0, 3.0])).toEqual([1.0, 2.0, 3.0]);
  });

  it("normalizes an empty array to an empty array", () => {
    expect(normalize([])).toEqual([]);
  });

  it("strips multiple trailing zeros", () => {
    expect(normalize([5.0, 0.0, 0.0, 0.0])).toEqual([5.0]);
  });
});

// ---------------------------------------------------------------------------
// degree
// ---------------------------------------------------------------------------

describe("degree", () => {
  it("returns the degree of a quadratic", () => {
    expect(degree([3.0, 0.0, 2.0])).toBe(2);
  });

  it("returns 0 for a constant polynomial", () => {
    expect(degree([7.0])).toBe(0);
  });

  it("returns 0 for the zero polynomial (empty array)", () => {
    expect(degree([])).toBe(0);
  });

  it("returns 0 for [0.0]", () => {
    expect(degree([0.0])).toBe(0);
  });

  it("returns correct degree for linear polynomial", () => {
    expect(degree([0.0, 3.0])).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// zero and one
// ---------------------------------------------------------------------------

describe("zero", () => {
  it("returns [0.0]", () => {
    expect(zero()).toEqual([0.0]);
  });

  it("is the additive identity: add(zero(), p) == p", () => {
    const p = [1.0, 2.0, 3.0];
    expect(add(zero(), p)).toEqual(p);
    expect(add(p, zero())).toEqual(p);
  });
});

describe("one", () => {
  it("returns [1.0]", () => {
    expect(one()).toEqual([1.0]);
  });

  it("is the multiplicative identity: multiply(one(), p) == p", () => {
    const p = [1.0, 2.0, 3.0];
    expect(multiply(one(), p)).toEqual(p);
    expect(multiply(p, one())).toEqual(p);
  });
});

// ---------------------------------------------------------------------------
// add
// ---------------------------------------------------------------------------

describe("add", () => {
  it("adds two polynomials of the same length", () => {
    // [1, 2, 3] + [4, 5, 6] = [5, 7, 9]
    expect(add([1.0, 2.0, 3.0], [4.0, 5.0, 6.0])).toEqual([5.0, 7.0, 9.0]);
  });

  it("adds polynomials of different lengths", () => {
    // [1, 2, 3] + [4, 5] = [5, 7, 3]
    expect(add([1.0, 2.0, 3.0], [4.0, 5.0])).toEqual([5.0, 7.0, 3.0]);
  });

  it("normalizes the result (cancelling high-degree terms)", () => {
    // [1, 2, 3] + [0, 0, -3] = [1, 2, 0] -> normalized to [1, 2]
    expect(add([1.0, 2.0, 3.0], [0.0, 0.0, -3.0])).toEqual([1.0, 2.0]);
  });

  it("handles adding zero polynomial", () => {
    expect(add([1.0, 2.0], [])).toEqual([1.0, 2.0]);
  });
});

// ---------------------------------------------------------------------------
// subtract
// ---------------------------------------------------------------------------

describe("subtract", () => {
  it("subtracts two polynomials", () => {
    // [5, 7, 3] - [1, 2, 3] = [4, 5, 0] -> [4, 5]
    expect(subtract([5.0, 7.0, 3.0], [1.0, 2.0, 3.0])).toEqual([4.0, 5.0]);
  });

  it("subtracting a polynomial from itself gives zero", () => {
    const p = [1.0, 2.0, 3.0];
    expect(subtract(p, p)).toEqual([]);
  });

  it("handles subtracting zero polynomial", () => {
    expect(subtract([1.0, 2.0], [])).toEqual([1.0, 2.0]);
  });

  it("handles subtracting a longer polynomial", () => {
    // [1, 2] - [1, 2, 3] = [0, 0, -3] -> [-3] at degree 2
    expect(subtract([1.0, 2.0], [1.0, 2.0, 3.0])).toEqual([0.0, 0.0, -3.0]);
  });
});

// ---------------------------------------------------------------------------
// multiply
// ---------------------------------------------------------------------------

describe("multiply", () => {
  it("multiplies two linear polynomials", () => {
    // (1 + 2x)(3 + 4x) = 3 + 10x + 8x²
    expect(multiply([1.0, 2.0], [3.0, 4.0])).toEqual([3.0, 10.0, 8.0]);
  });

  it("multiplying by zero polynomial gives zero", () => {
    expect(multiply([1.0, 2.0, 3.0], [])).toEqual([]);
    expect(multiply([], [1.0, 2.0, 3.0])).toEqual([]);
  });

  it("multiplying by one polynomial is identity", () => {
    const p = [1.0, 2.0, 3.0];
    expect(multiply(p, one())).toEqual(p);
  });

  it("multiplies a constant by a polynomial", () => {
    // 2 * (1 + x + x²) = 2 + 2x + 2x²
    expect(multiply([2.0], [1.0, 1.0, 1.0])).toEqual([2.0, 2.0, 2.0]);
  });
});

// ---------------------------------------------------------------------------
// divmodPoly
// ---------------------------------------------------------------------------

describe("divmodPoly", () => {
  it("performs polynomial long division and returns [quotient, remainder]", () => {
    // (5 + x + 3x² + 2x³) / (2 + x)
    // quotient = 3 - x + 2x²,  remainder = -1
    const [q, r] = divmodPoly([5.0, 1.0, 3.0, 2.0], [2.0, 1.0]);
    expect(q).toEqual([3.0, -1.0, 2.0]);
    expect(r).toEqual([-1.0]);
  });

  it("returns empty quotient and dividend as remainder when dividend has lower degree", () => {
    // dividend degree < divisor degree => quotient=[], remainder=dividend
    const [q, r] = divmodPoly([1.0, 2.0], [1.0, 0.0, 1.0]);
    expect(q).toEqual([]);
    expect(r).toEqual([1.0, 2.0]);
  });

  it("divides evenly (zero remainder)", () => {
    // (x² - 1) / (x - 1) = x + 1,  remainder = 0
    // [−1, 0, 1] / [−1, 1]
    const [q, r] = divmodPoly([-1.0, 0.0, 1.0], [-1.0, 1.0]);
    expect(q).toEqual([1.0, 1.0]); // x + 1
    expect(r).toEqual([]); // zero remainder
  });

  it("throws when dividing by the zero polynomial", () => {
    expect(() => divmodPoly([1.0, 2.0], [])).toThrow();
    expect(() => divmodPoly([1.0, 2.0], [0.0])).toThrow();
  });
});

// ---------------------------------------------------------------------------
// divide
// ---------------------------------------------------------------------------

describe("divide", () => {
  it("returns the quotient of polynomial division", () => {
    const q = divide([5.0, 1.0, 3.0, 2.0], [2.0, 1.0]);
    expect(q).toEqual([3.0, -1.0, 2.0]);
  });

  it("throws when dividing by zero", () => {
    expect(() => divide([1.0, 2.0], [])).toThrow();
  });
});

// ---------------------------------------------------------------------------
// modulo
// ---------------------------------------------------------------------------

describe("modulo", () => {
  it("returns the remainder of polynomial division", () => {
    const r = modulo([5.0, 1.0, 3.0, 2.0], [2.0, 1.0]);
    expect(r).toEqual([-1.0]);
  });

  it("returns empty remainder when divisor divides evenly", () => {
    // (x² - 1) mod (x - 1) = 0
    const r = modulo([-1.0, 0.0, 1.0], [-1.0, 1.0]);
    expect(r).toEqual([]);
  });

  it("throws when divisor is zero", () => {
    expect(() => modulo([1.0, 2.0], [])).toThrow();
  });
});

// ---------------------------------------------------------------------------
// evaluate
// ---------------------------------------------------------------------------

describe("evaluate", () => {
  it("evaluates a quadratic at x=2", () => {
    // 3 + 0x + 1x²  at x=2 = 3 + 0 + 4 = 7
    expect(evaluate([3.0, 0.0, 1.0], 2.0)).toBe(7.0);
  });

  it("evaluates a linear polynomial at x=3", () => {
    // 1 + 2x at x=3 = 1 + 6 = 7
    expect(evaluate([1.0, 2.0], 3.0)).toBe(7.0);
  });

  it("evaluates a constant at any x", () => {
    expect(evaluate([5.0], 100.0)).toBe(5.0);
    expect(evaluate([5.0], 0.0)).toBe(5.0);
  });

  it("evaluates the zero polynomial to 0", () => {
    expect(evaluate([], 42.0)).toBe(0.0);
    expect(evaluate([0.0], 42.0)).toBe(0.0);
  });

  it("evaluates at x=0 gives the constant term", () => {
    // p(0) = coefficient of x^0 = first element
    expect(evaluate([7.0, 3.0, 2.0], 0.0)).toBe(7.0);
  });
});

// ---------------------------------------------------------------------------
// gcd
// ---------------------------------------------------------------------------

describe("gcd", () => {
  it("computes the GCD of two polynomials with a common factor", () => {
    // gcd(x² - 3x + 2, x - 1)
    // x² - 3x + 2 = (x-1)(x-2), so GCD = x-1
    // In array form: x² - 3x + 2 = [2, -3, 1]
    //                x - 1        = [-1, 1]
    // GCD = [-1, 1] or a scalar multiple thereof
    const g = gcd([2.0, -3.0, 1.0], [-1.0, 1.0]);
    // The gcd is x-1 = [-1, 1]. Verify it divides both.
    expect(modulo([2.0, -3.0, 1.0], g)).toEqual([]);
    expect(modulo([-1.0, 1.0], g)).toEqual([]);
  });

  it("returns one of the inputs when they share no common factor", () => {
    // gcd of two coprime polynomials should be a constant
    const g = gcd([1.0], [1.0, 1.0]);
    // constant polynomial divides everything
    expect(degree(g)).toBe(0);
  });

  it("gcd(p, p) = p (up to normalization)", () => {
    const p = [1.0, -1.0]; // x - 1
    const g = gcd(p, p);
    // g should be proportional to p
    expect(g.length).toBe(p.length);
    expect(degree(g)).toBe(degree(p));
  });

  it("gcd(p, zero) = p", () => {
    const p = [1.0, 2.0, 1.0]; // 1 + 2x + x²
    const g = gcd(p, []);
    // normalize both for comparison
    expect(normalize(g)).toEqual(normalize(p));
  });
});
