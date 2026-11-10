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

from coding_adventures_lz77 import Token, compress, decode, decompress

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


def test_max_count_header_only() -> None:
    assert _bounded(decompress, b"\xff\xff\xff\xff") == b""


def test_count_larger_than_remaining_bytes_yields_present_tokens() -> None:
    data = b"\xff\xff\xff\xff" + bytes([0, 0, 0, 104, 0, 0, 0, 105])
    assert _bounded(decompress, data) == b"hi"
    assert _bounded(decompress, data + b"\x00\x00\x00") == b"hi"


def test_count_smaller_than_remaining_bytes_stops_at_count() -> None:
    data = struct.pack(">I", 1) + bytes([0, 0, 0, 104, 0, 0, 0, 105])
    assert decompress(data) == b"h"


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


def test_decode_offset_zero_stops_and_returns_prefix() -> None:
    # Offset 0 would give start == len(output), reading past the end. Decoding
    # must stop at that token: its length and next_char are not emitted, and
    # neither is anything after it.
    tokens = [Token(0, 0, 65), Token(0, 3, 66), Token(0, 0, 67)]
    assert decode(tokens) == b"A"


def test_decode_negative_offset_stops_instead_of_wrapping() -> None:
    # A negative offset would make start positive-from-the-end in Python
    # indexing terms and silently copy the wrong bytes. It must stop instead.
    tokens = [Token(0, 0, 65), Token(0, 0, 66), Token(-1, 2, 67)]
    assert decode(tokens) == b"AB"


def test_decode_offset_beyond_output_stops_and_returns_prefix() -> None:
    # Only one byte has been produced, so offset 5 reaches before the start.
    tokens = [Token(0, 0, 65), Token(5, 2, 66)]
    assert decode(tokens) == b"A"


def test_decode_offset_one_past_output_stops_and_returns_prefix() -> None:
    # Two bytes exist ("AB"), so offset 3 is the first invalid value.
    tokens = [Token(0, 0, 65), Token(1, 0, 66), Token(3, 1, 67)]
    assert decode(tokens) == b"AB"


def test_decode_offset_equal_to_output_length_is_valid() -> None:
    # Boundary: offset == len(output) is the furthest back a match may reach
    # and must still decode. "A" then (offset 1, length 2) gives "AAA", then "B".
    tokens = [Token(0, 0, 65), Token(1, 2, 66)]
    assert decode(tokens) == b"AAAB"


def test_decompress_hostile_offset_returns_prefix_without_error() -> None:
    # Serialised stream: count 2, token (offset 0, length 0, 'A'),
    # then token (offset 9, length 4, 'B') which points before the start.
    data = (
        struct.pack(">I", 2)
        + struct.pack(">HBB", 0, 0, 65)
        + struct.pack(">HBB", 9, 4, 66)
    )
    assert _bounded(decompress, data) == b"A"
