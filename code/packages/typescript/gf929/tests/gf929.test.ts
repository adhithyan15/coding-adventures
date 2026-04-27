/**
 * Tests for @coding-adventures/gf929 — GF(929) prime field arithmetic.
 *
 * Test strategy:
 * 1. Field axioms (identity, commutativity, associativity, distributivity)
 * 2. Known values from the spec and manual computations
 * 3. Table integrity (exp/log round-trips)
 * 4. Fermat's little theorem
 * 5. Edge cases (zero, one, boundaries)
 * 6. Error conditions (div by zero, invalid exponent)
 */

import { describe, it, expect } from "vitest";
import {
  PRIME,
  ORDER,
  ALPHA,
  EXP,
  LOG,
  add,
  subtract,
  multiply,
  divide,
  power,
  inverse,
  zero,
  one,
  isElement,
} from "../src/index.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

describe("constants", () => {
  it("PRIME is 929", () => {
    expect(PRIME).toBe(929);
  });

  it("ORDER is 928", () => {
    expect(ORDER).toBe(928);
  });

  it("ALPHA (generator) is 3", () => {
    expect(ALPHA).toBe(3);
  });

  it("929 is prime", () => {
    // Check no prime ≤ sqrt(929) ≈ 30.5 divides 929.
    const primes = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29];
    for (const p of primes) {
      expect(929 % p).not.toBe(0);
    }
  });
});

// ---------------------------------------------------------------------------
// Table integrity
// ---------------------------------------------------------------------------

describe("EXP and LOG tables", () => {
  it("EXP has 929 entries", () => {
    expect(EXP.length).toBe(929);
  });

  it("LOG has 929 entries", () => {
    expect(LOG.length).toBe(929);
  });

  it("EXP[0] = 1 (α^0 = 1)", () => {
    expect(EXP[0]).toBe(1);
  });

  it("EXP[1] = 3 (α^1 = 3)", () => {
    expect(EXP[1]).toBe(3);
  });

  it("EXP[2] = 9 (α^2 = 3^2 = 9)", () => {
    expect(EXP[2]).toBe(9);
  });

  it("EXP[3] = 27 (α^3 = 3^3 = 27)", () => {
    expect(EXP[3]).toBe(27);
  });

  it("EXP[928] = 1 (wrap-around convenience entry)", () => {
    expect(EXP[928]).toBe(1);
  });

  it("LOG[1] = 0 (log of multiplicative identity)", () => {
    expect(LOG[1]).toBe(0);
  });

  it("LOG[3] = 1 (log of generator α)", () => {
    expect(LOG[3]).toBe(1);
  });

  it("EXP/LOG round-trip for all non-zero elements", () => {
    for (let v = 1; v <= 928; v++) {
      expect(EXP[LOG[v]]).toBe(v);
    }
  });

  it("LOG/EXP round-trip for all exponents 0..927", () => {
    for (let i = 0; i < 928; i++) {
      expect(LOG[EXP[i]]).toBe(i);
    }
  });

  it("EXP table covers all 928 non-zero elements exactly once", () => {
    const seen = new Set<number>();
    for (let i = 0; i < 928; i++) {
      const v = EXP[i];
      expect(v).toBeGreaterThanOrEqual(1);
      expect(v).toBeLessThanOrEqual(928);
      expect(seen.has(v)).toBe(false); // no duplicates
      seen.add(v);
    }
    expect(seen.size).toBe(928); // covers all 928 non-zero elements
  });

  it("EXP[4] = 81 (3^4 = 81)", () => {
    expect(EXP[4]).toBe(81);
  });

  it("EXP[7] = 2187 mod 929 = 329 (3^7 = 2187, 2187 mod 929 = 329)", () => {
    // 2187 / 929 = 2 remainder 329
    expect(2187 % 929).toBe(329);
    expect(EXP[7]).toBe(329);
  });
});

