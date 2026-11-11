"""Security regression tests: hostile headers must not drive work or memory.

Every case below is a few bytes of input whose header claims an enormous count
or length.  A decompressor that sizes an allocation or a loop from that claim
would use gigabytes or run for minutes; these tests assert bounded time and
bounded peak memory (tracemalloc), plus round-trip sanity over all inputs of
length <= 2 and a seeded random sample.
"""

from __future__ import annotations

import random
import struct
import time
import tracemalloc
from collections.abc import Callable

from coding_adventures_lzss import compress, decompress

LIMIT_BYTES = 64 * 1024 * 1024
LIMIT_SECONDS = 10.0


def _bounded(fn: Callable[[bytes], object], data: bytes) -> object:
    """Run fn(data) asserting it is quick and low-memory; return its result
    (or the exception it raised, so error-vs-empty is the caller's choice)."""
    tracemalloc.start()
    start = time.monotonic()
    try:
        try:
            result = fn(data)
        except Exception as exc:  # noqa: BLE001 - malformed input may raise
            result = exc
        _, peak = tracemalloc.get_traced_memory()
    finally:
        tracemalloc.stop()
    assert time.monotonic() - start < LIMIT_SECONDS
    assert peak < LIMIT_BYTES, f"peak {peak} bytes"
    return result


def _hdr(length: int, blocks: int) -> bytes:
    return struct.pack(">II", length, blocks)


def test_max_header_only() -> None:
    assert _bounded(decompress, b"\xff" * 8) == b""


def test_max_length_one_literal() -> None:
    assert _bounded(decompress, _hdr(0xFFFFFFFF, 1) + bytes([0, 120])) == b"x"


def test_block_count_larger_than_remaining_bytes() -> None:
    data = _hdr(2, 0xFFFFFFFF) + bytes([0, 104, 105])
    assert _bounded(decompress, data) == b"hi"


def test_length_inconsistent_with_tokens() -> None:
    assert decompress(_hdr(1, 1) + bytes([0, 104, 105])) == b"h"


def test_round_trip_all_inputs_up_to_length_2() -> None:
    inputs = [b""] + [bytes([a]) for a in range(256)]
    inputs += [bytes([a, b]) for a in range(256) for b in range(256)]
    for data in inputs:
        assert decompress(compress(data)) == data, data


def test_round_trip_seeded_random_inputs() -> None:
    rng = random.Random(0x5EED)
    for i in range(300):
        alphabet = (2, 4, 16, 256)[i % 4]
        data = bytes(rng.randrange(alphabet) for _ in range(rng.randrange(3, 600)))
        assert decompress(compress(data)) == data
