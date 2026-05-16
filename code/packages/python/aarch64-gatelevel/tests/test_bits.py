"""test_bits.py — Tests for the 64-bit bit-list helpers in bits.py.

Tests all functions in bits.py including:
  - int_to_bits / bits_to_int (roundtrip)
  - add_64bit / sub_64bit (arithmetic with carry and overflow)
  - add_32bit / sub_32bit
  - Bitwise: and_64bit / or_64bit / xor_64bit / not_64bit (and 32-bit variants)
  - compute_zero (NOR tree)
  - Shifts: shl_64, shr_64_logical, shr_64_arith, ror_64 (and 32-bit variants)
  - clz_64 / clz_32 (count leading zeros)
  - mul_64 / umulh_64 / smulh_64 (multiply)
  - udiv_64 / sdiv_64 (divide)
"""

import pytest
from aarch64_gatelevel.bits import (
    add_32bit,
    add_64bit,
    and_32bit,
    and_64bit,
    bits_to_int,
    clz_32,
    clz_64,
    compute_zero,
    int_to_bits,
    mul_64,
    not_32bit,
    not_64bit,
    or_32bit,
    or_64bit,
    ror_32,
    ror_64,
    sdiv_64,
    shl_32,
    shl_64,
    shr_32_arith,
    shr_32_logical,
    shr_64_arith,
    shr_64_logical,
    smulh_64,
    sub_32bit,
    sub_64bit,
    udiv_64,
    umulh_64,
    xor_32bit,
    xor_64bit,
)


# ── int_to_bits / bits_to_int ─────────────────────────────────────────────────


def test_int_to_bits_zero():
    b = int_to_bits(0, 64)
    assert len(b) == 64
    assert all(x == 0 for x in b)


def test_int_to_bits_one():
    b = int_to_bits(1, 64)
    assert b[0] == 1
    assert all(x == 0 for x in b[1:])


def test_int_to_bits_max64():
    b = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    assert all(x == 1 for x in b)


def test_int_to_bits_5():
    b = int_to_bits(5, 8)
    assert b == [1, 0, 1, 0, 0, 0, 0, 0]


def test_int_to_bits_truncates():
    # Value wider than width should be masked
    b = int_to_bits(0x1FF, 8)  # 0x1FF & 0xFF = 0xFF
    assert bits_to_int(b) == 0xFF


def test_bits_to_int_zero():
    assert bits_to_int([0] * 64) == 0


def test_bits_to_int_one():
    b = [0] * 64
    b[0] = 1
    assert bits_to_int(b) == 1


def test_bits_to_int_max():
    assert bits_to_int([1] * 64) == 0xFFFFFFFFFFFFFFFF


def test_roundtrip_64():
    for v in [0, 1, 42, 0xDEADBEEF, 0xFFFFFFFFFFFFFFFF, 0x8000000000000000]:
        assert bits_to_int(int_to_bits(v, 64)) == v


def test_roundtrip_32():
    for v in [0, 1, 255, 0xDEADBEEF, 0xFFFFFFFF, 0x80000000]:
        assert bits_to_int(int_to_bits(v, 32)) == v


# ── add_64bit ─────────────────────────────────────────────────────────────────


def test_add_64_basic():
    a = int_to_bits(3, 64)
    b = int_to_bits(4, 64)
    r, c, v = add_64bit(a, b)
    assert bits_to_int(r) == 7
    assert c == 0
    assert v == 0


def test_add_64_zero():
    a = int_to_bits(0, 64)
    b = int_to_bits(0, 64)
    r, c, v = add_64bit(a, b)
    assert bits_to_int(r) == 0
    assert c == 0
    assert v == 0


def test_add_64_carry_out():
    # 0xFFFF... + 1 → 0 with carry
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    b = int_to_bits(1, 64)
    r, c, v = add_64bit(a, b)
    assert bits_to_int(r) == 0
    assert c == 1
    assert v == 0  # no signed overflow (both operands effectively -1 and 1 in signed)


