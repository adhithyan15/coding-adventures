import { describe, expect, it } from "vitest";
import { ACOSH, ADD, AND, ASIN, ASINH, ATAN, ATANH, DIV, EQUAL, EXP, GREATER, GREATER_EQUAL, LESS, LESS_EQUAL, LOG, MUL, NEG, POW, RULE, SIN, SINH, SQRT, SUB, TAN, TANH, app, equals, int, numberNode, rational, sym, type IRNode, type IRSymbol } from "@coding-adventures/symbolic-ir";
import {
  ALL_SOLUTIONS,
  CBRT,
  FREE_INTEGER,
  Frac,
  I_UNIT,
  nsolveFractionPoly,
  nsolvePoly,
  rootsToIr,
  solveCubic,
  solveLinear,
  solveLinearSystem,
  solveQuadratic,
  solveQuartic,
  trySolveInequality,
  trySolveTranscendental,
  type SolveResult,
} from "../src/index";

function frac(n: number, d: number): Frac {
  return new Frac(n, d);
}

function fi(n: number): Frac {
  return Frac.fromInt(n);
}

function expectSolutions(result: SolveResult, expected: readonly IRNode[]): void {
  expect(result.kind).toBe("solutions");
  if (result.kind === "solutions") {
    expect(result.roots.length).toBe(expected.length);
    expected.forEach((node, index) => expect(equals(result.roots[index], node)).toBe(true));
  }
}

function expectContainsSolutions(result: SolveResult, expected: readonly IRNode[]): void {
  expect(result.kind).toBe("solutions");
  if (result.kind === "solutions") {
    expect(result.roots.length).toBe(expected.length);
    expected.forEach((node) => expect(result.roots.some((root) => equals(root, node))).toBe(true));
  }
}

function containsSymbol(node: IRNode, name: string): boolean {
  if (node.kind === "symbol") return node.name === name;
  return node.kind === "apply" && (containsSymbol(node.head, name) || node.args.some((arg) => containsSymbol(arg, name)));
}

function containsHead(node: IRNode, name: string): boolean {
  return node.kind === "apply"
    && ((node.head.kind === "symbol" && node.head.name === name) || node.args.some((arg) => containsHead(arg, name)));
}

function expectNumericRootsClose(
  computed: readonly { readonly re: number; readonly im: number }[],
  expected: readonly { readonly re: number; readonly im: number }[],
  tol = 1e-8,
): void {
  expect(computed.length).toBe(expected.length);
  const used = new Array(expected.length).fill(false);
  for (const root of computed) {
    const match = expected.findIndex((candidate, index) =>
      !used[index] && Math.hypot(root.re - candidate.re, root.im - candidate.im) < tol);
    expect(match).not.toBe(-1);
    used[match] = true;
  }
}

function eq(lhs: IRNode, rhs: IRNode): IRNode {
  return app(EQUAL, [lhs, rhs]);
}

function add(...args: readonly IRNode[]): IRNode {
  return app(ADD, args);
}

function sub(lhs: IRNode, rhs: IRNode): IRNode {
  return app(SUB, [lhs, rhs]);
}

function mul(...args: readonly IRNode[]): IRNode {
  return app(MUL, args);
}

function pow(base: IRNode, exponent: IRNode): IRNode {
  return app(POW, [base, exponent]);
}

function expectNodes(actual: readonly IRNode[] | null, expected: readonly IRNode[]): void {
  expect(actual).not.toBeNull();
  expect(actual?.length).toBe(expected.length);
  expected.forEach((node, index) => expect(equals(actual?.[index] ?? sym("missing"), node)).toBe(true));
}

function ruleValue(rules: readonly IRNode[], variable: IRSymbol): IRNode {
  for (const rule of rules) {
    if (rule.kind === "apply" && equals(rule.head, RULE) && equals(rule.args[0], variable)) {
      return rule.args[1];
    }
  }
  throw new Error(`missing rule for ${variable.name}`);
}

describe("Frac", () => {
  it("normalizes and computes exact arithmetic", () => {
    expect(frac(2, 4).equals(frac(1, 2))).toBe(true);
    expect(frac(1, -2).equals(frac(-1, 2))).toBe(true);
    expect(frac(1, 2).add(frac(1, 4)).equals(frac(3, 4))).toBe(true);
    expect(frac(1, 2).sub(frac(3, 4)).equals(frac(-1, 4))).toBe(true);
    expect(frac(2, 3).mul(frac(3, 4)).equals(frac(1, 2))).toBe(true);
    expect(frac(2, 3).div(frac(4, 5)).equals(frac(5, 6))).toBe(true);
    expect(() => new Frac(1, 0)).toThrow(RangeError);
    expect(() => frac(1, 2).div(Frac.zero())).toThrow(RangeError);
  });
});

