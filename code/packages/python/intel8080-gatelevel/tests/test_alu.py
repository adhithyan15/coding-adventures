"""Tests for ALU8080 — gate-level arithmetic/logic operations."""

from __future__ import annotations

import pytest

from intel8080_gatelevel.alu import ALU8080


class TestALUAdd:
    def setup_method(self) -> None:
        self.alu = ALU8080()

    def test_add_basic(self) -> None:
        r = self.alu.execute(0, 10, 5, False, False)
        assert r.result == 15
        assert r.cy is False
        assert r.z is False

    def test_add_overflow(self) -> None:
        r = self.alu.execute(0, 0xFF, 1, False, False)
        assert r.result == 0
        assert r.cy is True
        assert r.z is True

    def test_add_sets_sign(self) -> None:
        r = self.alu.execute(0, 0x70, 0x70, False, False)
        assert r.s is True   # bit7 set

    def test_add_sets_parity(self) -> None:
        # 3 + 0 = 3 = 0b00000011 → 2 ones → even parity
        r = self.alu.execute(0, 3, 0, False, False)
        assert r.p is True

    def test_adc_with_carry(self) -> None:
        # 5 + 3 + 1 = 9
        r = self.alu.execute(1, 5, 3, True, False)
        assert r.result == 9

    def test_adc_without_carry(self) -> None:
        r = self.alu.execute(1, 5, 3, False, False)
        assert r.result == 8


class TestALUSub:
    def setup_method(self) -> None:
        self.alu = ALU8080()

    def test_sub_no_borrow(self) -> None:
        r = self.alu.execute(2, 10, 3, False, False)
        assert r.result == 7
        assert r.cy is False

    def test_sub_with_borrow(self) -> None:
        r = self.alu.execute(2, 3, 10, False, False)
        assert r.result == 0xF9   # 3 - 10 = -7 mod 256
        assert r.cy is True

    def test_sub_self(self) -> None:
        r = self.alu.execute(2, 5, 5, False, False)
        assert r.result == 0
        assert r.cy is False
        assert r.z is True

    def test_sbb_no_borrow(self) -> None:
        # 8 - 3 - 0 = 5
        r = self.alu.execute(3, 8, 3, False, False)
        assert r.result == 5

    def test_sbb_with_borrow(self) -> None:
        # 8 - 3 - 1 = 4
        r = self.alu.execute(3, 8, 3, True, False)
        assert r.result == 4


class TestALULogical:
    def setup_method(self) -> None:
        self.alu = ALU8080()

    def test_ana_basic(self) -> None:
        r = self.alu.execute(4, 0xFF, 0x0F, False, False)
        assert r.result == 0x0F
        assert r.cy is False

    def test_ana_clears_carry(self) -> None:
        r = self.alu.execute(4, 0xFF, 0xFF, True, False)
        assert r.cy is False  # ANA always clears CY

    def test_ana_ac_quirk(self) -> None:
        # AC = OR(bit3(a), bit3(b)) — 8080-specific quirk
        # a = 0x08 (bit3=1), b = 0x00 (bit3=0): AC = OR(1,0) = 1
        r = self.alu.execute(4, 0x08, 0x00, False, False)
        assert r.ac is True
        # a = 0x00 (bit3=0), b = 0x00 (bit3=0): AC = OR(0,0) = 0
        r2 = self.alu.execute(4, 0x00, 0x00, False, False)
        assert r2.ac is False

    def test_xra_self(self) -> None:
        # XRA A clears accumulator
        r = self.alu.execute(5, 0xFF, 0xFF, False, False)
        assert r.result == 0
        assert r.cy is False
        assert r.ac is False

    def test_ora_basic(self) -> None:
        r = self.alu.execute(6, 0x0F, 0xF0, False, False)
        assert r.result == 0xFF
        assert r.cy is False
        assert r.ac is False

    def test_cmp_equal(self) -> None:
        # CMP with equal values: Z=1, CY=0, A unchanged
        r = self.alu.execute(7, 5, 5, False, False)
        assert r.z is True
        assert r.cy is False

    def test_cmp_less(self) -> None:
        # A < B: borrow → CY=1
        r = self.alu.execute(7, 5, 10, False, False)
        assert r.cy is True

    def test_cmp_greater(self) -> None:
        r = self.alu.execute(7, 10, 5, False, False)
        assert r.cy is False
        assert r.z is False


