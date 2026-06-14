"""Polynomial × atanh(linear) integration — Phase 14c.

Integrates ``P(x) · atanh(ax+b)`` for ``P ∈ Q[x]`` and ``a ∈ Q \\ {0}``
via integration by parts.

    IBP: u = atanh(ax+b),  dv = P(x) dx
         du = a/(1−(ax+b)²) dx,  v = Q(x) = ∫P(x) dx

Closed form:

    ∫ P(x)·atanh(ax+b) dx  =  Q(x)·atanh(ax+b)  −  a·∫ Q(x)/(1−(ax+b)²) dx

The residual rational integral is resolved by polynomial long division of
Q(x) by D(x) = 1−(ax+b)² = −a²x² − 2abx + (1−b²):

    Q = S·D + R     (deg R ≤ 1)
    R(x) = r₁·x + r₀

    ∫ Q/D dx  =  T(x)  +  ∫ R(x)/D dx

where T = ∫S dx (polynomial antiderivative) and the residual is computed by
substituting t = ax+b (dt = a dx):

    ∫ R(x)/D dx  =  (1/a²)·∫ (r₁·t + (r₀·a − r₁·b))/(1−t²) dt

Using:

    ∫ t/(1−t²) dt   =  −(1/2)·log(1−t²)
    ∫ 1/(1−t²) dt   =  atanh(t)

we get:

    ∫ R(x)/D dx  =  −(r₁/(2a²))·log(1−(ax+b)²)
                  +  (r₀/a − r₁·b/a²)·atanh(ax+b)

Final result:

    [Q(x) − (r₀ − r₁·b/a)] · atanh(ax+b)  −  a·T(x)
                                            +  (r₁/(2a))·log(1−(ax+b)²)

**Verification** (P=1, bare atanh):  Q = x, S = 0, r₁ = 1, r₀ = 0:

    → (x + b/a)·atanh(ax+b) + (1/(2a))·log(1−(ax+b)²)  ✓

**Verification** (P=x, a=1, b=0):  Q = x²/2, S = −1/2, r₁ = 0, r₀ = 1/2:

    → [x²/2 − 1/2]·atanh(x) − (−x/2) + 0·log(…)
    =  (x²−1)/2·atanh(x) + x/2                           ✓

Contrast with ``atan_poly_integral.py`` (Phase 11) which uses the
denominator ``D = (ax+b)²+1`` (always positive); here ``D = 1−(ax+b)²``
is positive only for ``|ax+b| < 1`` (the domain where atanh is real-valued).
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import (
    Polynomial,
    divmod_poly,
    normalize,
)
from symbolic_ir import (
    ADD,
    ATANH,
    LOG,
    MUL,
    NEG,
    POW,
    SUB,
    IRApply,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

from symbolic_vm.polynomial_bridge import from_polynomial, linear_to_ir


def atanh_poly_integral(
    poly: Polynomial,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ poly(x) · atanh(ax+b) dx``.

    Pre-conditions (caller's responsibility):

    - ``a ≠ 0``
    - ``poly`` is a non-empty polynomial with ``Fraction`` coefficients.

    Always returns a closed-form IR.

    Parameters
    ----------
    poly :
        Non-empty polynomial in ascending coefficient order (index i = xⁱ).
    a, b :
        Linear coefficients for the atanh argument ``ax+b``.
    x_sym :
        Integration variable symbol.

    Returns
    -------
    IR node for the antiderivative.

    Examples
    --------
    ::

        # ∫ atanh(x) dx = x·atanh(x) + (1/2)·log(1−x²)
        atanh_poly_integral((Fraction(1),), Fraction(1), Fraction(0), IRSymbol("x"))

        # ∫ x·atanh(x) dx = (x²−1)/2·atanh(x) + x/2
        atanh_poly_integral(
            (Fraction(0), Fraction(1)),
            Fraction(1), Fraction(0), IRSymbol("x")
        )
    """
    p = tuple(Fraction(c) for c in normalize(poly))
    if not p:
        return IRInteger(0)

    # Q(x) = ∫ p(x) dx  (polynomial antiderivative, integration constant = 0).
    Q = _integrate_poly(p)

    # D(x) = 1 − (ax+b)²  in ascending coefficient order.
    # Expanded: 1 − a²x² − 2abx − b² = (1−b²) + (−2ab)x + (−a²)x²
    D: tuple = (Fraction(1) - b * b, Fraction(-2) * a * b, -a * a)

    # Long division: Q(x) = S(x)·D(x) + R(x),  deg R < 2.
    Q_fracs = tuple(Fraction(c) for c in Q)
    S_raw, R_raw = divmod_poly(Q_fracs, D)
    S = normalize(S_raw)
    R = normalize(R_raw)

    # Remainder coefficients (pad to length 2).
    r0 = R[0] if len(R) > 0 else Fraction(0)
    r1 = R[1] if len(R) > 1 else Fraction(0)

    # T(x) = ∫ S(x) dx.
    T = _integrate_poly(S) if S else ()

    # atanh_correction = r₀ − r₁·b/a
    # Subtract from Q's constant term to get the atanh coefficient polynomial.
    atanh_correction = r0 - r1 * b / a

    # log coefficient: r₁ / (2a)
    log_coeff = r1 / (Fraction(2) * a)

    # ─── Build IR ────────────────────────────────────────────────────────────
    arg_ir = linear_to_ir(a, b, x_sym)
    atanh_ir = IRApply(ATANH, (arg_ir,))

    # Compute Q_mod = Q − atanh_correction (adjust the constant term).
    Q_list = list(Q_fracs)
    if Q_list:
        Q_list[0] = Q_list[0] - atanh_correction
    else:
        Q_list = [-atanh_correction]
    Q_mod = normalize(tuple(Q_list))

    # Q_mod(x) · atanh(ax+b)
    Q_mod_ir = from_polynomial(Q_mod, x_sym) if Q_mod else IRInteger(0)
    result: IRNode = IRApply(MUL, (Q_mod_ir, atanh_ir))

    # − a · T(x)
    T_normalized = normalize(T)
    if T_normalized:
        T_ir = from_polynomial(T_normalized, x_sym)
        poly_term = _mul_by_a(a, T_ir)
        result = IRApply(SUB, (result, poly_term))

    # + (r₁/(2a)) · log(1 − (ax+b)²)
    if log_coeff != Fraction(0):
        log_arg = IRApply(SUB, (IRInteger(1), IRApply(POW, (arg_ir, IRInteger(2)))))
        log_node = IRApply(LOG, (log_arg,))
        log_term = _mul_by_a(log_coeff, log_node)
        result = IRApply(ADD, (result, log_term))

    return result


# ---------------------------------------------------------------------------
# Private helpers
# ---------------------------------------------------------------------------


def _integrate_poly(p: Polynomial) -> Polynomial:
    """Polynomial antiderivative: aᵢ·xⁱ → aᵢ/(i+1)·xⁱ⁺¹ (constant = 0)."""
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


def _mul_by_a(a: Fraction, node: IRNode) -> IRNode:
    """Return ``a · node``, simplified for a = ±1."""
    if a == Fraction(1):
        return node
    if a == Fraction(-1):
        return IRApply(NEG, (node,))
    return IRApply(MUL, (_frac_ir(a), node))


__all__ = ["atanh_poly_integral"]
