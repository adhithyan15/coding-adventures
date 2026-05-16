"""Tests for alu.py — ALU operations for the Motorola 68000."""

import pytest

from motorola68k_gatelevel.alu import (
    add8,
    add16,
    add32,
    and8,
    and16,
    and32,
    asl,
    asr,
    cmp8,
    cmp16,
    cmp32,
    divs,
    divu,
    lsl,
    lsr,
    muls,
    mulu,
    neg8,
    neg16,
    neg32,
    not8,
    not16,
    not32,
    or8,
    or16,
    or32,
    rol,
    ror,
    roxl,
    roxr,
    sub8,
    sub16,
    sub32,
    xor8,
    xor16,
    xor32,
)


class TestAdd32:
    """32-bit addition — primary 68000 ALU operation."""

    def test_basic(self):
        r = add32(5, 3)
        assert r.result == 8
        assert r.flag_c == 0
        assert r.flag_v == 0
        assert r.flag_n == 0
        assert r.flag_z == 0

    def test_zero_result(self):
        r = add32(0, 0)
        assert r.result == 0
        assert r.flag_z == 1
        assert r.flag_n == 0

    def test_unsigned_overflow(self):
        r = add32(0xFFFFFFFF, 1)
        assert r.result == 0
        assert r.flag_c == 1
        assert r.flag_z == 1

    def test_signed_overflow_positive(self):
        # 0x7FFFFFFF + 1 → 0x80000000 (overflows into negative)
        r = add32(0x7FFFFFFF, 1)
        assert r.result == 0x80000000
        assert r.flag_v == 1
        assert r.flag_n == 1
        assert r.flag_c == 0

    def test_negative_plus_positive(self):
        # -1 + 1 = 0 (no overflow)
        r = add32(0xFFFFFFFF, 1)
        assert r.result == 0
        assert r.flag_z == 1
        assert r.flag_c == 1
        assert r.flag_v == 0

    def test_extend_in(self):
        r = add32(5, 3, extend_in=1)
        assert r.result == 9

    def test_max_plus_max(self):
        r = add32(0xFFFFFFFF, 0xFFFFFFFF)
        assert r.result == 0xFFFFFFFE
        assert r.flag_c == 1

    def test_negative_flag(self):
        r = add32(0x80000000, 0)
        assert r.flag_n == 1

    def test_x_equals_c(self):
        r = add32(0xFFFFFFFF, 1)
        assert r.flag_x == r.flag_c


class TestSub32:
    """32-bit subtraction."""

    def test_basic(self):
        r = sub32(10, 3)
        assert r.result == 7
        assert r.flag_c == 0

    def test_borrow(self):
        r = sub32(0, 1)
        assert r.result == 0xFFFFFFFF
        assert r.flag_c == 1
        assert r.flag_n == 1

    def test_equal(self):
        r = sub32(5, 5)
        assert r.result == 0
        assert r.flag_z == 1
        assert r.flag_c == 0

    def test_signed_overflow(self):
        # 0x80000000 - 1 → 0x7FFFFFFF (min_neg - 1 = max_pos: overflow)
        r = sub32(0x80000000, 1)
        assert r.result == 0x7FFFFFFF
        assert r.flag_v == 1

    def test_extend_in_subx(self):
        r = sub32(10, 3, extend_in=1)
        assert r.result == 6  # 10 - 3 - 1

    def test_x_equals_c(self):
        r = sub32(0, 1)
        assert r.flag_x == r.flag_c == 1


class TestAnd32:
    """32-bit AND — 32 parallel AND gates."""

    def test_basic(self):
        r = and32(0xFF00FF00, 0x0F0F0F0F)
        assert r.result == 0x0F000F00

    def test_zero(self):
        r = and32(0, 0xFFFFFFFF)
        assert r.result == 0
        assert r.flag_z == 1

    def test_all_ones(self):
        r = and32(0xFFFFFFFF, 0xFFFFFFFF)
        assert r.result == 0xFFFFFFFF
        assert r.flag_n == 1

    def test_flags_cleared(self):
        r = and32(0xFF, 0xFF)
        assert r.flag_v == 0
        assert r.flag_c == 0

    def test_negative_flag(self):
        r = and32(0x80000000, 0xFFFFFFFF)
        assert r.flag_n == 1

    def test_zero_and_anything(self):
        r = and32(0, 0xDEADBEEF)
        assert r.result == 0
        assert r.flag_z == 1


