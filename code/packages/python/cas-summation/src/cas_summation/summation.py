"""Main dispatcher for symbolic summation and product evaluation.

This module provides two public functions:

- ``evaluate_sum(f, k, lo, hi, vm)``   — evaluates Σ_{k=lo}^{hi} f(k)
- ``evaluate_product(f, k, lo, hi, vm)`` — evaluates Π_{k=lo}^{hi} f(k)

Both return an IR node representing the closed form, or return the
unevaluated ``SUM``/``PRODUCT`` node when no pattern matches.

Dispatch order for ``evaluate_sum``
------------------------------------

1. **Constant** — f does not contain k:
       Σ c = c · (hi − lo + 1)

2. **Geometric** — f = coeff · base^k (base constant in k):
       Finite:   coeff · base^lo · (base^(hi−lo+1) − 1) / (base − 1)
       Infinite: coeff · base^lo / (1 − base)

3. **Power of index** — f = coeff · k^m (m = 0…5):
       Uses Faulhaber's formula  Σ_{k=lo}^{hi} k^m = F(hi,m) − F(lo−1,m)

4. **Telescoping** (Phase 39 finite + Phase 41 infinite) —
   f = g(k+1) − g(k) (or its antisymmetric):
       Σ_{k=lo}^{hi} [g(k+1) − g(k)] = g(hi+1) − g(lo)         (finite)
       Σ_{k=lo}^{hi} [g(k) − g(k+1)] = g(lo) − g(hi+1)         (finite)
       Σ_{k=lo}^{∞}  [g(k+1) − g(k)] = −g(lo)                   (Phase 41)
       Σ_{k=lo}^{∞}  [g(k) − g(k+1)] =  g(lo)                   (Phase 41)
   The infinite case only fires when ``g(k)`` provably vanishes at
   infinity (``Div(constant, positive-degree-polynomial-in-k)`` shapes).
   Pure structural detection: substitute k → k+1 in one half and
   compare to the other half after VM normalisation.

5. **Classic infinite series** — when hi = %inf (or inf):
       Σ 1/k²        → π²/6
       Σ 1/k⁴        → π⁴/90
       Σ (-1)^k/(2k+1) → π/4
       Σ 1/k!        → %e
       Σ x^k/k!      → exp(x)

6. **Numeric small range** — lo and hi are concrete integers (range ≤ 1000):
       Compute directly via repeated substitution + VM eval.

7. **Fallback** — return unevaluated ``SUM(f, k, lo, hi)``.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    DIV,
    EXP,
    LOG,
    MUL,
    NEG,
    POW,
    PRODUCT,
    SQRT,
    SUB,
    SUM,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_summation.geometric_sum import geometric_sum_ir
from cas_summation.poly_sum import poly_sum_ir
from cas_summation.product_eval import evaluate_product_expr
from cas_summation.special_sums import try_special_infinite

# ---------------------------------------------------------------------------
# Internal helpers
# ---------------------------------------------------------------------------


def _int(n: int) -> IRNode:
    return IRInteger(n)


def _frac(c: Fraction) -> IRNode:
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _is_inf(node: IRNode) -> bool:
    """True iff *node* represents +∞ (both MACSYMA ``%inf`` and raw ``inf``)."""
    return isinstance(node, IRSymbol) and node.name in {"inf", "%inf"}


def _is_constant_in(f: IRNode, k: IRSymbol) -> bool:
    """True iff *f* contains no occurrence of *k*."""
    if f == k:
        return False
    if isinstance(f, IRApply):
        return all(_is_constant_in(arg, k) for arg in f.args)
    return True


def _ir_rational_val(node: IRNode) -> Fraction | None:
    """Return the value of an integer/rational IR literal, or None."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    return None


def _ir_int_val(node: IRNode) -> int | None:
    """Return the Python int value of an IRInteger, or None."""
    return node.value if isinstance(node, IRInteger) else None


def _try_geometric(
    f: IRNode, k: IRSymbol
) -> tuple[IRNode, IRNode] | None:
    """If f = coeff · base^k, return (coeff, base); else None.

    Handles:
    - base^k              → (IRInteger(1), base)
    - coeff * base^k      → (coeff, base)   where coeff is constant in k
    - base^k * coeff      → (coeff, base)
    - 1 / base^k          → (IRInteger(1), 1/base)   e.g. 1/2^k = (1/2)^k
    - coeff / base^k      → (coeff, 1/base)
    """
    # Direct: base^k  where base is constant in k (and base ≠ k)
    if (
        isinstance(f, IRApply)
        and f.head == POW
        and len(f.args) == 2
        and f.args[1] == k
        and _is_constant_in(f.args[0], k)
        and f.args[0] != k
    ):
        return (IRInteger(1), f.args[0])

    # MUL(c, base^k) or MUL(base^k, c)
    if isinstance(f, IRApply) and f.head == MUL and len(f.args) == 2:
        a, b = f.args
        for coeff_cand, pow_cand in ((a, b), (b, a)):
            if (
                isinstance(pow_cand, IRApply)
                and pow_cand.head == POW
                and len(pow_cand.args) == 2
                and pow_cand.args[1] == k
                and _is_constant_in(pow_cand.args[0], k)
                and pow_cand.args[0] != k
                and _is_constant_in(coeff_cand, k)
            ):
                return (coeff_cand, pow_cand.args[0])

    # DIV(coeff, base^k) — recognise  coeff / base^k  as  coeff * (1/base)^k.
    # e.g. sum(1/2^k, k, 0, inf) → (1/2)^k with ratio 1/2.
    if isinstance(f, IRApply) and f.head == DIV and len(f.args) == 2:
        numer, denom = f.args
        if _is_constant_in(numer, k):
            # Check denominator is base^k.
            if (
                isinstance(denom, IRApply)
                and denom.head == POW
                and len(denom.args) == 2
                and denom.args[1] == k
                and _is_constant_in(denom.args[0], k)
                and denom.args[0] != k
            ):
                base = denom.args[0]
                recip_base = IRApply(DIV, (IRInteger(1), base))
                return (numer, recip_base)

    return None


def _try_telescoping(
    f: IRNode, k: IRSymbol, vm: object
) -> tuple[IRNode, IRNode] | None:
    """Detect a *structurally telescoping* summand ``f = g(k+1) − g(k)``.

    Returns the pair ``(g_expr, sign)`` where:
    - ``g_expr`` is the expression representing ``g(k)`` (i.e. the
      "minus" half of the SUB shape).  The closed form is then
      ``g(hi+1) − g(lo)``.
    - ``sign`` is ``+1`` for the standard ``g(k+1) − g(k)`` orientation
      and ``-1`` for the antisymmetric ``g(k) − g(k+1)`` form
      (in which case the closed form is ``g(lo) − g(hi+1)``).

    The detection is purely structural: we substitute ``k → k+1`` in one
    half of the SUB and compare to the other half *after* normalising
    both through ``vm.eval``.  No partial-fraction decomposition is
    attempted here — that would be a follow-on phase.  Returns ``None``
    when the shape doesn't match (caller will fall through to later
    rules or to the unevaluated fallback).

    Detection table
    ---------------
    +-------------------------+----------------+----------------------------+
    | Summand shape           | Returns        | Closed form                |
    +=========================+================+============================+
    | ``g(k+1) − g(k)``       | ``(g, +1)``    | ``g(hi+1) − g(lo)``        |
    | ``g(k) − g(k+1)``       | ``(g, −1)``    | ``g(lo) − g(hi+1)``        |
    +-------------------------+----------------+----------------------------+

    Examples
    --------
    - ``∑_{k=1}^{N} [(k+1)² − k²] = (N+1)² − 1 = N² + 2N``
    - ``∑_{k=1}^{N} [1/k − 1/(k+1)] = 1 − 1/(N+1)`` (antisymmetric)

    Notes
    -----
    The classic ``1/(k(k+1))`` form is recognisable *only after* a
    partial-fraction expansion to ``1/k − 1/(k+1)``.  This rule does
    not perform that expansion; a Phase 40-style helper could compose
    it later by trying ``Apart(f, k)`` before this check.
    """
    if not isinstance(f, IRApply) or f.head != SUB or len(f.args) != 2:
        return None
    from cas_substitution import subst

    left, right = f.args
    k_plus_one = IRApply(ADD, (k, IRInteger(1)))
    # Helper: True when ``a[k → k+1]`` evaluates equal to ``b``.
    def shifted_equals(a: IRNode, b: IRNode) -> bool:
        shifted = subst(k_plus_one, k, a)
        return vm.eval(shifted) == vm.eval(b)  # type: ignore[attr-defined]

    # Standard orientation: f = g(k+1) − g(k).  Check whether
    # ``right[k → k+1] == left``: that is, ``g(k)`` shifted equals
    # ``g(k+1)``.
    if shifted_equals(right, left):
        # right is g(k); standard orientation.
        return right, IRInteger(1)
    # Antisymmetric: f = g(k) − g(k+1).  Check whether
    # ``left[k → k+1] == right``.
    if shifted_equals(left, right):
        # left is g(k); antisymmetric orientation.
        return left, IRInteger(-1)
    return None


def _is_positive_degree_polynomial_in_k(node: IRNode, k: IRSymbol) -> bool:
    """Conservative recogniser: True when ``node`` is a polynomial in ``k``
    of strictly positive degree.

    Used by :func:`_g_vanishes_at_infinity` to decide whether a denominator
    grows without bound as ``k → ∞`` (in which case ``c / denominator → 0``).

    Recognised shapes
    -----------------

    +-------------------------------------+
    | ``k`` itself                        |
    | ``k^n`` with ``n ≥ 1`` integer      |
    | ``Add(...)`` with at least one      |
    |   positive-degree term, all other   |
    |   args either constant-in-k or      |
    |   positive-degree                   |
    | ``Mul(...)`` where at least one     |
    |   factor has positive degree, all   |
    |   other factors are constant-in-k   |
    |   or positive-degree                |
    +-------------------------------------+

    Anything else returns ``False`` — most importantly, ``Div`` shapes
    (e.g. ``1/k``) are rejected because their limit is 0, not ∞, which
    would make a ``c / (1/k)`` shape *not* vanish at infinity.
    """
    # k itself — degree 1.
    if node == k:
        return True
    if not isinstance(node, IRApply):
        return False
    # k^n with integer n ≥ 1.
    if node.head == POW and len(node.args) == 2:
        base, exp = node.args
        if base == k and isinstance(exp, IRInteger) and exp.value >= 1:
            return True
    # Add(...): at least one term has positive degree; every other
    # argument must be constant-in-k or itself positive-degree.
    if node.head == ADD and len(node.args) >= 2:
        if not any(
            _is_positive_degree_polynomial_in_k(arg, k) for arg in node.args
        ):
            return False
        return all(
            _is_constant_in(arg, k) or _is_positive_degree_polynomial_in_k(arg, k)
            for arg in node.args
        )
    # Mul(...): at least one factor has positive degree; every other
    # factor must be constant-in-k or also positive-degree.
    if node.head == MUL and len(node.args) >= 2:
        has_positive = False
        for arg in node.args:
            if _is_constant_in(arg, k):
                continue
            if _is_positive_degree_polynomial_in_k(arg, k):
                has_positive = True
                continue
            return False  # unrecognised k-dependence (e.g. 1/k factor)
        return has_positive
    return False


def _polynomial_degree_in_k(node: IRNode, k: IRSymbol) -> int | None:
    """Return the polynomial degree of ``node`` in ``k`` (Phase 42).

    Returns ``0`` for expressions constant in ``k``, ``1`` for bare
    ``k``, ``n`` for ``k^n``, the maximum of children for ``Add``, and
    the sum of children for ``Mul``.  Returns ``None`` for shapes that
    are not pure polynomials in ``k`` (e.g. ``Div``, ``Sin``, ``Cos``,
    ``Log``, ``Pow(k, fractional)``, ``Pow(non-constant, k)``).

    The polynomial degree is used by :func:`_g_vanishes_at_infinity` to
    decide whether a rational ``P(k)/Q(k)`` summand tends to 0 as
    ``k → ∞`` — true iff ``deg(P) < deg(Q)``.

    Recognised shapes
    -----------------

    +-------------------------+-------------------------+
    | Input                   | Degree                  |
    +=========================+=========================+
    | constant in ``k``       | ``0``                   |
    | ``k``                   | ``1``                   |
    | ``k^n`` integer n ≥ 0   | ``n``                   |
    | ``Neg(p)``              | ``deg(p)``              |
    | ``Add(p1, p2, …)``      | ``max(deg(pi))``        |
    | ``Sub(p1, p2)``         | ``max(deg(p1), deg(p2))`` |
    | ``Mul(p1, p2, …)``      | ``sum(deg(pi))``        |
    | otherwise (Div, Pow,…)  | ``None``                |
    +-------------------------+-------------------------+
    """
    # Constant in k — degree 0.
    if _is_constant_in(node, k):
        return 0
    # Bare k — degree 1.
    if node == k:
        return 1
    if not isinstance(node, IRApply):
        return None
    # k^n where n is a non-negative integer.
    if node.head == POW and len(node.args) == 2:
        base, exp = node.args
        if base == k and isinstance(exp, IRInteger) and exp.value >= 0:
            return int(exp.value)
        # Pow(non-k base depending on k, …) or fractional exponent: not a
        # pure polynomial in k.
        return None
    # Unary Neg preserves degree.
    if node.head == NEG and len(node.args) == 1:
        return _polynomial_degree_in_k(node.args[0], k)
    # ADD / SUB: max of child degrees (None propagates).
    if node.head in (ADD, SUB):
        degrees = [_polynomial_degree_in_k(a, k) for a in node.args]
        if any(d is None for d in degrees):
            return None
        return max(degrees)  # type: ignore[arg-type]
    # MUL: sum of child degrees (None propagates).
    if node.head == MUL:
        degrees = [_polynomial_degree_in_k(a, k) for a in node.args]
        if any(d is None for d in degrees):
            return None
        return sum(degrees)  # type: ignore[misc]
    # Div, Sin, Cos, Log, Exp, Sqrt, … — not pure polynomials.
    return None


