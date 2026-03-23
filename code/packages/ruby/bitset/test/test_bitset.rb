# frozen_string_literal: true

require_relative "test_helper"

# ==========================================================================
# TestBitset -- Comprehensive tests for CodingAdventures::Bitset::Bitset
# ==========================================================================
#
# These tests cover every public method of the Bitset class, including:
#   - Constructors: new, from_integer, from_binary_str
#   - Single-bit operations: set, clear, test?, toggle
#   - Bulk bitwise operations: bitwise_and, bitwise_or, bitwise_xor,
#     bitwise_not, and_not
#   - Operator overloads: &, |, ^, ~
#   - Counting/query: popcount, size, capacity, any?, all?, none?, empty?
#   - Iteration: each_set_bit
#   - Conversion: to_integer, to_binary_str, to_s
#   - Equality: ==, eql?, hash
#   - Auto-growth semantics
#   - Clean-trailing-bits invariant
#
# Test naming convention: test_<method>_<scenario>

class TestBitset < Minitest::Test
  Bitset = CodingAdventures::Bitset::Bitset
  BitsetError = CodingAdventures::Bitset::BitsetError

  # ========================================================================
  # Constructor: new
  # ========================================================================

  def test_new_creates_zeroed_bitset
    bs = Bitset.new(100)
    assert_equal 100, bs.size
    assert_equal 128, bs.capacity  # 2 words * 64 bits
    assert_equal 0, bs.popcount
  end

  def test_new_zero_size
    bs = Bitset.new(0)
    assert_equal 0, bs.size
    assert_equal 0, bs.capacity
    assert_equal 0, bs.popcount
  end

  def test_new_exact_word_boundary
    bs = Bitset.new(64)
    assert_equal 64, bs.size
    assert_equal 64, bs.capacity  # exactly 1 word
  end

  def test_new_one_past_word_boundary
    bs = Bitset.new(65)
    assert_equal 65, bs.size
    assert_equal 128, bs.capacity  # 2 words
  end

  def test_new_negative_raises
    assert_raises(ArgumentError) { Bitset.new(-1) }
  end

  # ========================================================================
  # Constructor: from_integer
  # ========================================================================

  def test_from_integer_zero
    bs = Bitset.from_integer(0)
    assert_equal 0, bs.size
    assert_equal 0, bs.to_integer
  end

  def test_from_integer_simple
    bs = Bitset.from_integer(5)  # binary: 101
    assert_equal 3, bs.size
    assert bs.test?(0)
    refute bs.test?(1)
    assert bs.test?(2)
  end

  def test_from_integer_single_bit
    bs = Bitset.from_integer(1)
    assert_equal 1, bs.size
    assert bs.test?(0)
  end

  def test_from_integer_power_of_two
    bs = Bitset.from_integer(128)  # bit 7
    assert_equal 8, bs.size
    assert bs.test?(7)
    refute bs.test?(0)
  end

  def test_from_integer_large_value
    # Test with a value that spans multiple words (> 64 bits)
    value = (1 << 100) | (1 << 50) | 1
    bs = Bitset.from_integer(value)
    assert_equal 101, bs.size
    assert bs.test?(0)
    assert bs.test?(50)
    assert bs.test?(100)
    assert_equal value, bs.to_integer
  end

  def test_from_integer_all_ones_64bit
    value = (1 << 64) - 1  # 0xFFFFFFFFFFFFFFFF
    bs = Bitset.from_integer(value)
    assert_equal 64, bs.size
    assert_equal value, bs.to_integer
  end

  def test_from_integer_negative_raises
    assert_raises(ArgumentError) { Bitset.from_integer(-1) }
  end

  # ========================================================================
  # Constructor: from_binary_str
  # ========================================================================

  def test_from_binary_str_simple
    bs = Bitset.from_binary_str("1010")
    assert_equal 4, bs.size
    refute bs.test?(0)
    assert bs.test?(1)
    refute bs.test?(2)
    assert bs.test?(3)
    assert_equal 10, bs.to_integer
  end

  def test_from_binary_str_all_ones
    bs = Bitset.from_binary_str("1111")
    assert_equal 4, bs.size
    assert bs.all?
    assert_equal 15, bs.to_integer
  end

  def test_from_binary_str_all_zeros
    bs = Bitset.from_binary_str("0000")
    assert_equal 4, bs.size
    assert bs.none?
    assert_equal 0, bs.to_integer
  end

  def test_from_binary_str_empty
    bs = Bitset.from_binary_str("")
    assert_equal 0, bs.size
  end

  def test_from_binary_str_single_one
    bs = Bitset.from_binary_str("1")
    assert_equal 1, bs.size
    assert bs.test?(0)
  end

  def test_from_binary_str_leading_zeros
    bs = Bitset.from_binary_str("00101")
    assert_equal 5, bs.size
    assert bs.test?(0)
    assert bs.test?(2)
    refute bs.test?(1)
    refute bs.test?(3)
    refute bs.test?(4)
  end

  def test_from_binary_str_invalid_chars
    assert_raises(BitsetError) { Bitset.from_binary_str("10102") }
    assert_raises(BitsetError) { Bitset.from_binary_str("abc") }
    assert_raises(BitsetError) { Bitset.from_binary_str("10 10") }
  end

  def test_from_binary_str_roundtrip
    bs = Bitset.from_binary_str("110100101")
    assert_equal "110100101", bs.to_binary_str
  end

  # ========================================================================
  # Single-bit: set
  # ========================================================================

  def test_set_basic
    bs = Bitset.new(10)
    bs.set(5)
    assert bs.test?(5)
    assert_equal 1, bs.popcount
  end

  def test_set_returns_self
    bs = Bitset.new(10)
    assert_same bs, bs.set(5)
  end

  def test_set_idempotent
    bs = Bitset.new(10)
    bs.set(5)
    bs.set(5)
    assert_equal 1, bs.popcount
  end

  def test_set_multiple_bits
    bs = Bitset.new(10)
    bs.set(0).set(3).set(9)
    assert bs.test?(0)
    assert bs.test?(3)
    assert bs.test?(9)
    assert_equal 3, bs.popcount
  end

  def test_set_auto_grows
    bs = Bitset.new(10)
    bs.set(100)
    assert_equal 101, bs.size
    assert bs.test?(100)
    assert bs.capacity >= 101
  end

  def test_set_auto_grows_from_empty
    bs = Bitset.new(0)
    bs.set(3)
    assert_equal 4, bs.size
    assert_equal 64, bs.capacity
    assert bs.test?(3)
  end

  def test_set_auto_grows_doubling
    bs = Bitset.new(64)  # capacity = 64
    bs.set(200)
    # 64 -> 128 -> 256, so capacity should be 256
    assert_equal 256, bs.capacity
    assert_equal 201, bs.size
  end

  # ========================================================================
  # Single-bit: clear
  # ========================================================================

  def test_clear_basic
    bs = Bitset.new(10)
    bs.set(5)
    bs.clear(5)
    refute bs.test?(5)
    assert_equal 0, bs.popcount
  end

  def test_clear_returns_self
    bs = Bitset.new(10)
    assert_same bs, bs.clear(5)
  end

  def test_clear_already_zero
    bs = Bitset.new(10)
    bs.clear(5)  # already zero, should be no-op
    refute bs.test?(5)
  end

  def test_clear_beyond_len_no_growth
    bs = Bitset.new(10)
    bs.clear(999)
    assert_equal 10, bs.size  # no growth
    assert_equal 64, bs.capacity
  end

  def test_clear_preserves_other_bits
    bs = Bitset.new(10)
    bs.set(3).set(5).set(7)
    bs.clear(5)
    assert bs.test?(3)
    refute bs.test?(5)
    assert bs.test?(7)
  end

  # ========================================================================
  # Single-bit: test? / test
  # ========================================================================

  def test_test_set_bit
    bs = Bitset.new(10)
    bs.set(5)
    assert bs.test?(5)
    assert bs.test(5)  # alias
  end

  def test_test_unset_bit
    bs = Bitset.new(10)
    refute bs.test?(5)
  end

  def test_test_beyond_len_returns_false
    bs = Bitset.new(10)
    refute bs.test?(999)
  end

  def test_test_boundary_bits
    bs = Bitset.new(128)
    bs.set(0)
    bs.set(63)
    bs.set(64)
    bs.set(127)
    assert bs.test?(0)
    assert bs.test?(63)
    assert bs.test?(64)
    assert bs.test?(127)
    refute bs.test?(1)
    refute bs.test?(62)
    refute bs.test?(65)
  end

  # ========================================================================
  # Single-bit: toggle
  # ========================================================================

  def test_toggle_zero_to_one
    bs = Bitset.new(10)
    bs.toggle(5)
    assert bs.test?(5)
  end

  def test_toggle_one_to_zero
    bs = Bitset.new(10)
    bs.set(5)
    bs.toggle(5)
    refute bs.test?(5)
  end

  def test_toggle_returns_self
    bs = Bitset.new(10)
    assert_same bs, bs.toggle(5)
  end

  def test_toggle_auto_grows
    bs = Bitset.new(10)
    bs.toggle(100)
    assert_equal 101, bs.size
    assert bs.test?(100)
  end

  def test_toggle_twice_is_identity
    bs = Bitset.new(10)
    bs.set(5)
    bs.toggle(5)
    bs.toggle(5)
    assert bs.test?(5)
  end

  # ========================================================================
  # Bulk: bitwise_and / &
  # ========================================================================

  def test_bitwise_and_basic
    a = Bitset.from_integer(0b1100)  # bits 2,3
    b = Bitset.from_integer(0b1010)  # bits 1,3
    c = a.bitwise_and(b)
    assert_equal 0b1000, c.to_integer  # only bit 3
  end

  def test_bitwise_and_operator
    a = Bitset.from_integer(0b1100)
    b = Bitset.from_integer(0b1010)
    c = a & b
    assert_equal 0b1000, c.to_integer
  end

  def test_bitwise_and_different_sizes
    a = Bitset.from_integer(0b11111111)  # len=8
    b = Bitset.from_integer(0b1010)      # len=4
    c = a & b
    assert_equal [a.size, b.size].max, c.size
    assert_equal 0b1010, c.to_integer
  end

  def test_bitwise_and_with_empty
    a = Bitset.from_integer(0b1010)
    b = Bitset.new(0)
    c = a & b
    assert_equal 0, c.to_integer
  end

  def test_bitwise_and_does_not_mutate
    a = Bitset.from_integer(0b1100)
    b = Bitset.from_integer(0b1010)
    _c = a & b
    assert_equal 0b1100, a.to_integer
    assert_equal 0b1010, b.to_integer
  end

  # ========================================================================
  # Bulk: bitwise_or / |
  # ========================================================================

  def test_bitwise_or_basic
    a = Bitset.from_integer(0b1100)
    b = Bitset.from_integer(0b1010)
    c = a.bitwise_or(b)
    assert_equal 0b1110, c.to_integer
  end

  def test_bitwise_or_operator
    a = Bitset.from_integer(0b1100)
    b = Bitset.from_integer(0b1010)
    c = a | b
    assert_equal 0b1110, c.to_integer
  end

  def test_bitwise_or_different_sizes
    a = Bitset.from_integer(0b11110000)  # len=8
    b = Bitset.from_integer(0b1010)      # len=4
    c = a | b
    assert_equal 0b11111010, c.to_integer
  end

  def test_bitwise_or_with_empty
    a = Bitset.from_integer(0b1010)
    b = Bitset.new(0)
    c = a | b
    assert_equal 0b1010, c.to_integer
  end

  # ========================================================================
  # Bulk: bitwise_xor / ^
  # ========================================================================

  def test_bitwise_xor_basic
    a = Bitset.from_integer(0b1100)
    b = Bitset.from_integer(0b1010)
    c = a.bitwise_xor(b)
    assert_equal 0b0110, c.to_integer
  end

  def test_bitwise_xor_operator
    a = Bitset.from_integer(0b1100)
    b = Bitset.from_integer(0b1010)
    c = a ^ b
    assert_equal 0b0110, c.to_integer
  end

  def test_bitwise_xor_same_value_gives_zero
    a = Bitset.from_integer(0b1010)
    c = a ^ a
    assert_equal 0, c.to_integer
  end

  def test_bitwise_xor_different_sizes
    a = Bitset.from_integer(0b11110000)
    b = Bitset.from_integer(0b1010)
    c = a ^ b
    assert_equal 0b11111010, c.to_integer
  end

  # ========================================================================
  # Bulk: bitwise_not / ~
  # ========================================================================

  def test_bitwise_not_basic
    a = Bitset.from_integer(0b1010)  # len=4
    b = a.bitwise_not
    assert_equal 0b0101, b.to_integer
    assert_equal 4, b.size
  end

  def test_bitwise_not_operator
    a = Bitset.from_integer(0b1010)
    b = ~a
    assert_equal 0b0101, b.to_integer
  end

  def test_bitwise_not_preserves_len
    a = Bitset.from_binary_str("00001010")  # len=8
    b = ~a
    assert_equal 8, b.size
    assert_equal 0b11110101, b.to_integer
  end

  def test_bitwise_not_double_is_identity
    a = Bitset.from_integer(0b1010)
    b = ~~a
    assert_equal a, b
  end

  def test_bitwise_not_empty
    a = Bitset.new(0)
    b = ~a
    assert_equal 0, b.size
    assert_equal 0, b.to_integer
  end

  def test_bitwise_not_all_ones
    a = Bitset.from_binary_str("1111")  # len=4
    b = ~a
    assert_equal 0, b.to_integer
    assert_equal 4, b.size
  end

  # ========================================================================
  # Bulk: and_not
  # ========================================================================

  def test_and_not_basic
    a = Bitset.from_integer(0b1110)  # bits 1,2,3
    b = Bitset.from_integer(0b1010)  # bits 1,3
    c = a.and_not(b)
    assert_equal 0b0100, c.to_integer  # only bit 2
  end

  def test_and_not_with_self
    a = Bitset.from_integer(0b1010)
    c = a.and_not(a)
    assert_equal 0, c.to_integer
  end

  def test_and_not_with_empty
    a = Bitset.from_integer(0b1010)
    b = Bitset.new(0)
    c = a.and_not(b)
    assert_equal 0b1010, c.to_integer
  end

  def test_and_not_different_sizes
    a = Bitset.from_integer(0b11111111)  # len=8
    b = Bitset.from_integer(0b1010)      # len=4
    c = a.and_not(b)
    assert_equal 0b11110101, c.to_integer
  end

  # ========================================================================
  # Counting: popcount
  # ========================================================================

  def test_popcount_empty
    bs = Bitset.new(0)
    assert_equal 0, bs.popcount
  end

  def test_popcount_zeroed
    bs = Bitset.new(100)
    assert_equal 0, bs.popcount
  end

  def test_popcount_simple
    bs = Bitset.from_integer(0b10110)
    assert_equal 3, bs.popcount
  end

  def test_popcount_all_ones
    bs = Bitset.from_binary_str("1" * 100)
    assert_equal 100, bs.popcount
  end

  def test_popcount_across_word_boundary
    bs = Bitset.new(128)
    bs.set(0)
    bs.set(63)
    bs.set(64)
    bs.set(127)
    assert_equal 4, bs.popcount
  end

  # ========================================================================
  # Counting: size, capacity
  # ========================================================================

  def test_size_matches_constructor
    bs = Bitset.new(200)
    assert_equal 200, bs.size
  end

  def test_capacity_rounds_up
    bs = Bitset.new(1)
    assert_equal 64, bs.capacity

    bs = Bitset.new(64)
    assert_equal 64, bs.capacity

    bs = Bitset.new(65)
    assert_equal 128, bs.capacity
  end

  def test_capacity_empty
    bs = Bitset.new(0)
    assert_equal 0, bs.capacity
  end

  # ========================================================================
  # Counting: any?, all?, none?, empty?
  # ========================================================================

  def test_any_false_when_empty
    bs = Bitset.new(100)
    refute bs.any?
  end

  def test_any_true_when_bit_set
    bs = Bitset.new(100)
    bs.set(50)
    assert bs.any?
  end

  def test_all_vacuous_truth
    bs = Bitset.new(0)
    assert bs.all?
  end

  def test_all_true_when_full
    bs = Bitset.from_binary_str("1111")
    assert bs.all?
  end

  def test_all_false_when_not_full
    bs = Bitset.from_binary_str("1110")
    refute bs.all?
  end

  def test_all_full_word_boundary
    # len = 64, all bits set -> should be all?
    bs = Bitset.new(64)
    64.times { |i| bs.set(i) }
    assert bs.all?
  end

  def test_all_multiple_words
    bs = Bitset.new(100)
    100.times { |i| bs.set(i) }
    assert bs.all?
  end

  def test_all_multiple_words_one_missing
    bs = Bitset.new(100)
    100.times { |i| bs.set(i) }
    bs.clear(50)
    refute bs.all?
  end

  def test_none_true_when_empty
    bs = Bitset.new(100)
    assert bs.none?
  end

  def test_none_false_when_bit_set
    bs = Bitset.new(100)
    bs.set(50)
    refute bs.none?
  end

  def test_empty_true
    bs = Bitset.new(0)
    assert bs.empty?
  end

  def test_empty_false
    bs = Bitset.new(1)
    refute bs.empty?
  end

  # ========================================================================
  # Iteration: each_set_bit
  # ========================================================================

  def test_each_set_bit_basic
    bs = Bitset.from_integer(0b10100101)
    bits = []
    bs.each_set_bit { |i| bits << i }
    assert_equal [0, 2, 5, 7], bits
  end

  def test_each_set_bit_empty
    bs = Bitset.new(100)
    bits = []
    bs.each_set_bit { |i| bits << i }
    assert_equal [], bits
  end

  def test_each_set_bit_across_words
    bs = Bitset.new(200)
    bs.set(0)
    bs.set(63)
    bs.set(64)
    bs.set(127)
    bs.set(128)
    bs.set(199)
    bits = []
    bs.each_set_bit { |i| bits << i }
    assert_equal [0, 63, 64, 127, 128, 199], bits
  end

  def test_each_set_bit_returns_self
    bs = Bitset.from_integer(5)
    assert_same bs, bs.each_set_bit { |_| }
  end

  def test_each_set_bit_enumerator
    bs = Bitset.from_integer(0b10100101)
    enum = bs.each_set_bit
    assert_kind_of Enumerator, enum
    assert_equal [0, 2, 5, 7], enum.to_a
  end

  def test_each_set_bit_respects_len
    # Create a bitset where capacity > len, ensure we don't yield beyond len
    bs = Bitset.new(5)
    bs.set(0)
    bs.set(4)
    bits = bs.each_set_bit.to_a
    assert_equal [0, 4], bits
    bits.each { |b| assert b < 5 }
  end

  def test_each_set_bit_dense
    bs = Bitset.from_binary_str("1" * 10)
    assert_equal (0...10).to_a, bs.each_set_bit.to_a
  end

  # ========================================================================
  # Conversion: to_integer
  # ========================================================================

  def test_to_integer_empty
    bs = Bitset.new(0)
    assert_equal 0, bs.to_integer
  end

  def test_to_integer_roundtrip
    [0, 1, 5, 42, 255, 1023, (1 << 64) - 1, (1 << 100) | 7].each do |val|
      bs = Bitset.from_integer(val)
      assert_equal val, bs.to_integer, "roundtrip failed for #{val}"
    end
  end

  def test_to_integer_large
    value = (1 << 200) - 1
    bs = Bitset.from_integer(value)
    assert_equal value, bs.to_integer
  end

  # ========================================================================
  # Conversion: to_binary_str
  # ========================================================================

  def test_to_binary_str_empty
    bs = Bitset.new(0)
    assert_equal "", bs.to_binary_str
  end

  def test_to_binary_str_simple
    bs = Bitset.from_integer(5)  # binary 101
    assert_equal "101", bs.to_binary_str
  end

  def test_to_binary_str_with_leading_zeros
    bs = Bitset.from_binary_str("00101")
    assert_equal "00101", bs.to_binary_str
  end

  def test_to_binary_str_roundtrip
    ["0", "1", "101", "11111111", "00001010", "1" * 100].each do |str|
      bs = Bitset.from_binary_str(str)
      assert_equal str, bs.to_binary_str, "roundtrip failed for #{str}"
    end
  end

  # ========================================================================
  # Conversion: to_s / inspect
  # ========================================================================

  def test_to_s_format
    bs = Bitset.from_integer(5)
    assert_equal "Bitset(101)", bs.to_s
  end

  def test_to_s_empty
    bs = Bitset.new(0)
    assert_equal "Bitset()", bs.to_s
  end

  def test_inspect_same_as_to_s
    bs = Bitset.from_integer(5)
    assert_equal bs.to_s, bs.inspect
  end

  # ========================================================================
  # Equality
  # ========================================================================

  def test_equality_same_bits
    a = Bitset.from_integer(42)
    b = Bitset.from_integer(42)
    assert_equal a, b
  end

  def test_equality_different_bits
    a = Bitset.from_integer(42)
    b = Bitset.from_integer(43)
    refute_equal a, b
  end

  def test_equality_different_len_same_bits
    # Same bits set but different len -> not equal
    a = Bitset.from_binary_str("0101")   # len=4
    b = Bitset.from_binary_str("00101")  # len=5
    refute_equal a, b
  end

  def test_equality_with_non_bitset
    bs = Bitset.from_integer(5)
    refute_equal bs, 5
    refute_equal bs, "101"
  end

  def test_eql_and_hash
    a = Bitset.from_integer(42)
    b = Bitset.from_integer(42)
    assert a.eql?(b)
    assert_equal a.hash, b.hash
  end

  def test_equality_empty
    a = Bitset.new(0)
    b = Bitset.new(0)
    assert_equal a, b
  end

  def test_equality_different_capacity_same_bits
    # Both have same len and bits, but potentially different capacity
    a = Bitset.new(10)
    a.set(3).set(7)

    b = Bitset.new(10)
    b.set(200)  # grows capacity
    b.clear(200)  # This won't shrink capacity, but len goes back... actually len stays at 201
    # Let's do this differently
    b = Bitset.new(10)
    b.set(3).set(7)
    assert_equal a, b
  end

  # ========================================================================
  # Auto-growth edge cases
  # ========================================================================

  def test_growth_from_zero_capacity
    bs = Bitset.new(0)
    assert_equal 0, bs.capacity
    bs.set(0)
    assert_equal 64, bs.capacity
    assert_equal 1, bs.size
  end

  def test_growth_large_jump
    bs = Bitset.new(10)
    bs.set(1000)
    assert_equal 1001, bs.size
    assert bs.capacity >= 1001
    assert bs.test?(1000)
    # Other bits should still be zero
    refute bs.test?(500)
  end

  def test_growth_preserves_existing_bits
    bs = Bitset.new(10)
    bs.set(0).set(5).set(9)
    bs.set(100)  # triggers growth
    assert bs.test?(0)
    assert bs.test?(5)
    assert bs.test?(9)
    assert bs.test?(100)
  end

  # ========================================================================
  # Clean-trailing-bits invariant
  # ========================================================================

  def test_clean_trailing_bits_after_not
    # NOT on a non-word-aligned bitset must clean trailing bits.
    bs = Bitset.from_binary_str("101")  # len=3
    inv = ~bs
    # Should have len=3, bits: 010
    assert_equal 3, inv.size
    assert_equal 0b010, inv.to_integer
    # popcount should reflect only the 3 valid bits
    assert_equal 1, inv.popcount
  end

  def test_clean_trailing_bits_popcount
    # Create bitset with len not on word boundary, check popcount is correct
    bs = Bitset.new(5)
    bs.set(0).set(1).set(2).set(3).set(4)
    assert_equal 5, bs.popcount  # should not count any trailing bits
  end

  # ========================================================================
  # Multi-word operations
  # ========================================================================

  def test_and_multi_word
    a = Bitset.new(128)
    b = Bitset.new(128)
    (0...128).each { |i| a.set(i) }
    (64...128).each { |i| b.set(i) }
    c = a & b
    assert_equal 64, c.popcount  # only upper 64 bits
  end

  def test_or_multi_word
    a = Bitset.new(128)
    b = Bitset.new(128)
    (0...64).each { |i| a.set(i) }
    (64...128).each { |i| b.set(i) }
    c = a | b
    assert_equal 128, c.popcount  # all bits
  end

  def test_xor_multi_word
    a = Bitset.new(128)
    b = Bitset.new(128)
    (0...128).each { |i| a.set(i) }
    (0...128).each { |i| b.set(i) }
    c = a ^ b
    assert_equal 0, c.popcount  # all cancel out
  end

  # ========================================================================
  # Misc / edge cases
  # ========================================================================

  def test_set_clear_toggle_bit_zero
    bs = Bitset.new(1)
    bs.set(0)
    assert bs.test?(0)
    bs.clear(0)
    refute bs.test?(0)
    bs.toggle(0)
    assert bs.test?(0)
  end

  def test_from_integer_roundtrip_various
    [0, 1, 2, 3, 63, 64, 65, 127, 128, 255, 256, 1023, 1024].each do |val|
      bs = Bitset.from_integer(val)
      assert_equal val, bs.to_integer, "roundtrip failed for #{val}"
    end
  end

  def test_from_binary_str_long
    str = "1" + "0" * 199
    bs = Bitset.from_binary_str(str)
    assert_equal 200, bs.size
    assert bs.test?(199)
    refute bs.test?(0)
    assert_equal 1, bs.popcount
  end

  def test_iteration_order_ascending
    bs = Bitset.new(200)
    indices = [199, 0, 100, 50, 150]
    indices.each { |i| bs.set(i) }
    result = bs.each_set_bit.to_a
    assert_equal result, result.sort
  end

  def test_operations_on_size_1_bitsets
    a = Bitset.from_binary_str("1")
    b = Bitset.from_binary_str("0")

    assert_equal 0, (a & b).to_integer
    assert_equal 1, (a | b).to_integer
    assert_equal 1, (a ^ b).to_integer
    assert_equal 0, (~a).to_integer
    assert_equal 1, (~b).to_integer
  end

  def test_not_on_word_aligned_bitset
    # len=64 (exactly one word, all set)
    bs = Bitset.new(64)
    64.times { |i| bs.set(i) }
    inv = ~bs
    assert_equal 0, inv.popcount
    assert_equal 64, inv.size
  end

  def test_not_on_multi_word_partial
    # len=100 (2 words, partial last word)
    bs = Bitset.new(100)
    100.times { |i| bs.set(i) }
    inv = ~bs
    assert_equal 0, inv.popcount
    assert_equal 100, inv.size
  end

  def test_and_not_multi_word
    a = Bitset.new(128)
    128.times { |i| a.set(i) }
    b = Bitset.new(128)
    (0...64).each { |i| b.set(i) }
    c = a.and_not(b)
    # Should have bits 64-127 set
    assert_equal 64, c.popcount
    refute c.test?(0)
    assert c.test?(64)
    assert c.test?(127)
  end

  def test_toggle_on_grown_bitset_cleans_trailing
    bs = Bitset.new(5)
    bs.toggle(4)  # set bit 4
    assert_equal 1, bs.popcount
    # Ensure trailing bits beyond 5 are clean
    inv = ~bs
    assert_equal 4, inv.popcount  # bits 0,1,2,3 should be set, bit 4 cleared
  end

  # ========================================================================
  # Additional branch coverage tests
  # ========================================================================

  def test_ensure_capacity_within_capacity_but_beyond_len
    # Tests the branch in ensure_capacity where i < capacity but i >= @len
    bs = Bitset.new(5)  # len=5, capacity=64
    bs.set(30)  # i=30 < capacity=64, but i=30 >= len=5
    assert_equal 31, bs.size
    assert bs.test?(30)
  end

  def test_all_on_single_bit_set
    bs = Bitset.new(1)
    bs.set(0)
    assert bs.all?
  end

  def test_all_on_single_bit_unset
    bs = Bitset.new(1)
    refute bs.all?
  end

  def test_equality_different_word_count
    # a has more words than b due to different capacity
    a = Bitset.new(10)
    a.set(3)
    b = Bitset.new(10)
    b.set(3)
    b.set(200)  # grows capacity
    # now b has len=201, a has len=10 => different len => not equal
    refute_equal a, b
  end

  def test_clean_trailing_bits_on_word_boundary_len
    # len that is exactly a multiple of 64 -> remaining=0, clean_trailing_bits returns early
    bs = Bitset.new(64)
    64.times { |i| bs.set(i) }
    inv = ~bs
    assert_equal 0, inv.popcount
  end

  def test_clean_trailing_bits_on_empty
    # Empty bitset: clean_trailing_bits returns early
    bs = Bitset.new(0)
    assert_equal 0, bs.popcount
  end

  def test_each_set_bit_zero_sized_bitset
    bs = Bitset.new(0)
    bits = bs.each_set_bit.to_a
    assert_equal [], bits
  end

  def test_from_integer_exactly_64_bits
    value = (1 << 63) | 1  # bit 0 and bit 63
    bs = Bitset.from_integer(value)
    assert_equal 64, bs.size
    assert bs.test?(0)
    assert bs.test?(63)
    refute bs.test?(1)
  end

  def test_bitwise_ops_both_empty
    a = Bitset.new(0)
    b = Bitset.new(0)
    assert_equal 0, (a & b).size
    assert_equal 0, (a | b).size
    assert_equal 0, (a ^ b).size
    assert_equal 0, a.and_not(b).size
  end

  def test_and_with_zero_words
    # one empty, one non-empty
    a = Bitset.new(0)
    b = Bitset.from_integer(0b1010)
    c = a & b
    assert_equal 0, c.to_integer
  end

  def test_or_with_zero_words
    a = Bitset.new(0)
    b = Bitset.from_integer(0b1010)
    c = a | b
    assert_equal 0b1010, c.to_integer
  end

  def test_xor_with_zero_words
    a = Bitset.new(0)
    b = Bitset.from_integer(0b1010)
    c = a ^ b
    assert_equal 0b1010, c.to_integer
  end

  def test_set_within_capacity_already_in_len
    # Tests the branch where i < capacity AND i < @len (no growth or len update)
    bs = Bitset.new(100)
    bs.set(50)
    assert_equal 100, bs.size  # len unchanged
    assert bs.test?(50)
  end

  def test_toggle_clears_bit_within_len
    # Toggle a set bit -> clears it, no growth needed
    bs = Bitset.new(100)
    bs.set(50)
    bs.toggle(50)
    refute bs.test?(50)
    assert_equal 100, bs.size
  end

  def test_clear_at_boundary
    bs = Bitset.new(64)
    bs.set(63)
    bs.clear(63)
    refute bs.test?(63)
  end

  def test_from_binary_str_only_zeros
    bs = Bitset.from_binary_str("000")
    assert_equal 3, bs.size
    assert_equal 0, bs.popcount
    assert_equal "000", bs.to_binary_str
  end

  def test_equality_nil
    bs = Bitset.from_integer(5)
    refute_equal bs, nil
  end

  # Tests for XOR/AND-NOT where self has fewer words than other
  def test_xor_self_shorter_than_other
    a = Bitset.from_integer(0b11)     # len=2, 1 word
    b = Bitset.new(200)               # len=200, 4 words
    b.set(100)
    c = a ^ b
    assert c.test?(0)
    assert c.test?(1)
    assert c.test?(100)
  end

  def test_and_not_self_shorter_than_other
    a = Bitset.from_integer(0b11)     # len=2, 1 word
    b = Bitset.new(200)               # len=200, 4 words
    b.set(100)
    c = a.and_not(b)
    # a has bits 0,1 set. b has bit 100 set.
    # a & ~b should keep bits 0,1 (not in b) and for word 1+, a=0 so result=0
    assert c.test?(0)
    assert c.test?(1)
    refute c.test?(100)
  end

  def test_each_set_bit_ignores_trailing_capacity_bits
    # Craft a bitset where the last word has trailing bits beyond len.
    # We can't directly set them via public API (clean_trailing_bits prevents it),
    # but we can test the boundary: set a bit at len-1 in a non-word-aligned bitset.
    bs = Bitset.new(3)
    bs.set(0).set(1).set(2)
    bits = bs.each_set_bit.to_a
    assert_equal [0, 1, 2], bits
    # All bits should be < len
    bits.each { |b| assert b < 3 }
  end

  def test_equality_same_len_different_capacity
    a = Bitset.new(10)
    a.set(3)
    b = Bitset.new(10)
    b.set(3)
    assert_equal a, b
  end

  def test_xor_self_longer_than_other
    # self has more words than other -> other.word_at returns 0 for missing words
    a = Bitset.new(200)               # 4 words
    a.set(100)
    b = Bitset.from_integer(0b11)     # 1 word
    c = a ^ b
    assert c.test?(0)
    assert c.test?(1)
    assert c.test?(100)
  end

  def test_and_not_self_longer_words
    # self has more words than other
    a = Bitset.new(200)
    a.set(0).set(100)
    b = Bitset.from_integer(0b01)   # 1 word, bit 0 set
    c = a.and_not(b)
    refute c.test?(0)    # removed by b
    assert c.test?(100)  # kept, b doesn't have this word
  end
end
