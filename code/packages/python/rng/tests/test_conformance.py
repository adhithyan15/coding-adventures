"""The shared rng corpus (spec rng.md, "Conformance corpus").

`code/specs/fixtures/rng-v1/cases.json` pins exact outputs that every
language's rng package must reproduce. Its results come from an independent
reference, `code/scripts/rng_conformance.py`, so passing here means this
package and that reference agree call for call.
"""

from __future__ import annotations

import json
import struct
from pathlib import Path

import pytest

from rng import LCG, PCG32, Xorshift64

CORPUS = Path(__file__).resolve().parents[4] / "specs/fixtures/rng-v1/cases.json"
GENERATORS = {"lcg": LCG, "xorshift64": Xorshift64, "pcg32": PCG32}
CASES = json.loads(CORPUS.read_text(encoding="utf-8"))["cases"]


def _float_bits(value: float) -> str:
    return f"{struct.unpack('<Q', struct.pack('<d', value))[0]:016x}"


@pytest.mark.parametrize("case", CASES, ids=[case["id"] for case in CASES])
def test_corpus_case(case: dict) -> None:
    g = GENERATORS[case["generator"]](int(case["seed"]))
    for index, call in enumerate(case["calls"]):
        op = call["op"]
        where = f"{case['id']} call {index} ({op})"
        if op == "next_u32":
            assert g.next_u32() == int(call["result"]), where
        elif op == "next_u64":
            assert g.next_u64() == int(call["result"]), where
        elif op == "next_float":
            assert _float_bits(g.next_float()) == call["result_bits"], where
        elif "error" in call:
            with pytest.raises(ValueError):
                g.next_int_in_range(int(call["min"]), int(call["max"]))
        else:
            got = g.next_int_in_range(int(call["min"]), int(call["max"]))
            assert got == int(call["result"]), where


# --- regressions for the v1 range bug --------------------------------------

I64_MIN, I64_MAX = -(1 << 63), (1 << 63) - 1


@pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
def test_wide_ranges_reach_beyond_two_to_the_32(cls: type) -> None:
    # v1 drew 32 bits for every span, so it never returned more than
    # 2^32 - 1 above min_val. Over 2^40 values, 64 draws all landing below
    # 2^32 has probability 2^-512.
    g = cls(3)
    assert any(g.next_int_in_range(0, 1 << 40) >= 1 << 32 for _ in range(64))


@pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
def test_the_whole_signed_64_bit_domain(cls: type) -> None:
    g = cls(3)
    values = [g.next_int_in_range(I64_MIN, I64_MAX) for _ in range(64)]
    assert all(I64_MIN <= v <= I64_MAX for v in values)
    assert any(v < 0 for v in values) and any(v > 0 for v in values)


@pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
@pytest.mark.parametrize(
    "bounds", [(1, 0), (I64_MIN - 1, 0), (0, I64_MAX + 1), (I64_MIN - 1, I64_MAX + 1)]
)
def test_invalid_bounds_raise_and_draw_nothing(
    cls: type, bounds: tuple[int, int]
) -> None:
    g, fresh = cls(9), cls(9)
    with pytest.raises(ValueError):
        g.next_int_in_range(*bounds)
    assert g.next_u32() == fresh.next_u32()
