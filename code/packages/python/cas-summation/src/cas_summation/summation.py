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
    MUL,
    NEG,
    POW,
    PRODUCT,
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

    +-------------------------------+--------------------------+
    | ``g`` shape                   | Provably ``→ 0``?        |
    +===============================+==========================+
    | ``Div(constant, k+a)``        | yes (Phase 41)           |
    | ``Div(constant, k²·(k+1))``   | yes (Phase 41)           |
    | ``Div(k, k²+1)``              | yes (Phase 42)           |
    | ``Div(k+1, k³−5)``            | yes (Phase 42)           |
    | constant                      | no (limit is the const)  |
    | ``k`` or ``k²+1``             | no (limit is ∞)          |
    | ``1/sin(k)`` / ``log(k)/k``   | no (numerator is not a   |
    |                               |    polynomial; transcend- |
    |                               |    ental limits deferred) |
    +-------------------------------+--------------------------+

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
    # Phase 41 fast path: constant numerator + positive-degree denominator.
    if _is_constant_in(num, k):
        return _is_positive_degree_polynomial_in_k(den, k)
    # Phase 42 widening: deg(num) < deg(den) on pure polynomials in k.
    num_degree = _polynomial_degree_in_k(num, k)
    if num_degree is None:
        return False
    den_degree = _polynomial_degree_in_k(den, k)
    if den_degree is None:
        return False
    return num_degree < den_degree


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
