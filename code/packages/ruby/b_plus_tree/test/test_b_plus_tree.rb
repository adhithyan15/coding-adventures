# frozen_string_literal: true

require_relative "test_helper"

class TestBPlusTree < Minitest::Test
  BPlusTree = CodingAdventures::BPlusTree::BPlusTree

  def test_range_scan_and_iteration_work
    tree = BPlusTree.new(3)
    tree.insert(5, "e")
    tree.insert(1, "a")
    tree.insert(3, "c")
    tree.insert(7, "g")

    assert_equal [[3, "c"], [5, "e"]], tree.range_scan(2, 6)
    assert_equal [[1, "a"], [3, "c"], [5, "e"], [7, "g"]], tree.full_scan
    assert_equal [1, 3, 5, 7], tree.iter.to_a
    assert_equal 4, tree.len
    assert tree.is_valid
  end

  def test_items_indexing_and_display_work
    tree = BPlusTree.new(2)
    tree.insert(2, "two")
    tree.insert(1, "one")
    tree.insert(3, "three")
    tree.insert(2, "TWO")

    assert_equal "TWO", tree[2]
    tree[1] = "ONE"
    assert_equal "ONE", tree[1]
    assert_equal [[1, "ONE"], [2, "TWO"], [3, "three"]], tree.items.to_a
    assert_includes tree.to_s, "BPlusTree(t=2"
  end
end
