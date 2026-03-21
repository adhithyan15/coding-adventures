# frozen_string_literal: true

# --------------------------------------------------------------------------
# test_tree.rb -- Comprehensive Tests for the Tree Library
# --------------------------------------------------------------------------
#
# We organize tests by category:
#
# 1. Construction -- creating trees, verifying initial state
# 2. add_child -- building trees, error cases
# 3. remove_subtree -- pruning branches, error cases
# 4. Queries -- parent, children, siblings, leaf?, root?, depth, height, etc.
# 5. Traversals -- preorder, postorder, level_order
# 6. path_to -- root-to-node paths
# 7. lca -- lowest common ancestor
# 8. subtree -- extracting subtrees
# 9. to_ascii -- ASCII visualization
# 10. Edge cases -- single-node trees, deep chains, wide trees
# 11. graph property -- accessing the underlying DirectedGraph
# --------------------------------------------------------------------------

require_relative "test_helper"

# =========================================================================
# Helper: Build a sample tree for many tests
# =========================================================================
#
# This tree is used across many test categories:
#
#         A
#        / \
#       B   C
#      / \   \
#     D   E   F
#    /
#   G

def make_sample_tree
  t = CodingAdventures::Tree::Tree.new("A")
  t.add_child("A", "B")
  t.add_child("A", "C")
  t.add_child("B", "D")
  t.add_child("B", "E")
  t.add_child("C", "F")
  t.add_child("D", "G")
  t
end

# =========================================================================
# 1. Construction
# =========================================================================

class TestConstruction < Minitest::Test
  def test_create_tree_with_root
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal "root", t.root
  end

  def test_new_tree_has_size_one
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal 1, t.size
  end

  def test_new_tree_root_is_leaf
    t = CodingAdventures::Tree::Tree.new("root")
    assert t.leaf?("root")
  end

  def test_new_tree_root_is_root
    t = CodingAdventures::Tree::Tree.new("root")
    assert t.root?("root")
  end

  def test_new_tree_root_has_no_parent
    t = CodingAdventures::Tree::Tree.new("root")
    assert_nil t.parent("root")
  end

  def test_new_tree_root_has_no_children
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal [], t.children("root")
  end

  def test_new_tree_root_has_depth_zero
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal 0, t.depth("root")
  end

  def test_new_tree_height_zero
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal 0, t.height
  end

  def test_new_tree_has_root_in_nodes
    t = CodingAdventures::Tree::Tree.new("root")
    assert_includes t.nodes, "root"
  end

  def test_to_s
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal 'Tree(root="root", size=1)', t.to_s
  end
end

# =========================================================================
# 2. add_child
# =========================================================================

class TestAddChild < Minitest::Test
  def test_add_one_child
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "child")
    assert_equal 2, t.size
  end

  def test_child_has_correct_parent
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "child")
    assert_equal "root", t.parent("child")
  end

  def test_parent_has_child_in_children_list
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "child")
    assert_includes t.children("root"), "child"
  end

  def test_add_multiple_children_to_same_parent
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "A")
    t.add_child("root", "B")
    t.add_child("root", "C")
    assert_equal %w[A B C], t.children("root")
  end

  def test_add_child_to_non_root
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "mid")
    t.add_child("mid", "leaf")
    assert_equal "mid", t.parent("leaf")
  end

  def test_build_deep_tree
    t = CodingAdventures::Tree::Tree.new("level0")
    (1...10).each do |i|
      t.add_child("level#{i - 1}", "level#{i}")
    end
    assert_equal 10, t.size
    assert_equal 9, t.depth("level9")
  end

  def test_add_child_nonexistent_parent_raises
    t = CodingAdventures::Tree::Tree.new("root")
    err = assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.add_child("nonexistent", "child")
    end
    assert_equal "nonexistent", err.node
  end

  def test_add_duplicate_child_raises
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "child")
    err = assert_raises(CodingAdventures::Tree::DuplicateNodeError) do
      t.add_child("root", "child")
    end
    assert_equal "child", err.node
  end

  def test_add_root_as_child_raises
    t = CodingAdventures::Tree::Tree.new("root")
    assert_raises(CodingAdventures::Tree::DuplicateNodeError) do
      t.add_child("root", "root")
    end
  end

  def test_add_child_makes_parent_not_leaf
    t = CodingAdventures::Tree::Tree.new("root")
    assert t.leaf?("root")
    t.add_child("root", "child")
    refute t.leaf?("root")
  end

  def test_new_child_is_leaf
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "child")
    assert t.leaf?("child")
  end

  def test_errors_inherit_from_tree_error
    t = CodingAdventures::Tree::Tree.new("root")
    assert_raises(CodingAdventures::Tree::TreeError) do
      t.add_child("nonexistent", "child")
    end
    t.add_child("root", "child")
    assert_raises(CodingAdventures::Tree::TreeError) do
      t.add_child("root", "child")
    end
  end
