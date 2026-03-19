# frozen_string_literal: true

require "test_helper"

# Tests for CacheSet and CacheConfig -- set-associative lookup with LRU.
class TestCacheConfigValidation < Minitest::Test
  # A typical L1 config should be accepted.
  def test_valid_config
    config = CodingAdventures::Cache::CacheConfig.new(
      name: "L1D", total_size: 65536, line_size: 64,
      associativity: 4, access_latency: 1
    )
    assert_equal 256, config.num_sets
    assert_equal 1024, config.num_lines
  end

  # Total size must be positive.
  def test_invalid_total_size
    assert_raises(ArgumentError) do
      CodingAdventures::Cache::CacheConfig.new(name: "bad", total_size: 0)
    end
  end

  # Line size must be a power of 2.
  def test_invalid_line_size_not_power_of_2
    assert_raises(ArgumentError) do
      CodingAdventures::Cache::CacheConfig.new(name: "bad", total_size: 256, line_size: 48)
    end
  end

  # Associativity must be positive.
  def test_invalid_associativity
    assert_raises(ArgumentError) do
      CodingAdventures::Cache::CacheConfig.new(name: "bad", total_size: 256, associativity: 0)
    end
  end

  # total_size must be divisible by line_size * associativity.
  def test_invalid_size_alignment
    assert_raises(ArgumentError) do
      CodingAdventures::Cache::CacheConfig.new(name: "bad", total_size: 100, line_size: 64, associativity: 4)
    end
  end

  # Write policy must be 'write-back' or 'write-through'.
  def test_invalid_write_policy
    assert_raises(ArgumentError) do
      CodingAdventures::Cache::CacheConfig.new(
        name: "bad", total_size: 256, line_size: 64,
        associativity: 1, write_policy: "write-around"
      )
    end
  end

  # Access latency must be non-negative.
  def test_negative_latency
    assert_raises(ArgumentError) do
      CodingAdventures::Cache::CacheConfig.new(
        name: "bad", total_size: 256, line_size: 64,
        associativity: 1, access_latency: -1
      )
    end
  end

  # Write-through is a valid write policy.
  def test_write_through_config
    config = CodingAdventures::Cache::CacheConfig.new(
      name: "L1D", total_size: 256, line_size: 64,
      associativity: 1, write_policy: "write-through"
    )
    assert_equal "write-through", config.write_policy
  end

  # CacheConfig is frozen (immutable via Data.define).
  def test_config_is_frozen
    config = CodingAdventures::Cache::CacheConfig.new(
      name: "L1D", total_size: 256, line_size: 64, associativity: 1
    )
    assert config.frozen?
  end
end

class TestCacheSetLookup < Minitest::Test
  # An empty set should always miss (all lines invalid).
  def test_lookup_miss_on_empty_set
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 4, line_size: 64)
    hit, way = cs.lookup(42)
    assert_equal false, hit
    assert_nil way
  end

  # After filling a line, lookup should find it.
  def test_lookup_hit_after_fill
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 4, line_size: 8)
    cs.lines[0].fill(tag: 42, data: [0] * 8, cycle: 0)
    hit, way = cs.lookup(42)
    assert_equal true, hit
    assert_equal 0, way
  end

  # Lookup with a different tag should miss.
  def test_lookup_miss_wrong_tag
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 4, line_size: 8)
    cs.lines[0].fill(tag: 42, data: [0] * 8, cycle: 0)
    hit, way = cs.lookup(99)
    assert_equal false, hit
    assert_nil way
  end

  # When multiple ways are valid, lookup returns the matching one.
  def test_lookup_finds_correct_way
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 4, line_size: 8)
    cs.lines[0].fill(tag: 10, data: [0] * 8, cycle: 0)
    cs.lines[1].fill(tag: 20, data: [0] * 8, cycle: 0)
    cs.lines[2].fill(tag: 30, data: [0] * 8, cycle: 0)
    hit, way = cs.lookup(20)
    assert_equal true, hit
    assert_equal 1, way
  end
end

class TestCacheSetAccess < Minitest::Test
  # On a hit, the line's last_access should be updated.
  def test_access_hit_updates_lru
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 2, line_size: 8)
    cs.lines[0].fill(tag: 10, data: [0] * 8, cycle: 5)
    hit, line = cs.access(10, 100)
    assert_equal true, hit
    assert_equal 100, line.last_access
  end

  # On a miss with all ways full, return the LRU line.
  def test_access_miss_returns_lru_victim
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 2, line_size: 8)
    cs.lines[0].fill(tag: 10, data: [0] * 8, cycle: 1)
    cs.lines[1].fill(tag: 20, data: [0] * 8, cycle: 5)
    hit, victim = cs.access(99, 10)
    assert_equal false, hit
    assert_equal 10, victim.tag
  end
end

class TestCacheSetAllocate < Minitest::Test
  # If there's an invalid line, use it (no eviction).
  def test_allocate_into_empty_slot
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 4, line_size: 8)
    evicted = cs.allocate(tag: 42, data: [0xAA] * 8, cycle: 10)
    assert_nil evicted
    hit, _way = cs.lookup(42)
    assert_equal true, hit
  end

  # When all ways are valid, LRU line is evicted.
  def test_allocate_evicts_lru_when_full
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 2, line_size: 8)
    cs.allocate(tag: 10, data: [0] * 8, cycle: 1)
    cs.allocate(tag: 20, data: [0] * 8, cycle: 2)
    evicted = cs.allocate(tag: 30, data: [0] * 8, cycle: 3)
    assert_nil evicted # tag=10 was clean
    hit_10, _ = cs.lookup(10)
    hit_30, _ = cs.lookup(30)
    assert_equal false, hit_10
    assert_equal true, hit_30
  end

  # If the LRU line is dirty, it should be returned for writeback.
  def test_allocate_returns_dirty_eviction
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 2, line_size: 8)
    cs.allocate(tag: 10, data: [0xAA] * 8, cycle: 1)
    cs.lines[0].dirty = true
    cs.allocate(tag: 20, data: [0] * 8, cycle: 2)
    evicted = cs.allocate(tag: 30, data: [0] * 8, cycle: 3)
    refute_nil evicted
    assert_equal true, evicted.dirty
    assert_equal 10, evicted.tag
    assert_equal [0xAA] * 8, evicted.data
  end

  # Empty slots should be filled before any eviction occurs.
  def test_allocate_fills_all_empty_slots_first
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 4, line_size: 8)
    4.times do |i|
      evicted = cs.allocate(tag: i, data: [0] * 8, cycle: i)
      assert_nil evicted
    end
    cs.allocate(tag: 99, data: [0] * 8, cycle: 10)
    hit_0, _ = cs.lookup(0)
    assert_equal false, hit_0
  end
end

class TestDirectMappedSet < Minitest::Test
  # Two addresses mapping to the same set cause a conflict miss.
  def test_direct_mapped_conflict
    cs = CodingAdventures::Cache::CacheSet.new(associativity: 1, line_size: 8)
    cs.allocate(tag: 10, data: [0] * 8, cycle: 1)
    cs.allocate(tag: 20, data: [0] * 8, cycle: 2)
    hit_10, _ = cs.lookup(10)
    hit_20, _ = cs.lookup(20)
    assert_equal false, hit_10
    assert_equal true, hit_20
  end
end
