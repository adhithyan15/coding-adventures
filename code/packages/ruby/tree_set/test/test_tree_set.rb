# frozen_string_literal: true

require_relative "test_helper"

class TestTreeSet < Minitest::Test
  TreeSet = CodingAdventures::TreeSet::TreeSet

  def test_basic_insertion_lookup_and_iteration
    tree = TreeSet.new([5, 1, 3, 3, 9])
    tree.add(7)

    assert_equal [1, 3, 5, 7, 9], tree.to_a
    assert_equal 5, tree.size
    assert_equal 5, tree.length
    assert tree.include?(7)
    assert tree.contains?(7)
    refute tree.include?(2)
    assert_equal [1, 3, 5, 7, 9], tree.each.to_a
  end

  def test_rank_selection_and_range
    tree = TreeSet.from_values([10, 20, 30, 40])

    assert_equal 2, tree.rank(25)
    assert_equal 10, tree.by_rank(0)
    assert_equal 30, tree.kth_smallest(3)
    assert_equal 20, tree.predecessor(30)
    assert_equal 40, tree.successor(30)
    assert_equal [20, 30], tree.range(15, 35)
    assert_equal [30], tree.range(20, 35, false)
  end

  def test_set_algebra
    left = TreeSet.from_values([1, 2, 3, 5])
    right = TreeSet.from_values([3, 4, 5, 6])

    assert_equal [1, 2, 3, 4, 5, 6], left.union(right).to_a
    assert_equal [3, 5], left.intersection(right).to_a
    assert_equal [1, 2], left.difference(right).to_a
    assert_equal [1, 2, 4, 6], left.symmetric_difference(right).to_a
    assert left.subset?(left.union(right))
    assert left.superset?(left.intersection(right))
    assert left.disjoint?(TreeSet.from_values([8, 9]))
    assert left.equals(TreeSet.from_values([1, 2, 3, 5]))
  end

  def test_custom_comparator_and_delete
    by_length = TreeSet.new([], lambda do |left, right|
      if left.length < right.length
        -1
      elsif left.length > right.length
        1
      else
        left <=> right
      end
    end)
    by_length.add("banana")
    by_length.add("fig")
    by_length.add("apple")

    assert_equal %w[fig apple banana], by_length.to_a
    assert by_length.delete("apple")
    assert_equal %w[fig banana], by_length.to_a
    refute by_length.delete("missing")
    assert_equal 'TreeSet(["fig", "banana"])', by_length.to_s
  end
end