end

# =========================================================================
# 3. remove_subtree
# =========================================================================

class TestRemoveSubtree < Minitest::Test
  def test_remove_leaf
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "leaf")
    t.remove_subtree("leaf")
    assert_equal 1, t.size
    refute t.has_node?("leaf")
  end

  def test_remove_subtree_removes_descendants
    t = make_sample_tree
    t.remove_subtree("B")
    assert_equal 3, t.size
    refute t.has_node?("B")
    refute t.has_node?("D")
    refute t.has_node?("E")
    refute t.has_node?("G")
  end

  def test_remove_subtree_preserves_siblings
    t = make_sample_tree
    t.remove_subtree("B")
    assert t.has_node?("C")
    assert t.has_node?("F")
    assert_equal ["C"], t.children("A")
  end

  def test_remove_deep_subtree
    t = make_sample_tree
    t.remove_subtree("D")
    assert_equal 5, t.size
    refute t.has_node?("D")
    refute t.has_node?("G")
    assert_equal ["E"], t.children("B")
  end

  def test_remove_root_raises
    t = CodingAdventures::Tree::Tree.new("root")
    assert_raises(CodingAdventures::Tree::RootRemovalError) do
      t.remove_subtree("root")
    end
  end

  def test_remove_nonexistent_raises
    t = CodingAdventures::Tree::Tree.new("root")
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.remove_subtree("nonexistent")
    end
  end

  def test_remove_then_readd
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "child")
    t.remove_subtree("child")
    t.add_child("root", "child")
    assert t.has_node?("child")
  end

  def test_remove_single_child_of_parent
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "only_child")
    t.remove_subtree("only_child")
    assert t.leaf?("root")
  end

  def test_root_removal_error_inherits_from_tree_error
    t = CodingAdventures::Tree::Tree.new("root")
    assert_raises(CodingAdventures::Tree::TreeError) do
      t.remove_subtree("root")
    end
  end
end

# =========================================================================
# 4. Queries
# =========================================================================

