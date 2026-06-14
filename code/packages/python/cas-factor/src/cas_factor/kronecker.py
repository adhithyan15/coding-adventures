"""Kronecker's algorithm for finding non-trivial polynomial factors over Z.

Given a *primitive* integer polynomial p of degree d ≥ 2 with no rational
roots (linear factors already extracted), this module attempts to find a
factor of degree k (1 ≤ k ≤ ⌊d/2⌋) using the following approach:

1. Choose k+1 distinct evaluation points a₀, a₁, …, aₖ.
2. Compute vᵢ = p(aᵢ).  Any factor q satisfies q(aᵢ) | p(aᵢ).
3. Enumerate all combinations of signed divisors (one per point).
4. Lagrange-interpolate the unique degree-≤k polynomial q through the
   chosen value pairs (aᵢ, dᵢ).
5. If q has integer coefficients and divides p exactly, return (q, p/q).

Worked examples::

    kronecker_factor([4, 0, 0, 0, 1])   # x⁴ + 4
    # ([2, 2, 1], [2, -2, 1])           # (x²+2x+2)(x²-2x+2)

    kronecker_factor([1, 0, 1, 0, 1])   # x⁴ + x² + 1
    # ([1, 1, 1], [1, -1, 1])           # (x²+x+1)(x²-x+1)

    kronecker_factor([1, 0, 1])         # x² + 1  (irreducible over Z)
    # None
"""

from __future__ import annotations

from fractions import Fraction
from itertools import product as _iproduct

from cas_factor.polynomial import (
    Poly,
    content,
    degree,
    divisors,
    evaluate,
    normalize,
)

# Hard cap on the number of divisor-combination trials per factor degree k.
_MAX_COMBOS = 10_000


# ---------------------------------------------------------------------------
# Evaluation helpers
# ---------------------------------------------------------------------------


def _eval_points(n: int) -> list[int]:
    """Return *n* evaluation points: 0, 1, −1, 2, −2, … in that order."""
    pts: list[int] = []
    i = 0
    while len(pts) < n:
        if i == 0:
            pts.append(0)
        else:
            pts.append(i)
            if len(pts) < n:
                pts.append(-i)
        i += 1
    return pts[:n]


def _signed_divisors(n: int) -> list[int]:
    """Return all signed divisors of |n|: ±d for each positive d | |n|."""
    if n == 0:
        return []
    pos = divisors(n)  # already sorted ascending
    result: list[int] = []
    for d in pos:
        result.append(d)
        result.append(-d)
    return result


# ---------------------------------------------------------------------------
# Lagrange interpolation
# ---------------------------------------------------------------------------


def _lagrange_interpolate(xs: list[int], ys: list[int]) -> list[Fraction] | None:
    """Unique polynomial through integer points (xᵢ, yᵢ) via Lagrange.

    Returns coefficient list ``[c₀, c₁, …, cₖ]`` in *ascending* degree
    order with ``Fraction`` values.  Returns ``None`` if x-points are not
    all distinct.
    """
    n = len(xs)
    result: list[Fraction] = [Fraction(0)] * n

    for i in range(n):
        # Denominator: ∏_{j≠i} (xᵢ − xⱼ)
        denom = Fraction(1)
        for j in range(n):
            if j != i:
                diff = xs[i] - xs[j]
                if diff == 0:
                    return None  # duplicate x-coordinate
                denom *= diff

        weight = Fraction(ys[i]) / denom

        # Build the basis polynomial ∏_{j≠i} (x − xⱼ) in ascending
        # coefficient order, starting with the constant polynomial 1.
        basis: list[Fraction] = [Fraction(1)]
        for j in range(n):
            if j != i:
                # Multiply basis by (x − xⱼ).
                new_basis: list[Fraction] = [Fraction(0)] * (len(basis) + 1)
                for k_idx, c in enumerate(basis):
                    new_basis[k_idx + 1] += c        # × x
                    new_basis[k_idx] -= c * xs[j]    # × (−xⱼ)
                basis = new_basis

        for k_idx in range(len(basis)):
            result[k_idx] += weight * basis[k_idx]

    return result


# ---------------------------------------------------------------------------
# Polynomial division over Q
# ---------------------------------------------------------------------------


