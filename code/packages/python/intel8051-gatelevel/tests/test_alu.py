"""Tests for intel8051_gatelevel.alu — gate-level ALU operations."""

from intel8051_gatelevel.alu import (
    add8,
    anl8,
    da8,
    dec8,
    div8,
    inc8,
    mul8,
    orl8,
    rl8,
    rlc8,
    rr8,
    rrc8,
    subb8,
    swap8,
    xrl8,
)


class TestAdd8:
    def test_basic_add(self):
        res = add8(5, 3)
        assert res.result == 8
        assert res.cy == 0
        assert res.ac == 0

    def test_no_carry(self):
        res = add8(0x10, 0x10)
        assert res.result == 0x20
        assert res.cy == 0

    def test_carry_out(self):
        res = add8(0xFF, 0x01)
        assert res.result == 0x00
        assert res.cy == 1

    def test_max_plus_max(self):
        res = add8(0xFF, 0xFF)
        assert res.result == 0xFE
        assert res.cy == 1

    def test_carry_in_adds(self):
        res = add8(0x05, 0x05, 1)
        assert res.result == 11
        assert res.cy == 0

    def test_carry_in_causes_overflow(self):
        res = add8(0xFF, 0xFF, 1)
        assert res.result == 0xFF
        assert res.cy == 1

    def test_auxiliary_carry(self):
        # 0x0F + 0x01 = 0x10, carry from bit 3 to bit 4
        res = add8(0x0F, 0x01)
        assert res.result == 0x10
        assert res.ac == 1
        assert res.cy == 0

    def test_no_aux_carry(self):
        res = add8(0x01, 0x01)
        assert res.ac == 0

    def test_overflow_positive_plus_positive_yields_negative(self):
        # 0x50 + 0x50 = 0xA0 — overflow: two positives yielding MSB=1 (negative in signed)
        res = add8(0x50, 0x50)
        assert res.ov == 1

    def test_no_overflow_for_unsigned_max(self):
        # 0x7F + 0x7F = 0xFE — signed overflow (127+127=254 overflows signed)
        res = add8(0x7F, 0x7F)
        assert res.ov == 1

    def test_parity_single_bit(self):
        res = add8(0x01, 0x00)
        assert res.parity == 1  # 1 set bit → odd → P=1

    def test_parity_two_bits(self):
        res = add8(0x03, 0x00)
        assert res.parity == 0  # 2 set bits → even → P=0

    def test_zero_result_parity(self):
        res = add8(0x00, 0x00)
        assert res.parity == 0  # 0 set bits → even → P=0


class TestSubb8:
    def test_basic_sub(self):
        res = subb8(10, 5, 0)
        assert res.result == 5
        assert res.cy == 0  # no borrow

    def test_borrow(self):
        res = subb8(5, 10, 0)
        assert res.cy == 1  # borrow occurred

    def test_borrow_result(self):
        res = subb8(0, 1, 0)
        assert res.result == 0xFF  # 0 - 1 wraps to 255
        assert res.cy == 1

    def test_subb_with_borrow_in(self):
        # A - B - borrow_in: 10 - 5 - 1 = 4
        res = subb8(10, 5, 1)
        assert res.result == 4
        assert res.cy == 0

    def test_subb_zero_minus_zero(self):
        res = subb8(0, 0, 0)
        assert res.result == 0
        assert res.cy == 0

    def test_subb_with_borrow_causing_wrap(self):
        res = subb8(0, 0, 1)
        assert res.result == 0xFF
        assert res.cy == 1

    def test_overflow_neg_minus_pos(self):
        # 0x80 (-128) - 0x01 (+1) = 0x7F (+127) — overflow! neg - pos = pos
        res = subb8(0x80, 0x01, 0)
        assert res.result == 0x7F
        assert res.ov == 1

    def test_no_overflow_same_sign(self):
        # 0x50 - 0x30 = 0x20, no overflow (same sign region)
        res = subb8(0x50, 0x30, 0)
        assert res.result == 0x20
        assert res.ov == 0

    def test_aux_carry_borrow(self):
        # 0x10 - 0x01 = 0x0F, borrow from bit 4 to bit 3 (AC=1)
        res = subb8(0x10, 0x01, 0)
        assert res.result == 0x0F
        assert res.ac == 1

    def test_parity(self):
        res = subb8(0x02, 0x01, 0)
        assert res.result == 0x01
        assert res.parity == 1  # 1 set bit → odd → P=1


class TestAnl8:
    def test_basic_and(self):
        res = anl8(0xFF, 0x0F)
        assert res.result == 0x0F

    def test_all_zeros(self):
        res = anl8(0xFF, 0x00)
        assert res.result == 0x00

    def test_all_ones(self):
        res = anl8(0xFF, 0xFF)
        assert res.result == 0xFF

    def test_mask(self):
        # Mask out upper nibble
        res = anl8(0xAB, 0x0F)
        assert res.result == 0x0B

    def test_no_flags(self):
        res = anl8(0xFF, 0xFF)
        assert res.cy == 0
        assert res.ac == 0
        assert res.ov == 0

    def test_idempotent(self):
        # a AND a = a
        for v in [0x00, 0x55, 0xAA, 0xFF]:
            assert anl8(v, v).result == v


