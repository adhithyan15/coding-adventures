// Track E2 — Generic tabular integration-by-parts fallback (TypeScript port).
//
// Mirrors `tests/test_ibp.py` from the Python reference (Track E1).  All
// correctness checks use **numeric differentiation of the returned
// antiderivative** against the original integrand.  This avoids
// hard-coding the exact algebraic form of the answer — any equivalent
// shape (Sub(Sin(x), Mul(x, Cos(x))) vs. Add(Sin(x), Neg(Mul(x, Cos(x))))
// vs. anything the simplifier picks tomorrow) is accepted as long as the
// symbolic antiderivative evaluates to the correct numeric value.

import { describe, expect, it } from "vitest";
import {
  COS,
  DIV,
  EXP,
  INTEGRATE,
  IRNode,
  LOG,
  MUL,
  POW,
  SIN,
  app,
  equals,
  int,
  numberNode,
  sym,
} from "@coding-adventures/symbolic-ir";
import { SymbolicBackend, VM } from "../src/index.js";

const X = sym("x");
const PI = sym("%pi");

function integrate(f: IRNode): IRNode {
  return app(INTEGRATE, [f, X]);
}

function makeVM(): VM {
  return new VM(new SymbolicBackend());
}

function subst(node: IRNode, varNode: IRNode, value: IRNode): IRNode {
  if (equals(node, varNode)) return value;
  if (node.kind === "apply") {
    return app(node.head, node.args.map((a) => subst(a, varNode, value)));
  }
  return node;
}

function evalAt(vm: VM, expr: IRNode, xVal: number): number {
  const substituted = subst(expr, X, numberNode(xVal));
  const folded = vm.eval(substituted);
  if (folded.kind === "float") return folded.value;
  if (folded.kind === "integer") return Number(folded.value);
  if (folded.kind === "rational") return Number(folded.numer) / Number(folded.denom);
  return Number.NaN;
}

function containsHead(node: IRNode, head: IRNode): boolean {
  if (node.kind === "apply") {
    if (equals(node.head, head)) return true;
    return node.args.some((a) => containsHead(a, head));
  }
  return false;
}

function trapezoidal(fn: (x: number) => number, a: number, b: number, n = 50_000): number {
  const h = (b - a) / n;
  let total = 0.5 * (fn(a) + fn(b));
  for (let i = 1; i < n; i += 1) total += fn(a + i * h);
  return total * h;
}

describe("Track E2 — generic tabular IBP fallback", () => {
  // -------------------------------------------------------------------------
  // Acceptance #1 — ∫ x·sin(x) dx = sin(x) − x·cos(x).
  // -------------------------------------------------------------------------
  it("closes ∫ x·sin(x) dx via tabular IBP", () => {
    const vm = makeVM();
    const integrand = app(MUL, [X, app(SIN, [X])]);
    const out = vm.eval(integrate(integrand));
    expect(containsHead(out, INTEGRATE)).toBe(false);
    // F(1) − F(0) should equal sin(1) − cos(1).
    const diff = evalAt(vm, out, 1.0) - evalAt(vm, out, 0.0);
    expect(diff).toBeCloseTo(Math.sin(1.0) - Math.cos(1.0), 9);
  });

  // -------------------------------------------------------------------------
  // Acceptance #2 — ∫ x²·eˣ dx = (x² − 2x + 2)·eˣ.
  // -------------------------------------------------------------------------
  it("closes ∫ x²·eˣ dx via tabular IBP", () => {
    const vm = makeVM();
    const integrand = app(MUL, [app(POW, [X, int(2)]), app(EXP, [X])]);
    const out = vm.eval(integrate(integrand));
    expect(containsHead(out, INTEGRATE)).toBe(false);
    // F(2) − F(0) = (4 − 4 + 2)·e² − (0 − 0 + 2)·e⁰ = 2e² − 2.
    const diff = evalAt(vm, out, 2.0) - evalAt(vm, out, 0.0);
    expect(diff).toBeCloseTo(2.0 * Math.exp(2.0) - 2.0, 9);
  });

  // -------------------------------------------------------------------------
  // Acceptance #3 — higher-degree polynomial × trig: ∫ x³·cos(x) dx.
  // -------------------------------------------------------------------------
  it("closes ∫ x³·cos(x) dx via tabular IBP", () => {
    const vm = makeVM();
    const integrand = app(MUL, [app(POW, [X, int(3)]), app(COS, [X])]);
    const out = vm.eval(integrate(integrand));
    expect(containsHead(out, INTEGRATE)).toBe(false);
    const diff = evalAt(vm, out, 1.0) - evalAt(vm, out, 0.0);
    const numeric = trapezoidal((xv) => xv * xv * xv * Math.cos(xv), 0.0, 1.0);
    expect(diff).toBeCloseTo(numeric, 5);
  });

  // -------------------------------------------------------------------------
  // Acceptance #4 — fallthrough: ∫ 1/x dx returns log(x); IBP does not fire
  // (integrand isn't a Mul — head is DIV).
  // -------------------------------------------------------------------------
  it("falls through to the log handler for ∫ 1/x dx (IBP returns undefined)", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), X]);
    const out = vm.eval(integrate(integrand));
    expect(containsHead(out, INTEGRATE)).toBe(false);
    expect(containsHead(out, LOG)).toBe(true);
    // F(2) − F(1) = log 2.
    const diff = evalAt(vm, out, 2.0) - evalAt(vm, out, 1.0);
    expect(diff).toBeCloseTo(Math.log(2.0), 12);
  });

  // -------------------------------------------------------------------------
  // Acceptance #5 — Phase 23 Fresnel fallback: ∫ sin(x²) dx closes to FresnelS.
  // IBP can't help — the integrand isn't a Mul — so this must come from the
  // shape-specific special-function recognizer.
  // -------------------------------------------------------------------------
  it("closes ∫ sin(x²) dx to FresnelS", () => {
    const vm = makeVM();
    const integrand = app(SIN, [app(POW, [X, int(2)])]);
    const out = vm.eval(integrate(integrand));
    expect(containsHead(out, INTEGRATE)).toBe(false);
    expect(containsHead(out, sym("FresnelS"))).toBe(true);
  });

  // -------------------------------------------------------------------------
  // Regression #6 — ∫ cos(x²) dx closes to FresnelC rather than falling
  // through to the generic unevaluated Integrate form.
  // -------------------------------------------------------------------------
  it("closes ∫ cos(x²) dx to FresnelC", () => {
    const vm = makeVM();
    const integrand = app(COS, [app(POW, [X, int(2)])]);
    const out = vm.eval(integrate(integrand));
    expect(containsHead(out, INTEGRATE)).toBe(false);
    expect(containsHead(out, sym("FresnelC"))).toBe(true);
  });

  it("uses the canonical FresnelS(x) form for ∫ sin(%pi·x²/2) dx", () => {
    const vm = makeVM();
    const integrand = app(SIN, [app(DIV, [app(MUL, [PI, app(POW, [X, int(2)])]), int(2)])]);
    const out = vm.eval(integrate(integrand));
    expect(containsHead(out, INTEGRATE)).toBe(false);
    expect(out).toEqual(app(sym("FresnelS"), [X]));
  });
});
