import { describe, expect, it } from "vitest";
import {
  ADD,
  COS,
  COSH,
  DIV,
  EXP,
  MUL,
  POW,
  SIN,
  SUB,
  app,
  equals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  DIRAC_DELTA,
  ILT,
  LAPLACE,
  UNIT_STEP,
  buildLaplaceHandlerTable,
  diracDeltaHandler,
  iltHandler,
  inverseLaplace,
  laplaceHandler,
  laplaceTransform,
  unitStepHandler,
} from "../src/index";

const t = sym("t");
const s = sym("s");

describe("laplaceTransform", () => {
  it("handles constants and powers", () => {
    expect(laplaceTransform(int(1), t, s)).toEqual(app(DIV, [int(1), s]));
    expect(laplaceTransform(t, t, s)).toEqual(app(DIV, [int(1), app(POW, [s, int(2)])]));
    expect(laplaceTransform(app(POW, [t, int(3)]), t, s)).toEqual(app(DIV, [int(6), app(POW, [s, int(4)])]));
  });

  it("handles exp, trig, hyperbolic, and shifted trig products", () => {
    expect(laplaceTransform(app(EXP, [app(MUL, [int(3), t])]), t, s)).toEqual(app(DIV, [int(1), app(SUB, [s, int(3)])]));
    expect(laplaceTransform(app(SIN, [app(MUL, [int(2), t])]), t, s)).toEqual(
      app(DIV, [int(2), app(ADD, [app(POW, [s, int(2)]), app(POW, [int(2), int(2)])])]),
    );

    const expSin = app(MUL, [app(EXP, [t]), app(SIN, [app(MUL, [int(2), t])])]);
    expect(isHead(laplaceTransform(expSin, t, s), DIV)).toBe(true);
    expect(isHead(laplaceTransform(app(COSH, [t]), t, s), DIV)).toBe(true);
  });

  it("applies linearity and falls through honestly", () => {
    const sum = app(ADD, [app(SIN, [t]), app(COS, [t])]);
    expect(isHead(laplaceTransform(sum, t, s), ADD)).toBe(true);
    expect(isHead(laplaceTransform(app(MUL, [int(5), app(SIN, [t])]), t, s), MUL)).toBe(true);

    const unknown = app(sym("Mystery"), [t]);
    expect(laplaceTransform(unknown, t, s)).toEqual(app(LAPLACE, [unknown, t, s]));
  });

  it("handles special heads", () => {
    expect(laplaceTransform(app(DIRAC_DELTA, [t]), t, s)).toEqual(int(1));
    expect(laplaceTransform(app(UNIT_STEP, [t]), t, s)).toEqual(app(DIV, [int(1), s]));
  });
});

describe("inverseLaplace", () => {
  it("handles standard inverse table entries", () => {
    expect(inverseLaplace(app(DIV, [int(1), s]), s, t)).toEqual(app(UNIT_STEP, [t]));
    expect(inverseLaplace(app(DIV, [int(1), app(SUB, [s, int(3)])]), s, t)).toEqual(app(EXP, [app(MUL, [int(3), t])]));
    expect(inverseLaplace(app(DIV, [int(2), app(ADD, [app(POW, [s, int(2)]), int(4)])]), s, t)).toEqual(
      app(SIN, [app(MUL, [int(2), t])]),
    );
    expect(inverseLaplace(app(DIV, [s, app(SUB, [app(POW, [s, int(2)]), int(1)])]), s, t)).toEqual(
      app(COSH, [app(MUL, [int(1), t])]),
    );
  });

  it("returns unevaluated ILT for unknown forms", () => {
    const unknown = app(sym("Unknown"), [s]);
    expect(inverseLaplace(unknown, s, t)).toEqual(app(ILT, [unknown, s, t]));
  });
});

describe("handlers", () => {
  const id = (node: IRNode): IRNode => node;

  it("dispatches transform handlers", () => {
    expect(laplaceHandler(app(LAPLACE, [int(1), t, s]), id)).toEqual(app(DIV, [int(1), s]));
    expect(iltHandler(app(ILT, [app(DIV, [int(1), s]), s, t]), id)).toEqual(app(UNIT_STEP, [t]));
  });

  it("evaluates special functions and exposes table keys", () => {
    expect(diracDeltaHandler(app(DIRAC_DELTA, [int(0)]))).toEqual(int(1));
    expect(unitStepHandler(app(UNIT_STEP, [int(-1)]))).toEqual(int(0));
    expect(unitStepHandler(app(UNIT_STEP, [int(0)]))).toEqual(rational(1, 2));
    expect(unitStepHandler(app(UNIT_STEP, [int(2)]))).toEqual(int(1));
    expect([...buildLaplaceHandlerTable().keys()]).toEqual(["Laplace", "ILT", "DiracDelta", "UnitStep"]);
  });
});

function isHead(node: IRNode, head: IRNode): boolean {
  return node.kind === "apply" && equals(node.head, head);
}
