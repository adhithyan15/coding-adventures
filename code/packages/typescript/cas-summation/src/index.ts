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
  equals as irEquals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const GAMMA_FUNC = sym("GammaFunc");

export interface RationalValue {
  readonly numer: bigint;
  readonly denom: bigint;
}

export type EvalFn = (node: IRNode) => IRNode;

export function rationalValue(node: IRNode): RationalValue | undefined {
  if (node.kind === "integer") return makeRational(node.value, 1n);
  if (node.kind === "rational") return makeRational(node.numer, node.denom);
  return undefined;
}

export function isConstantIn(node: IRNode, k: IRNode): boolean {
  if (equals(node, k)) return false;
  return node.kind !== "apply" || node.args.every((arg) => isConstantIn(arg, k));
}

export function geometricSumIr(
  coeff: IRNode,
  base: IRNode,
  lo: IRNode,
  hi: IRNode | undefined,
  isInfinite: boolean,
): IRNode {
  const sumPart = (() => {
    if (isInfinite) {
      const oneMinusBase = app(SUB, [int(1), base]);
      return isInt(lo, 0n) ? app(DIV, [int(1), oneMinusBase]) : app(DIV, [app(POW, [base, lo]), oneMinusBase]);
    }
    if (hi === undefined) throw new TypeError("hi must be provided for finite geometric sums");
    const spanPlusOne = app(ADD, [app(SUB, [hi, lo]), int(1)]);
    const numerator = app(SUB, [app(POW, [base, spanPlusOne]), int(1)]);
    const denominator = app(SUB, [base, int(1)]);
    return app(MUL, [app(POW, [base, lo]), app(DIV, [numerator, denominator])]);
  })();
  return isInt(coeff, 1n) ? sumPart : app(MUL, [coeff, sumPart]);
}

export function faulhaberIr(m: number, n: IRNode): IRNode | undefined {
  switch (m) {
    case 0:
      return n;
    case 1:
      return app(DIV, [app(MUL, [n, app(ADD, [n, int(1)])]), int(2)]);
    case 2:
      return app(DIV, [
        app(MUL, [n, app(MUL, [app(ADD, [n, int(1)]), app(ADD, [app(MUL, [int(2), n]), int(1)])])]),
        int(6),
      ]);
    case 3: {
      const half = app(DIV, [app(MUL, [n, app(ADD, [n, int(1)])]), int(2)]);
      return app(POW, [half, int(2)]);
    }
    case 4: {
      const inner = app(SUB, [app(ADD, [app(MUL, [int(3), app(POW, [n, int(2)])]), app(MUL, [int(3), n])]), int(1)]);
      const twoNPlusOne = app(ADD, [app(MUL, [int(2), n]), int(1)]);
      return app(DIV, [app(MUL, [n, app(MUL, [app(ADD, [n, int(1)]), app(MUL, [twoNPlusOne, inner])])]), int(30)]);
    }
    case 5: {
      const inner = app(SUB, [app(ADD, [app(MUL, [int(2), app(POW, [n, int(2)])]), app(MUL, [int(2), n])]), int(1)]);
      return app(DIV, [app(MUL, [app(POW, [n, int(2)]), app(MUL, [app(POW, [app(ADD, [n, int(1)]), int(2)]), inner])]), int(12)]);
    }
    default:
      return undefined;
  }
}

export function polySumIr(m: number, coeff: RationalValue, loValue: bigint, hi: IRNode): IRNode | undefined {
  const sHi = faulhaberIr(m, hi);
  if (sHi === undefined) return undefined;
  const loMinusOne = loValue - 1n;
  let diff = loMinusOne <= 0n ? sHi : app(SUB, [sHi, faulhaberIr(m, int(loMinusOne)) ?? int(0)]);
  if (loValue === 0n && m === 0) diff = app(ADD, [diff, int(1)]);
  return rationalEquals(coeff, makeRational(1n, 1n)) ? diff : app(MUL, [rationalToIr(coeff), diff]);
}

export function trySpecialInfinite(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  if (isInt(lo, 1n) && matchInvKPow(f, k, 2n)) return app(DIV, [app(POW, [sym("%pi"), int(2)]), int(6)]);
  if (isInt(lo, 1n) && matchInvKPow(f, k, 4n)) return app(DIV, [app(POW, [sym("%pi"), int(4)]), int(90)]);
  if (isInt(lo, 0n) && matchLeibniz(f, k)) return app(DIV, [sym("%pi"), int(4)]);
  if (isInt(lo, 0n) && matchInvFactorial(f, k)) return sym("%e");
  if (isInt(lo, 0n)) {
    const x = matchExpSeries(f, k);
    if (x !== undefined) return app(EXP, [x]);
  }
  return undefined;
}