def _polynomial_leading_coeff_sign_in_k(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 43 helper: return the *sign* of the leading coefficient of
    ``node`` as a polynomial in ``k``, or ``None`` if ``node`` is not a
    polynomial in ``k`` (or has degree 0).

    Returns ``+1`` or ``−1``.  Used by :func:`_h_diverges_at_infinity` to
    decide whether the polynomial exponent inside ``Exp`` / ``Pow``
    actually drives the value toward ``+∞`` (sign ``+1``, divergent) or
    toward ``−∞`` (sign ``−1``, makes ``exp(h)`` / ``b^h`` → 0).

    A naïve "positive-degree polynomial" check (Phase 41/42) is *not*
    sign-aware: ``Mul(-1, k)`` (the canonical IR for ``-k``) passes the
    positive-degree test but its leading coefficient is ``-1``.  Without
    this helper, ``Exp(-k)`` and ``Pow(2, -k)`` would be wrongly claimed
    to diverge — they actually vanish.

    Recognised shapes
    -----------------

    +-----------------------------+-------------------------------+
    | Input                       | Sign of leading coefficient   |
    +=============================+===============================+
    | constant in k               | ``None`` (degree 0)           |
    | ``k``                       | +1                            |
    | ``k^n`` with n ≥ 1 integer  | +1                            |
    | ``Neg(p)``                  | flips sign of ``p``           |
    | ``Mul(c, p)`` with ``c``    | ``sign(c) * sign(p)``         |
    |   constant in k (rational)  |                               |
    | ``Mul(p1, p2, …)`` pure     | product of signs              |
    |   polynomials in k          |                               |
    | ``Add(p1, p2, …)`` — sign   | sign of the maximum-degree    |
    |   of the highest-degree     | term (assuming non-cancelling |
    |   term                      | leading coefficients)         |
    | anything else (Div, Sin,    | ``None``                      |
    |   fractional Pow, …)        |                               |
    +-----------------------------+-------------------------------+
    """
    # Constant in k — no leading coefficient (degree 0).
    if _is_constant_in(node, k):
        return None
    if node == k:
        return 1
    if not isinstance(node, IRApply):
        return None
    # k^n with n ≥ 1.
    if node.head == POW and len(node.args) == 2:
        base, exp_arg = node.args
        if (
            base == k
            and isinstance(exp_arg, IRInteger)
            and exp_arg.value >= 1
        ):
            return 1
        return None
    # Neg(p): flip sign.
    if node.head == NEG and len(node.args) == 1:
        inner = _polynomial_leading_coeff_sign_in_k(node.args[0], k)
        return None if inner is None else -inner
    # Mul(...): pick the k-bearing factor(s) and multiply their signs;
    # constants-in-k contribute their own sign.
    if node.head == MUL:
        sign = 1
        any_k_bearing = False
        for arg in node.args:
            if _is_constant_in(arg, k):
                # Constant factor — multiply by its sign.
                val = _ir_rational_val(arg)
                if val is None:
                    # Symbolic constant whose sign we can't decide.
                    return None
                if val == 0:
                    return None  # Vanishing factor; not a leading coeff.
                if val < 0:
                    sign = -sign
                continue
            inner = _polynomial_leading_coeff_sign_in_k(arg, k)
            if inner is None:
                return None
            sign = sign if inner == 1 else -sign
            any_k_bearing = True
        return sign if any_k_bearing else None
    # Add(...): the highest-degree term dominates.  Walk children,
    # picking the one with the largest polynomial degree and use its
    # leading-coefficient sign.  If multiple children share the maximum
    # degree we conservatively return ``None`` (could cancel).
    if node.head == ADD:
        max_deg: int = -1
        leader_sign: int | None = None
        tied_at_max = False
        for arg in node.args:
            deg = _polynomial_degree_in_k(arg, k)
            if deg is None:
                return None  # non-polynomial child — refuse
            if deg == 0:
                continue  # constant terms don't determine leading sign
            if deg > max_deg:
                max_deg = deg
                leader_sign = _polynomial_leading_coeff_sign_in_k(arg, k)
                tied_at_max = False
            elif deg == max_deg:
                tied_at_max = True
        if tied_at_max:
            # Two or more terms share the highest degree; the leading
            # coefficient could cancel, so we conservatively refuse.
            return None
        return leader_sign
    return None


def _h_diverges_at_infinity(node: IRNode, k: IRSymbol) -> bool:
    """Phase 43: Return True when ``node`` provably diverges (to ±∞) as
    ``k → ∞``.

    A union of the Phase 41/42 positive-degree polynomial recogniser and
    new transcendental cases:

    +-----------------------------------+------------------------------+
    | Input                             | Diverges?                    |
    +===================================+==============================+
    | positive-degree polynomial in k   | yes (Phase 41/42)            |
    | ``Exp(h(k))`` with ``h``          | yes (Phase 43)               |
    |   diverging                       |                              |
    | ``Pow(b, h(k))`` with             | yes (Phase 43)               |
    |   ``b ∈ ℤ``, ``|b| > 1`` rational |                              |
    |   and ``h`` diverging             |                              |
    | ``Mul(...)`` where at least       | yes (Phase 43)               |
    |   one factor diverges and others  |                              |
    |   are constant-in-k or diverging  |                              |
    | anything else                     | no (conservatively)          |
    +-----------------------------------+------------------------------+

    Notes
    -----
    The ``b`` check uses ``|b| > 1`` so both ``2`` and ``−2`` (and
    rationals like ``3/2``) count.  For negative bases the sign
    oscillates but the magnitude diverges; ``c / b^k`` still tends to
    0 because the absolute value of the denominator grows.
    """
    # Phase 41/42 fast path — pure polynomial in k of positive degree.
    if _is_positive_degree_polynomial_in_k(node, k):
        return True
    if not isinstance(node, IRApply):
        return False
    # Phase 43: Exp(h(k)) with h → +∞.  `exp(+∞) = +∞` but `exp(−∞) = 0`,
    # so we MUST verify the leading coefficient of h is positive — a
    # naïve "positive-degree polynomial" check accepts ``Mul(-1, k)``
    # (i.e. ``-k``) whose leading coefficient is ``-1``, in which case
    # ``exp(h) → 0`` and the closed form would be silently wrong.
    if node.head == EXP and len(node.args) == 1:
        inner = node.args[0]
        if _is_positive_degree_polynomial_in_k(inner, k):
            return _polynomial_leading_coeff_sign_in_k(inner, k) == 1
        return False
    # Phase 43: Pow(b, h(k)) with |b| > 1 rational and h → +∞.  Same
    # sign-of-leading-coefficient requirement as the Exp branch —
    # ``Pow(2, Mul(-1, k))`` is ``2^(-k) → 0``, not ∞.
    if node.head == POW and len(node.args) == 2:
        base, exp = node.args
        if _is_constant_in(base, k):
            base_val = _ir_rational_val(base)
            if base_val is not None and abs(base_val) > 1:
                if _is_positive_degree_polynomial_in_k(exp, k):
                    if _polynomial_leading_coeff_sign_in_k(exp, k) == 1:
                        return True
    # Phase 44: Log(h(k)) where h(k) → +∞.  Two requirements:
    #   (a) h(k) → +∞ (not just |h| → ∞)
    #   (b) h(k) > 0 for k sufficiently large
    # so that log(h) is real-valued and diverges to +∞.  We can't reuse
    # the bare `_h_diverges_at_infinity` recursion because that helper's
    # Phase 41/42 polynomial-magnitude branch ignores sign — it accepts
    # ``Mul(-1, k)`` whose magnitude diverges but whose *value* goes to
    # −∞, in which case ``log(h)`` would be complex / undefined.  So:
    #   • Polynomial h: require positive leading coefficient explicitly
    #     via `_polynomial_leading_coeff_sign_in_k`.
    #   • Exp(h'): always positive (exp(x) > 0 for real x); defer to
    #     `_h_diverges_at_infinity` which has the sign-aware check on h'.
    #   • Pow(b, h'): require base ``b > 1`` (strictly positive, not just
    #     ``|b| > 1``) so the value is positive.  ``Pow(-2, k)``
    #     oscillates in sign and ``log((-2)^k)`` is not real-valued.
    # Other shapes are conservatively refused.
    if node.head == LOG and len(node.args) == 1:
        inner = node.args[0]
        if _is_positive_degree_polynomial_in_k(inner, k):
            return _polynomial_leading_coeff_sign_in_k(inner, k) == 1
        if isinstance(inner, IRApply) and inner.head == EXP:
            return _h_diverges_at_infinity(inner, k)
        if isinstance(inner, IRApply) and inner.head == POW and len(inner.args) == 2:
            base = inner.args[0]
            if _is_constant_in(base, k):
                base_val = _ir_rational_val(base)
                if base_val is not None and base_val > 1:
                    # Strictly positive base > 1 — value is positive and
                    # diverges.  Defer to the Pow branch's sign-aware
                    # check on the exponent.
                    return _h_diverges_at_infinity(inner, k)
        return False
    # Phase 43: Mul(...) — at least one factor diverges, others are
    # constant in k or also diverging.  Recursive.
    if node.head == MUL and len(node.args) >= 2:
        has_divergent = False
        for arg in node.args:
            if _is_constant_in(arg, k):
                continue
            if _h_diverges_at_infinity(arg, k):
                has_divergent = True
                continue
            return False  # unrecognised k-dependence
        return has_divergent
    return False


_SIN = IRSymbol("Sin")
_COS = IRSymbol("Cos")


def _is_bounded_in_k(node: IRNode, k: IRSymbol) -> bool:
    """Return True when ``node`` is *provably* uniformly bounded in ``k``.

    Phase 49 — used by :func:`_g_vanishes_at_infinity` to recognise
    shapes like ``sin(k)/k²`` where the numerator is bounded
    (``|sin(k)| ≤ 1``) and the denominator diverges, hence the
    quotient vanishes.

    +-----------------------------+----------------------------+
    | ``node`` shape              | Provably bounded?          |
    +=============================+============================+
    | constant in ``k``           | yes (trivially)            |
    | ``Sin(h(k))`` / ``Cos(...)``| yes (``|sin|, |cos| ≤ 1``) |
    | ``Mul(bounded, bounded)``   | yes (recursive)            |
    | ``Add(bounded, bounded)``   | yes (recursive)            |
    | ``Neg(bounded)``            | yes (sign flip preserves)  |
    | ``k`` / ``k²``              | no (diverges)              |
    | ``Exp(k)``                  | no (diverges)              |
    | ``Log(k)``                  | no (diverges)              |
    +-----------------------------+----------------------------+

    Conservative — when in doubt, returns False so the caller falls
    through to the unevaluated ``Sum(...)`` form.
    """
    # Trivial case: nothing depending on k is bounded by being
    # constant in k.
    if _is_constant_in(node, k):
        return True
    if not isinstance(node, IRApply):
        # Bare ``k`` is *not* bounded; it diverges.
        return False
    # Sin / Cos with any inner argument are bounded by 1 in modulus.
    # The inner argument can depend on k freely — ``sin(k²)`` is still
    # bounded by 1.
    if node.head == _SIN and len(node.args) == 1:
        return True
    if node.head == _COS and len(node.args) == 1:
        return True
    # Closure under Mul / Add / Neg.
    if node.head == MUL:
        return all(_is_bounded_in_k(a, k) for a in node.args)
    if node.head == ADD:
        return all(_is_bounded_in_k(a, k) for a in node.args)
    if node.head == NEG and len(node.args) == 1:
        return _is_bounded_in_k(node.args[0], k)
    return False


def _g_vanishes_at_infinity(g: IRNode, k: IRSymbol) -> bool:
    """Return True when ``g(k)`` provably tends to 0 as ``k → ∞``.

    Two-tier recognition:

    1.  **Phase 41 fast path** — ``Div(c, h(k))`` with ``c`` constant
        in ``k`` and ``h(k)`` recognised as a positive-degree polynomial
        in ``k`` (via :func:`_is_positive_degree_polynomial_in_k`).
        This handles every shape Apart can emit from a rational summand
        whose denominator factors over ℚ into simple linear factors.

    2.  **Phase 42 widening** — ``Div(P(k), Q(k))`` where both ``P`` and
        ``Q`` are polynomials in ``k`` (via
        :func:`_polynomial_degree_in_k`) and ``deg(P) < deg(Q)``.  This
        closes telescopes like ``k/((k+1)(k+2)(k+3))`` and any other
        proper rational shape Apart might emit.

    +-----------------------------------+-----------------------------+
    | ``g`` shape                       | Provably ``→ 0``?           |
    +===================================+=============================+
    | ``Div(constant, k+a)``            | yes (Phase 41)              |
    | ``Div(constant, k²·(k+1))``       | yes (Phase 41)              |
    | ``Div(k, k²+1)``                  | yes (Phase 42)              |
    | ``Div(k+1, k³−5)``                | yes (Phase 42)              |
    | ``Div(sin(k), k²)``               | yes (Phase 49)              |
    | ``Div(log(k), k²)``               | yes (Phase 50)              |
    | ``Div(sin(k)·log(k), k²)``        | yes (Phase 55)              |
    | constant                          | no (limit is the const)     |
    | ``k`` or ``k²+1``                 | no (limit is ∞)             |
    | ``1/sin(k)`` / ``log(k)/k``       | no (numerator is not a      |
    |                                   |    polynomial; transcend-   |
    |                                   |    ental limits deferred)   |
    +-----------------------------------+-----------------------------+

    Examples
    --------
    - ``1/k`` → True (Phase 41: constant/k).
    - ``k/(k²+1)`` → True (Phase 42: deg 1 < deg 2).
    - ``(k²+1)/k³`` → True (Phase 42: deg 2 < deg 3).
    - ``k/(k+1)`` → False (deg 1 = deg 1; limit is 1, not 0).
    - ``k²/(k+1)`` → False (improper; limit is ∞).
    """
    if not isinstance(g, IRApply) or g.head != DIV or len(g.args) != 2:
        return False
    num, den = g.args
    # Phase 41/43 fast path: constant numerator + diverging denominator
    # (positive-degree polynomial OR exp / b^k transcendental).
    if _is_constant_in(num, k):
        return _h_diverges_at_infinity(den, k)
    # Phase 49: bounded numerator + diverging denominator.  Covers
    # shapes like ``sin(k)/k²`` and ``cos(k)·sin(k)/k³`` where the
    # numerator is uniformly bounded (``|sin|, |cos| ≤ 1`` and
    # closures under Mul/Add/Neg).
    if _is_bounded_in_k(num, k) and _h_diverges_at_infinity(den, k):
        return True
    # Phase 50: Log(diverging) numerator + diverging denominator.
    # log(h(k)) → +∞ at logarithmic rate, while any positive-degree
    # polynomial / exp / b^k denominator grows strictly faster, so the
    # quotient vanishes.  Reuses ``_h_diverges_at_infinity`` for both
    # the numerator argument and the denominator — same sign-aware
    # divergence check used elsewhere.
    if _is_log_of_diverging_in_k(num, k) and _h_diverges_at_infinity(den, k):
        return True
    # Phase 51: ``Sqrt(P(k))`` numerator pattern.  ``sqrt(P)`` has
    # effective growth rate ``deg(P)/2``.  The quotient
    # ``Sqrt(P)/Q`` vanishes when ``deg(Q) > deg(P)/2``, i.e. when
    # ``2*deg(Q) > deg(P)``.  We use ×2 integer arithmetic throughout
    # to avoid fractions.  ``_sqrt_effective_half_degree_x2`` returns
    # ``deg(P)`` and requires the leading coefficient of ``P`` to be
    # positive (so ``Sqrt(P)`` is real-valued and diverging).
    sqrt_inner_deg = _sqrt_effective_half_degree_x2(num, k)
    if sqrt_inner_deg is not None:
        den_deg_sq = _polynomial_degree_in_k(den, k)
        if den_deg_sq is not None and 2 * den_deg_sq > sqrt_inner_deg:
            return True
    # Phase 52: ``Mul(bounded, polynomial)`` numerator pattern.  When the
    # numerator factorises as ``bounded × P(k)`` (with ``bounded`` non-
    # constant — so Phase 49 missed it — and ``P`` a positive-degree
    # polynomial in ``k``), the effective growth of the numerator is
    # ``deg(P)``.  Vanishes when ``deg(den) > deg(P)``.
    bounded_poly = _split_bounded_polynomial_factor(num, k)
    if bounded_poly is not None:
        _, poly_deg = bounded_poly
        den_deg_bp = _polynomial_degree_in_k(den, k)
        if den_deg_bp is not None and den_deg_bp > poly_deg:
            return True
    # Phase 53: ``Mul(Sqrt(P), polynomial_factors)`` numerator pattern.
    # The effective growth rate is ``deg(P)/2 + deg(Q)``.  Using ×2
    # integer arithmetic: vanishes when ``2*deg(den) > deg(P) + 2*deg(Q)``.
    # ``_sqrt_poly_numerator_effective_degree_x2`` returns
    # ``deg(P) + 2*deg(Q)`` and requires exactly one Sqrt factor with a
    # positive-leading-coefficient polynomial inner.
    sqrt_poly_eff = _sqrt_poly_numerator_effective_degree_x2(num, k)
    if sqrt_poly_eff is not None:
        den_deg_sp = _polynomial_degree_in_k(den, k)
        if den_deg_sp is not None and 2 * den_deg_sp > sqrt_poly_eff:
            return True
    # Phase 54: ``Mul(Log(diverging), polynomial_factors)`` numerator
    # pattern.  ``log(h(k))`` grows sub-polynomially — slower than any
    # positive power of ``k`` — so the effective growth degree of
    # ``log(h) · P(k)`` equals ``deg(P)`` (the log factor is negligible
    # for degree comparisons).  The quotient vanishes when
    # ``deg(den) > deg(P)`` (strictly).
    #
    # Note: When ``deg(den) == deg(P)`` the expression reduces to
    # ``log(h(k)) * constant``, which diverges to ±∞, so equality is
    # correctly refused.
    log_poly = _split_log_polynomial_factor(num, k)
    if log_poly is not None:
        _, poly_deg_lp = log_poly
        den_deg_lp = _polynomial_degree_in_k(den, k)
        if den_deg_lp is not None and den_deg_lp > poly_deg_lp:
            return True
    # Phase 55: ``Mul(bounded, Log(diverging))`` numerator + diverging
    # denominator.  The numerator is the product of a uniformly bounded
    # function (``|f| ≤ C``) and a logarithm that grows sub-polynomially.
    # Because ``log(h(k)) = o(k^ε)`` for any ``ε > 0``, the whole
    # numerator grows sub-polynomially — dominated by any polynomial
    # (or faster-growing) denominator.  This is the bounded-times-log
    # complement to Phase 52 (bounded × polynomial) and Phase 54
    # (log × polynomial).
    #
    # Note: Unlike Phase 54, the comparison here is against the denominator's
    # divergence (not a strict degree inequality), because the numerator's
    # effective polynomial degree is 0 — it doesn't grow polynomially at all.
    # Any strictly diverging denominator therefore dominates.
    if _is_bounded_times_log_in_k(num, k) and _h_diverges_at_infinity(den, k):
        return True
    # Phase 56: ``Mul(bounded, Sqrt(diverging))`` numerator pattern.
    # The bounded part is uniformly bounded; ``Sqrt(P(k))`` grows like
    # ``k^{deg(P)/2}``.  The whole numerator therefore has effective
    # growth ``k^{deg(P)/2}``, and the quotient vanishes when the
    # denominator grows strictly faster.  Stays exact via the ×2 integer
    # trick already used in Phase 51 / 53: returns
    # ``sqrt_inner_deg = deg(P)`` (= 2 × half-degree); compare to
    # ``2 × den_poly_degree``.  When the denominator is non-polynomial
    # but diverging (Exp / Pow / Log×poly / Mul of these), it dominates
    # any sub-polynomial sqrt growth automatically.
    sqrt_inner_deg = _bounded_times_sqrt_inner_deg(num, k)
    if sqrt_inner_deg is not None:
        den_deg_bs = _polynomial_degree_in_k(den, k)
        if den_deg_bs is not None:
            if 2 * den_deg_bs > sqrt_inner_deg:
                return True
        elif _h_diverges_at_infinity(den, k):
            # Non-polynomial diverging denominator dominates ``k^{m/2}``
            # for any ``m`` since ``Exp / Pow(b>1, …)`` grows faster
            # than any polynomial, hence faster than any half-degree.
            return True
    # Phase 57: ``Mul(bounded..., Log(diverging), Sqrt(positive-poly))``
    # numerator pattern.  Combines sub-polynomial Log growth with half-
    # polynomial Sqrt growth.  Effective ``log(k) · k^{deg(P)/2}`` is
    # strictly dominated by ``k^{deg(P)/2 + ε}`` for any ``ε > 0``.
    # Vanishes when ``2·den_deg > deg(P)`` (polynomial) or non-polynomial
    # diverging denominator.  Requires both Log and Sqrt — one-only
    # patterns fall through to Phase 55 / Phase 56.
    bls_sqrt_deg = _bounded_log_sqrt_inner_deg(num, k)
    if bls_sqrt_deg is not None:
        den_deg_bls = _polynomial_degree_in_k(den, k)
        if den_deg_bls is not None:
            if 2 * den_deg_bls > bls_sqrt_deg:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 58: ``Mul(bounded, Log(diverging), polynomial)`` numerator.
    # Bounded × Log × polynomial: effective growth ``log(k)·k^m`` which is
    # ``o(k^{m+ε})`` for any ``ε > 0``.  Vanishes when the denominator
    # grows strictly faster than ``k^m``:
    #   - polynomial denominator with ``den_deg > poly_deg``, OR
    #   - non-polynomial diverging denominator (Exp / Pow / Log×poly).
    # This closes the gap left by Phase 54 (Log × polynomial, refuses
    # bounded factors) and Phase 55 (bounded × Log, refuses polynomial
    # factors).  Sqrt factors are intentionally refused and handled by
    # Phase 57.
    blp_deg = _bounded_log_poly_degree(num, k)
    if blp_deg is not None:
        den_deg_blp = _polynomial_degree_in_k(den, k)
        if den_deg_blp is not None:
            if den_deg_blp > blp_deg:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 59: ``Mul(bounded, Sqrt(positive-poly), polynomial)`` numerator.
    # Bounded × Sqrt × polynomial: effective growth ``k^{deg(P)/2 + poly_deg}``.
    # Using ×2 trick: effective_x2 = deg(P) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial denominator) or
    # when the denominator is non-polynomial diverging.
    # Closes the gap between Phase 53 (Sqrt × poly, refuses bounded) and
    # Phase 56 (bounded × Sqrt, refuses polynomial factors).
    # Log factors are intentionally refused — use Phase 57 (bounded × Log × Sqrt).
    bsp_x2 = _bounded_sqrt_poly_effective_x2(num, k)
    if bsp_x2 is not None:
        den_deg_bsp = _polynomial_degree_in_k(den, k)
        if den_deg_bsp is not None:
            if 2 * den_deg_bsp > bsp_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 60: ``Mul(bounded..., Log(diverging), Sqrt(positive-poly), polynomial)``
    # numerator pattern.  Extends Phase 57 (bounded × Log × Sqrt, refuses poly) by
    # allowing polynomial factors alongside the Log and Sqrt.
    # Effective growth: ``log(k) · k^{deg(P)/2 + poly_deg}``
    # = ``o(k^{deg(P)/2 + poly_deg + ε})``.
    # ×2 trick: ``effective_x2 = sqrt_inner_deg + 2·poly_deg``.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging.
    blsp_x2 = _bounded_log_sqrt_poly_effective_x2(num, k)
    if blsp_x2 is not None:
        den_deg_blsp = _polynomial_degree_in_k(den, k)
        if den_deg_blsp is not None:
            if 2 * den_deg_blsp > blsp_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 61: ``Mul(Sqrt(P1), Sqrt(P2), polynomial..., bounded...)``
    # numerator.  Extends Phases 53/56/59 to the case of two Sqrt factors.
    # effective_x2 = deg(P1) + deg(P2) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    tsp_x2 = _two_sqrt_poly_effective_x2(num, k)
    if tsp_x2 is not None:
        den_deg_tsp = _polynomial_degree_in_k(den, k)
        if den_deg_tsp is not None:
            if 2 * den_deg_tsp > tsp_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 62: ``Mul(Log(diverging), Log(diverging), polynomial..., bounded...)``
    # numerator.  Extends Phase 50 (single Log) to the case of two Log factors.
    # log²(k) is still sub-polynomial: log²(k)·k^m = o(k^{m+ε}) for any ε>0.
    # effective_x2 = 2·poly_deg (log² contributes nothing to effective degree).
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    tlp_x2 = _two_log_poly_effective_x2(num, k)
    if tlp_x2 is not None:
        den_deg_tlp = _polynomial_degree_in_k(den, k)
        if den_deg_tlp is not None:
            if 2 * den_deg_tlp > tlp_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 63: ``Mul(Sqrt(P1), Sqrt(P2), Log(diverging), polynomial..., bounded...)``
    # numerator.  Two Sqrt factors plus one Log; log is sub-polynomial and
    # does not change the effective degree.
    # effective_x2 = deg(P1) + deg(P2) + 2·poly_deg (same as Phase 61).
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    tslp_x2 = _two_sqrt_log_poly_effective_x2(num, k)
    if tslp_x2 is not None:
        den_deg_tslp = _polynomial_degree_in_k(den, k)
        if den_deg_tslp is not None:
            if 2 * den_deg_tslp > tslp_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 64: ``Mul(Log(diverging), Log(diverging), Sqrt(positive-poly),
    #           polynomial..., bounded...)`` numerator.
    # Two Log factors plus one Sqrt; log² is sub-polynomial and does not
    # change the effective degree.
    # effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    tlsp_x2 = _two_log_sqrt_poly_effective_x2(num, k)
    if tlsp_x2 is not None:
        den_deg_tlsp = _polynomial_degree_in_k(den, k)
        if den_deg_tlsp is not None:
            if 2 * den_deg_tlsp > tlsp_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 65: ``Mul(Sqrt(P1), Sqrt(P2), Log(diverging), Log(diverging),
    #           polynomial..., bounded...)`` numerator.
    # Two Sqrt + two Log; log² sub-polynomial; effective_x2 = deg(P1) + deg(P2) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    ts2l_x2 = _two_sqrt_two_log_poly_effective_x2(num, k)
    if ts2l_x2 is not None:
        den_deg_ts2l = _polynomial_degree_in_k(den, k)
        if den_deg_ts2l is not None:
            if 2 * den_deg_ts2l > ts2l_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 66: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), polynomial..., bounded...)``
    # numerator.  Three Sqrt factors; log factors intentionally refused.
    # effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    tsp_x2 = _three_sqrt_poly_effective_x2(num, k)
    if tsp_x2 is not None:
        den_deg_tsp = _polynomial_degree_in_k(den, k)
        if den_deg_tsp is not None:
            if 2 * den_deg_tsp > tsp_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 67: ``Mul(Log(diverging), Log(diverging), Log(diverging),
    #           polynomial..., bounded...)`` numerator.
    # Three Log factors; log³ is sub-polynomial: log³(k)·k^m = o(k^{m+ε}).
    # effective_x2 = 2·poly_deg (log³ contributes nothing to effective degree).
    # Sqrt factors are refused — use Phase 63/64/65 for sqrt+log combos.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    tlp3_x2 = _three_log_poly_effective_x2(num, k)
    if tlp3_x2 is not None:
        den_deg_tlp3 = _polynomial_degree_in_k(den, k)
        if den_deg_tlp3 is not None:
            if 2 * den_deg_tlp3 > tlp3_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 68: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(diverging),
    #           polynomial..., bounded...)`` numerator.
    # Three Sqrt factors + one Log factor; log is sub-polynomial → contributes 0.
    # effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    ts3lp_x2 = _three_sqrt_log_poly_effective_x2(num, k)
    if ts3lp_x2 is not None:
        den_deg_ts3lp = _polynomial_degree_in_k(den, k)
        if den_deg_ts3lp is not None:
            if 2 * den_deg_ts3lp > ts3lp_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 69: ``Mul(Sqrt(P), Log(diverging), Log(diverging), Log(diverging),
    #           polynomial..., bounded...)`` numerator.
    # One Sqrt factor + three Log factors; log³ is sub-polynomial → contributes 0.
    # effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s1l3p_x2 = _one_sqrt_three_log_poly_effective_x2(num, k)
    if s1l3p_x2 is not None:
        den_deg_s1l3p = _polynomial_degree_in_k(den, k)
        if den_deg_s1l3p is not None:
            if 2 * den_deg_s1l3p > s1l3p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 70: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1), Log(h2),
    #           polynomial..., bounded...)`` numerator.
    # Three Sqrt + two Log factors; log² sub-polynomial → contributes 0.
    # effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    ts3l2p_x2 = _three_sqrt_two_log_poly_effective_x2(num, k)
    if ts3l2p_x2 is not None:
        den_deg_ts3l2p = _polynomial_degree_in_k(den, k)
        if den_deg_ts3l2p is not None:
            if 2 * den_deg_ts3l2p > ts3l2p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 71: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1), Log(h2), Log(h3),
    #           polynomial..., bounded...)`` numerator.
    # Two Sqrt + three Log factors; log³ sub-polynomial → contributes 0.
    # effective_x2 = deg(P1) + deg(P2) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    ts2l3p_x2 = _two_sqrt_three_log_poly_effective_x2(num, k)
    if ts2l3p_x2 is not None:
        den_deg_ts2l3p = _polynomial_degree_in_k(den, k)
        if den_deg_ts2l3p is not None:
            if 2 * den_deg_ts2l3p > ts2l3p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 72: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1), Log(h2), Log(h3),
    #           polynomial..., bounded...)`` numerator.
    # Three Sqrt + three Log factors; log³ sub-polynomial → contributes 0.
    # effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    ts3l3p_x2 = _three_sqrt_three_log_poly_effective_x2(num, k)
    if ts3l3p_x2 is not None:
        den_deg_ts3l3p = _polynomial_degree_in_k(den, k)
        if den_deg_ts3l3p is not None:
            if 2 * den_deg_ts3l3p > ts3l3p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 73: ``Mul(Log(h1), Log(h2), Log(h3), Log(h4),
    #           polynomial..., bounded...)`` numerator.
    # Four Log factors; log⁴ sub-polynomial → contributes 0.
    # effective_x2 = 2·poly_deg (no Sqrt factors).
    # Sqrt factors refused — use Sqrt × log phases for mixed forms.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    flp4_x2 = _four_log_poly_effective_x2(num, k)
    if flp4_x2 is not None:
        den_deg_flp4 = _polynomial_degree_in_k(den, k)
        if den_deg_flp4 is not None:
            if 2 * den_deg_flp4 > flp4_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 74: ``Mul(Sqrt(P), Log(h1), Log(h2), Log(h3), Log(h4),
    #           polynomial..., bounded...)`` numerator.
    # One Sqrt factor + four Log factors; log⁴ sub-polynomial → 0.
    # effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s1l4p_x2 = _one_sqrt_four_log_poly_effective_x2(num, k)
    if s1l4p_x2 is not None:
        den_deg_s1l4 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l4 is not None:
            if 2 * den_deg_s1l4 > s1l4p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 75: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1), Log(h2), Log(h3), Log(h4),
    #           polynomial..., bounded...)`` numerator.
    # Two Sqrt factors + four Log factors; log⁴ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s2l4p_x2 = _two_sqrt_four_log_poly_effective_x2(num, k)
    if s2l4p_x2 is not None:
        den_deg_s2l4 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l4 is not None:
            if 2 * den_deg_s2l4 > s2l4p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 76: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1), Log(h2), Log(h3), Log(h4),
    #           polynomial..., bounded...)`` numerator.
    # Three Sqrt factors + four Log factors; log⁴ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s3l4p_x2 = _three_sqrt_four_log_poly_effective_x2(num, k)
    if s3l4p_x2 is not None:
        den_deg_s3l4 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l4 is not None:
            if 2 * den_deg_s3l4 > s3l4p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 77: ``Mul(Log(h1), Log(h2), Log(h3), Log(h4), Log(h5),
    #           polynomial..., bounded...)`` numerator.
    # Five Log factors; no Sqrt; log⁵ sub-polynomial → 0.
    # effective_x2 = 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    flp5_x2 = _five_log_poly_effective_x2(num, k)
    if flp5_x2 is not None:
        den_deg_fl5 = _polynomial_degree_in_k(den, k)
        if den_deg_fl5 is not None:
            if 2 * den_deg_fl5 > flp5_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 78: ``Mul(Sqrt(P), Log(h1), Log(h2), Log(h3), Log(h4), Log(h5),
    #           polynomial..., bounded...)`` numerator.
    # One Sqrt factor + five Log factors; log⁵ sub-polynomial → 0.
    # effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s1l5p_x2 = _one_sqrt_five_log_poly_effective_x2(num, k)
    if s1l5p_x2 is not None:
        den_deg_s1l5 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l5 is not None:
            if 2 * den_deg_s1l5 > s1l5p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 79: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1), Log(h2), Log(h3), Log(h4), Log(h5),
    #           polynomial..., bounded...)`` numerator.
    # Two Sqrt factors + five Log factors; log⁵ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s2l5p_x2 = _two_sqrt_five_log_poly_effective_x2(num, k)
    if s2l5p_x2 is not None:
        den_deg_s2l5 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l5 is not None:
            if 2 * den_deg_s2l5 > s2l5p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 80: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1), Log(h2), Log(h3), Log(h4), Log(h5),
    #           polynomial..., bounded...)`` numerator.
    # Three Sqrt factors + five Log factors; log⁵ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s3l5p_x2 = _three_sqrt_five_log_poly_effective_x2(num, k)
    if s3l5p_x2 is not None:
        den_deg_s3l5 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l5 is not None:
            if 2 * den_deg_s3l5 > s3l5p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 81: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Log(h1), Log(h2), Log(h3),
    #           Log(h4), Log(h5), polynomial..., bounded...)`` numerator.
    # Four Sqrt factors + five Log factors; log⁵ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s4l5p_x2 = _four_sqrt_five_log_poly_effective_x2(num, k)
    if s4l5p_x2 is not None:
        den_deg_s4l5 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l5 is not None:
            if 2 * den_deg_s4l5 > s4l5p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 82: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Sqrt(P4), Sqrt(P5), Log(h1), Log(h2),
    #           Log(h3), Log(h4), Log(h5), polynomial..., bounded...)`` numerator.
    # Five Sqrt factors + five Log factors; log⁵ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + sqrt5_deg_x2
    #              + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s5l5p_x2 = _five_sqrt_five_log_poly_effective_x2(num, k)
    if s5l5p_x2 is not None:
        den_deg_s5l5 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l5 is not None:
            if 2 * den_deg_s5l5 > s5l5p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 83: ``Mul(Log(h1), Log(h2), Log(h3), Log(h4), Log(h5), Log(h6),
    #           polynomial..., bounded...)`` numerator.
    # Zero Sqrt factors + six Log factors; log⁶ sub-polynomial → 0.
    # effective_x2 = 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    sl6p_x2 = _six_log_poly_effective_x2(num, k)
    if sl6p_x2 is not None:
        den_deg_sl6 = _polynomial_degree_in_k(den, k)
        if den_deg_sl6 is not None:
            if 2 * den_deg_sl6 > sl6p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 84: ``Mul(Sqrt(P), Log(h1), Log(h2), Log(h3), Log(h4), Log(h5), Log(h6),
    #           polynomial..., bounded...)`` numerator.
    # One Sqrt factor + six Log factors; log⁶ sub-polynomial → 0.
    # effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s1l6p_x2 = _one_sqrt_six_log_poly_effective_x2(num, k)
    if s1l6p_x2 is not None:
        den_deg_s1l6 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l6 is not None:
            if 2 * den_deg_s1l6 > s1l6p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 85: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1), Log(h2), Log(h3), Log(h4), Log(h5), Log(h6),
    #           polynomial..., bounded...)`` numerator.
    # Two Sqrt factors + six Log factors; log⁶ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s2l6p_x2 = _two_sqrt_six_log_poly_effective_x2(num, k)
    if s2l6p_x2 is not None:
        den_deg_s2l6 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l6 is not None:
            if 2 * den_deg_s2l6 > s2l6p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 89: ``Mul(Log(h1), ..., Log(h7), polynomial..., bounded...)`` numerator.
    # Zero Sqrt factors + seven Log factors; log⁷ sub-polynomial → 0.
    # effective_x2 = 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    sl7p_x2 = _seven_log_poly_effective_x2(num, k)
    if sl7p_x2 is not None:
        den_deg_sl7 = _polynomial_degree_in_k(den, k)
        if den_deg_sl7 is not None:
            if 2 * den_deg_sl7 > sl7p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 90: ``Mul(Sqrt(P), Log(h1), ..., Log(h7), polynomial..., bounded...)`` numerator.
    # One Sqrt factor + seven Log factors; log⁷ sub-polynomial → 0.
    # effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s1l7p_x2 = _one_sqrt_seven_log_poly_effective_x2(num, k)
    if s1l7p_x2 is not None:
        den_deg_s1l7 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l7 is not None:
            if 2 * den_deg_s1l7 > s1l7p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 91: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1), ..., Log(h7), polynomial..., bounded...)`` numerator.
    # Two Sqrt factors + seven Log factors; log⁷ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s2l7p_x2 = _two_sqrt_seven_log_poly_effective_x2(num, k)
    if s2l7p_x2 is not None:
        den_deg_s2l7 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l7 is not None:
            if 2 * den_deg_s2l7 > s2l7p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 92: ``Mul(Sqrt(P1), Sqrt(P2), Sqrt(P3), Log(h1), ..., Log(h7), polynomial..., bounded...)`` numerator.
    # Three Sqrt factors + seven Log factors; log⁷ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s3l7p_x2 = _three_sqrt_seven_log_poly_effective_x2(num, k)
    if s3l7p_x2 is not None:
        den_deg_s3l7 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l7 is not None:
            if 2 * den_deg_s3l7 > s3l7p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 93: ``Mul(Sqrt(P1)..Sqrt(P4), Log(h1), ..., Log(h7), polynomial..., bounded...)`` numerator.
    # Four Sqrt factors + seven Log factors; log⁷ sub-polynomial → 0.
    # effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s4l7p_x2 = _four_sqrt_seven_log_poly_effective_x2(num, k)
    if s4l7p_x2 is not None:
        den_deg_s4l7 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l7 is not None:
            if 2 * den_deg_s4l7 > s4l7p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 94: ``Mul(Sqrt(P1)..Sqrt(P5), Log(h1), ..., Log(h7), polynomial..., bounded...)`` numerator.
    # Five Sqrt factors + seven Log factors; log⁷ sub-polynomial → 0.
    # effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s5l7p_x2 = _five_sqrt_seven_log_poly_effective_x2(num, k)
    if s5l7p_x2 is not None:
        den_deg_s5l7 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l7 is not None:
            if 2 * den_deg_s5l7 > s5l7p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 95: ``Mul(Log(h1), ..., Log(h8), polynomial..., bounded...)`` numerator.
    # Zero Sqrt factors + eight Log factors; log⁸ sub-polynomial → 0.
    # effective_x2 = 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    sl8p_x2 = _eight_log_poly_effective_x2(num, k)
    if sl8p_x2 is not None:
        den_deg_sl8 = _polynomial_degree_in_k(den, k)
        if den_deg_sl8 is not None:
            if 2 * den_deg_sl8 > sl8p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 96: ``Mul(Sqrt(P), Log(h1), ..., Log(h8), polynomial..., bounded...)`` numerator.
    # One Sqrt factor + eight Log factors; log⁸ sub-polynomial → 0.
    # effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s1l8p_x2 = _one_sqrt_eight_log_poly_effective_x2(num, k)
    if s1l8p_x2 is not None:
        den_deg_s1l8 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l8 is not None:
            if 2 * den_deg_s1l8 > s1l8p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 97: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1), ..., Log(h8), polynomial..., bounded...)`` numerator.
    # Two Sqrt factors + eight Log factors; log⁸ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s2l8p_x2 = _two_sqrt_eight_log_poly_effective_x2(num, k)
    if s2l8p_x2 is not None:
        den_deg_s2l8 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l8 is not None:
            if 2 * den_deg_s2l8 > s2l8p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 98: ``Mul(Sqrt(P1)..Sqrt(P3), Log(h1), ..., Log(h8), polynomial..., bounded...)`` numerator.
    # Three Sqrt factors + eight Log factors; log⁸ sub-polynomial → 0.
    # effective_x2 = sqrt1+sqrt2+sqrt3_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s3l8p_x2 = _three_sqrt_eight_log_poly_effective_x2(num, k)
    if s3l8p_x2 is not None:
        den_deg_s3l8 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l8 is not None:
            if 2 * den_deg_s3l8 > s3l8p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 99: ``Mul(Sqrt(P1)..Sqrt(P4), Log(h1), ..., Log(h8), polynomial..., bounded...)`` numerator.
    # Four Sqrt factors + eight Log factors; log⁸ sub-polynomial → 0.
    # effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s4l8p_x2 = _four_sqrt_eight_log_poly_effective_x2(num, k)
    if s4l8p_x2 is not None:
        den_deg_s4l8 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l8 is not None:
            if 2 * den_deg_s4l8 > s4l8p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 100: ``Mul(Sqrt(P1)..Sqrt(P5), Log(h1), ..., Log(h8), polynomial..., bounded...)`` numerator.
    # Five Sqrt factors + eight Log factors; log⁸ sub-polynomial → 0.
    # effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s5l8p_x2 = _five_sqrt_eight_log_poly_effective_x2(num, k)
    if s5l8p_x2 is not None:
        den_deg_s5l8 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l8 is not None:
            if 2 * den_deg_s5l8 > s5l8p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 101: ``Mul(Log(h1), ..., Log(h9), polynomial..., bounded...)`` numerator.
    # Zero Sqrt factors + nine Log factors; log⁹ sub-polynomial → 0.
    # effective_x2 = 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    sl9p_x2 = _nine_log_poly_effective_x2(num, k)
    if sl9p_x2 is not None:
        den_deg_sl9 = _polynomial_degree_in_k(den, k)
        if den_deg_sl9 is not None:
            if 2 * den_deg_sl9 > sl9p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 102: ``Mul(Sqrt(P), Log(h1), ..., Log(h9), polynomial..., bounded...)`` numerator.
    # One Sqrt factor + nine Log factors; log⁹ sub-polynomial → 0.
    # effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s1l9p_x2 = _one_sqrt_nine_log_poly_effective_x2(num, k)
    if s1l9p_x2 is not None:
        den_deg_s1l9 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l9 is not None:
            if 2 * den_deg_s1l9 > s1l9p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 103: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1), ..., Log(h9), polynomial..., bounded...)`` numerator.
    # Two Sqrt factors + nine Log factors; log⁹ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s2l9p_x2 = _two_sqrt_nine_log_poly_effective_x2(num, k)
    if s2l9p_x2 is not None:
        den_deg_s2l9 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l9 is not None:
            if 2 * den_deg_s2l9 > s2l9p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 104: Three Sqrt + nine Log factors; log⁹ sub-polynomial → 0.
    # effective_x2 = sqrt1_x2 + sqrt2_x2 + sqrt3_x2 + 2·poly_deg.
    s3l9p_x2 = _three_sqrt_nine_log_poly_effective_x2(num, k)
    if s3l9p_x2 is not None:
        den_deg_s3l9 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l9 is not None:
            if 2 * den_deg_s3l9 > s3l9p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 105: Four Sqrt + nine Log factors; log⁹ sub-polynomial → 0.
    # effective_x2 = sqrt1_x2 + sqrt2_x2 + sqrt3_x2 + sqrt4_x2 + 2·poly_deg.
    s4l9p_x2 = _four_sqrt_nine_log_poly_effective_x2(num, k)
    if s4l9p_x2 is not None:
        den_deg_s4l9 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l9 is not None:
            if 2 * den_deg_s4l9 > s4l9p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 106: Five Sqrt + nine Log factors; log⁹ sub-polynomial → 0.
    # effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5_x2 + 2·poly_deg.
    s5l9p_x2 = _five_sqrt_nine_log_poly_effective_x2(num, k)
    if s5l9p_x2 is not None:
        den_deg_s5l9 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l9 is not None:
            if 2 * den_deg_s5l9 > s5l9p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 112: ``Mul(Sqrt(P1)×5, Log(h1)×10, polynomial..., bounded...)`` numerator.
    # Five Sqrt + ten Log factors; log^10 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l10p_x2 = _five_sqrt_ten_log_poly_effective_x2(num, k)
    if s5l10p_x2 is not None:
        den_deg_s5l10 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l10 is not None:
            if 2 * den_deg_s5l10 > s5l10p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 111: ``Mul(Sqrt(P1)×4, Log(h1)×10, polynomial..., bounded...)`` numerator.
    s4l10p_x2 = _four_sqrt_ten_log_poly_effective_x2(num, k)
    if s4l10p_x2 is not None:
        den_deg_s4l10 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l10 is not None:
            if 2 * den_deg_s4l10 > s4l10p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 110: ``Mul(Sqrt(P1)×3, Log(h1)×10, polynomial..., bounded...)`` numerator.
    s3l10p_x2 = _three_sqrt_ten_log_poly_effective_x2(num, k)
    if s3l10p_x2 is not None:
        den_deg_s3l10 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l10 is not None:
            if 2 * den_deg_s3l10 > s3l10p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 109: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×10, polynomial..., bounded...)`` numerator.
    s2l10p_x2 = _two_sqrt_ten_log_poly_effective_x2(num, k)
    if s2l10p_x2 is not None:
        den_deg_s2l10 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l10 is not None:
            if 2 * den_deg_s2l10 > s2l10p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 108: ``Mul(Sqrt(P), Log(h1)×10, polynomial..., bounded...)`` numerator.
    s1l10p_x2 = _one_sqrt_ten_log_poly_effective_x2(num, k)
    if s1l10p_x2 is not None:
        den_deg_s1l10 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l10 is not None:
            if 2 * den_deg_s1l10 > s1l10p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 107: ``Mul(Log(h1)×10, polynomial..., bounded...)`` numerator.
    sl10p_x2 = _ten_log_poly_effective_x2(num, k)
    if sl10p_x2 is not None:
        den_deg_sl10 = _polynomial_degree_in_k(den, k)
        if den_deg_sl10 is not None:
            if 2 * den_deg_sl10 > sl10p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 424: ``Mul(Sqrt(P1)×5, Log(h1)×62, polynomial..., bounded...)`` numerator.
    s5l62p_x2 = _five_sqrt_sixty_two_log_poly_effective_x2(num, k)
    if s5l62p_x2 is not None:
        den_deg_s5l62 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l62 is not None:
            if 2 * den_deg_s5l62 > s5l62p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 423: ``Mul(Sqrt(P1)×4, Log(h1)×62, polynomial..., bounded...)`` numerator.
    s4l62p_x2 = _four_sqrt_sixty_two_log_poly_effective_x2(num, k)
    if s4l62p_x2 is not None:
        den_deg_s4l62 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l62 is not None:
            if 2 * den_deg_s4l62 > s4l62p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 422: ``Mul(Sqrt(P1)×3, Log(h1)×62, polynomial..., bounded...)`` numerator.
    s3l62p_x2 = _three_sqrt_sixty_two_log_poly_effective_x2(num, k)
    if s3l62p_x2 is not None:
        den_deg_s3l62 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l62 is not None:
            if 2 * den_deg_s3l62 > s3l62p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 421: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×62, polynomial..., bounded...)`` numerator.
    s2l62p_x2 = _two_sqrt_sixty_two_log_poly_effective_x2(num, k)
    if s2l62p_x2 is not None:
        den_deg_s2l62 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l62 is not None:
            if 2 * den_deg_s2l62 > s2l62p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 420: ``Mul(Sqrt(P), Log(h1)×62, polynomial..., bounded...)`` numerator.
    s1l62p_x2 = _one_sqrt_sixty_two_log_poly_effective_x2(num, k)
    if s1l62p_x2 is not None:
        den_deg_s1l62 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l62 is not None:
            if 2 * den_deg_s1l62 > s1l62p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 419: ``Mul(Log(h1)×62, polynomial..., bounded...)`` numerator.
    sl62p_x2 = _sixty_two_log_poly_effective_x2(num, k)
    if sl62p_x2 is not None:
        den_deg_sl62 = _polynomial_degree_in_k(den, k)
        if den_deg_sl62 is not None:
            if 2 * den_deg_sl62 > sl62p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 418: ``Mul(Sqrt(P1)×5, Log(h1)×61, polynomial..., bounded...)`` numerator.
    s5l61p_x2 = _five_sqrt_sixty_one_log_poly_effective_x2(num, k)
    if s5l61p_x2 is not None:
        den_deg_s5l61 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l61 is not None:
            if 2 * den_deg_s5l61 > s5l61p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 417: ``Mul(Sqrt(P1)×4, Log(h1)×61, polynomial..., bounded...)`` numerator.
    s4l61p_x2 = _four_sqrt_sixty_one_log_poly_effective_x2(num, k)
    if s4l61p_x2 is not None:
        den_deg_s4l61 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l61 is not None:
            if 2 * den_deg_s4l61 > s4l61p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 416: ``Mul(Sqrt(P1)×3, Log(h1)×61, polynomial..., bounded...)`` numerator.
    s3l61p_x2 = _three_sqrt_sixty_one_log_poly_effective_x2(num, k)
    if s3l61p_x2 is not None:
        den_deg_s3l61 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l61 is not None:
            if 2 * den_deg_s3l61 > s3l61p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 415: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×61, polynomial..., bounded...)`` numerator.
    s2l61p_x2 = _two_sqrt_sixty_one_log_poly_effective_x2(num, k)
    if s2l61p_x2 is not None:
        den_deg_s2l61 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l61 is not None:
            if 2 * den_deg_s2l61 > s2l61p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 414: ``Mul(Sqrt(P), Log(h1)×61, polynomial..., bounded...)`` numerator.
    s1l61p_x2 = _one_sqrt_sixty_one_log_poly_effective_x2(num, k)
    if s1l61p_x2 is not None:
        den_deg_s1l61 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l61 is not None:
            if 2 * den_deg_s1l61 > s1l61p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 413: ``Mul(Log(h1)×61, polynomial..., bounded...)`` numerator.
    sl61p_x2 = _sixty_one_log_poly_effective_x2(num, k)
    if sl61p_x2 is not None:
        den_deg_sl61 = _polynomial_degree_in_k(den, k)
        if den_deg_sl61 is not None:
            if 2 * den_deg_sl61 > sl61p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 412: ``Mul(Sqrt(P1)×5, Log(h1)×60, polynomial..., bounded...)`` numerator.
    s5l60p_x2 = _five_sqrt_sixty_log_poly_effective_x2(num, k)
    if s5l60p_x2 is not None:
        den_deg_s5l60 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l60 is not None:
            if 2 * den_deg_s5l60 > s5l60p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 411: ``Mul(Sqrt(P1)×4, Log(h1)×60, polynomial..., bounded...)`` numerator.
    s4l60p_x2 = _four_sqrt_sixty_log_poly_effective_x2(num, k)
    if s4l60p_x2 is not None:
        den_deg_s4l60 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l60 is not None:
            if 2 * den_deg_s4l60 > s4l60p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 410: ``Mul(Sqrt(P1)×3, Log(h1)×60, polynomial..., bounded...)`` numerator.
    s3l60p_x2 = _three_sqrt_sixty_log_poly_effective_x2(num, k)
    if s3l60p_x2 is not None:
        den_deg_s3l60 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l60 is not None:
            if 2 * den_deg_s3l60 > s3l60p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 409: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×60, polynomial..., bounded...)`` numerator.
    s2l60p_x2 = _two_sqrt_sixty_log_poly_effective_x2(num, k)
    if s2l60p_x2 is not None:
        den_deg_s2l60 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l60 is not None:
            if 2 * den_deg_s2l60 > s2l60p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 408: ``Mul(Sqrt(P), Log(h1)×60, polynomial..., bounded...)`` numerator.
    s1l60p_x2 = _one_sqrt_sixty_log_poly_effective_x2(num, k)
    if s1l60p_x2 is not None:
        den_deg_s1l60 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l60 is not None:
            if 2 * den_deg_s1l60 > s1l60p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 407: ``Mul(Log(h1)×60, polynomial..., bounded...)`` numerator.
    sl60p_x2 = _sixty_log_poly_effective_x2(num, k)
    if sl60p_x2 is not None:
        den_deg_sl60 = _polynomial_degree_in_k(den, k)
        if den_deg_sl60 is not None:
            if 2 * den_deg_sl60 > sl60p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 406: ``Mul(Sqrt(P1)×5, Log(h1)×59, polynomial..., bounded...)`` numerator.
    s5l59p_x2 = _five_sqrt_fifty_nine_log_poly_effective_x2(num, k)
    if s5l59p_x2 is not None:
        den_deg_s5l59 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l59 is not None:
            if 2 * den_deg_s5l59 > s5l59p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 405: ``Mul(Sqrt(P1)×4, Log(h1)×59, polynomial..., bounded...)`` numerator.
    s4l59p_x2 = _four_sqrt_fifty_nine_log_poly_effective_x2(num, k)
    if s4l59p_x2 is not None:
        den_deg_s4l59 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l59 is not None:
            if 2 * den_deg_s4l59 > s4l59p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 404: ``Mul(Sqrt(P1)×3, Log(h1)×59, polynomial..., bounded...)`` numerator.
    s3l59p_x2 = _three_sqrt_fifty_nine_log_poly_effective_x2(num, k)
    if s3l59p_x2 is not None:
        den_deg_s3l59 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l59 is not None:
            if 2 * den_deg_s3l59 > s3l59p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 403: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×59, polynomial..., bounded...)`` numerator.
    s2l59p_x2 = _two_sqrt_fifty_nine_log_poly_effective_x2(num, k)
    if s2l59p_x2 is not None:
        den_deg_s2l59 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l59 is not None:
            if 2 * den_deg_s2l59 > s2l59p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 402: ``Mul(Sqrt(P), Log(h1)×59, polynomial..., bounded...)`` numerator.
    s1l59p_x2 = _one_sqrt_fifty_nine_log_poly_effective_x2(num, k)
    if s1l59p_x2 is not None:
        den_deg_s1l59 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l59 is not None:
            if 2 * den_deg_s1l59 > s1l59p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 401: ``Mul(Log(h1)×59, polynomial..., bounded...)`` numerator.
    sl59p_x2 = _fifty_nine_log_poly_effective_x2(num, k)
    if sl59p_x2 is not None:
        den_deg_sl59 = _polynomial_degree_in_k(den, k)
        if den_deg_sl59 is not None:
            if 2 * den_deg_sl59 > sl59p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 400: ``Mul(Sqrt(P1)×5, Log(h1)×58, polynomial..., bounded...)`` numerator.
    s5l58p_x2 = _five_sqrt_fifty_eight_log_poly_effective_x2(num, k)
    if s5l58p_x2 is not None:
        den_deg_s5l58 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l58 is not None:
            if 2 * den_deg_s5l58 > s5l58p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 399: ``Mul(Sqrt(P1)×4, Log(h1)×58, polynomial..., bounded...)`` numerator.
    s4l58p_x2 = _four_sqrt_fifty_eight_log_poly_effective_x2(num, k)
    if s4l58p_x2 is not None:
        den_deg_s4l58 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l58 is not None:
            if 2 * den_deg_s4l58 > s4l58p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 398: ``Mul(Sqrt(P1)×3, Log(h1)×58, polynomial..., bounded...)`` numerator.
    s3l58p_x2 = _three_sqrt_fifty_eight_log_poly_effective_x2(num, k)
    if s3l58p_x2 is not None:
        den_deg_s3l58 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l58 is not None:
            if 2 * den_deg_s3l58 > s3l58p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 397: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×58, polynomial..., bounded...)`` numerator.
    s2l58p_x2 = _two_sqrt_fifty_eight_log_poly_effective_x2(num, k)
    if s2l58p_x2 is not None:
        den_deg_s2l58 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l58 is not None:
            if 2 * den_deg_s2l58 > s2l58p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 396: ``Mul(Sqrt(P), Log(h1)×58, polynomial..., bounded...)`` numerator.
    s1l58p_x2 = _one_sqrt_fifty_eight_log_poly_effective_x2(num, k)
    if s1l58p_x2 is not None:
        den_deg_s1l58 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l58 is not None:
            if 2 * den_deg_s1l58 > s1l58p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 395: ``Mul(Log(h1)×58, polynomial..., bounded...)`` numerator.
    sl58p_x2 = _fifty_eight_log_poly_effective_x2(num, k)
    if sl58p_x2 is not None:
        den_deg_sl58 = _polynomial_degree_in_k(den, k)
        if den_deg_sl58 is not None:
            if 2 * den_deg_sl58 > sl58p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 394: ``Mul(Sqrt(P1)×5, Log(h1)×57, polynomial..., bounded...)`` numerator.
    s5l57p_x2 = _five_sqrt_fifty_seven_log_poly_effective_x2(num, k)
    if s5l57p_x2 is not None:
        den_deg_s5l57 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l57 is not None:
            if 2 * den_deg_s5l57 > s5l57p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 393: ``Mul(Sqrt(P1)×4, Log(h1)×57, polynomial..., bounded...)`` numerator.
    s4l57p_x2 = _four_sqrt_fifty_seven_log_poly_effective_x2(num, k)
    if s4l57p_x2 is not None:
        den_deg_s4l57 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l57 is not None:
            if 2 * den_deg_s4l57 > s4l57p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 392: ``Mul(Sqrt(P1)×3, Log(h1)×57, polynomial..., bounded...)`` numerator.
    s3l57p_x2 = _three_sqrt_fifty_seven_log_poly_effective_x2(num, k)
    if s3l57p_x2 is not None:
        den_deg_s3l57 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l57 is not None:
            if 2 * den_deg_s3l57 > s3l57p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 391: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×57, polynomial..., bounded...)`` numerator.
    s2l57p_x2 = _two_sqrt_fifty_seven_log_poly_effective_x2(num, k)
    if s2l57p_x2 is not None:
        den_deg_s2l57 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l57 is not None:
            if 2 * den_deg_s2l57 > s2l57p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 390: ``Mul(Sqrt(P), Log(h1)×57, polynomial..., bounded...)`` numerator.
    s1l57p_x2 = _one_sqrt_fifty_seven_log_poly_effective_x2(num, k)
    if s1l57p_x2 is not None:
        den_deg_s1l57 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l57 is not None:
            if 2 * den_deg_s1l57 > s1l57p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 389: ``Mul(Log(h1)×57, polynomial..., bounded...)`` numerator.
    sl57p_x2 = _fifty_seven_log_poly_effective_x2(num, k)
    if sl57p_x2 is not None:
        den_deg_sl57 = _polynomial_degree_in_k(den, k)
        if den_deg_sl57 is not None:
            if 2 * den_deg_sl57 > sl57p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 388: ``Mul(Sqrt(P1)×5, Log(h1)×56, polynomial..., bounded...)`` numerator.
    s5l56p_x2 = _five_sqrt_fifty_six_log_poly_effective_x2(num, k)
    if s5l56p_x2 is not None:
        den_deg_s5l56 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l56 is not None:
            if 2 * den_deg_s5l56 > s5l56p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 387: ``Mul(Sqrt(P1)×4, Log(h1)×56, polynomial..., bounded...)`` numerator.
    s4l56p_x2 = _four_sqrt_fifty_six_log_poly_effective_x2(num, k)
    if s4l56p_x2 is not None:
        den_deg_s4l56 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l56 is not None:
            if 2 * den_deg_s4l56 > s4l56p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 386: ``Mul(Sqrt(P1)×3, Log(h1)×56, polynomial..., bounded...)`` numerator.
    s3l56p_x2 = _three_sqrt_fifty_six_log_poly_effective_x2(num, k)
    if s3l56p_x2 is not None:
        den_deg_s3l56 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l56 is not None:
            if 2 * den_deg_s3l56 > s3l56p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 385: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×56, polynomial..., bounded...)`` numerator.
    s2l56p_x2 = _two_sqrt_fifty_six_log_poly_effective_x2(num, k)
    if s2l56p_x2 is not None:
        den_deg_s2l56 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l56 is not None:
            if 2 * den_deg_s2l56 > s2l56p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 384: ``Mul(Sqrt(P), Log(h1)×56, polynomial..., bounded...)`` numerator.
    s1l56p_x2 = _one_sqrt_fifty_six_log_poly_effective_x2(num, k)
    if s1l56p_x2 is not None:
        den_deg_s1l56 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l56 is not None:
            if 2 * den_deg_s1l56 > s1l56p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 383: ``Mul(Log(h1)×56, polynomial..., bounded...)`` numerator.
    sl56p_x2 = _fifty_six_log_poly_effective_x2(num, k)
    if sl56p_x2 is not None:
        den_deg_sl56 = _polynomial_degree_in_k(den, k)
        if den_deg_sl56 is not None:
            if 2 * den_deg_sl56 > sl56p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 382: ``Mul(Sqrt(P1)×5, Log(h1)×55, polynomial..., bounded...)`` numerator.
    s5l55p_x2 = _five_sqrt_fifty_five_log_poly_effective_x2(num, k)
    if s5l55p_x2 is not None:
        den_deg_s5l55 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l55 is not None:
            if 2 * den_deg_s5l55 > s5l55p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 381: ``Mul(Sqrt(P1)×4, Log(h1)×55, polynomial..., bounded...)`` numerator.
    s4l55p_x2 = _four_sqrt_fifty_five_log_poly_effective_x2(num, k)
    if s4l55p_x2 is not None:
        den_deg_s4l55 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l55 is not None:
            if 2 * den_deg_s4l55 > s4l55p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 380: ``Mul(Sqrt(P1)×3, Log(h1)×55, polynomial..., bounded...)`` numerator.
    s3l55p_x2 = _three_sqrt_fifty_five_log_poly_effective_x2(num, k)
    if s3l55p_x2 is not None:
        den_deg_s3l55 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l55 is not None:
            if 2 * den_deg_s3l55 > s3l55p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 379: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×55, polynomial..., bounded...)`` numerator.
    s2l55p_x2 = _two_sqrt_fifty_five_log_poly_effective_x2(num, k)
    if s2l55p_x2 is not None:
        den_deg_s2l55 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l55 is not None:
            if 2 * den_deg_s2l55 > s2l55p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 378: ``Mul(Sqrt(P), Log(h1)×55, polynomial..., bounded...)`` numerator.
    s1l55p_x2 = _one_sqrt_fifty_five_log_poly_effective_x2(num, k)
    if s1l55p_x2 is not None:
        den_deg_s1l55 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l55 is not None:
            if 2 * den_deg_s1l55 > s1l55p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 377: ``Mul(Log(h1)×55, polynomial..., bounded...)`` numerator.
    sl55p_x2 = _fifty_five_log_poly_effective_x2(num, k)
    if sl55p_x2 is not None:
        den_deg_sl55 = _polynomial_degree_in_k(den, k)
        if den_deg_sl55 is not None:
            if 2 * den_deg_sl55 > sl55p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 376: ``Mul(Sqrt(P1)×5, Log(h1)×54, polynomial..., bounded...)`` numerator.
    s5l54p_x2 = _five_sqrt_fifty_four_log_poly_effective_x2(num, k)
    if s5l54p_x2 is not None:
        den_deg_s5l54 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l54 is not None:
            if 2 * den_deg_s5l54 > s5l54p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 375: ``Mul(Sqrt(P1)×4, Log(h1)×54, polynomial..., bounded...)`` numerator.
    s4l54p_x2 = _four_sqrt_fifty_four_log_poly_effective_x2(num, k)
    if s4l54p_x2 is not None:
        den_deg_s4l54 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l54 is not None:
            if 2 * den_deg_s4l54 > s4l54p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 374: ``Mul(Sqrt(P1)×3, Log(h1)×54, polynomial..., bounded...)`` numerator.
    s3l54p_x2 = _three_sqrt_fifty_four_log_poly_effective_x2(num, k)
    if s3l54p_x2 is not None:
        den_deg_s3l54 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l54 is not None:
            if 2 * den_deg_s3l54 > s3l54p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 373: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×54, polynomial..., bounded...)`` numerator.
    s2l54p_x2 = _two_sqrt_fifty_four_log_poly_effective_x2(num, k)
    if s2l54p_x2 is not None:
        den_deg_s2l54 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l54 is not None:
            if 2 * den_deg_s2l54 > s2l54p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 372: ``Mul(Sqrt(P), Log(h1)×54, polynomial..., bounded...)`` numerator.
    s1l54p_x2 = _one_sqrt_fifty_four_log_poly_effective_x2(num, k)
    if s1l54p_x2 is not None:
        den_deg_s1l54 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l54 is not None:
            if 2 * den_deg_s1l54 > s1l54p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 371: ``Mul(Log(h1)×54, polynomial..., bounded...)`` numerator.
    sl54p_x2 = _fifty_four_log_poly_effective_x2(num, k)
    if sl54p_x2 is not None:
        den_deg_sl54 = _polynomial_degree_in_k(den, k)
        if den_deg_sl54 is not None:
            if 2 * den_deg_sl54 > sl54p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 370: ``Mul(Sqrt(P1)×5, Log(h1)×53, polynomial..., bounded...)`` numerator.
    s5l53p_x2 = _five_sqrt_fifty_three_log_poly_effective_x2(num, k)
    if s5l53p_x2 is not None:
        den_deg_s5l53 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l53 is not None:
            if 2 * den_deg_s5l53 > s5l53p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 369: ``Mul(Sqrt(P1)×4, Log(h1)×53, polynomial..., bounded...)`` numerator.
    s4l53p_x2 = _four_sqrt_fifty_three_log_poly_effective_x2(num, k)
    if s4l53p_x2 is not None:
        den_deg_s4l53 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l53 is not None:
            if 2 * den_deg_s4l53 > s4l53p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 368: ``Mul(Sqrt(P1)×3, Log(h1)×53, polynomial..., bounded...)`` numerator.
    s3l53p_x2 = _three_sqrt_fifty_three_log_poly_effective_x2(num, k)
    if s3l53p_x2 is not None:
        den_deg_s3l53 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l53 is not None:
            if 2 * den_deg_s3l53 > s3l53p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 367: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×53, polynomial..., bounded...)`` numerator.
    s2l53p_x2 = _two_sqrt_fifty_three_log_poly_effective_x2(num, k)
    if s2l53p_x2 is not None:
        den_deg_s2l53 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l53 is not None:
            if 2 * den_deg_s2l53 > s2l53p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 366: ``Mul(Sqrt(P), Log(h1)×53, polynomial..., bounded...)`` numerator.
    s1l53p_x2 = _one_sqrt_fifty_three_log_poly_effective_x2(num, k)
    if s1l53p_x2 is not None:
        den_deg_s1l53 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l53 is not None:
            if 2 * den_deg_s1l53 > s1l53p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 365: ``Mul(Log(h1)×53, polynomial..., bounded...)`` numerator.
    sl53p_x2 = _fifty_three_log_poly_effective_x2(num, k)
    if sl53p_x2 is not None:
        den_deg_sl53 = _polynomial_degree_in_k(den, k)
        if den_deg_sl53 is not None:
            if 2 * den_deg_sl53 > sl53p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 364: ``Mul(Sqrt(P1)×5, Log(h1)×52, polynomial..., bounded...)`` numerator.
    s5l52p_x2 = _five_sqrt_fifty_two_log_poly_effective_x2(num, k)
    if s5l52p_x2 is not None:
        den_deg_s5l52 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l52 is not None:
            if 2 * den_deg_s5l52 > s5l52p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 363: ``Mul(Sqrt(P1)×4, Log(h1)×52, polynomial..., bounded...)`` numerator.
    s4l52p_x2 = _four_sqrt_fifty_two_log_poly_effective_x2(num, k)
    if s4l52p_x2 is not None:
        den_deg_s4l52 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l52 is not None:
            if 2 * den_deg_s4l52 > s4l52p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 362: ``Mul(Sqrt(P1)×3, Log(h1)×52, polynomial..., bounded...)`` numerator.
    s3l52p_x2 = _three_sqrt_fifty_two_log_poly_effective_x2(num, k)
    if s3l52p_x2 is not None:
        den_deg_s3l52 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l52 is not None:
            if 2 * den_deg_s3l52 > s3l52p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 361: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×52, polynomial..., bounded...)`` numerator.
    s2l52p_x2 = _two_sqrt_fifty_two_log_poly_effective_x2(num, k)
    if s2l52p_x2 is not None:
        den_deg_s2l52 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l52 is not None:
            if 2 * den_deg_s2l52 > s2l52p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 360: ``Mul(Sqrt(P), Log(h1)×52, polynomial..., bounded...)`` numerator.
    s1l52p_x2 = _one_sqrt_fifty_two_log_poly_effective_x2(num, k)
    if s1l52p_x2 is not None:
        den_deg_s1l52 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l52 is not None:
            if 2 * den_deg_s1l52 > s1l52p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 359: ``Mul(Log(h1)×52, polynomial..., bounded...)`` numerator.
    sl52p_x2 = _fifty_two_log_poly_effective_x2(num, k)
    if sl52p_x2 is not None:
        den_deg_sl52 = _polynomial_degree_in_k(den, k)
        if den_deg_sl52 is not None:
            if 2 * den_deg_sl52 > sl52p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 358: ``Mul(Sqrt(P1)×5, Log(h1)×51, polynomial..., bounded...)`` numerator.
    s5l51p_x2 = _five_sqrt_fifty_one_log_poly_effective_x2(num, k)
    if s5l51p_x2 is not None:
        den_deg_s5l51 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l51 is not None:
            if 2 * den_deg_s5l51 > s5l51p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 357: ``Mul(Sqrt(P1)×4, Log(h1)×51, polynomial..., bounded...)`` numerator.
    s4l51p_x2 = _four_sqrt_fifty_one_log_poly_effective_x2(num, k)
    if s4l51p_x2 is not None:
        den_deg_s4l51 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l51 is not None:
            if 2 * den_deg_s4l51 > s4l51p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 356: ``Mul(Sqrt(P1)×3, Log(h1)×51, polynomial..., bounded...)`` numerator.
    s3l51p_x2 = _three_sqrt_fifty_one_log_poly_effective_x2(num, k)
    if s3l51p_x2 is not None:
        den_deg_s3l51 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l51 is not None:
            if 2 * den_deg_s3l51 > s3l51p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 355: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×51, polynomial..., bounded...)`` numerator.
    s2l51p_x2 = _two_sqrt_fifty_one_log_poly_effective_x2(num, k)
    if s2l51p_x2 is not None:
        den_deg_s2l51 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l51 is not None:
            if 2 * den_deg_s2l51 > s2l51p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 354: ``Mul(Sqrt(P), Log(h1)×51, polynomial..., bounded...)`` numerator.
    s1l51p_x2 = _one_sqrt_fifty_one_log_poly_effective_x2(num, k)
    if s1l51p_x2 is not None:
        den_deg_s1l51 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l51 is not None:
            if 2 * den_deg_s1l51 > s1l51p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 353: ``Mul(Log(h1)×51, polynomial..., bounded...)`` numerator.
    sl51p_x2 = _fifty_one_log_poly_effective_x2(num, k)
    if sl51p_x2 is not None:
        den_deg_sl51 = _polynomial_degree_in_k(den, k)
        if den_deg_sl51 is not None:
            if 2 * den_deg_sl51 > sl51p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 352: ``Mul(Sqrt(P1)×5, Log(h1)×50, polynomial..., bounded...)`` numerator.
    s5l50p_x2 = _five_sqrt_fifty_log_poly_effective_x2(num, k)
    if s5l50p_x2 is not None:
        den_deg_s5l50 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l50 is not None:
            if 2 * den_deg_s5l50 > s5l50p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 351: ``Mul(Sqrt(P1)×4, Log(h1)×50, polynomial..., bounded...)`` numerator.
    s4l50p_x2 = _four_sqrt_fifty_log_poly_effective_x2(num, k)
    if s4l50p_x2 is not None:
        den_deg_s4l50 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l50 is not None:
            if 2 * den_deg_s4l50 > s4l50p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 350: ``Mul(Sqrt(P1)×3, Log(h1)×50, polynomial..., bounded...)`` numerator.
    s3l50p_x2 = _three_sqrt_fifty_log_poly_effective_x2(num, k)
    if s3l50p_x2 is not None:
        den_deg_s3l50 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l50 is not None:
            if 2 * den_deg_s3l50 > s3l50p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 349: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×50, polynomial..., bounded...)`` numerator.
    s2l50p_x2 = _two_sqrt_fifty_log_poly_effective_x2(num, k)
    if s2l50p_x2 is not None:
        den_deg_s2l50 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l50 is not None:
            if 2 * den_deg_s2l50 > s2l50p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 348: ``Mul(Sqrt(P), Log(h1)×50, polynomial..., bounded...)`` numerator.
    s1l50p_x2 = _one_sqrt_fifty_log_poly_effective_x2(num, k)
    if s1l50p_x2 is not None:
        den_deg_s1l50 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l50 is not None:
            if 2 * den_deg_s1l50 > s1l50p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 347: ``Mul(Log(h1)×50, polynomial..., bounded...)`` numerator.
    sl50p_x2 = _fifty_log_poly_effective_x2(num, k)
    if sl50p_x2 is not None:
        den_deg_sl50 = _polynomial_degree_in_k(den, k)
        if den_deg_sl50 is not None:
            if 2 * den_deg_sl50 > sl50p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 346: ``Mul(Sqrt(P1)×5, Log(h1)×49, polynomial..., bounded...)`` numerator.
    s5l49p_x2 = _five_sqrt_forty_nine_log_poly_effective_x2(num, k)
    if s5l49p_x2 is not None:
        den_deg_s5l49 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l49 is not None:
            if 2 * den_deg_s5l49 > s5l49p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 345: ``Mul(Sqrt(P1)×4, Log(h1)×49, polynomial..., bounded...)`` numerator.
    s4l49p_x2 = _four_sqrt_forty_nine_log_poly_effective_x2(num, k)
    if s4l49p_x2 is not None:
        den_deg_s4l49 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l49 is not None:
            if 2 * den_deg_s4l49 > s4l49p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 344: ``Mul(Sqrt(P1)×3, Log(h1)×49, polynomial..., bounded...)`` numerator.
    s3l49p_x2 = _three_sqrt_forty_nine_log_poly_effective_x2(num, k)
    if s3l49p_x2 is not None:
        den_deg_s3l49 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l49 is not None:
            if 2 * den_deg_s3l49 > s3l49p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 343: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×49, polynomial..., bounded...)`` numerator.
    s2l49p_x2 = _two_sqrt_forty_nine_log_poly_effective_x2(num, k)
    if s2l49p_x2 is not None:
        den_deg_s2l49 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l49 is not None:
            if 2 * den_deg_s2l49 > s2l49p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 342: ``Mul(Sqrt(P), Log(h1)×49, polynomial..., bounded...)`` numerator.
    s1l49p_x2 = _one_sqrt_forty_nine_log_poly_effective_x2(num, k)
    if s1l49p_x2 is not None:
        den_deg_s1l49 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l49 is not None:
            if 2 * den_deg_s1l49 > s1l49p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 341: ``Mul(Log(h1)×49, polynomial..., bounded...)`` numerator.
    sl49p_x2 = _forty_nine_log_poly_effective_x2(num, k)
    if sl49p_x2 is not None:
        den_deg_sl49 = _polynomial_degree_in_k(den, k)
        if den_deg_sl49 is not None:
            if 2 * den_deg_sl49 > sl49p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 340: ``Mul(Sqrt(P1)×5, Log(h1)×48, polynomial..., bounded...)`` numerator.
    s5l48p_x2 = _five_sqrt_forty_eight_log_poly_effective_x2(num, k)
    if s5l48p_x2 is not None:
        den_deg_s5l48 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l48 is not None:
            if 2 * den_deg_s5l48 > s5l48p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 339: ``Mul(Sqrt(P1)×4, Log(h1)×48, polynomial..., bounded...)`` numerator.
    s4l48p_x2 = _four_sqrt_forty_eight_log_poly_effective_x2(num, k)
    if s4l48p_x2 is not None:
        den_deg_s4l48 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l48 is not None:
            if 2 * den_deg_s4l48 > s4l48p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 338: ``Mul(Sqrt(P1)×3, Log(h1)×48, polynomial..., bounded...)`` numerator.
    s3l48p_x2 = _three_sqrt_forty_eight_log_poly_effective_x2(num, k)
    if s3l48p_x2 is not None:
        den_deg_s3l48 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l48 is not None:
            if 2 * den_deg_s3l48 > s3l48p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 337: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×48, polynomial..., bounded...)`` numerator.
    s2l48p_x2 = _two_sqrt_forty_eight_log_poly_effective_x2(num, k)
    if s2l48p_x2 is not None:
        den_deg_s2l48 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l48 is not None:
            if 2 * den_deg_s2l48 > s2l48p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 336: ``Mul(Sqrt(P), Log(h1)×48, polynomial..., bounded...)`` numerator.
    s1l48p_x2 = _one_sqrt_forty_eight_log_poly_effective_x2(num, k)
    if s1l48p_x2 is not None:
        den_deg_s1l48 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l48 is not None:
            if 2 * den_deg_s1l48 > s1l48p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 335: ``Mul(Log(h1)×48, polynomial..., bounded...)`` numerator.
    sl48p_x2 = _forty_eight_log_poly_effective_x2(num, k)
    if sl48p_x2 is not None:
        den_deg_sl48 = _polynomial_degree_in_k(den, k)
        if den_deg_sl48 is not None:
            if 2 * den_deg_sl48 > sl48p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 334: ``Mul(Sqrt(P1)×5, Log(h1)×47, polynomial..., bounded...)`` numerator.
    s5l47p_x2 = _five_sqrt_forty_seven_log_poly_effective_x2(num, k)
    if s5l47p_x2 is not None:
        den_deg_s5l47 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l47 is not None:
            if 2 * den_deg_s5l47 > s5l47p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 333: ``Mul(Sqrt(P1)×4, Log(h1)×47, polynomial..., bounded...)`` numerator.
    s4l47p_x2 = _four_sqrt_forty_seven_log_poly_effective_x2(num, k)
    if s4l47p_x2 is not None:
        den_deg_s4l47 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l47 is not None:
            if 2 * den_deg_s4l47 > s4l47p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 332: ``Mul(Sqrt(P1)×3, Log(h1)×47, polynomial..., bounded...)`` numerator.
    s3l47p_x2 = _three_sqrt_forty_seven_log_poly_effective_x2(num, k)
    if s3l47p_x2 is not None:
        den_deg_s3l47 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l47 is not None:
            if 2 * den_deg_s3l47 > s3l47p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 331: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×47, polynomial..., bounded...)`` numerator.
    s2l47p_x2 = _two_sqrt_forty_seven_log_poly_effective_x2(num, k)
    if s2l47p_x2 is not None:
        den_deg_s2l47 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l47 is not None:
            if 2 * den_deg_s2l47 > s2l47p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 330: ``Mul(Sqrt(P), Log(h1)×47, polynomial..., bounded...)`` numerator.
    s1l47p_x2 = _one_sqrt_forty_seven_log_poly_effective_x2(num, k)
    if s1l47p_x2 is not None:
        den_deg_s1l47 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l47 is not None:
            if 2 * den_deg_s1l47 > s1l47p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 329: ``Mul(Log(h1)×47, polynomial..., bounded...)`` numerator.
    sl47p_x2 = _forty_seven_log_poly_effective_x2(num, k)
    if sl47p_x2 is not None:
        den_deg_sl47 = _polynomial_degree_in_k(den, k)
        if den_deg_sl47 is not None:
            if 2 * den_deg_sl47 > sl47p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 328: ``Mul(Sqrt(P1)×5, Log(h1)×46, polynomial..., bounded...)`` numerator.
    s5l46p_x2 = _five_sqrt_forty_six_log_poly_effective_x2(num, k)
    if s5l46p_x2 is not None:
        den_deg_s5l46 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l46 is not None:
            if 2 * den_deg_s5l46 > s5l46p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 327: ``Mul(Sqrt(P1)×4, Log(h1)×46, polynomial..., bounded...)`` numerator.
    s4l46p_x2 = _four_sqrt_forty_six_log_poly_effective_x2(num, k)
    if s4l46p_x2 is not None:
        den_deg_s4l46 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l46 is not None:
            if 2 * den_deg_s4l46 > s4l46p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 326: ``Mul(Sqrt(P1)×3, Log(h1)×46, polynomial..., bounded...)`` numerator.
    s3l46p_x2 = _three_sqrt_forty_six_log_poly_effective_x2(num, k)
    if s3l46p_x2 is not None:
        den_deg_s3l46 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l46 is not None:
            if 2 * den_deg_s3l46 > s3l46p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 325: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×46, polynomial..., bounded...)`` numerator.
    s2l46p_x2 = _two_sqrt_forty_six_log_poly_effective_x2(num, k)
    if s2l46p_x2 is not None:
        den_deg_s2l46 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l46 is not None:
            if 2 * den_deg_s2l46 > s2l46p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 324: ``Mul(Sqrt(P), Log(h1)×46, polynomial..., bounded...)`` numerator.
    s1l46p_x2 = _one_sqrt_forty_six_log_poly_effective_x2(num, k)
    if s1l46p_x2 is not None:
        den_deg_s1l46 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l46 is not None:
            if 2 * den_deg_s1l46 > s1l46p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 323: ``Mul(Log(h1)×46, polynomial..., bounded...)`` numerator.
    sl46p_x2 = _forty_six_log_poly_effective_x2(num, k)
    if sl46p_x2 is not None:
        den_deg_sl46 = _polynomial_degree_in_k(den, k)
        if den_deg_sl46 is not None:
            if 2 * den_deg_sl46 > sl46p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 322: ``Mul(Sqrt(P1)×5, Log(h1)×45, polynomial..., bounded...)`` numerator.
    s5l45p_x2 = _five_sqrt_forty_five_log_poly_effective_x2(num, k)
    if s5l45p_x2 is not None:
        den_deg_s5l45 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l45 is not None:
            if 2 * den_deg_s5l45 > s5l45p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 321: ``Mul(Sqrt(P1)×4, Log(h1)×45, polynomial..., bounded...)`` numerator.
    s4l45p_x2 = _four_sqrt_forty_five_log_poly_effective_x2(num, k)
    if s4l45p_x2 is not None:
        den_deg_s4l45 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l45 is not None:
            if 2 * den_deg_s4l45 > s4l45p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 320: ``Mul(Sqrt(P1)×3, Log(h1)×45, polynomial..., bounded...)`` numerator.
    s3l45p_x2 = _three_sqrt_forty_five_log_poly_effective_x2(num, k)
    if s3l45p_x2 is not None:
        den_deg_s3l45 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l45 is not None:
            if 2 * den_deg_s3l45 > s3l45p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 319: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×45, polynomial..., bounded...)`` numerator.
    s2l45p_x2 = _two_sqrt_forty_five_log_poly_effective_x2(num, k)
    if s2l45p_x2 is not None:
        den_deg_s2l45 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l45 is not None:
            if 2 * den_deg_s2l45 > s2l45p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 318: ``Mul(Sqrt(P), Log(h1)×45, polynomial..., bounded...)`` numerator.
    s1l45p_x2 = _one_sqrt_forty_five_log_poly_effective_x2(num, k)
    if s1l45p_x2 is not None:
        den_deg_s1l45 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l45 is not None:
            if 2 * den_deg_s1l45 > s1l45p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 317: ``Mul(Log(h1)×45, polynomial..., bounded...)`` numerator.
    sl45p_x2 = _forty_five_log_poly_effective_x2(num, k)
    if sl45p_x2 is not None:
        den_deg_sl45 = _polynomial_degree_in_k(den, k)
        if den_deg_sl45 is not None:
            if 2 * den_deg_sl45 > sl45p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 316: ``Mul(Sqrt(P1)×5, Log(h1)×44, polynomial..., bounded...)`` numerator.
    s5l44p_x2 = _five_sqrt_forty_four_log_poly_effective_x2(num, k)
    if s5l44p_x2 is not None:
        den_deg_s5l44 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l44 is not None:
            if 2 * den_deg_s5l44 > s5l44p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 315: ``Mul(Sqrt(P1)×4, Log(h1)×44, polynomial..., bounded...)`` numerator.
    s4l44p_x2 = _four_sqrt_forty_four_log_poly_effective_x2(num, k)
    if s4l44p_x2 is not None:
        den_deg_s4l44 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l44 is not None:
            if 2 * den_deg_s4l44 > s4l44p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 314: ``Mul(Sqrt(P1)×3, Log(h1)×44, polynomial..., bounded...)`` numerator.
    s3l44p_x2 = _three_sqrt_forty_four_log_poly_effective_x2(num, k)
    if s3l44p_x2 is not None:
        den_deg_s3l44 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l44 is not None:
            if 2 * den_deg_s3l44 > s3l44p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 313: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×44, polynomial..., bounded...)`` numerator.
    s2l44p_x2 = _two_sqrt_forty_four_log_poly_effective_x2(num, k)
    if s2l44p_x2 is not None:
        den_deg_s2l44 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l44 is not None:
            if 2 * den_deg_s2l44 > s2l44p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 312: ``Mul(Sqrt(P), Log(h1)×44, polynomial..., bounded...)`` numerator.
    s1l44p_x2 = _one_sqrt_forty_four_log_poly_effective_x2(num, k)
    if s1l44p_x2 is not None:
        den_deg_s1l44 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l44 is not None:
            if 2 * den_deg_s1l44 > s1l44p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 311: ``Mul(Log(h1)×44, polynomial..., bounded...)`` numerator.
    sl44p_x2 = _forty_four_log_poly_effective_x2(num, k)
    if sl44p_x2 is not None:
        den_deg_sl44 = _polynomial_degree_in_k(den, k)
        if den_deg_sl44 is not None:
            if 2 * den_deg_sl44 > sl44p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 310: ``Mul(Sqrt(P1)×5, Log(h1)×43, polynomial..., bounded...)`` numerator.
    s5l43p_x2 = _five_sqrt_forty_three_log_poly_effective_x2(num, k)
    if s5l43p_x2 is not None:
        den_deg_s5l43 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l43 is not None:
            if 2 * den_deg_s5l43 > s5l43p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 309: ``Mul(Sqrt(P1)×4, Log(h1)×43, polynomial..., bounded...)`` numerator.
    s4l43p_x2 = _four_sqrt_forty_three_log_poly_effective_x2(num, k)
    if s4l43p_x2 is not None:
        den_deg_s4l43 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l43 is not None:
            if 2 * den_deg_s4l43 > s4l43p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 308: ``Mul(Sqrt(P1)×3, Log(h1)×43, polynomial..., bounded...)`` numerator.
    s3l43p_x2 = _three_sqrt_forty_three_log_poly_effective_x2(num, k)
    if s3l43p_x2 is not None:
        den_deg_s3l43 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l43 is not None:
            if 2 * den_deg_s3l43 > s3l43p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 307: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×43, polynomial..., bounded...)`` numerator.
    s2l43p_x2 = _two_sqrt_forty_three_log_poly_effective_x2(num, k)
    if s2l43p_x2 is not None:
        den_deg_s2l43 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l43 is not None:
            if 2 * den_deg_s2l43 > s2l43p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 306: ``Mul(Sqrt(P), Log(h1)×43, polynomial..., bounded...)`` numerator.
    s1l43p_x2 = _one_sqrt_forty_three_log_poly_effective_x2(num, k)
    if s1l43p_x2 is not None:
        den_deg_s1l43 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l43 is not None:
            if 2 * den_deg_s1l43 > s1l43p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 305: ``Mul(Log(h1)×43, polynomial..., bounded...)`` numerator.
    sl43p_x2 = _forty_three_log_poly_effective_x2(num, k)
    if sl43p_x2 is not None:
        den_deg_sl43 = _polynomial_degree_in_k(den, k)
        if den_deg_sl43 is not None:
            if 2 * den_deg_sl43 > sl43p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 304: ``Mul(Sqrt(P1)×5, Log(h1)×42, polynomial..., bounded...)`` numerator.
    s5l42p_x2 = _five_sqrt_forty_two_log_poly_effective_x2(num, k)
    if s5l42p_x2 is not None:
        den_deg_s5l42 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l42 is not None:
            if 2 * den_deg_s5l42 > s5l42p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 303: ``Mul(Sqrt(P1)×4, Log(h1)×42, polynomial..., bounded...)`` numerator.
    s4l42p_x2 = _four_sqrt_forty_two_log_poly_effective_x2(num, k)
    if s4l42p_x2 is not None:
        den_deg_s4l42 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l42 is not None:
            if 2 * den_deg_s4l42 > s4l42p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 302: ``Mul(Sqrt(P1)×3, Log(h1)×42, polynomial..., bounded...)`` numerator.
    s3l42p_x2 = _three_sqrt_forty_two_log_poly_effective_x2(num, k)
    if s3l42p_x2 is not None:
        den_deg_s3l42 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l42 is not None:
            if 2 * den_deg_s3l42 > s3l42p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 301: ``Mul(Sqrt(P), Sqrt(P2), Log(h1)×42, polynomial..., bounded...)`` numerator.
    s2l42p_x2 = _two_sqrt_forty_two_log_poly_effective_x2(num, k)
    if s2l42p_x2 is not None:
        den_deg_s2l42 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l42 is not None:
            if 2 * den_deg_s2l42 > s2l42p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 300: ``Mul(Sqrt(P), Log(h1)×42, polynomial..., bounded...)`` numerator.
    s1l42p_x2 = _one_sqrt_forty_two_log_poly_effective_x2(num, k)
    if s1l42p_x2 is not None:
        den_deg_s1l42 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l42 is not None:
            if 2 * den_deg_s1l42 > s1l42p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 299: ``Mul(Log(h1)×42, polynomial..., bounded...)`` numerator.
    sl42p_x2 = _forty_two_log_poly_effective_x2(num, k)
    if sl42p_x2 is not None:
        den_deg_sl42 = _polynomial_degree_in_k(den, k)
        if den_deg_sl42 is not None:
            if 2 * den_deg_sl42 > sl42p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 298: ``Mul(Sqrt(P1)×5, Log(h1)×41, polynomial..., bounded...)`` numerator.
    s5l41p_x2 = _five_sqrt_forty_one_log_poly_effective_x2(num, k)
    if s5l41p_x2 is not None:
        den_deg_s5l41 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l41 is not None:
            if 2 * den_deg_s5l41 > s5l41p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 297: ``Mul(Sqrt(P1)×4, Log(h1)×41, polynomial..., bounded...)`` numerator.
    s4l41p_x2 = _four_sqrt_forty_one_log_poly_effective_x2(num, k)
    if s4l41p_x2 is not None:
        den_deg_s4l41 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l41 is not None:
            if 2 * den_deg_s4l41 > s4l41p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 296: ``Mul(Sqrt(P1)×3, Log(h1)×41, polynomial..., bounded...)`` numerator.
    s3l41p_x2 = _three_sqrt_forty_one_log_poly_effective_x2(num, k)
    if s3l41p_x2 is not None:
        den_deg_s3l41 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l41 is not None:
            if 2 * den_deg_s3l41 > s3l41p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 295: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×41, polynomial..., bounded...)`` numerator.
    s2l41p_x2 = _two_sqrt_forty_one_log_poly_effective_x2(num, k)
    if s2l41p_x2 is not None:
        den_deg_s2l41 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l41 is not None:
            if 2 * den_deg_s2l41 > s2l41p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 294: ``Mul(Sqrt(P), Log(h1)×41, polynomial..., bounded...)`` numerator.
    s1l41p_x2 = _one_sqrt_forty_one_log_poly_effective_x2(num, k)
    if s1l41p_x2 is not None:
        den_deg_s1l41 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l41 is not None:
            if 2 * den_deg_s1l41 > s1l41p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 293: ``Mul(Log(h1)×41, polynomial..., bounded...)`` numerator.
    sl41p_x2 = _forty_one_log_poly_effective_x2(num, k)
    if sl41p_x2 is not None:
        den_deg_sl41 = _polynomial_degree_in_k(den, k)
        if den_deg_sl41 is not None:
            if 2 * den_deg_sl41 > sl41p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 292: ``Mul(Sqrt(P1)×5, Log(h1)×40, polynomial..., bounded...)`` numerator.
    s5l40p_x2 = _five_sqrt_forty_log_poly_effective_x2(num, k)
    if s5l40p_x2 is not None:
        den_deg_s5l40 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l40 is not None:
            if 2 * den_deg_s5l40 > s5l40p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 291: ``Mul(Sqrt(P1)×4, Log(h1)×40, polynomial..., bounded...)`` numerator.
    s4l40p_x2 = _four_sqrt_forty_log_poly_effective_x2(num, k)
    if s4l40p_x2 is not None:
        den_deg_s4l40 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l40 is not None:
            if 2 * den_deg_s4l40 > s4l40p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 290: ``Mul(Sqrt(P1)×3, Log(h1)×40, polynomial..., bounded...)`` numerator.
    s3l40p_x2 = _three_sqrt_forty_log_poly_effective_x2(num, k)
    if s3l40p_x2 is not None:
        den_deg_s3l40 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l40 is not None:
            if 2 * den_deg_s3l40 > s3l40p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 289: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×40, polynomial..., bounded...)`` numerator.
    s2l40p_x2 = _two_sqrt_forty_log_poly_effective_x2(num, k)
    if s2l40p_x2 is not None:
        den_deg_s2l40 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l40 is not None:
            if 2 * den_deg_s2l40 > s2l40p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 288: ``Mul(Sqrt(P), Log(h1)×40, polynomial..., bounded...)`` numerator.
    s1l40p_x2 = _one_sqrt_forty_log_poly_effective_x2(num, k)
    if s1l40p_x2 is not None:
        den_deg_s1l40 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l40 is not None:
            if 2 * den_deg_s1l40 > s1l40p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 287: ``Mul(Log(h1)×40, polynomial..., bounded...)`` numerator.
    sl40p_x2 = _forty_log_poly_effective_x2(num, k)
    if sl40p_x2 is not None:
        den_deg_sl40 = _polynomial_degree_in_k(den, k)
        if den_deg_sl40 is not None:
            if 2 * den_deg_sl40 > sl40p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 286: ``Mul(Sqrt(P1)×5, Log(h1)×39, polynomial..., bounded...)`` numerator.
    s5l39p_x2 = _five_sqrt_thirty_nine_log_poly_effective_x2(num, k)
    if s5l39p_x2 is not None:
        den_deg_s5l39 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l39 is not None:
            if 2 * den_deg_s5l39 > s5l39p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 285: ``Mul(Sqrt(P1)×4, Log(h1)×39, polynomial..., bounded...)`` numerator.
    s4l39p_x2 = _four_sqrt_thirty_nine_log_poly_effective_x2(num, k)
    if s4l39p_x2 is not None:
        den_deg_s4l39 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l39 is not None:
            if 2 * den_deg_s4l39 > s4l39p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 284: ``Mul(Sqrt(P1)×3, Log(h1)×39, polynomial..., bounded...)`` numerator.
    s3l39p_x2 = _three_sqrt_thirty_nine_log_poly_effective_x2(num, k)
    if s3l39p_x2 is not None:
        den_deg_s3l39 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l39 is not None:
            if 2 * den_deg_s3l39 > s3l39p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 283: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×39, polynomial..., bounded...)`` numerator.
    s2l39p_x2 = _two_sqrt_thirty_nine_log_poly_effective_x2(num, k)
    if s2l39p_x2 is not None:
        den_deg_s2l39 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l39 is not None:
            if 2 * den_deg_s2l39 > s2l39p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 282: ``Mul(Sqrt(P), Log(h1)×39, polynomial..., bounded...)`` numerator.
    s1l39p_x2 = _one_sqrt_thirty_nine_log_poly_effective_x2(num, k)
    if s1l39p_x2 is not None:
        den_deg_s1l39 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l39 is not None:
            if 2 * den_deg_s1l39 > s1l39p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 281: ``Mul(Log(h1)×39, polynomial..., bounded...)`` numerator.
    sl39p_x2 = _thirty_nine_log_poly_effective_x2(num, k)
    if sl39p_x2 is not None:
        den_deg_sl39 = _polynomial_degree_in_k(den, k)
        if den_deg_sl39 is not None:
            if 2 * den_deg_sl39 > sl39p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 280: ``Mul(Sqrt(P1)×5, Log(h1)×38, polynomial..., bounded...)`` numerator.
    s5l38p_x2 = _five_sqrt_thirty_eight_log_poly_effective_x2(num, k)
    if s5l38p_x2 is not None:
        den_deg_s5l38 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l38 is not None:
            if 2 * den_deg_s5l38 > s5l38p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 279: ``Mul(Sqrt(P1)×4, Log(h1)×38, polynomial..., bounded...)`` numerator.
    s4l38p_x2 = _four_sqrt_thirty_eight_log_poly_effective_x2(num, k)
    if s4l38p_x2 is not None:
        den_deg_s4l38 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l38 is not None:
            if 2 * den_deg_s4l38 > s4l38p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 278: ``Mul(Sqrt(P1)×3, Log(h1)×38, polynomial..., bounded...)`` numerator.
    s3l38p_x2 = _three_sqrt_thirty_eight_log_poly_effective_x2(num, k)
    if s3l38p_x2 is not None:
        den_deg_s3l38 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l38 is not None:
            if 2 * den_deg_s3l38 > s3l38p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 277: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×38, polynomial..., bounded...)`` numerator.
    s2l38p_x2 = _two_sqrt_thirty_eight_log_poly_effective_x2(num, k)
    if s2l38p_x2 is not None:
        den_deg_s2l38 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l38 is not None:
            if 2 * den_deg_s2l38 > s2l38p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 276: ``Mul(Sqrt(P), Log(h1)×38, polynomial..., bounded...)`` numerator.
    s1l38p_x2 = _one_sqrt_thirty_eight_log_poly_effective_x2(num, k)
    if s1l38p_x2 is not None:
        den_deg_s1l38 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l38 is not None:
            if 2 * den_deg_s1l38 > s1l38p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 275: ``Mul(Log(h1)×38, polynomial..., bounded...)`` numerator.
    sl38p_x2 = _thirty_eight_log_poly_effective_x2(num, k)
    if sl38p_x2 is not None:
        den_deg_sl38 = _polynomial_degree_in_k(den, k)
        if den_deg_sl38 is not None:
            if 2 * den_deg_sl38 > sl38p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 274: ``Mul(Sqrt(P1)×5, Log(h1)×37, polynomial..., bounded...)`` numerator.
    s5l37p_x2 = _five_sqrt_thirty_seven_log_poly_effective_x2(num, k)
    if s5l37p_x2 is not None:
        den_deg_s5l37 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l37 is not None:
            if 2 * den_deg_s5l37 > s5l37p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 273: ``Mul(Sqrt(P1)×4, Log(h1)×37, polynomial..., bounded...)`` numerator.
    s4l37p_x2 = _four_sqrt_thirty_seven_log_poly_effective_x2(num, k)
    if s4l37p_x2 is not None:
        den_deg_s4l37 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l37 is not None:
            if 2 * den_deg_s4l37 > s4l37p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 272: ``Mul(Sqrt(P1)×3, Log(h1)×37, polynomial..., bounded...)`` numerator.
    s3l37p_x2 = _three_sqrt_thirty_seven_log_poly_effective_x2(num, k)
    if s3l37p_x2 is not None:
        den_deg_s3l37 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l37 is not None:
            if 2 * den_deg_s3l37 > s3l37p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 271: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×37, polynomial..., bounded...)`` numerator.
    s2l37p_x2 = _two_sqrt_thirty_seven_log_poly_effective_x2(num, k)
    if s2l37p_x2 is not None:
        den_deg_s2l37 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l37 is not None:
            if 2 * den_deg_s2l37 > s2l37p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 270: ``Mul(Sqrt(P), Log(h1)×37, polynomial..., bounded...)`` numerator.
    s1l37p_x2 = _one_sqrt_thirty_seven_log_poly_effective_x2(num, k)
    if s1l37p_x2 is not None:
        den_deg_s1l37 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l37 is not None:
            if 2 * den_deg_s1l37 > s1l37p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 269: ``Mul(Log(h1)×37, polynomial..., bounded...)`` numerator.
    sl37p_x2 = _thirty_seven_log_poly_effective_x2(num, k)
    if sl37p_x2 is not None:
        den_deg_sl37 = _polynomial_degree_in_k(den, k)
        if den_deg_sl37 is not None:
            if 2 * den_deg_sl37 > sl37p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 268: ``Mul(Sqrt(P1)×5, Log(h1)×36, polynomial..., bounded...)`` numerator.
    s5l36p_x2 = _five_sqrt_thirty_six_log_poly_effective_x2(num, k)
    if s5l36p_x2 is not None:
        den_deg_s5l36 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l36 is not None:
            if 2 * den_deg_s5l36 > s5l36p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 267: ``Mul(Sqrt(P1)×4, Log(h1)×36, polynomial..., bounded...)`` numerator.
    s4l36p_x2 = _four_sqrt_thirty_six_log_poly_effective_x2(num, k)
    if s4l36p_x2 is not None:
        den_deg_s4l36 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l36 is not None:
            if 2 * den_deg_s4l36 > s4l36p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 266: ``Mul(Sqrt(P1)×3, Log(h1)×36, polynomial..., bounded...)`` numerator.
    s3l36p_x2 = _three_sqrt_thirty_six_log_poly_effective_x2(num, k)
    if s3l36p_x2 is not None:
        den_deg_s3l36 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l36 is not None:
            if 2 * den_deg_s3l36 > s3l36p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 265: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×36, polynomial..., bounded...)`` numerator.
    s2l36p_x2 = _two_sqrt_thirty_six_log_poly_effective_x2(num, k)
    if s2l36p_x2 is not None:
        den_deg_s2l36 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l36 is not None:
            if 2 * den_deg_s2l36 > s2l36p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 264: ``Mul(Sqrt(P), Log(h1)×36, polynomial..., bounded...)`` numerator.
    s1l36p_x2 = _one_sqrt_thirty_six_log_poly_effective_x2(num, k)
    if s1l36p_x2 is not None:
        den_deg_s1l36 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l36 is not None:
            if 2 * den_deg_s1l36 > s1l36p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 263: ``Mul(Log(h1)×36, polynomial..., bounded...)`` numerator.
    sl36p_x2 = _thirty_six_log_poly_effective_x2(num, k)
    if sl36p_x2 is not None:
        den_deg_sl36 = _polynomial_degree_in_k(den, k)
        if den_deg_sl36 is not None:
            if 2 * den_deg_sl36 > sl36p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 262: ``Mul(Sqrt(P1)×5, Log(h1)×35, polynomial..., bounded...)`` numerator.
    s5l35p_x2 = _five_sqrt_thirty_five_log_poly_effective_x2(num, k)
    if s5l35p_x2 is not None:
        den_deg_s5l35 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l35 is not None:
            if 2 * den_deg_s5l35 > s5l35p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 261: ``Mul(Sqrt(P1)×4, Log(h1)×35, polynomial..., bounded...)`` numerator.
    s4l35p_x2 = _four_sqrt_thirty_five_log_poly_effective_x2(num, k)
    if s4l35p_x2 is not None:
        den_deg_s4l35 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l35 is not None:
            if 2 * den_deg_s4l35 > s4l35p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 260: ``Mul(Sqrt(P1)×3, Log(h1)×35, polynomial..., bounded...)`` numerator.
    s3l35p_x2 = _three_sqrt_thirty_five_log_poly_effective_x2(num, k)
    if s3l35p_x2 is not None:
        den_deg_s3l35 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l35 is not None:
            if 2 * den_deg_s3l35 > s3l35p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 259: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×35, polynomial..., bounded...)`` numerator.
    s2l35p_x2 = _two_sqrt_thirty_five_log_poly_effective_x2(num, k)
    if s2l35p_x2 is not None:
        den_deg_s2l35 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l35 is not None:
            if 2 * den_deg_s2l35 > s2l35p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 258: ``Mul(Sqrt(P), Log(h1)×35, polynomial..., bounded...)`` numerator.
    s1l35p_x2 = _one_sqrt_thirty_five_log_poly_effective_x2(num, k)
    if s1l35p_x2 is not None:
        den_deg_s1l35 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l35 is not None:
            if 2 * den_deg_s1l35 > s1l35p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 257: ``Mul(Log(h1)×35, polynomial..., bounded...)`` numerator.
    sl35p_x2 = _thirty_five_log_poly_effective_x2(num, k)
    if sl35p_x2 is not None:
        den_deg_sl35 = _polynomial_degree_in_k(den, k)
        if den_deg_sl35 is not None:
            if 2 * den_deg_sl35 > sl35p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 256: ``Mul(Sqrt(P1)×5, Log(h1)×34, polynomial..., bounded...)`` numerator.
    s5l34p_x2 = _five_sqrt_thirty_four_log_poly_effective_x2(num, k)
    if s5l34p_x2 is not None:
        den_deg_s5l34 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l34 is not None:
            if 2 * den_deg_s5l34 > s5l34p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 255: ``Mul(Sqrt(P1)×4, Log(h1)×34, polynomial..., bounded...)`` numerator.
    s4l34p_x2 = _four_sqrt_thirty_four_log_poly_effective_x2(num, k)
    if s4l34p_x2 is not None:
        den_deg_s4l34 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l34 is not None:
            if 2 * den_deg_s4l34 > s4l34p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 254: ``Mul(Sqrt(P1)×3, Log(h1)×34, polynomial..., bounded...)`` numerator.
    s3l34p_x2 = _three_sqrt_thirty_four_log_poly_effective_x2(num, k)
    if s3l34p_x2 is not None:
        den_deg_s3l34 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l34 is not None:
            if 2 * den_deg_s3l34 > s3l34p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 253: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×34, polynomial..., bounded...)`` numerator.
    s2l34p_x2 = _two_sqrt_thirty_four_log_poly_effective_x2(num, k)
    if s2l34p_x2 is not None:
        den_deg_s2l34 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l34 is not None:
            if 2 * den_deg_s2l34 > s2l34p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 252: ``Mul(Sqrt(P), Log(h1)×34, polynomial..., bounded...)`` numerator.
    s1l34p_x2 = _one_sqrt_thirty_four_log_poly_effective_x2(num, k)
    if s1l34p_x2 is not None:
        den_deg_s1l34 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l34 is not None:
            if 2 * den_deg_s1l34 > s1l34p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 251: ``Mul(Log(h1)×34, polynomial..., bounded...)`` numerator.
    sl34p_x2 = _thirty_four_log_poly_effective_x2(num, k)
    if sl34p_x2 is not None:
        den_deg_sl34 = _polynomial_degree_in_k(den, k)
        if den_deg_sl34 is not None:
            if 2 * den_deg_sl34 > sl34p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 250: ``Mul(Sqrt(P1)×5, Log(h1)×33, polynomial..., bounded...)`` numerator.
    s5l33p_x2 = _five_sqrt_thirty_three_log_poly_effective_x2(num, k)
    if s5l33p_x2 is not None:
        den_deg_s5l33 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l33 is not None:
            if 2 * den_deg_s5l33 > s5l33p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 249: ``Mul(Sqrt(P1)×4, Log(h1)×33, polynomial..., bounded...)`` numerator.
    s4l33p_x2 = _four_sqrt_thirty_three_log_poly_effective_x2(num, k)
    if s4l33p_x2 is not None:
        den_deg_s4l33 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l33 is not None:
            if 2 * den_deg_s4l33 > s4l33p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 248: ``Mul(Sqrt(P1)×3, Log(h1)×33, polynomial..., bounded...)`` numerator.
    s3l33p_x2 = _three_sqrt_thirty_three_log_poly_effective_x2(num, k)
    if s3l33p_x2 is not None:
        den_deg_s3l33 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l33 is not None:
            if 2 * den_deg_s3l33 > s3l33p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 247: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×33, polynomial..., bounded...)`` numerator.
    s2l33p_x2 = _two_sqrt_thirty_three_log_poly_effective_x2(num, k)
    if s2l33p_x2 is not None:
        den_deg_s2l33 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l33 is not None:
            if 2 * den_deg_s2l33 > s2l33p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 246: ``Mul(Sqrt(P), Log(h1)×33, polynomial..., bounded...)`` numerator.
    s1l33p_x2 = _one_sqrt_thirty_three_log_poly_effective_x2(num, k)
    if s1l33p_x2 is not None:
        den_deg_s1l33 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l33 is not None:
            if 2 * den_deg_s1l33 > s1l33p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 245: ``Mul(Log(h1)×33, polynomial..., bounded...)`` numerator.
    sl33p_x2 = _thirty_three_log_poly_effective_x2(num, k)
    if sl33p_x2 is not None:
        den_deg_sl33 = _polynomial_degree_in_k(den, k)
        if den_deg_sl33 is not None:
            if 2 * den_deg_sl33 > sl33p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 244: ``Mul(Sqrt(P1)×5, Log(h1)×32, polynomial..., bounded...)`` numerator.
    s5l32p_x2 = _five_sqrt_thirty_two_log_poly_effective_x2(num, k)
    if s5l32p_x2 is not None:
        den_deg_s5l32 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l32 is not None:
            if 2 * den_deg_s5l32 > s5l32p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 243: ``Mul(Sqrt(P1)×4, Log(h1)×32, polynomial..., bounded...)`` numerator.
    s4l32p_x2 = _four_sqrt_thirty_two_log_poly_effective_x2(num, k)
    if s4l32p_x2 is not None:
        den_deg_s4l32 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l32 is not None:
            if 2 * den_deg_s4l32 > s4l32p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 242: ``Mul(Sqrt(P1)×3, Log(h1)×32, polynomial..., bounded...)`` numerator.
    s3l32p_x2 = _three_sqrt_thirty_two_log_poly_effective_x2(num, k)
    if s3l32p_x2 is not None:
        den_deg_s3l32 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l32 is not None:
            if 2 * den_deg_s3l32 > s3l32p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 241: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×32, polynomial..., bounded...)`` numerator.
    s2l32p_x2 = _two_sqrt_thirty_two_log_poly_effective_x2(num, k)
    if s2l32p_x2 is not None:
        den_deg_s2l32 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l32 is not None:
            if 2 * den_deg_s2l32 > s2l32p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 240: ``Mul(Sqrt(P), Log(h1)×32, polynomial..., bounded...)`` numerator.
    s1l32p_x2 = _one_sqrt_thirty_two_log_poly_effective_x2(num, k)
    if s1l32p_x2 is not None:
        den_deg_s1l32 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l32 is not None:
            if 2 * den_deg_s1l32 > s1l32p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 239: ``Mul(Log(h1)×32, polynomial..., bounded...)`` numerator.
    sl32p_x2 = _thirty_two_log_poly_effective_x2(num, k)
    if sl32p_x2 is not None:
        den_deg_sl32 = _polynomial_degree_in_k(den, k)
        if den_deg_sl32 is not None:
            if 2 * den_deg_sl32 > sl32p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 238: ``Mul(Sqrt(P1)×5, Log(h1)×31, polynomial..., bounded...)`` numerator.
    s5l31p_x2 = _five_sqrt_thirty_one_log_poly_effective_x2(num, k)
    if s5l31p_x2 is not None:
        den_deg_s5l31 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l31 is not None:
            if 2 * den_deg_s5l31 > s5l31p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 237: ``Mul(Sqrt(P1)×4, Log(h1)×31, polynomial..., bounded...)`` numerator.
    s4l31p_x2 = _four_sqrt_thirty_one_log_poly_effective_x2(num, k)
    if s4l31p_x2 is not None:
        den_deg_s4l31 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l31 is not None:
            if 2 * den_deg_s4l31 > s4l31p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 236: ``Mul(Sqrt(P1)×3, Log(h1)×31, polynomial..., bounded...)`` numerator.
    s3l31p_x2 = _three_sqrt_thirty_one_log_poly_effective_x2(num, k)
    if s3l31p_x2 is not None:
        den_deg_s3l31 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l31 is not None:
            if 2 * den_deg_s3l31 > s3l31p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 235: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×31, polynomial..., bounded...)`` numerator.
    s2l31p_x2 = _two_sqrt_thirty_one_log_poly_effective_x2(num, k)
    if s2l31p_x2 is not None:
        den_deg_s2l31 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l31 is not None:
            if 2 * den_deg_s2l31 > s2l31p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 234: ``Mul(Sqrt(P), Log(h1)×31, polynomial..., bounded...)`` numerator.
    s1l31p_x2 = _one_sqrt_thirty_one_log_poly_effective_x2(num, k)
    if s1l31p_x2 is not None:
        den_deg_s1l31 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l31 is not None:
            if 2 * den_deg_s1l31 > s1l31p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 233: ``Mul(Log(h1)×31, polynomial..., bounded...)`` numerator.
    sl31p_x2 = _thirty_one_log_poly_effective_x2(num, k)
    if sl31p_x2 is not None:
        den_deg_sl31 = _polynomial_degree_in_k(den, k)
        if den_deg_sl31 is not None:
            if 2 * den_deg_sl31 > sl31p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 232: ``Mul(Sqrt(P1)×5, Log(h1)×30, polynomial..., bounded...)`` numerator.
    s5l30p_x2 = _five_sqrt_thirty_log_poly_effective_x2(num, k)
    if s5l30p_x2 is not None:
        den_deg_s5l30 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l30 is not None:
            if 2 * den_deg_s5l30 > s5l30p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 231: ``Mul(Sqrt(P1)×4, Log(h1)×30, polynomial..., bounded...)`` numerator.
    s4l30p_x2 = _four_sqrt_thirty_log_poly_effective_x2(num, k)
    if s4l30p_x2 is not None:
        den_deg_s4l30 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l30 is not None:
            if 2 * den_deg_s4l30 > s4l30p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 230: ``Mul(Sqrt(P1)×3, Log(h1)×30, polynomial..., bounded...)`` numerator.
    s3l30p_x2 = _three_sqrt_thirty_log_poly_effective_x2(num, k)
    if s3l30p_x2 is not None:
        den_deg_s3l30 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l30 is not None:
            if 2 * den_deg_s3l30 > s3l30p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 229: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×30, polynomial..., bounded...)`` numerator.
    s2l30p_x2 = _two_sqrt_thirty_log_poly_effective_x2(num, k)
    if s2l30p_x2 is not None:
        den_deg_s2l30 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l30 is not None:
            if 2 * den_deg_s2l30 > s2l30p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 228: ``Mul(Sqrt(P), Log(h1)×30, polynomial..., bounded...)`` numerator.
    s1l30p_x2 = _one_sqrt_thirty_log_poly_effective_x2(num, k)
    if s1l30p_x2 is not None:
        den_deg_s1l30 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l30 is not None:
            if 2 * den_deg_s1l30 > s1l30p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 227: ``Mul(Log(h1)×30, polynomial..., bounded...)`` numerator.
    sl30p_x2 = _thirty_log_poly_effective_x2(num, k)
    if sl30p_x2 is not None:
        den_deg_sl30 = _polynomial_degree_in_k(den, k)
        if den_deg_sl30 is not None:
            if 2 * den_deg_sl30 > sl30p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 226: ``Mul(Sqrt(P1)×5, Log(h1)×29, polynomial..., bounded...)`` numerator.
    s5l29p_x2 = _five_sqrt_twenty_nine_log_poly_effective_x2(num, k)
    if s5l29p_x2 is not None:
        den_deg_s5l29 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l29 is not None:
            if 2 * den_deg_s5l29 > s5l29p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 225: ``Mul(Sqrt(P1)×4, Log(h1)×29, polynomial..., bounded...)`` numerator.
    s4l29p_x2 = _four_sqrt_twenty_nine_log_poly_effective_x2(num, k)
    if s4l29p_x2 is not None:
        den_deg_s4l29 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l29 is not None:
            if 2 * den_deg_s4l29 > s4l29p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 224: ``Mul(Sqrt(P1)×3, Log(h1)×29, polynomial..., bounded...)`` numerator.
    s3l29p_x2 = _three_sqrt_twenty_nine_log_poly_effective_x2(num, k)
    if s3l29p_x2 is not None:
        den_deg_s3l29 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l29 is not None:
            if 2 * den_deg_s3l29 > s3l29p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 223: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×29, polynomial..., bounded...)`` numerator.
    s2l29p_x2 = _two_sqrt_twenty_nine_log_poly_effective_x2(num, k)
    if s2l29p_x2 is not None:
        den_deg_s2l29 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l29 is not None:
            if 2 * den_deg_s2l29 > s2l29p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 222: ``Mul(Sqrt(P), Log(h1)×29, polynomial..., bounded...)`` numerator.
    s1l29p_x2 = _one_sqrt_twenty_nine_log_poly_effective_x2(num, k)
    if s1l29p_x2 is not None:
        den_deg_s1l29 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l29 is not None:
            if 2 * den_deg_s1l29 > s1l29p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 221: ``Mul(Log(h1)×29, polynomial..., bounded...)`` numerator.
    sl29p_x2 = _twenty_nine_log_poly_effective_x2(num, k)
    if sl29p_x2 is not None:
        den_deg_sl29 = _polynomial_degree_in_k(den, k)
        if den_deg_sl29 is not None:
            if 2 * den_deg_sl29 > sl29p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 220: ``Mul(Sqrt(P1)×5, Log(h1)×28, polynomial..., bounded...)`` numerator.
    s5l28p_x2 = _five_sqrt_twenty_eight_log_poly_effective_x2(num, k)
    if s5l28p_x2 is not None:
        den_deg_s5l28 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l28 is not None:
            if 2 * den_deg_s5l28 > s5l28p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 219: ``Mul(Sqrt(P1)×4, Log(h1)×28, polynomial..., bounded...)`` numerator.
    s4l28p_x2 = _four_sqrt_twenty_eight_log_poly_effective_x2(num, k)
    if s4l28p_x2 is not None:
        den_deg_s4l28 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l28 is not None:
            if 2 * den_deg_s4l28 > s4l28p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 218: ``Mul(Sqrt(P1)×3, Log(h1)×28, polynomial..., bounded...)`` numerator.
    s3l28p_x2 = _three_sqrt_twenty_eight_log_poly_effective_x2(num, k)
    if s3l28p_x2 is not None:
        den_deg_s3l28 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l28 is not None:
            if 2 * den_deg_s3l28 > s3l28p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 217: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×28, polynomial..., bounded...)`` numerator.
    s2l28p_x2 = _two_sqrt_twenty_eight_log_poly_effective_x2(num, k)
    if s2l28p_x2 is not None:
        den_deg_s2l28 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l28 is not None:
            if 2 * den_deg_s2l28 > s2l28p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 216: ``Mul(Sqrt(P), Log(h1)×28, polynomial..., bounded...)`` numerator.
    s1l28p_x2 = _one_sqrt_twenty_eight_log_poly_effective_x2(num, k)
    if s1l28p_x2 is not None:
        den_deg_s1l28 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l28 is not None:
            if 2 * den_deg_s1l28 > s1l28p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 215: ``Mul(Log(h1)×28, polynomial..., bounded...)`` numerator.
    sl28p_x2 = _twenty_eight_log_poly_effective_x2(num, k)
    if sl28p_x2 is not None:
        den_deg_sl28 = _polynomial_degree_in_k(den, k)
        if den_deg_sl28 is not None:
            if 2 * den_deg_sl28 > sl28p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 214: ``Mul(Sqrt(P1)×5, Log(h1)×27, polynomial..., bounded...)`` numerator.
    s5l27p_x2 = _five_sqrt_twenty_seven_log_poly_effective_x2(num, k)
    if s5l27p_x2 is not None:
        den_deg_s5l27 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l27 is not None:
            if 2 * den_deg_s5l27 > s5l27p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 213: ``Mul(Sqrt(P1)×4, Log(h1)×27, polynomial..., bounded...)`` numerator.
    s4l27p_x2 = _four_sqrt_twenty_seven_log_poly_effective_x2(num, k)
    if s4l27p_x2 is not None:
        den_deg_s4l27 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l27 is not None:
            if 2 * den_deg_s4l27 > s4l27p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 212: ``Mul(Sqrt(P1)×3, Log(h1)×27, polynomial..., bounded...)`` numerator.
    s3l27p_x2 = _three_sqrt_twenty_seven_log_poly_effective_x2(num, k)
    if s3l27p_x2 is not None:
        den_deg_s3l27 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l27 is not None:
            if 2 * den_deg_s3l27 > s3l27p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 211: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×27, polynomial..., bounded...)`` numerator.
    s2l27p_x2 = _two_sqrt_twenty_seven_log_poly_effective_x2(num, k)
    if s2l27p_x2 is not None:
        den_deg_s2l27 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l27 is not None:
            if 2 * den_deg_s2l27 > s2l27p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 210: ``Mul(Sqrt(P), Log(h1)×27, polynomial..., bounded...)`` numerator.
    s1l27p_x2 = _one_sqrt_twenty_seven_log_poly_effective_x2(num, k)
    if s1l27p_x2 is not None:
        den_deg_s1l27 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l27 is not None:
            if 2 * den_deg_s1l27 > s1l27p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 209: ``Mul(Log(h1)×27, polynomial..., bounded...)`` numerator.
    sl27p_x2 = _twenty_seven_log_poly_effective_x2(num, k)
    if sl27p_x2 is not None:
        den_deg_sl27 = _polynomial_degree_in_k(den, k)
        if den_deg_sl27 is not None:
            if 2 * den_deg_sl27 > sl27p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 208: ``Mul(Sqrt(P1)×5, Log(h1)×26, polynomial..., bounded...)`` numerator.
    s5l26p_x2 = _five_sqrt_twenty_six_log_poly_effective_x2(num, k)
    if s5l26p_x2 is not None:
        den_deg_s5l26 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l26 is not None:
            if 2 * den_deg_s5l26 > s5l26p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 207: ``Mul(Sqrt(P1)×4, Log(h1)×26, polynomial..., bounded...)`` numerator.
    s4l26p_x2 = _four_sqrt_twenty_six_log_poly_effective_x2(num, k)
    if s4l26p_x2 is not None:
        den_deg_s4l26 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l26 is not None:
            if 2 * den_deg_s4l26 > s4l26p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 206: ``Mul(Sqrt(P1)×3, Log(h1)×26, polynomial..., bounded...)`` numerator.
    s3l26p_x2 = _three_sqrt_twenty_six_log_poly_effective_x2(num, k)
    if s3l26p_x2 is not None:
        den_deg_s3l26 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l26 is not None:
            if 2 * den_deg_s3l26 > s3l26p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 205: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×26, polynomial..., bounded...)`` numerator.
    s2l26p_x2 = _two_sqrt_twenty_six_log_poly_effective_x2(num, k)
    if s2l26p_x2 is not None:
        den_deg_s2l26 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l26 is not None:
            if 2 * den_deg_s2l26 > s2l26p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 204: ``Mul(Sqrt(P), Log(h1)×26, polynomial..., bounded...)`` numerator.
    s1l26p_x2 = _one_sqrt_twenty_six_log_poly_effective_x2(num, k)
    if s1l26p_x2 is not None:
        den_deg_s1l26 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l26 is not None:
            if 2 * den_deg_s1l26 > s1l26p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 203: ``Mul(Log(h1)×26, polynomial..., bounded...)`` numerator.
    sl26p_x2 = _twenty_six_log_poly_effective_x2(num, k)
    if sl26p_x2 is not None:
        den_deg_sl26 = _polynomial_degree_in_k(den, k)
        if den_deg_sl26 is not None:
            if 2 * den_deg_sl26 > sl26p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 202: ``Mul(Sqrt(P1)×5, Log(h1)×25, polynomial..., bounded...)`` numerator.
    s5l25p_x2 = _five_sqrt_twenty_five_log_poly_effective_x2(num, k)
    if s5l25p_x2 is not None:
        den_deg_s5l25 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l25 is not None:
            if 2 * den_deg_s5l25 > s5l25p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 201: ``Mul(Sqrt(P1)×4, Log(h1)×25, polynomial..., bounded...)`` numerator.
    s4l25p_x2 = _four_sqrt_twenty_five_log_poly_effective_x2(num, k)
    if s4l25p_x2 is not None:
        den_deg_s4l25 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l25 is not None:
            if 2 * den_deg_s4l25 > s4l25p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 200: ``Mul(Sqrt(P1)×3, Log(h1)×25, polynomial..., bounded...)`` numerator.
    s3l25p_x2 = _three_sqrt_twenty_five_log_poly_effective_x2(num, k)
    if s3l25p_x2 is not None:
        den_deg_s3l25 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l25 is not None:
            if 2 * den_deg_s3l25 > s3l25p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 199: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×25, polynomial..., bounded...)`` numerator.
    s2l25p_x2 = _two_sqrt_twenty_five_log_poly_effective_x2(num, k)
    if s2l25p_x2 is not None:
        den_deg_s2l25 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l25 is not None:
            if 2 * den_deg_s2l25 > s2l25p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 198: ``Mul(Sqrt(P), Log(h1)×25, polynomial..., bounded...)`` numerator.
    s1l25p_x2 = _one_sqrt_twenty_five_log_poly_effective_x2(num, k)
    if s1l25p_x2 is not None:
        den_deg_s1l25 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l25 is not None:
            if 2 * den_deg_s1l25 > s1l25p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 197: ``Mul(Log(h1)×25, polynomial..., bounded...)`` numerator.
    sl25p_x2 = _twenty_five_log_poly_effective_x2(num, k)
    if sl25p_x2 is not None:
        den_deg_sl25 = _polynomial_degree_in_k(den, k)
        if den_deg_sl25 is not None:
            if 2 * den_deg_sl25 > sl25p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 196: ``Mul(Sqrt(P1)×5, Log(h1)×24, polynomial..., bounded...)`` numerator.
    s5l24p_x2 = _five_sqrt_twenty_four_log_poly_effective_x2(num, k)
    if s5l24p_x2 is not None:
        den_deg_s5l24 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l24 is not None:
            if 2 * den_deg_s5l24 > s5l24p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 195: ``Mul(Sqrt(P1)×4, Log(h1)×24, polynomial..., bounded...)`` numerator.
    s4l24p_x2 = _four_sqrt_twenty_four_log_poly_effective_x2(num, k)
    if s4l24p_x2 is not None:
        den_deg_s4l24 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l24 is not None:
            if 2 * den_deg_s4l24 > s4l24p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 194: ``Mul(Sqrt(P1)×3, Log(h1)×24, polynomial..., bounded...)`` numerator.
    s3l24p_x2 = _three_sqrt_twenty_four_log_poly_effective_x2(num, k)
    if s3l24p_x2 is not None:
        den_deg_s3l24 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l24 is not None:
            if 2 * den_deg_s3l24 > s3l24p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 193: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×24, polynomial..., bounded...)`` numerator.
    s2l24p_x2 = _two_sqrt_twenty_four_log_poly_effective_x2(num, k)
    if s2l24p_x2 is not None:
        den_deg_s2l24 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l24 is not None:
            if 2 * den_deg_s2l24 > s2l24p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 192: ``Mul(Sqrt(P), Log(h1)×24, polynomial..., bounded...)`` numerator.
    s1l24p_x2 = _one_sqrt_twenty_four_log_poly_effective_x2(num, k)
    if s1l24p_x2 is not None:
        den_deg_s1l24 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l24 is not None:
            if 2 * den_deg_s1l24 > s1l24p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 191: ``Mul(Log(h1)×24, polynomial..., bounded...)`` numerator.
    sl24p_x2 = _twenty_four_log_poly_effective_x2(num, k)
    if sl24p_x2 is not None:
        den_deg_sl24 = _polynomial_degree_in_k(den, k)
        if den_deg_sl24 is not None:
            if 2 * den_deg_sl24 > sl24p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 190: ``Mul(Sqrt(P1)×5, Log(h1)×23, polynomial..., bounded...)`` numerator.
    s5l23p_x2 = _five_sqrt_twenty_three_log_poly_effective_x2(num, k)
    if s5l23p_x2 is not None:
        den_deg_s5l23 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l23 is not None:
            if 2 * den_deg_s5l23 > s5l23p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 189: ``Mul(Sqrt(P1)×4, Log(h1)×23, polynomial..., bounded...)`` numerator.
    s4l23p_x2 = _four_sqrt_twenty_three_log_poly_effective_x2(num, k)
    if s4l23p_x2 is not None:
        den_deg_s4l23 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l23 is not None:
            if 2 * den_deg_s4l23 > s4l23p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 188: ``Mul(Sqrt(P1)×3, Log(h1)×23, polynomial..., bounded...)`` numerator.
    s3l23p_x2 = _three_sqrt_twenty_three_log_poly_effective_x2(num, k)
    if s3l23p_x2 is not None:
        den_deg_s3l23 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l23 is not None:
            if 2 * den_deg_s3l23 > s3l23p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 187: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×23, polynomial..., bounded...)`` numerator.
    s2l23p_x2 = _two_sqrt_twenty_three_log_poly_effective_x2(num, k)
    if s2l23p_x2 is not None:
        den_deg_s2l23 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l23 is not None:
            if 2 * den_deg_s2l23 > s2l23p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 186: ``Mul(Sqrt(P), Log(h1)×23, polynomial..., bounded...)`` numerator.
    s1l23p_x2 = _one_sqrt_twenty_three_log_poly_effective_x2(num, k)
    if s1l23p_x2 is not None:
        den_deg_s1l23 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l23 is not None:
            if 2 * den_deg_s1l23 > s1l23p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 185: ``Mul(Log(h1)×23, polynomial..., bounded...)`` numerator.
    sl23p_x2 = _twenty_three_log_poly_effective_x2(num, k)
    if sl23p_x2 is not None:
        den_deg_sl23 = _polynomial_degree_in_k(den, k)
        if den_deg_sl23 is not None:
            if 2 * den_deg_sl23 > sl23p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 184: ``Mul(Sqrt(P1)×5, Log(h1)×22, polynomial..., bounded...)`` numerator.
    # Five Sqrt + twenty-two Log factors; effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    s5l22p_x2 = _five_sqrt_twenty_two_log_poly_effective_x2(num, k)
    if s5l22p_x2 is not None:
        den_deg_s5l22 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l22 is not None:
            if 2 * den_deg_s5l22 > s5l22p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 183: ``Mul(Sqrt(P1)×4, Log(h1)×22, polynomial..., bounded...)`` numerator.
    s4l22p_x2 = _four_sqrt_twenty_two_log_poly_effective_x2(num, k)
    if s4l22p_x2 is not None:
        den_deg_s4l22 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l22 is not None:
            if 2 * den_deg_s4l22 > s4l22p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 182: ``Mul(Sqrt(P1)×3, Log(h1)×22, polynomial..., bounded...)`` numerator.
    s3l22p_x2 = _three_sqrt_twenty_two_log_poly_effective_x2(num, k)
    if s3l22p_x2 is not None:
        den_deg_s3l22 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l22 is not None:
            if 2 * den_deg_s3l22 > s3l22p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 181: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×22, polynomial..., bounded...)`` numerator.
    s2l22p_x2 = _two_sqrt_twenty_two_log_poly_effective_x2(num, k)
    if s2l22p_x2 is not None:
        den_deg_s2l22 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l22 is not None:
            if 2 * den_deg_s2l22 > s2l22p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 180: ``Mul(Sqrt(P), Log(h1)×22, polynomial..., bounded...)`` numerator.
    s1l22p_x2 = _one_sqrt_twenty_two_log_poly_effective_x2(num, k)
    if s1l22p_x2 is not None:
        den_deg_s1l22 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l22 is not None:
            if 2 * den_deg_s1l22 > s1l22p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 179: ``Mul(Log(h1)×22, polynomial..., bounded...)`` numerator.
    sl22p_x2 = _twenty_two_log_poly_effective_x2(num, k)
    if sl22p_x2 is not None:
        den_deg_sl22 = _polynomial_degree_in_k(den, k)
        if den_deg_sl22 is not None:
            if 2 * den_deg_sl22 > sl22p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 178: ``Mul(Sqrt(P1)×5, Log(h1)×21, polynomial..., bounded...)`` numerator.
    # Five Sqrt factors + twenty-one Log factors; log²¹ sub-polynomial → 0.
    # effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l21p_x2 = _five_sqrt_twenty_one_log_poly_effective_x2(num, k)
    if s5l21p_x2 is not None:
        den_deg_s5l21 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l21 is not None:
            if 2 * den_deg_s5l21 > s5l21p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 177: ``Mul(Sqrt(P1)×4, Log(h1)×21, polynomial..., bounded...)`` numerator.
    s4l21p_x2 = _four_sqrt_twenty_one_log_poly_effective_x2(num, k)
    if s4l21p_x2 is not None:
        den_deg_s4l21 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l21 is not None:
            if 2 * den_deg_s4l21 > s4l21p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 176: ``Mul(Sqrt(P1)×3, Log(h1)×21, polynomial..., bounded...)`` numerator.
    s3l21p_x2 = _three_sqrt_twenty_one_log_poly_effective_x2(num, k)
    if s3l21p_x2 is not None:
        den_deg_s3l21 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l21 is not None:
            if 2 * den_deg_s3l21 > s3l21p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 175: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×21, polynomial..., bounded...)`` numerator.
    s2l21p_x2 = _two_sqrt_twenty_one_log_poly_effective_x2(num, k)
    if s2l21p_x2 is not None:
        den_deg_s2l21 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l21 is not None:
            if 2 * den_deg_s2l21 > s2l21p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 174: ``Mul(Sqrt(P), Log(h1)×21, polynomial..., bounded...)`` numerator.
    s1l21p_x2 = _one_sqrt_twenty_one_log_poly_effective_x2(num, k)
    if s1l21p_x2 is not None:
        den_deg_s1l21 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l21 is not None:
            if 2 * den_deg_s1l21 > s1l21p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 173: ``Mul(Log(h1)×21, polynomial..., bounded...)`` numerator.
    sl21p_x2 = _twenty_one_log_poly_effective_x2(num, k)
    if sl21p_x2 is not None:
        den_deg_sl21 = _polynomial_degree_in_k(den, k)
        if den_deg_sl21 is not None:
            if 2 * den_deg_sl21 > sl21p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 172: ``Mul(Sqrt(P1)×5, Log(h1)×20, polynomial..., bounded...)`` numerator.
    # Five Sqrt factors + twenty Log factors; log²⁰ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + sqrt5_deg_x2
    #              + 2·poly_deg.
    # Vanishes when ``2·den_deg > effective_x2`` (polynomial) or
    # non-polynomial diverging denominator.
    s5l20p_x2 = _five_sqrt_twenty_log_poly_effective_x2(num, k)
    if s5l20p_x2 is not None:
        den_deg_s5l20 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l20 is not None:
            if 2 * den_deg_s5l20 > s5l20p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 171: ``Mul(Sqrt(P1)×4, Log(h1)×20, polynomial..., bounded...)`` numerator.
    # Four Sqrt factors + twenty Log factors; log²⁰ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + 2·poly_deg.
    s4l20p_x2 = _four_sqrt_twenty_log_poly_effective_x2(num, k)
    if s4l20p_x2 is not None:
        den_deg_s4l20 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l20 is not None:
            if 2 * den_deg_s4l20 > s4l20p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 170: ``Mul(Sqrt(P1)×3, Log(h1)×20, polynomial..., bounded...)`` numerator.
    # Three Sqrt factors + twenty Log factors; log²⁰ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg.
    s3l20p_x2 = _three_sqrt_twenty_log_poly_effective_x2(num, k)
    if s3l20p_x2 is not None:
        den_deg_s3l20 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l20 is not None:
            if 2 * den_deg_s3l20 > s3l20p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 169: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×20, polynomial..., bounded...)`` numerator.
    # Two Sqrt factors + twenty Log factors; log²⁰ sub-polynomial → 0.
    # effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg.
    s2l20p_x2 = _two_sqrt_twenty_log_poly_effective_x2(num, k)
    if s2l20p_x2 is not None:
        den_deg_s2l20 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l20 is not None:
            if 2 * den_deg_s2l20 > s2l20p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 168: ``Mul(Sqrt(P), Log(h1)×20, polynomial..., bounded...)`` numerator.
    # One Sqrt factor + twenty Log factors; log²⁰ sub-polynomial → 0.
    # effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    s1l20p_x2 = _one_sqrt_twenty_log_poly_effective_x2(num, k)
    if s1l20p_x2 is not None:
        den_deg_s1l20 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l20 is not None:
            if 2 * den_deg_s1l20 > s1l20p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 167: ``Mul(Log(h1)×20, polynomial..., bounded...)`` numerator.
    # Zero Sqrt factors + twenty Log factors; log²⁰ sub-polynomial → 0.
    # effective_x2 = 2·poly_deg.
    sl20p_x2 = _twenty_log_poly_effective_x2(num, k)
    if sl20p_x2 is not None:
        den_deg_sl20 = _polynomial_degree_in_k(den, k)
        if den_deg_sl20 is not None:
            if 2 * den_deg_sl20 > sl20p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 166: ``Mul(Sqrt(P1)×5, Log(h1)×19, polynomial..., bounded...)`` numerator.
    # Five Sqrt + nineteen Log factors; log^19 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l19p_x2 = _five_sqrt_nineteen_log_poly_effective_x2(num, k)
    if s5l19p_x2 is not None:
        den_deg_s5l19 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l19 is not None:
            if 2 * den_deg_s5l19 > s5l19p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 165: ``Mul(Sqrt(P1)×4, Log(h1)×19, polynomial..., bounded...)`` numerator.
    s4l19p_x2 = _four_sqrt_nineteen_log_poly_effective_x2(num, k)
    if s4l19p_x2 is not None:
        den_deg_s4l19 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l19 is not None:
            if 2 * den_deg_s4l19 > s4l19p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 164: ``Mul(Sqrt(P1)×3, Log(h1)×19, polynomial..., bounded...)`` numerator.
    s3l19p_x2 = _three_sqrt_nineteen_log_poly_effective_x2(num, k)
    if s3l19p_x2 is not None:
        den_deg_s3l19 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l19 is not None:
            if 2 * den_deg_s3l19 > s3l19p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 163: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×19, polynomial..., bounded...)`` numerator.
    s2l19p_x2 = _two_sqrt_nineteen_log_poly_effective_x2(num, k)
    if s2l19p_x2 is not None:
        den_deg_s2l19 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l19 is not None:
            if 2 * den_deg_s2l19 > s2l19p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 162: ``Mul(Sqrt(P), Log(h1)×19, polynomial..., bounded...)`` numerator.
    s1l19p_x2 = _one_sqrt_nineteen_log_poly_effective_x2(num, k)
    if s1l19p_x2 is not None:
        den_deg_s1l19 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l19 is not None:
            if 2 * den_deg_s1l19 > s1l19p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 161: ``Mul(Log(h1)×19, polynomial..., bounded...)`` numerator.
    sl19p_x2 = _nineteen_log_poly_effective_x2(num, k)
    if sl19p_x2 is not None:
        den_deg_sl19 = _polynomial_degree_in_k(den, k)
        if den_deg_sl19 is not None:
            if 2 * den_deg_sl19 > sl19p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 160: ``Mul(Sqrt(P1)×5, Log(h1)×18, polynomial..., bounded...)`` numerator.
    # Five Sqrt + eighteen Log factors; log^18 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l18p_x2 = _five_sqrt_eighteen_log_poly_effective_x2(num, k)
    if s5l18p_x2 is not None:
        den_deg_s5l18 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l18 is not None:
            if 2 * den_deg_s5l18 > s5l18p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 159: ``Mul(Sqrt(P1)×4, Log(h1)×18, polynomial..., bounded...)`` numerator.
    s4l18p_x2 = _four_sqrt_eighteen_log_poly_effective_x2(num, k)
    if s4l18p_x2 is not None:
        den_deg_s4l18 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l18 is not None:
            if 2 * den_deg_s4l18 > s4l18p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 158: ``Mul(Sqrt(P1)×3, Log(h1)×18, polynomial..., bounded...)`` numerator.
    s3l18p_x2 = _three_sqrt_eighteen_log_poly_effective_x2(num, k)
    if s3l18p_x2 is not None:
        den_deg_s3l18 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l18 is not None:
            if 2 * den_deg_s3l18 > s3l18p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 157: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×18, polynomial..., bounded...)`` numerator.
    s2l18p_x2 = _two_sqrt_eighteen_log_poly_effective_x2(num, k)
    if s2l18p_x2 is not None:
        den_deg_s2l18 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l18 is not None:
            if 2 * den_deg_s2l18 > s2l18p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 156: ``Mul(Sqrt(P), Log(h1)×18, polynomial..., bounded...)`` numerator.
    s1l18p_x2 = _one_sqrt_eighteen_log_poly_effective_x2(num, k)
    if s1l18p_x2 is not None:
        den_deg_s1l18 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l18 is not None:
            if 2 * den_deg_s1l18 > s1l18p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 155: ``Mul(Log(h1)×18, polynomial..., bounded...)`` numerator.
    sl18p_x2 = _eighteen_log_poly_effective_x2(num, k)
    if sl18p_x2 is not None:
        den_deg_sl18 = _polynomial_degree_in_k(den, k)
        if den_deg_sl18 is not None:
            if 2 * den_deg_sl18 > sl18p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 154: ``Mul(Sqrt(P1)×5, Log(h1)×17, polynomial..., bounded...)`` numerator.
    # Five Sqrt + seventeen Log factors; log^17 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l17p_x2 = _five_sqrt_seventeen_log_poly_effective_x2(num, k)
    if s5l17p_x2 is not None:
        den_deg_s5l17 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l17 is not None:
            if 2 * den_deg_s5l17 > s5l17p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 153: ``Mul(Sqrt(P1)×4, Log(h1)×17, polynomial..., bounded...)`` numerator.
    s4l17p_x2 = _four_sqrt_seventeen_log_poly_effective_x2(num, k)
    if s4l17p_x2 is not None:
        den_deg_s4l17 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l17 is not None:
            if 2 * den_deg_s4l17 > s4l17p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 152: ``Mul(Sqrt(P1)×3, Log(h1)×17, polynomial..., bounded...)`` numerator.
    s3l17p_x2 = _three_sqrt_seventeen_log_poly_effective_x2(num, k)
    if s3l17p_x2 is not None:
        den_deg_s3l17 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l17 is not None:
            if 2 * den_deg_s3l17 > s3l17p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 151: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×17, polynomial..., bounded...)`` numerator.
    s2l17p_x2 = _two_sqrt_seventeen_log_poly_effective_x2(num, k)
    if s2l17p_x2 is not None:
        den_deg_s2l17 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l17 is not None:
            if 2 * den_deg_s2l17 > s2l17p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 150: ``Mul(Sqrt(P), Log(h1)×17, polynomial..., bounded...)`` numerator.
    s1l17p_x2 = _one_sqrt_seventeen_log_poly_effective_x2(num, k)
    if s1l17p_x2 is not None:
        den_deg_s1l17 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l17 is not None:
            if 2 * den_deg_s1l17 > s1l17p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 149: ``Mul(Log(h1)×17, polynomial..., bounded...)`` numerator.
    sl17p_x2 = _seventeen_log_poly_effective_x2(num, k)
    if sl17p_x2 is not None:
        den_deg_sl17 = _polynomial_degree_in_k(den, k)
        if den_deg_sl17 is not None:
            if 2 * den_deg_sl17 > sl17p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 148: ``Mul(Sqrt(P1)×5, Log(h1)×16, polynomial..., bounded...)`` numerator.
    # Five Sqrt + sixteen Log factors; log^16 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l16p_x2 = _five_sqrt_sixteen_log_poly_effective_x2(num, k)
    if s5l16p_x2 is not None:
        den_deg_s5l16 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l16 is not None:
            if 2 * den_deg_s5l16 > s5l16p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 147: ``Mul(Sqrt(P1)×4, Log(h1)×16, polynomial..., bounded...)`` numerator.
    s4l16p_x2 = _four_sqrt_sixteen_log_poly_effective_x2(num, k)
    if s4l16p_x2 is not None:
        den_deg_s4l16 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l16 is not None:
            if 2 * den_deg_s4l16 > s4l16p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 146: ``Mul(Sqrt(P1)×3, Log(h1)×16, polynomial..., bounded...)`` numerator.
    s3l16p_x2 = _three_sqrt_sixteen_log_poly_effective_x2(num, k)
    if s3l16p_x2 is not None:
        den_deg_s3l16 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l16 is not None:
            if 2 * den_deg_s3l16 > s3l16p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 145: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×16, polynomial..., bounded...)`` numerator.
    s2l16p_x2 = _two_sqrt_sixteen_log_poly_effective_x2(num, k)
    if s2l16p_x2 is not None:
        den_deg_s2l16 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l16 is not None:
            if 2 * den_deg_s2l16 > s2l16p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 144: ``Mul(Sqrt(P), Log(h1)×16, polynomial..., bounded...)`` numerator.
    s1l16p_x2 = _one_sqrt_sixteen_log_poly_effective_x2(num, k)
    if s1l16p_x2 is not None:
        den_deg_s1l16 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l16 is not None:
            if 2 * den_deg_s1l16 > s1l16p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 143: ``Mul(Log(h1)×16, polynomial..., bounded...)`` numerator.
    sl16p_x2 = _sixteen_log_poly_effective_x2(num, k)
    if sl16p_x2 is not None:
        den_deg_sl16 = _polynomial_degree_in_k(den, k)
        if den_deg_sl16 is not None:
            if 2 * den_deg_sl16 > sl16p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 142: ``Mul(Sqrt(P1)×5, Log(h1)×15, polynomial..., bounded...)`` numerator.
    # Five Sqrt + fifteen Log factors; log^15 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l15p_x2 = _five_sqrt_fifteen_log_poly_effective_x2(num, k)
    if s5l15p_x2 is not None:
        den_deg_s5l15 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l15 is not None:
            if 2 * den_deg_s5l15 > s5l15p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 141: ``Mul(Sqrt(P1)×4, Log(h1)×15, polynomial..., bounded...)`` numerator.
    s4l15p_x2 = _four_sqrt_fifteen_log_poly_effective_x2(num, k)
    if s4l15p_x2 is not None:
        den_deg_s4l15 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l15 is not None:
            if 2 * den_deg_s4l15 > s4l15p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 140: ``Mul(Sqrt(P1)×3, Log(h1)×15, polynomial..., bounded...)`` numerator.
    s3l15p_x2 = _three_sqrt_fifteen_log_poly_effective_x2(num, k)
    if s3l15p_x2 is not None:
        den_deg_s3l15 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l15 is not None:
            if 2 * den_deg_s3l15 > s3l15p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 139: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×15, polynomial..., bounded...)`` numerator.
    s2l15p_x2 = _two_sqrt_fifteen_log_poly_effective_x2(num, k)
    if s2l15p_x2 is not None:
        den_deg_s2l15 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l15 is not None:
            if 2 * den_deg_s2l15 > s2l15p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 138: ``Mul(Sqrt(P), Log(h1)×15, polynomial..., bounded...)`` numerator.
    s1l15p_x2 = _one_sqrt_fifteen_log_poly_effective_x2(num, k)
    if s1l15p_x2 is not None:
        den_deg_s1l15 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l15 is not None:
            if 2 * den_deg_s1l15 > s1l15p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 137: ``Mul(Log(h1)×15, polynomial..., bounded...)`` numerator.
    sl15p_x2 = _fifteen_log_poly_effective_x2(num, k)
    if sl15p_x2 is not None:
        den_deg_sl15 = _polynomial_degree_in_k(den, k)
        if den_deg_sl15 is not None:
            if 2 * den_deg_sl15 > sl15p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 136: ``Mul(Sqrt(P1)×5, Log(h1)×14, polynomial..., bounded...)`` numerator.
    # Five Sqrt + fourteen Log factors; log^14 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l14p_x2 = _five_sqrt_fourteen_log_poly_effective_x2(num, k)
    if s5l14p_x2 is not None:
        den_deg_s5l14 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l14 is not None:
            if 2 * den_deg_s5l14 > s5l14p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 135: ``Mul(Sqrt(P1)×4, Log(h1)×14, polynomial..., bounded...)`` numerator.
    s4l14p_x2 = _four_sqrt_fourteen_log_poly_effective_x2(num, k)
    if s4l14p_x2 is not None:
        den_deg_s4l14 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l14 is not None:
            if 2 * den_deg_s4l14 > s4l14p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 134: ``Mul(Sqrt(P1)×3, Log(h1)×14, polynomial..., bounded...)`` numerator.
    s3l14p_x2 = _three_sqrt_fourteen_log_poly_effective_x2(num, k)
    if s3l14p_x2 is not None:
        den_deg_s3l14 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l14 is not None:
            if 2 * den_deg_s3l14 > s3l14p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 133: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×14, polynomial..., bounded...)`` numerator.
    s2l14p_x2 = _two_sqrt_fourteen_log_poly_effective_x2(num, k)
    if s2l14p_x2 is not None:
        den_deg_s2l14 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l14 is not None:
            if 2 * den_deg_s2l14 > s2l14p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 132: ``Mul(Sqrt(P), Log(h1)×14, polynomial..., bounded...)`` numerator.
    s1l14p_x2 = _one_sqrt_fourteen_log_poly_effective_x2(num, k)
    if s1l14p_x2 is not None:
        den_deg_s1l14 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l14 is not None:
            if 2 * den_deg_s1l14 > s1l14p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 131: ``Mul(Log(h1)×14, polynomial..., bounded...)`` numerator.
    sl14p_x2 = _fourteen_log_poly_effective_x2(num, k)
    if sl14p_x2 is not None:
        den_deg_sl14 = _polynomial_degree_in_k(den, k)
        if den_deg_sl14 is not None:
            if 2 * den_deg_sl14 > sl14p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 130: ``Mul(Sqrt(P1)×5, Log(h1)×13, polynomial..., bounded...)`` numerator.
    # Five Sqrt + thirteen Log factors; log^13 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l13p_x2 = _five_sqrt_thirteen_log_poly_effective_x2(num, k)
    if s5l13p_x2 is not None:
        den_deg_s5l13 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l13 is not None:
            if 2 * den_deg_s5l13 > s5l13p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 129: ``Mul(Sqrt(P1)×4, Log(h1)×13, polynomial..., bounded...)`` numerator.
    s4l13p_x2 = _four_sqrt_thirteen_log_poly_effective_x2(num, k)
    if s4l13p_x2 is not None:
        den_deg_s4l13 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l13 is not None:
            if 2 * den_deg_s4l13 > s4l13p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 128: ``Mul(Sqrt(P1)×3, Log(h1)×13, polynomial..., bounded...)`` numerator.
    s3l13p_x2 = _three_sqrt_thirteen_log_poly_effective_x2(num, k)
    if s3l13p_x2 is not None:
        den_deg_s3l13 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l13 is not None:
            if 2 * den_deg_s3l13 > s3l13p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 127: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×13, polynomial..., bounded...)`` numerator.
    s2l13p_x2 = _two_sqrt_thirteen_log_poly_effective_x2(num, k)
    if s2l13p_x2 is not None:
        den_deg_s2l13 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l13 is not None:
            if 2 * den_deg_s2l13 > s2l13p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 126: ``Mul(Sqrt(P), Log(h1)×13, polynomial..., bounded...)`` numerator.
    s1l13p_x2 = _one_sqrt_thirteen_log_poly_effective_x2(num, k)
    if s1l13p_x2 is not None:
        den_deg_s1l13 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l13 is not None:
            if 2 * den_deg_s1l13 > s1l13p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 125: ``Mul(Log(h1)×13, polynomial..., bounded...)`` numerator.
    sl13p_x2 = _thirteen_log_poly_effective_x2(num, k)
    if sl13p_x2 is not None:
        den_deg_sl13 = _polynomial_degree_in_k(den, k)
        if den_deg_sl13 is not None:
            if 2 * den_deg_sl13 > sl13p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 124: ``Mul(Sqrt(P1)×5, Log(h1)×12, polynomial..., bounded...)`` numerator.
    # Five Sqrt + twelve Log factors; log^12 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l12p_x2 = _five_sqrt_twelve_log_poly_effective_x2(num, k)
    if s5l12p_x2 is not None:
        den_deg_s5l12 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l12 is not None:
            if 2 * den_deg_s5l12 > s5l12p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 123: ``Mul(Sqrt(P1)×4, Log(h1)×12, polynomial..., bounded...)`` numerator.
    s4l12p_x2 = _four_sqrt_twelve_log_poly_effective_x2(num, k)
    if s4l12p_x2 is not None:
        den_deg_s4l12 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l12 is not None:
            if 2 * den_deg_s4l12 > s4l12p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 122: ``Mul(Sqrt(P1)×3, Log(h1)×12, polynomial..., bounded...)`` numerator.
    s3l12p_x2 = _three_sqrt_twelve_log_poly_effective_x2(num, k)
    if s3l12p_x2 is not None:
        den_deg_s3l12 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l12 is not None:
            if 2 * den_deg_s3l12 > s3l12p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 121: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×12, polynomial..., bounded...)`` numerator.
    s2l12p_x2 = _two_sqrt_twelve_log_poly_effective_x2(num, k)
    if s2l12p_x2 is not None:
        den_deg_s2l12 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l12 is not None:
            if 2 * den_deg_s2l12 > s2l12p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 120: ``Mul(Sqrt(P), Log(h1)×12, polynomial..., bounded...)`` numerator.
    s1l12p_x2 = _one_sqrt_twelve_log_poly_effective_x2(num, k)
    if s1l12p_x2 is not None:
        den_deg_s1l12 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l12 is not None:
            if 2 * den_deg_s1l12 > s1l12p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 119: ``Mul(Log(h1)×12, polynomial..., bounded...)`` numerator.
    sl12p_x2 = _twelve_log_poly_effective_x2(num, k)
    if sl12p_x2 is not None:
        den_deg_sl12 = _polynomial_degree_in_k(den, k)
        if den_deg_sl12 is not None:
            if 2 * den_deg_sl12 > sl12p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 118: ``Mul(Sqrt(P1)×5, Log(h1)×11, polynomial..., bounded...)`` numerator.
    # Five Sqrt + eleven Log factors; log^11 sub-polynomial → effective_x2 = sum(sqrt_degs_x2) + 2·poly_deg.
    # Closes when ``2·den_deg > effective_x2`` or non-polynomial diverging denominator.
    s5l11p_x2 = _five_sqrt_eleven_log_poly_effective_x2(num, k)
    if s5l11p_x2 is not None:
        den_deg_s5l11 = _polynomial_degree_in_k(den, k)
        if den_deg_s5l11 is not None:
            if 2 * den_deg_s5l11 > s5l11p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 117: ``Mul(Sqrt(P1)×4, Log(h1)×11, polynomial..., bounded...)`` numerator.
    s4l11p_x2 = _four_sqrt_eleven_log_poly_effective_x2(num, k)
    if s4l11p_x2 is not None:
        den_deg_s4l11 = _polynomial_degree_in_k(den, k)
        if den_deg_s4l11 is not None:
            if 2 * den_deg_s4l11 > s4l11p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 116: ``Mul(Sqrt(P1)×3, Log(h1)×11, polynomial..., bounded...)`` numerator.
    s3l11p_x2 = _three_sqrt_eleven_log_poly_effective_x2(num, k)
    if s3l11p_x2 is not None:
        den_deg_s3l11 = _polynomial_degree_in_k(den, k)
        if den_deg_s3l11 is not None:
            if 2 * den_deg_s3l11 > s3l11p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 115: ``Mul(Sqrt(P1), Sqrt(P2), Log(h1)×11, polynomial..., bounded...)`` numerator.
    s2l11p_x2 = _two_sqrt_eleven_log_poly_effective_x2(num, k)
    if s2l11p_x2 is not None:
        den_deg_s2l11 = _polynomial_degree_in_k(den, k)
        if den_deg_s2l11 is not None:
            if 2 * den_deg_s2l11 > s2l11p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 114: ``Mul(Sqrt(P), Log(h1)×11, polynomial..., bounded...)`` numerator.
    s1l11p_x2 = _one_sqrt_eleven_log_poly_effective_x2(num, k)
    if s1l11p_x2 is not None:
        den_deg_s1l11 = _polynomial_degree_in_k(den, k)
        if den_deg_s1l11 is not None:
            if 2 * den_deg_s1l11 > s1l11p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 113: ``Mul(Log(h1)×11, polynomial..., bounded...)`` numerator.
    sl11p_x2 = _eleven_log_poly_effective_x2(num, k)
    if sl11p_x2 is not None:
        den_deg_sl11 = _polynomial_degree_in_k(den, k)
        if den_deg_sl11 is not None:
            if 2 * den_deg_sl11 > sl11p_x2:
                return True
        elif _h_diverges_at_infinity(den, k):
            return True
    # Phase 42 widening: deg(num) < deg(den) on pure polynomials in k.
    num_degree = _polynomial_degree_in_k(num, k)
    if num_degree is None:
        return False
    den_degree = _polynomial_degree_in_k(den, k)
    if den_degree is None:
        return False
    return num_degree < den_degree



def _is_log_of_diverging_in_k(node: IRNode, k: IRSymbol) -> bool:
    """Return True when ``node = Log(h(k))`` with ``h(k) → +∞``.

    Phase 50 — used by :func:`_g_vanishes_at_infinity` to recognise
    that ``log(k)/k²``, ``log(k+1)/k``, ``log(2^k)/k³``, etc. all
    vanish at infinity.  The squeeze argument: ``log(h) → ∞`` at a
    logarithmic rate, while any positive-degree polynomial /
    exponential denominator grows strictly faster, so ``log/poly → 0``.

    Sign-aware: delegates the divergence check to
    :func:`_h_diverges_at_infinity` *on the entire `Log(...)` node*,
    which routes through Phase 44's Log branch.  That branch already
    refuses shapes like ``Log(Mul(-1, k))`` (negative leading
    coefficient — ``log(-k)`` isn't real for odd k), so Phase 50
    inherits the same conservatism for free.
    """
    if not isinstance(node, IRApply):
        return False
    if node.head != LOG or len(node.args) != 1:
        return False
    # Delegate to the full Log-aware divergence check.  This refuses
    # Log(negative-polynomial) shapes without us having to redo the
    # sign analysis.
    return _h_diverges_at_infinity(node, k)


def _split_bounded_polynomial_factor(
    node: IRNode, k: IRSymbol
) -> tuple[IRNode, int] | None:
    """Return ``(bounded_factor, poly_degree)`` when ``node`` is a
    ``Mul`` whose factors split into a bounded part and a polynomial
    part in ``k``; ``None`` otherwise.

    Phase 52 — Used by :func:`_g_vanishes_at_infinity` to recognise
    that ``sin(k)·k/k³`` vanishes (bounded × deg 1 over deg 3).  The
    bounded factor must contain at least one non-constant-in-k
    expression (otherwise Phase 49 would have caught the whole
    numerator).

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. Partition each factor into bounded vs polynomial buckets.
         (Factors that are neither bounded nor polynomial → bail.)
      3. Require at least one non-constant-in-k bounded factor
         (otherwise we'd just be re-applying Phase 42).
      4. Sum the polynomial factors' degrees.
      5. Return ``(bounded_aggregate, summed_poly_degree)``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    bounded_factors: list[IRNode] = []
    poly_degree = 0
    has_non_constant_bounded = False
    for arg in node.args:
        if _is_bounded_in_k(arg, k):
            bounded_factors.append(arg)
            if not _is_constant_in(arg, k):
                has_non_constant_bounded = True
            continue
        # Try as polynomial.
        deg = _polynomial_degree_in_k(arg, k)
        if deg is None:
            return None  # Unrecognised factor.
        poly_degree += deg
    if not has_non_constant_bounded:
        # Pure polynomial — Phase 42 will handle it.
        return None
    # Aggregate the bounded factors into a single representative node.
    # (Caller only uses the second return value; aggregate kept for
    # API parity with future extensions.)
    if not bounded_factors:
        return None
    if len(bounded_factors) == 1:
        bounded_aggregate = bounded_factors[0]
    else:
        bounded_aggregate = IRApply(MUL, tuple(bounded_factors))
    return (bounded_aggregate, poly_degree)


def _sqrt_effective_half_degree_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 51 helper: return 2× the effective half-degree of a ``Sqrt(P(k))``
    node, or ``None`` when the shape isn't recognised.

    For ``Sqrt(P(k))`` the effective growth rate is ``deg(P) / 2``.  To
    avoid floating-point arithmetic, we return ``deg(P)`` (i.e., twice the
    half-degree) and let the caller compare against ``2 * den_deg``
    instead of against the fractional ``deg(P) / 2``.

    Requirements:
      - ``node = Sqrt(P)`` (exactly one argument under the Sqrt head).
      - ``P`` is a polynomial in ``k``.
      - The leading coefficient of ``P`` in ``k`` is positive (so
        ``Sqrt(P(k))`` is real-valued and diverges to ``+∞``).  This
        refuses shapes like ``Sqrt(Mul(-1, k))`` where ``P(k) = -k``
        goes negative for large ``k``.

    Returns ``deg(P)`` (a non-negative integer), which equals
    ``2 * effective_half_degree``.

    +------------------------+----------------------+
    | Input                  | Return               |
    +========================+======================+
    | ``Sqrt(k)``            | 1 (deg 1 poly)       |
    | ``Sqrt(k²)``           | 2                    |
    | ``Sqrt(k² + k + 1)``   | 2                    |
    | ``Sqrt(Mul(-1, k))``   | None (neg leading)   |
    | ``k``                  | None (not Sqrt)      |
    | ``Sqrt(Sin(k))``       | None (not poly)      |
    +------------------------+----------------------+
    """
    if not isinstance(node, IRApply):
        return None
    if node.head != SQRT or len(node.args) != 1:
        return None
    inner = node.args[0]
    # Inner must be a polynomial in k.
    deg = _polynomial_degree_in_k(inner, k)
    if deg is None:
        return None
    # Degree-0 inner (constant) means Sqrt(c) is a constant — no effective
    # polynomial growth.  Return None so Phase 42 handles it.
    if deg == 0:
        return None
    # Leading coefficient of the inner polynomial must be positive so that
    # P(k) → +∞ (real-valued and diverging).  Reject if we can't confirm
    # sign.
    if _polynomial_leading_coeff_sign_in_k(inner, k) != 1:
        return None
    return deg  # = 2 × effective half-degree


def _sqrt_poly_numerator_effective_degree_x2(
    node: IRNode, k: IRSymbol
) -> int | None:
    """Phase 53 helper: return 2× the effective growth degree of a
    ``Mul(Sqrt(P), polynomial_factors)`` numerator, or ``None`` when
    the shape isn't recognised.

    The numerator ``Sqrt(P(k)) · Q(k)`` grows at rate
    ``deg(P)/2 + deg(Q)``.  To keep the comparison integral, we return
    ``deg(P) + 2·deg(Q)`` (= 2 × the effective degree).  The caller
    compares ``2 * den_deg > effective_x2`` to decide whether the
    quotient vanishes.

    Requirements:
      - ``node = Mul(...)`` (Phase 51 handles the plain-Sqrt case).
      - Exactly one factor is a ``Sqrt(P)`` with positive-leading-coeff
        polynomial inner (via :func:`_sqrt_effective_half_degree_x2`).
      - All other factors are polynomials in ``k``.

    Returns ``deg(P) + 2·deg(Q)`` (a non-negative integer).

    +----------------------------------+----------------+
    | Input (numerator node)           | Return         |
    +==================================+================+
    | ``Mul(Sqrt(k²), k)``             | 2 + 2 = 4      |
    | ``Mul(Sqrt(k), k²)``             | 1 + 4 = 5      |
    | ``Mul(Sqrt(k), Sin(k))``         | None (Sin not  |
    |                                  |  poly)         |
    | ``Sqrt(k)`` (plain, not Mul)     | None (→ Ph 51) |
    | ``Mul(Sqrt(k), Sqrt(k))``        | None (two Sqrt)|
    +----------------------------------+----------------+
    """
    if not isinstance(node, IRApply):
        return None
    if node.head != MUL:
        return None
    sqrt_inner_deg: int | None = None
    poly_deg_sum: int = 0
    for arg in node.args:
        # Try to classify this factor as a Sqrt(P) shape.
        eff = _sqrt_effective_half_degree_x2(arg, k)
        if eff is not None:
            # Only one Sqrt factor is allowed — if we see a second, bail.
            if sqrt_inner_deg is not None:
                return None
            sqrt_inner_deg = eff
            continue
        # Otherwise it must be a polynomial in k.
        deg = _polynomial_degree_in_k(arg, k)
        if deg is None:
            # Neither a Sqrt shape nor a polynomial — pattern rejected.
            return None
        poly_deg_sum += deg
    # Must have found exactly one Sqrt factor.
    if sqrt_inner_deg is None:
        return None
    return sqrt_inner_deg + 2 * poly_deg_sum


def _split_log_polynomial_factor(
    node: IRNode, k: IRSymbol
) -> tuple[IRNode, int] | None:
    """Return ``(log_factor, poly_degree)`` when ``node`` is a ``Mul``
    whose factors split into exactly one ``Log(diverging)`` part and a
    polynomial part in ``k``; ``None`` otherwise.

    Phase 54 — Used by :func:`_g_vanishes_at_infinity` to recognise that
    ``log(k)·k/k³`` (and similar shapes) vanish at infinity.  The key
    mathematical fact is that ``log(h(k)) = o(k^ε)`` for any ``ε > 0``,
    so ``log(h) · P(k)`` has the same effective growth degree as ``P(k)``
    alone — the log factor is negligible for degree comparisons.

    +-----------------------------------+---------------------+
    | Input                             | Return              |
    +===================================+=====================+
    | ``Mul(Log(k), k)``                | ``(Log(k), 1)``     |
    | ``Mul(Log(k), k²)``               | ``(Log(k), 2)``     |
    | ``Mul(Log(k+1), k³)``             | ``(Log(k+1), 3)``   |
    | ``Mul(Log(k), Sin(k))``           | None (Sin not poly) |
    | ``Mul(Log(k), Log(k))``           | None (two Log)      |
    | ``Mul(Sin(k), k)``                | None (no Log factor)|
    | ``Log(k)`` (plain, not Mul)       | None (→ Phase 50)   |
    +-----------------------------------+---------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. Partition each factor:
         - Exactly one :func:`_is_log_of_diverging_in_k` factor → ``log_factor``.
         - All others must be polynomials in ``k``; sum their degrees.
         - Any factor that is neither → bail.
      3. Must find exactly one log factor; zero or two → return ``None``.
      4. Return ``(log_factor, poly_deg_sum)``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_factor: IRNode | None = None
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            # Only one Log(diverging) factor is allowed.
            if log_factor is not None:
                return None
            log_factor = arg
            continue
        # Otherwise it must be a polynomial in k.
        deg = _polynomial_degree_in_k(arg, k)
        if deg is None:
            # Neither a Log(diverging) shape nor a polynomial — bail.
            return None
        poly_deg_sum += deg
    # Must have found exactly one Log(diverging) factor.
    if log_factor is None:
        return None
    return (log_factor, poly_deg_sum)


def _is_bounded_times_log_in_k(node: IRNode, k: IRSymbol) -> bool:
    """Return True when ``node`` is a ``Mul`` with exactly one
    ``Log(diverging)`` factor and all remaining factors bounded in ``k``.

    Phase 55 — Used by :func:`_g_vanishes_at_infinity` to recognise that
    ``sin(k)·log(k)/k²`` (and similar shapes) vanish at infinity.  The
    bounded part is uniformly bounded by some constant ``C``, and
    ``log(h(k))`` grows sub-polynomially.  Therefore the numerator as a
    whole grows sub-polynomially, which is dominated by any polynomial
    denominator of degree ≥ 1 (or any other diverging denominator).

    This is the bounded-times-log analog of Phase 52
    (``Mul(bounded, polynomial)``) and Phase 54
    (``Mul(Log(diverging), polynomial_factors)``).

    +-------------------------------------+-----------------------+
    | Input                               | Return                |
    +=====================================+=======================+
    | ``Mul(Sin(k), Log(k))``             | True                  |
    | ``Mul(Cos(k), Log(k))``             | True                  |
    | ``Mul(Sin(k), Cos(k), Log(k))``     | True (two bounded)    |
    | ``Mul(Sin(k), Log(k+1))``           | True (log of k+1)     |
    | ``Mul(k, Log(k))``                  | False (k not bounded) |
    | ``Mul(Sin(k), Log(k), Log(k))``     | False (two Log)       |
    | ``Mul(Sin(k), k, Log(k))``          | False (k not bounded) |
    | ``Log(k)`` (plain, not Mul)         | False (→ Phase 50)    |
    | ``Mul(Sin(k), k)`` (no Log)         | False (→ Phase 52)    |
    +-------------------------------------+-----------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - If it is ``Log(diverging_in_k)`` → count as log factor.
         - If it is ``bounded_in_k`` → accept as bounded factor.
         - Otherwise → return False (unrecognised factor).
      3. Require exactly one log factor; zero or two+ → return False.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return False
    log_count = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            continue
        if _is_bounded_in_k(arg, k):
            continue
        # Factor is neither Log(diverging) nor bounded — unrecognised.
        return False
    return log_count == 1


def _bounded_times_sqrt_inner_deg(node: IRNode, k: IRSymbol) -> int | None:
    """Return the ``Sqrt`` inner polynomial degree (×2) when ``node`` is a
    ``Mul`` with exactly one ``Sqrt(positive-leading polynomial)`` factor
    and all remaining factors bounded in ``k``; ``None`` otherwise.

    Phase 56 — bounded × sqrt analogue of Phase 55's bounded × log.
    Mirrors :func:`_is_bounded_times_log_in_k` but returns the sqrt
    inner-degree (× 2 to stay exact) so the caller can compare growth
    rates without floats.

    The bounded part is uniformly bounded by some constant ``C``; the
    sqrt part grows like ``k^{deg(P)/2}``.  Therefore the numerator's
    effective polynomial degree is ``deg(P)/2``, expressed here as
    ``deg(P)`` to be compared against ``2 × den_polynomial_degree``.

    +---------------------------------------+------------------+
    | Input                                 | Return           |
    +=======================================+==================+
    | ``Mul(Sin(k), Sqrt(k))``              | ``1`` (deg P=1)  |
    | ``Mul(Cos(k), Sqrt(k³))``             | ``3``            |
    | ``Mul(Sin(k), Cos(k), Sqrt(k+1))``    | ``1``            |
    | ``Mul(k, Sqrt(k))``                   | ``None`` (k not bounded) |
    | ``Mul(Sin(k), Sqrt(k), Sqrt(k))``     | ``None`` (two sqrt)      |
    | ``Sqrt(k)`` (plain, not Mul)          | ``None`` (→ Phase 51)    |
    | ``Mul(Sin(k), Sqrt(-k))``             | ``None`` (negative poly) |
    +---------------------------------------+------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - If it is ``Sqrt(positive-leading polynomial)`` → record its
           ``×2`` degree; refuse if a second sqrt appears.
         - If it is ``bounded_in_k`` → accept.
         - Otherwise → return ``None`` (unrecognised factor).
      3. Require exactly one sqrt factor; zero → ``None``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_inner_deg: int | None = None
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_inner_deg is not None:
                # Two Sqrt factors — refuse (would need a different
                # growth-rate calculation we haven't justified yet).
                return None
            sqrt_inner_deg = deg_x2
            continue
        if _is_bounded_in_k(arg, k):
            continue
        # Neither Sqrt(positive-poly) nor bounded → unrecognised.
        return None
    return sqrt_inner_deg


def _bounded_log_sqrt_inner_deg(node: IRNode, k: IRSymbol) -> int | None:
    """Return the ``Sqrt`` inner polynomial degree (×2) when ``node`` is a
    ``Mul`` with exactly one ``Log(diverging)`` factor, exactly one
    ``Sqrt(positive-leading polynomial)`` factor, and any number of
    bounded factors; ``None`` otherwise.

    Phase 57 — Combines the sub-polynomial growth of ``Log`` with the
    half-polynomial growth of ``Sqrt(P)``.  Effective growth
    ``log(k)·k^{deg(P)/2}`` is strictly dominated by ``k^{deg(P)/2+ε}``
    for any ``ε > 0`` since ``log(k) = o(k^ε)``.  Caller compares
    ``2·den_deg > deg(P)``.

    Requires **both** Log and Sqrt — patterns with only one fall
    through to Phase 55 (bounded × Log) or Phase 56 (bounded × Sqrt).
    Two-of-either is refused (conservative, would need combined
    growth-rate logic).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count = 0
    sqrt_inner_deg: int | None = None
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 1:
                # Two or more Log factors — refuse.
                return None
            continue
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_inner_deg is not None:
                # Two Sqrt factors — refuse.
                return None
            sqrt_inner_deg = deg_x2
            continue
        if _is_bounded_in_k(arg, k):
            continue
        # Unrecognised factor.
        return None
    if log_count != 1 or sqrt_inner_deg is None:
        return None
    return sqrt_inner_deg


def _bounded_log_poly_degree(node: IRNode, k: IRSymbol) -> int | None:
    """Return the total polynomial degree when ``node`` is a ``Mul`` with
    exactly one ``Log(diverging)`` factor, any polynomial factors (degree
    ≥ 0), and any number of bounded (but non-polynomial) factors in ``k``;
    ``None`` otherwise.

    Phase 58 — Bounded × Log(diverging) × polynomial numerator.

    This phase fills the gap between:

    * **Phase 54** — ``Mul(Log, polynomial_only)``; refuses the moment any
      factor is neither Log nor polynomial, so ``Sin(k)·Log(k)·k`` fails.
    * **Phase 55** — ``Mul(bounded, Log)``; requires *all* non-Log factors
      to be bounded, so ``Sin(k)·Log(k)·k`` fails (``k`` diverges).

    Here we allow any mix of polynomial and bounded factors alongside one
    Log:

    +---------------------------------------+--------------+
    | Input                                 | Return       |
    +=======================================+==============+
    | ``Mul(Sin(k), Log(k), k)``            | ``1``        |
    | ``Mul(Cos(k), Log(k), k²)``           | ``2``        |
    | ``Mul(Sin(k), Cos(k), Log(k), k)``    | ``1``        |
    | ``Mul(Sin(k), Log(k))``               | ``0``        |
    | ``Mul(Sin(k), Log(k), Log(k))``       | None (2 Log) |
    | ``Mul(Sqrt(k), Log(k), k)``           | None (Sqrt)  |
    | ``Mul(Sin(k), k)``                    | None (no Log)|
    +---------------------------------------+--------------+

    Mathematical basis:
      ``|Sin(k)·Log(k)·k^m| = O(k^m · log k) = o(k^{m+ε})`` for any
      ``ε > 0``.  The quotient therefore vanishes whenever the denominator
      grows strictly faster than ``k^m`` (i.e. ``den_deg > m`` or the
      denominator is non-polynomial diverging).

    The Sqrt case is intentionally refused here — that is handled by
    Phase 57 (``bounded × Log × Sqrt``).

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - Exactly one ``Log(diverging)`` → record it; bail on second.
         - Polynomial in ``k`` → add its degree to ``poly_deg_sum``.
         - Bounded in ``k`` (non-polynomial) → accept silently.
         - Sqrt or anything else → bail (unrecognised).
      3. Require exactly one Log factor found.
      4. Return ``poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 1:
                # Two or more Log factors — refuse (combined rate logic needed).
                return None
            continue
        # Polynomial factor (including degree-0 constants)?
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        # Bounded but non-polynomial (e.g. Sin, Cos, bounded Mul)?
        if _is_bounded_in_k(arg, k):
            continue
        # Unrecognised factor (Sqrt, Exp, free diverging, …) — bail.
        # Sqrt is handled by Phase 57; we don't want to silently consume it.
        return None
    if log_count != 1:
        return None
    return poly_deg_sum


def _bounded_sqrt_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_inner_deg_x2 + 2 * poly_deg_sum`` when ``node`` is a
    ``Mul`` with exactly one ``Sqrt(positive-leading polynomial)`` factor,
    any polynomial factors, and any number of bounded (non-polynomial,
    non-Sqrt) factors in ``k``; ``None`` otherwise.

    Phase 59 — Bounded × Sqrt(P) × polynomial numerator.

    This phase closes the gap between:

    * **Phase 53** — ``Mul(Sqrt, polynomial_only)``; refuses the moment any
      factor is neither ``Sqrt`` nor a polynomial, so ``Sin(k)·Sqrt(k)·k``
      fails.
    * **Phase 56** — ``Mul(bounded, Sqrt)``; requires *all* non-Sqrt factors
      to be bounded, so ``Sin(k)·Sqrt(k)·k`` fails (``k`` diverges
      polynomially).

    Here we allow any mix of polynomial and bounded factors alongside one
    ``Sqrt(positive-leading polynomial)`` factor:

    +---------------------------------------+-------------------+
    | Input                                 | Return            |
    +=======================================+===================+
    | ``Mul(Sin(k), Sqrt(k), k)``           | ``1 + 2 = 3``     |
    | ``Mul(Cos(k), Sqrt(k²), k²)``         | ``2 + 4 = 6``     |
    | ``Mul(Sin(k), Cos(k), Sqrt(k), k)``   | ``1 + 2 = 3``     |
    | ``Mul(Sin(k), Sqrt(k))``              | ``1 + 0 = 1``     |
    | ``Mul(Sin(k), Sqrt(k), Log(k), k)``   | ``None`` (Log)    |
    | ``Mul(Sin(k), Sqrt(k), Sqrt(k), k)``  | ``None`` (2 Sqrt) |
    | ``Mul(Sin(k), k)``                    | ``None`` (no Sqrt)|
    +---------------------------------------+-------------------+

    Mathematical basis:
      ``|bounded(k)·Sqrt(P(k))·k^m| = O(k^{deg(P)/2 + m})``.  Using the
      ×2 trick to stay exact: ``effective_x2 = deg(P) + 2·m``.  Caller
      checks ``2·den_deg > effective_x2``.

    Log factors are explicitly refused here — that combination is handled
    by Phase 57 (``bounded × Log × Sqrt``).

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - Exactly one ``Sqrt(positive-leading polynomial)`` → record
           ``×2`` degree; refuse if a second appears.
         - Log(diverging) → immediately return ``None`` (Phase 57 territory).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Sqrt, non-Log) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly one Sqrt factor.
      4. Return ``sqrt_inner_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_inner_deg: int | None = None
    poly_deg_sum: int = 0
    for arg in node.args:
        # Sqrt(positive-poly) factor?
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_inner_deg is not None:
                # Two Sqrt factors — refuse.
                return None
            sqrt_inner_deg = deg_x2
            continue
        # Log factor — refuse (belongs to Phase 57 / Phase 58 territory).
        if _is_log_of_diverging_in_k(arg, k):
            return None
        # Polynomial factor (degree ≥ 0)?
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        # Bounded (non-polynomial, non-Sqrt, non-Log)?
        if _is_bounded_in_k(arg, k):
            continue
        # Unrecognised factor — bail.
        return None
    if sqrt_inner_deg is None:
        return None
    return sqrt_inner_deg + 2 * poly_deg_sum


def _bounded_log_sqrt_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_inner_deg_x2 + 2 * poly_deg_sum`` when ``node`` is a
    ``Mul`` with exactly one ``Log(diverging)`` factor, exactly one
    ``Sqrt(positive-leading polynomial P)`` factor, any polynomial factors,
    and any number of bounded factors; ``None`` otherwise.

    Phase 60 — Bounded × Log(diverging) × Sqrt(P) × polynomial numerator.

    Extends Phase 57 (``bounded × Log × Sqrt``, refuses polynomial factors)
    by allowing polynomial factors alongside the ``Log`` and ``Sqrt``.

    Effective growth:
    ``|bounded · log(k) · Sqrt(P(k)) · k^m| = O(log(k) · k^{deg(P)/2 + m})``.
    Since ``log(k) = o(k^ε)`` for any ``ε > 0``, this is
    ``o(k^{deg(P)/2 + m + ε})``.  Using the ×2 trick:
    ``effective_x2 = deg(P) + 2·m``.  Caller checks
    ``2·den_deg > effective_x2``.

    +--------------------------------------------------+-------------------+
    | Input                                            | Return            |
    +==================================================+===================+
    | ``Mul(Sin(k), Log(k), Sqrt(k), k)``              | ``1 + 2 = 3``     |
    | ``Mul(Cos(k), Log(k), Sqrt(k²), k²)``            | ``2 + 4 = 6``     |
    | ``Mul(Sin(k), Cos(k), Log(k), Sqrt(k), k)``      | ``1 + 2 = 3``     |
    | ``Mul(Sin(k), Log(k), Sqrt(k))``                 | ``1 + 0 = 1``     |
    | ``Mul(Sin(k), Log(k), Log(k), Sqrt(k), k)``      | None (2 Log)      |
    | ``Mul(Sin(k), Log(k), Sqrt(k), Sqrt(k), k)``     | None (2 Sqrt)     |
    | ``Mul(Sin(k), Log(k), k)``                       | None (no Sqrt)    |
    | ``Mul(Sin(k), Sqrt(k), k)``                      | None (no Log)     |
    +--------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - Exactly one ``Log(diverging)`` → count; bail on second.
         - Exactly one ``Sqrt(positive-leading polynomial)`` → record ×2
           degree; bail on second.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly one Log AND exactly one Sqrt.
      4. Return ``sqrt_inner_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count = 0
    sqrt_inner_deg: int | None = None
    poly_deg_sum: int = 0
    for arg in node.args:
        # Log(diverging) factor?
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 1:
                # Two Log factors — refuse.
                return None
            continue
        # Sqrt(positive-leading polynomial) factor?
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_inner_deg is not None:
                # Two Sqrt factors — refuse.
                return None
            sqrt_inner_deg = deg_x2
            continue
        # Polynomial factor (degree ≥ 0)?
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        # Bounded (non-polynomial, non-Log, non-Sqrt)?
        if _is_bounded_in_k(arg, k):
            continue
        # Unrecognised factor — bail.
        return None
    if log_count != 1 or sqrt_inner_deg is None:
        return None
    return sqrt_inner_deg + 2 * poly_deg_sum


def _two_sqrt_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``deg(P1) + deg(P2) + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Sqrt(positive-leading polynomial)`` factors, any
    polynomial factors (total degree ``m``), and any number of bounded
    factors; ``None`` otherwise.

    Phase 61 — Two-Sqrt × polynomial numerator.

    Extends Phase 53 (``Mul(Sqrt(P), polynomial_only)``), Phase 56
    (``bounded × Sqrt``), and Phase 59 (``bounded × Sqrt × poly``) to the
    case where two distinct square-root factors appear, e.g.
    ``Sqrt(k) · Sqrt(k + 1) · k`` (product of consecutive Sqrt factors).

    Effective growth:
    ``Sqrt(P1(k)) · Sqrt(P2(k)) · k^m ≈ k^{deg(P1)/2 + deg(P2)/2 + m}``.
    Using the ×2 integer trick:
    ``effective_x2 = deg(P1) + deg(P2) + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    ``Log`` factors are refused (belong to Phase 60 / a future phase).

    +-----------------------------------------------------+-------------------+
    | Input                                               | Return            |
    +=====================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k+1), k)``                      | ``1 + 1 + 2 = 4`` |
    | ``Mul(Sqrt(k²), Sqrt(k), k²)``                      | ``2 + 1 + 4 = 7`` |
    | ``Mul(Sin(k), Sqrt(k), Sqrt(k+1), k)``              | ``1 + 1 + 2 = 4`` |
    | ``Mul(Sqrt(k), Sqrt(k))``                           | ``1 + 1 + 0 = 2`` |
    | ``Mul(Sqrt(k), k)``                                 | None (only 1 Sqrt)|
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k))``                  | None (3 Sqrt)     |
    | ``Mul(Log(k), Sqrt(k), Sqrt(k+1))``                 | None (Log present)|
    +-----------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree in a
           list; bail after the second one.
         - ``Log(diverging)`` → bail immediately (log-bearing patterns
           belong to Phase 60 and future log-Sqrt phases).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly two Sqrt factors.
      4. Return ``sqrt_deg1_x2 + sqrt_deg2_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs: list[int] = []
    poly_deg_sum: int = 0
    for arg in node.args:
        # Sqrt(positive-leading polynomial) factor?
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            sqrt_degs.append(deg_x2)
            if len(sqrt_degs) > 2:
                # Three or more Sqrt factors — refuse (conservative).
                return None
            continue
        # Log(diverging) factor — refuse (belongs to phase 60+ territory).
        if _is_log_of_diverging_in_k(arg, k):
            return None
        # Polynomial factor?
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        # Bounded (non-polynomial, non-Log, non-Sqrt)?
        if _is_bounded_in_k(arg, k):
            continue
        # Unrecognised factor — bail.
        return None
    if len(sqrt_degs) != 2:
        return None
    return sqrt_degs[0] + sqrt_degs[1] + 2 * poly_deg_sum


