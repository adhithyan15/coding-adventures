/**
 * Tests for Gosper's algorithm — Track H2 (port of Python H1, PR #5366).
 *
 * These mirror the 14 acceptance + structural cases from
 * ``code/packages/python/cas-summation/tests/test_gosper.py``.  Each test
 * verifies the closed form against the direct numeric sum at several
 * concrete values of the free parameter ``N``.
 */
import { describe, expect, it } from "vitest";
import {
  ADD,
  DIV,
  MUL,
  NEG,
  POW,
  SUB,
  SUM,
  app,
  equals as irEquals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";
import {
  GAMMA_FUNC,
  MAX_POLY_DEGREE,
  evaluateSum,
  rationalValue,
  tryGosperSum,
  type RationalValue,
} from "../src/index";
import { __test } from "../src/gosper";

// ---------------------------------------------------------------------------
// Stub VM — evaluates arithmetic + reduces ``GammaFunc(integer)`` to the
// corresponding factorial so we can verify closed-form numerics.
// ---------------------------------------------------------------------------

function rationalToIr(value: RationalValue): IRNode {
  return value.denom === 1n ? int(value.numer) : rational(value.numer, value.denom);
}

function gcd(a: bigint, b: bigint): bigint {
  let x = a < 0n ? -a : a;
  let y = b < 0n ? -b : b;
  while (y !== 0n) {
    const t = y;
    y = x % y;
    x = t;
  }
  return x === 0n ? 1n : x;
}

function reduceR(value: RationalValue): RationalValue {
  let n = value.numer;
  let d = value.denom;
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  const g = gcd(n, d);
  return { numer: n / g, denom: d / g };
}

function addR(a: RationalValue, b: RationalValue): RationalValue {
  return reduceR({ numer: a.numer * b.denom + b.numer * a.denom, denom: a.denom * b.denom });
}
function subR(a: RationalValue, b: RationalValue): RationalValue {
  return reduceR({ numer: a.numer * b.denom - b.numer * a.denom, denom: a.denom * b.denom });
}
function mulR(a: RationalValue, b: RationalValue): RationalValue {
  return reduceR({ numer: a.numer * b.numer, denom: a.denom * b.denom });
}
function divR(a: RationalValue, b: RationalValue): RationalValue {
  return reduceR({ numer: a.numer * b.denom, denom: a.denom * b.numer });
}

function evalNode(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const args = node.args.map(evalNode);
  const head = node.head;
  if (head.kind !== "symbol") return app(head, args);
  // GammaFunc(IRInteger n ≥ 1) = (n-1)!
  if (head.name === GAMMA_FUNC.name && args.length === 1) {
    const a = args[0];
    if (a.kind === "integer" && a.value >= 1n) {
      let f = 1n;
      for (let i = 1n; i < a.value; i++) f *= i;
      return int(f);
    }
    return app(head, args);
  }
  switch (head.name) {
    case ADD.name: {
      let acc: RationalValue = { numer: 0n, denom: 1n };
      for (const a of args) {
        const v = rationalValue(a);
        if (v === undefined) return app(head, args);
        acc = addR(acc, v);
      }
      return rationalToIr(acc);
    }
    case SUB.name: {
      if (args.length !== 2) return app(head, args);
      const a = rationalValue(args[0]);
      const b = rationalValue(args[1]);
      if (a === undefined || b === undefined) return app(head, args);
      return rationalToIr(subR(a, b));
    }
    case MUL.name: {
      let acc: RationalValue = { numer: 1n, denom: 1n };
      for (const a of args) {
        const v = rationalValue(a);
        if (v === undefined) return app(head, args);
        acc = mulR(acc, v);
      }
      return rationalToIr(acc);
    }
    case DIV.name: {
      if (args.length !== 2) return app(head, args);
      const a = rationalValue(args[0]);
      const b = rationalValue(args[1]);
      if (a === undefined || b === undefined || b.numer === 0n) return app(head, args);
      return rationalToIr(divR(a, b));
    }
    case POW.name: {
      if (args.length !== 2) return app(head, args);
      const base = rationalValue(args[0]);
      if (base === undefined || args[1].kind !== "integer") return app(head, args);
      const exp = args[1].value;
      if (exp >= 0n) {
        let r: RationalValue = { numer: 1n, denom: 1n };
        for (let i = 0n; i < exp; i++) r = mulR(r, base);
        return rationalToIr(r);
      }
      if (base.numer === 0n) return app(head, args);
      let r: RationalValue = { numer: 1n, denom: 1n };
      for (let i = 0n; i < -exp; i++) r = mulR(r, base);
      return rationalToIr({ numer: r.denom, denom: r.numer });
    }
    case NEG.name: {
      if (args.length !== 1) return app(head, args);
      const a = rationalValue(args[0]);
      if (a === undefined) return app(head, args);
      return rationalToIr({ numer: -a.numer, denom: a.denom });
    }
    default:
      return app(head, args);
  }
}

const k = sym("k");
const N = sym("N");

function substitute(node: IRNode, from: IRNode, to: IRNode): IRNode {
  if (irEquals(node, from)) return to;
  if (node.kind !== "apply") return node;
  return app(node.head, node.args.map((a) => substitute(a, from, to)));
}

function evalAt(node: IRNode, symNode: IRNode, value: bigint): RationalValue | undefined {
  return rationalValue(evalNode(substitute(node, symNode, int(value))));
}

function rv(numer: bigint, denom: bigint = 1n): RationalValue {
  return reduceR({ numer, denom });
}

function eqR(a: RationalValue | undefined, b: RationalValue): boolean {
  if (a === undefined) return false;
  return a.numer === b.numer && a.denom === b.denom;
}

function factorialBig(n: bigint): bigint {
  let r = 1n;
  for (let i = 1n; i <= n; i++) r *= i;
  return r;
}

function makeKTimes2K(): IRNode {
  return app(MUL, [k, app(POW, [int(2), k])]);
}
function makeKTimesKFact(): IRNode {
  return app(MUL, [k, app(GAMMA_FUNC, [app(ADD, [k, int(1)])])]);
}

// ---------------------------------------------------------------------------
// Internal helper tests — polynomial primitives.
// ---------------------------------------------------------------------------

describe("gosper: poly helpers", () => {
  it("polyAdd basic: (1 + 2k) + (3 + k^2) = 4 + 2k + k^2", () => {
    const { polyAdd, mkF, fFromInt } = __test;
    const result = polyAdd(
      [fFromInt(1n), fFromInt(2n)],
      [fFromInt(3n), fFromInt(0n), fFromInt(1n)],
    );
    expect(result.map((f) => f.n)).toEqual([4n, 2n, 1n]);
  });
  it("polyMul basic: (1 + k)^2 = 1 + 2k + k^2", () => {
    const { polyMul, fFromInt } = __test;
    const result = polyMul([fFromInt(1n), fFromInt(1n)], [fFromInt(1n), fFromInt(1n)]);
    expect(result.map((f) => f.n)).toEqual([1n, 2n, 1n]);
  });
  it("polyShift basic: shift k^2 by +1 → 1 + 2k + k^2", () => {
    const { polyShift, fFromInt } = __test;
    const p = [fFromInt(0n), fFromInt(0n), fFromInt(1n)];
    const result = polyShift(p, 1n);
    expect(result.map((f) => f.n)).toEqual([1n, 2n, 1n]);
  });
  it("polyGcd basic: gcd(k^2 − 1, k − 1) = k − 1 (monic)", () => {
    const { polyGcd, fFromInt } = __test;
    const a = [fFromInt(-1n), fFromInt(0n), fFromInt(1n)];
    const b = [fFromInt(-1n), fFromInt(1n)];
    const g = polyGcd(a, b);
    expect(g.map((f) => `${f.n}/${f.d}`)).toEqual(["-1/1", "1/1"]);
  });
});

// ---------------------------------------------------------------------------
// Acceptance cases.
// ---------------------------------------------------------------------------

describe("gosper: acceptance", () => {
  it("∑_{k=1}^{5} k·2^k = 258 via dispatcher", () => {
    const f = makeKTimes2K();
    const result = evaluateSum(f, k, int(1), int(5), evalNode);
    expect(rationalValue(result)).toEqual({ numer: 258n, denom: 1n });
  });

  it("∑_{k=1}^{N} k·2^k symbolic closed form: matches direct sums at small N", () => {
    const f = makeKTimes2K();
    const result = tryGosperSum(f, k, int(1), N);
    expect(result).toBeDefined();
    // Not the unevaluated SUM.
    expect(result!.kind === "apply" && irEquals(result!.head, SUM)).toBe(false);
    for (const n of [1n, 2n, 3n, 5n, 7n]) {
      let expected = 0n;
      for (let j = 1n; j <= n; j++) expected += j * (2n ** j);
      expect(eqR(evalAt(result!, N, n), rv(expected))).toBe(true);
    }
  });

  it("∑_{k=0}^{N} k·k! = (N+1)! − 1: matches direct sums at small N", () => {
    const f = makeKTimesKFact();
    const result = tryGosperSum(f, k, int(0), N);
    expect(result).toBeDefined();
    expect(result!.kind === "apply" && irEquals(result!.head, SUM)).toBe(false);
    for (const n of [0n, 1n, 2n, 3n, 4n, 5n]) {
      let expected = 0n;
      for (let j = 0n; j <= n; j++) expected += j * factorialBig(j);
      expect(eqR(evalAt(result!, N, n), rv(expected))).toBe(true);
    }
  });

  it("∑_{k=0}^{5} 2^k = 63 via existing geometric handler (no regression)", () => {
    const f = app(POW, [int(2), k]);
    const result = evaluateSum(f, k, int(0), int(5), evalNode);
    expect(rationalValue(result)).toEqual({ numer: 63n, denom: 1n });
  });
});

// ---------------------------------------------------------------------------
// Fall-through safety.
// ---------------------------------------------------------------------------

describe("gosper: fall-through", () => {
  it("∑ sin(k) falls through to unevaluated SUM", () => {
    const f = app(sym("Sin"), [k]);
    const result = evaluateSum(f, k, int(1), N, evalNode);
    expect(result.kind === "apply" && irEquals(result.head, SUM)).toBe(true);
  });
  it("∑ log(k) falls through to unevaluated SUM", () => {
    const f = app(sym("Log"), [k]);
    const result = evaluateSum(f, k, int(1), N, evalNode);
    expect(result.kind === "apply" && irEquals(result.head, SUM)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Regression: existing handlers still fire first.
// ---------------------------------------------------------------------------

describe("gosper: regression — existing handlers still take priority", () => {
  it("∑_{k=1}^{4} k = 10 via Faulhaber", () => {
    const result = evaluateSum(k, k, int(1), int(4), evalNode);
    expect(rationalValue(result)).toEqual({ numer: 10n, denom: 1n });
  });
  it("∑_{k=1}^{10} 5 = 50 via constant handler", () => {
    const result = evaluateSum(int(5), k, int(1), int(10), evalNode);
    expect(rationalValue(result)).toEqual({ numer: 50n, denom: 1n });
  });
});

// ---------------------------------------------------------------------------
// Lower-level structural pieces.
// ---------------------------------------------------------------------------

describe("gosper: structural pieces", () => {
  it("_decompose(k·2^k): poly = k, exp_factors = [(2, k)]", () => {
    const { decompose } = __test;
    const f = makeKTimes2K();
    const h = decompose(f, k);
    expect(h).toBeDefined();
    expect(h!.poly.map((f) => `${f.n}/${f.d}`)).toEqual(["0/1", "1/1"]);
    expect(h!.expFactors.length).toBe(1);
    expect(h!.expFactors[0].base).toEqual({ n: 2n, d: 1n });
    expect(h!.expFactors[0].exp.map((f) => `${f.n}/${f.d}`)).toEqual(["0/1", "1/1"]);
  });
  it("_hypRatio(k·2^k): numer = 2 + 2k, denom = k", () => {
    const { decompose, hypRatio } = __test;
    const f = makeKTimes2K();
    const h = decompose(f, k);
    const ratio = hypRatio(h!);
    expect(ratio).toBeDefined();
    const [numer, denom] = ratio!;
    expect(numer.map((f) => `${f.n}/${f.d}`)).toEqual(["2/1", "2/1"]);
    expect(denom.map((f) => `${f.n}/${f.d}`)).toEqual(["0/1", "1/1"]);
  });
});

// ---------------------------------------------------------------------------
// DoS cap: very large polynomial exponents are refused before they
// balloon memory.
// ---------------------------------------------------------------------------

describe("gosper: MAX_POLY_DEGREE DoS cap", () => {
  it("Pow(k, 10**9) is refused — sum falls through, no memory blowup", () => {
    expect(MAX_POLY_DEGREE).toBe(64);
    const f = app(POW, [k, int(10n ** 9n)]);
    // Should fall through and either return a small-range numeric or
    // unevaluated SUM — what matters is it returns promptly.
    const result = evaluateSum(f, k, int(1), N, evalNode);
    // Symbolic N can't be numerically summed, so should be SUM.
    expect(result.kind === "apply" && irEquals(result.head, SUM)).toBe(true);
  });
});
