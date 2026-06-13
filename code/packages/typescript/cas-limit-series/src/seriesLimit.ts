/**
 * `trySeriesLimit` — Taylor-series-expansion fallback for limits.
 *
 * This module is the **Track J2** TypeScript port of the Python Track J1
 * fallback (`series_limit.py`). It mirrors that file 1:1 so the two
 * implementations stay in lockstep.
 *
 * Why a separate fallback?
 * ------------------------
 * L'Hopital can fail for several reasons: differentiation blows up the
 * expression size, the recursion is bounded, and the simplifier may
 * not collapse intermediate forms back to a recognisable `0/0`.
 * A series expansion sidesteps all of that — polynomial arithmetic
 * stays small and exact, and reading the leading order is a constant-
 * time table lookup once the expansion exists.
 *
 * Algorithm
 * ---------
 * For `limit(f(x) / g(x), x, a)` where direct sub gives `0/0`:
 *
 *   1. Translate the limit point to the origin.
 *        a = 0          ⇒ u = x
 *        a finite ≠ 0   ⇒ u = x − a
 *        a = ±∞         ⇒ u = 1/x (not implemented here — falls through)
 *   2. Taylor-expand both numerator and denominator to bounded order N,
 *      starting at N = 4, using a transcendental-aware series ring.
 *   3. Read off leading coefficients
 *        N(u) = c_p · u^p + O(u^{p+1})
 *        D(u) = d_q · u^q + O(u^{q+1})
 *      and dispatch on p vs q.
 *   4. If both leading orders are still zero, bump N += 2 (max 12) and
 *      retry.
 *
 * Bounds
 * ------
 *   - `maxOrder` defaults to 12 and is hard-capped at 12.
 *   - The series ring uses exact `bigint` rationals.
 *   - No recursion: a fixed loop runs at most five iterations
 *     (orders 4, 6, 8, 10, 12).
 *   - Inputs are IRNode trees, never strings — no `eval` of user data.
 */

import {
  ADD,
  COS,
  DIV,
  EXP,
  LOG,
  MUL,
  NEG,
  POW,
  SIN,
  SUB,
  TAN,
  app,
  equals,
  headName,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

// ---------------------------------------------------------------------------
// Hard cap and error type
// ---------------------------------------------------------------------------

const MAX_ORDER_LIMIT = 12;

class SeriesError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "SeriesError";
  }
}

// ---------------------------------------------------------------------------
// RatQ — exact rational over BigInt
// ---------------------------------------------------------------------------
//
// A self-contained `Fraction`-like type matching Python's
// `fractions.Fraction`. Keeps every coefficient exact; no float drift.

function bgcd(a: bigint, b: bigint): bigint {
  if (a < 0n) a = -a;
  if (b < 0n) b = -b;
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a === 0n ? 1n : a;
}

class RatQ {
  readonly n: bigint;
  readonly d: bigint;

  constructor(n: bigint, d: bigint = 1n) {
    if (d === 0n) throw new RangeError("RatQ denominator is zero");
    if (d < 0n) {
      n = -n;
      d = -d;
    }
    if (n === 0n) {
      this.n = 0n;
      this.d = 1n;
      return;
    }
    const g = bgcd(n < 0n ? -n : n, d);
    this.n = n / g;
    this.d = d / g;
  }

  static zero(): RatQ {
    return new RatQ(0n, 1n);
  }

  static one(): RatQ {
    return new RatQ(1n, 1n);
  }

  isZero(): boolean {
    return this.n === 0n;
  }

  isPositive(): boolean {
    return this.n > 0n;
  }

  eq(rhs: RatQ): boolean {
    return this.n === rhs.n && this.d === rhs.d;
  }

  neg(): RatQ {
    return new RatQ(-this.n, this.d);
  }

  add(rhs: RatQ): RatQ {
    return new RatQ(this.n * rhs.d + rhs.n * this.d, this.d * rhs.d);
  }

  sub(rhs: RatQ): RatQ {
    return new RatQ(this.n * rhs.d - rhs.n * this.d, this.d * rhs.d);
  }

  mul(rhs: RatQ): RatQ {
    return new RatQ(this.n * rhs.n, this.d * rhs.d);
  }

  div(rhs: RatQ): RatQ {
    if (rhs.n === 0n) throw new RangeError("RatQ: division by zero");
    return new RatQ(this.n * rhs.d, this.d * rhs.n);
  }

