"""test_alu.py — Unit tests for alu.py (64-bit gate-level ALU).

Tests cover every exported function with:
  - Normal cases
  - Edge cases (0, max value, wrap-around)
  - Sign extension for L variants
  - Compare operations (0 or 1 result)
  - Multiply (small values, edge cases)
"""

from __future__ import annotations

from alpha_axp_gatelevel.alu import (
    addl,
    addq,
    andq,
    bicq,
    cmpeq,
    cmple,
    cmplt,
    cmpule,
    cmpult,
    eqvq,
    mull,
    mulq,
    ornot,
    orq,
    s4addl,
    s4addq,
    s4subl,
    s4subq,
    s8addl,
    s8addq,
    s8subl,
    s8subq,
    sll64,
    sra64,
    srl64,
    subl,
    subq,
    umulh,
    xorq,
)

MASK64 = 0xFFFF_FFFF_FFFF_FFFF
MASK32 = 0xFFFF_FFFF
MIN_NEG = 0x8000_0000_0000_0000   # -2^63 as unsigned
MAX_POS = 0x7FFF_FFFF_FFFF_FFFF   # 2^63 - 1


# ── Helper ────────────────────────────────────────────────────────────────────

def signed(v: int) -> int:
    """Reinterpret 64-bit unsigned as signed."""
    v = v & MASK64
    if v >= 0x8000_0000_0000_0000:
        return v - (1 << 64)
    return v


# ── ADDQ ─────────────────────────────────────────────────────────────────────

class TestAddq:
    def test_basic(self):
        r = addq(3, 4)
        assert r.result == 7
        assert r.carry == 0
        assert r.overflow == 0
        assert r.zero == 0
        assert r.negative == 0

    def test_zero(self):
        r = addq(0, 0)
        assert r.result == 0
        assert r.zero == 1

    def test_carry_wrap(self):
        r = addq(MASK64, 1)
        assert r.result == 0
        assert r.carry == 1
        assert r.zero == 1

    def test_overflow_pos_pos(self):
        r = addq(MAX_POS, 1)
        assert r.overflow == 1
        assert r.result == MIN_NEG

    def test_no_overflow_neg_pos(self):
        r = addq(MIN_NEG, MAX_POS)
        assert r.overflow == 0

    def test_negative_flag(self):
        r = addq(MIN_NEG, 0)
        assert r.negative == 1

    def test_carry_in(self):
        r = addq(MASK64, 0, carry_in=1)
        assert r.result == 0
        assert r.carry == 1


# ── SUBQ ─────────────────────────────────────────────────────────────────────

class TestSubq:
    def test_basic(self):
        r = subq(10, 3)
        assert r.result == 7

    def test_zero_minus_zero(self):
        r = subq(0, 0)
        assert r.result == 0
        assert r.zero == 1

    def test_borrow(self):
        # 0 - 1 = 0xFFFF...FFFF (borrow: carry_out=0)
        r = subq(0, 1)
        assert r.result == MASK64
        assert r.carry == 0

    def test_overflow_neg_minus_pos(self):
        # MIN_NEG - 1 → overflow (MIN_NEG - 1 = 0x7FFF...FFFF)
        r = subq(MIN_NEG, 1)
        assert r.overflow == 1

    def test_no_overflow_pos_minus_smaller(self):
        r = subq(10, 5)
        assert r.overflow == 0
        assert r.result == 5


# ── AND / BIS / XOR / BIC / ORNOT / EQV ──────────────────────────────────────

