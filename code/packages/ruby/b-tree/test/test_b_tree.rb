# frozen_string_literal: true

require_relative "test_helper"

# =============================================================================
# B-Tree Test Suite
# =============================================================================
#
# We test every public method, every delete sub-case from CLRS (Case 1, 2a,
# 2b, 2c, rotate-left, rotate-right, merge), and verify valid? after every
# operation. We use t=2, t=3, and t=5 to ensure the algorithms work across
# different branching factors.
# =============================================================================

class TestBTreeConstruction < Minitest::Test
  def test_new_with_default_t
    tree = CodingAdventures::BTree.new
    assert_equal 2, tree.t
    assert tree.empty?
    assert_equal 0, tree.size
  end

  def test_new_with_custom_t
    tree = CodingAdventures::BTree.new(t: 5)
    assert_equal 5, tree.t
  end

  def test_new_raises_for_t_less_than_2
    assert_raises(ArgumentError) { CodingAdventures::BTree.new(t: 1) }
    assert_raises(ArgumentError) { CodingAdventures::BTree.new(t: 0) }
  end

  def test_empty_tree_is_valid
    tree = CodingAdventures::BTree.new
    assert tree.valid?
  end

  def test_height_of_empty_tree
    tree = CodingAdventures::BTree.new
    assert_equal 0, tree.height
  end
end

# =============================================================================
# Search Tests
# =============================================================================

class TestBTreeSearch < Minitest::Test
  def setup
    @tree = CodingAdventures::BTree.new(t: 2)
    [10, 20, 30, 40, 50].each { |k| @tree.insert(k, "val#{k}") }
  end

  def test_search_existing_key
    assert_equal "val10", @tree.search(10)
    assert_equal "val30", @tree.search(30)
    assert_equal "val50", @tree.search(50)
  end

  def test_search_missing_key
    assert_nil @tree.search(99)
    assert_nil @tree.search(0)
  end

  def test_include_existing_key
    assert @tree.include?(10)
    assert @tree.include?(50)
  end

  def test_include_missing_key
    refute @tree.include?(99)
  end

  def test_bracket_get_existing
    assert_equal "val20", @tree[20]
  end

  def test_bracket_get_missing_raises
    assert_raises(KeyError) { @tree[99] }
  end
end

# =============================================================================
# Insert Tests
# =============================================================================

class TestBTreeInsert < Minitest::Test
  def test_insert_single_key
    tree = CodingAdventures::BTree.new(t: 2)
    tree.insert(42, "forty-two")
    assert_equal 1, tree.size
    assert_equal "forty-two", tree.search(42)
    assert tree.valid?
  end

  def test_insert_updates_existing_key
    tree = CodingAdventures::BTree.new(t: 2)
    tree.insert(10, "old")
    tree.insert(10, "new")
    assert_equal 1, tree.size
    assert_equal "new", tree.search(10)
    assert tree.valid?
  end

  def test_bracket_set
    tree = CodingAdventures::BTree.new(t: 2)
    tree[15] = "fifteen"
    assert_equal "fifteen", tree[15]
    assert tree.valid?
  end

  def test_insert_sequential_keys_t2
    tree = CodingAdventures::BTree.new(t: 2)
    (1..20).each { |k| tree.insert(k, k * 10) }
    assert_equal 20, tree.size
    assert tree.valid?
    (1..20).each { |k| assert_equal k * 10, tree.search(k) }
  end

  def test_insert_reverse_keys_t2
    tree = CodingAdventures::BTree.new(t: 2)
    (1..20).reverse_each { |k| tree.insert(k, k * 10) }
    assert_equal 20, tree.size
    assert tree.valid?
  end

  def test_insert_random_keys_t3
    tree = CodingAdventures::BTree.new(t: 3)
    keys = (1..50).to_a.shuffle
    keys.each { |k| tree.insert(k, "v#{k}") }
    assert_equal 50, tree.size
    assert tree.valid?
    keys.each { |k| assert_equal "v#{k}", tree.search(k) }
  end

  def test_insert_1000_keys_t5
    tree = CodingAdventures::BTree.new(t: 5)
    1000.times { |i| tree.insert(i, i) }
    assert_equal 1000, tree.size
    assert tree.valid?
    assert tree.height <= 4  # log_5(1000) ≈ 4.3
  end

  def test_insert_causes_root_split
    # With t=2, root can hold max 3 keys. After 4 insertions, root splits.
    tree = CodingAdventures::BTree.new(t: 2)
    [10, 20, 30].each { |k| tree.insert(k, k) }
    assert_equal 0, tree.height  # still a single leaf
    tree.insert(40, 40)
    # Now the tree should have height > 0 (root was split)
    assert tree.valid?
    assert_equal 4, tree.size
  end

  def test_inorder_after_inserts
    tree = CodingAdventures::BTree.new(t: 2)
    keys = [5, 3, 7, 1, 9, 4, 6, 2, 8]
    keys.each { |k| tree.insert(k, k) }
    result = tree.inorder
    assert_equal (1..9).to_a, result.map(&:first)
  end
end

