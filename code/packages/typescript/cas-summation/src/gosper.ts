/**
 * Gosper's algorithm for indefinite hypergeometric summation.
 *
 * Track H2 port of `code/packages/python/cas-summation/src/cas_summation/gosper.py`
 * (Track H1, PR #5366).  See the Python source for the full mathematical
 * background.  The structure mirrors the Python module 1:1:
 *
 *   1. Structural decomposition of the summand into a hypergeometric
 *      product `poly(k) · ∏ base^exp(k) · ∏ Γ(k+s) / ∏ Γ(k+t)`.
 *   2. Compute the shift ratio `a(k+1)/a(k)` as two polynomials.
 *   3. Petkovšek-normalise the ratio: `r(k) = A(k)·C(k+1) / (B(k)·C(k))`
 *      with `gcd(A(k), B(k+h)) = 1` for every integer `h ≥ 0`.
 *   4. Bound the degree of `x(k)` in Gosper's key equation
 *      `A(k)·x(k+1) − B(k−1)·x(k) = C(k)` and solve the linear system
 *      via Gaussian elimination over rational coefficients.
 *   5. Reconstruct `T(k) = B(k−1)·x(k)·a(k) / C(k)` and return
 *      `T(hi+1) − T(lo)` as the closed-form IR.
 *
 * Coefficients are exact rationals built on `bigint` — no floats — to
 * match the Python `fractions.Fraction` semantics bit-for-bit.
 *
 * Defensive cap `MAX_POLY_DEGREE = 64` rejects polynomial exponents
 * larger than that during IR-to-poly conversion to prevent memory-bomb
 * inputs like `Pow(k, 10**9)`.
 */

