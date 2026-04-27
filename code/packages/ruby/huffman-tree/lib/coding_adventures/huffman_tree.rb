# frozen_string_literal: true

# =============================================================================
# CodingAdventures::HuffmanTree — DT27
# =============================================================================
#
# A Huffman tree is a full binary tree (every internal node has exactly two
# children) built from a symbol alphabet so that each symbol gets a unique
# variable-length bit code.  Symbols that appear often get short codes;
# symbols that appear rarely get long codes.  The total bits needed to encode
# a message is minimised — it is the theoretically optimal prefix-free code
# for a given symbol frequency distribution.
#
# Think of it like Morse code.  In Morse, "E" is "." (one dot) and "Z" is
# "--.." (four symbols).  The designers knew "E" is the most common letter in
# English so they gave it the shortest code.  Huffman's algorithm does this
# automatically and optimally for any alphabet with any frequency distribution.
#
# ── Algorithm: Greedy construction via min-heap ───────────────────────────────
#
# 1. Create one leaf node per distinct symbol, each with its frequency as its
#    weight.  Push all leaves onto a min-heap keyed by weight.
#
# 2. While the heap has more than one node:
#      a. Pop the two nodes with the smallest weight.
#      b. Create a new internal node whose weight = sum of the two children.
#      c. Set left = the first popped node, right = the second popped node.
#      d. Push the new internal node back onto the heap.
#
# 3. The one remaining node is the root of the Huffman tree.
#
# Tie-breaking rules (for deterministic output across implementations):
#   1. Lowest weight pops first.
#   2. Leaf nodes have higher priority than internal nodes at equal weight
#      ("leaf-before-internal" rule).
#   3. Among leaves of equal weight, lower symbol value wins.
#   4. Among internal nodes of equal weight, earlier-created node wins
#      (insertion-order FIFO).
#
# Why these rules?  Without tie-breaking, different implementations could
# build structurally different trees from the same input — producing different
# (but equally valid) code lengths.  Deterministic tie-breaking ensures the
# canonical code table is identical everywhere.
#
# ── Prefix-free property: why it works ───────────────────────────────────────
#
# In a Huffman tree:
#   - Symbols live ONLY at the leaves, never at internal nodes.
#   - The code for a symbol is the path from root to its leaf
#     (left edge = '0', right edge = '1').
#
# Since one leaf is never an ancestor of another leaf, no code can be a prefix
# of another code.  This is the prefix-free property, and it means the bit
# stream can be decoded unambiguously without separator characters: just walk
# the tree bit by bit until you hit a leaf.
#
# ── Canonical codes (DEFLATE / zlib style) ────────────────────────────────────
#
# The standard tree-walk produces valid codes, but different tree shapes can
# produce different codes for the same symbol lengths.  Canonical codes
# normalise this: given only the code *lengths*, you can reconstruct the exact
# canonical code table without transmitting the tree structure.
#
# Algorithm:
#   1. Collect (symbol, code_length) pairs from the tree.
#   2. Sort by (code_length, symbol_value).
#   3. Assign codes numerically:
#        code[0] = 0 (left-padded to length[0] bits)
#        code[i] = (code[i-1] + 1) << (length[i] - length[i-1])
#
# This is exactly what DEFLATE uses: the compressed stream contains only the
# length table, not the tree, saving space.
#
# Example with AAABBC:
#   A: weight=3, B: weight=2, C: weight=1
#   Tree:      [6]
#              / \
#             A   [3]
#            (3)  / \
#                B   C
#               (2) (1)
#   Lengths: A=1, B=2, C=2
#   Sorted by (length, symbol): A(1), B(2), C(2)
#   Canonical codes:
#     A → 0        (length 1,  code = 0)
#     B → 10       (length 2,  code = 0+1=1, shifted 1 bit → 10)
#     C → 11       (length 2,  code = 10+1 = 11)
# =============================================================================

require "coding_adventures_heap"
require_relative "huffman_tree/version"

