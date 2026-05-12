import { subst } from "@coding-adventures/cas-substitution";
import {
  ADD,
  ATAN,
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
  SQRT,
  SUB,
  TAN,
  TANH,
  app,
  equals,
  headName,
  int,
  numberNode,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const LIMIT = "Limit";
export const TAYLOR = "Taylor";
export const SERIES = "Series";
export const BIG_O = "BigO";

export type IntegerLike = bigint | number | string;
export type LimitDirection = "plus" | "minus";
export type LimitCallback = (node: IRNode) => IRNode | null | undefined;
export type LimitDerivativeCallback = (node: IRNode, variable: IRNode) => IRNode | null | undefined;

export interface LimitAdvancedOptions {
  readonly direction?: LimitDirection;
  readonly differentiate?: LimitDerivativeCallback;
  readonly simplify?: LimitCallback;
  readonly evaluate?: LimitCallback;
  readonly maxDepth?: number;
}

export class PolynomialError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "PolynomialError";
  }
}

class Frac {
  readonly numer: bigint;
  readonly denom: bigint;

  constructor(numerInput: IntegerLike, denomInput: IntegerLike = 1n) {
    let numer = toBigInt(numerInput);
    let denom = toBigInt(denomInput);
    if (denom === 0n) throw new RangeError("Frac: denominator must not be zero");
    if (numer === 0n) {
      this.numer = 0n;
      this.denom = 1n;
      return;
    }
    if (denom < 0n) {
      numer = -numer;
      denom = -denom;
    }
    const g = gcd(abs(numer), denom);
    this.numer = numer / g;
    this.denom = denom / g;
  }

  static zero(): Frac {
    return new Frac(0n);
  }

  static one(): Frac {
    return new Frac(1n);
  }

  static fromInt(value: IntegerLike): Frac {
    return new Frac(value, 1n);
  }

  isZero(): boolean {
    return this.numer === 0n;
  }

  isOne(): boolean {
    return this.numer === 1n && this.denom === 1n;
  }

  neg(): Frac {
    return new Frac(-this.numer, this.denom);
  }

  add(rhs: Frac): Frac {
    return new Frac(this.numer * rhs.denom + rhs.numer * this.denom, this.denom * rhs.denom);
  }

  sub(rhs: Frac): Frac {
    return this.add(rhs.neg());
  }

  mul(rhs: Frac): Frac {
    return new Frac(this.numer * rhs.numer, this.denom * rhs.denom);
  }

  div(rhs: Frac): Frac {
    if (rhs.numer === 0n) throw new RangeError("Frac: division by zero");
    return new Frac(this.numer * rhs.denom, this.denom * rhs.numer);
  }

  powi(exp: number): Frac {
    let result = Frac.one();
    for (let i = 0; i < exp; i += 1) {
      result = result.mul(this);
    }
    return result;
  }

  toIrNode(): IRNode {
    return this.denom === 1n ? int(this.numer) : rational(this.numer, this.denom);
  }
}

export function limitDirect(expr: IRNode, variable: IRNode, point: IRNode): IRNode {
  const out = subst(point, variable, expr);
  if (looksIndeterminate(out)) {
    return app(sym(LIMIT), [expr, variable, point]);
  }
  return out;
}

export function limitAdvanced(
  expr: IRNode,
  variable: IRNode,
  point: IRNode,
  options: LimitAdvancedOptions = {},
): IRNode {
  return limitAdvancedInner(expr, variable, point, options, 0);
}

export function taylorPolynomial(expr: IRNode, variable: IRNode, point: IRNode, order: number): IRNode {
  if (!Number.isInteger(order) || order < 0) {
    throw new RangeError("taylorPolynomial order must be a non-negative integer");
  }
  const coeffsInVariable = toCoefficients(expr, variable);
  const shift = toFraction(point);
  const coeffsInDelta = shiftPolynomial(coeffsInVariable, shift);
  return fromCoefficients(coeffsInDelta.slice(0, order + 1), variable, point);
}

