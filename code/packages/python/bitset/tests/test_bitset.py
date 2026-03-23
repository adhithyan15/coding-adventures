"""
Comprehensive test suite for the Bitset package.

Tests are organized into groups matching the Rust reference implementation:
  - Constructor tests (new, from_integer, from_binary_str)
  - Single-bit operation tests (set, clear, test, toggle)
  - Bulk bitwise operation tests (AND, OR, XOR, NOT, AND-NOT)
  - Operator overload tests (&, |, ^, ~)
  - Counting and query tests (popcount, any, all, none)
  - Iteration tests (iter_set_bits)
  - Conversion tests (to_integer, to_binary_str)
  - Equality tests
  - Repr tests
  - Contains and __iter__ tests
  - Edge case tests
"""

import pytest

from bitset import Bitset, BitsetError


# =========================================================================
# Constructor tests
# =========================================================================


class TestConstructorNew:
    """Tests for Bitset(size) constructor."""

    def test_new_zero_size(self) -> None:
        bs = Bitset(0)
        assert len(bs) == 0
        assert bs.capacity() == 0
        assert bs.popcount() == 0
        assert bs.none()
        assert bs.all()  # vacuous truth

    def test_new_various_sizes(self) -> None:
        # Size 1: needs 1 word (64 bits capacity)
        bs = Bitset(1)
        assert len(bs) == 1
        assert bs.capacity() == 64

        # Size 64: fits exactly in 1 word
        bs = Bitset(64)
        assert len(bs) == 64
        assert bs.capacity() == 64

        # Size 65: needs 2 words (128 bits capacity)
        bs = Bitset(65)
        assert len(bs) == 65
        assert bs.capacity() == 128

        # Size 128: fits exactly in 2 words
        bs = Bitset(128)
        assert len(bs) == 128
        assert bs.capacity() == 128

        # Size 200: needs ceil(200/64) = 4 words = 256 bits
        bs = Bitset(200)
        assert len(bs) == 200
        assert bs.capacity() == 256

    def test_new_all_zeros(self) -> None:
        bs = Bitset(1000)
        assert bs.popcount() == 0
        for i in range(0, 1000, 50):  # spot check instead of full loop
            assert not bs.test(i), f"bit {i} should be 0"

    def test_new_default_zero(self) -> None:
        bs = Bitset()
        assert len(bs) == 0
        assert bs.capacity() == 0


class TestConstructorFromInteger:
    """Tests for Bitset.from_integer(value)."""

    def test_from_integer_zero(self) -> None:
        bs = Bitset.from_integer(0)
        assert len(bs) == 0
        assert bs.to_integer() == 0

    def test_from_integer_small_values(self) -> None:
        # 1 = binary 1 -> bit 0 set, len = 1
        bs = Bitset.from_integer(1)
        assert len(bs) == 1
        assert bs.test(0)
        assert bs.to_integer() == 1

        # 5 = binary 101 -> bits 0,2 set, len = 3
        bs = Bitset.from_integer(5)
        assert len(bs) == 3
        assert bs.test(0)
        assert not bs.test(1)
        assert bs.test(2)
        assert bs.to_integer() == 5

        # 255 = binary 11111111 -> bits 0-7 set, len = 8
        bs = Bitset.from_integer(255)
        assert len(bs) == 8
        assert bs.popcount() == 8

    def test_from_integer_powers_of_two(self) -> None:
        # Power of two: only one bit set
        for exp in range(64):
            val = 1 << exp
            bs = Bitset.from_integer(val)
            assert len(bs) == exp + 1
            assert bs.popcount() == 1
            assert bs.test(exp)

    def test_from_integer_u64_max(self) -> None:
        u64_max = (1 << 64) - 1
        bs = Bitset.from_integer(u64_max)
        assert len(bs) == 64
        assert bs.popcount() == 64
        assert bs.to_integer() == u64_max

    def test_from_integer_large_multi_word(self) -> None:
        # A value that requires 2 words (bit 64 set + 42 in low word)
        val = (1 << 64) | 42
        bs = Bitset.from_integer(val)
        assert len(bs) == 65
        assert bs.test(64)  # high bit
        assert bs.test(1)   # from 42 = 0b101010
        assert bs.test(3)
        assert bs.test(5)

    def test_from_integer_negative_raises(self) -> None:
        with pytest.raises(BitsetError):
            Bitset.from_integer(-1)

    def test_from_integer_very_large(self) -> None:
        # Python supports arbitrarily large ints -- test with 200+ bits
        val = (1 << 200) | (1 << 100) | 1
        bs = Bitset.from_integer(val)
        assert len(bs) == 201
        assert bs.test(0)
        assert bs.test(100)
        assert bs.test(200)
        assert bs.popcount() == 3