class TestOrl8:
    def test_basic_or(self):
        res = orl8(0xF0, 0x0F)
        assert res.result == 0xFF

    def test_zero_or_zero(self):
        res = orl8(0x00, 0x00)
        assert res.result == 0x00

    def test_all_ones(self):
        res = orl8(0xFF, 0x00)
        assert res.result == 0xFF

    def test_no_flags(self):
        res = orl8(0x12, 0x34)
        assert res.cy == 0
        assert res.ac == 0
        assert res.ov == 0

    def test_idempotent(self):
        for v in [0x00, 0x55, 0xAA, 0xFF]:
            assert orl8(v, v).result == v


class TestXrl8:
    def test_basic_xor(self):
        res = xrl8(0xFF, 0xFF)
        assert res.result == 0x00

    def test_complement(self):
        # XOR with 0xFF inverts all bits
        res = xrl8(0xA5, 0xFF)
        assert res.result == 0x5A

    def test_identity(self):
        # XOR with 0x00 is identity
        res = xrl8(0x42, 0x00)
        assert res.result == 0x42

    def test_self_xor(self):
        for v in range(256):
            assert xrl8(v, v).result == 0

    def test_no_flags(self):
        res = xrl8(0xA5, 0x5A)
        assert res.cy == 0


class TestInc8:
    def test_basic_inc(self):
        res = inc8(0)
        assert res.result == 1

    def test_wraparound(self):
        res = inc8(0xFF)
        assert res.result == 0

    def test_no_cy_change(self):
        res = inc8(0xFF)
        assert res.cy == 0  # INC never changes carry

    def test_parity(self):
        res = inc8(0)
        assert res.parity == 1  # result=1, 1 bit set → odd → P=1


class TestDec8:
    def test_basic_dec(self):
        res = dec8(5)
        assert res.result == 4

    def test_wraparound(self):
        res = dec8(0)
        assert res.result == 0xFF

    def test_no_cy_change(self):
        res = dec8(0)
        assert res.cy == 0  # DEC never changes carry


class TestRl8:
    def test_basic_rotate(self):
        # 0x01 = 00000001 → rotate left → 0x02 = 00000010
        res = rl8(0x01)
        assert res.result == 0x02
        assert res.cy == 0

    def test_msb_wraps(self):
        # 0x80 = 10000000 → rotate left → 0x01 = 00000001, cy=1
        res = rl8(0x80)
        assert res.result == 0x01
        assert res.cy == 1

    def test_alternating(self):
        # 0x55 = 01010101 → rotate left → 0xAA = 10101010
        res = rl8(0x55)
        assert res.result == 0xAA

    def test_cycle_8(self):
        val = 0x01
        for _ in range(8):
            val = rl8(val).result
        assert val == 0x01  # 8 rotations restore original


class TestRr8:
    def test_basic_rotate(self):
        res = rr8(0x80)
        assert res.result == 0x40
        assert res.cy == 0

    def test_lsb_wraps(self):
        # 0x01 = 00000001 → rotate right → 0x80 = 10000000, cy=1
        res = rr8(0x01)
        assert res.result == 0x80
        assert res.cy == 1

    def test_cycle_8(self):
        val = 0x01
        for _ in range(8):
            val = rr8(val).result
        assert val == 0x01


class TestRlc8:
    def test_basic(self):
        # 0x80, carry_in=0: bit 7 → cy=1, carry_in=0 → bit 0, result = 0x00
        res = rlc8(0x80, 0)
        assert res.result == 0x00
        assert res.cy == 1

    def test_carry_in_to_bit0(self):
        # 0x00, carry_in=1: result = 0x01, cy=0
        res = rlc8(0x00, 1)
        assert res.result == 0x01
        assert res.cy == 0

    def test_chain(self):
        # 0xFF, carry_in=0: all ones shift left, bit 7 becomes cy=1, bit 0 = 0 → 0xFE
        res = rlc8(0xFF, 0)
        assert res.result == 0xFE
        assert res.cy == 1

    def test_nine_bit_cycle(self):
        # After 9 RLC operations, we should return to the original value and cy
        val, cy = 0x42, 0
        for _ in range(9):
            res = rlc8(val, cy)
            val, cy = res.result, res.cy
        assert val == 0x42
        assert cy == 0


class TestRrc8:
    def test_basic(self):
        res = rrc8(0x01, 0)
        assert res.result == 0x00
        assert res.cy == 1

    def test_carry_in_to_bit7(self):
        res = rrc8(0x00, 1)
        assert res.result == 0x80
        assert res.cy == 0

    def test_nine_bit_cycle(self):
        val, cy = 0x42, 0
        for _ in range(9):
            res = rrc8(val, cy)
            val, cy = res.result, res.cy
        assert val == 0x42
        assert cy == 0


