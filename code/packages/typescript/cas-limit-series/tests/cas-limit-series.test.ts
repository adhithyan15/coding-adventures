import { describe, expect, it } from "vitest";
import { ADD, DIV, MUL, NEG, POW, SUB, app, equals, int, rational, sym, type IRNode } from "@coding-adventures/symbolic-ir";
import { LIMIT, PolynomialError, limitAdvanced, limitDirect, taylorPolynomial } from "../src/index";

function expectEqual(actual: IRNode, expected: IRNode): void {
  expect(equals(actual, expected), `${display(actual)} !== ${display(expected)}`).toBe(true);
}

function display(node: IRNode): string {
  return JSON.stringify(node, (_key, value) => (typeof value === "bigint" ? value.toString() : value));
}

function differentiateFixture(node: IRNode, variable: IRNode): IRNode {
  if (equals(node, variable)) return int(1);
  if (node.kind === "integer" || node.kind === "rational" || node.kind === "float" || node.kind === "symbol") {
    return int(0);
  }
  if (node.kind !== "apply") return int(0);
  if (equals(node.head, ADD)) return app(ADD, node.args.map((arg) => differentiateFixture(arg, variable)));
  if (equals(node.head, SUB) && node.args.length === 2) {
    return app(SUB, [
      differentiateFixture(node.args[0], variable),
      differentiateFixture(node.args[1], variable),
    ]);
  }
  if (equals(node.head, NEG) && node.args.length === 1) return app(NEG, [differentiateFixture(node.args[0], variable)]);
  if (equals(node.head, MUL) && node.args.length === 2) {
    const [f, g] = node.args;
    return app(ADD, [
      app(MUL, [differentiateFixture(f, variable), g]),
      app(MUL, [f, differentiateFixture(g, variable)]),
    ]);
  }
  if (equals(node.head, DIV) && node.args.length === 2) {
    const [f, g] = node.args;
    return app(DIV, [
      app(SUB, [
        app(MUL, [differentiateFixture(f, variable), g]),
        app(MUL, [f, differentiateFixture(g, variable)]),
      ]),
      app(POW, [g, int(2)]),
    ]);
  }
  if (equals(node.head, POW) && node.args.length === 2 && node.args[1].kind === "integer") {
    const [base, exponent] = node.args;
    if (exponent.value === 0n) return int(0);
    return app(MUL, [
      app(MUL, [int(exponent.value), app(POW, [base, int(exponent.value - 1n)])]),
      differentiateFixture(base, variable),
    ]);
  }
  return int(0);
}

function evaluateFixture(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const args = node.args.map(evaluateFixture);
  if (equals(node.head, ADD)) {
    if (args.every(isIntegerNode)) return int(args.reduce((sum, arg) => sum + arg.value, 0n));
    const nonzero = args.filter((arg) => !isIntegerValue(arg, 0n));
    if (nonzero.length === 1) return nonzero[0];
  }
  if (equals(node.head, SUB) && args.length === 2) {
    if (isIntegerNode(args[0]) && isIntegerNode(args[1])) return int(args[0].value - args[1].value);
    if (isIntegerValue(args[1], 0n)) return args[0];
  }
  if (equals(node.head, NEG) && args.length === 1 && isIntegerNode(args[0])) return int(-args[0].value);
  if (equals(node.head, MUL)) {
    if (args.every(isIntegerNode)) return int(args.reduce((product, arg) => product * arg.value, 1n));
    if (args.length === 2 && isIntegerValue(args[0], 1n)) return args[1];
    if (args.length === 2 && isIntegerValue(args[1], 1n)) return args[0];
  }
  if (equals(node.head, DIV) && args.length === 2) {
    const [numer, denom] = args;
    if (equals(numer, denom)) return int(1);
    if (isIntegerValue(denom, 1n)) return numer;
    if (isIntegerNode(numer) && isIntegerNode(denom) && denom.value !== 0n) return rational(numer.value, denom.value);
  }
  if (equals(node.head, POW) && args.length === 2) {
    const [base, exponent] = args;
    if (isIntegerValue(exponent, 0n)) return int(1);
    if (isIntegerValue(exponent, 1n)) return base;
    if (isIntegerNode(base) && isIntegerNode(exponent) && exponent.value >= 0n) {
      return int(base.value ** exponent.value);
    }
  }
  return app(node.head, args);
}

function isIntegerNode(node: IRNode): node is Extract<IRNode, { readonly kind: "integer" }> {
  return node.kind === "integer";
}

function isIntegerValue(node: IRNode, value: bigint): boolean {
  return isIntegerNode(node) && node.value === value;
}

describe("limitDirect", () => {
  it("substitutes finite points without simplifying", () => {
    const x = sym("x");
    const expr = app(ADD, [app(POW, [x, int(2)]), int(1)]);
    expectEqual(limitDirect(expr, x, int(2)), app(ADD, [app(POW, [int(2), int(2)]), int(1)]));
  });

  it("substitutes in compound expressions", () => {
    const x = sym("x");
    expectEqual(limitDirect(app(MUL, [int(2), x]), x, int(3)), app(MUL, [int(2), int(3)]));
  });

  it("does not simplify substitution output", () => {
    const x = sym("x");
    expectEqual(limitDirect(app(ADD, [x, int(0)]), x, int(5)), app(ADD, [int(5), int(0)]));
  });

  it("leaves expressions without the variable unchanged", () => {
    const x = sym("x");
    const y = sym("y");
    const expr = app(MUL, [int(2), y]);
    expectEqual(limitDirect(expr, x, int(0)), expr);
  });

  it("returns unevaluated Limit for literal 0/0", () => {
    const x = sym("x");
    const expr = app(DIV, [int(0), int(0)]);
    expectEqual(limitDirect(expr, x, int(0)), app(sym(LIMIT), [expr, x, int(0)]));
  });

  it("keeps constants unchanged", () => {
    expectEqual(limitDirect(int(42), sym("x"), int(5)), int(42));
  });
});

