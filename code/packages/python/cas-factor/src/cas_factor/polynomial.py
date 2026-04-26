"""Tiny polynomial helpers used by the factorizer.

We use a coefficient list: ``[a_0, a_1, ..., a_n]`` represents the
polynomial ``a_0 + a_1 x + ... + a_n x^n``. Trailing zeros are stripped
on every operation. The zero polynomial is the empty list.

Coefficients are integers in Phase 1. Rational coefficients
arrive via ``content`` extraction (we factor the rational scaling out
first, then operate on the integer polynomial).
"""

from __future__ import annotations

from math import gcd

Poly = list[int]


# ---------------------------------------------------------------------------
# Normalization
# ---------------------------------------------------------------------------


def normalize(p: Poly) -> Poly:
    """Strip trailing zeros."""
    out = list(p)
    while out and out[-1] == 0:
        out.pop()
    return out


def degree(p: Poly) -> int:
    p = normalize(p)
    return len(p) - 1 if p else -1


# ---------------------------------------------------------------------------
# Content
# ---------------------------------------------------------------------------


def content(p: Poly) -> int:
    """The integer GCD of every coefficient (positive).

    By convention the content of the zero polynomial is 0; the content
    of a polynomial whose only nonzero coefficient is negative is
    positive (we keep the sign in the polynomial).
    """
    p = normalize(p)
    if not p:
        return 0
    g = 0
    for c in p:
        g = gcd(g, abs(c))
    return g


def primitive_part(p: Poly) -> Poly:
    """Divide ``p`` through by its content. Returns ``[]`` for zero."""
    c = content(p)
    if c <= 1:
        return list(p)
    return [coef // c for coef in p]


# ---------------------------------------------------------------------------
# Evaluation, division
# ---------------------------------------------------------------------------


def evaluate(p: Poly, x: int) -> int:
    """Horner-rule evaluation at integer ``x``."""
    out = 0
    for c in reversed(normalize(p)):
        out = out * x + c
    return out


def divide_linear(p: Poly, root: int) -> Poly:
    """Synthetic division: divide ``p(x)`` by ``(x - root)``.

    Caller must already know ``root`` is a root (i.e. ``evaluate(p, root) == 0``).
    Returns the quotient. Behavior is undefined when ``root`` isn't an
    actual root.
    """
    p = normalize(p)
    if not p:
        return []
    n = len(p)
    quotient = [0] * (n - 1)
    remainder = 0
    for i in range(n - 1, -1, -1):
        remainder = remainder * root + p[i]
        if i > 0:
            quotient[i - 1] = remainder
    return normalize(quotient)


# ---------------------------------------------------------------------------
# Divisor enumeration
# ---------------------------------------------------------------------------


def divisors(n: int) -> list[int]:
    """All positive integer divisors of ``|n|``, sorted ascending."""
    n = abs(n)
    if n == 0:
        return []
    out: list[int] = []
    i = 1
    while i * i <= n:
        if n % i == 0:
            out.append(i)
            if i != n // i:
                out.append(n // i)
        i += 1
    return sorted(out)
