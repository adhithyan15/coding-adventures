"""
polynomial — Polynomial arithmetic over real numbers.

A polynomial is represented as a tuple of floats where the index equals
the degree of that term's coefficient:

    (3.0, 0.0, 2.0)  →  3 + 0·x + 2·x²
    ()               →  the zero polynomial

This "little-endian" convention makes addition position-aligned and
Horner's method natural to implement.

All functions return normalized polynomials — trailing zeros are stripped.
So (1.0, 0.0, 0.0) and (1.0,) both represent the constant polynomial 1.
"""

from __future__ import annotations

VERSION = "0.1.0"

# Polynomial type alias: a tuple of numbers where index = degree.
# The zero polynomial is the empty tuple ().
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
    result = [0.0] * result_len
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
    quot = [0.0] * (deg_a - deg_b + 1)
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


def evaluate(p: Polynomial, x: float) -> float:
    """Evaluate a polynomial at x using Horner's method.

    Horner's method rewrites the polynomial as:
        a₀ + x(a₁ + x(a₂ + ... + x·aₙ))

    This needs only n additions and n multiplications — no powers of x.

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
        39.0
        >>> evaluate((7,), 100)
        7
    """
    n = normalize(p)
    if not n:
        return 0
    acc: float = 0
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
