"""Exp × sinh/cosh integration — Phase 14.

Computes ``∫ exp(ax+b) · sinh(cx+d) dx`` and
``∫ exp(ax+b) · cosh(cx+d) dx`` for ``a, c ∈ Q \\ {0}``, ``a² ≠ c²``,
via the double integration-by-parts closed form.

Derivation
----------
Express sinh and cosh in terms of exponentials:

    sinh(cx+d) = (e^(cx+d) − e^(-(cx+d))) / 2
    cosh(cx+d) = (e^(cx+d) + e^(-(cx+d))) / 2

Multiply by e^(ax+b) and split into two simpler integrals:

    ∫ e^(ax+b) · sinh(cx+d) dx
      = (1/2) ∫ e^((a+c)x+(b+d)) dx  −  (1/2) ∫ e^((a-c)x+(b-d)) dx
      = e^(ax+b) / 2 · [ e^(cx+d)/(a+c) − e^(-(cx+d))/(a-c) ]

Recombine the bracket into hyperbolic form.  The numerator of the bracket
factor is:

    (a−c)·e^(cx+d) − (a+c)·e^(-(cx+d))
        = a·(e^(cx+d) − e^(-(cx+d))) − c·(e^(cx+d) + e^(-(cx+d)))
        = 2a·sinh(cx+d) − 2c·cosh(cx+d)

so the final result is:

    ∫ e^(ax+b) · sinh(cx+d) dx = e^(ax+b) · [a·sinh(cx+d) − c·cosh(cx+d)] / (a²−c²)

Similarly:

    ∫ e^(ax+b) · cosh(cx+d) dx = e^(ax+b) · [a·cosh(cx+d) − c·sinh(cx+d)] / (a²−c²)

The denominator ``D = a² − c²`` is non-zero when ``a ≠ ±c``.  Callers must
check this precondition; both functions assume ``D ≠ 0``.

Contrast with ``exp_trig_integral.py`` (Phase 4c) where the denominator is
``a² + c²``.  The sign difference comes from the sign in the hyperbolic vs
trigonometric Pythagorean identity (``cosh²−sinh²=1`` vs ``sin²+cos²=1``).
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    COSH,
    EXP,
    MUL,
    SINH,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import linear_to_ir


def exp_sinh_integral(
    a: Fraction,
    b: Fraction,
    c: Fraction,
    d: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ exp(ax+b) · sinh(cx+d) dx``.

    Pre-conditions: ``a ≠ 0``, ``c ≠ 0``, ``a² ≠ c²``.

    Formula::

        exp(ax+b) · [a·sinh(cx+d) − c·cosh(cx+d)] / (a²−c²)

    Parameters
    ----------
    a, b :
        Linear coefficient and constant for the exponent argument: ``ax+b``.
    c, d :
        Linear coefficient and constant for the hyperbolic argument: ``cx+d``.
    x_sym :
        The integration variable symbol.

    Returns
    -------
    IR expression for the antiderivative.

    Examples
    --------
    ::

        # ∫ exp(x)·sinh(x) dx  — note: a²=c² here, so caller should guard
        # ∫ exp(2x)·sinh(x) dx = exp(2x)·[2·sinh(x) − cosh(x)] / 3
        exp_sinh_integral(Fraction(2), Fraction(0),
                          Fraction(1), Fraction(0), IRSymbol("x"))
        # → Mul(Exp(Mul(2, x)), Div(Sub(Mul(2, Sinh(x)), Cosh(x)), 3))
    """
    D = a * a - c * c  # a² − c²; caller ensures D ≠ 0
    exp_ir = IRApply(EXP, (linear_to_ir(a, b, x_sym),))
    hyp_arg = linear_to_ir(c, d, x_sym)
    sinh_ir = IRApply(SINH, (hyp_arg,))
    cosh_ir = IRApply(COSH, (hyp_arg,))
    # a/D · sinh − c/D · cosh
    a_sinh = IRApply(MUL, (_frac_ir(a / D), sinh_ir))
    c_cosh = IRApply(MUL, (_frac_ir(c / D), cosh_ir))
    bracket = IRApply(SUB, (a_sinh, c_cosh))
    return IRApply(MUL, (exp_ir, bracket))


