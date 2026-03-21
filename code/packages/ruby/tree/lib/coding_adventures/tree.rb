# frozen_string_literal: true

# --------------------------------------------------------------------------
# tree.rb -- A Rooted Tree Backed by a Directed Graph
# --------------------------------------------------------------------------
#
# What Is a Tree?
# ---------------
#
# A **tree** is one of the most fundamental data structures in computer
# science. You encounter trees everywhere:
#
# - File systems: directories contain files and subdirectories
# - HTML/XML: elements contain child elements
# - Programming languages: Abstract Syntax Trees (ASTs) represent code
# - Organization charts: managers have direct reports
#
# Formally, a tree is a connected, acyclic graph where:
#
# 1. There is exactly **one root** node (a node with no parent).
# 2. Every other node has exactly **one parent**.
# 3. There are **no cycles** -- you can never follow edges and return to
#    where you started.
#
# These constraints mean a tree with N nodes always has exactly N-1 edges.
#
#     Tree vs. Graph
#     ~~~~~~~~~~~~~~
#
#     A tree IS a graph (specifically, a directed acyclic graph with the
#     single-parent constraint). We leverage this by building our Tree on
#     top of the DirectedGraph class from the directed-graph package. The
#     DirectedGraph handles all the low-level node/edge storage, while
#     this Tree class enforces the tree invariants and provides
#     tree-specific operations like traversals, depth calculation, and
#     lowest common ancestor.
#
#     Edges point from parent to child:
#
#         Program
#         +-- Assignment    (edge: Program -> Assignment)
#         |   +-- Name      (edge: Assignment -> Name)
#         |   +-- BinaryOp  (edge: Assignment -> BinaryOp)
#         +-- Print         (edge: Program -> Print)
#
#
# Tree Terminology
# ----------------
#
# - **Root**: The topmost node. It has no parent. Every tree has exactly one.
# - **Parent**: The node directly above another node.
# - **Child**: A node directly below another node.
# - **Siblings**: Nodes that share the same parent.
# - **Leaf**: A node with no children.
# - **Depth**: The number of edges from the root to a node. Root = 0.
# - **Height**: The maximum depth of any node in the tree.
# - **Subtree**: A node together with all its descendants.
# - **Path**: The sequence of nodes from the root to a given node.
# - **LCA**: Lowest Common Ancestor -- the deepest common ancestor of two nodes.
#
# Implementation Strategy
# -----------------------
#
# We store the tree as a DirectedGraph with edges pointing parent -> child.
# This means:
#
# - graph.successors(node)   returns the children
# - graph.predecessors(node) returns a list with zero or one element
#   (the parent, or empty for the root)
#
# We maintain the tree invariants by checking them in add_child:
#
# - The parent must already exist in the tree
# - The child must NOT already exist (no duplicate nodes)
# - Since we only add one parent edge per child, cycles are impossible
# --------------------------------------------------------------------------

require "coding_adventures_directed_graph"
require_relative "tree/errors"

