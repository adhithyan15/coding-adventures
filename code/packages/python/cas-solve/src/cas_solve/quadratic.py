"""Quadratic-equation closed form via the quadratic formula.

For ``a x² + b x + c = 0`` with rational coefficients:

    x = (-b ± sqrt(disc)) / (2a)
    disc = b² - 4ac

Three cases:

1. ``disc`` is a perfect square (numerator and denominator both perfect
   squares of the underlying Fraction): both roots are rational.
2. ``disc > 0`` but not a perfect square: roots are real-irrational,
   expressed using ``Sqrt(disc)`` in the IR.
3. ``disc < 0``: roots are complex, expressed as ``r ± k*%i`` using
   Maxima's imaginary-unit symbol ``%i``.

If ``a == 0``, falls back to :func:`solve_linear`.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import (
    ADD,
    DIV,
    MUL,
    NEG,
    SQRT,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from cas_solve.linear import solve_linear

I_UNIT = IRSymbol("%i")


def solve_quadratic(a: Fraction, b: Fraction, c: Fraction) -> list[IRNode] | str:
    """Solve ``a*x^2 + b*x + c = 0`` over Q (with i if needed).

    Returns a list of root IR (one or two elements), or whatever
    :func:`solve_linear` returns when ``a == 0``.
    """
    if a == 0:
        return solve_linear(b, c)

    discriminant = b * b - 4 * a * c
    two_a = 2 * a

    if discriminant > 0:
        sqrt_node = _sqrt_or_rational(discriminant)
        if isinstance(sqrt_node, Fraction):
            # Perfect square: roots are rational.
            roots = sorted([(-b + sqrt_node) / two_a, (-b - sqrt_node) / two_a])
            return [_fraction_to_ir(r) for r in roots]
        # Irrational discriminant: roots use Sqrt(disc) in the IR.
        return [
            _build_irrational_root(-b, two_a, sqrt_node, sign=1),
            _build_irrational_root(-b, two_a, sqrt_node, sign=-1),
        ]

    if discriminant == 0:
        # Single repeated root.
        root = -b / two_a
        return [_fraction_to_ir(root)]

    # Negative discriminant — complex roots.
    abs_disc = -discriminant
    sqrt_abs = _sqrt_or_rational(abs_disc)
    return [
        _build_complex_root(-b, two_a, sqrt_abs, sign=1),
        _build_complex_root(-b, two_a, sqrt_abs, sign=-1),
    ]


# ---------------------------------------------------------------------------
# Square-root helper
# ---------------------------------------------------------------------------


def _sqrt_or_rational(value: Fraction) -> Fraction | IRApply:
    """If ``value`` is a perfect square, return the rational square root.

    Otherwise return ``IRApply(SQRT, (literal,))``.
    """
    if value < 0:
        return IRApply(SQRT, (_fraction_to_ir(value),))
    num = value.numerator
    den = value.denominator
    rn = _isqrt(num) if num >= 0 else None
    rd = _isqrt(den) if den >= 0 else None
    if rn is not None and rd is not None:
        return Fraction(rn, rd)
    return IRApply(SQRT, (_fraction_to_ir(value),))


def _isqrt(n: int) -> int | None:
    """Integer square root if ``n`` is a perfect square; else None."""
    if n < 0:
        return None
    r = int(n**0.5)
    # Bracket-search to handle float rounding for large n.
    for cand in (r - 1, r, r + 1):
        if cand >= 0 and cand * cand == n:
            return cand
    return None


# ---------------------------------------------------------------------------
# IR builders
# ---------------------------------------------------------------------------


def _build_irrational_root(
    neg_b: Fraction,
    two_a: Fraction,
    sqrt_node: IRApply,
    *,
    sign: int,
) -> IRNode:
    """Build ``(-b ± sqrt(disc)) / (2a)`` as IR for irrational discriminants."""
    head_op = ADD if sign > 0 else SUB
    numer = IRApply(head_op, (_fraction_to_ir(neg_b), sqrt_node))
    return IRApply(DIV, (numer, _fraction_to_ir(two_a)))


def _build_complex_root(
    neg_b: Fraction,
    two_a: Fraction,
    sqrt_abs: Fraction | IRApply,
    *,
    sign: int,
) -> IRNode:
    """Build ``-b/(2a) ± (sqrt(|disc|)/(2a))*%i`` for negative discriminants."""
    real_part = _fraction_to_ir(neg_b / two_a)
    if isinstance(sqrt_abs, Fraction):
        coef = sqrt_abs / two_a
        imag_part = IRApply(MUL, (_fraction_to_ir(coef), I_UNIT))
    else:
        coef_ir: IRNode = IRApply(DIV, (sqrt_abs, _fraction_to_ir(two_a)))
        imag_part = IRApply(MUL, (coef_ir, I_UNIT))
    head_op = ADD if sign > 0 else SUB
    return IRApply(head_op, (real_part, imag_part))


def _fraction_to_ir(f: Fraction) -> IRNode:
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


# Reference avoids unused-import warnings for NEG (we keep it imported
# because the build helpers may end up using it in a follow-up release).
_ = NEG
