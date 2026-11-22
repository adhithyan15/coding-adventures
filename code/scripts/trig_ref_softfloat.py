"""Integer-only binary64 soft-float and the TRIG-REF-1 model (PHY00a 3.1-3.9, 6.4).

This module is the *oracle* for the conformance vectors of
``code/specs/fixtures/trig-reference-v1``.  It never touches a host ``float``:
every binary64 value is a Python ``int`` holding the IEEE bit pattern, and
every operation (add, subtract, multiply, divide, compare) is defined by exact
integer arithmetic followed by exactly one round-to-nearest-even step.  That is
the arithmetic model of PHY00a 3.1, written so that nothing about the host
CPU, the host libm or the host compiler can leak into a vector.

How a double is held
--------------------

A finite binary64 is ``(-1)**sign * m * 2**e`` with integer ``m``.  ``decode``
returns ``(sign, m, e)``; ``_round`` is the inverse and is the single place
where rounding happens::

        value  = m * 2**e              (m, e exact integers, any size)
        round  : keep 53 significant bits (or fewer, for subnormals),
                 ties to even, overflow to infinity.

``fma`` (exact multiply-add, one rounding) exists only for the *fusion
sensitivity* gate: it lets the generator prove that a vector would notice a
compiler that fused ``a*b + c``.  The reference model never fuses.
"""

from __future__ import annotations

SIGN = 1 << 63
EXP_MASK = 0x7FF0000000000000
FRAC_MASK = (1 << 52) - 1
QNAN = 0x7FF8000000000000
INF = 0x7FF0000000000000


class NonFinite(Exception):
    """Raised by ``to_int`` on NaN/inf: the model's stand-in for ``int(nan)``."""


def is_nan(b: int) -> bool:
    return (b & EXP_MASK) == EXP_MASK and (b & FRAC_MASK) != 0


def is_inf(b: int) -> bool:
    return (b & ~SIGN) == INF


def is_zero(b: int) -> bool:
    return (b & ~SIGN) == 0


def is_finite(b: int) -> bool:
    return (b & EXP_MASK) != EXP_MASK


def decode(b: int):
    """Finite bits -> (sign, m, e) with value (-1)**sign * m * 2**e (m may be 0)."""
    sign = b >> 63
    ex = (b >> 52) & 0x7FF
    fr = b & FRAC_MASK
    if ex == 0:
        return sign, fr, -1074
    return sign, fr | (1 << 52), ex - 1075


def _round(sign: int, m: int, e: int) -> int:
    """Round (-1)**sign * m * 2**e (m > 0 exact) to binary64 bits, ties to even."""
    if m == 0:
        return sign << 63
    nbits = m.bit_length()
    lead = e + nbits - 1  # exponent of the leading bit
    if lead >= -1022:
        shift = nbits - 53
        sub = False
    else:
        shift = -1074 - e
        sub = True
    if shift <= 0:
        q = m << -shift
        low = e + shift  # exponent of the least significant kept bit
    else:
        q = m >> shift
        rem = m & ((1 << shift) - 1)
        half = 1 << (shift - 1)
        if rem > half or (rem == half and (q & 1)):
            q += 1
        low = e + shift
    if sub:
        # low == -1074; q may have become 2**52, which is the smallest normal.
        return (sign << 63) | q
    if q == 1 << 53:
        q >>= 1
        low += 1
    biased = low + 52 + 1023
    if biased >= 2047:
        return (sign << 63) | INF
    return (sign << 63) | (biased << 52) | (q & FRAC_MASK)


def round_exact(sign: int, m: int, e: int) -> int:
    return _round(sign, m, e)


def neg(a: int) -> int:
    return a ^ SIGN


def fabs(a: int) -> int:
    return a & ~SIGN


def add(a: int, b: int) -> int:
    if is_nan(a) or is_nan(b):
        return QNAN
    if is_inf(a) or is_inf(b):
        if is_inf(a) and is_inf(b):
            return a if (a >> 63) == (b >> 63) else QNAN
        return a if is_inf(a) else b
    sa, ma, ea = decode(a)
    sb, mb, eb = decode(b)
    if ma == 0 and mb == 0:
        return (sa & sb) << 63
    e = min(ea, eb)
    va = (-ma if sa else ma) << (ea - e)
    vb = (-mb if sb else mb) << (eb - e)
    s = va + vb
    if s == 0:
        return 0
    return _round(1 if s < 0 else 0, abs(s), e)


