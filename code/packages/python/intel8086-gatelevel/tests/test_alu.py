"""Tests for alu.py — gate-level ALU operations."""

import pytest

from intel8086_gatelevel.alu import (
    aaa,
    aad,
    aam,
    aas,
    add16,
    add8,
    and16,
    and8,
    daa,
    das,
    dec16,
    dec8,
    div16,
    div8,
    idiv16,
    idiv8,
    imul16,
    imul8,
    inc16,
    inc8,
    mul16,
    mul8,
    neg16,
    neg8,
    not16,
    not8,
    or16,
    or8,
    rcl,
    rcr,
    rol,
    ror,
    sar,
    shl,
    shr,
    sub16,
    sub8,
    xor16,
    xor8,
)


# ── add16 ─────────────────────────────────────────────────────────────────────

class TestAdd16:
    def test_simple(self):
        r = add16(5, 3)
        assert r.result == 8
        assert r.flag_cf == 0
        assert r.flag_zf == 0
        assert r.flag_sf == 0

    def test_zero_result(self):
        r = add16(0, 0)
        assert r.result == 0
        assert r.flag_zf == 1

    def test_carry_out(self):
        r = add16(0xFFFF, 1)
        assert r.result == 0
        assert r.flag_cf == 1
        assert r.flag_zf == 1

    def test_overflow_positive(self):
        # 0x7FFF + 1 = 0x8000: signed overflow
        r = add16(0x7FFF, 1)
        assert r.result == 0x8000
        assert r.flag_of == 1
        assert r.flag_sf == 1

    def test_overflow_negative(self):
        # 0x8000 + 0x8000 = 0: overflow (neg + neg = pos in signed)
        r = add16(0x8000, 0x8000)
        assert r.result == 0
        assert r.flag_of == 1
        assert r.flag_cf == 1

    def test_no_overflow(self):
        # 1 + 1: no overflow
        r = add16(1, 1)
        assert r.flag_of == 0

    def test_adc_with_carry(self):
        r = add16(0xFFFF, 0, 1)
        assert r.result == 0
        assert r.flag_cf == 1

    def test_sign_flag(self):
        r = add16(0x7000, 0x1000)
        assert r.flag_sf == 1  # bit 15 set

    def test_parity(self):
        r = add16(0, 3)  # result=3=0b11 → 2 ones → even → PF=1
        assert r.flag_pf == 1

    def test_aux_carry(self):
        r = add16(0x000F, 0x0001)
        assert r.flag_af == 1

    def test_no_aux_carry(self):
        r = add16(0x0001, 0x0001)
        assert r.flag_af == 0


# ── sub16 ─────────────────────────────────────────────────────────────────────

class TestSub16:
    def test_simple(self):
        r = sub16(10, 3)
        assert r.result == 7
        assert r.flag_cf == 0  # no borrow

    def test_equal(self):
        r = sub16(5, 5)
        assert r.result == 0
        assert r.flag_zf == 1
        assert r.flag_cf == 0

    def test_borrow(self):
        r = sub16(0, 1)
        assert r.result == 0xFFFF
        assert r.flag_cf == 1  # borrow

    def test_overflow_positive(self):
        # 0x8000 - 1 = 0x7FFF: neg - pos = pos → overflow
        r = sub16(0x8000, 1)
        assert r.result == 0x7FFF
        assert r.flag_of == 1

    def test_overflow_negative(self):
        # 0x7FFF - 0x8000: pos - neg = neg → overflow
        r = sub16(0x7FFF, 0x8000)
        assert r.flag_of == 1

    def test_no_overflow(self):
        r = sub16(10, 5)
        assert r.flag_of == 0

    def test_sbb_with_borrow(self):
        # a - b - 1
        r = sub16(5, 3, 1)
        assert r.result == 1

    def test_sign_flag(self):
        r = sub16(0, 1)
        assert r.flag_sf == 1  # negative result

    def test_neg16_via_sub(self):
        r = sub16(0, 0x1234)
        assert r.result == (0x10000 - 0x1234) & 0xFFFF


# ── and16, or16, xor16 ────────────────────────────────────────────────────────

