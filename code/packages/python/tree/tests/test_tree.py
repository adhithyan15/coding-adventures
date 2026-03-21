"""
test_tree.py -- Comprehensive Tests for the Tree Library
=========================================================

We organize tests by category:

1. Construction -- creating trees, verifying initial state
2. add_child -- building trees, error cases
3. remove_subtree -- pruning branches, error cases
4. Queries -- parent, children, siblings, is_leaf, is_root, depth, height, etc.
5. Traversals -- preorder, postorder, level_order
6. path_to -- root-to-node paths
7. lca -- lowest common ancestor
8. subtree -- extracting subtrees
9. to_ascii -- ASCII visualization
10. Edge cases -- single-node trees, deep chains, wide trees
11. graph property -- accessing the underlying DirectedGraph
"""

import pytest

from tree import (
    DuplicateNodeError,
    NodeNotFoundError,
    RootRemovalError,
    Tree,
    TreeError,
)


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


def make_sample_tree() -> Tree:
    """Build the sample tree described above."""
    t = Tree("A")
    t.add_child("A", "B")
    t.add_child("A", "C")
    t.add_child("B", "D")
    t.add_child("B", "E")
    t.add_child("C", "F")
    t.add_child("D", "G")
    return t


# =========================================================================
# 1. Construction
# =========================================================================


class TestConstruction:
    """Tests for tree creation and initial properties."""

    def test_create_tree_with_root(self):
        """A new tree has the specified root."""
        t = Tree("root")
        assert t.root == "root"

    def test_new_tree_has_size_one(self):
        """A new tree contains exactly one node (the root)."""
        t = Tree("root")
        assert t.size() == 1

    def test_new_tree_root_is_leaf(self):
        """In a single-node tree, the root is also a leaf."""
        t = Tree("root")
        assert t.is_leaf("root") is True

    def test_new_tree_root_is_root(self):
        """The root node is identified as root."""
        t = Tree("root")
        assert t.is_root("root") is True

    def test_new_tree_root_has_no_parent(self):
        """The root has no parent (returns None)."""
        t = Tree("root")
        assert t.parent("root") is None

    def test_new_tree_root_has_no_children(self):
        """The root of a fresh tree has no children."""
        t = Tree("root")
        assert t.children("root") == []

    def test_new_tree_root_has_depth_zero(self):
        """The root is at depth 0."""
        t = Tree("root")
        assert t.depth("root") == 0

    def test_new_tree_height_zero(self):
        """A single-node tree has height 0."""
        t = Tree("root")
        assert t.height() == 0

    def test_new_tree_has_root_in_nodes(self):
        """The root appears in the nodes list."""
        t = Tree("root")
        assert "root" in t.nodes()

    def test_repr(self):
        """The repr shows root and size."""
        t = Tree("root")
        assert repr(t) == "Tree(root='root', size=1)"


# =========================================================================
# 2. add_child
# =========================================================================