def sub(a: int, b: int) -> int:
    return add(a, neg(b))


def mul(a: int, b: int) -> int:
    if is_nan(a) or is_nan(b):
        return QNAN
    sg = (a >> 63) ^ (b >> 63)
    if is_inf(a) or is_inf(b):
        if is_zero(a) or is_zero(b):
            return QNAN
        return (sg << 63) | INF
    _, ma, ea = decode(a)
    _, mb, eb = decode(b)
    if ma == 0 or mb == 0:
        return sg << 63
    return _round(sg, ma * mb, ea + eb)


def div(a: int, b: int) -> int:
    if is_nan(a) or is_nan(b):
        return QNAN
    sg = (a >> 63) ^ (b >> 63)
    if is_inf(a):
        return QNAN if is_inf(b) else (sg << 63) | INF
    if is_inf(b):
        return sg << 63
    _, ma, ea = decode(a)
    _, mb, eb = decode(b)
    if mb == 0:
        return QNAN if ma == 0 else (sg << 63) | INF
    if ma == 0:
        return sg << 63
    s = max(0, 64 + mb.bit_length() - ma.bit_length())
    q, r = divmod(ma << s, mb)
    return _round(sg, (q << 1) | (1 if r else 0), ea - eb - s - 1)


def fma(a: int, b: int, c: int) -> int:
    """Exact a*b + c rounded once (only for the fusion-sensitivity gate)."""
    if not (is_finite(a) and is_finite(b) and is_finite(c)):
        return add(mul(a, b), c)
    sa, ma, ea = decode(a)
    sb, mb, eb = decode(b)
    sc, mc, ec = decode(c)
    p = ma * mb
    pe = ea + eb
    ps = sa ^ sb
    if p == 0 and mc == 0:
        return add(mul(a, b), c)
    e = min(pe, ec)
    vp = (-p if ps else p) << (pe - e)
    vc = (-mc if sc else mc) << (ec - e)
    s = vp + vc
    if s == 0:
        return 0
    return _round(1 if s < 0 else 0, abs(s), e)


def key(a: int) -> int:
    """Monotone integer key of a non-NaN double (+0 and -0 share key 0)."""
    mag = a & ~SIGN
    return -mag if a >> 63 else mag


def lt(a: int, b: int) -> bool:
    return not (is_nan(a) or is_nan(b)) and key(a) < key(b)


def gt(a: int, b: int) -> bool:
    return lt(b, a)


def le(a: int, b: int) -> bool:
    return not (is_nan(a) or is_nan(b)) and key(a) <= key(b)


def ge(a: int, b: int) -> bool:
    return le(b, a)


def eq(a: int, b: int) -> bool:
    return not (is_nan(a) or is_nan(b)) and key(a) == key(b)


def from_int(n: int) -> int:
    """Exact small integer -> double bits."""
    if n == 0:
        return 0
    return _round(1 if n < 0 else 0, abs(n), 0)


def to_int(a: int) -> int:
    """Integer-valued finite double -> Python int (the model's ``int(k)``)."""
    if not is_finite(a):
        raise NonFinite(hex(a))
    s, m, e = decode(a)
    v = m << e if e >= 0 else m >> -e
    return -v if s else v


# ---------------------------------------------------------------------------
# The constants of PHY00a 3.2 (bit patterns are authoritative).
# ---------------------------------------------------------------------------

