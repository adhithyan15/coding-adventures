# frozen_string_literal: true

require_relative "test_helper"

class TestTreap < Minitest::Test
  Treap = CodingAdventures::Treap::Treap

  def test_split_merge_and_search_work
    tree = Treap.empty
    [5, 1, 8, 3, 7].each { |value| tree = tree.insert(value) }

    assert_equal [1, 3, 5, 7, 8], tree.to_sorted_array
    assert tree.is_valid_treap
    assert tree.contains(7)
    assert_equal 1, tree.min_key
    assert_equal 8, tree.max_key
    assert_equal 3, tree.predecessor(5)
    assert_equal 7, tree.successor(5)
    assert_equal 5, tree.kth_smallest(3)
    assert_equal 5, tree.size
    assert tree.height >= 1

    left, right = tree.split(5)
    assert_equal [1, 3], left.to_sorted_array
    assert_equal [5, 7, 8], right.to_sorted_array
    merged = Treap.merge(left, right)
    assert_equal [1, 3, 5, 7, 8], merged.to_sorted_array
  end

  def test_delete_and_order_statistics_work
    tree = Treap.empty
    [40, 20, 60, 10, 30, 50, 70].each { |value| tree = tree.insert(value) }

    assert_equal [10, 20, 30, 40, 50, 60, 70], tree.to_sorted_array
    assert_equal 40, tree.kth_smallest(4)

    tree = tree.delete(20)
    refute tree.contains(20)
    assert tree.is_valid_treap
  end
end
