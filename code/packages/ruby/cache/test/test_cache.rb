# frozen_string_literal: true

require "test_helper"

# Tests for CacheSimulator -- a single configurable cache level.
class TestAddressDecomposition < Minitest::Test
  def make_cache
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "test", total_size: 1024, line_size: 64, associativity: 4
      )
    )
  end

  # Address 0 should decompose to tag=0, set=0, offset=0.
  def test_address_zero
    cache = make_cache
    tag, set_idx, offset = cache.decompose_address(0)
    assert_equal 0, tag
    assert_equal 0, set_idx
    assert_equal 0, offset
  end

  # Low 6 bits should be the offset (byte within line).
  def test_offset_extraction
    cache = make_cache
    tag, set_idx, offset = cache.decompose_address(0x1F)
    assert_equal 31, offset
    assert_equal 0, set_idx
  end

  # Bits 6-7 should be the set index (for 4 sets).
  def test_set_index_extraction
    cache = make_cache
    _, set_idx, offset = cache.decompose_address(0x40)
    assert_equal 0, offset
    assert_equal 1, set_idx

    _, set_idx2, _ = cache.decompose_address(0x80)
    assert_equal 2, set_idx2

    _, set_idx3, _ = cache.decompose_address(0xC0)
    assert_equal 3, set_idx3
  end

  # Bits above set+offset are the tag.
  def test_tag_extraction
    cache = make_cache
    tag, set_idx, offset = cache.decompose_address(0x100)
    assert_equal 0, offset
    assert_equal 0, set_idx
    assert_equal 1, tag
  end

  # Full decomposition of a known address.
  def test_known_address_decomposition
    cache = make_cache
    tag, set_idx, offset = cache.decompose_address(0x1A2B3C4D)
    assert_equal 0x0D, offset
    assert_equal((0x1A2B3C4D >> 6) & 0x3, set_idx)
    assert_equal(0x1A2B3C4D >> 8, tag)
  end
end

class TestCacheRead < Minitest::Test
  def make_cache
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "test", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 3
      )
    )
  end

  # The first read to any address is always a compulsory miss.
  def test_first_read_is_miss
    cache = make_cache
    access = cache.read(address: 0x100, cycle: 0)
    assert_equal false, access.hit
    assert_equal 3, access.cycles
  end

  # After a miss brings data in, the next read should hit.
  def test_second_read_same_address_is_hit
    cache = make_cache
    cache.read(address: 0x100, cycle: 0)
    access = cache.read(address: 0x100, cycle: 1)
    assert_equal true, access.hit
    assert_equal 3, access.cycles
  end

  # Addresses within the same cache line share the line.
  def test_read_different_address_same_line
    cache = make_cache
    cache.read(address: 0x100, cycle: 0)
    access = cache.read(address: 0x110, cycle: 1)
    assert_equal true, access.hit
  end

  # A read miss should be reflected in the statistics.
  def test_read_miss_updates_stats
    cache = make_cache
    cache.read(address: 0x100, cycle: 0)
    assert_equal 1, cache.stats.reads
    assert_equal 1, cache.stats.misses
    assert_equal 0, cache.stats.hits
  end

  # A read hit should be reflected in the statistics.
  def test_read_hit_updates_stats
    cache = make_cache
    cache.read(address: 0x100, cycle: 0)
    cache.read(address: 0x100, cycle: 1)
    assert_equal 2, cache.stats.reads
    assert_equal 1, cache.stats.hits
    assert_equal 1, cache.stats.misses
  end

  # The CacheAccess should contain the correct address decomposition.
  def test_read_returns_correct_decomposition
    cache = make_cache
    access = cache.read(address: 0x100, cycle: 0)
    assert_equal 0x100, access.address
    assert_kind_of Integer, access.tag
    assert_kind_of Integer, access.offset
  end
end

