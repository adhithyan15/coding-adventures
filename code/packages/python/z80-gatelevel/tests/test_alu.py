"""Tests for alu.py — gate-level ALU operations."""

from z80_gatelevel.alu import (
    adc16,
    add8,
    add16,
    and8,
    bit_test,
    cpl8,
    daa8,
    dec8,
    inc8,
    neg8,
    or8,
    res_bit,
    rl8,
    rla8,
    rlc8,
    rlca8,
    rr8,
    rra8,
    rrc8,
    rrca8,
    sbc16,
    set_bit,
    sla8,
    sra8,
    srl8,
    sub8,
    xor8,
)


class TestAdd8:
    def test_basic(self):
        r = add8(5, 3)
        assert r.result == 8
        assert r.flag_z == 0
        assert r.flag_c == 0
        assert r.flag_n == 0

    def test_zero_result(self):
        r = add8(0, 0)
        assert r.result == 0
        assert r.flag_z == 1
        assert r.flag_s == 0

    def test_carry_out(self):
        r = add8(0xFF, 0x01)
        assert r.result == 0
        assert r.flag_c == 1
        assert r.flag_z == 1

    def test_overflow_positive(self):
        # 0x7F + 0x01 = 0x80: sign changes from + to - → overflow
        r = add8(0x7F, 0x01)
        assert r.result == 0x80
        assert r.flag_pv == 1
        assert r.flag_s == 1

    def test_no_overflow(self):
        r = add8(0x40, 0x20)  # 64 + 32 = 96, no overflow
        assert r.result == 0x60
        assert r.flag_pv == 0

    def test_half_carry(self):
        r = add8(0x0F, 0x01)
        assert r.result == 0x10
        assert r.flag_h == 1

    def test_adc_with_carry(self):
        r = add8(0x05, 0x03, carry_in=1)
        assert r.result == 9

    def test_sign_flag(self):
        r = add8(0x80, 0x00)
        assert r.flag_s == 1

    def test_n_flag_zero(self):
        r = add8(1, 2)
        assert r.flag_n == 0


class TestSub8:
    def test_basic(self):
        r = sub8(10, 5)
        assert r.result == 5
        assert r.flag_z == 0
        assert r.flag_c == 0
        assert r.flag_n == 1

    def test_zero_result(self):
        r = sub8(5, 5)
        assert r.result == 0
        assert r.flag_z == 1

    def test_borrow(self):
        # 5 - 10 = -5 → wraps to 0xFB, carry=1 means borrow
        r = sub8(5, 10)
        assert r.result == 0xFB
        assert r.flag_c == 1

    def test_overflow_neg(self):
        # 0x80 - 0x01 = 0x7F: sign changes from - to + → overflow
        r = sub8(0x80, 0x01)
        assert r.result == 0x7F
        assert r.flag_pv == 1

    def test_n_flag_set(self):
        r = sub8(1, 1)
        assert r.flag_n == 1

    def test_sbc_with_borrow(self):
        # 10 - 5 - 1 = 4
        r = sub8(10, 5, borrow_in=1)
        assert r.result == 4

    def test_half_borrow(self):
        # 0x10 - 0x01: borrow from bit 4 to bit 3 → H=1
        r = sub8(0x10, 0x01)
        assert r.flag_h == 1


class TestAnd8:
    def test_basic(self):
        r = and8(0xFF, 0x0F)
        assert r.result == 0x0F
        assert r.flag_h == 1  # AND always sets H
        assert r.flag_n == 0
        assert r.flag_c == 0

    def test_zero_result(self):
        r = and8(0xF0, 0x0F)
        assert r.result == 0
        assert r.flag_z == 1

    def test_parity(self):
        r = and8(0xFF, 0xFF)
        # 0xFF = all ones = even parity
        assert r.flag_pv == 1

    def test_h_always_1(self):
        r = and8(0, 0)
        assert r.flag_h == 1


