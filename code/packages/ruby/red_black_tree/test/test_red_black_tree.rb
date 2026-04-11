# frozen_string_literal: true

require_relative "test_helper"

class TestRedBlackTree < Minitest::Test
  RBTree = CodingAdventures::RedBlackTree::RBTree

  def test_insert_search_and_delete_work
    tree = RBTree.empty
    [5, 1, 8, 3, 7].each { |value| tree = tree.insert(value) }

    assert_equal [1, 3, 5, 7, 8], tree.to_sorted_array
    assert tree.is_valid_rb
    assert_equal 3, tree.black_height
    assert_equal 1, tree.min_value
    assert_equal 8, tree.max_value
    assert_equal 3, tree.predecessor(5)
    assert_equal 7, tree.successor(5)
    assert_equal 3, tree.kth_smallest(2)

    tree = tree.delete(5)
    refute tree.contains(5)
    assert tree.is_valid_rb
  end

  def test_root_is_black
    tree = RBTree.empty
    [10, 20, 30].each { |value| tree = tree.insert(value) }

    assert_equal CodingAdventures::RedBlackTree::Color::Black, tree.root.color
  end

  def test_backend_conversion_and_edge_cases_work
    backend = CodingAdventures::AVLTree::AVLTree.empty
    [2, 1, 3].each { |value| backend = backend.insert(value) }

    tree = RBTree.from_backend(backend)
    assert_equal [1, 2, 3], tree.to_sorted_array
    assert_equal 2, tree.root_node.value
    assert_equal CodingAdventures::RedBlackTree::Color::Black, tree.root_node.color
    assert_equal 2, tree.black_height
    assert tree.is_valid_rb
    assert tree.contains(1)
    assert_equal 1, tree.min_value
    assert_equal 3, tree.max_value
    assert_equal 1, tree.predecessor(2)
    assert_equal 3, tree.successor(2)
    assert_equal 2, tree.kth_smallest(2)
    assert_equal 3, tree.size
    assert_equal 1, tree.height
    assert_nil tree.search(99)

    rebuilt = RBTree.new(CodingAdventures::RedBlackTree::RBNode.new(value: 42, color: CodingAdventures::RedBlackTree::Color::Black, left: nil, right: nil, size: 1))
    assert_equal [42], rebuilt.to_sorted_array
    assert rebuilt.is_valid_rb
  end
end
