# frozen_string_literal: true

require "test_helper"

# Tests for CacheHierarchy -- multi-level cache system.
class TestHierarchyRead < Minitest::Test
  def make_l1d(size: 256)
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L1D", total_size: size, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
  end

  def make_l2(size: 1024)
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L2", total_size: size, line_size: 64,
        associativity: 4, access_latency: 10
      )
    )
  end

  def make_l3(size: 4096)
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L3", total_size: size, line_size: 64,
        associativity: 8, access_latency: 30
      )
    )
  end

  # On a cold cache, the first read must go all the way to main memory.
  def test_first_read_goes_to_memory
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: make_l1d, l2: make_l2, main_memory_latency: 100
    )
    result = h.read(address: 0x1000, cycle: 0)
    assert_equal "memory", result.served_by
    assert_equal 1 + 10 + 100, result.total_cycles
  end

  # After data is filled into L1, the second read should hit L1.
  def test_second_read_hits_l1
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: make_l1d, l2: make_l2, main_memory_latency: 100
    )
    h.read(address: 0x1000, cycle: 0)
    result = h.read(address: 0x1000, cycle: 1)
    assert_equal "L1D", result.served_by
    assert_equal 1, result.total_cycles
  end

  # If L1 misses but L2 has it, data should be served from L2.
  def test_l1_miss_l2_hit
    l1d = make_l1d
    l2 = make_l2
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: l1d, l2: l2, main_memory_latency: 100
    )
    l2.fill_line(address: 0x1000, data: [0] * 64, cycle: 0)
    result = h.read(address: 0x1000, cycle: 1)
    assert_equal "L2", result.served_by
    assert_equal 1 + 10, result.total_cycles
  end

  # L1 and L2 miss, but L3 has the data.
  def test_l1_l2_miss_l3_hit
    l1d = make_l1d
    l2 = make_l2
    l3 = make_l3
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: l1d, l2: l2, l3: l3, main_memory_latency: 100
    )
    l3.fill_line(address: 0x2000, data: [0] * 64, cycle: 0)
    result = h.read(address: 0x2000, cycle: 1)
    assert_equal "L3", result.served_by
    assert_equal 1 + 10 + 30, result.total_cycles
  end

  # When all cache levels miss, the request goes to main memory.
  def test_all_levels_miss_goes_to_memory
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: make_l1d, l2: make_l2, l3: make_l3, main_memory_latency: 100
    )
    result = h.read(address: 0x3000, cycle: 0)
    assert_equal "memory", result.served_by
    assert_equal 1 + 10 + 30 + 100, result.total_cycles
  end

  # When L2 serves data, L1 should also be filled (inclusive policy).
  def test_inclusive_fill_after_l2_hit
    l1d = make_l1d
    l2 = make_l2
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: l1d, l2: l2, main_memory_latency: 100
    )
    l2.fill_line(address: 0x1000, data: [0] * 64, cycle: 0)
    h.read(address: 0x1000, cycle: 1)
    result = h.read(address: 0x1000, cycle: 2)
    assert_equal "L1D", result.served_by
  end

  # When memory serves data, all levels should be filled.
  def test_inclusive_fill_after_memory
    l1d = make_l1d
    l2 = make_l2
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: l1d, l2: l2, main_memory_latency: 100
    )
    h.read(address: 0x5000, cycle: 0)
    result = h.read(address: 0x5000, cycle: 1)
    assert_equal "L1D", result.served_by
  end
end

class TestHarvardArchitecture < Minitest::Test
  # Instruction reads should go through L1I, not L1D.
  def test_instruction_read_uses_l1i
    l1i = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L1I", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
    l1d = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L1D", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
    l2 = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L2", total_size: 1024, line_size: 64,
        associativity: 4, access_latency: 10
      )
    )
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1i: l1i, l1d: l1d, l2: l2, main_memory_latency: 100
    )
    l1i.fill_line(address: 0x1000, data: [0] * 64, cycle: 0)
    result = h.read(address: 0x1000, is_instruction: true, cycle: 1)
    assert_equal "L1I", result.served_by
    assert_equal 1, result.total_cycles
  end

  # Data reads should use L1D, even if L1I has the data.
  def test_data_read_does_not_use_l1i
    l1i = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L1I", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
    l1d = CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L1D", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1i: l1i, l1d: l1d, main_memory_latency: 100
    )
    l1i.fill_line(address: 0x1000, data: [0] * 64, cycle: 0)
    result = h.read(address: 0x1000, is_instruction: false, cycle: 1)
    assert_equal "memory", result.served_by
  end
end

