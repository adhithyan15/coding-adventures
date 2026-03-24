"""Tests for wasm-leb128.

These tests cover all 11 required cases:
  1.  Zero
  2.  One-byte unsigned
  3.  One-byte signed negative
  4.  Multi-byte unsigned (624485)
  5.  Max u32
  6.  Max i32
  7.  Min i32
  8.  Round-trip encode/decode
  9.  Unterminated input → LEB128Error
  10. Non-zero offset
  11. Encode/decode of negative values (signed)
"""

from __future__ import annotations

import pytest

from wasm_leb128 import (
    LEB128Error,
    __version__,
    decode_signed,
    decode_unsigned,
    encode_signed,
    encode_unsigned,
)


# ---------------------------------------------------------------------------
# Version sanity
# ---------------------------------------------------------------------------


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


# ---------------------------------------------------------------------------
# Test case 1 — Zero
# ---------------------------------------------------------------------------


class TestZero:
    """LEB128 encoding of zero is a single 0x00 byte."""

    def test_decode_unsigned_zero(self) -> None:
        value, consumed = decode_unsigned(bytes([0x00]))
        assert value == 0
        assert consumed == 1

    def test_decode_signed_zero(self) -> None:
        value, consumed = decode_signed(bytes([0x00]))
        assert value == 0
        assert consumed == 1

    def test_encode_unsigned_zero(self) -> None:
        assert encode_unsigned(0) == bytes([0x00])

    def test_encode_signed_zero(self) -> None:
        assert encode_signed(0) == bytes([0x00])


# ---------------------------------------------------------------------------
# Test case 2 — One-byte unsigned (value = 3)
# ---------------------------------------------------------------------------


class TestOneByteUnsigned:
    """Single-byte values 1–127 encode without a continuation bit."""

    def test_decode_value_3(self) -> None:
        value, consumed = decode_unsigned(bytes([0x03]))
        assert value == 3
        assert consumed == 1

    def test_decode_value_127(self) -> None:
        # 0x7F is the largest one-byte unsigned LEB128 value.
        value, consumed = decode_unsigned(bytes([0x7F]))
        assert value == 127
        assert consumed == 1

    def test_decode_value_1(self) -> None:
        value, consumed = decode_unsigned(bytes([0x01]))
        assert value == 1
        assert consumed == 1

    def test_encode_value_3(self) -> None:
        assert encode_unsigned(3) == bytes([0x03])

    def test_encode_value_127(self) -> None:
        assert encode_unsigned(127) == bytes([0x7F])


# ---------------------------------------------------------------------------
# Test case 3 — One-byte signed negative (0x7E → -2)
# ---------------------------------------------------------------------------


class TestOneByteSignedNegative:
    """Negative values whose magnitude fits in 7 bits encode as one byte.

    0x7E = 0b0_1111110
               ^       MSB=0 → last byte
                ^^^^^^^ bits 6–0 = 1111110 → sign-extended = -2
    """

    def test_decode_minus_two(self) -> None:
        value, consumed = decode_signed(bytes([0x7E]))
        assert value == -2
        assert consumed == 1

    def test_decode_minus_one(self) -> None:
        # 0x7F = 0b01111111; bit 6 = 1 → sign extend → -1
        value, consumed = decode_signed(bytes([0x7F]))
        assert value == -1
        assert consumed == 1

    def test_decode_minus_64(self) -> None:
        # 0x40 = 0b01000000; bit 6 = 1 → sign extend → -64
        value, consumed = decode_signed(bytes([0x40]))
        assert value == -64
        assert consumed == 1

    def test_encode_minus_two(self) -> None:
        assert encode_signed(-2) == bytes([0x7E])

    def test_encode_minus_one(self) -> None:
        assert encode_signed(-1) == bytes([0x7F])


# ---------------------------------------------------------------------------
# Test case 4 — Multi-byte: [0xE5, 0x88, 0x26] → 624485
# ---------------------------------------------------------------------------


