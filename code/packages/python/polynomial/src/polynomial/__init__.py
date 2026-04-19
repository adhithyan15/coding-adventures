"""
polynomial — Univariate polynomial arithmetic.

A polynomial is represented as a tuple of coefficients where the index
equals the degree of that term:

    (3, 0, 2)  →  3 + 0·x + 2·x²
    ()         →  the zero polynomial

This "little-endian" convention makes addition position-aligned and
Horner's method natural to implement.

All functions return normalized polynomials — trailing zeros are stripped.
So (1, 0, 0) and (1,) both represent the constant polynomial 1.

## Coefficient types

The module is coefficient-agnostic. Every operation only uses ``+``,
``-``, ``*``, ``/``, and comparison with ``0`` — so any numeric type
that supports those works. In practice the callers are:

- ``gf256`` / ``reed-solomon`` use ``int`` coefficients (GF(2^8)).
- The symbolic integrator uses ``fractions.Fraction`` coefficients for
  exact rational arithmetic on Q[x] (see ``symbolic-computation.md``
  Phase 2).

Accumulators seed with the integer ``0`` — the additive identity for
every supported type. Seeding with ``0.0`` would silently demote
``Fraction`` to ``float`` and break exact arithmetic.
"""

from __future__ import annotations

VERSION = "0.2.0"

# Polynomial type alias: a tuple of numbers where index = degree.
# The zero polynomial is the empty tuple ().
# ``int | float`` is retained for backwards compatibility, but Fraction
# and any other numeric type with +/-/*// also works — the functions
# never inspect the concrete type.
Polynomial = tuple[int | float, ...]


# =============================================================================
# Fundamentals
# =============================================================================


def normalize(p: Polynomial) -> Polynomial:
    """Remove trailing zeros from a polynomial.

    Trailing zeros represent zero-coefficient high-degree terms. They do not
    change the mathematical value but affect degree comparisons and division.

    Examples:
        >>> normalize((1.0, 0.0, 0.0))
        (1.0,)
        >>> normalize((0.0,))
        ()
        >>> normalize(())
        ()
        >>> normalize((1.0, 0.0, 2.0))
        (1.0, 0.0, 2.0)
    """
    lst = list(p)
    while lst and lst[-1] == 0:
        lst.pop()
    return tuple(lst)


def degree(p: Polynomial) -> int:
    """Return the degree of a polynomial.

    The degree is the index of the highest non-zero coefficient.
    By convention, the zero polynomial has degree -1.

    This sentinel value (-1) lets polynomial long division terminate:
    the loop condition ``degree(remainder) >= degree(divisor)`` is False
    when remainder is zero.

    Examples:
        >>> degree(())
        -1
        >>> degree((0.0,))
        -1
        >>> degree((7.0,))
        0
        >>> degree((3.0, 0.0, 2.0))
        2
    """
    n = normalize(p)
    return len(n) - 1  # -1 when empty (zero polynomial)


def zero() -> Polynomial:
    """Return the zero polynomial ().

    Zero is the additive identity: add(zero(), p) == p for any p.
    """
    return ()


def one() -> Polynomial:
    """Return the multiplicative identity polynomial (1,).

    multiply(one(), p) == p for any p.
    """
    return (1,)


# =============================================================================
# Addition and Subtraction
# =============================================================================


def add(a: Polynomial, b: Polynomial) -> Polynomial:
    """Add two polynomials term-by-term.

    Coefficients at the same index are added; the shorter polynomial is
    implicitly zero-padded.

    Example:
        [1,2,3] + [4,5] = [5,7,3]
        1+2x+3x² + 4+5x = 5+7x+3x²

    Examples:
        >>> add((1, 2, 3), (4, 5))
        (5, 7, 3)
        >>> add((), (1, 2))
        (1, 2)
    """
    length = max(len(a), len(b))
    result = [
        (a[i] if i < len(a) else 0) + (b[i] if i < len(b) else 0)
        for i in range(length)
    ]
    return normalize(tuple(result))


