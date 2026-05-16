import { describe, expect, it } from "vitest";
import {
  ADD,
  BESSEL_J,
  BESSEL_Y,
  CHEBYSHEV_T,
  CHEBYSHEV_U,
  COS,
  D,
  DIV,
  EQUAL,
  EXP,
  HERMITE_H,
  HERMITE_H2,
  INTEGRATE,
  LEGENDRE_P,
  LEGENDRE_Q,
  LOG,
  MUL,
  POW,
  SIN,
  SUB,
  app,
  equals,
  headName,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import { C, C1, C2, ODE2, buildOdeHandlerTable, ode2, solveOde, substRatioIr } from "../src/index";

const x = sym("x");
const y = sym("y");
const yp = app(D, [y, x]);
const ypp = app(D, [yp, x]);

function expectEqual(actual: IRNode, expected: IRNode): void {
  expect(equals(actual, expected), `${display(actual)} !== ${display(expected)}`).toBe(true);
}

function display(node: IRNode): string {
  return JSON.stringify(node, (_key, value) => (typeof value === "bigint" ? value.toString() : value));
}

function hasHead(node: IRNode, name: string): boolean {
  return node.kind === "apply" && (headName(node.head) === name || node.args.some((arg) => hasHead(arg, name)));
}

function rhsOfEqual(node: IRNode): IRNode {
  expect(node.kind).toBe("apply");
  expect(node.kind === "apply" && equals(node.head, EQUAL)).toBe(true);
  return (node as Extract<IRNode, { kind: "apply" }>).args[1];
}

function lhsOfEqual(node: IRNode): IRNode {
  expect(node.kind).toBe("apply");
  expect(node.kind === "apply" && equals(node.head, EQUAL)).toBe(true);
  return (node as Extract<IRNode, { kind: "apply" }>).args[0];
}

describe("cas-ode package shape", () => {
  it("exports an ODE2 handler table", () => {
    const table = buildOdeHandlerTable();
    expect(table.get("ODE2")).toBeTypeOf("function");
  });

  it("falls through as an unevaluated ODE2 node", () => {
    const unsupported = app(ADD, [yp, app(SIN, [app(MUL, [x, y])])]);
    expectEqual(ode2(unsupported, y, x), app(ODE2, [unsupported, y, x]));
  });
});

describe("first-order ODEs", () => {
  it("substitutes exact y/x ratios for the homogeneous recognizer", () => {
    const v = sym("v");
    const yOverX = app(DIV, [y, x]);
    expectEqual(substRatioIr(yOverX, y, x, v)!, v);
    expectEqual(substRatioIr(app(POW, [yOverX, int(2)]), y, x, v)!, app(POW, [v, int(2)]));
    expectEqual(substRatioIr(app(ADD, [yOverX, yOverX]), y, x, v)!, app(ADD, [v, v]));
  });

  it("rejects y when it is not structurally y/x", () => {
    const v = sym("v");
    expect(substRatioIr(y, y, x, v)).toBeNull();
    expect(substRatioIr(app(DIV, [app(ADD, [y, x]), x]), y, x, v)).toBeNull();
    expect(substRatioIr(app(MUL, [y, x]), y, x, v)).toBeNull();
    expectEqual(substRatioIr(x, y, x, v)!, x);
  });

  it("solves first-order linear equations with symbolic integrating factors", () => {
    const equation = app(SUB, [yp, app(MUL, [int(2), y])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    const rhs = rhsOfEqual(result!);
    expect(hasHead(rhs, EXP.name)).toBe(true);
    expect(hasHead(rhs, DIV.name)).toBe(true);
    expect(display(rhs)).toContain("%c");
  });

  it("returns implicit integrals for separable nonlinear equations", () => {
    const equation = app(SUB, [yp, app(MUL, [x, app(POW, [y, int(2)])])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    expect(result?.kind === "apply" && equals(result.head, EQUAL)).toBe(true);
    expect(display(result!)).toContain("%c");
  });

  it("solves Bernoulli equations through the linearized substitution", () => {
    const equation = app(ADD, [app(SUB, [yp, y]), app(POW, [y, int(2)])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    const rhs = rhsOfEqual(result!);
    expect(hasHead(rhs, POW.name)).toBe(true);
    expect(display(rhs)).toContain("%c");
  });

  it("solves exact equations as implicit potentials", () => {
    const M = app(ADD, [app(MUL, [int(2), x, y]), int(1)]);
    const N = app(POW, [x, int(2)]);
    const equation = app(ADD, [M, app(MUL, [N, yp])]);
    const result = solveOde(equation, y, x);
    const expectedPotential = app(ADD, [x, app(MUL, [y, app(POW, [x, int(2)])])]);
    expectEqual(result!, app(EQUAL, [expectedPotential, C]));
  });

  it("solves the homogeneous degenerate y prime equals y over x case", () => {
    const equation = app(SUB, [yp, app(DIV, [y, x])]);
    const result = solveOde(equation, y, x);
    expectEqual(result!, app(EQUAL, [y, app(MUL, [C, x])]));
  });

  it("returns an implicit homogeneous solution when the v integral is symbolic", () => {
    const yOverX = app(DIV, [y, x]);
    const equation = app(SUB, [yp, app(EXP, [yOverX])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    expect(hasHead(lhsOfEqual(result!), INTEGRATE.name)).toBe(true);
    expect(hasHead(rhsOfEqual(result!), LOG.name)).toBe(true);
    expect(display(result!)).toContain("%c");
    expect(display(result!)).not.toContain("_hom_v");
  });

  it("uses primitive homogeneous integration when the v integral is available", () => {
    const yOverX = app(DIV, [y, x]);
    const equation = app(SUB, [yp, app(MUL, [int(2), yOverX])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    expect(hasHead(lhsOfEqual(result!), LOG.name)).toBe(true);
    expect(hasHead(rhsOfEqual(result!), LOG.name)).toBe(true);
  });

  it("falls through when y also appears outside y over x", () => {
    const yOverX = app(DIV, [y, x]);
    const unsupported = app(SUB, [yp, app(ADD, [app(MUL, [y, x]), yOverX])]);
    expectEqual(ode2(unsupported, y, x), app(ODE2, [unsupported, y, x]));
  });
});

describe("second-order ODEs", () => {
  it("solves constant-coefficient homogeneous equations", () => {
    const result = solveOde(app(ADD, [ypp, y]), y, x);
    const expected = app(EQUAL, [
      y,
      app(ADD, [
        app(MUL, [C1, app(COS, [x])]),
        app(MUL, [C2, app(SIN, [x])]),
      ]),
    ]);
    expectEqual(result!, expected);
  });

  it("solves polynomial-forced constant-coefficient equations", () => {
    const equation = app(SUB, [app(ADD, [ypp, y]), x]);
    const result = solveOde(equation, y, x);
    const rhs = rhsOfEqual(result!);
    expect(display(rhs)).toContain("%c1");
    expect(display(rhs)).toContain("%c2");
    expect(display(rhs)).toContain("\"name\":\"x\"");
  });

  it("uses variation of parameters with symbolic Integrate fallback", () => {
    const equation = app(SUB, [app(ADD, [ypp, y]), app(LOG, [x])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    expect(hasHead(result!, INTEGRATE.name)).toBe(true);
  });

  it("solves Euler-Cauchy equations", () => {
    const equation = app(SUB, [app(MUL, [app(POW, [x, int(2)]), ypp]), app(MUL, [int(2), y])]);
    const result = solveOde(equation, y, x);
    const rhs = rhsOfEqual(result!);
    expect(display(rhs)).toContain("%c1");
    expect(display(rhs)).toContain("%c2");
    expect(hasHead(rhs, POW.name)).toBe(true);
  });
});

// ============================================================================
// Phase 21 — Variable-coefficient named ODE recognition
// ============================================================================

/**
 * Build the Legendre ODE expression (zero form) for order n:
 *   (1 − x²)·y'' − 2x·y' + n(n+1)·y = 0
 */
function legendreOdeExpr(n: number): IRNode {
  const xSq = app(POW, [x, int(2)]);
  const oneMinusXSq = app(SUB, [int(1), xSq]);
  const lambda = n * (n + 1);
  return app(ADD, [
    app(MUL, [oneMinusXSq, ypp]),
    app(ADD, [
      app(MUL, [int(-2), app(MUL, [x, yp])]),
      app(MUL, [int(lambda), y]),
    ]),
  ]);
}

/**
 * Build the Bessel ODE expression (zero form) for order ν given as an IR node:
 *   x²·y'' + x·y' + (x² − ν²)·y = 0
 */
function besselOdeExpr(nuIr: IRNode, nuSqIr: IRNode): IRNode {
  const xSq = app(POW, [x, int(2)]);
  return app(ADD, [
    app(MUL, [xSq, ypp]),
    app(ADD, [
      app(MUL, [x, yp]),
      app(MUL, [app(SUB, [xSq, nuSqIr]), y]),
    ]),
  ]);
}

/**
 * Build the Hermite ODE expression (zero form) for order n:
 *   y'' − 2x·y' + 2n·y = 0
 */
function hermiteOdeExpr(n: number): IRNode {
  return app(ADD, [
    ypp,
    app(ADD, [
      app(MUL, [int(-2), app(MUL, [x, yp])]),
      app(MUL, [int(2 * n), y]),
    ]),
  ]);
}

/**
 * Build the Chebyshev ODE expression (zero form) for order n:
 *   (1 − x²)·y'' − x·y' + n²·y = 0
 */
function chebyshevOdeExpr(n: number): IRNode {
  const xSq = app(POW, [x, int(2)]);
  const oneMinusXSq = app(SUB, [int(1), xSq]);
  return app(ADD, [
    app(MUL, [oneMinusXSq, ypp]),
    app(ADD, [
      app(MUL, [int(-1), app(MUL, [x, yp])]),
      app(MUL, [int(n * n), y]),
    ]),
  ]);
}

describe("Phase 21 — named variable-coefficient ODEs", () => {
  it("recognises Legendre ODE n=2 and returns LegendreP/Q solution", () => {
    const result = solveOde(legendreOdeExpr(2), y, x);
    expect(result).not.toBeNull();
    const rhs = rhsOfEqual(result!);
    // Solution: %c1·LegendreP(2,x) + %c2·LegendreQ(2,x)
    const expected = app(ADD, [
      app(MUL, [C1, app(LEGENDRE_P, [int(2), x])]),
      app(MUL, [C2, app(LEGENDRE_Q, [int(2), x])]),
    ]);
    expectEqual(rhs, expected);
  });

  it("recognises Legendre ODE n=3 and encodes n=3 in the solution", () => {
    const result = solveOde(legendreOdeExpr(3), y, x);
    expect(result).not.toBeNull();
    expect(display(result!)).toContain("LegendreP");
    expect(display(result!)).toContain("LegendreQ");
    // The integer 3 must appear as the order parameter
    expect(display(result!)).toContain('"3"');
  });

  it("recognises Bessel ODE ν=1 (integer order) and returns BesselJ/Y solution", () => {
    // x²y'' + xy' + (x²−1)y = 0
    const result = solveOde(besselOdeExpr(int(1), int(1)), y, x);
    expect(result).not.toBeNull();
    const rhs = rhsOfEqual(result!);
    const expected = app(ADD, [
      app(MUL, [C1, app(BESSEL_J, [int(1), x])]),
      app(MUL, [C2, app(BESSEL_Y, [int(1), x])]),
    ]);
    expectEqual(rhs, expected);
  });

  it("recognises Bessel ODE ν=2 (integer order)", () => {
    // x²y'' + xy' + (x²−4)y = 0
    const result = solveOde(besselOdeExpr(int(2), int(4)), y, x);
    expect(result).not.toBeNull();
    expect(display(result!)).toContain("BesselJ");
    expect(display(result!)).toContain("BesselY");
    expect(display(result!)).toContain('"2"');
  });

  it("recognises Bessel ODE ν=1/2 (half-integer order)", () => {
    // x²y'' + xy' + (x²−1/4)y = 0  →  ν = 1/2
    const result = solveOde(besselOdeExpr(rational(1, 2), rational(1, 4)), y, x);
    expect(result).not.toBeNull();
    expect(display(result!)).toContain("BesselJ");
    expect(display(result!)).toContain("BesselY");
  });

  it("recognises Hermite ODE n=3 and returns HermiteH/H2 solution", () => {
    const result = solveOde(hermiteOdeExpr(3), y, x);
    expect(result).not.toBeNull();
    const rhs = rhsOfEqual(result!);
    const expected = app(ADD, [
      app(MUL, [C1, app(HERMITE_H, [int(3), x])]),
      app(MUL, [C2, app(HERMITE_H2, [int(3), x])]),
    ]);
    expectEqual(rhs, expected);
  });

  it("recognises Hermite ODE n=0 (trivial: y'' = 0)", () => {
    // y'' + 0·y' + 0·y = 0 should match Hermite with n=0
    const result = solveOde(hermiteOdeExpr(0), y, x);
    expect(result).not.toBeNull();
    expect(display(result!)).toContain("HermiteH");
  });

  it("recognises Chebyshev ODE n=2 and returns ChebyshevT/U solution", () => {
    const result = solveOde(chebyshevOdeExpr(2), y, x);
    expect(result).not.toBeNull();
    const rhs = rhsOfEqual(result!);
    const expected = app(ADD, [
      app(MUL, [C1, app(CHEBYSHEV_T, [int(2), x])]),
      app(MUL, [C2, app(CHEBYSHEV_U, [int(2), x])]),
    ]);
    expectEqual(rhs, expected);
  });

  it("recognises Chebyshev ODE n=3", () => {
    const result = solveOde(chebyshevOdeExpr(3), y, x);
    expect(result).not.toBeNull();
    expect(display(result!)).toContain("ChebyshevT");
    expect(display(result!)).toContain("ChebyshevU");
    expect(display(result!)).toContain('"3"');
  });

  it("distinguishes Chebyshev (Q≈-x) from Legendre (Q≈-2x)", () => {
    const legendreResult = solveOde(legendreOdeExpr(2), y, x);
    const chebyshevResult = solveOde(chebyshevOdeExpr(2), y, x);
    // Legendre should NOT produce Chebyshev output and vice versa
    expect(display(legendreResult!)).toContain("LegendreP");
    expect(display(legendreResult!)).not.toContain("ChebyshevT");
    expect(display(chebyshevResult!)).toContain("ChebyshevT");
    expect(display(chebyshevResult!)).not.toContain("LegendreP");
  });

  it("does not misidentify generic 2nd-order ODEs as named families", () => {
    // y'' + x³·y' + y = 0 — P=1 constant, Q=x³ not ±x or ±2x, R=1
    // (would match Hermite P check since P=1, but Q check fails since x³ ≠ -2x)
    const xCubed = app(POW, [x, int(3)]);
    const expr = app(ADD, [ypp, app(ADD, [app(MUL, [xCubed, yp]), y])]);
    const result = solveOde(expr, y, x);
    if (result !== null) {
      // If any solver catches it, it must not be a named-ODE family result
      expect(display(result)).not.toContain("LegendreP");
      expect(display(result)).not.toContain("BesselJ");
      expect(display(result)).not.toContain("HermiteH");
      expect(display(result)).not.toContain("ChebyshevT");
    }
  });

  it("regression: Euler-Cauchy still works after Phase 21 dispatch", () => {
    // x²y'' − 2y = 0 should still be caught by tryEulerCauchy BEFORE tryVarCoeffNamedOde
    const equation = app(SUB, [app(MUL, [app(POW, [x, int(2)]), ypp]), app(MUL, [int(2), y])]);
    const result = solveOde(equation, y, x);
    expect(result).not.toBeNull();
    // Euler-Cauchy with roots r² + (0-1)r - 2 = 0: r²-r-2=0 → (r-2)(r+1)=0 → r=2,-1
    expect(display(result!)).toContain("Pow");
    expect(display(result!)).not.toContain("LegendreP");
    expect(display(result!)).not.toContain("BesselJ");
  });
});