  toIrNode(): IRNode {
    return this.d === 1n ? int(this.n) : rational(this.n, this.d);
  }
}

// ---------------------------------------------------------------------------
// Series — truncated power series a_0 + a_1·u + ... + a_N·u^N
// ---------------------------------------------------------------------------
//
// `coeffs` is always exactly `order + 1` long. Higher-order terms are
// silently dropped — that is what truncation means.

class Series {
  readonly coeffs: RatQ[];
  readonly order: number;

  constructor(coeffs: RatQ[], order: number) {
    if (order < 0) throw new SeriesError("series order must be non-negative");
    let c = coeffs;
    if (c.length < order + 1) {
      c = c.slice();
      while (c.length < order + 1) c.push(RatQ.zero());
    } else if (c.length > order + 1) {
      c = c.slice(0, order + 1);
    }
    this.coeffs = c;
    this.order = order;
  }

  static constant(c: RatQ, order: number): Series {
    return new Series([c], order);
  }

  static variable(order: number): Series {
    if (order < 1) return new Series([RatQ.zero()], order);
    return new Series([RatQ.zero(), RatQ.one()], order);
  }

  add(other: Series): Series {
    const n = this.order;
    const out: RatQ[] = [];
    for (let i = 0; i <= n; i += 1) out.push(this.coeffs[i].add(other.coeffs[i]));
    return new Series(out, n);
  }

  sub(other: Series): Series {
    const n = this.order;
    const out: RatQ[] = [];
    for (let i = 0; i <= n; i += 1) out.push(this.coeffs[i].sub(other.coeffs[i]));
    return new Series(out, n);
  }

  neg(): Series {
    return new Series(this.coeffs.map((c) => c.neg()), this.order);
  }

  mul(other: Series): Series {
    const n = this.order;
    const out: RatQ[] = Array.from({ length: n + 1 }, () => RatQ.zero());
    for (let i = 0; i <= n; i += 1) {
      const ai = this.coeffs[i];
      if (ai.isZero()) continue;
      for (let j = 0; j <= n - i; j += 1) {
        out[i + j] = out[i + j].add(ai.mul(other.coeffs[j]));
      }
    }
    return new Series(out, n);
  }

  scaled(c: RatQ): Series {
    return new Series(this.coeffs.map((a) => c.mul(a)), this.order);
  }

  leadingIndex(): number {
    for (let k = 0; k < this.coeffs.length; k += 1) {
      if (!this.coeffs[k].isZero()) return k;
    }
    return -1;
  }

  /**
   * `1 / self` provided `self(0) ≠ 0`. Newton-style recursion:
   *   b_0 = 1/a_0
   *   b_k = -1/a_0 · sum_{j=1..k} a_j · b_{k-j}
   */
  reciprocal(): Series {
    const a = this.coeffs;
    const n = this.order;
    if (a[0].isZero()) throw new SeriesError("reciprocal of series with zero constant term");
    const b: RatQ[] = Array.from({ length: n + 1 }, () => RatQ.zero());
    b[0] = RatQ.one().div(a[0]);
    for (let k = 1; k <= n; k += 1) {
      let s = RatQ.zero();
      for (let j = 1; j <= k; j += 1) s = s.add(a[j].mul(b[k - j]));
      b[k] = s.neg().div(a[0]);
    }
    return new Series(b, n);
  }

  /** `self ** k`, non-negative integer k, via repeated squaring. */
  integerPower(k: number): Series {
    if (k < 0) throw new SeriesError("series integerPower requires k >= 0");
    if (k === 0) return Series.constant(RatQ.one(), this.order);
    let result = Series.constant(RatQ.one(), this.order);
    // eslint-disable-next-line @typescript-eslint/no-this-alias
    let base: Series = this;
    let e = k;
    while (e > 0) {
      if ((e & 1) === 1) result = result.mul(base);
      e >>= 1;
      if (e > 0) base = base.mul(base);
    }
    return result;
  }

  /**
   * `self(inner(u))` provided `inner(0) == 0`. Sum of `a_k · inner^k`
   * truncated at `order`.
   */
  composeWithZeroConstant(inner: Series): Series {
    if (!inner.coeffs[0].isZero()) {
      throw new SeriesError("composeWithZeroConstant: inner series has nonzero constant");
    }
    const n = this.order;
    let result = Series.constant(RatQ.zero(), n);
    let innerPow = Series.constant(RatQ.one(), n);  // inner^0 = 1
    for (let k = 0; k <= n; k += 1) {
      if (!this.coeffs[k].isZero()) {
        result = result.add(innerPow.scaled(this.coeffs[k]));
      }
      if (k < n) innerPow = innerPow.mul(inner);
    }
    return result;
  }
}

