"""Linear-equation closed form: a*x + b = 0  →  x = -b/a."""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRInteger, IRNode, IRRational

# Sentinel returned when every x is a solution.
ALL = "ALL"


def solve_linear(a: Fraction, b: Fraction) -> list[IRNode] | str:
    """Solve ``a*x + b = 0`` over Q.

    Returns:
        - ``[x]`` (a single-element list of IR) when ``a ≠ 0``.
        - ``[]`` (no solutions) when ``a = 0`` and ``b ≠ 0``.
        - the string ``"ALL"`` (every x satisfies) when ``a = 0`` and
          ``b = 0``.

    Coefficients are :class:`fractions.Fraction` for exact arithmetic.
    """
    if a == 0:
        if b == 0:
            return ALL
        return []
    x = -b / a
    return [_fraction_to_ir(x)]


def _fraction_to_ir(f: Fraction) -> IRNode:
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)
