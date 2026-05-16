"""test_alu.py — Unit tests for the 32-bit gate-level ALU.

Tests cover:
- add32 (with/without carry)
- sub32
- and32, or32, xor32, nand32, nor32, eqv32, andc32, orc32
- sll32, srl32, sra32 (CA setting)
- rotl32
- cntlzw
- cmp32 (signed), cmpl32 (unsigned)
- mul32_lo, mul32_hi_signed, mul32_hi_unsigned
- divw (signed), divwu (unsigned, divide by 0)
"""

from __future__ import annotations

from powerpc601_gatelevel.alu import (
    add32,
    and32,
    andc32,
    cmp32,
    cmpl32,
    cntlzw,
    divw,
    divwu,
    eqv32,
    mul32_hi_signed,
    mul32_hi_unsigned,
    mul32_lo,
    nand32,
    nor32,
    or32,
    orc32,
    rotl32,
    sll32,
    sra32,
    srl32,
    sub32,
    xor32,
)


def _u32(v: int) -> int:
    return v & 0xFFFFFFFF


# ── add32 ─────────────────────────────────────────────────────────────────────

class TestAdd32:
    def test_simple(self):
        r = add32(3, 4)
        assert r.result == 7
        assert r.carry == 0
        assert r.overflow == 0
        assert r.zero == 0
        assert r.negative == 0

    def test_zero_result(self):
        r = add32(0, 0)
        assert r.zero == 1
        assert r.result == 0

    def test_carry_out(self):
        r = add32(0xFFFFFFFF, 1)
        assert r.result == 0
        assert r.carry == 1
        assert r.zero == 1

    def test_signed_overflow(self):
        # MAX_INT + 1 = MIN_INT: overflow
        r = add32(0x7FFFFFFF, 1)
        assert r.result == 0x80000000
        assert r.overflow == 1
        assert r.negative == 1

    def test_with_carry_in(self):
        r = add32(1, 1, carry_in=1)
        assert r.result == 3

    def test_negative_result(self):
        r = add32(0xFFFFFFFF, 0)
        assert r.negative == 1
        assert r.zero == 0

    def test_minus_one_plus_one(self):
        r = add32(0xFFFFFFFF, 1)
        assert r.result == 0
        assert r.carry == 1
        assert r.overflow == 0


# ── sub32 ─────────────────────────────────────────────────────────────────────

class TestSub32:
    def test_simple(self):
        r = sub32(10, 3)
        assert r.result == 7
        assert r.zero == 0

    def test_equal(self):
        r = sub32(5, 5)
        assert r.result == 0
        assert r.zero == 1

    def test_zero_minus_one(self):
        # 0 - 1 = -1 = 0xFFFFFFFF
        r = sub32(0, 1)
        assert r.result == 0xFFFFFFFF
        assert r.negative == 1
        assert r.carry == 0  # borrow occurred

    def test_no_borrow(self):
        r = sub32(5, 3)
        assert r.carry == 1  # no borrow

    def test_negative_minus_negative(self):
        # (-1) - (-1) = 0
        r = sub32(0xFFFFFFFF, 0xFFFFFFFF)
        assert r.result == 0
        assert r.zero == 1

    def test_overflow_positive(self):
        # MIN_INT - 1 overflows: MIN_INT = 0x80000000
        r = sub32(0x80000000, 1)
        assert r.overflow == 1


# ── Logical operations ─────────────────────────────────────────────────────────

class TestAnd32:
    def test_basic(self):
        assert and32(0b1010, 0b1100).result == 0b1000

    def test_all_ones(self):
        assert and32(0xFFFFFFFF, 0xFFFFFFFF).result == 0xFFFFFFFF

    def test_zero(self):
        assert and32(0, 0xFFFFFFFF).result == 0
        assert and32(0, 0xFFFFFFFF).zero == 1

    def test_pattern(self):
        assert and32(0xAAAAAAAA, 0x55555555).result == 0


class TestOr32:
    def test_basic(self):
        assert or32(0b1010, 0b0101).result == 0b1111

    def test_zero_or_x(self):
        assert or32(0, 0xDEADBEEF).result == 0xDEADBEEF

    def test_all_ones(self):
        assert or32(0xFFFFFFFF, 0).result == 0xFFFFFFFF

    def test_pattern(self):
        assert or32(0xAAAAAAAA, 0x55555555).result == 0xFFFFFFFF


