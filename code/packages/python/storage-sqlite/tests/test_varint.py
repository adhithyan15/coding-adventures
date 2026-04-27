"""Tests for the SQLite varint codec."""

from __future__ import annotations

import pytest

from storage_sqlite.varint import decode, decode_signed, encode, encode_signed, size

# ------------------------------------------------------------------
# Known golden values (cross-checked against the SQLite spec).
# ------------------------------------------------------------------


@pytest.mark.parametrize(
    ("value", "raw"),
    [
        (0, b"\x00"),
        (1, b"\x01"),
        (127, b"\x7f"),                 # largest 1-byte
        (128, b"\x81\x00"),              # smallest 2-byte
        (300, b"\x82\x2c"),              # docstring example
        (16383, b"\xff\x7f"),            # largest 2-byte
        (16384, b"\x81\x80\x00"),        # smallest 3-byte
        ((1 << 56) - 1, b"\xff\xff\xff\xff\xff\xff\xff\x7f"),  # largest 8-byte
        ((1 << 56), b"\x80\xc0\x80\x80\x80\x80\x80\x80\x00"),   # smallest 9-byte
        ((1 << 64) - 1, b"\xff\xff\xff\xff\xff\xff\xff\xff\xff"),  # largest 9-byte
    ],
)
def test_encode_matches_golden(value: int, raw: bytes) -> None:
    assert encode(value) == raw


@pytest.mark.parametrize(
    ("value", "raw"),
    [
        (0, b"\x00"),
        (1, b"\x01"),
        (127, b"\x7f"),
        (128, b"\x81\x00"),
        (300, b"\x82\x2c"),
        (16384, b"\x81\x80\x00"),
        ((1 << 56), b"\x80\xc0\x80\x80\x80\x80\x80\x80\x00"),
        ((1 << 64) - 1, b"\xff\xff\xff\xff\xff\xff\xff\xff\xff"),
    ],
)
def test_decode_matches_golden(value: int, raw: bytes) -> None:
    got, consumed = decode(raw)
    assert got == value
    assert consumed == len(raw)


# ------------------------------------------------------------------
# Round-trips across the full width distribution.
# ------------------------------------------------------------------


@pytest.mark.parametrize(
    "value",
    [
        0, 1, 7, 63, 127, 128, 255, 16383, 16384, 65535,
        (1 << 21) - 1, (1 << 21),
        (1 << 28) - 1, (1 << 28),
        (1 << 35) - 1, (1 << 35),
        (1 << 42) - 1, (1 << 42),
        (1 << 49) - 1, (1 << 49),
        (1 << 56) - 1, (1 << 56),
        (1 << 63) - 1, (1 << 63),
        (1 << 64) - 1,
    ],
)
def test_round_trip(value: int) -> None:
    raw = encode(value)
    got, consumed = decode(raw)
    assert got == value
    assert consumed == len(raw)
    assert size(value) == len(raw)


# ------------------------------------------------------------------
# Signed round-trip.
# ------------------------------------------------------------------


@pytest.mark.parametrize(
    "value",
    [
        -1,
        -(1 << 63),
        -123456789,
        -2,
        0,
        1,
        2,
        127,
        (1 << 63) - 1,
    ],
)
def test_signed_round_trip(value: int) -> None:
    raw = encode_signed(value)
    got, consumed = decode_signed(raw)
    assert got == value
    assert consumed == len(raw)


# ------------------------------------------------------------------
# Range checks.
# ------------------------------------------------------------------


def test_encode_rejects_negative() -> None:
    with pytest.raises(ValueError, match="out of range"):
        encode(-1)


def test_encode_rejects_too_large() -> None:
    with pytest.raises(ValueError, match="out of range"):
        encode(1 << 64)


def test_encode_signed_rejects_out_of_range() -> None:
    with pytest.raises(ValueError, match="signed varint out of range"):
        encode_signed(1 << 63)
    with pytest.raises(ValueError, match="signed varint out of range"):
        encode_signed(-(1 << 63) - 1)


def test_size_rejects_negative() -> None:
    with pytest.raises(ValueError):
        size(-1)


# ------------------------------------------------------------------
# Decode error paths.
# ------------------------------------------------------------------


def test_decode_empty_buffer() -> None:
    with pytest.raises(ValueError, match="empty"):
        decode(b"")


def test_decode_truncated_continuation() -> None:
    # Continuation bit set, no more bytes — must raise.
    with pytest.raises(ValueError, match="truncated"):
        decode(b"\x80")


def test_decode_truncated_nine_byte_form() -> None:
    # 8 bytes with continuation but no 9th byte.
    raw = b"\x80" * 8
    with pytest.raises(ValueError, match="before 9th byte"):
        decode(raw)


def test_decode_with_offset() -> None:
    prefix = b"\xaa\xbb"
    value_bytes = encode(300)
    got, consumed = decode(prefix + value_bytes, offset=2)
    assert got == 300
    assert consumed == len(value_bytes)


# ------------------------------------------------------------------
# size() consistency.
# ------------------------------------------------------------------


@pytest.mark.parametrize(
    "value",
    [0, 127, 128, 16383, 16384, (1 << 56) - 1, (1 << 56), (1 << 64) - 1],
)
def test_size_matches_encode(value: int) -> None:
    assert size(value) == len(encode(value))
