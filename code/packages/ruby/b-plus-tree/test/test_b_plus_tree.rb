# frozen_string_literal: true

require_relative "test_helper"

# =============================================================================
# B+ Tree Test Suite
# =============================================================================
#
# We test:
#   - All public methods including Enumerable
#   - Leaf linked list integrity after every operation
#   - range_scan using the linked list (no backtracking)
#   - full_scan efficiency
#   - Delete borrow and merge cases
#   - valid? after every operation
#   - t=2, t=3, t=5
#   - 1000+ key stress tests
# =============================================================================

class TestBPlusTreeConstruction < Minitest::Test
  def test_new_default_t
    tree = CodingAdventures::BPlusTree.new
    assert_equal 2, tree.t
    assert tree.empty?
    assert_equal 0, tree.size
  end

  def test_new_custom_t
    tree = CodingAdventures::BPlusTree.new(t: 5)
    assert_equal 5, tree.t
  end

  def test_new_raises_for_small_t
    assert_raises(ArgumentError) { CodingAdventures::BPlusTree.new(t: 1) }
  end

  def test_empty_tree_valid
    tree = CodingAdventures::BPlusTree.new
    assert tree.valid?
  end

  def test_height_empty
    tree = CodingAdventures::BPlusTree.new
    assert_equal 0, tree.height
  end
end

# =============================================================================
# Search Tests
# =============================================================================

class TestBPlusTreeSearch < Minitest::Test
  def setup
    @tree = CodingAdventures::BPlusTree.new(t: 2)
    [10, 20, 30, 40, 50].each { |k| @tree.insert(k, "v#{k}") }
  end

  def test_search_existing
    assert_equal "v10", @tree.search(10)
    assert_equal "v50", @tree.search(50)
  end

  def test_search_missing
    assert_nil @tree.search(99)
  end

  def test_include
    assert @tree.include?(30)
    refute @tree.include?(31)
  end

  def test_bracket_get
    assert_equal "v20", @tree[20]
  end

  def test_bracket_get_missing_raises
    assert_raises(KeyError) { @tree[999] }
  end
end

# =============================================================================
# Insert Tests
# =============================================================================

class TestBPlusTreeInsert < Minitest::Test
  def test_insert_single
    tree = CodingAdventures::BPlusTree.new(t: 2)
    tree.insert(1, "one")
    assert_equal 1, tree.size
    assert_equal "one", tree.search(1)
    assert tree.valid?
  end

  def test_insert_updates_existing
    tree = CodingAdventures::BPlusTree.new(t: 2)
    tree.insert(5, "old")
    tree.insert(5, "new")
    assert_equal 1, tree.size
    assert_equal "new", tree.search(5)
    assert tree.valid?
  end

  def test_bracket_set
    tree = CodingAdventures::BPlusTree.new(t: 2)
    tree[42] = "forty-two"
    assert_equal "forty-two", tree[42]
  end

  def test_insert_sequential_t2
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..20).each { |k| tree.insert(k, k) }
    assert_equal 20, tree.size
    assert tree.valid?
  end

  def test_insert_reverse_t2
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..20).reverse_each { |k| tree.insert(k, k) }
    assert_equal 20, tree.size
    assert tree.valid?
  end

  def test_insert_triggers_root_split
    tree = CodingAdventures::BPlusTree.new(t: 2)
    [1, 2, 3].each { |k| tree.insert(k, k) }
    assert_equal 0, tree.height  # still leaf root
    tree.insert(4, 4)
    assert tree.valid?
    assert_equal 4, tree.size
  end

  def test_full_scan_after_inserts
    tree = CodingAdventures::BPlusTree.new(t: 2)
    keys = [5, 3, 7, 1, 9, 4, 6, 2, 8]
    keys.each { |k| tree.insert(k, k * 10) }
    result = tree.full_scan
    assert_equal (1..9).to_a, result.map(&:first)
    assert_equal [10, 20, 30, 40, 50, 60, 70, 80, 90], result.map(&:last)
  end
end

# =============================================================================
# Leaf Linked List Tests
# =============================================================================

