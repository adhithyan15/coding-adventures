"""Polynomial × arctan(linear) integration — Phase 11.

Integrates ``P(x) · atan(ax+b)`` for ``P ∈ Q[x]`` and ``a ∈ Q \\ {0}`` via
integration by parts.

    IBP: u = atan(ax+b),  dv = P(x) dx
         du = a/((ax+b)²+1) dx,  v = Q(x) = ∫P(x) dx

Closed form:

    ∫ P(x)·atan(ax+b) dx
        =  Q(x) · atan(ax+b)  −  a · ∫ Q(x)/((ax+b)²+1) dx

The residual rational integral is resolved by polynomial long division of
Q(x) by D(x) = (ax+b)²+1 = a²x² + 2abx + (b²+1):

    Q = S·D + R     (deg R < 2)

    ∫ Q/D dx  =  T(x)  +  arctan_integral(R, D)

where T(x) = ∫S(x) dx (polynomial) and ``arctan_integral`` is the existing
Phase 2e helper.  D is always irreducible over Q because its discriminant
equals −4a² < 0.

Final result:

    Q(x) · atan(ax+b)  −  a·T(x)  −  a · arctan_integral(R, D)

See ``code/specs/phase11-poly-arctan.md`` for the full derivation and worked
examples (P=x gives (x²+1)/2 · atan(x) − x/2; P=x² gives x³/3·atan(x) −
x²/6 + log(x²+1)/6).
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import (
    Polynomial,
    divmod_poly,
    normalize,
)
from symbolic_ir import (
    ATAN,
    MUL,
    NEG,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.arctan_integral import arctan_integral
from symbolic_vm.polynomial_bridge import from_polynomial, linear_to_ir


def atan_poly_integral(
    poly: Polynomial,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ poly(x) · atan(ax+b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is a polynomial with ``Fraction`` coefficients, non-empty.

    Always returns a closed-form IR.
    """
    p = tuple(Fraction(c) for c in normalize(poly))
    if not p:
        return IRInteger(0)

    # Q(x) = ∫ p(x) dx  (polynomial antiderivative, constant = 0).
    Q = _integrate_poly(p)

    # D(x) = (ax+b)² + 1  expanded in ascending coefficient order.
    # (ax+b)² + 1 = a²x² + 2abx + (b²+1)
    D: tuple = (b * b + Fraction(1), Fraction(2) * a * b, a * a)

    # Long division: Q(x) = S(x)·D(x) + R(x),  deg R < 2.
    Q_fracs = tuple(Fraction(c) for c in Q)
    S_raw, R_raw = divmod_poly(Q_fracs, D)
    S = normalize(S_raw)
    R = normalize(R_raw)

    # T(x) = ∫ S(x) dx.
    T = _integrate_poly(S) if S else ()

    # ∫ R(x)/D(x) dx via arctan_integral (Phase 2e).
    # Pad R to length 2 so arctan_integral can read both coefficients.
    R_2: tuple = (R[0] if len(R) > 0 else Fraction(0),
                  R[1] if len(R) > 1 else Fraction(0))
    arctan_part = arctan_integral(R_2, D, x_sym)

    # Build Q(x)·atan(ax+b).
    arg_ir = linear_to_ir(a, b, x_sym)
    atan_ir = IRApply(ATAN, (arg_ir,))
    Q_ir = from_polynomial(Q, x_sym)
    atan_term = IRApply(MUL, (Q_ir, atan_ir))

    # remaining = a · (T(x) + arctan_part).
    T_normalized = normalize(T)
    if T_normalized:
        T_ir = from_polynomial(T_normalized, x_sym)
        inner = IRApply(MUL, (_frac_ir(a), _add_ir(T_ir, arctan_part)))
    else:
        inner = _mul_by_a(a, arctan_part)

    # Result = Q·atan(ax+b) - inner.
    return IRApply(SUB, (atan_term, inner))


# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------


def _integrate_poly(p: Polynomial) -> Polynomial:
    """Polynomial antiderivative: aᵢ·xⁱ → aᵢ/(i+1)·xⁱ⁺¹."""
    if not p:
        return ()
    result: list[Fraction] = [Fraction(0)]
    for i, c in enumerate(p):
        result.append(Fraction(c) / Fraction(i + 1))
    return normalize(tuple(result))


def _frac_ir(f: Fraction) -> IRNode:
    """Convert a Fraction to an IRInteger or IRRational node."""
    if f.denominator == 1:
        return IRInteger(f.numerator)
    return IRRational(f.numerator, f.denominator)


def _add_ir(a: IRNode, b: IRNode) -> IRNode:
    from symbolic_ir import ADD
    return IRApply(ADD, (a, b))


def _mul_by_a(a: Fraction, node: IRNode) -> IRNode:
    """Return ``a · node``, simplified for a = ±1."""
    if a == Fraction(1):
        return node
    if a == Fraction(-1):
        return IRApply(NEG, (node,))
    return IRApply(MUL, (_frac_ir(a), node))


__all__ = ["atan_poly_integral"]