import {
  ADD,
  DIV,
  MUL,
  NEG,
  POW,
  SUB,
  app,
  equals as irEquals,
  int,
  rational,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

import { GAMMA_FUNC } from "./index";

// ---------------------------------------------------------------------------
// Exact rational arithmetic (mirrors Python's `fractions.Fraction`).
// ---------------------------------------------------------------------------

/** A reduced rational with `denom > 0`. */
export interface Frac {
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

function fEq(a: Frac, b: Frac): boolean {
  return a.n === b.n && a.d === b.d;
}

function fIsZero(a: Frac): boolean {
  return a.n === 0n;
}

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

function fFromInt(n: bigint): Frac {
  return { n, d: 1n };
}

/** Integer power of a Frac.  `exp` may be negative. */
function fPow(base: Frac, exp: bigint): Frac {
  if (exp === 0n) return F1;
  if (exp < 0n) {
    if (base.n === 0n) throw new RangeError("0 to a negative power");
    return fPow({ n: base.d * (base.n < 0n ? -1n : 1n), d: absBig(base.n) }, -exp);
  }
  let result = F1;
  let b = base;
  let e = exp;
  while (e > 0n) {
    if ((e & 1n) === 1n) result = fMul(result, b);
    e >>= 1n;
    if (e > 0n) b = fMul(b, b);
  }
  return result;
}

// ---------------------------------------------------------------------------
// Univariate polynomial arithmetic over Frac.  `Poly` is the list of
// coefficients with `p[i]` the coefficient of `k^i`; trailing zeros are
// trimmed.  Zero polynomial → `[]`.
// ---------------------------------------------------------------------------

export type Poly = Frac[];

/**
 * Defensive cap on polynomial degree — without this, an adversarial
 * summand like `Pow(k, 10n ** 9n)` would balloon `irToPoly` into a
 * memory-bomb.  Gosper-summable expressions have very small polynomial
 * degree in practice (typically ≤ 5).
 */
export const MAX_POLY_DEGREE = 64;

function polyTrim(p: Poly): Poly {
  let n = p.length;
  while (n > 0 && fIsZero(p[n - 1])) n--;
  return p.slice(0, n);
}

function polyDeg(p: Poly): number {
  const pp = polyTrim(p);
  return pp.length - 1;
}

function polyAdd(a: Poly, b: Poly): Poly {
  const n = Math.max(a.length, b.length);
  const out: Frac[] = new Array(n);
  for (let i = 0; i < n; i++) {
    const ai = i < a.length ? a[i] : F0;
    const bi = i < b.length ? b[i] : F0;
    out[i] = fAdd(ai, bi);
  }
  return polyTrim(out);
}

function polySub(a: Poly, b: Poly): Poly {
  return polyAdd(a, b.map(fNeg));
}

function polyMul(a: Poly, b: Poly): Poly {
  const ta = polyTrim(a);
  const tb = polyTrim(b);
  if (ta.length === 0 || tb.length === 0) return [];
  const out: Frac[] = new Array(ta.length + tb.length - 1).fill(F0);
  for (let i = 0; i < ta.length; i++) {
    if (fIsZero(ta[i])) continue;
    for (let j = 0; j < tb.length; j++) {
      if (fIsZero(tb[j])) continue;
      out[i + j] = fAdd(out[i + j], fMul(ta[i], tb[j]));
    }
  }
  return polyTrim(out);
}

function polyScalar(p: Poly, c: Frac): Poly {
  if (fIsZero(c)) return [];
  return p.map((x) => fMul(x, c));
}

/**
 * Return `p(k + h)` as a new polynomial via the binomial expansion
 * `(k + h)^i = Σ_j C(i, j) · h^(i - j) · k^j`.
 */
function polyShift(p: Poly, h: bigint): Poly {
  const n = p.length;
  const out: Frac[] = new Array(n).fill(F0);
  for (let i = 0; i < n; i++) {
    if (fIsZero(p[i])) continue;
    // Pascal's row i.
    let binom = 1n;
    for (let j = 0; j <= i; j++) {
      // C(i, j) · h^(i - j) — integer arithmetic.
      const hpow = h ** BigInt(i - j);
      const term = fMul(p[i], fFromInt(binom * hpow));
      out[j] = fAdd(out[j], term);
      // Advance Pascal: next binom = binom * (i - j) / (j + 1).
      binom = (binom * BigInt(i - j)) / BigInt(j + 1);
    }
  }
  return polyTrim(out);
}

/**
 * Polynomial long division.  Returns `[q, r]` with `a = q·b + r` and
 * `deg(r) < deg(b)`.  `b` must be non-zero.
 */
function polyDivmod(a: Poly, b: Poly): [Poly, Poly] {
  const ta = polyTrim(a);
  const tb = polyTrim(b);
  if (tb.length === 0) throw new RangeError("polynomial division by zero");
  if (polyDeg(ta) < polyDeg(tb)) return [[], ta];
  const q: Frac[] = new Array(ta.length - tb.length + 1).fill(F0);
  let r = ta.slice();
  while (polyDeg(r) >= polyDeg(tb)) {
    const degDiff = polyDeg(r) - polyDeg(tb);
    const coeff = fDiv(r[r.length - 1], tb[tb.length - 1]);
    q[degDiff] = coeff;
    // Subtract coeff · k^degDiff · b from r.
    const shifted: Frac[] = new Array(degDiff).fill(F0).concat(
      tb.map((c) => fMul(c, coeff)),
    );
    r = polySub(r, shifted);
  }
  return [polyTrim(q), polyTrim(r)];
}

/** Monic GCD via Euclid; output has leading coefficient 1 (or empty). */
function polyGcd(a: Poly, b: Poly): Poly {
  let x = polyTrim(a);
  let y = polyTrim(b);
  while (y.length > 0) {
    const [, r] = polyDivmod(x, y);
    x = y;
    y = r;
  }
  if (x.length === 0) return [];
  // Monic-normalise.
  const lc = x[x.length - 1];
  return x.map((c) => fDiv(c, lc));
}

function polyEq(a: Poly, b: Poly): boolean {
  const ta = polyTrim(a);
  const tb = polyTrim(b);
  if (ta.length !== tb.length) return false;
  for (let i = 0; i < ta.length; i++) {
    if (!fEq(ta[i], tb[i])) return false;
  }
  return true;
}

/**
 * Solve `M · x = rhs` over the rationals via Gaussian elimination.
 * Returns the solution vector (under-determined systems pick free
 * variables = 0) or `undefined` if the system is inconsistent.
 */
function solveLinearSystem(matrix: Frac[][], rhs: Frac[]): Frac[] | undefined {
  if (matrix.length === 0) return rhs.length === 0 ? [] : undefined;
  const rows = matrix.length;
  const cols = matrix[0].length;
  const m: Frac[][] = matrix.map((row, i) => [...row, rhs[i]]);
  let row = 0;
  for (let col = 0; col < cols; col++) {
    let pivot = -1;
    for (let r = row; r < rows; r++) {
      if (!fIsZero(m[r][col])) {
        pivot = r;
        break;
      }
    }
    if (pivot === -1) continue;
    [m[row], m[pivot]] = [m[pivot], m[row]];
    const piv = m[row][col];
    m[row] = m[row].map((c) => fDiv(c, piv));
    for (let r = 0; r < rows; r++) {
      if (r === row) continue;
      const factor = m[r][col];
      if (fIsZero(factor)) continue;
      for (let i = 0; i <= cols; i++) {
        m[r][i] = fSub(m[r][i], fMul(factor, m[row][i]));
      }
    }
    row++;
  }
  // Inconsistency: 0 = non-zero anywhere.
  for (let r = 0; r < rows; r++) {
    let allZero = true;
    for (let c = 0; c < cols; c++) {
      if (!fIsZero(m[r][c])) {
        allZero = false;
        break;
      }
    }
    if (allZero && !fIsZero(m[r][cols])) return undefined;
  }
  // Read off the solution.
  const x: Frac[] = new Array(cols).fill(F0);
  const rowForCol = new Map<number, number>();
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < cols; c++) {
      if (fEq(m[r][c], F1)) {
        let isPivotCol = true;
        for (let r2 = 0; r2 < rows; r2++) {
          if (r2 !== r && !fIsZero(m[r2][c])) {
            isPivotCol = false;
            break;
          }
        }
        if (isPivotCol) {
          rowForCol.set(c, r);
          break;
        }
      }
    }
  }
  for (let c = 0; c < cols; c++) {
    const r = rowForCol.get(c);
    if (r !== undefined) x[c] = m[r][cols];
  }
  return x;
}