class TestLogic16:
    def test_and16_basic(self):
        r = and16(0xFF00, 0x0FF0)
        assert r.result == 0x0F00
        assert r.flag_cf == 0
        assert r.flag_of == 0
        assert r.flag_af == 0

    def test_and16_zero(self):
        r = and16(0xAAAA, 0x5555)
        assert r.result == 0
        assert r.flag_zf == 1

    def test_or16_basic(self):
        r = or16(0xFF00, 0x00FF)
        assert r.result == 0xFFFF
        assert r.flag_sf == 1

    def test_or16_zero(self):
        r = or16(0, 0)
        assert r.flag_zf == 1

    def test_xor16_basic(self):
        r = xor16(0xAAAA, 0x5555)
        assert r.result == 0xFFFF

    def test_xor16_same(self):
        r = xor16(0x1234, 0x1234)
        assert r.result == 0
        assert r.flag_zf == 1

    def test_xor16_cf_of_zero(self):
        r = xor16(0x1234, 0x5678)
        assert r.flag_cf == 0
        assert r.flag_of == 0


# ── inc16 / dec16 ─────────────────────────────────────────────────────────────

class TestIncDec16:
    def test_inc16_simple(self):
        r = inc16(5)
        assert r.result == 6
        assert r.flag_zf == 0

    def test_inc16_zero(self):
        r = inc16(0)
        assert r.result == 1

    def test_inc16_overflow_7fff(self):
        r = inc16(0x7FFF)
        assert r.result == 0x8000
        assert r.flag_of == 1

    def test_inc16_wrap_ffff(self):
        r = inc16(0xFFFF)
        assert r.result == 0
        assert r.flag_zf == 1

    def test_inc16_no_overflow_normal(self):
        r = inc16(0x1234)
        assert r.flag_of == 0

    def test_dec16_simple(self):
        r = dec16(5)
        assert r.result == 4

    def test_dec16_to_zero(self):
        r = dec16(1)
        assert r.result == 0
        assert r.flag_zf == 1

    def test_dec16_overflow_8000(self):
        r = dec16(0x8000)
        assert r.result == 0x7FFF
        assert r.flag_of == 1

    def test_dec16_wrap_0(self):
        r = dec16(0)
        assert r.result == 0xFFFF


# ── neg16, not16 ──────────────────────────────────────────────────────────────

class TestNegNot16:
    def test_neg16_one(self):
        r = neg16(1)
        assert r.result == 0xFFFF

    def test_neg16_zero(self):
        r = neg16(0)
        assert r.result == 0
        assert r.flag_cf == 0

    def test_neg16_nonzero_sets_cf(self):
        r = neg16(5)
        assert r.flag_cf == 1

    def test_not16_zero(self):
        assert not16(0) == 0xFFFF

    def test_not16_ffff(self):
        assert not16(0xFFFF) == 0

    def test_not16_aaaa(self):
        assert not16(0xAAAA) == 0x5555


# ── 8-bit arithmetic ──────────────────────────────────────────────────────────

class TestAdd8:
    def test_simple(self):
        r = add8(5, 3)
        assert r.result == 8

    def test_carry_out(self):
        r = add8(0xFF, 1)
        assert r.result == 0
        assert r.flag_cf == 1

    def test_overflow(self):
        r = add8(0x7F, 1)
        assert r.flag_of == 1

    def test_zero_flag(self):
        r = add8(0, 0)
        assert r.flag_zf == 1

    def test_sign_flag(self):
        r = add8(0x40, 0x40)
        assert r.flag_sf == 1


class TestSub8:
    def test_simple(self):
        r = sub8(10, 3)
        assert r.result == 7
        assert r.flag_cf == 0

    def test_borrow(self):
        r = sub8(0, 1)
        assert r.result == 0xFF
        assert r.flag_cf == 1

    def test_overflow_8bit(self):
        # 0x80 - 1 = 0x7F: signed overflow (neg - pos = pos)
        r = sub8(0x80, 1)
        assert r.flag_of == 1


class TestAnd8Or8Xor8:
    def test_and8(self):
        r = and8(0xF0, 0x0F)
        assert r.result == 0
        assert r.flag_zf == 1

    def test_or8(self):
        r = or8(0xF0, 0x0F)
        assert r.result == 0xFF

    def test_xor8(self):
        r = xor8(0xFF, 0xFF)
        assert r.result == 0
        assert r.flag_zf == 1

    def test_and8_cf_of_zero(self):
        r = and8(0x1, 0x1)
        assert r.flag_cf == 0
        assert r.flag_of == 0