describe("limitAdvanced", () => {
  it("returns a direct finite result after exact substitution", () => {
    const x = sym("x");
    const expr = app(ADD, [app(POW, [x, int(2)]), int(1)]);
    expectEqual(limitAdvanced(expr, x, int(2), { evaluate: evaluateFixture }), int(5));
  });

  it("returns unevaluated Limit for indeterminate quotients without callbacks", () => {
    const x = sym("x");
    const expr = app(DIV, [
      app(SUB, [app(POW, [x, int(2)]), int(1)]),
      app(SUB, [x, int(1)]),
    ]);
    expectEqual(limitAdvanced(expr, x, int(1)), app(sym(LIMIT), [expr, x, int(1)]));
  });

  it("uses injected differentiation for a simple L'Hopital rational form", () => {
    const x = sym("x");
    const expr = app(DIV, [
      app(SUB, [app(POW, [x, int(2)]), int(1)]),
      app(SUB, [x, int(1)]),
    ]);
    expectEqual(limitAdvanced(expr, x, int(1), {
      differentiate: differentiateFixture,
      evaluate: evaluateFixture,
    }), int(2));
  });

  it("rewrites a zero-times-infinity product into a quotient form", () => {
    const x = sym("x");
    const expr = app(MUL, [x, app(DIV, [int(1), x])]);
    expectEqual(limitAdvanced(expr, x, int(0), {
      differentiate: differentiateFixture,
      evaluate: evaluateFixture,
    }), int(1));
  });

  it("keeps indeterminate powers unevaluated without callbacks", () => {
    const x = sym("x");
    const expr = app(POW, [x, x]);
    expectEqual(limitAdvanced(expr, x, int(0), { direction: "plus" }), app(sym(LIMIT), [expr, x, int(0), sym("plus")]));
  });
});

describe("taylorPolynomial", () => {
  it("returns constants unchanged", () => {
    expectEqual(taylorPolynomial(int(7), sym("x"), int(2), 3), int(7));
  });

  it("expands x and x^2 around zero", () => {
    const x = sym("x");
    expectEqual(taylorPolynomial(x, x, int(0), 2), x);
    expectEqual(taylorPolynomial(app(POW, [x, int(2)]), x, int(0), 2), app(POW, [x, int(2)]));
  });

  it("truncates by requested order", () => {
    const x = sym("x");
    expectEqual(taylorPolynomial(app(POW, [x, int(2)]), x, int(0), 1), int(0));
    expectEqual(taylorPolynomial(x, x, int(0), 0), int(0));
  });

  it("expands around a non-zero point", () => {
    const x = sym("x");
    const expr = app(POW, [x, int(2)]);
    const expected = app(ADD, [
      int(1),
      app(MUL, [int(2), app(SUB, [x, int(1)])]),
      app(POW, [app(SUB, [x, int(1)]), int(2)]),
    ]);
    expectEqual(taylorPolynomial(expr, x, int(1), 2), expected);
  });

  it("handles compound polynomial addition", () => {
    const x = sym("x");
    const expr = app(ADD, [app(POW, [x, int(2)]), int(1)]);
    expectEqual(taylorPolynomial(expr, x, int(0), 2), app(ADD, [int(1), app(POW, [x, int(2)])]));
  });

  it("handles subtraction and negation", () => {
    const x = sym("x");
    expectEqual(taylorPolynomial(app(SUB, [x, int(1)]), x, int(0), 1), app(ADD, [int(-1), x]));
  });

  it("handles linear polynomials at non-zero points", () => {
    const x = sym("x");
    const expr = app(ADD, [app(MUL, [int(3), x]), int(2)]);
    expectEqual(taylorPolynomial(expr, x, int(1), 1), app(ADD, [int(5), app(MUL, [int(3), app(SUB, [x, int(1)])])]));
  });

  it("handles rational coefficients", () => {
    const x = sym("x");
    expectEqual(taylorPolynomial(app(DIV, [x, int(2)]), x, int(0), 1), app(MUL, [rational(1, 2), x]));
  });

  it("returns the constant term at order zero", () => {
    const x = sym("x");
    const expr = app(ADD, [app(POW, [x, int(2)]), app(MUL, [int(3), x]), int(1)]);
    expectEqual(taylorPolynomial(expr, x, int(0), 0), int(1));
  });

  it("rejects non-polynomial input", () => {
    const x = sym("x");
    expect(() => taylorPolynomial(app(sym("Sin"), [x]), x, int(0), 3)).toThrow(PolynomialError);
    expect(() => taylorPolynomial(app(MUL, [sym("y"), x]), x, int(0), 2)).toThrow(PolynomialError);
  });
});
