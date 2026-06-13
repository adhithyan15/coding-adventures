/**
 * Bivariate Hensel lifting over ℚ[x, y].
 *
 * Ports `cas_factor.hensel` (Python) to TypeScript.  Algorithm:
 *
 * 1. Substitute ``y = y₀`` for a small integer ``y₀`` (we try
 *    0, 1, -1, 2, -2, …).  Require the univariate image ``f(x, y₀)`` to
 *    be squarefree with full x-degree (a *lucky* substitution).
 * 2. Factor the univariate image over ℚ via
 *    {@link factorIntegerPolynomial} after clearing denominators.
 * 3. Lift the factors back to ℚ[x, y] via Hensel's lemma — at each
 *    y-layer, solve a univariate diophantine ``u·g₀ + v·h₀ = e_k`` and
 *    add ``v·y^k`` to one factor and ``u·y^k`` to the other.
 * 4. After ``deg_y(f) + 1`` iterations the lift is exact; verify
 *    ``g · h == f`` and return ``[g, h]``.
 *
 * Multi-factor inputs (univariate image splits into r ≥ 2 pieces) are
 * handled by iterated two-factor lift: peel one factor against the
 * product of the rest, recurse.
 *
 * Returns ``null`` when the input is degenerate (single variable, zero,
 * etc.), when no lucky ``y₀`` exists in the search range, when the
 * univariate image is irreducible, or when final-product verification
 * fails.
 *
 * See ``code/packages/python/cas-factor/src/cas_factor/hensel.py`` for
 * the complete mathematical exposition.
 */

import { factorIntegerPolynomial } from "./index.js";

/** Sparse bivariate polynomial: key ``"i,j"`` ↦ coefficient of ``x^i·y^j``. */
export type BiPoly = Map<string, Rational>;

/** Univariate polynomial over ℚ in ascending-degree order. */
type UniQPoly = Rational[];

/** Bound on the ``|y₀|`` we try as a Hensel substitution point. */
const MAX_Y0_SEARCH = 8;

// ---------------------------------------------------------------------------
// Rational number type (self-contained — cas-factor's index.ts ``Rational``
// is private to that module).  Uses bigint for arbitrary precision so the
// lift can never overflow on coefficient growth.
// ---------------------------------------------------------------------------

/** Exact rational in lowest terms; ``denom > 0``. */
export class Rational {
  static readonly ZERO = new Rational(0n, 1n);
  static readonly ONE = new Rational(1n, 1n);

  readonly numer: bigint;
  readonly denom: bigint;

  constructor(numer: bigint, denom: bigint) {
    if (denom === 0n) throw new RangeError("Rational denominator cannot be zero");
    if (numer === 0n) {
      this.numer = 0n;
      this.denom = 1n;
      return;
    }
    if (denom < 0n) {
      numer = -numer;
      denom = -denom;
    }
    const g = bgcd(numer < 0n ? -numer : numer, denom);
    this.numer = numer / g;
    this.denom = denom / g;
  }

  static fromInt(value: bigint | number): Rational {
    const b = typeof value === "bigint" ? value : BigInt(value);
    return new Rational(b, 1n);
  }

  add(other: Rational): Rational {
    return new Rational(this.numer * other.denom + other.numer * this.denom, this.denom * other.denom);
  }

  sub(other: Rational): Rational {
    return new Rational(this.numer * other.denom - other.numer * this.denom, this.denom * other.denom);
  }

  mul(other: Rational): Rational {
    return new Rational(this.numer * other.numer, this.denom * other.denom);
  }

  div(other: Rational): Rational {
    if (other.numer === 0n) throw new RangeError("Rational division by zero");
    return new Rational(this.numer * other.denom, this.denom * other.numer);
  }

  neg(): Rational {
    return new Rational(-this.numer, this.denom);
  }

  isZero(): boolean {
    return this.numer === 0n;
  }

  equals(other: Rational): boolean {
    return this.numer === other.numer && this.denom === other.denom;
  }

  pow(n: number): Rational {
    if (n < 0) throw new RangeError("Rational.pow requires non-negative exponent");
    let result = Rational.ONE;
    let base: Rational = this;
    let exp = n;
    while (exp > 0) {
      if ((exp & 1) === 1) result = result.mul(base);
      base = base.mul(base);
      exp >>>= 1;
    }
    return result;
  }
}

function bgcd(a: bigint, b: bigint): bigint {
  a = a < 0n ? -a : a;
  b = b < 0n ? -b : b;
  while (b !== 0n) {
    const t = b;
    b = a % b;
    a = t;
  }
  return a === 0n ? 1n : a;
}

function blcm(a: bigint, b: bigint): bigint {
  if (a === 0n || b === 0n) return 0n;
  return (a / bgcd(a, b)) * b;
}

function babs(a: bigint): bigint {
  return a < 0n ? -a : a;
}

// ---------------------------------------------------------------------------
// BiPoly key encoding.  We use a string ``"i,j"`` instead of a tuple so the
// Map's structural-equality semantics work in JS.
// ---------------------------------------------------------------------------

function key(i: number, j: number): string {
  return `${i},${j}`;
}

function parseKey(k: string): [number, number] {
  const idx = k.indexOf(",");
  return [Number(k.slice(0, idx)), Number(k.slice(idx + 1))];
}

// ---------------------------------------------------------------------------
// Univariate Q[x] helpers.
// ---------------------------------------------------------------------------