function looksIndeterminate(node: IRNode): boolean {
  return node.kind === "apply"
    && headName(node.head) === DIV.name
    && node.args.length === 2
    && isIntegerValue(node.args[0], 0n)
    && isIntegerValue(node.args[1], 0n);
}

const INF_THRESHOLD = 1e100;
const EPSILON = 1e-300;
const DEFAULT_MAX_DEPTH = 8;
const INF = sym("inf");
const MINF = sym("minf");

function limitAdvancedInner(
  expr: IRNode,
  variable: IRNode,
  point: IRNode,
  options: LimitAdvancedOptions,
  depth: number,
): IRNode {
  if (depth > (options.maxDepth ?? DEFAULT_MAX_DEPTH)) {
    return buildUnevaluatedLimit(expr, variable, point, options.direction);
  }

  const pointValue = pointToNumber(point);
  if (Number.isNaN(pointValue)) {
    return buildUnevaluatedLimit(expr, variable, point, options.direction);
  }

  const epsilon = options.direction === "minus" ? -EPSILON : EPSILON;
  const testPoint = Number.isFinite(pointValue) ? pointValue + epsilon : pointValue;
  const testValue = evalAtNumber(expr, variable, testPoint);
  if (isInfiniteLike(testValue)) {
    return testValue > 0 ? INF : MINF;
  }
  if (Number.isNaN(testValue)) {
    return handleIndeterminateForm(expr, variable, point, options, depth);
  }

  let substituted = subst(point, variable, expr);
  substituted = applyLimitCallbacks(substituted, options);
  const exactValue = numericEval(substituted);
  if (!Number.isNaN(exactValue)) {
    if (isInfiniteLike(exactValue)) {
      return exactValue > 0 ? INF : MINF;
    }
    return substituted;
  }

  return handleIndeterminateForm(expr, variable, point, options, depth);
}

function handleIndeterminateForm(
  expr: IRNode,
  variable: IRNode,
  point: IRNode,
  options: LimitAdvancedOptions,
  depth: number,
): IRNode {
  const exactPoint = pointToNumber(point);
  if (Number.isNaN(exactPoint)) {
    return buildUnevaluatedLimit(expr, variable, point, options.direction);
  }

  if (expr.kind === "apply" && headName(expr.head) === DIV.name && expr.args.length === 2) {
    const [numer, denom] = expr.args;
    const numerValue = evalAtNumber(numer, variable, exactPoint);
    const denomValue = evalAtNumber(denom, variable, exactPoint);
    const zeroZero = numerValue === 0 && denomValue === 0;
    const infInf = isInfiniteLike(numerValue) && isInfiniteLike(denomValue);
    if ((zeroZero || infInf) && options.differentiate !== undefined) {
      return lhopital(numer, denom, variable, point, options, depth);
    }
  }

  if (expr.kind === "apply" && headName(expr.head) === MUL.name && expr.args.length === 2) {
    const rewritten = rewriteZeroInfinityProduct(expr.args[0], expr.args[1], variable, point, exactPoint, options, depth);
    if (rewritten !== null) return rewritten;
  }

  if (expr.kind === "apply" && headName(expr.head) === POW.name && expr.args.length === 2) {
    const rewritten = rewriteIndeterminatePower(expr.args[0], expr.args[1], variable, point, exactPoint, options, depth);
    if (rewritten !== null) return rewritten;
  }

  return buildUnevaluatedLimit(expr, variable, point, options.direction);
}

function lhopital(
  numer: IRNode,
  denom: IRNode,
  variable: IRNode,
  point: IRNode,
  options: LimitAdvancedOptions,
  depth: number,
): IRNode {
  const dNumer = options.differentiate?.(numer, variable);
  const dDenom = options.differentiate?.(denom, variable);
  if (dNumer === null || dNumer === undefined || dDenom === null || dDenom === undefined) {
    return buildUnevaluatedLimit(app(DIV, [numer, denom]), variable, point, options.direction);
  }
  const ratio = applyLimitCallbacks(app(DIV, [
    applyLimitCallbacks(dNumer, options),
    applyLimitCallbacks(dDenom, options),
  ]), options);
  return limitAdvancedInner(ratio, variable, point, options, depth + 1);
}

