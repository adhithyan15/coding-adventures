defmodule CodingAdventures.BinarySearchTreeTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BinarySearchTree
  alias CodingAdventures.BinarySearchTree.Node

  defp populated do
    Enum.reduce([5, 1, 8, 3, 7], BinarySearchTree.empty(), &BinarySearchTree.insert(&2, &1))
  end

  test "inserts, searches, ranks, and deletes" do
    tree = populated()

    assert BinarySearchTree.to_sorted_array(tree) == [1, 3, 5, 7, 8]
    assert BinarySearchTree.size(tree) == 5
    assert BinarySearchTree.contains?(tree, 7)
    assert BinarySearchTree.search(tree, 7).value == 7
    assert BinarySearchTree.min_value(tree) == 1
    assert BinarySearchTree.max_value(tree) == 8
    assert BinarySearchTree.predecessor(tree, 5) == 3
    assert BinarySearchTree.successor(tree, 5) == 7
    assert BinarySearchTree.rank(tree, 4) == 2
    assert BinarySearchTree.kth_smallest(tree, 4) == 7

    deleted = BinarySearchTree.delete(tree, 5)
    refute BinarySearchTree.contains?(deleted, 5)
    assert BinarySearchTree.valid?(deleted)
    assert BinarySearchTree.contains?(tree, 5)
  end

  test "builds balanced trees from sorted arrays" do
    tree = BinarySearchTree.from_sorted_array([1, 2, 3, 4, 5, 6, 7])

    assert BinarySearchTree.to_sorted_array(tree) == [1, 2, 3, 4, 5, 6, 7]
    assert BinarySearchTree.height(tree) == 2
    assert BinarySearchTree.size(tree) == 7
    assert BinarySearchTree.valid?(tree)
  end

  test "handles empty trees and edge queries" do
    tree = BinarySearchTree.empty()

    assert BinarySearchTree.search(tree, 1) == nil
    assert BinarySearchTree.min_value(tree) == nil
    assert BinarySearchTree.max_value(tree) == nil
    assert BinarySearchTree.predecessor(tree, 1) == nil
    assert BinarySearchTree.successor(tree, 1) == nil
    assert BinarySearchTree.kth_smallest(tree, 0) == nil
    assert BinarySearchTree.kth_smallest(tree, 1) == nil
    assert BinarySearchTree.rank(tree, 1) == 0
    assert BinarySearchTree.height(tree) == -1
    assert BinarySearchTree.size(tree) == 0
    assert to_string(tree) == "BinarySearchTree(root=nil, size=0)"
  end

  test "ignores duplicates and deletes one-child nodes" do
    tree = BinarySearchTree.from_sorted_array([2, 4, 6, 8])

    assert tree.root.value == 6
    assert tree.root.size == 4

    assert BinarySearchTree.to_sorted_array(BinarySearchTree.insert(tree, 4)) ==
             BinarySearchTree.to_sorted_array(tree)

    assert BinarySearchTree.to_sorted_array(BinarySearchTree.delete(tree, 2)) == [4, 6, 8]
    assert to_string(tree) == "BinarySearchTree(root=6, size=4)"
  end

  test "validation catches bad ordering and size metadata" do
    bad_order = %BinarySearchTree{root: %Node{value: 5, left: %Node{value: 6}, size: 2}}
    bad_size = %BinarySearchTree{root: %Node{value: 5, left: %Node{value: 3}, size: 99}}

    refute BinarySearchTree.valid?(bad_order)
    refute BinarySearchTree.valid?(bad_size)
  end
end
