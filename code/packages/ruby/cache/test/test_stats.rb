# frozen_string_literal: true

require "test_helper"

# Tests for CacheStats -- verifying hit rate calculation and counter tracking.
#
# These tests ensure the scorecard is accurate. If the stats are wrong,
# every performance analysis built on top will be misleading.
class TestCacheStatsBasics < Minitest::Test
  # A fresh CacheStats should have all counters at zero.
  def test_initial_state_is_all_zeros
    stats = CodingAdventures::Cache::CacheStats.new
    assert_equal 0, stats.reads
    assert_equal 0, stats.writes
    assert_equal 0, stats.hits
    assert_equal 0, stats.misses
    assert_equal 0, stats.evictions
    assert_equal 0, stats.writebacks
    assert_equal 0, stats.total_accesses
  end

  # Recording a read hit should increment reads and hits.
  def test_record_read_hit
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: true)
    assert_equal 1, stats.reads
    assert_equal 1, stats.hits
    assert_equal 0, stats.misses
  end

  # Recording a read miss should increment reads and misses.
  def test_record_read_miss
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: false)
    assert_equal 1, stats.reads
    assert_equal 0, stats.hits
    assert_equal 1, stats.misses
  end

  # Recording a write hit should increment writes and hits.
  def test_record_write_hit
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_write(hit: true)
    assert_equal 1, stats.writes
    assert_equal 1, stats.hits
  end

  # Recording a write miss should increment writes and misses.
  def test_record_write_miss
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_write(hit: false)
    assert_equal 1, stats.writes
    assert_equal 1, stats.misses
  end

  # A clean eviction increments evictions but not writebacks.
  def test_record_eviction_clean
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_eviction(dirty: false)
    assert_equal 1, stats.evictions
    assert_equal 0, stats.writebacks
  end

  # A dirty eviction increments both evictions and writebacks.
  def test_record_eviction_dirty
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_eviction(dirty: true)
    assert_equal 1, stats.evictions
    assert_equal 1, stats.writebacks
  end
end

class TestCacheStatsRates < Minitest::Test
  # Hit rate should be 0.0 when no accesses have been made.
  def test_hit_rate_no_accesses
    stats = CodingAdventures::Cache::CacheStats.new
    assert_in_delta 0.0, stats.hit_rate
  end

  # Miss rate should be 0.0 when no accesses have been made.
  def test_miss_rate_no_accesses
    stats = CodingAdventures::Cache::CacheStats.new
    assert_in_delta 0.0, stats.miss_rate
  end

  # 100% hit rate when every access is a hit.
  def test_hit_rate_all_hits
    stats = CodingAdventures::Cache::CacheStats.new
    10.times { stats.record_read(hit: true) }
    assert_in_delta 1.0, stats.hit_rate
    assert_in_delta 0.0, stats.miss_rate
  end

  # 0% hit rate when every access is a miss.
  def test_hit_rate_all_misses
    stats = CodingAdventures::Cache::CacheStats.new
    10.times { stats.record_read(hit: false) }
    assert_in_delta 0.0, stats.hit_rate
    assert_in_delta 1.0, stats.miss_rate
  end

  # 50% hit rate with equal hits and misses.
  def test_hit_rate_fifty_percent
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: true)
    stats.record_read(hit: false)
    assert_in_delta 0.5, stats.hit_rate
    assert_in_delta 0.5, stats.miss_rate
  end

  # Hit rate includes both reads and writes.
  def test_hit_rate_mixed_reads_and_writes
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: true)
    stats.record_write(hit: true)
    stats.record_read(hit: false)
    stats.record_write(hit: false)
    assert_equal 4, stats.total_accesses
    assert_in_delta 0.5, stats.hit_rate
  end

  # Hit rate + miss rate should always equal 1.0 (with accesses).
  def test_hit_rate_plus_miss_rate_equals_one
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: true)
    stats.record_read(hit: true)
    stats.record_read(hit: false)
    assert_in_delta 1.0, stats.hit_rate + stats.miss_rate
  end
end

class TestCacheStatsReset < Minitest::Test
  # Reset should bring all counters back to zero.
  def test_reset_clears_all_counters
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: true)
    stats.record_write(hit: false)
    stats.record_eviction(dirty: true)
    stats.reset
    assert_equal 0, stats.reads
    assert_equal 0, stats.writes
    assert_equal 0, stats.hits
    assert_equal 0, stats.misses
    assert_equal 0, stats.evictions
    assert_equal 0, stats.writebacks
    assert_equal 0, stats.total_accesses
  end

  # Stats should work correctly after a reset.
  def test_reset_then_record
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: true)
    stats.reset
    stats.record_read(hit: false)
    assert_equal 1, stats.reads
    assert_equal 1, stats.misses
    assert_in_delta 0.0, stats.hit_rate
  end
end

class TestCacheStatsToData < Minitest::Test
  # to_data returns an immutable snapshot.
  def test_to_data_snapshot
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: true)
    stats.record_read(hit: false)
    data = stats.to_data
    assert_equal 1, data.hits
    assert_equal 1, data.misses
    assert_equal 2, data.total_accesses
    assert_in_delta 0.5, data.hit_rate
    assert data.frozen?
  end
end

class TestCacheStatsRepr < Minitest::Test
  # to_s should show accesses, hits, misses, and hit rate.
  def test_to_s_includes_key_info
    stats = CodingAdventures::Cache::CacheStats.new
    stats.record_read(hit: true)
    stats.record_read(hit: false)
    s = stats.to_s
    assert_includes s, "accesses=2"
    assert_includes s, "hits=1"
    assert_includes s, "misses=1"
    assert_includes s, "50.0%"
  end
end