// ---------------------------------------------------------------------------
// IR ↔ polynomial bridge.
// ---------------------------------------------------------------------------

function rationalOf(node: IRNode): Frac | undefined {
  if (node.kind === "integer") return fFromInt(node.value);
  if (node.kind === "rational") return mkF(node.numer, node.denom);
  return undefined;
}

/**
 * Convert an IR expression that is a polynomial in `k` to a `Poly`.
 * Returns `undefined` if the expression has any non-polynomial structure
 * (division by k-bearing denominator, transcendentals, free non-k symbols,
 * negative/fractional exponents, or exponents above `MAX_POLY_DEGREE`).
 */
export function irToPoly(node: IRNode, k: IRNode): Poly | undefined {
  const r = rationalOf(node);
  if (r !== undefined) return [r];
  if (node.kind === "symbol") {
    if (irEquals(node, k)) return [F0, F1];
    return undefined;
  }
  if (node.kind !== "apply") return undefined;
  if (irEquals(node.head, NEG) && node.args.length === 1) {
    const inner = irToPoly(node.args[0], k);
    return inner === undefined ? undefined : polyScalar(inner, fFromInt(-1n));
  }
  if (irEquals(node.head, ADD)) {
    let out: Poly = [];
    for (const arg of node.args) {
      const sub = irToPoly(arg, k);
      if (sub === undefined) return undefined;
      out = polyAdd(out, sub);
    }
    return out;
  }
  if (irEquals(node.head, SUB) && node.args.length === 2) {
    const a = irToPoly(node.args[0], k);
    const b = irToPoly(node.args[1], k);
    if (a === undefined || b === undefined) return undefined;
    return polySub(a, b);
  }
  if (irEquals(node.head, MUL)) {
    let out: Poly = [F1];
    for (const arg of node.args) {
      const sub = irToPoly(arg, k);
      if (sub === undefined) return undefined;
      out = polyMul(out, sub);
    }
    return out;
  }
  if (irEquals(node.head, POW) && node.args.length === 2) {
    const [base, exp] = node.args;
    const basePoly = irToPoly(base, k);
    if (basePoly === undefined) return undefined;
    if (exp.kind !== "integer" || exp.value < 0n) return undefined;
    if (exp.value > BigInt(MAX_POLY_DEGREE)) return undefined;
    let result: Poly = [F1];
    const e = Number(exp.value);
    for (let i = 0; i < e; i++) {
      result = polyMul(result, basePoly);
      if (polyDeg(result) > MAX_POLY_DEGREE) return undefined;
    }
    return result;
  }
  if (irEquals(node.head, DIV) && node.args.length === 2) {
    const np = irToPoly(node.args[0], k);
    const dp = irToPoly(node.args[1], k);
    if (np === undefined || dp === undefined) return undefined;
    if (polyDeg(dp) !== 0) return undefined;
    return polyScalar(np, fDiv(F1, dp[0]));
  }
  return undefined;
}