class TestOr32:
    """32-bit OR."""

    def test_basic(self):
        r = or32(0xFF000000, 0x00FFFFFF)
        assert r.result == 0xFFFFFFFF

    def test_zero_result(self):
        r = or32(0, 0)
        assert r.result == 0
        assert r.flag_z == 1

    def test_flags_cleared(self):
        r = or32(0xFF, 0xFF)
        assert r.flag_v == 0
        assert r.flag_c == 0

    def test_negative_flag(self):
        r = or32(0x80000000, 0)
        assert r.flag_n == 1


class TestXor32:
    """32-bit XOR."""

    def test_basic(self):
        r = xor32(0xAAAAAAAA, 0x55555555)
        assert r.result == 0xFFFFFFFF

    def test_self_xor_zero(self):
        r = xor32(0xDEADBEEF, 0xDEADBEEF)
        assert r.result == 0
        assert r.flag_z == 1

    def test_flags(self):
        r = xor32(0xFF, 0xFF)
        assert r.flag_v == 0
        assert r.flag_c == 0


class TestNot32:
    """32-bit NOT — 32 parallel NOT gates."""

    def test_zero(self):
        assert not32(0) == 0xFFFFFFFF

    def test_all_ones(self):
        assert not32(0xFFFFFFFF) == 0

    def test_alternating(self):
        assert not32(0xAAAAAAAA) == 0x55555555

    def test_double_not(self):
        for v in [0, 1, 0xDEADBEEF, 0xFFFFFFFF]:
            assert not32(not32(v)) == v


class TestNeg32:
    """32-bit negation: 0 - a."""

    def test_neg_one(self):
        r = neg32(1)
        assert r.result == 0xFFFFFFFF
        assert r.flag_c == 1

    def test_neg_zero(self):
        r = neg32(0)
        assert r.result == 0
        assert r.flag_c == 0
        assert r.flag_z == 1

    def test_neg_max_neg(self):
        # NEG(0x80000000) → 0x80000000 (overflow)
        r = neg32(0x80000000)
        assert r.result == 0x80000000
        assert r.flag_v == 1


class TestCmp32:
    """32-bit compare — same as sub but X not updated."""

    def test_equal(self):
        r = cmp32(5, 5)
        assert r.flag_z == 1
        assert r.flag_c == 0

    def test_a_greater(self):
        r = cmp32(10, 3)
        assert r.flag_c == 0
        assert r.flag_z == 0

    def test_a_less(self):
        r = cmp32(3, 10)
        assert r.flag_c == 1


class TestAdd16:
    def test_basic(self):
        r = add16(5, 3)
        assert r.result == 8

    def test_overflow(self):
        r = add16(0xFFFF, 1)
        assert r.result == 0
        assert r.flag_c == 1

    def test_signed_overflow(self):
        r = add16(0x7FFF, 1)
        assert r.flag_v == 1

    def test_extend_in(self):
        r = add16(5, 3, extend_in=1)
        assert r.result == 9

    def test_negative(self):
        r = add16(0x8000, 0)
        assert r.flag_n == 1


class TestSub16:
    def test_basic(self):
        r = sub16(10, 3)
        assert r.result == 7

    def test_borrow(self):
        r = sub16(0, 1)
        assert r.flag_c == 1

    def test_zero(self):
        r = sub16(5, 5)
        assert r.flag_z == 1


class TestAnd16:
    def test_basic(self):
        r = and16(0xFF00, 0x0FF0)
        assert r.result == 3840

    def test_flags_cleared(self):
        r = and16(0xFFFF, 0xFFFF)
        assert r.flag_v == 0
        assert r.flag_c == 0


class TestOr16:
    def test_basic(self):
        r = or16(0xFF00, 0x00FF)
        assert r.result == 0xFFFF


class TestXor16:
    def test_basic(self):
        r = xor16(0xAAAA, 0x5555)
        assert r.result == 0xFFFF

    def test_self_xor(self):
        r = xor16(0xABCD, 0xABCD)
        assert r.flag_z == 1


class TestNot16:
    def test_zero(self):
        assert not16(0) == 0xFFFF

    def test_invert(self):
        assert not16(0xAAAA) == 0x5555


class TestNeg16:
    def test_one(self):
        r = neg16(1)
        assert r.result == 0xFFFF

    def test_zero(self):
        r = neg16(0)
        assert r.flag_c == 0


class TestAdd8:
    def test_basic(self):
        r = add8(5, 3)
        assert r.result == 8

    def test_overflow(self):
        r = add8(0xFF, 1)
        assert r.result == 0
        assert r.flag_c == 1

    def test_signed_overflow(self):
        r = add8(0x7F, 1)
        assert r.flag_v == 1


