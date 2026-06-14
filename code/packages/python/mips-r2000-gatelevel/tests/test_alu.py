"""Tests for alu.py — 32-bit gate-level ALU operations."""


from mips_r2000_gatelevel.alu import (
    ALUResult32,
    add32,
    and32,
    div32,
    divu32,
    mult32,
    multu32,
    nor32,
    or32,
    sll32,
    slt32,
    sltu32,
    sra32,
    srl32,
    sub32,
    xor32,
)

# ── add32 ──────────────────────────────────────────────────────────────────────


class TestAdd32:
    def test_basic(self):
        r = add32(1, 2)
        assert r.result == 3
        assert r.zero == 0
        assert r.negative == 0

    def test_zero_result(self):
        r = add32(0, 0)
        assert r.result == 0
        assert r.zero == 1

    def test_carry_propagation(self):
        r = add32(0xFFFF_FFFF, 1)
        assert r.result == 0
        assert r.carry == 1
        assert r.zero == 1

    def test_no_overflow_simple(self):
        r = add32(100, 200)
        assert r.overflow == 0
        assert r.result == 300

    def test_signed_overflow_positive(self):
        # MAX_INT + 1 = overflow
        r = add32(0x7FFF_FFFF, 1)
        assert r.result == 0x8000_0000
        assert r.overflow == 1

    def test_signed_overflow_negative(self):
        # MIN_INT + MIN_INT = overflow
        r = add32(0x8000_0000, 0x8000_0000)
        assert r.overflow == 1

    def test_no_overflow_negative_plus_positive(self):
        # -1 + 1 = 0; no overflow
        r = add32(0xFFFF_FFFF, 1)
        assert r.overflow == 0

    def test_carry_in(self):
        r = add32(0, 0, carry_in=1)
        assert r.result == 1

    def test_negative_flag(self):
        r = add32(0x7FFF_FFFF, 0x7FFF_FFFF)
        assert r.negative == 1  # bit 31 set

    def test_result_type(self):
        r = add32(1, 1)
        assert isinstance(r, ALUResult32)


# ── sub32 ──────────────────────────────────────────────────────────────────────


class TestSub32:
    def test_basic(self):
        r = sub32(5, 3)
        assert r.result == 2
        assert r.carry == 1  # no borrow

    def test_zero_result(self):
        r = sub32(5, 5)
        assert r.result == 0
        assert r.zero == 1

    def test_borrow(self):
        # 3 - 5: borrow occurs, carry=0
        r = sub32(3, 5)
        assert r.carry == 0  # borrow

    def test_signed_overflow_sub(self):
        # MIN_INT - 1 = overflow (0x80000000 - 1 = 0x7FFFFFFF with no OV, but:
        # actually 0x80000000 - 0x00000001: -2147483648 - 1 should overflow
        r = sub32(0x8000_0000, 1)
        assert r.overflow == 1

    def test_no_overflow(self):
        r = sub32(10, 5)
        assert r.overflow == 0
        assert r.result == 5

    def test_unsigned_wrap(self):
        # 0 - 1 = 0xFFFFFFFF (unsigned wrap)
        r = sub32(0, 1)
        assert r.result == 0xFFFF_FFFF
        assert r.carry == 0

    def test_negative_result(self):
        r = sub32(0, 1)
        assert r.negative == 1


# ── and32 / or32 / xor32 / nor32 ──────────────────────────────────────────────


class TestBitwiseOps:
    def test_and_basic(self):
        r = and32(0xFF00_FF00, 0x0F0F_0F0F)
        assert r.result == 0x0F00_0F00

    def test_and_all_zeros(self):
        r = and32(0xFFFF_FFFF, 0)
        assert r.result == 0
        assert r.zero == 1

    def test_and_all_ones(self):
        r = and32(0xFFFF_FFFF, 0xFFFF_FFFF)
        assert r.result == 0xFFFF_FFFF

    def test_or_basic(self):
        r = or32(0xFF00_0000, 0x00FF_0000)
        assert r.result == 0xFFFF_0000

    def test_or_zero(self):
        r = or32(0, 0)
        assert r.result == 0
        assert r.zero == 1

    def test_xor_basic(self):
        r = xor32(0xFFFF_FFFF, 0xFFFF_FFFF)
        assert r.result == 0
        assert r.zero == 1

    def test_xor_different(self):
        r = xor32(0xAAAA_AAAA, 0x5555_5555)
        assert r.result == 0xFFFF_FFFF

    def test_nor_basic(self):
        # NOR(0, 0) = NOT(OR(0,0)) = NOT(0) = 0xFFFFFFFF
        r = nor32(0, 0)
        assert r.result == 0xFFFF_FFFF

    def test_nor_all_ones(self):
        r = nor32(0xFFFF_FFFF, 0)
        assert r.result == 0

    def test_nor_identity_with_zero(self):
        # NOR(x, 0) = NOT(x)
        val = 0x1234_5678
        r = nor32(val, 0)
        assert r.result == (~val) & 0xFFFF_FFFF

    def test_bitwise_carry_always_zero(self):
        for op in [and32, or32, xor32, nor32]:
            r = op(0xFFFF_FFFF, 0xFFFF_FFFF)
            assert r.carry == 0
            assert r.overflow == 0