// ---------------------------------------------------------------------------
// Known transcendental Taylor series (around u = 0)
// ---------------------------------------------------------------------------

function factorial(n: number): bigint {
  let out = 1n;
  for (let k = 2; k <= n; k += 1) out *= BigInt(k);
  return out;
}

function seriesExp(order: number): Series {
  const coeffs: RatQ[] = [];
  for (let k = 0; k <= order; k += 1) coeffs.push(new RatQ(1n, factorial(k)));
  return new Series(coeffs, order);
}

function seriesSin(order: number): Series {
  const coeffs: RatQ[] = Array.from({ length: order + 1 }, () => RatQ.zero());
  let sign = 1n;
  for (let k = 1; k <= order; k += 2) {
    coeffs[k] = new RatQ(sign, factorial(k));
    sign = -sign;
  }
  return new Series(coeffs, order);
}

function seriesCos(order: number): Series {
  const coeffs: RatQ[] = Array.from({ length: order + 1 }, () => RatQ.zero());
  let sign = 1n;
  for (let k = 0; k <= order; k += 2) {
    coeffs[k] = new RatQ(sign, factorial(k));
    sign = -sign;
  }
  return new Series(coeffs, order);
}

function seriesLogOnePlus(order: number): Series {
  const coeffs: RatQ[] = Array.from({ length: order + 1 }, () => RatQ.zero());
  let sign = 1n;
  for (let k = 1; k <= order; k += 1) {
    coeffs[k] = new RatQ(sign, BigInt(k));
    sign = -sign;
  }
  return new Series(coeffs, order);
}

function seriesTan(order: number): Series {
  // tan = sin / cos. cos has nonzero constant, so direct reciprocal works.
  return seriesSin(order).mul(seriesCos(order).reciprocal());
}

// ---------------------------------------------------------------------------
// IR → Series translation
// ---------------------------------------------------------------------------

function toRat(node: IRNode): RatQ {
  if (node.kind === "integer") return new RatQ(node.value, 1n);
  if (node.kind === "rational") return new RatQ(node.numer, node.denom);
  if (node.kind === "float") {
    // Bounded conversion: keep up to denominator 1_000_000. Same idea as
    // Python's `Fraction(value).limit_denominator()`.
    return floatToRat(node.value);
  }
  throw new SeriesError(`expected literal, got ${nodeDebug(node)}`);
}

function floatToRat(value: number): RatQ {
  if (!Number.isFinite(value)) throw new SeriesError("float coefficient must be finite");
  if (value === 0) return RatQ.zero();
  const sign = value < 0 ? -1n : 1n;
  const av = Math.abs(value);
  let bestN = BigInt(Math.round(av));
  let bestD = 1n;
  let bestErr = Math.abs(av - Number(bestN));
  const MAX_D = 1_000_000n;
  for (let d = 1n; d <= MAX_D; d += 1n) {
    const numer = BigInt(Math.round(av * Number(d)));
    const err = Math.abs(av - Number(numer) / Number(d));
    if (err < bestErr) {
      bestErr = err;
      bestN = numer;
      bestD = d;
    }
    if (bestErr === 0) break;
  }
  return new RatQ(sign * bestN, bestD);
}

