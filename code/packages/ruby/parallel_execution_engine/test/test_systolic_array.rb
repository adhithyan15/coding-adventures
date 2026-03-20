# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module ParallelExecutionEngine
    # =========================================================================
    # Tests for SystolicArray -- dataflow execution (Google TPU style).
    # =========================================================================

    # -----------------------------------------------------------------------
    # SystolicConfig
    # -----------------------------------------------------------------------

    class TestSystolicConfig < Minitest::Test
      def test_defaults
        config = SystolicConfig.new
        assert_equal 4, config.rows
        assert_equal 4, config.cols
      end

      def test_custom
        config = SystolicConfig.new(rows: 8, cols: 8)
        assert_equal 8, config.rows
        assert_equal 8, config.cols
      end
    end

    # -----------------------------------------------------------------------
    # SystolicPE
    # -----------------------------------------------------------------------

    class TestSystolicPE < Minitest::Test
      def test_creation
        zero = FpArithmetic.float_to_bits(0.0, FpArithmetic::FP32)
        pe = SystolicPE.new(row: 0, col: 0, weight: zero, accumulator: zero)
        assert_equal 0, pe.row
        assert_equal 0, pe.col
        assert_nil pe.input_buffer
      end

      # No input -> no computation.
      def test_compute_no_input
        zero = FpArithmetic.float_to_bits(0.0, FpArithmetic::FP32)
        pe = SystolicPE.new(row: 0, col: 0, weight: zero, accumulator: zero)
        result = pe.compute
        assert_nil result
      end

      # With input: acc = 0 + 2.0 * 3.0 = 6.0
      def test_compute_with_input
        weight = FpArithmetic.float_to_bits(3.0, FpArithmetic::FP32)
        zero = FpArithmetic.float_to_bits(0.0, FpArithmetic::FP32)
        input_val = FpArithmetic.float_to_bits(2.0, FpArithmetic::FP32)
        pe = SystolicPE.new(row: 0, col: 0, weight: weight,
                            accumulator: zero, input_buffer: input_val)

        output = pe.compute
        refute_nil output

        acc = FpArithmetic.bits_to_float(pe.accumulator)
        assert_in_delta 6.0, acc, 0.01
      end

      # Multiple computes should accumulate.
      def test_compute_accumulates
        weight = FpArithmetic.float_to_bits(1.0, FpArithmetic::FP32)
        zero = FpArithmetic.float_to_bits(0.0, FpArithmetic::FP32)
        pe = SystolicPE.new(row: 0, col: 0, weight: weight, accumulator: zero)

        pe.input_buffer = FpArithmetic.float_to_bits(2.0, FpArithmetic::FP32)
        pe.compute

        pe.input_buffer = FpArithmetic.float_to_bits(3.0, FpArithmetic::FP32)
        pe.compute

        assert_in_delta 5.0, FpArithmetic.bits_to_float(pe.accumulator), 0.01
      end
    end

    # -----------------------------------------------------------------------
    # SystolicArray -- basic properties
    # -----------------------------------------------------------------------

    class TestSystolicArrayProperties < Minitest::Test
      def test_name
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        assert_equal "SystolicArray", array.name
      end

      def test_width
        array = SystolicArray.new(SystolicConfig.new(rows: 3, cols: 4), Clock::ClockGenerator.new)
        assert_equal 12, array.width
      end

      def test_execution_model
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        assert_equal :systolic, array.execution_model
      end

      def test_initial_not_halted
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        refute array.halted?
      end

      def test_config_access
        config = SystolicConfig.new(rows: 3, cols: 3)
        array = SystolicArray.new(config, Clock::ClockGenerator.new)
        assert_same config, array.config
      end

      def test_grid_access
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        assert_equal 2, array.grid.length
        assert_equal 2, array.grid[0].length
      end

      def test_to_s
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        r = array.to_s
        assert_includes r, "SystolicArray"
        assert_includes r, "2x2"
      end
    end

    # -----------------------------------------------------------------------
    # SystolicArray -- weight loading
    # -----------------------------------------------------------------------

    class TestSystolicArrayWeights < Minitest::Test
      def test_load_weights
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        array.load_weights([[1.0, 2.0], [3.0, 4.0]])

        assert_equal 1.0, FpArithmetic.bits_to_float(array.grid[0][0].weight)
        assert_equal 2.0, FpArithmetic.bits_to_float(array.grid[0][1].weight)
        assert_equal 3.0, FpArithmetic.bits_to_float(array.grid[1][0].weight)
        assert_equal 4.0, FpArithmetic.bits_to_float(array.grid[1][1].weight)
      end
    end

    # -----------------------------------------------------------------------
    # SystolicArray -- input feeding
    # -----------------------------------------------------------------------

    class TestSystolicArrayInput < Minitest::Test
      def test_feed_input
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        array.feed_input(0, 5.0)
        # No error = success
      end

      def test_feed_input_out_of_range
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        assert_raises(IndexError) { array.feed_input(2, 1.0) }
        assert_raises(IndexError) { array.feed_input(-1, 1.0) }
      end

      def test_feed_input_vector
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        array.feed_input_vector([1.0, 2.0])
        # No error = success
      end
    end

    # -----------------------------------------------------------------------
    # SystolicArray -- matrix multiplication
    # -----------------------------------------------------------------------

    class TestSystolicArrayMatmul < Minitest::Test
      # Multiply by identity matrix should return the input.
      def test_identity_weights
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        result = array.run_matmul(
          activations: [[1.0, 0.0], [0.0, 1.0]],
          weights: [[1.0, 0.0], [0.0, 1.0]]
        )
        assert_in_delta 1.0, result[0][0], 0.01
        assert_in_delta 0.0, result[0][1], 0.01
        assert_in_delta 0.0, result[1][0], 0.01
        assert_in_delta 1.0, result[1][1], 0.01
      end

      # 2x2 matrix multiply.
      def test_simple_matmul
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        result = array.run_matmul(
          activations: [[1.0, 2.0], [3.0, 4.0]],
          weights: [[5.0, 6.0], [7.0, 8.0]]
        )
        assert_in_delta 19.0, result[0][0], 0.1
        assert_in_delta 22.0, result[0][1], 0.1
        assert_in_delta 43.0, result[1][0], 0.1
        assert_in_delta 50.0, result[1][1], 0.1
      end

      # 3x3 matrix multiply.
      def test_3x3_matmul
        a = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]]
        w = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]]
        array = SystolicArray.new(SystolicConfig.new(rows: 3, cols: 3), Clock::ClockGenerator.new)
        result = array.run_matmul(activations: a, weights: w)
        assert_in_delta 1.0, result[0][0], 0.1
        assert_in_delta 2.0, result[0][1], 0.1
        assert_in_delta 3.0, result[0][2], 0.1
        assert_in_delta 8.0, result[1][0], 0.1
        assert_in_delta 10.0, result[1][1], 0.1
        assert_in_delta 27.0, result[2][2], 0.1
      end

      # drain_outputs returns the correct shape.
      def test_drain_outputs
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 3), Clock::ClockGenerator.new)
        result = array.drain_outputs
        assert_equal 2, result.length
        assert_equal 3, result[0].length
      end
    end

    # -----------------------------------------------------------------------
    # SystolicArray -- stepping and traces
    # -----------------------------------------------------------------------

    class TestSystolicArrayStepping < Minitest::Test
      def test_step_produces_trace
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        array.load_weights([[1.0, 0.0], [0.0, 1.0]])
        array.feed_input(0, 2.0)

        trace = array.step(ParallelExecutionEngine.make_edge)
        assert_equal 1, trace.cycle
        assert_equal "SystolicArray", trace.engine_name
        assert_equal :systolic, trace.execution_model
        refute_nil trace.dataflow_info
      end

      def test_halts_when_no_data
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        array.load_weights([[1.0, 0.0], [0.0, 1.0]])
        array.feed_input(0, 1.0)

        20.times do |i|
          array.step(ParallelExecutionEngine.make_edge(i + 1))
          break if array.halted?
        end
        assert array.halted?
      end
    end

    # -----------------------------------------------------------------------
    # SystolicArray -- reset
    # -----------------------------------------------------------------------

    class TestSystolicArrayReset < Minitest::Test
      def test_reset
        array = SystolicArray.new(SystolicConfig.new(rows: 2, cols: 2), Clock::ClockGenerator.new)
        array.load_weights([[1.0, 2.0], [3.0, 4.0]])
        array.run_matmul(
          activations: [[1.0, 0.0], [0.0, 1.0]],
          weights: [[1.0, 2.0], [3.0, 4.0]]
        )

        array.reset
        refute array.halted?
        assert_equal 0.0, FpArithmetic.bits_to_float(array.grid[0][0].accumulator)
      end
    end
  end
end
