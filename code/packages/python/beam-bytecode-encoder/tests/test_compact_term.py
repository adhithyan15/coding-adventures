"""Unit tests for ``encode_compact_term`` — BEAM's operand encoding.

These tests live separately from the module-level encode tests
because the bit-wrangling is fiddly enough that we want a dense
property-style suite covering all three length forms (small,
medium, large) and the boundary values between them.

Round-trip parity against ``beam-bytes-decoder._decode_compact_u``
lives in ``test_round_trip.py`` for the ``BEAMTag.U`` cases the
decoder handles.
"""

from __future__ import annotations

import pytest
from beam_bytecode_encoder import (
    BEAMEncodeError,
    BEAMTag,
    encode_compact_term,
)


class TestSmallForm:
    """The high-4-bit form for values 0..15."""

    def test_zero_atom(self) -> None:
        # tag=A (2), value=0 → first byte 0b00000010 = 0x02
        assert encode_compact_term(BEAMTag.A, 0) == bytes([0x02])

    def test_zero_x_register(self) -> None:
        # tag=X (3), value=0 → 0b00000011 = 0x03
        assert encode_compact_term(BEAMTag.X, 0) == bytes([0x03])

    def test_max_small(self) -> None:
        # value=15 fits in the high 4 bits.  tag=U → 0b11110000.
        assert encode_compact_term(BEAMTag.U, 15) == bytes([0xF0])

    @pytest.mark.parametrize("value", list(range(16)))
    def test_all_small_values_round_trip_via_first_byte(self, value: int) -> None:
        """Every value 0..15 fits in one byte under the small form."""
        encoded = encode_compact_term(BEAMTag.U, value)
        assert len(encoded) == 1
        assert (encoded[0] & 0b1000) == 0, "small form sets bit 3 to 0"
        assert (encoded[0] >> 4) == value


class TestMediumForm:
    """The 11-bit form for values 16..2047."""

    def test_just_over_small_threshold(self) -> None:
        # value=16 needs the medium form.  tag=U → first byte
        # has bits: high-3 of value (=0b000) shifted into the top 3
        # positions (0b00000000), bit 4 = 0 (medium), bit 3 = 1, low 3 bits = tag (U=0).
        # = 0b00001000 = 0x08; second byte = low 8 bits of 16 = 0x10.
        assert encode_compact_term(BEAMTag.U, 16) == bytes([0x08, 0x10])

    def test_max_medium(self) -> None:
        # value=2047 (= 0x7FF = 0b11111111111, 11 bits):
        # top 3 bits = 0b111, low 8 bits = 0xFF.
        # First byte = (0b111 << 5) | 0b1000 | tag(U=0) = 0xE8.
        assert encode_compact_term(BEAMTag.U, 2047) == bytes([0xE8, 0xFF])

    @pytest.mark.parametrize("value", [16, 100, 255, 256, 1023, 2047])
    def test_medium_form_is_two_bytes(self, value: int) -> None:
        encoded = encode_compact_term(BEAMTag.U, value)
        assert len(encoded) == 2
        # Bit 4 = 0, bit 3 = 1 ⇒ low nibble 0b1000 plus tag bits.
        assert (encoded[0] & 0b00010000) == 0
        assert (encoded[0] & 0b00001000) != 0


class TestLargeForm:
    """The variable-byte form for values >= 2048."""

    def test_just_over_medium_threshold(self) -> None:
        # value=2048 = 0x800, fits in 2 bytes (0x08 0x00).
        # length-2 = 0 → top 3 bits of first byte = 0.
        # First byte = 0b00011000 | tag_U(0) = 0x18.
        assert encode_compact_term(BEAMTag.U, 2048) == bytes([0x18, 0x08, 0x00])

    def test_three_byte_value(self) -> None:
        # value=0x10000 = 65536, needs 3 bytes; length-2 = 1.
        # First byte = (1 << 5) | 0b11000 | tag_U(0) = 0b00111000 = 0x38.
        assert encode_compact_term(BEAMTag.U, 0x10000) == bytes([0x38, 0x01, 0x00, 0x00])


class TestErrors:
    def test_negative_value_rejected(self) -> None:
        with pytest.raises(BEAMEncodeError, match="non-negative"):
            encode_compact_term(BEAMTag.U, -1)