function uNormalize(p: UniQPoly): UniQPoly {
  const out = [...p];
  while (out.length > 0 && out[out.length - 1].isZero()) out.pop();
  return out;
}

function uDegree(p: UniQPoly): number {
  const n = uNormalize(p);
  return n.length - 1;
}

function uAdd(a: UniQPoly, b: UniQPoly): UniQPoly {
  const n = Math.max(a.length, b.length);
  const out: Rational[] = Array.from({ length: n }, () => Rational.ZERO);
  for (let i = 0; i < a.length; i += 1) out[i] = out[i].add(a[i]);
  for (let i = 0; i < b.length; i += 1) out[i] = out[i].add(b[i]);
  return uNormalize(out);
}

function uSub(a: UniQPoly, b: UniQPoly): UniQPoly {
  const n = Math.max(a.length, b.length);
  const out: Rational[] = Array.from({ length: n }, () => Rational.ZERO);
  for (let i = 0; i < a.length; i += 1) out[i] = out[i].add(a[i]);
  for (let i = 0; i < b.length; i += 1) out[i] = out[i].sub(b[i]);
  return uNormalize(out);
}

function uMul(a: UniQPoly, b: UniQPoly): UniQPoly {
  if (a.length === 0 || b.length === 0) return [];
  const out: Rational[] = Array.from({ length: a.length + b.length - 1 }, () => Rational.ZERO);
  for (let i = 0; i < a.length; i += 1) {
    if (a[i].isZero()) continue;
    for (let j = 0; j < b.length; j += 1) {
      out[i + j] = out[i + j].add(a[i].mul(b[j]));
    }
  }
  return uNormalize(out);
}

function uScale(a: UniQPoly, s: Rational): UniQPoly {
  if (s.isZero()) return [];
  return uNormalize(a.map((c) => c.mul(s)));
}

/** Polynomial division ``a = q · b + r`` in ℚ[x]; returns ``[q, r]``. */
function uDivmod(a: UniQPoly, b: UniQPoly): [UniQPoly, UniQPoly] {
  const na = uNormalize(a);
  const nb = uNormalize(b);
  if (nb.length === 0) throw new RangeError("division by zero polynomial");
  const db = nb.length - 1;
  const lcB = nb[nb.length - 1];
  const qRev: Rational[] = [];
  const rem = [...na];
  while (rem.length - 1 >= db && rem.length > 0) {
    const shift = rem.length - 1 - db;
    const c = rem[rem.length - 1].div(lcB);
    qRev.push(c);
    for (let k = 0; k < nb.length; k += 1) {
      rem[shift + k] = rem[shift + k].sub(c.mul(nb[k]));
    }
    while (rem.length > 0 && rem[rem.length - 1].isZero()) rem.pop();
  }
  return [uNormalize(qRev.reverse()), uNormalize(rem)];
}

/** Extended Euclidean: returns ``[g, s, t]`` with ``s·a + t·b = g``, ``g`` monic. */
function uGcdExt(a: UniQPoly, b: UniQPoly): [UniQPoly, UniQPoly, UniQPoly] {
  let oldR = uNormalize(a);
  let r = uNormalize(b);
  let oldS: UniQPoly = [Rational.ONE];
  let s: UniQPoly = [];
  let oldT: UniQPoly = [];
  let t: UniQPoly = [Rational.ONE];

  while (r.length > 0) {
    const [q] = uDivmod(oldR, r);
    [oldR, r] = [r, uSub(oldR, uMul(q, r))];
    [oldS, s] = [s, uSub(oldS, uMul(q, s))];
    [oldT, t] = [t, uSub(oldT, uMul(q, t))];
  }

  let g = oldR;
  if (g.length > 0 && !g[g.length - 1].equals(Rational.ONE)) {
    const inv = Rational.ONE.div(g[g.length - 1]);
    g = uScale(g, inv);
    oldS = uScale(oldS, inv);
    oldT = uScale(oldT, inv);
  }
  return [g, oldS, oldT];
}

/** Solve ``u · g₀ + v · h₀ = c`` with deg u < deg h₀, deg v < deg g₀. */
function uDiophantine(g0: UniQPoly, h0: UniQPoly, c: UniQPoly): [UniQPoly, UniQPoly] | null {
  let [g, s, t] = uGcdExt(g0, h0);
  if (uDegree(g) !== 0) return null;
  const inv = Rational.ONE.div(g[0]);
  s = uScale(s, inv);
  t = uScale(t, inv);
  const sc = uMul(s, c);
  const [q, u] = uDivmod(sc, h0);
  const tc = uMul(t, c);
  const vRaw = uAdd(tc, uMul(q, g0));
  const [, v] = uDivmod(vRaw, g0);
  return [u, v];
}

// ---------------------------------------------------------------------------
// Bivariate polynomial helpers.
// ---------------------------------------------------------------------------

function biNormalize(p: BiPoly): BiPoly {
  const out: BiPoly = new Map();
  for (const [k, v] of p) {
    if (!v.isZero()) out.set(k, v);
  }
  return out;
}

function biDegreeX(p: BiPoly): number {
  let best = -1;
  for (const [k, v] of p) {
    if (v.isZero()) continue;
    const [i] = parseKey(k);
    if (i > best) best = i;
  }
  return best;
}

function biDegreeY(p: BiPoly): number {
  let best = -1;
  for (const [k, v] of p) {
    if (v.isZero()) continue;
    const [, j] = parseKey(k);
    if (j > best) best = j;
  }
  return best;
}

