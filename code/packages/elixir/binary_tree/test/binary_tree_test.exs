defmodule CodingAdventures.BinaryTreeTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.BinaryTree
  alias CodingAdventures.BinaryTree.Node

  test "round trips level-order input" do
    tree = BinaryTree.from_level_order([1, 2, 3, 4, 5, 6, 7])

    assert tree.root.value == 1
    assert BinaryTree.to_array(tree) == [1, 2, 3, 4, 5, 6, 7]
    assert BinaryTree.level_order(tree) == [1, 2, 3, 4, 5, 6, 7]
  end

  test "answers shape queries" do
    tree = BinaryTree.from_level_order([1, 2, nil])

    refute BinaryTree.full?(tree)
    assert BinaryTree.complete?(tree)
    refute BinaryTree.perfect?(tree)
    assert BinaryTree.height(tree) == 1
    assert BinaryTree.size(tree) == 2
    assert BinaryTree.left_child(tree, 1).value == 2
    assert BinaryTree.right_child(tree, 1) == nil
    assert BinaryTree.find(tree, 999) == nil
  end

  test "traverses sparse trees" do
    tree = BinaryTree.from_level_order([1, 2, 3, 4, nil, 5, nil])

    assert BinaryTree.preorder(tree) == [1, 2, 4, 3, 5]
    assert BinaryTree.inorder(tree) == [4, 2, 1, 5, 3]
    assert BinaryTree.postorder(tree) == [4, 2, 5, 3, 1]
    assert BinaryTree.level_order(tree) == [1, 2, 3, 4, 5]
    assert BinaryTree.to_array(tree) == [1, 2, 3, 4, nil, 5, nil]
  end

  test "recognizes perfect full trees" do
    tree = BinaryTree.from_level_order(["A", "B", "C", "D", "E", "F", "G"])

    assert BinaryTree.full?(tree)
    assert BinaryTree.complete?(tree)
    assert BinaryTree.perfect?(tree)
    assert BinaryTree.left_child(tree, "A").value == "B"
    assert BinaryTree.right_child(tree, "A").value == "C"
  end

  test "handles empty trees" do
    tree = BinaryTree.new()

    assert tree.root == nil
    assert BinaryTree.full?(tree)
    assert BinaryTree.complete?(tree)
    assert BinaryTree.perfect?(tree)
    assert BinaryTree.height(tree) == -1
    assert BinaryTree.size(tree) == 0
    assert BinaryTree.to_array(tree) == []
    assert BinaryTree.to_ascii(tree) == ""
    assert BinaryTree.level_order(tree) == []
    assert to_string(tree) == "BinaryTree(root=nil, size=0)"
  end

  test "supports explicit roots and ASCII rendering" do
    root = %Node{value: "root", left: %Node{value: "left"}, right: %Node{value: "right"}}
    tree = BinaryTree.with_root(root)

    ascii = BinaryTree.to_ascii(tree)
    assert ascii =~ "root"
    assert ascii =~ "left"
    assert ascii =~ "right"
    assert to_string(tree) == ~s|BinaryTree(root="root", size=3)|
  end

  test "wraps plain root values" do
    tree = BinaryTree.new(:root)

    assert tree.root.value == :root
    assert BinaryTree.size(tree) == 1
  end
end