function fracToIr(f: Frac): IRNode {
  if (f.d === 1n) return int(f.n);
  return rational(f.n, f.d);
}

function polyToIr(p: Poly, k: IRNode): IRNode {
  const tp = polyTrim(p);
  if (tp.length === 0) return int(0);
  const terms: IRNode[] = [];
  for (let i = 0; i < tp.length; i++) {
    const c = tp[i];
    if (fIsZero(c)) continue;
    if (i === 0) {
      terms.push(fracToIr(c));
    } else if (i === 1) {
      if (fEq(c, F1)) {
        terms.push(k);
      } else {
        terms.push(app(MUL, [fracToIr(c), k]));
      }
    } else {
      const power = app(POW, [k, int(i)]);
      if (fEq(c, F1)) {
        terms.push(power);
      } else {
        terms.push(app(MUL, [fracToIr(c), power]));
      }
    }
  }
  if (terms.length === 0) return int(0);
  if (terms.length === 1) return terms[0];
  return app(ADD, terms);
}

// ---------------------------------------------------------------------------
// Structural factoring: a(k) → (poly(k), exponentials, factorials).
// ---------------------------------------------------------------------------

interface Hyp {
  poly: Poly;
  expFactors: { base: Frac; exp: Poly }[];
  gammaShifts: bigint[];
  recipGammaShifts: bigint[];
}

function newHyp(): Hyp {
  return { poly: [F1], expFactors: [], gammaShifts: [], recipGammaShifts: [] };
}

/**
 * If `node = α·k + β` with integer α, β, return `[α, β]`; pure constants
 * come back as `[0n, β]`.  Otherwise `undefined`.
 */
function tryLinearInK(node: IRNode, k: IRNode): [bigint, bigint] | undefined {
  const p = irToPoly(node, k);
  if (p === undefined) return undefined;
  if (p.length === 0) return [0n, 0n];
  if (polyDeg(p) > 1) return undefined;
  const a = p.length >= 2 ? p[1] : F0;
  const b = p[0];
  if (a.d !== 1n || b.d !== 1n) return undefined;
  return [a.n, b.n];
}