class TestBPlusTreeLeafList < Minitest::Test
  # The leaf linked list is the key structural property of B+ trees.
  # We verify it's correctly maintained after inserts and deletes.

  def test_leaf_list_sorted_after_inserts
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..15).to_a.shuffle.each { |k| tree.insert(k, k) }
    # full_scan walks the linked list — if it's sorted, the list is correct
    result = tree.full_scan.map(&:first)
    assert_equal result.sort, result
    assert tree.valid?
  end

  def test_leaf_list_sorted_after_deletes
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..15).each { |k| tree.insert(k, k) }
    [3, 7, 11].each { |k| tree.delete(k) }
    result = tree.full_scan.map(&:first)
    assert_equal result.sort, result
    assert tree.valid?
  end

  def test_full_scan_contains_all_keys
    tree = CodingAdventures::BPlusTree.new(t: 3)
    expected = (1..30).to_a
    expected.shuffle.each { |k| tree.insert(k, k) }
    result = tree.full_scan.map(&:first)
    assert_equal expected.sort, result
  end

  def test_linked_list_no_gaps_after_many_deletes
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..20).each { |k| tree.insert(k, k) }
    (1..20).step(2) { |k| tree.delete(k) }
    remaining = (2..20).step(2).to_a
    assert_equal remaining, tree.full_scan.map(&:first)
    assert tree.valid?
  end
end

# =============================================================================
# Range Scan Tests
# =============================================================================

class TestBPlusTreeRangeScan < Minitest::Test
  def setup
    @tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..30).each { |k| @tree.insert(k, k * 100) }
  end

  def test_range_scan_full
    result = @tree.range_scan(1, 30)
    assert_equal (1..30).to_a, result.map(&:first)
  end

  def test_range_scan_middle
    result = @tree.range_scan(10, 20)
    assert_equal (10..20).to_a, result.map(&:first)
  end

  def test_range_scan_single
    result = @tree.range_scan(15, 15)
    assert_equal [[15, 1500]], result
  end

  def test_range_scan_empty_result
    result = @tree.range_scan(100, 200)
    assert_empty result
  end

  def test_range_scan_t3
    tree = CodingAdventures::BPlusTree.new(t: 3)
    (1..100).each { |k| tree.insert(k, k) }
    result = tree.range_scan(45, 55)
    assert_equal (45..55).to_a, result.map(&:first)
  end

  def test_range_scan_t5
    tree = CodingAdventures::BPlusTree.new(t: 5)
    (1..1000).each { |k| tree.insert(k, k) }
    result = tree.range_scan(500, 600)
    assert_equal (500..600).to_a, result.map(&:first)
    assert tree.valid?
  end
end

# =============================================================================
# Delete Tests
# =============================================================================

class TestBPlusTreeDelete < Minitest::Test
  def test_delete_existing
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..10).each { |k| tree.insert(k, k) }
    tree.delete(5)
    assert_nil tree.search(5)
    assert_equal 9, tree.size
    assert tree.valid?
  end

  def test_delete_missing_is_noop
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..5).each { |k| tree.insert(k, k) }
    tree.delete(99)
    assert_equal 5, tree.size
    assert tree.valid?
  end

  def test_delete_all_keys
    tree = CodingAdventures::BPlusTree.new(t: 2)
    keys = (1..15).to_a
    keys.each { |k| tree.insert(k, k) }
    keys.shuffle.each { |k| tree.delete(k) }
    assert tree.empty?
    assert tree.valid?
  end

  def test_delete_first_key
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..10).each { |k| tree.insert(k, k) }
    tree.delete(1)
    assert_nil tree.search(1)
    assert_equal 2, tree.min_key
    assert tree.valid?
  end

  def test_delete_last_key
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..10).each { |k| tree.insert(k, k) }
    tree.delete(10)
    assert_nil tree.search(10)
    assert_equal 9, tree.max_key
    assert tree.valid?
  end

  def test_delete_borrow_from_left_sibling
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..10).each { |k| tree.insert(k, k) }
    # Delete from right side to force borrowing from left
    [8, 9, 10].each { |k| tree.delete(k) }
    assert tree.valid?
    (1..7).each { |k| assert_equal k, tree.search(k) }
  end

  def test_delete_borrow_from_right_sibling
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..10).each { |k| tree.insert(k, k) }
    # Delete from left side to force borrowing from right
    [1, 2, 3].each { |k| tree.delete(k) }
    assert tree.valid?
    (4..10).each { |k| assert_equal k, tree.search(k) }
  end

  def test_delete_causes_merge
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..7).each { |k| tree.insert(k, k) }
    (1..7).each { |k| tree.delete(k) }
    assert tree.empty?
    assert tree.valid?
  end

  def test_delete_valid_after_every_step
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..30).each { |k| tree.insert(k, k) }
    (1..30).to_a.shuffle.each do |k|
      tree.delete(k)
      assert tree.valid?, "Tree invalid after deleting #{k}"
    end
  end
