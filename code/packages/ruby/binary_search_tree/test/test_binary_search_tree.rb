# frozen_string_literal: true

require_relative "test_helper"

class TestBinarySearchTree < Minitest::Test
  BST = CodingAdventures::BinarySearchTree::BinarySearchTree

  def test_insert_search_and_delete_work
    tree = BST.empty
    [5, 1, 8, 3, 7].each { |value| tree = tree.insert(value) }

    assert_equal [1, 3, 5, 7, 8], tree.to_sorted_array
    assert_equal 5, tree.size
    assert tree.contains(7)
    assert_equal 1, tree.min_value
    assert_equal 8, tree.max_value
    assert_equal 3, tree.predecessor(5)
    assert_equal 7, tree.successor(5)
    assert_equal 2, tree.rank(4)
    assert_equal 7, tree.kth_smallest(4)

    tree = tree.delete(5)
    refute tree.contains(5)
    assert tree.is_valid
  end

  def test_from_sorted_array_builds_balanced_tree
    tree = BST.from_sorted_array([1, 2, 3, 4, 5, 6, 7])

    assert_equal [1, 2, 3, 4, 5, 6, 7], tree.to_sorted_array
    assert_equal 2, tree.height
    assert_equal 7, tree.size
    assert tree.is_valid
  end

  def test_edge_cases_cover_empty_tree_and_root_accessors
    tree = BST.empty

    assert_nil tree.search(1)
    assert_nil tree.min_value
    assert_nil tree.max_value
    assert_nil tree.predecessor(1)
    assert_nil tree.successor(1)
    assert_nil tree.kth_smallest(0)
    assert_equal 0, tree.rank(1)
    assert_match(/BinarySearchTree/, tree.to_s)

    tree = BST.from_sorted_array([2, 4, 6, 8])
    assert_equal 6, tree.root_node.value
    assert_equal tree.root_node, tree.root

    duplicate = tree.insert(4)
    assert_equal tree.to_sorted_array, duplicate.to_sorted_array

    assert_equal [4, 6, 8], tree.delete(2).to_sorted_array
  end
end
