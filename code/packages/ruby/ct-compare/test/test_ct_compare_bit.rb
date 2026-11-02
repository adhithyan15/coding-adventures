# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_ct_compare"

# The bit-returning equality variants (CT01): they agree with the booleans and
# are always exactly 0 or 1.
class TestCtCompareBit < Minitest::Test
  CT = CodingAdventures::CtCompare

  def test_ct_eq_bit_agrees_on_every_byte_pair
    256.times do |a|
      256.times do |b|
        left = a.chr(Encoding::BINARY)
        right = b.chr(Encoding::BINARY)
        want = CT.ct_eq(left, right) ? 1 : 0
        assert_equal want, CT.ct_eq_bit(left, right), "#{a} vs #{b}"
        assert_equal want, CT.ct_eq_fixed_bit(left, right), "#{a} vs #{b}"
      end
    end
  end

  def test_lengths_and_every_bit_position
    assert_equal 1, CT.ct_eq_bit("", "")
    assert_equal 0, CT.ct_eq_bit("a", "ab")
    assert_raises(ArgumentError) { CT.ct_eq_bit([256], "\x00") }
    base = "\x00".b * 64
    64.times do |i|
      8.times do |bit|
        other = base.dup
        other.setbyte(i, 1 << bit)
        assert_equal 0, CT.ct_eq_bit(base, other), "byte #{i} bit #{bit}"
      end
    end
  end

  def test_ct_eq_u64_bit
    values = [0, 1, 2, 2**63, 2**63 - 1, 2**64 - 1, 0x0123456789abcdef]
    values.each do |x|
      values.each { |y| assert_equal((x == y) ? 1 : 0, CT.ct_eq_u64_bit(x, y)) }
      64.times { |bit| assert_equal 0, CT.ct_eq_u64_bit(x, x ^ (1 << bit)) }
    end
    assert_raises(ArgumentError) { CT.ct_eq_u64_bit(-1, 0) }
    assert_raises(ArgumentError) { CT.ct_eq_u64_bit(0, 2**64) }
  end

  def test_equality_bit_feeds_selection
    assert_equal "left", CT.ct_select_bytes("left", "rite", CT.ct_eq_bit("k", "k"))
    assert_equal "rite", CT.ct_select_bytes("left", "rite", CT.ct_eq_bit("k", "j"))
    assert_equal "left", CT.ct_select_bytes("left", "rite", CT.ct_eq_u64_bit(7, 7))
  end
end
