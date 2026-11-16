"""
Trigonometric Functions from First Principles (TRIG-REF-1)
==========================================================

This module implements sine, cosine and their friends from **addition,
subtraction, multiplication and comparison alone**.  It never calls the host
``sin`` or ``cos``, and ``sin``/``cos``/``radians``/``degrees`` do not even
divide.  Every number is a binary64 ``float`` and every operation rounds once,
so the answer is *bit for bit* the same in all seventeen language ports of this
package.  The algorithm is specified in ``code/specs/PHY00a-trig-convergence.md``
and checked against the shared vectors in
``code/specs/fixtures/trig-reference-v1``.

The big idea
------------

A sine wave repeats, so knowing ``sin`` on the small interval [-pi/4, pi/4] and
*which quarter-turn* an angle sits in is enough to know it everywhere:

    x = k * (pi/2) + r        with  |r| <= pi/4   and   k a whole number

* ``sin`` of ``x`` is one of ``sin r``, ``cos r``, ``-sin r``, ``-cos r``,
  chosen by ``k mod 4``.
* ``cos`` of ``x`` is one of ``cos r``, ``-sin r``, ``-cos r``, ``sin r``.

So the work is (1) finding ``k`` and ``r`` accurately, and (2) evaluating
``sin r`` and ``cos r`` for a small ``r`` by their Maclaurin series.

Step 1: range reduction (Cody-Waite)
------------------------------------

``k`` is the whole number nearest to ``x / (pi/2)``.  We find it with an
add-and-subtract trick: ``MAGIC = 1.5 * 2**52`` has its units digit in the last
place of the significand, so ``(t + MAGIC) - MAGIC`` is ``t`` rounded to the
nearest integer, using only ``+`` and ``-`` (no ``floor``, no ``round``, no
float-to-int cast, so nothing special happens at NaN or infinity).

Then ``r = x - k*(pi/2)``.  The catch: ``pi/2`` is irrational, so a 53-bit
``pi/2`` times a big ``k`` loses the very digits we need.  The fix is to store
``pi/2`` as a *sum of five short pieces*, each at most 24 bits, so that
``k * piece`` is **exact** (24 bits + 29 bits of ``k`` = 53 bits)::

    x --------------------------------------------------------- the angle
    |-- k*PIO2_1 --|                                             24 bits of k*pi/2
                   |-- k*PIO2_2 --|                              next 24
                                  |-- k*PIO2_3 --|               next 24
                                                 |- k*PIO2_4 -|  next 24
                                                              |k*PIO2_5| last, rounds

Each subtraction strips one more slice of ``k*pi/2`` off ``x``.  The pieces add
up to ``pi/2`` with a residual of ``2**-150``.

Step 2: the series
------------------

On |r| <= pi/4 both functions are short polynomials in ``z = r*r``::

    sin r = r + r*z*(S1 + S2*z + ... + S9*z**8)      S[n] = (-1)**n / (2n+1)!
    cos r = 1 - z/2 + z*z*(C2 + C3*z + ... + C9*z**7)  C[n] = (-1)**n / (2n)!

The coefficients are the factorial table of the Maclaurin series, each rounded
*once* to a double and stored as a bit pattern, so no port ever divides.  They
are summed by **Horner's rule**, innermost first: the smallest terms are added
first, which is the accurate order, and each step is one multiply and one add.

How accurate is it?
-------------------

Within 3 ulp of the exact sine/cosine on the whole domain |x| <= 2**29 (the
vectors gate this; the worst case found is 2.38 ulp), and **not** a libm: a
correctly rounded sine is a project of its own.

The domain
----------

``sin``, ``cos`` and ``tan`` are defined for ``|x| <= 2**29`` (about 5.4e8
radians).  Beyond that, and for NaN and the infinities, they return **NaN**: the
quotient ``k`` would no longer fit the exact products above, and returning a
finite number would be a wrong answer that looks like a right one.
"""

from __future__ import annotations

import struct

# ---------------------------------------------------------------------------
# Constants, specified by bit pattern (PHY00a 3.2).
#
# A constant is a *bit pattern*; the decimal is only a convenience.  Building
# each one from its bits means no parser, on any platform, can round it wrongly.
# ---------------------------------------------------------------------------


def _f(bits: int) -> float:
    """The double whose IEEE 754 bit pattern is *bits*."""
    return struct.unpack("<d", struct.pack("<Q", bits))[0]


