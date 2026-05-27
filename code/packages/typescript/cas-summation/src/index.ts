import {
  ADD,
  DIV,
  EXP,
  LOG,
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

  // Phase 39 (finite) + Phase 41/42 (infinite) telescoping sums.
  //
  // Detect ``f = g(k+1) − g(k)`` (or its antisymmetric ``g(k) − g(k+1)``)
  // and emit a closed form:
  // - Phase 39 (finite ``hi``): ``g(hi+1) − g(lo)`` / ``g(lo) − g(hi+1)``.
  // - Phase 41+42 (``hi = %inf``): emit ``−g(lo)`` (standard) or ``g(lo)``
  //   (antisymmetric) when ``g(k)`` provably vanishes at infinity per
  //   :func:`gVanishesAtInfinity` (constant numerator + positive-degree
  //   polynomial denominator, or any proper rational with
  //   ``deg(num) < deg(den)``).  When the limit isn't decidable by the
  //   narrow recogniser, fall through to later rules — the original
  //   unevaluated ``Sum`` node is then returned at the bottom.
  {
    const tele = tryTelescoping(f, k, evalFn);
    if (tele !== undefined) {
      if (infUpper) {
        if (gVanishesAtInfinity(tele.gExpr, k)) {
          const gAtLo = substitute(tele.gExpr, k, lo);
          if (tele.sign === 1) {
            // ∑[g(k+1) − g(k)] from lo to ∞ = 0 − g(lo) = −g(lo)
            return evalFn(app(NEG, [gAtLo]));
          }
          // ∑[g(k) − g(k+1)] from lo to ∞ = g(lo) − 0 = g(lo)
          return evalFn(gAtLo);
        }
        // Limit not provably zero — fall through.
      } else {
        const hiPlusOne = app(ADD, [hi, int(1)]);
        const gAtHiPlusOne = substitute(tele.gExpr, k, hiPlusOne);
        const gAtLo = substitute(tele.gExpr, k, lo);
        if (tele.sign === 1) {
          return evalFn(app(SUB, [gAtHiPlusOne, gAtLo]));
        }
        return evalFn(app(SUB, [gAtLo, gAtHiPlusOne]));
      }
    }
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

/**
 * Phase 40+46 (TypeScript port): Detect whether ``node`` represents a
 * negation, and if so return the corresponding positive magnitude.
 *
 * Two recognised shapes:
 *
 *   1.  Top-level ``Neg(x)``                       → ``x``
 *   2.  ``Div(c, d)`` with literal ``c < 0``       → ``Div(|c|, d)``
 *
 * Case 2 is the Phase 46 widening — Python's ``Apart`` of
 * ``5/(k(k+1))`` returns ``Add(Div(-5, k+1), Div(5, k))`` with the
 * negation folded into the numerator.  Even without ``Apart`` on the
 * TypeScript side, users who write ``g(k+1) − g(k)`` as
 * ``Add(g(k+1), Div(-1, ...))`` directly get the benefit of the
 * widened telescope detector.
 *
 * Returns ``undefined`` when ``node`` is not a recognised negation.
 */
function extractNegation(node: IRNode): IRNode | undefined {
  if (node.kind !== "apply") return undefined;
  // Case 1: top-level Neg wrapper.
  if (irEquals(node.head, NEG) && node.args.length === 1) {
    return node.args[0];
  }
  // Case 2: Div with a negative literal numerator (Integer or
  // Rational).  Only handle the canonical two-arg Div shape.
  if (irEquals(node.head, DIV) && node.args.length === 2) {
    const [numer, denom] = node.args;
    if (numer.kind === "integer" && numer.value < 0n) {
      return app(DIV, [int(-numer.value), denom]);
    }
    if (numer.kind === "rational" && numer.numer < 0n) {
      return app(DIV, [rational(-numer.numer, numer.denom), denom]);
    }
  }
  return undefined;
}

/**
 * Phase 40+46 (TypeScript port): Rewrite two-term ``Add`` nodes
 * containing a (recognised) negation into the equivalent ``Sub`` shape.
 *
 * Used by :func:`tryTelescoping` as a fallback when the direct ``Sub``
 * match fails — the telescope detector keys off ``Sub``, so summands
 * already in ``Add(a, Neg(b))`` or ``Add(a, Div(-c, d))`` form would
 * otherwise miss.
 *
 *   Input shape                              | Output
 *   -----------------------------------------+----------------------
 *   ``Add(a, Neg(b))``                       | ``Sub(a, b)``
 *   ``Add(Neg(b), a)``                       | ``Sub(a, b)``
 *   ``Add(a, Div(-c, d))`` (Phase 46)        | ``Sub(a, Div(c, d))``
 *   ``Add(Div(-c, d), a)`` (Phase 46)        | ``Sub(a, Div(c, d))``
 *   ``Add(Neg(a), Neg(b))``                  | unchanged
 *   anything else                            | unchanged
 *
 * Returns the input unchanged when no rewrite applies.
 */
function normaliseAddNegToSub(node: IRNode): IRNode {
  if (node.kind !== "apply" || !irEquals(node.head, ADD) || node.args.length !== 2) {
    return node;
  }
  const [left, right] = node.args;
  const leftPos = extractNegation(left);
  const rightPos = extractNegation(right);
  if (leftPos !== undefined && rightPos !== undefined) {
    // Both sides genuinely negative — no telescope to expose.
    return node;
  }
  if (rightPos !== undefined) {
    return app(SUB, [left, rightPos]);
  }
  if (leftPos !== undefined) {
    return app(SUB, [right, leftPos]);
  }
  return node;
}

/**
 * Phase 39: Detect a *structurally telescoping* summand
 * ``f = g(k+1) − g(k)`` (or its antisymmetric ``g(k) − g(k+1)``).
 *
 * The dispatcher then emits the closed form
 * ``g(hi+1) − g(lo)`` (sign +1) or ``g(lo) − g(hi+1)`` (sign −1)
 * by substituting the bounds into ``g_expr`` and subtracting.
 *
 * Detection is purely structural: substitute ``k → k+1`` in one half
 * of the ``SUB`` shape and compare against the other half after
 * normalisation via ``evalFn``.  No partial-fraction expansion is
 * attempted — the classic ``1/(k(k+1))`` form needs an explicit
 * ``Apart`` step first, which a follow-on phase can compose.
 *
 * Returns ``undefined`` when the summand is not a ``SUB`` of two
 * shifted halves of the same ``g(k)`` expression.
 */
function tryTelescoping(
  f: IRNode,
  k: IRNode,
  evalFn: EvalFn,
): { gExpr: IRNode; sign: 1 | -1 } | undefined {
  // Phase 46: if f is an Add-with-negation shape, normalise to Sub first
  // so the existing structural match below fires.  No-op when f is
  // already a Sub or a non-Add shape — the helper returns its input
  // unchanged in those cases.
  if (f.kind === "apply" && equals(f.head, ADD) && f.args.length === 2) {
    f = normaliseAddNegToSub(f);
  }
  if (f.kind !== "apply" || !equals(f.head, SUB) || f.args.length !== 2) {
    return undefined;
  }
  const [left, right] = f.args;
  const kPlusOne = app(ADD, [k, int(1)]);
  // Standard orientation: f = g(k+1) − g(k).  We check whether
  // substituting k → k+1 in `right` yields `left` (after normalisation).
  const rightShifted = substitute(right, k, kPlusOne);
  if (equals(evalFn(rightShifted), evalFn(left))) {
    return { gExpr: right, sign: 1 };
  }
  // Antisymmetric: f = g(k) − g(k+1).  Check whether substituting
  // k → k+1 in `left` yields `right`.
  const leftShifted = substitute(left, k, kPlusOne);
  if (equals(evalFn(leftShifted), evalFn(right))) {
    return { gExpr: left, sign: -1 };
  }
  return undefined;
}

/**
 * Phase 41+42: True when ``node`` is recognised as a polynomial in ``k``
 * of strictly positive degree.
 *
 * Used by :func:`gVanishesAtInfinity` to decide whether a denominator
 * grows without bound as ``k → ∞``.  Recognised shapes: ``k``, ``k^n``
 * (integer n ≥ 1), ``Add`` with at least one positive-degree term and
 * all other args either constant-in-k or positive-degree, and ``Mul``
 * with at least one positive-degree factor and all others either
 * constant-in-k or positive-degree.  Anything else returns ``false``.
 */
function isPositiveDegreePolynomialInK(node: IRNode, k: IRNode): boolean {
  if (equals(node, k)) return true;
  if (node.kind !== "apply") return false;
  if (equals(node.head, POW) && node.args.length === 2) {
    const [base, exp] = node.args;
    if (equals(base, k) && exp.kind === "integer" && exp.value >= 1n) {
      return true;
    }
  }
  if (equals(node.head, ADD) && node.args.length >= 2) {
    if (!node.args.some((a) => isPositiveDegreePolynomialInK(a, k))) return false;
    return node.args.every(
      (a) => isConstantIn(a, k) || isPositiveDegreePolynomialInK(a, k),
    );
  }
  if (equals(node.head, MUL) && node.args.length >= 2) {
    let hasPositive = false;
    for (const arg of node.args) {
      if (isConstantIn(arg, k)) continue;
      if (isPositiveDegreePolynomialInK(arg, k)) {
        hasPositive = true;
        continue;
      }
      return false;
    }
    return hasPositive;
  }
  return false;
}

/**
 * Phase 42: Return the polynomial degree of ``node`` in ``k``, or
 * ``undefined`` for non-polynomial shapes.
 *
 * +-----------------------+---------------------------+
 * | Input                 | Returns                   |
 * +=======================+===========================+
 * | constant in k         | 0                         |
 * | k                     | 1                         |
 * | k^n (integer n ≥ 0)   | n                         |
 * | Neg(p)                | deg(p)                    |
 * | Add(p1, p2, …)        | max(deg(pi))              |
 * | Sub(p1, p2)           | max(deg(p1), deg(p2))     |
 * | Mul(p1, p2, …)        | sum(deg(pi))              |
 * | otherwise             | undefined                 |
 * +-----------------------+---------------------------+
 */
function polynomialDegreeInK(node: IRNode, k: IRNode): number | undefined {
  if (isConstantIn(node, k)) return 0;
  if (equals(node, k)) return 1;
  if (node.kind !== "apply") return undefined;
  if (equals(node.head, POW) && node.args.length === 2) {
    const [base, exp] = node.args;
    if (equals(base, k) && exp.kind === "integer" && exp.value >= 0n) {
      return Number(exp.value);
    }
    return undefined;
  }
  if (equals(node.head, NEG) && node.args.length === 1) {
    return polynomialDegreeInK(node.args[0], k);
  }
  if (equals(node.head, ADD) || equals(node.head, SUB)) {
    const degrees = node.args.map((a) => polynomialDegreeInK(a, k));
    if (degrees.some((d) => d === undefined)) return undefined;
    return Math.max(...(degrees as number[]));
  }
  if (equals(node.head, MUL)) {
    const degrees = node.args.map((a) => polynomialDegreeInK(a, k));
    if (degrees.some((d) => d === undefined)) return undefined;
    return (degrees as number[]).reduce((a, b) => a + b, 0);
  }
  return undefined;
}

/**
 * Phase 41+42: True when ``g(k)`` provably tends to 0 as ``k → ∞``.
 *
 * Two-tier recognition:
 *
 * 1. **Phase 41 fast path** — ``Div(c, h(k))`` with ``c`` constant in
 *    ``k`` and ``h(k)`` recognised as a positive-degree polynomial.
 * 2. **Phase 42 widening** — ``Div(P(k), Q(k))`` with both
 *    ``P`` and ``Q`` pure polynomials in ``k`` and
 *    ``deg(P) < deg(Q)``.
 *
 * Anything else (transcendental numerator, improper rational with
 * ``deg(P) ≥ deg(Q)``, non-Div shapes) returns ``false`` —
 * conservatively refusing keeps the closed-form emission safe.
 */
/**
 * Phase 43 helper: return the sign (+1 or -1) of the leading
 * coefficient of `node` as a polynomial in `k`, or `undefined` for
 * non-polynomial / degree-0 / unknown-sign shapes.
 *
 * Required by `hDivergesAtInfinity` to verify that `Exp(h)` / `Pow(b, h)`
 * actually drive toward +∞ rather than 0.  Naïve "positive-degree
 * polynomial" tests accept `Mul(-1, k)` (i.e. `-k`) whose leading
 * coefficient is negative — without this helper we'd wrongly claim
 * `exp(-k)` or `2^(-k)` diverge (they vanish).
 */
function polynomialLeadingCoeffSignInK(
  node: IRNode,
  k: IRNode,
): 1 | -1 | undefined {
  if (isConstantIn(node, k)) return undefined;
  if (equals(node, k)) return 1;
  if (node.kind !== "apply") return undefined;
  // k^n (n >= 1)
  if (equals(node.head, POW) && node.args.length === 2) {
    const [base, exp] = node.args;
    if (equals(base, k) && exp.kind === "integer" && exp.value >= 1n) {
      return 1;
    }
    return undefined;
  }
  // Neg(p) flips sign.
  if (equals(node.head, NEG) && node.args.length === 1) {
    const inner = polynomialLeadingCoeffSignInK(node.args[0], k);
    return inner === undefined ? undefined : ((inner === 1 ? -1 : 1) as 1 | -1);
  }
  // Mul: multiply signs of constant + k-bearing factors.  Symbolic
  // constants of unknown sign / zero literals → undefined (refuse).
  if (equals(node.head, MUL)) {
    let sign: 1 | -1 = 1;
    let anyKBearing = false;
    for (const arg of node.args) {
      if (isConstantIn(arg, k)) {
        const val = rationalValue(arg);
        if (val === undefined) return undefined;
        if (val.numer === 0n) return undefined;
        if (val.numer < 0n) sign = (sign === 1 ? -1 : 1) as 1 | -1;
        continue;
      }
      const inner = polynomialLeadingCoeffSignInK(arg, k);
      if (inner === undefined) return undefined;
      sign = inner === 1 ? sign : ((sign === 1 ? -1 : 1) as 1 | -1);
      anyKBearing = true;
    }
    return anyKBearing ? sign : undefined;
  }
  // Add: dominated by the highest-degree term.  Tied max degrees → refuse
  // (leading coefficients could cancel).
  if (equals(node.head, ADD)) {
    let maxDeg = -1;
    let leaderSign: 1 | -1 | undefined;
    let tiedAtMax = false;
    for (const arg of node.args) {
      const deg = polynomialDegreeInK(arg, k);
      if (deg === undefined) return undefined;
      if (deg === 0) continue;
      if (deg > maxDeg) {
        maxDeg = deg;
        leaderSign = polynomialLeadingCoeffSignInK(arg, k);
        tiedAtMax = false;
      } else if (deg === maxDeg) {
        tiedAtMax = true;
      }
    }
    return tiedAtMax ? undefined : leaderSign;
  }
  return undefined;
}

/**
 * Phase 43: True when `node` provably diverges to ±∞ as `k → ∞`.
 *
 * Union of Phase 41/42 positive-degree polynomial + three transcendental
 * cases:
 *   1. `Exp(h(k))` with h positive-degree AND positive leading coeff.
 *   2. `Pow(b, h(k))` with rational |b| > 1 AND h positive-degree with
 *      positive leading coefficient.
 *   3. `Mul(...)` where at least one factor diverges and the rest are
 *      constant-in-k or also diverging.  Recursive.
 *
 * The sign-aware leading-coefficient check is critical: `exp(-k) → 0`
 * and `2^(-k) → 0`, not ∞.
 */
function hDivergesAtInfinity(node: IRNode, k: IRNode): boolean {
  // Phase 41/42 fast path.
  if (isPositiveDegreePolynomialInK(node, k)) return true;
  if (node.kind !== "apply") return false;
  // Phase 43: Exp(h) with h → +∞.
  if (equals(node.head, EXP) && node.args.length === 1) {
    const inner = node.args[0];
    if (isPositiveDegreePolynomialInK(inner, k)) {
      return polynomialLeadingCoeffSignInK(inner, k) === 1;
    }
    return false;
  }
  // Phase 43: Pow(b, h) with |b| > 1 and h → +∞.
  if (equals(node.head, POW) && node.args.length === 2) {
    const [base, exp] = node.args;
    if (isConstantIn(base, k)) {
      const baseVal = rationalValue(base);
      if (baseVal !== undefined) {
        // |base| > 1 iff |numer| > denom (denom always > 0 in normalised form).
        const absNumer = baseVal.numer < 0n ? -baseVal.numer : baseVal.numer;
        if (absNumer > baseVal.denom) {
          if (isPositiveDegreePolynomialInK(exp, k)) {
            if (polynomialLeadingCoeffSignInK(exp, k) === 1) {
              return true;
            }
          }
        }
      }
    }
  }
  // Phase 43: Mul(...) — at least one factor diverges, others constant
  // in k or also diverging.  Recursive.
  if (equals(node.head, MUL) && node.args.length >= 2) {
    let hasDivergent = false;
    for (const arg of node.args) {
      if (isConstantIn(arg, k)) continue;
      if (hDivergesAtInfinity(arg, k)) {
        hasDivergent = true;
        continue;
      }
      return false;
    }
    return hasDivergent;
  }
  // Phase 44: Log(h(k)) where h(k) → +∞.  Two requirements:
  //   (a) h(k) → +∞ (not just |h| → ∞)
  //   (b) h(k) > 0 for k sufficiently large
  // so log(h) is real-valued and diverges to +∞.
  //
  // Three sub-cases:
  //   - Polynomial h: require positive leading coefficient explicitly.
  //   - Exp(h'): always positive; defer to hDivergesAtInfinity recursion.
  //   - Pow(b, h'): require strictly positive base b > 1 (not just
  //     |b| > 1; Pow(-2, k) oscillates in sign so log((-2)^k) is
  //     not real-valued).
  // Other shapes (Log(const), Log(Sin), Log(Mul(...))) refused.
  if (equals(node.head, LOG) && node.args.length === 1) {
    const inner = node.args[0];
    if (isPositiveDegreePolynomialInK(inner, k)) {
      return polynomialLeadingCoeffSignInK(inner, k) === 1;
    }
    if (inner.kind === "apply" && equals(inner.head, EXP)) {
      return hDivergesAtInfinity(inner, k);
    }
    if (
      inner.kind === "apply" &&
      equals(inner.head, POW) &&
      inner.args.length === 2
    ) {
      const base = inner.args[0];
      if (isConstantIn(base, k)) {
        const baseVal = rationalValue(base);
        if (baseVal !== undefined) {
          // base > 1 strictly: numer > denom AND numer > 0 (denom > 0).
          if (baseVal.numer > baseVal.denom && baseVal.numer > 0n) {
            return hDivergesAtInfinity(inner, k);
          }
        }
      }
    }
    return false;
  }
  return false;
}

const _SIN_HEAD = sym("Sin");
const _COS_HEAD = sym("Cos");

/**
 * Phase 49: True when ``node`` is *provably* uniformly bounded in ``k``.
 *
 * Used by :func:`gVanishesAtInfinity` to recognise shapes like
 * ``sin(k)/k²`` where the numerator is bounded (``|sin(k)| ≤ 1``) and
 * the denominator diverges, hence the quotient vanishes.
 *
 *   node shape                   | Provably bounded?
 *   -----------------------------|----------------------------
 *   constant in ``k``            | yes (trivially)
 *   ``Sin(h(k))`` / ``Cos(...)`` | yes (``|sin|, |cos| ≤ 1``)
 *   ``Mul(bounded, bounded)``    | yes (recursive)
 *   ``Add(bounded, bounded)``    | yes (recursive)
 *   ``Neg(bounded)``             | yes
 *   ``k`` / ``k²``               | no (diverges)
 *   ``Exp(k)`` / ``Log(k)``      | no (diverges)
 *
 * Conservative — when in doubt, returns false.
 */
function isBoundedInK(node: IRNode, k: IRNode): boolean {
  if (isConstantIn(node, k)) return true;
  if (node.kind !== "apply") return false;
  if (equals(node.head, _SIN_HEAD) && node.args.length === 1) return true;
  if (equals(node.head, _COS_HEAD) && node.args.length === 1) return true;
  if (equals(node.head, MUL)) {
    return node.args.every((a) => isBoundedInK(a, k));
  }
  if (equals(node.head, ADD)) {
    return node.args.every((a) => isBoundedInK(a, k));
  }
  if (equals(node.head, NEG) && node.args.length === 1) {
    return isBoundedInK(node.args[0], k);
  }
  return false;
}

/**
 * Phase 50 (TypeScript port): True when ``node = Log(h(k))`` with
 * ``h(k) → +∞``.
 *
 * The squeeze argument: ``log(h) → ∞`` at a logarithmic rate, while
 * any positive-degree polynomial / exponential denominator grows
 * strictly faster.  Sign-aware: delegates to ``hDivergesAtInfinity``
 * on the full ``Log(...)`` node so Phase 44's Log branch refuses
 * ``Log(Mul(-1, k))``-style negative-polynomial shapes for free.
 */
function isLogOfDivergingInK(node: IRNode, k: IRNode): boolean {
  if (node.kind !== "apply") return false;
  if (!equals(node.head, LOG) || node.args.length !== 1) return false;
  return hDivergesAtInfinity(node, k);
}

const _SQRT_HEAD = sym("Sqrt");

/**
 * Phase 51 (TypeScript port): Return the effective polynomial half-
 * degree of ``Sqrt(P(k))`` when ``P`` is a positive-degree polynomial
 * with positive leading coefficient.  Returns ``undefined`` otherwise.
 *
 * The half-degree is ``deg(P) / 2`` (so ``sqrt(k³)`` is degree ``1.5``).
 * Used in :func:`gVanishesAtInfinity` to recognise that
 * ``sqrt(P)/Q`` vanishes when ``deg(Q) > deg(P)/2``.
 *
 * Conservative — refuses ``Sqrt(negative-polynomial)`` (not real) and
 * non-Sqrt heads.
 */
function sqrtEffectiveHalfDegree(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply") return undefined;
  if (!equals(node.head, _SQRT_HEAD) || node.args.length !== 1) return undefined;
  const inner = node.args[0];
  const innerDeg = polynomialDegreeInK(inner, k);
  if (innerDeg === undefined || innerDeg < 1) return undefined;
  if (polynomialLeadingCoeffSignInK(inner, k) !== 1) return undefined;
  return innerDeg / 2;
}

/**
 * Phase 52 (TypeScript port): Return the effective polynomial degree of the
 * polynomial part when ``node = Mul(bounded_factors, polynomial_factors)`` in
 * ``k``; ``undefined`` otherwise.
 *
 * Used by :func:`gVanishesAtInfinity` to recognise that ``sin(k)·k/k³``
 * vanishes (bounded × deg 1 over deg 3).  The bounded part must contain at
 * least one non-constant-in-k factor — otherwise Phase 49 would catch the
 * whole numerator as a single bounded expression.
 *
 * Algorithm:
 *   1. Require ``node = Mul(...)``.
 *   2. Partition each factor into bounded vs polynomial buckets.
 *      Factors that are neither bounded nor polynomial → return undefined.
 *   3. Require ≥ 1 non-constant-in-k bounded factor.
 *   4. Sum the polynomial factors' degrees.
 *   5. Return ``{ bounded: aggregate, polyDeg: summed }``.
 */
function splitBoundedPolynomialFactor(
  node: IRNode,
  k: IRNode
): { bounded: IRNode; polyDeg: number } | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const boundedFactors: IRNode[] = [];
  let polyDeg = 0;
  let hasNonConstantBounded = false;
  for (const arg of node.args) {
    if (isBoundedInK(arg, k)) {
      boundedFactors.push(arg);
      if (!isConstantIn(arg, k)) hasNonConstantBounded = true;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg === undefined) return undefined;  // Unrecognised factor.
    polyDeg += deg;
  }
  // Pure polynomial — Phase 42 will handle it; no non-constant bounded factor.
  if (!hasNonConstantBounded) return undefined;
  if (boundedFactors.length === 0) return undefined;
  const bounded =
    boundedFactors.length === 1
      ? boundedFactors[0]
      : app(MUL, boundedFactors);
  return { bounded, polyDeg };
}

/**
 * Phase 53 (TypeScript port): Return the effective growth degree of a
 * ``Mul(Sqrt(P), polynomial_factors)`` numerator, or ``undefined`` when
 * the shape isn't recognised.
 *
 * The numerator ``Sqrt(P(k)) · Q(k)`` grows at rate
 * ``deg(P)/2 + deg(Q)``.  Returns that combined value (a possibly
 * fractional number).  The caller compares against ``den_deg`` directly.
 *
 * Requirements:
 *   - ``node = Mul(...)`` — the plain-``Sqrt`` case is handled by Phase 51.
 *   - Exactly one factor is a ``Sqrt(P)`` with positive-leading-coeff poly inner.
 *   - All remaining factors are polynomials in ``k``.
 */
function sqrtPolyNumeratorEffectiveDegree(
  node: IRNode,
  k: IRNode
): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let polyDegSum = 0;
  for (const arg of node.args) {
    const sqrtDeg = sqrtEffectiveHalfDegree(arg, k);
    if (sqrtDeg !== undefined) {
      // Only one Sqrt factor is allowed; bail on a second.
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sqrtDeg;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg === undefined) return undefined; // Neither Sqrt nor polynomial.
    polyDegSum += deg;
  }
  // Must have found exactly one Sqrt factor.
  if (sqrtHalfDeg === undefined) return undefined;
  return sqrtHalfDeg + polyDegSum;
}

