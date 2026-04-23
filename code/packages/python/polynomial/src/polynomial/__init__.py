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

VERSION = "0.4.0"

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


def extended_gcd(
    a: Polynomial, b: Polynomial
) -> tuple[Polynomial, Polynomial, Polynomial]:
    """Extended Euclidean algorithm: return ``(g, s, t)`` with ``s·a + t·b = g``.

    ``g`` is ``gcd(a, b)`` (up to leading-coefficient scaling; this
    routine does not force a monic result — the caller can scale if
    needed). ``s`` and ``t`` are the Bézout cofactors.

    Hermite reduction feeds this with coprime inputs (``g`` is a
    non-zero constant) and uses ``s`` to invert the multiplier in
    ``U·V'`` modulo ``V``. Over Q[x] every step stays in the field, so
    no pseudo-division tricks are needed.

    The loop maintains the invariants

        s·a + t·b = u
        s'·a + t'·b = v

    and performs the same ``(u, v) ← (v, u mod v)`` update on the
    cofactors: if ``u = q·v + r`` then
    ``s − q·s' , t − q·t'`` are the cofactors for ``r``. When ``v``
    hits zero, ``u`` is the GCD and the current ``(s, t)`` are its
    cofactors.

    Edge cases:

    - ``a = ()`` → returns ``((), zero-poly, (1,))`` (i.e. ``t = 1``
      because ``0 + 1·b = b``; gcd of zero and b is b).
    - ``b = ()`` → returns ``(a, (1,), ())``.
    - ``a = b = ()`` → returns ``((), (), ())`` — the degenerate case;
      no Bézout combination exists for the zero pair.

    Examples:
        >>> from fractions import Fraction
        >>> # gcd(x^2 - 1, x - 1) = x - 1
        >>> one = Fraction(1)
        >>> a = (Fraction(-1), Fraction(0), one)   # x^2 - 1
        >>> b = (Fraction(-1), one)                # x - 1
        >>> g, s, t = extended_gcd(a, b)
        >>> # Check: s*a + t*b == g
        >>> add(multiply(s, a), multiply(t, b)) == g
        True
    """
    u, v = normalize(a), normalize(b)
    # Cofactor pairs: (s, t) for u, (s_, t_) for v. Seed with integer 0
    # / 1 for the same Fraction-safe reason as elsewhere in the package.
    s, t = (1,), ()
    s_next, t_next = (), (1,)
    while v:
        q, r = divmod_poly(u, v)
        u, v = v, r
        # (s, t) ← (s_next, t_next); (s_next, t_next) ← (s - q·s_next,
        # t - q·t_next). The subtraction stays in the coefficient ring.
        s, s_next = s_next, subtract(s, multiply(q, s_next))
        t, t_next = t_next, subtract(t, multiply(q, t_next))
    return (u, s, t)


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


# =============================================================================
# Resultant
# =============================================================================


def resultant(a: Polynomial, b: Polynomial):
    """Scalar resultant ``res(a, b)`` over the coefficient field.

    The resultant of two polynomials is a scalar that vanishes exactly
    when ``a`` and ``b`` share a root in the algebraic closure — i.e.
    ``res(a, b) == 0  iff  gcd(a, b)`` has positive degree. Two useful
    identities follow from the definition:

        res(a, b)  =  lc(a)^deg(b) · ∏ b(α_i)  where α_i are roots of a
                   =  lc(b)^deg(a) · ∏ a(β_j)  where β_j are roots of b

    Rothstein–Trager uses this as the core of a log-coefficient finder:
    the polynomial ``resₓ(C(x) − z·E'(x), E(x)) ∈ Q[z]`` has roots
    exactly the constants that appear as coefficients in the log part
    of ``∫ C/E dx``. See ``rothstein-trager.md``.

    This implementation uses the **Euclidean-PRS** recurrence. For
    ``deg a ≥ deg b > 0`` and ``r = a mod b``:

        res(a, b) = (−1)^(deg a · deg b) · lc(b)^(deg a − deg r) · res(b, r)

    with base case ``res(a, constant c) = c^deg(a)``. Every step stays
    in the coefficient field (we never need pseudo-division over Z),
    which is exactly what the Q-coefficient CAS path wants.

    Edge cases:

    - ``a = ()`` or ``b = ()`` → returns ``0`` (the zero polynomial
      shares every root with everything).
    - ``res(a, c)`` for a non-zero constant ``c`` → ``c^deg(a)``; in
      particular ``res(a, (1,)) = 1``.
    - When ``deg a < deg b`` we swap, which flips the sign by
      ``(−1)^(deg a · deg b)``; the recurrence is stated with
      ``deg a ≥ deg b`` for bookkeeping, not because it cares which is
      larger.

    Examples:
        >>> from fractions import Fraction
        >>> # res(x - 1, x - 2) = (1)(−1) = -1 when we evaluate
        >>> #   a(β) at the root β = 2 of b.
        >>> resultant((Fraction(-1), Fraction(1)), (Fraction(-2), Fraction(1)))
        Fraction(-1, 1)
        >>> # Shared root ⇒ resultant vanishes.
        >>> xm1 = (Fraction(-1), Fraction(1))
        >>> xp2 = (Fraction(2), Fraction(1))
        >>> xp3 = (Fraction(3), Fraction(1))
        >>> resultant(multiply(xm1, xp2), multiply(xm1, xp3))
        Fraction(0, 1)
    """
    u = normalize(a)
    v = normalize(b)

    # The zero polynomial shares a (formal) root with anything.
    if not u or not v:
        return 0

    # Ensure ``u`` has no-smaller degree; remember the sign flip from
    # swapping so the recurrence returns the right value.
    sign = 1
    if degree(u) < degree(v):
        parity = (degree(u) * degree(v)) % 2
        if parity:
            sign = -sign
        u, v = v, u

    # Now iterate the Euclidean-PRS recurrence in-place.
    result = 1
    while degree(v) > 0:
        r = mod(u, v)
        if not r:
            # Shared root — ``v`` divides ``u``. Resultant is zero.
            return 0
        m = degree(u)
        n = degree(v)
        d = degree(r)
        parity = (m * n) % 2
        if parity:
            sign = -sign
        # lc(v)^(m − d) — the coefficient ring must support
        # exponentiation by a non-negative integer, which it always
        # does via repeated multiplication.
        result = result * (v[-1] ** (m - d))
        u, v = v, r

    # Base case: ``v`` is a non-zero constant. res(u, c) = c^deg(u).
    m = degree(u)
    result = result * (v[0] ** m)
    return sign * result


