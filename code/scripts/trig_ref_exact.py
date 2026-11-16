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

# Deliberately no import from trig_ref_softfloat: the exact reference must not share
# a single line of code with the model whose answers it checks (PHY00a 6.4).  The
# binary64 encoding and decoding it needs are written out again below, by a
# different method (exact rationals, not the model's guard/round/sticky integers);
# tests/test_trig_reference_fixture.py asserts that the two agree.
SIGN = 1 << 63
EXP_MASK = 0x7FF << 52
FRAC_BITS = 52
BIAS = 1023

_PI_CACHE: dict[int, int] = {}


def is_finite(bits: int) -> bool:
    """True unless the exponent field is all ones (infinity or NaN)."""
    return (bits & EXP_MASK) != EXP_MASK


def unpack(bits: int) -> Fraction:
    """The exact value of a finite binary64 bit pattern, as a Fraction.

    ``(-1)**sign * (1 + frac/2**52) * 2**(exp-1023)`` for a normal number and
    ``(-1)**sign * frac * 2**-1074`` for a subnormal one; negative zero is 0.
    """
    assert is_finite(bits)
    exp_field = (bits >> FRAC_BITS) & 0x7FF
    frac = bits & ((1 << FRAC_BITS) - 1)
    if exp_field == 0:
        magnitude = Fraction(frac, 1 << 1074)
    else:
        significand = (1 << FRAC_BITS) | frac
        shift = exp_field - BIAS - FRAC_BITS
        magnitude = Fraction(significand) * (Fraction(2) ** shift)
    return -magnitude if bits & SIGN else magnitude


def pack(v: Fraction) -> int:
    """The binary64 bit pattern of the double nearest ``v`` (ties to even).

    Method: find ``e`` with ``2**e <= |v| < 2**(e+1)``; the spacing of doubles there
    is ``q = 2**(e-52)`` (or ``2**-1074`` below the normal range); ``|v| / q`` is a
    rational whose floor is a candidate integer significand; the remainder is compared
    *exactly* with one half.  Carrying a rounded-up significand of ``2**53`` into the
    exponent field, and a subnormal one of ``2**52`` into the first normal number, is
    done by simple addition, because the fields are adjacent.
    """
    if v == 0:
        return 0
    sign = SIGN if v < 0 else 0
    a = abs(v)
    e = a.numerator.bit_length() - a.denominator.bit_length()
    if Fraction(2) ** e > a:
        e -= 1
    # now 2**e <= a < 2**(e+1)
    q_exp = max(e - FRAC_BITS, -1074)
    n = a / (Fraction(2) ** q_exp)
    f, rem = divmod(n.numerator, n.denominator)
    twice = 2 * rem
    if twice > n.denominator or (twice == n.denominator and f & 1):
        f += 1
    if q_exp == -1074 and f < (1 << FRAC_BITS):
        bits = f  # subnormal (or zero): exponent field 0
    else:
        # f is in [2**52, 2**53]; 2**53 carries into the exponent field.
        bits = ((q_exp + FRAC_BITS + BIAS) << FRAC_BITS) + (f - (1 << FRAC_BITS))
    bits = min(bits, EXP_MASK)  # anything at or past the top is infinity
    return sign | bits


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
    return unpack(bits)


def round_fraction(v: Fraction) -> int:
    """Correctly rounded binary64 bits of an exact rational (ties to even)."""
    return pack(v)


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


def cr_tan(xbits: int) -> int:
    """Correctly rounded tan bits for an in-domain finite x (|x| <= 2**29).

    tan = sin / cos of the exact input.  Each of sin and cos is known only to within
    its error bound, so the quotient is bracketed by the four extreme ratios, which is
    a valid enclosure because the cosine interval excludes zero (the cosine of a
    nonzero double is never within 2**-100 of zero).  If the bracket straddles a
    rounding boundary the precision is doubled, as in ``cr_sincos``.  The reference
    ``tan`` returns a sentinel instead near the poles; the caller omits ``cr`` there.
    """
    x = to_fraction(xbits)
    if x == 0:
        return xbits  # tan(+-0) = +-0
    if abs(x) < Fraction(1, 1 << 27):
        # tan x = x + x**3/3 + ... lies strictly between x and x + x**3/2 for 0 < x < 1.
        a, b = x, x + x**3 / 2
        lo, hi = (a, b) if x > 0 else (b, a)
        got = round_interval(lo, hi)
        if got is None:
            raise RuntimeError("tiny-argument tan interval straddles a boundary")
        return got
    prec = 240
    while True:
        (sv, se), (cv, ce) = sincos_exact(xbits, prec)
        c_lo, c_hi = cv - ce, cv + ce
        if c_lo < 0 < c_hi or c_lo == 0 or c_hi == 0:
            raise RuntimeError(f"cosine interval contains zero for {xbits:#x}")
        quotients = [sn / cn for sn in (sv - se, sv + se) for cn in (c_lo, c_hi)]
        got = round_interval(min(quotients), max(quotients))
        if got is not None:
            return got
        prec *= 2
        if prec > 4000:
            raise RuntimeError(f"cr_tan did not converge for {xbits:#x}")


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
