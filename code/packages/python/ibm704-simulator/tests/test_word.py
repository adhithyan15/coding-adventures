"""Tests for the word module — sign-magnitude arithmetic, FP, packing."""

from __future__ import annotations

import math

import pytest

from ibm704_simulator.word import (
    MAGNITUDE_MASK,
    SIGN_BIT,
    WORD_BYTES,
    WORD_MASK,
    add_sign_magnitude,
    float_to_fp,
    fp_to_float,
    make_word,
    negate_word,
    pack_program,
    pack_word,
    signed_int_to_word,
    unpack_program,
    unpack_word,
    word_magnitude,
    word_sign,
    word_to_signed_int,
)


# ---------------------------------------------------------------------------
# make_word / sign / magnitude
# ---------------------------------------------------------------------------


class TestMakeWord:
    def test_positive_zero(self) -> None:
        assert make_word(0, 0) == 0

    def test_negative_zero_distinct_from_positive_zero(self) -> None:
        # -0 has sign bit set but magnitude zero. Bit-distinct from +0.
        assert make_word(1, 0) == SIGN_BIT
        assert make_word(1, 0) != make_word(0, 0)

    def test_round_trip_through_sign_and_magnitude(self) -> None:
        for sign, mag in [(0, 1), (1, 1), (0, 12345), (1, MAGNITUDE_MASK)]:
            w = make_word(sign, mag)
            assert word_sign(w) == sign
            assert word_magnitude(w) == mag

    def test_invalid_sign_raises(self) -> None:
        with pytest.raises(ValueError):
            make_word(2, 0)

    def test_invalid_magnitude_raises(self) -> None:
        with pytest.raises(ValueError):
            make_word(0, MAGNITUDE_MASK + 1)
        with pytest.raises(ValueError):
            make_word(0, -1)


# ---------------------------------------------------------------------------
# Conversions to/from Python int
# ---------------------------------------------------------------------------


class TestSignedIntConversions:
    def test_positive_round_trip(self) -> None:
        for v in [0, 1, 100, MAGNITUDE_MASK]:
            assert word_to_signed_int(signed_int_to_word(v)) == v

    def test_negative_round_trip(self) -> None:
        for v in [-1, -100, -MAGNITUDE_MASK]:
            assert word_to_signed_int(signed_int_to_word(v)) == v

    def test_negative_zero_collapses_in_python_int(self) -> None:
        # -0 word → Python int 0
        assert word_to_signed_int(make_word(1, 0)) == 0

    def test_overflow_raises(self) -> None:
        with pytest.raises(OverflowError):
            signed_int_to_word(MAGNITUDE_MASK + 1)
        with pytest.raises(OverflowError):
            signed_int_to_word(-(MAGNITUDE_MASK + 1))


# ---------------------------------------------------------------------------
# Sign-magnitude addition
# ---------------------------------------------------------------------------


class TestAddSignMagnitude:
    def test_same_sign_no_overflow(self) -> None:
        # 3 + 4 = 7 (positive)
        assert add_sign_magnitude(0, 3, 0, 4) == (0, 7, False)
        # -3 + -4 = -7
        assert add_sign_magnitude(1, 3, 1, 4) == (1, 7, False)

    def test_different_signs_no_overflow(self) -> None:
        # 3 + -4 = -1
        assert add_sign_magnitude(0, 3, 1, 4) == (1, 1, False)
        # -3 + 4 = 1
        assert add_sign_magnitude(1, 3, 0, 4) == (0, 1, False)
        # 4 + -3 = 1
        assert add_sign_magnitude(0, 4, 1, 3) == (0, 1, False)

    def test_zero_canonicalization(self) -> None:
        # +3 + -3 produces +0, not -0.
        assert add_sign_magnitude(0, 3, 1, 3) == (0, 0, False)
        # -3 + 3 produces +0 too.
        assert add_sign_magnitude(1, 3, 0, 3) == (0, 0, False)
        # 0 + 0 stays +0.
        assert add_sign_magnitude(0, 0, 0, 0) == (0, 0, False)
        # -0 + -0 → +0 (canonical zero).
        assert add_sign_magnitude(1, 0, 1, 0) == (0, 0, False)

    def test_same_sign_overflow(self) -> None:
        sign, mag, ovf = add_sign_magnitude(0, MAGNITUDE_MASK, 0, 1)
        # MAGNITUDE_MASK + 1 = 2**35; low 35 bits = 0
        assert ovf is True
        assert sign == 0
        assert mag == 0

    def test_different_signs_never_overflow(self) -> None:
        # |a-b| <= max(|a|,|b|), so this cannot overflow.
        sign, mag, ovf = add_sign_magnitude(
            0, MAGNITUDE_MASK, 1, MAGNITUDE_MASK
        )
        assert ovf is False
        assert (sign, mag) == (0, 0)


