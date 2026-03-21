# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Tests for the AMD Compute Unit (CU) simulator.
# ---------------------------------------------------------------------------

class TestAMDComputeUnit < Minitest::Test
  include CodingAdventures

  def setup
    @clock = Clock::ClockGenerator.new
    @config = ComputeUnit::AMDCUConfig.new(
      num_simd_units: 2,
      wave_width: 4,
      max_wavefronts: 8,
      max_work_groups: 4,
      scheduling_policy: :lrr,
      vgpr_per_simd: 256,
      sgpr_count: 104,
      lds_size: 4096,
      memory_latency_cycles: 5
    )
    @cu = ComputeUnit::AMDComputeUnit.new(@config, @clock)
  end

  # --- Properties ---

  def test_name
    assert_equal "CU", @cu.name
  end

  def test_architecture
    assert_equal :amd_cu, @cu.architecture
  end

  def test_idle_when_empty
    assert @cu.idle?
  end

  def test_occupancy_when_empty
    assert_in_delta 0.0, @cu.occupancy
  end

  # --- Dispatch and run ---

  def test_dispatch_and_run
    prog = [GpuCore.limm(0, 2.0), GpuCore.limm(1, 3.0), GpuCore.fmul(2, 0, 1), GpuCore.halt]
    work = ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4
    )

    @cu.dispatch(work)
    refute @cu.idle?

    traces = @cu.run
    assert @cu.idle?
    refute_empty traces
  end

  # --- Wavefront decomposition ---

  def test_wavefront_decomposition
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    work = ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 8  # 2 wavefronts
    )
    @cu.dispatch(work)
    assert_equal 2, @cu.wavefront_slots.length
  end

  # --- SIMD unit assignment ---

  def test_simd_unit_round_robin_assignment
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    work = ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 8  # 2 wavefronts
    )
    @cu.dispatch(work)

    # With 2 SIMD units: wave 0 -> SIMD 0, wave 1 -> SIMD 1
    assert_equal 0, @cu.wavefront_slots[0].simd_unit
    assert_equal 1, @cu.wavefront_slots[1].simd_unit
  end

  # --- Per-lane data ---

  def test_per_lane_data
    prog = [GpuCore.limm(1, 10.0), GpuCore.fadd(2, 0, 1), GpuCore.halt]
    per_thread = {0 => {0 => 1.0}, 1 => {0 => 2.0}}
    work = ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4,
      per_thread_data: per_thread
    )

    @cu.dispatch(work)
    @cu.run
    assert @cu.idle?
  end

  # --- Resource exhaustion ---

  def test_resource_error_too_many_wavefronts
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]

    8.times do |i|
      @cu.dispatch(ComputeUnit::WorkItem.new(
        work_id: i, program: prog, thread_count: 4
      ))
    end

    assert_raises(ComputeUnit::ResourceError) do
      @cu.dispatch(ComputeUnit::WorkItem.new(
        work_id: 99, program: prog, thread_count: 4
      ))
    end
  end

  def test_resource_error_lds
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]

    assert_raises(ComputeUnit::ResourceError) do
      @cu.dispatch(ComputeUnit::WorkItem.new(
        work_id: 0, program: prog, thread_count: 4,
        shared_mem_bytes: 8192
      ))
    end
  end

  # --- Trace contents ---

  def test_trace_architecture
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    @cu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4
    ))
    traces = @cu.run
    traces.each do |trace|
      assert_equal "CU", trace.unit_name
      assert_equal :amd_cu, trace.architecture
    end
  end

  # --- Occupancy changes ---

  def test_occupancy_changes
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]

    @cu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4
    ))
    occ1 = @cu.occupancy

    @cu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 1, program: prog, thread_count: 4
    ))
    occ2 = @cu.occupancy

    assert_operator occ2, :>, occ1
  end

  # --- Reset ---

  def test_reset
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    @cu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4
    ))
    @cu.run
    @cu.reset

    assert @cu.idle?
    assert_in_delta 0.0, @cu.occupancy
    assert_empty @cu.wavefront_slots
  end

  # --- to_s ---

  def test_to_s
    str = @cu.to_s
    assert_includes str, "AMDComputeUnit"
    assert_includes str, "waves="
  end
end