class TestAddChild:
    """Tests for adding children to the tree."""

    def test_add_one_child(self):
        """Adding a child increases size by 1."""
        t = Tree("root")
        t.add_child("root", "child")
        assert t.size() == 2

    def test_child_has_correct_parent(self):
        """A newly added child has the specified parent."""
        t = Tree("root")
        t.add_child("root", "child")
        assert t.parent("child") == "root"

    def test_parent_has_child_in_children_list(self):
        """After adding a child, it appears in the parent's children list."""
        t = Tree("root")
        t.add_child("root", "child")
        assert "child" in t.children("root")

    def test_add_multiple_children_to_same_parent(self):
        """A parent can have multiple children."""
        t = Tree("root")
        t.add_child("root", "A")
        t.add_child("root", "B")
        t.add_child("root", "C")
        assert t.children("root") == ["A", "B", "C"]

    def test_add_child_to_non_root(self):
        """Children can be added to any existing node, not just root."""
        t = Tree("root")
        t.add_child("root", "mid")
        t.add_child("mid", "leaf")
        assert t.parent("leaf") == "mid"

    def test_build_deep_tree(self):
        """Can build a tree many levels deep."""
        t = Tree("level0")
        for i in range(1, 10):
            t.add_child(f"level{i - 1}", f"level{i}")
        assert t.size() == 10
        assert t.depth("level9") == 9

    def test_add_child_nonexistent_parent_raises(self):
        """Adding a child under a nonexistent parent raises NodeNotFoundError."""
        t = Tree("root")
        with pytest.raises(NodeNotFoundError) as exc_info:
            t.add_child("nonexistent", "child")
        assert exc_info.value.node == "nonexistent"

    def test_add_duplicate_child_raises(self):
        """Adding a node that already exists raises DuplicateNodeError."""
        t = Tree("root")
        t.add_child("root", "child")
        with pytest.raises(DuplicateNodeError) as exc_info:
            t.add_child("root", "child")
        assert exc_info.value.node == "child"

    def test_add_root_as_child_raises(self):
        """Cannot add the root node as a child (it already exists)."""
        t = Tree("root")
        with pytest.raises(DuplicateNodeError):
            t.add_child("root", "root")

    def test_add_child_makes_parent_not_leaf(self):
        """After adding a child, the parent is no longer a leaf."""
        t = Tree("root")
        assert t.is_leaf("root") is True
        t.add_child("root", "child")
        assert t.is_leaf("root") is False

    def test_new_child_is_leaf(self):
        """A newly added child (with no children of its own) is a leaf."""
        t = Tree("root")
        t.add_child("root", "child")
        assert t.is_leaf("child") is True

    def test_errors_inherit_from_tree_error(self):
        """NodeNotFoundError and DuplicateNodeError inherit from TreeError."""
        t = Tree("root")
        with pytest.raises(TreeError):
            t.add_child("nonexistent", "child")
        t.add_child("root", "child")
        with pytest.raises(TreeError):
            t.add_child("root", "child")


# =========================================================================
# 3. remove_subtree
# =========================================================================


class TestRemoveSubtree:
    """Tests for removing subtrees."""

    def test_remove_leaf(self):
        """Removing a leaf removes just that one node."""
        t = Tree("root")
        t.add_child("root", "leaf")
        t.remove_subtree("leaf")
        assert t.size() == 1
        assert t.has_node("leaf") is False

    def test_remove_subtree_removes_descendants(self):
        """Removing a node removes all its descendants."""
        t = make_sample_tree()
        # Remove B (which has children D, E and grandchild G)
        t.remove_subtree("B")
        assert t.size() == 3  # A, C, F remain
        assert not t.has_node("B")
        assert not t.has_node("D")
        assert not t.has_node("E")
        assert not t.has_node("G")

    def test_remove_subtree_preserves_siblings(self):
        """Removing a subtree doesn't affect sibling subtrees."""
        t = make_sample_tree()
        t.remove_subtree("B")
        assert t.has_node("C")
        assert t.has_node("F")
        assert t.children("A") == ["C"]

    def test_remove_deep_subtree(self):
        """Removing a deeply nested subtree works correctly."""
        t = make_sample_tree()
        t.remove_subtree("D")
        assert t.size() == 5  # A, B, C, E, F remain
        assert not t.has_node("D")
        assert not t.has_node("G")
        assert t.children("B") == ["E"]

    def test_remove_root_raises(self):
        """Cannot remove the root node."""
        t = Tree("root")
        with pytest.raises(RootRemovalError):
            t.remove_subtree("root")

    def test_remove_nonexistent_raises(self):
        """Removing a nonexistent node raises NodeNotFoundError."""
        t = Tree("root")
        with pytest.raises(NodeNotFoundError):
            t.remove_subtree("nonexistent")

    def test_remove_then_readd(self):
        """After removing a node, it can be re-added."""
        t = Tree("root")
        t.add_child("root", "child")
        t.remove_subtree("child")
        t.add_child("root", "child")  # Should work now
        assert t.has_node("child")

    def test_remove_single_child_of_parent(self):
        """After removing the only child, parent becomes a leaf."""
        t = Tree("root")
        t.add_child("root", "only_child")
        t.remove_subtree("only_child")
        assert t.is_leaf("root") is True

    def test_root_removal_error_inherits_from_tree_error(self):
        """RootRemovalError inherits from TreeError."""
        t = Tree("root")
        with pytest.raises(TreeError):
            t.remove_subtree("root")


