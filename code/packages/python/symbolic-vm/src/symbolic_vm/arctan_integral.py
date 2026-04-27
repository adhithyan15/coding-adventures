"""Arctan integration — Phase 2e of the rational-function roadmap.

After Hermite reduction (Phase 2c) the log part is ``C(x)/E(x)`` with
``E`` squarefree. Rothstein–Trager (Phase 2d) handles it whenever all
log coefficients lie in Q. When RT returns ``None`` *and* ``E`` is an
irreducible quadratic over Q — degree 2, no rational roots — this module
produces the closed-form antiderivative directly:

    ∫ (px + q) / (ax² + bx + c) dx
        =  A · log(ax² + bx + c)
         + (2B / D) · arctan((2ax + b) / D)

where

    A  =  p / (2a)
    B  =  q  −  p·b / (2a)
    D  =  √(4ac − b²)     (real and positive; E is irreducible ⟹ b²−4ac < 0)

If D is a rational number (i.e. 4ac − b² is the perfect square of a
Fraction) the output carries only ``IRRational`` and ``IRInteger`` leaves —
no ``Sqrt`` node. Otherwise the coefficient 2B/D and the argument of
``Atan`` carry an ``Sqrt`` sub-tree, which the symbolic backend leaves
unevaluated while the numeric backend folds to a float.

Preconditions (caller's responsibility, not re-checked here):

- ``deg den == 2``
- ``rational_roots(den) == []``   (irreducible over Q)
- ``deg num < 2``                  (proper fraction — Hermite's guarantee)
- Coefficients in both polynomials are ``Fraction``.

See ``code/specs/arctan-integral.md`` for the full spec.
"""

from __future__ import annotations

from fractions import Fraction
from math import isqrt

from polynomial import normalize
from symbolic_ir import (
    ADD,
    ATAN,
    LOG,
    MUL,
    NEG,
    SQRT,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)


