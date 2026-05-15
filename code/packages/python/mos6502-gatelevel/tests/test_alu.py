"""Tests for mos6502_gatelevel.alu — gate-level ALU operations."""

from __future__ import annotations

import pytest

from mos6502_gatelevel.alu import (
    ALUResult6502,
    add8,
    asl8,
    and8,
    bit8,
    compare8,
    daa_adc,
    daa_sbc,
    dec8,
    inc8,
    lsr8,
    or8,
    rol8,
    ror8,
    sub8,
    xor8,
)


# ── ALUResult6502 ─────────────────────────────────────────────────────────────

class TestALUResult6502:
    def test_fields(self):
        r = ALUResult6502(result=42, flag_n=0, flag_v=0, flag_z=0, flag_c=1)
        assert r.result == 42
        assert r.flag_c == 1

    def test_is_dataclass(self):
        from dataclasses import fields
        names = {f.name for f in fields(ALUResult6502)}
        assert names == {"result", "flag_n", "flag_v", "flag_z", "flag_c"}


# ── add8 ──────────────────────────────────────────────────────────────────────

class TestAdd8:
    def test_basic(self):
        r = add8(5, 3, 0)
        assert r.result == 8
        assert r.flag_n == 0
        assert r.flag_z == 0
        assert r.flag_c == 0
        assert r.flag_v == 0

    def test_zero_result(self):
        r = add8(0, 0, 0)
        assert r.result == 0
        assert r.flag_z == 1
        assert r.flag_n == 0

    def test_carry_out(self):
        r = add8(0xFF, 0x01, 0)
        assert r.result == 0
        assert r.flag_c == 1
        assert r.flag_z == 1

    def test_carry_in(self):
        r = add8(0xFF, 0x00, 1)
        assert r.result == 0
        assert r.flag_c == 1

    def test_negative_flag(self):
        r = add8(0x40, 0x40, 0)
        assert r.result == 0x80
        assert r.flag_n == 1

    def test_overflow_positive_to_negative(self):
        # 0x7F + 0x01 = 0x80 (127 + 1 = 128 = -128 in signed → overflow)
        r = add8(0x7F, 0x01, 0)
        assert r.result == 0x80
        assert r.flag_v == 1
        assert r.flag_n == 1

    def test_overflow_negative_to_positive(self):
        # 0x80 + 0x80 = 0x100 → 0x00 (-128 + -128 = 0 → overflow)
        r = add8(0x80, 0x80, 0)
        assert r.flag_v == 1

    def test_no_overflow_ff_plus_1(self):
        # 0xFF + 0x01 = 0x00 (unsigned overflow, but no signed overflow)
        r = add8(0xFF, 0x01, 0)
        assert r.flag_v == 0   # -1 + 1 = 0, no signed overflow

    def test_adc_carry_propagation(self):
        # ADC with carry: 0xFF + 0x00 + 1 = 0x100 → carry out
        r = add8(0xFF, 0x00, 1)
        assert r.result == 0
        assert r.flag_c == 1

    def test_all_values_match_python(self):
        for a in range(0, 256, 31):
            for b in range(0, 256, 31):
                r = add8(a, b, 0)
                expected = (a + b) & 0xFF
                assert r.result == expected
                assert r.flag_c == int((a + b) > 0xFF)
                assert r.flag_z == int(expected == 0)
                assert r.flag_n == int((expected & 0x80) != 0)


# ── sub8 ──────────────────────────────────────────────────────────────────────

