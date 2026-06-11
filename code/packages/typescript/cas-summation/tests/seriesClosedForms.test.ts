/**
 * Tests for the Track I2 canonical infinite-series recogniser.
 *
 * Mirrors
 * ``code/packages/python/cas-summation/tests/test_series_closed_forms.py``
 * (Track I1, PR #5382).  Constructs each summand IR by hand, calls
 * ``tryClosedFormSeries`` directly to verify the structural recogniser,
 * numerically evaluates the returned IR via a tiny recursive helper,
 * and compares to the expected mathematical value.  End-to-end smoke
 * tests through ``evaluateSum`` confirm the dispatcher wiring routes
 * ``hi = %inf`` cases through the new path without disturbing the
 * pre-existing finite Gosper / Faulhaber routes.
 */

import { describe, expect, it } from "vitest";
import {
  ADD,
  COS,
  COSH,
  DIV,
  EXP,
  LOG,
  MUL,
  NEG,
  POW,
  SIN,
  SINH,
  SUB,
  SUM,
  app,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  GAMMA_FUNC,
  bernoulliRational,
  evaluateSum,
  tryClosedFormSeries,
} from "../src/index";

const k = sym("k");
const x = sym("x");
const INF = sym("%inf");

// ---------------------------------------------------------------------------
// Tiny numeric evaluator — folds Integer/Rational arithmetic and the few
// transcendental heads we need (Pi, %e, log, exp).  Used only by the test
// assertions; production code never needs floats.
// ---------------------------------------------------------------------------

function numeric(node: IRNode): number {
  if (node.kind === "integer") return Number(node.value);
  if (node.kind === "rational") return Number(node.numer) / Number(node.denom);
  if (node.kind === "float") return node.value;
  if (node.kind === "symbol") {
    if (node.name === "%pi") return Math.PI;
    if (node.name === "%e") return Math.E;
    throw new Error(`Free symbol ${node.name} in numeric eval`);
  }
  if (node.kind !== "apply") throw new Error(`Unsupported node kind ${node.kind}`);
  const head = node.head;
  if (head.kind !== "symbol") throw new Error("Unsupported head kind");
  const args = node.args;
  switch (head.name) {
    case "Add":
      return args.reduce((acc: number, a: IRNode) => acc + numeric(a), 0);
    case "Sub":
      return numeric(args[0]) - numeric(args[1]);
    case "Mul":
      return args.reduce((acc: number, a: IRNode) => acc * numeric(a), 1);
    case "Div":
      return numeric(args[0]) / numeric(args[1]);
    case "Neg":
      return -numeric(args[0]);
    case "Pow":
      return Math.pow(numeric(args[0]), numeric(args[1]));
    case "Log":
      return Math.log(numeric(args[0]));
    case "Exp":
      return Math.exp(numeric(args[0]));
    default:
      throw new Error(`Unsupported head ${head.name} in numeric eval`);
  }
}

// ---------------------------------------------------------------------------
// IR-shape helpers — match the parser's emitted forms for the summands.
// ---------------------------------------------------------------------------

function invKPow(m: bigint): IRNode {
  if (m === 1n) return app(DIV, [int(1), k]);
  return app(DIV, [int(1), app(POW, [k, int(m)])]);
}

function altInvKPow(m: bigint): IRNode {
  const negOnePow = app(POW, [int(-1n), app(SUB, [k, int(1)])]);
  if (m === 1n) return app(DIV, [negOnePow, k]);
  return app(DIV, [negOnePow, app(POW, [k, int(m)])]);
}

function invFactorial(): IRNode {
  const gamma = app(GAMMA_FUNC, [app(ADD, [k, int(1)])]);
  return app(DIV, [int(1), gamma]);
}

function xkOverFactorial(): IRNode {
  const gamma = app(GAMMA_FUNC, [app(ADD, [k, int(1)])]);
  return app(DIV, [app(POW, [x, k]), gamma]);
}

