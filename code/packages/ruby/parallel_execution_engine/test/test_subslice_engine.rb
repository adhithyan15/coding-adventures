# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module ParallelExecutionEngine
    # =========================================================================
    # Tests for SubsliceEngine -- Intel Xe hybrid SIMD execution engine.
    # =========================================================================

    # -----------------------------------------------------------------------
    # SubsliceConfig
    # -----------------------------------------------------------------------

    class TestSubsliceConfig < Minitest::Test
      def test_defaults
        config = SubsliceConfig.new
        assert_equal 8, config.num_eus
        assert_equal 7, config.threads_per_eu
        assert_equal 8, config.simd_width
        assert_equal 128, config.grf_size
        assert_equal 65_536, config.slm_size
      end

      def test_custom
        config = SubsliceConfig.new(num_eus: 4, threads_per_eu: 2, simd_width: 4)
        assert_equal 4, config.num_eus
        assert_equal 2, config.threads_per_eu
        assert_equal 4, config.simd_width
      end
    end

    # -----------------------------------------------------------------------
    # ExecutionUnit
    # -----------------------------------------------------------------------

    class TestExecutionUnit < Minitest::Test
      def test_creation
        config = SubsliceConfig.new(num_eus: 1, threads_per_eu: 2, simd_width: 4)
        eu = ExecutionUnit.new(eu_id: 0, config: config)
        assert_equal 0, eu.eu_id
        assert_equal 2, eu.threads.length
        assert_equal 4, eu.threads[0].length
      end

      def test_load_program
        config = SubsliceConfig.new(num_eus: 1, threads_per_eu: 2, simd_width: 2)
        eu = ExecutionUnit.new(eu_id: 0, config: config)
        eu.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        refute eu.all_halted?
      end

      def test_step
        config = SubsliceConfig.new(num_eus: 1, threads_per_eu: 2, simd_width: 2)
        eu = ExecutionUnit.new(eu_id: 0, config: config)
        eu.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        traces = eu.step
        assert traces.length > 0
      end

      def test_all_halted
        config = SubsliceConfig.new(num_eus: 1, threads_per_eu: 1, simd_width: 2)
        eu = ExecutionUnit.new(eu_id: 0, config: config)
        eu.load_program([GpuCore.halt])
        eu.step
        assert eu.all_halted?
      end

      def test_set_thread_lane_register
        config = SubsliceConfig.new(num_eus: 1, threads_per_eu: 2, simd_width: 2)
        eu = ExecutionUnit.new(eu_id: 0, config: config)
        eu.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        eu.set_thread_lane_register(0, 1, 5, 42.0)
        assert_equal 42.0, eu.threads[0][1].registers.read_float(5)
      end

      def test_reset
        config = SubsliceConfig.new(num_eus: 1, threads_per_eu: 1, simd_width: 2)
        eu = ExecutionUnit.new(eu_id: 0, config: config)
        eu.load_program([GpuCore.halt])
        eu.step
        assert eu.all_halted?

        eu.reset
        refute eu.all_halted?
      end
    end

    # -----------------------------------------------------------------------
    # SubsliceEngine -- basic properties
    # -----------------------------------------------------------------------

    class TestSubsliceEngineProperties < Minitest::Test
      def test_name
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 4),
          Clock::ClockGenerator.new
        )
        assert_equal "SubsliceEngine", engine.name
      end

      def test_width
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 3, simd_width: 4),
          Clock::ClockGenerator.new
        )
        assert_equal 2 * 3 * 4, engine.width  # 24
      end

      def test_execution_model
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 4),
          Clock::ClockGenerator.new
        )
        assert_equal :simd, engine.execution_model
      end

      def test_initial_halted
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 4),
          Clock::ClockGenerator.new
        )
        refute engine.halted?
      end

      def test_config_access
        config = SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 4)
        engine = SubsliceEngine.new(config, Clock::ClockGenerator.new)
        assert_same config, engine.config
      end

      def test_eus_access
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 3, threads_per_eu: 2, simd_width: 4),
          Clock::ClockGenerator.new
        )
        assert_equal 3, engine.eus.length
      end

      def test_to_s
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 4),
          Clock::ClockGenerator.new
        )
        r = engine.to_s
        assert_includes r, "SubsliceEngine"
      end
    end

    # -----------------------------------------------------------------------
    # SubsliceEngine -- execution
    # -----------------------------------------------------------------------

    class TestSubsliceEngineExecution < Minitest::Test
      # All EU threads execute a simple program.
      def test_simple_program
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 2),
          Clock::ClockGenerator.new
        )
        engine.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
        traces = engine.run
        assert traces.length > 0
        assert engine.halted?
      end

      # Set and verify per-lane registers on specific EU/thread.
      def test_per_eu_thread_lane_register
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 2),
          Clock::ClockGenerator.new
        )
        engine.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        engine.set_eu_thread_lane_register(0, 0, 1, 5, 99.0)

        result = engine.eus[0].threads[0][1].registers.read_float(5)
        assert_equal 99.0, result
      end

      def test_step_produces_trace
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 2),
          Clock::ClockGenerator.new
        )
        engine.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])

        trace = engine.step(ParallelExecutionEngine.make_edge)
        assert_equal 1, trace.cycle
        assert_equal "SubsliceEngine", trace.engine_name
        assert_equal 2 * 2 * 2, trace.total_count  # 8
      end

      def test_halted_step
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 1, threads_per_eu: 1, simd_width: 2),
          Clock::ClockGenerator.new
        )
        engine.load_program([GpuCore.halt])
        engine.run
        assert engine.halted?

        trace = engine.step(ParallelExecutionEngine.make_edge)
        assert_equal 0, trace.active_count
        assert_includes trace.description.downcase, "halted"
      end
    end

    # -----------------------------------------------------------------------
    # SubsliceEngine -- reset
    # -----------------------------------------------------------------------

    class TestSubsliceEngineReset < Minitest::Test
      def test_reset
        engine = SubsliceEngine.new(
          SubsliceConfig.new(num_eus: 2, threads_per_eu: 2, simd_width: 2),
          Clock::ClockGenerator.new
        )
        engine.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
        engine.run
        assert engine.halted?

        engine.reset
        refute engine.halted?

        engine.run
        assert engine.halted?
      end
    end
  end
end