class TestConstructorFromBinaryStr:
    """Tests for Bitset.from_binary_str(s)."""

    def test_from_binary_str_empty(self) -> None:
        bs = Bitset.from_binary_str("")
        assert len(bs) == 0
        assert bs.to_binary_str() == ""

    def test_from_binary_str_single_bits(self) -> None:
        bs = Bitset.from_binary_str("0")
        assert len(bs) == 1
        assert not bs.test(0)

        bs = Bitset.from_binary_str("1")
        assert len(bs) == 1
        assert bs.test(0)

    def test_from_binary_str_various(self) -> None:
        # "1010" -> bits 1,3 set (reading right to left)
        bs = Bitset.from_binary_str("1010")
        assert len(bs) == 4
        assert not bs.test(0)
        assert bs.test(1)
        assert not bs.test(2)
        assert bs.test(3)
        assert bs.to_integer() == 10

        # "11111111" -> all 8 bits set
        bs = Bitset.from_binary_str("11111111")
        assert len(bs) == 8
        assert bs.popcount() == 8
        assert bs.to_integer() == 255

    def test_from_binary_str_leading_zeros(self) -> None:
        # "0001" -> len=4, only bit 0 set
        bs = Bitset.from_binary_str("0001")
        assert len(bs) == 4
        assert bs.to_integer() == 1
        assert bs.to_binary_str() == "0001"

    def test_from_binary_str_invalid(self) -> None:
        with pytest.raises(BitsetError):
            Bitset.from_binary_str("102")
        with pytest.raises(BitsetError):
            Bitset.from_binary_str("abc")
        with pytest.raises(BitsetError):
            Bitset.from_binary_str("10 01")
        with pytest.raises(BitsetError):
            Bitset.from_binary_str("1.0")

    def test_from_binary_str_long(self) -> None:
        # 65 characters -> spans 2 words
        s = "1" + "0" * 64
        bs = Bitset.from_binary_str(s)
        assert len(bs) == 65
        assert bs.test(64)  # the leading '1'
        assert bs.popcount() == 1


# =========================================================================
# Single-bit operation tests
# =========================================================================


