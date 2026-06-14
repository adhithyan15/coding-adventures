import { describe, expect, it } from "vitest";
import {
  ARG_HEAD,
  ABS_HEAD,
  ATAN2_HEAD,
  EXP_HEAD,
  IMAGINARY_UNIT,
  IMAGINARY_UNIT_NODE,
  POLAR_FORM_HEAD,
  SQRT_HEAD,
  argument,
  complexNormalize,
  complexPow,
  conjugate,
  imagPart,
  modulus,
  polarForm,
  realPart,
  rectForm,
  splitComplex,
} from "../src/index";
import { ADD, MUL, NEG, POW, SUB, app, equals, int, numberNode, sym, type IRNode } from "@coding-adventures/symbolic-ir";

function z34(): IRNode {
  return app(ADD, [int(3), app(MUL, [int(4), sym(IMAGINARY_UNIT)])]);
}

function expectFloatClose(node: IRNode, expected: number): void {
  expect(node.kind).toBe("float");
  if (node.kind === "float") expect(node.value).toBeCloseTo(expected, 10);
}

describe("complexNormalize", () => {
  it("keeps real literals and I in canonical form", () => {
    expect(equals(complexNormalize(int(5)), int(5))).toBe(true);
    expect(equals(complexNormalize(int(0)), int(0))).toBe(true);
    expect(equals(complexNormalize(sym(IMAGINARY_UNIT)), sym(IMAGINARY_UNIT))).toBe(true);
  });

  it("normalizes pure imaginary and rectangular values", () => {
    const pure = app(MUL, [int(3), sym(IMAGINARY_UNIT)]);
    expect(equals(realPart(complexNormalize(pure)), int(0))).toBe(true);
    expect(equals(imagPart(complexNormalize(pure)), int(3))).toBe(true);
    expect(equals(realPart(complexNormalize(z34())), int(3))).toBe(true);
    expect(equals(imagPart(complexNormalize(z34())), int(4))).toBe(true);
  });

  it("cycles powers of I exactly", () => {
    expect(equals(complexNormalize(app(POW, [sym(IMAGINARY_UNIT), int(2)])), int(-1))).toBe(true);
    expect(equals(realPart(complexNormalize(app(POW, [sym(IMAGINARY_UNIT), int(3)]))), int(0))).toBe(true);
    expect(equals(imagPart(complexNormalize(app(POW, [sym(IMAGINARY_UNIT), int(3)]))), int(-1))).toBe(true);
    expect(equals(complexNormalize(app(POW, [sym(IMAGINARY_UNIT), int(4)])), int(1))).toBe(true);
    expect(equals(imagPart(complexNormalize(app(POW, [sym(IMAGINARY_UNIT), int(-1)]))), int(-1))).toBe(true);
  });

  it("multiplies complex values", () => {
    const a = app(ADD, [int(1), sym(IMAGINARY_UNIT)]);
    const b = app(ADD, [int(1), app(NEG, [sym(IMAGINARY_UNIT)])]);
    const product = app(MUL, [a, b]);
    expect(equals(realPart(complexNormalize(product)), int(2))).toBe(true);
    expect(equals(imagPart(complexNormalize(product)), int(0))).toBe(true);

    expect(equals(complexNormalize(app(MUL, [sym(IMAGINARY_UNIT), sym(IMAGINARY_UNIT)])), int(-1))).toBe(true);

    const threeI = app(MUL, [int(3), sym(IMAGINARY_UNIT)]);
    const fourI = app(MUL, [int(4), sym(IMAGINARY_UNIT)]);
    expect(equals(complexNormalize(app(MUL, [threeI, fourI])), int(-12))).toBe(true);

    const oneMinusTwoI = app(SUB, [int(1), app(MUL, [int(2), sym(IMAGINARY_UNIT)])]);
    const result = complexNormalize(app(MUL, [z34(), oneMinusTwoI]));
    expect(equals(realPart(result), int(11))).toBe(true);
    expect(equals(imagPart(result), int(-2))).toBe(true);
  });
});

