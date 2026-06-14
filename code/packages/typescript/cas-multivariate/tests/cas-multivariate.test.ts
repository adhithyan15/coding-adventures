import { describe, expect, it } from "vitest";
import {
  ADD,
  LIST,
  MUL,
  NEG,
  POW,
  RULE,
  SIN,
  SUB,
  app,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  ConversionError,
  Fraction,
  GROEBNER,
  GrobnerError,
  IDEAL_SOLVE,
  MPoly,
  POLY_REDUCE,
  buchberger,
  buildMultivariateHandlerTable,
  cmpMonomials,
  divMonomial,
  divReductionStep,
  divides,
  extractPolyList,
  extractVarList,
  frac,
  groebnerHandler,
  idealSolve,
  idealSolveHandler,
  irToMPoly,
  lcmMonomial,
  makeVar,
  mpolyToIR,
  polyReduceHandler,
  rationalRoots,
  reducePoly,
  sPoly,
  solveUnivariate,
  totalDegree,
} from "../src/index";

const x = sym("x");
const y = sym("y");

function p(terms: Array<readonly [readonly number[], Fraction | bigint | number]>, nvars = 2): MPoly {
  return new MPoly(terms, nvars);
}

function expectPoly(poly: MPoly, terms: Array<readonly [readonly number[], Fraction | bigint | number]>, nvars = poly.nvars): void {
  expect(poly.equals(new MPoly(terms, nvars))).toBe(true);
}

function list(...args: IRNode[]): IRNode {
  return app(LIST, args);
}

describe("monomial helpers", () => {
  it("orders monomials in lex, grlex, and grevlex", () => {
    expect(cmpMonomials([2, 0], [1, 5], "lex")).toBe(1);
    expect(cmpMonomials([0, 3], [1, 1], "grlex")).toBe(1);
    expect(cmpMonomials([3, 0, 0], [1, 1, 1], "grevlex")).toBe(1);
  });

  it("computes lcm, division, divisibility, and degree", () => {
    expect(lcmMonomial([2, 1, 0], [1, 2, 3])).toEqual([2, 2, 3]);
    expect(divides([1, 1], [2, 3])).toBe(true);
    expect(divides([2, 1], [1, 2])).toBe(false);
    expect(divMonomial([3, 2], [1, 1])).toEqual([2, 1]);
    expect(totalDegree([3, 2, 1])).toBe(6);
  });
});

describe("MPoly", () => {
  it("constructs constants, variables, and cleans zero coefficients", () => {
    expect(MPoly.zero(2).isZero()).toBe(true);
    expectPoly(MPoly.constant(frac(3, 2), 2), [[[0, 0], frac(3, 2)]]);
    expectPoly(MPoly.constant(0, 3), [], 3);
    expectPoly(MPoly.monomial([2, 1], 3, 2), [[[2, 1], 3]]);
    expectPoly(p([[[2, 0], 1], [[0, 1], 0], [[0, 0], -1]]), [[[2, 0], 1], [[0, 0], -1]]);
    expectPoly(makeVar(1, 2), [[[0, 1], 1]]);
  });

  it("computes leading terms and arithmetic", () => {
    const poly = p([[[2, 1], 3], [[0, 3], 2], [[0, 1], 1]]);
    expect(poly.lm("grlex")).toEqual([2, 1]);
    expect(poly.lc("grlex").equals(3)).toBe(true);
    expectPoly(poly.lt("grlex"), [[[2, 1], 3]]);

    const a = p([[[1, 0], 2]]);
    const b = p([[[1, 0], -1], [[0, 1], 3]]);
    expectPoly(a.add(b), [[[1, 0], 1], [[0, 1], 3]]);
    expect(a.add(a.neg()).isZero()).toBe(true);
    expectPoly(p([[[1, 0], 1], [[0, 0], 1]]).mul(p([[[1, 0], 1], [[0, 0], -1]])), [[[2, 0], 1], [[0, 0], -1]]);
    expectPoly(p([[[2, 0], 2]]).scale(frac(3, 2)), [[[2, 0], 3]]);
    expectPoly(p([[[1, 0], 1], [[0, 0], 1]]).mulMonomial([1, 0]), [[[2, 0], 1], [[1, 0], 1]]);
  });

  it("supports univariate extraction, derivatives, and evaluation", () => {
    const poly = p([[[2, 0], 1], [[0, 0], -1]]);
    expect(poly.totalDegree()).toBe(2);
    expect(poly.isUnivariate()).toBe(0);
    expect(poly.toUnivariateCoeffs(0).map(String)).toEqual(["-1", "0", "1"]);
    expectPoly(p([[[2, 1], 1], [[1, 0], 1]]).diff(0), [[[1, 1], 2], [[0, 0], 1]]);
    expectPoly(p([[[2, 0], 1], [[0, 1], 1]]).evalAt(0, 2), [[[0, 1], 1], [[0, 0], 4]]);
  });
});

