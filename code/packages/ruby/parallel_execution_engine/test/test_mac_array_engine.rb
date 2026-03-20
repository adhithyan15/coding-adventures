# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module ParallelExecutionEngine
    # =========================================================================
    # Tests for MACArrayEngine -- scheduled MAC array execution (NPU style).
    # =========================================================================

    # -----------------------------------------------------------------------
    # MACOperation and ActivationFunction constants
    # -----------------------------------------------------------------------

    class TestEnums < Minitest::Test
      def test_mac_operations
        assert_includes MAC_OPERATIONS, :load_input
        assert_includes MAC_OPERATIONS, :load_weights
        assert_includes MAC_OPERATIONS, :mac
        assert_includes MAC_OPERATIONS, :reduce
        assert_includes MAC_OPERATIONS, :activate
        assert_includes MAC_OPERATIONS, :store_output
      end

      def test_activation_functions
        assert_includes ACTIVATION_FUNCTIONS, :none
        assert_includes ACTIVATION_FUNCTIONS, :relu
        assert_includes ACTIVATION_FUNCTIONS, :sigmoid
        assert_includes ACTIVATION_FUNCTIONS, :tanh
      end
    end

    # -----------------------------------------------------------------------
    # MACScheduleEntry
    # -----------------------------------------------------------------------

    class TestMACScheduleEntry < Minitest::Test
      def test_creation
        entry = MACScheduleEntry.new(
          cycle: 1,
          operation: :mac,
          input_indices: [0, 1],
          weight_indices: [0, 1],
          output_index: 0
        )
        assert_equal 1, entry.cycle
        assert_equal :mac, entry.operation
        assert_equal [0, 1], entry.input_indices
        assert_equal 0, entry.output_index
      end

      def test_defaults
        entry = MACScheduleEntry.new(cycle: 0, operation: :mac)
        assert_equal [], entry.input_indices
        assert_equal [], entry.weight_indices
        assert_equal 0, entry.output_index
        assert_equal :none, entry.activation
      end

      def test_frozen
        entry = MACScheduleEntry.new(cycle: 0, operation: :mac)
        assert_raises(FrozenError) { entry.instance_variable_set(:@cycle, 5) }
      end
    end

    # -----------------------------------------------------------------------
    # MACArrayConfig
    # -----------------------------------------------------------------------

    class TestMACArrayConfig < Minitest::Test
      def test_defaults
        config = MACArrayConfig.new
        assert_equal 8, config.num_macs
        assert_equal 1024, config.input_buffer_size
        assert_equal 4096, config.weight_buffer_size
        assert_equal 1024, config.output_buffer_size
        assert_equal true, config.has_activation_unit
      end
    end

    # -----------------------------------------------------------------------
    # MACArrayEngine -- basic properties
    # -----------------------------------------------------------------------

    class TestMACArrayEngineProperties < Minitest::Test
      def test_name
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        assert_equal "MACArrayEngine", engine.name
      end

      def test_width
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 8), Clock::ClockGenerator.new)
        assert_equal 8, engine.width
      end

      def test_execution_model
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        assert_equal :scheduled_mac, engine.execution_model
      end

      def test_initial_halted
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        refute engine.halted?
      end

      def test_config_access
        config = MACArrayConfig.new(num_macs: 4)
        engine = MACArrayEngine.new(config, Clock::ClockGenerator.new)
        assert_same config, engine.config
      end

      def test_to_s
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        r = engine.to_s
        assert_includes r, "MACArrayEngine"
        assert_includes r, "num_macs=4"
      end
    end

    # -----------------------------------------------------------------------
    # MACArrayEngine -- data loading
    # -----------------------------------------------------------------------

    class TestMACArrayEngineLoading < Minitest::Test
      def test_load_inputs
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        engine.load_inputs([1.0, 2.0, 3.0, 4.0])
        outputs = engine.read_outputs
        assert_equal 0.0, outputs[0]
      end

      def test_load_weights
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        engine.load_weights([0.5, 0.5, 0.5, 0.5])
        # No error = success
      end
    end

    # -----------------------------------------------------------------------
    # MACArrayEngine -- execution
    # -----------------------------------------------------------------------

    class TestMACArrayEngineExecution < Minitest::Test
      # Compute a dot product: sum(input[i] * weight[i]).
      def test_dot_product
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        engine.load_inputs([1.0, 2.0, 3.0, 4.0])
        engine.load_weights([1.0, 1.0, 1.0, 1.0])

        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0, 1, 2, 3], weight_indices: [0, 1, 2, 3], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0),
          MACScheduleEntry.new(cycle: 3, operation: :store_output, output_index: 0)
        ]
        engine.load_schedule(schedule)
        engine.run

        outputs = engine.read_outputs
        assert_in_delta 10.0, outputs[0], 0.01
      end

      # Compute a weighted sum with different weights.
      def test_weighted_sum
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        engine.load_inputs([2.0, 3.0, 4.0, 5.0])
        engine.load_weights([0.5, 0.25, 0.125, 0.0625])

        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0, 1, 2, 3], weight_indices: [0, 1, 2, 3], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0),
          MACScheduleEntry.new(cycle: 3, operation: :store_output, output_index: 0)
        ]
        engine.load_schedule(schedule)
        engine.run

        expected = 2.0 * 0.5 + 3.0 * 0.25 + 4.0 * 0.125 + 5.0 * 0.0625
        assert_in_delta expected, engine.read_outputs[0], 0.01
      end

      # Test ReLU activation: max(0, x).
      def test_relu_activation
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 2), Clock::ClockGenerator.new)
        engine.load_inputs([3.0, -5.0])
        engine.load_weights([1.0, 1.0])

        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0, 1], weight_indices: [0, 1], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0),
          MACScheduleEntry.new(cycle: 3, operation: :activate, output_index: 0, activation: :relu),
          MACScheduleEntry.new(cycle: 4, operation: :store_output, output_index: 0)
        ]
        engine.load_schedule(schedule)
        engine.run

        # 3*1 + (-5)*1 = -2 -> ReLU(-2) = 0
        assert_equal 0.0, engine.read_outputs[0]
      end

      # Test sigmoid activation: 1/(1+e^-x).
      def test_sigmoid_activation
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 1), Clock::ClockGenerator.new)
        engine.load_inputs([0.0])
        engine.load_weights([1.0])

        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0], weight_indices: [0], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0),
          MACScheduleEntry.new(cycle: 3, operation: :activate, output_index: 0, activation: :sigmoid)
        ]
        engine.load_schedule(schedule)
        engine.run

        # sigmoid(0) = 0.5
        assert_in_delta 0.5, engine.read_outputs[0], 0.01
      end

      # Test tanh activation.
      def test_tanh_activation
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 1), Clock::ClockGenerator.new)
        engine.load_inputs([1.0])
        engine.load_weights([1.0])

        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0], weight_indices: [0], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0),
          MACScheduleEntry.new(cycle: 3, operation: :activate, output_index: 0, activation: :tanh)
        ]
        engine.load_schedule(schedule)
        engine.run

        assert_in_delta Math.tanh(1.0), engine.read_outputs[0], 0.01
      end

      # When has_activation_unit=false, ACTIVATE is skipped.
      def test_no_activation_unit
        engine = MACArrayEngine.new(
          MACArrayConfig.new(num_macs: 1, has_activation_unit: false),
          Clock::ClockGenerator.new
        )
        engine.load_inputs([5.0])
        engine.load_weights([1.0])

        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0], weight_indices: [0], output_index: 0),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0),
          MACScheduleEntry.new(cycle: 3, operation: :activate, output_index: 0, activation: :relu)
        ]
        engine.load_schedule(schedule)
        engine.run

        assert_in_delta 5.0, engine.read_outputs[0], 0.01
      end

      # LOAD_INPUT operation in schedule.
      def test_load_input_operation
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        engine.load_inputs([1.0, 2.0])

        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :load_input, input_indices: [0, 1])
        ]
        engine.load_schedule(schedule)
        traces = engine.run
        assert traces.length >= 1
        assert_includes traces[0].description, "LOAD_INPUT"
      end

      # LOAD_WEIGHTS operation in schedule.
      def test_load_weights_operation
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        engine.load_weights([1.0, 2.0])

        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :load_weights, weight_indices: [0, 1])
        ]
        engine.load_schedule(schedule)
        traces = engine.run
        assert_includes traces[0].description, "LOAD_WEIGHTS"
      end
    end

    # -----------------------------------------------------------------------
    # MACArrayEngine -- halting and idle cycles
    # -----------------------------------------------------------------------

    class TestMACArrayEngineHalting < Minitest::Test
      def test_halts_after_schedule
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0], weight_indices: [0])
        ]
        engine.load_schedule(schedule)
        engine.run
        assert engine.halted?
      end

      # Cycles with no schedule entry produce idle traces.
      def test_idle_cycle
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        schedule = [
          MACScheduleEntry.new(cycle: 3, operation: :mac,
            input_indices: [0], weight_indices: [0])
        ]
        engine.load_schedule(schedule)

        trace1 = engine.step(ParallelExecutionEngine.make_edge(1))
        assert_includes trace1.description, "No operation"
        assert_equal 0, trace1.active_count
      end

      # Stepping after completion returns halted trace.
      def test_halted_step
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        schedule = [
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0], weight_indices: [0])
        ]
        engine.load_schedule(schedule)
        engine.run

        trace = engine.step(ParallelExecutionEngine.make_edge(99))
        assert_includes trace.description.downcase, "complete"
      end
    end

    # -----------------------------------------------------------------------
    # MACArrayEngine -- reset
    # -----------------------------------------------------------------------

    class TestMACArrayEngineReset < Minitest::Test
      def test_reset
        engine = MACArrayEngine.new(MACArrayConfig.new(num_macs: 4), Clock::ClockGenerator.new)
        engine.load_inputs([1.0, 2.0])
        engine.load_weights([0.5, 0.5])
        engine.load_schedule([
          MACScheduleEntry.new(cycle: 1, operation: :mac,
            input_indices: [0, 1], weight_indices: [0, 1]),
          MACScheduleEntry.new(cycle: 2, operation: :reduce, output_index: 0)
        ])
        engine.run

        engine.reset
        refute engine.halted?
        assert_equal 0.0, engine.read_outputs[0]
      end
    end
  end
end