describe("solveLinear", () => {
  it("solves linear equations over rationals", () => {
    expectSolutions(solveLinear(fi(2), fi(3)), [rational(-3, 2)]);
    expectSolutions(solveLinear(fi(1), fi(-5)), [int(5)]);
    expectSolutions(solveLinear(fi(0), fi(5)), []);
    expect(solveLinear(fi(0), fi(0))).toBe(ALL_SOLUTIONS);
    expectSolutions(solveLinear(fi(3), fi(0)), [int(0)]);
    expectSolutions(solveLinear(frac(1, 2), frac(1, 4)), [rational(-1, 2)]);
  });
});

describe("solveQuadratic", () => {
  it("solves rational-root quadratics", () => {
    expectSolutions(solveQuadratic(fi(1), fi(-5), fi(6)), [int(2), int(3)]);
    expectSolutions(solveQuadratic(fi(1), fi(-4), fi(4)), [int(2)]);
    expectSolutions(solveQuadratic(fi(0), fi(2), fi(4)), [int(-2)]);
    expect(solveQuadratic(fi(0), fi(0), fi(0))).toBe(ALL_SOLUTIONS);
    expectSolutions(solveQuadratic(fi(4), fi(0), fi(-1)), [rational(-1, 2), rational(1, 2)]);
    expectSolutions(solveQuadratic(fi(1), fi(0), fi(-1)), [int(-1), int(1)]);
    expectSolutions(solveQuadratic(fi(1), fi(-100), fi(2499)), [int(49), int(51)]);
  });

  it("returns symbolic irrational roots", () => {
    const result = solveQuadratic(fi(1), fi(0), fi(-2));
    expect(result.kind).toBe("solutions");
    if (result.kind === "solutions") {
      expect(result.roots.length).toBe(2);
      expect(result.roots.some((root) => containsHead(root, "Sqrt"))).toBe(true);
      expect(equals(result.roots[0], app(DIV, [app(ADD, [int(0), app(SQRT, [int(8)])]), int(2)]))).toBe(true);
    }
  });

  it("returns symbolic complex roots", () => {
    const result = solveQuadratic(fi(1), fi(0), fi(1));
    expect(result.kind).toBe("solutions");
    if (result.kind === "solutions") {
      expect(result.roots.length).toBe(2);
      expect(result.roots.some((root) => containsSymbol(root, I_UNIT))).toBe(true);
      expect(equals(result.roots[0], app(ADD, [int(0), app(MUL, [int(1), sym(I_UNIT)])]))).toBe(true);
    }
  });

  it("keeps non-square negative discriminants symbolic", () => {
    const result = solveQuadratic(fi(1), fi(1), fi(1));
    expect(result.kind).toBe("solutions");
    if (result.kind === "solutions") {
      expect(result.roots.length).toBe(2);
      expect(result.roots.some((root) => containsSymbol(root, I_UNIT))).toBe(true);
      expect(result.roots.some((root) => containsHead(root, "Sqrt"))).toBe(true);
    }
  });
});

