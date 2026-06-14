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
  LOG,
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

  it("negative a in the cosine arctan branch closes with the correct sign", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(ADD, [int(-2), app(COS, [X])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    expect(containsHead(phi, ATAN)).toBe(true);
    for (const xVal of [-1.5, -0.4, 0.0, 0.4, 1.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / (-2 + Math.cos(xVal));
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

  // (The non-bare-argument deferral previously asserted here has been
  // promoted to a Phase 38 success test in the dedicated Phase 38 block
  // below — Phase 38 closes those integrals via linear substitution.)

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

  it("∫ 1/cos(x) dx closes as the a = 0 cosine log branch", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(COS, [X])]);
    const result = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(result)).toBe(false);
    expect(containsHead(result, LOG)).toBe(true);
    for (const xVal of [-1.0, -0.4, 0.0, 0.4, 1.0]) {
      const got = numericalDerivative(vm, result, xVal);
      const expected = 1 / Math.cos(xVal);
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
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
  it("∫ 1/sin(x) dx closes as log|tan(x/2)|", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(SIN, [X])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    expect(containsHead(phi, LOG)).toBe(true);
    for (const xVal of [0.4, 0.8, 1.2, 1.6, 2.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1 / Math.sin(xVal);
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

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

  it("Phase 37: cos branch with b < −|a| (∫ 1/(1 − 2·cos x) dx) now closes", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(1), app(SUB, [int(1), app(MUL, [int(2), app(COS, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    // 1 − 2 cos x zeros at cos x = 1/2 (x = ±π/3 ≈ ±1.047).
    // Sample on (π/3, π) where the integrand is real and finite.
    for (const xVal of [1.2, 1.6, 2.0, 2.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (1.0 - 2.0 * Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });
});

describe("Phase 37: cos branch with b < −|a|", () => {
  it("∫ 1/(−1 − 3·cos x) dx — both a and b negative", () => {
    const vm = makeVM();
    const integrand = app(DIV, [
      int(1),
      app(SUB, [int(-1), app(MUL, [int(3), app(COS, [X])])]),
    ]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    // −1 − 3 cos x zeros at cos x = −1/3.  Sample safely.
    for (const xVal of [0.5, 1.0, 1.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (-1.0 - 3.0 * Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("∫ 5/(1 − 2·cos x) dx — numerator scales", () => {
    const vm = makeVM();
    const integrand = app(DIV, [int(5), app(SUB, [int(1), app(MUL, [int(2), app(COS, [X])])])]);
    const phi = vm.eval(integrate(integrand));
    for (const xVal of [1.2, 1.6, 2.0, 2.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 5.0 / (1.0 - 2.0 * Math.cos(xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });
});

// ---------------------------------------------------------------------------
// Phase 38: linear substitution lifts Weierstrass to ``trig(α·x + β)``.
//
// With ``u = α·x + β`` (``du = α·dx``), every Phase 34/35/36/37 closed form
// applies unchanged with ``tan((α·x+β)/2)`` in place of ``tan(x/2)`` and the
// outer coefficient scaled by ``1/α``.  Each case is verified by central
// differencing the closed form against the original integrand at sample
// points avoiding the integrand's singularities.
// ---------------------------------------------------------------------------

describe("Phase 38: linear-argument substitution lifts Weierstrass", () => {
  it("∫ 1/(2 + sin 2x) dx closes (promoted from Phase 34 deferral)", () => {
    const vm = makeVM();
    const twoX = app(MUL, [int(2), X]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [twoX])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    expect(containsHead(phi, ATAN)).toBe(true);
    for (const xVal of [-1.0, -0.3, 0.0, 0.3, 1.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (2.0 + Math.sin(2.0 * xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("scaled csc branch closes for ∫ 3/(2·sin(2x+1)) dx", () => {
    const vm = makeVM();
    const arg = app(ADD, [app(MUL, [int(2), X]), int(1)]);
    const denominator = app(MUL, [int(2), app(SIN, [arg])]);
    const phi = vm.eval(integrate(app(DIV, [int(3), denominator])));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    expect(containsHead(phi, LOG)).toBe(true);
    for (const xVal of [0.0, 0.2, 0.5, 0.8]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 3 / (2 * Math.sin(2 * xVal + 1));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("∫ 1/(2 + cos 3x) dx — α = 3 cos variant", () => {
    const vm = makeVM();
    const threeX = app(MUL, [int(3), X]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(COS, [threeX])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-0.5, -0.2, 0.0, 0.2, 0.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (2.0 + Math.cos(3.0 * xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("∫ 1/(2 + sin(x + 1)) dx — pure phase shift α = 1, β = 1", () => {
    const vm = makeVM();
    const xPlusOne = app(ADD, [X, int(1)]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [xPlusOne])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-2.0, -1.0, 0.0, 1.0, 2.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (2.0 + Math.sin(xVal + 1.0));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("∫ 1/(2 + sin(2x + 1)) dx — full α = 2, β = 1 case", () => {
    const vm = makeVM();
    const twoXPlusOne = app(ADD, [app(MUL, [int(2), X]), int(1)]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [twoXPlusOne])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-0.8, -0.3, 0.0, 0.3, 0.8]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (2.0 + Math.sin(2.0 * xVal + 1.0));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("∫ 1/(2 + sin(x/2)) dx — rational α = 1/2", () => {
    const vm = makeVM();
    const halfX = app(MUL, [rational(1, 2), X]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [halfX])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-2.0, -1.0, 0.5, 1.5]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (2.0 + Math.sin(0.5 * xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("∫ 1/(2 + sin(−2x)) dx — negative α = −2", () => {
    const vm = makeVM();
    const negTwoX = app(MUL, [int(-2), X]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [negTwoX])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    for (const xVal of [-1.0, -0.3, 0.0, 0.3, 1.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (2.0 + Math.sin(-2.0 * xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-4);
    }
  });

  it("∫ 1/(1 + cos 2x) dx — degenerate Phase 35 branch under α = 2", () => {
    const vm = makeVM();
    const twoX = app(MUL, [int(2), X]);
    const integrand = app(DIV, [int(1), app(ADD, [int(1), app(COS, [twoX])])]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    // 1 + cos 2x = 2 cos²(x); singularities at x = ±π/2 ≈ ±1.57.
    for (const xVal of [-1.0, -0.3, 0.3, 1.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (1.0 + Math.cos(2.0 * xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("∫ 1/(1 + 2·sin 2x) dx — log-form Phase 36 branch under α = 2", () => {
    const vm = makeVM();
    const twoX = app(MUL, [int(2), X]);
    const integrand = app(DIV, [
      int(1),
      app(ADD, [int(1), app(MUL, [int(2), app(SIN, [twoX])])]),
    ]);
    const phi = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(phi)).toBe(false);
    // 1 + 2·sin 2x zeros at sin 2x = −1/2, so 2x ∈ {−π/6, 7π/6,...}; staying
    // clear of x ≈ ±π/12 ≈ ±0.26 by sampling outside that window.
    for (const xVal of [-1.0, -0.5, 0.0, 0.5, 1.0]) {
      const got = numericalDerivative(vm, phi, xVal);
      const expected = 1.0 / (1.0 + 2.0 * Math.sin(2.0 * xVal));
      expect(Math.abs(got - expected)).toBeLessThan(1e-3);
    }
  });

  it("∫ 1/(2 + sin(x²)) dx falls through — argument is not linear in x", () => {
    const vm = makeVM();
    const xSq = app(MUL, [X, X]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [xSq])])]);
    const result = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(result)).toBe(true);
  });

  it("∫ 1/(2 + sin(α·x)) dx falls through — symbolic α", () => {
    const vm = makeVM();
    const alpha = sym("alpha");
    const alphaX = app(MUL, [alpha, X]);
    const integrand = app(DIV, [int(1), app(ADD, [int(2), app(SIN, [alphaX])])]);
    const result = vm.eval(integrate(integrand));
    expect(isUnevaluatedIntegrate(result)).toBe(true);
  });
});