# =========================================================================
# 4. Queries
# =========================================================================


class TestQueries:
    """Tests for tree query methods."""

    # --- parent ---

    def test_parent_of_child(self):
        t = make_sample_tree()
        assert t.parent("B") == "A"

    def test_parent_of_grandchild(self):
        t = make_sample_tree()
        assert t.parent("G") == "D"

    def test_parent_of_root_is_none(self):
        t = make_sample_tree()
        assert t.parent("A") is None

    def test_parent_nonexistent_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.parent("Z")

    # --- children ---

    def test_children_of_root(self):
        t = make_sample_tree()
        assert t.children("A") == ["B", "C"]

    def test_children_of_internal_node(self):
        t = make_sample_tree()
        assert t.children("B") == ["D", "E"]

    def test_children_of_leaf(self):
        t = make_sample_tree()
        assert t.children("G") == []

    def test_children_nonexistent_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.children("Z")

    # --- siblings ---

    def test_siblings_of_node_with_sibling(self):
        t = make_sample_tree()
        assert t.siblings("B") == ["C"]

    def test_siblings_are_mutual(self):
        t = make_sample_tree()
        assert t.siblings("C") == ["B"]

    def test_siblings_of_only_child(self):
        t = make_sample_tree()
        assert t.siblings("F") == []

    def test_siblings_of_root(self):
        t = make_sample_tree()
        assert t.siblings("A") == []

    def test_siblings_nonexistent_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.siblings("Z")

    def test_siblings_multiple(self):
        """A node with multiple siblings returns all of them."""
        t = Tree("root")
        t.add_child("root", "A")
        t.add_child("root", "B")
        t.add_child("root", "C")
        t.add_child("root", "D")
        assert t.siblings("B") == ["A", "C", "D"]

    # --- is_leaf ---

    def test_is_leaf_true(self):
        t = make_sample_tree()
        assert t.is_leaf("G") is True
        assert t.is_leaf("E") is True
        assert t.is_leaf("F") is True

    def test_is_leaf_false(self):
        t = make_sample_tree()
        assert t.is_leaf("A") is False
        assert t.is_leaf("B") is False

    def test_is_leaf_nonexistent_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.is_leaf("Z")

    # --- is_root ---

    def test_is_root_true(self):
        t = make_sample_tree()
        assert t.is_root("A") is True

    def test_is_root_false(self):
        t = make_sample_tree()
        assert t.is_root("B") is False

    def test_is_root_nonexistent_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.is_root("Z")

    # --- depth ---

    def test_depth_root(self):
        t = make_sample_tree()
        assert t.depth("A") == 0

    def test_depth_level_one(self):
        t = make_sample_tree()
        assert t.depth("B") == 1
        assert t.depth("C") == 1

    def test_depth_level_two(self):
        t = make_sample_tree()
        assert t.depth("D") == 2
        assert t.depth("E") == 2
        assert t.depth("F") == 2

    def test_depth_level_three(self):
        t = make_sample_tree()
        assert t.depth("G") == 3

    def test_depth_nonexistent_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.depth("Z")

    # --- height ---

    def test_height_sample_tree(self):
        t = make_sample_tree()
        assert t.height() == 3

    def test_height_single_node(self):
        t = Tree("root")
        assert t.height() == 0

    def test_height_flat_tree(self):
        """A tree where root has many children but no grandchildren."""
        t = Tree("root")
        for i in range(5):
            t.add_child("root", f"child{i}")
        assert t.height() == 1

    def test_height_deep_chain(self):
        """A linear chain of nodes."""
        t = Tree("0")
        for i in range(1, 20):
            t.add_child(str(i - 1), str(i))
        assert t.height() == 19

    # --- size ---

    def test_size_sample(self):
        t = make_sample_tree()
        assert t.size() == 7

    def test_size_after_add(self):
        t = Tree("root")
        assert t.size() == 1
        t.add_child("root", "A")
        assert t.size() == 2

    # --- nodes ---

    def test_nodes_returns_all(self):
        t = make_sample_tree()
        assert t.nodes() == ["A", "B", "C", "D", "E", "F", "G"]

    # --- leaves ---

    def test_leaves_sample(self):
        t = make_sample_tree()
        assert t.leaves() == ["E", "F", "G"]

    def test_leaves_single_node(self):
        t = Tree("root")
        assert t.leaves() == ["root"]

    def test_leaves_flat_tree(self):
        t = Tree("root")
        t.add_child("root", "A")
        t.add_child("root", "B")
        t.add_child("root", "C")
        assert t.leaves() == ["A", "B", "C"]

    # --- has_node ---

    def test_has_node_true(self):
        t = make_sample_tree()
        assert t.has_node("A") is True

    def test_has_node_false(self):
        t = make_sample_tree()
        assert t.has_node("Z") is False

    # --- __len__ ---

    def test_len(self):
        t = make_sample_tree()
        assert len(t) == 7

    # --- __contains__ ---

    def test_contains_true(self):
        t = make_sample_tree()
        assert "A" in t

    def test_contains_false(self):
        t = make_sample_tree()
        assert "Z" not in t