// ---------------------------------------------------------------------------
// add()
// ---------------------------------------------------------------------------

describe("add()", () => {
  it("add(0, 0) = 0", () => {
    expect(add(0, 0)).toBe(0);
  });

  it("add(0, 500) = 500 (identity element)", () => {
    expect(add(0, 500)).toBe(500);
  });

  it("add(500, 0) = 500 (commutativity with 0)", () => {
    expect(add(500, 0)).toBe(500);
  });

  it("add(100, 900) = 71 (spec example)", () => {
    // (100 + 900) mod 929 = 1000 mod 929 = 71
    expect(add(100, 900)).toBe(71);
  });

  it("add(928, 1) = 0 (wraps around: 929 mod 929 = 0)", () => {
    expect(add(928, 1)).toBe(0);
  });

  it("add(928, 928) = 927 (2*928 = 1856, 1856 mod 929 = 927)", () => {
    expect(add(928, 928)).toBe(927);
  });

  it("add is commutative for sample pairs", () => {
    const pairs = [[5, 10], [100, 800], [300, 700], [0, 928], [464, 465]];
    for (const [a, b] of pairs) {
      expect(add(a!, b!)).toBe(add(b!, a!));
    }
  });

  it("add is associative for sample triples", () => {
    const triples = [[10, 20, 30], [100, 200, 700], [400, 500, 600]];
    for (const [a, b, c] of triples) {
      expect(add(add(a!, b!), c!)).toBe(add(a!, add(b!, c!)));
    }
  });

  it("additive inverse: add(a, PRIME - a) = 0 for several a", () => {
    for (const a of [1, 100, 464, 928]) {
      expect(add(a, PRIME - a)).toBe(0);
    }
  });
});

// ---------------------------------------------------------------------------
// subtract()
// ---------------------------------------------------------------------------

describe("subtract()", () => {
  it("sub(10, 5) = 5", () => {
    expect(subtract(10, 5)).toBe(5);
  });

  it("sub(5, 10) = 924 (spec example: (5 - 10 + 929) mod 929 = 924)", () => {
    expect(subtract(5, 10)).toBe(924);
  });

  it("sub(0, 1) = 928 (additive inverse of 1)", () => {
    expect(subtract(0, 1)).toBe(928);
  });

  it("sub(500, 0) = 500", () => {
    expect(subtract(500, 0)).toBe(500);
  });

  it("sub(a, a) = 0 for all a", () => {
    for (const a of [0, 1, 100, 464, 928]) {
      expect(subtract(a, a)).toBe(0);
    }
  });

  it("sub(a, b) = add(a, PRIME - b) for several pairs", () => {
    const pairs = [[100, 50], [5, 10], [928, 1], [0, 0]];
    for (const [a, b] of pairs) {
      expect(subtract(a!, b!)).toBe(add(a!, (PRIME - b!) % PRIME));
    }
  });
});

// ---------------------------------------------------------------------------
// multiply()
// ---------------------------------------------------------------------------

