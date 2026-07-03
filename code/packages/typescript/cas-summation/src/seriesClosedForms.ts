/**
 * Canonical infinite-series closed-form recogniser — Track I2.
 *
 * TypeScript port of
 * ``code/packages/python/cas-summation/src/cas_summation/series_closed_forms.py``
 * (Track I1, PR #5382).  See the Python source for the full mathematical
 * background — this module mirrors it 1:1.
 *
 * Recognised series
 * -----------------
 *   ∑_{k=1}^∞ 1/k^(2m)        (m = 1..6)   → (2π)^(2m) · |B_{2m}| / (2·(2m)!)
 *   ∑_{k=1}^∞ (-1)^(k-1)/k                  → log(2)
 *   ∑_{k=1}^∞ (-1)^(k-1)/k^(2m) (m = 1..3) → (1 − 2^(1-2m)) · ζ(2m)
 *   ∑_{k=0}^∞ 1/k!                          → %e
 *   ∑_{k=0}^∞ x^k/k!                        → exp(x)
 *   ∑_{k=0}^∞ (-1)^k · x^(2k)/(2k)!         → cos(x)
 *   ∑_{k=0}^∞ (-1)^k · x^(2k+1)/(2k+1)!     → sin(x)
 *   ∑_{k=0}^∞ x^(2k)/(2k)!                  → cosh(x)
 *   ∑_{k=0}^∞ x^(2k+1)/(2k+1)!              → sinh(x)
 *
 * Design constraints (per Python reference):
 *   - One generic Bernoulli helper, computed via the textbook recurrence
 *     ``B_0 = 1; Σ_{j=0}^{n} C(n+1, j) · B_j = 0``.  Bounded depth
 *     (n ≤ 12).
 *   - Exact arithmetic via BigInt-backed Frac (mirrors fractions.Fraction).
 *   - Only fires when ``hi = %inf``; finite ``hi`` returns ``undefined``.
 */

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
  app,
  equals as irEquals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

import { GAMMA_FUNC } from "./index";

// π / %e as IR symbols (MACSYMA convention).
const PI = sym("%pi");
const E_SYM = sym("%e");

/** Maximum even-zeta exponent — spec table covers k = 2..12 (m = 1..6). */
const MAX_ZETA_M = 6;
/** Maximum even-eta exponent — spec table covers m = 1..3. */
const MAX_ETA_M = 3;

// ---------------------------------------------------------------------------
// Exact rational arithmetic on BigInt.  Mirrors Python's ``Fraction`` and the
// gosper.ts ``Frac`` type but kept module-local so we don't widen the public
// API of cas-summation.
// ---------------------------------------------------------------------------

interface Frac {
  readonly n: bigint;
  readonly d: bigint;
}

function absBig(value: bigint): bigint {
  return value < 0n ? -value : value;
}

function gcdBig(a: bigint, b: bigint): bigint {
  let x = absBig(a);
  let y = absBig(b);
  while (y !== 0n) {
    const t = y;
    y = x % y;
    x = t;
  }
  return x === 0n ? 1n : x;
}

function mkF(n: bigint, d: bigint): Frac {
  if (d === 0n) throw new RangeError("Frac denominator cannot be zero");
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  const g = gcdBig(n, d);
  return { n: n / g, d: d / g };
}

const F0: Frac = { n: 0n, d: 1n };
const F1: Frac = { n: 1n, d: 1n };

function fAdd(a: Frac, b: Frac): Frac {
  return mkF(a.n * b.d + b.n * a.d, a.d * b.d);
}

function fSub(a: Frac, b: Frac): Frac {
  return mkF(a.n * b.d - b.n * a.d, a.d * b.d);
}

function fMul(a: Frac, b: Frac): Frac {
  return mkF(a.n * b.n, a.d * b.d);
}

function fDiv(a: Frac, b: Frac): Frac {
  if (b.n === 0n) throw new RangeError("Frac division by zero");
  return mkF(a.n * b.d, a.d * b.n);
}

function fNeg(a: Frac): Frac {
  return { n: -a.n, d: a.d };
}

