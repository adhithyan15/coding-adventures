"""Tests for the record codec (serial types + round-trip)."""

from __future__ import annotations

import math
import struct

import pytest

from storage_sqlite.errors import CorruptDatabaseError
from storage_sqlite.record import decode, encode
from storage_sqlite.varint import encode as varint_encode

# ------------------------------------------------------------------
# Empty record.
# ------------------------------------------------------------------


def test_empty_record() -> None:
    raw = encode([])
    assert raw == b"\x01"  # header-length varint = 1, no columns
    values, consumed = decode(raw)
    assert values == []
    assert consumed == 1


# ------------------------------------------------------------------
# NULLs.
# ------------------------------------------------------------------


def test_single_null() -> None:
    raw = encode([None])
    values, consumed = decode(raw)
    assert values == [None]
    assert consumed == len(raw)


def test_multiple_nulls() -> None:
    values, _ = decode(encode([None, None, None]))
    assert values == [None, None, None]


# ------------------------------------------------------------------
# Integer serial-type selection (byte-compat matters).
# ------------------------------------------------------------------


@pytest.mark.parametrize(
    ("value", "expected_serial_type", "payload_width"),
    [
        (0, 8, 0),
        (1, 9, 0),
        (2, 1, 1),
        (-1, 1, 1),
        (127, 1, 1),
        (-128, 1, 1),
        (128, 2, 2),
        (-129, 2, 2),
        (32767, 2, 2),
        (32768, 3, 3),
        (8_388_607, 3, 3),
        (8_388_608, 4, 4),
        (2_147_483_647, 4, 4),
        (2_147_483_648, 5, 6),
        ((1 << 47) - 1, 5, 6),
        (1 << 47, 6, 8),
        ((1 << 63) - 1, 6, 8),
        (-(1 << 63), 6, 8),
    ],
)
def test_int_serial_type_is_smallest_fit(
    value: int, expected_serial_type: int, payload_width: int
) -> None:
    raw = encode([value])
    # Header: 1 byte header-length varint + 1 byte serial type (all our
    # serial types here fit in a single-byte varint).
    assert raw[0] == 2  # header_len = 2
    assert raw[1] == expected_serial_type
    assert len(raw) == 2 + payload_width
    values, _ = decode(raw)
    assert values == [value]


def test_int_out_of_sqlite_range() -> None:
    with pytest.raises(ValueError, match="out of SQLite 64-bit range"):
        encode([1 << 63])


# ------------------------------------------------------------------
# Booleans collapse to 0/1 ints per SQLite convention.
# ------------------------------------------------------------------


def test_bool_true_encodes_as_int_1() -> None:
    raw = encode([True])
    # Serial type 9 = constant 1, zero payload bytes.
    assert raw == b"\x02\x09"
    values, _ = decode(raw)
    assert values == [1]


def test_bool_false_encodes_as_int_0() -> None:
    raw = encode([False])
    assert raw == b"\x02\x08"


# ------------------------------------------------------------------
# Floats.
# ------------------------------------------------------------------


@pytest.mark.parametrize("value", [0.0, 1.5, -3.14159, 1e100, -1e-300])
def test_float_round_trip(value: float) -> None:
    values, _ = decode(encode([value]))
    assert values == [value]


def test_float_nan_round_trips() -> None:
    values, _ = decode(encode([float("nan")]))
    assert math.isnan(values[0])


def test_float_inf_round_trips() -> None:
    values, _ = decode(encode([float("inf")]))
    assert values == [float("inf")]


# ------------------------------------------------------------------
# Strings (TEXT).
# ------------------------------------------------------------------


@pytest.mark.parametrize(
    "value",
    ["", "hello", "a" * 100, "αβγ", "🍉 watermelon"],
)
def test_text_round_trip(value: str) -> None:
    values, _ = decode(encode([value]))
    assert values == [value]


def test_text_serial_type_is_13_plus_2n() -> None:
    raw = encode(["abc"])
    # header_len = 2 (1-byte header varint + 1-byte serial type varint
    # since 13 + 2*3 = 19 fits in 1 byte). Serial type = 19.
    assert raw[0] == 2
    assert raw[1] == 19
    assert raw[2:] == b"abc"


# ------------------------------------------------------------------
# Blobs.
# ------------------------------------------------------------------


@pytest.mark.parametrize(
    "value",
    [b"", b"\x00\x01\x02", b"\xff" * 50],
)
def test_blob_round_trip(value: bytes) -> None:
    values, _ = decode(encode([value]))
    assert values == [value]


def test_blob_serial_type_is_12_plus_2n() -> None:
    raw = encode([b"xyz"])
    assert raw[0] == 2  # header_len
    assert raw[1] == 18  # 12 + 2*3
    assert raw[2:] == b"xyz"


def test_bytearray_accepted() -> None:
    raw = encode([bytearray(b"\x01\x02")])
    values, _ = decode(raw)
    assert values == [b"\x01\x02"]