function expand(expr: IRNode, variable: IRNode, order: number): Series {
  // --- literal numbers ---
  if (expr.kind === "integer" || expr.kind === "rational" || expr.kind === "float") {
    return Series.constant(toRat(expr), order);
  }

  // --- the expansion variable ---
  if (expr.kind === "symbol") {
    if (equals(expr, variable)) return Series.variable(order);
    // Opaque symbol: cannot Taylor-expand around an unknown constant.
    throw new SeriesError(`unsupported symbol ${expr.name}`);
  }

  if (expr.kind !== "apply" || expr.head.kind !== "symbol") {
    throw new SeriesError(`unsupported expression: ${nodeDebug(expr)}`);
  }

  const h = headName(expr.head);
  const args = expr.args;

  // --- arithmetic ---
  if (h === ADD.name) {
    let result = Series.constant(RatQ.zero(), order);
    for (const a of args) result = result.add(expand(a, variable, order));
    return result;
  }
  if (h === SUB.name) {
    if (args.length !== 2) throw new SeriesError("Sub expects 2 args");
    return expand(args[0], variable, order).sub(expand(args[1], variable, order));
  }
  if (h === NEG.name) {
    if (args.length !== 1) throw new SeriesError("Neg expects 1 arg");
    return expand(args[0], variable, order).neg();
  }
  if (h === MUL.name) {
    let result = Series.constant(RatQ.one(), order);
    for (const a of args) result = result.mul(expand(a, variable, order));
    return result;
  }
  if (h === DIV.name) {
    if (args.length !== 2) throw new SeriesError("Div expects 2 args");
    const ns = expand(args[0], variable, order);
    const ds = expand(args[1], variable, order);
    if (!ds.coeffs[0].isZero()) return ns.mul(ds.reciprocal());
    // Inner Div by vanishing series — would change the leading order
    // of the surrounding expansion. Top-level `trySeriesLimit` handles
    // the f/g case separately.
    throw new SeriesError("inner Div by series vanishing at 0");
  }
  if (h === POW.name) {
    if (args.length !== 2) throw new SeriesError("Pow expects 2 args");
    const [base, expNode] = args;
    if (expNode.kind === "integer") {
      const k = Number(expNode.value);
      if (!Number.isSafeInteger(k)) throw new SeriesError("Pow exponent too large");
      if (k >= 0) return expand(base, variable, order).integerPower(k);
      const baseSer = expand(base, variable, order);
      if (baseSer.coeffs[0].isZero()) {
        throw new SeriesError("Pow negative-int exponent over vanishing base");
      }
      return baseSer.reciprocal().integerPower(-k);
    }
    throw new SeriesError("Pow exponent must be a non-negative integer literal");
  }

  // --- transcendentals ---
  if (h === EXP.name) {
    if (args.length !== 1) throw new SeriesError("Exp expects 1 arg");
    const inner = expand(args[0], variable, order);
    if (!inner.coeffs[0].isZero()) throw new SeriesError("Exp with nonzero constant inner term");
    return seriesExp(order).composeWithZeroConstant(inner);
  }
  if (h === SIN.name) {
    if (args.length !== 1) throw new SeriesError("Sin expects 1 arg");
    const inner = expand(args[0], variable, order);
    if (!inner.coeffs[0].isZero()) throw new SeriesError("Sin with nonzero constant inner term");
    return seriesSin(order).composeWithZeroConstant(inner);
  }
  if (h === COS.name) {
    if (args.length !== 1) throw new SeriesError("Cos expects 1 arg");
    const inner = expand(args[0], variable, order);
    if (!inner.coeffs[0].isZero()) throw new SeriesError("Cos with nonzero constant inner term");
    return seriesCos(order).composeWithZeroConstant(inner);
  }
  if (h === TAN.name) {
    if (args.length !== 1) throw new SeriesError("Tan expects 1 arg");
    const inner = expand(args[0], variable, order);
    if (!inner.coeffs[0].isZero()) throw new SeriesError("Tan with nonzero constant inner term");
    return seriesTan(order).composeWithZeroConstant(inner);
  }
  if (h === LOG.name) {
    if (args.length !== 1) throw new SeriesError("Log expects 1 arg");
    const inner = expand(args[0], variable, order);
    const c0 = inner.coeffs[0];
    if (!c0.eq(RatQ.one())) {
      throw new SeriesError("Log with constant inner term != 1; not in rational ring");
    }
    // log(1 + (inner - 1)) where (inner - 1)(0) = 0.
    const shiftedCoeffs = inner.coeffs.slice();
    shiftedCoeffs[0] = RatQ.zero();
    const shifted = new Series(shiftedCoeffs, order);
    return seriesLogOnePlus(order).composeWithZeroConstant(shifted);
  }

  throw new SeriesError(`unsupported head: ${h}`);
}

// ---------------------------------------------------------------------------
// Top-level entry point
// ---------------------------------------------------------------------------

/**
 * Recognise both `Div(N, D)` and `Mul(N, Pow(D, -1))` as quotients.
 * Returns `null` for any other top-level shape — the Taylor fallback
 * only fires on rational structures.
 */