class TestLogical:
    def test_and_basic(self):
        assert andq(0b1100, 0b1010).result == 0b1000

    def test_and_zero(self):
        assert andq(0, MASK64).result == 0

    def test_and_all_ones(self):
        assert andq(MASK64, MASK64).result == MASK64

    def test_or_basic(self):
        assert orq(0b1100, 0b0011).result == 0b1111

    def test_or_all_zeros(self):
        assert orq(0, 0).result == 0

    def test_or_all_ones(self):
        assert orq(MASK64, 0).result == MASK64

    def test_xor_basic(self):
        assert xorq(0b1111, 0b1010).result == 0b0101

    def test_xor_self(self):
        assert xorq(0xDEAD_BEEF, 0xDEAD_BEEF).result == 0

    def test_xor_zero(self):
        assert xorq(0xABCD, 0).result == 0xABCD

    def test_bic_basic(self):
        # BIC: a & ~b
        assert bicq(0b1111, 0b1010).result == 0b0101

    def test_bic_clear_all(self):
        assert bicq(MASK64, MASK64).result == 0

    def test_ornot_basic(self):
        # ORNOT: a | ~b  — for a=0, b=0: 0 | ~0 = all-ones
        assert ornot(0, 0).result == MASK64

    def test_ornot_all_ones(self):
        # ORNOT(MASK64, 0) = MASK64 | ~0 = MASK64 | MASK64 = MASK64
        assert ornot(MASK64, 0).result == MASK64

    def test_eqv_same(self):
        # EQV(a, a): a ^ ~a = all ones
        v = 0xDEAD_BEEF_CAFE_BABE
        assert eqvq(v, v).result == MASK64

    def test_eqv_zero(self):
        # EQV(0, MASK64): 0 ^ ~MASK64 = 0 ^ 0 = 0
        assert eqvq(0, MASK64).result == 0


# ── Shifts ────────────────────────────────────────────────────────────────────

class TestShifts:
    def test_sll_by_zero(self):
        assert sll64(5, 0).result == 5

    def test_sll_by_one(self):
        assert sll64(1, 1).result == 2

    def test_sll_by_32(self):
        assert sll64(1, 32).result == 0x1_0000_0000

    def test_sll_by_63(self):
        assert sll64(1, 63).result == MIN_NEG

    def test_srl_by_zero(self):
        assert srl64(8, 0).result == 8

    def test_srl_by_one(self):
        assert srl64(8, 1).result == 4

    def test_srl_by_32(self):
        assert srl64(0x1_0000_0000, 32).result == 1

    def test_srl_zero_fills_msbs(self):
        r = srl64(MASK64, 1)
        assert r.result == 0x7FFF_FFFF_FFFF_FFFF

    def test_sra_positive(self):
        assert sra64(8, 1).result == 4

    def test_sra_negative_sign_extend(self):
        r = sra64(MASK64, 1)
        assert r.result == MASK64  # sign extends, still all-ones

    def test_sra_by_63_negative(self):
        r = sra64(MIN_NEG, 63)
        assert r.result == MASK64  # all ones

    def test_sra_by_63_positive(self):
        r = sra64(MAX_POS, 63)
        assert r.result == 0


# ── Compare ───────────────────────────────────────────────────────────────────

class TestCompare:
    def test_cmpeq_equal(self):
        assert cmpeq(5, 5) == 1

    def test_cmpeq_not_equal(self):
        assert cmpeq(5, 6) == 0

    def test_cmpeq_zero(self):
        assert cmpeq(0, 0) == 1

    def test_cmplt_less(self):
        # Signed: signed(-1) < 0 → True
        assert cmplt(MASK64, 0) == 1  # -1 < 0 signed

    def test_cmplt_not_less(self):
        assert cmplt(5, 5) == 0
        assert cmplt(6, 5) == 0

    def test_cmplt_positive(self):
        assert cmplt(3, 5) == 1

    def test_cmple_less(self):
        assert cmple(3, 5) == 1

    def test_cmple_equal(self):
        assert cmple(5, 5) == 1

    def test_cmple_greater(self):
        assert cmple(6, 5) == 0

    def test_cmpult_less(self):
        assert cmpult(3, 5) == 1

    def test_cmpult_greater(self):
        assert cmpult(5, 3) == 0

    def test_cmpult_equal(self):
        assert cmpult(5, 5) == 0

    def test_cmpult_unsigned_large(self):
        # Unsigned: MASK64 > 0 (even though signed MASK64 = -1)
        assert cmpult(MASK64, 1) == 0  # MASK64 > 1 unsigned

    def test_cmpule_equal(self):
        assert cmpule(5, 5) == 1

    def test_cmpule_less(self):
        assert cmpule(3, 5) == 1

    def test_cmpule_greater(self):
        assert cmpule(6, 5) == 0


# ── ADDL / SUBL (32-bit with sign extension) ──────────────────────────────────

