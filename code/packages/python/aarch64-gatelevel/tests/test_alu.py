"""test_alu.py — Tests for the gate-level AArch64 ALU (alu.py).

Covers:
  - add64 / sub64 / add32 / sub32 with NZCV flag correctness
  - and64 / or64 / xor64 / not64 (and 32-bit variants)
  - logical_flags_64 / logical_flags_32 / flags_to_nzcv
  - apply_shift (all four shift types, both sf=0 and sf=1)
  - clz64 / clz32
  - rev_bytes / rev16_bytes / rev32_bytes
  - mul64 / umulh64 / smulh64
  - udiv64 / sdiv64
"""

import pytest
from aarch64_gatelevel.alu import (
    ALUResult64,
    add32,
    add64,
    and32,
    and64,
    apply_shift,
    clz32,
    clz64,
    flags_to_nzcv,
    logical_flags_32,
    logical_flags_64,
    mul64,
    not32,
    not64,
    or32,
    or64,
    rev16_bytes,
    rev32_bytes,
    rev_bytes,
    sdiv64,
    smulh64,
    sub32,
    sub64,
    udiv64,
    umulh64,
    xor32,
    xor64,
)
from aarch64_gatelevel.bits import bits_to_int, int_to_bits


# ── add64 ─────────────────────────────────────────────────────────────────────


def test_add64_basic():
    a = int_to_bits(3, 64)
    b = int_to_bits(4, 64)
    r = add64(a, b)
    assert r.result == 7
    assert r.carry == 0
    assert r.overflow == 0
    assert r.zero == 0
    assert r.negative == 0


def test_add64_zero_result():
    a = int_to_bits(0, 64)
    b = int_to_bits(0, 64)
    r = add64(a, b)
    assert r.result == 0
    assert r.zero == 1
    assert r.negative == 0


def test_add64_unsigned_overflow():
    # 0xFFFF... + 1 → 0 with carry
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    b = int_to_bits(1, 64)
    r = add64(a, b)
    assert r.result == 0
    assert r.carry == 1
    assert r.zero == 1
    assert r.overflow == 0


def test_add64_signed_overflow():
    # max_pos + 1 → min_neg (signed overflow)
    a = int_to_bits(0x7FFFFFFFFFFFFFFF, 64)
    b = int_to_bits(1, 64)
    r = add64(a, b)
    assert r.result == 0x8000000000000000
    assert r.overflow == 1
    assert r.negative == 1


def test_add64_negative_flag():
    a = int_to_bits(0x8000000000000000, 64)
    b = int_to_bits(0, 64)
    r = add64(a, b)
    assert r.negative == 1


def test_add64_carry_in():
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    b = int_to_bits(0, 64)
    r = add64(a, b, carry_in=1)
    assert r.result == 0
    assert r.carry == 1


# ── sub64 ─────────────────────────────────────────────────────────────────────


def test_sub64_basic():
    a = int_to_bits(10, 64)
    b = int_to_bits(3, 64)
    r = sub64(a, b)
    assert r.result == 7
    assert r.carry == 1   # no borrow
    assert r.overflow == 0


def test_sub64_equal():
    a = int_to_bits(5, 64)
    b = int_to_bits(5, 64)
    r = sub64(a, b)
    assert r.result == 0
    assert r.zero == 1
    assert r.carry == 1   # no borrow (a >= b)


def test_sub64_borrow():
    # 0 - 1 → borrow (carry = 0)
    a = int_to_bits(0, 64)
    b = int_to_bits(1, 64)
    r = sub64(a, b)
    assert r.result == 0xFFFFFFFFFFFFFFFF
    assert r.carry == 0   # borrow occurred
    assert r.negative == 1


def test_sub64_signed_overflow():
    # min_neg - 1 → signed overflow
    a = int_to_bits(0x8000000000000000, 64)
    b = int_to_bits(1, 64)
    r = sub64(a, b)
    assert r.overflow == 1


# ── add32 / sub32 ─────────────────────────────────────────────────────────────


def test_add32_basic():
    a = int_to_bits(5, 32)
    b = int_to_bits(3, 32)
    r = add32(a, b)
    assert r.result == 8
    assert r.carry == 0
    assert r.zero == 0


def test_add32_carry():
    a = int_to_bits(0xFFFFFFFF, 32)
    b = int_to_bits(1, 32)
    r = add32(a, b)
    assert r.result == 0
    assert r.carry == 1
    assert r.zero == 1


def test_add32_overflow():
    a = int_to_bits(0x7FFFFFFF, 32)
    b = int_to_bits(1, 32)
    r = add32(a, b)
    assert r.overflow == 1
    assert r.negative == 1


