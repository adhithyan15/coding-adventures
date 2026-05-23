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
