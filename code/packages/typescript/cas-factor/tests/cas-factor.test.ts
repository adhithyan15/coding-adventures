import { describe, expect, it } from "vitest";
import {
  bzhFactor,
  content,
  degree,
  divideLinear,
  divisors,
  evaluate,
  extractLinearFactors,
  factorIntegerPolynomial,
  findIntegerRoots,
  kroneckerFactor,
  normalize,
  primitivePart,
} from "../src/index";

function sortedFactors(factors: Array<[bigint[], number]>): Array<[string, number]> {
  return factors.map(([poly, mult]) => [poly.join(","), mult] as [string, number]).sort();
}

function verifyFactorization(original: readonly bigint[] | readonly number[], factors: readonly bigint[][]): void {
  for (let x = -5; x <= 5; x += 1) {
    let product = 1n;
    for (const factor of factors) {
      product *= evaluate(factor, x);
    }
    expect(product).toBe(evaluate(original, x));
  }
}

describe("polynomial helpers", () => {
  it("normalizes trailing zeros", () => {
    expect(normalize([1, 2, 0, 0])).toEqual([1n, 2n]);
    expect(normalize([0, 0])).toEqual([]);
  });

  it("computes degree, content, primitive part, evaluation, and divisors", () => {
    expect(degree([1, 2, 3])).toBe(2);
    expect(degree([])).toBe(-1);
    expect(content([2, 4, 6])).toBe(2n);
    expect(content([-6, 4, 2])).toBe(2n);
    expect(content([])).toBe(0n);
    expect(primitivePart([2, 4, 6])).toEqual([1n, 2n, 3n]);
    expect(evaluate([1, 2, 3], 2)).toBe(17n);
    expect(divisors(12)).toEqual([1n, 2n, 3n, 4n, 6n, 12n]);
    expect(divisors(-12)).toEqual([1n, 2n, 3n, 4n, 6n, 12n]);
    expect(divisors(0)).toEqual([]);
  });

  it("divides by a known linear root", () => {
    expect(divideLinear([-1, 0, 1], 1)).toEqual([1n, 1n]);
    expect(divideLinear([-6, 11, -6, 1], 1)).toEqual([6n, -5n, 1n]);
  });
});

describe("integer roots", () => {
  it("finds integer roots", () => {
    expect(findIntegerRoots([-1, 0, 1])).toEqual([-1n, 1n]);
    expect(findIntegerRoots([-6, 11, -6, 1])).toEqual([1n, 2n, 3n]);
    expect(findIntegerRoots([1, 0, 1])).toEqual([]);
    expect(findIntegerRoots([0, -1, 1])).toEqual([0n, 1n]);
  });

  it("extracts linear factors with multiplicity", () => {
    expect(extractLinearFactors([-1, 0, 1])).toEqual([
      [[-1n, 1], [1n, 1]],
      [1n],
    ]);
    expect(extractLinearFactors([1, 2, 1])).toEqual([[[-1n, 2]], [1n]]);
    expect(extractLinearFactors([1, 0, 1])).toEqual([[], [1n, 0n, 1n]]);
  });
});

describe("bzhFactor", () => {
  it("factors x^5 - 1", () => {
    const factors = bzhFactor([-1, 0, 0, 0, 0, 1]);
    expect(factors).not.toBeNull();
    expect(factors?.map((factor) => factor.length - 1).sort()).toEqual([1, 4]);
    verifyFactorization([-1, 0, 0, 0, 0, 1], factors ?? []);
  });

  it("factors x^8 - 1", () => {
    const factors = bzhFactor([-1, 0, 0, 0, 0, 0, 0, 0, 1]);
    expect(factors).not.toBeNull();
    verifyFactorization([-1, 0, 0, 0, 0, 0, 0, 0, 1], factors ?? []);
  });

  it("returns null for irreducible x^4 + 1", () => {
    expect(bzhFactor([1, 0, 0, 0, 1])).toBeNull();
  });

  it("returns null for non-monic inputs", () => {
    expect(bzhFactor([1, 0, 0, 0, 2])).toBeNull();
  });
});

