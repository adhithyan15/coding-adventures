import { describe, expect, it } from "vitest";
import { ADD, INV, LIST, MUL, NEG, POW, app, int, sym } from "@coding-adventures/symbolic-ir";
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
});
