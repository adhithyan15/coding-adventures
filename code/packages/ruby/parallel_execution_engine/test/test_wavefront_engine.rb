# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module ParallelExecutionEngine
    # =========================================================================
    # Tests for WavefrontEngine -- SIMD parallel execution (AMD GCN/RDNA style).
    # =========================================================================

    # -----------------------------------------------------------------------
    # VectorRegisterFile
    # -----------------------------------------------------------------------

    class TestVectorRegisterFile < Minitest::Test
      def test_creation
        vrf = VectorRegisterFile.new(num_vgprs: 8, wave_width: 4)
        assert_equal 8, vrf.num_vgprs
        assert_equal 4, vrf.wave_width
      end

      def test_read_write
        vrf = VectorRegisterFile.new(num_vgprs: 8, wave_width: 4)
        vrf.write(0, 2, 3.14)
        assert_in_delta 3.14, vrf.read(0, 2), 0.01
      end

      # Different lanes of the same register are independent.
      def test_lanes_independent
        vrf = VectorRegisterFile.new(num_vgprs: 4, wave_width: 4)
        vrf.write(0, 0, 1.0)
        vrf.write(0, 1, 2.0)
        vrf.write(0, 2, 3.0)
        vrf.write(0, 3, 4.0)
        assert_equal 1.0, vrf.read(0, 0)
        assert_equal 2.0, vrf.read(0, 1)
        assert_equal 3.0, vrf.read(0, 2)
        assert_equal 4.0, vrf.read(0, 3)
      end

      def test_read_all_lanes
        vrf = VectorRegisterFile.new(num_vgprs: 4, wave_width: 4)
        4.times { |lane| vrf.write(0, lane, (lane + 1).to_f) }
        assert_equal [1.0, 2.0, 3.0, 4.0], vrf.read_all_lanes(0)
      end
    end

    # -----------------------------------------------------------------------
    # ScalarRegisterFile
    # -----------------------------------------------------------------------

    class TestScalarRegisterFile < Minitest::Test
      def test_creation
        srf = ScalarRegisterFile.new(num_sgprs: 8)
        assert_equal 8, srf.num_sgprs
      end

      def test_read_write
        srf = ScalarRegisterFile.new(num_sgprs: 8)
        srf.write(3, 42.0)
        assert_equal 42.0, srf.read(3)
      end

      def test_initial_zero
        srf = ScalarRegisterFile.new(num_sgprs: 8)
        assert_equal 0.0, srf.read(0)
      end
    end

    # -----------------------------------------------------------------------
    # WavefrontConfig
    # -----------------------------------------------------------------------

    class TestWavefrontConfig < Minitest::Test
      def test_defaults
        config = WavefrontConfig.new
        assert_equal 32, config.wave_width
        assert_equal 256, config.num_vgprs
        assert_equal 104, config.num_sgprs
        assert_equal 65_536, config.lds_size
      end
    end

    # -----------------------------------------------------------------------
    # WavefrontEngine -- basic properties
    # -----------------------------------------------------------------------

    class TestWavefrontEngineProperties < Minitest::Test
      def test_name
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        assert_equal "WavefrontEngine", engine.name
      end

      def test_width
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 8), Clock::ClockGenerator.new)
        assert_equal 8, engine.width
      end

      def test_execution_model
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        assert_equal :simd, engine.execution_model
      end

      def test_initial_halted
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        refute engine.halted?
      end

      def test_exec_mask_all_true
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        assert_equal [true, true, true, true], engine.exec_mask
      end

      def test_config_access
        config = WavefrontConfig.new(wave_width: 4)
        engine = WavefrontEngine.new(config, Clock::ClockGenerator.new)
        assert_same config, engine.config
      end

      def test_vrf_and_srf_access
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        refute_nil engine.vrf
        refute_nil engine.srf
      end

      def test_to_s
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        r = engine.to_s
        assert_includes r, "WavefrontEngine"
      end
    end

    # -----------------------------------------------------------------------
    # WavefrontEngine -- execution
    # -----------------------------------------------------------------------

    class TestWavefrontEngineExecution < Minitest::Test
      # All lanes execute LIMM + HALT.
      def test_simple_program
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
        traces = engine.run
        assert traces.length >= 2
        assert engine.halted?
      end

      # Each lane gets different input via vector registers.
      def test_per_lane_data
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([
          GpuCore.limm(1, 2.0),
          GpuCore.fmul(2, 0, 1),
          GpuCore.halt
        ])

        4.times { |lane| engine.set_lane_register(lane, 0, (lane + 1).to_f) }
        engine.run

        4.times do |lane|
          result = engine.vrf.read(2, lane)
          assert_equal (lane + 1) * 2.0, result
        end
      end

      def test_lane_register_out_of_range
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        assert_raises(IndexError) { engine.set_lane_register(4, 0, 1.0) }
        assert_raises(IndexError) { engine.set_lane_register(-1, 0, 1.0) }
      end

      def test_scalar_register_out_of_range
        engine = WavefrontEngine.new(
          WavefrontConfig.new(wave_width: 4, num_sgprs: 8),
          Clock::ClockGenerator.new
        )
        assert_raises(IndexError) { engine.set_scalar_register(8, 1.0) }
        assert_raises(IndexError) { engine.set_scalar_register(-1, 1.0) }
      end
    end

    # -----------------------------------------------------------------------
    # WavefrontEngine -- EXEC mask
    # -----------------------------------------------------------------------

    class TestWavefrontEngineExecMask < Minitest::Test
      def test_set_exec_mask
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.set_exec_mask([true, false, true, false])
        assert_equal [true, false, true, false], engine.exec_mask
      end

      def test_exec_mask_wrong_length
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        assert_raises(ArgumentError) { engine.set_exec_mask([true, false]) }
      end

      # Masked lanes should not update their VRF results.
      def test_masked_lanes_dont_update
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 99.0), GpuCore.halt])
        engine.set_exec_mask([true, true, false, false])
        engine.run

        assert_equal 99.0, engine.vrf.read(0, 0)
        assert_equal 99.0, engine.vrf.read(0, 1)
        assert_equal 0.0, engine.vrf.read(0, 2)
        assert_equal 0.0, engine.vrf.read(0, 3)
      end

      # Utilization should reflect the EXEC mask.
      def test_utilization_with_mask
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])
        engine.set_exec_mask([true, true, false, false])

        trace = engine.step(ParallelExecutionEngine.make_edge)
        assert_equal 2, trace.active_count
        assert_in_delta 0.5, trace.utilization, 0.01
      end
    end

    # -----------------------------------------------------------------------
    # WavefrontEngine -- reset
    # -----------------------------------------------------------------------

    class TestWavefrontEngineReset < Minitest::Test
      def test_reset
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 42.0), GpuCore.halt])
        engine.run
        assert engine.halted?

        engine.reset
        refute engine.halted?
        assert_equal [true, true, true, true], engine.exec_mask

        engine.run
        assert engine.halted?
      end
    end

    # -----------------------------------------------------------------------
    # WavefrontEngine -- traces
    # -----------------------------------------------------------------------

    class TestWavefrontEngineTraces < Minitest::Test
      # SIMD traces should include divergence info (the EXEC mask).
      def test_trace_has_divergence_info
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.limm(0, 1.0), GpuCore.halt])

        trace = engine.step(ParallelExecutionEngine.make_edge)
        refute_nil trace.divergence_info
      end

      def test_halted_step
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.halt])
        engine.run

        trace = engine.step(ParallelExecutionEngine.make_edge)
        assert_equal 0, trace.active_count
      end
    end
  end
end
