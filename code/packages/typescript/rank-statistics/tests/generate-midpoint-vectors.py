"""Generate exact binary64 midpoint vectors without using the TS algorithm.

Python's Fraction stores each input float as an exact rational. Converting the
exact mean back to float uses Python's binary64 round-to-nearest/ties-to-even
conversion; the checked-in output bits let the TypeScript tests avoid sharing
the production rounding implementation.
"""

from __future__ import annotations

import hashlib
import json
import random
import struct
import sys
from fractions import Fraction
from pathlib import Path

SIGN = 1 << 63
EXPONENT_MASK = 0x7FF0000000000000
MAX_FINITE = 0x7FEFFFFFFFFFFFFF


def from_bits(bits: int) -> float:
    return struct.unpack(">d", bits.to_bytes(8, "big"))[0]


def to_bits(value: float) -> str:
    return f"{int.from_bytes(struct.pack('>d', value), 'big'):016x}"


def midpoint(left_bits: int, right_bits: int) -> int:
    left = from_bits(left_bits)
    right = from_bits(right_bits)
    if left_bits == SIGN and right_bits == SIGN:
        return SIGN
    exact = (Fraction.from_float(left) + Fraction.from_float(right)) / 2
    return int.from_bytes(struct.pack(">d", float(exact)), "big")


def special_values() -> list[tuple[str, int]]:
    named = [
        ("positive_zero", 0x0000000000000000),
        ("negative_zero", 0x8000000000000000),
        ("min_subnormal", 0x0000000000000001),
        ("two_subnormals", 0x0000000000000002),
        ("three_subnormals", 0x0000000000000003),
        ("max_subnormal", 0x000FFFFFFFFFFFFF),
        ("min_normal", 0x0010000000000000),
        ("next_min_normal", 0x0010000000000001),
        ("one_below", 0x3FEFFFFFFFFFFFFF),
        ("one", 0x3FF0000000000000),
        ("one_above", 0x3FF0000000000001),
        ("two_below", 0x3FFFFFFFFFFFFFFF),
        ("two", 0x4000000000000000),
        ("two_above", 0x4000000000000001),
        ("max_finite_previous", MAX_FINITE - 1),
        ("max_finite", MAX_FINITE),
    ]
    result = list(named)
    result.extend(
        (f"negative_{name}", bits | SIGN)
        for name, bits in named
        if bits not in (0, SIGN)
    )
    return result


def main() -> None:
    output = Path(__file__).with_name("fixtures")
    output.mkdir(parents=True, exist_ok=True)
    values = special_values()
    vectors: list[dict[str, str]] = []
    for left_name, left_bits in values:
        for right_name, right_bits in values:
            vectors.append(
                {
                    "name": f"{left_name}__{right_name}",
                    "left_bits": f"{left_bits:016x}",
                    "right_bits": f"{right_bits:016x}",
                    "expected_bits": f"{midpoint(left_bits, right_bits):016x}",
                }
            )

    rng = random.Random(0x52415449)
    rejected = 0
    accepted = 0
    while accepted < 256:
        pair: list[int] = []
        while len(pair) < 2:
            candidate = rng.getrandbits(64)
            if candidate & EXPONENT_MASK == EXPONENT_MASK:
                rejected += 1
                continue
            pair.append(candidate)
        vectors.append(
            {
                "name": f"seeded_{accepted:03d}",
                "left_bits": f"{pair[0]:016x}",
                "right_bits": f"{pair[1]:016x}",
                "expected_bits": f"{midpoint(pair[0], pair[1]):016x}",
            }
        )
        accepted += 1

    payload = (json.dumps({"version": 1, "vectors": vectors}, indent=2) + "\n").encode()
    fixture_path = output / "midpoint-vectors-v1.json"
    fixture_path.write_bytes(payload)
    metadata = {
        "generator": "tests/generate-midpoint-vectors.py",
        "python": sys.version.split()[0],
        "arithmetic": "fractions.Fraction.from_float exact rational mean, then Python binary64 conversion",
        "special_value_count": len(values),
        "special_ordered_pair_count": len(values) ** 2,
        "seed": "0x52415449",
        "seeded_accepted_pairs": accepted,
        "seeded_rejected_nonfinite_bit_patterns": rejected,
        "vector_count": len(vectors),
        "fixture_sha256": hashlib.sha256(payload).hexdigest(),
    }
    (output / "midpoint-vectors-v1.meta.json").write_text(
        json.dumps(metadata, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