class TestSub8:
    def test_basic_subtraction(self):
        # 10 - 3 = 7 (with C=1, no borrow)
        r = sub8(10, 3, 1)
        assert r.result == 7
        assert r.flag_c == 1   # No borrow
        assert r.flag_z == 0
        assert r.flag_n == 0

    def test_equal_values(self):
        r = sub8(5, 5, 1)
        assert r.result == 0
        assert r.flag_z == 1
        assert r.flag_c == 1   # A >= M → no borrow

    def test_borrow(self):
        # 3 - 5 = -2 = 0xFE with borrow
        r = sub8(3, 5, 1)
        assert r.result == 0xFE
        assert r.flag_c == 0   # Borrow occurred
        assert r.flag_n == 1

    def test_zero_minus_one(self):
        r = sub8(0, 1, 1)
        assert r.result == 0xFF
        assert r.flag_c == 0   # Borrow

    def test_with_borrow_in(self):
        # A - B - 1: C=0 means "subtract an extra 1"
        r = sub8(10, 3, 0)
        assert r.result == 6   # 10 - 3 - 1 = 6
        assert r.flag_c == 1

    def test_overflow_positive(self):
        # 0x50 - 0xB0: positive - negative = negative → overflow
        r = sub8(0x50, 0xB0, 1)
        assert r.flag_v == 1

    def test_no_overflow_same_sign(self):
        r = sub8(0x50, 0x10, 1)
        assert r.flag_v == 0
        assert r.result == 0x40

    def test_sbc_matches_behavioral(self):
        # SBC is A + NOT(B) + C; verify against expected values
        cases = [
            (0x50, 0x10, 1, 0x40),
            (0x10, 0x50, 1, 0xC0),
            (0xFF, 0x01, 1, 0xFE),
        ]
        for a, b, c, expected in cases:
            r = sub8(a, b, c)
            assert r.result == expected, f"sub8({a},{b},{c}) expected {expected} got {r.result}"


# ── and8 ──────────────────────────────────────────────────────────────────────

class TestAnd8:
    def test_basic(self):
        r = and8(0b11001100, 0b10101010)
        assert r.result == 0b10001000

    def test_zero_result(self):
        r = and8(0x0F, 0xF0)
        assert r.result == 0
        assert r.flag_z == 1

    def test_all_ones(self):
        r = and8(0xFF, 0xFF)
        assert r.result == 0xFF
        assert r.flag_n == 1

    def test_mask(self):
        r = and8(0xAB, 0x0F)
        assert r.result == 0x0B

    def test_flags_not_affect_v_c(self):
        # AND doesn't change V or C — alu returns 0 for them
        r = and8(0xFF, 0xFF)
        assert r.flag_v == 0
        assert r.flag_c == 0

    def test_negative_flag(self):
        r = and8(0xFF, 0x80)
        assert r.flag_n == 1

    def test_commutative(self):
        for a, b in [(0xAA, 0x55), (0xFF, 0x0F), (0x12, 0x34)]:
            assert and8(a, b).result == and8(b, a).result


# ── or8 ───────────────────────────────────────────────────────────────────────

class TestOr8:
    def test_basic(self):
        r = or8(0b11000000, 0b00001111)
        assert r.result == 0b11001111

    def test_zero_or_zero(self):
        r = or8(0, 0)
        assert r.result == 0
        assert r.flag_z == 1

    def test_all_ones(self):
        r = or8(0xFF, 0x00)
        assert r.result == 0xFF
        assert r.flag_n == 1

    def test_commutative(self):
        for a, b in [(0xAA, 0x55), (0x12, 0x34)]:
            assert or8(a, b).result == or8(b, a).result

    def test_flags(self):
        r = or8(0x80, 0x01)
        assert r.flag_n == 1
        assert r.flag_z == 0
        assert r.flag_v == 0
        assert r.flag_c == 0


# ── xor8 ──────────────────────────────────────────────────────────────────────

class TestXor8:
    def test_basic(self):
        r = xor8(0xFF, 0xFF)
        assert r.result == 0
        assert r.flag_z == 1

    def test_identity(self):
        r = xor8(0xAB, 0x00)
        assert r.result == 0xAB

    def test_toggle_bit(self):
        r = xor8(0xFF, 0x0F)
        assert r.result == 0xF0

    def test_self_xor_is_zero(self):
        for v in [0, 1, 0x55, 0xAA, 0xFF]:
            assert xor8(v, v).result == 0
            assert xor8(v, v).flag_z == 1

    def test_negative_flag(self):
        r = xor8(0x80, 0x01)
        assert r.flag_n == 1

    def test_commutative(self):
        for a, b in [(0x12, 0x34), (0xAA, 0x55)]:
            assert xor8(a, b).result == xor8(b, a).result


# ── asl8 ──────────────────────────────────────────────────────────────────────