function biAdd(a: BiPoly, b: BiPoly): BiPoly {
  const out: BiPoly = new Map(a);
  for (const [k, v] of b) {
    const cur = out.get(k) ?? Rational.ZERO;
    out.set(k, cur.add(v));
  }
  return biNormalize(out);
}

function biSub(a: BiPoly, b: BiPoly): BiPoly {
  const out: BiPoly = new Map(a);
  for (const [k, v] of b) {
    const cur = out.get(k) ?? Rational.ZERO;
    out.set(k, cur.sub(v));
  }
  return biNormalize(out);
}

function biMul(a: BiPoly, b: BiPoly): BiPoly {
  const out: BiPoly = new Map();
  for (const [k1, c1] of a) {
    if (c1.isZero()) continue;
    const [i1, j1] = parseKey(k1);
    for (const [k2, c2] of b) {
      if (c2.isZero()) continue;
      const [i2, j2] = parseKey(k2);
      const k = key(i1 + i2, j1 + j2);
      const cur = out.get(k) ?? Rational.ZERO;
      out.set(k, cur.add(c1.mul(c2)));
    }
  }
  return biNormalize(out);
}

function biEquals(a: BiPoly, b: BiPoly): boolean {
  const na = biNormalize(a);
  const nb = biNormalize(b);
  if (na.size !== nb.size) return false;
  for (const [k, v] of na) {
    const w = nb.get(k);
    if (w === undefined || !w.equals(v)) return false;
  }
  return true;
}

/** Substitute ``y = y₀`` and return the univariate-in-x image. */
function biSubstituteY(p: BiPoly, y0: Rational): UniQPoly {
  const dx = biDegreeX(p);
  if (dx < 0) return [];
  const out: Rational[] = Array.from({ length: dx + 1 }, () => Rational.ZERO);
  for (const [k, c] of p) {
    const [i, j] = parseKey(k);
    out[i] = out[i].add(c.mul(y0.pow(j)));
  }
  return uNormalize(out);
}

/** Embed a univariate-in-x polynomial as a bivariate polynomial. */
function biUniX(p: UniQPoly): BiPoly {
  const out: BiPoly = new Map();
  for (let i = 0; i < p.length; i += 1) {
    if (!p[i].isZero()) out.set(key(i, 0), p[i]);
  }
  return out;
}

/** Extract the univariate-in-x coefficient of ``y^k``. */
function biCoeffAtYPower(p: BiPoly, kPow: number): UniQPoly {
  let dx = -1;
  for (const [k, c] of p) {
    if (c.isZero()) continue;
    const [i, j] = parseKey(k);
    if (j === kPow && i > dx) dx = i;
  }
  if (dx < 0) return [];
  const out: Rational[] = Array.from({ length: dx + 1 }, () => Rational.ZERO);
  for (const [k, c] of p) {
    const [i, j] = parseKey(k);
    if (j === kPow) out[i] = out[i].add(c);
  }
  return uNormalize(out);
}

/** Rewrite ``p`` as a polynomial in ``(y − y₀)`` instead of ``y``. */
function biShiftY(p: BiPoly, y0: Rational): BiPoly {
  if (y0.isZero()) return new Map(p);
  const out: BiPoly = new Map();
  for (const [k, c] of p) {
    if (c.isZero()) continue;
    const [i, j] = parseKey(k);
    // (y - y0)^? expansion: y^j = ∑ C(j,m) (y-y0)^m y0^(j-m)
    for (let m = 0; m <= j; m += 1) {
      const coeff = c.mul(Rational.fromInt(binomial(j, m))).mul(y0.pow(j - m));
      const kk = key(i, m);
      const cur = out.get(kk) ?? Rational.ZERO;
      out.set(kk, cur.add(coeff));
    }
  }
  return biNormalize(out);
}

function binomial(n: number, k: number): bigint {
  if (k < 0 || k > n) return 0n;
  if (k === 0 || k === n) return 1n;
  const kk = Math.min(k, n - k);
  let num = 1n;
  let den = 1n;
  for (let i = 0; i < kk; i += 1) {
    num *= BigInt(n - i);
    den *= BigInt(i + 1);
  }
  return num / den;
}

// ---------------------------------------------------------------------------
// Univariate ℚ-factoring via factorIntegerPolynomial.
// ---------------------------------------------------------------------------

function factorUniQ(p: UniQPoly): UniQPoly[] | null {
  const np = uNormalize(p);
  if (np.length < 2) return null;
  // Clear denominators to integer coefficients.
  let denomLcm = 1n;
  for (const c of np) {
    denomLcm = blcm(denomLcm, c.denom);
  }
  const intP: bigint[] = np.map((c) => (c.numer * denomLcm) / c.denom);
  const [content, factors] = factorIntegerPolynomial(intP);
  if (factors.length === 0) return null;
  const flat: UniQPoly[] = [];
  for (const [coeffs, mult] of factors) {
    for (let i = 0; i < mult; i += 1) {
      flat.push(coeffs.map((c: bigint) => Rational.fromInt(c)));
    }
  }
  if (flat.length === 1) {
    const f0 = flat[0];
    const scale = new Rational(content, denomLcm);
    const scaled = uScale(f0, scale);
    if (scaled.length === np.length && scaled.every((v, i) => v.equals(np[i]))) {
      return null;
    }
  }
  if (flat.length > 0) {
    const scale = new Rational(content, denomLcm);
    flat[0] = uScale(flat[0], scale);
  }
  return flat;
}