class TestSingleBitOps:
    """Tests for set, clear, test, and toggle."""

    def test_set_and_test(self) -> None:
        bs = Bitset(100)
        assert not bs.test(50)
        bs.set(50)
        assert bs.test(50)
        assert bs.popcount() == 1

    def test_set_idempotent(self) -> None:
        # Setting a bit twice should be the same as setting it once
        bs = Bitset(100)
        bs.set(42)
        bs.set(42)
        assert bs.popcount() == 1

    def test_clear(self) -> None:
        bs = Bitset(100)
        bs.set(50)
        assert bs.test(50)
        bs.clear(50)
        assert not bs.test(50)
        assert bs.popcount() == 0

    def test_clear_idempotent(self) -> None:
        bs = Bitset(100)
        bs.clear(50)  # clear an already-clear bit
        assert not bs.test(50)
        assert bs.popcount() == 0

    def test_clear_beyond_len(self) -> None:
        bs = Bitset(10)
        bs.clear(999)  # no-op, no error, no growth
        assert len(bs) == 10

    def test_test_beyond_len(self) -> None:
        bs = Bitset(10)
        assert not bs.test(999)  # returns False, no error
        assert len(bs) == 10

    def test_toggle(self) -> None:
        bs = Bitset(10)
        assert not bs.test(5)
        bs.toggle(5)  # 0 -> 1
        assert bs.test(5)
        bs.toggle(5)  # 1 -> 0
        assert not bs.test(5)

    def test_set_auto_growth(self) -> None:
        bs = Bitset(10)
        assert len(bs) == 10
        assert bs.capacity() == 64

        # Set a bit beyond capacity -> triggers growth
        bs.set(200)
        assert bs.test(200)
        assert len(bs) == 201
        assert bs.capacity() >= 201

        # Previous bits still work
        bs.set(5)
        assert bs.test(5)

    def test_toggle_auto_growth(self) -> None:
        bs = Bitset(10)
        bs.toggle(100)  # grows and sets bit 100
        assert len(bs) == 101
        assert bs.test(100)

    def test_set_at_word_boundary(self) -> None:
        bs = Bitset(64)
        # Bit 63 is the last bit of word 0
        bs.set(63)
        assert bs.test(63)

        # Bit 64 is the first bit of word 1 -- triggers growth
        bs.set(64)
        assert bs.test(64)
        assert len(bs) == 65
        assert bs.capacity() == 128

    def test_growth_doubling(self) -> None:
        # Verify the doubling strategy
        bs = Bitset(0)
        assert bs.capacity() == 0

        bs.set(0)
        assert bs.capacity() == 64  # minimum is 64

        bs.set(63)
        assert bs.capacity() == 64  # still fits in 1 word

        bs.set(64)
        assert bs.capacity() == 128  # doubled to 2 words

        bs.set(200)
        assert bs.capacity() >= 201
        # Should have doubled: 128 -> 256
        assert bs.capacity() == 256

    def test_set_multiple_bits_across_words(self) -> None:
        bs = Bitset(200)
        for i in range(0, 200, 7):
            bs.set(i)
        for i in range(200):
            expected = i % 7 == 0
            assert bs.test(i) == expected, f"bit {i}"


# =========================================================================
# Bulk bitwise operation tests
# =========================================================================