function decompose(node: IRNode, k: IRNode, hyp?: Hyp): Hyp | undefined {
  const h = hyp ?? newHyp();
  // Polynomial sub-tree.
  const poly = irToPoly(node, k);
  if (poly !== undefined) {
    h.poly = polyMul(h.poly, poly);
    return h;
  }
  if (node.kind !== "apply") return undefined;
  if (irEquals(node.head, MUL)) {
    for (const arg of node.args) {
      if (decompose(arg, k, h) === undefined) return undefined;
    }
    return h;
  }
  if (irEquals(node.head, NEG) && node.args.length === 1) {
    const inner = decompose(node.args[0], k, h);
    if (inner === undefined) return undefined;
    inner.poly = polyScalar(inner.poly, fFromInt(-1n));
    return inner;
  }
  if (irEquals(node.head, DIV) && node.args.length === 2) {
    const [num, den] = node.args;
    if (decompose(num, k, h) === undefined) return undefined;
    const denPoly = irToPoly(den, k);
    if (denPoly !== undefined) {
      if (polyDeg(denPoly) !== 0 || denPoly.length === 0) return undefined;
      h.poly = polyScalar(h.poly, fDiv(F1, denPoly[0]));
      return h;
    }
    if (den.kind === "apply" && irEquals(den.head, GAMMA_FUNC) && den.args.length === 1) {
      const lin = tryLinearInK(den.args[0], k);
      if (lin === undefined || lin[0] !== 1n) return undefined;
      h.recipGammaShifts.push(lin[1]);
      return h;
    }
    return undefined;
  }
  if (irEquals(node.head, POW) && node.args.length === 2) {
    const [base, exp] = node.args;
    const basePoly = irToPoly(base, k);
    if (basePoly === undefined) return undefined;
    if (polyDeg(basePoly) !== 0 || basePoly.length === 0) return undefined;
    const b = basePoly[0];
    if (fIsZero(b)) return undefined;
    const expPoly = irToPoly(exp, k);
    if (expPoly === undefined) return undefined;
    if (polyDeg(expPoly) > 1) return undefined;
    h.expFactors.push({ base: b, exp: expPoly });
    return h;
  }
  if (irEquals(node.head, GAMMA_FUNC) && node.args.length === 1) {
    const lin = tryLinearInK(node.args[0], k);
    if (lin === undefined || lin[0] !== 1n) return undefined;
    h.gammaShifts.push(lin[1]);
    return h;
  }
  return undefined;
}

// ---------------------------------------------------------------------------
// Ratio computation: r(k) = a(k+1) / a(k).
// ---------------------------------------------------------------------------

function hypRatio(h: Hyp): [Poly, Poly] | undefined {
  const poly = h.poly;
  if (polyTrim(poly).length === 0) return undefined;
  let numer = polyShift(poly, 1n);
  let denom: Poly = poly.slice();
  for (const { base, exp } of h.expFactors) {
    if (polyDeg(exp) === 0) continue;
    const alpha = exp[1];
    if (alpha.d !== 1n) return undefined;
    const alphaInt = alpha.n;
    let factor: Frac;
    if (alphaInt >= 0n) {
      factor = fPow(base, alphaInt);
    } else {
      if (fIsZero(base)) return undefined;
      factor = fDiv(F1, fPow(base, -alphaInt));
    }
    numer = polyScalar(numer, factor);
  }
  for (const s of h.gammaShifts) {
    numer = polyMul(numer, [fFromInt(s), F1]);
  }
  for (const t of h.recipGammaShifts) {
    denom = polyMul(denom, [fFromInt(t), F1]);
  }
  return [numer, denom];
}

// ---------------------------------------------------------------------------
// Petkovšek normalisation.
// ---------------------------------------------------------------------------

function petkovsekNormalise(a: Poly, b: Poly): [Poly, Poly, Poly] | undefined {
  let A: Poly = a.slice();
  let B: Poly = b.slice();
  let C: Poly = [F1];
  const maxH = Math.max(polyDeg(A), polyDeg(B), 0) + 2;
  for (;;) {
    let peeled = false;
    for (let h = 0; h <= maxH; h++) {
      const Bshifted = polyShift(B, BigInt(h));
      const g = polyGcd(A, Bshifted);
      if (polyDeg(g) >= 1) {
        const [Anew, remA] = polyDivmod(A, g);
        if (remA.length > 0) return undefined;
        const gBack = polyShift(g, BigInt(-h));
        const [Bnew, remB] = polyDivmod(B, gBack);
        if (remB.length > 0) return undefined;
        let acc: Poly = [F1];
        for (let i = 1; i <= h; i++) {
          acc = polyMul(acc, polyShift(g, BigInt(-i)));
        }
        C = polyMul(C, acc);
        A = Anew;
        B = Bnew;
        peeled = true;
        break;
      }
    }
    if (!peeled) return [A, B, C];
  }
}

