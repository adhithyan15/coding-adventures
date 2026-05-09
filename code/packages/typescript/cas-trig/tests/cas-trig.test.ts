import { describe, expect, it } from "vitest";
import {
  PI,
  acosEval,
  asinEval,
  atanEval,
  cosEval,
  expandTrig,
  extractPiMultiple,
  powerReduce,
  sinEval,
  tanEval,
  trigSimplify,
} from "../src/index";
import {
  ACOS,
  ADD,
  ASIN,
  ATAN,
  COS,
  MUL,
  NEG,
  POW,
  SIN,
  SQRT,
  SUB,
  TAN,
  app,
  equals,
  int,
  numberNode,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

function piMult(n: number, d: number): IRNode {
  return app(MUL, [rational(n, d), sym(PI)]);
}

function sqrtOf(n: number): IRNode {
  return app(SQRT, [int(n)]);
}

function expectFloatClose(node: IRNode, expected: number): void {
  expect(node.kind).toBe("float");
  if (node.kind === "float") expect(node.value).toBeCloseTo(expected, 10);
}

describe("special values", () => {
  it("evaluates sine exact values and periodic forms", () => {
    expect(equals(sinEval(int(0)), int(0))).toBe(true);
    expect(equals(sinEval(sym(PI)), int(0))).toBe(true);
    expect(equals(sinEval(piMult(1, 2)), int(1))).toBe(true);
    expect(equals(sinEval(piMult(3, 2)), int(-1))).toBe(true);
    expect(equals(sinEval(piMult(1, 6)), rational(1, 2))).toBe(true);
    expect(equals(sinEval(piMult(7, 6)), rational(-1, 2))).toBe(true);
    expect(equals(sinEval(piMult(1, 4)), app(MUL, [rational(1, 2), sqrtOf(2)]))).toBe(true);
    expect(equals(sinEval(piMult(5, 4)), app(NEG, [app(MUL, [rational(1, 2), sqrtOf(2)])]))).toBe(true);
    expect(equals(sinEval(piMult(1, 3)), app(MUL, [rational(1, 2), sqrtOf(3)]))).toBe(true);
    expect(equals(sinEval(app(MUL, [int(2), sym(PI)])), int(0))).toBe(true);
    expect(equals(sinEval(app(NEG, [piMult(1, 2)])), int(-1))).toBe(true);
  });

  it("evaluates cosine exact values and periodic forms", () => {
    expect(equals(cosEval(int(0)), int(1))).toBe(true);
    expect(equals(cosEval(sym(PI)), int(-1))).toBe(true);
    expect(equals(cosEval(piMult(1, 2)), int(0))).toBe(true);
    expect(equals(cosEval(piMult(1, 3)), rational(1, 2))).toBe(true);
    expect(equals(cosEval(piMult(2, 3)), rational(-1, 2))).toBe(true);
    expect(equals(cosEval(piMult(1, 4)), app(MUL, [rational(1, 2), sqrtOf(2)]))).toBe(true);
    expect(equals(cosEval(piMult(3, 4)), app(NEG, [app(MUL, [rational(1, 2), sqrtOf(2)])]))).toBe(true);
    expect(equals(cosEval(piMult(1, 6)), app(MUL, [rational(1, 2), sqrtOf(3)]))).toBe(true);
    expect(equals(cosEval(app(MUL, [int(2), sym(PI)])), int(1))).toBe(true);
    expect(equals(cosEval(app(NEG, [sym(PI)])), int(-1))).toBe(true);
  });

  it("evaluates tangent exact values and leaves poles unevaluated", () => {
    expect(equals(tanEval(int(0)), int(0))).toBe(true);
    expect(equals(tanEval(sym(PI)), int(0))).toBe(true);
    expect(equals(tanEval(piMult(1, 4)), int(1))).toBe(true);
    expect(equals(tanEval(piMult(3, 4)), int(-1))).toBe(true);
    expect(equals(tanEval(piMult(1, 3)), sqrtOf(3))).toBe(true);
    expect(equals(tanEval(piMult(2, 3)), app(NEG, [sqrtOf(3)]))).toBe(true);
    expect(equals(tanEval(piMult(1, 6)), app(MUL, [rational(1, 3), sqrtOf(3)]))).toBe(true);
    expect(equals(tanEval(piMult(5, 6)), app(NEG, [app(MUL, [rational(1, 3), sqrtOf(3)])]))).toBe(true);
    expect(equals(tanEval(piMult(1, 2)), app(TAN, [piMult(1, 2)]))).toBe(true);
  });
});

describe("numeric and inverse trig", () => {
  it("evaluates finite numeric arguments", () => {
    expect(equals(sinEval(numberNode(0)), int(0))).toBe(true);
    expect(equals(cosEval(numberNode(0)), int(1))).toBe(true);
    expect(equals(sinEval(numberNode(Math.PI)), int(0))).toBe(true);
    expect(equals(cosEval(numberNode(Math.PI)), int(-1))).toBe(true);
    expectFloatClose(sinEval(numberNode(1)), Math.sin(1));
    expectFloatClose(cosEval(numberNode(1)), Math.cos(1));
    expectFloatClose(tanEval(numberNode(0.5)), Math.tan(0.5));
    expectFloatClose(sinEval(rational(1, 2)), Math.sin(0.5));
  });

  it("evaluates inverse trig and preserves symbolic or out-of-domain values", () => {
    expect(equals(atanEval(int(0)), numberNode(0))).toBe(true);
    expectFloatClose(atanEval(int(1)), Math.PI / 4);
    expect(equals(atanEval(sym("x")), app(ATAN, [sym("x")]))).toBe(true);
    expect(equals(asinEval(int(0)), numberNode(0))).toBe(true);
    expectFloatClose(asinEval(int(1)), Math.PI / 2);
    expect(equals(asinEval(int(2)), app(ASIN, [int(2)]))).toBe(true);
    expect(equals(acosEval(int(2)), app(ACOS, [int(2)]))).toBe(true);
  });
});

describe("tree operations", () => {
  it("simplifies trig nodes bottom-up", () => {
    expect(equals(trigSimplify(int(5)), int(5))).toBe(true);
    expect(equals(trigSimplify(app(SIN, [int(0)])), int(0))).toBe(true);
    expect(equals(trigSimplify(app(COS, [sym(PI)])), int(-1))).toBe(true);
    const expr = app(ADD, [app(SIN, [int(0)]), app(COS, [sym(PI)])]);
    expect(equals(trigSimplify(expr), app(ADD, [int(0), int(-1)]))).toBe(true);
    expect(equals(trigSimplify(app(SIN, [sym("x")])), app(SIN, [sym("x")]))).toBe(true);
  });

  it("extracts pi multiples", () => {
    expect(extractPiMultiple(int(0))).toEqual([0n, 1n]);
    expect(extractPiMultiple(sym(PI))).toEqual([1n, 1n]);
    expect(extractPiMultiple(app(NEG, [sym(PI)]))).toEqual([-1n, 1n]);
    expect(extractPiMultiple(piMult(3, 4))).toEqual([3n, 4n]);
    expect(extractPiMultiple(sym("x"))).toBeNull();
  });
});

describe("expandTrig", () => {
  it("expands angle addition and subtraction", () => {
    const x = sym("x");
    const y = sym("y");
    expect(equals(expandTrig(app(SIN, [app(ADD, [x, y])])), app(ADD, [
      app(MUL, [app(SIN, [x]), app(COS, [y])]),
      app(MUL, [app(COS, [x]), app(SIN, [y])]),
    ]))).toBe(true);
    expect(equals(expandTrig(app(COS, [app(ADD, [x, y])])), app(SUB, [
      app(MUL, [app(COS, [x]), app(COS, [y])]),
      app(MUL, [app(SIN, [x]), app(SIN, [y])]),
    ]))).toBe(true);
    expect(equals(expandTrig(app(SIN, [app(SUB, [x, y])])), app(SUB, [
      app(MUL, [app(SIN, [x]), app(COS, [y])]),
      app(MUL, [app(COS, [x]), app(SIN, [y])]),
    ]))).toBe(true);
  });

  it("expands negation and double angles", () => {
    const x = sym("x");
    expect(equals(expandTrig(app(SIN, [app(NEG, [x])])), app(NEG, [app(SIN, [x])]))).toBe(true);
    expect(equals(expandTrig(app(COS, [app(NEG, [x])])), app(COS, [x]))).toBe(true);
    expect(equals(expandTrig(app(SIN, [app(MUL, [int(2), x])])), app(MUL, [int(2), app(MUL, [app(SIN, [x]), app(COS, [x])])]))).toBe(true);
    const sinX = app(SIN, [x]);
    const cosX = app(COS, [x]);
    expect(equals(expandTrig(app(COS, [app(MUL, [int(2), x])])), app(SUB, [app(MUL, [cosX, cosX]), app(MUL, [sinX, sinX])]))).toBe(true);
    expect(equals(expandTrig(app(ADD, [int(1), x])), app(ADD, [int(1), x]))).toBe(true);
  });
});

describe("powerReduce", () => {
  it("reduces squared sin and cos", () => {
    const x = sym("x");
    const cos2x = app(COS, [app(MUL, [int(2), x])]);
    expect(equals(powerReduce(app(POW, [app(SIN, [x]), int(2)])), app(MUL, [rational(1, 2), app(SUB, [int(1), cos2x])]))).toBe(true);
    expect(equals(powerReduce(app(POW, [app(COS, [x]), int(2)])), app(MUL, [rational(1, 2), app(ADD, [int(1), cos2x])]))).toBe(true);
    expect(equals(powerReduce(app(POW, [x, int(3)])), app(POW, [x, int(3)]))).toBe(true);
  });

  it("walks into compound expressions", () => {
    const x = sym("x");
    const expr = app(ADD, [app(POW, [app(SIN, [x]), int(2)]), app(POW, [app(COS, [x]), int(2)])]);
    const result = powerReduce(expr);
    expect(result.kind).toBe("apply");
    if (result.kind === "apply") {
      expect(equals(result.head, ADD)).toBe(true);
      expect(result.args.every((arg) => arg.kind === "apply" && equals(arg.head, MUL))).toBe(true);
    }
  });
});
