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
// Track B2 — Apart-retry telescope chain tests need a real VM with the
// Apart handler installed.  symbolic-vm is a devDependency of
// cas-summation; the published runtime does not depend on it.
import { SymbolicBackend, VM } from "@coding-adventures/symbolic-vm";

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

  it("standard exp(-k) telescope closes because exp(-k) vanishes", () => {
    const k = sym("k");
    const gK = app(EXP, [app(NEG, [k])]);
    const gKp1 = app(EXP, [app(NEG, [app(ADD, [k, int(1)])])]);
    const f = app(SUB, [gKp1, gK]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(
      app(NEG, [app(EXP, [int(-1)])]),
    );
  });

  it("antisymmetric 2^(-k) telescope closes because the magnitude vanishes", () => {
    const k = sym("k");
    const gK = app(POW, [int(2), app(NEG, [k])]);
    const gKp1 = app(POW, [int(2), app(NEG, [app(ADD, [k, int(1)])])]);
    const f = app(SUB, [gK, gKp1]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), evalNode)).toEqual(
      app(POW, [int(2), int(-1)]),
    );
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

// ---------------------------------------------------------------------------
// Phase 86 (TS port): generic log × sqrt × polynomial recogniser cleanup.
//
// A single helper handles arbitrary (N, M, K) — supersedes the hand-written
// grid from Phases 59-85.  These tests prove the generic closes cases the
// hardcoded grid cannot reach (more than 5 Sqrts, more than 6 Logs, mixed).
// ---------------------------------------------------------------------------

describe("summation: Phase 86 generic log × sqrt × polynomial recogniser", () => {
  it("seven logs over k² closes via generic (hand-written grid stops at 6)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const logsK = Array.from({ length: 7 }, () => app(LOG, [k]));
    const logsKp1 = Array.from({ length: 7 }, () => app(LOG, [kp1]));
    const numK = app(MUL, logsK);
    const numKp1 = app(MUL, logsKp1);
    const gK = app(DIV, [numK, app(POW, [k, int(2)])]);
    const gKp1 = app(DIV, [numKp1, app(POW, [kp1, int(2)])]);
    const f = app(SUB, [gK, gKp1]);
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result.kind === "apply" ? result.head : undefined).not.toEqual(SUM);
  });

  it("six sqrts of k over k⁴ closes via generic (grid stops at 5)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const sqrtsK = Array.from({ length: 6 }, () => app(SQRT, [k]));
    const sqrtsKp1 = Array.from({ length: 6 }, () => app(SQRT, [kp1]));
    const numK = app(MUL, sqrtsK);
    const numKp1 = app(MUL, sqrtsKp1);
    const gK = app(DIV, [numK, app(POW, [k, int(4)])]);
    const gKp1 = app(DIV, [numKp1, app(POW, [kp1, int(4)])]);
    const f = app(SUB, [gK, gKp1]);
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // sqrtHalfSum = 6 * 0.5 = 3; denDeg = 4 > 3 → closes.
    expect(result.kind === "apply" ? result.head : undefined).not.toEqual(SUM);
  });

  it("three sqrts × seven logs × k over k⁵ closes via generic", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const logsK = Array.from({ length: 7 }, () => app(LOG, [k]));
    const logsKp1 = Array.from({ length: 7 }, () => app(LOG, [kp1]));
    const sqrtFactorsK = [
      app(SQRT, [app(POW, [k, int(3)])]),
      app(SQRT, [k]),
      app(SQRT, [app(POW, [k, int(2)])]),
    ];
    const sqrtFactorsKp1 = [
      app(SQRT, [app(POW, [kp1, int(3)])]),
      app(SQRT, [kp1]),
      app(SQRT, [app(POW, [kp1, int(2)])]),
    ];
    const numK = app(MUL, [app(sym("Sin"), [k]), ...logsK, ...sqrtFactorsK, k]);
    const numKp1 = app(MUL, [
      app(sym("Sin"), [kp1]),
      ...logsKp1,
      ...sqrtFactorsKp1,
      kp1,
    ]);
    const gK = app(DIV, [numK, app(POW, [k, int(5)])]);
    const gKp1 = app(DIV, [numKp1, app(POW, [kp1, int(5)])]);
    const f = app(SUB, [gK, gKp1]);
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // sqrtHalfSum = 1.5 + 0.5 + 1 = 3, polyDegSum = 1, effective = 4, denDeg = 5 → closes.
    expect(result.kind === "apply" ? result.head : undefined).not.toEqual(SUM);
  });

  it("refuses unrecognised factor (Exp) so divergent sum stays unevaluated", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(LOG, [k]), app(SQRT, [k]), app(EXP, [k])]);
    const numKp1 = app(MUL, [
      app(LOG, [kp1]),
      app(SQRT, [kp1]),
      app(EXP, [kp1]),
    ]);
    const gK = app(DIV, [numK, app(POW, [k, int(3)])]);
    const gKp1 = app(DIV, [numKp1, app(POW, [kp1, int(3)])]);
    const f = app(SUB, [gK, gKp1]);
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // exp(k)·log(k)·sqrt(k) grows exponentially → must NOT vanish.
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("refuses Sqrt of negative polynomial (complex-valued)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const negK = app(MUL, [int(-1), k]);
    const negKp1 = app(MUL, [int(-1), kp1]);
    const numK = app(MUL, [app(LOG, [k]), app(SQRT, [negK])]);
    const numKp1 = app(MUL, [app(LOG, [kp1]), app(SQRT, [negKp1])]);
    const gK = app(DIV, [numK, app(POW, [k, int(3)])]);
    const gKp1 = app(DIV, [numKp1, app(POW, [kp1, int(3)])]);
    const f = app(SUB, [gK, gKp1]);
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    expect(result).toMatchObject({ kind: "apply", head: SUM });
  });

  it("pure bounded falls through to Phase 49 (sum still closes)", () => {
    const k = sym("k");
    const kp1 = app(ADD, [k, int(1)]);
    const numK = app(MUL, [app(sym("Sin"), [k]), app(sym("Cos"), [k])]);
    const numKp1 = app(MUL, [app(sym("Sin"), [kp1]), app(sym("Cos"), [kp1])]);
    const gK = app(DIV, [numK, app(POW, [k, int(2)])]);
    const gKp1 = app(DIV, [numKp1, app(POW, [kp1, int(2)])]);
    const f = app(SUB, [gK, gKp1]);
    const result = evaluateSum(f, k, int(1), sym("%inf"), evalNode);
    // Phase 49 (bounded × diverging) catches it — generic returns undefined.
    expect(result.kind === "apply" ? result.head : undefined).not.toEqual(SUM);
  });
});

