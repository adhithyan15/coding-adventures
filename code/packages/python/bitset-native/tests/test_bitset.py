"""
Comprehensive test suite for the native Bitset extension.

Tests mirror the pure Python bitset test suite to ensure the native
extension is a drop-in replacement. Organized into groups:
  - Constructor tests (Bitset(size), from_integer, from_binary_str)
  - Single-bit operation tests (set, clear, test, toggle)
  - Bulk bitwise operation tests (bitwise_and, bitwise_or, bitwise_xor,
    bitwise_not, and_not)
  - Operator overload tests (&, |, ^, ~)
  - Counting and query tests (popcount, any, all, none)
  - Iteration tests (iter_set_bits, __iter__)
  - Conversion tests (to_integer, to_binary_str)
  - Equality and hash tests
  - Repr tests
  - Contains tests (__contains__)
  - Edge case tests
"""

import pytest

from bitset_native import Bitset, BitsetError


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
        for i in range(0, 1000, 50):
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
        bs = Bitset.from_integer(1)
        assert len(bs) == 1
        assert bs.test(0)
        assert bs.to_integer() == 1

        bs = Bitset.from_integer(5)
        assert len(bs) == 3
        assert bs.test(0)
        assert not bs.test(1)
        assert bs.test(2)
        assert bs.to_integer() == 5

        bs = Bitset.from_integer(255)
        assert len(bs) == 8
        assert bs.popcount() == 8

    def test_from_integer_powers_of_two(self) -> None:
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
        # A value that requires more than 64 bits
        val = (1 << 64) | 42
        bs = Bitset.from_integer(val)
        assert len(bs) == 65
        assert bs.test(64)
        assert bs.test(1)   # from 42 = 0b101010
        assert bs.test(3)
        assert bs.test(5)

    def test_from_integer_negative_raises(self) -> None:
        with pytest.raises(BitsetError):
            Bitset.from_integer(-1)

    def test_from_integer_very_large(self) -> None:
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
        bs = Bitset.from_binary_str("1010")
        assert len(bs) == 4
        assert not bs.test(0)
        assert bs.test(1)
        assert not bs.test(2)
        assert bs.test(3)
        assert bs.to_integer() == 10

        bs = Bitset.from_binary_str("11111111")
        assert len(bs) == 8
        assert bs.popcount() == 8
        assert bs.to_integer() == 255

    def test_from_binary_str_leading_zeros(self) -> None:
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
        s = "1" + "0" * 64
        bs = Bitset.from_binary_str(s)
        assert len(bs) == 65
        assert bs.test(64)
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
        bs.clear(50)
        assert not bs.test(50)
        assert bs.popcount() == 0

    def test_clear_beyond_len(self) -> None:
        bs = Bitset(10)
        bs.clear(999)  # no-op, no error, no growth
        assert len(bs) == 10

    def test_test_beyond_len(self) -> None:
        bs = Bitset(10)
        assert not bs.test(999)
        assert len(bs) == 10

    def test_toggle(self) -> None:
        bs = Bitset(10)
        assert not bs.test(5)
        bs.toggle(5)
        assert bs.test(5)
        bs.toggle(5)
        assert not bs.test(5)

    def test_set_auto_growth(self) -> None:
        bs = Bitset(10)
        assert len(bs) == 10
        assert bs.capacity() == 64

        bs.set(200)
        assert bs.test(200)
        assert len(bs) == 201
        assert bs.capacity() >= 201

        bs.set(5)
        assert bs.test(5)

    def test_toggle_auto_growth(self) -> None:
        bs = Bitset(10)
        bs.toggle(100)
        assert len(bs) == 101
        assert bs.test(100)

    def test_set_at_word_boundary(self) -> None:
        bs = Bitset(64)
        bs.set(63)
        assert bs.test(63)

        bs.set(64)
        assert bs.test(64)
        assert len(bs) == 65
        assert bs.capacity() == 128

    def test_growth_doubling(self) -> None:
        bs = Bitset(0)
        assert bs.capacity() == 0

        bs.set(0)
        assert bs.capacity() == 64

        bs.set(63)
        assert bs.capacity() == 64

        bs.set(64)
        assert bs.capacity() == 128

        bs.set(200)
        assert bs.capacity() >= 201
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
    """Tests for bitwise_and, bitwise_or, bitwise_xor, bitwise_not, and_not."""

    def test_and_truth_table(self) -> None:
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        c = a.bitwise_and(b)
        assert c.to_integer() == 0b1000

        assert not c.test(0)
        assert not c.test(1)
        assert not c.test(2)
        assert c.test(3)

    def test_or_truth_table(self) -> None:
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        c = a.bitwise_or(b)
        assert c.to_integer() == 0b1110

        assert not c.test(0)
        assert c.test(1)
        assert c.test(2)
        assert c.test(3)

    def test_xor_truth_table(self) -> None:
        a = Bitset.from_integer(0b1100)
        b = Bitset.from_integer(0b1010)
        c = a.bitwise_xor(b)
        assert c.to_integer() == 0b0110

        assert not c.test(0)
        assert c.test(1)
        assert c.test(2)
        assert not c.test(3)

    def test_not_truth_table(self) -> None:
        a = Bitset.from_integer(0b1010)
        b = a.bitwise_not()
        assert len(b) == 4
        assert b.to_integer() == 0b0101

        assert b.test(0)
        assert not b.test(1)
        assert b.test(2)
        assert not b.test(3)

    def test_and_not_truth_table(self) -> None:
        a = Bitset.from_integer(0b1110)
        b = Bitset.from_integer(0b1010)
        c = a.and_not(b)
        assert c.to_integer() == 0b0100

        assert not c.test(0)
        assert not c.test(1)
        assert c.test(2)
        assert not c.test(3)

    def test_bulk_ops_different_sizes(self) -> None:
        a = Bitset.from_integer(0b1010)
        b = Bitset.from_integer(0b11001100)
        c = a.bitwise_or(b)
        assert len(c) == 8
        assert c.to_integer() == 0b11001110

    def test_bulk_ops_with_empty(self) -> None:
        a = Bitset.from_integer(42)
        empty = Bitset(0)

        c = a.bitwise_and(empty)
        assert len(c) == len(a)
        assert c.popcount() == 0

        c = a.bitwise_or(empty)
        assert c.to_integer() == 42

        c = a.bitwise_xor(empty)
        assert c.to_integer() == 42

    def test_not_clean_trailing_bits(self) -> None:
        a = Bitset.from_binary_str("10101")
        b = a.bitwise_not()
        assert len(b) == 5
        assert b.to_binary_str() == "01010"
        assert b.popcount() == 2

    def test_not_involution(self) -> None:
        a = Bitset.from_integer(0b11001010)
        b = a.bitwise_not().bitwise_not()
        assert a == b

    def test_and_not_with_different_sizes(self) -> None:
        a = Bitset.from_integer(0xFF)
        b = Bitset.from_integer(0x0F)
        c = a.and_not(b)
        assert c.to_integer() == 0xF0

    def test_bulk_ops_do_not_mutate_operands(self) -> None:
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
        a = Bitset.from_integer(0b1010)
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
        bs = Bitset.from_integer(0b10110)
        assert bs.popcount() == 3

        u64_max = (1 << 64) - 1
        bs = Bitset.from_integer(u64_max)
        assert bs.popcount() == 64

    def test_any_none(self) -> None:
        bs = Bitset(100)
        assert not bs.any()
        assert bs.none()

        bs.set(50)
        assert bs.any()
        assert not bs.none()

    def test_all(self) -> None:
        assert Bitset(0).all()

        bs = Bitset.from_binary_str("1111")
        assert bs.all()

        bs = Bitset.from_binary_str("1110")
        assert not bs.all()

        bs = Bitset.from_binary_str("1")
        assert bs.all()

        bs = Bitset.from_binary_str("0")
        assert not bs.all()

    def test_all_full_words(self) -> None:
        bs = Bitset(64)
        for i in range(64):
            bs.set(i)
        assert bs.all()
        assert bs.popcount() == 64

    def test_all_partial_last_word(self) -> None:
        bs = Bitset(70)
        for i in range(70):
            bs.set(i)
        assert bs.all()

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
        bs = Bitset.from_integer(0b10100101)
        assert list(bs.iter_set_bits()) == [0, 2, 5, 7]

    def test_iter_set_bits_across_words(self) -> None:
        bs = Bitset(200)
        bs.set(0)
        bs.set(63)
        bs.set(64)
        bs.set(127)
        bs.set(128)
        bs.set(199)
        assert list(bs.iter_set_bits()) == [0, 63, 64, 127, 128, 199]

    def test_iter_set_bits_all_set(self) -> None:
        bs = Bitset.from_binary_str("1111")
        assert list(bs.iter_set_bits()) == [0, 1, 2, 3]

    def test_dunder_iter(self) -> None:
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
        val = (1 << 200) | (1 << 100) | 42
        bs = Bitset.from_integer(val)
        assert bs.to_integer() == val

    def test_to_binary_str_empty(self) -> None:
        assert Bitset(0).to_binary_str() == ""

    def test_to_binary_str_roundtrip(self) -> None:
        for s in ["0", "1", "101", "1010", "11111111", "0001"]:
            assert Bitset.from_binary_str(s).to_binary_str() == s

    def test_to_binary_str_from_integer(self) -> None:
        bs = Bitset.from_integer(5)
        assert bs.to_binary_str() == "101"

        bs = Bitset.from_integer(10)
        assert bs.to_binary_str() == "1010"

    def test_to_binary_str_preserves_length(self) -> None:
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
        a = Bitset.from_binary_str("101")
        b = Bitset.from_binary_str("0101")
        assert a != b

    def test_equal_different_capacity(self) -> None:
        a = Bitset(64)
        a.set(0)
        a.set(5)

        b = Bitset(64)
        b.set(0)
        b.set(5)
        assert a == b

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
        s: set = set()
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
        assert 3.14 not in bs  # type: ignore[operator]


