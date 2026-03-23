defmodule CodingAdventures.Tree.TreeTest do
  @moduledoc """
  Comprehensive tests for the Tree module.

  We organize tests by category:

  1. Construction -- creating trees, verifying initial state
  2. add_child -- building trees, error cases
  3. remove_subtree -- pruning branches, error cases
  4. Queries -- parent, children, siblings, is_leaf?, is_root?, depth, height, etc.
  5. Traversals -- preorder, postorder, level_order
  6. path_to -- root-to-node paths
  7. lca -- lowest common ancestor
  8. subtree -- extracting subtrees
  9. to_ascii -- ASCII visualization
  10. Edge cases -- single-node trees, deep chains, wide trees
  11. graph property -- accessing the underlying DirectedGraph
  """

  use ExUnit.Case, async: true

  alias CodingAdventures.Tree.Tree

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

  defp make_sample_tree do
    tree = Tree.new("A")
    {:ok, tree} = Tree.add_child(tree, "A", "B")
    {:ok, tree} = Tree.add_child(tree, "A", "C")
    {:ok, tree} = Tree.add_child(tree, "B", "D")
    {:ok, tree} = Tree.add_child(tree, "B", "E")
    {:ok, tree} = Tree.add_child(tree, "C", "F")
    {:ok, tree} = Tree.add_child(tree, "D", "G")
    tree
  end

  # =========================================================================
  # 1. Construction
  # =========================================================================

  describe "construction" do
    test "create tree with root" do
      tree = Tree.new("root")
      assert Tree.root(tree) == "root"
    end

    test "new tree has size one" do
      tree = Tree.new("root")
      assert Tree.size(tree) == 1
    end

    test "new tree root is a leaf" do
      tree = Tree.new("root")
      assert Tree.is_leaf?(tree, "root") == true
    end

    test "new tree root is root" do
      tree = Tree.new("root")
      assert Tree.is_root?(tree, "root") == true
    end

    test "new tree root has no parent" do
      tree = Tree.new("root")
      assert Tree.parent(tree, "root") == {:ok, nil}
    end

    test "new tree root has no children" do
      tree = Tree.new("root")
      assert Tree.children(tree, "root") == {:ok, []}
    end

    test "new tree root has depth zero" do
      tree = Tree.new("root")
      assert Tree.depth(tree, "root") == {:ok, 0}
    end

    test "new tree height is zero" do
      tree = Tree.new("root")
      assert Tree.height(tree) == 0
    end

    test "new tree root appears in nodes" do
      tree = Tree.new("root")
      assert "root" in Tree.nodes(tree)
    end
  end

  # =========================================================================
  # 2. add_child
  # =========================================================================

  describe "add_child" do
    test "add one child increases size" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      assert Tree.size(tree) == 2
    end

    test "child has correct parent" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      assert Tree.parent(tree, "child") == {:ok, "root"}
    end

    test "parent has child in children list" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      {:ok, children} = Tree.children(tree, "root")
      assert "child" in children
    end

    test "add multiple children to same parent" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "root", "B")
      {:ok, tree} = Tree.add_child(tree, "root", "C")
      assert Tree.children(tree, "root") == {:ok, ["A", "B", "C"]}
    end

    test "add child to non-root" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "mid")
      {:ok, tree} = Tree.add_child(tree, "mid", "leaf")
      assert Tree.parent(tree, "leaf") == {:ok, "mid"}
    end

    test "build deep tree" do
      tree = Tree.new("level0")

      tree =
        Enum.reduce(1..9, tree, fn i, acc ->
          {:ok, acc} = Tree.add_child(acc, "level#{i - 1}", "level#{i}")
          acc
        end)

      assert Tree.size(tree) == 10
      assert Tree.depth(tree, "level9") == {:ok, 9}
    end

    test "nonexistent parent returns error" do
      tree = Tree.new("root")
      assert {:error, {:node_not_found, "nonexistent"}} = Tree.add_child(tree, "nonexistent", "child")
    end

    test "duplicate child returns error" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      assert {:error, {:duplicate_node, "child"}} = Tree.add_child(tree, "root", "child")
    end

    test "adding root as child returns error" do
      tree = Tree.new("root")
      assert {:error, {:duplicate_node, "root"}} = Tree.add_child(tree, "root", "root")
    end

    test "add child makes parent not a leaf" do
      tree = Tree.new("root")
      assert Tree.is_leaf?(tree, "root") == true
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      assert Tree.is_leaf?(tree, "root") == false
    end

    test "new child is a leaf" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      assert Tree.is_leaf?(tree, "child") == true
    end
  end

  # =========================================================================
  # 3. remove_subtree
  # =========================================================================

  describe "remove_subtree" do
    test "remove leaf" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "leaf")
      {:ok, tree} = Tree.remove_subtree(tree, "leaf")
      assert Tree.size(tree) == 1
      assert Tree.has_node?(tree, "leaf") == false
    end

    test "remove subtree removes descendants" do
      tree = make_sample_tree()
      {:ok, tree} = Tree.remove_subtree(tree, "B")
      assert Tree.size(tree) == 3
      assert Tree.has_node?(tree, "B") == false
      assert Tree.has_node?(tree, "D") == false
      assert Tree.has_node?(tree, "E") == false
      assert Tree.has_node?(tree, "G") == false
    end

    test "remove subtree preserves siblings" do
      tree = make_sample_tree()
      {:ok, tree} = Tree.remove_subtree(tree, "B")
      assert Tree.has_node?(tree, "C") == true
      assert Tree.has_node?(tree, "F") == true
      assert Tree.children(tree, "A") == {:ok, ["C"]}
    end

    test "remove deep subtree" do
      tree = make_sample_tree()
      {:ok, tree} = Tree.remove_subtree(tree, "D")
      assert Tree.size(tree) == 5
      assert Tree.has_node?(tree, "D") == false
      assert Tree.has_node?(tree, "G") == false
      assert Tree.children(tree, "B") == {:ok, ["E"]}
    end

    test "remove root returns error" do
      tree = Tree.new("root")
      assert {:error, :root_removal} = Tree.remove_subtree(tree, "root")
    end

    test "remove nonexistent returns error" do
      tree = Tree.new("root")
      assert {:error, {:node_not_found, "nonexistent"}} = Tree.remove_subtree(tree, "nonexistent")
    end

    test "remove then readd" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      {:ok, tree} = Tree.remove_subtree(tree, "child")
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      assert Tree.has_node?(tree, "child") == true
    end

    test "remove single child makes parent a leaf" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "only_child")
      {:ok, tree} = Tree.remove_subtree(tree, "only_child")
      assert Tree.is_leaf?(tree, "root") == true
    end
  end

  # =========================================================================
  # 4. Queries
  # =========================================================================

  describe "parent" do
    test "parent of child" do
      tree = make_sample_tree()
      assert Tree.parent(tree, "B") == {:ok, "A"}
    end

    test "parent of grandchild" do
      tree = make_sample_tree()
      assert Tree.parent(tree, "G") == {:ok, "D"}
    end

    test "parent of root is nil" do
      tree = make_sample_tree()
      assert Tree.parent(tree, "A") == {:ok, nil}
    end

    test "parent of nonexistent returns error" do
      tree = make_sample_tree()
      assert {:error, {:node_not_found, "Z"}} = Tree.parent(tree, "Z")
    end
  end

  describe "children" do
    test "children of root" do
      tree = make_sample_tree()
      assert Tree.children(tree, "A") == {:ok, ["B", "C"]}
    end

    test "children of internal node" do
      tree = make_sample_tree()
      assert Tree.children(tree, "B") == {:ok, ["D", "E"]}
    end

    test "children of leaf" do
      tree = make_sample_tree()
      assert Tree.children(tree, "G") == {:ok, []}
    end

    test "children of nonexistent returns error" do
      tree = make_sample_tree()
      assert {:error, {:node_not_found, "Z"}} = Tree.children(tree, "Z")
    end
  end

  describe "siblings" do
    test "siblings of node with sibling" do
      tree = make_sample_tree()
      assert Tree.siblings(tree, "B") == {:ok, ["C"]}
    end

    test "siblings are mutual" do
      tree = make_sample_tree()
      assert Tree.siblings(tree, "C") == {:ok, ["B"]}
    end

    test "siblings of only child" do
      tree = make_sample_tree()
      assert Tree.siblings(tree, "F") == {:ok, []}
    end

    test "siblings of root" do
      tree = make_sample_tree()
      assert Tree.siblings(tree, "A") == {:ok, []}
    end

    test "siblings of nonexistent returns error" do
      tree = make_sample_tree()
      assert {:error, {:node_not_found, "Z"}} = Tree.siblings(tree, "Z")
    end

    test "siblings with multiple" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "root", "B")
      {:ok, tree} = Tree.add_child(tree, "root", "C")
      {:ok, tree} = Tree.add_child(tree, "root", "D")
      assert Tree.siblings(tree, "B") == {:ok, ["A", "C", "D"]}
    end
  end

  describe "is_leaf?" do
    test "leaf nodes" do
      tree = make_sample_tree()
      assert Tree.is_leaf?(tree, "G") == true
      assert Tree.is_leaf?(tree, "E") == true
      assert Tree.is_leaf?(tree, "F") == true
    end

    test "non-leaf nodes" do
      tree = make_sample_tree()
      assert Tree.is_leaf?(tree, "A") == false
      assert Tree.is_leaf?(tree, "B") == false
    end
  end

  describe "is_root?" do
    test "root is root" do
      tree = make_sample_tree()
      assert Tree.is_root?(tree, "A") == true
    end

    test "non-root is not root" do
      tree = make_sample_tree()
      assert Tree.is_root?(tree, "B") == false
    end
  end

  describe "depth" do
    test "depth of root" do
      tree = make_sample_tree()
      assert Tree.depth(tree, "A") == {:ok, 0}
    end

    test "depth level one" do
      tree = make_sample_tree()
      assert Tree.depth(tree, "B") == {:ok, 1}
      assert Tree.depth(tree, "C") == {:ok, 1}
    end

    test "depth level two" do
      tree = make_sample_tree()
      assert Tree.depth(tree, "D") == {:ok, 2}
      assert Tree.depth(tree, "E") == {:ok, 2}
      assert Tree.depth(tree, "F") == {:ok, 2}
    end

    test "depth level three" do
      tree = make_sample_tree()
      assert Tree.depth(tree, "G") == {:ok, 3}
    end

    test "depth of nonexistent returns error" do
      tree = make_sample_tree()
      assert {:error, {:node_not_found, "Z"}} = Tree.depth(tree, "Z")
    end
  end

  describe "height" do
    test "height of sample tree" do
      tree = make_sample_tree()
      assert Tree.height(tree) == 3
    end

    test "height of single node" do
      tree = Tree.new("root")
      assert Tree.height(tree) == 0
    end

    test "height of flat tree" do
      tree = Tree.new("root")

      tree =
        Enum.reduce(0..4, tree, fn i, acc ->
          {:ok, acc} = Tree.add_child(acc, "root", "child#{i}")
          acc
        end)

      assert Tree.height(tree) == 1
    end

    test "height of deep chain" do
      tree = Tree.new("n0")

      tree =
        Enum.reduce(1..19, tree, fn i, acc ->
          {:ok, acc} = Tree.add_child(acc, "n#{i - 1}", "n#{i}")
          acc
        end)

      assert Tree.height(tree) == 19
    end
  end

  describe "size" do
    test "size of sample tree" do
      tree = make_sample_tree()
      assert Tree.size(tree) == 7
    end

    test "size after add" do
      tree = Tree.new("root")
      assert Tree.size(tree) == 1
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      assert Tree.size(tree) == 2
    end
  end

  describe "nodes" do
    test "nodes returns all sorted" do
      tree = make_sample_tree()
      assert Tree.nodes(tree) == ["A", "B", "C", "D", "E", "F", "G"]
    end
  end

  describe "leaves" do
    test "leaves of sample tree" do
      tree = make_sample_tree()
      assert Tree.leaves(tree) == ["E", "F", "G"]
    end

    test "leaves of single node" do
      tree = Tree.new("root")
      assert Tree.leaves(tree) == ["root"]
    end

    test "leaves of flat tree" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "root", "B")
      {:ok, tree} = Tree.add_child(tree, "root", "C")
      assert Tree.leaves(tree) == ["A", "B", "C"]
    end
  end

  describe "has_node?" do
    test "existing node" do
      tree = make_sample_tree()
      assert Tree.has_node?(tree, "A") == true
    end

    test "nonexistent node" do
      tree = make_sample_tree()
      assert Tree.has_node?(tree, "Z") == false
    end
  end

  # =========================================================================
  # 5. Traversals
  # =========================================================================

  describe "preorder" do
    test "sample tree" do
      tree = make_sample_tree()
      assert Tree.preorder(tree) == ["A", "B", "D", "G", "E", "C", "F"]
    end

    test "single node" do
      tree = Tree.new("root")
      assert Tree.preorder(tree) == ["root"]
    end

    test "flat tree (sorted children)" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "C")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "root", "B")
      assert Tree.preorder(tree) == ["root", "A", "B", "C"]
    end

    test "deep chain" do
      tree = Tree.new("A")
      {:ok, tree} = Tree.add_child(tree, "A", "B")
      {:ok, tree} = Tree.add_child(tree, "B", "C")
      assert Tree.preorder(tree) == ["A", "B", "C"]
    end
  end

  describe "postorder" do
    test "sample tree" do
      tree = make_sample_tree()
      assert Tree.postorder(tree) == ["G", "D", "E", "B", "F", "C", "A"]
    end

    test "single node" do
      tree = Tree.new("root")
      assert Tree.postorder(tree) == ["root"]
    end

    test "flat tree" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "C")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "root", "B")
      assert Tree.postorder(tree) == ["A", "B", "C", "root"]
    end

    test "deep chain" do
      tree = Tree.new("A")
      {:ok, tree} = Tree.add_child(tree, "A", "B")
      {:ok, tree} = Tree.add_child(tree, "B", "C")
      assert Tree.postorder(tree) == ["C", "B", "A"]
    end
  end

  describe "level_order" do
    test "sample tree" do
      tree = make_sample_tree()
      assert Tree.level_order(tree) == ["A", "B", "C", "D", "E", "F", "G"]
    end

    test "single node" do
      tree = Tree.new("root")
      assert Tree.level_order(tree) == ["root"]
    end

    test "flat tree" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "C")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "root", "B")
      assert Tree.level_order(tree) == ["root", "A", "B", "C"]
    end

    test "deep chain" do
      tree = Tree.new("A")
      {:ok, tree} = Tree.add_child(tree, "A", "B")
      {:ok, tree} = Tree.add_child(tree, "B", "C")
      assert Tree.level_order(tree) == ["A", "B", "C"]
    end
  end

  describe "traversal consistency" do
    test "all traversals same length" do
      tree = make_sample_tree()

      assert length(Tree.preorder(tree)) == 7
      assert length(Tree.postorder(tree)) == 7
      assert length(Tree.level_order(tree)) == 7
    end

    test "all traversals same elements" do
      tree = make_sample_tree()
      pre = MapSet.new(Tree.preorder(tree))
      post = MapSet.new(Tree.postorder(tree))
      level = MapSet.new(Tree.level_order(tree))
      assert pre == post
      assert post == level
    end

    test "preorder root is first" do
      tree = make_sample_tree()
      assert hd(Tree.preorder(tree)) == "A"
    end

    test "postorder root is last" do
      tree = make_sample_tree()
      assert List.last(Tree.postorder(tree)) == "A"
    end

    test "level_order root is first" do
      tree = make_sample_tree()
      assert hd(Tree.level_order(tree)) == "A"
    end
  end

  # =========================================================================
  # 6. path_to
  # =========================================================================

  describe "path_to" do
    test "path to root" do
      tree = make_sample_tree()
      assert Tree.path_to(tree, "A") == {:ok, ["A"]}
    end

    test "path to child" do
      tree = make_sample_tree()
      assert Tree.path_to(tree, "B") == {:ok, ["A", "B"]}
    end

    test "path to grandchild" do
      tree = make_sample_tree()
      assert Tree.path_to(tree, "D") == {:ok, ["A", "B", "D"]}
    end

    test "path to deep node" do
      tree = make_sample_tree()
      assert Tree.path_to(tree, "G") == {:ok, ["A", "B", "D", "G"]}
    end

    test "path to right branch" do
      tree = make_sample_tree()
      assert Tree.path_to(tree, "F") == {:ok, ["A", "C", "F"]}
    end

    test "path to nonexistent returns error" do
      tree = make_sample_tree()
      assert {:error, {:node_not_found, "Z"}} = Tree.path_to(tree, "Z")
    end

    test "path length equals depth plus one" do
      tree = make_sample_tree()

      for node <- Tree.nodes(tree) do
        {:ok, path} = Tree.path_to(tree, node)
        {:ok, d} = Tree.depth(tree, node)
        assert length(path) == d + 1
      end
    end
  end

  # =========================================================================
  # 7. lca
  # =========================================================================

  describe "lca" do
    test "same node" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "D", "D") == {:ok, "D"}
    end

    test "siblings" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "D", "E") == {:ok, "B"}
    end

    test "parent and child" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "B", "D") == {:ok, "B"}
    end

    test "child and parent (symmetric)" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "D", "B") == {:ok, "B"}
    end

    test "nodes in different subtrees" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "D", "F") == {:ok, "A"}
    end

    test "root and leaf" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "A", "G") == {:ok, "A"}
    end

    test "deep nodes same subtree" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "G", "E") == {:ok, "B"}
    end

    test "both leaves different subtrees" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "G", "F") == {:ok, "A"}
    end

    test "nonexistent a returns error" do
      tree = make_sample_tree()
      assert {:error, {:node_not_found, "Z"}} = Tree.lca(tree, "Z", "A")
    end

    test "nonexistent b returns error" do
      tree = make_sample_tree()
      assert {:error, {:node_not_found, "Z"}} = Tree.lca(tree, "A", "Z")
    end

    test "root with root" do
      tree = make_sample_tree()
      assert Tree.lca(tree, "A", "A") == {:ok, "A"}
    end
  end

  # =========================================================================
  # 8. subtree
  # =========================================================================

  describe "subtree" do
    test "subtree of leaf" do
      tree = make_sample_tree()
      {:ok, sub} = Tree.subtree(tree, "G")
      assert Tree.root(sub) == "G"
      assert Tree.size(sub) == 1
    end

    test "subtree of internal node" do
      tree = make_sample_tree()
      {:ok, sub} = Tree.subtree(tree, "B")
      assert Tree.root(sub) == "B"
      assert Tree.size(sub) == 4
      assert Tree.has_node?(sub, "D")
      assert Tree.has_node?(sub, "E")
      assert Tree.has_node?(sub, "G")
    end

    test "subtree preserves structure" do
      tree = make_sample_tree()
      {:ok, sub} = Tree.subtree(tree, "B")
      assert Tree.children(sub, "B") == {:ok, ["D", "E"]}
      assert Tree.children(sub, "D") == {:ok, ["G"]}
      assert Tree.is_leaf?(sub, "G") == true
      assert Tree.is_leaf?(sub, "E") == true
    end

    test "subtree of root is entire tree" do
      tree = make_sample_tree()
      {:ok, sub} = Tree.subtree(tree, "A")
      assert Tree.size(sub) == Tree.size(tree)
      assert Tree.nodes(sub) == Tree.nodes(tree)
    end

    test "subtree does not modify original" do
      tree = make_sample_tree()
      original_size = Tree.size(tree)
      {:ok, _sub} = Tree.subtree(tree, "B")
      assert Tree.size(tree) == original_size
    end

    test "subtree of nonexistent returns error" do
      tree = make_sample_tree()
      assert {:error, {:node_not_found, "Z"}} = Tree.subtree(tree, "Z")
    end

    test "subtree is independent" do
      tree = make_sample_tree()
      {:ok, sub} = Tree.subtree(tree, "B")
      {:ok, _sub} = Tree.add_child(sub, "E", "new_node")
      assert Tree.has_node?(tree, "new_node") == false
    end

    test "subtree right branch" do
      tree = make_sample_tree()
      {:ok, sub} = Tree.subtree(tree, "C")
      assert Tree.root(sub) == "C"
      assert Tree.size(sub) == 2
      assert Tree.children(sub, "C") == {:ok, ["F"]}
    end
  end

  # =========================================================================
  # 9. to_ascii
  # =========================================================================

  describe "to_ascii" do
    test "single node" do
      tree = Tree.new("root")
      assert Tree.to_ascii(tree) == "root"
    end

    test "root with one child" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "child")
      assert Tree.to_ascii(tree) == "root\n└── child"
    end

    test "root with two children" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "root", "B")
      assert Tree.to_ascii(tree) == "root\n├── A\n└── B"
    end

    test "sample tree" do
      tree = make_sample_tree()

      expected =
        "A\n" <>
          "├── B\n" <>
          "│   ├── D\n" <>
          "│   │   └── G\n" <>
          "│   └── E\n" <>
          "└── C\n" <>
          "    └── F"

      assert Tree.to_ascii(tree) == expected
    end

    test "deep chain" do
      tree = Tree.new("A")
      {:ok, tree} = Tree.add_child(tree, "A", "B")
      {:ok, tree} = Tree.add_child(tree, "B", "C")
      assert Tree.to_ascii(tree) == "A\n└── B\n    └── C"
    end

    test "wide tree" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "root", "B")
      {:ok, tree} = Tree.add_child(tree, "root", "C")
      {:ok, tree} = Tree.add_child(tree, "root", "D")
      assert Tree.to_ascii(tree) == "root\n├── A\n├── B\n├── C\n└── D"
    end
  end

  # =========================================================================
  # 10. Edge Cases
  # =========================================================================

  describe "edge cases" do
    test "single node tree traversals" do
      tree = Tree.new("solo")
      assert Tree.preorder(tree) == ["solo"]
      assert Tree.postorder(tree) == ["solo"]
      assert Tree.level_order(tree) == ["solo"]
    end

    test "single node tree leaves" do
      tree = Tree.new("solo")
      assert Tree.leaves(tree) == ["solo"]
    end

    test "deep chain height" do
      tree = Tree.new("n0")

      tree =
        Enum.reduce(1..99, tree, fn i, acc ->
          {:ok, acc} = Tree.add_child(acc, "n#{i - 1}", "n#{i}")
          acc
        end)

      assert Tree.height(tree) == 99
      assert Tree.size(tree) == 100
    end

    test "wide tree height" do
      tree = Tree.new("root")

      tree =
        Enum.reduce(0..99, tree, fn i, acc ->
          {:ok, acc} = Tree.add_child(acc, "root", "child#{i}")
          acc
        end)

      assert Tree.height(tree) == 1
      assert Tree.size(tree) == 101
    end

    test "balanced binary tree" do
      tree = Tree.new("1")
      {:ok, tree} = Tree.add_child(tree, "1", "2")
      {:ok, tree} = Tree.add_child(tree, "1", "3")
      {:ok, tree} = Tree.add_child(tree, "2", "4")
      {:ok, tree} = Tree.add_child(tree, "2", "5")
      {:ok, tree} = Tree.add_child(tree, "3", "6")
      {:ok, tree} = Tree.add_child(tree, "3", "7")
      assert Tree.size(tree) == 7
      assert Tree.height(tree) == 2
      assert Tree.leaves(tree) == ["4", "5", "6", "7"]
    end

    test "path to single node" do
      tree = Tree.new("solo")
      assert Tree.path_to(tree, "solo") == {:ok, ["solo"]}
    end

    test "lca in single node tree" do
      tree = Tree.new("solo")
      assert Tree.lca(tree, "solo", "solo") == {:ok, "solo"}
    end

    test "subtree of single node" do
      tree = Tree.new("solo")
      {:ok, sub} = Tree.subtree(tree, "solo")
      assert Tree.root(sub) == "solo"
      assert Tree.size(sub) == 1
    end

    test "remove and rebuild" do
      tree = Tree.new("root")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "A", "B")
      {:ok, tree} = Tree.remove_subtree(tree, "A")
      {:ok, tree} = Tree.add_child(tree, "root", "A")
      {:ok, tree} = Tree.add_child(tree, "A", "C")
      assert Tree.children(tree, "A") == {:ok, ["C"]}
      assert Tree.has_node?(tree, "B") == false
    end
  end

  # =========================================================================
  # 11. graph property
  # =========================================================================

  describe "graph property" do
    test "graph returns a Graph struct" do
      tree = make_sample_tree()
      graph = Tree.graph(tree)
      assert %CodingAdventures.DirectedGraph.Graph{} = graph
    end

    test "graph has correct nodes" do
      tree = make_sample_tree()
      graph = Tree.graph(tree)
      nodes = MapSet.new(CodingAdventures.DirectedGraph.Graph.nodes(graph))
      assert nodes == MapSet.new(["A", "B", "C", "D", "E", "F", "G"])
    end

    test "graph has correct edge count" do
      tree = make_sample_tree()
      graph = Tree.graph(tree)
      edges = CodingAdventures.DirectedGraph.Graph.edges(graph)
      assert length(edges) == 6
    end

    test "graph has no cycles" do
      tree = make_sample_tree()
      graph = Tree.graph(tree)
      assert CodingAdventures.DirectedGraph.Graph.has_cycle?(graph) == false
    end

    test "graph topological sort starts with root" do
      tree = make_sample_tree()
      graph = Tree.graph(tree)
      {:ok, topo} = CodingAdventures.DirectedGraph.Graph.topological_sort(graph)
      assert hd(topo) == "A"
    end
  end
end
