"""Hyperbolic power integration — Phase 14.

Computes antiderivatives for three families:

1. ``∫ sinh^n(ax+b) dx``  — via IBP reduction formula (any integer n ≥ 0)
2. ``∫ cosh^n(ax+b) dx``  — via IBP reduction formula (any integer n ≥ 0)
3. ``∫ sinh(ax+b) · cosh^n(ax+b) dx``  and
   ``∫ sinh^m(ax+b) · cosh(ax+b) dx``  — via u-substitution (any m, n ≥ 1)

Reduction formulas
------------------

**sinh^n (derivation)**

IBP with ``u = sinh^(n-1)(t)``, ``dv = sinh(t) dt``:

    v = cosh(t),  du = (n-1)·sinh^(n-2)(t)·cosh(t) dt

    ∫ sinh^n(t) dt = sinh^(n-1)(t)·cosh(t) − (n-1)·∫ sinh^(n-2)(t)·cosh²(t) dt
                   = sinh^(n-1)(t)·cosh(t) − (n-1)·∫ sinh^(n-2)(t)·(sinh²(t)+1) dt
                   = sinh^(n-1)(t)·cosh(t) − (n-1)·∫ sinh^n(t) dt
                                            − (n-1)·∫ sinh^(n-2)(t) dt

Solving for I_n = ∫ sinh^n(t) dt:

    n · I_n = sinh^(n-1)(t)·cosh(t) − (n-1)·I_{n-2}
    I_n = (1/n)·sinh^(n-1)(t)·cosh(t) − (n-1)/n · I_{n-2}

Base cases: ``I_0 = t``, ``I_1 = cosh(t)``.

With substitution ``t = ax+b``, ``dt = a·dx``:

    ∫ sinh^n(ax+b) dx = (1/a)·I_n(ax+b)
    = (1/(na))·sinh^(n-1)(ax+b)·cosh(ax+b) − (n-1)/n · ∫ sinh^(n-2)(ax+b) dx

**cosh^n (derivation)**

Analogous — only the base cases and sign of the recursive term change:

    ∫ cosh^n(t) dt = (1/n)·cosh^(n-1)(t)·sinh(t) + (n-1)/n · ∫ cosh^(n-2)(t) dt

(The ``+`` sign comes from ``d/dt[cosh(t)] = sinh(t)``.)

Base cases: ``I_0 = t``, ``I_1 = sinh(t)``.

With substitution:

    ∫ cosh^n(ax+b) dx = (1/(na))·cosh^(n-1)(ax+b)·sinh(ax+b)
                       + (n-1)/n · ∫ cosh^(n-2)(ax+b) dx

**sinh × cosh^n and sinh^m × cosh (u-substitution)**

When exactly one factor has exponent 1 the derivative relationship is
direct:

    ∫ sinh(ax+b) · cosh^n(ax+b) dx:
        Let u = cosh(ax+b), du = a·sinh(ax+b) dx.
        = (1/a) ∫ u^n du = u^(n+1) / ((n+1)·a)
        = cosh^(n+1)(ax+b) / ((n+1)·a)

    ∫ sinh^m(ax+b) · cosh(ax+b) dx:
        Let u = sinh(ax+b), du = a·cosh(ax+b) dx.
        = (1/a) ∫ u^m du = u^(m+1) / ((m+1)·a)
        = sinh^(m+1)(ax+b) / ((m+1)·a)

    ∫ sinh(ax+b) · cosh(ax+b) dx  (both n=m=1):
        Either formula gives  sinh²(ax+b) / (2a)
        (equivalently sinh(2(ax+b)) / (4a) — same result up to a constant).
"""

from __future__ import annotations

from fractions import Fraction
from math import comb as _comb

