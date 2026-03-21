# frozen_string_literal: true

# --------------------------------------------------------------------------
# errors.rb -- Custom exception hierarchy for the Tree library
# --------------------------------------------------------------------------
#
# Trees impose strict structural constraints on top of directed graphs.
# When those constraints are violated, we need clear, specific errors
# rather than generic StandardError. Each exception class here corresponds
# to one particular kind of violation:
#
#   TreeError           -- the base class for all tree-specific errors.
#                          You can rescue this to handle any tree error
#                          generically, or rescue a more specific subclass
#                          when you want to handle one case differently.
#
#   NodeNotFoundError   -- raised when you reference a node that doesn't
#                          exist in the tree. This is the tree-level
#                          equivalent of the directed graph's
#                          NodeNotFoundError, but we define our own so
#                          that callers can rescue tree errors without
#                          importing the graph library.
#
#   DuplicateNodeError  -- raised when you try to add a node that already
#                          exists. In a tree, every node name must be
#                          unique because each node has exactly one
#                          position in the hierarchy.
#
#   RootRemovalError    -- raised when you try to remove the root node.
#                          The root is the anchor of the entire tree;
#                          removing it would leave a disconnected
#                          collection of subtrees.
# --------------------------------------------------------------------------

module CodingAdventures
  module Tree
    # Base exception for all tree-related errors.
    #
    # This exists so callers can write `rescue TreeError` to catch any tree
    # error without listing every subclass. It also serves as documentation:
    # if you see TreeError in a backtrace, you know the problem is with tree
    # structure, not with the underlying graph.
    class TreeError < StandardError; end

    # Raised when an operation references a node not in the tree.
    #
    # The +node+ attribute carries the missing node's name, so error messages
    # can tell you exactly what was missing.
    #
    # Example:
    #
    #   begin
    #     tree.parent("nonexistent")
    #   rescue NodeNotFoundError => e
    #     puts "Missing: #{e.node}"  # Missing: nonexistent
    #   end
    class NodeNotFoundError < TreeError
      attr_reader :node

      def initialize(node)
        @node = node
        super("Node not found in tree: #{node.inspect}")
      end
    end

    # Raised when trying to add a node that already exists in the tree.
    #
    # In a tree, every node occupies a unique position. If you could add a
    # node twice, it would have two parents -- violating the tree invariant
    # that every non-root node has exactly one parent.
    class DuplicateNodeError < TreeError
      attr_reader :node

      def initialize(node)
        @node = node
        super("Node already exists in tree: #{node.inspect}")
      end
    end

    # Raised when trying to remove the root node.
    #
    # The root is special: it's the only node with no parent, and every
    # other node is reachable from it. Removing the root would destroy
    # the tree's connected structure.
    #
    # If you want to replace the entire tree, create a new Tree instead.
    class RootRemovalError < TreeError
      def initialize
        super("Cannot remove the root node")
      end
    end
  end
end