describe("factorIntegerPolynomial", () => {
  it("factors x^2 - 1", () => {
    const [c, factors] = factorIntegerPolynomial([-1, 0, 1]);
    expect(c).toBe(1n);
    expect(sortedFactors(factors)).toEqual(sortedFactors([[[-1n, 1n], 1], [[1n, 1n], 1]]));
  });

  it("factors content and repeated roots", () => {
    const [c, factors] = factorIntegerPolynomial([2, 4, 2]);
    expect(c).toBe(2n);
    expect(factors).toEqual([[[1n, 1n], 2]]);
  });

  it("keeps irreducible quadratics", () => {
    expect(factorIntegerPolynomial([1, 0, 1])).toEqual([1n, [[[1n, 0n, 1n], 1]]]);
  });

  it("factors a cubic into linear factors", () => {
    const [c, factors] = factorIntegerPolynomial([-6, 11, -6, 1]);
    expect(c).toBe(1n);
    expect(sortedFactors(factors)).toEqual(sortedFactors([
      [[-1n, 1n], 1],
      [[-2n, 1n], 1],
      [[-3n, 1n], 1],
    ]));
  });

  it("handles the zero polynomial", () => {
    expect(factorIntegerPolynomial([])).toEqual([0n, []]);
  });

  it("uses Kronecker for Sophie Germain quartics", () => {
    const [c, factors] = factorIntegerPolynomial([4, 0, 0, 0, 1]);
    expect(c).toBe(1n);
    expect(factors).toHaveLength(2);
    for (let x = -5; x <= 5; x += 1) {
      let product = 1n;
      for (const [poly, mult] of factors) {
        product *= evaluate(poly, x) ** BigInt(mult);
      }
      expect(product).toBe(evaluate([4, 0, 0, 0, 1], x));
    }
  });

  it("factors x^4 + x^2 + 1", () => {
    const [c, factors] = factorIntegerPolynomial([1, 0, 1, 0, 1]);
    expect(c).toBe(1n);
    expect(factors).toHaveLength(2);
    for (let x = -5; x <= 5; x += 1) {
      let product = 1n;
      for (const [poly, mult] of factors) {
        product *= evaluate(poly, x) ** BigInt(mult);
      }
      expect(product).toBe(evaluate([1, 0, 1, 0, 1], x));
    }
  });

  it("factors repeated irreducible quadratics", () => {
    const [c, factors] = factorIntegerPolynomial([1, 0, 2, 0, 1]);
    expect(c).toBe(1n);
    expect(factors).toEqual([[[1n, 0n, 1n], 2]]);
  });

  it("keeps a quadratic residual after a linear factor", () => {
    const [c, factors] = factorIntegerPolynomial([-2, 1, -2, 1]);
    expect(c).toBe(1n);
    expect(sortedFactors(factors)).toEqual(sortedFactors([
      [[-2n, 1n], 1],
      [[1n, 0n, 1n], 1],
    ]));
  });

  it("uses BZH fallback for x^5 - 1 after linear extraction", () => {
    const [c, factors] = factorIntegerPolynomial([-1, 0, 0, 0, 0, 1]);
    expect(c).toBe(1n);
    expect(sortedFactors(factors)).toEqual(sortedFactors([
      [[-1n, 1n], 1],
      [[1n, 1n, 1n, 1n, 1n], 1],
    ]));
  });

  it("extracts content before BZH fallback", () => {
    const [c, factors] = factorIntegerPolynomial([-2, 0, 0, 0, 0, 2]);
    expect(c).toBe(2n);
    verifyFactorization([-1, 0, 0, 0, 0, 1], factors.flatMap(([factor, mult]) => Array(mult).fill(factor)));
  });

  it("keeps x^4 + 1 irreducible through the integrated path", () => {
    expect(factorIntegerPolynomial([2, 0, 0, 0, 2])).toEqual([2n, [[[1n, 0n, 0n, 0n, 1n], 1]]]);
  });

  it("factors BZH residuals and then recurses through Kronecker", () => {
    const [c, factors] = factorIntegerPolynomial([-1, 0, 0, 0, 0, 0, 1]);
    expect(c).toBe(1n);
    verifyFactorization([-1, 0, 0, 0, 0, 0, 1], factors.flatMap(([factor, mult]) => Array(mult).fill(factor)));
  });
});

describe("kroneckerFactor", () => {
  it("returns null for irreducible quadratics", () => {
    expect(kroneckerFactor([1, 0, 1])).toBeNull();
  });
});