def arctan_integral(
    num: tuple,
    den: tuple,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ num/den dx`` when ``den`` is an irreducible quadratic.

    The caller guarantees the preconditions above. This function always
    returns a valid ``IRNode`` — it never returns ``None``. (When the
    preconditions are met there is always a closed-form answer.)

    The returned tree has the shape:

        A·log(den_ir) + (2B/D)·arctan((2ax+b)/D)

    with the ``A·log`` term omitted entirely when ``A == 0`` (pure arctan),
    and sign-normalised (``Neg`` instead of a negative ``Mul`` scalar when
    the coefficient is ``−1``).
    """
    num_n = normalize(num)
    den_n = normalize(den)

    # Extract den coefficients: den = a·x² + b·x + c
    # normalize returns (c₀, c₁, c₂) for degree-2 polynomial.
    c_coef = Fraction(den_n[0])
    b_coef = Fraction(den_n[1])
    a_coef = Fraction(den_n[2])

    # Extract num coefficients: num = p·x + q  (or just q)
    if len(num_n) == 0:
        p_coef = Fraction(0)
        q_coef = Fraction(0)
    elif len(num_n) == 1:
        p_coef = Fraction(0)
        q_coef = Fraction(num_n[0])
    else:
        p_coef = Fraction(num_n[1])
        q_coef = Fraction(num_n[0])

    # A = p / (2a),  B = q − p·b / (2a)
    two_a = Fraction(2) * a_coef
    a_val = p_coef / two_a
    b_val = q_coef - p_coef * b_coef / two_a

    # D² = 4ac − b²  (positive because E is irreducible)
    d_sq = Fraction(4) * a_coef * c_coef - b_coef * b_coef

    # Try to simplify √(D²) to a rational.
    d_rational = _rational_sqrt(d_sq)

    # Build IR nodes for the denominator polynomial  ax² + bx + c.
    den_ir = _quadratic_ir(a_coef, b_coef, c_coef, x_sym)

    # Build IR for (2ax + b) — the numerator of the Atan argument.
    atan_num_ir = _linear_ir(two_a, b_coef, x_sym)

    # Build Atan argument: (2ax + b) / D.
    d_ir = _d_ir(d_sq, d_rational)
    atan_arg = _div_ir(atan_num_ir, d_ir)
    atan_ir = IRApply(ATAN, (atan_arg,))

    # Scale the Atan term by 2B / D.
    two_b = Fraction(2) * b_val
    if d_rational is not None:
        atan_coef_frac = two_b / d_rational
        atan_term = _scale_ir(atan_coef_frac, atan_ir)
    else:
        # 2B/D = 2B · (1/√D²); build as Mul(2B_ir, Inv(√D²_ir)).
        atan_term = _irrational_scale_ir(two_b, d_sq, atan_ir)

    # Build the Log term  A · log(den) — omit entirely when A == 0.
    if a_val == 0:
        return atan_term

    log_ir = IRApply(LOG, (den_ir,))
    log_term = _scale_ir(a_val, log_ir)
    return IRApply(ADD, (log_term, atan_term))


# ---------------------------------------------------------------------------
# IR construction helpers
# ---------------------------------------------------------------------------


def _rational_sqrt(d_sq: Fraction) -> Fraction | None:
    """Return √d_sq as a Fraction if it is a perfect rational square, else None."""
    num, den = d_sq.numerator, d_sq.denominator
    n_sqrt = isqrt(num)
    d_sqrt = isqrt(den)
    if n_sqrt * n_sqrt == num and d_sqrt * d_sqrt == den:
        return Fraction(n_sqrt, d_sqrt)
    return None


def _frac_ir(f: Fraction) -> IRNode:
    """Convert a Fraction to the smallest IR number node."""
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


def _scale_ir(coef: Fraction, expr: IRNode) -> IRNode:
    """Return ``coef · expr`` with canonical sign handling."""
    if coef == 1:
        return expr
    if coef == -1:
        return IRApply(NEG, (expr,))
    return IRApply(MUL, (_frac_ir(coef), expr))


def _d_ir(d_sq: Fraction, d_rational: Fraction | None) -> IRNode:
    """Return the IR node representing D = √(d_sq)."""
    if d_rational is not None:
        return _frac_ir(d_rational)
    return IRApply(SQRT, (_frac_ir(d_sq),))


def _div_ir(numerator: IRNode, denominator: IRNode) -> IRNode:
    """Return numerator/denominator, simplifying when denominator is 1."""
    if isinstance(denominator, IRInteger) and denominator.value == 1:
        return numerator
    from symbolic_ir import DIV
    return IRApply(DIV, (numerator, denominator))


def _quadratic_ir(a: Fraction, b: Fraction, c: Fraction, x: IRSymbol) -> IRNode:
    """Build IR for ax² + bx + c."""
    from symbolic_ir import POW
    x_sq = IRApply(POW, (x, IRInteger(2)))
    ax_sq = _scale_ir(a, x_sq)
    bx = _scale_ir(b, x)
    c_ir = _frac_ir(c)

    # Assemble left-associatively: ((ax²) + bx) + c or simplified forms.
    if b == 0 and c == 0:
        return ax_sq
    if b == 0:
        return IRApply(ADD, (ax_sq, c_ir))
    if c == 0:
        return IRApply(ADD, (ax_sq, bx))
    return IRApply(ADD, (IRApply(ADD, (ax_sq, bx)), c_ir))


def _linear_ir(slope: Fraction, intercept: Fraction, x: IRSymbol) -> IRNode:
    """Build IR for slope·x + intercept."""
    slope_x = _scale_ir(slope, x)
    if intercept == 0:
        return slope_x
    return IRApply(ADD, (slope_x, _frac_ir(intercept)))


def _irrational_scale_ir(two_b: Fraction, d_sq: Fraction, expr: IRNode) -> IRNode:
    """Build (2B / √D²) · expr when D is irrational.

    Returns (2B · (1/√D²)) · expr = Mul(2B_ir, Mul(Inv(√D²_ir), expr)).
    When 2B == 0 the whole term vanishes — caller already guards against
    A == 0 leaving only an arctan; here the arctan always has a non-zero
    coefficient because B == 0 and A == 0 simultaneously would mean the
    integrand is 0 (impossible after Hermite's proper-fraction guarantee).
    """
    from symbolic_ir import INV
    sqrt_d = IRApply(SQRT, (_frac_ir(d_sq),))
    inv_sqrt = IRApply(INV, (sqrt_d,))
    scaled = IRApply(MUL, (inv_sqrt, expr))
    return _scale_ir(two_b, scaled)


__all__ = ["arctan_integral"]