// ---------------------------------------------------------------------------
// Two-factor bivariate Hensel lift.
// ---------------------------------------------------------------------------

function twoFactorLift(
  f: BiPoly,
  g0: UniQPoly,
  h0: UniQPoly,
  degY: number,
): [BiPoly, BiPoly] | null {
  let g: BiPoly = biUniX(g0);
  let h: BiPoly = biUniX(h0);

  for (let k = 1; k <= degY; k += 1) {
    const error = biSub(f, biMul(g, h));
    if (error.size === 0) break;
    const eK = biCoeffAtYPower(error, k);
    if (eK.length === 0) continue;
    const solved = uDiophantine(g0, h0, eK);
    if (solved === null) return null;
    const [u, v] = solved;
    for (let i = 0; i < v.length; i += 1) {
      if (v[i].isZero()) continue;
      const kk = key(i, k);
      const cur = g.get(kk) ?? Rational.ZERO;
      g.set(kk, cur.add(v[i]));
    }
    for (let i = 0; i < u.length; i += 1) {
      if (u[i].isZero()) continue;
      const kk = key(i, k);
      const cur = h.get(kk) ?? Rational.ZERO;
      h.set(kk, cur.add(u[i]));
    }
    g = biNormalize(g);
    h = biNormalize(h);
  }

  if (!biEquals(biMul(g, h), f)) return null;
  return [g, h];
}

// ---------------------------------------------------------------------------
// Top-level: tryBivariateHensel.
// ---------------------------------------------------------------------------

function y0Candidates(): number[] {
  const out: number[] = [0];
  let i = 1;
  while (out.length < MAX_Y0_SEARCH) {
    out.push(i);
    if (out.length < MAX_Y0_SEARCH) out.push(-i);
    i += 1;
  }
  return out;
}

function isLucky(p: BiPoly, image: UniQPoly): boolean {
  if (uDegree(image) !== biDegreeX(p)) return false;
  if (uDegree(image) < 1) return false;
  const deriv: Rational[] = [];
  for (let i = 1; i < image.length; i += 1) {
    deriv.push(Rational.fromInt(i).mul(image[i]));
  }
  const dn = uNormalize(deriv);
  if (dn.length === 0) return false;
  const [g] = uGcdExt(image, dn);
  return uDegree(g) === 0;
}

/**
 * Attempt to factor a bivariate polynomial via Hensel lifting.
 *
 * Returns a list of irreducible bivariate factors whose product equals
 * ``f``, or ``null`` if no non-trivial factorisation was found.
 */
export function tryBivariateHensel(fIn: BiPoly): BiPoly[] | null {
  const f = biNormalize(fIn);
  if (f.size === 0) return null;
  if (biDegreeY(f) < 1) return null;
  if (biDegreeX(f) < 1) return null;

  const degY = biDegreeY(f);

  for (const y0 of y0Candidates()) {
    const y0Frac = Rational.fromInt(y0);
    const fShifted = biShiftY(f, y0Frac);
    const image = biSubstituteY(fShifted, Rational.ZERO);
    if (!isLucky(fShifted, image)) continue;

    const uniFactors = factorUniQ(image);
    if (uniFactors === null || uniFactors.length < 2) continue;

    let remainingBi: BiPoly = fShifted;
    const biFactors: BiPoly[] = [];
    let remainingUni = [...uniFactors];
    let success = true;

    while (remainingUni.length >= 2) {
      const g0 = remainingUni[0];
      let h0: UniQPoly = [Rational.ONE];
      for (let i = 1; i < remainingUni.length; i += 1) {
        h0 = uMul(h0, remainingUni[i]);
      }
      const lifted = twoFactorLift(remainingBi, g0, h0, degY);
      if (lifted === null) {
        success = false;
        break;
      }
      const [gBi, hBi] = lifted;
      biFactors.push(gBi);
      remainingBi = hBi;
      remainingUni = remainingUni.slice(1);
    }
    if (!success) continue;
    biFactors.push(remainingBi);

    // Un-shift each factor back to the original y-frame.
    const factors = y0 === 0 ? biFactors : biFactors.map((fac) => biShiftY(fac, Rational.fromInt(-y0)));

    // Verify product reconstructs f.
    let prod: BiPoly = new Map([[key(0, 0), Rational.ONE]]);
    for (const fac of factors) prod = biMul(prod, fac);
    if (!biEquals(prod, f)) continue;

    // Filter out trivial (constant) factors; absorb their product into the
    // first non-trivial factor.
    const nonTrivial: BiPoly[] = [];
    let scalar = Rational.ONE;
    for (const fac of factors) {
      if (biDegreeX(fac) === 0 && biDegreeY(fac) === 0) {
        if (fac.size > 0) {
          for (const v of fac.values()) {
            scalar = scalar.mul(v);
            break;
          }
        }
      } else {
        nonTrivial.push(fac);
      }
    }
    if (nonTrivial.length < 2) continue;
    if (!scalar.equals(Rational.ONE)) {
      const scaleMap: BiPoly = new Map([[key(0, 0), scalar]]);
      nonTrivial[0] = biMul(nonTrivial[0], scaleMap);
    }
    return nonTrivial;
  }
  return null;
}

// Used by tests so they can verify products without re-implementing arithmetic.
export const _internals = {
  biMul,
  biNormalize,
  biEquals,
  key,
  nMul,
  nNormalize,
  nOne,
};

