/**
 * Tests for the Track J2 Taylor-series limit fallback.
 *
 * Mirrors the Python reference tests in
 * `code/packages/python/cas-limit-series/tests/test_series_limit.py`.
 *
 * Each acceptance case is taken straight from the
 * `macsyma-truly-finish-plan` Track J1/J2 spec.
 */

import { describe, expect, it } from "vitest";
import {
  ADD,
  COS,
  DIV,
  EXP,
  LOG,
  MUL,
  NEG,
  POW,
  SIN,
  SUB,
  TAN,
  app,
  equals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import { limitAdvanced, trySeriesLimit } from "../src/index";

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

function x(): IRNode {
  return sym("x");
}

function eq(actual: IRNode, expected: IRNode): void {
  expect(
    equals(actual, expected),
    `actual: ${display(actual)} expected: ${display(expected)}`,
  ).toBe(true);
}

function display(node: IRNode): string {
  return JSON.stringify(node, (_k, v) => (typeof v === "bigint" ? v.toString() : v));
}

// ---------------------------------------------------------------------------
// Acceptance cases (Track J1/J2 spec)
// ---------------------------------------------------------------------------

describe("trySeriesLimit — acceptance cases", () => {
  it("(sin(x) - x)/x^3 → -1/6", () => {
    const expr = app(DIV, [app(SUB, [app(SIN, [x()]), x()]), app(POW, [x(), int(3)])]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, rational(-1, 6));
  });

  it("(1 - cos(x))/x^2 → 1/2", () => {
    const expr = app(DIV, [app(SUB, [int(1), app(COS, [x()])]), app(POW, [x(), int(2)])]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, rational(1, 2));
  });

  it("(exp(x) - 1 - x)/x^2 → 1/2", () => {
    const numer = app(SUB, [app(SUB, [app(EXP, [x()]), int(1)]), x()]);
    const expr = app(DIV, [numer, app(POW, [x(), int(2)])]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, rational(1, 2));
  });

  it("(tan(x) - x)/x^3 → 1/3", () => {
    const expr = app(DIV, [app(SUB, [app(TAN, [x()]), x()]), app(POW, [x(), int(3)])]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, rational(1, 3));
  });

  it("(log(1 + x) - x)/x^2 → -1/2", () => {
    const onePlusX = app(ADD, [int(1), x()]);
    const numer = app(SUB, [app(LOG, [onePlusX]), x()]);
    const expr = app(DIV, [numer, app(POW, [x(), int(2)])]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, rational(-1, 2));
  });

  it("(sin(x) - x) / (exp(x^2) - 1) → 0 (leading u^3 vs u^2)", () => {
    const numer = app(SUB, [app(SIN, [x()]), x()]);
    const denom = app(SUB, [app(EXP, [app(POW, [x(), int(2)])]), int(1)]);
    const expr = app(DIV, [numer, denom]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, int(0));
  });
});

// ---------------------------------------------------------------------------
// Regression — sin(x)/x and x^2/x still close
// ---------------------------------------------------------------------------

describe("trySeriesLimit — regression", () => {
  it("sin(x)/x → 1", () => {
    const expr = app(DIV, [app(SIN, [x()]), x()]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, int(1));
  });

  it("x^2/x → 0 by leading-order analysis", () => {
    const expr = app(DIV, [app(POW, [x(), int(2)]), x()]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, int(0));
  });

  it("limit(x^2, x, 0) — bare polynomial returns null (not a quotient)", () => {
    expect(trySeriesLimit(app(POW, [x(), int(2)]), x(), int(0))).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Fall-through — return null gracefully
// ---------------------------------------------------------------------------

describe("trySeriesLimit — fall-through", () => {
  it("returns null on a non-quotient", () => {
    expect(trySeriesLimit(app(POW, [x(), int(2)]), x(), int(0))).toBeNull();
  });

  it("returns null on an unsupported head", () => {
    const weird = app(sym("Asin"), [x()]);
    const expr = app(DIV, [weird, x()]);
    expect(trySeriesLimit(expr, x(), int(0))).toBeNull();
  });

  it("returns null for limits at +infinity (u = 1/x rewrite not implemented)", () => {
    const expr = app(DIV, [app(SIN, [x()]), x()]);
    expect(trySeriesLimit(expr, x(), sym("inf"))).toBeNull();
  });

  it("returns null for limits at -infinity", () => {
    const expr = app(DIV, [app(SIN, [x()]), x()]);
    expect(trySeriesLimit(expr, x(), sym("minf"))).toBeNull();
  });

  it("fall-through for limit(sin(x), x, %inf) via limitAdvanced", () => {
    // limitAdvanced still falls through to an unevaluated Limit(...)
    // for `limit(sin(x), x, inf)` — sin is bounded but not a quotient
    // that the Taylor fallback can resolve.
    const result = limitAdvanced(app(SIN, [x()]), x(), sym("inf"));
    expect(result.kind).toBe("apply");
    if (result.kind === "apply") {
      expect(result.head.kind === "symbol" && result.head.name === "Limit").toBe(true);
    }
  });
});

// ---------------------------------------------------------------------------
// Divergent quotients return ±∞ sentinels
// ---------------------------------------------------------------------------

describe("trySeriesLimit — divergent forms", () => {
  it("1/x^2 → +inf", () => {
    const expr = app(DIV, [int(1), app(POW, [x(), int(2)])]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    if (result && result.kind === "symbol") expect(result.name).toBe("inf");
    else throw new Error(`expected symbol("inf"), got ${display(result!)}`);
  });

  it("-1/x^2 → -inf", () => {
    const expr = app(DIV, [int(-1), app(POW, [x(), int(2)])]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    if (result && result.kind === "symbol") expect(result.name).toBe("minf");
    else throw new Error(`expected symbol("minf"), got ${display(result!)}`);
  });
});

// ---------------------------------------------------------------------------
// Non-origin expansion point
// ---------------------------------------------------------------------------

describe("trySeriesLimit — shifted expansion point", () => {
  it("(x^2 - 1)/(x - 1) at x = 1 → 2", () => {
    const numer = app(SUB, [app(POW, [x(), int(2)]), int(1)]);
    const denom = app(SUB, [x(), int(1)]);
    const expr = app(DIV, [numer, denom]);
    const result = trySeriesLimit(expr, x(), int(1));
    expect(result).not.toBeNull();
    eq(result!, int(2));
  });
});

// ---------------------------------------------------------------------------
// Expander edges — Neg, Pow(., -1), Mul-as-quotient, constants, max_order
// ---------------------------------------------------------------------------

describe("trySeriesLimit — expander edges", () => {
  it("-sin(x)/x → -1 (Neg head)", () => {
    const numer = app(NEG, [app(SIN, [x()])]);
    const expr = app(DIV, [numer, x()]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, int(-1));
  });

  it("(1 - x)^-1 / 1 → 1 via Pow(., -1)", () => {
    const oneMinusX = app(SUB, [int(1), x()]);
    const expr = app(DIV, [app(POW, [oneMinusX, int(-1)]), int(1)]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, int(1));
  });

  it("Mul(sin(x), Pow(x, -1)) recognised as a quotient", () => {
    const expr = app(MUL, [app(SIN, [x()]), app(POW, [x(), int(-1)])]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, int(1));
  });

  it("1/1 → 1 (rational-ring constant case)", () => {
    const expr = app(DIV, [int(1), int(1)]);
    const result = trySeriesLimit(expr, x(), int(0));
    expect(result).not.toBeNull();
    eq(result!, int(1));
  });

  it("maxOrder above the hard cap is clamped", () => {
    const expr = app(DIV, [app(SIN, [x()]), x()]);
    const result = trySeriesLimit(expr, x(), int(0), 999);
    expect(result).not.toBeNull();
    eq(result!, int(1));
  });

  it("maxOrder below 4 is bumped to 4", () => {
    const expr = app(DIV, [app(SIN, [x()]), x()]);
    const result = trySeriesLimit(expr, x(), int(0), 2);
    expect(result).not.toBeNull();
    eq(result!, int(1));
  });
});

// ---------------------------------------------------------------------------
// Dispatcher wiring — limitAdvanced picks up Taylor without diff_fn
// ---------------------------------------------------------------------------

describe("limitAdvanced + Taylor wiring", () => {
  it("closes sin(x)/x at 0 without a differentiate callback", () => {
    // Pre-J2 this returned an unevaluated Limit(...) when no diff_fn was
    // supplied. J2 wires the Taylor fallback in after L'Hopital so it
    // now resolves to 1.
    const result = limitAdvanced(app(DIV, [app(SIN, [x()]), x()]), x(), int(0));
    eq(result, int(1));
  });

  it("closes (1 - cos(x))/x^2 at 0 without a differentiate callback", () => {
    const expr = app(DIV, [app(SUB, [int(1), app(COS, [x()])]), app(POW, [x(), int(2)])]);
    const result = limitAdvanced(expr, x(), int(0));
    eq(result, rational(1, 2));
  });
});