describe("solveCubic", () => {
  it("delegates to quadratic when the cubic coefficient is zero", () => {
    expectSolutions(solveCubic(fi(0), fi(1), fi(-5), fi(6)), [int(2), int(3)]);
  });

  it("finds three exact rational roots through deflation", () => {
    expectContainsSolutions(solveCubic(fi(1), fi(-6), fi(11), fi(-6)), [int(1), int(2), int(3)]);
    expectContainsSolutions(solveCubic(fi(2), fi(-3), fi(-11), fi(6)), [int(-2), rational(1, 2), int(3)]);
  });

  it("deduplicates repeated rational roots", () => {
    expectContainsSolutions(solveCubic(fi(1), fi(0), fi(-3), fi(-2)), [int(-1), int(2)]);
    expectSolutions(solveCubic(fi(1), fi(-6), fi(12), fi(-8)), [int(2)]);
  });

  it("returns the rational root plus symbolic complex quadratic roots", () => {
    const result = solveCubic(fi(1), fi(0), fi(0), fi(1));
    expect(result.kind).toBe("solutions");
    if (result.kind === "solutions") {
      expect(result.roots.length).toBe(3);
      expect(result.roots.some((root) => equals(root, int(-1)))).toBe(true);
      expect(result.roots.filter((root) => containsSymbol(root, I_UNIT)).length).toBe(2);
    }
  });

  it("uses Cardano symbolic roots when there is one real root and a complex pair", () => {
    const result = solveCubic(fi(1), fi(0), fi(1), fi(1));
    expect(result.kind).toBe("solutions");
    if (result.kind === "solutions") {
      expect(result.roots.length).toBe(3);
      expect(result.roots.some((root) => containsHead(root, CBRT))).toBe(true);
      expect(result.roots.some((root) => containsSymbol(root, I_UNIT))).toBe(true);
    }
  });

  it("leaves casus irreducibilis unevaluated as an empty solution list", () => {
    expectSolutions(solveCubic(fi(1), fi(0), fi(-3), fi(1)), []);
  });
});

describe("solveQuartic", () => {
  it("delegates to cubic when quartic coefficient is zero", () => {
    expectContainsSolutions(solveQuartic(fi(0), fi(1), fi(-6), fi(11), fi(-6)), [int(1), int(2), int(3)]);
  });

  it("finds four rational roots through rational-root deflation", () => {
    expectContainsSolutions(solveQuartic(fi(1), fi(0), fi(-10), fi(0), fi(9)), [int(-3), int(-1), int(1), int(3)]);
    expectContainsSolutions(solveQuartic(fi(1), fi(-10), fi(35), fi(-50), fi(24)), [int(1), int(2), int(3), int(4)]);
  });

  it("handles zero roots and deduplicates repeated roots", () => {
    expectContainsSolutions(solveQuartic(fi(1), fi(-1), fi(0), fi(0), fi(0)), [int(0), int(1)]);
    expectContainsSolutions(solveQuartic(fi(1), fi(1), fi(0), fi(0), fi(0)), [int(-1), int(0)]);
  });

  it("uses the biquadratic path for no-rational-root even quartics", () => {
    const result = solveQuartic(fi(1), fi(0), fi(4), fi(0), fi(3));
    expect(result.kind).toBe("solutions");
    if (result.kind === "solutions") {
      expect(result.roots.length).toBe(4);
      expect(result.roots.every((root) => containsHead(root, "Sqrt") || containsHead(root, "Neg"))).toBe(true);
    }
  });

  it("uses Ferrari factorization when the resolvent has a rational root", () => {
    const result = solveQuartic(fi(1), fi(0), fi(1), fi(2), fi(6));
    expect(result.kind).toBe("solutions");
    if (result.kind === "solutions") {
      expect(result.roots.length).toBe(4);
      expect(result.roots.every((root) => root.kind === "apply")).toBe(true);
      expect(result.roots.some((root) => containsSymbol(root, I_UNIT))).toBe(true);
    }
  });

  it("leaves quartics without a usable rational resolvent root unevaluated", () => {
    expectSolutions(solveQuartic(fi(1), fi(0), fi(0), fi(1), fi(1)), []);
  });
});

describe("nsolvePoly", () => {
  it("solves linear, quadratic, and cubic polynomials numerically", () => {
    expectNumericRootsClose(nsolvePoly([1, -2]), [{ re: 2, im: 0 }], 1e-10);
    expectNumericRootsClose(nsolvePoly([1, 0, -1]), [{ re: 1, im: 0 }, { re: -1, im: 0 }]);
    expectNumericRootsClose(nsolvePoly([1, 0, 1]), [{ re: 0, im: 1 }, { re: 0, im: -1 }]);
    expectNumericRootsClose(nsolvePoly([1, -6, 11, -6]), [
      { re: 1, im: 0 },
      { re: 2, im: 0 },
      { re: 3, im: 0 },
    ]);
  });

  it("solves quintics and returns no roots for constants", () => {
    const roots = nsolvePoly([1, 0, 0, 0, 0, -1]);
    expect(roots.length).toBe(5);
    roots.forEach((root) => expect(Math.abs(Math.hypot(root.re, root.im) - 1)).toBeLessThan(1e-8));
    expect(nsolvePoly([5])).toEqual([]);
  });

  it("converts numeric roots to symbolic IR", () => {
    expect(rootsToIr([{ re: 2, im: 1e-15 }, { re: -3, im: -1e-14 }])).toEqual([
      numberNode(2),
      numberNode(-3),
    ]);
    const complexRoot = rootsToIr([{ re: 1, im: 2 }])[0];
    expect(complexRoot.kind).toBe("apply");
    expect(containsSymbol(complexRoot, I_UNIT)).toBe(true);
  });

  it("wraps fraction coefficients and returns IR roots", () => {
    const roots = nsolveFractionPoly([fi(1), fi(-6), fi(11), fi(-6)]);
    expect(roots.length).toBe(3);
    const values = roots
      .filter((root) => root.kind === "float")
      .map((root) => root.kind === "float" ? root.value : 0)
      .sort((a, b) => a - b);
    expect(values[0]).toBeCloseTo(1, 8);
    expect(values[1]).toBeCloseTo(2, 8);
    expect(values[2]).toBeCloseTo(3, 8);
  });
});

