"""Reciprocal hyperbolic power integration — Phase 16 and 17.

Computes antiderivatives for four families:

1. ``∫ sech^n(ax+b) dx``  — via IBP reduction formula (any integer n ≥ 0)
2. ``∫ csch^n(ax+b) dx``  — via IBP reduction formula (any integer n ≥ 0)
3. ``∫ coth^n(ax+b) dx``  — via the identity coth² = 1 + csch² (any integer n ≥ 0)
4. ``∫ tanh^n(ax+b) dx``  — via the identity tanh² = 1 − sech² (any integer n ≥ 0)

Reduction formulas
------------------

**sech^n (derivation)**

IBP with ``u = sech^(n-2)(t)``, ``dv = sech²(t) dt``:

    v = tanh(t),   du = -(n-2)·sech^(n-2)(t)·tanh(t) dt

    ∫ sech^n dt = sech^(n-2)·tanh + (n-2) ∫ sech^(n-2)·tanh² dt
               = sech^(n-2)·tanh + (n-2) ∫ sech^(n-2)·(1 − sech²) dt
               = sech^(n-2)·tanh + (n-2)·I_{n-2} − (n-2)·I_n

Solving for I_n:

    I_n = sech^(n-2)·tanh / (n-1) + (n-2)/(n-1) · I_{n-2}

Base cases: I_0 = t, I_1 = atan(sinh(t)), I_2 = tanh(t).

With ``t = ax+b``:  ∫ sech^n(ax+b) dx = chain-rule factor 1/a applied as
coefficient of each base case and main term.

**csch^n (derivation)**

IBP with ``u = csch^(n-2)(t)``, ``dv = csch²(t) dt``:

    v = -coth(t),  du = -(n-2)·csch^(n-2)(t)·coth(t) dt

    ∫ csch^n dt = -csch^(n-2)·coth − (n-2) ∫ csch^(n-2)·coth² dt
               = -csch^(n-2)·coth − (n-2) ∫ csch^(n-2)·(1 + csch²) dt
               = -csch^(n-2)·coth − (n-2)·I_{n-2} − (n-2)·I_n

Solving for I_n:

    I_n = -csch^(n-2)·coth / (n-1) − (n-2)/(n-1) · I_{n-2}

Base cases: I_0 = t, I_1 = log(tanh(t/2)), I_2 = -coth(t).

(Note the ``−`` signs — contrast with the ``+`` for sech.)

**coth^n (identity reduction)**

Expand via ``coth²(t) = 1 + csch²(t)``:

    ∫ coth^n dt = ∫ coth^(n-2)·(1 + csch²) dt
               = I_{n-2} + ∫ coth^(n-2)·csch² dt

For the last integral substitute u = coth(t), du = -csch²(t) dt:

    ∫ coth^(n-2)·csch² dt = -coth^(n-1) / (n-1)

So:  I_n = I_{n-2} - coth^(n-1) / (n-1)

Base cases: I_0 = t, I_1 = log(sinh(t)).

Unlike sech/csch there is no "outer product" term in the recursion step —
just the previous integral minus a single power.

Design notes
------------

- ``_frac_ir`` is defined locally (same pattern as ``hyp_power_integral.py``)
  to avoid a circular import with ``integrate.py``.
- ``linear_to_ir`` is imported from ``symbolic_vm.polynomial_bridge``.
- None of the three functions call back into ``integrate.py``, so there is
  no import cycle.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    ATAN,
    COSH,
    COTH,
    CSCH,
    DIV,
    LOG,
    MUL,
    NEG,
    POW,
    SECH,
    SINH,
    SUB,
    TANH,
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


def sech_power_integral(
    n: int,
    a: Fraction,
    b: Fraction,
    x: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ sech^n(ax+b) dx``.

    Uses the IBP reduction formula recursively:

        I_n = sech^(n-2)(ax+b)·tanh(ax+b) / ((n-1)·a)
             + (n-2)/(n-1) · I_{n-2}

    Base cases:

        I_0 = x
        I_1 = atan(sinh(ax+b)) / a          [Phase 15 _sech_integral formula]
        I_2 = tanh(ax+b) / a                [most-used identity: ∫sech²=tanh]

    Pre-conditions: ``n ≥ 0``, ``a ≠ 0``.

    Parameters
    ----------
    n :
        Non-negative integer power.
    a, b :
        Fraction coefficients of the linear argument ``ax+b``.
    x :
        Integration variable symbol.

    Examples
    --------
    ::

        # ∫ sech²(x) dx = tanh(x)
        sech_power_integral(2, Fraction(1), Fraction(0), IRSymbol("x"))

        # ∫ sech³(x) dx = sech(x)·tanh(x)/2 + (1/2)·atan(sinh(x))
        sech_power_integral(3, Fraction(1), Fraction(0), IRSymbol("x"))
    """
    if n == 0:
        return x
    arg_ir = linear_to_ir(a, b, x)
    if n == 1:
        # ∫ sech(ax+b) dx = atan(sinh(ax+b)) / a
        atan_sinh = IRApply(ATAN, (IRApply(SINH, (arg_ir,)),))
        return atan_sinh if a == Fraction(1) else IRApply(DIV, (atan_sinh, _frac_ir(a)))
    if n == 2:
        # ∫ sech²(ax+b) dx = tanh(ax+b) / a
        tanh_ir = IRApply(TANH, (arg_ir,))
        return tanh_ir if a == Fraction(1) else IRApply(DIV, (tanh_ir, _frac_ir(a)))

    # n ≥ 3:  I_n = sech^(n-2)·tanh / ((n-1)·a)  +  (n-2)/(n-1) · I_{n-2}
    #
    # When n-2 == 1 we use SECH(arg_ir) directly instead of POW(SECH, 1).
    sech_ir = IRApply(SECH, (arg_ir,))
    tanh_ir = IRApply(TANH, (arg_ir,))
    sech_pow = (
        sech_ir
        if n - 2 == 1
        else IRApply(POW, (sech_ir, IRInteger(n - 2)))
    )
    coeff1 = Fraction(1, n - 1) / a         # 1 / ((n-1)·a)
    main_term = IRApply(MUL, (_frac_ir(coeff1), IRApply(MUL, (sech_pow, tanh_ir))))
    rec_coeff = Fraction(n - 2, n - 1)      # (n-2) / (n-1)  — always > 0 for n ≥ 3
    tail = sech_power_integral(n - 2, a, b, x)
    return IRApply(ADD, (main_term, IRApply(MUL, (_frac_ir(rec_coeff), tail))))