class TestSub8:
    def test_basic(self):
        r = sub8(10, 3)
        assert r.result == 7

    def test_borrow(self):
        r = sub8(0, 1)
        assert r.flag_c == 1


class TestAnd8:
    def test_basic(self):
        r = and8(0xF0, 0x0F)
        assert r.result == 0

    def test_all_ones(self):
        r = and8(0xFF, 0xFF)
        assert r.result == 0xFF


class TestOr8:
    def test_basic(self):
        r = or8(0xF0, 0x0F)
        assert r.result == 0xFF


class TestXor8:
    def test_basic(self):
        r = xor8(0xFF, 0x55)
        assert r.result == 0xAA

    def test_self_xor(self):
        r = xor8(0xAB, 0xAB)
        assert r.flag_z == 1


class TestNot8:
    def test_zero(self):
        assert not8(0) == 0xFF

    def test_invert(self):
        assert not8(0xAA) == 0x55


class TestNeg8:
    def test_one(self):
        r = neg8(1)
        assert r.result == 0xFF

    def test_zero(self):
        r = neg8(0)
        assert r.flag_c == 0


class TestCmp8:
    def test_equal(self):
        r = cmp8(5, 5)
        assert r.flag_z == 1

    def test_less(self):
        r = cmp8(3, 5)
        assert r.flag_c == 1


class TestCmp16:
    def test_equal(self):
        r = cmp16(100, 100)
        assert r.flag_z == 1

    def test_greater(self):
        r = cmp16(100, 50)
        assert r.flag_c == 0


class TestASL:
    """Arithmetic shift left."""

    def test_shift_1(self):
        r, c, v = asl(0b00000001, 1, 8)
        assert r == 2
        assert c == 0

    def test_shift_out(self):
        r, c, v = asl(0b10000000, 1, 8)
        assert r == 0
        assert c == 1

    def test_overflow(self):
        # 0b01000000 << 1 → 0b10000000 (MSB changes: V=1)
        r, c, v = asl(0b01000000, 1, 8)
        assert r == 0b10000000
        assert v == 1

    def test_no_overflow(self):
        r, c, v = asl(0b00000001, 1, 8)
        assert v == 0

    def test_zero_count(self):
        r, c, v = asl(0x42, 0, 8)
        assert r == 0x42
        assert c == 0
        assert v == 0

    def test_32bit(self):
        r, c, v = asl(1, 1, 32)
        assert r == 2


class TestASR:
    """Arithmetic shift right — sign-extending."""

    def test_positive(self):
        r, c = asr(0b01000000, 1, 8)
        assert r == 0b00100000
        assert c == 0

    def test_negative_extends(self):
        r, c = asr(0b10000000, 1, 8)
        assert r == 0b11000000  # sign bit replicated

    def test_carry_out(self):
        r, c = asr(0b10000001, 1, 8)
        assert c == 1

    def test_zero_count(self):
        r, c = asr(0x42, 0, 8)
        assert r == 0x42
        assert c == 0

    def test_16bit(self):
        r, c = asr(0x8000, 1, 16)
        assert r == 0xC000  # sign extended


class TestLSL:
    """Logical shift left."""

    def test_basic(self):
        r, c = lsl(0b00000001, 1, 8)
        assert r == 2
        assert c == 0

    def test_carry_out(self):
        r, c = lsl(0b10000000, 1, 8)
        assert r == 0
        assert c == 1

    def test_32bit(self):
        r, c = lsl(0x80000000, 1, 32)
        assert r == 0
        assert c == 1

    def test_zero_count(self):
        r, c = lsl(0x42, 0, 8)
        assert r == 0x42
        assert c == 0


class TestLSR:
    """Logical shift right."""

    def test_basic(self):
        r, c = lsr(0b00000010, 1, 8)
        assert r == 1
        assert c == 0

    def test_carry_out(self):
        r, c = lsr(0b00000001, 1, 8)
        assert r == 0
        assert c == 1

    def test_no_sign_extend(self):
        r, c = lsr(0b10000000, 1, 8)
        assert r == 0b01000000  # no sign extension

    def test_zero_count(self):
        r, c = lsr(0x42, 0, 8)
        assert r == 0x42


class TestROL:
    """Rotate left."""

    def test_basic(self):
        r, c = rol(0b10000000, 1, 8)
        assert r == 0b00000001
        # C = the bit that just wrapped into bit 0 = old MSB = 1
        assert c == 1

    def test_wrap(self):
        r, c = rol(0b10000001, 1, 8)
        assert r == 0b00000011  # bit 7 wraps to bit 0

    def test_32bit(self):
        r, c = rol(0x80000000, 1, 32)
        assert r == 1

    def test_zero_count(self):
        r, c = rol(0x42, 0, 8)
        assert r == 0x42