describe("solveLinearSystem", () => {
  const x = sym("x");
  const y = sym("y");
  const z = sym("z");

  it("solves simple exact 2x2 systems", () => {
    const result = solveLinearSystem([
      eq(add(x, y), int(3)),
      eq(sub(x, y), int(1)),
    ], [x, y]);

    expect(result).not.toBeNull();
    expect(ruleValue(result ?? [], x)).toEqual(int(2));
    expect(ruleValue(result ?? [], y)).toEqual(int(1));
  });

  it("solves systems with rational results", () => {
    const result = solveLinearSystem([
      eq(add(mul(int(2), x), mul(int(3), y)), int(7)),
      eq(sub(mul(int(4), x), y), int(1)),
    ], [x, y]);

    expect(result).not.toBeNull();
    expect(ruleValue(result ?? [], x)).toEqual(rational(5, 7));
    expect(ruleValue(result ?? [], y)).toEqual(rational(13, 7));
  });

  it("solves 3x3 systems and zero-form equations", () => {
    const result = solveLinearSystem([
      eq(add(x, y, z), int(6)),
      eq(add(mul(int(2), x), y), int(5)),
      eq(z, int(3)),
    ], [x, y, z]);

    expect(result).not.toBeNull();
    expect(ruleValue(result ?? [], x)).toEqual(int(2));
    expect(ruleValue(result ?? [], y)).toEqual(int(1));
    expect(ruleValue(result ?? [], z)).toEqual(int(3));

    const zeroForm = solveLinearSystem([add(x, y), sub(x, y)], [x, y]);
    expect(ruleValue(zeroForm ?? [], x)).toEqual(int(0));
    expect(ruleValue(zeroForm ?? [], y)).toEqual(int(0));
  });

  it("returns null for wrong-size, empty, singular, and non-linear systems", () => {
    expect(solveLinearSystem([eq(add(x, y), int(3))], [x, y])).toBeNull();
    expect(solveLinearSystem([], [])).toBeNull();
    expect(solveLinearSystem([
      eq(add(x, y), int(1)),
      eq(add(mul(int(2), x), mul(int(2), y)), int(2)),
    ], [x, y])).toBeNull();
    expect(solveLinearSystem([eq(pow(x, int(2)), int(4))], [x])).toBeNull();
  });

  it("returns Rule nodes in variable order", () => {
    const result = solveLinearSystem([
      eq(add(x, y), int(3)),
      eq(sub(x, y), int(1)),
    ], [x, y]);

    expect(result?.length).toBe(2);
    expect(result?.[0]).toEqual(app(RULE, [x, int(2)]));
    expect(result?.[1]).toEqual(app(RULE, [y, int(1)]));
  });
});