describe("rectForm and polarForm", () => {
  it("rectForm delegates to rectangular normalization", () => {
    const onePlusTwoI = app(ADD, [int(1), app(MUL, [int(2), sym(IMAGINARY_UNIT)])]);
    const result = rectForm(onePlusTwoI);
    expect(equals(realPart(result), int(1))).toBe(true);
    expect(equals(imagPart(result), int(2))).toBe(true);
  });

  it("builds symbolic polar form for numeric rectangular complex expressions", () => {
    const result = polarForm(z34());
    const expected = app(MUL, [
      app(SQRT_HEAD, [app(ADD, [app(POW, [int(3), int(2)]), app(POW, [int(4), int(2)])])]),
      app(EXP_HEAD, [app(MUL, [IMAGINARY_UNIT_NODE, app(ATAN2_HEAD, [int(4), int(3)])])]),
    ]);
    expect(equals(result, expected)).toBe(true);
  });

  it("builds symbolic polar form for symbolic rectangular complex expressions", () => {
    const expr = app(ADD, [sym("a"), app(MUL, [sym("b"), sym(IMAGINARY_UNIT)])]);
    const result = polarForm(expr);
    const expected = app(MUL, [
      app(SQRT_HEAD, [app(ADD, [app(POW, [sym("a"), int(2)]), app(POW, [sym("b"), int(2)])])]),
      app(EXP_HEAD, [app(MUL, [IMAGINARY_UNIT_NODE, app(ATAN2_HEAD, [sym("b"), sym("a")])])]),
    ]);
    expect(equals(result, expected)).toBe(true);
  });

  it("leaves pure real expressions as unevaluated PolarForm", () => {
    expect(equals(polarForm(sym("x")), app(POLAR_FORM_HEAD, [sym("x")]))).toBe(true);
    expect(equals(polarForm(int(3)), app(POLAR_FORM_HEAD, [int(3)]))).toBe(true);
  });
});

describe("parts and conjugates", () => {
  it("extracts real and imaginary parts", () => {
    expect(equals(realPart(int(7)), int(7))).toBe(true);
    expect(equals(realPart(app(MUL, [int(5), sym(IMAGINARY_UNIT)])), int(0))).toBe(true);
    expect(equals(realPart(sym(IMAGINARY_UNIT)), int(0))).toBe(true);
    expect(equals(imagPart(sym(IMAGINARY_UNIT)), int(1))).toBe(true);
    expect(equals(imagPart(int(5)), int(0))).toBe(true);
    expect(equals(imagPart(z34()), int(4))).toBe(true);
  });

  it("returns split parts directly", () => {
    const [re, im] = splitComplex(z34());
    expect(equals(re, int(3))).toBe(true);
    expect(equals(im, int(4))).toBe(true);
  });

  it("conjugates real, imaginary, and rectangular expressions", () => {
    expect(equals(realPart(conjugate(int(5))), int(5))).toBe(true);
    expect(equals(imagPart(conjugate(int(5))), int(0))).toBe(true);
    expect(equals(realPart(conjugate(z34())), int(3))).toBe(true);
    expect(equals(imagPart(conjugate(z34())), int(-4))).toBe(true);
    expect(equals(imagPart(conjugate(sym(IMAGINARY_UNIT))), int(-1))).toBe(true);
  });
});

describe("polar helpers", () => {
  it("computes numeric modulus and argument", () => {
    expectFloatClose(modulus(z34()), 5);
    expectFloatClose(modulus(int(3)), 3);
    expectFloatClose(modulus(sym(IMAGINARY_UNIT)), 1);
    expectFloatClose(modulus(int(0)), 0);
    expectFloatClose(argument(int(1)), 0);
    expectFloatClose(argument(int(-1)), Math.PI);
    expectFloatClose(argument(sym(IMAGINARY_UNIT)), Math.PI / 2);
  });

  it("returns symbolic Abs and Arg for non-numeric parts", () => {
    const xPlusI = app(ADD, [sym("x"), sym(IMAGINARY_UNIT)]);
    expect(equals(modulus(xPlusI), app(ABS_HEAD, [xPlusI]))).toBe(true);
    expect(equals(argument(xPlusI), app(ARG_HEAD, [xPlusI]))).toBe(true);
  });
});

describe("complexPow", () => {
  it("handles I and simple numeric powers", () => {
    expect(equals(complexPow(sym(IMAGINARY_UNIT), int(4)), int(1))).toBe(true);
    expect(equals(complexPow(sym(IMAGINARY_UNIT), int(2)), int(-1))).toBe(true);
    expect(equals(complexPow(sym(IMAGINARY_UNIT), int(0)), int(1))).toBe(true);

    const onePlusI = app(ADD, [int(1), sym(IMAGINARY_UNIT)]);
    const squared = complexPow(onePlusI, int(2));
    expect(equals(realPart(squared), int(0))).toBe(true);
    expect(equals(imagPart(squared), int(2))).toBe(true);

    const zSquared = complexPow(z34(), int(2));
    expect(equals(realPart(zSquared), int(-7))).toBe(true);
    expect(equals(imagPart(zSquared), int(24))).toBe(true);
  });

  it("handles inverse and unevaluated fallbacks", () => {
    const inverse = complexPow(z34(), int(-1));
    expect(equals(realPart(inverse), numberNode(0.12))).toBe(true);
    expect(equals(imagPart(inverse), numberNode(-0.16))).toBe(true);

    const x = sym("x");
    expect(equals(complexPow(x, int(2)), app(POW, [x, int(2)]))).toBe(true);
    expect(equals(complexPow(int(0), int(-1)), app(POW, [int(0), int(-1)]))).toBe(true);
    expect(equals(complexPow(sym(IMAGINARY_UNIT), numberNode(2)), app(POW, [sym(IMAGINARY_UNIT), numberNode(2)]))).toBe(true);
  });
});