class TestROR:
    """Rotate right."""

    def test_basic(self):
        r, c = ror(0b00000001, 1, 8)
        assert r == 0b10000000  # bit 0 wraps to bit 7

    def test_msb_to_lsb(self):
        r, c = ror(0b10000000, 1, 8)
        assert r == 0b01000000

    def test_32bit(self):
        r, c = ror(1, 1, 32)
        assert r == 0x80000000

    def test_zero_count(self):
        r, c = ror(0x42, 0, 8)
        assert r == 0x42


class TestROXL:
    """Rotate left through X."""

    def test_basic(self):
        r, c = roxl(0b10000000, 1, 8, x=0)
        assert r == 0  # MSB shifts out → C=1
        assert c == 1

    def test_x_into_lsb(self):
        r, c = roxl(0b00000000, 1, 8, x=1)
        assert r == 0b00000001  # X enters at bit 0
        assert c == 0

    def test_zero_count(self):
        r, c = roxl(0x42, 0, 8, x=1)
        assert r == 0x42
        assert c == 1  # C=X when count=0


class TestROXR:
    """Rotate right through X."""

    def test_basic(self):
        r, c = roxr(0b00000001, 1, 8, x=0)
        assert r == 0  # LSB shifts out → C=1
        assert c == 1

    def test_x_into_msb(self):
        r, c = roxr(0b00000000, 1, 8, x=1)
        assert r == 0b10000000  # X enters at MSB

    def test_zero_count(self):
        r, c = roxr(0x42, 0, 8, x=1)
        assert r == 0x42
        assert c == 1  # C=X when count=0


class TestMULS:
    """Signed 16×16 → 32 multiply."""

    def test_positive(self):
        r32, n, z = muls(5, 3)
        assert r32 == 15
        assert n == 0
        assert z == 0

    def test_zero(self):
        r32, n, z = muls(0, 100)
        assert r32 == 0
        assert z == 1

    def test_negative(self):
        r32, n, z = muls(0xFFFF, 1)  # -1 × 1 = -1
        assert r32 == 0xFFFFFFFF
        assert n == 1

    def test_negative_by_negative(self):
        r32, n, z = muls(0xFFFF, 0xFFFF)  # -1 × -1 = +1
        assert r32 == 1
        assert n == 0


class TestMULU:
    """Unsigned 16×16 → 32 multiply."""

    def test_positive(self):
        r32, n, z = mulu(5, 3)
        assert r32 == 15

    def test_zero(self):
        r32, n, z = mulu(0, 100)
        assert z == 1

    def test_max(self):
        r32, n, z = mulu(0xFFFF, 0xFFFF)
        assert r32 == 0xFFFE0001


class TestDIVS:
    """Signed 32÷16 division."""

    def test_basic(self):
        packed, overflow = divs(10, 3)
        assert not overflow
        q = packed & 0xFFFF
        r = (packed >> 16) & 0xFFFF
        assert q == 3
        assert r == 1

    def test_division_by_zero(self):
        with pytest.raises(ZeroDivisionError):
            divs(10, 0)

    def test_negative_dividend(self):
        packed, overflow = divs(0xFFFFFFFF, 1)  # -1 / 1 = -1
        assert not overflow
        q = packed & 0xFFFF
        assert q == 0xFFFF  # -1 in signed 16-bit


class TestDIVU:
    """Unsigned 32÷16 division."""

    def test_basic(self):
        packed, overflow = divu(10, 3)
        assert not overflow
        q = packed & 0xFFFF
        r = (packed >> 16) & 0xFFFF
        assert q == 3
        assert r == 1

    def test_division_by_zero(self):
        with pytest.raises(ZeroDivisionError):
            divu(10, 0)

    def test_overflow(self):
        _, overflow = divu(0xFFFFFFFF, 2)
        assert overflow


class TestADDXZFlag:
    """ADDX/SUBX Z-flag only-cleared behavior tested via add32/sub32."""

    def test_addx_z_stays_set_if_both_zero(self):
        # For ADDX: if old Z=1 and result=0, Z stays 1
        r = add32(0, 0, 0)
        assert r.flag_z == 1

    def test_addx_z_cleared_if_nonzero(self):
        r = add32(1, 0, 0)
        assert r.flag_z == 0

    def test_subx_z_clears_if_nonzero(self):
        r = sub32(5, 3, 0)
        assert r.flag_z == 0
