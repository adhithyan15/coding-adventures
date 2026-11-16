"""Generate and verify ``code/specs/fixtures/trig-reference-v1`` (PHY00a section 6).

The conformance vectors say, bit for bit, what every ``trig`` port must return
(TRIG-REF-1) and, beside each reference result, what the *correctly rounded*
mathematical value is.  They are generated **independently of every port**:

* inputs are integers (IEEE bit patterns) from fixed lists and a specified PRNG;
  no ``float()``, ``math.sin`` or ``random`` is ever called on a corpus value;
* the reference result ``y`` comes from an integer-only soft-float model of the
  algorithm (``trig_ref_softfloat.py``) and is cross-checked against two
  unrelated host implementations, C (``-ffp-contract=off``) and Rust
  (``trig_reference_xcheck/``): any disagreement stops the generator;
* the correctly rounded value ``cr`` comes from exact rational arithmetic
  (``trig_ref_exact.py``: pi by Machin to 1200 bits, a fixed-point Taylor series
  with an error bound, rounding by exact comparison) and is cross-checked
  against GNU ``bc -l`` at 100 decimal digits on every edge row and a pinned
  one-in-eight of each sweep.

Usage::

    python3 code/scripts/generate_trig_reference_fixture.py            # write + gates
    python3 code/scripts/generate_trig_reference_fixture.py --check    # byte-for-byte
    python3 code/scripts/generate_trig_reference_fixture.py --no-external   # skip C/Rust/bc

Row format (PHY00a 6.2): ``op<TAB>x<TAB>y<TAB>cr<TAB>flags``, ``\\n`` endings.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from fractions import Fraction
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

from trig_ref_exact import (
    cr_conversion,
    cr_sincos,
    cr_tiny,
    pi_fraction,
    round_fraction,
    to_fraction,
    ulp_dist,
    ulp_of,
)
from trig_ref_softfloat import (
    CONSTANTS,
    FRAC_MASK,
    INF,
    QNAN,
    SIGN,
    SITES,
    Model,
    NonFinite,
    div,
    fabs,
    is_finite,
    is_inf,
    is_nan,
    is_zero,
)

GENERATOR_VERSION = "1"
ALGORITHM = "TRIG-REF-1"
ROOT = HERE.parent.parent
FIXTURE = ROOT / "code" / "specs" / "fixtures" / "trig-reference-v1"

LIMIT = CONSTANTS["LIMIT"]
TINY = CONSTANTS["TINY"]
MAX_FINITE = 0x7FEFFFFFFFFFFFFF
SEED = int.from_bytes(b"TRIG-REF", "big")
TRIG_OPS = ("sin", "cos", "tan", "radians", "degrees")
ALL_FLAGS = (
    "nan-class",
    "nonfinite=unrepresentable",
    "nonfinite=rejects",
    "out-of-domain",
    "pole-sentinel",
    "beam-raises",
    "signed-zero",
)

# 3.2 constants whose last bit is flipped by the mutation gate.
MUTATED = [
    n
    for n in CONSTANTS
    if n
    not in (
        "PI",
        "TWO_PI",
        "HALF_PI",
        "TAN_POLE",
        "TAN_BIG",
        "ATAN_TERM",
        "SQRT_TINY",
        "SQRT_REL",
    )
]
# Fusing a product that is exact (24-bit chunk x 29-bit integer, or 0.5*z) changes
# nothing, so those sites cannot be "noticed"; the gate asserts they are inert.
EXACT_PRODUCT_SITES = ("red.1", "red.2", "red.3", "red.4", "cos.half")
# Sites and constants whose effect on a result is *masked by the final rounding*.
# Fusing the fifth reduction step or a high-order Horner step moves an intermediate
# by about 1e-31 of its size; flipping the last bit of S3..S9, C3..C9, PIO2_5 or TINY
# moves a result by under 1e-20 of an ulp.  No sampled vector can see that (the
# chance per input is below 1e-4 for S3 and below 1e-16 for the rest), so the gate
# of PHY00a 6.4 (b) and (c) cannot require it.  The generator reports these
# explicitly; everything else must be detected, and a port still pins the constants
# by asserting their bits (PHY00a 6.5 e).
MASKED_FUSION = (
    *EXACT_PRODUCT_SITES,
    "red.5",
    *[f"sin.h{n}" for n in range(8, 1, -1)],
    *[f"cos.h{n}" for n in range(8, 2, -1)],
    "cos.h2",
)
MASKED_MUTATION = (
    "PIO2_5",
    "TINY",
    *[f"S{n}" for n in range(3, 10)],
    *[f"C{n}" for n in range(3, 10)],
)


# ---------------------------------------------------------------------------
# The PRNG of 6.3: xoshiro256** seeded by SplitMix64 of the integer "TRIG-REF"
# ---------------------------------------------------------------------------

M64 = (1 << 64) - 1


def _rotl(x: int, k: int) -> int:
    return ((x << k) | (x >> (64 - k))) & M64


class Xoshiro256ss:
    def __init__(self, seed: int):
        state = seed & M64
        s = []
        for _ in range(4):
            state = (state + 0x9E3779B97F4A7C15) & M64
            z = state
            z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M64
            z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M64
            s.append(z ^ (z >> 31))
        self.s = s

    def next(self) -> int:
        s = self.s
        result = (_rotl((s[1] * 5) & M64, 7) * 9) & M64
        t = (s[1] << 17) & M64
        s[2] ^= s[0]
        s[3] ^= s[1]
        s[1] ^= s[2]
        s[0] ^= s[3]
        s[2] ^= t
        s[3] = _rotl(s[3], 45)
        return result


# ---------------------------------------------------------------------------
# Input construction (all integers: bit patterns)
# ---------------------------------------------------------------------------


def hx(b: int) -> str:
    return f"{b:016x}"


def fl(v: Fraction) -> int:
    """The double nearest an exact rational (no host float involved)."""
    return round_fraction(v)


def nbr(b: int, d: int) -> int:
    """Neighbour of a finite double ``d`` steps away in magnitude (sign kept)."""
    mag = (b & ~SIGN) + d
    if mag < 0 or mag > INF:
        raise ValueError("neighbour out of range")
    return (b & SIGN) | mag


def with_neighbours(b: int, span: int = 1) -> list[int]:
    out = []
    for d in range(-span, span + 1):
        try:
            out.append(nbr(b, d))
        except ValueError:
            pass
    return out


def both_signs(bs):
    out = []
    for b in bs:
        out.append(b)
        out.append(b ^ SIGN)
    return out


def dedupe(seq):
    seen, out = set(), []
    for b in seq:
        if b not in seen:
            seen.add(b)
            out.append(b)
    return out


def _pow2(k: int) -> int:
    return fl(Fraction(2) ** k)


def f64_edge() -> list[int]:
    """VER04p 11.1's edge list, in order, bit patterns, duplicates removed."""
    pi = pi_fraction()
    out: list[int] = [0, SIGN]  # 1. the zeros
    min_sub, max_sub, min_norm = 1, 0x000FFFFFFFFFFFFF, 0x0010000000000000
    for b in (min_sub, max_sub, min_norm):  # 2. subnormal/normal boundary
        out += with_neighbours(b, 1)
    one = 0x3FF0000000000000
    out += with_neighbours(one, 1)  # 3. one and its two neighbours
    out += [one | SIGN, _pow2(1), _pow2(-1), fl(Fraction(3, 2))]
    out += [MAX_FINITE, MAX_FINITE - 1]  # 4. largest finite and predecessor
    out += [INF]  # 5. infinities
    out += [  # 6. NaN shapes: canonical, negative, signalling, payload 1, max, mid-bit
        0x7FF8000000000000,
        0xFFF8000000000000,
        0x7FF0000000000001,
        0x7FF8000000000001,
        0x7FFFFFFFFFFFFFFF,
        0x7FF8000002000000,
    ]
    # 7. integer boundaries (2**53 + 1 is not a double: its neighbours stand in)
    out += [fl(Fraction(2**53 - 1)), _pow2(53), nbr(_pow2(53), 1)]
    out += [fl(Fraction(2**31 - 1)), _pow2(31), fl(Fraction(2**31 + 1))]
    for k in (63, 64):
        out += with_neighbours(_pow2(k), 1)
    # 8. halfway cases: a midpoint of adjacent doubles needs 54 bits, so it is not an
    # input; the doubles one ulp either side of 1 and of 2**k stand in for it.
    for k in (
        -1074,
        -1022,
        -1000,
        -500,
        -100,
        -52,
        -27,
        -1,
        1,
        10,
        27,
        29,
        30,
        52,
        100,
        500,
        1000,
        1023,
    ):
        out += with_neighbours(_pow2(k), 1)
    # 10. pi/2 and pi rounded, and their neighbours
    out += with_neighbours(fl(pi / 2), 2) + with_neighbours(fl(pi), 2)
    # 9. the sign of every one of the above
    out += [b ^ SIGN for b in out]
    return dedupe(out)