def exp_cosh_integral(
    a: Fraction,
    b: Fraction,
    c: Fraction,
    d: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ exp(ax+b) · cosh(cx+d) dx``.

    Pre-conditions: ``a ≠ 0``, ``c ≠ 0``, ``a² ≠ c²``.

    Formula::

        exp(ax+b) · [a·cosh(cx+d) − c·sinh(cx+d)] / (a²−c²)

    The derivation mirrors ``exp_sinh_integral`` with sinh and cosh swapped
    in the bracket.

    Parameters
    ----------
    a, b :
        Linear coefficient and constant for the exponent argument.
    c, d :
        Linear coefficient and constant for the hyperbolic argument.
    x_sym :
        The integration variable symbol.

    Returns
    -------
    IR expression for the antiderivative.

    Examples
    --------
    ::

        # ∫ exp(2x)·cosh(x) dx = exp(2x)·[2·cosh(x) − sinh(x)] / 3
        exp_cosh_integral(Fraction(2), Fraction(0),
                          Fraction(1), Fraction(0), IRSymbol("x"))
    """
    D = a * a - c * c  # a² − c²; caller ensures D ≠ 0
    exp_ir = IRApply(EXP, (linear_to_ir(a, b, x_sym),))
    hyp_arg = linear_to_ir(c, d, x_sym)
    cosh_ir = IRApply(COSH, (hyp_arg,))
    sinh_ir = IRApply(SINH, (hyp_arg,))
    # a/D · cosh − c/D · sinh
    a_cosh = IRApply(MUL, (_frac_ir(a / D), cosh_ir))
    c_sinh = IRApply(MUL, (_frac_ir(c / D), sinh_ir))
    bracket = IRApply(SUB, (a_cosh, c_sinh))
    return IRApply(MUL, (exp_ir, bracket))


def exp_hyp_degenerate(
    a: Fraction,
    b: Fraction,
    c: Fraction,
    d: Fraction,
    is_sinh: bool,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ exp(ax+b) · sinh/cosh(cx+d) dx`` when ``a² = c²``.

    The standard double-IBP formula has denominator ``D = a² − c²``, which
    is zero when ``a = ±c``.  In that case we expand sinh/cosh into
    exponentials and integrate each term directly:

    .. code-block:: text

        e^(ax+b)·sinh(cx+d) = (e^((a+c)x+b+d) − e^((a-c)x+b-d)) / 2
        e^(ax+b)·cosh(cx+d) = (e^((a+c)x+b+d) + e^((a-c)x+b-d)) / 2

    Since ``a² = c²``, exactly one of ``a+c`` and ``a-c`` is zero:

    - ``a = c``  → ``a+c = 2a`` (non-zero), ``a-c = 0``
    - ``a = -c`` → ``a+c = 0``,  ``a-c = 2a`` (non-zero)

    Integrating each half:

    - ``∫ e^(kx+K) dx = e^(kx+K) / k``  (k ≠ 0)
    - ``∫ e^K dx      = e^K · x``         (k = 0)

    Pre-conditions: ``a ≠ 0``, ``c ≠ 0``, ``a² = c²``.

    Parameters
    ----------
    a, b :
        Linear coefficient and constant for the exponent argument.
    c, d :
        Linear coefficient and constant for the hyperbolic argument.
    is_sinh :
        ``True`` for sinh (uses subtraction sign), ``False`` for cosh (addition).
    x_sym :
        Integration variable.

    Returns
    -------
    IR node for the antiderivative.

    Examples
    --------
    ::

        # ∫ exp(x)·sinh(x) dx  (a = c = 1)
        # = exp(2x)/(4) − x/2
        exp_hyp_degenerate(Fraction(1), Fraction(0),
                           Fraction(1), Fraction(0),
                           is_sinh=True, x_sym=IRSymbol("x"))

        # ∫ exp(x)·cosh(x) dx  (a = c = 1)
        # = exp(2x)/(4) + x/2
        exp_hyp_degenerate(Fraction(1), Fraction(0),
                           Fraction(1), Fraction(0),
                           is_sinh=False, x_sym=IRSymbol("x"))

        # ∫ exp(x)·sinh(-x) dx  (a = 1, c = -1)
        # = x/2 − exp(2x)/(4)
        exp_hyp_degenerate(Fraction(1), Fraction(0),
                           Fraction(-1), Fraction(0),
                           is_sinh=True, x_sym=IRSymbol("x"))
    """
    # sign2: -1 for sinh (subtraction), +1 for cosh (addition)
    sign2 = Fraction(-1) if is_sinh else Fraction(1)

    # After multiplying e^(ax+b) into the exponential expansion:
    #   term1 coefficient: +1/2,    argument: (a+c)x + (b+d)
    #   term2 coefficient: sign2/2, argument: (a-c)x + (b-d)
    lin1 = a + c
    K1 = b + d
    lin2 = a - c
    K2 = b - d

    # Build IR for coeff * ∫ e^(lin*x + K) dx.
    def _antideriv_exp_linear(
        lin: Fraction, K: Fraction, coeff: Fraction
    ) -> IRNode:
        """Return ``coeff · ∫ exp(lin·x + K) dx``."""
        if lin == Fraction(0):
            # ∫ e^K dx = e^K · x
            if Fraction(0) == K:
                # e^0 = 1, so coeff · x
                if coeff == Fraction(1):
                    return x_sym
                return IRApply(MUL, (_frac_ir(coeff), x_sym))
            e_K = IRApply(EXP, (_frac_ir(K),))
            if coeff == Fraction(1):
                return IRApply(MUL, (e_K, x_sym))
            return IRApply(MUL, (_frac_ir(coeff), IRApply(MUL, (e_K, x_sym))))
        else:
            # ∫ e^(lin·x+K) dx = e^(lin·x+K) / lin
            total = coeff / lin
            exp_node = IRApply(EXP, (linear_to_ir(lin, K, x_sym),))
            if total == Fraction(1):
                return exp_node
            return IRApply(MUL, (_frac_ir(total), exp_node))

    t1 = _antideriv_exp_linear(lin1, K1, Fraction(1, 2))
    t2 = _antideriv_exp_linear(lin2, K2, sign2 * Fraction(1, 2))
    return IRApply(ADD, (t1, t2))


def _frac_ir(c: Fraction) -> IRNode:
    """Lift a Fraction to its canonical IR literal."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


__all__ = ["exp_cosh_integral", "exp_hyp_degenerate", "exp_sinh_integral"]