function rewriteZeroInfinityProduct(
  a: IRNode,
  b: IRNode,
  variable: IRNode,
  point: IRNode,
  exactPoint: number,
  options: LimitAdvancedOptions,
  depth: number,
): IRNode | null {
  if (options.differentiate === undefined) return null;
  const aValue = evalAtNumber(a, variable, exactPoint);
  const bValue = evalAtNumber(b, variable, exactPoint);
  if (aValue === 0 && isInfiniteLike(bValue)) {
    return limitAdvancedInner(app(DIV, [b, app(DIV, [int(1), a])]), variable, point, {
      ...options,
      direction: undefined,
    }, depth + 1);
  }
  if (bValue === 0 && isInfiniteLike(aValue)) {
    return limitAdvancedInner(app(DIV, [a, app(DIV, [int(1), b])]), variable, point, {
      ...options,
      direction: undefined,
    }, depth + 1);
  }
  return null;
}

function rewriteIndeterminatePower(
  base: IRNode,
  exponent: IRNode,
  variable: IRNode,
  point: IRNode,
  exactPoint: number,
  options: LimitAdvancedOptions,
  depth: number,
): IRNode | null {
  const baseValue = evalAtNumber(base, variable, exactPoint);
  const exponentValue = evalAtNumber(exponent, variable, exactPoint);
  const oneToInfinity = Math.abs(baseValue - 1) < 1e-10 && isInfiniteLike(exponentValue);
  const zeroToZero = baseValue === 0 && exponentValue === 0;
  const infinityToZero = isInfiniteLike(baseValue) && exponentValue === 0;
  if (!(oneToInfinity || zeroToZero || infinityToZero)) return null;

  if (options.differentiate === undefined) {
    return buildUnevaluatedLimit(app(POW, [base, exponent]), variable, point, options.direction);
  }

  const exponentLimit = limitAdvancedInner(app(MUL, [exponent, app(LOG, [base])]), variable, point, {
    ...options,
    direction: undefined,
  }, depth + 1);
  if (isLimitNode(exponentLimit)) {
    return buildUnevaluatedLimit(app(POW, [base, exponent]), variable, point, options.direction);
  }
  return applyLimitCallbacks(app(EXP, [exponentLimit]), options);
}

function applyLimitCallbacks(node: IRNode, options: LimitAdvancedOptions): IRNode {
  let out = node;
  const simplified = options.simplify?.(out);
  if (simplified !== null && simplified !== undefined) out = simplified;
  const evaluated = options.evaluate?.(out);
  if (evaluated !== null && evaluated !== undefined) out = evaluated;
  return out;
}

function buildUnevaluatedLimit(expr: IRNode, variable: IRNode, point: IRNode, direction?: LimitDirection): IRNode {
  const args = direction === undefined ? [expr, variable, point] : [expr, variable, point, sym(direction)];
  return app(sym(LIMIT), args);
}

function isLimitNode(node: IRNode): boolean {
  return node.kind === "apply" && headName(node.head) === LIMIT;
}

function evalAtNumber(expr: IRNode, variable: IRNode, point: number): number {
  const pointNode = Number.isFinite(point) ? numberNode(point) : point > 0 ? INF : MINF;
  return numericEval(subst(pointNode, variable, expr));
}

function pointToNumber(point: IRNode): number {
  return numericEval(point);
}

function numericEval(node: IRNode): number {
  try {
    return numericEvalUnsafe(node);
  } catch {
    return Number.NaN;
  }
}