describe("multiply()", () => {
  it("mul(0, 0) = 0", () => {
    expect(multiply(0, 0)).toBe(0);
  });

  it("mul(0, 500) = 0 (zero annihilates)", () => {
    expect(multiply(0, 500)).toBe(0);
  });

  it("mul(500, 0) = 0 (zero annihilates)", () => {
    expect(multiply(500, 0)).toBe(0);
  });

  it("mul(1, 500) = 500 (multiplicative identity)", () => {
    expect(multiply(1, 500)).toBe(500);
  });

  it("mul(500, 1) = 500 (multiplicative identity)", () => {
    expect(multiply(500, 1)).toBe(500);
  });

  it("mul(3, 3) = 9", () => {
    expect(multiply(3, 3)).toBe(9);
  });

  it("mul(9, 3) = 27", () => {
    expect(multiply(9, 3)).toBe(27);
  });

  it("mul(400, 400) = 160000 mod 929", () => {
    const expected = 160000 % 929;
    expect(multiply(400, 400)).toBe(expected);
  });

  it("mul(928, 928) = 928*928 mod 929", () => {
    const expected = (928 * 928) % 929;
    expect(multiply(928, 928)).toBe(expected);
  });

  it("multiply is commutative for sample pairs", () => {
    const pairs = [[3, 9], [100, 200], [400, 400], [1, 928]];
    for (const [a, b] of pairs) {
      expect(multiply(a!, b!)).toBe(multiply(b!, a!));
    }
  });

  it("multiply is associative for sample triples", () => {
    const triples = [[2, 3, 5], [10, 20, 30]];
    for (const [a, b, c] of triples) {
      expect(multiply(multiply(a!, b!), c!)).toBe(multiply(a!, multiply(b!, c!)));
    }
  });

  it("distributivity: a*(b+c) = a*b + a*c", () => {
    const [a, b, c] = [7, 100, 200];
    expect(multiply(a, add(b, c))).toBe(add(multiply(a, b), multiply(a, c)));
  });

  it("mul(27, 81) = 3^3 * 3^4 = 3^7 = 329", () => {
    // 27 = 3^3, 81 = 3^4, so 27*81 = 3^7 = EXP[7] = 329
    expect(multiply(27, 81)).toBe(329);
  });
});

// ---------------------------------------------------------------------------
// inverse()
// ---------------------------------------------------------------------------

describe("inverse()", () => {
  it("inverse(3) = 310 (spec: 3 × 310 = 930 ≡ 1 mod 929)", () => {
    // Verify the inverse directly: 3 * 310 = 930; 930 mod 929 = 1
    expect(3 * 310 % 929).toBe(1);
    expect(inverse(3)).toBe(310);
  });

  it("inverse(1) = 1 (multiplicative identity is its own inverse)", () => {
    expect(inverse(1)).toBe(1);
  });

  it("inverse(928) = 928 (928 ≡ -1 mod 929, so (-1)*(-1)=1)", () => {
    expect(multiply(928, inverse(928))).toBe(1);
  });

  it("a × inverse(a) = 1 for all non-zero elements (spot checks)", () => {
    const samples = [1, 2, 3, 10, 100, 310, 464, 500, 928];
    for (const a of samples) {
      expect(multiply(a, inverse(a))).toBe(1);
    }
  });

  it("a × inverse(a) = 1 for all 928 non-zero elements", () => {
    for (let a = 1; a <= 928; a++) {
      expect(multiply(a, inverse(a))).toBe(1);
    }
  });

  it("inverse(0) throws", () => {
    expect(() => inverse(0)).toThrow("GF929");
  });
});

// ---------------------------------------------------------------------------
// divide()
// ---------------------------------------------------------------------------

describe("divide()", () => {
  it("div(9, 3) = 3", () => {
    expect(divide(9, 3)).toBe(3);
  });

  it("div(0, 5) = 0 (zero divided by anything is zero)", () => {
    expect(divide(0, 5)).toBe(0);
  });

  it("div(500, 1) = 500", () => {
    expect(divide(500, 1)).toBe(500);
  });

  it("div(a, a) = 1 for non-zero a", () => {
    for (const a of [1, 3, 100, 500, 928]) {
      expect(divide(a, a)).toBe(1);
    }
  });

  it("div(a, b) * b = a for sample pairs", () => {
    const pairs = [[100, 50], [9, 3], [500, 250], [928, 2]];
    for (const [a, b] of pairs) {
      expect(multiply(divide(a!, b!), b!)).toBe(a!);
    }
  });

  it("div(a, 0) throws", () => {
    expect(() => divide(5, 0)).toThrow("GF929");
  });
});

// ---------------------------------------------------------------------------
// power()
// ---------------------------------------------------------------------------

