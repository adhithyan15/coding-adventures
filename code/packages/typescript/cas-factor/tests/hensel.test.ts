/**
 * Acceptance tests for bivariate Hensel lifting.
 *
 * Mirrors the Python ``test_hensel.py`` suite — same 5 acceptance cases
 * plus a univariate fall-through regression — to guarantee cross-language
 * parity with the Python reference.
 */

import { describe, expect, it } from "vitest";
import { tryBivariateHensel, BiRational } from "../src/index";
import type { BiPoly } from "../src/index";
import { _internals } from "../src/hensel";

function make(terms: Array<[number, number, number]>): BiPoly {
  const out: BiPoly = new Map();
  for (const [i, j, c] of terms) {
    if (c === 0) continue;
    const k = _internals.key(i, j);
    const cur = out.get(k) ?? BiRational.ZERO;
    out.set(k, cur.add(BiRational.fromInt(c)));
  }
  return _internals.biNormalize(out);
}

function verifyProduct(factors: BiPoly[], expected: BiPoly): boolean {
  let prod: BiPoly = new Map([[_internals.key(0, 0), BiRational.ONE]]);
  for (const f of factors) prod = _internals.biMul(prod, f);
  return _internals.biEquals(prod, expected);
}

describe("tryBivariateHensel", () => {
  it("factors x^2 + xy - 2y^2 = (x+2y)(x-y)", () => {
    const f = make([[2, 0, 1], [1, 1, 1], [0, 2, -2]]);
    const result = tryBivariateHensel(f);
    expect(result).not.toBeNull();
    expect(result!.length).toBe(2);
    expect(verifyProduct(result!, f)).toBe(true);
  });

  it("factors 2x^2 + 3xy - 2y^2 (non-unit leading coefficient)", () => {
    const f = make([[2, 0, 2], [1, 1, 3], [0, 2, -2]]);
    const result = tryBivariateHensel(f);
    expect(result).not.toBeNull();
    expect(result!.length).toBeGreaterThanOrEqual(2);
    expect(verifyProduct(result!, f)).toBe(true);
  });

  it("factors x^3 - y^3 = (x-y)(x^2+xy+y^2)", () => {
    const f = make([[3, 0, 1], [0, 3, -1]]);
    const result = tryBivariateHensel(f);
    expect(result).not.toBeNull();
    expect(result!.length).toBe(2);
    expect(verifyProduct(result!, f)).toBe(true);
  });

  it("returns null for irreducible x^2 + y^2 + 1", () => {
    const f = make([[2, 0, 1], [0, 2, 1], [0, 0, 1]]);
    const result = tryBivariateHensel(f);
    expect(result).toBeNull();
  });

  it("returns null for univariate x^2 - 1 (falls through to caller)", () => {
    const f = make([[2, 0, 1], [0, 0, -1]]);
    const result = tryBivariateHensel(f);
    expect(result).toBeNull();
  });

  it("returns null for already-linear x + y", () => {
    const f = make([[1, 0, 1], [0, 1, 1]]);
    const result = tryBivariateHensel(f);
    expect(result).toBeNull();
  });
});
