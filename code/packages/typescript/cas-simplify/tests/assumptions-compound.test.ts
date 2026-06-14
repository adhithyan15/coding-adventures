/**
 * Tests for compound-relation support in {@link AssumptionContext}
 * (Track G2 TypeScript port of Python G1, macsyma-truly-finish-plan).
 *
 * Until Track G2 the assumption store only understood
 * plain-symbol-vs-zero relations (``assume(x > 0)``).  Compound
 * relations such as ``assume(a^2 > b^2)`` were silently dropped, which
 * prevented the symbolic-coefficient Weierstrass integrator from
 * learning the discriminant sign at integration time.
 *
 * These tests exercise the new compound-relation path end-to-end at
 * the {@link AssumptionContext} API level — they neither require nor
 * exercise the VM.  The integrator-side behaviour is tested in
 * ``symbolic-vm/tests/weierstrass-symbolic-coefficients.test.ts``.
 */

import { describe, expect, it } from "vitest";
import {
  EQUAL,
  GREATER,
  GREATER_EQUAL,
  IRNode,
  LESS,
  LESS_EQUAL,
  NOT_EQUAL,
  POW,
  app,
  int,
  sym,
} from "@coding-adventures/symbolic-ir";
import { AssumptionContext } from "../src/assumptions.js";

// ``a^2`` / ``b^2`` as IR.  Built once at module scope because IR nodes
// are structural — the same node identity flows through every test.
const A = sym("a");
const B = sym("b");
const TWO = int(2);
const A_SQ = app(POW, [A, TWO]);
const B_SQ = app(POW, [B, TWO]);

const gt = (lhs: IRNode, rhs: IRNode): IRNode => app(GREATER, [lhs, rhs]);
const lt = (lhs: IRNode, rhs: IRNode): IRNode => app(LESS, [lhs, rhs]);
const ge = (lhs: IRNode, rhs: IRNode): IRNode => app(GREATER_EQUAL, [lhs, rhs]);
const le = (lhs: IRNode, rhs: IRNode): IRNode => app(LESS_EQUAL, [lhs, rhs]);
const eq = (lhs: IRNode, rhs: IRNode): IRNode => app(EQUAL, [lhs, rhs]);
const ne = (lhs: IRNode, rhs: IRNode): IRNode => app(NOT_EQUAL, [lhs, rhs]);

describe("AssumptionContext compound relations — direct lookups", () => {
  it("assume(a^2 > b^2) then is(a^2 > b^2) returns true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(gt(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(gt(A_SQ, B_SQ))).toBe(true);
  });

  it("assume(a^2 = b^2) then is(a^2 = b^2) returns true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(eq(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(eq(A_SQ, B_SQ))).toBe(true);
  });

  it("assume(a^2 < b^2) then is(a^2 < b^2) returns true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(lt(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(lt(A_SQ, B_SQ))).toBe(true);
  });

  it("assume(a^2 >= b^2) then is(a^2 >= b^2) returns true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(ge(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(ge(A_SQ, B_SQ))).toBe(true);
  });

  it("assume(a^2 != b^2) then is(a^2 != b^2) returns true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(ne(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(ne(A_SQ, B_SQ))).toBe(true);
  });
});

describe("AssumptionContext compound relations — commutative rewrites", () => {
  it("assume(a^2 > b^2) implies is(b^2 < a^2) is true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(gt(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(lt(B_SQ, A_SQ))).toBe(true);
  });

  it("assume(a^2 < b^2) implies is(b^2 > a^2) is true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(lt(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(gt(B_SQ, A_SQ))).toBe(true);
  });

  it("assume(a^2 >= b^2) implies is(b^2 <= a^2) is true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(ge(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(le(B_SQ, A_SQ))).toBe(true);
  });

  it("assume(a^2 = b^2) implies is(b^2 = a^2) is true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(eq(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(eq(B_SQ, A_SQ))).toBe(true);
  });

  it("assume(a^2 != b^2) implies is(b^2 != a^2) is true", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(ne(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(ne(B_SQ, A_SQ))).toBe(true);
  });
});

describe("AssumptionContext compound relations — unknown / no-negative inference", () => {
  it("no assertion → query returns undefined (not false)", () => {
    const ctx = new AssumptionContext();
    const c = sym("c");
    const d = sym("d");
    const cSq = app(POW, [c, TWO]);
    const dSq = app(POW, [d, TWO]);
    expect(ctx.isTrueRelation(gt(cSq, dSq))).toBeUndefined();
  });

  it("assume(a^2 > b^2) does NOT imply is(a^2 < b^2) is false", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(gt(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(lt(A_SQ, B_SQ))).toBeUndefined();
  });
});

describe("AssumptionContext compound relations — forget path", () => {
  it("forgetRelation removes the stored compound fact", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(gt(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(gt(A_SQ, B_SQ))).toBe(true);
    ctx.forgetRelation(gt(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(gt(A_SQ, B_SQ))).toBeUndefined();
  });

  it("forgetAll clears both plain-symbol facts AND compound relations", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(gt(A_SQ, B_SQ));
    const x = sym("x");
    ctx.assumeRelation(gt(x, int(0)));
    ctx.forgetAll();
    expect(ctx.isTrueRelation(gt(A_SQ, B_SQ))).toBeUndefined();
    expect(ctx.isPositive("x")).toBeUndefined();
  });

  it("re-asserting the same compound fact (or its commuted form) dedupes", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(gt(A_SQ, B_SQ));
    ctx.assumeRelation(gt(A_SQ, B_SQ));
    // Re-assert via the commuted form — canonicalisation should fold.
    ctx.assumeRelation(lt(B_SQ, A_SQ));
    // A single forget should zero out the fact regardless of which
    // surface form we used.
    ctx.forgetRelation(gt(A_SQ, B_SQ));
    expect(ctx.isTrueRelation(gt(A_SQ, B_SQ))).toBeUndefined();
  });
});

describe("AssumptionContext compound relations — plain-symbol path unchanged", () => {
  it("assume(x > 0) still threads through the per-symbol fact table", () => {
    const ctx = new AssumptionContext();
    const x = sym("x");
    ctx.assumeRelation(gt(x, int(0)));
    expect(ctx.isPositive("x")).toBe(true);
    expect(ctx.isTrueRelation(gt(x, int(0)))).toBe(true);
    expect(ctx.isTrueRelation(lt(x, int(0)))).toBe(false);
  });
});
