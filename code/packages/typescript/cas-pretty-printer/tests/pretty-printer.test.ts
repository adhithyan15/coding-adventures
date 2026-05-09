import { describe, expect, it } from "vitest";
import { ADD, LIST, MUL, NEG, POW, app, int, sym } from "@coding-adventures/symbolic-ir";
import { MacsymaDialect, MathematicaDialect, formatLisp, pretty } from "../src/index";

describe("cas-pretty-printer", () => {
  it("formats Lisp prefix trees", () => {
    const expr = app(ADD, [int(2), app(MUL, [int(3), sym("x")])]);
    expect(formatLisp(expr)).toBe("(Add 2 (Mul 3 x))");
  });

  it("formats MACSYMA infix with precedence", () => {
    const expr = app(ADD, [sym("x"), app(MUL, [int(3), app(POW, [sym("y"), int(2)])])]);
    expect(pretty(expr, MacsymaDialect)).toBe("x + 3 * y ^ 2");
  });

  it("formats common MACSYMA sugar", () => {
    expect(pretty(app(ADD, [sym("x"), app(NEG, [sym("y")])]))).toBe("x - y");
    expect(pretty(app(MUL, [int(-1), sym("x")]))).toBe("-x");
  });

  it("formats dialect-specific calls and lists", () => {
    const expr = app(LIST, [sym("x"), app(sym("Sin"), [sym("x")])]);
    expect(pretty(expr, MacsymaDialect)).toBe("[x, sin(x)]");
    expect(pretty(expr, MathematicaDialect)).toBe("{x, Sin[x]}");
  });
});
