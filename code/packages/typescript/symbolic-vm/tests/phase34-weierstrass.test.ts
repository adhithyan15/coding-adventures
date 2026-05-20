// Phase 34: Weierstrass-substitution closed forms for
//   ∫ c / (a + b·sin(x)) dx   and   ∫ c / (a + b·cos(x)) dx
//
// Mirrors `tests/test_phase34_weierstrass.py` from the Python port.
// Closed forms are validated via *numerical differentiation*: substitute
// x ← x_val (IRFloat) into the closed form, evaluate via vm.eval, then
// central-difference around several sample values and require agreement
// with the original integrand to a tight tolerance.

import { describe, expect, it } from "vitest";
import {
  ADD,
  ATAN,
  COS,
  DIV,
  INTEGRATE,
  IRNode,
  MUL,
  NEG,
  SIN,
  SQRT,
  SUB,
  app,
  equals,
  int,
  numberNode,
  rational,
  sym,
} from "@coding-adventures/symbolic-ir";
import { SymbolicBackend, VM } from "../src/index.js";

const X = sym("x");

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

/** Evaluate `expr` after substituting x ← x_val, returning the numeric value. */
function evalAt(vm: VM, expr: IRNode, xVal: number): number {
  const substituted = subst(expr, X, numberNode(xVal));
  const folded = vm.eval(substituted);
  if (folded.kind === "float") return folded.value;
  if (folded.kind === "integer") return Number(folded.value);
  if (folded.kind === "rational") return Number(folded.numer) / Number(folded.denom);
  return Number.NaN;
}

/** Central-difference derivative of `expr` w.r.t. x at xVal. */
function numericalDerivative(vm: VM, expr: IRNode, xVal: number): number {
  const h = 1e-5;
  return (evalAt(vm, expr, xVal + h) - evalAt(vm, expr, xVal - h)) / (2 * h);
}

function containsHead(node: IRNode, head: IRNode): boolean {
  if (node.kind === "apply") {
    if (equals(node.head, head)) return true;
    return node.args.some((a) => containsHead(a, head));
  }
  return false;
}

function isUnevaluatedIntegrate(node: IRNode): boolean {
  return node.kind === "apply" && equals(node.head, INTEGRATE);
}

// ---------------------------------------------------------------------------
// ∫ 1/(a + b·sin(x)) dx — arctan form, a² > b²
// ---------------------------------------------------------------------------