def _e_fraction() -> Fraction:
    total, term = Fraction(0), Fraction(1)
    for n in range(1, 60):
        total += term
        term /= n
    return total


def reduction_tie_cases() -> list[int]:
    """Inputs next to an odd multiple of pi/4 where fusing ``x*INV_PIO2 + MAGIC`` changes a result.

    ``t = x * INV_PIO2`` is rounded before MAGIC is added; a fused multiply-add rounds the
    exact product instead, and near a tie (a half-integer ``t``) the two can choose
    different quotients ``k``.  The first 32 hits for small ``n`` and the first 32 for
    ``n`` spread to 1.7e8 are kept, so the fixture can see that one fusable site (5).
    """
    pi4 = pi_fraction() / 4
    plain, fused = Model(), Model(fuse={"red.t"})
    small = list(range(6000))
    spread = [(i * i * 7919) % 170_000_000 for i in range(1, 2000)]
    out: list[int] = []
    for ns in (small, spread):
        found = 0
        for n in ns:
            x0 = fl(Fraction(2 * n + 1) * pi4)
            for d in range(-3, 4):
                x = nbr(x0, d)
                if found < 32 and not (
                    same(plain.sin(x), fused.sin(x))
                    and same(plain.cos(x), fused.cos(x))
                ):
                    out += [x, x | SIGN]
                    found += 1
            if found >= 32:
                break
    return dedupe(out)


