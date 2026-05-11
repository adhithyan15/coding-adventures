import { describe, expect, it } from "vitest";
import { ADD, MUL, POW, SQRT, SUB, app, int, sym } from "@coding-adventures/symbolic-ir";
import {
  algFactorIr,
  extractRadicalD,
  factorOverExtension,
  rationalSquareRoot,
  trySplitDepressedQuartic,
  trySplitQuadratic,
} from "../src/index";

function sqrtD(d: bigint | number) {
  return app(SQRT, [int(d)]);
}

describe("rationalSquareRoot", () => {
  it("detects exact rational squares", () => {
    expect(rationalSquareRoot({ numer: 4n, denom: 1n })).toEqual({ numer: 2n, denom: 1n });
    expect(rationalSquareRoot({ numer: 1n, denom: 4n })).toEqual({ numer: 1n, denom: 2n });
    expect(rationalSquareRoot({ numer: 2n, denom: 1n })).toBeNull();
    expect(rationalSquareRoot({ numer: -1n, denom: 1n })).toBeNull();
  });
});

describe("quadratic extension factoring", () => {
  it("splits x^2 - d over Q[sqrt(d)]", () => {
    const result = trySplitQuadratic([-2, 0, 1], 2);
    expect(result).not.toBeNull();
    expect(result).toHaveLength(2);
    const radicals = result?.map((factor) => factor[0].radical.numer).sort();
    expect(radicals).toEqual([-1n, 1n]);
  });

  it("does not split wrong discriminants", () => {
    expect(trySplitQuadratic([2, 0, 1], 2)).toBeNull();
    expect(trySplitQuadratic([1, 1, 1], 2)).toBeNull();
  });

  it("splits x^4 + 1 over sqrt(2)", () => {
    const result = factorOverExtension([1, 0, 0, 0, 1], 2);
    expect(result).not.toBeNull();
    expect(result).toHaveLength(2);
    const radicals = result?.map((factor) => factor[1].radical.numer).sort();
    expect(radicals).toEqual([-1n, 1n]);
  });

  it("does not split x^4 + 1 over sqrt(3)", () => {
    expect(factorOverExtension([1, 0, 0, 0, 1], 3)).toBeNull();
  });

  it("checks depressed quartic shape", () => {
    expect(trySplitDepressedQuartic([1, 0, 0, 1, 1], 2)).toBeNull();
    expect(trySplitDepressedQuartic([2, 0, 0, 0, 1], 2)).toBeNull();
  });

  it("keeps rational factors when residuals split", () => {
    const result = factorOverExtension([2, -2, -1, 1], 2);
    expect(result).not.toBeNull();
    expect(result).toHaveLength(3);
    expect(result?.some((factor) => factor.length === 2 && factor[0].rational.numer === -1n)).toBe(true);
  });
});

describe("IR adapter", () => {
  it("extracts radical extensions", () => {
    expect(extractRadicalD(sqrtD(2))).toBe(2n);
    expect(extractRadicalD(sqrtD(4))).toBeNull();
    expect(extractRadicalD(int(2))).toBeNull();
  });

  it("factors a Pow polynomial", () => {
    const x = sym("x");
    const poly = app(SUB, [app(POW, [x, int(2)]), int(2)]);
    const result = algFactorIr(poly, sqrtD(2), x);
    expect(result?.kind).toBe("apply");
  });

  it("factors a nested multiplication polynomial", () => {
    const x = sym("x");
    const x4 = app(MUL, [x, x, x, x]);
    const result = algFactorIr(app(ADD, [x4, int(1)]), sqrtD(2), x);
    expect(result?.kind).toBe("apply");
  });

  it("returns null for non-polynomials", () => {
    const x = sym("x");
    expect(algFactorIr(app(sym("Sin"), [x]), sqrtD(2), x)).toBeNull();
  });
});
