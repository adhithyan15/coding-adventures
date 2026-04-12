# frozen_string_literal: true

# =============================================================================
# coding_adventures/b_tree/node.rb -- B-Tree Node
# =============================================================================
#
# A B-tree node is the fundamental building block of the B-tree. Unlike a
# binary tree node (which has exactly one key and two children), a B-tree
# node can hold MANY keys and children — this is what makes B-trees so
# efficient for disk-based storage.
#
# ANATOMY OF A B-TREE NODE
# ─────────────────────────
#
# Consider a node with keys [10, 20, 30]:
#
#   children[0] | 10 | children[1] | 20 | children[2] | 30 | children[3]
#
# Each key acts as a "divider" between two subtrees:
#   - children[0]: all keys < 10
#   - children[1]: all keys 10 < k < 20
#   - children[2]: all keys 20 < k < 30
#   - children[3]: all keys > 30
#
# LEAF VS INTERNAL
# ─────────────────
# - Leaf nodes: children is empty. They hold actual data.
# - Internal nodes: children.length == keys.length + 1
#
# BOUNDS (for minimum degree t)
# ──────────────────────────────
# Every non-root node must have:
#   - At least t-1 keys  (and t children if internal)
#   - At most 2t-1 keys  (and 2t children if internal)
#
# The root may have 1..2t-1 keys (no minimum, but at least 1 when non-empty).
#
# WHY "MINIMUM DEGREE"?
# ──────────────────────
# The parameter `t` (minimum degree) controls how "fat" each node can be.
# Larger t → fewer disk reads per lookup → better for slow storage.
# Typical values: t=2 (2-3-4 tree), t=128 (filesystem B-trees).
# =============================================================================

module CodingAdventures
  class BTree
    # -------------------------------------------------------------------------
    # BTreeNode: an internal node of the B-tree
    # -------------------------------------------------------------------------
    #
    # Each node stores parallel arrays:
    #   keys[i]     -- the i-th key
    #   values[i]   -- the value associated with keys[i]
    #   children[i] -- a BTreeNode or nil (for leaves)
    #
    # Invariant: children.empty? ⟺ leaf node
    # Invariant: children.length == keys.length + 1 (for non-leaf nodes)
    class BTreeNode
      # Public accessors so the BTree can manipulate them directly.
      # In a production implementation you might make these private and
      # expose only a carefully chosen interface. Here we keep them public
      # to keep the tree algorithms readable.
      attr_accessor :keys, :values, :children

      # Build a new empty node.
      #
      # @param leaf [Boolean] true if this node has no children (a leaf)
      def initialize(leaf: true)
        @keys     = []   # Comparable keys in ascending order
        @values   = []   # Parallel array — values[i] belongs to keys[i]
        @children = []   # Child pointers; empty for leaf nodes
        @leaf     = leaf
      end

      # Is this node a leaf (has no children)?
      def leaf?
        @leaf
      end

      # Mark this node as an internal node (has children).
      # Called when we split the root and need to promote a new root.
      def leaf=(val)
        @leaf = val
      end

      # How many keys does this node hold?
      def size
        @keys.length
      end

      # -----------------------------------------------------------------------
      # search(key) -- O(log n) within node using binary search
      # -----------------------------------------------------------------------
      #
      # Returns [found, index] where:
      #   found == true  → key is at keys[index]
      #   found == false → key would be inserted at index (descent direction)
      #
      # Binary search is O(log(keys.length)) which matters when t is large
      # (e.g. t=128 → 255 keys per node → 8 comparisons instead of 128).
      def search(key)
        lo = 0
        hi = @keys.length - 1
        while lo <= hi
          mid = (lo + hi) / 2
          cmp = key <=> @keys[mid]
          if cmp == 0
            return [true, mid]
          elsif cmp < 0
            hi = mid - 1
          else
            lo = mid + 1
          end
        end
        [false, lo]
      end

      # -----------------------------------------------------------------------
      # full?(t) -- is this node holding the maximum 2t-1 keys?
      # -----------------------------------------------------------------------
      def full?(t)
        @keys.length == 2 * t - 1
      end

      # -----------------------------------------------------------------------
      # Debug representation (not for production, but invaluable when
      # developing and verifying tree structure)
      # -----------------------------------------------------------------------
      def inspect
        if leaf?
          "Leaf(#{@keys.zip(@values).map { |k, v| "#{k}:#{v}" }.join(", ")})"
        else
          "Internal(#{@keys.zip(@values).map { |k, v| "#{k}:#{v}" }.join(", ")})"
        end
      end
    end
  end
end