def trig_points() -> list[int]:
    """The trig-specific edge points of 6.3, in the order of the table."""
    pi = pi_fraction()
    out: list[int] = []
    # multiples of pi/2 and their neighbours
    for k in (1, 2, 3, 4, 5, 6, 7, 8, 16, 100, 1000, 10**5, 2**20, 2**28):
        out += both_signs(with_neighbours(fl(Fraction(k) * pi / 2), 2))
    # thresholds
    for b in (TINY, LIMIT):
        out += both_signs(with_neighbours(b, 2))
    for v in (2**30, 2**53 - 1, 2**53, 10**10, 10**20, 10**22, 10**300):
        out += both_signs([fl(Fraction(v))])
    out += both_signs(
        [nbr(_pow2(53), 1), _pow2(63), _pow2(64), MAX_FINITE, MAX_FINITE - 1]
    )
    # worst cases on record (4.1); any input later found worse is added here
    out += [0x41BB531AD2E28013, 0x41B08A633012E355]
    # quadrant edges
    for num in (1, 3, 5, 7):
        out += both_signs(with_neighbours(fl(Fraction(num) * pi / 4), 1))
    out += reduction_tie_cases()
    # common values
    common = [Fraction(1, 2), Fraction(1), Fraction(2), Fraction(3), Fraction(1, 10)]
    common += [_e_fraction(), Fraction(355, 113), Fraction(10**5), Fraction(10**6)]
    out += both_signs([fl(v) for v in common])
    # poles for tan
    out += with_neighbours(fl(pi / 2), 1) + [fl(3 * pi / 2), fl(-pi / 2)]
    # conversions
    for v in (180, 90, 45, 360):
        out += both_signs([fl(Fraction(v))])
    for v in (pi, pi / 2, pi / 4, 2 * pi):
        out += both_signs([fl(v)])
    out += both_signs([MAX_FINITE, 1])
    return dedupe(out)


def part2_rows() -> list[tuple[str, str]]:
    """(op, x-field) pairs for sqrt, atan, atan2 (6.3 Part II set)."""
    inf, ninf = INF, INF | SIGN
    nan = 0x7FF8000000000000
    one, two = 0x3FF0000000000000, 0x4000000000000000
    rows: list[tuple[str, str]] = []
    sq = [
        0,
        SIGN,
        one,
        two,
        0x4010000000000000,
        0x3FD0000000000000,
        fl(Fraction(1, 10**300)),
        MAX_FINITE,
        inf,
        nan,
    ]
    sq += [
        1,
        0x000FFFFFFFFFFFFF,
        0x0010000000000000,
        fl(Fraction(3)),
        fl(Fraction(9)),
        fl(Fraction(10**300)),
        fl(Fraction(1, 2)),
    ]
    sq += [one | SIGN, two | SIGN, 1 | SIGN, ninf, MAX_FINITE | SIGN]
    rows += [("sqrt", hx(b)) for b in dedupe(sq)]
    at = both_signs(with_neighbours(TINY, 2)) + both_signs(
        [one, two, 1, fl(Fraction(10**300))]
    )
    at += both_signs(with_neighbours(one, 1)) + [0, SIGN, inf, ninf, nan]
    at += both_signs(
        [
            fl(Fraction(v))
            for v in (
                Fraction(1, 2),
                Fraction(1, 10),
                Fraction(3),
                Fraction(10),
                Fraction(10**5),
            )
        ]
    )
    at += [fl(to_fraction(0x3FDA827999FCEF32))]  # near tan(pi/8)
    rows += [("atan", hx(b)) for b in dedupe(at)]
    zeros = [0, SIGN]
    ys = zeros + [one, one | SIGN, two]
    xs = zeros + [one, one | SIGN, two, two | SIGN]
    pairs = [(y, x) for y in ys for x in xs]  # the quadrant and axis cases, both zeros
    pairs += [
        (one, nan),
        (nan, one),
        (nan, nan),
        (nan, one | SIGN),
        (one | SIGN, nan),
        (nan, 0),
        (0, nan),
    ]
    pairs += [(nan, one), (one, nan), (SIGN | nan, one)]
    pairs += [(inf, inf), (inf, ninf), (ninf, inf), (ninf, ninf)]
    pairs += [
        (one, inf),
        (one, ninf),
        (one | SIGN, ninf),
        (inf, one),
        (ninf, one),
        (inf, 0),
        (ninf, 0),
    ]
    pairs += [
        (fl(Fraction(10**300)), fl(Fraction(1, 10**300))),
        (MAX_FINITE, 1),
        (MAX_FINITE | SIGN, 1),
    ]
    pairs += [
        (0, one | SIGN),
        (SIGN, one | SIGN),
        (0, MAX_FINITE | SIGN),
        (SIGN, MAX_FINITE | SIGN),
    ]
    rows += [("atan2", f"{hx(y)},{hx(x)}") for y, x in dedupe(pairs)]
    return rows


def sweep_binade() -> list[int]:
    out = []
    for e in range(-30, 31):
        for j in range(32):
            v = Fraction(32 + j) * Fraction(2) ** (e - 5)
            b = fl(v)
            out += [b, b | SIGN]
    return out