class TestALUInrDcr:
    def setup_method(self) -> None:
        self.alu = ALU8080()

    def test_inr_basic(self) -> None:
        r = self.alu.execute(8, 5, 0, False, False)
        assert r.result == 6
        assert r.update_cy is False  # INR does not update CY

    def test_inr_wraps(self) -> None:
        r = self.alu.execute(8, 0xFF, 0, False, False)
        assert r.result == 0
        assert r.z is True

    def test_dcr_basic(self) -> None:
        r = self.alu.execute(9, 5, 0, False, False)
        assert r.result == 4
        assert r.update_cy is False

    def test_dcr_wraps(self) -> None:
        r = self.alu.execute(9, 0, 0, False, False)
        assert r.result == 0xFF


class TestALURotates:
    def setup_method(self) -> None:
        self.alu = ALU8080()

    def test_rlc(self) -> None:
        # 0x85 = 0b10000101; RLC → 0b00001011 = 0x0B; CY=1
        r = self.alu.execute(10, 0x85, 0, False, False)
        assert r.result == 0x0B
        assert r.cy is True

    def test_rlc_no_carry(self) -> None:
        r = self.alu.execute(10, 0x05, 0, False, False)
        assert r.result == 0x0A
        assert r.cy is False

    def test_rrc(self) -> None:
        # 0x85 = 0b10000101; RRC → 0b11000010 = 0xC2; CY=1 (lsb was 1)
        r = self.alu.execute(11, 0x85, 0, False, False)
        assert r.result == 0xC2
        assert r.cy is True

    def test_ral_with_carry(self) -> None:
        # CY=1, A=0x85; RAL → A7=1 → new CY=1; CY_in=1 → A0=1
        r = self.alu.execute(12, 0x85, 0, True, False)
        assert r.result == 0x0B
        assert r.cy is True

    def test_ral_without_carry(self) -> None:
        # CY=0, A=0x85; new A = 0x0A; new CY=1
        r = self.alu.execute(12, 0x85, 0, False, False)
        assert r.result == 0x0A
        assert r.cy is True

    def test_rar_with_carry(self) -> None:
        # STC; A=0x85; RAR → old CY=1 → A7=1; A0=1 → new CY=1; A=(1<<7)|(0x85>>1)
        r = self.alu.execute(13, 0x85, 0, True, False)
        assert r.result == 0xC2
        assert r.cy is True

    def test_rar_without_carry(self) -> None:
        r = self.alu.execute(13, 0x84, 0, False, False)
        assert r.result == 0x42
        assert r.cy is False


class TestALUSpecial:
    def setup_method(self) -> None:
        self.alu = ALU8080()

    def test_cma(self) -> None:
        r = self.alu.execute(14, 0xAA, 0, False, False)
        assert r.result == 0x55

    def test_cma_ff(self) -> None:
        r = self.alu.execute(14, 0xFF, 0, False, False)
        assert r.result == 0x00

    def test_daa_no_adjust(self) -> None:
        r = self.alu.execute(15, 0x05, 0, False, False)
        assert r.result == 0x05

    def test_daa_low_nibble(self) -> None:
        # 0x0A → add 6 → 0x10
        r = self.alu.execute(15, 0x0A, 0, False, False)
        assert r.result == 0x10

    def test_daa_after_bcd_add(self) -> None:
        # 0x25 + 0x38 = 0x5D; DAA → 0x63 (25+38=63 in BCD)
        r = self.alu.execute(15, 0x5D, 0, False, False)
        assert r.result == 0x63

    def test_unknown_op(self) -> None:
        with pytest.raises(ValueError):
            self.alu.execute(16, 0, 0, False, False)
