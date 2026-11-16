"""Bit-exact conformance of ``trig`` against the shared TRIG-REF-1 vectors.

``code/specs/fixtures/trig-reference-v1`` holds every vector as a row

    op <TAB> x <TAB> y <TAB> cr <TAB> flags

where ``x`` and ``y`` are IEEE binary64 *bit patterns* (16 lowercase hex
digits), never decimals.  This test decodes ``x`` by a bit cast, calls the
public function, encodes the result back to bits and compares the two
**integers** (PHY00a 6.5).  Printed floats are never compared: they hide the last
digits.  The only relaxation is NaN, which matches any NaN (VER04p ``same``).

The tests also assert the bits of every constant the module embeds, including
the decimal literals the source writes in code (``1e-15`` and friends), because
"the literal parses to the nearest double" is a property of the interpreter,
not of this package.
"""

from __future__ import annotations

import hashlib
import json
import math
import struct
from pathlib import Path

import pytest
import trig
from trig import trig as impl

FIXTURE = (
    Path(__file__).resolve().parents[4] / "specs" / "fixtures" / "trig-reference-v1"
)
GROUPS = ("edge", "sweep-binade", "sweep-cw", "sweep-prng")
QNAN_BITS = 0x7FF8000000000000


def bits_to_float(bits: int) -> float:
    return struct.unpack("<d", struct.pack("<Q", bits))[0]


def float_to_bits(value: float) -> int:
    return struct.unpack("<Q", struct.pack("<d", value))[0]


def load_rows() -> list[tuple[str, str, str, str, str, str]]:
    """(group, op, x, y, cr, flags) for every vector, in file order."""
    rows = []
    for group in GROUPS:
        text = (FIXTURE / f"{group}.tsv").read_text(encoding="utf-8")
        assert text.endswith("\n")
        for line in text.split("\n")[:-1]:
            op, x, y, cr, flags = line.split("\t")
            rows.append((group, op, x, y, cr, flags))
    return rows


ROWS = load_rows()


def call(op: str, x: str) -> float:
    if op == "atan2":
        ys, xs = x.split(",")
        return trig.atan2(bits_to_float(int(ys, 16)), bits_to_float(int(xs, 16)))
    return getattr(trig, op)(bits_to_float(int(x, 16)))


class TestFixtureIntegrity:
    """The fixture on disk is the one the manifest describes."""

    def test_manifest_hashes_and_counts(self) -> None:
        manifest = json.loads((FIXTURE / "manifest.json").read_text(encoding="utf-8"))
        assert manifest["algorithm"] == "TRIG-REF-1"
        names = [g["name"] for g in manifest["groups"]]
        assert names == list(GROUPS)
        for g in manifest["groups"]:
            data = (FIXTURE / g["file"]).read_bytes()
            assert hashlib.sha256(data).hexdigest() == g["sha256"]
            assert data.count(b"\n") == g["rows"]

    def test_row_syntax(self) -> None:
        hexdigit = set("0123456789abcdef")
        for _group, op, x, y, cr, _flags in ROWS:
            assert op in {
                "sin",
                "cos",
                "tan",
                "radians",
                "degrees",
                "sqrt",
                "atan",
                "atan2",
            }
            parts = x.split(",") if op == "atan2" else [x]
            assert len(parts) == (2 if op == "atan2" else 1)
            for field in [
                *parts,
                *([y] if y != "error" else []),
                *([cr] if cr != "-" else []),
            ]:
                assert len(field) == 16 and set(field) <= hexdigit

    def test_constants_in_the_manifest_match_the_module(self) -> None:
        manifest = json.loads((FIXTURE / "manifest.json").read_text(encoding="utf-8"))
        consts = {k: int(v, 16) for k, v in manifest["constants"].items()}
        assert float_to_bits(impl.PI) == consts["PI"]
        assert float_to_bits(impl.TWO_PI) == consts["TWO_PI"]
        assert float_to_bits(impl.HALF_PI) == consts["HALF_PI"]
        assert float_to_bits(impl._INV_PIO2) == consts["INV_PIO2"]
        for i, piece in enumerate(impl._PIO2, start=1):
            assert float_to_bits(piece) == consts[f"PIO2_{i}"]
        assert float_to_bits(impl._MAGIC) == consts["MAGIC"]
        assert float_to_bits(impl._LIMIT) == consts["LIMIT"]
        assert float_to_bits(impl._TINY) == consts["TINY"]
        for n in range(1, 10):
            assert float_to_bits(impl._S[n]) == consts[f"S{n}"]
        for n in range(2, 10):
            assert float_to_bits(impl._C[n]) == consts[f"C{n}"]
        assert float_to_bits(impl._RAD_PER_DEG) == consts["RAD_PER_DEG"]
        assert float_to_bits(impl._DEG_PER_RAD) == consts["DEG_PER_RAD"]


class TestLiteralConstants:
    """Decimal literals written in the source parse to the pinned bit patterns."""

    @pytest.mark.parametrize(
        ("literal", "bits"),
        [
            (1e-15, 0x3CD203AF9EE75616),  # tan pole threshold, sqrt tolerance
            (1.0e308, 0x7FE1CCF385EBC8A0),  # tan pole sentinel
            (1e-17, 0x3C670EF54646D497),  # atan series cut-off
            (1e-300, 0x01A56E1FC2F8F359),  # sqrt subnormal floor
            (
                7.450580596923828e-9,
                0x3E40000000000000,
            ),  # atan identity threshold == 2**-27
        ],
    )
    def test_literal_bits(self, literal: float, bits: int) -> None:
        assert float_to_bits(literal) == bits

    def test_degree_constants_equal_the_divisions(self) -> None:
        assert impl._RAD_PER_DEG == impl.PI / 180.0
        assert impl._DEG_PER_RAD == 180.0 / impl.PI