class TestIncDec8:
    def test_inc8_simple(self):
        r = inc8(5)
        assert r.result == 6

    def test_inc8_overflow_7f(self):
        r = inc8(0x7F)
        assert r.result == 0x80
        assert r.flag_of == 1

    def test_dec8_simple(self):
        r = dec8(5)
        assert r.result == 4

    def test_dec8_overflow_80(self):
        r = dec8(0x80)
        assert r.flag_of == 1


class TestNegNot8:
    def test_neg8_one(self):
        r = neg8(1)
        assert r.result == 0xFF

    def test_not8_zero(self):
        assert not8(0) == 0xFF

    def test_not8_ff(self):
        assert not8(0xFF) == 0


# ── Shifts and rotates ────────────────────────────────────────────────────────

class TestShl:
    def test_shl_by_1(self):
        result, cf = shl(1, 1, 8)
        assert result == 2
        assert cf == 0

    def test_shl_cf_set(self):
        result, cf = shl(0x80, 1, 8)
        assert result == 0
        assert cf == 1

    def test_shl_by_0(self):
        result, cf = shl(0xFF, 0, 8)
        assert result == 0xFF

    def test_shl_16bit(self):
        result, cf = shl(0x8000, 1, 16)
        assert result == 0
        assert cf == 1

    def test_shl_by_4(self):
        result, cf = shl(0x0F, 4, 8)
        assert result == 0xF0
        assert cf == 0


class TestShr:
    def test_shr_by_1(self):
        result, cf = shr(2, 1, 8)
        assert result == 1
        assert cf == 0

    def test_shr_cf_set(self):
        result, cf = shr(1, 1, 8)
        assert result == 0
        assert cf == 1

    def test_shr_16bit(self):
        result, cf = shr(0x8000, 1, 16)
        assert result == 0x4000
        assert cf == 0


class TestSar:
    def test_sar_negative(self):
        result, cf = sar(0x80, 1, 8)
        assert result == 0xC0   # Sign bit preserved

    def test_sar_positive(self):
        result, cf = sar(0x40, 1, 8)
        assert result == 0x20
        assert cf == 0

    def test_sar_with_cf(self):
        result, cf = sar(0x81, 1, 8)
        assert cf == 1

    def test_sar_16bit_negative(self):
        result, cf = sar(0x8000, 1, 16)
        assert result == 0xC000


class TestRol:
    def test_rol_basic(self):
        # ROL(0b10000000, 1): MSB wraps to bit 0 → result = 0b00000001, CF = new bit 0 = 1
        result, cf = rol(0b10000000, 1, 8, 0)
        assert result == 0b00000001
        assert cf == 1

    def test_rol_16bit(self):
        result, cf = rol(0x8000, 1, 16, 0)
        assert result == 0x0001
        assert cf == 1


class TestRor:
    def test_ror_basic(self):
        result, cf = ror(0x01, 1, 8, 0)
        assert result == 0x80
        assert cf == 1  # new MSB = 1

    def test_ror_16bit(self):
        result, cf = ror(0x0001, 1, 16, 0)
        assert result == 0x8000


class TestRcl:
    def test_rcl_carry_in(self):
        result, cf = rcl(0x00, 1, 8, 1)
        assert result == 1
        assert cf == 0

    def test_rcl_msb_to_carry(self):
        result, cf = rcl(0x80, 1, 8, 0)
        assert result == 0
        assert cf == 1


class TestRcr:
    def test_rcr_carry_in(self):
        result, cf = rcr(0x00, 1, 8, 1)
        assert result == 0x80
        assert cf == 0

    def test_rcr_lsb_to_carry(self):
        result, cf = rcr(0x01, 1, 8, 0)
        assert result == 0
        assert cf == 1


# ── MUL / IMUL ────────────────────────────────────────────────────────────────