module CodingAdventures
  # ── Node types ────────────────────────────────────────────────────────────────

  # A Leaf node represents a single symbol at the bottom of the tree.
  # Every symbol in the alphabet becomes exactly one Leaf.
  #
  # Attributes:
  #   symbol  — integer identifier for the symbol (e.g. ASCII byte value)
  #   weight  — how often the symbol appears (its frequency)
  #
  # Leaves are immutable value objects: once created, neither the symbol
  # nor the weight can change.
  class HuffmanLeaf
    attr_reader :symbol, :weight

    # @param symbol [Integer] non-negative integer identifier
    # @param weight [Integer] positive frequency count
    def initialize(symbol, weight)
      @symbol = symbol
      @weight = weight
    end
  end

  # An Internal node combines two sub-trees into one.
  # It has no symbol of its own — only the leaves below it hold symbols.
  #
  # Attributes:
  #   weight  — sum of left.weight + right.weight
  #   left    — the sub-tree that the '0' edge leads to
  #   right   — the sub-tree that the '1' edge leads to
  #   order   — monotonic counter recording creation order (for tie-breaking)
  #
  # The order field is NOT part of the logical tree semantics.  It exists solely
  # so the heap comparator can break ties deterministically: earlier-created
  # internal nodes have higher priority (FIFO rule).
  class HuffmanInternal
    attr_reader :weight, :left, :right, :order

    # @param weight [Integer] combined weight
    # @param left   [HuffmanLeaf, HuffmanInternal] first popped child (lower priority)
    # @param right  [HuffmanLeaf, HuffmanInternal] second popped child (higher priority)
    # @param order  [Integer] creation counter (0 = first created)
    def initialize(weight, left, right, order)
      @weight = weight
      @left = left
      @right = right
      @order = order
    end
  end

  # ── HuffmanTree ───────────────────────────────────────────────────────────────

  # A full binary tree that assigns optimal prefix-free bit codes to symbols.
  #
  # Build the tree once from symbol frequencies; then:
  #   - Use #code_table to get a {symbol => bit_string} map for encoding.
  #   - Use #decode_all to decode a bit stream back to symbols.
  #   - Use #canonical_code_table for DEFLATE-style transmissible codes.
  #
  # All symbols are integers (typically 0..255 for byte-level coding, but any
  # non-negative integer is valid).  Frequencies must be positive integers.
  #
  # The tree is immutable after construction.  Build a new tree if frequencies
  # change.
  #
  # Example:
  #   tree  = CodingAdventures::HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
  #   table = tree.code_table
  #   table[65]  # => "0"   (A gets shortest code)
  #   tree.decode_all("0", 1)  # => [65]
  class HuffmanTree
    # ── Construction ──────────────────────────────────────────────────────────

    # Build a Huffman tree from (symbol, frequency) pairs.
    #
    # The greedy algorithm uses a min-heap.  At each step it pops the two
    # lowest-weight nodes, combines them into a new internal node, and pushes
    # the internal node back.  The single remaining node is the root.
    #
    # Tie-breaking (for deterministic output across implementations):
    #   1. Lowest weight pops first.
    #   2. Leaves before internal nodes at equal weight.
    #   3. Lower symbol value wins among leaves of equal weight.
    #   4. Earlier-created internal node wins among internal nodes of equal
    #      weight (FIFO insertion order).
    #
    # @param weights [Array<Array(Integer, Integer)>] list of [symbol, frequency]
    # @return [HuffmanTree]
    # @raise [ArgumentError] if weights is empty or any frequency is <= 0
    #
    # Example:
    #   tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
    #   tree.symbol_count  # => 3
    def self.build(weights)
      raise ArgumentError, "weights must not be empty" if weights.empty?

      weights.each do |sym, freq|
        if freq <= 0
          raise ArgumentError,
            "frequency must be positive; got symbol=#{sym}, freq=#{freq}"
        end
      end

      # A min-heap ordered by the node priority tuple.
      #
      # Each element is a [priority_tuple, node] pair.  The heap comparator
      # compares the priority tuples element-by-element so Ruby's Array <=>
      # gives us lexicographic ordering for free.
      #
      # Priority tuple: [weight, leaf_flag, symbol_or_huge, order_or_huge]
      #   weight        — lower is better (pop smallest weight first)
      #   leaf_flag     — 0 = leaf, 1 = internal  (leaves pop before internals)
      #   symbol_or_huge — for leaves: symbol value; for internals: Float::INFINITY
      #   order_or_huge  — for internals: creation counter; for leaves: Float::INFINITY
      heap = CodingAdventures::Heap::MinHeap.new { |a, b| a[0] <=> b[0] }

      weights.each do |sym, freq|
        leaf = HuffmanLeaf.new(sym, freq)
        heap.push([node_priority(leaf), leaf])
      end

      order_counter = 0 # monotonic counter for internal node creation order

      while heap.size > 1
        _, left = heap.pop
        _, right = heap.pop

        combined_weight = left.weight + right.weight
        internal = HuffmanInternal.new(combined_weight, left, right, order_counter)
        order_counter += 1

        heap.push([node_priority(internal), internal])
      end

      _, root = heap.pop
      new(root, weights.length)
    end

    # ── Encoding helpers ───────────────────────────────────────────────────────

    # Return {symbol => bit_string} for all symbols in the tree.
    #
    # Left edges are '0', right edges are '1'.  For a single-symbol tree the
    # convention is {symbol => '0'} (one bit per occurrence).
    #
    # Time: O(n) where n = number of distinct symbols.
    #
    # Example:
    #   tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
    #   tree.code_table  # => {65=>"0", 66=>"10", 67=>"11"}
    def code_table
      table = {}
      walk(@root, "", table)
      table
    end

    # Return the bit string for a specific symbol, or nil if not in the tree.
    #
    # Walks the tree searching for the leaf with the given symbol; does NOT
    # build the full code table.
    #
    # Time: O(n) worst case (full tree traversal).
    #
    # Example:
    #   tree.code_for(65)   # => "0"
    #   tree.code_for(999)  # => nil
    def code_for(symbol)
      find_code(@root, symbol, "")
    end

    # Return canonical Huffman codes (DEFLATE-style).
    #
    # Sorted by (code_length, symbol_value); codes assigned numerically.
    # Useful when you need to transmit only code lengths, not the tree.
    #
    # Time: O(n log n).
    #
    # Example:
    #   tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
    #   tree.canonical_code_table  # => {65=>"0", 66=>"10", 67=>"11"}
    def canonical_code_table
      # Step 1: collect code lengths for every leaf
      lengths = {}
      collect_lengths(@root, 0, lengths)

      # Single-leaf edge case: assign length 1 by convention (one '0' bit)
      if lengths.size == 1
        sym = lengths.keys.first
        return {sym => "0"}
      end

      # Step 2: sort by (length, symbol_value) — this is the canonical ordering
      sorted_syms = lengths.sort_by { |sym, len| [len, sym] }

      # Step 3: assign canonical codes numerically.
      #
      # We start at code 0 for the shortest code.  Each time we advance to the
      # next symbol we increment the code by 1.  When the code length increases
      # we left-shift to "make room" for the extra bit.  This matches DEFLATE.
      code_val = 0
      prev_len = sorted_syms.first[1]
      result = {}

      sorted_syms.each do |sym, length|
        code_val <<= (length - prev_len) if length > prev_len
        result[sym] = format("%0#{length}b", code_val)
        code_val += 1
        prev_len = length
      end

      result
    end

    # ── Decoding ──────────────────────────────────────────────────────────────

    # Decode exactly +count+ symbols from a bit string by walking the tree.
    #
    # The decoder starts at the root.  It reads bits one at a time:
    #   '0' → go left
    #   '1' → go right
    # When it reaches a leaf it records the symbol and resets to the root.
    #
    # Args:
    #   bits  — String of '0' and '1' characters
    #   count — exact number of symbols to decode
    #
    # Returns: Array of decoded symbol integers, length == count.
    #
    # Raises ArgumentError if the bit stream is exhausted before +count+
    # symbols are decoded.
    #
    # For a single-leaf tree, each '0' bit decodes to that symbol.
    #
    # Time: O(total bits consumed).
    #
    # Example:
    #   tree = HuffmanTree.build([[65, 3], [66, 2], [67, 1]])
    #   tree.decode_all("010011", 4)  # => [65, 65, 66, 67]
    def decode_all(bits, count)
      result = []
      node = @root
      i = 0

      # Single-leaf trees have a root that is already a leaf.  Each symbol is
      # represented by exactly one '0' bit.  Multi-leaf trees: when we reach
      # a leaf the index is already past the last consumed bit — no extra
      # advance needed.
      single_leaf = @root.is_a?(HuffmanLeaf)

      while result.length < count
        if node.is_a?(HuffmanLeaf)
          result << node.symbol
          node = @root
          if single_leaf
            # Consume the dummy '0' bit for this symbol
            i += 1 if i < bits.length
          end
          next
        end

        if i >= bits.length
          raise ArgumentError,
            "Bit stream exhausted after #{result.length} symbols; expected #{count}"
        end

        bit = bits[i]
        i += 1
        node = (bit == "0") ? node.left : node.right
      end

      result
    end

    # ── Inspection ────────────────────────────────────────────────────────────

    # Total weight of the tree = sum of all leaf frequencies = root weight.
    # O(1) — stored at the root.
    #
    # Example:
    #   tree.weight  # => 6
    def weight
      @root.weight
    end

    # Maximum code length = depth of the deepest leaf.
    # O(n) — must traverse the tree.
    #
    # For a single-leaf tree the depth is 0 (root IS the leaf).
    #
    # Example:
    #   tree.depth  # => 2
    def depth
      max_depth(@root, 0)
    end

    # Number of distinct symbols (= number of leaf nodes).
    # O(1) — stored at construction time.
    #
    # Example:
    #   tree.symbol_count  # => 3
    attr_reader :symbol_count

    # In-order traversal of leaves.
    #
    # Returns [[symbol, code], ...], left subtree before right subtree.
    # Useful for visualisation and debugging.
    #
    # Time: O(n).
    #
    # Example:
    #   tree.leaves  # => [[65, "0"], [66, "10"], [67, "11"]]
    def leaves
      table = code_table
      result = []
      in_order_leaves(@root, result, table)
      result
    end

    # Check structural invariants.  For testing only.
    #
    #   1. Every internal node has exactly 2 children (full binary tree).
    #   2. weight(internal) == weight(left) + weight(right).
    #   3. No symbol appears in more than one leaf.
    #
    # Returns true if all invariants hold.
    #
    # Example:
    #   tree.valid?  # => true
    def valid?
      seen = Set.new
      check_invariants(@root, seen)
    end

    private

    # ── Private constructor ────────────────────────────────────────────────────

    # @param root         [HuffmanLeaf, HuffmanInternal] the root node
    # @param symbol_count [Integer] how many distinct symbols are in the tree
    def initialize(root, symbol_count)
      @root = root
      @symbol_count = symbol_count
    end

    # ── Priority key ──────────────────────────────────────────────────────────

    # Compute the 4-element priority tuple for a node.
    #
    # Lower values = higher priority (min-heap pops the smallest key first).
    #
    # Fields:
    #   [0] weight         — lower weight wins
    #   [1] leaf_flag      — 0=leaf (higher priority), 1=internal
    #   [2] symbol_or_huge — leaf: symbol value; internal: Float::INFINITY
    #   [3] order_or_huge  — internal: insertion order (FIFO); leaf: Float::INFINITY
    #
    # Ruby's Array <=> compares element-by-element, so this tuple gives us
    # the full tie-breaking order with a single comparison.
    def self.node_priority(node)
      if node.is_a?(HuffmanLeaf)
        [node.weight, 0, node.symbol, Float::INFINITY]
      else
        [node.weight, 1, Float::INFINITY, node.order]
      end
    end
    private_class_method :node_priority

    # ── Tree traversal helpers ─────────────────────────────────────────────────

    # Recursively walk the tree building the code table.
    #
    # We pass the current path prefix as a string and append '0' or '1' at
    # each branching point.  The single-leaf edge case: if the prefix is
    # empty when we reach a leaf (i.e., the tree has only one node) we
    # assign '0' by convention.
    def walk(node, prefix, table)
      if node.is_a?(HuffmanLeaf)
        table[node.symbol] = prefix.empty? ? "0" : prefix
        return
      end
      walk(node.left, "#{prefix}0", table)
      walk(node.right, "#{prefix}1", table)
    end

    # Search the tree for a specific symbol and return its code, or nil.
    #
    # We try the left sub-tree first.  If the symbol is found there we return
    # immediately without exploring the right sub-tree (short-circuit search).
    def find_code(node, symbol, prefix)
      if node.is_a?(HuffmanLeaf)
        if node.symbol == symbol
          return prefix.empty? ? "0" : prefix
        else
          return nil
        end
      end
      left_result = find_code(node.left, symbol, "#{prefix}0")
      return left_result unless left_result.nil?

      find_code(node.right, symbol, "#{prefix}1")
    end

    # Collect code lengths for all leaves.
    #
    # Single-leaf trees have depth 0, but we assign length 1 by convention
    # (every encoding of a symbol must consume at least one bit).
    def collect_lengths(node, depth, lengths)
      if node.is_a?(HuffmanLeaf)
        lengths[node.symbol] = (depth > 0) ? depth : 1
        return
      end
      collect_lengths(node.left, depth + 1, lengths)
      collect_lengths(node.right, depth + 1, lengths)
    end

    # Return the maximum depth of any leaf in the subtree rooted at +node+.
    def max_depth(node, depth)
      return depth if node.is_a?(HuffmanLeaf)

      [max_depth(node.left, depth + 1), max_depth(node.right, depth + 1)].max
    end

    # Collect leaves in left-to-right (in-order) traversal.
    def in_order_leaves(node, result, table)
      if node.is_a?(HuffmanLeaf)
        result << [node.symbol, table[node.symbol]]
        return
      end
      in_order_leaves(node.left, result, table)
      in_order_leaves(node.right, result, table)
    end

    # Recursively validate tree invariants.
    #
    # We pass a Set of already-seen symbols so we can detect duplicate leaves.
    # If any internal node has a weight mismatch or any symbol appears twice,
    # we return false immediately.
    def check_invariants(node, seen)
      if node.is_a?(HuffmanLeaf)
        return false if seen.include?(node.symbol)

        seen.add(node.symbol)
        return true
      end
      return false if node.weight != node.left.weight + node.right.weight

      check_invariants(node.left, seen) && check_invariants(node.right, seen)
    end
  end
end
