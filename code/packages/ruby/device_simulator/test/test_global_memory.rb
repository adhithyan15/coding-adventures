# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_device_simulator"

# Tests for global memory -- VRAM / HBM simulation.
class TestGlobalMemoryReadWrite < Minitest::Test
  include CodingAdventures

  def test_write_and_read_back
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    mem.write(0, "\x41\x42\x43\x44".b)
    data = mem.read(0, 4)
    assert_equal "\x41\x42\x43\x44".b, data
  end

  def test_read_uninitialized_returns_zeros
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    data = mem.read(0, 8)
    assert_equal "\x00".b * 8, data
  end

  def test_write_float
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    raw = [3.14].pack("e")
    mem.write(0, raw)
    data = mem.read(0, 4)
    result = data.unpack1("e")
    assert_in_delta 3.14, result, 0.01
  end

  def test_read_out_of_range
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 64)
    assert_raises(IndexError) { mem.read(60, 8) }
  end

  def test_write_out_of_range
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 64)
    assert_raises(IndexError) { mem.write(60, "\x00".b * 8) }
  end

  def test_read_negative_address
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 64)
    assert_raises(IndexError) { mem.read(-1, 4) }
  end

  def test_write_negative_address
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 64)
    assert_raises(IndexError) { mem.write(-1, "\x00".b) }
  end

  def test_multiple_writes_at_different_addresses
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    mem.write(0, "\x01\x02".b)
    mem.write(100, "\x03\x04".b)
    assert_equal "\x01\x02".b, mem.read(0, 2)
    assert_equal "\x03\x04".b, mem.read(100, 2)
  end

  def test_overwrite
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    mem.write(0, "\x01\x02".b)
    mem.write(0, "\x03\x04".b)
    assert_equal "\x03\x04".b, mem.read(0, 2)
  end
end

class TestGlobalMemoryAllocation < Minitest::Test
  include CodingAdventures

  def test_allocate_returns_aligned_address
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024 * 1024)
    addr = mem.allocate(256, alignment: 256)
    assert_equal 0, addr % 256
  end

  def test_sequential_allocations_dont_overlap
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024 * 1024)
    a1 = mem.allocate(256)
    a2 = mem.allocate(256)
    assert a2 >= a1 + 256
  end

  def test_allocate_out_of_memory
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 512)
    mem.allocate(256)
    assert_raises(DeviceSimulator::MemoryError) { mem.allocate(512) }
  end

  def test_free_tracked
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    addr = mem.allocate(128)
    mem.free(addr)
    # Double free is a no-op
    mem.free(addr)
  end

  def test_allocate_with_default_alignment
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024 * 1024)
    addr = mem.allocate(64)
    assert_equal 0, addr % 256
  end
end

class TestGlobalMemoryHostTransfers < Minitest::Test
  include CodingAdventures

  def test_copy_from_host
    mem = DeviceSimulator::SimpleGlobalMemory.new(
      capacity: 1024, host_bandwidth: 64.0, host_latency: 100
    )
    cycles = mem.copy_from_host(0, "\x01".b * 128)
    assert cycles > 0
    assert_equal "\x01".b * 4, mem.read(0, 4)
  end

  def test_copy_to_host
    mem = DeviceSimulator::SimpleGlobalMemory.new(
      capacity: 1024, host_bandwidth: 64.0, host_latency: 100
    )
    mem.write(0, "\xAA\xBB\xCC\xDD".b)
    data, cycles = mem.copy_to_host(0, 4)
    assert_equal "\xAA\xBB\xCC\xDD".b, data
    assert cycles > 0
  end

  def test_unified_memory_zero_cost
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024, unified: true)
    cycles = mem.copy_from_host(0, "\x01".b * 256)
    assert_equal 0, cycles

    data, cycles = mem.copy_to_host(0, 256)
    assert_equal 0, cycles
    assert_equal "\x01".b * 256, data
  end

  def test_transfer_stats_tracked
    mem = DeviceSimulator::SimpleGlobalMemory.new(
      capacity: 1024, host_bandwidth: 64.0, host_latency: 10
    )
    mem.copy_from_host(0, "\x00".b * 128)
    stats = mem.stats
    assert_equal 128, stats.host_to_device_bytes
    assert stats.host_transfer_cycles > 0
  end

  def test_device_to_host_stats
    mem = DeviceSimulator::SimpleGlobalMemory.new(
      capacity: 1024, host_bandwidth: 64.0, host_latency: 10
    )
    mem.write(0, "\x00".b * 64)
    mem.copy_to_host(0, 64)
    stats = mem.stats
    assert_equal 64, stats.device_to_host_bytes
  end