# =============================================================================
# Rational-root finding
# =============================================================================


def _as_fraction(x):
    """Coerce ``x`` to ``fractions.Fraction``. Local import to keep
    the module's import graph tidy — callers that don't touch rational
    roots don't pay for ``fractions``."""
    from fractions import Fraction
    return Fraction(x)


def rational_roots(p: Polynomial) -> list:
    """Return the distinct rational roots of ``p ∈ Q[z]`` in ascending order.

    Uses the **Rational Roots Theorem**: if ``p(z) = a_n·z^n + … + a_0``
    has integer coefficients and admits a rational root ``r = u/v`` in
    lowest terms, then ``u ∣ a_0`` and ``v ∣ a_n``. So every candidate
    rational root comes from a finite enumeration of ``divisors(a_0) ×
    divisors(a_n)``.

    Handles input with ``Fraction`` or mixed numeric coefficients by
    first rescaling to an integer polynomial (multiply through by the
    LCM of denominators and then by the sign of the leading
    coefficient, so ``a_n > 0``). The roots themselves are returned as
    `Fraction` values regardless.

    Roots of multiplicity ``> 1`` are returned only once — the list is
    *distinct*. This matches the Rothstein–Trager use case, where the
    resultant has distinct roots in the generic (squarefree) case, and
    any degenerate multiplicity is a symptom we treat as a non-answer.

    Edge cases:

    - Zero polynomial → ``[]`` (every value is formally a root; there's
      no meaningful finite answer).
    - Non-zero constant → ``[]`` (no roots).
    - Polynomial with no rational root (e.g. ``z² − 2``) → ``[]``.

    Examples:
        >>> from fractions import Fraction
        >>> # (z - 1)(z - 2)(z - 3)
        >>> zm1 = (Fraction(-1), Fraction(1))
        >>> zm2 = (Fraction(-2), Fraction(1))
        >>> zm3 = (Fraction(-3), Fraction(1))
        >>> rational_roots(multiply(zm1, multiply(zm2, zm3)))
        [Fraction(1, 1), Fraction(2, 1), Fraction(3, 1)]
        >>> # z^2 - 2 has no rational roots
        >>> rational_roots((Fraction(-2), Fraction(0), Fraction(1)))
        []
    """
    n = normalize(p)
    if len(n) <= 1:
        return []

    # Promote every coefficient to Fraction so the LCM step is
    # well-defined even when the caller passed int or mixed types.
    frac_coeffs = [_as_fraction(c) for c in n]

    # Clear denominators: multiply through by the LCM of all
    # denominators to get an integer-coefficient polynomial with the
    # same roots.
    import math as _math
    lcm_den = 1
    for c in frac_coeffs:
        lcm_den = lcm_den * c.denominator // _math.gcd(lcm_den, c.denominator)
    int_coeffs = [int(c * lcm_den) for c in frac_coeffs]

    # Make the leading coefficient positive — the candidate enumeration
    # is symmetric in sign, so this is just cosmetic for the divisor
    # loop.
    if int_coeffs[-1] < 0:
        int_coeffs = [-c for c in int_coeffs]

    # Enumerate divisors of |a_0| and |a_n|. Divisors of 0 is a special
    # case: z = 0 is a root iff a_0 = 0, handled separately below.
    a0, an = int_coeffs[0], int_coeffs[-1]

    def _divisors(m: int) -> list[int]:
        # ``a0`` is zero-checked at the call site (a root at z = 0
        # handled separately), and ``an`` is the leading coefficient of
        # a normalised polynomial — non-zero by construction. So ``m``
        # is always non-zero here.
        m = abs(m)
        return [d for d in range(1, m + 1) if m % d == 0]

    roots: set = set()

    if a0 == 0:
        # z = 0 is a root (constant term vanishes). Record it and
        # peel ``z`` off for the remaining search.
        roots.add(_as_fraction(0))
        # Drop the zero coefficient and re-search — the non-zero tail
        # still captures every other rational root.
        tail = normalize(tuple(int_coeffs[1:]))
        return sorted(set(rational_roots(tail)) | roots)

    p_divs = _divisors(a0)
    q_divs = _divisors(an)

    # Candidate rationals ±u/v in lowest terms. Python's Fraction
    # normalises, so the set dedupes 2/4 vs 1/2 automatically.
    candidates: set = set()
    for u in p_divs:
        for v in q_divs:
            candidates.add(_as_fraction(u) / _as_fraction(v))
            candidates.add(_as_fraction(-u) / _as_fraction(v))

    for r in candidates:
        # Evaluate with the integer-coefficient polynomial; r ·
        # Fraction stays exact.
        if evaluate(tuple(int_coeffs), r) == 0:
            roots.add(r)

    return sorted(roots)
