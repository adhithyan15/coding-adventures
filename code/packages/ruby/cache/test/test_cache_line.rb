# frozen_string_literal: true

require "test_helper"

# Tests for CacheLine -- the smallest unit of data in a cache.
#
# Verifies the lifecycle of a cache line: creation (invalid), filling
# (valid + data), touching (LRU update), modification (dirty), and
# invalidation.
class TestCacheLineCreation < Minitest::Test
  # A new cache line should be invalid (empty box).
  def test_default_line_is_invalid
    line = CodingAdventures::Cache::CacheLine.new
    assert_equal false, line.valid
    assert_equal false, line.dirty
    assert_equal 0, line.tag
    assert_equal 0, line.last_access
  end

  # Default line size is 64 bytes (standard on modern CPUs).
  def test_default_line_size_is_64
    line = CodingAdventures::Cache::CacheLine.new
    assert_equal 64, line.data.length
    assert_equal 64, line.line_size
  end

  # Lines can be created with non-standard sizes (e.g., 32 bytes).
  def test_custom_line_size
    line = CodingAdventures::Cache::CacheLine.new(line_size: 32)
    assert_equal 32, line.data.length
    assert_equal 32, line.line_size
  end

  # All bytes in a new line should be zero.
  def test_data_initialized_to_zeros
    line = CodingAdventures::Cache::CacheLine.new(line_size: 8)
    assert_equal [0, 0, 0, 0, 0, 0, 0, 0], line.data
  end
end

class TestCacheLineFill < Minitest::Test
  # After fill, the line should be valid with the correct tag.
  def test_fill_makes_line_valid
    line = CodingAdventures::Cache::CacheLine.new(line_size: 8)
    line.fill(tag: 42, data: [1, 2, 3, 4, 5, 6, 7, 8], cycle: 100)
    assert_equal true, line.valid
    assert_equal 42, line.tag
    assert_equal 100, line.last_access
  end

  # Fill should store the provided data bytes.
  def test_fill_sets_data
    line = CodingAdventures::Cache::CacheLine.new(line_size: 4)
    line.fill(tag: 7, data: [0xAA, 0xBB, 0xCC, 0xDD], cycle: 0)
    assert_equal [0xAA, 0xBB, 0xCC, 0xDD], line.data
  end

  # Freshly loaded data is always clean (not modified).
  def test_fill_clears_dirty_bit
    line = CodingAdventures::Cache::CacheLine.new(line_size: 4)
    line.dirty = true
    line.fill(tag: 1, data: [0] * 4, cycle: 0)
    assert_equal false, line.dirty
  end

  # Fill should copy the data, not hold a reference to the original.
  def test_fill_makes_defensive_copy
    line = CodingAdventures::Cache::CacheLine.new(line_size: 4)
    original = [1, 2, 3, 4]
    line.fill(tag: 1, data: original, cycle: 0)
    original[0] = 99
    assert_equal 1, line.data[0]
  end
end

class TestCacheLineTouch < Minitest::Test
  # touch() should update the LRU timestamp.
  def test_touch_updates_last_access
    line = CodingAdventures::Cache::CacheLine.new
    line.fill(tag: 1, data: [0] * 64, cycle: 10)
    assert_equal 10, line.last_access
    line.touch(50)
    assert_equal 50, line.last_access
  end
end

class TestCacheLineInvalidate < Minitest::Test
  # Invalidation marks the line as not present.
  def test_invalidate_clears_valid_and_dirty
    line = CodingAdventures::Cache::CacheLine.new(line_size: 4)
    line.fill(tag: 5, data: [1, 2, 3, 4], cycle: 10)
    line.dirty = true
    line.invalidate
    assert_equal false, line.valid
    assert_equal false, line.dirty
  end

  # Data is not erased on invalidation (just marked invalid).
  def test_invalidate_does_not_zero_data
    line = CodingAdventures::Cache::CacheLine.new(line_size: 4)
    line.fill(tag: 5, data: [0xAA, 0xBB, 0xCC, 0xDD], cycle: 0)
    line.invalidate
    assert_equal [0xAA, 0xBB, 0xCC, 0xDD], line.data
  end
end

class TestCacheLineToData < Minitest::Test
  # to_data returns an immutable snapshot.
  def test_to_data_snapshot
    line = CodingAdventures::Cache::CacheLine.new(line_size: 4)
    line.fill(tag: 42, data: [1, 2, 3, 4], cycle: 10)
    data = line.to_data
    assert_equal true, data.valid
    assert_equal 42, data.tag
    assert data.frozen?
  end
end

class TestCacheLineRepr < Minitest::Test
  # Invalid lines show '--' for valid/dirty flags.
  def test_repr_invalid_line
    line = CodingAdventures::Cache::CacheLine.new
    assert_includes line.to_s, "--"
  end

  # Valid clean lines show 'V-'.
  def test_repr_valid_clean_line
    line = CodingAdventures::Cache::CacheLine.new(line_size: 4)
    line.fill(tag: 0xFF, data: [0] * 4, cycle: 0)
    s = line.to_s
    assert_includes s, "V-"
    assert_includes s.downcase, "0xff"
  end

  # Valid dirty lines show 'VD'.
  def test_repr_valid_dirty_line
    line = CodingAdventures::Cache::CacheLine.new(line_size: 4)
    line.fill(tag: 1, data: [0] * 4, cycle: 0)
    line.dirty = true
    assert_includes line.to_s, "VD"
  end
end
