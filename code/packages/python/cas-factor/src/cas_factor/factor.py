"""Top-level ``factor_integer_polynomial`` orchestrator.

Given an integer polynomial as a coefficient list, returns:

1. The integer ``content`` — the GCD of every coefficient.
2. A list of ``(factor, multiplicity)`` pairs over Q where each
   ``factor`` is itself a coefficient list. Phase 1 only finds
   *linear* factors via the rational-root test; the residual after
   dividing all of those out is appended as a single factor with
   multiplicity 1 if it isn't trivial.

Examples::

    factor_integer_polynomial([-1, 0, 1])
    # (1, [([-1, 1], 1), ([1, 1], 1)])
    # i.e. 1 * (x - 1) * (x + 1)

    factor_integer_polynomial([2, 4, 2])
    # (2, [([1, 1], 2)])
    # i.e. 2 * (x + 1)^2

    factor_integer_polynomial([1, 0, 1])
    # (1, [([1, 0, 1], 1)])
    # i.e. 1 * (x^2 + 1) — Phase 1 leaves the irreducible quadratic intact
"""

from __future__ import annotations

from cas_factor.polynomial import Poly, content, primitive_part
from cas_factor.rational_roots import extract_linear_factors

# A factored polynomial is a list of (factor_coeffs, multiplicity).
FactorList = list[tuple[Poly, int]]


def factor_integer_polynomial(p: Poly) -> tuple[int, FactorList]:
    """Factor ``p`` over Z[x] (linear factors via rational-root test).

    Returns ``(content, factors)``. The product equals ``p`` modulo a
    sign (we pick a positive content; signs of individual factors are
    chosen so that the product matches).
    """
    if not p:
        return (0, [])
    c = content(p)
    pp = primitive_part(p)
    linear_factors, residual = extract_linear_factors(pp)

    factors: FactorList = []
    for root, mult in linear_factors:
        # Linear factor (x - root) → coefficients [-root, 1].
        factors.append(([-root, 1], mult))

    # The residual may still be ±1 (everything pulled out) or a
    # higher-degree irreducible. Append it iff it's non-trivial.
    if residual and residual != [1] and residual != [-1]:
        factors.append((residual, 1))
    elif residual == [-1]:
        # Pull the -1 into the content.
        c = -c

    return (c, factors)
