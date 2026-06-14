import { describe, expect, it } from "vitest";
import { ADD, MUL, POW, SQRT, SUB, app, equals, int, rational, sym, type IRNode } from "@coding-adventures/symbolic-ir";
import {
  ALG_FACTOR,
  algFactorIr,
  algFactorHandler,
  buildAlgFactorHandlerTable,
  extractRadicalD,
  factorOverExtension,
  rationalSquareRoot,
  trySplitDepressedQuartic,
  trySplitQuadratic,
} from "../src/index";

function sqrtD(d: bigint | number) {
  return app(SQRT, [int(d)]);
}

function algFactor(poly: IRNode, radical: IRNode) {
  return app(ALG_FACTOR, [poly, radical]);
}

describe("rationalSquareRoot", () => {
  it("detects exact rational squares", () => {
    expect(rationalSquareRoot({ numer: 4n, denom: 1n })).toEqual({ numer: 2n, denom: 1n });
    expect(rationalSquareRoot({ numer: 1n, denom: 4n })).toEqual({ numer: 1n, denom: 2n });
    expect(rationalSquareRoot({ numer: 2n, denom: 1n })).toBeNull();
    expect(rationalSquareRoot({ numer: -1n, denom: 1n })).toBeNull();
  });
});

describe("quadratic extension factoring", () => {
  it("splits x^2 - d over Q[sqrt(d)]", () => {
    const result = trySplitQuadratic([-2, 0, 1], 2);
    expect(result).not.toBeNull();
    expect(result).toHaveLength(2);
    const radicals = result?.map((factor) => factor[0].radical.numer).sort();
    expect(radicals).toEqual([-1n, 1n]);
  });

  it("does not split wrong discriminants", () => {
    expect(trySplitQuadratic([2, 0, 1], 2)).toBeNull();
    expect(trySplitQuadratic([1, 1, 1], 2)).toBeNull();
  });

  it("splits x^4 + 1 over sqrt(2)", () => {
    const result = factorOverExtension([1, 0, 0, 0, 1], 2);
    expect(result).not.toBeNull();
    expect(result).toHaveLength(2);
    const radicals = result?.map((factor) => factor[1].radical.numer).sort();
    expect(radicals).toEqual([-1n, 1n]);
  });

  it("does not split x^4 + 1 over sqrt(3)", () => {
    expect(factorOverExtension([1, 0, 0, 0, 1], 3)).toBeNull();
  });

  it("checks depressed quartic shape", () => {
    expect(trySplitDepressedQuartic([1, 0, 0, 1, 1], 2)).toBeNull();
    expect(trySplitDepressedQuartic([2, 0, 0, 0, 1], 2)).toBeNull();
  });

  it("keeps rational factors when residuals split", () => {
    const result = factorOverExtension([2, -2, -1, 1], 2);
    expect(result).not.toBeNull();
    expect(result).toHaveLength(3);
    expect(result?.some((factor) => factor.length === 2 && factor[0].rational.numer === -1n)).toBe(true);
  });
});

describe("IR adapter", () => {
  it("extracts radical extensions", () => {
    expect(extractRadicalD(sqrtD(2))).toBe(2n);
    expect(extractRadicalD(sqrtD(4))).toBeNull();
    expect(extractRadicalD(int(2))).toBeNull();
  });

  it("factors a Pow polynomial", () => {
    const x = sym("x");
    const poly = app(SUB, [app(POW, [x, int(2)]), int(2)]);
    const result = algFactorIr(poly, sqrtD(2), x);
    expect(result?.kind).toBe("apply");
  });

  it("factors a nested multiplication polynomial", () => {
    const x = sym("x");
    const x4 = app(MUL, [x, x, x, x]);
    const result = algFactorIr(app(ADD, [x4, int(1)]), sqrtD(2), x);
    expect(result?.kind).toBe("apply");
  });

  it("returns null for non-polynomials", () => {
    const x = sym("x");
    expect(algFactorIr(app(sym("Sin"), [x]), sqrtD(2), x)).toBeNull();
  });
});

describe("AlgFactor handler", () => {
  it("registers a table entry under AlgFactor", () => {
    const table = buildAlgFactorHandlerTable();
    expect([...table.keys()]).toEqual(["AlgFactor"]);
    expect(table.get("AlgFactor")).toBe(algFactorHandler);
  });

  it("falls through unchanged for wrong arity", () => {
    const expr = app(ALG_FACTOR, [int(1), sqrtD(2), int(3)]);
    expect(algFactorHandler(expr)).toBe(expr);
  });

  it("falls through unchanged for non-Sqrt extensions", () => {
    const x = sym("x");
    const expr = algFactor(app(SUB, [app(POW, [x, int(2)]), int(2)]), int(2));
    expect(algFactorHandler(expr)).toBe(expr);
  });

  it("falls through unchanged for non-polynomial inputs", () => {
    const x = sym("x");
    const expr = algFactor(app(sym("Sin"), [x]), sqrtD(2));
    expect(algFactorHandler(expr)).toBe(expr);
  });

  it("falls through unchanged for irreducible polynomials", () => {
    const x = sym("x");
    const expr = algFactor(app(ADD, [app(POW, [x, int(2)]), int(1)]), sqrtD(2));
    expect(algFactorHandler(expr)).toBe(expr);
  });

  it("factors AlgFactor(x^2 - 2, Sqrt(2)) to product IR", () => {
    const x = sym("x");
    const expr = algFactor(app(SUB, [app(POW, [x, int(2)]), int(2)]), sqrtD(2));
    const result = algFactorHandler(expr);
    expect(result.kind).toBe("apply");
    expect(result.kind === "apply" && equals(result.head, MUL)).toBe(true);
  });

  it("clears rational polynomial denominators in the handler path", () => {
    const x = sym("x");
    const expr = algFactor(app(SUB, [app(MUL, [rational(1, 2), app(POW, [x, int(2)])]), int(1)]), sqrtD(2));
    const result = algFactorHandler(expr);
    expect(result.kind).toBe("apply");
    expect(result.kind === "apply" && equals(result.head, MUL)).toBe(true);
  });

  it("uses an evaluator callback before polynomial conversion", () => {
    const x = sym("x");
    const y = sym("y");
    const normalized = app(SUB, [app(POW, [x, int(2)]), int(2)]);
    const expr = algFactor(y, sqrtD(2));
    const result = algFactorHandler(expr, {
      eval(node) {
        return equals(node, y) ? normalized : node;
      },
    });
    expect(result.kind).toBe("apply");
    expect(result.kind === "apply" && equals(result.head, MUL)).toBe(true);
  });
});