def csch_power_integral(
    n: int,
    a: Fraction,
    b: Fraction,
    x: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ csch^n(ax+b) dx``.

    Uses the IBP reduction formula recursively:

        I_n = −csch^(n-2)(ax+b)·coth(ax+b) / ((n-1)·a)
             − (n-2)/(n-1) · I_{n-2}

    Base cases:

        I_0 = x
        I_1 = log(tanh((ax+b)/2)) / a       [Phase 15 _csch_integral formula]
        I_2 = −coth(ax+b) / a               [note negative sign]

    Pre-conditions: ``n ≥ 0``, ``a ≠ 0``.

    Parameters
    ----------
    n :
        Non-negative integer power.
    a, b :
        Fraction coefficients of the linear argument ``ax+b``.
    x :
        Integration variable symbol.

    Examples
    --------
    ::

        # ∫ csch²(x) dx = -coth(x)
        csch_power_integral(2, Fraction(1), Fraction(0), IRSymbol("x"))

        # ∫ csch³(x) dx = -csch(x)·coth(x)/2 - (1/2)·log(tanh(x/2))
        csch_power_integral(3, Fraction(1), Fraction(0), IRSymbol("x"))
    """
    if n == 0:
        return x
    arg_ir = linear_to_ir(a, b, x)
    if n == 1:
        # ∫ csch(ax+b) dx = log(tanh((ax+b)/2)) / a
        half_a = a / Fraction(2)
        half_b = b / Fraction(2)
        half_arg = linear_to_ir(half_a, half_b, x)
        log_tanh = IRApply(LOG, (IRApply(TANH, (half_arg,)),))
        return log_tanh if a == Fraction(1) else IRApply(DIV, (log_tanh, _frac_ir(a)))
    if n == 2:
        # ∫ csch²(ax+b) dx = -coth(ax+b) / a
        neg_coth = IRApply(NEG, (IRApply(COTH, (arg_ir,)),))
        return neg_coth if a == Fraction(1) else IRApply(DIV, (neg_coth, _frac_ir(a)))

    # n ≥ 3:  I_n = -csch^(n-2)·coth / ((n-1)·a)  −  (n-2)/(n-1) · I_{n-2}
    #
    # Both terms are negative — use SUB(NEG(main), rec_term) = -(main + rec_term)
    # Equivalently: result = NEG(main_term) - rec_coeff * tail
    csch_ir = IRApply(CSCH, (arg_ir,))
    coth_ir = IRApply(COTH, (arg_ir,))
    csch_pow = (
        csch_ir
        if n - 2 == 1
        else IRApply(POW, (csch_ir, IRInteger(n - 2)))
    )
    coeff1 = Fraction(1, n - 1) / a
    main_term = IRApply(
        NEG,
        (IRApply(MUL, (_frac_ir(coeff1), IRApply(MUL, (csch_pow, coth_ir)))),),
    )
    rec_coeff = Fraction(n - 2, n - 1)
    tail = csch_power_integral(n - 2, a, b, x)
    return IRApply(SUB, (main_term, IRApply(MUL, (_frac_ir(rec_coeff), tail))))


def coth_power_integral(
    n: int,
    a: Fraction,
    b: Fraction,
    x: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ coth^n(ax+b) dx``.

    Uses the identity ``coth²(t) = 1 + csch²(t)`` to derive the reduction:

        I_n = I_{n-2} − coth^(n-1)(ax+b) / ((n-1)·a)

    Base cases:

        I_0 = x
        I_1 = log(sinh(ax+b)) / a           [Phase 15 _coth_integral formula]

    Note: unlike sech/csch there is no explicit outer-product term involving
    two hyperbolic functions.  Each step just subtracts one power of coth and
    recurses down by 2, producing the telescoping series::

        ∫ coth^4(x) dx = x - coth/1 - coth³/3
        ∫ coth^5(x) dx = log(sinh) - coth²/2 - coth⁴/4

    Pre-conditions: ``n ≥ 0``, ``a ≠ 0``.

    Parameters
    ----------
    n :
        Non-negative integer power.
    a, b :
        Fraction coefficients of the linear argument ``ax+b``.
    x :
        Integration variable symbol.

    Examples
    --------
    ::

        # ∫ coth²(x) dx = x - coth(x)
        coth_power_integral(2, Fraction(1), Fraction(0), IRSymbol("x"))

        # ∫ coth³(x) dx = log(sinh(x)) - coth²(x)/2
        coth_power_integral(3, Fraction(1), Fraction(0), IRSymbol("x"))
    """
    if n == 0:
        return x
    arg_ir = linear_to_ir(a, b, x)
    if n == 1:
        # ∫ coth(ax+b) dx = log(sinh(ax+b)) / a
        log_sinh = IRApply(LOG, (IRApply(SINH, (arg_ir,)),))
        return log_sinh if a == Fraction(1) else IRApply(DIV, (log_sinh, _frac_ir(a)))

    # n ≥ 2:  I_n = I_{n-2} − coth^(n-1) / ((n-1)·a)
    #
    # When n-1 == 1 use COTH(arg_ir) directly.
    coth_ir = IRApply(COTH, (arg_ir,))
    coth_pow = (
        coth_ir
        if n - 1 == 1
        else IRApply(POW, (coth_ir, IRInteger(n - 1)))
    )
    coeff = Fraction(1, n - 1) / a          # 1 / ((n-1)·a)
    power_term = IRApply(MUL, (_frac_ir(coeff), coth_pow))
    tail = coth_power_integral(n - 2, a, b, x)
    return IRApply(SUB, (tail, power_term))


def tanh_power_integral(
    n: int,
    a: Fraction,
    b: Fraction,
    x: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ tanh^n(ax+b) dx``.

    Uses the identity ``tanh²(t) = 1 − sech²(t)`` to derive the reduction:

        I_n = I_{n-2} − tanh^(n-1)(ax+b) / ((n-1)·a)

    Base cases:

        I_0 = x
        I_1 = log(cosh(ax+b)) / a           [Phase 13 tanh bare integral]

    This is the direct analog of ``coth_power_integral`` — compare:

    - coth: ``coth² = 1 + csch²``  →  ``I_n = I_{n-2} − coth^(n-1)/((n-1)a)``
    - tanh: ``tanh² = 1 − sech²``  →  ``I_n = I_{n-2} − tanh^(n-1)/((n-1)a)``

    Both have the same recursion structure; only the identity sign differs
    (``+`` vs ``−``), and the bases differ.

    **Derivation** — expand via ``tanh² = 1 − sech²``:

    .. code-block:: text

        ∫ tanh^n dt = ∫ tanh^(n-2)·tanh² dt
                    = ∫ tanh^(n-2)·(1 − sech²) dt
                    = I_{n-2} − ∫ tanh^(n-2)·sech² dt

    For the last term, substitute u = tanh(t), du = sech²(t) dt:

    .. code-block:: text

        ∫ tanh^(n-2)·sech² dt = tanh^(n-1) / (n-1)

    **Verification examples:**

    - n=2: F = x − tanh.  F' = 1 − sech² = tanh²  ✓
    - n=3: F = log(cosh) − tanh²/2.  F' = tanh(1−sech²) = tanh³  ✓
    - n=4: F = x − tanh − tanh³/3.  F' = tanh²(1−sech²) = tanh⁴  ✓

    Pre-conditions: ``n ≥ 0``, ``a ≠ 0``.

    Parameters
    ----------
    n :
        Non-negative integer power.
    a, b :
        Fraction coefficients of the linear argument ``ax+b``.
    x :
        Integration variable symbol.

    Examples
    --------
    ::

        # ∫ tanh²(x) dx = x − tanh(x)
        tanh_power_integral(2, Fraction(1), Fraction(0), IRSymbol("x"))

        # ∫ tanh³(x) dx = log(cosh(x)) − tanh²(x)/2
        tanh_power_integral(3, Fraction(1), Fraction(0), IRSymbol("x"))
    """
    if n == 0:
        return x
    arg_ir = linear_to_ir(a, b, x)
    if n == 1:
        # ∫ tanh(ax+b) dx = log(cosh(ax+b)) / a
        log_cosh = IRApply(LOG, (IRApply(COSH, (arg_ir,)),))
        return log_cosh if a == Fraction(1) else IRApply(DIV, (log_cosh, _frac_ir(a)))

    # n ≥ 2:  I_n = I_{n-2} − tanh^(n-1) / ((n-1)·a)
    #
    # When n-1 == 1 use TANH(arg_ir) directly (avoids POW(..., 1)).
    tanh_ir = IRApply(TANH, (arg_ir,))
    tanh_pow = (
        tanh_ir
        if n - 1 == 1
        else IRApply(POW, (tanh_ir, IRInteger(n - 1)))
    )
    coeff = Fraction(1, n - 1) / a          # 1 / ((n-1)·a)
    power_term = IRApply(MUL, (_frac_ir(coeff), tanh_pow))
    tail = tanh_power_integral(n - 2, a, b, x)
    return IRApply(SUB, (tail, power_term))


# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------


def _frac_ir(c: Fraction) -> IRNode:
    """Lift a Fraction to its canonical IR literal.

    Returns ``IRInteger`` when the denominator is 1, ``IRRational`` otherwise.
    This is a local copy of the same helper in ``hyp_power_integral.py`` —
    duplicated to avoid a circular import with ``integrate.py``.
    """
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)
