# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module ParallelExecutionEngine
    # =========================================================================
    # Tests for protocols -- ExecutionModel symbols, EngineTrace, DivergenceInfo.
    # =========================================================================

    # -----------------------------------------------------------------------
    # ExecutionModel symbols
    # -----------------------------------------------------------------------

    class TestExecutionModels < Minitest::Test
      # All five execution models should be defined.
      def test_all_five_models_exist
        assert_includes EXECUTION_MODELS, :simt
        assert_includes EXECUTION_MODELS, :simd
        assert_includes EXECUTION_MODELS, :systolic
        assert_includes EXECUTION_MODELS, :scheduled_mac
        assert_includes EXECUTION_MODELS, :vliw
      end

      # Exactly 5 execution models.
      def test_model_count
        assert_equal 5, EXECUTION_MODELS.length
      end

      # All models are symbols.
      def test_models_are_symbols
        EXECUTION_MODELS.each do |model|
          assert_kind_of Symbol, model
        end
      end

      # The list is frozen (immutable).
      def test_frozen
        assert EXECUTION_MODELS.frozen?
      end
    end

    # -----------------------------------------------------------------------
    # DivergenceInfo
    # -----------------------------------------------------------------------

    class TestDivergenceInfo < Minitest::Test
      # Create a DivergenceInfo with basic fields.
      def test_creation
        info = DivergenceInfo.new(
          active_mask_before: [true, true, true, true],
          active_mask_after: [true, true, false, false],
          reconvergence_pc: 10,
          divergence_depth: 1
        )
        assert_equal [true, true, true, true], info.active_mask_before
        assert_equal [true, true, false, false], info.active_mask_after
        assert_equal 10, info.reconvergence_pc
        assert_equal 1, info.divergence_depth
      end

      # Default reconvergence_pc is -1, depth is 0.
      def test_defaults
        info = DivergenceInfo.new(
          active_mask_before: [true],
          active_mask_after: [true]
        )
        assert_equal(-1, info.reconvergence_pc)
        assert_equal 0, info.divergence_depth
      end

      # DivergenceInfo is immutable (Data.define).
      def test_frozen
        info = DivergenceInfo.new(
          active_mask_before: [true],
          active_mask_after: [false]
        )
        assert_raises(FrozenError) { info.instance_variable_set(:@reconvergence_pc, 5) }
      end
    end

    # -----------------------------------------------------------------------
    # DataflowInfo
    # -----------------------------------------------------------------------

    class TestDataflowInfo < Minitest::Test
      # Create DataflowInfo with PE states.
      def test_creation
        info = DataflowInfo.new(
          pe_states: [["acc=1.0", "acc=2.0"], ["acc=3.0", "acc=4.0"]],
          data_positions: {"input_0" => [0, 1]}
        )
        assert_equal "acc=1.0", info.pe_states[0][0]
        assert_equal [0, 1], info.data_positions["input_0"]
      end

      # Default data_positions is empty hash.
      def test_defaults
        info = DataflowInfo.new(pe_states: [["x"]])
        assert_equal({}, info.data_positions)
      end

      # DataflowInfo is immutable.
      def test_frozen
        info = DataflowInfo.new(pe_states: [[]])
        assert_raises(FrozenError) { info.instance_variable_set(:@pe_states, []) }
      end
    end

    # -----------------------------------------------------------------------
    # EngineTrace
    # -----------------------------------------------------------------------

    class TestEngineTrace < Minitest::Test
      def make_trace
        EngineTrace.new(
          cycle: 3,
          engine_name: "WarpEngine",
          execution_model: :simt,
          description: "FADD R2, R0, R1 -- 3/4 threads active",
          unit_traces: {
            0 => "R2 = 1.0 + 2.0 = 3.0",
            1 => "R2 = 3.0 + 4.0 = 7.0",
            2 => "(masked)",
            3 => "R2 = 5.0 + 6.0 = 11.0"
          },
          active_mask: [true, true, false, true],
          active_count: 3,
          total_count: 4,
          utilization: 0.75
        )
      end

      # Create an EngineTrace with all fields.
      def test_creation
        trace = make_trace
        assert_equal 3, trace.cycle
        assert_equal "WarpEngine", trace.engine_name
        assert_equal :simt, trace.execution_model
        assert_equal 3, trace.active_count
        assert_equal 4, trace.total_count
        assert_equal 0.75, trace.utilization
      end

      # Optional divergence_info and dataflow_info default to nil.
      def test_optional_fields
        trace = make_trace
        assert_nil trace.divergence_info
        assert_nil trace.dataflow_info
      end

      # EngineTrace can include divergence info.
      def test_with_divergence_info
        div = DivergenceInfo.new(
          active_mask_before: [true] * 4,
          active_mask_after: [true, true, false, false],
          reconvergence_pc: 10,
          divergence_depth: 1
        )
        trace = EngineTrace.new(
          cycle: 1, engine_name: "WarpEngine", execution_model: :simt,
          description: "branch", unit_traces: {},
          active_mask: [true, true, false, false],
          active_count: 2, total_count: 4, utilization: 0.5,
          divergence_info: div
        )
        refute_nil trace.divergence_info
        assert_equal 1, trace.divergence_info.divergence_depth
      end

      # EngineTrace can include dataflow info.
      def test_with_dataflow_info
        df = DataflowInfo.new(pe_states: [["acc=0.0"]])
        trace = EngineTrace.new(
          cycle: 1, engine_name: "SystolicArray", execution_model: :systolic,
          description: "step", unit_traces: {},
          active_mask: [true], active_count: 1, total_count: 1,
          utilization: 1.0, dataflow_info: df
        )
        refute_nil trace.dataflow_info
      end

      # format returns readable output.
      def test_format
        trace = make_trace
        text = trace.format
        assert_includes text, "Cycle 3"
        assert_includes text, "WarpEngine"
        assert_includes text, "SIMT"
        assert_includes text, "75.0%"
        assert_includes text, "3/4 active"
      end

      # format includes divergence info when present.
      def test_format_with_divergence
        div = DivergenceInfo.new(
          active_mask_before: [true] * 4,
          active_mask_after: [true, true, false, false],
          reconvergence_pc: 10,
          divergence_depth: 1
        )
        trace = EngineTrace.new(
          cycle: 1, engine_name: "Test", execution_model: :simt,
          description: "test", unit_traces: {},
          active_mask: [true, true, false, false],
          active_count: 2, total_count: 4, utilization: 0.5,
          divergence_info: div
        )
        text = trace.format
        assert_includes text, "Divergence"
        assert_includes text, "depth=1"
      end

      # EngineTrace is immutable.
      def test_frozen
        trace = make_trace
        assert_raises(FrozenError) { trace.instance_variable_set(:@cycle, 99) }
      end
    end

    # -----------------------------------------------------------------------
    # ParallelExecutionEngine duck type
    # -----------------------------------------------------------------------

    class TestParallelExecutionEngineDuckType < Minitest::Test
      # WarpEngine satisfies the duck type.
      def test_warp_engine_duck_type
        clock = Clock::ClockGenerator.new
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), clock)
        assert_respond_to engine, :name
        assert_respond_to engine, :width
        assert_respond_to engine, :execution_model
        assert_respond_to engine, :step
        assert_respond_to engine, :halted?
        assert_respond_to engine, :reset
      end

      # WavefrontEngine satisfies the duck type.
      def test_wavefront_engine_duck_type
        clock = Clock::ClockGenerator.new
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), clock)
        assert_respond_to engine, :name
        assert_respond_to engine, :width
        assert_respond_to engine, :execution_model
        assert_respond_to engine, :step
        assert_respond_to engine, :halted?
        assert_respond_to engine, :reset
      end

      # SystolicArray satisfies the duck type.
      def test_systolic_array_duck_type
        clock = Clock::ClockGenerator.new
        engine = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), clock)
        assert_respond_to engine, :name
        assert_respond_to engine, :width
        assert_respond_to engine, :execution_model
        assert_respond_to engine, :step
        assert_respond_to engine, :halted?
        assert_respond_to engine, :reset
      end

      # MACArrayEngine satisfies the duck type.
      def test_mac_array_duck_type
        clock = Clock::ClockGenerator.new
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), clock)
        assert_respond_to engine, :name
        assert_respond_to engine, :width
        assert_respond_to engine, :execution_model
        assert_respond_to engine, :step
        assert_respond_to engine, :halted?
        assert_respond_to engine, :reset
      end

      # SubsliceEngine satisfies the duck type.
      def test_subslice_duck_type
        clock = Clock::ClockGenerator.new
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 2),
          clock
        )
        assert_respond_to engine, :name
        assert_respond_to engine, :width
        assert_respond_to engine, :execution_model
        assert_respond_to engine, :step
        assert_respond_to engine, :halted?
        assert_respond_to engine, :reset
      end
    end
  end
end
