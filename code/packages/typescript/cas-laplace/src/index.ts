/**
 * Laplace and inverse Laplace transforms over symbolic IR.
 *
 * This module provides:
 *
 * - `laplaceTransform(f, t, s)` — forward Laplace transform via table
 *   lookup and linearity rules.
 * - `inverseLaplace(F, s, t)` — inverse Laplace transform via a two-stage
 *   pipeline: direct table matching followed by a full partial-fraction
 *   decomposition engine.
 *
 * The partial-fraction engine handles three classes that the direct table
 * cannot:
 *
 * 1. **Improper fractions** — polynomial long division extracts the
 *    polynomial part; a constant quotient contributes a DiracDelta(t) term.
 *
 * 2. **Repeated rational poles** — Laurent expansion via formal power
 *    series (no symbolic differentiation needed).
 *
 * 3. **Irreducible quadratic factors** — complex-conjugate poles produce
 *    exp(−αt)·cos(βt) / exp(−αt)·sin(βt) pairs by completing the square.
 *
 * All arithmetic in the polynomial engine is exact, using a `Frac` type
 * backed by arbitrary-precision `bigint`.
 */

import {
  ADD,
  COS,
  COSH,
  DIV,
  EXP,
  MUL,
  NEG,
  POW,
  SIN,
  SINH,
  SQRT,
  SUB,
  app,
  equals,
  int,
  rational,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const LAPLACE = sym("Laplace");
export const ILT = sym("ILT");
export const DIRAC_DELTA = sym("DiracDelta");
export const UNIT_STEP = sym("UnitStep");

export type EvalFn = (node: IRNode) => IRNode;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

export function laplaceTransform(f: IRNode, t: IRNode, s: IRNode): IRNode {
  // Linearity: L{f + g} = L{f} + L{g}
  const addArgs = binaryArgs(f, ADD);
  if (addArgs !== undefined) {
    return bin(ADD, laplaceTransform(addArgs[0], t, s), laplaceTransform(addArgs[1], t, s));
  }

  // Linearity: L{c·f} = c·L{f}
  const extracted = extractCoeffAndFn(f, t);
  if (extracted !== undefined && !isInt(extracted.coeff, 1n)) {
    return bin(MUL, extracted.coeff, laplaceTransform(extracted.body, t, s));
  }

  return tableLookup(f, t, s) ?? app(LAPLACE, [f, t, s]);
}

export function inverseLaplace(f: IRNode, s: IRNode, t: IRNode): IRNode {
  return inverseLookup(f, s, t) ?? app(ILT, [f, s, t]);
}

export function laplaceHandler(expr: IRNode, evalFn: EvalFn): IRNode {
  const args = applyArgs(expr, LAPLACE);
  if (args === undefined || args.length !== 3 || args[1].kind !== "symbol" || args[2].kind !== "symbol") return expr;
  return evalFn(laplaceTransform(args[0], args[1], args[2]));
}

export function iltHandler(expr: IRNode, evalFn: EvalFn): IRNode {
  const args = applyArgs(expr, ILT);
  if (args === undefined || args.length !== 3 || args[1].kind !== "symbol" || args[2].kind !== "symbol") return expr;
  return evalFn(inverseLaplace(args[0], args[1], args[2]));
}

export function diracDeltaHandler(expr: IRNode): IRNode {
  const args = applyArgs(expr, DIRAC_DELTA);
  return args !== undefined && args.length === 1 && isInt(args[0], 0n) ? int(1) : expr;
}

export function unitStepHandler(expr: IRNode): IRNode {
  const args = applyArgs(expr, UNIT_STEP);
  if (args === undefined || args.length !== 1 || args[0].kind !== "integer") return expr;
  if (args[0].value < 0n) return int(0);
  if (args[0].value > 0n) return int(1);
  return rational(1, 2);
}

export function buildLaplaceHandlerTable(): ReadonlyMap<string, (expr: IRNode, evalFn: EvalFn) => IRNode> {
  return new Map([
    ["Laplace", laplaceHandler],
    ["ILT", iltHandler],
    ["DiracDelta", (expr) => diracDeltaHandler(expr)],
    ["UnitStep", (expr) => unitStepHandler(expr)],
  ]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Forward transform table
// ─────────────────────────────────────────────────────────────────────────────
//
// The table is a sequential lookup: each pattern matcher returns the transform
// result or undefined to fall through to the next entry.  Order matters —
// t^n·trig patterns for n≥2 must appear before the n=1 entry.

function tableLookup(f: IRNode, t: IRNode, s: IRNode): IRNode | undefined {
  // L{1} = 1/s
  if (isOne(f)) return bin(DIV, int(1), s);

  // L{t^n} = n! / s^{n+1}
  const n = matchPowerOfT(f, t);
  if (n !== undefined) return bin(DIV, int(factorial(n)), bin(POW, s, int(n + 1n)));

  // L{exp(at)} = 1/(s-a)
  const expShift = matchUnaryLinear(f, EXP, t);
  if (expShift !== undefined) return bin(DIV, int(1), bin(SUB, s, expShift));

  // L{sin(ωt)} = ω/(s²+ω²)
  const sinOmega = matchUnaryLinear(f, SIN, t);
  if (sinOmega !== undefined) return bin(DIV, sinOmega, sumSSqParamSq(s, sinOmega));

  // L{cos(ωt)} = s/(s²+ω²)
  const cosOmega = matchUnaryLinear(f, COS, t);
  if (cosOmega !== undefined) return bin(DIV, s, sumSSqParamSq(s, cosOmega));

  // L{sinh(at)} = a/(s²-a²)
  const sinhA = matchUnaryLinear(f, SINH, t);
  if (sinhA !== undefined) return bin(DIV, sinhA, subSSqParamSq(s, sinhA));

  // L{cosh(at)} = s/(s²-a²)
  const coshA = matchUnaryLinear(f, COSH, t);
  if (coshA !== undefined) return bin(DIV, s, subSSqParamSq(s, coshA));

  // L{DiracDelta(t)} = 1, L{UnitStep(t)} = 1/s
  if (isApplyOfVar(f, DIRAC_DELTA, t)) return int(1);
  if (isApplyOfVar(f, UNIT_STEP, t)) return bin(DIV, int(1), s);

  // L{exp(at)·trig(ωt)}: shifted oscillator pairs
  const expTrig = matchExpTimesTrig(f, t);
  if (expTrig !== undefined) {
    const shifted = bin(SUB, s, expTrig.shift);
    const denom = bin(ADD, bin(POW, shifted, int(2)), bin(POW, expTrig.omega, int(2)));
    return equals(expTrig.trigHead, SIN) ? bin(DIV, expTrig.omega, denom) : bin(DIV, shifted, denom);
  }

  // L{t^n·exp(at)}: n! / (s-a)^{n+1}
  const tExp = matchTPowerTimesExp(f, t);
  if (tExp !== undefined) {
    return bin(DIV, int(factorial(tExp.power)), bin(POW, bin(SUB, s, tExp.shift), int(tExp.power + 1n)));
  }

  // L{t^n·sin(ωt)} / L{t^n·cos(ωt)} for n ≥ 2 — must come BEFORE the n=1 case below.
  // Formulas derived by repeated differentiation of L{sin/cos}:
  //   L{t²·sin(ωt)} = 2ω(3s²−ω²) / (s²+ω²)³
  //   L{t²·cos(ωt)} = 2s(s²−3ω²) / (s²+ω²)³
  //   L{t³·sin(ωt)} = 24ωs(s²−ω²) / (s²+ω²)⁴
  //   L{t³·cos(ωt)} = 6(s⁴−6s²ω²+ω⁴) / (s²+ω²)⁴
  const tnTrig = matchTnTimesTrig(f, t);
  if (tnTrig !== undefined) {
    const { power: tn, trigHead, omega } = tnTrig;
    const s2 = bin(POW, s, int(2));
    const w2 = bin(POW, omega, int(2));
    const s2pw2 = bin(ADD, s2, w2);

    if (equals(trigHead, SIN)) {
      if (tn === 2n) {
        // Numerator: 2ω · (3s² − ω²)
        const num = bin(MUL, bin(MUL, int(2), omega), bin(SUB, bin(MUL, int(3), s2), w2));
        return bin(DIV, num, bin(POW, s2pw2, int(3)));
      }
      if (tn === 3n) {
        // Numerator: 24ω · s · (s² − ω²)
        const num = bin(MUL, bin(MUL, int(24), omega), bin(MUL, s, bin(SUB, s2, w2)));
        return bin(DIV, num, bin(POW, s2pw2, int(4)));
      }
      return undefined; // n ≥ 4: fall through to unevaluated
    } else {
      // COS
      if (tn === 2n) {
        // Numerator: 2s · (s² − 3ω²)
        const num = bin(MUL, bin(MUL, int(2), s), bin(SUB, s2, bin(MUL, int(3), w2)));
        return bin(DIV, num, bin(POW, s2pw2, int(3)));
      }
      if (tn === 3n) {
        // Numerator: 6 · (s⁴ − 6s²ω² + ω⁴)
        const s4 = bin(POW, s, int(4));
        const w4 = bin(POW, omega, int(4));
        const inner = bin(ADD, bin(SUB, s4, bin(MUL, int(6), bin(MUL, s2, w2))), w4);
        return bin(DIV, bin(MUL, int(6), inner), bin(POW, s2pw2, int(4)));
      }
      return undefined; // n ≥ 4: fall through to unevaluated
    }
  }

  // L{t·sin(ωt)} = 2ωs / (s²+ω²)²,  L{t·cos(ωt)} = (s²−ω²) / (s²+ω²)²
  const tTrig = matchTTimesTrig(f, t);
  if (tTrig !== undefined) {
    const denom = bin(POW, sumSSqParamSq(s, tTrig.omega), int(2));
    return equals(tTrig.trigHead, SIN)
      ? bin(DIV, bin(MUL, int(2), bin(MUL, tTrig.omega, s)), denom)
      : bin(DIV, bin(SUB, bin(POW, s, int(2)), bin(POW, tTrig.omega, int(2))), denom);
  }

  return undefined;
}

// ─────────────────────────────────────────────────────────────────────────────
// Inverse transform: direct table then partial-fraction engine
// ─────────────────────────────────────────────────────────────────────────────

function inverseLookup(f: IRNode, s: IRNode, t: IRNode): IRNode | undefined {
  // ── Step 1: Direct pattern matching ─────────────────────────────────────
  const div = binaryArgs(f, DIV);
  if (div !== undefined) {
    const [num, den] = div;

    // 1/s → UnitStep(t)
    if (isInt(num, 1n) && equals(den, s)) return app(UNIT_STEP, [t]);

    if (isInt(num, 1n)) {
      // 1/(s-a) → exp(at)
      const shift = matchSMinusA(den, s);
      if (shift !== undefined) return app(EXP, [bin(MUL, shift, t)]);

      // 1/s^n  (n ≥ 2) → t^{n-1} / (n-1)!
      const pow = matchPowOf(den, s);
      if (pow !== undefined && pow >= 2n) {
        const power = pow === 2n ? t : bin(POW, t, int(pow - 1n));
        return pow === 2n ? power : bin(DIV, power, int(factorial(pow - 1n)));
      }
    }

    // ω/(s²+ω²) → sin(ωt),  s/(s²+ω²) → cos(ωt)
    const plusParam = matchSSqPlusParamSq(den, s);
    if (plusParam !== undefined) {
      if (equals(num, plusParam)) return app(SIN, [bin(MUL, plusParam, t)]);
      if (equals(num, s)) return app(COS, [bin(MUL, plusParam, t)]);
    }

    // a/(s²-a²) → sinh(at),  s/(s²-a²) → cosh(at)
    const minusParam = matchSSqMinusParamSq(den, s);
    if (minusParam !== undefined) {
      if (equals(num, minusParam)) return app(SINH, [bin(MUL, minusParam, t)]);
      if (equals(num, s)) return app(COSH, [bin(MUL, minusParam, t)]);
    }
  }

  // ── Step 2: Partial-fraction decomposition engine ────────────────────────
  return inversePF(f, s, t);
}

// ─────────────────────────────────────────────────────────────────────────────
// Fraction arithmetic (exact rational numbers over bigint)
// ─────────────────────────────────────────────────────────────────────────────
//
// A `Frac` is always stored in reduced form with d > 0.
// The `frac()` constructor enforces this invariant.

type Frac = { readonly n: bigint; readonly d: bigint };

/** Construct a reduced Frac. */
function frac(n: bigint, d: bigint = 1n): Frac {
  if (d === 0n) throw new Error("frac: division by zero");
  if (d < 0n) {
    n = -n;
    d = -d;
  }
  const g = bigGcd(n < 0n ? -n : n, d);
  if (g === 0n) return { n: 0n, d: 1n };
  return { n: n / g, d: d / g };
}

function bigGcd(a: bigint, b: bigint): bigint {
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a;
}

const ZERO_F: Frac = { n: 0n, d: 1n };
const ONE_F: Frac = { n: 1n, d: 1n };

function fracAdd(a: Frac, b: Frac): Frac {
  return frac(a.n * b.d + b.n * a.d, a.d * b.d);
}
function fracSub(a: Frac, b: Frac): Frac {
  return frac(a.n * b.d - b.n * a.d, a.d * b.d);
}
function fracMul(a: Frac, b: Frac): Frac {
  return frac(a.n * b.n, a.d * b.d);
}
function fracDiv(a: Frac, b: Frac): Frac {
  return frac(a.n * b.d, a.d * b.n);
}
function fracEq(a: Frac, b: Frac): boolean {
  return a.n === b.n && a.d === b.d;
}
function fracNeg(a: Frac): Frac {
  return { n: -a.n, d: a.d };
}
function fracIsZero(a: Frac): boolean {
  return a.n === 0n;
}
function fracIsNeg(a: Frac): boolean {
  return a.n < 0n;
}

/** Convert a Frac to an IRNode (integer if d=1, rational otherwise). */
function fracToIR(f: Frac): IRNode {
  return f.d === 1n ? int(f.n) : rational(f.n, f.d);
}

/**
 * Return sqrt(f) as a Frac if f is a perfect rational square, else undefined.
 *
 * Examples:
 *   fracRationalSqrt({n:4n,d:1n})  → {n:2n,d:1n}
 *   fracRationalSqrt({n:1n,d:4n})  → {n:1n,d:2n}
 *   fracRationalSqrt({n:2n,d:1n})  → undefined
 */
function fracRationalSqrt(f: Frac): Frac | undefined {
  const sn = bigintSqrtExact(f.n);
  const sd = bigintSqrtExact(f.d);
  if (sn !== undefined && sd !== undefined) return frac(sn, sd);
  return undefined;
}

function bigintSqrtExact(n: bigint): bigint | undefined {
  if (n < 0n) return undefined;
  if (n === 0n) return 0n;
  const r = bigintSqrt(n);
  return r * r === n ? r : undefined;
}

// ─────────────────────────────────────────────────────────────────────────────
// Polynomial arithmetic over Frac
// ─────────────────────────────────────────────────────────────────────────────
//
// Polynomials are represented as `Frac[]` in ascending degree order:
//   poly[i]  is the coefficient of s^i.
//
// So (c0, c1, c2) represents  c0 + c1·s + c2·s².

type Poly = Frac[];

function polyNormalize(p: Poly): Poly {
  const r = [...p];
  while (r.length > 1 && fracIsZero(r[r.length - 1])) r.pop();
  return r.length === 0 ? [ZERO_F] : r;
}

function polyDegree(p: Poly): number {
  return polyNormalize(p).length - 1;
}

function polyIsZero(p: Poly): boolean {
  const n = polyNormalize(p);
  return n.length === 1 && fracIsZero(n[0]);
}

function polyAdd(a: Poly, b: Poly): Poly {
  const len = Math.max(a.length, b.length);
  const result: Poly = [];
  for (let i = 0; i < len; i++) {
    const ca = i < a.length ? a[i] : ZERO_F;
    const cb = i < b.length ? b[i] : ZERO_F;
    result.push(fracAdd(ca, cb));
  }
  return polyNormalize(result);
}

function polyNeg(p: Poly): Poly {
  return p.map(fracNeg);
}

function polyMul(a: Poly, b: Poly): Poly {
  const result: Poly = Array.from({ length: a.length + b.length - 1 }, () => ZERO_F);
  for (let i = 0; i < a.length; i++) {
    for (let j = 0; j < b.length; j++) {
      result[i + j] = fracAdd(result[i + j], fracMul(a[i], b[j]));
    }
  }
  return polyNormalize(result);
}

function polyScale(p: Poly, c: Frac): Poly {
  return p.map((x) => fracMul(x, c));
}

/** Raise polynomial p to the non-negative integer power n (binary exponentiation). */
function polyPow(p: Poly, n: bigint): Poly {
  if (n === 0n) return [ONE_F];
  if (n === 1n) return [...p];
  let result: Poly = [ONE_F];
  let base = [...p];
  let k = n;
  while (k > 0n) {
    if (k & 1n) result = polyMul(result, base);
    base = polyMul(base, base);
    k >>= 1n;
  }
  return result;
}

/** Evaluate polynomial at x using Horner's method. */
function polyEval(p: Poly, x: Frac): Frac {
  let result = ZERO_F;
  for (let i = p.length - 1; i >= 0; i--) {
    result = fracAdd(fracMul(result, x), p[i]);
  }
  return result;
}

/** Polynomial long division: returns [quotient, remainder]. */
function polyDivmod(num: Poly, den: Poly): [Poly, Poly] {
  let n = polyNormalize(num);
  const d = polyNormalize(den);
  const degN = polyDegree(n);
  const degD = polyDegree(d);
  if (degN < degD) return [[ZERO_F], [...n]];

  const q: Poly = Array.from({ length: degN - degD + 1 }, () => ZERO_F);
  const rem = [...n];

  for (let i = degN - degD; i >= 0; i--) {
    if (i + degD < rem.length && !fracIsZero(d[degD])) {
      const coeff = fracDiv(rem[i + degD], d[degD]);
      q[i] = coeff;
      for (let j = 0; j <= degD; j++) {
        if (i + j < rem.length) {
          rem[i + j] = fracSub(rem[i + j], fracMul(coeff, d[j]));
        }
      }
    }
  }
  return [polyNormalize(q), polyNormalize(rem)];
}

/**
 * Compute p(s + r) as a polynomial in s.
 *
 * Substitutes s → s + r via binomial expansion: each term c·s^i becomes
 * c·(s+r)^i.  This is used to shift a polynomial so a given root r is
 * moved to the origin, enabling the formal Laurent expansion for repeated
 * poles.
 */
function polyShift(p: Poly, r: Frac): Poly {
  let result: Poly = [ZERO_F];
  for (let i = 0; i < p.length; i++) {
    if (fracIsZero(p[i])) continue;
    let term: Poly;
    if (i === 0) {
      term = [p[i]];
    } else {
      // (s + r)^i in ascending-coefficient form
      const sPlusR: Poly = [r, ONE_F];
      term = polyScale(polyPow(sPlusR, BigInt(i)), p[i]);
    }
    result = polyAdd(result, term);
  }
  return polyNormalize(result);
}

/**
 * First `terms` Taylor coefficients of the formal power series N(t)/D(t).
 *
 * Requires D[0] ≠ 0.  Uses the recurrence:
 *
 *   g_0 = N[0] / D[0]
 *   g_k = (N[k] − Σ_{j=0}^{k-1} D[k-j]·g_j) / D[0]
 */
function powerSeriesCoeffs(num: Poly, den: Poly, terms: number): Frac[] {
  const q0 = den[0];
  const g: Frac[] = [];
  for (let k = 0; k < terms; k++) {
    const nk = k < num.length ? num[k] : ZERO_F;
    let subtract = ZERO_F;
    for (let j = 0; j < k; j++) {
      const dkj = k - j < den.length ? den[k - j] : ZERO_F;
      subtract = fracAdd(subtract, fracMul(dkj, g[j]));
    }
    g.push(fracDiv(fracSub(nk, subtract), q0));
  }
  return g;
}

/**
 * Compute Laurent coefficients [A_m, A_{m-1}, …, A_1] for a pole of
 * multiplicity m at r.
 *
 * For F(s) = N(s)/D(s) with D having a zero of order m at r, we:
 * 1. Shift: N_t(t) = N(r+t), D_t(t) = D(r+t).
 * 2. Verify D_t[0..m-1] are zero (confirms multiplicity ≥ m).
 * 3. Form Q_other = D_t[m:] (constant term nonzero).
 * 4. Compute m Taylor coefficients of N_t / Q_other.
 *
 * Returns undefined if the claimed multiplicity is inconsistent.
 */
function computeRepeatedResidues(num: Poly, den: Poly, r: Frac, m: number): Frac[] | undefined {
  const Nt = polyShift(num, r);
  const Dt = polyShift(den, r);

  for (let i = 0; i < m; i++) {
    const val = i < Dt.length ? Dt[i] : ZERO_F;
    if (!fracIsZero(val)) return undefined; // multiplicity claim is wrong
  }
  if (Dt.length <= m) return undefined; // degenerate
  const Qother = Dt.slice(m);
  if (fracIsZero(Qother[0])) return undefined; // higher multiplicity than claimed

  return powerSeriesCoeffs(Nt, Qother, m);
}

// ─────────────────────────────────────────────────────────────────────────────
// Rational root finding
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Find rational roots of p using the rational root theorem.
 *
 * Any rational root p/q of an integer polynomial must have p | a_0 and
 * q | a_n.  We clear denominators to reduce to the integer case.
 */
function rationalRoots(p: Poly): Frac[] {
  p = polyNormalize(p);
  if (p.length <= 1) return [];

  // s = 0 is a root iff the constant term is zero
  if (fracIsZero(p[0])) return [ZERO_F];

  // Clear denominators: multiply by LCM of all denominators
  let lcm = 1n;
  for (const c of p) {
    const g = bigGcd(lcm, c.d);
    lcm = (lcm / g) * c.d;
  }
  const intCoeffs = p.map((c) => c.n * (lcm / c.d));

  const constantTerm = intCoeffs[0] < 0n ? -intCoeffs[0] : intCoeffs[0];
  const leadCoeff =
    intCoeffs[intCoeffs.length - 1] < 0n
      ? -intCoeffs[intCoeffs.length - 1]
      : intCoeffs[intCoeffs.length - 1];

  const pDivs = divisorsBigint(constantTerm);
  const qDivs = divisorsBigint(leadCoeff);

  const roots: Frac[] = [];
  const seen = new Set<string>();

  for (const pv of pDivs) {
    for (const qv of qDivs) {
      for (const sign of [1n, -1n]) {
        const candidate = frac(sign * pv, qv);
        const key = `${candidate.n}/${candidate.d}`;
        if (seen.has(key)) continue;
        seen.add(key);
        if (fracIsZero(polyEval(p, candidate))) {
          roots.push(candidate);
        }
      }
    }
  }
  return roots;
}

/** Positive divisors of n (bigint). */
function divisorsBigint(n: bigint): bigint[] {
  if (n === 0n) return [0n];
  const divs: bigint[] = [];
  for (let i = 1n; i * i <= n; i++) {
    if (n % i === 0n) {
      divs.push(i);
      if (i !== n / i) divs.push(n / i);
    }
  }
  return divs;
}

/**
 * Extract all rational roots of p with multiplicity (each root repeated in
 * the returned list according to its multiplicity).
 *
 * Uses repeated division: find a root, divide it out, repeat.
 */
function extractAllRationalRoots(p: Poly): Frac[] {
  p = polyNormalize(p);
  const roots: Frac[] = [];

  while (polyDegree(p) >= 1) {
    const found = rationalRoots(p);
    if (found.length === 0) break;
    const root = found[0];
    // Extract all copies of this root
    let progress = false;
    while (polyDegree(p) >= 1 && fracIsZero(polyEval(p, root))) {
      roots.push(root);
      const [q] = polyDivmod(p, [fracNeg(root), ONE_F]);
      p = polyNormalize(q);
      progress = true;
    }
    if (!progress) break; // safety: root claim was wrong
  }
  return roots;
}

// ─────────────────────────────────────────────────────────────────────────────
// IR ↔ rational function conversion
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Convert an IR expression to a rational function (num/den pair of Poly).
 *
 * Returns undefined if the node is not a polynomial in s with rational
 * coefficients, or not a ratio of two such polynomials.
 *
 * Handles: integer, rational, symbol (s only), Add, Sub, Mul, Div, Neg, Pow.
 */
function irToRational(node: IRNode, s: IRNode): { num: Poly; den: Poly } | undefined {
  if (node.kind === "integer") {
    return { num: [frac(node.value)], den: [ONE_F] };
  }
  if (node.kind === "rational") {
    return { num: [frac(node.numer, node.denom)], den: [ONE_F] };
  }
  if (node.kind === "symbol") {
    // Only the integration variable s is recognized; any other symbol is
    // non-rational from this engine's perspective.
    if (equals(node, s)) return { num: [ZERO_F, ONE_F], den: [ONE_F] };
    return undefined;
  }
  if (node.kind !== "apply") return undefined;

  const h = node.head;
  const args = node.args;

  if (equals(h, ADD) && args.length === 2) {
    const r1 = irToRational(args[0], s);
    const r2 = irToRational(args[1], s);
    if (!r1 || !r2) return undefined;
    // (n1/d1) + (n2/d2) = (n1·d2 + n2·d1) / (d1·d2)
    return {
      num: polyNormalize(polyAdd(polyMul(r1.num, r2.den), polyMul(r2.num, r1.den))),
      den: polyNormalize(polyMul(r1.den, r2.den)),
    };
  }

  if (equals(h, SUB) && args.length === 2) {
    const r1 = irToRational(args[0], s);
    const r2 = irToRational(args[1], s);
    if (!r1 || !r2) return undefined;
    return {
      num: polyNormalize(polyAdd(polyMul(r1.num, r2.den), polyMul(polyNeg(r2.num), r1.den))),
      den: polyNormalize(polyMul(r1.den, r2.den)),
    };
  }

  if (equals(h, MUL) && args.length === 2) {
    const r1 = irToRational(args[0], s);
    const r2 = irToRational(args[1], s);
    if (!r1 || !r2) return undefined;
    return {
      num: polyNormalize(polyMul(r1.num, r2.num)),
      den: polyNormalize(polyMul(r1.den, r2.den)),
    };
  }

  if (equals(h, DIV) && args.length === 2) {
    const r1 = irToRational(args[0], s);
    const r2 = irToRational(args[1], s);
    if (!r1 || !r2) return undefined;
    // (n1/d1) / (n2/d2) = (n1·d2) / (d1·n2)
    return {
      num: polyNormalize(polyMul(r1.num, r2.den)),
      den: polyNormalize(polyMul(r1.den, r2.num)),
    };
  }

  if (equals(h, NEG) && args.length === 1) {
    const r1 = irToRational(args[0], s);
    if (!r1) return undefined;
    return { num: polyNormalize(polyNeg(r1.num)), den: r1.den };
  }

  if (equals(h, POW) && args.length === 2) {
    const expNode = args[1];
    if (expNode.kind !== "integer") return undefined;
    const nExp = expNode.value;
    const r1 = irToRational(args[0], s);
    if (!r1) return undefined;
    if (nExp < 0n) {
      // base^{-n} = 1 / base^n  →  num = den^n, den = num^n
      const posN = -nExp;
      return {
        num: polyNormalize(polyPow(r1.den, posN)),
        den: polyNormalize(polyPow(r1.num, posN)),
      };
    }
    return {
      num: polyNormalize(polyPow(r1.num, nExp)),
      den: polyNormalize(polyPow(r1.den, nExp)),
    };
  }

  return undefined;
}

// ─────────────────────────────────────────────────────────────────────────────
// Inverse transform builders (pole → time-domain term)
// ─────────────────────────────────────────────────────────────────────────────

/** L⁻¹{A/(s−a)}: returns A·exp(at).  If a=0, returns A·UnitStep(t). */
function iltSimplePole(A: Frac, a: Frac, t: IRNode): IRNode {
  if (fracIsZero(a)) {
    const step = app(UNIT_STEP, [t]);
    if (fracEq(A, ONE_F)) return step;
    return bin(MUL, fracToIR(A), step);
  }
  const expTerm = fracEq(a, ONE_F) ? app(EXP, [t]) : app(EXP, [bin(MUL, fracToIR(a), t)]);
  if (fracEq(A, ONE_F)) return expTerm;
  if (fracEq(A, frac(-1n))) return app(NEG, [expTerm]);
  return bin(MUL, fracToIR(A), expTerm);
}

/** L⁻¹{A/(s−a)^n}  (n ≥ 2):  A·t^{n-1}·exp(at) / (n-1)! */
function iltRepeatedPole(A: Frac, a: Frac, n: number, t: IRNode): IRNode {
  const factNm1 = factorial(BigInt(n - 1));
  const coeff = fracDiv(A, frac(factNm1));
  const coeffNode = fracToIR(coeff);

  // t^{n-1}  (n-1=1 → just t)
  const tPow: IRNode = n - 1 === 1 ? t : bin(POW, t, int(n - 1));

  if (fracIsZero(a)) {
    // exp(0) = 1; only the t^{n-1} factor remains
    return fracEq(coeff, ONE_F) ? tPow : bin(MUL, coeffNode, tPow);
  }

  const expTerm = fracEq(a, ONE_F) ? app(EXP, [t]) : app(EXP, [bin(MUL, fracToIR(a), t)]);
  const inner = bin(MUL, tPow, expTerm);
  return fracEq(coeff, ONE_F) ? inner : bin(MUL, coeffNode, inner);
}

/**
 * Invert (A·s + B) / (s² + b·s + c) with discriminant b²−4c < 0.
 *
 * Completes the square: s² + bs + c = (s + α)² + β²
 * where α = b/2 and β = √(c − α²).
 *
 * Decomposition:
 *
 *   (A·s + B) / ((s+α)² + β²)
 *   = A·(s+α)/((s+α)²+β²)  +  (B−Aα)/β · β/((s+α)²+β²)
 *
 * Inverse:
 *
 *   A·exp(−αt)·cos(βt)  +  (B−Aα)/β · exp(−αt)·sin(βt)
 *
 * When β is irrational, a Sqrt(β²) IR node is built to keep the result
 * exact.
 *
 * Returns a list of terms (may be empty if all coefficients are zero),
 * or undefined if the quadratic is not irreducible.
 */
function iltIrreducibleQuad(linNum: Poly, quadDen: Poly, t: IRNode): IRNode[] | undefined {
  if (polyDegree(quadDen) !== 2) return undefined;

  const leading = quadDen.length >= 3 ? quadDen[2] : ZERO_F;
  if (fracIsZero(leading)) return undefined;

  // Make monic: divide num and den by leading coefficient
  const invLeading = fracDiv(ONE_F, leading);
  const c = fracMul(quadDen[0] !== undefined ? quadDen[0] : ZERO_F, invLeading);
  const b = fracMul(quadDen.length >= 2 ? quadDen[1] : ZERO_F, invLeading);
  const B = fracMul(linNum.length >= 1 ? linNum[0] : ZERO_F, invLeading);
  const A = fracMul(linNum.length >= 2 ? linNum[1] : ZERO_F, invLeading);

  // Discriminant check: b² − 4c < 0 means complex conjugate poles
  const disc = fracSub(fracMul(b, b), fracMul(frac(4n), c));
  if (!fracIsNeg(disc)) return undefined; // real roots — not irreducible

  // Complete the square: α = b/2, β² = c − α²
  const alpha = fracDiv(b, frac(2n));
  const betaSq = fracSub(c, fracMul(alpha, alpha));
  if (!fracIsNeg(fracNeg(betaSq)) && fracIsZero(betaSq)) return undefined; // degenerate
  if (fracIsNeg(betaSq)) return undefined;

  const betaRat = fracRationalSqrt(betaSq);
  const betaIR: IRNode =
    betaRat !== undefined ? fracToIR(betaRat) : app(SQRT, [fracToIR(betaSq)]);
  const negAlpha = fracNeg(alpha);

  const betaIsOne = betaRat !== undefined && fracEq(betaRat, ONE_F);
  const alphaIsZero = fracIsZero(alpha);

  /** Build coeff · exp(−α·t) · trig(β·t). */
  function makeExpTrig(coeff: IRNode, isCos: boolean): IRNode {
    // Trig argument: β·t (simplified to t when β=1)
    const trigArg: IRNode = betaIsOne ? t : bin(MUL, betaIR, t);
    const trigFn = isCos ? COS : SIN;
    const trigTerm = app(trigFn, [trigArg]);

    // Exponential factor exp(−α·t) — omitted when α=0
    let oscillator: IRNode;
    if (alphaIsZero) {
      oscillator = trigTerm;
    } else {
      const expArg: IRNode = fracEq(negAlpha, ONE_F)
        ? t
        : fracEq(negAlpha, frac(-1n))
        ? app(NEG, [t])
        : bin(MUL, fracToIR(negAlpha), t);
      oscillator = bin(MUL, app(EXP, [expArg]), trigTerm);
    }

    // Scale by coefficient (skip mul-by-1)
    return isOne(coeff) ? oscillator : bin(MUL, coeff, oscillator);
  }

  const terms: IRNode[] = [];

  // Term 1: A · exp(−αt) · cos(βt)
  if (!fracIsZero(A)) {
    terms.push(makeExpTrig(fracToIR(A), true));
  }

  // Term 2: (B − A·α) / β · exp(−αt) · sin(βt)
  const baa = fracSub(B, fracMul(A, alpha)); // B − A·α
  if (!fracIsZero(baa)) {
    const coeff2: IRNode =
      betaRat !== undefined
        ? fracToIR(fracDiv(baa, betaRat))
        : bin(DIV, fracToIR(baa), betaIR);
    terms.push(makeExpTrig(coeff2, false));
  }

  return terms;
}

// ─────────────────────────────────────────────────────────────────────────────
// Full partial-fraction decomposition engine
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Attempt inverse Laplace transform via partial-fraction decomposition.
 *
 * Pipeline:
 *
 * 1. Convert F(s) to a rational function (num, den) over Frac.
 * 2. Polynomial long division for improper fractions.
 * 3. Extract all rational roots of den with multiplicity.
 * 4. Build residues for each pole (simple or repeated via power series).
 * 5. Handle any irreducible quadratic factor.
 * 6. Assemble terms into an ADD chain.
 *
 * Returns undefined if decomposition is not possible (e.g., irreducible
 * cubic or non-rational coefficients).
 */
function inversePF(F: IRNode, s: IRNode, t: IRNode): IRNode | undefined {
  const rf = irToRational(F, s);
  if (!rf) return undefined;

  let { num, den } = rf;
  num = polyNormalize(num);
  den = polyNormalize(den);

  // ── Step 1: Improper fractions via polynomial long division ─────────────
  // If deg(N) ≥ deg(D), extract the polynomial quotient P(s).
  // L⁻¹{P(s)} = P[0]·δ(t) for deg-0 quotient; higher degrees return None.
  let polyPart: Poly = [ZERO_F];
  if (polyDegree(num) >= polyDegree(den)) {
    const [q, r] = polyDivmod(num, den);
    polyPart = polyNormalize(q);
    num = polyNormalize(r);
  }

  // ── Step 2: Extract rational roots and factor denominator ────────────────
  const roots = extractAllRationalRoots(den);

  // Q_rat = Π(s − r_i) for each rational root r_i
  let Q_rat: Poly = [ONE_F];
  for (const r of roots) {
    Q_rat = polyMul(Q_rat, [fracNeg(r), ONE_F]);
  }
  Q_rat = polyNormalize(Q_rat);

  // Q_irred = den / Q_rat  (the irreducible remainder)
  const [Q_irred, remCheck] = polyDivmod(den, Q_rat);
  if (!polyIsZero(remCheck)) return undefined; // should be exact

  const irredDeg = polyDegree(Q_irred);
  if (irredDeg > 2) return undefined; // can't handle irreducible cubic+

  // ── Step 3: Polynomial-part terms ────────────────────────────────────────
  const irTerms: IRNode[] = [];

  for (let deg = 0; deg < polyPart.length; deg++) {
    const coeff = polyPart[deg];
    if (fracIsZero(coeff)) continue;
    if (deg === 0) {
      const delta = app(DIRAC_DELTA, [t]);
      irTerms.push(fracEq(coeff, ONE_F) ? delta : bin(MUL, fracToIR(coeff), delta));
    } else {
      return undefined; // DiracDelta derivatives not supported
    }
  }

  // ── Step 4: Rational pole terms ──────────────────────────────────────────
  // Group roots by distinct value (accumulating multiplicity count)
  const distinctRoots = new Map<string, { root: Frac; count: number }>();
  for (const r of roots) {
    const key = `${r.n}/${r.d}`;
    const existing = distinctRoots.get(key);
    if (existing) existing.count++;
    else distinctRoots.set(key, { root: r, count: 1 });
  }

  // residuesMap[key] = [A_m, A_{m-1}, …, A_1] for that root
  const residuesMap = new Map<string, Frac[]>();

  for (const [key, { root: r, count: m }] of distinctRoots) {
    const linearPowM = polyPow([fracNeg(r), ONE_F], BigInt(m));
    const [Q_rat_no_rm] = polyDivmod(Q_rat, linearPowM);
    // Q_other = (Q_rat / (s−r)^m) · Q_irred — everything except the (s−r)^m factor
    const Q_other = polyNormalize(polyMul(Q_rat_no_rm, Q_irred));

    if (m === 1) {
      // Simple pole: residue = N(r) / Q_other(r)
      const q_other_r = polyEval(Q_other, r);
      if (fracIsZero(q_other_r)) return undefined;
      const resA = fracDiv(polyEval(num, r), q_other_r);
      residuesMap.set(key, [resA]);
      irTerms.push(iltSimplePole(resA, r, t));
    } else {
      // Repeated pole: Laurent expansion via power series
      const residues = computeRepeatedResidues(num, den, r, m);
      if (!residues) return undefined;
      residuesMap.set(key, residues);
      for (let k = 0; k < residues.length; k++) {
        const resA = residues[k];
        if (fracIsZero(resA)) continue;
        const poleOrder = m - k; // m, m−1, …, 1
        irTerms.push(
          poleOrder === 1
            ? iltSimplePole(resA, r, t)
            : iltRepeatedPole(resA, r, poleOrder, t),
        );
      }
    }
  }

  // ── Step 5: Irreducible quadratic term ───────────────────────────────────
  if (irredDeg === 2) {
    // Recover the linear numerator (A·s + B) for the term (A·s+B)/Q_irred.
    //
    // The partial-fraction identity (multiplied by D = Q_rat·Q_irred) is:
    //
    //   N(s) = [rational contributions] + (A·s+B)·Q_rat(s)
    //
    // where each rational contribution for root r with multiplicity m at
    // pole order k is: A_k · Q_irred · Q_rat/(s−r)^k.
    //
    // Sum rational contributions into rat_poly, then:
    //   A·s+B = [N − rat_poly] / Q_rat  (exact division)

    let rat_poly: Poly = [ZERO_F];
    for (const [key, { root: r, count: m }] of distinctRoots) {
      const residues = residuesMap.get(key)!;
      for (let kIdx = 0; kIdx < residues.length; kIdx++) {
        const resA = residues[kIdx];
        if (fracIsZero(resA)) continue;
        const poleOrder = m - kIdx;
        const linearPowK = polyPow([fracNeg(r), ONE_F], BigInt(poleOrder));
        const [Q_rat_div_k] = polyDivmod(Q_rat, linearPowK);
        const contrib = polyScale(polyMul(Q_irred, Q_rat_div_k), resA);
        rat_poly = polyAdd(rat_poly, contrib);
      }
    }

    const irred_times_qrat = polyNormalize(polyAdd(num, polyNeg(rat_poly)));
    const [linNum, rem2] = polyDivmod(irred_times_qrat, Q_rat);
    if (!polyIsZero(rem2)) return undefined; // numerator recovery failed

    const irredTerms = iltIrreducibleQuad(polyNormalize(linNum), polyNormalize(Q_irred), t);
    if (!irredTerms) return undefined;
    irTerms.push(...irredTerms);
  }

  // ── Step 6: Assemble result ───────────────────────────────────────────────
  if (irTerms.length === 0) return int(0);
  if (irTerms.length === 1) return irTerms[0];
  let result = irTerms[0];
  for (let i = 1; i < irTerms.length; i++) {
    result = bin(ADD, result, irTerms[i]);
  }
  return result;
}

// ─────────────────────────────────────────────────────────────────────────────
// Pattern matchers for the forward table
// ─────────────────────────────────────────────────────────────────────────────

/** Match t^n · trig(ω·t) for n ≥ 2.  Used for forward table entries. */
function matchTnTimesTrig(
  f: IRNode,
  t: IRNode,
): { power: bigint; trigHead: IRNode; omega: IRNode } | undefined {
  const args = binaryArgs(f, MUL);
  if (!args) return undefined;
  for (const [left, right] of [args, [args[1], args[0]]] as const) {
    const n = matchPowerOfT(left, t);
    if (n === undefined || n < 2n) continue;
    const sinOmega = matchUnaryLinear(right, SIN, t);
    if (sinOmega !== undefined) return { power: n, trigHead: SIN, omega: sinOmega };
    const cosOmega = matchUnaryLinear(right, COS, t);
    if (cosOmega !== undefined) return { power: n, trigHead: COS, omega: cosOmega };
  }
  return undefined;
}

function matchExpTimesTrig(f: IRNode, t: IRNode): { shift: IRNode; trigHead: IRNode; omega: IRNode } | undefined {
  const args = binaryArgs(f, MUL);
  if (args === undefined) return undefined;
  for (const [expNode, trigNode] of [args, [args[1], args[0]]] as const) {
    const shift = matchUnaryLinear(expNode, EXP, t);
    if (shift === undefined) continue;
    const sinOmega = matchUnaryLinear(trigNode, SIN, t);
    if (sinOmega !== undefined) return { shift, trigHead: SIN, omega: sinOmega };
    const cosOmega = matchUnaryLinear(trigNode, COS, t);
    if (cosOmega !== undefined) return { shift, trigHead: COS, omega: cosOmega };
  }
  return undefined;
}

function matchTPowerTimesExp(f: IRNode, t: IRNode): { power: bigint; shift: IRNode } | undefined {
  const args = binaryArgs(f, MUL);
  if (args === undefined) return undefined;
  for (const [powerNode, expNode] of [args, [args[1], args[0]]] as const) {
    const power = matchPowerOfT(powerNode, t);
    const shift = matchUnaryLinear(expNode, EXP, t);
    if (power !== undefined && shift !== undefined) return { power, shift };
  }
  return undefined;
}

function matchTTimesTrig(f: IRNode, t: IRNode): { trigHead: IRNode; omega: IRNode } | undefined {
  const args = binaryArgs(f, MUL);
  if (args === undefined) return undefined;
  for (const [left, right] of [args, [args[1], args[0]]] as const) {
    if (!equals(left, t)) continue;
    const sinOmega = matchUnaryLinear(right, SIN, t);
    if (sinOmega !== undefined) return { trigHead: SIN, omega: sinOmega };
    const cosOmega = matchUnaryLinear(right, COS, t);
    if (cosOmega !== undefined) return { trigHead: COS, omega: cosOmega };
  }
  return undefined;
}

function matchPowerOfT(f: IRNode, t: IRNode): bigint | undefined {
  if (equals(f, t)) return 1n;
  const pow = binaryArgs(f, POW);
  if (pow !== undefined && equals(pow[0], t) && pow[1].kind === "integer" && pow[1].value >= 1n)
    return pow[1].value;
  return undefined;
}

function matchUnaryLinear(f: IRNode, head: IRNode, t: IRNode): IRNode | undefined {
  const args = applyArgs(f, head);
  return args !== undefined && args.length === 1 ? extractLinearArg(args[0], t) : undefined;
}

function extractLinearArg(arg: IRNode, t: IRNode): IRNode | undefined {
  if (equals(arg, t)) return int(1);
  const mul = binaryArgs(arg, MUL);
  if (mul !== undefined) {
    if (equals(mul[0], t) && isConstant(mul[1], t)) return mul[1];
    if (equals(mul[1], t) && isConstant(mul[0], t)) return mul[0];
  }
  const neg = applyArgs(arg, NEG);
  if (neg !== undefined && neg.length === 1) {
    const inner = extractLinearArg(neg[0], t);
    if (inner !== undefined) return negate(inner);
  }
  return undefined;
}

function extractCoeffAndFn(node: IRNode, t: IRNode): { coeff: IRNode; body: IRNode } | undefined {
  const args = binaryArgs(node, MUL);
  if (args === undefined) return undefined;
  if (isConstant(args[0], t)) return { coeff: args[0], body: args[1] };
  if (isConstant(args[1], t)) return { coeff: args[1], body: args[0] };
  return undefined;
}

function isConstant(node: IRNode, variable: IRNode): boolean {
  if (equals(node, variable)) return false;
  return node.kind !== "apply" || node.args.every((arg) => isConstant(arg, variable));
}

function matchSMinusA(node: IRNode, s: IRNode): IRNode | undefined {
  const args = binaryArgs(node, SUB);
  return args !== undefined && equals(args[0], s) ? args[1] : undefined;
}

function matchPowOf(node: IRNode, base: IRNode): bigint | undefined {
  const args = binaryArgs(node, POW);
  return args !== undefined && equals(args[0], base) && args[1].kind === "integer"
    ? args[1].value
    : undefined;
}

function matchSSqPlusParamSq(node: IRNode, s: IRNode): IRNode | undefined {
  const args = binaryArgs(node, ADD);
  if (args === undefined) return undefined;
  return matchSSqParamSq(args[0], args[1], s) ?? matchSSqParamSq(args[1], args[0], s);
}

function matchSSqMinusParamSq(node: IRNode, s: IRNode): IRNode | undefined {
  const args = binaryArgs(node, SUB);
  return args === undefined ? undefined : matchSSqParamSq(args[0], args[1], s);
}

function matchSSqParamSq(sSq: IRNode, paramSq: IRNode, s: IRNode): IRNode | undefined {
  return matchPowOf(sSq, s) === 2n ? sqrtParam(paramSq) : undefined;
}

function sqrtParam(node: IRNode): IRNode | undefined {
  const pow = binaryArgs(node, POW);
  if (pow !== undefined && isInt(pow[1], 2n)) return pow[0];
  if (node.kind === "integer" && node.value >= 0n) {
    const root = bigintSqrt(node.value);
    if (root * root === node.value) return int(root);
  }
  return undefined;
}

function isApplyOfVar(node: IRNode, head: IRNode, variable: IRNode): boolean {
  const args = applyArgs(node, head);
  return args !== undefined && args.length === 1 && equals(args[0], variable);
}

function sumSSqParamSq(s: IRNode, param: IRNode): IRNode {
  return bin(ADD, bin(POW, s, int(2)), bin(POW, param, int(2)));
}

function subSSqParamSq(s: IRNode, param: IRNode): IRNode {
  return bin(SUB, bin(POW, s, int(2)), bin(POW, param, int(2)));
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility helpers
// ─────────────────────────────────────────────────────────────────────────────

function applyArgs(node: IRNode, head: IRNode): readonly IRNode[] | undefined {
  return node.kind === "apply" && equals(node.head, head) ? node.args : undefined;
}

function binaryArgs(node: IRNode, head: IRNode): readonly [IRNode, IRNode] | undefined {
  const args = applyArgs(node, head);
  return args !== undefined && args.length === 2 ? [args[0], args[1]] : undefined;
}

function bin(head: IRNode, a: IRNode, b: IRNode): IRNode {
  return app(head, [a, b]);
}

function negate(node: IRNode): IRNode {
  return node.kind === "integer" ? int(-node.value) : app(NEG, [node]);
}

function isInt(node: IRNode, value: bigint): boolean {
  return node.kind === "integer" && node.value === value;
}

function isOne(node: IRNode): boolean {
  return isInt(node, 1n) || (node.kind === "rational" && node.numer === node.denom);
}

function factorial(n: bigint): bigint {
  let out = 1n;
  for (let i = 2n; i <= n; i += 1n) out *= i;
  return out;
}

function bigintSqrt(n: bigint): bigint {
  if (n < 2n) return n;
  let lo = 1n;
  let hi = n;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1n;
    const sq = mid * mid;
    if (sq === n) return mid;
    if (sq < n) lo = mid + 1n;
    else hi = mid - 1n;
  }
  return hi;
}
