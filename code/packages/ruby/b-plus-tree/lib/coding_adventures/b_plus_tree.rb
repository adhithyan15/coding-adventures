# frozen_string_literal: true

# =============================================================================
# coding_adventures/b_plus_tree.rb -- B+ Tree (B-Plus Tree)
# =============================================================================
#
# WHAT IS A B+ TREE?
# ───────────────────
# A B+ tree is a variant of the B-tree where:
#
#   1. VALUES are stored ONLY in leaf nodes
#   2. Internal nodes hold only SEPARATOR KEYS for routing
#   3. Leaf nodes are linked in a sorted linked list
#
# DATABASE USAGE
# ───────────────
# B+ trees power most relational database indexes (MySQL InnoDB, PostgreSQL,
# SQLite) and file systems (NTFS, HFS+, ext4 with htree). The leaf linked
# list makes range queries like "SELECT * WHERE age BETWEEN 20 AND 30"
# extremely fast — no backtracking needed.
#
# ROUTING CONVENTION
# ───────────────────
# An internal node with keys [k0, k1, ..., km] has m+1 children [c0..cm]:
#
#   c0: all keys <  k0
#   c1: all keys >= k0 and < k1
#   ...
#   cm: all keys >= k(m-1)
#
# To route key q, find i = number of separators <= q, descend to children[i].
#
# LEAF SPLIT RULE (B+ tree specific)
# ────────────────────────────────────
# When a full leaf splits at position mid = t-1:
#
#   Before: [a, b, c, d, e]  (2t-1=5 keys, t=3)
#   After:  left=[a,b]  right=[c,d,e]  separator=c pushed to parent
#
#   Unlike B-tree: c stays in the right leaf! (It's copied, not moved.)
#   Invariant: separator always == minimum key of right child.
#
# INTERNAL SPLIT (same as B-tree)
# ─────────────────────────────────
# Median key moves up to parent, removed from the split node.
# =============================================================================

require_relative "b_plus_tree/node"