// ===========================================================================
// n-variate Hensel lifting — Track K2 (TS port of Python Track K1, PR #5590).
// ===========================================================================
//
// Strategy (one generic algorithm — NOT per-variable-count helpers):
//
//   1. Pick a "main" variable v_0 (always index 0 in the sparse-tuple
//      representation).
//   2. Substitute v_1..v_{n-1} with small integer values to reduce f to a
//      univariate polynomial in v_0.
//   3. Factor the univariate image via the existing factor-uni-q chain.
//   4. Lift the univariate factors back to the full n-variate ring one
//      variable at a time.  At step k we have factors of
//      ``f|_{v_{k+1}=a_{k+1}, …, v_{n-1}=a_{n-1}}`` in Q[v_0, …, v_{k-1}]
//      and lift to factors of ``f|_{v_{k+1}=a_{k+1}, …}`` in Q[v_0, …, v_k]
//      by Hensel-style expansion in powers of ``(v_k − a_k)``.
//   5. Each lift step solves a coefficient-ring diophantine equation
//      ``A·u + B·v = c`` in Q[v_0, …, v_{k-1}].  Recursively: when the
//      coefficient ring has ≥ 2 variables, specialise to reduce to Q[v_0],
//      solve via the existing uDiophantine, then lift back.  Base case
//      hits uDiophantine directly.
//   6. Verify the final product equals the input; if not, return null and
//      let the caller fall through.
//
// Representation:
//   ``NPoly = Map<string, Rational>`` where the string key is the comma-
//   joined exponent tuple ``"e_0,e_1,…,e_{n-1}"``.  Tuple length equals
//   the number of variables; the variable count must be passed alongside
//   this map because the empty polynomial doesn't carry it.
//
// Bounded resource discipline:
//   - At most MAX_N_SPECIALISATION lucky-point tuples are tried (10).
//   - Recursion depth bounded by n (number of variables).
//   - Each lift loop bounded by ``deg_{v_k}(f) + 1`` iterations.

/** Sparse n-variate polynomial — comma-joined exponent tuple ↦ coefficient. */
export type NPoly = Map<string, Rational>;

const MAX_N_SPECIALISATION = 10;

function nKey(tuple: number[]): string {
  return tuple.join(",");
}

function nParseKey(k: string, numVars: number): number[] {
  const out: number[] = [];
  let start = 0;
  for (let i = 0; i < numVars - 1; i += 1) {
    const idx = k.indexOf(",", start);
    out.push(Number(k.slice(start, idx)));
    start = idx + 1;
  }
  out.push(Number(k.slice(start)));
  return out;
}

function nNormalize(p: NPoly): NPoly {
  const out: NPoly = new Map();
  for (const [k, v] of p) {
    if (!v.isZero()) out.set(k, v);
  }
  return out;
}

function nZero(): NPoly {
  return new Map();
}

function nOne(numVars: number): NPoly {
  const k = nKey(new Array<number>(numVars).fill(0));
  return new Map([[k, Rational.ONE]]);
}

function nConst(numVars: number, c: Rational): NPoly {
  if (c.isZero()) return new Map();
  const k = nKey(new Array<number>(numVars).fill(0));
  return new Map([[k, c]]);
}

function nDegreeIn(p: NPoly, varIdx: number, numVars: number): number {
  let best = -1;
  for (const [k, v] of p) {
    if (v.isZero()) continue;
    const tup = nParseKey(k, numVars);
    if (tup[varIdx] > best) best = tup[varIdx];
  }
  return best;
}

function nTotalDegree(p: NPoly, numVars: number): number {
  let best = -1;
  for (const [k, v] of p) {
    if (v.isZero()) continue;
    const tup = nParseKey(k, numVars);
    let s = 0;
    for (const e of tup) s += e;
    if (s > best) best = s;
  }
  return best;
}

function nAdd(a: NPoly, b: NPoly): NPoly {
  const out: NPoly = new Map(a);
  for (const [k, v] of b) {
    const cur = out.get(k) ?? Rational.ZERO;
    out.set(k, cur.add(v));
  }
  return nNormalize(out);
}

function nSub(a: NPoly, b: NPoly): NPoly {
  const out: NPoly = new Map(a);
  for (const [k, v] of b) {
    const cur = out.get(k) ?? Rational.ZERO;
    out.set(k, cur.sub(v));
  }
  return nNormalize(out);
}

function nMul(a: NPoly, b: NPoly, numVars: number): NPoly {
  const out: NPoly = new Map();
  for (const [k1, c1] of a) {
    if (c1.isZero()) continue;
    const t1 = nParseKey(k1, numVars);
    for (const [k2, c2] of b) {
      if (c2.isZero()) continue;
      const t2 = nParseKey(k2, numVars);
      const t: number[] = new Array<number>(numVars);
      for (let i = 0; i < numVars; i += 1) t[i] = t1[i] + t2[i];
      const k = nKey(t);
      const cur = out.get(k) ?? Rational.ZERO;
      out.set(k, cur.add(c1.mul(c2)));
    }
  }
  return nNormalize(out);
}

function nEquals(a: NPoly, b: NPoly): boolean {
  const na = nNormalize(a);
  const nb = nNormalize(b);
  if (na.size !== nb.size) return false;
  for (const [k, v] of na) {
    const w = nb.get(k);
    if (w === undefined || !w.equals(v)) return false;
  }
  return true;
}