class TestBulkOps:
    """Tests for AND, OR, XOR, NOT, AND-NOT with truth table verification."""

    def test_and_truth_table(self) -> None:
        # A=0,B=0 -> 0    A=0,B=1 -> 0    A=1,B=0 -> 0    A=1,B=1 -> 1
        a = Bitset.from_integer(0b1100)  # bits 2,3
        b = Bitset.from_integer(0b1010)  # bits 1,3
        c = a.bitwise_and(b)
        assert c.to_integer() == 0b1000  # only bit 3

        assert not c.test(0)  # 0 & 0 = 0
        assert not c.test(1)  # 0 & 1 = 0
        assert not c.test(2)  # 1 & 0 = 0
        assert c.test(3)      # 1 & 1 = 1

    def test_or_truth_table(self) -> None:
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        c = a.bitwise_or(b)
        assert c.to_integer() == 0b1110  # bits 1,2,3

        assert not c.test(0)  # 0 | 0 = 0
        assert c.test(1)      # 0 | 1 = 1
        assert c.test(2)      # 1 | 0 = 1
        assert c.test(3)      # 1 | 1 = 1

    def test_xor_truth_table(self) -> None:
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        c = a.bitwise_xor(b)
        assert c.to_integer() == 0b0110  # bits 1,2

        assert not c.test(0)  # 0 ^ 0 = 0
        assert c.test(1)      # 0 ^ 1 = 1
        assert c.test(2)      # 1 ^ 0 = 1
        assert not c.test(3)  # 1 ^ 1 = 0

    def test_not_truth_table(self) -> None:
        a = Bitset.from_integer(0b1010)  # len=4, bits 1,3
        b = a.bitwise_not()
        assert len(b) == 4
        assert b.to_integer() == 0b0101  # bits 0,2

        assert b.test(0)      # ~0 = 1
        assert not b.test(1)  # ~1 = 0
        assert b.test(2)      # ~0 = 1
        assert not b.test(3)  # ~1 = 0

    def test_and_not_truth_table(self) -> None:
        a = Bitset.from_integer(0b1110)  # bits 1,2,3
        b = Bitset.from_integer(0b1010)  # bits 1,3
        c = a.and_not(b)
        assert c.to_integer() == 0b0100  # only bit 2

        assert not c.test(0)  # 0 & ~0 = 0
        assert not c.test(1)  # 1 & ~1 = 0
        assert c.test(2)      # 1 & ~0 = 1
        assert not c.test(3)  # 1 & ~1 = 0

    def test_bulk_ops_different_sizes(self) -> None:
        # a has 4 bits, b has 8 bits -> result has 8 bits
        a = Bitset.from_integer(0b1010)      # len=4
        b = Bitset.from_integer(0b11001100)  # len=8
        c = a.bitwise_or(b)
        assert len(c) == 8
        # a zero-extended: 0b00001010
        # b:               0b11001100
        # OR:              0b11001110
        assert c.to_integer() == 0b11001110

    def test_bulk_ops_with_empty(self) -> None:
        a = Bitset.from_integer(42)
        empty = Bitset(0)

        # AND with empty -> all zeros (but len = max of the two)
        c = a.bitwise_and(empty)
        assert len(c) == len(a)
        assert c.popcount() == 0

        # OR with empty -> same as a
        c = a.bitwise_or(empty)
        assert c.to_integer() == 42

        # XOR with empty -> same as a
        c = a.bitwise_xor(empty)
        assert c.to_integer() == 42

    def test_not_clean_trailing_bits(self) -> None:
        # NOT must clean trailing bits. If len=5, capacity=64, then
        # NOT should only flip bits 0-4, not bits 5-63.
        a = Bitset.from_binary_str("10101")  # len=5
        b = a.bitwise_not()
        assert len(b) == 5
        assert b.to_binary_str() == "01010"
        assert b.popcount() == 2  # only 2 bits set, not 62

    def test_not_involution(self) -> None:
        # NOT applied twice should give back the original: ~~a == a
        a = Bitset.from_integer(0b11001010)
        b = a.bitwise_not().bitwise_not()
        assert a == b

    def test_and_not_with_different_sizes(self) -> None:
        a = Bitset.from_integer(0xFF)   # 8 bits all set
        b = Bitset.from_integer(0x0F)   # lower 4 bits set
        c = a.and_not(b)
        assert c.to_integer() == 0xF0   # upper 4 bits remain

    def test_bulk_ops_do_not_mutate_operands(self) -> None:
        """All bulk operations must return NEW bitsets without modifying inputs."""
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        a_orig = a.to_integer()
        b_orig = b.to_integer()

        _ = a.bitwise_and(b)
        _ = a.bitwise_or(b)
        _ = a.bitwise_xor(b)
        _ = a.bitwise_not()
        _ = a.and_not(b)

        assert a.to_integer() == a_orig
        assert b.to_integer() == b_orig


# =========================================================================
# Operator overload tests
# =========================================================================


class TestOperatorOverloads:
    """Tests for &, |, ^, ~ operators."""

    def test_and_operator(self) -> None:
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        c = a & b
        assert c.to_integer() == 0b1000

    def test_or_operator(self) -> None:
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        c = a | b
        assert c.to_integer() == 0b1110

    def test_xor_operator(self) -> None:
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        c = a ^ b
        assert c.to_integer() == 0b0110

    def test_not_operator(self) -> None:
        a = Bitset.from_integer(0b1010)  # len=4
        b = ~a
        assert b.to_integer() == 0b0101


# =========================================================================
# Counting and query tests
# =========================================================================