def test_add_64_signed_overflow():
    # max positive + 1 → negative (signed overflow)
    a = int_to_bits(0x7FFFFFFFFFFFFFFF, 64)
    b = int_to_bits(1, 64)
    r, c, v = add_64bit(a, b)
    assert bits_to_int(r) == 0x8000000000000000
    assert v == 1  # signed overflow


def test_add_64_carry_in():
    a = int_to_bits(5, 64)
    b = int_to_bits(5, 64)
    r, c, v = add_64bit(a, b, carry_in=1)
    assert bits_to_int(r) == 11
    assert c == 0


def test_add_64_both_negative_overflow():
    # min_negative + min_negative → positive (overflow)
    a = int_to_bits(0x8000000000000000, 64)
    b = int_to_bits(0x8000000000000000, 64)
    r, c, v = add_64bit(a, b)
    assert bits_to_int(r) == 0
    assert v == 1   # signed overflow: -inf + -inf = +0 is wrong in signed
    assert c == 1


# ── sub_64bit ─────────────────────────────────────────────────────────────────


def test_sub_64_basic():
    a = int_to_bits(10, 64)
    b = int_to_bits(3, 64)
    r, c, v = sub_64bit(a, b)
    assert bits_to_int(r) == 7
    assert c == 1   # no borrow


def test_sub_64_zero_result():
    a = int_to_bits(5, 64)
    b = int_to_bits(5, 64)
    r, c, v = sub_64bit(a, b)
    assert bits_to_int(r) == 0
    assert c == 1   # equal → no borrow


def test_sub_64_borrow():
    # 0 - 1 → borrow
    a = int_to_bits(0, 64)
    b = int_to_bits(1, 64)
    r, c, v = sub_64bit(a, b)
    assert bits_to_int(r) == 0xFFFFFFFFFFFFFFFF
    assert c == 0   # borrow occurred


def test_sub_64_signed_overflow():
    # min_neg - 1 → signed overflow
    a = int_to_bits(0x8000000000000000, 64)   # -2^63
    b = int_to_bits(1, 64)
    r, c, v = sub_64bit(a, b)
    assert bits_to_int(r) == 0x7FFFFFFFFFFFFFFF
    assert v == 1   # overflow: -inf - 1 = +max


# ── add_32bit / sub_32bit ─────────────────────────────────────────────────────


def test_add_32_basic():
    a = int_to_bits(5, 32)
    b = int_to_bits(3, 32)
    r, c, v = add_32bit(a, b)
    assert bits_to_int(r) == 8
    assert c == 0
    assert v == 0


def test_add_32_carry():
    a = int_to_bits(0xFFFFFFFF, 32)
    b = int_to_bits(1, 32)
    r, c, v = add_32bit(a, b)
    assert bits_to_int(r) == 0
    assert c == 1


def test_add_32_overflow():
    a = int_to_bits(0x7FFFFFFF, 32)
    b = int_to_bits(1, 32)
    r, c, v = add_32bit(a, b)
    assert bits_to_int(r) == 0x80000000
    assert v == 1


def test_sub_32_basic():
    a = int_to_bits(10, 32)
    b = int_to_bits(3, 32)
    r, c, v = sub_32bit(a, b)
    assert bits_to_int(r) == 7
    assert c == 1


def test_sub_32_borrow():
    a = int_to_bits(0, 32)
    b = int_to_bits(1, 32)
    r, c, v = sub_32bit(a, b)
    assert bits_to_int(r) == 0xFFFFFFFF
    assert c == 0


# ── Bitwise 64-bit ─────────────────────────────────────────────────────────────


def test_and_64():
    a = int_to_bits(0b1010, 64)
    b = int_to_bits(0b1100, 64)
    r = and_64bit(a, b)
    assert bits_to_int(r) == 0b1000


def test_or_64():
    a = int_to_bits(0b1010, 64)
    b = int_to_bits(0b0101, 64)
    r = or_64bit(a, b)
    assert bits_to_int(r) == 0b1111


def test_xor_64():
    a = int_to_bits(0b1111, 64)
    b = int_to_bits(0b1010, 64)
    r = xor_64bit(a, b)
    assert bits_to_int(r) == 0b0101