def test_memoryview_accepted() -> None:
    raw = encode([memoryview(b"abc")])
    values, _ = decode(raw)
    assert values == [b"abc"]


# ------------------------------------------------------------------
# Mixed rows.
# ------------------------------------------------------------------


def test_mixed_row() -> None:
    row = [None, 42, -1, 3.14, "name", b"\x00\x01", True, 0]
    values, consumed = decode(encode(row))
    assert values == [None, 42, -1, 3.14, "name", b"\x00\x01", 1, 0]
    assert consumed == len(encode(row))


# ------------------------------------------------------------------
# Unsupported types.
# ------------------------------------------------------------------


def test_rejects_list_value() -> None:
    with pytest.raises(TypeError, match="cannot encode"):
        encode([["nested"]])


def test_rejects_dict_value() -> None:
    with pytest.raises(TypeError, match="cannot encode"):
        encode([{"key": "value"}])


# ------------------------------------------------------------------
# Large record (exercises multi-byte header-length varint).
# ------------------------------------------------------------------


def test_large_record_with_multibyte_header() -> None:
    # Enough columns that the header-length varint itself needs 2 bytes.
    # Each serial type here is 0 (NULL), 1 byte. We need header_len > 127.
    row = [None] * 200
    raw = encode(row)
    values, consumed = decode(raw)
    assert values == row
    assert consumed == len(raw)


def test_large_text_payload() -> None:
    value = "x" * 1000
    raw = encode([value])
    values, _ = decode(raw)
    assert values == [value]


# ------------------------------------------------------------------
# Decode error paths.
# ------------------------------------------------------------------


def test_decode_bad_header_length_varint() -> None:
    # Empty buffer — varint decode itself raises, wrapped as Corrupt.
    with pytest.raises(CorruptDatabaseError, match="header length"):
        decode(b"")


def test_decode_header_shorter_than_its_own_length_varint() -> None:
    # A header-length of 0 is impossible (it would lie about its own size).
    with pytest.raises(CorruptDatabaseError, match="shorter"):
        decode(b"\x00")


def test_decode_header_runs_past_buffer() -> None:
    # header_len=50 but buffer is tiny.
    with pytest.raises(CorruptDatabaseError, match="past buffer"):
        decode(b"\x32\x01")


def test_decode_reserved_serial_type_10() -> None:
    # header_len=2, serial type 10. Then try to decode — _payload_width
    # bails out on reserved types.
    raw = b"\x02\x0a"
    with pytest.raises(CorruptDatabaseError, match="reserved"):
        decode(raw)


def test_decode_reserved_serial_type_11() -> None:
    raw = b"\x02\x0b"
    with pytest.raises(CorruptDatabaseError, match="reserved"):
        decode(raw)


def test_decode_payload_truncated() -> None:
    # header_len=2, serial type 6 (int64 = 8 payload bytes), but no payload.
    raw = b"\x02\x06"
    with pytest.raises(CorruptDatabaseError, match="payload truncated"):
        decode(raw)


def test_decode_bad_utf8_in_text_column() -> None:
    # header_len=2, serial type 15 (TEXT length 1), payload is an invalid
    # UTF-8 continuation byte with no leader.
    raw = b"\x02\x0f\xff"
    with pytest.raises(CorruptDatabaseError, match="UTF-8"):
        decode(raw)


def test_decode_bad_serial_type_varint() -> None:
    # Claim header is 3 bytes, fill the 2-byte body with a truncated
    # varint (0x80 needs a continuation byte but the header ends there).
    raw = b"\x03\x80\x80"
    with pytest.raises(CorruptDatabaseError, match="serial type"):
        decode(raw)


def test_decode_offset() -> None:
    prefix = b"\xaa\xbb\xcc"
    raw = encode([42, "hi"])
    values, consumed = decode(prefix + raw, offset=len(prefix))
    assert values == [42, "hi"]
    assert consumed == len(raw)


# ------------------------------------------------------------------
# Golden on-wire format (pin down exact layout).
# ------------------------------------------------------------------


def test_golden_int_record() -> None:
    raw = encode([300])
    # header_len=2, serial type=2 (int16), payload=2 bytes big-endian.
    assert raw == b"\x02\x02" + (300).to_bytes(2, "big", signed=True)


def test_golden_float_record() -> None:
    raw = encode([1.25])
    assert raw == b"\x02\x07" + struct.pack(">d", 1.25)


def test_golden_null_int_text_record() -> None:
    raw = encode([None, 7, "hi"])
    # header_len = 1 (len varint) + 3 (three 1-byte serial types) = 4
    # serial types: 0 (NULL), 1 (int8), 17 (TEXT, 13+2*2)
    expected = b"\x04\x00\x01\x11" + b"\x07" + b"hi"
    assert raw == expected


# Not strictly needed for coverage but ensures the imports are exercised.
def test_varint_encode_integration() -> None:
    assert varint_encode(5) == b"\x05"