# Pi, and the multiples the rest of the package leans on.
PI: float = _f(0x400921FB54442D18)  # 3.141592653589793
TWO_PI: float = _f(0x401921FB54442D18)
HALF_PI: float = _f(0x3FF921FB54442D18)

# 2/pi, the double nearest to it: x * INV_PIO2 is x measured in quarter-turns.
_INV_PIO2: float = _f(0x3FE45F306DC9C883)

# pi/2 as five pieces.  The first four have at most 24 significant bits (so
# k * piece is exact for |k| < 2**29); the fifth is the remainder, rounded.
_PIO2 = (
    _f(0x3FF921FB40000000),  # 1.570796251296997
    _f(0x3E74442D00000000),  # 7.549789415861596e-08
    _f(0x3CF8469880000000),  # 5.390302529957765e-15
    _f(0x3B78CC5160000000),  # 3.282003415807913e-22
    _f(0x39F01B839A25204A),  # 1.2706558760139879e-29
)

# 1.5 * 2**52: adding then subtracting it rounds a number to the nearest integer.
_MAGIC: float = _f(0x4338000000000000)

# The edge of the domain, 2**29, and the threshold 2**-27 below which
# sin(x) == x exactly (the next term, x**3/6, is under half an ulp of x).
_LIMIT: float = _f(0x41C0000000000000)
_TINY: float = _f(0x3E40000000000000)

# Sine coefficients S[n] = (-1)**n / (2n+1)!   (index 0 is unused)
#   S1 = -1/3!   S2 = +1/5!   S3 = -1/7!   S4 = +1/9!   S5 = -1/11!
#   S6 = +1/13!  S7 = -1/15!  S8 = +1/17!  S9 = -1/19!
_S = (
    0.0,
    _f(0xBFC5555555555555),
    _f(0x3F81111111111111),
    _f(0xBF2A01A01A01A01A),
    _f(0x3EC71DE3A556C734),
    _f(0xBE5AE64567F544E4),
    _f(0x3DE6124613A86D09),
    _f(0xBD6AE7F3E733B81F),
    _f(0x3CE952C77030AD4A),
    _f(0xBC62F49B46814157),
)

# Cosine coefficients C[n] = (-1)**n / (2n)!   (indices 0 and 1 are unused:
# the n = 1 term, -1/2, is applied as the exact product 0.5 * z)
#   C2 = +1/4!   C3 = -1/6!   C4 = +1/8!   C5 = -1/10!
#   C6 = +1/12!  C7 = -1/14!  C8 = +1/16!  C9 = -1/18!
_C = (
    0.0,
    0.0,
    _f(0x3FA5555555555555),
    _f(0xBF56C16C16C16C17),
    _f(0x3EFA01A01A01A01A),
    _f(0xBE927E4FB7789F5C),
    _f(0x3E21EED8EFF8D898),
    _f(0xBDA93974A8C07C9D),
    _f(0x3D2AE7F3E733B81F),
    _f(0xBCA6827863B97D97),
)

# pi/180 and 180/pi, pre-divided so a conversion is ONE multiply (one rounding).
_RAD_PER_DEG: float = _f(0x3F91DF46A2529D39)  # 0.017453292519943295
_DEG_PER_RAD: float = _f(0x404CA5DC1A63C1F8)  # 57.29577951308232

_NAN: float = _f(0x7FF8000000000000)


def _to_float(x: float) -> float:
    """Convert an int argument to binary64 once (``float`` keeps -0.0)."""
    return x if type(x) is float else float(x)


# ---------------------------------------------------------------------------
# Range reduction
# ---------------------------------------------------------------------------


def _reduce(x: float) -> tuple[float, float]:
    """Return ``(k, r)`` with ``x = k*(pi/2) + r`` and ``|r| <= pi/4`` (about).

    Precondition: ``|x| <= 2**29`` (the callers' domain guard has run), so ``k``
    is a finite whole number below 2**29 in magnitude.

    Every product and the subtraction that uses it are written as two
    statements.  They must stay two operations: a compiler that fused
    ``r - k*p`` into one fused multiply-add would round once instead of twice
    and change the last bit.  (CPython never fuses; the habit is the contract.)
    """
    t = x * _INV_PIO2  # x measured in quarter-turns, one rounding
    k = (t + _MAGIC) - _MAGIC  # nearest whole number, ties to even
    r = x
    for piece in _PIO2:
        m = k * piece  # exact for the first four pieces
        r = r - m
    return k, r


# ---------------------------------------------------------------------------
# The series kernels, Horner's rule on stored coefficients
# ---------------------------------------------------------------------------