@pytest.mark.parametrize("group", GROUPS)
def test_every_vector_matches_bit_for_bit(group: str) -> None:
    """Every row of the group: the function returns exactly the reference bits."""
    failures = []
    checked = 0
    for g, op, x, y, _cr, _flags in ROWS:
        if g != group:
            continue
        checked += 1
        if y == "error":
            try:
                call(op, x)
            except ValueError:
                continue
            failures.append((op, x, "no error raised"))
            continue
        got = call(op, x)
        if int(y, 16) == QNAN_BITS:
            if not math.isnan(got):
                failures.append((op, x, f"expected NaN, got {float_to_bits(got):016x}"))
        elif float_to_bits(got) != int(y, 16):
            failures.append((op, x, f"expected {y}, got {float_to_bits(got):016x}"))
    assert checked > 0
    assert not failures, failures[:10]


def test_accuracy_against_the_correctly_rounded_column() -> None:
    """Every row with a ``cr`` is within 3 steps (sin, cos) or 1 step (radians, degrees)."""

    def ordered(bits: int) -> int:
        mag = bits & 0x7FFFFFFFFFFFFFFF
        return -mag if bits >> 63 else mag

    limit = {"sin": 3, "cos": 3, "radians": 1, "degrees": 1}
    worst = dict.fromkeys(limit, 0)
    for _g, op, x, _y, cr, _flags in ROWS:
        if cr == "-":
            continue
        got = float_to_bits(call(op, x))
        steps = abs(ordered(got) - ordered(int(cr, 16)))
        worst[op] = max(worst[op], steps)
    for op, bound in limit.items():
        assert worst[op] <= bound, (op, worst[op])


class TestContract:
    """The behaviours PHY00a 3.3 names, asserted directly."""

    def test_sin_of_negative_zero_is_negative_zero(self) -> None:
        assert float_to_bits(trig.sin(-0.0)) == 0x8000000000000000
        assert float_to_bits(trig.sin(0.0)) == 0

    def test_cos_of_zero_is_one(self) -> None:
        assert trig.cos(0.0) == 1.0
        assert trig.cos(-0.0) == 1.0

    def test_tiny_arguments_are_returned_unchanged(self) -> None:
        tiny = bits_to_float(0x3E3FFFFFFFFFFFFF)
        assert trig.sin(tiny) == tiny
        assert trig.sin(5e-324) == 5e-324
        assert trig.cos(tiny) == 1.0

    @pytest.mark.parametrize("fn", [trig.sin, trig.cos, trig.tan])
    @pytest.mark.parametrize(
        "x",
        [
            float("nan"),
            float("inf"),
            -float("inf"),
            1e20,
            2.0**29 * (1 + 2.0**-52),
            1.7976931348623157e308,
        ],
    )
    def test_outside_the_domain_is_nan(self, fn, x: float) -> None:
        assert math.isnan(fn(x))

    def test_the_last_in_domain_value_is_finite(self) -> None:
        assert float_to_bits(trig.sin(2.0**29)) == 0x3FD4E67C0E2622DF
        assert float_to_bits(trig.cos(2.0**29)) == 0xBFEE3EDD2DFEEF90

    def test_integer_arguments_are_converted_once(self) -> None:
        assert trig.sin(1) == trig.sin(1.0)
        assert trig.cos(3) == trig.cos(3.0)
        assert trig.radians(180) == trig.PI
        assert trig.degrees(0) == 0.0
        assert float_to_bits(trig.radians(-0.0)) == 0x8000000000000000
        assert float_to_bits(trig.degrees(-0.0)) == 0x8000000000000000

    def test_radians_and_degrees_are_single_multiplies(self) -> None:
        assert trig.radians(180.0) == trig.PI
        assert trig.degrees(trig.PI) == 180.0
        assert trig.degrees(1.7e308) == float("inf")
        assert math.isnan(trig.radians(float("nan")))

    def test_tan_pole_sentinel(self) -> None:
        assert trig.tan(trig.HALF_PI) == 1e308
        assert trig.tan(-trig.HALF_PI) == -1e308

    def test_atan2_of_nan_is_nan(self) -> None:
        nan = float("nan")
        assert math.isnan(trig.atan2(nan, 1.0))
        assert math.isnan(trig.atan2(1.0, nan))
        assert math.isnan(trig.atan2(-1.0, nan))
        assert math.isnan(trig.atan2(nan, nan))

    def test_atan2_pinned_conventions(self) -> None:
        assert trig.atan2(-0.0, -1.0) == trig.PI
        assert trig.atan2(0.0, 0.0) == 0.0
        assert math.isnan(trig.atan2(float("inf"), float("inf")))
        assert trig.atan2(1e300, 1e-300) == trig.HALF_PI

    def test_atan_keeps_the_sign_of_zero(self) -> None:
        assert float_to_bits(trig.atan(-0.0)) == 0x8000000000000000

    def test_negative_quadrant_arithmetic(self) -> None:
        # k = -1, -5 map to quadrant 3: "& 3" is the floor modulus.
        x = -trig.HALF_PI
        assert trig.sin(x) == -trig.sin(trig.HALF_PI)
        assert trig.cos(-5 * trig.HALF_PI) == pytest.approx(0.0, abs=1e-15)

    def test_huge_integers_beyond_float_range_are_rejected_loudly(self) -> None:
        with pytest.raises(OverflowError):
            trig.sin(10**400)