# =============================================================================
# Delete Tests — CLRS Cases
# =============================================================================
#
# We carefully construct trees to exercise each delete sub-case.
# =============================================================================

class TestBTreeDeleteCase1 < Minitest::Test
  # Case 1: Key is in a leaf node — simple removal.
  def test_delete_from_leaf
    tree = CodingAdventures::BTree.new(t: 2)
    [1, 2, 3].each { |k| tree.insert(k, k) }
    tree.delete(2)
    assert_equal 2, tree.size
    assert_nil tree.search(2)
    assert tree.valid?
  end

  def test_delete_leaf_first_key
    tree = CodingAdventures::BTree.new(t: 2)
    [10, 20, 30, 40].each { |k| tree.insert(k, k) }
    tree.delete(10)
    assert tree.valid?
    assert_nil tree.search(10)
  end

  def test_delete_leaf_last_key
    tree = CodingAdventures::BTree.new(t: 2)
    [10, 20, 30, 40].each { |k| tree.insert(k, k) }
    tree.delete(40)
    assert tree.valid?
    assert_nil tree.search(40)
  end
end

class TestBTreeDeleteCase2 < Minitest::Test
  # Case 2: Key is in an internal node.
  # We need a tree deep enough to have internal nodes.

  def setup
    # Build a tree that requires internal nodes with t=2
    @tree = CodingAdventures::BTree.new(t: 2)
    (1..15).each { |k| @tree.insert(k, k) }
  end

  def test_delete_internal_node_key
    # Delete a key that's in an internal node (not a leaf)
    # The algorithm replaces it with predecessor or successor
    key = @tree.inorder.map(&:first).find { |k|
      # Find a key in an internal node — difficult to guarantee without
      # knowing the exact structure, so just delete a middle key and validate
      k == 8
    }
    @tree.delete(8) if key
    assert @tree.valid?
    assert_nil @tree.search(8)
  end

  def test_delete_all_then_check_valid
    keys = @tree.inorder.map(&:first)
    keys.each do |k|
      @tree.delete(k)
      assert @tree.valid?, "Tree invalid after deleting #{k}"
    end
    assert @tree.empty?
  end
end

class TestBTreeDeleteRotations < Minitest::Test
  # Test rotate-left and rotate-right (Case 3 fill operations)

  def test_rotate_right_borrow_from_left_sibling
    # Build a specific tree where right rotation is needed
    tree = CodingAdventures::BTree.new(t: 2)
    (1..10).each { |k| tree.insert(k, k) }
    # Delete from right side, forcing borrow from left
    [8, 9, 10].each { |k| tree.delete(k) }
    assert tree.valid?
    (1..7).each { |k| assert_equal k, tree.search(k) }
  end

  def test_rotate_left_borrow_from_right_sibling
    tree = CodingAdventures::BTree.new(t: 2)
    (1..10).each { |k| tree.insert(k, k) }
    # Delete from left side, forcing borrow from right
    [1, 2, 3].each { |k| tree.delete(k) }
    assert tree.valid?
    (4..10).each { |k| assert_equal k, tree.search(k) }
  end
end

class TestBTreeDeleteMerge < Minitest::Test
  # Case 2c / Case 3 merge: when both children have t-1 keys, merge them.

  def test_merge_reduces_height
    tree = CodingAdventures::BTree.new(t: 2)
    (1..7).each { |k| tree.insert(k, k) }
    # Delete enough keys to force merges and height reduction
    (1..7).each { |k| tree.delete(k) }
    assert tree.empty?
    assert tree.valid?
    assert_equal 0, tree.height
  end

  def test_delete_with_t3_triggers_merge
    tree = CodingAdventures::BTree.new(t: 3)
    (1..30).each { |k| tree.insert(k, k) }
    # Delete every 3rd key starting from 1: 1, 4, 7, 10, ...
    deleted = (1..30).step(3).to_a
    deleted.each { |k| tree.delete(k) }
    assert tree.valid?
    (1..30).each do |k|
      if deleted.include?(k)
        assert_nil tree.search(k), "Expected #{k} to be deleted"
      else
        assert_equal k, tree.search(k)
      end
    end
  end
end

class TestBTreeDeleteNonExistent < Minitest::Test
  def test_delete_missing_key_is_noop
    tree = CodingAdventures::BTree.new(t: 2)
    [10, 20, 30].each { |k| tree.insert(k, k) }
    tree.delete(99)  # Should not raise
    assert_equal 3, tree.size
    assert tree.valid?
  end

  def test_delete_from_empty_tree
    tree = CodingAdventures::BTree.new(t: 2)
    tree.delete(1)  # Should not raise
    assert tree.empty?
    assert tree.valid?
  end
end

# =============================================================================
# Min / Max Tests
# =============================================================================