def test_not_64():
    a = int_to_bits(0, 64)
    r = not_64bit(a)
    assert bits_to_int(r) == 0xFFFFFFFFFFFFFFFF


def test_not_64_all_ones():
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = not_64bit(a)
    assert bits_to_int(r) == 0


def test_and_32():
    a = int_to_bits(0xFF00, 32)
    b = int_to_bits(0x0FF0, 32)
    r = and_32bit(a, b)
    assert bits_to_int(r) == 0x0F00


def test_or_32():
    a = int_to_bits(0xF0, 32)
    b = int_to_bits(0x0F, 32)
    r = or_32bit(a, b)
    assert bits_to_int(r) == 0xFF


def test_xor_32():
    a = int_to_bits(0xFF, 32)
    b = int_to_bits(0x0F, 32)
    r = xor_32bit(a, b)
    assert bits_to_int(r) == 0xF0


def test_not_32():
    a = int_to_bits(0, 32)
    r = not_32bit(a)
    assert bits_to_int(r) == 0xFFFFFFFF


# ── compute_zero ──────────────────────────────────────────────────────────────


def test_compute_zero_all_zeros():
    assert compute_zero([0] * 64) == 1


def test_compute_zero_one_set():
    b = [0] * 64
    b[0] = 1
    assert compute_zero(b) == 0


def test_compute_zero_all_ones():
    assert compute_zero([1] * 64) == 0


def test_compute_zero_msb_set():
    b = [0] * 64
    b[63] = 1
    assert compute_zero(b) == 0


# ── shl_64 ────────────────────────────────────────────────────────────────────


def test_shl_64_by_0():
    b = int_to_bits(1, 64)
    assert bits_to_int(shl_64(b, 0)) == 1


def test_shl_64_by_1():
    b = int_to_bits(1, 64)
    assert bits_to_int(shl_64(b, 1)) == 2


def test_shl_64_by_63():
    b = int_to_bits(1, 64)
    assert bits_to_int(shl_64(b, 63)) == 0x8000000000000000


def test_shl_64_by_64():
    b = int_to_bits(1, 64)
    assert bits_to_int(shl_64(b, 64)) == 0


def test_shl_64_msb_disappears():
    # MSB shifted out
    b = int_to_bits(0x8000000000000000, 64)
    assert bits_to_int(shl_64(b, 1)) == 0


# ── shr_64_logical ────────────────────────────────────────────────────────────


def test_shr_64_logical_basic():
    b = int_to_bits(8, 64)
    assert bits_to_int(shr_64_logical(b, 3)) == 1


def test_shr_64_logical_fills_zero():
    b = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = shr_64_logical(b, 1)
    assert bits_to_int(r) == 0x7FFFFFFFFFFFFFFF


def test_shr_64_logical_64():
    b = int_to_bits(1, 64)
    assert bits_to_int(shr_64_logical(b, 64)) == 0


# ── shr_64_arith ─────────────────────────────────────────────────────────────


def test_shr_64_arith_positive():
    b = int_to_bits(8, 64)
    assert bits_to_int(shr_64_arith(b, 3)) == 1


def test_shr_64_arith_negative():
    # -1 >> 1 = -1 (all ones)
    b = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = shr_64_arith(b, 1)
    assert bits_to_int(r) == 0xFFFFFFFFFFFFFFFF


def test_shr_64_arith_min_neg():
    # min_neg >> 1 = 0xC000000000000000
    b = int_to_bits(0x8000000000000000, 64)
    r = shr_64_arith(b, 1)
    assert bits_to_int(r) == 0xC000000000000000


def test_shr_64_arith_saturate():
    # Shift >= 64 saturates to 63 (sign-fill)
    b = int_to_bits(0x8000000000000000, 64)
    r = shr_64_arith(b, 64)
    assert bits_to_int(r) == 0xFFFFFFFFFFFFFFFF


# ── ror_64 ────────────────────────────────────────────────────────────────────


def test_ror_64_by_1():
    b = int_to_bits(1, 64)
    r = ror_64(b, 1)
    assert bits_to_int(r) == 0x8000000000000000


def test_ror_64_by_0():
    b = int_to_bits(0xDEADBEEF, 64)
    r = ror_64(b, 0)
    assert bits_to_int(r) == 0xDEADBEEF


