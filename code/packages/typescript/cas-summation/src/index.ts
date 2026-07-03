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

// Re-exported from ``gosper.ts``.  Track H2 — see PR #5366 (H1, Python).
export { tryGosperSum, MAX_POLY_DEGREE } from "./gosper";
import { tryGosperSum } from "./gosper";
// Re-exported from ``seriesClosedForms.ts``.  Track I2 — see PR #5382 (I1, Python).
export { tryClosedFormSeries, bernoulliRational } from "./seriesClosedForms";
import { tryClosedFormSeries } from "./seriesClosedForms";

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
  return evaluateSumInner(f, k, lo, hi, evalFn, false);
}

/**
 * Track B2 (TypeScript port of Phase 40 + Phase 46 from Python ``symbolic-vm``
 * ``sum_handler``).  The ``apartRetried`` flag prevents infinite recursion
 * when the retry path falls back to the outer entry point — Apart is
 * attempted at most once per top-level call.
 *
 * Apart-retry telescope chain
 * ---------------------------
 * When the structural telescope detector and every other narrow recogniser
 * fall through to the unevaluated ``Sum(...)`` shape AND the summand is a
 * proper rational ``Div(P(k), Q(k))``, we expand once via
 * ``Apart(f, k)`` (dispatched through the user-provided ``evalFn`` —
 * typically a ``symbolic-vm`` VM with the Apart handler installed) and
 * retry ``evaluateSum`` on the partial-fraction decomposed shape.
 *
 * The classic case is ``∑ 1/(k·(k+1))``: Apart emits
 * ``Add(Div(1, k), Div(-1, k+1))`` which the Phase 40+46 Add-with-negation
 * normaliser rewrites to ``Sub(1/k, 1/(k+1))`` so the telescope detector
 * fires and emits ``1 − 1/(hi+1)`` (finite) or ``1`` (infinite, since
 * ``1/(k+1) → 0``).  When ``evalFn`` does not dispatch Apart (e.g. a bare
 * arithmetic evaluator), ``apart_attempt`` retains the same head
 * (``Apply(Apart, ...)``), structurally differs from ``f``, but
 * ``evaluateSumInner`` will not close on it so the original unevaluated
 * Sum is returned — exactly the Python fall-through behaviour.
 */
