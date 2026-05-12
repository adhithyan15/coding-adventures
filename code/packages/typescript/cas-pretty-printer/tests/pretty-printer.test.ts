import { describe, expect, it } from "vitest";
import { ADD, DIV, INV, LIST, MUL, NEG, POW, SQRT, app, int, numberNode, rational, stringNode, sym } from "@coding-adventures/symbolic-ir";
import { Box, MacsymaDialect, MathematicaDialect, atomBox, formatLisp, hbox, pretty, pretty2D, vbox } from "../src/index";

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

  it("formats MACSYMA subtraction sugar for negative literals and products", () => {
    expect(pretty(app(ADD, [int(-5), sym("y")]))).toBe("y - 5");
    expect(pretty(app(ADD, [sym("a"), app(MUL, [sym("b"), app(NEG, [sym("c")])])]))).toBe("a - b * c");
  });

  it("formats MACSYMA multiplication sugar for inverse and negative factors", () => {
    expect(pretty(app(MUL, [sym("a"), app(INV, [sym("b")])]))).toBe("a / b");
    expect(pretty(app(MUL, [sym("a"), app(NEG, [sym("b")])]))).toBe("-(a * b)");
    expect(pretty(app(MUL, [int(-1), sym("x"), sym("y")]))).toBe("-(x * y)");
  });

  it("formats MACSYMA function aliases", () => {
    const aliases: ReadonlyArray<readonly [string, string]> = [
      ["Select", "sublist"],
      ["MakeList", "makelist"],
      ["Inverse", "invert"],
      ["RatSimplify", "ratsimp"],
      ["Apart", "partfrac"],
      ["TrigSimplify", "trigsimp"],
      ["TrigExpand", "trigexpand"],
      ["TrigReduce", "trigreduce"],
      ["Re", "realpart"],
      ["Im", "imagpart"],
      ["Arg", "carg"],
      ["RectForm", "rectform"],
      ["PolarForm", "polarform"],
      ["IsPrime", "primep"],
      ["NextPrime", "next_prime"],
      ["PrevPrime", "prev_prime"],
      ["FactorInteger", "ifactor"],
      ["Divisors", "divisors"],
      ["Totient", "totient"],
      ["MoebiusMu", "moebius"],
      ["JacobiSymbol", "jacobi"],
      ["ChineseRemainder", "chinese"],
      ["IntegerLength", "numdigits"],
    ];

    for (const [head, alias] of aliases) {
      expect(pretty(app(sym(head), [sym("x")]), MacsymaDialect)).toBe(`${alias}(x)`);
    }
    expect(pretty(sym("ImaginaryUnit"), MacsymaDialect)).toBe("%i");
  });

  it("formats dialect-specific calls and lists", () => {
    const expr = app(LIST, [sym("x"), app(sym("Sin"), [sym("x")])]);
    expect(pretty(expr, MacsymaDialect)).toBe("[x, sin(x)]");
    expect(pretty(expr, MathematicaDialect)).toBe("{x, Sin[x]}");
  });

  it("keeps the default pretty API linear while accepting explicit linear style", () => {
    const expr = app(DIV, [sym("x"), sym("y")]);
    expect(pretty(expr)).toBe("x / y");
    expect(pretty(expr, MacsymaDialect, { style: "linear" })).toBe("x / y");
  });
});

describe("2D box layout", () => {
  it("builds atom geometry", () => {
    const box = atomBox("42");
    expect(box.width).toBe(2);
    expect(box.height).toBe(1);
    expect(box.baseline).toBe(0);
    expect(box.lines).toEqual(["42"]);
    expect(box.render()).toBe("42");
  });

  it("pads atom boxes by alignment", () => {
    expect(atomBox("x").padWidth(5).lines).toEqual(["  x  "]);
    expect(atomBox("x").padWidth(4, "left").lines).toEqual(["x   "]);
    expect(atomBox("x").padWidth(4, "right").lines).toEqual(["   x"]);
  });

  it("composes hbox and vbox primitives", () => {
    expect(hbox([atomBox("a"), atomBox("b")], " ").render()).toBe("a b");

    const stacked = vbox([atomBox("wide"), atomBox("x")]);
    expect(stacked.width).toBe(4);
    expect(stacked.height).toBe(2);
    expect(stacked.lines).toEqual(["wide", " x  "]);
  });

  it("aligns hbox inputs on their baselines", () => {
    const fraction = new Box([" a ", "───", " b "], 1);
    const rendered = hbox([atomBox("x"), atomBox(" + "), fraction]).render();
    expect(rendered).toBe([
      "     a ",
      "x + ───",
      "     b ",
    ].join("\n"));
  });

  it("renders division as numerator, bar, and denominator rows", () => {
    expect(pretty2D(app(DIV, [sym("x"), sym("y")]))).toBe([
      " x ",
      "───",
      " y ",
    ].join("\n"));
    expect(pretty(app(DIV, [sym("x"), sym("y")]), MacsymaDialect, { style: "2d" })).toContain("───");
  });

  it("renders power exponents above the base row", () => {
    expect(pretty2D(app(POW, [sym("x"), int(2)]))).toBe([
      " 2",
      "x ",
    ].join("\n"));
  });

  it("renders square root with a radical and overline", () => {
    expect(pretty2D(app(SQRT, [sym("x")]))).toBe([
      "  ┌───┐",
      "√ │ x │",
    ].join("\n"));
  });

  it("renders nested fractions with baseline-preserving rows", () => {
    const expr = app(DIV, [app(DIV, [sym("x"), sym("y")]), sym("z")]);
    expect(pretty2D(expr)).toBe([
      "  x  ",
      " ─── ",
      "  y  ",
      "─────",
      "  z  ",
    ].join("\n"));
  });

  it("renders arithmetic and list forms in 2D", () => {
    expect(pretty(app(NEG, [sym("x")]), MacsymaDialect, "2d")).toBe("-x");
    expect(pretty(app(ADD, [sym("x"), sym("y")]), MacsymaDialect, "2d")).toBe("x + y");
    expect(pretty(app(ADD, [sym("x"), app(NEG, [sym("y")])]), MacsymaDialect, "2d")).toBe("x - y");
    expect(pretty(app(MUL, [sym("x"), sym("y")]), MacsymaDialect, "2d")).toBe("x*y");
    expect(pretty(app(LIST, [sym("x"), app(DIV, [int(1), int(2)])]), MacsymaDialect, "2d")).toBe([
      "     1  ",
      "[x, ───]",
      "     2  ",
    ].join("\n"));
  });

  it("formats leaf nodes in 2D", () => {
    expect(pretty(int(42), MacsymaDialect, "2d")).toBe("42");
    expect(pretty(rational(1, 2), MacsymaDialect, "2d")).toBe("1/2");
    expect(pretty(numberNode(3.14), MacsymaDialect, "2d")).toBe("3.14");
    expect(pretty(stringNode("hello"), MacsymaDialect, "2d")).toBe("\"hello\"");
  });

  it("falls back to linear formatting for unsupported heads", () => {
    const expr = app(sym("Sin"), [app(ADD, [sym("x"), int(1)])]);
    expect(pretty(expr, MacsymaDialect, { style: "2d" })).toBe("sin(x + 1)");
  });

  it("rejects unknown pretty styles at runtime", () => {
    expect(() => pretty(sym("x"), MacsymaDialect, { style: "3d" as "linear" })).toThrow(/unsupported style/);
  });
});
