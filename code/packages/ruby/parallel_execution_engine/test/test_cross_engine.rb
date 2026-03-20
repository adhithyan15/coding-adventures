# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module ParallelExecutionEngine
    # =========================================================================
    # Cross-engine tests -- verify same computation produces same results
    # on all engines.
    # =========================================================================
    #
    # This is the educational payoff of having multiple engines: you can run the
    # SAME computation on NVIDIA-style SIMT, AMD-style SIMD, Google-style
    # systolic, and Apple-style MAC arrays, and verify they all produce the
    # same numerical results -- just with different execution traces, cycle
    # counts, and utilization.

    class TestCrossEngineScalarMultiply < Minitest::Test
      # SIMT: each thread computes 3.0 * 4.0 = 12.0.
      def test_simt_multiply
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([
          GpuCore.limm(0, 3.0), GpuCore.limm(1, 4.0),
          GpuCore.fmul(2, 0, 1), GpuCore.halt
        ])
        engine.run
        engine.threads.each do |t|
          assert_equal 12.0, t.core.registers.read_float(2)
        end
      end

      # SIMD: all lanes compute 3.0 * 4.0 = 12.0.
      def test_simd_multiply
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([
          GpuCore.limm(0, 3.0), GpuCore.limm(1, 4.0),
          GpuCore.fmul(2, 0, 1), GpuCore.halt
        ])
        engine.run
        4.times do |lane|
          assert_equal 12.0, engine.vrf.read(2, lane)
        end
      end

      # Systolic: 1x1 matmul is just a multiply.
      def test_systolic_multiply
        array = SystolicArray.new(SystolicConfig.new(rows: 1, cols: 1), Clock::ClockGenerator.new)
        result = array.run_matmul(activations: [[3.0]], weights: [[4.0]])
        assert_in_delta 12.0, result[0][0], 0.01
      end

      # MAC: one MAC unit computes 3.0 * 4.0 = 12.0.
      def test_mac_multiply
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 1), Clock::ClockGenerator.new)
        engine.load_inputs([3.0])
        engine.load_weights([4.0])
        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0], weight_indices: [0], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0),
          MACScheduleEntry.new(cycle: 3, operation: :store_output, output_index: 0)
        ]
        engine.load_schedule(schedule)
        engine.run
        assert_in_delta 12.0, engine.read_outputs[0], 0.01
      end
    end

    class TestCrossEngineDotProduct < Minitest::Test
      # SIMT: each thread multiplies one pair, then manual sum.
      def test_simt_dot_product
        engine = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.fmul(2, 0, 1), GpuCore.halt])
        a = [1.0, 2.0, 3.0, 4.0]
        b = [5.0, 6.0, 7.0, 8.0]
        4.times do |t|
          engine.set_thread_register(t, 0, a[t])
          engine.set_thread_register(t, 1, b[t])
        end
        engine.run

        total = 4.times.sum { |t| engine.threads[t].core.registers.read_float(2) }
        assert_in_delta 70.0, total, 0.1
      end

      # SIMD: all lanes multiply in parallel.
      def test_simd_dot_product
        engine = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        engine.load_program([GpuCore.fmul(2, 0, 1), GpuCore.halt])
        a = [1.0, 2.0, 3.0, 4.0]
        b = [5.0, 6.0, 7.0, 8.0]
        4.times do |lane|
          engine.set_lane_register(lane, 0, a[lane])
          engine.set_lane_register(lane, 1, b[lane])
        end
        engine.run

        total = 4.times.sum { |lane| engine.vrf.read(2, lane) }
        assert_in_delta 70.0, total, 0.1
      end

      # MAC: parallel MACs + reduce.
      def test_mac_dot_product
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        engine.load_inputs([1.0, 2.0, 3.0, 4.0])
        engine.load_weights([5.0, 6.0, 7.0, 8.0])
        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0, 1, 2, 3], weight_indices: [0, 1, 2, 3], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0)
        ]
        engine.load_schedule(schedule)
        engine.run
        assert_in_delta 70.0, engine.read_outputs[0], 0.1
      end
    end

    class TestCrossEngineMatmul < Minitest::Test
      # Systolic 2x2 matmul should match MAC 2x2 matmul.
      def test_systolic_matmul_matches_mac
        a = [[1.0, 2.0], [3.0, 4.0]]
        w = [[5.0, 6.0], [7.0, 8.0]]

        # Systolic
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        systolic_result = array.run_matmul(activations: a, weights: w)

        # MAC (manual C[0][0])
        mac = MACArrayEngine.new(MACArrayConfig.new(num_macs: 2), Clock::ClockGenerator.new)
        mac.load_inputs([1.0, 2.0])
        mac.load_weights([5.0, 7.0])
        mac.load_schedule([
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0, 1], weight_indices: [0, 1], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0)
        ])
        mac.run

        assert_in_delta 19.0, systolic_result[0][0], 0.1
        assert_in_delta 19.0, mac.read_outputs[0], 0.1
      end
    end

    class TestCrossEngineExecutionModels < Minitest::Test
      # Verify that each engine reports the correct execution model.
      def test_all_models
        warp = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        wave = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        systolic = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        mac = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)

        assert_equal :simt, warp.execution_model
        assert_equal :simd, wave.execution_model
        assert_equal :systolic, systolic.execution_model
        assert_equal :scheduled_mac, mac.execution_model
      end

      # All engines have names.
      def test_all_have_names
        warp = WarpEngine.new(WarpConfig.new(warp_width: 4), Clock::ClockGenerator.new)
        wave = WavefrontEngine.new(WavefrontConfig.new(wave_width: 4), Clock::ClockGenerator.new)
        systolic = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        mac = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)

        assert_equal "WarpEngine", warp.name
        assert_equal "WavefrontEngine", wave.name
        assert_equal "SystolicArray", systolic.name
        assert_equal "MACArrayEngine", mac.name
      end
    end
  end
end