export function evaluateSum(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode, evalFn: EvalFn): IRNode {
  const infUpper = isInf(hi);
  if (isConstantIn(f, k)) {
    return evalFn(app(MUL, [f, app(ADD, [app(SUB, [hi, lo]), int(1)])]));
  }

  const geo = tryGeometric(f, k);
  if (geo !== undefined) {
    return evalFn(geometricSumIr(geo.coeff, geo.base, lo, hi, infUpper));
  }

  const power = tryPowerOfK(f, k);
  if (power !== undefined && lo.kind === "integer" && lo.value >= 0n && !infUpper) {
    const raw = polySumIr(power.m, power.coeff, lo.value, hi);
    if (raw !== undefined) return evalFn(raw);
  }

  if (infUpper) {
    const raw = trySpecialInfinite(f, k, lo);
    if (raw !== undefined) return evalFn(raw);
  }

  if (lo.kind === "integer" && hi.kind === "integer" && hi.value - lo.value >= 0n && hi.value - lo.value <= 999n) {
    let total = makeRational(0n, 1n);
    let ok = true;
    for (let value = lo.value; value <= hi.value; value += 1n) {
      const evaluated = evalFn(substitute(f, k, int(value)));
      const r = rationalValue(evaluated);
      if (r === undefined) {
        ok = false;
        break;
      }
      total = addR(total, r);
    }
    if (ok) return rationalToIr(total);
  }

  return app(SUM, [f, k, lo, hi]);
}

export function evaluateProduct(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode, evalFn: EvalFn): IRNode {
  const raw = evaluateProductExpr(f, k, lo, hi);
  if (raw !== undefined) return evalFn(raw);

  if (lo.kind === "integer" && hi.kind === "integer" && hi.value - lo.value >= 0n && hi.value - lo.value <= 20n) {
    let total = makeRational(1n, 1n);
    let ok = true;
    for (let value = lo.value; value <= hi.value; value += 1n) {
      const evaluated = evalFn(substitute(f, k, int(value)));
      const r = rationalValue(evaluated);
      if (r === undefined) {
        ok = false;
        break;
      }
      total = mulR(total, r);
    }
    if (ok) return rationalToIr(total);
  }

  return app(PRODUCT, [f, k, lo, hi]);
}

export function evaluateProductExpr(f: IRNode, k: IRNode, lo: IRNode, hi: IRNode): IRNode | undefined {
  if (isConstantIn(f, k)) return app(POW, [f, app(ADD, [app(SUB, [hi, lo]), int(1)])]);
  if (isInt(lo, 1n) && equals(f, k)) return gamma(hi);
  if (isInt(lo, 1n)) {
    const coeff = splitLinearCoeff(f, k);
    if (coeff !== undefined) {
      if (rationalEquals(coeff, makeRational(1n, 1n))) return gamma(hi);
      return app(MUL, [app(POW, [rationalToIr(coeff), hi]), gamma(hi)]);
    }
  }
  return undefined;
}

function tryGeometric(f: IRNode, k: IRNode): { coeff: IRNode; base: IRNode } | undefined {
  if (f.kind === "apply" && equals(f.head, POW) && f.args.length === 2 && equals(f.args[1], k) && !equals(f.args[0], k) && isConstantIn(f.args[0], k)) {
    return { coeff: int(1), base: f.args[0] };
  }
  if (f.kind === "apply" && equals(f.head, MUL) && f.args.length === 2) {
    for (const [coeff, pow] of [[f.args[0], f.args[1]], [f.args[1], f.args[0]]] as const) {
      if (pow.kind === "apply" && equals(pow.head, POW) && pow.args.length === 2 && equals(pow.args[1], k) && !equals(pow.args[0], k) && isConstantIn(pow.args[0], k) && isConstantIn(coeff, k)) {
        return { coeff, base: pow.args[0] };
      }
    }
  }
  return undefined;
}

function tryPowerOfK(f: IRNode, k: IRNode): { coeff: RationalValue; m: number } | undefined {
  if (equals(f, k)) return { coeff: makeRational(1n, 1n), m: 1 };
  if (f.kind === "apply" && equals(f.head, POW) && f.args.length === 2 && equals(f.args[0], k) && f.args[1].kind === "integer" && f.args[1].value >= 0n && f.args[1].value <= 5n) {
    return { coeff: makeRational(1n, 1n), m: Number(f.args[1].value) };
  }
  if (f.kind === "apply" && equals(f.head, MUL) && f.args.length === 2) {
    for (const [coeffNode, other] of [[f.args[0], f.args[1]], [f.args[1], f.args[0]]] as const) {
      const coeff = rationalValue(coeffNode);
      if (coeff === undefined) continue;
      if (equals(other, k)) return { coeff, m: 1 };
      if (other.kind === "apply" && equals(other.head, POW) && other.args.length === 2 && equals(other.args[0], k) && other.args[1].kind === "integer" && other.args[1].value >= 0n && other.args[1].value <= 5n) {
        return { coeff, m: Number(other.args[1].value) };
      }
    }
  }
  return undefined;
}

