# frozen_string_literal: true

# ================================================================
# ImmutableList — Persistent Vector with Structural Sharing
# ================================================================
#
# What is a persistent data structure?
# ======================================
# A "persistent" data structure is one that *preserves its previous
# versions* when you modify it. Instead of mutating in place, every
# "modification" returns a brand-new version while leaving the old
# one completely unchanged.
#
# This is the opposite of a Ruby Array, which mutates in place:
#
#   arr = ["a", "b", "c"]
#   arr.push("d")         # arr is now ["a", "b", "c", "d"] — MUTATED
#
# With an ImmutableList:
#
#   list = ImmutableList.empty
#   list2 = list.push("a")    # list is still empty; list2 has ["a"]
#   list3 = list2.push("b")   # list2 still has ["a"]; list3 has ["a","b"]
#
# This "time travel" property is invaluable for:
#   - Functional programming (pure functions, no side effects)
#   - Undo/redo systems (each version is an old state)
#   - Concurrent access (no locks needed — readers see a stable snapshot)
#
# The 32-way trie
# ================
# The naive implementation of persistence is to copy the entire array
# on every modification. For a 1000-element list, every push costs O(n).
#
# A smarter approach uses a **trie** (prefix tree). Instead of one big
# array, the data is stored in a tree of small fixed-size arrays. Each
# internal node has up to 32 children. A modification only needs to copy
# the nodes along a single root-to-leaf path — O(log₃₂(n)) nodes.
#
# For 1,000,000 elements, that's only log₃₂(1_000_000) ≈ 4 nodes copied.
# Compare to copying 1,000,000 elements the naive way!
#
#   tree (shift=5, height=2)
#   ┌──────────────────────────────────────┐
#   │ root (internal node, 32 slots)       │
#   │  [0]: [v0, v1, …, v31]              │  ← leaf node (32 values)
#   │  [1]: [v32, v33, …, v63]            │  ← leaf node
#   │  …                                   │
#   └──────────────────────────────────────┘
#
# Tail buffer optimization
# =========================
# Every push to the end of the trie would require traversing the tree.
# Clojure (and our implementation) uses a **tail buffer** — a plain Ruby
# Array holding the last up to 32 elements. Appending to the tail is O(1).
# Only when the tail fills up to 32 elements does it get "promoted" into
# the trie (one O(log n) trie update, amortized to near-O(1) per push).
#
# Key invariant: tail.length >= 1 (except for the empty list).
#
# Architecture summary
# =====================
#   ImmutableList has four fields:
#     @root  — the root node of the trie (Array of child nodes or leaf arrays)
#     @size  — total number of elements
#     @shift — bit shift for indexing (5 for height-1, 10 for height-2, …)
#     @tail  — a frozen Ruby Array holding the last 1..32 elements
#
#   BITS      = 5     (each node address fits in 5 bits)
#   BRANCHING = 32    (2^5 = 32 children per node)
#   MASK      = 0x1F  (5-bit mask to extract a node index)
#
# Indexing
# =========
# To look up element i:
#   1. If i is in the tail: tail[i - tail_offset]
#   2. Otherwise, walk the trie: at each level, use 5 bits of i as the
#      child index. The "level" is controlled by @shift.
#
#       level=1: child index = (i >> 5) & 0x1F
#       level=2: child index = (i >> 10) & 0x1F
#       level=3: child index = (i >> 15) & 0x1F
#       … until we reach a leaf array.

