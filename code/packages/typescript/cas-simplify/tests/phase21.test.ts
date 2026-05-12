import { describe, expect, it } from "vitest";
import {
  ADD,
  COS,
  COSH,
  DIV,
  EQUAL,
  EXP,
  GREATER,
  GREATER_EQUAL,
  IRNode,
  LESS,
  LOG,
  MUL,
  NOT_EQUAL,
  POW,
  SIN,
  SINH,
  SQRT,
  SUB,
  TAN,
  TANH,
  app,
  int,
  rational,
  sym,
} from "@coding-adventures/symbolic-ir";
import {
  AssumptionContext,
  IMAGINARY_UNIT,
  demoivre,
  exponentialize,
  logcontract,
  logexpand,
  radcan,
} from "../src/index";

const x = sym("x");
const y = sym("y");
const a = sym("a");
const b = sym("b");
const n = sym("n");

function greaterZero(node: IRNode): IRNode {
  return app(GREATER, [node, int(0)]);
}

function positiveContext(...nodes: readonly IRNode[]): AssumptionContext {
  const ctx = new AssumptionContext();
  for (const node of nodes) ctx.assumeRelation(greaterZero(node));
  return ctx;
}

describe("AssumptionContext", () => {
  it("tracks and forgets sign and property facts", () => {
    const ctx = new AssumptionContext();
    expect(ctx.isPositive("x")).toBeUndefined();
    expect(ctx.isInteger("n")).toBe(false);

    ctx.assumeRelation(greaterZero(x));
    ctx.assumeRelation(app(NOT_EQUAL, [y, int(0)]));
    ctx.assumeProperty(n, sym("integer"));

    expect(ctx.isPositive("x")).toBe(true);
    expect(ctx.isNegative("x")).toBe(false);
    expect(ctx.signOf("x")).toBe(1);
    expect(ctx.isTrueRelation(greaterZero(x))).toBe(true);
    expect(ctx.isTrueRelation(app(EQUAL, [y, int(0)]))).toBe(false);
    expect(ctx.isInteger("n")).toBe(true);
    expect(ctx.hasAnyFacts("x")).toBe(true);
    expect(ctx.factsFor("n")).toEqual(["integer"]);
    expect(ctx.symbolsWithFacts()).toEqual(["n", "x", "y"]);

    ctx.forgetRelation(greaterZero(x));
    expect(ctx.isPositive("x")).toBeUndefined();
    ctx.forgetAll();
    expect(ctx.isInteger("n")).toBe(false);
  });

  it("records negative and nonnegative assumptions", () => {
    const ctx = new AssumptionContext();
    ctx.assumeRelation(app(LESS, [x, int(0)]));
    ctx.assumeRelation(app(GREATER_EQUAL, [y, int(0)]));
    expect(ctx.isNegative("x")).toBe(true);
    expect(ctx.isPositive("x")).toBe(false);
    expect(ctx.signOf("x")).toBe(-1);
    expect(ctx.isNonneg("y")).toBe(true);
    expect(ctx.isNegative("y")).toBe(false);
  });

  it("returns deterministic fact and symbol metadata", () => {
    const ctx = new AssumptionContext();
    ctx.assumeProperty(n, sym("positive"));
    ctx.assumeProperty(n, sym("integer"));
    ctx.assumeRelation(greaterZero(x));

    expect(ctx.factsFor("n")).toEqual(["integer", "positive"]);
    expect(ctx.factsFor("missing")).toEqual([]);
    expect(ctx.symbolsWithFacts()).toEqual(["n", "x"]);
  });
});

describe("radcan", () => {
  it("simplifies integer perfect-square radicals", () => {
    expect(radcan(app(SQRT, [int(4)]))).toEqual(int(2));
    expect(radcan(app(SQRT, [int(9)]))).toEqual(int(3));
    expect(radcan(app(SQRT, [int(2)]))).toEqual(app(SQRT, [int(2)]));
  });

  it("uses positivity when simplifying Sqrt(x^2)", () => {
    expect(radcan(app(SQRT, [app(POW, [x, int(2)])]))).toEqual(app(SQRT, [app(POW, [x, int(2)])]));
    expect(radcan(app(SQRT, [app(POW, [x, int(2)])]), positiveContext(x))).toEqual(x);
  });

  it("extracts and merges square-root products", () => {
    const extracted = radcan(app(SQRT, [app(MUL, [app(POW, [x, int(2)]), y])]), positiveContext(x));
    expect(extracted).toEqual(app(MUL, [x, app(SQRT, [y])]));

    const merged = radcan(app(MUL, [app(SQRT, [a]), app(SQRT, [b])]));
    expect(merged).toEqual(app(SQRT, [app(MUL, [a, b])]));
  });

  it("cancels square and exp/log inverse shapes", () => {
    expect(radcan(app(POW, [app(SQRT, [x]), int(2)]))).toEqual(x);
    expect(radcan(app(EXP, [app(LOG, [x])]))).toEqual(x);
    expect(radcan(app(LOG, [app(EXP, [x])]))).toEqual(x);
  });

  it("collects common non-half rational exponents", () => {
    const third = rational(1, 3);
    expect(radcan(app(MUL, [app(POW, [a, third]), app(POW, [b, third])]))).toEqual(
      app(POW, [app(MUL, [a, b]), third]),
    );
  });
});

