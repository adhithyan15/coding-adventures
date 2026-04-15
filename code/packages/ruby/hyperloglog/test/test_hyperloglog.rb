# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_hyperloglog"

class TestHyperLogLog < Minitest::Test
  include CodingAdventures::HyperLogLog

  def test_add_and_count_is_reasonable
    hll = HyperLogLog.new(precision: 10)
    1_000.times { |value| hll.add("user-#{value}") }

    assert_in_delta 1_000, hll.count, 120
  end

  def test_merge
    left = HyperLogLog.new(precision: 10)
    right = HyperLogLog.new(precision: 10)

    250.times { |value| left.add("left-#{value}") }
    250.times { |value| right.add("right-#{value}") }

    left.merge!(right)
    assert_in_delta 500, left.count, 80
  end

  def test_clear_and_empty
    hll = HyperLogLog.new
    assert hll.empty?
    hll.add("a")
    refute hll.empty?
    hll.clear
    assert hll.empty?
  end
end