def test_sub32_basic():
    a = int_to_bits(10, 32)
    b = int_to_bits(3, 32)
    r = sub32(a, b)
    assert r.result == 7
    assert r.carry == 1


def test_sub32_borrow():
    a = int_to_bits(0, 32)
    b = int_to_bits(1, 32)
    r = sub32(a, b)
    assert r.result == 0xFFFFFFFF
    assert r.carry == 0


# ── Logical operations ─────────────────────────────────────────────────────────


def test_and64_basic():
    a = int_to_bits(0b1010, 64)
    b = int_to_bits(0b1100, 64)
    r = and64(a, b)
    assert r.result == 0b1000
    assert r.carry == 0
    assert r.overflow == 0


def test_or64_basic():
    a = int_to_bits(0b1010, 64)
    b = int_to_bits(0b0101, 64)
    r = or64(a, b)
    assert r.result == 0b1111


def test_xor64_basic():
    a = int_to_bits(0b1111, 64)
    b = int_to_bits(0b1010, 64)
    r = xor64(a, b)
    assert r.result == 0b0101


def test_not64_basic():
    a = int_to_bits(0, 64)
    r = not64(a)
    assert r.result == 0xFFFFFFFFFFFFFFFF


def test_not64_all_ones():
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = not64(a)
    assert r.result == 0


def test_and32_basic():
    a = int_to_bits(0xFF, 32)
    b = int_to_bits(0x0F, 32)
    r = and32(a, b)
    assert r.result == 0x0F


def test_or32_basic():
    a = int_to_bits(0xF0, 32)
    b = int_to_bits(0x0F, 32)
    r = or32(a, b)
    assert r.result == 0xFF


def test_xor32_basic():
    a = int_to_bits(0xFF, 32)
    b = int_to_bits(0x0F, 32)
    r = xor32(a, b)
    assert r.result == 0xF0


def test_not32_basic():
    a = int_to_bits(0, 32)
    r = not32(a)
    assert r.result == 0xFFFFFFFF


# ── Logical flags ─────────────────────────────────────────────────────────────


def test_logical_flags_64_zero():
    r = int_to_bits(0, 64)
    n, z, c, v = logical_flags_64(r)
    assert n == 0
    assert z == 1
    assert c == 0
    assert v == 0


def test_logical_flags_64_negative():
    r = int_to_bits(0x8000000000000000, 64)
    n, z, c, v = logical_flags_64(r)
    assert n == 1
    assert z == 0
    assert c == 0
    assert v == 0


def test_logical_flags_32_zero():
    r = int_to_bits(0, 32)
    n, z, c, v = logical_flags_32(r)
    assert n == 0
    assert z == 1
    assert c == 0
    assert v == 0


def test_logical_flags_32_negative():
    r = int_to_bits(0x80000000, 32)
    n, z, c, v = logical_flags_32(r)
    assert n == 1


def test_flags_to_nzcv():
    assert flags_to_nzcv(1, 0, 1, 0) == 0b1010
    assert flags_to_nzcv(0, 1, 0, 0) == 0b0100
    assert flags_to_nzcv(0, 0, 0, 1) == 0b0001
    assert flags_to_nzcv(1, 1, 1, 1) == 0b1111
    assert flags_to_nzcv(0, 0, 0, 0) == 0


# ── apply_shift ───────────────────────────────────────────────────────────────


def test_apply_shift_lsl_64():
    v = int_to_bits(1, 64)
    r = apply_shift(v, 0, 4, sf=1)
    assert bits_to_int(r) == 16


def test_apply_shift_lsr_64():
    v = int_to_bits(16, 64)
    r = apply_shift(v, 1, 4, sf=1)
    assert bits_to_int(r) == 1


def test_apply_shift_asr_64_positive():
    v = int_to_bits(16, 64)
    r = apply_shift(v, 2, 4, sf=1)
    assert bits_to_int(r) == 1


def test_apply_shift_asr_64_negative():
    # -1 >> 4 = -1 (arithmetic right shift of all-ones)
    v = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = apply_shift(v, 2, 4, sf=1)
    assert bits_to_int(r) == 0xFFFFFFFFFFFFFFFF


def test_apply_shift_ror_64():
    v = int_to_bits(1, 64)
    r = apply_shift(v, 3, 1, sf=1)
    assert bits_to_int(r) == 0x8000000000000000


def test_apply_shift_lsl_32():
    v = int_to_bits(1, 32)
    r = apply_shift(v, 0, 4, sf=0)
    assert bits_to_int(r) == 16


