# frozen_string_literal: true

require_relative "test_helper"

# ---------------------------------------------------------------------------
# Cross-architecture tests: verify that the same computation produces
# the same results across different compute unit architectures.
# ---------------------------------------------------------------------------

class TestCrossArchitecture < Minitest::Test
  include CodingAdventures

  def setup
    @clock = Clock::ClockGenerator.new
  end

  # --- Same program, SM vs CU ---

  def test_sm_and_cu_produce_same_results
    prog = [GpuCore.limm(0, 2.0), GpuCore.limm(1, 3.0), GpuCore.fmul(2, 0, 1), GpuCore.halt]

    # NVIDIA SM
    sm = ComputeUnit::StreamingMultiprocessor.new(
      ComputeUnit::SMConfig.new(
        num_schedulers: 1, warp_width: 4, max_warps: 4,
        register_file_size: 4096, shared_memory_size: 4096
      ),
      @clock
    )
    sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4, registers_per_thread: 8
    ))
    sm.run

    sm_result = sm.warp_slots[0].engine.threads[0].core.registers.read_float(2)

    # AMD CU
    cu = ComputeUnit::AMDComputeUnit.new(
      ComputeUnit::AMDCUConfig.new(
        num_simd_units: 1, wave_width: 4, max_wavefronts: 4,
        lds_size: 4096
      ),
      @clock
    )
    cu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4
    ))
    cu.run

    # Read from the wavefront engine's internal lane core
    cu_result = cu.wavefront_slots[0].engine.vrf.read(2, 0)

    # Both should compute 2.0 * 3.0 = 6.0
    assert_in_delta 6.0, sm_result, 0.01
    assert_in_delta 6.0, cu_result, 0.01
  end

  # --- Same matmul, MXU vs ANE ---

  def test_mxu_and_ane_produce_same_matmul
    input_data = [[1.0, 2.0], [3.0, 4.0]]
    weight_data = [[5.0, 6.0], [7.0, 8.0]]

    # Google MXU
    mxu = ComputeUnit::MatrixMultiplyUnit.new(
      ComputeUnit::MXUConfig.new(array_rows: 4, array_cols: 4),
      @clock
    )
    mxu_result = mxu.run_matmul(activations: input_data, weights: weight_data)

    # Apple ANE
    ane = ComputeUnit::NeuralEngineCore.new(
      ComputeUnit::ANECoreConfig.new(num_macs: 4),
      @clock
    )
    ane_result = ane.run_inference(inputs: input_data, weights: weight_data, activation_fn: "none")

    # Both should produce [[19, 22], [43, 50]]
    assert_in_delta mxu_result[0][0], ane_result[0][0], 0.1
    assert_in_delta mxu_result[0][1], ane_result[0][1], 0.1
    assert_in_delta mxu_result[1][0], ane_result[1][0], 0.1
    assert_in_delta mxu_result[1][1], ane_result[1][1], 0.1
  end

  # --- All compute units share the ComputeUnitTrace interface ---

  def test_all_units_produce_compute_unit_traces
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]

    units = []

    # SM
    sm = ComputeUnit::StreamingMultiprocessor.new(
      ComputeUnit::SMConfig.new(
        num_schedulers: 1, warp_width: 4, max_warps: 4,
        register_file_size: 4096, shared_memory_size: 4096
      ),
      @clock
    )
    sm.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4, registers_per_thread: 8
    ))
    units << sm

    # CU
    cu = ComputeUnit::AMDComputeUnit.new(
      ComputeUnit::AMDCUConfig.new(
        num_simd_units: 1, wave_width: 4, max_wavefronts: 4,
        lds_size: 4096
      ),
      @clock
    )
    cu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 4
    ))
    units << cu

    # MXU
    mxu = ComputeUnit::MatrixMultiplyUnit.new(
      ComputeUnit::MXUConfig.new(array_rows: 4, array_cols: 4),
      @clock
    )
    mxu.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, input_data: [[1.0]], weight_data: [[2.0]]
    ))
    units << mxu

    # XeCore
    xe = ComputeUnit::XeCore.new(
      ComputeUnit::XeCoreConfig.new(
        num_eus: 2, threads_per_eu: 2, simd_width: 4,
        slm_size: 4096
      ),
      @clock
    )
    xe.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, program: prog, thread_count: 8
    ))
    units << xe

    # ANE
    ane = ComputeUnit::NeuralEngineCore.new(
      ComputeUnit::ANECoreConfig.new(num_macs: 4),
      @clock
    )
    ane.dispatch(ComputeUnit::WorkItem.new(
      work_id: 0, input_data: [[1.0]], weight_data: [[2.0]]
    ))
    units << ane

    # All should run and produce traces
    units.each do |unit|
      traces = unit.run
      refute_empty traces, "#{unit.name} should produce traces"

      traces.each do |trace|
        assert_instance_of ComputeUnit::ComputeUnitTrace, trace
        refute_nil trace.cycle
        refute_nil trace.unit_name
        refute_nil trace.architecture
        refute_nil trace.scheduler_action
        refute_nil trace.occupancy
      end

      assert unit.idle?, "#{unit.name} should be idle after run"
    end
  end

  # --- All compute units support reset ---

  def test_all_units_support_reset
    prog = [GpuCore.limm(0, 1.0), GpuCore.halt]

    sm = ComputeUnit::StreamingMultiprocessor.new(
      ComputeUnit::SMConfig.new(
        num_schedulers: 1, warp_width: 4, max_warps: 4,
        register_file_size: 4096, shared_memory_size: 4096
      ),
      @clock
    )
    cu = ComputeUnit::AMDComputeUnit.new(
      ComputeUnit::AMDCUConfig.new(
        num_simd_units: 1, wave_width: 4, max_wavefronts: 4,
        lds_size: 4096
      ),
      @clock
    )
    mxu = ComputeUnit::MatrixMultiplyUnit.new(
      ComputeUnit::MXUConfig.new(array_rows: 4, array_cols: 4),
      @clock
    )
    xe = ComputeUnit::XeCore.new(
      ComputeUnit::XeCoreConfig.new(
        num_eus: 2, threads_per_eu: 2, simd_width: 4,
        slm_size: 4096
      ),
      @clock
    )
    ane = ComputeUnit::NeuralEngineCore.new(
      ComputeUnit::ANECoreConfig.new(num_macs: 4),
      @clock
    )

    [sm, cu, mxu, xe, ane].each do |unit|
      unit.reset
      assert unit.idle?, "#{unit.name} should be idle after reset"
    end
  end
end