def subtract(a: Polynomial, b: Polynomial) -> Polynomial:
    """Subtract polynomial b from polynomial a term-by-term.

    Example:
        [5,7,3] - [1,2,3] = [4,5,0] → (4,5)

    Examples:
        >>> subtract((5, 7, 3), (1, 2, 3))
        (4, 5)
        >>> subtract((1, 2, 3), (1, 2, 3))
        ()
    """
    length = max(len(a), len(b))
    result = [
        (a[i] if i < len(a) else 0) - (b[i] if i < len(b) else 0)
        for i in range(length)
    ]
    return normalize(tuple(result))


# =============================================================================
# Multiplication
# =============================================================================


def multiply(a: Polynomial, b: Polynomial) -> Polynomial:
    """Multiply two polynomials using polynomial convolution.

    Each term a[i]·xⁱ of a multiplies each term b[j]·xʲ of b,
    contributing a[i]·b[j] to the result at index i+j.

    Example:
        (1+2x)(3+4x) = 3 + 10x + 8x²

        >>> multiply((1, 2), (3, 4))
        (3, 10, 8)

    Examples:
        >>> multiply((1, 2), (3, 4))
        (3, 10, 8)
        >>> multiply((), (1, 2, 3))
        ()
        >>> multiply((1,), (5, 6))
        (5, 6)
    """
    if not a or not b:
        return ()

    result_len = len(a) + len(b) - 1
    # Seed with int 0 — not 0.0 — so Fraction/int inputs don't silently
    # demote to float. ``0 + Fraction(1, 2)`` stays Fraction; ``0.0 +
    # Fraction(1, 2)`` becomes 0.5 and loses exactness.
    result = [0] * result_len
    for i, ai in enumerate(a):
        for j, bj in enumerate(b):
            result[i + j] += ai * bj
    return normalize(tuple(result))


# =============================================================================
# Division
# =============================================================================


def divmod_poly(a: Polynomial, b: Polynomial) -> tuple[Polynomial, Polynomial]:
    """Polynomial long division: return (quotient, remainder).

    Finds q and r such that: a = b * q + r and degree(r) < degree(b).

    The algorithm is the polynomial analog of school long division.
    See the spec (MA00-polynomial.md) for a detailed step-by-step example.

    Note: Named ``divmod_poly`` to avoid shadowing Python's built-in ``divmod``.

    Args:
        a: Dividend polynomial.
        b: Divisor polynomial (must not be zero).

    Returns:
        A tuple (quotient, remainder).

    Raises:
        ValueError: If b is the zero polynomial.

    Examples:
        >>> divmod_poly((5, 1, 3, 2), (2, 1))
        ((3.0, -1.0, 2.0), (-1.0,))
        >>> divmod_poly((1, 2), (0, 0, 1))  # low degree dividend
        ((), (1, 2))
    """
    nb = normalize(b)
    if not nb:
        raise ValueError("polynomial division by zero")

    na = normalize(a)
    deg_a = len(na) - 1
    deg_b = len(nb) - 1

    # If dividend has lower degree than divisor, quotient is 0, remainder is a.
    if deg_a < deg_b:
        return ((), na)

    # Work on a mutable copy of the remainder.
    rem = list(na)
    quot = [0] * (deg_a - deg_b + 1)
    lead_b = nb[deg_b]
    deg_rem = deg_a

    while deg_rem >= deg_b:
        lead_rem = rem[deg_rem]
        coeff = lead_rem / lead_b
        power = deg_rem - deg_b
        quot[power] = coeff

        # Subtract coeff · x^power · b from rem.
        for j in range(len(nb)):
            rem[power + j] -= coeff * nb[j]

        # Decrement deg_rem, skipping new trailing zeros.
        deg_rem -= 1
        while deg_rem >= 0 and rem[deg_rem] == 0:
            deg_rem -= 1

    return (normalize(tuple(quot)), normalize(tuple(rem)))


