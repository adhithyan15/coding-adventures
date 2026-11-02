from collections.abc import Iterator

import pytest

from coding_adventures_ct_compare import ct_eq, ct_eq_fixed, ct_eq_u64, ct_select_bytes


def test_ct_eq_matches_byte_equality() -> None:
    assert ct_eq(b"abcdef", b"abcdef")
    assert ct_eq(bytearray(b"abcdef"), memoryview(b"abcdef"))
    assert ct_eq(b"", b"")
    assert not ct_eq(b"abcdef", b"abcdeg")
    assert not ct_eq(b"abcdef", b"bbcdef")
    assert not ct_eq(b"abc", b"abcd")


def test_ct_eq_detects_every_single_bit_position() -> None:
    base = bytes([0x42]) * 32
    for index in range(32):
        for bit in range(8):
            flipped = bytearray(base)
            flipped[index] ^= 1 << bit
            assert not ct_eq(base, flipped)


def test_ct_eq_fixed_is_dynamic_alias() -> None:
    assert ct_eq_fixed([0x11] * 16, [0x11] * 16)
    assert not ct_eq_fixed([0x11] * 16, [0x11] * 15 + [0x10])


def test_ct_select_bytes_chooses_without_mutating_inputs() -> None:
    left = bytes(range(256))
    right = bytes(reversed(range(256)))

    assert ct_select_bytes(left, right, True) == left
    assert ct_select_bytes(left, right, False) == right
    assert ct_select_bytes(b"", b"", True) == b""

    with pytest.raises(ValueError, match="equal-length"):
        ct_select_bytes(b"abc", b"abcd", True)


def test_bare_integer_is_rejected_not_turned_into_zero_bytes() -> None:
    # Regression: bytes(3) is three zero bytes, so ct_eq(3, b"\0\0\0") was True.
    with pytest.raises(TypeError):
        ct_eq(3, b"\0\0\0")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        ct_select_bytes(2, b"\0\0", True)  # type: ignore[arg-type]


def test_non_byte_inputs_are_rejected() -> None:
    with pytest.raises(TypeError):
        ct_eq("abc", "abc")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        ct_eq(None, b"")  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        ct_eq(memoryview(bytes(8)).cast("I"), bytes(8))
    with pytest.raises(ValueError, match="0 to 255"):
        ct_eq([256], [0])
    with pytest.raises(ValueError, match="0 to 255"):
        ct_eq([True], [1])
    with pytest.raises(ValueError, match="0 to 255"):
        ct_eq((1.0,), (1,))  # type: ignore[arg-type]


def test_legitimate_byte_shapes_still_work() -> None:
    assert ct_eq((0, 127, 255), b"\x00\x7f\xff")
    assert ct_eq(memoryview(bytearray(b"ab")), [97, 98])


def test_ct_select_bytes_requires_a_real_bool_choice() -> None:
    for not_a_bool in (1, 0, "0", "", None, [1]):
        with pytest.raises(TypeError):
            ct_select_bytes(b"a", b"b", not_a_bool)  # type: ignore[arg-type]


def test_ct_select_bytes_returns_a_fresh_copy() -> None:
    left = bytearray(b"abc")
    picked = ct_select_bytes(left, b"xyz", True)
    left[0] = 0
    assert picked == b"abc"


def test_ct_eq_u64_rejects_bool() -> None:
    with pytest.raises(ValueError):
        ct_eq_u64(True, 1)


def test_ct_eq_u64_handles_edges_and_rejects_out_of_range() -> None:
    assert ct_eq_u64(0, 0)
    assert ct_eq_u64((1 << 64) - 1, (1 << 64) - 1)
    assert not ct_eq_u64(0, 1 << 63)

    base = 0x1234_5678_9ABC_DEF0
    for bit in range(64):
        assert not ct_eq_u64(base, base ^ (1 << bit))

    with pytest.raises(ValueError):
        ct_eq_u64(-1, 0)
    with pytest.raises(ValueError):
        ct_eq_u64(0, 1 << 64)


class _LyingList(list):
    def __bytes__(self) -> bytes:
        return b""


class _ShiftyList(list):
    """Shows validation one sequence and a later pass another."""

    def __init__(self, *args: object) -> None:
        super().__init__(*args)
        self.calls = 0

    def __iter__(self) -> Iterator[int]:
        self.calls += 1
        return iter([1, 2] if self.calls == 1 else [9])


class _ShortBytes(bytes):
    def __len__(self) -> int:
        return 0


class _LyingBytearray(bytearray):
    def __bytes__(self) -> bytes:
        return b"\0"


def test_subclasses_cannot_lie_about_their_bytes() -> None:
    # Regression: each of these used to compare equal to the wrong value.
    with pytest.raises(TypeError):
        ct_eq(_LyingList([1, 2, 3]), b"")
    with pytest.raises(TypeError):
        ct_eq(_ShiftyList([1, 2]), [9])
    with pytest.raises(TypeError):
        ct_eq(_ShortBytes(b"secret"), b"")
    with pytest.raises(TypeError):
        ct_eq(_LyingBytearray(b"zz"), b"\0")