class TestMul:
    def test_mul8_simple(self):
        ax, cf_of = mul8(5, 3)
        assert ax == 15
        assert cf_of == 0

    def test_mul8_overflow(self):
        ax, cf_of = mul8(0xFF, 0xFF)
        assert ax == 0xFE01
        assert cf_of == 1  # AH != 0

    def test_mul16_simple(self):
        dx, ax, cf_of = mul16(5, 3)
        assert ax == 15
        assert dx == 0
        assert cf_of == 0

    def test_mul16_overflow(self):
        dx, ax, cf_of = mul16(0xFFFF, 0xFFFF)
        assert cf_of == 1

    def test_imul8_positive(self):
        ax, cf_of = imul8(5, 3)
        assert ax & 0xFF == 15
        assert cf_of == 0

    def test_imul8_negative(self):
        # (-1) * 2 = -2 = 0xFFFE; AH=0xFF = sign-extension of AL=0xFE, so CF/OF=0
        ax, cf_of = imul8(0xFF, 2)  # 0xFF = -1 signed
        assert ax & 0xFFFF == 0xFFFE
        assert cf_of == 0  # result fits in sign-extended byte

    def test_imul8_overflow(self):
        # 127 * 2 = 254, but in signed 8-bit only goes to 127 → overflow
        ax, cf_of = imul8(0x7F, 2)   # 127 * 2 = 254 = 0x00FE
        assert ax & 0xFFFF == 0x00FE
        assert cf_of == 1  # AH=0x00, but AL has bit7=1 → expected_hi=0xFF ≠ AH

    def test_imul16_simple(self):
        dx, ax, cf_of = imul16(5, 3)
        assert ax == 15

    def test_imul16_negative(self):
        dx, ax, cf_of = imul16(0xFFFF, 2)  # -1 * 2 = -2
        assert ax == 0xFFFE
        assert dx == 0xFFFF  # sign extension


# ── DIV / IDIV ────────────────────────────────────────────────────────────────

class TestDiv:
    def test_div8_simple(self):
        q, r = div8(10, 3)
        assert q == 3
        assert r == 1

    def test_div8_exact(self):
        q, r = div8(10, 2)
        assert q == 5
        assert r == 0

    def test_div8_zero_raises(self):
        with pytest.raises(ZeroDivisionError):
            div8(10, 0)

    def test_div16_simple(self):
        q, r = div16(10, 3)
        assert q == 3
        assert r == 1

    def test_div16_large(self):
        # DX:AX = 0x0001:0x0000 = 65536; divide by 4 → quotient 0x4000
        q, r = div16(0x10000, 4)
        assert q == 0x4000
        assert r == 0

    def test_idiv8_simple(self):
        q, r = idiv8(10, 3)
        assert q & 0xFF == 3

    def test_idiv8_negative(self):
        # -10 / 3 = -3 remainder -1
        ax = (0xFFF6) & 0xFFFF  # -10 as 16-bit
        q, r = idiv8(ax, 3)
        assert (q & 0xFF) == ((-3) & 0xFF)

    def test_idiv16_simple(self):
        q, r = idiv16(10, 3)
        assert q & 0xFFFF == 3

    def test_div16_zero_raises(self):
        with pytest.raises(ZeroDivisionError):
            div16(10, 0)

    def test_idiv8_zero_raises(self):
        with pytest.raises(ZeroDivisionError):
            idiv8(10, 0)

    def test_idiv16_zero_raises(self):
        with pytest.raises(ZeroDivisionError):
            idiv16(10, 0)


# ── BCD operations ────────────────────────────────────────────────────────────

class TestBcd:
    def test_daa_no_correction(self):
        # AL=0x09, no AF, no CF → no correction
        al, af, cf = daa(0x09, 0, 0)
        assert al == 0x09
        assert af == 0
        assert cf == 0

    def test_daa_low_nibble_correction(self):
        # AL=0x0A → low nibble > 9 → add 6 → 0x10
        al, af, cf = daa(0x0A, 0, 0)
        assert al == 0x10
        assert af == 1

    def test_daa_high_nibble_correction(self):
        # AL=0x9A → > 0x99 → add 0x60
        al, af, cf = daa(0x9A, 0, 0)
        assert cf == 1

    def test_das_no_correction(self):
        al, af, cf = das(0x09, 0, 0)
        assert al == 0x09

    def test_aaa_correction(self):
        # AL=0x0F → digit correction
        al, ah, af_cf = aaa(0x0F, 0, 0)
        assert af_cf == 1

    def test_aaa_no_correction(self):
        al, ah, af_cf = aaa(0x05, 0, 0)
        assert af_cf == 0

    def test_aas_correction(self):
        al, ah, af_cf = aas(0x0F, 0, 0)
        assert af_cf == 1

    def test_aam_base10(self):
        # 25 / 10 = 2 remainder 5
        ah, al = aam(25)
        assert ah == 2
        assert al == 5

    def test_aam_zero(self):
        ah, al = aam(0)
        assert ah == 0
        assert al == 0

    def test_aad_base10(self):
        # AH=2, AL=5 → 2*10 + 5 = 25
        result = aad(2, 5)
        assert result == 25

    def test_aad_zero(self):
        assert aad(0, 0) == 0
