import { describe, it, expect } from "vitest";
import {
  VERSION,
  normalize,
  degree,
  zero,
  one,
  add,
  subtract,
  multiply,
  divmod,
  divide,
  mod,
  evaluate,
  gcd,
} from "../src/index.js";

// =============================================================================
// Helpers
// =============================================================================

/** Compare two polynomials coefficient-by-coefficient within epsilon. */
function polyEqual(
  a: readonly number[],
  b: readonly number[],
  eps = 1e-9
): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (Math.abs(a[i] - b[i]) > eps) return false;
  }
  return true;
}

// =============================================================================
// VERSION
// =============================================================================

describe("VERSION", () => {
  it("is a semver string", () => {
    expect(VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});

// =============================================================================
// normalize
// =============================================================================

describe("normalize", () => {
  it("strips trailing zeros", () => {
    expect(normalize([1, 0, 0])).toEqual([1]);
  });

  it("strips all-zero array to empty", () => {
    expect(normalize([0])).toEqual([]);
    expect(normalize([0, 0, 0])).toEqual([]);
  });

  it("returns empty array unchanged", () => {
    expect(normalize([])).toEqual([]);
  });

  it("leaves already-normalized polynomial unchanged", () => {
    expect(normalize([1, 2, 3])).toEqual([1, 2, 3]);
  });

  it("preserves non-trailing zeros", () => {
    expect(normalize([1, 0, 2])).toEqual([1, 0, 2]);
  });
});

// =============================================================================
// degree
// =============================================================================

describe("degree", () => {
  it("returns -1 for zero polynomial", () => {
    expect(degree([])).toBe(-1);
    expect(degree([0])).toBe(-1);
    expect(degree([0, 0, 0])).toBe(-1);
  });

  it("returns 0 for constant polynomial", () => {
    expect(degree([7])).toBe(0);
    expect(degree([1])).toBe(0);
  });

  it("returns correct degree for higher-degree polynomials", () => {
    expect(degree([1, 2])).toBe(1);
    expect(degree([1, 2, 3])).toBe(2);
    expect(degree([3, 0, 2])).toBe(2);
  });

  it("ignores trailing zeros when computing degree", () => {
    expect(degree([3, 0, 0])).toBe(0);
    expect(degree([1, 2, 0])).toBe(1);
  });
});

// =============================================================================
// zero and one
// =============================================================================

describe("zero", () => {
  it("returns empty array", () => {
    expect(zero()).toEqual([]);
  });

  it("is additive identity", () => {
    const p = [1, 2, 3];
    expect(add(zero(), p)).toEqual(p);
    expect(add(p, zero())).toEqual(p);
  });
});

describe("one", () => {
  it("returns [1]", () => {
    expect(one()).toEqual([1]);
  });

  it("is multiplicative identity", () => {
    const p = [1, 2, 3];
    expect(multiply(one(), p)).toEqual(p);
    expect(multiply(p, one())).toEqual(p);
  });
});

// =============================================================================
// add
// =============================================================================

describe("add", () => {
  it("adds polynomials of same length", () => {
    // [1,2,3] + [4,5,6] = [5,7,9]
    expect(add([1, 2, 3], [4, 5, 6])).toEqual([5, 7, 9]);
  });

  it("extends shorter polynomial with zeros", () => {
    // [1,2,3] + [4,5] = [5,7,3]
    expect(add([1, 2, 3], [4, 5])).toEqual([5, 7, 3]);
    expect(add([4, 5], [1, 2, 3])).toEqual([5, 7, 3]);
  });

  it("returns zero polynomial when coefficients cancel", () => {
    expect(add([1, 2, 3], [-1, -2, -3])).toEqual([]);
  });

  it("adding zero polynomial is identity", () => {
    expect(add([], [1, 2])).toEqual([1, 2]);
    expect(add([1, 2], [])).toEqual([1, 2]);
  });

  it("is commutative", () => {
    const a = [1, 2, 3];
    const b = [4, 5, 6, 7];
    expect(add(a, b)).toEqual(add(b, a));
  });

  it("normalizes the result", () => {
    // [1,2,3] + [0,0,-3] → [1,2,0] → [1,2]
    expect(add([1, 2, 3], [0, 0, -3])).toEqual([1, 2]);
  });
});

// =============================================================================
// subtract
// =============================================================================

describe("subtract", () => {
  it("subtracts polynomials of same length", () => {
    // [5,7,3] - [1,2,3] = [4,5,0] → [4,5]
    expect(subtract([5, 7, 3], [1, 2, 3])).toEqual([4, 5]);
  });

  it("extends shorter polynomial with zeros", () => {
    // [1,2,3] - [1,2] = [0,0,3] → [0,0,3]
    expect(subtract([1, 2, 3], [1, 2])).toEqual([0, 0, 3]);
  });

  it("returns zero when subtracting from itself", () => {
    expect(subtract([1, 2, 3], [1, 2, 3])).toEqual([]);
  });

  it("subtracting zero polynomial is identity", () => {
    expect(subtract([1, 2, 3], [])).toEqual([1, 2, 3]);
  });

  it("p - q + q = p", () => {
    const p = [3, 1, 4];
    const q = [1, 5, 9];
    expect(add(subtract(p, q), q)).toEqual(p);
  });
});

// =============================================================================
// multiply
// =============================================================================

describe("multiply", () => {
  it("multiplies two linear polynomials", () => {
    // (1+2x)(3+4x) = 3 + 10x + 8x²
    expect(multiply([1, 2], [3, 4])).toEqual([3, 10, 8]);
  });

  it("multiplies by zero polynomial", () => {
    expect(multiply([1, 2, 3], [])).toEqual([]);
    expect(multiply([], [1, 2, 3])).toEqual([]);
  });

  it("multiplies by one polynomial", () => {
    expect(multiply([1, 2, 3], [1])).toEqual([1, 2, 3]);
    expect(multiply([1], [1, 2, 3])).toEqual([1, 2, 3]);
  });

  it("is commutative", () => {
    const a = [1, 2, 3];
    const b = [4, 5];
    const ab = multiply(a, b);
    const ba = multiply(b, a);
    expect(ab).toEqual(ba);
  });

  it("is associative", () => {
    const a = [1, 2];
    const b = [3, 4];
    const c = [5, 6];
    const left = multiply(multiply(a, b), c);
    const right = multiply(a, multiply(b, c));
    expect(left.length).toBe(right.length);
    for (let i = 0; i < left.length; i++) {
      expect(left[i]).toBeCloseTo(right[i], 9);
    }
  });

  it("is distributive over addition", () => {
    const a = [1, 2];
    const b = [3, 4];
    const c = [5, 6];
    // a*(b+c) = a*b + a*c
    const lhs = multiply(a, add(b, c));
    const rhs = add(multiply(a, b), multiply(a, c));
    expect(lhs.length).toBe(rhs.length);
    for (let i = 0; i < lhs.length; i++) {
      expect(lhs[i]).toBeCloseTo(rhs[i], 9);
    }
  });

  it("multiplies constant polynomials", () => {
    expect(multiply([3], [4])).toEqual([12]);
  });

  it("result degree is sum of input degrees", () => {
    const a = [1, 2, 3]; // degree 2
    const b = [4, 5, 6]; // degree 2
    const result = multiply(a, b);
    expect(degree(result)).toBe(4); // degree 4
  });
});

// =============================================================================
// divmod
// =============================================================================

describe("divmod", () => {
  it("throws for zero divisor", () => {
    expect(() => divmod([1, 2, 3], [])).toThrow();
    expect(() => divmod([1, 2, 3], [0])).toThrow();
  });

  it("returns [0, a] when degree(a) < degree(b)", () => {
    const [q, r] = divmod([1, 2], [1, 0, 1]);
    expect(q).toEqual([]);
    expect(r).toEqual([1, 2]);
  });

  it("divides perfectly (zero remainder)", () => {
    // (1+x)(1+x) = 1 + 2x + x²
    // (1 + 2x + x²) / (1+x) should give (1+x) remainder 0
    const product = multiply([1, 1], [1, 1]);
    const [q, r] = divmod(product, [1, 1]);
    expect(polyEqual(q, [1, 1])).toBe(true);
    expect(r).toEqual([]);
  });

  it("satisfies a = b*q + r", () => {
    const a = [5, 1, 3, 2]; // 5 + x + 3x² + 2x³
    const b = [2, 1]; // 2 + x
    const [q, r] = divmod(a, b);
    // Verify: b*q + r should equal a
    const reconstructed = add(multiply(b, q), r);
    expect(polyEqual(reconstructed, normalize(a))).toBe(true);
  });

  it("handles constant divisor", () => {
    const [q, r] = divmod([4, 6, 8], [2]);
    expect(polyEqual(q, [2, 3, 4])).toBe(true);
    expect(r).toEqual([]);
  });

  it("divides by itself", () => {
    const p = [1, 2, 3];
    const [q, r] = divmod(p, p);
    expect(polyEqual(q, [1])).toBe(true);
    expect(r).toEqual([]);
  });

  it("detailed example from spec", () => {
    // 5 + x + 3x² + 2x³  divided by  2 + x
    const a = [5, 1, 3, 2];
    const b = [2, 1];
    const [q, r] = divmod(a, b);
    // quotient should be 3 - x + 2x²  = [3, -1, 2]
    // remainder should be -1
    expect(polyEqual(q, [3, -1, 2])).toBe(true);
    expect(polyEqual(r, [-1])).toBe(true);
  });
});

// =============================================================================
// divide and mod
// =============================================================================

describe("divide", () => {
  it("returns quotient of divmod", () => {
    // 1 - x² = (1-x)(1+x); divide by (1+x) should give (1-x) = [1, -1]
    const a = [1, 0, -1]; // 1 - x²
    const b = [1, 1]; // 1 + x
    const q = divide(a, b);
    // verify: b * q = a
    expect(polyEqual(multiply(b, q), normalize(a))).toBe(true);
  });

  it("throws for zero divisor", () => {
    expect(() => divide([1, 2], [])).toThrow();
  });
});

describe("mod", () => {
  it("returns remainder of divmod", () => {
    const a = [1, 2, 3];
    const b = [1, 1];
    const r = mod(a, b);
    const q = divide(a, b);
    // Verify b*q + r = a
    const reconstructed = add(multiply(b, q), r);
    expect(polyEqual(reconstructed, a)).toBe(true);
  });

  it("returns zero remainder for exact division", () => {
    const p = multiply([1, 1], [2, 1]); // (1+x)(2+x)
    expect(mod(p, [1, 1])).toEqual([]);
  });

  it("throws for zero divisor", () => {
    expect(() => mod([1, 2], [])).toThrow();
  });
});

// =============================================================================
// evaluate
// =============================================================================

describe("evaluate", () => {
  it("evaluates zero polynomial to 0", () => {
    expect(evaluate([], 5)).toBe(0);
    expect(evaluate([], 0)).toBe(0);
  });

  it("evaluates constant polynomial", () => {
    expect(evaluate([7], 0)).toBe(7);
    expect(evaluate([7], 100)).toBe(7);
  });

  it("evaluates linear polynomial at x=0", () => {
    expect(evaluate([3, 2], 0)).toBe(3);
  });

  it("evaluates linear polynomial at x=1", () => {
    // 3 + 2·1 = 5
    expect(evaluate([3, 2], 1)).toBe(5);
  });

  it("evaluates quadratic at x=4 (spec example)", () => {
    // 3 + x + 2x² at x=4 → 3 + 4 + 32 = 39
    expect(evaluate([3, 1, 2], 4)).toBeCloseTo(39, 9);
  });

  it("evaluates at x=0 returns constant term", () => {
    expect(evaluate([5, 3, 1], 0)).toBe(5);
  });

  it("uses Horner's method (result matches naive evaluation)", () => {
    const p = [1, -3, 2]; // 1 - 3x + 2x²
    const x = 3;
    // Naive: 1 - 9 + 18 = 10
    const naive = p[0] + p[1] * x + p[2] * x * x;
    expect(evaluate(p, x)).toBeCloseTo(naive, 9);
  });
});

// =============================================================================
// gcd
// =============================================================================

describe("gcd", () => {
  it("gcd with zero returns the other polynomial (normalized)", () => {
    const p = [1, 2, 3];
    expect(polyEqual(gcd(p, []), normalize(p))).toBe(true);
    expect(polyEqual(gcd([], p), normalize(p))).toBe(true);
  });

  it("gcd of polynomial with itself", () => {
    const p = [1, 2, 3];
    const g = gcd(p, p);
    // GCD should be a scalar multiple of p
    expect(degree(g)).toBe(degree(p));
  });

  it("gcd of coprime polynomials is constant", () => {
    // (x+1) and (x+2) share no common factor
    const a = [1, 1]; // 1 + x
    const b = [2, 1]; // 2 + x
    const g = gcd(a, b);
    expect(degree(g)).toBe(0); // constant GCD
  });

  it("gcd of polynomials with common factor", () => {
    // (x+1)(x+2) = 2+3x+x² and (x+1)(x+3) = 3+4x+x²
    // GCD should be x+1 (up to scalar)
    const f1 = multiply([1, 1], [2, 1]); // (x+1)(x+2)
    const f2 = multiply([1, 1], [3, 1]); // (x+1)(x+3)
    const g = gcd(f1, f2);
    // degree should be 1 (linear factor)
    expect(degree(g)).toBe(1);
    // verify g divides both
    expect(mod(f1, g)).toEqual([]);
    expect(mod(f2, g)).toEqual([]);
  });
});