def _sin_k(r: float) -> float:
    """sin(r) for |r| <= pi/4:  r + r*z*(S1 + S2*z + ... + S9*z**8)."""
    z = r * r
    p = _S[9]
    for n in range(8, 0, -1):
        p = p * z
        p = p + _S[n]
    w = r * z
    w = w * p
    return r + w


def _cos_k(r: float) -> float:
    """cos(r) for |r| <= pi/4:  (1 - z/2) + z*z*(C2 + C3*z + ... + C9*z**7)."""
    z = r * r
    q = _C[9]
    for n in range(8, 1, -1):
        q = q * z
        q = q + _C[n]
    u = 0.5 * z  # exact
    u = 1.0 - u
    v = z * z
    v = v * q
    return u + v


# ---------------------------------------------------------------------------
# Sine and cosine
# ---------------------------------------------------------------------------


def sin(x: float) -> float:
    """
    Sine of *x* (radians), for ``|x| <= 2**29``; NaN outside that domain.

    The result table (PHY00a 3.3):

    ===================== ==============================
    input                 result
    ===================== ==============================
    NaN, +-inf            NaN
    ``|x| > 2**29``       NaN (outside the domain)
    +0 / -0               +0 / **-0** (``sin(-0) = -0``)
    ``0 < |x| < 2**-27``  ``x`` itself (correctly rounded)
    otherwise             reduce, then pick a kernel
    ===================== ==============================

    The domain guard is a *negated* ``<=``.  A NaN fails every comparison, so
    ``not (abs(x) <= LIMIT)`` is true for NaN, both infinities and every large
    finite value, in one test.  The tempting ``abs(x) > LIMIT`` lets a NaN
    through to ``int(k)``, which raises.
    """
    x = _to_float(x)
    if not (abs(x) <= _LIMIT):
        return _NAN
    if abs(x) < _TINY:
        return x  # also keeps -0.0 and subnormals exact
    k, r = _reduce(x)
    # k is a whole number below 2**29, so int(k) is exact; "& 3" is k mod 4
    # (floor-modulus for negatives: -1 & 3 == 3).
    q = int(k) & 3
    if q == 0:
        return _sin_k(r)
    if q == 1:
        return _cos_k(r)
    if q == 2:
        return -_sin_k(r)
    return -_cos_k(r)


def cos(x: float) -> float:
    """
    Cosine of *x* (radians), for ``|x| <= 2**29``; NaN outside that domain.

    ``cos(+-0) = 1``, and ``cos(x) = 1`` for ``|x| < 2**-27`` (the first
    correction, ``x**2/2``, is below half an ulp of the double just under 1).
    The quarter-turn table is ``cos r, -sin r, -cos r, sin r`` for
    ``k mod 4 = 0, 1, 2, 3``.
    """
    x = _to_float(x)
    if not (abs(x) <= _LIMIT):
        return _NAN
    if abs(x) < _TINY:
        return 1.0
    k, r = _reduce(x)
    q = int(k) & 3
    if q == 0:
        return _cos_k(r)
    if q == 1:
        return -_sin_k(r)
    if q == 2:
        return -_cos_k(r)
    return _sin_k(r)


# ---------------------------------------------------------------------------
# Degree / Radian Conversion
# ---------------------------------------------------------------------------


def radians(deg: float) -> float:
    """
    Convert an angle from degrees to radians: ONE multiply by ``pi/180``.

    180 degrees is pi radians, so ``radians = degrees * (pi / 180)``.  The
    constant is pre-divided and stored as a bit pattern, so the conversion
    rounds once (``deg * PI / 180`` rounds twice and can overflow its
    intermediate).  NaN propagates, infinities map to infinities and signed
    zero keeps its sign.
    """
    return _to_float(deg) * _RAD_PER_DEG


def degrees(rad: float) -> float:
    """
    Convert an angle from radians to degrees: ONE multiply by ``180/pi``.

    The inverse of :func:`radians`.  A result above the largest double is
    ``inf``.
    """
    return _to_float(rad) * _DEG_PER_RAD


# ---------------------------------------------------------------------------
# Square Root — Newton's (Babylonian) Method
# ---------------------------------------------------------------------------


