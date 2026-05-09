import { describe, expect, it } from "vitest";
import { ADD, DIV, MUL, POW, SUB, app, equals, int, rational, sym, type IRNode } from "@coding-adventures/symbolic-ir";
import { LIMIT, PolynomialError, limitDirect, taylorPolynomial } from "../src/index";

function expectEqual(actual: IRNode, expected: IRNode): void {
  expect(equals(actual, expected), `${display(actual)} !== ${display(expected)}`).toBe(true);
}

function display(node: IRNode): string {
  return JSON.stringify(node, (_key, value) => (typeof value === "bigint" ? value.toString() : value));
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
