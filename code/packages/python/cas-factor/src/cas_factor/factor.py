"""Top-level ``factor_integer_polynomial`` orchestrator.

Given an integer polynomial as a coefficient list, returns:

1. The integer ``content`` — the GCD of every coefficient.
2. A list of ``(factor, multiplicity)`` pairs over Z where each
   ``factor`` is itself a coefficient list.

Phase 1 extracts *linear* factors via the rational-root test.
Phase 2 (Kronecker) recursively factors any remaining residual into
irreducible pieces over Z.

Examples::

    factor_integer_polynomial([-1, 0, 1])
    # (1, [([-1, 1], 1), ([1, 1], 1)])
    # i.e. 1 * (x - 1) * (x + 1)

    factor_integer_polynomial([2, 4, 2])
    # (2, [([1, 1], 2)])
    # i.e. 2 * (x + 1)^2

    factor_integer_polynomial([1, 0, 1])
    # (1, [([1, 0, 1], 1)])
    # i.e. 1 * (x^2 + 1) — irreducible over Z

    factor_integer_polynomial([4, 0, 0, 0, 1])
    # (1, [([2, 2, 1], 1), ([2, -2, 1], 1)])
    # i.e. 1 * (x^2 + 2x + 2) * (x^2 - 2x + 2)  [Sophie Germain identity]
"""

from __future__ import annotations

from cas_factor.kronecker import kronecker_factor
from cas_factor.polynomial import Poly, content, degree, normalize, primitive_part
from cas_factor.rational_roots import extract_linear_factors

# A factored polynomial is a list of (factor_coeffs, multiplicity).
FactorList = list[tuple[Poly, int]]


def _factor_residual(residual: Poly) -> FactorList:
    """Recursively factor *residual* using Kronecker's method.

    Starts a work-queue with the residual.  At each step, if Kronecker
    finds a split, both pieces are re-queued.  Otherwise the piece is
    recorded as irreducible.  Identical factors accumulate their
    multiplicities.
    """
    factors_dict: dict[tuple[int, ...], int] = {}
    queue: list[Poly] = [normalize(residual)]

    while queue:
        piece = normalize(queue.pop())
        if not piece or degree(piece) <= 0:
            continue

        if degree(piece) == 1:
            # Linear piece — normalise to positive leading coefficient.
            if piece[-1] < 0:
                piece = [-c for c in piece]
            key = tuple(piece)
            factors_dict[key] = factors_dict.get(key, 0) + 1
            continue

        split = kronecker_factor(piece)
        if split is None:
            # Irreducible (within the combo budget).
            if piece[-1] < 0:
                piece = [-c for c in piece]
            key = tuple(piece)
            factors_dict[key] = factors_dict.get(key, 0) + 1
        else:
            f1, f2 = split
            queue.append(f1)
            queue.append(f2)

    return [(list(k), mult) for k, mult in factors_dict.items()]


def factor_integer_polynomial(p: Poly) -> tuple[int, FactorList]:
    """Factor ``p`` over Z[x].

    Returns ``(content, factors)`` where ``content`` is the positive
    integer GCD of all coefficients and ``factors`` is a list of
    ``(poly_coeffs, multiplicity)`` pairs whose product equals the
    primitive part of ``p``.
    """
    if not p:
        return (0, [])
    c = content(p)
    pp = primitive_part(p)
    linear_factors, residual = extract_linear_factors(pp)

    factors: FactorList = []
    for root, mult in linear_factors:
        # Linear factor (x − root) → coefficients [−root, 1].
        factors.append(([-root, 1], mult))

    # The residual may still factor further (irreducible quadratics, Sophie
    # Germain quartics, repeated quadratics, …).  Append iff non-trivial.
    if residual and residual != [1] and residual != [-1]:
        factors.extend(_factor_residual(residual))
    elif residual == [-1]:
        # Pull the −1 sign into the content.
        c = -c

    return (c, factors)
