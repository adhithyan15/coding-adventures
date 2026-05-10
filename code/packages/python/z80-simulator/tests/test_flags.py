"""Unit tests for z80_simulator.flags — pure flag computation helpers.

These tests are independent of the full simulator.  They verify the
mathematical correctness of each helper in isolation.
"""


from z80_simulator.flags import (
    compute_half_carry_add,
    compute_half_carry_sub,
    compute_overflow_add,
    compute_overflow_sub,
    compute_parity,
    compute_sz,
    daa,
    pack_f,
    unpack_f,
)

# ── compute_sz ────────────────────────────────────────────────────────────────

class TestComputeSZ:
    def test_zero(self):
        s, z = compute_sz(0x00)
        assert s is False
        assert z is True

    def test_positive(self):
        s, z = compute_sz(0x42)
        assert s is False
        assert z is False

    def test_negative(self):
        s, z = compute_sz(0xFF)
        assert s is True
        assert z is False

    def test_min_negative(self):  # 0x80 = -128 in two's complement
        s, z = compute_sz(0x80)
        assert s is True
        assert z is False

    def test_masks_to_8_bits(self):
        # Value 0x100 should be treated as 0x00
        s, z = compute_sz(0x100)
        assert s is False
        assert z is True


# ── compute_parity ────────────────────────────────────────────────────────────

class TestComputeParity:
    def test_zero_has_even_parity(self):
        assert compute_parity(0x00) is True   # 0 set bits → even

    def test_one_bit_odd(self):
        assert compute_parity(0x01) is False  # 1 set bit

    def test_two_bits_even(self):
        assert compute_parity(0x03) is True   # 2 set bits

    def test_all_bits_even(self):
        assert compute_parity(0xFF) is True   # 8 set bits

    def test_single_high_bit_odd(self):
        assert compute_parity(0x80) is False  # 1 set bit

    def test_half_byte_even(self):
        assert compute_parity(0x0F) is True   # 4 set bits

    def test_masks_to_8_bits(self):
        # 0x101 = 0b100000001 → masked to 0x01 → odd
        assert compute_parity(0x101) is False


# ── compute_overflow_add ──────────────────────────────────────────────────────

class TestComputeOverflowAdd:
    def test_no_overflow_positive(self):
        # 1 + 1 = 2: no overflow
        assert compute_overflow_add(0x01, 0x01, 0x02) is False

    def test_overflow_positive_to_negative(self):
        # 0x7F + 0x01 = 0x80 (127 + 1 → -128): overflow!
        assert compute_overflow_add(0x7F, 0x01, 0x80) is True

    def test_overflow_negative_to_positive(self):
        # 0x80 + 0x80 = 0x00 (-128 + -128 → 0): overflow!
        assert compute_overflow_add(0x80, 0x80, 0x00) is True

    def test_no_overflow_negative(self):
        # 0xFF + 0xFF = 0xFE (-1 + -1 = -2): no overflow
        assert compute_overflow_add(0xFF, 0xFF, 0xFE) is False

    def test_no_overflow_mixed(self):
        # 5 + (-3) = 2: no overflow
        assert compute_overflow_add(0x05, 0xFD, 0x02) is False


# ── compute_overflow_sub ──────────────────────────────────────────────────────

class TestComputeOverflowSub:
    def test_no_overflow(self):
        # 5 - 3 = 2: no overflow
        assert compute_overflow_sub(0x05, 0x03, 0x02) is False

    def test_overflow_negative_minus_positive(self):
        # 0x80 - 0x01 = 0x7F (-128 - 1 → +127): overflow!
        assert compute_overflow_sub(0x80, 0x01, 0x7F) is True

    def test_overflow_positive_minus_negative(self):
        # 0x7F - 0xFF = 0x80 (127 - (-1) → -128): overflow!
        assert compute_overflow_sub(0x7F, 0xFF, 0x80) is True

    def test_no_overflow_zero_result(self):
        # 5 - 5 = 0
        assert compute_overflow_sub(0x05, 0x05, 0x00) is False


# ── compute_half_carry_add ────────────────────────────────────────────────────

class TestComputeHalfCarryAdd:
    def test_half_carry_occurs(self):
        # 0x0F + 0x01 = 0x10: carry from bit 3→4
        assert compute_half_carry_add(0x0F, 0x01) is True

    def test_no_half_carry(self):
        # 0x07 + 0x08 = 0x0F: sum is exactly 0x0F, no carry
        assert compute_half_carry_add(0x07, 0x08) is False

    def test_half_carry_with_carry_in(self):
        # 0x0F + 0x00 + 1 = 0x10: carry from bit 3→4
        assert compute_half_carry_add(0x0F, 0x00, 1) is True

    def test_no_half_carry_large_values(self):
        # 0xF0 + 0x10: low nibbles 0+0=0, no half-carry
        assert compute_half_carry_add(0xF0, 0x10) is False