module CodingAdventures
  module ImmutableList
    # Branching factor of the trie. Each node holds up to 32 children.
    # 32 is a sweet spot: wide enough to keep the tree shallow, small
    # enough to fit in a CPU cache line.
    BITS = 5
    BRANCHING = 1 << BITS   # 32
    MASK = BRANCHING - 1    # 0x1F = 31

    # PersistentVector is the main class implementing the immutable list.
    #
    # Instances are frozen on creation — no mutation is possible after
    # construction. Every modifying operation returns a new instance.
    class PersistentVector
      # @return [Integer] total number of elements in the list
      attr_reader :size

      # Create a new PersistentVector. Not intended for direct use;
      # use PersistentVector.empty or the factory methods instead.
      #
      # @param root  [Array, nil] the trie root (nil for empty)
      # @param size  [Integer]   total element count
      # @param shift [Integer]   trie height encoded as bit shift
      # @param tail  [Array]     the tail buffer (last ≤32 elements)
      def initialize(root, size, shift, tail)
        @root  = root
        @size  = size
        @shift = shift
        @tail  = tail
        freeze
      end

      # ──────────────────────────────────────────────────────────────
      # Factory: the canonical empty list (singleton-like)
      # ──────────────────────────────────────────────────────────────

      # @return [PersistentVector] the canonical empty list
      def self.empty
        new(nil, 0, BITS, [].freeze)
      end

      # Build a PersistentVector from a Ruby Array in one shot.
      #
      # @param arr [Array] source elements
      # @return [PersistentVector] a new vector containing those elements
      def self.from_array(arr)
        arr.reduce(empty) { |v, el| v.push(el) }
      end

      # ──────────────────────────────────────────────────────────────
      # Queries
      # ──────────────────────────────────────────────────────────────

      # @return [Boolean] true iff the list is empty
      def empty?
        @size == 0
      end

      # Retrieve the element at +index+.
      #
      # Out-of-bounds indices return nil (consistent with Ruby Array#[]).
      # Negative indices are not supported and return nil.
      #
      # @param index [Integer] 0-based index
      # @return [Object, nil] the element or nil
      def get(index)
        return nil if index < 0 || index >= @size

        if index >= tail_offset
          # Fast path: element is in the tail buffer
          @tail[index - tail_offset]
        else
          # Slow path: traverse the trie
          node = @root
          level = @shift
          while level > 0
            child_index = (index >> level) & MASK
            node = node[child_index]
            level -= BITS
          end
          node[index & MASK]
        end
      end

      # Array-subscript alias for get.
      alias [] get

      # ──────────────────────────────────────────────────────────────
      # Modifications (all return a new PersistentVector)
      # ──────────────────────────────────────────────────────────────

      # Return a new list with +value+ appended to the end.
      #
      # Implementation sketch:
      #   - If the tail still has room (< 32 elements), just copy the
      #     tail array and append to the copy. O(1) amortized.
      #   - If the tail is full (32 elements), promote it into the trie,
      #     then start a fresh tail. O(log n).
      #
      # @param value [Object] the element to append
      # @return [PersistentVector] a new list with one more element
      def push(value)
        new_size  = @size + 1
        new_tail_size = @size - tail_offset

        if new_tail_size < BRANCHING
          # Tail still has room — just grow the tail
          new_tail = (@tail + [value]).freeze
          PersistentVector.new(@root, new_size, @shift, new_tail)
        else
          # Tail is full — promote the old tail into the trie
          new_root, new_shift = push_tail(@root, @shift, @tail)
          new_tail = [value].freeze
          PersistentVector.new(new_root, new_size, new_shift, new_tail)
        end
      end

      # Return a new list with the element at +index+ replaced by +value+.
      #
      # Raises IndexError for out-of-bounds indices.
      #
      # @param index [Integer] 0-based index
      # @param value [Object]  the replacement value
      # @return [PersistentVector] a new list with the element replaced
      def set(index, value)
        raise IndexError, "index #{index} out of bounds (size #{@size})" if index < 0 || index >= @size

        if index >= tail_offset
          # Replace in the tail — just copy the tail with one slot changed
          new_tail = @tail.dup
          new_tail[index - tail_offset] = value
          PersistentVector.new(@root, @size, @shift, new_tail.freeze)
        else
          # Replace in the trie — copy the path from root to leaf
          new_root = set_in_trie(@root, @shift, index, value)
          PersistentVector.new(new_root, @size, @shift, @tail)
        end
      end

      # Remove and return the last element.
      #
      # Returns a two-element Array: [new_vector, removed_value].
      # Raises IndexError if called on an empty list.
      #
      # Example:
      #   list = ImmutableList.of("a", "b", "c")
      #   shorter, val = list.pop
      #   val     # => "c"
      #   shorter # => ImmutableList["a", "b"]
      #
      # @return [Array(PersistentVector, Object)] [new_list, removed_element]
      def pop
        raise IndexError, "cannot pop from an empty list" if @size == 0

        last_val = get(@size - 1)

        if @size == 1
          return [PersistentVector.empty, last_val]
        end

        new_size = @size - 1

        if (@size - tail_offset) > 1
          # Tail still has multiple elements — just shrink the tail
          new_tail = @tail[0...-1].freeze
          return [PersistentVector.new(@root, new_size, @shift, new_tail), last_val]
        end

        # Tail had exactly one element — we need to pull the last leaf from the trie
        new_tail = leaf_for(@root, @shift, new_size - 1).freeze
        new_root, new_shift = pop_tail(@root, @shift)

        [PersistentVector.new(new_root, new_size, new_shift, new_tail), last_val]
      end

      # ──────────────────────────────────────────────────────────────
      # Conversion and display
      # ──────────────────────────────────────────────────────────────

      # @return [Array] a new Ruby Array containing all elements in order
      def to_a
        result = []
        @size.times { |i| result << get(i) }
        result
      end

      # @return [String] a human-readable representation
      def to_s
        "ImmutableList[#{to_a.map(&:inspect).join(", ")}]"
      end

      alias inspect to_s

      # Equality by value.
      def ==(other)
        return false unless other.is_a?(PersistentVector)
        return false unless other.size == @size
        @size.times.all? { |i| get(i) == other.get(i) }
      end

      # ──────────────────────────────────────────────────────────────
      # Private helpers
      # ──────────────────────────────────────────────────────────────

      private

      # The index of the first element stored in the tail.
      # Everything from 0 to tail_offset-1 is in the trie.
      # Everything from tail_offset to size-1 is in @tail.
      #
      # Formula: (size - 1) with its bottom 5 bits zeroed out,
      # i.e. the floor of (size-1) rounded down to the nearest 32.
      def tail_offset
        return 0 if @size < BRANCHING
        ((@size - 1) >> BITS) << BITS
      end

      # Promote a full tail leaf into the trie, returning the new root
      # and (possibly grown) shift.
      #
      # If the trie overflows its current capacity, we grow the tree by
      # one level (shift += BITS) and create a new root with two children:
      # the old root and a new right subtree leading to the leaf.
      def push_tail(root, shift, tail_leaf)
        if root.nil?
          # First trie node — the leaf becomes the only child
          new_root = [tail_leaf].freeze
          return [new_root, shift]
        end

        # Try to insert into the existing tree
        new_root = insert_leaf(root, shift, @size - 1, tail_leaf)

        if new_root.nil?
          # Tree is full at current height — grow by one level
          new_shift = shift + BITS
          path_for_leaf = new_path(shift, tail_leaf)
          new_root = [root, path_for_leaf].freeze
          [new_root, new_shift]
        else
          [new_root, shift]
        end
      end

      # Recursively insert +leaf+ into the subtree rooted at +node+ at the
      # position implied by +index+. Returns nil if the subtree is full.
      def insert_leaf(node, shift, index, leaf)
        child_index = (index >> shift) & MASK

        if shift == BITS
          # One level above the leaf — node holds leaf arrays
          if child_index < node.length
            # Replace existing slot
            new_node = node.dup
            new_node[child_index] = leaf
            new_node.freeze
          else
            # Append new slot (only valid if child_index == node.length)
            return nil if child_index > node.length
            (node + [leaf]).freeze
          end
        else
          # Internal node — recurse
          if child_index < node.length
            new_child = insert_leaf(node[child_index], shift - BITS, index, leaf)
            if new_child
              new_node = node.dup
              new_node[child_index] = new_child
              new_node.freeze
            else
              return nil if child_index < node.length - 1
              new_sub = new_path(shift - BITS, leaf)
              return nil if node.length >= BRANCHING
              (node + [new_sub]).freeze
            end
          else
            # Append a brand-new path to leaf
            return nil if node.length >= BRANCHING
            new_sub = new_path(shift - BITS, leaf)
            (node + [new_sub]).freeze
          end
        end
      end

      # Build a brand-new trie path from +shift+ levels down to a leaf.
      # Returns an Array wrapping the leaf, wrapped again per level.
      def new_path(shift, leaf)
        if shift == 0
          leaf
        else
          [new_path(shift - BITS, leaf)].freeze
        end
      end

      # Replace the value at +index+ inside the trie, returning a new root.
      # Copies only the nodes along the path (structural sharing for the rest).
      def set_in_trie(node, shift, index, value)
        child_index = (index >> shift) & MASK
        new_node = node.dup

        if shift == 0
          # Leaf level — replace the value directly
          new_node[child_index] = value
        else
          # Internal level — recurse into the child
          new_node[child_index] = set_in_trie(node[child_index], shift - BITS, index, value)
        end

        new_node.freeze
      end

      # Find the leaf array that contains the element at +index+.
      def leaf_for(node, shift, index)
        if shift == 0
          node
        else
          child_index = (index >> shift) & MASK
          leaf_for(node[child_index], shift - BITS, index)
        end
      end

      # Pop the rightmost leaf from the trie, returning [new_root, new_shift].
      # The actual last value has already been captured by the caller.
      def pop_tail(node, shift)
        if node.nil? || node.empty?
          return [nil, BITS]
        end

        new_root = remove_last_leaf(node, shift)

        # Shrink the tree height if the root now has only one child
        if new_root && new_root.length == 1 && shift > BITS
          [new_root[0], shift - BITS]
        else
          [new_root, shift]
        end
      end

      # Recursively remove the rightmost leaf from the subtree. Returns the
      # updated node (or nil if the node becomes empty).
      def remove_last_leaf(node, shift)
        return nil if node.nil? || node.empty?

        if shift == BITS
          # One level above leaves
          new_node = node[0...-1].freeze
          new_node.empty? ? nil : new_node
        else
          last_child_idx = node.length - 1
          new_child = remove_last_leaf(node[last_child_idx], shift - BITS)

          new_node = node.dup
          if new_child.nil?
            new_node.pop
          else
            new_node[last_child_idx] = new_child
          end
          new_node.empty? ? nil : new_node.freeze
        end
      end
    end
  end
end
