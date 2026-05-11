import { describe, expect, it } from "vitest";
import {
  ADD,
  ASSIGN,
  D,
  DEFINE,
  DIV,
  EQUAL,
  GREATER,
  IF,
  LESS,
  LIST,
  MUL,
  NEG,
  POW,
  SIN,
  SUB,
  app,
  int,
  sym,
  toDisplayString,
} from "@coding-adventures/symbolic-ir";

import { DISPLAY, SUPPRESS, compileMacsyma } from "../src/index.js";

function one(source: string) {
  const statements = compileMacsyma(source);
  expect(statements).toHaveLength(1);
  return statements[0];
}

describe("macsyma compiler", () => {
  it("compiles atoms", () => {
    expect(one("42;")).toEqual(int(42));
    expect(one("x;")).toEqual(sym("x"));
    expect(one("%pi;")).toEqual(sym("%pi"));
  });

  it("compiles arithmetic precedence and associativity", () => {
    expect(one("1 + 2 * 3;")).toEqual(app(ADD, [int(1), app(MUL, [int(2), int(3)])]));
    expect(one("a - b - c;")).toEqual(app(SUB, [app(SUB, [sym("a"), sym("b")]), sym("c")]));
    expect(one("a / b * c;")).toEqual(app(MUL, [app(DIV, [sym("a"), sym("b")]), sym("c")]));
    expect(one("a^b^c;")).toEqual(app(POW, [sym("a"), app(POW, [sym("b"), sym("c")])]));
    expect(one("-x;")).toEqual(app(NEG, [sym("x")]));
  });

  it("compiles function calls and standard function names", () => {
    expect(one("f(x, y);")).toEqual(app(sym("f"), [sym("x"), sym("y")]));
    expect(one("diff(x^2, x);")).toEqual(app(D, [app(POW, [sym("x"), int(2)]), sym("x")]));
    expect(one("sin(x);")).toEqual(app(SIN, [sym("x")]));
  });

  it("compiles comparisons and logic", () => {
    expect(one("x = 4;")).toEqual(app(EQUAL, [sym("x"), int(4)]));
    expect(one("a < b;")).toEqual(app(LESS, [sym("a"), sym("b")]));
    expect(one("a > b;")).toEqual(app(GREATER, [sym("a"), sym("b")]));
    expect(toDisplayString(one("a and b and c;"))).toBe("And(a, b, c)");
  });

  it("compiles assignment and function definition", () => {
    expect(one("a : 5;")).toEqual(app(ASSIGN, [sym("a"), int(5)]));
    expect(one("f(x) := x^2;")).toEqual(app(DEFINE, [
      sym("f"),
      app(LIST, [sym("x")]),
      app(POW, [sym("x"), int(2)]),
    ]));
  });

  it("compiles lists and optional terminator wrappers", () => {
    expect(one("[1, 2, 3];")).toEqual(app(LIST, [int(1), int(2), int(3)]));
    expect(compileMacsyma("x; y$", { wrapTerminators: true })).toEqual([
      app(DISPLAY, [sym("x")]),
      app(SUPPRESS, [sym("y")]),
    ]);
  });

  it("compiles if expressions to symbolic If", () => {
    expect(one("if x < 0 then -x else x;")).toEqual(app(IF, [
      app(LESS, [sym("x"), int(0)]),
      app(NEG, [sym("x")]),
      sym("x"),
    ]));
  });
});