def _convergents(a: Fraction):
    """Continued-fraction convergents (p, q) of a positive rational."""
    out = []
    p0, q0, p1, q1 = 1, 0, 0, 1
    n, d = a.numerator, a.denominator
    while d:
        t = n // d
        n, d = d, n - t * d
        p0, p1 = t * p0 + p1, p0
        q0, q1 = t * q0 + q1, q0
        out.append((p0, q0))
        if len(out) > 80:
            break
    return out


def cf_hard_cases() -> list[int]:
    """Doubles nearest to multiples of pi/2 (the hardest cases for the reduction, 4.2)."""
    half_pi = pi_fraction() / 2
    out = []
    for e in range(-3, 29):
        alpha = half_pi * Fraction(2) ** (52 - e)
        for p, q in _convergents(alpha):
            for j in (1, 2, 3, 4):
                m = j * p
                if (1 << 52) <= m < (1 << 53):
                    bits = fl(Fraction(m) * Fraction(2) ** (e - 52))
                    out.append(bits)
    return dedupe(out)


def sweep_cw() -> list[int]:
    pi = pi_fraction()
    half_pi = pi / 2
    out = []
    ks = list(range(1, 257)) + [
        257 + ((340_000_000 - 257) * i * i) // (63 * 63) for i in range(64)
    ]
    for k in ks:
        base = fl(Fraction(k) * half_pi)
        for d in (-2, -1, 0, 1, 2):
            b = nbr(base, d)
            out += [b, b | SIGN]
    for b in cf_hard_cases():
        out += [b, b | SIGN]
    rng = Xoshiro256ss(SEED ^ 1)
    anchors = (Fraction(1, 2), Fraction(1, 4), Fraction(1, 8))
    for i in range(512):
        k = (1 << 26) + rng.next() % (340_000_000 - (1 << 26) + 1)
        u = rng.next() >> 11
        sgn = rng.next() >> 63
        c = anchors[i % 3]
        r = c + c * Fraction(u, 1 << 53) / 16  # "just above" the anchor
        b = fl(Fraction(k) * half_pi + r)
        out.append(b | (sgn << 63))
    return out


def sweep_prng() -> list[int]:
    rng = Xoshiro256ss(SEED)
    out = [rng.next() for _ in range(1024)]
    for _ in range(1024):
        u = rng.next()
        ex = -27 + (u % 56)
        frac = rng.next() & FRAC_MASK
        out.append(((u >> 63) << 63) | ((1023 + ex) << 52) | frac)
    for _ in range(2048):
        n = rng.next() >> 11
        out.append(fl(Fraction(n - (1 << 52), 1 << 23)))
    return out


# ---------------------------------------------------------------------------
# Row computation
# ---------------------------------------------------------------------------

REF = Model()


def same(a, b) -> bool:
    """VER04p 4.3 ``same``: bit-equal, except every NaN equals every NaN."""
    if isinstance(a, str) or isinstance(b, str):
        return a == b
    if is_nan(a) and is_nan(b):
        return True
    return a == b


def model_eval(model: Model, op: str, arg: str):
    if op == "atan2":
        ys, xs = arg.split(",")
        return model.atan2(int(ys, 16), int(xs, 16))
    return getattr(model, op)(int(arg, 16))


class Row:
    __slots__ = ("cr", "exact", "flags", "op", "x", "y")

    def line(self) -> str:
        return "\t".join((self.op, self.x, self.y, self.cr, ",".join(self.flags)))


_CR_CACHE: dict = {}


def _sincos_cache(xb: int):
    if xb not in _CR_CACHE:
        _CR_CACHE[xb] = cr_sincos(xb)
    return _CR_CACHE[xb]


def correct_rounded(op: str, xb: int):
    """(cr bits or None, exact Fraction or None) for a single-argument row."""
    if op in ("sin", "cos"):
        if not is_finite(xb) or not (fabs(xb) <= LIMIT):
            return None, None
        if fabs(xb) < TINY:
            return cr_tiny(op, xb), None
        sb, sv, cb, cv = _sincos_cache(xb)
        return (sb, sv) if op == "sin" else (cb, cv)
    if op in ("radians", "degrees"):
        if not is_finite(xb):
            return None, None
        got, exact = cr_conversion(op, xb)
        return (None, None) if is_inf(got) else (got, exact)
    return None, None


def make_row(op: str, arg: str) -> Row:
    row = Row()
    row.op, row.x = op, arg
    res = model_eval(REF, op, arg)
    flags: set[str] = set()
    ins = [int(p, 16) for p in arg.split(",")]
    if any(not is_finite(b) for b in ins):
        flags.add("nonfinite=unrepresentable")
    if res == "error":
        row.y, row.cr, row.exact = "error", "-", None
        row.flags = sorted(flags, key=ALL_FLAGS.index)
        return row
    if is_nan(res):
        res = QNAN
        flags.add("nan-class")
    row.y = hx(res)
    if is_zero(res):
        flags.add("signed-zero")
    unrep = "nonfinite=unrepresentable" in flags
    if not unrep and not is_finite(res):
        flags.add("nonfinite=rejects")
    if (
        op in ("sin", "cos", "tan")
        and is_finite(ins[0])
        and not (fabs(ins[0]) <= LIMIT)
    ):
        flags |= {"out-of-domain", "nan-class", "nonfinite=rejects"}
    if op == "tan" and fabs(res) == CONSTANTS["TAN_BIG"]:
        flags.add("pole-sentinel")
    if (
        op == "atan2"
        and not unrep
        and not is_zero(ins[1])
        and is_inf(div(ins[0], ins[1]))
    ):
        flags.add("beam-raises")
    cr, exact = (None, None)
    if "," not in arg:
        cr, exact = correct_rounded(op, ins[0])
    row.cr = hx(cr) if cr is not None else "-"
    row.exact = exact
    row.flags = sorted(flags, key=ALL_FLAGS.index)
    return row