class TestMultiByte:
    """Three-byte LEB128 encoding of 624485.

    Derivation:
        624485 = 0x98465
        Group 1 (bits 0–6):  0x65 → 0x65 | 0x80 = 0xE5
        Group 2 (bits 7–13): 0x0E → 0x0E | 0x80 = 0x8E
        Group 3 (bits 14–20):0x26 → 0x26 (last byte, no continuation bit)

    Note: The WASM spec example uses [0xE5, 0x8E, 0x26] for 624485.
    """

    DATA = bytes([0xE5, 0x8E, 0x26])

    def test_decode_unsigned(self) -> None:
        value, consumed = decode_unsigned(self.DATA)
        assert value == 624485
        assert consumed == 3

    def test_decode_signed(self) -> None:
        # 624485 is positive; signed and unsigned agree.
        value, consumed = decode_signed(self.DATA)
        assert value == 624485
        assert consumed == 3

    def test_two_byte_value(self) -> None:
        # 128 requires 2 bytes: [0x80, 0x01]
        #   0x80 = 1_0000000 (continuation, payload=0)
        #   0x01 = 0_0000001 (last, payload=1)
        #   value = 0 | (1 << 7) = 128
        value, consumed = decode_unsigned(bytes([0x80, 0x01]))
        assert value == 128
        assert consumed == 2

    def test_encode_two_byte_value(self) -> None:
        encoded = encode_unsigned(128)
        assert encoded == bytes([0x80, 0x01])


# ---------------------------------------------------------------------------
# Test case 5 — Max u32: 4294967295
# ---------------------------------------------------------------------------


class TestMaxU32:
    """Maximum 32-bit unsigned integer = 2^32 - 1 = 0xFFFFFFFF.

    Encoding:
        0xFFFFFFFF in binary: 11111111_11111111_11111111_11111111

        7-bit groups (LSB first):
            1111111  →  0x7F | 0x80 = 0xFF
            1111111  →  0x7F | 0x80 = 0xFF
            1111111  →  0x7F | 0x80 = 0xFF
            1111111  →  0x7F | 0x80 = 0xFF
            0001111  →  0x0F (last byte — only 4 bits remain)

        Result: [0xFF, 0xFF, 0xFF, 0xFF, 0x0F]
    """

    DATA = bytes([0xFF, 0xFF, 0xFF, 0xFF, 0x0F])
    VALUE = 4294967295

    def test_decode_unsigned(self) -> None:
        value, consumed = decode_unsigned(self.DATA)
        assert value == self.VALUE
        assert consumed == 5

    def test_encode_unsigned(self) -> None:
        assert encode_unsigned(self.VALUE) == self.DATA

    def test_round_trip(self) -> None:
        encoded = encode_unsigned(self.VALUE)
        decoded, _ = decode_unsigned(encoded)
        assert decoded == self.VALUE


# ---------------------------------------------------------------------------
# Test case 6 — Max i32: 2147483647
# ---------------------------------------------------------------------------


class TestMaxI32:
    """Maximum signed 32-bit integer = 2^31 - 1 = 0x7FFFFFFF.

    Encoding:
        0x7FFFFFFF = 01111111_11111111_11111111_11111111

        7-bit groups (LSB first):
            1111111  →  0x7F | 0x80 = 0xFF
            1111111  →  0x7F | 0x80 = 0xFF
            1111111  →  0x7F | 0x80 = 0xFF
            1111111  →  0x7F | 0x80 = 0xFF
            0000111  →  0x07 (last byte, bit 6 = 0 → positive sign ✓)

        Result: [0xFF, 0xFF, 0xFF, 0xFF, 0x07]
    """

    DATA = bytes([0xFF, 0xFF, 0xFF, 0xFF, 0x07])
    VALUE = 2147483647

    def test_decode_signed(self) -> None:
        value, consumed = decode_signed(self.DATA)
        assert value == self.VALUE
        assert consumed == 5

    def test_encode_signed(self) -> None:
        assert encode_signed(self.VALUE) == self.DATA

    def test_round_trip(self) -> None:
        encoded = encode_signed(self.VALUE)
        decoded, _ = decode_signed(encoded)
        assert decoded == self.VALUE