describe("logcontract and logexpand", () => {
  it("contracts log sums, differences, and numeric multiples", () => {
    expect(logcontract(app(ADD, [app(LOG, [a]), app(LOG, [b])]))).toEqual(app(LOG, [app(MUL, [a, b])]));
    expect(logcontract(app(ADD, [app(LOG, [a]), x, app(LOG, [b])]))).toEqual(
      app(ADD, [x, app(LOG, [app(MUL, [a, b])])]),
    );
    expect(logcontract(app(SUB, [app(LOG, [a]), app(LOG, [b])]))).toEqual(app(LOG, [app(DIV, [a, b])]));
    expect(logcontract(app(MUL, [int(2), app(LOG, [x])]))).toEqual(app(LOG, [app(POW, [x, int(2)])]));
    expect(logcontract(app(MUL, [x, app(LOG, [y])]))).toEqual(app(MUL, [x, app(LOG, [y])]));
  });

  it("expands logs over powers products and quotients", () => {
    expect(logexpand(app(LOG, [app(POW, [x, int(3)])]))).toEqual(app(MUL, [int(3), app(LOG, [x])]));
    expect(logexpand(app(LOG, [app(POW, [x, rational(1, 2)])]))).toEqual(
      app(MUL, [rational(1, 2), app(LOG, [x])]),
    );
    expect(logexpand(app(LOG, [app(MUL, [a, b, x])]))).toEqual(
      app(ADD, [app(ADD, [app(LOG, [a]), app(LOG, [b])]), app(LOG, [x])]),
    );
    expect(logexpand(app(LOG, [app(DIV, [a, b])]))).toEqual(app(SUB, [app(LOG, [a]), app(LOG, [b])]));
    expect(logexpand(app(LOG, [x]), positiveContext(x))).toEqual(app(LOG, [x]));
  });
});

describe("exponentialize", () => {
  it("rewrites circular trig functions to exponential form", () => {
    const sinX = exponentialize(app(SIN, [x]));
    expect(sinX.kind).toBe("apply");
    expect(sinX.kind === "apply" ? sinX.head : undefined).toEqual(DIV);
    expect(sinX.kind === "apply" ? sinX.args[0] : undefined).toMatchObject({ head: SUB });
    expect(sinX.kind === "apply" ? sinX.args[1] : undefined).toEqual(app(MUL, [int(2), IMAGINARY_UNIT]));

    const cosX = exponentialize(app(COS, [x]));
    expect(cosX.kind === "apply" ? cosX.args[0] : undefined).toMatchObject({ head: ADD });
    expect(cosX.kind === "apply" ? cosX.args[1] : undefined).toEqual(int(2));

    const tanX = exponentialize(app(TAN, [x]));
    expect(tanX.kind === "apply" ? tanX.args[1] : undefined).toMatchObject({ head: ADD });
  });

  it("rewrites hyperbolic functions to exponential form", () => {
    const sinhX = exponentialize(app(SINH, [x]));
    const coshX = exponentialize(app(COSH, [x]));
    const tanhX = exponentialize(app(TANH, [x]));
    expect(sinhX.kind === "apply" ? sinhX.args[0] : undefined).toMatchObject({ head: SUB });
    expect(coshX.kind === "apply" ? coshX.args[0] : undefined).toMatchObject({ head: ADD });
    expect(tanhX.kind === "apply" ? tanhX.args[1] : undefined).toMatchObject({ head: ADD });
  });
});

describe("demoivre", () => {
  it("decomposes pure imaginary exponentials", () => {
    expect(demoivre(app(EXP, [IMAGINARY_UNIT]))).toEqual(
      app(ADD, [app(COS, [int(1)]), app(MUL, [IMAGINARY_UNIT, app(SIN, [int(1)])])]),
    );
    expect(demoivre(app(EXP, [app(MUL, [IMAGINARY_UNIT, y])]))).toEqual(
      app(ADD, [app(COS, [y]), app(MUL, [IMAGINARY_UNIT, app(SIN, [y])])]),
    );
    expect(demoivre(app(EXP, [app(MUL, [y, IMAGINARY_UNIT])]))).toEqual(
      app(ADD, [app(COS, [y]), app(MUL, [IMAGINARY_UNIT, app(SIN, [y])])]),
    );
  });

  it("splits mixed real plus imaginary exponentials", () => {
    expect(demoivre(app(EXP, [app(ADD, [x, app(MUL, [IMAGINARY_UNIT, y])])]))).toEqual(
      app(MUL, [app(EXP, [x]), app(ADD, [app(COS, [y]), app(MUL, [IMAGINARY_UNIT, app(SIN, [y])])])]),
    );
    expect(demoivre(app(EXP, [x]))).toEqual(app(EXP, [x]));
    expect(demoivre(app(ADD, [app(EXP, [app(MUL, [IMAGINARY_UNIT, x])]), y]))).toEqual(
      app(ADD, [app(ADD, [app(COS, [x]), app(MUL, [IMAGINARY_UNIT, app(SIN, [x])])]), y]),
    );
  });
});