class TestCacheWrite < Minitest::Test
  def make_wb_cache
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "test", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1,
        write_policy: "write-back"
      )
    )
  end

  def make_wt_cache
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "test", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1,
        write_policy: "write-through"
      )
    )
  end

  # A write miss should allocate a new cache line (write-allocate).
  def test_write_miss_allocates_line
    cache = make_wb_cache
    access = cache.write(address: 0x100, data: [0xAB], cycle: 0)
    assert_equal false, access.hit
    read_access = cache.read(address: 0x100, cycle: 1)
    assert_equal true, read_access.hit
  end

  # In write-back, a write hit marks the line as dirty.
  def test_write_hit_marks_dirty_in_writeback
    cache = make_wb_cache
    cache.read(address: 0x100, cycle: 0)
    cache.write(address: 0x100, data: [0xAB], cycle: 1)
    tag, set_idx, _ = cache.decompose_address(0x100)
    hit, way = cache.sets[set_idx].lookup(tag)
    assert_equal true, hit
    refute_nil way
    assert_equal true, cache.sets[set_idx].lines[way].dirty
  end

  # In write-through, lines are never dirty.
  def test_write_through_does_not_mark_dirty
    cache = make_wt_cache
    cache.read(address: 0x100, cycle: 0)
    cache.write(address: 0x100, data: [0xAB], cycle: 1)
    tag, set_idx, _ = cache.decompose_address(0x100)
    hit, way = cache.sets[set_idx].lookup(tag)
    assert_equal true, hit
    refute_nil way
    assert_equal false, cache.sets[set_idx].lines[way].dirty
  end

  # Written data should be readable from the cache line.
  def test_write_stores_data
    cache = make_wb_cache
    cache.write(address: 0x100, data: [0xDE, 0xAD], cycle: 0)
    tag, set_idx, offset = cache.decompose_address(0x100)
    _, way = cache.sets[set_idx].lookup(tag)
    refute_nil way
    line = cache.sets[set_idx].lines[way]
    assert_equal 0xDE, line.data[offset]
    assert_equal 0xAD, line.data[offset + 1]
  end

  # Write operations should be tracked in stats.
  def test_write_updates_stats
    cache = make_wb_cache
    cache.write(address: 0x100, cycle: 0)
    cache.write(address: 0x100, cycle: 1)
    assert_equal 2, cache.stats.writes
    assert_equal 1, cache.stats.misses
    assert_equal 1, cache.stats.hits
  end
end

class TestDirtyEviction < Minitest::Test
  # When a dirty line is evicted, the CacheAccess should report it.
  def test_dirty_eviction_returns_evicted_line
    cache = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "test", total_size: 64, line_size: 64,
        associativity: 1, access_latency: 1,
        write_policy: "write-back"
      )
    )
    cache.write(address: 0, data: [0xFF], cycle: 0)
    access = cache.read(address: 64, cycle: 1)
    assert_equal false, access.hit
    refute_nil access.evicted
    assert_equal true, access.evicted.dirty
  end

  # Evictions and writebacks should be counted in stats.
  def test_eviction_stats_tracked
    cache = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "test", total_size: 64, line_size: 64,
        associativity: 1, access_latency: 1,
        write_policy: "write-back"
      )
    )
    cache.write(address: 0, data: [0xFF], cycle: 0)
    cache.read(address: 64, cycle: 1)
    assert cache.stats.evictions >= 1
    assert cache.stats.writebacks >= 1
  end
end

class TestCacheInvalidation < Minitest::Test
  # After invalidation, every access should be a miss.
  def test_invalidate_causes_all_misses
    cache = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "test", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
    cache.read(address: 0x100, cycle: 0)
    cache.read(address: 0x100, cycle: 1)
    assert_equal 1, cache.stats.hits

    cache.invalidate
    access = cache.read(address: 0x100, cycle: 2)
    assert_equal false, access.hit
  end
end

class TestCacheEdgeCases < Minitest::Test
  # A cache with only 1 set (fully associative for its size).
  def test_single_set_cache
    cache = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "tiny", total_size: 128, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
    cache.read(address: 0, cycle: 0)
    cache.read(address: 64, cycle: 1)
    assert_equal true, cache.read(address: 0, cycle: 2).hit
    assert_equal true, cache.read(address: 64, cycle: 3).hit
  end

  # Direct-mapped: two addresses to the same set cause thrashing.
  def test_direct_mapped_conflict_eviction
    cache = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "dm", total_size: 256, line_size: 64,
        associativity: 1, access_latency: 1
      )
    )
    addr_a = 0x000
    addr_b = 0x100
    cache.read(address: addr_a, cycle: 0)
    cache.read(address: addr_b, cycle: 1)
    cache.read(address: addr_a, cycle: 2)
    cache.read(address: addr_b, cycle: 3)
    assert_equal 0, cache.stats.hits
    assert_equal 4, cache.stats.misses
  end

  # fill_line() installs data without going through read/write stats.
  def test_fill_line_directly
    cache = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "test", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
    cache.fill_line(address: 0x100, data: [0xAB] * 64, cycle: 0)
    access = cache.read(address: 0x100, cycle: 1)
    assert_equal true, access.hit
  end

  # Cache to_s should show configuration summary.
  def test_to_s
    cache = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L1D", total_size: 65536, line_size: 64,
        associativity: 4, access_latency: 1
      )
    )
    s = cache.to_s
    assert_includes s, "L1D"
    assert_includes s, "64KB"
    assert_includes s, "4-way"
  end
end
