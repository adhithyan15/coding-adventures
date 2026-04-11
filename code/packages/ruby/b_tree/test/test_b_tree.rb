# frozen_string_literal: true

require_relative "test_helper"

class TestBTree < Minitest::Test
  BTree = CodingAdventures::BTree::BTree

  def test_insert_search_delete_and_ranges_work
    tree = BTree.new(2)
    tree.insert(5, "e")
    tree.insert(1, "a")
    tree.insert(3, "c")
    tree.insert(7, "g")

    assert_equal "c", tree.search(3)
    assert_equal 1, tree.min_key
    assert_equal 7, tree.max_key
    assert_equal [[3, "c"], [5, "e"]], tree.range_query(2, 6)
    assert_equal [[1, "a"], [3, "c"], [5, "e"], [7, "g"]], tree.inorder
    assert_equal 4, tree.len
    assert tree.is_valid

    tree.delete(5)
    refute tree.contains(5)
  end

  def test_indexing_and_display_work
    tree = BTree.new(3)
    tree[2] = "two"
    tree[1] = "one"
    tree[2] = "TWO"

    assert_equal "TWO", tree[2]
    assert_equal({ 1 => "one", 2 => "TWO" }, tree.to_h)
    assert_includes tree.to_s, "BTree(t=3"
  end

  def test_empty_tree_and_enumeration_work
    tree = BTree.new(4)

    assert tree.is_empty
    assert_equal 0, tree.height
    assert_nil tree.min_key
    assert_nil tree.max_key
    assert_equal [], tree.range_query(5, 1)

    tree.insert(3, "three")
    tree.insert(1, "one")

    assert_equal [[1, "one"], [3, "three"]], tree.each.to_a
    assert_equal "one", tree.search(1)
    assert tree.contains(3)
    assert_equal({ 1 => "one", 3 => "three" }, tree.to_h)
    assert_equal "three", tree[3]
    tree.delete(3)
    refute tree.contains(3)
  end
end