def test_ror_64_by_64():
    v = 0xDEADBEEFCAFEBABE
    b = int_to_bits(v, 64)
    r = ror_64(b, 64)
    assert bits_to_int(r) == v  # full rotation = no change


def test_ror_64_msb_to_lsb():
    b = int_to_bits(0x8000000000000000, 64)
    r = ror_64(b, 63)
    assert bits_to_int(r) == 1


# ── shl_32 / shr_32_logical / shr_32_arith / ror_32 ─────────────────────────


def test_shl_32_basic():
    b = int_to_bits(1, 32)
    assert bits_to_int(shl_32(b, 4)) == 16


def test_shl_32_overflow():
    b = int_to_bits(1, 32)
    assert bits_to_int(shl_32(b, 32)) == 0


def test_shr_32_logical_basic():
    b = int_to_bits(16, 32)
    assert bits_to_int(shr_32_logical(b, 4)) == 1


def test_shr_32_arith_negative():
    b = int_to_bits(0xFFFFFFFF, 32)
    r = shr_32_arith(b, 1)
    assert bits_to_int(r) == 0xFFFFFFFF


def test_shr_32_arith_positive():
    b = int_to_bits(8, 32)
    r = shr_32_arith(b, 3)
    assert bits_to_int(r) == 1


def test_ror_32_basic():
    b = int_to_bits(1, 32)
    r = ror_32(b, 1)
    assert bits_to_int(r) == 0x80000000


# ── clz_64 / clz_32 ──────────────────────────────────────────────────────────


def test_clz_64_zero():
    assert clz_64(int_to_bits(0, 64)) == 64


def test_clz_64_one():
    assert clz_64(int_to_bits(1, 64)) == 63


def test_clz_64_msb():
    assert clz_64(int_to_bits(0x8000000000000000, 64)) == 0


def test_clz_64_half():
    assert clz_64(int_to_bits(0x0000000080000000, 64)) == 32


def test_clz_32_zero():
    assert clz_32(int_to_bits(0, 32)) == 32


def test_clz_32_one():
    assert clz_32(int_to_bits(1, 32)) == 31


def test_clz_32_msb():
    assert clz_32(int_to_bits(0x80000000, 32)) == 0


# ── mul_64 ────────────────────────────────────────────────────────────────────


def test_mul_64_basic():
    a = int_to_bits(6, 64)
    b = int_to_bits(7, 64)
    r = mul_64(a, b)
    assert bits_to_int(r) == 42


def test_mul_64_zero():
    a = int_to_bits(0, 64)
    b = int_to_bits(12345, 64)
    r = mul_64(a, b)
    assert bits_to_int(r) == 0


def test_mul_64_one():
    a = int_to_bits(1, 64)
    b = int_to_bits(0xDEADBEEF, 64)
    r = mul_64(a, b)
    assert bits_to_int(r) == 0xDEADBEEF


def test_mul_64_overflow_wraps():
    # (2^32)^2 = 2^64 → low 64 bits = 0
    a = int_to_bits(0x100000000, 64)
    b = int_to_bits(0x100000000, 64)
    r = mul_64(a, b)
    assert bits_to_int(r) == 0


def test_mul_64_large():
    # 0xFFFFFFFF * 0xFFFFFFFF = 0xFFFFFFFE00000001
    a = int_to_bits(0xFFFFFFFF, 64)
    b = int_to_bits(0xFFFFFFFF, 64)
    r = mul_64(a, b)
    assert bits_to_int(r) == 0xFFFFFFFE00000001


# ── umulh_64 ─────────────────────────────────────────────────────────────────


def test_umulh_64_zero():
    a = int_to_bits(0, 64)
    b = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = umulh_64(a, b)
    assert bits_to_int(r) == 0


def test_umulh_64_one():
    # 1 * anything: high bits are 0
    a = int_to_bits(1, 64)
    b = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = umulh_64(a, b)
    assert bits_to_int(r) == 0