function numericEvalUnsafe(node: IRNode): number {
  switch (node.kind) {
    case "integer":
      return Number(node.value);
    case "rational":
      return Number(node.numer) / Number(node.denom);
    case "float":
      return node.value;
    case "symbol":
      if (node.name === "inf") return Number.POSITIVE_INFINITY;
      if (node.name === "minf") return Number.NEGATIVE_INFINITY;
      if (node.name === "%pi") return Math.PI;
      if (node.name === "%e") return Math.E;
      return Number.NaN;
    case "string":
      return Number.NaN;
    case "apply":
      return numericEvalApply(node);
  }
}

function numericEvalApply(node: Extract<IRNode, { readonly kind: "apply" }>): number {
  const name = headName(node.head);
  const args = node.args;
  if (name === ADD.name) return args.reduce((sum, arg) => sum + numericEvalUnsafe(arg), 0);
  if (name === SUB.name && args.length === 2) return numericEvalUnsafe(args[0]) - numericEvalUnsafe(args[1]);
  if (name === MUL.name) return args.reduce((product, arg) => product * numericEvalUnsafe(arg), 1);
  if (name === DIV.name && args.length === 2) {
    const numer = numericEvalUnsafe(args[0]);
    const denom = numericEvalUnsafe(args[1]);
    if (denom === 0) {
      if (numer === 0) return Number.NaN;
      return Math.sign(numer) >= 0 ? Number.POSITIVE_INFINITY : Number.NEGATIVE_INFINITY;
    }
    return numer / denom;
  }
  if (name === NEG.name && args.length === 1) return -numericEvalUnsafe(args[0]);
  if (name === POW.name && args.length === 2) {
    const base = numericEvalUnsafe(args[0]);
    const exponent = numericEvalUnsafe(args[1]);
    if (Math.abs(base - 1) < 1e-10 && isInfiniteLike(exponent)) return Number.NaN;
    if (base === 0 && exponent === 0) return Number.NaN;
    if (isInfiniteLike(base) && exponent === 0) return Number.NaN;
    if (base < 0 && !Number.isInteger(exponent)) return Number.NaN;
    return base ** exponent;
  }
  if (name === SQRT.name && args.length === 1) {
    const value = numericEvalUnsafe(args[0]);
    return value < 0 ? Number.NaN : Math.sqrt(value);
  }
  if (name === EXP.name && args.length === 1) return safeExp(numericEvalUnsafe(args[0]));
  if (name === LOG.name && args.length === 1) {
    const value = numericEvalUnsafe(args[0]);
    if (value === 0) return Number.NEGATIVE_INFINITY;
    if (value < 0) return Number.NaN;
    return Math.log(value);
  }
  if (name === SIN.name && args.length === 1) {
    const value = numericEvalUnsafe(args[0]);
    return Number.isFinite(value) ? Math.sin(value) : Number.NaN;
  }
  if (name === COS.name && args.length === 1) {
    const value = numericEvalUnsafe(args[0]);
    return Number.isFinite(value) ? Math.cos(value) : Number.NaN;
  }
  if (name === TAN.name && args.length === 1) {
    const value = numericEvalUnsafe(args[0]);
    return Number.isFinite(value) ? Math.tan(value) : Number.NaN;
  }
  if (name === ATAN.name && args.length === 1) return Math.atan(numericEvalUnsafe(args[0]));
  if (name === SINH.name && args.length === 1) return Math.sinh(numericEvalUnsafe(args[0]));
  if (name === COSH.name && args.length === 1) return Math.cosh(numericEvalUnsafe(args[0]));
  if (name === TANH.name && args.length === 1) return Math.tanh(numericEvalUnsafe(args[0]));
  return Number.NaN;
}

function safeExp(value: number): number {
  if (value === Number.POSITIVE_INFINITY) return Number.POSITIVE_INFINITY;
  if (value === Number.NEGATIVE_INFINITY) return 0;
  return Math.exp(value);
}

function isInfiniteLike(value: number): boolean {
  return Number.isFinite(value) ? Math.abs(value) > INF_THRESHOLD : !Number.isNaN(value);
}