class TestOr8:
    def test_basic(self):
        r = or8(0xF0, 0x0F)
        assert r.result == 0xFF
        assert r.flag_h == 0
        assert r.flag_n == 0
        assert r.flag_c == 0

    def test_zero_result(self):
        r = or8(0, 0)
        assert r.result == 0
        assert r.flag_z == 1

    def test_parity(self):
        r = or8(0x01, 0x00)
        # 0x01 = 1 one → odd parity
        assert r.flag_pv == 0


class TestXor8:
    def test_basic(self):
        r = xor8(0xFF, 0x0F)
        assert r.result == 0xF0
        assert r.flag_h == 0
        assert r.flag_n == 0
        assert r.flag_c == 0

    def test_zero_self_xor(self):
        r = xor8(0x42, 0x42)
        assert r.result == 0
        assert r.flag_z == 1

    def test_parity(self):
        r = xor8(0x03, 0x00)
        # 0x03 = two 1-bits → even parity
        assert r.flag_pv == 1


class TestInc8:
    def test_basic(self):
        r = inc8(5)
        assert r.result == 6
        assert r.flag_n == 0

    def test_wraparound(self):
        r = inc8(0xFF)
        assert r.result == 0
        assert r.flag_z == 1

    def test_overflow_at_7f(self):
        # 0x7F + 1 = 0x80: signed overflow
        r = inc8(0x7F)
        assert r.result == 0x80
        assert r.flag_pv == 1

    def test_half_carry(self):
        r = inc8(0x0F)
        assert r.flag_h == 1


class TestDec8:
    def test_basic(self):
        r = dec8(5)
        assert r.result == 4
        assert r.flag_n == 1

    def test_wraparound(self):
        r = dec8(0)
        assert r.result == 0xFF

    def test_overflow_at_80(self):
        # 0x80 - 1 = 0x7F: signed overflow (from -128 to +127)
        r = dec8(0x80)
        assert r.result == 0x7F
        assert r.flag_pv == 1

    def test_zero(self):
        r = dec8(1)
        assert r.result == 0
        assert r.flag_z == 1


class TestNeg8:
    def test_zero_stays_zero(self):
        r = neg8(0)
        assert r.result == 0
        assert r.flag_c == 0  # No borrow when negating 0

    def test_positive(self):
        r = neg8(5)
        assert r.result == 0xFB  # -5 in two's complement

    def test_0x80_overflow(self):
        # 0 - 0x80 = 0x80: only value that overflows
        r = neg8(0x80)
        assert r.result == 0x80
        assert r.flag_pv == 1

    def test_borrow_when_nonzero(self):
        r = neg8(1)
        assert r.flag_c == 1  # Borrow occurred


class TestCpl8:
    def test_basic(self):
        r = cpl8(0xAA)
        assert r.result == 0x55

    def test_zero(self):
        r = cpl8(0)
        assert r.result == 0xFF

    def test_0xff(self):
        r = cpl8(0xFF)
        assert r.result == 0

    def test_flags_h_n(self):
        r = cpl8(0x42)
        assert r.flag_h == 1
        assert r.flag_n == 1