class TestXor32:
    def test_basic(self):
        assert xor32(0b1111, 0b1010).result == 0b0101

    def test_same_is_zero(self):
        r = xor32(0xDEADBEEF, 0xDEADBEEF)
        assert r.result == 0
        assert r.zero == 1

    def test_zero_preserves(self):
        assert xor32(0xDEADBEEF, 0).result == 0xDEADBEEF

    def test_involution(self):
        v = 0x12345678
        assert xor32(xor32(v, 0xABCDEF01).result, 0xABCDEF01).result == v


class TestNand32:
    def test_all_ones_nand_all_ones(self):
        # NAND(all 1s, all 1s) = all 0s
        assert nand32(0xFFFFFFFF, 0xFFFFFFFF).result == 0

    def test_zero_nand_anything(self):
        # AND(0, x) = 0; NAND(0, x) = NOT(0) = all 1s
        assert nand32(0, 0xFFFFFFFF).result == 0xFFFFFFFF

    def test_basic(self):
        # NAND(0b1100, 0b1010) = NOT(0b1000) = 0b0111...
        expected = _u32(~(0b1100 & 0b1010))
        assert nand32(0b1100, 0b1010).result == expected


class TestNor32:
    def test_zero_nor_zero(self):
        assert nor32(0, 0).result == 0xFFFFFFFF

    def test_all_ones_nor(self):
        assert nor32(0xFFFFFFFF, 0).result == 0

    def test_basic(self):
        expected = _u32(~(0b1010 | 0b0101))
        assert nor32(0b1010, 0b0101).result == expected


class TestEqv32:
    def test_same_is_all_ones(self):
        assert eqv32(0b1010, 0b1010).result == 0xFFFFFFFF

    def test_opposite_is_zero(self):
        assert eqv32(0xAAAAAAAA, 0x55555555).result == 0

    def test_basic(self):
        expected = _u32(~(0b1010 ^ 0b1100))
        assert eqv32(0b1010, 0b1100).result == expected


class TestAndc32:
    def test_basic(self):
        # AND(0b1111, NOT(0b1010)) = AND(0b1111, 0b0101) = 0b0101
        assert andc32(0b1111, 0b1010).result == 0b0101

    def test_zero(self):
        assert andc32(0, 0xFFFFFFFF).result == 0

    def test_all_ones(self):
        # AND(all 1s, NOT(0)) = AND(all 1s, all 1s) = all 1s
        assert andc32(0xFFFFFFFF, 0).result == 0xFFFFFFFF


class TestOrc32:
    def test_zero_or_complement_of_zero(self):
        # ORC(0, 0) = OR(0, NOT(0)) = OR(0, all 1s) = all 1s
        assert orc32(0, 0).result == 0xFFFFFFFF

    def test_basic(self):
        expected = _u32(0b1010 | ~0b1100)
        assert orc32(0b1010, 0b1100).result == expected


# ── Shift operations ──────────────────────────────────────────────────────────

class TestSll32:
    def test_shift_zero(self):
        assert sll32(1, 0).result == 1

    def test_shift_one(self):
        assert sll32(1, 1).result == 2

    def test_shift_31(self):
        assert sll32(1, 31).result == 0x80000000

    def test_shift_32_is_zero(self):
        assert sll32(0xFFFFFFFF, 32).result == 0

    def test_shift_40_is_zero(self):
        # shamt 40 & 0x3F = 40 >= 32, so result is 0
        assert sll32(1, 40).result == 0

    def test_bits_shift_out(self):
        # 0x80000000 << 1 = 0
        assert sll32(0x80000000, 1).result == 0


class TestSrl32:
    def test_shift_zero(self):
        assert srl32(16, 0).result == 16

    def test_shift_four(self):
        assert srl32(16, 4).result == 1

    def test_no_sign_extension(self):
        assert srl32(0xFFFFFFFF, 1).result == 0x7FFFFFFF

    def test_shift_32_is_zero(self):
        assert srl32(0xFFFFFFFF, 32).result == 0