function toCoefficients(expr: IRNode, variable: IRNode): Frac[] {
  if (expr.kind === "integer") return [Frac.fromInt(expr.value)];
  if (expr.kind === "rational") return [new Frac(expr.numer, expr.denom)];
  if (expr.kind === "float") {
    const [numer, denom] = floatToRational(expr.value, 1_000_000n);
    return [new Frac(numer, denom)];
  }
  if (equals(expr, variable)) return [Frac.zero(), Frac.one()];
  if (expr.kind === "symbol") {
    throw new PolynomialError(`taylor: expression contains symbol ${JSON.stringify(expr.name)} other than the expansion variable`);
  }
  if (expr.kind !== "apply") {
    throw new PolynomialError(`taylor: unsupported expression ${JSON.stringify(expr)}`);
  }

  switch (headName(expr.head)) {
    case ADD.name: {
      let result = [Frac.zero()];
      for (const arg of expr.args) {
        result = coeffsAdd(result, toCoefficients(arg, variable));
      }
      return result;
    }
    case SUB.name: {
      if (expr.args.length !== 2) throw new PolynomialError("Sub must have exactly 2 args");
      return coeffsSub(toCoefficients(expr.args[0], variable), toCoefficients(expr.args[1], variable));
    }
    case NEG.name: {
      if (expr.args.length !== 1) throw new PolynomialError("Neg must have exactly 1 arg");
      return toCoefficients(expr.args[0], variable).map((c) => c.neg());
    }
    case MUL.name: {
      let result = [Frac.one()];
      for (const arg of expr.args) {
        result = coeffsMul(result, toCoefficients(arg, variable));
      }
      return result;
    }
    case POW.name: {
      if (expr.args.length !== 2) throw new PolynomialError("Pow must have exactly 2 args");
      const exponent = expr.args[1];
      if (exponent.kind !== "integer" || exponent.value < 0n || exponent.value > BigInt(Number.MAX_SAFE_INTEGER)) {
        throw new PolynomialError(`Pow exponent must be a non-negative integer literal, got ${nodeDebug(exponent)}`);
      }
      const base = toCoefficients(expr.args[0], variable);
      let result = [Frac.one()];
      for (let i = 0; i < Number(exponent.value); i += 1) {
        result = coeffsMul(result, base);
      }
      return result;
    }
    case DIV.name: {
      if (expr.args.length !== 2) throw new PolynomialError("Div must have exactly 2 args");
      const denom = literalFraction(expr.args[1], "Div: denominator must be a numeric literal for polynomial Taylor");
      return toCoefficients(expr.args[0], variable).map((c) => c.div(denom));
    }
    default:
      throw new PolynomialError(`taylor: unsupported operation ${JSON.stringify(headName(expr.head))} for polynomial input`);
  }
}

function coeffsAdd(a: readonly Frac[], b: readonly Frac[]): Frac[] {
  const n = Math.max(a.length, b.length);
  const out: Frac[] = [];
  for (let i = 0; i < n; i += 1) {
    out.push((a[i] ?? Frac.zero()).add(b[i] ?? Frac.zero()));
  }
  return out;
}

function coeffsSub(a: readonly Frac[], b: readonly Frac[]): Frac[] {
  const n = Math.max(a.length, b.length);
  const out: Frac[] = [];
  for (let i = 0; i < n; i += 1) {
    out.push((a[i] ?? Frac.zero()).sub(b[i] ?? Frac.zero()));
  }
  return out;
}

function coeffsMul(a: readonly Frac[], b: readonly Frac[]): Frac[] {
  if (a.length === 0 || b.length === 0) return [];
  const out = Array.from({ length: a.length + b.length - 1 }, () => Frac.zero());
  for (let i = 0; i < a.length; i += 1) {
    for (let j = 0; j < b.length; j += 1) {
      out[i + j] = out[i + j].add(a[i].mul(b[j]));
    }
  }
  return out;
}