class TestDaa8:
    def test_after_add_no_correction(self):
        # 0x55 (55 BCD) + 0x22 (22 BCD) = 0x77 (77 BCD) — no correction needed
        r = daa8(0x77, flag_n=0, flag_h=0, flag_c=0)
        assert r.result == 0x77

    def test_low_nibble_correction(self):
        # 0x09 + 0x01 = 0x0A → DAA should correct to 0x10 (BCD 10)
        # After add: A=0x0A (not valid BCD), H=1 (carry from nibble)
        r = daa8(0x0A, flag_n=0, flag_h=0, flag_c=0)
        assert r.result == 0x10

    def test_high_nibble_correction(self):
        # 0x90 + 0x20 = 0xB0: high nibble > 9 needs correction
        r = daa8(0xB0, flag_n=0, flag_h=0, flag_c=0)
        assert r.result == 0x10  # 0xB0 + 0x60 = 0x110 → 0x10 with carry

    def test_after_sub(self):
        # After subtraction: DAA subtracts correction
        # 0x09 - 0x01 = 0x08 (valid BCD), no adjustment needed
        r = daa8(0x08, flag_n=1, flag_h=0, flag_c=0)
        assert r.result == 0x08

    def test_n_flag_preserved(self):
        r = daa8(0x00, flag_n=1, flag_h=0, flag_c=0)
        assert r.flag_n == 1

    def test_zero_flag(self):
        r = daa8(0x00, flag_n=0, flag_h=0, flag_c=0)
        assert r.flag_z == 1


class TestAdd16:
    def test_basic(self):
        r = add16(0x1234, 0x0001)
        assert r.result == 0x1235
        assert r.flag_c == 0
        assert r.flag_n == 0

    def test_carry(self):
        r = add16(0xFFFF, 0x0001)
        assert r.result == 0
        assert r.flag_c == 1

    def test_half_carry_16(self):
        r = add16(0x0FFF, 0x0001)
        assert r.result == 0x1000
        assert r.flag_h == 1


class TestAdc16:
    def test_basic(self):
        r = adc16(0x1000, 0x1000, 0)
        assert r.result == 0x2000
        assert r.flag_z == 0

    def test_with_carry(self):
        r = adc16(0x1000, 0x1000, 1)
        assert r.result == 0x2001

    def test_zero(self):
        r = adc16(0, 0, 0)
        assert r.result == 0
        assert r.flag_z == 1

    def test_overflow_16(self):
        # 0x7FFF + 0x0001 = 0x8000: signed 16-bit overflow
        r = adc16(0x7FFF, 0x0001, 0)
        assert r.result == 0x8000
        assert r.flag_pv == 1


class TestSbc16:
    def test_basic(self):
        r = sbc16(0x2000, 0x1000, 0)
        assert r.result == 0x1000
        assert r.flag_c == 0
        assert r.flag_n == 1

    def test_borrow(self):
        r = sbc16(0x1000, 0x2000, 0)
        assert r.flag_c == 1

    def test_zero(self):
        r = sbc16(0x1000, 0x1000, 0)
        assert r.result == 0
        assert r.flag_z == 1

    def test_n_flag(self):
        r = sbc16(10, 5, 0)
        assert r.flag_n == 1


class TestRlc8:
    def test_basic(self):
        r = rlc8(0b00000001)
        assert r.result == 0b00000010
        assert r.flag_c == 0

    def test_msb_wraps(self):
        r = rlc8(0b10000000)
        assert r.result == 0b00000001
        assert r.flag_c == 1

    def test_0xff(self):
        r = rlc8(0xFF)
        assert r.result == 0xFF
        assert r.flag_c == 1


class TestRrc8:
    def test_basic(self):
        r = rrc8(0b00000010)
        assert r.result == 0b00000001
        assert r.flag_c == 0

    def test_lsb_wraps(self):
        r = rrc8(0b00000001)
        assert r.result == 0b10000000
        assert r.flag_c == 1


class TestRl8:
    def test_no_carry(self):
        r = rl8(0b00000001, 0)
        assert r.result == 0b00000010
        assert r.flag_c == 0

    def test_carry_in(self):
        r = rl8(0b00000000, 1)
        assert r.result == 0b00000001
        assert r.flag_c == 0

    def test_carry_out(self):
        r = rl8(0b10000000, 0)
        assert r.result == 0b00000000
        assert r.flag_c == 1


class TestRr8:
    def test_no_carry(self):
        r = rr8(0b00000010, 0)
        assert r.result == 0b00000001
        assert r.flag_c == 0

    def test_carry_in(self):
        r = rr8(0b00000000, 1)
        assert r.result == 0b10000000
        assert r.flag_c == 0

    def test_carry_out(self):
        r = rr8(0b00000001, 0)
        assert r.result == 0b00000000
        assert r.flag_c == 1


