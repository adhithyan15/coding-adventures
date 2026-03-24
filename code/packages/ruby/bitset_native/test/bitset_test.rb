# frozen_string_literal: true

# --------------------------------------------------------------------------
# bitset_test.rb — Tests for the Rust-backed BitsetNative::Bitset
# --------------------------------------------------------------------------
#
# These tests exercise every public method of the native Bitset extension
# to ensure the Rust implementation is correctly exposed to Ruby.

require_relative "test_helper"

class BitsetTest < Minitest::Test
  # ========================================================================
  # Constructor tests
  # ========================================================================

  def test_new_creates_empty_bitset
    bs = CodingAdventures::BitsetNative::Bitset.new(100)
    assert_equal 100, bs.len
    assert_equal 0, bs.popcount
    assert bs.none?
  end

  def test_new_zero_size
    bs = CodingAdventures::BitsetNative::Bitset.new(0)
    assert_equal 0, bs.len
    assert_equal 0, bs.popcount
    assert bs.none?
    assert bs.all?  # vacuous truth
    assert bs.empty?
  end

  def test_new_capacity_rounds_up_to_64
    bs = CodingAdventures::BitsetNative::Bitset.new(1)
    assert_equal 1, bs.len
    assert_equal 64, bs.capacity

    bs = CodingAdventures::BitsetNative::Bitset.new(64)
    assert_equal 64, bs.len
    assert_equal 64, bs.capacity

    bs = CodingAdventures::BitsetNative::Bitset.new(65)
    assert_equal 65, bs.len
    assert_equal 128, bs.capacity
  end

  # ========================================================================
  # from_integer tests
  # ========================================================================

  def test_from_integer_zero
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(0)
    assert_equal 0, bs.len
    assert_equal 0, bs.to_integer
  end

  def test_from_integer_small
    # 5 = binary 101, bits 0 and 2 set
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(5)
    assert_equal 3, bs.len
    assert bs.test?(0)
    refute bs.test?(1)
    assert bs.test?(2)
    assert_equal 5, bs.to_integer
  end

  def test_from_integer_power_of_two
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(128)
    assert_equal 8, bs.len
    assert bs.test?(7)
    refute bs.test?(0)
    assert_equal 128, bs.to_integer
  end

  def test_from_integer_255
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(255)
    assert_equal 8, bs.len
    assert_equal 8, bs.popcount
    assert bs.all?
    assert_equal 255, bs.to_integer
  end

  # ========================================================================
  # from_binary_str tests
  # ========================================================================

  def test_from_binary_str_simple
    bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("1010")
    assert_equal 4, bs.len
    refute bs.test?(0)  # rightmost char = bit 0
    assert bs.test?(1)
    refute bs.test?(2)
    assert bs.test?(3)  # leftmost char = highest bit
  end

  def test_from_binary_str_all_ones
    bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("1111")
    assert_equal 4, bs.len
    assert bs.all?
    assert_equal 15, bs.to_integer
  end

  def test_from_binary_str_all_zeros
    bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("0000")
    assert_equal 4, bs.len
    assert bs.none?
    assert_equal 0, bs.to_integer
  end

  def test_from_binary_str_empty
    bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("")
    assert_equal 0, bs.len
    assert bs.empty?
  end

  def test_from_binary_str_invalid_raises
    assert_raises(CodingAdventures::BitsetNative::BitsetError) do
      CodingAdventures::BitsetNative::Bitset.from_binary_str("102")
    end
  end

  def test_from_binary_str_invalid_letters
    assert_raises(CodingAdventures::BitsetNative::BitsetError) do
      CodingAdventures::BitsetNative::Bitset.from_binary_str("abc")
    end
  end

  # ========================================================================
  # Single-bit operation tests
  # ========================================================================

  def test_set_and_test
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.set(5)
    assert bs.test?(5)
    refute bs.test?(3)
  end

  def test_set_is_idempotent
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.set(5)
    bs.set(5)
    assert_equal 1, bs.popcount
  end

  def test_set_auto_grows
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.set(100)
    assert_equal 101, bs.len
    assert bs.test?(100)
  end

  def test_clear
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.set(5)
    assert bs.test?(5)
    bs.clear(5)
    refute bs.test?(5)
  end

  def test_clear_beyond_len_is_noop
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.clear(999)  # no error, no growth
    assert_equal 10, bs.len
  end

  def test_test_beyond_len_returns_false
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    refute bs.test?(999)
  end

  def test_toggle
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.toggle(5)        # 0 -> 1
    assert bs.test?(5)
    bs.toggle(5)        # 1 -> 0
    refute bs.test?(5)
  end

  def test_toggle_auto_grows
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.toggle(200)
    assert bs.test?(200)
    assert_equal 201, bs.len
  end

  # ========================================================================
  # Bulk bitwise operation tests
  # ========================================================================

  def test_and
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1100)  # bits 2,3
    b = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)  # bits 1,3
    c = a.and(b)
    assert_equal 0b1000, c.to_integer  # only bit 3
  end

  def test_or
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1100)  # bits 2,3
    b = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)  # bits 1,3
    c = a.or(b)
    assert_equal 0b1110, c.to_integer  # bits 1,2,3
  end

  def test_xor
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1100)  # bits 2,3
    b = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)  # bits 1,3
    c = a.xor(b)
    assert_equal 0b0110, c.to_integer  # bits 1,2
  end

  def test_not
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)  # len=4, bits 1,3
    b = a.not
    assert_equal 0b0101, b.to_integer  # len=4, bits 0,2
    assert_equal 4, b.len
  end

  def test_and_not
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1110)  # bits 1,2,3
    b = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)  # bits 1,3
    c = a.and_not(b)
    assert_equal 0b0100, c.to_integer  # only bit 2
  end

  def test_bitwise_ops_dont_modify_operands
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1100)
    b = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)

    a.and(b)
    assert_equal 0b1100, a.to_integer
    assert_equal 0b1010, b.to_integer

    a.or(b)
    assert_equal 0b1100, a.to_integer

    a.xor(b)
    assert_equal 0b1100, a.to_integer

    a.not
    assert_equal 0b1100, a.to_integer
  end

  def test_bitwise_ops_with_different_lengths
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b11)   # len=2
    b = CodingAdventures::BitsetNative::Bitset.from_integer(0b1100) # len=4
    c = a.or(b)
    assert_equal 0b1111, c.to_integer
    assert_equal 4, c.len  # max of the two lengths
  end

  # ========================================================================
  # Counting and query operation tests
  # ========================================================================

  def test_popcount
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(0b10110)
    assert_equal 3, bs.popcount  # bits 1, 2, 4
  end

  def test_popcount_empty
    bs = CodingAdventures::BitsetNative::Bitset.new(100)
    assert_equal 0, bs.popcount
  end

  def test_len
    bs = CodingAdventures::BitsetNative::Bitset.new(200)
    assert_equal 200, bs.len
  end

  def test_capacity
    bs = CodingAdventures::BitsetNative::Bitset.new(200)
    assert_equal 256, bs.capacity  # ceil(200/64) * 64 = 4 * 64 = 256
  end

  def test_any_true
    bs = CodingAdventures::BitsetNative::Bitset.new(100)
    bs.set(50)
    assert bs.any?
  end

  def test_any_false
    bs = CodingAdventures::BitsetNative::Bitset.new(100)
    refute bs.any?
  end

  def test_all_true
    bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("1111")
    assert bs.all?
  end

  def test_all_false
    bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("1110")
    refute bs.all?
  end

  def test_all_vacuous_truth
    bs = CodingAdventures::BitsetNative::Bitset.new(0)
    assert bs.all?
  end

  def test_none_true
    bs = CodingAdventures::BitsetNative::Bitset.new(100)
    assert bs.none?
  end

  def test_none_false
    bs = CodingAdventures::BitsetNative::Bitset.new(100)
    bs.set(0)
    refute bs.none?
  end

  def test_empty_true
    bs = CodingAdventures::BitsetNative::Bitset.new(0)
    assert bs.empty?
  end

  def test_empty_false
    bs = CodingAdventures::BitsetNative::Bitset.new(1)
    refute bs.empty?
  end

  # ========================================================================
  # Iteration tests
  # ========================================================================

  def test_each_set_bit
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(0b10100101)
    assert_equal [0, 2, 5, 7], bs.each_set_bit
  end

  def test_each_set_bit_empty
    bs = CodingAdventures::BitsetNative::Bitset.new(100)
    assert_equal [], bs.each_set_bit
  end

  def test_each_set_bit_single
    bs = CodingAdventures::BitsetNative::Bitset.new(100)
    bs.set(42)
    assert_equal [42], bs.each_set_bit
  end

  def test_each_set_bit_consecutive
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.set(0)
    bs.set(1)
    bs.set(2)
    assert_equal [0, 1, 2], bs.each_set_bit
  end

  def test_each_set_bit_spanning_words
    # Set bits in different 64-bit words to test word boundary handling
    bs = CodingAdventures::BitsetNative::Bitset.new(200)
    bs.set(0)    # word 0
    bs.set(63)   # word 0 (last bit)
    bs.set(64)   # word 1 (first bit)
    bs.set(127)  # word 1 (last bit)
    bs.set(128)  # word 2 (first bit)
    assert_equal [0, 63, 64, 127, 128], bs.each_set_bit
  end

  # ========================================================================
  # Conversion tests
  # ========================================================================

  def test_to_integer
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(42)
    assert_equal 42, bs.to_integer
  end

  def test_to_integer_empty
    bs = CodingAdventures::BitsetNative::Bitset.new(0)
    assert_equal 0, bs.to_integer
  end

  def test_to_integer_negative_when_too_large
    # Create a bitset with bits set in multiple words (>64 bits)
    # Returns -1 when the bitset cannot fit in a single 64-bit integer
    # (On some platforms, -1 as c_long may render as an unsigned value)
    bs = CodingAdventures::BitsetNative::Bitset.new(200)
    bs.set(0)
    bs.set(100)  # in word 1
    result = bs.to_integer
    assert(result < 0 || result == 4_294_967_295, "expected -1 or 4294967295, got #{result}")
  end

  def test_to_binary_str
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(5)
    assert_equal "101", bs.to_binary_str
  end

  def test_to_binary_str_empty
    bs = CodingAdventures::BitsetNative::Bitset.new(0)
    assert_equal "", bs.to_binary_str
  end

  def test_to_binary_str_with_leading_zeros
    bs = CodingAdventures::BitsetNative::Bitset.from_binary_str("0010")
    assert_equal "0010", bs.to_binary_str
  end

  def test_to_s
    bs = CodingAdventures::BitsetNative::Bitset.from_integer(5)
    assert_equal "Bitset(101)", bs.to_s
  end

  def test_to_s_empty
    bs = CodingAdventures::BitsetNative::Bitset.new(0)
    assert_equal "Bitset()", bs.to_s
  end

  # ========================================================================
  # Equality tests
  # ========================================================================

  def test_equality_same_bitsets
    a = CodingAdventures::BitsetNative::Bitset.from_integer(42)
    b = CodingAdventures::BitsetNative::Bitset.from_integer(42)
    assert_equal a, b
  end

  def test_equality_different_values
    a = CodingAdventures::BitsetNative::Bitset.from_integer(42)
    b = CodingAdventures::BitsetNative::Bitset.from_integer(43)
    refute_equal a, b
  end

  def test_equality_different_lengths
    # Same bits set but different logical lengths should not be equal
    a = CodingAdventures::BitsetNative::Bitset.from_binary_str("101")
    b = CodingAdventures::BitsetNative::Bitset.from_binary_str("0101")
    refute_equal a, b
  end

  def test_equality_empty_bitsets
    a = CodingAdventures::BitsetNative::Bitset.new(0)
    b = CodingAdventures::BitsetNative::Bitset.new(0)
    assert_equal a, b
  end

  # ========================================================================
  # Round-trip tests
  # ========================================================================

  def test_from_integer_to_integer_roundtrip
    [0, 1, 5, 42, 255, 1023, 65535].each do |n|
      bs = CodingAdventures::BitsetNative::Bitset.from_integer(n)
      assert_equal n, bs.to_integer, "round-trip failed for #{n}"
    end
  end

  def test_from_binary_str_to_binary_str_roundtrip
    ["", "0", "1", "1010", "11111111", "10000001", "0010"].each do |s|
      bs = CodingAdventures::BitsetNative::Bitset.from_binary_str(s)
      assert_equal s, bs.to_binary_str, "round-trip failed for #{s.inspect}"
    end
  end

  # ========================================================================
  # Error class tests
  # ========================================================================

  def test_bitset_error_is_standard_error
    assert CodingAdventures::BitsetNative::BitsetError < StandardError
  end

  def test_bitset_error_has_message
    err = assert_raises(CodingAdventures::BitsetNative::BitsetError) do
      CodingAdventures::BitsetNative::Bitset.from_binary_str("xyz")
    end
    assert_match(/invalid binary string/, err.message)
  end

  # ========================================================================
  # Edge case tests
  # ========================================================================

  def test_set_bit_zero
    bs = CodingAdventures::BitsetNative::Bitset.new(10)
    bs.set(0)
    assert bs.test?(0)
    assert_equal 1, bs.popcount
  end

  def test_large_bitset
    bs = CodingAdventures::BitsetNative::Bitset.new(10000)
    bs.set(0)
    bs.set(5000)
    bs.set(9999)
    assert_equal 3, bs.popcount
    assert_equal [0, 5000, 9999], bs.each_set_bit
  end

  def test_all_bits_set_in_word
    # Set all 64 bits in the first word
    bs = CodingAdventures::BitsetNative::Bitset.new(64)
    64.times { |i| bs.set(i) }
    assert bs.all?
    assert_equal 64, bs.popcount
  end

  def test_not_of_not_is_identity
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)
    b = a.not.not
    assert_equal a, b
  end

  def test_and_with_self_is_identity
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)
    b = a.and(a)
    assert_equal a, b
  end

  def test_or_with_self_is_identity
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)
    b = a.or(a)
    assert_equal a, b
  end

  def test_xor_with_self_is_zero
    a = CodingAdventures::BitsetNative::Bitset.from_integer(0b1010)
    b = a.xor(a)
    assert_equal 0, b.popcount
    assert bs_none?(b)
  end

  def test_de_morgans_law
    # ~(A & B) == (~A) | (~B)
    a = CodingAdventures::BitsetNative::Bitset.from_binary_str("11001010")
    b = CodingAdventures::BitsetNative::Bitset.from_binary_str("10101100")

    lhs = a.and(b).not
    rhs = a.not.or(b.not)
    assert_equal lhs, rhs
  end

  private

  def bs_none?(bs)
    bs.none?
  end
end