class TestQueries < Minitest::Test
  # --- parent ---

  def test_parent_of_child
    t = make_sample_tree
    assert_equal "A", t.parent("B")
  end

  def test_parent_of_grandchild
    t = make_sample_tree
    assert_equal "D", t.parent("G")
  end

  def test_parent_of_root_is_nil
    t = make_sample_tree
    assert_nil t.parent("A")
  end

  def test_parent_nonexistent_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.parent("Z")
    end
  end

  # --- children ---

  def test_children_of_root
    t = make_sample_tree
    assert_equal %w[B C], t.children("A")
  end

  def test_children_of_internal_node
    t = make_sample_tree
    assert_equal %w[D E], t.children("B")
  end

  def test_children_of_leaf
    t = make_sample_tree
    assert_equal [], t.children("G")
  end

  def test_children_nonexistent_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.children("Z")
    end
  end

  # --- siblings ---

  def test_siblings_of_node_with_sibling
    t = make_sample_tree
    assert_equal ["C"], t.siblings("B")
  end

  def test_siblings_are_mutual
    t = make_sample_tree
    assert_equal ["B"], t.siblings("C")
  end

  def test_siblings_of_only_child
    t = make_sample_tree
    assert_equal [], t.siblings("F")
  end

  def test_siblings_of_root
    t = make_sample_tree
    assert_equal [], t.siblings("A")
  end

  def test_siblings_nonexistent_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.siblings("Z")
    end
  end

  def test_siblings_multiple
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "A")
    t.add_child("root", "B")
    t.add_child("root", "C")
    t.add_child("root", "D")
    assert_equal %w[A C D], t.siblings("B")
  end

  # --- leaf? ---

  def test_leaf_true
    t = make_sample_tree
    assert t.leaf?("G")
    assert t.leaf?("E")
    assert t.leaf?("F")
  end

  def test_leaf_false
    t = make_sample_tree
    refute t.leaf?("A")
    refute t.leaf?("B")
  end

  def test_leaf_nonexistent_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.leaf?("Z")
    end
  end

  # --- root? ---

  def test_root_true
    t = make_sample_tree
    assert t.root?("A")
  end

  def test_root_false
    t = make_sample_tree
    refute t.root?("B")
  end

  def test_root_nonexistent_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.root?("Z")
    end
  end

  # --- depth ---

  def test_depth_root
    t = make_sample_tree
    assert_equal 0, t.depth("A")
  end

  def test_depth_level_one
    t = make_sample_tree
    assert_equal 1, t.depth("B")
    assert_equal 1, t.depth("C")
  end

  def test_depth_level_two
    t = make_sample_tree
    assert_equal 2, t.depth("D")
    assert_equal 2, t.depth("E")
    assert_equal 2, t.depth("F")
  end

  def test_depth_level_three
    t = make_sample_tree
    assert_equal 3, t.depth("G")
  end

  def test_depth_nonexistent_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.depth("Z")
    end
  end

  # --- height ---

  def test_height_sample_tree
    t = make_sample_tree
    assert_equal 3, t.height
  end

  def test_height_single_node
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal 0, t.height
  end

  def test_height_flat_tree
    t = CodingAdventures::Tree::Tree.new("root")
    5.times { |i| t.add_child("root", "child#{i}") }
    assert_equal 1, t.height
  end

  def test_height_deep_chain
    t = CodingAdventures::Tree::Tree.new("0")
    (1...20).each { |i| t.add_child((i - 1).to_s, i.to_s) }
    assert_equal 19, t.height
  end

  # --- size ---

  def test_size_sample
    t = make_sample_tree
    assert_equal 7, t.size
  end

  def test_size_after_add
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal 1, t.size
    t.add_child("root", "A")
    assert_equal 2, t.size
  end

  # --- nodes ---

  def test_nodes_returns_all
    t = make_sample_tree
    assert_equal %w[A B C D E F G], t.nodes
  end

  # --- leaves ---

  def test_leaves_sample
    t = make_sample_tree
    assert_equal %w[E F G], t.leaves
  end

  def test_leaves_single_node
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal ["root"], t.leaves
  end

  def test_leaves_flat_tree
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "A")
    t.add_child("root", "B")
    t.add_child("root", "C")
    assert_equal %w[A B C], t.leaves
  end

  # --- has_node? ---

  def test_has_node_true
    t = make_sample_tree
    assert t.has_node?("A")
  end

  def test_has_node_false
    t = make_sample_tree
    refute t.has_node?("Z")
  end
end

# =========================================================================
# 5. Traversals
# =========================================================================

