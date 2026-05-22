import { describe, expect, it } from "vitest";
import {
  ADD,
  DIV,
  EXP,
  MUL,
  NEG,
  POW,
  PRODUCT,
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
  evaluateProduct,
  evaluateProductExpr,
  evaluateSum,
  faulhaberIr,
  geometricSumIr,
  polySumIr,
  rationalValue,
  trySpecialInfinite,
  type RationalValue,
} from "../src/index";

function evalNode(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const head = node.head;
  const args = node.args.map(evalNode);
  if (head.kind !== "symbol") return app(head, args);
  const out = (() => {
    switch (head.name) {
      case ADD.name:
        return fold(args, { numer: 0n, denom: 1n }, addR);
      case SUB.name:
        return args.length === 2 ? binary(args[0], args[1], subR) : undefined;
      case MUL.name:
        return fold(args, { numer: 1n, denom: 1n }, mulR);
      case DIV.name:
        return args.length === 2 ? binary(args[0], args[1], divR) : undefined;
      case POW.name:
        return powNode(args[0], args[1]);
      case NEG.name: {
        const value = rationalValue(args[0]);
        return value === undefined ? undefined : rationalToIr({ numer: -value.numer, denom: value.denom });
      }
      default:
        return undefined;
    }
  })();
  return out ?? app(head, args);
}

function fold(args: readonly IRNode[], init: RationalValue, op: (a: RationalValue, b: RationalValue) => RationalValue): IRNode | undefined {
  let acc = init;
  for (const arg of args) {
    const value = rationalValue(arg);
    if (value === undefined) return undefined;
    acc = op(acc, value);
  }
  return rationalToIr(acc);
}

function binary(a: IRNode, b: IRNode, op: (a: RationalValue, b: RationalValue) => RationalValue): IRNode | undefined {
  const av = rationalValue(a);
  const bv = rationalValue(b);
  return av === undefined || bv === undefined ? undefined : rationalToIr(op(av, bv));
}

function powNode(a: IRNode, b: IRNode): IRNode | undefined {
  const base = rationalValue(a);
  if (base === undefined || b.kind !== "integer" || b.value < 0n) return undefined;
  let out = { numer: 1n, denom: 1n };
  for (let i = 0n; i < b.value; i += 1n) out = mulR(out, base);
  return rationalToIr(out);
}

function rationalToIr(value: RationalValue): IRNode {
  return value.denom === 1n ? int(value.numer) : rational(value.numer, value.denom);
}

function addR(a: RationalValue, b: RationalValue): RationalValue {
  return reduce({ numer: a.numer * b.denom + b.numer * a.denom, denom: a.denom * b.denom });
}

function subR(a: RationalValue, b: RationalValue): RationalValue {
  return reduce({ numer: a.numer * b.denom - b.numer * a.denom, denom: a.denom * b.denom });
}

function mulR(a: RationalValue, b: RationalValue): RationalValue {
  return reduce({ numer: a.numer * b.numer, denom: a.denom * b.denom });
}

function divR(a: RationalValue, b: RationalValue): RationalValue {
  return reduce({ numer: a.numer * b.denom, denom: a.denom * b.numer });
}

function reduce(value: RationalValue): RationalValue {
  let n = value.numer;
  let d = value.denom;
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  const g = gcd(n < 0n ? -n : n, d);
  return { numer: n / g, denom: d / g };
}

function gcd(a: bigint, b: bigint): bigint {
  let x = a;
  let y = b;
  while (y !== 0n) {
    const t = y;
    y = x % y;
    x = t;
  }
  return x === 0n ? 1n : x;
}

