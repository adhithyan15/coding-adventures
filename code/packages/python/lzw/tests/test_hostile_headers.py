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

from coding_adventures_lzw import compress, decompress

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


def test_max_header_only() -> None:
    _bounded(decompress, b"\xff" * 4)
    _bounded(decompress, b"\xff" * 64)


def test_max_declared_length_on_small_stream() -> None:
    c = compress(b"hello hello hello")
    hostile = struct.pack(">I", 0xFFFFFFFF) + c[4:]
    assert _bounded(decompress, hostile) == b"hello hello hello"


def test_declared_length_truncates_output() -> None:
    c = compress(b"hello hello hello")
    hostile = struct.pack(">I", 5) + c[4:]
    assert _bounded(decompress, hostile) == b"hello"


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