class TestSra32:
    def test_positive_no_change_in_sign(self):
        result_r, ca = sra32(8, 3)
        assert result_r.result == 1
        assert ca == 0

    def test_negative_sign_extension(self):
        result_r, ca = sra32(0xFFFFFFFF, 1)
        assert result_r.result == 0xFFFFFFFF  # -1 >> 1 = -1

    def test_negative_with_ca(self):
        # Negative value, bits shifted out are non-zero
        result_r, ca = sra32(0xFFFFFFFF, 1)  # -1 >> 1
        assert ca == 1  # shifted out bit was 1

    def test_negative_no_ca(self):
        # -4 = 0xFFFFFFFC >> 2: bits[0:2] = 0b00, so no CA
        result_r, ca = sra32(0xFFFFFFFC, 2)
        assert result_r.result == 0xFFFFFFFF  # -1
        assert ca == 0

    def test_min_int_shift(self):
        result_r, ca = sra32(0x80000000, 1)
        assert result_r.result == 0xC0000000

    def test_shift_zero(self):
        result_r, ca = sra32(0xDEADBEEF, 0)
        assert result_r.result == 0xDEADBEEF
        assert ca == 0


# ── rotl32 ────────────────────────────────────────────────────────────────────

class TestRotl32:
    def test_rotate_zero(self):
        assert rotl32(0xDEADBEEF, 0).result == 0xDEADBEEF

    def test_rotate_msb_wraps(self):
        assert rotl32(0x80000000, 1).result == 1

    def test_rotate_by_one(self):
        assert rotl32(1, 1).result == 2

    def test_nibble_rotation(self):
        assert rotl32(0x12345678, 4).result == 0x23456781


# ── cntlzw ────────────────────────────────────────────────────────────────────

class TestCntlzw:
    def test_zero(self):
        assert cntlzw(0).result == 32

    def test_one(self):
        assert cntlzw(1).result == 31

    def test_msb_set(self):
        assert cntlzw(0x80000000).result == 0

    def test_second_bit(self):
        assert cntlzw(0x40000000).result == 1

    def test_all_ones(self):
        assert cntlzw(0xFFFFFFFF).result == 0

    def test_mid_values(self):
        assert cntlzw(0x00010000).result == 15
        assert cntlzw(0x00000001).result == 31
        assert cntlzw(0x00000002).result == 30

    def test_byte_boundary(self):
        assert cntlzw(0x01000000).result == 7


# ── Compare operations ─────────────────────────────────────────────────────────

class TestCmp32:
    def test_equal(self):
        lt, gt, eq = cmp32(5, 5)
        assert lt == 0 and gt == 0 and eq == 1

    def test_less_than(self):
        lt, gt, eq = cmp32(3, 5)
        assert lt == 1 and gt == 0 and eq == 0

    def test_greater_than(self):
        lt, gt, eq = cmp32(5, 3)
        assert lt == 0 and gt == 1 and eq == 0

    def test_negative_less_than_positive(self):
        # -1 (0xFFFFFFFF) < 1 in signed
        lt, gt, eq = cmp32(0xFFFFFFFF, 1)
        assert lt == 1

    def test_positive_greater_than_negative(self):
        lt, gt, eq = cmp32(1, 0xFFFFFFFF)
        assert gt == 1

    def test_negative_equal(self):
        lt, gt, eq = cmp32(0xFFFFFFFF, 0xFFFFFFFF)
        assert eq == 1

    def test_overflow_boundary(self):
        # MIN_INT < MAX_INT
        lt, gt, eq = cmp32(0x80000000, 0x7FFFFFFF)
        assert lt == 1


class TestCmpl32:
    def test_equal(self):
        lt, gt, eq = cmpl32(5, 5)
        assert eq == 1

    def test_less_than(self):
        lt, gt, eq = cmpl32(3, 5)
        assert lt == 1 and eq == 0

    def test_greater_than(self):
        lt, gt, eq = cmpl32(5, 3)
        assert gt == 1

    def test_large_unsigned_greater(self):
        # 0xFFFFFFFF > 0 in unsigned
        lt, gt, eq = cmpl32(0xFFFFFFFF, 0)
        assert gt == 1

    def test_large_unsigned_less(self):
        lt, gt, eq = cmpl32(0, 0xFFFFFFFF)
        assert lt == 1


# ── Multiply ───────────────────────────────────────────────────────────────────

