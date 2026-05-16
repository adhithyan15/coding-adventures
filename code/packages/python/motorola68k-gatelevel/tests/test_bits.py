"""Tests for bits.py — bit conversion and arithmetic helpers."""


from motorola68k_gatelevel.bits import (
    add_8bit,
    add_16bit,
    add_32bit,
    bits_to_int,
    compute_parity,
    compute_zero,
    int_to_bits,
    invert_8bit,
    invert_16bit,
    invert_32bit,
)


class TestIntToBits:
    """int_to_bits: integer → LSB-first bit list."""

    def test_zero_8bit(self):
        assert int_to_bits(0, 8) == [0] * 8

    def test_one_8bit(self):
        b = int_to_bits(1, 8)
        assert b[0] == 1
        assert all(x == 0 for x in b[1:])

    def test_five_8bit(self):
        assert int_to_bits(5, 8) == [1, 0, 1, 0, 0, 0, 0, 0]

    def test_255_8bit(self):
        assert int_to_bits(0xFF, 8) == [1] * 8

    def test_0x0100_16bit(self):
        b = int_to_bits(0x0100, 16)
        assert b[8] == 1
        assert sum(b) == 1

    def test_32bit_msb(self):
        b = int_to_bits(0x80000000, 32)
        assert b[31] == 1
        assert sum(b) == 1

    def test_mask_overflow(self):
        # Value larger than width should be masked
        b = int_to_bits(0x1FF, 8)
        assert bits_to_int(b) == 0xFF

    def test_all_ones_32bit(self):
        b = int_to_bits(0xFFFFFFFF, 32)
        assert all(x == 1 for x in b)
        assert len(b) == 32

    def test_16bit_length(self):
        assert len(int_to_bits(0, 16)) == 16

    def test_32bit_length(self):
        assert len(int_to_bits(0, 32)) == 32

    def test_alternating_aa(self):
        b = int_to_bits(0xAA, 8)
        # 0xAA = 10101010b → LSB first: 0,1,0,1,0,1,0,1
        assert b == [0, 1, 0, 1, 0, 1, 0, 1]


class TestBitsToInt:
    """bits_to_int: LSB-first bit list → integer."""

    def test_zero(self):
        assert bits_to_int([0, 0, 0, 0, 0, 0, 0, 0]) == 0

    def test_one(self):
        assert bits_to_int([1, 0, 0, 0, 0, 0, 0, 0]) == 1

    def test_five(self):
        assert bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) == 5

    def test_255(self):
        assert bits_to_int([1, 1, 1, 1, 1, 1, 1, 1]) == 255

    def test_round_trip_8bit(self):
        for v in range(256):
            assert bits_to_int(int_to_bits(v, 8)) == v

    def test_round_trip_16bit(self):
        for v in [0, 1, 0x1234, 0x8000, 0xFFFF]:
            assert bits_to_int(int_to_bits(v, 16)) == v

    def test_round_trip_32bit(self):
        for v in [0, 1, 0x12345678, 0x80000000, 0xFFFFFFFF]:
            assert bits_to_int(int_to_bits(v, 32)) == v

    def test_empty(self):
        assert bits_to_int([]) == 0


class TestAdd8bit:
    """add_8bit: 8-bit addition through ripple-carry adder."""

    def test_basic(self):
        r, c, _ = add_8bit(10, 5)
        assert r == 15
        assert c == 0

    def test_overflow(self):
        r, c, _ = add_8bit(0xFF, 1)
        assert r == 0
        assert c == 1

    def test_carry_in(self):
        r, c, _ = add_8bit(10, 5, 1)
        assert r == 16
        assert c == 0

    def test_max_plus_max(self):
        r, c, _ = add_8bit(0xFF, 0xFF)
        assert r == 0xFE
        assert c == 1

    def test_carry_propagation(self):
        r, c, _ = add_8bit(0x7F, 1)
        assert r == 0x80
        assert c == 0

    def test_aux_carry(self):
        # 0x0F + 0x01 = carry out of bit 3
        _, _, ac = add_8bit(0x0F, 0x01)
        assert ac == 1

    def test_no_aux_carry(self):
        _, _, ac = add_8bit(0x00, 0x00)
        assert ac == 0

    def test_zero_plus_zero(self):
        r, c, ac = add_8bit(0, 0)
        assert r == 0
        assert c == 0
        assert ac == 0