class TestTraversals < Minitest::Test
  # --- preorder ---

  def test_preorder_sample
    t = make_sample_tree
    assert_equal %w[A B D G E C F], t.preorder
  end

  def test_preorder_single_node
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal ["root"], t.preorder
  end

  def test_preorder_flat_tree
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "C")
    t.add_child("root", "A")
    t.add_child("root", "B")
    assert_equal %w[root A B C], t.preorder
  end

  def test_preorder_deep_chain
    t = CodingAdventures::Tree::Tree.new("A")
    t.add_child("A", "B")
    t.add_child("B", "C")
    assert_equal %w[A B C], t.preorder
  end

  # --- postorder ---

  def test_postorder_sample
    t = make_sample_tree
    assert_equal %w[G D E B F C A], t.postorder
  end

  def test_postorder_single_node
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal ["root"], t.postorder
  end

  def test_postorder_flat_tree
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "C")
    t.add_child("root", "A")
    t.add_child("root", "B")
    assert_equal %w[A B C root], t.postorder
  end

  def test_postorder_deep_chain
    t = CodingAdventures::Tree::Tree.new("A")
    t.add_child("A", "B")
    t.add_child("B", "C")
    assert_equal %w[C B A], t.postorder
  end

  # --- level_order ---

  def test_level_order_sample
    t = make_sample_tree
    assert_equal %w[A B C D E F G], t.level_order
  end

  def test_level_order_single_node
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal ["root"], t.level_order
  end

  def test_level_order_flat_tree
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "C")
    t.add_child("root", "A")
    t.add_child("root", "B")
    assert_equal %w[root A B C], t.level_order
  end

  def test_level_order_deep_chain
    t = CodingAdventures::Tree::Tree.new("A")
    t.add_child("A", "B")
    t.add_child("B", "C")
    assert_equal %w[A B C], t.level_order
  end

  # --- traversal consistency ---

  def test_all_traversals_same_length
    t = make_sample_tree
    assert_equal 7, t.preorder.length
    assert_equal 7, t.postorder.length
    assert_equal 7, t.level_order.length
  end

  def test_all_traversals_same_elements
    t = make_sample_tree
    assert_equal t.preorder.sort, t.postorder.sort
    assert_equal t.preorder.sort, t.level_order.sort
  end

  def test_preorder_root_is_first
    t = make_sample_tree
    assert_equal "A", t.preorder[0]
  end

  def test_postorder_root_is_last
    t = make_sample_tree
    assert_equal "A", t.postorder[-1]
  end

  def test_level_order_root_is_first
    t = make_sample_tree
    assert_equal "A", t.level_order[0]
  end
end

# =========================================================================
# 6. path_to
# =========================================================================

class TestPathTo < Minitest::Test
  def test_path_to_root
    t = make_sample_tree
    assert_equal ["A"], t.path_to("A")
  end

  def test_path_to_child
    t = make_sample_tree
    assert_equal %w[A B], t.path_to("B")
  end

  def test_path_to_grandchild
    t = make_sample_tree
    assert_equal %w[A B D], t.path_to("D")
  end

  def test_path_to_deep_node
    t = make_sample_tree
    assert_equal %w[A B D G], t.path_to("G")
  end

  def test_path_to_right_branch
    t = make_sample_tree
    assert_equal %w[A C F], t.path_to("F")
  end

  def test_path_to_nonexistent_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.path_to("Z")
    end
  end

  def test_path_length_equals_depth_plus_one
    t = make_sample_tree
    t.nodes.each do |node|
      assert_equal t.depth(node) + 1, t.path_to(node).length
    end
  end
end

# =========================================================================
# 7. lca (Lowest Common Ancestor)
# =========================================================================

class TestLCA < Minitest::Test
  def test_lca_same_node
    t = make_sample_tree
    assert_equal "D", t.lca("D", "D")
  end

  def test_lca_siblings
    t = make_sample_tree
    assert_equal "B", t.lca("D", "E")
  end

  def test_lca_parent_child
    t = make_sample_tree
    assert_equal "B", t.lca("B", "D")
  end

  def test_lca_child_parent
    t = make_sample_tree
    assert_equal "B", t.lca("D", "B")
  end

  def test_lca_cousins
    t = make_sample_tree
    assert_equal "A", t.lca("D", "F")
  end

  def test_lca_root_and_leaf
    t = make_sample_tree
    assert_equal "A", t.lca("A", "G")
  end

  def test_lca_deep_nodes
    t = make_sample_tree
    assert_equal "B", t.lca("G", "E")
  end

  def test_lca_both_leaves_different_subtrees
    t = make_sample_tree
    assert_equal "A", t.lca("G", "F")
  end

  def test_lca_nonexistent_a_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.lca("Z", "A")
    end
  end

  def test_lca_nonexistent_b_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.lca("A", "Z")
    end
  end

  def test_lca_root_with_root
    t = make_sample_tree
    assert_equal "A", t.lca("A", "A")
  end