def divide(a: Polynomial, b: Polynomial) -> Polynomial:
    """Return the quotient of divmod_poly(a, b).

    Raises:
        ValueError: If b is the zero polynomial.
    """
    return divmod_poly(a, b)[0]


def mod(a: Polynomial, b: Polynomial) -> Polynomial:
    """Return the remainder of divmod_poly(a, b).

    Raises:
        ValueError: If b is the zero polynomial.
    """
    return divmod_poly(a, b)[1]


# =============================================================================
# Evaluation
# =============================================================================


def evaluate(p: Polynomial, x):
    """Evaluate a polynomial at x using Horner's method.

    Horner's method rewrites the polynomial as:
        a₀ + x(a₁ + x(a₂ + ... + x·aₙ))

    This needs only n additions and n multiplications — no powers of x.

    The return type matches the coefficient / ``x`` ring: pass ``int``
    coefficients and an ``int`` x to get an ``int`` back; pass
    ``Fraction`` to stay exact.

    Algorithm:
        acc = 0
        for i from degree downto 0:
            acc = acc * x + p[i]
        return acc

    Example:
        evaluate((3, 1, 2), 4) → 39
        Because 3 + 4 + 2·16 = 3 + 4 + 32 = 39.

    Examples:
        >>> evaluate((), 5)
        0
        >>> evaluate((3, 1, 2), 4)
        39
        >>> evaluate((7,), 100)
        7
    """
    n = normalize(p)
    if not n:
        return 0
    acc = 0
    for coeff in reversed(n):
        acc = acc * x + coeff
    return acc


# =============================================================================
# Greatest Common Divisor
# =============================================================================


def gcd(a: Polynomial, b: Polynomial) -> Polynomial:
    """Compute the GCD of two polynomials using the Euclidean algorithm.

    Repeatedly replaces (a, b) with (b, a mod b) until b is the zero
    polynomial. The last non-zero remainder is the GCD.

    This mirrors integer GCD exactly, with polynomial mod in place of
    integer mod. Used in Reed-Solomon decoding.

    Examples:
        >>> gcd((), (1, 2))
        (1, 2)
        >>> degree(gcd(multiply((1, 1), (2, 1)), multiply((1, 1), (3, 1))))
        1
    """
    u = normalize(a)
    v = normalize(b)
    while v:
        r = mod(u, v)
        u = v
        v = r
    return normalize(u)


# =============================================================================
# Calculus and factorization extensions
# =============================================================================
#
# These extensions power the symbolic integrator (Phase 2 of
# symbolic-computation.md). Over Q[x] — i.e. with ``Fraction``
# coefficients — ``deriv``, ``monic``, and ``squarefree`` together are
# the primitives Hermite reduction and Rothstein–Trager are built on.
#
# They work over any field. ``monic`` and ``squarefree`` assume division
# by a non-zero leading coefficient stays in the same ring — fine for
# Q, R, C, and finite fields; not well-defined over Z (where 1/c isn't
# an integer). The gf256 / reed-solomon callers don't exercise these
# functions, so the existing integer-only paths are unaffected.


def deriv(p: Polynomial) -> Polynomial:
    """Return the formal derivative of ``p``.

    Rule: ``(a_0 + a_1·x + a_2·x² + … + a_n·x^n)' = a_1 + 2·a_2·x + … + n·a_n·x^(n-1)``.

    The derivative of a constant (degree 0 or the zero polynomial) is
    the zero polynomial. The coefficient ring must support
    multiplication by a Python ``int``; ``Fraction * int`` and
    ``float * int`` both stay in the ring.

    Examples:
        >>> deriv(())
        ()
        >>> deriv((7,))
        ()
        >>> deriv((3, 1, 2))        # 3 + x + 2x²  →  1 + 4x
        (1, 4)
        >>> deriv((0, 0, 0, 5))     # 5x³  →  15x²
        (0, 0, 15)
    """
    n = normalize(p)
    if len(n) <= 1:
        return ()
    return normalize(tuple(i * c for i, c in enumerate(n) if i > 0))