function fAbs(a: Frac): Frac {
  return a.n < 0n ? { n: -a.n, d: a.d } : a;
}

function fFromInt(n: bigint): Frac {
  return { n, d: 1n };
}

function fToIr(c: Frac): IRNode {
  if (c.d === 1n) return int(c.n);
  return rational(c.n, c.d);
}

// ---------------------------------------------------------------------------
// IR construction helpers (private)
// ---------------------------------------------------------------------------

function powIr(base: IRNode, exp: IRNode): IRNode {
  return app(POW, [base, exp]);
}

function mulIr(a: IRNode, b: IRNode): IRNode {
  return app(MUL, [a, b]);
}

function divIr(a: IRNode, b: IRNode): IRNode {
  return app(DIV, [a, b]);
}

function isIntNode(node: IRNode, value: bigint): boolean {
  return node.kind === "integer" && node.value === value;
}

/** True for ``-1`` whether stored as ``Integer(-1)`` or ``Neg(1)``. */
function isNegOneBase(node: IRNode): boolean {
  if (isIntNode(node, -1n)) return true;
  return (
    node.kind === "apply" &&
    irEquals(node.head, NEG) &&
    node.args.length === 1 &&
    isIntNode(node.args[0], 1n)
  );
}

/** True iff ``node`` is structurally constant in ``k``. */
function isConstantInK(node: IRNode, k: IRNode): boolean {
  if (irEquals(node, k)) return false;
  if (node.kind !== "apply") return true;
  return node.args.every((arg) => isConstantInK(arg, k));
}

// ---------------------------------------------------------------------------
// Bernoulli numbers (one generic helper) and ζ(2m) / η(2m) coefficients
// ---------------------------------------------------------------------------

/**
 * Cache of Bernoulli numbers computed up to the maximum index we ever
 * need (2 · MAX_ZETA_M = 12).  Mutated lazily on first call.
 */
const BERNOULLI_CACHE: (Frac | undefined)[] = [];

/**
 * Return ``B_n`` (the n-th Bernoulli number) as an exact ``Frac``.
 *
 * Computed via the textbook recurrence:
 *   B_0 = 1
 *   B_n = − (1 / (n+1)) · Σ_{j=0}^{n-1} C(n+1, j) · B_j   (n ≥ 1)
 *
 * Pure iterative ``for`` loop, depth ``n``.  Caller only ever asks for
 * ``n ≤ 12``, so the helper is provably terminating in O(n²) BigInt ops.
 *
 * Convention: ``B_1 = −1/2`` (Knuth / MACSYMA).  Matches the Python port.
 */
export function bernoulliRational(n: number): Frac {
  if (n < 0 || !Number.isInteger(n)) {
    throw new RangeError(`Bernoulli index must be a non-negative integer, got ${n}`);
  }
  const cached = BERNOULLI_CACHE[n];
  if (cached !== undefined) return cached;
  // Build all values from 0 up to n in one pass; cache each.
  const startFrom = BERNOULLI_CACHE.length;
  // Ensure index 0 exists in the cache before the loop.
  if (BERNOULLI_CACHE[0] === undefined) {
    BERNOULLI_CACHE[0] = F1;
  }
  for (let m = Math.max(1, startFrom); m <= n; m++) {
    // B_m = − (1 / (m+1)) · Σ_{j=0}^{m-1} C(m+1, j) · B_j
    let total: Frac = F0;
    // Iterative binomial: C(m+1, 0) = 1, then update C(m+1, j) → C(m+1, j+1).
    let binom = 1n;
    const mBig = BigInt(m);
    for (let j = 0; j < m; j++) {
      const bj = BERNOULLI_CACHE[j];
      if (bj === undefined) throw new Error("Bernoulli cache hole");
      total = fAdd(total, fMul({ n: binom, d: 1n }, bj));
      // C(m+1, j+1) = C(m+1, j) · (m + 1 − j) / (j + 1).  Exact integer
      // division because binomials are integers.
      const jBig = BigInt(j);
      binom = (binom * (mBig + 1n - jBig)) / (jBig + 1n);
    }
    BERNOULLI_CACHE[m] = fDiv(fNeg(total), { n: mBig + 1n, d: 1n });
  }
  const result = BERNOULLI_CACHE[n];
  if (result === undefined) throw new Error("Bernoulli cache failed to populate");
  return result;
}