# ---------------------------------------------------------------------------
# Test case 7 — Min i32: -2147483648
# ---------------------------------------------------------------------------


class TestMinI32:
    """Minimum signed 32-bit integer = -2^31 = 0x80000000 (two's complement).

    Encoding:
        -2147483648 in binary (infinite precision, two's complement):
            ...11111111_10000000_00000000_00000000_00000000

        7-bit groups (LSB first):
            0000000  →  0x00 | 0x80 = 0x80  (continuation)
            0000000  →  0x00 | 0x80 = 0x80  (continuation)
            0000000  →  0x00 | 0x80 = 0x80  (continuation)
            0000000  →  0x00 | 0x80 = 0x80  (continuation)
            1111000  →  0x78  (last byte; bit 6 = 1 → negative sign ✓)

        Result: [0x80, 0x80, 0x80, 0x80, 0x78]
    """

    DATA = bytes([0x80, 0x80, 0x80, 0x80, 0x78])
    VALUE = -2147483648

    def test_decode_signed(self) -> None:
        value, consumed = decode_signed(self.DATA)
        assert value == self.VALUE
        assert consumed == 5

    def test_encode_signed(self) -> None:
        assert encode_signed(self.VALUE) == self.DATA

    def test_round_trip(self) -> None:
        encoded = encode_signed(self.VALUE)
        decoded, _ = decode_signed(encoded)
        assert decoded == self.VALUE


# ---------------------------------------------------------------------------
# Test case 8 — Round-trip: encode then decode returns original value
# ---------------------------------------------------------------------------


class TestRoundTrip:
    """Encoding and then decoding must return the original value.

    We test a range of values including edge cases, small values, large
    values, and negative values.
    """

    UNSIGNED_VALUES = [
        0,
        1,
        63,
        64,
        127,
        128,
        255,
        256,
        16383,
        16384,
        2097151,
        268435455,
        4294967295,  # max u32
    ]

    SIGNED_VALUES = [
        0,
        1,
        -1,
        63,
        64,
        -64,
        -65,
        127,
        -128,
        2147483647,   # max i32
        -2147483648,  # min i32
        100,
        -100,
        1000,
        -1000,
    ]

    @pytest.mark.parametrize("value", UNSIGNED_VALUES)
    def test_unsigned_round_trip(self, value: int) -> None:
        encoded = encode_unsigned(value)
        decoded, consumed = decode_unsigned(encoded)
        assert decoded == value, f"Round-trip failed for {value}: encoded={list(encoded)}"
        assert consumed == len(encoded)

    @pytest.mark.parametrize("value", SIGNED_VALUES)
    def test_signed_round_trip(self, value: int) -> None:
        encoded = encode_signed(value)
        decoded, consumed = decode_signed(encoded)
        assert decoded == value, f"Round-trip failed for {value}: encoded={list(encoded)}"
        assert consumed == len(encoded)


# ---------------------------------------------------------------------------
# Test case 9 — Unterminated: [0x80, 0x80] → LEB128Error
# ---------------------------------------------------------------------------