def _two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly two**
    ``Log(diverging-in-k)`` factors, any polynomial factors (total degree
    ``m``), and any number of bounded factors; ``None`` otherwise.

    Phase 62 — Two-Log × polynomial numerator.

    Effective growth:
    ``log(k)² · k^m ≈ o(k^{m + ε})`` for any ε > 0 (log² is sub-polynomial).
    Using the ×2 integer trick:
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    ``Sqrt`` factors are refused (belong to the two-Sqrt / log-Sqrt phases).

    +-------------------------------------------+-----------+
    | Input                                     | Return    |
    +===========================================+===========+
    | ``Mul(Log(k), Log(k))``                   | ``0``     |
    | ``Mul(Log(k), Log(k+1), k)``              | ``2``     |
    | ``Mul(Sin(k), Log(k), Log(k+1))``         | ``0``     |
    | ``Mul(Log(k), Log(k), k²)``               | ``4``     |
    | ``Mul(Log(k),)``                          | None (1 Log)|
    | ``Mul(Log(k), Log(k), Log(k))``           | None (3 Log)|
    | ``Mul(Log(k), Log(k), Sqrt(k))``          | None (Sqrt)|
    +-------------------------------------------+-----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Log(diverging)`` → count; bail after the second one.
         - ``Sqrt(...)`` → bail immediately (log-Sqrt patterns are separate).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly two Log factors.
      4. Return ``2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        # Log(diverging) factor?
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 2:
                # Three or more Log factors — refuse (conservative).
                return None
            continue
        # Sqrt factor — refuse (belongs to two-Sqrt / log-Sqrt phases).
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        # Polynomial factor?
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        # Bounded (non-polynomial, non-Log, non-Sqrt)?
        if _is_bounded_in_k(arg, k):
            continue
        # Unrecognised factor — bail.
        return None
    if log_count != 2:
        return None
    return 2 * poly_deg_sum


