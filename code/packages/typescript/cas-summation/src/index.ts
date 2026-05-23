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