/**
 * Return the rational coefficient ``c`` such that ``ζ(2m) = c · π^(2m)``.
 *
 *   c = 2^(2m) · |B_{2m}| / (2 · (2m)!)
 */
function zetaEvenCoeff(m: number): Frac {
  if (m < 1) {
    throw new RangeError(`zeta-even index must be ≥ 1, got ${m}`);
  }
  const b = bernoulliRational(2 * m);
  let factorial2m = 1n;
  for (let i = 1; i <= 2 * m; i++) {
    factorial2m *= BigInt(i);
  }
  const twoToTwoM = 1n << BigInt(2 * m); // 2^(2m).
  // c = 2^(2m) · |B_{2m}| / (2 · (2m)!)
  return fDiv(fMul({ n: twoToTwoM, d: 1n }, fAbs(b)), { n: 2n * factorial2m, d: 1n });
}

/**
 * Return the rational coefficient ``c`` such that ``η(2m) = c · π^(2m)``.
 *
 *   η(2m) = (1 − 2^(1−2m)) · ζ(2m)
 */
function etaEvenCoeff(m: number): Frac {
  if (m < 1) {
    throw new RangeError(`eta-even index must be ≥ 1, got ${m}`);
  }
  // 1 − 2^(1−2m) = 1 − 1/2^(2m-1).
  const twoExp = 1n << BigInt(2 * m - 1);
  const oneMinus = fSub(F1, { n: 1n, d: twoExp });
  return fMul(oneMinus, zetaEvenCoeff(m));
}

/**
 * Build the IR for ``coeff · π^power``.
 *
 * Emits ``π^power / denom`` when the coefficient is ``1/denom``
 * (canonical form ``π²/6`` rather than ``(1/6)·π²``); otherwise emits
 * the general ``coeff · π^power`` shape.
 */
function piPowerWithCoeff(coeff: Frac, power: number): IRNode {
  if (coeff.n === 1n && coeff.d > 1n) {
    return divIr(powIr(PI, int(power)), int(coeff.d));
  }
  return mulIr(fToIr(coeff), powIr(PI, int(power)));
}

// ---------------------------------------------------------------------------
// Pattern recognisers — each matches a structural shape and returns the IR
// for the closed form, or undefined when the shape doesn't match.
// ---------------------------------------------------------------------------

/** Match ``1/k^m`` (or ``1/k`` ≡ m=1) and return ``m``; else undefined. */
function extractInvKPow(f: IRNode, k: IRNode): number | undefined {
  if (f.kind !== "apply" || !irEquals(f.head, DIV) || f.args.length !== 2) {
    return undefined;
  }
  const [numer, denom] = f.args;
  if (!isIntNode(numer, 1n)) return undefined;
  if (irEquals(denom, k)) return 1;
  if (
    denom.kind === "apply" &&
    irEquals(denom.head, POW) &&
    denom.args.length === 2 &&
    irEquals(denom.args[0], k) &&
    denom.args[1].kind === "integer" &&
    denom.args[1].value >= 1n
  ) {
    return Number(denom.args[1].value);
  }
  return undefined;
}

/** Match ``(-1)^(k-1) / k^m`` and return ``m``; else undefined. */
function extractAltInvKPow(f: IRNode, k: IRNode): number | undefined {
  if (f.kind !== "apply" || !irEquals(f.head, DIV) || f.args.length !== 2) {
    return undefined;
  }
  const [numer, denom] = f.args;
  // Numerator: (-1)^(k-1).
  if (
    numer.kind !== "apply" ||
    !irEquals(numer.head, POW) ||
    numer.args.length !== 2 ||
    !isNegOneBase(numer.args[0])
  ) {
    return undefined;
  }
  const exp = numer.args[1];
  if (
    exp.kind !== "apply" ||
    !irEquals(exp.head, SUB) ||
    exp.args.length !== 2 ||
    !irEquals(exp.args[0], k) ||
    !isIntNode(exp.args[1], 1n)
  ) {
    return undefined;
  }
  // Denominator: k or k^m.
  if (irEquals(denom, k)) return 1;
  if (
    denom.kind === "apply" &&
    irEquals(denom.head, POW) &&
    denom.args.length === 2 &&
    irEquals(denom.args[0], k) &&
    denom.args[1].kind === "integer" &&
    denom.args[1].value >= 1n
  ) {
    return Number(denom.args[1].value);
  }
  return undefined;
}