def _three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly three**
    ``Log(diverging-in-k)`` factors, any polynomial factors (total degree
    ``m``), and any number of bounded factors; ``None`` otherwise.

    Phase 67 — Three-Log × polynomial numerator.

    Effective growth:
    ``log(k)³ · k^m ≈ o(k^{m + ε})`` for any ε > 0 (log³ is sub-polynomial).
    Using the ×2 integer trick:
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    ``Sqrt`` factors are refused (belong to the Sqrt / log-Sqrt phases).

    +-------------------------------------------+-----------+
    | Input                                     | Return    |
    +===========================================+===========+
    | ``Mul(Log(k), Log(k), Log(k))``           | ``0``     |
    | ``Mul(Log(k), Log(k), Log(k+1), k)``      | ``2``     |
    | ``Mul(Log(k), Log(k), Log(k), k²)``       | ``4``     |
    | ``Mul(Log(k), Log(k))``                   | None (2 Log)|
    | ``Mul(Log(k), Log(k), Log(k), Log(k))``   | None (4 Log)|
    | ``Mul(Log(k), Log(k), Log(k), Sqrt(k))``  | None (Sqrt)|
    +-------------------------------------------+-----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Log(diverging)`` → count; bail after the third one.
         - ``Sqrt(...)`` → bail immediately (sqrt patterns are separate).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly three Log factors.
      4. Return ``2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        # Log(diverging) factor?
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 3:
                # Four or more Log factors — refuse (conservative).
                return None
            continue
        # Sqrt factor — refuse (belongs to sqrt / log-Sqrt phases).
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        # Polynomial factor?
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        # Bounded (non-polynomial, non-Log, non-Sqrt)?
        if _is_bounded_in_k(arg, k):
            continue
        # Unrecognised factor — bail.
        return None
    if log_count != 3:
        return None
    return 2 * poly_deg_sum


