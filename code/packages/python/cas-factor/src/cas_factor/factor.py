"""Top-level ``factor_integer_polynomial`` orchestrator.

Given an integer polynomial as a coefficient list, returns:

1. The integer ``content`` — the GCD of every coefficient.
2. A list of ``(factor, multiplicity)`` pairs over Z where each
   ``factor`` is itself a coefficient list.

Phase 1 extracts *linear* factors via the rational-root test.
Phase 2 (Kronecker) recursively factors any remaining residual into
irreducible pieces over Z.
Phase 3 (BZH) handles residuals that Phase 2 missed — specifically
high-degree cyclotomic polynomials and products where Kronecker's
divisor-combo budget is exhausted.

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

    factor_integer_polynomial([-1, 0, 0, 0, 0, 1])
    # (1, [([-1, 1], 1), ([1, 1, 1, 1, 1], 1)])
    # i.e. 1 * (x - 1) * (x^4 + x^3 + x^2 + x + 1)  [BZH]

    factor_integer_polynomial([2, 0, 0, 0, 2])
    # (2, [([1, 0, 0, 0, 1], 1)])
    # i.e. 2 * (x^4 + 1) — x^4+1 is irreducible over Q  [content + BZH]
"""

from __future__ import annotations

from cas_factor.bzh import bzh_factor
from cas_factor.kronecker import kronecker_factor
from cas_factor.polynomial import Poly, content, degree, normalize, primitive_part
from cas_factor.rational_roots import extract_linear_factors

# A factored polynomial is a list of (factor_coeffs, multiplicity).
FactorList = list[tuple[Poly, int]]


def _factor_residual(residual: Poly) -> FactorList:
    """Recursively factor *residual* using Kronecker's method + BZH fallback.

    Maintains a work-queue of polynomials to factor.  At each step:

    1. If the piece is degree ≤ 1, record it directly.
    2. Try Kronecker (fast, works for many low-degree cases).
    3. If Kronecker returns ``None`` *and* the piece is monic *and* degree ≥ 4,
       try BZH (Berlekamp-Zassenhaus-Hensel).
    4. If both return ``None``, the piece is declared irreducible.

    When a split is found (either by Kronecker or BZH), both pieces are
    re-queued for further factoring.  Identical factors accumulate their
    multiplicities in ``factors_dict``.

    BZH trigger condition: ``degree ≥ 4`` and ``piece[-1] == 1`` (monic).
    This targets cyclotomic and other structured polynomials that escape
    Kronecker's divisor-combo budget but have clean modular structure.
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

        # --- Phase 2: Kronecker ---
        split = kronecker_factor(piece)

        # --- Phase 3: BZH fallback (monic degree ≥ 4 only) ---
        if split is None and degree(piece) >= 4 and piece[-1] == 1:
            bzh_result = bzh_factor(list(piece))
            if bzh_result is not None and len(bzh_result) >= 2:
                # BZH found a split — re-queue all factors.
                for fac in bzh_result:
                    queue.append(fac)
                continue

        if split is None:
            # Irreducible (within both budgets).
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
