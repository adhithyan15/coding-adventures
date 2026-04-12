# frozen_string_literal: true

# =============================================================================
# coding_adventures/b_tree.rb -- B-Tree (Balanced Multi-way Search Tree)
# =============================================================================
#
# WHAT IS A B-TREE?
# ──────────────────
# A B-tree is a self-balancing search tree designed for systems that read and
# write large blocks of data — most famously hard disks and SSDs.
#
# It was invented in 1970 by Rudolf Bayer and Edward McCreight while working
# at Boeing Research Labs. The "B" likely stands for "Balanced" (or "Boeing").
#
# THE CORE INSIGHT
# ─────────────────
# Hard disks are slow (milliseconds per read), but reading a large block is
# nearly as fast as reading a single byte. So instead of binary trees with
# one key per node, use "fat" nodes with MANY keys. A B-tree of degree 128
# with height 3 can index over 4 million records with only 3 disk reads.
#
# For in-memory use, B-trees still shine because:
#   • Cache efficiency: fewer cache misses than pointer-heavy BSTs
#   • Predictable height: O(log_t n) which is very shallow for large t
#   • Ordered iteration: inorder traversal is straightforward
#
# STRUCTURE AT A GLANCE
# ──────────────────────
#
#   Minimum degree t = 2 (the classic "2-3-4 tree"):
#
#            [20]
#           /    \
#        [10]   [30, 40]
#        /  \   /  |  \
#       [5] [15][25][35][50]
#
#   - Root: [20] — 1 key, 2 children
#   - Internal: [30, 40] — 2 keys, 3 children  (≥ t-1=1, ≤ 2t-1=3)
#   - Leaves: [5], [15], etc. — no children
#
# ALL LEAVES ARE AT THE SAME DEPTH — this is the "balanced" invariant.
#
# OPERATIONS
# ───────────
#   search:  O(t · log_t n)  — binary search within nodes + descent
#   insert:  O(t · log_t n)  — proactive splits on the way down
#   delete:  O(t · log_t n)  — fill-up or merge on the way down
#   range:   O(t · log_t n + k) — k results returned
#
# ALGORITHM REFERENCE
# ────────────────────
# Cormen, Leiserson, Rivest, Stein — "Introduction to Algorithms", 3rd ed.,
# Chapter 18: "B-Trees". The implementation follows CLRS conventions closely.
#
# B-TREE VS B+ TREE
# ──────────────────
# The plain B-tree stores values in EVERY node (leaf and internal).
# The B+ tree stores values ONLY in leaves, and links leaves in a list.
# B+ trees are better for range scans; B-trees are better for point lookups
# when many queries hit internal nodes. See the b_plus_tree package for B+.
#
# =============================================================================

require_relative "b_tree/node"