class TestUnterminated:
    """An unterminated LEB128 sequence must raise LEB128Error.

    A sequence is unterminated when every byte has the continuation bit
    (bit 7) set, but we run out of bytes before finding a terminating byte.

    [0x80, 0x80] means:
        byte 0: 0x80 = 1_0000000  — continuation bit set, payload = 0
        byte 1: 0x80 = 1_0000000  — continuation bit set, payload = 0
        (no more bytes) → error
    """

    def test_unterminated_two_bytes(self) -> None:
        with pytest.raises(LEB128Error) as exc_info:
            decode_unsigned(bytes([0x80, 0x80]))
        err = exc_info.value
        assert err.offset == 0
        assert "unterminated" in err.message.lower()

    def test_unterminated_one_byte(self) -> None:
        # A single byte with continuation bit set but nothing following.
        with pytest.raises(LEB128Error) as exc_info:
            decode_unsigned(bytes([0x80]))
        assert exc_info.value.offset == 0

    def test_unterminated_empty(self) -> None:
        # Empty input at offset 0 → error immediately.
        with pytest.raises(LEB128Error):
            decode_unsigned(bytes([]))

    def test_unterminated_signed_two_bytes(self) -> None:
        with pytest.raises(LEB128Error) as exc_info:
            decode_signed(bytes([0x80, 0x80]))
        assert exc_info.value.offset == 0

    def test_unterminated_at_nonzero_offset(self) -> None:
        # Extra good byte at offset 0, then unterminated at offset 1.
        with pytest.raises(LEB128Error) as exc_info:
            decode_unsigned(bytes([0x01, 0x80, 0x80]), offset=1)
        # The error offset should be where decoding started (offset=1).
        assert exc_info.value.offset == 1

    def test_leb128_error_attributes(self) -> None:
        """LEB128Error must have both message and offset attributes."""
        try:
            decode_unsigned(bytes([0x80, 0x80]))
            pytest.fail("Expected LEB128Error was not raised")
        except LEB128Error as e:
            assert isinstance(e.message, str)
            assert isinstance(e.offset, int)
            assert len(e.message) > 0

    def test_leb128_error_is_exception(self) -> None:
        """LEB128Error must be catchable as a plain Exception."""
        with pytest.raises(Exception):  # noqa: B017
            decode_unsigned(bytes([0x80, 0x80]))


# ---------------------------------------------------------------------------
# Test case 10 — Non-zero offset
# ---------------------------------------------------------------------------


class TestNonZeroOffset:
    """decode_* functions accept an offset parameter to start reading mid-buffer.

    This allows parsing LEB128 values out of a larger byte stream without
    copying sub-slices.
    """

    def test_unsigned_at_offset_1(self) -> None:
        # Prefix garbage byte, then [0xE5, 0x8E, 0x26] = 624485 at offset 1.
        data = bytes([0xFF, 0xE5, 0x8E, 0x26])
        value, consumed = decode_unsigned(data, offset=1)
        assert value == 624485
        assert consumed == 3

    def test_unsigned_at_offset_2(self) -> None:
        data = bytes([0x00, 0x00, 0x03])
        value, consumed = decode_unsigned(data, offset=2)
        assert value == 3
        assert consumed == 1

    def test_signed_at_offset_1(self) -> None:
        data = bytes([0x00, 0x7E])  # 0x7E = -2
        value, consumed = decode_signed(data, offset=1)
        assert value == -2
        assert consumed == 1

    def test_offset_zero_same_as_default(self) -> None:
        data = bytes([0x03])
        v1, c1 = decode_unsigned(data)
        v2, c2 = decode_unsigned(data, offset=0)
        assert v1 == v2
        assert c1 == c2

    def test_offset_past_end_raises(self) -> None:
        data = bytes([0x03])
        with pytest.raises(LEB128Error):
            decode_unsigned(data, offset=5)

    def test_multi_byte_at_offset(self) -> None:
        # [0x80, 0x01] = 128, at offset 3 in a larger buffer.
        data = bytes([0x00, 0x00, 0x00, 0x80, 0x01, 0x00])
        value, consumed = decode_unsigned(data, offset=3)
        assert value == 128
        assert consumed == 2


# ---------------------------------------------------------------------------
# Test case 11 — Encode/decode of negative values (signed)
# ---------------------------------------------------------------------------