class TestAsl8:
    def test_basic(self):
        result, carry = asl8(0b00000001)
        assert result == 0b00000010
        assert carry == 0

    def test_carry_out(self):
        result, carry = asl8(0b10000001)
        assert result == 0b00000010
        assert carry == 1

    def test_zero(self):
        result, carry = asl8(0)
        assert result == 0
        assert carry == 0

    def test_ff(self):
        result, carry = asl8(0xFF)
        assert result == 0xFE
        assert carry == 1

    def test_multiply_by_two(self):
        for v in [1, 2, 4, 8, 16, 32, 64]:
            result, _ = asl8(v)
            assert result == v * 2

    def test_0x80_carries(self):
        result, carry = asl8(0x80)
        assert result == 0
        assert carry == 1


# ── lsr8 ──────────────────────────────────────────────────────────────────────

class TestLsr8:
    def test_basic(self):
        result, carry = lsr8(0b00000010)
        assert result == 0b00000001
        assert carry == 0

    def test_lsb_to_carry(self):
        result, carry = lsr8(0b00000011)
        assert result == 0b00000001
        assert carry == 1

    def test_zero(self):
        result, carry = lsr8(0)
        assert result == 0
        assert carry == 0

    def test_msb_clears(self):
        result, carry = lsr8(0xFF)
        assert result == 0x7F
        assert carry == 1

    def test_halves_even(self):
        for v in [2, 4, 8, 16, 32, 64, 128]:
            result, carry = lsr8(v)
            assert result == v // 2
            assert carry == 0


# ── rol8 ──────────────────────────────────────────────────────────────────────

class TestRol8:
    def test_rotate_with_carry_zero(self):
        result, carry = rol8(0b10000000, 0)
        assert result == 0b00000000
        assert carry == 1

    def test_carry_enters_lsb(self):
        result, carry = rol8(0b00000000, 1)
        assert result == 0b00000001
        assert carry == 0

    def test_full_rotation_9_steps(self):
        val = 0b10000000
        c = 0
        for _ in range(9):
            val, c = rol8(val, c)
        # After 9 rotations through carry, back to original
        assert val == 0b10000000

    def test_msb_to_carry(self):
        result, carry = rol8(0xFF, 0)
        assert result == 0xFE
        assert carry == 1


# ── ror8 ──────────────────────────────────────────────────────────────────────

class TestRor8:
    def test_rotate_with_carry_zero(self):
        result, carry = ror8(0b00000001, 0)
        assert result == 0b00000000
        assert carry == 1

    def test_carry_enters_msb(self):
        result, carry = ror8(0b00000000, 1)
        assert result == 0b10000000
        assert carry == 0

    def test_lsb_to_carry(self):
        _, carry = ror8(0b11111111, 0)
        assert carry == 1

    def test_full_rotation_9_steps(self):
        val = 0b00000001
        c = 0
        for _ in range(9):
            val, c = ror8(val, c)
        assert val == 0b00000001


# ── inc8 ──────────────────────────────────────────────────────────────────────

class TestInc8:
    def test_basic(self):
        r = inc8(0)
        assert r.result == 1
        assert r.flag_z == 0
        assert r.flag_n == 0

    def test_wraparound(self):
        r = inc8(0xFF)
        assert r.result == 0
        assert r.flag_z == 1

    def test_negative_result(self):
        r = inc8(0x7F)
        assert r.result == 0x80
        assert r.flag_n == 1


# ── dec8 ──────────────────────────────────────────────────────────────────────

class TestDec8:
    def test_basic(self):
        r = dec8(5)
        assert r.result == 4
        assert r.flag_z == 0
        assert r.flag_n == 0

    def test_wraparound(self):
        r = dec8(0)
        assert r.result == 0xFF
        assert r.flag_n == 1

    def test_to_zero(self):
        r = dec8(1)
        assert r.result == 0
        assert r.flag_z == 1


# ── compare8 ─────────────────────────────────────────────────────────────────

class TestCompare8:
    def test_equal(self):
        n, z, c = compare8(5, 5)
        assert z == 1
        assert n == 0
        assert c == 1   # 5 >= 5: no borrow

    def test_greater(self):
        n, z, c = compare8(10, 5)
        assert z == 0
        assert n == 0
        assert c == 1

    def test_less(self):
        n, z, c = compare8(3, 5)
        assert z == 0
        assert c == 0   # 3 < 5: borrow

    def test_zero_vs_ff(self):
        n, z, c = compare8(0, 0xFF)
        assert c == 0   # 0 < 255: borrow
        assert z == 0

    def test_ff_vs_zero(self):
        n, z, c = compare8(0xFF, 0)
        assert c == 1
        assert z == 0

    def test_all_same(self):
        for v in range(256):
            n, z, c = compare8(v, v)
            assert z == 1
            assert c == 1

    def test_comparison_consistent(self):
        for a in range(0, 256, 17):
            for b in range(0, 256, 17):
                n, z, c = compare8(a, b)
                diff = (a - b) & 0xFF
                assert z == int(diff == 0)
                assert n == int((diff & 0x80) != 0)
                assert c == int(a >= b)