function shiftPolynomial(coeffs: readonly Frac[], shift: Frac): Frac[] {
  const out: Frac[] = [];
  for (let k = 0; k < coeffs.length; k += 1) {
    let subtotal = Frac.zero();
    for (let i = k; i < coeffs.length; i += 1) {
      const ff = new Frac(fallingFactorial(i, k));
      subtotal = subtotal.add(ff.mul(coeffs[i]).mul(shift.powi(i - k)));
    }
    out.push(subtotal.div(new Frac(factorial(k))));
  }
  return out;
}

function fromCoefficients(coeffs: readonly Frac[], variable: IRNode, point: IRNode): IRNode {
  const terms: IRNode[] = [];
  for (let k = 0; k < coeffs.length; k += 1) {
    const coeff = coeffs[k];
    if (coeff.isZero()) continue;
    const coeffNode = coeff.toIrNode();
    if (k === 0) {
      terms.push(coeffNode);
      continue;
    }
    const delta = isIntegerValue(point, 0n) ? variable : app(SUB, [variable, point]);
    const base = k === 1 ? delta : app(POW, [delta, int(k)]);
    terms.push(coeff.isOne() ? base : app(MUL, [coeffNode, base]));
  }
  if (terms.length === 0) return int(0);
  if (terms.length === 1) return terms[0];
  return app(ADD, terms);
}

function toFraction(node: IRNode): Frac {
  return literalFraction(node, "taylor: expansion point must be a literal number");
}

function literalFraction(node: IRNode, messagePrefix: string): Frac {
  if (node.kind === "integer") return Frac.fromInt(node.value);
  if (node.kind === "rational") return new Frac(node.numer, node.denom);
  if (node.kind === "float") {
    const [numer, denom] = floatToRational(node.value, 1_000_000n);
    return new Frac(numer, denom);
  }
  throw new PolynomialError(`${messagePrefix}, got ${nodeDebug(node)}`);
}

function factorial(n: number): bigint {
  let out = 1n;
  for (let i = 2; i <= n; i += 1) {
    out *= BigInt(i);
  }
  return out;
}

function fallingFactorial(n: number, k: number): bigint {
  let out = 1n;
  for (let value = n - k + 1; value <= n; value += 1) {
    out *= BigInt(value);
  }
  return out;
}

function floatToRational(value: number, maxDenom: bigint): readonly [bigint, bigint] {
  if (!Number.isFinite(value)) throw new RangeError("float coefficient must be finite");
  if (value === 0) return [0n, 1n];
  const sign = value < 0 ? -1n : 1n;
  const absValue = Math.abs(value);
  let bestNumer = BigInt(Math.round(absValue));
  let bestDenom = 1n;
  let bestError = Math.abs(absValue - Number(bestNumer) / Number(bestDenom));
  for (let denominator = 1n; denominator <= maxDenom; denominator += 1n) {
    const numerator = BigInt(Math.round(absValue * Number(denominator)));
    const error = Math.abs(absValue - Number(numerator) / Number(denominator));
    if (error < bestError) {
      bestError = error;
      bestNumer = numerator;
      bestDenom = denominator;
    }
    if (bestError === 0) break;
  }
  return [sign * bestNumer, bestDenom];
}

function isIntegerValue(node: IRNode, value: bigint): boolean {
  return node.kind === "integer" && node.value === value;
}

function toBigInt(value: IntegerLike): bigint {
  if (typeof value === "bigint") return value;
  if (typeof value === "string") return BigInt(value);
  if (!Number.isSafeInteger(value)) {
    throw new RangeError("integer number inputs must be safe integers; pass a string or bigint for larger values");
  }
  return BigInt(value);
}

function gcd(a: bigint, b: bigint): bigint {
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a === 0n ? 1n : a;
}

function abs(n: bigint): bigint {
  return n < 0n ? -n : n;
}

function nodeDebug(node: IRNode): string {
  return JSON.stringify(node, (_key, value) => (typeof value === "bigint" ? value.toString() : value));
}
