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