/** ``Σ_{k=1}^∞ 1/k^(2m) → ζ(2m) · π^(2m)`` for ``m = 1..6``. */
function tryZeta2m(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  if (!isIntNode(lo, 1n)) return undefined;
  const mExp = extractInvKPow(f, k);
  if (mExp === undefined) return undefined;
  if (mExp % 2 !== 0) return undefined; // Odd zeta is not closed form.
  const m = mExp / 2;
  if (m < 1 || m > MAX_ZETA_M) return undefined;
  return piPowerWithCoeff(zetaEvenCoeff(m), 2 * m);
}

/** ``Σ_{k=1}^∞ (-1)^(k-1)/k^(2m) → η(2m) · π^(2m)`` for ``m = 1..3``. */
function tryEta2m(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  if (!isIntNode(lo, 1n)) return undefined;
  const mExp = extractAltInvKPow(f, k);
  if (mExp === undefined) return undefined;
  if (mExp % 2 !== 0) return undefined;
  const m = mExp / 2;
  if (m < 1 || m > MAX_ETA_M) return undefined;
  return piPowerWithCoeff(etaEvenCoeff(m), 2 * m);
}

/** ``Σ_{k=1}^∞ (-1)^(k-1)/k → log(2)`` (Mercator series). */
function tryEta1(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  if (!isIntNode(lo, 1n)) return undefined;
  const mExp = extractAltInvKPow(f, k);
  if (mExp !== 1) return undefined;
  return app(LOG, [int(2)]);
}

/** True iff ``node = GammaFunc(k + 1)`` (= ``k!``). */
function matchGammaKp1(node: IRNode, k: IRNode): boolean {
  if (
    node.kind !== "apply" ||
    !irEquals(node.head, GAMMA_FUNC) ||
    node.args.length !== 1
  ) {
    return false;
  }
  const arg = node.args[0];
  return (
    arg.kind === "apply" &&
    irEquals(arg.head, ADD) &&
    arg.args.length === 2 &&
    irEquals(arg.args[0], k) &&
    isIntNode(arg.args[1], 1n)
  );
}

/**
 * True iff ``node = GammaFunc(slope·k + intercept + 1)``.
 *
 * Matches the IR ``GammaFunc(Add(Mul(slope, k), intercept+1))`` used for
 * ``(slope·k + intercept)!``.  E.g. ``(2k)!`` is ``GammaFunc(2k + 1)``;
 * ``(2k+1)!`` is ``GammaFunc(2k + 2)``.
 */
function matchGammaOfLinearInKPlus1(
  node: IRNode,
  k: IRNode,
  slope: number,
  intercept: number,
): boolean {
  if (
    node.kind !== "apply" ||
    !irEquals(node.head, GAMMA_FUNC) ||
    node.args.length !== 1
  ) {
    return false;
  }
  const arg = node.args[0];
  if (arg.kind !== "apply" || !irEquals(arg.head, ADD) || arg.args.length !== 2) {
    return false;
  }
  const [left, right] = arg.args;
  if (
    left.kind !== "apply" ||
    !irEquals(left.head, MUL) ||
    left.args.length !== 2 ||
    !isIntNode(left.args[0], BigInt(slope)) ||
    !irEquals(left.args[1], k)
  ) {
    return false;
  }
  return isIntNode(right, BigInt(intercept + 1));
}

/** ``Σ_{k=0}^∞ 1/k! → %e``. */
function tryESeries(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  if (!isIntNode(lo, 0n)) return undefined;
  if (f.kind !== "apply" || !irEquals(f.head, DIV) || f.args.length !== 2) {
    return undefined;
  }
  const [numer, denom] = f.args;
  if (!isIntNode(numer, 1n)) return undefined;
  if (!matchGammaKp1(denom, k)) return undefined;
  return E_SYM;
}