# ---------------------------------------------------------------------------
# Negation
# ---------------------------------------------------------------------------


class TestNegateWord:
    def test_negate_positive(self) -> None:
        assert negate_word(make_word(0, 5)) == make_word(1, 5)

    def test_negate_negative(self) -> None:
        assert negate_word(make_word(1, 5)) == make_word(0, 5)

    def test_negate_zero_yields_negative_zero(self) -> None:
        # On 704, -(+0) is bit-distinct from +0.
        assert negate_word(0) == SIGN_BIT


# ---------------------------------------------------------------------------
# Floating-point round trips
# ---------------------------------------------------------------------------


class TestFloatingPoint:
    @pytest.mark.parametrize("value", [
        0.0, 1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 1.5, -3.25, 100.0, -100.0,
    ])
    def test_clean_round_trip(self, value: float) -> None:
        encoded = float_to_fp(value)
        decoded = fp_to_float(encoded)
        assert decoded == value, f"round-trip failed for {value}"

    def test_zero_encodes_to_zero_word(self) -> None:
        assert float_to_fp(0.0) == 0
        assert fp_to_float(0) == 0.0

    def test_nan_and_inf_collapse_to_zero(self) -> None:
        # The 704 has no NaN/Inf representation; we collapse to +0.
        assert float_to_fp(float("nan")) == 0
        assert float_to_fp(float("inf")) == 0
        assert float_to_fp(float("-inf")) == 0

    def test_negative_round_trip_preserves_sign(self) -> None:
        for v in [-1.0, -0.25, -100.5]:
            decoded = fp_to_float(float_to_fp(v))
            assert decoded == v
            assert math.copysign(1.0, decoded) == -1.0

    def test_overflow_saturates(self) -> None:
        # 2^200 is way past characteristic range (max char = 255 → exp 127).
        encoded = float_to_fp(2.0 ** 200)
        # Expect saturation (max representable magnitude).
        assert encoded != 0
        decoded = fp_to_float(encoded)
        assert decoded > 0  # magnitude preserved at maximum

    def test_underflow_collapses_to_zero(self) -> None:
        encoded = float_to_fp(2.0 ** -200)
        assert encoded == 0


# ---------------------------------------------------------------------------
# Word packing
# ---------------------------------------------------------------------------


class TestPacking:
    def test_pack_zero(self) -> None:
        assert pack_word(0) == bytes(WORD_BYTES)

    def test_pack_all_ones(self) -> None:
        # All 36 bits set → low 36 bits of 5-byte big-endian = 0x0F_FF_FF_FF_FF
        assert pack_word(WORD_MASK) == bytes.fromhex("0fffffffff")

    def test_pack_unpack_round_trip(self) -> None:
        for w in [0, 1, 0x123456789, WORD_MASK, SIGN_BIT, MAGNITUDE_MASK]:
            assert unpack_word(pack_word(w)) == w

    def test_pack_rejects_oversize(self) -> None:
        with pytest.raises(ValueError):
            pack_word(WORD_MASK + 1)

    def test_unpack_rejects_wrong_length(self) -> None:
        with pytest.raises(ValueError):
            unpack_word(b"\x00" * 4)
        with pytest.raises(ValueError):
            unpack_word(b"\x00" * 6)

    def test_unpack_rejects_high_bits_set(self) -> None:
        # If any of the top 4 bits are set, the word doesn't fit in 36 bits.
        with pytest.raises(ValueError):
            unpack_word(bytes.fromhex("F000000000"))

    def test_pack_program_round_trip(self) -> None:
        words = [0, 1, 0x123, MAGNITUDE_MASK, WORD_MASK]
        packed = pack_program(words)
        assert len(packed) == len(words) * WORD_BYTES
        assert unpack_program(packed) == words

    def test_unpack_rejects_misaligned_byte_stream(self) -> None:
        with pytest.raises(ValueError):
            unpack_program(b"\x00" * 7)