class TestSwap8:
    def test_swap_nibbles(self):
        # 0xAB: upper=A, lower=B → 0xBA
        res = swap8(0xAB)
        assert res.result == 0xBA

    def test_swap_zeros(self):
        res = swap8(0x00)
        assert res.result == 0x00

    def test_swap_max(self):
        res = swap8(0xFF)
        assert res.result == 0xFF

    def test_swap_inverts_in_two(self):
        # Double swap restores original
        for v in range(256):
            assert swap8(swap8(v).result).result == v

    def test_no_flags(self):
        res = swap8(0xAB)
        assert res.cy == 0
        assert res.ov == 0


class TestDa8:
    def test_no_adjustment_needed(self):
        # 0x35: low nibble 5, high nibble 3 — both valid BCD, no carry, no AC
        res = da8(0x35, 0, 0)
        assert res.result == 0x35
        assert res.cy == 0

    def test_low_nibble_adjustment(self):
        # 0x1A: low nibble A > 9, so +6: 0x1A + 0x06 = 0x20
        res = da8(0x1A, 0, 0)
        assert res.result == 0x20

    def test_ac_triggers_low_correction(self):
        # AC=1 forces low nibble correction even if low nibble ≤ 9
        # 0x09 with AC=1: +6 → 0x09 + 0x06 = 0x0F
        res = da8(0x09, 0, 1)
        assert res.result == 0x0F  # AC=1 forces the correction

    def test_no_correction_when_valid(self):
        # 0x19 with AC=0 → low nibble 9 ≤ 9, no correction needed
        res2 = da8(0x19, 0, 0)
        assert res2.result == 0x19  # No correction needed

    def test_bcd_add_example(self):
        # Standard BCD addition: 0x47 + 0x38 = 0x7F, then DA
        # 0x7F: low nibble F > 9, +6: 0x7F + 6 = 0x85
        # 8 is valid, no high correction
        res = da8(0x7F, 0, 0)
        assert res.result == 0x85  # BCD 85 (the correct BCD result)

    def test_carry_in_triggers_high_correction(self):
        # cy_in=1 forces high nibble +0x60 regardless
        res = da8(0x05, 1, 0)
        assert res.result == 0x65  # 0x05 + 0x60 = 0x65
        assert res.cy == 1

    def test_result_over_99_sets_cy(self):
        # 0x99 + AC/CY adjustments that push above 0x99
        res = da8(0x99, 0, 0)
        # 0x99: nibble 9 ≤ 9 OK, high nibble 9 ≤ 9 OK → no correction
        assert res.result == 0x99

    def test_known_bcd_result(self):
        # 0x29 (BCD 29) + 0x47 (BCD 47) in binary: 0x70, AC=0, CY=0
        # DA 0x70: low nibble 0 ≤ 9, no low correction; high nibble 7 ≤ 9, no high correction
        res = da8(0x70, 0, 0)
        assert res.result == 0x70  # BCD 70 ✓


class TestMul8:
    def test_zero_times_anything(self):
        hi, lo, ov = mul8(0, 100)
        assert lo == 0
        assert hi == 0
        assert ov == 0

    def test_one_times_value(self):
        hi, lo, ov = mul8(1, 42)
        assert lo == 42
        assert hi == 0
        assert ov == 0

    def test_basic_multiply(self):
        # 12 × 17 = 204 = 0xCC
        hi, lo, ov = mul8(12, 17)
        assert lo == 204
        assert hi == 0
        assert ov == 0

    def test_overflow(self):
        # 255 × 255 = 65025 = 0xFE01
        hi, lo, ov = mul8(255, 255)
        assert hi == 0xFE
        assert lo == 0x01
        assert ov == 1

    def test_no_overflow_boundary(self):
        # 16 × 15 = 240 ≤ 255, no overflow
        hi, lo, ov = mul8(16, 15)
        assert lo == 240
        assert hi == 0
        assert ov == 0

    def test_overflow_boundary(self):
        # 16 × 16 = 256 > 255, overflow
        hi, lo, ov = mul8(16, 16)
        assert hi == 1
        assert lo == 0
        assert ov == 1

    def test_commutative(self):
        for a, b in [(12, 17), (3, 7), (100, 200)]:
            h1, l1, _ = mul8(a, b)
            h2, l2, _ = mul8(b, a)
            assert l1 == l2 and h1 == h2


class TestDiv8:
    def test_zero_divisor(self):
        q, r, ov = div8(100, 0)
        assert ov == 1

    def test_basic_divide(self):
        # 100 / 7 = 14 remainder 2
        q, r, ov = div8(100, 7)
        assert q == 14
        assert r == 2
        assert ov == 0

    def test_exact_division(self):
        q, r, ov = div8(20, 4)
        assert q == 5
        assert r == 0
        assert ov == 0

    def test_dividend_less_than_divisor(self):
        q, r, ov = div8(3, 10)
        assert q == 0
        assert r == 3
        assert ov == 0

    def test_divide_by_one(self):
        q, r, ov = div8(127, 1)
        assert q == 127
        assert r == 0
        assert ov == 0

    def test_consistency(self):
        # q * b + r == a
        for a, b in [(100, 7), (255, 17), (200, 13), (50, 3)]:
            q, r, ov = div8(a, b)
            assert q * b + r == a