# ── slt32 / sltu32 ─────────────────────────────────────────────────────────────


class TestSlt:
    def test_slt_less(self):
        r = slt32(1, 2)
        assert r.result == 1

    def test_slt_equal(self):
        r = slt32(5, 5)
        assert r.result == 0

    def test_slt_greater(self):
        r = slt32(2, 1)
        assert r.result == 0

    def test_slt_signed_negative(self):
        # -1 (0xFFFFFFFF) < 0: signed comparison
        r = slt32(0xFFFF_FFFF, 0)
        assert r.result == 1

    def test_slt_signed_min(self):
        # MIN_INT < 0
        r = slt32(0x8000_0000, 0)
        assert r.result == 1

    def test_slt_signed_max(self):
        # MAX_INT > 0
        r = slt32(0x7FFF_FFFF, 0)
        assert r.result == 0

    def test_slt_signed_min_max(self):
        # MIN_INT < MAX_INT
        r = slt32(0x8000_0000, 0x7FFF_FFFF)
        assert r.result == 1

    def test_sltu_basic(self):
        r = sltu32(1, 2)
        assert r.result == 1

    def test_sltu_equal(self):
        r = sltu32(5, 5)
        assert r.result == 0

    def test_sltu_unsigned_max(self):
        # 0xFFFFFFFF is the largest unsigned; it's NOT less than anything
        r = sltu32(0xFFFF_FFFF, 0)
        assert r.result == 0

    def test_sltu_unsigned_zero_vs_max(self):
        # 0 < 0xFFFFFFFF unsigned
        r = sltu32(0, 0xFFFF_FFFF)
        assert r.result == 1

    def test_sltu_signed_negative_treated_as_large(self):
        # 0xFFFFFFFF treated as large unsigned > 0
        r = sltu32(0, 0xFFFF_FFFF)
        assert r.result == 1


# ── sll32 / srl32 / sra32 ─────────────────────────────────────────────────────


class TestShifts:
    def test_sll_zero(self):
        assert sll32(0b1010, 0).result == 0b1010

    def test_sll_one(self):
        assert sll32(1, 1).result == 2

    def test_sll_15(self):
        assert sll32(1, 15).result == 0x8000

    def test_sll_31(self):
        assert sll32(1, 31).result == 0x8000_0000

    def test_srl_zero(self):
        assert srl32(4, 0).result == 4

    def test_srl_one(self):
        assert srl32(4, 1).result == 2

    def test_srl_15(self):
        assert srl32(0x8000_0000, 15).result == 0x10000

    def test_srl_31(self):
        assert srl32(0x8000_0000, 31).result == 1

    def test_srl_no_sign_fill(self):
        r = srl32(0xFFFF_FFFF, 1)
        assert r.result == 0x7FFF_FFFF

    def test_sra_positive_same_as_srl(self):
        assert sra32(4, 1).result == 2

    def test_sra_negative_sign_fill(self):
        # 0x80000000 (most negative) >> 1 = 0xC0000000 (sign fill)
        r = sra32(0x8000_0000, 1)
        assert r.result == 0xC000_0000

    def test_sra_31_positions(self):
        # 0x80000000 >> 31 = 0xFFFFFFFF
        r = sra32(0x8000_0000, 31)
        assert r.result == 0xFFFF_FFFF

    def test_sra_zero_shift(self):
        assert sra32(0xDEAD_BEEF, 0).result == 0xDEAD_BEEF

    def test_sll_zero_flag(self):
        r = sll32(0, 5)
        assert r.zero == 1

    def test_srl_zero_flag(self):
        r = srl32(0, 5)
        assert r.zero == 1