/** Substitute v_{varIdx} = value, keep the tuple shape (slot stays at exponent 0). */
function nSubstituteVarKeep(p: NPoly, varIdx: number, value: Rational, numVars: number): NPoly {
  const out: NPoly = new Map();
  for (const [k, c] of p) {
    const tup = nParseKey(k, numVars);
    const e = tup[varIdx];
    const newTup = [...tup];
    newTup[varIdx] = 0;
    const contrib = c.mul(value.pow(e));
    if (contrib.isZero()) continue;
    const nk = nKey(newTup);
    const cur = out.get(nk) ?? Rational.ZERO;
    out.set(nk, cur.add(contrib));
  }
  return nNormalize(out);
}

/** Extract the (n−1)-variate-feeling coefficient at varIdx^kPow — but kept in n-variate ring with varIdx slot = 0. */
function nCoeffAtPower(p: NPoly, varIdx: number, kPow: number, numVars: number): NPoly {
  const out: NPoly = new Map();
  for (const [k, c] of p) {
    const tup = nParseKey(k, numVars);
    if (tup[varIdx] !== kPow) continue;
    const newTup = [...tup];
    newTup[varIdx] = 0;
    const nk = nKey(newTup);
    const cur = out.get(nk) ?? Rational.ZERO;
    out.set(nk, cur.add(c));
  }
  return nNormalize(out);
}

/** Embed a univariate-in-varIdx polynomial into the n-variate ring. */
function uToN(p: UniQPoly, varIdx: number, numVars: number): NPoly {
  const out: NPoly = new Map();
  for (let e = 0; e < p.length; e += 1) {
    if (p[e].isZero()) continue;
    const tup = new Array<number>(numVars).fill(0);
    tup[varIdx] = e;
    out.set(nKey(tup), p[e]);
  }
  return out;
}

/** Convert a polynomial that only uses varIdx to a UniQPoly. */
function nToUnivariate(p: NPoly, varIdx: number, numVars: number): UniQPoly {
  if (p.size === 0) return [];
  let maxE = 0;
  for (const k of p.keys()) {
    const tup = nParseKey(k, numVars);
    if (tup[varIdx] > maxE) maxE = tup[varIdx];
  }
  const out: Rational[] = Array.from({ length: maxE + 1 }, () => Rational.ZERO);
  for (const [k, c] of p) {
    const tup = nParseKey(k, numVars);
    out[tup[varIdx]] = out[tup[varIdx]].add(c);
  }
  return uNormalize(out);
}

function nOnlyUsesVar(p: NPoly, varIdx: number, numVars: number): boolean {
  for (const k of p.keys()) {
    const tup = nParseKey(k, numVars);
    for (let i = 0; i < numVars; i += 1) {
      if (i !== varIdx && tup[i] !== 0) return false;
    }
  }
  return true;
}

/** Rewrite p as polynomial in (v_{varIdx} − value) — binomial expansion. */
function nShiftVar(p: NPoly, varIdx: number, value: Rational, numVars: number): NPoly {
  if (value.isZero()) return new Map(p);
  const out: NPoly = new Map();
  for (const [k, c] of p) {
    if (c.isZero()) continue;
    const tup = nParseKey(k, numVars);
    const e = tup[varIdx];
    for (let m = 0; m <= e; m += 1) {
      const coeff = c.mul(Rational.fromInt(binomial(e, m))).mul(value.pow(e - m));
      const newTup = [...tup];
      newTup[varIdx] = m;
      const nk = nKey(newTup);
      const cur = out.get(nk) ?? Rational.ZERO;
      out.set(nk, cur.add(coeff));
    }
  }
  return nNormalize(out);
}

// ---------------------------------------------------------------------------
// Recursive coefficient-ring diophantine: solve u·g0 + v·h0 = c in
// Q[active_vars] ⊂ Q[v_0, …, v_{n-1}].
// ---------------------------------------------------------------------------

