# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Tests for the Intel XeCore simulator.
# ---------------------------------------------------------------------------

class TestXeCore < Minitest::Test
  include CodingAdventures

  def setup
    @clock = Clock::ClockGenerator.new
    @config = ComputeUnit::XeCoreConfig.new(
      num_eus: 2,
      threads_per_eu: 2,
      simd_width: 4,
      grf_per_eu: 32,
      slm_size: 4096
    )
    @xe = ComputeUnit::XeCore.new(@config, @clock)
  end

  # --- Properties ---

  def test_name
    assert_equal "XeCore", @xe.name
  end

  def test_architecture
    assert_equal :intel_xe_core, @xe.architecture
  end

  def test_idle_when_empty
    assert @xe.idle?
  end

  # --- Dispatch and run ---

  def test_dispatch_and_run
    prog = [GpuCore.limm(0, 2.0), GpuCore.limm(1, 3.0), GpuCore.fmul(2, 0, 1), GpuCore.halt]
    work = ComputeUnit::WorkItem.new(
      work_id: 0,
      program: prog,
      thread_count: 16
    )

    @xe.dispatch(work)
    refute @xe.idle?

    traces = @xe.run
    assert @xe.idle?
    refute_empty traces
  end

  # --- Trace contents ---

  def test_trace_fields
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    @xe.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 8
    ))
    traces = @xe.run

    traces.each do |trace|
      assert_equal "XeCore", trace.unit_name
      assert_equal :intel_xe_core, trace.architecture
    end
  end

  # --- SLM access ---

  def test_slm_accessible
    assert_instance_of ComputeUnit::SharedMemory, @xe.slm
    @xe.slm.write(0, 42.0, 0)
    assert_in_delta 42.0, @xe.slm.read(0, 0), 0.01
  end

  # --- Reset ---

  def test_reset
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]
    @xe.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 8
    ))
    @xe.run
    @xe.reset

    assert @xe.idle?
  end

  # --- Per-thread data ---

  def test_per_thread_data_dispatch
    prog = [GpuCore.limm(1, 5.0), GpuCore.fadd(2, 0, 1), GpuCore.halt]
    per_thread = {0 => {0 => 10.0}}
    work = ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 8,
      per_thread_data: per_thread
    )

    @xe.dispatch(work)
    traces = @xe.run
    assert @xe.idle?
  end

  # --- to_s ---

  def test_to_s
    str = @xe.to_s
    assert_includes str, "XeCore"
    assert_includes str, "eus="
  end
end