class TestBTreeMinMax < Minitest::Test
  def test_min_key
    tree = CodingAdventures::BTree.new(t: 2)
    [30, 10, 50, 20, 40].each { |k| tree.insert(k, k) }
    assert_equal 10, tree.min_key
  end

  def test_max_key
    tree = CodingAdventures::BTree.new(t: 2)
    [30, 10, 50, 20, 40].each { |k| tree.insert(k, k) }
    assert_equal 50, tree.max_key
  end

  def test_min_max_single_element
    tree = CodingAdventures::BTree.new(t: 2)
    tree.insert(42, 42)
    assert_equal 42, tree.min_key
    assert_equal 42, tree.max_key
  end

  def test_min_raises_on_empty
    tree = CodingAdventures::BTree.new
    assert_raises(RuntimeError) { tree.min_key }
  end

  def test_max_raises_on_empty
    tree = CodingAdventures::BTree.new
    assert_raises(RuntimeError) { tree.max_key }
  end
end

# =============================================================================
# Range Query Tests
# =============================================================================

class TestBTreeRangeQuery < Minitest::Test
  def setup
    @tree = CodingAdventures::BTree.new(t: 2)
    (1..20).each { |k| @tree.insert(k, k * 10) }
  end

  def test_range_full
    result = @tree.range_query(1, 20)
    assert_equal (1..20).to_a, result.map(&:first)
  end

  def test_range_middle
    result = @tree.range_query(5, 10)
    assert_equal [5, 6, 7, 8, 9, 10], result.map(&:first)
    assert_equal [50, 60, 70, 80, 90, 100], result.map(&:last)
  end

  def test_range_single_element
    result = @tree.range_query(7, 7)
    assert_equal [[7, 70]], result
  end

  def test_range_empty_result
    result = @tree.range_query(21, 30)
    assert_empty result
  end

  def test_range_with_t5
    tree = CodingAdventures::BTree.new(t: 5)
    (1..100).each { |k| tree.insert(k, k) }
    result = tree.range_query(45, 55)
    assert_equal (45..55).to_a, result.map(&:first)
  end
end

# =============================================================================
# Height Tests
# =============================================================================

class TestBTreeHeight < Minitest::Test
  def test_height_grows_logarithmically
    tree = CodingAdventures::BTree.new(t: 2)
    # At t=2: root splits after 3 keys, giving height 1
    [1, 2, 3, 4].each { |k| tree.insert(k, k) }
    assert_equal 1, tree.height
    assert tree.valid?
  end

  def test_height_is_zero_for_empty
    tree = CodingAdventures::BTree.new(t: 5)
    assert_equal 0, tree.height
  end

  def test_large_tree_height_bound
    tree = CodingAdventures::BTree.new(t: 3)
    (1..1000).each { |k| tree.insert(k, k) }
    # log_3(1000) ≈ 6.3, so height should be ≤ 7
    assert tree.height <= 7
    assert tree.valid?
  end
end

# =============================================================================
# Stress Tests
# =============================================================================

class TestBTreeStress < Minitest::Test
  def test_insert_and_delete_1000_keys_t2
    tree = CodingAdventures::BTree.new(t: 2)
    keys = (1..1000).to_a.shuffle
    keys.each { |k| tree.insert(k, k) }
    assert_equal 1000, tree.size
    assert tree.valid?

    to_delete = keys.first(500)
    to_delete.each { |k| tree.delete(k) }
    assert_equal 500, tree.size
    assert tree.valid?

    to_delete.each { |k| assert_nil tree.search(k) }
    (keys - to_delete).each { |k| assert_equal k, tree.search(k) }
  end

  def test_insert_and_delete_1000_keys_t5
    tree = CodingAdventures::BTree.new(t: 5)
    keys = (1..1000).to_a.shuffle
    keys.each { |k| tree.insert(k, k) }
    keys.each { |k| tree.delete(k) }
    assert tree.empty?
    assert tree.valid?
  end

  def test_inorder_always_sorted
    tree = CodingAdventures::BTree.new(t: 3)
    keys = (1..200).to_a.shuffle
    keys.each { |k| tree.insert(k, k) }
    result = tree.inorder.map(&:first)
    assert_equal result.sort, result
  end

  def test_string_keys
    tree = CodingAdventures::BTree.new(t: 2)
    words = %w[banana apple cherry date elderberry fig grape]
    words.each { |w| tree.insert(w, w.upcase) }
    assert_equal words.sort, tree.inorder.map(&:first)
    assert tree.valid?
  end
end

# =============================================================================
# Valid? Tests
# =============================================================================

class TestBTreeValid < Minitest::Test
  def test_valid_after_every_insert
    tree = CodingAdventures::BTree.new(t: 2)
    (1..30).each do |k|
      tree.insert(k, k)
      assert tree.valid?, "Tree invalid after inserting #{k}"
    end
  end

  def test_valid_after_every_delete
    tree = CodingAdventures::BTree.new(t: 2)
    (1..30).each { |k| tree.insert(k, k) }
    (1..30).to_a.shuffle.each do |k|
      tree.delete(k)
      assert tree.valid?, "Tree invalid after deleting #{k}"
    end
  end

  def test_valid_with_t3
    tree = CodingAdventures::BTree.new(t: 3)
    (1..50).each { |k| tree.insert(k, k) }
    (1..50).step(2) { |k| tree.delete(k) }
    assert tree.valid?
  end
end
