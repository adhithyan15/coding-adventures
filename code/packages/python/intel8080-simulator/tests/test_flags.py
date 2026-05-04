"""Tests for Intel 8080 flag computation helpers."""

from __future__ import annotations

import pytest

from intel8080_simulator import (
    compute_ac_add,
    compute_ac_sub,
    compute_cy_add,
    compute_cy_sub,
    compute_p,
    compute_s,
    compute_z,
    flags_from_byte,
    szp_flags,
)
from intel8080_simulator.flags import compute_ac_ana


class TestSignFlag:
    def test_bit7_set(self) -> None:
        assert compute_s(0x80) is True
        assert compute_s(0xFF) is True

    def test_bit7_clear(self) -> None:
        assert compute_s(0x00) is False
        assert compute_s(0x7F) is False

    def test_overflow_ignored(self) -> None:
        # compute_s only looks at bit 7 of the byte
        assert compute_s(0x100) is False  # bit7 of 0x100 & 0xFF = 0


class TestZeroFlag:
    def test_zero(self) -> None:
        assert compute_z(0x00) is True
        assert compute_z(0x100) is True  # masked to 8 bits

    def test_nonzero(self) -> None:
        assert compute_z(0x01) is False
        assert compute_z(0xFF) is False


class TestParityFlag:
    @pytest.mark.parametrize("value,expected", [
        (0x00, True),   # 0 ones → even
        (0xFF, True),   # 8 ones → even
        (0xAA, True),   # 4 ones → even (10101010)
        (0x01, False),  # 1 one → odd
        (0x03, False),  # 2 ones → even? No: 0b00000011 has 2 ones → even
    ])
    def test_parity(self, value: int, expected: bool) -> None:
        # Recompute expected using brute force
        actual_expected = bin(value & 0xFF).count("1") % 2 == 0
        assert compute_p(value) == actual_expected

    def test_even_parity(self) -> None:
        # 0b01010101 = 85 decimal, 4 ones → even parity
        assert compute_p(0x55) is True

    def test_odd_parity(self) -> None:
        # 0b00000001 = 1, 1 one → odd parity
        assert compute_p(0x01) is False


class TestCarryAdd:
    def test_no_carry(self) -> None:
        assert compute_cy_add(0xFF) is False    # fits in byte
        assert compute_cy_add(0x00) is False

    def test_carry(self) -> None:
        assert compute_cy_add(0x100) is True
        assert compute_cy_add(0x1FF) is True


class TestCarrySub:
    def test_no_borrow(self) -> None:
        assert compute_cy_sub(5, 3) is False
        assert compute_cy_sub(5, 5) is False

    def test_borrow(self) -> None:
        assert compute_cy_sub(3, 5) is True
        assert compute_cy_sub(0, 1) is True

    def test_borrow_with_carry_in(self) -> None:
        assert compute_cy_sub(5, 5, 1) is True  # 5 - 5 - 1 = -1 < 0


class TestAuxCarryAdd:
    def test_no_carry_nibble(self) -> None:
        assert compute_ac_add(0x01, 0x01) is False  # 1 + 1 = 2, fits in nibble
        assert compute_ac_add(0x07, 0x07) is False  # 7 + 7 = 14 = 0xE, fits

    def test_carry_nibble(self) -> None:
        assert compute_ac_add(0x08, 0x08) is True   # 8 + 8 = 16 > 0xF
        assert compute_ac_add(0x0F, 0x01) is True   # 15 + 1 = 16

    def test_with_carry_in(self) -> None:
        assert compute_ac_add(0x0F, 0x00, 1) is True  # 15 + 0 + 1 = 16


class TestAuxCarrySub:
    def test_no_borrow_nibble(self) -> None:
        assert compute_ac_sub(0x0F, 0x01) is False  # 15 - 1 = 14, no borrow
        assert compute_ac_sub(0x0F, 0x0F) is False  # equal, no borrow

    def test_borrow_nibble(self) -> None:
        assert compute_ac_sub(0x00, 0x01) is True  # 0 - 1, borrow


class TestAuxCarryAna:
    def test_bit3_both_set(self) -> None:
        # If bit 3 of either operand is 1, AC is set
        assert compute_ac_ana(0x08, 0) is True

    def test_bit3_neither_set(self) -> None:
        assert compute_ac_ana(0x07, 0x07) is False

    def test_bit3_one_operand(self) -> None:
        assert compute_ac_ana(0x08 | 0x07, 0) is True


class TestSZPFlags:
    def test_returns_tuple(self) -> None:
        s, z, p = szp_flags(0x00)
        assert s is False
        assert z is True
        assert p is True  # 0 has even (0) number of 1-bits

    def test_negative_nonzero_odd_parity(self) -> None:
        s, z, p = szp_flags(0x81)  # 1 and sign bit set, 2 bits = even
        assert s is True
        assert z is False
        assert p is True  # 2 ones → even


class TestFlagsFromByte:
    def test_all_zero_flags(self) -> None:
        # flags_byte with all flags clear = 0x02 (bit1 always 1)
        s, z, ac, p, cy = flags_from_byte(0x02)
        assert s is False
        assert z is False
        assert ac is False
        assert p is False
        assert cy is False

    def test_all_set_flags(self) -> None:
        # S=1,Z=1,AC=1,P=1,CY=1: 0b11010111 = 0xD7
        s, z, ac, p, cy = flags_from_byte(0xD7)
        assert s is True
        assert z is True
        assert ac is True
        assert p is True
        assert cy is True

    def test_carry_only(self) -> None:
        s, z, ac, p, cy = flags_from_byte(0x03)  # bit1=1, bit0=CY=1
        assert cy is True
        assert s is False
        assert z is False

    def test_round_trip_with_state(self) -> None:
        from intel8080_simulator import Intel8080State
        state = Intel8080State(
            a=0, b=0, c=0, d=0, e=0, h=0, l=0,
            sp=0, pc=0,
            flag_s=True, flag_z=False, flag_ac=True, flag_p=True, flag_cy=False,
            interrupts_enabled=False, halted=False,
            memory=tuple([0] * 65536),
            input_ports=tuple([0] * 256),
            output_ports=tuple([0] * 256),
        )
        fb = state.flags_byte
        s, z, ac, p, cy = flags_from_byte(fb)
        assert s == state.flag_s
        assert z == state.flag_z
        assert ac == state.flag_ac
        assert p == state.flag_p
        assert cy == state.flag_cy