CONSTANTS = {
    "INV_PIO2": 0x3FE45F306DC9C883,
    "PIO2_1": 0x3FF921FB40000000,
    "PIO2_2": 0x3E74442D00000000,
    "PIO2_3": 0x3CF8469880000000,
    "PIO2_4": 0x3B78CC5160000000,
    "PIO2_5": 0x39F01B839A25204A,
    "MAGIC": 0x4338000000000000,
    "LIMIT": 0x41C0000000000000,
    "TINY": 0x3E40000000000000,
    "S1": 0xBFC5555555555555,
    "S2": 0x3F81111111111111,
    "S3": 0xBF2A01A01A01A01A,
    "S4": 0x3EC71DE3A556C734,
    "S5": 0xBE5AE64567F544E4,
    "S6": 0x3DE6124613A86D09,
    "S7": 0xBD6AE7F3E733B81F,
    "S8": 0x3CE952C77030AD4A,
    "S9": 0xBC62F49B46814157,
    "C2": 0x3FA5555555555555,
    "C3": 0xBF56C16C16C16C17,
    "C4": 0x3EFA01A01A01A01A,
    "C5": 0xBE927E4FB7789F5C,
    "C6": 0x3E21EED8EFF8D898,
    "C7": 0xBDA93974A8C07C9D,
    "C8": 0x3D2AE7F3E733B81F,
    "C9": 0xBCA6827863B97D97,
    "RAD_PER_DEG": 0x3F91DF46A2529D39,
    "DEG_PER_RAD": 0x404CA5DC1A63C1F8,
    "PI": 0x400921FB54442D18,
    "TWO_PI": 0x401921FB54442D18,
    "HALF_PI": 0x3FF921FB54442D18,
    "TAN_POLE": 0x3CD203AF9EE75616,  # 1e-15
    "TAN_BIG": 0x7FE1CCF385EBC8A0,  # 1e308
    "ATAN_TERM": 0x3C670EF54646D497,  # 1e-17
    "SQRT_TINY": 0x01A56E1FC2F8F359,  # 1e-300
    "SQRT_REL": 0x3CD203AF9EE75616,  # 1e-15 (sqrt convergence, 3.9)
}

ONE = 0x3FF0000000000000
HALF = 0x3FE0000000000000
TWO = 0x4000000000000000
FOUR = 0x4010000000000000
QUARTER = 0x3FD0000000000000

# The 24 fusable sites of PHY00a section 5.
SITES = (
    ["red.t"]
    + [f"red.{i}" for i in range(1, 6)]
    + [f"sin.h{n}" for n in range(8, 0, -1)]
    + ["sin.fin"]
    + [f"cos.h{n}" for n in range(8, 1, -1)]
    + ["cos.half", "cos.fin"]
)
assert len(SITES) == 24