describe("reduction and Groebner bases", () => {
  it("computes S-polynomials", () => {
    expect(sPoly(p([[[2, 0], 1]]), p([[[1, 1], 1]]), "grlex").isZero()).toBe(true);
    expectPoly(
      sPoly(p([[[2, 0], 1], [[0, 1], 1]]), p([[[1, 1], 1], [[0, 0], 1]]), "grlex"),
      [[[0, 2], 1], [[1, 0], -1]],
    );
    expect(() => sPoly(p([[[1, 0], 1]]), MPoly.zero(2))).toThrow();
  });

  it("reduces polynomials with multivariate division", () => {
    expect(reducePoly(p([[[2, 0], 1], [[0, 0], -1]]), [p([[[1, 0], 1], [[0, 0], -1]])], "lex").isZero()).toBe(true);
    expectPoly(
      reducePoly(p([[[2, 0], 1], [[0, 1], 1]]), [p([[[2, 0], 1], [[0, 0], -1]])], "grlex"),
      [[[0, 1], 1], [[0, 0], 1]],
    );
    const step = divReductionStep(p([[[2, 0], 1]]), p([[[1, 0], 1]]), "lex");
    expect(step).not.toBeNull();
    expectPoly(step![0], [[[1, 0], 1]]);
  });

  it("computes reduced Groebner bases", () => {
    const f1 = p([[[1, 0], 1], [[0, 1], 1], [[0, 0], -1]]);
    const f2 = p([[[1, 0], 1], [[0, 1], -1]]);
    const basis = buchberger([f1, f2], "lex");
    expect(basis).toHaveLength(2);
    expect(reducePoly(f1, basis, "lex").isZero()).toBe(true);
    expect(reducePoly(f2, basis, "lex").isZero()).toBe(true);
    expect(buchberger([MPoly.zero(2)], "grlex")).toEqual([]);
    expect(() => buchberger([p([[[9, 0], 1]])], "grlex")).toThrow(GrobnerError);
  });
});

describe("solving", () => {
  it("finds rational roots and solves univariate polynomials", () => {
    expect(new Set(rationalRoots([frac(-1), frac(0), frac(1)]).map(String))).toEqual(new Set(["1", "-1"]));
    expect(rationalRoots([frac(-2), frac(0), frac(1)])).toEqual([]);
    expect(solveUnivariate([frac(-4), frac(2)])?.map(String)).toEqual(["2"]);
    expect(new Set(solveUnivariate([frac(-4), frac(0), frac(1)])?.map(String))).toEqual(new Set(["2", "-2"]));
    expect(solveUnivariate([frac(1), frac(0), frac(1)])).toEqual([]);
    expect(new Set(solveUnivariate([frac(-6), frac(11), frac(-6), frac(1)])?.map(String))).toEqual(new Set(["1", "2", "3"]));
  });

  it("solves triangular ideals", () => {
    const linear = idealSolve([
      p([[[1, 0], 1], [[0, 1], 1], [[0, 0], -1]]),
      p([[[1, 0], 1], [[0, 1], -1]]),
    ]);
    expect(linear?.map((solution) => solution.map(String))).toEqual([["1/2", "1/2"]]);

    const quadratic = idealSolve([
      p([[[2, 0], 1], [[0, 0], -1]]),
      p([[[0, 1], 1], [[1, 0], -1]]),
    ]);
    expect(new Set(quadratic?.map((solution) => solution.map(String).join(",")))).toEqual(new Set(["1,1", "-1,-1"]));
    expect(idealSolve([p([[[2, 0], 1], [[0, 0], 1]]), p([[[0, 1], 1], [[1, 0], -1]])])).toBeNull();
    expect(idealSolve([])).toBeNull();
  });
});