/**
 * If ``node = Pow(x, k)`` with ``x`` constant in ``k`` and ``x ≠ k``,
 * return ``x``; else undefined.
 */
function extractPowOfXInK(node: IRNode, k: IRNode): IRNode | undefined {
  if (
    node.kind !== "apply" ||
    !irEquals(node.head, POW) ||
    node.args.length !== 2
  ) {
    return undefined;
  }
  const [base, exp] = node.args;
  if (!irEquals(exp, k)) return undefined;
  if (irEquals(base, k)) return undefined;
  if (!isConstantInK(base, k)) return undefined;
  return base;
}

/** ``Σ_{k=0}^∞ x^k/k! → exp(x)`` (symbolic ``x ≠ k``). */
function tryExpSeries(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  if (!isIntNode(lo, 0n)) return undefined;
  if (f.kind !== "apply" || !irEquals(f.head, DIV) || f.args.length !== 2) {
    return undefined;
  }
  const [numer, denom] = f.args;
  const x = extractPowOfXInK(numer, k);
  if (x === undefined) return undefined;
  if (!matchGammaKp1(denom, k)) return undefined;
  return app(EXP, [x]);
}

/**
 * If ``node = Pow(x, slope·k + intercept)`` (or ``Pow(x, slope·k)`` when
 * ``intercept == 0``) with ``x`` constant in ``k``, return ``x``.
 */
function extractPowOfXInLinearK(
  node: IRNode,
  k: IRNode,
  slope: number,
  intercept: number,
): IRNode | undefined {
  if (
    node.kind !== "apply" ||
    !irEquals(node.head, POW) ||
    node.args.length !== 2
  ) {
    return undefined;
  }
  const [base, exp] = node.args;
  if (irEquals(base, k) || !isConstantInK(base, k)) return undefined;
  // Bare slope·k form (intercept = 0).
  if (intercept === 0) {
    if (
      exp.kind !== "apply" ||
      !irEquals(exp.head, MUL) ||
      exp.args.length !== 2 ||
      !isIntNode(exp.args[0], BigInt(slope)) ||
      !irEquals(exp.args[1], k)
    ) {
      return undefined;
    }
    return base;
  }
  // slope·k + intercept form.
  if (exp.kind !== "apply" || !irEquals(exp.head, ADD) || exp.args.length !== 2) {
    return undefined;
  }
  const [left, right] = exp.args;
  if (
    left.kind !== "apply" ||
    !irEquals(left.head, MUL) ||
    left.args.length !== 2 ||
    !isIntNode(left.args[0], BigInt(slope)) ||
    !irEquals(left.args[1], k)
  ) {
    return undefined;
  }
  if (!isIntNode(right, BigInt(intercept))) return undefined;
  return base;
}

/**
 * Generic ``Σ_{k=0}^∞ (-1)^k · x^(slope·k + intercept) / (slope·k + intercept)!``.
 *
 * Used by :func:`tryCosSeries` (slope=2, intercept=0, head=COS) and
 * :func:`trySinSeries` (slope=2, intercept=1, head=SIN).
 *
 * Expected IR shape:
 *   Mul(Pow(-1, k), Div(Pow(x, slope·k + intercept),
 *                       GammaFunc(slope·k + intercept + 1)))
 *
 * Symmetric: tries both operand orders.
 */