# ── compute_half_carry_sub ────────────────────────────────────────────────────

class TestComputeHalfCarrySub:
    def test_half_borrow(self):
        # 0x10 - 0x01: low nibble 0 < 1, need to borrow
        assert compute_half_carry_sub(0x10, 0x01) is True

    def test_no_half_borrow(self):
        # 0x1F - 0x0F: low nibble 0xF >= 0xF, no borrow
        assert compute_half_carry_sub(0x1F, 0x0F) is False

    def test_half_borrow_zero(self):
        # 0x00 - 0x01: 0 < 1
        assert compute_half_carry_sub(0x00, 0x01) is True

    def test_with_borrow_in(self):
        # 0x10 - 0x00 - 1: 0 < 0+1=1, borrow
        assert compute_half_carry_sub(0x10, 0x00, 1) is True


# ── pack_f / unpack_f ─────────────────────────────────────────────────────────

class TestPackUnpackF:
    def test_all_zero(self):
        f = pack_f(False, False, False, False, False, False)
        assert f == 0x00

    def test_all_one(self):
        f = pack_f(True, True, True, True, True, True)
        # S=bit7, Z=bit6, H=bit4, PV=bit2, N=bit1, C=bit0
        # = 0b11010111 = 0xD7
        assert f == 0xD7

    def test_only_carry(self):
        f = pack_f(False, False, False, False, False, True)
        assert f == 0x01

    def test_only_zero(self):
        f = pack_f(False, True, False, False, False, False)
        assert f == 0x40

    def test_roundtrip(self):
        original = (True, False, True, False, True, False)
        f = pack_f(*original)
        result = unpack_f(f)
        assert result == original

    def test_roundtrip_all_flags(self):
        for s in (True, False):
            for z in (True, False):
                for h in (True, False):
                    for pv in (True, False):
                        for n in (True, False):
                            for c in (True, False):
                                flags = (s, z, h, pv, n, c)
                                assert unpack_f(pack_f(*flags)) == flags

    def test_unpack_known_byte(self):
        # Z=1, PV=1 → bits 6 and 2 set = 0x44
        s, z, h, pv, n, c = unpack_f(0x44)
        assert s is False
        assert z is True
        assert h is False
        assert pv is True
        assert n is False
        assert c is False


# ── daa ───────────────────────────────────────────────────────────────────────

class TestDAA:
    def test_add_no_correction_needed(self):
        # BCD 4 + 3 = 7; no nibble overflow
        new_a, new_h, new_pv, new_c = daa(0x07, False, False, False)
        assert new_a == 0x07
        assert new_c is False

    def test_add_low_nibble_correction(self):
        # BCD 5 + 5 = 0x0A; low nibble > 9 → add 6 → 0x10 (BCD 10)
        new_a, new_h, new_pv, new_c = daa(0x0A, False, False, False)
        assert new_a == 0x10
        assert new_c is False

    def test_add_high_nibble_correction(self):
        # BCD 50 + 60 = 0xB0; high nibble > 9 → add 0x60 → 0x10 with carry
        new_a, new_h, new_pv, new_c = daa(0xB0, False, False, False)
        assert new_a == 0x10
        assert new_c is True

    def test_add_both_corrections(self):
        # Result 0x9A → add 0x66 → 0x00 with carry (BCD 100)
        new_a, new_h, new_pv, new_c = daa(0x9A, False, False, False)
        assert new_a == 0x00
        assert new_c is True

    def test_add_with_carry_flag(self):
        # C=1 already set → always add 0x60
        new_a, new_h, new_pv, new_c = daa(0x05, False, False, True)
        assert new_a == 0x65
        assert new_c is True

    def test_sub_no_correction(self):
        # BCD 9 - 3 = 6; no correction
        new_a, new_h, new_pv, new_c = daa(0x06, True, False, False)
        assert new_a == 0x06
        assert new_c is False

    def test_sub_with_half_carry(self):
        # After sub with H=1: subtract 6 from low nibble
        new_a, new_h, new_pv, new_c = daa(0x0F, True, True, False)
        assert new_a == 0x09
        assert new_c is False