/**
 * Phase 54 (TypeScript port): Return the effective polynomial degree of a
 * ``Mul(Log(diverging), polynomial_factors)`` numerator, or ``undefined``
 * when the shape isn't recognised.
 *
 * ``log(h(k))`` grows sub-polynomially — ``log(h) = o(k^ε)`` for any
 * ``ε > 0`` — so the effective growth degree of ``log(h) · P(k)``
 * equals ``deg(P)`` alone.  The quotient vanishes when
 * ``den_deg > poly_deg`` (strictly).
 *
 * Requirements:
 *   - ``node = Mul(...)`` — a bare ``Log(h)`` numerator goes via Phase 50.
 *   - Exactly one factor passes ``isLogOfDivergingInK``.
 *   - All remaining factors are polynomials in ``k``.
 */
function splitLogPolynomialFactor(
  node: IRNode,
  k: IRNode
): { logFactor: IRNode; polyDeg: number } | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logFactor: IRNode | undefined;
  let polyDegSum = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      // Only one Log(diverging) factor allowed; bail on a second.
      if (logFactor !== undefined) return undefined;
      logFactor = arg;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg === undefined) return undefined; // Neither Log(diverging) nor polynomial.
    polyDegSum += deg;
  }
  if (logFactor === undefined) return undefined; // No Log factor found.
  return { logFactor, polyDeg: polyDegSum };
}