describe("power()", () => {
  it("pow(3, 0) = 1", () => {
    expect(power(3, 0)).toBe(1);
  });

  it("pow(3, 1) = 3", () => {
    expect(power(3, 1)).toBe(3);
  });

  it("pow(3, 2) = 9", () => {
    expect(power(3, 2)).toBe(9);
  });

  it("pow(3, 3) = 27", () => {
    expect(power(3, 3)).toBe(27);
  });

  it("pow(3, 928) = 1 (Fermat's little theorem: α^{p-1} ≡ 1 mod p)", () => {
    expect(power(3, 928)).toBe(1);
  });

  it("pow(100, 928) = 1 (any non-zero element satisfies Fermat)", () => {
    expect(power(100, 928)).toBe(1);
  });

  it("pow(0, 0) = 1 by convention", () => {
    expect(power(0, 0)).toBe(1);
  });

  it("pow(0, 5) = 0", () => {
    expect(power(0, 5)).toBe(0);
  });

  it("pow(1, 1000) = 1", () => {
    expect(power(1, 1000)).toBe(1);
  });

  it("pow(3, 927) = inverse(3) = 310", () => {
    // By Fermat: 3^{927} ≡ 3^{-1} ≡ 310 mod 929
    expect(power(3, 927)).toBe(310);
    expect(power(3, 927)).toBe(inverse(3));
  });

  it("pow throws for negative exponent", () => {
    expect(() => power(3, -1)).toThrow("GF929");
  });

  it("pow throws for non-integer exponent", () => {
    expect(() => power(3, 1.5)).toThrow("GF929");
  });

  it("pow(a, 2) = mul(a, a) for several a", () => {
    for (const a of [2, 3, 10, 100, 500]) {
      expect(power(a, 2)).toBe(multiply(a, a));
    }
  });
});

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

describe("zero() and one()", () => {
  it("zero() returns 0", () => {
    expect(zero()).toBe(0);
  });

  it("one() returns 1", () => {
    expect(one()).toBe(1);
  });

  it("add(zero(), x) = x", () => {
    expect(add(zero(), 500)).toBe(500);
  });

  it("multiply(one(), x) = x", () => {
    expect(multiply(one(), 500)).toBe(500);
  });
});

describe("isElement()", () => {
  it("isElement(0) = true", () => {
    expect(isElement(0)).toBe(true);
  });

  it("isElement(928) = true", () => {
    expect(isElement(928)).toBe(true);
  });

  it("isElement(929) = false (out of range)", () => {
    expect(isElement(929)).toBe(false);
  });

  it("isElement(-1) = false", () => {
    expect(isElement(-1)).toBe(false);
  });

  it("isElement(1.5) = false (non-integer)", () => {
    expect(isElement(1.5)).toBe(false);
  });

  it("isElement(464) = true (middle value)", () => {
    expect(isElement(464)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Field axiom: Fermat's little theorem exhaustive check
// ---------------------------------------------------------------------------

describe("Fermat's little theorem", () => {
  it("a^928 ≡ 1 (mod 929) for a = 1..928 (spot check 20 values)", () => {
    // Check every 46th element to cover a range without being too slow.
    for (let a = 1; a <= 928; a += 46) {
      expect(power(a, ORDER)).toBe(1);
    }
  });
});

// ---------------------------------------------------------------------------
// PDF417-specific: ECC generator polynomial roots
// ---------------------------------------------------------------------------

describe("PDF417 RS generator roots (b=3 convention)", () => {
  it("EXP[3] = α^3 = 27", () => {
    expect(EXP[3]).toBe(27);
  });

  it("EXP[4] = α^4 = 81", () => {
    expect(EXP[4]).toBe(81);
  });

  it("EXP[7] = α^7 = 329 (2187 mod 929)", () => {
    expect(EXP[7]).toBe(329);
  });

  it("27 × 81 = 2187 mod 929 = 329 (product of α^3 and α^4 = α^7)", () => {
    expect(multiply(27, 81)).toBe(329);
  });
});