describe("summation", () => {
  it("evaluates constant and geometric sums", () => {
    const k = sym("k");
    expect(evaluateSum(int(5), k, int(1), int(10), evalNode)).toEqual(int(50));
    expect(evaluateSum(app(POW, [rational(1, 2), k]), k, int(0), sym("%inf"), evalNode)).toEqual(int(2));
  });

  it("builds finite and infinite geometric sums", () => {
    expect(evalNode(geometricSumIr(int(1), int(3), int(0), int(3), false))).toEqual(int(40));
    expect(evalNode(geometricSumIr(int(1), rational(1, 4), int(2), undefined, true))).toEqual(rational(1, 12));
  });

  it("covers Faulhaber and polynomial power sums", () => {
    const expected = [4, 10, 30, 100, 354, 1300];
    for (const [m, value] of expected.entries()) {
      expect(evalNode(faulhaberIr(m, int(4)) ?? sym("bad"))).toEqual(int(value));
    }
    expect(faulhaberIr(6, int(4))).toBeUndefined();
    expect(evalNode(polySumIr(2, { numer: 1n, denom: 1n }, 1n, int(4)) ?? sym("bad"))).toEqual(int(30));
    expect(evalNode(polySumIr(0, { numer: 1n, denom: 1n }, 0n, int(4)) ?? sym("bad"))).toEqual(int(5));
    expect(evaluateSum(app(MUL, [int(3), sym("k")]), sym("k"), int(1), int(4), evalNode)).toEqual(int(30));
  });

  it("recognises classic infinite series", () => {
    const k = sym("k");
    const x = sym("x");
    expect(trySpecialInfinite(app(DIV, [int(1), app(POW, [k, int(2)])]), k, int(1))).toEqual(
      app(DIV, [app(POW, [sym("%pi"), int(2)]), int(6)]),
    );
    const gamma = app(GAMMA_FUNC, [app(ADD, [k, int(1)])]);
    expect(trySpecialInfinite(app(DIV, [int(1), gamma]), k, int(0))).toEqual(sym("%e"));
    expect(trySpecialInfinite(app(DIV, [app(POW, [x, k]), gamma]), k, int(0))).toEqual(app(EXP, [x]));
  });

  it("evaluates products and preserves fallback nodes", () => {
    const k = sym("k");
    const n = sym("n");
    expect(evaluateProduct(k, k, int(1), n, evalNode)).toEqual(app(GAMMA_FUNC, [app(ADD, [n, int(1)])]));
    expect(evaluateProduct(int(2), k, int(0), int(4), evalNode)).toEqual(int(32));
    expect(evaluateProduct(app(MUL, [int(2), k]), k, int(1), n, evalNode)).toEqual(
      app(MUL, [app(POW, [int(2), n]), app(GAMMA_FUNC, [app(ADD, [n, int(1)])])]),
    );
    expect(evaluateProductExpr(k, k, int(0), n)).toBeUndefined();
    const fallback = evaluateProduct(app(POW, [k, int(3)]), k, int(1), n, evalNode);
    expect(fallback.kind).toBe("apply");
    expect(fallback.kind === "apply" ? fallback.head : undefined).toEqual(PRODUCT);
  });

  it("falls back for unknown sums", () => {
    const k = sym("k");
    const out = evaluateSum(app(sym("Sin"), [k]), k, int(1), sym("n"), evalNode);
    expect(out.kind).toBe("apply");
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 39: Telescoping sums.
//
// ∑_{k=lo}^{hi} [g(k+1) − g(k)] = g(hi+1) − g(lo)
// ∑_{k=lo}^{hi} [g(k) − g(k+1)] = g(lo) − g(hi+1)
//
// Detection is purely structural: substitute k → k+1 in one half of the SUB
// shape and compare to the other half after evalNode normalisation.
// ---------------------------------------------------------------------------

describe("summation: Phase 39 telescoping", () => {
  it("standard (k+1)² − k² telescope at concrete bounds", () => {
    const k = sym("k");
    // ∑_{k=1}^{4} [(k+1)² − k²] = 5² − 1² = 24
    const kPlusOneSq = app(POW, [app(ADD, [k, int(1)]), int(2)]);
    const kSq = app(POW, [k, int(2)]);
    const f = app(SUB, [kPlusOneSq, kSq]);
    expect(evaluateSum(f, k, int(1), int(4), evalNode)).toEqual(int(24));
  });

  it("antisymmetric k² − (k+1)² orientation", () => {
    const k = sym("k");
    // ∑_{k=1}^{3} [k² − (k+1)²] = 1² − 4² = −15
    const kSq = app(POW, [k, int(2)]);
    const kPlusOneSq = app(POW, [app(ADD, [k, int(1)]), int(2)]);
    const f = app(SUB, [kSq, kPlusOneSq]);
    expect(evaluateSum(f, k, int(1), int(3), evalNode)).toEqual(int(-15));
  });

  it("linear g(k) = k telescope (f ≡ 1 counts terms)", () => {
    const k = sym("k");
    // ∑_{k=1}^{10} [(k+1) − k] = g(11) − g(1) = 11 − 1 = 10
    const f = app(SUB, [app(ADD, [k, int(1)]), k]);
    expect(evaluateSum(f, k, int(1), int(10), evalNode)).toEqual(int(10));
  });

  it("g(k) = k + 5 (constant offset is preserved through substitution)", () => {
    const k = sym("k");
    // ∑_{k=1}^{5} [(k + 6) − (k + 5)] = g(6) − g(1) = 11 − 6 = 5
    const gAtKPlus1 = app(ADD, [app(ADD, [k, int(1)]), int(5)]);
    const gAtK = app(ADD, [k, int(5)]);
    const f = app(SUB, [gAtKPlus1, gAtK]);
    expect(evaluateSum(f, k, int(1), int(5), evalNode)).toEqual(int(5));
  });

  it("non-telescoping k² − k falls through to numeric/Faulhaber", () => {
    const k = sym("k");
    // ∑_{k=1}^{3} [k² − k] = (1−1)+(4−2)+(9−3) = 0+2+6 = 8
    const f = app(SUB, [app(POW, [k, int(2)]), k]);
    expect(evaluateSum(f, k, int(1), int(3), evalNode)).toEqual(int(8));
  });

  it("constant-difference summand routes through step 1 (constant)", () => {
    const k = sym("k");
    // ∑_{k=1}^{10} [5 − 3] = ∑ 2 = 20
    const f = app(SUB, [int(5), int(3)]);
    expect(evaluateSum(f, k, int(1), int(10), evalNode)).toEqual(int(20));
  });

  it("symbolic upper bound: result is non-unevaluated", () => {
    const k = sym("k");
    const n = sym("n");
    const f = app(SUB, [app(ADD, [k, int(1)]), k]);
    const out = evaluateSum(f, k, int(1), n, evalNode);
    // Should not stay as a Sum(...) node.
    expect(out.kind === "apply" && out.head === SUM).toBe(false);
  });

  it("infinite upper bound with non-vanishing g falls through (Phase 41 guard)", () => {
    // g(k) = k grows at infinity, so the Phase 41 limit check refuses.
    const k = sym("k");
    const f = app(SUB, [app(ADD, [k, int(1)]), k]);
    const out = evaluateSum(f, k, int(0), sym("%inf"), evalNode);
    expect(out.kind).toBe("apply");
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 41+42: limit-aware infinite telescope.
//
// When `hi = %inf` AND `g(k)` provably vanishes at infinity, the
// dispatcher emits −g(lo) (standard orientation) or g(lo) (antisymmetric).
//
// The narrow vanishing-at-infinity recogniser handles:
//   - Phase 41 fast path: Div(constant, positive-degree-polynomial-in-k)
//   - Phase 42 widening: Div(P, Q) with both polynomials and deg(P)<deg(Q)
//
// Anything else (transcendental, improper rational, non-Div) falls
// through to the unevaluated Sum(...).
// ---------------------------------------------------------------------------

describe("summation: Phase 41+42 limit-aware infinite telescope", () => {
  it("∑_{k=1}^∞ [1/k − 1/(k+1)] = 1 (Phase 41 antisymmetric)", () => {
    const k = sym("k");
    const f = app(SUB, [
      app(DIV, [int(1), k]),
      app(DIV, [int(1), app(ADD, [k, int(1)])]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(int(1));
  });

  it("∑_{k=1}^∞ [1/(k+1) − 1/k] = −1 (Phase 41 standard orientation)", () => {
    const k = sym("k");
    const f = app(SUB, [
      app(DIV, [int(1), app(ADD, [k, int(1)])]),
      app(DIV, [int(1), k]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(int(-1));
  });

  it("higher starting index ∑_{k=2}^∞ [1/k − 1/(k+1)] = 1/2", () => {
    const k = sym("k");
    const f = app(SUB, [
      app(DIV, [int(1), k]),
      app(DIV, [int(1), app(ADD, [k, int(1)])]),
    ]);
    expect(evaluateSum(f, k, int(2), sym("%inf"), evalNode)).toEqual(rational(1, 2));
  });

  it("quadratic denominator ∑_{k=1}^∞ [1/k² − 1/(k+1)²] = 1", () => {
    const k = sym("k");
    const kSq = app(POW, [k, int(2)]);
    const kPlus1Sq = app(POW, [app(ADD, [k, int(1)]), int(2)]);
    const f = app(SUB, [app(DIV, [int(1), kSq]), app(DIV, [int(1), kPlus1Sq])]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(int(1));
  });

  it("Phase 42 proper rational ∑_{k=1}^∞ [k/(k²+1) − (k+1)/((k+1)²+1)] = 1/2", () => {
    const k = sym("k");
    const gK = app(DIV, [k, app(ADD, [app(POW, [k, int(2)]), int(1)])]);
    const kp1 = app(ADD, [k, int(1)]);
    const gKp1 = app(DIV, [kp1, app(ADD, [app(POW, [kp1, int(2)]), int(1)])]);
    const f = app(SUB, [gK, gKp1]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(rational(1, 2));
  });

  it("Phase 42 improper rational ∑_{k=1}^∞ [k/(k+1) − (k+1)/(k+2)] falls through", () => {
    // g(k) = k/(k+1) has equal degrees, limit is 1 (not 0).
    const k = sym("k");
    const gK = app(DIV, [k, app(ADD, [k, int(1)])]);
    const gKp1 = app(DIV, [app(ADD, [k, int(1)]), app(ADD, [k, int(2)])]);
    const f = app(SUB, [gK, gKp1]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });

  it("transcendental numerator ∑_{k=1}^∞ [sin(k)/k² − sin(k+1)/(k+1)²] falls through", () => {
    // sin(k)/k² is non-polynomial; conservative refuse.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kPlus1]);
    const f = app(SUB, [
      app(DIV, [sinK, app(POW, [k, int(2)])]),
      app(DIV, [sinKp1, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 43: transcendental vanishing-at-infinity.
//
// Extends Phase 41/42 to accept exponentially diverging shapes
// (Exp(h(k)), Pow(b, h(k)) with |b| > 1, and Mul of such factors).
// Sign-aware leading-coefficient check refuses `exp(-k)`, `2^(-k)`,
// and the Mul / NEG-wrapped variants of those (these vanish, not diverge).
// ---------------------------------------------------------------------------

describe("summation: Phase 43 transcendental vanishing-at-infinity", () => {
  it("∑_{k=0}^∞ [1/2^k − 1/2^(k+1)] = 1 (Pow(2, k) diverges)", () => {
    const k = sym("k");
    const f = app(SUB, [
      app(DIV, [int(1), app(POW, [int(2), k])]),
      app(DIV, [int(1), app(POW, [int(2), app(ADD, [k, int(1)])])]),
    ]);
    expect(evaluateSum(f, k, int(0), sym("%inf"), evalNode)).toEqual(int(1));
  });

  it("∑_{k=1}^∞ [1/3^k − 1/3^(k+1)] = 1/3", () => {
    const k = sym("k");
    const f = app(SUB, [
      app(DIV, [int(1), app(POW, [int(3), k])]),
      app(DIV, [int(1), app(POW, [int(3), app(ADD, [k, int(1)])])]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(rational(1, 3));
  });

  it("base 1/2 falls through (Pow(1/2, k) → 0, not ∞)", () => {
    const k = sym("k");
    const f = app(SUB, [
      app(DIV, [int(1), app(POW, [rational(1, 2), k])]),
      app(DIV, [int(1), app(POW, [rational(1, 2), app(ADD, [k, int(1)])])]),
    ]);
    const out = evaluateSum(f, k, int(0), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });

  it("Mul of polynomial × exponential ∑ 1/(k·2^k) − 1/((k+1)·2^(k+1)) = 1/2", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const f = app(SUB, [
      app(DIV, [int(1), app(MUL, [k, app(POW, [int(2), k])])]),
      app(DIV, [int(1), app(MUL, [kp1, app(POW, [int(2), kp1])])]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(rational(1, 2));
  });

  it("regression: Pow(2, Mul(-1, k)) = 2^(-k) does NOT diverge — refuse", () => {
    // Sign-aware leading-coefficient check must catch this.
    const k = sym("k");
    const negK = app(MUL, [int(-1), k]);
    const negKp1 = app(MUL, [int(-1), app(ADD, [k, int(1)])]);
    const f = app(SUB, [
      app(DIV, [int(1), app(POW, [int(2), negK])]),
      app(DIV, [int(1), app(POW, [int(2), negKp1])]),
    ]);
    const out = evaluateSum(f, k, int(0), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });

  it("regression: Pow(2, Neg(k)) does NOT diverge — refuse (NEG wrapper form)", () => {
    const k = sym("k");
    const f = app(SUB, [
      app(DIV, [int(1), app(POW, [int(2), app(NEG, [k])])]),
      app(DIV, [int(1), app(POW, [int(2), app(NEG, [app(ADD, [k, int(1)])])])]),
    ]);
    const out = evaluateSum(f, k, int(0), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});
