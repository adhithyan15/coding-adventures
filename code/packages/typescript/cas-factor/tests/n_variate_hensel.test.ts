/**
 * Tests for n-variate (n ≥ 3) Hensel lifting — Track K2 (TS port of
 * Python Track K1, PR #5590).
 *
 * Strategy: build the input via multiplying known factors with `_internals.nMul`
 * (or by enumerating monomials), run `tryNVariateHensel`, and verify the
 * product of returned factors equals the input.  Factor ordering is not
 * pinned — different specialisation tuples may yield permutations.
 */

import { describe, expect, it } from "vitest";
import { tryNVariateHensel, BiRational } from "../src/index";
import type { NPoly } from "../src/index";
import { _internals } from "../src/hensel";

function nKey(tup: number[]): string {
  return tup.join(",");
}

function make(numVars: number, terms: Array<[number[], number]>): NPoly {
  const out: NPoly = new Map();
  for (const [k, c] of terms) {
    if (k.length !== numVars) throw new Error("tuple length mismatch");
    if (c === 0) continue;
    const key = nKey(k);
    const cur = out.get(key) ?? BiRational.ZERO;
    out.set(key, cur.add(BiRational.fromInt(c)));
  }
  return _internals.nNormalize(out);
}

function verifyProduct(numVars: number, factors: NPoly[], expected: NPoly): boolean {
  let prod: NPoly = _internals.nOne(numVars);
  for (const f of factors) prod = _internals.nMul(prod, f, numVars);
  // Equality: same key set, equal coefficients.
  const expN = _internals.nNormalize(expected);
  if (prod.size !== expN.size) return false;
  for (const [k, v] of prod) {
    const w = expN.get(k);
    if (w === undefined || !w.equals(v)) return false;
  }
  return true;
}

describe("tryNVariateHensel — acceptance", () => {
  it("trivariate x² − y² − z² − 2yz = (x+y+z)(x−y−z)", () => {
    const poly = make(3, [
      [[2, 0, 0], 1],
      [[0, 2, 0], -1],
      [[0, 0, 2], -1],
      [[0, 1, 1], -2],
    ]);
    const result = tryNVariateHensel(poly, 3);
    expect(result).not.toBeNull();
    expect(result!.length).toBeGreaterThanOrEqual(2);
    expect(verifyProduct(3, result!, poly)).toBe(true);
  });

  it("trivariate (x+y+z)(x+2y+3z) = x²+3xy+4xz+2y²+5yz+3z²", () => {
    const poly = make(3, [
      [[2, 0, 0], 1],
      [[1, 1, 0], 3],
      [[1, 0, 1], 4],
      [[0, 2, 0], 2],
      [[0, 1, 1], 5],
      [[0, 0, 2], 3],
    ]);
    const result = tryNVariateHensel(poly, 3);
    expect(result).not.toBeNull();
    expect(result!.length).toBe(2);
    expect(verifyProduct(3, result!, poly)).toBe(true);
  });

  it("trivariate sum-of-cubes companion: (x+y+z)(x²+y²+z²−xy−yz−xz)", () => {
    const factorA = make(3, [
      [[1, 0, 0], 1],
      [[0, 1, 0], 1],
      [[0, 0, 1], 1],
    ]);
    const factorB = make(3, [
      [[2, 0, 0], 1],
      [[0, 2, 0], 1],
      [[0, 0, 2], 1],
      [[1, 1, 0], -1],
      [[0, 1, 1], -1],
      [[1, 0, 1], -1],
    ]);
    const poly = _internals.nMul(factorA, factorB, 3);
    const result = tryNVariateHensel(poly, 3);
    expect(result).not.toBeNull();
    expect(result!.length).toBeGreaterThanOrEqual(2);
    expect(verifyProduct(3, result!, poly)).toBe(true);
  });

  it("quadrivariate (x+y)(x+z+w) — iterated lift across two aux vars", () => {
    const factorA = make(4, [
      [[1, 0, 0, 0], 1],
      [[0, 1, 0, 0], 1],
    ]);
    const factorB = make(4, [
      [[1, 0, 0, 0], 1],
      [[0, 0, 1, 0], 1],
      [[0, 0, 0, 1], 1],
    ]);
    const poly = _internals.nMul(factorA, factorB, 4);
    const result = tryNVariateHensel(poly, 4);
    expect(result).not.toBeNull();
    expect(result!.length).toBe(2);
    expect(verifyProduct(4, result!, poly)).toBe(true);
  });
});

describe("tryNVariateHensel — fall-through", () => {
  it("returns null for irreducible x² + y² + z² + 1", () => {
    const poly = make(3, [
      [[2, 0, 0], 1],
      [[0, 2, 0], 1],
      [[0, 0, 2], 1],
      [[0, 0, 0], 1],
    ]);
    expect(tryNVariateHensel(poly, 3)).toBeNull();
  });

  it("returns null for univariate-in-three-var-ring x² − 1", () => {
    const poly = make(3, [
      [[2, 0, 0], 1],
      [[0, 0, 0], -1],
    ]);
    expect(tryNVariateHensel(poly, 3)).toBeNull();
  });

  it("returns null when numVars < 2", () => {
    const poly = make(1, [
      [[2], 1],
      [[0], -1],
    ]);
    expect(tryNVariateHensel(poly, 1)).toBeNull();
  });

  it("returns null for pure constant", () => {
    const poly = make(3, [[[0, 0, 0], 7]]);
    expect(tryNVariateHensel(poly, 3)).toBeNull();
  });

  it("returns null for empty polynomial", () => {
    const poly: NPoly = new Map();
    expect(tryNVariateHensel(poly, 3)).toBeNull();
  });

  it("returns null for linear x + y + z (irreducible)", () => {
    const poly = make(3, [
      [[1, 0, 0], 1],
      [[0, 1, 0], 1],
      [[0, 0, 1], 1],
    ]);
    expect(tryNVariateHensel(poly, 3)).toBeNull();
  });
});

describe("tryNVariateHensel — bivariate regressions via n-variate path", () => {
  it("x² + xy − 2y² factors via the n-variate front door", () => {
    const poly = make(2, [
      [[2, 0], 1],
      [[1, 1], 1],
      [[0, 2], -2],
    ]);
    const result = tryNVariateHensel(poly, 2);
    expect(result).not.toBeNull();
    expect(result!.length).toBe(2);
    expect(verifyProduct(2, result!, poly)).toBe(true);
  });

  it("x³ − y³ factors via the n-variate front door", () => {
    const poly = make(2, [
      [[3, 0], 1],
      [[0, 3], -1],
    ]);
    const result = tryNVariateHensel(poly, 2);
    expect(result).not.toBeNull();
    expect(result!.length).toBe(2);
    expect(verifyProduct(2, result!, poly)).toBe(true);
  });
});

describe("tryNVariateHensel — bounded resource discipline", () => {
  it("high-degree irreducible does not loop forever", () => {
    // x^4 + y^2 + z^2 + 1 — irreducible over Q (x^4 + const is irreducible
    // for non-square positive const).
    const poly = make(3, [
      [[4, 0, 0], 1],
      [[0, 0, 0], 1],
      [[0, 0, 2], 1],
      [[0, 2, 0], 1],
    ]);
    expect(tryNVariateHensel(poly, 3)).toBeNull();
  });
});