class TestSla8:
    def test_basic(self):
        r = sla8(0b00000001)
        assert r.result == 0b00000010
        assert r.flag_c == 0

    def test_carry(self):
        r = sla8(0b10000000)
        assert r.result == 0b00000000
        assert r.flag_c == 1

    def test_zero_in(self):
        r = sla8(0b10000001)
        assert r.result == 0b00000010  # 0 shifted into bit 0, not 1
        assert r.flag_c == 1


class TestSra8:
    def test_positive(self):
        # Sign bit 0: arithmetic shift = logical shift
        r = sra8(0b00000010)
        assert r.result == 0b00000001
        assert r.flag_c == 0

    def test_negative_preserves_sign(self):
        # Sign bit 1: preserved
        r = sra8(0b10000000)
        assert r.result == 0b11000000
        assert r.flag_c == 0

    def test_carry_out(self):
        r = sra8(0b00000001)
        assert r.result == 0b00000000
        assert r.flag_c == 1


class TestSrl8:
    def test_basic(self):
        r = srl8(0b00000010)
        assert r.result == 0b00000001
        assert r.flag_c == 0

    def test_clears_msb(self):
        r = srl8(0b10000000)
        assert r.result == 0b01000000
        assert r.flag_c == 0

    def test_carry(self):
        r = srl8(0b00000001)
        assert r.result == 0
        assert r.flag_c == 1


class TestBitTest:
    def test_bit_set(self):
        r = bit_test(0b00000001, 0)
        assert r.flag_z == 0   # Z=0: bit was SET
        assert r.flag_h == 1

    def test_bit_clear(self):
        r = bit_test(0b00000000, 0)
        assert r.flag_z == 1   # Z=1: bit was CLEAR
        assert r.flag_h == 1

    def test_bit7_set(self):
        r = bit_test(0b10000000, 7)
        assert r.flag_z == 0
        assert r.flag_s == 1  # S set for bit 7

    def test_n_flag(self):
        r = bit_test(0xFF, 3)
        assert r.flag_n == 0

    def test_result_zero(self):
        r = bit_test(0xFF, 0)
        assert r.result == 0  # BIT never modifies the register


class TestSetResbit:
    def test_set_bit(self):
        assert set_bit(0b00000000, 3) == 0b00001000

    def test_set_already_set(self):
        assert set_bit(0b11111111, 3) == 0b11111111

    def test_res_bit(self):
        assert res_bit(0b11111111, 3) == 0b11110111

    def test_res_already_clear(self):
        assert res_bit(0b00000000, 3) == 0b00000000


class TestAccumRotates:
    def test_rlca(self):
        r = rlca8(0b10000000)
        assert r.result == 0b00000001
        assert r.flag_c == 1

    def test_rrca(self):
        r = rrca8(0b00000001)
        assert r.result == 0b10000000
        assert r.flag_c == 1

    def test_rla(self):
        r = rla8(0b10000000, 0)
        assert r.result == 0
        assert r.flag_c == 1

    def test_rla_carry_in(self):
        r = rla8(0b00000000, 1)
        assert r.result == 0b00000001
        assert r.flag_c == 0

    def test_rra(self):
        r = rra8(0b00000001, 0)
        assert r.result == 0
        assert r.flag_c == 1

    def test_rra_carry_in(self):
        r = rra8(0b00000000, 1)
        assert r.result == 0b10000000
        assert r.flag_c == 0

    def test_rlca_flags_not_set(self):
        # RLCA/RRCA/RLA/RRA should not set S/Z/PV
        r = rlca8(0)
        assert r.flag_s == 0
        assert r.flag_z == 0
        assert r.flag_pv == 0