def _two_sqrt_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``deg(P1) + deg(P2) + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly one** ``Log(diverging-in-k)`` factor, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 63 — Two-Sqrt × Log × polynomial numerator.

    ``log(k)`` is sub-polynomial and does not change the effective degree.
    ``effective_x2 = deg(P1) + deg(P2) + 2·poly_deg``.
    Caller checks ``2·den_deg > effective_x2``.

    +--------------------------------------------------+-------------------+
    | Input                                            | Return            |
    +==================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k), Log(k))``                | ``1 + 1 = 2``     |
    | ``Mul(Sqrt(k³), Sqrt(k), Log(k))``               | ``3 + 1 = 4``     |
    | ``Mul(Sin(k), Sqrt(k), Sqrt(k), Log(k), k)``     | ``1 + 1 + 2 = 4`` |
    | ``Mul(Sqrt(k), Log(k))``                         | None (1 Sqrt)     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k), Log(k))``        | None (2 Logs)     |
    +--------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 2.
         - ``Log(diverging)`` → count; bail after 1.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Sqrt and exactly 1 Log.
      4. Return ``sqrt_deg1_x2 + sqrt_deg2_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            sqrt_degs.append(deg_x2)
            if len(sqrt_degs) > 2:
                return None
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 1:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs) != 2 or log_count != 1:
        return None
    return sqrt_degs[0] + sqrt_degs[1] + 2 * poly_deg_sum