# =========================================================================
# Edge case tests
# =========================================================================


class TestEdgeCases:
    """Edge cases and stress tests."""

    def test_set_bit_zero_on_empty(self) -> None:
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
        assert result == bs

    def test_or_with_self(self) -> None:
        bs = Bitset.from_integer(0b1010)
        result = bs.bitwise_or(bs)
        assert result == bs

    def test_xor_with_self(self) -> None:
        bs = Bitset.from_integer(0b1010)
        result = bs.bitwise_xor(bs)
        assert result.popcount() == 0

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
        bs = Bitset(10000)
        for i in range(0, 10000, 3):
            bs.set(i)
        expected_count = len(range(0, 10000, 3))
        assert bs.popcount() == expected_count

        bits = list(bs.iter_set_bits())
        assert bits == list(range(0, 10000, 3))

    def test_all_with_exactly_64_bits(self) -> None:
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
        bs = Bitset(5)
        bs.set(0)
        bs.set(4)
        assert bs.popcount() == 2

        inv = bs.bitwise_not()
        assert inv.popcount() == 3

        bs.toggle(4)
        assert bs.popcount() == 1

    def test_from_integer_then_binary_str_roundtrip(self) -> None:
        for val in [0, 1, 7, 42, 255, 1023, (1 << 64) - 1]:
            bs1 = Bitset.from_integer(val)
            s = bs1.to_binary_str()
            bs2 = Bitset.from_binary_str(s)
            assert bs1 == bs2