def build_groups():
    """All input lists and the rows of every group, deterministically."""
    edge_pts = f64_edge()
    trig_pts = trig_points()
    edge_inputs = dedupe(edge_pts + trig_pts)
    domains = {
        "f64_edge": edge_pts,
        "trig_points": trig_pts,
        "sweep-binade": sweep_binade(),
        "sweep-cw": sweep_cw(),
        "sweep-prng": sweep_prng(),
    }
    groups = {}
    edge_rows = [make_row(op, hx(b)) for op in TRIG_OPS for b in edge_inputs]
    edge_rows += [make_row(op, arg) for op, arg in part2_rows()]
    groups["edge"] = edge_rows
    for name in ("sweep-binade", "sweep-cw", "sweep-prng"):
        groups[name] = [make_row(op, hx(b)) for op in TRIG_OPS for b in domains[name]]
    return domains, edge_inputs, groups


def group_text(rows) -> bytes:
    return ("".join(r.line() + "\n" for r in rows)).encode("utf-8")


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def decimal_ratio(frac: Fraction, digits: int = 4) -> str:
    """Non-negative Fraction as a decimal string, truncated, with integer maths only."""
    scaled = (frac.numerator * 10**digits) // frac.denominator
    s = str(scaled).rjust(digits + 1, "0")
    return f"{s[:-digits]}.{s[-digits:]}"


def measure_accuracy(groups):
    """Worst error in ulps of the exact value, and in representable steps, per op."""
    worst = {}
    for rows in groups.values():
        for r in rows:
            if r.op not in ("sin", "cos", "radians", "degrees") or r.cr == "-":
                continue
            y, cr = int(r.y, 16), int(r.cr, 16)
            steps = ulp_dist(y, cr)
            if r.exact is None or y == cr:
                err = Fraction(0)
            else:
                err = abs(to_fraction(y) - r.exact) / ulp_of(r.exact)
            w = worst.setdefault(
                r.op, {"ulp": Fraction(0), "steps": 0, "x_ulp": None, "x_steps": None}
            )
            if err > w["ulp"]:
                w["ulp"], w["x_ulp"] = err, r.x
            if steps > w["steps"]:
                w["steps"], w["x_steps"] = steps, r.x
    return worst


# ---------------------------------------------------------------------------
# Manifest, schema and README
# ---------------------------------------------------------------------------

SCHEMA = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "$id": "trig-reference-v1/schema.json",
    "title": "trig-reference-v1 manifest",
    "type": "object",
    "additionalProperties": False,
    "required": [
        "version",
        "algorithm",
        "generator_version",
        "seed",
        "constants",
        "groups",
        "domains",
        "accuracy",
    ],
    "properties": {
        "version": {"const": 1},
        "algorithm": {"const": ALGORITHM},
        "generator_version": {"type": "string"},
        "seed": {"type": "string", "pattern": "^[0-9a-f]{16}$"},
        "constants": {
            "type": "object",
            "additionalProperties": {"type": "string", "pattern": "^[0-9a-f]{16}$"},
        },
        "groups": {
            "type": "array",
            "items": {
                "type": "object",
                "additionalProperties": False,
                "required": ["name", "file", "rows", "sha256"],
                "properties": {
                    "name": {"type": "string"},
                    "file": {"type": "string"},
                    "rows": {"type": "integer", "minimum": 1},
                    "sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                },
            },
        },
        "domains": {
            "type": "object",
            "additionalProperties": {
                "type": "object",
                "additionalProperties": False,
                "required": ["count", "sha256"],
                "properties": {
                    "count": {"type": "integer", "minimum": 1},
                    "sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
                },
            },
        },
        "accuracy": {
            "type": "object",
            "additionalProperties": {
                "type": "object",
                "additionalProperties": False,
                "required": [
                    "worst_ulp",
                    "worst_ulp_x",
                    "worst_steps",
                    "worst_steps_x",
                    "gate_ulp",
                ],
                "properties": {
                    "worst_ulp": {"type": "string"},
                    "worst_ulp_x": {"type": ["string", "null"]},
                    "worst_steps": {"type": "integer"},
                    "worst_steps_x": {"type": ["string", "null"]},
                    "gate_ulp": {"type": "integer"},
                },
            },
        },
    },
}

GATE_ULP = {"sin": 3, "cos": 3, "radians": 1, "degrees": 1}