module CodingAdventures
  class BPlusTree
    include Enumerable

    attr_reader :t

    def initialize(t: 2)
      raise ArgumentError, "t must be >= 2, got #{t}" if t < 2
      @t    = t
      @root = BPlusLeafNode.new
      @size = 0
    end

    # =========================================================================
    # SEARCH
    # =========================================================================
    #
    # All searches reach a leaf. Even if a key appears as a separator in an
    # internal node, we still descend to the leaf to retrieve the value.
    # =========================================================================

    def search(key)
      leaf = _find_leaf(key)
      _leaf_get(leaf, key)
    end

    def include?(key)
      leaf = _find_leaf(key)
      _leaf_index(leaf, key) >= 0
    end

    alias member? include?

    def [](key)
      leaf = _find_leaf(key)
      idx  = _leaf_index(leaf, key)
      raise KeyError, "key not found: #{key.inspect}" if idx < 0
      leaf.values[idx]
    end

    # =========================================================================
    # INSERT
    # =========================================================================

    def []=(key, value)
      insert(key, value)
    end

    def insert(key, value)
      if @root.full?(@t)
        # Root is full — split it first, then insert into non-full root
        old_root = @root
        new_root = BPlusInternalNode.new
        new_root.children = [old_root]
        @root = new_root
        sep, right = old_root.leaf? ? _split_leaf(old_root) : _split_internal(old_root)
        new_root.keys     = [sep]
        new_root.children = [old_root, right]
      end
      _insert_non_full(@root, key, value)
    end

    # =========================================================================
    # DELETE
    # =========================================================================

    def delete(key)
      return unless include?(key)
      _delete_node(@root, key, nil, 0)
      @root = @root.children[0] if !@root.leaf? && @root.keys.empty?
      @size -= 1
    end

    # =========================================================================
    # SCANS
    # =========================================================================

    # Range scan via leaf linked list — O(log n + k).
    #
    # Step 1: Find the leftmost leaf that could contain keys >= low.
    # Step 2: Walk the leaf linked list collecting keys in [low, high].
    # Step 3: Stop when we exceed high or reach the end.
    def range_scan(low, high)
      result = []
      leaf   = _find_leaf_gte(low)
      while leaf
        leaf.keys.each_with_index do |k, i|
          result << [k, leaf.values[i]] if k >= low && k <= high
        end
        break if leaf.keys.empty? || leaf.keys[-1] >= high
        leaf = leaf.next_leaf
      end
      result
    end

    # Full scan via leaf linked list — O(n). Does NOT visit internal nodes.
    def full_scan
      result = []
      leaf   = _leftmost_leaf
      while leaf
        leaf.keys.each_with_index { |k, i| result << [k, leaf.values[i]] }
        leaf = leaf.next_leaf
      end
      result
    end

    def each(&block)
      full_scan.each(&block)
    end

    # =========================================================================
    # ACCESSORS
    # =========================================================================

    def min_key
      raise "Tree is empty" if empty?
      _leftmost_leaf.keys[0]
    end

    def max_key
      raise "Tree is empty" if empty?
      _rightmost_leaf.keys[-1]
    end

    def size
      @size
    end

    def empty?
      @size == 0
    end

    def height
      h, node = 0, @root
      until node.leaf?
        h += 1
        node = node.children[0]
      end
      h
    end

    # =========================================================================
    # VALIDATION
    # =========================================================================

    def valid?
      return true if @root.leaf? && @root.keys.empty?
      ed = _leaf_depth(@root, 0)
      return false unless _valid_node?(@root, ed, 0, is_root: true)
      _valid_leaf_chain?
    end

    # =========================================================================
    # PRIVATE
    # =========================================================================
    private

    # -----------------------------------------------------------------------
    # Routing: how many separators are <= key? That gives child index.
    # -----------------------------------------------------------------------
    def _child_idx(node, key)
      # Binary search for rightmost separator <= key
      lo = 0
      hi = node.keys.length - 1
      idx = 0
      while lo <= hi
        mid = (lo + hi) / 2
        if node.keys[mid] <= key
          idx = mid + 1
          lo  = mid + 1
        else
          hi = mid - 1
        end
      end
      idx
    end

    # -----------------------------------------------------------------------
    # Leaf search helpers
    # -----------------------------------------------------------------------
    def _leaf_index(leaf, key)
      lo = 0
      hi = leaf.keys.length - 1
      while lo <= hi
        mid = (lo + hi) / 2
        cmp = key <=> leaf.keys[mid]
        return mid if cmp == 0
        cmp < 0 ? hi = mid - 1 : lo = mid + 1
      end
      -1
    end

    def _leaf_get(leaf, key)
      idx = _leaf_index(leaf, key)
      idx >= 0 ? leaf.values[idx] : nil
    end

    # Find the leaf position for key (sorted insert position)
    def _leaf_insert_pos(leaf, key)
      lo = 0
      hi = leaf.keys.length - 1
      while lo <= hi
        mid = (lo + hi) / 2
        cmp = key <=> leaf.keys[mid]
        return [true, mid] if cmp == 0
        cmp < 0 ? hi = mid - 1 : lo = mid + 1
      end
      [false, lo]
    end

    # -----------------------------------------------------------------------
    # Navigation
    # -----------------------------------------------------------------------
    def _find_leaf(key)
      node = @root
      until node.leaf?
        node = node.children[_child_idx(node, key)]
      end
      node
    end

    def _find_leaf_gte(low)
      node = @root
      until node.leaf?
        # Go left of the first separator > low (or rightmost child if all <= low)
        idx = 0
        node.keys.each_with_index do |sep, i|
          break if low < sep
          idx = i + 1
        end
        node = node.children[idx]
      end
      node
    end

    def _leftmost_leaf
      node = @root
      node = node.children[0] until node.leaf?
      node
    end

    def _rightmost_leaf
      node = @root
      node = node.children[-1] until node.leaf?
      node
    end

    # -----------------------------------------------------------------------
    # _insert_non_full(node, key, value)
    #
    # Insert into a node guaranteed to NOT be full (we split proactively on
    # the way down, so the node always has room when we reach the leaf).
    # -----------------------------------------------------------------------
    def _insert_non_full(node, key, value)
      if node.leaf?
        found, pos = _leaf_insert_pos(node, key)
        if found
          node.values[pos] = value  # update existing
          return
        end
        node.keys.insert(pos, key)
        node.values.insert(pos, value)
        @size += 1
      else
        ci    = _child_idx(node, key)
        child = node.children[ci]
        if child.full?(@t)
          # Split child before descending
          sep, right = child.leaf? ? _split_leaf(child) : _split_internal(child)
          node.keys.insert(ci, sep)
          node.children.insert(ci + 1, right)
          # Recompute ci after split
          ci += 1 if key >= node.keys[ci]
        end
        _insert_non_full(node.children[ci], key, value)
      end
    end

    # Leaf split: copy separator, keep it in right leaf
    def _split_leaf(leaf)
      mid   = @t - 1
      right = BPlusLeafNode.new

      right.keys    = leaf.keys.slice!(mid, leaf.keys.length)
      right.values  = leaf.values.slice!(mid, leaf.values.length)
      right.next_leaf  = leaf.next_leaf
      leaf.next_leaf   = right

      [right.keys[0], right]
    end

    # Internal split: move separator up (same as B-tree)
    def _split_internal(node)
      mid   = @t - 1
      right = BPlusInternalNode.new

      right.keys     = node.keys.slice!(mid + 1, node.keys.length)
      right.children = node.children.slice!(mid + 1, node.children.length)
      sep            = node.keys.delete_at(mid)

      [sep, right]
    end

    # -----------------------------------------------------------------------
    # DELETE recursive
    #
    # Descends to the leaf, removes the key, then fixes underflow and updates
    # separator keys on the way back up.
    #
    # After deleting from a child, we:
    #   1. Fix underflow (borrow or merge)
    #   2. Update separator[ci] = min key of children[ci+1] if needed
    # -----------------------------------------------------------------------
    def _delete_node(node, key, parent, ci)
      if node.leaf?
        idx = _leaf_index(node, key)
        return if idx < 0
        node.keys.delete_at(idx)
        node.values.delete_at(idx)
      else
        child_ci = _child_idx(node, key)
        child    = node.children[child_ci]
        _delete_node(child, key, node, child_ci)

        # Fix underflow after recursive call
        _fix_underflow(node, child_ci) if child.keys.length < @t - 1

        # Update separator keys: in B+ tree, separator[i] == min key of children[i+1]
        # We need to update all separators that may have been affected.
        _refresh_separators(node)
      end
    end

    # Refresh separator keys in an internal node to match the actual
    # minimum keys of each right child.
    def _refresh_separators(node)
      return if node.leaf?
      node.keys.each_with_index do |_sep, i|
        cmin = _min_key_of(node.children[i + 1])
        node.keys[i] = cmin if cmin
      end
    end

    # -----------------------------------------------------------------------
    # _fix_underflow(parent, ci)
    #
    # parent.children[ci] has too few keys. Fix by borrow or merge.
    # -----------------------------------------------------------------------
    def _fix_underflow(parent, ci)
      node      = parent.children[ci]
      left_sib  = ci > 0                             ? parent.children[ci - 1] : nil
      right_sib = ci < parent.children.length - 1    ? parent.children[ci + 1] : nil

      if right_sib && right_sib.keys.length >= @t
        # Borrow from right sibling
        if node.leaf?
          # Rotate-left for leaves: move right_sib's first key/value to end of node
          node.keys.push(right_sib.keys.shift)
          node.values.push(right_sib.values.shift)
          # separator[ci] = new min of right_sib
          parent.keys[ci] = right_sib.keys[0]
        else
          # Rotate-left for internal nodes:
          # Move right_sib.children[0] to end of node.
          # The separator between node and the borrowed child is min of right_sib.children[0].
          borrowed_child = right_sib.children.shift
          sep_for_node   = _min_key_of(borrowed_child)
          node.keys.push(sep_for_node)
          node.children.push(borrowed_child)
          # Remove the first key from right_sib (it's now represented by the new separator)
          right_sib.keys.shift
          # Update parent separator: sep[ci] = new min of right_sib = min of right_sib.children[0]
          parent.keys[ci] = _min_key_of(right_sib.children[0])
        end

      elsif left_sib && left_sib.keys.length >= @t
        # Borrow from left sibling
        if node.leaf?
          # Rotate-right for leaves: move left_sib's last key/value to front of node
          node.keys.unshift(left_sib.keys.pop)
          node.values.unshift(left_sib.values.pop)
          parent.keys[ci - 1] = node.keys[0]
        else
          # Rotate-right for internal nodes:
          # Move left_sib.children[-1] to front of node.
          # The separator to add in node = min of node's current first child (which becomes children[1]).
          borrowed_child = left_sib.children.pop
          sep_for_node   = _min_key_of(node.children[0])
          node.keys.unshift(sep_for_node)
          node.children.unshift(borrowed_child)
          # Remove last key from left_sib
          left_sib.keys.pop
          # Update parent separator: sep[ci-1] = min of node = min of borrowed_child
          parent.keys[ci - 1] = _min_key_of(borrowed_child)
        end

      elsif right_sib
        # Merge node with right sibling
        if node.leaf?
          node.keys.concat(right_sib.keys)
          node.values.concat(right_sib.values)
          node.next_leaf = right_sib.next_leaf
        else
          # B+ tree internal merge: the separator from parent goes between the two halves.
          # But in B+ tree, separators equal min of right subtree. The right subtree's
          # children are right_sib.children. The correct separator is min of right_sib.children[0].
          # However, we use right_sib.keys[0] if right_sib has keys, else _min_key_of(right_sib.children[0]).
          sep = right_sib.keys.empty? ? _min_key_of(right_sib.children[0]) : _min_key_of(right_sib.children[0])
          node.keys.push(sep)
          node.keys.concat(right_sib.keys)
          node.children.concat(right_sib.children)
        end
        parent.keys.delete_at(ci)
        parent.children.delete_at(ci + 1)

      elsif left_sib
        # Merge left sibling with node
        if node.leaf?
          left_sib.keys.concat(node.keys)
          left_sib.values.concat(node.values)
          left_sib.next_leaf = node.next_leaf
        else
          # In B+ tree, the separator pulled from parent is stale after merge.
          # Use the actual min key of node's first child as the separator.
          sep = _min_key_of(node.children[0])
          left_sib.keys.push(sep)
          left_sib.keys.concat(node.keys)
          left_sib.children.concat(node.children)
        end
        parent.keys.delete_at(ci - 1)
        parent.children.delete_at(ci)
      end
    end

    # -----------------------------------------------------------------------
    # Validation helpers
    # -----------------------------------------------------------------------

    def _valid_node?(node, expected_depth, depth, is_root: false)
      if node.leaf?
        return false if !is_root && node.keys.length < @t - 1
        return false if node.keys.length > 2 * @t - 1
        (1...node.keys.length).each { |i| return false if node.keys[i] <= node.keys[i - 1] }
        depth == expected_depth
      else
        return false if !is_root && node.keys.length < @t - 1
        return false if node.keys.length > 2 * @t - 1
        return false unless node.children.length == node.keys.length + 1

        (1...node.keys.length).each { |i| return false if node.keys[i] <= node.keys[i - 1] }

        node.children.each do |child|
          return false unless _valid_node?(child, expected_depth, depth + 1)
        end

        # B+ invariant: separator[i] == min key of children[i+1]
        node.keys.each_with_index do |sep, i|
          cmin = _min_key_of(node.children[i + 1])
          return false if cmin != sep
        end

        true
      end
    end

    def _min_key_of(node)
      return node.keys[0] if node.leaf?
      _min_key_of(node.children[0])
    end

    def _valid_leaf_chain?
      leaf     = _leftmost_leaf
      prev_max = nil
      count    = 0
      while leaf
        return false if prev_max && !leaf.keys.empty? && leaf.keys[0] <= prev_max
        prev_max = leaf.keys[-1] unless leaf.keys.empty?
        count   += leaf.keys.length
        leaf     = leaf.next_leaf
      end
      count == @size
    end

    def _leaf_depth(node, depth)
      return depth if node.leaf?
      _leaf_depth(node.children[0], depth + 1)
    end
  end
end
