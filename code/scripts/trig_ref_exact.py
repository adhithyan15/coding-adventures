"""Exact (rational) reference for the trig conformance vectors (PHY00a 6.4 step 3).

Nothing here calls a host ``sin``, ``cos`` or ``float``.  Doubles come in as
bit patterns and become exact ``Fraction`` values; ``pi`` comes from Machin's
formula over big integers; sine and cosine are Taylor series summed in fixed
point with an explicit error bound; and the answer is rounded to binary64 by an
*exact* comparison against the rounding boundaries.  If the interval of
uncertainty straddles a boundary (the table-maker's dilemma) the precision is
doubled and the row is recomputed, so a returned value is always the correctly
rounded one.

The result is the *correctly rounded value of the mathematical function at the
exact binary64 input* ("cr" in the fixture), which is what an accuracy claim is
measured against.
"""

from __future__ import annotations

from fractions import Fraction

from trig_ref_softfloat import SIGN, _round, decode, is_finite

_PI_CACHE: dict[int, int] = {}


def _atan_inv(n: int, bits: int) -> int:
    """atan(1/n) * 2**bits (floor-ish; error a few units)."""
    one = 1 << bits
    power = one // n
    n2 = n * n
    total = power
    k = 1
    while power:
        power //= n2
        term = power // (2 * k + 1)
        total += -term if k & 1 else term
        k += 1
    return total


def pi_fixed(bits: int) -> int:
    """pi * 2**bits via Machin, 16 atan(1/5) - 4 atan(1/239); error under 2**-(bits-8)."""
    if bits not in _PI_CACHE:
        g = bits + 32
        v = 16 * _atan_inv(5, g) - 4 * _atan_inv(239, g)
        _PI_CACHE[bits] = v >> 32
    return _PI_CACHE[bits]


PI_BITS = 1200


def pi_fraction() -> Fraction:
    return Fraction(pi_fixed(PI_BITS), 1 << PI_BITS)


def to_fraction(bits: int) -> Fraction:
    """Exact value of a finite double."""
    assert is_finite(bits)
    s, m, e = decode(bits)
    f = Fraction(m) * (Fraction(2) ** e)
    return -f if s else f


def round_fraction(v: Fraction) -> int:
    """Correctly rounded binary64 bits of an exact rational (ties to even)."""
    if v == 0:
        return 0
    sign = 1 if v < 0 else 0
    num, den = abs(v.numerator), v.denominator
    # Pick L, the exponent of the last kept bit, from the leading-bit exponent.
    e = num.bit_length() - den.bit_length()
    if (num << max(0, -e)) < (den << max(0, e)):
        e -= 1  # now 2**e <= v < 2**(e+1)
    low = max(e - 52, -1074)
    n = num << max(0, -low)
    d = den << max(0, low)
    q4, r4 = divmod(n << 3, d)
    m = (q4 << 1) | (1 if r4 else 0)  # value = m * 2**(low - 4)
    return _round(sign, m, low - 4)


def round_interval(lo: Fraction, hi: Fraction):
    """Bits if lo and hi round to the same double, else None."""
    a, b = round_fraction(lo), round_fraction(hi)
    return a if a == b else None


def ulp_of(v: Fraction) -> Fraction:
    """Spacing of binary64 values in the binade of v (the 'ulp of the exact value')."""
    v = abs(v)
    num, den = v.numerator, v.denominator
    e = num.bit_length() - den.bit_length()
    if (num << max(0, -e)) < (den << max(0, e)):
        e -= 1
    return Fraction(2) ** max(e - 52, -1074)


def ordered(a: int) -> int:
    """Monotone integer of a non-NaN double (+0 and -0 both 0): for ulp_dist."""
    mag = a & ~SIGN
    return -mag if a & SIGN else mag


def ulp_dist(a: int, b: int) -> int:
    """VER04p 5.4: the number of representable steps between two doubles."""
    return abs(ordered(a) - ordered(b))


# ---------------------------------------------------------------------------
# sin / cos of an exact double
# ---------------------------------------------------------------------------


def _sincos_fixed(r: Fraction, prec: int):
    """sin r and cos r as fixed-point integers at ``prec`` bits, with an error bound."""
    one = 1 << prec
    rr = (r.numerator << prec) // r.denominator
    z = (rr * rr) >> prec
    s, c = rr, one
    ts, tc = rr, one
    n = 1
    while ts or tc:
        tc = -((tc * z) >> prec) // ((2 * n - 1) * (2 * n))
        ts = -((ts * z) >> prec) // ((2 * n) * (2 * n + 1))
        c += tc
        s += ts
        n += 1
    err = 16 * n + 16  # units of 2**-prec: a loose bound on accumulated floors
    return s, c, err


def sincos_exact(xbits: int, prec: int = 240):
    """(sin, cos) of an exact finite double as Fractions, each with an error bound.

    Returns ((sin_value, sin_err), (cos_value, cos_err)) where the true value
    lies within value +- err.  ``|x|`` must be at most 2**29 (the domain).
    """
    x = to_fraction(xbits)
    pi = Fraction(pi_fixed(PI_BITS), 1 << PI_BITS)
    half_pi = pi / 2
    k = (x / half_pi + Fraction(1, 2)).__floor__()
    r = x - k * half_pi
    s, c, err = _sincos_fixed(r, prec)
    unit = Fraction(1, 1 << prec)
    # k * (pi error) stays below 2**-1100 for |k| < 2**30: negligible next to err.
    q = k % 4
    sv = (s, c, -s, -c)[q]
    cv = (c, -s, -c, s)[q]
    return (Fraction(sv) * unit, err * unit), (Fraction(cv) * unit, err * unit)


def cr_sincos(xbits: int):
    """Correctly rounded (sin, cos) bits and exact values for an in-domain finite x.

    Returns (sin_bits, sin_exact, cos_bits, cos_exact); the exact values are
    rationals within 2**-200 of the true ones, enough for ulp measurements.
    """
    prec = 240
    while True:
        (sv, se), (cv, ce) = sincos_exact(xbits, prec)
        sb = round_interval(sv - se, sv + se)
        cb = round_interval(cv - ce, cv + ce)
        if sb is not None and cb is not None:
            return sb, sv, cb, cv
        prec *= 2
        if prec > 4000:
            raise RuntimeError(f"cr_sincos did not converge for {xbits:#x}")


def cr_tiny(op: str, xbits: int) -> int:
    """sin/cos below TINY: exact interval argument that the result is x (sin) or 1 (cos)."""
    x = to_fraction(xbits)
    if op == "sin":
        lo = x - x**3 / 6 if x > 0 else x
        hi = x if x > 0 else x - x**3 / 6
        got = round_interval(lo, hi)
    else:
        got = round_interval(1 - x * x / 2, Fraction(1))
    if got is None:
        raise RuntimeError("tiny-argument interval straddles a boundary")
    # round_fraction(0) is +0; a sine of -0 keeps its sign.
    if op == "sin" and x == 0:
        return xbits
    return got


def cr_conversion(op: str, xbits: int):
    """Correctly rounded radians/degrees bits and the exact value (None if zero)."""
    x = to_fraction(xbits)
    if x == 0:
        return xbits, None
    pi = pi_fraction()
    v = x * pi / 180 if op == "radians" else x * 180 / pi
    eps = Fraction(1, 1 << 1000)
    got = round_interval(v * (1 - eps), v * (1 + eps))
    if got is None:
        raise RuntimeError("conversion straddles a boundary")
    return got, v