class TestMul32Lo:
    def test_zero(self):
        lo, hi, ov = mul32_lo(0, 100)
        assert lo == 0
        assert hi == 0

    def test_simple(self):
        lo, hi, ov = mul32_lo(6, 7)
        assert lo == 42
        assert hi == 0

    def test_one(self):
        lo, hi, ov = mul32_lo(0xDEADBEEF, 1)
        assert lo == 0xDEADBEEF
        assert hi == 0

    def test_large_no_overflow(self):
        lo, hi, ov = mul32_lo(0x10000, 0x10000)
        assert lo == 0
        assert hi == 1

    def test_max_unsigned(self):
        # 0xFFFFFFFF * 0xFFFFFFFF = 0xFFFFFFFE00000001
        lo, hi, ov = mul32_lo(0xFFFFFFFF, 0xFFFFFFFF)
        expected_product = 0xFFFFFFFF * 0xFFFFFFFF
        expected_lo = expected_product & 0xFFFFFFFF
        expected_hi = (expected_product >> 32) & 0xFFFFFFFF
        assert lo == expected_lo
        assert hi == expected_hi


class TestMul32HiUnsigned:
    def test_zero(self):
        assert mul32_hi_unsigned(0, 0xFFFFFFFF) == 0

    def test_small(self):
        assert mul32_hi_unsigned(6, 7) == 0

    def test_large(self):
        # (2^32-1)^2: hi = 2^32-2
        result = mul32_hi_unsigned(0xFFFFFFFF, 0xFFFFFFFF)
        expected = ((0xFFFFFFFF * 0xFFFFFFFF) >> 32) & 0xFFFFFFFF
        assert result == expected

    def test_power_of_two(self):
        # 0x80000000 * 2 = 2^32, hi = 1
        assert mul32_hi_unsigned(0x80000000, 2) == 1


class TestMul32HiSigned:
    def test_zero(self):
        assert mul32_hi_signed(0, 0xFFFFFFFF) == 0

    def test_pos_pos(self):
        assert mul32_hi_signed(6, 7) == 0

    def test_neg_pos(self):
        # -1 * 2 = -2; in 64-bit two's complement: 0xFFFFFFFFFFFFFFFE
        # high 32 bits = 0xFFFFFFFF
        result = mul32_hi_signed(0xFFFFFFFF, 2)
        assert result == 0xFFFFFFFF  # -1 (upper 32 bits of -2)

    def test_neg_neg(self):
        # -1 * -1 = 1; upper 32 bits = 0
        result = mul32_hi_signed(0xFFFFFFFF, 0xFFFFFFFF)
        assert result == 0

    def test_neg_large(self):
        # -1 * 0x40000000 (= 0x40000000 as signed positive) = -0x40000000
        # 64-bit signed: 0xFFFFFFFFC0000000 → high 32 bits = 0xFFFFFFFF
        result = mul32_hi_signed(0xFFFFFFFF, 0x40000000)
        assert result == 0xFFFFFFFF


# ── Divide ────────────────────────────────────────────────────────────────────

class TestDivwu:
    def test_simple(self):
        assert divwu(100, 7) == 14

    def test_exact(self):
        assert divwu(42, 6) == 7

    def test_zero_dividend(self):
        assert divwu(0, 5) == 0

    def test_one_divisor(self):
        assert divwu(0xDEADBEEF, 1) == 0xDEADBEEF

    def test_same(self):
        assert divwu(42, 42) == 1

    def test_divisor_zero(self):
        # Undefined: return 0
        assert divwu(5, 0) == 0

    def test_large(self):
        assert divwu(1000000, 7) == 142857

    def test_max_by_max(self):
        assert divwu(0xFFFFFFFF, 0xFFFFFFFF) == 1


class TestDivw:
    def test_positive(self):
        assert divw(100, 7) == 14

    def test_negative_dividend(self):
        # -100 / 7 = -14 (truncate toward zero)
        result = divw(_u32(-100), 7)
        assert result == _u32(-14)

    def test_negative_divisor(self):
        # 100 / -7 = -14
        result = divw(100, _u32(-7))
        assert result == _u32(-14)

    def test_both_negative(self):
        # -100 / -7 = 14
        result = divw(_u32(-100), _u32(-7))
        assert result == 14

    def test_zero_dividend(self):
        assert divw(0, 5) == 0

    def test_divisor_zero(self):
        assert divw(5, 0) == 0

    def test_one(self):
        assert divw(42, 1) == 42

    def test_minus_one_divisor(self):
        # 42 / -1 = -42
        result = divw(42, _u32(-1))
        assert result == _u32(-42)