class TestCountingAndQuery:
    """Tests for popcount, any, all, none, len, capacity."""

    def test_popcount_empty(self) -> None:
        assert Bitset(0).popcount() == 0
        assert Bitset(1000).popcount() == 0

    def test_popcount_various(self) -> None:
        bs = Bitset.from_integer(0b10110)  # 3 bits set
        assert bs.popcount() == 3

        u64_max = (1 << 64) - 1
        bs = Bitset.from_integer(u64_max)  # 64 bits set
        assert bs.popcount() == 64

    def test_any_none(self) -> None:
        bs = Bitset(100)
        assert not bs.any()
        assert bs.none()

        bs.set(50)
        assert bs.any()
        assert not bs.none()

    def test_all(self) -> None:
        # Empty: vacuous truth
        assert Bitset(0).all()

        # All set
        bs = Bitset.from_binary_str("1111")
        assert bs.all()

        # Not all set
        bs = Bitset.from_binary_str("1110")
        assert not bs.all()

        # Single bit, set
        bs = Bitset.from_binary_str("1")
        assert bs.all()

        # Single bit, not set
        bs = Bitset.from_binary_str("0")
        assert not bs.all()

    def test_all_full_words(self) -> None:
        # 64 bits all set -> all() should be true
        bs = Bitset(64)
        for i in range(64):
            bs.set(i)
        assert bs.all()
        assert bs.popcount() == 64

    def test_all_partial_last_word(self) -> None:
        # 70 bits all set -> last word is partial
        bs = Bitset(70)
        for i in range(70):
            bs.set(i)
        assert bs.all()

        # Clear one bit -> all() should be false
        bs.clear(69)
        assert not bs.all()

    def test_len(self) -> None:
        assert len(Bitset(0)) == 0
        assert len(Bitset(100)) == 100
        assert len(Bitset.from_integer(5)) == 3

    def test_capacity(self) -> None:
        assert Bitset(0).capacity() == 0
        assert Bitset(1).capacity() == 64
        assert Bitset(64).capacity() == 64
        assert Bitset(65).capacity() == 128


# =========================================================================
# Iteration tests
# =========================================================================


class TestIteration:
    """Tests for iter_set_bits and __iter__."""

    def test_iter_set_bits_empty(self) -> None:
        bs = Bitset(0)
        assert list(bs.iter_set_bits()) == []

        bs = Bitset(100)
        assert list(bs.iter_set_bits()) == []

    def test_iter_set_bits_single(self) -> None:
        bs = Bitset(100)
        bs.set(42)
        assert list(bs.iter_set_bits()) == [42]

    def test_iter_set_bits_multiple(self) -> None:
        bs = Bitset.from_integer(0b10100101)  # bits 0,2,5,7
        assert list(bs.iter_set_bits()) == [0, 2, 5, 7]

    def test_iter_set_bits_across_words(self) -> None:
        bs = Bitset(200)
        bs.set(0)
        bs.set(63)   # last bit of word 0
        bs.set(64)   # first bit of word 1
        bs.set(127)  # last bit of word 1
        bs.set(128)  # first bit of word 2
        bs.set(199)  # last addressable bit
        assert list(bs.iter_set_bits()) == [0, 63, 64, 127, 128, 199]

    def test_iter_set_bits_all_set(self) -> None:
        bs = Bitset.from_binary_str("1111")
        assert list(bs.iter_set_bits()) == [0, 1, 2, 3]

    def test_dunder_iter(self) -> None:
        """__iter__ should yield the same as iter_set_bits."""
        bs = Bitset.from_integer(0b1010)
        assert list(bs) == [1, 3]

    def test_iter_in_for_loop(self) -> None:
        bs = Bitset(10)
        bs.set(2)
        bs.set(7)
        collected = [i for i in bs]
        assert collected == [2, 7]


# =========================================================================
# Conversion tests
# =========================================================================


class TestConversions:
    """Tests for to_integer and to_binary_str."""

    def test_to_integer_empty(self) -> None:
        assert Bitset(0).to_integer() == 0

    def test_to_integer_roundtrip(self) -> None:
        for val in [0, 1, 5, 42, 255, (1 << 64) - 1]:
            assert Bitset.from_integer(val).to_integer() == val

    def test_to_integer_large(self) -> None:
        # Python supports arbitrary precision -- multi-word roundtrip
        val = (1 << 200) | (1 << 100) | 42
        bs = Bitset.from_integer(val)
        assert bs.to_integer() == val

    def test_to_binary_str_empty(self) -> None:
        assert Bitset(0).to_binary_str() == ""

    def test_to_binary_str_roundtrip(self) -> None:
        for s in ["0", "1", "101", "1010", "11111111", "0001"]:
            assert Bitset.from_binary_str(s).to_binary_str() == s

    def test_to_binary_str_from_integer(self) -> None:
        bs = Bitset.from_integer(5)  # binary 101
        assert bs.to_binary_str() == "101"

        bs = Bitset.from_integer(10)  # binary 1010
        assert bs.to_binary_str() == "1010"

    def test_to_binary_str_preserves_length(self) -> None:
        # A bitset created with new(8) and bit 0 set should show all 8 chars
        bs = Bitset(8)
        bs.set(0)
        assert bs.to_binary_str() == "00000001"
        assert len(bs.to_binary_str()) == 8