function gammaLin(slope: bigint, intercept: bigint): IRNode {
  return app(GAMMA_FUNC, [
    app(ADD, [app(MUL, [int(slope), k]), int(intercept + 1n)]),
  ]);
}

function powXLin(slope: bigint, intercept: bigint): IRNode {
  const exp =
    intercept === 0n
      ? app(MUL, [int(slope), k])
      : app(ADD, [app(MUL, [int(slope), k]), int(intercept)]);
  return app(POW, [x, exp]);
}

function cosSummand(): IRNode {
  const sign = app(POW, [int(-1n), k]);
  const body = app(DIV, [powXLin(2n, 0n), gammaLin(2n, 0n)]);
  return app(MUL, [sign, body]);
}

function sinSummand(): IRNode {
  const sign = app(POW, [int(-1n), k]);
  const body = app(DIV, [powXLin(2n, 1n), gammaLin(2n, 1n)]);
  return app(MUL, [sign, body]);
}

function coshSummand(): IRNode {
  return app(DIV, [powXLin(2n, 0n), gammaLin(2n, 0n)]);
}

function sinhSummand(): IRNode {
  return app(DIV, [powXLin(2n, 1n), gammaLin(2n, 1n)]);
}

// ---------------------------------------------------------------------------
// Minimal identity eval (mirrors the Python ``_StubVM`` minus the fold).
// We only need the wired dispatcher to behave; closed-form IR returned by
// the recogniser does not require simplification for the tests.
// ---------------------------------------------------------------------------

function identityEval(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  return app(node.head, node.args.map(identityEval));
}

describe("bernoulli helper", () => {
  it("matches known values (Knuth convention B_1 = -1/2)", () => {
    expect(bernoulliRational(0)).toEqual({ n: 1n, d: 1n });
    expect(bernoulliRational(1)).toEqual({ n: -1n, d: 2n });
    expect(bernoulliRational(2)).toEqual({ n: 1n, d: 6n });
    expect(bernoulliRational(3)).toEqual({ n: 0n, d: 1n });
    expect(bernoulliRational(4)).toEqual({ n: -1n, d: 30n });
    expect(bernoulliRational(6)).toEqual({ n: 1n, d: 42n });
    expect(bernoulliRational(8)).toEqual({ n: -1n, d: 30n });
    expect(bernoulliRational(10)).toEqual({ n: 5n, d: 66n });
    expect(bernoulliRational(12)).toEqual({ n: -691n, d: 2730n });
  });

  it("odd indices ≥ 3 are zero", () => {
    for (const n of [3, 5, 7, 9, 11]) {
      expect(bernoulliRational(n)).toEqual({ n: 0n, d: 1n });
    }
  });
});

describe("zeta(2m) family", () => {
  const cases: Array<[bigint, number]> = [
    [2n, Math.PI ** 2 / 6],
    [4n, Math.PI ** 4 / 90],
    [6n, Math.PI ** 6 / 945],
    [8n, Math.PI ** 8 / 9450],
    [10n, Math.PI ** 10 / 93555],
    [12n, (691 * Math.PI ** 12) / 638512875],
  ];
  for (const [twoM, expected] of cases) {
    it(`recognises Σ 1/k^${twoM}`, () => {
      const result = tryClosedFormSeries(invKPow(twoM), k, int(1), INF);
      expect(result).not.toBeUndefined();
      expect(numeric(result as IRNode)).toBeCloseTo(expected, 10);
    });
  }

  it("Σ 1/k^3 (odd zeta) falls through", () => {
    expect(tryClosedFormSeries(invKPow(3n), k, int(1), INF)).toBeUndefined();
  });

  it("Σ 1/k^14 (past m=6) falls through", () => {
    expect(tryClosedFormSeries(invKPow(14n), k, int(1), INF)).toBeUndefined();
  });

  it("wrong lo=2 falls through", () => {
    expect(tryClosedFormSeries(invKPow(2n), k, int(2), INF)).toBeUndefined();
  });
});

