"""Rational-root test for linear factors over Q.

By the Rational Root Theorem, any rational root ``p/q`` (in lowest
terms) of an integer polynomial ``a_0 + a_1 x + ... + a_n x^n``
satisfies ``p | a_0`` and ``q | a_n``. We enumerate the finite set
of candidates and test each.

When a root ``p/q`` is found, the corresponding linear factor is
``q*x - p`` (up to sign). After scaling so the leading coefficient is
positive, the factor is ``(-p, q)`` in coefficient-list form (constant
term first).

For Phase 1 we only chase *integer* roots — a strict subset of
rational roots that is enough for the textbook cases. Full rational
support is one straightforward extension; we'll add it once we have
fraction-coefficient polynomials in the polynomial package.
"""

from __future__ import annotations

from cas_factor.polynomial import Poly, divide_linear, divisors, evaluate, normalize


def find_integer_roots(p: Poly) -> list[int]:
    """Return all integer roots of ``p``, in canonical order.

    Repeated roots appear in the list once each — multiplicities are
    discovered by repeated division in :func:`extract_linear_factors`.
    """
    p = normalize(p)
    if not p:
        return []
    constant = p[0]
    if constant == 0:
        # x divides p — root 0 is one of the roots; recurse on the rest.
        # Strip the trailing factor of x and continue.
        # NOTE: in this Phase 1 routine we only return root 0 once;
        # multiplicity is the caller's job.
        rest = p[1:]
        return [0] + find_integer_roots(rest) if rest else [0]
    candidates = sorted(set(divisors(constant) + [-d for d in divisors(constant)]))
    return [c for c in candidates if evaluate(p, c) == 0]


def extract_linear_factors(p: Poly) -> tuple[list[tuple[int, int]], Poly]:
    """Pull out every integer-root linear factor.

    Returns:
        (factors, residual) where:
          - ``factors`` is a list of ``(root, multiplicity)`` pairs,
            sorted ascending by root.
          - ``residual`` is the polynomial that remains after all
            integer-root linear factors are divided out.

    The residual may still factor further (irreducible quadratics,
    repeated complex roots, etc.) but Phase 1 leaves that to Phase 2.
    """
    p = list(normalize(p))
    factors: dict[int, int] = {}
    while True:
        roots = find_integer_roots(p)
        if not roots:
            break
        for r in roots:
            # Pull out one copy of (x - r) per iteration.
            p = divide_linear(p, r)
            factors[r] = factors.get(r, 0) + 1
    return sorted(factors.items()), p