def _two_log_sqrt_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_inner_deg + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Log(diverging-in-k)`` factors, **exactly one**
    ``Sqrt(positive-leading polynomial)`` factor, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 64 — Two-Log × Sqrt × polynomial numerator.

    ``log²(k)`` is sub-polynomial and does not change the effective degree.
    ``effective_x2 = sqrt_inner_deg_x2 + 2·poly_deg``.
    Caller checks ``2·den_deg > effective_x2``.

    +--------------------------------------------------+-------------------+
    | Input                                            | Return            |
    +==================================================+===================+
    | ``Mul(Log(k), Log(k), Sqrt(k))``                 | ``1 + 0 = 1``     |
    | ``Mul(Log(k), Log(k+1), Sqrt(k³))``              | ``3 + 0 = 3``     |
    | ``Mul(Sin(k), Log(k), Log(k), Sqrt(k), k)``      | ``1 + 2 = 3``     |
    | ``Mul(Log(k), Sqrt(k))``                         | None (1 Log)      |
    | ``Mul(Log(k), Log(k), Log(k), Sqrt(k))``         | None (3 Logs)     |
    | ``Mul(Log(k), Log(k), Sqrt(k), Sqrt(k))``        | None (2 Sqrts)    |
    +--------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Log(diverging)`` → count; bail after 2.
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 1.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Log and exactly 1 Sqrt.
      4. Return ``sqrt_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    sqrt_deg_x2: int | None = None
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 2:
                return None
            continue
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                # Second Sqrt — refuse.
                return None
            sqrt_deg_x2 = deg_x2
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 2 or sqrt_deg_x2 is None:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``deg(P1) + deg(P2) + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly two** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 65 — Two-Sqrt × Two-Log × polynomial numerator.

    ``log²(k)`` is sub-polynomial and does not change the effective degree.
    ``effective_x2 = deg(P1) + deg(P2) + 2·poly_deg`` (same as Phase 61).
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------+-------------------+
    | Input                                                | Return            |
    +======================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k), Log(k), Log(k))``            | ``1 + 1 = 2``     |
    | ``Mul(Sqrt(k³), Sqrt(k), Log(k), Log(k+1))``         | ``3 + 1 = 4``     |
    | ``Mul(Sin(k), Sqrt(k), Sqrt(k), Log(k), Log(k), k)`` | ``1+1+2 = 4``     |
    | ``Mul(Sqrt(k), Log(k), Log(k))``                     | None (1 Sqrt)     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k))``                    | None (1 Log)      |
    +------------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 2.
         - ``Log(diverging)`` → count; bail after 2.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Sqrt and exactly 2 Log.
      4. Return ``sqrt_deg1_x2 + sqrt_deg2_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            sqrt_degs.append(deg_x2)
            if len(sqrt_degs) > 2:
                return None
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 2:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs) != 2 or log_count != 2:
        return None
    return sqrt_degs[0] + sqrt_degs[1] + 2 * poly_deg_sum


def _three_sqrt_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``deg(P1) + deg(P2) + deg(P3) + 2·poly_deg`` when ``node`` is a
    ``Mul`` with **exactly three** ``Sqrt(positive-leading polynomial)`` factors,
    any polynomial factors, and any bounded factors; ``None`` otherwise.

    Phase 66 — Three-Sqrt × polynomial numerator.

    Three ``Sqrt`` factors are each sub-polynomial (``o(k^ε)`` relative to
    ``k^(deg/2)``), so their combined growth is ``k^((d1+d2+d3)/2)``.
    ``effective_x2 = deg(P1) + deg(P2) + deg(P3) + 2·poly_deg``
    (stores ``2×`` the true half-degree for integer arithmetic).
    Caller checks ``2·den_deg > effective_x2``.

    Log factors are intentionally refused here — use Phase 63 / 64 / 65
    for the sqrt+log combinations.

    +---------------------------------------------------+-------------------+
    | Input                                             | Return            |
    +===================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k))``                | ``1+1+1 = 3``     |
    | ``Mul(Sqrt(k³), Sqrt(k), Sqrt(k))``               | ``3+1+1 = 5``     |
    | ``Mul(Sin(k), Sqrt(k), Sqrt(k), Sqrt(k))``        | ``1+1+1 = 3``     |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), k)``             | ``1+1+1+2 = 5``   |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k))``                 | None (2 Sqrts)    |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k))``        | None (log present)|
    +---------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 3.
         - ``Log(diverging)`` → return ``None`` immediately (not handled here).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 3 Sqrt factors.
      4. Return ``sqrt_deg1_x2 + sqrt_deg2_x2 + sqrt_deg3_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs: list[int] = []
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            sqrt_degs.append(deg_x2)
            if len(sqrt_degs) > 3:
                return None
            continue
        # Log factors not handled here — bail so Phase 63/64/65 can catch them.
        if _is_log_of_diverging_in_k(arg, k):
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs) != 3:
        return None
    return sqrt_degs[0] + sqrt_degs[1] + sqrt_degs[2] + 2 * poly_deg_sum


def _three_sqrt_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``deg(P1) + deg(P2) + deg(P3) + 2·poly_deg`` when ``node`` is a
    ``Mul`` with **exactly three** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly one** ``Log(diverging-in-k)`` factor, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 68 — Three-Sqrt × Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^d1) · sqrt(k^d2) · sqrt(k^d3) · log(k) · k^m``
    ``≈ k^{(d1+d2+d3)/2} · log(k) · k^m``.
    Log is sub-polynomial (``o(k^ε)``), so it contributes 0 to effective degree.
    Using the ×2 integer trick:
    ``effective_x2 = d1 + d2 + d3 + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------+-------------------+
    | Input                                              | Return            |
    +====================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k))``         | ``1+1+1 = 3``     |
    | ``Mul(Sqrt(k³), Sqrt(k), Sqrt(k), Log(k+1))``      | ``3+1+1 = 5``     |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k), k)``      | ``1+1+1+2 = 5``   |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k), Log(k))`` | None (2 Logs)     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k))``                  | None (2 Sqrts)    |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k))``                 | None (0 Logs)     |
    +----------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 3.
         - ``Log(diverging)`` → count; bail after 1.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 3 Sqrt factors AND exactly 1 Log factor.
      4. Return ``sqrt_deg1_x2 + sqrt_deg2_x2 + sqrt_deg3_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            sqrt_degs.append(deg_x2)
            if len(sqrt_degs) > 3:
                return None
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 1:
                # Two or more Log factors — not this phase.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs) != 3 or log_count != 1:
        return None
    return sqrt_degs[0] + sqrt_degs[1] + sqrt_degs[2] + 2 * poly_deg_sum


def _one_sqrt_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_inner_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly one** ``Sqrt(positive-leading polynomial)`` factor,
    **exactly three** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 69 — One-Sqrt × Three-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^d) · log(k)³ · k^m ≈ k^{d/2} · log³(k) · k^m``.
    ``log³(k)`` is sub-polynomial (``o(k^ε)``), so it contributes 0 to effective
    degree.  Using the ×2 integer trick:
    ``effective_x2 = d + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------+------------------+
    | Input                                                    | Return           |
    +==========================================================+==================+
    | ``Mul(Sqrt(k), Log(k), Log(k), Log(k))``                 | ``1 + 0 = 1``    |
    | ``Mul(Sqrt(k³), Log(k), Log(k+1), Log(k+2))``            | ``3 + 0 = 3``    |
    | ``Mul(Sqrt(k), Log(k), Log(k), Log(k), k)``              | ``1 + 2 = 3``    |
    | ``Mul(Sqrt(k), Log(k), Log(k))``                         | None (2 Logs)    |
    | ``Mul(Sqrt(k), Log(k), Log(k), Log(k), Log(k))``         | None (4 Logs)    |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k), Log(k), Log(k))``        | None (2 Sqrts)   |
    | ``Mul(Log(k), Log(k), Log(k))``                          | None (0 Sqrts)   |
    +----------------------------------------------------------+------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 1.
         - ``Log(diverging)`` → count; bail after 3.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 1 Sqrt AND exactly 3 Log factors.
      4. Return ``sqrt_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                # Second Sqrt — not this phase.
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 3:
                # Four or more Log factors — not this phase.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 3:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _three_sqrt_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``deg(P1) + deg(P2) + deg(P3) + 2·poly_deg`` when ``node`` is a
    ``Mul`` with **exactly three** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly two** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 70 — Three-Sqrt × Two-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^d1) · sqrt(k^d2) · sqrt(k^d3) · log²(k) · k^m``
    ``≈ k^{(d1+d2+d3)/2} · log²(k) · k^m``.
    ``log²(k)`` is sub-polynomial (``o(k^ε)``), so it contributes 0.
    Using the ×2 integer trick:
    ``effective_x2 = d1 + d2 + d3 + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +-------------------------------------------------------+-------------------+
    | Input                                                 | Return            |
    +=======================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k), Log(k))``    | ``1+1+1 = 3``     |
    | ``Mul(Sqrt(k³), Sqrt(k), Sqrt(k), Log(k), Log(k+1))`` | ``3+1+1 = 5``     |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k), Log(k), k)`` | ``1+1+1+2 = 5``   |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k))``            | None (1 Log)      |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k), Log(k))``             | None (2 Sqrts)    |
    +-------------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 3.
         - ``Log(diverging)`` → count; bail after 2.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 3 Sqrt AND exactly 2 Log factors.
      4. Return ``sqrt_deg1_x2 + sqrt_deg2_x2 + sqrt_deg3_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            sqrt_degs.append(deg_x2)
            if len(sqrt_degs) > 3:
                return None
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 2:
                # Three or more Log factors — not this phase.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs) != 3 or log_count != 2:
        return None
    return sqrt_degs[0] + sqrt_degs[1] + sqrt_degs[2] + 2 * poly_deg_sum


def _two_sqrt_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``deg(P1) + deg(P2) + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly three** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 71 — Two-Sqrt × Three-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^d1) · sqrt(k^d2) · log³(k) · k^m``
    ``≈ k^{(d1+d2)/2} · log³(k) · k^m``.
    ``log³(k)`` is sub-polynomial (``o(k^ε)``), so it contributes 0.
    Using the ×2 integer trick:
    ``effective_x2 = d1 + d2 + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +-------------------------------------------------------+-------------------+
    | Input                                                 | Return            |
    +=======================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k), Log(k), Log(k), Log(k))``     | ``1+1 = 2``       |
    | ``Mul(Sqrt(k³), Sqrt(k), Log(k), Log(k), Log(k+1))``  | ``3+1 = 4``       |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k), Log(k), Log(k), k)``  | ``1+1+2 = 4``     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k), Log(k))``             | None (2 Logs)     |
    | ``Mul(Sqrt(k), Log(k), Log(k), Log(k))``              | None (1 Sqrt)     |
    +-------------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 2.
         - ``Log(diverging)`` → count; bail after 3.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Sqrt AND exactly 3 Log factors.
      4. Return ``sqrt_deg1_x2 + sqrt_deg2_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            sqrt_degs.append(deg_x2)
            if len(sqrt_degs) > 2:
                # Three or more Sqrt factors — not this phase.
                return None
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 3:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs) != 2 or log_count != 3:
        return None
    return sqrt_degs[0] + sqrt_degs[1] + 2 * poly_deg_sum


def _three_sqrt_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``deg(P1) + deg(P2) + deg(P3) + 2·poly_deg`` when ``node`` is a
    ``Mul`` with **exactly three** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly three** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 72 — Three-Sqrt × Three-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^d1) · sqrt(k^d2) · sqrt(k^d3) · log³(k) · k^m``
    ``≈ k^{(d1+d2+d3)/2} · log³(k) · k^m``.
    ``log³(k)`` is sub-polynomial (``o(k^ε)``), so it contributes 0.
    Using the ×2 integer trick:
    ``effective_x2 = d1 + d2 + d3 + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------------------+-------------------+
    | Input                                                            | Return            |
    +==================================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k), Log(k), Log(k))``       | ``1+1+1 = 3``     |
    | ``Mul(Sqrt(k³), Sqrt(k), Sqrt(k), Log(k), Log(k), Log(k+1))``    | ``3+1+1 = 5``     |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k), Log(k), Log(k), k)``    | ``1+1+1+2 = 5``   |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k), Log(k))``              | None (2 Logs)     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k), Log(k), Log(k))``               | None (2 Sqrts)    |
    +------------------------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 3.
         - ``Log(diverging)`` → count; bail after 3.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 3 Sqrt AND exactly 3 Log factors.
      4. Return ``sqrt_deg1_x2 + sqrt_deg2_x2 + sqrt_deg3_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            sqrt_degs.append(deg_x2)
            if len(sqrt_degs) > 3:
                # More than three Sqrt factors — not this phase.
                return None
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 3:
                # More than three Log factors — not this phase.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs) != 3 or log_count != 3:
        return None
    return sqrt_degs[0] + sqrt_degs[1] + sqrt_degs[2] + 2 * poly_deg_sum


def _four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly four**
    ``Log(diverging-in-k)`` factors, any polynomial factors (total degree
    ``m``), and any number of bounded factors; ``None`` otherwise.

    Phase 73 — Four-Log × polynomial numerator.

    Effective growth:
    ``log(k)⁴ · k^m ≈ o(k^{m + ε})`` for any ε > 0 (log⁴ is sub-polynomial).
    Using the ×2 integer trick:
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    ``Sqrt`` factors are refused (belong to the Sqrt / log-Sqrt phases).

    +---------------------------------------------+-----------+
    | Input                                       | Return    |
    +=============================================+===========+
    | ``Mul(Log(k), Log(k), Log(k), Log(k))``     | ``0``     |
    | ``Mul(Log(k), Log(k), Log(k), Log(k+1), k)``| ``2``     |
    | ``Mul(Log(k)×4, k²)``                       | ``4``     |
    | ``Mul(Log(k)×3)``                           | None (3 Logs) |
    | ``Mul(Log(k)×5)``                           | None (5 Logs) |
    | ``Mul(Log(k)×4, Sqrt(k))``                  | None (Sqrt)   |
    +---------------------------------------------+-----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Log(diverging)`` → count; bail after the fourth one.
         - ``Sqrt(...)`` → bail immediately (sqrt patterns are separate).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly four Log factors.
      4. Return ``2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        # Log(diverging) factor?
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 4:
                # Five or more Log factors — refuse (conservative).
                return None
            continue
        # Sqrt factor — refuse (belongs to sqrt / log-Sqrt phases).
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        # Polynomial factor?
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        # Bounded (non-polynomial, non-Log, non-Sqrt)?
        if _is_bounded_in_k(arg, k):
            continue
        # Unrecognised factor — bail.
        return None
    if log_count != 4:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_inner_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly one** ``Sqrt(positive-leading polynomial)`` factor,
    **exactly four** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 74 — One-Sqrt × Four-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^d) · log(k)⁴ · k^m ≈ k^{d/2} · log⁴(k) · k^m``.
    ``log⁴(k)`` is sub-polynomial (``o(k^ε)``), so it contributes 0.
    Using the ×2 integer trick:
    ``effective_x2 = d + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------+------------------+
    | Input                                                    | Return           |
    +==========================================================+==================+
    | ``Mul(Sqrt(k), Log(k)×4)``                               | ``1 + 0 = 1``    |
    | ``Mul(Sqrt(k³), Log(k)×4)``                              | ``3 + 0 = 3``    |
    | ``Mul(Sqrt(k), Log(k)×4, k)``                            | ``1 + 2 = 3``    |
    | ``Mul(Sqrt(k), Log(k)×3)``                               | None (3 Logs)    |
    | ``Mul(Sqrt(k), Log(k)×5)``                               | None (5 Logs)    |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×4)``                      | None (2 Sqrts)   |
    | ``Mul(Log(k)×4)``                                        | None (0 Sqrts)   |
    +----------------------------------------------------------+------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 1.
         - ``Log(diverging)`` → count; bail after 4.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 1 Sqrt AND exactly 4 Log factors.
      4. Return ``sqrt_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                # Second Sqrt — not this phase.
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 4:
                # Five or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 4:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _one_sqrt_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_inner_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly one** ``Sqrt(positive-leading polynomial)`` factor,
    **exactly five** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 78 — One-Sqrt × Five-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^d) · log(k)⁵ · k^m ≈ k^{d/2} · log⁵(k) · k^m``.
    ``log⁵(k)`` is sub-polynomial (``o(k^ε)``), so it contributes 0.
    Using the ×2 integer trick:
    ``effective_x2 = d + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------+------------------+
    | Input                                                    | Return           |
    +==========================================================+==================+
    | ``Mul(Sqrt(k), Log(k)×5)``                               | ``1 + 0 = 1``    |
    | ``Mul(Sqrt(k³), Log(k)×5)``                              | ``3 + 0 = 3``    |
    | ``Mul(Sqrt(k), Log(k)×5, k)``                            | ``1 + 2 = 3``    |
    | ``Mul(Sqrt(k), Log(k)×4)``                               | None (4 Logs)    |
    | ``Mul(Sqrt(k), Log(k)×6)``                               | None (6 Logs)    |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×5)``                      | None (2 Sqrts)   |
    | ``Mul(Log(k)×5)``                                        | None (0 Sqrts)   |
    +----------------------------------------------------------+------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 1.
         - ``Log(diverging)`` → count; bail after 5.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded (non-polynomial, non-Log, non-Sqrt) → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 1 Sqrt AND exactly 5 Log factors.
      4. Return ``sqrt_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                # Second Sqrt — not this phase.
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 5:
                # Six or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 5:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly four** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 75 — Two-Sqrt × Four-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · log(k)⁴ · k^m ≈ k^{(a+b)/2} · log⁴(k) · k^m``.
    ``log⁴(k)`` is sub-polynomial (``o(k^ε)``), contributing 0.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------------+------------------+
    | Input                                                          | Return           |
    +================================================================+==================+
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×4)``                            | ``1 + 1 = 2``    |
    | ``Mul(Sqrt(k³), Sqrt(k³), Log(k)×4)``                          | ``3 + 3 = 6``    |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×4, k)``                         | ``1 + 1 + 2 = 4``|
    | ``Mul(Sqrt(k), Log(k)×4)``                                     | None (1 Sqrt)    |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×4)``                   | None (3 Sqrts)   |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×3)``                            | None (3 Logs)    |
    +----------------------------------------------------------------+------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 2.
         - ``Log(diverging)`` → count; bail after 4.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Sqrt AND exactly 4 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                # Third Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 4:
                # Five or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 4:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg_sum


