import { describe, expect, it } from "vitest";
import {
  ADD,
  DIV,
  EXP,
  LOG,
  MUL,
  NEG,
  POW,
  PRODUCT,
  SQRT,
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

  it("transcendental numerator ∑_{k=1}^∞ [sin(k)/k² − sin(k+1)/(k+1)²] closes via Phase 49", () => {
    // Phase 49 (bounded-numerator widening) recognises |sin(k)| ≤ 1 +
    // k² → ∞, so the quotient vanishes and the antisymmetric telescope
    // closes to g(1) = sin(1)/1² = sin(1).
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kPlus1]);
    const f = app(SUB, [
      app(DIV, [sinK, app(POW, [k, int(2)])]),
      app(DIV, [sinKp1, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
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

// ---------------------------------------------------------------------------
// Phase 44: Log divergence in vanishing-at-infinity recogniser.
//
// Extends Phase 43 to accept `Log(h(k))` where h(k) → +∞ (so log(h) → +∞).
// Sign-aware guards:
//   - Polynomial inner: positive leading coefficient required.
//   - Exp inner: always positive; defer.
//   - Pow inner: base > 1 strictly required (not just |b| > 1).
//
// The telescope detector compares `g(k+1)` (via k→k+1 substitution in g)
// against the supplied `g_kp1`.  The stub VM doesn't canonicalise
// `Add(Add(k,1), 1) ↔ Add(k, 2)`, so we build `g_kp1` via substitution
// from `g_k` so the structural `==` comparison succeeds.
// ---------------------------------------------------------------------------

describe("summation: Phase 44 Log divergence", () => {
  // Helper: substitute k → k+1 in `node` to build the shifted half.
  function substitute(node: IRNode, from: IRNode, to: IRNode): IRNode {
    if (node.kind === "apply") {
      return app(node.head, node.args.map((a) => substitute(a, from, to)));
    }
    if (node.kind === "symbol" && from.kind === "symbol" && node.name === from.name) {
      return to;
    }
    return node;
  }

  it("Log(k+1) recognised → telescope closes", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const gK = app(DIV, [int(1), app(LOG, [kp1])]);
    const gKp1 = substitute(gK, k, kp1);
    const f = app(SUB, [gK, gKp1]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // Must not stay as Sum(...) — closed form will be a symbolic
    // 1/log(2) expression that the stub VM can't fold further.
    expect(out.kind === "apply" && out.head === SUM).toBe(false);
  });

  it("Log(2^k) recognised via Phase 43 Pow delegation", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const gK = app(DIV, [int(1), app(LOG, [app(POW, [int(2), k])])]);
    const gKp1 = substitute(gK, k, kp1);
    const f = app(SUB, [gK, gKp1]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" && out.head === SUM).toBe(false);
  });

  it("regression: Log(Pow(-2, k)) refused — negative base", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const gK = app(DIV, [int(1), app(LOG, [app(POW, [int(-2), k])])]);
    const gKp1 = substitute(gK, k, kp1);
    const f = app(SUB, [gK, gKp1]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });

  it("regression: Log(Mul(-1, k)) refused — negative leading polynomial", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const negK = app(MUL, [int(-1), k]);
    const gK = app(DIV, [int(1), app(LOG, [negK])]);
    const gKp1 = substitute(gK, k, kp1);
    const f = app(SUB, [gK, gKp1]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 40+46 (TypeScript port): Add-with-negation telescope normaliser.
//
// Ports the Python helpers `_extract_negation` and
// `_normalise_add_neg_to_sub` so that a user-written summand in
// `Add(g(k+1), Neg(g(k)))` or `Add(g(k+1), Div(-c, d))` form is
// rewritten to the canonical `Sub` shape before the Phase 39 / 41
// telescope detectors run.
//
// On the Python side this also feeds the symbolic-vm Apart-retry path,
// but `cas-summation` (TypeScript) doesn't depend on an Apart
// implementation — the value here is purely letting the telescope
// detector match more shapes the user might write directly.
// ---------------------------------------------------------------------------

describe("summation: Phase 40+46 Add-with-negation normaliser", () => {
  it("Add(g(k+1), Neg(g(k))) is treated as Sub(g(k+1), g(k)) — standard", () => {
    // ∑_{k=1}^∞ [1/(k+1) + Neg(1/k)] = -1 (same as the canonical
    // Phase 41 standard-orientation case, just spelled differently).
    const k = sym("k");
    const f = app(ADD, [
      app(DIV, [int(1), app(ADD, [k, int(1)])]),
      app(NEG, [app(DIV, [int(1), k])]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(int(-1));
  });

  it("Add(Neg(g(k)), g(k+1)) — order swapped — still closes", () => {
    const k = sym("k");
    const f = app(ADD, [
      app(NEG, [app(DIV, [int(1), k])]),
      app(DIV, [int(1), app(ADD, [k, int(1)])]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(int(-1));
  });

  it("Add(g(k), Div(-1, k+1)) (Phase 46: numerator-folded Neg) — antisymmetric", () => {
    // ∑_{k=1}^∞ [1/k + (-1)/(k+1)] = 1 (antisymmetric).
    const k = sym("k");
    const f = app(ADD, [
      app(DIV, [int(1), k]),
      app(DIV, [int(-1), app(ADD, [k, int(1)])]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(int(1));
  });

  it("Add(Div(-c, k+1), Div(c, k)) with c=5 (non-unit constant) closes to 5", () => {
    // ∑_{k=1}^∞ [(-5)/(k+1) + 5/k] = 5 — the Phase 46 constant-numerator
    // case after numerator-folded negation detection.
    const k = sym("k");
    const f = app(ADD, [
      app(DIV, [int(-5), app(ADD, [k, int(1)])]),
      app(DIV, [int(5), k]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(int(5));
  });

  it("Add(Div(1/2, k), Div(-1/2, k+1)) with rational numerator closes to 1/2", () => {
    // Exercises the IRRational arm of extractNegation's symmetric case.
    const k = sym("k");
    const f = app(ADD, [
      app(DIV, [rational(1, 2), k]),
      app(DIV, [rational(-1, 2), app(ADD, [k, int(1)])]),
    ]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(rational(1, 2));
  });

  it("Add(Neg(a), Neg(b)) (both negative) is left untouched — no telescope", () => {
    // ∑_{k=1}^N [-1 + -1/(k+1)] should NOT route through the telescope
    // detector; it has no g(k+1)−g(k) structure even after rewriting.
    const k = sym("k");
    const f = app(ADD, [
      app(NEG, [app(DIV, [int(1), k])]),
      app(NEG, [app(DIV, [int(1), app(ADD, [k, int(1)])])]),
    ]);
    // The summand doesn't telescope, so we expect the result NOT to be
    // the closed-form integer 1 or -1.  It either evaluates numerically
    // (for finite hi) or stays as an unevaluated Sum (for ∞).
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 49 (TypeScript port): Bounded × vanishing recogniser.
//
// Extends gVanishesAtInfinity to accept Div(bounded, diverging) shapes
// where the numerator is uniformly bounded (Sin/Cos, closed under
// Mul/Add/Neg, constants in k).  Closes telescopes like
// ∑ [sin(k)/k² − sin(k+1)/(k+1)²] = sin(1) that the Phase 42 degree-
// aware path refused (sin isn't a polynomial).
// ---------------------------------------------------------------------------

describe("summation: Phase 49 bounded × vanishing", () => {
  it("∑_{k=1}^∞ [sin(k)/k² − sin(k+1)/(k+1)²] closes", () => {
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kPlus1]);
    const f = app(SUB, [
      app(DIV, [sinK, app(POW, [k, int(2)])]),
      app(DIV, [sinKp1, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // Not the unevaluated Sum form.
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑_{k=1}^∞ [cos(k)/k³ − cos(k+1)/(k+1)³] closes", () => {
    const k = sym("k");
    const cosK = app(sym("Cos"), [k]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const cosKp1 = app(sym("Cos"), [kPlus1]);
    const f = app(SUB, [
      app(DIV, [cosK, app(POW, [k, int(3)])]),
      app(DIV, [cosKp1, app(POW, [kPlus1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("sin(k)·cos(k)/k² (Mul of bounded factors) closes", () => {
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const cosK = app(sym("Cos"), [k]);
    const numK = app(MUL, [sinK, cosK]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kPlus1]);
    const cosKp1 = app(sym("Cos"), [kPlus1]);
    const numKp1 = app(MUL, [sinKp1, cosKp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 50 (TypeScript port): Log/polynomial growth-rate recogniser.
//
// Closes Div(Log(diverging), diverging) telescopes via the squeeze
// argument: log(h) → ∞ at a logarithmic rate, denominator grows
// faster polynomially / exponentially, so log/poly → 0.
// Phase 49 refused log(k)/k² (log isn't bounded); Phase 50 closes it.
// ---------------------------------------------------------------------------

describe("summation: Phase 50 log/polynomial growth-rate", () => {
  it("∑_{k=1}^∞ [log(k)/k² − log(k+1)/(k+1)²] closes", () => {
    const k = sym("k");
    const logK = app(LOG, [k]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const logKp1 = app(LOG, [kPlus1]);
    const f = app(SUB, [
      app(DIV, [logK, app(POW, [k, int(2)])]),
      app(DIV, [logKp1, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑_{k=1}^∞ [log(k²+1)/k³ − log((k+1)²+1)/(k+1)³] closes", () => {
    const k = sym("k");
    const kSqPlus1 = app(ADD, [app(POW, [k, int(2)]), int(1)]);
    const logK = app(LOG, [kSqPlus1]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const kPlus1SqPlus1 = app(ADD, [app(POW, [kPlus1, int(2)]), int(1)]);
    const logKp1 = app(LOG, [kPlus1SqPlus1]);
    const f = app(SUB, [
      app(DIV, [logK, app(POW, [k, int(3)])]),
      app(DIV, [logKp1, app(POW, [kPlus1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: log(Mul(-1, k))/k² stays unevaluated", () => {
    // log of negative argument isn't real-valued for odd k.
    const k = sym("k");
    const negK = app(MUL, [int(-1), k]);
    const logNegK = app(LOG, [negK]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const negKp1 = app(MUL, [int(-1), kPlus1]);
    const logNegKp1 = app(LOG, [negKp1]);
    const f = app(SUB, [
      app(DIV, [logNegK, app(POW, [k, int(2)])]),
      app(DIV, [logNegKp1, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // Phase 50 must NOT close this.
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 51 (TypeScript port): Sqrt/polynomial growth-rate.
// ---------------------------------------------------------------------------

describe("summation: Phase 51 sqrt/polynomial growth-rate", () => {
  it("∑ [sqrt(k)/k² − sqrt(k+1)/(k+1)²] closes (1/2 < 2)", () => {
    const k = sym("k");
    const sqrtK = app(sym("Sqrt"), [k]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const sqrtKp1 = app(sym("Sqrt"), [kPlus1]);
    const f = app(SUB, [
      app(DIV, [sqrtK, app(POW, [k, int(2)])]),
      app(DIV, [sqrtKp1, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [sqrt(k³)/k² − ...] closes (3/2 < 2)", () => {
    const k = sym("k");
    const sqrtK3 = app(sym("Sqrt"), [app(POW, [k, int(3)])]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const sqrtKp1_3 = app(sym("Sqrt"), [app(POW, [kPlus1, int(3)])]);
    const f = app(SUB, [
      app(DIV, [sqrtK3, app(POW, [k, int(2)])]),
      app(DIV, [sqrtKp1_3, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: sqrt(Mul(-1, k))/k² stays unevaluated", () => {
    const k = sym("k");
    const negK = app(MUL, [int(-1), k]);
    const sqrtNegK = app(sym("Sqrt"), [negK]);
    const kPlus1 = app(ADD, [k, int(1)]);
    const sqrtNegKp1 = app(sym("Sqrt"), [app(MUL, [int(-1), kPlus1])]);
    const f = app(SUB, [
      app(DIV, [sqrtNegK, app(POW, [k, int(2)])]),
      app(DIV, [sqrtNegKp1, app(POW, [kPlus1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 52 (TypeScript port): Bounded × polynomial numerator.
// ---------------------------------------------------------------------------
// The numerator is Mul(bounded_factor, polynomial_in_k).  Phase 52 catches
// shapes like sin(k)·k/k³ that Phase 49 misses (the whole Mul isn't bounded)
// and Phase 42 refuses (sin is not polynomial).
//
// Tests mirror Python Phase 52 (cas-summation 1.0.0).

describe("summation: Phase 52 bounded × polynomial numerator", () => {
  it("∑ [sin(k)·k/k³ − sin(k+1)·(k+1)/(k+1)³] closes (bounded × deg 1 over deg 3)", () => {
    // Numerator = sin(k)·k = Mul(sin(k), k): bounded × polynomial deg 1.
    // Denominator = k³: polynomial deg 3.  3 > 1 → closes.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const sinK = app(sym("Sin"), [k]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const numK = app(MUL, [sinK, k]);
    const numKp1 = app(MUL, [sinKp1, kp1]);
    const denK = app(POW, [k, int(3)]);
    const denKp1 = app(POW, [kp1, int(3)]);
    const f = app(SUB, [
      app(DIV, [numK, denK]),
      app(DIV, [numKp1, denKp1]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [k·cos(k)/k² − ...] closes (factor order irrelevant: polynomial × bounded)", () => {
    // Numerator = k·cos(k) = Mul(k, cos(k)): polynomial deg 1 × bounded.
    // Denominator = k²: polynomial deg 2.  2 > 1 → closes.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const cosK = app(sym("Cos"), [k]);
    const cosKp1 = app(sym("Cos"), [kp1]);
    const numK = app(MUL, [k, cosK]);
    const numKp1 = app(MUL, [kp1, cosKp1]);
    const denK = app(POW, [k, int(2)]);
    const denKp1 = app(POW, [kp1, int(2)]);
    const f = app(SUB, [
      app(DIV, [numK, denK]),
      app(DIV, [numKp1, denKp1]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [sin(k)·k²/k³ − ...] closes (bounded × deg 2 over deg 3)", () => {
    // Numerator = sin(k)·k²: bounded × polynomial deg 2.
    // Denominator = k³: polynomial deg 3.  3 > 2 → closes.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const sinK = app(sym("Sin"), [k]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const numK = app(MUL, [sinK, app(POW, [k, int(2)])]);
    const numKp1 = app(MUL, [sinKp1, app(POW, [kp1, int(2)])]);
    const denK = app(POW, [k, int(3)]);
    const denKp1 = app(POW, [kp1, int(3)]);
    const f = app(SUB, [
      app(DIV, [numK, denK]),
      app(DIV, [numKp1, denKp1]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: sin(k)·k²/k² stays unevaluated (degrees tie: 2 > 2 is false)", () => {
    // Numerator = sin(k)·k²: bounded × polynomial deg 2.
    // Denominator = k²: polynomial deg 2.  2 > 2 is false → stays.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const sinK = app(sym("Sin"), [k]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const numK = app(MUL, [sinK, app(POW, [k, int(2)])]);
    const numKp1 = app(MUL, [sinKp1, app(POW, [kp1, int(2)])]);
    const denK = app(POW, [k, int(2)]);
    const denKp1 = app(POW, [kp1, int(2)]);
    const f = app(SUB, [
      app(DIV, [numK, denK]),
      app(DIV, [numKp1, denKp1]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });

  it("regression: k/k² still closes via Phase 42 (no bounded factor in numerator)", () => {
    // Numerator = k: pure polynomial deg 1.  No bounded factor → Phase 52 skips.
    // Phase 42 closes it: deg 1 < deg 2.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const f = app(SUB, [
      app(DIV, [k, app(POW, [k, int(2)])]),
      app(DIV, [kp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 53 (TypeScript port): Sqrt × polynomial numerator.
//
// The numerator is Mul(Sqrt(P(k)), polynomial_in_k).  Phase 53 catches
// shapes like sqrt(k)·k/k³ (eff deg = ½+1 = 3/2 < 3) and
// sqrt(k²)·k/k³ (eff deg = 1+1 = 2 < 3) that fall through all earlier phases.
//
// Effective degree = deg(P)/2 + deg(Q).  Closes when deg(den) > eff_deg.
// Tests mirror Python Phase 53 (cas-summation 1.1.0).
// ---------------------------------------------------------------------------
describe("summation: Phase 53 Sqrt × polynomial numerator", () => {
  it("Sqrt(k)·k/k³ closes (eff deg = ½+1 = 3/2 < 3)", () => {
    // Numerator = Sqrt(k)·k: eff deg 1/2 + 1 = 3/2.  Denominator = k³: deg 3.
    // 3 > 3/2 → closes.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Sqrt"), [k]), k]);
    const numKp1 = app(MUL, [app(sym("Sqrt"), [kp1]), kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("Sqrt(k²)·k/k³ closes (eff deg = 1+1 = 2 < 3)", () => {
    // Numerator = Sqrt(k²)·k: eff deg 2/2 + 1 = 2.  Denominator = k³: deg 3.
    // 3 > 2 → closes.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Sqrt"), [app(POW, [k, int(2)])]), k]);
    const numKp1 = app(MUL, [app(sym("Sqrt"), [app(POW, [kp1, int(2)])]), kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("Sqrt(k)·k²/k³ closes (eff deg = ½+2 = 5/2 < 3)", () => {
    // Numerator = Sqrt(k)·k²: eff deg 1/2 + 2 = 5/2.  Denominator = k³: deg 3.
    // 3 > 5/2 → closes.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Sqrt"), [k]), app(POW, [k, int(2)])]);
    const numKp1 = app(MUL, [app(sym("Sqrt"), [kp1]), app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: Sqrt(k)·k²/k² stays unevaluated (eff deg 5/2 > 2)", () => {
    // Numerator = Sqrt(k)·k²: eff deg 1/2 + 2 = 5/2.  Denominator = k²: deg 2.
    // 2 > 5/2 is false → stays unevaluated.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Sqrt"), [k]), app(POW, [k, int(2)])]);
    const numKp1 = app(MUL, [app(sym("Sqrt"), [kp1]), app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });

  it("regression: plain Sqrt(k)/k² still closes via Phase 51 (not Phase 53)", () => {
    // Phase 53 requires a Mul node; plain Sqrt(P) is handled by Phase 51.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const f = app(SUB, [
      app(DIV, [app(sym("Sqrt"), [k]), app(POW, [k, int(2)])]),
      app(DIV, [app(sym("Sqrt"), [kp1]), app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 54 — Log×polynomial numerator (TS port).
// ---------------------------------------------------------------------------
// log(h(k))·P(k)/Q(k) vanishes when deg(Q) > deg(P) (strictly).
// log grows sub-polynomially so its effective growth degree equals deg(P).
// ---------------------------------------------------------------------------

describe("summation: Phase 54 Log×polynomial numerator", () => {
  it("log(k)·k / k³ closes (poly_deg=1, den_deg=3)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Log"), [k]), k]);
    const numKp1 = app(MUL, [app(sym("Log"), [kp1]), kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("log(k)·k² / k³ closes (poly_deg=2, den_deg=3)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Log"), [k]), app(POW, [k, int(2)])]);
    const numKp1 = app(MUL, [app(sym("Log"), [kp1]), app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("log(k)·k / k² closes (poly_deg=1, den_deg=2)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Log"), [k]), k]);
    const numKp1 = app(MUL, [app(sym("Log"), [kp1]), kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("log(k)·k² / k² stays unevaluated (equal degrees — diverges)", () => {
    // log(k)*k²/k² = log(k) → diverges; equal degrees must be refused.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Log"), [k]), app(POW, [k, int(2)])]);
    const numKp1 = app(MUL, [app(sym("Log"), [kp1]), app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });

  it("regression: plain log(k)/k³ still closes via Phase 50", () => {
    // Phase 54 requires a Mul node; bare Log(k) is handled by Phase 50.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const f = app(SUB, [
      app(DIV, [app(sym("Log"), [k]), app(POW, [k, int(3)])]),
      app(DIV, [app(sym("Log"), [kp1]), app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 55 — Bounded×Log(diverging) numerator (TS port).
// ---------------------------------------------------------------------------
// ``sin(k)·log(k)/Q(k)`` vanishes at infinity when Q(k) diverges.
// The numerator is bounded×sub-polynomial — dominated by any polynomial
// or faster-growing denominator.
// isBoundedTimesLogInK requires exactly one Log(diverging) factor; all
// other factors must pass isBoundedInK.
// ---------------------------------------------------------------------------

describe("summation: Phase 55 Bounded×Log(diverging) numerator", () => {
  it("sin(k)·log(k) / k² closes (bounded×log / poly-2)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Sin"), [k]), app(sym("Log"), [k])]);
    const numKp1 = app(MUL, [app(sym("Sin"), [kp1]), app(sym("Log"), [kp1])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("cos(k)·log(k) / k closes (bounded×log / poly-1)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Cos"), [k]), app(sym("Log"), [k])]);
    const numKp1 = app(MUL, [app(sym("Cos"), [kp1]), app(sym("Log"), [kp1])]);
    const f = app(SUB, [
      app(DIV, [numK, k]),
      app(DIV, [numKp1, kp1]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("sin(k)·cos(k)·log(k) / k³ closes (two bounded×log / poly-3)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Sin"), [k]), app(sym("Cos"), [k]), app(sym("Log"), [k])]);
    const numKp1 = app(MUL, [app(sym("Sin"), [kp1]), app(sym("Cos"), [kp1]), app(sym("Log"), [kp1])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("sin(k)·log(k²) / k³ closes (log of k² diverges, bounded×log)", () => {
    // log(k²) diverges (k² is a positive-degree polynomial).
    // After substituting k→k+1: log((k+1)²) — structural equality holds.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const kSq = app(POW, [k, int(2)]);
    const kp1Sq = app(POW, [kp1, int(2)]);
    const numK = app(MUL, [app(sym("Sin"), [k]), app(sym("Log"), [kSq])]);
    const numKp1 = app(MUL, [app(sym("Sin"), [kp1]), app(sym("Log"), [kp1Sq])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("sin(k)·log(k) / 1 stays unevaluated (constant denominator — does not diverge)", () => {
    // Denominator = 1 (constant). hDivergesAtInfinity(1) = false.
    // Phase 55 correctly refuses; no other phase closes.
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Sin"), [k]), app(sym("Log"), [k])]);
    const numKp1 = app(MUL, [app(sym("Sin"), [kp1]), app(sym("Log"), [kp1])]);
    const f = app(SUB, [
      app(DIV, [numK, int(1)]),
      app(DIV, [numKp1, int(1)]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Phase 56 (TS port): bounded × Sqrt(diverging) numerator.
// ---------------------------------------------------------------------------

describe("summation: Phase 56 bounded × sqrt numerator", () => {
  it("∑ [sin(k)·sqrt(k)/k² − ...] closes (1/2 < 2)", () => {
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const sqrtK = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [sinK, sqrtK]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const sqrtKp1 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, sqrtKp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [sin(k)·sqrt(k³)/2^k − ...] closes (sqrt < exp)", () => {
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const sqrtK3 = app(sym("Sqrt"), [app(POW, [k, int(3)])]);
    const numK = app(MUL, [sinK, sqrtK3]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const sqrtKp1_3 = app(sym("Sqrt"), [app(POW, [kp1, int(3)])]);
    const numKp1 = app(MUL, [sinKp1, sqrtKp1_3]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [int(2), k])]),
      app(DIV, [numKp1, app(POW, [int(2), kp1])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: sin(k)·sqrt(k³)/k stays unevaluated (3/2 > 1)", () => {
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const sqrtK3 = app(sym("Sqrt"), [app(POW, [k, int(3)])]);
    const numK = app(MUL, [sinK, sqrtK3]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const sqrtKp1_3 = app(sym("Sqrt"), [app(POW, [kp1, int(3)])]);
    const numKp1 = app(MUL, [sinKp1, sqrtKp1_3]);
    const f = app(SUB, [
      app(DIV, [numK, k]),
      app(DIV, [numKp1, kp1]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // 3/2 > 1 → does not vanish.
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// Phase 57 (TS port): bounded × Log(diverging) × Sqrt(positive-poly) numerator.
// ---------------------------------------------------------------------------

describe("summation: Phase 57 bounded × log × sqrt numerator", () => {
  it("∑ [sin(k)·log(k)·sqrt(k)/k² − ...] closes (log·k^½ < k²)", () => {
    // sin(k)·log(k)·sqrt(k) / k²: effective growth k^½·log(k) dominated
    // by k² (half-degree = 1/2, den-deg = 2, 2 > 1/2 ✓).
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(sym("Log"), [k]);
    const sqrtK = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [sinK, logK, sqrtK]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(sym("Log"), [kp1]);
    const sqrtKp1 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, logKp1, sqrtKp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [log(k)·sqrt(k)/k² − ...] closes (no bounded factor needed)", () => {
    // Pure log·sqrt without a bounded factor — still Phase 57 (log_count=1,
    // sqrt present).  half-degree = 1/2, den-deg = 2.
    const k = sym("k");
    const logK = app(sym("Log"), [k]);
    const sqrtK = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [logK, sqrtK]);
    const kp1 = app(ADD, [k, int(1)]);
    const logKp1 = app(sym("Log"), [kp1]);
    const sqrtKp1 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [logKp1, sqrtKp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [sin(k)·log(k)·sqrt(k³)/2^k − ...] closes (exp dominates)", () => {
    // Exponential denominator — non-polynomial diverging, dominates any
    // half-polynomial growth.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(sym("Log"), [k]);
    const sqrtK3 = app(sym("Sqrt"), [app(POW, [k, int(3)])]);
    const numK = app(MUL, [sinK, logK, sqrtK3]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(sym("Log"), [kp1]);
    const sqrtKp1_3 = app(sym("Sqrt"), [app(POW, [kp1, int(3)])]);
    const numKp1 = app(MUL, [sinKp1, logKp1, sqrtKp1_3]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [int(2), k])]),
      app(DIV, [numKp1, app(POW, [int(2), kp1])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: sin(k)·log(k)·sqrt(k³)/k stays unevaluated (3/2 > 1)", () => {
    // half-degree of sqrt(k³) = 3/2 > den-deg 1 → does NOT vanish.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(sym("Log"), [k]);
    const sqrtK3 = app(sym("Sqrt"), [app(POW, [k, int(3)])]);
    const numK = app(MUL, [sinK, logK, sqrtK3]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(sym("Log"), [kp1]);
    const sqrtKp1_3 = app(sym("Sqrt"), [app(POW, [kp1, int(3)])]);
    const numKp1 = app(MUL, [sinKp1, logKp1, sqrtKp1_3]);
    const f = app(SUB, [
      app(DIV, [numK, k]),
      app(DIV, [numKp1, kp1]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // sqrt(k³) half-degree 3/2 > denDeg 1 → refused.
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

// Phase 58 (TS port): bounded × Log(diverging) × polynomial numerator.
// ---------------------------------------------------------------------------

describe("summation: Phase 58 bounded × log × polynomial numerator", () => {
  it("∑ [sin(k)·log(k)·k/k³ − ...] closes (polyDeg=1 < denDeg=3)", () => {
    // sin(k)·log(k)·k / k³: log sub-polynomial, effective poly deg = 1,
    // denominator deg = 3, 3 > 1 ✓.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(sym("Log"), [k]);
    const numK = app(MUL, [sinK, logK, k]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(sym("Log"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, logKp1, kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [sin(k)·log(k)·k²/2^k − ...] closes (exp denominator dominates)", () => {
    // Exponential denominator — non-polynomial diverging, dominates any k^m.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(sym("Log"), [k]);
    const numK = app(MUL, [sinK, logK, app(POW, [k, int(2)])]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(sym("Log"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, logKp1, app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [int(2), k])]),
      app(DIV, [numKp1, app(POW, [int(2), kp1])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [cos(k)·log(k)·k²/k⁴ − ...] closes (polyDeg=2 < denDeg=4)", () => {
    const k = sym("k");
    const cosK = app(sym("Cos"), [k]);
    const logK = app(sym("Log"), [k]);
    const numK = app(MUL, [cosK, logK, app(POW, [k, int(2)])]);
    const kp1 = app(ADD, [k, int(1)]);
    const cosKp1 = app(sym("Cos"), [kp1]);
    const logKp1 = app(sym("Log"), [kp1]);
    const numKp1 = app(MUL, [cosKp1, logKp1, app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(4)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(4)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: sin(k)·log(k)·k²/k² stays unevaluated (equal degrees)", () => {
    // polyDeg = denDeg = 2: log(k)·C diverges → refused.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(sym("Log"), [k]);
    const numK = app(MUL, [sinK, logK, app(POW, [k, int(2)])]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(sym("Log"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, logKp1, app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

describe("summation: Phase 59 bounded × Sqrt(positive-poly) × polynomial numerator", () => {
  // Phase 59 fills the gap between:
  //   Phase 53: Mul(Sqrt, polynomial_only) — refuses bounded factors
  //   Phase 56: Mul(bounded, Sqrt)         — refuses polynomial factors
  // Effective growth: C·k^{deg(P)/2 + polyDeg}.
  // ×2 trick: effective_x2 = deg(P) + 2·polyDeg.
  // Vanishes when 2·denDeg > effective_x2 or non-polynomial diverging denom.

  it("∑ [sin(k)·√k·k/k³ − ...] closes (x2=3, 2·3=6>3)", () => {
    // sqrt_inner_deg_x2=1, poly_deg=1, effective_x2=3; 2·3=6 > 3.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const sqrtK = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [sinK, sqrtK, k]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const sqrtKp1 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, sqrtKp1, kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [cos(k)·√(k²)·k²/k⁴ − ...] closes (x2=6, 2·4=8>6)", () => {
    // sqrt_inner_deg_x2=2, poly_deg=2, effective_x2=6; 2·4=8 > 6.
    const k = sym("k");
    const cosK = app(sym("Cos"), [k]);
    const sqrtK2 = app(sym("Sqrt"), [app(POW, [k, int(2)])]);
    const numK = app(MUL, [cosK, sqrtK2, app(POW, [k, int(2)])]);
    const kp1 = app(ADD, [k, int(1)]);
    const cosKp1 = app(sym("Cos"), [kp1]);
    const sqrtKp1_2 = app(sym("Sqrt"), [app(POW, [kp1, int(2)])]);
    const numKp1 = app(MUL, [cosKp1, sqrtKp1_2, app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(4)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(4)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [sin(k)·√k·k²/2^k − ...] closes (exp denominator dominates)", () => {
    // Non-polynomial diverging denominator.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const sqrtK = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [sinK, sqrtK, app(POW, [k, int(2)])]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const sqrtKp1 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, sqrtKp1, app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [int(2), k])]),
      app(DIV, [numKp1, app(POW, [int(2), kp1])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: sin(k)·√(k²)·k/k² stays unevaluated (equal: x2=4, 2·2=4)", () => {
    // sqrt_inner_deg_x2=2, poly_deg=1, effective_x2=4; 2·2=4 not > 4 → refused.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const sqrtK2 = app(sym("Sqrt"), [app(POW, [k, int(2)])]);
    const numK = app(MUL, [sinK, sqrtK2, k]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const sqrtKp1_2 = app(sym("Sqrt"), [app(POW, [kp1, int(2)])]);
    const numKp1 = app(MUL, [sinKp1, sqrtKp1_2, kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

describe("summation: Phase 60 bounded × Log(diverging) × Sqrt(positive-poly) × polynomial numerator", () => {
  // Phase 60 closes the gap left by Phase 57 (bounded×Log×Sqrt, refuses poly).
  // Effective growth: log(k)·k^{sqrtHalfDeg + polyDeg} = o(k^{sqrtHalfDeg+polyDeg+ε}).
  // TypeScript convention: compare denDeg > sqrtHalfDeg + polyDeg (no ×2).

  it("∑ [sin(k)·log(k)·√k·k/k³ − ...] closes (halfDeg=0.5, polyDeg=1, eff=1.5, denDeg=3)", () => {
    // sqrtHalfDeg=0.5, polyDeg=1, effectiveDeg=1.5; denDeg=3 > 1.5 → closes.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(LOG, [k]);
    const sqrtK = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [sinK, logK, sqrtK, k]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(LOG, [kp1]);
    const sqrtKp1 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, logKp1, sqrtKp1, kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(3)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(3)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [cos(k)·log(k)·√(k²)·k²/k⁴ − ...] closes (halfDeg=1, polyDeg=2, eff=3, denDeg=4)", () => {
    // sqrtHalfDeg=1, polyDeg=2, effectiveDeg=3; denDeg=4 > 3 → closes.
    const k = sym("k");
    const cosK = app(sym("Cos"), [k]);
    const logK = app(LOG, [k]);
    const sqrtK2 = app(sym("Sqrt"), [app(POW, [k, int(2)])]);
    const numK = app(MUL, [cosK, logK, sqrtK2, app(POW, [k, int(2)])]);
    const kp1 = app(ADD, [k, int(1)]);
    const cosKp1 = app(sym("Cos"), [kp1]);
    const logKp1 = app(LOG, [kp1]);
    const sqrtKp1_2 = app(sym("Sqrt"), [app(POW, [kp1, int(2)])]);
    const numKp1 = app(MUL, [cosKp1, logKp1, sqrtKp1_2, app(POW, [kp1, int(2)])]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(4)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(4)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [sin(k)·log(k)·√k·k/2^k − ...] closes (exp denominator dominates)", () => {
    // Non-polynomial diverging denominator dominates any poly×log×sqrt growth.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(LOG, [k]);
    const sqrtK = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [sinK, logK, sqrtK, k]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(LOG, [kp1]);
    const sqrtKp1 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, logKp1, sqrtKp1, kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [int(2), k])]),
      app(DIV, [numKp1, app(POW, [int(2), kp1])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: sin(k)·log(k)·√(k²)·k/k² stays unevaluated (equal: halfDeg=1, polyDeg=1, eff=2, denDeg=2)", () => {
    // sqrtHalfDeg=1, polyDeg=1, effectiveDeg=2; denDeg=2 not > 2 → refused.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const logK = app(LOG, [k]);
    const sqrtK2 = app(sym("Sqrt"), [app(POW, [k, int(2)])]);
    const numK = app(MUL, [sinK, logK, sqrtK2, k]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const logKp1 = app(LOG, [kp1]);
    const sqrtKp1_2 = app(sym("Sqrt"), [app(POW, [kp1, int(2)])]);
    const numKp1 = app(MUL, [sinKp1, logKp1, sqrtKp1_2, kp1]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

describe("summation: Phase 61 two Sqrt × polynomial numerator", () => {
  // Phase 61: Mul(Sqrt(P1), Sqrt(P2), poly..., bounded...) numerator.
  // Effective degree: sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  // Vanishes when denDeg > effectiveDeg or non-polynomial diverging denom.

  it("∑ [√k · √(k³) / k³ − ...] closes (halfDeg1=0.5, halfDeg2=1.5, eff=2, denDeg=3)", () => {
    // Two distinct Sqrt degrees: x2=1+3=4; TS: eff=0.5+1.5=2; denDeg=3 > 2 → closes.
    const k = sym("k");
    const k3 = app(POW, [k, int(3)]);
    const sqrtK = app(sym("Sqrt"), [k]);
    const sqrtK3 = app(sym("Sqrt"), [k3]);
    const numK = app(MUL, [sqrtK, sqrtK3]);
    const kp1 = app(ADD, [k, int(1)]);
    const kp1_3 = app(POW, [kp1, int(3)]);
    const sqrtKp1 = app(sym("Sqrt"), [kp1]);
    const sqrtKp1_3 = app(sym("Sqrt"), [kp1_3]);
    const numKp1 = app(MUL, [sqrtKp1, sqrtKp1_3]);
    const f = app(SUB, [
      app(DIV, [numK, k3]),
      app(DIV, [numKp1, kp1_3]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [sin(k)·√k·√k / k² − ...] closes (halfDeg1=0.5, halfDeg2=0.5, eff=1, denDeg=2)", () => {
    // bounded × two Sqrt: eff=0.5+0.5=1; denDeg=2 > 1 → closes.
    const k = sym("k");
    const sinK = app(sym("Sin"), [k]);
    const sqrtK1 = app(sym("Sqrt"), [k]);
    const sqrtK2 = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [sinK, sqrtK1, sqrtK2]);
    const kp1 = app(ADD, [k, int(1)]);
    const sinKp1 = app(sym("Sin"), [kp1]);
    const sqrtKp1_1 = app(sym("Sqrt"), [kp1]);
    const sqrtKp1_2 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [sinKp1, sqrtKp1_1, sqrtKp1_2]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [k, int(2)])]),
      app(DIV, [numKp1, app(POW, [kp1, int(2)])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("∑ [√k·√k / 2^k − ...] closes (exp denominator dominates)", () => {
    // Non-polynomial diverging denominator dominates any two-Sqrt growth.
    const k = sym("k");
    const sqrtK1 = app(sym("Sqrt"), [k]);
    const sqrtK2 = app(sym("Sqrt"), [k]);
    const numK = app(MUL, [sqrtK1, sqrtK2]);
    const kp1 = app(ADD, [k, int(1)]);
    const sqrtKp1_1 = app(sym("Sqrt"), [kp1]);
    const sqrtKp1_2 = app(sym("Sqrt"), [kp1]);
    const numKp1 = app(MUL, [sqrtKp1_1, sqrtKp1_2]);
    const f = app(SUB, [
      app(DIV, [numK, app(POW, [int(2), k])]),
      app(DIV, [numKp1, app(POW, [int(2), kp1])]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).not.toEqual(SUM);
  });

  it("regression: √(k²)·√(k²)/k² stays unevaluated (equal: eff=2, denDeg=2)", () => {
    // halfDeg1=1, halfDeg2=1, polyDeg=0, eff=2; denDeg=2 not > 2 → refused.
    const k = sym("k");
    const k2 = app(POW, [k, int(2)]);
    const sqrtK2_1 = app(sym("Sqrt"), [k2]);
    const sqrtK2_2 = app(sym("Sqrt"), [k2]);
    const numK = app(MUL, [sqrtK2_1, sqrtK2_2]);
    const kp1 = app(ADD, [k, int(1)]);
    const kp1_2 = app(POW, [kp1, int(2)]);
    const sqrtKp1_2_1 = app(sym("Sqrt"), [kp1_2]);
    const sqrtKp1_2_2 = app(sym("Sqrt"), [kp1_2]);
    const numKp1 = app(MUL, [sqrtKp1_2_1, sqrtKp1_2_2]);
    const f = app(SUB, [
      app(DIV, [numK, k2]),
      app(DIV, [numKp1, kp1_2]),
    ]);
    const out = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(out.kind === "apply" ? out.head : undefined).toEqual(SUM);
  });
});

describe("Phase 62 — Two-Log × polynomial numerator", () => {
  it("log(k)·log(k) / k² closes (poly_deg=0, denDeg=2 > 0)", () => {
    const k = sym("k");
    const logK = { kind: "apply" as const, head: LOG, args: [k] };
    const logK2 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [logK, logK2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const logKp1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const logKp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [logKp1, logKp1_2] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)²·k / k³ closes (poly_deg=1, denDeg=3 > 1)", () => {
    const k = sym("k");
    const logK = { kind: "apply" as const, head: LOG, args: [k] };
    const logK2 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [logK, logK2, k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const logKp1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const logKp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [logKp1, logKp1_2, kp1] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)²·k² / k² refused (poly_deg=2, denDeg=2 not > 2)", () => {
    const k = sym("k");
    const logK = { kind: "apply" as const, head: LOG, args: [k] };
    const logK2 = { kind: "apply" as const, head: LOG, args: [k] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const numK = { kind: "apply" as const, head: MUL, args: [logK, logK2, k2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const logKp1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const logKp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [logKp1, logKp1_2, kp1_2] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)² / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const logK = { kind: "apply" as const, head: LOG, args: [k] };
    const logK2 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [logK, logK2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const logKp1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const logKp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [logKp1, logKp1_2] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 63 — Two-Sqrt × Log × polynomial numerator", () => {
  it("√k·√k·log(k) / k² closes (effective=1, denDeg=2 > 1)", () => {
    const k = sym("k");
    const sqrt_k1 = { kind: "apply" as const, head: sym("Sqrt"), args: [k] };
    const sqrt_k2 = { kind: "apply" as const, head: sym("Sqrt"), args: [k] };
    const log_k = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k1, sqrt_k2, log_k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const sqrt_kp1_1 = { kind: "apply" as const, head: sym("Sqrt"), args: [kp1] };
    const sqrt_kp1_2 = { kind: "apply" as const, head: sym("Sqrt"), args: [kp1] };
    const log_kp1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_1, sqrt_kp1_2, log_kp1] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√k·log(k) / k³ closes (effective=2, denDeg=3 > 2)", () => {
    const k = sym("k");
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const sqrt_k3 = { kind: "apply" as const, head: sym("Sqrt"), args: [k3] };
    const sqrt_k = { kind: "apply" as const, head: sym("Sqrt"), args: [k] };
    const log_k = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k3, sqrt_k, log_k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const sqrt_kp1_3 = { kind: "apply" as const, head: sym("Sqrt"), args: [kp1_3] };
    const sqrt_kp1 = { kind: "apply" as const, head: sym("Sqrt"), args: [kp1] };
    const log_kp1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_3, sqrt_kp1, log_kp1] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k²)·√(k²)·log(k) / k² refused (effective=2, denDeg=2 not > 2)", () => {
    const k = sym("k");
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const sqrt_k2_1 = { kind: "apply" as const, head: sym("Sqrt"), args: [k2] };
    const sqrt_k2_2 = { kind: "apply" as const, head: sym("Sqrt"), args: [k2] };
    const log_k = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k2_1, sqrt_k2_2, log_k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const sqrt_kp1_2_1 = { kind: "apply" as const, head: sym("Sqrt"), args: [kp1_2] };
    const sqrt_kp1_2_2 = { kind: "apply" as const, head: sym("Sqrt"), args: [kp1_2] };
    const log_kp1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_2_1, sqrt_kp1_2_2, log_kp1] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·log(k) / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const sqrt_k1 = { kind: "apply" as const, head: sym("Sqrt"), args: [k] };
    const sqrt_k2 = { kind: "apply" as const, head: sym("Sqrt"), args: [k] };
    const log_k = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k1, sqrt_k2, log_k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const sqrt_kp1_1 = { kind: "apply" as const, head: sym("Sqrt"), args: [kp1] };
    const sqrt_kp1_2 = { kind: "apply" as const, head: sym("Sqrt"), args: [kp1] };
    const log_kp1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_1, sqrt_kp1_2, log_kp1] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 65 — Two-Sqrt × Two-Log × polynomial numerator", () => {
  it("√k·√k·log(k)² / k² closes (effective=1, denDeg=2 > 1)", () => {
    const k = sym("k");
    const sqrt_k1 = { kind: "apply" as const, head: SQRT, args: [k] };
    const sqrt_k2 = { kind: "apply" as const, head: SQRT, args: [k] };
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k1, sqrt_k2, log_k1, log_k2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const sqrt_kp1_1 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const sqrt_kp1_2 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_1, sqrt_kp1_2, log_kp1_1, log_kp1_2] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√k·log(k)² / k³ closes (effective=2, denDeg=3 > 2)", () => {
    const k = sym("k");
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const sqrt_k3 = { kind: "apply" as const, head: SQRT, args: [k3] };
    const sqrt_k = { kind: "apply" as const, head: SQRT, args: [k] };
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k3, sqrt_k, log_k1, log_k2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const sqrt_kp1_3 = { kind: "apply" as const, head: SQRT, args: [kp1_3] };
    const sqrt_kp1 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_3, sqrt_kp1, log_kp1_1, log_kp1_2] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k²)·√(k²)·log(k)² / k² refused (effective=2, denDeg=2 not > 2)", () => {
    const k = sym("k");
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const sqrt_k2_1 = { kind: "apply" as const, head: SQRT, args: [k2] };
    const sqrt_k2_2 = { kind: "apply" as const, head: SQRT, args: [k2] };
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k2_1, sqrt_k2_2, log_k1, log_k2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const sqrt_kp1_2_1 = { kind: "apply" as const, head: SQRT, args: [kp1_2] };
    const sqrt_kp1_2_2 = { kind: "apply" as const, head: SQRT, args: [kp1_2] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_2_1, sqrt_kp1_2_2, log_kp1_1, log_kp1_2] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·log(k)² / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const sqrt_k1 = { kind: "apply" as const, head: SQRT, args: [k] };
    const sqrt_k2 = { kind: "apply" as const, head: SQRT, args: [k] };
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k1, sqrt_k2, log_k1, log_k2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const sqrt_kp1_1 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const sqrt_kp1_2 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_1, sqrt_kp1_2, log_kp1_1, log_kp1_2] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 66 — Three-Sqrt × polynomial numerator", () => {
  it("√k·√k·√k / k² closes (effective=1.5, denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const sqrt_k1 = { kind: "apply" as const, head: SQRT, args: [k] };
    const sqrt_k2 = { kind: "apply" as const, head: SQRT, args: [k] };
    const sqrt_k3 = { kind: "apply" as const, head: SQRT, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k1, sqrt_k2, sqrt_k3] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const sqrt_kp1_1 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const sqrt_kp1_2 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const sqrt_kp1_3 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_1, sqrt_kp1_2, sqrt_kp1_3] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√k·√k / k³ closes (effective=2.5, denDeg=3 > 2.5)", () => {
    const k = sym("k");
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const sqrt_k3 = { kind: "apply" as const, head: SQRT, args: [k3] };
    const sqrt_k1 = { kind: "apply" as const, head: SQRT, args: [k] };
    const sqrt_k2 = { kind: "apply" as const, head: SQRT, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k3, sqrt_k1, sqrt_k2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const sqrt_kp1_3 = { kind: "apply" as const, head: SQRT, args: [kp1_3] };
    const sqrt_kp1_1 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const sqrt_kp1_2 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_3, sqrt_kp1_1, sqrt_kp1_2] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k²)·√(k²)·√(k²) / k² refused (effective=3, denDeg=2 not > 3)", () => {
    const k = sym("k");
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const sqrt_k2_1 = { kind: "apply" as const, head: SQRT, args: [k2] };
    const sqrt_k2_2 = { kind: "apply" as const, head: SQRT, args: [k2] };
    const sqrt_k2_3 = { kind: "apply" as const, head: SQRT, args: [k2] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k2_1, sqrt_k2_2, sqrt_k2_3] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const sqrt_kp1_2_1 = { kind: "apply" as const, head: SQRT, args: [kp1_2] };
    const sqrt_kp1_2_2 = { kind: "apply" as const, head: SQRT, args: [kp1_2] };
    const sqrt_kp1_2_3 = { kind: "apply" as const, head: SQRT, args: [kp1_2] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_2_1, sqrt_kp1_2_2, sqrt_kp1_2_3] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const sqrt_k1 = { kind: "apply" as const, head: SQRT, args: [k] };
    const sqrt_k2 = { kind: "apply" as const, head: SQRT, args: [k] };
    const sqrt_k3 = { kind: "apply" as const, head: SQRT, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [sqrt_k1, sqrt_k2, sqrt_k3] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const sqrt_kp1_1 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const sqrt_kp1_2 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const sqrt_kp1_3 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [sqrt_kp1_1, sqrt_kp1_2, sqrt_kp1_3] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 67 — Three-Log × polynomial numerator", () => {
  it("log(k)³ / k closes (effective=0, denDeg=1 > 0)", () => {
    const k = sym("k");
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k3 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [log_k1, log_k2, log_k3] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_3 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [log_kp1_1, log_kp1_2, log_kp1_3] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)³·k / k² closes (effective=1, denDeg=2 > 1)", () => {
    const k = sym("k");
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k3 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [log_k1, log_k2, log_k3, k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_3 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [log_kp1_1, log_kp1_2, log_kp1_3, kp1] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)³·k / k refused (effective=1, denDeg=1 not > 1)", () => {
    const k = sym("k");
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k3 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [log_k1, log_k2, log_k3, k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_3 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [log_kp1_1, log_kp1_2, log_kp1_3, kp1] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)³ / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k3 = { kind: "apply" as const, head: LOG, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [log_k1, log_k2, log_k3] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_3 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [log_kp1_1, log_kp1_2, log_kp1_3] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 64 — Two-Log × Sqrt × polynomial numerator", () => {
  it("log(k)²·√k / k² closes (effective=0.5, denDeg=2 > 0.5)", () => {
    const k = sym("k");
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const sqrt_k = { kind: "apply" as const, head: SQRT, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [log_k1, log_k2, sqrt_k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const sqrt_kp1 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [log_kp1_1, log_kp1_2, sqrt_kp1] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)²·√(k³) / k³ closes (effective=1.5, denDeg=3 > 1.5)", () => {
    const k = sym("k");
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const sqrt_k3 = { kind: "apply" as const, head: SQRT, args: [k3] };
    const numK = { kind: "apply" as const, head: MUL, args: [log_k1, log_k2, sqrt_k3] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const sqrt_kp1_3 = { kind: "apply" as const, head: SQRT, args: [kp1_3] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [log_kp1_1, log_kp1_2, sqrt_kp1_3] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)²·√(k²) / k refused (effective=1, denDeg=1 not > 1)", () => {
    const k = sym("k");
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const sqrt_k2 = { kind: "apply" as const, head: SQRT, args: [k2] };
    const numK = { kind: "apply" as const, head: MUL, args: [log_k1, log_k2, sqrt_k2] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const sqrt_kp1_2 = { kind: "apply" as const, head: SQRT, args: [kp1_2] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [log_kp1_1, log_kp1_2, sqrt_kp1_2] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)²·√k / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const log_k1 = { kind: "apply" as const, head: LOG, args: [k] };
    const log_k2 = { kind: "apply" as const, head: LOG, args: [k] };
    const sqrt_k = { kind: "apply" as const, head: SQRT, args: [k] };
    const numK = { kind: "apply" as const, head: MUL, args: [log_k1, log_k2, sqrt_k] };
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const log_kp1_1 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const log_kp1_2 = { kind: "apply" as const, head: LOG, args: [kp1] };
    const sqrt_kp1 = { kind: "apply" as const, head: SQRT, args: [kp1] };
    const numKp1 = { kind: "apply" as const, head: MUL, args: [log_kp1_1, log_kp1_2, sqrt_kp1] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 68 — Three-Sqrt × Log × polynomial numerator", () => {
  it("√k·√k·√k·log(k) / k² closes (effective=1.5, denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√k·√k·log(k) / k³ closes (effective=2.5, denDeg=3 > 2.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k) / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k) / k refused (effective=1.5, denDeg=1 not > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 69 — One-Sqrt × Three-Log × polynomial numerator", () => {
  it("√k·log(k)³ / k² closes (effective=0.5, denDeg=2 > 0.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·log(k)³ / k³ closes (effective=1.5, denDeg=3 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·log(k)³ / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·log(k)³ / 1 refused (effective=0.5, denDeg=0 not > 0.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, int(1)] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, int(1)] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 70 — Three-Sqrt × Two-Log × polynomial numerator", () => {
  it("√k·√k·√k·log(k)²/k² closes (effective=1.5, denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√k·√k·log(k)²/k³ closes (effective=2.5, denDeg=3 > 2.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k)² / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k)² / k refused (effective=1.5, denDeg=1 not > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 71 — Two-Sqrt × Three-Log × polynomial numerator", () => {
  it("√k·√k·log(k)³/k² closes (effective=1, denDeg=2 > 1)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√k·log(k)³/k³ closes (effective=2, denDeg=3 > 2)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·log(k)³ / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·log(k)³ / k refused (effective=1, denDeg=1 not > 1)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 72 — Three-Sqrt × Three-Log × polynomial numerator", () => {
  it("√k·√k·√k·log(k)³/k² closes (effective=1.5, denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√k·√k·log(k)³/k³ closes (effective=2.5, denDeg=3 > 2.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k3] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_3] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k)³ / 2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k)³ / k refused (effective=1.5, denDeg=1 not > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 73 — Four-Log × polynomial numerator", () => {
  // g(k) = log(k)^4 * [poly...] / den(k), effective_deg = polyDeg.
  // Closes when denDeg > polyDeg (or denominator is non-polynomial but diverging).

  it("log(k)⁴/k closes (polyDeg=0, denDeg=1 > 0)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)⁴·k/k² closes (polyDeg=1, denDeg=2 > 1)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)⁴/2^k closes (exponential denominator)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, { kind: "apply" as const, head: POW, args: [int(2), k] }] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, { kind: "apply" as const, head: POW, args: [int(2), kp1] }] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)⁴·k/k refused (polyDeg=1, denDeg=1 not > 1)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 74 — One-Sqrt × Four-Log × polynomial numerator", () => {
  // g(k) = √P(k) · log(k)⁴ · [poly...] / den(k)
  // effective_deg = sqrtHalfDeg + polyDeg; closes when denDeg > effective_deg.

  it("√k·log(k)⁴/k closes (sqrtHalf=0.5, polyDeg=0, denDeg=1 > 0.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK74 = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp174 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f74 = { kind: "apply" as const, head: SUB, args: [gK74, gKp174] };
    const result = evaluateSum(f74, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·log(k)⁴/k² closes (sqrtHalf=1.5, polyDeg=0, denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK74b = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp174b = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f74b = { kind: "apply" as const, head: SUB, args: [gK74b, gKp174b] };
    const result = evaluateSum(f74b, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·log(k)⁴·k/k refused (sqrtHalf=0.5, polyDeg=1, denDeg=1 not > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK74r = { kind: "apply" as const, head: DIV, args: [numK, k] };
    const gKp174r = { kind: "apply" as const, head: DIV, args: [numKp1, kp1] };
    const f74r = { kind: "apply" as const, head: SUB, args: [gK74r, gKp174r] };
    const result = evaluateSum(f74r, k, int(1), sym("%inf"), evalNode);
    // eff_x2=1+2=3; 2*den_deg=2*1=2 not > 3 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 75 — Two-Sqrt × Four-Log × polynomial numerator", () => {
  // g(k) = √P1(k) · √P2(k) · log(k)⁴ · [poly...] / den(k)
  // effective_deg = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  // Closes when denDeg > effective_deg.

  it("√k·√k·log(k)⁴/k² closes (each sqrtHalf=0.5, polyDeg=0, denDeg=2 > 1.0)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK75 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp175 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK75 = { kind: "apply" as const, head: DIV, args: [numK75, k2] };
    const gKp175 = { kind: "apply" as const, head: DIV, args: [numKp175, kp1_2] };
    const f75 = { kind: "apply" as const, head: SUB, args: [gK75, gKp175] };
    const result = evaluateSum(f75, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√(k³)·log(k)⁴/k² refused (each sqrtHalf=1.5, effective=3.0, denDeg=2 not > 3.0)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK75b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp175b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK75b = { kind: "apply" as const, head: DIV, args: [numK75b, k2] };
    const gKp175b = { kind: "apply" as const, head: DIV, args: [numKp175b, kp1_2] };
    const f75b = { kind: "apply" as const, head: SUB, args: [gK75b, gKp175b] };
    const result = evaluateSum(f75b, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·log(k)⁴·k/k² refused (effective=1.0, polyDeg=1, effective=2.0, denDeg=2 not > 2.0)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK75r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp175r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK75r = { kind: "apply" as const, head: DIV, args: [numK75r, k2] };
    const gKp175r = { kind: "apply" as const, head: DIV, args: [numKp175r, kp1_2] };
    const f75r = { kind: "apply" as const, head: SUB, args: [gK75r, gKp175r] };
    const result = evaluateSum(f75r, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=0.5, sqrtHalf2=0.5, polyDeg=1 → effective=2.0; denDeg=2 not > 2.0 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 76 — Three-Sqrt × Four-Log × polynomial numerator", () => {
  it("√k·√k·√k·log(k)⁴/k² closes (effective=1.5, denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp1 = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK = { kind: "apply" as const, head: DIV, args: [numK, k2] };
    const gKp1 = { kind: "apply" as const, head: DIV, args: [numKp1, kp1_2] };
    const f = { kind: "apply" as const, head: SUB, args: [gK, gKp1] };
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=0.5, sqrtHalf2=0.5, sqrtHalf3=0.5 → effective=1.5; denDeg=2 > 1.5 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√(k³)·√(k³)·log(k)⁴/k² refused (effective=4.5, denDeg=2 not > 4.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK76b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp176b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK76b = { kind: "apply" as const, head: DIV, args: [numK76b, k2] };
    const gKp176b = { kind: "apply" as const, head: DIV, args: [numKp176b, kp1_2] };
    const f76b = { kind: "apply" as const, head: SUB, args: [gK76b, gKp176b] };
    const result = evaluateSum(f76b, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf×3=4.5; denDeg=2 not > 4.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k)⁴·k/k² refused (effective=2.5, denDeg=2 not > 2.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK76r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp176r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK76r = { kind: "apply" as const, head: DIV, args: [numK76r, k2] };
    const gKp176r = { kind: "apply" as const, head: DIV, args: [numKp176r, kp1_2] };
    const f76r = { kind: "apply" as const, head: SUB, args: [gK76r, gKp176r] };
    const result = evaluateSum(f76r, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=sqrtHalf2=sqrtHalf3=0.5, polyDeg=1 → effective=2.5; denDeg=2 not > 2.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 77 — Five-Log × polynomial numerator", () => {
  it("log(k)⁵/k² closes (effective=0, denDeg=2 > 0)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK77a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp177a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK77a = { kind: "apply" as const, head: DIV, args: [numK77a, k2] };
    const gKp177a = { kind: "apply" as const, head: DIV, args: [numKp177a, kp1_2] };
    const f77a = { kind: "apply" as const, head: SUB, args: [gK77a, gKp177a] };
    const result = evaluateSum(f77a, k, int(1), sym("%inf"), evalNode);
    // log⁵ sub-poly → effective=0; denDeg=2 > 0 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)⁵·k²/k³ closes (effective=2, denDeg=3 > 2)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK77b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k2,
    ]};
    const numKp177b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1_2,
    ]};
    const gK77b = { kind: "apply" as const, head: DIV, args: [numK77b, k3] };
    const gKp177b = { kind: "apply" as const, head: DIV, args: [numKp177b, kp1_3] };
    const f77b = { kind: "apply" as const, head: SUB, args: [gK77b, gKp177b] };
    const result = evaluateSum(f77b, k, int(1), sym("%inf"), evalNode);
    // polyDeg=2 → effective=2; denDeg=3 > 2 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)⁵·k³/k³ refused (effective=3, denDeg=3 not > 3)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK77r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k3,
    ]};
    const numKp177r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1_3,
    ]};
    const gK77r = { kind: "apply" as const, head: DIV, args: [numK77r, k3] };
    const gKp177r = { kind: "apply" as const, head: DIV, args: [numKp177r, kp1_3] };
    const f77r = { kind: "apply" as const, head: SUB, args: [gK77r, gKp177r] };
    const result = evaluateSum(f77r, k, int(1), sym("%inf"), evalNode);
    // polyDeg=3 → effective=3; denDeg=3 not > 3 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 78 — One-Sqrt × Five-Log × polynomial numerator", () => {
  it("√k·log(k)⁵/k² closes (effective=0.5, denDeg=2 > 0.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK78a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp178a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK78a = { kind: "apply" as const, head: DIV, args: [numK78a, k2] };
    const gKp178a = { kind: "apply" as const, head: DIV, args: [numKp178a, kp1_2] };
    const f78a = { kind: "apply" as const, head: SUB, args: [gK78a, gKp178a] };
    const result = evaluateSum(f78a, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf=0.5, log⁵ sub-poly → effective=0.5; denDeg=2 > 0.5 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·log(k)⁵/k refused (effective=1.5, denDeg=1 not > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK78b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp178b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK78b = { kind: "apply" as const, head: DIV, args: [numK78b, k] };
    const gKp178b = { kind: "apply" as const, head: DIV, args: [numKp178b, kp1] };
    const f78b = { kind: "apply" as const, head: SUB, args: [gK78b, gKp178b] };
    const result = evaluateSum(f78b, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf=1.5; denDeg=1 not > 1.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·log(k)⁵·k/k refused (effective=1.5, denDeg=1 not > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK78r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp178r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK78r = { kind: "apply" as const, head: DIV, args: [numK78r, k] };
    const gKp178r = { kind: "apply" as const, head: DIV, args: [numKp178r, kp1] };
    const f78r = { kind: "apply" as const, head: SUB, args: [gK78r, gKp178r] };
    const result = evaluateSum(f78r, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf=0.5, polyDeg=1 → effective=1.5; denDeg=1 not > 1.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 79 — Two-Sqrt × Five-Log × polynomial numerator", () => {
  it("√k·√k·log(k)⁵/k² closes (effective=1, denDeg=2 > 1)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK79a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp179a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK79a = { kind: "apply" as const, head: DIV, args: [numK79a, k2] };
    const gKp179a = { kind: "apply" as const, head: DIV, args: [numKp179a, kp1_2] };
    const f79a = { kind: "apply" as const, head: SUB, args: [gK79a, gKp179a] };
    const result = evaluateSum(f79a, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=0.5, sqrtHalf2=0.5, polyDeg=0 → effective=1; denDeg=2 > 1 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√(k³)·log(k)⁵/k refused (effective=3, denDeg=1 not > 3)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK79b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp179b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK79b = { kind: "apply" as const, head: DIV, args: [numK79b, k] };
    const gKp179b = { kind: "apply" as const, head: DIV, args: [numKp179b, kp1] };
    const f79b = { kind: "apply" as const, head: SUB, args: [gK79b, gKp179b] };
    const result = evaluateSum(f79b, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=1.5, sqrtHalf2=1.5 → effective=3; denDeg=1 not > 3 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·log(k)⁵·k/k² refused (effective=2, denDeg=2 not > 2)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK79c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp179c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK79c = { kind: "apply" as const, head: DIV, args: [numK79c, k2] };
    const gKp179c = { kind: "apply" as const, head: DIV, args: [numKp179c, kp1_2] };
    const f79c = { kind: "apply" as const, head: SUB, args: [gK79c, gKp179c] };
    const result = evaluateSum(f79c, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=0.5, sqrtHalf2=0.5, polyDeg=1 → effective=2; denDeg=2 not > 2 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 80 — Three-Sqrt × Five-Log × polynomial numerator", () => {
  it("√k·√k·√k·log(k)⁵/k² closes (effective=1.5, denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK80a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp180a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK80a = { kind: "apply" as const, head: DIV, args: [numK80a, k2] };
    const gKp180a = { kind: "apply" as const, head: DIV, args: [numKp180a, kp1_2] };
    const f80a = { kind: "apply" as const, head: SUB, args: [gK80a, gKp180a] };
    const result = evaluateSum(f80a, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=0.5, sqrtHalf2=0.5, sqrtHalf3=0.5, polyDeg=0 → effective=1.5; denDeg=2 > 1.5 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√(k³)·√(k³)·log(k)⁵/k refused (effective=4.5, denDeg=1 not > 4.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK80b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp180b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK80b = { kind: "apply" as const, head: DIV, args: [numK80b, k] };
    const gKp180b = { kind: "apply" as const, head: DIV, args: [numKp180b, kp1] };
    const f80b = { kind: "apply" as const, head: SUB, args: [gK80b, gKp180b] };
    const result = evaluateSum(f80b, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=1.5, sqrtHalf2=1.5, sqrtHalf3=1.5 → effective=4.5; denDeg=1 not > 4.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k)⁵·k/k² refused (effective=2.5, denDeg=2 not > 2.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK80c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp180c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK80c = { kind: "apply" as const, head: DIV, args: [numK80c, k2] };
    const gKp180c = { kind: "apply" as const, head: DIV, args: [numKp180c, kp1_2] };
    const f80c = { kind: "apply" as const, head: SUB, args: [gK80c, gKp180c] };
    const result = evaluateSum(f80c, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=0.5, sqrtHalf2=0.5, sqrtHalf3=0.5, polyDeg=1 → effective=2.5; denDeg=2 not > 2.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 81 — Four-Sqrt × Five-Log × polynomial numerator", () => {
  it("√k·√k·√k·√k·log(k)⁵/k³ closes (effective=2, denDeg=3 > 2)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK81a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp181a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK81a = { kind: "apply" as const, head: DIV, args: [numK81a, k3] };
    const gKp181a = { kind: "apply" as const, head: DIV, args: [numKp181a, kp1_3] };
    const f81a = { kind: "apply" as const, head: SUB, args: [gK81a, gKp181a] };
    const result = evaluateSum(f81a, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=0.5, sqrtHalf2=0.5, sqrtHalf3=0.5, sqrtHalf4=0.5, polyDeg=0 → effective=2; denDeg=3 > 2 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√(k³)·√(k³)·√(k³)·log(k)⁵/k refused (effective=6, denDeg=1 not > 6)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK81b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp181b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK81b = { kind: "apply" as const, head: DIV, args: [numK81b, k] };
    const gKp181b = { kind: "apply" as const, head: DIV, args: [numKp181b, kp1] };
    const f81b = { kind: "apply" as const, head: SUB, args: [gK81b, gKp181b] };
    const result = evaluateSum(f81b, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=1.5, sqrtHalf2=1.5, sqrtHalf3=1.5, sqrtHalf4=1.5 → effective=6; denDeg=1 not > 6 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·√k·log(k)⁵·k/k³ refused (effective=3, denDeg=3 not > 3)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK81c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp181c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK81c = { kind: "apply" as const, head: DIV, args: [numK81c, k3] };
    const gKp181c = { kind: "apply" as const, head: DIV, args: [numKp181c, kp1_3] };
    const f81c = { kind: "apply" as const, head: SUB, args: [gK81c, gKp181c] };
    const result = evaluateSum(f81c, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf1=0.5, sqrtHalf2=0.5, sqrtHalf3=0.5, sqrtHalf4=0.5, polyDeg=1 → effective=3; denDeg=3 not > 3 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 82 — Five-Sqrt × Five-Log × polynomial numerator", () => {
  it("√k·√k·√k·√k·√k·log(k)⁵/k³ closes (effective=2.5, denDeg=3 > 2.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK82a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp182a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK82a = { kind: "apply" as const, head: DIV, args: [numK82a, k3] };
    const gKp182a = { kind: "apply" as const, head: DIV, args: [numKp182a, kp1_3] };
    const f82a = { kind: "apply" as const, head: SUB, args: [gK82a, gKp182a] };
    const result = evaluateSum(f82a, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf×5=0.5×5=2.5, polyDeg=0 → effective=2.5; denDeg=3 > 2.5 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√(k³)·√(k³)·√(k³)·√(k³)·log(k)⁵/k refused (effective=7.5, denDeg=1 not > 7.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK82b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp182b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK82b = { kind: "apply" as const, head: DIV, args: [numK82b, k] };
    const gKp182b = { kind: "apply" as const, head: DIV, args: [numKp182b, kp1] };
    const f82b = { kind: "apply" as const, head: SUB, args: [gK82b, gKp182b] };
    const result = evaluateSum(f82b, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf×5=1.5×5=7.5 → effective=7.5; denDeg=1 not > 7.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·√k·√k·log(k)⁵·k/k⁴ closes (effective=3.5, denDeg=4 > 3.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k4 = { kind: "apply" as const, head: POW, args: [k, int(4)] };
    const kp1_4 = { kind: "apply" as const, head: POW, args: [kp1, int(4)] };
    const numK82c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp182c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK82c = { kind: "apply" as const, head: DIV, args: [numK82c, k4] };
    const gKp182c = { kind: "apply" as const, head: DIV, args: [numKp182c, kp1_4] };
    const f82c = { kind: "apply" as const, head: SUB, args: [gK82c, gKp182c] };
    const result = evaluateSum(f82c, k, int(1), sym("%inf"), evalNode);
    // sqrtHalf×5=0.5×5=2.5, polyDeg=1 → effective=3.5; denDeg=4 > 3.5 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 83 — Six-Log × polynomial numerator", () => {
  it("log(k)⁶/k² closes (effective=0, denDeg=2 > 0)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK83a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp183a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK83a = { kind: "apply" as const, head: DIV, args: [numK83a, k2] };
    const gKp183a = { kind: "apply" as const, head: DIV, args: [numKp183a, kp1_2] };
    const f83a = { kind: "apply" as const, head: SUB, args: [gK83a, gKp183a] };
    const result = evaluateSum(f83a, k, int(1), sym("%inf"), evalNode);
    // log⁶ sub-poly → effective=0; denDeg=2 > 0 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)⁶·k²/k³ closes (effective=2, denDeg=3 > 2)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK83b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k2,
    ]};
    const numKp183b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1_2,
    ]};
    const gK83b = { kind: "apply" as const, head: DIV, args: [numK83b, k3] };
    const gKp183b = { kind: "apply" as const, head: DIV, args: [numKp183b, kp1_3] };
    const f83b = { kind: "apply" as const, head: SUB, args: [gK83b, gKp183b] };
    const result = evaluateSum(f83b, k, int(1), sym("%inf"), evalNode);
    // polyDeg=2 → effective=2; denDeg=3 > 2 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)⁶·k³/k³ refused (effective=3, denDeg=3 not > 3)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK83r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k3,
    ]};
    const numKp183r = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1_3,
    ]};
    const gK83r = { kind: "apply" as const, head: DIV, args: [numK83r, k3] };
    const gKp183r = { kind: "apply" as const, head: DIV, args: [numKp183r, kp1_3] };
    const f83r = { kind: "apply" as const, head: SUB, args: [gK83r, gKp183r] };
    const result = evaluateSum(f83r, k, int(1), sym("%inf"), evalNode);
    // polyDeg=3 → effective=3; denDeg=3 not > 3 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 84 — One-Sqrt × Six-Log × polynomial numerator", () => {
  it("√k·log(k)⁶/k² closes (sqrtHalf=0.5, polyDeg=0 → effective=0.5; denDeg=2 > 0.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK84a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp184a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK84a = { kind: "apply" as const, head: DIV, args: [numK84a, k2] };
    const gKp184a = { kind: "apply" as const, head: DIV, args: [numKp184a, kp1_2] };
    const f84a = { kind: "apply" as const, head: SUB, args: [gK84a, gKp184a] };
    const result = evaluateSum(f84a, k, int(1), sym("%inf"), evalNode);
    // effective=0.5; denDeg=2 > 0.5 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·log(k)⁶·k/k² closes (sqrtHalf=0.5, polyDeg=1 → effective=1.5; denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK84b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp184b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK84b = { kind: "apply" as const, head: DIV, args: [numK84b, k2] };
    const gKp184b = { kind: "apply" as const, head: DIV, args: [numKp184b, kp1_2] };
    const f84b = { kind: "apply" as const, head: SUB, args: [gK84b, gKp184b] };
    const result = evaluateSum(f84b, k, int(1), sym("%inf"), evalNode);
    // effective=1.5; denDeg=2 > 1.5 → closes
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·log(k)⁶·k/k refused (sqrtHalf=0.5, polyDeg=1 → effective=1.5; denDeg=1 not > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK84c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp184c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK84c = { kind: "apply" as const, head: DIV, args: [numK84c, k] };
    const gKp184c = { kind: "apply" as const, head: DIV, args: [numKp184c, kp1] };
    const f84c = { kind: "apply" as const, head: SUB, args: [gK84c, gKp184c] };
    const result = evaluateSum(f84c, k, int(1), sym("%inf"), evalNode);
    // effective=1.5; denDeg=1 not > 1.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 89 — Seven-Log × polynomial numerator", () => {
  // log(k)^7 · poly(k) / denom — effective degree = polyDeg (log^7 sub-polynomial).

  it("log(k)^7 / k² closes (eff=0; denDeg=2 > 0)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK89a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp189a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK89a = { kind: "apply" as const, head: DIV, args: [numK89a, k2] };
    const gKp189a = { kind: "apply" as const, head: DIV, args: [numKp189a, kp1_2] };
    const f89a = { kind: "apply" as const, head: SUB, args: [gK89a, gKp189a] };
    const result = evaluateSum(f89a, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)^7·k² / k³ closes (eff=2; denDeg=3 > 2)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK89b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k2,
    ]};
    const numKp189b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1_2,
    ]};
    const gK89b = { kind: "apply" as const, head: DIV, args: [numK89b, k3] };
    const gKp189b = { kind: "apply" as const, head: DIV, args: [numKp189b, kp1_3] };
    const f89b = { kind: "apply" as const, head: SUB, args: [gK89b, gKp189b] };
    const result = evaluateSum(f89b, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)^7·k³ / k³ refused (eff=3; denDeg=3 not > 3)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK89c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k3,
    ]};
    const numKp189c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      k3,
    ]};
    const gK89c = { kind: "apply" as const, head: DIV, args: [numK89c, k3] };
    const gKp189c = { kind: "apply" as const, head: DIV, args: [numKp189c, kp1_3] };
    const f89c = { kind: "apply" as const, head: SUB, args: [gK89c, gKp189c] };
    const result = evaluateSum(f89c, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 90 — One-Sqrt × Seven-Log × polynomial numerator", () => {
  // √(P(k)) · log(k)^7 · poly(k) / denom — effective degree = sqrtHalfDeg + polyDeg.

  it("√k·log(k)^7 / k² closes (eff=0.5; denDeg=2 > 0.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK90a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp190a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK90a = { kind: "apply" as const, head: DIV, args: [numK90a, k2] };
    const gKp190a = { kind: "apply" as const, head: DIV, args: [numKp190a, kp1_2] };
    const f90a = { kind: "apply" as const, head: SUB, args: [gK90a, gKp190a] };
    const result = evaluateSum(f90a, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·log(k)^7·k / k² closes (eff=1.5; denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK90b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp190b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK90b = { kind: "apply" as const, head: DIV, args: [numK90b, k2] };
    const gKp190b = { kind: "apply" as const, head: DIV, args: [numKp190b, kp1_2] };
    const f90b = { kind: "apply" as const, head: SUB, args: [gK90b, gKp190b] };
    const result = evaluateSum(f90b, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·log(k)^7·k / k refused (eff=1.5; denDeg=1 not > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK90c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp190c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK90c = { kind: "apply" as const, head: DIV, args: [numK90c, k] };
    const gKp190c = { kind: "apply" as const, head: DIV, args: [numKp190c, kp1] };
    const f90c = { kind: "apply" as const, head: SUB, args: [gK90c, gKp190c] };
    const result = evaluateSum(f90c, k, int(1), sym("%inf"), evalNode);
    // effective=1.5; denDeg=1 not > 1.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 91 — Two-Sqrt × Seven-Log × polynomial numerator", () => {
  // √(P1(k)) · √(P2(k)) · log(k)^7 · poly(k) / denom — effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.

  it("√k·√k·log(k)^7 / k² closes (eff=1; denDeg=2 > 1)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK91a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp191a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK91a = { kind: "apply" as const, head: DIV, args: [numK91a, k2] };
    const gKp191a = { kind: "apply" as const, head: DIV, args: [numKp191a, kp1_2] };
    const f91a = { kind: "apply" as const, head: SUB, args: [gK91a, gKp191a] };
    const result = evaluateSum(f91a, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√(k³)·log(k)^7 / k⁴ closes (eff=3; denDeg=4 > 3)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const k4 = { kind: "apply" as const, head: POW, args: [k, int(4)] };
    const kp1_4 = { kind: "apply" as const, head: POW, args: [kp1, int(4)] };
    const numK91b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp191b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK91b = { kind: "apply" as const, head: DIV, args: [numK91b, k4] };
    const gKp191b = { kind: "apply" as const, head: DIV, args: [numKp191b, kp1_4] };
    const f91b = { kind: "apply" as const, head: SUB, args: [gK91b, gKp191b] };
    const result = evaluateSum(f91b, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·log(k)^7·k / k² refused (eff=2; denDeg=2 not > 2)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK91c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp191c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK91c = { kind: "apply" as const, head: DIV, args: [numK91c, k2] };
    const gKp191c = { kind: "apply" as const, head: DIV, args: [numKp191c, kp1_2] };
    const f91c = { kind: "apply" as const, head: SUB, args: [gK91c, gKp191c] };
    const result = evaluateSum(f91c, k, int(1), sym("%inf"), evalNode);
    // effective=2; denDeg=2 not > 2 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 92 — Three-Sqrt × Seven-Log × polynomial numerator", () => {
  // √(P1(k)) · √(P2(k)) · √(P3(k)) · log(k)^7 · poly(k) / denom

  it("√k·√k·√k·log(k)^7 / k² closes (eff=1.5; denDeg=2 > 1.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK92a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp192a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK92a = { kind: "apply" as const, head: DIV, args: [numK92a, k2] };
    const gKp192a = { kind: "apply" as const, head: DIV, args: [numKp192a, kp1_2] };
    const f92a = { kind: "apply" as const, head: SUB, args: [gK92a, gKp192a] };
    const result = evaluateSum(f92a, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k³)·√(k³)·√(k³)·log(k)^7 / k⁵ closes (eff=4.5; denDeg=5 > 4.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const k5 = { kind: "apply" as const, head: POW, args: [k, int(5)] };
    const kp1_5 = { kind: "apply" as const, head: POW, args: [kp1, int(5)] };
    const numK92b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: SQRT, args: [k3] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp192b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: SQRT, args: [kp1_3] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK92b = { kind: "apply" as const, head: DIV, args: [numK92b, k5] };
    const gKp192b = { kind: "apply" as const, head: DIV, args: [numKp192b, kp1_5] };
    const f92b = { kind: "apply" as const, head: SUB, args: [gK92b, gKp192b] };
    const result = evaluateSum(f92b, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k·√k·√k·log(k)^7·k / k² refused (eff=2.5; denDeg=2 not > 2.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK92c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp192c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK92c = { kind: "apply" as const, head: DIV, args: [numK92c, k2] };
    const gKp192c = { kind: "apply" as const, head: DIV, args: [numKp192c, kp1_2] };
    const f92c = { kind: "apply" as const, head: SUB, args: [gK92c, gKp192c] };
    const result = evaluateSum(f92c, k, int(1), sym("%inf"), evalNode);
    // effective=2.5; denDeg=2 not > 2.5 → refused
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 93 — Four-Sqrt × Seven-Log × polynomial numerator", () => {
  it("√k×4·log(k)^7 / k³ closes (eff=2; denDeg=3 > 2)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK93a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp193a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK93a = { kind: "apply" as const, head: DIV, args: [numK93a, k3] };
    const gKp193a = { kind: "apply" as const, head: DIV, args: [numKp193a, kp1_3] };
    const f93a = { kind: "apply" as const, head: SUB, args: [gK93a, gKp193a] };
    const result = evaluateSum(f93a, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k²)×4·log(k)^7 / k⁵ closes (eff=4; denDeg=5 > 4)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const k5 = { kind: "apply" as const, head: POW, args: [k, int(5)] };
    const kp1_5 = { kind: "apply" as const, head: POW, args: [kp1, int(5)] };
    const numK93b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp193b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK93b = { kind: "apply" as const, head: DIV, args: [numK93b, k5] };
    const gKp193b = { kind: "apply" as const, head: DIV, args: [numKp193b, kp1_5] };
    const f93b = { kind: "apply" as const, head: SUB, args: [gK93b, gKp193b] };
    const result = evaluateSum(f93b, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k×4·log(k)^7·k / k³ refused (eff=3; denDeg=3 not > 3)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK93c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp193c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK93c = { kind: "apply" as const, head: DIV, args: [numK93c, k3] };
    const gKp193c = { kind: "apply" as const, head: DIV, args: [numKp193c, kp1_3] };
    const f93c = { kind: "apply" as const, head: SUB, args: [gK93c, gKp193c] };
    const result = evaluateSum(f93c, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 94 — Five-Sqrt × Seven-Log × polynomial numerator", () => {
  it("√k×5·log(k)^7 / k⁴ closes (eff=2.5; denDeg=4 > 2.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k4 = { kind: "apply" as const, head: POW, args: [k, int(4)] };
    const kp1_4 = { kind: "apply" as const, head: POW, args: [kp1, int(4)] };
    const numK94a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp194a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK94a = { kind: "apply" as const, head: DIV, args: [numK94a, k4] };
    const gKp194a = { kind: "apply" as const, head: DIV, args: [numKp194a, kp1_4] };
    const f94a = { kind: "apply" as const, head: SUB, args: [gK94a, gKp194a] };
    const result = evaluateSum(f94a, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√(k²)×5·log(k)^7 / k⁶ closes (eff=5; denDeg=6 > 5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const k6 = { kind: "apply" as const, head: POW, args: [k, int(6)] };
    const kp1_6 = { kind: "apply" as const, head: POW, args: [kp1, int(6)] };
    const numK94b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: SQRT, args: [k2] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp194b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: SQRT, args: [kp1_2] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK94b = { kind: "apply" as const, head: DIV, args: [numK94b, k6] };
    const gKp194b = { kind: "apply" as const, head: DIV, args: [numKp194b, kp1_6] };
    const f94b = { kind: "apply" as const, head: SUB, args: [gK94b, gKp194b] };
    const result = evaluateSum(f94b, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("√k×5·log(k)^7·k / k³ refused (eff=3.5; denDeg=3 not > 3.5)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK94c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: SQRT, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp194c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: SQRT, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK94c = { kind: "apply" as const, head: DIV, args: [numK94c, k3] };
    const gKp194c = { kind: "apply" as const, head: DIV, args: [numKp194c, kp1_3] };
    const f94c = { kind: "apply" as const, head: SUB, args: [gK94c, gKp194c] };
    const result = evaluateSum(f94c, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});

describe("Phase 95 — Eight-Log × polynomial numerator", () => {
  it("log(k)^8 / k² closes (eff=0; denDeg=2 > 0)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k2 = { kind: "apply" as const, head: POW, args: [k, int(2)] };
    const kp1_2 = { kind: "apply" as const, head: POW, args: [kp1, int(2)] };
    const numK95a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
    ]};
    const numKp195a = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
    ]};
    const gK95a = { kind: "apply" as const, head: DIV, args: [numK95a, k2] };
    const gKp195a = { kind: "apply" as const, head: DIV, args: [numKp195a, kp1_2] };
    const f95a = { kind: "apply" as const, head: SUB, args: [gK95a, gKp195a] };
    const result = evaluateSum(f95a, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)^8·k / k³ closes (eff=1; denDeg=3 > 1)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const k3 = { kind: "apply" as const, head: POW, args: [k, int(3)] };
    const kp1_3 = { kind: "apply" as const, head: POW, args: [kp1, int(3)] };
    const numK95b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp195b = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK95b = { kind: "apply" as const, head: DIV, args: [numK95b, k3] };
    const gKp195b = { kind: "apply" as const, head: DIV, args: [numKp195b, kp1_3] };
    const f95b = { kind: "apply" as const, head: SUB, args: [gK95b, gKp195b] };
    const result = evaluateSum(f95b, k, int(1), sym("%inf"), evalNode);
    expect(result).not.toMatchObject({ kind: "apply", head: SUM });
  });

  it("log(k)^8·k / k refused (eff=1; denDeg=1 not > 1)", () => {
    const k = sym("k");
    const kp1 = { kind: "apply" as const, head: ADD, args: [k, int(1)] };
    const numK95c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      { kind: "apply" as const, head: LOG, args: [k] },
      k,
    ]};
    const numKp195c = { kind: "apply" as const, head: MUL, args: [
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      { kind: "apply" as const, head: LOG, args: [kp1] },
      kp1,
    ]};
    const gK95c = { kind: "apply" as const, head: DIV, args: [numK95c, k] };
    const gKp195c = { kind: "apply" as const, head: DIV, args: [numKp195c, kp1] };
    const f95c = { kind: "apply" as const, head: SUB, args: [gK95c, gKp195c] };
    const result = evaluateSum(f95c, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });
});