describe("Symbolic IR conversion and handlers", () => {
  it("converts IR to MPoly and back without parsing", () => {
    const expr = app(ADD, [app(POW, [x, int(2)]), app(MUL, [int(2), x]), int(1)]);
    expectPoly(irToMPoly(expr, ["x"]), [[[2], 1], [[1], 2], [[0], 1]], 1);
    expectPoly(irToMPoly(rational(1, 2), ["x", "y"]), [[[0, 0], frac(1, 2)]]);
    expectPoly(irToMPoly(app(NEG, [x]), ["x", "y"]), [[[1, 0], -1]]);
    expectPoly(irToMPoly(app(SUB, [x, y]), ["x", "y"]), [[[1, 0], 1], [[0, 1], -1]]);
    expectPoly(irToMPoly(app(POW, [x, int(0)]), ["x"]), [[[0], 1]], 1);

    const roundTrip = mpolyToIR(irToMPoly(app(ADD, [app(POW, [x, int(2)]), y, int(-1)]), ["x", "y"]), [x, y]);
    expect(irToMPoly(roundTrip, ["x", "y"]).equals(p([[[2, 0], 1], [[0, 1], 1], [[0, 0], -1]]))).toBe(true);
  });

  it("rejects non-polynomial IR shapes", () => {
    expect(() => irToMPoly(sym("z"), ["x", "y"])).toThrow(ConversionError);
    expect(() => irToMPoly(app(POW, [x, int(-1)]), ["x"])).toThrow(ConversionError);
    expect(() => irToMPoly(app(POW, [x, y]), ["x", "y"])).toThrow(ConversionError);
    expect(() => irToMPoly(app(SIN, [x]), ["x"])).toThrow(ConversionError);
  });

  it("extracts variable and polynomial lists", () => {
    expect(extractVarList(list(x, y))).toEqual(["x", "y"]);
    expect(extractVarList(x)).toBeNull();
    expect(extractVarList(list(int(1)))).toBeNull();
    expect(extractPolyList(list(x, y), ["x", "y"])).toHaveLength(2);
    expect(extractPolyList(list(app(SIN, [x])), ["x"])).toBeNull();
  });

  it("handles Groebner, PolyReduce, and IdealSolve IR calls", () => {
    const equations = list(app(ADD, [x, y, int(-1)]), app(SUB, [x, y]));
    const variables = list(x, y);

    const groebnerResult = groebnerHandler(app(GROEBNER, [equations, variables]));
    expect(groebnerResult.kind).toBe("apply");
    expect(groebnerResult.kind === "apply" ? groebnerResult.head : null).toEqual(LIST);

    const reduceResult = polyReduceHandler(app(POLY_REDUCE, [app(POW, [x, int(2)]), list(app(SUB, [x, int(1)])), list(x)]));
    expect(reduceResult).toEqual(int(1));

    const solveResult = idealSolveHandler(app(IDEAL_SOLVE, [equations, variables]));
    expect(solveResult).toEqual(app(LIST, [
      app(LIST, [
        app(RULE, [x, rational(1, 2)]),
        app(RULE, [y, rational(1, 2)]),
      ]),
    ]));

    expect(groebnerHandler(app(GROEBNER, [variables]))).toEqual(app(GROEBNER, [variables]));
    expect(polyReduceHandler(app(POLY_REDUCE, [app(SIN, [x]), list(x), list(x)]))).toEqual(app(POLY_REDUCE, [app(SIN, [x]), list(x), list(x)]));
    expect(idealSolveHandler(app(IDEAL_SOLVE, [list(app(ADD, [app(POW, [x, int(2)]), int(1)])), list(x)]))).toEqual(app(IDEAL_SOLVE, [list(app(ADD, [app(POW, [x, int(2)]), int(1)])), list(x)]));
  });

  it("builds a handler table", () => {
    const table = buildMultivariateHandlerTable();
    expect(table.get("Groebner")).toBe(groebnerHandler);
    expect(table.get("PolyReduce")).toBe(polyReduceHandler);
    expect(table.get("IdealSolve")).toBe(idealSolveHandler);
  });
});