class TestHierarchyWrite < Minitest::Test
  def make_l1d
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L1D", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
  end

  def make_l2
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L2", total_size: 1024, line_size: 64,
        associativity: 4, access_latency: 10
      )
    )
  end

  # If L1D has the data, write hits there.
  def test_write_hit_at_l1
    l1d = make_l1d
    h = CodingAdventures::Cache::CacheHierarchy.new(l1d: l1d, main_memory_latency: 100)
    h.read(address: 0x1000, cycle: 0)
    result = h.write(address: 0x1000, data: [0xAB], cycle: 1)
    assert_equal "L1D", result.served_by
    assert_equal 1, result.total_cycles
  end

  # A write miss at L1 walks down to find the data.
  def test_write_miss_goes_to_lower_levels
    l1d = make_l1d
    l2 = make_l2
    h = CodingAdventures::Cache::CacheHierarchy.new(l1d: l1d, l2: l2, main_memory_latency: 100)
    result = h.write(address: 0x2000, data: [0xFF], cycle: 0)
    assert_equal "memory", result.served_by
  end

  # Write miss at L1, but L2 has the data.
  def test_write_miss_l2_hit
    l1d = make_l1d
    l2 = make_l2
    h = CodingAdventures::Cache::CacheHierarchy.new(l1d: l1d, l2: l2, main_memory_latency: 100)
    l2.fill_line(address: 0x1000, data: [0] * 64, cycle: 0)
    result = h.write(address: 0x1000, data: [0xAB], cycle: 1)
    assert_equal "L2", result.served_by
  end
end

class TestNoCacheHierarchy < Minitest::Test
  # With no caches, every read costs main memory latency.
  def test_read_goes_straight_to_memory
    h = CodingAdventures::Cache::CacheHierarchy.new(main_memory_latency: 200)
    result = h.read(address: 0x1000, cycle: 0)
    assert_equal "memory", result.served_by
    assert_equal 200, result.total_cycles
  end

  # With no caches, every write costs main memory latency.
  def test_write_goes_straight_to_memory
    h = CodingAdventures::Cache::CacheHierarchy.new(main_memory_latency: 200)
    result = h.write(address: 0x1000, data: [0xAB], cycle: 0)
    assert_equal "memory", result.served_by
    assert_equal 200, result.total_cycles
  end
end

class TestHierarchyUtilities < Minitest::Test
  def make_l1d
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L1D", total_size: 256, line_size: 64,
        associativity: 2, access_latency: 1
      )
    )
  end

  def make_l2
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L2", total_size: 1024, line_size: 64,
        associativity: 4, access_latency: 10
      )
    )
  end

  def make_l3
    CodingAdventures::Cache::CacheSimulator.new(
      CodingAdventures::Cache::CacheConfig.new(
        name: "L3", total_size: 4096, line_size: 64,
        associativity: 8, access_latency: 30
      )
    )
  end

  # invalidate_all() should cause all subsequent reads to miss.
  def test_invalidate_all
    l1d = make_l1d
    l2 = make_l2
    h = CodingAdventures::Cache::CacheHierarchy.new(l1d: l1d, l2: l2, main_memory_latency: 100)
    h.read(address: 0x1000, cycle: 0)
    h.read(address: 0x1000, cycle: 1)
    h.invalidate_all
    result = h.read(address: 0x1000, cycle: 2)
    assert_equal "memory", result.served_by
  end

  # reset_stats() should zero all cache level stats.
  def test_reset_stats
    l1d = make_l1d
    l2 = make_l2
    h = CodingAdventures::Cache::CacheHierarchy.new(l1d: l1d, l2: l2, main_memory_latency: 100)
    h.read(address: 0x1000, cycle: 0)
    h.reset_stats
    assert_equal 0, l1d.stats.total_accesses
    assert_equal 0, l2.stats.total_accesses
  end

  # to_s should summarize the hierarchy configuration.
  def test_to_s
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: make_l1d, l2: make_l2, l3: make_l3, main_memory_latency: 100
    )
    s = h.to_s
    assert_includes s, "L1D"
    assert_includes s, "L2"
    assert_includes s, "L3"
    assert_includes s, "mem=100cyc"
  end

  # HierarchyAccess should report which level index served the data.
  def test_hit_at_level_tracking
    h = CodingAdventures::Cache::CacheHierarchy.new(
      l1d: make_l1d, l2: make_l2, main_memory_latency: 100
    )
    result = h.read(address: 0x1000, cycle: 0)
    assert_equal 2, result.hit_at_level
    result = h.read(address: 0x1000, cycle: 1)
    assert_equal 0, result.hit_at_level
  end
end