# =========================================================================
# 5. Traversals
# =========================================================================


class TestTraversals:
    """Tests for tree traversal methods."""

    # --- preorder ---

    def test_preorder_sample(self):
        """Preorder visits parent before children, children in sorted order."""
        t = make_sample_tree()
        assert t.preorder() == ["A", "B", "D", "G", "E", "C", "F"]

    def test_preorder_single_node(self):
        t = Tree("root")
        assert t.preorder() == ["root"]

    def test_preorder_flat_tree(self):
        t = Tree("root")
        t.add_child("root", "C")
        t.add_child("root", "A")
        t.add_child("root", "B")
        # Children sorted: A, B, C
        assert t.preorder() == ["root", "A", "B", "C"]

    def test_preorder_deep_chain(self):
        t = Tree("A")
        t.add_child("A", "B")
        t.add_child("B", "C")
        assert t.preorder() == ["A", "B", "C"]

    # --- postorder ---

    def test_postorder_sample(self):
        """Postorder visits children before parent."""
        t = make_sample_tree()
        assert t.postorder() == ["G", "D", "E", "B", "F", "C", "A"]

    def test_postorder_single_node(self):
        t = Tree("root")
        assert t.postorder() == ["root"]

    def test_postorder_flat_tree(self):
        t = Tree("root")
        t.add_child("root", "C")
        t.add_child("root", "A")
        t.add_child("root", "B")
        assert t.postorder() == ["A", "B", "C", "root"]

    def test_postorder_deep_chain(self):
        t = Tree("A")
        t.add_child("A", "B")
        t.add_child("B", "C")
        assert t.postorder() == ["C", "B", "A"]

    # --- level_order ---

    def test_level_order_sample(self):
        """Level-order visits by depth, sorted within each level."""
        t = make_sample_tree()
        assert t.level_order() == ["A", "B", "C", "D", "E", "F", "G"]

    def test_level_order_single_node(self):
        t = Tree("root")
        assert t.level_order() == ["root"]

    def test_level_order_flat_tree(self):
        t = Tree("root")
        t.add_child("root", "C")
        t.add_child("root", "A")
        t.add_child("root", "B")
        assert t.level_order() == ["root", "A", "B", "C"]

    def test_level_order_deep_chain(self):
        t = Tree("A")
        t.add_child("A", "B")
        t.add_child("B", "C")
        assert t.level_order() == ["A", "B", "C"]

    # --- traversal consistency ---

    def test_all_traversals_same_length(self):
        """All traversals visit the same number of nodes."""
        t = make_sample_tree()
        assert len(t.preorder()) == len(t.postorder()) == len(t.level_order()) == 7

    def test_all_traversals_same_elements(self):
        """All traversals visit the same set of nodes."""
        t = make_sample_tree()
        assert set(t.preorder()) == set(t.postorder()) == set(t.level_order())

    def test_preorder_root_is_first(self):
        """In preorder, the root is always the first element."""
        t = make_sample_tree()
        assert t.preorder()[0] == "A"

    def test_postorder_root_is_last(self):
        """In postorder, the root is always the last element."""
        t = make_sample_tree()
        assert t.postorder()[-1] == "A"

    def test_level_order_root_is_first(self):
        """In level-order, the root is always the first element."""
        t = make_sample_tree()
        assert t.level_order()[0] == "A"