function tryAltTaylorSeries(
  f: IRNode,
  k: IRNode,
  lo: IRNode,
  slope: number,
  intercept: number,
  head: IRNode,
): IRNode | undefined {
  if (!isIntNode(lo, 0n)) return undefined;
  if (f.kind !== "apply" || !irEquals(f.head, MUL) || f.args.length !== 2) {
    return undefined;
  }
  const [a, b] = f.args;

  function tryOrientation(signTerm: IRNode, body: IRNode): IRNode | undefined {
    // signTerm must be (-1)^k.
    if (
      signTerm.kind !== "apply" ||
      !irEquals(signTerm.head, POW) ||
      signTerm.args.length !== 2 ||
      !isNegOneBase(signTerm.args[0]) ||
      !irEquals(signTerm.args[1], k)
    ) {
      return undefined;
    }
    // body must be Div(Pow(x, slope·k + intercept), GammaFunc(... + 1)).
    if (
      body.kind !== "apply" ||
      !irEquals(body.head, DIV) ||
      body.args.length !== 2
    ) {
      return undefined;
    }
    const [numer, denom] = body.args;
    const x = extractPowOfXInLinearK(numer, k, slope, intercept);
    if (x === undefined) return undefined;
    if (!matchGammaOfLinearInKPlus1(denom, k, slope, intercept)) return undefined;
    return app(head, [x]);
  }

  const r1 = tryOrientation(a, b);
  if (r1 !== undefined) return r1;
  return tryOrientation(b, a);
}

/** ``Σ_{k=0}^∞ (−1)^k · x^(2k) / (2k)! → cos(x)``. */
function tryCosSeries(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  return tryAltTaylorSeries(f, k, lo, 2, 0, COS);
}

/** ``Σ_{k=0}^∞ (−1)^k · x^(2k+1) / (2k+1)! → sin(x)``. */
function trySinSeries(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  return tryAltTaylorSeries(f, k, lo, 2, 1, SIN);
}

/**
 * Generic ``Σ_{k=0}^∞ x^(slope·k + intercept) / (slope·k + intercept)!``.
 *
 * Returns ``head(x)`` (``cosh(x)`` or ``sinh(x)``) when the shape matches.
 * The sign factor is absent (hyperbolic series); the body is just
 * ``Div(Pow(x, …), GammaFunc(… + 1))``.
 */
function tryHyperbolicTaylorSeries(
  f: IRNode,
  k: IRNode,
  lo: IRNode,
  slope: number,
  intercept: number,
  head: IRNode,
): IRNode | undefined {
  if (!isIntNode(lo, 0n)) return undefined;
  if (f.kind !== "apply" || !irEquals(f.head, DIV) || f.args.length !== 2) {
    return undefined;
  }
  const [numer, denom] = f.args;
  const x = extractPowOfXInLinearK(numer, k, slope, intercept);
  if (x === undefined) return undefined;
  if (!matchGammaOfLinearInKPlus1(denom, k, slope, intercept)) return undefined;
  return app(head, [x]);
}

/** ``Σ_{k=0}^∞ x^(2k) / (2k)! → cosh(x)``. */
function tryCoshSeries(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  return tryHyperbolicTaylorSeries(f, k, lo, 2, 0, COSH);
}

/** ``Σ_{k=0}^∞ x^(2k+1) / (2k+1)! → sinh(x)``. */
function trySinhSeries(f: IRNode, k: IRNode, lo: IRNode): IRNode | undefined {
  return tryHyperbolicTaylorSeries(f, k, lo, 2, 1, SINH);
}

// ---------------------------------------------------------------------------
// Public dispatcher
// ---------------------------------------------------------------------------

/**
 * Return the closed form for a recognised canonical infinite series, or
 * undefined when no pattern matches.
 *
 * Mirrors :func:`series_closed_forms.try_closed_form_series` in the
 * Python reference.  Only fires when ``hi = %inf``; finite ``hi``
 * returns ``undefined`` so the caller falls through to Faulhaber /
 * geometric / Gosper.
 */
export function tryClosedFormSeries(
  summand: IRNode,
  k: IRNode,
  lo: IRNode,
  hi: IRNode,
): IRNode | undefined {
  // Infinite-bound only.
  if (
    hi.kind !== "symbol" ||
    (hi.name !== "inf" && hi.name !== "%inf")
  ) {
    return undefined;
  }
  // The patterns are non-overlapping; order matters only for clarity.
  const patterns = [
    tryZeta2m,
    tryEta2m,
    tryEta1,
    tryESeries,
    tryExpSeries,
    tryCosSeries,
    trySinSeries,
    tryCoshSeries,
    trySinhSeries,
  ];
  for (const pattern of patterns) {
    const result = pattern(summand, k, lo);
    if (result !== undefined) return result;
  }
  return undefined;
}