/**
 * Phase 55 (TypeScript port): Return true when ``node`` is a ``Mul`` with
 * exactly one ``Log(diverging)`` factor and all remaining factors bounded
 * in ``k``.
 *
 * The bounded part is uniformly bounded by some constant ``C`` and
 * ``log(h(k))`` grows sub-polynomially, so their product is dominated by
 * any polynomial or faster-growing denominator.  This is the
 * bounded-times-log complement of Phase 52 (bounded × polynomial) and
 * Phase 54 (log × polynomial).
 *
 * Requirements:
 *   - ``node = Mul(...)`` — a bare ``Log(h)`` numerator goes via Phase 50.
 *   - Exactly one factor passes ``isLogOfDivergingInK``.
 *   - All remaining factors pass ``isBoundedInK``.
 *   - Any factor that is neither → return false.
 */
function isBoundedTimesLogInK(node: IRNode, k: IRNode): boolean {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return false;
  let logCount = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      continue;
    }
    if (isBoundedInK(arg, k)) {
      continue;
    }
    // Factor is neither Log(diverging) nor bounded — unrecognised.
    return false;
  }
  return logCount === 1;
}

/**
 * Phase 56 (TypeScript port): Return the ``Sqrt`` inner half-degree
 * when ``node`` is a ``Mul`` with exactly one
 * ``Sqrt(positive-leading polynomial)`` factor and all remaining
 * factors bounded in ``k``; ``undefined`` otherwise.
 *
 * Mirror of :func:`isBoundedTimesLogInK` but for sqrt instead of log.
 * Returns the half-degree directly (Phase 51's
 * ``sqrtEffectiveHalfDegree`` already returns the half value in TS),
 * so the caller compares ``denDeg > sqrtHalfDeg`` directly.
 *
 * Algorithm:
 *   1. Require ``node = Mul(...)``.
 *   2. For each factor:
 *      - ``Sqrt(positive-leading polynomial)`` → record its half-deg;
 *        refuse if a second sqrt appears.
 *      - ``bounded`` → accept.
 *      - otherwise → return undefined.
 *   3. Require exactly one sqrt factor.
 */
function boundedTimesSqrtHalfDegree(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  for (const arg of node.args) {
    const deg = sqrtEffectiveHalfDegree(arg, k);
    if (deg !== undefined) {
      if (sqrtHalfDeg !== undefined) {
        // Two Sqrt factors — refuse (conservative, would need
        // combined growth-rate logic).
        return undefined;
      }
      sqrtHalfDeg = deg;
      continue;
    }
    if (isBoundedInK(arg, k)) {
      continue;
    }
    // Neither Sqrt(positive-poly) nor bounded → unrecognised.
    return undefined;
  }
  return sqrtHalfDeg;
}

/**
 * Phase 57 (TypeScript port): Return the ``Sqrt`` inner half-degree when
 * ``node`` is a ``Mul`` with **exactly one** ``Log(diverging)`` factor AND
 * **exactly one** ``Sqrt(positive-leading polynomial)`` factor, plus any
 * number of bounded factors; ``undefined`` otherwise.
 *
 * Combines sub-polynomial ``Log`` growth with half-polynomial ``Sqrt``
 * growth.  Effective growth ``log(k)·k^{deg(P)/2}`` is strictly dominated
 * by ``k^{deg(P)/2+ε}`` for any ``ε > 0`` since ``log(k) = o(k^ε)``.
 * Caller compares ``denDeg > sqrtHalfDeg`` directly (same convention as
 * Phase 56's ``boundedTimesSqrtHalfDegree``).
 *
 * Requires **both** Log and Sqrt — one-only patterns fall through to
 * Phase 55 (bounded × Log) or Phase 56 (bounded × Sqrt).  Two-of-either
 * is refused (conservative; combined growth-rate logic would be needed).
 *
 * Algorithm:
 *   1. Require ``node = Mul(...)``.
 *   2. For each factor:
 *      - ``Log(diverging)`` → count; refuse if count > 1.
 *      - ``Sqrt(positive-poly)`` → record half-degree; refuse if second one.
 *      - ``bounded`` → accept.
 *      - otherwise → return undefined.
 *   3. Require exactly one Log AND exactly one Sqrt.
 */
function boundedLogSqrtHalfDegree(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let sqrtHalfDeg: number | undefined;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 1) {
        // Two or more Log factors — refuse (would need combined rate logic).
        return undefined;
      }
      continue;
    }
    const deg = sqrtEffectiveHalfDegree(arg, k);
    if (deg !== undefined) {
      if (sqrtHalfDeg !== undefined) {
        // Two Sqrt factors — refuse (conservative).
        return undefined;
      }
      sqrtHalfDeg = deg;
      continue;
    }
    if (isBoundedInK(arg, k)) {
      continue;
    }
    // Neither Log(diverging) nor Sqrt(positive-poly) nor bounded — refuse.
    return undefined;
  }
  if (logCount !== 1 || sqrtHalfDeg === undefined) {
    return undefined;
  }
  return sqrtHalfDeg;
}

/**
 * Phase 58 (TypeScript port): Return the total polynomial degree when
 * ``node`` is a ``Mul`` with exactly one ``Log(diverging)`` factor, any
 * polynomial factors, and any number of bounded (non-polynomial) factors;
 * ``undefined`` otherwise.
 *
 * Fills the gap between:
 * - **Phase 54** — ``Mul(Log, polynomial_only)``; refuses bounded factors.
 * - **Phase 55** — ``Mul(bounded, Log)``; refuses polynomial factors.
 * - **Phase 57** — ``Mul(bounded, Log, Sqrt)``; the Sqrt specialisation.
 *
 * Effective growth ``log(k)·k^m = o(k^{m+ε})``.  Caller compares
 * ``denDeg > polyDeg`` (strict).  Sqrt factors are refused here and
 * handled by Phase 57.
 *
 * Algorithm:
 *   1. Require ``node = Mul(...)``.
 *   2. For each factor:
 *      - ``Log(diverging)`` → count; refuse if count > 1.
 *      - polynomial → add its degree to ``polyDeg``.
 *      - ``bounded`` (non-polynomial, non-Sqrt) → accept silently.
 *      - Sqrt or unrecognised → return undefined.
 *   3. Require exactly one Log.
 *   4. Return ``polyDeg``.
 */
function boundedLogPolyDegree(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 1) {
        // Two or more Log factors — refuse.
        return undefined;
      }
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) {
      polyDeg += deg;
      continue;
    }
    if (isBoundedInK(arg, k)) {
      // Bounded but non-polynomial (e.g. Sin, Cos) — accept.
      continue;
    }
    // Sqrt or unrecognised factor — bail (Sqrt is handled by Phase 57).
    return undefined;
  }
  if (logCount !== 1) {
    return undefined;
  }
  return polyDeg;
}

/**
 * Return ``sqrtHalfDeg + polyDeg`` when ``node`` is a ``Mul`` with
 * exactly one ``Sqrt(positive-leading polynomial P)`` factor, any polynomial
 * factors (total degree ``polyDeg``), and any number of bounded factors;
 * ``undefined`` otherwise.
 *
 * Phase 59 — Bounded × Sqrt(P) × polynomial numerator.
 *
 * Fills the gap between:
 *   - Phase 53: ``Mul(Sqrt, polynomial_only)`` — refuses bounded factors.
 *   - Phase 56: ``Mul(bounded, Sqrt)`` — refuses polynomial factors.
 *
 * Effective growth: ``C·k^{deg(P)/2 + polyDeg}``.
 * TypeScript uses actual half-degree (unlike Python/Rust which use ×2 to stay
 * integer-exact).  Caller checks ``denDeg > sqrtHalfDeg + polyDeg`` directly.
 *
 * Log factors are explicitly refused — that combination is handled by
 * Phase 57 (bounded × Log × Sqrt).
 *
 * Algorithm:
 *   1. Require ``node = Mul(...)``.
 *   2. For each factor:
 *      - ``Sqrt(positive-leading polynomial)`` → record half-degree via
 *        ``sqrtEffectiveHalfDegree``; refuse second Sqrt.
 *      - ``Log(diverging)`` → return undefined (Phase 57 territory).
 *      - polynomial → add its integer degree to ``polyDeg``.
 *      - ``bounded`` (non-polynomial, non-Sqrt, non-Log) → accept silently.
 *      - Unrecognised → return undefined.
 *   3. Require exactly one Sqrt.
 *   4. Return ``sqrtHalfDeg + polyDeg``.
 */
function boundedSqrtPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined = undefined;
  let polyDeg = 0;
  for (const arg of node.args) {
    // Sqrt(positive-leading polynomial) factor?
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      if (sqrtHalfDeg !== undefined) {
        // Two Sqrt factors — refuse.
        return undefined;
      }
      sqrtHalfDeg = halfDeg;
      continue;
    }
    // Log factor — refuse (belongs to Phase 57 territory).
    if (isLogOfDivergingInK(arg, k)) {
      return undefined;
    }
    // Polynomial factor?
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) {
      polyDeg += deg;
      continue;
    }
    // Bounded (non-polynomial, non-Sqrt, non-Log)?
    if (isBoundedInK(arg, k)) {
      continue;
    }
    // Unrecognised factor — bail.
    return undefined;
  }
  if (sqrtHalfDeg === undefined) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 60 (TypeScript port): Return the effective degree when ``node`` is a
 * ``Mul`` with **exactly one** ``Log(diverging)`` factor, **exactly one**
 * ``Sqrt(positive-leading polynomial P)``, any polynomial factors (total
 * degree ``m``), and any number of bounded factors; ``undefined`` otherwise.
 *
 * Closes the gap left by Phase 57 (``Mul(bounded, Log, Sqrt)``; refuses
 * polynomial factors).
 *
 * Effective growth: ``log(k) · k^{deg(P)/2 + m}`` — log is sub-polynomial so
 * the dominant term is the Sqrt×poly part.  Effective degree:
 *   ``sqrtHalfDeg + polyDeg``
 *
 * Caller compares ``denDeg > sqrtHalfDeg + polyDeg`` (strict), matching the
 * TypeScript convention (actual half-degrees, no ×2).
 *
 * Algorithm:
 *   1. Require ``node = Mul(...)``.
 *   2. For each factor:
 *      - ``Log(diverging)`` → count; refuse if count > 1.
 *      - ``Sqrt(positive-poly)`` → record half-degree; refuse if second one.
 *      - polynomial → add degree to ``polyDeg``.
 *      - ``bounded`` → accept silently.
 *      - Unrecognised → return undefined.
 *   3. Require exactly one Log AND exactly one Sqrt.
 *   4. Return ``sqrtHalfDeg + polyDeg``.
 */
function boundedLogSqrtPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let sqrtHalfDeg: number | undefined = undefined;
  let polyDeg = 0;
  for (const arg of node.args) {
    // Log(diverging) factor?
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 1) {
        // Two or more Log factors — refuse.
        return undefined;
      }
      continue;
    }
    // Sqrt(positive-poly) factor?
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      if (sqrtHalfDeg !== undefined) {
        // Two Sqrt factors — refuse (conservative).
        return undefined;
      }
      sqrtHalfDeg = halfDeg;
      continue;
    }
    // Polynomial factor?
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) {
      polyDeg += deg;
      continue;
    }
    // Bounded (non-polynomial, non-Sqrt, non-Log)?
    if (isBoundedInK(arg, k)) {
      continue;
    }
    // Unrecognised factor — bail.
    return undefined;
  }
  if (logCount !== 1 || sqrtHalfDeg === undefined) {
    return undefined;
  }
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 61 (TypeScript port): Return the effective degree when ``node`` is a
 * ``Mul`` with **exactly two** ``Sqrt(positive-leading polynomial)`` factors,
 * any polynomial factors (total degree ``m``), and any number of bounded
 * factors; ``undefined`` otherwise.
 *
 * Closes the gap where Phases 51, 53, 56, 59 each require exactly one Sqrt
 * and hard-reject a second.
 *
 * Effective growth: ``k^{deg(P1)/2 + deg(P2)/2 + m}``.
 * TypeScript convention: compare ``denDeg > sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg``
 * (actual half-degrees, no ×2).
 *
 * ``Log`` factors are refused (belong to future Log×two-Sqrt phases).
 *
 * Algorithm:
 *   1. Require ``node = Mul(...)``.
 *   2. For each factor:
 *      - ``Sqrt(positive-poly)`` → accumulate half-degrees; refuse if third one.
 *      - ``Log(diverging)`` → refuse immediately.
 *      - polynomial → add degree to ``polyDeg``.
 *      - ``bounded`` → accept silently.
 *      - Unrecognised → return undefined.
 *   3. Require exactly two Sqrt factors.
 *   4. Return ``sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg``.
 */
function twoSqrtPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let polyDeg = 0;
  for (const arg of node.args) {
    // Sqrt(positive-poly) factor?
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      sqrtHalfDegs.push(halfDeg);
      if (sqrtHalfDegs.length > 2) {
        // Three or more Sqrt factors — refuse (conservative).
        return undefined;
      }
      continue;
    }
    // Log(diverging) factor — refuse (future Log×two-Sqrt phase territory).
    if (isLogOfDivergingInK(arg, k)) {
      return undefined;
    }
    // Polynomial factor?
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) {
      polyDeg += deg;
      continue;
    }
    // Bounded (non-polynomial, non-Sqrt, non-Log)?
    if (isBoundedInK(arg, k)) {
      continue;
    }
    // Unrecognised factor — bail.
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 62 — Two-Log × polynomial numerator.
 *
 * Returns the effective polynomial degree when `node` is a `Mul` with
 * **exactly two** `Log(diverging-in-k)` factors, any polynomial factors,
 * and any bounded factors; `undefined` otherwise.
 *
 * `log(k)² · k^m` grows sub-polynomially, so the effective degree equals
 * `poly_deg`. Caller checks `denDeg > twoLogPolyEffectiveDeg(num, k)`.
 *
 * Sqrt factors are refused (belong to the two-Sqrt / log-Sqrt family).
 */
function twoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 2) return undefined;
      continue;
    }
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt → refuse
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 2) return undefined;
  return polyDeg;
}

/**
 * Phase 63 — Two-Sqrt × Log × polynomial numerator.
 *
 * Returns the effective degree when `node` is a `Mul` with exactly two
 * Sqrt factors, exactly one Log(diverging) factor, any polynomial factors,
 * and any bounded factors; `undefined` otherwise.
 *
 * Log is sub-polynomial; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      sqrtHalfDegs.push(halfDeg);
      if (sqrtHalfDegs.length > 2) return undefined;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 1) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 1) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 64 — Two-Log × Sqrt × polynomial numerator.
 *
 * Returns the effective degree when `node` is a `Mul` with exactly two
 * Log(diverging) factors, exactly one Sqrt factor, any polynomial factors,
 * and any bounded factors; `undefined` otherwise.
 *
 * log² is sub-polynomial; effective degree = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > twoLogSqrtPolyEffectiveDeg(num, k)`.
 */
function twoLogSqrtPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let sqrtHalfDeg: number | undefined = undefined;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 2) return undefined;
      continue;
    }
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // second Sqrt → refuse
      sqrtHalfDeg = halfDeg;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 2 || sqrtHalfDeg === undefined) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 65 — Two-Sqrt × Two-Log × polynomial numerator.
 *
 * Returns sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg when `node` is a `Mul`
 * with exactly two Sqrt factors, exactly two Log(diverging) factors,
 * any polynomial factors, and any bounded factors; `undefined` otherwise.
 *
 * log² is sub-polynomial; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtTwoLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      sqrtHalfDegs.push(halfDeg);
      if (sqrtHalfDegs.length > 2) return undefined;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 2) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 2) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 66 — Three-Sqrt × polynomial numerator.
 *
 * Returns sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg when `node`
 * is a `Mul` with exactly three Sqrt factors, any polynomial factors, and any
 * bounded factors; `undefined` otherwise.
 *
 * Log factors are rejected immediately (use Phase 63/64/65 for sqrt+log combos).
 * effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtPolyEffectiveDeg(num, k)`.
 */
function threeSqrtPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let polyDeg = 0;
  for (const arg of node.args) {
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      sqrtHalfDegs.push(halfDeg);
      if (sqrtHalfDegs.length > 3) return undefined;
      continue;
    }
    // Log factors not handled here — bail so Phase 63/64/65 can catch them.
    if (isLogOfDivergingInK(arg, k)) return undefined;
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 67 — Three-Log × polynomial numerator.
 *
 * Returns the effective polynomial degree when `node` is a `Mul` with
 * **exactly three** `Log(diverging-in-k)` factors, any polynomial factors,
 * and any bounded factors; `undefined` otherwise.
 *
 * `log(k)³ · k^m` grows sub-polynomially, so the effective degree equals
 * `poly_deg`. Caller checks `denDeg > threeLogPolyEffectiveDeg(num, k)`.
 *
 * Sqrt factors are refused (belong to the sqrt/log family).
 */
function threeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 3) return undefined;
      continue;
    }
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt → refuse
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 3) return undefined;
  return polyDeg;
}

/**
 * Phase 68 — Three-Sqrt × Log × polynomial numerator.
 *
 * Returns `sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg` when `node`
 * is a `Mul` with exactly three `Sqrt` factors, exactly one `Log(diverging)`
 * factor, any polynomial factors, and any bounded factors; `undefined` otherwise.
 *
 * The Log factor is sub-polynomial (`o(k^ε)`), so it contributes 0 to effective
 * degree. effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      sqrtHalfDegs.push(halfDeg);
      if (sqrtHalfDegs.length > 3) return undefined;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 1) return undefined; // more than one Log — refuse
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 1) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 69 — One-Sqrt × Three-Log × polynomial numerator.
 *
 * Returns `sqrtHalfDeg + polyDeg` when `node` is a `Mul` with exactly one
 * `Sqrt` factor, exactly three `Log(diverging)` factors, any polynomial
 * factors, and any bounded factors; `undefined` otherwise.
 *
 * `log³(k)` is sub-polynomial (`o(k^ε)`), so it contributes 0 to effective
 * degree. effective degree = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtThreeLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined = undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // second Sqrt — refuse
      sqrtHalfDeg = halfDeg;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 3) return undefined; // more than three Logs — refuse
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 3) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 70 — Three-Sqrt × Two-Log × polynomial numerator.
 *
 * Returns `sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg` when
 * `node` is a `Mul` with exactly three `Sqrt` factors, exactly two
 * `Log(diverging)` factors, any polynomial factors, and any bounded
 * factors; `undefined` otherwise.
 *
 * `log²(k)` is sub-polynomial (`o(k^ε)`), so it contributes 0 to effective
 * degree. effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtTwoLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      sqrtHalfDegs.push(halfDeg);
      if (sqrtHalfDegs.length > 3) return undefined; // more than three Sqrts — refuse
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 2) return undefined; // more than two Logs — refuse
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 2) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 71 — Two-Sqrt × Three-Log × polynomial numerator.
 *
 * Returns `sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg` when `node` is a `Mul`
 * with exactly two `Sqrt` factors, exactly three `Log(diverging)` factors,
 * any polynomial factors, and any bounded factors; `undefined` otherwise.
 *
 * `log³(k)` is sub-polynomial (`o(k^ε)`), so it contributes 0 to effective
 * degree. effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtThreeLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      sqrtHalfDegs.push(halfDeg);
      if (sqrtHalfDegs.length > 2) return undefined; // more than two Sqrts — refuse
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 3) return undefined; // more than three Logs — refuse
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 3) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 72 — Three-Sqrt × Three-Log × polynomial numerator.
 *
 * Returns `sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg` when
 * `node` is a `Mul` with exactly three `Sqrt` factors, exactly three
 * `Log(diverging)` factors, any polynomial factors, and any bounded
 * factors; `undefined` otherwise.
 *
 * `log³(k)` is sub-polynomial (`o(k^ε)`), so it contributes 0 to effective
 * degree. effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtThreeLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const halfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (halfDeg !== undefined) {
      sqrtHalfDegs.push(halfDeg);
      if (sqrtHalfDegs.length > 3) return undefined; // more than three Sqrts — refuse
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 3) return undefined; // more than three Logs — refuse
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 3) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 73 — Four-Log × polynomial numerator.
 *
 * Returns `polyDeg` when `node` is a `Mul` with exactly four
 * `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
 * factors; `undefined` otherwise.
 *
 * `log⁴(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective
 * polynomial degree.  effective degree = polyDeg.
 * Sqrt factors are refused — use Sqrt × log phases for mixed forms.
 * Caller checks `denDeg > fourLogPolyEffectiveDeg(num, k)`.
 */
function fourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 4) return undefined;
      continue;
    }
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt → refuse
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 4) return undefined;
  return polyDeg;
}

/**
 * Phase 74 — One-Sqrt × Four-Log × polynomial numerator.
 *
 * Returns `sqrtHalfDeg + polyDeg` when `node` is a `Mul` with exactly one
 * `Sqrt` factor, exactly four `Log(diverging-in-k)` factors, any polynomial
 * factors, and any bounded factors; `undefined` otherwise.
 *
 * `log⁴(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective
 * polynomial degree.  effective degree = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtFourLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // second Sqrt — refuse
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 4) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 4) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 78 — One-Sqrt × Five-Log × polynomial numerator.
 *
 * Returns `sqrtHalfDeg + polyDeg` when `node` is a `Mul` with exactly one
 * `Sqrt(positive-leading polynomial)` factor, exactly five `Log(diverging-in-k)`
 * factors, any polynomial factors, and any bounded factors; `undefined` otherwise.
 *
 * `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective degree.
 * TypeScript stores the actual half-degree (not ×2), so:
 *   `effectiveDeg = sqrtHalfDeg + polyDeg`.
 * Caller checks `denDeg > oneSqrtFiveLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // second Sqrt — refuse
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 5) return undefined; // six or more Logs — refuse
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 5) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 75 — Two-Sqrt × Four-Log × polynomial numerator.
 *
 * Returns `sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg` when `node` is a `Mul`
 * with exactly two `Sqrt` factors, exactly four `Log(diverging-in-k)` factors,
 * any polynomial factors, and any bounded factors; `undefined` otherwise.
 *
 * `log⁴(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective
 * polynomial degree.  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtFourLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined; // third Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 4) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 4) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Return `sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg` when `node`
 * is a `Mul` with **exactly three** `Sqrt(positive-leading polynomial)` factors,
 * **exactly four** `Log(diverging-in-k)` factors, any polynomial factors,
 * and any bounded factors; `undefined` otherwise.
 *
 * Phase 76 — Three-Sqrt × Four-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁴ · k^m ≈ k^{(a+b+c)/2} · log⁴(k) · k^m`.
 * `log⁴(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective
 * polynomial degree.  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtFourLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined; // fourth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 4) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 4) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 77 — Five-Log × polynomial numerator.
 *
 * Returns `polyDeg` when `node` is a `Mul` with **exactly five**
 * `Log(diverging-in-k)` factors, any polynomial factors, and any bounded
 * factors; `undefined` otherwise.
 *
 * `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective degree.
 * TypeScript stores the actual half-degree (not ×2), so:
 *   `effectiveDeg = polyDeg`.
 * Caller checks `denDeg > fiveLogPolyEffectiveDeg(num, k)`.
 *
 * Sqrt factors are explicitly refused so that Sqrt-bearing phases (73–76, 78+)
 * are not shadowed by this function.
 */
function fiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 5) return undefined; // six or more Logs — not this phase
      continue;
    }
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 5) return undefined;
  return polyDeg;
}

/**
 * Phase 79 — Two-Sqrt × Five-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)⁵ · k^m ≈ k^{(a+b)/2} · log⁵(k) · k^m`.
 * `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective
 * polynomial degree.  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtFiveLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined; // third Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 5) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 5) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 80 — Three-Sqrt × Five-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁵ · k^m ≈ k^{(a+b+c)/2} · log⁵(k) · k^m`.
 * `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective
 * polynomial degree.  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtFiveLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined; // fourth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 5) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 5) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 81 — Four-Sqrt × Five-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)⁵ · k^m
 * ≈ k^{(a+b+c+d)/2} · log⁵(k) · k^m`.
 * `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective
 * polynomial degree.  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + sqrtHalfDeg4 + polyDeg.
 * Caller checks `denDeg > fourSqrtFiveLogPolyEffectiveDeg(num, k)`.
 */
function fourSqrtFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined; // fifth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 5) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 5) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/**
 * Phase 82 — Five-Sqrt × Five-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a₁)·…·sqrt(k^a₅)·log(k)⁵·k^m ≈ k^{(a₁+…+a₅)/2}·log⁵(k)·k^m`.
 * `log⁵(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective
 * polynomial degree.  effective degree = sqrtHalfDeg1+sqrtHalfDeg2+sqrtHalfDeg3+sqrtHalfDeg4+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtFiveLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined; // sixth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 5) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 5) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/**
 * Phase 83 — Six-Log × polynomial numerator.
 *
 * Effective growth: `log(k)⁶ · k^m`. `log⁶(k)` is sub-polynomial (`o(k^ε)`),
 * contributing 0 to effective polynomial degree.
 * effective degree = polyDeg.
 * Caller checks `denDeg > sixLogPolyEffectiveDeg(num, k)`.
 */
function sixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 6) return undefined; // seven or more Logs — not this phase
      continue;
    }
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 6) return undefined;
  return polyDeg;
}

/**
 * Phase 89 — Seven-Log × polynomial numerator.
 *
 * Effective growth: `log(k)⁷ · k^m`. `log⁷(k)` is sub-polynomial (`o(k^ε)`),
 * contributing 0 to effective polynomial degree.
 * effective degree = polyDeg.
 * Sqrt factors are explicitly refused so Sqrt-bearing phases handle them.
 * Caller checks `denDeg > sevenLogPolyEffectiveDeg(num, k)`.
 */
function sevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 7) return undefined; // eight or more Logs — not this phase
      continue;
    }
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 7) return undefined;
  return polyDeg;
}

/**
 * Phase 95 — Eight-Log × polynomial numerator (zero Sqrt).
 *
 * Effective growth: `log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial (`o(k^ε)`),
 * contributing 0 to effective polynomial degree.
 * effective degree = polyDeg.
 * Sqrt factors are explicitly refused so Sqrt-bearing phases handle them.
 * Caller checks `denDeg > eightLogPolyEffectiveDeg(num, k)`.
 */
function eightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 8) return undefined; // nine or more Logs — not this phase
      continue;
    }
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 8) return undefined;
  return polyDeg;
}

function elevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 11) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 11) return undefined;
  return polyDeg;
}

function oneSqrtElevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 11) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 11) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtElevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 11) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 11) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtElevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 11) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 11) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtElevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 11) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 11) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtElevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 11) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 11) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

function twelveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 12) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 12) return undefined;
  return polyDeg;
}

function oneSqrtTwelveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 12) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 12) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtTwelveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 12) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 12) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtTwelveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 12) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 12) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtTwelveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 12) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 12) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtTwelveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 12) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 12) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

function tenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 10) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 10) return undefined;
  return polyDeg;
}

function oneSqrtTenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 10) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 10) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtTenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 10) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 10) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtTenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 10) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 10) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtTenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 10) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 10) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtTenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 10) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 10) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/**
 * Phase 101 — Nine-Log × polynomial numerator.
 *
 * Effective growth: `log(k)⁹ · k^m`. `log⁹(k)` is sub-polynomial (`o(k^ε)`),
 * contributing 0 to effective polynomial degree.
 * effective degree = polyDeg.
 * Caller checks `denDeg > nineLogPolyEffectiveDeg(num, k)`.
 */
function nineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 9) return undefined; // ten or more Logs — not this phase
      continue;
    }
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 9) return undefined;
  return polyDeg;
}

/**
 * Phase 102 — One-Sqrt × Nine-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · log(k)⁹ · k^m`. `log⁹(k)` is sub-polynomial (`o(k^ε)`),
 * contributing 0 to effective polynomial degree.
 * effective degree = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtNineLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // second Sqrt — refuse
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 9) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 9) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 103 — Two-Sqrt × Nine-Log × polynomial numerator.
 *
 * effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtNineLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined; // third Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 9) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 9) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 9) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 9) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 9) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 9) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 9) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 9) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/**
 * Phase 96 — One-Sqrt × Eight-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial (`o(k^ε)`),
 * contributing 0 to effective polynomial degree.
 * effective degree = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtEightLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // second Sqrt — refuse
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 8) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 8) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 97 — Two-Sqrt × Eight-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial,
 * contributing 0.  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 */
function twoSqrtEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined; // third Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 8) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 8) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 100 — Five-Sqrt × Eight-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a)·sqrt(k^b)·sqrt(k^c)·sqrt(k^d)·sqrt(k^e)·log(k)⁸·k^m`.
 * `log⁸(k)` is sub-polynomial, contributing 0.
 * effective degree = sum(sqrtHalfDegs) + polyDeg. Completes the Eight-Log family.
 */
function fiveSqrtEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined; // sixth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 8) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 8) return undefined;
  return sqrtHalfDegs.reduce((a, b) => a + b, 0) + polyDeg;
}

/**
 * Phase 99 — Four-Sqrt × Eight-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial,
 * contributing 0.  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + sqrtHalfDeg4 + polyDeg.
 */
function fourSqrtEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined; // fifth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 8) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 8) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/**
 * Phase 98 — Three-Sqrt × Eight-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁸ · k^m`. `log⁸(k)` is sub-polynomial,
 * contributing 0.  effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
 */
function threeSqrtEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined; // fourth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 8) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 8) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 90 — One-Sqrt × Seven-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · log(k)⁷ · k^m`. `log⁷(k)` is sub-polynomial (`o(k^ε)`),
 * contributing 0 to effective polynomial degree.
 * effective degree = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtSevenLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtSevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // second Sqrt — refuse
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 7) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 7) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 91 — Two-Sqrt × Seven-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)⁷ · k^m ≈ k^{(a+b)/2} · log⁷(k) · k^m`.
 * `log⁷(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective polynomial degree.
 * effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtSevenLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtSevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined; // third Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 7) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 7) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 92 — Three-Sqrt × Seven-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁷ · k^m ≈ k^{(a+b+c)/2} · log⁷(k) · k^m`.
 * `log⁷(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective polynomial degree.
 * effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtSevenLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtSevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined; // fourth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 7) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 7) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 93 — Four-Sqrt × Seven-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a)·sqrt(k^b)·sqrt(k^c)·sqrt(k^d)·log(k)⁷·k^m`.
 * `log⁷(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
 * effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + sqrtHalfDeg4 + polyDeg.
 */
function fourSqrtSevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined; // fifth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 7) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 7) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/**
 * Phase 94 — Five-Sqrt × Seven-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a)·sqrt(k^b)·sqrt(k^c)·sqrt(k^d)·sqrt(k^e)·log(k)⁷·k^m`.
 * `log⁷(k)` is sub-polynomial (`o(k^ε)`), contributing 0.
 * effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + sqrtHalfDeg4 + sqrtHalfDeg5 + polyDeg.
 */
function fiveSqrtSevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined; // sixth Sqrt — refuse
      sqrtHalfDegs.push(hd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 7) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 7) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/**
 * Phase 84 — One-Sqrt × Six-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · log(k)⁶ · k^m`. `log⁶(k)` is sub-polynomial (`o(k^ε)`),
 * contributing 0 to effective polynomial degree.
 * effective degree = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtSixLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtSixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // second Sqrt — refuse
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 6) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 6) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function gVanishesAtInfinity(g: IRNode, k: IRNode): boolean {
  if (g.kind !== "apply" || !equals(g.head, DIV) || g.args.length !== 2) {
    return false;
  }
  const [num, den] = g.args;
  // Phase 41/43 fast path: constant numerator + diverging denominator
  // (positive-degree polynomial OR exp / b^k transcendental).
  if (isConstantIn(num, k)) {
    return hDivergesAtInfinity(den, k);
  }
  // Phase 49: bounded numerator + diverging denominator.  Covers
  // shapes like ``sin(k)/k²`` and ``cos(k)·sin(k)/k³``.
  if (isBoundedInK(num, k) && hDivergesAtInfinity(den, k)) {
    return true;
  }
  // Phase 50: Log(diverging) numerator + diverging denominator.
  // log/poly → 0 always (log grows slower than any positive power).
  if (isLogOfDivergingInK(num, k) && hDivergesAtInfinity(den, k)) {
    return true;
  }
  // Phase 51: Sqrt(positive-poly) numerator + polynomial denominator
  // with deg(den) > deg(P)/2.
  const sqrtHalfDeg = sqrtEffectiveHalfDegree(num, k);
  if (sqrtHalfDeg !== undefined) {
    const denDegSqrt = polynomialDegreeInK(den, k);
    if (denDegSqrt !== undefined && denDegSqrt > sqrtHalfDeg) {
      return true;
    }
  }
  // Phase 52: Mul(bounded, polynomial) numerator pattern.  When the
  // numerator factors as bounded × polynomial with positive poly degree,
  // the quotient vanishes iff den_deg > poly_deg.  Catches shapes like
  // sin(k)·k/k³ that Phase 49 misses (Mul isn't wholly bounded) and
  // Phase 42 refuses (sin is not polynomial).
  const bpResult = splitBoundedPolynomialFactor(num, k);
  if (bpResult !== undefined) {
    const denDegBp = polynomialDegreeInK(den, k);
    if (denDegBp !== undefined && denDegBp > bpResult.polyDeg) {
      return true;
    }
  }
  // Phase 53: Mul(Sqrt(P), polynomial_factors) numerator pattern.
  // The effective growth rate is deg(P)/2 + deg(Q).  Vanishes when
  // deg(den) > deg(P)/2 + deg(Q).  Handled by sqrtPolyNumeratorEffectiveDegree
  // which requires exactly one Sqrt factor and all others polynomial.
  const sqrtPolyEff = sqrtPolyNumeratorEffectiveDegree(num, k);
  if (sqrtPolyEff !== undefined) {
    const denDegSp = polynomialDegreeInK(den, k);
    if (denDegSp !== undefined && denDegSp > sqrtPolyEff) {
      return true;
    }
  }
  // Phase 54: Mul(Log(diverging), polynomial_factors) numerator pattern.
  // log(h(k)) grows sub-polynomially so the effective growth degree is just
  // deg(poly_part).  Vanishes when den_deg > poly_deg (strictly).
  // Equal degrees are refused: log(k)*constant diverges to ±∞.
  const logPolyResult = splitLogPolynomialFactor(num, k);
  if (logPolyResult !== undefined) {
    const denDegLp = polynomialDegreeInK(den, k);
    if (denDegLp !== undefined && denDegLp > logPolyResult.polyDeg) {
      return true;
    }
  }
  // Phase 55: Mul(bounded, Log(diverging)) numerator + diverging denominator.
  // bounded × log(h(k)) grows sub-polynomially (log dominates bounded part,
  // but log itself is dominated by any polynomial or faster-growing denominator).
  if (isBoundedTimesLogInK(num, k) && hDivergesAtInfinity(den, k)) {
    return true;
  }
  // Phase 56: Mul(bounded, Sqrt(positive-poly)) numerator pattern.
  // Effective growth degree is deg(P)/2.  Vanishes when:
  //   - denominator is polynomial with deg(den) > deg(P)/2, OR
  //   - denominator is non-polynomial diverging (Exp / Pow / Log×poly)
  //     which dominates any sub-polynomial sqrt growth automatically.
  const sqrtBoundedHalfDeg = boundedTimesSqrtHalfDegree(num, k);
  if (sqrtBoundedHalfDeg !== undefined) {
    const denDegBs = polynomialDegreeInK(den, k);
    if (denDegBs !== undefined) {
      if (denDegBs > sqrtBoundedHalfDeg) {
        return true;
      }
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 57: Mul(bounded, Log(diverging), Sqrt(positive-poly)) numerator.
  // Effective growth ``log(k)·k^{deg(P)/2}`` is dominated by any
  // ``k^{deg(P)/2+ε}``, so the quotient vanishes when:
  //   - polynomial denominator with ``denDeg > deg(P)/2``, OR
  //   - non-polynomial diverging denominator (Exp / Pow / Log×poly).
  // Requires both Log and Sqrt; one-only falls through to Phase 55 / 56.
  const blsHalfDeg = boundedLogSqrtHalfDegree(num, k);
  if (blsHalfDeg !== undefined) {
    const denDegBls = polynomialDegreeInK(den, k);
    if (denDegBls !== undefined) {
      if (denDegBls > blsHalfDeg) {
        return true;
      }
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 58: Mul(bounded, Log(diverging), polynomial) numerator.
  // Effective growth ``log(k)·k^m = o(k^{m+ε})``.  Vanishes when:
  //   - polynomial denominator with ``denDeg > polyDeg`` (strict), OR
  //   - non-polynomial diverging denominator (Exp / Pow / Log×poly).
  // Fills the gap between Phase 54 (Log × poly, refuses bounded) and
  // Phase 55 (bounded × Log, refuses poly).  Sqrt is refused here →
  // Phase 57 handles that case.
  const blpDeg = boundedLogPolyDegree(num, k);
  if (blpDeg !== undefined) {
    const denDegBlp = polynomialDegreeInK(den, k);
    if (denDegBlp !== undefined) {
      if (denDegBlp > blpDeg) {
        return true;
      }
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 59: Mul(bounded, Sqrt(positive-poly), polynomial) numerator.
  // Effective degree: sqrtHalfDeg + polyDeg.  Vanishes when
  // denDeg > sqrtHalfDeg + polyDeg (polynomial) or non-polynomial diverging denom.
  // Fills the gap between Phase 53 (Sqrt×poly, refuses bounded) and
  // Phase 56 (bounded×Sqrt, refuses poly).  Log is refused → Phase 57.
  const bspDeg = boundedSqrtPolyEffectiveDeg(num, k);
  if (bspDeg !== undefined) {
    const denDegBsp = polynomialDegreeInK(den, k);
    if (denDegBsp !== undefined) {
      if (denDegBsp > bspDeg) {
        return true;
      }
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 60: Mul(bounded, Log(diverging), Sqrt(positive-poly), polynomial)
  // numerator.  Closes the gap left by Phase 57 (bounded×Log×Sqrt, refuses
  // polynomial factors).  Effective degree: sqrtHalfDeg + polyDeg.  Vanishes
  // when denDeg > sqrtHalfDeg + polyDeg (polynomial) or non-polynomial
  // diverging denominator.
  const blspDeg = boundedLogSqrtPolyEffectiveDeg(num, k);
  if (blspDeg !== undefined) {
    const denDegBlsp = polynomialDegreeInK(den, k);
    if (denDegBlsp !== undefined) {
      if (denDegBlsp > blspDeg) {
        return true;
      }
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 61: Mul(Sqrt(P1), Sqrt(P2), polynomial..., bounded...) numerator.
  // Extends Phases 53/56/59 to two Sqrt factors.
  // Effective degree: sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  // Vanishes when denDeg > tspDeg (polynomial) or non-polynomial diverging.
  const tspDeg = twoSqrtPolyEffectiveDeg(num, k);
  if (tspDeg !== undefined) {
    const denDegTsp = polynomialDegreeInK(den, k);
    if (denDegTsp !== undefined) {
      if (denDegTsp > tspDeg) {
        return true;
      }
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 62: Mul(Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
  // log²(k) is sub-polynomial; effective degree = poly_deg.
  // Closes when denDeg > twoLogPolyEffectiveDeg or non-polynomial diverging denom.
  const tlpDeg = twoLogPolyEffectiveDeg(num, k);
  if (tlpDeg !== undefined) {
    const denDegTlp = polynomialDegreeInK(den, k);
    if (denDegTlp !== undefined) {
      if (denDegTlp > tlpDeg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 63: Mul(Sqrt(P1), Sqrt(P2), Log(diverging), polynomial..., bounded...) numerator.
  // Two Sqrts + one Log; log is sub-polynomial; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  const tslpDeg = twoSqrtLogPolyEffectiveDeg(num, k);
  if (tslpDeg !== undefined) {
    const denDegTslp = polynomialDegreeInK(den, k);
    if (denDegTslp !== undefined) {
      if (denDegTslp > tslpDeg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 64: Mul(Log(diverging), Log(diverging), Sqrt(P), polynomial..., bounded...) numerator.
  // Two Logs + one Sqrt; log² sub-polynomial; effective degree = sqrtHalfDeg + polyDeg.
  const tlspDeg = twoLogSqrtPolyEffectiveDeg(num, k);
  if (tlspDeg !== undefined) {
    const denDegTlsp = polynomialDegreeInK(den, k);
    if (denDegTlsp !== undefined) {
      if (denDegTlsp > tlspDeg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 65: Mul(Sqrt(P1), Sqrt(P2), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
  // Two Sqrts + two Logs; log² sub-polynomial; effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  const ts2lDeg = twoSqrtTwoLogPolyEffectiveDeg(num, k);
  if (ts2lDeg !== undefined) {
    const denDegTs2l = polynomialDegreeInK(den, k);
    if (denDegTs2l !== undefined) {
      if (denDegTs2l > ts2lDeg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 66: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), polynomial..., bounded...) numerator.
  // Three Sqrt factors; log factors refused (use Phase 63/64/65 for sqrt+log combos).
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  const tsp3Deg = threeSqrtPolyEffectiveDeg(num, k);
  if (tsp3Deg !== undefined) {
    const denDegTsp3 = polynomialDegreeInK(den, k);
    if (denDegTsp3 !== undefined) {
      if (denDegTsp3 > tsp3Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 67: Mul(Log(diverging), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
  // log³(k) is sub-polynomial; effective degree = poly_deg.
  // Closes when denDeg > threeLogPolyEffectiveDeg or non-polynomial diverging denom.
  const tlp3Deg = threeLogPolyEffectiveDeg(num, k);
  if (tlp3Deg !== undefined) {
    const denDegTlp3 = polynomialDegreeInK(den, k);
    if (denDegTlp3 !== undefined) {
      if (denDegTlp3 > tlp3Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 68: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging), polynomial..., bounded...) numerator.
  // Three Sqrt factors + one Log; Log is sub-polynomial — contributes 0 to effective degree.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  // Closes when denDeg > threeSqrtLogPolyEffectiveDeg or non-polynomial diverging denom.
  const ts3lDeg = threeSqrtLogPolyEffectiveDeg(num, k);
  if (ts3lDeg !== undefined) {
    const denDegTs3l = polynomialDegreeInK(den, k);
    if (denDegTs3l !== undefined) {
      if (denDegTs3l > ts3lDeg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 69: Mul(Sqrt(P), Log(diverging), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
  // One Sqrt factor + three Log factors; log³ is sub-polynomial — contributes 0 to effective degree.
  // effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtThreeLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l3Deg = oneSqrtThreeLogPolyEffectiveDeg(num, k);
  if (s1l3Deg !== undefined) {
    const denDegS1l3 = polynomialDegreeInK(den, k);
    if (denDegS1l3 !== undefined) {
      if (denDegS1l3 > s1l3Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 70: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
  // Three Sqrt factors + two Log factors; log² is sub-polynomial — contributes 0 to effective degree.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  // Closes when denDeg > threeSqrtTwoLogPolyEffectiveDeg or non-polynomial diverging denom.
  const ts3l2Deg = threeSqrtTwoLogPolyEffectiveDeg(num, k);
  if (ts3l2Deg !== undefined) {
    const denDegTs3l2 = polynomialDegreeInK(den, k);
    if (denDegTs3l2 !== undefined) {
      if (denDegTs3l2 > ts3l2Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 71: Mul(Sqrt(P1), Sqrt(P2), Log(diverging), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
  // Two Sqrt factors + three Log factors; log³ is sub-polynomial — contributes 0 to effective degree.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  // Closes when denDeg > twoSqrtThreeLogPolyEffectiveDeg or non-polynomial diverging denom.
  const ts2l3Deg = twoSqrtThreeLogPolyEffectiveDeg(num, k);
  if (ts2l3Deg !== undefined) {
    const denDegTs2l3 = polynomialDegreeInK(den, k);
    if (denDegTs2l3 !== undefined) {
      if (denDegTs2l3 > ts2l3Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 72: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging), Log(diverging), Log(diverging), polynomial..., bounded...) numerator.
  // Three Sqrt factors + three Log factors; log³ is sub-polynomial — contributes 0 to effective degree.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  // Closes when denDeg > threeSqrtThreeLogPolyEffectiveDeg or non-polynomial diverging denom.
  const ts3l3Deg = threeSqrtThreeLogPolyEffectiveDeg(num, k);
  if (ts3l3Deg !== undefined) {
    const denDegTs3l3 = polynomialDegreeInK(den, k);
    if (denDegTs3l3 !== undefined) {
      if (denDegTs3l3 > ts3l3Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 73: Mul(Log(diverging)×4, polynomial..., bounded...) numerator.
  // Four Log factors; log⁴ is sub-polynomial — contributes 0 to effective degree.
  // effective degree = polyDeg. Sqrt factors refused.
  // Closes when denDeg > fourLogPolyEffectiveDeg or non-polynomial diverging denom.
  const flp4Deg = fourLogPolyEffectiveDeg(num, k);
  if (flp4Deg !== undefined) {
    const denDegFlp4 = polynomialDegreeInK(den, k);
    if (denDegFlp4 !== undefined) {
      if (denDegFlp4 > flp4Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 74: Mul(Sqrt(P), Log(diverging)×4, polynomial..., bounded...) numerator.
  // One Sqrt + four Log factors; log⁴ sub-polynomial — contributes 0.
  // effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtFourLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l4Deg = oneSqrtFourLogPolyEffectiveDeg(num, k);
  if (s1l4Deg !== undefined) {
    const denDegS1l4 = polynomialDegreeInK(den, k);
    if (denDegS1l4 !== undefined) {
      if (denDegS1l4 > s1l4Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 75: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×4, polynomial..., bounded...) numerator.
  // Two Sqrt + four Log factors; log⁴ sub-polynomial — contributes 0.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  // Closes when denDeg > twoSqrtFourLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l4Deg = twoSqrtFourLogPolyEffectiveDeg(num, k);
  if (s2l4Deg !== undefined) {
    const denDegS2l4 = polynomialDegreeInK(den, k);
    if (denDegS2l4 !== undefined) {
      if (denDegS2l4 > s2l4Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 76: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging)×4, polynomial..., bounded...) numerator.
  // Three Sqrt + four Log factors; log⁴ sub-polynomial — contributes 0.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  // Closes when denDeg > threeSqrtFourLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l4Deg = threeSqrtFourLogPolyEffectiveDeg(num, k);
  if (s3l4Deg !== undefined) {
    const denDegS3l4 = polynomialDegreeInK(den, k);
    if (denDegS3l4 !== undefined) {
      if (denDegS3l4 > s3l4Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 77: Mul(Log(diverging)×5, polynomial..., bounded...) numerator.
  // Five Log factors; no Sqrt; log⁵ sub-polynomial — contributes 0.
  // effective degree = polyDeg.
  // Closes when denDeg > fiveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const fl5Deg = fiveLogPolyEffectiveDeg(num, k);
  if (fl5Deg !== undefined) {
    const denDegFl5 = polynomialDegreeInK(den, k);
    if (denDegFl5 !== undefined) {
      if (denDegFl5 > fl5Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 78: Mul(Sqrt(P), Log(diverging)×5, polynomial..., bounded...) numerator.
  // One Sqrt + five Log factors; log⁵ sub-polynomial — contributes 0.
  // effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtFiveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l5Deg = oneSqrtFiveLogPolyEffectiveDeg(num, k);
  if (s1l5Deg !== undefined) {
    const denDegS1l5 = polynomialDegreeInK(den, k);
    if (denDegS1l5 !== undefined) {
      if (denDegS1l5 > s1l5Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 79: Mul(Sqrt(P1), Sqrt(P2), Log(diverging)×5, polynomial..., bounded...) numerator.
  // Two Sqrt + five Log factors; log⁵ sub-polynomial — contributes 0.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  // Closes when denDeg > twoSqrtFiveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l5Deg = twoSqrtFiveLogPolyEffectiveDeg(num, k);
  if (s2l5Deg !== undefined) {
    const denDegS2l5 = polynomialDegreeInK(den, k);
    if (denDegS2l5 !== undefined) {
      if (denDegS2l5 > s2l5Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 80: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging)×5, polynomial..., bounded...) numerator.
  // Three Sqrt + five Log factors; log⁵ sub-polynomial — contributes 0.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  // Closes when denDeg > threeSqrtFiveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l5Deg = threeSqrtFiveLogPolyEffectiveDeg(num, k);
  if (s3l5Deg !== undefined) {
    const denDegS3l5 = polynomialDegreeInK(den, k);
    if (denDegS3l5 !== undefined) {
      if (denDegS3l5 > s3l5Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 81: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(diverging)×5, polynomial..., bounded...) numerator.
  // Four Sqrt + five Log factors; log⁵ sub-polynomial — contributes 0.
  // effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + sqrtHalfDeg4 + polyDeg.
  // Closes when denDeg > fourSqrtFiveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l5Deg = fourSqrtFiveLogPolyEffectiveDeg(num, k);
  if (s4l5Deg !== undefined) {
    const denDegS4l5 = polynomialDegreeInK(den, k);
    if (denDegS4l5 !== undefined) {
      if (denDegS4l5 > s4l5Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 82: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Sqrt(P5), Log(diverging)×5, polynomial..., bounded...) numerator.
  // Five Sqrt + five Log factors; log⁵ sub-polynomial — contributes 0.
  // effective degree = sqrtHalfDeg1+sqrtHalfDeg2+sqrtHalfDeg3+sqrtHalfDeg4+sqrtHalfDeg5 + polyDeg.
  // Closes when denDeg > fiveSqrtFiveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l5Deg = fiveSqrtFiveLogPolyEffectiveDeg(num, k);
  if (s5l5Deg !== undefined) {
    const denDegS5l5 = polynomialDegreeInK(den, k);
    if (denDegS5l5 !== undefined) {
      if (denDegS5l5 > s5l5Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 83: Mul(Log(h1)×6, polynomial..., bounded...) numerator — zero Sqrt factors.
  // log⁶ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > sixLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl6Deg = sixLogPolyEffectiveDeg(num, k);
  if (sl6Deg !== undefined) {
    const denDegSl6 = polynomialDegreeInK(den, k);
    if (denDegSl6 !== undefined) {
      if (denDegSl6 > sl6Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 84: Mul(Sqrt(P), Log(h1)×6, polynomial..., bounded...) numerator.
  // One Sqrt + six Log factors; log⁶ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtSixLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l6Deg = oneSqrtSixLogPolyEffectiveDeg(num, k);
  if (s1l6Deg !== undefined) {
    const denDegS1l6 = polynomialDegreeInK(den, k);
    if (denDegS1l6 !== undefined) {
      if (denDegS1l6 > s1l6Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 89: Mul(Log(h1)×7, polynomial..., bounded...) numerator — zero Sqrt factors.
  // log⁷ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > sevenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl7Deg = sevenLogPolyEffectiveDeg(num, k);
  if (sl7Deg !== undefined) {
    const denDegSl7 = polynomialDegreeInK(den, k);
    if (denDegSl7 !== undefined) {
      if (denDegSl7 > sl7Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 90: Mul(Sqrt(P), Log(h1)×7, polynomial..., bounded...) numerator.
  // One Sqrt + seven Log factors; log⁷ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtSevenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l7Deg = oneSqrtSevenLogPolyEffectiveDeg(num, k);
  if (s1l7Deg !== undefined) {
    const denDegS1l7 = polynomialDegreeInK(den, k);
    if (denDegS1l7 !== undefined) {
      if (denDegS1l7 > s1l7Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 91: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×7, polynomial..., bounded...) numerator.
  // Two Sqrt + seven Log factors; log⁷ sub-polynomial → effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  // Closes when denDeg > twoSqrtSevenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l7Deg = twoSqrtSevenLogPolyEffectiveDeg(num, k);
  if (s2l7Deg !== undefined) {
    const denDegS2l7 = polynomialDegreeInK(den, k);
    if (denDegS2l7 !== undefined) {
      if (denDegS2l7 > s2l7Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 92: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1)×7, polynomial..., bounded...) numerator.
  // Three Sqrt + seven Log factors; log⁷ sub-polynomial → effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + sqrtHalfDeg3 + polyDeg.
  // Closes when denDeg > threeSqrtSevenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l7Deg = threeSqrtSevenLogPolyEffectiveDeg(num, k);
  if (s3l7Deg !== undefined) {
    const denDegS3l7 = polynomialDegreeInK(den, k);
    if (denDegS3l7 !== undefined) {
      if (denDegS3l7 > s3l7Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 93: Mul(Sqrt(P1)..Sqrt(P4), Log(h1)×7, polynomial..., bounded...) numerator.
  // Four Sqrt + seven Log factors; log⁷ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtSevenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l7Deg = fourSqrtSevenLogPolyEffectiveDeg(num, k);
  if (s4l7Deg !== undefined) {
    const denDegS4l7 = polynomialDegreeInK(den, k);
    if (denDegS4l7 !== undefined) {
      if (denDegS4l7 > s4l7Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 94: Mul(Sqrt(P1)..Sqrt(P5), Log(h1)×7, polynomial..., bounded...) numerator.
  // Five Sqrt + seven Log factors; log⁷ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtSevenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l7Deg = fiveSqrtSevenLogPolyEffectiveDeg(num, k);
  if (s5l7Deg !== undefined) {
    const denDegS5l7 = polynomialDegreeInK(den, k);
    if (denDegS5l7 !== undefined) {
      if (denDegS5l7 > s5l7Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 95: Mul(Log(h1)×8, polynomial..., bounded...) numerator.
  // Zero Sqrt + eight Log factors; log⁸ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > eightLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl8Deg = eightLogPolyEffectiveDeg(num, k);
  if (sl8Deg !== undefined) {
    const denDegSl8 = polynomialDegreeInK(den, k);
    if (denDegSl8 !== undefined) {
      if (denDegSl8 > sl8Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 96: Mul(Sqrt(P), Log(h1)×8, polynomial..., bounded...) numerator.
  // One Sqrt + eight Log factors; log⁸ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtEightLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l8Deg = oneSqrtEightLogPolyEffectiveDeg(num, k);
  if (s1l8Deg !== undefined) {
    const denDegS1l8 = polynomialDegreeInK(den, k);
    if (denDegS1l8 !== undefined) {
      if (denDegS1l8 > s1l8Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 97: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×8, polynomial..., bounded...) numerator.
  // Two Sqrt + eight Log factors; log⁸ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtEightLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l8Deg = twoSqrtEightLogPolyEffectiveDeg(num, k);
  if (s2l8Deg !== undefined) {
    const denDegS2l8 = polynomialDegreeInK(den, k);
    if (denDegS2l8 !== undefined) {
      if (denDegS2l8 > s2l8Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 124: Mul(Sqrt(P1)×5, Log(h1)×12, polynomial..., bounded...) numerator.
  // Five Sqrt + twelve Log factors; log¹² sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtTwelveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l12Deg = fiveSqrtTwelveLogPolyEffectiveDeg(num, k);
  if (s5l12Deg !== undefined) {
    const denDegS5l12 = polynomialDegreeInK(den, k);
    if (denDegS5l12 !== undefined) {
      if (denDegS5l12 > s5l12Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 123: Mul(Sqrt(P1)×4, Log(h1)×12, polynomial..., bounded...) numerator.
  // Four Sqrt + twelve Log factors; log¹² sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtTwelveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l12Deg = fourSqrtTwelveLogPolyEffectiveDeg(num, k);
  if (s4l12Deg !== undefined) {
    const denDegS4l12 = polynomialDegreeInK(den, k);
    if (denDegS4l12 !== undefined) {
      if (denDegS4l12 > s4l12Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 122: Mul(Sqrt(P1)×3, Log(h1)×12, polynomial..., bounded...) numerator.
  // Three Sqrt + twelve Log factors; log¹² sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtTwelveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l12Deg = threeSqrtTwelveLogPolyEffectiveDeg(num, k);
  if (s3l12Deg !== undefined) {
    const denDegS3l12 = polynomialDegreeInK(den, k);
    if (denDegS3l12 !== undefined) {
      if (denDegS3l12 > s3l12Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 121: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×12, polynomial..., bounded...) numerator.
  // Two Sqrt + twelve Log factors; log¹² sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtTwelveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l12Deg = twoSqrtTwelveLogPolyEffectiveDeg(num, k);
  if (s2l12Deg !== undefined) {
    const denDegS2l12 = polynomialDegreeInK(den, k);
    if (denDegS2l12 !== undefined) {
      if (denDegS2l12 > s2l12Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 120: Mul(Sqrt(P), Log(h1)×12, polynomial..., bounded...) numerator.
  // One Sqrt + twelve Log factors; log¹² sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtTwelveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l12Deg = oneSqrtTwelveLogPolyEffectiveDeg(num, k);
  if (s1l12Deg !== undefined) {
    const denDegS1l12 = polynomialDegreeInK(den, k);
    if (denDegS1l12 !== undefined) {
      if (denDegS1l12 > s1l12Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 119: Mul(Log(h1)×12, polynomial..., bounded...) numerator.
  // Zero Sqrt + twelve Log factors; log¹² sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > twelveLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl12Deg = twelveLogPolyEffectiveDeg(num, k);
  if (sl12Deg !== undefined) {
    const denDegSl12 = polynomialDegreeInK(den, k);
    if (denDegSl12 !== undefined) {
      if (denDegSl12 > sl12Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 118: Mul(Sqrt(P1)×5, Log(h1)×11, polynomial..., bounded...) numerator.
  const s5l11Deg = fiveSqrtElevenLogPolyEffectiveDeg(num, k);
  if (s5l11Deg !== undefined) {
    const denDegS5l11 = polynomialDegreeInK(den, k);
    if (denDegS5l11 !== undefined) {
      if (denDegS5l11 > s5l11Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 117: Mul(Sqrt(P1)×4, Log(h1)×11, polynomial..., bounded...) numerator.
  const s4l11Deg = fourSqrtElevenLogPolyEffectiveDeg(num, k);
  if (s4l11Deg !== undefined) {
    const denDegS4l11 = polynomialDegreeInK(den, k);
    if (denDegS4l11 !== undefined) {
      if (denDegS4l11 > s4l11Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 116: Mul(Sqrt(P1)×3, Log(h1)×11, polynomial..., bounded...) numerator.
  const s3l11Deg = threeSqrtElevenLogPolyEffectiveDeg(num, k);
  if (s3l11Deg !== undefined) {
    const denDegS3l11 = polynomialDegreeInK(den, k);
    if (denDegS3l11 !== undefined) {
      if (denDegS3l11 > s3l11Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 115: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×11, polynomial..., bounded...) numerator.
  const s2l11Deg = twoSqrtElevenLogPolyEffectiveDeg(num, k);
  if (s2l11Deg !== undefined) {
    const denDegS2l11 = polynomialDegreeInK(den, k);
    if (denDegS2l11 !== undefined) {
      if (denDegS2l11 > s2l11Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 114: Mul(Sqrt(P), Log(h1)×11, polynomial..., bounded...) numerator.
  const s1l11Deg = oneSqrtElevenLogPolyEffectiveDeg(num, k);
  if (s1l11Deg !== undefined) {
    const denDegS1l11 = polynomialDegreeInK(den, k);
    if (denDegS1l11 !== undefined) {
      if (denDegS1l11 > s1l11Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 113: Mul(Log(h1)×11, polynomial..., bounded...) numerator.
  const sl11Deg = elevenLogPolyEffectiveDeg(num, k);
  if (sl11Deg !== undefined) {
    const denDegSl11 = polynomialDegreeInK(den, k);
    if (denDegSl11 !== undefined) {
      if (denDegSl11 > sl11Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 112: Mul(Sqrt(P1)×5, Log(h1)×10, polynomial..., bounded...) numerator.
  const s5l10Deg = fiveSqrtTenLogPolyEffectiveDeg(num, k);
  if (s5l10Deg !== undefined) {
    const denDegS5l10 = polynomialDegreeInK(den, k);
    if (denDegS5l10 !== undefined) {
      if (denDegS5l10 > s5l10Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 111: Mul(Sqrt(P1)×4, Log(h1)×10, polynomial..., bounded...) numerator.
  const s4l10Deg = fourSqrtTenLogPolyEffectiveDeg(num, k);
  if (s4l10Deg !== undefined) {
    const denDegS4l10 = polynomialDegreeInK(den, k);
    if (denDegS4l10 !== undefined) {
      if (denDegS4l10 > s4l10Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 110: Mul(Sqrt(P1)×3, Log(h1)×10, polynomial..., bounded...) numerator.
  const s3l10Deg = threeSqrtTenLogPolyEffectiveDeg(num, k);
  if (s3l10Deg !== undefined) {
    const denDegS3l10 = polynomialDegreeInK(den, k);
    if (denDegS3l10 !== undefined) {
      if (denDegS3l10 > s3l10Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 109: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×10, polynomial..., bounded...) numerator.
  const s2l10Deg = twoSqrtTenLogPolyEffectiveDeg(num, k);
  if (s2l10Deg !== undefined) {
    const denDegS2l10 = polynomialDegreeInK(den, k);
    if (denDegS2l10 !== undefined) {
      if (denDegS2l10 > s2l10Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 108: Mul(Sqrt(P), Log(h1)×10, polynomial..., bounded...) numerator.
  const s1l10Deg = oneSqrtTenLogPolyEffectiveDeg(num, k);
  if (s1l10Deg !== undefined) {
    const denDegS1l10 = polynomialDegreeInK(den, k);
    if (denDegS1l10 !== undefined) {
      if (denDegS1l10 > s1l10Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 107: Mul(Log(h1)×10, polynomial..., bounded...) numerator.
  const sl10Deg = tenLogPolyEffectiveDeg(num, k);
  if (sl10Deg !== undefined) {
    const denDegSl10 = polynomialDegreeInK(den, k);
    if (denDegSl10 !== undefined) {
      if (denDegSl10 > sl10Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 106: Mul(Sqrt(P1)×5, Log(h1)×9, polynomial..., bounded...) numerator.
  // Five Sqrt + nine Log factors; log⁹ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtNineLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l9Deg = fiveSqrtNineLogPolyEffectiveDeg(num, k);
  if (s5l9Deg !== undefined) {
    const denDegS5l9 = polynomialDegreeInK(den, k);
    if (denDegS5l9 !== undefined) {
      if (denDegS5l9 > s5l9Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 105: Mul(Sqrt(P1)×4, Log(h1)×9, polynomial..., bounded...) numerator.
  // Four Sqrt + nine Log factors; log⁹ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtNineLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l9Deg = fourSqrtNineLogPolyEffectiveDeg(num, k);
  if (s4l9Deg !== undefined) {
    const denDegS4l9 = polynomialDegreeInK(den, k);
    if (denDegS4l9 !== undefined) {
      if (denDegS4l9 > s4l9Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 104: Mul(Sqrt(P1)×3, Log(h1)×9, polynomial..., bounded...) numerator.
  // Three Sqrt + nine Log factors; log⁹ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtNineLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l9Deg = threeSqrtNineLogPolyEffectiveDeg(num, k);
  if (s3l9Deg !== undefined) {
    const denDegS3l9 = polynomialDegreeInK(den, k);
    if (denDegS3l9 !== undefined) {
      if (denDegS3l9 > s3l9Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 103: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×9, polynomial..., bounded...) numerator.
  // Two Sqrt + nine Log factors; log⁹ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtNineLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l9Deg = twoSqrtNineLogPolyEffectiveDeg(num, k);
  if (s2l9Deg !== undefined) {
    const denDegS2l9 = polynomialDegreeInK(den, k);
    if (denDegS2l9 !== undefined) {
      if (denDegS2l9 > s2l9Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 102: Mul(Sqrt(P), Log(h1)×9, polynomial..., bounded...) numerator.
  // One Sqrt + nine Log factors; log⁹ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtNineLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l9Deg = oneSqrtNineLogPolyEffectiveDeg(num, k);
  if (s1l9Deg !== undefined) {
    const denDegS1l9 = polynomialDegreeInK(den, k);
    if (denDegS1l9 !== undefined) {
      if (denDegS1l9 > s1l9Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 101: Mul(Log(h1)×9, polynomial..., bounded...) numerator.
  // Zero Sqrt + nine Log factors; log⁹ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > nineLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl9Deg = nineLogPolyEffectiveDeg(num, k);
  if (sl9Deg !== undefined) {
    const denDegSl9 = polynomialDegreeInK(den, k);
    if (denDegSl9 !== undefined) {
      if (denDegSl9 > sl9Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 100: Mul(Sqrt(P1)×5, Log(h1)×8, polynomial..., bounded...) numerator.
  // Five Sqrt + eight Log factors; log⁸ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtEightLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l8Deg = fiveSqrtEightLogPolyEffectiveDeg(num, k);
  if (s5l8Deg !== undefined) {
    const denDegS5l8 = polynomialDegreeInK(den, k);
    if (denDegS5l8 !== undefined) {
      if (denDegS5l8 > s5l8Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 99: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(h1)×8, polynomial..., bounded...) numerator.
  // Four Sqrt + eight Log factors; log⁸ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtEightLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l8Deg = fourSqrtEightLogPolyEffectiveDeg(num, k);
  if (s4l8Deg !== undefined) {
    const denDegS4l8 = polynomialDegreeInK(den, k);
    if (denDegS4l8 !== undefined) {
      if (denDegS4l8 > s4l8Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 98: Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1)×8, polynomial..., bounded...) numerator.
  // Three Sqrt + eight Log factors; log⁸ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtEightLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l8Deg = threeSqrtEightLogPolyEffectiveDeg(num, k);
  if (s3l8Deg !== undefined) {
    const denDegS3l8 = polynomialDegreeInK(den, k);
    if (denDegS3l8 !== undefined) {
      if (denDegS3l8 > s3l8Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 42 widening: deg(num) < deg(den) on pure polynomials.
  const numDeg = polynomialDegreeInK(num, k);
  if (numDeg === undefined) return false;
  const denDeg = polynomialDegreeInK(den, k);
  if (denDeg === undefined) return false;
  return numDeg < denDeg;
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