# ── multu32 ────────────────────────────────────────────────────────────────────


class TestMultu32:
    def test_zero(self):
        hi, lo = multu32(0, 100)
        assert hi == 0
        assert lo == 0

    def test_one(self):
        hi, lo = multu32(1, 42)
        assert hi == 0
        assert lo == 42

    def test_basic(self):
        hi, lo = multu32(6, 7)
        assert hi == 0
        assert lo == 42

    def test_large(self):
        hi, lo = multu32(0xFFFF_FFFF, 2)
        # 0xFFFFFFFF * 2 = 0x1FFFFFFFE
        assert hi == 1
        assert lo == 0xFFFF_FFFE

    def test_max_unsigned(self):
        # 0xFFFFFFFF * 0xFFFFFFFF = 0xFFFFFFFE00000001
        hi, lo = multu32(0xFFFF_FFFF, 0xFFFF_FFFF)
        assert hi == 0xFFFF_FFFE
        assert lo == 1

    def test_power_of_two(self):
        hi, lo = multu32(1, 0x8000_0000)
        assert hi == 0
        assert lo == 0x8000_0000


# ── mult32 ─────────────────────────────────────────────────────────────────────


class TestMult32:
    def test_positive_positive(self):
        hi, lo = mult32(6, 7)
        assert hi == 0
        assert lo == 42

    def test_negative_positive(self):
        # -1 * 1 = -1 (0xFFFFFFFF_FFFFFFFF as 64-bit two's complement)
        hi, lo = mult32(0xFFFF_FFFF, 1)
        assert hi == 0xFFFF_FFFF
        assert lo == 0xFFFF_FFFF

    def test_negative_negative(self):
        # -1 * -1 = 1
        hi, lo = mult32(0xFFFF_FFFF, 0xFFFF_FFFF)
        assert hi == 0
        assert lo == 1

    def test_min_int_times_one(self):
        # -2147483648 * 1 = -2147483648 (0xFFFFFFFF80000000 as 64-bit)
        hi, lo = mult32(0x8000_0000, 1)
        assert hi == 0xFFFF_FFFF
        assert lo == 0x8000_0000

    def test_zero(self):
        hi, lo = mult32(0, 0xFFFF_FFFF)
        assert hi == 0
        assert lo == 0


# ── divu32 ─────────────────────────────────────────────────────────────────────


class TestDivu32:
    def test_basic(self):
        q, r = divu32(10, 3)
        assert q == 3
        assert r == 1

    def test_exact_division(self):
        q, r = divu32(12, 4)
        assert q == 3
        assert r == 0

    def test_divide_by_one(self):
        q, r = divu32(42, 1)
        assert q == 42
        assert r == 0

    def test_dividend_less_than_divisor(self):
        q, r = divu32(3, 10)
        assert q == 0
        assert r == 3

    def test_divide_by_zero(self):
        q, r = divu32(100, 0)
        assert q == 0xFFFF_FFFF
        assert r == 100

    def test_large(self):
        q, r = divu32(0xFFFF_FFFF, 2)
        assert q == 0x7FFF_FFFF
        assert r == 1

    def test_quotient_one(self):
        q, r = divu32(7, 5)
        assert q == 1
        assert r == 2


# ── div32 ──────────────────────────────────────────────────────────────────────


class TestDiv32:
    def test_basic_positive(self):
        q, r = div32(10, 3)
        assert q == 3
        assert r == 1

    def test_exact(self):
        q, r = div32(12, 4)
        assert q == 3
        assert r == 0

    def test_negative_dividend(self):
        # -10 / 3 = -3 remainder -1
        q, r = div32(0xFFFF_FFF6, 3)
        assert q == 0xFFFF_FFFD  # -3
        assert r == 0xFFFF_FFFF  # -1

    def test_negative_divisor(self):
        # 10 / -3 = -3 remainder 1
        q, r = div32(10, 0xFFFF_FFFD)
        assert q == 0xFFFF_FFFD  # -3
        assert r == 1

    def test_negative_both(self):
        # -10 / -3 = 3 remainder -1
        q, r = div32(0xFFFF_FFF6, 0xFFFF_FFFD)
        assert q == 3
        assert r == 0xFFFF_FFFF  # -1

    def test_divide_by_zero(self):
        q, r = div32(10, 0)
        assert q == 0xFFFF_FFFF
        assert r == 10

    def test_divide_one(self):
        q, r = div32(42, 1)
        assert q == 42
        assert r == 0