class TestLongword:
    def test_addl_basic(self):
        assert addl(1, 2).result == 3

    def test_addl_sign_extend_positive(self):
        # 0x7FFFFFFF + 1 = 0x80000000 → sign-extended to 0xFFFFFFFF80000000
        r = addl(0x7FFF_FFFF, 1)
        assert r.result == 0xFFFF_FFFF_8000_0000

    def test_addl_wraps_32bit(self):
        # 0xFFFFFFFF + 1 = 0x100000000 → 32-bit = 0x00000000 → sext = 0
        r = addl(0xFFFF_FFFF, 1)
        assert r.result == 0

    def test_subl_basic(self):
        r = subl(10, 3)
        assert r.result == 7

    def test_subl_sign_extend_negative(self):
        # 0 - 1 = -1 as 32-bit = 0xFFFFFFFF → sext = 0xFFFFFFFFFFFFFFFF
        r = subl(0, 1)
        assert r.result == MASK64


# ── Scaled add ────────────────────────────────────────────────────────────────

class TestScaledAdd:
    def test_s4addq(self):
        # Ra=2, Rb=3: 2*4 + 3 = 11
        assert s4addq(2, 3).result == 11

    def test_s8addq(self):
        # Ra=2, Rb=3: 2*8 + 3 = 19
        assert s8addq(2, 3).result == 19

    def test_s4addl(self):
        r = s4addl(2, 3)
        assert r.result == 11

    def test_s8addl(self):
        r = s8addl(2, 3)
        assert r.result == 19

    def test_s4subq(self):
        # Ra=2, Rb=3: 2*4 - 3 = 5
        assert s4subq(2, 3).result == 5

    def test_s8subq(self):
        assert s8subq(2, 3).result == 13

    def test_s4subl(self):
        assert s4subl(2, 3).result == 5

    def test_s8subl(self):
        assert s8subl(2, 3).result == 13

    def test_s4addq_zero(self):
        assert s4addq(0, 0).result == 0

    def test_s8addq_zero(self):
        assert s8addq(0, 0).result == 0


# ── MULQ ──────────────────────────────────────────────────────────────────────

class TestMulq:
    def test_simple(self):
        assert mulq(3, 4) == 12

    def test_six_times_seven(self):
        assert mulq(6, 7) == 42

    def test_zero(self):
        assert mulq(0, 12345) == 0
        assert mulq(12345, 0) == 0

    def test_one(self):
        assert mulq(1, 42) == 42
        assert mulq(42, 1) == 42

    def test_overflow_wraps(self):
        # 2^32 * 2^32 = 2^64, but lower 64 bits = 0
        r = mulq(0x1_0000_0000, 0x1_0000_0000)
        assert r == 0

    def test_negative_times_positive(self):
        # -1 * 2 = -2 as 64-bit = 0xFFFFFFFFFFFFFFFE
        result = mulq(MASK64, 2)
        assert result == (MASK64 - 1)  # 0xFFFFFFFFFFFFFFFE


# ── UMULH ─────────────────────────────────────────────────────────────────────

class TestUmulh:
    def test_small_no_high(self):
        # Small numbers: product < 2^64, upper = 0
        assert umulh(3, 4) == 0

    def test_max_times_two(self):
        # 0xFFFF...FFFF * 2 = 0x1_FFFF...FFFE
        # upper 64 bits = 0x1
        assert umulh(MASK64, 2) == 1

    def test_max_times_max(self):
        # 0xFFFF...FFFF * 0xFFFF...FFFF
        # = (2^64 - 1)^2 = 2^128 - 2^65 + 1
        # upper = 2^64 - 2 = 0xFFFF...FFFE
        result = umulh(MASK64, MASK64)
        assert result == MASK64 - 1  # 0xFFFFFFFFFFFFFFFE

    def test_zero(self):
        assert umulh(0, MASK64) == 0


# ── MULL ──────────────────────────────────────────────────────────────────────

class TestMull:
    def test_basic(self):
        assert mull(6, 7) == 42

    def test_sign_extend_positive(self):
        # Small positive result stays positive
        assert mull(1, 1) == 1

    def test_negative_result(self):
        # 0x80000000 * 2 = 0x100000000 → 32-bit = 0 → sext = 0
        r = mull(0x8000_0000, 2)
        assert r == 0

    def test_minus_one(self):
        # 0xFFFFFFFF (= -1 as 32-bit) * 1 = 0xFFFFFFFF → sext64 = MASK64
        result = mull(0xFFFF_FFFF, 1)
        assert result == MASK64