# ── bit8 ─────────────────────────────────────────────────────────────────────

class TestBit8:
    def test_n_from_m7(self):
        flag_n, flag_v, flag_z = bit8(0xFF, 0x80)
        assert flag_n == 1   # M[7] = 1

    def test_v_from_m6(self):
        flag_n, flag_v, flag_z = bit8(0xFF, 0x40)
        assert flag_v == 1   # M[6] = 1
        assert flag_n == 0   # M[7] = 0

    def test_z_set_when_and_is_zero(self):
        flag_n, flag_v, flag_z = bit8(0x0F, 0xF0)
        assert flag_z == 1   # A & M = 0

    def test_z_clear_when_and_nonzero(self):
        flag_n, flag_v, flag_z = bit8(0xFF, 0x01)
        assert flag_z == 0   # A & M = 0x01 ≠ 0

    def test_all_bits_m_set(self):
        flag_n, flag_v, flag_z = bit8(0xFF, 0xFF)
        assert flag_n == 1
        assert flag_v == 1
        assert flag_z == 0   # A & M = 0xFF

    def test_m_zero(self):
        flag_n, flag_v, flag_z = bit8(0xFF, 0x00)
        assert flag_n == 0
        assert flag_v == 0
        assert flag_z == 1   # A & 0 = 0


# ── daa_adc ──────────────────────────────────────────────────────────────────

class TestDaaAdc:
    def test_binary_mode(self):
        r = daa_adc(5, 3, 0, 0)
        assert r.result == 8
        assert r.flag_c == 0

    def test_bcd_nine_plus_one(self):
        # 09 + 01 = 10 in BCD
        r = daa_adc(0x09, 0x01, 0, 1)
        assert r.result == 0x10
        assert r.flag_c == 0

    def test_bcd_carry(self):
        # 99 + 01 = 100 in BCD → 00 with carry
        r = daa_adc(0x99, 0x01, 0, 1)
        assert r.result == 0x00
        assert r.flag_c == 1

    def test_bcd_five_plus_five(self):
        r = daa_adc(0x05, 0x05, 0, 1)
        assert r.result == 0x10

    def test_bcd_nmos_flags_from_binary(self):
        # NMOS: N/V/Z from binary, not BCD result
        r = daa_adc(0x09, 0x01, 0, 1)
        # Binary result is 0x0A = 10; N=0, Z=0
        assert r.flag_n == 0
        assert r.flag_z == 0

    def test_bcd_with_carry_in(self):
        r = daa_adc(0x09, 0x00, 1, 1)
        assert r.result == 0x10

    def test_bcd_high_nibble_correction(self):
        r = daa_adc(0x50, 0x50, 0, 1)
        # 50 + 50 = 100 in BCD
        assert r.result == 0x00
        assert r.flag_c == 1


# ── daa_sbc ──────────────────────────────────────────────────────────────────

class TestDaaSbc:
    def test_binary_mode(self):
        r = daa_sbc(10, 3, 1, 0)
        assert r.result == 7
        assert r.flag_c == 1

    def test_bcd_ten_minus_one(self):
        # 10 - 01 = 09 in BCD (C=1 = no borrow)
        r = daa_sbc(0x10, 0x01, 1, 1)
        assert r.result == 0x09
        assert r.flag_c == 1

    def test_bcd_zero_minus_one_borrow(self):
        # 00 - 01 = 99 in BCD with borrow
        r = daa_sbc(0x00, 0x01, 1, 1)
        assert r.result == 0x99
        assert r.flag_c == 0   # Borrow occurred

    def test_bcd_nmos_flags_from_binary(self):
        # NMOS: N/V/Z from binary subtract
        r = daa_sbc(0x10, 0x01, 1, 1)
        # Binary: 0x10 - 0x01 = 0x0F; N=0, Z=0
        assert r.flag_n == 0
        assert r.flag_z == 0