module CodingAdventures
  module Tree
    # A rooted tree backed by a DirectedGraph.
    #
    # A tree is a directed graph with three constraints:
    #
    # 1. Exactly one root (no predecessors)
    # 2. Every non-root node has exactly one parent
    # 3. No cycles
    #
    # Edges point parent -> child. The tree is constructed by specifying a
    # root node and then adding children one at a time with +add_child+.
    #
    # Example:
    #
    #   t = CodingAdventures::Tree::Tree.new("Program")
    #   t.add_child("Program", "Assignment")
    #   t.add_child("Program", "Print")
    #   t.add_child("Assignment", "Name")
    #   t.add_child("Assignment", "BinaryOp")
    #
    #   puts t.to_ascii
    #   # Program
    #   # +-- Assignment
    #   # |   +-- BinaryOp
    #   # |   +-- Name
    #   # +-- Print
    class Tree
      # The root node of this tree. Set at construction time, never changes.
      attr_reader :root

      # The underlying DirectedGraph instance. Exposed for advanced use.
      attr_reader :graph

      # ------------------------------------------------------------------
      # Construction
      # ------------------------------------------------------------------
      # A tree always starts with a root. You can't have an empty tree
      # (that would be a forest, or nothing at all).

      def initialize(root)
        @graph = CodingAdventures::DirectedGraph::Graph.new
        @graph.add_node(root)
        @root = root
      end

      # ------------------------------------------------------------------
      # Mutation
      # ------------------------------------------------------------------

      # Add a child node under the given parent.
      #
      # This is the primary way to build up a tree. Each call adds one
      # new node and one edge (parent -> child).
      #
      # Raises NodeNotFoundError if +parent+ is not in the tree.
      # Raises DuplicateNodeError if +child+ is already in the tree.
      #
      # Why not allow adding a node that already exists? Because in a tree,
      # every node has exactly one parent. If we allowed adding "X" under
      # both "A" and "B", node "X" would have two parents -- violating
      # the tree invariant.
      def add_child(parent, child)
        raise NodeNotFoundError, parent unless @graph.has_node?(parent)
        raise DuplicateNodeError, child if @graph.has_node?(child)

        @graph.add_edge(parent, child)
      end

      # Remove a node and all its descendants from the tree.
      #
      # This is a "prune" operation -- it cuts off an entire branch.
      # The node and everything below it is removed. The parent of the
      # removed node is unaffected.
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      # Raises RootRemovalError if +node+ is the root.
      #
      # How it works:
      # We collect all nodes in the subtree via BFS, then remove them
      # in reverse order (children before parents) so the graph stays
      # consistent at each step.
      def remove_subtree(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)
        raise RootRemovalError.new if node == @root

        to_remove = collect_subtree_nodes(node)

        # Remove from bottom up: BFS gives parent-first order, so we
        # reverse to get children-first.
        to_remove.reverse_each { |n| @graph.remove_node(n) }
      end

      # ------------------------------------------------------------------
      # Queries
      # ------------------------------------------------------------------

      # Return the parent of a node, or nil if the node is the root.
      #
      # In a tree, every non-root node has exactly one parent. The root
      # has no parent, so we return nil for it.
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      def parent(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)

        preds = @graph.predecessors(node)
        preds.empty? ? nil : preds[0]
      end

      # Return the children of a node (sorted alphabetically).
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      def children(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)

        @graph.successors(node).sort
      end

      # Return the siblings of a node (other children of the same parent).
      #
      # The root has no siblings. A node whose parent has only one child
      # also has no siblings.
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      def siblings(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)

        parent_node = parent(node)
        return [] if parent_node.nil?

        children(parent_node).reject { |c| c == node }
      end

      # Return true if the node has no children (a "leaf").
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      def leaf?(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)

        @graph.successors(node).empty?
      end

      # Return true if the node is the root of the tree.
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      def root?(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)

        node == @root
      end

      # Return the depth of a node (distance from root).
      #
      # The depth is the number of edges on the path from the root to
      # this node. Root = 0, its children = 1, grandchildren = 2, etc.
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      def depth(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)

        d = 0
        current = node
        while current != @root
          preds = @graph.predecessors(current)
          current = preds[0]
          d += 1
        end
        d
      end

      # Return the height of the tree (maximum depth of any node).
      #
      # A single-node tree has height 0.
      #
      # We use BFS from the root, tracking the depth at each level.
      def height
        max_depth = 0
        queue = [[@root, 0]]

        until queue.empty?
          current, d = queue.shift
          max_depth = d if d > max_depth
          @graph.successors(current).each do |child|
            queue << [child, d + 1]
          end
        end

        max_depth
      end

      # Return the total number of nodes in the tree.
      def size
        @graph.size
      end

      # Return a sorted list of all nodes in the tree.
      def nodes
        @graph.nodes.sort
      end

      # Return all leaf nodes (sorted alphabetically).
      def leaves
        @graph.nodes.select { |n| @graph.successors(n).empty? }.sort
      end

      # Return true if the node exists in the tree.
      def has_node?(node)
        @graph.has_node?(node)
      end

      # ------------------------------------------------------------------
      # Traversals
      # ------------------------------------------------------------------
      #
      # Tree traversals visit every node exactly once, in different orders.
      #
      # 1. **Preorder** (root first): Visit a node, then visit all its
      #    children. Top-down. Good for copying a tree, prefix notation.
      #
      # 2. **Postorder** (root last): Visit all children, then the node.
      #    Bottom-up. Good for computing sizes, deleting trees.
      #
      # 3. **Level-order** (BFS): Visit all nodes at depth 0, then 1,
      #    then 2, etc. Good for finding shortest paths.
      #
      # For a tree:
      #       A
      #      / \
      #     B   C
      #    / \
      #   D   E
      #
      # Preorder:    A, B, D, E, C
      # Postorder:   D, E, B, C, A
      # Level-order: A, B, C, D, E

      # Return nodes in preorder (parent before children).
      #
      # Uses an explicit stack (not recursion) to avoid stack overflow
      # on deep trees. Children are pushed in reverse sorted order so
      # that the smallest pops first.
      def preorder
        result = []
        stack = [@root]

        until stack.empty?
          node = stack.pop
          result << node
          children = @graph.successors(node).sort.reverse
          stack.concat(children)
        end

        result
      end

      # Return nodes in postorder (children before parent).
      #
      # Uses a recursive helper. Children visited in sorted order.
      def postorder
        result = []
        postorder_recursive(@root, result)
        result
      end

      # Return nodes in level-order (breadth-first).
      #
      # Classic BFS using a queue. Within each level, children are
      # visited in sorted order.
      def level_order
        result = []
        queue = [@root]

        until queue.empty?
          node = queue.shift
          result << node
          @graph.successors(node).sort.each { |child| queue << child }
        end

        result
      end

      # ------------------------------------------------------------------
      # Utilities
      # ------------------------------------------------------------------

      # Return the path from the root to the given node.
      #
      # The path is a list starting with the root and ending with the
      # target node.
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      def path_to(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)

        path = []
        current = node

        until current.nil?
          path << current
          current = parent(current)
        end

        path.reverse
      end

      # Return the lowest common ancestor (LCA) of nodes a and b.
      #
      # The LCA is the deepest node that is an ancestor of both a and b.
      #
      # Algorithm:
      # 1. Compute path from root to a
      # 2. Compute path from root to b
      # 3. Walk both paths from root. Last matching node is the LCA.
      #
      # Raises NodeNotFoundError if a or b is not in the tree.
      def lca(a, b)
        raise NodeNotFoundError, a unless @graph.has_node?(a)
        raise NodeNotFoundError, b unless @graph.has_node?(b)

        path_a = path_to(a)
        path_b = path_to(b)

        lca_node = @root
        path_a.zip(path_b).each do |na, nb|
          if na == nb
            lca_node = na
          else
            break
          end
        end

        lca_node
      end

      # Extract the subtree rooted at the given node.
      #
      # Returns a NEW Tree object containing the node and all its
      # descendants. The original tree is not modified.
      #
      # Raises NodeNotFoundError if +node+ is not in the tree.
      def subtree(node)
        raise NodeNotFoundError, node unless @graph.has_node?(node)

        new_tree = Tree.new(node)
        queue = [node]

        until queue.empty?
          current = queue.shift
          @graph.successors(current).sort.each do |child|
            new_tree.add_child(current, child)
            queue << child
          end
        end

        new_tree
      end

      # ------------------------------------------------------------------
      # Visualization
      # ------------------------------------------------------------------

      # Render the tree as an ASCII art diagram.
      #
      # Produces output like:
      #
      #   Program
      #   +-- Assignment
      #   |   +-- BinaryOp
      #   |   +-- Name
      #   +-- Print
      #
      # The box-drawing characters used are:
      # - +-- for a child that has more siblings after it
      # - +-- for the last child of its parent
      # - |   for a vertical continuation line
      # - "    " (spaces) for padding where no continuation is needed
      def to_ascii
        lines = []
        ascii_recursive(@root, "", "", lines)
        lines.join("\n")
      end

      # String representation showing root and size.
      def to_s
        "Tree(root=#{@root.inspect}, size=#{size})"
      end

      alias_method :inspect, :to_s

      private

      # Collect all nodes in the subtree rooted at +node+ using BFS.
      def collect_subtree_nodes(node)
        result = []
        queue = [node]

        until queue.empty?
          current = queue.shift
          result << current
          @graph.successors(current).sort.each { |child| queue << child }
        end

        result
      end

      # Recursive postorder helper.
      def postorder_recursive(node, result)
        @graph.successors(node).sort.each do |child|
          postorder_recursive(child, result)
        end
        result << node
      end

      # Recursive helper for to_ascii.
      #
      # +prefix+ is the prefix for this node's line (includes connector).
      # +child_prefix+ is the prefix for this node's children (includes
      # continuation lines).
      def ascii_recursive(node, prefix, child_prefix, lines)
        lines << "#{prefix}#{node}"
        kids = @graph.successors(node).sort

        kids.each_with_index do |child, i|
          if i < kids.length - 1
            ascii_recursive(child, "#{child_prefix}\u251C\u2500\u2500 ",
              "#{child_prefix}\u2502   ", lines)
          else
            ascii_recursive(child, "#{child_prefix}\u2514\u2500\u2500 ",
              "#{child_prefix}    ", lines)
          end
        end
      end
    end
  end
end