def _two_sqrt_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly five** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 79 — Two-Sqrt × Five-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · log(k)⁵ · k^m ≈ k^{(a+b)/2} · log⁵(k) · k^m``.
    ``log⁵(k)`` is sub-polynomial (``o(k^ε)``), contributing 0.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------------+------------------+
    | Input                                                          | Return           |
    +================================================================+==================+
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×5)``                            | ``1 + 1 = 2``    |
    | ``Mul(Sqrt(k³), Sqrt(k³), Log(k)×5)``                          | ``3 + 3 = 6``    |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×5, k)``                         | ``1+1+2 = 4``    |
    | ``Mul(Sqrt(k), Log(k)×5)``                                     | None (1 Sqrt)    |
    | ``Mul(Sqrt(k)×3, Log(k)×5)``                                   | None (3 Sqrts)   |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×4)``                            | None (4 Logs)    |
    +----------------------------------------------------------------+------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 2.
         - ``Log(diverging)`` → count; bail after 5.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Sqrt AND exactly 5 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                # Third Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 5:
                # Six or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 5:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg_sum


def _five_sqrt_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + sqrt5_deg_x2 + 2·poly_deg``
    when ``node`` is a ``Mul`` with **exactly five** ``Sqrt(positive-leading polynomial)``
    factors, **exactly five** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 82 — Five-Sqrt × Five-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a₁) · sqrt(k^a₂) · sqrt(k^a₃) · sqrt(k^a₄) · sqrt(k^a₅) · log(k)⁵ · k^m
    ≈ k^{(a₁+a₂+a₃+a₄+a₅)/2} · log⁵(k) · k^m``.
    ``log⁵(k)`` is sub-polynomial (``o(k^ε)``), contributing 0.
    Using the ×2 integer trick:
    ``effective_x2 = a₁ + a₂ + a₃ + a₄ + a₅ + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +--------------------------------------------------------------------------+---------------------+
    | Input                                                                    | Return              |
    +==========================================================================+=====================+
    | ``Mul(Sqrt(k)×5, Log(k)×5)``                                            | ``1+1+1+1+1 = 5``   |
    | ``Mul(Sqrt(k³)×5, Log(k)×5)``                                           | ``3+3+3+3+3 = 15``  |
    | ``Mul(Sqrt(k)×5, Log(k)×5, k)``                                         | ``1+1+1+1+1+2 = 7`` |
    | ``Mul(Sqrt(k)×4, Log(k)×5)``                                            | None (4 Sqrts)      |
    | ``Mul(Sqrt(k)×6, Log(k)×5)``                                            | None (6 Sqrts)      |
    | ``Mul(Sqrt(k)×5, Log(k)×4)``                                            | None (4 Logs)       |
    +--------------------------------------------------------------------------+---------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 5.
         - ``Log(diverging)`` → count; bail after 5.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 5 Sqrt AND exactly 5 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + sqrt5_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                # Sixth Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 5:
                # Six or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 5:
        return None
    return (sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2]
            + sqrt_degs_x2[3] + sqrt_degs_x2[4] + 2 * poly_deg_sum)


def _three_sqrt_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg`` when ``node`` is
    a ``Mul`` with **exactly three** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly four** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 76 — Three-Sqrt × Four-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁴ · k^m ≈ k^{(a+b+c)/2} · log⁴(k) · k^m``.
    ``log⁴(k)`` is sub-polynomial (``o(k^ε)``), contributing 0.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + c + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------------------+---------------------+
    | Input                                                            | Return              |
    +==================================================================+=====================+
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×4)``                     | ``1 + 1 + 1 = 3``   |
    | ``Mul(Sqrt(k³), Sqrt(k³), Sqrt(k³), Log(k)×4)``                  | ``3 + 3 + 3 = 9``   |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×4, k)``                  | ``1+1+1+2 = 5``     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×4)``                              | None (2 Sqrts)      |
    | ``Mul(Sqrt(k)×4, Log(k)×4)``                                     | None (4 Sqrts)      |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×3)``                     | None (3 Logs)       |
    +------------------------------------------------------------------+---------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 3.
         - ``Log(diverging)`` → count; bail after 4.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 3 Sqrt AND exactly 4 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                # Fourth Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 4:
                # Five or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 4:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg_sum


def _five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly five**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 77 — Five-Log × polynomial numerator.

    Effective growth:
    ``log(k)⁵ · k^m``.  ``log⁵(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (73–76, 78+).

    +-----------------------------------------------------+----------+
    | Input                                               | Return   |
    +=====================================================+==========+
    | ``Mul(Log(k)×5)``                                   | ``0``    |
    | ``Mul(Log(k)×5, k)``                                | ``2``    |
    | ``Mul(Log(k)×5, k², 3)``                            | ``4``    |
    | ``Mul(Log(k)×4)``                                   | None (4) |
    | ``Mul(Log(k)×6)``                                   | None (6) |
    | ``Mul(Sqrt(k), Log(k)×5)``                          | None     |
    +-----------------------------------------------------+----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Log(diverging)`` → count; bail after 5.
         - ``Sqrt(...)`` → refuse (return ``None``).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 5 Log factors.
      4. Return ``2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 5:
                # Six or more Log factors — not this phase.
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            # Sqrt factor present — refuse so Sqrt-bearing phases handle it.
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 5:
        return None
    return 2 * poly_deg_sum


def _three_sqrt_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg`` when ``node`` is
    a ``Mul`` with **exactly three** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly five** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 80 — Three-Sqrt × Five-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁵ · k^m ≈ k^{(a+b+c)/2} · log⁵(k) · k^m``.
    ``log⁵(k)`` is sub-polynomial (``o(k^ε)``), contributing 0.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + c + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------------------+---------------------+
    | Input                                                            | Return              |
    +==================================================================+=====================+
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×5)``                     | ``1 + 1 + 1 = 3``   |
    | ``Mul(Sqrt(k³), Sqrt(k³), Sqrt(k³), Log(k)×5)``                  | ``3 + 3 + 3 = 9``   |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×5, k)``                  | ``1+1+1+2 = 5``     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×5)``                              | None (2 Sqrts)      |
    | ``Mul(Sqrt(k)×4, Log(k)×5)``                                     | None (4 Sqrts)      |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×4)``                     | None (4 Logs)       |
    +------------------------------------------------------------------+---------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 3.
         - ``Log(diverging)`` → count; bail after 5.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 3 Sqrt AND exactly 5 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                # Fourth Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 5:
                # Six or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 5:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg_sum


def _four_sqrt_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + 2·poly_deg``
    when ``node`` is a ``Mul`` with **exactly four** ``Sqrt(positive-leading polynomial)``
    factors, **exactly five** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 81 — Four-Sqrt × Five-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)⁵ · k^m
    ≈ k^{(a+b+c+d)/2} · log⁵(k) · k^m``.
    ``log⁵(k)`` is sub-polynomial (``o(k^ε)``), contributing 0.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + c + d + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------------------------+---------------------+
    | Input                                                                  | Return              |
    +========================================================================+=====================+
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×5)``                  | ``1+1+1+1 = 4``     |
    | ``Mul(Sqrt(k³), Sqrt(k³), Sqrt(k³), Sqrt(k³), Log(k)×5)``              | ``3+3+3+3 = 12``    |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×5, k)``               | ``1+1+1+1+2 = 6``   |
    | ``Mul(Sqrt(k)×3, Log(k)×5)``                                           | None (3 Sqrts)      |
    | ``Mul(Sqrt(k)×5, Log(k)×5)``                                           | None (5 Sqrts)      |
    | ``Mul(Sqrt(k)×4, Log(k)×4)``                                           | None (4 Logs)       |
    +------------------------------------------------------------------------+---------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 4.
         - ``Log(diverging)`` → count; bail after 5.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 4 Sqrt AND exactly 5 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                # Fifth Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 5:
                # Six or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 5:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + sqrt_degs_x2[3] + 2 * poly_deg_sum


def _six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly six**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 83 — Six-Log × polynomial numerator.

    Effective growth:
    ``log(k)⁶ · k^m``.  ``log⁶(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (73–76, 78+).

    +-----------------------------------------------------+----------+
    | Input                                               | Return   |
    +=====================================================+==========+
    | ``Mul(Log(k)×6)``                                   | ``0``    |
    | ``Mul(Log(k)×6, k)``                                | ``2``    |
    | ``Mul(Log(k)×6, k², 3)``                            | ``4``    |
    | ``Mul(Log(k)×5)``                                   | None (5) |
    | ``Mul(Log(k)×7)``                                   | None (7) |
    | ``Mul(Sqrt(k), Log(k)×6)``                          | None     |
    +-----------------------------------------------------+----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Log(diverging)`` → count; bail after 6.
         - ``Sqrt(...)`` → refuse (return ``None``).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 6 Log factors.
      4. Return ``2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 6:
                # Seven or more Log factors — not this phase.
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            # Sqrt factor present — refuse so Sqrt-bearing phases handle it.
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 6:
        return None
    return 2 * poly_deg_sum


def _seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly seven**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 89 — Seven-Log × polynomial numerator.

    Effective growth:
    ``log(k)⁷ · k^m``.  ``log⁷(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 90 onward).

    +-----------------------------------------------------+----------+
    | Input                                               | Return   |
    +=====================================================+==========+
    | ``Mul(Log(k)×7)``                                   | ``0``    |
    | ``Mul(Log(k)×7, k)``                                | ``2``    |
    | ``Mul(Log(k)×7, k², 3)``                            | ``4``    |
    | ``Mul(Log(k)×6)``                                   | None (6) |
    | ``Mul(Log(k)×8)``                                   | None (8) |
    | ``Mul(Sqrt(k), Log(k)×7)``                          | None     |
    +-----------------------------------------------------+----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Log(diverging)`` → count; bail after 7.
         - ``Sqrt(...)`` → refuse (return ``None``).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 7 Log factors.
      4. Return ``2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 7:
                # Eight or more Log factors — not this phase.
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            # Sqrt factor present — refuse so Sqrt-bearing phases handle it.
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 7:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul`` with
    **exactly one** ``Sqrt(positive-polynomial)`` factor, **exactly seven**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 90 — One-Sqrt × Seven-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · log(k)⁷ · k^m``.  ``log⁷(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick: ``effective_x2 = a + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------+----------+
    | Input                                                    | Return   |
    +==========================================================+==========+
    | ``Mul(Sqrt(k), Log(k)×7)``                               | ``1``    |
    | ``Mul(Sqrt(k²), Log(k)×7, k)``                           | ``4``    |
    | ``Mul(Sqrt(k), Log(k)×7, k², 3)``                        | ``5``    |
    | ``Mul(Log(k)×7)``                                        | None     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×7)``                      | None (2) |
    | ``Mul(Sqrt(k), Log(k)×6)``                               | None (6) |
    +----------------------------------------------------------+----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(P)`` → record ``sqrt_effective_half_degree_x2``; bail on second Sqrt.
         - ``Log(diverging)`` → count; bail after 7.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 1 Sqrt and exactly 7 Log factors.
      4. Return ``sqrt_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        s_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if s_x2 is not None:
            if sqrt_deg_x2 is not None:
                # Second Sqrt — not this phase.
                return None
            sqrt_deg_x2 = s_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 7:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 7:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`` when ``node`` is a
    ``Mul`` with **exactly two** ``Sqrt(positive-leading polynomial)`` factors,
    **exactly seven** ``Log(diverging-in-k)`` factors, any polynomial factors,
    and any bounded factors; ``None`` otherwise.

    Phase 91 — Two-Sqrt × Seven-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · log(k)⁷ · k^m ≈ k^{(a+b)/2} · log⁷(k) · k^m``.
    ``log⁷(k)`` is sub-polynomial (``o(k^ε)``), contributing 0 to the
    effective polynomial degree.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------------+-------------------+
    | Input                                                          | Return            |
    +================================================================+===================+
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×7)``                            | ``1 + 1 = 2``     |
    | ``Mul(Sqrt(k³), Sqrt(k³), Log(k)×7)``                          | ``3 + 3 = 6``     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×7, k)``                         | ``1+1+2 = 4``     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×7, k², 3)``                     | ``1+1+4 = 6``     |
    | ``Mul(Sqrt(k), Log(k)×7)``                                     | None (1 Sqrt)     |
    | ``Mul(Sqrt(k)×3, Log(k)×7)``                                   | None (3 Sqrts)    |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×6)``                            | None (6 Logs)     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×8)``                            | None (8 Logs)     |
    +----------------------------------------------------------------+-------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 2.
         - ``Log(diverging)`` → count; bail after 7.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Sqrt AND exactly 7 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                # Third Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 7:
                # Eight or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 7:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg_sum


def _three_sqrt_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2·poly_deg`` when
    ``node`` is a ``Mul`` with **exactly three** ``Sqrt(positive-leading polynomial)``
    factors, **exactly seven** ``Log(diverging-in-k)`` factors, any polynomial
    factors, and any bounded factors; ``None`` otherwise.

    Phase 92 — Three-Sqrt × Seven-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · log(k)⁷ · k^m ≈ k^{(a+b+c)/2} · log⁷(k) · k^m``.
    ``log⁷(k)`` is sub-polynomial (``o(k^ε)``), contributing 0 to the
    effective polynomial degree.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + c + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------------------+---------------------+
    | Input                                                                | Return              |
    +======================================================================+=====================+
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×7)``                         | ``1 + 1 + 1 = 3``   |
    | ``Mul(Sqrt(k³), Sqrt(k³), Sqrt(k³), Log(k)×7)``                      | ``3 + 3 + 3 = 9``   |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×7, k)``                      | ``1+1+1+2 = 5``     |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×7, k², 3)``                  | ``1+1+1+4 = 7``     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×7)``                                  | None (2 Sqrts)      |
    | ``Mul(Sqrt(k)×4, Log(k)×7)``                                         | None (4 Sqrts)      |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×6)``                         | None (6 Logs)       |
    | ``Mul(Sqrt(k), Sqrt(k), Sqrt(k), Log(k)×8)``                         | None (8 Logs)       |
    +----------------------------------------------------------------------+---------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 3.
         - ``Log(diverging)`` → count; bail after 7.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 3 Sqrt AND exactly 7 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                # Fourth Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 7:
                # Eight or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 7:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + sqrt_degs_x2[2] + 2 * poly_deg_sum


