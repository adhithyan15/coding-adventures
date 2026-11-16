"""Tests for fuzz testing mutators and execution engine."""

import pytest

from coding_adventures_test_bootstrap.fuzz import (
    MAX_CRASHES,
    MAX_INPUT_BYTES,
    MAX_ITERATIONS,
    bit_flip,
    byte_delete,
    byte_flip,
    byte_insert,
    byte_replace,
    crossover,
    fuzz_bytes,
    fuzz_text,
    mutate_bytes,
    mutate_text,
)
from coding_adventures_test_bootstrap.prng import SplitMix64


def test_bit_flip() -> None:
    data = b"\x00\x00"
    flipped = bit_flip(data, 0, 1)
    assert flipped == b"\x01\x00"

    assert bit_flip(b"", 0, 5) == b""
    assert bit_flip(b"a", 0, 0) == b"a"

    # Wrapping parity test: 8 flips on single byte flips all bits to 0xff
    assert bit_flip(b"\x00", 0, 8) == b"\xff"
    assert bit_flip(b"\x00", 0, 16) == b"\x00"
    # 9 flips: full cycle (all bits) + bit 0 again -> bits 1..7 are 1, bit 0 is 0
    assert bit_flip(b"\x00", 0, 9) == b"\xfe"


def test_byte_flip() -> None:
    data = b"\x00\x00"
    flipped = byte_flip(data, 0, 1)
    assert flipped == b"\xff\x00"

    assert byte_flip(b"", 0, 5) == b""
    assert byte_flip(b"a", 0, 0) == b"a"
    assert byte_flip(b"\x00", 0, 2) == b"\x00"


def test_byte_insert_delete_replace() -> None:
    assert byte_insert(b"ac", 1, b"b") == b"abc"
    assert byte_insert(b"", 0, b"x") == b"x"
    assert byte_insert(b"a", 0, b"") == b"a"

    assert byte_delete(b"abcd", 1, 2) == b"ad"
    assert byte_delete(b"", 0, 1) == b""
    assert byte_delete(b"a", 0, 0) == b"a"

    assert byte_replace(b"abcd", 1, b"XY") == b"aXYd"
    assert byte_replace(b"", 0, b"x") == b""
    assert byte_replace(b"a", 0, b"") == b"a"


def test_crossover() -> None:
    d1 = b"AAAA"
    d2 = b"BBBB"
    cross = crossover(d1, d2, 2)
    assert cross == b"AABB"
    assert crossover(b"", d2, 1) == d2
    assert crossover(d1, b"", 1) == d1


def test_mutate_bytes_branches() -> None:
    rng = SplitMix64(42)
    empty_mut = mutate_bytes(b"", rng)
    assert len(empty_mut) > 0

    base = b"abcdefghij"
    for _ in range(50):
        mut = mutate_bytes(base, rng)
        assert isinstance(mut, bytes)


def test_mutate_text_branches() -> None:
    rng = SplitMix64(99)
    empty_mut = mutate_text("", rng)
    assert len(empty_mut) > 0

    base = "sample string for mutation"
    for _ in range(50):
        mut = mutate_text(base, rng)
        assert isinstance(mut, str)


def test_fuzz_bytes_no_crashes() -> None:
    def safe_target(data: bytes) -> None:
        pass

    res = fuzz_bytes(safe_target, seed=123, iterations=50)
    assert res.iterations_run == 50
    assert len(res.crashes) == 0
    assert res.unique_crashes == 0


def test_fuzz_bytes_catches_crashes() -> None:
    def crashing_target(data: bytes) -> None:
        if b"\x00\x00" in data:
            raise ValueError("Found double zero")

    res = fuzz_bytes(crashing_target, seed=42, iterations=100)
    if res.crashes:
        assert res.unique_crashes >= 1
        assert "ValueError" in res.crashes[0].exception


def test_fuzz_text_catches_crashes() -> None:
    def crashing_target(s: str) -> None:
        if len(s) > 3 and "x" in s:
            raise RuntimeError("Contains x")

    res = fuzz_text(crashing_target, seed=77, iterations=100)
    if res.crashes:
        assert res.unique_crashes >= 1
        assert "RuntimeError" in res.crashes[0].exception