describe("trySolveInequality", () => {
  const x = sym("x");

  it("solves linear inequalities with exact rational boundaries", () => {
    expectNodes(
      trySolveInequality(app(LESS, [add(mul(int(2), x), int(3)), int(0)]), x),
      [app(LESS, [x, rational(-3, 2)])],
    );
    expectNodes(
      trySolveInequality(app(GREATER_EQUAL, [sub(x, int(1)), int(0)]), x),
      [app(GREATER_EQUAL, [x, int(1)])],
    );
  });

  it("solves quadratic inequalities into interval conditions", () => {
    expectNodes(
      trySolveInequality(app(GREATER, [sub(pow(x, int(2)), int(1)), int(0)]), x),
      [app(LESS, [x, int(-1)]), app(GREATER, [x, int(1)])],
    );
    expectNodes(
      trySolveInequality(app(LESS_EQUAL, [sub(pow(x, int(2)), int(1)), int(0)]), x),
      [app(AND, [app(GREATER_EQUAL, [x, int(-1)]), app(LESS_EQUAL, [x, int(1)])])],
    );
  });

  it("handles shifted quadratic intervals and all-real sentinels", () => {
    expectNodes(
      trySolveInequality(app(LESS, [add(sub(pow(x, int(2)), mul(int(3), x)), int(2)), int(0)]), x),
      [app(AND, [app(GREATER, [x, int(1)]), app(LESS, [x, int(2)])])],
    );
    expectNodes(
      trySolveInequality(app(GREATER_EQUAL, [pow(x, int(2)), int(0)]), x),
      [app(GREATER_EQUAL, [int(0), int(0)])],
    );
  });

  it("uses numeric boundaries for irrational roots and rejects non-polynomials", () => {
    const result = trySolveInequality(app(GREATER, [sub(pow(x, int(2)), int(2)), int(0)]), x);
    expect(result?.length).toBe(2);
    expect(result?.[0].kind).toBe("apply");
    if (result?.[0].kind === "apply" && result[1].kind === "apply") {
      expect(equals(result[0].head, LESS)).toBe(true);
      expect(equals(result[1].head, GREATER)).toBe(true);
      expect(result[0].args[1].kind).toBe("float");
      expect(result[1].args[1].kind).toBe("float");
    }
    expect(trySolveInequality(app(GREATER, [app(sym("Sin"), [x]), int(0)]), x)).toBeNull();
    expect(trySolveInequality(eq(x, int(0)), x)).toBeNull();
  });
});

describe("trySolveTranscendental", () => {
  const x = sym("x");

  it("solves direct exponential and logarithmic equations", () => {
    expectNodes(
      trySolveTranscendental(eq(app(EXP, [x]), int(2)), x),
      [app(LOG, [int(2)])],
    );
    expectNodes(
      trySolveTranscendental(eq(app(LOG, [add(mul(int(2), x), int(3))]), int(5)), x),
      [app(DIV, [app(SUB, [app(EXP, [int(5)]), int(3)]), int(2)])],
    );
  });

  it("solves direct trig equations with periodic families", () => {
    const twoPiK = app(MUL, [int(2), app(MUL, [sym("%pi"), sym(FREE_INTEGER)])]);
    expectNodes(
      trySolveTranscendental(eq(app(SIN, [x]), int(0)), x),
      [
        app(ADD, [app(ASIN, [int(0)]), twoPiK]),
        app(ADD, [app(SUB, [sym("%pi"), app(ASIN, [int(0)])]), twoPiK]),
      ],
    );
    expectNodes(
      trySolveTranscendental(eq(app(TAN, [add(mul(int(2), x), int(1))]), int(3)), x),
      [app(DIV, [app(SUB, [app(ADD, [app(ATAN, [int(3)]), app(MUL, [sym("%pi"), sym(FREE_INTEGER)])]), int(1)]), int(2)])],
    );
  });

  it("solves direct hyperbolic equations", () => {
    expectNodes(
      trySolveTranscendental(eq(app(SINH, [x]), int(2)), x),
      [app(ASINH, [int(2)])],
    );
    expectNodes(
      trySolveTranscendental(eq(app(TANH, [add(x, int(4))]), rational(1, 2)), x),
      [app(SUB, [app(ATANH, [rational(1, 2)]), int(4)])],
    );
    expectNodes(
      trySolveTranscendental(eq(app(sym("Cosh"), [x]), int(3)), x),
      [app(ACOSH, [int(3)]), app(NEG, [app(ACOSH, [int(3)])])],
    );
  });

  it("handles bare and reversed equations and rejects unsupported patterns", () => {
    expect(trySolveTranscendental(app(EXP, [x]), x)?.length).toBe(1);
    expect(trySolveTranscendental(eq(int(2), app(EXP, [x])), x)?.length).toBe(1);
    expect(trySolveTranscendental(eq(app(SIN, [sym("y")]), int(0)), x)).toBeNull();
    expect(trySolveTranscendental(eq(app(SIN, [app(SIN, [x])]), int(0)), x)).toBeNull();
  });
});