describe("eta family", () => {
  it("Mercator: Σ (-1)^(k-1)/k → log(2)", () => {
    const result = tryClosedFormSeries(altInvKPow(1n), k, int(1), INF);
    expect(result).not.toBeUndefined();
    const r = result as IRNode;
    expect(r.kind).toBe("apply");
    if (r.kind === "apply") {
      expect(r.head).toEqual(LOG);
      expect(r.args[0]).toEqual(int(2));
    }
    expect(numeric(r)).toBeCloseTo(Math.log(2), 10);
  });

  const cases: Array<[bigint, number]> = [
    [2n, Math.PI ** 2 / 12],
    [4n, (7 * Math.PI ** 4) / 720],
    [6n, (31 * Math.PI ** 6) / 30240],
  ];
  for (const [twoM, expected] of cases) {
    it(`recognises Σ (-1)^(k-1)/k^${twoM}`, () => {
      const result = tryClosedFormSeries(altInvKPow(twoM), k, int(1), INF);
      expect(result).not.toBeUndefined();
      expect(numeric(result as IRNode)).toBeCloseTo(expected, 10);
    });
  }
});

describe("factorial-based series", () => {
  it("Σ_{k=0}^∞ 1/k! = %e", () => {
    const result = tryClosedFormSeries(invFactorial(), k, int(0), INF);
    expect(result).not.toBeUndefined();
    const r = result as IRNode;
    expect(r.kind).toBe("symbol");
    if (r.kind === "symbol") {
      expect(r.name).toBe("%e");
    }
  });

  it("Σ_{k=0}^∞ x^k/k! = exp(x)", () => {
    const result = tryClosedFormSeries(xkOverFactorial(), k, int(0), INF);
    expect(result).not.toBeUndefined();
    const r = result as IRNode;
    expect(r.kind).toBe("apply");
    if (r.kind === "apply") {
      expect(r.head).toEqual(EXP);
      expect(r.args[0]).toEqual(x);
    }
  });

  it("Σ (-1)^k · x^(2k)/(2k)! = cos(x)", () => {
    const result = tryClosedFormSeries(cosSummand(), k, int(0), INF);
    expect(result).not.toBeUndefined();
    const r = result as IRNode;
    expect(r.kind).toBe("apply");
    if (r.kind === "apply") {
      expect(r.head).toEqual(COS);
      expect(r.args[0]).toEqual(x);
    }
  });

  it("Σ (-1)^k · x^(2k+1)/(2k+1)! = sin(x)", () => {
    const result = tryClosedFormSeries(sinSummand(), k, int(0), INF);
    expect(result).not.toBeUndefined();
    const r = result as IRNode;
    expect(r.kind).toBe("apply");
    if (r.kind === "apply") {
      expect(r.head).toEqual(SIN);
      expect(r.args[0]).toEqual(x);
    }
  });

  it("Σ x^(2k)/(2k)! = cosh(x)", () => {
    const result = tryClosedFormSeries(coshSummand(), k, int(0), INF);
    expect(result).not.toBeUndefined();
    const r = result as IRNode;
    expect(r.kind).toBe("apply");
    if (r.kind === "apply") {
      expect(r.head).toEqual(COSH);
      expect(r.args[0]).toEqual(x);
    }
  });

  it("Σ x^(2k+1)/(2k+1)! = sinh(x)", () => {
    const result = tryClosedFormSeries(sinhSummand(), k, int(0), INF);
    expect(result).not.toBeUndefined();
    const r = result as IRNode;
    expect(r.kind).toBe("apply");
    if (r.kind === "apply") {
      expect(r.head).toEqual(SINH);
      expect(r.args[0]).toEqual(x);
    }
  });

  it("wrong lo=1 for factorial falls through", () => {
    expect(tryClosedFormSeries(invFactorial(), k, int(1), INF)).toBeUndefined();
  });
});