from symbolic_ir import (
    ADD,
    COSH,
    DIV,
    MUL,
    POW,
    SINH,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import linear_to_ir

# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def sinh_power_integral(
    n: int,
    a: Fraction,
    b: Fraction,
    x: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ sinh^n(ax+b) dx``.

    Uses the IBP reduction formula recursively:

        I_n = (1/(na))·sinh^(n-1)(ax+b)·cosh(ax+b) − (n-1)/n · I_{n-2}

    Base cases:

        I_0 = x
        I_1 = (1/a)·cosh(ax+b)

    Pre-conditions: ``n ≥ 0``, ``a ≠ 0``.

    Parameters
    ----------
    n :
        Non-negative integer power.
    a, b :
        Linear coefficients so the argument is ``ax+b``.
    x :
        Integration variable symbol.

    Returns
    -------
    IR node for the antiderivative.

    Examples
    --------
    ::

        # ∫ sinh²(x) dx = (1/2)·sinh(x)·cosh(x) − x/2
        sinh_power_integral(2, Fraction(1), Fraction(0), IRSymbol("x"))
    """
    if n == 0:
        return x
    arg_ir = linear_to_ir(a, b, x)
    sinh_ir = IRApply(SINH, (arg_ir,))
    cosh_ir = IRApply(COSH, (arg_ir,))
    if n == 1:
        # ∫ sinh(ax+b) dx = (1/a)·cosh(ax+b)
        if a == Fraction(1):
            return cosh_ir
        return IRApply(DIV, (cosh_ir, _frac_ir(a)))

    # n ≥ 2: I_n = (1/(na))·sinh^(n-1)·cosh − (n-1)/n · I_{n-2}
    coeff = Fraction(1, n) / a          # 1 / (n·a)
    sinh_pow = (
        sinh_ir
        if n - 1 == 1
        else IRApply(POW, (sinh_ir, IRInteger(n - 1)))
    )
    main_term = IRApply(MUL, (_frac_ir(coeff), IRApply(MUL, (sinh_pow, cosh_ir))))
    rec_coeff = Fraction(n - 1, n)      # (n-1)/n
    rec_term = sinh_power_integral(n - 2, a, b, x)
    sub_term = IRApply(MUL, (_frac_ir(rec_coeff), rec_term))
    return IRApply(SUB, (main_term, sub_term))


def cosh_power_integral(
    n: int,
    a: Fraction,
    b: Fraction,
    x: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ cosh^n(ax+b) dx``.

    Uses the IBP reduction formula recursively:

        I_n = (1/(na))·cosh^(n-1)(ax+b)·sinh(ax+b) + (n-1)/n · I_{n-2}

    Base cases:

        I_0 = x
        I_1 = (1/a)·sinh(ax+b)

    Pre-conditions: ``n ≥ 0``, ``a ≠ 0``.

    Note the ``+`` sign in the recursive term (contrast with
    :func:`sinh_power_integral` which has ``−``).  The sign difference
    comes from ``d/dt[cosh] = sinh`` vs ``d/dt[sinh] = cosh``.

    Parameters
    ----------
    n :
        Non-negative integer power.
    a, b :
        Linear coefficients so the argument is ``ax+b``.
    x :
        Integration variable symbol.

    Returns
    -------
    IR node for the antiderivative.

    Examples
    --------
    ::

        # ∫ cosh²(x) dx = (1/2)·cosh(x)·sinh(x) + x/2
        cosh_power_integral(2, Fraction(1), Fraction(0), IRSymbol("x"))
    """
    if n == 0:
        return x
    arg_ir = linear_to_ir(a, b, x)
    cosh_ir = IRApply(COSH, (arg_ir,))
    sinh_ir = IRApply(SINH, (arg_ir,))
    if n == 1:
        # ∫ cosh(ax+b) dx = (1/a)·sinh(ax+b)
        if a == Fraction(1):
            return sinh_ir
        return IRApply(DIV, (sinh_ir, _frac_ir(a)))

    # n ≥ 2: I_n = (1/(na))·cosh^(n-1)·sinh + (n-1)/n · I_{n-2}
    coeff = Fraction(1, n) / a
    cosh_pow = (
        cosh_ir
        if n - 1 == 1
        else IRApply(POW, (cosh_ir, IRInteger(n - 1)))
    )
    main_term = IRApply(MUL, (_frac_ir(coeff), IRApply(MUL, (cosh_pow, sinh_ir))))
    rec_coeff = Fraction(n - 1, n)
    rec_term = cosh_power_integral(n - 2, a, b, x)
    add_term = IRApply(MUL, (_frac_ir(rec_coeff), rec_term))
    return IRApply(ADD, (main_term, add_term))


def sinh_times_cosh_power(
    m: int,
    n: int,
    a: Fraction,
    b: Fraction,
    x: IRSymbol,
) -> IRNode | None:
    """Return IR for ``∫ sinh^m(ax+b) · cosh^n(ax+b) dx`` via u-substitution.

    Handles the two cases where exactly one factor has exponent 1:

    - ``m = 1`` (``n ≥ 1``): u = cosh(ax+b), result = cosh^(n+1)/(n+1)/a
    - ``n = 1`` (``m ≥ 1``): u = sinh(ax+b), result = sinh^(m+1)/(m+1)/a

    Both m=1, n=1 is handled by the m=1 branch (gives sinh²/(2a)).

    Returns ``None`` if neither m nor n equals 1 (caller falls through).

    Pre-conditions: ``m ≥ 1``, ``n ≥ 1``, ``a ≠ 0``.

    Parameters
    ----------
    m, n :
        Positive integer exponents (one must equal 1).
    a, b :
        Linear coefficients for the argument ``ax+b``.
    x :
        Integration variable symbol.

    Returns
    -------
    IR node or ``None`` if the u-sub pattern does not apply.

    Examples
    --------
    ::

        # ∫ sinh(x)·cosh²(x) dx = cosh³(x)/3
        sinh_times_cosh_power(1, 2, Fraction(1), Fraction(0), IRSymbol("x"))

        # ∫ sinh²(x)·cosh(x) dx = sinh³(x)/3
        sinh_times_cosh_power(2, 1, Fraction(1), Fraction(0), IRSymbol("x"))
    """
    arg_ir = linear_to_ir(a, b, x)
    a_ir = _frac_ir(a)

    if m == 1:
        # u = cosh(ax+b) → cosh^(n+1)(ax+b) / ((n+1)·a)
        cosh_ir = IRApply(COSH, (arg_ir,))
        new_exp = n + 1
        cosh_pow = IRApply(POW, (cosh_ir, IRInteger(new_exp)))
        denom = IRApply(MUL, (IRInteger(new_exp), a_ir))
        return IRApply(DIV, (cosh_pow, denom))

    if n == 1:
        # u = sinh(ax+b) → sinh^(m+1)(ax+b) / ((m+1)·a)
        sinh_ir = IRApply(SINH, (arg_ir,))
        new_exp = m + 1
        sinh_pow = IRApply(POW, (sinh_ir, IRInteger(new_exp)))
        denom = IRApply(MUL, (IRInteger(new_exp), a_ir))
        return IRApply(DIV, (sinh_pow, denom))

    # ─────────────────────────────────────────────────────────────────────
    # General case: both m ≥ 2 and n ≥ 2.
    #
    # Three strategies based on parity.  The key algebraic identities are:
    #
    #   sinh²(u) = cosh²(u) − 1   →  d(cosh)/dx involves sinh
    #   cosh²(u) = sinh²(u) + 1   →  d(sinh)/dx involves cosh
    # ─────────────────────────────────────────────────────────────────────

    cosh_arg = IRApply(COSH, (arg_ir,))
    sinh_arg = IRApply(SINH, (arg_ir,))

    if m % 2 == 1:
        # ── Sub-case A: m odd (m = 2p+1 ≥ 3) ──────────────────────────────
        #
        # Pull one sinh out and use sinh² = cosh²−1:
        #   sinh^(2p+1)·cosh^n = sinh · (cosh²−1)^p · cosh^n
        #
        # Substitute u = cosh(ax+b), du = a·sinh(ax+b) dx:
        #   ∫ = (1/a) ∫ (u²−1)^p · u^n du
        #
        # Expand (u²−1)^p by binomial:
        #   (u²−1)^p = Σ_{k=0}^{p} C(p,k)·(−1)^(p−k)·u^(2k)
        #
        # Antiderivative (back-sub u = cosh):
        #   Σ_{k=0}^{p} [C(p,k)·(−1)^(p−k) / (a·(2k+n+1))] · cosh^(2k+n+1)
        p = (m - 1) // 2
        terms: list[IRNode] = []
        for k in range(p + 1):
            sign = 1 if (p - k) % 2 == 0 else -1
            binom_k = _comb(p, k)
            exp_val = 2 * k + n + 1
            coeff = Fraction(sign * binom_k, exp_val) / a
            cosh_pow = IRApply(POW, (cosh_arg, IRInteger(exp_val)))
            terms.append(IRApply(MUL, (_frac_ir(coeff), cosh_pow)))
        return _fold_add(terms)

    if n % 2 == 1:
        # ── Sub-case B: n odd (n = 2q+1 ≥ 3) ──────────────────────────────
        #
        # Pull one cosh out and use cosh² = sinh²+1:
        #   sinh^m · cosh^(2q+1) = cosh · (sinh²+1)^q · sinh^m
        #
        # Substitute u = sinh(ax+b), du = a·cosh(ax+b) dx:
        #   ∫ = (1/a) ∫ (u²+1)^q · u^m du
        #
        # Expand (u²+1)^q = Σ_{k=0}^{q} C(q,k)·u^(2k)  (all signs positive)
        #
        # Antiderivative (back-sub u = sinh):
        #   Σ_{k=0}^{q} [C(q,k) / (a·(2k+m+1))] · sinh^(2k+m+1)
        q = (n - 1) // 2
        terms = []
        for k in range(q + 1):
            binom_k = _comb(q, k)
            exp_val = 2 * k + m + 1
            coeff = Fraction(binom_k, exp_val) / a
            sinh_pow = IRApply(POW, (sinh_arg, IRInteger(exp_val)))
            terms.append(IRApply(MUL, (_frac_ir(coeff), sinh_pow)))
        return _fold_add(terms)

    # ── Sub-case C: both m, n even ──────────────────────────────────────────
    #
    # Use sinh^(2p) = (cosh²−1)^p to express everything in terms of
    # even powers of cosh, then call cosh_power_integral:
    #
    #   sinh^(2p)·cosh^(2q) = (cosh²−1)^p · cosh^(2q)
    #                       = Σ_{k=0}^{p} C(p,k)·(−1)^(p−k) · cosh^(2k+2q)
    #   ∫ = Σ_{k=0}^{p} C(p,k)·(−1)^(p−k) · ∫ cosh^(2k+n) dx
    p = m // 2
    terms = []
    for k in range(p + 1):
        sign = 1 if (p - k) % 2 == 0 else -1
        binom_k = _comb(p, k)
        power = 2 * k + n
        sub_integral = cosh_power_integral(power, a, b, x)
        coeff = Fraction(sign * binom_k)
        if coeff == Fraction(1):
            terms.append(sub_integral)
        else:
            terms.append(IRApply(MUL, (_frac_ir(coeff), sub_integral)))
    return _fold_add(terms)


# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------


def _frac_ir(c: Fraction) -> IRNode:
    """Lift a Fraction to its canonical IR literal."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


def _fold_add(terms: list[IRNode]) -> IRNode:
    """Left-fold a non-empty list of IR nodes into binary ADD nodes.

    Example: ``[a, b, c]`` → ``ADD(ADD(a, b), c)``.

    This matches the VM's binary-arity ADD handler.  Never pass an empty
    list.
    """
    assert terms, "_fold_add requires at least one term"
    acc = terms[0]
    for t in terms[1:]:
        acc = IRApply(ADD, (acc, t))
    return acc


__all__ = [
    "cosh_power_integral",
    "sinh_power_integral",
    "sinh_times_cosh_power",
]