# =========================================================================
# 6. path_to
# =========================================================================


class TestPathTo:
    """Tests for the path_to method."""

    def test_path_to_root(self):
        t = make_sample_tree()
        assert t.path_to("A") == ["A"]

    def test_path_to_child(self):
        t = make_sample_tree()
        assert t.path_to("B") == ["A", "B"]

    def test_path_to_grandchild(self):
        t = make_sample_tree()
        assert t.path_to("D") == ["A", "B", "D"]

    def test_path_to_deep_node(self):
        t = make_sample_tree()
        assert t.path_to("G") == ["A", "B", "D", "G"]

    def test_path_to_right_branch(self):
        t = make_sample_tree()
        assert t.path_to("F") == ["A", "C", "F"]

    def test_path_to_nonexistent_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.path_to("Z")

    def test_path_length_equals_depth_plus_one(self):
        """The path length is always depth + 1."""
        t = make_sample_tree()
        for node in t.nodes():
            assert len(t.path_to(node)) == t.depth(node) + 1


# =========================================================================
# 7. lca (Lowest Common Ancestor)
# =========================================================================


class TestLCA:
    """Tests for the lowest common ancestor method."""

    def test_lca_same_node(self):
        """LCA of a node with itself is the node."""
        t = make_sample_tree()
        assert t.lca("D", "D") == "D"

    def test_lca_siblings(self):
        """LCA of siblings is their parent."""
        t = make_sample_tree()
        assert t.lca("D", "E") == "B"

    def test_lca_parent_child(self):
        """LCA of a parent and its child is the parent."""
        t = make_sample_tree()
        assert t.lca("B", "D") == "B"

    def test_lca_child_parent(self):
        """LCA is symmetric (order doesn't matter)."""
        t = make_sample_tree()
        assert t.lca("D", "B") == "B"

    def test_lca_cousins(self):
        """LCA of nodes in different subtrees is the common ancestor."""
        t = make_sample_tree()
        assert t.lca("D", "F") == "A"

    def test_lca_root_and_leaf(self):
        """LCA of root and any node is root."""
        t = make_sample_tree()
        assert t.lca("A", "G") == "A"

    def test_lca_deep_nodes(self):
        """LCA of deeply nested nodes."""
        t = make_sample_tree()
        assert t.lca("G", "E") == "B"

    def test_lca_both_leaves_different_subtrees(self):
        t = make_sample_tree()
        assert t.lca("G", "F") == "A"

    def test_lca_nonexistent_a_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.lca("Z", "A")

    def test_lca_nonexistent_b_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.lca("A", "Z")

    def test_lca_root_with_root(self):
        t = make_sample_tree()
        assert t.lca("A", "A") == "A"