function splitQuotient(expr: IRNode): readonly [IRNode, IRNode] | null {
  if (expr.kind !== "apply") return null;
  if (headName(expr.head) === DIV.name && expr.args.length === 2) {
    return [expr.args[0], expr.args[1]];
  }
  if (headName(expr.head) === MUL.name && expr.args.length === 2) {
    const [a, b] = expr.args;
    if (isPowNegOne(b)) return [a, (b as { args: readonly IRNode[] }).args[0]];
    if (isPowNegOne(a)) return [b, (a as { args: readonly IRNode[] }).args[0]];
  }
  return null;
}

function isPowNegOne(node: IRNode): boolean {
  return node.kind === "apply"
    && headName(node.head) === POW.name
    && node.args.length === 2
    && node.args[1].kind === "integer"
    && node.args[1].value === -1n;
}

/**
 * Substitute `variable := variable + point` so the original
 * `variable = point` corresponds to `variable = 0` after the shift.
 *
 * Done by an explicit IR walk rather than via `cas-substitution`,
 * matching the Python reference's intent of a totally local rewrite.
 */
function shiftToOrigin(expr: IRNode, variable: IRNode, point: IRNode): IRNode {
  if (point.kind === "integer" && point.value === 0n) return expr;
  const go = (node: IRNode): IRNode => {
    if (node.kind === "symbol") {
      if (equals(node, variable)) return app(ADD, [variable, point]);
      return node;
    }
    if (node.kind === "apply") {
      return app(node.head, node.args.map(go));
    }
    return node;
  };
  return go(expr);
}

/**
 * Taylor-series fallback for `limit(expr, variable, point)`.
 *
 * Returns:
 *   - an integer or rational literal on success,
 *   - `sym("inf")` / `sym("minf")` on a divergent ratio,
 *   - `null` if the fallback cannot determine the value (caller falls
 *     through to an unevaluated `Limit(...)`).
 *
 * `point` must be a literal number — limits at ±∞ are not yet handled
 * here (they would need a `u = 1/x` rewrite) and return `null`.
 *
 * `maxOrder` is clamped to `[4, 12]` — keeping polynomial multiplication
 * bounded by O(N^2) within a fixed, small constant.
 */
export function trySeriesLimit(
  expr: IRNode,
  variable: IRNode,
  point: IRNode,
  maxOrder: number = MAX_ORDER_LIMIT,
): IRNode | null {
  let cap = Math.floor(maxOrder);
  if (!Number.isFinite(cap)) cap = MAX_ORDER_LIMIT;
  if (cap < 4) cap = 4;
  if (cap > MAX_ORDER_LIMIT) cap = MAX_ORDER_LIMIT;

  const nd = splitQuotient(expr);
  if (nd === null) return null;
  const [numer, denom] = nd;

  if (point.kind === "symbol" && (point.name === "inf" || point.name === "minf")) {
    return null;
  }
  if (point.kind !== "integer" && point.kind !== "rational" && point.kind !== "float") {
    return null;
  }

  const shiftedN = shiftToOrigin(numer, variable, point);
  const shiftedD = shiftToOrigin(denom, variable, point);

  let order = 4;
  while (order <= cap) {
    let nSer: Series;
    let dSer: Series;
    try {
      nSer = expand(shiftedN, variable, order);
      dSer = expand(shiftedD, variable, order);
    } catch (err) {
      if (err instanceof SeriesError) return null;
      throw err;
    }

    const p = nSer.leadingIndex();
    const q = dSer.leadingIndex();

    // Both fully zero — bump order and retry. The expansion may simply
    // not be deep enough yet.
    if (p === -1 && q === -1) {
      order += 2;
      continue;
    }

    // Numerator vanishes harder than denominator within our tracked order.
    if (p === -1 && q !== -1) return int(0);

    // Denominator vanishes harder than numerator — divergence; sign
    // follows the leading numerator coefficient.
    if (q === -1 && p !== -1) {
      return nSer.coeffs[p].isPositive() ? sym("inf") : sym("minf");
    }

    const cp = nSer.coeffs[p];
    const dq = dSer.coeffs[q];
    if (p > q) return int(0);
    if (p < q) {
      const signVal = cp.div(dq);
      return signVal.isPositive() ? sym("inf") : sym("minf");
    }
    // p == q — exact rational ratio.
    return cp.div(dq).toIrNode();
  }

  return null;
}

function nodeDebug(node: IRNode): string {
  return JSON.stringify(node, (_key, value) => (typeof value === "bigint" ? value.toString() : value));
}

// Internal exports for whitebox tests.
export const __internal = { Series, RatQ, SeriesError, MAX_ORDER_LIMIT };
