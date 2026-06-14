import { describe, expect, it } from "vitest";
import {
  ADD,
  COS,
  DIV,
  EXP,
  MUL,
  NEG,
  POW,
  SIN,
  app,
  equals,
  int,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  DIRAC_DELTA,
  FOURIER,
  IFOURIER,
  IMAGINARY_UNIT,
  PI,
  UNIT_STEP,
  buildFourierHandlerTable,
  fourierHandler,
  fourierTransform,
  ifourierHandler,
  ifourierTransform,
} from "../src/index";

const t = sym("t");
const omega = sym("omega");

describe("fourierTransform", () => {
  it("handles delta and constants", () => {
    expect(fourierTransform(app(DIRAC_DELTA, [t]), t, omega)).toEqual(int(1));
    const constant = fourierTransform(int(1), t, omega);
    expect(containsHead(constant, DIRAC_DELTA)).toBe(true);
    expect(containsSymbol(constant, "%pi")).toBe(true);
  });

  it("handles causal exp, complex exp, trig, and gaussian entries", () => {
    const causal = app(EXP, [app(NEG, [app(MUL, [int(2), t])])]);
    const causalResult = fourierTransform(causal, t, omega);
    expect(isHead(causalResult, DIV)).toBe(true);
    expect(containsSymbol(causalResult, "ImaginaryUnit")).toBe(true);

    const complex = app(EXP, [app(MUL, [app(MUL, [IMAGINARY_UNIT, int(3)]), t])]);
    expect(containsHead(fourierTransform(complex, t, omega), DIRAC_DELTA)).toBe(true);
    expect(containsHead(fourierTransform(app(SIN, [t]), t, omega), DIRAC_DELTA)).toBe(true);
    expect(containsSymbol(fourierTransform(app(COS, [t]), t, omega), "%pi")).toBe(true);

    const gaussian = app(EXP, [app(NEG, [app(POW, [t, int(2)])])]);
    expect(containsHead(fourierTransform(gaussian, t, omega), sym("Sqrt"))).toBe(true);
  });

  it("applies linearity and falls through", () => {
    const sum = app(ADD, [app(DIRAC_DELTA, [t]), int(1)]);
    expect(isHead(fourierTransform(sum, t, omega), ADD)).toBe(true);
    expect(isHead(fourierTransform(app(MUL, [int(4), app(DIRAC_DELTA, [t])]), t, omega), MUL)).toBe(true);

    const unknown = app(sym("Mystery"), [t]);
    expect(fourierTransform(unknown, t, omega)).toEqual(app(FOURIER, [unknown, t, omega]));
  });
});

describe("ifourierTransform", () => {
  it("handles inverse table entries", () => {
    expect(ifourierTransform(int(1), omega, t)).toEqual(app(DIRAC_DELTA, [t]));
    expect(ifourierTransform(app(DIRAC_DELTA, [omega]), omega, t)).toEqual(app(DIV, [int(1), app(MUL, [int(2), PI])]));

    const twoPiDelta = app(MUL, [app(MUL, [int(2), PI]), app(DIRAC_DELTA, [omega])]);
    expect(ifourierTransform(twoPiDelta, omega, t)).toEqual(int(1));

    const causal = app(DIV, [int(1), app(ADD, [int(2), app(MUL, [IMAGINARY_UNIT, omega])])]);
    const inverse = ifourierTransform(causal, omega, t);
    expect(containsHead(inverse, UNIT_STEP)).toBe(true);
    expect(containsHead(inverse, EXP)).toBe(true);
  });
});

describe("handlers", () => {
  const id = (node: IRNode): IRNode => node;

  it("dispatches forward and inverse handlers", () => {
    expect(fourierHandler(app(FOURIER, [app(DIRAC_DELTA, [t]), t, omega]), id)).toEqual(int(1));
    expect(ifourierHandler(app(IFOURIER, [int(1), omega, t]), id)).toEqual(app(DIRAC_DELTA, [t]));
    expect([...buildFourierHandlerTable().keys()]).toEqual(["Fourier", "IFourier"]);
  });
});

function isHead(node: IRNode, head: IRNode): boolean {
  return node.kind === "apply" && equals(node.head, head);
}

function containsHead(node: IRNode, head: IRNode): boolean {
  return node.kind === "apply" && (equals(node.head, head) || node.args.some((arg) => containsHead(arg, head)));
}

function containsSymbol(node: IRNode, name: string): boolean {
  if (node.kind === "symbol") return node.name === name;
  if (node.kind === "apply") return containsSymbol(node.head, name) || node.args.some((arg) => containsSymbol(arg, name));
  return false;
}