def sqrt(x: float) -> float:
    """
    Compute the square root of *x* using Newton's method.

    Newton's Method for Square Roots
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    Newton's method (also called the Babylonian method — it was known to
    Babylonian mathematicians over 3,000 years ago) says: if *guess* is
    an approximation to sqrt(x), then the average of *guess* and *x/guess*
    is a better approximation.

    Intuition: if guess < sqrt(x), then x/guess > sqrt(x), so their average
    "squeezes" toward the true value from both sides.

    The convergence is **quadratic** — the number of correct decimal digits
    doubles with each iteration. For x = 2:

        Iteration 0: guess = 2.0
        Iteration 1: guess = (2.0 + 2.0/2.0) / 2 = 1.5
        Iteration 2: guess = (1.5 + 2.0/1.5) / 2 ≈ 1.41667
        Iteration 3: guess ≈ 1.41422
        Iteration 4: guess ≈ 1.41421356237...  (full precision in 4 steps!)

    Typically converges in 10–15 iterations for any normal double-precision input.
    """
    x = _to_float(x)
    if x < 0:
        raise ValueError(f"sqrt: domain error — input {x} is negative")

    # sqrt(0) is exactly 0 (and keeps the sign of -0).
    if x == 0.0:
        return x
    if x == float("inf"):
        return x

    scaled = x
    result_scale = 1.0
    while scaled < 0.25:
        scaled *= 4.0
        result_scale *= 0.5
    while scaled >= 4.0:
        scaled *= 0.25
        result_scale *= 2.0

    # Initial guess: x itself for x >= 1 (good for large numbers),
    # 1.0 for x < 1 (avoids dividing by a tiny number in the first step).
    guess: float = max(scaled, 1.0)

    # Iterate until convergence or safety limit.
    for _ in range(60):
        next_guess = (guess + scaled / guess) / 2.0

        # Stop when improvement is below the precision floor.
        # 1e-15 * guess is relative precision; 1e-300 handles subnormals.
        if abs(next_guess - guess) < 1e-15 * guess + 1e-300:
            return next_guess * result_scale

        guess = next_guess

    return guess * result_scale


# ---------------------------------------------------------------------------
# Tangent — Sine over Cosine
# ---------------------------------------------------------------------------


def tan(x: float) -> float:
    """
    Compute the tangent of *x* (in radians).

    Definition and Geometry
    ~~~~~~~~~~~~~~~~~~~~~~~

    Tangent is the ratio of sine to cosine:

        tan(x) = sin(x) / cos(x)

    On the unit circle, if you draw a vertical line tangent to the circle at
    (1, 0), then tan(x) is where the ray at angle x meets that line. This is
    the literal origin of the name "tangent."

    The pole sentinel (a pinned legacy contract)
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    Wherever cos(x) = 0 — at x = pi/2 + k*pi — tan(x) is undefined. When
    ``|cos(x)| < 1e-15`` we return +-1e308 (signed like ``sin``) instead of an
    infinity.  That is **a convention, not an approximation**: the true tangent
    of the double nearest pi/2 is about 1.6e16.  PHY00 documents it and the
    shared corpus requires it, so it is pinned.  No accuracy claim is made for
    ``tan``.

    ``tan(+-0) = +-0``; NaN, infinities and ``|x| > 2**29`` give NaN (both
    ``sin`` and ``cos`` are NaN, the sentinel test is false for NaN, and
    ``NaN / NaN`` is NaN).  ``cos`` is never zero here, so the division cannot
    raise.
    """
    s = sin(x)  # our own sin — no math.sin
    c = cos(x)  # our own cos — no math.cos

    # False when c is NaN, so a NaN falls through to NaN / NaN = NaN.
    if abs(c) < 1e-15:
        return 1.0e308 if s > 0 else -1.0e308

    return s / c


# ---------------------------------------------------------------------------
# Arctangent — Inverse of Tangent
# ---------------------------------------------------------------------------