// ---------------------------------------------------------------------------
// Gosper degree bound + linear solver for x(k) in
//     A(k)·x(k+1) − B(k−1)·x(k) = C(k).
// ---------------------------------------------------------------------------

function gosperDegreeBound(A: Poly, B: Poly, C: Poly): number {
  const Bshifted = polyShift(B, -1n);
  const S = polyAdd(A, Bshifted);
  const D = polySub(A, Bshifted);
  const degS = polyDeg(S);
  const degD = polyDeg(D);
  const degC = polyDeg(C);
  let bound: number;
  if (degS > degD + 1) {
    bound = degC - degS;
  } else {
    const m = Math.max(polyDeg(A), polyDeg(Bshifted));
    if (m < 0) return 0;
    const Stop = m < S.length ? S[m] : F0;
    if (fIsZero(Stop)) {
      bound = degC - m;
    } else {
      const DatM1 = m - 1 >= 0 && m - 1 < D.length ? D[m - 1] : F0;
      // -2·D[m-1]/S[m] - 1 as a rational, then ceiling toward +∞.
      const cand = fSub(fDiv(fMul(fFromInt(-2n), DatM1), Stop), F1);
      let candInt: number;
      if (cand.n < 0n) {
        candInt = 0;
      } else {
        // Ceiling of n/d for non-negative rational.
        const q = cand.n / cand.d;
        const r = cand.n % cand.d;
        candInt = r === 0n ? Number(q) : Number(q) + 1;
      }
      bound = Math.max(degC - m, candInt);
    }
  }
  if (bound < 0) return -1;
  return bound + 1;
}

function solveKeyEquation(
  A: Poly,
  B: Poly,
  C: Poly,
  degBound: number,
): Poly | undefined {
  if (degBound < 0) return undefined;
  const nUnknowns = degBound + 1;
  const Bshifted = polyShift(B, -1n);
  const basisPolys: Poly[] = [];
  let maxDeg = 0;
  for (let i = 0; i < nUnknowns; i++) {
    const kPowI: Poly = new Array(i).fill(F0).concat([F1]);
    const kp1PowI = polyShift(kPowI, 1n);
    const left = polyMul(A, kp1PowI);
    const right = polyMul(Bshifted, kPowI);
    const bp = polySub(left, right);
    basisPolys.push(bp);
    if (polyDeg(bp) > maxDeg) maxDeg = polyDeg(bp);
  }
  const Ctrim = polyTrim(C);
  let rhsLen = Math.max(maxDeg + 1, Ctrim.length);
  if (rhsLen === 0) rhsLen = 1;
  const rhs: Frac[] = new Array(rhsLen).fill(F0);
  for (let j = 0; j < Ctrim.length; j++) rhs[j] = Ctrim[j];
  const matrix: Frac[][] = [];
  for (let j = 0; j < rhsLen; j++) {
    const row: Frac[] = [];
    for (let i = 0; i < nUnknowns; i++) {
      const bp = basisPolys[i];
      row.push(j < bp.length ? bp[j] : F0);
    }
    matrix.push(row);
  }
  const sol = solveLinearSystem(matrix, rhs);
  if (sol === undefined) return undefined;
  const xPoly = polyTrim(sol);
  if (xPoly.length === 0) {
    if (polyTrim(C).length > 0) return undefined;
    return [F0];
  }
  // Verify the solution.
  const xShifted = polyShift(xPoly, 1n);
  const lhs = polySub(polyMul(A, xShifted), polyMul(Bshifted, xPoly));
  if (!polyEq(lhs, C)) return undefined;
  return xPoly;
}

// ---------------------------------------------------------------------------
// Top-level entry: try Gosper on ∑_{k=lo}^{hi} summand.
// ---------------------------------------------------------------------------

/**
 * Attempt Gosper's algorithm on `∑_{k=lo}^{hi} summand`.  Returns the IR
 * closed form `T(hi+1) − T(lo)` on success, or `undefined` to signal
 * fall-through.  Mirrors `try_gosper_sum` in the Python module.
 */
