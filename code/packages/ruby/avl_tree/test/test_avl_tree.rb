# frozen_string_literal: true

require_relative "test_helper"

class TestAVLTree < Minitest::Test
  AVLTree = CodingAdventures::AVLTree::AVLTree

  def test_rotations_rebalance_the_tree
    tree = AVLTree.empty
    [10, 20, 30].each { |value| tree = tree.insert(value) }

    assert_equal [10, 20, 30], tree.to_sorted_array
    assert tree.is_valid_bst
    assert tree.is_valid_avl
    assert_equal 1, tree.height
    assert_equal 3, tree.size
  end

  def test_search_and_order_statistics_work
    tree = AVLTree.empty
    [40, 20, 60, 10, 30, 50, 70].each { |value| tree = tree.insert(value) }

    assert_equal 20, tree.search(20).value
    assert tree.contains(50)
    assert_equal 10, tree.min_value
    assert_equal 70, tree.max_value
    assert_equal 30, tree.predecessor(40)
    assert_equal 50, tree.successor(40)
    assert_equal 40, tree.kth_smallest(4)
    assert_equal 3, tree.rank(35)

    tree = tree.delete(20)
    refute tree.contains(20)
    assert tree.is_valid_avl
  end

  def test_edge_cases_and_root_accessors_work
    tree = AVLTree.empty

    assert_nil tree.search(1)
    assert_nil tree.min_value
    assert_nil tree.max_value
    assert_nil tree.predecessor(1)
    assert_nil tree.successor(1)
    assert_nil tree.kth_smallest(0)
    assert_equal 0, tree.rank(1)
    assert_equal 0, tree.balance_factor(nil)
    assert_match(/BinarySearchTree/, tree.to_s)

    [30, 20, 40, 10, 25, 35, 50].each { |value| tree = tree.insert(value) }
    assert_equal 30, tree.root_node.value
    assert_equal tree.root_node, tree.root

    duplicate = tree.insert(25)
    assert_equal tree.to_sorted_array, duplicate.to_sorted_array

    tree = tree.delete(10)
    tree = tree.delete(25)
    assert tree.is_valid_bst
    assert tree.is_valid_avl

    single = AVLTree.new(5)
    assert_equal 5, single.root.value
    assert_equal 0, single.height
    assert_equal 1, single.size
    assert_equal 0, single.balance_factor(single.root)

    rotated = AVLTree.empty
    [30, 20, 10].each { |value| rotated = rotated.insert(value) }
    assert_equal [10, 20, 30], rotated.to_sorted_array
    assert_equal 20, rotated.root.value

    duplicate = rotated.insert(20)
    assert_equal rotated.to_sorted_array, duplicate.to_sorted_array
    assert_equal rotated.to_sorted_array, rotated.delete(999).to_sorted_array
  end
end