def _atan_core(x: float) -> float:
    """
    Core atan computation for |x| <= 1, using Taylor series + half-angle reduction.

    This is a private helper. Callers should use atan() instead.

    Half-Angle Reduction
    ~~~~~~~~~~~~~~~~~~~~

    The Taylor series for atan:

        atan(x) = x - x^3/3 + x^5/5 - x^7/7 + ...   (for |x| <= 1)

    converges slowly near x = 1 (requires ~50 terms for full precision).

    Fix: the half-angle identity for atan:

        atan(x) = 2 * atan( x / (1 + sqrt(1 + x^2)) )

    This halves the argument: if |x| <= 1, then after reduction we get
    |y| <= tan(pi/8) ~= 0.414, where the series converges in ~15 terms.

    Taylor Series Iteration
    ~~~~~~~~~~~~~~~~~~~~~~~

    atan(t) = t - t^3/3 + t^5/5 - t^7/7 + ...

    Iterative ratio between consecutive terms:

        term_n = term_{n-1} * (-t^2) * (2n-1) / (2n+1)

    The operations run strictly left to right, ``((term * -t^2) * (2n-1)) /
    (2n+1)``, one rounding each (PHY00a 3.9).  We multiply the final result by
    2 to undo the half-angle halving.
    """
    # Half-angle reduction: shrink |x| to |y| <= tan(pi/8) ~= 0.414.
    # We use our own sqrt here — no math.sqrt.
    reduced: float = x / (1.0 + sqrt(1.0 + x * x))

    # Taylor series on the reduced argument.
    t: float = reduced
    t_sq: float = t * t
    term: float = t
    result: float = t

    for n in range(1, 31):
        term = term * (-t_sq)
        term = term * (2 * n - 1)
        term = term / (2 * n + 1)
        result += term

        # Early exit when terms are negligibly small.
        if abs(term) < 1e-17:
            break

    # Undo the half-angle: atan(x) = 2 * atan(reduced).
    return 2.0 * result


def atan(x: float) -> float:
    """
    Compute the arctangent of *x*, returning the angle in radians.

    Return range: (-pi/2, pi/2).

    Range Reduction
    ~~~~~~~~~~~~~~~

    The Taylor series for atan converges only for |x| <= 1. For |x| > 1
    we use the complementary identity:

        atan(x)  = pi/2 - atan(1/x)    for x > 1
        atan(x)  = -pi/2 - atan(1/x)   for x < -1

    Proof: atan(x) + atan(1/x) = pi/2 for x > 0. If theta = atan(x),
    then tan(pi/2 - theta) = cot(theta) = 1/x, so atan(1/x) = pi/2 - theta.
    """
    # The public contract is binary64 even when a Python caller supplies an int.
    x = _to_float(x)

    # atan(x) rounds exactly to x here; avoid halving subnormals and retain -0.
    if abs(x) <= 7.450580596923828e-9:
        return x

    if x > 1.0:
        return HALF_PI - _atan_core(1.0 / x)
    if x < -1.0:
        return -HALF_PI - _atan_core(1.0 / x)

    return _atan_core(x)


def atan2(y: float, x: float) -> float:
    """
    Compute the four-quadrant arctangent of *y* and *x*.

    Return range: (-pi, pi].

    Why Is atan2 Different from atan?
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    atan(y/x) only gives angles in (-pi/2, pi/2) — the right half of the
    plane. It cannot distinguish between:

        (y=1, x=1)  →  angle +pi/4   (first quadrant)
        (y=-1, x=-1) →  y/x = 1, so atan gives pi/4 — WRONG, should be -3pi/4

    atan2 inspects the signs of both y and x separately to determine the
    correct quadrant.

    Quadrant Diagram:
    ~~~~~~~~~~~~~~~~~

            y > 0
        Q2  |  Q1        atan2 > 0 in Q1 and Q2
      ------+------  x   atan2 < 0 in Q3 and Q4
        Q3  |  Q4        atan2 = ±pi on the negative x-axis

    Special Cases:
    ~~~~~~~~~~~~~~
      (y=0, x>0)   → 0       (positive x-axis)
      (y>0, x=0)   → pi/2    (positive y-axis)
      (y=0, x<0)   → pi      (negative x-axis, by atan(0/x<0) + pi = pi)
      (y<0, x=0)   → -pi/2   (negative y-axis)
      (y=0, x=0)   → 0       (undefined by convention)
      (NaN, any) or (any, NaN) → NaN   (added by PHY00a: a finite answer to an
                                        undefined question is a wrong answer)

    These are *pinned conventions*, not IEEE 754's: ``atan2(-0, x < 0)`` is
    ``+pi`` and ``atan2(+-inf, +-inf)`` is NaN (because ``y/x`` is NaN).
    """
    y = _to_float(y)
    x = _to_float(x)
    # A NaN fails every comparison below and would fall through to 0.0.
    if x != x or y != y:  # noqa: PLR0124
        return _NAN
    if x > 0.0:
        return atan(y / x)
    if x < 0.0 and y >= 0.0:
        return atan(y / x) + PI
    if x < 0.0 and y < 0.0:
        return atan(y / x) - PI
    if x == 0.0 and y > 0.0:
        return HALF_PI
    if x == 0.0 and y < 0.0:
        return -HALF_PI
    # x == 0.0 and y == 0.0: undefined, return 0 by convention.
    return 0.0