def test_apply_shift_ror_32():
    v = int_to_bits(1, 32)
    r = apply_shift(v, 3, 1, sf=0)
    assert bits_to_int(r) == 0x80000000


def test_apply_shift_zero_amount():
    v = int_to_bits(42, 64)
    r = apply_shift(v, 0, 0, sf=1)
    assert bits_to_int(r) == 42


# ── clz64 / clz32 ────────────────────────────────────────────────────────────


def test_clz64_zero():
    r = clz64(int_to_bits(0, 64))
    assert bits_to_int(r) == 64


def test_clz64_one():
    r = clz64(int_to_bits(1, 64))
    assert bits_to_int(r) == 63


def test_clz64_msb():
    r = clz64(int_to_bits(0x8000000000000000, 64))
    assert bits_to_int(r) == 0


def test_clz32_zero():
    r = clz32(int_to_bits(0, 32))
    assert bits_to_int(r) == 32


def test_clz32_one():
    r = clz32(int_to_bits(1, 32))
    assert bits_to_int(r) == 31


# ── rev_bytes ─────────────────────────────────────────────────────────────────


def test_rev_bytes_8():
    # Reverse byte order of 64-bit value
    v = int_to_bits(0x0102030405060708, 64)
    r = rev_bytes(v, 8)
    assert bits_to_int(r) == 0x0807060504030201


def test_rev_bytes_4():
    # Reverse 4-byte word
    v = int_to_bits(0x01020304, 64)
    r = rev_bytes(v[:32], 4)
    assert bits_to_int(r) == 0x04030201


def test_rev16_bytes_32():
    v = int_to_bits(0x01020304, 64)
    r = rev16_bytes(v, 32)
    expected = bits_to_int(r)
    # Each 16-bit halfword reversed: 0x0102 → 0x0201, 0x0304 → 0x0403
    # In the 32-bit value 0x01020304: byte order is 0304_0102 after swap-within-halfword
    # Actually: rev16 swaps bytes within each 16-bit chunk
    # chunk 0 (bits 0..15) = 0x0304 → 0x0403; chunk 1 (bits 16..31) = 0x0102 → 0x0201
    assert expected == 0x02010403


def test_rev32_bytes():
    v = int_to_bits(0x0102030405060708, 64)
    r = rev32_bytes(v)
    # Each 32-bit word reversed: 01020304 → 04030201, 05060708 → 08070605
    assert bits_to_int(r) == 0x0403020108070605


# ── mul64 / umulh64 / smulh64 ────────────────────────────────────────────────


def test_mul64_basic():
    a = int_to_bits(6, 64)
    b = int_to_bits(7, 64)
    r = mul64(a, b)
    assert bits_to_int(r) == 42


def test_mul64_zero():
    a = int_to_bits(0, 64)
    b = int_to_bits(0xDEAD, 64)
    r = mul64(a, b)
    assert bits_to_int(r) == 0


def test_umulh64_basic():
    # 2^63 * 2 = 2^64 → high 64 = 1
    a = int_to_bits(0x8000000000000000, 64)
    b = int_to_bits(2, 64)
    r = umulh64(a, b)
    assert bits_to_int(r) == 1


def test_smulh64_neg1_sq():
    a = int_to_bits(0xFFFFFFFFFFFFFFFF, 64)
    r = smulh64(a, a)
    assert bits_to_int(r) == 0   # (-1)^2 = +1, high bits = 0


# ── udiv64 / sdiv64 ──────────────────────────────────────────────────────────


def test_udiv64_basic():
    a = int_to_bits(100, 64)
    b = int_to_bits(7, 64)
    r = udiv64(a, b)
    assert bits_to_int(r) == 14


def test_udiv64_zero():
    a = int_to_bits(5, 64)
    b = int_to_bits(0, 64)
    r = udiv64(a, b)
    assert bits_to_int(r) == 0


def test_sdiv64_positive():
    a = int_to_bits(21, 64)
    b = int_to_bits(7, 64)
    r = sdiv64(a, b)
    assert bits_to_int(r) == 3


def test_sdiv64_negative():
    a = int_to_bits(-21 & 0xFFFFFFFFFFFFFFFF, 64)
    b = int_to_bits(7, 64)
    r = sdiv64(a, b)
    val = bits_to_int(r)
    if val >= 0x8000000000000000:
        val -= 0x10000000000000000
    assert val == -3


def test_sdiv64_zero_divisor():
    a = int_to_bits(100, 64)
    b = int_to_bits(0, 64)
    r = sdiv64(a, b)
    assert bits_to_int(r) == 0
