import { factorIntegerPolynomial, type IntegerLike } from "@coding-adventures/cas-factor";
import {
  ADD,
  MUL,
  NEG,
  POW,
  SQRT,
  SUB,
  app,
  equals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const ALG_FACTOR = sym("AlgFactor");

export type AlgFactorEvalFn = (node: IRNode) => IRNode;

export interface AlgFactorEvaluator {
  eval(node: IRNode): IRNode;
}

export type AlgFactorHandlerEvaluator = AlgFactorEvalFn | AlgFactorEvaluator;

export type AlgFactorHandler = (expr: IRNode, evaluator?: AlgFactorHandlerEvaluator) => IRNode;

export interface RationalValue {
  readonly numer: bigint;
  readonly denom: bigint;
}

export interface AlgCoeff {
  readonly rational: RationalValue;
  readonly radical: RationalValue;
}

export type AlgPoly = readonly AlgCoeff[];

const ZERO = ratValue(0n);
const ONE = ratValue(1n);

export function factorOverExtension(coeffsInput: readonly IntegerLike[], dInput: IntegerLike): AlgPoly[] | null {
  const coeffs = coeffsInput.map(toBigInt);
  const d = toBigInt(dInput);
  if (coeffs.length <= 2 || d <= 0n) return null;

  const [, factorsZ] = factorIntegerPolynomial(coeffs);
  const result: AlgPoly[] = [];
  let anySplit = false;

  for (const [factor, multiplicity] of factorsZ) {
    const split = trySplitSingle(factor, d);
    if (split !== null) {
      for (let i = 0; i < multiplicity; i += 1) {
        result.push(...split);
      }
      anySplit = true;
    } else {
      const algFactor = integerPolyToAlgPoly(factor);
      for (let i = 0; i < multiplicity; i += 1) {
        result.push(algFactor);
      }
    }
  }

  return anySplit ? result : null;
}

export function trySplitSingle(coeffsInput: readonly IntegerLike[], dInput: IntegerLike): AlgPoly[] | null {
  const coeffs = coeffsInput.map(toBigInt);
  const d = toBigInt(dInput);
  if (coeffs.length < 3) return null;
  const normalized = coeffs[coeffs.length - 1] === -1n ? coeffs.map((coeff) => -coeff) : coeffs;
  if (normalized.length === 3) return trySplitQuadratic(normalized, d);
  if (normalized.length === 5) return trySplitDepressedQuartic(normalized, d);
  return null;
}

export function trySplitQuadratic(coeffsInput: readonly IntegerLike[], dInput: IntegerLike): AlgPoly[] | null {
  const coeffs = coeffsInput.map(toBigInt);
  const d = toBigInt(dInput);
  if (coeffs.length !== 3 || coeffs[2] !== 1n || d <= 0n) return null;

  const c = ratValue(coeffs[0]);
  const b = ratValue(coeffs[1]);
  const disc = subRat(mulRat(b, b), mulRat(ratValue(4n), c));
  if (isZeroRat(disc)) return null;

  const twoBeta = rationalSquareRoot(divRat(disc, ratValue(d)));
  if (twoBeta === null || isZeroRat(twoBeta)) return null;
  const beta = divRat(twoBeta, ratValue(2n));
  const rootRational = divRat(negRat(b), ratValue(2n));

  const linear = (radical: RationalValue): AlgPoly => [
    algCoeff(negRat(rootRational), radical),
    algCoeff(ONE, ZERO),
  ];
  return [linear(negRat(beta)), linear(beta)];
}

export function trySplitDepressedQuartic(coeffsInput: readonly IntegerLike[], dInput: IntegerLike): AlgPoly[] | null {
  const coeffs = coeffsInput.map(toBigInt);
  const d = toBigInt(dInput);
  if (coeffs.length !== 5 || coeffs[4] !== 1n || coeffs[3] !== 0n || coeffs[1] !== 0n || d <= 0n) {
    return null;
  }

  const qRoot = rationalSquareRoot(ratValue(coeffs[0]));
  if (qRoot === null) return null;
  const p = ratValue(coeffs[2]);

  for (const s of [qRoot, negRat(qRoot)]) {
    const numerator = subRat(mulRat(ratValue(2n), s), p);
    if (isNegativeRat(numerator)) continue;
    const r = rationalSquareRoot(divRat(numerator, ratValue(d)));
    if (r === null || isZeroRat(r)) continue;
    return [
      [algCoeff(s, ZERO), algCoeff(ZERO, r), algCoeff(ONE, ZERO)],
      [algCoeff(s, ZERO), algCoeff(ZERO, negRat(r)), algCoeff(ONE, ZERO)],
    ];
  }

  return null;
}

export function rationalSquareRoot(value: RationalValue): RationalValue | null {
  if (isNegativeRat(value)) return null;
  if (isZeroRat(value)) return ZERO;
  const numer = integerSquareRoot(value.numer);
  const denom = integerSquareRoot(value.denom);
  return numer === null || denom === null ? null : ratValue(numer, denom);
}

export function extractRadicalD(node: IRNode): bigint | null {
  if (node.kind !== "apply" || !equals(node.head, SQRT) || node.args.length !== 1) return null;
  const inner = node.args[0];
  if (inner.kind !== "integer" || inner.value <= 0n || integerSquareRoot(inner.value) !== null) return null;
  return inner.value;
}

export function algFactorIr(poly: IRNode, sqrtD: IRNode, variable: IRNode): IRNode | null {
  const d = extractRadicalD(sqrtD);
  if (d === null) return null;
  const coeffs = irToIntegerPoly(poly, variable);
  if (coeffs === null) return null;
  const factors = factorOverExtension(coeffs, d);
  return factors === null ? null : factorsToIr(factors, variable, sqrtD);
}

function algFactorNormalizedIr(poly: IRNode, sqrtD: IRNode, variable: IRNode): IRNode | null {
  const d = extractRadicalD(sqrtD);
  if (d === null) return null;
  const rationalCoeffs = irToRationalPoly(poly, variable);
  if (rationalCoeffs === null) return null;
  const commonDenom = rationalCoeffs.reduce((acc, coeff) => lcm(acc, coeff.denom), 1n);
  const coeffs = rationalCoeffs.map((coeff) => coeff.numer * (commonDenom / coeff.denom));
  const factors = factorOverExtension(trimBigint(coeffs), d);
  return factors === null ? null : factorsToIr(factors, variable, sqrtD);
}

export function algFactorHandler(expr: IRNode, evaluator?: AlgFactorHandlerEvaluator): IRNode {
  if (expr.kind !== "apply" || !equals(expr.head, ALG_FACTOR) || expr.args.length !== 2) return expr;
  const [poly, sqrtD] = expr.args;
  const evaluatedPoly = evaluatePolynomial(poly, evaluator);
  const variable = findPolynomialVariable(evaluatedPoly);
  if (variable === null) return expr;
  return algFactorNormalizedIr(evaluatedPoly, sqrtD, variable) ?? expr;
}

export function buildAlgFactorHandlerTable(): ReadonlyMap<string, AlgFactorHandler> {
  return new Map([[ALG_FACTOR.name, algFactorHandler]]);
}

export const alg_factor_handler = algFactorHandler;
export const build_alg_factor_handler_table = buildAlgFactorHandlerTable;

export function factorsToIr(factors: readonly AlgPoly[], variable: IRNode, sqrtD: IRNode): IRNode {
  if (factors.length === 0) return int(1);
  return factors.map((factor) => algPolyToIr(factor, variable, sqrtD))
    .reduce((lhs, rhs) => app(MUL, [lhs, rhs]));
}

export function algPolyToIr(poly: AlgPoly, variable: IRNode, sqrtD: IRNode): IRNode {
  const terms: IRNode[] = [];
  poly.forEach((coeff, degree) => {
    if (isZeroRat(coeff.rational) && isZeroRat(coeff.radical)) return;
    const coeffIr = algCoeffToIr(coeff, sqrtD);
    const term = degree === 0
      ? coeffIr
      : multiplyCoeff(coeffIr, degree === 1 ? variable : app(POW, [variable, int(degree)]));
    terms.push(term);
  });
  return terms.length === 0 ? int(0) : terms.reduce((lhs, rhs) => app(ADD, [lhs, rhs]));
}

export function algCoeffToIr(coeff: AlgCoeff, sqrtD: IRNode): IRNode {
  const rationalPart = rationalToIr(coeff.rational);
  if (isZeroRat(coeff.radical)) return rationalPart;

  const radicalPart = isOneRat(coeff.radical)
    ? sqrtD
    : eqRat(coeff.radical, ratValue(-1n))
      ? app(NEG, [sqrtD])
      : app(MUL, [rationalToIr(coeff.radical), sqrtD]);

  return isZeroRat(coeff.rational) ? radicalPart : app(ADD, [rationalPart, radicalPart]);
}

export function irToIntegerPoly(node: IRNode, variable: IRNode): bigint[] | null {
  const poly = irToRationalPoly(node, variable);
  if (poly === null) return null;
  const out: bigint[] = [];
  for (const coeff of poly) {
    if (coeff.denom !== 1n) return null;
    out.push(coeff.numer);
  }
  return trimBigint(out);
}

function irToRationalPoly(node: IRNode, variable: IRNode): RationalValue[] | null {
  if (equals(node, variable)) return [ZERO, ONE];
  if (node.kind === "integer") return [ratValue(node.value)];
  if (node.kind === "rational") return [ratValue(node.numer, node.denom)];
  if (node.kind !== "apply") return null;

  if (equals(node.head, ADD)) {
    return node.args.reduce<RationalValue[] | null>((acc, arg) => {
      if (acc === null) return null;
      const rhs = irToRationalPoly(arg, variable);
      return rhs === null ? null : polyAdd(acc, rhs);
    }, [ZERO]);
  }

  if (equals(node.head, SUB) && node.args.length === 2) {
    const lhs = irToRationalPoly(node.args[0], variable);
    const rhs = irToRationalPoly(node.args[1], variable);
    return lhs === null || rhs === null ? null : polySub(lhs, rhs);
  }

  if (equals(node.head, MUL)) {
    return node.args.reduce<RationalValue[] | null>((acc, arg) => {
      if (acc === null) return null;
      const rhs = irToRationalPoly(arg, variable);
      return rhs === null ? null : polyMul(acc, rhs);
    }, [ONE]);
  }

  if (equals(node.head, NEG) && node.args.length === 1) {
    const inner = irToRationalPoly(node.args[0], variable);
    return inner === null ? null : inner.map(negRat);
  }

  if (equals(node.head, POW) && node.args.length === 2) {
    const exp = node.args[1];
    if (exp.kind !== "integer" || exp.value < 0n) return null;
    const base = irToRationalPoly(node.args[0], variable);
    if (base === null) return null;
    let acc = [ONE];
    for (let i = 0n; i < exp.value; i += 1n) {
      acc = polyMul(acc, base);
    }
    return acc;
  }

  return null;
}

function integerPolyToAlgPoly(poly: readonly bigint[]): AlgPoly {
  return poly.map((coeff) => algCoeff(ratValue(coeff), ZERO));
}

const CONSTANT_SYMBOL_NAMES = new Set(["True", "False", "%pi", "%e", "%i"]);

function evaluatePolynomial(poly: IRNode, evaluator: AlgFactorHandlerEvaluator | undefined): IRNode {
  if (evaluator === undefined) return poly;
  return typeof evaluator === "function" ? evaluator(poly) : evaluator.eval(poly);
}

function findPolynomialVariable(node: IRNode): IRNode | null {
  if (node.kind === "symbol") return CONSTANT_SYMBOL_NAMES.has(node.name) ? null : node;
  if (node.kind !== "apply") return null;
  for (const arg of node.args) {
    const found = findPolynomialVariable(arg);
    if (found !== null) return found;
  }
  return null;
}

function algCoeff(rationalPart: RationalValue, radicalPart: RationalValue): AlgCoeff {
  return Object.freeze({ rational: rationalPart, radical: radicalPart });
}

function multiplyCoeff(coeff: IRNode, term: IRNode): IRNode {
  if (equals(coeff, int(1))) return term;
  if (equals(coeff, int(-1))) return app(NEG, [term]);
  return app(MUL, [coeff, term]);
}

function rationalToIr(value: RationalValue): IRNode {
  return value.denom === 1n ? int(value.numer) : rational(value.numer, value.denom);
}

function polyAdd(lhs: RationalValue[], rhs: RationalValue[]): RationalValue[] {
  const length = Math.max(lhs.length, rhs.length);
  return Array.from({ length }, (_, index) => addRat(lhs[index] ?? ZERO, rhs[index] ?? ZERO));
}

function polySub(lhs: RationalValue[], rhs: RationalValue[]): RationalValue[] {
  const length = Math.max(lhs.length, rhs.length);
  return Array.from({ length }, (_, index) => subRat(lhs[index] ?? ZERO, rhs[index] ?? ZERO));
}

function polyMul(lhs: RationalValue[], rhs: RationalValue[]): RationalValue[] {
  if (lhs.length === 0 || rhs.length === 0) return [ZERO];
  const out = Array.from({ length: lhs.length + rhs.length - 1 }, () => ZERO);
  for (let i = 0; i < lhs.length; i += 1) {
    for (let j = 0; j < rhs.length; j += 1) {
      out[i + j] = addRat(out[i + j], mulRat(lhs[i], rhs[j]));
    }
  }
  return out;
}

function trimBigint(poly: bigint[]): bigint[] {
  while (poly.length > 0 && poly[poly.length - 1] === 0n) poly.pop();
  return poly;
}

function ratValue(numerInput: IntegerLike, denomInput: IntegerLike = 1n): RationalValue {
  let numer = toBigInt(numerInput);
  let denom = toBigInt(denomInput);
  if (denom === 0n) throw new RangeError("Rational denominator cannot be zero");
  if (numer === 0n) return Object.freeze({ numer: 0n, denom: 1n });
  if (denom < 0n) {
    numer = -numer;
    denom = -denom;
  }
  const g = gcd(abs(numer), denom);
  return Object.freeze({ numer: numer / g, denom: denom / g });
}

function addRat(lhs: RationalValue, rhs: RationalValue): RationalValue {
  return ratValue(lhs.numer * rhs.denom + rhs.numer * lhs.denom, lhs.denom * rhs.denom);
}

function subRat(lhs: RationalValue, rhs: RationalValue): RationalValue {
  return ratValue(lhs.numer * rhs.denom - rhs.numer * lhs.denom, lhs.denom * rhs.denom);
}

function mulRat(lhs: RationalValue, rhs: RationalValue): RationalValue {
  return ratValue(lhs.numer * rhs.numer, lhs.denom * rhs.denom);
}

function divRat(lhs: RationalValue, rhs: RationalValue): RationalValue {
  if (isZeroRat(rhs)) throw new RangeError("Rational division by zero");
  return ratValue(lhs.numer * rhs.denom, lhs.denom * rhs.numer);
}

function negRat(value: RationalValue): RationalValue {
  return ratValue(-value.numer, value.denom);
}

function isZeroRat(value: RationalValue): boolean {
  return value.numer === 0n;
}

function isOneRat(value: RationalValue): boolean {
  return value.numer === value.denom;
}

function isNegativeRat(value: RationalValue): boolean {
  return value.numer < 0n;
}

function eqRat(lhs: RationalValue, rhs: RationalValue): boolean {
  return lhs.numer === rhs.numer && lhs.denom === rhs.denom;
}

function integerSquareRoot(value: bigint): bigint | null {
  if (value < 0n) return null;
  if (value < 2n) return value;
  let lo = 1n;
  let hi = value;
  while (lo <= hi) {
    const mid = (lo + hi) / 2n;
    const square = mid * mid;
    if (square === value) return mid;
    if (square < value) lo = mid + 1n;
    else hi = mid - 1n;
  }
  return null;
}

function toBigInt(value: IntegerLike): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "string") return BigInt(value);
  if (!Number.isSafeInteger(value)) throw new RangeError("number inputs must be safe integers");
  return BigInt(value);
}

function gcd(mutA: bigint, mutB: bigint): bigint {
  let a = abs(mutA);
  let b = abs(mutB);
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a;
}

function lcm(a: bigint, b: bigint): bigint {
  return a === 0n || b === 0n ? 0n : (a / gcd(a, b)) * b;
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}