# =========================================================================
# 8. subtree
# =========================================================================


class TestSubtree:
    """Tests for subtree extraction."""

    def test_subtree_leaf(self):
        """Subtree of a leaf is a single-node tree."""
        t = make_sample_tree()
        sub = t.subtree("G")
        assert sub.root == "G"
        assert sub.size() == 1

    def test_subtree_internal_node(self):
        """Subtree of an internal node includes all descendants."""
        t = make_sample_tree()
        sub = t.subtree("B")
        assert sub.root == "B"
        assert sub.size() == 4  # B, D, E, G
        assert sub.has_node("D")
        assert sub.has_node("E")
        assert sub.has_node("G")

    def test_subtree_preserves_structure(self):
        """The subtree has the same parent-child relationships."""
        t = make_sample_tree()
        sub = t.subtree("B")
        assert sub.children("B") == ["D", "E"]
        assert sub.children("D") == ["G"]
        assert sub.is_leaf("G")
        assert sub.is_leaf("E")

    def test_subtree_root(self):
        """Subtree of the root is the entire tree."""
        t = make_sample_tree()
        sub = t.subtree("A")
        assert sub.size() == t.size()
        assert sub.nodes() == t.nodes()

    def test_subtree_does_not_modify_original(self):
        """Extracting a subtree doesn't change the original tree."""
        t = make_sample_tree()
        original_size = t.size()
        _sub = t.subtree("B")
        assert t.size() == original_size

    def test_subtree_nonexistent_raises(self):
        t = make_sample_tree()
        with pytest.raises(NodeNotFoundError):
            t.subtree("Z")

    def test_subtree_is_independent(self):
        """Modifying the subtree doesn't affect the original."""
        t = make_sample_tree()
        sub = t.subtree("B")
        sub.add_child("E", "new_node")
        assert not t.has_node("new_node")

    def test_subtree_right_branch(self):
        t = make_sample_tree()
        sub = t.subtree("C")
        assert sub.root == "C"
        assert sub.size() == 2
        assert sub.children("C") == ["F"]


# =========================================================================
# 9. to_ascii
# =========================================================================


class TestToAscii:
    """Tests for ASCII visualization."""

    def test_single_node(self):
        t = Tree("root")
        assert t.to_ascii() == "root"

    def test_root_with_one_child(self):
        t = Tree("root")
        t.add_child("root", "child")
        expected = "root\n└── child"
        assert t.to_ascii() == expected

    def test_root_with_two_children(self):
        t = Tree("root")
        t.add_child("root", "A")
        t.add_child("root", "B")
        expected = "root\n├── A\n└── B"
        assert t.to_ascii() == expected

    def test_sample_tree_ascii(self):
        """Verify the full ASCII output for the sample tree."""
        t = make_sample_tree()
        expected = (
            "A\n"
            "├── B\n"
            "│   ├── D\n"
            "│   │   └── G\n"
            "│   └── E\n"
            "└── C\n"
            "    └── F"
        )
        assert t.to_ascii() == expected

    def test_deep_chain_ascii(self):
        t = Tree("A")
        t.add_child("A", "B")
        t.add_child("B", "C")
        expected = "A\n└── B\n    └── C"
        assert t.to_ascii() == expected

    def test_wide_tree_ascii(self):
        t = Tree("root")
        t.add_child("root", "A")
        t.add_child("root", "B")
        t.add_child("root", "C")
        t.add_child("root", "D")
        expected = "root\n├── A\n├── B\n├── C\n└── D"
        assert t.to_ascii() == expected


# =========================================================================
# 10. Edge Cases
# =========================================================================