function splitLinearCoeff(f: IRNode, k: IRNode): RationalValue | undefined {
  if (equals(f, k)) return makeRational(1n, 1n);
  if (f.kind !== "apply" || !equals(f.head, MUL) || f.args.length !== 2) return undefined;
  const a = rationalValue(f.args[0]);
  if (a !== undefined && equals(f.args[1], k)) return a;
  const b = rationalValue(f.args[1]);
  if (b !== undefined && equals(f.args[0], k)) return b;
  return undefined;
}

function matchInvKPow(f: IRNode, k: IRNode, exp: bigint): boolean {
  return f.kind === "apply" && equals(f.head, DIV) && f.args.length === 2 && isInt(f.args[0], 1n)
    && f.args[1].kind === "apply" && equals(f.args[1].head, POW) && f.args[1].args.length === 2
    && equals(f.args[1].args[0], k) && isInt(f.args[1].args[1], exp);
}

function matchLeibniz(f: IRNode, k: IRNode): boolean {
  if (f.kind !== "apply" || !equals(f.head, DIV) || f.args.length !== 2) return false;
  const [numerator, denominator] = f.args;
  const numOk = numerator.kind === "apply" && equals(numerator.head, POW) && numerator.args.length === 2
    && equals(numerator.args[1], k) && (isInt(numerator.args[0], -1n) || isNegOne(numerator.args[0]));
  if (!numOk || denominator.kind !== "apply" || !equals(denominator.head, ADD) || denominator.args.length !== 2) return false;
  const [a, b] = denominator.args;
  return isTwoKPlusOne(a, b, k) || isTwoKPlusOne(b, a, k);
}

function matchInvFactorial(f: IRNode, k: IRNode): boolean {
  return f.kind === "apply" && equals(f.head, DIV) && f.args.length === 2 && isInt(f.args[0], 1n)
    && matchGammaKPlusOne(f.args[1], k);
}

function matchExpSeries(f: IRNode, k: IRNode): IRNode | undefined {
  if (f.kind !== "apply" || !equals(f.head, DIV) || f.args.length !== 2) return undefined;
  const numerator = f.args[0];
  if (numerator.kind !== "apply" || !equals(numerator.head, POW) || numerator.args.length !== 2 || !equals(numerator.args[1], k) || equals(numerator.args[0], k)) return undefined;
  return matchGammaKPlusOne(f.args[1], k) ? numerator.args[0] : undefined;
}

function matchGammaKPlusOne(node: IRNode, k: IRNode): boolean {
  return node.kind === "apply" && equals(node.head, GAMMA_FUNC) && node.args.length === 1
    && node.args[0].kind === "apply" && equals(node.args[0].head, ADD) && node.args[0].args.length === 2
    && equals(node.args[0].args[0], k) && isInt(node.args[0].args[1], 1n);
}

function substitute(node: IRNode, from: IRNode, to: IRNode): IRNode {
  if (equals(node, from)) return to;
  if (node.kind !== "apply") return node;
  return app(node.head, node.args.map((arg) => substitute(arg, from, to)));
}

function gamma(n: IRNode): IRNode {
  return app(GAMMA_FUNC, [app(ADD, [n, int(1)])]);
}

function isInt(node: IRNode, value: bigint): boolean {
  return node.kind === "integer" && node.value === value;
}

function isInf(node: IRNode): boolean {
  return node.kind === "symbol" && (node.name === "inf" || node.name === "%inf");
}

function isNegOne(node: IRNode): boolean {
  return node.kind === "apply" && equals(node.head, NEG) && node.args.length === 1 && isInt(node.args[0], 1n);
}

function isTwoKPlusOne(twoK: IRNode, one: IRNode, k: IRNode): boolean {
  return isInt(one, 1n) && twoK.kind === "apply" && equals(twoK.head, MUL) && twoK.args.length === 2 && isInt(twoK.args[0], 2n) && equals(twoK.args[1], k);
}

function equals(a: IRNode, b: IRNode): boolean {
  return irEquals(a, b);
}

function rationalToIr(value: RationalValue): IRNode {
  return value.denom === 1n ? int(value.numer) : rational(value.numer, value.denom);
}

function makeRational(numer: bigint, denom: bigint): RationalValue {
  if (denom === 0n) throw new RangeError("denominator cannot be zero");
  let n = numer;
  let d = denom;
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  const g = gcd(abs(n), d);
  return { numer: n / g, denom: d / g };
}

function rationalEquals(a: RationalValue, b: RationalValue): boolean {
  return a.numer === b.numer && a.denom === b.denom;
}

function addR(a: RationalValue, b: RationalValue): RationalValue {
  return makeRational(a.numer * b.denom + b.numer * a.denom, a.denom * b.denom);
}

function mulR(a: RationalValue, b: RationalValue): RationalValue {
  return makeRational(a.numer * b.numer, a.denom * b.denom);
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

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}