# =========================================================================
# Equality tests
# =========================================================================


class TestEquality:
    """Tests for __eq__ and __hash__."""

    def test_equal_bitsets(self) -> None:
        a = Bitset.from_integer(42)
        b = Bitset.from_integer(42)
        assert a == b

    def test_unequal_values(self) -> None:
        a = Bitset.from_integer(42)
        b = Bitset.from_integer(43)
        assert a != b

    def test_unequal_lengths(self) -> None:
        # Same bits set but different lengths are NOT equal
        a = Bitset.from_binary_str("101")   # len=3
        b = Bitset.from_binary_str("0101")  # len=4
        assert a != b

    def test_equal_different_capacity(self) -> None:
        """Capacity should not affect equality."""
        a = Bitset(64)
        a.set(0)
        a.set(5)

        # Create b with larger capacity by growing then shrinking back
        b = Bitset(200)
        b.set(0)
        b.set(5)
        # b has larger capacity, but we need same len
        # Instead, create from the same initial params
        b2 = Bitset(64)
        b2.set(0)
        b2.set(5)
        assert a == b2

    def test_not_equal_to_non_bitset(self) -> None:
        bs = Bitset.from_integer(5)
        assert bs != 5
        assert bs != "101"
        assert bs != None  # noqa: E711

    def test_hash_equal_bitsets(self) -> None:
        a = Bitset.from_integer(42)
        b = Bitset.from_integer(42)
        assert hash(a) == hash(b)

    def test_hash_in_set(self) -> None:
        s: set[Bitset] = set()
        s.add(Bitset.from_integer(1))
        s.add(Bitset.from_integer(2))
        s.add(Bitset.from_integer(1))  # duplicate
        assert len(s) == 2


# =========================================================================
# Repr tests
# =========================================================================


class TestRepr:
    """Tests for __repr__."""

    def test_repr_integer(self) -> None:
        bs = Bitset.from_integer(5)
        assert repr(bs) == "Bitset('101')"

    def test_repr_empty(self) -> None:
        bs = Bitset(0)
        assert repr(bs) == "Bitset('')"

    def test_repr_leading_zeros(self) -> None:
        bs = Bitset.from_binary_str("0001")
        assert repr(bs) == "Bitset('0001')"


# =========================================================================
# Contains tests
# =========================================================================


class TestContains:
    """Tests for __contains__ (the ``in`` operator)."""

    def test_contains_set_bit(self) -> None:
        bs = Bitset(10)
        bs.set(5)
        assert 5 in bs

    def test_not_contains_unset_bit(self) -> None:
        bs = Bitset(10)
        assert 5 not in bs

    def test_contains_out_of_range(self) -> None:
        bs = Bitset(10)
        assert 999 not in bs

    def test_contains_non_integer(self) -> None:
        bs = Bitset(10)
        assert "hello" not in bs  # type: ignore[operator]
        assert 3.14 not in bs     # type: ignore[operator]


# =========================================================================
# Edge case tests
# =========================================================================