class TestSignedNegativeValues:
    """Signed encoding of negative values must survive round-trips and
    produce the expected byte sequences.

    Key cases:
        -1    → [0x7F]      (one byte; bit 6 = 1 → sign = negative ✓)
        -2    → [0x7E]
        -64   → [0x40]      (boundary: smallest one-byte negative)
        -65   → [0xBF, 0x7F] (just past one-byte range)
        -128  → [0x80, 0x7F]
    """

    @pytest.mark.parametrize(
        ("value", "expected"),
        [
            (-1, [0x7F]),
            (-2, [0x7E]),
            (-64, [0x40]),
            (-65, [0xBF, 0x7F]),
            (-128, [0x80, 0x7F]),
            (-129, [0xFF, 0x7E]),
        ],
    )
    def test_encode_specific_negatives(self, value: int, expected: list[int]) -> None:
        assert list(encode_signed(value)) == expected

    @pytest.mark.parametrize(
        ("encoded", "expected_value"),
        [
            ([0x7F], -1),
            ([0x7E], -2),
            ([0x40], -64),
            ([0xBF, 0x7F], -65),
            ([0x80, 0x7F], -128),
        ],
    )
    def test_decode_specific_negatives(
        self, encoded: list[int], expected_value: int
    ) -> None:
        value, _ = decode_signed(bytes(encoded))
        assert value == expected_value

    def test_positive_64_needs_two_bytes(self) -> None:
        # 64 = 0x40 in one byte — but bit 6 is set, so the decoder would
        # sign-extend and return -64. Therefore, +64 requires TWO bytes.
        encoded = encode_signed(64)
        assert len(encoded) == 2
        assert list(encoded) == [0xC0, 0x00]
        decoded, _ = decode_signed(encoded)
        assert decoded == 64

    def test_encode_unsigned_rejects_negative(self) -> None:
        """encode_unsigned must reject negative values with ValueError."""
        with pytest.raises(ValueError, match="non-negative"):
            encode_unsigned(-1)

    @pytest.mark.parametrize("value", [-1, -2, -64, -128, -2147483648])
    def test_signed_round_trip_negatives(self, value: int) -> None:
        encoded = encode_signed(value)
        decoded, consumed = decode_signed(encoded)
        assert decoded == value
        assert consumed == len(encoded)


# ---------------------------------------------------------------------------
# Additional edge-case tests for coverage completeness
# ---------------------------------------------------------------------------


class TestEdgeCases:
    """Miscellaneous edge cases to push coverage above 95%."""

    def test_decode_unsigned_bytearray(self) -> None:
        """decode_unsigned should accept bytearray as well as bytes."""
        value, consumed = decode_unsigned(bytearray([0x03]))
        assert value == 3
        assert consumed == 1

    def test_decode_signed_bytearray(self) -> None:
        value, consumed = decode_signed(bytearray([0x7E]))
        assert value == -2
        assert consumed == 1

    def test_encode_decode_large_sequence(self) -> None:
        """Encoding multiple values into a single buffer and decoding them."""
        buf = bytearray()
        values = [0, 1, 127, 128, 255, 256, 624485, 4294967295]
        for v in values:
            buf.extend(encode_unsigned(v))

        offset = 0
        for expected in values:
            val, consumed = decode_unsigned(buf, offset=offset)
            assert val == expected
            offset += consumed

    def test_leb128_error_str_representation(self) -> None:
        """str(LEB128Error) should return the message."""
        err = LEB128Error("test error", 5)
        assert str(err) == "test error"
        assert err.message == "test error"
        assert err.offset == 5

    def test_decode_value_128(self) -> None:
        """128 = 0x8000 in LEB128: [0x80, 0x01]."""
        value, consumed = decode_unsigned(bytes([0x80, 0x01]))
        assert value == 128
        assert consumed == 2

    def test_encode_decode_signed_positive_values(self) -> None:
        """Positive values should encode/decode identically through signed API."""
        for v in [0, 1, 63, 127, 128, 300, 16383, 2147483647]:
            encoded = encode_signed(v)
            decoded, _ = decode_signed(encoded)
            assert decoded == v, f"Failed for {v}"

    def test_max_unsigned_five_bytes(self) -> None:
        """4294967295 must encode to exactly 5 bytes."""
        encoded = encode_unsigned(4294967295)
        assert len(encoded) == 5

    def test_data_after_leb128_is_ignored(self) -> None:
        """Bytes after a complete LEB128 value are not consumed."""
        # [0x03, 0xFF, 0xFF] — value is 3, trailing bytes are garbage.
        value, consumed = decode_unsigned(bytes([0x03, 0xFF, 0xFF]))
        assert value == 3
        assert consumed == 1  # only 1 byte consumed