def test_fuzz_validation_bounds() -> None:
    with pytest.raises(TypeError):
        fuzz_bytes(lambda d: None, iterations="10")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="positive"):
        fuzz_bytes(lambda d: None, iterations=0)
    with pytest.raises(ValueError, match="MAX_ITERATIONS"):
        fuzz_bytes(lambda d: None, iterations=MAX_ITERATIONS + 1)
    with pytest.raises(TypeError):
        fuzz_bytes(lambda d: None, max_input_bytes="10")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="positive"):
        fuzz_bytes(lambda d: None, max_input_bytes=0)
    with pytest.raises(ValueError, match="MAX_INPUT_BYTES"):
        fuzz_bytes(lambda d: None, max_input_bytes=MAX_INPUT_BYTES + 1)

    with pytest.raises(TypeError):
        fuzz_text(lambda s: None, iterations="10")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="positive"):
        fuzz_text(lambda s: None, iterations=-1)
    with pytest.raises(ValueError, match="MAX_ITERATIONS"):
        fuzz_text(lambda s: None, iterations=MAX_ITERATIONS + 1)
    with pytest.raises(TypeError):
        fuzz_text(lambda s: None, max_input_bytes="10")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="positive"):
        fuzz_text(lambda s: None, max_input_bytes=-1)
    with pytest.raises(ValueError, match="MAX_INPUT_BYTES"):
        fuzz_text(lambda s: None, max_input_bytes=MAX_INPUT_BYTES + 1)


def test_mutator_strict_int_and_max_len() -> None:
    rng = SplitMix64(1)
    with pytest.raises(TypeError):
        bit_flip(b"a", True)
    with pytest.raises(TypeError):
        byte_flip(b"a", 0, True)
    with pytest.raises(TypeError):
        byte_insert(b"a", True, b"b")
    with pytest.raises(TypeError):
        byte_delete(b"a", 0, True)
    with pytest.raises(TypeError):
        byte_replace(b"a", True, b"b")
    with pytest.raises(TypeError):
        crossover(b"a", b"b", True)

    with pytest.raises(TypeError):
        mutate_bytes(b"a", rng, max_len=True)
    with pytest.raises(ValueError):
        mutate_bytes(b"a", rng, max_len=0)
    with pytest.raises(ValueError):
        mutate_bytes(b"a", rng, max_len=MAX_INPUT_BYTES + 1)

    with pytest.raises(TypeError):
        mutate_text("a", rng, max_len=False)
    with pytest.raises(ValueError):
        mutate_text("a", rng, max_len=0)
    with pytest.raises(ValueError):
        mutate_text("a", rng, max_len=MAX_INPUT_BYTES + 1)


def test_initial_corpus_validation_and_crashes_cap() -> None:
    # fuzz_bytes corpus validation
    with pytest.raises(TypeError):
        fuzz_bytes(lambda d: None, initial_corpus="not_sequence")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="cannot be empty"):
        fuzz_bytes(lambda d: None, initial_corpus=[])
    with pytest.raises(TypeError, match="must be bytes"):
        fuzz_bytes(lambda d: None, initial_corpus=["string_not_bytes"])  # type: ignore[list-item]
    with pytest.raises(ValueError, match="exceeds max_input_bytes"):
        fuzz_bytes(lambda d: None, max_input_bytes=5, initial_corpus=[b"123456"])
    with pytest.raises(TypeError):
        fuzz_bytes(lambda d: None, seed=True)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        fuzz_bytes(lambda d: None, iterations=True)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        fuzz_bytes(lambda d: None, max_input_bytes=True)  # type: ignore[arg-type]

    # fuzz_text corpus validation
    with pytest.raises(TypeError):
        fuzz_text(lambda s: None, initial_corpus=123)  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="cannot be empty"):
        fuzz_text(lambda s: None, initial_corpus=[])
    with pytest.raises(TypeError, match="must be str"):
        fuzz_text(lambda s: None, initial_corpus=[b"bytes_not_str"])  # type: ignore[list-item]
    with pytest.raises(ValueError, match="exceeds max_input_bytes"):
        fuzz_text(lambda s: None, max_input_bytes=5, initial_corpus=["123456"])
    with pytest.raises(TypeError):
        fuzz_text(lambda s: None, seed=False)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        fuzz_text(lambda s: None, iterations=False)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        fuzz_text(lambda s: None, max_input_bytes=False)  # type: ignore[arg-type]

    # Valid custom initial_corpus passes
    res_b = fuzz_bytes(lambda d: None, iterations=10, initial_corpus=[b"abc", b"def"])
    assert res_b.iterations_run == 10
    res_t = fuzz_text(lambda s: None, iterations=10, initial_corpus=["abc", "def"])
    assert res_t.iterations_run == 10

    # Crash capping to MAX_CRASHES
    def always_crash(data: bytes) -> None:
        raise ValueError(f"Crash: {len(data)}")

    res_cap = fuzz_bytes(always_crash, iterations=1050)
    assert len(res_cap.crashes) == MAX_CRASHES