function evaluateSumInner(
  f: IRNode,
  k: IRNode,
  lo: IRNode,
  hi: IRNode,
  evalFn: EvalFn,
  apartRetried: boolean,
): IRNode {
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

  // Track I2 — closed-form transcendental infinite sums.  Recognises the
  // canonical zeta(2m), eta(2m), eta(1) = log(2), e_series, exp/cos/sin/
  // cosh/sinh Taylor series.  Mirrors the Python dispatch insertion
  // point (step 5a): placed after ``trySpecialInfinite`` so its
  // pre-existing patterns (Basel zeta(2)/zeta(4), Leibniz π/4) keep
  // their IR shapes and tests; ``tryClosedFormSeries`` only fires on
  // patterns the legacy handler refuses (e.g. ``Σ 1/k⁶``, the eta
  // family, sin/cos/sinh/cosh).
  if (infUpper) {
    const raw = tryClosedFormSeries(f, k, lo, hi);
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

  // Track H2 — Gosper hypergeometric closed-form attempt.  Runs after
  // all narrow recognisers (constant, geometric, Faulhaber, telescoping,
  // small-range numeric, special infinite series) but before the
  // Apart-retry telescope chain and the unevaluated fallthrough.
  //
  // Mirrors the Python dispatch insertion point in
  // ``cas_summation.summation`` (step 5b): Gosper only runs for *finite*
  // upper bounds because the algorithm returns ``T(hi+1) − T(lo)`` which
  // is only meaningful when ``hi+1`` is a real value.  Infinite upper
  // bounds belong to the dedicated limit-aware paths above (telescope at
  // ∞, classic series).  This guard also preserves the Phase 41 fall-
  // through contract for non-vanishing telescopes.
  if (!infUpper) {
    const gosper = tryGosperSum(f, k, lo, hi);
    if (gosper !== undefined) return evalFn(gosper);
  }

  // Track B2 — Apart-retry telescope chain.  Mirrors the Python
  // ``sum_handler`` Phase 40 / Phase 46 retry path: when every direct rule
  // above fails on a rational summand, expand once via ``Apart(f, k)``
  // (dispatched through the user-provided ``evalFn``, i.e. a real VM with
  // the Apart handler installed) and try the whole pipeline again.  The
  // ``apartRetried`` guard pins the retry to at most one round, matching
  // Python's structural one-shot behaviour and guaranteeing termination.
  //
  // Only attempted when ``f`` is structurally ``Div(num, den)`` — Apart
  // leaves other heads unchanged, so this saves a wasted round-trip
  // through the VM dispatch.  When ``apart_attempt`` is structurally
  // equal to ``f`` (e.g. denominator irreducible over ℚ, or Apart isn't
  // wired into ``evalFn``), no retry is performed.
  if (!apartRetried && f.kind === "apply" && equals(f.head, DIV)) {
    const apartAttempt = evalFn(app(sym("Apart"), [f, k]));
    if (!equals(apartAttempt, f)) {
      // Apart emits two-term partial fractions as ``Add(a, Div(-c, d))``
      // or ``Add(Neg(a), b)``.  The structural telescope detector keys
      // off ``Sub`` heads, so normalise via the existing helper before
      // retrying.  ``normaliseAddNegToSub`` is a no-op when the head is
      // not an Add — safe to apply unconditionally.
      const normalised = normaliseAddNegToSub(apartAttempt);
      const retry = evaluateSumInner(normalised, k, lo, hi, evalFn, true);
      // Only return when the retry actually closed the sum (i.e. it is
      // no longer the unevaluated ``Sum(...)`` head).  Otherwise fall
      // through and return the original unevaluated form below.
      if (!(retry.kind === "apply" && equals(retry.head, SUM))) {
        return retry;
      }
    }
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
    return canonicaliseAddOperandOrder(node);
  }
  const [left, right] = node.args;
  const leftPos = extractNegation(left);
  const rightPos = extractNegation(right);
  if (leftPos !== undefined && rightPos !== undefined) {
    // Both sides genuinely negative — no telescope to expose.
    return canonicaliseAddOperandOrder(node);
  }
  if (rightPos !== undefined) {
    return canonicaliseAddOperandOrder(app(SUB, [left, rightPos]));
  }
  if (leftPos !== undefined) {
    return canonicaliseAddOperandOrder(app(SUB, [right, leftPos]));
  }
  return canonicaliseAddOperandOrder(node);
}

/**
 * Track B2 (TypeScript port of Python ``_canonicalise_add_operand_order``):
 * deep-rewrite every ``Add`` so that numeric literals appear *last* among
 * its arguments.
 *
 * The symbolic VM doesn't currently impose a canonical operand order on
 * ``Add``, so two structurally distinct trees can represent the same
 * mathematical expression — e.g. ``Add(k, 1)`` vs ``Add(1, k)``.  The
 * Phase 39 telescope detector relies on ``==`` after ``evalFn``, so
 * these must look identical for the Apart-rewritten summand to match the
 * substituted half (``Apart`` emits ``Add(1, k)`` while substitution
 * produces ``Add(k, 1)``).
 *
 * Walks the tree and sorts each ``Add``'s arguments so that integer /
 * rational / float literals come *last*, preserving the relative order of
 * non-literal children.  Other heads recurse into their arguments unchanged.
 */
function canonicaliseAddOperandOrder(node: IRNode): IRNode {
  if (node.kind !== "apply") return node;
  const newArgs = node.args.map(canonicaliseAddOperandOrder);
  if (irEquals(node.head, ADD) && newArgs.length >= 2) {
    const literals: IRNode[] = [];
    const nonLiterals: IRNode[] = [];
    for (const arg of newArgs) {
      if (arg.kind === "integer" || arg.kind === "rational" || arg.kind === "float") {
        literals.push(arg);
      } else {
        nonLiterals.push(arg);
      }
    }
    if (literals.length > 0 && nonLiterals.length > 0) {
      const reordered = [...nonLiterals, ...literals];
      // Only rebuild when the order actually changed.
      const changed = reordered.some((arg, idx) => arg !== newArgs[idx]);
      if (changed) {
        return app(node.head, reordered);
      }
    }
  }
  const argsChanged = newArgs.some((arg, idx) => arg !== node.args[idx]);
  return argsChanged ? app(node.head, newArgs) : node;
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
 * Return ``Σ sqrtHalfDeg + Σ polyDeg`` when ``node`` is a ``Mul`` whose
 * factors split into any combination of:
 *
 *   - ``Log(diverging)`` factors (any count, including zero),
 *   - ``Sqrt(positive-leading polynomial)`` factors (any count),
 *   - polynomial factors in ``k`` (any count),
 *   - bounded-in-``k`` factors (any count, e.g. ``Sin``, ``Cos``, constants),
 *
 * and at least one of the Log/Sqrt/polynomial factors is present.  Returns
 * ``undefined`` when any factor is unrecognised (e.g. ``Exp(k)``) or when
 * the numerator is purely bounded.
 *
 * **Phase 86 — cleanup.**  Supersedes the hand-written grid of
 * ``N-Sqrt × M-Log × polynomial`` helpers (Phases 59-85).  The convergence
 * math is identical for every non-negative ``(N, M)`` pair:
 *
 *   - The product of ``N`` ``Log(diverging)`` factors is sub-polynomial —
 *     ``log^N(k) = o(k^ε)`` for any ``ε > 0`` — so ``N`` contributes ``0``
 *     to the effective growth degree.
 *   - Each ``Sqrt(P_i)`` contributes ``deg(P_i)/2`` (here as a fractional
 *     half-degree, matching the existing TS-port convention).
 *   - Each polynomial factor ``Q_j`` contributes its own ``deg(Q_j)``.
 *   - Bounded factors contribute ``0``.
 *
 * Effective growth:
 *
 *   effective = Σ_i sqrtHalfDeg(Sqrt(P_i)) + Σ_j deg(Q_j)
 *
 * Caller compares ``denDeg > effective`` (polynomial denominator) or
 * short-circuits on non-polynomial diverging denominator.
 *
 * Conservative refusals:
 *
 *   - Empty ``Mul`` (no recognised growth factor) → undefined.
 *   - ``Sqrt`` of a polynomial whose leading coefficient is negative → undefined.
 *   - Any unrecognised factor (Exp, free symbol, …) → undefined.
 */
function logSqrtPolyEffectiveDegGeneric(
  node: IRNode,
  k: IRNode,
): number | undefined {
  if (node.kind !== "apply" || !equals(node.head, MUL)) return undefined;
  let sqrtHalfDegSum = 0;
  let polyDegSum = 0;
  let foundLog = false;
  let foundSqrt = false;
  let foundPoly = false;
  for (const arg of node.args) {
    if (isLogOfDivergingInK(arg, k)) {
      foundLog = true;
      continue;
    }
    const sqrtHalfDeg = sqrtEffectiveHalfDegree(arg, k);
    if (sqrtHalfDeg !== undefined) {
      sqrtHalfDegSum += sqrtHalfDeg;
      foundSqrt = true;
      continue;
    }
    if (isBoundedInK(arg, k)) {
      // Constants and Sin/Cos/closures — contribute nothing.
      continue;
    }
    const polyDeg = polynomialDegreeInK(arg, k);
    if (polyDeg !== undefined && polyDeg >= 1) {
      polyDegSum += polyDeg;
      foundPoly = true;
      continue;
    }
    // Unrecognised factor (Exp, free symbol, …) — bail.
    return undefined;
  }
  if (!foundLog && !foundSqrt && !foundPoly) {
    // Pure-bounded numerator — let Phase 49 handle it.
    return undefined;
  }
  return sqrtHalfDegSum + polyDegSum;
}

function vanishesAtInfinity(node: IRNode, k: IRNode): boolean {
  if (isConstantIn(node, k)) {
    const value = rationalValue(node);
    return value !== undefined && value.numer === 0n;
  }
  if (node.kind !== "apply") return false;
  if (equals(node.head, NEG) && node.args.length === 1) {
    return vanishesAtInfinity(node.args[0], k);
  }
  if (equals(node.head, ADD)) {
    return node.args.every((arg) => vanishesAtInfinity(arg, k));
  }
  if (equals(node.head, EXP) && node.args.length === 1) {
    const inner = node.args[0];
    const degree = polynomialDegreeInK(inner, k);
    return (
      degree !== undefined &&
      degree > 0 &&
      polynomialLeadingCoeffSignInK(inner, k) === -1
    );
  }
  if (equals(node.head, POW) && node.args.length === 2) {
    const [base, exp] = node.args;
    if (isConstantIn(base, k)) {
      const baseVal = rationalValue(base);
      if (baseVal !== undefined) {
        const absNumer = baseVal.numer < 0n ? -baseVal.numer : baseVal.numer;
        if (absNumer > baseVal.denom) {
          const degree = polynomialDegreeInK(exp, k);
          return (
            degree !== undefined &&
            degree > 0 &&
            polynomialLeadingCoeffSignInK(exp, k) === -1
          );
        }
      }
    }
  }
  if (equals(node.head, MUL)) {
    let hasVanishing = false;
    for (const arg of node.args) {
      if (isConstantIn(arg, k)) {
        const value = rationalValue(arg);
        if (value !== undefined && value.numer === 0n) return true;
        continue;
      }
      if (isBoundedInK(arg, k)) continue;
      if (vanishesAtInfinity(arg, k)) {
        hasVanishing = true;
        continue;
      }
      return false;
    }
    return hasVanishing;
  }
  return false;
}

function gVanishesAtInfinity(g: IRNode, k: IRNode): boolean {
  if (vanishesAtInfinity(g, k)) return true;
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
  // ---- Generic recogniser (Phase 86 cleanup) ----
  //
  // Mul(bounded..., Log_1, ..., Log_N, Sqrt_1, ..., Sqrt_M, Poly_1, ..., Poly_K)
  // over diverging denominator: the product of any number of
  // ``Log(diverging)`` factors is still sub-polynomial
  // (``log^N(k) = o(k^ε)``), so ``N`` drops out of the comparison.  The
  // Sqrt factors contribute ``Σ deg(P_i)/2`` and the polynomial factors
  // contribute ``Σ deg(Q_j)``.  Effective:
  //
  //   effective = Σ sqrtHalfDeg + Σ polyDeg
  //
  // Vanishes when ``denDeg > effective``; non-polynomial diverging
  // denominators dominate automatically.
  //
  // Supersedes the hand-written grid of ``N-Sqrt × M-Log × polynomial``
  // helpers (Phases 59-85): the math is identical for every (N, M) ≥ (0, 0).
  // The hardcoded helpers remain in place for now but are preempted by this
  // branch; a follow-up cleanup PR will delete them.
  const genDeg = logSqrtPolyEffectiveDegGeneric(num, k);
  if (genDeg !== undefined) {
    const denDegGen = polynomialDegreeInK(den, k);
    if (denDegGen !== undefined) {
      if (denDegGen > genDeg) {
        return true;
      }
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
