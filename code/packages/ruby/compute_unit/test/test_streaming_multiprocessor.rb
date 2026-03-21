# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Tests for the NVIDIA StreamingMultiprocessor (SM) simulator.
# ---------------------------------------------------------------------------

class TestStreamingMultiprocessor < Minitest::Test
  include CodingAdventures

  def setup
    @clock = Clock::ClockGenerator.new
    # Use small configs for fast testing
    @config = ComputeUnit::SMConfig.new(
      num_schedulers: 2,
      warp_width: 4,
      max_warps: 8,
      max_threads: 32,
      max_blocks: 4,
      scheduling_policy: :gto,
      register_file_size: 4096,
      shared_memory_size: 4096,
      memory_latency_cycles: 5
    )
    @sm = ComputeUnit::StreamingMultiprocessor.new(@config, @clock)
  end

  # --- Properties ---

  def test_name
    assert_equal "SM", @sm.name
  end

  def test_architecture
    assert_equal :nvidia_sm, @sm.architecture
  end

  def test_idle_when_empty
    assert @sm.idle?
  end

  def test_occupancy_when_empty
    assert_in_delta 0.0, @sm.occupancy
  end

  # --- Simple dispatch and run ---

  def test_dispatch_and_run_simple_program
    # A simple 4-instruction program: load immediate, multiply, halt
    prog = [GpuCore.limm(0, 2.0), GpuCore.limm(1, 3.0), GpuCore.fmul(2, 0, 1), GpuCore.halt]
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      program: prog,
      thread_count: 4,  # 1 warp
      registers_per_thread: 8
    )

    @sm.dispatch(work)
    refute @sm.idle?
    assert_operator @sm.occupancy, :>, 0.0

    traces = @sm.run
    assert @sm.idle?
    refute_empty traces

    # Each trace should be a ComputeUnitTrace
    traces.each do |trace|
      assert_equal "SM", trace.unit_name
      assert_equal :nvidia_sm, trace.architecture
    end
  end

  # --- Thread block decomposition ---

  def test_thread_block_decomposed_into_warps
    # 8 threads with warp_width=4 should create 2 warps
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      program: prog,
      thread_count: 8,
      registers_per_thread: 8
    )

    @sm.dispatch(work)
    assert_equal 2, @sm.warp_slots.length
  end

  # --- Partial warp (thread count not multiple of warp width) ---

  def test_partial_warp
    # 6 threads with warp_width=4 should create 2 warps (4 + 2)
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      program: prog,
      thread_count: 6,
      registers_per_thread: 8
    )

    @sm.dispatch(work)
    assert_equal 2, @sm.warp_slots.length
    traces = @sm.run
    assert @sm.idle?
  end

  # --- Per-thread data ---

  def test_per_thread_data
    prog = [GpuCore.limm(1, 10.0), GpuCore.fadd(2, 0, 1), GpuCore.halt]
    per_thread = {
      0 => {0 => 1.0},
      1 => {0 => 2.0},
      2 => {0 => 3.0},
      3 => {0 => 4.0}
    }
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      program: prog,
      thread_count: 4,
      per_thread_data: per_thread,
      registers_per_thread: 8
    )

    @sm.dispatch(work)
    traces = @sm.run
    assert @sm.idle?

    # Verify per-thread results: each thread should have R2 = R0 + 10.0
    slot = @sm.warp_slots[0]
    assert_in_delta 11.0, slot.engine.threads[0].core.registers.read_float(2), 0.01
    assert_in_delta 12.0, slot.engine.threads[1].core.registers.read_float(2), 0.01
  end

  # --- Occupancy calculation ---

  def test_occupancy_increases_with_dispatch
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    work1 = ComputeUnit::WorkItem.new(work_id: 0, program: prog, thread_count: 4, registers_per_thread: 8)
    work2 = ComputeUnit::WorkItem.new(work_id: 1, program: prog, thread_count: 4, registers_per_thread: 8)

    @sm.dispatch(work1)
    occ1 = @sm.occupancy

    @sm.dispatch(work2)
    occ2 = @sm.occupancy

    assert_operator occ2, :>, occ1
  end

  # --- Theoretical occupancy calculation ---

  def test_compute_occupancy_register_limited
    # 64 regs/thread * 4 (warp_width) = 256 regs/warp
    # 4096 total / 256 = 16 warps. 16/8 max = 200% -> capped at 1.0
    occ = @sm.compute_occupancy(
      registers_per_thread: 64,
      shared_mem_per_block: 0,
      threads_per_block: 4
    )
    assert_in_delta 1.0, occ
  end

  def test_compute_occupancy_shared_memory_limited
    # 4096 bytes total. 2048 per block -> 2 blocks. 1 warp each -> 2 warps / 8 = 25%
    occ = @sm.compute_occupancy(
      registers_per_thread: 8,
      shared_mem_per_block: 2048,
      threads_per_block: 4
    )
    assert_in_delta 0.25, occ
  end

  # --- Resource exhaustion ---

  def test_resource_error_too_many_warps
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]

    # Fill all 8 warp slots
    8.times do |i|
      @sm.dispatch(ComputeUnit::WorkItem.new(
        work_id: i, program: prog, thread_count: 4, registers_per_thread: 8
      ))
    end

    # 9th dispatch should fail
    assert_raises(ComputeUnit::ResourceError) do
      @sm.dispatch(ComputeUnit::WorkItem.new(
        work_id: 99, program: prog, thread_count: 4, registers_per_thread: 8
      ))
    end
  end

  def test_resource_error_shared_memory
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]

    # Request more shared memory than available
    assert_raises(ComputeUnit::ResourceError) do
      @sm.dispatch(ComputeUnit::WorkItem.new(
        work_id: 0, program: prog, thread_count: 4,
        registers_per_thread: 8, shared_mem_bytes: 8192
      ))
    end
  end

  def test_resource_error_registers
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]

    # Request way too many registers per thread
    assert_raises(ComputeUnit::ResourceError) do
      @sm.dispatch(ComputeUnit::WorkItem.new(
        work_id: 0, program: prog, thread_count: 4,
        registers_per_thread: 2000
      ))
    end
  end

  # --- Scheduling ---

  def test_scheduler_distributes_warps_round_robin
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    # 4 warps, 2 schedulers -> each gets 2 warps
    work = ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 16, registers_per_thread: 8
    )
    @sm.dispatch(work)
    assert_equal 4, @sm.warp_slots.length
  end

  # --- Reset ---

  def test_reset_clears_state
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    @sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4, registers_per_thread: 8
    ))
    @sm.run

    @sm.reset
    assert @sm.idle?
    assert_in_delta 0.0, @sm.occupancy
    assert_empty @sm.warp_slots
  end

  # --- Trace contents ---

  def test_trace_scheduler_action_present
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    @sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4, registers_per_thread: 8
    ))
    traces = @sm.run
    first_trace = traces[0]
    refute_empty first_trace.scheduler_action
  end

  # --- Scheduling policies ---

  def test_round_robin_scheduling
    config = ComputeUnit::SMConfig.new(
      num_schedulers: 1, warp_width: 4, max_warps: 8,
      scheduling_policy: :round_robin, register_file_size: 4096,
      shared_memory_size: 4096
    )
    sm = ComputeUnit::StreamingMultiprocessor.new(config, @clock)

    prog = [GpuCore.limm(0, 1.0), GpuCore.limm(1, 2.0), GpuCore.halt]
    sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 8, registers_per_thread: 8
    ))

    traces = sm.run
    assert sm.idle?
  end

  def test_lrr_scheduling
    config = ComputeUnit::SMConfig.new(
      num_schedulers: 1, warp_width: 4, max_warps: 8,
      scheduling_policy: :lrr, register_file_size: 4096,
      shared_memory_size: 4096
    )
    sm = ComputeUnit::StreamingMultiprocessor.new(config, @clock)

    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4, registers_per_thread: 8
    ))

    traces = sm.run
    assert sm.idle?
  end

  def test_oldest_first_scheduling
    config = ComputeUnit::SMConfig.new(
      num_schedulers: 1, warp_width: 4, max_warps: 8,
      scheduling_policy: :oldest_first, register_file_size: 4096,
      shared_memory_size: 4096
    )
    sm = ComputeUnit::StreamingMultiprocessor.new(config, @clock)

    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 8, registers_per_thread: 8
    ))

    traces = sm.run
    assert sm.idle?
  end

  # --- Multiple dispatches ---

  def test_multiple_work_items
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    @sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4, registers_per_thread: 8
    ))
    @sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 1, program: prog, thread_count: 4, registers_per_thread: 8
    ))

    traces = @sm.run
    assert @sm.idle?
    assert_equal 2, @sm.warp_slots.length
  end

  # --- to_s ---

  def test_to_s
    str = @sm.to_s
    assert_includes str, "StreamingMultiprocessor"
    assert_includes str, "warps="
    assert_includes str, "occupancy="
  end
end