end

# =========================================================================
# 8. subtree
# =========================================================================

class TestSubtree < Minitest::Test
  def test_subtree_leaf
    t = make_sample_tree
    sub = t.subtree("G")
    assert_equal "G", sub.root
    assert_equal 1, sub.size
  end

  def test_subtree_internal_node
    t = make_sample_tree
    sub = t.subtree("B")
    assert_equal "B", sub.root
    assert_equal 4, sub.size
    assert sub.has_node?("D")
    assert sub.has_node?("E")
    assert sub.has_node?("G")
  end

  def test_subtree_preserves_structure
    t = make_sample_tree
    sub = t.subtree("B")
    assert_equal %w[D E], sub.children("B")
    assert_equal ["G"], sub.children("D")
    assert sub.leaf?("G")
    assert sub.leaf?("E")
  end

  def test_subtree_root
    t = make_sample_tree
    sub = t.subtree("A")
    assert_equal t.size, sub.size
    assert_equal t.nodes, sub.nodes
  end

  def test_subtree_does_not_modify_original
    t = make_sample_tree
    original_size = t.size
    _sub = t.subtree("B")
    assert_equal original_size, t.size
  end

  def test_subtree_nonexistent_raises
    t = make_sample_tree
    assert_raises(CodingAdventures::Tree::NodeNotFoundError) do
      t.subtree("Z")
    end
  end

  def test_subtree_is_independent
    t = make_sample_tree
    sub = t.subtree("B")
    sub.add_child("E", "new_node")
    refute t.has_node?("new_node")
  end

  def test_subtree_right_branch
    t = make_sample_tree
    sub = t.subtree("C")
    assert_equal "C", sub.root
    assert_equal 2, sub.size
    assert_equal ["F"], sub.children("C")
  end
end

# =========================================================================
# 9. to_ascii
# =========================================================================

class TestToAscii < Minitest::Test
  def test_single_node
    t = CodingAdventures::Tree::Tree.new("root")
    assert_equal "root", t.to_ascii
  end

  def test_root_with_one_child
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "child")
    assert_equal "root\n\u2514\u2500\u2500 child", t.to_ascii
  end

  def test_root_with_two_children
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "A")
    t.add_child("root", "B")
    expected = "root\n\u251C\u2500\u2500 A\n\u2514\u2500\u2500 B"
    assert_equal expected, t.to_ascii
  end

  def test_sample_tree_ascii
    t = make_sample_tree
    expected = [
      "A",
      "\u251C\u2500\u2500 B",
      "\u2502   \u251C\u2500\u2500 D",
      "\u2502   \u2502   \u2514\u2500\u2500 G",
      "\u2502   \u2514\u2500\u2500 E",
      "\u2514\u2500\u2500 C",
      "    \u2514\u2500\u2500 F"
    ].join("\n")
    assert_equal expected, t.to_ascii
  end

  def test_deep_chain_ascii
    t = CodingAdventures::Tree::Tree.new("A")
    t.add_child("A", "B")
    t.add_child("B", "C")
    expected = "A\n\u2514\u2500\u2500 B\n    \u2514\u2500\u2500 C"
    assert_equal expected, t.to_ascii
  end

  def test_wide_tree_ascii
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "A")
    t.add_child("root", "B")
    t.add_child("root", "C")
    t.add_child("root", "D")
    expected = "root\n\u251C\u2500\u2500 A\n\u251C\u2500\u2500 B\n\u251C\u2500\u2500 C\n\u2514\u2500\u2500 D"
    assert_equal expected, t.to_ascii
  end
