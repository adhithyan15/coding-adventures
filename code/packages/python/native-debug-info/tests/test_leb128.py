"""Tests for ULEB128 and SLEB128 encoding/decoding."""

import pytest

from native_debug_info.leb128 import (
    decode_sleb128,
    decode_uleb128,
    encode_sleb128,
    encode_uleb128,
)


class TestUleb128Encode:
    def test_zero(self):
        assert encode_uleb128(0) == b"\x00"

    def test_one(self):
        assert encode_uleb128(1) == b"\x01"

    def test_127(self):
        assert encode_uleb128(127) == b"\x7f"

    def test_128_uses_two_bytes(self):
        assert encode_uleb128(128) == b"\x80\x01"

    def test_300(self):
        # 300 = 0x12c → [0xac, 0x02]
        assert encode_uleb128(300) == b"\xac\x02"

    def test_624485(self):
        # DWARF spec example: 624485 → e5 8e 26
        assert encode_uleb128(624485) == b"\xe5\x8e\x26"

    def test_negative_raises(self):
        with pytest.raises(ValueError):
            encode_uleb128(-1)

    def test_large_value(self):
        result = encode_uleb128(0xFFFFFFFF)
        assert len(result) == 5  # needs 5 bytes for 32-bit value

    def test_very_large(self):
        # 64-bit max
        result = encode_uleb128(0xFFFFFFFFFFFFFFFF)
        assert len(result) == 10


class TestUleb128Decode:
    def test_zero(self):
        val, n = decode_uleb128(b"\x00")
        assert val == 0 and n == 1

    def test_one(self):
        val, n = decode_uleb128(b"\x01")
        assert val == 1 and n == 1

    def test_127(self):
        val, n = decode_uleb128(b"\x7f")
        assert val == 127 and n == 1

    def test_128(self):
        val, n = decode_uleb128(b"\x80\x01")
        assert val == 128 and n == 2

    def test_624485(self):
        val, n = decode_uleb128(b"\xe5\x8e\x26")
        assert val == 624485 and n == 3

    def test_offset(self):
        data = b"\xff\x00\x80\x01"
        val, n = decode_uleb128(data, offset=2)
        assert val == 128 and n == 2

    def test_round_trip(self):
        for v in [0, 1, 127, 128, 300, 624485, 2**32, 2**64 - 1]:
            encoded = encode_uleb128(v)
            decoded, consumed = decode_uleb128(encoded)
            assert decoded == v
            assert consumed == len(encoded)


class TestSleb128Encode:
    def test_zero(self):
        assert encode_sleb128(0) == b"\x00"

    def test_one(self):
        assert encode_sleb128(1) == b"\x01"

    def test_minus_one(self):
        assert encode_sleb128(-1) == b"\x7f"

    def test_63(self):
        assert encode_sleb128(63) == b"\x3f"

    def test_64_uses_two_bytes(self):
        assert encode_sleb128(64) == b"\xc0\x00"

    def test_minus_64(self):
        assert encode_sleb128(-64) == b"\x40"

    def test_minus_65_uses_two_bytes(self):
        result = encode_sleb128(-65)
        assert len(result) == 2

    def test_minus_123456(self):
        # DWARF spec example: -123456 → c0 bb 78
        assert encode_sleb128(-123456) == b"\xc0\xbb\x78"

    def test_large_positive(self):
        result = encode_sleb128(2**31 - 1)
        assert len(result) == 5

    def test_large_negative(self):
        result = encode_sleb128(-(2**31))
        assert len(result) == 5


class TestSleb128Decode:
    def test_zero(self):
        val, n = decode_sleb128(b"\x00")
        assert val == 0 and n == 1

    def test_one(self):
        val, n = decode_sleb128(b"\x01")
        assert val == 1 and n == 1

    def test_minus_one(self):
        val, n = decode_sleb128(b"\x7f")
        assert val == -1 and n == 1

    def test_minus_123456(self):
        val, n = decode_sleb128(b"\xc0\xbb\x78")
        assert val == -123456 and n == 3

    def test_round_trip(self):
        for v in [0, 1, -1, 63, -64, 64, -65, 123456, -123456, 2**30, -(2**30)]:
            encoded = encode_sleb128(v)
            decoded, consumed = decode_sleb128(encoded)
            assert decoded == v
            assert consumed == len(encoded)