end

class TestGlobalMemoryCoalescing < Minitest::Test
  include CodingAdventures

  def test_fully_coalesced_access
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024, transaction_size: 128)
    addrs = (0...32).map { |i| i * 4 }
    transactions = mem.coalesce(addrs)
    assert_equal 1, transactions.length
    assert_equal 128, transactions[0].size
    assert_equal 0, transactions[0].address
  end

  def test_scattered_access_many_transactions
    mem = DeviceSimulator::SimpleGlobalMemory.new(
      capacity: 1024 * 1024, transaction_size: 128
    )
    addrs = (0...4).map { |i| i * 512 }
    transactions = mem.coalesce(addrs)
    assert_equal 4, transactions.length
  end

  def test_two_transactions_for_strided
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024, transaction_size: 128)
    addrs = (0...32).map { |i| i * 4 }
    addrs += (0...32).map { |i| 128 + i * 4 }
    transactions = mem.coalesce(addrs)
    assert_equal 2, transactions.length
  end

  def test_thread_mask_correct
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024, transaction_size: 128)
    addrs = [0, 4, 256]
    transactions = mem.coalesce(addrs)
    assert_equal 2, transactions.length
    first = transactions.find { |t| t.address == 0 }
    assert_equal 0b11, first.thread_mask & 0b11
  end

  def test_coalescing_stats
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024, transaction_size: 128)
    mem.coalesce((0...32).map { |i| i * 4 })
    stats = mem.stats
    assert_equal 32, stats.total_requests
    assert_equal 1, stats.total_transactions
    assert_in_delta 32.0, stats.coalescing_efficiency, 0.001
  end
end

class TestGlobalMemoryPartitionConflicts < Minitest::Test
  include CodingAdventures

  def test_no_partition_conflict
    mem = DeviceSimulator::SimpleGlobalMemory.new(
      capacity: 1024, channels: 4, transaction_size: 128
    )
    addrs = (0...4).map { |i| i * 128 }
    mem.coalesce(addrs)
    stats = mem.stats
    assert_equal 0, stats.partition_conflicts
  end

  def test_partition_conflict_detected
    mem = DeviceSimulator::SimpleGlobalMemory.new(
      capacity: 4096, channels: 4, transaction_size: 128
    )
    addrs = [0, 512]
    mem.coalesce(addrs)
    stats = mem.stats
    assert stats.partition_conflicts >= 1
  end
end

class TestGlobalMemoryReset < Minitest::Test
  include CodingAdventures

  def test_reset_clears_data
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    mem.write(0, "\xFF".b * 4)
    mem.reset
    assert_equal "\x00".b * 4, mem.read(0, 4)
  end

  def test_reset_clears_stats
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    mem.write(0, "\x00".b)
    mem.read(0, 1)
    mem.reset
    stats = mem.stats
    assert_equal 0, stats.total_reads
    assert_equal 0, stats.total_writes
  end

  def test_reset_clears_allocations
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024)
    mem.allocate(512)
    mem.reset
    addr = mem.allocate(512)
    assert_equal 0, addr
  end
end

class TestGlobalMemoryProperties < Minitest::Test
  include CodingAdventures

  def test_capacity
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 4096)
    assert_equal 4096, mem.capacity
  end

  def test_bandwidth
    mem = DeviceSimulator::SimpleGlobalMemory.new(capacity: 1024, bandwidth: 3350.0)
    assert_in_delta 3350.0, mem.bandwidth, 0.001
  end
end