end

# =============================================================================
# Min / Max Tests
# =============================================================================

class TestBPlusTreeMinMax < Minitest::Test
  def test_min_key
    tree = CodingAdventures::BPlusTree.new(t: 2)
    [30, 10, 50, 20, 40].each { |k| tree.insert(k, k) }
    assert_equal 10, tree.min_key
  end

  def test_max_key
    tree = CodingAdventures::BPlusTree.new(t: 2)
    [30, 10, 50, 20, 40].each { |k| tree.insert(k, k) }
    assert_equal 50, tree.max_key
  end

  def test_min_raises_empty
    tree = CodingAdventures::BPlusTree.new
    assert_raises(RuntimeError) { tree.min_key }
  end

  def test_max_raises_empty
    tree = CodingAdventures::BPlusTree.new
    assert_raises(RuntimeError) { tree.max_key }
  end
end

# =============================================================================
# Enumerable Tests
# =============================================================================

class TestBPlusTreeEnumerable < Minitest::Test
  def setup
    @tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..10).each { |k| @tree.insert(k, k * 10) }
  end

  def test_each_yields_sorted_pairs
    pairs = []
    @tree.each { |k, v| pairs << [k, v] }
    assert_equal (1..10).to_a, pairs.map(&:first)
    assert_equal (1..10).map { |k| k * 10 }, pairs.map(&:last)
  end

  def test_map
    result = @tree.map { |k, _v| k * 2 }
    assert_equal (1..10).map { |k| k * 2 }, result
  end

  def test_select
    evens = @tree.select { |k, _v| k.even? }.map(&:first)
    assert_equal [2, 4, 6, 8, 10], evens
  end

  def test_min_by
    pair = @tree.min_by { |k, _v| k }
    assert_equal [1, 10], pair
  end
end

# =============================================================================
# Stress Tests
# =============================================================================

class TestBPlusTreeStress < Minitest::Test
  def test_insert_delete_1000_keys_t2
    tree = CodingAdventures::BPlusTree.new(t: 2)
    keys = (1..1000).to_a.shuffle
    keys.each { |k| tree.insert(k, k) }
    assert_equal 1000, tree.size
    assert tree.valid?

    to_delete = keys.first(500)
    to_delete.each { |k| tree.delete(k) }
    assert_equal 500, tree.size
    assert tree.valid?

    # Verify full_scan matches expected remaining keys
    expected = (keys - to_delete).sort
    assert_equal expected, tree.full_scan.map(&:first)
  end

  def test_insert_delete_1000_keys_t5
    tree = CodingAdventures::BPlusTree.new(t: 5)
    keys = (1..1000).to_a.shuffle
    keys.each { |k| tree.insert(k, k) }
    keys.each { |k| tree.delete(k) }
    assert tree.empty?
    assert tree.valid?
  end

  def test_range_scan_1000_keys
    tree = CodingAdventures::BPlusTree.new(t: 3)
    (1..1000).each { |k| tree.insert(k, k) }
    result = tree.range_scan(200, 300)
    assert_equal (200..300).to_a, result.map(&:first)
    assert tree.valid?
  end

  def test_full_scan_walks_leaf_list
    tree = CodingAdventures::BPlusTree.new(t: 4)
    (1..500).to_a.shuffle.each { |k| tree.insert(k, k) }
    result = tree.full_scan
    assert_equal (1..500).to_a, result.map(&:first)
    assert_equal result.map(&:first).sort, result.map(&:first)
  end

  def test_string_keys
    tree = CodingAdventures::BPlusTree.new(t: 2)
    words = %w[mango apple zebra banana cherry date elderberry fig grape]
    words.each { |w| tree.insert(w, w.length) }
    assert_equal words.sort, tree.full_scan.map(&:first)
    assert tree.valid?
  end