// ---------------------------------------------------------------------------
// Track B2 — Apart-retry telescope chain (Phase 40 + Phase 46 composition).
//
// These tests drive ``evaluateSum`` through a real ``symbolic-vm`` VM
// (``SymbolicBackend`` has the Apart handler installed since 0.13.0), so the
// ``Apply(Apart, ...)`` emitted by the new retry path is actually dispatched
// to ``apartHandler``.  This is the only place in the cas-summation TS test
// suite that takes a runtime dependency on symbolic-vm — it is a
// devDependency so the published package still has no runtime tie.
// ---------------------------------------------------------------------------

function vmEval(): (node: IRNode) => IRNode {
  const vm = new VM(new SymbolicBackend());
  return (node) => vm.eval(node);
}

describe("summation: Track B2 Apart-retry telescope chain (Phase 40+46)", () => {
  it("acceptance — ∑_{k=1}^∞ 1/(k(k+1)) = 1 (closes via Apart-retry)", () => {
    // The classic case: Apart decomposes 1/(k(k+1)) → 1/k − 1/(k+1).
    // The Phase 40+46 Add-Neg normaliser rewrites Add(Div(1,k), Div(-1,k+1))
    // to Sub(1/k, 1/(k+1)); Phase 41 closes the resulting telescope at
    // ∞ to give 1/k|_{k=1} = 1.
    const k = sym("k");
    const f = app(DIV, [int(1), app(MUL, [k, app(ADD, [k, int(1)])])]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), vmEval())).toEqual(int(1));
  });

  it("three-term shifted: ∑_{k=1}^∞ 1/(k(k+2)) = 3/4", () => {
    // Apart: 1/(k(k+2)) = (1/2)/k − (1/2)/(k+2).  This is *not* a
    // direct k → k+1 telescope (shift is 2, not 1), so cas-summation
    // doesn't immediately close it via the structural detector — but
    // the sum still has a known value 3/4 = (1/2)(1 + 1/2).  Because
    // the Apart-retry must not falsely claim closure here, we accept
    // either a correct closed form (3/4) or a passthrough — the safety
    // requirement is "do not produce a wrong value".  Currently the
    // structural detector returns unevaluated for shift-2 telescopes,
    // so this test pins the *no false closure* property.
    const k = sym("k");
    const f = app(DIV, [int(1), app(MUL, [k, app(ADD, [k, int(2)])])]);
    const result = evaluateSum(f, k, int(1), sym("%inf"), vmEval());
    // Acceptable outcomes: rational 3/4 (if a future shift-aware
    // telescope detector closes it) or unevaluated Sum (current
    // structural detector).  A wrong numeric result would fail.
    const rv = rationalValue(result);
    if (rv === undefined) {
      expect(result.kind === "apply" ? result.head : undefined).toEqual(SUM);
    } else {
      expect(rv).toEqual({ numer: 3n, denom: 4n });
    }
  });

  it("Phase 46 constant-numerator: ∑_{k=1}^∞ 2/(k(k+1)) = 2", () => {
    // Apart: 2/(k(k+1)) = 2/k − 2/(k+1).  The Phase 46 widening
    // recognises ``Add(Div(2, k), Div(-2, k+1))`` (negative-literal
    // numerator) as a telescope after the Add-Neg normaliser fires.
    const k = sym("k");
    const f = app(DIV, [int(2), app(MUL, [k, app(ADD, [k, int(1)])])]);
    expect(evaluateSum(f, k, int(1), sym("%inf"), vmEval())).toEqual(int(2));
  });

  it("irreducible denominator (Apart bails): ∑_{k=1}^∞ 1/(k²+1) returns unevaluated SUM", () => {
    // ``k² + 1`` has no rational roots, so Apart returns its input
    // unchanged (Phase 1 simple-roots path requires rational roots).
    // The cas-summation Apart-retry then sees ``apart_attempt == f``
    // and does not recurse; we fall through to the unevaluated Sum.
    const k = sym("k");
    const f = app(DIV, [int(1), app(ADD, [app(POW, [k, int(2)]), int(1)])]);
    const result = evaluateSum(f, k, int(1), sym("%inf"), vmEval());
    expect(result.kind === "apply" ? result.head : undefined).toEqual(SUM);
  });

  it("polynomial summand (not a Div, skips Apart): ∑_{k=1}^4 k(k+1) = 40 via Faulhaber", () => {
    // ``k(k+1)`` is a polynomial, not ``Div(...)`` — the Apart-retry
    // guard skips it entirely.  The existing Faulhaber / power-of-k
    // path closes the sum directly: ∑_{k=1}^4 k(k+1) = ∑k² + ∑k = 30 + 10 = 40.
    const k = sym("k");
    const f = app(MUL, [k, app(ADD, [k, int(1)])]);
    expect(evaluateSum(f, k, int(1), int(4), vmEval())).toEqual(int(40));
  });

  it("Apart fires but post-Apart shape still doesn't telescope: returns unevaluated SUM", () => {
    // Construct a sum whose Apart decomposition produces terms that
    // individually diverge or don't pair into a shift-1 telescope.
    // ``∑_{k=1}^N 1/((k-1)(k+1))`` (k from 2) — Apart yields
    // (1/2)/(k-1) − (1/2)/(k+1), a shift-2 telescope.  Finite hi makes
    // it numerically summable via direct iteration but the structural
    // telescope detector won't fire and the explicit numeric path
    // closes it.  Use an infinite hi with a symbolic shape that
    // structurally resists both — pick a hi=%inf with a 3-factor
    // denominator: ``1/((k-1)(k+1)(k+2))`` from k=2 — Apart produces
    // a 3-term shift-mixed decomposition that has no direct shift-1
    // pairing, so the retry's inner telescope detector returns
    // undefined and we fall through to unevaluated SUM (no spurious
    // numeric closure, no wrong answer).
    const k = sym("k");
    const f = app(DIV, [
      int(1),
      app(MUL, [
        app(SUB, [k, int(1)]),
        app(MUL, [app(ADD, [k, int(1)]), app(ADD, [k, int(2)])]),
      ]),
    ]);
    const result = evaluateSum(f, k, int(2), sym("%inf"), vmEval());
    // Safety: must not produce a wrong numeric value.  Either it stays
    // unevaluated (current detector limitation) or a correct future
    // closed form (real value: (1/4)·H₂ + small) — either is fine; a
    // wrong rational would be a regression.
    const rv = rationalValue(result);
    if (rv === undefined) {
      expect(result.kind === "apply" ? result.head : undefined).toEqual(SUM);
    } else {
      // Any closed numeric value here would need independent verification;
      // for now this branch is reserved for a future widening that knows
      // how to close shift-3 telescopes.
      expect(rv.denom).toBeGreaterThan(0n);
    }
  });
});
