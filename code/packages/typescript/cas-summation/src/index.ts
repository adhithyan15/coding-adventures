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

function thirteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 13) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 13) return undefined;
  return polyDeg;
}

function oneSqrtThirteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 13) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 13) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtThirteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 13) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 13) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtThirteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 13) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 13) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtThirteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 13) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 13) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtThirteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 13) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 13) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

function fourteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 14) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 14) return undefined;
  return polyDeg;
}

function oneSqrtFourteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 14) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 14) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtFourteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 14) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 14) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtFourteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 14) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 14) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtFourteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 14) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 14) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtFourteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 14) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 14) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

function fifteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 15) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 15) return undefined;
  return polyDeg;
}

function oneSqrtFifteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 15) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 15) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtFifteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 15) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 15) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtFifteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 15) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 15) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtFifteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 15) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 15) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtFifteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 15) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 15) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

function sixteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 143 — Sixteen-Log × polynomial numerator.
  // log^16(k) is sub-polynomial; effective degree = polyDeg. No Sqrt factors allowed.
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 16) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 16) return undefined;
  return polyDeg;
}

function oneSqrtSixteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 144 — One-Sqrt × Sixteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDeg + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 16) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 16) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtSixteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 145 — Two-Sqrt × Sixteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 16) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 16) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtSixteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 146 — Three-Sqrt × Sixteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 16) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 16) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtSixteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 147 — Four-Sqrt × Sixteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + … + sqrtHalfDegs[3] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 16) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 16) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtSixteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 148 — Five-Sqrt × Sixteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + … + sqrtHalfDegs[4] + polyDeg.
  // Completes the Sixteen-Log family (Phases 143-148).
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 16) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 16) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

function seventeenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 149 — Seventeen-Log × polynomial numerator.
  // log^17(k) is sub-polynomial; effective degree = polyDeg. No Sqrt factors allowed.
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 17) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 17) return undefined;
  return polyDeg;
}

function oneSqrtSeventeenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 150 — One-Sqrt × Seventeen-Log × polynomial numerator.
  // effective degree = sqrtHalfDeg + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 17) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 17) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtSeventeenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 151 — Two-Sqrt × Seventeen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 17) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 17) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtSeventeenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 152 — Three-Sqrt × Seventeen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 17) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 17) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtSeventeenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 153 — Four-Sqrt × Seventeen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + … + sqrtHalfDegs[3] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 17) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 17) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtSeventeenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 154 — Five-Sqrt × Seventeen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + … + sqrtHalfDegs[4] + polyDeg.
  // Completes the Seventeen-Log family (Phases 149-154).
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 17) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 17) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

function eighteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 155 — Eighteen-Log × polynomial numerator.
  // log^18(k) is sub-polynomial; effective degree = polyDeg. No Sqrt factors allowed.
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 18) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 18) return undefined;
  return polyDeg;
}

function oneSqrtEighteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 156 — One-Sqrt × Eighteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDeg + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 18) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 18) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtEighteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 157 — Two-Sqrt × Eighteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 18) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 18) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtEighteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 158 — Three-Sqrt × Eighteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 18) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 18) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtEighteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 159 — Four-Sqrt × Eighteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + … + sqrtHalfDegs[3] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 18) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 18) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtEighteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 160 — Five-Sqrt × Eighteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + … + sqrtHalfDegs[4] + polyDeg.
  // Completes the Eighteen-Log family (Phases 155-160).
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 18) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 18) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

function nineteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 161 — Nineteen-Log × polynomial numerator.
  // log^19(k) is sub-polynomial; effective degree = polyDeg. No Sqrt factors allowed.
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 19) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 19) return undefined;
  return polyDeg;
}

function oneSqrtNineteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 162 — One-Sqrt × Nineteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDeg + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 19) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 19) return undefined;
  return sqrtHalfDeg + polyDeg;
}

function twoSqrtNineteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 163 — Two-Sqrt × Nineteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 19) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 19) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

function threeSqrtNineteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 164 — Three-Sqrt × Nineteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 19) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 19) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

function fourSqrtNineteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 165 — Four-Sqrt × Nineteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + … + sqrtHalfDegs[3] + polyDeg.
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 19) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 19) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

function fiveSqrtNineteenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  // Phase 166 — Five-Sqrt × Nineteen-Log × polynomial numerator.
  // effective degree = sqrtHalfDegs[0] + … + sqrtHalfDegs[4] + polyDeg.
  // Completes the Nineteen-Log family (Phases 161-166).
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 19) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 19) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/**
 * Phase 167 — Twenty-Log × polynomial numerator.
 * Caller checks `denDeg > twentyLogPolyEffectiveDeg(num, k)`.
 */
function twentyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 20) return undefined; // twenty-one or more Logs — not this phase
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 20) return undefined;
  return polyDeg;
}

/**
 * Phase 168 — One-Sqrt × Twenty-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtTwentyLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 20) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 20) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 169 — Two-Sqrt × Twenty-Log × polynomial numerator.
 * Caller checks `denDeg > twoSqrtTwentyLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtTwentyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 20) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 20) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 170 — Three-Sqrt × Twenty-Log × polynomial numerator.
 * Caller checks `denDeg > threeSqrtTwentyLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtTwentyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 20) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 20) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 171 — Four-Sqrt × Twenty-Log × polynomial numerator.
 * Caller checks `denDeg > fourSqrtTwentyLogPolyEffectiveDeg(num, k)`.
 */
function fourSqrtTwentyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 20) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 20) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/**
 * Phase 172 — Five-Sqrt × Twenty-Log × polynomial numerator.
 * Completes the Twenty-Log family (Phases 167-172).
 * Caller checks `denDeg > fiveSqrtTwentyLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 20) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 20) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/**
 * Phase 173 — Twenty-One-Log × polynomial numerator.
 * Caller checks `denDeg > twentyOneLogPolyEffectiveDeg(num, k)`.
 */
function twentyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 21) return undefined; // twenty-two or more Logs — not this phase
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 21) return undefined;
  return polyDeg;
}

/**
 * Phase 174 — One-Sqrt × Twenty-One-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtTwentyOneLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 21) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 21) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 175 — Two-Sqrt × Twenty-One-Log × polynomial numerator.
 * Caller checks `denDeg > twoSqrtTwentyOneLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtTwentyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 21) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 21) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 176 — Three-Sqrt × Twenty-One-Log × polynomial numerator.
 * Caller checks `denDeg > threeSqrtTwentyOneLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtTwentyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 21) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 21) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 177 — Four-Sqrt × Twenty-One-Log × polynomial numerator.
 * Caller checks `denDeg > fourSqrtTwentyOneLogPolyEffectiveDeg(num, k)`.
 */
function fourSqrtTwentyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 21) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 21) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/**
 * Phase 178 — Five-Sqrt × Twenty-One-Log × polynomial numerator.
 * Completes the Twenty-One-Log family (Phases 173-178).
 * Caller checks `denDeg > fiveSqrtTwentyOneLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 21) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 21) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/**
 * Phase 179 — Twenty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > twentyTwoLogPolyEffectiveDeg(num, k)`.
 */
function twentyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 22) return undefined; // twenty-three or more Logs — not this phase
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 22) return undefined;
  return polyDeg;
}

/**
 * Phase 180 — One-Sqrt × Twenty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtTwentyTwoLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 22) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 22) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 181 — Two-Sqrt × Twenty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > twoSqrtTwentyTwoLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtTwentyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 22) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 22) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 182 — Three-Sqrt × Twenty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > threeSqrtTwentyTwoLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtTwentyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 22) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 22) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 183 — Four-Sqrt × Twenty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > fourSqrtTwentyTwoLogPolyEffectiveDeg(num, k)`.
 */
function fourSqrtTwentyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 22) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 22) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/**
 * Phase 184 — Five-Sqrt × Twenty-Two-Log × polynomial numerator.
 * Completes the Twenty-Two-Log family (Phases 179-184).
 * Caller checks `denDeg > fiveSqrtTwentyTwoLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 22) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 22) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/**
 * Phase 185 — Twenty-Three-Log × polynomial numerator.
 * Caller checks `denDeg > twentyThreeLogPolyEffectiveDeg(num, k)`.
 */
function twentyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 23) return undefined; // twenty-four or more Logs — not this phase
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 23) return undefined;
  return polyDeg;
}

/**
 * Phase 186 — One-Sqrt × Twenty-Three-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtTwentyThreeLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 23) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 23) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/**
 * Phase 187 — Two-Sqrt × Twenty-Three-Log × polynomial numerator.
 * Caller checks `denDeg > twoSqrtTwentyThreeLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtTwentyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 23) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 23) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/**
 * Phase 188 — Three-Sqrt × Twenty-Three-Log × polynomial numerator.
 * Caller checks `denDeg > threeSqrtTwentyThreeLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtTwentyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 23) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 23) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/**
 * Phase 189 — Four-Sqrt × Twenty-Three-Log × polynomial numerator.
 * Caller checks `denDeg > fourSqrtTwentyThreeLogPolyEffectiveDeg(num, k)`.
 */
function fourSqrtTwentyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 23) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 23) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/**
 * Phase 190 — Five-Sqrt × Twenty-Three-Log × polynomial numerator.
 * Completes the Twenty-Three-Log family (Phases 185-190).
 * Caller checks `denDeg > fiveSqrtTwentyThreeLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 23) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 23) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 191 — Twenty-Four-Log × polynomial numerator.
 * `log(k)^24` is sub-polynomial; effectiveDeg = polyDeg.
 * Caller checks `denDeg > twentyFourLogPolyEffectiveDeg(num, k)`.
 */
function twentyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 24) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 24) return undefined;
  return polyDeg;
}

/** Phase 192 — One-Sqrt × Twenty-Four-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtTwentyFourLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
      if (logCount > 24) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 24) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 193 — Two-Sqrt × Twenty-Four-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg.
 */
function twoSqrtTwentyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 24) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 24) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 194 — Three-Sqrt × Twenty-Four-Log × polynomial numerator.
 * effectiveDeg = sum(sqrtHalfDegs) + polyDeg.
 */
function threeSqrtTwentyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 24) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 24) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 195 — Four-Sqrt × Twenty-Four-Log × polynomial numerator.
 * effectiveDeg = sum(sqrtHalfDegs) + polyDeg.
 */
function fourSqrtTwentyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 24) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 24) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 196 — Five-Sqrt × Twenty-Four-Log × polynomial numerator.
 * Completes the Twenty-Four-Log family (Phases 191-196).
 * Caller checks `denDeg > fiveSqrtTwentyFourLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 24) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 24) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 197 — Zero-Sqrt × Twenty-Five-Log × polynomial numerator.
 * effectiveDeg = polyDeg.
 * Caller checks `denDeg > twentyFiveLogPolyEffectiveDeg(num, k)`.
 */
function twentyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 25) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 25) return undefined;
  return polyDeg;
}

/** Phase 198 — One-Sqrt × Twenty-Five-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtTwentyFiveLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 25) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 25) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 199 — Two-Sqrt × Twenty-Five-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 */
function twoSqrtTwentyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 25) return undefined; continue; }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 25) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 200 — Three-Sqrt × Twenty-Five-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+sqrtHalfDeg2+sqrtHalfDeg3 + polyDeg.
 */
function threeSqrtTwentyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 25) return undefined; continue; }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 25) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 201 — Four-Sqrt × Twenty-Five-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg4 + polyDeg.
 */
function fourSqrtTwentyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 25) return undefined; continue; }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 25) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 202 — Five-Sqrt × Twenty-Five-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtTwentyFiveLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 25) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 25) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 203 — Zero-Sqrt × Twenty-Six-Log × polynomial numerator.
 * effectiveDeg = polyDeg.
 * Caller checks `denDeg > twentySixLogPolyEffectiveDeg(num, k)`.
 */
function twentySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 26) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 26) return undefined;
  return polyDeg;
}

/** Phase 204 — One-Sqrt × Twenty-Six-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtTwentySixLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined; // only 1 sqrt
      sqrtHalfDeg = hd;
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 26) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 26) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 205 — Two-Sqrt × Twenty-Six-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtTwentySixLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtTwentySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 26) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 26) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 206 — Three-Sqrt × Twenty-Six-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+sqrtHalfDeg2+sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtTwentySixLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtTwentySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 26) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 26) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 207 — Four-Sqrt × Twenty-Six-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg4 + polyDeg.
 * Caller checks `denDeg > fourSqrtTwentySixLogPolyEffectiveDeg(num, k)`.
 */
function fourSqrtTwentySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 26) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 26) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 208 — Five-Sqrt × Twenty-Six-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtTwentySixLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 26) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 26) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 209 — Zero-Sqrt × Twenty-Seven-Log × polynomial numerator.
 * effectiveDeg = polyDeg.
 * Caller checks `denDeg > twentySevenLogPolyEffectiveDeg(num, k)`.
 */
function twentySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 27) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 27) return undefined;
  return polyDeg;
}

/** Phase 210 — One-Sqrt × Twenty-Seven-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtTwentySevenLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 27) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 27) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 211 — Two-Sqrt × Twenty-Seven-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtTwentySevenLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtTwentySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 27) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 27) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 212 — Three-Sqrt × Twenty-Seven-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+sqrtHalfDeg2+sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtTwentySevenLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtTwentySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 27) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 27) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 213 — Four-Sqrt × Twenty-Seven-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg4 + polyDeg.
 * Caller checks `denDeg > fourSqrtTwentySevenLogPolyEffectiveDeg(num, k)`.
 */
function fourSqrtTwentySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 27) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 27) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 214 — Five-Sqrt × Twenty-Seven-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtTwentySevenLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 27) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 27) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 215 — Zero-Sqrt × Twenty-Eight-Log × polynomial numerator.
 * effectiveDeg = polyDeg.
 * Caller checks `denDeg > twentyEightLogPolyEffectiveDeg(num, k)`.
 *
 * The presence of exactly 28 log-diverging factors multiplied by a polynomial
 * in `k` makes this summand grow slower than any power of k, yet still slower
 * than any denominator whose degree strictly exceeds polyDeg.
 */
function twentyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 28) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 28) return undefined;
  return polyDeg;
}

/** Phase 216 — One-Sqrt × Twenty-Eight-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtTwentyEightLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 28) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 28) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 217 — Two-Sqrt × Twenty-Eight-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtTwentyEightLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtTwentyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 28) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 28) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 218 — Three-Sqrt × Twenty-Eight-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+sqrtHalfDeg2+sqrtHalfDeg3 + polyDeg.
 * Caller checks `denDeg > threeSqrtTwentyEightLogPolyEffectiveDeg(num, k)`.
 */
function threeSqrtTwentyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 28) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 28) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 219 — Four-Sqrt × Twenty-Eight-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg4 + polyDeg.
 * Caller checks `denDeg > fourSqrtTwentyEightLogPolyEffectiveDeg(num, k)`.
 */
function fourSqrtTwentyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 28) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 28) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 220 — Five-Sqrt × Twenty-Eight-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtTwentyEightLogPolyEffectiveDeg(num, k)`.
 */
function fiveSqrtTwentyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 28) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 28) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 221 — Zero-Sqrt × Twenty-Nine-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > twentyNineLogPolyEffectiveDeg(num, k)`.
 */
function twentyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 29) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 29) return undefined;
  return polyDeg;
}

/** Phase 222 — One-Sqrt × Twenty-Nine-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtTwentyNineLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtTwentyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 29) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 29) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 223 — Two-Sqrt × Twenty-Nine-Log × polynomial numerator. */
function twoSqrtTwentyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 29) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 29) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 224 — Three-Sqrt × Twenty-Nine-Log × polynomial numerator. */
function threeSqrtTwentyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 29) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 29) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 225 — Four-Sqrt × Twenty-Nine-Log × polynomial numerator. */
function fourSqrtTwentyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 29) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 29) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 226 — Five-Sqrt × Twenty-Nine-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtTwentyNineLogPolyEffectiveDeg(num, k)`.
 * Completes the Twenty-Nine-Log family (Phases 221–226).
 */
function fiveSqrtTwentyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 29) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 29) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 233 — Zero-Sqrt × Thirty-One-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtyOneLogPolyEffectiveDeg(num, k)`.
 */
function thirtyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 31) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 31) return undefined;
  return polyDeg;
}

/** Phase 234 — One-Sqrt × Thirty-One-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtThirtyOneLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 31) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 31) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 235 — Two-Sqrt × Thirty-One-Log × polynomial numerator. */
function twoSqrtThirtyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 31) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 31) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 236 — Three-Sqrt × Thirty-One-Log × polynomial numerator. */
function threeSqrtThirtyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 31) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 31) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 237 — Four-Sqrt × Thirty-One-Log × polynomial numerator. */
function fourSqrtThirtyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 31) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 31) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 238 — Five-Sqrt × Thirty-One-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtThirtyOneLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-One-Log family (Phases 233–238).
 */
function fiveSqrtThirtyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 31) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 31) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 257 — Zero-Sqrt × Thirty-Five-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtyFiveLogPolyEffectiveDeg(num, k)`.
 */
function thirtyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 35) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 35) return undefined;
  return polyDeg;
}

/** Phase 258 — One-Sqrt × Thirty-Five-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtThirtyFiveLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 35) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 35) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 259 — Two-Sqrt × Thirty-Five-Log × polynomial numerator. */
function twoSqrtThirtyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 35) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 35) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 260 — Three-Sqrt × Thirty-Five-Log × polynomial numerator. */
function threeSqrtThirtyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 35) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 35) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 261 — Four-Sqrt × Thirty-Five-Log × polynomial numerator. */
function fourSqrtThirtyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 35) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 35) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 262 — Five-Sqrt × Thirty-Five-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtThirtyFiveLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Five-Log family (Phases 257–262).
 */
function fiveSqrtThirtyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 35) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 35) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 263 — Zero-Sqrt × Thirty-Six-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtySixLogPolyEffectiveDeg(num, k)`.
 */
function thirtySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 36) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 36) return undefined;
  return polyDeg;
}

/** Phase 264 — One-Sqrt × Thirty-Six-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtThirtySixLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 36) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 36) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 265 — Two-Sqrt × Thirty-Six-Log × polynomial numerator. */
function twoSqrtThirtySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 36) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 36) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 266 — Three-Sqrt × Thirty-Six-Log × polynomial numerator. */
function threeSqrtThirtySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 36) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 36) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 267 — Four-Sqrt × Thirty-Six-Log × polynomial numerator. */
function fourSqrtThirtySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 36) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 36) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 268 — Five-Sqrt × Thirty-Six-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtThirtySixLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Six-Log family (Phases 263–268).
 */
function fiveSqrtThirtySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 36) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 36) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 269 — Zero-Sqrt × Thirty-Seven-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtySevenLogPolyEffectiveDeg(num, k)`.
 */
function thirtySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 37) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 37) return undefined;
  return polyDeg;
}

/** Phase 270 — One-Sqrt × Thirty-Seven-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtThirtySevenLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 37) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 37) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 271 — Two-Sqrt × Thirty-Seven-Log × polynomial numerator. */
function twoSqrtThirtySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 37) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 37) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 272 — Three-Sqrt × Thirty-Seven-Log × polynomial numerator. */
function threeSqrtThirtySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 37) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 37) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 273 — Four-Sqrt × Thirty-Seven-Log × polynomial numerator. */
function fourSqrtThirtySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 37) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 37) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 274 — Five-Sqrt × Thirty-Seven-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtThirtySevenLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Seven-Log family (Phases 269–274).
 */
function fiveSqrtThirtySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 37) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 37) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 275 — Zero-Sqrt × Thirty-Eight-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtyEightLogPolyEffectiveDeg(num, k)`.
 */
function thirtyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 38) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 38) return undefined;
  return polyDeg;
}

/** Phase 276 — One-Sqrt × Thirty-Eight-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtThirtyEightLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 38) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 38) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 277 — Two-Sqrt × Thirty-Eight-Log × polynomial numerator. */
function twoSqrtThirtyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 38) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 38) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 278 — Three-Sqrt × Thirty-Eight-Log × polynomial numerator. */
function threeSqrtThirtyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 38) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 38) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 279 — Four-Sqrt × Thirty-Eight-Log × polynomial numerator. */
function fourSqrtThirtyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 38) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 38) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 280 — Five-Sqrt × Thirty-Eight-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtThirtyEightLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Eight-Log family (Phases 275–280).
 */
function fiveSqrtThirtyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 38) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 38) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 281 — Zero-Sqrt × Thirty-Nine-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtyNineLogPolyEffectiveDeg(num, k)`.
 */
function thirtyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 39) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 39) return undefined;
  return polyDeg;
}

/** Phase 282 — One-Sqrt × Thirty-Nine-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtThirtyNineLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 39) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 39) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 283 — Two-Sqrt × Thirty-Nine-Log × polynomial numerator. */
function twoSqrtThirtyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 39) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 39) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 284 — Three-Sqrt × Thirty-Nine-Log × polynomial numerator. */
function threeSqrtThirtyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 39) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 39) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 285 — Four-Sqrt × Thirty-Nine-Log × polynomial numerator. */
function fourSqrtThirtyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 39) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 39) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 286 — Five-Sqrt × Thirty-Nine-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtThirtyNineLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Nine-Log family (Phases 281–286).
 */
function fiveSqrtThirtyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 39) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 39) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 287 — Zero-Sqrt × Forty-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortyLogPolyEffectiveDeg(num, k)`.
 */
function fortyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 40) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 40) return undefined;
  return polyDeg;
}

/** Phase 288 — One-Sqrt × Forty-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortyLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 40) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 40) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 289 — Two-Sqrt × Forty-Log × polynomial numerator. */
function twoSqrtFortyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 40) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 40) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 290 — Three-Sqrt × Forty-Log × polynomial numerator. */
function threeSqrtFortyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 40) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 40) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 291 — Four-Sqrt × Forty-Log × polynomial numerator. */
function fourSqrtFortyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 40) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 40) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 292 — Five-Sqrt × Forty-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortyLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Log family (Phases 287–292).
 */
function fiveSqrtFortyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 40) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 40) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 299 — Zero-Sqrt × Forty-Two-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortyTwoLogPolyEffectiveDeg(num, k)`.
 */
function fortyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 42) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 42) return undefined;
  return polyDeg;
}

/** Phase 300 — One-Sqrt × Forty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortyTwoLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 42) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 42) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 301 — Two-Sqrt × Forty-Two-Log × polynomial numerator. */
function twoSqrtFortyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 42) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 42) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 302 — Three-Sqrt × Forty-Two-Log × polynomial numerator. */
function threeSqrtFortyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 42) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 42) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 303 — Four-Sqrt × Forty-Two-Log × polynomial numerator. */
function fourSqrtFortyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 42) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 42) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 304 — Five-Sqrt × Forty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortyTwoLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Two-Log family (Phases 299–304).
 */
function fiveSqrtFortyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 42) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 42) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 305 — Zero-Sqrt × Forty-Three-Log × polynomial numerator.
 * The Forty-Three-Log family (Phases 305–310) extends the recogniser to
 * summands whose numerator contains exactly forty-three logarithmic factors
 * and zero to five square-root factors.  This is Phase 305: zero sqrts.
 * Caller checks `denDeg > fortyThreeLogPolyEffectiveDeg(num, k)`.
 */
function fortyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 43) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 43) return undefined;
  return polyDeg;
}

/** Phase 306 — One-Sqrt × Forty-Three-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortyThreeLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 43) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 43) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 307 — Two-Sqrt × Forty-Three-Log × polynomial numerator. */
function twoSqrtFortyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 43) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 43) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 308 — Three-Sqrt × Forty-Three-Log × polynomial numerator. */
function threeSqrtFortyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 43) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 43) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 309 — Four-Sqrt × Forty-Three-Log × polynomial numerator. */
function fourSqrtFortyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 43) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 43) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 364 — Five-Sqrt × Fifty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFiftyTwoLogPolyEffectiveDeg(num, k)`.
 * Completes the Fifty-Two-Log family (Phases 359–364).
 */
function fiveSqrtFiftyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 52) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 52) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 363 — Four-Sqrt × Fifty-Two-Log × polynomial numerator. */
function fourSqrtFiftyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 52) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 52) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 362 — Three-Sqrt × Fifty-Two-Log × polynomial numerator. */
function threeSqrtFiftyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 52) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 52) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 361 — Two-Sqrt × Fifty-Two-Log × polynomial numerator. */
function twoSqrtFiftyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 52) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 52) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 360 — One-Sqrt × Fifty-Two-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFiftyTwoLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFiftyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 52) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 52) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 365 — Zero-Sqrt × Fifty-Three-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftyThreeLogPolyEffectiveDeg(num, k)`.
 */
function fiftyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 53) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 53) return undefined;
  return polyDeg;
}

/** Phase 366 — One-Sqrt × Fifty-Three-Log × polynomial numerator. */
function oneSqrtFiftyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 53) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 53) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 367 — Two-Sqrt × Fifty-Three-Log × polynomial numerator. */
function twoSqrtFiftyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 53) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 53) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 368 — Three-Sqrt × Fifty-Three-Log × polynomial numerator. */
function threeSqrtFiftyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 53) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 53) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 369 — Four-Sqrt × Fifty-Three-Log × polynomial numerator. */
function fourSqrtFiftyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 53) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 53) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 370 — Five-Sqrt × Fifty-Three-Log × polynomial numerator.
 * Completes the Fifty-Three-Log family (Phases 365–370).
 */
function fiveSqrtFiftyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 53) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 53) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 377 — Zero-Sqrt × Fifty-Five-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftyFiveLogPolyEffectiveDeg(num, k)`.
 */
function fiftyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 55) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 55) return undefined;
  return polyDeg;
}

/** Phase 378 — One-Sqrt × Fifty-Five-Log × polynomial numerator. */
function oneSqrtFiftyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 55) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 55) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 379 — Two-Sqrt × Fifty-Five-Log × polynomial numerator. */
function twoSqrtFiftyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 55) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 55) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 380 — Three-Sqrt × Fifty-Five-Log × polynomial numerator. */
function threeSqrtFiftyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 55) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 55) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 381 — Four-Sqrt × Fifty-Five-Log × polynomial numerator. */
function fourSqrtFiftyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 55) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 55) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 382 — Five-Sqrt × Fifty-Five-Log × polynomial numerator.
 * Completes the Fifty-Five-Log family (Phases 377–382).
 */
function fiveSqrtFiftyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 55) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 55) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 383 — Zero-Sqrt × Fifty-Six-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftySixLogPolyEffectiveDeg(num, k)`.
 */
function fiftySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 56) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 56) return undefined;
  return polyDeg;
}

/** Phase 384 — One-Sqrt × Fifty-Six-Log × polynomial numerator. */
function oneSqrtFiftySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 56) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 56) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 385 — Two-Sqrt × Fifty-Six-Log × polynomial numerator. */
function twoSqrtFiftySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 56) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 56) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 386 — Three-Sqrt × Fifty-Six-Log × polynomial numerator. */
function threeSqrtFiftySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 56) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 56) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 387 — Four-Sqrt × Fifty-Six-Log × polynomial numerator. */
function fourSqrtFiftySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 56) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 56) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 388 — Five-Sqrt × Fifty-Six-Log × polynomial numerator.
 * Completes the Fifty-Six-Log family (Phases 383–388).
 */
function fiveSqrtFiftySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 56) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 56) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 389 — Zero-Sqrt × Fifty-Seven-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftySevenLogPolyEffectiveDeg(num, k)`.
 */
function fiftySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 57) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 57) return undefined;
  return polyDeg;
}

/** Phase 390 — One-Sqrt × Fifty-Seven-Log × polynomial numerator. */
function oneSqrtFiftySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 57) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 57) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 391 — Two-Sqrt × Fifty-Seven-Log × polynomial numerator. */
function twoSqrtFiftySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 57) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 57) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 392 — Three-Sqrt × Fifty-Seven-Log × polynomial numerator. */
function threeSqrtFiftySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 57) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 57) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 393 — Four-Sqrt × Fifty-Seven-Log × polynomial numerator. */
function fourSqrtFiftySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 57) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 57) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 394 — Five-Sqrt × Fifty-Seven-Log × polynomial numerator.
 * Completes the Fifty-Seven-Log family (Phases 389–394).
 */
function fiveSqrtFiftySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 57) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 57) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 401 — Zero-Sqrt × Fifty-Nine-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftyNineLogPolyEffectiveDeg(num, k)`.
 */
function fiftyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 59) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 59) return undefined;
  return polyDeg;
}

/** Phase 402 — One-Sqrt × Fifty-Nine-Log × polynomial numerator. */
function oneSqrtFiftyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 59) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 59) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 403 — Two-Sqrt × Fifty-Nine-Log × polynomial numerator. */
function twoSqrtFiftyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 59) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 59) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 404 — Three-Sqrt × Fifty-Nine-Log × polynomial numerator. */
function threeSqrtFiftyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 59) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 59) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 405 — Four-Sqrt × Fifty-Nine-Log × polynomial numerator. */
function fourSqrtFiftyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 59) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 59) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 406 — Five-Sqrt × Fifty-Nine-Log × polynomial numerator.
 * Completes the Fifty-Nine-Log family (Phases 401–406).
 */
function fiveSqrtFiftyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 59) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 59) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 395 — Zero-Sqrt × Fifty-Eight-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftyEightLogPolyEffectiveDeg(num, k)`.
 */
function fiftyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 58) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 58) return undefined;
  return polyDeg;
}

/** Phase 396 — One-Sqrt × Fifty-Eight-Log × polynomial numerator. */
function oneSqrtFiftyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 58) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 58) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 397 — Two-Sqrt × Fifty-Eight-Log × polynomial numerator. */
function twoSqrtFiftyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 58) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 58) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 398 — Three-Sqrt × Fifty-Eight-Log × polynomial numerator. */
function threeSqrtFiftyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 58) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 58) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 399 — Four-Sqrt × Fifty-Eight-Log × polynomial numerator. */
function fourSqrtFiftyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 58) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 58) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 400 — Five-Sqrt × Fifty-Eight-Log × polynomial numerator.
 * Completes the Fifty-Eight-Log family (Phases 395–400).
 */
function fiveSqrtFiftyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 58) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 58) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 371 — Zero-Sqrt × Fifty-Four-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftyFourLogPolyEffectiveDeg(num, k)`.
 */
function fiftyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 54) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 54) return undefined;
  return polyDeg;
}

/** Phase 372 — One-Sqrt × Fifty-Four-Log × polynomial numerator. */
function oneSqrtFiftyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 54) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 54) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 373 — Two-Sqrt × Fifty-Four-Log × polynomial numerator. */
function twoSqrtFiftyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 54) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 54) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 374 — Three-Sqrt × Fifty-Four-Log × polynomial numerator. */
function threeSqrtFiftyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 54) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 54) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 375 — Four-Sqrt × Fifty-Four-Log × polynomial numerator. */
function fourSqrtFiftyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 54) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 54) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 376 — Five-Sqrt × Fifty-Four-Log × polynomial numerator.
 * Completes the Fifty-Four-Log family (Phases 371–376).
 */
function fiveSqrtFiftyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 54) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 54) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 359 — Zero-Sqrt × Fifty-Two-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftyTwoLogPolyEffectiveDeg(num, k)`.
 */
function fiftyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 52) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 52) return undefined;
  return polyDeg;
}

/** Phase 358 — Five-Sqrt × Fifty-One-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFiftyOneLogPolyEffectiveDeg(num, k)`.
 * Completes the Fifty-One-Log family (Phases 353–358).
 */
function fiveSqrtFiftyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 51) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 51) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 357 — Four-Sqrt × Fifty-One-Log × polynomial numerator. */
function fourSqrtFiftyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 51) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 51) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 356 — Three-Sqrt × Fifty-One-Log × polynomial numerator. */
function threeSqrtFiftyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 51) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 51) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 355 — Two-Sqrt × Fifty-One-Log × polynomial numerator. */
function twoSqrtFiftyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 51) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 51) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 354 — One-Sqrt × Fifty-One-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFiftyOneLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFiftyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 51) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 51) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 353 — Zero-Sqrt × Fifty-One-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftyOneLogPolyEffectiveDeg(num, k)`.
 */
function fiftyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 51) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 51) return undefined;
  return polyDeg;
}

/** Phase 352 — Five-Sqrt × Fifty-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFiftyLogPolyEffectiveDeg(num, k)`.
 * Completes the Fifty-Log family (Phases 347–352).
 */
function fiveSqrtFiftyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 50) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 50) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 351 — Four-Sqrt × Fifty-Log × polynomial numerator. */
function fourSqrtFiftyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 50) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 50) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 350 — Three-Sqrt × Fifty-Log × polynomial numerator. */
function threeSqrtFiftyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 50) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 50) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 349 — Two-Sqrt × Fifty-Log × polynomial numerator. */
function twoSqrtFiftyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 50) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 50) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 348 — One-Sqrt × Fifty-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFiftyLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFiftyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 50) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 50) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 347 — Zero-Sqrt × Fifty-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fiftyLogPolyEffectiveDeg(num, k)`.
 */
function fiftyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 50) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 50) return undefined;
  return polyDeg;
}

/** Phase 346 — Five-Sqrt × Forty-Nine-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortyNineLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Nine-Log family (Phases 341–346).
 */
function fiveSqrtFortyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 49) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 49) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 345 — Four-Sqrt × Forty-Nine-Log × polynomial numerator. */
function fourSqrtFortyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 49) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 49) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 344 — Three-Sqrt × Forty-Nine-Log × polynomial numerator. */
function threeSqrtFortyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 49) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 49) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 343 — Two-Sqrt × Forty-Nine-Log × polynomial numerator. */
function twoSqrtFortyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 49) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 49) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 342 — One-Sqrt × Forty-Nine-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortyNineLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 49) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 49) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 341 — Zero-Sqrt × Forty-Nine-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortyNineLogPolyEffectiveDeg(num, k)`.
 */
function fortyNineLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 49) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 49) return undefined;
  return polyDeg;
}

/** Phase 340 — Five-Sqrt × Forty-Eight-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortyEightLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Eight-Log family (Phases 335–340).
 */
function fiveSqrtFortyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 48) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 48) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 339 — Four-Sqrt × Forty-Eight-Log × polynomial numerator. */
function fourSqrtFortyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 48) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 48) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 338 — Three-Sqrt × Forty-Eight-Log × polynomial numerator. */
function threeSqrtFortyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 48) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 48) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 337 — Two-Sqrt × Forty-Eight-Log × polynomial numerator. */
function twoSqrtFortyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 48) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 48) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 336 — One-Sqrt × Forty-Eight-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortyEightLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 48) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 48) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 335 — Zero-Sqrt × Forty-Eight-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortyEightLogPolyEffectiveDeg(num, k)`.
 */
function fortyEightLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 48) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 48) return undefined;
  return polyDeg;
}

/** Phase 334 — Five-Sqrt × Forty-Seven-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortySevenLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Seven-Log family (Phases 329–334).
 */
function fiveSqrtFortySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 47) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 47) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 333 — Four-Sqrt × Forty-Seven-Log × polynomial numerator. */
function fourSqrtFortySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 47) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 47) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 332 — Three-Sqrt × Forty-Seven-Log × polynomial numerator. */
function threeSqrtFortySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 47) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 47) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 331 — Two-Sqrt × Forty-Seven-Log × polynomial numerator. */
function twoSqrtFortySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 47) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 47) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 330 — One-Sqrt × Forty-Seven-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortySevenLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 47) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 47) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 329 — Zero-Sqrt × Forty-Seven-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortySevenLogPolyEffectiveDeg(num, k)`.
 */
function fortySevenLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 47) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 47) return undefined;
  return polyDeg;
}

/** Phase 328 — Five-Sqrt × Forty-Six-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortySixLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Six-Log family (Phases 323–328).
 */
function fiveSqrtFortySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 46) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 46) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 327 — Four-Sqrt × Forty-Six-Log × polynomial numerator. */
function fourSqrtFortySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 46) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 46) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 326 — Three-Sqrt × Forty-Six-Log × polynomial numerator. */
function threeSqrtFortySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 46) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 46) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 325 — Two-Sqrt × Forty-Six-Log × polynomial numerator. */
function twoSqrtFortySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 46) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 46) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 324 — One-Sqrt × Forty-Six-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortySixLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 46) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 46) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 323 — Zero-Sqrt × Forty-Six-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortySixLogPolyEffectiveDeg(num, k)`.
 */
function fortySixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 46) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 46) return undefined;
  return polyDeg;
}

/** Phase 322 — Five-Sqrt × Forty-Five-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortyFiveLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Five-Log family (Phases 317–322).
 */
function fiveSqrtFortyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 45) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 45) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 321 — Four-Sqrt × Forty-Five-Log × polynomial numerator. */
function fourSqrtFortyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 45) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 45) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 320 — Three-Sqrt × Forty-Five-Log × polynomial numerator. */
function threeSqrtFortyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 45) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 45) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 319 — Two-Sqrt × Forty-Five-Log × polynomial numerator. */
function twoSqrtFortyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 45) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 45) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 318 — One-Sqrt × Forty-Five-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortyFiveLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 45) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 45) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 317 — Zero-Sqrt × Forty-Five-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortyFiveLogPolyEffectiveDeg(num, k)`.
 */
function fortyFiveLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 45) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 45) return undefined;
  return polyDeg;
}

/** Phase 316 — Five-Sqrt × Forty-Four-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortyFourLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Four-Log family (Phases 311–316).
 */
function fiveSqrtFortyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 44) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 44) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 315 — Four-Sqrt × Forty-Four-Log × polynomial numerator. */
function fourSqrtFortyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 44) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 44) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 314 — Three-Sqrt × Forty-Four-Log × polynomial numerator. */
function threeSqrtFortyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 44) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 44) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 313 — Two-Sqrt × Forty-Four-Log × polynomial numerator. */
function twoSqrtFortyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(hd); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 44) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 44) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 312 — One-Sqrt × Forty-Four-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortyFourLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const hd = sqrtEffectiveHalfDegree(arg, k);
    if (hd !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = hd; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 44) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 44) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 311 — Zero-Sqrt × Forty-Four-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortyFourLogPolyEffectiveDeg(num, k)`.
 */
function fortyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // Sqrt present — refuse
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 44) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 44) return undefined;
  return polyDeg;
}

/** Phase 310 — Five-Sqrt × Forty-Three-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortyThreeLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-Three-Log family (Phases 305–310).
 */
function fiveSqrtFortyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 43) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 43) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 293 — Zero-Sqrt × Forty-One-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > fortyOneLogPolyEffectiveDeg(num, k)`.
 */
function fortyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined;
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 41) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 41) return undefined;
  return polyDeg;
}

/** Phase 294 — One-Sqrt × Forty-One-Log × polynomial numerator.
 * Caller checks `denDeg > oneSqrtFortyOneLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtFortyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDeg: number | undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDeg !== undefined) return undefined;
      sqrtHalfDeg = sh; continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 41) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 41) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 295 — Two-Sqrt × Forty-One-Log × polynomial numerator. */
function twoSqrtFortyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 2) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 41) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 41) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 296 — Three-Sqrt × Forty-One-Log × polynomial numerator. */
function threeSqrtFortyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 3) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 41) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 41) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 297 — Four-Sqrt × Forty-One-Log × polynomial numerator. */
function fourSqrtFortyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 4) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 41) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 41) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 298 — Five-Sqrt × Forty-One-Log × polynomial numerator.
 * Caller checks `denDeg > fiveSqrtFortyOneLogPolyEffectiveDeg(num, k)`.
 * Completes the Forty-One-Log family (Phases 293–298).
 */
function fiveSqrtFortyOneLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sh = sqrtEffectiveHalfDegree(arg, k);
    if (sh !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sh); continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 41) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 41) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 251 — Zero-Sqrt × Thirty-Four-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtyFourLogPolyEffectiveDeg(num, k)`.
 */
function thirtyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 34) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 34) return undefined;
  return polyDeg;
}

/** Phase 252 — One-Sqrt × Thirty-Four-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtThirtyFourLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 34) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 34) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 253 — Two-Sqrt × Thirty-Four-Log × polynomial numerator. */
function twoSqrtThirtyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 34) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 34) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 254 — Three-Sqrt × Thirty-Four-Log × polynomial numerator. */
function threeSqrtThirtyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 34) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 34) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 255 — Four-Sqrt × Thirty-Four-Log × polynomial numerator. */
function fourSqrtThirtyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 34) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 34) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 256 — Five-Sqrt × Thirty-Four-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtThirtyFourLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Four-Log family (Phases 251–256).
 */
function fiveSqrtThirtyFourLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 34) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 34) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 245 — Zero-Sqrt × Thirty-Three-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtyThreeLogPolyEffectiveDeg(num, k)`.
 */
function thirtyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 33) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 33) return undefined;
  return polyDeg;
}

/** Phase 246 — One-Sqrt × Thirty-Three-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtThirtyThreeLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 33) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 33) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 247 — Two-Sqrt × Thirty-Three-Log × polynomial numerator. */
function twoSqrtThirtyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 33) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 33) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 248 — Three-Sqrt × Thirty-Three-Log × polynomial numerator. */
function threeSqrtThirtyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 33) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 33) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 249 — Four-Sqrt × Thirty-Three-Log × polynomial numerator. */
function fourSqrtThirtyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 33) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 33) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 250 — Five-Sqrt × Thirty-Three-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtThirtyThreeLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Three-Log family (Phases 245–250).
 */
function fiveSqrtThirtyThreeLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 33) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 33) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 239 — Zero-Sqrt × Thirty-Two-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtyTwoLogPolyEffectiveDeg(num, k)`.
 */
function thirtyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 32) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 32) return undefined;
  return polyDeg;
}

/** Phase 240 — One-Sqrt × Thirty-Two-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtThirtyTwoLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 32) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 32) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 241 — Two-Sqrt × Thirty-Two-Log × polynomial numerator. */
function twoSqrtThirtyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 32) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 32) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 242 — Three-Sqrt × Thirty-Two-Log × polynomial numerator. */
function threeSqrtThirtyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 32) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 32) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 243 — Four-Sqrt × Thirty-Two-Log × polynomial numerator. */
function fourSqrtThirtyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 32) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 32) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 244 — Five-Sqrt × Thirty-Two-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtThirtyTwoLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Two-Log family (Phases 239–244).
 */
function fiveSqrtThirtyTwoLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 32) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 32) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + sqrtHalfDegs[4] + polyDeg;
}

/** Phase 227 — Zero-Sqrt × Thirty-Log × polynomial numerator.
 * effectiveDeg = polyDeg (no sqrt factors).
 * Caller checks `denDeg > thirtyLogPolyEffectiveDeg(num, k)`.
 */
function thirtyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    if (sqrtEffectiveHalfDegree(arg, k) !== undefined) return undefined; // no Sqrts allowed
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 30) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (logCount !== 30) return undefined;
  return polyDeg;
}

/** Phase 228 — One-Sqrt × Thirty-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg + polyDeg.
 * Caller checks `denDeg > oneSqrtThirtyLogPolyEffectiveDeg(num, k)`.
 */
function oneSqrtThirtyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 30) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDeg === undefined || logCount !== 30) return undefined;
  return sqrtHalfDeg + polyDeg;
}

/** Phase 229 — Two-Sqrt × Thirty-Log × polynomial numerator. */
function twoSqrtThirtyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 30) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 30) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
}

/** Phase 230 — Three-Sqrt × Thirty-Log × polynomial numerator. */
function threeSqrtThirtyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 30) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 3 || logCount !== 30) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + polyDeg;
}

/** Phase 231 — Four-Sqrt × Thirty-Log × polynomial numerator. */
function fourSqrtThirtyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
    if (isLogOfDivergingInK(arg, k)) { logCount++; if (logCount > 30) return undefined; continue; }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 4 || logCount !== 30) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + sqrtHalfDegs[2] + sqrtHalfDegs[3] + polyDeg;
}

/** Phase 232 — Five-Sqrt × Thirty-Log × polynomial numerator.
 * effectiveDeg = sqrtHalfDeg1+…+sqrtHalfDeg5 + polyDeg.
 * Caller checks `denDeg > fiveSqrtThirtyLogPolyEffectiveDeg(num, k)`.
 * Completes the Thirty-Log family (Phases 227–232).
 */
function fiveSqrtThirtyLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  const sqrtHalfDegs: number[] = [];
  let logCount = 0;
  let polyDeg = 0;
  for (const arg of node.args) {
    const sd = sqrtEffectiveHalfDegree(arg, k);
    if (sd !== undefined) {
      if (sqrtHalfDegs.length >= 5) return undefined;
      sqrtHalfDegs.push(sd);
      continue;
    }
    if (isLogOfDivergingInK(arg, k)) {
      logCount++;
      if (logCount > 30) return undefined;
      continue;
    }
    const d = polynomialDegreeInK(arg, k);
    if (d !== undefined) { polyDeg += d; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 5 || logCount !== 30) return undefined;
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

/**
 * Phase 85 — Two-Sqrt × Six-Log × polynomial numerator.
 *
 * Effective growth: `sqrt(k^a) · sqrt(k^b) · log(k)⁶ · k^m ≈ k^{(a+b)/2} · log⁶(k) · k^m`.
 * `log⁶(k)` is sub-polynomial (`o(k^ε)`), contributing 0 to effective polynomial degree.
 * effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
 * Caller checks `denDeg > twoSqrtSixLogPolyEffectiveDeg(num, k)`.
 */
function twoSqrtSixLogPolyEffectiveDeg(node: IRNode, k: IRNode): number | undefined {
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
      if (logCount > 6) return undefined;
      continue;
    }
    const deg = polynomialDegreeInK(arg, k);
    if (deg !== undefined) { polyDeg += deg; continue; }
    if (isBoundedInK(arg, k)) continue;
    return undefined;
  }
  if (sqrtHalfDegs.length !== 2 || logCount !== 6) return undefined;
  return sqrtHalfDegs[0] + sqrtHalfDegs[1] + polyDeg;
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
  // Phase 85: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×6, polynomial..., bounded...) numerator.
  // Two Sqrt + six Log factors; log⁶ sub-polynomial → effective degree = sqrtHalfDeg1 + sqrtHalfDeg2 + polyDeg.
  // Closes when denDeg > twoSqrtSixLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l6Deg = twoSqrtSixLogPolyEffectiveDeg(num, k);
  if (s2l6Deg !== undefined) {
    const denDegS2l6 = polynomialDegreeInK(den, k);
    if (denDegS2l6 !== undefined) {
      if (denDegS2l6 > s2l6Deg) return true;
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
  // Phase 406: five sqrt + 59 logs
  const s5l59Deg = fiveSqrtFiftyNineLogPolyEffectiveDeg(num, k);
  if (s5l59Deg !== undefined) {
    const denDegS5l59 = polynomialDegreeInK(den, k);
    if (denDegS5l59 !== undefined) {
      if (denDegS5l59 > s5l59Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 405: four sqrt + 59 logs
  const s4l59Deg = fourSqrtFiftyNineLogPolyEffectiveDeg(num, k);
  if (s4l59Deg !== undefined) {
    const denDegS4l59 = polynomialDegreeInK(den, k);
    if (denDegS4l59 !== undefined) {
      if (denDegS4l59 > s4l59Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 404: three sqrt + 59 logs
  const s3l59Deg = threeSqrtFiftyNineLogPolyEffectiveDeg(num, k);
  if (s3l59Deg !== undefined) {
    const denDegS3l59 = polynomialDegreeInK(den, k);
    if (denDegS3l59 !== undefined) {
      if (denDegS3l59 > s3l59Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 403: two sqrt + 59 logs
  const s2l59Deg = twoSqrtFiftyNineLogPolyEffectiveDeg(num, k);
  if (s2l59Deg !== undefined) {
    const denDegS2l59 = polynomialDegreeInK(den, k);
    if (denDegS2l59 !== undefined) {
      if (denDegS2l59 > s2l59Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 402: one sqrt + 59 logs
  const s1l59Deg = oneSqrtFiftyNineLogPolyEffectiveDeg(num, k);
  if (s1l59Deg !== undefined) {
    const denDegS1l59 = polynomialDegreeInK(den, k);
    if (denDegS1l59 !== undefined) {
      if (denDegS1l59 > s1l59Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 401: zero sqrt + 59 logs
  const sl59Deg = fiftyNineLogPolyEffectiveDeg(num, k);
  if (sl59Deg !== undefined) {
    const denDegSl59 = polynomialDegreeInK(den, k);
    if (denDegSl59 !== undefined) {
      if (denDegSl59 > sl59Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 400: five sqrt + 58 logs
  const s5l58Deg = fiveSqrtFiftyEightLogPolyEffectiveDeg(num, k);
  if (s5l58Deg !== undefined) {
    const denDegS5l58 = polynomialDegreeInK(den, k);
    if (denDegS5l58 !== undefined) {
      if (denDegS5l58 > s5l58Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 399: four sqrt + 58 logs
  const s4l58Deg = fourSqrtFiftyEightLogPolyEffectiveDeg(num, k);
  if (s4l58Deg !== undefined) {
    const denDegS4l58 = polynomialDegreeInK(den, k);
    if (denDegS4l58 !== undefined) {
      if (denDegS4l58 > s4l58Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 398: three sqrt + 58 logs
  const s3l58Deg = threeSqrtFiftyEightLogPolyEffectiveDeg(num, k);
  if (s3l58Deg !== undefined) {
    const denDegS3l58 = polynomialDegreeInK(den, k);
    if (denDegS3l58 !== undefined) {
      if (denDegS3l58 > s3l58Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 397: two sqrt + 58 logs
  const s2l58Deg = twoSqrtFiftyEightLogPolyEffectiveDeg(num, k);
  if (s2l58Deg !== undefined) {
    const denDegS2l58 = polynomialDegreeInK(den, k);
    if (denDegS2l58 !== undefined) {
      if (denDegS2l58 > s2l58Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 396: one sqrt + 58 logs
  const s1l58Deg = oneSqrtFiftyEightLogPolyEffectiveDeg(num, k);
  if (s1l58Deg !== undefined) {
    const denDegS1l58 = polynomialDegreeInK(den, k);
    if (denDegS1l58 !== undefined) {
      if (denDegS1l58 > s1l58Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 395: zero sqrt + 58 logs
  const s0l58Deg = fiftyEightLogPolyEffectiveDeg(num, k);
  if (s0l58Deg !== undefined) {
    const denDegS0l58 = polynomialDegreeInK(den, k);
    if (denDegS0l58 !== undefined) {
      if (denDegS0l58 > s0l58Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 394: five sqrt + 57 logs
  const s5l57Deg = fiveSqrtFiftySevenLogPolyEffectiveDeg(num, k);
  if (s5l57Deg !== undefined) {
    const denDegS5l57 = polynomialDegreeInK(den, k);
    if (denDegS5l57 !== undefined) {
      if (denDegS5l57 > s5l57Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 393: four sqrt + 57 logs
  const s4l57Deg = fourSqrtFiftySevenLogPolyEffectiveDeg(num, k);
  if (s4l57Deg !== undefined) {
    const denDegS4l57 = polynomialDegreeInK(den, k);
    if (denDegS4l57 !== undefined) {
      if (denDegS4l57 > s4l57Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 392: three sqrt + 57 logs
  const s3l57Deg = threeSqrtFiftySevenLogPolyEffectiveDeg(num, k);
  if (s3l57Deg !== undefined) {
    const denDegS3l57 = polynomialDegreeInK(den, k);
    if (denDegS3l57 !== undefined) {
      if (denDegS3l57 > s3l57Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 391: two sqrt + 57 logs
  const s2l57Deg = twoSqrtFiftySevenLogPolyEffectiveDeg(num, k);
  if (s2l57Deg !== undefined) {
    const denDegS2l57 = polynomialDegreeInK(den, k);
    if (denDegS2l57 !== undefined) {
      if (denDegS2l57 > s2l57Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 390: one sqrt + 57 logs
  const s1l57Deg = oneSqrtFiftySevenLogPolyEffectiveDeg(num, k);
  if (s1l57Deg !== undefined) {
    const denDegS1l57 = polynomialDegreeInK(den, k);
    if (denDegS1l57 !== undefined) {
      if (denDegS1l57 > s1l57Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 389: zero sqrt + 57 logs
  const s0l57Deg = fiftySevenLogPolyEffectiveDeg(num, k);
  if (s0l57Deg !== undefined) {
    const denDegS0l57 = polynomialDegreeInK(den, k);
    if (denDegS0l57 !== undefined) {
      if (denDegS0l57 > s0l57Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 388: five sqrt + 56 logs
  const s5l56Deg = fiveSqrtFiftySixLogPolyEffectiveDeg(num, k);
  if (s5l56Deg !== undefined) {
    const denDegS5l56 = polynomialDegreeInK(den, k);
    if (denDegS5l56 !== undefined) {
      if (denDegS5l56 > s5l56Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 387: four sqrt + 56 logs
  const s4l56Deg = fourSqrtFiftySixLogPolyEffectiveDeg(num, k);
  if (s4l56Deg !== undefined) {
    const denDegS4l56 = polynomialDegreeInK(den, k);
    if (denDegS4l56 !== undefined) {
      if (denDegS4l56 > s4l56Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 386: three sqrt + 56 logs
  const s3l56Deg = threeSqrtFiftySixLogPolyEffectiveDeg(num, k);
  if (s3l56Deg !== undefined) {
    const denDegS3l56 = polynomialDegreeInK(den, k);
    if (denDegS3l56 !== undefined) {
      if (denDegS3l56 > s3l56Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 385: two sqrt + 56 logs
  const s2l56Deg = twoSqrtFiftySixLogPolyEffectiveDeg(num, k);
  if (s2l56Deg !== undefined) {
    const denDegS2l56 = polynomialDegreeInK(den, k);
    if (denDegS2l56 !== undefined) {
      if (denDegS2l56 > s2l56Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 384: one sqrt + 56 logs
  const s1l56Deg = oneSqrtFiftySixLogPolyEffectiveDeg(num, k);
  if (s1l56Deg !== undefined) {
    const denDegS1l56 = polynomialDegreeInK(den, k);
    if (denDegS1l56 !== undefined) {
      if (denDegS1l56 > s1l56Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 383: zero sqrt + 56 logs
  const s0l56Deg = fiftySixLogPolyEffectiveDeg(num, k);
  if (s0l56Deg !== undefined) {
    const denDegS0l56 = polynomialDegreeInK(den, k);
    if (denDegS0l56 !== undefined) {
      if (denDegS0l56 > s0l56Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 382: five sqrt + 55 logs
  const s5l55Deg = fiveSqrtFiftyFiveLogPolyEffectiveDeg(num, k);
  if (s5l55Deg !== undefined) {
    const denDegS5l55 = polynomialDegreeInK(den, k);
    if (denDegS5l55 !== undefined) {
      if (denDegS5l55 > s5l55Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 381: four sqrt + 55 logs
  const s4l55Deg = fourSqrtFiftyFiveLogPolyEffectiveDeg(num, k);
  if (s4l55Deg !== undefined) {
    const denDegS4l55 = polynomialDegreeInK(den, k);
    if (denDegS4l55 !== undefined) {
      if (denDegS4l55 > s4l55Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 380: three sqrt + 55 logs
  const s3l55Deg = threeSqrtFiftyFiveLogPolyEffectiveDeg(num, k);
  if (s3l55Deg !== undefined) {
    const denDegS3l55 = polynomialDegreeInK(den, k);
    if (denDegS3l55 !== undefined) {
      if (denDegS3l55 > s3l55Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 379: two sqrt + 55 logs
  const s2l55Deg = twoSqrtFiftyFiveLogPolyEffectiveDeg(num, k);
  if (s2l55Deg !== undefined) {
    const denDegS2l55 = polynomialDegreeInK(den, k);
    if (denDegS2l55 !== undefined) {
      if (denDegS2l55 > s2l55Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 378: one sqrt + 55 logs
  const s1l55Deg = oneSqrtFiftyFiveLogPolyEffectiveDeg(num, k);
  if (s1l55Deg !== undefined) {
    const denDegS1l55 = polynomialDegreeInK(den, k);
    if (denDegS1l55 !== undefined) {
      if (denDegS1l55 > s1l55Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 377: zero sqrt + 55 logs
  const s0l55Deg = fiftyFiveLogPolyEffectiveDeg(num, k);
  if (s0l55Deg !== undefined) {
    const denDegS0l55 = polynomialDegreeInK(den, k);
    if (denDegS0l55 !== undefined) {
      if (denDegS0l55 > s0l55Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 376: five sqrt + 54 logs
  const s5l54Deg = fiveSqrtFiftyFourLogPolyEffectiveDeg(num, k);
  if (s5l54Deg !== undefined) {
    const denDegS5l54 = polynomialDegreeInK(den, k);
    if (denDegS5l54 !== undefined) {
      if (denDegS5l54 > s5l54Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 375: four sqrt + 54 logs
  const s4l54Deg = fourSqrtFiftyFourLogPolyEffectiveDeg(num, k);
  if (s4l54Deg !== undefined) {
    const denDegS4l54 = polynomialDegreeInK(den, k);
    if (denDegS4l54 !== undefined) {
      if (denDegS4l54 > s4l54Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 374: three sqrt + 54 logs
  const s3l54Deg = threeSqrtFiftyFourLogPolyEffectiveDeg(num, k);
  if (s3l54Deg !== undefined) {
    const denDegS3l54 = polynomialDegreeInK(den, k);
    if (denDegS3l54 !== undefined) {
      if (denDegS3l54 > s3l54Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 373: two sqrt + 54 logs
  const s2l54Deg = twoSqrtFiftyFourLogPolyEffectiveDeg(num, k);
  if (s2l54Deg !== undefined) {
    const denDegS2l54 = polynomialDegreeInK(den, k);
    if (denDegS2l54 !== undefined) {
      if (denDegS2l54 > s2l54Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 372: one sqrt + 54 logs
  const s1l54Deg = oneSqrtFiftyFourLogPolyEffectiveDeg(num, k);
  if (s1l54Deg !== undefined) {
    const denDegS1l54 = polynomialDegreeInK(den, k);
    if (denDegS1l54 !== undefined) {
      if (denDegS1l54 > s1l54Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 371: zero sqrt + 54 logs
  const s0l54Deg = fiftyFourLogPolyEffectiveDeg(num, k);
  if (s0l54Deg !== undefined) {
    const denDegS0l54 = polynomialDegreeInK(den, k);
    if (denDegS0l54 !== undefined) {
      if (denDegS0l54 > s0l54Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 370: five sqrt + 53 logs
  const s5l53Deg = fiveSqrtFiftyThreeLogPolyEffectiveDeg(num, k);
  if (s5l53Deg !== undefined) {
    const denDegS5l53 = polynomialDegreeInK(den, k);
    if (denDegS5l53 !== undefined) {
      if (denDegS5l53 > s5l53Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 369: four sqrt + 53 logs
  const s4l53Deg = fourSqrtFiftyThreeLogPolyEffectiveDeg(num, k);
  if (s4l53Deg !== undefined) {
    const denDegS4l53 = polynomialDegreeInK(den, k);
    if (denDegS4l53 !== undefined) {
      if (denDegS4l53 > s4l53Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 368: three sqrt + 53 logs
  const s3l53Deg = threeSqrtFiftyThreeLogPolyEffectiveDeg(num, k);
  if (s3l53Deg !== undefined) {
    const denDegS3l53 = polynomialDegreeInK(den, k);
    if (denDegS3l53 !== undefined) {
      if (denDegS3l53 > s3l53Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 367: two sqrt + 53 logs
  const s2l53Deg = twoSqrtFiftyThreeLogPolyEffectiveDeg(num, k);
  if (s2l53Deg !== undefined) {
    const denDegS2l53 = polynomialDegreeInK(den, k);
    if (denDegS2l53 !== undefined) {
      if (denDegS2l53 > s2l53Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 366: one sqrt + 53 logs
  const s1l53Deg = oneSqrtFiftyThreeLogPolyEffectiveDeg(num, k);
  if (s1l53Deg !== undefined) {
    const denDegS1l53 = polynomialDegreeInK(den, k);
    if (denDegS1l53 !== undefined) {
      if (denDegS1l53 > s1l53Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 365: zero sqrt + 53 logs
  const s0l53Deg = fiftyThreeLogPolyEffectiveDeg(num, k);
  if (s0l53Deg !== undefined) {
    const denDegS0l53 = polynomialDegreeInK(den, k);
    if (denDegS0l53 !== undefined) {
      if (denDegS0l53 > s0l53Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 364: five sqrt + 52 logs
  const s5l52Deg = fiveSqrtFiftyTwoLogPolyEffectiveDeg(num, k);
  if (s5l52Deg !== undefined) {
    const denDegS5l52 = polynomialDegreeInK(den, k);
    if (denDegS5l52 !== undefined) {
      if (denDegS5l52 > s5l52Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 363: four sqrt + 52 logs
  const s4l52Deg = fourSqrtFiftyTwoLogPolyEffectiveDeg(num, k);
  if (s4l52Deg !== undefined) {
    const denDegS4l52 = polynomialDegreeInK(den, k);
    if (denDegS4l52 !== undefined) {
      if (denDegS4l52 > s4l52Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 362: three sqrt + 52 logs
  const s3l52Deg = threeSqrtFiftyTwoLogPolyEffectiveDeg(num, k);
  if (s3l52Deg !== undefined) {
    const denDegS3l52 = polynomialDegreeInK(den, k);
    if (denDegS3l52 !== undefined) {
      if (denDegS3l52 > s3l52Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 361: two sqrt + 52 logs
  const s2l52Deg = twoSqrtFiftyTwoLogPolyEffectiveDeg(num, k);
  if (s2l52Deg !== undefined) {
    const denDegS2l52 = polynomialDegreeInK(den, k);
    if (denDegS2l52 !== undefined) {
      if (denDegS2l52 > s2l52Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 360: one sqrt + 52 logs
  const s1l52Deg = oneSqrtFiftyTwoLogPolyEffectiveDeg(num, k);
  if (s1l52Deg !== undefined) {
    const denDegS1l52 = polynomialDegreeInK(den, k);
    if (denDegS1l52 !== undefined) {
      if (denDegS1l52 > s1l52Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 359: zero sqrt + 52 logs
  const s0l52Deg = fiftyTwoLogPolyEffectiveDeg(num, k);
  if (s0l52Deg !== undefined) {
    const denDegS0l52 = polynomialDegreeInK(den, k);
    if (denDegS0l52 !== undefined) {
      if (denDegS0l52 > s0l52Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 358: five sqrt + 51 logs
  const s5l51Deg = fiveSqrtFiftyOneLogPolyEffectiveDeg(num, k);
  if (s5l51Deg !== undefined) {
    const denDegS5l51 = polynomialDegreeInK(den, k);
    if (denDegS5l51 !== undefined) {
      if (denDegS5l51 > s5l51Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 357: four sqrt + 51 logs
  const s4l51Deg = fourSqrtFiftyOneLogPolyEffectiveDeg(num, k);
  if (s4l51Deg !== undefined) {
    const denDegS4l51 = polynomialDegreeInK(den, k);
    if (denDegS4l51 !== undefined) {
      if (denDegS4l51 > s4l51Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 356: three sqrt + 51 logs
  const s3l51Deg = threeSqrtFiftyOneLogPolyEffectiveDeg(num, k);
  if (s3l51Deg !== undefined) {
    const denDegS3l51 = polynomialDegreeInK(den, k);
    if (denDegS3l51 !== undefined) {
      if (denDegS3l51 > s3l51Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 355: two sqrt + 51 logs
  const s2l51Deg = twoSqrtFiftyOneLogPolyEffectiveDeg(num, k);
  if (s2l51Deg !== undefined) {
    const denDegS2l51 = polynomialDegreeInK(den, k);
    if (denDegS2l51 !== undefined) {
      if (denDegS2l51 > s2l51Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 354: one sqrt + 51 logs
  const s1l51Deg = oneSqrtFiftyOneLogPolyEffectiveDeg(num, k);
  if (s1l51Deg !== undefined) {
    const denDegS1l51 = polynomialDegreeInK(den, k);
    if (denDegS1l51 !== undefined) {
      if (denDegS1l51 > s1l51Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 353: zero sqrt + 51 logs
  const s0l51Deg = fiftyOneLogPolyEffectiveDeg(num, k);
  if (s0l51Deg !== undefined) {
    const denDegS0l51 = polynomialDegreeInK(den, k);
    if (denDegS0l51 !== undefined) {
      if (denDegS0l51 > s0l51Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 352: five sqrt + 50 logs
  const s5l50Deg = fiveSqrtFiftyLogPolyEffectiveDeg(num, k);
  if (s5l50Deg !== undefined) {
    const denDegS5l50 = polynomialDegreeInK(den, k);
    if (denDegS5l50 !== undefined) {
      if (denDegS5l50 > s5l50Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 351: four sqrt + 50 logs
  const s4l50Deg = fourSqrtFiftyLogPolyEffectiveDeg(num, k);
  if (s4l50Deg !== undefined) {
    const denDegS4l50 = polynomialDegreeInK(den, k);
    if (denDegS4l50 !== undefined) {
      if (denDegS4l50 > s4l50Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 350: three sqrt + 50 logs
  const s3l50Deg = threeSqrtFiftyLogPolyEffectiveDeg(num, k);
  if (s3l50Deg !== undefined) {
    const denDegS3l50 = polynomialDegreeInK(den, k);
    if (denDegS3l50 !== undefined) {
      if (denDegS3l50 > s3l50Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 349: two sqrt + 50 logs
  const s2l50Deg = twoSqrtFiftyLogPolyEffectiveDeg(num, k);
  if (s2l50Deg !== undefined) {
    const denDegS2l50 = polynomialDegreeInK(den, k);
    if (denDegS2l50 !== undefined) {
      if (denDegS2l50 > s2l50Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 348: one sqrt + 50 logs
  const s1l50Deg = oneSqrtFiftyLogPolyEffectiveDeg(num, k);
  if (s1l50Deg !== undefined) {
    const denDegS1l50 = polynomialDegreeInK(den, k);
    if (denDegS1l50 !== undefined) {
      if (denDegS1l50 > s1l50Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 347: zero sqrt + 50 logs
  const s0l50Deg = fiftyLogPolyEffectiveDeg(num, k);
  if (s0l50Deg !== undefined) {
    const denDegS0l50 = polynomialDegreeInK(den, k);
    if (denDegS0l50 !== undefined) {
      if (denDegS0l50 > s0l50Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 346: five sqrt + 49 logs
  const s5l49Deg = fiveSqrtFortyNineLogPolyEffectiveDeg(num, k);
  if (s5l49Deg !== undefined) {
    const denDegS5l49 = polynomialDegreeInK(den, k);
    if (denDegS5l49 !== undefined) {
      if (denDegS5l49 > s5l49Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 345: four sqrt + 49 logs
  const s4l49Deg = fourSqrtFortyNineLogPolyEffectiveDeg(num, k);
  if (s4l49Deg !== undefined) {
    const denDegS4l49 = polynomialDegreeInK(den, k);
    if (denDegS4l49 !== undefined) {
      if (denDegS4l49 > s4l49Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 344: three sqrt + 49 logs
  const s3l49Deg = threeSqrtFortyNineLogPolyEffectiveDeg(num, k);
  if (s3l49Deg !== undefined) {
    const denDegS3l49 = polynomialDegreeInK(den, k);
    if (denDegS3l49 !== undefined) {
      if (denDegS3l49 > s3l49Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 343: two sqrt + 49 logs
  const s2l49Deg = twoSqrtFortyNineLogPolyEffectiveDeg(num, k);
  if (s2l49Deg !== undefined) {
    const denDegS2l49 = polynomialDegreeInK(den, k);
    if (denDegS2l49 !== undefined) {
      if (denDegS2l49 > s2l49Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 342: one sqrt + 49 logs
  const s1l49Deg = oneSqrtFortyNineLogPolyEffectiveDeg(num, k);
  if (s1l49Deg !== undefined) {
    const denDegS1l49 = polynomialDegreeInK(den, k);
    if (denDegS1l49 !== undefined) {
      if (denDegS1l49 > s1l49Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 341: zero sqrt + 49 logs
  const s0l49Deg = fortyNineLogPolyEffectiveDeg(num, k);
  if (s0l49Deg !== undefined) {
    const denDegS0l49 = polynomialDegreeInK(den, k);
    if (denDegS0l49 !== undefined) {
      if (denDegS0l49 > s0l49Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 340: five sqrt + 48 logs
  const s5l48Deg = fiveSqrtFortyEightLogPolyEffectiveDeg(num, k);
  if (s5l48Deg !== undefined) {
    const denDegS5l48 = polynomialDegreeInK(den, k);
    if (denDegS5l48 !== undefined) {
      if (denDegS5l48 > s5l48Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 339: four sqrt + 48 logs
  const s4l48Deg = fourSqrtFortyEightLogPolyEffectiveDeg(num, k);
  if (s4l48Deg !== undefined) {
    const denDegS4l48 = polynomialDegreeInK(den, k);
    if (denDegS4l48 !== undefined) {
      if (denDegS4l48 > s4l48Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 338: three sqrt + 48 logs
  const s3l48Deg = threeSqrtFortyEightLogPolyEffectiveDeg(num, k);
  if (s3l48Deg !== undefined) {
    const denDegS3l48 = polynomialDegreeInK(den, k);
    if (denDegS3l48 !== undefined) {
      if (denDegS3l48 > s3l48Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 337: two sqrt + 48 logs
  const s2l48Deg = twoSqrtFortyEightLogPolyEffectiveDeg(num, k);
  if (s2l48Deg !== undefined) {
    const denDegS2l48 = polynomialDegreeInK(den, k);
    if (denDegS2l48 !== undefined) {
      if (denDegS2l48 > s2l48Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 336: one sqrt + 48 logs
  const s1l48Deg = oneSqrtFortyEightLogPolyEffectiveDeg(num, k);
  if (s1l48Deg !== undefined) {
    const denDegS1l48 = polynomialDegreeInK(den, k);
    if (denDegS1l48 !== undefined) {
      if (denDegS1l48 > s1l48Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 335: zero sqrt + 48 logs
  const s0l48Deg = fortyEightLogPolyEffectiveDeg(num, k);
  if (s0l48Deg !== undefined) {
    const denDegS0l48 = polynomialDegreeInK(den, k);
    if (denDegS0l48 !== undefined) {
      if (denDegS0l48 > s0l48Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 334: five sqrt + 47 logs
  const s5l47Deg = fiveSqrtFortySevenLogPolyEffectiveDeg(num, k);
  if (s5l47Deg !== undefined) {
    const denDegS5l47 = polynomialDegreeInK(den, k);
    if (denDegS5l47 !== undefined) {
      if (denDegS5l47 > s5l47Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 333: four sqrt + 47 logs
  const s4l47Deg = fourSqrtFortySevenLogPolyEffectiveDeg(num, k);
  if (s4l47Deg !== undefined) {
    const denDegS4l47 = polynomialDegreeInK(den, k);
    if (denDegS4l47 !== undefined) {
      if (denDegS4l47 > s4l47Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 332: three sqrt + 47 logs
  const s3l47Deg = threeSqrtFortySevenLogPolyEffectiveDeg(num, k);
  if (s3l47Deg !== undefined) {
    const denDegS3l47 = polynomialDegreeInK(den, k);
    if (denDegS3l47 !== undefined) {
      if (denDegS3l47 > s3l47Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 331: two sqrt + 47 logs
  const s2l47Deg = twoSqrtFortySevenLogPolyEffectiveDeg(num, k);
  if (s2l47Deg !== undefined) {
    const denDegS2l47 = polynomialDegreeInK(den, k);
    if (denDegS2l47 !== undefined) {
      if (denDegS2l47 > s2l47Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 330: one sqrt + 47 logs
  const s1l47Deg = oneSqrtFortySevenLogPolyEffectiveDeg(num, k);
  if (s1l47Deg !== undefined) {
    const denDegS1l47 = polynomialDegreeInK(den, k);
    if (denDegS1l47 !== undefined) {
      if (denDegS1l47 > s1l47Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 329: zero sqrt + 47 logs
  const s0l47Deg = fortySevenLogPolyEffectiveDeg(num, k);
  if (s0l47Deg !== undefined) {
    const denDegS0l47 = polynomialDegreeInK(den, k);
    if (denDegS0l47 !== undefined) {
      if (denDegS0l47 > s0l47Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 328: five sqrt + 46 logs
  const s5l46Deg = fiveSqrtFortySixLogPolyEffectiveDeg(num, k);
  if (s5l46Deg !== undefined) {
    const denDegS5l46 = polynomialDegreeInK(den, k);
    if (denDegS5l46 !== undefined) {
      if (denDegS5l46 > s5l46Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 327: four sqrt + 46 logs
  const s4l46Deg = fourSqrtFortySixLogPolyEffectiveDeg(num, k);
  if (s4l46Deg !== undefined) {
    const denDegS4l46 = polynomialDegreeInK(den, k);
    if (denDegS4l46 !== undefined) {
      if (denDegS4l46 > s4l46Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 326: three sqrt + 46 logs
  const s3l46Deg = threeSqrtFortySixLogPolyEffectiveDeg(num, k);
  if (s3l46Deg !== undefined) {
    const denDegS3l46 = polynomialDegreeInK(den, k);
    if (denDegS3l46 !== undefined) {
      if (denDegS3l46 > s3l46Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 325: two sqrt + 46 logs
  const s2l46Deg = twoSqrtFortySixLogPolyEffectiveDeg(num, k);
  if (s2l46Deg !== undefined) {
    const denDegS2l46 = polynomialDegreeInK(den, k);
    if (denDegS2l46 !== undefined) {
      if (denDegS2l46 > s2l46Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 324: one sqrt + 46 logs
  const s1l46Deg = oneSqrtFortySixLogPolyEffectiveDeg(num, k);
  if (s1l46Deg !== undefined) {
    const denDegS1l46 = polynomialDegreeInK(den, k);
    if (denDegS1l46 !== undefined) {
      if (denDegS1l46 > s1l46Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 323: zero sqrt + 46 logs
  const s0l46Deg = fortySixLogPolyEffectiveDeg(num, k);
  if (s0l46Deg !== undefined) {
    const denDegS0l46 = polynomialDegreeInK(den, k);
    if (denDegS0l46 !== undefined) {
      if (denDegS0l46 > s0l46Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 322: five sqrt + 45 logs
  const s5l45Deg = fiveSqrtFortyFiveLogPolyEffectiveDeg(num, k);
  if (s5l45Deg !== undefined) {
    const denDegS5l45 = polynomialDegreeInK(den, k);
    if (denDegS5l45 !== undefined) {
      if (denDegS5l45 > s5l45Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 321: four sqrt + 45 logs
  const s4l45Deg = fourSqrtFortyFiveLogPolyEffectiveDeg(num, k);
  if (s4l45Deg !== undefined) {
    const denDegS4l45 = polynomialDegreeInK(den, k);
    if (denDegS4l45 !== undefined) {
      if (denDegS4l45 > s4l45Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 320: three sqrt + 45 logs
  const s3l45Deg = threeSqrtFortyFiveLogPolyEffectiveDeg(num, k);
  if (s3l45Deg !== undefined) {
    const denDegS3l45 = polynomialDegreeInK(den, k);
    if (denDegS3l45 !== undefined) {
      if (denDegS3l45 > s3l45Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 319: two sqrt + 45 logs
  const s2l45Deg = twoSqrtFortyFiveLogPolyEffectiveDeg(num, k);
  if (s2l45Deg !== undefined) {
    const denDegS2l45 = polynomialDegreeInK(den, k);
    if (denDegS2l45 !== undefined) {
      if (denDegS2l45 > s2l45Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 318: one sqrt + 45 logs
  const s1l45Deg = oneSqrtFortyFiveLogPolyEffectiveDeg(num, k);
  if (s1l45Deg !== undefined) {
    const denDegS1l45 = polynomialDegreeInK(den, k);
    if (denDegS1l45 !== undefined) {
      if (denDegS1l45 > s1l45Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 317: zero sqrt + 45 logs
  const s0l45Deg = fortyFiveLogPolyEffectiveDeg(num, k);
  if (s0l45Deg !== undefined) {
    const denDegS0l45 = polynomialDegreeInK(den, k);
    if (denDegS0l45 !== undefined) {
      if (denDegS0l45 > s0l45Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 316: five sqrt + 44 logs
  const s5l44Deg = fiveSqrtFortyFourLogPolyEffectiveDeg(num, k);
  if (s5l44Deg !== undefined) {
    const denDegS5l44 = polynomialDegreeInK(den, k);
    if (denDegS5l44 !== undefined) {
      if (denDegS5l44 > s5l44Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 315: four sqrt + 44 logs
  const s4l44Deg = fourSqrtFortyFourLogPolyEffectiveDeg(num, k);
  if (s4l44Deg !== undefined) {
    const denDegS4l44 = polynomialDegreeInK(den, k);
    if (denDegS4l44 !== undefined) {
      if (denDegS4l44 > s4l44Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 314: three sqrt + 44 logs
  const s3l44Deg = threeSqrtFortyFourLogPolyEffectiveDeg(num, k);
  if (s3l44Deg !== undefined) {
    const denDegS3l44 = polynomialDegreeInK(den, k);
    if (denDegS3l44 !== undefined) {
      if (denDegS3l44 > s3l44Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 313: two sqrt + 44 logs
  const s2l44Deg = twoSqrtFortyFourLogPolyEffectiveDeg(num, k);
  if (s2l44Deg !== undefined) {
    const denDegS2l44 = polynomialDegreeInK(den, k);
    if (denDegS2l44 !== undefined) {
      if (denDegS2l44 > s2l44Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 312: one sqrt + 44 logs
  const s1l44Deg = oneSqrtFortyFourLogPolyEffectiveDeg(num, k);
  if (s1l44Deg !== undefined) {
    const denDegS1l44 = polynomialDegreeInK(den, k);
    if (denDegS1l44 !== undefined) {
      if (denDegS1l44 > s1l44Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 311: zero sqrt + 44 logs
  const s0l44Deg = fortyFourLogPolyEffectiveDeg(num, k);
  if (s0l44Deg !== undefined) {
    const denDegS0l44 = polynomialDegreeInK(den, k);
    if (denDegS0l44 !== undefined) {
      if (denDegS0l44 > s0l44Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 310: five sqrt + 43 logs
  const s5l43Deg = fiveSqrtFortyThreeLogPolyEffectiveDeg(num, k);
  if (s5l43Deg !== undefined) {
    const denDegS5l43 = polynomialDegreeInK(den, k);
    if (denDegS5l43 !== undefined) {
      if (denDegS5l43 > s5l43Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 309: four sqrt + 43 logs
  const s4l43Deg = fourSqrtFortyThreeLogPolyEffectiveDeg(num, k);
  if (s4l43Deg !== undefined) {
    const denDegS4l43 = polynomialDegreeInK(den, k);
    if (denDegS4l43 !== undefined) {
      if (denDegS4l43 > s4l43Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 308: three sqrt + 43 logs
  const s3l43Deg = threeSqrtFortyThreeLogPolyEffectiveDeg(num, k);
  if (s3l43Deg !== undefined) {
    const denDegS3l43 = polynomialDegreeInK(den, k);
    if (denDegS3l43 !== undefined) {
      if (denDegS3l43 > s3l43Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 307: two sqrt + 43 logs
  const s2l43Deg = twoSqrtFortyThreeLogPolyEffectiveDeg(num, k);
  if (s2l43Deg !== undefined) {
    const denDegS2l43 = polynomialDegreeInK(den, k);
    if (denDegS2l43 !== undefined) {
      if (denDegS2l43 > s2l43Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 306: one sqrt + 43 logs
  const s1l43Deg = oneSqrtFortyThreeLogPolyEffectiveDeg(num, k);
  if (s1l43Deg !== undefined) {
    const denDegS1l43 = polynomialDegreeInK(den, k);
    if (denDegS1l43 !== undefined) {
      if (denDegS1l43 > s1l43Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 305: zero sqrt + 43 logs
  const s0l43Deg = fortyThreeLogPolyEffectiveDeg(num, k);
  if (s0l43Deg !== undefined) {
    const denDegS0l43 = polynomialDegreeInK(den, k);
    if (denDegS0l43 !== undefined) {
      if (denDegS0l43 > s0l43Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 304: five sqrt + 42 logs
  const s5l42Deg = fiveSqrtFortyTwoLogPolyEffectiveDeg(num, k);
  if (s5l42Deg !== undefined) {
    const denDegS5l42 = polynomialDegreeInK(den, k);
    if (denDegS5l42 !== undefined) {
      if (denDegS5l42 > s5l42Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 303: four sqrt + 42 logs
  const s4l42Deg = fourSqrtFortyTwoLogPolyEffectiveDeg(num, k);
  if (s4l42Deg !== undefined) {
    const denDegS4l42 = polynomialDegreeInK(den, k);
    if (denDegS4l42 !== undefined) {
      if (denDegS4l42 > s4l42Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 302: three sqrt + 42 logs
  const s3l42Deg = threeSqrtFortyTwoLogPolyEffectiveDeg(num, k);
  if (s3l42Deg !== undefined) {
    const denDegS3l42 = polynomialDegreeInK(den, k);
    if (denDegS3l42 !== undefined) {
      if (denDegS3l42 > s3l42Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 301: two sqrt + 42 logs
  const s2l42Deg = twoSqrtFortyTwoLogPolyEffectiveDeg(num, k);
  if (s2l42Deg !== undefined) {
    const denDegS2l42 = polynomialDegreeInK(den, k);
    if (denDegS2l42 !== undefined) {
      if (denDegS2l42 > s2l42Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 300: one sqrt + 42 logs
  const s1l42Deg = oneSqrtFortyTwoLogPolyEffectiveDeg(num, k);
  if (s1l42Deg !== undefined) {
    const denDegS1l42 = polynomialDegreeInK(den, k);
    if (denDegS1l42 !== undefined) {
      if (denDegS1l42 > s1l42Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 299: zero sqrt + 42 logs
  const s0l42Deg = fortyTwoLogPolyEffectiveDeg(num, k);
  if (s0l42Deg !== undefined) {
    const denDegS0l42 = polynomialDegreeInK(den, k);
    if (denDegS0l42 !== undefined) {
      if (denDegS0l42 > s0l42Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 298: five sqrt + 41 logs
  const s5l41Deg = fiveSqrtFortyOneLogPolyEffectiveDeg(num, k);
  if (s5l41Deg !== undefined) {
    const denDegS5l41 = polynomialDegreeInK(den, k);
    if (denDegS5l41 !== undefined) {
      if (denDegS5l41 > s5l41Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 297: four sqrt + 41 logs
  const s4l41Deg = fourSqrtFortyOneLogPolyEffectiveDeg(num, k);
  if (s4l41Deg !== undefined) {
    const denDegS4l41 = polynomialDegreeInK(den, k);
    if (denDegS4l41 !== undefined) {
      if (denDegS4l41 > s4l41Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 296: three sqrt + 41 logs
  const s3l41Deg = threeSqrtFortyOneLogPolyEffectiveDeg(num, k);
  if (s3l41Deg !== undefined) {
    const denDegS3l41 = polynomialDegreeInK(den, k);
    if (denDegS3l41 !== undefined) {
      if (denDegS3l41 > s3l41Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 295: two sqrt + 41 logs
  const s2l41Deg = twoSqrtFortyOneLogPolyEffectiveDeg(num, k);
  if (s2l41Deg !== undefined) {
    const denDegS2l41 = polynomialDegreeInK(den, k);
    if (denDegS2l41 !== undefined) {
      if (denDegS2l41 > s2l41Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 294: one sqrt + 41 logs
  const s1l41Deg = oneSqrtFortyOneLogPolyEffectiveDeg(num, k);
  if (s1l41Deg !== undefined) {
    const denDegS1l41 = polynomialDegreeInK(den, k);
    if (denDegS1l41 !== undefined) {
      if (denDegS1l41 > s1l41Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 293: zero sqrt + 41 logs
  const s0l41Deg = fortyOneLogPolyEffectiveDeg(num, k);
  if (s0l41Deg !== undefined) {
    const denDegS0l41 = polynomialDegreeInK(den, k);
    if (denDegS0l41 !== undefined) {
      if (denDegS0l41 > s0l41Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 292: five sqrt + 40 logs
  const s5l40Deg = fiveSqrtFortyLogPolyEffectiveDeg(num, k);
  if (s5l40Deg !== undefined) {
    const denDegS5l40 = polynomialDegreeInK(den, k);
    if (denDegS5l40 !== undefined) {
      if (denDegS5l40 > s5l40Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 291: four sqrt + 40 logs
  const s4l40Deg = fourSqrtFortyLogPolyEffectiveDeg(num, k);
  if (s4l40Deg !== undefined) {
    const denDegS4l40 = polynomialDegreeInK(den, k);
    if (denDegS4l40 !== undefined) {
      if (denDegS4l40 > s4l40Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 290: three sqrt + 40 logs
  const s3l40Deg = threeSqrtFortyLogPolyEffectiveDeg(num, k);
  if (s3l40Deg !== undefined) {
    const denDegS3l40 = polynomialDegreeInK(den, k);
    if (denDegS3l40 !== undefined) {
      if (denDegS3l40 > s3l40Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 289: two sqrt + 40 logs
  const s2l40Deg = twoSqrtFortyLogPolyEffectiveDeg(num, k);
  if (s2l40Deg !== undefined) {
    const denDegS2l40 = polynomialDegreeInK(den, k);
    if (denDegS2l40 !== undefined) {
      if (denDegS2l40 > s2l40Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 288: one sqrt + 40 logs
  const s1l40Deg = oneSqrtFortyLogPolyEffectiveDeg(num, k);
  if (s1l40Deg !== undefined) {
    const denDegS1l40 = polynomialDegreeInK(den, k);
    if (denDegS1l40 !== undefined) {
      if (denDegS1l40 > s1l40Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 287: zero sqrt + 40 logs
  const s0l40Deg = fortyLogPolyEffectiveDeg(num, k);
  if (s0l40Deg !== undefined) {
    const denDegS0l40 = polynomialDegreeInK(den, k);
    if (denDegS0l40 !== undefined) {
      if (denDegS0l40 > s0l40Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 286: five sqrt + 39 logs
  const s5l39Deg = fiveSqrtThirtyNineLogPolyEffectiveDeg(num, k);
  if (s5l39Deg !== undefined) {
    const denDegS5l39 = polynomialDegreeInK(den, k);
    if (denDegS5l39 !== undefined) {
      if (denDegS5l39 > s5l39Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 285: four sqrt + 39 logs
  const s4l39Deg = fourSqrtThirtyNineLogPolyEffectiveDeg(num, k);
  if (s4l39Deg !== undefined) {
    const denDegS4l39 = polynomialDegreeInK(den, k);
    if (denDegS4l39 !== undefined) {
      if (denDegS4l39 > s4l39Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 284: three sqrt + 39 logs
  const s3l39Deg = threeSqrtThirtyNineLogPolyEffectiveDeg(num, k);
  if (s3l39Deg !== undefined) {
    const denDegS3l39 = polynomialDegreeInK(den, k);
    if (denDegS3l39 !== undefined) {
      if (denDegS3l39 > s3l39Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 283: two sqrt + 39 logs
  const s2l39Deg = twoSqrtThirtyNineLogPolyEffectiveDeg(num, k);
  if (s2l39Deg !== undefined) {
    const denDegS2l39 = polynomialDegreeInK(den, k);
    if (denDegS2l39 !== undefined) {
      if (denDegS2l39 > s2l39Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 282: one sqrt + 39 logs
  const s1l39Deg = oneSqrtThirtyNineLogPolyEffectiveDeg(num, k);
  if (s1l39Deg !== undefined) {
    const denDegS1l39 = polynomialDegreeInK(den, k);
    if (denDegS1l39 !== undefined) {
      if (denDegS1l39 > s1l39Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 281: zero sqrt + 39 logs
  const s0l39Deg = thirtyNineLogPolyEffectiveDeg(num, k);
  if (s0l39Deg !== undefined) {
    const denDegS0l39 = polynomialDegreeInK(den, k);
    if (denDegS0l39 !== undefined) {
      if (denDegS0l39 > s0l39Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 280: five sqrt + 38 logs
  const s5l38Deg = fiveSqrtThirtyEightLogPolyEffectiveDeg(num, k);
  if (s5l38Deg !== undefined) {
    const denDegS5l38 = polynomialDegreeInK(den, k);
    if (denDegS5l38 !== undefined) {
      if (denDegS5l38 > s5l38Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 279: four sqrt + 38 logs
  const s4l38Deg = fourSqrtThirtyEightLogPolyEffectiveDeg(num, k);
  if (s4l38Deg !== undefined) {
    const denDegS4l38 = polynomialDegreeInK(den, k);
    if (denDegS4l38 !== undefined) {
      if (denDegS4l38 > s4l38Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 278: three sqrt + 38 logs
  const s3l38Deg = threeSqrtThirtyEightLogPolyEffectiveDeg(num, k);
  if (s3l38Deg !== undefined) {
    const denDegS3l38 = polynomialDegreeInK(den, k);
    if (denDegS3l38 !== undefined) {
      if (denDegS3l38 > s3l38Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 277: two sqrt + 38 logs
  const s2l38Deg = twoSqrtThirtyEightLogPolyEffectiveDeg(num, k);
  if (s2l38Deg !== undefined) {
    const denDegS2l38 = polynomialDegreeInK(den, k);
    if (denDegS2l38 !== undefined) {
      if (denDegS2l38 > s2l38Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 276: one sqrt + 38 logs
  const s1l38Deg = oneSqrtThirtyEightLogPolyEffectiveDeg(num, k);
  if (s1l38Deg !== undefined) {
    const denDegS1l38 = polynomialDegreeInK(den, k);
    if (denDegS1l38 !== undefined) {
      if (denDegS1l38 > s1l38Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 275: zero sqrt + 38 logs
  const s0l38Deg = thirtyEightLogPolyEffectiveDeg(num, k);
  if (s0l38Deg !== undefined) {
    const denDegS0l38 = polynomialDegreeInK(den, k);
    if (denDegS0l38 !== undefined) {
      if (denDegS0l38 > s0l38Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 274: five sqrt + 37 logs
  const s5l37Deg = fiveSqrtThirtySevenLogPolyEffectiveDeg(num, k);
  if (s5l37Deg !== undefined) {
    const denDegS5l37 = polynomialDegreeInK(den, k);
    if (denDegS5l37 !== undefined) {
      if (denDegS5l37 > s5l37Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 273: four sqrt + 37 logs
  const s4l37Deg = fourSqrtThirtySevenLogPolyEffectiveDeg(num, k);
  if (s4l37Deg !== undefined) {
    const denDegS4l37 = polynomialDegreeInK(den, k);
    if (denDegS4l37 !== undefined) {
      if (denDegS4l37 > s4l37Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 272: three sqrt + 37 logs
  const s3l37Deg = threeSqrtThirtySevenLogPolyEffectiveDeg(num, k);
  if (s3l37Deg !== undefined) {
    const denDegS3l37 = polynomialDegreeInK(den, k);
    if (denDegS3l37 !== undefined) {
      if (denDegS3l37 > s3l37Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 271: two sqrt + 37 logs
  const s2l37Deg = twoSqrtThirtySevenLogPolyEffectiveDeg(num, k);
  if (s2l37Deg !== undefined) {
    const denDegS2l37 = polynomialDegreeInK(den, k);
    if (denDegS2l37 !== undefined) {
      if (denDegS2l37 > s2l37Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 270: one sqrt + 37 logs
  const s1l37Deg = oneSqrtThirtySevenLogPolyEffectiveDeg(num, k);
  if (s1l37Deg !== undefined) {
    const denDegS1l37 = polynomialDegreeInK(den, k);
    if (denDegS1l37 !== undefined) {
      if (denDegS1l37 > s1l37Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 269: zero sqrt + 37 logs
  const s0l37Deg = thirtySevenLogPolyEffectiveDeg(num, k);
  if (s0l37Deg !== undefined) {
    const denDegS0l37 = polynomialDegreeInK(den, k);
    if (denDegS0l37 !== undefined) {
      if (denDegS0l37 > s0l37Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 268: five sqrt + 36 logs
  const s5l36Deg = fiveSqrtThirtySixLogPolyEffectiveDeg(num, k);
  if (s5l36Deg !== undefined) {
    const denDegS5l36 = polynomialDegreeInK(den, k);
    if (denDegS5l36 !== undefined) {
      if (denDegS5l36 > s5l36Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 267: four sqrt + 36 logs
  const s4l36Deg = fourSqrtThirtySixLogPolyEffectiveDeg(num, k);
  if (s4l36Deg !== undefined) {
    const denDegS4l36 = polynomialDegreeInK(den, k);
    if (denDegS4l36 !== undefined) {
      if (denDegS4l36 > s4l36Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 266: three sqrt + 36 logs
  const s3l36Deg = threeSqrtThirtySixLogPolyEffectiveDeg(num, k);
  if (s3l36Deg !== undefined) {
    const denDegS3l36 = polynomialDegreeInK(den, k);
    if (denDegS3l36 !== undefined) {
      if (denDegS3l36 > s3l36Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 265: two sqrt + 36 logs
  const s2l36Deg = twoSqrtThirtySixLogPolyEffectiveDeg(num, k);
  if (s2l36Deg !== undefined) {
    const denDegS2l36 = polynomialDegreeInK(den, k);
    if (denDegS2l36 !== undefined) {
      if (denDegS2l36 > s2l36Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 264: one sqrt + 36 logs
  const s1l36Deg = oneSqrtThirtySixLogPolyEffectiveDeg(num, k);
  if (s1l36Deg !== undefined) {
    const denDegS1l36 = polynomialDegreeInK(den, k);
    if (denDegS1l36 !== undefined) {
      if (denDegS1l36 > s1l36Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 263: zero sqrt + 36 logs
  const s0l36Deg = thirtySixLogPolyEffectiveDeg(num, k);
  if (s0l36Deg !== undefined) {
    const denDegS0l36 = polynomialDegreeInK(den, k);
    if (denDegS0l36 !== undefined) {
      if (denDegS0l36 > s0l36Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 262: five sqrt + 35 logs
  const s5l35Deg = fiveSqrtThirtyFiveLogPolyEffectiveDeg(num, k);
  if (s5l35Deg !== undefined) {
    const denDegS5l35 = polynomialDegreeInK(den, k);
    if (denDegS5l35 !== undefined) {
      if (denDegS5l35 > s5l35Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 261: four sqrt + 35 logs
  const s4l35Deg = fourSqrtThirtyFiveLogPolyEffectiveDeg(num, k);
  if (s4l35Deg !== undefined) {
    const denDegS4l35 = polynomialDegreeInK(den, k);
    if (denDegS4l35 !== undefined) {
      if (denDegS4l35 > s4l35Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 260: three sqrt + 35 logs
  const s3l35Deg = threeSqrtThirtyFiveLogPolyEffectiveDeg(num, k);
  if (s3l35Deg !== undefined) {
    const denDegS3l35 = polynomialDegreeInK(den, k);
    if (denDegS3l35 !== undefined) {
      if (denDegS3l35 > s3l35Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 259: two sqrt + 35 logs
  const s2l35Deg = twoSqrtThirtyFiveLogPolyEffectiveDeg(num, k);
  if (s2l35Deg !== undefined) {
    const denDegS2l35 = polynomialDegreeInK(den, k);
    if (denDegS2l35 !== undefined) {
      if (denDegS2l35 > s2l35Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 258: one sqrt + 35 logs
  const s1l35Deg = oneSqrtThirtyFiveLogPolyEffectiveDeg(num, k);
  if (s1l35Deg !== undefined) {
    const denDegS1l35 = polynomialDegreeInK(den, k);
    if (denDegS1l35 !== undefined) {
      if (denDegS1l35 > s1l35Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 257: zero sqrt + 35 logs
  const s0l35Deg = thirtyFiveLogPolyEffectiveDeg(num, k);
  if (s0l35Deg !== undefined) {
    const denDegS0l35 = polynomialDegreeInK(den, k);
    if (denDegS0l35 !== undefined) {
      if (denDegS0l35 > s0l35Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 256: five sqrt + 34 logs
  const s5l34Deg = fiveSqrtThirtyFourLogPolyEffectiveDeg(num, k);
  if (s5l34Deg !== undefined) {
    const denDegS5l34 = polynomialDegreeInK(den, k);
    if (denDegS5l34 !== undefined) {
      if (denDegS5l34 > s5l34Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 255: four sqrt + 34 logs
  const s4l34Deg = fourSqrtThirtyFourLogPolyEffectiveDeg(num, k);
  if (s4l34Deg !== undefined) {
    const denDegS4l34 = polynomialDegreeInK(den, k);
    if (denDegS4l34 !== undefined) {
      if (denDegS4l34 > s4l34Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 254: three sqrt + 34 logs
  const s3l34Deg = threeSqrtThirtyFourLogPolyEffectiveDeg(num, k);
  if (s3l34Deg !== undefined) {
    const denDegS3l34 = polynomialDegreeInK(den, k);
    if (denDegS3l34 !== undefined) {
      if (denDegS3l34 > s3l34Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 253: two sqrt + 34 logs
  const s2l34Deg = twoSqrtThirtyFourLogPolyEffectiveDeg(num, k);
  if (s2l34Deg !== undefined) {
    const denDegS2l34 = polynomialDegreeInK(den, k);
    if (denDegS2l34 !== undefined) {
      if (denDegS2l34 > s2l34Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 252: one sqrt + 34 logs
  const s1l34Deg = oneSqrtThirtyFourLogPolyEffectiveDeg(num, k);
  if (s1l34Deg !== undefined) {
    const denDegS1l34 = polynomialDegreeInK(den, k);
    if (denDegS1l34 !== undefined) {
      if (denDegS1l34 > s1l34Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 251: zero sqrt + 34 logs
  const s0l34Deg = thirtyFourLogPolyEffectiveDeg(num, k);
  if (s0l34Deg !== undefined) {
    const denDegS0l34 = polynomialDegreeInK(den, k);
    if (denDegS0l34 !== undefined) {
      if (denDegS0l34 > s0l34Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 250: five sqrt + 33 logs
  const s5l33Deg = fiveSqrtThirtyThreeLogPolyEffectiveDeg(num, k);
  if (s5l33Deg !== undefined) {
    const denDegS5l33 = polynomialDegreeInK(den, k);
    if (denDegS5l33 !== undefined) {
      if (denDegS5l33 > s5l33Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 249: four sqrt + 33 logs
  const s4l33Deg = fourSqrtThirtyThreeLogPolyEffectiveDeg(num, k);
  if (s4l33Deg !== undefined) {
    const denDegS4l33 = polynomialDegreeInK(den, k);
    if (denDegS4l33 !== undefined) {
      if (denDegS4l33 > s4l33Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 248: three sqrt + 33 logs
  const s3l33Deg = threeSqrtThirtyThreeLogPolyEffectiveDeg(num, k);
  if (s3l33Deg !== undefined) {
    const denDegS3l33 = polynomialDegreeInK(den, k);
    if (denDegS3l33 !== undefined) {
      if (denDegS3l33 > s3l33Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 247: two sqrt + 33 logs
  const s2l33Deg = twoSqrtThirtyThreeLogPolyEffectiveDeg(num, k);
  if (s2l33Deg !== undefined) {
    const denDegS2l33 = polynomialDegreeInK(den, k);
    if (denDegS2l33 !== undefined) {
      if (denDegS2l33 > s2l33Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 246: one sqrt + 33 logs
  const s1l33Deg = oneSqrtThirtyThreeLogPolyEffectiveDeg(num, k);
  if (s1l33Deg !== undefined) {
    const denDegS1l33 = polynomialDegreeInK(den, k);
    if (denDegS1l33 !== undefined) {
      if (denDegS1l33 > s1l33Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 245: zero sqrt + 33 logs
  const s0l33Deg = thirtyThreeLogPolyEffectiveDeg(num, k);
  if (s0l33Deg !== undefined) {
    const denDegS0l33 = polynomialDegreeInK(den, k);
    if (denDegS0l33 !== undefined) {
      if (denDegS0l33 > s0l33Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 244: five sqrt + 32 logs
  const s5l32Deg = fiveSqrtThirtyTwoLogPolyEffectiveDeg(num, k);
  if (s5l32Deg !== undefined) {
    const denDegS5l32 = polynomialDegreeInK(den, k);
    if (denDegS5l32 !== undefined) {
      if (denDegS5l32 > s5l32Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 243: four sqrt + 32 logs
  const s4l32Deg = fourSqrtThirtyTwoLogPolyEffectiveDeg(num, k);
  if (s4l32Deg !== undefined) {
    const denDegS4l32 = polynomialDegreeInK(den, k);
    if (denDegS4l32 !== undefined) {
      if (denDegS4l32 > s4l32Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 242: three sqrt + 32 logs
  const s3l32Deg = threeSqrtThirtyTwoLogPolyEffectiveDeg(num, k);
  if (s3l32Deg !== undefined) {
    const denDegS3l32 = polynomialDegreeInK(den, k);
    if (denDegS3l32 !== undefined) {
      if (denDegS3l32 > s3l32Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 241: two sqrt + 32 logs
  const s2l32Deg = twoSqrtThirtyTwoLogPolyEffectiveDeg(num, k);
  if (s2l32Deg !== undefined) {
    const denDegS2l32 = polynomialDegreeInK(den, k);
    if (denDegS2l32 !== undefined) {
      if (denDegS2l32 > s2l32Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 240: one sqrt + 32 logs
  const s1l32Deg = oneSqrtThirtyTwoLogPolyEffectiveDeg(num, k);
  if (s1l32Deg !== undefined) {
    const denDegS1l32 = polynomialDegreeInK(den, k);
    if (denDegS1l32 !== undefined) {
      if (denDegS1l32 > s1l32Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 239: zero sqrt + 32 logs
  const sl32Deg = thirtyTwoLogPolyEffectiveDeg(num, k);
  if (sl32Deg !== undefined) {
    const denDegSl32 = polynomialDegreeInK(den, k);
    if (denDegSl32 !== undefined) {
      if (denDegSl32 > sl32Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 238: five sqrt + 31 logs
  const s5l31Deg = fiveSqrtThirtyOneLogPolyEffectiveDeg(num, k);
  if (s5l31Deg !== undefined) {
    const denDegS5l31 = polynomialDegreeInK(den, k);
    if (denDegS5l31 !== undefined) {
      if (denDegS5l31 > s5l31Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 237: four sqrt + 31 logs
  const s4l31Deg = fourSqrtThirtyOneLogPolyEffectiveDeg(num, k);
  if (s4l31Deg !== undefined) {
    const denDegS4l31 = polynomialDegreeInK(den, k);
    if (denDegS4l31 !== undefined) {
      if (denDegS4l31 > s4l31Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 236: three sqrt + 31 logs
  const s3l31Deg = threeSqrtThirtyOneLogPolyEffectiveDeg(num, k);
  if (s3l31Deg !== undefined) {
    const denDegS3l31 = polynomialDegreeInK(den, k);
    if (denDegS3l31 !== undefined) {
      if (denDegS3l31 > s3l31Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 235: two sqrt + 31 logs
  const s2l31Deg = twoSqrtThirtyOneLogPolyEffectiveDeg(num, k);
  if (s2l31Deg !== undefined) {
    const denDegS2l31 = polynomialDegreeInK(den, k);
    if (denDegS2l31 !== undefined) {
      if (denDegS2l31 > s2l31Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 234: one sqrt + 31 logs
  const s1l31Deg = oneSqrtThirtyOneLogPolyEffectiveDeg(num, k);
  if (s1l31Deg !== undefined) {
    const denDegS1l31 = polynomialDegreeInK(den, k);
    if (denDegS1l31 !== undefined) {
      if (denDegS1l31 > s1l31Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 233: zero sqrt + 31 logs
  const sl31Deg = thirtyOneLogPolyEffectiveDeg(num, k);
  if (sl31Deg !== undefined) {
    const denDegSl31 = polynomialDegreeInK(den, k);
    if (denDegSl31 !== undefined) {
      if (denDegSl31 > sl31Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 232: five sqrt + 30 logs
  const s5l30Deg = fiveSqrtThirtyLogPolyEffectiveDeg(num, k);
  if (s5l30Deg !== undefined) {
    const denDegS5l30 = polynomialDegreeInK(den, k);
    if (denDegS5l30 !== undefined) {
      if (denDegS5l30 > s5l30Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 231: four sqrt + 30 logs
  const s4l30Deg = fourSqrtThirtyLogPolyEffectiveDeg(num, k);
  if (s4l30Deg !== undefined) {
    const denDegS4l30 = polynomialDegreeInK(den, k);
    if (denDegS4l30 !== undefined) {
      if (denDegS4l30 > s4l30Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 230: three sqrt + 30 logs
  const s3l30Deg = threeSqrtThirtyLogPolyEffectiveDeg(num, k);
  if (s3l30Deg !== undefined) {
    const denDegS3l30 = polynomialDegreeInK(den, k);
    if (denDegS3l30 !== undefined) {
      if (denDegS3l30 > s3l30Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 229: two sqrt + 30 logs
  const s2l30Deg = twoSqrtThirtyLogPolyEffectiveDeg(num, k);
  if (s2l30Deg !== undefined) {
    const denDegS2l30 = polynomialDegreeInK(den, k);
    if (denDegS2l30 !== undefined) {
      if (denDegS2l30 > s2l30Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 228: one sqrt + 30 logs
  const s1l30Deg = oneSqrtThirtyLogPolyEffectiveDeg(num, k);
  if (s1l30Deg !== undefined) {
    const denDegS1l30 = polynomialDegreeInK(den, k);
    if (denDegS1l30 !== undefined) {
      if (denDegS1l30 > s1l30Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 227: zero sqrt + 30 logs
  const sl30Deg = thirtyLogPolyEffectiveDeg(num, k);
  if (sl30Deg !== undefined) {
    const denDegSl30 = polynomialDegreeInK(den, k);
    if (denDegSl30 !== undefined) {
      if (denDegSl30 > sl30Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 226: five sqrt + 29 logs
  const s5l29Deg = fiveSqrtTwentyNineLogPolyEffectiveDeg(num, k);
  if (s5l29Deg !== undefined) {
    const denDegS5l29 = polynomialDegreeInK(den, k);
    if (denDegS5l29 !== undefined) {
      if (denDegS5l29 > s5l29Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 225: four sqrt + 29 logs
  const s4l29Deg = fourSqrtTwentyNineLogPolyEffectiveDeg(num, k);
  if (s4l29Deg !== undefined) {
    const denDegS4l29 = polynomialDegreeInK(den, k);
    if (denDegS4l29 !== undefined) {
      if (denDegS4l29 > s4l29Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 224: three sqrt + 29 logs
  const s3l29Deg = threeSqrtTwentyNineLogPolyEffectiveDeg(num, k);
  if (s3l29Deg !== undefined) {
    const denDegS3l29 = polynomialDegreeInK(den, k);
    if (denDegS3l29 !== undefined) {
      if (denDegS3l29 > s3l29Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 223: two sqrt + 29 logs
  const s2l29Deg = twoSqrtTwentyNineLogPolyEffectiveDeg(num, k);
  if (s2l29Deg !== undefined) {
    const denDegS2l29 = polynomialDegreeInK(den, k);
    if (denDegS2l29 !== undefined) {
      if (denDegS2l29 > s2l29Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 222: one sqrt + 29 logs
  const s1l29Deg = oneSqrtTwentyNineLogPolyEffectiveDeg(num, k);
  if (s1l29Deg !== undefined) {
    const denDegS1l29 = polynomialDegreeInK(den, k);
    if (denDegS1l29 !== undefined) {
      if (denDegS1l29 > s1l29Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 221: zero sqrt + 29 logs
  const sl29Deg = twentyNineLogPolyEffectiveDeg(num, k);
  if (sl29Deg !== undefined) {
    const denDegSl29 = polynomialDegreeInK(den, k);
    if (denDegSl29 !== undefined) {
      if (denDegSl29 > sl29Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 220: five sqrt + 28 logs
  const s5l28Deg = fiveSqrtTwentyEightLogPolyEffectiveDeg(num, k);
  if (s5l28Deg !== undefined) {
    const denDegS5l28 = polynomialDegreeInK(den, k);
    if (denDegS5l28 !== undefined) {
      if (denDegS5l28 > s5l28Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 219: four sqrt + 28 logs
  const s4l28Deg = fourSqrtTwentyEightLogPolyEffectiveDeg(num, k);
  if (s4l28Deg !== undefined) {
    const denDegS4l28 = polynomialDegreeInK(den, k);
    if (denDegS4l28 !== undefined) {
      if (denDegS4l28 > s4l28Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 218: three sqrt + 28 logs
  const s3l28Deg = threeSqrtTwentyEightLogPolyEffectiveDeg(num, k);
  if (s3l28Deg !== undefined) {
    const denDegS3l28 = polynomialDegreeInK(den, k);
    if (denDegS3l28 !== undefined) {
      if (denDegS3l28 > s3l28Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 217: two sqrt + 28 logs
  const s2l28Deg = twoSqrtTwentyEightLogPolyEffectiveDeg(num, k);
  if (s2l28Deg !== undefined) {
    const denDegS2l28 = polynomialDegreeInK(den, k);
    if (denDegS2l28 !== undefined) {
      if (denDegS2l28 > s2l28Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 216: one sqrt + 28 logs
  const s1l28Deg = oneSqrtTwentyEightLogPolyEffectiveDeg(num, k);
  if (s1l28Deg !== undefined) {
    const denDegS1l28 = polynomialDegreeInK(den, k);
    if (denDegS1l28 !== undefined) {
      if (denDegS1l28 > s1l28Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 215: zero sqrt + 28 logs
  const sl28Deg = twentyEightLogPolyEffectiveDeg(num, k);
  if (sl28Deg !== undefined) {
    const denDegSl28 = polynomialDegreeInK(den, k);
    if (denDegSl28 !== undefined) {
      if (denDegSl28 > sl28Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 214: five sqrt + 27 logs
  const s5l27Deg = fiveSqrtTwentySevenLogPolyEffectiveDeg(num, k);
  if (s5l27Deg !== undefined) {
    const denDegS5l27 = polynomialDegreeInK(den, k);
    if (denDegS5l27 !== undefined) {
      if (denDegS5l27 > s5l27Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 213: four sqrt + 27 logs
  const s4l27Deg = fourSqrtTwentySevenLogPolyEffectiveDeg(num, k);
  if (s4l27Deg !== undefined) {
    const denDegS4l27 = polynomialDegreeInK(den, k);
    if (denDegS4l27 !== undefined) {
      if (denDegS4l27 > s4l27Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 212: three sqrt + 27 logs
  const s3l27Deg = threeSqrtTwentySevenLogPolyEffectiveDeg(num, k);
  if (s3l27Deg !== undefined) {
    const denDegS3l27 = polynomialDegreeInK(den, k);
    if (denDegS3l27 !== undefined) {
      if (denDegS3l27 > s3l27Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 211: two sqrt + 27 logs
  const s2l27Deg = twoSqrtTwentySevenLogPolyEffectiveDeg(num, k);
  if (s2l27Deg !== undefined) {
    const denDegS2l27 = polynomialDegreeInK(den, k);
    if (denDegS2l27 !== undefined) {
      if (denDegS2l27 > s2l27Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 210: one sqrt + 27 logs
  const s1l27Deg = oneSqrtTwentySevenLogPolyEffectiveDeg(num, k);
  if (s1l27Deg !== undefined) {
    const denDegS1l27 = polynomialDegreeInK(den, k);
    if (denDegS1l27 !== undefined) {
      if (denDegS1l27 > s1l27Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 209: zero sqrt + 27 logs
  const sl27Deg = twentySevenLogPolyEffectiveDeg(num, k);
  if (sl27Deg !== undefined) {
    const denDegSl27 = polynomialDegreeInK(den, k);
    if (denDegSl27 !== undefined) {
      if (denDegSl27 > sl27Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 208: five sqrt + 26 logs
  const s5l26Deg = fiveSqrtTwentySixLogPolyEffectiveDeg(num, k);
  if (s5l26Deg !== undefined) {
    const denDegS5l26 = polynomialDegreeInK(den, k);
    if (denDegS5l26 !== undefined) {
      if (denDegS5l26 > s5l26Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 207: four sqrt + 26 logs
  const s4l26Deg = fourSqrtTwentySixLogPolyEffectiveDeg(num, k);
  if (s4l26Deg !== undefined) {
    const denDegS4l26 = polynomialDegreeInK(den, k);
    if (denDegS4l26 !== undefined) {
      if (denDegS4l26 > s4l26Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 206: three sqrt + 26 logs
  const s3l26Deg = threeSqrtTwentySixLogPolyEffectiveDeg(num, k);
  if (s3l26Deg !== undefined) {
    const denDegS3l26 = polynomialDegreeInK(den, k);
    if (denDegS3l26 !== undefined) {
      if (denDegS3l26 > s3l26Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 205: two sqrt + 26 logs
  const s2l26Deg = twoSqrtTwentySixLogPolyEffectiveDeg(num, k);
  if (s2l26Deg !== undefined) {
    const denDegS2l26 = polynomialDegreeInK(den, k);
    if (denDegS2l26 !== undefined) {
      if (denDegS2l26 > s2l26Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 204: one sqrt + 26 logs
  const s1l26Deg = oneSqrtTwentySixLogPolyEffectiveDeg(num, k);
  if (s1l26Deg !== undefined) {
    const denDegS1l26 = polynomialDegreeInK(den, k);
    if (denDegS1l26 !== undefined) {
      if (denDegS1l26 > s1l26Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 203: zero sqrt + 26 logs
  const sl26Deg = twentySixLogPolyEffectiveDeg(num, k);
  if (sl26Deg !== undefined) {
    const denDegSl26 = polynomialDegreeInK(den, k);
    if (denDegSl26 !== undefined) {
      if (denDegSl26 > sl26Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 202: five sqrt + 25 logs
  const s5l25Deg = fiveSqrtTwentyFiveLogPolyEffectiveDeg(num, k);
  if (s5l25Deg !== undefined) {
    const denDegS5l25 = polynomialDegreeInK(den, k);
    if (denDegS5l25 !== undefined) {
      if (denDegS5l25 > s5l25Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 201: four sqrt + 25 logs
  const s4l25Deg = fourSqrtTwentyFiveLogPolyEffectiveDeg(num, k);
  if (s4l25Deg !== undefined) {
    const denDegS4l25 = polynomialDegreeInK(den, k);
    if (denDegS4l25 !== undefined) {
      if (denDegS4l25 > s4l25Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 200: three sqrt + 25 logs
  const s3l25Deg = threeSqrtTwentyFiveLogPolyEffectiveDeg(num, k);
  if (s3l25Deg !== undefined) {
    const denDegS3l25 = polynomialDegreeInK(den, k);
    if (denDegS3l25 !== undefined) {
      if (denDegS3l25 > s3l25Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 199: two sqrt + 25 logs
  const s2l25Deg = twoSqrtTwentyFiveLogPolyEffectiveDeg(num, k);
  if (s2l25Deg !== undefined) {
    const denDegS2l25 = polynomialDegreeInK(den, k);
    if (denDegS2l25 !== undefined) {
      if (denDegS2l25 > s2l25Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 198: one sqrt + 25 logs
  const s1l25Deg = oneSqrtTwentyFiveLogPolyEffectiveDeg(num, k);
  if (s1l25Deg !== undefined) {
    const denDegS1l25 = polynomialDegreeInK(den, k);
    if (denDegS1l25 !== undefined) {
      if (denDegS1l25 > s1l25Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 197: zero sqrt + 25 logs
  const sl25Deg = twentyFiveLogPolyEffectiveDeg(num, k);
  if (sl25Deg !== undefined) {
    const denDegSl25 = polynomialDegreeInK(den, k);
    if (denDegSl25 !== undefined) {
      if (denDegSl25 > sl25Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 196: five sqrt + 24 logs
  const s5l24Deg = fiveSqrtTwentyFourLogPolyEffectiveDeg(num, k);
  if (s5l24Deg !== undefined) {
    const denDegS5l24 = polynomialDegreeInK(den, k);
    if (denDegS5l24 !== undefined) {
      if (denDegS5l24 > s5l24Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 195: four sqrt + 24 logs
  const s4l24Deg = fourSqrtTwentyFourLogPolyEffectiveDeg(num, k);
  if (s4l24Deg !== undefined) {
    const denDegS4l24 = polynomialDegreeInK(den, k);
    if (denDegS4l24 !== undefined) {
      if (denDegS4l24 > s4l24Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 194: three sqrt + 24 logs
  const s3l24Deg = threeSqrtTwentyFourLogPolyEffectiveDeg(num, k);
  if (s3l24Deg !== undefined) {
    const denDegS3l24 = polynomialDegreeInK(den, k);
    if (denDegS3l24 !== undefined) {
      if (denDegS3l24 > s3l24Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 193: two sqrt + 24 logs
  const s2l24Deg = twoSqrtTwentyFourLogPolyEffectiveDeg(num, k);
  if (s2l24Deg !== undefined) {
    const denDegS2l24 = polynomialDegreeInK(den, k);
    if (denDegS2l24 !== undefined) {
      if (denDegS2l24 > s2l24Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 192: one sqrt + 24 logs
  const s1l24Deg = oneSqrtTwentyFourLogPolyEffectiveDeg(num, k);
  if (s1l24Deg !== undefined) {
    const denDegS1l24 = polynomialDegreeInK(den, k);
    if (denDegS1l24 !== undefined) {
      if (denDegS1l24 > s1l24Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 191: zero sqrt + 24 logs
  const sl24Deg = twentyFourLogPolyEffectiveDeg(num, k);
  if (sl24Deg !== undefined) {
    const denDegSl24 = polynomialDegreeInK(den, k);
    if (denDegSl24 !== undefined) {
      if (denDegSl24 > sl24Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 190: five sqrt + 23 logs
  const s5l23Deg = fiveSqrtTwentyThreeLogPolyEffectiveDeg(num, k);
  if (s5l23Deg !== undefined) {
    const denDegS5l23 = polynomialDegreeInK(den, k);
    if (denDegS5l23 !== undefined) {
      if (denDegS5l23 > s5l23Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 189: four sqrt + 23 logs
  const s4l23Deg = fourSqrtTwentyThreeLogPolyEffectiveDeg(num, k);
  if (s4l23Deg !== undefined) {
    const denDegS4l23 = polynomialDegreeInK(den, k);
    if (denDegS4l23 !== undefined) {
      if (denDegS4l23 > s4l23Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 188: three sqrt + 23 logs
  const s3l23Deg = threeSqrtTwentyThreeLogPolyEffectiveDeg(num, k);
  if (s3l23Deg !== undefined) {
    const denDegS3l23 = polynomialDegreeInK(den, k);
    if (denDegS3l23 !== undefined) {
      if (denDegS3l23 > s3l23Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 187: two sqrt + 23 logs
  const s2l23Deg = twoSqrtTwentyThreeLogPolyEffectiveDeg(num, k);
  if (s2l23Deg !== undefined) {
    const denDegS2l23 = polynomialDegreeInK(den, k);
    if (denDegS2l23 !== undefined) {
      if (denDegS2l23 > s2l23Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 186: one sqrt + 23 logs
  const s1l23Deg = oneSqrtTwentyThreeLogPolyEffectiveDeg(num, k);
  if (s1l23Deg !== undefined) {
    const denDegS1l23 = polynomialDegreeInK(den, k);
    if (denDegS1l23 !== undefined) {
      if (denDegS1l23 > s1l23Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 185: zero sqrt + 23 logs
  const sl23Deg = twentyThreeLogPolyEffectiveDeg(num, k);
  if (sl23Deg !== undefined) {
    const denDegSl23 = polynomialDegreeInK(den, k);
    if (denDegSl23 !== undefined) {
      if (denDegSl23 > sl23Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 184: five sqrt + 22 logs
  const s5l22Deg = fiveSqrtTwentyTwoLogPolyEffectiveDeg(num, k);
  if (s5l22Deg !== undefined) {
    const denDegS5l22 = polynomialDegreeInK(den, k);
    if (denDegS5l22 !== undefined) {
      if (denDegS5l22 > s5l22Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 183: four sqrt + 22 logs
  const s4l22Deg = fourSqrtTwentyTwoLogPolyEffectiveDeg(num, k);
  if (s4l22Deg !== undefined) {
    const denDegS4l22 = polynomialDegreeInK(den, k);
    if (denDegS4l22 !== undefined) {
      if (denDegS4l22 > s4l22Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 182: three sqrt + 22 logs
  const s3l22Deg = threeSqrtTwentyTwoLogPolyEffectiveDeg(num, k);
  if (s3l22Deg !== undefined) {
    const denDegS3l22 = polynomialDegreeInK(den, k);
    if (denDegS3l22 !== undefined) {
      if (denDegS3l22 > s3l22Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 181: two sqrt + 22 logs
  const s2l22Deg = twoSqrtTwentyTwoLogPolyEffectiveDeg(num, k);
  if (s2l22Deg !== undefined) {
    const denDegS2l22 = polynomialDegreeInK(den, k);
    if (denDegS2l22 !== undefined) {
      if (denDegS2l22 > s2l22Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 180: one sqrt + 22 logs
  const s1l22Deg = oneSqrtTwentyTwoLogPolyEffectiveDeg(num, k);
  if (s1l22Deg !== undefined) {
    const denDegS1l22 = polynomialDegreeInK(den, k);
    if (denDegS1l22 !== undefined) {
      if (denDegS1l22 > s1l22Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 179: zero sqrt + 22 logs
  const sl22Deg = twentyTwoLogPolyEffectiveDeg(num, k);
  if (sl22Deg !== undefined) {
    const denDegSl22 = polynomialDegreeInK(den, k);
    if (denDegSl22 !== undefined) {
      if (denDegSl22 > sl22Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 178: five sqrt + 21 logs
  const s5l21Deg = fiveSqrtTwentyOneLogPolyEffectiveDeg(num, k);
  if (s5l21Deg !== undefined) {
    const denDegS5l21 = polynomialDegreeInK(den, k);
    if (denDegS5l21 !== undefined) {
      if (denDegS5l21 > s5l21Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 177: four sqrt + 21 logs
  const s4l21Deg = fourSqrtTwentyOneLogPolyEffectiveDeg(num, k);
  if (s4l21Deg !== undefined) {
    const denDegS4l21 = polynomialDegreeInK(den, k);
    if (denDegS4l21 !== undefined) {
      if (denDegS4l21 > s4l21Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 176: three sqrt + 21 logs
  const s3l21Deg = threeSqrtTwentyOneLogPolyEffectiveDeg(num, k);
  if (s3l21Deg !== undefined) {
    const denDegS3l21 = polynomialDegreeInK(den, k);
    if (denDegS3l21 !== undefined) {
      if (denDegS3l21 > s3l21Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 175: two sqrt + 21 logs
  const s2l21Deg = twoSqrtTwentyOneLogPolyEffectiveDeg(num, k);
  if (s2l21Deg !== undefined) {
    const denDegS2l21 = polynomialDegreeInK(den, k);
    if (denDegS2l21 !== undefined) {
      if (denDegS2l21 > s2l21Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 174: one sqrt + 21 logs
  const s1l21Deg = oneSqrtTwentyOneLogPolyEffectiveDeg(num, k);
  if (s1l21Deg !== undefined) {
    const denDegS1l21 = polynomialDegreeInK(den, k);
    if (denDegS1l21 !== undefined) {
      if (denDegS1l21 > s1l21Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 173: zero sqrt + 21 logs
  const sl21Deg = twentyOneLogPolyEffectiveDeg(num, k);
  if (sl21Deg !== undefined) {
    const denDegSl21 = polynomialDegreeInK(den, k);
    if (denDegSl21 !== undefined) {
      if (denDegSl21 > sl21Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 172: Mul(Sqrt(P1)×5, Log(h1)×20, polynomial..., bounded...) numerator.
  // Five Sqrt + twenty Log factors; log²⁰ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtTwentyLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l20Deg = fiveSqrtTwentyLogPolyEffectiveDeg(num, k);
  if (s5l20Deg !== undefined) {
    const denDegS5l20 = polynomialDegreeInK(den, k);
    if (denDegS5l20 !== undefined) {
      if (denDegS5l20 > s5l20Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 171: Mul(Sqrt(P1)×4, Log(h1)×20, polynomial..., bounded...) numerator.
  const s4l20Deg = fourSqrtTwentyLogPolyEffectiveDeg(num, k);
  if (s4l20Deg !== undefined) {
    const denDegS4l20 = polynomialDegreeInK(den, k);
    if (denDegS4l20 !== undefined) {
      if (denDegS4l20 > s4l20Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 170: Mul(Sqrt(P1)×3, Log(h1)×20, polynomial..., bounded...) numerator.
  const s3l20Deg = threeSqrtTwentyLogPolyEffectiveDeg(num, k);
  if (s3l20Deg !== undefined) {
    const denDegS3l20 = polynomialDegreeInK(den, k);
    if (denDegS3l20 !== undefined) {
      if (denDegS3l20 > s3l20Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 169: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×20, polynomial..., bounded...) numerator.
  const s2l20Deg = twoSqrtTwentyLogPolyEffectiveDeg(num, k);
  if (s2l20Deg !== undefined) {
    const denDegS2l20 = polynomialDegreeInK(den, k);
    if (denDegS2l20 !== undefined) {
      if (denDegS2l20 > s2l20Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 168: Mul(Sqrt(P), Log(h1)×20, polynomial..., bounded...) numerator.
  const s1l20Deg = oneSqrtTwentyLogPolyEffectiveDeg(num, k);
  if (s1l20Deg !== undefined) {
    const denDegS1l20 = polynomialDegreeInK(den, k);
    if (denDegS1l20 !== undefined) {
      if (denDegS1l20 > s1l20Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 167: Closes when denDeg > twentyLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl20Deg = twentyLogPolyEffectiveDeg(num, k);
  if (sl20Deg !== undefined) {
    const denDegSl20 = polynomialDegreeInK(den, k);
    if (denDegSl20 !== undefined) {
      if (denDegSl20 > sl20Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 166: Mul(Sqrt(P1)×5, Log(h1)×19, polynomial..., bounded...) numerator.
  // Five Sqrt + nineteen Log factors; log¹⁹ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtNineteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l19Deg = fiveSqrtNineteenLogPolyEffectiveDeg(num, k);
  if (s5l19Deg !== undefined) {
    const denDegS5l19 = polynomialDegreeInK(den, k);
    if (denDegS5l19 !== undefined) {
      if (denDegS5l19 > s5l19Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 165: Mul(Sqrt(P1)×4, Log(h1)×19, polynomial..., bounded...) numerator.
  // Four Sqrt + nineteen Log factors; log¹⁹ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtNineteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l19Deg = fourSqrtNineteenLogPolyEffectiveDeg(num, k);
  if (s4l19Deg !== undefined) {
    const denDegS4l19 = polynomialDegreeInK(den, k);
    if (denDegS4l19 !== undefined) {
      if (denDegS4l19 > s4l19Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 164: Mul(Sqrt(P1)×3, Log(h1)×19, polynomial..., bounded...) numerator.
  // Three Sqrt + nineteen Log factors; log¹⁹ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtNineteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l19Deg = threeSqrtNineteenLogPolyEffectiveDeg(num, k);
  if (s3l19Deg !== undefined) {
    const denDegS3l19 = polynomialDegreeInK(den, k);
    if (denDegS3l19 !== undefined) {
      if (denDegS3l19 > s3l19Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 163: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×19, polynomial..., bounded...) numerator.
  // Two Sqrt + nineteen Log factors; log¹⁹ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtNineteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l19Deg = twoSqrtNineteenLogPolyEffectiveDeg(num, k);
  if (s2l19Deg !== undefined) {
    const denDegS2l19 = polynomialDegreeInK(den, k);
    if (denDegS2l19 !== undefined) {
      if (denDegS2l19 > s2l19Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 162: Mul(Sqrt(P), Log(h1)×19, polynomial..., bounded...) numerator.
  // One Sqrt + nineteen Log factors; log¹⁹ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtNineteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l19Deg = oneSqrtNineteenLogPolyEffectiveDeg(num, k);
  if (s1l19Deg !== undefined) {
    const denDegS1l19 = polynomialDegreeInK(den, k);
    if (denDegS1l19 !== undefined) {
      if (denDegS1l19 > s1l19Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 161: Mul(Log(h1)×19, polynomial..., bounded...) numerator.
  // Zero Sqrt + nineteen Log factors; log¹⁹ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > nineteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl19Deg = nineteenLogPolyEffectiveDeg(num, k);
  if (sl19Deg !== undefined) {
    const denDegSl19 = polynomialDegreeInK(den, k);
    if (denDegSl19 !== undefined) {
      if (denDegSl19 > sl19Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 160: Mul(Sqrt(P1)×5, Log(h1)×18, polynomial..., bounded...) numerator.
  // Five Sqrt + eighteen Log factors; log¹⁸ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtEighteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l18Deg = fiveSqrtEighteenLogPolyEffectiveDeg(num, k);
  if (s5l18Deg !== undefined) {
    const denDegS5l18 = polynomialDegreeInK(den, k);
    if (denDegS5l18 !== undefined) {
      if (denDegS5l18 > s5l18Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 159: Mul(Sqrt(P1)×4, Log(h1)×18, polynomial..., bounded...) numerator.
  // Four Sqrt + eighteen Log factors; log¹⁸ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtEighteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l18Deg = fourSqrtEighteenLogPolyEffectiveDeg(num, k);
  if (s4l18Deg !== undefined) {
    const denDegS4l18 = polynomialDegreeInK(den, k);
    if (denDegS4l18 !== undefined) {
      if (denDegS4l18 > s4l18Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 158: Mul(Sqrt(P1)×3, Log(h1)×18, polynomial..., bounded...) numerator.
  // Three Sqrt + eighteen Log factors; log¹⁸ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtEighteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l18Deg = threeSqrtEighteenLogPolyEffectiveDeg(num, k);
  if (s3l18Deg !== undefined) {
    const denDegS3l18 = polynomialDegreeInK(den, k);
    if (denDegS3l18 !== undefined) {
      if (denDegS3l18 > s3l18Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 157: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×18, polynomial..., bounded...) numerator.
  // Two Sqrt + eighteen Log factors; log¹⁸ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtEighteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l18Deg = twoSqrtEighteenLogPolyEffectiveDeg(num, k);
  if (s2l18Deg !== undefined) {
    const denDegS2l18 = polynomialDegreeInK(den, k);
    if (denDegS2l18 !== undefined) {
      if (denDegS2l18 > s2l18Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 156: Mul(Sqrt(P), Log(h1)×18, polynomial..., bounded...) numerator.
  // One Sqrt + eighteen Log factors; log¹⁸ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtEighteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l18Deg = oneSqrtEighteenLogPolyEffectiveDeg(num, k);
  if (s1l18Deg !== undefined) {
    const denDegS1l18 = polynomialDegreeInK(den, k);
    if (denDegS1l18 !== undefined) {
      if (denDegS1l18 > s1l18Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 155: Mul(Log(h1)×18, polynomial..., bounded...) numerator.
  // Zero Sqrt + eighteen Log factors; log¹⁸ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > eighteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl18Deg = eighteenLogPolyEffectiveDeg(num, k);
  if (sl18Deg !== undefined) {
    const denDegSl18 = polynomialDegreeInK(den, k);
    if (denDegSl18 !== undefined) {
      if (denDegSl18 > sl18Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 154: Mul(Sqrt(P1)×5, Log(h1)×17, polynomial..., bounded...) numerator.
  // Five Sqrt + seventeen Log factors; log¹⁷ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtSeventeenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l17Deg = fiveSqrtSeventeenLogPolyEffectiveDeg(num, k);
  if (s5l17Deg !== undefined) {
    const denDegS5l17 = polynomialDegreeInK(den, k);
    if (denDegS5l17 !== undefined) {
      if (denDegS5l17 > s5l17Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 153: Mul(Sqrt(P1)×4, Log(h1)×17, polynomial..., bounded...) numerator.
  // Four Sqrt + seventeen Log factors; log¹⁷ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtSeventeenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l17Deg = fourSqrtSeventeenLogPolyEffectiveDeg(num, k);
  if (s4l17Deg !== undefined) {
    const denDegS4l17 = polynomialDegreeInK(den, k);
    if (denDegS4l17 !== undefined) {
      if (denDegS4l17 > s4l17Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 152: Mul(Sqrt(P1)×3, Log(h1)×17, polynomial..., bounded...) numerator.
  // Three Sqrt + seventeen Log factors; log¹⁷ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtSeventeenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l17Deg = threeSqrtSeventeenLogPolyEffectiveDeg(num, k);
  if (s3l17Deg !== undefined) {
    const denDegS3l17 = polynomialDegreeInK(den, k);
    if (denDegS3l17 !== undefined) {
      if (denDegS3l17 > s3l17Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 151: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×17, polynomial..., bounded...) numerator.
  // Two Sqrt + seventeen Log factors; log¹⁷ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtSeventeenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l17Deg = twoSqrtSeventeenLogPolyEffectiveDeg(num, k);
  if (s2l17Deg !== undefined) {
    const denDegS2l17 = polynomialDegreeInK(den, k);
    if (denDegS2l17 !== undefined) {
      if (denDegS2l17 > s2l17Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 150: Mul(Sqrt(P), Log(h1)×17, polynomial..., bounded...) numerator.
  // One Sqrt + seventeen Log factors; log¹⁷ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtSeventeenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l17Deg = oneSqrtSeventeenLogPolyEffectiveDeg(num, k);
  if (s1l17Deg !== undefined) {
    const denDegS1l17 = polynomialDegreeInK(den, k);
    if (denDegS1l17 !== undefined) {
      if (denDegS1l17 > s1l17Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 149: Mul(Log(h1)×17, polynomial..., bounded...) numerator.
  // Zero Sqrt + seventeen Log factors; log¹⁷ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > seventeenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl17Deg = seventeenLogPolyEffectiveDeg(num, k);
  if (sl17Deg !== undefined) {
    const denDegSl17 = polynomialDegreeInK(den, k);
    if (denDegSl17 !== undefined) {
      if (denDegSl17 > sl17Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 148: Mul(Sqrt(P1)×5, Log(h1)×16, polynomial..., bounded...) numerator.
  // Five Sqrt + sixteen Log factors; log¹⁶ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtSixteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l16Deg = fiveSqrtSixteenLogPolyEffectiveDeg(num, k);
  if (s5l16Deg !== undefined) {
    const denDegS5l16 = polynomialDegreeInK(den, k);
    if (denDegS5l16 !== undefined) {
      if (denDegS5l16 > s5l16Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 147: Mul(Sqrt(P1)×4, Log(h1)×16, polynomial..., bounded...) numerator.
  // Four Sqrt + sixteen Log factors; log¹⁶ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtSixteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l16Deg = fourSqrtSixteenLogPolyEffectiveDeg(num, k);
  if (s4l16Deg !== undefined) {
    const denDegS4l16 = polynomialDegreeInK(den, k);
    if (denDegS4l16 !== undefined) {
      if (denDegS4l16 > s4l16Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 146: Mul(Sqrt(P1)×3, Log(h1)×16, polynomial..., bounded...) numerator.
  // Three Sqrt + sixteen Log factors; log¹⁶ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtSixteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l16Deg = threeSqrtSixteenLogPolyEffectiveDeg(num, k);
  if (s3l16Deg !== undefined) {
    const denDegS3l16 = polynomialDegreeInK(den, k);
    if (denDegS3l16 !== undefined) {
      if (denDegS3l16 > s3l16Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 145: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×16, polynomial..., bounded...) numerator.
  // Two Sqrt + sixteen Log factors; log¹⁶ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtSixteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l16Deg = twoSqrtSixteenLogPolyEffectiveDeg(num, k);
  if (s2l16Deg !== undefined) {
    const denDegS2l16 = polynomialDegreeInK(den, k);
    if (denDegS2l16 !== undefined) {
      if (denDegS2l16 > s2l16Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 144: Mul(Sqrt(P), Log(h1)×16, polynomial..., bounded...) numerator.
  // One Sqrt + sixteen Log factors; log¹⁶ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtSixteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l16Deg = oneSqrtSixteenLogPolyEffectiveDeg(num, k);
  if (s1l16Deg !== undefined) {
    const denDegS1l16 = polynomialDegreeInK(den, k);
    if (denDegS1l16 !== undefined) {
      if (denDegS1l16 > s1l16Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 143: Mul(Log(h1)×16, polynomial..., bounded...) numerator.
  // Zero Sqrt + sixteen Log factors; log¹⁶ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > sixteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl16Deg = sixteenLogPolyEffectiveDeg(num, k);
  if (sl16Deg !== undefined) {
    const denDegSl16 = polynomialDegreeInK(den, k);
    if (denDegSl16 !== undefined) {
      if (denDegSl16 > sl16Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 142: Mul(Sqrt(P1)×5, Log(h1)×15, polynomial..., bounded...) numerator.
  // Five Sqrt + fifteen Log factors; log¹⁵ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtFifteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l15Deg = fiveSqrtFifteenLogPolyEffectiveDeg(num, k);
  if (s5l15Deg !== undefined) {
    const denDegS5l15 = polynomialDegreeInK(den, k);
    if (denDegS5l15 !== undefined) {
      if (denDegS5l15 > s5l15Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 141: Mul(Sqrt(P1)×4, Log(h1)×15, polynomial..., bounded...) numerator.
  // Four Sqrt + fifteen Log factors; log¹⁵ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtFifteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l15Deg = fourSqrtFifteenLogPolyEffectiveDeg(num, k);
  if (s4l15Deg !== undefined) {
    const denDegS4l15 = polynomialDegreeInK(den, k);
    if (denDegS4l15 !== undefined) {
      if (denDegS4l15 > s4l15Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 140: Mul(Sqrt(P1)×3, Log(h1)×15, polynomial..., bounded...) numerator.
  // Three Sqrt + fifteen Log factors; log¹⁵ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtFifteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l15Deg = threeSqrtFifteenLogPolyEffectiveDeg(num, k);
  if (s3l15Deg !== undefined) {
    const denDegS3l15 = polynomialDegreeInK(den, k);
    if (denDegS3l15 !== undefined) {
      if (denDegS3l15 > s3l15Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 139: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×15, polynomial..., bounded...) numerator.
  // Two Sqrt + fifteen Log factors; log¹⁵ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtFifteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l15Deg = twoSqrtFifteenLogPolyEffectiveDeg(num, k);
  if (s2l15Deg !== undefined) {
    const denDegS2l15 = polynomialDegreeInK(den, k);
    if (denDegS2l15 !== undefined) {
      if (denDegS2l15 > s2l15Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 138: Mul(Sqrt(P), Log(h1)×15, polynomial..., bounded...) numerator.
  // One Sqrt + fifteen Log factors; log¹⁵ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtFifteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l15Deg = oneSqrtFifteenLogPolyEffectiveDeg(num, k);
  if (s1l15Deg !== undefined) {
    const denDegS1l15 = polynomialDegreeInK(den, k);
    if (denDegS1l15 !== undefined) {
      if (denDegS1l15 > s1l15Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 137: Mul(Log(h1)×15, polynomial..., bounded...) numerator.
  // Zero Sqrt + fifteen Log factors; log¹⁵ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > fifteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl15Deg = fifteenLogPolyEffectiveDeg(num, k);
  if (sl15Deg !== undefined) {
    const denDegSl15 = polynomialDegreeInK(den, k);
    if (denDegSl15 !== undefined) {
      if (denDegSl15 > sl15Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 136: Mul(Sqrt(P1)×5, Log(h1)×14, polynomial..., bounded...) numerator.
  // Five Sqrt + fourteen Log factors; log¹⁴ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtFourteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l14Deg = fiveSqrtFourteenLogPolyEffectiveDeg(num, k);
  if (s5l14Deg !== undefined) {
    const denDegS5l14 = polynomialDegreeInK(den, k);
    if (denDegS5l14 !== undefined) {
      if (denDegS5l14 > s5l14Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 135: Mul(Sqrt(P1)×4, Log(h1)×14, polynomial..., bounded...) numerator.
  // Four Sqrt + fourteen Log factors; log¹⁴ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtFourteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l14Deg = fourSqrtFourteenLogPolyEffectiveDeg(num, k);
  if (s4l14Deg !== undefined) {
    const denDegS4l14 = polynomialDegreeInK(den, k);
    if (denDegS4l14 !== undefined) {
      if (denDegS4l14 > s4l14Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 134: Mul(Sqrt(P1)×3, Log(h1)×14, polynomial..., bounded...) numerator.
  // Three Sqrt + fourteen Log factors; log¹⁴ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtFourteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l14Deg = threeSqrtFourteenLogPolyEffectiveDeg(num, k);
  if (s3l14Deg !== undefined) {
    const denDegS3l14 = polynomialDegreeInK(den, k);
    if (denDegS3l14 !== undefined) {
      if (denDegS3l14 > s3l14Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 133: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×14, polynomial..., bounded...) numerator.
  // Two Sqrt + fourteen Log factors; log¹⁴ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtFourteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l14Deg = twoSqrtFourteenLogPolyEffectiveDeg(num, k);
  if (s2l14Deg !== undefined) {
    const denDegS2l14 = polynomialDegreeInK(den, k);
    if (denDegS2l14 !== undefined) {
      if (denDegS2l14 > s2l14Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 132: Mul(Sqrt(P), Log(h1)×14, polynomial..., bounded...) numerator.
  // One Sqrt + fourteen Log factors; log¹⁴ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtFourteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l14Deg = oneSqrtFourteenLogPolyEffectiveDeg(num, k);
  if (s1l14Deg !== undefined) {
    const denDegS1l14 = polynomialDegreeInK(den, k);
    if (denDegS1l14 !== undefined) {
      if (denDegS1l14 > s1l14Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 131: Mul(Log(h1)×14, polynomial..., bounded...) numerator.
  // Zero Sqrt + fourteen Log factors; log¹⁴ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > fourteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl14Deg = fourteenLogPolyEffectiveDeg(num, k);
  if (sl14Deg !== undefined) {
    const denDegSl14 = polynomialDegreeInK(den, k);
    if (denDegSl14 !== undefined) {
      if (denDegSl14 > sl14Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 130: Mul(Sqrt(P1)×5, Log(h1)×13, polynomial..., bounded...) numerator.
  // Five Sqrt + thirteen Log factors; log¹³ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fiveSqrtThirteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s5l13Deg = fiveSqrtThirteenLogPolyEffectiveDeg(num, k);
  if (s5l13Deg !== undefined) {
    const denDegS5l13 = polynomialDegreeInK(den, k);
    if (denDegS5l13 !== undefined) {
      if (denDegS5l13 > s5l13Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 129: Mul(Sqrt(P1)×4, Log(h1)×13, polynomial..., bounded...) numerator.
  // Four Sqrt + thirteen Log factors; log¹³ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > fourSqrtThirteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s4l13Deg = fourSqrtThirteenLogPolyEffectiveDeg(num, k);
  if (s4l13Deg !== undefined) {
    const denDegS4l13 = polynomialDegreeInK(den, k);
    if (denDegS4l13 !== undefined) {
      if (denDegS4l13 > s4l13Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 128: Mul(Sqrt(P1)×3, Log(h1)×13, polynomial..., bounded...) numerator.
  // Three Sqrt + thirteen Log factors; log¹³ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > threeSqrtThirteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s3l13Deg = threeSqrtThirteenLogPolyEffectiveDeg(num, k);
  if (s3l13Deg !== undefined) {
    const denDegS3l13 = polynomialDegreeInK(den, k);
    if (denDegS3l13 !== undefined) {
      if (denDegS3l13 > s3l13Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 127: Mul(Sqrt(P1), Sqrt(P2), Log(h1)×13, polynomial..., bounded...) numerator.
  // Two Sqrt + thirteen Log factors; log¹³ sub-polynomial → effective degree = sum(sqrtHalfDegs) + polyDeg.
  // Closes when denDeg > twoSqrtThirteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s2l13Deg = twoSqrtThirteenLogPolyEffectiveDeg(num, k);
  if (s2l13Deg !== undefined) {
    const denDegS2l13 = polynomialDegreeInK(den, k);
    if (denDegS2l13 !== undefined) {
      if (denDegS2l13 > s2l13Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 126: Mul(Sqrt(P), Log(h1)×13, polynomial..., bounded...) numerator.
  // One Sqrt + thirteen Log factors; log¹³ sub-polynomial → effective degree = sqrtHalfDeg + polyDeg.
  // Closes when denDeg > oneSqrtThirteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const s1l13Deg = oneSqrtThirteenLogPolyEffectiveDeg(num, k);
  if (s1l13Deg !== undefined) {
    const denDegS1l13 = polynomialDegreeInK(den, k);
    if (denDegS1l13 !== undefined) {
      if (denDegS1l13 > s1l13Deg) return true;
    } else if (hDivergesAtInfinity(den, k)) {
      return true;
    }
  }
  // Phase 125: Mul(Log(h1)×13, polynomial..., bounded...) numerator.
  // Zero Sqrt + thirteen Log factors; log¹³ sub-polynomial → effective degree = polyDeg.
  // Closes when denDeg > thirteenLogPolyEffectiveDeg or non-polynomial diverging denom.
  const sl13Deg = thirteenLogPolyEffectiveDeg(num, k);
  if (sl13Deg !== undefined) {
    const denDegSl13 = polynomialDegreeInK(den, k);
    if (denDegSl13 !== undefined) {
      if (denDegSl13 > sl13Deg) return true;
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
