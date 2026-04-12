# frozen_string_literal: true

# =============================================================================
# coding_adventures/b_plus_tree/node.rb -- B+ Tree Node Types
# =============================================================================
#
# The B+ tree uses TWO distinct node types, unlike the plain B-tree:
#
# 1. BPlusLeafNode  — holds actual key-value data, linked in a sorted list
# 2. BPlusInternalNode — holds only separator keys and child pointers
#
# WHY TWO NODE TYPES?
# ────────────────────
# In a plain B-tree, every node (leaf and internal) holds values. This means
# a search that finds the key in an internal node is done immediately.
#
# In a B+ tree, ONLY leaves hold values. Internal nodes hold SEPARATOR KEYS
# that guide search. This has two big advantages:
#
#   1. Full scans skip internal nodes entirely — just walk the leaf linked list
#   2. Internal nodes can hold MORE keys (no value storage) → shallower tree
#
# THE LEAF LINKED LIST
# ─────────────────────
# All leaf nodes are connected in a sorted singly-linked list:
#
#   root
#    │
#    └─ internal ─┬─ [1,2,3] →→ [4,5,6] →→ [7,8,9] →→ nil
#
# This is what makes range_scan and full_scan so efficient:
#   • range_scan(3, 7): find leaf with 3, walk right until key > 7 — O(log n + k)
#   • full_scan: start at leftmost leaf, walk to nil — O(n)
#
# LEAF SPLIT SEPARATOR RULE
# ──────────────────────────
# B+ tree leaf split:  separator COPIED to parent AND stays in right leaf
# B+ tree int split:   separator MOVES up (same as plain B-tree)
#
# The separator always equals the minimum key of the right child.
# =============================================================================

module CodingAdventures
  class BPlusTree
    # -------------------------------------------------------------------------
    # BPlusLeafNode: a leaf in the B+ tree
    #
    # Stores key-value pairs and a pointer to the next leaf in sorted order.
    # Invariant: t-1 ≤ keys.length ≤ 2t-1 (except root-as-leaf)
    # Invariant: keys are strictly ascending
    # -------------------------------------------------------------------------
    class BPlusLeafNode
      attr_accessor :keys, :values, :next_leaf

      def initialize
        @keys      = []    # Comparable keys in ascending order
        @values    = []    # values[i] corresponds to keys[i]
        @next_leaf = nil   # Pointer to the next leaf node (for range scans)
      end

      def leaf?
        true
      end

      def full?(t)
        @keys.length == 2 * t - 1
      end

      def inspect
        "Leaf(#{@keys.zip(@values).map { |k, v| "#{k}:#{v}" }.join(", ")})"
      end
    end

    # -------------------------------------------------------------------------
    # BPlusInternalNode: an internal (routing) node in the B+ tree
    #
    # Stores ONLY separator keys and child pointers. No values stored here.
    #
    # ROUTING INVARIANT:
    #   separator[i] == minimum key in children[i+1]
    #
    # This invariant must be maintained after every insert and delete.
    # -------------------------------------------------------------------------
    class BPlusInternalNode
      attr_accessor :keys, :children

      def initialize
        @keys     = []    # Separator keys (no values stored here)
        @children = []    # Child pointers (BPlusLeafNode or BPlusInternalNode)
      end

      def leaf?
        false
      end

      def full?(t)
        @keys.length == 2 * t - 1
      end

      def inspect
        "Internal(#{@keys.join(", ")})"
      end
    end
  end
end
