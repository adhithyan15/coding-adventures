/**
 * Tests for Track G2 (TypeScript port): symbolic-coefficient
 * Weierstrass lift.  Mirrors
 * ``python/symbolic-vm/tests/test_weierstrass_symbolic_coefficients.py``
 * shipped with Python G1 / PR #5361.
 *
 * The numeric Phase-34 Weierstrass helper only fires when ``a`` and
 * ``b`` in ``∫ c / (a + b·sin(α·x+β)) dx`` are concrete rationals.
 * Track G2 generalises it: when the user has declared the sign of the
 * discriminant ``a² − b²`` via ``Assume(...)``, the integrator emits
 * the corresponding closed form with symbolic ``a, b``.
 *
 * The branch selection is driven by ``vm.assumptions`` lookups
 * against the compound-relation store added in the cas-simplify Track
 * G2 first half.  These tests cover all four branches
 * (``> 0``, ``< 0``, ``= 0``, no assumption → unevaluated) plus the
 * linear-argument lifting that must still compose.
 *
 * Structural assertions rather than numeric ones — the result is a
 * tree in symbolic ``a, b`` that no numeric evaluation can collapse
 * cheaply.  We assert the kind of the outer head (``Atan``, ``Log``,
 * ``Integrate``) and that the recorded discriminant radicand appears
 * literally somewhere in the tree.
 */

import { describe, expect, it } from "vitest";
import {
  ADD,
  ATAN,
  COS,
  DIV,
  EQUAL,
  GREATER,
  INTEGRATE,
  IRNode,
  LESS,
  LOG,
  MUL,
  POW,
  SIN,
  SQRT,
  SUB,
  app,
  equals,
  int,
  sym,
} from "@coding-adventures/symbolic-ir";
import { SymbolicBackend, VM } from "../src/index.js";

const X = sym("x");
const A = sym("a");
const B = sym("b");
const TWO = int(2);

function makeVM(): VM {
  return new VM(new SymbolicBackend());
}

function assume(vm: VM, rel: IRNode): void {
  vm.eval(app(sym("Assume"), [rel]));
}

const gt = (lhs: IRNode, rhs: IRNode): IRNode => app(GREATER, [lhs, rhs]);
const lt = (lhs: IRNode, rhs: IRNode): IRNode => app(LESS, [lhs, rhs]);
const eq = (lhs: IRNode, rhs: IRNode): IRNode => app(EQUAL, [lhs, rhs]);
const sq = (node: IRNode): IRNode => app(POW, [node, TWO]);

function integrate(f: IRNode): IRNode {
  return app(INTEGRATE, [f, X]);
}

function containsHead(node: IRNode, head: IRNode): boolean {
  if (node.kind !== "apply") return false;
  if (equals(node.head, head)) return true;
  return node.args.some((a) => containsHead(a, head));
}

function containsSubtree(node: IRNode, target: IRNode): boolean {
  if (equals(node, target)) return true;
  if (node.kind !== "apply") return false;
  return node.args.some((a) => containsSubtree(a, target));
}

describe("symbolic-coefficient Weierstrass — disc > 0 arctan branch", () => {
  it("∫ 1/(a + b·sin(x)) with assume(a² > b²) returns arctan form with Sqrt(a² − b²)", () => {
    const vm = makeVM();
    assume(vm, gt(sq(A), sq(B)));
    const denom = app(ADD, [A, app(MUL, [B, app(SIN, [X])])]);
    const result = vm.eval(integrate(app(DIV, [int(1), denom])));
    expect(result.kind === "apply" && equals(result.head, INTEGRATE)).toBe(false);
    expect(containsHead(result, ATAN)).toBe(true);
    const expectedSqrt = app(SQRT, [app(SUB, [sq(A), sq(B)])]);
    expect(containsSubtree(result, expectedSqrt)).toBe(true);
  });

  it("∫ 1/(a + b·cos(x)) with assume(a² > b²) returns arctan form with Sqrt(a² − b²)", () => {
    const vm = makeVM();
    assume(vm, gt(sq(A), sq(B)));
    const denom = app(ADD, [A, app(MUL, [B, app(COS, [X])])]);
    const result = vm.eval(integrate(app(DIV, [int(1), denom])));
    expect(result.kind === "apply" && equals(result.head, INTEGRATE)).toBe(false);
    expect(containsHead(result, ATAN)).toBe(true);
    const expectedSqrt = app(SQRT, [app(SUB, [sq(A), sq(B)])]);
    expect(containsSubtree(result, expectedSqrt)).toBe(true);
  });
});

