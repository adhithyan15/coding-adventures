defmodule CodingAdventures.CacheTest do
  use ExUnit.Case

  alias CodingAdventures.Cache
  alias CodingAdventures.CacheConfig
  alias CodingAdventures.CacheLine
  alias CodingAdventures.CacheHierarchy
  alias CodingAdventures.CacheSet
  alias CodingAdventures.CacheStats

  test "config computes lines and sets" do
    config =
      CacheConfig.new!(
        name: "L1D",
        total_size: 1024,
        line_size: 64,
        associativity: 4,
        access_latency: 1
      )

    assert CacheConfig.num_lines(config) == 16
    assert CacheConfig.num_sets(config) == 4
  end

  test "config validation rejects invalid values" do
    assert {:error, _message} = CacheConfig.new(name: "", total_size: 64)
    assert {:error, _message} = CacheConfig.new(name: "bad", total_size: 64, line_size: 48)
    assert {:error, _message} = CacheConfig.new(name: "bad", total_size: 64, write_policy: "invalid")
  end

  test "cache line helpers fill touch and invalidate" do
    line = CacheLine.new(8)
    refute line.valid
    assert CacheLine.line_size(line) == 8

    line = CacheLine.fill(line, 7, [1, 2, 3, 4], 10)
    assert line.valid
    assert line.tag == 7
    assert line.last_access == 10

    line = CacheLine.touch(line, 11)
    assert line.last_access == 11

    line = CacheLine.invalidate(line)
    refute line.valid
    refute line.dirty
  end

  test "decompose_address splits tag set and offset" do
    cache =
      CacheConfig.new!(name: "test", total_size: 1024, line_size: 64, associativity: 4)
      |> Cache.new()

    assert Cache.decompose_address(cache, 0) == {0, 0, 0}
    assert Cache.decompose_address(cache, 0x40) == {0, 1, 0}

    {tag, set_index, offset} = Cache.decompose_address(cache, 0x1A2B3C4D)
    assert offset == 0x0D
    assert set_index == Bitwise.band(Bitwise.bsr(0x1A2B3C4D, 6), 0x3)
    assert tag == Bitwise.bsr(0x1A2B3C4D, 8)
  end

  test "first read misses and second read hits" do
    cache =
      CacheConfig.new!(name: "test", total_size: 256, line_size: 64, associativity: 2, access_latency: 3)
      |> Cache.new()

    {cache, first} = Cache.read(cache, 0x100, 1, 0)
    assert first.hit == false
    assert first.cycles == 3

    {cache, second} = Cache.read(cache, 0x100, 1, 1)
    assert second.hit
    assert cache.stats.reads == 2
    assert cache.stats.hits == 1
    assert cache.stats.misses == 1
    assert_in_delta CacheStats.hit_rate(cache.stats), 0.5, 1.0e-9
  end

  test "cache set finds invalid slot before evicting lru" do
    cache_set = CacheSet.new(2, 8)
    assert CacheSet.find_lru_index(cache_set) == 0

    {cache_set, _evicted, false} = CacheSet.allocate(cache_set, 1, List.duplicate(0, 8), 0)
    assert CacheSet.find_lru_index(cache_set) == 1
  end

  test "write back marks lines dirty and stores data" do
    cache =
      CacheConfig.new!(
        name: "test",
        total_size: 256,
        line_size: 64,
        associativity: 2,
        write_policy: "write-back"
      )
      |> Cache.new()

    {cache, access} = Cache.write(cache, 0x100, [0xDE, 0xAD], 0)
    assert access.hit == false

    {tag, set_index, offset} = Cache.decompose_address(cache, 0x100)
    cache_set = Enum.at(cache.sets, set_index)
    {:hit, way_index} = CodingAdventures.CacheSet.lookup(cache_set, tag)
    line = Enum.at(cache_set.lines, way_index)

    assert line.dirty
    assert Enum.at(line.data, offset) == 0xDE
    assert Enum.at(line.data, offset + 1) == 0xAD
  end

  test "write through leaves line clean on hit" do
    cache =
      CacheConfig.new!(
        name: "test",
        total_size: 256,
        line_size: 64,
        associativity: 2,
        write_policy: "write-through"
      )
      |> Cache.new()

    {cache, _} = Cache.read(cache, 0x100, 1, 0)
    {cache, _} = Cache.write(cache, 0x100, [0xAB], 1)

    {tag, set_index, _offset} = Cache.decompose_address(cache, 0x100)
    cache_set = Enum.at(cache.sets, set_index)
    {:hit, way_index} = CodingAdventures.CacheSet.lookup(cache_set, tag)
    line = Enum.at(cache_set.lines, way_index)

    refute line.dirty
  end

  test "dirty eviction returns evicted line and records writeback" do
    cache =
      CacheConfig.new!(
        name: "test",
        total_size: 64,
        line_size: 64,
        associativity: 1,
        write_policy: "write-back"
      )
      |> Cache.new()

    {cache, _} = Cache.write(cache, 0, [0xFF], 0)
    {cache, access} = Cache.read(cache, 64, 1, 1)

    assert access.hit == false
    assert access.evicted != nil
    assert access.evicted.dirty
    assert cache.stats.evictions == 1
    assert cache.stats.writebacks == 1
  end

  test "invalidate flushes all cache lines" do
    cache =
      CacheConfig.new!(name: "test", total_size: 256, line_size: 64, associativity: 2)
      |> Cache.new()

    {cache, _} = Cache.read(cache, 0x100, 1, 0)
    cache = Cache.invalidate(cache)

    assert Enum.all?(cache.sets, fn cache_set ->
             Enum.all?(cache_set.lines, &(not &1.valid))
           end)
  end

  test "stats reset and rates behave as expected" do
    stats = %CacheStats{} |> CacheStats.record_read(hit: true) |> CacheStats.record_write(hit: false)
    assert CacheStats.total_accesses(stats) == 2
    assert_in_delta CacheStats.hit_rate(stats), 0.5, 1.0e-9
    assert_in_delta CacheStats.miss_rate(stats), 0.5, 1.0e-9

    stats = CacheStats.reset(stats)
    assert stats == %CacheStats{}
  end

  test "hierarchy fills upper levels after lower-level hit" do
    l1d = CacheConfig.new!(name: "L1D", total_size: 256, line_size: 64, associativity: 2, access_latency: 1) |> Cache.new()
    l2 = CacheConfig.new!(name: "L2", total_size: 512, line_size: 64, associativity: 2, access_latency: 10) |> Cache.new()
    {l2, _} = Cache.fill_line(l2, 0x1000, List.duplicate(0, 64), 0)
    hierarchy = CacheHierarchy.new(l1d: l1d, l2: l2, main_memory_latency: 100)

    {hierarchy, first} = CacheHierarchy.read(hierarchy, 0x1000, false, 1)
    assert first.served_by == "L2"
    assert first.total_cycles == 11

    {hierarchy, second} = CacheHierarchy.read(hierarchy, 0x1000, false, 2)
    assert second.served_by == "L1D"
    assert second.total_cycles == 1

    assert hierarchy.l1d.stats.hits >= 1
  end

  test "hierarchy write miss reports the serving level and can reset state" do
    l1d =
      CacheConfig.new!(name: "L1D", total_size: 256, line_size: 64, associativity: 2, access_latency: 1)
      |> Cache.new()

    l2 =
      CacheConfig.new!(name: "L2", total_size: 512, line_size: 64, associativity: 2, access_latency: 10)
      |> Cache.new()

    {l2, _} = Cache.fill_line(l2, 0x300, List.duplicate(0, 64), 0)
    hierarchy = CacheHierarchy.new(l1d: l1d, l2: l2, main_memory_latency: 100)

    {hierarchy, access} = CacheHierarchy.write(hierarchy, 0x300, [0xAA], 1)
    assert access.served_by == "L2"
    assert access.total_cycles == 11

    hierarchy = CacheHierarchy.invalidate_all(hierarchy)
    assert Enum.all?(hierarchy.l1d.sets, fn cache_set -> Enum.all?(cache_set.lines, &(not &1.valid)) end)

    hierarchy = CacheHierarchy.reset_stats(hierarchy)
    assert hierarchy.l1d.stats == %CacheStats{}
    assert hierarchy.l2.stats == %CacheStats{}
  end

  test "instruction reads use L1I when present" do
    l1i = CacheConfig.new!(name: "L1I", total_size: 256, line_size: 64, associativity: 2, access_latency: 2) |> Cache.new()
    hierarchy = CacheHierarchy.new(l1i: l1i, main_memory_latency: 50)

    {hierarchy, first} = CacheHierarchy.read(hierarchy, 0x200, true, 0)
    assert first.served_by == "memory"
    assert first.total_cycles == 52

    {_hierarchy, second} = CacheHierarchy.read(hierarchy, 0x200, true, 1)
    assert second.served_by == "L1I"
    assert second.total_cycles == 2
  end
end