function nDiophantine(
  g0: NPoly,
  h0: NPoly,
  c: NPoly,
  numVars: number,
  mainVar: number,
  activeVars: readonly number[],
): [NPoly, NPoly] | null {
  // Base case: univariate ring.
  if (activeVars.length === 1) {
    const only = activeVars[0];
    if (!(nOnlyUsesVar(g0, only, numVars) && nOnlyUsesVar(h0, only, numVars) && nOnlyUsesVar(c, only, numVars))) {
      return null;
    }
    const g0u = nToUnivariate(g0, only, numVars);
    const h0u = nToUnivariate(h0, only, numVars);
    const cu = nToUnivariate(c, only, numVars);
    const solved = uDiophantine(g0u, h0u, cu);
    if (solved === null) return null;
    const [uu, vu] = solved;
    return [uToN(uu, only, numVars), uToN(vu, only, numVars)];
  }

  // Recursive case: eliminate the last variable in activeVars.
  const w = activeVars[activeVars.length - 1];
  const rest = activeVars.slice(0, -1);

  const maxWDeg = Math.max(
    nDegreeIn(g0, w, numVars),
    nDegreeIn(h0, w, numVars),
    nDegreeIn(c, w, numVars),
    0,
  );

  const candidates = y0Candidates().slice(0, MAX_N_SPECIALISATION);
  for (const w0Int of candidates) {
    const w0 = Rational.fromInt(w0Int);
    const g0Shift = nShiftVar(g0, w, w0, numVars);
    const h0Shift = nShiftVar(h0, w, w0, numVars);
    const cShift = nShiftVar(c, w, w0, numVars);
    const g0Base = nCoeffAtPower(g0Shift, w, 0, numVars);
    const h0Base = nCoeffAtPower(h0Shift, w, 0, numVars);
    const cBase = nCoeffAtPower(cShift, w, 0, numVars);

    const solved = nDiophantine(g0Base, h0Base, cBase, numVars, mainVar, rest);
    if (solved === null) continue;
    let [u, v] = solved;

    let success = true;
    for (let k = 1; k <= maxWDeg; k += 1) {
      const prod = nAdd(nMul(u, g0Shift, numVars), nMul(v, h0Shift, numVars));
      const err = nSub(cShift, prod);
      if (err.size === 0) break;
      const eK = nCoeffAtPower(err, w, k, numVars);
      if (eK.size === 0) continue;
      const sub = nDiophantine(g0Base, h0Base, eK, numVars, mainVar, rest);
      if (sub === null) {
        success = false;
        break;
      }
      const [du, dv] = sub;
      const uNext: NPoly = new Map(u);
      for (const [keyDu, coef] of du) {
        const tup = nParseKey(keyDu, numVars);
        const newTup = [...tup];
        newTup[w] = k;
        const nk = nKey(newTup);
        const cur = uNext.get(nk) ?? Rational.ZERO;
        uNext.set(nk, cur.add(coef));
      }
      const vNext: NPoly = new Map(v);
      for (const [keyDv, coef] of dv) {
        const tup = nParseKey(keyDv, numVars);
        const newTup = [...tup];
        newTup[w] = k;
        const nk = nKey(newTup);
        const cur = vNext.get(nk) ?? Rational.ZERO;
        vNext.set(nk, cur.add(coef));
      }
      u = nNormalize(uNext);
      v = nNormalize(vNext);
    }

    if (!success) continue;

    // Verify in the shifted ring.
    const check = nAdd(nMul(u, g0Shift, numVars), nMul(v, h0Shift, numVars));
    if (!nEquals(check, nNormalize(cShift))) continue;

    // Unshift u and v back.
    if (!w0.isZero()) {
      u = nShiftVar(u, w, w0.neg(), numVars);
      v = nShiftVar(v, w, w0.neg(), numVars);
    }
    return [u, v];
  }
  return null;
}

// ---------------------------------------------------------------------------
// n-variate two-factor lift.
// ---------------------------------------------------------------------------

function nTwoFactorLift(
  f: NPoly,
  g0: NPoly,
  h0: NPoly,
  numVars: number,
  mainVar: number,
  liftVar: number,
  coeffVars: readonly number[],
): [NPoly, NPoly] | null {
  let g: NPoly = new Map(g0);
  let h: NPoly = new Map(h0);

  const degLift = nDegreeIn(f, liftVar, numVars);
  const active: number[] = [mainVar, ...coeffVars];

  for (let k = 1; k <= degLift; k += 1) {
    const error = nSub(f, nMul(g, h, numVars));
    if (error.size === 0) break;
    const eK = nCoeffAtPower(error, liftVar, k, numVars);
    if (eK.size === 0) continue;
    const solved = nDiophantine(g0, h0, eK, numVars, mainVar, active);
    if (solved === null) return null;
    const [du, dv] = solved;
    // du is the correction to h (matches bivariate convention).
    const hNext: NPoly = new Map(h);
    for (const [keyDu, coef] of du) {
      const tup = nParseKey(keyDu, numVars);
      const newTup = [...tup];
      newTup[liftVar] = k;
      const nk = nKey(newTup);
      const cur = hNext.get(nk) ?? Rational.ZERO;
      hNext.set(nk, cur.add(coef));
    }
    const gNext: NPoly = new Map(g);
    for (const [keyDv, coef] of dv) {
      const tup = nParseKey(keyDv, numVars);
      const newTup = [...tup];
      newTup[liftVar] = k;
      const nk = nKey(newTup);
      const cur = gNext.get(nk) ?? Rational.ZERO;
      gNext.set(nk, cur.add(coef));
    }
    g = nNormalize(gNext);
    h = nNormalize(hNext);
  }

  if (!nEquals(nMul(g, h, numVars), nNormalize(f))) return null;
  return [g, h];
}

// ---------------------------------------------------------------------------
// Top-level n-variate Hensel.
// ---------------------------------------------------------------------------

function nSpecialisationCandidates(numAux: number): number[][] {
  if (numAux === 0) return [[]];
  const primitives: number[] = [1, 2, -1, 3, -2];
  const tuples: number[][] = [];
  // Tier 1: constant tuples.
  for (const v of primitives) {
    tuples.push(new Array<number>(numAux).fill(v));
    if (tuples.length >= MAX_N_SPECIALISATION) return tuples;
  }
  // Tier 2: vary one coordinate at a time off (1,1,…) base.
  const base = new Array<number>(numAux).fill(1);
  for (let i = 0; i < numAux; i += 1) {
    for (let j = 1; j < primitives.length; j += 1) {
      const cand = [...base];
      cand[i] = primitives[j];
      tuples.push(cand);
      if (tuples.length >= MAX_N_SPECIALISATION) return tuples;
    }
  }
  return tuples.slice(0, MAX_N_SPECIALISATION);
}