def _four_sqrt_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + sqrt3_deg_x2 + sqrt4_deg_x2 + 2·poly_deg``
    when ``node`` is a ``Mul`` with **exactly four** ``Sqrt(positive-leading polynomial)``
    factors, **exactly seven** ``Log(diverging-in-k)`` factors, any polynomial
    factors, and any bounded factors; ``None`` otherwise.

    Phase 93 — Four-Sqrt × Seven-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · log(k)⁷ · k^m``.
    ``log⁷(k)`` is sub-polynomial (``o(k^ε)``), contributing 0 to the
    effective polynomial degree.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + c + d + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------------------------+------------------------+
    | Input                                                                  | Return                 |
    +========================================================================+========================+
    | ``Mul(Sqrt(k)×4, Log(k)×7)``                                           | ``1+1+1+1 = 4``        |
    | ``Mul(Sqrt(k²)×4, Log(k)×7)``                                          | ``2+2+2+2 = 8``        |
    | ``Mul(Sqrt(k)×4, Log(k)×7, k)``                                        | ``4+2 = 6``            |
    | ``Mul(Sqrt(k)×3, Log(k)×7)``                                           | None (3 Sqrts)         |
    | ``Mul(Sqrt(k)×5, Log(k)×7)``                                           | None (5 Sqrts)         |
    | ``Mul(Sqrt(k)×4, Log(k)×6)``                                           | None (6 Logs)          |
    +------------------------------------------------------------------------+------------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 4.
         - ``Log(diverging)`` → count; bail after 7.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 4 Sqrt AND exactly 7 Log factors.
      4. Return sum of all sqrt_deg_x2 values + 2 * poly_deg_sum.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                # Fifth Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 7:
                # Eight or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 7:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + … + sqrt5_deg_x2 + 2·poly_deg``
    when ``node`` is a ``Mul`` with **exactly five** ``Sqrt(positive-leading polynomial)``
    factors, **exactly seven** ``Log(diverging-in-k)`` factors, any polynomial
    factors, and any bounded factors; ``None`` otherwise.

    Phase 94 — Five-Sqrt × Seven-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · sqrt(k^c) · sqrt(k^d) · sqrt(k^e) · log(k)⁷ · k^m``.
    ``log⁷(k)`` is sub-polynomial (``o(k^ε)``), contributing 0 to the
    effective polynomial degree.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + c + d + e + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------------------------+------------------------+
    | Input                                                                  | Return                 |
    +========================================================================+========================+
    | ``Mul(Sqrt(k)×5, Log(k)×7)``                                           | ``1+1+1+1+1 = 5``      |
    | ``Mul(Sqrt(k²)×5, Log(k)×7)``                                          | ``2+2+2+2+2 = 10``     |
    | ``Mul(Sqrt(k)×5, Log(k)×7, k)``                                        | ``5+2 = 7``            |
    | ``Mul(Sqrt(k)×4, Log(k)×7)``                                           | None (4 Sqrts)         |
    | ``Mul(Sqrt(k)×6, Log(k)×7)``                                           | None (6 Sqrts)         |
    | ``Mul(Sqrt(k)×5, Log(k)×6)``                                           | None (6 Logs)          |
    +------------------------------------------------------------------------+------------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 5.
         - ``Log(diverging)`` → count; bail after 7.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 5 Sqrt AND exactly 7 Log factors.
      4. Return sum of all sqrt_deg_x2 values + 2 * poly_deg_sum.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                # Sixth Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 7:
                # Eight or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 7:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly eight**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 95 — Eight-Log × polynomial numerator.

    Effective growth:
    ``log(k)⁸ · k^m``.  ``log⁸(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 96 onward).

    +-----------------------------------------------------+----------+
    | Input                                               | Return   |
    +=====================================================+==========+
    | ``Mul(Log(k)×8)``                                   | ``0``    |
    | ``Mul(Log(k)×8, k)``                                | ``2``    |
    | ``Mul(Log(k)×8, k², 3)``                            | ``4``    |
    | ``Mul(Log(k)×7)``                                   | None (7) |
    | ``Mul(Log(k)×9)``                                   | None (9) |
    | ``Mul(Sqrt(k), Log(k)×8)``                          | None     |
    +-----------------------------------------------------+----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Log(diverging)`` → count; bail after 8.
         - ``Sqrt(...)`` → refuse (return ``None``).
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 8 Log factors.
      4. Return ``2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 8:
                # Nine or more Log factors — not this phase.
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            # Sqrt factor present — refuse so Sqrt-bearing phases handle it.
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 8:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul`` with
    **exactly one** ``Sqrt(positive-polynomial)`` factor, **exactly eight**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 96 — One-Sqrt × Eight-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · log(k)⁸ · k^m``.  ``log⁸(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick: ``effective_x2 = a + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------+----------+
    | Input                                                    | Return   |
    +==========================================================+==========+
    | ``Mul(Sqrt(k), Log(k)×8)``                               | ``1``    |
    | ``Mul(Sqrt(k²), Log(k)×8, k)``                           | ``4``    |
    | ``Mul(Sqrt(k), Log(k)×7)``                               | None (7) |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×8)``                      | None (2) |
    +----------------------------------------------------------+----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(P)`` → record; bail on second Sqrt.
         - ``Log(diverging)`` → count; bail after 8.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 1 Sqrt AND exactly 8 Log factors.
      4. Return ``sqrt_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        s_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if s_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None  # second Sqrt — not this phase
            sqrt_deg_x2 = s_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 8:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 8:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Sqrt(positive-polynomial)`` factors, **exactly eight**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 97 — Two-Sqrt × Eight-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · log(k)⁸ · k^m``.  ``log⁸(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick: ``effective_x2 = a + b + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------------------+----------+
    | Input                                                            | Return   |
    +==================================================================+==========+
    | ``Mul(Sqrt(k)×2, Log(k)×8)``                                     | ``2``    |
    | ``Mul(Sqrt(k²), Sqrt(k), Log(k)×8, k)``                          | ``5``    |
    | ``Mul(Sqrt(k), Log(k)×8)``                                       | None (1) |
    | ``Mul(Sqrt(k)×3, Log(k)×8)``                                     | None (3) |
    | ``Mul(Sqrt(k)×2, Log(k)×7)``                                     | None (7) |
    +------------------------------------------------------------------+----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(P)`` → record; bail after 2 Sqrts.
         - ``Log(diverging)`` → count; bail after 8.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Sqrt AND exactly 8 Log factors.
      4. Return ``sum(sqrt_degs_x2) + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None  # third Sqrt — not this phase
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 8:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 8:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_x2 + sqrt2_x2 + sqrt3_x2 + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly three** ``Sqrt`` factors, **exactly eight** ``Log`` factors, any
    polynomial factors, and any bounded factors; ``None`` otherwise.

    Phase 98 — Three-Sqrt × Eight-Log × polynomial numerator.

    Using the ×2 integer trick: ``effective_x2 = a + b + c + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None  # fourth Sqrt — not this phase
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 8:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 8:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_x2 + sqrt2_x2 + sqrt3_x2 + sqrt4_x2 + 2·poly_deg`` when ``node`` is a
    ``Mul`` with **exactly four** ``Sqrt`` factors, **exactly eight** ``Log`` factors, any
    polynomial factors, and any bounded factors; ``None`` otherwise.

    Phase 99 — Four-Sqrt × Eight-Log × polynomial numerator.

    Using the ×2 integer trick: ``effective_x2 = a + b + c + d + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None  # fifth Sqrt — not this phase
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 8:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 8:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_x2 + … + sqrt5_x2 + 2·poly_deg`` when ``node`` is a
    ``Mul`` with **exactly five** ``Sqrt`` factors, **exactly eight** ``Log`` factors,
    any polynomial factors, and any bounded factors; ``None`` otherwise.

    Phase 100 — Five-Sqrt × Eight-Log × polynomial numerator.
    Completes the Eight-Log family (Phases 95–100).

    Using the ×2 integer trick: ``effective_x2 = a + b + c + d + e + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None  # sixth Sqrt — not this phase
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 8:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 8:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly nine**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 101 — Nine-Log × polynomial numerator.

    Effective growth:
    ``log(k)⁹ · k^m``.  ``log⁹(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 102 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 9:
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 9:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul`` with
    **exactly one** ``Sqrt(positive-polynomial)`` factor, **exactly nine**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 102 — One-Sqrt × Nine-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · log(k)⁹ · k^m``.  ``log⁹(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick: ``effective_x2 = a + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        s_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if s_x2 is not None:
            if sqrt_deg_x2 is not None:
                # Second Sqrt — not this phase.
                return None
            sqrt_deg_x2 = s_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 9:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 9:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`` when ``node`` is a
    ``Mul`` with **exactly two** ``Sqrt`` factors, **exactly nine** ``Log`` factors,
    any polynomial factors, and any bounded factors; ``None`` otherwise.

    Phase 103 — Two-Sqrt × Nine-Log × polynomial numerator.

    ``effective_x2 = sqrt1_deg_x2 + sqrt2_deg_x2 + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 9:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 9:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg_sum


def _three_sqrt_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 104 — Three-Sqrt × Nine-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 9:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 9:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 105 — Four-Sqrt × Nine-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 9:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 9:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 106 — Five-Sqrt × Nine-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Nine-Log family (Phases 101–106).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 9:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 9:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _ten_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly ten**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 107 — Ten-Log × polynomial numerator.

    Effective growth:
    ``log(k)^10 · k^m``.  ``log^10(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 108 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 10:
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 10:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_ten_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 108 — One-Sqrt × Ten-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 10:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 10:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_ten_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 109 — Two-Sqrt × Ten-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 10:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 10:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_ten_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 110 — Three-Sqrt × Ten-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 10:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 10:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_ten_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 111 — Four-Sqrt × Ten-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 10:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 10:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_ten_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 112 — Five-Sqrt × Ten-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Ten-Log family (Phases 107-112).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 10:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 10:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _eleven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly eleven**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 113 — Eleven-Log × polynomial numerator.

    Effective growth:
    ``log(k)^11 · k^m``.  ``log^11(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 114 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 11:
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 11:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_eleven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 114 — One-Sqrt × Eleven-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 11:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 11:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_eleven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 115 — Two-Sqrt × Eleven-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 11:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 11:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_eleven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 116 — Three-Sqrt × Eleven-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 11:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 11:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_eleven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 117 — Four-Sqrt × Eleven-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 11:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 11:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_eleven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 118 — Five-Sqrt × Eleven-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Eleven-Log family (Phases 113-118).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 11:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 11:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twelve_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly twelve**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 119 — Twelve-Log × polynomial numerator.

    Effective growth:
    ``log(k)^12 · k^m``.  ``log^12(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 120 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 12:
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 12:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twelve_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 120 — One-Sqrt × Twelve-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 12:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 12:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twelve_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 121 — Two-Sqrt × Twelve-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 12:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 12:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twelve_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 122 — Three-Sqrt × Twelve-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 12:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 12:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twelve_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 123 — Four-Sqrt × Twelve-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 12:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 12:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twelve_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 124 — Five-Sqrt × Twelve-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Twelve-Log family (Phases 119-124).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 12:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 12:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly thirteen**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 125 — Thirteen-Log × polynomial numerator.

    Effective growth:
    ``log(k)^13 · k^m``.  ``log^13(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 126 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 13:
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 13:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 126 — One-Sqrt × Thirteen-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 13:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 13:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 127 — Two-Sqrt × Thirteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 13:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 13:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 128 — Three-Sqrt × Thirteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 13:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 13:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 129 — Four-Sqrt × Thirteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 13:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 13:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 130 — Five-Sqrt × Thirteen-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Thirteen-Log family (Phases 125-130).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 13:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 13:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fourteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly fourteen**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 131 — Fourteen-Log × polynomial numerator.

    Effective growth:
    ``log(k)^14 · k^m``.  ``log^14(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 132 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 14:
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 14:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fourteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 132 — One-Sqrt × Fourteen-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 14:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 14:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fourteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 133 — Two-Sqrt × Fourteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 14:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 14:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fourteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 134 — Three-Sqrt × Fourteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 14:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 14:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fourteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 135 — Four-Sqrt × Fourteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 14:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 14:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fourteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 136 — Five-Sqrt × Fourteen-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Fourteen-Log family (Phases 131-136).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 14:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 14:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly fifteen**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 137 — Fifteen-Log × polynomial numerator.

    Effective growth:
    ``log(k)^15 · k^m``.  ``log^15(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 138 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 15:
                return None
            continue
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 15:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 138 — One-Sqrt × Fifteen-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 15:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 15:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 139 — Two-Sqrt × Fifteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 15:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 15:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 140 — Three-Sqrt × Fifteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 15:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 15:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 141 — Four-Sqrt × Fifteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 15:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 15:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 142 — Five-Sqrt × Fifteen-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Fifteen-Log family (Phases 137-142).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 15:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 15:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _sixteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly sixteen**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 143 — Sixteen-Log × polynomial numerator.

    Effective growth:
    ``log(k)^16 · k^m``.  ``log^16(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 144 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 16:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 16:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_sixteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 144 — One-Sqrt × Sixteen-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 16:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 16:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_sixteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 145 — Two-Sqrt × Sixteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 16:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 16:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_sixteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 146 — Three-Sqrt × Sixteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 16:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 16:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_sixteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 147 — Four-Sqrt × Sixteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 16:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 16:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_sixteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 148 — Five-Sqrt × Sixteen-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Sixteen-Log family (Phases 143-148).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 16:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 16:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _seventeen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly seventeen**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 149 — Seventeen-Log × polynomial numerator.

    Effective growth:
    ``log(k)^17 · k^m``.  ``log^17(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 150 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 17:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 17:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_seventeen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 150 — One-Sqrt × Seventeen-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 17:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 17:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_seventeen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 151 — Two-Sqrt × Seventeen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 17:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 17:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_seventeen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 152 — Three-Sqrt × Seventeen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 17:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 17:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_seventeen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 153 — Four-Sqrt × Seventeen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 17:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 17:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_seventeen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 154 — Five-Sqrt × Seventeen-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Seventeen-Log family (Phases 149-154).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 17:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 17:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _eighteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly eighteen**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 155 — Eighteen-Log × polynomial numerator.

    Effective growth:
    ``log(k)^18 · k^m``.  ``log^18(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 156 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 18:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 18:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_eighteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 156 — One-Sqrt × Eighteen-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 18:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 18:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_eighteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 157 — Two-Sqrt × Eighteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 18:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 18:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_eighteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 158 — Three-Sqrt × Eighteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 18:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 18:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_eighteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 159 — Four-Sqrt × Eighteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 18:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 18:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_eighteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 160 — Five-Sqrt × Eighteen-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Eighteen-Log family (Phases 155-160).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 18:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 18:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _nineteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly nineteen**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 161 — Nineteen-Log × polynomial numerator.

    Effective growth:
    ``log(k)^19 · k^m``.  ``log^19(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 162 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 19:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 19:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_nineteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 162 — One-Sqrt × Nineteen-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 19:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 19:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_nineteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 163 — Two-Sqrt × Nineteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 19:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 19:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_nineteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 164 — Three-Sqrt × Nineteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 19:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 19:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_nineteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 165 — Four-Sqrt × Nineteen-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 19:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 19:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_nineteen_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 166 — Five-Sqrt × Nineteen-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Nineteen-Log family (Phases 161-166).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 19:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 19:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly twenty**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 167 — Twenty-Log × polynomial numerator.

    Effective growth:
    ``log(k)^20 · k^m``.  ``log^20(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 168 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 20:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 20:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 168 — One-Sqrt × Twenty-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 20:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 20:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 169 — Two-Sqrt × Twenty-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 20:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 20:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 170 — Three-Sqrt × Twenty-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 20:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 20:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 171 — Four-Sqrt × Twenty-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 20:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 20:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 172 — Five-Sqrt × Twenty-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Twenty-Log family (Phases 167-172).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 20:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 20:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly
    twenty-one** ``Log(diverging-in-k)`` factors, any polynomial factors, and
    any bounded factors; ``None`` otherwise.

    Phase 173 — Twenty-One-Log × polynomial numerator.

    Effective growth:
    ``log(k)^21 · k^m``.  ``log^21(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick (no Sqrt factors present):
    ``effective_x2 = 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    Sqrt factors are explicitly refused so that this function does not
    shadow the Sqrt-bearing phases (Phase 174 onward).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 21:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 21:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 174 — One-Sqrt × Twenty-One-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 21:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 21:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 175 — Two-Sqrt × Twenty-One-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 21:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 21:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 176 — Three-Sqrt × Twenty-One-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 21:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 21:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 177 — Four-Sqrt × Twenty-One-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 21:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 21:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 178 — Five-Sqrt × Twenty-One-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Twenty-One-Log family (Phases 173-178).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 21:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 21:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``2·poly_deg`` when ``node`` is a ``Mul`` with **exactly
    twenty-two** ``Log(diverging-in-k)`` factors, any polynomial factors, and
    any bounded factors; ``None`` otherwise.

    Phase 179 — Twenty-Two-Log × polynomial numerator.
    ``log(k)^22`` is sub-polynomial; ``effective_x2 = 2·poly_deg``.
    Sqrt factors are explicitly refused.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 22:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 22:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 180 — One-Sqrt × Twenty-Two-Log × polynomial numerator.
    effective_x2 = sqrt_deg_x2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 22:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 22:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 181 — Two-Sqrt × Twenty-Two-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 22:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 22:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 182 — Three-Sqrt × Twenty-Two-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 22:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 22:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 183 — Four-Sqrt × Twenty-Two-Log × polynomial numerator.
    effective_x2 = sqrt1 + sqrt2 + sqrt3 + sqrt4 + 2·poly_deg.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 22:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 22:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 184 — Five-Sqrt × Twenty-Two-Log × polynomial numerator.
    effective_x2 = sqrt1+sqrt2+sqrt3+sqrt4+sqrt5 + 2·poly_deg.
    Completes the Twenty-Two-Log family (Phases 179-184).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 22:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 22:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 185 — Twenty-Three-Log × polynomial numerator.
    ``log(k)^23`` is sub-polynomial; ``effective_x2 = 2·poly_deg``.
    Sqrt factors explicitly refused.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 23:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 23:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 186 — One-Sqrt × Twenty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 23:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 23:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 187 — Two-Sqrt × Twenty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 23:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 23:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 188 — Three-Sqrt × Twenty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 23:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 23:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 189 — Four-Sqrt × Twenty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 23:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 23:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 190 — Five-Sqrt × Twenty-Three-Log × polynomial numerator.
    Completes the Twenty-Three-Log family (Phases 185-190).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 23:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 23:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 191 — Zero-Sqrt × Twenty-Four-Log × polynomial numerator.

    The Twenty-Four-Log family (Phases 191–196) extends the recogniser to
    summands whose numerator contains exactly twenty-four logarithmic factors
    and zero to five square-root factors.  This is Phase 191: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 24:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 24:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 192 — One-Sqrt × Twenty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 24:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 24:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 193 — Two-Sqrt × Twenty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 24:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 24:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 194 — Three-Sqrt × Twenty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 24:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 24:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 195 — Four-Sqrt × Twenty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 24:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 24:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 196 — Five-Sqrt × Twenty-Four-Log × polynomial numerator.
    Completes the Twenty-Four-Log family (Phases 191-196).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 24:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 24:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 197 — Zero-Sqrt × Twenty-Five-Log × polynomial numerator.

    The Twenty-Five-Log family (Phases 197–202) extends the recogniser to
    summands whose numerator contains exactly twenty-five logarithmic factors
    and zero to five square-root factors.  This is Phase 197: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 25:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 25:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 198 — One-Sqrt × Twenty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 25:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 25:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 199 — Two-Sqrt × Twenty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 25:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 25:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 200 — Three-Sqrt × Twenty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 25:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 25:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 201 — Four-Sqrt × Twenty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 25:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 25:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 202 — Five-Sqrt × Twenty-Five-Log × polynomial numerator.
    Completes the Twenty-Five-Log family (Phases 197-202).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 25:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 25:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 203 — Zero-Sqrt × Twenty-Six-Log × polynomial numerator.

    The Twenty-Six-Log family (Phases 203–208) extends the recogniser to
    summands whose numerator contains exactly twenty-six logarithmic factors
    and zero to five square-root factors.  This is Phase 203: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 26:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 26:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 204 — One-Sqrt × Twenty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 26:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 26:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 205 — Two-Sqrt × Twenty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 26:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 26:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 206 — Three-Sqrt × Twenty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 26:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 26:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 207 — Four-Sqrt × Twenty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 26:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 26:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 208 — Five-Sqrt × Twenty-Six-Log × polynomial numerator.
    Completes the Twenty-Six-Log family (Phases 203-208).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 26:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 26:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 215 — Zero-Sqrt × Twenty-Eight-Log × polynomial numerator.

    The Twenty-Eight-Log family (Phases 215–220) extends the recogniser to
    summands whose numerator contains exactly twenty-eight logarithmic factors
    and zero to five square-root factors.  This is Phase 215: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 28:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 28:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 216 — One-Sqrt × Twenty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 28:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 28:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 217 — Two-Sqrt × Twenty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 28:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 28:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 218 — Three-Sqrt × Twenty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 28:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 28:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 219 — Four-Sqrt × Twenty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 28:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 28:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 220 — Five-Sqrt × Twenty-Eight-Log × polynomial numerator.
    Completes the Twenty-Eight-Log family (Phases 215-220).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 28:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 28:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 221 — Zero-Sqrt × Twenty-Nine-Log × polynomial numerator.

    The Twenty-Nine-Log family (Phases 221–226) extends the recogniser to
    summands whose numerator contains exactly twenty-nine logarithmic factors
    and zero to five square-root factors.  This is Phase 221: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 29:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 29:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 222 — One-Sqrt × Twenty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 29:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 29:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 223 — Two-Sqrt × Twenty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 29:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 29:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 224 — Three-Sqrt × Twenty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 29:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 29:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 225 — Four-Sqrt × Twenty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 29:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 29:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 226 — Five-Sqrt × Twenty-Nine-Log × polynomial numerator.
    Completes the Twenty-Nine-Log family (Phases 221-226).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 29:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 29:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 233 — Zero-Sqrt × Thirty-One-Log × polynomial numerator.

    The Thirty-One-Log family (Phases 233–238) extends the recogniser to
    summands whose numerator contains exactly thirty-one logarithmic factors
    and zero to five square-root factors.  This is Phase 233: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 31:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 31:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 234 — One-Sqrt × Thirty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 31:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 31:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 235 — Two-Sqrt × Thirty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 31:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 31:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 236 — Three-Sqrt × Thirty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 31:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 31:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 237 — Four-Sqrt × Thirty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 31:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 31:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 238 — Five-Sqrt × Thirty-One-Log × polynomial numerator.
    Completes the Thirty-One-Log family (Phases 233-238).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 31:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 31:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 257 — Zero-Sqrt × Thirty-Five-Log × polynomial numerator.

    The Thirty-Five-Log family (Phases 257–262) extends the recogniser to
    summands whose numerator contains exactly thirty-five logarithmic factors
    and zero to five square-root factors.  This is Phase 257: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 35:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 35:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 258 — One-Sqrt × Thirty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 35:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 35:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 259 — Two-Sqrt × Thirty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 35:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 35:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 260 — Three-Sqrt × Thirty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 35:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 35:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 261 — Four-Sqrt × Thirty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 35:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 35:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 262 — Five-Sqrt × Thirty-Five-Log × polynomial numerator.
    Completes the Thirty-Five-Log family (Phases 257-262).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 35:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 35:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 263 — Zero-Sqrt × Thirty-Six-Log × polynomial numerator.

    The Thirty-Six-Log family (Phases 263–268) extends the recogniser to
    summands whose numerator contains exactly thirty-six logarithmic factors
    and zero to five square-root factors.  This is Phase 263: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 36:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 36:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 264 — One-Sqrt × Thirty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 36:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 36:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 265 — Two-Sqrt × Thirty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 36:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 36:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 266 — Three-Sqrt × Thirty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 36:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 36:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 267 — Four-Sqrt × Thirty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 36:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 36:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 268 — Five-Sqrt × Thirty-Six-Log × polynomial numerator.
    Completes the Thirty-Six-Log family (Phases 263-268).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 36:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 36:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 269 — Zero-Sqrt × Thirty-Seven-Log × polynomial numerator.

    The Thirty-Seven-Log family (Phases 269–274) extends the recogniser to
    summands whose numerator contains exactly thirty-seven logarithmic factors
    and zero to five square-root factors.  This is Phase 269: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 37:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 37:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 270 — One-Sqrt × Thirty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 37:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 37:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 271 — Two-Sqrt × Thirty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 37:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 37:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 272 — Three-Sqrt × Thirty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 37:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 37:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 273 — Four-Sqrt × Thirty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 37:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 37:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 274 — Five-Sqrt × Thirty-Seven-Log × polynomial numerator.
    Completes the Thirty-Seven-Log family (Phases 269-274).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 37:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 37:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 275 — Zero-Sqrt × Thirty-Eight-Log × polynomial numerator.

    The Thirty-Eight-Log family (Phases 275–280) extends the recogniser to
    summands whose numerator contains exactly thirty-eight logarithmic factors
    and zero to five square-root factors.  This is Phase 275: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 38:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 38:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 276 — One-Sqrt × Thirty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 38:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 38:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 277 — Two-Sqrt × Thirty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 38:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 38:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 278 — Three-Sqrt × Thirty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 38:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 38:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 279 — Four-Sqrt × Thirty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 38:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 38:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 280 — Five-Sqrt × Thirty-Eight-Log × polynomial numerator.
    Completes the Thirty-Eight-Log family (Phases 275-280).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 38:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 38:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 281 — Zero-Sqrt × Thirty-Nine-Log × polynomial numerator.

    The Thirty-Nine-Log family (Phases 281–286) extends the recogniser to
    summands whose numerator contains exactly thirty-nine logarithmic factors
    and zero to five square-root factors.  This is Phase 281: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 39:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 39:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 282 — One-Sqrt × Thirty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 39:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 39:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 283 — Two-Sqrt × Thirty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 39:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 39:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 284 — Three-Sqrt × Thirty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 39:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 39:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 285 — Four-Sqrt × Thirty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 39:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 39:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 286 — Five-Sqrt × Thirty-Nine-Log × polynomial numerator.
    Completes the Thirty-Nine-Log family (Phases 281-286).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 39:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 39:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 287 — Zero-Sqrt × Forty-Log × polynomial numerator.

    The Forty-Log family (Phases 287–292) extends the recogniser to
    summands whose numerator contains exactly forty logarithmic factors
    and zero to five square-root factors.  This is Phase 287: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 40:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 40:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 288 — One-Sqrt × Forty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 40:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 40:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 289 — Two-Sqrt × Forty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 40:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 40:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 290 — Three-Sqrt × Forty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 40:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 40:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 291 — Four-Sqrt × Forty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 40:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 40:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 292 — Five-Sqrt × Forty-Log × polynomial numerator.
    Completes the Forty-Log family (Phases 287-292).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 40:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 40:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 299 — Zero-Sqrt × Forty-Two-Log × polynomial numerator.

    The Forty-Two-Log family (Phases 299–304) extends the recogniser to
    summands whose numerator contains exactly forty-two logarithmic factors
    and zero to five square-root factors.  This is Phase 299: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 42:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 42:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 300 — One-Sqrt × Forty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 42:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 42:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 301 — Two-Sqrt × Forty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 42:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 42:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 302 — Three-Sqrt × Forty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 42:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 42:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 303 — Four-Sqrt × Forty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 42:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 42:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 304 — Five-Sqrt × Forty-Two-Log × polynomial numerator.
    Completes the Forty-Two-Log family (Phases 299-304).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 42:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 42:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 305 — Zero-Sqrt × Forty-Three-Log × polynomial numerator.

    The Forty-Three-Log family (Phases 305–310) extends the recogniser to
    summands whose numerator contains exactly forty-three logarithmic factors
    and zero to five square-root factors.  This is Phase 305: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 43:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 43:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 306 — One-Sqrt × Forty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 43:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 43:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 307 — Two-Sqrt × Forty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 43:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 43:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 308 — Three-Sqrt × Forty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 43:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 43:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 309 — Four-Sqrt × Forty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 43:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 43:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 310 — Five-Sqrt × Forty-Three-Log × polynomial numerator.
    Completes the Forty-Three-Log family (Phases 305-310).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 43:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 43:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 353 — Zero-Sqrt × Fifty-One-Log × polynomial numerator.

    The Fifty-One-Log family (Phases 353–358) extends the recogniser to
    summands whose numerator contains exactly fifty-one logarithmic factors
    and zero to five square-root factors.  This is Phase 353: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 51:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 51:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 354 — One-Sqrt × Fifty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 51:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 51:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 355 — Two-Sqrt × Fifty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 51:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 51:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 356 — Three-Sqrt × Fifty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 51:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 51:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 357 — Four-Sqrt × Fifty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 51:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 51:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 358 — Five-Sqrt × Fifty-One-Log × polynomial numerator.
    Completes the Fifty-One-Log family (Phases 353-358).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 51:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 51:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 365 — Zero-Sqrt × Fifty-Three-Log × polynomial numerator.

    The Fifty-Three-Log family (Phases 365–370) extends the recogniser to
    summands whose numerator contains exactly fifty-three logarithmic factors
    and zero to five square-root factors.  This is Phase 365: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 53:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 53:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 366 — One-Sqrt × Fifty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 53:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 53:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 367 — Two-Sqrt × Fifty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 53:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 53:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 368 — Three-Sqrt × Fifty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 53:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 53:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 369 — Four-Sqrt × Fifty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 53:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 53:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 370 — Five-Sqrt × Fifty-Three-Log × polynomial numerator.
    Completes the Fifty-Three-Log family (Phases 365-370).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 53:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 53:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 383 — Zero-Sqrt × Fifty-Six-Log × polynomial numerator.

    The Fifty-Six-Log family (Phases 383–388) extends the recogniser to
    summands whose numerator contains exactly fifty-six logarithmic factors
    and zero to five square-root factors.  This is Phase 383: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 56:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 56:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 384 — One-Sqrt × Fifty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 56:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 56:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 385 — Two-Sqrt × Fifty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 56:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 56:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 386 — Three-Sqrt × Fifty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 56:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 56:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 387 — Four-Sqrt × Fifty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 56:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 56:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 388 — Five-Sqrt × Fifty-Six-Log × polynomial numerator.
    Completes the Fifty-Six-Log family (Phases 383-388).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 56:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 56:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 389 — Zero-Sqrt × Fifty-Seven-Log × polynomial numerator.

    The Fifty-Seven-Log family (Phases 389–394) extends the recogniser to
    summands whose numerator contains exactly fifty-seven logarithmic factors
    and zero to five square-root factors.  This is Phase 389: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 57:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 57:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 390 — One-Sqrt × Fifty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 57:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 57:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 391 — Two-Sqrt × Fifty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 57:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 57:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 392 — Three-Sqrt × Fifty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 57:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 57:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 393 — Four-Sqrt × Fifty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 57:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 57:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 394 — Five-Sqrt × Fifty-Seven-Log × polynomial numerator.
    Completes the Fifty-Seven-Log family (Phases 389-394).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 57:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 57:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _sixty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 419 — Zero-Sqrt × Sixty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 62:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 62:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_sixty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 420 — One-Sqrt × Sixty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 62:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 62:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_sixty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 421 — Two-Sqrt × Sixty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 62:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 62:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_sixty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 422 — Three-Sqrt × Sixty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 62:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 62:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_sixty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 423 — Four-Sqrt × Sixty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 62:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 62:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_sixty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 424 — Five-Sqrt × Sixty-Two-Log × polynomial numerator.
    Completes the Sixty-Two-Log family (Phases 419-424).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 62:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 62:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _sixty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 413 — Zero-Sqrt × Sixty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 61:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 61:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_sixty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 414 — One-Sqrt × Sixty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 61:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 61:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_sixty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 415 — Two-Sqrt × Sixty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 61:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 61:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_sixty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 416 — Three-Sqrt × Sixty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 61:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 61:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_sixty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 417 — Four-Sqrt × Sixty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 61:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 61:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_sixty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 418 — Five-Sqrt × Sixty-One-Log × polynomial numerator.
    Completes the Sixty-One-Log family (Phases 413-418).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 61:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 61:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _sixty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 407 — Zero-Sqrt × Sixty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 60:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 60:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_sixty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 408 — One-Sqrt × Sixty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 60:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 60:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_sixty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 409 — Two-Sqrt × Sixty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 60:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 60:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_sixty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 410 — Three-Sqrt × Sixty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 60:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 60:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_sixty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 411 — Four-Sqrt × Sixty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 60:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 60:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_sixty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 412 — Five-Sqrt × Sixty-Log × polynomial numerator.
    Completes the Sixty-Log family (Phases 407-412).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 60:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 60:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 401 — Zero-Sqrt × Fifty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 59:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 59:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 402 — One-Sqrt × Fifty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 59:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 59:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 403 — Two-Sqrt × Fifty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 59:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 59:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 404 — Three-Sqrt × Fifty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 59:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 59:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 405 — Four-Sqrt × Fifty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 59:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 59:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 406 — Five-Sqrt × Fifty-Nine-Log × polynomial numerator.
    Completes the Fifty-Nine-Log family (Phases 401-406).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 59:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 59:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 395 — Zero-Sqrt × Fifty-Eight-Log × polynomial numerator.

    The Fifty-Eight-Log family (Phases 395–400) extends the recogniser to
    summands whose numerator contains exactly fifty-eight logarithmic factors
    and zero to five square-root factors.  This is Phase 395: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 58:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 58:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 396 — One-Sqrt × Fifty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 58:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 58:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 397 — Two-Sqrt × Fifty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 58:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 58:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 398 — Three-Sqrt × Fifty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 58:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 58:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 399 — Four-Sqrt × Fifty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 58:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 58:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 400 — Five-Sqrt × Fifty-Eight-Log × polynomial numerator.
    Completes the Fifty-Eight-Log family (Phases 395-400).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 58:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 58:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 377 — Zero-Sqrt × Fifty-Five-Log × polynomial numerator.

    The Fifty-Five-Log family (Phases 377–382) extends the recogniser to
    summands whose numerator contains exactly fifty-five logarithmic factors
    and zero to five square-root factors.  This is Phase 377: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 55:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 55:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 378 — One-Sqrt × Fifty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 55:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 55:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 379 — Two-Sqrt × Fifty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 55:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 55:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 380 — Three-Sqrt × Fifty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 55:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 55:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 381 — Four-Sqrt × Fifty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 55:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 55:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 382 — Five-Sqrt × Fifty-Five-Log × polynomial numerator.
    Completes the Fifty-Five-Log family (Phases 377-382).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 55:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 55:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 371 — Zero-Sqrt × Fifty-Four-Log × polynomial numerator.

    The Fifty-Four-Log family (Phases 371–376) extends the recogniser to
    summands whose numerator contains exactly fifty-four logarithmic factors
    and zero to five square-root factors.  This is Phase 371: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 54:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 54:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 372 — One-Sqrt × Fifty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 54:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 54:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 373 — Two-Sqrt × Fifty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 54:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 54:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 374 — Three-Sqrt × Fifty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 54:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 54:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 375 — Four-Sqrt × Fifty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 54:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 54:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 376 — Five-Sqrt × Fifty-Four-Log × polynomial numerator.
    Completes the Fifty-Four-Log family (Phases 371-376).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 54:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 54:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 359 — Zero-Sqrt × Fifty-Two-Log × polynomial numerator.

    The Fifty-Two-Log family (Phases 359–364) extends the recogniser to
    summands whose numerator contains exactly fifty-two logarithmic factors
    and zero to five square-root factors.  This is Phase 359: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 52:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 52:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 360 — One-Sqrt × Fifty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 52:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 52:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 361 — Two-Sqrt × Fifty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 52:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 52:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 362 — Three-Sqrt × Fifty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 52:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 52:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 363 — Four-Sqrt × Fifty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 52:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 52:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 364 — Five-Sqrt × Fifty-Two-Log × polynomial numerator.
    Completes the Fifty-Two-Log family (Phases 359-364).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 52:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 52:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _fifty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 347 — Zero-Sqrt × Fifty-Log × polynomial numerator.

    The Fifty-Log family (Phases 347–352) extends the recogniser to
    summands whose numerator contains exactly fifty logarithmic factors
    and zero to five square-root factors.  This is Phase 347: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 50:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 50:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_fifty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 348 — One-Sqrt × Fifty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 50:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 50:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_fifty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 349 — Two-Sqrt × Fifty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 50:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 50:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_fifty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 350 — Three-Sqrt × Fifty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 50:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 50:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_fifty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 351 — Four-Sqrt × Fifty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 50:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 50:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_fifty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 352 — Five-Sqrt × Fifty-Log × polynomial numerator.
    Completes the Fifty-Log family (Phases 347-352).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 50:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 50:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 341 — Zero-Sqrt × Forty-Nine-Log × polynomial numerator.

    The Forty-Nine-Log family (Phases 341–346) extends the recogniser to
    summands whose numerator contains exactly forty-nine logarithmic factors
    and zero to five square-root factors.  This is Phase 341: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 49:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 49:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 342 — One-Sqrt × Forty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 49:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 49:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 343 — Two-Sqrt × Forty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 49:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 49:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 344 — Three-Sqrt × Forty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 49:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 49:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 345 — Four-Sqrt × Forty-Nine-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 49:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 49:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_nine_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 346 — Five-Sqrt × Forty-Nine-Log × polynomial numerator.
    Completes the Forty-Nine-Log family (Phases 341-346).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 49:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 49:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 335 — Zero-Sqrt × Forty-Eight-Log × polynomial numerator.

    The Forty-Eight-Log family (Phases 335–340) extends the recogniser to
    summands whose numerator contains exactly forty-eight logarithmic factors
    and zero to five square-root factors.  This is Phase 335: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 48:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 48:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 336 — One-Sqrt × Forty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 48:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 48:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 337 — Two-Sqrt × Forty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 48:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 48:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 338 — Three-Sqrt × Forty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 48:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 48:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 339 — Four-Sqrt × Forty-Eight-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 48:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 48:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_eight_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 340 — Five-Sqrt × Forty-Eight-Log × polynomial numerator.
    Completes the Forty-Eight-Log family (Phases 335-340).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 48:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 48:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 329 — Zero-Sqrt × Forty-Seven-Log × polynomial numerator.

    The Forty-Seven-Log family (Phases 329–334) extends the recogniser to
    summands whose numerator contains exactly forty-seven logarithmic factors
    and zero to five square-root factors.  This is Phase 329: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 47:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 47:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 330 — One-Sqrt × Forty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 47:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 47:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 331 — Two-Sqrt × Forty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 47:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 47:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 332 — Three-Sqrt × Forty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 47:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 47:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 333 — Four-Sqrt × Forty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 47:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 47:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 334 — Five-Sqrt × Forty-Seven-Log × polynomial numerator.
    Completes the Forty-Seven-Log family (Phases 329-334).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 47:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 47:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 323 — Zero-Sqrt × Forty-Six-Log × polynomial numerator.

    The Forty-Six-Log family (Phases 323–328) extends the recogniser to
    summands whose numerator contains exactly forty-six logarithmic factors
    and zero to five square-root factors.  This is Phase 323: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 46:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 46:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 324 — One-Sqrt × Forty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 46:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 46:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 325 — Two-Sqrt × Forty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 46:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 46:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 326 — Three-Sqrt × Forty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 46:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 46:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 327 — Four-Sqrt × Forty-Six-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 46:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 46:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 328 — Five-Sqrt × Forty-Six-Log × polynomial numerator.
    Completes the Forty-Six-Log family (Phases 323-328).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 46:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 46:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 317 — Zero-Sqrt × Forty-Five-Log × polynomial numerator.

    The Forty-Five-Log family (Phases 317–322) extends the recogniser to
    summands whose numerator contains exactly forty-five logarithmic factors
    and zero to five square-root factors.  This is Phase 317: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 45:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 45:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 318 — One-Sqrt × Forty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 45:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 45:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 319 — Two-Sqrt × Forty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 45:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 45:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 320 — Three-Sqrt × Forty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 45:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 45:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 321 — Four-Sqrt × Forty-Five-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 45:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 45:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_five_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 322 — Five-Sqrt × Forty-Five-Log × polynomial numerator.
    Completes the Forty-Five-Log family (Phases 317-322).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 45:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 45:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 311 — Zero-Sqrt × Forty-Four-Log × polynomial numerator.

    The Forty-Four-Log family (Phases 311–316) extends the recogniser to
    summands whose numerator contains exactly forty-four logarithmic factors
    and zero to five square-root factors.  This is Phase 311: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 44:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 44:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 312 — One-Sqrt × Forty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 44:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 44:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 313 — Two-Sqrt × Forty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 44:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 44:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 314 — Three-Sqrt × Forty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 44:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 44:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 315 — Four-Sqrt × Forty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 44:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 44:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 316 — Five-Sqrt × Forty-Four-Log × polynomial numerator.
    Completes the Forty-Four-Log family (Phases 311-316).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 44:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 44:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _forty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 293 — Zero-Sqrt × Forty-One-Log × polynomial numerator.

    The Forty-One-Log family (Phases 293–298) extends the recogniser to
    summands whose numerator contains exactly forty-one logarithmic factors
    and zero to five square-root factors.  This is Phase 293: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 41:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 41:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_forty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 294 — One-Sqrt × Forty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 41:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 41:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_forty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 295 — Two-Sqrt × Forty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 41:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 41:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_forty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 296 — Three-Sqrt × Forty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 41:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 41:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_forty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 297 — Four-Sqrt × Forty-One-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 41:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 41:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_forty_one_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 298 — Five-Sqrt × Forty-One-Log × polynomial numerator.
    Completes the Forty-One-Log family (Phases 293-298).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 41:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 41:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 251 — Zero-Sqrt × Thirty-Four-Log × polynomial numerator.

    The Thirty-Four-Log family (Phases 251–256) extends the recogniser to
    summands whose numerator contains exactly thirty-four logarithmic factors
    and zero to five square-root factors.  This is Phase 251: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 34:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 34:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 252 — One-Sqrt × Thirty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 34:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 34:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 253 — Two-Sqrt × Thirty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 34:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 34:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 254 — Three-Sqrt × Thirty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 34:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 34:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 255 — Four-Sqrt × Thirty-Four-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 34:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 34:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_four_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 256 — Five-Sqrt × Thirty-Four-Log × polynomial numerator.
    Completes the Thirty-Four-Log family (Phases 251-256).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 34:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 34:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 245 — Zero-Sqrt × Thirty-Three-Log × polynomial numerator.

    The Thirty-Three-Log family (Phases 245–250) extends the recogniser to
    summands whose numerator contains exactly thirty-three logarithmic factors
    and zero to five square-root factors.  This is Phase 245: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 33:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 33:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 246 — One-Sqrt × Thirty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 33:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 33:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 247 — Two-Sqrt × Thirty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 33:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 33:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 248 — Three-Sqrt × Thirty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 33:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 33:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 249 — Four-Sqrt × Thirty-Three-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 33:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 33:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_three_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 250 — Five-Sqrt × Thirty-Three-Log × polynomial numerator.
    Completes the Thirty-Three-Log family (Phases 245-250).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 33:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 33:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 239 — Zero-Sqrt × Thirty-Two-Log × polynomial numerator.

    The Thirty-Two-Log family (Phases 239–244) extends the recogniser to
    summands whose numerator contains exactly thirty-two logarithmic factors
    and zero to five square-root factors.  This is Phase 239: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 32:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 32:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 240 — One-Sqrt × Thirty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 32:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 32:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 241 — Two-Sqrt × Thirty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 32:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 32:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 242 — Three-Sqrt × Thirty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 32:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 32:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 243 — Four-Sqrt × Thirty-Two-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 32:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 32:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_two_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 244 — Five-Sqrt × Thirty-Two-Log × polynomial numerator.
    Completes the Thirty-Two-Log family (Phases 239-244).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 32:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 32:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _thirty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 227 — Zero-Sqrt × Thirty-Log × polynomial numerator.

    The Thirty-Log family (Phases 227–232) extends the recogniser to
    summands whose numerator contains exactly thirty logarithmic factors
    and zero to five square-root factors.  This is Phase 227: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 30:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 30:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_thirty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 228 — One-Sqrt × Thirty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 30:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 30:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_thirty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 229 — Two-Sqrt × Thirty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 30:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 30:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_thirty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 230 — Three-Sqrt × Thirty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 30:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 30:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_thirty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 231 — Four-Sqrt × Thirty-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 30:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 30:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_thirty_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 232 — Five-Sqrt × Thirty-Log × polynomial numerator.
    Completes the Thirty-Log family (Phases 227-232).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 30:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 30:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _twenty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 209 — Zero-Sqrt × Twenty-Seven-Log × polynomial numerator.

    The Twenty-Seven-Log family (Phases 209–214) extends the recogniser to
    summands whose numerator contains exactly twenty-seven logarithmic factors
    and zero to five square-root factors.  This is Phase 209: zero sqrts.

    Parameters
    ----------
    node : IRNode
        The numerator node.
    k : IRSymbol
        The summation variable.

    Returns
    -------
    int | None
        ``2 * poly_deg_sum`` when the shape matches (always even); ``None``
        when the pattern is not recognised.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        if _sqrt_effective_half_degree_x2(arg, k) is not None:
            return None
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 27:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if log_count != 27:
        return None
    return 2 * poly_deg_sum


def _one_sqrt_twenty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 210 — One-Sqrt × Twenty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if sqrt_deg_x2 is not None:
                return None
            sqrt_deg_x2 = deg_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 27:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 27:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_twenty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 211 — Two-Sqrt × Twenty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 27:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 27:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _three_sqrt_twenty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 212 — Three-Sqrt × Twenty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 3:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 27:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 3 or log_count != 27:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _four_sqrt_twenty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 213 — Four-Sqrt × Twenty-Seven-Log × polynomial numerator."""
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 4:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 27:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 4 or log_count != 27:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _five_sqrt_twenty_seven_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Phase 214 — Five-Sqrt × Twenty-Seven-Log × polynomial numerator.
    Completes the Twenty-Seven-Log family (Phases 209-214).
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 5:
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 27:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 5 or log_count != 27:
        return None
    return sum(sqrt_degs_x2) + 2 * poly_deg_sum


def _one_sqrt_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul`` with
    **exactly one** ``Sqrt(positive-polynomial)`` factor, **exactly six**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded
    factors; ``None`` otherwise.

    Phase 84 — One-Sqrt × Six-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · log(k)⁶ · k^m``.  ``log⁶(k)`` is sub-polynomial (``o(k^ε)``),
    contributing 0 to the effective polynomial degree.
    Using the ×2 integer trick: ``effective_x2 = a + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +----------------------------------------------------------+----------+
    | Input                                                    | Return   |
    +==========================================================+==========+
    | ``Mul(Sqrt(k), Log(k)×6)``                               | ``1``    |
    | ``Mul(Sqrt(k²), Log(k)×6, k)``                           | ``4``    |
    | ``Mul(Sqrt(k), Log(k)×6, k², 3)``                        | ``5``    |
    | ``Mul(Log(k)×6)``                                        | None     |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×6)``                      | None (2) |
    | ``Mul(Sqrt(k), Log(k)×5)``                               | None (5) |
    +----------------------------------------------------------+----------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(P)`` → record ``sqrt_effective_half_degree_x2``; bail on second Sqrt.
         - ``Log(diverging)`` → count; bail after 6.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 1 Sqrt and exactly 6 Log factors.
      4. Return ``sqrt_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_deg_x2: int | None = None
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        s_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if s_x2 is not None:
            if sqrt_deg_x2 is not None:
                # Second Sqrt — not this phase.
                return None
            sqrt_deg_x2 = s_x2
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 6:
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if sqrt_deg_x2 is None or log_count != 6:
        return None
    return sqrt_deg_x2 + 2 * poly_deg_sum


def _two_sqrt_six_log_poly_effective_x2(node: IRNode, k: IRSymbol) -> int | None:
    """Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2·poly_deg`` when ``node`` is a ``Mul``
    with **exactly two** ``Sqrt(positive-leading polynomial)`` factors, **exactly six**
    ``Log(diverging-in-k)`` factors, any polynomial factors, and any bounded factors;
    ``None`` otherwise.

    Phase 85 — Two-Sqrt × Six-Log × polynomial numerator.

    Effective growth:
    ``sqrt(k^a) · sqrt(k^b) · log(k)⁶ · k^m ≈ k^{(a+b)/2} · log⁶(k) · k^m``.
    ``log⁶(k)`` is sub-polynomial (``o(k^ε)``), contributing 0.
    Using the ×2 integer trick:
    ``effective_x2 = a + b + 2·m``.
    Caller checks ``2·den_deg > effective_x2``.

    +------------------------------------------------------------------+---------------------+
    | Input                                                            | Return              |
    +==================================================================+=====================+
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×6)``                              | ``1 + 1 = 2``       |
    | ``Mul(Sqrt(k³), Sqrt(k³), Log(k)×6)``                           | ``3 + 3 = 6``       |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×6, k)``                          | ``1+1+2 = 4``       |
    | ``Mul(Sqrt(k), Log(k)×6)``                                       | None (1 Sqrt)       |
    | ``Mul(Sqrt(k)×3, Log(k)×6)``                                     | None (3 Sqrts)      |
    | ``Mul(Sqrt(k), Sqrt(k), Log(k)×5)``                              | None (5 Logs)       |
    +------------------------------------------------------------------+---------------------+

    Algorithm:
      1. Require ``node = Mul(...)``.
      2. For each factor:
         - ``Sqrt(positive-leading polynomial)`` → record ×2 degree; bail after 2.
         - ``Log(diverging)`` → count; bail after 6.
         - Polynomial in ``k`` → accumulate degree.
         - Bounded → accept silently.
         - Anything else → return ``None``.
      3. Require exactly 2 Sqrt AND exactly 6 Log factors.
      4. Return ``sqrt1_deg_x2 + sqrt2_deg_x2 + 2 * poly_deg_sum``.
    """
    if not isinstance(node, IRApply) or node.head != MUL:
        return None
    sqrt_degs_x2: list[int] = []
    log_count: int = 0
    poly_deg_sum: int = 0
    for arg in node.args:
        deg_x2 = _sqrt_effective_half_degree_x2(arg, k)
        if deg_x2 is not None:
            if len(sqrt_degs_x2) >= 2:
                # Third Sqrt factor — not this phase.
                return None
            sqrt_degs_x2.append(deg_x2)
            continue
        if _is_log_of_diverging_in_k(arg, k):
            log_count += 1
            if log_count > 6:
                # Seven or more Log factors — refuse.
                return None
            continue
        deg = _polynomial_degree_in_k(arg, k)
        if deg is not None:
            poly_deg_sum += deg
            continue
        if _is_bounded_in_k(arg, k):
            continue
        return None
    if len(sqrt_degs_x2) != 2 or log_count != 6:
        return None
    return sqrt_degs_x2[0] + sqrt_degs_x2[1] + 2 * poly_deg_sum


def _try_power_of_k(
    f: IRNode, k: IRSymbol
) -> tuple[Fraction, int] | None:
    """If f = coeff · k^m (m non-negative integer ≤ 5), return (coeff, m); else None.

    Handles:
    - k            → (Fraction(1), 1)
    - k^m          → (Fraction(1), m)
    - coeff * k^m  → (coeff, m)
    - coeff * k    → (coeff, 1)
    """
    # f == k  (i.e. k^1 with coeff=1)
    if f == k:
        return Fraction(1), 1

    # f = POW(k, m)
    if (
        isinstance(f, IRApply)
        and f.head == POW
        and len(f.args) == 2
        and f.args[0] == k
    ):
        m = _ir_int_val(f.args[1])
        if m is not None and 0 <= m <= 5:
            return Fraction(1), m

    # f = MUL(coeff, k) or MUL(k, coeff)
    if isinstance(f, IRApply) and f.head == MUL and len(f.args) == 2:
        a, b = f.args
        for coeff_cand, other in ((a, b), (b, a)):
            c = _ir_rational_val(coeff_cand)
            if c is None or not _is_constant_in(coeff_cand, k):
                continue
            # other must be k or POW(k, m)
            if other == k:
                return c, 1
            if (
                isinstance(other, IRApply)
                and other.head == POW
                and len(other.args) == 2
                and other.args[0] == k
            ):
                m = _ir_int_val(other.args[1])
                if m is not None and 0 <= m <= 5:
                    return c, m

    return None


# ---------------------------------------------------------------------------
# Main public functions
# ---------------------------------------------------------------------------


def evaluate_sum(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
    vm: object,
) -> IRNode:
    """Evaluate Σ_{k=lo}^{hi} f(k) symbolically, or return unevaluated SUM.

    Parameters
    ----------
    f:
        The summand expression (may contain *k*).
    k:
        The index variable (an ``IRSymbol``).
    lo:
        Lower bound (already evaluated by the VM).
    hi:
        Upper bound (already evaluated by the VM).
    vm:
        The symbolic VM instance (used for sub-expression evaluation).

    Returns
    -------
    IRNode
        Closed-form result, or ``IRApply(SUM, (f, k, lo, hi))`` if
        no pattern matches.
    """
    inf_upper = _is_inf(hi)

    # ── 1. Constant summand ─────────────────────────────────────────────────
    if _is_constant_in(f, k):
        # Σ_{k=lo}^{hi} c = c * (hi - lo + 1)
        count = IRApply(ADD, (IRApply(SUB, (hi, lo)), _int(1)))
        return vm.eval(IRApply(MUL, (f, count)))

    # ── 2. Geometric series ─────────────────────────────────────────────────
    geo = _try_geometric(f, k)
    if geo is not None:
        coeff, base = geo
        raw = geometric_sum_ir(
            coeff=coeff,
            base=base,
            lo=lo,
            hi=hi,
            is_infinite=inf_upper,
        )
        return vm.eval(raw)

    # ── 3. Power of index (Faulhaber) ───────────────────────────────────────
    power = _try_power_of_k(f, k)
    if power is not None:
        coeff, m = power
        lo_int = _ir_int_val(lo)
        if lo_int is not None and lo_int >= 0 and not inf_upper:
            raw = poly_sum_ir(m=m, coeff=coeff, lo_val=lo_int, hi=hi)
            if raw is not None:
                return vm.eval(raw)

    # ── 4. Telescoping sums (Phase 39 finite + Phase 41 infinite) ──────────
    # Detect ``f = g(k+1) − g(k)`` (or its antisymmetric ``g(k) − g(k+1)``)
    # and emit a closed form.
    #
    # - **Phase 39 (finite range)**: ``∑_{k=lo}^{hi} [g(k+1) − g(k)] =
    #   g(hi+1) − g(lo)`` (and the antisymmetric mirror).
    # - **Phase 41 (infinite range)**: when ``hi`` is ``%inf`` AND ``g(k)``
    #   provably vanishes at infinity per :func:`_g_vanishes_at_infinity`,
    #   ``∑_{k=lo}^{∞} [g(k+1) − g(k)] = lim g − g(lo) = −g(lo)`` (and
    #   ``g(lo) − lim g = g(lo)`` for the antisymmetric case).  When
    #   the limit isn't decidable by the narrow recogniser, fall through
    #   to later rules — the original unevaluated SUM is then returned by
    #   the bottom of this function.
    tele = _try_telescoping(f, k, vm)
    if tele is not None:
        from cas_substitution import subst

        g_expr, sign = tele
        sign_val = _ir_int_val(sign)
        if inf_upper:
            # Phase 41: only close when we can prove the limit is 0.
            if _g_vanishes_at_infinity(g_expr, k):
                g_at_lo = subst(lo, k, g_expr)
                if sign_val == 1:
                    # ∑[g(k+1) − g(k)] from lo to ∞ = 0 − g(lo) = −g(lo)
                    return vm.eval(IRApply(NEG, (g_at_lo,)))
                # ∑[g(k) − g(k+1)] from lo to ∞ = g(lo) − 0 = g(lo)
                return vm.eval(g_at_lo)
            # Limit not provably zero — fall through so the original
            # unevaluated SUM is returned at the bottom.
        else:
            # Phase 39 (finite range) — bit-for-bit the same code path as
            # the original implementation.
            hi_plus_one = IRApply(ADD, (hi, IRInteger(1)))
            g_at_hi_plus_one = subst(hi_plus_one, k, g_expr)
            g_at_lo = subst(lo, k, g_expr)
            if sign_val == 1:
                # ∑[g(k+1) − g(k)] = g(hi+1) − g(lo)
                return vm.eval(IRApply(SUB, (g_at_hi_plus_one, g_at_lo)))
            # ∑[g(k) − g(k+1)] = g(lo) − g(hi+1)
            return vm.eval(IRApply(SUB, (g_at_lo, g_at_hi_plus_one)))

    # ── 5. Classic infinite series ──────────────────────────────────────────
    if inf_upper:
        result = try_special_infinite(f, k, lo)
        if result is not None:
            return vm.eval(result)

    # ── 6. Numeric small range ──────────────────────────────────────────────
    lo_int = _ir_int_val(lo)
    hi_int = _ir_int_val(hi)
    if lo_int is not None and hi_int is not None and 0 <= hi_int - lo_int <= 999:
        try:
            from cas_substitution import subst

            total = Fraction(0)
            for kv in range(lo_int, hi_int + 1):
                term = subst(IRInteger(kv), k, f)
                evaluated = vm.eval(term)
                r = _ir_rational_val(evaluated)
                if r is None:
                    total = None  # type: ignore[assignment]
                    break
                total += r
            if total is not None:
                return _frac(total)
        except Exception:
            pass

    # ── 7. Unevaluated ──────────────────────────────────────────────────────
    return IRApply(SUM, (f, k, lo, hi))


def evaluate_product(
    f: IRNode,
    k: IRSymbol,
    lo: IRNode,
    hi: IRNode,
    vm: object,
) -> IRNode:
    """Evaluate Π_{k=lo}^{hi} f(k) symbolically, or return unevaluated PRODUCT.

    Parameters
    ----------
    f:
        The factor expression (may contain *k*).
    k:
        The product index variable (an ``IRSymbol``).
    lo:
        Lower bound (already evaluated by the VM).
    hi:
        Upper bound (already evaluated by the VM).
    vm:
        The symbolic VM instance.

    Returns
    -------
    IRNode
        Closed-form result, or ``IRApply(PRODUCT, (f, k, lo, hi))`` if
        no pattern matches.
    """
    result = evaluate_product_expr(f, k, lo, hi, vm)
    if result is not None:
        return vm.eval(result)
    return IRApply(PRODUCT, (f, k, lo, hi))