def test_umulh_64_large():
    # (2^63) * 2 = 2^64 → high 64 bits = 1
    a = int_to_bits(0x8000000000000000, 64)
    b = int_to_bits(2, 64)
    r = umulh_64(a, b)
    assert bits_to_int(r) == 1


def test_umulh_64_all_ones():
    # (2^64-1)^2 = 2^128 - 2^65 + 1 → high 64 = 2^64-2 = 0xFFFFFFFFFFFFFFFE
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = umulh_64(a, a)
    assert bits_to_int(r) == 0xFFFFFFFFFFFFFFFE


# ── smulh_64 ─────────────────────────────────────────────────────────────────


def test_smulh_64_neg1_times_neg1():
    # -1 * -1 = +1; high 64 bits = 0
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)  # -1
    r = smulh_64(a, a)
    assert bits_to_int(r) == 0


def test_smulh_64_neg1_times_2():
    # -1 * 2 = -2; upper 64 bits of -2 as 128-bit = all ones = 0xFFFFFFFFFFFFFFFF
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)  # -1
    b = int_to_bits(2, 64)
    r = smulh_64(a, b)
    assert bits_to_int(r) == 0xFFFFFFFFFFFFFFFF


def test_smulh_64_pos_times_pos():
    # 2 * 2 = 4; high 64 bits = 0
    a = int_to_bits(2, 64)
    b = int_to_bits(2, 64)
    r = smulh_64(a, b)
    assert bits_to_int(r) == 0


# ── udiv_64 ───────────────────────────────────────────────────────────────────


def test_udiv_64_basic():
    a = int_to_bits(100, 64)
    b = int_to_bits(7, 64)
    q, r = udiv_64(a, b)
    assert bits_to_int(q) == 14
    assert bits_to_int(r) == 2


def test_udiv_64_exact():
    a = int_to_bits(42, 64)
    b = int_to_bits(6, 64)
    q, r = udiv_64(a, b)
    assert bits_to_int(q) == 7
    assert bits_to_int(r) == 0


def test_udiv_64_zero_divisor():
    a = int_to_bits(42, 64)
    b = int_to_bits(0, 64)
    q, r = udiv_64(a, b)
    assert bits_to_int(q) == 0
    assert bits_to_int(r) == 0


def test_udiv_64_dividend_zero():
    a = int_to_bits(0, 64)
    b = int_to_bits(7, 64)
    q, r = udiv_64(a, b)
    assert bits_to_int(q) == 0
    assert bits_to_int(r) == 0


def test_udiv_64_divisor_larger():
    a = int_to_bits(3, 64)
    b = int_to_bits(7, 64)
    q, r = udiv_64(a, b)
    assert bits_to_int(q) == 0
    assert bits_to_int(r) == 3


def test_udiv_64_one():
    a = int_to_bits(0xDEAD, 64)
    b = int_to_bits(1, 64)
    q, r = udiv_64(a, b)
    assert bits_to_int(q) == 0xDEAD
    assert bits_to_int(r) == 0


# ── sdiv_64 ───────────────────────────────────────────────────────────────────


def test_sdiv_64_positive():
    a = int_to_bits(100, 64)
    b = int_to_bits(7, 64)
    q, r = sdiv_64(a, b)
    assert bits_to_int(q) == 14


def test_sdiv_64_negative_dividend():
    # -14 / 3 = -4 (truncates toward zero)
    a = int_to_bits(-14 & 0xFFFFFFFFFFFFFFFF, 64)
    b = int_to_bits(3, 64)
    q, r = sdiv_64(a, b)
    result = bits_to_int(q)
    # Convert to signed
    if result >= 0x8000000000000000:
        result -= 0x10000000000000000
    assert result == -4


def test_sdiv_64_both_negative():
    a = int_to_bits(-15 & 0xFFFFFFFFFFFFFFFF, 64)
    b = int_to_bits(-5 & 0xFFFFFFFFFFFFFFFF, 64)
    q, r = sdiv_64(a, b)
    assert bits_to_int(q) == 3


def test_sdiv_64_zero_divisor():
    a = int_to_bits(100, 64)
    b = int_to_bits(0, 64)
    q, r = sdiv_64(a, b)
    assert bits_to_int(q) == 0