class Model:
    """TRIG-REF-1, transcribed over the integer soft-float above.

    ``consts`` overrides constants (mutation gate), ``fuse`` is a set of
    SITES computed as a fused multiply-add (fusion gate), ``guard`` selects the
    domain-guard form: ``"neg_le"`` (correct, 3.6), ``"gt"`` (the wrong
    ``|x| > LIMIT`` rewrite) or ``"none"`` (no guard).
    """

    def __init__(self, consts=None, fuse=(), guard="neg_le"):
        self.c = dict(CONSTANTS)
        if consts:
            self.c.update(consts)
        self.fuse = frozenset(fuse)
        self.guard = guard
        c = self.c
        self.S = [None] + [c[f"S{n}"] for n in range(1, 10)]
        self.C = [None, None] + [c[f"C{n}"] for n in range(2, 10)]
        self.P = [c[f"PIO2_{i}"] for i in range(1, 6)]

    # -- helpers that honour the fusion set -------------------------------
    def _mad(self, site, a, b, c):
        """a*b + c, as two roundings (reference) or one (fused site)."""
        if site in self.fuse:
            return fma(a, b, c)
        return add(mul(a, b), c)

    # -- 3.4 range reduction ----------------------------------------------
    def reduce(self, x):
        c = self.c
        if "red.t" in self.fuse:
            u = fma(x, c["INV_PIO2"], c["MAGIC"])
        else:
            t = mul(x, c["INV_PIO2"])
            u = add(t, c["MAGIC"])
        k = sub(u, c["MAGIC"])
        r = x
        for i, p in enumerate(self.P, start=1):
            if (f"red.{i}") in self.fuse:
                r = fma(neg(k), p, r)
            else:
                m = mul(k, p)
                r = sub(r, m)
        return k, r

    # -- 3.5 kernels -------------------------------------------------------
    def sin_k(self, r):
        z = mul(r, r)
        p = self.S[9]
        for n in range(8, 0, -1):
            p = self._mad(f"sin.h{n}", p, z, self.S[n])
        w = mul(r, z)
        if "sin.fin" in self.fuse:
            return fma(w, p, r)
        w = mul(w, p)
        return add(r, w)

    def cos_k(self, r):
        z = mul(r, r)
        q = self.C[9]
        for n in range(8, 1, -1):
            q = self._mad(f"cos.h{n}", q, z, self.C[n])
        if "cos.half" in self.fuse:
            u = fma(neg(HALF), z, ONE)
        else:
            u = sub(ONE, mul(HALF, z))
        v = mul(z, z)
        if "cos.fin" in self.fuse:
            return fma(v, q, u)
        v = mul(v, q)
        return add(u, v)

    # -- 3.6 sin / cos -----------------------------------------------------
    def _out_of_domain(self, x):
        limit = self.c["LIMIT"]
        if self.guard == "neg_le":
            return not le(fabs(x), limit)
        if self.guard == "gt":
            return gt(fabs(x), limit)
        return False

    def sin(self, x):
        if self._out_of_domain(x):
            return QNAN
        if lt(fabs(x), self.c["TINY"]):
            return x
        k, r = self.reduce(x)
        q = to_int(k) & 3
        if q == 0:
            return self.sin_k(r)
        if q == 1:
            return self.cos_k(r)
        if q == 2:
            return neg(self.sin_k(r))
        return neg(self.cos_k(r))

    def cos(self, x):
        if self._out_of_domain(x):
            return QNAN
        if lt(fabs(x), self.c["TINY"]):
            return ONE
        k, r = self.reduce(x)
        q = to_int(k) & 3
        if q == 0:
            return self.cos_k(r)
        if q == 1:
            return neg(self.sin_k(r))
        if q == 2:
            return neg(self.cos_k(r))
        return self.sin_k(r)

    # -- 3.7 / 3.8 ---------------------------------------------------------
    def radians(self, d):
        return mul(d, self.c["RAD_PER_DEG"])

    def degrees(self, r):
        return mul(r, self.c["DEG_PER_RAD"])

    def tan(self, x):
        s = self.sin(x)
        c = self.cos(x)
        if lt(fabs(c), self.c["TAN_POLE"]):
            return self.c["TAN_BIG"] if gt(s, 0) else neg(self.c["TAN_BIG"])
        return div(s, c)

    # -- 3.9 Part II (unchanged majority algorithm, pinned) ----------------
    def sqrt(self, x):
        """Returns bits, or the string ``"error"`` for a negative argument."""
        if lt(x, 0):
            return "error"
        if is_zero(x):
            return x
        if is_inf(x) and not (x >> 63):
            return x
        scaled, scale = x, ONE
        while lt(scaled, QUARTER):
            scaled = mul(scaled, FOUR)
            scale = mul(scale, HALF)
        while ge(scaled, FOUR):
            scaled = mul(scaled, QUARTER)
            scale = mul(scale, TWO)
        guess = scaled if ge(scaled, ONE) else ONE
        for _ in range(60):
            nxt = div(add(guess, div(scaled, guess)), TWO)
            bound = add(mul(self.c["SQRT_REL"], guess), self.c["SQRT_TINY"])
            if lt(fabs(sub(nxt, guess)), bound):
                return mul(nxt, scale)
            guess = nxt
        return mul(guess, scale)

    def atan_core(self, x):
        reduced = div(x, add(ONE, self.sqrt(add(ONE, mul(x, x)))))
        t = reduced
        tsq = mul(t, t)
        term = t
        result = t
        for n in range(1, 31):
            term = mul(term, neg(tsq))
            term = mul(term, from_int(2 * n - 1))
            term = div(term, from_int(2 * n + 1))
            result = add(result, term)
            if lt(fabs(term), self.c["ATAN_TERM"]):
                break
        return mul(TWO, result)

    def atan(self, x):
        if le(fabs(x), self.c["TINY"]):
            return x
        if gt(x, ONE):
            return sub(self.c["HALF_PI"], self.atan_core(div(ONE, x)))
        if lt(x, neg(ONE)):
            return sub(neg(self.c["HALF_PI"]), self.atan_core(div(ONE, x)))
        return self.atan_core(x)

    def atan2(self, y, x):
        if is_nan(x) or is_nan(y):
            return QNAN
        if gt(x, 0):
            return self.atan(div(y, x))
        if lt(x, 0) and ge(y, 0):
            return add(self.atan(div(y, x)), self.c["PI"])
        if lt(x, 0) and lt(y, 0):
            return sub(self.atan(div(y, x)), self.c["PI"])
        if is_zero(x) and gt(y, 0):
            return self.c["HALF_PI"]
        if is_zero(x) and lt(y, 0):
            return neg(self.c["HALF_PI"])
        return 0
