# frozen_string_literal: true

require_relative "test_helper"

class FenwickTreeTest < Minitest::Test
  FenwickTree = CodingAdventures::FenwickTree::FenwickTree
  FenwickError = CodingAdventures::FenwickTree::FenwickError
  IndexOutOfRangeError = CodingAdventures::FenwickTree::IndexOutOfRangeError
  EmptyTreeError = CodingAdventures::FenwickTree::EmptyTreeError

  def assert_close(expected, actual)
    assert_in_delta expected, actual, 1e-9
  end

  def test_new_and_from_list
    tree = FenwickTree.from_list([3, 2, 1, 7, 4])
    assert_equal 5, tree.length
    refute tree.empty?
    [3, 5, 6, 13, 17].each_with_index do |expected, index|
      assert_close expected, tree.prefix_sum(index + 1)
    end

    assert FenwickTree.new(0).empty?
    assert_raises(FenwickError) { FenwickTree.new(-1) }
  end

  def test_prefix_range_point_and_update
    tree = FenwickTree.from_list([3, 2, 1, 7, 4])
    assert_close 0, tree.prefix_sum(0)
    assert_close 10, tree.range_sum(2, 4)
    assert_close 7, tree.point_query(4)

    tree.update(3, 5)
    assert_close 6, tree.point_query(3)
    assert_close 11, tree.prefix_sum(3)
  end

  def test_find_kth
    tree = FenwickTree.from_list([1, 2, 3, 4, 5])
    assert_equal 1, tree.find_kth(1)
    assert_equal 2, tree.find_kth(2)
    assert_equal 2, tree.find_kth(3)
    assert_equal 3, tree.find_kth(4)
    assert_equal 4, tree.find_kth(10)
  end

  def test_find_kth_errors
    assert_raises(EmptyTreeError) { FenwickTree.new(0).find_kth(1) }
    tree = FenwickTree.from_list([1, 2, 3])
    assert_raises(FenwickError) { tree.find_kth(0) }
    assert_raises(FenwickError) { tree.find_kth(100) }
  end

  def test_invalid_indices_and_ranges
    tree = FenwickTree.from_list([1, 2, 3])
    assert_raises(IndexOutOfRangeError) { tree.prefix_sum(4) }
    assert_raises(IndexOutOfRangeError) { tree.update(0, 1) }
    assert_raises(FenwickError) { tree.range_sum(3, 1) }
    assert_raises(IndexOutOfRangeError) { tree.range_sum(0, 3) }
  end

  def test_brute_force_and_rendering
    values = [5, -2, 7, 1.5, 4.5]
    tree = FenwickTree.from_list(values)
    values.each_index do |index|
      assert_close values[0..index].sum, tree.prefix_sum(index + 1)
    end
    assert_equal values.length, tree.bit_array.length
    assert_match(/FenwickTree/, tree.to_s)
  end
end
