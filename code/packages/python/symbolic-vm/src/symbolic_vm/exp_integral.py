"""Polynomial × exp(linear) integration — Phase 3d.

Integrates ``p(x) · exp(a·x + b)`` for ``p ∈ Q[x]`` and ``a ∈ Q \\ {0}``
by solving the Risch differential equation

    g′(x) + a·g(x) = p(x)

over ``Q[x]``.  The solution always exists and is unique when ``a ≠ 0``
and ``p`` is a polynomial: ``g`` has the same degree as ``p``, and its
coefficients are determined by back-substitution starting from the
leading term:

    gₙ = pₙ / a
    gₖ = (pₖ − (k+1)·gₖ₊₁) / a     for k = n−1, ..., 0

The antiderivative is ``g(x) · exp(a·x + b)``.

See ``code/specs/phase3-transcendental.md`` for the full derivation.
"""

from __future__ import annotations

from fractions import Fraction

from polynomial import Polynomial, normalize
from symbolic_ir import EXP, MUL, IRApply, IRNode, IRSymbol

from symbolic_vm.polynomial_bridge import from_polynomial, linear_to_ir


def exp_integral(
    poly: Polynomial,
    a: Fraction,
    b: Fraction,
    x_sym: IRSymbol,
) -> IRNode:
    """Return the IR for ``∫ poly(x) · exp(a·x + b) dx``.

    Pre-conditions (caller's responsibility):
    - ``a ≠ 0``
    - ``poly`` is a non-empty polynomial with ``Fraction`` coefficients.

    The result is always a closed-form IR.  The caller must verify ``a ≠ 0``
    before calling (otherwise the Risch DE has no polynomial solution).
    """
    p = tuple(Fraction(c) for c in normalize(poly))
    if not p:
        p = (Fraction(0),)

    g = _solve_risch_de_poly(p, a)

    exp_arg = linear_to_ir(a, b, x_sym)
    exp_ir = IRApply(EXP, (exp_arg,))
    g_ir = from_polynomial(g, x_sym)
    return IRApply(MUL, (g_ir, exp_ir))


def _solve_risch_de_poly(p: Polynomial, a: Fraction) -> Polynomial:
    """Solve ``g′ + a·g = p`` in ``Q[x]``, returning ``g`` as a tuple.

    ``p`` must be non-empty (non-zero polynomial).  ``a`` must be non-zero.
    The result has the same degree as ``p``.
    """
    n = len(p) - 1  # degree of p (and g)
    g: list[Fraction] = [Fraction(0)] * (n + 1)

    # Back-substitution from the leading coefficient downward.
    g[n] = p[n] / a
    for k in range(n - 1, -1, -1):
        g[k] = (p[k] - Fraction(k + 1) * g[k + 1]) / a

    return tuple(g)


__all__ = ["exp_integral"]