function isLuckyUni(pN: NPoly, image: UniQPoly, mainVar: number, numVars: number): boolean {
  if (uDegree(image) !== nDegreeIn(pN, mainVar, numVars)) return false;
  if (uDegree(image) < 1) return false;
  const deriv: Rational[] = [];
  for (let i = 1; i < image.length; i += 1) {
    deriv.push(Rational.fromInt(i).mul(image[i]));
  }
  const dn = uNormalize(deriv);
  if (dn.length === 0) return false;
  const [g] = uGcdExt(image, dn);
  return uDegree(g) === 0;
}

/**
 * Attempt to factor an n-variate (n ≥ 2) polynomial via iterated bivariate
 * Hensel lifting.
 *
 * @param fIn  Sparse n-variate polynomial in ℚ[v_0, …, v_{numVars-1}].
 * @param numVars Number of variables; must be ≥ 2 for a non-trivial result.
 * @returns A list of factors whose product equals `fIn`, or `null` when no
 *          factorisation was found.
 *
 * Falls through (`null`) when numVars < 2, the polynomial doesn't genuinely
 * depend on at least two variables, no lucky specialisation tuple gives a
 * squarefree univariate image of full v_0-degree (bounded search of 10
 * tuples), the univariate image is irreducible, or any lift/verification
 * step fails.
 */
export function tryNVariateHensel(fIn: NPoly, numVars: number): NPoly[] | null {
  const f = nNormalize(fIn);
  if (f.size === 0) return null;
  if (numVars < 2) return null;
  if (nDegreeIn(f, 0, numVars) < 1) return null;
  let anyAux = false;
  for (let i = 1; i < numVars; i += 1) {
    if (nDegreeIn(f, i, numVars) >= 1) {
      anyAux = true;
      break;
    }
  }
  if (!anyAux) return null;

  const mainVar = 0;
  const auxVars: number[] = [];
  for (let i = 1; i < numVars; i += 1) auxVars.push(i);

  for (const specTuple of nSpecialisationCandidates(auxVars.length)) {
    const spec = new Map<number, Rational>();
    for (let i = 0; i < auxVars.length; i += 1) {
      spec.set(auxVars[i], Rational.fromInt(specTuple[i]));
    }

    // Shift f so each auxiliary variable is recentred to 0.
    let fShift = f;
    for (const [vI, wI] of spec) {
      fShift = nShiftVar(fShift, vI, wI, numVars);
    }
    fShift = nNormalize(fShift);

    // Specialise all aux vars to 0 → univariate in main_var.
    let fUni = fShift;
    for (const vI of auxVars) {
      fUni = nSubstituteVarKeep(fUni, vI, Rational.ZERO, numVars);
    }
    if (!nOnlyUsesVar(fUni, mainVar, numVars)) continue;
    const image = nToUnivariate(fUni, mainVar, numVars);
    if (!isLuckyUni(fShift, image, mainVar, numVars)) continue;

    const uniFactors = factorUniQ(image);
    if (uniFactors === null || uniFactors.length < 2) continue;

    let nFactorsCurrent: NPoly[] = uniFactors.map((u) => uToN(u, mainVar, numVars));

    let success = true;
    for (let liftIdx = 0; liftIdx < auxVars.length; liftIdx += 1) {
      const liftVar = auxVars[liftIdx];
      let fStage = fShift;
      for (let later = liftIdx + 1; later < auxVars.length; later += 1) {
        fStage = nSubstituteVarKeep(fStage, auxVars[later], Rational.ZERO, numVars);
      }
      fStage = nNormalize(fStage);

      const coeffVars = auxVars.slice(0, liftIdx);

      let remaining = fStage;
      const newFactors: NPoly[] = [];
      let remainingFactors = [...nFactorsCurrent];
      while (remainingFactors.length >= 2) {
        const g0 = remainingFactors[0];
        let h0 = nOne(numVars);
        for (let i = 1; i < remainingFactors.length; i += 1) {
          h0 = nMul(h0, remainingFactors[i], numVars);
        }
        const lifted = nTwoFactorLift(remaining, g0, h0, numVars, mainVar, liftVar, coeffVars);
        if (lifted === null) {
          success = false;
          break;
        }
        const [gLift, hLift] = lifted;
        newFactors.push(gLift);
        remaining = hLift;
        remainingFactors = remainingFactors.slice(1);
      }
      if (!success) break;
      newFactors.push(remaining);
      nFactorsCurrent = newFactors;
    }
    if (!success) continue;

    // Unshift each factor back to original frame.
    let result = nFactorsCurrent;
    for (const [vI, wI] of spec) {
      result = result.map((fac) => nShiftVar(fac, vI, wI.neg(), numVars));
    }
    result = result.map((fac) => nNormalize(fac));

    // Verify product reconstructs f.
    let prod: NPoly = nOne(numVars);
    for (const fac of result) prod = nMul(prod, fac, numVars);
    if (!nEquals(prod, nNormalize(f))) continue;

    // Drop pure constants, fold scalar into factor 0.
    const nonTrivial: NPoly[] = [];
    let scalar = Rational.ONE;
    for (const fac of result) {
      if (nTotalDegree(fac, numVars) <= 0) {
        if (fac.size > 0) {
          for (const v of fac.values()) {
            scalar = scalar.mul(v);
            break;
          }
        }
      } else {
        nonTrivial.push(fac);
      }
    }
    if (nonTrivial.length < 2) continue;
    if (!scalar.equals(Rational.ONE)) {
      nonTrivial[0] = nMul(nonTrivial[0], nConst(numVars, scalar), numVars);
    }
    return nonTrivial;
  }
  return null;
}
