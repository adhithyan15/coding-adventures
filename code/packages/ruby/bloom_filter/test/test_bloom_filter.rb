# frozen_string_literal: true

require_relative "test_helper"

class BloomFilterTest < Minitest::Test
  BloomFilter = CodingAdventures::BloomFilter::BloomFilter

  def test_default_starts_empty
    filter = BloomFilter.new
    assert_equal 0, filter.bits_set
    assert_equal 0.0, filter.fill_ratio
    assert_equal 0.0, filter.estimated_false_positive_rate
    refute filter.over_capacity?
    refute filter.contains?("anything")
  end

  def test_no_false_negatives
    filter = BloomFilter.new(expected_items: 1_000, false_positive_rate: 0.01)
    250.times { |i| filter.add("item-#{i}") }
    250.times { |i| assert filter.contains?("item-#{i}") }
    assert_operator filter.bits_set, :>, 0
  end

  def test_from_params
    filter = BloomFilter.from_params(bit_count: 10_000, hash_count: 7)
    assert_equal 10_000, filter.bit_count
    assert_equal 7, filter.hash_count
    assert_equal 1_250, filter.size_bytes
    filter.add("hello")
    assert filter.include?("hello")
    refute filter.over_capacity?
  end

  def test_duplicate_add_does_not_double_count_bits
    filter = BloomFilter.new
    filter.add("dup")
    after_first = filter.bits_set
    filter.add("dup")
    assert_equal after_first, filter.bits_set
  end

  def test_sizing_helpers
    m = BloomFilter.optimal_m(1_000_000, 0.01)
    k = BloomFilter.optimal_k(m, 1_000_000)
    assert_operator m, :>, 9_000_000
    assert_equal 7, k
    assert_operator BloomFilter.capacity_for_memory(1_000_000, 0.01), :>, 0
  end

  def test_over_capacity_and_rendering
    filter = BloomFilter.new(expected_items: 3, false_positive_rate: 0.01)
    %w[a b c].each { |value| filter.add(value) }
    refute filter.over_capacity?
    filter.add("d")
    assert filter.over_capacity?
    assert_operator filter.estimated_false_positive_rate, :>, 0
    assert_match(/BloomFilter/, filter.to_s)
  end

  def test_various_element_types
    filter = BloomFilter.new(expected_items: 100)
    [42, 3.14, true, nil, [1, 2], "cafe\u0301"].each do |value|
      filter.add(value)
      assert filter.contains?(value)
    end
  end

  def test_invalid_parameters
    assert_raises(ArgumentError) { BloomFilter.new(expected_items: 0) }
    assert_raises(ArgumentError) { BloomFilter.new(false_positive_rate: 0) }
    assert_raises(ArgumentError) { BloomFilter.new(false_positive_rate: 1) }
    assert_raises(ArgumentError) { BloomFilter.from_params(bit_count: 0, hash_count: 1) }
    assert_raises(ArgumentError) { BloomFilter.from_params(bit_count: 1, hash_count: 0) }
  end
end
