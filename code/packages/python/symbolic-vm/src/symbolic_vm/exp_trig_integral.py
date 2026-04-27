"""Exp × sin/cos integration — Phase 4c.

Integrates ``exp(ax+b) · sin(cx+d)`` and ``exp(ax+b) · cos(cx+d)`` for
``a, c ∈ Q \\ {0}`` via the double integration-by-parts closed form.

Applying IBP twice and solving for ``I``:

    ∫ exp(ax+b)·sin(cx+d) dx  =  exp(ax+b) · [a·sin(cx+d) − c·cos(cx+d)] / (a²+c²)
    ∫ exp(ax+b)·cos(cx+d) dx  =  exp(ax+b) · [a·cos(cx+d) + c·sin(cx+d)] / (a²+c²)

The denominator ``D = a² + c²`` is always positive for ``a, c ≠ 0``.

See ``code/specs/phase4-trig-integration.md`` for the full derivation.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    COS,
    EXP,
    MUL,
    SIN,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import linear_to_ir


def exp_sin_integral(
    a: Fraction,
    b: Fraction,
    c: Fraction,
    d: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ exp(ax+b) · sin(cx+d) dx``.

    Pre-conditions: ``a ≠ 0``, ``c ≠ 0``.
    Result: ``exp(ax+b) · [a·sin(cx+d) − c·cos(cx+d)] / (a²+c²)``.
    """
    D = a * a + c * c  # always positive
    exp_ir = IRApply(EXP, (linear_to_ir(a, b, x_sym),))
    trig_arg = linear_to_ir(c, d, x_sym)
    sin_ir = IRApply(SIN, (trig_arg,))
    cos_ir = IRApply(COS, (trig_arg,))
    # a·sin − c·cos, each scaled by 1/D
    a_sin = IRApply(MUL, (_frac_ir(a / D), sin_ir))
    c_cos = IRApply(MUL, (_frac_ir(c / D), cos_ir))
    bracket = IRApply(SUB, (a_sin, c_cos))
    return IRApply(MUL, (exp_ir, bracket))


def exp_cos_integral(
    a: Fraction,
    b: Fraction,
    c: Fraction,
    d: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return IR for ``∫ exp(ax+b) · cos(cx+d) dx``.

    Pre-conditions: ``a ≠ 0``, ``c ≠ 0``.
    Result: ``exp(ax+b) · [a·cos(cx+d) + c·sin(cx+d)] / (a²+c²)``.
    """
    D = a * a + c * c
    exp_ir = IRApply(EXP, (linear_to_ir(a, b, x_sym),))
    trig_arg = linear_to_ir(c, d, x_sym)
    cos_ir = IRApply(COS, (trig_arg,))
    sin_ir = IRApply(SIN, (trig_arg,))
    # a·cos + c·sin, each scaled by 1/D
    a_cos = IRApply(MUL, (_frac_ir(a / D), cos_ir))
    c_sin = IRApply(MUL, (_frac_ir(c / D), sin_ir))
    bracket = IRApply(ADD, (a_cos, c_sin))
    return IRApply(MUL, (exp_ir, bracket))


def _frac_ir(c: Fraction) -> IRNode:
    """Lift a Fraction to its canonical IR literal."""
    if c.denominator == 1:
        return IRInteger(c.numerator)
    return IRRational(c.numerator, c.denominator)


__all__ = ["exp_cos_integral", "exp_sin_integral"]
