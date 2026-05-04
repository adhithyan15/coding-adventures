"""Tests for MOS 6502 flag computation helpers."""

from __future__ import annotations

from mos6502_simulator import (
    bcd_add,
    bcd_sub,
    compute_nz,
    compute_overflow_add,
    compute_overflow_sub,
    pack_p,
    unpack_p,
)


class TestComputeNZ:
    def test_zero(self) -> None:
        n, z = compute_nz(0x00)
        assert n is False
        assert z is True

    def test_negative(self) -> None:
        n, z = compute_nz(0xFF)
        assert n is True
        assert z is False

    def test_positive(self) -> None:
        n, z = compute_nz(0x42)
        assert n is False
        assert z is False

    def test_bit7_set(self) -> None:
        n, z = compute_nz(0x80)
        assert n is True
        assert z is False

    def test_masks_to_8_bit(self) -> None:
        n, z = compute_nz(0x100)  # bit 8 set — should be masked to 0
        assert n is False
        assert z is True


class TestOverflow:
    def test_add_positive_overflow(self) -> None:
        # 0x7F + 0x01 = 0x80: two positives → negative result → overflow
        assert compute_overflow_add(0x7F, 0x01, 0x80) is True

    def test_add_negative_overflow(self) -> None:
        # 0x80 + 0xFF = 0x7F: two negatives → positive result → overflow
        assert compute_overflow_add(0x80, 0xFF, 0x7F) is True

    def test_add_no_overflow(self) -> None:
        # 0x50 + 0x10 = 0x60: both positive, result positive
        assert compute_overflow_add(0x50, 0x10, 0x60) is False

    def test_add_different_signs_no_overflow(self) -> None:
        # 0x7F + 0x80 = 0xFF: different signs can never overflow
        assert compute_overflow_add(0x7F, 0x80, 0xFF) is False

    def test_sub_overflow(self) -> None:
        # 0x80 - 0x01 = 0x7F: -128 - 1 = +127 → overflow
        # SBC: A=0x80, M=0x01, result=0x7F
        assert compute_overflow_sub(0x80, 0x01, 0x7F) is True

    def test_sub_no_overflow(self) -> None:
        # 0x50 - 0x30 = 0x20: no overflow
        assert compute_overflow_sub(0x50, 0x30, 0x20) is False


class TestPackUnpack:
    def test_pack_unpack_roundtrip(self) -> None:
        # I=1, Z=1 → P = 0x26
        p = pack_p(False, False, False, False, True, True, False)
        n, v, b, d, i, z, c = unpack_p(p)
        assert n is False
        assert v is False
        assert b is False
        assert d is False
        assert i is True
        assert z is True
        assert c is False

    def test_bit5_always_set(self) -> None:
        p = pack_p(False, False, False, False, False, False, False)
        assert p & 0x20   # bit 5 always 1

    def test_all_set(self) -> None:
        p = pack_p(True, True, True, True, True, True, True)
        assert p == 0xFF

    def test_reset_p(self) -> None:
        # Power-on P = 0x24 (I=1, bit5=1)
        p = pack_p(False, False, False, False, True, False, False)
        assert p == 0x24


class TestBCD:
    def test_bcd_add_simple(self) -> None:
        result, c = bcd_add(0x09, 0x01, False)
        assert result == 0x10
        assert c is False

    def test_bcd_add_carry(self) -> None:
        result, c = bcd_add(0x99, 0x01, False)
        assert result == 0x00
        assert c is True

    def test_bcd_add_mid_digit(self) -> None:
        result, c = bcd_add(0x05, 0x05, False)
        assert result == 0x10
        assert c is False

    def test_bcd_add_with_carry_in(self) -> None:
        result, c = bcd_add(0x09, 0x00, True)
        assert result == 0x10
        assert c is False

    def test_bcd_sub_simple(self) -> None:
        result, c = bcd_sub(0x10, 0x01, True)
        assert result == 0x09
        assert c is True   # no borrow

    def test_bcd_sub_borrow(self) -> None:
        result, c = bcd_sub(0x00, 0x01, True)
        assert result == 0x99
        assert c is False  # borrow occurred

    def test_bcd_sub_99_minus_99(self) -> None:
        result, c = bcd_sub(0x99, 0x99, True)
        assert result == 0x00
        assert c is True