describe("symbolic-coefficient Weierstrass — disc < 0 log branch", () => {
  it("∫ 1/(a + b·sin(x)) with assume(a² < b²) returns log form with Sqrt(b² − a²)", () => {
    const vm = makeVM();
    assume(vm, lt(sq(A), sq(B)));
    const denom = app(ADD, [A, app(MUL, [B, app(SIN, [X])])]);
    const result = vm.eval(integrate(app(DIV, [int(1), denom])));
    expect(result.kind === "apply" && equals(result.head, INTEGRATE)).toBe(false);
    expect(containsHead(result, LOG)).toBe(true);
    const expectedSqrt = app(SQRT, [app(SUB, [sq(B), sq(A)])]);
    expect(containsSubtree(result, expectedSqrt)).toBe(true);
  });
});

describe("symbolic-coefficient Weierstrass — disc = 0 degenerate branch", () => {
  it("∫ 1/(a + b·sin(x)) with assume(a² = b²) returns rational-in-tan(x/2), no outer Atan or Log", () => {
    const vm = makeVM();
    assume(vm, eq(sq(A), sq(B)));
    const denom = app(ADD, [A, app(MUL, [B, app(SIN, [X])])]);
    const result = vm.eval(integrate(app(DIV, [int(1), denom])));
    expect(result.kind === "apply" && equals(result.head, INTEGRATE)).toBe(false);
    expect(containsHead(result, ATAN)).toBe(false);
    expect(containsHead(result, LOG)).toBe(false);
  });
});

describe("symbolic-coefficient Weierstrass — no assumption", () => {
  it("Without an Assume(...), the integral is left unevaluated", () => {
    const vm = makeVM();
    const denom = app(ADD, [A, app(MUL, [B, app(SIN, [X])])]);
    const result = vm.eval(integrate(app(DIV, [int(1), denom])));
    expect(result.kind === "apply" && equals(result.head, INTEGRATE)).toBe(true);
  });
});

describe("symbolic-coefficient Weierstrass — linear-argument lifting", () => {
  it("∫ 1/(a + b·sin(2x + 1)) with assume(a² > b²) lifts to arctan form with same radicand", () => {
    const vm = makeVM();
    assume(vm, gt(sq(A), sq(B)));
    const inner = app(ADD, [app(MUL, [TWO, X]), int(1)]);
    const denom = app(ADD, [A, app(MUL, [B, app(SIN, [inner])])]);
    const result = vm.eval(integrate(app(DIV, [int(1), denom])));
    expect(result.kind === "apply" && equals(result.head, INTEGRATE)).toBe(false);
    expect(containsHead(result, ATAN)).toBe(true);
    const expectedSqrt = app(SQRT, [app(SUB, [sq(A), sq(B)])]);
    expect(containsSubtree(result, expectedSqrt)).toBe(true);
  });
});

describe("symbolic-coefficient Weierstrass — numeric regression", () => {
  it("∫ 1/(2 + sin(x)) still closes to the numeric arctan form", () => {
    const vm = makeVM();
    const denom = app(ADD, [int(2), app(SIN, [X])]);
    const result = vm.eval(integrate(app(DIV, [int(1), denom])));
    expect(result.kind === "apply" && equals(result.head, INTEGRATE)).toBe(false);
    expect(containsHead(result, ATAN)).toBe(true);
  });

  it("∫ 1/(1 + 2·sin(x)) still closes to the numeric log form", () => {
    const vm = makeVM();
    const denom = app(ADD, [int(1), app(MUL, [int(2), app(SIN, [X])])]);
    const result = vm.eval(integrate(app(DIV, [int(1), denom])));
    expect(result.kind === "apply" && equals(result.head, INTEGRATE)).toBe(false);
    expect(containsHead(result, LOG)).toBe(true);
  });
});