class TestAdd16bit:
    """add_16bit: 16-bit addition through ripple-carry adder."""

    def test_basic(self):
        r, c, _ = add_16bit(0x1234, 0x0001)
        assert r == 0x1235
        assert c == 0

    def test_overflow(self):
        r, c, _ = add_16bit(0xFFFF, 0x0001)
        assert r == 0
        assert c == 1

    def test_carry_in(self):
        r, c, _ = add_16bit(0xFFFF, 0, 1)
        assert r == 0
        assert c == 1

    def test_max_both(self):
        r, c, _ = add_16bit(0xFFFF, 0xFFFF)
        assert r == 0xFFFE
        assert c == 1

    def test_aux_carry_16bit(self):
        _, _, ac = add_16bit(0x000F, 0x0001)
        assert ac == 1


class TestAdd32bit:
    """add_32bit: 32-bit addition — the primary 68000 adder."""

    def test_basic(self):
        r, c = add_32bit(5, 3)
        assert r == 8
        assert c == 0

    def test_overflow(self):
        r, c = add_32bit(0xFFFFFFFF, 1)
        assert r == 0
        assert c == 1

    def test_carry_in(self):
        r, c = add_32bit(0xFFFFFFFF, 0, 1)
        assert r == 0
        assert c == 1

    def test_no_carry(self):
        r, c = add_32bit(0x7FFFFFFF, 1)
        assert r == 0x80000000
        assert c == 0

    def test_large_values(self):
        r, c = add_32bit(0xDEADBEEF, 0x12345678)
        assert r == (0xDEADBEEF + 0x12345678) & 0xFFFFFFFF

    def test_zero_plus_zero(self):
        r, c = add_32bit(0, 0)
        assert r == 0
        assert c == 0

    def test_max_plus_max(self):
        r, c = add_32bit(0xFFFFFFFF, 0xFFFFFFFF)
        assert r == 0xFFFFFFFE
        assert c == 1


class TestInvert:
    """invert_8/16/32bit: bitwise NOT through NOT gate chains."""

    def test_invert_8bit_zero(self):
        assert invert_8bit(0) == 0xFF

    def test_invert_8bit_all_ones(self):
        assert invert_8bit(0xFF) == 0

    def test_invert_8bit_aa(self):
        assert invert_8bit(0xAA) == 0x55

    def test_invert_16bit_zero(self):
        assert invert_16bit(0) == 0xFFFF

    def test_invert_16bit_all_ones(self):
        assert invert_16bit(0xFFFF) == 0

    def test_invert_16bit_aaaa(self):
        assert invert_16bit(0xAAAA) == 0x5555

    def test_invert_32bit_zero(self):
        assert invert_32bit(0) == 0xFFFFFFFF

    def test_invert_32bit_all_ones(self):
        assert invert_32bit(0xFFFFFFFF) == 0

    def test_invert_32bit_aaaa(self):
        assert invert_32bit(0xAAAAAAAA) == 0x55555555

    def test_double_invert_8bit(self):
        for v in [0, 1, 0x42, 0xFF]:
            assert invert_8bit(invert_8bit(v)) == v

    def test_double_invert_32bit(self):
        for v in [0, 1, 0xDEADBEEF, 0xFFFFFFFF]:
            assert invert_32bit(invert_32bit(v)) == v


class TestComputeZero:
    """compute_zero: NOR tree — 1 if all bits are 0."""

    def test_all_zeros_8(self):
        assert compute_zero([0] * 8) == 1

    def test_all_zeros_32(self):
        assert compute_zero([0] * 32) == 1

    def test_one_set(self):
        assert compute_zero([1] + [0] * 7) == 0

    def test_msb_set(self):
        assert compute_zero([0] * 7 + [1]) == 0

    def test_all_ones(self):
        assert compute_zero([1] * 8) == 0

    def test_32bit_msb_set(self):
        assert compute_zero([0] * 31 + [1]) == 0

    def test_32bit_all_zero(self):
        b = int_to_bits(0, 32)
        assert compute_zero(b) == 1

    def test_32bit_nonzero(self):
        b = int_to_bits(1, 32)
        assert compute_zero(b) == 0


class TestComputeParity:
    """compute_parity: XOR tree over low 8 bits."""

    def test_all_zeros(self):
        # Zero 1s → even → PF=1
        assert compute_parity([0] * 8) == 1

    def test_one_one(self):
        # One 1 → odd → PF=0
        assert compute_parity([1] + [0] * 7) == 0

    def test_two_ones(self):
        # Two 1s → even → PF=1
        assert compute_parity([1, 1] + [0] * 6) == 1

    def test_all_ones(self):
        # Eight 1s → even → PF=1
        assert compute_parity([1] * 8) == 1

    def test_seven_ones(self):
        # Seven 1s → odd → PF=0
        assert compute_parity([1] * 7 + [0]) == 0

    def test_longer_list_uses_low_8(self):
        # 32-bit list: upper bits ignored
        bits = int_to_bits(0x100, 32)
        # 0x100 = bit 8 set; low 8 bits = 0 → even → PF=1
        assert compute_parity(bits) == 1