describe("Phase 34: ∫ 1/(a + b·sin x) dx (Weierstrass arctan form)", () => {
  it("∫ 1/(2 + sin x) dx closes with an Atan in the body", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [X])])]);
    const result = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(result)).toBe(false);
    expect(containsHead(result, ATAN)).toBe(true);
  });

  it("numerical derivative of the ∫ 1/(2 + sin x) closed form matches at multiple points", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [X])])]);
    const phi = vm.eval(integrate(integrand));
    for (const xVal of [-2.5, -1.0, -0.3, 0.0, 0.3, 1.0, 2.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (2 + Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("perfect-square discriminant (a=5, b=3 → disc=16) folds to Sqrt-free output", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(5), app(MUL, [int(3), app(SIN, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    expect(containsHead(phi, SQRT)).toBe(false);
    for (const xVal of [-1.0, -0.2, 0.0, 0.7, 1.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (5 + 3 * Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("numerator coefficient scales the closed form (∫ 3/(2 + sin x))", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(3), app(ADD, [int(2), app(SIN, [X])])]);
    const phi = vm.eval(integrate(integrand));
    for (const xVal of [-1.0, -0.2, 0.0, 0.7, 1.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 3 / (2 + Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("rational coefficients a=3/2, b=1/2 (disc=2)", () => {
    const vm = makeVM();
    const integrand = app(DIV, [
      int(1),
      app(ADD, [rational(3, 2), app(MUL, [rational(1, 2), app(SIN, [X])])]),
    ]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-1.5, -0.4, 0.0, 0.4, 1.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (1.5 + 0.5 * Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });
});

// ---------------------------------------------------------------------------
// ∫ 1/(a + b·cos(x)) dx — arctan form, a² > b², a > 0
// ---------------------------------------------------------------------------

describe("Phase 34: ∫ 1/(a + b·cos x) dx (Weierstrass arctan form)", () => {
  it("∫ 1/(2 + cos x) dx closes; derivative matches", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(COS, [X])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-1.5, -0.4, 0.0, 0.4, 1.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (2 + Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("∫ 1/(5 + 3·cos x) dx: both disc and ratio are perfect squares → Sqrt-free", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(5), app(MUL, [int(3), app(COS, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    expect(containsHead(phi, SQRT)).toBe(false);
    for (const xVal of [-1.5, -0.4, 0.0, 0.4, 1.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (5 + 3 * Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });
});

// ---------------------------------------------------------------------------
// Operand-order robustness
// ---------------------------------------------------------------------------

describe("Phase 34: operand-order robustness", () => {
  it("∫ 1/(sin x + 2) dx — constant on the right — still closes", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [app(SIN, [X]), int(2)])]);
    const result = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(result)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Fallthroughs — must stay unevaluated
// ---------------------------------------------------------------------------

describe("Phase 34: deferred discriminant cases", () => {
  it("Phase 36: a² < b² (∫ 1/(1 + 2·sin x) dx) now closes via the log form", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(1), app(MUL, [int(2), app(SIN, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    // 1+2 sin x has poles at sin x = -1/2.  Sample x in (-π/4, π/4).
    for (const xVal of [-0.7, -0.2, 0.0, 0.2, 0.7]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (1.0 + 2.0 * Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("Phase 35: a² = b² (∫ 1/(1 + sin x) dx) now closes via the degenerate form", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(1), app(SIN, [X])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-2.0, -1.0, -0.3, 0.3, 1.0, 2.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (1 + Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("non-bare argument (∫ 1/(2 + sin 2x) dx) — Phase 34 doesn't compose with subs", () => {
    const vm = makeVM();
    const twoX = app(MUL, [int(2), X]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [twoX])])]);
    const result = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(result)).toBe(true);
  });

  it("symbolic coefficient (∫ 1/(a + sin x) dx) — discriminant sign undecidable", () => {
    const vm = makeVM();
    const aSym = sym("a");
    const integrand = app(DIV, [int(1), app(ADD, [aSym, app(SIN, [X])])]);
    const result = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(result)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Regressions — Phase 34 must not steal integrals it doesn't own
// ---------------------------------------------------------------------------

describe("Phase 34: regression — must not interfere with existing rules", () => {
  it("∫ sin(x) dx = −cos(x) unchanged", () => {
    const vm = makeVM();
    const result = vm.eval(integrate(app(SIN, [X])));
    const cosX = app(COS, [X]);
    const negCos = app(NEG, [cosX]);
    const mulNeg = app(MUL, [int(-1), cosX]);
    expect(equals(result, negCos) || equals(result, mulNeg)).toBe(true);
  });

  it("∫ 1/cos(x) dx is NOT misinterpreted as Weierstrass (no additive constant)", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(COS, [X])]);
    const result = vm.eval(integrate(integrand));
    // Either unevaluated, or some elementary fold — but specifically MUST NOT be
    // a top-level Atan (which would only come from Phase 34).
    if (result.kind === "apply" && equals(result.head, ATAN)) {
      throw new Error(`Phase 34 incorrectly fired on ∫ 1/cos(x) dx: got ${JSON.stringify(result)}`);
    }
  });
});

// ---------------------------------------------------------------------------
// Phase 35: degenerate a² = b² cases — all four sign combinations
// ---------------------------------------------------------------------------

describe("Phase 35: degenerate a² = b² cases", () => {
  it("∫ 1/(2 − 2·sin x) dx — sin, b = −a", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(SUB, [int(2), app(MUL, [int(2), app(SIN, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-2.0, -1.0, -0.3, 0.3, 1.0, 2.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (2 - 2 * Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("∫ 1/(1 + cos x) dx — cos, b = a → tan(x/2)", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(1), app(COS, [X])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-2.0, -1.0, -0.3, 0.3, 1.0, 2.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (1 + Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("∫ 1/(1 − cos x) dx — cos, b = −a → −cot(x/2)", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(SUB, [int(1), app(COS, [X])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    // Sample on (0, π) — avoid x = 0, 2π where 1 − cos x = 0.
    for (const xVal of [0.5, 1.0, 1.5, 2.0, 2.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (1 - Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("numerator coefficient (∫ 5/(2 + 2·sin x) dx) scales the closed form", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(5), app(ADD, [int(2), app(MUL, [int(2), app(SIN, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    for (const xVal of [-2.0, -1.0, -0.3, 0.3, 1.0, 2.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 5 / (2 + 2 * Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("rational coefficients (a = b = 3/2) close cleanly", () => {
    const vm = makeVM();
    const integrand = app(DIV, [
      int(1),
      app(ADD, [rational(3, 2), app(MUL, [rational(3, 2), app(COS, [X])])]),
    ]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-2.0, -1.0, -0.3, 0.3, 1.0, 2.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (1.5 + 1.5 * Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });
});

// ---------------------------------------------------------------------------
// Phase 36: log form for a² < b²
// ---------------------------------------------------------------------------

describe("Phase 36: log form for a² < b²", () => {
  it("∫ 1/(1 + 2·cos x) dx — cos branch with b > |a|", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(1), app(MUL, [int(2), app(COS, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    // 1+2 cos x zeros at x = ±2π/3.  Sample x in (-π/2, π/2).
    for (const xVal of [-1.2, -0.5, 0.0, 0.5, 1.2]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (1 + 2 * Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("∫ 1/(−1 + 2·sin x) dx — sin branch with a < 0 still works", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(-1), app(MUL, [int(2), app(SIN, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [1.0, 1.5, 2.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (-1 + 2 * Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("numerator coefficient (∫ 3/(1 + 2·sin x) dx) scales correctly", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(3), app(ADD, [int(1), app(MUL, [int(2), app(SIN, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    for (const xVal of [-0.7, -0.2, 0.0, 0.2, 0.7]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 3 / (1 + 2 * Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("perfect-square |disc| (∫ 1/(3 + 5·sin x) dx) folds Sqrt away", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(3), app(MUL, [int(5), app(SIN, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    expect(containsHead(phi, SQRT)).toBe(false);
    // 3+5 sin x zero at sin x = -3/5.  Sample safely.
    for (const xVal of [-0.3, 0.0, 0.3, 0.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (3 + 5 * Math.sin(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("cos branch with b < |a| still defers (1 − 2·cos x)", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(SUB, [int(1), app(MUL, [int(2), app(COS, [X])])])]);
    const result = vm.eval(integrate(integrand));
    // The b = -2 < |a| = 1 branch is deferred — log argument has the
    // opposite sign pattern.
    expect(isUnevaluatedIntegrate(result)).toBe(true);
  });
});