end

# =============================================================================
# Height Tests
# =============================================================================

class TestBPlusTreeHeight < Minitest::Test
  def test_height_of_empty_tree
    tree = CodingAdventures::BPlusTree.new(t: 2)
    assert_equal 0, tree.height
  end

  def test_height_grows_with_splits
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..10).each { |k| tree.insert(k, k) }
    assert tree.height > 0
    assert tree.valid?
  end

  def test_height_large_tree
    tree = CodingAdventures::BPlusTree.new(t: 3)
    (1..100).each { |k| tree.insert(k, k) }
    assert tree.height > 0
    assert tree.height < 10
    assert tree.valid?
  end
end

# =============================================================================
# Internal Borrow and Merge Tests
# =============================================================================
#
# These tests specifically exercise the borrow-from-left/right and merge
# operations on INTERNAL nodes during deletion, which require updating
# separator keys correctly in the B+ tree.
# =============================================================================

class TestBPlusTreeInternalRebalance < Minitest::Test
  def test_internal_rotate_left_borrow_from_right
    # Build a tree where an internal node must borrow from its right sibling
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..20).each { |k| tree.insert(k, k) }
    # Delete from left subtree to force internal rotation
    (1..5).each { |k| tree.delete(k) }
    assert tree.valid?
    (6..20).each { |k| assert_equal k, tree.search(k) }
  end

  def test_internal_rotate_right_borrow_from_left
    # Build a tree where an internal node must borrow from its left sibling
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..20).each { |k| tree.insert(k, k) }
    # Delete from right subtree to force internal rotation
    (16..20).each { |k| tree.delete(k) }
    assert tree.valid?
    (1..15).each { |k| assert_equal k, tree.search(k) }
  end

  def test_internal_merge_reduces_tree
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..30).each { |k| tree.insert(k, k) }
    h_before = tree.height
    # Delete enough to force multiple merges
    (1..30).to_a.shuffle(random: Random.new(7)).first(20).each { |k| tree.delete(k) }
    assert tree.valid?
    assert tree.height <= h_before
  end

  def test_random_delete_order_valid_30_keys
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..30).each { |k| tree.insert(k, k) }
    order = (1..30).to_a.shuffle(random: Random.new(42))
    order.each do |k|
      tree.delete(k)
      assert tree.valid?, "Tree invalid after deleting #{k}"
    end
    assert tree.empty?
  end

  def test_random_delete_order_t3
    tree = CodingAdventures::BPlusTree.new(t: 3)
    (1..50).each { |k| tree.insert(k, k) }
    order = (1..50).to_a.shuffle(random: Random.new(99))
    order.each do |k|
      tree.delete(k)
      assert tree.valid?, "Tree invalid after deleting #{k}"
    end
    assert tree.empty?
  end
end

# =============================================================================
# Valid? Tests
# =============================================================================

class TestBPlusTreeValid < Minitest::Test
  def test_valid_after_every_insert
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..30).each do |k|
      tree.insert(k, k)
      assert tree.valid?, "Tree invalid after inserting #{k}"
    end
  end

  def test_valid_after_every_delete
    tree = CodingAdventures::BPlusTree.new(t: 2)
    (1..30).each { |k| tree.insert(k, k) }
    (1..30).to_a.shuffle.each do |k|
      tree.delete(k)
      assert tree.valid?, "Tree invalid after deleting #{k}"
    end
  end

  def test_valid_t3
    tree = CodingAdventures::BPlusTree.new(t: 3)
    (1..50).each { |k| tree.insert(k, k) }
    (1..50).step(3) { |k| tree.delete(k) }
    assert tree.valid?
  end
end