export function tryGosperSum(
  summand: IRNode,
  k: IRNode,
  lo: IRNode,
  hi: IRNode,
): IRNode | undefined {
  const hyp = decompose(summand, k);
  if (hyp === undefined) return undefined;
  const ratio = hypRatio(hyp);
  if (ratio === undefined) return undefined;
  const [aTop, bBot] = ratio;
  if (polyTrim(hyp.poly).length === 0) return int(0);
  const norm = petkovsekNormalise(aTop, bBot);
  if (norm === undefined) return undefined;
  const [Anorm, Bnorm, Cpoly] = norm;
  const degBound = gosperDegreeBound(Anorm, Bnorm, Cpoly);
  const xPoly = solveKeyEquation(Anorm, Bnorm, Cpoly, degBound);
  if (xPoly === undefined || polyTrim(xPoly).length === 0) return undefined;

  // Reconstruct T(k) = B(k−1)·x(k)·a(k) / C(k).  Cancel common polynomial
  // factors against C(k) so removable singularities at the substitution
  // boundary (k = lo) don't break the closed form.
  const BatKminus1 = polyShift(Bnorm, -1n);
  let fullNumerPoly = polyMul(polyMul(BatKminus1, xPoly), hyp.poly);
  let denomPoly: Poly = Cpoly.slice();
  const g = polyGcd(fullNumerPoly, denomPoly);
  if (polyDeg(g) >= 1) {
    const [nq, remN] = polyDivmod(fullNumerPoly, g);
    const [dq, remD] = polyDivmod(denomPoly, g);
    if (remN.length === 0 && remD.length === 0) {
      fullNumerPoly = nq;
      denomPoly = dq;
    }
  }

  // Build the transcendental IR ∏ base^exp(k) · ∏ Γ(k+s) / ∏ Γ(k+t).
  function buildTranscendentalPart(): IRNode {
    const pieces: IRNode[] = [];
    for (const { base, exp } of hyp!.expFactors) {
      pieces.push(app(POW, [fracToIr(base), polyToIr(exp, k)]));
    }
    for (const s of hyp!.gammaShifts) {
      const arg = s === 0n ? k : app(ADD, [k, int(s)]);
      pieces.push(app(GAMMA_FUNC, [arg]));
    }
    const denominatorGammas: IRNode[] = [];
    for (const t of hyp!.recipGammaShifts) {
      const arg = t === 0n ? k : app(ADD, [k, int(t)]);
      denominatorGammas.push(app(GAMMA_FUNC, [arg]));
    }
    if (pieces.length === 0 && denominatorGammas.length === 0) return int(1);
    const numer = pieces.length === 0
      ? int(1)
      : pieces.length === 1
        ? pieces[0]
        : app(MUL, pieces);
    if (denominatorGammas.length === 0) return numer;
    const denom = denominatorGammas.length === 1
      ? denominatorGammas[0]
      : app(MUL, denominatorGammas);
    return app(DIV, [numer, denom]);
  }
  const transcendentalIr = buildTranscendentalPart();

  function substitute(node: IRNode, from: IRNode, to: IRNode): IRNode {
    if (irEquals(node, from)) return to;
    if (node.kind !== "apply") return node;
    return app(node.head, node.args.map((arg) => substitute(arg, from, to)));
  }

  function tAt(kValue: IRNode): IRNode {
    const numerIr = polyToIr(fullNumerPoly, k);
    const denomIr = polyToIr(denomPoly, k);
    const numerAt = substitute(numerIr, k, kValue);
    const denomAt = substitute(denomIr, k, kValue);
    const transAt = substitute(transcendentalIr, k, kValue);
    return app(DIV, [app(MUL, [numerAt, transAt]), denomAt]);
  }

  const hiPlusOne = app(ADD, [hi, int(1)]);
  return app(SUB, [tAt(hiPlusOne), tAt(lo)]);
}

// Re-exports for testing.
export const __test = {
  polyAdd,
  polyMul,
  polyShift,
  polyGcd,
  polyDivmod,
  polyDeg,
  fFromInt,
  mkF,
  decompose,
  hypRatio,
};