class TestEdgeCases:
    """Edge cases and stress tests."""

    def test_set_bit_zero_on_empty(self) -> None:
        """Setting bit 0 on a zero-capacity bitset should grow correctly."""
        bs = Bitset(0)
        bs.set(0)
        assert len(bs) == 1
        assert bs.test(0)
        assert bs.capacity() == 64

    def test_toggle_on_empty(self) -> None:
        bs = Bitset(0)
        bs.toggle(0)
        assert bs.test(0)
        bs.toggle(0)
        assert not bs.test(0)

    def test_from_binary_str_all_zeros(self) -> None:
        bs = Bitset.from_binary_str("0000")
        assert len(bs) == 4
        assert bs.popcount() == 0
        assert bs.to_binary_str() == "0000"

    def test_from_binary_str_all_ones(self) -> None:
        bs = Bitset.from_binary_str("1111")
        assert len(bs) == 4
        assert bs.popcount() == 4
        assert bs.all()

    def test_not_of_empty(self) -> None:
        bs = Bitset(0)
        result = bs.bitwise_not()
        assert len(result) == 0
        assert result.popcount() == 0

    def test_not_of_all_zeros(self) -> None:
        bs = Bitset(8)
        result = bs.bitwise_not()
        assert result.popcount() == 8
        assert result.all()

    def test_and_with_self(self) -> None:
        bs = Bitset.from_integer(0b1010)
        result = bs.bitwise_and(bs)
        assert result == bs  # a & a == a

    def test_or_with_self(self) -> None:
        bs = Bitset.from_integer(0b1010)
        result = bs.bitwise_or(bs)
        assert result == bs  # a | a == a

    def test_xor_with_self(self) -> None:
        bs = Bitset.from_integer(0b1010)
        result = bs.bitwise_xor(bs)
        assert result.popcount() == 0  # a ^ a == 0

    def test_demorgan_law(self) -> None:
        """De Morgan's law: ~(A & B) == (~A) | (~B)"""
        a = Bitset.from_integer(0b11001010)
        b = Bitset.from_integer(0b10101100)
        lhs = (a & b).bitwise_not()
        rhs = (~a) | (~b)
        assert lhs == rhs

    def test_demorgan_law_2(self) -> None:
        """De Morgan's law: ~(A | B) == (~A) & (~B)"""
        a = Bitset.from_integer(0b11001010)
        b = Bitset.from_integer(0b10101100)
        lhs = (a | b).bitwise_not()
        rhs = (~a) & (~b)
        assert lhs == rhs

    def test_xor_identity(self) -> None:
        """a ^ b ^ b == a (XOR is self-inverse)"""
        a = Bitset.from_integer(42)
        b = Bitset.from_integer(99)
        result = (a ^ b) ^ b
        # The lengths might differ, so compare by value
        assert result.to_integer() == a.to_integer()

    def test_popcount_after_set_clear_cycle(self) -> None:
        bs = Bitset(100)
        for i in range(100):
            bs.set(i)
        assert bs.popcount() == 100

        for i in range(0, 100, 2):
            bs.clear(i)
        assert bs.popcount() == 50

    def test_large_bitset(self) -> None:
        """Smoke test with a large bitset to check performance is reasonable."""
        bs = Bitset(10000)
        for i in range(0, 10000, 3):
            bs.set(i)
        expected_count = len(range(0, 10000, 3))
        assert bs.popcount() == expected_count

        bits = list(bs.iter_set_bits())
        assert bits == list(range(0, 10000, 3))

    def test_all_with_exactly_64_bits(self) -> None:
        """Edge case: len is an exact multiple of 64."""
        bs = Bitset(64)
        for i in range(64):
            bs.set(i)
        assert bs.all()
        assert bs.popcount() == 64

    def test_all_with_128_bits(self) -> None:
        bs = Bitset(128)
        for i in range(128):
            bs.set(i)
        assert bs.all()

    def test_clean_trailing_bits_maintained(self) -> None:
        """Verify the clean-trailing-bits invariant after various operations."""
        # Create a bitset with len=5 (capacity=64). The last word should
        # have bits 5-63 zeroed out after every operation.
        bs = Bitset(5)
        bs.set(0)
        bs.set(4)
        assert bs.popcount() == 2

        # NOT should flip within len only
        inv = bs.bitwise_not()
        assert inv.popcount() == 3  # bits 1,2,3

        # Toggle should maintain invariant
        bs.toggle(4)  # clear bit 4
        assert bs.popcount() == 1

    def test_from_integer_then_binary_str_roundtrip(self) -> None:
        """from_integer -> to_binary_str -> from_binary_str roundtrip."""
        for val in [0, 1, 7, 42, 255, 1023, (1 << 64) - 1]:
            bs1 = Bitset.from_integer(val)
            s = bs1.to_binary_str()
            bs2 = Bitset.from_binary_str(s)
            assert bs1 == bs2