end

# =========================================================================
# 10. Edge Cases
# =========================================================================

class TestEdgeCases < Minitest::Test
  def test_single_node_tree_traversals
    t = CodingAdventures::Tree::Tree.new("solo")
    assert_equal ["solo"], t.preorder
    assert_equal ["solo"], t.postorder
    assert_equal ["solo"], t.level_order
  end

  def test_single_node_tree_leaves
    t = CodingAdventures::Tree::Tree.new("solo")
    assert_equal ["solo"], t.leaves
  end

  def test_deep_chain_height
    t = CodingAdventures::Tree::Tree.new("n0")
    (1...100).each { |i| t.add_child("n#{i - 1}", "n#{i}") }
    assert_equal 99, t.height
    assert_equal 100, t.size
  end

  def test_wide_tree_height
    t = CodingAdventures::Tree::Tree.new("root")
    100.times { |i| t.add_child("root", "child#{i}") }
    assert_equal 1, t.height
    assert_equal 101, t.size
  end

  def test_balanced_binary_tree
    t = CodingAdventures::Tree::Tree.new("1")
    t.add_child("1", "2")
    t.add_child("1", "3")
    t.add_child("2", "4")
    t.add_child("2", "5")
    t.add_child("3", "6")
    t.add_child("3", "7")
    assert_equal 7, t.size
    assert_equal 2, t.height
    assert_equal %w[4 5 6 7], t.leaves
  end

  def test_node_names_with_spaces
    t = CodingAdventures::Tree::Tree.new("my root")
    t.add_child("my root", "my child")
    assert_equal "my root", t.parent("my child")
  end

  def test_node_names_with_special_chars
    t = CodingAdventures::Tree::Tree.new("root:main")
    t.add_child("root:main", "child.1")
    assert t.has_node?("child.1")
  end

  def test_path_to_single_node
    t = CodingAdventures::Tree::Tree.new("solo")
    assert_equal ["solo"], t.path_to("solo")
  end

  def test_lca_in_single_node_tree
    t = CodingAdventures::Tree::Tree.new("solo")
    assert_equal "solo", t.lca("solo", "solo")
  end

  def test_subtree_of_single_node
    t = CodingAdventures::Tree::Tree.new("solo")
    sub = t.subtree("solo")
    assert_equal "solo", sub.root
    assert_equal 1, sub.size
  end

  def test_remove_and_rebuild
    t = CodingAdventures::Tree::Tree.new("root")
    t.add_child("root", "A")
    t.add_child("A", "B")
    t.remove_subtree("A")
    t.add_child("root", "A")
    t.add_child("A", "C")
    assert_equal ["C"], t.children("A")
    refute t.has_node?("B")
  end
end

# =========================================================================
# 11. graph property
# =========================================================================

class TestGraphProperty < Minitest::Test
  def test_graph_is_directed_graph
    t = make_sample_tree
    assert_instance_of CodingAdventures::DirectedGraph::Graph, t.graph
  end

  def test_graph_has_correct_nodes
    t = make_sample_tree
    assert_equal %w[A B C D E F G].to_set, t.graph.nodes.to_set
  end

  def test_graph_has_correct_edges
    t = make_sample_tree
    edges = t.graph.edges.to_set
    assert_includes edges, %w[A B]
    assert_includes edges, %w[A C]
    assert_includes edges, %w[B D]
    assert_includes edges, %w[B E]
    assert_includes edges, %w[C F]
    assert_includes edges, %w[D G]
  end

  def test_graph_edge_count
    t = make_sample_tree
    assert_equal 6, t.graph.edges.length
  end

  def test_graph_has_no_cycles
    t = make_sample_tree
    refute t.graph.has_cycle?
  end

  def test_graph_topological_sort_starts_with_root
    t = make_sample_tree
    topo = t.graph.topological_sort
    assert_equal "A", topo[0]
  end
end