describe("fall-through cases", () => {
  it("Σ sin(k) (not in table) returns undefined", () => {
    const f = app(SIN, [k]);
    expect(tryClosedFormSeries(f, k, int(1), INF)).toBeUndefined();
  });

  it("finite hi → undefined (Faulhaber/Gosper handles it)", () => {
    expect(tryClosedFormSeries(invKPow(2n), k, int(1), int(100))).toBeUndefined();
  });

  it("symbolic x = Neg(y) in exp series still matches", () => {
    const y = sym("y");
    const gamma = app(GAMMA_FUNC, [app(ADD, [k, int(1)])]);
    const negY = app(NEG, [y]);
    const f = app(DIV, [app(POW, [negY, k]), gamma]);
    const result = tryClosedFormSeries(f, k, int(0), INF);
    expect(result).not.toBeUndefined();
    const r = result as IRNode;
    expect(r.kind).toBe("apply");
    if (r.kind === "apply") {
      expect(r.head).toEqual(EXP);
    }
  });
});

describe("dispatcher integration", () => {
  it("evaluateSum(1/k^6, k, 1, inf) → π^6/945", () => {
    const result = evaluateSum(invKPow(6n), k, int(1), INF, identityEval);
    expect(numeric(result)).toBeCloseTo(Math.PI ** 6 / 945, 10);
  });

  it("evaluateSum((-1)^(k-1)/k, k, 1, inf) → log(2) head", () => {
    const result = evaluateSum(altInvKPow(1n), k, int(1), INF, identityEval);
    expect(result.kind).toBe("apply");
    if (result.kind === "apply") expect(result.head).toEqual(LOG);
  });

  it("evaluateSum(cos summand, k, 0, inf) → Cos(x)", () => {
    const result = evaluateSum(cosSummand(), k, int(0), INF, identityEval);
    expect(result.kind).toBe("apply");
    if (result.kind === "apply") {
      expect(result.head).toEqual(COS);
      expect(result.args[0]).toEqual(x);
    }
  });

  it("evaluateSum(1/k^2, k, 1, 100) does NOT use I2 path (finite)", () => {
    // Provide a folding eval so the small-range numeric handler can
    // accumulate rationals; otherwise dispatcher falls through to the
    // unevaluated Sum.  We only need Div folding for ``1/k^m`` terms
    // and a Pow folder for the denominator.
    const foldEval = (node: IRNode): IRNode => {
      if (node.kind !== "apply") return node;
      const args = node.args.map(foldEval);
      const head = node.head;
      if (head.kind === "symbol") {
        if (head.name === "Pow" && args[0].kind === "integer" && args[1].kind === "integer" && args[1].value >= 0n) {
          let acc = 1n;
          for (let i = 0n; i < args[1].value; i++) acc *= args[0].value;
          return int(acc);
        }
        if (head.name === "Div" && args[0].kind === "integer" && args[1].kind === "integer" && args[1].value !== 0n) {
          if (args[0].value % args[1].value === 0n) return int(args[0].value / args[1].value);
          return rational(args[0].value, args[1].value);
        }
      }
      return app(head, args);
    };
    const result = evaluateSum(invKPow(2n), k, int(1), int(100), foldEval);
    // The small-range numeric handler returns an exact rational.
    expect(result.kind === "rational" || result.kind === "integer").toBe(true);
    let expected = 0;
    for (let i = 1; i <= 100; i++) expected += 1 / (i * i);
    expect(numeric(result)).toBeCloseTo(expected, 9);
  });

  it("Σ sin(k) (unrecognised) → unevaluated Sum", () => {
    const f = app(SIN, [k]);
    const result = evaluateSum(f, k, int(1), INF, identityEval);
    expect(result.kind).toBe("apply");
    if (result.kind === "apply") expect(result.head).toEqual(SUM);
  });
});