def _poly_divmod_frac(
    a: list[Fraction], b: list[Fraction]
) -> tuple[list[Fraction], list[Fraction]]:
    """Polynomial long division over Q.

    Both *a* and *b* are in ascending coefficient order.  Returns
    ``(quotient, remainder)`` in the same order.  Trailing-zero stripping
    is applied to the remainder.
    """
    a = list(a)
    while a and a[-1] == Fraction(0):
        a.pop()
    while b and b[-1] == Fraction(0):
        b.pop()
    if not b:
        raise ZeroDivisionError("division by zero polynomial")

    db = len(b) - 1
    quot: list[Fraction] = []

    while len(a) > db:
        c = a[-1] / b[-1]
        quot.append(c)
        shift = len(a) - len(b)
        for k in range(len(b)):
            a[shift + k] -= c * b[k]
        a.pop()

    # Strip trailing zeros from remainder.
    while a and a[-1] == Fraction(0):
        a.pop()

    # quot was built highest-degree-first; reverse to ascending order.
    return (list(reversed(quot)), a)


def _divides_exactly(p: Poly, cand: Poly) -> Poly | None:
    """Return the integer cofactor if *cand* divides *p* exactly, else ``None``."""
    p_frac = [Fraction(c) for c in p]
    b_frac = [Fraction(c) for c in cand]

    while p_frac and p_frac[-1] == Fraction(0):
        p_frac.pop()
    while b_frac and b_frac[-1] == Fraction(0):
        b_frac.pop()

    if not b_frac:
        return None

    quot_frac, rem = _poly_divmod_frac(p_frac, b_frac)

    if rem:  # non-zero remainder
        return None

    # Check quotient has integer coefficients.
    quot_int: Poly = []
    for c in quot_frac:
        if c.denominator != 1:
            return None
        quot_int.append(c.numerator)

    return normalize(quot_int)


# ---------------------------------------------------------------------------
# Kronecker's method
# ---------------------------------------------------------------------------


def kronecker_factor(p: Poly) -> tuple[Poly, Poly] | None:
    """Find a non-trivial factor of primitive polynomial *p* using Kronecker.

    *p* must be primitive (``content(p) == 1``) and have degree ≥ 2.
    All linear integer-root factors must have been extracted beforehand;
    this function searches for factors of degree 1 ≤ k ≤ ⌊deg(p)/2⌋.

    Returns ``(factor, cofactor)`` — both normalised to positive leading
    coefficient and integer coefficients — or ``None`` if no non-trivial
    factor is found within the combo budget.
    """
    p = normalize(p)
    d = degree(p)
    if d < 2:
        return None

    for k in range(1, d // 2 + 1):
        pts = _eval_points(k + 1)        # k+1 distinct integer points
        vals = [evaluate(p, a) for a in pts]

        # Any zero value means p has an integer root → should have been
        # caught by extract_linear_factors, but guard defensively.
        if any(v == 0 for v in vals):
            continue

        # Build the per-point signed-divisor sets.
        div_sets = [_signed_divisors(v) for v in vals]

        # Bail early if any divisor set is empty (value was 0 — handled
        # above) or if the product of sizes exceeds the budget.
        total = 1
        for ds in div_sets:
            if not ds:
                break
            total *= len(ds)
            if total > _MAX_COMBOS:
                break

        combos_tried = 0
        for combo in _iproduct(*div_sets):
            if combos_tried >= _MAX_COMBOS:
                break
            combos_tried += 1

            # Lagrange-interpolate through (pts, combo).
            coeffs_frac = _lagrange_interpolate(pts, list(combo))
            if coeffs_frac is None:
                continue

            # Require all coefficients to be integers.
            cand: Poly = []
            ok = True
            for c in coeffs_frac:
                if c.denominator != 1:
                    ok = False
                    break
                cand.append(c.numerator)
            if not ok:
                continue

            cand = normalize(cand)
            if not cand or degree(cand) < 1:
                continue

            # Normalise: positive leading coefficient, then make primitive.
            if cand[-1] < 0:
                cand = [-c for c in cand]

            c_content = content(cand)
            if c_content > 1:
                cand = [c // c_content for c in cand]

            if degree(cand) != k:
                continue

            # Test exact division p / cand.
            cofactor = _divides_exactly(p, cand)
            if cofactor is not None and degree(cofactor) >= 1:
                if cofactor[-1] < 0:
                    cofactor = [-c for c in cofactor]
                return (cand, cofactor)

    return None
