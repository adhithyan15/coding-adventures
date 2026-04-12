# frozen_string_literal: true

require_relative "test_helper"

class TestBinaryTree < Minitest::Test
  BinaryTree = CodingAdventures::BinaryTree::BinaryTree
  Node = CodingAdventures::BinaryTree::BinaryTreeNode

  def test_level_order_round_trip
    tree = BinaryTree.from_level_order(%w[A B C D E F G])

    assert_equal "A", tree.root.value
    assert_equal %w[A B C D E F G], tree.level_order
    assert_equal %w[A B C D E F G], tree.to_array
  end

  def test_shape_queries_work
    tree = BinaryTree.from_level_order([1, 2, 3, 4, 5, 6, 7])

    assert tree.is_full
    assert tree.is_complete
    assert tree.is_perfect
    assert_equal 2, tree.height
    assert_equal 7, tree.size
    assert_equal 2, tree.left_child(1).value
    assert_equal 3, tree.right_child(1).value
  end

  def test_traversals_work
    tree = BinaryTree.from_level_order(%w[A B C D E F G])

    assert_equal %w[D B E A F C G], tree.inorder
    assert_equal %w[A B D E C F G], tree.preorder
    assert_equal %w[D E B F G C A], tree.postorder
    assert_equal %w[A B C D E F G], tree.level_order
  end

  def test_ascii_render_contains_values
    tree = BinaryTree.from_level_order(%w[root left right])

    ascii = tree.to_ascii
    assert_includes ascii, "root"
    assert_includes ascii, "left"
    assert_includes ascii, "right"
  end
end