def build_manifest(domains, groups, files) -> dict:
    acc = measure_accuracy(groups)
    return {
        "version": 1,
        "algorithm": ALGORITHM,
        "generator_version": GENERATOR_VERSION,
        "seed": f"{SEED:016x}",
        "constants": {k: hx(v) for k, v in CONSTANTS.items()},
        "groups": [
            {
                "name": n,
                "file": f"{n}.tsv",
                "rows": len(groups[n]),
                "sha256": sha(files[n]),
            }
            for n in ("edge", "sweep-binade", "sweep-cw", "sweep-prng")
        ],
        "domains": {
            n: {
                "count": len(v),
                "sha256": sha(("".join(hx(b) + "\n" for b in v)).encode()),
            }
            for n, v in domains.items()
        },
        "accuracy": {
            op: {
                "worst_ulp": decimal_ratio(w["ulp"]),
                "worst_ulp_x": w["x_ulp"],
                "worst_steps": w["steps"],
                "worst_steps_x": w["x_steps"],
                "gate_ulp": GATE_ULP[op],
            }
            for op, w in sorted(acc.items())
        },
    }


def manifest_text(manifest: dict) -> bytes:
    return (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode("utf-8")


def schema_text() -> bytes:
    return (json.dumps(SCHEMA, indent=2, sort_keys=True) + "\n").encode("utf-8")


def build_outputs():
    """Everything the generator writes: {relative filename: bytes}, plus the groups."""
    domains, edge_inputs, groups = build_groups()
    files = {n: group_text(rows) for n, rows in groups.items()}
    manifest = build_manifest(domains, groups, files)
    out = {f"{n}.tsv": data for n, data in files.items()}
    out["manifest.json"] = manifest_text(manifest)
    out["schema.json"] = schema_text()
    return out, groups, domains, manifest, edge_inputs


# ---------------------------------------------------------------------------
# Gates (6.4 step 4)
# ---------------------------------------------------------------------------


class GateFailure(Exception):
    pass


def gate_accuracy(groups):
    worst = measure_accuracy(groups)
    for op, w in worst.items():
        limit = GATE_ULP[op]
        if w["ulp"] > limit:
            raise GateFailure(
                f"{op}: {decimal_ratio(w['ulp'])} ulp > {limit} at {w['x_ulp']}"
            )
        if w["steps"] > limit:
            raise GateFailure(f"{op}: {w['steps']} steps > {limit} at {w['x_steps']}")
    return worst


def _scan(variant: Model, inputs, ops):
    """First (op, x) at which the variant differs from the reference, else None."""
    for xb in inputs:
        for op in ops:
            ref = getattr(REF, op)(xb)
            try:
                got = getattr(variant, op)(xb)
            except NonFinite:
                return op, xb
            if not same(ref, got):
                return op, xb
    return None


def gate_sensitivity(inputs):
    """Mutation, fusion and guard sensitivity; returns a report, raises on failure."""
    report = {"mutation": {}, "fusion": {}, "guard": {}}
    sc = ("sin", "cos")
    for name in MUTATED:
        ops = ("radians", "degrees") if name in ("RAD_PER_DEG", "DEG_PER_RAD") else sc
        hit = _scan(Model(consts={name: CONSTANTS[name] ^ 1}), inputs, ops)
        report["mutation"][name] = hit
        if hit is None and name not in MASKED_MUTATION:
            raise GateFailure(
                f"a last-bit flip of {name} is not detected by any vector"
            )
    for site in SITES:
        hit = _scan(Model(fuse={site}), inputs, sc)
        report["fusion"][site] = hit
        if site in EXACT_PRODUCT_SITES:
            if hit is not None:
                raise GateFailure(
                    f"fusion at {site} changed {hit}, but its product is exact"
                )
        elif hit is None and site not in MASKED_FUSION:
            raise GateFailure(f"fusion at {site} is not detected by any vector")
    for label, model in (
        ("gt-guard", Model(guard="gt")),
        ("limit+1ulp", Model(consts={"LIMIT": LIMIT + 1})),
        ("no-guard", Model(guard="none")),
    ):
        hit = _scan(model, inputs, sc)
        report["guard"][label] = hit
        if hit is None:
            raise GateFailure(f"guard variant {label} is not detected by any vector")
    return report


# ---------------------------------------------------------------------------
# External cross-checks
# ---------------------------------------------------------------------------


def _tmpdir() -> Path:
    return Path(tempfile.mkdtemp(prefix="trig-xcheck-"))


def cross_check_hosts(groups):
    """Every y must equal the C (-ffp-contract=off) and Rust implementations' results."""
    work = _tmpdir()
    try:
        cc = shutil.which("gcc") or shutil.which("cc")
        rustc = shutil.which("rustc")
        if not cc or not rustc:
            raise GateFailure(
                "cross-check needs gcc and rustc (use --no-external to skip)"
            )
        xdir = HERE / "trig_reference_xcheck"
        subprocess.run(
            [
                cc,
                "-O2",
                "-ffp-contract=off",
                "-o",
                str(work / "xc"),
                str(xdir / "xcheck.c"),
            ],
            check=True,
        )
        subprocess.run(
            [rustc, "-O", "-o", str(work / "xr"), str(xdir / "xcheck.rs")], check=True
        )
        rows = [r for rs in groups.values() for r in rs]
        text = "".join(f"{r.op} {r.x}\n" for r in rows)
        for exe in ("xc", "xr"):
            out = subprocess.run(
                [str(work / exe)],
                input=text,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.split()
            if len(out) != len(rows):
                raise GateFailure(f"{exe}: {len(out)} results for {len(rows)} rows")
            for r, got in zip(rows, out):
                if got == "error" or r.y == "error":
                    ok = got == r.y
                else:
                    ok = same(int(r.y, 16), int(got, 16))
                if not ok:
                    raise GateFailure(
                        f"{exe} disagrees at {r.op} {r.x}: model {r.y}, host {got}"
                    )
        return len(rows)
    finally:
        shutil.rmtree(work, ignore_errors=True)


def cross_check_bc(groups):
    """GNU bc -l (scale=100) must round to the same cr on every edge row and 1 in 8 of each sweep."""
    bc = shutil.which("bc")
    if not bc:
        raise GateFailure("bc not found (use --no-external to skip)")
    picked = []
    for name, rows in groups.items():
        for i, r in enumerate(rows):
            if (
                r.op in ("sin", "cos")
                and r.cr != "-"
                and (name == "edge" or i % 8 == 0)
                and fabs(int(r.x, 16)) >= TINY
            ):
                picked.append(r)
    lines = ["scale=100"]
    for r in picked:
        xf = to_fraction(int(r.x, 16))
        fn = "s" if r.op == "sin" else "c"
        lines.append(f"{fn}(({xf.numerator})/({xf.denominator}))")
    env = dict(os.environ, BC_LINE_LENGTH="0")
    out = subprocess.run(
        [bc, "-l"],
        input="\n".join(lines) + "\n",
        capture_output=True,
        text=True,
        env=env,
        check=True,
    ).stdout.split()
    if len(out) != len(picked):
        raise GateFailure(f"bc returned {len(out)} values for {len(picked)} rows")
    for r, txt in zip(picked, out):
        neg = txt.startswith("-")
        digits = txt.lstrip("-")
        whole, _, frac = digits.partition(".")
        frac = frac or "0"
        v = Fraction(int(whole or "0") * 10 ** len(frac) + int(frac), 10 ** len(frac))
        got = round_fraction(-v if neg else v)
        if got != int(r.cr, 16):
            raise GateFailure(f"bc disagrees at {r.op} {r.x}: cr {r.cr}, bc {got:016x}")
    return len(picked)


# ---------------------------------------------------------------------------
# README
# ---------------------------------------------------------------------------


def readme_text(manifest: dict) -> bytes:
    acc = manifest["accuracy"]
    groups = "\n".join(
        f"| `{g['file']}` | {g['rows']} | `{g['sha256']}` |" for g in manifest["groups"]
    )
    worst = "\n".join(
        f"| `{op}` | {a['worst_ulp']} ulp at `{a['worst_ulp_x']}` | {a['worst_steps']} at `{a['worst_steps_x']}` | {a['gate_ulp']} |"
        for op, a in acc.items()
    )
    text = f"""# trig-reference-v1

The bit-exact conformance corpus of [PHY00a](../../PHY00a-trig-convergence.md)
(TRIG-REF-1). Every `trig` port must return **exactly** the bits in the `y` column.
It extends, and does not replace, the tolerance corpus
[`phy00-phy01-v1`](../phy00-phy01-v1/README.md).

Do not edit the `.tsv`, `manifest.json` or `schema.json` by hand. They are generated
and hashed, and `.gitattributes` marks them `-text` so no checkout rewrites a line end.

## Row format

One vector per line, five tab-separated fields, `\\n` line ends, UTF-8 without BOM:

    op <TAB> x <TAB> y <TAB> cr <TAB> flags

| Field | Content |
|---|---|
| `op` | `sin cos tan radians degrees sqrt atan atan2` |
| `x` | the input's IEEE binary64 bits, 16 lowercase hex digits. `atan2` has `<y>,<x>` (33 characters, the first argument first) |
| `y` | the TRIG-REF-1 result bits, 16 lowercase hex digits. A NaN result is written `7ff8000000000000` and matches **any** NaN (VER04p `same`); `error` for `sqrt` of a negative |
| `cr` | the correctly rounded value of the mathematical function at the exact input, 16 hex digits, or `-` when none is claimed (NaN or infinite result, every `tan` row, Part II) |
| `flags` | comma-separated, closed set: `nan-class`, `nonfinite=unrepresentable` (an input that cannot exist as a BEAM float; such a row never also carries `nonfinite=rejects`), `nonfinite=rejects` (a non-finite result from a representable input), `out-of-domain` (finite `|x| > 2^29` given to `sin`/`cos`/`tan`), `pole-sentinel` (a `tan` result of +-1e308), `beam-raises` (`atan2` whose `y/x` overflows), `signed-zero` (the result is +-0) |

A decimal number, an exponent letter, a `0x` prefix, upper-case hex or whitespace inside a
field is a corpus error (one value has one spelling, so hashes are stable).

## Groups

| File | Rows | SHA-256 |
|---|---|---|
{groups}

`edge.tsv` holds, for each of `sin cos tan radians degrees`, VER04p 11.1's edge list
plus the trig-specific points of PHY00a 6.3, then the Part II rows (`sqrt`, `atan`,
`atan2`). The three sweeps are bounded sets (never "exhaustive"): `sweep-binade`
(`2^e (1 + j/32)`, `e` in [-30, 30], both signs), `sweep-cw` (neighbours of `fl(k pi/2)`
for `k` up to 3.4e8, the continued-fraction hard cases, and 512 binade-drop inputs
`fl(k pi/2 + r)` with `r` just above 1/2, 1/4, 1/8), and `sweep-prng` (1,024 raw bit
patterns, 1,024 log-uniform, 2,048 uniform in [-2^29, 2^29]). `manifest.json` records
each group's row count and SHA-256, the constant table of PHY00a 3.2, the hash of every
input list, and the measured accuracy.

### The PRNG

xoshiro256** whose four state words are the first four outputs of SplitMix64 seeded with
the integer `0x{SEED:016x}` (the ASCII bytes `TRIG-REF`, big-endian). `sweep-prng` starts a
stream from that seed; the binade-drop part of `sweep-cw` starts a second stream from the
seed XOR 1. Log-uniform: `u = next()`, sign `u >> 63`, exponent `-27 + u mod 56`, mantissa
`next() & (2^52 - 1)`. Uniform: `n = next() >> 11`, value `(n - 2^52) 2^-23`. Binade-drop:
`k = 2^26 + next() mod (340000000 - 2^26 + 1)`, `u = next() >> 11`, `s = next() >> 63`,
`r = c (1 + u / 2^53 / 16)` for `c` cycling over 1/2, 1/4, 1/8, input
`(-1)^s fl(k pi/2 + r)` rounded once, from exact arithmetic.

## Measured accuracy (every row that has a `cr`)

| Op | Worst error vs the exact value | Worst steps vs `cr` | Gate |
|---|---|---|---|
{worst}

## Independence

`y` is computed by an integer-only soft-float model of the algorithm
(`code/scripts/trig_ref_softfloat.py`) and cross-checked against two unrelated host
implementations (`code/scripts/trig_reference_xcheck/xcheck.c` built with
`-ffp-contract=off`, and `xcheck.rs`); the generator stops on any disagreement. `cr` is
computed from exact rational arithmetic (`code/scripts/trig_ref_exact.py`) and
cross-checked against GNU `bc -l` at 100 digits. No host `sin`, `cos` or `float()`
touches a corpus value.

## Regenerate and check

    python3 code/scripts/generate_trig_reference_fixture.py          # write, run the gates and cross-checks
    python3 code/scripts/generate_trig_reference_fixture.py --check  # regenerate in memory; fail on any byte of difference
    python3 -m pytest code/scripts/tests/test_trig_reference_fixture.py

## What a port does

Decode `x` from hex by a bit cast, call the public function, encode the result back to
bits and compare as integers (NaN as a class). Skip `nonfinite=unrepresentable` rows (BEAM);
on `nonfinite=rejects` and `beam-raises` rows require the raise. PHY00a 6.5 has the rules.
"""
    return text.encode("utf-8")


# ---------------------------------------------------------------------------
# Command line
# ---------------------------------------------------------------------------


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument(
        "--check",
        action="store_true",
        help="regenerate in memory and fail on any byte of difference",
    )
    ap.add_argument(
        "--no-external", action="store_true", help="skip the C/Rust/bc cross-checks"
    )
    ap.add_argument(
        "--no-gates", action="store_true", help="skip the sensitivity gates (slow)"
    )
    ap.add_argument("--out", type=Path, default=FIXTURE)
    args = ap.parse_args(argv)

    out, groups, _domains, manifest, _edge_inputs = build_outputs()
    out["README.md"] = readme_text(manifest)
    print(
        f"rows: { {n: len(r) for n, r in groups.items()} }  total {sum(len(r) for r in groups.values())}"
    )

    acc = gate_accuracy(groups)
    for op, w in sorted(acc.items()):
        print(
            f"accuracy {op}: worst {decimal_ratio(w['ulp'])} ulp at {w['x_ulp']}, {w['steps']} steps (gate {GATE_ULP[op]})"
        )
    for rows in groups.values():
        for r in rows:
            if (
                r.op in ("sin", "cos", "tan")
                and "out-of-domain" in r.flags
                and r.y != hx(QNAN)
            ):
                raise GateFailure(f"finite result beyond LIMIT at {r.op} {r.x}")
    if not args.no_gates:
        inputs = dedupe(
            int(r.x, 16) for rs in groups.values() for r in rs if r.op == "sin"
        )
        report = gate_sensitivity(inputs)
        for kind, items in report.items():
            undetected = [k for k, v in items.items() if v is None]
            print(
                f"{kind} sensitivity: {len(items) - len(undetected)}/{len(items)} detected; undetected: {undetected}"
            )
    if not args.no_external:
        print("host cross-check (C, Rust): rows agreeing =", cross_check_hosts(groups))
        print("bc cross-check of cr: rows agreeing =", cross_check_bc(groups))

    if args.check:
        bad = []
        for name, data in out.items():
            path = args.out / name
            if not path.exists() or path.read_bytes() != data:
                bad.append(name)
        extra = sorted(p.name for p in args.out.glob("*") if p.name not in out)
        if bad or extra:
            print(f"MISMATCH: differs {bad}, unexpected files {extra}", file=sys.stderr)
            return 1
        print("fixture reproduced byte for byte:", ", ".join(sorted(out)))
        return 0
    args.out.mkdir(parents=True, exist_ok=True)
    for name, data in out.items():
        (args.out / name).write_bytes(data)
    print("wrote", args.out)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except GateFailure as exc:
        print(f"GATE FAILURE: {exc}", file=sys.stderr)
        sys.exit(1)