def monic(p: Polynomial) -> Polynomial:
    """Normalize ``p`` to be monic — leading coefficient equal to ``1``.

    Divides every coefficient by the leading one. The zero polynomial
    passes through unchanged (no leading coefficient to divide by).

    Requires that division by the leading coefficient stays in the
    coefficient ring — i.e. the ring must be a field. Over Q this is
    always true; over Z it is not, and the caller gets a ``Fraction``
    back from ``int / int``.

    Examples:
        >>> from fractions import Fraction
        >>> monic(())
        ()
        >>> monic((Fraction(2), Fraction(4), Fraction(6)))
        (Fraction(1, 3), Fraction(2, 3), Fraction(1, 1))
        >>> monic((3, 6, 9))   # ints become Fractions via true division
        (1.0, 2.0, 3.0)
    """
    n = normalize(p)
    if not n:
        return ()
    lead = n[-1]
    return normalize(tuple(c / lead for c in n))


def squarefree(p: Polynomial) -> list[Polynomial]:
    """Squarefree factorization via Yun's algorithm.

    Decomposes ``p`` into monic squarefree factors ``[s_1, s_2, …, s_k]``
    such that

        p  =  c · s_1 · s_2² · s_3³ · … · s_k^k

    where ``c`` is a constant (the original leading coefficient), every
    ``s_i`` is monic and squarefree, and any two distinct ``s_i``, ``s_j``
    are coprime. Multiplicity is encoded by **position**, not inside the
    factor — ``s_2`` is the product of all double roots, ``s_3`` the
    product of all triple roots, and so on.

    Yun's algorithm uses only GCD and derivative — no factoring over
    irreducibles. It works over any field of characteristic 0 (which is
    all this package cares about for the CAS path).

    The sketch: if ``p = c · ∏_i s_i^i``, then ``gcd(p, p') = ∏_i s_i^(i-1)``
    carries multiplicities one lower. Dividing and differencing
    repeatedly peels off one squarefree layer at a time.

    For ``p = (x - 1)·(x - 2)²·(x - 3)³`` the result is
    ``[x-1, x-2, x-3]`` — three entries, one per multiplicity.

    The zero polynomial returns ``[]``. A constant polynomial returns
    ``[]`` (nothing squarefree to factor out — the constant is absorbed
    into the implicit leading coefficient ``c``).

    Examples:
        >>> from fractions import Fraction
        >>> # (x - 1) — already squarefree, degree 1
        >>> squarefree((Fraction(-1), Fraction(1)))
        [(Fraction(-1, 1), Fraction(1, 1))]
    """
    n = normalize(p)
    if len(n) <= 1:
        # Zero polynomial and constants have no squarefree factors
        # beyond the absorbed leading coefficient.
        return []

    # Work over the monic polynomial. The leading coefficient
    # becomes the implicit prefactor; the recorded factors are monic.
    a = monic(n)
    a_prime = deriv(a)

    # b := gcd(a, a'). Over a field of characteristic 0, the GCD is
    # ∏ s_i^(i-1) — every factor in ``a`` with its multiplicity
    # dropped by one. If b is 1, ``a`` is already squarefree.
    b = monic(gcd(a, a_prime))

    if degree(b) <= 0:
        return [a]

    # Yun's recurrence:
    #   c_1 := a / b   (one copy of every distinct factor: ∏ s_i)
    #   d_1 := a' / b  (derivative rescaled)
    # Then iterate until c becomes constant:
    #   v_k := d_k - c_k'
    #   s_k := gcd(c_k, v_k)          ← the k-th squarefree factor
    #   c_{k+1} := c_k / s_k
    #   d_{k+1} := v_k / s_k
    # Termination: degree(c) strictly decreases each round.
    c = divide(a, b)
    d = divide(a_prime, b)

    factors: list[Polynomial] = []
    while degree(c) > 0:
        v = subtract(d, deriv(c))
        s = monic(gcd(c, v))
        factors.append(s)
        c = divide(c, s)
        d = divide(v, s)

    return factors
