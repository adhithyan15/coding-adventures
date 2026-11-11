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

from coding_adventures_lz78 import compress, decompress

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


def _hdr(length: int, count: int) -> bytes:
    return struct.pack(">II", length, count)


def test_max_header_only() -> None:
    assert _bounded(decompress, b"\xff" * 8) == b""


def test_max_length_zero_tokens() -> None:
    assert _bounded(decompress, _hdr(0xFFFFFFFF, 0)) == b""


def test_count_larger_than_remaining_bytes_yields_present_tokens() -> None:
    data = _hdr(2, 0xFFFFFFFF) + bytes([0, 0, 104, 0, 0, 0, 105, 0])
    assert _bounded(decompress, data) == b"hi"
    assert _bounded(decompress, data + b"\x00\x00\x00") == b"hi"


def test_length_larger_than_tokens_supply() -> None:
    data = _hdr(0xFFFFFFFF, 1) + bytes([0, 0, 104, 0])
    assert _bounded(decompress, data) == b"h"


def test_chain_tokens_stop_at_declared_length() -> None:
    # 20k tokens each extending the previous entry would expand to ~2e8 bytes.
    n = 20000
    body = b"".join(struct.pack(">HBB", i, 97, 0) for i in range(n))
    out = _bounded(decompress, _hdr(1000, n) + body)
    assert len(out) == 1000


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