class TestEdgeCases:
    """Tests for unusual but valid tree configurations."""

    def test_single_node_tree_traversals(self):
        """All traversals of a single-node tree return just the root."""
        t = Tree("solo")
        assert t.preorder() == ["solo"]
        assert t.postorder() == ["solo"]
        assert t.level_order() == ["solo"]

    def test_single_node_tree_leaves(self):
        t = Tree("solo")
        assert t.leaves() == ["solo"]

    def test_deep_chain_height(self):
        """A chain of 100 nodes has height 99."""
        t = Tree("n0")
        for i in range(1, 100):
            t.add_child(f"n{i - 1}", f"n{i}")
        assert t.height() == 99
        assert t.size() == 100

    def test_wide_tree_height(self):
        """A tree with 100 children of root has height 1."""
        t = Tree("root")
        for i in range(100):
            t.add_child("root", f"child{i}")
        assert t.height() == 1
        assert t.size() == 101

    def test_balanced_binary_tree(self):
        """Build a balanced binary tree and verify properties."""
        t = Tree("1")
        t.add_child("1", "2")
        t.add_child("1", "3")
        t.add_child("2", "4")
        t.add_child("2", "5")
        t.add_child("3", "6")
        t.add_child("3", "7")
        assert t.size() == 7
        assert t.height() == 2
        assert t.leaves() == ["4", "5", "6", "7"]

    def test_node_names_with_spaces(self):
        """Node names can contain spaces."""
        t = Tree("my root")
        t.add_child("my root", "my child")
        assert t.parent("my child") == "my root"

    def test_node_names_with_special_chars(self):
        """Node names can contain special characters."""
        t = Tree("root:main")
        t.add_child("root:main", "child.1")
        assert t.has_node("child.1")

    def test_path_to_single_node(self):
        """Path to root in a single-node tree."""
        t = Tree("solo")
        assert t.path_to("solo") == ["solo"]

    def test_lca_in_single_node_tree(self):
        """LCA of root with itself in single-node tree."""
        t = Tree("solo")
        assert t.lca("solo", "solo") == "solo"

    def test_subtree_of_single_node(self):
        """Subtree of a single-node tree is a copy with just the root."""
        t = Tree("solo")
        sub = t.subtree("solo")
        assert sub.root == "solo"
        assert sub.size() == 1

    def test_remove_and_rebuild(self):
        """Can remove a subtree and rebuild it differently."""
        t = Tree("root")
        t.add_child("root", "A")
        t.add_child("A", "B")
        t.remove_subtree("A")
        t.add_child("root", "A")
        t.add_child("A", "C")  # Different child this time
        assert t.children("A") == ["C"]
        assert not t.has_node("B")


# =========================================================================
# 11. graph property
# =========================================================================


class TestGraphProperty:
    """Tests for accessing the underlying DirectedGraph."""

    def test_graph_is_directed_graph(self):
        """The graph property returns a DirectedGraph."""
        from directed_graph import DirectedGraph

        t = make_sample_tree()
        assert isinstance(t.graph, DirectedGraph)

    def test_graph_has_correct_nodes(self):
        t = make_sample_tree()
        assert set(t.graph.nodes()) == {"A", "B", "C", "D", "E", "F", "G"}

    def test_graph_has_correct_edges(self):
        t = make_sample_tree()
        edges = set(t.graph.edges())
        assert ("A", "B") in edges
        assert ("A", "C") in edges
        assert ("B", "D") in edges
        assert ("B", "E") in edges
        assert ("C", "F") in edges
        assert ("D", "G") in edges

    def test_graph_edge_count(self):
        """A tree with N nodes has N-1 edges."""
        t = make_sample_tree()
        assert len(t.graph.edges()) == 6  # 7 nodes - 1

    def test_graph_has_no_cycles(self):
        """The underlying graph should have no cycles."""
        t = make_sample_tree()
        assert t.graph.has_cycle() is False

    def test_graph_topological_sort_starts_with_root(self):
        """The topological sort of the tree starts with the root."""
        t = make_sample_tree()
        topo = t.graph.topological_sort()
        assert topo[0] == "A"
