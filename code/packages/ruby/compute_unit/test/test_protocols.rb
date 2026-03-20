# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Tests for the shared protocol types: Architecture, WarpState,
# SchedulingPolicy, WorkItem, ComputeUnitTrace, SharedMemory.
# ---------------------------------------------------------------------------

class TestArchitecture < Minitest::Test
  # Verify all five vendor architectures are defined.
  def test_all_architectures_present
    expected = %i[nvidia_sm amd_cu google_mxu intel_xe_core apple_ane_core]
    expected.each do |arch|
      assert_includes CodingAdventures::ComputeUnit::ARCHITECTURES, arch
    end
  end

  def test_architectures_frozen
    assert CodingAdventures::ComputeUnit::ARCHITECTURES.frozen?
  end
end

class TestWarpState < Minitest::Test
  # Verify all warp states are defined.
  def test_all_warp_states_present
    expected = %i[ready running stalled_memory stalled_barrier stalled_dependency completed]
    expected.each do |state|
      assert_includes CodingAdventures::ComputeUnit::WARP_STATES, state
    end
  end
end

class TestSchedulingPolicy < Minitest::Test
  # Verify all scheduling policies are defined.
  def test_all_policies_present
    expected = %i[round_robin greedy oldest_first gto lrr]
    expected.each do |policy|
      assert_includes CodingAdventures::ComputeUnit::SCHEDULING_POLICIES, policy
    end
  end
end

class TestWorkItem < Minitest::Test
  include CodingAdventures

  # A WorkItem with just an ID should use sensible defaults.
  def test_default_values
    wi = ComputeUnit::WorkItem.new(work_id: 0)
    assert_equal 0, wi.work_id
    assert_nil wi.program
    assert_equal 32, wi.thread_count
    assert_equal({}, wi.per_thread_data)
    assert_nil wi.input_data
    assert_nil wi.weight_data
    assert_nil wi.schedule
    assert_equal 0, wi.shared_mem_bytes
    assert_equal 32, wi.registers_per_thread
  end

  # A WorkItem with a program and threads set.
  def test_custom_values
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    wi = ComputeUnit::WorkItem.new(
      work_id: 42,
      program: prog,
      thread_count: 64,
      shared_mem_bytes: 4096,
      registers_per_thread: 16
    )
    assert_equal 42, wi.work_id
    assert_equal 64, wi.thread_count
    assert_equal 4096, wi.shared_mem_bytes
    assert_equal 16, wi.registers_per_thread
  end

  # A WorkItem for dataflow architectures (TPU/NPU).
  def test_dataflow_work_item
    wi = ComputeUnit::WorkItem.new(
      work_id: 1,
      input_data: [[1.0, 2.0], [3.0, 4.0]],
      weight_data: [[5.0, 6.0], [7.0, 8.0]]
    )
    assert_equal [[1.0, 2.0], [3.0, 4.0]], wi.input_data
    assert_equal [[5.0, 6.0], [7.0, 8.0]], wi.weight_data
  end

  # WorkItem is immutable (Data.define).
  def test_frozen
    wi = ComputeUnit::WorkItem.new(work_id: 0)
    assert wi.frozen?
  end
end

class TestComputeUnitTrace < Minitest::Test
  include CodingAdventures

  def setup
    @trace = ComputeUnit::ComputeUnitTrace.new(
      cycle: 5,
      unit_name: "SM",
      architecture: :nvidia_sm,
      scheduler_action: "S0: issued warp 3",
      active_warps: 48,
      total_warps: 64,
      engine_traces: {},
      shared_memory_used: 49_152,
      shared_memory_total: 98_304,
      register_file_used: 32_768,
      register_file_total: 65_536,
      occupancy: 0.75
    )
  end

  def test_basic_fields
    assert_equal 5, @trace.cycle
    assert_equal "SM", @trace.unit_name
    assert_equal :nvidia_sm, @trace.architecture
    assert_equal 48, @trace.active_warps
    assert_in_delta 0.75, @trace.occupancy
  end

  def test_default_cache_stats
    assert_equal 0, @trace.l1_hits
    assert_equal 0, @trace.l1_misses
  end

  def test_format
    formatted = @trace.format
    assert_includes formatted, "[Cycle 5]"
    assert_includes formatted, "SM"
    assert_includes formatted, "75.0%"
    assert_includes formatted, "48/64"
    assert_includes formatted, "Scheduler: S0: issued warp 3"
    assert_includes formatted, "Shared memory:"
    assert_includes formatted, "Registers:"
  end

  def test_frozen
    assert @trace.frozen?
  end
end

class TestSharedMemory < Minitest::Test
  include CodingAdventures

  def setup
    @smem = ComputeUnit::SharedMemory.new(size: 1024)
  end

  # Basic write and read.
  def test_write_and_read
    @smem.write(0, 3.14, 0)
    val = @smem.read(0, 0)
    assert_in_delta 3.14, val, 0.001
  end

  # Multiple addresses.
  def test_multiple_addresses
    @smem.write(0, 1.0, 0)
    @smem.write(4, 2.0, 1)
    @smem.write(8, 3.0, 2)
    assert_in_delta 1.0, @smem.read(0, 0), 0.001
    assert_in_delta 2.0, @smem.read(4, 1), 0.001
    assert_in_delta 3.0, @smem.read(8, 2), 0.001
  end

  # Out-of-range read.
  def test_read_out_of_range
    assert_raises(IndexError) { @smem.read(1024, 0) }
    assert_raises(IndexError) { @smem.read(-1, 0) }
  end

  # Out-of-range write.
  def test_write_out_of_range
    assert_raises(IndexError) { @smem.write(1024, 1.0, 0) }
  end

  # Bank conflict detection: no conflicts.
  def test_no_bank_conflicts
    # Each thread accesses a different bank
    addresses = [0, 4, 8, 12]
    conflicts = @smem.check_bank_conflicts(addresses)
    assert_empty conflicts
  end

  # Bank conflict detection: 2-way conflict.
  def test_two_way_bank_conflict
    # Thread 0 -> addr 0 (bank 0), Thread 2 -> addr 128 (bank 0)
    addresses = [0, 4, 128, 12]
    conflicts = @smem.check_bank_conflicts(addresses)
    assert_equal 1, conflicts.length
    assert_includes conflicts[0], 0
    assert_includes conflicts[0], 2
  end

  # Access counting.
  def test_access_counting
    @smem.write(0, 1.0, 0)
    @smem.read(0, 0)
    assert_equal 2, @smem.total_accesses
  end

  # Reset clears data and statistics.
  def test_reset
    @smem.write(0, 42.0, 0)
    @smem.check_bank_conflicts([0, 128])
    @smem.reset
    assert_in_delta 0.0, @smem.read(0, 0), 0.001
    # After reset, accesses = 1 (the read we just did)
    assert_equal 1, @smem.total_accesses
    assert_equal 0, @smem.total_conflicts
  end
end

class TestResourceError < Minitest::Test
  # ResourceError is a proper exception class.
  def test_resource_error_is_standard_error
    assert CodingAdventures::ComputeUnit::ResourceError < StandardError
  end

  def test_resource_error_message
    err = CodingAdventures::ComputeUnit::ResourceError.new("out of registers")
    assert_equal "out of registers", err.message
  end
end
