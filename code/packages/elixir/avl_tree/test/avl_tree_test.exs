defmodule CodingAdventures.AVLTreeTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.AVLTree
  alias CodingAdventures.AVLTree.Node

  test "rotations rebalance the tree" do
    right_heavy = AVLTree.from_values([10, 20, 30])
    left_heavy = AVLTree.from_values([30, 20, 10])

    assert right_heavy.root.value == 20
    assert left_heavy.root.value == 20
    assert AVLTree.to_sorted_array(right_heavy) == [10, 20, 30]
    assert AVLTree.valid_bst?(right_heavy)
    assert AVLTree.valid_avl?(right_heavy)
    assert AVLTree.height(right_heavy) == 1
    assert AVLTree.size(right_heavy) == 3
  end

  test "search and order statistics work" do
    tree = AVLTree.from_values([40, 20, 60, 10, 30, 50, 70])

    assert AVLTree.search(tree, 20).value == 20
    assert AVLTree.contains?(tree, 50)
    assert AVLTree.min_value(tree) == 10
    assert AVLTree.max_value(tree) == 70
    assert AVLTree.predecessor(tree, 40) == 30
    assert AVLTree.successor(tree, 40) == 50
    assert AVLTree.kth_smallest(tree, 4) == 40
    assert AVLTree.rank(tree, 35) == 3

    deleted = AVLTree.delete(tree, 20)
    refute AVLTree.contains?(deleted, 20)
    assert AVLTree.valid_avl?(deleted)
    assert AVLTree.contains?(tree, 20)
  end

  test "empty trees and duplicates work" do
    empty = AVLTree.empty()

    assert AVLTree.search(empty, 1) == nil
    refute AVLTree.contains?(empty, 1)
    assert AVLTree.min_value(empty) == nil
    assert AVLTree.max_value(empty) == nil
    assert AVLTree.predecessor(empty, 1) == nil
    assert AVLTree.successor(empty, 1) == nil
    assert AVLTree.kth_smallest(empty, 0) == nil
    assert AVLTree.rank(empty, 1) == 0
    assert AVLTree.balance_factor(nil) == 0

    tree = AVLTree.from_values([30, 20, 40, 10, 25, 35, 50])
    assert AVLTree.to_sorted_array(AVLTree.insert(tree, 25)) == AVLTree.to_sorted_array(tree)
    assert AVLTree.to_sorted_array(AVLTree.delete(tree, 999)) == AVLTree.to_sorted_array(tree)
  end

  test "double rotations and validation failures work" do
    assert AVLTree.from_values([30, 10, 20]).root.value == 20
    assert AVLTree.from_values([10, 30, 20]).root.value == 20

    bad_order = %AVLTree{root: %Node{value: 5, left: %Node{value: 6}, height: 1, size: 2}}
    bad_height = %AVLTree{root: %Node{value: 5, left: %Node{value: 3}, height: 99, size: 2}}

    refute AVLTree.valid_bst?(bad_order)
    refute AVLTree.valid_avl?(bad_order)
    refute AVLTree.valid_avl?(bad_height)
  end
end