module CodingAdventures
  # ===========================================================================
  # class BTree
  # ===========================================================================
  #
  # A full-featured, in-memory B-tree. Keys must be Comparable (Integer,
  # String, Float, or any object implementing <=>). Values can be anything.
  #
  # Usage:
  #
  #   tree = CodingAdventures::BTree.new(t: 3)
  #   tree.insert(10, "ten")
  #   tree.insert(5,  "five")
  #   tree.insert(20, "twenty")
  #   tree.search(10)           # => "ten"
  #   tree.range_query(1, 15)   # => [[5, "five"], [10, "ten"]]
  #   tree.delete(10)
  #   tree.include?(10)         # => false
  #   tree.valid?               # => true
  #
  class BTree
    include Comparable

    # The minimum degree t. Each non-root node has between t-1 and 2t-1 keys.
    # t = 2 gives a "2-3-4 tree" (each node has 2, 3, or 4 children).
    attr_reader :t

    # -------------------------------------------------------------------------
    # new(t: 2) -- Create an empty B-tree with minimum degree t.
    #
    # @param t [Integer] minimum degree, must be >= 2
    # -------------------------------------------------------------------------
    def initialize(t: 2)
      raise ArgumentError, "t must be >= 2, got #{t}" if t < 2

      @t    = t
      @root = BTreeNode.new(leaf: true)  # Start with an empty leaf root
      @size = 0
    end

    # =========================================================================
    # SEARCH — Find a key and return its value
    # =========================================================================
    #
    # We descend from the root using binary search at each node.
    # At each node either:
    #   a) We find the key (return its value), or
    #   b) We identify the child subtree to descend into.
    #
    # Time: O(t · log_t n) — log_t n levels, up to 2t-1 comparisons per level.
    # =========================================================================

    # Return the value for +key+, or nil if not found.
    def search(key)
      _search(@root, key)
    end

    # Return true if +key+ exists in the tree.
    def include?(key)
      !search(key).nil?
    end

    alias member? include?

    # Return the value for +key+, raising KeyError if absent.
    def [](key)
      val = search(key)
      raise KeyError, "key not found: #{key.inspect}" if val.nil? && !_key_exists?(@root, key)
      val
    end

    # =========================================================================
    # INSERT — Add a key-value pair
    # =========================================================================
    #
    # CLRS "proactive split" strategy: we split any full node we encounter
    # on the WAY DOWN. This guarantees that when we reach a leaf, its parent
    # has room for a promoted key after a split.
    #
    # Why proactive? Because if we split on the way UP (retroactive), we might
    # need to split all the way to the root, requiring a second traversal.
    # Proactive splits do it in a single root-to-leaf pass.
    #
    # SPLIT ANIMATION
    # ────────────────
    # A full node [k1 k2 k3 k4 k5] (t=3) splits into:
    #
    #   Left [k1 k2] | k3 (promoted) | Right [k4 k5]
    #
    # k3 (the median) moves UP to the parent. The node becomes two nodes.
    #
    # Before split:
    #   Parent: [k0]
    #   Child:  [k1 k2 k3 k4 k5]  ← full
    #
    # After split:
    #   Parent: [k0, k3]           ← k3 inserted
    #   Left:   [k1 k2]            ← original node, trimmed
    #   Right:  [k4 k5]            ← new node
    # =========================================================================

    # Insert +key+ with +value+. If +key+ already exists, updates the value.
    def []=(key, value)
      insert(key, value)
    end

    def insert(key, value)
      root = @root

      if root.full?(@t)
        # The root is full — we must grow the tree height by 1.
        # Create a new root whose only child is the old root, then split.
        #
        #   Old: root=[k1..k2t-1]
        #   New: new_root → [left][right]
        #        where left and right are halves of old root
        #
        new_root = BTreeNode.new(leaf: false)
        new_root.children.push(root)
        @root = new_root
        _split_child(new_root, 0)
        _insert_non_full(new_root, key, value)
      else
        _insert_non_full(root, key, value)
      end
    end

    # =========================================================================
    # DELETE — Remove a key from the tree
    # =========================================================================
    #
    # Deletion is the most complex B-tree operation. CLRS identifies 3 cases:
    #
    # CASE 1: Key is in a LEAF node.
    #   → Simply remove it. (Node still has ≥ t-1 keys because we filled up
    #     sparse nodes on the way down.)
    #
    # CASE 2: Key is in an INTERNAL node.
    #   2a. Left child has ≥ t keys → replace key with its PREDECESSOR
    #       (the largest key in the left subtree), then delete the predecessor.
    #   2b. Right child has ≥ t keys → replace key with its SUCCESSOR
    #       (the smallest key in the right subtree), then delete the successor.
    #   2c. Both children have t-1 keys → merge them (key descends into merged
    #       node), then delete from the merged node.
    #
    # CASE 3: Key is NOT in this node — descend into the correct child.
    #   Before descending, ensure the child has ≥ t keys:
    #     Rotate-Left:  borrow from right sibling (if it has ≥ t keys)
    #     Rotate-Right: borrow from left sibling (if it has ≥ t keys)
    #     Merge:        merge with a sibling (when both have t-1 keys)
    # =========================================================================

    def delete(key)
      return unless _key_exists?(@root, key)

      _delete(@root, key)

      # If the root lost its last key (after a merge), its only child becomes
      # the new root. This is how the tree shrinks in height.
      if @root.keys.empty? && !@root.leaf?
        @root = @root.children[0]
      end

      @size -= 1
    end

    # =========================================================================
    # ACCESSORS
    # =========================================================================

    # Return the minimum key in the tree (leftmost leaf).
    def min_key
      raise "Tree is empty" if empty?
      node = @root
      node = node.children[0] until node.leaf?
      node.keys[0]
    end

    # Return the maximum key in the tree (rightmost leaf's last key).
    def max_key
      raise "Tree is empty" if empty?
      node = @root
      node = node.children[-1] until node.leaf?
      node.keys[-1]
    end

    # Return all [key, value] pairs in sorted order (in-order traversal).
    #
    # In a B-tree, in-order means: for each node, interleave children and keys:
    #   inorder(child[0]), key[0], inorder(child[1]), key[1], ...
    def inorder
      result = []
      _inorder(@root, result)
      result
    end

    # Return all [key, value] pairs where low <= key <= high, in sorted order.
    #
    # This is more efficient than full inorder + filter because we prune
    # subtrees that cannot contain keys in range.
    def range_query(low, high)
      result = []
      _range(@root, low, high, result)
      result
    end

    # How many key-value pairs are stored?
    def size
      # Count lazily — we maintain @size incrementally
      @size
    end

    # Is the tree empty?
    def empty?
      @size == 0
    end

    # Height of the tree (0 for a single-node/empty tree).
    #
    # All leaves are at the same depth by the B-tree invariant, so we just
    # walk down the leftmost path.
    def height
      return 0 if @root.leaf?
      h = 0
      node = @root
      until node.leaf?
        h += 1
        node = node.children[0]
      end
      h
    end

    # =========================================================================
    # VALIDATION — Check all B-tree invariants
    # =========================================================================
    #
    # Invariants:
    #   1. All leaves are at the same depth
    #   2. Every non-root node has ≥ t-1 keys and ≤ 2t-1 keys
    #   3. The root has 0..2t-1 keys (or 0 if empty)
    #   4. Keys within each node are strictly sorted
    #   5. Each internal node with k keys has exactly k+1 children
    #   6. For each key keys[i] in an internal node:
    #        all keys in children[i] < keys[i] < all keys in children[i+1]
    # =========================================================================

    def valid?
      return true if @root.keys.empty?

      _valid_node?(@root, nil, nil, _leaf_depth(@root, 0), 0, is_root: true)
    end

    # =========================================================================
    # PRIVATE IMPLEMENTATION
    # =========================================================================

    private

    # -----------------------------------------------------------------------
    # _search(node, key) → value | nil
    # -----------------------------------------------------------------------
    def _search(node, key)
      found, idx = node.search(key)
      if found
        node.values[idx]
      elsif node.leaf?
        nil
      else
        _search(node.children[idx], key)
      end
    end

    # Check if key exists (handles the nil-value case for [])
    def _key_exists?(node, key)
      found, idx = node.search(key)
      if found
        true
      elsif node.leaf?
        false
      else
        _key_exists?(node.children[idx], key)
      end
    end

    # -----------------------------------------------------------------------
    # _split_child(parent, i) -- Split parent.children[i] (which is full)
    # -----------------------------------------------------------------------
    #
    # Before: parent.children[i] is full (2t-1 keys)
    # After:  median key moves up to parent; full node becomes two nodes
    #
    # Visual (t=2, so 2t-1=3 keys):
    #
    #   parent.keys = [... X ...]
    #   child        = [a  b  c]   (full, 3 keys)
    #
    #   → parent.keys = [... X  b  ...]
    #     left  (child) = [a]
    #     right (new)   = [c]
    #
    # The median key `b` (index t-1) is promoted to the parent.
    # -----------------------------------------------------------------------
    def _split_child(parent, i)
      child = parent.children[i]
      new_node = BTreeNode.new(leaf: child.leaf?)

      # Median index
      mid = @t - 1

      # Move upper half of child's keys/values to new_node
      new_node.keys   = child.keys.slice!(mid + 1, @t - 1)
      new_node.values = child.values.slice!(mid + 1, @t - 1)

      # Move upper half of children (if internal)
      new_node.children = child.children.slice!(@t, @t) unless child.leaf?

      # Promote the median key/value to parent
      median_key   = child.keys.delete_at(mid)
      median_value = child.values.delete_at(mid)

      parent.keys.insert(i, median_key)
      parent.values.insert(i, median_value)
      parent.children.insert(i + 1, new_node)
    end

    # -----------------------------------------------------------------------
    # _insert_non_full(node, key, value)
    # -----------------------------------------------------------------------
    # Insert into a node that is guaranteed to NOT be full (we split on the
    # way down, so this guarantee holds when we reach the leaf).
    def _insert_non_full(node, key, value)
      found, idx = node.search(key)

      if found
        # Key already exists — update the value (upsert behaviour)
        node.values[idx] = value
        return
      end

      if node.leaf?
        # Base case: insert into the leaf (it's not full, so there's room)
        node.keys.insert(idx, key)
        node.values.insert(idx, value)
        @size += 1
      else
        # Recurse into the correct child, splitting if needed
        child = node.children[idx]
        if child.full?(@t)
          _split_child(node, idx)
          # After split, node.keys[idx] is the promoted median.
          # Decide which side to descend into.
          cmp = key <=> node.keys[idx]
          if cmp == 0
            # Key equals the promoted median — update in place
            node.values[idx] = value
            return
          elsif cmp > 0
            idx += 1
          end
        end
        _insert_non_full(node.children[idx], key, value)
      end
    end

    # -----------------------------------------------------------------------
    # _delete(node, key) -- Delete key from the subtree rooted at node.
    #
    # CLRS algorithm: we ensure every node we visit (except root) has ≥ t keys
    # BEFORE descending, so we can safely delete without violating invariants.
    # -----------------------------------------------------------------------
    def _delete(node, key)
      found, idx = node.search(key)

      if found
        # The key is in this node
        if node.leaf?
          # Case 1: Key in leaf — just remove it
          node.keys.delete_at(idx)
          node.values.delete_at(idx)
        else
          # Key in internal node
          left  = node.children[idx]
          right = node.children[idx + 1]

          if left.keys.length >= @t
            # Case 2a: Left child has ≥ t keys — replace with predecessor
            pred_k, pred_v = _rightmost(left)
            node.keys[idx]   = pred_k
            node.values[idx] = pred_v
            _delete(left, pred_k)

          elsif right.keys.length >= @t
            # Case 2b: Right child has ≥ t keys — replace with successor
            succ_k, succ_v = _leftmost(right)
            node.keys[idx]   = succ_k
            node.values[idx] = succ_v
            _delete(right, succ_k)

          else
            # Case 2c: Both children have t-1 keys — merge them
            # After merge: left absorbs key[idx] and all of right's keys
            _merge_children(node, idx)
            _delete(node.children[idx], key)  # descend into merged node
          end
        end
      else
        # Key is NOT in this node — descend into the correct child
        if node.leaf?
          # Key doesn't exist in the tree (already checked with _key_exists?)
          return
        end

        child = node.children[idx]

        if child.keys.length < @t
          # Fill up the child to ensure it has ≥ t keys before descending
          _fill_child(node, idx)

          # After fill, the structure may have changed — recompute idx
          # (merge may have shifted the child we want)
          found2, idx2 = node.search(key)
          if found2
            # The fill promoted/merged a key that equals ours — recurse on node
            _delete(node, key)
            return
          end
          idx = idx2
        end

        _delete(node.children[idx], key)
      end
    end

    # -----------------------------------------------------------------------
    # _fill_child(parent, i) -- Ensure parent.children[i] has ≥ t keys
    #
    # Three strategies:
    #   1. Rotate-Right: borrow from left sibling (children[i-1])
    #   2. Rotate-Left:  borrow from right sibling (children[i+1])
    #   3. Merge: merge children[i] with a sibling
    # -----------------------------------------------------------------------
    def _fill_child(parent, i)
      child = parent.children[i]
      left_sib  = i > 0                          ? parent.children[i - 1] : nil
      right_sib = i < parent.children.length - 1 ? parent.children[i + 1] : nil

      if left_sib && left_sib.keys.length >= @t
        # Rotate-Right: borrow from left sibling
        #
        # Before:
        #   parent: [... L_key ...]      (L_key separates left_sib and child)
        #   left_sib: [a, b, c]
        #   child:    [x]               (sparse)
        #
        # After:
        #   parent: [... c ...]
        #   left_sib: [a, b]
        #   child:    [L_key, x]
        child.keys.unshift(parent.keys[i - 1])
        child.values.unshift(parent.values[i - 1])
        parent.keys[i - 1]   = left_sib.keys.pop
        parent.values[i - 1] = left_sib.values.pop
        child.children.unshift(left_sib.children.pop) unless child.leaf?

      elsif right_sib && right_sib.keys.length >= @t
        # Rotate-Left: borrow from right sibling
        #
        # Before:
        #   parent: [... R_key ...]      (R_key separates child and right_sib)
        #   child:    [x]               (sparse)
        #   right_sib: [a, b, c]
        #
        # After:
        #   parent: [... a ...]
        #   child:    [x, R_key]
        #   right_sib: [b, c]
        child.keys.push(parent.keys[i])
        child.values.push(parent.values[i])
        parent.keys[i]   = right_sib.keys.shift
        parent.values[i] = right_sib.values.shift
        child.children.push(right_sib.children.shift) unless child.leaf?

      elsif left_sib
        # Merge child into left sibling (use left sibling + parent key + child)
        _merge_children(parent, i - 1)

      else
        # Merge right sibling into child
        _merge_children(parent, i)
      end
    end

    # -----------------------------------------------------------------------
    # _merge_children(parent, i)
    #
    # Merge parent.children[i] and parent.children[i+1] into one node.
    # parent.keys[i] (the separator) descends into the merged node.
    #
    # After merge, parent.children[i+1] and parent.keys[i] are removed.
    #
    # Visual (t=2, children each have t-1=1 key):
    #
    #   parent.keys  = [... A B C ...]
    #   left  = [x]      right = [y]
    #
    #   At index i=1 (A=10, B=20, C=30), merging children[1] and children[2]:
    #
    #   merged = [x, B, y]   (left + separator + right)
    #   parent.keys = [... A C ...]    (B removed)
    # -----------------------------------------------------------------------
    def _merge_children(parent, i)
      left  = parent.children[i]
      right = parent.children[i + 1]

      # Pull separator key/value from parent into left
      left.keys.push(parent.keys.delete_at(i))
      left.values.push(parent.values.delete_at(i))

      # Append right's keys, values, children into left
      left.keys.concat(right.keys)
      left.values.concat(right.values)
      left.children.concat(right.children) unless left.leaf?

      # Remove right child from parent
      parent.children.delete_at(i + 1)
    end

    # -----------------------------------------------------------------------
    # _leftmost(node) → [key, value] (the in-order successor)
    # -----------------------------------------------------------------------
    def _leftmost(node)
      node = node.children[0] until node.leaf?
      [node.keys[0], node.values[0]]
    end

    # -----------------------------------------------------------------------
    # _rightmost(node) → [key, value] (the in-order predecessor)
    # -----------------------------------------------------------------------
    def _rightmost(node)
      node = node.children[-1] until node.leaf?
      [node.keys[-1], node.values[-1]]
    end

    # -----------------------------------------------------------------------
    # _inorder(node, result)
    # -----------------------------------------------------------------------
    def _inorder(node, result)
      if node.leaf?
        node.keys.each_with_index { |k, i| result << [k, node.values[i]] }
      else
        node.keys.each_with_index do |k, i|
          _inorder(node.children[i], result)
          result << [k, node.values[i]]
        end
        _inorder(node.children[-1], result)
      end
    end

    # -----------------------------------------------------------------------
    # _range(node, low, high, result)
    # -----------------------------------------------------------------------
    def _range(node, low, high, result)
      if node.leaf?
        node.keys.each_with_index do |k, i|
          result << [k, node.values[i]] if k >= low && k <= high
        end
      else
        node.keys.each_with_index do |k, i|
          _range(node.children[i], low, high, result) if k > low || (k >= low)
          result << [k, node.values[i]] if k >= low && k <= high
        end
        _range(node.children[-1], low, high, result)
      end
    end

    # -----------------------------------------------------------------------
    # _valid_node? -- Recursive validator
    # -----------------------------------------------------------------------
    def _valid_node?(node, min_key, max_key, expected_leaf_depth, depth, is_root: false)
      # Check key count bounds
      unless is_root
        return false if node.keys.length < @t - 1
      end
      return false if node.keys.length > 2 * @t - 1

      # Check sorted order and range constraints
      node.keys.each_with_index do |k, i|
        return false if min_key && k <= min_key
        return false if max_key && k >= max_key
        return false if i > 0 && k <= node.keys[i - 1]
      end

      if node.leaf?
        # All leaves must be at the same depth
        return depth == expected_leaf_depth
      else
        # Internal node: must have exactly keys.length + 1 children
        return false unless node.children.length == node.keys.length + 1

        node.keys.each_with_index do |k, i|
          left_max  = k
          right_min = k
          return false unless _valid_node?(node.children[i], min_key, left_max, expected_leaf_depth, depth + 1)
          min_key = right_min
        end
        _valid_node?(node.children[-1], min_key, max_key, expected_leaf_depth, depth + 1)
      end
    end

    # Compute the depth of the leftmost leaf (all leaves have the same depth).
    def _leaf_depth(node, depth)
      return depth if node.leaf?
      _leaf_depth(node.children[0], depth + 1)
    end
  end
end
